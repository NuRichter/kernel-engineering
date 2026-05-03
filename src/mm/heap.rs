// src/mm/heap.rs -- kernel heap: slab for small allocs, linked-list for large
// implements GlobalAlloc so alloc::* works in kernel

use core::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::LockedHeap;
use crate::mm::{vmm, pmm, slab::SLAB};

// kernel heap virtual address range
pub const HEAP_START: u64 = 0xFFFF_C000_0000_0000;
pub const HEAP_SIZE:  u64 = 8 * 1024 * 1024;   // 8 MiB initial

#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator;

static LLHEAP: LockedHeap = LockedHeap::empty();

struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // slab handles <= 2048 bytes; linked-list handles the rest
        if layout.size() <= 2048 {
            let ptr = SLAB.lock().alloc(layout);
            if !ptr.is_null() { return ptr; }
        }
        LLHEAP.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() <= 2048 {
            SLAB.lock().dealloc(ptr, layout);
        } else {
            LLHEAP.dealloc(ptr, layout);
        }
    }
}

pub fn init() {
    // map HEAP_SIZE bytes starting at HEAP_START
    let pages = (HEAP_SIZE as usize).div_ceil(pmm::PAGE_SIZE);

    for i in 0..pages {
        let va = HEAP_START + (i * pmm::PAGE_SIZE) as u64;
        let pa = pmm::alloc_frame().expect("heap: out of physical memory during init");
        vmm::map(va, pa, vmm::flags::KERNEL_RW);
    }

    unsafe {
        LLHEAP.lock().init(HEAP_START as *mut u8, HEAP_SIZE as usize);
    }
}
