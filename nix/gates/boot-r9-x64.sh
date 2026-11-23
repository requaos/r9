#!/usr/bin/env bash
# nix/gates/boot-r9-x64.sh — THE one command on x86_64: compose r9's
# pinned kernel — r9's packages.kernel-x64 store path (the
# q35churnshell-config kernel ELF) — with an image assembled by r9's
# own engine, and boot to the interactive rc prompt on the qemu q35
# machine off the composed GRUB ISO.
#
# Moved from Harvey bd7a698's nix/boot-r9-x64.sh and rewired onto r9's
# own supply (plan 01-31, D-07/D-16/D-19, HARN-08; the 01-18/01-23/
# 01-24 pattern):
#
#   1. the image — `cargo xtask image --arch x86-64 --image
#      <chain|cpu>` from the r9 checkout replaces Harvey's in-checkout
#      stage-build + subset-build + mkimage trio.  The image engine
#      stages from the read-only flake tree, runs the per-bake p9sk1
#      authdomain where the manifest carries one, and writes
#      target/images/x86_64/<image>.paq (+ <image>.creds at 0600 for
#      cpu).  Nothing is written into the Harvey tree (HARN-02); the
#      R9_SKIP_BUILD iteration hatch is deleted (the assembler's own
#      staleness check is the only cache).
#   2. compose + launch — r9's exported iso-builder script
#      (R9_ISO_BUILDER: ONE reproducibility implementation, D-05 — the
#      pinned-date xorriso pipeline, strip-all staging, the templated
#      module2 emission) builds the ISO from the kernel + the selected
#      image, then:
#        qemu-system-x86_64 -M q35 -accel kvm -cpu host -smp 1 -m 1024M
#          -cdrom <composed ISO>
#          <the emitter's netdev+device argv for NET_MODE — the
#           R9_NETDEV_EMITTER's output, D-05: no netdev literal here>
#          -nographic
#      NO DTB patch, NO qemu loader-device flag anywhere — the aarch64
#      delivery is deliberately not carried: GRUB delivers bootfs as an
#      mb2 tag-3 module off the same ISO (14-05 D-03).  qemu runs in the
#      FOREGROUND with stdio inherited — the user's terminal IS the
#      console (ctrl-a x to quit).
#
# Accel discipline (Phase 13, carried): the accelerator comes ONLY from
# R9_ACCEL (kvm default) — never a silent downgrade.  A kvm request on
# a KVM-less host FAILS LOUDLY naming R9_ACCEL=tcg as the explicit
# fallback; TCG selects -cpu qemu64 (the hosted-runner shape).
#
# Net discipline (Phase 16, D-01): the net mode comes ONLY from
# --net=<mode> over R9_NET over the slirp default — never a silent
# downgrade.  An unknown mode is a usage error listing slirp|tap|vmnet;
# a cross-venue request (--net=vmnet on linux, --net=tap on darwin) is
# rejected at parse naming the venue's valid modes; the tap leg refuses
# to launch unless the net-doctor's tap checks pass first (D-03 — the
# doctor's per-issue remediations print inline in the failure, never
# auto-applied).  R9_NET_DOCTOR names the moved r9-net-doctor
# (`netDoctorPkg`, plan 01-24) instead of Harvey's copy; the tap
# preconditions themselves are unchanged.
#
# System discipline (Phase 19, 19-09 — INT-05/the OQ-4 settlement,
# carried): --system=cpu|terminal selects WHAT the ONE command boots,
# the same single-accepted-form discipline as --net (duplicate = error;
# the = form only; an unknown mode is a usage error naming exactly
# terminal and cpu — never a silent downgrade, never a fall-through to
# a default venue).  terminal (the DEFAULT, and the behavior with no
# flag at all) keeps the chain path byte-untouched: image chain, the
# no-append ISO compose — every existing gate rides it unchanged.
# cpu is the FULL system: the §15 image (r9's xtask/manifests/amd64/
# cpu.toml — chain + the auth suite + cmd/cpu + aux/listen + the real
# cpurc + tcp17010 + the font payload + the bake-time p9sk1 domain)
# with the bootconf append (init=/amd64/init -c through the 19-01
# iso-builder seam), so one command brings up net + auth + the cpu
# listener together (cpurc's fail-loud ladder: pairing -> ether ->
# keyfs -> authsrv -> factotum -> aux/listen).  Harvey's mkimage baked
# EVERY contract and picked one at its image-selection seam; the r9
# assembler is one invocation per image, so this script assembles ONLY
# the image its system mode selects (the marker image is never
# selected and therefore never assembled).  The frozen aarch64
# boot-r9 app is NOT touched: the full-system mode lands here, on
# boot-r9-x64, only.
#
# Env contract (the flake app apps.x86_64-linux.boot-r9-x64 exports the
# package paths; a standalone run needs the same values):
#   R9_HARVEY_DIR_AMD64 REQUIRED — the read-only amd64 tree carrying
#                       /HARVEY-TREE (the flake's harvey-tree-amd64);
#                       the image engine stages from it and nothing is
#                       written to it.
#   R9_ROOT             the r9 checkout root (default $PWD); the app is
#                       run from the repo root.
#   R9_KERNEL_X64_PKG   REQUIRED — the r9 kernel-x64 package's store
#                       path; the kernel arrives ONLY via this
#                       nix-controlled, read-only store path (presence
#                       validated before the builder runs).
#   R9_ISO_BUILDER      REQUIRED — r9's exported iso-builder script
#                       (<pkg>/bin/r9-iso-builder).
#   R9_NETDEV_EMITTER   REQUIRED — r9's netdev-args emitter bin; the
#                       qemu netdev+device argv tokens come ONLY from
#                       its output for the selected mode (D-05: the
#                       D-04 hostfwd remap and the NixOS tap helper path
#                       are pinned TOML data, never script text).
#   R9_NET_DOCTOR       REQUIRED for --net=tap — the r9-net-doctor app
#                       path (16-03/D-14, read-only); the tap leg runs
#                       it FIRST and aborts on any FAIL with the
#                       per-issue remediation blocks inline (D-03).
#   R9_MKPAQFS          REQUIRED — the host mkpaqfs (image bake).
#   R9_AUTHDOMAIN_GEN   REQUIRED when the selected image's manifest
#                       carries an authdomain entry (cpu); ignored
#                       otherwise.
#   R9_GATE_OUT         output dir (default target/gates/boot-r9-x64);
#                       the compose scratch dir lives under it.
#   R9_ACCEL            kvm (default) | tcg — no auto, no silent
#                       downgrade (see above).
#   R9_CPU              override the cpu model (default: host under kvm,
#                       qemu64 under tcg).
#   R9_NET              default net mode when no --net flag:
#                       slirp|tap|vmnet (default slirp).  The flag wins
#                       over the env; venue-validated at parse.
#   R9_SYSTEM           default system mode when no --system flag:
#                       terminal|cpu (default terminal).  The flag wins
#                       over the env; cpu is the full-system venue.
#   KERNDATE            bake-time timestamp override; the wall clock
#                       otherwise (the tree is a store path with no
#                       .git).
#
# Hygiene: every output lands under R9_GATE_OUT or the run scratch dir;
# nothing is written under $HARVEY (HARN-02, T-01-78).
#
# Wrapper provides (the flake app's runtimeInputs):
# bootR9X64RuntimeInputs — the Rust toolchain (`cargo xtask image`),
# qemu, expect, procps, iproute2 and the text tools.
#
# usage: nix run .#boot-r9-x64 [-- --net=slirp|tap|vmnet] [-- --system=terminal|cpu]   (from the r9 repo root; defaults slirp/terminal — cpu is the full system: net + auth + cpu listener)

