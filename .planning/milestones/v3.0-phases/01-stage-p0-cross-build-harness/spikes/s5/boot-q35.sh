#!/bin/sh
# boot-q35.sh — S5 launch-and-classify harness for Harvey's own 9k kernel.
#
# Boots a copy of Harvey's amd64/9k10cpu.elf under QEMU, captures the serial
# console from launch (capture-before-start), waits for the `Plan 9` banner,
# then watches for the `root is from` prompt line, the recorded 01-04 failure
# signatures, or qemu exiting.  Writes nothing into the Harvey checkout.
#
# The same script runs the Mac TCG legs (S5, Phase 1 pass 1) and the required
# owner KVM witness (Plan 06) against the same kernel bytes.  KVM is never
# downgraded to TCG silently (D-13): an unwritable /dev/kvm is a named error.
#
# Usage:
#   S5_KERNEL=<path to 9k10cpu.elf copy> sh boot-q35.sh
#   S5_ACCEL=kvm S5_MACHINE=q35 S5_NET=virtio S5_LOG=/tmp/kvm.log sh boot-q35.sh
#
# Environment:
#   S5_KERNEL        kernel ELF (required)
#   S5_ACCEL         tcg | kvm                         (default tcg)
#   S5_MACHINE       q35 | pc                          (default q35)
#   S5_NET           virtio | none | default           (default default)
#                    virtio = -netdev user,id=n0 -device virtio-net-pci,netdev=n0
#                    none   = -nic none
#                    default= qemu's machine default NIC
#                    no mode sets hostfwd
#   S5_LOG           serial capture path               (default work/boot-<m>-<n>-<a>.log)
#   S5_BOOT_BUDGET   seconds to wait for the banner    (default 600 tcg / 60 kvm)
#   S5_PROMPT_BUDGET seconds to wait after the banner  (default 600 tcg / 120 kvm)
#   S5_QEMU          qemu-system-x86_64                (default PATH lookup)
#
# Output contract:
#   BANNER=<yes|no>
#   PROMPT=<exact `root is from` line, or none>
#   SIGS=<space-free list separated by ;, or none>
#   NIC=<the #l lines, or none>
#   BANNER_SECONDS=<n>
#   PROMPT_SECONDS=<n>   (from launch, not from banner)
#   S5-RESULT machine=<m> accel=<a> net=<n> banner=<yes|no> prompt=<yes|no> sigs=<list|none>
#
# Exit 0 iff banner and prompt were seen and no signature matched; exit 1
# otherwise (a FAIL with evidence is a complete S5 answer); exit 2 on a
# configuration/runtime error (named `S5-ERROR <reason>: ...`).

set -eu

err() {
	echo "S5-ERROR $*" >&2
	exit 2
}

KERNEL="${S5_KERNEL:-}"
[ -n "$KERNEL" ] || err "kernel-required: set S5_KERNEL to a 9k10cpu.elf copy"
[ -f "$KERNEL" ] || err "kernel-missing: $KERNEL not found"

ACCEL="${S5_ACCEL:-tcg}"
case "$ACCEL" in
tcg) CPU=qemu64 ;;
kvm)
	if [ ! -w /dev/kvm ]; then
		err "kvm-unavailable: S5_ACCEL=kvm requires a writable /dev/kvm; refusing to fall back to TCG (D-13)"
	fi
	CPU=host
	;;
*)
	err "bad-accel: S5_ACCEL must be tcg or kvm, got '$ACCEL'"
	;;
esac

MACHINE="${S5_MACHINE:-q35}"
case "$MACHINE" in
q35 | pc) ;;
*) err "bad-machine: S5_MACHINE must be q35 or pc, got '$MACHINE'" ;;
esac

NET="${S5_NET:-default}"
case "$NET" in
virtio | none | default) ;;
*) err "bad-net: S5_NET must be virtio, none or default, got '$NET'" ;;
esac

S5_DIR=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
LOG="${S5_LOG:-$S5_DIR/work/boot-$MACHINE-$NET-$ACCEL.log}"
mkdir -p "$(dirname -- "$LOG")"

BOOT_BUDGET="${S5_BOOT_BUDGET:-}"
if [ -z "$BOOT_BUDGET" ]; then
	if [ "$ACCEL" = tcg ]; then BOOT_BUDGET=600; else BOOT_BUDGET=60; fi
fi
PROMPT_BUDGET="${S5_PROMPT_BUDGET:-}"
if [ -z "$PROMPT_BUDGET" ]; then
	if [ "$ACCEL" = tcg ]; then PROMPT_BUDGET=600; else PROMPT_BUDGET=120; fi
fi

