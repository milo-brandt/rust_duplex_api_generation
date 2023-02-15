use futures::future::ready;
use futures::{StreamExt, Sink, SinkExt, Stream};
use futures::channel::mpsc;
use pin_project::pin_project;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::sync::{Mutex, Arc};
use std::task::{self, Poll};
use crate::communication_context::{Context, DeferingContext};

use crate::generic::{Receivable, SendableAs};

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct Channel<T>(pub u64, pub PhantomData<T>);

impl<T> Clone for Channel<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), Default::default())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelStream<T>(pub Channel<Option<T>>);
// represent a stream antiparallel to the message
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelCoStream<T>(pub Channel<Option<T>>);
// represent a oneshot channel parallel to the message
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelFuture<T>(pub Channel<T>);
// represent a oneshot cannel antiparallel to the message
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelCoFuture<T>(pub Channel<T>);

/*
    Trait implementations
*/

// CoStreams are received as an object that can be used as a Sink for any type that is sendable
// as the desired type.
// TODO - could make this Clone if we needed...
pub struct ChannelCoStreamSender<T: Serialize> {
    context: Context,
    channel: Channel<Option<T>>,
    closed: bool,
}
impl<T: Serialize> ChannelCoStreamSender<T> {
    // TODO: Handle errors?
    pub fn channel_send(&self, value: impl SendableAs<T>) {
        self.channel_send_scoped(|ctx| value.send_in_context(ctx));
    }
    pub fn channel_send_raw(&self, value: T) {
        self.context.sender.send(&self.channel, Some(value));
    }
    pub fn channel_send_scoped<F: FnOnce(&DeferingContext) -> T>(&self, callback: F) {
        let defering = self.context.clone().defering();
        self.channel_send_raw(callback(&defering));
        let (_, futures) = defering.destructure();
        for future in futures {
            self.context.spawner.spawn_boxed(future);
        }
    }
}
impl<T: Serialize> Drop for ChannelCoStreamSender<T> {
    fn drop(&mut self) {
        if !self.closed {
            self.context.sender.send(&self.channel, None);
        }
    }
}
impl<T: Serialize, U: SendableAs<T>> Sink<U> for ChannelCoStreamSender<T> {
    type Error = ();

    fn poll_ready(self: std::pin::Pin<&mut Self>, _: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: U) -> Result<(), Self::Error> {
        self.channel_send(item);
        Ok(())
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.context.sender.send(&self.channel, None);
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
impl<T: Serialize + Send + 'static> Receivable for ChannelCoStream<T> {
    type ReceivedAs = ChannelCoStreamSender<T>;
    fn receive_in_context(self, context: &Context) -> Self::ReceivedAs {
        ChannelCoStreamSender {
            context: context.clone(),
            channel: self.0,
            closed: false,
        }
    }
}
// Add a wrapper around sinks that allows it to be sent this way... issue with conflicting traits
pub struct SendableSink<U>(U);

// We don't need inherently need Unpin here, but it's easier to write...
impl<T: Receivable + DeserializeOwned + Send + 'static, U: Sink<T::ReceivedAs> + Send + Unpin + 'static> SendableAs<ChannelCoStream<T>> for SendableSink<U> {
    fn send_in_context(self, context: &DeferingContext) -> ChannelCoStream<T> {
        let channel = context.channel_allocator.incoming::<Option<T>>();
        context.controller.listen_with_handle(channel.clone(), {
            let context = context.context_clone();
            let sink = Arc::new(async_mutex::Mutex::new(self));
            move |message, handle| {
                let sink = sink.clone();
                let context = context.clone();
                async move {
                    // Safe to lock as these futures must execute sequentially.
                    // Could probably do try_lock
                    let sink = &mut sink.lock().await.0;
                    if let Some(message) = message {
                        drop(sink.send(message.receive_in_context(&context)));
                    } else {
                        drop(sink.close().await);
                        handle.disconnect();
                    }
                }
            }
        });
        ChannelCoStream(channel)
    }
}

impl<T: DeserializeOwned + Receivable> Receivable for ChannelStream<T>
where T::ReceivedAs: Send + 'static
{
    type ReceivedAs = mpsc::UnboundedReceiver<T::ReceivedAs>;

    fn receive_in_context(self, context: &Context) -> Self::ReceivedAs {
        let (sender, receiver) = mpsc::unbounded();
        context.controller.listen_with_handle(self.0, {
            let context = context.clone();
            move |message, handle| {
                match message {
                    Some(message) => {
                        drop(sender.unbounded_send(message.receive_in_context(&context)))
                    },
                    None => {
                        drop(sender.close_channel());
                        handle.disconnect();
                    }
                }
                ready(())
            }
        });
        receiver
    }
}

pub struct SendableStream<U>(U);
impl<T: Serialize + Send + 'static, U: Stream + Unpin + Send + 'static> SendableAs<ChannelStream<T>> for SendableStream<U>
where U::Item: SendableAs<T>
{
    fn send_in_context(mut self, context: &DeferingContext) -> ChannelStream<T> {
        let channel = context.channel_allocator.outgoing::<Option<T>>();
        context.defer_future({
            let context = context.context_clone();
            let channel = channel.clone();
            async move {
                loop {
                    let item = self.0.next().await;
                    let context = context.clone().defering();
                    let value = item.map(|item| item.send_in_context(&context));
                    context.sender.send(&channel, value);
                    let (context, futures) = context.destructure();
                    context.spawner.spawn_boxeds(futures)
                }
            }
        });
        ChannelStream(channel)
    }
}