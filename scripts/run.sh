#!/usr/bin/env bash
# scripts/run.sh -- launch kernel in QEMU
# Arch: pacman -S qemu-system-x86
# Kali: apt install qemu-system-x86

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ISO="$ROOT/target/kernel-engineering.iso"
KERNEL="$ROOT/target/x86_64-unknown-none/release/kernel"

if [[ ! -f "$ISO" ]]; then
    echo "[!] ISO not found, run scripts/build.sh first"
    exit 1
fi

QEMU_ARGS=(
    -machine q35
    -cpu qemu64,+rdtscp
    -smp 2
    -m 256M
    -cdrom "$ISO"
    -serial stdio
    -display none
    -no-reboot
    -no-shutdown
)

# KVM if available
if [[ -e /dev/kvm ]] && groups | grep -q kvm; then
    QEMU_ARGS+=(-enable-kvm)
    echo "[qemu] KVM enabled"
fi

echo "[qemu] launching..."
exec qemu-system-x86_64 "${QEMU_ARGS[@]}"
