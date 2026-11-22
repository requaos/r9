---
phase: 05-shell-boot-one-command-integration
plan: 03
subsystem: infra
tags: [harvey, aarch64, donor-kernel, syscall-trace, measure-first, boot-chain, paqfs, devproc, startsyscall, qemu]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-02)
    provides: out/{boot,init,paqfs,stacksmoke} + the baked bootfs.paq + aarch64/bin subset — the trace targets
  - phase: 04-r9-userspace-exec-harvey-r9-abi (04-01)
    provides: the D-14 hello-path trace rig (trace-syscalls.py hello mode), the startsyscall+/proc/N/syscall recipe, and the artifact schema this plan reuses
  - phase: 02-aarch64-toolchain-libc-9front-donor
    provides: the donor harness (lima guest, run-gate staging discipline, srv9/gdbpoke9/drive9 machinery)
provides:
  - 05-03-trace.txt — the MEASURED syscall set of the real boot chain (26 names; 16 NEW vs Phase 4) — the authoritative implement-exactly-this input for 05-04..05-09
  - the real-chain trace rig: trace-syscalls.py --boot-chain + trace-boot.rc + pidwait.c + trace-namespace/trace-termrc/trace-sweep.rc guest assets
  - proof that our boot(8)/init/paqfs/rc/stacksmoke binaries run their REAL flows on the donor kernel (opencons → connectembed → fversion → mount → nsinit → execinit → init -t → newns → termrc → sweep → exit)
affects: [05-04..05-09 (every dispatch-growth plan implements exactly this set), 05-09 (the sourced-vs-typed sweep accommodation informs the r9 smoke design), INT-02]

# Actuals (#2632) — pairs with the plan's estimate (80000 tokens, confidence: low)
actuals:
  tokens: 14800     # chars/4 over the realized diff (~59KB authored: rig +1221 lines incl. 3 guest assets, the artifact 83 lines)
  tasks: 2
  commits: 2        # MEASURED: git rev-list --count 2d3ac0189..HEAD (task commits); the metadata commit rides on top
plan_head_before: "2d3ac01898276489e13f2a4af7ed2369d654354e"

# Tech tracking
tech-stack:
  added: []          # zero new dependencies (T-05-SC held: no package installs)
  patterns:
    - "born-traced children: a traced process's fork inherits procctl (donor sysproc.c:155) and self-stops at its first syscall with the entry fragment pending — arm via pidwait -w + traceloop1 (read the pending fragment BEFORE the first startsyscall resume)"
    - "per-pid fragment tagging: the donor's entry fragments carry no trailing newline (sysretfmt's return supplies it), so concurrent loops MUST re-emit via sed 's/^/TAG/' — raw cats glue entries mid-line and line-anchored parsing loses singletons"
    - "a traced pid must contain no forks before arming is complete: rc has no bind builtin (forks /bin/bind), @{...} is a subshell (forks again) — both wedge in Await with born-traced self-stopped children nobody armed"
    - "Log buffers for offset-based searching must never trim (a trim silently shifts every captured base)"

key-files:
  created:
    - build/aarch64/pidwait.c
    - build/aarch64/trace-boot.rc
    - build/aarch64/trace-namespace
    - build/aarch64/trace-termrc
    - build/aarch64/trace-sweep.rc
    - .planning/phases/05-shell-boot-one-command-integration/05-03-trace.txt
  modified:
    - build/aarch64/trace-syscalls.py

key-decisions:
  - "the sweep is SOURCED (trace termrc → `rc -c '. /boot/oursweep.rc'`), not typed: the donor's devcons fails /dev/cons reads by design (Egreg — userland console input on the donor is kbdfs-served, and opencons's bind #c MBEFORE pins the kernel cons into the chain's fd 0); r9's devcons serves reads directly (D-06) so this divergence does not carry to the target"
  - "#e/init is removed at seed time: the donor's bootrc leaks init=/arm64/init -t into the shared env group and execinit prefers $init over the cputype branch — the trace must take the cputype branch (/aarch64/init -t), which is the r9-faithful one"
  - "the donor's /srv/boot is removed before launch (srvcreate(\"boot\") is fatal on collision) and at teardown (the posted chan pins the connectembed pipe's p[0] open — the paqfs server's read would otherwise never see the hangup)"
  - "paqfs is traced as TWO streams: its main line (init+verify+fork+exits — the stdio path's parent exits by design) and its fork child, the actual 9P server (born traced, self-stopped, armed via pidwait -w paqfs)"

