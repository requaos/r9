#!/usr/bin/env bash
# nix/gates/tier1-r9.sh — the r9-side aarch64 tier-1 gate as an r9 flake
# app (plan 01-21, D-07/D-09/D-16/D-19, HARN-08).  Moved from Harvey
# bd7a698's nix/tier1-r9.sh and rewired onto r9's own supply.  Runs the
# same composed pipeline as nix/gates/smoke-r9.sh but launches qemu under
# gates/aarch64/tier1-r9.expect, which types ONE sweep line at the %
# prompt (r9's devcons serves reads) and asserts the marker set against
# the serial wire.  The smoke's own behavior is untouched.
#
# The pipeline, each step loud on failure:
#
#   1. the image — `cargo xtask image --arch aarch64 --image bootfs`
#      from the r9 checkout replaces Harvey's in-checkout userland build
#      and bake.  The image engine stages from the read-only flake tree
#      and writes target/images/aarch64/bootfs.paq; the tier-1 payload
#      (/arm64/bin/h* binaries, /lib/tier1-sweep.rc, /lib/tier1/* fixed
#      inputs) rides the same manifest.  Nothing is written into the
#      Harvey tree (HARN-02).
#   2. DTB patch: a throwaway copy of the package's selected DTB
#      (virt-gicv3.dtb by default) gets /chosen linux,initrd-start/-end as
#      two 32-bit BE cells, high word first (dtc has no 8-byte property
#      type).  Mirrors r9's xtask/src/config.rs prepare_initrd_dtb.  The
#      copy lives under R9_GATE_OUT.
#   3. the gate — gates/aarch64/tier1-r9.expect spawns qemu with serial
#      on stdio (-nographic) and asserts: staged boot → one typed line
#      `rc /lib/tier1-sweep.rc r9` → staged marker waits (float canary
#      T1-OK float-d01 on BOTH kernels, D-01) → T1-DONE → the whole-file
#      post-pass (every expected T1-OK, the T1-SKIP set EQUAL to the
#      known write-flow/build-chain list, zero T1-FAIL, zero panic).
#      Markers are the ONLY proof — the supervisor's exit status is
#      never trusted.
#
# Env contract:
#   R9_HARVEY_DIR_ARM64 REQUIRED — the read-only arm64 tree carrying
#                  /HARVEY-TREE (the flake's harvey-tree-arm64); the
#                  tier lists and tier1-sweep.rc are read from it, nothing
#                  is written to it.
#   R9_ROOT        the r9 checkout root (default $PWD); the app is run
#                  from the repo root.
#   R9_KERNEL_PKG  REQUIRED — the r9 kernel-virt package's store path;
#                  the flake app exports it from
#                  self.packages.<system>.kernel-virt.  Kernel and DTB
#                  arrive ONLY via this nix-controlled, read-only store
#                  path (presence validated before launch).
#   R9_MKPAQFS     REQUIRED — the host mkpaqfs the image engine bakes
#                  with (the flake's harvey-mkpaqfs).
#   R9_GATE_OUT    output dir (default target/gates/tier1-r9); the
#                  patched DTB copy and the capture live under it.
#   R9_TIER1_LOG   default $R9_GATE_OUT/tier1-r9.log (the serial capture).
#   R9_TIER1_DRIVER  default $R9_ROOT/gates/aarch64/tier1-r9.expect.
#   R9_QEMU        default qemu-system-aarch64.
#   R9_ACCEL       auto (default) | hvf | kvm | tcg — auto picks hvf on
#                  darwin-aarch64 when qemu supports it, tcg otherwise.
#   R9_GICV2       =1 restores the v1.0 reference shape (gic-version=2,
#                  cortex-a72, tcg) — a bisection escape hatch.
#
# Hygiene: the capture is rm -f'd BEFORE launch (a pre-seeded stale log
# cannot pass — it starts empty and only this run's boot writes it); the
# expect driver kills qemu's process group on every exit path; THIS
# script's EXIT trap sweeps any survivor via a bracket-fixed pkill keyed
# on this run's unique R9_RUN_TAG (exported below; the driver adds it to
# qemu's argv as -name) — never a kernel-name class, so a foreign r9 qemu
# (a dev's interactive boot-r9, a parallel CI job) is never matched.  The
# '[q]emu…' pattern cannot self-match the harness shell's or pkill's own
# argv.
#
# Wrapper provides (the flake app's runtimeInputs): PLAN9 with $PLAN9/bin
# LAST on PATH; the Rust toolchain (cargo xtask image); qemu, dtc
# (fdtput), git, expect and the text tools the marker computation needs.
# Standalone use needs the same tools on PATH.
#
# usage: nix run .#boot-r9-tier1   (from the r9 repo root)

