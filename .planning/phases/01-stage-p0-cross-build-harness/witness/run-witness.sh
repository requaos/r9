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
# Stop-B steps (plan 01-22, D-13):
#   trees         native x86_64-linux builds of both Harvey trees,
#                 `nix store verify --no-trust` on each, and each
#                 HARVEY-TREE stamp.
#   kernel        nix build .#kernel-x64 with the kernel-x64.elf sha256,
#                 then nix build .#boot-iso --rebuild — the fixed-output
#                 check that proves the x86_64 kernel bytes match the
#                 committed outputHash.
#   gate          cargo xtask gate --arch x86-64 --accel kvm, run twice;
#                 both runs are required.
#   gate_neg      the same gate with --force-no-rio; the step succeeds
#                 only on a non-zero gate exit carrying
#                 `GATE-FAIL rio-not-serving'.
#   probe         a throwaway clone at PROBE-FLIP-SHA (plan 01-20):
#                 cargo xtask probe --arch x86-64, then the gate with
#                 --serial-check '/bin/ps=R9-RUST-AOUT-PROBE-OK'.
#   flipback      a throwaway clone at PROBE-FLIPBACK-SHA with the plain
#                 gate.
#   profiles      all nine x86_64 boot-test profiles under KVM, each
#                 logged with its exit.
#   cpuauth_ref   the cpu and auth --compare-reference runs.  01-16
#                 recorded authdomain-gen as Linux-only, so stop B runs
#                 them natively on the witness host.
#
# Stop-C steps (plan 01-26, D-14/D-26/HARN-08):
#   full          cargo xtask gate --arch x86-64 --full --accel kvm — the
#                 eight moved apps in sequence (xtask/src/gate.rs's
#                 FULL_APPS_X86_64); each app's capture set is copied
#                 into out/full/apps/.
#   soak          the owner's tap-venue soak (R9_ACCEL=kvm nix run
#                 .#boot-r9-net-soak-x64).  The step preflights the venue
#                 with `nix run .#net-doctor -- --mode tap' (the ONE
#                 read-only checker, D-03) and fails loudly when it is
#                 absent — the soak accepts no slirp substitute.
#   stress        the owner's tap-venue reduced-stress gate
#                 (R9_ACCEL=kvm nix run .#boot-r9-stress-x64), same
#                 precondition and refusal.
#   equiv         compares each r9 gate log (target/gates/<app>/) with
#                 equivalence.tsv's committed Harvey-hosted baselines;
#                 a `none' row runs the pinned Harvey-hosted app
#                 (github:requaos/harvey/<HARVEY_PIN>#<app>) in the
#                 writable scratch tree and uses that fresh baseline.
#                 Writes out/equiv/report.tsv (gate, match|differ, details).
#   store_verify  nix store verify --no-trust on both tree paths.  Run it
#                 LAST — the run loop honours the selected order.
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
# Plan 01-20's swap-proof commits (the stop-B KVM witness inputs):
#   PROBE-FLIP-SHA     cpu.toml's /amd64/bin/ps is the S6 Rust a.out;
#   PROBE-FLIPBACK-SHA Harvey's ps restored and the probe deleted.
PROBE_FLIP_SHA=064747f42c7f3eff6cd677a8820e6b4b1fff95e3
PROBE_FLIPBACK_SHA=af7df344b9c3702100d4408e85044a722e861a67
PROFILE_LIST=(q35 q35lowmcfg q35net q35exec q35churn q35shell q35churnshell q35cpu q35auth)
# The D-14 full tier's x86_64 apps (xtask/src/gate.rs FULL_APPS_X86_64).
# soak and stress are NOT in it: they need the owner's tap venue and run
# as their own steps.
FULL_APPS=(drawterm-gate-x64 boot-r9-x64 boot-r9-smoke-x64 boot-r9-tier1-x64
	boot-r9-net-smoke-x64 boot-r9-services-x64 boot-r9-churn-x64 boot-r9-auth-x64)
P="$P_REL"

selected=0
for step in $R9_WITNESS_STEPS; do
	case "$step" in
	rio-diag | rio | control | a13 | trees | kernel | gate | gate_neg | probe | flipback | profiles | cpuauth_ref | full | soak | stress | equiv | store_verify)
		selected=$((selected + 1))
		;;
	*)
		die "bad-step '$step': R9_WITNESS_STEPS accepts only rio-diag rio control a13 trees kernel gate gate_neg probe flipback profiles cpuauth_ref full soak stress equiv store_verify"
		;;
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

