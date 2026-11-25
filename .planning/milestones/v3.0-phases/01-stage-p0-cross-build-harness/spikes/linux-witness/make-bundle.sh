#!/usr/bin/env bash
# make-bundle.sh — Plan 06's Mac-side builder for the Linux witness bundle.
#
# Runs on the Mac from the r9 checkout root.  It assembles a staging tree
# under $HOME/r9-spike-bundle/stage/ from the committed spike sources,
# copies Harvey's own 9k10cpu.elf OUT of the read-only Harvey checkout
# (harvey is never written), rebuilds the amd64 a.out probe from the S6
# sources, writes a MANIFEST.sha256 over every file, and packs
# $HOME/r9-spike-bundle/r9-spikes-linux-witness.tar.gz.
#
# The bundle contains only what the owner's x86_64 Linux KVM host needs:
#
#   s1/  flake.nix flake.lock venue.nix hostbin.nix          (rung 1 + rung 2)
#   s3/  flake.nix flake.lock rio-gate.sh rio-gate.expect
#        determinism.sh                                      (rio, control, A12)
#   s5/  boot-q35.sh 9k10cpu.elf                            (Harvey's own kernel)
#   s6/  linux-x86_64.sh q35shell-probe.expect aout.py
#        probe-amd64.aout probe-amd64.sha256.txt             (the KVM swap leg)
#   linux-witness/ run-all.sh README.md
#   MANIFEST.sha256
#
# No work/, evidence/ or FINDINGS file goes in: the owner's run creates the
# evidence, and only trimmed excerpts return to the repository.
#
# Nothing here writes to /Users/neilskinner/code/harvey: the kernel copy is
# read out with cp, and its sha256 must equal the value recorded in
# s5/evidence/kernel-provenance.txt.
#
# Usage: bash .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/make-bundle.sh
set -euo pipefail

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
spikes=$(CDPATH='' cd -- "$here/.." && pwd)
root=$(git -C "$spikes" rev-parse --show-toplevel)

bundle_root=${R9_SPIKE_BUNDLE:-"$HOME/r9-spike-bundle"}
stage="$bundle_root/stage"
check="$bundle_root/check"
tarball="$bundle_root/r9-spikes-linux-witness.tar.gz"
harvey_kernel=${HARVEY_KERNEL:-/Users/neilskinner/code/harvey/amd64/9k10cpu.elf}

die() {
	echo "make-bundle: $*" >&2
	exit 1
}

[ -d "$spikes/s1" ] || die "spikes/s1 missing (run from the r9 checkout)"
[ -f "$harvey_kernel" ] || die "Harvey kernel not found at $harvey_kernel"

# --- the recorded provenance we check the copied artifacts against ------
want_kernel=$(grep -E '^[0-9a-f]{64}$' "$spikes/s5/evidence/kernel-provenance.txt" | head -1)
[ -n "$want_kernel" ] || die "no sha256 found in s5/evidence/kernel-provenance.txt"
want_probe=$(grep -E '^[0-9a-f]{64}  .*probe-amd64\.aout$' "$spikes/s6/evidence/amd64-static.txt" |
	awk '{ print $1; exit }')
[ -n "$want_probe" ] || die "no probe-amd64.aout sha256 found in s6/evidence/amd64-static.txt"

# --- staging tree -------------------------------------------------------
rm -rf -- "$stage"
mkdir -p "$stage/s1" "$stage/s3" "$stage/s5" "$stage/s6" "$stage/linux-witness"

for f in flake.nix flake.lock venue.nix hostbin.nix; do
	cp "$spikes/s1/$f" "$stage/s1/$f"
done

for f in flake.nix flake.lock rio-gate.sh rio-gate.expect determinism.sh; do
	cp "$spikes/s3/$f" "$stage/s3/$f"
done

