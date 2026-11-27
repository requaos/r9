#!/usr/bin/env bash
# One x86_64 AP-bring-up debug cycle, for an x86_64 Linux host with KVM.
#
#   bash scripts/kvm-ap-debug.sh                 # build + one KVM boot, print the bring-up lines
#   bash scripts/kvm-ap-debug.sh --push          # ...and push the capture on an ap-debug-<ts> branch
#   bash scripts/kvm-ap-debug.sh --gate boot-r9-tier1-x64
#   bash scripts/kvm-ap-debug.sh --config q35cpu
#
# The record of the KVM AP bring-up fault is
# .planning/debug/x86_64-ap-bringup.md (root cause: the trampoline's data
# segment was expand-down).  The next x86_64 -smp 2 blocker is
# .planning/debug/x86_64-smp2-mcs-kstack.md.  If the capture ends without
# the two bring-up lines and the gate says `qemu exited (EOF)`, that is a
# triple fault: stop the VM at it (-no-reboot -no-shutdown) and read the AP
# through the QEMU monitor before changing anything.
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
echo "== bring-up lines =="
grep -aE 'lapic: ap [0-9]+ online-ready|smp: ' "$log" | head -10 || true
echo
echo "A clean -smp 2 bring-up prints 'lapic: ap 1 online-ready', then 'smp: cpu 1 online (apic 0x1)',"
echo "then 'smp: tlb shootdown ok'."

if [ "$push" -eq 1 ]; then
    br="ap-debug-$ts"
    git switch -c "$br" >/dev/null || { echo "FATAL: cannot create $br" >&2; exit 1; }
    git add -f "$out" || { echo "FATAL: cannot stage $out" >&2; exit 1; }
    git commit -m "evidence: x86_64 AP bring-up $ts" >/dev/null || { echo "FATAL: commit failed" >&2; exit 1; }
    git push -u origin "$br" || { echo "FATAL: push failed" >&2; exit 1; }
    echo
    echo "PUSHED $br"
fi
