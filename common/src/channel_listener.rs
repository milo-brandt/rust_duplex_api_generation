use std::{cell::{UnsafeCell, Cell}, collections::{HashMap, VecDeque, HashSet}, mem, future::Future, convert::Infallible, task::{Poll, Context}, pin::Pin};

use futures::{Sink, Stream, stream::Fuse, StreamExt};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use serde_json::{from_str, to_string};

use crate::generic::{InChannel, OutChannel};
use pin_project::pin_project;



/*
    This is very silly, apparently: we basically split the input message by channel, treat the
    listeners like streams of "finite sinks" and the others as a stream. Could probably implement
    with primitives.
*/

pub enum StreamListenerResult {
    Continue,
    Stop
}
pub trait StreamListener {
    fn receive(self: Pin<&mut Self>, message: String);
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<StreamListenerResult>;
}
pub trait IntoStreamListener {
    type Listener: StreamListener;
    fn into_stream_listener(self) -> Self::Listener;
}

#[pin_project]
pub struct AsyncFunctionListener<F, Fut> {
    function: F,
    #[pin]
    future: Option<Fut>
}



impl<F, Fut> StreamListener for AsyncFunctionListener<F, Fut>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output=StreamListenerResult>
{
    fn receive(self: Pin<&mut Self>, message: String) {
        let mut proj_self = self.project();
        let new_future = (proj_self.function)(message);
        proj_self.future.set(Some(new_future));
    }

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<StreamListenerResult> {
        let mut proj_self = self.project();
        match proj_self.future.as_mut().as_pin_mut() {
            None => Poll::Ready(StreamListenerResult::Continue),
            Some(fut) => match fut.poll(cx) {
                Poll::Ready(value) => {
                    proj_self.future.set(None);
                    Poll::Ready(value)
                }
                Poll::Pending => Poll::Pending
            },
        }
    }
}
impl<F, Fut> IntoStreamListener for F
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output=StreamListenerResult>
{
    type Listener = AsyncFunctionListener<F, Fut>;

    fn into_stream_listener(self) -> Self::Listener {
        AsyncFunctionListener {
            function: self,
            future: None,
        }
    }
}

pub enum ChannelCommand {
    Receive(u64, String),
    Listen(u64, Box<dyn StreamListener + Send>),
}
impl ChannelCommand {
    pub fn listen<Listener>(channel: u64, listener: Listener) -> Self
    where
        Listener: IntoStreamListener,
        Listener::Listener: Send + 'static
    {
        ChannelCommand::Listen(channel, Box::new(listener.into_stream_listener()))
    }
}

#[derive(Default)]
struct StreamState {
    listener: VecDeque<Pin<Box<dyn StreamListener + Send>>>,
    waiting: VecDeque<String>,
}
#[pin_project(project = ChanneledListenerProjection)]
pub struct ChanneledListener<Input> {
    running: HashSet<u64>,
    channels: HashMap<u64, StreamState>,
    #[pin]
    input: Fuse<Input>,
}

impl StreamState {
    fn try_to_run(&mut self, cx: &mut Context<'_>) -> bool {
        loop {
            match self.listener.front_mut().map(|item| item.as_mut().poll_ready(cx)) {
                Some(Poll::Ready(StreamListenerResult::Continue)) => (),
                Some(Poll::Ready(StreamListenerResult::Stop)) => {
                    self.listener.pop_front();
                },
                // Future is running.
                Some(Poll::Pending) => return true,
                // No listener; return.
                None => return false,
            }
            // If we get here, there is nothing running, but maybe a possibility to run something.
            if let Some(listener) = self.listener.front_mut() {
                if let Some(message) = self.waiting.pop_front() {
                    listener.as_mut().receive(message);
                    continue;
                }
            }
            // Either no listener or no messages; return.
            return false;
        }
    }
}

impl<Input: Stream<Item=ChannelCommand>> ChanneledListener<Input> {
    pub fn new(input: Input) -> Self {
        Self {
            running: Default::default(),
            channels: Default::default(),
            input: input.fuse(),
        }
    }
    // Try to run a stream state; keep global state updated.
    fn try_to_run(running: &mut HashSet<u64>, channel: u64, state: &mut StreamState, cx: &mut Context<'_>) {
        if state.try_to_run(cx) {
            running.insert(channel);
        } else {
            running.remove(&channel);
        }
    } 
}