mkdir -p "$WORK" "$OUT/rio" "$OUT/control" "$OUT/a13" "$OUT/rio-diag" \
	"$OUT/trees" "$OUT/kernel" "$OUT/gate" "$OUT/gate-neg" "$OUT/probe" \
	"$OUT/flipback" "$OUT/profiles" "$OUT/cpuauth-ref" "$OUT/full" \
	"$OUT/soak" "$OUT/stress" "$OUT/equiv" "$OUT/store-verify" ||
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

# --- stop B (plan 01-22): the x86_64 KVM witness batch ------------------

# clone_at_sha <sha> <dir>: a throwaway clone of this r9 checkout at <sha>.
# The probe legs must run the committed tree at that exact revision, not
# the witness checkout's working state, so each leg gets its own clone
# under the work dir.
clone_at_sha() {
	local sha=$1 dir=$2
	rm -rf -- "$dir" || return 1
	git clone --quiet "$R9" "$dir" || return 1
	git -C "$dir" checkout --quiet --detach "$sha"
}

# overlay_fixed_driver <dir>: PROBE_FLIP_SHA and PROBE_FLIPBACK_SHA predate
# 01-33's driver fixes (the fast-fail registration and the final teardown),
# so a bare clone at those revisions aborts at `expect_before -i $spawn_id`
# before the probe marker leg can assert anything (stop B2's probe/flipback
# FAILs).  The probe's subject is the ps swap and the gate's assertion, not
# the driver's history: overlay THIS checkout's driver into the throwaway
# clone, leaving the image, the swapped ps and every other file at the exact
# SHA.
overlay_fixed_driver() {
	local dir=$1
	cp "$R9/gates/x86_64/drawterm-gate.expect" \
		"$dir/gates/x86_64/drawterm-gate.expect"
}

# --- trees: both native x86_64-linux Harvey trees, verified ------------
step_trees() {
	local start end rc paths p
	start=$(now)
	rc=0
	note "trees: nix build .#harvey-tree-amd64 .#harvey-tree-arm64 (native x86_64-linux)"
	: >"$OUT/trees/trees.log"
	paths=$(cd "$R9" || exit 1
		nix build --no-link --print-out-paths .#harvey-tree-amd64 .#harvey-tree-arm64 \
			2>>"$OUT/trees/trees.log") || rc=1
	if [ "$rc" -ne 0 ]; then
		echo "TREES-BUILD-FAILED" | tee -a "$OUT/trees/trees.log"
	fi
	while IFS= read -r p; do
		[ -n "$p" ] || continue
		printf 'TREES-STORE %s\n' "$p" | tee -a "$OUT/trees/trees.log"
		if [ ! -f "$p/HARVEY-TREE" ]; then
			printf 'TREES-MISSING-STAMP %s\n' "$p" | tee -a "$OUT/trees/trees.log"
			rc=1
		else
			printf -- '--- %s/HARVEY-TREE ---\n' "$p" | tee -a "$OUT/trees/trees.log"
			sed 's/^/  /' "$p/HARVEY-TREE" | tee -a "$OUT/trees/trees.log"
		fi
		if nix store verify --no-trust "$p" >>"$OUT/trees/trees.log" 2>&1; then
			printf 'TREES-VERIFY-OK %s\n' "$p" | tee -a "$OUT/trees/trees.log"
		else
			printf 'TREES-VERIFY-FAIL %s\n' "$p" | tee -a "$OUT/trees/trees.log"
			rc=1
		fi
	done <<<"$paths"
	end=$(now)
	record trees "$rc" "$((end - start))"
}

