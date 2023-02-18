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
pub enum EchoResponse {
    Alright(types::Primitive<String>),
    UhOh(types::Primitive<u64>),
}


#[derive(Serialize, Deserialize)]
pub struct EchoMessage {
    pub message: types::Primitive<String>,
    pub future: base::ChannelCoFuture<EchoResponse>,
}

pub mod received {
    use protocol_util::{base, types, generic::Receivable};

    pub struct EchoMessage {
        pub message: String,
        pub future: base::ChannelCoFutureSender<super::EchoResponse>,
    }
    pub enum EchoResponse {
        Alright(<types::Primitive<String> as Receivable>::ReceivedAs),
        UhOh(<types::Primitive<u64> as Receivable>::ReceivedAs),
    }
}
pub mod sendable {
    use protocol_util::generic::Infallible;

    pub struct EchoMessage<Message, Future> {
        pub message: Message,
        pub future: Future
    }
    pub struct EchoResponseAlright<Alright0>(pub Alright0);
    pub struct EchoResponseUhOh<UhOh0>(pub UhOh0);
    pub enum EchoResponse<Alright0, UhOh0> {
        Alright(Alright0),
        UhOh(UhOh0),
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
impl Receivable for EchoResponse {
    type ReceivedAs = received::EchoResponse;

    fn receive_in_context(self, context: &Context) -> Self::ReceivedAs {
        match self {
            EchoResponse::Alright(value) => received::EchoResponse::Alright(value.receive_in_context(context)),
            EchoResponse::UhOh(value) => received::EchoResponse::UhOh(value.receive_in_context(context)),
        }
    }
}
impl<Message: SendableAs<types::Primitive<String>>, Future: SendableAs<base::ChannelCoFuture<EchoResponse>>> SendableAs<EchoMessage> for sendable::EchoMessage<Message, Future> {
    fn prepare_in_context(self, context: &DeferingContext) -> EchoMessage {
        EchoMessage { message: self.message.prepare_in_context(context), future: self.future.prepare_in_context(context) }
    }
}
impl<Alright0: SendableAs<types::Primitive<String>>, UhOh0: SendableAs<types::Primitive<u64>>> SendableAs<EchoResponse> for sendable::EchoResponse<Alright0, UhOh0> {
    fn prepare_in_context(self, context: &DeferingContext) -> EchoResponse {
        match self {
            sendable::EchoResponse::Alright(value) => EchoResponse::Alright(value.prepare_in_context(context)),
            sendable::EchoResponse::UhOh(value) => EchoResponse::UhOh(value.prepare_in_context(context)),
        }
    }
}
impl<Alright0: SendableAs<types::Primitive<String>>> SendableAs<EchoResponse> for sendable::EchoResponseAlright<Alright0> {
    fn prepare_in_context(self, context: &DeferingContext) -> EchoResponse {
        EchoResponse::Alright(self.0.prepare_in_context(context))
    }
}
impl<UhOh0: SendableAs<types::Primitive<u64>>> SendableAs<EchoResponse> for sendable::EchoResponseUhOh<UhOh0> {
    fn prepare_in_context(self, context: &DeferingContext) -> EchoResponse {
        EchoResponse::UhOh(self.0.prepare_in_context(context))
    }
}