#!/usr/bin/env bash
# One x86_64 AP-bring-up debug cycle, for an x86_64 Linux host with KVM.
#
#   bash scripts/kvm-ap-debug.sh                 # build + one KVM boot, print the AP marker chain
#   bash scripts/kvm-ap-debug.sh --push          # ...and push the capture on an ap-debug-<ts> branch
#   bash scripts/kvm-ap-debug.sh --gate boot-r9-tier1-x64
#   bash scripts/kvm-ap-debug.sh --config q35cpu
#
# The debug brief is .planning/debug/x86_64-ap-bringup.md — read it first; it
# carries the marker legend, the eliminated hypotheses and the experiment plan.
#
# NOTE: `nix run .#<gate>` builds the kernel through the flake, so a code change
# must be COMMITTED before you run this (the script refuses a dirty tree unless
# --allow-dirty, because a dirty tree silently builds the wrong kernel).
set -uo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null) || { echo "not in a git repo" >&2; exit 1; }
cd "$repo_root" || exit 1

config=q35churnshell
gate=boot-r9-smoke-x64
push=0
allow_dirty=0
while [ $# -gt 0 ]; do
    case "$1" in
        --push) push=1 ;;
        --allow-dirty) allow_dirty=1 ;;
        --config) config="${2:?--config needs a value}"; shift ;;
        --gate) gate="${2:?--gate needs a value}"; shift ;;
        -h|--help) sed -n '2,14p' "$0"; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -w /dev/kvm ] || { echo "FATAL: /dev/kvm missing or not writable (this script is KVM-only)" >&2; exit 1; }
if [ -n "$(git status --porcelain)" ] && [ "$allow_dirty" -ne 1 ]; then
    echo "FATAL: working tree is dirty; commit the change first (or pass --allow-dirty)" >&2
    git status --short >&2
    exit 1
fi

out="target/ap-debug"
mkdir -p "$out"
ts=$(date -u +%Y%m%dT%H%M%SZ)
log="$out/run-$ts.log"

echo "== head: $(git rev-parse --short HEAD)  config: $config  gate: $gate =="
echo "== build (compile check; the gate rebuilds through the flake) =="
nix develop -c cargo xtask build --arch x86-64 --config "$config" 2>&1 | tail -3

echo "== boot under KVM =="
nix run ".#$gate" 2>&1 | tee "$log" | tail -25

echo
echo "== AP marker chain =="
markers=$(grep -ao 'AP:.' "$log" 2>/dev/null | tr '\n' ' ')
echo "${markers:-<none>}"
echo "== bring-up lines =="
grep -aE 'smp:|online|tlb shootdown' "$log" | head -10 || true
echo
echo "Legend: AP:R G P 4 5 6 C T J 2 3 4 5 6 ; last one seen is the last stage that completed."

if [ "$push" -eq 1 ]; then
    br="ap-debug-$ts"
    git switch -c "$br" >/dev/null || { echo "FATAL: cannot create $br" >&2; exit 1; }
    git add -f "$out" || { echo "FATAL: cannot stage $out" >&2; exit 1; }
    git commit -m "evidence: x86_64 AP bring-up $ts" >/dev/null || { echo "FATAL: commit failed" >&2; exit 1; }
    git push -u origin "$br" || { echo "FATAL: push failed" >&2; exit 1; }
    echo
    echo "PUSHED $br"
fi
