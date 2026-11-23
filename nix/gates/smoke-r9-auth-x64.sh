#!/usr/bin/env bash
# nix/gates/smoke-r9-auth-x64.sh — the composed AUTH gate as an r9
# flake app (plan 01-31, D-07/D-16/D-19, HARN-08; moved from Harvey
# bd7a698's nix/smoke-r9-auth-x64.sh and rewired onto r9's own supply,
# the 01-18/01-23/01-24 pattern).
#
# The SAME composed pipeline as the x86_64 net smoke gate (kernel
# contract → r9's image assembler → r9's iso-builder) but the assembled
# image is the AUTH payload (r9's xtask/manifests/amd64/auth.toml: the
# chain tree + the auth suite + the bake-time p9sk1 domain from
# authdomain-gen, with the D-02 pairing assert over the staged copies)
# and the launch runs under gates/x86_64/smoke-r9-auth.expect, which
# boots the image to the auth bring-up sweep (auth-termrc →
# `rc /lib/auth-sweep.rc r9', no typed legs) and asserts: the D-02 boot
# pairing banner with the DEMO label, keyfs's NON-interactive start
# (`N keys read', the nvram= env path), the sweep's AUTH-OK verdicts +
# AUTH-DONE, the never-appears post-pass for the exact getpass string,
# and zero AUTH-FAIL — and exits 0/1 for CI.  smoke-r9-net-x64's
# behavior is untouched (app-per-gate, one-image-per-gate — 14-06).
#
# Rewired onto r9's own supply:
#
#   1. the image — `cargo xtask image --arch x86-64 --image auth` from
#      the r9 checkout replaces Harvey's in-checkout stage-build +
#      subset-build + mkimage trio.  The image engine stages from the
#      read-only amd64 tree, runs the per-bake p9sk1 authdomain, and
#      writes target/images/x86_64/auth.paq plus the 0600
#      target/images/x86_64/auth.creds.  Nothing is written into the
#      Harvey tree (HARN-02); the R9_SKIP_BUILD iteration hatch is
#      deleted (the assembler's own staleness check is the only cache).
#      Harvey's script booted only the auth contract, so only
#      `--image auth` is assembled — `--image substrate` is NOT
#      assembled (no boot here consumes it).
#   2. the creds — auth.creds is parsed LOUDLY (user= then password=,
#      CR/LF stripped, non-empty or an error NAMING THE PATH) before
#      any compose or spawn.  Harvey's step 2b exported the password
#      VALUE as R9_DEMO_PW (T-01-77); here the password is written to a
#      0600 file under R9_GATE_OUT and only that PATH is exported as
#      R9_DEMO_PWFILE (the name plan 01-18 uses).  The value is never
#      argv, never an exported variable, and the EXIT trap removes the
#      file.  This script never prints the password.
#   3. compose — r9's exported iso-builder (R9_ISO_BUILDER: ONE
#      reproducibility implementation) builds the ISO from the pinned
#      kernel-x64 + auth.paq as /boot/bootfs.paq.
#   4. the gate — gates/x86_64/smoke-r9-auth.expect spawns qemu with
#      the composed ISO on -cdrom (q35, -nographic, the emitter's
#      slirp argv).  Markers are the ONLY proof — the supervisor's exit
#      status is never trusted.
#
# VENUE + NETDEV (16-06's mechanism, carried): the venue stays FIXED
# slirp — hermetic, unprivileged, CI-able — and the qemu netdev+device
# argv flows from the SAME pinned TOML data the boot app consumes
# (R9_NETDEV_EMITTER captured once below, exported as R9_NETDEV_ARGV
# for the expect driver to split) with the D-04 hostfwd pair included:
# host 5670 -> guest 567 (the authsrv listener's Phase-19 drawterm
# entry — the gate observes the host-side listener while the guest
# boots, 16-D-04 landing on a real listener day one, D-09).
#
# Env contract (the flake app apps.x86_64-linux.boot-r9-auth-x64
# exports the package paths; a standalone run needs the same values):
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree carrying
#                       /HARVEY-TREE (the flake's harvey-tree-amd64).
#   R9_ROOT             the r9 checkout root (default $PWD); the app is
#                       run from the repo root.
#   R9_KERNEL_X64_PKG   REQUIRED — the kernel-x64 package store path.
#   R9_ISO_BUILDER      REQUIRED — r9's iso-builder executable.
#   R9_NETDEV_EMITTER   REQUIRED — r9's netdev-args emitter executable
#                       (the slirp argv with the D-04 hostfwd pair comes
#                       ONLY from its output).
#   R9_MKPAQFS          REQUIRED — the host mkpaqfs (image bake).
#   R9_AUTHDOMAIN_GEN   REQUIRED — the host p9sk1 domain generator (the
#                       auth image's per-bake authdomain).
#   R9_GATE_OUT         output dir (default target/gates/smoke-r9-auth-x64).
#   R9_AUTH_LOG         default $R9_GATE_OUT/smoke-r9-auth.log.
#   R9_AUTH_DRIVER      default $R9_ROOT/gates/x86_64/smoke-r9-auth.expect.
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
# adds it to qemu's argv as -name), removes the 0600 password file, and
# removes the compose scratch — never a kernel-name class, so a foreign
# r9 qemu (a dev's interactive boot-r9-x64, a parallel CI job, a
# SIBLING gate) is never matched.  The r9x64auth- prefix differs from
# the sibling x64 gates' prefixes so the gates' survivor sweeps can
# never cross-kill.  Every output lives under R9_GATE_OUT; nothing is
# written under $HARVEY (T-01-78).
#
# Wrapper provides (the flake app's runtimeInputs):
# bootR9X64RuntimeInputs — the Rust toolchain (`cargo xtask image`),
# qemu, expect, procps (the pkill sweep), iproute2 (the driver's
# `exec ss') and the text tools.  stdenv.cc is NOT carried: the
# authdomain generator is r9's prebuilt store package, not an
# in-bake host build.
#
# usage: nix run .#boot-r9-auth-x64   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/smoke-r9-auth-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#boot-r9-auth-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/auth.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/auth.toml is missing — the auth image manifest is part of this checkout"
R9_AUTH_DRIVER="${R9_AUTH_DRIVER:-$R9_ROOT/gates/x86_64/smoke-r9-auth.expect}"
[ -f "$R9_AUTH_DRIVER" ] || err "R9_AUTH_DRIVER=$R9_AUTH_DRIVER: the auth gate driver is missing"

