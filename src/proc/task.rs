// src/proc/task.rs -- Task Control Block (TCB) and kernel thread management

use alloc::{boxed::Box, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use crate::mm::{pmm, vmm};

static NEXT_PID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskState {
    Running,
    Ready,
    Blocked,
    Zombie,
}

// saved register state for context switch
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CpuContext {
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub rbx: u64, pub rbp: u64,
    pub rsp: u64, pub rip: u64,
    pub rflags: u64,
}

const KSTACK_PAGES: usize = 4;
const KSTACK_SIZE:  usize = KSTACK_PAGES * pmm::PAGE_SIZE;

pub struct Task {
    pub pid:     u64,
    pub ppid:    u64,
    pub state:   TaskState,
    pub ctx:     CpuContext,
    pub kstack:  u64,        // top of kernel stack (virtual)
    pub name:    &'static str,
    pub ticks:   u64,        // total scheduler ticks consumed
    pub priority: i8,        // -20..19 (nice value)
}

impl Task {
    pub fn new_kernel(entry: fn() -> !, name: &'static str, priority: i8) -> Box<Task> {
        let pid   = NEXT_PID.fetch_add(1, Ordering::Relaxed);
        let stack = alloc_kstack();
        let top   = stack + KSTACK_SIZE as u64;

        let mut ctx = CpuContext::default();
        ctx.rip    = entry as u64;
        ctx.rsp    = top - 8;     // 16-byte alignment
        ctx.rflags = 0x202;       // IF set

        Box::new(Task {
            pid, ppid: 0, state: TaskState::Ready, ctx,
            kstack: stack, name, ticks: 0, priority,
        })
    }
}

fn alloc_kstack() -> u64 {
    // very simple: grab contiguous physical pages, map to a VA in kernel heap area
    static STACK_VA_BUMP: Mutex<u64> = Mutex::new(0xFFFF_E000_0000_0000);

    let phys = pmm::alloc_frames(KSTACK_PAGES)
        .expect("proc: out of memory for kernel stack");

    let va = {
        let mut bump = STACK_VA_BUMP.lock();
        let addr = *bump;
        *bump   += KSTACK_SIZE as u64 + pmm::PAGE_SIZE as u64;  // guard page gap
        addr
    };

    for i in 0..KSTACK_PAGES {
        let page_va = va + (i * pmm::PAGE_SIZE) as u64;
        let page_pa = phys + (i * pmm::PAGE_SIZE) as u64;
        vmm::map(page_va, page_pa, vmm::flags::KERNEL_RW);
    }

    va
}

static ALL_TASKS: Mutex<Vec<Box<Task>>> = Mutex::new(Vec::new());

pub fn init() {
    // nothing extra; idle task created by scheduler::init
}

pub fn register(task: Box<Task>) {
    ALL_TASKS.lock().push(task);
}
