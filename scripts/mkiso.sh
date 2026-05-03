#!/usr/bin/env bash
# scripts/mkiso.sh -- wrap kernel in a GRUB2 bootable ISO

set -euo pipefail

KERNEL="${1:?usage: mkiso.sh <kernel_elf>}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ISO_DIR="$ROOT/target/iso"
ISO_OUT="$ROOT/target/kernel-engineering.iso"

mkdir -p "$ISO_DIR/boot/grub"

cp "$KERNEL" "$ISO_DIR/boot/kernel.elf"

cat >"$ISO_DIR/boot/grub/grub.cfg" <<'EOF'
set timeout=0
set default=0

menuentry "kernel-engineering" {
    multiboot2 /boot/kernel.elf
    boot
}
EOF

grub-mkrescue -o "$ISO_OUT" "$ISO_DIR" 2>/dev/null
echo "[iso ] $ISO_OUT"
