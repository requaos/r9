#!/usr/bin/env bash
# nix/gates/stress-r9-net-x64.sh — the CR-07 reduced-stress gate as an
# r9 flake app (plan 01-24, D-07/D-09/D-16/D-19, HARN-08; moved from
# Harvey bd7a698's nix/stress-r9-net-x64.sh and rewired onto r9's own
# supply, the 01-18/01-23 pattern): soak-r9-net-x64.sh's composed
# pipeline re-keyed onto the 16-08 deferred record's REDUCED stress
# shape (rounds=1/burstn=8, burst depth 512 — the #66 class fired
# 1-in-5 on the full soak, ~2 burst rounds in; the reduced run puts the
# exposure window at the top of every run) with the #66 toolkit's
# capture: qemu runs with `-d int,guest_errors -D <per-run-file>'
# (pure HOST-side logging — no guest-visible byte changes; a
# debugcon-instrumented kernel is disqualified as evidence, and this
# gate never boots one).
#
# VENUE (the 17-07 venue rule): tap is the preferred venue — #66's
# own, doctor-gated FIRST (any FAIL aborts here with the remediation
# blocks inline, BEFORE any assembly or boot).  --net=slirp is the
# explicitly-labeled RACE-STRESS fallback (the fixture prints the
# RACE-STRESS label on the wire; the driver asserts it; never an
# L2-truth claim — unlike the soak gate, whose verdict leg refuses
# slirp outright, the STRESS verdict is race evidence, venue-honest
# about what it is NOT).
#
# The pipeline, each step loud on failure:
#
#   0a. net-mode parse (--net=tap default | --net=slirp; the tap
#       venue is linux-only, vmnet never — the boot script's venue
#       table).
#   0b. the flake contract: R9_KERNEL_X64_PKG + R9_ISO_BUILDER +
#       R9_NETDEV_EMITTER/R9_NET_DOCTOR + HOST_ECHO_X64 + R9_MKPAQFS
#       (r9's assembler) + accel selection (no silent downgrade, the
#       Phase-13 discipline).
#   0c. DOCTOR PREFLIGHT (tap mode only): "$R9_NET_DOCTOR" --mode
#       tap; any FAIL aborts with the remediation inline (the
#       operator remediates or reruns with --net=slirp — the labeled
#       fallback — never a silent downgrade).
#   0d. the seam parse (tap_gwv4/blastport; the slirp peer binds the
#       host loopback — slirp's 10.0.2.2 maps there by construction).
#       HARVEY_NETADDR_SEAM names the seam file INSIDE the read-only
#       tree (D-19).
#   0e. the emitter argv: tap carries a pcap capture in the evidence
#       dir; slirp is capture-free (the RACE-STRESS label makes no
#       L2 claim, so there is no pcap evidence to retain).
#   0f. the host echo peer (a background CHILD of this script, died
#       by the EXIT trap): tap binds the seam's bridge address,
#       slirp binds 127.0.0.1 (the guest's 10.0.2.2 dial lands
#       there).
#   1. the net image — `cargo xtask image --arch x86-64 --image net`
#      from the r9 checkout replaces Harvey's in-checkout build +
#      bake; netstress.rc rides the tree's root-tree skeleton.  The
#      engine writes target/images/x86_64/net.paq.  R9_SKIP_BUILD is
#      deleted (the assembler's staleness check is the only cache).
#   2. compose — r9's exported iso-builder: the pinned kernel-x64 +
#      the net bake as /boot/bootfs.paq.
#   3. the gate — gates/x86_64/stress-r9.expect spawns qemu with the
#      wrapper's -d int,guest_errors -D tokens (R9_QEMU, split by the
#      driver), drives `rc /lib/netstress.rc <venue> 1 8 512', the
#      venue label, the burst barrier, the tcpblast: done COUNT
#      (exactly 9), zero FAIL, and the #66 RESET signature as a
#      FAST-FAIL arm (boot-chain reappearance post-prompt names the
#      triple-fault class immediately; the -D file holds the
#      exception cascade).
#
# Env contract:
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree carrying
#                      /HARVEY-TREE (the flake's harvey-tree-amd64);
#                      the netaddr seam is read from it (or from
#                      HARVEY_NETADDR_SEAM), nothing is written to it.
#   R9_ROOT            the r9 checkout root (default $PWD).
#   R9_KERNEL_X64_PKG  REQUIRED — the r9 kernel-x64 package's store
#                      path; the flake app exports it.
#   R9_ISO_BUILDER     REQUIRED — r9's exported iso-builder BINARY.
#   R9_NETDEV_EMITTER  REQUIRED — r9's netdev-args emitter bin.
#   R9_NET_DOCTOR      REQUIRED — the r9 net-doctor app path (the tap
#                      preflight; unused in slirp mode).
#   HOST_ECHO_X64      REQUIRED — the committed host echo peer binary.
#   R9_MKPAQFS         REQUIRED — the host mkpaqfs (image bake).
#   HARVEY_NETADDR_SEAM optional — the netaddr seam file inside the
#                      read-only tree (the flake app exports the tree
#                      path, D-19).
#   R9_GATE_OUT        output dir (default
#                      target/gates/stress-r9-net-x64); the run
#                      scratch, the retained evidence scratch and the
#                      capture live under it.
#   R9_NET             default net mode when no --net flag (tap).
#   R9_STRESS_LOG      default $R9_GATE_OUT/stress-r9.log.
#   R9_ACCEL           kvm (default) | tcg — no auto, no silent
#                      downgrade.
#   R9_CPU             override the cpu model (default: host under kvm,
#                      qemu64 under tcg).
#   KERNDATE           bake-time timestamp override; the wall clock
#                      otherwise (the tree is a store path with no
#                      .git).
#
# Hygiene: the capture + the -D cascade file are rm -f'd fresh before
# launch; the expect driver kills qemu's process group on every exit
# path; THIS script's EXIT trap sweeps any qemu survivor via a
# bracket-fixed pkill keyed on this run's unique R9_RUN_TAG AND kills
# the peer child.  The tag's r9x64stress- prefix differs from the
# sibling gates' tags so the sweeps never cross-kill.  The evidence
# scratch (pcap on tap + the peer's log + the qemu -D cascade) is
# RETAINED past the run — it is this run's capture evidence.  Every
# output lives under R9_GATE_OUT or the run scratch dir; nothing is
# written under $HARVEY (T-01-78).
#
# Wrapper provides (the flake app's runtimeInputs): the Rust toolchain
# (cargo xtask image), qemu, expect, procps (the survivor sweep) and
# the text tools.  Standalone use needs the same tools on PATH.
#
# usage: nix run .#boot-r9-stress-x64 [-- --net=tap|--net=slirp]   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/stress-r9-net-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#boot-r9-stress-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/net.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/net.toml is missing — the net image manifest is part of this checkout"
R9_STRESS_DRIVER="${R9_STRESS_DRIVER:-$R9_ROOT/gates/x86_64/stress-r9.expect}"
[ -f "$R9_STRESS_DRIVER" ] || err "R9_STRESS_DRIVER=$R9_STRESS_DRIVER: the net stress driver is missing"

