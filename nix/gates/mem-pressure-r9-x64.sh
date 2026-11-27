#!/usr/bin/env bash
# nix/gates/mem-pressure-r9-x64.sh — the packaged x86_64 small-memory pressure
# gate as an r9 flake app (phase 05, MEM-10/MEM-15/MEM-16/MEM-17, D-17).
#
# nix/gates/smoke-r9-x64.sh's chain-leg pipeline, changed so the guest boots
# with a REAL but SMALL managed frame pool below the evictor low-water mark:
#
#   * the composed ISO's GRUB bootconf append carries
#     `palloc_max_frames=2048` (R9_PALLOC_MAX_FRAMES) — an 8 MiB bounded
#     PAGE_ALLOC pool read from the Multiboot2 cmdline;
#   * the guest runs with `-m $R9_MEM` (default 256M) and `-smp 2`, safely
#     above the ~108 MiB static footprint (MOD_STASH_SIZE 64 MiB +
#     GLOBAL_HEAP_SIZE 32 MiB `.bss`) so the .bss/initrd still fit while the
#     managed pool stays tiny;
#   * a pinned-blocked reader on one CPU and a memory-pressure worker on the
#     other contend for the small pool.
#
# gates/x86_64/mem-pressure.expect asserts `palloc: managed_frames=2048`,
# drives the pinned-buffer reader + pressure worker + byte-identical re-fault
# from Harvey's rc shell, reads `cat '#c/sysstat'` (served read-only by
# port/src/devcons.rs from port::palloc::evictor_stats()), and requires
# dropped_text > 0, dropped_data > 0, pinned_skipped > 0 and MEM_PRESSURE_OK.
#
# Env contract:
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree (harvey-tree-amd64).
#   R9_ROOT             the r9 checkout root (default $PWD).
#   R9_KERNEL_X64_PKG   REQUIRED — the kernel-x64 package store path.
#   R9_ISO_BUILDER      REQUIRED — r9's iso-builder executable.
#   R9_MKPAQFS          REQUIRED — the host mkpaqfs.
#   R9_GATE_OUT         output dir (default target/gates/mem-pressure-r9-x64).
#   R9_SMOKE_LOG        default $R9_GATE_OUT/mem-pressure-r9-x64.log.
#   R9_SMOKE_DRIVER     default $R9_ROOT/gates/x86_64/mem-pressure.expect.
#   R9_QEMU             default qemu-system-x86_64.
#   R9_ACCEL            kvm (default) | tcg — no silent downgrade.
#   R9_CPU              override the cpu model (default host/kvm, qemu64/tcg).
#   R9_MEM              guest RAM (default 256M — above the ~108 MiB `.bss`).
#   R9_PALLOC_MAX_FRAMES  the managed-frame cap (default 2048 = 8 MiB).
#   KERNDATE            bake-time timestamp override.
#
# Hygiene: capture rm -f'd BEFORE launch; group-kill on every exit path; the
# EXIT trap sweeps survivors via a bracket-fixed pkill keyed on R9_RUN_TAG.
#
# usage: nix run .#mem-pressure-r9-x64   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/mem-pressure-r9-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#mem-pressure-r9-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/chain.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/chain.toml is missing — the chain image manifest is part of this checkout"
R9_SMOKE_DRIVER="${R9_SMOKE_DRIVER:-$R9_ROOT/gates/x86_64/mem-pressure.expect}"
[ -f "$R9_SMOKE_DRIVER" ] || err "R9_SMOKE_DRIVER=$R9_SMOKE_DRIVER: the pressure driver is missing"

# --- step 0: the kernel-package + builder + assembler contract ---------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_MKPAQFS; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#mem-pressure-r9-x64), which exports the r9 supply"
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

R9_MEM="${R9_MEM:-256M}"
R9_PALLOC_MAX_FRAMES="${R9_PALLOC_MAX_FRAMES:-2048}"
export R9_MEM R9_PALLOC_MAX_FRAMES

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
echo "== shape:        q35 accel=$R9_ACCEL cpu=$R9_CPU mem=$R9_MEM"
echo "== palloc cap:   palloc_max_frames=$R9_PALLOC_MAX_FRAMES (8 MiB managed pool)"

# --- step 1: the chain image ------------------------------------------

mkdir -p "$R9_GATE_OUT"
echo "== step 1/3: assembling image chain (cargo xtask image --arch x86-64 --image chain)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image chain ) \
	|| err "chain image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
CHAIN_PAQ="$R9_ROOT/target/images/x86_64/chain.paq"
[ -s "$CHAIN_PAQ" ] || err "$CHAIN_PAQ missing/empty after assembly (the chain manifest bakes it)"
echo "== image ok: $CHAIN_PAQ"

# --- step 2: compose the ISO with the palloc_max_frames append --------

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
R9_RUN_TAG="r9x64mempress-$$-$(date +%s)"
export R9_RUN_TAG
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT

echo "== step 2/3: composing the ISO (r9's iso-builder: kernel + chain.paq as /boot/bootfs.paq + --append=\"palloc_max_frames=${R9_PALLOC_MAX_FRAMES}\")"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" --append="palloc_max_frames=${R9_PALLOC_MAX_FRAMES}" "$CHAIN_PAQ" bootfs.paq \
	|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 3: the expect-driven pressure load --------------------------

R9_SMOKE_LOG="${R9_SMOKE_LOG:-$R9_GATE_OUT/mem-pressure-r9-x64.log}"
R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"

rm -f "$R9_SMOKE_LOG"

echo "== step 3/3: mem-pressure ($R9_SMOKE_DRIVER)"
echo "== capture: $R9_SMOKE_LOG"

export R9_ISO R9_SMOKE_LOG R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL

set +e
expect "$R9_SMOKE_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "mem-pressure FAILED (the driver named the stage on stderr; capture: $R9_SMOKE_LOG)"
fi
echo "== mem-pressure PASS — capture retained at $R9_SMOKE_LOG"
exit 0
