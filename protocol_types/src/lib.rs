use futures::{channel::oneshot, Future, FutureExt};
use protocol_util::{communication_context::{Context, DeferingContext}, base, generic::{Receivable, SendableAs}};
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
    pub future: base::ChannelCoFuture<String>,
}

pub mod received {
    use protocol_util::{generic::Receivable, base};

    pub struct EchoMessage {
        pub message: String,
        pub future: <base::ChannelCoFuture<String> as Receivable>::ReceivedAs,
    }
}
pub mod sendable {
    pub struct EchoMessage<Future> {
        pub message: String,
        pub future: Future
    }
}

impl Receivable for EchoMessage {
    type ReceivedAs = received::EchoMessage;

    fn receive_in_context(self, context: &Context) -> Self::ReceivedAs {
        received::EchoMessage {
            message: self.message,
            future: self.future.receive_in_context(context)
        }
    }
}
impl<Future: SendableAs<base::ChannelCoFuture<String>>> SendableAs<EchoMessage> for sendable::EchoMessage<Future> {
    fn prepare_in_context(self, context: &DeferingContext) -> EchoMessage {
        EchoMessage { message: self.message, future: self.future.prepare_in_context(context) }
    }
}