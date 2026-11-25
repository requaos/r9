#!/bin/sh
# S6 run-arm64-hvf.sh (D-17, Task 2): prove the Rust-to-a.out swap end to
# end on this Mac.
#
# Three legs, each a fresh re-bake of a SCRATCH copy of Harvey's aarch64
# tree plus one r9 HVF boot driven by virt-probe.expect:
#
#   control  — original echo; type /bin/echo R9-ECHO-CONTROL; the typed
#              echo and the program output must both appear.
#   probe    — arm64/bin/echo replaced by work/probe-arm64.aout; type
#              /bin/echo x; the probe must print R9-RUST-AOUT-PROBE-OK.
#   flipback — original echo restored; the control marker again, and the
#              baked bootfs.paq sha256 must equal the control leg's.
#
# Harvey is READ-ONLY: every bake runs with HARVEY pointing at the scratch
# copy under s6/work/ (gitignored). Nothing is written to
# /Users/neilskinner/code/harvey.
#
# The scratch's mkpaqfs is wrapped by a determinism shim: Harvey's
# aarch64 mkimage.sh has no KERNDATE/mtime pin, and this checkout's
# prebuilt darwin mkpaqfs predates KERNDATE. The shim pins staged mtimes,
# sets the header time and recomputes the trailer SHA1, so the flip-back
# byte-equality proof is meaningful. The real mkpaqfs binary is untouched
# as mkpaqfs.real and is still the byte-format writer.
#
# Env overrides: S6_HARVEY_SRC, S6_SCRATCH, S6_PROBE, S6_RAW, S6_REFRESH=1.
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
root=$(git -C "$here" rev-parse --show-toplevel)
source_harvey=${S6_HARVEY_SRC:-/Users/neilskinner/code/harvey}
scratch=${S6_SCRATCH:-$here/work/harvey-arm64}
probe=${S6_PROBE:-$here/work/probe-arm64.aout}
raw=${S6_RAW:-$here/work}
evidence=$here/evidence

mkdir -p "$raw" "$evidence"

command -v expect >/dev/null 2>&1 || {
	echo "error: expect not found (macOS ships /usr/bin/expect)" >&2
	exit 1
}
command -v nix >/dev/null 2>&1 || {
	echo "error: nix not found (the boot runs inside r9's devshell)" >&2
	exit 1
}

# Harvey provenance: the pin this spike was written against. The mtime
# guard is the plan's acceptance check; the HEAD check here is the cheap
# upfront tripwire.
harvey_pin=$(git -C "$source_harvey" rev-parse HEAD)
want_pin=25d97dc4d4976bfb7411f01970755acf6261c81b
[ "$harvey_pin" = "$want_pin" ] || {
	echo "error: Harvey HEAD $harvey_pin != $want_pin (the spike's pin)" >&2
	exit 1
}
[ -d "$source_harvey/sys/src" ] || {
	echo "error: $source_harvey does not look like the Harvey checkout" >&2
	exit 1
}

[ -f "$probe" ] || {
	echo "error: $probe missing — run: sh $here/build-probe.sh arm64" >&2
	exit 1
}
python3 -I "$here/aout.py" check --arch arm64 "$probe" >/dev/null

cleanup_qemu() {
	pids=$(pgrep -f "qemu-system-aarch64.*$scratch" 2>/dev/null || true)
	if [ -n "$pids" ]; then
		# shellcheck disable=SC2086
		kill -KILL $pids 2>/dev/null || true
	fi
}
trap cleanup_qemu EXIT INT TERM

# --- scratch copy (never the real checkout) --------------------------
if [ "${S6_REFRESH:-0}" = "1" ] || [ ! -d "$scratch/build/aarch64" ]; then
	echo "S6: creating scratch Harvey copy at $scratch"
	rm -rf "$scratch"
	mkdir -p "$scratch/build" "$scratch/amd64/include" "$scratch/.build/bin" \
		"$scratch/sys/src" "$scratch/rc/lib" "$scratch/adm"
	cp -R "$source_harvey/build/aarch64" "$scratch/build/aarch64"
	cp -R "$source_harvey/arm64" "$scratch/arm64"
	cp "$source_harvey/amd64/include/ureg.h" "$scratch/amd64/include/ureg.h"
	cp "$source_harvey/rc/lib/rcmain" "$scratch/rc/lib/rcmain"
	cp -R "$source_harvey/adm/timezone" "$scratch/adm/timezone"
	cp "$source_harvey/.build/bin/mkpaqfs" "$scratch/.build/bin/mkpaqfs.real"
	chmod 755 "$scratch/.build/bin/mkpaqfs.real"
fi

