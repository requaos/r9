#!/usr/bin/env bash
# nix/gates/boot-r9.sh — the plain aarch64 boot as an r9 flake app
# (plan 01-21, D-07/D-09/D-16/D-19, HARN-08).
#
# Moved from Harvey bd7a698's nix/boot-r9.sh and rewired onto r9's own
# supply:
#
#   1. the image — `cargo xtask image --arch aarch64 --image bootfs`
#      from the r9 checkout replaces Harvey's in-checkout userland build
#      and bake.  The image engine stages from the read-only flake tree
#      and writes target/images/aarch64/bootfs.paq.  Nothing is written
#      into the Harvey tree (HARN-02), and no namespace transport is
#      involved.
#   2. DTB patch: a throwaway copy of the package's selected DTB
#      (virt-gicv3.dtb by default) gets /chosen linux,initrd-start/-end as
#      two 32-bit BE cells, high word first (dtc has no 8-byte property
#      type).  Mirrors r9's xtask/src/config.rs prepare_initrd_dtb.  The
#      copy lives under R9_GATE_OUT.
#   3. launch — qemu argv from the selection below (the r9 kernel
#      dispatches its GIC driver on the DTB's compatible string, so one
#      kernel-virt.elf serves both machine shapes):
#        qemu-system-aarch64 -M virt,gic-version=3 -cpu $R9_CPU
#          -smp 1 -m 1024M -accel $R9_ACCEL -kernel <r9 ELF>
#          -dtb <patched copy>
#          -device loader,file=<bootfs.paq>,addr=0x48000000
#          -netdev user,id=n0 -device virtio-net-pci-non-transitional,netdev=n0
#          -nographic
#      Default: gicv3 everywhere; -accel hvf -cpu host on darwin-aarch64
#      (hvf rejects gicv2), TCG + cortex-a72 elsewhere.  qemu runs in the
#      FOREGROUND with stdio inherited — the user's terminal IS the
#      console (ctrl-a x to quit).  A foreground child rather than exec so
#      the EXIT trap reclaims the patched-DTB run directory (same console
#      semantics: qemu is the group's foreground process).
#
# Env contract:
#   R9_HARVEY_DIR_ARM64 REQUIRED — the read-only arm64 tree carrying
#                  /HARVEY-TREE (the flake's harvey-tree-arm64); fixtures
#                  are read from it, nothing is written to it.
#   R9_ROOT        the r9 checkout root (default $PWD); the app is run
#                  from the repo root.
#   R9_KERNEL_PKG  REQUIRED — the r9 kernel-virt package's store path;
#                  the flake app exports it from
#                  self.packages.<system>.kernel-virt.  The kernel and DTB
#                  arrive ONLY via this nix-controlled, read-only store
#                  path (no caller-supplied kernel path is exec'd;
#                  presence is validated before launch).
#   R9_MKPAQFS     REQUIRED — the host mkpaqfs the image engine bakes
#                  with (the flake's harvey-mkpaqfs).
#   R9_GATE_OUT    output dir (default target/gates/boot-r9); the patched
#                  DTB copy lives under it.
#   R9_ACCEL       auto (default) | hvf | kvm | tcg — auto picks hvf on
#                  darwin-aarch64 when qemu supports it, tcg otherwise.
#   R9_GICV2       =1 restores the v1.0 reference shape (gic-version=2,
#                  cortex-a72, tcg) — a bisection escape hatch.
#   R9_BOOTARGS    optional boot string for the kernel's ROOT env seed
#                  (key=value pairs, e.g. "init=/arm64/init -c"); when
#                  set and non-empty, step 2 writes it with fdtput -t s
#                  onto /chosen of the DTB copy.  Unset or empty (the
#                  default): no extra patch runs and the DTB copy is
#                  byte-identical to the two-patch default.
#   R9_BOOTARGS_PROP  the /chosen property name R9_BOOTARGS is written
#                  to (default bootargs); only read when R9_BOOTARGS is
#                  non-empty.
#
# Wrapper provides (the flake app's runtimeInputs): PLAN9 with $PLAN9/bin
# LAST on PATH; the Rust toolchain (cargo xtask image); qemu, dtc
# (fdtput), git and expect.  Standalone use needs the same tools on PATH.
#
# usage: nix run .#boot-r9   (from the r9 repo root)

set -e

HARVEY="${R9_HARVEY_DIR_ARM64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/boot-r9}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and kernel package contract -----------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64 unset — run via the flake app (nix run .#boot-r9), which exports the read-only arm64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_ARM64 must name harvey-tree-arm64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/arm64/bootfs.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/arm64/bootfs.toml is missing — the bootfs image manifest is part of this checkout"

