---
phase: 15-userspace-tiers-bootfs-payload-amd64
plan: 03
subsystem: services-venue
tags: [services, r9x64, svc-markers, d-01-flushout, push-before-pin, fod-rewire, bootfs-services, mkimage-fifth-contract, tcreate, twrite, rfork-rffdg, uspace-07, x86_64]
duration: 120 min
completed: 2026-11-10
requires:
  - "15-02-complete donor services venue (services-sweep.rc + services-termrc baked UNCHANGED; the probe semantics, the rc-dialect rules, the loopback bring-up settled once for both venues)"
  - "15-01-complete tier-2 compile machinery + the phase build-tree lock the gate's build stage rides"
provides:
  - "mkimage.sh section 11 — the FIFTH bake contract: out/bootfs-services.paq (the chain tree + the tier-1 h* helpers + the tier-2 services set COMPUTED from tiers/tier2-services.txt (file/dir/flat-BIN resolver, duplicate h* names a bake error) + the 14-09 net members + the sweep's /boot-side members (digest-pinned sentinel, rcmain, tcp9901/tcp9902 staged copies) + the hsvctcpecho/hmkdir host-side 6c/6l builds; printed manifest, factotum-absence, KERNDATE determinism, both magic asserts)"
  - "build/x86_64/services-r9.expect + nix/services-r9-x64.sh + the boot-r9-services-x64 flake app — the composed r9 services gate: computed SVC expectations (subjects verified against the tier data, factotum on its tier-1 line, the declared SKIP set grepped verbatim from the sweep's own echo lines), the whole-file post-pass (OK-or-emitted-declared-SKIP per subject + the SKIP converse + zero SVC-FAIL/panic — the 15-02 donor classify semantics for conditional skips), green TWICE consecutively"
  - "the tier-1 SKIP flips (Pitfall 4, one commit): tier1-sweep.rc's r9 arm runs ipconfig (usage-level, the stale no-devrandom reason retired) and factotum (START+ANSWER, the 15-02 ctl-round-trip semantics in minimal tier-1 form) LIVE — tier1-r9.expect's venue-skip stage became the two live stages; the tier-1 gate re-greened in the same commit at 38 T1-OK + zero SKIP"
  - "build/x86_64/tier2.ledger — both venues' execution markers (exec=SVC-OK:k10t1+SVC-OK:r9x64), sha256 over the gate-run /amd64/bin artifacts, ZERO SKIP rows (no 18-owned substrate entry fired)"
  - "seven r9 kernel fixes (e560405..2a31fcc, all pushed before the pin advance): non-RFPROC rfork RFFDG/RFCFDG (the lib9p postmountsrv enabler), devmnt Twrite, devmnt Tcreate + sys_create DMDIR, devproc private/noswap verbs, the kernel heap 4->32 MiB with the oom arena-stats print, the devproc_test hook initializer, and the boot-iso FOD re-wire (two-derivation byte-identity proof)"
affects:
  - "build/x86_64/{mkimage.sh,tier1-sweep.rc,tier1-r9.expect,services-r9.expect,tier2.ledger}, nix/{services-r9-x64.sh,tier1-r9-x64.sh}, build/mm/boot-9k10t1-sweep.sh, flake.{nix,lock}"
  - "../r9 (7 commits at the advanced pin 3362cf7 + the test fix 2a31fcc)"
  - "15-05/15-09 (the same build-tree lock discipline, now proven under a services-length gate), Phase 17 (the measured per-exit heap residue leak handed over explicitly), Phase 18 (the auth substrate: no entry fired, the D-08 boundary held)"
