#!/usr/bin/env bash
# nix/gates/smoke-r9-x64.sh — the COMPOSED smoke gate as an r9 flake app
# (plan 01-23, D-07/D-09/D-16/D-19, HARN-08; moved from Harvey bd7a698's
# nix/smoke-r9-x64.sh).
#
# The SAME composed pipeline as the x86_64 boot gate (kernel contract →
# r9's image assembler → r9's iso-builder) but launched under
# gates/x86_64/smoke-r9.expect, which drives the full command sweep
# against the serial wire AND the full-system cpu chain in ONE composed
# gate, and exits 0/1 for CI.
#
# Rewired onto r9's own supply (the 01-18/01-21 pattern):
#
#   1. the images — `cargo xtask image --arch x86-64 --image chain` and
#      `--image cpu` from the r9 checkout replace Harvey's in-checkout
#      stage-build + subset-build + mkimage pair.  The image engine
#      stages from the read-only amd64 tree, runs the per-bake p9sk1
#      authdomain for the cpu image, and writes
#      target/images/x86_64/{chain,cpu}.paq plus cpu.creds.  Nothing is
#      written into the Harvey tree (HARN-02).
#   2. compose the CHAIN ISO — r9's exported iso-builder (R9_ISO_BUILDER)
#      builds the ISO from the pinned kernel-x64 + the chain bake as
#      /boot/bootfs.paq.
#   3. compose the CPU ISO — the same iso-builder WITH the bootconf
#      append (19-01's seam: --append=init=/amd64/init -c) over cpu.paq
#      as /boot/bootfs.paq.
#   4. smoke — gates/x86_64/smoke-r9.expect spawns qemu TWICE (one gate,
#      two legs): the chain leg (the composed chain ISO on -cdrom, q35,
#      -nographic) asserts banner → paqfs/version → the %  prompt →
#      echo round-trip → empty-line safety → ls /bin subset → ps →
#      cat /lib/namespace → DEL safety → exit → init restart → the
#      whole-capture post-passes; then the CPU-CHAIN leg (the cpu ISO
#      under the emitter's slirp argv, the D-04 hostfwd pair included)
#      asserts the §15 boot with the bootconf append, the host-side
#      hostfwd observation (17010 + 5670), the cpurc ladder in order
#      through LISTEN-17010 and DONE, zero FAIL, and the
#      desktop-absent-at-boot proof.  Markers are the ONLY proof — the
#      supervisor's exit status is never trusted.
#
# Env contract (the flake app apps.x86_64-linux.boot-r9-smoke-x64
# exports the package paths; a standalone run needs the same values):
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree carrying
#                       /HARVEY-TREE (the flake's harvey-tree-amd64);
#                       fixtures are read from it, nothing is written
#                       to it.
#   R9_ROOT             the r9 checkout root (default $PWD); the app is
#                       run from the repo root.
#   R9_KERNEL_X64_PKG   REQUIRED — the kernel-x64 package store path.
#   R9_ISO_BUILDER      REQUIRED — r9's iso-builder executable.
#   R9_NETDEV_EMITTER   REQUIRED — r9's netdev-args emitter executable
#                       (the cpu leg's slirp argv with the D-04 hostfwd
#                       pair comes ONLY from its output, captured once
#                       and exported as R9_NETDEV_ARGV).
#   R9_MKPAQFS          REQUIRED — the host mkpaqfs (image bake).
#   R9_AUTHDOMAIN_GEN   REQUIRED — the host p9sk1 domain generator (the
#                       cpu image's per-bake authdomain).
#   R9_GATE_OUT         output dir (default target/gates/smoke-r9-x64);
#                       the run scratch dirs and BOTH captures live
#                       under it.
#   R9_SMOKE_LOG        default $R9_GATE_OUT/smoke-r9.log (the chain
#                       leg's capture).
#   R9_CPU_LOG          default $R9_GATE_OUT/smoke-r9-cpu.log (the
#                       cpu-chain leg's capture).
#   R9_SMOKE_DRIVER     default $R9_ROOT/gates/x86_64/smoke-r9.expect.
#   R9_QEMU             default qemu-system-x86_64.
#   R9_ACCEL            kvm (default) | tcg — no auto, no silent
#                       downgrade (the Phase-13 discipline).
#   R9_CPU              override the cpu model (default: host under kvm,
#                       qemu64 under tcg).
#   KERNDATE            bake-time timestamp override; the wall clock
#                       otherwise (the tree is a store path with no .git).
#
# Hygiene: BOTH captures are rm -f'd BEFORE launch (a pre-seeded stale
# log cannot pass — each starts empty and only this run's boot writes
# it); the expect driver kills the live spawn's process group on every
# exit path; THIS script's EXIT trap sweeps any survivor via a
# bracket-fixed pkill keyed on this run's unique R9_RUN_TAG (exported
# below; the driver adds it to BOTH spawns' argv as -name) — never a
# kernel-name class, so a foreign r9 qemu (a dev's interactive boot, a
# parallel CI job) is never matched.  The '[q]emu…' pattern cannot
# self-match the harness shell's or pkill's own argv.
#
# Wrapper provides (the flake app's runtimeInputs): the Rust toolchain
# (cargo xtask image), qemu, expect, procps (the survivor sweep),
# iproute2 (the cpu leg's hostfwd observation) and the text tools.
#
# usage: nix run .#boot-r9-smoke-x64   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/smoke-r9-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#boot-r9-smoke-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
for m in chain cpu; do
	[ -f "$R9_ROOT/xtask/manifests/amd64/$m.toml" ] \
		|| err "$R9_ROOT/xtask/manifests/amd64/$m.toml is missing — the $m image manifest is part of this checkout"
