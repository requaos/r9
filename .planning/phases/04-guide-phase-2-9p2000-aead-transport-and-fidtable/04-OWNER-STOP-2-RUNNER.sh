#!/usr/bin/env bash
# Phase 4 second owner stop (plan 04-24) — one-command runner for the x86_64
# Linux host, for the gap closure. Runs sections 3-9 of 04-OWNER-STOP-2.md in
# order, fail-fast.
#
# Usage (from the r9 checkout, on the branch):
#   bash .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP-2-RUNNER.sh
#   bash .../04-OWNER-STOP-2-RUNNER.sh /path/to/r9 <expected-tip-sha>
#   PUSH_LOGS=1 bash .../04-OWNER-STOP-2-RUNNER.sh
#   RERUN=1 bash .../04-OWNER-STOP-2-RUNNER.sh
#
# Nothing is fixed here: the only edit is the flake.nix outputHash re-derivation
# that the checklist assigns to the owner, made mechanical by this script.
set -uo pipefail

REPO="${1:-$PWD}"
EXPECTED_TIP="${2:-}"
BRANCH="phase-04/aead-wire-gaps"
RETURN_DIR=".planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/owner-stop-2/return"
STOP_DIR="$HOME/r9-p04-stop2"
PUSH_LOGS="${PUSH_LOGS:-0}"
RERUN="${RERUN:-0}"
REPORT="$STOP_DIR/REPORT.txt"
GATED_SHA=""
HASH_COMMIT="none"
NEW_HASH="unchanged"
GOT=""

say() { printf '\n=== %s ===\n' "$*"; }
fail() {
  printf '\n*** STOP: %s\n' "$*" >&2
  printf 'Partial logs: %s\n' "$STOP_DIR" >&2
  [ -f "$REPORT" ] && { printf '\n--- report so far ---\n' >&2; cat "$REPORT" >&2; }
  exit 1
}

[ "$(uname -sm)" = "Linux x86_64" ] || fail "run this on the x86_64 Linux host (uname -sm = '$(uname -sm)')"
mkdir -p "$STOP_DIR"
: > "$REPORT"

cd "$REPO" 2>/dev/null || fail "no repo at '$REPO' (pass the r9 checkout path as argument 1)"
git rev-parse --git-dir >/dev/null 2>&1 || fail "'$REPO' is not a git checkout"

run_logged() {
  local name="$1"; shift
  local log="$STOP_DIR/$name.log"
  { printf 'HEAD=%s\n' "$(git rev-parse HEAD)"; "$@"; } 2>&1 | tee "$log"
  local rc=${PIPESTATUS[0]}
  printf 'EXIT=%s\n' "$rc" >> "$log"
  printf 'EXIT=%s  (log: %s)\n' "$rc" "$log"
  return "$rc"
}

say "section 2: fetch $BRANCH and confirm the branch tip"
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  git status --porcelain --untracked-files=no >&2
  fail "tracked files are dirty in '$REPO'; commit or discard them first"
fi
git fetch origin || fail "git fetch origin failed"
git switch -C "$BRANCH" "origin/$BRANCH" || fail "cannot switch to '$BRANCH' from origin"
HOST_TIP=$(git rev-parse HEAD)
REMOTE_TIP=$(git rev-parse "origin/$BRANCH")
printf 'pushed-tip:           %s\nhost-tip:             %s\n' "$REMOTE_TIP" "$HOST_TIP" | tee -a "$REPORT"
[ "$HOST_TIP" = "$REMOTE_TIP" ] || fail "host HEAD $HOST_TIP does not equal origin/$BRANCH $REMOTE_TIP"
if [ -n "$EXPECTED_TIP" ] && [ "$HOST_TIP" != "$EXPECTED_TIP" ]; then
  fail "host HEAD is $HOST_TIP but the expected tip is $EXPECTED_TIP"
fi

