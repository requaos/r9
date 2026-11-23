#!/usr/bin/env bash
# nix/gates/services-r9-x64.sh — the r9-x64 SERVICES gate as an r9 flake
# app (plan 01-23, D-07/D-09/D-16/D-19, HARN-08; moved from Harvey
# bd7a698's nix/services-r9-x64.sh).
#
# The SAME composed pipeline as nix/gates/tier1-r9-x64.sh (kernel
# contract → r9's image assembler → r9's iso-builder) but the image is
# the SERVICES image (xtask/manifests/amd64/services.toml: the chain
# tree + the tier-1 h* helpers + the tier-2 services set computed from
# tiers/tier2-services.txt + the net members + the sweep's /boot-side
# members + services-sweep.rc at /lib with the services-termrc
# override), and the launch runs under gates/x86_64/services-r9.expect,
# which asserts the sweep's SVC marker contract against the serial
# wire and exits 0/1 for CI.
#
# Rewired onto r9's own supply (the 01-18/01-21 pattern):
#
#   1. the image — `cargo xtask image --arch x86-64 --image services`
#      from the r9 checkout replaces Harvey's in-checkout stage-build +
#      subset-build + mkimage pair.  The image engine stages from the
#      read-only amd64 tree (including the 4303-file font tree) and
#      writes target/images/x86_64/services.paq.  Nothing is written
#      into the Harvey tree (HARN-02).
#   2. the expected marker sets are COMPUTED here (step 2) from the
#      committed tier data + the committed sweep script in the
#      read-only tree — never a second hardcoded list (the donor
#      gate's compute_svc_expected discipline).
#   3. compose — r9's exported iso-builder (R9_ISO_BUILDER) builds the
#      ISO from the pinned kernel-x64 + the services bake as
#      /boot/bootfs.paq.
#   4. the gate — gates/x86_64/services-r9.expect spawns qemu with the
#      composed ISO on -cdrom (q35, -nographic) and asserts: banner →
#      paqfs → version → the services sweep from the boot flow (the
#      services image's termrc runs it — no typed line) with the net
#      bring-up preceding the daemons → every subject's verdict →
#      SVC-DONE → the % prompt → the whole-capture post-pass (every
#      computed subject earned SVC-OK or its declared conditional
#      SVC-SKIP, the converse on every emitted SVC-SKIP, zero
#      SVC-FAIL, zero panic).  Markers are the ONLY proof — the
#      supervisor's exit status is never trusted.
#
# Env contract (the flake app apps.x86_64-linux.boot-r9-services-x64
# exports the package paths; a standalone run needs the same values):
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree carrying
#                      /HARVEY-TREE (the flake's harvey-tree-amd64).
#   R9_ROOT            the r9 checkout root (default $PWD); the app is
#                      run from the repo root.
#   R9_KERNEL_X64_PKG  REQUIRED — the kernel-x64 package store path.
#   R9_ISO_BUILDER     REQUIRED — r9's iso-builder executable.
#   R9_MKPAQFS         REQUIRED — the host mkpaqfs (image bake).
#   R9_GATE_OUT        output dir (default target/gates/services-r9-x64);
#                      the run scratch dir and the capture live under it.
#   R9_SVC_LOG         default $R9_GATE_OUT/services-r9.log (the serial
#                      capture).
#   R9_SVC_DRIVER      default $R9_ROOT/gates/x86_64/services-r9.expect.
#   R9_QEMU            default qemu-system-x86_64.
#   R9_ACCEL           kvm (default) | tcg — no auto, no silent
#                      downgrade.
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
# usage: nix run .#boot-r9-services-x64   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/services-r9-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#boot-r9-services-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/services.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/services.toml is missing — the services image manifest is part of this checkout"
R9_SVC_DRIVER="${R9_SVC_DRIVER:-$R9_ROOT/gates/x86_64/services-r9.expect}"
[ -f "$R9_SVC_DRIVER" ] || err "R9_SVC_DRIVER=$R9_SVC_DRIVER: the services driver is missing"

# Resolve a host grep once (D-02): the marker-set computation needs
# -o/-E, which plan9 grep lacks — but a hardcoded /usr/bin/grep breaks
# on NixOS-class hosts, so fall back to PATH.
GREP=$(command -v /usr/bin/grep || command -v grep) || err "no host grep found"

# --- step 0: the kernel-package + builder + assembler contract ---------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_MKPAQFS; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#boot-r9-services-x64), which exports the r9 supply"
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

# --- step 1: the services image (r9's assembler) -----------------------

