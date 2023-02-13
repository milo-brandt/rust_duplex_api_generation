use std::{future::ready, task::{Poll, self}, pin::Pin};

use futures::{channel::{mpsc, oneshot}, Stream, Future};
use pin_project::pin_project;
use protocol_util::{generic::Channel, receiver::ArcListenHandle, sender::Sender};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use protocol_util::communication_context::Context;

/*
Best is probably: receiver is obligated to listen to all messages that arrive.

Sender must respond to requests to close channels.
*/

// represent a stream parallel to the message
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelStream<T>(Channel<Option<T>>);
// represent a stream antiparallel to the message
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelCoStream<T>(Channel<Option<T>>);
// represent a oneshot channel parallel to the message
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelFuture<T>(Channel<T>);
// represent a oneshot cannel antiparallel to the message
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChannelCoFuture<T>(Channel<T>);

pub mod received {

}

impl<T: DeserializeOwned> ChannelStream<T> {
    pub fn receive_mapped<Out: Send + 'static, F: FnMut(T) -> Out + Send + 'static>(self, context: &Context, mut map: F) -> mpsc::UnboundedReceiver<Out> {
        let (mut sender, receiver) = mpsc::unbounded();
        context.controller.listen_with_handle(self.0, move |message, stop_handle| {
            match message {
                Some(message) => drop(sender.unbounded_send(map(message))),
                None => {
                    sender.disconnect();
                    stop_handle.disconnect();
                }
            }
            ready(())
        });
        receiver
    }
    pub fn allocate(self, context: &Context) -> (ChannelStream<T>, ChannelCoStream<T>) {
        let channel = context.channel_allocator.outgoing();
        (ChannelStream(channel.clone()), ChannelCoStream(channel))
    }
}

#[pin_project]
pub struct ChannelCoStreamFuture<T, Input> {
    sender: Sender,
    channel: Channel<Option<T>>,
    #[pin]
    stream: Input
}
impl<T: Serialize, Input: Stream<Item=T>> Future for ChannelCoStreamFuture<T, Input>
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut proj_self = self.project();
        loop {
            match proj_self.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(message)) => proj_self.sender.send(&proj_self.channel, Some(message)),
                Poll::Ready(None) => {
                    proj_self.sender.send(&proj_self.channel, None);
                    return Poll::Ready(())
                },
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<T: Serialize> ChannelCoStream<T> {
    pub fn receive_feed<Input: Stream<Item=T> + Send>(self, context: &Context, input: Input) -> ChannelCoStreamFuture<T, Input> {
        ChannelCoStreamFuture {
            sender: context.sender.clone(),
            channel: self.0,
            stream: input,
        }
    }
    pub fn allocate(context: &Context) -> (ChannelCoStream<T>, ChannelStream<T>) {
        let channel = context.channel_allocator.incoming();
        (ChannelCoStream(channel.clone()), ChannelStream(channel))
    }
}

impl<T: DeserializeOwned> ChannelFuture<T> {
    pub fn receive_mapped<Out: Send + 'static, Map: FnOnce(T) -> Out + Send + 'static>(self, context: &Context, map: Map) -> oneshot::Receiver<Out> {
        let (sender, receiver) = oneshot::channel();
        let mut state = Some((sender, map));
        context.controller.listen_with_handle(self.0, move |message, stop_handle| {
            if let Some((sender, map)) = state.take() {
                drop(sender.send(map(message)));
                stop_handle.disconnect()
            } else {
                panic!("A one-shot future was hit twice!");
            }
            ready(())
        });
        receiver
    }
    pub fn allocate(context: &Context) -> (ChannelFuture<T>, ChannelCoFuture<T>) {
        let channel = context.channel_allocator.outgoing();
        (ChannelFuture(channel.clone()), ChannelCoFuture(channel))
    }
}

#[pin_project]
pub struct ChannelCoFutureFuture<T, Input> {
    sender: Sender,
    channel: Channel<T>,
    #[pin]
    future: Input
}
impl<T: Serialize, Input: Future<Output=T>> Future for ChannelCoFutureFuture<T, Input>
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut proj_self = self.project();
        match proj_self.future.as_mut().poll(cx) {
            Poll::Ready(message) => {
                proj_self.sender.send(&proj_self.channel, message);
                Poll::Ready(())
            }
            Poll::Pending => return Poll::Pending,
        }
    }
}

impl<T: Serialize> ChannelCoFuture<T> {
    pub fn receive_feed<Input: Future<Output=T> + Send>(self, context: &Context, input: Input) -> ChannelCoFutureFuture<T, Input> {
        ChannelCoFutureFuture {
            sender: context.sender.clone(),
            channel: self.0,
            future: input,
        }
    }
    pub fn allocate(context: &Context) -> (ChannelCoFuture<T>, ChannelFuture<T>) {
        let channel = context.channel_allocator.incoming();
        (ChannelCoFuture(channel.clone()), ChannelFuture(channel))
    }
}




/*
Contract on allocate seems something like this: the thing allocated is *obligated* to be sent to the receiver.
The co-thing allocated is *obligated* to be received by us (immediately, if it cares).
 */