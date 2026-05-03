// src/drivers/vga.rs -- VGA text mode 80x25 driver (legacy, serial preferred)

use core::fmt::{self, Write};
use spin::Mutex;

const VGA_BUF: *mut u16 = 0xB8000 as *mut u16;
const COLS: usize = 80;
const ROWS: usize = 25;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black   = 0,  Blue    = 1,  Green  = 2,  Cyan   = 3,
    Red     = 4,  Magenta = 5,  Brown  = 6,  White  = 7,
    BrBlack = 8,  BrBlue  = 9,  BrGreen = 10, BrCyan = 11,
    BrRed   = 12, Pink    = 13, Yellow = 14, BrWhite = 15,
}

fn attr(fg: Color, bg: Color) -> u8 { (bg as u8) << 4 | fg as u8 }

struct Vga {
    col:   usize,
    row:   usize,
    color: u8,
}

impl Vga {
    const fn new() -> Self {
        Self { col: 0, row: 0, color: (Color::Black as u8) << 4 | Color::BrCyan as u8 }
    }

    fn set_color(&mut self, fg: Color, bg: Color) {
        self.color = attr(fg, bg);
    }

    fn put_char(&mut self, c: u8) {
        match c {
            b'\n' => { self.col = 0; self.row += 1; }
            b'\r' => { self.col = 0; }
            _ => {
                let entry = (self.color as u16) << 8 | c as u16;
                unsafe { VGA_BUF.add(self.row * COLS + self.col).write_volatile(entry) };
                self.col += 1;
                if self.col >= COLS { self.col = 0; self.row += 1; }
            }
        }
        if self.row >= ROWS { self.scroll(); }
    }

    fn scroll(&mut self) {
        unsafe {
            // move rows 1..ROWS-1 up one row
            for r in 1..ROWS {
                for c in 0..COLS {
                    let src = VGA_BUF.add(r * COLS + c).read_volatile();
                    VGA_BUF.add((r - 1) * COLS + c).write_volatile(src);
                }
            }
            // clear last row
            for c in 0..COLS {
                VGA_BUF.add((ROWS - 1) * COLS + c).write_volatile((self.color as u16) << 8 | b' ' as u16);
            }
        }
        self.row = ROWS - 1;
    }

    pub fn clear(&mut self) {
        let blank = (self.color as u16) << 8 | b' ' as u16;
        for i in 0..(COLS * ROWS) {
            unsafe { VGA_BUF.add(i).write_volatile(blank) };
        }
        self.col = 0;
        self.row = 0;
    }
}

impl Write for Vga {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() { self.put_char(b); }
        Ok(())
    }
}

pub static VGA: Mutex<Vga> = Mutex::new(Vga::new());

pub fn clear() { VGA.lock().clear(); }
