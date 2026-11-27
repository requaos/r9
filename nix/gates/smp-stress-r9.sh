#!/usr/bin/env bash
# nix/gates/smp-stress-r9.sh — the packaged aarch64 two-CPU stress gate as an
# r9 flake app (phase 05, SMP-08/SMP-09/MEM-17, D-24/D-27/D-28).
#
# The composed pipeline of nix/gates/smoke-r9.sh, changed in exactly two
# places: the DEFAULT bootargs carry the exact `aeadloop=1` token (so the
# in-kernel 9P2000.aead loopback server starts and publishes #M — D-28),
# and the driver is gates/aarch64/smp-stress.expect, which drives concurrent
# opens/reads/clunks over '#M/data0'..'#M/data7' plus exec churn across both
# CPUs under timer interrupts (D-27).
#
# The pipeline, each step loud on failure:
#
#   1. the image — `cargo xtask image --arch aarch64 --image bootfs` from the
#      r9 checkout writes target/images/aarch64/bootfs.paq (nothing is written
#      into the Harvey tree, HARN-02).
#   2. DTB patch: a throwaway copy of the package's selected DTB
#      (virt-gicv3.dtb by default) gets /chosen linux,initrd-start/-end as two
#      32-bit BE cells, then the boot string `aeadloop=1` is written with
#      `fdtput -t s "$DTB" /chosen bootargs "$R9_BOOTARGS"` (the exact
#      nix/gates/smoke-r9.sh:215-220 mechanism).  The kernel's DTB bootconf
#      reader turns it into the command line aead_loop::parse_boot_switch
#      requires.
#   3. smp-stress — gates/aarch64/smp-stress.expect spawns qemu with `-smp 2`
#      and `-dtb $env(R9_DTB)` (never a QEMU `-append`) and asserts the SMP,
#      SIMD and aeadloop markers, drives the #M read/clunk + exec-churn load
#      from Harvey's rc shell, and proves SMP_STRESS_OK with zero
#      watchdog/panic/rank faults.
#
# Env contract:
#   R9_HARVEY_DIR_ARM64 REQUIRED — the read-only arm64 tree carrying
#                  /HARVEY-TREE (the flake's harvey-tree-arm64).
#   R9_ROOT        the r9 checkout root (default $PWD).
#   R9_KERNEL_PKG  REQUIRED — the r9 kernel-virt package store path.
#   R9_MKPAQFS     REQUIRED — the host mkpaqfs the image engine bakes with.
#   R9_GATE_OUT    output dir (default target/gates/smp-stress-r9).
#   R9_SMOKE_LOG   default $R9_GATE_OUT/smp-stress-r9.log (serial capture).
#   R9_SMOKE_DRIVER  default $R9_ROOT/gates/aarch64/smp-stress.expect.
#   R9_QEMU        default qemu-system-aarch64.
#   R9_ACCEL       auto (default) | hvf | kvm | tcg.
#   R9_GICV2       =1 restores the gicv2/cortex-a72/tcg bisection shape.
#   R9_BOOTARGS    the /chosen bootargs string (default aeadloop=1 — the
#                  switch that starts the in-kernel 9P2000.aead server).
#   R9_BOOTARGS_PROP  the /chosen property name (default bootargs).
#
# Hygiene: the capture is rm -f'd BEFORE launch; the expect driver kills
# qemu's process group on every exit path; this script's EXIT trap sweeps any
# survivor via a bracket-fixed pkill keyed on this run's unique R9_RUN_TAG.
#
# usage: nix run .#smp-stress-r9   (from the r9 repo root)

set -e

HARVEY="${R9_HARVEY_DIR_ARM64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/smp-stress-r9}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and kernel package contract -----------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64 unset — run via the flake app (nix run .#smp-stress-r9), which exports the read-only arm64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_ARM64 must name harvey-tree-arm64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/arm64/bootfs.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/arm64/bootfs.toml is missing — the bootfs image manifest is part of this checkout"
R9_SMOKE_DRIVER="${R9_SMOKE_DRIVER:-$R9_ROOT/gates/aarch64/smp-stress.expect}"
[ -f "$R9_SMOKE_DRIVER" ] || err "R9_SMOKE_DRIVER=$R9_SMOKE_DRIVER: the stress driver is missing"

[ -n "${R9_KERNEL_PKG:-}" ] || err "R9_KERNEL_PKG unset — run via the flake app (nix run .#smp-stress-r9), which exports the r9 kernel-virt store path"
[ -d "$R9_KERNEL_PKG" ] || err "R9_KERNEL_PKG $R9_KERNEL_PKG: not a directory (unrealised store path?)"
[ -n "${R9_MKPAQFS:-}" ] || err "R9_MKPAQFS unset — run via the flake app (nix run .#smp-stress-r9), which exports the host mkpaqfs"
[ -f "$R9_MKPAQFS" ] && [ -x "$R9_MKPAQFS" ] \
	|| err "R9_MKPAQFS $R9_MKPAQFS: not an executable file (want <pkg>/bin/mkpaqfs)"

