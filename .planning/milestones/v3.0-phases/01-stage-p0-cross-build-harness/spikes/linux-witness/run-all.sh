#!/usr/bin/env bash
# run-all.sh — Plan 06's owner-run driver for the Linux-only spike legs.
#
# Runs on the owner's x86_64 Linux host with KVM (the D-13 witness).  The
# Mac cannot reach that host, so this driver plus README.md is the whole
# handoff: it verifies the bundle, records the host facts, runs each leg in
# isolation, scans the outputs for credentials, and prints the command that
# returns out/ and run-all.log.
#
# Contracts, all enforced below:
#   * refuses root (id -u == 0) and never calls sudo;
#   * verifies MANIFEST.sha256 BEFORE running any step;
#   * writes only under R9_SPIKE_WORK (default $HOME/r9-spike-work) and ./out
#     (relative to the extracted bundle root);
#   * one failing step never stops the next; each appends
#     step<TAB>exit<TAB>seconds to out/summary.tsv;
#   * R9_SPIKES_ONLY="s1 s5 s3 s6" (default) selects a subset;
#   * the qemu, git, expect and python3 tools the legs need are supplied by
#     nix from the committed spike locks (no host packages beyond nix).
#
# Steps:
#   s1  nix build the S1 x86_64-linux derivations: the rung-1 venue probe
#       and the rung-2 hostbin + amd64 libc build.
#   s5  Harvey's own 9k10cpu.elf under q35 KVM (and, if S5_KERNEL_EXTRA is
#       exported, the owner's own kernel built at bd7a698 as a second leg).
#   s3  the pinned Harvey source as a writable scratch copy, then the
#       headless rio gate (rio), its -G control (control-noG), and A12's
#       two-bake determinism check.
#   s6  a blobless clone of r9 at the phase pin plus the x86_64 q35shell
#       chain-image swap (control, probe, flip-back).  Skipped with the
#       named reason S6-SKIP no-harvey-build when the s3 step left no built
#       build/x86_64/out.
#
# Exit: 0 when every selected step exited 0; 1 otherwise (a skipped s6
# records exit 77).  A non-zero step is a complete answer, not a crash:
# the trace is in out/<step>/.
set -uo pipefail

bundle_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$bundle_root" || {
	echo "R9-RUN-ERROR bundle-root: cannot enter $bundle_root" >&2
	exit 1
}

R9_SPIKE_WORK=${R9_SPIKE_WORK:-"$HOME/r9-spike-work"}
R9_SPIKES_ONLY=${R9_SPIKES_ONLY:-"s1 s5 s3 s6"}
KERNDATE=${KERNDATE:-1795382101}
HARVEY_PIN=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
R9_PIN=116bcc23c8052a1728992cdbc7b625143260d683
OUT="$bundle_root/out"
SUMMARY="$OUT/summary.tsv"

die() {
	echo "R9-RUN-ERROR $*" >&2
	exit 1
}

note() {
	echo "== $*"
}

now() {
	date +%s
}

record() {
	printf '%s\t%s\t%s\n' "$1" "$2" "$3" >>"$SUMMARY"
}

# --- the root refusal and the manifest come first -----------------------
[ "$(id -u)" -ne 0 ] ||
	die "running-as-root: this driver refuses root and needs no privileges; run it as your user"

[ -f MANIFEST.sha256 ] ||
	die "manifest-missing: MANIFEST.sha256 is not in $bundle_root (run this from the extracted bundle)"

if command -v shasum >/dev/null 2>&1; then
	shasum -a 256 -c MANIFEST.sha256 >/dev/null ||
		die "manifest-verify-failed: the bundle does not match MANIFEST.sha256 (re-copy the tarball)"
elif command -v sha256sum >/dev/null 2>&1; then
	sha256sum -c MANIFEST.sha256 >/dev/null ||
		die "manifest-verify-failed: the bundle does not match MANIFEST.sha256 (re-copy the tarball)"
else
	die "no-sha256-tool: need shasum or sha256sum to verify the manifest"
fi

command -v nix >/dev/null 2>&1 ||
	die "nix-missing: nix with flakes and the sandbox is required (see README.md)"
command -v tee >/dev/null 2>&1 ||
	die "tee-missing: coreutils tee is required"

