#!/bin/sh
# 15-01-scoping-proof.sh — the Task-3 scoping proof (15-01; widened to
# the four-file tier union by 15-04's recorded heads-up + 15-07's tier-4
# append).
#
# Proves: under objtype=amd64 WITHOUT COVERAGE_SWEEP, the default
# build's cmd-level surface (cmd/mkfile TARG+DIRS) and every container
# leaf (aux, ip, auth, factotum) evaluate to EXACTLY the
# tier1+tier2+tier3+tier4 membership — and COVERAGE_SWEEP=1 bypasses
# every filter (the 15-07 coverage proof depends on that escape).
#
# Method: mk dry runs INSIDE the Plan 9 namespace (the mkfiles parse
# only there — /sys, /amd64, rc), extracting the mk-EXPANDED variable
# sets from the printed recipes (`mk -n` expands mk variables in
# recipe text, so `for (i in $TARG)` prints the evaluated list), then
# comparing them against the expected sets computed from the committed
# tier data with the SAME awk keep-rules the filters run.
#
#   cmd/mkfile   TARG via `mk -n cmds`    (the $O.% link recipes)
#                DIRS via `mk -n dirs`    (the mk-expanded for-list)
#   aux/ip/auth  TARG via `mk -n safeinstall` (mkmany target none of
#                the three overrides; its for-list prints $TARG)
#                DIRS via `mk -n installdirs` (each container's own
#                override; its for-list prints $DIRS)
#   factotum     TARG via `mk -n install` (not overridden there)
#
# TOOLS: absolute system paths everywhere — the devshell puts Harvey's
# own plan9 tools (hostbin) FIRST on PATH, and plan9 tr/sed/ls would
# garble both sides of the comparison.
#
# usage: nix develop -c sh .planning/phases/15-*/evidence/15-01-scoping-proof.sh
#        (repo root; writes 15-01-scoping-proof.log beside this script)

set -eu

# GNU/system tools ahead of the devshell's hostbin plan9 tools
PATH="/run/current-system/sw/bin:$PATH"
export PATH

HARVEY="${HARVEY:-$PWD}"
cd "$HARVEY"
EVID="$HARVEY/.planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence"
T1="$HARVEY/build/x86_64/tiers/tier1-cmds.txt"
T2="$HARVEY/build/x86_64/tiers/tier2-services.txt"
T3="$HARVEY/build/x86_64/tiers/tier3-gui.txt"
T4="$HARVEY/build/x86_64/tiers/tier4-tail.txt"
LOG="$EVID/15-01-scoping-proof.log"

[ -n "${PLAN9:-}" ] || { echo "error: run inside the nix devshell (\$PLAN9 unset)" >&2; exit 1; }
[ -d "$HARVEY/sys/src" ] || { echo "error: not the harvey checkout" >&2; exit 1; }

# ---------- expected sets (host-side, the filters' own awk rules) ----------