impl<Input> Future for ChanneledListener<Input>
where
    Input: Stream<Item=ChannelCommand>
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: May want to limit number of times this can run to prevent blocking on busy input...
        let mut proj_self = self.project();
        while let Poll::Ready(Some(value)) = proj_self.input.as_mut().poll_next(cx) {
            match value {
                ChannelCommand::Receive(channel, message) => {
                    let state = proj_self.channels.entry(channel).or_insert_with(Default::default);
                    state.waiting.push_back(message);
                    Self::try_to_run(&mut proj_self.running, channel, state, cx);
                },
                ChannelCommand::Listen(channel, listener) => {
                    let state = proj_self.channels.entry(channel).or_insert_with(Default::default);
                    state.listener.push_back(Box::into_pin(listener));
                    Self::try_to_run(&mut proj_self.running, channel, state, cx);
                },
            }
        }
        // TODO: Should delete empty channels...
        proj_self.running.retain(|channel| {
            match proj_self.channels.get_mut(channel) {
                Some(state) => state.try_to_run(cx),
                None => false,
            }
        });
        if proj_self.input.is_done() && proj_self.running.is_empty() {
            // Could potentially return state (could have listeners or messages remaining)
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}







#[cfg(test)]
mod tests {
    use std::{sync::{Arc, Mutex}, ops::Deref};

    use tokio::join;
    use assert_impl::assert_impl;
    use futures::{channel::mpsc, SinkExt, future::ready, poll};

    use super::*;

    #[tokio::test]
    async fn test_future() {
        let (mut tx, rx) = mpsc::unbounded::<ChannelCommand>();
        let receiver = ChanneledListener::new(rx);
        tx.close().await.unwrap();
        receiver.await;
    }

    // For testing purposes: may want a simpler channel guarunteed to have the value ready upon await (?) and
    // a simple executor that crashes if there is nothing to poll.
    
    #[tokio::test]
    async fn test_callback() {
        let (mut tx, rx) = mpsc::unbounded::<ChannelCommand>();
        let mut receiver = ChanneledListener::new(rx);
        let value = Arc::new(Mutex::new(String::new()));
        tx.send(ChannelCommand::Receive(0, "Hello".into())).await.unwrap();
        assert_eq!(poll!(&mut receiver), Poll::Pending);
        assert_eq!(value.lock().unwrap().deref(), "");        
        tx.send(ChannelCommand::listen(0, {
            let value = value.clone();
            move |message| {
                *value.lock().unwrap() = message;
                ready(StreamListenerResult::Continue)
            }
        })).await.unwrap();
        assert_eq!(poll!(&mut receiver), Poll::Pending);
        assert_eq!(value.lock().unwrap().deref(), "Hello");       

        tx.send(ChannelCommand::Receive(0, "Bye".into())).await.unwrap();
        assert_eq!(value.lock().unwrap().deref(), "Hello");       
        assert_eq!(poll!(&mut receiver), Poll::Pending);
        assert_eq!(value.lock().unwrap().deref(), "Bye");       

        tx.send(ChannelCommand::Receive(1, "Ohno".into())).await.unwrap();
        assert_eq!(poll!(&mut receiver), Poll::Pending);
        assert_eq!(value.lock().unwrap().deref(), "Bye");
        
        tx.send(ChannelCommand::listen(0, {
            let value = value.clone();
            move |message| {
                *value.lock().unwrap() = "Oh no".into(); // Never called because predecessor never stops...
                ready(StreamListenerResult::Continue)
            }
        })).await.unwrap();

        tx.send(ChannelCommand::Receive(0, "Hey".into())).await.unwrap();
        assert_eq!(poll!(&mut receiver), Poll::Pending);
        assert_eq!(value.lock().unwrap().deref(), "Hey");

        tx.close().await.unwrap();
        receiver.await;
    }

    #[test]
    fn items_are_send() {
        assert_impl!(Send: ChanneledListener<mpsc::UnboundedReceiver<u64>>);
    }
}



