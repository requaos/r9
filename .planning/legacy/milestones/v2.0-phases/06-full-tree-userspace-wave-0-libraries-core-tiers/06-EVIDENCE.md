# Phase 6 — Evidence Bundle

**Phase:** 06-full-tree-userspace-wave-0-libraries-core-tiers
**Started:** 2026-11-05

## The gate stack (06-03 proven end-to-end)

Tier data files (`build/aarch64/tiers/tier{0,1}-*.txt`, D-13 append-only
allowlists) drive `subset-build.sh` (tier-0 fresh-archive lib builds with
T0-ART sha256 markers; tier-1 entry-form dispatch with T1-ART sha256
markers) and `subset-exec.sh` (stage h-prefix binaries + `tier1-sweep.rc`
+ `hello.c` into the donor tree → marker-guarded CONF=qemu bootdir patch
→ targeted rebake → qemu TCG boot → the staged termrc override SOURCES
the sweep → per-binary T1-OK/T1-FAIL serial markers + the D-01 float
canary + T1-DONE → host presence assertions, zero T1-FAIL, zero
panic|trap:).  mk exit status is never trusted anywhere in the flow.
Ledgers: `build/aarch64/markers/tier{0,1}.ledger` (D-14 append-only;
8-field fixed-order lines; last line per dir wins).

Venues: darwin (lima guest builds, host-side qemu) and native linux
(nsrun sudo-unshare; `--print-platform` dispatch proof).  Donor pin
350266104f1fed81c78c3d05fa44f523bec0ae4d verified host-side per run.

## 06-03 tracer run record

- **Commit:** 7123bce9f5956dfc5e3c92b81e46001c627f9638 (machinery state
  the gate ran; the 7l LP64 fix is 2bbca81ed, included)
- **Host:** darwin-aarch64 (lima guest builds + host qemu TCG)
- **Runs:** two consecutive green donor gates — the initial gate and the
  tracer-feedback re-verify (same retained log path; the retained log is
  the re-verify run)
- **Markers:** 19 × T1-OK (stacksmoke echo ls cat grep sed wc date test
  rc ps ipconfig factotum awk mkdir cp mv rm float-d01) + T1-DONE; zero
  T1-FAIL; zero panic|trap:; float canary `awk 'BEGIN{printf "%.3f\n",
  1.0/3.0}'` → `0.333` (D-01 execution-asserted)
- **Log:** [evidence/06-03-tracer-donor.log](evidence/06-03-tracer-donor.log)
- **The tracer's catch:** the first run faulted hawk (null read in
  abort()) — root-caused to 7l's LP64 double-constant pool emission
  (struct Ieee is 16 bytes on LP64 hosts; the 8-byte emit wrote l's low
  half + zeroes, never h) → fixed `2bbca81ed`, re-proven.

## Tier-0 carrier mapping (D-06 consumer-carried execution)

Per-lib carrier evidence: a swept tier-1 binary links the lib iff the
lib's header is included by the binary's sources.  Grep evidence
(`grep -lE '<lib\.h>'` over the swept sources, 2026-11-05):