say "section 2 guard: a round already returned?"
RETURNED=$(git ls-tree -r --name-only "$HOST_TIP" -- "$RETURN_DIR")
if [ -n "$RETURNED" ]; then
  if [ "$RERUN" != "1" ]; then
    fail "a round was already returned on this branch (the tip already contains files under $RETURN_DIR); this checklist is run once per round. Re-run with RERUN=1 only if a second run is unavoidable — it will be disclosed as measured at the return commit"
  fi
  printf 'rerun: yes (a round was already returned at this tip; this run is a second run and is disclosed as measured at the return commit %s)\n' "$HOST_TIP" | tee -a "$REPORT"
else
  printf 'rerun: no\n' >> "$REPORT"
fi

say "section 3: x86_64 host tests first (D-29)"
run_logged host-tests nix develop -c cargo xtask test
[ $? -eq 0 ] || fail "host-tests failed (section 3)"
run_logged host-test-9p nix develop -c cargo xtask test-9p
[ $? -eq 0 ] || fail "host-test-9p failed (section 3)"

say "section 4: sandboxed kernel builds at the tip"
rm -f "$STOP_DIR/tip-kernel-x64" "$STOP_DIR/tip-kernel-virt"
run_logged tip-kernel-x64 nix build .#kernel-x64 -o "$STOP_DIR/tip-kernel-x64"
[ $? -eq 0 ] || fail "nix build .#kernel-x64 failed (section 4)"
run_logged tip-kernel-virt nix build .#kernel-virt -o "$STOP_DIR/tip-kernel-virt"
[ $? -eq 0 ] || fail "nix build .#kernel-virt failed (section 4)"

say "section 5: boot-iso fixed-output hash"
run_logged boot-iso-first nix build .#boot-iso
BOOT1_RC=$?
if [ "$BOOT1_RC" -eq 0 ]; then
  HASH_COMMIT="none (first build succeeded; no flake.nix change)"
  echo "boot-iso-first built green: no hash commit is needed." | tee -a "$REPORT"
  GATED_SHA=$(git rev-parse HEAD)
else
  GOT=$(sed -n 's/.*got:[[:space:]]*\(sha256-[A-Za-z0-9+/=]*\).*/\1/p' "$STOP_DIR/boot-iso-first.log" | tail -1)
  [ -n "$GOT" ] || fail "boot-iso-first failed but no 'got:' sha256 was found in its log; refusing to guess a hash"
  say "re-deriving outputHash -> $GOT (flake.nix only, by the owner)"
  TODAY=$(date -u +%F)
  awk -v new="$GOT" -v d="$TODAY" '
    /^[[:space:]]*outputHash = "sha256-[^"]*";[[:space:]]*$/ {
      ind = $0; sub(/outputHash.*/, "", ind);
      print ind "# " d " (Phase 4 gap closure second owner stop, plan 04-25): re-derived on the x86_64 Linux host for the Phase 4 gap-closure kernel (re-key and fid table fixes under port/src).";
      print ind "outputHash = \"" new "\";";
      n++; next
    }
    { print }
    END { if (n != 1) { printf "expected exactly 1 outputHash line, found %d\n", n > "/dev/stderr"; exit 3 } }
  ' flake.nix > flake.nix.phase4-gaps.new || { rm -f flake.nix.phase4-gaps.new; fail "could not rewrite flake.nix outputHash (see the awk error)"; }
  mv flake.nix.phase4-gaps.new flake.nix
  [ "$(git diff --numstat -- flake.nix | awk '{print $1" "$2}')" = "2 1" ] || fail "flake.nix diff is not the expected +2/-1 (dated comment + hash); inspect 'git diff -- flake.nix' and re-run"
  git add flake.nix || fail "git add flake.nix failed"
  git commit -m "flake: re-derive boot-iso outputHash for the Phase 4 gap closure (re-key and fid table fixes)" -- flake.nix >/dev/null || fail "git commit failed (is a git identity configured on this host?)"
  HASH_COMMIT=$(git rev-parse HEAD)
  FILES_IN_COMMIT=$(git diff-tree --no-commit-id --name-only -r "$HASH_COMMIT" | tr '\n' ' ')
  [ "$FILES_IN_COMMIT" = "flake.nix " ] || fail "the hash commit touches '$FILES_IN_COMMIT', expected only flake.nix"
  printf 'hash-commit: %s\nnew-outputHash: %s\n' "$HASH_COMMIT" "$GOT" | tee -a "$REPORT"
  GATED_SHA="$HASH_COMMIT"
  run_logged boot-iso-after nix build .#boot-iso
  [ $? -eq 0 ] || fail "boot-iso-after failed with the new hash (section 5)"
  run_logged boot-iso-rebuild nix build .#boot-iso --rebuild
  [ $? -eq 0 ] || fail "boot-iso-rebuild failed (section 5)"
  say "pushing the hash commit to $BRANCH"
  git push origin "HEAD:refs/heads/$BRANCH" || fail "pushing the hash commit failed"