# --- step 0: the kernel-package + builder + assembler contract ---------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_NETDEV_EMITTER R9_MKPAQFS R9_AUTHDOMAIN_GEN; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#boot-r9-auth-x64), which exports the r9 supply"
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

# --- step 0c: the netdev argv — the emitter's slirp tokens (16-06) -------

# The venue stays FIXED slirp (D-02); the argv flows from the same
# pinned TOML data the boot app consumes, the D-04 hostfwd pair
# (host 5670 -> guest 567) included.  Resolved ONCE here, before any
# assembly spend, so an emitter failure is a named contract error — and
# exported as R9_NETDEV_ARGV for the expect driver to split.
NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode slirp) \
	|| err "the netdev emitter failed for mode slirp (its diagnostics are above; the argv comes only from the pinned r9 TOML — D-05)"
[ -n "$NETDEV_ARGV" ] || err "the netdev emitter printed no netdev argv for mode slirp"
export R9_NETDEV_ARGV="$NETDEV_ARGV"

echo "== r9 tree:      $HARVEY"
echo "== r9 root:      $R9_ROOT"
echo "== gate output:  $R9_GATE_OUT"
echo "== r9 kernel package: $R9_KERNEL_X64_PKG"
echo "   kernel:        $R9_KERNEL"
echo "   iso-builder:   $R9_ISO_BUILDER"
echo "   netdev emitter: $R9_NETDEV_EMITTER"
echo "   netdev argv (slirp): $NETDEV_ARGV"
echo "   shape:         q35 accel=$R9_ACCEL cpu=$R9_CPU"

