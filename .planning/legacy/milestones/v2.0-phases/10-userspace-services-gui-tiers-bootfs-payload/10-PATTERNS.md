# Phase 10: Userspace Services + GUI Tiers + Bootfs Payload - Pattern Map

**Mapped:** 2026-11-05
**Files analyzed:** 34 (new + modified)
**Analogs found:** 32 / 34 (2 no-analog: server-side event-script replay format; drawterm serve-only listen loop — partial analogs noted)

**Tracked-source gate (#3645):** every analog path below is git-TRACKED (verified via `git ls-files --error-unmatch`, 2026-11-05). The 9front donor checkout `.build/9front-donor/` is GITIGNORED — it appears below ONLY as a vendoring *source* (copy FROM it INTO tracked tree), never as an edit target, matching the Phase 2 D-10 precedent (7a/7c/7l were vendored from the same pin).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `build/aarch64/tiers/tier2-services.txt` (NEW) | config/data | batch-read | `build/aarch64/tiers/tier1-cmds.txt` | exact |
| `build/aarch64/tiers/tier3-gui.txt` (NEW) | config/data | batch-read | `build/aarch64/tiers/tier1-cmds.txt` | exact |
| `build/aarch64/tiers/tier4-tail.txt` (NEW) | config/data | batch-read | `build/aarch64/tiers/tier1-cmds.txt` | exact |
| `build/aarch64/markers/tier2.ledger` (NEW) | config/data | append-only log | `build/aarch64/markers/tier1.ledger` | exact |
| `build/aarch64/markers/tier3.ledger` (NEW) | config/data | append-only log | `build/aarch64/markers/tier1.ledger` | exact |
| `build/aarch64/markers/tier4.ledger` (NEW) | config/data | append-only log | `build/aarch64/markers/tier0.ledger` (EXCLUDED vocabulary) | exact |
| `sys/src/mkfile` (MOD) | build config | batch | itself, lines 68-75 | exact |
| `sys/src/cmd/mkfile` (MOD) | build config | batch | itself, lines 29-33 | exact |
| `sys/src/cmd/ip/mkfile` (MOD) | build config | batch | itself, lines 32-36 | exact |
| `sys/src/cmd/auth/mkfile` (MOD) | build config | batch | itself, lines 48-52 | exact |
| `sys/src/cmd/auth/factotum/mkfile` (MOD) | build config | batch | itself, lines 11-14 | exact |
| `sys/src/cmd/aux/mkfile` (MOD — NEW leaf filter) | build config | batch | `sys/src/cmd/auth/factotum/mkfile` (T1KEEPT leaf shape) | exact |
| `sys/src/games/mkfile` (MOD — NEW filter, drops mp3enc) | build config | batch | `sys/src/cmd/ip/mkfile` + factotum leaf (hybrid: TARG + DIRS both filtered) | role-match |
| `build/aarch64/subset-build.sh` (MOD — tier2/3/4 loops + per-entry `mk clean`) | host driver | batch | itself, lines 138-253 | exact |
| `build/aarch64/tier2-sweep.rc` (NEW) | guest test script | request-response (probes) | `build/aarch64/tier1-sweep.rc` + `build/aarch64/root-tree/lib/netip-static.rc` + `.../net-sweep.rc` | exact |
| `build/aarch64/subset-exec.sh` (MOD — tier-2 leg: staging, termrc override, host asserts) | host driver | event-driven (qemu run) | itself, lines 660-687 + 796-841 | exact |
| `sys/src/ape/lib/ap/aarch64/` (NEW — vendored: main9.s, memmove.s, memset.s, lock.c, cycles.s, getfcr.s, _seek.c, notetramp.c, mkfile) | vendored lib | batch (compile) | README: `sys/src/cmd/7a/README`; mkfile: in-tree `sys/src/ape/lib/ap/amd64/mkfile` | exact (provenance), exact (mk) |
| `sys/src/ape/lib/ap/aarch64/README` (NEW) | doc | — | `sys/src/cmd/7a/README` + `sys/src/cmd/unix/9cc/README` | exact |
| `sys/src/cmd/gs/default.aarch64.h` (NEW — vendored from donor `default.arm64.h`) | vendored config | batch (compile) | donor file (verbatim content in 10-RESEARCH.md §Code Examples); provenance-comment pattern from 7a/README | exact |
| drawterm-derived serve-only host draw binary (NEW — home is planner discretion) | host tool (C) | event-driven 9P server | `sys/src/cmd/unix/drawterm/` (in-tree: kern/devdraw.c, devmouse.c, devcons.c, cpu.c, exportfs/) + `build/aarch64/mkpaqfs-host.sh` (host-artifact discipline) | role-match (see caveats) |
| committed event-script files per program (NEW — rio/acme/sam replay scripts) | config/data | batch-read by server | none (new mechanism — see No Analog Found) | — |
| `build/harness/src/trace.rs` (MOD — `--gui` mode) | host tool (Rust) | event-driven (qemu run + parse) | itself, `--tier1` mode (lines 1978-2087) | exact |
| GUI trace termrc override (NEW — guest script mounting host draw) | guest script | request-response | `build/aarch64/trace-termrc` + `usr/glenda/lib/profile` cpu-case binds (lines 23-25) | role-match |
| `build/aarch64/mkimage.sh` (MOD — fonts/profile/ndb staging + asserts) | host bake script | file-I/O batch | itself, lines 184-261 (bake-time copies + manifest asserts) | exact |
| `build/aarch64/root-tree/lib/ndb/local.template` (NEW — home is planner discretion) | config template | bake-time transform | `lib/ndb/local` (tuple format) + `build/aarch64/netaddr.rc` (substitution source) | role-match |
| `build/aarch64/root-tree/usr/glenda/lib/profile` (MOD — D-14 trimmed profile replaces 3-line skeleton) | config/data | read at boot | `usr/glenda/lib/profile` (verbatim trim reference) | exact |
| gs render-smoke `.ps` fixture (NEW — tiny committed fixture, home TBD) | test data | batch | `build/aarch64/hello.c` / `tiny.y` / `tiny.l` class (small committed fixed inputs) | role-match |
| `.github/workflows/phase10.yml` (NEW sibling — compile gates only, D-11a) | CI config | batch | `.github/workflows/phase6.yml` | exact |
| `sys/src/libmemdraw/*` (MOD — drawtest/arctest LP64 pixel-path fixes) | lib source | — | the files themselves (bug-fix, no external pattern; Plan 9 C style per CONVENTIONS) | n/a |
| `sys/src/liboventi/mkfile`, `sys/src/libmemdraw/mkfile` (MOD — stale test targets: remove or restore) | build config | — | themselves (decision item; mksyslib shape) | n/a |
| `sys/src/libdynld/dynld.c:128` (MOD — ULONG<0 note fix) | lib source | — | itself (one-line class fix) | n/a |
| `sys/src/cmd/rc/exec.c`/`simple.c`/`glob.c` (MOD — pushlist/poplist residual audit) | cmd source | — | themselves (audit-in-place; ~49 call sites per 10-RESEARCH Pitfall E) | n/a |
| `build/aarch64/dir-coverage.txt` (MOD — regenerated as flips land) | report data | batch | itself (06-04 regeneration discipline; CI `git diff --exit-code`) | exact |

---

## Pattern Assignments

### 1. Tier data files: `tiers/tier2-services.txt`, `tier3-gui.txt`, `tier4-tail.txt` (NEW)

**Analog:** `build/aarch64/tiers/tier1-cmds.txt` (TRACKED)

**Header grammar — the contract every new tier file re-states** (lines 1-34):
```
# tier1-cmds.txt — D-13 append-only aarch64 command allowlist (USPACE-04).
#
# Plain data, one entry per line, sys/src-relative.  Consumers: the
# objtype-gated mk filters (sys/src/mkfile CMDS/SUBSYS, sys/src/cmd/mkfile
# TARG/DIRS, and the container mkfiles cmd/ip, cmd/auth, cmd/auth/factotum),
# all reading through the $HARVEY-absolute path, and the tier drivers
# (build/aarch64 subset/tier scripts, 06-03+).
#
# APPEND-ONLY: growth is appending a line, after the entry's per-tier gate
# passes.  Never reorder, edit, or delete existing lines — later tiers
# (Phase 10) append, and the coverage report + CI diff assert against this
# file (D-13 is rated one-way).
#
# Entry forms:
#   cmd/<single>           single-file command in cmd/ (cmd/cat)
#   cmd/<dir>              whole dir recursed by the sweep (cmd/rc)
#   cmd/<container>/<dir>  nested dir inside a container (cmd/ip/ipconfig)
#   cmd/<dir>:<target>     the dir is kept in the sweep (its mk recursion
#                          builds the dir's TARG); the <target> qualifier is
#                          consumed by the tier drivers (per-target
#                          `mk <target>.install`, never whole-dir) and by
#                          filter-equipped leaf mkfiles (cmd/auth/factotum
#                          drops fgui this way)
```

**Comment-safety rule** (lines 32-34 — the extraction contract the mk awk filters rely on):
```
# Keep-rule (uniform across all five filter mkfiles): a candidate survives
# iff its name is the first path component after that mkfile's prefix in
# some entry below — member (cmd/rc keeps rc), container (cmd/ip/ipconfig
# keeps ip here and ipconfig in cmd/ip), and dir:target (cmd/paqfs:paqfs
# keeps paqfs here and only paqfs in a filter-equipped leaf).  `#` lines and
# trailing comments never match: the extraction keeps only lines starting
# [a-z0-9] and cuts at the first whitespace.
```

**Documented-exclusions block** (lines 40-52 — D-08's "exclusions by name with committed comments" shape; tier4-tail.txt MUST carry this for fossil/venti-runtime, mp3enc, snes/gb, aux/vga/usbd/disk/prep):
```
# Documented exclusions from the aarch64 sweep (USPACE-04 transparency —
# nothing is dropped silently):
#   ape                 subsystem excluded by milestone decision (not cmd/)
#   gs                  ghostscript: tier-excluded (Phase 10 long tail)
#   ...
```

**Entry examples with provenance comments** (lines 71-72, 82, 107):
```
cmd/ip/ipconfig
cmd/auth/factotum:factotum	# fgui excluded: needs the out-of-tier graphics stack (libdraw/libcontrol)
cmd/cc				# shared frontend; tier evidence is the cc.a7 archive the 7c build links
cmd/dd				# write-flow (dash-pair -if/-of syntax — 07-04's measured dialect)
```

**Planner note:** wave banners are `# --- <plan>: <description> ---` comment blocks (lines 54, 83-94). Copy that shape per landing wave.

---

### 2. Tier ledgers: `markers/tier{2,3,4}.ledger` (NEW)

**Analog:** `build/aarch64/markers/tier1.ledger` + `tier0.ledger` (TRACKED)

**Header + line grammar** (tier1.ledger lines 1-10):
```
# tier1.ledger — D-14 append-only marker ledger for tier-1 commands.
#
# Line format (fixed field order, space-separated key=value):
#   tier=1 dir=<sys/src-rel> bin=<name> sha256=<64hex> exec=<marker[,marker]> evidence=<path> commit=<40hex> date=<YYYY-MM-DD>
#
# Append-only: new lines land in their own commits with the gate evidence.
# Supersede semantics: last line per dir wins (a later line redefines an
# earlier one for the same dir for the same dir).  `#` lines are comments; consumers strip
# them.  sha256 is the artifact at gate time (evidence, not a
# reproducibility oracle — artifacts are gitignored build outputs).
```

**Committed-header-first precedent** (06-02): new ledgers land header-only in their introducing commit; entries append with gate evidence.

**Exec vocabulary already in use** (extend, never invent silently — 10-RESEARCH Pitfall G names this phase's additions: `T2-OK`, `TRACED:donor`, `COMPILE-ONLY`, `EXCLUDED:reason`):
- tier1.ledger: `exec=T1-OK:donor`, `exec=PENDING-CHAIN:06-07`, `exec=BOOT:donor` (lines 11-39)
- tier0.ledger: `exec=CARRIED:tier1-consumer`, `exec=EXCLUDED:no-tier1-consumer`, `exec=TEST:donor`, `exec=TEST-FAIL:donor` (lines 11, 47-60)

**Venue honesty (D-04):** every exec marker this phase carries `:donor` — the `venue:donor` convention is per-line suffix on the exec token, per the existing lines above.

---

### 3. mk filter extensions — the five existing sites + two new leaf filters

**Analog A:** `sys/src/mkfile` lines 68-75 (TRACKED) — the shape each filter grows into (multi-file awk, ONE call, no generated union file — 10-RESEARCH Pattern 1):
```mk
TIER0=$HARVEY/build/aarch64/tiers/tier0-libs.txt
TIER1=$HARVEY/build/aarch64/tiers/tier1-cmds.txt
T0KEEP=`{awk '/^[a-z0-9]/{ sub(/[\/: 	].*$/,""); print }' $TIER0}
T1KEEP=`{awk '/^[a-z0-9]/{ sub(/[\/: 	].*$/,""); print }' $TIER1}

LIBS=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLLIBS) if(~ $i $T0KEEP) echo $i; if not echo $ALLLIBS}
CMDS=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLCMDS) if(~ $i $T1KEEP) echo $i; if not echo $ALLCMDS}
SUBSYS=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLSUBSYS) if(~ $i $T1KEEP) echo $i; if not echo $ALLSUBSYS}
```
The Phase-10 delta: add `TIER2/TIER3/TIER4` variables and pass all four files as multi-file awk args (`$TIER1 $TIER2 $TIER3 $TIER4`). **Backquote constraints** (the plan9port mk lexer, measured 06-02 and restated in sys/src/mkfile lines 65-67): single-line, brace-free rc, single-quoted awk program, no literal newlines inside backquotes.

**Analog B:** `sys/src/cmd/mkfile` lines 29-33 (prefix-stripped keep extraction — each container mkfile strips its own prefix before the whitespace cut):
```mk
TIER1=$HARVEY/build/aarch64/tiers/tier1-cmds.txt
T1KEEP=`{awk '/^cmd\//{ sub(/^cmd\//,""); sub(/[\/: 	].*$/,""); print }' $TIER1}

TARG=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLTARG) if(~ $i $T1KEEP) echo $i; if not echo $ALLTARG}
DIRS=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLDIRS) if(~ $i $T1KEEP) echo $i; if not echo $ALLDIRS}
```