set -e

HARVEY="${R9_HARVEY_DIR_ARM64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/tier1-r9}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and kernel package contract -----------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64 unset — run via the flake app (nix run .#boot-r9-tier1), which exports the read-only arm64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_ARM64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_ARM64 must name harvey-tree-arm64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/arm64/bootfs.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/arm64/bootfs.toml is missing — the bootfs image manifest is part of this checkout"
R9_TIER1_DRIVER="${R9_TIER1_DRIVER:-$R9_ROOT/gates/aarch64/tier1-r9.expect}"
[ -f "$R9_TIER1_DRIVER" ] || err "R9_TIER1_DRIVER=$R9_TIER1_DRIVER: the tier-1 driver is missing"

[ -n "${R9_KERNEL_PKG:-}" ] || err "R9_KERNEL_PKG unset — run via the flake app (nix run .#boot-r9-tier1), which exports the r9 kernel-virt store path"
[ -d "$R9_KERNEL_PKG" ] || err "R9_KERNEL_PKG $R9_KERNEL_PKG: not a directory (unrealised store path?)"
[ -n "${R9_MKPAQFS:-}" ] || err "R9_MKPAQFS unset — run via the flake app (nix run .#boot-r9-tier1), which exports the host mkpaqfs"
[ -f "$R9_MKPAQFS" ] && [ -x "$R9_MKPAQFS" ] \
	|| err "R9_MKPAQFS $R9_MKPAQFS: not an executable file (want <pkg>/bin/mkpaqfs)"

R9_KERNEL="$R9_KERNEL_PKG/kernel-virt.elf"
if [ ! -f "$R9_KERNEL" ]; then
	# find fallback: the package's layout is the published contract, but
	# locate the ELF by name rather than failing blind on a layout drift
	R9_KERNEL=$(find "$R9_KERNEL_PKG" -name 'kernel-virt.elf' -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_KERNEL" ] && [ -f "$R9_KERNEL" ] || err "kernel-virt.elf not found under $R9_KERNEL_PKG"

# Resolve a host grep once (D-02): the marker-set computation needs
# -o/-E, which plan9 grep lacks — but a hardcoded /usr/bin/grep breaks
# on NixOS-class hosts, so fall back to PATH.
GREP=$(command -v /usr/bin/grep || command -v grep) || err "no host grep found"

# --- step 0b: machine/accel selection ---------------------------------
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
			&& qemu-system-aarch64 -accel help 2>/dev/null | "$GREP" -qx 'hvf'; then
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
echo "== step 1/4: assembling image bootfs (cargo xtask image --arch aarch64 --image bootfs)"
( cd "$R9_ROOT" && cargo xtask image --arch aarch64 --image bootfs ) \
	|| err "bootfs image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
R9_INITRD="$R9_ROOT/target/images/aarch64/bootfs.paq"
[ -s "$R9_INITRD" ] || err "$R9_INITRD missing/empty after assembly (the bootfs manifest bakes it)"
echo "== image ok: $R9_INITRD"

# --- step 2: the expected marker sets (computed, never a second
#     hardcoded list — subset-exec.sh's 06-06 rule).  The OK set: tier
#     entries map to marker names (basename; dir:target → target) minus
#     the never-swept members (the boot chain rides the 05-03 trace;
#     7c/7l/cc ride the canary markers) minus stacksmoke (its T1-OK is
#     the donor termrc's line; the r9 boot's own stacksmoke stage covers
#     it here) minus every SKIP-named member (the donor-guarded legs
#     emit T1-SKIP on r9) — plus the fixed markers float-d01 (D-01),
#     libtest-tdraw, forkstress, writeprobe, cexecprobe, cexeccreate,
#     rcloseprobe, brkprobe and fpprobe.
#     These are fixed because no tiers/tier1-cmds.txt entry names them:
#     float-d01 is a canary check, libtest-tdraw a libtest binary,
#     forkstress (20-03, the WINDOWS #49 pin) a build/aarch64 fixture
#     baked only into the r9 image, writeprobe (20-08, the pin for the
#     aarch64 rfork fd-group split, devmnt write and DMDIR/union create
#     ports) the r9-only write-flow probe over a ramfs mount, cexecprobe
#     (20-10, the pin for the aarch64 close-on-exec sweep), cexeccreate
#     (20-11, `cexecprobe -c', the pin for r9 437b752's create(OCEXEC)
#     on both create arms), rcloseprobe (20-11, the pin for r9
#     5629cf0's ORCLOSE remove-on-close on /srv and /env), brkprobe
#     (20-07/20-10, the lone-member RFMEM brk pin, D-09) and fpprobe
#     (the pin for r9 2edfdf8's per-proc user FP/SIMD state) r9-only
#     build/aarch64 fixtures.  Listing them here makes a missing T1-OK
#     forkstress, writeprobe, cexecprobe, cexeccreate, rcloseprobe,
#     brkprobe or fpprobe red the post-pass.  The SKIP set is extracted
#     VERBATIM from tier1-sweep.rc's r9-venue SKIP lines (the file is the
#     source of truth). ---
T1_SKIP=$("$GREP" -oE '(^|[[:space:]])echo T1-SKIP [A-Za-z0-9.-]+' "$HARVEY/build/aarch64/tier1-sweep.rc" \
	| awk '{print $NF}' | sort -u | tr '\n' ' ')
T1_OK_ALL=$(
	awk 'NF && $1 !~ /^#/ { print $1 }' "$HARVEY/build/aarch64/tiers/tier1-cmds.txt" |
	while read -r e; do
		case "$e" in
		9k/boot|stage/init|cmd/paqfs:paqfs|cmd/7c|cmd/7l|cmd/cc) continue ;;
		stage/stacksmoke) continue ;;
		esac
		e="${e#cmd/}"
		case "$e" in *:*) e="${e##*:}" ;; esac
		echo "${e##*/}"
	done | sort -u | tr '\n' ' '
)
# drop every SKIP-named member from the OK set (the donor-guarded
# members emit T1-SKIP on r9, not T1-OK)
T1_OK=""
for m in $T1_OK_ALL; do
	case " $T1_SKIP" in
	*" $m "*) ;;
	*) T1_OK="$T1_OK $m" ;;
	esac