set -euo pipefail

HARVEY="${R9_HARVEY_DIR_AMD64:-}"
R9_ROOT="${R9_ROOT:-$PWD}"
R9_GATE_OUT="${R9_GATE_OUT:-$R9_ROOT/target/gates/boot-r9-x64}"

err() { echo "error: $*" >&2; exit 1; }

# --- step 0: the tree, checkout and kernel-package contract ------------

[ -n "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64 unset — run via the flake app (nix run .#boot-r9-x64), which exports the read-only amd64 tree"
[ -d "$HARVEY" ] || err "R9_HARVEY_DIR_AMD64=$HARVEY: not a directory (unrealised store path?)"
[ -f "$HARVEY/HARVEY-TREE" ] \
	|| err "$HARVEY does not look like the flake-built Harvey tree (missing HARVEY-TREE) — R9_HARVEY_DIR_AMD64 must name harvey-tree-amd64"

[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
for m in chain cpu; do
	[ -f "$R9_ROOT/xtask/manifests/amd64/$m.toml" ] \
		|| err "$R9_ROOT/xtask/manifests/amd64/$m.toml is missing — the $m image manifest is part of this checkout"
done

[ -n "${R9_KERNEL_X64_PKG:-}" ] || err "R9_KERNEL_X64_PKG unset — run via the flake app (nix run .#boot-r9-x64), which exports the r9 kernel-x64 store path"
[ -d "$R9_KERNEL_X64_PKG" ] || err "R9_KERNEL_X64_PKG $R9_KERNEL_X64_PKG: not a directory (unrealised store path?)"

R9_KERNEL="$R9_KERNEL_X64_PKG/kernel-x64.elf"
if [ ! -f "$R9_KERNEL" ]; then
	# find fallback: the package's layout is the published contract, but
	# locate the ELF by name rather than failing blind on a layout drift
	R9_KERNEL=$(find "$R9_KERNEL_X64_PKG" -name 'kernel-x64.elf' -type f 2>/dev/null | head -1 || true)
fi
[ -n "$R9_KERNEL" ] && [ -f "$R9_KERNEL" ] || err "kernel-x64.elf not found under $R9_KERNEL_X64_PKG"

[ -n "${R9_ISO_BUILDER:-}" ] || err "R9_ISO_BUILDER unset — run via the flake app (nix run .#boot-r9-x64), which exports r9's iso-builder script path"
# a plain -x accepts DIRECTORIES too (the package root passes it —
# measured); the contract is the executable FILE <pkg>/bin/r9-iso-builder
[ -f "$R9_ISO_BUILDER" ] && [ -x "$R9_ISO_BUILDER" ] \
	|| err "R9_ISO_BUILDER $R9_ISO_BUILDER: not an executable file (want <pkg>/bin/r9-iso-builder — a directory here means the app exported the package root, or the pin predates r9's iso-builder export)"

[ -n "${R9_MKPAQFS:-}" ] || err "R9_MKPAQFS unset — run via the flake app (nix run .#boot-r9-x64), which exports the r9 supply"
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

# --- step 0c: net mode selection (--net flag > R9_NET > slirp) ---------

# the venue names the valid modes (D-01): linux's legs are slirp+tap,
# darwin's are slirp+vmnet (the manual leg, HNET-04) — a cross-venue
# request is a usage error HERE, never an obscure failure inside qemu
case "$(uname -s)" in
	Linux)	VENUE=linux ;;
	Darwin)	VENUE=darwin ;;
	*) err "uname $(uname -s): unsupported venue for --net (want linux|darwin)" ;;
