use std::marker::PhantomData;

use serde::{Serialize, Deserialize};

pub mod mock;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Channel(pub u64);
pub trait ChannelSender {
   fn allocate_send_channel(&self) -> Channel;
   fn send_message(&self, channel: Channel, message: String);
}
pub trait ChannelReceiver {
   fn allocate_receive_channel(&self) -> Channel;
   fn listen(&self, channel: Channel, listener: impl FnMut(String) + 'static);
}

// 


pub struct TypedChannel<T>(pub u64, PhantomData<T>);
pub trait TypedChannelSender {
   fn allocate_send_channel<T>(&self) -> TypedChannel<T>;
   fn send_message<T>(&self, channel: TypedChannel<T>, message: String);
}
pub trait TypedChannelReceiver {
   fn allocate_receive_channel<T>(&self) -> TypedChannel<T>;
   fn listen<T>(&self, channel: TypedChannel<T>, listener: impl FnMut(T) + 'static);
}