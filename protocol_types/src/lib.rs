pub mod generic;
use futures::{channel::oneshot, Future, FutureExt};
use protocol_util::communication_context::Context;
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

#[derive(Serialize, Deserialize)]
pub struct EchoMessage {
    pub message: String,
    pub future: generic::ChannelCoFuture<String>,
}

pub struct EchoMessageReceived {
    pub message: String,
    pub future: oneshot::Sender<String>,
}
impl EchoMessage {
    pub fn receive(self, context: &Context) -> (EchoMessageReceived, impl Future<Output=()> + Send + Unpin + 'static) {
        let (tx, rx) = oneshot::channel();
        let message_future = self.future.receive_feed(&context, rx.map(Result::unwrap));
        (EchoMessageReceived {
            message: self.message,
            future: tx
        }, message_future)
    }
}