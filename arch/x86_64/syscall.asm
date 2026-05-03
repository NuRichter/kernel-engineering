; syscall.asm -- SYSCALL/SYSRET entry point + context save/restore
; MSR setup done from Rust; this file owns the raw entry trampoline

global syscall_entry
global syscall_init_msrs
extern syscall_dispatch     ; Rust handler

%define IA32_EFER   0xC0000080
%define IA32_STAR   0xC0000081
%define IA32_LSTAR  0xC0000082
%define IA32_SFMASK 0xC0000084

; selectors: kernel CS = 0x08, kernel SS = 0x10, user CS = 0x18+16 (RPL3)
%define STAR_VAL  (0x001B000800000000)  ; user CS:SS | kernel CS:SS

section .text
bits 64

; called once during boot from Rust to program MSRs
syscall_init_msrs:
    ; enable SCE in EFER
    mov  ecx, IA32_EFER
    rdmsr
    or   eax, 1
    wrmsr

    ; STAR: segment selectors
    mov  ecx, IA32_STAR
    mov  edx, 0x001B0008
    xor  eax, eax
    wrmsr

    ; LSTAR: syscall entry RIP
    mov  ecx, IA32_LSTAR
    lea  rax, [rel syscall_entry]
    mov  rdx, rax
    shr  rdx, 32
    and  eax, 0xFFFFFFFF
    wrmsr

    ; SFMASK: clear IF + DF on syscall entry
    mov  ecx, IA32_SFMASK
    mov  eax, (1<<9)|(1<<10)
    xor  edx, edx
    wrmsr
    ret

; syscall ABI (Linux-compatible subset):
;   rax = syscall number
;   rdi rsi rdx r10 r8 r9 = args (r10 instead of rcx because SYSCALL clobbers rcx)
;   rcx = saved RIP (by hardware)
;   r11 = saved RFLAGS (by hardware)

syscall_entry:
    ; swap to kernel GS if we came from user (gsbase swapgs trick)
    swapgs

    ; save user stack, switch to kernel stack stored in kernel GS
    mov  [gs:0x10], rsp        ; save user RSP
    mov  rsp, [gs:0x08]        ; load kernel RSP from per-cpu struct

    ; build InterruptFrame-compatible save area
    push rcx               ; user RIP
    push r11               ; user RFLAGS
    push rax               ; syscall number
    push rbx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r12
    push r13
    push r14
    push r15

    ; arg fix-up: move r10 -> rcx for Rust (4th arg)
    mov  rcx, r10

    ; call Rust dispatcher(num: u64, a1, a2, a3, a4, a5, a6)
    mov  rdi, rax          ; syscall number
    ; rsi rdi already set by user, but rdi just got clobbered...
    ; fix up: load from saved frame
    mov  rsi, [rsp + 6*8]  ; saved rdi = arg1
    mov  rdx, [rsp + 5*8]  ; saved rsi = arg2
    mov  rcx, [rsp + 3*8]  ; saved r10 = arg3
    mov  r8,  [rsp + 8*8]  ; saved r8  = arg4
    mov  r9,  [rsp + 9*8]  ; saved r9  = arg5
    ; rdi still has syscall number -- swap
    xchg rdi, [rsp + 6*8]
    mov  rdi, rax
    call syscall_dispatch

    ; rax = return value from Rust
    pop  r15
    pop  r14
    pop  r13
    pop  r12
    pop  r10
    pop  r9
    pop  r8
    pop  rbp
    pop  rdi
    pop  rsi
    pop  rdx
    pop  rbx
    add  rsp, 8            ; syscall number slot
    pop  r11               ; restore user RFLAGS
    pop  rcx               ; restore user RIP

    ; restore user stack
    mov  rsp, [gs:0x10]
    swapgs
    sysretq
