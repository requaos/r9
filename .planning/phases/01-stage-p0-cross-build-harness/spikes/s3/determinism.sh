#!/usr/bin/env bash
# s3/determinism.sh — A12: are two mkimage.sh bakes of one tree with a
# pinned KERNDATE byte-identical?
#
# Run on the owner's x86_64 Linux host (Plan 06) against the writable
# scratch Harvey copy left by one full s3/rio-gate.sh run.  Harvey's
# x86_64 mkimage.sh pins every staged mtime to KERNDATE and passes the
# same KERNDATE to mkpaqfs, whose header time rides it
# (build/x86_64/mkimage.sh:105-110, 243-250) — so the six non-auth
# images must be byte-identical across two bakes.  The auth and cpu
# images embed a FRESH bake-time p9sk1 domain on every run by design
# (mkimage.sh:1290-1305, 1848-1856), so they are expected to differ;
# for those this script also compares the staged member lists (the
# bake logs' `auth manifest:` / `cpu manifest (§15):` lines) and the
# image sizes, and records which matched.
#
# What it does:
#   1. sha256 of every build/x86_64/out/*.paq -> $S3_OUT/bake1.sha256
#   2. moves those paqs aside (out/determinism-bake1/) inside the
#      scratch copy
#   3. reruns `sh build/x86_64/mkimage.sh` with the same KERNDATE
#   4. sha256 -> $S3_OUT/bake2.sha256
#   5. prints one `DETERMINISM <image> IDENTICAL|DIFFERENT` line per
#      image, the auth/cpu notes, then S3-DETERMINISM-DONE
#
# Refusals: a real Harvey checkout ($HARVEY/.git), a /nix/store path,
# or an incomplete first-bake set.  Credentials: the script never
# copies build/x86_64/out/auth-creds.txt (or authfeed.txt) anywhere —
# it only hashes image files and reads the bake logs.
#
# Env:
#   HARVEY        REQUIRED — writable scratch Harvey copy after one
#                 full rio-gate.sh run (out/ populated, no .git).
#   S3_OUT        REQUIRED — output/evidence directory (created).
#   KERNDATE      bake timestamp (default 1795382101, the bd7a698
#                 commit time — the same value Plan 06 passes the gate).
#   S3_BAKE1_LOG  optional — the first bake's log; defaults to
#                 $S3_OUT/bake.log (rio-gate.sh's tee target).
#
# usage (owner host, r9 checkout root):
#   HARVEY=<scratch> S3_OUT=<dir> KERNDATE=1795382101 \
#     nix develop -c bash .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/determinism.sh

set -euo pipefail

HARVEY="${HARVEY:-}"
S3_OUT="${S3_OUT:-}"
KERNDATE="${KERNDATE:-1795382101}"

err() { echo "S3-ERROR: $*" >&2; exit 1; }

