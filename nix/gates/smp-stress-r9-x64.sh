#!/usr/bin/env bash
# nix/gates/smp-stress-r9-x64.sh — the packaged x86_64 two-CPU stress gate as
# an r9 flake app (phase 05, SMP-08/SMP-09/MEM-17, D-24/D-27/D-28).
#
# nix/gates/smoke-r9-x64.sh's chain-leg pipeline, changed in exactly two
# places: the composed ISO is built with the bootconf append
# `--append="aeadloop=1"` (the real x86_64 GRUB Multiboot2 cmdline delivery —
# the exact nix/gates/smoke-r9-x64.sh:223 mechanism; NEVER a QEMU `-append`
# with `-cdrom`), so the in-kernel 9P2000.aead loopback server starts and
# publishes #M (D-28); and the driver is gates/x86_64/smp-stress.expect,
# which drives concurrent opens/reads/clunks over '#M/data0'..'#M/data7' plus
# exec churn across both CPUs under timer interrupts (D-27).
#
# The pipeline, each step loud on failure:
#
#   1. the image — `cargo xtask image --arch x86-64 --image smpstress`
#      writes target/images/x86_64/smpstress.paq (the chain image plus
#      wc(1); nothing is written into the Harvey tree, HARN-02).
#   2. compose the ISO — r9's exported iso-builder (R9_ISO_BUILDER) builds
#      the ISO from the pinned kernel-x64 + the smp-stress bake as
#      /boot/bootfs.paq, with `--append="aeadloop=1"` on the GRUB bootconf.
#   3. smp-stress — gates/x86_64/smp-stress.expect spawns qemu with `-M q35`,
#      `-smp 2` and the composed ISO on `-cdrom` (no QEMU `-append`) and
#      asserts the SMP, SIMD and aeadloop markers, drives the #M read/clunk
#      + exec-churn load from Harvey's rc shell, and proves SMP_STRESS_OK
#      with zero watchdog/panic/rank faults.
#
# Env contract:
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree (harvey-tree-amd64).
#   R9_ROOT             the r9 checkout root (default $PWD).
#   R9_KERNEL_X64_PKG   REQUIRED — the kernel-x64 package store path.
#   R9_ISO_BUILDER      REQUIRED — r9's iso-builder executable.
#   R9_MKPAQFS          REQUIRED — the host mkpaqfs (image bake).
#   R9_GATE_OUT         output dir (default target/gates/smp-stress-r9-x64).
#   R9_SMOKE_LOG        default $R9_GATE_OUT/smp-stress-r9-x64.log.
#   R9_SMOKE_DRIVER     default $R9_ROOT/gates/x86_64/smp-stress.expect.
#   R9_QEMU             default qemu-system-x86_64.
#   R9_ACCEL            kvm (default) | tcg — no silent downgrade.
#   R9_CPU              override the cpu model (default host/kvm, qemu64/tcg).
#   KERNDATE            bake-time timestamp override.
#
# Hygiene: the capture is rm -f'd BEFORE launch; the expect driver kills the
# spawn's process group on every exit path; this script's EXIT trap sweeps any
# survivor via a bracket-fixed pkill keyed on R9_RUN_TAG.
#
# usage: nix run .#smp-stress-r9-x64   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/smp-stress-r9-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#smp-stress-r9-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/smpstress.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/smpstress.toml is missing — the smp-stress image manifest is part of this checkout"
R9_SMOKE_DRIVER="${R9_SMOKE_DRIVER:-$R9_ROOT/gates/x86_64/smp-stress.expect}"
[ -f "$R9_SMOKE_DRIVER" ] || err "R9_SMOKE_DRIVER=$R9_SMOKE_DRIVER: the stress driver is missing"

