use futures::{channel::oneshot, Future, FutureExt};
use protocol_util::{communication_context::{Context, DeferingContext}, base, generic::{Receivable, SendableAs}, types};
use serde::{Serialize, Deserialize};

/*
#[derive(Serialize, Deserialize)]
pub enum Message {
    UserEnter(String),
    UserLeave(String),
    Message(String),
}
#[derive(Serialize, Deserialize)]
pub struct RoomResponse {
    messages: generic::ChannelStream<Message>, // parallel to this response.
    send_message: generic::ChannelCoStream<String>, // antiparallel; for user to send messages. sending None leaves channel.
}
#[derive(Serialize, Deserialize)]
pub struct RoomRequest {
    username: String,
    room_name: String,
    response: generic::ChannelCoFuture<RoomResponse>,
}
*/


#[protocol_util_macros::protocol_type]
pub enum EchoResponse {
    Alright(types::Primitive<String>),
    UhOh(types::Primitive<u64>),
}


#[protocol_util_macros::protocol_type]
pub struct EchoMessage {
    pub message: types::Primitive<String>,
    pub future: base::ChannelCoFuture<EchoResponse>,
}