# The scratch-only determinism shim (see the header note). Written every
# run so a stale shim cannot survive an edit to this script.
cat > "$scratch/.build/bin/mkpaqfs" <<'SHIM'
#!/bin/sh
# S6 scratch determinism shim: pin staged mtimes, run Harvey's real
# mkpaqfs, then pin the header time and recompute the trailer SHA1.
set -eu
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
case "$(basename -- "$0")" in
mkpaqfs) ;;
*)
	echo "mkpaqfs-shim: must be invoked as mkpaqfs" >&2
	exit 2
	;;
esac
image=
stage=
prev=
for a in "$@"; do
	if [ "$prev" = "-o" ]; then
		image=$a
	fi
	prev=$a
	stage=$a
done
[ -n "$image" ] || {
	echo "mkpaqfs-shim: no -o IMAGE argument" >&2
	exit 2
}
[ -d "$stage" ] || {
	echo "mkpaqfs-shim: last argument is not a directory: $stage" >&2
	exit 2
}
find "$stage" -type f -exec touch -t 202601010000.00 {} +
find "$stage" -type d -exec touch -t 202601010000.00 {} +
"$here/mkpaqfs.real" "$@"
python3 -I - "$image" <<'PY'
import hashlib
import struct
import sys

path = sys.argv[1]
data = bytearray(open(path, "rb").read())
if len(data) < 40:
    raise SystemExit(f"mkpaqfs-shim: {path}: short image")
data[8:12] = struct.pack(">I", 1795382101)  # bd7a698 commit time
data[-20:] = hashlib.sha1(memoryview(data)[:-20]).digest()
open(path, "wb").write(data)
PY
SHIM
chmod 755 "$scratch/.build/bin/mkpaqfs"

[ ! -e "$source_harvey/.build/bin/mkpaqfs.real" ] || {
	echo "error: shim artifact found in the real Harvey checkout" >&2
	exit 1
}

restore_echo() {
	cp "$source_harvey/arm64/bin/echo" "$scratch/arm64/bin/echo"
	chmod 755 "$scratch/arm64/bin/echo"
}

# bake <leg> — re-bake with Harvey's own mkimage.sh and echo the sha256.
bake() {
	leg=$1
	if ! HARVEY=$scratch sh "$scratch/build/aarch64/mkimage.sh" \
		>"$raw/bake-$leg.log" 2>&1; then
		echo "error: bake $leg failed — tail of $raw/bake-$leg.log:" >&2
		tail -n 20 "$raw/bake-$leg.log" >&2
		exit 1
	fi
	shasum -a 256 "$scratch/build/aarch64/bootfs.paq" | cut -d' ' -f1
}

bootcmd="R9_HARVEY_DIR=$scratch nix develop -c cargo xtask qemu --arch aarch64 --config virtnet --accel hvf"

# run_leg <leg> <guest-command> <marker> <count>
run_leg() {
	leg=$1
	guestcmd=$2
	marker=$3
	count=$4
	log="$raw/arm64-hvf-$leg.log"
	driver="$raw/arm64-hvf-$leg.driver.log"
	: >"$log"
	: >"$driver"
	if (cd "$root" && expect "$here/virt-probe.expect" "$log" "$guestcmd" \
		"$marker" "$count" "$bootcmd") >>"$driver" 2>&1; then
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

echo "S6: leg control"
restore_echo
control_sha=$(bake control)
run_leg control "/bin/echo R9-ECHO-CONTROL" "R9-ECHO-CONTROL" 2 || control=FAIL
echo "S6: control sha256=$control_sha"

echo "S6: leg probe"
cp "$probe" "$scratch/arm64/bin/echo"
chmod 755 "$scratch/arm64/bin/echo"
probe_sha=$(bake probe)
run_leg probe "/bin/echo x" "R9-RUST-AOUT-PROBE-OK" 1 || probe_res=FAIL
echo "S6: probe sha256=$probe_sha"

echo "S6: leg flipback"
restore_echo
flip_sha=$(bake flipback)
run_leg flipback "/bin/echo R9-ECHO-CONTROL" "R9-ECHO-CONTROL" 2 || flipback=FAIL
echo "S6: flipback sha256=$flip_sha"

shaeq=no
if [ "$control_sha" = "$flip_sha" ]; then
	shaeq=yes
fi

# Trim each console log to at most 200 lines of evidence.
for leg in control probe flipback; do
	tail -n 200 "$raw/arm64-hvf-$leg.log" >"$evidence/arm64-hvf-$leg.log"
done

# Final stray-process check (the trap also kills on exit).
if pgrep -f qemu-system-aarch64 >/dev/null 2>&1; then
	echo "error: a qemu-system-aarch64 process is still running" >&2
	exit 1
fi

echo "S6-ARM64 control=$control probe=$probe_res flipback=$flipback sha-equal=$shaeq"

[ "$control" = PASS ] && [ "$probe_res" = PASS ] && [ "$flipback" = PASS ] &&
	[ "$shaeq" = yes ]