tech-stack:
  added: []
  patterns:
    - "expect stage sequences are EMISSION-ORDERED, not merely presence-only: expect consumes the buffer up to each match, so a stage whose marker streamed past an earlier stage's match waits forever — the sweep's srv-before-exportfs pair is the measured case"
    - "grep-in-pipeline under `set -o pipefail` (every nix app wrapper) exits the driver on ZERO matches — every computed-expectation grep that may legitimately be empty takes `|| true` on the grep element, never a silent death between steps"
    - "lib9p's postmountsrv contract: the parent rfork(RFFDG)s IN PLACE to privatize its srvfd before the server child's close lands in the shared fd table — a kernel that rejects non-RFPROC RFFDG breaks every postmountsrv user with mount:bad-fd (the 15-03 measured finding)"
    - "9P policy lives in the SERVER, never the client transport: devmnt carries bytes (Twrite/Tcreate); paqfs keeps refusing (Erdonly is its answer) — 'the mounted tree is read-only' was a transport-layer policy bug"
    - "Tcreate TRANSFORMS the fid it is handed (fcall.h) — the create rides a CLONE of the parent's fid and the created chan owns the clone (the donor mntcreate shape)"
    - "the fixed .bss heap arena needs an era check whenever image sizes grow: the services venue measured 0x3ff000/0x400000 exhausted mid-sweep; the oom handler now prints the failed layout + arena high-water (arena-left = fragmentation, arena-full = capacity)"
key-files:
  created:
    - build/x86_64/services-r9.expect
    - nix/services-r9-x64.sh
    - build/x86_64/tier2.ledger
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-03-services-tracer1.log
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-03-services-runA.log
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-03-services-runB.log
    - .planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-03-tier1-flip.log
  modified:
    - build/x86_64/mkimage.sh
    - build/x86_64/tier1-sweep.rc
    - build/x86_64/tier1-r9.expect
    - nix/tier1-r9-x64.sh
    - build/mm/boot-9k10t1-sweep.sh
    - flake.nix
    - flake.lock
key-decisions:
  - "The tier-2 h* payload is the WHOLE computed member set (87 members), not a hand-picked probe list: every tiers/tier2-services.txt entry contributes every artifact it governs (file entries stage themselves, dir entries their whole installed tree, flat-BIN dir entries their basename artifact — subset-build's own BIN-disagreement table); duplicate h* names are a bake error and the probe subjects are asserted present in the computed set"
  - "The sweep's /boot-side members (sentinel, rcmain, tcp9901/tcp9902) ride the PAQ's boot/ and resolve through the namespace's `bind -a /root /` union — the donor's devroot-embed equivalents, one file per member, no sweep changes (the sweep rides UNCHANGED, both venues' single source)"
  - "The r9 gate's post-pass uses the 15-02 donor classify semantics, not literal SKIP-set equality: the sweep's SVC-SKIPs are CONDITIONAL arms (they fire only on venue degradation), so the check is per-subject OK-or-emitted-declared-SKIP plus the converse on every emitted name — on a green run the emitted set is empty and every subject must have earned its OK"
  - "The per-probe expect stages match the subject's VERDICT line (SVC-OK|SVC-FAIL), and the post-pass demands the OK — matching OK alone would turn every fix-loop iteration into a 600s stage timeout while teaching nothing (the capture is the evidence either way)"
  - "The kernel heap raise (4->32 MiB) is a capacity fix, deliberately NOT the leak fix: the arena trace measured a ~16 KiB per-exit residue (the never-reaped Proc husks' accumulated carve), and that leak audit is Phase 17's RFMEM-OOM-leaks backlog by the plan's own boundary — the raise removes the artificial ceiling and the oom print names the next era's evidence"
  - "The pin advanced to 3362cf7 (the five kernel fixes + the FOD re-wire); the devproc_test initializer fix (2a31fcc) landed after the pin — test-only bytes, no kernel change, no FOD impact"
patterns-established:
  - "The D-01 flush-out loop's measured shape this phase: boot the venue -> classify SVC-FAILs -> instrument (the arena-stats probe) -> root-cause against the 9k donor's source (sysproc.c/devproc.c named in every commit) -> one own commit per fix -> battery at each -> push -> FOD re-wire with the two-clone byte-identity proof -> lock-only pin"
  - "A SKIP-flip commit carries every consumer: the sweep's block, the expect's hardcoded stage list, and (measured) the pipefail-sensitive computed-expectation greps in ALL THREE drivers that read the sweep's SKIP lines"
requirements-completed: [USPACE-07]
actuals:
  tokens: 67782   # chars/4 over the realized diff (55228 code/build + 215903 evidence captures); estimate was 72000 — the first on-scale estimate of the phase
  tasks: 3
  commits: 4      # measured: git rev-list --count 062bd06a9..HEAD (harvey; +7 r9 commits named in the commit table)
