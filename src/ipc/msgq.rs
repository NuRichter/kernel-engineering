// src/ipc/msgq.rs -- kernel message queues (POSIX mq_* subset)

use alloc::{collections::VecDeque, vec::Vec};
use spin::Mutex;

pub const MAX_MSG_SIZE: usize = 512;
pub const MAX_QUEUE_DEPTH: usize = 64;

#[derive(Clone)]
pub struct Message {
    pub priority: u32,
    pub data:     Vec<u8>,
}

pub struct MsgQueue {
    inner: Mutex<MsgQueueInner>,
}

struct MsgQueueInner {
    queue: VecDeque<Message>,
    cap:   usize,
}

impl MsgQueue {
    pub fn new(cap: usize) -> Self {
        let cap = cap.min(MAX_QUEUE_DEPTH);
        Self {
            inner: Mutex::new(MsgQueueInner { queue: VecDeque::new(), cap }),
        }
    }

    /// send a message; returns Err if full
    pub fn send(&self, msg: Message) -> Result<(), &'static str> {
        if msg.data.len() > MAX_MSG_SIZE { return Err("message too large"); }

        let mut inner = self.inner.lock();
        if inner.queue.len() >= inner.cap { return Err("queue full"); }

        // insert in priority order (highest priority first)
        let pos = inner.queue.partition_point(|m| m.priority >= msg.priority);
        inner.queue.insert(pos, msg);
        Ok(())
    }

    /// receive the highest priority message, None if empty
    pub fn recv(&self) -> Option<Message> {
        self.inner.lock().queue.pop_front()
    }

    pub fn len(&self) -> usize { self.inner.lock().queue.len() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}
