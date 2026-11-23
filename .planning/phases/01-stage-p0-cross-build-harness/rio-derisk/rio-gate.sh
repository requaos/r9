#!/usr/bin/env bash
# rio-gate.sh — plan 01-09's fixed headless rio gate (D-28).
#
# A pass-2 copy of spike S3's `rio-gate.sh` (itself a rewired copy of
# Harvey bd7a6982a's `nix/drawterm-gate-x64.sh`), with the two pass-1
# defects fixed per rio-derisk/RIO-DERISK.md:
#
#   1. the DEMO password is parsed from the bake's out/auth-creds.txt
#      (CREDENTIAL-PATH) and written to a 0600 file inside the run's
#      scratch dir; only that path is exported.  The password is never an
#      argv entry and never an exported variable;
#   2. rio mode feeds the password through the X11 keyboard path with
#      xdotool/XTEST on the private Xvfb display (PROMPT-PATH + FIX),
#      because drawterm without -G has no host-stdin reader and paints
#      the prompt into its window, not the pty.
#
# Every other S3 contract is kept: the step-0 package checks, the
# KVM-default accel selection with NO silent downgrade, the netdev argv
# captured from r9's emitter, capture-before-start, the bake-time p9sk1
# creds parse (the password is never printed), and teardown on every exit
# path.  The teardown now reaps by recorded pid plus a run-identity sweep
# (cmdline/environ), never by the argv0 tag alone (S3-FINDINGS).
#
#   RIO_MODE=rio (default)
#     Xvfb starts on a free display with TCP listening off
#     (`-displayfd` + `-nolisten tcp`), DISPLAY is exported, and drawterm
#     runs WITHOUT `-G` so the cpu profile sees /mnt/term/dev/draw and
#     runs `plumber; exec rio`.  The driver waits for the drawterm window
#     (WM class `drawterm`), focuses it, and types the password from the
#     0600 file with XTEST.  Once the qemu serial console owns an
#     interactive rc, the driver types `ls /srv` and requires BOTH
#     `rio.<user>.<pid>` and `riowctl.<user>.<pid>`; it prints
#     RIO-SERVING and exits 0.  Named failures: RIO-FAIL rio-not-serving,
#     RIO-FAIL dt-feed-timeout, RIO-FAIL drawterm-exited, RIO-FAIL
#     no-session.
#
#   RIO_MODE=control-noG
#     The same pipeline with `-G` (no draw device, so the profile must
#     skip rio).  The unchanged pty password feed applies.  `ls /srv`
#     must show no `rio.` entry; the driver prints RIO-CONTROL-NO-RIO and
#     exits 0, or RIO-FAIL control-saw-rio with a non-zero exit when one
#     is present (D-28: the control stays unchanged).
#
#   RIO_MODE=rio-diag
#     The rio leg plus the login marker (plan 01-32; RIO-DERISK.md
#     "Diagnostic design and stop A2").  Before the bake, the scratch
#     tree's staged cpu profile
#     (build/x86_64/root-tree/usr/glenda/lib/profile) gains two creates
#     immediately before its `exec rio' line:
#         syscall create /srv/r9-login-ok 1 0666
#         syscall create /srv/r9-login-pid.$pid 1 0666
#     and is restored from a backup on every exit path.  The X11 feed,
#     session window and D-28 pair poll are the rio leg's, unchanged.
#     The serial console's `ls /srv' therefore shows r9-login-ok (and
#     r9-login-pid.<pid>) only when the authenticated profile reached its
#     rio handoff: presence separates RIO-START-FAIL (marker present,
#     pair absent) from LOGIN-FAIL (marker absent).  The marker is an #s
#     service entry, not a regular file: devsrv create is OWRITE-only and
#     its write expects a decimal fd, so the entry NAME is the payload
#     and an unposted entry is listed by read but cannot be cat'ed
#     (port/src/devsrv.rs create/read/write).  The driver also captures
#     x11.txt (window id/geometry/focus/xprop/xwininfo) and two PNGs
#     (before typing, after Return) into RIO_OUT.  This is a diagnostic,
#     not a gate: RIO-SERVING still requires the D-28 pair, and no PASS
#     is recorded from the marker alone.
#
# The session logs in as the user whose staged home carries the cpu
# profile (/usr/glenda/lib/profile).  The bake's creds file names the
# DEMO probe user (norbert), who has no staged home; the DEMO domain
# gives every user one shared password, so the parsed password
# authenticates glenda too (RIO-DERISK.md LOGIN-USER).
#
# Hygiene / T-01-30/T-01-31/T-01-32:
#   - HARVEY must be a WRITABLE SCRATCH COPY.  A directory with a .git
#     entry (a real checkout) or any path under /nix/store is refused
#     with a named error before anything runs.
#   - The script writes only under RIO_OUT and inside that scratch copy.
#     It never copies build/x86_64/out/auth-creds.txt (or any credential
#     artifact) into RIO_OUT; only the bake log and the redacted capture
#     are retained.  The 0600 password file lives in the run scratch dir
#     and is removed by the EXIT trap.
#   - Xvfb runs with TCP listening off and is killed on every exit path;
#     qemu, Xvfb and drawterm are reaped by recorded pid plus a sweep for
#     the run's identity in /proc/<pid>/{cmdline,environ}.
#
# Env contract (the flake app apps.x86_64-linux.rio-gate exports the
# package paths; a standalone run needs the same values):
#   HARVEY            REQUIRED — writable scratch Harvey copy.
#   RIO_OUT           output dir (default ${S3_OUT:-$PWD/rio-out}; created).
#   RIO_MODE          rio (default) | control-noG | rio-diag
#                     (S3_MODE is an alias; rio-diag is the diagnostic
#                     leg described above, never a gate).
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
#   RIO_DT_USER       drawterm login name (default glenda).
#   RIO_RIO_BUDGET    seconds allowed for rio to post its srv files
#                     after the console prompt (default 60).
#   RIO_SESSION_WINDOW seconds to watch the drawterm pty for an auth
#                     rejection before treating the session as accepted
#                     (default 30).
#   RIO_SRV_POLLS / RIO_SRV_POLL_WAIT
#                     `ls /srv` poll count / per-poll seconds (12 / 5).
#   RIO_FEED_WAIT     seconds to wait for the drawterm window before
#                     RIO-FAIL dt-feed-timeout (default 30).
#   RIO_FEED_SETTLE   seconds between focusing the window and typing
#                     (default 2).
#   RIO_EXPECT        the expect driver path (the flake app exports the
#                     store copy; a direct run defaults to a sibling).
#   RIO_FLAKE         the flake source path the app exports; used to
#                     resolve the three locked r9 packages when they are
#                     not set directly.
#
# usage: nix run path:<r9>/<phase>/rio-derisk#rio-gate   (x86_64 Linux)

