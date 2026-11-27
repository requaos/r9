#!/usr/bin/env bash
# x86_64 Linux KVM evidence run for Phase 05 Owner Stops 2 and 3.
#
# Runs the four checks, captures every log and boot capture, commits them on
# a fresh evidence branch, and pushes that branch so the orchestrator can
# review the full result. Nothing is pushed to main.
#
# Usage (from anywhere inside the repo on the x86_64 KVM host):
#   bash scripts/kvm-evidence.sh
set -uo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || { echo "not in a git repo" >&2; exit 1; }
cd "$repo_root" || exit 1

echo "== preflight =="
[ -w /dev/kvm ] || { echo "FATAL: /dev/kvm missing or not writable (no silent TCG downgrade)" >&2; exit 1; }
command -v drawterm >/dev/null || echo "WARN: drawterm not on PATH; the rio gate needs it" >&2
[ -n "$(git status --porcelain)" ] && echo "WARN: working tree is dirty; the evidence branch will include it" >&2

branch="kvm-evidence-$(date -u +%Y%m%dT%H%M%SZ)"
out="target/kvm-evidence"
log="$out/gates.log"
mkdir -p "$out"

# QEMU exception logging: every boot the gates spawn goes through this
# wrapper, so a triple fault is recorded with its vectors instead of
# being lost in a silent reset loop.  -no-reboot stops the guest at the
# fault (a reset loop tells us nothing); the gate still sees the boot
# fail, exactly as before.
diag_log="$repo_root/$out/qemu-diag.log"
diag_wrapper="$repo_root/$out/qemu-diag.sh"
rm -f "$diag_log"
cat >"$diag_wrapper" <<'WRAP'
#!/bin/sh
exec qemu-system-x86_64 -no-reboot -d int,guest_errors,cpu_reset -D "${R9_QEMU_LOG:-/tmp/qemu-diag.log}" "$@"
WRAP
chmod +x "$diag_wrapper"
export R9_QEMU="$diag_wrapper"
export R9_QEMU_LOG="$diag_log"

{
    echo "host: $(uname -a)"
    echo "head: $(git rev-parse HEAD)"
    echo "at:   $(date -u)"
    echo
    echo "=== 1/4 gate: x86-64 golden invariant (rio) at -smp 2, KVM ==="
    nix develop -c cargo xtask gate --arch x86-64 --accel kvm
    echo "GATE_EXIT=$?"
    echo
    echo "=== 2/4 smoke: retained -smp 1 leg ==="
    R9_SMP=1 nix run .#boot-r9-smoke-x64
    echo "SMP1_EXIT=$?"
    echo
    echo "=== 3/4 tier-1 at -smp 2 under KVM (-cpu host => real AVX2; runs fpprobe) ==="
    nix run .#boot-r9-tier1-x64
    echo "TIER1_EXIT=$?"
    echo
    echo "=== 4/4 smoke at -smp 2 (prints the derived fpu format line) ==="
    nix run .#boot-r9-smoke-x64
    echo "SMOKE_EXIT=$?"
    echo
    echo "=== derived fpu format lines from the captures ==="
    grep -Rh 'fpu: format=' target/gates 2>/dev/null | tail -10
    echo
    echo "=== last QEMU exception records (triple fault vectors, if any) ==="
    if [ -f "$diag_log" ]; then
        grep -nE 'check_exception|Triple fault|v=[0-9a-f]{2}|CPU Reset' "$diag_log" | tail -40
    else
        echo "(no qemu-diag.log)"
    fi
} >"$log" 2>&1

tar czf "$out/captures.tgz" target/gates 2>/dev/null || true

git switch -c "$branch" || { echo "FATAL: cannot create branch $branch" >&2; exit 1; }
git add -f "$out" || { echo "FATAL: cannot stage evidence" >&2; exit 1; }
git commit -m "evidence: x86_64 KVM verification" || { echo "FATAL: evidence commit failed" >&2; exit 1; }
git push -u origin "$branch" || { echo "FATAL: evidence push failed" >&2; exit 1; }

echo
echo "PUSHED $branch"
echo
grep -E '^(GATE|SMP1|TIER1|SMOKE)_EXIT=|^GATE (PASS|FAIL)|ALL STAGES PASS|fpu: format=' "$log" | tail -20