**Analog C:** `sys/src/cmd/ip/mkfile` lines 32-36 (container prefix `cmd\/ip\/`) and `sys/src/cmd/auth/mkfile` lines 48-52 (container prefix `cmd\/auth\/`) — identical shape, prefix differs.

**Analog D (the leaf/dir:target filter):** `sys/src/cmd/auth/factotum/mkfile` lines 11-14 — note the `:` anchor in the awk match so ONLY dir:target entries feed the leaf keep-list:
```mk
TIER1=$HARVEY/build/aarch64/tiers/tier1-cmds.txt
T1KEEPT=`{awk '/^cmd\/auth\/factotum:/{ sub(/^cmd\/auth\/factotum:/,""); sub(/[\/: 	].*$/,""); print }' $TIER1}

TARG=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLTARG) if(~ $i $T1KEEPT) echo $i; if not echo $ALLTARG}
```

**NEW filter 1 — `sys/src/cmd/aux/mkfile`** currently has NO filter (verified lines 3-58: flat TARG incl. apm/pcmcia/listen + DIRS incl. vga/realemu). The factotum shape retargeted (10-RESEARCH Pattern 1, verbatim):
```mk
TIER1=$HARVEY/build/aarch64/tiers/tier1-cmds.txt
TIER2=$HARVEY/build/aarch64/tiers/tier2-services.txt
T1KEEPT=`{awk '/^cmd\/aux:/{ sub(/^cmd\/aux:/,""); sub(/[\/: 	].*$/,""); print }' $TIER1 $TIER2}
TARG=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLTARG) if(~ $i $T1KEEPT) echo $i; if not echo $ALLTARG}
```
**Critical ordering (Pitfall F):** the aux leaf filter lands in the SAME commit as the first `cmd/aux:listen` tier entry — otherwise the keep-rule keeps `aux` in cmd's DIRS and the default aarch64 build sweeps aux's whole TARG (apm/pcmcia class) red. Note aux's mkfile also lacks an `ALLTARG` split (bare `TARG=` at line 3) — the filter needs the `ALLTARG`-then-`TARG` two-step like factotum's lines 3-14. aux also has unfiltered `DIRS=mnihongo flashfs gps na vga realemu` (lines 53-58) — decide whether the leaf filters DIRS too or the tier entry never triggers aux's `all:V: $DIRS` recursion (the tier drivers build per-target via `mk <target>.install`, which does not recurse DIRS; the default-build exposure is the cmd→aux DIRS keep, solved by cmd/mkfile's prefix rule).

