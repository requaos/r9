#!/usr/bin/env bash
# s3/rio-gate.sh — Harvey's drawterm gate rewired for spike S3 (D-11).
#
# Copy-and-rewire of Harvey bd7a6982a's `nix/drawterm-gate-x64.sh`, with
# `build/x86_64/drawterm-gate.expect` re-keyed as `s3/rio-gate.expect`.
# Every contract of the original is kept: the step-0 package checks, the
# KVM-default accel selection with NO silent downgrade, the netdev argv
# captured from r9's emitter, capture-before-start, the bake-time p9sk1
# creds parse (the password is never printed), the run-tag survivor
# sweeps, and teardown on every exit path.  What changes is what the
# drawterm session is asked to prove:
#
#   S3_MODE=rio (default)
#     Xvfb is started on a free display with TCP listening off
#     (`-displayfd` + `-nolisten tcp`), DISPLAY is exported, and
#     drawterm runs WITHOUT `-G` so the cpu profile sees
#     /mnt/term/dev/draw and runs `plumber; exec rio` (Pitfall 6: a
#     -G session has no draw device and never starts rio).  Once the
#     qemu serial console owns an interactive rc, the driver types
#     `ls /srv` and requires both `rio.<user>.<pid>` and
#     `riowctl.<user>.<pid>`; it prints S3-RIO-SERVING and exits 0.
#     Named failures: S3-FAIL rio-not-serving, S3-FAIL
#     drawterm-exited, S3-FAIL no-session.
#
#   S3_MODE=control-noG
#     The same pipeline with `-G` (no draw device, so the profile must
#     skip rio).  `ls /srv` must show no `rio.` entry; the driver
#     prints S3-CONTROL-NO-RIO and exits 0, or S3-FAIL
#     control-saw-rio with a non-zero exit when one is present.
#
# The session logs in as the user whose staged home carries the cpu
# profile (/usr/glenda/lib/profile).  The bake's creds file names the
# DEMO probe user (norbert), who has no staged home; the DEMO domain
# gives every user one shared password, so the parsed password
# authenticates glenda too.  S3_DT_USER overrides the login name.
#
# Hygiene / T-01-17/T-01-18/T-01-19:
#   - HARVEY must be a WRITABLE SCRATCH COPY.  A directory with a .git
#     entry (a real checkout) or any path under /nix/store is refused
#     with a named error before anything runs.
#   - The script writes only under S3_OUT and inside that scratch copy.
#     It never copies build/x86_64/out/auth-creds.txt (or any credential
#     artifact) into S3_OUT; only the bake log and the redacted capture
#     are retained.
#   - Xvfb runs with TCP listening off and is killed on every exit path
#     and swept by a display-keyed pkill; qemu and drawterm keep the
#     original per-run tag sweeps.
#
# Env contract (the flake app apps.x86_64-linux.s3-rio-gate exports the
# package paths; a standalone run needs the same values):
#   HARVEY            REQUIRED — writable scratch Harvey copy.
#   S3_OUT            output dir (default $PWD/s3-out; created).
#   S3_MODE           rio (default) | control-noG.
#   R9_KERNEL_X64_PKG REQUIRED — r9 kernel-x64 package store path.
#   R9_ISO_BUILDER    REQUIRED — r9's iso-builder executable.
#   R9_NETDEV_EMITTER REQUIRED — r9's netdev-args emitter executable.
#   DRAWTERM_BIN      REQUIRED — Harvey's drawterm package executable
#                     (the X11 backend; -G runs from the same binary).
#   R9_SKIP_BUILD     =1 skips step 1 (iteration hatch; not the gate).
#   R9_ACCEL          kvm (default) | tcg — no auto, no silent downgrade.
#   R9_CPU            override the cpu model.
#   KERNDATE          bake-time timestamp (default: scratch tree's git
#                     time when present, else the wall clock).
#   S3_DT_USER        drawterm login name (default glenda).
#   S3_RIO_BUDGET     seconds allowed for rio to post its srv files
#                     after the console prompt (default 60).
#   S3_SESSION_WINDOW seconds to watch the drawterm pty for an auth
#                     rejection before treating the session as accepted
#                     (default 30).
#   S3_SRV_POLLS / S3_SRV_POLL_WAIT
#                     `ls /srv` poll count / per-poll seconds (12 / 5).
#   S3_EXPECT         the expect driver path (the flake app exports the
#                     store copy; a direct run defaults to a sibling).
#   S3_FLAKE          the flake source path the app exports; used to
#                     resolve the three locked r9 packages when they are
#                     not set directly.
#
# usage: nix run path:<r9>/spikes/s3#s3-rio-gate   (owner's x86_64 Linux)