commits: 4
plan_head_before: 062bd06a9c4078ef3225dbf5bda044bd4d46a699
plan_head_after: ede5ea8fe36360e68e5ca6c53a4b48cb1c34773c
status: complete
coverage:
  - id: D1
    description: "Task 1 (tracer) — the fifth bake contract (bootfs-services.paq) + the boot-r9-services-x64 app + the first boot: the pipeline works end-to-end and the capture classifies the findings"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "sh build/x86_64/mkimage.sh: PASS bootfs-services.paq 7644231 bytes, 124 h* binaries, header 529ab12b trailer 6b46e688, factotum-mention count 0 in the bake log; nix eval .#apps.x86_64-linux.boot-r9-services-x64.type = app"
        status: pass
      - kind: command
        ref: "the first gate run (evidence/15-03-services-tracer1.log): banner + SVC-LOOPBACK-UP + 10 SVC-class lines + the classified findings (the tracer's letter: a classified SVC-FAIL capture is Task-3 input, not a tracer failure); re-run post-commit reproduced the identical classified state"
        status: pass
    human_judgment: false
  - id: D2
    description: "Task 2 — the full r9 services gate green TWICE consecutively with computed expectations"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "rm -f /tmp/svc2a.log /tmp/svc2b.log; two consecutive `nix run .#boot-r9-services-x64` exit 0, zero SVC-FAIL in both, SVC-DONE in both (evidence/15-03-services-runA.log + runB.log) — all nine subjects interaction-backed: SVC-CSOBS ip=127.0.0.1, SVC-DNSOBS probehost.test, SVC-LISTENOBS tcp9901, SVC-FACTOBS the masked key, SVC-AUTHOBS connected"
        status: pass
    human_judgment: false
  - id: D3
    description: "Task 2 — the tier-1 SKIP flips in the same commit as the gate expectations, the tier-1 gate re-green"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "nix run .#boot-r9-tier1-x64: ALL STAGES PASS — 38 T1-OK (36 + ipconfig + factotum live), T1-SKIP set = 0 names, zero T1-FAIL (evidence/15-03-tier1-flip.log; re-run green on the flip commit); the flip greps: 0 remaining r9-arm SKIP mentions, 0 'echo T1-SKIP ipconfig' lines"
        status: pass
      - kind: command
        ref: "test -s build/x86_64/tier2.ledger && 12 tier=2 lines + 13 SVC-OK:r9x64 rows; factotum's sha256 matches tier1.ledger's row (the deterministic-rebuild cross-check)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Task 3 — D-01 fix-everything: every surfaced r9 gap fixed, pushed-before-pin, battery-proven, FOD re-wired"
    requirement: USPACE-07
    verification:
      - kind: command
        ref: "r9 e560405..2a31fcc: 7 fix(15-03) commits pushed BEFORE the pin advance (git rev-list --count --grep=fix.15-03 = 7); r9 tree clean; cargo xtask check OK + clippy OK; the full battery green at the final kernel fix commit: q35exec/q35shell/q35/q35net/aarch64-virt all PASS, port --lib 101/101"
        status: pass
      - kind: command
        ref: "the boot-iso FOD re-wire (r9@3362cf7): two independent fresh-clone kernel builds -> byte-identical ISOs (sha256 d3a4b6c5...bab), the FOD's measured got-value sha256-NCK9JST5... agrees at the NAR layer; harvey's pin advanced lock-only (2c2fe5592); nix build .#boot-iso green post-wire"
        status: pass
      - kind: command
        ref: "the post-fix re-runs: R9_SVC_LOG=/tmp/svc3.log nix run .#boot-r9-services-x64 exit 0 (zero SVC-FAIL, SVC-DONE) AND nix run .#boot-r9-tier1-x64 ALL STAGES PASS at the pinned kernel"
        status: pass
    human_judgment: false
---

# Phase 15 Plan 3: Userspace Tiers + Bootfs Payload (amd64) — r9 Services Venue Summary