# --- step 0a: net-mode parse (tap default; slirp = the labeled arm) ------

case "$(uname -s)" in
	Linux)	VENUE=linux ;;
	Darwin)	VENUE=darwin ;;
	*) err "uname $(uname -s): unsupported venue for --net (want linux|darwin)" ;;
esac

NET_FLAG=
for ARG in "$@"; do
	case "$ARG" in
	--net=*)
		[ -z "$NET_FLAG" ] || err "duplicate --net ('$ARG' after '$NET_FLAG') — one net mode per run"
		NET_FLAG="$ARG"
		;;
	*) err "unknown argument '$ARG' — the accepted form is --net=<mode> (usage: nix run .#boot-r9-stress-x64 -- --net=tap|--net=slirp)" ;;
	esac
done

if [ -n "$NET_FLAG" ]; then
	NET_MODE="${NET_FLAG#--net=}"
else
	NET_MODE="${R9_NET:-tap}"
fi

case "$NET_MODE" in
	tap)
		[ "$VENUE" = linux ] || err "--net=tap on $VENUE: cross-venue (tap is the linux leg; $VENUE's valid net modes: slirp vmnet)"
		;;
	slirp)
		# the RACE-STRESS fallback: race evidence only, never an
		# L2-truth claim (the fixture's on-wire label carries it)
		;;
	vmnet)
		err "--net=vmnet: darwin's manual leg is not a stress venue (want tap|slirp)"
		;;
	*) err "--net/R9_NET=$NET_MODE: want tap|slirp (tap is the preferred stress venue, doctor-gated; slirp is the RACE-STRESS-labeled fallback)" ;;