[ -n "${R9_KERNEL_PKG:-}" ] || err "R9_KERNEL_PKG unset — run via the flake app (nix run .#boot-r9), which exports the r9 kernel-virt store path"
[ -d "$R9_KERNEL_PKG" ] || err "R9_KERNEL_PKG $R9_KERNEL_PKG: not a directory (unrealised store path?)"
[ -n "${R9_MKPAQFS:-}" ] || err "R9_MKPAQFS unset — run via the flake app (nix run .#boot-r9), which exports the host mkpaqfs"
[ -f "$R9_MKPAQFS" ] && [ -x "$R9_MKPAQFS" ] \
	|| err "R9_MKPAQFS $R9_MKPAQFS: not an executable file (want <pkg>/bin/mkpaqfs)"

R9_KERNEL="$R9_KERNEL_PKG/kernel-virt.elf"
if [ ! -f "$R9_KERNEL" ]; then
	# find fallback: the package's layout is the published contract, but
	# locate the ELF by name rather than failing blind on a layout drift
	R9_KERNEL=$(find "$R9_KERNEL_PKG" -name 'kernel-virt.elf' -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_KERNEL" ] && [ -f "$R9_KERNEL" ] || err "kernel-virt.elf not found under $R9_KERNEL_PKG"

# --- step 0b: machine/accel selection ---------------------------------
# gicv3 is the default machine shape everywhere (the kernel dispatches
# on the DTB compatible string; virtexec3 gates gicv3-under-TCG);
# accel prefers hvf on darwin-aarch64 (hvf rejects gicv2 on qemu
# 11.1.2) and falls back to TCG.  R9_ACCEL forces the accelerator;
# R9_GICV2=1 restores the whole v1.0 reference shape for bisection.

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
TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	rm -rf "$TMPD"
}
trap cleanup EXIT

echo "== step 1/2: assembling image bootfs (cargo xtask image --arch aarch64 --image bootfs)"
( cd "$R9_ROOT" && cargo xtask image --arch aarch64 --image bootfs ) \
	|| err "bootfs image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
R9_INITRD="$R9_ROOT/target/images/aarch64/bootfs.paq"
[ -s "$R9_INITRD" ] || err "$R9_INITRD missing/empty after assembly (the bootfs manifest bakes it)"
echo "== image ok: $R9_INITRD"

# --- step 2: DTB patch + qemu launch ---------------------------------
# (the Phase-4 fdtput recipe — two 32-bit BE cells, high word first —
# and the step-0b-selected virt argv; see the header)

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

# opt-in boot string (20-13, ARM-10): with R9_BOOTARGS set and
# non-empty, one more fdtput writes it as a string property on /chosen
# of the throwaway copy (property name R9_BOOTARGS_PROP, default
# bootargs) for the kernel's DTB bootconf reader.  Unset or empty:
# nothing runs and the copy is byte-identical to the two-patch default.
if [ -n "${R9_BOOTARGS:-}" ]; then
	R9_BOOTARGS_PROP="${R9_BOOTARGS_PROP:-bootargs}"
	echo "   bootargs: /chosen/$R9_BOOTARGS_PROP = '$R9_BOOTARGS'"
	fdtput -t s "$DTB" /chosen "$R9_BOOTARGS_PROP" "$R9_BOOTARGS" \
		|| err "fdtput /chosen $R9_BOOTARGS_PROP patch failed"
fi

# D-10: the standard boot speaks Ethernet — the always-on NIC rides every
# composed boot (the flag shape Phase 9's HNET-02 single-sources into
# platform TOML; no filter-dump object here — capture stays a gate feature).
set -- qemu-system-aarch64 \
	-M "$R9_MACHINE" -cpu "$R9_CPU" -smp 1 -m 1024M -accel "$R9_ACCEL" \
	-kernel "$R9_KERNEL" \
	-dtb "$DTB" \
	-device "loader,file=$R9_INITRD,addr=0x48000000" \
	-netdev user,id=n0 \
	-device virtio-net-pci-non-transitional,netdev=n0 \
	-nographic

# always print the assembled argv so captured gate logs are
# self-describing (run-r9.sh's discipline)
echo "== step 2/2: launching qemu — interactive on stdio (ctrl-a x to quit)"
echo "== qemu argv: $*"

set +e
"$@"
RC=$?
set -e
exit "$RC"