R9_KERNEL="$R9_KERNEL_PKG/kernel-virt.elf"
if [ ! -f "$R9_KERNEL" ]; then
	R9_KERNEL=$(find "$R9_KERNEL_PKG" -name 'kernel-virt.elf' -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_KERNEL" ] && [ -f "$R9_KERNEL" ] || err "kernel-virt.elf not found under $R9_KERNEL_PKG"

# --- step 0b: machine/accel selection ---------------------------------
# gicv3 is the default machine shape everywhere; accel prefers hvf on
# darwin-aarch64 and falls back to TCG.  R9_ACCEL forces the accelerator;
# R9_GICV2=1 restores the v1.0 reference shape for bisection.

if [ "${R9_GICV2:-0}" = "1" ]; then
	R9_ACCEL=tcg
	R9_MACHINE="virt,gic-version=2"
	R9_CPU=cortex-a72
	R9_DTB_NAME=virt-gicv2.dtb
else
	R9_ACCEL="${R9_ACCEL:-auto}"
	if [ "$R9_ACCEL" = "auto" ]; then
		R9_ACCEL=tcg
		if [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ] \
			&& qemu-system-aarch64 -accel help 2>/dev/null | grep -qx 'hvf'; then
			R9_ACCEL=hvf
		fi
	fi
	R9_MACHINE="virt,gic-version=3"
	R9_DTB_NAME=virt-gicv3.dtb
	case "$R9_ACCEL" in
		hvf|kvm) R9_CPU=host ;;
		tcg) R9_CPU=cortex-a72 ;;
		*) err "R9_ACCEL=$R9_ACCEL: want auto|hvf|kvm|tcg" ;;
	esac
fi

R9_DTB_SRC="$R9_KERNEL_PKG/aarch64/lib/$R9_DTB_NAME"
if [ ! -f "$R9_DTB_SRC" ]; then
	R9_DTB_SRC=$(find "$R9_KERNEL_PKG" -name "$R9_DTB_NAME" -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_DTB_SRC" ] && [ -f "$R9_DTB_SRC" ] || err "$R9_DTB_NAME not found under $R9_KERNEL_PKG"

echo "== r9 tree:     $HARVEY"
echo "== r9 root:     $R9_ROOT"
echo "== gate output: $R9_GATE_OUT"
echo "== kernel:      $R9_KERNEL"
echo "== dtb:         $R9_DTB_SRC"
echo "== shape:       $R9_MACHINE accel=$R9_ACCEL cpu=$R9_CPU"

# --- step 1: the bootfs image (r9's assembler replaces Harvey's bake) --

mkdir -p "$R9_GATE_OUT"
echo "== step 1/3: assembling image bootfs (cargo xtask image --arch aarch64 --image bootfs)"
( cd "$R9_ROOT" && cargo xtask image --arch aarch64 --image bootfs ) \
	|| err "bootfs image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
R9_INITRD="$R9_ROOT/target/images/aarch64/bootfs.paq"
[ -s "$R9_INITRD" ] || err "$R9_INITRD missing/empty after assembly (the bootfs manifest bakes it)"
echo "== image ok: $R9_INITRD"

# --- step 2: DTB patch (the Phase-4 fdtput recipe; see the header) ----

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
R9_RUN_TAG="r9smpstress-$$-$(date +%s)"
export R9_RUN_TAG
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	pkill -f "[q]emu-system-aarch64.*${R9_RUN_TAG}" 2>/dev/null || true
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT

DTB="$TMPD/virt-patched.dtb"
cp "$R9_DTB_SRC" "$DTB"
chmod u+w "$DTB"
SIZE=$(wc -c < "$R9_INITRD" | tr -d ' ')
START=$((0x48000000))
END=$((START + SIZE))
HI=$(printf '0x%x' $((START >> 32)))
LO=$(printf '0x%x' $((START & 0xffffffff)))
fdtput -t x "$DTB" /chosen linux,initrd-start "$HI" "$LO" \
	|| err "fdtput linux,initrd-start patch failed"
HI=$(printf '0x%x' $((END >> 32)))
LO=$(printf '0x%x' $((END & 0xffffffff)))
fdtput -t x "$DTB" /chosen linux,initrd-end "$HI" "$LO" \
	|| err "fdtput linux,initrd-end patch failed"

# SMP-08/D-28: the exact `aeadloop=1` token on /chosen/bootargs starts the
# in-kernel 9P2000.aead loopback server (#M).  Written as a string property
# on the throwaway DTB copy; the kernel's DTB bootconf reader seeds the
# command line from it.
R9_BOOTARGS="${R9_BOOTARGS:-aeadloop=1}"
R9_BOOTARGS_PROP="${R9_BOOTARGS_PROP:-bootargs}"
echo "   bootargs: /chosen/$R9_BOOTARGS_PROP = '$R9_BOOTARGS'"
fdtput -t s "$DTB" /chosen "$R9_BOOTARGS_PROP" "$R9_BOOTARGS" \
	|| err "fdtput /chosen $R9_BOOTARGS_PROP patch failed"

# --- step 3: the expect-driven stress ---------------------------------

R9_SMOKE_LOG="${R9_SMOKE_LOG:-$R9_GATE_OUT/smp-stress-r9.log}"
R9_QEMU="${R9_QEMU:-qemu-system-aarch64}"

rm -f "$R9_SMOKE_LOG"

echo "== step 2/3: DTB patched at $DTB"
echo "== step 3/3: smp-stress ($R9_SMOKE_DRIVER)"
echo "== capture: $R9_SMOKE_LOG"

export R9_KERNEL R9_INITRD R9_SMOKE_LOG R9_QEMU
R9_DTB="$DTB"
export R9_DTB
export R9_MACHINE R9_CPU R9_ACCEL

set +e
expect "$R9_SMOKE_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "smp-stress FAILED (the driver named the stage on stderr; capture: $R9_SMOKE_LOG)"
fi
echo "== smp-stress PASS — capture retained at $R9_SMOKE_LOG"
exit 0
