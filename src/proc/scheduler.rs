// src/proc/scheduler.rs -- preemptive round-robin scheduler with nice priorities
// context switch written in inline asm to avoid Rust register assumptions

use alloc::{collections::VecDeque, boxed::Box};
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::proc::task::{Task, TaskState, CpuContext};
use crate::serial;

static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

// ready queue -- priority queue via separate buckets (40 levels, nice -20..19)
const PRIO_LEVELS: usize = 40;

struct RunQueue {
    buckets:  [VecDeque<Box<Task>>; PRIO_LEVELS],
    current:  Option<Box<Task>>,
    idle:     Option<Box<Task>>,
}

unsafe impl Send for RunQueue {}

impl RunQueue {
    fn push(&mut self, task: Box<Task>) {
        let idx = (task.priority + 20).clamp(0, 39) as usize;
        self.buckets[idx].push_back(task);
    }

    fn pop_highest(&mut self) -> Option<Box<Task>> {
        for bucket in &mut self.buckets {
            if let Some(t) = bucket.pop_front() { return Some(t); }
        }
        None
    }
}

// runqueue is not trivially constructible at compile time with VecDeque
// use Once to init at runtime
static RQ: Mutex<Option<RunQueue>> = Mutex::new(None);

pub fn init() {
    let idle = Task::new_kernel(idle_fn, "idle", 19);

    // SAFETY: constructing VecDeque array at runtime
    let buckets: [VecDeque<Box<Task>>; PRIO_LEVELS] =
        core::array::from_fn(|_| VecDeque::new());

    let mut rq = RunQueue {
        buckets,
        current: None,
        idle:    Some(idle),
    };

    *RQ.lock() = Some(rq);
}

pub fn spawn(task: Box<Task>) {
    if let Some(rq) = RQ.lock().as_mut() {
        rq.push(task);
    }
}

/// called from timer IRQ handler (vector 32)
pub fn tick() {
    TICK_COUNT.fetch_add(1, Ordering::Relaxed);

    // every 10 ms (assuming 100 Hz PIT): preempt if another task is ready
    let tick = TICK_COUNT.load(Ordering::Relaxed);
    if tick % 10 != 0 { return; }

    schedule();
}

fn schedule() {
    let mut guard = RQ.lock();
    let rq = match guard.as_mut() {
        Some(r) => r,
        None    => return,
    };

    let mut next = match rq.pop_highest() {
        Some(t) => t,
        None    => return,  // only idle running, nothing to switch to
    };

    let prev = match rq.current.take() {
        Some(mut p) => {
            p.state = TaskState::Ready;
            p
        }
        None => return,
    };

    next.state    = TaskState::Running;
    next.ticks   += 1;

    // do the context switch
    let prev_ctx = &prev.ctx as *const CpuContext as *mut CpuContext;
    let next_ctx = &next.ctx as *const CpuContext;

    rq.push(prev);           // re-queue the previous task
    rq.current = Some(next);

    drop(guard);             // release lock before switch

    unsafe { context_switch(prev_ctx, next_ctx) };
}

// context switch: save current regs into *prev_ctx, restore from *next_ctx
// clobbers nothing that Rust expects preserved (callee-saved: rbx rbp r12-r15)
#[naked]
unsafe extern "C" fn context_switch(prev: *mut CpuContext, next: *const CpuContext) {
    core::arch::naked_asm!(
        // save callee-saved registers + rsp + rflags into *rdi (prev)
        "mov [rdi + 0*8], r15",
        "mov [rdi + 1*8], r14",
        "mov [rdi + 2*8], r13",
        "mov [rdi + 3*8], r12",
        "mov [rdi + 4*8], rbx",
        "mov [rdi + 5*8], rbp",
        "mov [rdi + 6*8], rsp",
        "lea rax, [rip + 1f]",
        "mov [rdi + 7*8], rax",   // save return address as rip
        "pushfq",
        "pop qword ptr [rdi + 8*8]",

        // restore from *rsi (next)
        "mov r15, [rsi + 0*8]",
        "mov r14, [rsi + 1*8]",
        "mov r13, [rsi + 2*8]",
        "mov r12, [rsi + 3*8]",
        "mov rbx, [rsi + 4*8]",
        "mov rbp, [rsi + 5*8]",
        "mov rsp, [rsi + 6*8]",
        "push qword ptr [rsi + 8*8]",
        "popfq",
        "jmp qword ptr [rsi + 7*8]",

        "1:",
        "ret",
    );
}

pub fn start() -> ! {
    {
        let mut guard = RQ.lock();
        let rq = guard.as_mut().unwrap();
        let mut idle = rq.idle.take().unwrap();
        idle.state   = TaskState::Running;
        rq.current   = Some(idle);
    }

    serial::println!("[sched] running");

    // enable interrupts -- timer will drive preemption from here
    unsafe { core::arch::asm!("sti") };

    loop { unsafe { core::arch::asm!("hlt") }; }
}

pub fn total_ticks() -> u64 {
    TICK_COUNT.load(Ordering::Relaxed)
}

fn idle_fn() -> ! {
    loop { unsafe { core::arch::asm!("hlt") }; }
}