set -euo pipefail

HARVEY="${HARVEY:-}"
RIO_OUT="${RIO_OUT:-${S3_OUT:-$PWD/rio-out}}"
RIO_MODE="${RIO_MODE:-${S3_MODE:-rio}}"
RIO_DT_USER="${RIO_DT_USER:-${S3_DT_USER:-glenda}}"
RIO_RIO_BUDGET="${RIO_RIO_BUDGET:-${S3_RIO_BUDGET:-60}}"
RIO_SESSION_WINDOW="${RIO_SESSION_WINDOW:-${S3_SESSION_WINDOW:-30}}"
RIO_SRV_POLLS="${RIO_SRV_POLLS:-${S3_SRV_POLLS:-12}}"
RIO_SRV_POLL_WAIT="${RIO_SRV_POLL_WAIT:-${S3_SRV_POLL_WAIT:-5}}"
RIO_FEED_WAIT="${RIO_FEED_WAIT:-30}"
RIO_FEED_SETTLE="${RIO_FEED_SETTLE:-2}"
RIO_EXPECT="${RIO_EXPECT:-${S3_EXPECT:-$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/rio-gate.expect}}"

# The diagnostic marker names (plan 01-32).  They are #s service entries
# in the kernel-wide srv table, so the serial console's `ls /srv' sees
# them from any namespace that binds #s at /srv; the pid rides the second
# entry's name because an unposted service file cannot be read.
RIO_DIAG=0
RIO_DIAG_MARKER="/srv/r9-login-ok"
RIO_DIAG_PID_PREFIX="/srv/r9-login-pid."
RIO_DIAG_PROFILE=""
RIO_DIAG_PROFILE_BAK=""

