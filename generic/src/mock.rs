use std::{collections::HashMap, rc::Rc, cell::RefCell};

use crate::{ChannelSender, ChannelReceiver, Channel};

#[derive(Default)]
struct SharedState {
   channel_count: u64,
   callbacks: HashMap<u64, Box<dyn FnMut(String)>>
}
struct Sender(Rc<RefCell<SharedState>>);
struct Receiver(Rc<RefCell<SharedState>>);

struct Duplex<S, R>(S, R);

impl ChannelSender for Sender {
    fn allocate_send_channel(&self) -> Channel {
      let mut shared_state = self.0.borrow_mut();
      let next_channel = shared_state.channel_count;
      shared_state.channel_count += 1;
      Channel(next_channel)
    }
    fn send_message(&self, channel: Channel, message: String) {
      // TODO: Debug misses?
      let mut shared_state = self.0.borrow_mut();
      shared_state.callbacks.get_mut(&channel.0).map(|callback|
         callback(message)
      );
   }
}
impl ChannelReceiver for Receiver {
   fn allocate_receive_channel(&self) -> Channel {
      let mut shared_state = self.0.borrow_mut();
      let next_channel = shared_state.channel_count;
      shared_state.channel_count += 1;
      Channel(next_channel)
   }
   fn listen(&self, channel: Channel, listener: impl FnMut(String) + 'static) {
         // TODO: Debug double setting?
         let mut shared_state = self.0.borrow_mut();
         shared_state.callbacks.insert(channel.0, Box::new(listener));
    }
}

impl<S: ChannelSender, R> ChannelSender for Duplex<S, R> {
    fn allocate_send_channel(&self) -> Channel {
        self.0.allocate_send_channel()
    }

    fn send_message(&self, channel: Channel, message: String) {
        self.0.send_message(channel, message)
    }
}
impl<S, R: ChannelReceiver> ChannelReceiver for Duplex<S, R> {
   fn allocate_receive_channel(&self) -> Channel {
       self.1.allocate_receive_channel()
   }
    fn listen(&self, channel: Channel, listener: impl FnMut(String) + 'static) {
        self.1.listen(channel, listener)
    }
}

pub fn simple_connection() -> (impl ChannelSender, impl ChannelReceiver) {
   let shared_state = Rc::new(RefCell::new(SharedState::default()));
   (Sender(shared_state.clone()), Receiver(shared_state))
}

pub fn simple_duplex_connection() -> (impl ChannelSender + ChannelReceiver, impl ChannelSender + ChannelReceiver) {
   let (sender_1, receiver_2) = simple_connection();
   let (sender_2, receiver_1) = simple_connection();
   (Duplex(sender_1, receiver_1), Duplex(sender_2, receiver_2))
}


#[cfg(test)]
mod tests {
   use std::cell::Cell;

use super::*;

    #[test]
    fn simple_connections_can_send_messages() {
         let (sender, receiver) = simple_connection();
         let channel = sender.allocate_send_channel();
         let received = Rc::new(Cell::new(String::new()));
         receiver.listen(channel, {
            let received = received.clone();
            move |message| received.set(message)
         });
         assert_eq!(received.replace(String::new()), String::new()); 
         sender.send_message(channel, String::from("Hello!"));
         assert_eq!(received.replace(String::new()), String::from("Hello!")); 
         sender.send_message(channel, String::from("Bye!"));
         assert_eq!(received.replace(String::new()), String::from("Bye!")); 
    }

}