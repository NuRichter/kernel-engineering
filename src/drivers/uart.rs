// src/drivers/uart.rs -- generic 16550 UART (COM1..COM4)

pub const COM1: u16 = 0x3F8;
pub const COM2: u16 = 0x2F8;
pub const COM3: u16 = 0x3E8;
pub const COM4: u16 = 0x2E8;

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}
unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nomem, nostack));
    v
}

pub struct Uart { pub base: u16 }

impl Uart {
    pub fn new(base: u16, baud: u32) -> Self {
        let divisor = (115200 / baud) as u16;
        unsafe {
            outb(base + 1, 0x00);               // disable interrupts
            outb(base + 3, 0x80);               // DLAB on
            outb(base + 0, (divisor & 0xFF) as u8);
            outb(base + 1, (divisor >> 8)   as u8);
            outb(base + 3, 0x03);               // 8N1, DLAB off
            outb(base + 2, 0xC7);               // FIFO 14-byte
            outb(base + 4, 0x0B);               // RTS/DSR
        }
        Self { base }
    }

    pub fn write_byte(&self, b: u8) {
        unsafe {
            while inb(self.base + 5) & 0x20 == 0 {}
            outb(self.base, b);
        }
    }

    pub fn read_byte(&self) -> Option<u8> {
        unsafe {
            if inb(self.base + 5) & 0x01 != 0 {
                Some(inb(self.base))
            } else {
                None
            }
        }
    }

    pub fn write_str(&self, s: &str) {
        for b in s.bytes() {
            if b == b'\n' { self.write_byte(b'\r'); }
            self.write_byte(b);
        }
    }
}
