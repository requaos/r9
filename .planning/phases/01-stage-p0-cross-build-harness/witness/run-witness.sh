#!/usr/bin/env bash
# shellcheck disable=SC2329  # steps are dispatched indirectly via run_one's "step_${step}"
# run-witness.sh — plan 01-09's reusable pass-2 witness driver (D-13/D-28).
#
# The owner runs this from an r9 checkout root on the x86_64 Linux KVM
# host.  Nothing is copied from the Mac: the host gets every script from
# GitHub with `git pull` (or a clone) and returns results through a
# pushed branch or a pasted file.  The Mac cannot reach that host.
#
# Contract:
#   * refuses root; no privileged step is needed and no command escalates
#     privileges;
#   * resolves the r9 root with git and records `git rev-parse HEAD`;
#   * writes only under R9_WITNESS_WORK (default $HOME/r9-witness-work),
#     with per-step results under $R9_WITNESS_WORK/out/<step>/;
#   * preflights out/env.txt (uname, nproc, memory, disk, nix, sandbox,
#     /dev/kvm) before the first step;
#   * steps are shell functions `step_<name>` selected by
#     R9_WITNESS_STEPS (space-separated); each runs in a subshell and
#     appends `step<TAB>exit<TAB>seconds` to out/summary.tsv;
#   * finish writes out/survivors.txt, out/secret-scan.txt and
#     WITNESS-RETURN.md beside out/.
#
# Steps in this plan:
#   rio      the fixed headless rio gate (RIO_MODE=rio) against a pinned
#            Harvey scratch tree; the D-28 marker must print RIO-SERVING.
#   control  the same gate with RIO_MODE=control-noG; it must print
#            RIO-CONTROL-NO-RIO (the unchanged discriminator).
#   a13      nix build .#checks.x86_64-linux.harvey-drawterm from the
#            r9 root (plan 01-10 adds that check; it is the drawterm
#            build gate, not a KVM leg).
#   rio-diag (plan 01-32) the same gate with RIO_MODE=rio-diag: the
#            staged profile marks the rio handoff with a serial-visible
#            #s entry (/srv/r9-login-ok + /srv/r9-login-pid.<pid>) and
#            the driver captures x11.txt plus two PNGs.  Its log names
#            `RIO-DIAG login-ok' or `RIO-FAIL login-not-reached' and then
#            the unchanged pair verdict.  It is a diagnostic, NOT a gate:
#            exit 0 still requires RIO-SERVING, and the marker alone
#            records no PASS.
#
# Later plans add steps to this file.  Keep each step self-contained so
# adding one never edits another.
#
# Exit: 0 only when every selected step exited 0; 1 otherwise.  A
# non-zero step is a complete answer: its log is in out/<step>/.

set -uo pipefail

die() {
	echo "R9-WITNESS-ERROR $*" >&2
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

# --- root refusal (first, before any other command) --------------------
[ "$(id -u)" -ne 0 ] ||
	die "running-as-root: this driver refuses root and needs no privileges; run it as your user"

# --- locate the r9 checkout from any cwd inside it ---------------------
script_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P) ||
	die "script-dir-unavailable: cannot resolve the directory holding $0"
R9=$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null) ||
	die "not-a-git-checkout: $script_dir is not inside an r9 git checkout"
P_REL=.planning/phases/01-stage-p0-cross-build-harness
[ -d "$R9/$P_REL/rio-derisk" ] ||
	die "wrong-checkout: $R9/$P_REL/rio-derisk is missing (is this r9?)"
[ -f "$R9/$P_REL/witness/run-witness.sh" ] ||
	die "wrong-checkout: $R9/$P_REL/witness/run-witness.sh is missing"
HEAD_SHA=$(git -C "$R9" rev-parse HEAD) ||
	die "git-head-failed: cannot read HEAD in $R9"