**The r9-x86_64 services venue runs 15-02's committed sweep unchanged and all nine subjects earn interaction-backed SVC-OKs through five root-caused kernel fixes (postmountsrv's rfork RFFDG, devmnt Twrite, devmnt Tcreate+DMDIR, devproc private/noswap, the heap arena) — every fix pushed before the lock-only pin advance with the FOD re-wired, the stale tier-1 ipconfig/factotum SKIPs flipped live in the same commit as every gate expectation, and the gate green twice consecutively.**

## Performance

- **Duration:** 120 min (01:18Z→03:18Z; ~11 boot cycles inside — the tracer's finding classification, the arena instrumentation run, and the fix-loop re-boots)
- **Started:** 2026-11-10T01:18:31Z
- **Completed:** 2026-11-10T03:18:36Z
- **Tasks:** 3/3 (Task 1 tracer)
- **Files modified:** 10 harvey-side (+6 evidence captures; +7 r9 commits)

## Accomplishments

- **The tracer (Task 1)** proved the composed pipeline on the first boot: the fifth bake contract (7.6 MB, 124 h* binaries computed from the tier data), the ISO compose, and the sweep running from the boot flow with the net bring-up GREEN (SVC-LOOPBACK-UP on the first try — 14-09's devip port held). The capture classified five findings that became Task 3's fix inventory verbatim.
- **The D-01 flush-out loop fixed five real kernel gaps, each root-caused against the 9k donor's own source:** (1) non-RFPROC rfork RFFDG/RFCFDG — lib9p's postmountsrv privatizes its srvfd with an in-place fd-group copy before the server child's close lands in the shared table (sysproc.c:50-57); without it EVERY postmountsrv mount died "mount: bad fd" (mntgen, cs, dns, factotum, keyfs). (2) devmnt Twrite — "the mounted tree is read-only" was transport-layer policy; the SERVER enforces (paqfs still refuses). (3) devmnt Tcreate + sys_create's DMDIR ban lifted — keyfs's DMDIR create through the mount was mkdir's first measured caller; Tcreate rides a cloned fid (the wire transforms the fid it is handed). (4) devproc private/noswap — factotum's private() writes, donor-faithful (recorded flag / retired no-op). (5) the kernel heap 4→32 MiB — the instrumented arena trace measured exhaustion at proc ~73 (exec staging peaks + resident daemons + a per-exit residue; the LEAK stays 17's).
- **The union-walk fear dissolved on evidence:** listen's announce failure was collateral of the dead mounts (announce's nettrans consulted the hung-up cs mount); with the fixes landed the /net union (dns MBEFORE + cs MBEFORE + #I) resolves exactly as the donor does.
- **The tier-1 SKIP flips (Pitfall 4, one commit):** ipconfig's reason was stale since 14-09 (the port devcons serves /dev/random on x64); factotum runs START+ANSWER in minimal tier-1 form. The flip commit carried the expect's stage list AND the pipefail fix for the three drivers' SKIP-set greps (the emptied SKIP block had silently killed the tier-1 driver — grep exits 1 on zero matches under set -o pipefail).
- **Zero SKIP→18 entries:** no Phase-18-owned substrate fired — keyfs ran as a real service over an absent test keyfile (the D-08 shallow subset), and the p9sk1 boundary held (no key material in any capture).

## Task Commits

1. **Task 1 (tracer): the fifth bake + the gate** — `f5a9c0278` (feat) + `2c2fe5592` (fix: the lock-only pin advance, riding the tracer's fix loop)
2. **Task 2: the flips + the twice-green gate** — `7a9ad4b70` (feat: flips + expect + drivers + evidence) + `ede5ea8fe` (feat: tier2.ledger)
3. **Task 3: the r9 fix set** — r9 `e045dde` (rfork RFFDG), `06f8aa1` (devmnt Twrite), `de1fba1` (devproc verbs), `242f228` (heap + oom print), `21d2350` (devmnt Tcreate/DMDIR), `3362cf7` (FOD re-wire), `2a31fcc` (test initializer) — all pushed before the pin

**Plan metadata:** (docs — follows)

## Files Created/Modified

- `build/x86_64/mkimage.sh` — section 11, the SERVICES bake (the fifth contract)
- `build/x86_64/services-r9.expect` — the r9 services gate driver (verdict stages in emission order + the computed post-pass)
- `nix/services-r9-x64.sh` + `flake.nix` — the boot-r9-services-x64 app (the tier1 pipeline clone)
- `build/x86_64/tier1-sweep.rc` — the ipconfig/factotum flips (live checks)
- `build/x86_64/tier1-r9.expect` — the venue-skip stage became the two live stages
- `nix/tier1-r9-x64.sh`, `build/mm/boot-9k10t1-sweep.sh` — the pipefail-safe SKIP-set greps
- `build/x86_64/tier2.ledger` — both venues' execution markers, zero SKIP rows
- `flake.lock` — the r9 pin at 3362cf7
- `../r9` — the seven fix commits (rfork/devmnt×2/devproc/heap/FOD/test)

## Decisions Made

See key-decisions in the frontmatter — the load-bearing ones: the whole computed tier-2 payload (no hand-picked list), the /boot-union staging for the sweep's donor-side members, the donor-classify post-pass semantics for conditional skips, verdict-matching stages with the post-pass demanding the OK, and the capacity-not-leak framing of the heap raise.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The flip's consumer set was larger than the plan's file list**
- **Found during:** Task 2 (the first flip gate run)
- **Issue:** tier1-r9.expect hardcodes `stage venue-skip {T1-SKIP ipconfig}` (would red on the flip), and the computed-SKIP greps in tier1-r9-x64.sh / services-r9-x64.sh / boot-9k10t1-sweep.sh's compute_r9_skip die silently under the app wrapper's `set -o pipefail` when the sweep's SKIP block empties (grep exits 1 on zero matches — the tier-1 driver died between the bake and the expectations print with no error line).
- **Fix:** the flip commit carries all of them (the expect stage list + `|| true` on the grep element in all three drivers, each with the measured reason in place).
- **Files modified:** build/x86_64/tier1-r9.expect, nix/tier1-r9-x64.sh, nix/services-r9-x64.sh, build/mm/boot-9k10t1-sweep.sh
- **Verification:** the tier-1 gate green on the flip commit; the services gate twice-green after.
- **Committed in:** 7a9ad4b70

**2. [Rule 1 - Bug] The expect stage order vs the sweep's emission order**
- **Found during:** Task 2 (the first twice-green attempt stalled 900s at srv-verdict)
- **Issue:** expect consumes the buffer up to each match — the sweep echoes `SVC-OK srv` BEFORE `SVC-OK exportfs` from one if-arm, so the exportfs-first stage list ate the srv marker and the srv stage waited forever.
- **Fix:** the stages reordered to the sweep's emission order, with the rule recorded in the expect's comment block.
- **Files modified:** build/x86_64/services-r9.expect
- **Verification:** both consecutive runs green through all stages + the prompt.
- **Committed in:** 7a9ad4b70

**3. [Rule 1 - Bug] r9's devproc_test missed the new ProcHooks field**
- **Found during:** Task 3 (cargo xtask check)
- **Issue:** the private/noswap fix added the set_private hook field; the host test's ProcHooks initializer failed E0063.
- **Fix:** the test hook added (accepts for a live pid — the recorded-only era's honest stub), r9@2a31fcc.
- **Files modified:** ../r9/port/tests/devproc_test.rs
- **Verification:** cargo xtask check green; devproc_test 1/1.
- **Committed in:** r9 2a31fcc

**4. [Rule 3 - Blocking] mkimage's standalone-bake preconditions under the app wrapper**
- **Found during:** Task 1 (the first gate launch)
- **Issue:** the flake's readFile requires git-tracked files (the untracked driver red the app eval), and the wrapper's shellcheck rejected `$e[[:space:]#]` (SC1087).
- **Fix:** intent-to-add before the eval; `${e}` braces in the pattern.
- **Files modified:** nix/services-r9-x64.sh
- **Verification:** the app evaluates + builds; the gate ran.
- **Committed in:** f5a9c0278

**5. [Plan-wording interpretation] The post-pass's "SKIP set equality" and the probe stages' "SVC-OK presence"**
- **Found during:** Task 2 design
- **Issue:** the sweep's SVC-SKIPs are CONDITIONAL arms — literal set equality would red every healthy run (the declared set is non-empty, the emitted set empty when green); and OK-only stage matching would turn fix-loop iterations into 600s timeouts.
- **Fix:** the 15-02 donor gate's classify semantics (per-subject OK-or-emitted-declared-SKIP + the converse on every emitted name) and verdict-matching stages — both documented in the files; the plan's INTENT (a stale sweep reds the converse; an unearned SKIP is impossible; zero FAIL) is met exactly.
- **Files modified:** build/x86_64/services-r9.expect
- **Verification:** twice-green with the converse active.
- **Committed in:** f5a9c0278, refined in 7a9ad4b70

---

**Total deviations:** 5 auto-fixed (2 bugs, 2 blocking, 1 interpretation recorded). **Impact on plan:** all fixes are consumer-completeness or measured mechanics of the gates themselves; the plan's terminal truths are met exactly — no scope creep (the leak audit stayed 17's, no 17/18 scope pulled forward).

## The 17/18 Boundary Record (Task 3's obligation)

**Fixed here (services-sweep-surfaced, per D-01):** the postmountsrv rfork RFFDG class (explicitly named IN-scope by the plan), devmnt Twrite/Tcreate (the A7 growth — sized as fix tasks when the sweep surfaced them, and it did), devproc private/noswap (the factotum class, IN-scope per the plan's boundary note), the heap arena capacity.

**Stays Phase 17's (cross-checked, untouched):** the measured ~16 KiB per-exit heap residue (the never-reaped Proc husks — the plan's RFMEM-OOM-leaks backlog item, now with the arena-stats evidence trail the oom print gives), conv_close FIN-drop, cap-walk hardening. No CR/WR backlog item was pulled forward.

**SKIP→18 entries: NONE.** devcap `#¤`, the bake-time /adm/keys domain, keyfs-as-real-service, and real p9sk1 never blocked a probe: keyfs served its empty-DB + write-back path over an absent test keyfile (the D-08 shallow subset), authsrv proved listening (the handshake stays 18's), and no key material appears in any capture.

## Authentication Gates

None.

## Known Stubs

None — no placeholder paths, no unwired data, no skipped verifications. The flagged assumptions both RESOLVED on evidence: the concurrency probe's "sequential per boot, bounded spins, fresh boot per re-run" held (both consecutive green runs are fresh boots); A7's devmnt/sys_mount growth was sized and LANDED as fix commits (devmnt write + create; sys_mount itself needed nothing — pipe-9P + MBEFORE + the RFFDG fix carried every probe's topology).

## Issues Encountered

- The oom hunt took one instrumentation pass (the arena-stats probe + the per-exit heap trace, later trimmed to the permanent oom print): the 4 MiB exhaustion had three overlapping causes (exec staging peaks, resident daemons, per-exit residue) and the trace separated them in one boot — the raise addresses the first two, the residue is 17's with evidence.
- The expect stage-order stall (deviation #2) presented as a hang with no error line — the capture told the story (both markers present, the stage sequence stuck between them), and the fix is the recorded pattern.

## User Setup Required

None — no external service configuration required.

## Next Plan Readiness

READY for the remaining phase plans: 15-05/15-09 inherit the proven services-length gate pipeline (the build-tree lock held under a ~10 min full gate), the fifth bake contract's resolver pattern, and the FOD re-wire discipline rehearsed end-to-end this plan. The tier-1 gate now expects ZERO r9 skips — any future gap must land as a fix or a newly-declared skip in the same commit as the expectation change. USPACE-07's execution half is complete on both venues with committed markers; the requirement's remaining declarer (15-08) still owns its share.

## Self-Check: PASSED

- files exist: mkimage.sh, services-r9.expect, services-r9-x64.sh, tier2.ledger, tier1-sweep.rc, tier1-r9.expect, all four retained captures ✓
- commits are ancestors of HEAD: f5a9c0278, 2c2fe5592, 7a9ad4b70, ede5ea8fe (4 commits, measured) ✓
- plan-level verification green at HEAD: both gates green at the pinned kernel (svc3 zero SVC-FAIL + SVC-DONE; tier-1 ALL STAGES PASS 38 T1-OK); r9 clean + check/clippy OK ✓

---
*Phase: 15-userspace-tiers-bootfs-payload-amd64*
*Completed: 2026-11-10*