set -euo pipefail

HARVEY="${HARVEY:-}"
S3_OUT="${S3_OUT:-$PWD/s3-out}"
S3_MODE="${S3_MODE:-rio}"
S3_DT_USER="${S3_DT_USER:-glenda}"
S3_RIO_BUDGET="${S3_RIO_BUDGET:-60}"
S3_SESSION_WINDOW="${S3_SESSION_WINDOW:-30}"
S3_SRV_POLLS="${S3_SRV_POLLS:-12}"
S3_SRV_POLL_WAIT="${S3_SRV_POLL_WAIT:-5}"
S3_EXPECT="${S3_EXPECT:-$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/rio-gate.expect}"

R9_RUN_TAG=""
R9_DT_TAG=""
S3_XVFB_PID=""
DISPLAY=""

err() { echo "S3-ERROR: $*" >&2; exit 1; }

# --- step 0: the scratch-copy and mode contract (S3 additions) ---------

[ -n "$HARVEY" ] || err "HARVEY unset — point HARVEY at the writable scratch Harvey copy (never the real checkout)"
[ -d "$HARVEY/sys/src" ] || err "$HARVEY does not look like a Harvey tree (missing sys/src) — run from a scratch copy of bd7a6982a"
case "$HARVEY" in
	/nix/store/*) err "harvey-in-store: HARVEY=$HARVEY is a read-only store path; S3 needs a writable scratch copy" ;;
esac
[ ! -e "$HARVEY/.git" ] || err "harvey-is-checkout: HARVEY=$HARVEY has a .git entry; S3 refuses to run against a real Harvey checkout (copy the pinned tree instead)"
case "$S3_OUT" in
	/nix/store/*) err "s3-out-in-store: S3_OUT=$S3_OUT is a read-only store path" ;;
esac
case "$S3_MODE" in
	rio|control-noG) ;;
	*) err "S3_MODE=$S3_MODE: want rio|control-noG" ;;
esac

mkdir -p "$S3_OUT"

# --- step 0a: resolve the locked r9 packages ---------------------------
# The flake app exports S3_FLAKE (this flake's store source path) and
# DRAWTERM_BIN, but deliberately does NOT interpolate r9's packages into
# the wrapper: r9 116bcc2's kernel derivation reads rust-src's
# Cargo.lock through import-from-derivation, which cannot be built on
# aarch64-darwin.  Resolving here keeps the Mac's `nix eval` of the app
# free of IFD while still binding the exact packages Harvey's own gate
# consumes (every input is locked by flake.lock).  A standalone run may
# export the three package paths directly instead.
if [ -z "${R9_KERNEL_X64_PKG:-}" ] || [ -z "${R9_ISO_BUILDER:-}" ] || [ -z "${R9_NETDEV_EMITTER:-}" ]; then
	[ -n "${S3_FLAKE:-}" ] || err "R9_KERNEL_X64_PKG/R9_ISO_BUILDER/R9_NETDEV_EMITTER unset and S3_FLAKE unset — run via the flake app (nix run ...#s3-rio-gate) or export all three"
	command -v nix >/dev/null 2>&1 || err "nix not found on PATH; cannot resolve the locked r9 packages through $S3_FLAKE"
	_s3_resolve() {
		nix build --no-link --print-out-paths "path:${S3_FLAKE}#$1"
	}
	R9_KERNEL_X64_PKG=$(_s3_resolve kernel-x64) || err "nix build of the locked kernel-x64 failed"
	R9_ISO_BUILDER="$(_s3_resolve iso-builder)/bin/r9-iso-builder"
	R9_NETDEV_EMITTER="$(_s3_resolve netdev-args)/bin/r9-netdev-args"
fi
export R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_NETDEV_EMITTER

# --- step 0b: the kernel-package + builder contract (verbatim) ---------

[ -n "${R9_KERNEL_X64_PKG:-}" ] || err "R9_KERNEL_X64_PKG unset — run via the flake app (nix run ...#s3-rio-gate), which resolves the r9 kernel-x64 store path"
[ -d "$R9_KERNEL_X64_PKG" ] || err "R9_KERNEL_X64_PKG $R9_KERNEL_X64_PKG: not a directory (unrealised store path?)"

R9_KERNEL="$R9_KERNEL_X64_PKG/kernel-x64.elf"
if [ ! -f "$R9_KERNEL" ]; then
	R9_KERNEL=$(find "$R9_KERNEL_X64_PKG" -name 'kernel-x64.elf' -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_KERNEL" ] && [ -f "$R9_KERNEL" ] || err "kernel-x64.elf not found under $R9_KERNEL_X64_PKG"

[ -n "${R9_ISO_BUILDER:-}" ] || err "R9_ISO_BUILDER unset — run via the flake app, which exports r9's iso-builder script path"
[ -f "$R9_ISO_BUILDER" ] && [ -x "$R9_ISO_BUILDER" ] \
	|| err "R9_ISO_BUILDER $R9_ISO_BUILDER: not an executable file (want <pkg>/bin/r9-iso-builder)"

[ -n "${R9_NETDEV_EMITTER:-}" ] || err "R9_NETDEV_EMITTER unset — run via the flake app, which exports r9's netdev-args emitter path"
[ -f "$R9_NETDEV_EMITTER" ] && [ -x "$R9_NETDEV_EMITTER" ] \
	|| err "R9_NETDEV_EMITTER $R9_NETDEV_EMITTER: not an executable file (want <pkg>/bin/r9-netdev-args)"

[ -n "${DRAWTERM_BIN:-}" ] || err "DRAWTERM_BIN unset — run via the flake app, which exports Harvey's drawterm package binary"
[ -f "$DRAWTERM_BIN" ] && [ -x "$DRAWTERM_BIN" ] \
	|| err "DRAWTERM_BIN $DRAWTERM_BIN: not an executable file (want drawterm's bin/drawterm; the X11 backend is linked)"

[ -f "$S3_EXPECT" ] || err "S3_EXPECT=$S3_EXPECT: the rio-gate.expect driver is missing"

# --- step 0b: accel selection (no silent downgrade; KVM is the witness) -

R9_ACCEL="${R9_ACCEL:-kvm}"
case "$R9_ACCEL" in
	kvm)
		[ -e /dev/kvm ] || err "/dev/kvm absent and R9_ACCEL=$R9_ACCEL — no silent downgrade: set R9_ACCEL=tcg explicitly for the TCG fallback (KVM is the D-13 witness)"
		R9_CPU="${R9_CPU:-host}"
		;;
	tcg)
		R9_CPU="${R9_CPU:-qemu64}"
		;;
	*) err "R9_ACCEL=$R9_ACCEL: want kvm|tcg" ;;
esac

# --- step 0c: the netdev argv — the emitter's slirp tokens -------------

NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode slirp) \
	|| err "the netdev emitter failed for mode slirp (its diagnostics are above; the argv comes only from the pinned r9 TOML — D-05)"
[ -n "$NETDEV_ARGV" ] || err "the netdev emitter printed no netdev argv for mode slirp"
export R9_NETDEV_ARGV="$NETDEV_ARGV"

echo "== S3 mode:        $S3_MODE"
echo "== scratch Harvey: $HARVEY"
echo "== S3 output:      $S3_OUT"
echo "== r9 kernel:      $R9_KERNEL"
echo "== iso-builder:    $R9_ISO_BUILDER"
echo "== netdev emitter: $R9_NETDEV_EMITTER"
echo "== drawterm:       $DRAWTERM_BIN"
echo "== netdev argv (slirp): $NETDEV_ARGV"
echo "== dial contract:  user=$S3_DT_USER cpu=127.0.0.1 auth=tcp!127.0.0.1!5670 (the committed hostfwd pair's loopback side; never widened)"
echo "== shape:          q35 accel=$R9_ACCEL cpu=$R9_CPU"

# --- provenance (KERNDATE; the boot-r9-x64.sh discipline) --------------

if [ -z "${KERNDATE:-}" ]; then
	KERNDATE=$(cd "$HARVEY" && git log -1 --format=%ct 2>/dev/null) || KERNDATE=
fi
if ! [[ "$KERNDATE" =~ ^[0-9]+$ ]]; then
	KERNDATE=$(date +%s)
fi
export KERNDATE

# --- step 1: the amd64 userland (the tier build + the cpu suite) -------

if [ "${R9_SKIP_BUILD:-0}" = "1" ]; then
	echo "== R9_SKIP_BUILD=1 — skipping the userland build (iteration hatch; not the gate)"
else
	echo "== step 1/5a: stage the mkimage inputs (build/x86_64/stage-build.sh)"
	sh "$HARVEY/build/x86_64/stage-build.sh" || err "stage build failed"
	echo "== step 1/5: amd64 tier build (build/x86_64/subset-build.sh — the cpu bake's inputs incl. cmd/cpu + aux/listen + sed)"
	sh "$HARVEY/build/x86_64/subset-build.sh" || err "tier build failed"
fi

# --- step 2: bake (ALL contracts; the cpu image last, so
#     out/auth-creds.txt is the cpu pair) + post-bake asserts ------------

OUTD="$HARVEY/build/x86_64/out"
BAKE_LOG="$S3_OUT/bake.log"
echo "== step 2/5: bake (build/x86_64/mkimage.sh — the cpu image composes last; log tee'd to $BAKE_LOG)"
mkdir -p "$OUTD"
rm -f "$BAKE_LOG"
sh "$HARVEY/build/x86_64/mkimage.sh" 2>&1 | tee "$BAKE_LOG" || err "bootfs bake failed (the bake log is at $BAKE_LOG)"

CPU_PAQ="$OUTD/bootfs-cpu.paq"
[ -s "$CPU_PAQ" ] || err "$CPU_PAQ missing/empty after the bake (mkimage's cpu section)"
grep -q 'bootfs-cpu.paq' "$BAKE_LOG" \
	|| err "the bake log names no bootfs-cpu.paq PASS line (the cpu bake did not complete — see $BAKE_LOG)"
grep -q 'cpu pairing verified' "$BAKE_LOG" \
	|| err "the bake log carries no cpu pairing line (cpu pairing verified — see $BAKE_LOG)"
echo "== post-bake asserts ok: bootfs-cpu.paq present, the pairing line in the bake log"

# --- step 3: the creds read (BEFORE any compose or spawn) --------------
# The password reaches only the pty feedstage and the redaction pass.
# Nothing here copies the bake creds file into the output directory.
R9_CREDS_FILE="$OUTD/auth-creds.txt"
[ -s "$R9_CREDS_FILE" ] || err "$R9_CREDS_FILE missing/empty after the bake (the cpu bake's host-side creds copy)"
grep -q '^user=' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: no user= line (the parse contract)"
grep -q '^password=' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: no password= line (the parse contract)"
R9_DEMO_USER=$(sed -n 's/^user=//p' "$R9_CREDS_FILE" | tr -d '\r\n')
R9_DEMO_PW=$(sed -n 's/^password=//p' "$R9_CREDS_FILE" | tr -d '\r\n')
[ -n "$R9_DEMO_USER" ] || err "$R9_CREDS_FILE: the user= line parsed empty (empty creds — aborting before any spawn)"
[ -n "$R9_DEMO_PW" ] || err "$R9_CREDS_FILE: the password= line parsed empty (empty creds — aborting before any spawn)"
export R9_DEMO_USER R9_DEMO_PW R9_CREDS_FILE
echo "== step 3/5: creds read ok (user=$R9_DEMO_USER; the password is never printed)"

# --- step 4: compose the ISO WITH the cpu append -----------------------

TMPD=$(mktemp -d "${TMPDIR:-/tmp}/s3-rio-gate.XXXXXX")
R9_RUN_TAG="r9s3riogate-$$-$(date +%s)"
R9_DT_TAG="r9s3riogate-dt-$$-$(date +%s)"
export R9_RUN_TAG R9_DT_TAG
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	if [ -n "$S3_XVFB_PID" ]; then
		kill "$S3_XVFB_PID" 2>/dev/null || true
		wait "$S3_XVFB_PID" 2>/dev/null || true
	fi
	if [ -n "$DISPLAY" ]; then
		pkill -f "[X]vfb ${DISPLAY} " 2>/dev/null || true
	fi
	if [ -n "$R9_RUN_TAG" ]; then
		pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	fi
	if [ -n "$R9_DT_TAG" ]; then
		pkill -f "^${R9_DT_TAG} " 2>/dev/null || true
	fi
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT

echo "== step 4/5: composing the ISO (r9's iso-builder: kernel + bootfs-cpu.paq as /boot/bootfs.paq + the init append)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" --append="init=/amd64/init -c" "$CPU_PAQ" bootfs.paq \
	|| err "iso-builder failed (R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 5: the Xvfb venue (rio mode only) + the expect gate ----------

S3_LOG="${S3_LOG:-$S3_OUT/rio-gate-${S3_MODE}.log}"
rm -f "$S3_LOG"

export S3_MODE S3_DT_USER S3_RIO_BUDGET S3_SESSION_WINDOW S3_SRV_POLLS S3_SRV_POLL_WAIT

if [ "$S3_MODE" = "rio" ]; then
	echo "== step 5/5: starting Xvfb (free display via -displayfd, TCP listening off)"
	S3_DISPLAY_FILE="$S3_OUT/xvfb-display"
	rm -f "$S3_DISPLAY_FILE"
	Xvfb -displayfd 3 -screen 0 1024x768x24 -nolisten tcp 3>"$S3_DISPLAY_FILE" &
	S3_XVFB_PID=$!
	_s3_wait=0
	while [ ! -s "$S3_DISPLAY_FILE" ]; do
		_s3_wait=$((_s3_wait + 1))
		if [ "$_s3_wait" -ge 100 ]; then
			err "Xvfb did not report a display on -displayfd within 10s (pid $S3_XVFB_PID)"
		fi
		sleep 0.1
	done
	DISPLAY=":$(cat "$S3_DISPLAY_FILE")"
	export DISPLAY
	echo "== Xvfb ready: DISPLAY=$DISPLAY (-nolisten tcp)"
else
	echo "== step 5/5: control-noG — no X server needed (drawterm -G has no draw device)"
fi

R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"
export R9_ISO R9_SMOKE_LOG="$S3_LOG" R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL

echo "== the S3 gate (s3/rio-gate.expect); capture: $S3_LOG"
set +e
expect "$S3_EXPECT"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "the S3 $S3_MODE gate FAILED (rc=$RC — the driver named the failure on stderr; capture: $S3_LOG)"
fi
echo "== S3 $S3_MODE gate PASS — capture retained at $S3_LOG"
exit 0
