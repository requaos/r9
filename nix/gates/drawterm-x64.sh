#!/usr/bin/env bash
# nix/gates/drawterm-x64.sh — r9's drawterm session wrapper (plan 01-31,
# D-20/D-08, HARN-08; moved from Harvey bd7a698's nix/drawterm-x64.sh,
# the 19-06 wrapper, and rewired onto r9's own cpu image).
#
# One command, pinned connection: the vendored in-tree drawterm (built
# by Harvey's flake package with NO fetch and NO patchPhase, D-02) is
# exec'd with the connection pinned on argv via -O (the OQ-2
# settlement): argv is the single source of connection parameters, so
# this wrapper SCRUBS the credential-bearing environment first — the
# $cpu/$auth defaults are a connection-hijack surface (research
# Pitfall 3 / T-19-12), never a fallback.
#
# The pinned values are the committed hostfwd pair's loopback side (the
# Phase-16 D-04 data the qemu emitter serves): host 17010 -> guest
# 17010 (cpu), host 5670 -> guest 567 (auth) — loopback-literal form
# per the 18-05 idiom ($attr cs translation reds on r9; name the 3-part
# tcp!host!port auth address and the bare cpu host verbatim).  The auth
# address carries its service, so netmkaddr passes it through as-is.
# user=glenda is the Phase-18 baked demo user (18-D-12) and the only
# staged home profile (the rio family logs in as glenda; the DEMO
# domain shares one password, so the creds file's probe user norbert
# and glenda authenticate alike).
#
# The cpu image this wrapper dials is R9's OWN: assemble it with
# `cargo xtask image --arch x86-64 --image cpu` (the flake app
# boot-r9-x64 --system=cpu does that and boots it), which writes
# target/images/x86_64/cpu.paq and the 0600
# target/images/x86_64/cpu.creds.  The wrapper reads the creds FILE
# only to prove the bake happened and that a password line exists: the
# password value is never extracted, never printed and never on argv
# (T-01-97) — drawterm prompts for it itself (its own /dev/cons
# prompt).
#
# Secstore: every line this wrapper prints or execs carries `-s ''`
# BEFORE -O — the same load-bearing flag the 19-08 gate spawns with
# (drawterm-gate.expect; the recorded 19-08 cross-plan finding, landed
# here after the owner's first Leg-1 attempt parked on an empty
# console).  Without a -s value drawterm defaults secstoreserver to the
# authserver, and the 3-part auth address above passes through
# netmkaddr AS-IS: the secstore probe dials the AUTH port, where
# authsrv's readn(TICKREQLEN) waits for more bytes while the probe
# waits for its reply — a mutual park neither side's timers heal (no
# password prompt ever prints).  `-s ''` makes secdial fall to its
# $auth parse (host-only -> port 5356), which the committed hostfwd
# pair does NOT forward: the dial is refused instantly and the probe is
# skipped — the standard drawterm behavior on a host with no reachable
# secstore.
#
# Encryption: the DEFAULT is encrypted (devssl #D rc4_256 sha1 via
# pushssl — D-15).  No -e flag appears in any line this wrapper prints;
# `-e clear` is the documented DEBUG fallback only.
#
# The wrapper takes NO arguments (its whole surface is the pin; a
# future flag class is a new decision, not a quiet passthrough).
#
# Env contract (the flake app apps.<linux>.drawterm exports
# DRAWTERM_BIN; a standalone run needs the same values):
#   DRAWTERM_BIN   REQUIRED — Harvey's drawterm package binary; the app
#                  exports it.  Standalone use: set it to the package's
#                  $out/bin/drawterm yourself.
#   R9_ROOT        the r9 checkout root (default $PWD); the app is run
#                  from the repo root.
#   R9_CREDS_FILE  default $R9_ROOT/target/images/x86_64/cpu.creds
#                  (r9's assembler output at 0600).
#
# usage: nix run .#drawterm        (windowed when a display exists;
#                                   headless falls back LOUDLY to -G)

set -eu

R9_ROOT="${R9_ROOT:-$PWD}"
R9_CREDS_FILE="${R9_CREDS_FILE:-$R9_ROOT/target/images/x86_64/cpu.creds}"

err() { echo "error: $*" >&2; exit 1; }

[ -n "${DRAWTERM_BIN:-}" ] || err "DRAWTERM_BIN unset — run via the flake app (nix run .#drawterm), which exports the package binary path"
[ -f "$DRAWTERM_BIN" ] && [ -x "$DRAWTERM_BIN" ] \
	|| err "DRAWTERM_BIN $DRAWTERM_BIN: not an executable file"

[ $# -eq 0 ] || err "unknown argument(s): $* — this wrapper takes no arguments; the connection is pinned (usage: nix run .#drawterm)"

# --- the r9 cpu image contract ----------------------------------------
# The dial target is r9's own cpu image; its creds file is the bake's
# own output.  Presence is required; the password VALUE is never read
# out of the file (T-01-97).
[ -f "$R9_ROOT/Cargo.toml" ] \
	|| err "R9_ROOT=$R9_ROOT is not an r9 checkout (missing Cargo.toml) — run the app from the r9 repo root"
[ -s "$R9_CREDS_FILE" ] \
	|| err "$R9_CREDS_FILE missing/empty — assemble r9's cpu image first: cargo xtask image --arch x86-64 --image cpu (or nix run .#boot-r9-x64 -- --system=cpu, which assembles and boots it)"
grep -q '^user=' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: no user= line (the parse contract)"
grep -q '^password=.' "$R9_CREDS_FILE" || err "$R9_CREDS_FILE: the password= line is empty (the bake wrote no DEMO password)"

# --- the scrub (Pitfall 3): before anything else ----------------------
# drawterm's no-O path reads $cpu/$auth/$USER as connection defaults;
# unset them here AND pin -O below — belt and suspenders, but the pin
# is what actually carries the session (with -O the env reads never
# execute).
unset cpu auth PASS USER

# --- the pin (seam-derived loopback literals) -------------------------
DT_CPU=127.0.0.1
DT_AUTH='tcp!127.0.0.1!5670'
DT_USER=glenda
SPEC="cpu=$DT_CPU,auth=$DT_AUTH,user=$DT_USER"

# --- the printed connect lines ----------------------------------------
# The human default is WINDOWED (no -G in the primary line); the -G
# text form is printed alongside (it is the CI-able/stdio path).  The
# spec rides unquoted (a pasteable line; in an INTERACTIVE bash shell
# the ! in the auth address wants single quotes around the spec).  The
# creds FILE is named; its password value never is (T-01-97).
echo "== r9 cpu image creds: $R9_CREDS_FILE (read by path only; the DEMO password is drawterm's own prompt — never printed, never on argv)"
echo "== drawterm: connect line (windowed default, encrypted rc4_256 sha1):"
echo "   $DRAWTERM_BIN -s '' -O $SPEC"
echo "== text form (-G: no window, stdio session):"
echo "   $DRAWTERM_BIN -G -s '' -O $SPEC"

# --- exec --------------------------------------------------------------
# The `-s ''` rides BEFORE -O on BOTH lines — the gate's exact form
# (the no-secstore pin above; without it the probe parks on the AUTH
# port before the password prompt ever prints).
if [ -z "${DISPLAY:-}" ]; then
	echo "== DISPLAY unset — headless venue: running the -G stdio session (the windowed line needs a display; this is the loud fallback, never a silent downgrade)"
	exec "$DRAWTERM_BIN" -G -s '' -O "$SPEC"
fi
exec "$DRAWTERM_BIN" -s '' -O "$SPEC"
