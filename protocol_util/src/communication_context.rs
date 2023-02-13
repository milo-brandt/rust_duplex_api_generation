use std::sync::Arc;

use futures::channel::mpsc;

use crate::receiver::ListenerController;
use crate::channel_allocator::TypedChannelAllocator;
use crate::sender::Sender;

#[derive(Clone)]
pub struct Context {
    pub channel_allocator: Arc<TypedChannelAllocator>,
    pub controller: ListenerController,
    pub sender: Sender
}