esac

# the script's argument surface: --net=<mode> and --system=<mode>, the
# = forms alone (--net tap / --system cpu are NOT accepted — one
# accepted form per flag, no ambiguity; duplicates are errors)
NET_FLAG=
SYSTEM_FLAG=
for ARG in "$@"; do
	case "$ARG" in
	--net=*)
		[ -z "$NET_FLAG" ] || err "duplicate --net ('$ARG' after '$NET_FLAG') — one net mode per boot"
		NET_FLAG="$ARG"
		;;
	--system=*)
		[ -z "$SYSTEM_FLAG" ] || err "duplicate --system ('$ARG' after '$SYSTEM_FLAG') — one system mode per boot"
		SYSTEM_FLAG="$ARG"
		;;
	*) err "unknown argument '$ARG' — the accepted forms are --net=<mode> and --system=<mode> (usage: nix run .#boot-r9-x64 -- --net=slirp|tap|vmnet --system=terminal|cpu)" ;;
	esac
done

if [ -n "$NET_FLAG" ]; then
	NET_MODE="${NET_FLAG#--net=}"
else
	NET_MODE="${R9_NET:-slirp}"
fi

case "$NET_MODE" in
	slirp)	;;
	tap)
		[ "$VENUE" = linux ] || err "--net=tap on $VENUE: cross-venue (tap is the linux leg; $VENUE's valid net modes: slirp vmnet)"
		;;
	vmnet)
		[ "$VENUE" = darwin ] || err "--net=vmnet on $VENUE: cross-venue (vmnet is darwin's manual leg, HNET-04; $VENUE's valid net modes: slirp tap)"
		;;
	*) err "--net/R9_NET=$NET_MODE: want slirp|tap|vmnet (the valid net modes)" ;;
esac

# --- step 0d: system mode selection (--system flag > R9_SYSTEM > terminal)

