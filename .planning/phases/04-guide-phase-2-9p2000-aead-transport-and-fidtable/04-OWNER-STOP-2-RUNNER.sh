#!/usr/bin/env bash
# Phase 4 gap-closure owner stop 2 - runner v2.
# Fixes v1's section 5: a plain `nix build .#boot-iso` returns the fixed-output path
# already in the store from the first stop, so v1 could report "no hash change" without
# ever rebuilding. This runner ALWAYS runs `--rebuild`, and commits the new hash when it
# mismatches. It pushes its own return logs (git is the only channel back).
#
# Run on the x86_64 Linux host, from the r9 checkout:
#   bash .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP-2-RUNNER.sh
set -uo pipefail

REPO="${1:-$PWD}"
BRANCH="phase-04/aead-wire-gaps"
STOP_DIR="$HOME/r9-p04-stop2/round2"
REPORT="$STOP_DIR/REPORT.txt"
PHASE_DIR=".planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable"
RETURN_DIR="$PHASE_DIR/evidence/owner-stop-2/return/round2"
SRC="port crypto ds xtask aarch64 x86_64 riscv64 Cargo.lock flake.nix"
GATED=""
HASH_COMMIT="none"
NEW_HASH="unchanged"
GOT=""

say() { printf '\n=== %s ===\n' "$*"; }
fail() {
  printf '\n*** STOP: %s\n' "$*" >&2
  [ -f "$REPORT" ] && { printf '\n--- report so far ---\n' >&2; cat "$REPORT" >&2; }
  printf 'Logs: %s\n' "$STOP_DIR" >&2
  exit 1
}

[ "$(uname -sm)" = "Linux x86_64" ] || fail "run this on the x86_64 Linux host (uname -sm = '$(uname -sm)')"
mkdir -p "$STOP_DIR"
: > "$REPORT"

cd "$REPO" 2>/dev/null || fail "no repo at '$REPO' (pass the checkout path as argument 1)"
git rev-parse --git-dir >/dev/null 2>&1 || fail "'$REPO' is not a git checkout"
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  git status --porcelain --untracked-files=no >&2
  fail "tracked files are dirty; commit or discard them first"
fi

run_logged() {
  local name="$1"; shift
  local log="$STOP_DIR/$name.log"
  { printf 'HEAD=%s\n' "$(git rev-parse HEAD)"; "$@"; } 2>&1 | tee "$log"
  local rc=${PIPESTATUS[0]}
  printf 'EXIT=%s\n' "$rc" | tee -a "$log"
  return "$rc"
}

say "section 2: fetch $BRANCH and confirm the tip"
git fetch origin || fail "git fetch failed"
git switch -C "$BRANCH" "origin/$BRANCH" || fail "cannot switch to '$BRANCH'"
TIP=$(git rev-parse HEAD)
REMOTE=$(git rev-parse "origin/$BRANCH")
[ "$TIP" = "$REMOTE" ] || fail "host HEAD $TIP != origin/$BRANCH $REMOTE"
printf 'pushed-tip: %s\nhost-tip:   %s\n' "$TIP" "$TIP" | tee -a "$REPORT"

say "section 3: host tests (D-29: before the hash step)"
run_logged host-tests nix develop -c cargo xtask test
[ $? -eq 0 ] || fail "host-tests failed"
run_logged host-test-9p nix develop -c cargo xtask test-9p
[ $? -eq 0 ] || fail "host-test-9p failed"

say "section 4: sandboxed kernel builds at the tip"
rm -f "$STOP_DIR/tip-kernel-x64" "$STOP_DIR/tip-kernel-virt"
run_logged tip-kernel-x64 nix build .#kernel-x64 -o "$STOP_DIR/tip-kernel-x64"
[ $? -eq 0 ] || fail "nix build .#kernel-x64 failed"
run_logged tip-kernel-virt nix build .#kernel-virt -o "$STOP_DIR/tip-kernel-virt"
[ $? -eq 0 ] || fail "nix build .#kernel-virt failed"