export KERNDATE="${KERNDATE:-$(date +%s)}"

# --- the run scratch dir and the EXIT trap -----------------------------

mkdir -p "$R9_GATE_OUT"
TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
# The per-run unique tag — exported so smoke-r9-auth.expect adds it to
# qemu's argv (-name), and this run's survivor sweep (cleanup below)
# keys on it.  The sweep NEVER matches a kernel-name class: a foreign
# r9 qemu (a dev's interactive boot-r9-x64 uses the same kernel; a
# parallel CI job on a shared runner; the SIBLING gates' qemu) must
# never be killed by this gate's teardown.  The r9x64auth- prefix
# differs from the sibling x64 gates' tags for exactly that reason.
R9_RUN_TAG="r9x64auth-$$-$(date +%s)"
R9_DEMO_PWFILE=""
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
	if [ -n "$R9_DEMO_PWFILE" ]; then
		rm -f "$R9_DEMO_PWFILE"
	fi
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT

# --- step 1: the auth image (r9's assembler replaces Harvey's bake) ----

echo "== step 1/4: assembling image auth (cargo xtask image --arch x86-64 --image auth)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image auth ) \
	|| err "auth image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
AUTH_PAQ="$R9_ROOT/target/images/x86_64/auth.paq"
[ -s "$AUTH_PAQ" ] || err "$AUTH_PAQ missing/empty after assembly (the auth manifest bakes it)"
echo "== image ok: $AUTH_PAQ"

# --- step 2: the creds read (BEFORE any compose or spawn) --------------
# The password reaches the 0600 file and the driver only; it is never
# printed, never exported, and never copied into R9_GATE_OUT elsewhere.
R9_CREDS_FILE="$R9_ROOT/target/images/x86_64/auth.creds"
[ -s "$R9_CREDS_FILE" ] || err "$R9_CREDS_FILE missing/empty after assembly (the authdomain generator writes it at 0600)"
grep -q '^user=' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: no user= line (the parse contract)"
grep -q '^password=' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: no password= line (the parse contract)"
_pw=$(sed -n 's/^password=//p' "$R9_CREDS_FILE" | tr -d '\r\n')
[ -n "$_pw" ] || err "$R9_CREDS_FILE: the password= line parsed empty (empty creds — aborting before any spawn)"
R9_DEMO_PWFILE="$R9_GATE_OUT/.auth-password.$$"
( umask 077; printf '%s' "$_pw" >"$R9_DEMO_PWFILE" )
unset _pw
chmod 600 "$R9_DEMO_PWFILE"
[ -s "$R9_DEMO_PWFILE" ] || err "the 0600 password file is empty — aborting before any spawn"
export R9_DEMO_PWFILE
echo "== step 2/4: creds read ok (the password is never printed and lives only in the 0600 file)"

# --- step 3: compose the ISO -------------------------------------------

echo "== step 3/4: composing the ISO (r9's iso-builder: kernel + auth.paq as /boot/bootfs.paq)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" "$AUTH_PAQ" bootfs.paq \
	|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 4: the expect-driven composed auth gate ----------------------

R9_AUTH_LOG="${R9_AUTH_LOG:-$R9_GATE_OUT/smoke-r9-auth.log}"
R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"

# fresh capture per run BEFORE launch (a pre-seeded stale log is dead
# evidence — it starts empty and only this run's boot writes it; the
# driver rm -f's it again belt-and-braces)
rm -f "$R9_AUTH_LOG"

echo "== step 4/4: composed auth gate ($R9_AUTH_DRIVER)"
echo "== capture: $R9_AUTH_LOG"

export R9_ISO R9_SMOKE_LOG="$R9_AUTH_LOG" R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL

set +e
expect "$R9_AUTH_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "composed auth gate FAILED (the driver named the stage on stderr; capture: $R9_AUTH_LOG)"
fi
echo "== composed auth gate PASS — capture retained at $R9_AUTH_LOG"
exit 0
