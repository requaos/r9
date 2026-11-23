#!/usr/bin/env bash
# nix/gates/tier1-r9-x64.sh — the r9-x64 tier-1 gate as an r9 flake app
# (plan 01-23, D-07/D-09/D-16/D-19, HARN-08; moved from Harvey
# bd7a698's nix/tier1-r9-x64.sh).
#
# The SAME composed pipeline as nix/gates/smoke-r9-x64.sh (kernel
# contract → r9's image assembler → r9's iso-builder) but the image is
# the TIER image (xtask/manifests/amd64/tier.toml — the chain tree +
# the h* sweep set + /lib/tier1-sweep.rc + the fixed inputs) and the
# launch runs under gates/x86_64/tier1-r9.expect, which asserts the
# sweep's marker set against the serial wire and exits 0/1 for CI.
#
# Rewired onto r9's own supply (the 01-18/01-21 pattern):
#
#   1. the image — `cargo xtask image --arch x86-64 --image tier` from
#      the r9 checkout replaces Harvey's in-checkout stage-build +
#      subset-build + mkimage pair.  The image engine stages from the
#      read-only amd64 tree and writes target/images/x86_64/tier.paq.
#      Nothing is written into the Harvey tree (HARN-02).
#   2. the expected marker sets are COMPUTED here (step 2) from the
#      committed tier data + the committed sweep script in the
#      read-only tree — never a second hardcoded list (the aarch64
#      gate's rule, carried).
#   3. compose — r9's exported iso-builder (R9_ISO_BUILDER) builds the
#      ISO from the pinned kernel-x64 + the tier bake as
#      /boot/bootfs.paq.
#   4. the gate — gates/x86_64/tier1-r9.expect spawns qemu with the
#      composed ISO on -cdrom (q35, -nographic) and asserts: banner →
#      paqfs → version → the sweep from the boot flow (the tier
#      image's termrc runs it — no typed line) with the float canary
#      T1-OK float-d01 (D-01: user SSE proven on BOTH kernels) →
#      T1-DONE → the % prompt → the whole-capture post-pass (every
#      expected T1-OK, the T1-SKIP set EQUAL to the sweep's own
#      declared list, zero T1-FAIL, zero panic).  Markers are the ONLY
#      proof — the supervisor's exit status is never trusted.
#
# Env contract (the flake app apps.x86_64-linux.boot-r9-tier1-x64
# exports the package paths; a standalone run needs the same values):
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree carrying
#                      /HARVEY-TREE (the flake's harvey-tree-amd64).
#   R9_ROOT            the r9 checkout root (default $PWD); the app is
#                      run from the repo root.
#   R9_KERNEL_X64_PKG  REQUIRED — the kernel-x64 package store path.
#   R9_ISO_BUILDER     REQUIRED — r9's iso-builder executable.
#   R9_MKPAQFS         REQUIRED — the host mkpaqfs (image bake).
#   R9_GATE_OUT        output dir (default target/gates/tier1-r9-x64);
#                      the run scratch dir and the capture live under it.
#   R9_TIER1_LOG       default $R9_GATE_OUT/tier1-r9.log (the serial
#                      capture).
#   R9_TIER1_DRIVER    default $R9_ROOT/gates/x86_64/tier1-r9.expect.
#   R9_QEMU            default qemu-system-x86_64.
#   R9_ACCEL           kvm (default) | tcg — no auto, no silent
#                      downgrade (the Phase-13 discipline).
#   R9_CPU             override the cpu model (default: host under kvm,
#                      qemu64 under tcg).
#   KERNDATE           bake-time timestamp override; the wall clock
#                      otherwise (the tree is a store path with no .git).
#
# Hygiene: the capture is rm -f'd BEFORE launch (a pre-seeded stale log
# cannot pass — it starts empty and only this run's boot writes it);
# the expect driver kills qemu's process group on every exit path;
# THIS script's EXIT trap sweeps any survivor via a bracket-fixed pkill
# keyed on this run's unique R9_RUN_TAG (exported below; the driver
# adds it to qemu's argv as -name) — never a kernel-name class, so a
# foreign r9 qemu (a dev's interactive boot, a parallel CI job) is
# never matched.  The '[q]emu…' pattern cannot self-match the harness
# shell's or pkill's own argv.
#
# Wrapper provides (the flake app's runtimeInputs): the Rust toolchain
# (cargo xtask image), qemu, expect, procps (the survivor sweep),
# gawk/gnugrep (the computed marker sets) and the text tools.
#
# usage: nix run .#boot-r9-tier1-x64   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/tier1-r9-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#boot-r9-tier1-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/tier.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/tier.toml is missing — the tier image manifest is part of this checkout"
R9_TIER1_DRIVER="${R9_TIER1_DRIVER:-$R9_ROOT/gates/x86_64/tier1-r9.expect}"
[ -f "$R9_TIER1_DRIVER" ] || err "R9_TIER1_DRIVER=$R9_TIER1_DRIVER: the tier-1 driver is missing"