**NEW filter 2 — `sys/src/games/mkfile`** has flat `TARG` (lines 3-10) + `DIRS` (lines 18-25, incl. mp3enc — pcc-built, fails on aarch64 per dir-coverage.txt). Hybrid: filter TARG by member entries AND DIRS by nested entries, dropping mp3enc by name (a `# games/mp3enc excluded: pcc/APE` comment in tier4-tail.txt is the D-08 record).

**Sites needing NO filter (whole-dir joins):** `sys/src/cmd/ndb/mkfile` (verified lines 4-16: all 12 TARG members are tier-2 — `cmd/ndb` whole-dir entry suffices), `sys/src/cmd/upas` (whole-dir per RESEARCH Pattern 1).

---

### 4. Tier driver: `build/aarch64/subset-build.sh` (MOD — tier2/3/4 loops + anti-staleness)

**Analog:** itself, lines 138-253 (TRACKED).

**tier-0 loop with D-07 fresh-archive + artifact marker** (lines 146-164):
```sh
TIER0="$HARVEY/build/aarch64/tiers/tier0-libs.txt"
[ -f "$TIER0" ] || fail "tier0 data file missing: $TIER0"
while read -r line; do
	case "$line" in ''|'#'*) continue;; esac
	d="${line%%[ 	]*}"
	case "$d" in ''|'#'*) continue;; esac
	...
	echo "== tier0 lib: $d"
	rm -f "/aarch64/lib/$lib"
	(cd "/sys/src/$d" && objtype=aarch64 mk install) || fail "lib $d"
	[ -s "/aarch64/lib/$lib" ] || fail "lib $d: no artifact /aarch64/lib/$lib"
	echo "T0-ART $d $(sha256sum "/aarch64/lib/$lib" | cut -d' ' -f1)"
done < "$TIER0"
```

