#!/usr/bin/env bash
# nix/gates/soak-r9-net-x64.sh — the NETIP-08 tap-leg soak gate as an
# r9 flake app (plan 01-24, D-07/D-09/D-16/D-19, HARN-08; moved from
# Harvey bd7a698's nix/soak-r9-net-x64.sh and rewired onto r9's own
# supply, the 01-18/01-23 pattern).
#
# The net-smoke pipeline re-keyed onto the tap venue — the L2-truth leg
# (evidence NETIP-08 exists for).  The pipeline, each step loud on
# failure:
#
#   0a. net-mode parse — the --net=<mode> over R9_NET over the slirp
#       default contract, IDENTICAL to nix/boot-r9-x64.sh step 0c (=
#       form only, flag > env > default, venue-validated) — then THE
#       SLIRP REFUSAL (D-11's venue-honesty control, mechanical): mode
#       slirp — including the hermetic DEFAULT — errs immediately with
#       `slirp evidence not accepted', before ANY assembly or boot.
#       This gate's verdict leg is tap or nothing: slirp forgives L2
#       bugs by design, and its captures carry the asserted
#       NOT-L2-TRUTH labels by contract.
#   0b. the flake contract: R9_KERNEL_X64_PKG + R9_ISO_BUILDER (the
#       14-07 contract) + R9_NETDEV_EMITTER/R9_NET_DOCTOR (16-04's)
#       + HOST_ECHO_X64 (this plan's peer) + R9_MKPAQFS (r9's
#       assembler) + accel selection (no silent downgrade, the
#       Phase-13 discipline).
#   0c. DOCTOR PREFLIGHT — "$R9_NET_DOCTOR" --mode tap FIRST: the
#       16-03 ONE implementation (read-only diagnose+print), any FAIL
#       aborts here with its per-issue remediation blocks inline —
#       BEFORE any assembly, boot, or listener (T-16-14: a stale ACL
#       never reaches a tap boot).
#   0d. the seam parse: TAP_GWV4/BLASTPORT from the netaddr seam
#       EXACTLY the way the doctor's seam_init reads it (grep-eval of
#       the tap_gwv4/blastport keys — one seam, host-side too; the
#       peer binds the seam's bridge address, the guest dials it).
#       HARVEY_NETADDR_SEAM names the seam file INSIDE the read-only
#       tree (D-19) — never a path in Harvey's build source.
#   0e. the emitter argv: NETDEV_ARGV via "$R9_NETDEV_EMITTER" --mode
#       tap --pcap <evidence-dir>/netsoak.pcap — the pinned r9 TOML's
#       tap stanza + a filter-dump capture (retained with this run's
#       evidence); exported as R9_NETDEV_ARGV for the expect driver to
#       split (no netdev literal anywhere in this script — D-05).
#   0f. the host echo peer: "$HOST_ECHO_X64" $TAP_GWV4 $BLASTPORT as
#       a background CHILD of this script — waited for its
#       `hostecho: listening' marker before any assembly spends work,
#       torn down by the EXIT trap (dies with the gate; the r9 xtask
#       HostEcho lifecycle precedent, D-12; no daemon, no systemd).
#       The doctor's blastport-free check ran first (0c).
#   1. the net image — `cargo xtask image --arch x86-64 --image net`
#      from the r9 checkout replaces Harvey's in-checkout userland
#      build + bake.  The image engine stages from the read-only tree
#      and writes target/images/x86_64/net.paq (the chain tree + the
#      ip-container binaries + the C helpers + the /lib fixtures incl.
#      16-02's netip-tap.rc — asserted present by the engine's
#      manifest).  R9_SKIP_BUILD is deleted: the assembler's own
#      staleness check is the only cache.
#   2. compose — r9's exported iso-builder (R9_ISO_BUILDER: ONE
#      reproducibility implementation) builds the ISO from the pinned
#      kernel-x64 + the net bake as /boot/bootfs.paq.
#   3. the gate — gates/x86_64/netsoak-r9.expect spawns qemu with the
#      composed ISO on -cdrom (q35) and the SPLIT emitter tap argv,
#      drives `rc /lib/netip-tap.rc 2048 8' (the tiered soak: baseline
#      2048 rounds, burst 8 x 512 — the gate's committed data, D-11),
#      asserts every fixture marker, counts `tcpblast: done'
#      occurrences (exactly 9) and zero `netip: FAIL', and fails
#      wedge-stalls LOUD with the capture tail.  Green-twice, the
#      kernel-fix loop, and the evidence/r9-SHA close-out are 16-08's
#      (the plan split: THAT plan builds and proves the machinery);
#      plan 01-27 owns this move's KVM witness.
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
#   R9_NETDEV_EMITTER  REQUIRED — r9's netdev-args emitter bin (the tap
#                      argv + the pcap object come from its output).
#   R9_NET_DOCTOR      REQUIRED — the r9 net-doctor app path (16-03/
#                      D-14; the tap preflight, ONE implementation
#                      consumed two ways).
#   HOST_ECHO_X64      REQUIRED — the committed host echo peer binary
#                      (the flake's hostEchoX64Pkg build of
#                      nix/gates/host-echo.c; D-12).
#   R9_MKPAQFS         REQUIRED — the host mkpaqfs (image bake).
#   HARVEY_NETADDR_SEAM optional — the netaddr seam file inside the
#                      read-only tree (the flake app exports the tree
#                      path, D-19; the fallback below is the same
#                      tree-relative shape — same bytes in a gate run).
#   R9_GATE_OUT        output dir (default
#                      target/gates/soak-r9-net-x64); the run scratch,
#                      the retained evidence scratch and the capture
#                      live under it.
#   R9_NET              default net mode when no --net flag (the boot
#                      script's contract; slirp — the default — is
#                      REFUSED here; want tap).
#   R9_SOAK_LOG        default $R9_GATE_OUT/netsoak-r9.log.
#   R9_ACCEL           kvm (default) | tcg — no auto, no silent
#                      downgrade.
#   R9_CPU             override the cpu model (default: host under kvm,
#                      qemu64 under tcg).
#   KERNDATE           bake-time timestamp override; the wall clock
#                      otherwise (the tree is a store path with no
#                      .git).
#
# Hygiene: the capture is rm -f'd BEFORE launch; the expect driver
# kills qemu's process group on every exit path; THIS script's EXIT
# trap sweeps any qemu survivor via a bracket-fixed pkill keyed on
# this run's unique R9_RUN_TAG (exported below; the driver adds it to
# qemu's argv as -name) AND kills the host echo peer child — the peer
# dies with the gate on every exit path.  The tag's r9x64soak- prefix
# differs from the sibling gates' tags so the survivor sweeps can
# never cross-kill.  The evidence scratch dir (pcap + the peer's own
# log) is RETAINED past the run — it is this run's capture evidence.
# Every output lives under R9_GATE_OUT or the run scratch dir; nothing
# is written under $HARVEY (T-01-78).
#
# Wrapper provides (the flake app's runtimeInputs): the Rust toolchain
# (cargo xtask image), qemu, expect, procps (the survivor sweep) and
# the text tools.  Standalone use needs the same tools on PATH.
#
# usage: nix run .#boot-r9-net-soak-x64 [-- --net=tap]   (from the r9 repo root)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/soak-r9-net-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and driver contract --------------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#boot-r9-net-soak-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -f "$R9_ROOT/xtask/manifests/amd64/net.toml" ] \
	|| err "$R9_ROOT/xtask/manifests/amd64/net.toml is missing — the net image manifest is part of this checkout"
