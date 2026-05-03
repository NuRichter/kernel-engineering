// src/syscall/table.rs -- individual syscall implementations

use crate::serial;

pub fn sys_write(fd: u64, buf_ptr: u64, count: u64) -> i64 {
    // fd 1 = stdout -> serial
    if fd != 1 && fd != 2 { return -9; }  // EBADF

    let count = count.min(4096) as usize;
    let slice = unsafe { core::slice::from_raw_parts(buf_ptr as *const u8, count) };

    if let Ok(s) = core::str::from_utf8(slice) {
        serial::print!("{}", s);
    } else {
        // write raw bytes
        for &b in slice {
            serial::print!("{}", b as char);
        }
    }
    count as i64
}

pub fn sys_read(_fd: u64, _buf: u64, _count: u64) -> i64 {
    // stub: no keyboard driver wired yet
    -11  // EAGAIN
}

pub fn sys_exit(code: i32) -> i64 {
    serial::println!("[sys ] process exited with code {}", code);
    // TODO: reap current task, schedule next
    loop { unsafe { core::arch::asm!("hlt") }; }
}