# Resolve a host grep once (D-02): the marker-set computation needs
# -o/-E, which plan9 grep lacks — but a hardcoded /usr/bin/grep breaks
# on NixOS-class hosts, so fall back to PATH.
GREP=$(command -v /usr/bin/grep || command -v grep) || err "no host grep found"

# --- step 0: the kernel-package + builder + assembler contract ---------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_MKPAQFS; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#boot-r9-tier1-x64), which exports the r9 supply"
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
		[ -e /dev/kvm ] || err "/dev/kvm absent and R9_ACCEL=$R9_ACCEL — no silent downgrade: set R9_ACCEL=tcg explicitly for the TCG fallback (the Phase-13 discipline)"
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
echo "== r9 kernel package: $R9_KERNEL_X64_PKG"
echo "   kernel:      $R9_KERNEL"
echo "   iso-builder: $R9_ISO_BUILDER"
echo "   shape:       q35 accel=$R9_ACCEL cpu=$R9_CPU"

# --- step 1: the tier image (r9's assembler) ---------------------------

mkdir -p "$R9_GATE_OUT"
echo "== step 1/4: assembling image tier (cargo xtask image --arch x86-64 --image tier)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image tier ) \
	|| err "tier image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
TIER_PAQ="$R9_ROOT/target/images/x86_64/tier.paq"
[ -s "$TIER_PAQ" ] || err "$TIER_PAQ missing/empty after assembly (the tier manifest bakes it)"
echo "== image ok: $TIER_PAQ"

# --- step 2: the expected marker sets (computed, never a second
#     hardcoded list — the aarch64 gate's rule, carried).  The OK set:
#     tier entries map to marker names (basename; dir:target -> target)
#     minus the driver-only members minus every SKIP-named member (the
#     donor-guarded legs emit T1-SKIP on r9) plus the mandatory float
#     canary.  The SKIP set is extracted VERBATIM from the committed
#     tier1-sweep.rc's SKIP lines (the file is the source of truth). ---
# NOTE: `|| true` — the app wrapper runs `set -o pipefail`, and grep
# exits 1 on ZERO matches: the 15-03 SKIP-flip emptied the sweep's SKIP
# block (ipconfig + factotum run live), and the bare pipeline killed
# the driver silently between the bake and the expectations print.
T1_SKIP=$({ "$GREP" -oE '(^|[[:space:]])echo T1-SKIP [A-Za-z0-9.-]+' "$HARVEY/build/x86_64/tier1-sweep.rc" || true; } \
	| awk '{print $NF}' | sort -u | tr '\n' ' ')
T1_OK_ALL=$(
	awk 'NF && $1 !~ /^#/ { print $1 }' "$HARVEY/build/x86_64/tiers/tier1-cmds.txt" |
	while read -r e; do
		case "$e" in
		9k/boot|stage/*|cmd/paqfs:paqfs|cmd/cc) continue ;;
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
R9_EXPECT_OK="$T1_OK float-d01 fpprobe"
R9_EXPECT_SKIP="$T1_SKIP"
export R9_EXPECT_OK R9_EXPECT_SKIP
echo "== step 2/4: expected on r9: $(echo "$R9_EXPECT_OK" | wc -w | tr -d ' ') T1-OK markers + $(echo "$R9_EXPECT_SKIP" | wc -w | tr -d ' ') T1-SKIP markers (computed)"

# --- step 3: compose the ISO -------------------------------------------

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
# The per-run unique tag — exported so tier1-r9.expect adds it to
# qemu's argv (-name), and this run's survivor sweep (cleanup below)
# keys on it.  The sweep NEVER matches a kernel-name class.
R9_RUN_TAG="r9x64tier1-$$-$(date +%s)"
export R9_RUN_TAG
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	# the expect driver kills qemu's process group on every exit path;
	# this trap sweeps any survivor.  the pattern is bracket-fixed
	# ([q]emu) so pkill cannot self-match this shell's or its own argv,
	# and keyed on THIS run's unique -name tag — never on the
	# kernel-path class.
	pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT

echo "== step 3/4: composing the ISO (r9's iso-builder: kernel + tier.paq as /boot/bootfs.paq)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" "$TIER_PAQ" bootfs.paq \
	|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 4: the expect-driven tier-1 gate ------------------------------

R9_TIER1_LOG="${R9_TIER1_LOG:-$R9_GATE_OUT/tier1-r9.log}"
R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"

# fresh capture per run BEFORE launch (a pre-seeded stale log is dead
# evidence — it starts empty and only this run's boot writes it; the
# driver rm -f's it again belt-and-braces)
rm -f "$R9_TIER1_LOG"

echo "== step 4/4: the tier-1 gate on r9 ($R9_TIER1_DRIVER)"
echo "== capture: $R9_TIER1_LOG"

export R9_ISO R9_TIER1_LOG R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL

set +e
expect "$R9_TIER1_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "tier-1 r9-x64 gate FAILED (the driver named the stage on stderr; capture: $R9_TIER1_LOG)"
fi
echo "== tier-1 r9-x64 gate PASS — capture retained at $R9_TIER1_LOG"
exit 0