# --- the selection contract --------------------------------------------
selected=0
for step in $R9_SPIKES_ONLY; do
	case "$step" in
	s1 | s3 | s5 | s6) selected=$((selected + 1)) ;;
	*) die "bad-step '$step': R9_SPIKES_ONLY accepts only s1 s3 s5 s6" ;;
	esac
done
[ "$selected" -gt 0 ] || die "empty-selection: R9_SPIKES_ONLY names no step"
want() {
	case " $R9_SPIKES_ONLY " in
	*" $1 "*) return 0 ;;
	*) return 1 ;;
	esac
}

mkdir -p "$R9_SPIKE_WORK" "$OUT" "$OUT/s1" "$OUT/s3" "$OUT/s5" "$OUT/s6" ||
	die "cannot-create-output: $OUT or $R9_SPIKE_WORK is not writable"
: >"$SUMMARY"

# --- preflight: the host facts S1/S3/S5/S6 are judged against ----------
preflight() {
	{
		echo "# r9 Linux witness preflight"
		echo "date=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
		echo "user=$(id -un) uid=$(id -u)"
		echo "bundle_root=$bundle_root"
		echo "work_dir=$R9_SPIKE_WORK"
		echo "uname=$(uname -a)"
		echo "nproc=$(nproc 2>/dev/null || echo unavailable)"
		if command -v free >/dev/null 2>&1; then
			free -h
		else
			grep -E '^(MemTotal|MemAvailable):' /proc/meminfo 2>/dev/null || echo "memory=unavailable"
		fi
		echo "--- free disk under $R9_SPIKE_WORK ---"
		df -h "$R9_SPIKE_WORK"
		echo "--- nix ---"
		nix --version 2>&1 | head -1
		nix show-config 2>/dev/null | grep -E '^(sandbox|experimental-features) ?=' ||
			echo "nix-show-config=unavailable"
		echo "--- /dev/kvm ---"
		if [ -w /dev/kvm ]; then
			echo "DEV_KVM=writable"
		else
			echo "DEV_KVM=not-writable"
		fi
		ls -l /dev/kvm 2>&1 || true
		echo "--- sysctls (unprivileged userns is what Harvey's x86_64 venue needs) ---"
		for k in user.max_user_namespaces kernel.unprivileged_userns_clone kernel.apparmor_restrict_unprivileged_userns; do
			if v=$(sysctl -n "$k" 2>/dev/null); then
				echo "$k=$v"
			else
				echo "$k=unavailable"
			fi
		done
	} >"$OUT/env.txt" 2>&1
}

# --- s1: the sandbox venue and Harvey's amd64 libc ---------------------
s1_step() {
	local start end rc
	start=$(now)
	rc=0
	note "s1: nix build s1-nsprobe (rung 1, unshare venue)"
	if ! nix build -L --no-link --print-out-paths \
		"path:./s1#packages.x86_64-linux.s1-nsprobe" 2>&1 |
		tee "$OUT/s1/nsprobe.log"; then
		rc=1
	fi
	note "s1: nix build s1-libc-amd64 (rung 2, proot venue + hostbin + amd64 libc)"
	if ! nix build -L --no-link --print-out-paths \
		"path:./s1#packages.x86_64-linux.s1-libc-amd64" 2>&1 |
		tee "$OUT/s1/libc-amd64.log"; then
		rc=1
	fi
	end=$(now)
	record s1 "$rc" "$((end - start))"
}

# --- s5: Harvey's own 9k kernel under q35 KVM --------------------------
s5_step() {
	local start end rc
	start=$(now)
	rc=0
	note "s5: Harvey's 9k10cpu.elf under q35 KVM (virtio-net)"
	if ! S5_ACCEL=kvm S5_MACHINE=q35 S5_NET=virtio \
		S5_KERNEL=./s5/9k10cpu.elf S5_LOG="$OUT/s5/q35-kvm.log" \
		nix shell --inputs-from "path:./s3" nixpkgs#qemu -c sh s5/boot-q35.sh 2>&1 |
		tee "$OUT/s5/q35-kvm-stdout.log"; then
		rc=1
	fi
	if [ -n "${S5_KERNEL_EXTRA:-}" ]; then
		note "s5: extra leg with the owner's kernel $S5_KERNEL_EXTRA"
		if ! S5_ACCEL=kvm S5_MACHINE=q35 S5_NET=virtio \
			S5_KERNEL="$S5_KERNEL_EXTRA" S5_LOG="$OUT/s5/q35-kvm-extra.log" \
			nix shell --inputs-from "path:./s3" nixpkgs#qemu -c sh s5/boot-q35.sh 2>&1 |
			tee "$OUT/s5/q35-kvm-extra-stdout.log"; then
			rc=1
		fi
	fi
	end=$(now)
	record s5 "$rc" "$((end - start))"
}

