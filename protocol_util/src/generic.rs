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
macro_rules! primitive {
    ($ty:ty) => {
        impl Receivable for $ty {
            type ReceivedAs = $ty;

            fn receive_in_context(self, _: &Context) -> Self::ReceivedAs {
                self
            }
        }
    };
}
primitive!(String);
primitive!(u8);
primitive!(u16);
primitive!(u32);
primitive!(u64);
primitive!(i8);
primitive!(i16);
primitive!(i32);
primitive!(i64);
primitive!(bool);

/*
Thoughts: Generics might be somewhat of a poor way to do this; it's hard to get the interface we want - ideally, would like
to have implementations of Receivable for every primitive type + allow Options to be handled correctly.

Trickiness is in making such a thing friendly to splitting across libraries/modules - may be a question of fancy macros?

Could also accept that the protocol types aren't really types of interest in themselves...
 */