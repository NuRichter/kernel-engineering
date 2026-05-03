// src/kernel/gdt.rs -- GDT + TSS management
// asm stubs live in arch/x86_64/gdt.asm

use core::mem::size_of;
use spin::Once;

extern "C" {
    fn gdt_load();
    fn tss_load(base: u64, selector: u16);
}

// TSS - Task State Segment (x86_64 layout)
#[repr(C, packed)]
pub struct Tss {
    _reserved0:   u32,
    pub rsp:      [u64; 3],   // RSP0..2 (kernel stacks for ring transitions)
    _reserved1:   u64,
    pub ist:      [u64; 7],   // Interrupt Stack Table
    _reserved2:   u64,
    _reserved3:   u16,
    pub iopb_off: u16,
}

impl Tss {
    pub const fn zero() -> Self {
        Self {
            _reserved0: 0,
            rsp:        [0; 3],
            _reserved1: 0,
            ist:        [0; 7],
            _reserved2: 0,
            _reserved3: 0,
            iopb_off:   size_of::<Tss>() as u16,
        }
    }
}

// 4 KiB interrupt stack for IST1
static mut IST_STACK: [u8; 4096] = [0; 4096];

static mut TSS: Tss = Tss::zero();

static GDT_INIT: Once<()> = Once::new();

pub fn init() {
    GDT_INIT.call_once(|| unsafe {
        // point IST[0] to top of stack
        TSS.ist[0] = IST_STACK.as_ptr() as u64 + 4096;

        let tss_base = &TSS as *const Tss as u64;

        // load GDT (already embedded in .data via gdt.asm, just lgdt)
        gdt_load();

        // slot for TSS descriptor is at offset 0x28 (5th entry, 8 bytes each)
        tss_load(tss_base, 0x28);
    });
}

// Selector constants (matching gdt.asm layout)
pub mod sel {
    pub const KERNEL_CODE: u16 = 0x08;
    pub const KERNEL_DATA: u16 = 0x10;
    pub const USER_CODE:   u16 = 0x18 | 3;
    pub const USER_DATA:   u16 = 0x20 | 3;
    pub const TSS:         u16 = 0x28;
}
