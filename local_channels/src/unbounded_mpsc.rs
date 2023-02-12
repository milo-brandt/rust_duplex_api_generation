use std::{task::{Waker, Context, Poll}, cell::UnsafeCell, future::Future, pin::Pin, borrow::BorrowMut, collections::VecDeque};

// Backing data structure of unbounded MPSC.
struct Inner<T> {
   waker: Option<Waker>,
   entries: VecDeque<T>,
}
pub struct Channel<T> {
   inner: UnsafeCell<Inner<T>>
}
pub struct Receiver<'a, T>(&'a Channel<T>);
pub struct ReceiverFuture<'a, T>(&'a Channel<T>);
#[derive(Clone, Copy)]
pub struct Sender<'a, T>(&'a Channel<T>);
impl<T> Channel<T> {
   pub fn new() -> Self {
      Channel {
         inner: UnsafeCell::new(Inner {
            waker: None,
            entries: VecDeque::new()
         })
      }
   }
   pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>) {
      (Sender(self), Receiver(self))
   }
}
impl<'a, T> Sender<'a, T> {
   pub fn send(&self, value: T) {
      let inner = unsafe { &mut *self.0.inner.get() }; 
      inner.entries.push_back(value);
      if let Some(waker) = inner.waker.take() {
         waker.wake();
      }
   }
}
impl<'a, T> Receiver<'a, T> {
   pub fn receive<'b>(&'b mut self) -> ReceiverFuture<'b, T> {
      ReceiverFuture(self.0)
   }
}
impl<'a, T> Future for ReceiverFuture<'a, T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = unsafe { &mut *self.0.inner.get() }; 
        match inner.entries.pop_front() {
            Some(value) => Poll::Ready(value),
            None => {
               inner.waker = Some(cx.waker().clone());
               Poll::Pending
            },
        }
    }
}
impl<'a, T> Drop for ReceiverFuture<'a, T> {
    fn drop(&mut self) {
      unsafe { &mut *self.0.inner.get() }.waker = None;
    }
}

#[cfg(test)]
mod tests {
    use tokio::join;
    use assert_impl::assert_impl;

    use super::*;

    #[tokio::test]
    async fn test_future() {
        let mut channel = Channel::<u64>::new();
        let (sender, mut receiver) = channel.split();
        let ((value_1, value_2), _) = join! {
            async {
                let value_1 = receiver.receive().await;
                let value_2 = receiver.receive().await;
                (value_1, value_2)
            },
            async {
                sender.send(5);
                sender.send(6);
            },
        };
        assert_eq!(value_1, 5);
        assert_eq!(value_2, 6);
    }

    #[test]
    fn items_are_send() {
        assert_impl!(Send: Channel<u64>);
    }
}