R9_SOAK_DRIVER="${R9_SOAK_DRIVER:-$R9_ROOT/gates/x86_64/netsoak-r9.expect}"
[ -f "$R9_SOAK_DRIVER" ] || err "R9_SOAK_DRIVER=$R9_SOAK_DRIVER: the net soak driver is missing"

# --- step 0a: net-mode parse (--net flag > R9_NET > slirp default) ------
# IDENTICAL to nix/boot-r9-x64.sh step 0c (the contract's own words) —
# then this gate's OWN refusal: slirp evidence is not accepted, ever,
# including via the default.  The refusal is BEFORE any contract
# spend, assembly, or boot (the venue-honesty control, mechanically
# asserted; a slirp soak must never even assemble).

case "$(uname -s)" in
	Linux)	VENUE=linux ;;
	Darwin)	VENUE=darwin ;;
	*) err "uname $(uname -s): unsupported venue for --net (want linux|darwin)" ;;
esac

# the script's ONLY argument surface: --net=<mode>, the = form alone
NET_FLAG=
for ARG in "$@"; do
	case "$ARG" in
	--net=*)
		[ -z "$NET_FLAG" ] || err "duplicate --net ('$ARG' after '$NET_FLAG') — one net mode per run"
		NET_FLAG="$ARG"
		;;
	*) err "unknown argument '$ARG' — the accepted form is --net=<mode> (usage: nix run .#boot-r9-net-soak-x64 -- --net=tap)" ;;
	esac
