#!/usr/bin/env bash
# Phase 3 end-of-phase owner stop - one-shot runner for the x86_64 Linux host.
# Fetches the phase branch, builds kernels at tip and base, compares bytes,
# runs boot-iso and the host suite, then pushes the logs back on the phase branch.
set -uo pipefail

BRANCH="phase-03/crypto-ds-foundations"
TIP_EXPECTED="95fe66aaadde14cb8b8fa8197f6b0547ea7b7898"
BASE="4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1"
PHASE=".planning/phases/03-guide-phase-1-crypto-and-ds-foundations"
ROOT="${ROOT:-$HOME/r9-p03-stop}"
SRC="$(cd "$(dirname "$0")" && pwd)"

echo "== Phase 3 owner stop =="
echo "repo: $SRC"
echo "logs: $ROOT"
mkdir -p "$ROOT"

cd "$SRC" || exit 1
git fetch origin "$BRANCH" || { echo "FETCH FAILED"; exit 1; }
TIP="$TIP_EXPECTED"
git cat-file -e "$TIP^{commit}" 2>/dev/null || { echo "TIP MISSING: $TIP"; exit 1; }
git merge-base --is-ancestor "$TIP" "origin/$BRANCH" || { echo "TIP NOT ON BRANCH: $TIP"; exit 1; }
echo "PUSHED-TIP: $TIP"

for wt in tip base; do
  git worktree remove -f "$ROOT/$wt" >/dev/null 2>&1 || true
  rm -rf "$ROOT/$wt"
done
git worktree add -f --detach "$ROOT/tip" "$TIP" || exit 1
git worktree add -f --detach "$ROOT/base" "$BASE" || exit 1

run_logged() {
  name=$1; shift
  log="$ROOT/$name.log"
  { printf 'HEAD=%s\n' "$(git rev-parse HEAD)"; "$@"; } 2>&1 | tee "$log"
  rc=${PIPESTATUS[0]}
  printf 'EXIT=%s\n' "$rc" | tee -a "$log"
  return "$rc"
}

must() {
  run_logged "$@" || { echo "STEP FAILED: $1 - stopping as the checklist says"; exit 1; }
}

finish() {
  rc=$?
  set +e
  echo
  echo "== result lines =="
  for f in "$ROOT"/*.log; do
    [ -f "$f" ] || continue
    echo "-- $(basename "$f")"
    grep -E '^(HEAD|EXIT)=' "$f"
  done
  cd "$ROOT/tip" || exit "$rc"
  git fetch origin "$BRANCH" >/dev/null 2>&1 || true
  git switch --detach "origin/$BRANCH" >/dev/null 2>&1 || true
  mkdir -p "$PHASE/evidence/owner-stop/return"
  cp "$ROOT"/*.log "$PHASE/evidence/owner-stop/return/" 2>/dev/null
  {
    echo "PUSHED-TIP $TIP"
    echo "SCRIPT-EXIT $rc"
    for f in "$ROOT"/*.log; do
      [ -f "$f" ] || continue
      echo "-- $(basename "$f")"
      grep -E '^(HEAD|EXIT)=' "$f"
    done
  } > "$PHASE/evidence/owner-stop/return/SUMMARY.txt"
  git add "$PHASE/evidence/owner-stop/return" || true
  git -c user.name="owner-stop" -c user.email="owner-stop@localhost" \
      commit -q -m "evidence(03-11): owner-stop return logs" || true
  if git push origin "HEAD:refs/heads/$BRANCH"; then
    echo "RETURN PUSHED: $BRANCH (logs under $PHASE/evidence/owner-stop/return/)"
  else
    echo "RETURN PUSH FAILED - logs are in $ROOT; send them another way"
  fi
  echo "DONE rc=$rc"
  exit "$rc"
}
trap finish EXIT

cd "$ROOT/tip" || exit 1
must tip-kernel-x64 nix build .#kernel-x64 -o "$ROOT/tip-kernel-x64"
must tip-kernel-virt nix build .#kernel-virt -o "$ROOT/tip-kernel-virt"

cd "$ROOT/base" || exit 1
must base-kernel-x64 nix build .#kernel-x64 -o "$ROOT/base-kernel-x64"
must base-kernel-virt nix build .#kernel-virt -o "$ROOT/base-kernel-virt"

cd "$ROOT/tip" || exit 1
must kernel-hashes sha256sum \
  "$ROOT/tip-kernel-x64/kernel-x64.elf" \
  "$ROOT/tip-kernel-virt/kernel-virt.elf" \
  "$ROOT/base-kernel-x64/kernel-x64.elf" \
  "$ROOT/base-kernel-virt/kernel-virt.elf"

must boot-iso-first nix build .#boot-iso
must boot-iso-rebuild nix build .#boot-iso --rebuild

must host-test nix develop -c cargo xtask test
must host-check nix develop -c cargo xtask check
must host-clippy nix develop -c cargo xtask clippy
must host-miri nix develop -c cargo xtask miri