**tier-1 entry-form dispatch** (lines 180-253 — the tier-2/3/4 loops reuse this case dispatch verbatim; only the tier file path and marker prefix change):
```sh
while read -r line; do
	case "$line" in ''|'#'*) continue;; esac
	entry="${line%%[ 	]*}"
	case "$entry" in ''|'#'*) continue;; esac
	echo "== tier1 entry: $entry"
	case "$entry" in
	cmd/*:*)
		# dir:target — per-target install in the dir (the mkmany
		# %.install rule), never the whole dir TARG.
		dir="${entry%%:*}"
		tgt="${entry##*:}"
		(cd "/sys/src/$dir" && objtype=aarch64 mk "$tgt.install") || fail "cmd $entry"
		art="/aarch64/bin/${dir#cmd/}"
		;;
	cmd/*)
		if [ -d "/sys/src/$entry" ]; then
			(cd "/sys/src/$entry" && objtype=aarch64 mk install) || fail "cmd $entry"
		else
			(cd /sys/src/cmd && objtype=aarch64 mk "${entry#cmd/}.install") || fail "cmd $entry"
		fi
		art="/aarch64/bin/${entry#cmd/}"
		;;
	*)	fail "tier1 entry form not understood: $entry" ;;
	esac
	[ -s "$art" ] || fail "$entry: no artifact $art"
	echo "T1-ART ${art##*/} $(sha256sum "$art" | cut -d' ' -f1)"
done < "$TIER1"
```

**Anti-staleness decision (the 06-03 open item this phase owns — RECOMMENDED in 10-RESEARCH Pattern 2):** per-entry `(cd "/sys/src/$dir" && objtype=aarch64 mk clean)` BEFORE `mk install` inside the tier driver loops only (never the default build). Precedent one level down: `donor-vm-build.sh` step-4b stale-object sweep. Rationale recorded: the working tree carries gitignored stale `.6`/`.7` objects in cmd/auth/*, games/*, lp/LOCK.7 today; mk's freshness graph never names the toolchain. Marker prefixes grow as `T2-ART`/`T3-ART`/`T4-ART`.

**⚠ Correction (checker iteration 1 — 10-01/10-04 revision):** the tier-1 dispatch's `art="/aarch64/bin/${dir#cmd/}"` formula above is correct only by coincidence — its two colon entries satisfy basename(dir)==target AND their mkfile BINs mirror the entry path (paqfs BIN=/$objtype/bin; factotum BIN=/$objtype/bin/auth). The tier-2/3/4 loops do NOT clone the formula: the driver resolves artifacts through a `bin_dir()` helper (the entry mkfile's last `BIN=` line, $objtype-substituted — aux and samterm install into bin/aux, the auth container into bin/auth, ip into bin/ip; cmd/auth/lib is an mksyslib archive with no bin artifact at all) and a `members_of()` member resolution for whole-dir entries. See 10-01 Task 2 step 4 (contract + colon arm) and 10-04 Task 1 step 2 (completion arms).

---

### 5. Tier-2 functional-probe sweep: `build/aarch64/tier2-sweep.rc` (NEW)

**Analog A:** `build/aarch64/tier1-sweep.rc` (TRACKED) — the master shape.

*Venue argument + path-prefix block* (lines 76-99; tier-2 is donor-only per D-04 — NO r9/9k venue guard needed, unlike tier1's three-venue shape):
```rc
venue=$1
if(~ $#venue 0) venue=donor

B=/boot
D=/boot
rcm=()
if(~ $venue r9){
	B=/bin
	D=/lib/tier1
}
if(~ $venue 9k){
	rcm=(-m /boot/rcmain)
}
```

*The `check` helper + never-wedge discipline* (lines 101-109):
```rc
# check NAME CMD... — run the fixed input; zero exit status → T1-OK.
# A failing entry never aborts the sweep: the command rides if(), its
# status becomes the marker, and the sweep continues.
fn check {
	n=$1
	shift
	if($*) echo T1-OK $n
	if not echo T1-FAIL $n
}
```

*Marker vocabulary header* (lines 30-52 — copy the comment-table shape; tier-2 emits `T2-OK <svc>` / `T2-FAIL <svc>` / `T2-DONE`).

**rc dialect lessons baked into every sweep block** (tier1-sweep.rc lines 374-385, measured on the donor 06-06 — the tier-2 probes MUST honor these):
- plan9port/guest rc lexer rejects subshells with separators inside backquotes → `fn` producers for multi-command input streams
- bare `=` ends a word → flag=value arguments must be quoted (`'-DVALUE=41'`)
- quoting DISABLES wildcarding in `~` patterns; multi-star spans over literal text unreliable → two-condition split asserts (lines 235-238)
- a failed redirect OPEN is an uncatchable rc Xerror → `test -e`/`test -d` before any `>` redirect (lines 479-485)

*Daemon settle pattern (TCG-tolerant, bounded)* (lines 604-611 — the factotum resident-check shape the authsrv/listen/plumber probes reuse):
```rc
$B/hfactotum -n -s subexec &
factok=0
for(i in 1 2 3 4 5 6 7 8 9 10){
	if(~ hfactotum `{$B/hps}) factok=1
	if(~ $factok 0) sleep 1
}
if(~ $factok 1) echo T1-OK factotum
if not echo T1-FAIL factotum
```

**Analog B:** `build/aarch64/root-tree/lib/netip-static.rc` (TRACKED) — the netaddr.rc seam sourcing + ipconfig invocation + venue label (lines 35-56, 100-106, 126-130):
```rc
# the D-11 seam (test -e first: a failed source is uncatchable)
test -e /lib/netaddr.rc || {
	echo netip: FAIL seam >[1=2]
	exit seam
}
. /lib/netaddr.rc

bind -a '#I' /net || {
	echo netip: FAIL bind >[1=2]
	exit bind
}
ip/ipconfig -g $slirp_gwv4 ether /net/ether0 add $slirp_guestv4 $slirp_maskv4 || { ... }
...
ip/ping -n 3 $slirp_gwv4 || { ... }
...
echo netip: NOT-L2-TRUTH slirp venue
echo netip: NETIP-DONE
```
**Pitfall C override for tier-2's ipconfig probe:** use `ipconfig -x /net.alt ...` against a FRESH net tree (bind '#I'/'#l0' into the alternate mtpt first) — never re-DHCP the root-serving leg (10-RESEARCH Pitfall C; ipconfig's `-x mtpt` verified at `sys/src/cmd/ip/ipconfig/main.c:543`).

**Analog C:** `build/aarch64/root-tree/lib/net-sweep.rc` (TRACKED) — the per-stage `|| { echo FAIL; exit }` idiom and `test -e` before ctl-write (lines 44-72). NOTE: tier2-sweep.rc follows tier1-sweep.rc's continue-on-fail (never-wedge) model, NOT net-sweep.rc's exit-on-fail — but borrows the FAIL-stage naming.

**Probe semantics per service (locked by D-03/D-05/D-12; verbatim shapes in 10-RESEARCH.md §Code Examples "Tier-2 probe shapes"):** canned clients are `ndb/csquery`, `ndb/dnsquery`, `dial/pass`, `dial/expect`, `srv -e` — all in-tree, all ride tier-2 as members (Don't-Hand-Roll table). keyfs over an ABSENT test file (`keyfs.c:866-868` — open failure returns 0, serves empty DB, writes back on create); authsrv via the listen + service-script topology (donor `rc/bin/service.auth/tcp567` shape: `exec /bin/auth/authsrv -N $3`); plumber needs `$user`/`$home`, `-p rules`, posts `/srv/plumb.$user.$pid` (ORCLOSE — rm for clean shutdown).

---

### 6. `build/aarch64/subset-exec.sh` extension (MOD — the tier-2 leg)

**Analog:** itself (TRACKED).

**Staged termrc override block** (lines 660-687 — the tier-2 leg copies this shape with `tier2-sweep.rc` substituted):
```sh
if [ -f "$ROOTFS/rc/bin/termrc" ] && "$GREP" -qF 'tier1-sweep.rc' "$ROOTFS/rc/bin/termrc"; then
	echo "== served termrc still carries a stranded 06-03 override — restoring the donor's first"
	cp "$DONOR/rc/bin/termrc" "$ROOTFS/rc/bin/termrc"
fi
cp "$ROOTFS/rc/bin/termrc" "$TERMRC_ORIG"
TERMCMD="${TIER1_TERMCMD:-. /boot/tier1-sweep.rc donor}"
cat > "$TMPD/tier1-termrc" <<TERMRC_EOF
#!/bin/rc
...
/boot/hstacksmoke
s=\$status
if(~ \$s '') echo T1-OK stacksmoke
if(! ~ \$s '') echo T1-FAIL stacksmoke
exec rc -c '$TERMCMD'
TERMRC_EOF
cp "$TMPD/tier1-termrc" "$ROOTFS/rc/bin/termrc"
```

**srv9 writable-prefix + boot stanza** (lines 693-715): `--writable-prefix /tmp --writable-prefix /usr/glenda/tmp`; the guestfwd at line 734:
```sh
-netdev "user,id=n0,guestfwd=tcp:$SRV9_GUEST_IP:564-tcp:127.0.0.1:$SRV9_PORT" \
```
The GUI trace leg grows a SECOND comma-separated guestfwd on this stanza (10-RESEARCH Pattern 5): `guestfwd=tcp:10.0.2.100:9999-tcp:127.0.0.1:<draw9 port>`.

**Host-side assertion battery** (lines 796-841 — the computed-expected-set pattern; tier-2's expected set is computed from tier2-services.txt the same way):
```sh
EXPECTED=$(
	awk 'NF && $1 !~ /^#/ { print $1 }' "$HARVEY/build/aarch64/tiers/tier1-cmds.txt" |
	while read -r e; do
		case "$e" in
		9k/boot|stage/init|cmd/paqfs:paqfs|cmd/7c|cmd/7l|cmd/cc) continue ;;
		stage/stacksmoke) echo stacksmoke; continue ;;
		esac
		e="${e#cmd/}"
		case "$e" in *:*) e="${e##*:}" ;; esac
		echo "${e##*/}"
	done
)
# per-marker asserts are boundary-anchored ERE (WR-06): an unanchored
# substring grep false-greens on superset names (mk matches mkdir, cp
# matches cpp)
for m in $EXPECTED float-d01 ...; do
	"$GREP" -qE 'T1-OK '"$m"'([^A-Za-z0-9.-]|$)' "$RUNLOG" || err "captured log lacks T1-OK $m"
