// src/drivers/pit.rs -- Intel 8253/8254 Programmable Interval Timer
// sets channel 0 to fire IRQ0 at the requested frequency

const PIT_CHANNEL0: u16 = 0x40;
const PIT_CMD:      u16 = 0x43;
const PIT_BASE_HZ:  u32 = 1_193_182;

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}

/// set PIT channel 0 to fire at `hz` interrupts per second (max ~1193 Hz)
pub fn init(hz: u32) {
    let divisor = (PIT_BASE_HZ / hz) as u16;

    unsafe {
        // channel 0, lobyte/hibyte, mode 3 (square wave)
        outb(PIT_CMD,      0x36);
        outb(PIT_CHANNEL0, (divisor & 0xFF) as u8);
        outb(PIT_CHANNEL0, (divisor >> 8)   as u8);
    }
}