# 19-09 (INT-05, the OQ-4 settlement): cpu boots the FULL system off
# the §15 image with the bootconf append; terminal — the default, and
# the no-flag behavior — keeps the chain path byte-untouched.  The
# --net discipline verbatim: the = form only, duplicates already
# rejected at parse, unknown modes are usage errors NAMING THE VALID
# SET — no silent downgrade, no fall-through to a default venue.
if [ -n "$SYSTEM_FLAG" ]; then
	SYSTEM_MODE="${SYSTEM_FLAG#--system=}"
else
	SYSTEM_MODE="${R9_SYSTEM:-terminal}"
fi

case "$SYSTEM_MODE" in
	terminal)	;;
	cpu)	;;
	*) err "--system/R9_SYSTEM=$SYSTEM_MODE: want terminal|cpu (the valid system modes)" ;;
esac

# the netdev argv comes ONLY from r9's emitter (D-05) — the contract
# check mirrors R9_ISO_BUILDER's -f/-x shape
[ -n "${R9_NETDEV_EMITTER:-}" ] || err "R9_NETDEV_EMITTER unset — run via the flake app (nix run .#boot-r9-x64), which exports r9's netdev-args emitter path"
[ -f "$R9_NETDEV_EMITTER" ] && [ -x "$R9_NETDEV_EMITTER" ] \
	|| err "R9_NETDEV_EMITTER $R9_NETDEV_EMITTER: not an executable file (want <pkg>/bin/r9-netdev-args)"

# the tap leg gates on the doctor FIRST (D-03): read-only checks, the
# per-issue remediation blocks printed inline on failure — never
# auto-fixed, never a fall-through to qemu.  The preconditions are
# Harvey's unchanged; the binary is the moved r9-net-doctor.
if [ "$NET_MODE" = tap ]; then
	[ -n "${R9_NET_DOCTOR:-}" ] || err "R9_NET_DOCTOR unset — run via the flake app (nix run .#boot-r9-x64), which exports the r9-net-doctor path (the tap leg's D-03 gate)"
	[ -f "$R9_NET_DOCTOR" ] && [ -x "$R9_NET_DOCTOR" ] \
		|| err "R9_NET_DOCTOR $R9_NET_DOCTOR: not an executable file (want <pkg>/bin/r9-net-doctor)"
	DOCTOR_RC=0
	# shellcheck disable=SC2312  # the captured rc is the point: FAIL => print + abort
	DOCTOR_OUT=$(HARVEY="$HARVEY" "$R9_NET_DOCTOR" --mode tap) || DOCTOR_RC=$?
	if [ "$DOCTOR_RC" -ne 0 ]; then
		printf '%s\n' "$DOCTOR_OUT" >&2
		err "tap prereqs unmet — the doctor's findings + per-issue remediation blocks are above; full report: nix run .#net-doctor -- --mode tap (the doctor is read-only: the human applies the remediations, then retries — no silent downgrade, ever)"
	fi
	printf '%s\n' "$DOCTOR_OUT"
fi

# shellcheck disable=SC2312  # the rc rides the ||: an emitter failure is a named contract error (its stderr passed through above)
NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode "$NET_MODE") \
	|| err "the netdev emitter failed for mode $NET_MODE (its diagnostics are above; the argv comes only from the pinned r9 TOML — D-05)"
[ -n "$NETDEV_ARGV" ] || err "the netdev emitter printed no netdev argv for mode $NET_MODE"

echo "== r9 tree:      $HARVEY"
echo "== r9 root:      $R9_ROOT"
echo "== gate output:  $R9_GATE_OUT"
echo "== r9 kernel package: $R9_KERNEL_X64_PKG"
echo "   kernel:        $R9_KERNEL"
echo "   iso-builder:   $R9_ISO_BUILDER"
echo "   netdev emitter: $R9_NETDEV_EMITTER"
echo "   netdev argv ($NET_MODE): $NETDEV_ARGV"
echo "   shape:         q35 accel=$R9_ACCEL cpu=$R9_CPU"
echo "   system:        $SYSTEM_MODE"
if [ "$SYSTEM_MODE" = cpu ]; then
	echo "   full system: net + auth + cpu listener together (the §15 cpu image + the bootconf append; cpurc's ladder brings the listeners up — D-07's session recipe can dial 127.0.0.1:17010/5670 once LISTEN-17010 prints)"
else
	echo "   terminal:   the chain image, no append (the existing gates' path)"
fi

export KERNDATE="${KERNDATE:-$(date +%s)}"

