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

#[derive(Serialize, Deserialize)]
pub struct EchoMessage {
    pub message: types::Primitive<String>,
    pub future: base::ChannelCoFuture<types::Primitive<String>>,
}

pub mod received {
    use protocol_util::{base, types};

    pub struct EchoMessage {
        pub message: String,
        pub future: base::ChannelCoFutureSender<types::Primitive<String>>,
    }
}
pub mod sendable {
    pub struct EchoMessage<Message, Future> {
        pub message: Message,
        pub future: Future
    }
}

impl Receivable for EchoMessage {
    type ReceivedAs = received::EchoMessage;

    fn receive_in_context(self, context: &Context) -> Self::ReceivedAs {
        received::EchoMessage {
            message: self.message.receive_in_context(context),
            future: self.future.receive_in_context(context)
        }
    }
}
impl<Message: SendableAs<types::Primitive<String>>, Future: SendableAs<base::ChannelCoFuture<types::Primitive<String>>>> SendableAs<EchoMessage> for sendable::EchoMessage<Message, Future> {
    fn prepare_in_context(self, context: &DeferingContext) -> EchoMessage {
        EchoMessage { message: self.message.prepare_in_context(context), future: self.future.prepare_in_context(context) }
    }
}