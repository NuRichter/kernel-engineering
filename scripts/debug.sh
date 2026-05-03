#!/usr/bin/env bash
# scripts/debug.sh -- launch QEMU with GDB server, attach automatically
# requires: gdb or gef (gdb-multiarch on Kali)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ISO="$ROOT/target/kernel-engineering.iso"
KERNEL="$ROOT/target/x86_64-unknown-none/debug/kernel"
GDB_PORT=1234

# prefer gef > gdb-multiarch > gdb
GDB="gdb"
for g in gef gdb-multiarch gdb; do
    if command -v "$g" &>/dev/null; then GDB="$g"; break; fi
done

echo "[dbg ] using $GDB"
echo "[dbg ] QEMU GDB port :$GDB_PORT"

# start QEMU in background
qemu-system-x86_64 \
    -machine q35 -cpu qemu64 -smp 1 -m 256M \
    -cdrom "$ISO" -serial stdio -display none \
    -no-reboot -s -S &

QEMU_PID=$!
sleep 0.5

"$GDB" "$KERNEL" \
    -ex "set architecture i386:x86-64" \
    -ex "target remote :$GDB_PORT" \
    -ex "break kernel_main" \
    -ex "continue"

kill "$QEMU_PID" 2>/dev/null || true
