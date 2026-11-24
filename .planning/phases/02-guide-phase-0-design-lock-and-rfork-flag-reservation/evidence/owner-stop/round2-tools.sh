#!/usr/bin/env bash
# Phase 2 / plan 02-06 round 2 — run every stop check at the gated commit,
# stamp each log with HEAD= and EXIT=, then push the logs as a return commit.
set -u
set -o pipefail

REPO="${R9_REPO:-$HOME/code/r9}"
if [ ! -f "$REPO/flake.nix" ]; then REPO="$(pwd)"; fi
BASE="$HOME/r9-p02-stop"
GATED="4624ea2f0cb45c81870dcc881d544d7d36a2e0bf"
PHASE_DIR=".planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation"
RETURN_DIR="$PHASE_DIR/evidence/owner-stop/return/round2"

cd "$REPO" || { echo "FATAL: no r9 checkout at $REPO (set R9_REPO)"; exit 1; }
[ -f flake.nix ] || { echo "FATAL: $REPO has no flake.nix"; exit 1; }
echo "== repo: $REPO"
git fetch origin || exit 1

git worktree remove --force "$BASE/run" >/dev/null 2>&1 || true
git worktree remove --force "$BASE/branch" >/dev/null 2>&1 || true
rm -rf "$BASE/run" "$BASE/branch" "$BASE/kernel" "$BASE/full" "$BASE/logs"
git worktree prune
mkdir -p "$BASE/logs"

git worktree add --detach "$BASE/run" "$GATED" || exit 1
cd "$BASE/run" || exit 1
[ "$(git rev-parse HEAD)" = "$GATED" ] || { echo "FATAL: worktree is not at $GATED"; exit 1; }

run_logged() {
  name="$1"; shift
  log="$BASE/logs/$name.log"
  { printf 'HEAD=%s\n' "$(git rev-parse HEAD)"; "$@"; } 2>&1 | tee "$log"
  rc=${PIPESTATUS[0]}
  printf 'EXIT=%s\n' "$rc" | tee -a "$log"
  return "$rc"
}

{
  echo "venue: $(uname -sm)"
  echo "gated: $(git rev-parse HEAD)"
  nix --version
} | tee "$BASE/logs/venue.log"

FAILED=""

echo ""
echo "== 3. x86_64 host tests =="
run_logged host-tests nix develop -c cargo xtask test --verbose || FAILED=host-tests

if [ -z "$FAILED" ]; then
  echo ""
  echo "== 4a. boot-iso build against the committed outputHash =="
  run_logged boot-iso nix build .#boot-iso || FAILED=boot-iso
fi

if [ -z "$FAILED" ]; then
  echo ""
  echo "== 4b. witness kernel step (plain build, then --rebuild) =="
  export R9_WITNESS_WORK="$BASE/kernel"
  export R9_WITNESS_STEPS="kernel"
  run_logged kernel-step bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh || FAILED=kernel-step
  unset R9_WITNESS_WORK R9_WITNESS_STEPS
fi

if [ -z "$FAILED" ]; then
  echo ""
  echo "== 5. KVM quick gate (once, direct) =="
  run_logged gate-kvm nix develop -c cargo xtask gate --arch x86-64 --accel kvm || FAILED=gate-kvm
fi

if [ -z "$FAILED" ]; then
  echo ""
  echo "== 6. TCG gate =="
  run_logged gate-tcg nix develop -c cargo xtask gate --arch x86-64 --accel tcg || FAILED=gate-tcg
fi

if [ -z "$FAILED" ]; then
  echo ""
  echo "== 7. full witness set =="
  export R9_WITNESS_WORK="$BASE/full"
  export R9_WITNESS_STEPS="full"
  run_logged gate-full bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh || FAILED=gate-full
  unset R9_WITNESS_WORK R9_WITNESS_STEPS
fi

echo ""
echo "===== summary ====="
for f in venue host-tests boot-iso kernel-step gate-kvm gate-tcg gate-full; do
  log="$BASE/logs/$f.log"
  [ -f "$log" ] || continue
  printf '\n--- %s ---\n' "$f"
  grep -E '^(HEAD=|EXIT=)' "$log" || true
  grep -E 'rfork_reserved_bits_are_unsupported|KERNEL-STORE|KERNEL-BOOTISO-(STORE|BUILD-OK|REBUILD-OK|BUILD-FAIL|REBUILD-FAIL)|kernel-x64\.elf|GATE PASS (quick|full)|R9-WITNESS-RUN-OK|hash mismatch|got:' "$log" | tail -8 || true
done

if [ -n "$FAILED" ]; then
  echo ""
  echo "STOP: '$FAILED' failed. Logs are in $BASE/logs — nothing was committed or pushed."
  echo "Paste the failing log's HEAD=/EXIT= and the decisive lines."
  exit 1
fi

echo ""
echo "== 8. committing the logs on phase-02/design-lock =="
git worktree add --detach "$BASE/branch" origin/phase-02/design-lock || exit 1
cd "$BASE/branch" || exit 1
mkdir -p "$RETURN_DIR"
cp "$BASE"/logs/*.log "$RETURN_DIR"/
git add -- "$RETURN_DIR"
if git diff --cached --name-only | grep -v "^$RETURN_DIR/"; then
  echo "FATAL: files outside $RETURN_DIR staged — refusing to commit"
  exit 1
fi
git commit -m "evidence(02-06): owner stop round 2 logs at the gated commit" || exit 1
NEW=$(git rev-parse HEAD)
git push origin HEAD:refs/heads/phase-02/design-lock || {
  echo "PUSH FAILED. Return commit is $NEW; push it manually with:"
  echo "  git push origin $NEW:refs/heads/phase-02/design-lock"
  exit 1
}
echo ""
echo "PUSHED-RETURN: $NEW"
echo "GATED-COMMIT: $GATED"
echo "Done. Reply: stop done; hash commit 4624ea2; returned via branch"
