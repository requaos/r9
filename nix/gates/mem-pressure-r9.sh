#!/usr/bin/env bash
# nix/gates/mem-pressure-r9.sh — the packaged aarch64 small-memory pressure
# gate as an r9 flake app (phase 05, MEM-10/MEM-15/MEM-16/MEM-17, D-17).
#
# The composed pipeline of nix/gates/smoke-r9.sh, changed in the DTB patch
# step so the guest boots with a REAL but SMALL managed frame pool below the
# evictor's low-water mark:
#
#   * the throwaway DTB's /chosen/bootargs carries `palloc_max_frames=2048`
#     (R9_PALLOC_MAX_FRAMES) — an 8 MiB bounded PAGE_ALLOC pool (the cap
#     parse in port::palloc::parse_max_frames_bootarg);
#   * the throwaway DTB's /memory@40000000 reg is patched to 256 MiB
#     (R9_MEM, default 256M) so the initrd loaded at 0x48000000 (a 128 MiB
#     offset) always lies squarely inside guest RAM [0x40000000, 0x50000000);
#   * the guest runs with `-smp 2` so the burst's hseq reader and hsort
#     allocator contend for the small pool on separate CPUs.
#
# gates/aarch64/mem-pressure.expect asserts `palloc: managed_frames=2048`,
# drives a BOUNDED `hseq 1 44000 | hsort` burst (R9_PRESSURE_LINES) and a
# byte-identical re-fault from Harvey's rc shell,
# reads `cat '#c/sysstat'` (served read-only by port/src/devcons.rs from
# port::palloc::evictor_stats()), and requires dropped_text > 0,
# dropped_data > 0, evict_calls > 0, reclaimed > 0, free_failures == 0 and
# MEM_PRESSURE_OK.  pinned_skipped is reported but not asserted: no
# userland path in this image can pin a cached page (the 05-21 structural
# finding); the host integration test
# port/src/pagecache.rs::pinned_cached_page_is_skipped_until_unpinned proves
# the pin path instead.
#
# Env contract:
#   R9_HARVEY_DIR_ARM64 REQUIRED — the read-only arm64 tree (harvey-tree-arm64).
#   R9_ROOT        the r9 checkout root (default $PWD).
#   R9_KERNEL_PKG  REQUIRED — the r9 kernel-virt package store path.
#   R9_MKPAQFS     REQUIRED — the host mkpaqfs.
#   R9_GATE_OUT    output dir (default target/gates/mem-pressure-r9).
#   R9_SMOKE_LOG   default $R9_GATE_OUT/mem-pressure-r9.log.
#   R9_SMOKE_DRIVER  default $R9_ROOT/gates/aarch64/mem-pressure.expect.
#   R9_QEMU        default qemu-system-aarch64.
#   R9_ACCEL       auto (default) | hvf | kvm | tcg.
#   R9_GICV2       =1 restores the gicv2/cortex-a72/tcg bisection shape.
#   R9_MEM         guest RAM (default 256M — above the aarch64 static
#                  footprint and covering the 0x48000000 initrd).
#   R9_PALLOC_MAX_FRAMES  the managed-frame cap (default 2048 = 8 MiB).
#   R9_PRESSURE_LINES     the bounded pressure burst's line count (default
#                         44000; measured on HVF and TCG: evict_calls=4,
#                         reclaimed=64, dropped_text=29, dropped_data=35).
#   R9_BOOTARGS    the /chosen bootargs (default palloc_max_frames=…).
#   R9_BOOTARGS_PROP  the /chosen property name (default bootargs).
#
# Hygiene: capture rm -f'd BEFORE launch; group-kill on every exit path; the
# EXIT trap sweeps survivors via a bracket-fixed pkill keyed on R9_RUN_TAG.
#
# usage: nix run .#mem-pressure-r9   (from the r9 repo root)

set -e

HARVEY="${R9_HARVEY_DIR_ARM64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/mem-pressure-r9}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and kernel package contract -----------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64 unset — run via the flake app (nix run .#mem-pressure-r9), which exports the read-only arm64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_ARM64 must name harvey-tree-arm64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/arm64/bootfs.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/arm64/bootfs.toml is missing — the bootfs image manifest is part of this checkout"
R9_SMOKE_DRIVER="${R9_SMOKE_DRIVER:-$R9_ROOT/gates/aarch64/mem-pressure.expect}"
[ -f "$R9_SMOKE_DRIVER" ] || err "R9_SMOKE_DRIVER=$R9_SMOKE_DRIVER: the pressure driver is missing"

