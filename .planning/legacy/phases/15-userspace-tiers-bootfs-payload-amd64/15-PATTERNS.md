# Phase 15: Userspace Tiers + Bootfs Payload (amd64) - Pattern Map

**Mapped:** 2026-11-09
**Files analyzed:** 35 (new + modified, harvey repo)
**Analogs found:** 33 / 35 (this phase is, by design, orchestration of proven Phase-14 machinery — nearly every new file is a clone of an in-tree shape)

All analog paths below verified git-TRACKED (`git ls-files` non-empty) this session. The r9 kernel fixes (D-01) live in `../r9` and are out of this map's scope; the plan tasks reference them by discipline, not by harvey-side analog.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `build/x86_64/tiers/tier2-services.txt` | config (data) | file-I/O | `build/x86_64/tiers/tier1-cmds.txt` | exact |
| `build/x86_64/tiers/tier3-gui.txt` | config (data) | file-I/O | `build/x86_64/tiers/tier1-cmds.txt` | exact |
| `build/x86_64/tiers/tier4-tail.txt` | config (data) | file-I/O | `build/x86_64/tiers/tier1-cmds.txt` | exact |
| `build/x86_64/tier2.ledger` (+3,4; names planner discretion) | config (ledger) | file-I/O | `build/x86_64/tier1.ledger` | exact |
| `build/x86_64/services-sweep.rc` | service-script (rc) | batch | `build/x86_64/tier1-sweep.rc` | exact (probe bodies per RESEARCH Pattern 8) |
| `build/x86_64/services-termrc` | config (termrc override) | file-I/O | `build/x86_64/tier-termrc` | exact |
| `build/x86_64/services-r9.expect` | gate (expect) | request-response | `build/x86_64/tier1-r9.expect` | exact |
| `build/x86_64/dir-coverage.txt` | config (proof output) | file-I/O | `build/aarch64/dir-coverage.txt` | exact |
| `build/x86_64/coverage-sweep.sh` (re-key) | utility (driver) | batch | `build/aarch64/coverage-sweep.sh` | exact (re-key precedent: `build/x86_64/subset-build.sh` is aarch64 subset-build re-keyed) |
| payload manifest data file (name planner discretion) | config (data) | file-I/O | `build/x86_64/tiers/tier1-cmds.txt` (plain-data) + mkimage printed manifests | role-match |
| `build/x86_64/root-tree/lib/ndb/local.template` | config (template) | transform | `build/x86_64/root-tree/lib/netaddr.rc` (seam source) | role-match (token template is new) |
| `build/x86_64/root-tree/usr/glenda/lib/profile` (modify) | config | file-I/O | `usr/glenda/lib/profile` (D-17 trim reference, verbatim source) | exact |
| `build/x86_64/mkimage.sh` (modify: 5th contract + payload) | utility (bake) | batch | its own sections 8/9/10 (mkimage.sh:210-574) | exact |
| `build/x86_64/subset-build.sh` (modify: tier2/3/4 + anti-staleness) | utility (build driver) | batch | its own tier-1 dispatch (subset-build.sh:158-242) | exact |
| `sys/src/mkfile` (modify: LIBS/CMDS/SUBSYS filters) | config (mk) | batch | its own amd64 arm (sys/src/mkfile:68-82) | exact |
| `sys/src/cmd/mkfile` (modify: TARG/DIRS filters) | config (mk) | batch | its own amd64 arm (sys/src/cmd/mkfile:29-37) | exact |
| `sys/src/cmd/ip/mkfile` (modify) | config (mk, container) | batch | its own amd64 arm (sys/src/cmd/ip/mkfile:31-41) | exact |
| `sys/src/cmd/auth/mkfile` (modify) | config (mk, container) | batch | its own amd64 arm (sys/src/cmd/auth/mkfile:47-57) | exact |
| `sys/src/cmd/auth/factotum/mkfile` (modify) | config (mk, leaf) | batch | its own T1KEEPT arm (factotum/mkfile:6-19) | exact |
| `sys/src/cmd/aux/mkfile` (modify: NEW leaf filter) | config (mk, container+leaf) | batch | `sys/src/cmd/auth/factotum/mkfile` T1KEEPT pattern | role-match (aux has NO filter today — Pitfall 3) |
| `sys/src/9k/k10/k10draw` (new config; name discretion) | config (kernel) | batch | `sys/src/9k/k10/k10f` (device set) + `sys/src/9k/k10/k10t1` (header discipline) | exact (composition of two) |
| `sys/src/9k/k10/vgadispi.c` (new VGAdev; route 1, name discretion) | driver (kernel device) | transform | `sys/src/9k/386/vgavesa.c` (VGAdev struct) + `sys/src/9k/k10/screen.c:459-501` (vgalinearpci) | partial (DISPI I/O programming itself is new) |
| `sys/src/9k/k10/multiboot.c` (modify: ~3-line addbootfile delta) | driver (kernel) | file-I/O | in-file Fmods loop (multiboot.c:83-96) + `sys/src/9k/port/devroot.c:79-83` | exact |
| `sys/src/9k/port/devcons.c` (modify: kbin-class write file) | driver (kernel device) | event-driven | `sys/src/9k/port/devmouse.c` Qmousein (dirtab+iseve+write) + devcons kbdputc | exact (as a pattern composition) |
| `sys/src/9k/k10/root/k10draw.rc` (new venue boot script) | config (rc boot) | event-driven | `sys/src/9k/k10/root/k10t1.rc` | exact |
| `build/mm/build-9kdraw.sh` | utility (venue build) | batch | `build/mm/build-9k10t1.sh` | exact |
| `build/mm/boot-9kdraw-trace.sh` | gate (venue harness) | request-response | `build/mm/boot-9k10t1-sweep.sh` | exact |
| replay scripts (3 committed rc data: rio/acme/sam) | config (data) | event-driven | `sys/src/9k/k10/root/k10t1.rc` (in-guest rc discipline) + mousein vocabulary (devmouse.c:427-444) | role-match |
| tracesup amd64 build (6c/6l of `build/aarch64/tracesup.c`) | utility (tracer) | streaming | `build/aarch64/tracesup.c` + the 6c-helper build precedent (subset-build.sh:285-289) | exact |
| `nix/services-r9-x64.sh` + `flake.nix` app (modify) | gate (nix app) | request-response | `nix/tier1-r9-x64.sh` + flake.nix:195-199,245-251 | exact |
| `.github/workflows/phase6.yml` (modify: tier compile gates) | gate (CI) | batch | phase6.yml's existing tier jobs (phase6.yml:238-239) | exact |
| `sys/src/libmemdraw/drawtest.c` (modify: LP64 fixes) | test (in-tree) | transform | itself; fix discipline = Phase-6 class-sweep | self |
| `sys/src/libmemdraw/arctest.c` (modify: LP64 fixes) | test (in-tree) | transform | itself | self |
| gs .ps render fixture (new, tiny, committed) | config (fixture) | file-I/O | `sys/src/cmd/gs/examples/*.ps` (placement precedent) | partial (no gate-driven fixture exists) |
| `.planning/phases/15-*/evidence/` (trace raw + distilled tables) | config (evidence) | file-I/O | `.planning/milestones/v1.0-phases/05-*/05-03-trace.txt` (05-03 format lineage) | role-match |