[ -n "$HARVEY" ] || err "harvey-unset: set HARVEY to the writable scratch Harvey copy"
[ -n "$S3_OUT" ] || err "s3-out-unset: set S3_OUT to the output/evidence directory"
case "$HARVEY" in
	/nix/store/*) err "harvey-in-store: HARVEY=$HARVEY is a read-only store path" ;;
esac
[ ! -e "$HARVEY/.git" ] || err "harvey-is-checkout: HARVEY=$HARVEY has a .git entry; use the scratch copy, never a real checkout"
[ -d "$HARVEY/build/x86_64" ] || err "not-a-harvey-tree: $HARVEY/build/x86_64 missing"
[ -f "$HARVEY/build/x86_64/mkimage.sh" ] || err "not-a-harvey-tree: $HARVEY/build/x86_64/mkimage.sh missing"
case "$S3_OUT" in
	/nix/store/*) err "s3-out-in-store: S3_OUT=$S3_OUT is a read-only store path" ;;
esac
[[ "$KERNDATE" =~ ^[0-9]+$ ]] || err "kerndate-invalid: KERNDATE=$KERNDATE is not an epoch (want e.g. 1795382101)"

OUT="$HARVEY/build/x86_64/out"
[ -d "$OUT" ] || err "no-out: $OUT missing — run s3/rio-gate.sh once first (a full bake)"
mkdir -p "$S3_OUT"

# The nine x86_64 bake contracts (mkimage.sh's IMAGE variables).
IMAGES="bootfs.paq bootfs-chain.paq bootfs-tier.paq bootfs-net.paq bootfs-services.paq bootfs-churn.paq bootfs-substrate.paq bootfs-auth.paq bootfs-cpu.paq"

for img in $IMAGES; do
	[ -s "$OUT/$img" ] || err "incomplete-bake: $OUT/$img missing — run the full rio gate first so both bakes start from the same tree"
done

sha256_of() {
	if command -v sha256sum >/dev/null 2>&1; then
		sha256sum "$1" | awk '{print $1}'
	else
		shasum -a 256 "$1" | awk '{print $1}'
	fi
}

record_hashes() {
	dest=$1
	: > "$dest"
	for img in $IMAGES; do
		printf '%s  %s\n' "$(sha256_of "$OUT/$img")" "$img" >> "$dest"
	done
}

BAKE1_DIR="$OUT/determinism-bake1"
BAKE2_LOG="$S3_OUT/determinism-bake2.log"
BAKE2_OK=0

# The first bake must have used the SAME KERNDATE, or every image would
# differ for the wrong reason.  mkimage prints the value in its PASS lines.
BAKE1_LOG="${S3_BAKE1_LOG:-$S3_OUT/bake.log}"
if [ -f "$BAKE1_LOG" ]; then
	log_kerndate=$(grep -oE 'KERNDATE [0-9]+' "$BAKE1_LOG" | head -1 | awk '{print $2}')
	if [ -n "$log_kerndate" ] && [ "$log_kerndate" != "$KERNDATE" ]; then
		err "bake1-kerndate-mismatch: the first bake used KERNDATE=$log_kerndate, not $KERNDATE; re-run s3/rio-gate.sh with KERNDATE=$KERNDATE"
	fi
fi

# On any failure after the move-aside, put the bake1 images back so a
# retry starts from the previous state.  Only .paq files move/copy here;
# no credential artifact is ever read or written by this script.
# shellcheck disable=SC2329  # invoked via trap below
on_exit() {
	status=$?
	if [ "$BAKE2_OK" != "1" ]; then
		for img in $IMAGES; do
			if [ -f "$BAKE1_DIR/$img" ] && [ ! -e "$OUT/$img" ]; then
				cp "$BAKE1_DIR/$img" "$OUT/$img"
			fi
		done
	fi
	exit "$status"
}
trap on_exit EXIT

echo "== determinism: scratch=$HARVEY KERNDATE=$KERNDATE"
echo "== bake 1 hashes -> $S3_OUT/bake1.sha256"
record_hashes "$S3_OUT/bake1.sha256"

mkdir -p "$BAKE1_DIR"
for img in $IMAGES; do
	mv "$OUT/$img" "$BAKE1_DIR/$img"
done

echo "== bake 2: sh build/x86_64/mkimage.sh (log: $BAKE2_LOG)"
if ! (cd "$HARVEY" && HARVEY="$HARVEY" KERNDATE="$KERNDATE" sh build/x86_64/mkimage.sh) > "$BAKE2_LOG" 2>&1; then
	echo "S3-ERROR: bake2-failed — the second bake did not complete; tail of $BAKE2_LOG follows" >&2
	tail -n 20 "$BAKE2_LOG" >&2 || true
	exit 1
fi
for img in $IMAGES; do
	[ -s "$OUT/$img" ] || err "incomplete-bake2: $OUT/$img missing after the second bake"
done
BAKE2_OK=1

echo "== bake 2 hashes -> $S3_OUT/bake2.sha256"
record_hashes "$S3_OUT/bake2.sha256"

echo "== verdicts"
for img in $IMAGES; do
	h1=$(awk -v i="$img" '$2 == i { print $1; exit }' "$S3_OUT/bake1.sha256")
	h2=$(awk -v i="$img" '$2 == i { print $1; exit }' "$S3_OUT/bake2.sha256")
	if [ "$h1" = "$h2" ]; then
		printf 'DETERMINISM %s IDENTICAL\n' "$img"
	else
		printf 'DETERMINISM %s DIFFERENT\n' "$img"
	fi
done

# --- auth and cpu: expected to differ; compare the secondary evidence --
if [ ! -f "$BAKE1_LOG" ]; then
	BAKE1_LOG=""
fi

manifest_compare() {
	# $1 image, $2 grep prefix, $3 note label
	img=$1
	prefix=$2
	label=$3
	if [ -z "$BAKE1_LOG" ]; then
		printf 'DETERMINISM-NOTE %s %s member-list=UNAVAILABLE (no first-bake log at %s)\n' "$img" "$label" "${S3_BAKE1_LOG:-$S3_OUT/bake.log}"
		return
	fi
	b1="$S3_OUT/bake1-$label-manifest.txt"
	b2="$S3_OUT/bake2-$label-manifest.txt"
	grep -E "^$prefix" "$BAKE1_LOG" > "$b1" || true
	grep -E "^$prefix" "$BAKE2_LOG" > "$b2" || true
	if cmp -s "$b1" "$b2"; then
		mlist=IDENTICAL
	else
		mlist=DIFFERENT
	fi
	s1=$(wc -c < "$BAKE1_DIR/$img" | tr -d ' ')
	s2=$(wc -c < "$OUT/$img" | tr -d ' ')
	if [ "$s1" = "$s2" ]; then
		sizecmp=IDENTICAL
	else
		sizecmp=DIFFERENT
	fi
	printf 'DETERMINISM-NOTE %s %s member-list=%s (%s lines) size=%s (%s vs %s bytes; expected: fresh bake-time p9sk1 domain)\n' \
		"$img" "$label" "$mlist" "$(wc -l < "$b1" | tr -d ' ')" "$sizecmp" "$s1" "$s2"
}

manifest_compare bootfs-auth.paq 'auth manifest' auth
manifest_compare bootfs-cpu.paq 'cpu manifest' cpu

echo "S3-DETERMINISM-DONE"
