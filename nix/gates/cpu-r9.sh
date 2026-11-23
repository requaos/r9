#!/usr/bin/env bash
# nix/gates/cpu-r9.sh — the arm64 cpu fixture gate as an r9 flake app
# (plan 01-30, D-07/D-09/D-16/D-19, HARN-08; moved from Harvey bd7a698's
# nix/cpu-r9.sh, 20-16/ARM-11).  ONE arm64 boot exercises every cpu-port
# arm live:
#
#   - the boot string (/chosen/bootargs = init=/arm64/init -c, 20-13)
#     reaches the kernel's env seed (`devenv: bootconf ...`), so boot
#     execs init -c and init's cpustart runs /rc/bin/cpurc;
#   - cpurc itself, and /lib/shebangecho, run through the kernel's #!
#     exec pre-pass (20-14);
#   - wstat, fwstat and remove validate the stat buffer and refuse by
#     name (cpuprobe stat, 20-14);
#   - devcap accepts a caphash write, refuses a bogus capuse with its
#     own message and removes caphash (cpuprobe cap + rm, 20-15);
#   - devssl carries a round trip and a mount (sslprobe -m, 20-15).
#
# The terminal CONTROL leg is boot-r9-smoke: the same image, the
# kernel-virt (virtshell) kernel and no boot string, so init -t runs
# termrc and cpurc never runs.
#
# The pipeline is nix/gates/smoke-r9.sh's (image -> DTB patch -> expect),
# with three differences:
#   1. R9_BOOTARGS defaults to `init=/arm64/init -c' (a caller's value
#      wins) and is written to /chosen/$R9_BOOTARGS_PROP (default
#      bootargs, the carrier 20-13 confirmed) of the DTB copy;
#   2. the capture is $R9_GATE_OUT/cpu-r9.log (env R9_CPU_LOG);
#   3. the driver is gates/aarch64/cpu-r9.expect.
#
# The image — `cargo xtask image --arch aarch64 --image bootfs` from the
# r9 checkout — replaces Harvey's in-checkout userland build and bake:
# the engine stages from the read-only flake tree and writes
# target/images/aarch64/bootfs.paq.  Nothing is written into the Harvey
# tree (HARN-02) and no namespace transport (lima) is involved.
#
# Env contract:
#   R9_HARVEY_DIR_ARM64 REQUIRED — the read-only arm64 tree carrying
#                  /HARVEY-TREE (the flake's harvey-tree-arm64); fixtures
#                  are read from it, nothing is written to it.
#   R9_ROOT        the r9 checkout root (default $PWD); the app is run
#                  from the repo root.
#   R9_KERNEL_PKG  REQUIRED — a store path holding kernel-virt.elf and
#                  aarch64/lib/virt-gicv{2,3}.dtb; the flake app exports
#                  self.packages.<system>.kernel-virt-cpu.
#   R9_MKPAQFS     REQUIRED — the host mkpaqfs the image engine bakes
#                  with (the flake's harvey-mkpaqfs).
#   R9_GATE_OUT    output dir (default target/gates/cpu-r9); the patched
#                  DTB copy and the capture live under it.
#   R9_CPU_LOG     default $R9_GATE_OUT/cpu-r9.log.
#   R9_CPU_DRIVER  default $R9_ROOT/gates/aarch64/cpu-r9.expect.
#   R9_QEMU        default qemu-system-aarch64.
#   R9_ACCEL       auto (default) | hvf | kvm | tcg.
#   R9_GICV2       =1 the gic-version=2 / cortex-a72 / tcg shape.
#   R9_BOOTARGS    default `init=/arm64/init -c'.
#   R9_BOOTARGS_PROP  default bootargs.
#
# Hygiene is smoke-r9.sh's: the capture is removed before launch, the
# driver kills qemu's process group on every exit path, and the EXIT
# trap sweeps any survivor keyed on this run's unique R9_RUN_TAG.
#
# Wrapper provides (the flake app's runtimeInputs): PLAN9 with $PLAN9/bin
# LAST on PATH; the Rust toolchain (cargo xtask image); qemu, dtc
# (fdtput), git and expect.  Standalone use needs the same tools on PATH.
#
# usage: nix run .#boot-r9-cpu   (from the r9 repo root)

set -e

HARVEY="${R9_HARVEY_DIR_ARM64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/cpu-r9}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and kernel package contract -----------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64 unset — run via the flake app (nix run .#boot-r9-cpu), which exports the read-only arm64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_ARM64 must name harvey-tree-arm64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/arm64/bootfs.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/arm64/bootfs.toml is missing — the bootfs image manifest is part of this checkout"
R9_CPU_DRIVER="${R9_CPU_DRIVER:-$R9_ROOT/gates/aarch64/cpu-r9.expect}"
[ -f "$R9_CPU_DRIVER" ] || err "R9_CPU_DRIVER=$R9_CPU_DRIVER: the cpu driver is missing"

