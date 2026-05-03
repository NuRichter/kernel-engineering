// src/kernel/serial.rs -- COM1 serial output (16550 UART)
// used as the kernel log sink throughout boot

use core::fmt::{self, Write};
use spin::Mutex;

const COM1: u16 = 0x3F8;

unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack));
}

unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") v, options(nomem, nostack));
    v
}

fn is_transmit_empty() -> bool {
    unsafe { (inb(COM1 + 5) & 0x20) != 0 }
}

fn send_byte(b: u8) {
    while !is_transmit_empty() {}
    unsafe { outb(COM1, b) };
}

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            if b == b'\n' { send_byte(b'\r'); }
            send_byte(b);
        }
        Ok(())
    }
}

pub static SERIAL: Mutex<SerialWriter> = Mutex::new(SerialWriter);

pub fn init() {
    unsafe {
        outb(COM1 + 1, 0x00);  // disable interrupts
        outb(COM1 + 3, 0x80);  // enable DLAB
        outb(COM1 + 0, 0x01);  // divisor low  (115200 baud)
        outb(COM1 + 1, 0x00);  // divisor high
        outb(COM1 + 3, 0x03);  // 8 bits, no parity, 1 stop (clear DLAB)
        outb(COM1 + 2, 0xC7);  // enable FIFO, clear, 14-byte threshold
        outb(COM1 + 4, 0x0B);  // IRQs enabled, RTS/DSR set
    }
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::kernel::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    ()              => ($crate::serial_print!("\n"));
    ($($arg:tt)*)   => ($crate::serial_print!("{}\n", format_args!($($arg)*)));
}

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    SERIAL.lock().write_fmt(args).unwrap();
}

// re-export so callers can do serial::println!
pub use serial_print   as print;
pub use serial_println as println;