done
"$GREP" -q 'T1-DONE' "$RUNLOG" || err "..."
if "$GREP" -q 'T1-FAIL' "$RUNLOG"; then
	err "captured log contains a T1-FAIL line — a swept binary failed"
fi
if "$GREP" -qiE 'panic|trap:' "$RUNLOG"; then
	err "captured log contains a kernel fault line"
fi
```

---

### 7. Vendored ape aarch64 slice: `sys/src/ape/lib/ap/aarch64/` + README (NEW)

**Provenance README analog:** `sys/src/cmd/7a/README` lines 1-13 + 34-36 (TRACKED — the D-10 pattern: pin SHA, what was imported, local-mods list, license):
```
# Vendored 9front arm64 assembler `7a`
# (https://github.com/9front/9front @ 350266104f1fed81c78c3d05fa44f523bec0ae4d,
# branch "front", pinned 2026-10-31).
...
# What was imported: sys/src/cmd/7a/ wholesale (a.h, a.y, lex.c).
#
# Local modifications vs upstream (D-13: drift is shimmed in vendored
# files only — Harvey's shared cc frontend is untouched):
#   * a.h — BUFSIZ defined as (16*1024): ...
...
# Licenses: MIT-compatible; credit Bell Labs/Lucent, Vita Nuova,
# C H Forsyth, and 9front.
```
The richer "Local modifications" bullet style for adapted files: `sys/src/cmd/unix/9cc/README` lines 14-49.

**mkfile shape (in-tree, TRACKED):** `sys/src/ape/lib/ap/amd64/mkfile` (verbatim head):
```mk
APE=/sys/src/ape
<$APE/config
LIB=/$objtype/lib/ape/libap.a
OFILES=\
	atom.$O\
	_seek.$O\
	...
	</sys/src/cmd/mksyslib

CFLAGS=-c -D_POSIX_SOURCE -D_PLAN9_SOURCE
```
The donor's `arm64/mkfile` (GITIGNORED source: `.build/9front-donor/sys/src/ape/lib/ap/arm64/mkfile`) is the same shape minus `atom/strchr/strlen` and plus `memmove/memset` — the vendored aarch64 mkfile starts from the donor's OFILES list, keeps Harvey's quoting idiom. Verified donor file list (D-01a): `main9.s, main9p.s, memmove.s, memset.s, lock.c, cycles.s, getfcr.s, _seek.c, notetramp.c, setjmp.s, tas.s, mkfile`.

**mk wiring note (agent's Discretion):** the pcc toolchain resolves `/$objtype/lib/ape/libap.a` and `-I/$objtype/include/ape` (sys/src/cmd/pcc.c:165-193 per 10-RESEARCH Pitfall A) — the vendored slice must build under the namespace build with `objtype=aarch64`; `sys/src/ape/config` sets `CC=pcc LD=pcc` (verified). Check whether the tier filters' SUBSYS keep-rule needs an `ape` entry form or whether the slice builds via an explicit driver block (ape is deliberately NOT in the default aarch64 sweep — the milestone's ape-environment exclusion stands; only the libap.a artifact is needed).

**`sys/src/cmd/gs/default.aarch64.h` (NEW):** vendor the donor's `sys/src/cmd/gs/default.arm64.h` VERBATIM (genarch output on real 9front arm64; full 30-line content reproduced verbatim in 10-RESEARCH.md §Code Examples) + prepend a provenance comment naming the donor pin (the 7a/README header shape, comment-class adapted to a C header: `/* Vendored from 9front @ 350266104f... genarch-produced on real arm64; vendored as default.aarch64.h per D-01a */`). gs's `$objtype.h` rule (`sys/src/cmd/gs/mkfile:148-161`) copies `default.$objtype.h` whenever `$objtype != $cputype` (always true in our namespace builds) and fails loudly if absent — the vendored file is exactly what that rule consumes.

---

### 8. Drawterm-derived serve-only host draw binary (NEW)

**Primary analog (TRACKED, in-tree):** `sys/src/cmd/unix/drawterm/` — a complete tracked drawterm checkout (older lineage).

*Device-bind init shape* — `main.c` lines 45-68 (the serve-only binary keeps this spine, drops `cpumain`):
```c
chandevreset();
chandevinit();
quotefmtinstall();

