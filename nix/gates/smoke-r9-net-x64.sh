#!/usr/bin/env bash
# nix/gates/smoke-r9-net-x64.sh — the composed NET smoke gate as an r9
# flake app (plan 01-24, D-07/D-09/D-16/D-19, HARN-08; moved from
# Harvey bd7a698's nix/smoke-r9-net-x64.sh and rewired onto r9's own
# supply, the 01-18/01-23 pattern).
#
# The SAME composed pipeline as the x86_64 tier-1 gate (kernel contract
# → r9's image assembler → r9's iso-builder) but the assembled image is
# the NET payload (r9's xtask/manifests/amd64/net.toml: the chain tree
# + the ip-container binaries + the 6c C helpers + the /lib fixtures)
# and the launch runs under gates/x86_64/smoke-r9-net.expect, which
# drives the user-visible net flows through the packaged kernel's devip
# #I tree in ONE boot — ipconfig static + dhcp (both NOT-L2-TRUTH
# labeled), the duplicate-binding rejection, loopback ping + udp echo,
# the no-address rejection, the tcp/7 echo leg — and exits 0/1 for CI.
# smoke-r9-x64's own behavior is untouched.
#
# VENUE + NETDEV (16-06, HNET-01/D-02/D-05): the gate's venue stays
# FIXED slirp — hermetic, unprivileged, CI-able (the tap/soak legs are
# the soak app's, primary-host-only by construction).  The qemu
# netdev+device argv flows from the SAME pinned TOML data the boot app
# consumes — the emitter's output captured once below and exported as
# R9_NETDEV_ARGV for the expect driver to split, ONE emission path
# everywhere — with the D-04 hostfwd pair included; the driver's
# hostfwd-* stages observe both loopback listeners on the HOST while
# the guest runs (qemu binds them eagerly at netdev init).
#
# Rewired onto r9's own supply:
#
#   1. the image — `cargo xtask image --arch x86-64 --image net` from
#      the r9 checkout replaces Harvey's in-checkout stage-build +
#      subset-build + mkimage trio.  The image engine stages from the
#      read-only amd64 tree and writes target/images/x86_64/net.paq.
#      Nothing is written into the Harvey tree (HARN-02); R9_SKIP_BUILD
#      is deleted (the assembler's own staleness check is the only
#      cache).
#   2. compose — r9's exported iso-builder (R9_ISO_BUILDER: ONE
#      reproducibility implementation) builds the ISO from the pinned
#      kernel-x64 + the net bake as /boot/bootfs.paq.
#   3. the gate — gates/x86_64/smoke-r9-net.expect spawns qemu with the
#      composed ISO on -cdrom (q35, -nographic, the emitter's slirp
#      argv) and asserts: chain boot (banner → paqfs → version → the %
#      prompt) → the seam source → the static leg (ok/arp/venue-label/
#      NETIP-DONE) → the duplicate-binding rejection (the devip error
#      + the marker) → the ether unbind → the dhcp leg → the second
#      unbind → the loopback leg (random/ping/udp-echo/NETIP-DONE) →
#      the no-address rejection → the inline tcp/7 echo leg → the
#      NET-SMOKE completion marker → the whole-capture post-passes
#      (pid-warning-absent + nic-bind).  Markers are the ONLY proof —
#      the supervisor's exit status is never trusted.
#
# Env contract (the flake app apps.x86_64-linux.boot-r9-net-smoke-x64
# exports the package paths; a standalone run needs the same values):
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree carrying
#                       /HARVEY-TREE (the flake's harvey-tree-amd64);
#                       fixtures are read from it, nothing is written to
#                       it.
#   R9_ROOT             the r9 checkout root (default $PWD); the app is
#                       run from the repo root.
#   R9_KERNEL_X64_PKG   REQUIRED — the kernel-x64 package store path.
#   R9_ISO_BUILDER      REQUIRED — r9's iso-builder executable.
#   R9_NETDEV_EMITTER   REQUIRED — r9's netdev-args emitter executable
#                       (the slirp argv with the D-04 hostfwd pair
#                       comes ONLY from its output, captured once and
#                       exported as R9_NETDEV_ARGV).
#   R9_MKPAQFS          REQUIRED — the host mkpaqfs (image bake).
#   R9_GATE_OUT         output dir (default
#                       target/gates/smoke-r9-net-x64); the run scratch
#                       dir and the capture live under it.
#   R9_NET_LOG          default $R9_GATE_OUT/smoke-r9-net.log.
#   R9_SMOKE_DRIVER     default $R9_ROOT/gates/x86_64/smoke-r9-net.expect.
#   R9_QEMU             default qemu-system-x86_64.
#   R9_ACCEL            kvm (default) | tcg — no auto, no silent
#                       downgrade (the Phase-13 discipline).
#   R9_CPU              override the cpu model (default: host under kvm,
#                       qemu64 under tcg).
#   KERNDATE            bake-time timestamp override; the wall clock
#                       otherwise (the tree is a store path with no
#                       .git).
#
# Hygiene: the capture is rm -f'd BEFORE launch (a pre-seeded stale log
# cannot pass — it starts empty and only this run's boot writes it);
# the expect driver kills qemu's process group on every exit path;
# THIS script's EXIT trap sweeps any survivor via a bracket-fixed pkill
# keyed on this run's unique R9_RUN_TAG (exported below; the driver
# adds it to qemu's argv as -name) — never a kernel-name class, so a
# foreign r9 qemu (a dev's interactive boot-r9-x64, a parallel CI job)
# is never matched.  The '[q]emu…' pattern cannot self-match the
# harness shell's or pkill's own argv.  The tag's r9x64net- prefix
# differs from the sibling x64 gates' tags so the gates' survivor
# sweeps can never cross-kill.  Every output lives under R9_GATE_OUT;
# nothing is written under $HARVEY (T-01-78).
#
# Wrapper provides (the flake app's runtimeInputs): the Rust toolchain
# (cargo xtask image), qemu, expect, procps (the survivor sweep),
# iproute2 (the driver's hostfwd observation) and the text tools.
#
# usage: nix run .#boot-r9-net-smoke-x64   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/smoke-r9-net-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#boot-r9-net-smoke-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/net.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/net.toml is missing — the net image manifest is part of this checkout"
R9_SMOKE_DRIVER="${R9_SMOKE_DRIVER:-$R9_ROOT/gates/x86_64/smoke-r9-net.expect}"
[ -f "$R9_SMOKE_DRIVER" ] || err "R9_SMOKE_DRIVER=$R9_SMOKE_DRIVER: the net smoke driver is missing"