mkdir -p "$R9_GATE_OUT"
echo "== step 1/4: assembling image services (cargo xtask image --arch x86-64 --image services)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image services ) \
	|| err "services image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
SVC_PAQ="$R9_ROOT/target/images/x86_64/services.paq"
[ -s "$SVC_PAQ" ] || err "$SVC_PAQ missing/empty after assembly (the services manifest bakes it)"
echo "== image ok: $SVC_PAQ"

# --- step 2: the expected marker sets (computed, never a second
#     hardcoded list — the donor gate's compute_svc_expected
#     discipline).  The OK set: the sweep's nine probe subjects, each
#     verified against the tier entry it rides (cmd/ndb is the
#     resolver pair's home — cs + dns are its probe subjects; factotum
#     rides its TIER-1 line, never a tier-2 member), minus every
#     SKIP-named member.  The declared SKIP set is extracted VERBATIM
#     from the committed services-sweep.rc's own `echo SVC-SKIP' lines
#     (the file is the source of truth — the sweep's skips are
#     CONDITIONAL, so this is the DECLARED set, not the expected
#     emitted set; the post-pass checks OK-or-emitted-declared-SKIP per
#     subject plus the converse on every emitted name). ---
SVC_T2="$HARVEY/build/x86_64/tiers/tier2-services.txt"
SVC_T1D="$HARVEY/build/x86_64/tiers/tier1-cmds.txt"
for e in cmd/mntgen cmd/ndb cmd/exportfs cmd/srv cmd/aux:listen cmd/auth/authsrv cmd/auth/keyfs; do
	"$GREP" -q "^${e}[[:space:]#]" "$SVC_T2" || err "svc expectation: tier-2 data lacks the probe-subject entry $e"
done
"$GREP" -q "^cmd/auth/factotum:factotum[[:space:]#]" "$SVC_T1D" || err "svc expectation: tier-1 data lacks cmd/auth/factotum:factotum (factotum rides its tier-1 line)"
# NOTE: `|| true` — the app wrapper runs `set -o pipefail` and grep
# exits 1 on ZERO matches (a sweep with no declared conditional SKIPs
# must compute an EMPTY set, not kill the driver).
SVC_SKIP=$({ "$GREP" -oE '(^|[[:space:]])echo SVC-SKIP [A-Za-z0-9.-]+' "$HARVEY/build/x86_64/services-sweep.rc" || true; } \
	| awk '{print $NF}' | sort -u | tr '\n' ' ')
SVC_OK_ALL="mntgen cs dns exportfs srv listen factotum keyfs authsrv"
SVC_OK=""
for m in $SVC_OK_ALL; do
	case " $SVC_SKIP" in
	*" $m "*) ;;
	*) SVC_OK="$SVC_OK $m" ;;
	esac
done
R9_EXPECT_SVC_OK="$SVC_OK"
R9_EXPECT_SVC_SKIP="$SVC_SKIP"
export R9_EXPECT_SVC_OK R9_EXPECT_SVC_SKIP
echo "== step 2/4: expected on r9: $(echo "$R9_EXPECT_SVC_OK" | wc -w | tr -d ' ') SVC-OK subjects + $(echo "$R9_EXPECT_SVC_SKIP" | wc -w | tr -d ' ') declared conditional SVC-SKIP names (computed)"

# --- step 3: compose the ISO -------------------------------------------

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
# The per-run unique tag — exported so services-r9.expect adds it to
# qemu's argv (-name), and this run's survivor sweep (cleanup below)
# keys on it.  The sweep NEVER matches a kernel-name class.
R9_RUN_TAG="r9x64svc-$$-$(date +%s)"
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

echo "== step 3/4: composing the ISO (r9's iso-builder: kernel + services.paq as /boot/bootfs.paq)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" "$SVC_PAQ" bootfs.paq \
	|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 4: the expect-driven services gate ----------------------------

R9_SVC_LOG="${R9_SVC_LOG:-$R9_GATE_OUT/services-r9.log}"
R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"

# fresh capture per run BEFORE launch (a pre-seeded stale log is dead
# evidence — it starts empty and only this run's boot writes it; the
# driver rm -f's it again belt-and-braces)
rm -f "$R9_SVC_LOG"

echo "== step 4/4: the services gate on r9 ($R9_SVC_DRIVER)"
echo "== capture: $R9_SVC_LOG"

export R9_ISO R9_SVC_LOG R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL

set +e
expect "$R9_SVC_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "services r9-x64 gate FAILED (the driver named the stage on stderr; capture: $R9_SVC_LOG)"
fi
echo "== services r9-x64 gate PASS — capture retained at $R9_SVC_LOG"
exit 0
