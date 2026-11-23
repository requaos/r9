#!/usr/bin/env bash
# nix/gates/drawterm-gate-x64.sh — the drawterm gate as an r9 flake app
# (plan 01-18, D-07/D-09/D-16, HARN-01/HARN-02/HARN-08).
#
# Moved from Harvey bd7a698's nix/drawterm-gate-x64.sh and rewired onto
# r9's own supply:
#
#   1. the image — `cargo xtask image --arch x86-64 --image cpu` from the
#      r9 checkout replaces Harvey's in-checkout userland build and bake.
#      The image engine stages from the read-only flake tree, runs the
#      per-bake p9sk1 authdomain, and writes target/images/x86_64/cpu.paq
#      plus the 0600 target/images/x86_64/cpu.creds.  Nothing is written
#      into the Harvey tree (HARN-02).
#   2. the creds — cpu.creds is parsed LOUDLY (user= then password=,
#      CR/LF stripped, non-empty or an error NAMING THE PATH) before any
#      compose or spawn.  The password is written to a 0600 file under
#      R9_GATE_OUT and only that path is exported (R9_DEMO_PWFILE,
#      T-01-58); the value is never argv, never an exported variable, and
#      the EXIT trap removes the file.
#   3. compose — r9's iso-builder with the cpu append
#      (`init=/amd64/init -c`) over the kernel package and cpu.paq as
#      /boot/bootfs.paq.
#   4. the gate — gates/x86_64/drawterm-gate.expect under GATE_MODE
#      (session is this app's default and keeps the Harvey-hosted
#      assertions; rio/control-noG/force-no-rio are the quick gate's
#      legs, driven by `cargo xtask gate`).
#
# Venue (D-13/D-14): the REQUIRED green witness is LOCAL KVM, green
# twice; the Mac can only statically check.  The accel discipline is the
# auth driver's verbatim (flag > env > default; NO silent downgrade — a
# KVM-less venue fails loudly unless TCG was selected explicitly via
# R9_ACCEL=tcg).  R9_CPU defaults to host under kvm, qemu64 under tcg.
#
# Env contract (the flake app apps.x86_64-linux.drawterm-gate-x64 exports
# the package paths; a standalone run needs the same values):
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
#                       image's per-bake authdomain).
#   DRAWTERM_BIN        REQUIRED — Harvey's drawterm package binary.
#   R9_GATE_OUT         output dir (default target/gates/drawterm-gate-x64).
#   R9_ACCEL            kvm (default) | tcg — no auto, no silent downgrade.
#   R9_CPU              override the cpu model.
#   GATE_MODE           session (default) | rio | control-noG | force-no-rio.
#   GATE_SERIAL_CHECKS  newline-separated <cmd>=<marker> pairs (the driver).
#   R9_QEMU             default qemu-system-x86_64.
#   KERNDATE            bake-time timestamp override; the wall clock
#                       otherwise (the tree is a store path with no .git).
#
# Hygiene: every output lands under R9_GATE_OUT or the run scratch dir;
# the driver kills and reaps qemu/drawterm/Xvfb by recorded pid on every
# exit path and sweeps this run's identity; the argv0 tag is an extra key,
# never the only one.  The r9x64dtgate- prefix is this gate's OWN (the
# sibling-gate no-cross-kill rule).
#
# usage: nix run .#drawterm-gate-x64   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/drawterm-gate-x64}"
GATE_MODE="${GATE_MODE:-session}"
GATE_DRIVER="${GATE_DRIVER:-$R9_ROOT/gates/x86_64/drawterm-gate.expect}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and mode contract ----------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#drawterm-gate-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/cpu.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/cpu.toml is missing — the cpu image manifest is part of this checkout"
[ -f "$GATE_DRIVER" ] || err "GATE_DRIVER=$GATE_DRIVER: the drawterm gate driver is missing"

case "$GATE_MODE" in
	session|rio|control-noG|force-no-rio) ;;
	*) err "GATE_MODE=$GATE_MODE: want session|rio|control-noG|force-no-rio" ;;
esac

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_NETDEV_EMITTER R9_MKPAQFS R9_AUTHDOMAIN_GEN DRAWTERM_BIN; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#drawterm-gate-x64), which exports the r9 supply"
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
[ -f "$DRAWTERM_BIN" ] && [ -x "$DRAWTERM_BIN" ] \
	|| err "DRAWTERM_BIN $DRAWTERM_BIN: not an executable file (want the drawterm package's bin/drawterm)"

# --- step 0b: accel selection (no silent downgrade; KVM is the witness) --

R9_ACCEL="${R9_ACCEL:-kvm}"
case "$R9_ACCEL" in
	kvm)
		[ -w /dev/kvm ] \
			|| err "/dev/kvm absent or not writable and R9_ACCEL=$R9_ACCEL — no silent downgrade: set R9_ACCEL=tcg explicitly for the TCG fallback"
		R9_CPU="${R9_CPU:-host}"
		;;
	tcg)
		R9_CPU="${R9_CPU:-qemu64}"
		;;
	*) err "R9_ACCEL=$R9_ACCEL: want kvm|tcg" ;;
esac
export R9_ACCEL R9_CPU

# --- step 0c: the netdev argv — the emitter's slirp tokens -------------

# The D-04 hostfwd pair (host 17010 -> guest 17010, host 5670 ->
# guest 567) rides the pinned TOML data; the drawterm session dials
# exactly this pair.  Resolved ONCE here, before any build spend, so an
# emitter failure is a named contract error.
NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode slirp) \
	|| err "the netdev emitter failed for mode slirp (its diagnostics are above; the argv comes only from the pinned r9 TOML)"
