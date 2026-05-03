// src/kernel/idt.rs -- IDT setup + interrupt dispatcher
// raw entry stubs live in arch/x86_64/idt.asm

use core::mem::size_of;
use spin::Once;
use crate::serial;

extern "C" {
    fn idt_load(idtr: *const Idtr);

    // exception stubs
    fn isr0();  fn isr1();  fn isr2();  fn isr3();
    fn isr4();  fn isr5();  fn isr6();  fn isr7();
    fn isr8();  fn isr9();  fn isr10(); fn isr11();
    fn isr12(); fn isr13(); fn isr14(); fn isr15();
    fn isr16(); fn isr17(); fn isr18(); fn isr19();
    fn isr20(); fn isr21(); fn isr22(); fn isr23();
    fn isr24(); fn isr25(); fn isr26(); fn isr27();
    fn isr28(); fn isr29(); fn isr30(); fn isr31();

    // IRQ stubs
    fn irq32(); fn irq33(); fn irq34(); fn irq35();
    fn irq36(); fn irq37(); fn irq38(); fn irq39();
    fn irq40(); fn irq41(); fn irq42(); fn irq43();
    fn irq44(); fn irq45(); fn irq46(); fn irq47();
}

// gate descriptor (128-bit, packed)
#[derive(Clone, Copy)]
#[repr(C, packed)]
struct Gate {
    off_low:   u16,
    selector:  u16,
    ist:       u8,
    attr:      u8,
    off_mid:   u16,
    off_high:  u32,
    _zero:     u32,
}

impl Gate {
    const fn null() -> Self {
        Self { off_low: 0, selector: 0, ist: 0, attr: 0, off_mid: 0, off_high: 0, _zero: 0 }
    }

    fn new(handler: u64, selector: u16, ist: u8, attr: u8) -> Self {
        Self {
            off_low:  (handler & 0xFFFF) as u16,
            selector,
            ist,
            attr,
            off_mid:  ((handler >> 16) & 0xFFFF) as u16,
            off_high: (handler >> 32) as u32,
            _zero:    0,
        }
    }
}

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base:  u64,
}

const IDT_LEN: usize = 256;

static mut IDT: [Gate; IDT_LEN] = [Gate::null(); IDT_LEN];

// interrupt stack frame passed from asm stub
#[repr(C)]
pub struct InterruptFrame {
    // saved by isr_common_stub (in reverse push order)
    pub r15: u64, pub r14: u64, pub r13: u64, pub r12: u64,
    pub r11: u64, pub r10: u64, pub r9:  u64, pub r8:  u64,
    pub rbp: u64, pub rdi: u64, pub rsi: u64, pub rdx: u64,
    pub rcx: u64, pub rbx: u64, pub rax: u64,
    // pushed by stub / CPU
    pub int_num:   u64,
    pub error_code: u64,
    // pushed by CPU on interrupt
    pub rip:    u64,
    pub cs:     u64,
    pub rflags: u64,
    pub rsp:    u64,
    pub ss:     u64,
}

const INT_GATE: u8 = 0x8E;  // present | DPL=0 | interrupt gate
const TRAP_GATE: u8 = 0x8F; // present | DPL=0 | trap gate

static IDT_INIT: Once<()> = Once::new();

pub fn init() {
    IDT_INIT.call_once(|| unsafe {
        let stubs: [u64; 48] = [
            isr0  as u64, isr1  as u64, isr2  as u64, isr3  as u64,
            isr4  as u64, isr5  as u64, isr6  as u64, isr7  as u64,
            isr8  as u64, isr9  as u64, isr10 as u64, isr11 as u64,
            isr12 as u64, isr13 as u64, isr14 as u64, isr15 as u64,
            isr16 as u64, isr17 as u64, isr18 as u64, isr19 as u64,
            isr20 as u64, isr21 as u64, isr22 as u64, isr23 as u64,
            isr24 as u64, isr25 as u64, isr26 as u64, isr27 as u64,
            isr28 as u64, isr29 as u64, isr30 as u64, isr31 as u64,
            irq32 as u64, irq33 as u64, irq34 as u64, irq35 as u64,
            irq36 as u64, irq37 as u64, irq38 as u64, irq39 as u64,
            irq40 as u64, irq41 as u64, irq42 as u64, irq43 as u64,
            irq44 as u64, irq45 as u64, irq46 as u64, irq47 as u64,
        ];

        for (i, &stub) in stubs.iter().enumerate() {
            // IST1 for double fault (vector 8), rest use IST0
            let ist = if i == 8 { 1 } else { 0 };
            IDT[i] = Gate::new(stub, crate::kernel::gdt::sel::KERNEL_CODE, ist, INT_GATE);
        }

        let idtr = Idtr {
            limit: (size_of::<[Gate; IDT_LEN]>() - 1) as u16,
            base:  IDT.as_ptr() as u64,
        };

        idt_load(&idtr);

        // enable interrupts
        core::arch::asm!("sti");
    });
}

// called from idt.asm isr_common_stub
#[no_mangle]
pub extern "C" fn interrupt_dispatch(frame: *mut InterruptFrame) {
    let frame = unsafe { &mut *frame };
    let num   = frame.int_num;

    match num {
        // page fault
        14 => {
            let cr2: u64;
            unsafe { core::arch::asm!("mov {}, cr2", out(reg) cr2) };
            serial::println!(
                "[pf  ] #PF at RIP=0x{:x} CR2=0x{:x} err=0x{:x}",
                frame.rip, cr2, frame.error_code
            );
            panic!("unhandled page fault");
        }
        // general protection
        13 => {
            serial::println!(
                "[gpf ] #GP at RIP=0x{:x} err=0x{:x}",
                frame.rip, frame.error_code
            );
            panic!("unhandled GPF");
        }
        // timer IRQ
        32 => {
            crate::proc::scheduler::tick();
            crate::drivers::pic::eoi(0);
        }
        // keyboard IRQ
        33 => {
            let _scancode: u8;
            unsafe { core::arch::asm!("in al, 0x60", out("al") _scancode) };
            crate::drivers::pic::eoi(1);
        }
        _ => {
            if num < 32 {
                serial::println!("[exc ] unhandled exception {} at RIP=0x{:x}", num, frame.rip);
                panic!("cpu exception");
            } else {
                // spurious or unhandled IRQ -- ack and ignore
                crate::drivers::pic::eoi((num - 32) as u8);
            }
        }
    }
}