esac

# --- step 0b: the flake contract + accel selection ----------------------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_NETDEV_EMITTER R9_NET_DOCTOR HOST_ECHO_X64 R9_MKPAQFS; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#boot-r9-stress-x64), which exports the r9 supply"
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
[ -f "$R9_NET_DOCTOR" ] && [ -x "$R9_NET_DOCTOR" ] \
	|| err "R9_NET_DOCTOR $R9_NET_DOCTOR: not an executable file (want <pkg>/bin/r9-net-doctor)"
[ -f "$HOST_ECHO_X64" ] && [ -x "$HOST_ECHO_X64" ] \
	|| err "HOST_ECHO_X64 $HOST_ECHO_X64: not an executable file (want <hostEchoX64Pkg>/bin/host-echo)"
[ -f "$R9_MKPAQFS" ] && [ -x "$R9_MKPAQFS" ] \
	|| err "R9_MKPAQFS $R9_MKPAQFS: not an executable file (want <pkg>/bin/mkpaqfs)"

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

echo "== r9 tree:        $HARVEY"
echo "== r9 root:        $R9_ROOT"
echo "== gate output:    $R9_GATE_OUT"
echo "== r9 kernel package: $R9_KERNEL_X64_PKG"
echo "   kernel:        $R9_KERNEL"
echo "   iso-builder:   $R9_ISO_BUILDER"
echo "   netdev emitter: $R9_NETDEV_EMITTER"
echo "   net doctor:    $R9_NET_DOCTOR"
echo "   host peer:     $HOST_ECHO_X64"
echo "   shape:         q35 accel=$R9_ACCEL cpu=$R9_CPU net=$NET_MODE (slirp runs carry the RACE-STRESS label on the wire)"

# --- run scratch + survivor sweep trap (installed BEFORE the peer starts)

mkdir -p "$R9_GATE_OUT"
TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
# the RETAINED evidence scratch: the qemu -D cascade (every run), the
# pcap (tap runs), the peer's own log
EVIDD=$(mktemp -d "$R9_GATE_OUT/evidence.XXXXXX")
# The per-run unique tag — the driver adds it to qemu's argv (-name)
# and this run's survivor sweep keys on it.  The r9x64stress- prefix
# differs from every sibling gate's tag so the sweeps never
# cross-kill (bracket-fixed so pkill cannot self-match this shell).
R9_RUN_TAG="r9x64stress-$$-$(date +%s)"
export R9_RUN_TAG
QEMU_DBG="$EVIDD/qemu-int-$R9_RUN_TAG.log"
PEER_PID=
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	# the expect driver kills qemu's process group on every exit path;
	# this trap sweeps any survivor keyed on THIS run's unique tag AND
	# kills the host echo peer child on EVERY exit path.
	pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	if [ -n "${PEER_PID:-}" ]; then
		kill "$PEER_PID" 2>/dev/null || true
	fi
	rm -rf "$TMPD"
	echo "== stress evidence scratch retained: $EVIDD (qemu -d cascade: qemu-int-*.log$( [ "$NET_MODE" = tap ] && echo ' + netstress.pcap' ) + hostecho.log)"
	exit "$status"
}
trap cleanup EXIT

