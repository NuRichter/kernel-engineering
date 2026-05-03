// src/ipc/mod.rs -- Inter-Process Communication
// provides: message queues, anonymous pipes, shared memory handles

mod msgq;
mod pipe;

pub use msgq::{MsgQueue, Message};
pub use pipe::Pipe;

use crate::serial;

pub fn init() {
    // nothing global to init; queues are per-process objects
    serial::println!("[ipc ] message queue + pipe subsystem ready");
}