R9_RUN_TAG=""
R9_DT_TAG=""
RIO_XVFB_PID=""
DISPLAY=""
TMPD=""

err() { echo "RIO-ERROR: $*" >&2; exit 1; }

# --- step 0: the scratch-copy and mode contract ------------------------

[ -n "$HARVEY" ] || err "HARVEY unset — point HARVEY at the writable scratch Harvey copy (never the real checkout)"
[ -d "$HARVEY/sys/src" ] || err "$HARVEY does not look like a Harvey tree (missing sys/src) — run from a scratch copy of bd7a6982a"
case "$HARVEY" in
	/nix/store/*) err "harvey-in-store: HARVEY=$HARVEY is a read-only store path; this gate needs a writable scratch copy" ;;
esac
[ ! -e "$HARVEY/.git" ] || err "harvey-is-checkout: HARVEY=$HARVEY has a .git entry; this gate refuses to run against a real Harvey checkout (copy the pinned tree instead)"
case "$RIO_OUT" in
	/nix/store/*) err "rio-out-in-store: RIO_OUT=$RIO_OUT is a read-only store path" ;;
esac
case "$RIO_MODE" in
	rio|control-noG) ;;
	rio-diag) RIO_DIAG=1 ;;
	*) err "RIO_MODE=$RIO_MODE: want rio|control-noG|rio-diag" ;;
esac

mkdir -p "$RIO_OUT"

# --- the run scratch dir and the EXIT trap -----------------------------
# Created before the creds parse so the 0600 password file and the
# composed ISO live under one unique identity from the first write; the
# trap is installed now so every error path below still reaps and wipes.
TMPD=$(mktemp -d "${TMPDIR:-/tmp}/rio-gate.XXXXXX")
R9_RUN_TAG="r9riogate-$$-$(date +%s)"
R9_DT_TAG="r9riogate-dt-$$-$(date +%s)"

# proc_has_run_identity: true when the process's cmdline or environment
# carries this run's tag or scratch path.  The argv0 tag alone is not a
# reliable key (S3: ps showed drawterm's store path, not the tag).
# shellcheck disable=SC2329  # invoked from the trap-reached cleanup chain
proc_has_run_identity() {
	local pid=$1
	[ -r "/proc/$pid/cmdline" ] || return 1
	{
		tr '\0' '\n' <"/proc/$pid/cmdline" 2>/dev/null || true
		tr '\0' '\n' <"/proc/$pid/environ" 2>/dev/null || true
	} | grep -qF -e "$R9_RUN_TAG" -e "$TMPD"
}

# sweep_run_identity: kill any leftover qemu/Xvfb/drawterm of this user
# whose recorded identity names this run.  Recorded pids are reaped
# first by cleanup(); this is the belt-and-braces pass.
# shellcheck disable=SC2329  # invoked from the trap-reached cleanup chain
sweep_run_identity() {
	local pid
	[ -n "$R9_RUN_TAG" ] || return 0
	for pid in $(pgrep -u "$(id -u)" -f 'qemu-system|Xvfb|drawterm' 2>/dev/null || true); do
		[ "$pid" = "$$" ] && continue
		if proc_has_run_identity "$pid"; then
			kill "$pid" 2>/dev/null || true
		fi
	done
}

# shellcheck disable=SC2329  # invoked via trap
cleanup() {
	status=$?
	if [ -n "$RIO_XVFB_PID" ]; then
		kill "$RIO_XVFB_PID" 2>/dev/null || true
		wait "$RIO_XVFB_PID" 2>/dev/null || true
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
	sweep_run_identity || true
	if [ -n "$RIO_DIAG_PROFILE" ] && [ -n "$RIO_DIAG_PROFILE_BAK" ] && [ -f "$RIO_DIAG_PROFILE_BAK" ]; then
		# Put the unmodified staged profile back (rio-diag only); the
		# backup lives under build/x86_64 so a killed run can restore
		# from it on the next diagnostic invocation.
		cp "$RIO_DIAG_PROFILE_BAK" "$RIO_DIAG_PROFILE" 2>/dev/null || true
	fi
	if [ -n "$TMPD" ]; then
		rm -rf "$TMPD"
	fi
	exit "$status"
}
trap cleanup EXIT

# --- step 0a: resolve the locked r9 packages ---------------------------
# The flake app exports RIO_FLAKE (this flake's store source path) and
# DRAWTERM_BIN, but deliberately does NOT interpolate r9's packages into
# the wrapper: r9 116bcc2's kernel derivation reads rust-src's
# Cargo.lock through import-from-derivation, which cannot be built on
# aarch64-darwin.  Resolving here keeps the Mac's `nix eval` of the app
# free of IFD while still binding the exact packages Harvey's own gate
# consumes (every input is locked by flake.lock).  A standalone run may
# export the three package paths directly instead.
if [ -z "${R9_KERNEL_X64_PKG:-}" ] || [ -z "${R9_ISO_BUILDER:-}" ] || [ -z "${R9_NETDEV_EMITTER:-}" ]; then
	[ -n "${RIO_FLAKE:-}" ] || err "R9_KERNEL_X64_PKG/R9_ISO_BUILDER/R9_NETDEV_EMITTER unset and RIO_FLAKE unset — run via the flake app (nix run ...#rio-gate) or export all three"
	command -v nix >/dev/null 2>&1 || err "nix not found on PATH; cannot resolve the locked r9 packages through $RIO_FLAKE"
	_rio_resolve() {
		nix build --no-link --print-out-paths "path:${RIO_FLAKE}#$1"
	}
	R9_KERNEL_X64_PKG=$(_rio_resolve kernel-x64) || err "nix build of the locked kernel-x64 failed"
	R9_ISO_BUILDER="$(_rio_resolve iso-builder)/bin/r9-iso-builder"
	R9_NETDEV_EMITTER="$(_rio_resolve netdev-args)/bin/r9-netdev-args"
fi
export R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_NETDEV_EMITTER

# --- step 0b: the kernel-package + builder contract --------------------

[ -n "${R9_KERNEL_X64_PKG:-}" ] || err "R9_KERNEL_X64_PKG unset — run via the flake app (nix run ...#rio-gate), which resolves the r9 kernel-x64 store path"
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

[ -f "$RIO_EXPECT" ] || err "RIO_EXPECT=$RIO_EXPECT: the rio-gate.expect driver is missing"
command -v xdotool >/dev/null 2>&1 || err "xdotool not found on PATH — the rio mode XTEST feed needs it (the flake app's runtimeInputs carry it)"
command -v Xvfb >/dev/null 2>&1 || err "Xvfb not found on PATH — the rio mode draw venue needs it (the flake app's runtimeInputs carry it)"

if [ "$RIO_DIAG" = "1" ]; then
	# The diagnostic must never fall back to a run without evidence: name
	# the capture tools before anything spawns.
	command -v xwd >/dev/null 2>&1 \
		|| err "RIO-FAIL diag-capture — xwd not found on PATH (the flake app's runtimeInputs carry it)"
	if ! command -v convert >/dev/null 2>&1 && ! command -v magick >/dev/null 2>&1 && ! command -v import >/dev/null 2>&1; then
		err "RIO-FAIL diag-capture — no ImageMagick tool (convert|magick|import) on PATH (the flake app's runtimeInputs carry imagemagick)"
	fi
	command -v xprop >/dev/null 2>&1 \
		|| err "RIO-FAIL diag-capture — xprop not found on PATH (the flake app's runtimeInputs carry it)"
	command -v xwininfo >/dev/null 2>&1 \
		|| err "RIO-FAIL diag-capture — xwininfo not found on PATH (the flake app's runtimeInputs carry it)"
fi

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

echo "== RIO mode:       $RIO_MODE"
echo "== scratch Harvey: $HARVEY"
echo "== RIO output:     $RIO_OUT"
echo "== run scratch:    $TMPD"
echo "== r9 kernel:      $R9_KERNEL"
echo "== iso-builder:    $R9_ISO_BUILDER"
echo "== netdev emitter: $R9_NETDEV_EMITTER"
echo "== drawterm:       $DRAWTERM_BIN"
echo "== netdev argv (slirp): $NETDEV_ARGV"
echo "== dial contract:  user=$RIO_DT_USER cpu=127.0.0.1 auth=tcp!127.0.0.1!5670 (the committed hostfwd pair's loopback side; never widened)"
echo "== shape:          q35 accel=$R9_ACCEL cpu=$R9_CPU"

# --- provenance (KERNDATE; the boot-r9-x64.sh discipline) --------------

if [ -z "${KERNDATE:-}" ]; then
	KERNDATE=$(cd "$HARVEY" && git log -1 --format=%ct 2>/dev/null) || KERNDATE=
fi
if ! [[ "$KERNDATE" =~ ^[0-9]+$ ]]; then
	KERNDATE=$(date +%s)
fi
export KERNDATE

# --- step 0d: the rio-diag login marker (diagnostic only) --------------
# Patch the scratch tree's staged cpu profile immediately before its
# `exec rio' line so the authenticated session leaves a serial-visible
# entry in the kernel-wide #s table before rio starts.  `syscall create'
# performs only the create (no write): devsrv create is OWRITE-only and
# its write expects a decimal fd number, so an ordinary
# `echo login-ok >/srv/<name>' CANNOT work here — the entry NAME is the
# payload, and an unposted entry is listed by read but cannot be read
# (port/src/devsrv.rs create/read/write).  The pid therefore rides the
# second entry's name.  The backup lives under build/x86_64 (outside the
# tree mkimage tars) and is restored on every exit path the script
# controls.
patch_diag_profile() {
	local prof="$HARVEY/build/x86_64/root-tree/usr/glenda/lib/profile"
	local bak="$HARVEY/build/x86_64/r9-diag-profile.orig"
	local patched="$TMPD/profile.patched"
	[ -f "$prof" ] || err "profile-missing: $prof (the cpu skeleton mkimage stages at section 15d)"
	if [ -f "$bak" ] && grep -qF "$RIO_DIAG_MARKER" "$prof"; then
		# A previous diagnostic died before its EXIT trap restored the
		# tree; the persistent backup is the pristine copy.
		cp "$bak" "$prof" || err "profile-restore-failed: $bak -> $prof"
	fi
	cp "$prof" "$bak" || err "profile-backup-failed: $prof -> $bak"
	RIO_DIAG_PROFILE="$prof"
	RIO_DIAG_PROFILE_BAK="$bak"
	local marker_line="syscall create $RIO_DIAG_MARKER 1 0666 || echo r9-diag-marker-failed"
	local pid_line="syscall create ${RIO_DIAG_PID_PREFIX}\$pid 1 0666 || echo r9-diag-pid-failed"
	if awk -v mline="$marker_line" -v pline="$pid_line" '
		BEGIN { inserted = 0 }
		{
			if (!inserted && $0 ~ /^[[:space:]]*exec[[:space:]]+rio([[:space:]]|$)/) {
				print "\t\t\t" mline
				print "\t\t\t" pline
				inserted = 1
			}
			print
		}
		END { exit (inserted ? 0 : 3) }
	' "$prof" >"$patched"; then
		cp "$patched" "$prof" || err "profile-patch-failed: $prof"
		echo "== rio-diag: staged profile patched (marker $RIO_DIAG_MARKER, pid $RIO_DIAG_PID_PREFIX<pid>; backup $bak)"
	else
		# The profile shape moved: stage the markers at the top so the
		# evidence still names the login, then refuse the run — a
		# diagnostic that silently mis-reads an unknown profile is worse
		# than no run.
		{
			printf '%s\n' "syscall create $RIO_DIAG_MARKER 1 0666 || echo r9-diag-marker-failed"
			printf '%s\n' "syscall create ${RIO_DIAG_PID_PREFIX}\$pid 1 0666 || echo r9-diag-pid-failed"
			cat "$prof"
		} >"$patched"
		cp "$patched" "$prof" || true
		err "profile-unparsable: no \`exec rio' line in $prof (markers staged at the top for diagnosis; refusing a run that cannot mark the handoff)"
	fi
}

if [ "$RIO_DIAG" = "1" ]; then
	echo "== step 0d: patching the staged cpu profile with the rio-diag login marker"
	patch_diag_profile
fi

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
BAKE_LOG="$RIO_OUT/bake.log"
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
# The password reaches the 0600 file and the driver only; it is never
# printed, never exported, and never copied into RIO_OUT.
R9_CREDS_FILE="$OUTD/auth-creds.txt"
[ -s "$R9_CREDS_FILE" ] || err "$R9_CREDS_FILE missing/empty after the bake (the cpu bake's host-side creds copy)"
grep -q '^user=' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: no user= line (the parse contract)"
grep -q '^password=' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: no password= line (the parse contract)"
R9_DEMO_USER=$(sed -n 's/^user=//p' "$R9_CREDS_FILE" | tr -d '\r\n')
R9_DEMO_PW=$(sed -n 's/^password=//p' "$R9_CREDS_FILE" | tr -d '\r\n')
[ -n "$R9_DEMO_USER" ] || err "$R9_CREDS_FILE: the user= line parsed empty (empty creds — aborting before any spawn)"
[ -n "$R9_DEMO_PW" ] || err "$R9_CREDS_FILE: the password= line parsed empty (empty creds — aborting before any spawn)"

# 0600 file inside the run scratch dir; only its path leaves this block.
R9_DT_PWFILE="$TMPD/dt-password"
(umask 077; printf '%s' "$R9_DEMO_PW" >"$R9_DT_PWFILE")
chmod 600 "$R9_DT_PWFILE"
[ -s "$R9_DT_PWFILE" ] || err "the 0600 password file is empty — aborting before any spawn"
unset R9_DEMO_PW
export R9_DEMO_USER R9_CREDS_FILE R9_DT_PWFILE
echo "== step 3/5: creds read ok (user=$R9_DEMO_USER; the password is never printed and lives only in the 0600 file)"

# --- step 4: compose the ISO WITH the cpu append -----------------------

echo "== step 4/5: composing the ISO (r9's iso-builder: kernel + bootfs-cpu.paq as /boot/bootfs.paq + the init append)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" --append="init=/amd64/init -c" "$CPU_PAQ" bootfs.paq \
	|| err "iso-builder failed (R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 5: the Xvfb venue (rio mode only) + the expect gate ----------

RIO_SMOKE_LOG="${RIO_SMOKE_LOG:-$RIO_OUT/rio-gate-${RIO_MODE}.log}"
rm -f "$RIO_SMOKE_LOG"

export RIO_MODE RIO_DT_USER RIO_RIO_BUDGET RIO_SESSION_WINDOW RIO_SRV_POLLS RIO_SRV_POLL_WAIT
export RIO_FEED_WAIT RIO_FEED_SETTLE RIO_OUT
if [ "$RIO_DIAG" = "1" ]; then
	export RIO_DIAG RIO_DIAG_MARKER RIO_DIAG_PID_PREFIX
fi

case "$RIO_MODE" in
rio|rio-diag)
	echo "== step 5/5: starting Xvfb (free display via -displayfd, TCP listening off)"
	RIO_DISPLAY_FILE="$TMPD/xvfb-display"
	R9_RUN_ID="$R9_RUN_TAG" Xvfb -displayfd 3 -screen 0 1024x768x24 -nolisten tcp 3>"$RIO_DISPLAY_FILE" &
	RIO_XVFB_PID=$!
	_rio_wait=0
	while [ ! -s "$RIO_DISPLAY_FILE" ]; do
		_rio_wait=$((_rio_wait + 1))
		if [ "$_rio_wait" -ge 100 ]; then
			err "Xvfb did not report a display on -displayfd within 10s (pid $RIO_XVFB_PID)"
		fi
		sleep 0.1
	done
	DISPLAY=":$(cat "$RIO_DISPLAY_FILE")"
	export DISPLAY
	echo "== Xvfb ready: DISPLAY=$DISPLAY (-nolisten tcp)"
	;;
*)
	echo "== step 5/5: control-noG — no X server needed (drawterm -G has no draw device)"
	;;
esac

R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"
export R9_ISO RIO_SMOKE_LOG R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL
export R9_RUN_TAG R9_DT_TAG

echo "== the rio gate (rio-gate.expect); capture: $RIO_SMOKE_LOG"
set +e
expect "$RIO_EXPECT"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "the $RIO_MODE gate FAILED (rc=$RC — the driver named the failure on stderr; capture: $RIO_SMOKE_LOG)"
fi
echo "== rio $RIO_MODE gate PASS — capture retained at $RIO_SMOKE_LOG"
exit 0