# --- kernel: the x86_64 kernel package and the boot-iso fixed output ---
step_kernel() {
	local start end rc out elf iso
	start=$(now)
	rc=0
	note "kernel: nix build .#kernel-x64 (native x86_64-linux)"
	: >"$OUT/kernel/kernel.log"
	out=$(cd "$R9" || exit 1
		nix build --no-link --print-out-paths .#kernel-x64 \
			2>>"$OUT/kernel/kernel.log") || rc=1
	if [ "$rc" -eq 0 ]; then
		elf="$out/kernel-x64.elf"
		printf 'KERNEL-STORE %s\n' "$out" | tee -a "$OUT/kernel/kernel.log"
		if [ -f "$elf" ]; then
			sha256sum "$elf" | tee -a "$OUT/kernel/kernel.log"
		else
			printf 'KERNEL-MISSING %s\n' "$elf" | tee -a "$OUT/kernel/kernel.log"
			rc=1
		fi
	fi
	if [ "$rc" -eq 0 ]; then
		# The fixed-output check needs a built (or substitutable) output:
		# `--rebuild` refuses a derivation that was never built ("some
		# outputs ... are not valid, so checking is not possible"), which
		# is what stop B tripped on.  Build it plainly first, verify the
		# printed output exists, and only then re-run with --rebuild.
		note "kernel: nix build .#boot-iso (the plain build --rebuild checks)"
		iso=$(cd "$R9" || exit 1
			nix build --no-link --print-out-paths .#boot-iso \
				2>>"$OUT/kernel/kernel.log") || rc=1
		if [ "$rc" -ne 0 ]; then
			echo "KERNEL-BOOTISO-BUILD-FAIL" | tee -a "$OUT/kernel/kernel.log"
		elif [ -z "$iso" ] || [ ! -e "$iso" ]; then
			printf 'KERNEL-BOOTISO-MISSING %s\n' "${iso:-<no output path printed>}" |
				tee -a "$OUT/kernel/kernel.log"
			echo "KERNEL-BOOTISO-BUILD-FAIL" | tee -a "$OUT/kernel/kernel.log"
			rc=1
		else
			printf 'KERNEL-BOOTISO-STORE %s\n' "$iso" | tee -a "$OUT/kernel/kernel.log"
			echo "KERNEL-BOOTISO-BUILD-OK" | tee -a "$OUT/kernel/kernel.log"
		fi
	fi
	if [ "$rc" -eq 0 ]; then
		note "kernel: nix build .#boot-iso --rebuild (the fixed-output kernel-byte check)"
		if (cd "$R9" || exit 1
			nix build --no-link .#boot-iso --rebuild) 2>&1 |
			tee -a "$OUT/kernel/kernel.log"; then
			echo "KERNEL-BOOTISO-REBUILD-OK" | tee -a "$OUT/kernel/kernel.log"
		else
			echo "KERNEL-BOOTISO-REBUILD-FAIL" | tee -a "$OUT/kernel/kernel.log"
			rc=1
		fi
	fi
	end=$(now)
	record kernel "$rc" "$((end - start))"
}

# --- gate: the quick gate twice (HARN-01, D-13) ------------------------
step_gate() {
	local start end rc run
	start=$(now)
	rc=0
	: >"$OUT/gate/gate.log"
	for run in 1 2; do
		note "gate: run $run/2 cargo xtask gate --arch x86-64 --accel kvm"
		printf '== gate run %s/2 ==\n' "$run" >>"$OUT/gate/gate.log"
		if (cd "$R9" || exit 1
			nix develop -c cargo xtask gate --arch x86-64 --accel kvm) 2>&1 |
			tee -a "$OUT/gate/gate.log"; then
			printf 'GATE-RUN %s exit=0\n' "$run" | tee -a "$OUT/gate/gate.log"
		else
			printf 'GATE-RUN %s exit=nonzero\n' "$run" | tee -a "$OUT/gate/gate.log"
			rc=1
		fi
	done
	end=$(now)
	record gate "$rc" "$((end - start))"
}

# --- gate_neg: the failing direction (HARN-01) -------------------------
step_gate_neg() {
	local start end rc log
	start=$(now)
	rc=0
	log="$OUT/gate-neg/gate-neg.log"
	note "gate_neg: cargo xtask gate --arch x86-64 --accel kvm --force-no-rio (must fail with GATE-FAIL rio-not-serving)"
	if (cd "$R9" || exit 1
		nix develop -c cargo xtask gate --arch x86-64 --accel kvm --force-no-rio) \
		2>&1 | tee "$log"; then
		echo "GATE-NEG-UNEXPECTED-PASS" | tee -a "$log"
		rc=1
	elif grep -qF 'GATE-FAIL rio-not-serving' "$log"; then
		echo "GATE-NEG-EXPECTED-FAIL" | tee -a "$log"
	else
		echo "GATE-NEG-WRONG-FAILURE (GATE-FAIL rio-not-serving absent)" | tee -a "$log"
		rc=1
	fi
	end=$(now)
	record gate_neg "$rc" "$((end - start))"
}

# --- probe: the gate at PROBE-FLIP-SHA with the ps marker --------------
step_probe() {
	local start end rc dir
	start=$(now)
	rc=0
	dir="$WORK/r9-probe-flip"
	: >"$OUT/probe/probe.log"
	note "probe: clone at PROBE-FLIP-SHA $PROBE_FLIP_SHA"
	if ! clone_at_sha "$PROBE_FLIP_SHA" "$dir" 2>&1 | tee -a "$OUT/probe/probe.log"; then
		rc=1
	fi
	if [ "$rc" -eq 0 ]; then
		note "probe: overlay the 01-33 fixed drawterm driver into the clone"
		if ! overlay_fixed_driver "$dir"; then
			printf 'PROBE-DRIVER-OVERLAY-FAIL\n' | tee -a "$OUT/probe/probe.log"
			rc=1
		fi
	fi
	if [ "$rc" -eq 0 ]; then
		note "probe: cargo xtask probe, then the gate with --serial-check /bin/ps=R9-RUST-AOUT-PROBE-OK"
		if ! (cd "$dir" || exit 1
			nix develop -c cargo xtask probe --arch x86-64 || exit 1
			nix develop -c cargo xtask gate --arch x86-64 --accel kvm \
				--serial-check '/bin/ps=R9-RUST-AOUT-PROBE-OK') 2>&1 |
			tee -a "$OUT/probe/probe.log"; then
			rc=1
		fi
	fi
	end=$(now)
	record probe "$rc" "$((end - start))"
}

# --- flipback: the gate at PROBE-FLIPBACK-SHA (HARN-04) ----------------
step_flipback() {
	local start end rc dir
	start=$(now)
	rc=0
	dir="$WORK/r9-probe-flipback"
	: >"$OUT/flipback/flipback.log"
	note "flipback: clone at PROBE-FLIPBACK-SHA $PROBE_FLIPBACK_SHA"
	if ! clone_at_sha "$PROBE_FLIPBACK_SHA" "$dir" 2>&1 |
		tee -a "$OUT/flipback/flipback.log"; then
		rc=1
	fi
	if [ "$rc" -eq 0 ]; then
		note "flipback: overlay the 01-33 fixed drawterm driver into the clone"
		if ! overlay_fixed_driver "$dir"; then
			printf 'FLIPBACK-DRIVER-OVERLAY-FAIL\n' |
				tee -a "$OUT/flipback/flipback.log"
			rc=1
		fi
	fi
	if [ "$rc" -eq 0 ]; then
		note "flipback: cargo xtask gate --arch x86-64 --accel kvm (the plain gate)"
		if ! (cd "$dir" || exit 1
			nix develop -c cargo xtask gate --arch x86-64 --accel kvm) \
			2>&1 | tee -a "$OUT/flipback/flipback.log"; then
			rc=1
		fi
	fi
	end=$(now)
	record flipback "$rc" "$((end - start))"
}

# --- profiles: all nine x86_64 boot-test profiles under KVM ------------
step_profiles() {
	local start end rc c
	start=$(now)
	rc=0
	: >"$OUT/profiles/profiles.log"
	for c in "${PROFILE_LIST[@]}"; do
		note "profiles: $c cargo xtask boot-test --arch x86-64 --config $c --accel kvm"
		: >"$OUT/profiles/$c.log"
		if (cd "$R9" || exit 1
			nix develop -c cargo xtask boot-test --arch x86-64 \
				--config "$c" --accel kvm) 2>&1 |
			tee -a "$OUT/profiles/$c.log"; then
			printf 'PROFILE %s exit=0\n' "$c" | tee -a "$OUT/profiles/profiles.log"
		else
			printf 'PROFILE %s exit=nonzero\n' "$c" | tee -a "$OUT/profiles/profiles.log"
			rc=1
		fi
	done
	end=$(now)
	record profiles "$rc" "$((end - start))"
}

# --- cpuauth_ref: the native cpu/auth reference compares ---------------
# 01-16 recorded authdomain-gen as Linux-only, so its conditional holds:
# stop B runs both compares natively on the witness host.
step_cpuauth_ref() {
	local start end rc img ref
	start=$(now)
	rc=0
	: >"$OUT/cpuauth-ref/cpuauth-ref.log"
	# The devshell's R9_HARVEY_REFERENCE_AMD64 is a context-discarded,
	# lazy path (01-15/A9): entering the shell builds nothing, and a
	# compare read from it fails with "No such file or directory" until
	# the reference is realised.  Realise it first and export the
	# printed store path for both compares.
	note "cpuauth_ref: nix build .#harvey-reference-amd64 (realise the reference before reading members)"
	ref=$(cd "$R9" || exit 1
		nix build --no-link --print-out-paths .#harvey-reference-amd64 \
			2>>"$OUT/cpuauth-ref/cpuauth-ref.log") || rc=1
	if [ "$rc" -ne 0 ] || [ -z "$ref" ]; then
		printf 'CPUAUTH-REF-REALISE-FAIL (harvey-reference-amd64 did not build or printed no path)\n' |
			tee -a "$OUT/cpuauth-ref/cpuauth-ref.log"
		rc=1
	else
		printf 'CPUAUTH-REF-REALISED %s\n' "$ref" | tee -a "$OUT/cpuauth-ref/cpuauth-ref.log"
	fi
	for img in cpu auth; do
		[ "$rc" -eq 0 ] || break
		note "cpuauth_ref: $img --compare-reference (authdomain-gen is Linux-only)"
		if (cd "$R9" || exit 1
			nix develop -c env -u R9_HARVEY_DIR \
				R9_HARVEY_REFERENCE_AMD64="$ref" \
				cargo xtask image --arch x86-64 \
				--image "$img" --compare-reference) 2>&1 |
			tee -a "$OUT/cpuauth-ref/cpuauth-ref.log"; then
			printf 'CPUAUTH-REF %s exit=0\n' "$img" | tee -a "$OUT/cpuauth-ref/cpuauth-ref.log"
		else
			printf 'CPUAUTH-REF %s exit=nonzero\n' "$img" | tee -a "$OUT/cpuauth-ref/cpuauth-ref.log"
			rc=1
		fi
	done
	end=$(now)
	record cpuauth_ref "$rc" "$((end - start))"
}

# --- stop C (plan 01-26): the full moved set, soak/stress, equivalence ---

# count_markers <file> <ERE>: occurrences (not lines) — equivalence.tsv's
# counting contract.
count_markers() {
	grep -aoE -- "$2" "$1" 2>/dev/null | wc -l | tr -d ' '
}

# count_in_dir <dir> <ERE>: occurrences over every *.log under <dir>.
count_in_dir() {
	local f n total=0
	while IFS= read -r f; do
		n=$(count_markers "$f" "$2")
		total=$((total + n))
	done < <(find "$1" -type f -name '*.log' -print 2>/dev/null)
	printf '%s\n' "$total"
}

# --- full: the eight moved apps in sequence (D-14) ----------------------
step_full() {
	local start end rc app run_dir stamp
	start=$(now)
	rc=0
	: >"$OUT/full/full.log"
	stamp="$OUT/full/start.stamp"
	: >"$stamp"
	note "full: cargo xtask gate --arch x86-64 --full --accel kvm (the eight moved apps)"
	# The gate's --accel flag is not propagated to the apps the full tier
	# drives; export it so every child app runs the KVM leg explicitly.
	if ! (cd "$R9" || exit 1
		R9_ACCEL=kvm nix develop -c cargo xtask gate --arch x86-64 --full --accel kvm) 2>&1 |
		tee -a "$OUT/full/full.log"; then
		rc=1
	fi
	# Keep every app's own capture set: the moved apps write under
	# target/gates/<app>/ (their R9_GATE_OUT default).
	rm -rf -- "$OUT/full/apps"
	mkdir -p "$OUT/full/apps"
	for app in "${FULL_APPS[@]}"; do
		if [ -d "$R9/target/gates/$app" ]; then
			cp -R -- "$R9/target/gates/$app" "$OUT/full/apps/" || rc=1
		else
			printf 'FULL-NO-CAPTURE %s\n' "$app" | tee -a "$OUT/full/full.log"
			rc=1
		fi
	done
	# The gate's own per-run verdict table (target/gate/<run-id>/full.tsv).
	run_dir=$(find "$R9/target/gate" -mindepth 1 -maxdepth 1 -type d \
		-newer "$stamp" -print -quit 2>/dev/null)
	if [ -n "$run_dir" ] && [ -d "$run_dir" ]; then
		printf 'FULL-RUN-DIR %s\n' "$run_dir" | tee -a "$OUT/full/full.log"
		cp -R -- "$run_dir" "$OUT/full/" || rc=1
	else
		echo "FULL-NO-RUN-DIR (target/gate/<run-id> absent)" | tee -a "$OUT/full/full.log"
	fi
	end=$(now)
	record full "$rc" "$((end - start))"
}

# --- soak / stress: the owner's tap venue (D-13) ------------------------
# Both gates accept ONLY the tap/bridge venue.  net-doctor --mode tap is
# the ONE read-only checker (D-03); its FAIL lines carry the remediation.
# A missing venue fails the step before any bake or boot — never a slirp
# substitute (slirp evidence is not L2 truth).
venue_gate() {
	local step=$1 app=$2 out=$3 start end rc
	start=$(now)
	rc=0
	mkdir -p "$out"
	: >"$out/$step.log"
	note "$step: precondition nix run .#net-doctor -- --mode tap (the owner's tap/bridge venue)"
	if ! (cd "$R9" || exit 1
		nix run .#net-doctor -- --mode tap) 2>&1 | tee -a "$out/$step.log"; then
		printf '%s-PRECONDITION-UNMET: the tap/bridge venue is absent.  This step needs the owner host: a configured bridge, the setuid wrapper and the pinned seam (16-05/D-13); slirp is never accepted as evidence.\n' "$step" |
			tee -a "$out/$step.log"
		rc=1
	fi
	if [ "$rc" -eq 0 ]; then
		note "$step: R9_ACCEL=kvm nix run .#$app (the owner's tap venue)"
		if ! (cd "$R9" || exit 1
			R9_ACCEL=kvm nix run ".#$app") 2>&1 | tee -a "$out/$step.log"; then
			rc=1
		fi
	fi
	end=$(now)
	record "$step" "$rc" "$((end - start))"
}

step_soak() { venue_gate soak boot-r9-net-soak-x64 "$OUT/soak"; }
step_stress() { venue_gate stress boot-r9-stress-x64 "$OUT/stress"; }

# --- equiv: the r9 logs against the Harvey-hosted baselines (HARN-08) --
# A `none' row runs the pinned Harvey-hosted app in the same writable
# scratch tree the rio steps use (ensure_harvey_scratch), with
# R9_GATE_OUT pointed at out/equiv/baseline/<gate>/ so the fresh capture
# and the app's stdout land together.
ensure_equiv_baseline() {
	local gate=$1 dir="$OUT/equiv/baseline/$1" scratch
	if [ -d "$dir" ] && [ -n "$(find "$dir" -type f -name '*.log' -print -quit 2>/dev/null)" ]; then
		return 0
	fi
	scratch=$(ensure_harvey_scratch) || return 1
	mkdir -p "$dir"
	note "equiv: $gate has no committed Harvey-hosted capture — running github:requaos/harvey/$HARVEY_PIN#$gate as the baseline" >&2
	if ! (cd "$scratch" || exit 1
		HARVEY="$scratch" R9_GATE_OUT="$dir" \
			nix run "github:requaos/harvey/$HARVEY_PIN#$gate") >"$dir/baseline.log" 2>&1; then
		printf 'EQUIV-BASELINE-FAIL %s (log: %s)\n' "$gate" "$dir/baseline.log" >&2
		return 1
	fi
}

step_equiv() {
	local start end rc gate regex want src got ok srclabel report details
	start=$(now)
	rc=0
	report="$OUT/equiv/report.tsv"
	details="$OUT/equiv/rows.tsv"
	if [ ! -f "$R9/$P/witness/equivalence.tsv" ]; then
		echo "EQUIV-TSV-MISSING $R9/$P/witness/equivalence.tsv" >&2
		printf 'gate\tverdict\tdetails\n' >"$report"
		printf 'equivalence-tsv\tdiffer\tmissing %s\n' "$P/witness/equivalence.tsv" >>"$report"
		end=$(now)
		record equiv 1 "$((end - start))"
		return
	fi
	: >"$details"
	while IFS=$'\t' read -r gate regex want src; do
		case "$gate" in
		'' | '#'* | gate) continue ;;
		esac
		srclabel=$src
		if [ "$src" = none ]; then
			if ensure_equiv_baseline "$gate"; then
				want=$(count_in_dir "$OUT/equiv/baseline/$gate" "$regex")
				srclabel="fresh github:requaos/harvey/$HARVEY_PIN run"
			else
				want=-1
				srclabel="baseline-run-failed"
			fi
		elif [ ! -f "$R9/$src" ]; then
			printf 'EQUIV-SOURCE-MISSING %s -> %s\n' "$gate" "$src" >&2
			want=-1
			srclabel="source-missing"
		fi
		if [ -d "$R9/target/gates/$gate" ]; then
			got=$(count_in_dir "$R9/target/gates/$gate" "$regex")
		else
			got=-1
		fi
		if [ "$want" != -1 ] && [ "$got" -eq "$want" ]; then ok=ok; else ok=bad; fi
		printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
			"$gate" "$regex" "$got" "$want" "$srclabel" "$ok" >>"$details"
	done <"$R9/$P/witness/equivalence.tsv"
	awk -F'\t' '
		BEGIN { OFS = "\t" }
		{
			g = $1
			if (!(g in seen)) {
				seen[g] = 1
				order[++n] = g
				verdict[g] = "match"
				details[g] = ""
			}
			if ($6 != "ok") verdict[g] = "differ"
			details[g] = details[g] (details[g] == "" ? "" : "; ") $2 "=" $3 "/" $4
		}
		END {
			print "gate", "verdict", "details"
			for (i = 1; i <= n; i++) print order[i], verdict[order[i]], details[order[i]]
		}
	' "$details" >"$report" || rc=1
	if awk -F'\t' '$2 == "differ" { found = 1 } END { exit !found }' "$report"; then
		rc=1
	fi
	end=$(now)
	record equiv "$rc" "$((end - start))"
}

# --- store_verify: both tree paths after everything else ---------------
step_store_verify() {
	local start end rc paths p
	start=$(now)
	rc=0
	: >"$OUT/store-verify/store-verify.log"
	note "store_verify: nix build both trees, then nix store verify --no-trust (run this step LAST)"
	paths=$(cd "$R9" || exit 1
		nix build --no-link --print-out-paths .#harvey-tree-amd64 .#harvey-tree-arm64 \
			2>>"$OUT/store-verify/store-verify.log") || rc=1
	if [ "$rc" -ne 0 ]; then
		echo "STORE-VERIFY-BUILD-FAILED" | tee -a "$OUT/store-verify/store-verify.log"
	fi
	while IFS= read -r p; do
		[ -n "$p" ] || continue
		printf 'STORE-VERIFY-PATH %s\n' "$p" | tee -a "$OUT/store-verify/store-verify.log"
		if nix store verify --no-trust "$p" >>"$OUT/store-verify/store-verify.log" 2>&1; then
			printf 'STORE-VERIFY-OK %s\n' "$p" | tee -a "$OUT/store-verify/store-verify.log"
		else
			printf 'STORE-VERIFY-FAIL %s\n' "$p" | tee -a "$OUT/store-verify/store-verify.log"
			rc=1
		fi
	done <<<"$paths"
	end=$(now)
	record store_verify "$rc" "$((end - start))"
}

# --- run one step in a subshell; record an abort the step did not ------
run_one() {
	local step=$1 rc fn
	# Step names may carry a hyphen (rio-diag); shell function names may not.
	# Map the separator before dispatch, and fail with a named error instead of
	# a bare 127 if the function is missing (the stop-A2 defect).
	fn="step_${step//-/_}"
	if ! declare -F "$fn" >/dev/null 2>&1; then
		echo "R9-WITNESS-ERROR dispatch: no function $fn for step $step" >&2
		record "$step" 127 0
		return
	fi
	( "$fn" )
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