done
R9_EXPECT_OK="$T1_OK float-d01 libtest-tdraw forkstress writeprobe cexecprobe cexeccreate rcloseprobe brkprobe fpprobe"
R9_EXPECT_SKIP="$T1_SKIP"
export R9_EXPECT_OK R9_EXPECT_SKIP
echo "== expected on r9: $(echo "$R9_EXPECT_OK" | wc -w | tr -d ' ') T1-OK markers + $(echo "$R9_EXPECT_SKIP" | wc -w | tr -d ' ') T1-SKIP markers (computed)"

# --- step 3: DTB patch (the Phase-4 fdtput recipe; see the header) ----

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
# The per-run unique tag — exported so tier1-r9.expect adds it to
# qemu's argv (-name), and this run's survivor sweep (cleanup below)
# keys on it.  The sweep NEVER matches a kernel-name class: a foreign
# r9 qemu (a dev's interactive boot-r9 uses the same kernel; a parallel
# CI job on a shared runner) must never be killed by this gate's
# teardown.
R9_RUN_TAG="r9tier1-$$-$(date +%s)"
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

# --- step 4: the expect-driven tier-1 gate ---------------------------

R9_TIER1_LOG="${R9_TIER1_LOG:-$R9_GATE_OUT/tier1-r9.log}"
R9_QEMU="${R9_QEMU:-qemu-system-aarch64}"

# fresh capture per run BEFORE launch (a
# pre-seeded stale log is dead evidence — it starts empty and only this
# run's boot writes it; the driver rm -f's it again belt-and-braces)
rm -f "$R9_TIER1_LOG"

echo "== step 3/4: DTB patched at $DTB"
echo "== step 4/4: the tier-1 gate on r9 ($R9_TIER1_DRIVER)"
echo "== capture: $R9_TIER1_LOG"

export R9_KERNEL R9_INITRD R9_TIER1_LOG R9_QEMU
R9_DTB="$DTB"
export R9_DTB
# The selected machine shape for the expect driver's spawn argv.
export R9_MACHINE R9_CPU R9_ACCEL

set +e
expect "$R9_TIER1_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "tier-1 r9 gate FAILED (the driver named the stage on stderr; capture: $R9_TIER1_LOG)"
fi
echo "== tier-1 r9 gate PASS — capture retained at $R9_TIER1_LOG"
exit 0