mkdir -p "$R9_GATE_OUT"

# --- step 1: the image (r9's assembler replaces Harvey's build+bake) ---

if [ "$SYSTEM_MODE" = cpu ]; then
	IMAGE=cpu
	# the §15 cpu manifest carries authdomain entries: the engine runs
	# the host p9sk1 generator and writes target/images/x86_64/cpu.creds
	# (0600).  The chain manifest carries none, so the terminal path
	# does not require the generator.
	[ -n "${R9_AUTHDOMAIN_GEN:-}" ] || err "R9_AUTHDOMAIN_GEN unset — the cpu image's manifest carries a bake-time authdomain generator; run via the flake app (nix run .#boot-r9-x64)"
	[ -f "$R9_AUTHDOMAIN_GEN" ] && [ -x "$R9_AUTHDOMAIN_GEN" ] \
		|| err "R9_AUTHDOMAIN_GEN $R9_AUTHDOMAIN_GEN: not an executable file (want <pkg>/bin/authdomain-gen)"
else
	IMAGE=chain
fi
echo "== step 1/3: assembling image $IMAGE (cargo xtask image --arch x86-64 --image $IMAGE)"
( cd "$R9_ROOT" && cargo xtask image --arch x86-64 --image "$IMAGE" ) \
	|| err "$IMAGE image assembly failed (the engine's diagnostics are above; tree: $HARVEY)"
BOOT_PAQ="$R9_ROOT/target/images/x86_64/$IMAGE.paq"
[ -s "$BOOT_PAQ" ] || err "$BOOT_PAQ missing/empty after assembly (the $IMAGE manifest bakes it)"
echo "== image ok: $BOOT_PAQ"

# --- step 2: compose the ISO -------------------------------------------

TMPD=$(mktemp -d "$R9_GATE_OUT/run.XXXXXX")
# shellcheck disable=SC2329  # invoked via trap below
cleanup() {
	rm -rf "$TMPD"
}
trap cleanup EXIT

if [ "$SYSTEM_MODE" = cpu ]; then
	# the bootconf append (19-01's seam): GRUB's -append carries
	# init=/amd64/init -c, the kernel's bootconf parse seeds it into
	# the ROOT env group, boot(8)'s execinit honors it, init -c runs
	# the real cpurc — the service=cpu identity arrives through the
	# BOOT PROTOCOL, never file content
	echo "== step 2/3: composing the ISO (r9's iso-builder: kernel + cpu.paq as /boot/bootfs.paq + the bootconf append init=/amd64/init -c)"
	"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" --append="init=/amd64/init -c" "$BOOT_PAQ" bootfs.paq \
		|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
else
	echo "== step 2/3: composing the ISO (r9's iso-builder: kernel + chain.paq as /boot/bootfs.paq)"
	"$R9_ISO_BUILDER" "$R9_KERNEL" "$TMPD" "$BOOT_PAQ" bootfs.paq \
		|| err "iso-builder failed (the kernel/builder contract is r9's; R9_KERNEL_X64_PKG/R9_ISO_BUILDER name this run's inputs)"
fi
ISO="$TMPD/boot.iso"
[ -s "$ISO" ] || err "$ISO missing/empty after the iso-builder"

# --- step 3: launch qemu (foreground; stdio is the console) ------------

# D-10 carried: the standard boot speaks Ethernet — the always-on NIC
# rides every composed boot (the aarch64 argv re-keyed to -cdrom; the
# nic-bind post-pass of the smoke gate asserts the driver's bind line).
# 16-04 (D-05): the netdev+device tokens are the emitter's output for
# NET_MODE, word-split unquoted — the emitter prints space-separated
# tokens containing no spaces; NO netdev literal lives in this script.
# shellcheck disable=SC2086  # the word-split IS the argv
set -- qemu-system-x86_64 \
	-M q35 -cpu "$R9_CPU" -smp 1 -m 1024M -accel "$R9_ACCEL" \
	-cdrom "$ISO" \
	$NETDEV_ARGV \
	-nographic

# always print the assembled argv so captured gate logs are
# self-describing (run-r9.sh's discipline); the launching line names
# the selected system so a human terminal shows which venue is up
echo "== step 3/3: launching qemu — system=$SYSTEM_MODE, interactive on stdio (ctrl-a x to quit)"
echo "== qemu argv: $*"

set +e
"$@"
RC=$?
set -e
exit "$RC"