if(bind("#c", "/dev", MBEFORE) < 0)
	panic("bind #c: %r");
if(bind("#m", "/dev", MBEFORE) < 0)
	panic("bind #m: %r");
if(bind("#i", "/dev", MBEFORE) < 0)
	panic("bind #i: %r");
...
cpumain(argc, argv);   // ← the cpu-client half: DROPPED in the serve-only binary
```

*The served draw tree shape* — `kern/devdraw.c` lines 257-299 (drawgen: `new` at level 2; per-client `colormap`/`ctl`/`data`/`refresh` at level 3) + `kern/devmouse.c` lines 26-27 (`cursor`, `mouse` static dirtab). This is the exact tree shape Harvey's 9k devdraw/devmouse define and the guest binds over /dev (10-RESEARCH Pattern 5 cross-verified all three).

*The 9P export spine* — `cpu.c` lines 182-214: after `rexcall` handshake, drawterm runs `exportfs(data, msgsize)` to serve its local namespace over the connection; `exportfs/` contains `exportfs.c` + `exportsrv.c` — the in-tree 9P server-loop over a device tree. The serve-only binary's core change: replace the dial-out+rexcall with a 127.0.0.1 9P2000/tcp LISTENER (loopback only — 10-RESEARCH Security table: never 0.0.0.0), then run the same export path.

*Host build shape* — `Makefile` + `Make.config` + per-platform `Make.osx` (OFILES/LIBS1/libmachdep pattern). **CAVEAT (verified this session):** in-tree `Make.osx` is `-m32 -framework Carbon -framework QuickTime` — ancient 32-bit Carbon, will NOT build on modern darwin aarch64. The buildable darwin backend (`gui-cocoa`) and `kern/devkbd.c` live in the 9front-lineage drawterm (`nixpkgs drawterm-0-unstable-2026-10-12`, verified in 10-RESEARCH §Sources/§Environment; NOT currently resident in the local nix store). **Planner fetch note:** re-fetch via nixpkgs (`nix build nixpkgs#drawterm.src` or the flake devshell's nixpkgs pin) at plan-execution time; vendor or reference per the D-10 provenance pattern. Do NOT treat `.build/` as its home — gitignored.

*Host-artifact discipline analog:* `build/aarch64/mkpaqfs-host.sh` (TRACKED, full 46 lines) — the `.build/bin/<tool>` gitignored-host-binary + `$PLAN9/bin LAST` PATH discipline + `set -eu` + trap-cleaned scratch:
```sh
# $PLAN9/bin LAST: strip any existing occurrence, then append.
P9LAST=$(echo "$PATH" | sed "s|:$PLAN9/bin||g; s|^$PLAN9/bin:||; s|^$PLAN9/bin$||")
PATH="$P9LAST:$PLAN9/bin"
```
(The drawterm server is a host C program — it does NOT belong in `build/harness/` (std-only Rust) and NOT in `sys/src/cmd/aux/` (target binaries). Home is planner discretion; a `build/` sibling dir or `sys/src/cmd/unix/drawterm-serve/` are the shapes consistent with the tree.)

---

### 9. GUI trace mode: `build/harness/src/trace.rs --gui` (MOD)

**Analog:** the `--tier1` mode in the same file (TRACKED, lines 1978-2087).

*Mode plumbing:* `main.rs` line 31 (`"trace-syscalls" => trace::run(&args[2..])`); `trace.rs` USAGE const (lines 446-456) gains the `--gui <prog>` line; `Cfg` struct (lines 460-485) + `cfg()` (487-562) show the per-mode artifact/runlog/kernel-path triple and the env-override discipline (`HARVEY`, `HARVEY_VM`, `BOOT_BUDGET`, `SRV9_PORT`, `TRACE_OUT`).

*Per-mode staging table* (lines 1995-2068 — the GUI mode gets its own const table: GUI binaries + fonts + event script + tracesup):
```rust
const STAGE_T1: &[(&str, &str)] = &[
    ("aarch64/bin/cat", "hcat"),
    ...
    ("$TMPD/tier1-tracemain.rc", "tier1-tracemain.rc"),
    ("$TMPD/tracesup.7.out", "tracesup"),
];
```

*The trace-main rc const* (lines 2079-2087 — host-side string constant staged as data; the GUI trace termrc follows this pattern):
```rust
const T1_TRACEMAIN_RC: &str = r#"#!/bin/rc
# 06-07 tier-1 trace main — runs INSIDE the traced shell: ...
/boot/hstacksmoke
s=$status
...
```

*In-guest supervisor (unchanged):* `build/aarch64/tracesup.c` (TRACKED) — generic over any process tree; the GUI mode rides it as-is. Arming discipline + loopchild shape (lines 17-110): root pid already Stopped → write `startsyscall` FIRST; born-traced child discovered Stopped → read pending fragment BEFORE first resume; teardown on Eprocdied → `_rootdone` sentinel → kill+start leftovers → `_done`.

*Guest trace termrc analog:* `build/aarch64/trace-termrc` (TRACKED, full file):
```rc
#!/bin/rc
stacksmoke
exec rc -c '. /boot/oursweep.rc'
```
The GUI trace termrc adds BEFORE the traced launch (10-RESEARCH Patterns 4-5): mount `tcp!10.0.2.100!<draw9 port>` (numeric — no cs), per-file-bind cons/consctl/mouse/cursor + dir-bind draw over /dev — the choreography mirrors `usr/glenda/lib/profile` lines 23-25:
```rc
bind /mnt/term/dev/cons /dev/cons
bind /mnt/term/dev/consctl /dev/consctl
bind -a /mnt/term/dev /dev
```