| Lib | Carrier (swept tier-1 binary ← including sources) | Ledger exec |
|-----|---------------------------------------------------|-------------|
| libc | every swept binary (u.h/libc.h universal) | CARRIED:tier1-consumer |
| libbio | awk (awk/*.c), grep (grep/grep.h), ipconfig (ip/ipconfig/*), ls.c, ps.c, sed.c, wc.c | CARRIED:tier1-consumer |
| libregexp | sed.c, awk/re.c | CARRIED:tier1-consumer |
| libip | ipconfig (ipv6.c, main.c, ppp.c); also factotum/secstore.c | CARRIED:tier1-consumer |
| libndb | ipconfig (main.c, ppp.c) | CARRIED:tier1-consumer |
| libmp | factotum (dat.h); also paqfs | CARRIED:tier1-consumer |
| libsec | factotum (dat.h); also paqfs | CARRIED:tier1-consumer |
| libString | factotum (dat.h) | CARRIED:tier1-consumer |
| libthread | factotum (dat.h) | CARRIED:tier1-consumer |
| lib9p | factotum (dat.h) | CARRIED:tier1-consumer |
| libauth | factotum (dat.h); also paqfs/paqfs.c | CARRIED:tier1-consumer |
| libauthsrv | factotum (dat.h) | CARRIED:tier1-consumer |
| libflate | paqfs (mkpaqfs.c, paqfs.c) — paqfs's donor-kernel execution evidence is the 05-03 real-chain trace (BOOT:donor) | CARRIED:tier1-consumer |
| libavl | **none** — tree-wide, avl.h's only consumer is venti/copy.c (Phase 10) | EXCLUDED:no-tier1-consumer |

The swept binaries' donor execution evidence is the 06-03 gate log;
factotum's dat.h carries the libsec/libmp/libString/libthread/lib9p/
libauth/libauthsrv/libip includes (the whole factotum closure).

## Boot-chain classification (06-03 Task 2)

| Entry | exec | Evidence |
|-------|------|----------|
| stage/stacksmoke | BOOT:donor | 06-03 gate log — the staged termrc runs /boot/hstacksmoke (`T1-OK stacksmoke` + `stacksmoke: depth 64 ok`) |
| 9k/boot | BOOT:donor | 05-03 real-chain trace (`05-03-trace.txt`): our boot(8) ran its real flow on the donor kernel (opencons → connectembed → fversion → mount → execinit) |
| stage/init | BOOT:donor | 05-03 real-chain trace: our init ran `init -t → newns → termrc` on the donor kernel |
| cmd/paqfs:paqfs | BOOT:donor | 05-03 real-chain trace: our paqfs served the root over the 9P pipe on the donor kernel |

The 06-03 subset-exec venue boots the donor's OWN boot chain (the guest
is the donor kernel + donor init + served rootfs), so the 06-03 gate log
cannot evidence our boot(8)/init/paqfs — the 05-03 trace is their honest
donor-kernel execution evidence.  (USPACE-04 transparency: no execution
claim without serial-marker evidence.)

## 06-04 coverage sweep run record (USPACE-02, D-15)

- **Commit:** the run executed against base `f4ce2326872414a464ac9fffd22fc4dd5ed2e2c0`
  plus the 06-04 machinery working-tree edits (games checkall,
  coverage-sweep.sh); the machinery lands in the 06-04 Task-1 commit on
  top, and the committed report's authoring-run line names that base.
- **Host:** darwin-aarch64; build in the lima guest's linux-aarch64
  namespace (the report's venue line — the build host class is always
  linux-aarch64).
- **Sweep:** 212 visits / 212 exits / 0 watchdog fires; universe 221,
  visited 206, excluded 15 — reconciliation CLOSED, phantom-free, no
  stale exclusions.  58 per-dir `build=fail` rows recorded as Phase-10
  backlog data (almost all missing tier-0 libraries: libmemdraw,
  libhttpd, libplumb, libventi, … — 06-05's wave).
- **Second-chance pass (35 dirs):** plan9port mk recipes abort at the
  first failing command (measured 2026-11-05 with a toy mkfile: a
  container's `for(i in $DIRS) @{ cd $i; mk all }` stops at the first
  failing sibling — the strict abort-on-first-failure semantics 06-02
  documented).  With tier-0 libs outstanding, ip's loop stopped at httpd
  (libhttpd), upas's at fs (libplumb), postscript's at common, aux's
  inside its DIRS — 35 dirs were never reached.  The sweep's pass 2
  visits each remaining dir with its own guarded `mk all` (the log's
  SECOND-CHANCE lines), so reach is TOTAL and the adjacency truth holds:
  ipconfig + all 6 ip siblings are visited and recorded.
- **Wave-0 blocker re-verification:** jpg's committed ycbcr.h/rgbv.h can
  satisfy mk without the generator chain firing, so the run FORCES it in
  the scratch guest tree (rm headers + unix.* generators, `mk ycbcr.h
  rgbv.h`): unix.rgbycc and unix.rgbrgbv rebuilt via 9c/9l and asserted
  host-ELF (`BLOCKER-CHECK … magic=7f454c46` ×2) — the Phase-1 fix
  (7be7e98c8) holds for the aarch64 sweep.  mahjongg `visited=1
  build=ok` (fresh; citation 01-01-SUMMARY.md:156).  4a `visited=1
  build=ok` as an aarch64-target build here; target-sweep-excluded by
  decision (01-01-SUMMARY.md:150 + PITFALLS notorious-dirs).
- **Logs:** driver log `.build/coverage-sweep.log` (gitignored, full
  compile output); reach evidence
  [evidence/06-04-coverage-log.txt](evidence/06-04-coverage-log.txt)
  (committed: the VISIT/EXIT/SECOND-CHANCE log the report was parsed
  from).
- **Toolchain dialect findings (recorded for every later namespace
  script):** inside the namespace (and on the devshell host PATH) the
  Harvey hostbin shadows core tools — Plan 9 sed garbles on `|` inside a
  /-delimited pattern and silently fails `\(\)`/`\1` extractions; Plan 9
  date prints the epoch for `-u +%Y-%m-%d`.  coverage-sweep.sh uses awk
  index/substr or plain-`^` seds and a probe-verified ISO date
  (D-02-shadowing class; 07-04's dd incident was the same class).
- **Observed anomaly (run 1):** jpg's generator chain did not fire
  during that run's checkall (headers committed + fresh; mk's missing-
  prereq chase is proven to fire in three controlled reproductions) —
  the guest's `.build` was concurrently wiped/rebuilt by the Phase-07
  session mid-run, the probable state-churn cause; per the
  concurrent-session rule the probes were made state-independent (the
  forced blocker re-verification) and the run repeated.
- **Ledger lines appended:** none (06-04 owns committed reports, not
  tier markers).

## 06-04 generator audit + .7-magic scan run record (USPACE-03)

- **Commit:** the run executed against base `f4ce2326872414a464ac9fffd22fc4dd5ed2e2c0`
  plus the 06-04 machinery (the Task-1 + Task-2 commits land it).
- **Audit:** `build/aarch64/generator-audit.txt` — exactly 9 `G<n>
  site=` rows (G1-G9, re-confirmed against the live mkfiles at
  authoring) + the `not-swept:` section (linker self-checks, compiler
  t:/x: manual targets, test targets, ip/httpd trial, jtagfs rtest,
  cmd/mkfile compilers:V, 9k/mk/parse).  Two plan-text corrections from
  live-mkfile re-confirmation: G5's artifact is `o.pcode`
  (cpuobjtype=`o` via /unix/mkfile's O=o), and G6's genarch is
  **pcc-built native-only** (gs/mkfile:163-164's build guard) — the
  plan's "$CC-built BUT guarded" described the older shape; the guard
  proof stands (run guard :149, build guard :164).
- **Scan:** `sh build/aarch64/generator-scan.sh` — GREEN on the swept
  tree: G1-G5 host ELF (7f454c46), G7 `#!/b…` (23212f), G9 ed-script
  text (6564202d, NOTE), G6/G8 skipped (guarded), whitelist armed on
  exactly 7.genarch (not present — expected under the current gs
  guard), zero exec-format lines in the sweep log.
  [evidence/06-04-generator-scan.log](evidence/06-04-generator-scan.log)
- **Teeth proof:** planted the sweep-built `sys/src/cmd/va/7.out`
  (R_MAGIC 00008c47, od-verified) as `sys/src/cmd/jpg/7.rgbycc` in the
  guest tree → scan exit 1 with
  `RED: target-built generator in the tree: sys/src/cmd/jpg/7.rgbycc`
  + `GENERATOR-SCAN-FAIL`; plant removed; re-run green.
  [evidence/06-04-generator-scan-teeth.log](evidence/06-04-generator-scan-teeth.log)
- **darwin probing:** on darwin the scan checks the tree the sweep
  built (the lima guest's rsync copy) over ssh with `</dev/null` on
  every probe — ssh consumes the caller's stdin otherwise (measured:
  the rows loop processed only G1 before the fix); on linux the local
  tree is the build tree (namespace binds it).

## Format note for later plans (06-04..06-08 appends)

One section per gated run: commit SHA (the machinery state that ran),
host, marker summary, retained log path under `evidence/`, and the
ledger lines appended by the run.  Append-only; never edit earlier
records (supersede via the ledgers' last-line-wins rule and a new
section here).

## 06-05 tier-0 library wave record (USPACE-05, D-05..D-08)

- **Commits:** verdict `2d2fa5725` (libdynld rename kept — probe-built
  green in the namespace venue BEFORE the wave); wave machinery
  `fa19c15f7` (21 appends in the research dependency sort + the liblex
  archive-name mapping + the darwin lib rsync-back); ledger seed
  `90224e2e8` (22 × PENDING-CLASSIFY in exact tier-file order); libtest
  machinery + donor gate `9397b6823` (the commit the classification
  lines name).
- **Host:** darwin-aarch64 (lima guest linux-aarch64 namespace builds;
  host qemu TCG donor boots).
- **Sweep:** 35/35 `T0-ART` lines, `TIER-BUILD-OK`, zero per-lib compile
  fixes needed.  Member sanity: every object member across all 35 fresh
  archives is `.7` — zero committed `.6`/`.8` cargo-culted (D-07;
  `__.SYMDEF` symtabs aside); host archives byte-matched the guest
  gate-time sha256s at rsync-back.
  Log: [evidence/06-05-tier0-wave-build.log](evidence/06-05-tier0-wave-build.log)
- **libdynld verdict (D-08 recommendation (i)):** `dynld-arm64.c →
  dynld-aarch64.c` rename KEPT — probe-built before the wave
  (`rm -f /aarch64/lib/libdynld.a`, `objtype=aarch64 mk install` →
  fresh 3-member `.7`-only archive, `DYNLD-PROBE-OK`).  Compile note
  filed below (not a failure).
- **liblex machinery fix (not a compile failure):** liblex's mkfile
  builds `libl.a` (the lex runtime's historical name — consumers link it
  by that name), not `liblex.a`; the driver's lib<dir-minus-lib> name
  map aborted the first sweep run at liblex (`no artifact
  /aarch64/lib/liblex.a` after a green compile).  Fixed with an explicit
  per-lib archive-name case in subset-build.sh.
- **Execution classification (D-06/D-08) — all 22 final:**
  - `TEST:donor` (in-tree test ran green on the donor): **libdraw**
    (`T1-OK libtest-tdraw` — the additive `$O.test` rule this wave adds;
    test.c had no rule), **libdisk** (`T1-OK libtest-disktest` —
    opendisk's plain-file fallback on a /tmp fixture).
  - `TEST-FAIL:donor` (in-tree tests ran and FAULTED — filed Phase-10
    checklist below): **libmemdraw** (drawtest + arctest).
  - `PENDING:nm-tier1`: **libmach** — `nm.c:8` `#include <mach.h>`; nm
    joins tier-1 in 06-06 and supersedes this line to CARRIED:nm.
  - `EXCLUDED:no-consumer`: **libaml** (only self-consumers tree-wide;
    the 9k kernel never includes aml.h), **libdynld** (zero dynld.h
    includers outside libdynld — rename verdict recorded, artifact
    green).
  - `EXCLUDED:no-tier1-consumer` (16): libavl (avl.h's only tree-wide
    consumer is venti/copy.c), libbin (upas/bayes/dfa.c, venti/copy.c),
    libcomplete (acme/text.c; gs excluded), libgeometry, liblex (libl.a
    is the runtime for lex-GENERATED scanners; the lex binary itself
    self-contains — sub1.c defines allprint), libstdio (153 cmd
    includers, ape-flavored, Phase-10), libplumb (tier-3 plumber),
    libscribble (sole consumer libcontrol/scribble.c — a tier-3 lib
    chain, no tier-1 binary), libmemlayer (tier-3 GUI; arctest
    references no memlayer symbol), libframe (acme/sam), libcontrol
    (tier-3 GUI), libhtml (webfs/links), libhttpd (ip/httpd),
    libsunrpc (9nfs/nfs), libventi + liboventi (venti suite).
    Grep evidence: no swept or 06-06-planned tier-1 source includes any
    of these libs' headers, and no `#pragma lib "<archive>"` references
    them (swept-set + tree-wide greps, 2026-11-05).
- **Ledger lines appended:** 22 PENDING-CLASSIFY seeds (`90224e2e8`),
  superseded by the 22 final classification lines (the classification
  commit; last-line-per-dir-wins).

### Phase-10 per-dir checklist (the plan's no-drive-by rule — filed, not fixed)

- **libmemdraw (drawtest):** donor run faults — `suicide: sys: trap:
  fault read addr=0x0 pc=0x1ab50`; the pc symbolizes INSIDE `abort()`
  (0x1ab4c..0x1ab5b), i.e. the stochastic memimagedraw pixel
  verification detected a mismatch on aarch64 (the null read is abort's
  own mechanism).  Pixel-path LP64/struct-punning audit needed
  (Pitfall-3 class).
- **libmemdraw (arctest):** `fault write addr=0x1c001374bc
  pc=0x155bc`; the pc sits inside `memsetl` — the caller's destination
  pointer is garbage (LP64-fossil signature).  memarc → fill-path audit.
- **libdynld:** `dynld.c:128` — 7c: "useless or misleading comparison:
  ULONG < 0" (compile note; the lib archived green).
- **liboventi:** the `$O.vttest` + `test:` mkfile targets reference a
  `vttest.c` that does not exist — stale target; Phase 10 decides
  (remove the target or restore the source).
- **libmemdraw mkfile:** the `$O.test` target references a `test.c`
  that does not exist (same stale-target class; only drawtest/arctest
  are real).

### rc `~` dialect discovery (sweep-author trap, measured host-side 2026-11-05)

Quoting DISABLES wildcarding in `~` patterns: a quoted `*` is a literal
star (`~ foo 'f*'` never matches; wildcards must be bare — the sweep's
existing `*20[0-9][0-9]*` idiom).  Multi-star spans over literal text
are unreliable even bare (the three-group `* [0-9]* [0-9]* [0-9]* from *
shape never matched a valid '2048 64 32 from guess').  Assert multi-part
output in separate proven-shape conditions (disktest: `*' from '*` AND
`[0-9]*`).  Same D-02-shadowing-class family as 06-04's sed/date
findings.

### drive9 fast-fail interaction (standing-gate shape note)

A faulting binary's per-proc `suicide: sys: trap:` note trips drive9's
deliberate shared panic fast-fail (`panic|trap:` hardcoded) — observed
twice this plan (06-03's hawk fault was the first).  The standing sweep
therefore carries only green libtest members (tdraw, disktest); the two
faulting libmemdraw tests are evidence-captured
([evidence/06-05-libtest-donor.log](evidence/06-05-libtest-donor.log) —
all four markers + both fault lines + T1-DONE) and rejoin the sweep when
Phase 10 fixes the lib.  The guest completes the sweep regardless of the
host drive's fate (USPACE-04's never-wedge edge held: T1-DONE in the
fault run's log).  Green standing gate:
[evidence/06-05-tier1-gate-donor.log](evidence/06-05-tier1-gate-donor.log)
(21 × T1-OK, zero T1-FAIL, zero panic|trap:, T1-DONE, float 0.333).

## 06-06 tier-1 completion + self-host canary run record (USPACE-06, D-09/D-11)

- **Commits:** machinery `5edf99e83` (membership + sweep + fixed inputs)
  → gate fixes `30914b2b0`, `4e16a8055`, `4ceb04875`, `ce779749f`,
  `a9c8ff7c3`, `0ef2825d0`, `bbe2c4e82` → Task-1 ledger `f8b0b790e` →
  canary machinery `17370bba1` → canary fixes `cd96f5ac7`, `fab7377c4`
  (the gate-ran commit for the Task-2 ledger lines).
- **Host:** darwin-aarch64 (lima guest linux-aarch64 namespace builds;
  host qemu TCG donor boots).
- **Build:** [evidence/06-06-tier1-build.log](evidence/06-06-tier1-build.log)
  — 62 T1-ART + 35 T0-ART markers, `TIER-BUILD-OK`; **zero per-cmd
  compile fixes needed** across the 37 new entries (the Pitfall-3 classes
  again live at runtime, not compile time — and the donor gate measured
  none either).
- **Task-1 donor gate (full ~58-entry scale):**
  [evidence/06-06-tier1-donor.log](evidence/06-06-tier1-donor.log) —
  58 × T1-OK (55 computed members + float-d01 + libtest-tdraw +
  libtest-disktest), zero T1-FAIL, zero panic|trap:, T1-DONE.  The
  expected marker set is now COMPUTED from tier1-cmds.txt (never a second
  hardcoded list to rot).
- **A7 settled by observation:** `T1-BCOBS 0.333` — this bc prints the
  fraction WITH the leading zero (RESEARCH's "may print .333" resolved:
  it prints `0.333`).
- **D-11 canary (full path — A3 verified, NO fallback):**
  [evidence/06-06-selfhost.log](evidence/06-06-selfhost.log) — 61 ×
  T1-OK including `T1-OK 7a` (the committed tiny.s assembled in-guest;
  the canary's `7c -FTVw` emits .7 directly so 7a carries its own
  marker), `T1-OK selfhost-compile` (the guest's OWN h7c+h7l compiled
  and linked the committed hello.c in the writable /tmp scratch,
  resolving headers + autolib libc.a from the srv9-served root's staged
  /aarch64 tree), and `T1-OK selfhost` with
  `T1-SELFHOST-OBS hello aarch64 harvey` — the guest-compiled binary
  EXECUTED from the srv9 writable prefix and printed the known string
  (T-06-12's separate compile/run markers; the known-output string is in
  the retained serial log).  The A3 writable-prefix-exec mechanic works —
  the fallback (COMPILE-ONLY + host-side R_MAGIC assert) was NOT needed.
- **Venue findings filed for later plans (the gate's iterations caught
  them):** the donor kernel's devroot caps /boot at Nbootfiles=32 —
  subset-exec.sh now raises it to 128 in the donor-COPY tree (guest
  scratch, scripted, idempotent, disclosed harness-side accommodation);
  the served rootfs has NO /sys tree — the gate stages Harvey's
  sys/lib/yaccpar + sys/lib/lex/ncform + sys/include into it (yacc/lex
  open their skeletons by compiled-in absolute paths; 7c's include
  search covers /sys/include for libc.h); 9front guest-rc lexes a bare
  `=` as a word terminator (flag=value arguments must be quoted) and
  plan9port rc rejects separated subshells inside backquotes (the sweep
  is subshell-free; fn producers carry multi-command input streams);
  rc redirect-open failure is an uncatchable Xerror (07-04's lesson —
  look's db redirect is dir-probe-guarded); bc.y's execl("/bin/dc") is
  hardcoded and the donor's served /bin is minimal — the sweep
  union-mounts a shim (/tmp/bc-bin/dc ← our hdc) over /bin with the
  donor's own /bin/bind; the guest's /tmp is the MCREATE bind of
  /usr/glenda/tmp, so scratch cleanup mirrors BOTH branches.
- **Ledger lines appended:** 37 × `exec=T1-OK:donor` tier-1 lines
  (build chain + coreutils, evidence = 06-06-tier1-donor.log), the
  tier-0 libmach supersede to `exec=CARRIED:nm` (nm.c:8 <mach.h>; D-06),
  and the 4 × `exec=T1-OK:donor` supersede lines for 7a/7c/7l/cc
  (evidence = 06-06-selfhost.log).  No PENDING-CANARY survives as any
  dir's latest line.
- **dir-coverage (D-15 standing rule):** regenerated after the plan's
  commits — **unchanged** (the plan's members are single-file targets or
  already-ok dirs; no dir-level build rows flipped), `git diff
  --exit-code` green; no re-baseline commit needed.

## 06-07 measure-first tier-1 trace + r9 dispatch delta record (USPACE-04)

- **Commits:** trace rig `d542d727b` (tracesup.c + harness
  `--tier1`); the parser/venue fixes land beside this record; r9 delta
  `2a1f29b9355527aa07b72740fe51e619b9e76f77` (requaos/r9 main, PUSHED
  before the pin); pin bump `f8723ec51` (flake.lock ALONE;
  locked-rev == live-HEAD == origin/main verified at commit time).
- **Host:** darwin-aarch64 (lima guest for the in-guest 7c/7l compile +
  donor rebake; host qemu TCG for the traced boot).
- **The rig (the 05-03 rig with the tier-1 sweep as the traced
  payload):** the staged termrc override runs the sweep SOURCED
  (`venue=donor`) inside a spin-windowed, stop-armed subshell, and the
  NEW in-guest supervisor `build/aarch64/tracesup.c` step-traces it AND
  every born-traced descendant (donor sysproc.c:155 procctl
  inheritance).  The 05-03 rc traceloop/fragprint shape moved into one C
  program: no per-fragment forks (the rc cat|sed pair per fragment would
  dominate at tier scale), one newline-framed fragment file per pid on
  the srv9 writable prefix (`/tmp/t1trace`, read host-side off the served
  root — no serial flood, no mid-line gluing, so no __GSD_FRAG_ tags).
  Root armed write-first (the Proc_stopme stop has no pending fragment);
  born-traced children armed read-first (pidwait.c's traceloop1 rule).
  Venue = subset-exec.sh's 06-06 donor gate exactly (the donor's own
  chain, writable prefixes, srvroot /sys+aarch64 staging, devroot
  Nbootfiles=128, self-healing CONF block) with the trace added.
- **The run:** boot → traced sweep → `T1-DONE` + `__T1_TRACE_DONE__` →
  init's restart → shell proof.  Zero T1-FAIL, zero panic|trap:.  201
  traced pids; **8581 entry fragments; 0 malformed**.  Artifact:
  [evidence/06-07-trace.txt](evidence/06-07-trace.txt) (per-binary
  multisets + classification + the delta section).  Serial log:
  `.build/9front-dist/tier1-trace-run.log` (gitignored transient).
- **The measured set (r9-runnable union, 19 names):** chdir, close,
  dup, exec, exits, open, rfork, pipe, create, fd2path, brk_, notify,
  rendezvous, seek, stat, await, pread, pwrite, nsec.  **The delta vs
  r9's dispatch = exactly {RENDEZVOUS(34) ×1 (hfactotum's libthread
  start), NSEC(53) ×2 (hdate, hipconfig — libc time() = nsec()/1e9)}** —
  expected-small confirmed by measurement, never assumed.  The
  write-flow union (donor-only; r9 T1-SKIPs it) is the artifact's second
  table: remove ×8, wstat ×3, fwstat ×1 fired there — **the measured
  evidence RATIFIES r9's loud-None for REMOVE/WSTAT/FWSTAT this era**
  (their only callers are write-flow binaries r9 cannot serve), and
  OSEEK never fired anywhere.
- **The ed finding (measured, reclassified):** the trace caught hed's
  unconditional `create("/tmp/eXXXXX")` + exit-time `remove`
  (ed.c:159/1092/930) — a /tmp-writable dependency the read-only block
  hid.  ed MOVED into the sweep's donor-venue guard with a named
  `T1-SKIP ed` on r9 (USPACE-04 transparency: never an unearned T1-OK);
  the SKIP set's "known list" gains ed by measured evidence.
- **r9-side implementation (exactly the delta):** NSEC(53) — the
  epoch-ns clock composed from qemu virt's PL031 RTC (DTB
  pl031@9010000, RTCDR epoch seconds) anchored once against the generic
  timer's monotonic CNTPCT count (runtime CNTFRQ; u128 scaling; raspi
  gets no fake clock — compile-time absence); RENDEZVOUS(34) — 9k
  sysrendezvous semantics over a new `port::rendez` group (Arc-shared
  per rfork's RFREND arm, now real; first-sleeps/peer-wakes symmetric
  exchange; exit sweeps the sleeper table per 9k pexit).  Both handlers
  follow the Phase-4/5 discipline (sargs validation, width rules,
  loud errors); the loud-unimplemented default stays for everything
  else.
- **r9 gates (all green with R9_HARVEY_DIR set):** `cargo xtask
  ureg-check` (35 fields ≡ aarch64/include/ureg.h); all five boot-test
  profiles — virt (banner/ticks/echo), virtexec (SC#2 + stacksmoke),
  virtmulti (SC#4 rfork/fd + preemption), virtnotes (notes), virtpanic
  (dump) — and `cargo test -p port` (83 green incl. the 2 new rendez
  table tests).
- **Push-before-pin (T-06-13):** r9 `2afa3d6..2a1f29b` pushed to
  requaos/r9 main BEFORE the Harvey flake.lock advance; the pin bump
  (`f8723ec51`) touches ONLY flake.lock and names both SHAs.
- **NMOUNT-ledger note (step 6):** no NMOUNT-class (number/semantics)
  divergence surfaced.  The trace's two name-level mappings are
  print-name only, same wire: `_nsec` ↔ NSEC(53) (already documented —
  aarch64/ABI.md:59) and the donor mksystab's printed `Rendez` ↔
  RENDEZVOUS(34) (mksystab's baked `Rendezvous→Rendez` rename; the
  parser's `resolve()` maps both).  Nothing to grow.
- **Venue notes for later plans:** the traced sweep's DRIVER shell was
  the donor's own rc (the venue boots the donor's chain; `exec rc`
  resolves the donor's /bin/rc) — the "rc" bucket in the artifact
  aggregates the driver + its pre-exec fork/backquote shims, and bare
  `echo` in sweep pipelines resolves to the donor's /bin/echo (measured:
  `echo` ×69).  Footprint classes are identical to the Harvey binaries'
  (open/pread/close/pwrite/brk_/exits); on r9 the driver is the image's
  /bin/rc (Harvey's).  The trace ran the pre-prefix sweep; the
  venue-prefix edit (B=/boot→/bin, D=/boot→/lib/tier1 on r9) and the ed
  move are venue-mapping only — no measured behavior changed.
- **Ledger lines appended:** none by this record (the r9-venue ledger
  lines — `exec=T1-OK:r9` / `exec=SKIP:r9-no-writable-fs` — land with
  the 06-07 Task-2 gate evidence; PENDING-CHAIN:06-07 resolves there).

## 06-07 Task 2 — the r9 venue record (D-12, USPACE-04's second venue)

- **Commits:** gate machinery `79e09967a` (mkimage payload +
  tier1-r9.expect + nix/tier1-r9.sh + flake.nix + tier1-sweep.rc venue
  prefixes); rc fix `cebb127ae` (Xbackq — see below); r9 substrate
  `2a1f29b` (NSEC + RENDEZVOUS), `76e54ce` (MAX_PROCS 64→512), `c82c8a3`
  + `46a2ed2` (devmnt contention wait + exit-sweep heap payloads;
  sys_seek SEEK_END + stream rejection) — all pushed to requaos/r9 main
  BEFORE the pins; pins `f8723ec51` → `dc0699339` → `5c261786a` (each
  its OWN flake.lock-only commit; the final pin names 46a2ed2 =
  origin/main).
- **Host:** darwin-aarch64 (lima guest builds; host qemu — the gate's
  default darwin leg is gicv3+hvf).
- **The gate (`nix run .#boot-r9-tier1`)**: GREEN —
  [evidence/06-07-tier1-r9.log](evidence/06-07-tier1-r9.log): 37 T1-OK +
  23 T1-SKIP + T1-DONE, zero T1-FAIL, zero panic/Unhandled-interrupt,
  `T1-OK float-d01` on r9 (D-01's gate holds on BOTH kernels).  The
  driver typed ONE line at the % prompt (`rc /lib/tier1-sweep.rc r9` —
  r9's devcons serves reads, the 05-03 finding; the donor's
  sourced-sweep accommodation does not carry).
- **Marker parity donor↔r9 (USPACE-04):** the donor re-verify of the
  edited sweep (06-07's venue-prefix + guard edits):
  [evidence/06-07-tier1-donor-reverify.log](evidence/06-07-tier1-donor-reverify.log)
  — 61 T1-OK + 0 SKIP (the donor runs everything).  Extracted sets:
  EVERY r9-runnable T1-OK name matches a donor T1-OK (parity), and the
  r9 SKIP set equals exactly the donor-guarded list (23 names) — the
  donor emits zero SKIPs.
- **The measured r9-era reclassifications (USPACE-04 transparency — a
  SKIP is declared, never silent; never an unearned T1-OK):**
  - **ed** → donor-guard (the 06-07 trace caught its mktemp
    create+remove, ed.c:159/930/1092 — /tmp-writable, hidden by the
    read-only block);
  - **ipconfig** → donor-guard (opens /dev/random at startup; r9 has no
    devrandom this era — a PRNG masquerading as /dev/random is threat
    surface, never a shim);
  - **factotum** → donor-guard (startup needs FAUTH + the devproc
    protect/noswap ctl verbs + a devsrv-service mount — r9's sys_mount
    is pipe-9P-only this era, and the image's auth stub-by-absence
    already means no factotum on r9).
  The driver's expected SKIP set is EXTRACTED verbatim from the sweep
  file (never a second hardcoded list), so the gate asserts equality
  against the living source.
- **r9-side substrate the gate measured and grew (all in the pushed
  pins):** the measured dispatch delta {NSEC(53), RENDEZVOUS(34)} (Task
  1) PLUS the gate-found correctness work the tier-scale sweep needed:
  - **devmnt RPC contention**: the one-outstanding-RPC rule (05-07)
    erred loudly on concurrency; the sweep's pipelines exec two children
    through the shared mount session CONCURRENTLY — the first measured
    concurrent caller.  Contention now blocks-and-retries at ~1-tick
    granularity (the injected hook + proc's mnt_contention_wait;
    sys_sleep's shape — never a spin, never a lost wake).
  - **exit-sweep heap payloads**: the proc-table slots are never reaped
    this era, so husk payloads (a fresh ~6 KB fd table per exit + argv +
    queues) rode forever — ~200 fork+exec+exit cycles exhausted the
    4 MiB heap arena (oom panic).  exit_tail now drops the payloads +
    shares ONE empty fd group per husk; the sweep completes at the
    ORIGINAL arena size.
  - **MAX_PROCS 64 → 512**: the sweep's measured 201 born-traced forks
    (the donor trace) walled the 64-slot boot-session fork budget.
  - **sys_seek SEEK_END + the stream-device rejection**: tail -1's
    length probe + join.c:148's random-access check need SEEK_END (the
    chan's stat length, the donor's case-2 shape); a pipe end is never
    seekable (the donor sseek's dc=='|' → Eisstream — tail.c's
    isseekable probe keys on it).
  - **exec MAX_ARGS 16 → 32** (exec.rs's seed-page pre-cap, matching
    sys_exec's MAX_EXEC_ARGS): the sweep's `echo T1-XDOBS $xdout` builds
    17 argv entries — 16 was too small.
- **The rc finding (Harvey in-tree, pre-existing, exposed by the r9
  venue running the sweep under Harvey's OWN rc — the donor gate runs
  9front's):** rc's Xbackq parent arm poplist()ed the split word's list
  without a guaranteed list below; at a top-level backquote assignment
  the stack drained to argv==0 and the next op crashed ("pushword but
  no argv!").  ~50% of sweep runs ate 1-2 markers pre-fix; ~15% killed
  the sweep's shell.  Fixed `cebb127ae` (restore the invariant: Xbackq
  leaves a list on the stack) — verified 10/10 consecutive T1-DONE runs
  + the gate's clean capture (zero rc crash prints).  The SAME latent
  shape is in plan9port's rc unchanged (9front rewrote the path); the
  residual class (any OTHER op that could drain the stack) is filed in
  deferred-items as a Phase-10 rc-audit item.
- **Ledger lines appended (D-14):** 59 r9-venue lines — 36 ×
  `exec=T1-OK:r9`, 21 × `exec=SKIP:r9-no-writable-fs`, 1 ×
  `SKIP:r9-no-devrandom` (ipconfig), 1 × `SKIP:r9-no-auth-era`
  (factotum) — evidence = the retained r9 gate log, commit = the
  gate-ran machinery (`cebb127ae`).  8-field lint green.
- **Regression:** `nix run .#boot-r9-smoke` GREEN over the tier-1-laden
  image (the v1.0 gate, byte-untouched expect).

## 06-08 D-16 CI workflow + local validation record (Task 1)

- **Commit:** authored at base `3003b050c` (the workflow file + this
  record land in the 06-08 Task-1 commits; validation ran on the
  working tree at that base).
- **Workflow:** `.github/workflows/phase6.yml` — the phase-6
  full-pipeline CI (D-16).  `name: phase6-userspace`;
  `on: [push, pull_request]`; `permissions: contents: read`;
  `concurrency: { group: phase6-${{ github.ref }}, cancel-in-progress:
  true }` (T-06-18).  Three jobs: `wave0-host` (fast, 45m — host
  toolchain, gethunk ×2 archive+verify repro gates, coverage regen +
  `git diff --exit-code`, generator .7-magic scan, grep-hygiene gate,
  FPdbleword assert), `tier-donor` (slow, 90m — subset-build.sh +
  subset-exec.sh on the native-linux venue, donor pin 350266104f
  host-side, TCG aarch64), `tier-r9` (slow, 90m — `nix run
  .#boot-r9-tier1`).  r9-shell.yml + harvey.yml byte-untouched.
- **Supply chain (T-06-16):** EVERY action pinned by 40-char SHA —
  actions/checkout `11d5960a326750d5838078e36cf38b85af677262` (v4) and
  actions/upload-artifact `ea165f8d65b6e75b540449e92b4886f43607fa02`
  (v4), both resolved via the GitHub API `git/refs/tags/<tag>` at
  authoring time (2026-11-05); DeterminateSystems/nix-installer-action
  `3138316df39ed29be04236d7ffc686fa525866aa` (v23, the 05-13 pin).
  Pin gate: PINS-OK 8/8.
- **Hosted run: PENDING FIRST PUSH** (the 05-13 substitution
  precedent — harvey master is never pushed from this session): the
  workflow is authored + locally validated; the first hosted run
  happens at first push.  Watch: `gh run list --workflow
  phase6.yml` / the repo's Actions tab.  Nothing fabricated.
- **Local validation matrix (2026-11-05, darwin-aarch64 host, inside
  `nix develop`):**

  | wave0-host step | Venue | Result |
  |---|---|---|
  | YAML parse + job assertion | host python3 (PyYAML 6.0.3) | JOBS-OK |
  | SHA-pin grep gate | host | PINS-OK 8/8 |
  | gethunk-repro-cc archive+verify | host | TEETH-OK 6c/7c (gethunk ENOMEM terminus from PREREF df4b1cb4…), GREEN 6c/7c — 4/4 TAP |
  | gethunk-repro-6l archive+verify | host | TEETH-OK 6l, GREEN 6l (PREREF 2e3b3702…) — 2/2 TAP |
  | coverage regen + `git diff --exit-code` | lima guest (linux-aarch64 namespace, darwin leg) | COVERAGE-SWEEP-OK (206 visited / 15 excluded / universe 221; jpg BLOCKER-CHECK host-ELF ×2); report byte-identical — COVERAGE-DIFF-CLEAN |
  | generator-scan | host probes the freshly swept guest tree | GENERATOR-SCAN-OK (G1-G5 host ELF, G7 `#!`, G9 NOTE, G6/G8 guarded-skip, whitelist armed on 7.genarch — absent as expected) |
  | grep-hygiene gate | host | GREP-HYGIENE-OK (no absolute-path grep CALL shape in build/aarch64/*.sh) |
  | FPdbleword region assert | host | FPDBL-OK (exactly the uint lo/hi members; no ulong in the union) |
  | tier-donor / tier-r9 job commands | not re-run by this plan | the encoded pipeline is exactly the 06-06 donor gate + 06-07 r9 gate machinery, both green locally in those plans (their records above); the hosted legs run them on ubuntu-latest |
  | x86_64-poolpanic job | — | Task 2's record below |

- **Retained logs:**
  [evidence/06-08-gethunk-verify.log](evidence/06-08-gethunk-verify.log)
  (both archive-mode teeth proofs + fixed-pair greens),
  [evidence/06-08-generator-scan.log](evidence/06-08-generator-scan.log),
  [evidence/06-08-coverage-log.txt](evidence/06-08-coverage-log.txt)
  (the regen's VISIT/EXIT/SECOND-CHANCE reach log; the compile flood
  stays gitignored per convention).
- **Ledger lines appended:** none (this plan owns CI + verdict
  evidence, not tier markers).
- **Host-class watch item (honest):** the committed coverage report's
  venue line claims host-independence from a linux-aarch64 build host;
  the hosted wave0-host runner is linux-x86_64.  If any per-dir build
  row proves host-dependent, the D-15 diff reds on first push — the
  gate working as designed; the report header's re-baseline standing
  rule applies.

## 06-08 Task 2 — the USPACE-10 pool-panic re-test record (D-04)

- **Commits:** verdict harness `30d9d5cd2` + the healthy-marker
  correction `cbdee1f87` (Rule-1 deviation — see below) + the
  x86_64-poolpanic CI job `c8f542246`.
- **Harness:** `build/mm/boot-9k10cpu-qemu.sh` — plain qemu `-kernel`
  (minimega-free, D-03), capture truncated before launch, accel probed
  exactly once per run (kvm iff /dev/kvm writable else tcg/-cpu qemu64,
  600s budgets, PP_BOOT_BUDGET/PP_CONTINUE_BUDGET overrides), whole-log
  classification against the recorded 01-04 signature set (`bad magic`,
  `panic`, `suicide: sys: trap`), verdict file grammar
  `VERDICT=PASS|FAIL <classification>` + banner/signatures/
  healthy_markers/accel/kernel/log/commit/date.  Exit 0 iff a verdict
  was PRODUCED (banner seen + classification written) — never gated on
  PASS; empty capture → FAIL no-banner + non-zero (the leg never greens
  on an empty log).
- **THE VERDICT (local, darwin-aarch64, qemu 11.1.2 TCG, kernel
  artifact amd64/9k10cpu.elf built from this pswt-fixed tree):**
  **`VERDICT=PASS no-panic-signature boot-continues`** — banner at 5s,
  the boot sailed PAST the 01-04 panic point ("508M memory…" → boot.c's
  `root is from (tcp)[tcp]: ` method prompt, boot.c:342), ZERO
  signature lines in the whole capture.  The Phase-2 pswt.c LP64 fix
  (`7efff3aa3`) cured the 9k-amd64 bad-magic pool panic — the same root
  cause as the donor kernel's, as the Phase-2 audit hypothesized.
  USPACE-10 is closed WITH INFORMATION.  Full record:
  [evidence/poolpanic-verdict.md](evidence/poolpanic-verdict.md);
  retained serial capture:
  [evidence/06-08-poolpanic-serial.log](evidence/06-08-poolpanic-serial.log).
- **Rule-1 deviation (marker vocabulary):** the plan-named healthy
  markers (boot.c's `argc=` / `version...`) are unreachable in the
  production build — `argc=` is `#ifdef DEBUG`-gated (boot.c:68-73) and
  `version...` prints only after a rootfs connection (boot.c:139), which
  a capture-only serial never establishes.  The first local run
  classified the healthy boot `banner-only` (healthy_markers=no).
  Corrected to the production build's actual post-panic print
  (`root is from`, with the plan's two retained as recognized
  alternatives) and re-run fresh — same boot, honest
  `boot-continues` classification.  Run-1's verdict is retained in the
  evidence record (superseded, never silently rewritten).
- **Teeth runs (all recorded in the evidence file):** incident-shaped
  canned log → `VERDICT=FAIL pool-panic-signature` exit 0
  (production-not-outcome); empty log → `FAIL no-banner` exit 1;
  healthy canned log → `PASS boot-continues` exit 0; a deliberately
  wrong kernel image through REAL qemu → `FAIL no-banner` exit 1
  (qemu `invalid kernel header` at 5s); the production-shape healthy
  log (root-is-from only) → `PASS boot-continues` exit 0.
- **CI leg:** the `x86_64-poolpanic` job (phase6.yml) builds 9k10cpu
  from the clean checkout via `nix run .#build` (the pswt-fixed 6c
  guaranteed independent of local hostbin), runs the harness, asserts
  the verdict FILE + VERDICT= line, and uploads the serial log +
  verdict as artifacts (`if: always()`).  **Hosted run: PENDING FIRST
  PUSH** — the KVM-classified verdict lands at first push; the local
  TCG verdict stands as the recorded evidence meanwhile (D-04's
  recorded-either-way letter satisfied).

## 06-09 Harvey-kernel tier venue record (G-06-1, USPACE-04/USPACE-06)

- **Venue design:** a new committed 9k kernel config
  `sys/src/9k/k10/k10t1` — k10cpu's device/dbgflg/mach/port/dir/lib
  sections VERBATIM (the poolpanic-proven set; the header comment bars
  pruning) — with only the `rootdir`/`boot` sections swapped for the
  venue: 43 files embedded via the 9k mkrootall machinery (the 35-strong
  amd64 tier-1 read-only set under their h* names + rc/echo/bind under
  plain names for the boot chain + rc/lib/rcmain + the boot script + the
  sweep/sentinel/nm-input staged files).  The embedded /boot/boot is
  `sys/src/9k/k10/root/k10t1.rc`, an rc script (the k10root
  standalone-boot shape — initcode.c:26-29's "could be a shell script"
  contract): it sets `path=(/boot /bin .)` first, binds `#p`→/proc,
  `#d`→/fd, `/boot`→/bin, sets the kernel clock from `#r/rtc` and the
  hostowner to `sys`, runs the sweep in a CHILD `rc -m /boot/rcmain`
  (rc sources an rcmain on EVERY invocation — exec.c:130-156's loadboot
  — and devroot has no /rc/lib/rcmain; the -m is load-bearing on both
  the shebang and the child), then blocks init forever
  (k10root.rc:114's idiom).  `sys/src/9k/port/devroot.c`'s Nbootfiles is
  raised 32→64 (additive only).  The boot harness
  `build/mm/boot-9k10t1-sweep.sh` is the poolpanic harness's shape
  (D-03 plain qemu — no minimega/network/srv9/9front anywhere in the
  venue; capture-before-start; exactly-once accel probe; deadline polls
  for banner then T1-DONE with the signature early-break set) with the
  tier marker contract: the expected set COMPUTED from
  tiers/tier1-cmds.txt (subset-exec.sh:803-814's awk+mapping; the six
  driver-only exclusions kept; stacksmoke excluded — r9-era, never
  staged on this venue; float-d01 plus the four venue-mandated SKIP
  markers required), boundary-anchored `T1-OK m([^A-Za-z0-9.-]|$)`
  matching (WR-06's fix — the mk<mkdir / cp<cpp substring false-green
  class), a fixed 24-name SKIP whitelist with the converse assert (a new
  SKIP name reds the gate), zero T1-FAIL / case-insensitive
  `panic|trap:` lines, T1-DONE required.  The build driver
  `build/mm/build-9k10t1.sh` runs the nuke sandwich
  (build/scripts/build:50-52's shape) in the linux private namespace:
  `mk nuke && mk CONF=k10cpu 9k10cpu && mk CONF=k10t1 9k10t1 && mk
  nuke` — the k10cpu leg is the devroot-raise's compile re-proof of the
  sacred amd64 kernel — with the embedded-count assert (43 addbootfile
  lines in the generated k10t1.c, counted in-namespace before the
  trailing nuke) and a git-clean gate on sys/src/9k (systab.c +
  k10t1.c + .tier1-stage ride .gitignore).
- **Local run record (the venue's green proof):** commit `36cff6c52`
  (the harness's commit field), lima guest (linux-aarch64),
  qemu-system-x86_64 11.1.2 TCG, banner at 5s, T1-DONE at +0s,
  **`T1-VENUE-PASS ok=36 skip=22+2libtest fail=0 done=yes`**.  Kernel
  sizes from that build: 9k10cpu.elf 2,022,154 B (the devroot-raise
  re-proof), 9k10t1.elf 4,225,158 B (the 43-file embed is visible,
  ~2.1×).  Retained serial log:
  [evidence/06-09-tier-9k-local.log](evidence/06-09-tier-9k-local.log).
- **Teeth proof (T1_CLASSIFY_ONLY, the gate's ability to red):** (a) the
  retained log → rc=0 green; (b) a copy with the `T1-OK awk` line
  deleted → rc=1 naming `missing markers: awk`; (c) a copy with `T1-OK
  awk` flipped to `T1-FAIL awk` → rc=1 (`T1-FAIL lines: 1`).
  Teeth-procedure note: the serial capture has CRLF terminators —
  doctor CR-aware and diff-check that the doctoring took before
  classifying (my first pass's `$`-anchored delete silently no-op'd;
  the harness correctly passed the unchanged log).
- **Findings the venue surfaced (each fixed in its own commit, re-run to
  green):**
  1. `778d4616b` — **6l LP64 double-constant pool emission** (06-REVIEW
     WR-01's named suspect class on amd64): awk's `%.3f` of 1/3 aborted
     `division by zero`, seq died `zero increment` — both ride strtod's
     double constants, which 6l's datblk emitted with hi=0 on LP64 hosts
     (struct ieee {long l; long h;} is 16 bytes there; the 8-byte emit
     wrote l's low half + l's zero high half).  Fixed as 7l's 06-03 fix
     (`2bbca81ed`): emit the two 32-bit halves via `int[2]`.  Byte-level
     proof: a test link carries the correct 1/3, 1.0, 2^32 constants;
     the venue then printed T1-OK awk + float-d01 + seq.  6l was 06-03's
     deferred "no current consumer in the amd64 build" — the venue is
     the consumer.
  2. `bba1eeaa8` — **the k10 early KZERO map covered only 0-4MiB**;
     the 43-file embed pushes the kernel image's data+BSS to 5.16MiB
     physical, and main()'s `memset(edata, 0, end-edata)` faulted at the
     first BSS page before the banner (qemu -d int: v=0e write at
     CR2=0xfffffffff04e13d8 → double/triple fault → qemu -no-reboot
     exit).  Extended to 8MiB (two more free consecutive PD slots;
     identity map + its teardown untouched; mmuinit still builds the
     real map later).  k10cpu runtime re-proof with the change:
     VERDICT=PASS no-panic-signature boot-continues (local TCG); the
     x86_64-poolpanic CI leg re-gates it on every push.
  3. `36542bdbb` — **parse keys each rootdir entry's C symbol name off
     the SOURCE path**, so the plan's dual-name embeds (hrc+rc,
     hecho+echo) collided (`_amd64_bin_rc{code,len}` 'multiple
     initialization' at 6l).  The second name of each pair rides a
     staged copy (.tier1-stage/rc, .tier1-stage/echo); the 43-entry
     count and the /boot vocabulary are unchanged.  Same commit: the
     namespace script's PATH needed the phase-01 `/rc/bin:.` tail
     (bootmkfile's archive rule execs `membername` from rc/bin —
     libboot.a6 otherwise builds EMPTY and bootk10cpu.out fails to
     link).
  4. `8b55abbda` + `8088471ba` — **venue plumbing the rc-script boot
     skips** (boot.c's chain normally does both): the clock (date read
     epoch 0 — `hcat '#r/rtc' > /dev/time`; devrtc registers under
     letter 'r' and nothing binds it into /dev on this venue, so the
     first attempt's `/dev/rtc` path did not exist) and the hostowner
     (init0 sets eve="" so every /proc status line carried an EMPTY user
     field — 'boot Await 20 10 100 0 0 0 244 13 13', 11 tokens not 12 —
     and ps(1)'s tokenize then silently returns per process; writing
     #c/hostowner sets eve AND the process user, sysauth.c:148's
     hostownerwrite — k10root.rc:28's own idiom).
  5. `fd544ef42` — **the sweep's 9k libtest SKIP pair is emitted via a
     block-local helper**: nix/tier1-r9.sh and ledger-r9-append.sh
     compute the r9 SKIP set by grepping the sweep's TEXT for
     `echo T1-SKIP <name>` lines (the file is the source of truth), and
     the two new literal lines grew r9's want-set by a marker r9 RUNS
     (tdraw) — the local parity run redded 'T1-SKIP set mismatch'.  The
     file's literal SKIP lines are again exactly r9's set (23 names);
     the 9k runtime emission is unchanged (24 lines).
- **Ledger decision (T-06-20):** NO tier-ledger lines are written for
  the amd64 venue.  The D-14 ledgers' sha256 fields pin the aarch64 tier
  artifacts and their `:donor`/`:r9` exec venues; amd64 CI regression
  evidence is not tier-graduation evidence, so D-14's grammar is honored
  by not writing to it.
- **Iteration-trap note (harness-level, not a plan deviation):**
  plan9port mk treats a missing INTERMEDIATE prerequisite as
  ancient-but-tolerated when the INSTALL target exists — deleting
  in-tree `6.<name>` binaries without deleting `/amd64/bin/<name>` makes
  `mk <name>.install` report 'up to date' and skip the relink.  The
  venue's binaries must be refreshed with BOTH deleted (or by the full
  clean-checkout build — the CI path in 06-10).  Also: subset-build.sh's
  darwin leg (vm-run.sh:65) rsyncs the host checkout INTO the guest on
  every run — a host-side r9 gate run reverts guest-local amd64/bin
  relinks; run the venues sequentially.
- **r9 venue parity:** GREEN locally — `nix run .#boot-r9-tier1` rc=0,
  'ALL STAGES PASS — boot, one typed sweep line, float canary on r9,
  marker parity + exact SKIP set, T1-DONE', with 'T1-SKIP set equals the
  known list (23 names)' (the fn-helper keeps the r9 extraction
  venue-correct).  PLUS an execution-trace byte-parity proof: `rc -x`
  traces of the 06-07 and 06-09 sweeps with venue=r9 are byte-identical
  modulo the sweep filename and pids (234 lines each) — the 06-09 arms
  are inert on r9.  Flake-storm disclosure: seven earlier runs this
  session redded at six DISTINCT random sites (tr / ipconfig-SKIP /
  echo / cal / sleep missing markers; float-canary stage deaths ×2) —
  all on a host under parallel TCG load from the 06-09 venue work; one
  (run 2's SKIP-set mismatch) was MY extraction bug, found and fixed
  (`fd544ef42`).  The quiet run greens on demand; the flake class is
  the recorded rc argv-stack residual (deferred-items, sharpened by
  these observations).
- **Hosted-run expectation:** the tier-9k CI job lands in 06-10; its
  first hosted run is PENDING FIRST PUSH per the 05-13/06-08 precedent —
  nothing fabricated.  The local TCG run above is the recorded evidence
  meanwhile.

## 06-10 tier-9k CI wiring + tier-donor retirement record (G-06-1, CR-01)

- **Commits:** `a6ef4dbba` (phase6.yml surgery: tier-9k job in,
  tier-donor leg out, venue-pin step, header rewrite),
  `0d7c1f537` (coverage venue wording + regen), and the records commit
  carrying this section + the CR-01 disposition + the subset-exec
  header venue-split note.
- **The retirement decision (maintainer direction 2026-11-05, quoted
  from 06-UAT.md's gap record):** "9front is the donor ONLY because
  Harvey lacked local arm64/aarch64 support; on amd64/x86_64 CI the
  donor evidence should come from HARVEY's OWN kernel (9k10cpu — proven
  booting on the runner by the poolpanic leg), not 9front.  Deliberate
  direction: focus on harvey + r9, avoid borrowing 9front-specific
  design decisions that deviate from Harvey's faithful Plan 9 4e
  implementation."
- **Rejected alternatives:** (a) CR-01's provisioning arm — wiring a
  native 9front donor clone/build into the CI job: rejected by the
  direction itself (9front on x86_64 CI is the anti-goal; the leg also
  redded EARLIER than the pin gate on the first hosted run — the host
  toolchain bootstrap died under nixpkgs gcc hardening flags, so the
  arm's real cost is a second toolchain-bootstrap surface to maintain
  for evidence the direction does not want); (b) keep the leg RED as a
  tripwire: rejected — a permanently-red leg trains red-blindness, and
  the wave0-host venue-pin step is the honest tripwire instead (a
  silent reintroduction reds the fast leg by grep, not by a 90-minute
  doomed boot).
- **What landed in phase6.yml:** the `tier-9k` job (ubuntu-latest,
  90m) — checkout + nix + setup/toolchain provisioning with the
  TOOLCHAIN-PRODUCTS-OK assert loop (the clean-runner class the first
  hosted run redded on), the full amd64 build tee'd to
  .build/tier-9k-build.log with the BUILD-SWALLOWED-FAIL artifact
  assert + digest (02-12 exit-0 class), `build/mm/build-9k10t1.sh`
  (self-asserting; tail to the step summary on failure),
  `build/mm/boot-9k10t1-sweep.sh` (marker-state digest + log tail
  captured to the step summary on failure; the result file cat'd on
  green), and the venue logs uploaded `if: always()` (tier-9k-logs:
  serial + result + build log).  The `tier-donor` job is REMOVED.
  wave0-host gained the venue-pin step after grep-hygiene: anchored
  negative-grep on the retired job key, a self-hosting-safe ERE
  negative-grep on the donor-gate script basenames (the pattern text
  contains parentheses + a backslash — it never matches itself), and a
  positive-grep requiring the tier-9k key.  wave0-host / tier-r9 /
  x86_64-poolpanic job bodies are byte-preserved (the only wave0-host
  change is the additive pin step); every action pin, the concurrency
  group, permissions, and triggers are untouched.
- **Local validation matrix (2026-11-05, darwin-aarch64 host, inside
  `nix develop`):**

  | Check | Result |
  |---|---|
  | YAML parse + exact job set | JOBS-OK {tier-9k, tier-r9, wave0-host, x86_64-poolpanic} |
  | SHA-pin regex on every `uses:` | PINS-OK 11/11 (40-char) |
  | Venue-pin replays host-side | retired-key anchored grep: 0; donor-script ERE: 0; tier-9k anchored grep: 1 |
  | Referenced-path existence enumeration | every repo-relative path the run blocks name exists (gethunk ×2, coverage-sweep.sh, generator-scan.sh, dir-coverage.txt, tier1-r9.log's producer path, tiers/tier1-cmds.txt, build/mm/build-9k10t1.sh, build/mm/boot-9k10t1-sweep.sh, build/mm/boot-9k10cpu-qemu.sh, aarch64/include/u.h) |
  | Byte-preservation diff review | tier-r9 BYTE-PRESERVED (679B), x86_64-poolpanic BYTE-PRESERVED (3852B), wave0-host BYTE-PRESERVED modulo the additive 1660B pin step |
  | tier-9k job structure | 7 steps in plan order; timeout 90; runs-on ubuntu-latest |

- **Coverage regen + normalized-diff record (USPACE-02, D-15):**
  `nix develop -c sh build/aarch64/coverage-sweep.sh` on the lima guest
  (linux-aarch64) → COVERAGE-SWEEP-OK (206 visited rows, 15 in-universe
  exclusions, universe 221; jpg BLOCKER-CHECK host-ELF magic 7f454c46
  ×2).  Raw diff vs the pre-regen commit: the new venue block +
  five visited= count flips (auth/lib, awk, jtagfs, spell, upas/smtp —
  all build=ok unchanged; the disclosed watchdog/second-chance jitter
  class).  Normalized diff vs the pre-regen commit: the venue block
  ONLY.  Post-commit gate replay (the workflow's exact
  `git show HEAD:... | sed visited=N | diff` shape): NO-OP — clean.
  The generator emission and the committed report's venue block are
  byte-identical; the jpg note line keeps its count embedding as data.
- **Hosted run: PENDING FIRST PUSH** (the 05-13/06-08 precedent —
  harvey master is never pushed from this session): the tier-9k leg's
  first hosted observation happens on the next push; a human pushes and
  watches `gh run list --workflow phase6-userspace`.  Nothing
  fabricated; the local matrix above + the 06-09 local venue green
  (evidence/06-09-tier-9k-local.log) are the recorded evidence
  meanwhile.
- **Ledger lines appended:** none (this plan owns CI wiring + wording,
  not tier markers — T-06-20's grammar holds).
