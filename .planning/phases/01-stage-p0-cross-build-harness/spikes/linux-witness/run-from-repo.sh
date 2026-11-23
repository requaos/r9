#!/usr/bin/env bash
# run-from-repo.sh — Plan 06's pull-only Linux witness entry point.
#
# The Mac cannot reach the owner's x86_64 Linux KVM host and nothing is copied
# between them (no scp, no shared filesystem).  Everything the host needs is
# committed to r9, so the host gets it from GitHub:
#
#   git clone git@github.com:requaos/r9.git      # or: git -C <r9> pull --ff-only
#   cd <r9> && bash .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/run-from-repo.sh
#
# The script rebuilds the S6 amd64 probe from the committed sources, verifies
# the committed S5 kernel against s5/evidence/kernel-provenance.txt, stages a
# complete bundle with the same layout make-bundle.sh used to scp, writes and
# verifies MANIFEST.sha256 from a fresh extraction, and then (unless --check)
# runs the existing linux-witness/run-all.sh driver from the staging root.
#
# Contracts, all enforced below:
#   * refuses root (id -u == 0) and never calls sudo;
#   * reads from the r9 checkout and writes only under
#     ${R9_WITNESS_STAGE:-$HOME/r9-spike-work/witness-bundle};
#   * aborts with a named error if the committed kernel does not match
#     s5/evidence/kernel-provenance.txt;
#   * a probe sha256 mismatch against s6/evidence/amd64-static.txt is
#     recorded, never fatal: a different rustc build host is expected, and
#     the structural aout.py check is the gate;
#   * default mode passes R9_SPIKES_ONLY through to run-all.sh, tees to
#     run-all.log, and writes WITNESS-RETURN.md beside out/.
#
# --check stops after staging and manifest verification and runs no
# Linux-only leg.  It is the mode that can be exercised on the Mac.
set -uo pipefail

die() {
	echo "R9-WITNESS-ERROR $*" >&2
	exit 1
}

note() {
	echo "== $*"
}

usage() {
	cat <<'EOF'
usage: bash run-from-repo.sh [--check]

  (no argument)  rebuild the probe, stage the bundle, verify the manifest,
                 then run linux-witness/run-all.sh from the staging root
  --check        stop after staging and manifest verification; run no
                 Linux-only leg (safe on the Mac)

Environment:
  R9_WITNESS_STAGE   staging root (default $HOME/r9-spike-work/witness-bundle)
  R9_SPIKE_WORK      scratch root used by run-all.sh (default $HOME/r9-spike-work)
  R9_SPIKES_ONLY     subset for run-all.sh (default: s1 s5 s3 s6)
  S5_KERNEL_EXTRA    optional second s5 kernel for run-all.sh
EOF
}

# --- root refusal and argument parsing ---------------------------------
[ "$(id -u)" -ne 0 ] ||
	die "running-as-root: this script refuses root and needs no privileges; run it as your user"

mode=run
while [ "$#" -gt 0 ]; do
	case "$1" in
	--check) mode=check ;;
	-h | --help)
		usage
		exit 0
		;;
	*) die "bad-argument '$1': usage: bash run-from-repo.sh [--check]" ;;
	esac
	shift
done

# --- locate the r9 checkout from any cwd inside it ---------------------
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P) ||
	die "script-dir-unavailable: cannot resolve the directory holding $0"
repo_root=$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null) ||
	die "not-a-git-checkout: $script_dir is not inside an r9 git checkout"
spikes="$repo_root/.planning/phases/01-stage-p0-cross-build-harness/spikes"
spikes_rel=.planning/phases/01-stage-p0-cross-build-harness/spikes
[ -d "$spikes/s1" ] || die "wrong-checkout: $spikes/s1 is missing (is this r9?)"