# --- step 0: the kernel-package + builder + assembler contract ---------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_NETDEV_EMITTER R9_MKPAQFS; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#boot-r9-net-smoke-x64), which exports the r9 supply"
done
[ -d "$R9_KERNEL_X64_PKG" ] || err "R9_KERNEL_X64_PKG $R9_KERNEL_X64_PKG: not a directory (unrealised store path?)"

R9_KERNEL="$R9_KERNEL_X64_PKG/kernel-x64.elf"
if [ ! -f "$R9_KERNEL" ]; then
	R9_KERNEL=$(find "$R9_KERNEL_X64_PKG" -name 'kernel-x64.elf' -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_KERNEL" ] && [ -f "$R9_KERNEL" ] || err "kernel-x64.elf not found under $R9_KERNEL_X64_PKG"

[ -f "$R9_ISO_BUILDER" ] && [ -x "$R9_ISO_BUILDER" ] \
	|| err "R9_ISO_BUILDER $R9_ISO_BUILDER: not an executable file (want <pkg>/bin/r9-iso-builder)"
[ -f "$R9_NETDEV_EMITTER" ] && [ -x "$R9_NETDEV_EMITTER" ] \
	|| err "R9_NETDEV_EMITTER $R9_NETDEV_EMITTER: not an executable file (want <pkg>/bin/r9-netdev-args)"
[ -f "$R9_MKPAQFS" ] && [ -x "$R9_MKPAQFS" ] \
	|| err "R9_MKPAQFS $R9_MKPAQFS: not an executable file (want <pkg>/bin/mkpaqfs)"

# --- step 0c: the netdev argv — the emitter's slirp tokens (16-06) -----
#
# The venue stays FIXED slirp (D-02 — the header's VENUE + NETDEV
# note); the argv flows from the same pinned TOML data the boot app
# consumes, the D-04 hostfwd pair included.  Resolved ONCE here, before
# any assembly spend, so an emitter failure is a named contract error —
# and exported as R9_NETDEV_ARGV for the expect driver to split.

NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode slirp) \
	|| err "the netdev emitter failed for mode slirp (its diagnostics are above; the argv comes only from the pinned r9 TOML — D-05)"
[ -n "$NETDEV_ARGV" ] || err "the netdev emitter printed no netdev argv for mode slirp"
export R9_NETDEV_ARGV="$NETDEV_ARGV"

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
echo "   kernel:        $R9_KERNEL"
echo "   iso-builder:   $R9_ISO_BUILDER"
echo "   netdev emitter: $R9_NETDEV_EMITTER"
echo "   netdev argv (slirp): $NETDEV_ARGV"
echo "   shape:         q35 accel=$R9_ACCEL cpu=$R9_CPU"

# --- step 1: the net image (r9's assembler replaces Harvey's bake) -----

mkdir -p "$R9_GATE_OUT"
echo "== step 1/3: assembling image net (cargo xtask image --arch x86-64 --image net)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image net ) \
	|| err "net image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
NET_PAQ="$R9_ROOT/target/images/x86_64/net.paq"
[ -s "$NET_PAQ" ] || err "$NET_PAQ missing/empty after assembly (the net manifest bakes it)"
echo "== image ok: $NET_PAQ"

# --- step 2: compose the ISO -------------------------------------------

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
# The per-run unique tag — exported so smoke-r9-net.expect adds it to
# qemu's argv (-name), and this run's survivor sweep (cleanup below)
# keys on it.  The sweep NEVER matches a kernel-name class: a foreign
# r9 qemu (a dev's interactive boot-r9-x64 uses the same kernel; a
# parallel CI job on a shared runner; the SIBLING gates' qemu) must
# never be killed by this gate's teardown.  The r9x64net- prefix
# differs from the sibling x64 gates' tags for exactly that reason.
R9_RUN_TAG="r9x64net-$$-$(date +%s)"
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

echo "== step 2/3: composing the ISO (r9's iso-builder: kernel + net.paq as /boot/bootfs.paq)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" "$NET_PAQ" bootfs.paq \
	|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 3: the expect-driven composed net gate -----------------------

R9_NET_LOG="${R9_NET_LOG:-$R9_GATE_OUT/smoke-r9-net.log}"
R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"

# fresh capture per run BEFORE launch (a pre-seeded stale log is dead
# evidence — it starts empty and only this run's boot writes it; the
# driver rm -f's it again belt-and-braces)
rm -f "$R9_NET_LOG"

echo "== step 3/3: composed net gate ($R9_SMOKE_DRIVER)"
echo "== capture: $R9_NET_LOG"

export R9_ISO R9_SMOKE_LOG="$R9_NET_LOG" R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL

set +e
expect "$R9_SMOKE_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "composed net gate FAILED (the driver named the stage on stderr; capture: $R9_NET_LOG)"
fi
echo "== composed net gate PASS — capture retained at $R9_NET_LOG"
exit 0
