#!/usr/bin/env bash
# scripts/build.sh -- build kernel-engineering
# only tested on Arch Linux and Kali Linux

set -euo pipefail

TARGET="x86_64-unknown-none"
PROFILE="${1:-release}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/target/$TARGET/$PROFILE"
ASM_OBJ="$ROOT/target/asm"
C_OBJ="$ROOT/target/c_obj"

echo "[build] kernel-engineering -- profile: $PROFILE"
echo "[build] root: $ROOT"

# ---- check deps ----
for dep in nasm x86_64-elf-gcc rustup; do
    if ! command -v "$dep" &>/dev/null; then
        echo "[!] missing dependency: $dep"
        echo "    Arch:  pacman -S nasm cross-x86_64-elf-gcc"
        echo "    Kali:  apt install nasm gcc-x86-64-linux-gnu"
        exit 1
    fi
done

# ---- assemble ASM sources ----
mkdir -p "$ASM_OBJ"
ASM_SRCS=(
    arch/x86_64/boot.asm
    arch/x86_64/gdt.asm
    arch/x86_64/idt.asm
    arch/x86_64/syscall.asm
)
ASM_OBJECTS=()

for src in "${ASM_SRCS[@]}"; do
    obj="$ASM_OBJ/$(basename "${src%.asm}").o"
    echo "[nasm] $src -> $obj"
    nasm -f elf64 "$ROOT/$src" -o "$obj"
    ASM_OBJECTS+=("$obj")
done

# ---- compile C sources ----
mkdir -p "$C_OBJ"
CFLAGS="-ffreestanding -fno-stack-protector -fno-pic -mno-red-zone \
        -mcmodel=kernel -std=c11 -O2 -Wall -Wextra \
        -I$ROOT/c/include"

C_SRCS=(c/drivers/acpi.c)
C_OBJECTS=()

for src in "${C_SRCS[@]}"; do
    obj="$C_OBJ/$(basename "${src%.c}").o"
    echo "[cc  ] $src -> $obj"
    x86_64-elf-gcc $CFLAGS -c "$ROOT/$src" -o "$obj"
    C_OBJECTS+=("$obj")
done

# ---- build Rust kernel ----
echo "[rust] cargo build --target $TARGET --profile $PROFILE"
RUSTFLAGS="-C link-arg=-T$ROOT/arch/linker.ld \
           -C link-arg=${ASM_OBJECTS[*]// / -C link-arg=} \
           -C link-arg=${C_OBJECTS[*]// / -C link-arg=}" \
    cargo build \
        --manifest-path "$ROOT/Cargo.toml" \
        --target "$TARGET" \
        $([ "$PROFILE" = "release" ] && echo "--release")

echo "[done] binary: $OUT/kernel"

# ---- create ISO ----
if command -v grub-mkrescue &>/dev/null; then
    bash "$ROOT/scripts/mkiso.sh" "$OUT/kernel"
else
    echo "[skip] grub-mkrescue not found, skipping ISO"
fi
