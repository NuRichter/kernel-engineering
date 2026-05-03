# kernel-engineering

a bare-metal x86_64 kernel written from scratch. rust for the high-level logic, C for the hardware shims, and hand-written assembly where the hardware demands it. no OS underneath. no libc.

> execution is only supported on **Arch Linux** and **Kali Linux**. if you're on something else, that's a you problem. respectfully.

---

## what this is

this repo is a personal deep-dive into kernel engineering -- not a toy, not a demo, but a real attempt at building a kernel that actually boots, manages memory, schedules tasks, and dispatches syscalls. it's a learning project that takes itself seriously.

stack breakdown:

| layer | language | what it does |
|-------|----------|-------------|
| boot sequence | NASM assembly | multiboot2 entry, long mode transition, GDT/IDT/syscall stubs |
| kernel core | Rust (no_std) | memory management, scheduling, IPC, syscall dispatch |
| hardware shims | C11 (freestanding) | ACPI parser, HAL macros, port I/O helpers |
| linker | GNU LD script | higher-half kernel layout, section control |

---

## tech stack (April 2026)

- **Rust** -- nightly, `no_std`, `no_main`, x86_64-unknown-none target
- **NASM** -- flat assembly, elf64 output, multiboot2 header
- **C11** -- freestanding, no libc, compiled with `x86_64-elf-gcc`
- **Shell / Bash** -- build orchestration, ISO creation, QEMU launch
- **GRUB2** -- multiboot2 bootloader
- **QEMU** -- development VM target

rustc features in use: `abi_x86_interrupt`, `naked_functions`, `allocator_api`, `const_mut_refs`

---

## architecture

```
kernel-engineering/
  arch/
    x86_64/
      boot.asm        multiboot2 header + long mode entry
      gdt.asm         GDT + TSS load stubs
      idt.asm         IDT stubs (32 exceptions + 16 IRQs)
      syscall.asm     SYSCALL/SYSRET entry trampoline
    linker.ld         kernel linker script (higher-half, -2 GiB)

  src/
    main.rs           kernel_main, multiboot2 parser, panic handler
    kernel/
      gdt.rs          TSS struct, GDT init
      idt.rs          gate descriptors, interrupt dispatcher
      serial.rs       COM1 UART logger (serial_println! macro)
      pic.rs          8259 PIC remap + EOI
    mm/
      pmm.rs          bitmap physical frame allocator
      vmm.rs          4-level paging, recursive mapping, map/unmap/translate
      heap.rs         global allocator (slab + linked-list)
      slab.rs         slab allocator (9 size classes, 8..2048 bytes)
    proc/
      task.rs         Task Control Block, kernel thread creation
      scheduler.rs    preemptive round-robin + nice priority queues
      elf.rs          ELF64 loader
    syscall/
      mod.rs          MSR setup, Linux-compatible dispatch
      table.rs        sys_read, sys_write, sys_exit
    ipc/
      msgq.rs         priority message queues
      pipe.rs         anonymous ring-buffer pipes
    drivers/
      pit.rs          8254 PIT (100 Hz timer)
      vga.rs          VGA text mode 80x25
      uart.rs         generic 16550 UART

  c/
    include/hal.h     inline port I/O, MSR, CPUID, control registers
    drivers/acpi.c    RSDP/RSDT/MADT parser (LAPIC + IOAPIC discovery)

  scripts/
    build.sh          assemble + compile C + cargo build + ISO
    mkiso.sh          GRUB2 ISO creation
    run.sh            QEMU launch
    debug.sh          QEMU + GDB/GEF attach
```

---

## setup

### Arch Linux

```bash
sudo pacman -S nasm qemu-system-x86 grub xorriso \
               rust nightly cross-x86_64-elf-gcc

rustup toolchain install nightly
rustup target add x86_64-unknown-none
rustup component add rust-src
```

### Kali Linux

```bash
sudo apt install nasm qemu-system-x86 grub-pc-bin xorriso \
                 gcc-x86-64-linux-gnu

curl https://sh.rustup.rs -sSf | sh
rustup toolchain install nightly
rustup target add x86_64-unknown-none
rustup component add rust-src
```

---

## build + run

```bash
# build (release)
bash scripts/build.sh release

# build (debug)
bash scripts/build.sh debug

# run in QEMU (serial output to stdout)
bash scripts/run.sh

# debug with GDB / GEF
bash scripts/debug.sh
```

---

## memory layout

```
0x0000_0000_0010_0000  kernel physical load address
0xFFFF_FFFF_8000_0000  kernel virtual base (-2 GiB, higher half)
0xFFFF_C000_0000_0000  kernel heap (8 MiB initial, slab + linked-list)
0xFFFF_E000_0000_0000  kernel stacks (4 KiB guard gap between each)
0xFFFF_FF00_0000_0000  recursive PML4 mapping (slot 510)
```

---

## boot sequence

```
BIOS/UEFI
  -> GRUB2 (multiboot2)
    -> boot.asm _start (32-bit protected mode)
      -> CPUID check, long mode enable
      -> identity map 1 GiB (2 MiB huge pages)
      -> switch to 64-bit via LGDT + far JMP
        -> kernel_main (Rust)
          -> GDT + TSS, IDT, PMM, VMM, heap, PIC, PIT, syscall MSRs
          -> idle task created, scheduler::start()
```

---

## syscall ABI

Linux-compatible register layout on x86_64. currently implemented:

| number | name | description |
|--------|------|-------------|
| 0 | `read` | read from fd (stub, returns EAGAIN) |
| 1 | `write` | write to stdout/stderr via serial |
| 60 | `exit` | terminate process |

---

## license

MIT. do whatever you want with it.

---

## author

**NuRichter** -- [github.com/NuRichter](https://github.com/NuRichter)