# --- the work root and the selection -----------------------------------
R9_WITNESS_WORK=${R9_WITNESS_WORK:-"$HOME/r9-witness-work"}
case "$R9_WITNESS_WORK" in
	/*) ;;
	*) die "bad-work: R9_WITNESS_WORK must be an absolute path, got '$R9_WITNESS_WORK'" ;;
esac
case "$R9_WITNESS_WORK" in
	/nix/store/*) die "work-in-store: R9_WITNESS_WORK=$R9_WITNESS_WORK is a read-only store path" ;;
esac
WORK=$R9_WITNESS_WORK
OUT="$WORK/out"
SUMMARY="$OUT/summary.tsv"
R9_WITNESS_STEPS=${R9_WITNESS_STEPS:-"rio control a13"}
HARVEY_PIN=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
KERNDATE=${KERNDATE:-1795382101}
P="$P_REL"

selected=0
for step in $R9_WITNESS_STEPS; do
	case "$step" in
	rio-diag | rio | control | a13) selected=$((selected + 1)) ;;
	*) die "bad-step '$step': R9_WITNESS_STEPS accepts only rio-diag rio control a13" ;;
	esac
done
[ "$selected" -gt 0 ] || die "empty-selection: R9_WITNESS_STEPS names no step"

want() {
	case " $R9_WITNESS_STEPS " in
	*" $1 "*) return 0 ;;
	*) return 1 ;;
	esac
}

command -v nix >/dev/null 2>&1 ||
	die "nix-missing: nix with flakes and the sandbox is required (see README.md)"
command -v tee >/dev/null 2>&1 ||
	die "tee-missing: coreutils tee is required"

mkdir -p "$WORK" "$OUT/rio" "$OUT/control" "$OUT/a13" "$OUT/rio-diag" ||
	die "cannot-create-output: $OUT or $WORK is not writable"
: >"$SUMMARY"

# --- preflight: the host facts the KVM steps are judged against --------
preflight() {
	{
		echo "# r9 pass-2 witness preflight"
		echo "date=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
		echo "user=$(id -un) uid=$(id -u)"
		echo "r9_root=$R9"
		echo "git_head=$HEAD_SHA"
		echo "work_dir=$WORK"
		echo "steps=$R9_WITNESS_STEPS"
		echo "harvey_pin=$HARVEY_PIN"
		echo "kerndate=$KERNDATE"
		echo "uname=$(uname -a)"
		echo "nproc=$(nproc 2>/dev/null || echo unavailable)"
		if command -v free >/dev/null 2>&1; then
			free -h
		else
			grep -E '^(MemTotal|MemAvailable):' /proc/meminfo 2>/dev/null || echo "memory=unavailable"
		fi
		echo "--- free disk under $WORK ---"
		df -h "$WORK"
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
	} >"$OUT/env.txt" 2>&1
}

# --- the shared pinned Harvey scratch tree -----------------------------
# Prefetch the phase pin, copy it writable, and drop .git.  The gate
# refuses a real checkout or a store path, so this is the only shape it
# accepts.  Reused by both gate steps when it already exists.
ensure_harvey_scratch() {
	local prefetch src _prof _bak
	# A previous rio-diag run killed before its EXIT trap can leave the
	# staged profile patched; the gate's backup is the pristine copy.
	# Restore it before any step reuses the tree.
	_prof="$WORK/harvey/build/x86_64/root-tree/usr/glenda/lib/profile"
	_bak="$WORK/harvey/build/x86_64/r9-diag-profile.orig"
	if [ -f "$_bak" ] && [ -f "$_prof" ] && grep -qF '/srv/r9-login-ok' "$_prof" 2>/dev/null; then
		cp "$_bak" "$_prof" || return 1
		echo "== restored the staged profile from $_bak (a prior rio-diag run left it patched)" >&2
	fi
	if [ -d "$WORK/harvey/sys/src" ] && [ ! -e "$WORK/harvey/.git" ]; then
		printf '%s\n' "$WORK/harvey"
		return 0
	fi
	note "prefetch github:requaos/harvey/$HARVEY_PIN" >&2
	prefetch=$(nix flake prefetch --json "github:requaos/harvey/$HARVEY_PIN" 2>"$OUT/prefetch.err") ||
		return 1
	src=$(printf '%s\n' "$prefetch" |
		sed -n 's/.*"storePath"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)
	[ -n "$src" ] || {
		echo "R9-WITNESS-ERROR prefetch: no storePath in the prefetch JSON" >&2
		return 1
	}
	rm -rf -- "$WORK/harvey" || return 1
	cp -r "$src" "$WORK/harvey" || return 1
	chmod -R u+w "$WORK/harvey" || return 1
	rm -rf -- "$WORK/harvey/.git"
	printf '%s\n' "$WORK/harvey"
}

# --- rio: the fixed headless rio gate (D-28 marker) --------------------
step_rio() {
	local start end rc scratch
	start=$(now)
	rc=0
	if ! scratch=$(ensure_harvey_scratch); then
		rc=1
	fi
	if [ "$rc" -eq 0 ]; then
		note "rio: nix run path:<r9>/$P/rio-derisk#rio-gate (RIO_MODE=rio)"
		if ! HARVEY="$scratch" RIO_OUT="$OUT/rio" RIO_MODE=rio \
			R9_ACCEL=kvm KERNDATE="$KERNDATE" \
			nix run "path:$R9/$P/rio-derisk#rio-gate" 2>&1 |
			tee "$OUT/rio/rio.log"; then
			rc=1
		fi
	fi
	end=$(now)
	record rio "$rc" "$((end - start))"
}

# --- rio-diag: the login/rio-start diagnostic (plan 01-32) -------------
# Same gate, RIO_MODE=rio-diag: the staged profile creates the serial-
# visible #s entries /srv/r9-login-ok and /srv/r9-login-pid.<pid> before
# `exec rio', and the driver captures x11.txt + two PNGs into
# out/rio-diag/.  The step's exit is the gate's: 0 only on RIO-SERVING.
# A non-zero exit with `RIO-DIAG login-ok' or `RIO-FAIL login-not-reached'
# in the log is a complete diagnostic answer, not a driver failure.
step_rio_diag() {
	local start end rc scratch
	start=$(now)
	rc=0
	if ! scratch=$(ensure_harvey_scratch); then
		rc=1
	fi
	if [ "$rc" -eq 0 ]; then
		note "rio-diag: nix run path:<r9>/$P/rio-derisk#rio-gate (RIO_MODE=rio-diag)"
		if ! HARVEY="$scratch" RIO_OUT="$OUT/rio-diag" RIO_MODE=rio-diag \
			R9_ACCEL=kvm KERNDATE="$KERNDATE" \
			nix run "path:$R9/$P/rio-derisk#rio-gate" 2>&1 |
			tee "$OUT/rio-diag/rio-diag.log"; then
			rc=1
		fi
	fi
	end=$(now)
	record rio-diag "$rc" "$((end - start))"
}

# --- control: the unchanged -G discriminator --------------------------
step_control() {
	local start end rc scratch
	start=$(now)
	rc=0
	if ! scratch=$(ensure_harvey_scratch); then
		rc=1
	fi
	if [ "$rc" -eq 0 ]; then
		note "control: nix run path:<r9>/$P/rio-derisk#rio-gate (RIO_MODE=control-noG)"
		if ! HARVEY="$scratch" RIO_OUT="$OUT/control" RIO_MODE=control-noG \
			R9_ACCEL=kvm KERNDATE="$KERNDATE" \
			nix run "path:$R9/$P/rio-derisk#rio-gate" 2>&1 |
			tee "$OUT/control/control.log"; then
			rc=1
		fi
	fi
	end=$(now)
	record control "$rc" "$((end - start))"
}

# --- a13: the Harvey drawterm build check (plan 01-10 adds it) ---------
step_a13() {
	local start end rc
	start=$(now)
	rc=0
	note "a13: nix build .#checks.x86_64-linux.harvey-drawterm (from the r9 root)"
	if ! (
		cd "$R9" || exit 1
		nix build --no-link --print-out-paths .#checks.x86_64-linux.harvey-drawterm
	) 2>&1 | tee "$OUT/a13/a13.log"; then
		rc=1
	fi
	end=$(now)
	record a13 "$rc" "$((end - start))"
}

# --- run one step in a subshell; record an abort the step did not ------
run_one() {
	local step=$1 rc
	( "step_${step}" )
	rc=$?
	if ! awk -F'\t' -v s="$step" '$1 == s { found = 1 } END { exit !found }' "$SUMMARY"; then
		record "$step" "$rc" 0
	fi
}

# --- finish: survivors, the credential scan, WITNESS-RETURN.md ---------
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
		for f in "$OUT"/auth-creds* "$OUT"/*.pwfile "$OUT"/*/dt-password; do
			[ -f "$f" ] || continue
			rel=${f#"$WORK"/}
			echo "DELETED credential artifact under out/: $rel"
			rm -f -- "$f"
			found=1
		done

		# Any creds file the bake left under the work root is the value
		# source; scan out/ for it without ever printing it.
		while IFS= read -r creds; do
			[ -n "$creds" ] || continue
			pw=$(sed -n 's/^password=//p' "$creds" | tr -d '\r\n' | head -1)
			if [ -n "$pw" ]; then
				while IFS= read -r f; do
					[ -n "$f" ] || continue
					rel=${f#"$WORK"/}
					echo "DELETED file containing the DEMO password value: $rel"
					rm -f -- "$f"
					found=1
				done < <(grep -rlF -- "$pw" "$OUT" 2>/dev/null || true)
				echo "scanned out/ for the password value of ${creds#"$WORK"/}"
			fi
		done < <(find "$WORK" -type f \( -name 'auth-creds.txt' -o -name 'auth-creds' \) 2>/dev/null)

		if [ "$found" -eq 0 ]; then
			echo "clean: no credential artifact and no DEMO password value in out/"
		else
			echo "REMINDER: list every deletion above to whoever reads the returns; do not restore them"
		fi
	} >"$OUT/secret-scan.txt"

	return_md="$WORK/WITNESS-RETURN.md"
	{
		echo "# r9 pass-2 witness return (plans 01-09 / 01-32)"
		echo
		echo "- date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
		echo "- git HEAD: $HEAD_SHA"
		echo "- r9 root: $R9"
		echo "- work root: $WORK"
		echo "- steps: $R9_WITNESS_STEPS"
		echo
		echo "## out/env.txt"
		echo
		emit_file "$OUT/env.txt"
		echo "## out/summary.tsv"
		echo
		emit_file "$SUMMARY"
		echo "## out/survivors.txt"
		echo
		emit_file "$OUT/survivors.txt"
		echo "## out/secret-scan.txt"
		echo
		emit_file "$OUT/secret-scan.txt"
		echo "## last 40 lines of each out/<step>/*.log"
		echo
		for log in "$OUT"/*/*.log; do
			[ -f "$log" ] || continue
			echo "### ${log#"$WORK"/}"
			echo
			printf '~~~\n'
			tail -n 40 "$log"
			printf '\n~~~\n\n'
		done
	} >"$return_md" || die "return-write-failed: cannot write $return_md"

	echo
	echo "== per-step summary (step, exit, seconds):"
	cat "$SUMMARY"
	echo
	echo "== WITNESS-RETURN.md is at $return_md"
	echo "== full artifacts stay under $OUT"
}

# --- run ---------------------------------------------------------------
preflight

for step in $R9_WITNESS_STEPS; do
	if want "$step"; then
		run_one "$step"
	fi
done

finish

fail=0
while IFS=$'\t' read -r _step exit_code _secs; do
	[ "$exit_code" = "0" ] || fail=1
done <"$SUMMARY"

if [ "$fail" -eq 0 ]; then
	echo "R9-WITNESS-RUN-OK: every selected step passed; WITNESS-RETURN.md is at $WORK/WITNESS-RETURN.md"
else
	echo "R9-WITNESS-RUN-DONE: at least one step failed; that is a complete answer — read out/summary.tsv and out/<step>/"
fi
exit "$fail"