*Event-loop proof markers (CONTEXT watch-out):* the scripted session types `echo T3-RIOLIVE` into the rio New window's rc (its stdout is the serial cons); acme executes `echo T3-ACMELIVE`; the host gate asserts these markers AND the required syscall set (open/read of /dev/draw/new, write to draw data). sam's marker: `ps` evidence of the samterm child — note sam execs `/bin/aux/samterm` hardcoded (`sys/src/cmd/sam/plan9.c:19`), so subset-exec-style srvroot staging of `arm64/bin/aux/samterm` is required (10-RESEARCH Pattern 4).

---

### 10. DESK-03 payload: `mkimage.sh` (MOD) + ndb template (NEW) + profile (MOD)

**Analog:** `build/aarch64/mkimage.sh` itself (TRACKED).

*Bake-time copy precedent* (lines 184-190 — fonts follow the adm/timezone shape):
```sh
# 4. bake-time verbatim copies (rcmain: never edited, single source of
#    truth; timezone: the repo's data, consumed by init's cpenv)
cp "$HARVEY/rc/lib/rcmain" "$STAGE/rc/lib/rcmain"
cp "$HARVEY/adm/timezone/local" "$STAGE/adm/timezone/local"
# 08-04: the D-11 addressing seam is baked verbatim at /lib/netaddr.rc
cp "$HARVEY/build/aarch64/netaddr.rc" "$STAGE/lib/netaddr.rc"
```
Fonts (D-13): `cp -R lib/font/bit "$STAGE/lib/font/bit"` — 36MB raw (measured: shinonome 13MB, symbola 6MB, lucida 5MB, gb 4MB). mkpaqfs deflates blocks (`sys/src/cmd/paqfs/mkpaqfs.c:373-375`); bootfs.paq currently 4.0MB → ~8× growth accepted; **the size + `-m 1G` r9 boot headroom check is an EARLY task in the DESK-03 plan** (CONTEXT watch-out).

*Manifest assertion shapes* (lines 194-257 — presence loops, content greps, count/size floors all follow these):
```sh
for p in boot/boot boot/paqfs aarch64/init ...; do
	[ -f "$STAGE/$p" ] || err "manifest: $p missing from staged tree"
done
"$GREP" -q "init: starting /bin/rc" "$OUT/init" \
	|| err "out/init is not the real init (staged-supply-chain pollution — re-run sh build/aarch64/subset-build.sh)"
cmp -s "$HARVEY/rc/lib/rcmain" "$STAGE/rc/lib/rcmain" || err "staged rcmain diverges from rc/lib/rcmain"
```
DESK-03 adds: the three app-named fonts (`lucm/unicode.9.font`, `lucidasans/euro.8.font`, `pelm/latin1.8.font`) + a font-count floor + profile content greps (`case cpu` present, `upas/fs` absent) + ndb/local substituted-values asserts.

*Image magic asserts* (lines 271-279): header `529ab12b` / trailer `6b46e688` via `od` (never dd — the D-02 hostbin-shadowing class).

*ndb/local template + substitution* (10-RESEARCH Pattern 6 shape — the seam stays single-source):
```sh
. "$HARVEY/build/aarch64/netaddr.rc"	# bare key=value, sh-compatible (verified)
sed -e "s/@GUESTV4@/$slirp_guestv4/g" -e "s/@GWV4@/$slirp_gwv4/g" \
	"$template" > "$STAGE/lib/ndb/local"
grep -q "$slirp_guestv4" "$STAGE/lib/ndb/local" || err "ndb/local substitution missing"
grep -q '@' "$STAGE/lib/ndb/local" && err "ndb/local carries an unsubstituted token"
```
**Format analog:** `lib/ndb/local` (TRACKED, in-tree) — ndb(6) attr=value tuple lines; the `database= file=/lib/ndb/local file=/lib/ndb/common` root block shape (its lines 1-10). netaddr.rc keys available for substitution (TRACKED): `slirp_guestv4`, `slirp_maskv4`, `slirp_gwv4`, `slirp_guestv6`, `slirp_gwv6`, `blastport`, `lo_guestv4` (netaddr.rc lines 18-38). **Pitfall 9:** no hardcoded 10.0.2.x anywhere outside netaddr.rc.

*Profile (D-14 trim reference, TRACKED):* `usr/glenda/lib/profile` (full 36 lines read). The trim: drop `upas/fs` (line 7) and `news` (lines 28, 35); the `case terminal` path keeps binds/tmp/font but LOSES `exec rio -i riostart` (line 16 — serial console at boot; rio must provably NOT start at boot, Pitfall 14); `case cpu` intact (lines 17-32, incl. `font=/lib/font/bit/pelm/latin1.8.font` + `exec rio` for the drawterm-called path). Current staged skeleton to replace: `build/aarch64/root-tree/usr/glenda/lib/profile` (3 lines, TRACKED).

---

### 11. CI sibling workflow: `.github/workflows/phase10.yml` (NEW)

**Analog:** `.github/workflows/phase6.yml` (TRACKED, lines 1-76).

Copy wholesale: the header comment contract (job inventory + venue split + supply-chain table), the 40-char-SHA action pins (lines 64-72 — `actions/checkout`=11d5960a326750d5838078e36cf38b85af677262, `nix-installer-action`=3138316df39ed29be04236d7ffc686fa525866aa, `upload-artifact`=ea165f8d65b6e75b540449e92b4886f43607fa02), `on: [push, pull_request]`, the concurrency-group cancel-superseded discipline (line 75-76), `permissions: contents: read` default.

**D-11a posture:** hosted CI carries ONLY tier compile gates (wave0-host-shape legs: coverage regen + diff, tier-file/diff asserts). ALL donor-execution gates (tier-2/3 functional probes) + the GUI trace are documented-manual on aarch64 dev hosts (darwin via lima) — the workflow MUST NOT provision the 9front donor (G-06-1's retirement spirit; phase6.yml lines 49-58's venue-pin negative-grep pattern is the enforcement shape to mirror).

---

### 12. Phase-6-filed checklist fixes (MOD — in-place bug fixes)

No external analogs — fixes land in the files themselves, Plan 9 C style per CONVENTIONS (tabs, `nil`, `waserror`, no `//` comments, one commit per bug class per the atomic class-sweep rule):