# --- step 0c: the doctor preflight (TAP mode only — the preferred venue) -

if [ "$NET_MODE" = tap ]; then
	DOCTOR_RC=0
	# shellcheck disable=SC2312  # the captured rc is the point: FAIL => abort
	DOCTOR_OUT=$(HARVEY="$HARVEY" "$R9_NET_DOCTOR" --mode tap) || DOCTOR_RC=$?
	printf '%s\n' "$DOCTOR_OUT"
	if [ "$DOCTOR_RC" -ne 0 ]; then
		err "tap prereqs unmet — the doctor's findings + per-issue remediation blocks are above; remediate and rerun, or use --net=slirp (the RACE-STRESS-labeled fallback — race evidence only, never L2 truth); no silent downgrade, ever"
	fi
	echo "== doctor preflight PASS (tap mode — the preferred stress venue, #66's own)"
fi

# --- step 0d: the seam parse (the doctor's seam_init shape) --------------

SEAM_PATH="${HARVEY_NETADDR_SEAM:-$HARVEY/build/x86_64/root-tree/lib/netaddr.rc}"
[ -f "$SEAM_PATH" ] || err "the netaddr seam is missing: $SEAM_PATH — run via the flake app (which exports the tree's seam path) or set HARVEY_NETADDR_SEAM"
eval "$(grep -E '^(tap_gwv4|blastport)=' "$SEAM_PATH" || true)"
[ -n "${tap_gwv4:-}" ] || err "seam $SEAM_PATH: no tap_gwv4 key (the tap leg expected)"
[ -n "${blastport:-}" ] || err "seam $SEAM_PATH: no blastport key"
TAP_GWV4=$tap_gwv4
BLASTPORT=$blastport

# the peer bind address per venue: tap binds the bridge (the guest
# dials the seam's tap_gwv4); slirp binds the host loopback (slirp's
# 10.0.2.2 host alias maps there by construction — the guest's dial
# lands on it)
if [ "$NET_MODE" = tap ]; then
	PEER_ADDR=$TAP_GWV4
else
	PEER_ADDR=127.0.0.1
fi
echo "== seam: tap_gwv4=$TAP_GWV4 blastport=$BLASTPORT ($SEAM_PATH); peer binds $PEER_ADDR (venue $NET_MODE)"

# --- step 0e: the emitter argv (tap carries the pcap; slirp is bare) -----

if [ "$NET_MODE" = tap ]; then
	# shellcheck disable=SC2312  # the rc rides the ||: an emitter failure is a named contract error
	NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode tap --pcap "$EVIDD/netstress.pcap") \
		|| err "the netdev emitter failed for mode tap (its diagnostics are above; the argv comes only from the pinned r9 TOML — D-05)"
	echo "== pcap capture: $EVIDD/netstress.pcap"
else
	# shellcheck disable=SC2312
	NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode slirp) \
		|| err "the netdev emitter failed for mode slirp (its diagnostics are above)"
fi
[ -n "$NETDEV_ARGV" ] || err "the netdev emitter printed no netdev argv for mode $NET_MODE"
export R9_NETDEV_ARGV="$NETDEV_ARGV"
echo "== netdev argv (emitter, $NET_MODE): $NETDEV_ARGV"

# --- step 0f: the host echo peer (a gate-owned child; dies with the gate)