done
R9_SMOKE_DRIVER="${R9_SMOKE_DRIVER:-$R9_ROOT/gates/x86_64/smoke-r9.expect}"
[ -f "$R9_SMOKE_DRIVER" ] || err "R9_SMOKE_DRIVER=$R9_SMOKE_DRIVER: the smoke driver is missing"

# --- step 0: the kernel-package + builder + assembler contract ---------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_NETDEV_EMITTER R9_MKPAQFS R9_AUTHDOMAIN_GEN; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#boot-r9-smoke-x64), which exports the r9 supply"
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
[ -f "$R9_AUTHDOMAIN_GEN" ] && [ -x "$R9_AUTHDOMAIN_GEN" ] \
	|| err "R9_AUTHDOMAIN_GEN $R9_AUTHDOMAIN_GEN: not an executable file (want <pkg>/bin/authdomain-gen)"

# --- step 0c: the netdev argv — the emitter's slirp tokens ------------
# The cpu-chain leg's qemu rides the emitter's slirp argv so the D-04
# hostfwd pair (host 17010 -> guest 17010, host 5670 -> guest 567) is
# pinned r9 TOML data, never script text (16-06's mechanism, carried):
# captured ONCE here, before any build spend, so an emitter failure is
# a named contract error — and exported as R9_NETDEV_ARGV for the
# expect driver to split.  The CHAIN leg's own spawn keeps its
# hostfwd-free netdev shape (untouched — its nic-bind stage wants the
# NIC, not the pair).

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

# --- step 1: the chain and cpu images (r9's assembler) ----------------

mkdir -p "$R9_GATE_OUT"
echo "== step 1/4: assembling images chain and cpu (cargo xtask image --arch x86-64 --image chain; --image cpu)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image chain ) \
	|| err "chain image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image cpu ) \
	|| err "cpu image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
CHAIN_PAQ="$R9_ROOT/target/images/x86_64/chain.paq"
[ -s "$CHAIN_PAQ" ] || err "$CHAIN_PAQ missing/empty after assembly (the chain manifest bakes it)"
CPU_PAQ="$R9_ROOT/target/images/x86_64/cpu.paq"
[ -s "$CPU_PAQ" ] || err "$CPU_PAQ missing/empty after assembly (the cpu manifest bakes it)"
echo "== images ok: $CHAIN_PAQ + $CPU_PAQ"

# --- step 2+3: compose the TWO ISOs (chain, then cpu with the append) --

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
TMPD2=$(mktemp -d "$R9_GATE_OUT/run-cpu.XXXXXX")
# The per-run unique tag — exported so smoke-r9.expect adds it to BOTH
# spawns' argv (-name), and this run's survivor sweep (cleanup below)
# keys on it.  The sweep NEVER matches a kernel-name class: a foreign
# r9 qemu (a dev's interactive boot uses the same kernel; a parallel CI
# job on a shared runner) must never be killed by this gate's teardown.
R9_RUN_TAG="r9x64smoke-$$-$(date +%s)"
export R9_RUN_TAG
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	# the expect driver kills the live spawn's process group on every
	# exit path; this trap sweeps any survivor of EITHER leg.  the
	# pattern is bracket-fixed ([q]emu) so pkill cannot self-match this
	# shell's or its own argv, and keyed on THIS run's unique -name tag
	# — never on the kernel-path class.
	pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	rm -rf "$TMPD" "$TMPD2"
	exit "$status"
}
trap cleanup EXIT

echo "== step 2/4: composing the CHAIN ISO (r9's iso-builder: kernel + chain.paq as /boot/bootfs.paq)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" "$CHAIN_PAQ" bootfs.paq \
	|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

echo "== step 3/4: composing the CPU ISO (r9's iso-builder: kernel + cpu.paq as /boot/bootfs.paq + the bootconf append init=/amd64/init -c — 19-01's seam)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD2" --append="init=/amd64/init -c" "$CPU_PAQ" bootfs.paq \
	|| err "iso-builder failed for the cpu ISO (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_CPU_ISO="$TMPD2/boot.iso"
[ -s "$R9_CPU_ISO" ] || err "$R9_CPU_ISO missing/empty after the iso-builder"

# --- step 4: the expect-driven smoke (the chain leg + the cpu-chain leg) --

R9_SMOKE_LOG="${R9_SMOKE_LOG:-$R9_GATE_OUT/smoke-r9.log}"
R9_CPU_LOG="${R9_CPU_LOG:-$R9_GATE_OUT/smoke-r9-cpu.log}"
R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"

# fresh captures per run BEFORE launch (a pre-seeded stale log is dead
# evidence — each starts empty and only its leg's boot writes it; the
# driver rm -f's them again belt-and-braces)
rm -f "$R9_SMOKE_LOG" "$R9_CPU_LOG"

echo "== step 4/4: the composed smoke ($R9_SMOKE_DRIVER — the chain leg + the cpu-chain leg)"
echo "== captures: $R9_SMOKE_LOG + $R9_CPU_LOG"

export R9_ISO R9_CPU_ISO R9_SMOKE_LOG R9_CPU_LOG R9_QEMU R9_NETDEV_ARGV
export R9_MACHINE=q35 R9_CPU R9_ACCEL

set +e
expect "$R9_SMOKE_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "smoke FAILED (the driver named the stage on stderr; captures: $R9_SMOKE_LOG + $R9_CPU_LOG)"
fi
echo "== smoke PASS (chain leg + cpu-chain leg) — captures retained at $R9_SMOKE_LOG + $R9_CPU_LOG"
exit 0
