// src/proc/mod.rs -- process subsystem

pub mod scheduler;
pub mod task;
pub mod elf;

use crate::serial;

pub fn init() {
    task::init();
    scheduler::init();
    serial::println!("[proc] idle task created");
}