done

if [ -n "$NET_FLAG" ]; then
	NET_MODE="${NET_FLAG#--net=}"
else
	NET_MODE="${R9_NET:-slirp}"
fi

case "$NET_MODE" in
	slirp)
		err "slirp evidence not accepted — the NETIP-08 soak verdict is the tap leg's (slirp is not L2 truth by contract and its captures say so); want --net=tap / R9_NET=tap; refusing before any assembly or boot (D-11's venue-honesty control)"
		;;
	tap)
		[ "$VENUE" = linux ] || err "--net=tap on $VENUE: cross-venue (tap is the linux leg; $VENUE's valid net modes: slirp vmnet)"
		;;
	vmnet)
		[ "$VENUE" = darwin ] || err "--net=vmnet on $VENUE: cross-venue (vmnet is darwin's manual leg, HNET-04; $VENUE's valid net modes: slirp tap)"
		;;
	*) err "--net/R9_NET=$NET_MODE: want slirp|tap|vmnet (the valid net modes; this gate runs tap only)" ;;
esac

# --- step 0b: the flake contract + accel selection ----------------------

for v in R9_KERNEL_X64_PKG R9_ISO_BUILDER R9_NETDEV_EMITTER R9_NET_DOCTOR HOST_ECHO_X64 R9_MKPAQFS; do
	[ -n "${!v:-}" ] \
		|| err "$v unset — run via the flake app (nix run .#boot-r9-net-soak-x64), which exports the r9 supply"
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
echo "   shape:         q35 accel=$R9_ACCEL cpu=$R9_CPU net=$NET_MODE (slirp refused upstream of here)"

# --- run scratch + survivor sweep trap (installed BEFORE the peer starts,
#     so teardown covers it from birth) -----------------------------------

mkdir -p "$R9_GATE_OUT"
TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
# the RETAINED evidence scratch: the pcap capture + the peer's own log
# (this run's evidence; 16-08's evidence/r9-SHA close-out and plan
# 01-27's witness read it)
EVIDD=$(mktemp -d "$R9_GATE_OUT/evidence.XXXXXX")
# The per-run unique tag — the driver adds it to qemu's argv (-name)
# and this run's survivor sweep keys on it.  The r9x64soak- prefix
# differs from every sibling gate's tag so the sweeps never
# cross-kill (a foreign r9 qemu — a dev's interactive boot, a parallel
# gate — is never matched; the pattern is bracket-fixed so pkill
# cannot self-match this shell's own argv either).
R9_RUN_TAG="r9x64soak-$$-$(date +%s)"
export R9_RUN_TAG
PEER_PID=
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	status=$?
	# the expect driver kills qemu's process group on every exit path;
	# this trap sweeps any survivor keyed on THIS run's unique tag —
	# never a kernel-name class — AND kills the host echo peer child:
	# the peer dies with the gate on EVERY exit path (D-12's lifecycle
	# rule; the trap is installed before the peer starts).
	pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	if [ -n "${PEER_PID:-}" ]; then
		kill "$PEER_PID" 2>/dev/null || true
	fi
	rm -rf "$TMPD"
	echo "== soak evidence scratch retained: $EVIDD (netsoak.pcap + hostecho.log — this run's evidence)"
	exit "$status"
}
trap cleanup EXIT

# --- step 0c: the doctor preflight (tap) — FIRST, before any assembly/boot --

DOCTOR_RC=0
# shellcheck disable=SC2312  # the captured rc is the point: FAIL => abort
DOCTOR_OUT=$(HARVEY="$HARVEY" "$R9_NET_DOCTOR" --mode tap) || DOCTOR_RC=$?
printf '%s\n' "$DOCTOR_OUT"
if [ "$DOCTOR_RC" -ne 0 ]; then
	err "tap prereqs unmet — the doctor's findings + per-issue remediation blocks are above; full report: nix run .#net-doctor -- --mode tap (the doctor is read-only: the human applies the remediations — the exact 'allow br0' line etc. — then the gate runs; no silent downgrade, ever)"
