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
        let value = value.send_in_context(&defering);
        let (_, futures) = defering.destructure();
        Self {
            value,
            futures
        }
    }
}

pub trait SendableAs<T> {
    fn send_in_context(self, context: &DeferingContext) -> T;
}
impl<T> SendableAs<T> for T {
    fn send_in_context(self, _: &DeferingContext) -> T {
        self
    }
}
impl<T> SendableAs<T> for SendableWithFutures<T> {
    fn send_in_context(self, context: &DeferingContext) -> T {
        context.defer_futures_boxed(self.futures);
        self.value
    }
}