- `sys/src/libmemdraw/` — drawtest/arctest LP64 pixel-path audit (06-05 fault evidence in 06-EVIDENCE.md; both tests rejoin tier1-sweep.rc's libtest block when green — the sweep's lines 638-690 comment block names the XFAIL-whitelist rejoin shape)
- `sys/src/liboventi/mkfile` / `sys/src/libmemdraw/mkfile` — stale `$O.vttest`/`$O.test` targets: remove or restore (this phase's decision)
- `sys/src/libdynld/dynld.c:128` — `ULONG < 0` comparison
- `sys/src/cmd/rc/exec.c` (+`simple.c`, `glob.c`) — pushlist/poplist balance audit (~49 call sites; priority: Xbackq's siblings Xsubshell/Xpipefd/Xassign/Xqdol/Xfor/Xglob error paths; pushlist/poplist defined at exec.c:77/86 — per 10-RESEARCH Pitfall E)

---

## Shared Patterns

### Append-only data-file discipline + comment-safe extraction
**Source:** `build/aarch64/tiers/tier1-cmds.txt` header + `sys/src/mkfile:65-67`
**Apply to:** all three new tier files, all three ledgers, dir-coverage.txt regenerations
- Growth = appending a line after the per-tier gate passes; never reorder/edit/delete (D-13 one-way)
- Extraction keeps only lines starting `[a-z0-9]`, cuts at first whitespace → `#` comments and trailing provenance notes never match
- Exclusions by name with committed `#` comments, never silent drops (D-08)

### Marker vocabulary + venue honesty
**Source:** `build/aarch64/markers/tier1.ledger` + `tier1-sweep.rc:30-52` + `subset-exec.sh:834-839`
**Apply to:** tier2-sweep.rc, all GUI trace markers, all three ledgers
- Sweep markers: `T2-OK <svc>` / `T2-FAIL <svc>` / `T2-DONE`; GUI: `T3-RIOLIVE`/`T3-ACMELIVE` class + `TRACED:donor` ledger vocabulary
- Ledger exec tokens carry `:donor` venue suffix (D-04); `COMPILE-ONLY`/`EXCLUDED:reason` for tier-4 default and named drops
- Named SKIP over unearned OK (the T1-SKIP precedent); fossil/venti lines carry the documented-runtime-unverifiable comment
- Host gate reds on ANY `T2-FAIL` and any `panic|trap:` line; presence-asserted, never interleaving order

### Sourced-not-typed + never-wedge sweep discipline
**Source:** `tier1-sweep.rc:101-109` + `subset-exec.sh:660-687` + rc dialect lessons (`tier1-sweep.rc:374-385`)
**Apply to:** tier2-sweep.rc, the GUI trace termrc, the gs render smoke
- Sourced via staged termrc override (`exec rc -c '. /boot/<sweep> donor'`), restored by the cleanup trap; NEVER typed (donor devcons fails /dev/cons reads Egreg by design)
- A failing probe emits its FAIL marker and the sweep CONTINUES; per-probe internal timeouts; bounded settle loops (the 10× sleep-1 factotum shape)
- rc dialect: fn producers not subshells in backquotes; quote flag=value args; two-condition split asserts; `test -e` before any redirect

### netaddr.rc single-source (Pitfall 9)
**Source:** `build/aarch64/netaddr.rc` + `netip-static.rc:35-56`
**Apply to:** tier2-sweep.rc (all probe addresses), the ndb/local template substitution, the GUI trace guestfwd values
- Every address comes from sourcing netaddr.rc values; a hardcoded 10.0.2.x anywhere is a review red
- ipconfig probe uses `-x /net.alt` against a fresh net tree — never touch the root-serving leg (Pitfall C)

### Vendored-source provenance (D-10)
**Source:** `sys/src/cmd/7a/README` + `sys/src/cmd/unix/9cc/README`
**Apply to:** `sys/src/ape/lib/ap/aarch64/README`, `sys/src/cmd/gs/default.aarch64.h` header comment, any drawterm vendoring
- Header: upstream URL + full 40-char pin SHA + pin date + what was imported + local-mods bullet list + license note (9front is MIT — compatible)
- Drift is shimmed in vendored files only; shared frontends untouched

### Host-tool discipline
**Source:** `build/aarch64/mkpaqfs-host.sh` + `mkimage.sh:44-47,271-279`
**Apply to:** the drawterm-derived server build, any new host scripts
- `set -eu`; trap-cleaned `mktemp -d` scratch; host artifacts into gitignored `.build/bin/`; `$PLAN9/bin` LAST in PATH
- Host grep resolved once (`command -v /usr/bin/grep || command -v grep`); `od` not `dd` for binary reads (Harvey's dd shadows POSIX dd on the devshell PATH)
- `build/aarch64/out/` last-writer-wins — tier builds and bakes share it; no third writer

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| Committed event-script files + server-side replay mechanism (rio/acme/sam input scripts) | config/data + server feature | batch-read + inject | New mechanism (D-10 amended): server-side replay of a committed event script in the drawterm-derived server's mouse/kbd read queues. No in-tree scripted-input replay exists. Format is planner/agent discretion — keep it line-based, timestamp-optional (deterministic under TCG), committed as data beside the traces. OS-level injection (osascript/cliclick) stays an optional eyeball leg; cliclick is NOT installed. |
| drawterm serve-only LISTEN loop | host tool | event-driven 9P server | Partial analogs only: drawterm's `cpu.c`+`exportfs/` is a dial-out+rexcall export (not a listener); `build/harness/src/srv9.rs` is a Rust fs-root server (wrong language/device-tree). The serve-only main (bind kern/ devices → listen 127.0.0.1 → export) is a small new composition of proven parts; the gui-cocoa backend + devkbd.c must come from the 9front-lineage drawterm (nixpkgs `drawterm-0-unstable-2026-10-12`; not currently in the local nix store — re-fetch at execution time). In-tree `sys/src/cmd/unix/drawterm/gui-osx` is 32-bit Carbon — do not attempt to build it. |

## Metadata

**Analog search scope:** `build/aarch64/` (tiers, markers, sweeps, drivers, bake harness, tracesup), `build/harness/src/` (trace.rs, main.rs), `sys/src/` (mkfile filter sites, cmd/{ip,auth,aux,ndb,games,gs,unix}, ape/lib/ap), `usr/glenda/lib/profile`, `lib/ndb/`, `.github/workflows/`, `.build/9front-donor/` (gitignored donor — vendoring source only)
**Files scanned:** 28 read/grepped; all 34 classification rows' analogs verified git-tracked via `git ls-files --error-unmatch` (2026-11-05)
**Pattern extraction date:** 2026-11-05
