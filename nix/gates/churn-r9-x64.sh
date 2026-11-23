#!/usr/bin/env bash
# nix/gates/churn-r9-x64.sh — the composed churn gate as an r9 flake app
# (plan 01-23, D-07/D-09/D-16/D-19, HARN-08; moved from Harvey
# bd7a698's nix/churn-r9-x64.sh).
#
# The same composed pipeline as nix/gates/smoke-r9-x64.sh re-keyed
# onto the CHURN venue — NET-FREE by design (no doctor preflight, no
# netdev emitter, no host peer: the churn load touches no net device).
# The pipeline, each step loud on failure:
#
#   0. the contract: R9_KERNEL_X64_PKG + R9_ISO_BUILDER (the 14-07
#      contract) + r9's assembler supply + accel selection (no silent
#      downgrade, the Phase-13 discipline).  The kernel package is the
#      q35churnshell build: the composed venue's kernel carries the
#      per-exit churngate heap marker through the same dev_churn cfg
#      gate as the r9-side tier (the bounded-memory mechanism this
#      gate's window post-pass consumes).
#   1. the image — `cargo xtask image --arch x86-64 --image churn` from
#      the r9 checkout replaces Harvey's in-checkout stage-build +
#      subset-build + mkimage pair.  The image engine stages from the
#      read-only amd64 tree and writes target/images/x86_64/churn.paq.
#      Nothing is written into the Harvey tree (HARN-02).
#   2. compose — r9's exported iso-builder (R9_ISO_BUILDER: ONE
#      reproducibility implementation) builds the ISO from the pinned
#      kernel-x64 + the churn bake as /boot/bootfs.paq.
#   3. the gate — gates/x86_64/churn-r9.expect spawns qemu with the
#      composed ISO on -cdrom (q35, net-free), drives `rc
#      /lib/churn-cats.rc' (the /bin/cat x 1400 fork+exec churn at the
#      #44 repro shape), asserts every fixture marker, the exact-count
#      contract (one CATS-DONE 1400), zero churn: FAIL, and the
#      bounded-memory window (the per-exit heap markers between BEFORE
#      and AFTER, the delta over the window's exit count against the
#      committed bound), and fails wedge-stalls LOUD with the capture
#      tail.
#
# Env contract (the flake app apps.x86_64-linux.boot-r9-churn-x64
# exports the package paths; a standalone run needs the same values):
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree carrying
#                      /HARVEY-TREE (the flake's harvey-tree-amd64).
#   R9_ROOT            the r9 checkout root (default $PWD); the app is
#                      run from the repo root.
#   R9_KERNEL_X64_PKG  REQUIRED — the kernel-x64 package store path.
#   R9_ISO_BUILDER     REQUIRED — r9's iso-builder executable.
#   R9_MKPAQFS         REQUIRED — the host mkpaqfs (image bake).
#   R9_GATE_OUT        output dir (default target/gates/churn-r9-x64);
#                      the run scratch dir and the capture live under it.
#   R9_CHURN_LOG       default $R9_GATE_OUT/churn-r9.log (the serial
#                      capture).
#   R9_CHURN_DRIVER    default $R9_ROOT/gates/x86_64/churn-r9.expect.
#   R9_QEMU            default qemu-system-x86_64.
#   R9_ACCEL           kvm (default) | tcg — no auto, no silent
#                      downgrade.
#   R9_CPU             override the cpu model (default: host under kvm,
#                      qemu64 under tcg).
#   KERNDATE           bake-time timestamp override; the wall clock
#                      otherwise (the tree is a store path with no .git).
#
# Hygiene: the capture is rm -f'd BEFORE launch; the expect driver
# kills qemu's process group on every exit path; THIS script's EXIT
# trap sweeps any qemu survivor via a bracket-fixed pkill keyed on
# this run's unique R9_RUN_TAG (exported below; the driver adds it to
# qemu's argv as -name).  The tag's r9x64churn- prefix differs from
# every sibling gate's (r9x64smoke-, r9x64tier1-, r9x64svc-) so the
# sweeps can never cross-kill.
#
# Wrapper provides (the flake app's runtimeInputs): the Rust toolchain
# (cargo xtask image), qemu, expect, procps (the survivor sweep) and
# the text tools.
#
# usage: nix run .#boot-r9-churn-x64   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/churn-r9-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#boot-r9-churn-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/churn.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/churn.toml is missing — the churn image manifest is part of this checkout"
R9_CHURN_DRIVER="${R9_CHURN_DRIVER:-$R9_ROOT/gates/x86_64/churn-r9.expect}"
[ -f "$R9_CHURN_DRIVER" ] || err "R9_CHURN_DRIVER=$R9_CHURN_DRIVER: the churn driver is missing"

# --- step 0: the kernel-package + builder + assembler contract ---------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_MKPAQFS; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#boot-r9-churn-x64), which exports the r9 supply"
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
echo "   shape:       q35 accel=$R9_ACCEL cpu=$R9_CPU net-free (the churn venue)"

# --- run scratch + survivor sweep trap -----------------------------------

mkdir -p "$R9_GATE_OUT"
TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
R9_RUN_TAG="r9x64churn-$$-$(date +%s)"
export R9_RUN_TAG
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	# the expect driver kills qemu's process group on every exit path;
	# this trap sweeps any survivor keyed on THIS run's unique tag
	pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT

# --- step 1: the churn image (r9's assembler) ---------------------------

echo "== step 1/3: assembling image churn (cargo xtask image --arch x86-64 --image churn)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image churn ) \
	|| err "churn image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
CHURN_PAQ="$R9_ROOT/target/images/x86_64/churn.paq"
[ -s "$CHURN_PAQ" ] || err "$CHURN_PAQ missing/empty after assembly (the churn manifest bakes it)"
echo "== image ok: $CHURN_PAQ"

# --- step 2: compose the ISO ---------------------------------------------

echo "== step 2/3: composing the ISO (r9's iso-builder: kernel + churn.paq as /boot/bootfs.paq)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" "$CHURN_PAQ" bootfs.paq \
	|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 3: the expect-driven churn gate ---------------------------------

R9_CHURN_LOG="${R9_CHURN_LOG:-$R9_GATE_OUT/churn-r9.log}"
R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"

# fresh capture per run BEFORE launch (a pre-seeded stale log is dead
# evidence — it starts empty and only this run's boot writes it; the
# driver rm -f's it again belt-and-braces)
rm -f "$R9_CHURN_LOG"

echo "== step 3/3: the churn gate ($R9_CHURN_DRIVER — /bin/cat x 1400 via rc /lib/churn-cats.rc, the bounded-memory window)"
echo "== capture: $R9_CHURN_LOG"

export R9_ISO R9_CHURN_LOG R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL

set +e
expect "$R9_CHURN_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "churn gate FAILED (the driver named the stage + printed the capture tail; capture: $R9_CHURN_LOG)"
fi
echo "== churn gate PASS — capture retained at $R9_CHURN_LOG"
exit 0