QEMU_NAME="${S5_QEMU:-qemu-system-x86_64}"
QEMU=$(command -v "$QEMU_NAME" 2>/dev/null || true)
[ -n "$QEMU" ] || err "qemu-missing: '$QEMU_NAME' not on PATH (run inside r9's nix devshell)"

# Build the qemu argv with `set --` (POSIX sh has no arrays).
set -- -M "$MACHINE" -accel "$ACCEL" -cpu "$CPU" -m 512 -kernel "$KERNEL" \
	-display none -no-reboot -serial "file:$LOG"
case "$NET" in
virtio) set -- "$@" -netdev user,id=n0 -device virtio-net-pci,netdev=n0 ;;
none) set -- "$@" -nic none ;;
default) : ;;
esac

QEMU_PID=""
stop_qemu() {
	if [ -n "$QEMU_PID" ]; then
		if kill -0 "$QEMU_PID" 2>/dev/null; then
			kill "$QEMU_PID" 2>/dev/null || true
		fi
		wait "$QEMU_PID" 2>/dev/null || true
		QEMU_PID=""
	fi
}
cleanup() {
	status=$?
	trap - EXIT INT TERM HUP
	stop_qemu
	exit "$status"
}
trap cleanup EXIT INT TERM HUP

# Capture-before-start: truncate, then launch with the chardev file-attached.
: > "$LOG"
printf 'S5-CMD:'
for arg in "$QEMU" "$@"; do
	printf ' %s' "$arg"
done
printf '\n'
"$QEMU" "$@" &
QEMU_PID=$!

banner=no
banner_seconds="$BOOT_BUDGET"
elapsed=0
while [ "$elapsed" -lt "$BOOT_BUDGET" ]; do
	if grep -aq 'Plan 9' "$LOG" 2>/dev/null; then
		banner=yes
		banner_seconds=$elapsed
		break
	fi
	if ! kill -0 "$QEMU_PID" 2>/dev/null; then
		banner_seconds=$elapsed
		break
	fi
	sleep 1
	elapsed=$((elapsed + 1))
done
if [ "$banner" = no ] && grep -aq 'Plan 9' "$LOG" 2>/dev/null; then
	# The banner landed during the last poll or just after qemu exited.
	banner=yes
fi

prompt=no
prompt_seconds="$banner_seconds"
sigs=""
if [ "$banner" = yes ]; then
	celapsed=0
	while [ "$celapsed" -lt "$PROMPT_BUDGET" ]; do
		if grep -aqF 'root is from' "$LOG" 2>/dev/null; then
			prompt=yes
			prompt_seconds=$((banner_seconds + celapsed))
			break
		fi
		for sig in 'bad magic' 'panic' 'suicide: sys: trap'; do
			if grep -aqF "$sig" "$LOG" 2>/dev/null; then
				sigs="${sigs}${sigs:+; }$sig"
			fi
		done
		if [ -n "$sigs" ]; then
			prompt_seconds=$((banner_seconds + celapsed))
			break
		fi
		if ! kill -0 "$QEMU_PID" 2>/dev/null; then
			prompt_seconds=$((banner_seconds + celapsed))
			break
		fi
		sleep 1
		celapsed=$((celapsed + 1))
	done
	if [ "$prompt" = no ] && grep -aqF 'root is from' "$LOG" 2>/dev/null; then
		prompt=yes
	fi
	# Final signature scan over the whole capture (the 01-04 discipline).
	for sig in 'bad magic' 'panic' 'suicide: sys: trap'; do
		case "$sigs" in
		*"$sig"*) : ;;
		*)
			if grep -aqF "$sig" "$LOG" 2>/dev/null; then
				sigs="${sigs}${sigs:+; }$sig"
			fi
			;;
		esac
	done
fi

# Stop and reap before reading the capture, so buffered chardev writes flush.
stop_qemu
sleep 1

prompt_line=$( (grep -aF 'root is from' "$LOG" 2>/dev/null || true) | tail -n 1 | tr -d '\r')
[ -n "$prompt_line" ] || prompt_line=none

nic_lines=$(awk '/^#l[0-9]+:/ { sub(/\r$/, ""); if (n++) printf "; "; printf "%s", $0 } END { if (n) printf "\n" }' "$LOG")
[ -n "$nic_lines" ] || nic_lines=none

[ -n "$sigs" ] || sigs=none

echo "BANNER=$banner"
echo "PROMPT=$prompt_line"
echo "SIGS=$sigs"
echo "NIC=$nic_lines"
echo "BANNER_SECONDS=$banner_seconds"
echo "PROMPT_SECONDS=$prompt_seconds"
echo "S5-RESULT machine=$MACHINE accel=$ACCEL net=$NET banner=$banner prompt=$prompt sigs=$sigs"

[ "$banner" = yes ] && [ "$prompt" = yes ] && [ "$sigs" = none ]