"$HOST_ECHO_X64" "$PEER_ADDR" "$BLASTPORT" >"$EVIDD/hostecho.log" 2>&1 &
PEER_PID=$!
# wait for its listening marker before any assembly spends work — a bind
# failure (address gone, port taken) surfaces HERE, loudly, with the
# peer's own log
PEER_WAIT=0
while ! grep -q 'hostecho: listening' "$EVIDD/hostecho.log" 2>/dev/null; do
	if ! kill -0 "$PEER_PID" 2>/dev/null; then
		err "the host echo peer exited early — its log: $EVIDD/hostecho.log (bind/listen failure at $PEER_ADDR:$BLASTPORT?)"
	fi
	PEER_WAIT=$((PEER_WAIT + 1))
	[ "$PEER_WAIT" -gt 100 ] && err "the host echo peer did not announce listening within ~10s — log: $EVIDD/hostecho.log"
	sleep 0.1
done
echo "== host echo peer up: pid $PEER_PID at $PEER_ADDR:$BLASTPORT (dies with this gate)"

# --- provenance (KERNDATE; the wall-clock seam) -------------------------

export KERNDATE="${KERNDATE:-$(date +%s)}"

# --- step 1: the net image (r9's assembler replaces Harvey's bake) ------

echo "== step 1/3: assembling image net (cargo xtask image --arch x86-64 --image net)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image net ) \
	|| err "net image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
NET_PAQ="$R9_ROOT/target/images/x86_64/net.paq"
[ -s "$NET_PAQ" ] || err "$NET_PAQ missing/empty after assembly (the net manifest bakes it)"
echo "== image ok: $NET_PAQ"

# --- step 2: compose the ISO ---------------------------------------------

echo "== step 2/3: composing the ISO (r9's iso-builder: kernel + net.paq as /boot/bootfs.paq)"
"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" "$NET_PAQ" bootfs.paq \
	|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
R9_ISO="$TMPD/boot.iso"
[ -s "$R9_ISO" ] || err "$R9_ISO missing/empty after the iso-builder"

# --- step 3: the expect-driven stress gate --------------------------------

R9_STRESS_LOG="${R9_STRESS_LOG:-$R9_GATE_OUT/stress-r9.log}"

# THE #66 CAPTURE (the 16-08 toolkit): -d int,guest_errors with the
# cascade in a per-run -D file — pure host-side logging; the guest
# sees nothing (the observer-effect lesson honored: no guest-visible
# instrumentation anywhere on this path).  The driver splits R9_QEMU
# into tokens (its own contract).  R9_QEMU_APPEND is the diagnosis
# hatch (17-07 Task 1's root-cause loop): extra HOST-side -d flags
# (e.g. cpu_reset for the reset event's own record) appended to the
# committed set — same doctrine, escalated capture, still zero
# guest-visible change.
export R9_QEMU="qemu-system-x86_64 -d int,guest_errors -D $QEMU_DBG${R9_QEMU_APPEND:+ $R9_QEMU_APPEND}"
export R9_STRESS_VENUE="$NET_MODE"
export R9_ISO R9_STRESS_LOG
# R9_MACHINE default q35; the diagnosis hatch (17-07 Task 1): an
# explicit R9_MACHINE override rides through (e.g. `q35,kernel-irqchip=off'
# — userspace-APIC delivery makes exception injection visible to -d int
# while keeping KVM pacing; still zero guest-visible kernel change).
export R9_MACHINE="${R9_MACHINE:-q35}" R9_CPU R9_ACCEL

# fresh capture + fresh cascade per run BEFORE launch (stale files
# are dead evidence)
rm -f "$R9_STRESS_LOG" "$QEMU_DBG"

echo "== step 3/3: the stress gate ($R9_STRESS_DRIVER — venue $NET_MODE, baseline 1 + burst 8x512 via rc /lib/netstress.rc, -d int,guest_errors cascade -> $QEMU_DBG)"
echo "== capture: $R9_STRESS_LOG"

set +e
expect "$R9_STRESS_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "stress gate FAILED (the driver named the stage + printed the capture tail; capture: $R9_STRESS_LOG, qemu cascade: $QEMU_DBG, peer log: $EVIDD/hostecho.log)"
fi
echo "== stress gate PASS — capture retained at $R9_STRESS_LOG; cascade + peer log at $EVIDD"
exit 0
