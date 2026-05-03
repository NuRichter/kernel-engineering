/* c/include/hal.h -- Hardware Abstraction Layer (x86_64)
   shared between C and Rust (via bindgen) */

#pragma once
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

/* ---- port I/O ---- */
static inline void outb(uint16_t port, uint8_t val) {
    __asm__ volatile("out %0, %1" :: "a"(val), "Nd"(port) : "memory");
}
static inline uint8_t inb(uint16_t port) {
    uint8_t v;
    __asm__ volatile("in %1, %0" : "=a"(v) : "Nd"(port) : "memory");
    return v;
}
static inline void outw(uint16_t port, uint16_t val) {
    __asm__ volatile("out %0, %1" :: "a"(val), "Nd"(port) : "memory");
}
static inline uint16_t inw(uint16_t port) {
    uint16_t v;
    __asm__ volatile("in %1, %0" : "=a"(v) : "Nd"(port) : "memory");
    return v;
}
static inline void outl(uint16_t port, uint32_t val) {
    __asm__ volatile("out %0, %1" :: "a"(val), "Nd"(port) : "memory");
}
static inline uint32_t inl(uint16_t port) {
    uint32_t v;
    __asm__ volatile("in %1, %0" : "=a"(v) : "Nd"(port) : "memory");
    return v;
}
static inline void io_wait(void) { outb(0x80, 0); }

/* ---- control registers ---- */
static inline uint64_t read_cr0(void) { uint64_t v; __asm__("mov %0, cr0" : "=r"(v)); return v; }
static inline uint64_t read_cr2(void) { uint64_t v; __asm__("mov %0, cr2" : "=r"(v)); return v; }
static inline uint64_t read_cr3(void) { uint64_t v; __asm__("mov %0, cr3" : "=r"(v)); return v; }
static inline uint64_t read_cr4(void) { uint64_t v; __asm__("mov %0, cr4" : "=r"(v)); return v; }
static inline void write_cr3(uint64_t v) { __asm__ volatile("mov cr3, %0" :: "r"(v) : "memory"); }

/* ---- MSR ---- */
static inline uint64_t rdmsr(uint32_t msr) {
    uint32_t lo, hi;
    __asm__ volatile("rdmsr" : "=a"(lo), "=d"(hi) : "c"(msr));
    return ((uint64_t)hi << 32) | lo;
}
static inline void wrmsr(uint32_t msr, uint64_t val) {
    __asm__ volatile("wrmsr" :: "c"(msr), "a"((uint32_t)val), "d"((uint32_t)(val >> 32)));
}

/* ---- CPUID ---- */
static inline void cpuid(uint32_t leaf, uint32_t *eax, uint32_t *ebx, uint32_t *ecx, uint32_t *edx) {
    __asm__ volatile("cpuid"
        : "=a"(*eax), "=b"(*ebx), "=c"(*ecx), "=d"(*edx)
        : "a"(leaf), "c"(0));
}

/* ---- interrupt control ---- */
static inline void cli(void) { __asm__ volatile("cli"); }
static inline void sti(void) { __asm__ volatile("sti"); }
static inline void hlt(void) { __asm__ volatile("hlt"); }

/* ---- memory barriers ---- */
static inline void mfence(void) { __asm__ volatile("mfence" ::: "memory"); }
static inline void lfence(void) { __asm__ volatile("lfence" ::: "memory"); }
static inline void sfence(void) { __asm__ volatile("sfence" ::: "memory"); }

/* ---- TLB ---- */
static inline void invlpg(uintptr_t va) {
    __asm__ volatile("invlpg [%0]" :: "r"(va) : "memory");
}
static inline void flush_tlb_all(void) {
    uint64_t cr3 = read_cr3();
    write_cr3(cr3);
}

/* ---- misc ---- */
static inline uint64_t rdtsc(void) {
    uint32_t lo, hi;
    __asm__ volatile("rdtsc" : "=a"(lo), "=d"(hi));
    return ((uint64_t)hi << 32) | lo;
}