# --- step 0: the kernel-package + builder + assembler contract ---------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_MKPAQFS; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#smp-stress-r9-x64), which exports the r9 supply"
done
[ -d "$R9_KERNEL_X64_PKG" ] || err "R9_KERNEL_X64_PKG $R9_KERNEL_X64_PKG: not a directory (unrealised store path?)"

R9_KERNEL="$R9_KERNEL_X64_PKG/kernel-x64.elf"
if [ ! -f "$R9_KERNEL" ]; then
	R9_KERNEL=$(find "$R9_KERNEL_X64_PKG" -name 'kernel-x64.elf' -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_KERNEL" ] && [ -f "$R9_KERNEL" ] || err "kernel-x64.elf not found under $R9_KERNEL_X64_PKG"

[ -f "$R9_ISO_BUILDER" ] && [ -x "$R9_ISO_BUILDER" ] \
	|| err "R9_ISO_BUILDER $R9_ISO_BUILDER: not an executable file (want <pkg>/bin/r9-iso-builder)"
[ -f "$R9_MKPAQFS" ] && [ -x "$R9_MKPAQFS" ] \
	|| err "R9_MKPAQFS $R9_MKPAQFS: not an executable file (want <pkg>/bin/mkpaqfs)"

# --- step 0b: accel selection (no silent downgrade) --------------------

R9_ACCEL="${R9_ACCEL:-kvm}"
case "$R9_ACCEL" in
	kvm)
		[ -e /dev/kvm ] || err "/dev/kvm absent and R9_ACCEL=$R9_ACCEL — no silent downgrade: set R9_ACCEL=tcg explicitly for the TCG fallback"
		R9_CPU="${R9_CPU:-host}"
		;;
	tcg)
		R9_CPU="${R9_CPU:-qemu64}"
		;;
	*) err "R9_ACCEL=$R9_ACCEL: want kvm|tcg" ;;
esac

export KERNDATE="${KERNDATE:-$(date +%s)}"

echo "== r9 tree:      $HARVEY"
echo "== r9 root:      $R9_ROOT"
echo "== gate output:  $R9_GATE_OUT"
echo "== kernel:       $R9_KERNEL"
echo "== iso-builder:  $R9_ISO_BUILDER"
echo "== shape:        q35 accel=$R9_ACCEL cpu=$R9_CPU"

# --- step 1: the smp-stress image (r9's assembler) -------------------

mkdir -p "$R9_GATE_OUT"
echo "== step 1/3: assembling image smpstress (cargo xtask image --arch x86-64 --image smpstress)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image smpstress ) \
	|| err "smpstress image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
IMAGE_PAQ="$R9_ROOT/target/images/x86_64/smpstress.paq"
[ -s "$IMAGE_PAQ" ] || err "$IMAGE_PAQ missing/empty after assembly (the smpstress manifest bakes it)"
echo "== image ok: $IMAGE_PAQ"

# --- step 2: compose the ISO with the aeadloop bootconf append --------

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
R9_RUN_TAG="r9x64smpstress-$$-$(date +%s)"
export R9_RUN_TAG
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT

echo "== step 2/3: composing the ISO (r9's iso-builder: kernel + smpstress.paq as /boot/bootfs.paq + --append=\"aeadloop=1\")"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" --append="aeadloop=1" "$IMAGE_PAQ" bootfs.paq \
	|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 3: the expect-driven stress ---------------------------------

R9_SMOKE_LOG="${R9_SMOKE_LOG:-$R9_GATE_OUT/smp-stress-r9-x64.log}"
R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"

rm -f "$R9_SMOKE_LOG"

echo "== step 3/3: smp-stress ($R9_SMOKE_DRIVER)"
echo "== capture: $R9_SMOKE_LOG"

export R9_ISO R9_SMOKE_LOG R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL

set +e
expect "$R9_SMOKE_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "smp-stress FAILED (the driver named the stage on stderr; capture: $R9_SMOKE_LOG)"
fi
echo "== smp-stress PASS — capture retained at $R9_SMOKE_LOG"
exit 0