stage_root=${R9_WITNESS_STAGE:-"$HOME/r9-spike-work/witness-bundle"}
case "$stage_root" in
	/*) ;;
	*) die "bad-stage: R9_WITNESS_STAGE must be an absolute path, got '$stage_root'" ;;
esac
case "$stage_root" in
	/nix/store/*) die "stage-in-store: R9_WITNESS_STAGE=$stage_root is a read-only store path" ;;
esac

sha256_of() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | awk '{ print $1 }'
	elif command -v shasum >/dev/null 2>&1; then
		shasum -a 256 "$1" | awk '{ print $1 }'
	else
		die "no-sha256-tool: need sha256sum or shasum"
	fi
}

verify_manifest() {
	local dir=$1
	if command -v sha256sum >/dev/null 2>&1; then
		(cd "$dir" && sha256sum -c MANIFEST.sha256)
	elif command -v shasum >/dev/null 2>&1; then
		(cd "$dir" && shasum -a 256 -c MANIFEST.sha256)
	else
		die "no-sha256-tool: need sha256sum or shasum"
	fi
}

emit_file() {
	local path=$1
	if [ -f "$path" ]; then
		printf '~~~\n'
		cat "$path"
		printf '\n~~~\n\n'
	else
		printf '(missing: %s)\n\n' "$path"
	fi
}

# --- the committed artifacts must match their recorded bytes -----------
provenance="$spikes/s5/evidence/kernel-provenance.txt"
kernel="$spikes/s5/9k10cpu.elf"
[ -f "$provenance" ] || die "provenance-missing: $provenance is not in this checkout"
[ -f "$kernel" ] || die "kernel-missing: $kernel is not in this checkout; pull the commit that adds it"
want_kernel=$(grep -E '^[0-9a-f]{64}$' "$provenance" | head -1)
[ -n "$want_kernel" ] || die "provenance-unreadable: no sha256 line in $provenance"
want_kernel_size=$(grep -E '^[[:space:]]*[0-9]+ bytes' "$provenance" | head -1 | awk '{ print $1 }')
kernel_sha=$(sha256_of "$kernel")
kernel_size=$(wc -c <"$kernel" | tr -d ' ')
[ "$kernel_sha" = "$want_kernel" ] ||
	die "kernel-sha-mismatch: $kernel is $kernel_sha, recorded $want_kernel"
if [ -n "$want_kernel_size" ] && [ "$kernel_size" != "$want_kernel_size" ]; then
	die "kernel-size-mismatch: $kernel is $kernel_size bytes, recorded $want_kernel_size"
fi
note "kernel sha256 ok ($kernel_sha, $kernel_size bytes)"

# --- rebuild the amd64 probe from committed sources --------------------
probe_dir="$spikes/s6"
probe_evidence="$probe_dir/evidence/amd64-static.txt"
[ -f "$probe_evidence" ] || die "probe-evidence-missing: $probe_evidence"
want_probe=$(grep -E '^[0-9a-f]{64}  .*probe-amd64\.aout$' "$probe_evidence" |
	awk '{ print $1; exit }')
[ -n "$want_probe" ] || die "probe-evidence-unreadable: no probe-amd64.aout sha256 in $probe_evidence"
command -v nix >/dev/null 2>&1 ||
	die "nix-missing: nix with flakes is required (see linux-witness/README.md)"

note "rebuilding the amd64 a.out probe from committed sources (build-probe.sh amd64)"
(cd "$repo_root" && nix develop -c sh "$probe_dir/build-probe.sh" amd64) ||
	die "probe-build-failed: build-probe.sh amd64 failed"
probe="$probe_dir/work/probe-amd64.aout"
[ -f "$probe" ] || die "probe-missing: $probe was not produced"
# The host needs only nix: the check runs in r9's devshell, which supplies
# python3, exactly like the build's wrap/check step did.
(cd "$repo_root" && nix develop -c python3 -I "$probe_dir/aout.py" check \
	--arch amd64 "$probe") ||
	die "probe-invalid: $probe failed aout.py check --arch amd64"
probe_sha=$(sha256_of "$probe")
if [ "$probe_sha" = "$want_probe" ]; then
	probe_match=yes
else
	probe_match=no
fi
note "probe sha256 $probe_sha (recorded $want_probe, match=$probe_match)"

# --- refuse to stage a dirty source tree -------------------------------
dirty=$(git -C "$repo_root" status --porcelain --untracked-files=no -- \
	"$spikes_rel/s1" "$spikes_rel/s3" "$spikes_rel/s5" "$spikes_rel/s6" \
	"$spikes_rel/linux-witness/run-all.sh" \
	"$spikes_rel/linux-witness/README.md" \
	"$spikes_rel/linux-witness/run-from-repo.sh") ||
	die "git-status-failed: cannot read the status of the spike sources"
[ -z "$dirty" ] ||
	die "dirty-sources: the spike sources have uncommitted changes; commit or discard them first:
$dirty"

# --- stage the bundle ---------------------------------------------------
note "staging the bundle at $stage_root"
rm -rf -- "$stage_root"
mkdir -p "$stage_root/s1" "$stage_root/s3" "$stage_root/s5" "$stage_root/s6" \
	"$stage_root/linux-witness" ||
	die "stage-create-failed: cannot create $stage_root"

for f in flake.nix flake.lock venue.nix hostbin.nix; do
	cp "$spikes/s1/$f" "$stage_root/s1/$f" || die "stage-copy-failed: s1/$f"
done

for f in flake.nix flake.lock rio-gate.sh rio-gate.expect determinism.sh; do
	cp "$spikes/s3/$f" "$stage_root/s3/$f" || die "stage-copy-failed: s3/$f"
done

for f in linux-x86_64.sh q35shell-probe.expect aout.py; do
	cp "$spikes/s6/$f" "$stage_root/s6/$f" || die "stage-copy-failed: s6/$f"
done

cp "$spikes/s5/boot-q35.sh" "$stage_root/s5/boot-q35.sh" ||
	die "stage-copy-failed: s5/boot-q35.sh"
cp "$kernel" "$stage_root/s5/9k10cpu.elf" ||
	die "stage-copy-failed: s5/9k10cpu.elf"
cp "$probe" "$stage_root/s6/probe-amd64.aout" ||
	die "stage-copy-failed: s6/probe-amd64.aout"

{
	echo "S6 amd64 probe rebuilt from the committed sources by run-from-repo.sh"
	echo "sha256: $probe_sha"
	echo "recorded in s6/evidence/amd64-static.txt: $want_probe"
	echo "match: $probe_match"
	echo "aout.py check: AOUT-OK amd64"
	echo
	echo "A mismatch is not a failure: the probe was rebuilt with this host's"
	echo "locked rustc, and aout.py asserts every rule r9's x86_64 loader applies."
	echo "The S6 KVM leg validates the actual load and run on this host."
} >"$stage_root/s6/probe-amd64.sha256.txt" ||
	die "stage-write-failed: s6/probe-amd64.sha256.txt"

cp "$spikes/linux-witness/run-all.sh" "$stage_root/linux-witness/run-all.sh" ||
	die "stage-copy-failed: linux-witness/run-all.sh"
cp "$spikes/linux-witness/README.md" "$stage_root/linux-witness/README.md" ||
	die "stage-copy-failed: linux-witness/README.md"
chmod +x "$stage_root/linux-witness/run-all.sh" "$stage_root/s3/rio-gate.sh" \
	"$stage_root/s3/determinism.sh" "$stage_root/s6/linux-x86_64.sh" \
	"$stage_root/s6/aout.py" ||
	die "stage-chmod-failed: cannot mark the staged scripts executable"

# --- MANIFEST.sha256 over every staged file ----------------------------
manifest_tmp=$(mktemp "${TMPDIR:-/tmp}/r9-witness-manifest.XXXXXX") ||
	die "mktemp-failed: cannot create a temporary manifest"
if ! (
	cd "$stage_root" || exit 1
	find . -type f ! -name MANIFEST.sha256 | LC_ALL=C sort |
		while IFS= read -r f; do
			h=$(sha256_of "$f") || exit 1
			printf '%s  %s\n' "$h" "$f"
		done
) >"$manifest_tmp"; then
	rm -f -- "$manifest_tmp"
	die "manifest-write-failed: cannot hash every staged file"
fi
mv "$manifest_tmp" "$stage_root/MANIFEST.sha256" ||
	die "manifest-write-failed: cannot install $stage_root/MANIFEST.sha256"

# Verify the manifest from a fresh extraction, not from the staging tree.
check_root="${stage_root}.check"
rm -rf -- "$check_root"
mkdir -p "$check_root" || die "check-dir-failed: cannot create $check_root"
if ! tar -C "$stage_root" -cf - . | tar -C "$check_root" -xf -; then
	die "check-extract-failed: cannot extract a fresh copy of $stage_root"
fi
note "verifying MANIFEST.sha256 from a fresh extraction"
verify_manifest "$check_root" ||
	die "stage-verify-failed: the fresh extraction does not match MANIFEST.sha256"
note "manifest ok ($(wc -l <"$stage_root/MANIFEST.sha256" | tr -d ' ') files)"
rm -rf -- "$check_root"

if [ "$mode" = check ]; then
	note "--check: staging and manifest verification done; no Linux leg was run"
	echo "R9-WITNESS-CHECK-OK stage=$stage_root"
	exit 0
fi

# --- run the existing driver from the staging root ---------------------
note "running linux-witness/run-all.sh from $stage_root"
run_rc=0
if ! (
	cd "$stage_root" || exit 1
	sh linux-witness/run-all.sh 2>&1 | tee run-all.log
); then
	run_rc=1
fi

# --- the compact return artifact, beside out/ --------------------------
out="$stage_root/out"
return_md="$stage_root/WITNESS-RETURN.md"
git_head=$(git -C "$repo_root" rev-parse HEAD)
{
	echo "# r9 Linux witness return (Plan 01-06)"
	echo
	echo "- date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
	echo "- git HEAD: $git_head"
	echo "- staging root: $stage_root"
	echo "- kernel sha256: $kernel_sha ($kernel_size bytes; matches s5/evidence/kernel-provenance.txt)"
	echo "- probe sha256: $probe_sha"
	echo "- probe recorded sha256: $want_probe"
	echo "- probe match: $probe_match"
	echo "- run-all exit: $run_rc"
	echo
	echo "## out/env.txt"
	echo
	emit_file "$out/env.txt"
	echo "## out/summary.tsv"
	echo
	emit_file "$out/summary.tsv"
	echo "## out/survivors.txt"
	echo
	emit_file "$out/survivors.txt"
	echo "## out/secret-scan.txt"
	echo
	emit_file "$out/secret-scan.txt"
	echo "## last 40 lines of each out/<step>/*.log"
	echo
	for log in "$out"/*/*.log; do
		[ -f "$log" ] || continue
		echo "### ${log#"$stage_root"/}"
		echo
		printf '~~~\n'
		tail -n 40 "$log"
		printf '\n~~~\n\n'
	done
} >"$return_md" || die "return-write-failed: cannot write $return_md"

