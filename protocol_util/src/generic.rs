use crate::{communication_context::{Context, DeferingContext}, spawner::BoxedFuture};

pub trait Receivable {
    type ReceivedAs;
    fn receive_in_context(self, context: &Context) -> Self::ReceivedAs;
}

pub struct SendableWithFutures<T> {
    pub value: T,
    pub futures: Vec<BoxedFuture>,
}
impl<T> SendableWithFutures<T> {
    pub fn prepare<U: SendableAs<T>>(context: &Context, value: U) -> Self {
        let defering = context.clone().defering();
        let value = value.prepare_in_context(&defering);
        let (_, futures) = defering.destructure();
        Self {
            value,
            futures
        }
    }
}

pub trait SendableAs<T> {
    fn prepare_in_context(self, context: &DeferingContext) -> T;
}
impl<T> SendableAs<T> for T {
    fn prepare_in_context(self, _: &DeferingContext) -> T {
        self
    }
}
impl<T> SendableAs<T> for SendableWithFutures<T> {
    fn prepare_in_context(self, context: &DeferingContext) -> T {
        context.defer_futures_boxed(self.futures);
        self.value
    }
}

// Add a wrapper around sinks that allows it to be sent this way... issue with conflicting traits
pub struct DefaultSendable<U>(pub U);
impl<T, U: SendableAs<T>> SendableAs<Option<T>> for DefaultSendable<Option<U>> {
    fn prepare_in_context(self, context: &DeferingContext) -> Option<T> {
        self.0.map(|value| value.prepare_in_context(context))
    }
}