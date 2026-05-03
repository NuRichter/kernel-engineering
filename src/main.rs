// src/main.rs -- kernel entry point
// called from boot.asm after long mode is active

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(allocator_api)]
#![feature(const_mut_refs)]
#![feature(naked_functions)]
#![allow(dead_code)]

extern crate alloc;

mod kernel;
mod mm;
mod proc;
mod drivers;
mod syscall;
mod ipc;

use core::panic::PanicInfo;
use kernel::{gdt, idt, serial};
use mm::pmm;
use proc::scheduler;

#[no_mangle]
pub extern "C" fn kernel_main(mb2_ptr: u64) -> ! {
    serial::init();
    serial::println!("[boot] kernel_engineering v{}", env!("CARGO_PKG_VERSION"));

    gdt::init();
    serial::println!("[gdt ] loaded");

    idt::init();
    serial::println!("[idt ] loaded");

    // parse multiboot2 info
    let mb2 = unsafe { mb2::parse(mb2_ptr) };
    serial::println!("[mb2 ] parsed, mmap entries: {}", mb2.mmap_count);

    pmm::init(&mb2);
    serial::println!("[pmm ] initialized, free: {} KiB", pmm::free_kib());

    mm::vmm::init();
    serial::println!("[vmm ] page tables ready");

    mm::heap::init();
    serial::println!("[heap] slab allocator online");

    drivers::pic::init();
    drivers::pit::init(100);   // 100 Hz tick
    serial::println!("[irq ] PIC + PIT ready");

    syscall::init();
    serial::println!("[sys ] SYSCALL/SYSRET wired");

    proc::init();
    serial::println!("[proc] process subsystem ready");

    ipc::init();
    serial::println!("[ipc ] message queues + pipes ready");

    serial::println!("[boot] jumping to scheduler");
    scheduler::start();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial::println!("\n[PANIC] {}", info);
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}

mod mb2 {
    use crate::serial;

    #[derive(Debug)]
    pub struct Info {
        pub mmap_count: usize,
        pub mmap_ptr:   u64,
        pub mmap_entry_size: u32,
    }

    /// parse multiboot2 info structure (minimal, mmap only)
    pub unsafe fn parse(ptr: u64) -> Info {
        // total_size at offset 0, reserved at offset 4, tags start at offset 8
        let base = ptr as *const u8;
        let mut off: usize = 8;
        let total = *(ptr as *const u32) as usize;

        let mut info = Info { mmap_count: 0, mmap_ptr: 0, mmap_entry_size: 0 };

        while off < total {
            let tag_ptr = base.add(off) as *const u32;
            let typ  = *tag_ptr;
            let size = *tag_ptr.add(1);

            match typ {
                6 => {
                    // memory map tag
                    let entry_size = *(tag_ptr.add(2));
                    let _entry_version = *(tag_ptr.add(3));
                    info.mmap_entry_size = entry_size;
                    info.mmap_ptr = base.add(off + 16) as u64;
                    info.mmap_count = ((size - 16) / entry_size) as usize;
                    serial::println!("[mb2 ] mmap @ 0x{:x}, {} entries", info.mmap_ptr, info.mmap_count);
                }
                0 => break,  // end tag
                _ => {}
            }

            // tags are 8-byte aligned
            off += ((size as usize) + 7) & !7;
        }

        info
    }
}
