#!/usr/bin/env bash
# S6 linux-x86_64.sh (D-17, Task 3): the owner-run x86_64 KVM leg.
#
# Runs on the owner's x86_64 Linux host (Plan 06's run-all). It proves the
# same Rust-to-a.out swap on the q35shell CHAIN image: control, probe,
# flip-back, each a fresh re-bake of Harvey's bootfs-chain.paq plus one
# r9 q35shell boot under KVM, driven by q35shell-probe.expect.
#
# Required environment:
#   S6_HARVEY  writable scratch Harvey copy that the S3 step already built
#              (build/x86_64/out/ populated). NEVER a real checkout.
#   S6_R9      scratch r9 clone at 116bcc23c8052a1728992cdbc7b625143260d683
#              (identical to r9 HEAD outside .planning).
#   S6_PROBE   the amd64 probe a.out from the S6 bundle.
#   S6_OUT     output directory for logs and the result line.
#   KERNDATE   optional; defaults to 1795382101 (the bd7a698 commit time).
#
# D-13: the KVM witness is the run, not a convenience. There is no
# software-emulation downgrade path here: an unwritable /dev/kvm is a
# named refusal. Never run as root; never touch a real Harvey checkout.
set -euo pipefail

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

: "${S6_HARVEY:?set S6_HARVEY to the writable scratch Harvey copy}"
: "${S6_R9:?set S6_R9 to the scratch r9 clone}"
: "${S6_PROBE:?set S6_PROBE to the amd64 probe a.out}"
: "${S6_OUT:?set S6_OUT to the output directory}"
KERNDATE=${KERNDATE:-1795382101}
export KERNDATE

die() {
	echo "S6-X86-ERROR: $*" >&2
	exit 1
}

# --- refusals: root, no KVM, a real checkout, wrong r9 -----------------
[ "$(id -u)" -ne 0 ] || die "refusing to run as root; this leg needs no privileges"
[ -w /dev/kvm ] || die "/dev/kvm is not writable — the D-13 witness requires KVM (no software-emulation downgrade)"
[ -d "$S6_HARVEY" ] || die "S6_HARVEY=$S6_HARVEY is not a directory"
[ ! -e "$S6_HARVEY/.git" ] || die "S6_HARVEY=$S6_HARVEY is a real Harvey checkout; use the S3-built scratch copy"
[ -x "$S6_HARVEY/build/x86_64/mkimage.sh" ] || [ -f "$S6_HARVEY/build/x86_64/mkimage.sh" ] ||
	die "$S6_HARVEY/build/x86_64/mkimage.sh missing (not the S3 scratch tree?)"
[ -x "$S6_HARVEY/.build/bin/mkpaqfs" ] ||
	die "$S6_HARVEY/.build/bin/mkpaqfs missing (the S3 build's host mkpaqfs)"
[ -d "$S6_HARVEY/build/x86_64/out" ] ||
	die "$S6_HARVEY/build/x86_64/out missing — run build/x86_64/stage-build.sh first"

want_r9=116bcc23c8052a1728992cdbc7b625143260d683
[ -d "$S6_R9" ] || die "S6_R9=$S6_R9 is not a directory"
got_r9=$(git -C "$S6_R9" rev-parse HEAD 2>/dev/null || true)
[ "$got_r9" = "$want_r9" ] ||
	die "S6_R9 HEAD is '${got_r9:-unknown}', want $want_r9 (the phase's pin)"
[ -z "$(git -C "$S6_R9" status --porcelain)" ] ||
	die "S6_R9 has uncommitted changes; use a clean scratch clone"

command -v nix >/dev/null 2>&1 || die "nix not found on PATH"
command -v expect >/dev/null 2>&1 || die "expect not found on PATH"
command -v python3 >/dev/null 2>&1 || die "python3 not found on PATH"

[ -f "$S6_PROBE" ] || die "S6_PROBE=$S6_PROBE missing"
# aout.py-style sanity: magic 0x8A97, text/entry bounds, exact file length,
# no data/bss/syms. Refuses a wrong-arch or malformed probe before boot.
python3 -I "$here/aout.py" check --arch amd64 "$S6_PROBE" >/dev/null ||
	die "$S6_PROBE is not a valid amd64 a.out for r9's loader"

