#!/usr/bin/env bash
# Rebuild the Phase 2 return commit from the round-2 wrapper logs and replace
# the misplaced return-p02-stop commit. Keeps gated commit 4624ea2 and the
# earlier return commit a0454ff; force-with-lease only for the branch tip.
set -u
set -o pipefail

REPO="${R9_REPO:-$HOME/code/r9}"
[ -f "$REPO/flake.nix" ] || REPO="$(pwd)"
BASE="$HOME/r9-p02-stop"
BRANCH="phase-02/design-lock"
KEEP="a0454ffed4f80473866239c4164a1918bf0d216c"
GATED="4624ea2f0cb45c81870dcc881d544d7d36a2e0bf"
OLD="2aaa28eeeee86fef75738f2988a3cd0576adde7d"
RET=".planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/owner-stop/return/round2"
LOGS="$BASE/logs"

cd "$REPO" || { echo "FATAL: no r9 checkout at $REPO"; exit 1; }
git fetch origin "$BRANCH" || exit 1
REMOTE=$(git rev-parse "origin/$BRANCH")
echo "== remote tip: $REMOTE"
if [ "$REMOTE" != "$OLD" ] && [ "$REMOTE" != "$KEEP" ]; then
  echo "FATAL: remote moved to $REMOTE; expected $OLD or $KEEP"
  exit 1
fi
git merge-base --is-ancestor "$KEEP" "$REMOTE" || { echo "FATAL: $KEEP not an ancestor of remote"; exit 1; }
EXTRA=$(git rev-list --count "$KEEP..$REMOTE")
if [ "$EXTRA" -gt 1 ]; then
  echo "FATAL: more than one commit above $KEEP:"
  git log --oneline "$KEEP..$REMOTE"
  exit 1
fi

echo "== checking round-2 logs in $LOGS"
[ -d "$LOGS" ] || { echo "FATAL: $LOGS missing — rerun the round-2 script"; exit 1; }
BAD=0
if [ ! -f "$LOGS/venue.log" ] || ! grep -q "gated: $GATED" "$LOGS/venue.log"; then
  echo "MISSING/BAD venue.log"; BAD=1
fi
for f in host-tests boot-iso kernel-step gate-kvm gate-tcg gate-full; do
  p="$LOGS/$f.log"
  if [ ! -f "$p" ]; then echo "MISSING $p"; BAD=1; continue; fi
  head -1 "$p" | grep -q "^HEAD=$GATED$" || { echo "BAD HEAD in $f: $(head -1 "$p")"; BAD=1; }
  tail -1 "$p" | grep -q '^EXIT=0$' || { echo "BAD EXIT in $f: $(tail -1 "$p")"; BAD=1; }
done
[ "$BAD" -eq 0 ] || { echo "FATAL: logs are missing or incomplete; nothing changed."; exit 1; }

for pair in "host-tests|rfork_reserved_bits_are_unsupported" \
            "kernel-step|KERNEL-BOOTISO-BUILD-OK" \
            "kernel-step|KERNEL-BOOTISO-REBUILD-OK" \
            "gate-kvm|GATE PASS quick" \
            "gate-tcg|GATE PASS quick" \
            "gate-full|GATE PASS full" \
            "gate-full|R9-WITNESS-RUN-OK"; do
  f=${pair%%|*}; pat=${pair#*|}
  grep -q "$pat" "$LOGS/$f.log" || { echo "FATAL: $f.log lacks '$pat'"; BAD=1; }
done
[ "$BAD" -eq 0 ] || { echo "FATAL: a decisive line is missing; nothing changed."; exit 1; }

echo "== building the return commit on $KEEP"
git worktree remove --force "$BASE/commit" >/dev/null 2>&1 || true
rm -rf "$BASE/commit"
git worktree prune
git worktree add --detach "$BASE/commit" "$KEEP" || exit 1
cd "$BASE/commit" || exit 1
mkdir -p "$RET/full"
cp "$LOGS"/*.log "$RET"/
for w in "$BASE/WITNESS-RETURN.md" "$BASE/full/WITNESS-RETURN.md"; do
  if [ -f "$w" ]; then cp "$w" "$RET/WITNESS-RETURN.md"; break; fi
done
if [ -d "$BASE/full" ]; then cp -R "$BASE/full/." "$RET/full/"; fi
git add -- "$RET"
if git diff --cached --name-only | grep -v "^$RET/"; then
  echo "FATAL: staged path outside $RET — refusing to commit"
  exit 1
fi
git commit -m "evidence(02-06): owner stop round 2 logs at the gated commit" || exit 1
NEW=$(git rev-parse HEAD)
git push --force-with-lease="refs/heads/$BRANCH:$REMOTE" origin "$NEW:refs/heads/$BRANCH" || {
  echo "PUSH FAILED. Return commit is $NEW; push manually with:"
  echo "  git push --force-with-lease=$BRANCH:$REMOTE origin $NEW:refs/heads/$BRANCH"
  exit 1
}

echo ""
echo "PUSHED-RETURN: $NEW"
echo "GATED-COMMIT: $GATED"
echo "The misplaced commit $OLD is gone from the branch; $KEEP history is intact."
cd "$REPO" && git worktree remove --force "$BASE/commit" >/dev/null 2>&1 || true
echo "Done. Reply: stop done; returned via branch"
