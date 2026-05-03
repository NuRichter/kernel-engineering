// src/mm/slab.rs -- slab allocator for fixed-size kernel objects
// each slab is one 4 KiB page; slabs are chained per size class

use core::{mem, ptr, alloc::Layout};
use spin::Mutex;
use crate::mm::{pmm, vmm, heap::HEAP_START};

// size classes (bytes): 8, 16, 32, 64, 128, 256, 512, 1024, 2048
const SIZE_CLASSES: [usize; 9] = [8, 16, 32, 64, 128, 256, 512, 1024, 2048];
const N_CLASSES:    usize       = SIZE_CLASSES.len();

#[repr(C)]
struct FreeNode {
    next: *mut FreeNode,
}

struct SlabClass {
    obj_size:  usize,
    free_list: *mut FreeNode,
    slab_head: *mut SlabHeader,
}

unsafe impl Send for SlabClass {}

#[repr(C)]
struct SlabHeader {
    next:    *mut SlabHeader,
    obj_size: usize,
    used:    usize,
    cap:     usize,
}

impl SlabClass {
    const fn new(obj_size: usize) -> Self {
        Self { obj_size, free_list: ptr::null_mut(), slab_head: ptr::null_mut() }
    }

    /// grow by adding a fresh slab page
    unsafe fn grow(&mut self) -> bool {
        let phys = match pmm::alloc_frame() {
            Some(p) => p,
            None    => return false,
        };

        // map at next available heap VA (simple bump for slab pages)
        static SLAB_VA: Mutex<u64> = Mutex::new(HEAP_START + 4 * 1024 * 1024);
        let va = {
            let mut v = SLAB_VA.lock();
            let addr  = *v;
            *v       += pmm::PAGE_SIZE as u64;
            addr
        };

        vmm::map(va, phys, vmm::flags::KERNEL_RW);
        let page = va as *mut u8;
        page.write_bytes(0, pmm::PAGE_SIZE);

        let header_size = mem::size_of::<SlabHeader>().next_multiple_of(self.obj_size);
        let obj_size    = self.obj_size;
        let cap         = (pmm::PAGE_SIZE - header_size) / obj_size;

        let header = page as *mut SlabHeader;
        (*header).next     = self.slab_head;
        (*header).obj_size = obj_size;
        (*header).used     = 0;
        (*header).cap      = cap;
        self.slab_head     = header;

        // push all free slots onto free_list
        let data = page.add(header_size);
        for i in (0..cap).rev() {
            let node = data.add(i * obj_size) as *mut FreeNode;
            (*node).next  = self.free_list;
            self.free_list = node;
        }
        true
    }

    unsafe fn alloc(&mut self) -> *mut u8 {
        if self.free_list.is_null() && !self.grow() {
            return ptr::null_mut();
        }
        let node = self.free_list;
        self.free_list = (*node).next;
        node as *mut u8
    }

    unsafe fn dealloc(&mut self, ptr: *mut u8) {
        let node = ptr as *mut FreeNode;
        (*node).next   = self.free_list;
        self.free_list = node;
    }
}

struct SlabAllocator {
    classes: [SlabClass; N_CLASSES],
}

unsafe impl Send for SlabAllocator {}

impl SlabAllocator {
    const fn new() -> Self {
        Self {
            classes: [
                SlabClass::new(8),    SlabClass::new(16),
                SlabClass::new(32),   SlabClass::new(64),
                SlabClass::new(128),  SlabClass::new(256),
                SlabClass::new(512),  SlabClass::new(1024),
                SlabClass::new(2048),
            ],
        }
    }

    fn class_for(&mut self, size: usize) -> Option<&mut SlabClass> {
        for (i, &s) in SIZE_CLASSES.iter().enumerate() {
            if size <= s { return Some(&mut self.classes[i]); }
        }
        None
    }

    pub unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align());
        match self.class_for(size) {
            Some(cls) => cls.alloc(),
            None      => ptr::null_mut(),   // fallback to heap for large allocs
        }
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(layout.align());
        if let Some(cls) = self.class_for(size) {
            cls.dealloc(ptr);
        }
        // large allocations: no-op dealloc for now (TODO: buddy system)
    }
}

pub static SLAB: Mutex<SlabAllocator> = Mutex::new(SlabAllocator::new());