fi
echo "== doctor preflight PASS (tap mode)"

# --- step 0d: the seam parse (the doctor's seam_init shape) --------------

SEAM_PATH="${HARVEY_NETADDR_SEAM:-$HARVEY/build/x86_64/root-tree/lib/netaddr.rc}"
[ -f "$SEAM_PATH" ] || err "the netaddr seam is missing: $SEAM_PATH — run via the flake app (which exports the tree's seam path) or set HARVEY_NETADDR_SEAM (the peer binds the seam's tap_gwv4, the guest dials it)"
# eval the grep-extracted key assignments (committed repo data — the
# trusted seam, not probe output), exactly the doctor's read
eval "$(grep -E '^(tap_gwv4|blastport)=' "$SEAM_PATH" || true)"
[ -n "${tap_gwv4:-}" ] || err "seam $SEAM_PATH: no tap_gwv4 key (16-02's tap leg expected)"
[ -n "${blastport:-}" ] || err "seam $SEAM_PATH: no blastport key"
TAP_GWV4=$tap_gwv4
BLASTPORT=$blastport
echo "== seam: tap_gwv4=$TAP_GWV4 blastport=$BLASTPORT ($SEAM_PATH)"

# --- step 0e: the emitter argv (tap + the retained pcap capture) --------

# shellcheck disable=SC2312  # the rc rides the ||: an emitter failure is a named contract error (its stderr passed through above)
NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode tap --pcap "$EVIDD/netsoak.pcap") \
	|| err "the netdev emitter failed for mode tap (its diagnostics are above; the argv comes only from the pinned r9 TOML — D-05)"
[ -n "$NETDEV_ARGV" ] || err "the netdev emitter printed no netdev argv for mode tap"
export R9_NETDEV_ARGV="$NETDEV_ARGV"
echo "== netdev argv (emitter, tap): $NETDEV_ARGV"
echo "== pcap capture: $EVIDD/netsoak.pcap"

# --- step 0f: the host echo peer (a gate-owned child; dies with the gate)

"$HOST_ECHO_X64" "$TAP_GWV4" "$BLASTPORT" >"$EVIDD/hostecho.log" 2>&1 &
PEER_PID=$!
# wait for its listening marker before any assembly spends work — a bind
# failure (address gone, port taken the instant after the doctor's
# check) surfaces HERE, loudly, with the peer's own log
PEER_WAIT=0
while ! grep -q 'hostecho: listening' "$EVIDD/hostecho.log" 2>/dev/null; do
	if ! kill -0 "$PEER_PID" 2>/dev/null; then
		err "the host echo peer exited early — its log: $EVIDD/hostecho.log (bind/listen failure? the doctor's blastport-free check passed moments ago)"
	fi
	PEER_WAIT=$((PEER_WAIT + 1))
	[ "$PEER_WAIT" -gt 100 ] && err "the host echo peer did not announce listening within ~10s — log: $EVIDD/hostecho.log"
	sleep 0.1
done
echo "== host echo peer up: pid $PEER_PID at $TAP_GWV4:$BLASTPORT (dies with this gate)"

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

# --- step 3: the expect-driven soak gate -------------------------------

R9_SOAK_LOG="${R9_SOAK_LOG:-$R9_GATE_OUT/netsoak-r9.log}"
R9_QEMU="${R9_QEMU:-qemu-system-x86_64}"

# fresh capture per run BEFORE launch (a pre-seeded stale log is dead
# evidence — it starts empty and only this run's boot writes it; the
# driver rm -f's it again belt-and-braces)
rm -f "$R9_SOAK_LOG"

echo "== step 3/3: the soak gate ($R9_SOAK_DRIVER — baseline 2048 + burst 8x512 via rc /lib/netip-tap.rc)"
echo "== capture: $R9_SOAK_LOG"

export R9_ISO R9_SOAK_LOG R9_QEMU
export R9_MACHINE=q35 R9_CPU R9_ACCEL

set +e
expect "$R9_SOAK_DRIVER"
RC=$?
set -e

if [ "$RC" -ne 0 ]; then
	err "soak gate FAILED (the driver named the stage + printed the capture tail; capture: $R9_SOAK_LOG, peer log: $EVIDD/hostecho.log)"
fi
echo "== soak gate PASS — capture retained at $R9_SOAK_LOG; pcap + peer log at $EVIDD"
exit 0
