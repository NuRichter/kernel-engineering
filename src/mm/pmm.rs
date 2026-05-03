// src/mm/pmm.rs -- Physical Memory Manager
// bitmap-based frame allocator, 4 KiB frames
// initialized from multiboot2 memory map

use spin::Mutex;
use crate::{mb2, serial};

pub const PAGE_SIZE: usize = 4096;

// maximum supported physical memory: 4 GiB -> 1M frames -> 128 KiB bitmap
const MAX_FRAMES: usize = 1024 * 1024;
const BITMAP_LEN: usize = MAX_FRAMES / 64;

struct Pmm {
    bitmap:      [u64; BITMAP_LEN],
    total_frames: usize,
    free_frames:  usize,
    last_alloc:   usize,   // hint for next-fit search
}

impl Pmm {
    const fn new() -> Self {
        Self {
            bitmap:       [u64::MAX; BITMAP_LEN],  // all frames reserved by default
            total_frames: 0,
            free_frames:  0,
            last_alloc:   0,
        }
    }

    /// mark a physical page frame as free
    fn mark_free(&mut self, frame: usize) {
        if frame >= MAX_FRAMES { return; }
        let word = frame / 64;
        let bit  = frame % 64;
        if self.bitmap[word] & (1 << bit) != 0 {
            self.bitmap[word] &= !(1 << bit);
            self.free_frames  += 1;
        }
    }

    /// mark a physical page frame as used
    fn mark_used(&mut self, frame: usize) {
        if frame >= MAX_FRAMES { return; }
        let word = frame / 64;
        let bit  = frame % 64;
        if self.bitmap[word] & (1 << bit) == 0 {
            self.bitmap[word] |= 1 << bit;
            self.free_frames  -= 1;
        }
    }

    /// allocate one free frame, returns physical address or None
    fn alloc(&mut self) -> Option<u64> {
        let start = self.last_alloc;
        let len   = self.bitmap.len();

        for i in 0..len {
            let idx = (start + i) % len;
            let word = self.bitmap[idx];
            if word == u64::MAX { continue; }  // all used

            let bit = word.trailing_ones() as usize;
            self.bitmap[idx] |= 1 << bit;
            self.free_frames  -= 1;
            self.last_alloc    = idx;

            let frame = idx * 64 + bit;
            return Some((frame * PAGE_SIZE) as u64);
        }
        None
    }

    /// free a physical address back to the pool
    fn free(&mut self, phys: u64) {
        let frame = (phys / PAGE_SIZE as u64) as usize;
        self.mark_free(frame);
    }

    /// allocate n contiguous frames (naive linear scan)
    fn alloc_contig(&mut self, n: usize) -> Option<u64> {
        let mut run = 0;
        let mut start_frame = 0;

        for i in 0..MAX_FRAMES {
            let word = self.bitmap[i / 64];
            let bit  = i % 64;
            if word & (1 << bit) == 0 {
                if run == 0 { start_frame = i; }
                run += 1;
                if run == n {
                    // mark all as used
                    for f in start_frame..start_frame + n {
                        self.mark_used(f);
                    }
                    return Some((start_frame * PAGE_SIZE) as u64);
                }
            } else {
                run = 0;
            }
        }
        None
    }
}

static PMM: Mutex<Pmm> = Mutex::new(Pmm::new());

// multiboot2 mmap entry (entry_size >= 24)
#[repr(C, packed)]
struct MmapEntry {
    base: u64,
    len:  u64,
    typ:  u32,
    _attr: u32,
}

pub fn init(mb2: &mb2::Info) {
    let mut pmm = PMM.lock();
    pmm.total_frames = 0;

    let entry_size = mb2.mmap_entry_size as usize;
    let count      = mb2.mmap_count;

    for i in 0..count {
        let ptr = (mb2.mmap_ptr + (i * entry_size) as u64) as *const MmapEntry;
        let entry = unsafe { &*ptr };
        let typ   = unsafe { core::ptr::read_unaligned(&entry.typ) };
        let base  = unsafe { core::ptr::read_unaligned(&entry.base) };
        let len   = unsafe { core::ptr::read_unaligned(&entry.len) };

        if typ == 1 {
            // available RAM
            let start_frame = (base as usize).div_ceil(PAGE_SIZE);
            let end_frame   = ((base + len) as usize) / PAGE_SIZE;

            for frame in start_frame..end_frame {
                pmm.mark_free(frame);
                pmm.total_frames += 1;
            }
        }
    }

    // always protect frame 0 and kernel region (0x100000..0x400000)
    pmm.mark_used(0);
    for frame in 0x100..0x400 {
        pmm.mark_used(frame);
    }

    serial::println!(
        "[pmm ] {} total frames, {} free ({} KiB)",
        pmm.total_frames,
        pmm.free_frames,
        pmm.free_frames * 4
    );
}

pub fn alloc_frame() -> Option<u64> {
    PMM.lock().alloc()
}

pub fn free_frame(phys: u64) {
    PMM.lock().free(phys);
}

pub fn alloc_frames(n: usize) -> Option<u64> {
    PMM.lock().alloc_contig(n)
}

pub fn free_kib() -> usize {
    PMM.lock().free_frames * 4
}