[ -n "${R9_KERNEL_PKG:-}" ] || err "R9_KERNEL_PKG unset — run via the flake app (nix run .#boot-r9-cpu), which exports the r9 kernel-virt-cpu store path"
[ -d "$R9_KERNEL_PKG" ] || err "R9_KERNEL_PKG $R9_KERNEL_PKG: not a directory (unrealised store path?)"
[ -n "${R9_MKPAQFS:-}" ] || err "R9_MKPAQFS unset — run via the flake app (nix run .#boot-r9-cpu), which exports the host mkpaqfs"
[ -f "$R9_MKPAQFS" ] && [ -x "$R9_MKPAQFS" ] \
	|| err "R9_MKPAQFS $R9_MKPAQFS: not an executable file (want <pkg>/bin/mkpaqfs)"

R9_KERNEL="$R9_KERNEL_PKG/kernel-virt.elf"
if [ ! -f "$R9_KERNEL" ]; then
	# find fallback: the package's layout is the published contract, but
	# locate the ELF by name rather than failing blind on a layout drift
	R9_KERNEL=$(find "$R9_KERNEL_PKG" -name 'kernel-virt.elf' -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_KERNEL" ] && [ -f "$R9_KERNEL" ] || err "kernel-virt.elf not found under $R9_KERNEL_PKG"

# --- step 0b: machine/accel selection (same as smoke-r9.sh) -----------
# gicv3 is the default machine shape everywhere; accel prefers hvf on
# darwin-aarch64 (hvf rejects gicv2 on qemu 11.1.2) and falls back to
# TCG.  R9_ACCEL forces the accelerator; R9_GICV2=1 restores the v1.0
# reference shape (gic-version=2 + cortex-a72 + tcg) for bisection.

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
# The per-run unique tag — exported so cpu-r9.expect adds it to
# qemu's argv (-name), and this run's survivor sweep (cleanup below)
# keys on it.  The sweep NEVER matches a kernel-name class: a foreign
# r9 qemu (a dev's interactive boot-r9 uses the same kernel; a parallel
# CI job on a shared runner) must never be killed by this gate's
# teardown.
R9_RUN_TAG="r9cpu-$$-$(date +%s)"
export R9_RUN_TAG
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	# the expect driver kills qemu's process group on every exit path;
	# this trap sweeps any survivor.  the pattern is bracket-fixed
	# ([q]emu) so pkill cannot self-match this shell's or its own argv,
	# and keyed on THIS run's unique -name tag — never on the
	# kernel-path class.
	pkill -f "[q]emu-system-aarch64.*${R9_RUN_TAG}" 2>/dev/null || true
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT

DTB="$TMPD/virt-patched.dtb"
# cp duplicates the store file's READ-ONLY mode (r--r--r--) — make the
# copy writable or fdtput fails EPERM (nix/setup.sh's install_tool
# chmod u+w precedent for store-derived files)
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

# the cpu boot string (20-13's carrier): init=/arm64/init -c unless the
# caller set R9_BOOTARGS; written as a string property on /chosen of
# the throwaway copy for the kernel's DTB bootconf reader.
R9_BOOTARGS="${R9_BOOTARGS:-init=/arm64/init -c}"
R9_BOOTARGS_PROP="${R9_BOOTARGS_PROP:-bootargs}"
echo "   bootargs: /chosen/$R9_BOOTARGS_PROP = '$R9_BOOTARGS'"
fdtput -t s "$DTB" /chosen "$R9_BOOTARGS_PROP" "$R9_BOOTARGS" \
	|| err "fdtput /chosen $R9_BOOTARGS_PROP patch failed"

# --- step 3: the expect-driven cpu gate ------------------------------

R9_CPU_LOG="${R9_CPU_LOG:-$R9_GATE_OUT/cpu-r9.log}"
R9_QEMU="${R9_QEMU:-qemu-system-aarch64}"

# fresh capture per run BEFORE launch (a stale log is dead evidence)
rm -f "$R9_CPU_LOG"

echo "== step 2/3: DTB patched at $DTB"
echo "== step 3/3: cpu gate ($R9_CPU_DRIVER)"
echo "== capture: $R9_CPU_LOG"

export R9_KERNEL R9_INITRD R9_CPU_LOG R9_QEMU R9_BOOTARGS
R9_DTB="$DTB"
export R9_DTB
# The selected machine shape for the expect driver's spawn argv.
export R9_MACHINE R9_CPU R9_ACCEL

set +e
expect "$R9_CPU_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "cpu gate FAILED (the driver named the stage on stderr; capture: $R9_CPU_LOG)"
fi
echo "== cpu gate PASS — capture retained at $R9_CPU_LOG"
exit 0