fi
NEW_HASH="${GOT:-unchanged}"

if [ -z "$GATED_SHA" ]; then GATED_SHA=$(git rev-parse HEAD); fi
printf 'gated-commit: %s\n' "$GATED_SHA" >> "$REPORT"

guard_gated() {
  local now; now=$(git rev-parse HEAD)
  [ "$now" = "$GATED_SHA" ] || fail "HEAD moved to $now after the gated commit $GATED_SHA — the one-commit rule is void; restart at section 2"
}

say "section 6: boot-to-rio quick gate under KVM"
guard_gated
run_logged gate-kvm nix develop -c cargo xtask gate --arch x86-64 --accel kvm
[ $? -eq 0 ] || fail "KVM boot-to-rio gate failed (section 6)"
grep -q 'GATE PASS quick x86_64' "$STOP_DIR/gate-kvm.log" || fail "gate-kvm exited 0 but printed no 'GATE PASS quick x86_64' line"

say "section 7: TCG gate, native on this host (D-25)"
guard_gated
run_logged gate-tcg nix develop -c cargo xtask gate --arch x86-64 --accel tcg
[ $? -eq 0 ] || fail "TCG gate failed (section 7)"

say "section 8: full gate set"
guard_gated
run_logged gate-full nix develop -c cargo xtask gate --arch x86-64 --full --accel kvm
[ $? -eq 0 ] || fail "full gate set failed (section 8)"
grep -q 'GATE PASS full x86_64-linux' "$STOP_DIR/gate-full.log" || fail "gate-full exited 0 but printed no 'GATE PASS full x86_64-linux' line"

say "section 9: report"
{
  echo "PHASE 4 OWNER STOP 2 RETURN"
  echo "pushed-tip: $REMOTE_TIP"
  echo "host-tip: $HOST_TIP"
  echo "hash-commit: $HASH_COMMIT"
  echo "gated-commit: $GATED_SHA"
  echo "new-outputHash: $NEW_HASH"
  echo "origin/main: $(git rev-parse origin/main)"
  for n in host-tests host-test-9p tip-kernel-x64 tip-kernel-virt boot-iso-first boot-iso-after boot-iso-rebuild gate-kvm gate-tcg gate-full; do
    [ -f "$STOP_DIR/$n.log" ] || continue
    echo
    echo "[$n]"
    grep -E '^(HEAD|EXIT)=' "$STOP_DIR/$n.log" || true
    grep -E 'GATE PASS|ALL STAGES PASS' "$STOP_DIR/$n.log" | tail -2 || true
  done
} >> "$REPORT"
cat "$REPORT"

if [ "$PUSH_LOGS" = "1" ]; then
  say "section 9 (branch route): commit the logs under $RETURN_DIR and push"
  mkdir -p "$RETURN_DIR"
  cp "$STOP_DIR"/*.log "$RETURN_DIR"/ || fail "could not copy the logs into $RETURN_DIR"
  cp "$REPORT" "$RETURN_DIR/REPORT.txt" || fail "could not copy REPORT.txt"
  git add "$RETURN_DIR" || fail "git add of the return logs failed"
  git commit -m "evidence(04-25): owner stop 2 return logs" -- "$RETURN_DIR" >/dev/null || fail "committing the return logs failed"
  git push origin "HEAD:refs/heads/$BRANCH" || fail "pushing the return logs failed"
  echo "return logs pushed on $BRANCH at $(git rev-parse HEAD)"
fi

printf '\nALL STEPS PASSED. Paste REPORT.txt back, or reply "returned via branch" if you used PUSH_LOGS=1.\n'
exit 0