# --- s3: the pinned scratch Harvey tree, rio, its control, determinism -
s3_step() {
	local start end rc prefetch harvey_src work_harvey bake1
	start=$(now)
	rc=0
	work_harvey="$R9_SPIKE_WORK/harvey"

	note "s3: nix flake prefetch github:requaos/harvey/$HARVEY_PIN"
	if prefetch=$(nix flake prefetch --json "github:requaos/harvey/$HARVEY_PIN" 2>"$OUT/s3/prefetch.err"); then
		harvey_src=$(printf '%s\n' "$prefetch" |
			sed -n 's/.*"storePath"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
		[ -n "$harvey_src" ] || {
			echo "R9-RUN-ERROR s3-prefetch: no storePath in the prefetch JSON" >&2
			rc=1
		}
	else
		cat "$OUT/s3/prefetch.err" >&2
		rc=1
	fi

	if [ "$rc" -eq 0 ]; then
		note "s3: writable scratch copy at $work_harvey (no .git)"
		rm -rf -- "$work_harvey"
		if ! cp -r "$harvey_src" "$work_harvey"; then
			rc=1
		fi
		if [ "$rc" -eq 0 ]; then
			chmod -R u+w "$work_harvey" || rc=1
			rm -rf -- "$work_harvey/.git"
		fi
	fi

	if [ "$rc" -eq 0 ]; then
		note "s3: rio leg (non -G drawterm under Xvfb, serial ls /srv)"
		if ! HARVEY="$work_harvey" S3_OUT="$OUT/s3/rio" S3_MODE=rio \
			KERNDATE="$KERNDATE" \
			nix run "path:./s3#s3-rio-gate" 2>&1 |
			tee "$OUT/s3/rio.log"; then
			rc=1
		fi
		note "s3: control-noG leg (the -G discriminator)"
		if ! HARVEY="$work_harvey" S3_OUT="$OUT/s3/control-noG" S3_MODE=control-noG \
			KERNDATE="$KERNDATE" \
			nix run "path:./s3#s3-rio-gate" 2>&1 |
			tee "$OUT/s3/control-noG.log"; then
			rc=1
		fi
		note "s3: determinism (two bakes at KERNDATE=$KERNDATE)"
		bake1="$OUT/s3/control-noG/bake.log"
		[ -f "$bake1" ] || bake1="$OUT/s3/rio/bake.log"
		if ! HARVEY="$work_harvey" S3_OUT="$OUT/s3/determinism" \
			S3_BAKE1_LOG="$bake1" KERNDATE="$KERNDATE" \
			nix develop "path:$work_harvey" -c bash s3/determinism.sh 2>&1 |
			tee "$OUT/s3/determinism.log"; then
			rc=1
		fi
	fi

	end=$(now)
	record s3 "$rc" "$((end - start))"
}

# --- s6: the x86_64 q35shell chain-image swap --------------------------
s6_step() {
	local start end rc work_r9
	start=$(now)
	rc=0
	work_r9="$R9_SPIKE_WORK/r9"

	if [ ! -d "$R9_SPIKE_WORK/harvey/build/x86_64/out" ] ||
		[ ! -s "$R9_SPIKE_WORK/harvey/build/x86_64/out/echo" ]; then
		note "s6: SKIP no-harvey-build (the s3 step left no built build/x86_64/out)"
		printf 'S6-SKIP no-harvey-build\n' | tee "$OUT/s6/skip.txt"
		record s6 77 0
		return 0
	fi

	note "s6: blobless clone of requaos/r9 at $R9_PIN"
	rm -rf -- "$work_r9"
	if ! nix shell --inputs-from "path:./s3" nixpkgs#git -c git clone \
		--filter=blob:none https://github.com/requaos/r9 "$work_r9" >"$OUT/s6/clone.log" 2>&1; then
		tail -n 20 "$OUT/s6/clone.log" >&2
		rc=1
	fi
	if [ "$rc" -eq 0 ]; then
		if ! nix shell --inputs-from "path:./s3" nixpkgs#git -c git -C "$work_r9" \
			checkout "$R9_PIN" >>"$OUT/s6/clone.log" 2>&1; then
			tail -n 20 "$OUT/s6/clone.log" >&2
			rc=1
		fi
	fi

	if [ "$rc" -eq 0 ]; then
		note "s6: q35shell control/probe/flip-back under KVM"
		if ! S6_HARVEY="$R9_SPIKE_WORK/harvey" S6_R9="$work_r9" \
			S6_PROBE=./s6/probe-amd64.aout S6_OUT="$OUT/s6" KERNDATE="$KERNDATE" \
			nix shell --inputs-from "path:./s3" nixpkgs#git nixpkgs#expect nixpkgs#python3 \
			nixpkgs#procps -c bash s6/linux-x86_64.sh 2>&1 |
			tee "$OUT/s6/linux-x86_64.log"; then
			rc=1
		fi
	fi

	end=$(now)
	record s6 "$rc" "$((end - start))"
}

# --- finish: survivors and the credential scan -------------------------
finish() {
	local found creds pw f rel
	{
		echo "# surviving qemu/Xvfb/drawterm processes owned by $(id -un)"
		echo "# at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
		if rows=$(pgrep -a -u "$(id -u)" -f 'qemu-system|Xvfb|drawterm' 2>/dev/null) &&
			[ -n "$rows" ]; then
			printf '%s\n' "$rows"
		else
			echo "none"
		fi
	} >"$OUT/survivors.txt"

	found=0
	{
		echo "# credential scan of out/ at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
		while IFS= read -r f; do
			rel=${f#"$bundle_root"/}
			echo "DELETED auth-creds artifact: $rel"
			rm -f -- "$f"
			found=1
		done < <(find "$OUT" -type f -name 'auth-creds*' 2>/dev/null)

		creds="$R9_SPIKE_WORK/harvey/build/x86_64/out/auth-creds.txt"
		if [ -f "$creds" ]; then
			pw=$(sed -n 's/^password=//p' "$creds" | tr -d '\r\n')
			if [ -n "$pw" ]; then
				while IFS= read -r f; do
					[ -n "$f" ] || continue
					rel=${f#"$bundle_root"/}
					echo "DELETED file containing the DEMO password value: $rel"
					rm -f -- "$f"
					found=1
				done < <(grep -rlF -- "$pw" "$OUT" 2>/dev/null || true)
				echo "scanned for the DEMO password value from the scratch tree's auth-creds.txt"
			else
				echo "auth-creds.txt exists but its password= line is empty; nothing to scan"
			fi
		else
			echo "no scratch auth-creds.txt at $creds (the bake did not run, or the tree was cleaned)"
		fi

		if [ "$found" -eq 0 ]; then
			echo "clean: no auth-creds artifact and no DEMO password value in out/"
		else
			echo "REMINDER: list every deletion above to whoever reads the returns; do not restore them"
		fi
	} >"$OUT/secret-scan.txt"

	echo
	echo "== per-step summary (step, exit, seconds):"
	cat "$SUMMARY"
	echo
	echo '== return command (run from the extracted bundle root):'
	echo "tar -C \"$bundle_root\" -czf \"\$HOME/r9-spikes-out.tar.gz\" out run-all.log"
}

# --- run ----------------------------------------------------------------
preflight

if want s1; then s1_step; fi
if want s5; then s5_step; fi
if want s3; then s3_step; fi
if want s6; then s6_step; fi

finish

fail=0
while IFS=$'\t' read -r _exit_step exit_code _exit_secs; do
	[ "$exit_code" = "0" ] || fail=1
done <"$SUMMARY"

if [ "$fail" -eq 0 ]; then
	echo "R9-RUN-DONE: every selected step passed"
else
	echo "R9-RUN-DONE: at least one step failed or was skipped — read out/summary.tsv and out/<step>/"
fi
exit "$fail"
