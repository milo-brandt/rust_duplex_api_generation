use futures::{channel::oneshot, Future, FutureExt};
use protocol_util::{communication_context::{Context, DeferingContext}, base, generic::{Receivable, SendableAs}, types};
use serde::{Serialize, Deserialize};

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