cp "$spikes/s5/boot-q35.sh" "$stage/s5/boot-q35.sh"
cp "$harvey_kernel" "$stage/s5/9k10cpu.elf"
got_kernel=$(shasum -a 256 "$stage/s5/9k10cpu.elf" | awk '{ print $1 }')
if [ "$got_kernel" != "$want_kernel" ]; then
	die "kernel sha256 mismatch: $stage/s5/9k10cpu.elf is $got_kernel, recorded $want_kernel"
fi
echo "make-bundle: kernel sha256 ok ($got_kernel)"

for f in linux-x86_64.sh q35shell-probe.expect aout.py; do
	cp "$spikes/s6/$f" "$stage/s6/$f"
done

# Rebuild the amd64 probe from the S6 sources (plain rustc from the r9
# devshell; never part of the cargo workspace), check it with aout.py, and
# record the comparison with the sha256 in s6/evidence/amd64-static.txt.
echo "make-bundle: rebuilding the amd64 a.out probe (build-probe.sh amd64)"
( cd "$root" && sh "$spikes/s6/build-probe.sh" amd64 )
python3 -I "$spikes/s6/aout.py" check --arch amd64 "$spikes/s6/work/probe-amd64.aout"
cp "$spikes/s6/work/probe-amd64.aout" "$stage/s6/probe-amd64.aout"
got_probe=$(shasum -a 256 "$stage/s6/probe-amd64.aout" | awk '{ print $1 }')
if [ "$got_probe" = "$want_probe" ]; then
	probe_match=yes
else
	probe_match=no
fi
{
	echo "S6 amd64 probe in this bundle"
	echo "sha256: $got_probe"
	echo "recorded in s6/evidence/amd64-static.txt: $want_probe"
	echo "match: $probe_match"
	echo "aout.py check: AOUT-OK amd64 (see the make-bundle output)"
	echo
	echo "A mismatch is not a bundle failure: the probe is rebuilt from the"
	echo "committed sources with this Mac's locked nightly, and aout.py"
	echo "asserts every rule r9's x86_64 loader applies. The S6 KVM leg"
	echo "validates the actual load and run on the owner's host."
} > "$stage/s6/probe-amd64.sha256.txt"
echo "make-bundle: probe sha256 $got_probe (recorded $want_probe, match=$probe_match)"

cp "$here/run-all.sh" "$stage/linux-witness/run-all.sh"
cp "$here/README.md" "$stage/linux-witness/README.md"
chmod +x "$stage/linux-witness/run-all.sh" "$stage/s3/rio-gate.sh" \
	"$stage/s3/determinism.sh" "$stage/s6/linux-x86_64.sh" \
	"$stage/s6/aout.py"

# --- manifest and tarball ----------------------------------------------
# Paths are relative to the bundle root, so `shasum -a 256 -c MANIFEST.sha256`
# (and `sha256sum -c` on Linux) verifies the extracted tree in place.
manifest_tmp=$(mktemp "$bundle_root/.manifest.XXXXXX")
(
	cd "$stage"
	find . -type f ! -name MANIFEST.sha256 | LC_ALL=C sort |
		while IFS= read -r f; do
			shasum -a 256 "$f"
		done
) >"$manifest_tmp"
mv "$manifest_tmp" "$stage/MANIFEST.sha256"

echo "make-bundle: packing $tarball"
rm -f -- "$tarball"
( cd "$stage" && tar -czf "$tarball" . )

if tar -tzf "$tarball" | grep -Eq '(^|/)(work|evidence)/|FINDINGS'; then
	die "the tarball contains a work/, evidence/ or FINDINGS entry"
fi

# --- verify the manifest from an extraction ----------------------------
rm -rf -- "$check"
mkdir -p "$check"
tar -C "$check" -xzf "$tarball"
( cd "$check" && shasum -a 256 -c MANIFEST.sha256 )

echo "make-bundle: OK"
echo "make-bundle: tarball $tarball"
echo "make-bundle: tarball sha256 $(shasum -a 256 "$tarball" | awk '{ print $1 }')"
echo "make-bundle: manifest files $(wc -l < "$stage/MANIFEST.sha256" | tr -d ' ')"
