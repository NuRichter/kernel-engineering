; idt.asm -- Interrupt Descriptor Table + ISR/IRQ stubs
; exception frames are passed to Rust handlers via C calling convention

%macro ISR_NOERR 1
global isr%1
isr%1:
    push qword 0        ; fake error code
    push qword %1       ; interrupt number
    jmp  isr_common_stub
%endmacro

%macro ISR_ERR 1
global isr%1
isr%1:
    push qword %1
    jmp  isr_common_stub
%endmacro

%macro IRQ_STUB 2
global irq%1
irq%2:
    push qword 0
    push qword (%1 + 32)
    jmp  isr_common_stub
%endmacro

extern interrupt_dispatch     ; Rust dispatcher

; CPU exceptions
ISR_NOERR 0    ; divide by zero
ISR_NOERR 1    ; debug
ISR_NOERR 2    ; NMI
ISR_NOERR 3    ; breakpoint
ISR_NOERR 4    ; overflow
ISR_NOERR 5    ; bound range exceeded
ISR_NOERR 6    ; invalid opcode
ISR_NOERR 7    ; device not available
ISR_ERR   8    ; double fault
ISR_NOERR 9    ; coprocessor segment overrun
ISR_ERR   10   ; invalid TSS
ISR_ERR   11   ; segment not present
ISR_ERR   12   ; stack-segment fault
ISR_ERR   13   ; general protection fault
ISR_ERR   14   ; page fault
ISR_NOERR 15
ISR_NOERR 16   ; x87 FPU error
ISR_ERR   17   ; alignment check
ISR_NOERR 18   ; machine check
ISR_NOERR 19   ; SIMD FP exception
ISR_NOERR 20   ; virtualization
ISR_NOERR 21
ISR_NOERR 22
ISR_NOERR 23
ISR_NOERR 24
ISR_NOERR 25
ISR_NOERR 26
ISR_NOERR 27
ISR_NOERR 28
ISR_NOERR 29
ISR_ERR   30   ; security
ISR_NOERR 31

; IRQs 0-15 mapped to vectors 32-47
IRQ_STUB 0,  32
IRQ_STUB 1,  33
IRQ_STUB 2,  34
IRQ_STUB 3,  35
IRQ_STUB 4,  36
IRQ_STUB 5,  37
IRQ_STUB 6,  38
IRQ_STUB 7,  39
IRQ_STUB 8,  40
IRQ_STUB 9,  41
IRQ_STUB 10, 42
IRQ_STUB 11, 43
IRQ_STUB 12, 44
IRQ_STUB 13, 45
IRQ_STUB 14, 46
IRQ_STUB 15, 47

section .text
bits 64

; common stub: saves full register state, calls Rust dispatcher
isr_common_stub:
    ; save caller-saved + extra regs
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov  rdi, rsp           ; pass stack frame pointer as first arg
    call interrupt_dispatch

    pop  r15
    pop  r14
    pop  r13
    pop  r12
    pop  r11
    pop  r10
    pop  r9
    pop  r8
    pop  rbp
    pop  rdi
    pop  rsi
    pop  rdx
    pop  rcx
    pop  rbx
    pop  rax

    add  rsp, 16            ; pop error code + interrupt number
    iretq

global idt_load
idt_load:
    ; rdi = pointer to IDTR struct { limit: u16, base: u64 }
    lidt [rdi]
    ret
