---
phase: 17-r9-kernel-flush-out
plan: 02
subsystem: kernel
tags: [r9, x86_64, churn-gate, heap-slope, rfmem, brk, exit-path, flake-pin, windows-44]

requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net
    provides: the RFMEM/sem substrate + WINDOWS #44 (the open arena-drain row)
  - phase: 17-r9-kernel-flush-out
    provides: the D-02 audit + ports at pin 476605d (this plan's gates ran against that tree), CR-03's deterministic sem pins
provides:
  - the D-06 tiered leak gates, green twice each at the final pin — FLUSH-02's verification transfer discharged
  - the q35churn boot profile (r9): a kernel-context churn driver exec'ing the baked marker 1400x with a heap_stats cursor-slope assertion + 39 RFMEM semacquire/semrelease contention generations — CR-03's in-kernel leg landed
  - the boot-r9-churn-x64 composed gate (Harvey): /bin/cat x 1400 real mounted-binary fork+exec under a bounded-memory window parsed from the kernel's own per-exit markers
  - WINDOWS #44 CLOSED with the measured root cause: the drain was exit_tail's never-dropped transients (the detached fd-group Arc, ~8 KiB/exit) + the Moribund husk's retained group handles — NOT the ByteBacking::Mnt suspect; fixed r9 c4f2f2c + 72edbba (16 KiB/exec -> 60 bytes/exec kernel-side, 16 bytes/exit composed)
  - a second in-phase kernel fix the contention leg surfaced: the lone-member RFMEM brk arm left growth UNMAPPED (33/39 generations died at a fixed page fault until r9 72edbba's arm fix)
  - the leak-hunt diagnosis surface: QuickFit::split_census (live vs pooled) + the churn_ladder host pin
affects: [17-03 (the rc pool corruption finding rides its rows), 17-04, 17-07, VERIFICATION (walks 17-FLUSHOUT.md — the #44/WR-03/CR-03 cells filled)]

actuals:
  tokens: 34615   # chars/4 over the realized diff: Harvey 56332/4 + r9 82129/4
  tasks: 3
  commits: 8      # measured: Harvey 8 (17-02-keyed); r9 9 more (its own repo)

tech-stack:
  added: []
  patterns: ["the tiered leak gate: kernel-level slope assertion (driver-side fail-closed + host-side re-derivation from the kernel's own markers — never a second hardcoded constant) + composed-venue per-exit marker windows", "kernel-context driver procs: a proc seeded for kernel entry (the SCHED_CONTEXT seeding shape on a proc) orchestrating exec loops via sched()/wait-record wakes — netgate's boot-context shape generalized to scheduler-driven loads", "the negative control as a gate-teeth proof: the ceiling doctored below the measured baseline must red (FAIL marker + withheld DONE) before the gate is trusted"]

key-files:
  created: [../r9/x86_64/lib/config_q35churn.toml, ../r9/x86_64/lib/config_q35churnshell.toml, ../r9/x86_64/src/platform/q35/churngate.rs, build/x86_64/churn-r9.expect, build/x86_64/churnmark.c, build/x86_64/churnsem.c, build/x86_64/churncats.c, build/x86_64/root-tree/lib/churn-cats.rc, nix/churn-r9-x64.sh, .planning/phases/17-r9-kernel-flush-out/17-02-deferred-items.md]
  modified: [../r9/x86_64/src/{proc,syscall,main,allocator}.rs, ../r9/port/src/{allocator,allocator_tests,globalalloc}.rs, ../r9/xtask/src/main.rs, ../r9/{flake.nix,x86_64/Cargo.toml}, build/x86_64/{mkimage,stage-build}.sh, flake.nix, flake.lock, .planning/WINDOWS.md, .planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md]

key-decisions:
  - "The #44 verdict per the gates: BOUNDED-AND-CLOSED, but at a different root cause than the plan's suspect — the survivor probe measured ~8.8 KiB/cycle of retention with ZERO live-tracked slots, naming exit_tail's diverging-tail transients (the ~8 KiB fd-group Arc) + the husk's group handles/chans; ByteBacking::Mnt was never implicated (the composed tier's Mnt-backed cats drain 16 bytes/exit post-fix)"
  - "The bounded-memory mechanism (the flagged assumption): the PREFERRED arm — the composed venue's kernel (kernel-x64, re-pointed at the new q35churnshell config = q35shell verbatim + the churn dev key) carries the per-exit `churngate: heap exit` marker through the same cfg gate as tier (a); the composed gate bounds the churn window's per-exit delta from the kernel's own emitted numbers.  q35shell.toml itself stays byte-stable (Pitfall 7)"
  - "The churn profile runs as a KERNEL-CONTEXT PROC (seeded for kernel entry, always IRQ-masked, blocking via sched() with sleep_until watchdogs) rather than boot-context like netgate — the loop needs to wait on its children; MAX_PROCS is 2048 under dev_churn (the #44 shape is ~1600 procs vs the 512 boot-session budget)"
  - "The composed leg drives a C churn driver (churncats.c) instead of the rc fixture: rc's own userland pool corrupts past ~428 iterations of backquote churn (a real finding, deferred to 17-03's rows) — the gate's subject is the kernel's exec path"

patterns-established:
  - "Leak-gate teeth: the ceiling is a CONST derived from a first measured-green baseline (documented at the const + pinned by a test asserting CEILING == derive_ceiling(measured)), the negative control (doctored below baseline) must demonstrably red, and the host-side post-pass re-derives the verdict from the kernel's own markers"
  - "split_census: a 'used' header slot includes free-list-parked blocks — a live/pooled split is mandatory before a QuickFit census can name a leak"

requirements-completed: [FLUSH-02, FLUSH-04]

coverage:
  - id: D1
    description: "The r9-side churn boot profile (D-06 tier a): 1400 exec iterations + 39 RFMEM sem contention generations under a heap-slope ceiling, with proven teeth"
    requirement: FLUSH-02
    verification:
      - kind: unit
        ref: "cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu (69 passed / 0 failed incl. the 5 churngate pins + the churn_ladder pool pin; port 102/102)"
        status: pass
      - kind: integration
        ref: "cargo xtask boot-test --config q35churn — PASS (DONE iterations=1400 semgens=39, slope steady=420 measured=60 <= ceiling=1772, 1361 churnmark ring-3 prints counted, host re-derivation agreed); negative control (ceiling=32) red via the FAIL marker + withheld DONE; GREEN TWICE at the final kernel state"
        status: pass
    human_judgment: false
  - id: D2
    description: "The composed churn gate (D-06 tier b): /bin/cat x 1400 real mounted-binary fork+exec with the bounded-memory window"
    requirement: FLUSH-02
    verification:
      - kind: integration
        ref: "nix run .#boot-r9-churn-x64 — exit 0 twice consecutively (churn: CATS-DONE 1400 exactly once, zero churn: FAIL, the heap window 16 bytes/exit <= 1772 parsed from the kernel's per-exit markers)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The #44 verdict + fix loop: the measured drain root-caused, fixed in-phase push-before-pin (two FOD re-wires with two-derivation proofs), both tiers re-proven from scratch"
    requirement: FLUSH-04
    verification:
      - kind: integration
        ref: "WINDOWS #44 JSON status fixed + reason naming the r9 SHAs + resolved_at stamped; both tiers green twice riding the fix commits (pins 76d34a873 then 75ababa53); r9 pushed before each pin (origin/main..HEAD = 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "WR-03's verification transfer discharged (both tiers' gate cells) + CR-03's in-kernel contention leg landed"
    requirement: FLUSH-02
    verification:
      - kind: manual_procedural
        ref: "17-FLUSHOUT.md: the WR-03 row cites both tiers green twice; the CR-03 row's contention cell names the 39 generations (the leg also root-caused the lone-member RFMEM brk bug, fixed r9 72edbba)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The existing composed gate family re-greened at the advanced pin (kernel-x64 changed config)"
    verification:
      - kind: integration
        ref: "nix run .#boot-r9-net-smoke-x64 — ALL STAGES PASS at pin 76d34a873 (the churnshell-configured kernel boots the chain unchanged apart from the diagnostic marker line)"
        status: pass
    human_judgment: false

# Metrics
duration: 224 min
completed: 2026-11-12
status: complete
---

# Phase 17 Plan 02: Tiered Leak Gates — WINDOWS #44 Run to a Verdict Summary

**The D-06 tiered leak gates built and run green twice each — and they EARNED their keep: the gates root-caused #44's ~16 KiB/exec drain to exit_tail's never-dropped fd-group Arc + the husk's retained group handles (not the Mnt suspect), fixed it in-phase (60 bytes/exec post-fix), and the contention leg surfaced a second real bug (the lone-member RFMEM brk leaving growth unmapped) also fixed in-phase.**

## Performance

- **Duration:** 224 min
- **Started:** 2026-11-12T17:35:41Z
- **Completed:** 2026-11-12T21:20:10Z
- **Tasks:** 3/3
- **Files modified:** 26 (Harvey 12 + r9 14)

## The Ceiling and Its Derivation (OQ-2)

`CEILING_BYTES_PER_EXEC = 1772 = max(3 x steady, 1024) + 512` where **steady = 420 bytes/exec** — the first measured-green baseline's 2..=40 leg (the early leg carries allocator-warmup drift; the steady leg itself measured **60**).  Documented at the const (churngate.rs), pinned by a test asserting `CEILING == derive_ceiling(420)`.  **Teeth proven**: the const doctored to 32 (far below the measured 60) fired `churngate: FAIL per-exec cursor delta 60 over ceiling 32 (unbounded drain — the #44 shape)` and withheld DONE, reding the xtask gate; then reverted.  (The control's first run caught a driver bug — the verdict path withheld DONE without printing the FAIL line — fixed before the control was trusted: the gate now reds FAST on the marker.)

The composed gate's host-side bound mirrors the same derivation: **1772 bytes/exit** over the per-exit marker window.

## The Four Green Runs (all at the final pin state, flake.lock r9 97a5b7b)

| Run | Tier | Evidence |
| --- | --- | --- |
| 1 | (a) boot-test q35churn | `/tmp/churn-a.log`: DONE iterations=1400 semgens=39, slope steady=420 measured=60 ceiling=1772, zero FAIL |
| 2 | (a) boot-test q35churn | `/tmp/churn-b.log`: identical shape — green twice consecutively |
| 3 | (b) boot-r9-churn-x64 | `/tmp/cc-a.log`: churn: CATS-DONE 1400 once, zero churn: FAIL, window 16 bytes/exit <= 1772 |
| 4 | (b) boot-r9-churn-x64 | `/tmp/cc-b.log`: identical shape — green twice consecutively |

Run 3/4's kernel rode the pinned kernel-x64 (the q35churnshell config); runs 1/2 rode the identical kernel bytes built by xtask from the clean r9 tree at 97a5b7b.  Earlier greens at intermediate states: the first full tier-(a) green at r9 40d3d0f; the composed first-complete at the sweep-fix tree (kernel heap flat at 1238016 through the rc-corrupting runs).

## The #44 Verdict Narrative (bounded — closed at a different root cause than the plan's suspect)

The first tier-(a) run **fired as designed**: ~16 KiB cursor growth per exec, arena nearly exhausted, `allocator: header table exhausted` panic at ~1350 execs.  The hunt, decided by three probes:
1. **split_census** (live vs pooled — the naive census counts free-list-parked blocks as used): live = 0 everywhere → no tracked retention.
2. **The pooled-address probe**: parked fragments at a fixed 0x8000 stride → fragmentation drift, not the whole story.
3. **The one-cycle survivor probe** (10 blocks/cycle at fresh addresses: 1x8192 + 1x128 + 8x64 ≈ 8.8 KiB) → real retention in SLOT-LESS tail allocations.

Root causes: **exit_tail DIVERGES** (park / sched_from_moribund) so its locals never drop — the detached fd-group Arc (~8 KiB with the preopen cons chans) rode every dead kstack out; and the never-reaped Moribund husk pinned its pgrp/env/rgrp Arcs + slash/dot chans.  Fixes (r9 c4f2f2c): the transients drop before the diverging tail; `sweep_husk_payloads` swaps the group handles for shared singletons (the donor pexit's close-shape re-keyed) and drops the chans; sys_exits stack-copies its message.  Post-fix: **0 survivors/cycle, 60 bytes/exec**.  The lone-member RFMEM brk fix (below) was the contention leg's own find, landing in the same class commit orbit.

The follow-up sweep correction (r9 72edbba): close_chan on slash/dot dispatched 9P Tclunks from mid-exit — never 9P-safe, and the mounted venue measured userland pool corruption; the chans now drop plainly (the husk's server-side fid leaks bounded, documented).

**ByteBacking::Mnt was never implicated**: the composed tier execs Mnt-backed /bin/cat 1400x at 16 bytes/exit.

## The Contention Leg's Second Finding (fixed in-phase)

39 RFMEM semacquire/semrelease generations (churnsem: 4 kids x 200 contended cycles on a shared Data word, spawned nowait so contenders overlap the following execs).  The first full run died **33/39 generations** at a fixed page fault (pc in libc, addr = the malloc pool header): `rfmem_brk_set`'s **lone-member arm** — the state a family is left in when the last sibling exits — moved only segs.bss.end and never mapped the grown pages, so the survivor's first post-reap malloc fault-killed it.  Fixed (r9 72edbba): the arm maps growth exactly like the private arm.  Post-fix: 39/39 complete, zero traps, green twice.

## The Bounded-Memory Mechanism (the flagged assumption, resolved)

**The preferred arm, taken**: the composed venue's kernel (r9's flake kernel-x64, re-pointed at the new `config_q35churnshell.toml` = q35shell VERBATIM + the churn dev key) carries `churngate: heap exit <used> <total>` per exit through the same cfg gate as tier (a) — one dev key, no new kernel surface beyond the tier's own marker.  The composed expect driver bounds the BEFORE/AFTER window: delta over the window's exit count (every number parsed from the wire) <= 1772.  The fallback (exact-count + zero-oom + slope re-cite) was not needed.  q35shell.toml and every existing profile stayed byte-stable (Pitfall 7); the composed family re-greened at the new pin (net-smoke ALL STAGES PASS).

## The Pin-Advance Lineage (476605d -> 97a5b7b, two full cycles)

- r9 `fbfb08f` test(17-02): the churngate slope-logic pins — RED (RED_EVIDENCE_OK: `over_ceiling_delta_fails` failed on its planned assertion; libtest JSON mechanically converted to TAP for the classifier, raw JSON in evidence/)
- r9 `71c09d4` feat(17-02): the churn profile + driver + xtask wiring + both configs
- r9 `c4f2f2c` fix(17-02): the two root-caused kernel drains (the #44 fix)
- r9 `77ca088` fix(17-02): machinery lessons (the netdev stanza the q35 discovery contract requires, the 900s KVM-pace budget — the 16-08 soak precedent, the census)
- r9 `426aa56` fix(17-02): the verdict FAIL line (teeth) + the honest ceiling derivation
- r9 `40d3d0f` fix(17-02): the DONE stage waits for the complete line (the mid-flush truncation catch)
- r9 `ffba94e` fix(17-02): **FOD re-wire #1** — two-derivation proof (kernel A plain + kernel B under a churnproof-b version override, both through the non-FOD lib.bootIso realization): byte-identical ISOs, file sha256 `16bef2bffc095dfbd3c2577d6006ac252bf154cb661cdef554cb9963a8d73340`, NAR `sha256-wLZWhmeBy8MQb0HcRj44ToH7StPvqNsJy2CO/ntnRJA=`; build + --rebuild green
- r9 `72edbba` fix(17-02): the lone-member RFMEM brk fix + the sweep's 9P-safe plain drop
- r9 `97a5b7b` fix(17-02): **FOD re-wire #2** (kernel bytes moved again): byte-identical ISOs, file sha256 `fc5cde5559cb1728433e7392aea0480400978ca7d07a09b44c3e760dabd4accf`, NAR `sha256-bw7AnkikIXnYGPYhJEd/DSa/cENfaL/dwu8DMN/GWbk=`
- r9 pushed `476605d..97a5b7b` BEFORE each Harvey advance — push-before-pin held both cycles (origin/main..HEAD = 0 verified at each push)
- Harvey `76d34a873` pin -> ffba94e (diff = **flake.lock only**, asserted PIN-COMMIT-CLEAN); Harvey `75ababa53` pin -> 97a5b7b (lock-only)

## Task Commits (Harvey repo)

1. **Task 1's Harvey half** - `da230038d` (feat): the churn marker pair staged into the marker image
2. **Task 2's pieces** - `0fd71322c` (feat: expect/fixture/wrapper), `1bb01a2e0` (feat: the churn bake + flake app), `981acd75b` (fix: the C churn driver + the Tcl brace fix)
3. **Pin advances** - `76d34a873`, `75ababa53` (chore, lock-only each)
4. **Task 3** - `ca2882223` (docs: the #44 verdict + FLUSHOUT cells)

(r9's 9 commits listed in the lineage above.)

## TDD Gate Compliance

Task 1 carried tdd="true": **RED** r9 fbfb08f (the target test `over_ceiling_delta_fails` failed on its planned assertion — stub verdict fn accepted an over-ceiling slope; 4/5 tests failed, classifier verdict RED_EVIDENCE_OK via a TAP conversion of the real libtest JSON stream, raw preserved in evidence/) → **GREEN** r9 71c09d4 (5/5).  One GREEN-phase test correction, documented in the test: the RED expectation `derive_ceiling(100) == 812` contradicted the formula's own floor rule (max(3x100, 1024) = 1024) — an arithmetic slip in the spec, corrected to pin the floor/3x crossover honestly.  No REFACTOR commit (the driver is first-landing code; no cleanup warranted).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The churn profile starved the q35 discovery contract**
- **Found during:** Task 1 (the first full gate run expired at the discovery stage with the whole churn in the capture)
- **Issue:** the profile was authored net-free (no [qemu.netdev] stanza) — but every q35 profile's finish() asserts the virtio-net device-found line, and qemu creates no NIC without a netdev
- **Fix:** the stanza carried from q35exec (device present, UNBOUND — no ether dev key); r9 77ca088
- **Files modified:** ../r9/x86_64/lib/config_q35churn.toml

**2. [Rule 3 - Blocking] The 300s default budget killed the churn loop mid-run**
- **Found during:** Task 1 (the fixed kernel's first full run completed the loop but the gate expired)
- **Issue:** the measured KVM pace is ~0.28s/exec cycle (~400s for the loop + drain), consistent with 16-08's measured KVM syscall pacing; the carried default is 300s
- **Fix:** the q35churn profile's own 900s budget in xtask (the 16-08 soak_stage_timeout precedent; env still wins); r9 77ca088

**3. [Rule 1 - Bug] The verdict path withheld DONE without printing the FAIL marker**
- **Found during:** Task 1's negative control (the doctored ceiling reded only as a slow budget expiry)
- **Fix:** the slope-over-ceiling arm prints the FAIL line naming both numbers before finish(false); r9 426aa56

**4. [Rule 1 - Bug] The DONE stage matched its marker mid-flush**
- **Found during:** Task 1 (the post-pass parsed a truncated `iterations=1400 semge` line and errored)
- **Fix:** the stage requires the parseable line (semgens= present); r9 40d3d0f

**5. [Rule 1 - Bug] The window post-pass's Tcl brace bug**
- **Found during:** Task 2 (the inner {} made a literal string, not a boolean)
- **Fix:** plain comparison; Harvey 981acd75b

**6. [Rule 2 - Missing critical] The composed fixture needed a C driver**
- **Found during:** Task 2 (rc's own pool corrupts past ~428 iterations — a real finding, deferred)
- **Fix:** churncats.c (the tcpblast lineage) driving fork+execve/waitpid directly; the rc fixture stays staged as cat's input + the recorded repro; Harvey 981acd75b

**7. [Rule 4-adjacent, resolved within the plan's own mandate] The #44 fix orbit was the exit path, not the exec staging**
- **Found during:** Task 1 (the survivor probe's 10-blocks/cycle retention)
- **Issue:** the plan named ByteBacking::Mnt teardown as the suspect; the measurement named exit_tail's transients + the husk's retained handles instead
- **Fix:** per the plan's own verdict rule ("the fix lands in-phase on the ... orbit ... as a class commit, push-before-pin, FOD re-wire, own-commit pin advance, then BOTH tiers re-run green twice from scratch") — executed exactly, at the measured site: r9 c4f2f2c, two FOD cycles, both tiers green twice from scratch

---

**Total deviations:** 7 auto-fixed (2 blocking, 4 bugs, 1 missing-critical) + the verdict-orbit note
**Impact on plan:** All fixes were the gates doing their job — the plan exists to run leak gates that can fail, and they failed USEFULLY twice (the drain + the brk bug) before going green.  No scope creep; the rc-pool finding is recorded, not chased.

## Issues Encountered

- The FOD proof needed two cycles (the sweep fix re-staled the first re-wire's kernel bytes within the same plan) — both with the full two-derivation proof
- The first local composed validation composed the WRONG kernel (the q35churn — non-bootchain — ELF instead of q35churnshell): the walk panic named it instantly; recomposed correctly
- expect's 127 in the r9 devshell context (not on PATH there): the composed validation runs from Harvey's devshell

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 17-03's rc rows gain a measured new member: the userland pool corruption under backquote churn (17-02-deferred-items.md, the repro shape committed)
- 17-04/17-07 proceed on the audited+churn-proven kernel at pin 97a5b7b; the FLUSHOUT index carries the filled #44/WR-03/CR-03 cells
- The sys_sleep units slip and the exit-path 9P-safety note are recorded in the deferred items

## Self-Check: PASSED

- Created/modified files exist on disk (churn-r9.expect, churncats.c, churn-cats.rc, churn-r9-x64.sh, mkimage/stage-build, 17-02-deferred-items.md; r9 churngate.rs + both configs)
- Commits verified ancestors of HEAD: da230038d, 0fd71322c, 76d34a873, 1bb01a2e0, 981acd75b, 75ababa53, ca2882223 (Harvey, 8 measured incl. this SUMMARY's docs commit); r9 fbfb08f..97a5b7b on origin/main
- Plan-level verification re-run post-commit: tier (a) green twice (/tmp/churn-{a,b}.log), tier (b) green twice (/tmp/cc-{a,b}.log), the task-3 greps (ROW44-CLOSED, FLUSHOUT-44-FILLED), net-smoke green at the advanced pin, batteries 69/69 + 102/102 + xtask 35

---
*Phase: 17-r9-kernel-flush-out*
*Completed: 2026-11-12*