## Pattern Assignments

### Group A: Tier data files — `build/x86_64/tiers/tier{2,3,4}-*.txt`

**Analog:** `build/x86_64/tiers/tier1-cmds.txt` (108 lines, read in full)

Entry-form grammar (header, lines 17-28) — the new files inherit verbatim:
```
#   cmd/<single>           single-file command in cmd/ (cmd/cat)
#   cmd/<dir>              whole dir recursed by the sweep (cmd/rc)
#   cmd/<container>/<dir>  nested dir inside a container (cmd/ip/ipconfig)
#   cmd/<dir>:<target>     the dir is kept in the sweep; the <target>
#                          qualifier is consumed by the tier driver
#                          (per-target `mk <target>.install`, never
#                          whole-dir) and by filter-equipped leaf mkfiles
```
Entry lines carry trailing `#` comments (the exclusion/rationale record — D-10's named-exclusion rule rides here):
```
cmd/rc				# dir; the boot chain's shell + the sweep's hrc parser check
cmd/ip/ipconfig			# the 14-09 net gates' binary; k10cpu's devroot embed member
cmd/auth/factotum:factotum	# fgui excluded: needs the out-of-tier graphics stack (libdraw/libcontrol)
cmd/paqfs:paqfs			# mkpaqfs stays host-only: only the host bakes images...
```
(lines 63, 102-108)

**Constraints the new files must keep:** APPEND-ONLY, never reorder/edit tier0/tier1 lines (header lines 12-15); every consumer (5 mk filters, subset-build.sh, mkimage.sh 9b, both venue gates) computes expectations from these files — never a second hardcoded list. The documented-exclusion comment block (lines 49-60) is the shape for tier-4's re-derived exclusion classes.

### Group B: Tier ledgers — `build/x86_64/tier{2,3,4}.ledger`

**Analog:** `build/x86_64/tier1.ledger` (37 lines)

Line grammar, OK form (line 1) and SKIP form (line 36):
```
tier=1 dir=cmd/rc bin=rc sha256=8a6a... exec=T1-OK:r9x64 evidence=.planning/phases/14-.../14-08-tier1-r9-x64.log commit=bce0f69f... date=2026-11-09
tier=1 dir=cmd/ip/ipconfig bin=ipconfig sha256=0226... exec=SKIP:r9x64-no-devrandom evidence=... commit=... date=2026-11-09
```
Phase-15 extensions per RESEARCH: venue names widen to `k10t1`/`r9x64` (exec=`T2-OK:k10t1` + `T2-OK:r9x64` or named `SKIP:r9x64-<reason>` / `SKIP→18:<substrate>`); marker prefix per tier is planner discretion (D-12). The append-only + sha256-artifact discipline is carried verbatim.

### Group C: The services sweep — `build/x86_64/services-sweep.rc`

**Analog:** `build/x86_64/tier1-sweep.rc` (307 lines, read in full)

Core never-wedge check helper (lines 79-84):
```rc
fn check {
	n=$1
	shift
	if($*) echo T1-OK $n
	if not echo T1-FAIL $n
}
```
Fixed-input check shape (lines 101-103) and the path-build assert idiom (lines 203-204 — single quotes never expand in rc, so venue paths are BUILT with `^`):
```rc
catout=`{$B/hcat $D/sentinel.txt}
if(~ $"catout 'alpha bravo charlie '*) echo T1-OK cat
if not echo T1-FAIL cat
...
if(~ $"md5out '6c3d57e94871f9606c5b63c315e76135 '^$D^/sentinel.txt) echo T1-OK md5sum
```
Named-SKIP block at the tail (lines 295-306) — the D-02/D-08 venue-honesty shape; the r9 leg's SKIP lines must move in the SAME commit as the tier-2 append (Pitfall 4: the ipconfig SKIP at line 303 flips live — sweep + gate expectations in one commit):
```rc
echo T1-SKIP ipconfig
echo T1-SKIP factotum

echo T1-DONE
exit
```
Daemon probes replace fixed-input checks: each probe gets per-probe internal timeouts + named FAIL markers (Pitfall 5 — a daemon never wedges the sweep). Probe client programs are all in-tree (RESEARCH Don't-Hand-Roll table): `ndb/csquery`, `ndb/dnsquery`, `dial/pass`, `srv -e 'hexportfs -r ...'`, `mntgen -s`. Readnvram's amd64 → `"pc"` mapping and open-fail → zero-key+warning path is verified at `sys/src/libauthsrv/readnvram.c:170-173` (keyfs probe over an ABSENT file serves an empty DB — no keyfile generator).

### Group D: `build/x86_64/services-termrc`

**Analog:** `build/x86_64/tier-termrc` (17 lines, whole file)
```rc
#!/bin/rc
# tier-termrc — the TIER image's /rc/bin/termrc (14-08, XEXEC-04):
# ... the in-image tier-1 sweep runs from the boot flow, exactly as the
# DONOR venue runs it ... The sweep runs in a CHILD rc (its trailing
# exit never kills init's shell); the interactive %  prompt follows
# T1-DONE.

rc /lib/tier1-sweep.rc r9

exec rc -i
```
The services termrc is this file with the sweep line swapped (`. /lib/services-sweep.rc`-invoked in a child rc, venue argument per the gate) — the net bring-up ordering (ipconfig BEFORE daemons, 14-09 netip-static flow) rides the sweep itself or a pre-sweep block, planner's call within D-03's letter.

### Group E: `build/x86_64/services-r9.expect`

**Analog:** `build/x86_64/tier1-r9.expect` (206 lines, read in full)

Copy-verbatim machinery: env contract check (lines 70-75), fresh-capture rule (line 82), qemu spawn with caller-selected q35 shape (lines 90-96), group-kill cleanup proc (lines 105-109), named-stage proc (lines 115-122), `expect_before` panic fast-fail (lines 126-132), and the whole-file post-pass with SKIP-set EQUALITY (lines 178-188):
```tcl
set gotraw [regexp -all -inline {T1-SKIP ([A-Za-z0-9.-]+)} $flat]
...
set got [lsort -unique $got]
set want [lsort -unique $env(R9_EXPECT_SKIP)]
if {$got ne $want} { ... exit 1 }
```
Changes for services: marker vocabulary (SVC-OK/FAIL/SKIP class), expected sets computed from `tiers/tier2-services.txt` + the sweep's own SKIP lines (driver side: `nix/tier1-r9-x64.sh:157-182` is the computation to clone), and D-02's probe-completion stages (each daemon earns a stage asserting its probe marker, presence-only, CRLF-tolerant).

### Group F: `build/x86_64/mkimage.sh` — fifth contract + payload (modify)

**Analog:** its own sections — clone section 9 (tier bake, lines 299-447) / section 10 (net bake, lines 449-574) as section 11. Load-bearing excerpts:

Computed staging set from the tier data — never a second list (lines 338-348):
```sh
TIER1_RELS=$(
	awk 'NF && $1 !~ /^#/ { print $1 }' "$HARVEY/build/x86_64/tiers/tier1-cmds.txt" |
	while read -r e; do
		case "$e" in
		9k/boot|stage/*|cmd/paqfs:paqfs|cmd/cc) continue ;;
		esac
		case "$e" in *:*) e="${e%%:*}" ;; esac
		echo "${e#cmd/}"
	done
)
```
h* payload staging + printed manifest (lines 384-388, 406-421):
```sh
for rel in $TIER1_RELS; do
	[ -f "$BIN/$rel" ] || err "$BIN/$rel missing — ..."
	cp "$BIN/$rel" "$STAGE3/amd64/bin/h${rel##*/}"
done
...
for p in boot/boot ... usr/glenda/lib/profile lib/tier1-sweep.rc ...; do
	[ -f "$STAGE3/$p" ] || err "tier manifest: $p missing from staged tree"
	echo "tier manifest: /$p staged"
done
```
Auth-stub-by-absence (lines 426-429), termrc override (lines 396-404), KERNDATE determinism + magic checks (lines 435-445), bake-time verbatim copy precedent for the 36MB fonts (lines 253-257 — the adm/timezone rule: repo data copied at bake, never a committed duplicate):
```sh
cp "$HARVEY/rc/lib/rcmain" "$STAGE2/rc/lib/rcmain"
cp "$HARVEY/adm/timezone/local" "$STAGE2/adm/timezone/local"
```
The D-11 seam grep the ndb template MUST pass (lines 488-489):
```sh
_stray=$("$GREP" -rn '10\.0\.2\.' "$ROOT_TREE/lib/" 2>/dev/null | "$GREP" -v 'netaddr\.rc' || true)
[ -z "$_stray" ] || err "venue address literal outside netaddr.rc (the D-11 seam violated): $_stray"
```
DESK-03 additions per this shape: font copy + manifest-data-file consumption (D-15: bake FAILS on any listed path missing/empty — same `[ -f ... ] || err` form; count/size floors + the three app-named fonts per RESEARCH Pattern 9), and the ndb template sed+leftover-token assert (RESEARCH Code Examples has the exact snippet). NOTE the seam grep scopes to `$ROOT_TREE/lib/` — the committed `local.template` with `@TOKEN@` placeholders passes it only if it carries no `10.0.2.` literal; assert zero leftover `@` post-substitution.

### Group G: `build/x86_64/subset-build.sh` — tier2/3/4 dispatch + anti-staleness (modify)

**Analog:** its own tier-1 dispatch (lines 158-242). Entry-form case blocks (lines 175-239): `9k/boot` (build-in-tree + harvest to out/ + clean), `stage/init` (`mk init.install` + harvest), `cmd/cc` (archive artifact), `cmd/*:*` (per-target `mk "$tgt.install"`), `cmd/<dir>` vs `cmd/<single>` (dir = `mk install`; single = LINK-ONLY `mk 6.<cmd>` + driver cp harvest — the 14-07 `._cp` lesson, lines 217-234):
```sh
name="${entry#cmd/}"
(cd /sys/src/cmd && objtype=amd64 mk "6.$name") || fail "cmd $entry"
cp "/sys/src/cmd/6.$name" "/amd64/bin/$name" || fail "harvest $name"
rm -f "/sys/src/cmd/6.$name" "/sys/src/cmd/$name.6"
```
Anti-staleness insertion point: the tier-0 D-07 fresh-archive rule (lines 141-156, `rm -f "/amd64/lib/$lib"` before `mk install` because `ar vu` never prunes) is the in-driver precedent; the 06-03 decision lands in the FIRST tier-2 plan as per-entry `mk clean` in the tier driver loop before each `mk install` (RESEARCH recommends; donor-vm-build step-4b precedent). Tier-2/3/4 appends = a second/third/fourth `while read` loop over the new data files (or one loop over a concatenated set — the entry grammar is uniform); marker prefixes widen (`T2-ART` etc., planner discretion).

The 6c/6l host-side helper build (lines 285-289) is the exact precedent for building `tracesup` for amd64:
```sh
for h in netblast tcpblast tcpecho; do
	(cd "$HARVEY/build/x86_64" && objtype=amd64 6c -FTVw "$h.c" && objtype=amd64 6l -o "$HARVEY/build/x86_64/out/$h" "$h.6" && rm -f "$h.6") || fail "net helper $h"
```

### Group H: The five mk filters + the NEW aux leaf filter

**Analogs (each file extends its own amd64 arm; read in full):**

`sys/src/mkfile:68-82` (LIBS/CMDS/SUBSYS):
```mk
TIER1A=$HARVEY/build/x86_64/tiers/tier1-cmds.txt
T1KEEPA=`{awk '/^[a-z0-9]/{ sub(/[\/: 	].*$/,""); print }' $TIER1A}
LIBS=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLLIBS) if(~ $i $T0KEEP) echo $i; if not if(~ $objtype amd64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLLIBS) if(~ $i $T0KEEPA) echo $i; if not echo $ALLLIBS}
```
`sys/src/cmd/mkfile:29-37` (TARG/DIRS — awk anchored on `^cmd\/`), `sys/src/cmd/ip/mkfile:31-41` and `sys/src/cmd/auth/mkfile:47-57` (container variants anchored `^cmd\/ip\/` / `^cmd\/auth\/`).

**The extension (RESEARCH Pattern 1):** each amd64 `*KEEPA` becomes ONE multi-file awk call:
```mk
T1KEEPA=`{awk '/^cmd\//{ sub(/^cmd\//,""); sub(/[\/: 	].*$/,""); print }' $TIER1A $T2 $T3 $T4}
```
The plan9port-mk constraint is documented at `sys/src/mkfile:64-67`: backquote recipes must be SINGLE-LINE, brace-free, awk program single-quoted (mk's lexer stops at the first unquoted `}`).

**The NEW aux leaf filter (Pitfall 3):** `sys/src/cmd/aux/mkfile` today has NO filter — a 39-member `TARG` mkmany list (lines 3-42) + 6 `DIRS` (lines 53-58). A `cmd/aux:listen` (or `cmd/aux:mouse`) tier entry keeps `aux` whole in cmd's DIRS unless a leaf filter lands. Copy the factotum dir:target pattern (`sys/src/cmd/auth/factotum/mkfile:6-19`):
```mk
TIER1A=$HARVEY/build/x86_64/tiers/tier1-cmds.txt
T1KEEPTA=`{awk '/^cmd\/auth\/factotum:/{ sub(/^cmd\/auth\/factotum:/,""); sub(/[\/: 	].*$/,""); print }' $TIER1A}
TARG=`{if(~ $objtype aarch64 && ...) ...; if not if(~ $objtype amd64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLTARG) if(~ $i $T1KEEPTA) echo $i; if not echo $ALLTARG}
```
(re-keyed to `^cmd\/aux:`) — and it lands in the SAME commit as the first aux tier entry.

### Group I: Draw donor config — `sys/src/9k/k10/k10draw` (name discretion)

**Analogs:** `sys/src/9k/k10/k10f` (draw device set, read in full) + `sys/src/9k/k10/k10t1` (venue header discipline, read in full)

The k10f deltas over k10cpu are exactly the draw set (k10f:25-28, 39-40, 65, 193-199):
```
	draw		drawalloc screen vga vgax
	mouse		mouse
	vga
	kbmap
...
vga +dev
	vgavesa
...
	realmode          (in link)
...
 lib
	libmemlayer
	libmemdraw
	libdraw
```
Config discipline from k10t1's header (lines 11-14): "The device/dbgflg/mach/port/dir/lib sections are k10cpu's set VERBATIM ... DO NOT PRUNE" — so the draw variant is a NEW sibling config file; k10cpu/k10t1 stay untouched; `k10/mkfile:3` `CONFLIST` need not change (`CONF=` override builds any config — `build-9k10t1.sh:146-148` proves it: `mk CONF=k10cpu 9k10cpu` / `mk CONF=k10t1 9k10t1`).

Parse grammar mapping (verified `sys/src/9k/mk/parse:294-317, 352-371`): `dev` lines → `&<name>devtab` (draw→devdraw.c, mouse→devmouse.c, vga→devvga.c, kbmap→devkbmap.c); extra words are bare objects (drawalloc/screen/vga/vgax → port/drawalloc.c, k10/screen.c, k10/vga.c, k10/vgax.c); the `vga +dev` section emits `vgadev[]` + `vgacur[] = { &swcursor }`; `rootdir` entries each emit one `addbootfile(...)` line; `dir` must list `386` for `386/vgavesa.c` to be reachable.

**THE GAP (route choice):** with `vgavesa`, `vbecall()` opens `/dev/realmode` (`sys/src/9k/386/vgavesa.c:59-64`) which no k10 kernel serves — Enonexist at runtime (Pitfall 1). Route 1 (recommended): a new small `vgadispi.c` VGAdev + `vga` section entry, LFB via in-tree `vgalinearpci()` — `vgavesa`/`realmode` NOT in the config. Route 2 (fallback): port the realmode trampoline (legacy `sys/src/9/pc/realmode.c`, `pc/l.s:323`, `pc/mem.h:61-63`).

### Group J: `sys/src/9k/k10/vgadispi.c` (new VGAdev, DISPI route)

**Analogs:** `sys/src/9k/386/vgavesa.c` (VGAdev struct + linear init) + `sys/src/9k/k10/screen.c` + `sys/src/9k/k10/devvga.c`

The VGAdev struct to implement (vgavesa.c:220-231 — 9front/Plan 9 `screen.h` shape):
```c
VGAdev vgavesadev = {
	"vesa",
	0,		/* enable */
	0,		/* disable */
	0,		/* page */
	vesalinear,	/* linear */
	0,		/* vlctl */
	0,		/* vmode */
	0,		/* move */
	0,		/* scroll */
	vesaflush,	/* flush */
};
```
LFB mapping — call, don't hand-roll (`screen.c:459-501`): `vgalinearpci(scr)` scans display-class PCI BARs (`p->ccrb != 3` skip, largest-prefetchable heuristic); `vgalinearpciid(scr, vid, did)` when matching a specific device. qemu std-vga at 00:01.0 exposes a 16MB BAR0 (RESEARCH A1).

Port reservation + vgactl verbs already exist (`devvga.c:69-76` ioalloc of the standard VGA ports; `devvga.c:51-66` cmdtab incl. `CMsize, "size", 3` and `CMdrawinit, "drawinit", 1`). Mode-set choreography (userspace-driven, via `/dev/vgactl`): `size 1024x768x32 x8r8g8b8` → `drawinit`; `screensize()` (`screen.c:49-109`) builds gscreen — soft-screen malloc path when `scr->paddr == 0`, hard LFB + flush otherwise. The DISPI register programming (I/O ports 0x1CE/0x1CF: ID check, XRES, YRES, BPP, ENABLE|LINEAR) is the genuinely NEW code — no in-tree analog (see No Analog Found).

### Group K: `sys/src/9k/k10/multiboot.c` — the addbootfile module delta (modify)

**Analog:** in-file + devroot API. The Fmods loop today (multiboot.c:83-96):
```c
if(mbi->flags & Fmods){
	for(i = 0; i < mbi->modscount; i++){
		mod = KADDR(mbi->modsaddr + i*16);
		if(mod->string != 0)
			p = KADDR(mod->string);
		else
			p = "";
		if(vflag)
			print("mod %#ux %#ux <%s>\n", mod->modstart, mod->modend, p);
		else
			asmmodinit(mod->modstart, mod->modend, p);
	}
}
```
The delta (RESEARCH Pattern 5): in the `else` branch also `addbootfile(name, KADDR(mod->modstart), mod->modend - mod->modstart)` — the devroot API the rootdir embed itself uses (`sys/src/9k/port/devroot.c:79-83`):
```c
void
addbootfile(char *name, uchar *contents, ulong len)
{
	addlist(&bootlist, name, contents, len, 0555);
}
```
Ceiling: `Nbootfiles = 64` with the 06-09 raise comment (`devroot.c:13-14`) — the donor services embed (~20 extra binaries) must fit or the ceiling raises again (RESEARCH A6). The module name for `addbootfile` should be a stable `/boot`-visible name the venue boot script then feeds to `paqfs` (the tier-1-built paqfs; the same bootfs.paq mechanism r9 uses).

### Group L: `sys/src/9k/port/devcons.c` — kbin-class write file (modify, ~a dozen lines)

**Analog (composition):** devmouse.c's mousein — dirtab row, eve-gate, text-parse write handler — feeding devcons's own kbdputc.

dirtab row (`devmouse.c:91-97`): `"mousein", {Qmousein}, 0, 0220,` — the new cons entry mirrors it (e.g. `"kbin", {Qkbin}, 0, 0220,` appended to `consdir[]` at `devcons.c:684-709`).

Open gate (`devmouse.c:192-195`) — the security-domain requirement (RESEARCH: the kbin addition MUST take the same eve-gate):
```c
case Qmousein:
	if(!iseve())
		error(Eperm);
	break;
```
Write handler shape (`devmouse.c:427-444` — first char skipped, strtols, kernel-timestamp fallback, feed the track entry):
```c
case Qmousein:
	if(n > sizeof buf-1)
		n = sizeof buf -1;
	memmove(buf, va, n);
	buf[n] = 0;
	...
	mousetrack(pt.x, pt.y, b, msec);
	return n;
```
The kbin write handler feeds the SAME rune path the i8042 keyboard uses — `kbdputc(kbdq, c)` (`sys/src/9k/386/kbd.c:477`; the function itself is `sys/src/9k/port/devcons.c:602-627`, declared `portfns.h:144`). Serial console alternative (zero kernel work): k10's `uartconsole()` already registers the uart input queue via `addkbdq(p->iq, -1)` (`sys/src/9k/port/devuart.c:149`) so `/dev/cons` reads see serial input — host-anchored determinism (marker-anchored sends, the expect-gate rig).

### Group M: Draw venue boot script — `sys/src/9k/k10/root/k10draw.rc`

**Analog:** `sys/src/9k/k10/root/k10t1.rc` (52 lines, read in full). Load-bearing idioms: the `-m /boot/rcmain` shebang (line 1 — NOT optional, devroot has no compiled-in rcmain); `path=(/boot /bin .)` (line 23); bare binds `bind '#p' /proc; bind '#d' /fd; bind -b /boot /bin` (lines 28-30); the venue plumbing block (lines 40-41: `hcat '#r/rtc' > /dev/time`, `echo -n sys > '#c/hostowner'` — the empty-hostowner ps lesson); the child-rc sweep invocation `rc -m /boot/rcmain -c '. /boot/tier1-sweep.rc 9k'` (line 47); and pid-1-never-exits `while(~ 1 1){ . -i '#d/0' }` (line 52). The draw variant adds: paqfs over the `-initrd` module exposed via Group K (`paqfs -m /n/paq ...` over the addbootfile'd module), font/GUI subtree binds, `size`+`drawinit` writes to `#v/vgactl`, mntgen scratch for `/tmp` (trace fragments), then the replay script + tracesup launch. Console truth: k10 console output fans to BOTH cga and serial — serial is the capture (RESEARCH Pattern 6 note).

### Group N: `build/mm/build-9kdraw.sh` + `boot-9kdraw-trace.sh`

**Analogs:** `build/mm/build-9k10t1.sh` (359 lines) + `build/mm/boot-9k10t1-sweep.sh` (674 lines), both read in full.

build script shape: platform gate (lines 53-59), named-remediation preconditions (61-83), the host-grep resolve (80-83), staging with digest-pinned fixtures (85-110), the nuke-sandwich namespace CORE with `CONF=` override (137-175 — `mk nuke; mk CONF=k10draw 9kdraw; ...; mk nuke` + the `addbootfile("` count assert against devroot's ceiling), venue split sudo/userns (177-287), git-clean trailing assert (311-320), and the r9-leg manifest writer (337-359 — the tier-data-driven member computation; the draw venue likely does NOT need an r9 leg, but the font/payload paq bake feeds its `-initrd`).

harness shape: self-contained qemu flags (`run_9k_leg`, lines 598-604: `-kernel $KERNEL -m 512 $ACCEL_ARGS -display none -no-reboot -serial file:$LOG`; the draw leg adds `-vga std` and `-initrd bootfs-draw.paq`), capture-before-start (line 598 `: > "$LOG"`), accel probe once (566-575), banner→completion deadline polling with signature-line early break (609-648), classify with boundary-anchored markers `T1-OK m([^A-Za-z0-9.-]|$)` (199) and the SKIP-whitelist converse (216-221). The trace gate asserts script COMPLETION markers per D-07 (the replay script's final action per program lands a marker on serial) — presence-only, never interleaving order, TCG-generous budgets (Pitfall 5). NO srv9/9front assumptions anywhere (watch-out): fragment egress is the serial capture after the session (tracesup fragments live in the mntgen scratch; serialized out at session end).

### Group O: Trace rig — tracesup amd64 + replay scripts + evidence

**Analogs:** `build/aarch64/tracesup.c` (321 lines; header read — arch-neutral Plan 9 C, `<u.h> <libc.h> <bio.h>`). Arming discipline carries verbatim (header lines 17-29): born-traced children are discovered Stopped with the entry fragment PENDING — read the pending fragment BEFORE the first `startsyscall` write (the write blocks; a racing write silently drops `Proc_tracesyscall`). 9k devproc serves the substrate (`/proc/N/syscall`, `startsyscall` ctl verb — `sys/src/9k/port/devproc.c:98,115`). Build it with 6c/6l via the Group G helper precedent (A5: may need trivial 7c-ism port). Distilled-table format lineage: `.planning/milestones/v1.0-phases/05-shell-boot-one-command-integration/05-03-trace.txt` (the 05-03 measured-set format: syscall → count → args-shape notes). Evidence lands under `.planning/phases/15-*/evidence/` (the 14-08 precedent: `evidence=.planning/phases/14-*/evidence/14-08-tier1-r9-x64.log` in the ledger).

Replay scripts (committed rc data) drive: `/dev/mousein` text writes (`echo 'M 512 384 0 0' > '#m/mousein'` — format verified `devmouse.c:427-444`; open is `iseve()`-gated so the replay runs as hostowner) and the kbin-class file (Group L) or serial chardev. In-guest rc script discipline = Group M's k10t1.rc shape (path setup, child rc, never kill init).

### Group P: nix gate app — `nix/services-r9-x64.sh` + flake wiring

**Analog:** `nix/tier1-r9-x64.sh` (237 lines, read in full) + `flake.nix:195-199` (the `mkScriptApp` def) and `flake.nix:245-251` (the `optionalAttrs` hasAttr gate where x64 compose apps live).

Clone: the kernel-package/iso-builder contract (lines 90-103), no-silent-downgrade accel (105-117), KERNDATE provenance (124-132), step-1 build / step-2 bake substitution, the computed expectations block (150-182 — OK set from tier data minus driver-only minus SKIP-named plus canary; SKIP set grepped verbatim from the sweep's own `echo T1-SKIP` lines), the bracket-fixed pkill survivor sweep keyed on the unique run tag (186-204), ISO compose (206-210), expect launch (228-235). Flake side, copy the `tier1R9X64Pkg` block:
```nix
tier1R9X64Pkg = mkScriptApp pkgs "harvey-boot-r9-tier1-x64" ./nix/tier1-r9-x64.sh ''
  export R9_KERNEL_X64_PKG="${r9.packages.${system}.kernel-x64}"
  export R9_ISO_BUILDER="${r9.packages.${system}.iso-builder}/bin/r9-iso-builder"
'' ([ pkgs.plan9port pkgs.bash pkgs.qemu pkgs.git pkgs.expect ]
  ++ pkgs.lib.optionals isLinux [ pkgs.util-linux pkgs.gnused pkgs.mount pkgs.binutils ]);
```
and the app registration inside the `kernel-x64`+`iso-builder` hasAttr gate.

### Group Q: coverage re-key — `build/x86_64/coverage-sweep.sh` + `dir-coverage.txt`

**Analogs:** `build/aarch64/coverage-sweep.sh` (645 lines; COVERAGE_SWEEP=1 passthrough, per-dir VISIT/EXIT markers, watchdog) → re-keyed to `objtype=amd64` with output `build/x86_64/dir-coverage.txt`. The re-key precedent for the whole file is `build/x86_64/subset-build.sh` itself (the aarch64 driver re-keyed — "re-key, don't reinvent" is its line 4). Output grammar: `build/aarch64/dir-coverage.txt` (303 lines) is the machine-verified proof format the D-09 proof replicates. The COVERAGE_SWEEP=1 passthrough hooks already exist in all five mk filters (Group H) — the sweep sets it so the coverage run certifies the whole tree, never the allowlist to itself (`sys/src/mkfile:53-57`).

### Group R: DESK-03 payload data — ndb template + trimmed profile

**`local.template` analog (constraint source):** `build/x86_64/root-tree/lib/netaddr.rc:27-44` defines the substitution keys (`slirp_guestv4=10.0.2.15`, `slirp_maskv4=255.255.255.0`, `slirp_gwv4=10.0.2.2`, `lo_guestv4=127.0.0.1`, `blastport=7777`) — rc-syntax key=value, one commented section per leg. The template carries `@TOKEN@` placeholders (planner picks syntax), NO literals (Group F's seam grep). mkimage sources netaddr.rc and seds; asserts substituted values present + zero leftover tokens.

**`usr/glenda/lib/profile` (the D-17 trim source, read verbatim — 36 lines):** trims are — drop line 7 `upas/fs` and line 28 `news`; terminal case (lines 10-16) keeps binds/tmp/font (`font = /lib/font/bit/pelm/euro.9.font`, line 6) but LOSES line 16 `exec rio -i riostart`; cpu case (lines 17-32) INTACT including `bind /mnt/term/dev/cons`, `font=/lib/font/bit/pelm/latin1.8.font` (line 30), `exec rio` (line 31). Watch the terminal case's `'#m/mousectl'` writes (lines 12-13) and `plumber` (line 11): on a serial-console image without devmouse these fail loudly — the trim/guard shape is planner discretion within D-17's letter. The file being REPLACED is the 3-line skeleton `build/x86_64/root-tree/usr/glenda/lib/profile` (comment-only today).

### Group S: in-tree fixes + CI + gs fixture

**libmemdraw (drawtest.c/arctest.c):** the files themselves (drawtest.c: stochastic memimagedraw verification, RGB2K macro; arctest: wild-pointer write per the aarch64 measurement). No code analog — the pattern is the Phase-6 landmine discipline: root-cause, fix in-tree, one commit per bug class, BEFORE/alongside trace capture (Pitfall 8: garbage pixels poison the distilled tables). `sys/src/libmemdraw/mkfile`'s stale test targets are the same inherited checklist (Phase-6 deferred-items).

**CI legs:** `.github/workflows/phase6.yml` is the analog — its tier jobs (e.g. `tier-1 r9 gate (boot-r9-tier1)` at phase6.yml:238-239 invoking `nix run .#boot-r9-tier1`) plus its venue-pin self-check (phase6.yml:200-206 — CI greps itself for stale donor references). Tier compile gates in CI follow this job shape; donor execution legs per the venue precedent (planner topology decision).

**gs .ps fixture:** placement precedent `sys/src/cmd/gs/examples/*.ps` (alphabet.ps, chess.ps, ...) — but no gate-driven fixture exists; a tiny NEW committed .ps rendered to a bitmap FILE device (no display) per D-11. gs's own build route: `sys/src/cmd/gs/mkfile` + `sys/src/cmd/gs/default.amd64.h` + `sys/src/ape/lib/ap/amd64/` (all verified in-tree — stock pcc route, no vendoring).

## Shared Patterns

### Marker vocabulary + venue honesty (apply to EVERY gate/sweep/ledger)
**Source:** `build/x86_64/tier1-sweep.rc:40-50`, `build/x86_64/tier1.ledger`, `build/mm/boot-9k10t1-sweep.sh:199,216-221`
- `<TIER>-OK <name>` only on real execution evidence; a daemon earns it only when a real client interaction succeeds through it (D-02)
- Impossible-on-venue = named `SKIP:<venue>-<reason>` — never a weaker pass, never silent; the gate asserts SKIP-set EQUALITY both directions (an emitted SKIP outside the declared set reds; a missing declared SKIP reds)
- Boundary-anchored marker match: `T1-OK m([^A-Za-z0-9.-]|$)` — the mk<mkdir false-green class
- Markers presence-only, never interleaving order (TCG-timing tolerant); zero FAIL, zero `panic|trap:` lines, DONE marker required

### mk exit status never trusted; artifact asserts are the only gates
**Source:** `build/x86_64/subset-build.sh:32-34,240-241,404-433`; `build/mm/build-9k10t1.sh:289-310`
- sha256 markers per artifact (`T1-ART <bin> <sha256>`), a.out magic checks (`00008a97` amd64), product asserts host-side after the namespace run, embedded-count assert from the generated `$CONF.c`

### Bake contract skeleton (mkimage fifth contract + payload)
**Source:** `build/x86_64/mkimage.sh` sections 8/9/10 (lines 210-574)
stage skeleton tar → mkdir mount-points → cp binaries (fixed order) → computed payload set → bake-time verbatim copies (rcmain/timezone/fonts precedent 8e) → PRINTED manifest (`for p in ...; [ -f ... ] || err; echo "manifest: /$p staged"`) → `/boot/factotum` ABSENCE assert → KERNDATE mtime pin → mkpaqfs → header `529ab12b`/trailer `6b46e688` magic. Payload manifest = committed data file consumed with the same `[ -f ] || err` loop (D-15: no second hardcoded list; missing/empty listed path fails the bake).

### Single-source-as-data + computed expectations
**Source:** tier1-cmds.txt header (lines 3-15); `nix/tier1-r9-x64.sh:157-182`; `build/mm/boot-9k10t1-sweep.sh:145-159,295-319`; `mkimage.sh:338-348`
Every gate computes its expected marker set from the committed tier data with the SAME awk + entry->name mapping (driver-only exclusions `9k/boot|stage/*|cmd/paqfs:paqfs|cmd/cc`; `dir:target` -> target basename), and the SKIP set from the sweep's own `echo T1-SKIP` lines. Phase 15's tier2/3/4 files join the SAME computations (extend every consumer in the same commit as the data append).

### Single-seam addressing (D-16)
**Source:** `build/x86_64/root-tree/lib/netaddr.rc`; the bake grep `mkimage.sh:488-489`
No venue address literal outside netaddr.rc — mechanically asserted at bake. The ndb template is substitution-based BY CONSTRAINT.

### eve-gate on synthetic-input files (security domain)
**Source:** `sys/src/9k/port/devmouse.c:192-195`
Any new kernel write-file that injects input (the kbin-class file) takes the same `iseve()` open gate.

### Kernel error discipline (all kernel deltas)
**Source:** project conventions + `sys/src/9k/386/vgavesa.c` (`error("vesa bios error")` at :76; `error("pci video card not found")` screen.c:453)
`error(E...)`/`error("...")` for device failure; `panic` only for unrecoverable state (devroot's "too many root files" at :63 — the Nbootfiles ceiling behavior the embed-count assert front-runs).

### Sourced-not-typed in-guest sweeps; serial is the only donor egress
**Source:** `build/x86_64/tier-termrc`; `sys/src/9k/k10/root/k10t1.rc:47`; `build/mm/boot-9k10t1-sweep.sh:8-14` (NO srv9, NO 9front tree, NO network devices on the donor venue)

## No Analog Found

Files/mechanisms with no close in-tree match (planner uses RESEARCH.md patterns + sizing):

| File/Mechanism | Role | Data Flow | Reason |
|------|------|-----------|--------|
| DISPI register programming inside `vgadispi.c` (ports 0x1CE/0x1CF sequence) | driver (kernel) | transform | No qemu-VBE DISPI code exists in-tree; the VGAdev struct/linear/vgactl scaffolding has analogs (Group J) but the mode-set I/O sequence is new (RESEARCH A1 flags the qemu-behavior assumption) |
| The draw-over-9P topology rig | — | — | DEFERRED out of phase (D-04) — do not plan it |
| Real p9sk1 handshakes, devcap `#¤`, bake-time `/adm/keys` | — | — | Phase 18-owned (D-01 boundary; D-08 stops short) — named SKIP→18 only |
| r9 kernel-gap fixes (devproc protect/noswap, devsrv-service mounts, whatever the services sweep surfaces) | driver (r9 repo) | — | Live in `../r9` (at pin e560405); no harvey-side code analog — plans reference the push-before-pin / FOD re-wire-then-pin discipline (13-06/14-07 lineage), one own commit per fix |
| Per-dir amd64 compile-failure fixes (D-10 long tail) | various | — | Inherently unknowable pre-sweep; the pattern is the Phase-6 landmine discipline (root-cause, fix in-tree, one commit per class) |

## Metadata

**Analog search scope:** `build/x86_64/`, `build/aarch64/`, `build/mm/`, `nix/`, `.github/workflows/`, `sys/src/9k/{k10,port,386}/`, `sys/src/{mkfile,cmd/mkfile,cmd/{ip,auth,aux}/mkfile}`, `sys/src/libmemdraw/`, `sys/src/libauthsrv/readnvram.c`, `usr/glenda/lib/profile`, `build/x86_64/root-tree/`, `flake.nix`, `.planning/milestones/v1.0-phases/05-*/`
**Files scanned:** 30 analog files read (24 in full, 6 targeted); all verified git-tracked
**Pattern extraction date:** 2026-11-09