patterns-established:
  - "trace a multi-process rendezvous chain: one startsyscall loop per pid + born-traced arming in strict birth order + a sweeper for untracked traced-born children + a watchdog that kills by name with existence-guarded redirects"
  - "the measured-set artifact names its coverage boundary in the header and proves the boot chain's first syscalls with a contiguous-run check (opencons septuplet at a named stream offset)"

requirements-completed: [INT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The rig extends to the real chain and drives it end-to-end on the donor kernel: our boot/init/paqfs/rc binaries run their real flows (connectembed pipe+fork+exec of our paqfs over our baked bootfs.paq, fversion, mount at /root, nsinit, execinit /aarch64/init -t, newns over the trace namespace, termrc's stacksmoke, the D-09 sweep sourced by rc); per-process fragments captured (boot+init 312, paqfs-main 84, paqfs-server 567, rc 392, sweep binaries 22-124 each)"
    requirement: INT-01
    verification:
      - kind: integration
        ref: "python3 build/aarch64/trace-syscalls.py --boot-chain → '== PASS: real-chain trace complete' (run log .build/bootchain-run.log; serial evidence .build/9front-dist/bootchain-trace-run.log)"
        status: pass
      - kind: other
        ref: "serial evidence markers: __GSD_ARM_boot/paqfs/srv/rc + CHAIN_ARMED + stacksmoke depth-64 marker + __GSD_SWEEP_ECHO__ + CHAIN_DONE + ALL_DONE, zero panic/trap lines"
        status: pass
    human_judgment: false
  - id: D2
    description: "hello-path mode unregressed: full 04-01 re-run completed '== PASS: D-14 trace complete' with a byte-identical 5-syscall measured set (pwrite/open/pread/close/exits)"
    requirement: INT-01
    verification:
      - kind: integration
        ref: "python3 build/aarch64/trace-syscalls.py (hello mode) re-run green; diff of the regenerated vs committed measured set empty; 04-01 artifact restored"
        status: pass
    human_judgment: false
  - id: D3
    description: "05-03-trace.txt: schema-clean measured set (26 names), every name sys.h-resolved, connectembed/nsinit core set (bind/pipe/fversion/mount/dup + stat/fstat) measured present, header names traced processes + donor pin + coverage boundary + NEW-vs-Phase-4 delta; zero source-read names folded into measured counts"
    requirement: INT-01
    verification:
      - kind: other
        ref: "the plan's Task-2 verify gate 1 (schema + sys.h resolution loop) and gate 2 (required-set presence + stat family) both PASS, run verbatim"
        status: pass
    human_judgment: false

# Metrics
duration: 225 min
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 03: Real-chain donor trace — the measured syscall set for the kernel stream Summary

**The REAL boot chain (05-02's aarch64 boot(8)/init/paqfs/rc + the D-09 sweep binaries) ran end-to-end on the 9front donor kernel and its syscall footprint is MEASURED and authoritative: 26 names, 16 NEW vs Phase 4 (bind, chdir, dup, alarm, fauth, sleep, pipe, create, fd2path, brk_, unmount, seek, fversion, stat, fstat, mount), with opencons provably captured and zero boot-flow slippage. The rig that produced it (one scripted run: stage → guest rebake → boot → arm-in-birth-order → sweep → teardown → parse) is committed and rerunnable; the hello-path mode re-ran green with a byte-identical set.**

## Performance

- **Duration:** 225 min (four harness bring-up iterations — each caught by its own gate, see Issues)
- **Started:** 2026-11-03T01:12:07Z
- **Completed:** 2026-11-03T04:57:00Z
- **Tasks:** 2
- **Files modified:** 7 (6 rig + 1 artifact)

## Accomplishments

- **The measure-first gate is satisfied for the real chain.** `05-03-trace.txt` records 26 measured syscalls — the boot-chain core (bind/pipe/fversion/mount/dup + stat/fstat) all present, opencons's close×3/bind/open×3 a contiguous run at stream offset 17 (the header's self-evidencing 'boot stream opens with' line), sleep from init's restart loop captured during teardown. Every name resolves against `sys/src/libc/9syscall/sys.h` (plan gate 1 verbatim, PASS) and the expected set is present (plan gate 2 verbatim, PASS). Per-process inventory in the header: boot+init pid 234 (312 fragments, exec-shared stream), paqfs main 1259 (84), paqfs 9P server 1741 (567), rc 3413 (392), standalone sweeps ourecho 23 / ourls 34 / ourps 124 / ourcat 30 / ourstacksmoke 22.
- **The measured set settles A1's informed list by evidence.** NEW vs the Phase-4 implemented set {close, exec, exits, open, rfork, notify, noted, errstr, await, pread, pwrite}: **bind, chdir, dup, alarm, fauth, sleep, pipe, create, fd2path, brk_, unmount, seek, fversion, stat, fstat, mount** (16). Surprises the trace surfaced: **brk_ ×28** (rc's malloc growth importing the env — A1 had none, Phase-4's hello path had none — r9 must implement it), **seek ×68 in the paqfs server** (its image-access pattern), **fd2path ×3** (rc bookkeeping), **unmount ×1** (boot's settime /tmp handling), **fauth ×2** (nsinit + newns famount — both fall through unauthenticated exactly as D-02 predicts). And the inverse surprise: A1 listed REMOVE and WSTAT as expected — neither fired (execinit's chmod of the absent partfs file fails before wstat; nothing removes). r9 keeps them loud-unimplemented per policy.
- **The rig generalizes the 04-01 recipe into a multi-process tracer.** The load-bearing pieces, all committed and documented in file headers: born-traced-child arming (`pidwait -w` + a read-the-pending-fragment-first loop shape), sed-tagged per-pid fragment lines (the donor's entry fragments carry no trailing newline — concurrent raw cats glue them mid-line and singletons like dup get lost), a sweeper for rc's traced-born command children, an existence-guarded watchdog (unguarded redirect errors abort rc scripts), and the donor accommodations: `/srv/boot` collision removal, the `'#c/null'` factotum cover (D-02), `#e/init` removal, and the trace namespace that re-mounts `#s/boot` at `/root` after newns's RFCNAMEG wipe (the k10root mechanism the image's scoped namespace file must grow in 05-08 — see Next Phase Readiness).

## Task Commits

Each task was committed atomically (harvey-only plan — no r9 files touched):

1. **Task 1: the real-chain trace rig** — `e4d98c9cd` (feat)
2. **Task 2: 05-03-trace.txt measured-set artifact** — `a624fc181` (feat)

**Plan metadata:** (the docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS)

## Files Created/Modified

- `build/aarch64/trace-syscalls.py` — extended with `--boot-chain` mode (staging, guest rebake, drive, parse); hello mode byte-preserved except two additive shared hardenings (ENTRY_RE tolerates fragments ending at the pc field; Log cap 1MB→256MB)
- `build/aarch64/trace-boot.rc` — the guest driver (chain + sweep phases, launcher spin-arm, arming order, sweeper, watchdog)
- `build/aarch64/pidwait.c` — the in-process /proc watcher (wins the fork→dup arming race rc can't)
- `build/aarch64/trace-namespace` + `trace-termrc` + `trace-sweep.rc` — guest-scratch rig assets (the sourced-sweep accommodation)
- `.planning/phases/05-shell-boot-one-command-integration/05-03-trace.txt` — THE artifact (05-04..05-09 implement exactly this set)

## Decisions Made

- **Sourced sweep over typed sweep (the donor-console divergence).** The plan's method note expected console-driving ("send the sweep commands, keep tracing the rc pid"). On the donor that cannot work: opencons's `bind("#c","/dev",MBEFORE)` pins the KERNEL cons into the chain's fd 0, and the donor's devcons fails /dev/cons reads with Egreg BY DESIGN (userland console input on the donor is kbdfs-served, mounted MBEFORE at /dev/cons by bootrc-era kbdfs — MBEFORE-vs-MBEFORE makes the kernel cons un-un-coverable). rc's first `. -i '#d/0'` read returns -1 instantly. The accommodation sources the sweep via a trace termrc override (`exec rc -c '. /boot/oursweep.rc'`): every sweep command is an rfork/exec/await triplet in rc's MEASURED stream, and the binaries' own footprints come from the standalone sweep phase. rc's console read itself is a documented donor/r9 divergence (D-06: r9's devcons serves reads directly; 05-09 drives the console for real on r9).
- **`#e/init` removal at seed time.** The donor's bootrc leaks `init=/arm64/init -t` into the shared env group; boot.c's execinit prefers `$init` over building `/%s/init -t` from cputype. The first full run measured the DONOR's init as a result — real, but not our binary. Removing the seed forces the cputype branch (the r9-faithful path).
- **paqfs is two traced streams.** paqfs always rforks its server and the parent exits immediately in stdio mode ("don't deadlock if child fails") — the 9P service is the grandchild's stream. The artifact's per-role counts name both (paqfs=84 main, srv=567 server).
- **Artifact numbers are syscall NUMBERS (04-01's exact schema), not occurrence counts** — occurrence counts ride the NEW-section comments.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] ENTRY_RE dropped fragments ending at the pc field**
- **Found during:** pre-run parser validation (synthetic capture test)
- **Issue:** the inherited 04-01 regex required whitespace after the pc; a stripped line ending at `pc` (a syscall whose entry prints no args) would be silently uncounted
- **Fix:** widened to `[0-9a-f]+(?:\s|$)` (additive-safe for the hello path; 04-01's artifact was unaffected — all five of its syscalls print args)
- **Files modified:** build/aarch64/trace-syscalls.py
- **Verification:** synthetic parse test + hello-mode regression run produced the identical set
- **Committed in:** `e4d98c9cd`

**2. [Rule 3 - Blocking] pidwait: dirreadall arity + kernel-decorated exit status**
- **Found during:** guest compile + first armed run
- **Issue:** (a) `Dir **dirs` was one level too deep for dirreadall(int, Dir**) (guest 7c error); (b) the pid was passed back via exit status, but the donor's pexit decorates non-empty exits to "name pid: status" — the guard misfired
- **Fix:** (a) `Dir *dirs` + `.name`; (b) pid printed to stdout, captured via backquote
- **Files modified:** build/aarch64/pidwait.c, build/aarch64/trace-boot.rc
- **Verification:** pidwait compiles clean (R_MAGIC proven); the connectembed child armed deterministically (`__GSD_ARM_paqfs_1255__`)
- **Committed in:** `e4d98c9cd`

**3. [Rule 3 - Blocking] the traced launcher wedged in Await (twice: /bin/bind fork, then `@{...}` subshell fork)**
- **Found during:** harness bring-up runs 2 and 3
- **Issue:** a traced process's fork children are born traced (donor sysproc.c:155) and self-stop at their first syscall; the launcher's `/bin/bind` forks and later the `@{ }` subshell fork each created an unarmed self-stopped child the launcher awaited forever
- **Fix:** staging binds/env writes moved to the untraced driver shell; the launcher is a PLAIN brace group (`{ spin; exec /boot/ourboot } &`) — no forks in the traced pid before the arm
- **Files modified:** build/aarch64/trace-boot.rc
- **Verification:** the chain armed end-to-end in the next run
- **Committed in:** `e4d98c9cd`

**4. [Rule 3 - Blocking] untagged fragment reads glued mid-line; rc `$"f` flatten quotes empty words**
- **Found during:** harness bring-up runs 3-4 (mid-line gluing observed in the serial log)
- **Issue:** the donor's entry fragments carry no trailing newline; concurrent per-pid cats glue entries mid-line, defeating line-anchored parsing (dup is a singleton pair — not losable); an rc backquote+echo re-tag attempt corrupted empty words (`""` quoting)
- **Fix:** each loop's fragment is re-emitted byte-exact as one tagged line via `cat | sed 's/^/__GSD_FRAG_<pid>__/'`; the parser keys on the tag and cross-checks the fragment's own pid field
- **Files modified:** build/aarch64/trace-boot.rc, build/aarch64/trace-syscalls.py
- **Verification:** zero entry fragments lost in the final run (all 596 non-entry tagged lines classify as return-side prints — sysretfmt dumps the data buffer before `= <ret>`; verified programmatically)
- **Committed in:** `e4d98c9cd`

**5. [Rule 3 - Blocking] #s/boot collision + teardown wedge (unguarded kill redirect + posted pipe pinning p[0] open)**
- **Found during:** harness bring-up runs 4-5
- **Issue:** (a) the donor's own boot posts its root-fs connection as `#s/boot`; our boot's `srvcreate("boot", fd)` is fatal on the collision; (b) at teardown the watchdog's kill of the already-exited paqfs main hit a redirect error (which aborts an rc script), and the chain's posted `#s/boot` entry pins the connectembed pipe's p[0] open so the paqfs server's read never saw the hangup — full wedge
- **Fix:** `rm /srv/boot` before launch (the donor root keeps working via its mount ref) and again at teardown; all watchdog kills guarded with `test -e`
- **Files modified:** build/aarch64/trace-boot.rc
- **Verification:** `__GSD_CHAIN_DONE__` then the donor prompt then `__GSD_ALL_DONE__` in the final run
- **Committed in:** `e4d98c9cd`

**6. [Rule 3 - Blocking] host driver: Log trim invalidated search bases; marker regexes missing `\r?`; prompt-base race**
- **Found during:** harness bring-up runs 5-6 (driver stuck past markers)
- **Issue:** (a) the inherited Log class trimmed its buffer at 1MB, silently shifting every captured search base; (b) the console cooks \n→\r\n, so anchored markers need `\r?$`; (c) the donor prompt could arrive in the same burst as CHAIN_DONE, before the follow-up `log.length()` capture
- **Fix:** (a) Log cap 256MB (effectively no trim; additive-safe for hello mode); (b) all anchored markers CRLF-tolerant; (c) prompt base captured before the CHAIN_DONE wait
- **Files modified:** build/aarch64/trace-syscalls.py
- **Verification:** the final run completed hands-off ('== PASS: real-chain trace complete'); hello-mode regression green after
- **Committed in:** `e4d98c9cd`

**7. [Rule 2 - Missing Critical] guest CONF=qemu bootdir exceeded the donor's Nbootfiles=32**
- **Found during:** first rebake (donor kernel panicked "too many root files" at boot)
- **Issue:** the guest-scratch CONF had accumulated bootdir lines from every gate since 02-05 (37 > 32)
- **Fix:** the guest rebake script prunes the bootdir section to the four stock rows before adding the 05-03 set (dropped gates re-add their own lines idempotently on their own reruns; nothing committed anywhere)
- **Files modified:** build/aarch64/trace-syscalls.py
- **Verification:** the rebaked kernel boots clean
- **Committed in:** `e4d98c9cd`

---

**Total deviations:** 7 auto-fixed (6 Rule 3 blocking, 1 Rule 1, 1 Rule 2 — the counts overlap: one fix covered two rule classes)
**Impact on plan:** Every deviation is the harness doing its job — each was caught by its own gate/wedge and fixed structurally, never papered over. The measured set itself is exactly what the plan mandated: measured, not read. No scope creep; the donor tree stays pristine (all accommodations are guest-scratch or harness-side).

## Issues Encountered

- **The donor bootrc leaks `init=/arm64/init -t` into the shared env group** — the first armed run measured the DONOR's init (execinit's `$init` branch). Fixed by removing `#e/init` at seed time (documented in the artifact header). Not a bug in our chain — a staging fidelity issue.
- **The sourced-vs-typed sweep** (Decision 1 above) is the one place the plan's method note ("drive the donor console") proved inapplicable to the donor: documented as a divergence, with r9-side unaffected (D-06).
- The watchdog's kill of the already-exited paqfs main prints a cosmetic redirect error (survived, harmless) — noted for rig hygiene; not worth a loop iteration of silence machinery.

## User Setup Required

None - no external service configuration required. (The rig reruns need the lima `harvey` VM and the devshell — the phase's standing environment.)

## Known Stubs

None.

## Threat Flags

None — no new trust-boundary surface beyond the plan's threat model. T-05-05 (measurement integrity) mitigations held: schema + name-resolution gates PASS, expected-set gate PASS, opencons contiguous-run check PASS, zero source-read names folded into measured counts, the coverage boundary is named in the artifact header. T-05-SC: zero package installs.

## Next Phase Readiness

- **05-04..05-09 implement EXACTLY the 26-name set** (plus Phase 4's 11): dispatch growth cites `05-03-trace.txt`, and the loud-unimplemented net catches any residual miss by name. Highest-value NEW names by occurrence: seek (srv's image access), brk_ (rc malloc growth), create (env/srv registration), stat, open/close volume, fauth (falls through), unmount (settime), fd2path (rc bookkeeping), alarm (init), chdir (ps/newns).
- **Finding for 05-08 (namespace plan):** the image's committed scoped `/lib/namespace` is missing the replay prerequisite the RFCNAMEG wipe makes mandatory — it must open with the k10root-style `mount -a $rootsrv /root $rootspec` + `bind -a /root /` (with `#s/boot` as rootsrv) BEFORE the `/root/$cputype/bin` binds resolve. The trace proved this empirically on the donor (the trace-namespace file carries exactly that shape); without it, init's post-newns namespace can't resolve /bin/rc.
- **Finding for 05-09 (smoke plan):** the r9 smoke CAN drive the console for real (r9's devcons serves reads, D-06) — the donor trace's sourced-sweep accommodation does not carry over. The `% ` prompt assertion belongs to the r9 run.
- The note about REMOVE/WSTAT never firing (loud-unimplemented stays) is recorded for the dispatch plans.
- r9 HEAD: untouched this plan (`d70e6c7`). Harvey task commits: `e4d98c9cd`, `a624fc181`.

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: pidwait.c, trace-boot.rc, trace-namespace, trace-termrc, trace-sweep.rc, trace-syscalls.py, 05-03-trace.txt, 05-03-SUMMARY.md — all FOUND on disk and committed.
- Commits: e4d98c9cd (rig), a624fc181 (artifact) — both FOUND in git log.
- Plan-level <verification>: rig extended without hello-path regression (re-run green, identical measured set); artifact schema-clean + names resolve + boot-chain core set measured present + coverage boundary documented (both Task-2 gates re-verified at summary time).
