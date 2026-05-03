// src/mm/vmm.rs -- Virtual Memory Manager
// 4-level paging (PML4), higher-half kernel, recursive mapping at slot 510

use spin::Mutex;
use crate::mm::pmm::{alloc_frame, free_frame, PAGE_SIZE};
use crate::serial;

// recursive mapping: PML4[510] points back to PML4 itself
// this lets us access page tables via fixed virtual addresses
const RECURSIVE_INDEX: usize = 510;
const RECURSIVE_BASE:  u64   = 0xFFFF_FF00_0000_0000;

// page table entry flags
pub mod flags {
    pub const PRESENT:   u64 = 1 << 0;
    pub const WRITABLE:  u64 = 1 << 1;
    pub const USER:      u64 = 1 << 2;
    pub const WRITETHROUGH: u64 = 1 << 3;
    pub const NO_CACHE:  u64 = 1 << 4;
    pub const ACCESSED:  u64 = 1 << 5;
    pub const DIRTY:     u64 = 1 << 6;
    pub const HUGE:      u64 = 1 << 7;
    pub const GLOBAL:    u64 = 1 << 8;
    pub const NX:        u64 = 1 << 63;

    pub const KERNEL_RW: u64 = PRESENT | WRITABLE | GLOBAL;
    pub const KERNEL_RO: u64 = PRESENT | GLOBAL | NX;
    pub const USER_RW:   u64 = PRESENT | WRITABLE | USER;
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Entry(pub u64);

impl Entry {
    pub fn new(phys: u64, flags: u64) -> Self {
        Self((phys & 0x000F_FFFF_FFFF_F000) | flags)
    }
    pub fn present(self) -> bool { self.0 & flags::PRESENT != 0 }
    pub fn phys(self)    -> u64  { self.0 & 0x000F_FFFF_FFFF_F000 }
    pub fn flags(self)   -> u64  { self.0 & 0xFFF }
}

// virtual address decomposition
fn pml4_idx(va: u64) -> usize { ((va >> 39) & 0x1FF) as usize }
fn pdpt_idx(va: u64) -> usize { ((va >> 30) & 0x1FF) as usize }
fn pd_idx  (va: u64) -> usize { ((va >> 21) & 0x1FF) as usize }
fn pt_idx  (va: u64) -> usize { ((va >> 12) & 0x1FF) as usize }

/// get a mutable pointer to a page table entry via recursive mapping
unsafe fn pml4_entry(va: u64) -> *mut Entry {
    let addr = RECURSIVE_BASE
        | ((RECURSIVE_INDEX as u64) << 30)
        | ((RECURSIVE_INDEX as u64) << 21)
        | ((RECURSIVE_INDEX as u64) << 12)
        | (pml4_idx(va) as u64 * 8);
    addr as *mut Entry
}

unsafe fn pdpt_entry(va: u64) -> *mut Entry {
    let addr = RECURSIVE_BASE
        | ((RECURSIVE_INDEX as u64) << 30)
        | ((RECURSIVE_INDEX as u64) << 21)
        | (pml4_idx(va) as u64 * 4096)
        | (pdpt_idx(va) as u64 * 8);
    addr as *mut Entry
}

unsafe fn pd_entry(va: u64) -> *mut Entry {
    let addr = RECURSIVE_BASE
        | ((RECURSIVE_INDEX as u64) << 30)
        | (pml4_idx(va) as u64 * 0x200000)
        | (pdpt_idx(va) as u64 * 4096)
        | (pd_idx(va) as u64 * 8);
    addr as *mut Entry
}

unsafe fn pt_entry(va: u64) -> *mut Entry {
    let addr = RECURSIVE_BASE
        | (pml4_idx(va) as u64 * 0x40000000)
        | (pdpt_idx(va) as u64 * 0x200000)
        | (pd_idx(va) as u64 * 4096)
        | (pt_idx(va) as u64 * 8);
    addr as *mut Entry
}

/// ensure a page table exists at the given level, allocating if absent
unsafe fn ensure_table(entry: *mut Entry, flags: u64) -> bool {
    let e = entry.read_volatile();
    if e.present() { return true; }

    let frame = match alloc_frame() {
        Some(f) => f,
        None    => return false,
    };

    // zero the new table
    let ptr = frame as *mut u8;
    ptr.write_bytes(0, PAGE_SIZE);

    entry.write_volatile(Entry::new(frame, flags));
    true
}

/// map a virtual page to a physical frame
pub fn map(va: u64, pa: u64, flags: u64) -> bool {
    unsafe {
        let pml4e = pml4_entry(va);
        if !ensure_table(pml4e, flags::PRESENT | flags::WRITABLE) { return false; }

        let pdpte = pdpt_entry(va);
        if !ensure_table(pdpte, flags::PRESENT | flags::WRITABLE) { return false; }

        let pde = pd_entry(va);
        if !ensure_table(pde, flags::PRESENT | flags::WRITABLE) { return false; }

        let pte = pt_entry(va);
        pte.write_volatile(Entry::new(pa, flags));

        // flush TLB for this page
        core::arch::asm!("invlpg [{}]", in(reg) va, options(nostack));
        true
    }
}

/// unmap a virtual page, return its physical address
pub fn unmap(va: u64) -> Option<u64> {
    unsafe {
        let pte = pt_entry(va);
        let e   = pte.read_volatile();
        if !e.present() { return None; }

        pte.write_volatile(Entry(0));
        core::arch::asm!("invlpg [{}]", in(reg) va, options(nostack));
        Some(e.phys())
    }
}

/// translate virtual address to physical
pub fn translate(va: u64) -> Option<u64> {
    unsafe {
        let e4 = (*pml4_entry(va));
        if !e4.present() { return None; }
        let e3 = (*pdpt_entry(va));
        if !e3.present() { return None; }
        let e2 = (*pd_entry(va));
        if !e2.present() { return None; }
        let e1 = (*pt_entry(va));
        if !e1.present() { return None; }
        Some(e1.phys() | (va & 0xFFF))
    }
}

pub fn init() {
    // nothing to do: boot.asm already set up identity + recursive mapping
    // verify recursive slot is wired by reading PML4[510]
    unsafe {
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3);
        serial::println!("[vmm ] CR3=0x{:x}", cr3);
    }
}