[ -n "${R9_KERNEL_PKG:-}" ] || err "R9_KERNEL_PKG unset — run via the flake app (nix run .#mem-pressure-r9), which exports the r9 kernel-virt store path"
[ -d "$R9_KERNEL_PKG" ] || err "R9_KERNEL_PKG $R9_KERNEL_PKG: not a directory (unrealised store path?)"
[ -n "${R9_MKPAQFS:-}" ] || err "R9_MKPAQFS unset — run via the flake app (nix run .#mem-pressure-r9), which exports the host mkpaqfs"
[ -f "$R9_MKPAQFS" ] && [ -x "$R9_MKPAQFS" ] \
	|| err "R9_MKPAQFS $R9_MKPAQFS: not an executable file (want <pkg>/bin/mkpaqfs)"

R9_KERNEL="$R9_KERNEL_PKG/kernel-virt.elf"
if [ ! -f "$R9_KERNEL" ]; then
	R9_KERNEL=$(find "$R9_KERNEL_PKG" -name 'kernel-virt.elf' -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_KERNEL" ] && [ -f "$R9_KERNEL" ] || err "kernel-virt.elf not found under $R9_KERNEL_PKG"

# --- step 0b: machine/accel selection ---------------------------------

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

R9_MEM="${R9_MEM:-256M}"
R9_PALLOC_MAX_FRAMES="${R9_PALLOC_MAX_FRAMES:-2048}"
R9_PRESSURE_LINES="${R9_PRESSURE_LINES:-44000}"
export R9_MEM R9_PALLOC_MAX_FRAMES R9_PRESSURE_LINES

echo "== r9 tree:     $HARVEY"
echo "== r9 root:     $R9_ROOT"
echo "== gate output: $R9_GATE_OUT"
echo "== kernel:      $R9_KERNEL"
echo "== dtb:         $R9_DTB_SRC"
echo "== shape:       $R9_MACHINE accel=$R9_ACCEL cpu=$R9_CPU mem=$R9_MEM"
echo "== palloc cap:  palloc_max_frames=$R9_PALLOC_MAX_FRAMES (8 MiB managed pool)"

# --- step 1: the bootfs image -----------------------------------------

mkdir -p "$R9_GATE_OUT"
echo "== step 1/3: assembling image bootfs (cargo xtask image --arch aarch64 --image bootfs)"
( cd "$R9_ROOT" && cargo xtask image --arch aarch64 --image bootfs ) \
	|| err "bootfs image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
R9_INITRD="$R9_ROOT/target/images/aarch64/bootfs.paq"
[ -s "$R9_INITRD" ] || err "$R9_INITRD missing/empty after assembly (the bootfs manifest bakes it)"
echo "== image ok: $R9_INITRD"

# --- step 2: DTB patch (initrd cells, memory shrink, bootargs) --------

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
R9_RUN_TAG="r9mempress-$$-$(date +%s)"
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

# MEM-15/D-17: shrink /memory@40000000 to 256 MiB (0x40000000..0x50000000)
# so the initrd at 0x48000000 is squarely inside guest RAM and the managed
# pool is small.  The reg property is two (base,size) 64-bit cells.
echo "   memory: /memory@40000000 reg = 0x40000000 + 0x10000000 (256 MiB)"
fdtput -t x "$DTB" /memory@40000000 reg 0x0 0x40000000 0x0 0x10000000 \
	|| err "fdtput /memory@40000000 reg patch failed"

# MEM-15/D-17: constrain the managed pool to an 8 MiB frame cap.
R9_BOOTARGS="${R9_BOOTARGS:-palloc_max_frames=${R9_PALLOC_MAX_FRAMES}}"
R9_BOOTARGS_PROP="${R9_BOOTARGS_PROP:-bootargs}"
echo "   bootargs: /chosen/$R9_BOOTARGS_PROP = '$R9_BOOTARGS'"
fdtput -t s "$DTB" /chosen "$R9_BOOTARGS_PROP" "$R9_BOOTARGS" \
	|| err "fdtput /chosen $R9_BOOTARGS_PROP patch failed"

# --- step 3: the expect-driven pressure load --------------------------

R9_SMOKE_LOG="${R9_SMOKE_LOG:-$R9_GATE_OUT/mem-pressure-r9.log}"
R9_QEMU="${R9_QEMU:-qemu-system-aarch64}"

rm -f "$R9_SMOKE_LOG"

echo "== step 2/3: DTB patched at $DTB"
echo "== step 3/3: mem-pressure ($R9_SMOKE_DRIVER)"
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
	err "mem-pressure FAILED (the driver named the stage on stderr; capture: $R9_SMOKE_LOG)"
fi
echo "== mem-pressure PASS — capture retained at $R9_SMOKE_LOG"
exit 0