say "section 5: boot-iso fixed-output hash (rebuild-proving)"
run_logged boot-iso-first nix build .#boot-iso
say "forcing the comparison that a plain build cannot make: --rebuild"
run_logged boot-iso-rebuild nix build .#boot-iso --rebuild
REBUILD_RC=$?
if [ "$REBUILD_RC" -ne 0 ]; then
  GOT=$(sed -n 's/.*got:[[:space:]]*\(sha256-[A-Za-z0-9+/=]*\).*/\1/p' "$STOP_DIR/boot-iso-rebuild.log" | tail -1)
  [ -n "$GOT" ] || fail "--rebuild failed but no 'got:' sha256 was in its log"
  say "hash is stale: committed value does not reproduce; new value $GOT"
  awk -v new="$GOT" -v d="$(date -u +%F)" '
    /^[[:space:]]*outputHash = "sha256-[^"]*";[[:space:]]*$/ {
      ind = $0; sub(/outputHash.*/, "", ind);
      print ind "# " d " (Phase 4 second owner stop, plan 04-25): re-derived on the x86_64 Linux host for the Phase 4 gap-closure kernel (re-key and fid table fixes under port/src).";
      print ind "outputHash = \"" new "\";";
      n++; next
    }
    { print }
    END { if (n != 1) { printf "expected exactly 1 outputHash line, found %d\n", n > "/dev/stderr"; exit 3 } }
  ' flake.nix > flake.nix.p4g.new || { rm -f flake.nix.p4g.new; fail "could not rewrite flake.nix outputHash"; }
  mv flake.nix.p4g.new flake.nix
  [ "$(git diff --numstat -- flake.nix | awk '{print $1" "$2}')" = "2 1" ] || fail "flake.nix diff is not the expected +2/-1"
  git add flake.nix || fail "git add flake.nix failed"
  git commit -m "flake: re-derive boot-iso outputHash for the Phase 4 gap closure (re-key and fid table fixes)" -- flake.nix >/dev/null || fail "git commit failed (git identity configured?)"
  HASH_COMMIT=$(git rev-parse HEAD)
  [ "$(git diff-tree --no-commit-id --name-only -r "$HASH_COMMIT" | tr '\n' ' ')" = "flake.nix " ] || fail "hash commit touches more than flake.nix"
  printf 'hash-commit: %s\nnew-outputHash: %s\n' "$HASH_COMMIT" "$GOT" | tee -a "$REPORT"
  git push origin "HEAD:refs/heads/$BRANCH" || fail "pushing the hash commit failed"
  run_logged boot-iso-after nix build .#boot-iso
  [ $? -eq 0 ] || fail "boot-iso-after failed with the new hash"
  run_logged boot-iso-rebuild2 nix build .#boot-iso --rebuild
  [ $? -eq 0 ] || fail "boot-iso-rebuild2 failed"
  GATED="$HASH_COMMIT"
else
  printf 'hash-commit: none (--rebuild reproduced the committed hash)\nnew-outputHash: unchanged\n' | tee -a "$REPORT"
  GATED="$TIP"
fi
NEW_HASH="${GOT:-unchanged}"

say "sections 6-8: gates at the gated commit $GATED"
NOW=$(git rev-parse HEAD)
[ "$NOW" = "$GATED" ] || fail "HEAD moved to $NOW after the gated commit $GATED - the one-commit rule is void; restart at section 2"
run_logged gate-kvm nix develop -c cargo xtask gate --arch x86-64 --accel kvm
[ $? -eq 0 ] || fail "KVM gate failed"
grep -q 'GATE PASS quick x86_64' "$STOP_DIR/gate-kvm.log" || fail "gate-kvm printed no GATE PASS quick x86_64 line"
run_logged gate-tcg nix develop -c cargo xtask gate --arch x86-64 --accel tcg
[ $? -eq 0 ] || fail "TCG gate failed"
run_logged gate-full nix develop -c cargo xtask gate --arch x86-64 --full --accel kvm
[ $? -eq 0 ] || fail "full gate set failed"
grep -q 'GATE PASS full x86_64-linux' "$STOP_DIR/gate-full.log" || fail "gate-full printed no GATE PASS full x86_64-linux line"

say "section 9: report and push the return logs"
{
  echo "PHASE 4 OWNER STOP 2 - ROUND 2 RETURN"
  echo "pushed-tip: $TIP"
  echo "gated-commit: $GATED"
  echo "hash-commit: $HASH_COMMIT"
  echo "new-outputHash: $NEW_HASH"
  echo "origin/main: $(git rev-parse origin/main)"
  for n in host-tests host-test-9p tip-kernel-x64 tip-kernel-virt boot-iso-first boot-iso-rebuild boot-iso-after boot-iso-rebuild2 gate-kvm gate-tcg gate-full; do
    [ -f "$STOP_DIR/$n.log" ] || continue
    echo
    echo "[$n]"
    grep -E '^(HEAD|EXIT)=' "$STOP_DIR/$n.log" || true
    grep -E 'GATE PASS|specified:|got:' "$STOP_DIR/$n.log" | tail -2 || true
  done
} >> "$REPORT"
cat "$REPORT"

mkdir -p "$RETURN_DIR"
cp "$STOP_DIR"/*.log "$REPORT" "$RETURN_DIR"/ || fail "could not copy the logs into $RETURN_DIR"
git add "$RETURN_DIR" || fail "git add of the return logs failed"
git commit -m "evidence(04-25): owner stop 2 round-2 return logs" -- "$RETURN_DIR" >/dev/null || fail "committing the return logs failed"
git push origin "HEAD:refs/heads/$BRANCH" || fail "pushing the return logs failed"
printf '\nRETURN PUSHED: %s\n' "$(git rev-parse HEAD)"
