use std::marker::PhantomData;
use generic::Channel;
use serde::{Deserialize, Serialize};

enum Direction {
   Send,
   Receive
}
enum Mode {
   Once,
   Stream
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(transparent)]
pub struct ParallelStream<T>(Channel, PhantomData<T>);
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(transparent)]
pub struct AntiStream<T>(Channel, PhantomData<T>);
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(transparent)]
pub struct ParallelFuture<T>(Channel, PhantomData<T>);
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(transparent)]
pub struct AntiFuture<T>(Channel, PhantomData<T>);

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct CancellableStream<T> {
   pub input: ParallelStream<T>,
   pub cancel: AntiFuture<()>,
}



pub struct FileUpload {
   pub input: ParallelStream<Vec<u8>>,
}
/*
pub struct InfiniteFuture {
   x: ParallelStream<InfiniteFuture>
}


Maybe streams should really just be things with callbacks to listen to? Then no async stuff
is implicated.

Could even make the type checking lazy/only implement meaningful things on that type when it
satisfies trait bounds. e.g.

mod received {
   struct ParallelStream<R, T>(R, TypedChannel<T>);
   impl<R, T> ParallelStream<R, T>
   where T: Receivable<R> {
      pub fn listen(self, f: impl FnMut(T::ReceivedType)) {
         self.0.listen(self.1, move |message| {
            // need to deserialize first?
            // ...that's probably its own step in the trait stack? i.e. "this is a channel that can be deserialized to..."
            f(self.0.receive())
         });
      }
   }

   struct AntiFuture<R, T>(R, TypedChannel<T>);
   impl<R, T> AntiFuture<R, T> {
      pub fn send<X: Sendable<R, T>(self, value: X);
   }
}

impl<R, T> Receivable<R> for ParallelFuture<T> {
   impl
}




*/