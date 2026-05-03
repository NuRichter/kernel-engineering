; gdt.asm -- Global Descriptor Table + Task State Segment
; called from Rust via FFI after long mode is active

global gdt_load
global tss_load
global gdt64_ptr

section .data
align 8

; null, kernel code (64-bit), kernel data, user code (64-bit), user data, TSS
gdt_table:
.null:      dq 0
.kcode:     dq (1<<44)|(1<<47)|(1<<41)|(1<<43)|(1<<53)   ; exec|present|readable|code|64-bit
.kdata:     dq (1<<44)|(1<<47)|(1<<41)                    ; data|present|writable
.ucode:     dq (1<<44)|(1<<47)|(1<<41)|(1<<43)|(1<<53)|(3<<45)
.udata:     dq (1<<44)|(1<<47)|(1<<41)|(3<<45)
.tss_low:   dq 0   ; filled at runtime by Rust
.tss_high:  dq 0
gdt_end:

gdt64_ptr:
    dw gdt_end - gdt_table - 1
    dq gdt_table

section .text
bits 64

; void gdt_load(void)
gdt_load:
    lgdt [gdt64_ptr]
    ; reload CS via far return
    push 0x08              ; kernel code selector
    lea  rax, [rel .reload]
    push rax
    retfq
.reload:
    mov ax, 0x10           ; kernel data selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    ret

; void tss_load(u64 tss_base, u16 selector)
; rdi = base address of TSS struct, rsi = selector offset
tss_load:
    ; write base into TSS descriptor slots in GDT
    lea  rax, [rel gdt_table]
    add  rax, rsi
    ; low 32 bits of descriptor
    mov  rcx, rdi
    mov  [rax + 2], cx         ; base[15:0]
    shr  rcx, 16
    mov  [rax + 4], cl         ; base[23:16]
    shr  rcx, 8
    mov  [rax + 7], cl         ; base[31:24]
    ; high 32 bits
    shr  rdi, 32
    mov  [rax + 8], edi        ; base[63:32]
    ; load TR
    mov  ax, si
    ltr  ax
    ret
