; boot.asm -- multiboot2 entry, long mode transition
; arch: x86_64 | assembler: nasm
; only tested on Arch Linux & Kali Linux

global _start
global stack_top
extern kernel_main

section .multiboot2
align 8
mb2_header:
    dd  0xE85250D6                          ; magic
    dd  0                                   ; arch: i386 protected
    dd  mb2_end - mb2_header               ; header length
    dd  -(0xE85250D6 + 0 + (mb2_end - mb2_header))  ; checksum

    ; framebuffer tag
    align 8
    dw  5
    dw  1
    dd  20
    dd  0
    dd  80
    dd  25

    ; end tag
    align 8
    dw  0
    dw  0
    dd  8
mb2_end:

section .bss
align 16
stack_bottom:
    resb 65536          ; 64 KiB kernel stack
stack_top:

align 4096
p4_table:   resb 4096
p3_table:   resb 4096
p2_table:   resb 4096

section .rodata
gdt64:
    dq 0
.code: equ $ - gdt64
    dq (1 << 43) | (1 << 44) | (1 << 47) | (1 << 53)
.pointer:
    dw $ - gdt64 - 1
    dq gdt64

section .text
bits 32
_start:
    mov esp, stack_top
    mov [mb2_info_ptr], ebx

    call check_multiboot
    call check_cpuid
    call check_long_mode
    call setup_page_tables
    call enable_paging

    lgdt [gdt64.pointer]
    jmp gdt64.code:long_mode_start

check_multiboot:
    cmp eax, 0x36D76289
    jne .no_multiboot
    ret
.no_multiboot:
    mov al, '0'
    jmp error

check_cpuid:
    pushfd
    pop eax
    mov ecx, eax
    xor eax, 1 << 21
    push eax
    popfd
    pushfd
    pop eax
    push ecx
    popfd
    cmp eax, ecx
    je .no_cpuid
    ret
.no_cpuid:
    mov al, '1'
    jmp error

check_long_mode:
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no_long_mode
    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29
    jz .no_long_mode
    ret
.no_long_mode:
    mov al, '2'
    jmp error

setup_page_tables:
    ; P4[0] -> P3
    mov eax, p3_table
    or  eax, 0b11
    mov [p4_table], eax

    ; P3[0] -> P2
    mov eax, p2_table
    or  eax, 0b11
    mov [p3_table], eax

    ; map P2 as huge pages (2 MiB each), 512 entries = 1 GiB identity
    mov ecx, 0
.map_p2:
    mov eax, 0x200000
    mul ecx
    or  eax, 0b10000011    ; present + writable + huge
    mov [p2_table + ecx * 8], eax
    inc ecx
    cmp ecx, 512
    jne .map_p2
    ret

enable_paging:
    mov eax, p4_table
    mov cr3, eax
    mov eax, cr4
    or  eax, 1 << 5        ; PAE
    mov cr4, eax
    mov ecx, 0xC0000080
    rdmsr
    or  eax, 1 << 8        ; long mode bit
    wrmsr
    mov eax, cr0
    or  eax, 1 << 31       ; paging
    mov cr0, eax
    ret

error:
    ; write 'ERR: X' to VGA text buffer
    mov dword [0xB8000], 0x4F524F45
    mov dword [0xB8004], 0x4F3A4F52
    mov dword [0xB8008], 0x4F204F20
    mov byte  [0xB800A], al
    hlt

section .data
mb2_info_ptr: dd 0

bits 64
long_mode_start:
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; pass multiboot2 info pointer as first argument
    movsx rdi, dword [mb2_info_ptr]
    call kernel_main
    hlt