[ -n "$NETDEV_ARGV" ] || err "the netdev emitter printed no netdev argv for mode slirp"
export R9_NETDEV_ARGV="$NETDEV_ARGV"

export KERNDATE="${KERNDATE:-$(date +%s)}"

echo "== r9 tree:      $HARVEY"
echo "== r9 root:      $R9_ROOT"
echo "== gate mode:    $GATE_MODE"
echo "== gate output:  $R9_GATE_OUT"
echo "== kernel:       $R9_KERNEL"
echo "== iso-builder:  $R9_ISO_BUILDER"
echo "== netdev emitter: $R9_NETDEV_EMITTER"
echo "== drawterm:     $DRAWTERM_BIN"
echo "== netdev argv (slirp): $NETDEV_ARGV"
echo "== dial contract:  cpu=127.0.0.1 auth=tcp!127.0.0.1!5670 (the committed hostfwd pair's loopback side; never widened)"
echo "== shape:        q35 accel=$R9_ACCEL cpu=$R9_CPU"

# --- the run scratch dir and the EXIT trap -----------------------------

mkdir -p "$R9_GATE_OUT"
TMPD=$(mktemp -d "${TMPDIR:-/tmp}/drawterm-gate-x64.XXXXXX")
R9_RUN_TAG="r9x64dtgate-$$-$(date +%s)"
R9_DT_TAG="r9x64dtgate-dt-$$-$(date +%s)"
R9_DEMO_PWFILE=""
export R9_RUN_TAG R9_DT_TAG

# shellcheck disable=SC2329  # invoked via trap
cleanup() {
	status=$?
	# The driver kills both process groups on every exit path; this trap
	# sweeps any survivor keyed on THIS run's unique tags — never on a
	# kernel-name or binary-name class, so a foreign r9 qemu (a dev's
	# interactive boot, a sibling gate) or drawterm session is never
	# cross-killed.
	pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	pkill -f "^${R9_DT_TAG} " 2>/dev/null || true
	pkill -f "[X]vfb .*${R9_RUN_TAG}" 2>/dev/null || true
	if [ -n "$R9_DEMO_PWFILE" ]; then
		rm -f "$R9_DEMO_PWFILE"
	fi
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT

# --- step 1: the cpu image (r9's assembler replaces Harvey's bake) -----

echo "== step 1/4: assembling image cpu (cargo xtask image --arch x86-64 --image cpu)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image cpu ) \
	|| err "cpu image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"

CPU_PAQ="$R9_ROOT/target/images/x86_64/cpu.paq"
[ -s "$CPU_PAQ" ] || err "$CPU_PAQ missing/empty after assembly (the cpu manifest bakes it)"
echo "== image ok: $CPU_PAQ"

# --- step 2: the creds read (BEFORE any compose or spawn) --------------
# The password reaches the 0600 file and the driver only; it is never
# printed, never exported, and never copied into R9_GATE_OUT elsewhere.
R9_CREDS_FILE="$R9_ROOT/target/images/x86_64/cpu.creds"
[ -s "$R9_CREDS_FILE" ] || err "$R9_CREDS_FILE missing/empty after assembly (the authdomain generator writes it at 0600)"
grep -q '^user=' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: no user= line (the parse contract)"
grep -q '^password=' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: no password= line (the parse contract)"
R9_DEMO_USER=$(sed -n 's/^user=//p' "$R9_CREDS_FILE" | tr -d '\r\n')
_pw=$(sed -n 's/^password=//p' "$R9_CREDS_FILE" | tr -d '\r\n')
[ -n "$R9_DEMO_USER" ] || err "$R9_CREDS_FILE: the user= line parsed empty (empty creds — aborting before any spawn)"
[ -n "$_pw" ] || err "$R9_CREDS_FILE: the password= line parsed empty (empty creds — aborting before any spawn)"

R9_DEMO_PWFILE="$R9_GATE_OUT/.dt-password.$$"
( umask 077; printf '%s' "$_pw" >"$R9_DEMO_PWFILE" )
unset _pw
chmod 600 "$R9_DEMO_PWFILE"
[ -s "$R9_DEMO_PWFILE" ] || err "the 0600 password file is empty — aborting before any spawn"
export R9_DEMO_USER R9_DEMO_PWFILE R9_CREDS_FILE
echo "== step 2/4: creds read ok (user=$R9_DEMO_USER; the password is never printed and lives only in the 0600 file)"

# --- step 3: compose the ISO WITH the cpu append -----------------------

echo "== step 3/4: composing the ISO (kernel + cpu.paq as /boot/bootfs.paq + the init append)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" --append='init=/amd64/init -c' "$CPU_PAQ" bootfs.paq \
	|| err "iso-builder failed (R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"
export R9_ISO R9_MACHINE=q35
export R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"
export GATE_MODE
export GATE_SERIAL_CHECKS="${GATE_SERIAL_CHECKS:-}"

# --- step 4: the expect-driven gate ------------------------------------

R9_SMOKE_LOG="${R9_SMOKE_LOG:-$R9_GATE_OUT/drawterm-gate-${GATE_MODE}.log}"
mkdir -p "$(dirname "$R9_SMOKE_LOG")"
rm -f "$R9_SMOKE_LOG"
export R9_SMOKE_LOG

echo "== step 4/4: the $GATE_MODE gate ($GATE_DRIVER)"
echo "== capture: $R9_SMOKE_LOG"

set +e
expect "$GATE_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "the $GATE_MODE drawterm gate FAILED (rc=$RC — the driver named the failure on stderr; capture: $R9_SMOKE_LOG)"
fi
echo "== drawterm gate PASS ($GATE_MODE) — capture retained at $R9_SMOKE_LOG"
exit 0