/*
enum SyncStream {
    Listening(Box<dyn FnMut(String) + Send + 'static>),
    Collecting(Vec<String>),
}
#[derive(Default)]
struct ListenerInner {
    callbacks: HashMap<u64, SyncStream>,
    receiving: Option<u64>,
}
pub struct Listener {
    inner: UnsafeCell<ListenerInner>,
}


impl Listener {
    pub fn new() -> Self {
        Listener {
            inner: UnsafeCell::new(Default::default()),
        }
    }
    pub fn listen(&self, channel: u64, mut f: impl FnMut(String) + Send + 'static) {
        let inner_ptr = self.inner.get();
        // Make sure that we're not currently receiving the channel we called listen on!
        match &unsafe { &*inner_ptr }.receiving {
            Some(receiving_channel) if *receiving_channel == channel => {
                panic!("Listener had listen called on receiving channel.")
            }
            _ => (),
        }
        {
            let entry = unsafe { &mut *inner_ptr }
                .callbacks
                .entry(channel)
                .or_insert_with(|| SyncStream::Collecting(Vec::new()));
            match entry {
                SyncStream::Listening(_) => todo!(),
                SyncStream::Collecting(collected) => {
                    for item in mem::take(collected) {
                        f(item);
                    }
                }
            }
            *entry = SyncStream::Listening(Box::new(f));
        }
    }
    pub fn receive_message(&self, channel: u64, message: String) {
        // Receive a message; should not be called recursively & the listener should be not be disturbed
        // while this operation is in progress!

        // Set a guard to make sure that our current listener proceeds undisturbed!
        let inner_ptr = self.inner.get();

        {
            let receiving = &mut unsafe { &mut *inner_ptr }.receiving;
            if receiving.is_some() {
                panic!("Listener received multiple messages at once!");
            }
            *receiving = Some(channel)
        }
        {
            match &mut unsafe { &mut *inner_ptr }
                .callbacks
                .entry(channel)
                .or_insert_with(|| SyncStream::Collecting(Vec::new()))
            {
                SyncStream::Listening(callback) => callback(message),
                SyncStream::Collecting(collected) => collected.push(message),
            }
        }
        // Reset the guard.
        unsafe { &mut *inner_ptr }.receiving = None;
    }
}

pub struct TypedListener {
    listener: Listener,
}
impl TypedListener {
    pub fn new() -> Self {
        TypedListener {
            listener: Listener::new(),
        }
    }
    pub fn listen<T: DeserializeOwned>(
        &self,
        target: InChannel<T>,
        mut callback: impl FnMut(T) + Send + 'static,
    ) {
        self.listener.listen(target.0, move |message| {
            callback(from_str(&message).unwrap())
        });
    }
    pub fn receive(&self, channel: u64, message: String) {
        self.listener.receive_message(channel, message);
    }
}

pub trait Sender {
    // Note: should not allow send to call itself recursively, however this is implemented.
    fn send(&self, channel: u64, message: String);
}
impl<T: Fn(u64, String)> Sender for T {
    fn send(&self, channel: u64, message: String) {
        self(channel, message)
    }
}

pub struct DuplexEndpoint<S: Sender> {
    sender: S,
    listener: TypedListener,
    next_local_channel: Cell<u64>,
    next_remote_channel: Cell<u64>,
}
/*
    I think this is secretly a trait...
*/
impl<S: Sender> DuplexEndpoint<S> {
    pub fn new(sender: S) -> Self {
        DuplexEndpoint {
            sender,
            listener: TypedListener::new(),
            next_local_channel: Cell::new(0),
            next_remote_channel: Cell::new(u64::MAX),
        }
    }
    pub fn out_channel<T>(&self) -> OutChannel<T> {
        let index = self.next_local_channel.get();
        self.next_local_channel.set(index + 1);
        OutChannel(index, Default::default())
    }
    pub fn in_channel<T>(&self) -> InChannel<T> {
        let index = self.next_remote_channel.get();
        self.next_remote_channel.set(index - 1);
        InChannel(index, Default::default())
    }
    pub fn send<T: Serialize>(&self, target: &OutChannel<T>, value: T) {
        self.sender.send(target.0, to_string(&value).unwrap());
    }
    pub fn listen<T: DeserializeOwned>(
        &self,
        target: InChannel<T>,
        callback: impl FnMut(T) + Send + 'static,
    ) {
        self.listener.listen(target, callback);
    }
    pub fn receive(&self, channel: u64, message: String) {
        self.listener.receive(channel, message);
    }
}*/
/*
    Maybe Listener/TypedListener need to abstract somehow? Or some sort of queue?
*/

/*
What we want is some Send + Sync object that can handle the
    out_channel
    in_channel
    send
    listen
    receive
methods in some sensible way without excessive blocking. Probably need to use synchronous primitives
for this?

*/
/*


*/

// Generally: want send + sync
/*use async_trait::async_trait;

trait ChannelAllocator {
    fn allocate_out_channel() -> u64;
    fn allocate_in_channel() -> u64;
}
trait ListenerAssigner {
    fn listen_to_channel(channel: u64, listener: Box<dyn FnMut(String) -> Box<dyn Future<Output = ()> + 'static> + 'static>);
}
struct DuplexEndpoint<
    Sender: Sink<(u64, String)>,
    Receiver: Sink<(u64, String)>,
    Listener: ListenerAssigner,
    Allocator: ChannelAllocator
> {
    sender: Sender,
    receiver: Receiver,
    listener: Listener,
    channel_allocator: Allocator,
}*/