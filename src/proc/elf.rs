// src/proc/elf.rs -- ELF64 loader for user-space programs

use crate::{mm::{pmm, vmm}, serial};

const ELFMAG:    [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ET_EXEC:   u16     = 2;
const ET_DYN:    u16     = 3;
const EM_X86_64: u16     = 62;
const PT_LOAD:   u32     = 1;
const PT_GNU_STACK: u32  = 0x6474E551;

#[repr(C)]
struct Ehdr {
    e_ident:     [u8; 16],
    e_type:      u16,
    e_machine:   u16,
    e_version:   u32,
    e_entry:     u64,
    e_phoff:     u64,
    e_shoff:     u64,
    e_flags:     u32,
    e_ehsize:    u16,
    e_phentsize: u16,
    e_phnum:     u16,
    e_shentsize: u16,
    e_shnum:     u16,
    e_shstrndx:  u16,
}

#[repr(C)]
struct Phdr {
    p_type:   u32,
    p_flags:  u32,
    p_offset: u64,
    p_vaddr:  u64,
    p_paddr:  u64,
    p_filesz: u64,
    p_memsz:  u64,
    p_align:  u64,
}

const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;

pub struct LoadedElf {
    pub entry:  u64,
    pub load_bias: u64,
}

/// load an ELF64 binary from a byte slice into the current address space
pub fn load(data: &[u8]) -> Result<LoadedElf, &'static str> {
    if data.len() < core::mem::size_of::<Ehdr>() {
        return Err("too small");
    }

    let ehdr = unsafe { &*(data.as_ptr() as *const Ehdr) };

    if &ehdr.e_ident[..4] != &ELFMAG          { return Err("bad magic"); }
    if ehdr.e_ident[4] != 2                   { return Err("not 64-bit"); }
    if ehdr.e_machine != EM_X86_64            { return Err("not x86_64"); }
    if ehdr.e_type != ET_EXEC && ehdr.e_type != ET_DYN {
        return Err("not executable");
    }

    let phdr_base = ehdr.e_phoff as usize;
    let phdr_size = ehdr.e_phentsize as usize;
    let phdr_cnt  = ehdr.e_phnum as usize;

    for i in 0..phdr_cnt {
        let off  = phdr_base + i * phdr_size;
        if off + phdr_size > data.len() { return Err("phdr out of bounds"); }

        let phdr = unsafe { &*(data.as_ptr().add(off) as *const Phdr) };

        if phdr.p_type != PT_LOAD { continue; }

        let file_off  = phdr.p_offset as usize;
        let file_size = phdr.p_filesz as usize;
        let mem_size  = phdr.p_memsz  as usize;
        let vaddr     = phdr.p_vaddr;

        if file_off + file_size > data.len() { return Err("segment out of bounds"); }

        // map pages for this segment
        let page_start = vaddr & !0xFFF;
        let page_end   = (vaddr + mem_size as u64 + 0xFFF) & !0xFFF;

        let mut flags = vmm::flags::PRESENT | vmm::flags::USER;
        if phdr.p_flags & PF_W != 0 { flags |= vmm::flags::WRITABLE; }
        if phdr.p_flags & PF_X == 0 { flags |= vmm::flags::NX; }

        let mut va = page_start;
        while va < page_end {
            let pa = pmm::alloc_frame().ok_or("elf: out of memory")?;
            // zero page
            unsafe { (pa as *mut u8).write_bytes(0, pmm::PAGE_SIZE) };
            vmm::map(va, pa, flags);
            va += pmm::PAGE_SIZE as u64;
        }

        // copy file data into virtual pages
        let src = &data[file_off..file_off + file_size];
        let dst = vaddr as *mut u8;
        unsafe { dst.copy_from_nonoverlapping(src.as_ptr(), file_size) };

        serial::println!(
            "[elf ] LOAD vaddr=0x{:x} filesz={} memsz={} flags=0x{:x}",
            vaddr, file_size, mem_size, phdr.p_flags
        );
    }

    Ok(LoadedElf { entry: ehdr.e_entry, load_bias: 0 })
}
