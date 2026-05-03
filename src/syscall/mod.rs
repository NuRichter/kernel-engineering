// src/syscall/mod.rs -- syscall dispatcher + number table
// entry trampoline lives in arch/x86_64/syscall.asm

mod table;

use crate::serial;

extern "C" {
    fn syscall_init_msrs();
}

pub fn init() {
    unsafe { syscall_init_msrs() };
}

// called from syscall.asm with Linux-compatible register layout
#[no_mangle]
pub extern "C" fn syscall_dispatch(
    num: u64,
    a1:  u64, a2: u64, a3: u64,
    a4:  u64, a5: u64, a6: u64,
) -> i64 {
    match num {
        0  => table::sys_read (a1, a2, a3),
        1  => table::sys_write(a1, a2, a3),
        60 => table::sys_exit (a1 as i32),
        _  => {
            serial::println!("[sys ] unknown syscall {}", num);
            -38  // ENOSYS
        }
    }
}