note "WITNESS-RETURN.md written to $return_md"

cat <<EOF

== What to report back
1. The git HEAD above: $git_head
2. The run-all exit code: $run_rc (a non-zero step is still a complete answer)
3. Any step you skipped, and why.

== Return path A (preferred): commit and push a witness-returns branch from this Linux host
cd "$repo_root"
git switch -C witness-returns/01-06
ret=.planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/return
mkdir -p "\$ret"
cp -a "$stage_root/out" "\$ret/"
cp -a "$stage_root/run-all.log" "\$ret/"
cp -a "$return_md" "\$ret/"
git add "\$ret"
git commit -m "witness(01-06): Linux KVM witness returns"
git push -u origin witness-returns/01-06
# then report: pushed witness-returns/01-06 at <commit sha>

== Return path B: push is impossible -- paste WITNESS-RETURN.md
cat "$return_md"

== Full artifacts
The full out/ tree and run-all.log stay at $stage_root.
Ignore the tar command that run-all.sh prints at its end: it belongs to the
retired scp flow.  Use return path A or B above.
EOF

if [ "$run_rc" -eq 0 ]; then
	echo "R9-WITNESS-RUN-OK: every selected step passed; WITNESS-RETURN.md is at $return_md"
else
	echo "R9-WITNESS-RUN-DONE: at least one step failed or was skipped; that is a complete answer — WITNESS-RETURN.md is at $return_md"
fi
exit "$run_rc"