# cmd-level keep: first path component after cmd/ (15-07: the four-file
# union — the filters' own read set; 15-04's recorded heads-up closed)
KEEP_CMD=$(awk '/^cmd\//{ sub(/^cmd\//,""); sub(/[\/: 	].*$/,""); print }' "$T1" "$T2" "$T3" "$T4" | sort -u)
# candidates the cmd sweep would see (NOTSYS/BUGGERED/OUTOFDATE applied as in cmd/mkfile)
CAND_DIRS=$(ls -l sys/src/cmd | sed '/^d/!d; s/.* //;' | grep -Ev '^(sml|dup|unix|old)$|\.')
CAND_TARG=$(ls sys/src/cmd/*.[cy] | sed 's|.*/||; s|\.[cy]$||' | grep -Ev '\.' | sort -u)
EXP_TARG=$(for c in $CAND_TARG; do echo "$KEEP_CMD" | grep -qx "$c" && echo "$c"; done | sort -u | awk '{printf "%s ", $0} END {print ""}')
EXP_DIRS=$(for c in $CAND_DIRS; do echo "$KEEP_CMD" | grep -qx "$c" && echo "$c"; done | sort -u | awk '{printf "%s ", $0} END {print ""}')
# container keeps (the four-file union, the filters' read set)
KEEP_AUX_T=$(awk '/^cmd\/aux:/{ sub(/^cmd\/aux:/,""); sub(/[\/: 	].*$/,""); print }' "$T1" "$T2" "$T3" "$T4" | sort -u | awk '{printf "%s ", $0} END {print ""}')
KEEP_AUX_D=$(awk '/^cmd\/aux\//{ sub(/^cmd\/aux\//,""); sub(/[\/: 	].*$/,""); print }' "$T1" "$T2" "$T3" "$T4" | sort -u | awk '{printf "%s ", $0} END {print ""}')
KEEP_IP=$(awk '/^cmd\/ip\//{ sub(/^cmd\/ip\//,""); sub(/[\/: 	].*$/,""); print }' "$T1" "$T2" "$T3" "$T4" | sort -u)
KEEP_AUTH=$(awk '/^cmd\/auth\//{ sub(/^cmd\/auth\//,""); sub(/[\/: 	].*$/,""); print }' "$T1" "$T2" "$T3" "$T4" | sort -u)
KEEP_FACT=$(awk '/^cmd\/auth\/factotum:/{ sub(/^cmd\/auth\/factotum:/,""); sub(/[\/: 	].*$/,""); print }' "$T1" "$T2" "$T3" "$T4" | sort -u | awk '{printf "%s ", $0} END {print ""}')
# ip/auth candidates from their mkfiles, intersected with the keeps
CAND_IP_T=$(sed 's/^ALLTARG *= *	*/ALLTARG=\
	/' sys/src/cmd/ip/mkfile | sed -n '/^ALLTARG/,/^[^	\\]/p' | sed -n 's/^	//;s/\\$//p' | grep -v '^ALLTARG' | sort -u)
CAND_AUTH_T=$(sed -n '/^ALLTARG/,/^[^	\\]/p' sys/src/cmd/auth/mkfile | sed -n 's/^	//;s/\\$//p' | grep -v '^ALLTARG' | sort -u)
EXP_IP_T=$(for c in $CAND_IP_T; do echo "$KEEP_IP" | grep -qx "$c" && echo "$c"; done | sort -u | awk '{printf "%s ", $0} END {print ""}')
EXP_AUTH_T=$(for c in $CAND_AUTH_T; do echo "$KEEP_AUTH" | grep -qx "$c" && echo "$c"; done | sort -u | awk '{printf "%s ", $0} END {print ""}')

# ---------- namespace side: mk dry-run evaluations ----------
TMPD=$(mktemp -d)
trap 'rm -rf "$TMPD"' EXIT

cat > "$TMPD/proof-guest.sh" <<'GUEST_EOF'
#!/usr/bin/env bash
# the in-namespace mk evaluations (dry runs only — nothing builds).
# Absolute /usr/bin tools: the chroot's PATH starts with $PLAN9/bin.
set -e
export FORCERCFORMK=1 objtype=amd64 cputype=unix HARVEY=/harvey
G=/usr/bin/grep
S=/usr/bin/sed
forlist() {
	# $1 dir, $2 target -> the mk-expanded `for (i in ...)` list, sorted
	(cd "$1" && mk -n "$2" 2>/dev/null | $S -n 's/^for *(i in \(.*\)).*$/\1/p' | head -1 | awk '{n=split($0,a," "); for(i=1;i<=n;i++) print a[i]}' | $S '/^$/d' | sort | awk '{printf "%s ", $0} END {print ""}'; echo)
}
links() {
	# $1 dir, $2 target -> the -o 6.<name> link targets, sorted
	(cd "$1" && mk -n "$2" 2>/dev/null | $G -oE -- '-o 6\.[a-z0-9._]+' | $S 's/^-o 6\.//' | sort -u | awk '{printf "%s ", $0} END {print ""}'; echo)
}
echo "MK:cmd-TARG: $(links /sys/src/cmd cmds)"
echo "MK:cmd-DIRS: $(forlist /sys/src/cmd dirs)"
echo "MK:aux-TARG: $(forlist /sys/src/cmd/aux safeinstall)"
echo "MK:aux-DIRS: $(forlist /sys/src/cmd/aux installdirs)"
echo "MK:aux-TARG-SWEEP-COUNT: $(cd /sys/src/cmd/aux && COVERAGE_SWEEP=1 mk -n safeinstall 2>/dev/null | $S -n 's/^for *(i in \(.*\)).*$/\1/p' | head -1 | wc -w)"
echo "MK:ip-TARG: $(forlist /sys/src/cmd/ip safeinstall)"
echo "MK:ip-DIRS: $(forlist /sys/src/cmd/ip installdirs)"
echo "MK:auth-TARG: $(forlist /sys/src/cmd/auth safeinstall)"
echo "MK:auth-DIRS: $(forlist /sys/src/cmd/auth installdirs)"
echo "MK:fact-TARG: $(forlist /sys/src/cmd/auth/factotum install)"
echo "MK:fact-TARG-SWEEP: $(cd /sys/src/cmd/auth/factotum && COVERAGE_SWEEP=1 mk -n install 2>/dev/null | $S -n 's/^for *(i in \(.*\)).*$/\1/p' | head -1 | awk '{n=split($0,a," "); for(i=1;i<=n;i++) print a[i]}' | $S '/^$/d' | sort | awk '{printf "%s ", $0} END {print ""}'; echo)"
GUEST_EOF

cat > "$TMPD/proof-ns.sh" <<'NS_EOF'
#!/usr/bin/env bash
set -e
NSX=$(mktemp -d)
mount -t tmpfs tmpfs "$NSX"
mkdir -p "$NSX"/{sys,amd64,rc,etc,tmp,harvey,unix,nix/store,run,dev,usr,bin}
mount --bind "$HARVEY_SYS" "$NSX/sys"
mount --bind "$HARVEY_AMD64" "$NSX/amd64"
mount --bind "$HARVEY_RC" "$NSX/rc"
mount --bind "$HARVEY_UNIX" "$NSX/unix"
mount --bind /etc "$NSX/etc"
mount --bind /tmp "$NSX/tmp"
mount --bind "$HARVEY_REAL" "$NSX/harvey"
mount --bind /nix/store "$NSX/nix/store"
ln -sfn "$(readlink /run/current-system)" "$NSX/run/current-system"
ln -sfn ../run/current-system/sw/bin "$NSX/usr/bin"
touch "$NSX/bin/rc"
mount --bind "$PLAN9/bin/rc" "$NSX/bin/rc"
exec chroot "$NSX" /usr/bin/env \
	PATH="$PLAN9/bin:$PATH" \
	HOME="$HOME" \
	FORCERCFORMK=1 objtype=amd64 cputype=unix \
	bash "$1"
NS_EOF

{
echo "=== 15-01 scoping proof $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
echo "expected (computed from the tier data by the filters' own awk keep-rules):"
echo "EXP:cmd-TARG: $EXP_TARG"
echo "EXP:cmd-DIRS: cc $EXP_DIRS"
echo "EXP:aux-TARG: $KEEP_AUX_T"
echo "EXP:aux-DIRS: $KEEP_AUX_D"
echo "EXP:ip-TARG: $EXP_IP_T"
echo "EXP:auth-TARG: $EXP_AUTH_T"
echo "EXP:fact-TARG: $KEEP_FACT"
echo
echo "mk-evaluated (namespace dry runs):"
HARVEY_SYS="$HARVEY/sys" HARVEY_AMD64="$HARVEY/amd64" HARVEY_RC="$HARVEY/rc" \
	HARVEY_UNIX="$HARVEY/unix" HARVEY_REAL="$HARVEY" \
	unshare -Ur -m bash "$TMPD/proof-ns.sh" "$TMPD/proof-guest.sh"
} > "$LOG" 2>&1

# ---------- compare ----------
fail=0
check() {
	# $1 = MK: label, $2 = expected words
	got=$(sed -n "s/^$1: *//p" "$LOG" | head -1 | awk '{n=split($0,a," "); for(i=1;i<=n;i++) print a[i]}' | sed '/^$/d' | sort | awk '{printf "%s ", $0} END {print ""}')
	want=$(echo "$2" | awk '{n=split($0,a," "); for(i=1;i<=n;i++) print a[i]}' | sed '/^$/d' | sort | awk '{printf "%s ", $0} END {print ""}')
	if [ "$got" = "$want" ]; then
		echo "PROOF-PASS $1"
	else
		echo "PROOF-FAIL $1:"
		echo "  want: $want"
		echo "  got:  $got"
		fail=1
	fi
}

check MK:cmd-TARG "$EXP_TARG"
check MK:cmd-DIRS "cc $EXP_DIRS"
check MK:aux-TARG "$KEEP_AUX_T"
check MK:aux-DIRS "$KEEP_AUX_D"
check MK:ip-TARG "$EXP_IP_T"
check MK:auth-TARG "$EXP_AUTH_T"
check MK:fact-TARG "$KEEP_FACT"
# the escapes: the aux sweep count must EQUAL aux's full ALLTARG
# membership (parsed from the mkfile itself), and factotum's sweep
# shows the full fgui pair — COVERAGE_SWEEP=1 bypasses every filter
AUX_ALL=$(awk '/^ALLTARG/{f=1;next} /^$/{f=0} f && /^\t/{c++} END{print c+0}' sys/src/cmd/aux/mkfile)
AUX_SWEEP=$(sed -n 's/^MK:aux-TARG-SWEEP-COUNT: *//p' "$LOG" | head -1 | tr -d ' ')
if [ "${AUX_SWEEP:-0}" = "$AUX_ALL" ]; then
	echo "PROOF-PASS MK:aux-TARG-SWEEP-COUNT ($AUX_SWEEP == aux ALLTARG's $AUX_ALL — COVERAGE_SWEEP=1 bypasses the aux leaf filter)"
else
	echo "PROOF-FAIL MK:aux-TARG-SWEEP-COUNT: got '$AUX_SWEEP'"
	fail=1
fi
check MK:fact-TARG-SWEEP "factotum fgui"

cat "$LOG"
if [ "$fail" -eq 0 ]; then
	echo "SCOPING-PROOF-OK"
else
	echo "SCOPING-PROOF-FAIL" >&2
	exit 1
fi
