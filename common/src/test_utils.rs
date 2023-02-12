/*
use std::{mem, sync::{Arc, Mutex}};

use crate::{
    endpoint::{DuplexEndpoint, Sender},
    generic::{InChannel, OutChannel},
};

#[derive(Debug, Copy, Clone)]
pub enum MockDuplexEnd {
    Local,
    Remote,
}

pub struct MockMessage {
    target: MockDuplexEnd,
    channel: u64,
    message: String,
}
#[derive(Default)]
struct MockDuplexChannelShared {
    messages: Vec<MockMessage>,
}
pub struct MockDuplexChannelSender {
    target: MockDuplexEnd,
    inner: Arc<Mutex<MockDuplexChannelShared>>,
}
pub struct MockDuplexChannel {
    inner: Arc<Mutex<MockDuplexChannelShared>>,
    pub local: DuplexEndpoint<MockDuplexChannelSender>,
    pub remote: DuplexEndpoint<MockDuplexChannelSender>,
}

impl Sender for MockDuplexChannelSender {
    fn send(&self, channel: u64, message: String) {
        self.inner.lock().unwrap().messages.push(MockMessage {
            target: self.target,
            channel,
            message,
        })
    }
}

impl MockDuplexChannel {
    pub fn new() -> Self {
        let inner = Arc::new(Mutex::new(Default::default()));
        Self {
            inner: inner.clone(),
            local: DuplexEndpoint::new(MockDuplexChannelSender {
                target: MockDuplexEnd::Remote,
                inner: inner.clone(),
            }),
            remote: DuplexEndpoint::new(MockDuplexChannelSender {
                target: MockDuplexEnd::Local,
                inner,
            }),
        }
    }
    pub fn flush(&self) {
        while !self.inner.lock().unwrap().messages.is_empty() {
            let messages = mem::take(&mut self.inner.lock().unwrap().messages);
            for message in messages {
                match message.target {
                    MockDuplexEnd::Local => &self.local,
                    MockDuplexEnd::Remote => &self.remote,
                }
                .receive(message.channel, message.message)
            }
        }
    }
    pub fn out_channel<Local, Remote>(&self) -> (OutChannel<Local>, InChannel<Remote>) {
        let local_side = self.local.out_channel();
        let index = local_side.0;
        (local_side, InChannel(index, Default::default()))
    }
    pub fn in_channel<Local, Remote>(&self) -> (InChannel<Local>, OutChannel<Remote>) {
        let local_side = self.local.in_channel();
        let index = local_side.0;
        (local_side, OutChannel(index, Default::default()))
    }
}

#[cfg(test)]
mod test {
    use std::{ops::Deref, sync::{mpsc::channel, Mutex, Arc}};

    use serde::{Serialize, Deserialize};

    use super::*;

    #[test]
    pub fn send_message() {
        // This fails because we're locking within the lock.
        let mut channel = MockDuplexChannel::new();

        let (local_channel, remote_channel) = channel.out_channel::<u64, u64>();

        let values = Arc::new(Mutex::new(Vec::new()));

        channel.remote.listen(remote_channel, {
            let values = values.clone();
            move |message| values.lock().unwrap().push(message)
        });

        assert_eq!(values.lock().unwrap().deref(), &Vec::<u64>::new());

        channel.local.send(&local_channel, 5);

        assert_eq!(values.lock().unwrap().deref(), &Vec::<u64>::new());

        channel.flush();

        assert_eq!(values.lock().unwrap().deref(), &vec![5]);
    }

    #[test]
    pub fn countdown() {
        let values = Arc::new(Mutex::new(Vec::new()));

        let channel = Arc::new(Mutex::new(MockDuplexChannel::new()));
       
        let (local_tx, remote_rx) = channel.lock().unwrap().out_channel::<i32, i32>();
        let (local_rx, remote_tx) = channel.lock().unwrap().in_channel::<i32, i32>();

        channel.lock().unwrap().remote.listen(remote_rx, {
            let values = values.clone();
            let channel = channel.clone();
            move |message| {
                values.lock().unwrap().push(message);
                if message > 0 {
                    channel.lock().unwrap().remote.send(&remote_tx, message - 2);
                }
            }
        });
        channel.lock().unwrap().local.listen(local_rx, {
            let values = values.clone();
            let channel = channel.clone();
            let local_tx = local_tx.clone();
            move |message| {
                values.lock().unwrap().push(message);
                if message > 0 {
                    channel.lock().unwrap().local.send(&local_tx, message - 1);
                }
            }
        });

        channel.lock().unwrap().local.send(&local_tx, 9);
        channel.lock().unwrap().flush();

        assert_eq!(values.lock().unwrap().deref(), &vec![9, 7, 6, 4, 3, 1, 0]);
    }

    #[derive(Serialize)]
    struct OutValueWithFuture {
      input: u64,
      output: InChannel<u64>,
    }
    #[derive(Deserialize)]
    struct InValueWithFuture {
      input: u64,
      output: OutChannel<u64>,
    }

    #[test]
    pub fn future() {
      let value = Arc::new(Mutex::new(None));

      let channel = Arc::new(Mutex::new(MockDuplexChannel::new()));
            
      let (local_channel, remote_channel) = channel.lock().unwrap().out_channel::<OutValueWithFuture, InValueWithFuture>();
      channel.lock().unwrap().remote.listen(remote_channel, {
         let channel = channel.clone();
         move |message| {
            channel.lock().unwrap().remote.send(&message.output, message.input * message.input);
         }
      });

      let receiver = channel.lock().unwrap().local.in_channel();
      channel.lock().unwrap().local.send(&local_channel, OutValueWithFuture {
         input: 17,
         output: receiver.clone(),
      });
      channel.lock().unwrap().local.listen(receiver, {
         let value = value.clone();
         move |message| {
            *value.lock().unwrap() = Some(message);
         }
      });

      channel.lock().unwrap().flush();

      assert_eq!(value.lock().unwrap().deref(), &Some(289));
   }
}
*/