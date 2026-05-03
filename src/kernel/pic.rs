// src/kernel/pic.rs -- Intel 8259 PIC (Programmable Interrupt Controller)
// remaps IRQs 0-15 to vectors 32-47 to avoid collision with CPU exceptions

const PIC1_CMD:  u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD:  u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const ICW1_ICW4:  u8 = 0x01;
const ICW1_INIT:  u8 = 0x10;
const ICW4_8086:  u8 = 0x01;
const PIC_EOI:    u8 = 0x20;

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}
unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nomem, nostack));
    v
}
unsafe fn io_wait() {
    outb(0x80, 0);
}

pub fn init() {
    unsafe {
        // save masks
        let mask1 = inb(PIC1_DATA);
        let mask2 = inb(PIC2_DATA);

        // start init sequence
        outb(PIC1_CMD,  ICW1_INIT | ICW1_ICW4); io_wait();
        outb(PIC2_CMD,  ICW1_INIT | ICW1_ICW4); io_wait();

        // vector offsets
        outb(PIC1_DATA, 0x20); io_wait();   // IRQ0 -> INT 32
        outb(PIC2_DATA, 0x28); io_wait();   // IRQ8 -> INT 40

        // cascade wiring
        outb(PIC1_DATA, 0x04); io_wait();   // slave on IRQ2
        outb(PIC2_DATA, 0x02); io_wait();   // cascade identity

        // 8086 mode
        outb(PIC1_DATA, ICW4_8086); io_wait();
        outb(PIC2_DATA, ICW4_8086); io_wait();

        // restore masks
        outb(PIC1_DATA, mask1);
        outb(PIC2_DATA, mask2);

        // unmask timer (IRQ0) and keyboard (IRQ1), mask everything else
        outb(PIC1_DATA, 0b11111100);
        outb(PIC2_DATA, 0b11111111);
    }
}

/// send end-of-interrupt for a given IRQ line (0-15)
pub fn eoi(irq: u8) {
    unsafe {
        if irq >= 8 { outb(PIC2_CMD, PIC_EOI); }
        outb(PIC1_CMD, PIC_EOI);
    }
}

/// mask a specific IRQ line
pub fn mask(irq: u8) {
    unsafe {
        let (port, bit) = if irq < 8 { (PIC1_DATA, irq) } else { (PIC2_DATA, irq - 8) };
        let val = inb(port) | (1 << bit);
        outb(port, val);
    }
}

/// unmask a specific IRQ line
pub fn unmask(irq: u8) {
    unsafe {
        let (port, bit) = if irq < 8 { (PIC1_DATA, irq) } else { (PIC2_DATA, irq - 8) };
        let val = inb(port) & !(1 << bit);
        outb(port, val);
    }
}