# The chain contract's echo input (build/x86_64/stage-build.sh harvests it
# to out/; mkimage.sh copies it to amd64/bin/echo).
echo_src="$S6_HARVEY/build/x86_64/out/echo"
echo_orig="$S6_HARVEY/build/x86_64/out/echo.s6-orig"
[ -s "$echo_src" ] || die "$echo_src missing — the S3 stage-build did not populate the chain set"
for f in boot init rc paqfs ls cat ps rm test; do
	[ -s "$S6_HARVEY/build/x86_64/out/$f" ] ||
		die "$S6_HARVEY/build/x86_64/out/$f missing — the S3 stage-build did not populate the chain set"
done

mkdir -p "$S6_OUT"

orig_ready=0
restore_echo() {
	if [ "$orig_ready" = "1" ]; then
		cp "$echo_orig" "$echo_src"
		chmod 755 "$echo_src"
	fi
}

cleanup_qemu() {
	local pids
	pids=$(pgrep -f "qemu-system-x86_64.*$S6_R9" 2>/dev/null || true)
	if [ -n "$pids" ]; then
		# shellcheck disable=SC2086
		kill -KILL $pids 2>/dev/null || true
	fi
}

on_exit() {
	restore_echo
	cleanup_qemu
}
trap on_exit EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# Backup the original chain echo once; every exit path restores it.
if [ ! -f "$echo_orig" ]; then
	cp "$echo_src" "$echo_orig"
	chmod 755 "$echo_orig"
fi
orig_ready=1

bake() {
	local leg=$1
	if ! (cd "$S6_HARVEY" && HARVEY="$S6_HARVEY" KERNDATE="$KERNDATE" \
		sh build/x86_64/mkimage.sh) >"$S6_OUT/bake-$leg.log" 2>&1; then
		echo "error: bake $leg failed — tail of $S6_OUT/bake-$leg.log:" >&2
		tail -n 20 "$S6_OUT/bake-$leg.log" >&2
		return 1
	fi
	sha256sum "$S6_HARVEY/build/x86_64/out/bootfs-chain.paq" | awk '{print $1}'
}

bootcmd="R9_HARVEY_DIR=$(printf '%q' "$S6_HARVEY") nix develop -c cargo xtask qemu --arch x86-64 --config q35shell --accel kvm"

# run_leg <leg> <guest-command> <marker> <count>
run_leg() {
	local leg=$1 guest=$2 marker=$3 count=$4
	local log="$S6_OUT/x86-$leg.log"
	local driver="$S6_OUT/x86-$leg.driver.log"
	: >"$log"
	: >"$driver"
	if (cd "$S6_R9" && expect "$here/q35shell-probe.expect" "$log" \
		"$guest" "$marker" "$count" "$bootcmd") >>"$driver" 2>&1; then
		return 0
	fi
	echo "error: $leg leg failed — driver tail:" >&2
	tail -n 20 "$driver" >&2
	echo "error: $leg console tail:" >&2
	tail -n 20 "$log" >&2
	return 1
}

control=PASS
probe_res=PASS
flipback=PASS

echo "S6-X86: leg control"
cp "$echo_orig" "$echo_src"
control_sha=$(bake control) || exit 1
run_leg control "/bin/echo R9-ECHO-CONTROL" "R9-ECHO-CONTROL" 2 || control=FAIL
echo "S6-X86: control sha256=$control_sha"

echo "S6-X86: leg probe"
cp "$S6_PROBE" "$echo_src"
chmod 755 "$echo_src"
probe_sha=$(bake probe) || exit 1
run_leg probe "/bin/echo x" "R9-RUST-AOUT-PROBE-OK" 1 || probe_res=FAIL
echo "S6-X86: probe sha256=$probe_sha"

echo "S6-X86: leg flipback"
cp "$echo_orig" "$echo_src"
flip_sha=$(bake flipback) || exit 1
run_leg flipback "/bin/echo R9-ECHO-CONTROL" "R9-ECHO-CONTROL" 2 || flipback=FAIL
echo "S6-X86: flipback sha256=$flip_sha"

shaeq=no
if [ "$control_sha" = "$flip_sha" ]; then
	shaeq=yes
fi

result="S6-X86 control=$control probe=$probe_res flipback=$flipback sha-equal=$shaeq"
echo "$result" | tee "$S6_OUT/result.txt"

[ "$control" = PASS ] && [ "$probe_res" = PASS ] && [ "$flipback" = PASS ] &&
	[ "$shaeq" = yes ]
