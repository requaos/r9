---
phase: 08-r9-tcp-ip-path-devip-compatible-net
plan: 09
subsystem: infra
tags: [smoltcp, devip, composed-smoke, expect, ci, github-actions, push-before-pin, flake-lock, evidence, supply-chain, plan9, aarch64, r9, no_std]

# Dependency graph
requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net (plans 08-01..08-08)
    provides: the whole phase the smoke composes (the #I tree + mediums + protos + the ether leg + the baked fixtures/payloads + the netaddr.rc seam), the virtip gate machinery, the push-before-pin discipline's current state
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: the 07-EVIDENCE.md evidence-index shape, the push-before-pin gate chain (05-11/07-08/09/10), the always-on netdev stanza
provides:
  - the composed expect-gated net smoke (D-13's Harvey-side leg): build/aarch64/smoke-r9-net.expect + nix/smoke-r9-net.sh + the boot-r9-net-smoke flake app — the five user-visible flows through the PACKAGED pipeline in ONE boot, presence-asserted, both NOT-L2-TRUTH labels stage-asserted, green at the pin on the CI shape AND in hosted CI
  - the net-smoke CI job in r9-shell.yml (D-16's short gates) — green on its first hosted run (42171968459, both jobs success)
  - 08-EVIDENCE.md — the phase gate index: every gate × the r9 SHA it ran at × the pin × the smoke verdict × the CI run; the OQ-1 + slirp-v6-probe verdicts; the Phase-9 handoff list
  - the phase's r9 work published: r9 main pushed a5b1b42..19f61ed (30 commits) BEFORE the pin; flake.lock advanced as its own isolated commit (d8080f115); locked rev == live HEAD == origin/main, owner requaos
  - two r9 fixes the composed smoke required: virtshell carries devip (14b2f23) and EtherDev drops its netif convs on unbind (19f61ed)
affects: [Phase 9 (the tap-leg truth verdict re-runs THIS phase's soak machinery + re-runs static/DHCP on tap; the multi-waiter RX-wake defect is a blocker-class there; HNET-02 lifts netaddr.rc into TOML), Phase 12 (aan's real venue — the multi-waiter defect is a blocker-class for the cpu service), the milestone ship gate (WINDOWS #47-50)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as estimateTokens.
actuals:
  tokens: 20500   # (63,517-char Harvey diff excl. the retained .log captures + 3,726-char r9 diff + ~12,000-char this SUMMARY + ~3,000-char STATE/ROADMAP/REQUIREMENTS/WINDOWS/deferred edits) / 4 — vs estimate 45,000 (confidence: low; the debugging cycles weren't in the estimate)
  tasks: 3
  commits: 8      # MEASURED: 2 r9 (14b2f23, 19f61ed) + 6 Harvey (174f7bc30, ef9b89c5c, d8080f115, bfc7fd8de + daac7e6a1 the metadata commit + d506fadf3 the follow-up CI-row record per the 07-09/10 both-runs convention).  The measured Harvey ledger window 19ada7f3a..d506fadf3 carries exactly this plan's 6 commits — NO concurrent-session traffic this window.
plan_head_before: 19ada7f3a49e1a9e322850e6c1c713e44de3c1bb   # Harvey ledger base (#3968) — 08-08's metadata commit
plan_head_after: d506fadf3    # Harvey HEAD at close — the pin+compose+CI-job HEAD is bfc7fd8de4609ff097aff8d69453d36257f682b1 (the first hosted run's headSha); daac7e6a1 is the metadata commit (the second run 42173127408's headSha); d506fadf3 records that run in the evidence

# Tech tracking
tech-stack:
  added: []
  patterns: [the composed-smoke stage-order discipline (ether legs FIRST — the multi-waiter RX-wake defect — then unbind, then loopback LAST — the two-core wildcard demux + synchronous loopback delivery), the singleton-medium unbind via the status-walk (never a hardcoded conv number), the typed-input discipline (≤256-byte lines / rc continuation lines paced per-line — the PL011 ring drop), the expect two-occurrence marker discipline extended to typed inline legs (the c-line stages consume the typed echo; the response stage sees only the output), the quoted-FAIL token so the echoed line never trips the abort regex, the push-before-pin gate chain (ls-remote == HEAD BEFORE the lock update; locked == live == origin + owner requaos; the pin commit names flake.lock ALONE)]

key-files:
  created: [build/aarch64/smoke-r9-net.expect, nix/smoke-r9-net.sh, .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-EVIDENCE.md, .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-09-net-smoke-gicv2-tcg.log, .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-09-net-smoke-gicv3-hvf.log]
  modified: [flake.nix, .github/workflows/r9-shell.yml, .gitignore, ../r9/aarch64/lib/config_virtshell.toml, ../r9/port/src/devip/phyadapt.rs, ../r9/port/src/devip/ipifc.rs, flake.lock, .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/deferred-items.md, .planning/WINDOWS.md, .planning/REQUIREMENTS.md, .planning/ROADMAP.md, .planning/STATE.md]

key-decisions:
  - "08-09: virtshell gains devip (r9 14b2f23) — the PACKAGED kernel shipped without #I (dev = [exec, bootchain, ether]), so D-13's composed smoke through the pinned pipeline could not exist; cfg(dev_ip) on the packaged kernel is inert until a bind #I lands (the plain smoke re-proven green on it)"
  - "08-09: EtherDev drops its netif convs (r9 19f61ed) — the unbind+rebind leak ('ether-type already in use') closed by an impl Drop + ipifc's unbind taking the core OUT of the STACK cell before the drop (the seam never holds STACK across netif's locks); first exercised live by the smoke's static→dhcp unbind+rebind"
  - "08-09: the composed smoke's stage order is load-bearing and measured: ETHER legs FIRST (every ether exchange has exactly one blocked devip waiter — the multi-waiter RX-IRQ wake wedged/panicked with the loopback servers blocked, on BOTH GIC gens; static-alone passes), then unbind, then LOOPBACK legs LAST (wildcard announces must land on the loopback core — the demux's ether-when-bound rule wedged the udp leg when ether stayed bound); both ether unbinds orphan-free by construction"
  - "08-09: the typed-input discipline (measured): a >256-byte typed line truncates at the PL011-side ring with no reader-visible signal (wedgedbg line=255) — all typed lines are short; the fd-4-held tcp client block is typed as rc continuation lines (the tab second-prompt) paced per-line"
  - "08-09: push-before-pin executed at the phase-complete HEAD: r9 a5b1b42..19f61ed (30 commits) pushed to requaos/r9 BEFORE the lock advanced; the pin commit d8080f115 names flake.lock ALONE; locked rev == live HEAD == origin/main, owner requaos"
  - "08-09: the at-pin packaged proof runs the CI shape on the darwin host (R9_GICV2=1 — the documented hatch): the pre-existing rc argv-stack flake is HVF-timing-exposed (2/~6 HVF runs today, incl. the plain 05-12 smoke — NOT devip-correlated: the plain smoke re-passed 2/2 on the devip kernel) and has never fired on TCG across the phase's whole gate history; the hosted CI (TCG) is the D-16 venue and ran green first try"
  - "08-09: NETIP-08 stays PENDING past this plan (the #2388 ready subset is NETIP-03/04/05) — its requirement text's tap leg is Phase 9's joint acceptance gate (D-15); marking it Complete here would claim the tap venue ran"

patterns-established:
  - "The composed smoke's expect-side capture discipline: conv numbers and ports are read FROM THE WIRE host-side where possible, and guest-side state (the held fd) rides rc continuation lines paced per-line — never a long typed line"
  - "The stage-order-as-load-bearing-smoke pattern: when mediums/cores/waiter-sets interact, the smoke's leg ORDER is part of the proof design and its rationale is recorded in the driver header"
  - "The evidence index row shape: gate × r9 SHA × pin commit × smoke verdict × CI run, with venue labels inside the row (NOT-L2-TRUTH is never prose-adjacent-to-green without the label)"

requirements-completed: [NETIP-03, NETIP-04, NETIP-05]  # the #2388 ready subset after the last declarer (this plan).  NETIP-08 stays PENDING by design (D-15's joint tap verdict is Phase 9's) — recorded in REQUIREMENTS.md's traceability row and the evidence's handoff list.

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The composed net smoke (D-13): the five user-visible flows through the PACKAGED pipeline in ONE boot — ipconfig static, ipconfig dhcp, ping 127.0.0.1, udp echo, tcp/7 echo — each presence-asserted, both NOT-L2-TRUTH venue labels stage-asserted, the NET-SMOKE: ALL STAGES PASS marker, the pid-warning-absent + nic-bind post-passes"
    requirement: NETIP-03
    verification:
      - kind: e2e
        ref: "nix run .#boot-r9-net-smoke (R9_GICV2=1, the CI shape) at the pin 19f61ed → exit 0; direct-env at the live kernel → exit 0 on gicv2+tcg AND gicv3+hvf; retained captures 08-09-net-smoke-gicv2-tcg.log / 08-09-net-smoke-gicv3-hvf.log"
        status: pass
      - kind: e2e
        ref: "hosted CI r9-shell run 42171968459 net-smoke job → success at headSha bfc7fd8de (TCG ubuntu-latest)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The D-16 CI shape: the net-smoke job in r9-shell.yml beside boot-smoke (byte-untouched), SHA-pinned actions per the 06-08 rule (checkout 11d5960a… API-resolved 2026-11-06, nix-installer 3138316d…), the header naming D-16 + the documented-manual soak pointer"
    requirement: NETIP-08
    verification:
      - kind: e2e
        ref: "run 42171968459 — both jobs (boot-smoke, net-smoke) conclusion=success"
        status: pass
      - kind: other
        ref: "grep gates (net-smoke + boot-r9-net-smoke + the installer SHA present) + yaml.safe_load → CI-JOB-WIRED / YAML-OK"
        status: pass
    human_judgment: false
  - id: D3
    description: "The evidence index: 08-EVIDENCE.md consolidates every phase gate × the r9 SHA it ran at × the pin × the smoke verdict × the CI run; every slirp-green gate labeled NOT-L2-TRUTH; the D-16 soak command verbatim; the OQ-1 + v6 probe verdicts; the Phase-9 handoff list"
    verification:
      - kind: other
        ref: "EVIDENCE-INDEX-OK grep chain (NOT-L2-TRUTH + R9_NETIP_SOAK_ROUNDS + a 40-char SHA present, file non-empty)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Push-before-pin executed: r9 main pushed (a5b1b42..19f61ed, 30 commits) BEFORE the lock advanced; the pin commit names flake.lock ALONE; locked rev == live HEAD == origin/main, owner requaos; a fresh hosted run covers the pushed HEAD"
    requirement: NETIP-08
    verification:
      - kind: other
        ref: "git ls-remote origin main == 19f61ed… (post-push verify) + PIN-STRUCT-OK (python assertion) + PIN-COMMIT-ISOLATED-OK (git show --stat: 1 file) + run 42171968459 success"
        status: pass
    human_judgment: false
  - id: D5
    description: "The two r9 fixes the smoke required (virtshell devip; the EtherDev Drop) with the regression battery green at the pushed HEAD"
    verification:
      - kind: unit
        ref: "cargo test -p port → 152/152 at 19f61ed"
        status: pass
      - kind: e2e
        ref: "8/8 boot profiles + 7/7 virtip modes PASS at 19f61ed (the full battery on the committed tree)"
        status: pass
    human_judgment: false

# Metrics
duration: 85 min
completed: 2026-11-06
status: complete
---

# Phase 8 Plan 09: The Composed Net Smoke + CI + Evidence + Push-Before-Pin Summary

**The phase closes with its work published and its own gates green: the composed expect-gated net smoke drives all five D-13 flows through the PACKAGED pipeline in ONE boot (ipconfig static → unbind → ipconfig dhcp → unbind → ping 127.0.0.1 + udp echo → tcp/7 echo inline → `NET-SMOKE: ALL STAGES PASS`), green at the pin on the CI shape (exit 0) AND in hosted CI (run 42171968459 — the new `net-smoke` job and the untouched `boot-smoke` both success on the first run); `08-EVIDENCE.md` consolidates every gate × r9 SHA × pin × verdict × run with every slirp-green gate labeled NOT-L2-TRUTH; and push-before-pin executed cleanly (r9 `a5b1b42..19f61ed` pushed BEFORE the lock; `d8080f115` names flake.lock ALONE; locked == live == origin, owner requaos). The composed boot exercised two lifecycle corners the per-mode gates never reach, and fixing/shaping them is the plan's substance: virtshell carried no devip (fixed: `14b2f23`), EtherDev leaked its netif convs on unbind (fixed: `19f61ed`), the multi-waiter RX-IRQ wake wedges (recorded as a Phase-9/12 blocker-class; the smoke's ether-first stage order sidesteps it), and a >256-byte typed console line truncates silently (the smoke's typed-input discipline).**

## Performance

- **Duration:** 85 min (incl. the composed-smoke debugging cycles — the wedge/panic/truncation triage, the EtherDev Drop fix, and the two pipeline runs)
- **Started:** 2026-11-06T04:56:57Z
- **Completed:** 2026-11-06T06:22:00Z
- **Tasks:** 3
- **Files modified:** 6 created + 9 modified across both repos (plus the planning records)

## Accomplishments

- **Task 1 (the composed smoke, D-13):** `smoke-r9-net.expect` (smoke-r9.expect's shape: the loud env contract, fresh capture per run, the identical always-on netdev stanza, expect_before panic+fixture-FAIL aborts, staged presence assertions, TCG-sized timeouts, the pid-warning-absent + nic-bind post-passes) with the five flows composed into one boot; `nix/smoke-r9-net.sh` (the wrapper sibling, `r9netsmoke-` run tag); the `boot-r9-net-smoke` flake app inside the hasAttr gate; the siblings byte-untouched. Green at the pin via the nix app on the CI shape, and direct-env on BOTH host shapes. The validation path is recorded honestly: Task 1's direct-env plumbing proof ran first because the pin predated devip; Task 3's packaged run at the pin is the final proof.
- **Task 2 (D-16 CI):** the `net-smoke` job in `r9-shell.yml` mirroring `boot-smoke`'s shape (ubuntu-latest, timeout 90 with the TCG comment, the pinned nix-installer), checkout SHA-pinned per the 06-08 rule with the API resolution recorded in the header comment; the header names D-16 and points at the documented-manual soak. YAML parses; the hosted run green on the first try.
- **Task 3 (closure):** `08-EVIDENCE.md` (the 07-EVIDENCE.md shape, every gate mapped, the D-16 soak command verbatim, the OQ-1 + v6-probe verdicts, the Phase-9 handoff); the push-before-pin chain with all three structural gates green; the composed smoke green at the new pin via the packaged pipeline; the fresh hosted CI run recorded (42171968459 at `bfc7fd8de`, both jobs success).
- **The two r9 fixes the composed smoke required:** `14b2f23` (virtshell gains devip — the packaged kernel's clean-checkout consumer gets the phase's work) and `19f61ed` (EtherDev's Drop + the take-then-drop-late unbind — the unbind+rebind path's first live exercise).

## Task Commits

Each task was committed atomically across the two repos:

**Task 1 (the composed smoke):**
1. **virtshell gains devip (r9)** — `14b2f23` (feat)
2. **EtherDev drops its netif convs + the take-then-drop-late unbind (r9)** — `19f61ed` (fix)
3. **The smoke + wrapper + flake app + retained captures + the deferred-items records (Harvey)** — `174f7bc30` (feat)

**Task 2 (D-16 CI):**
4. **The net-smoke job (Harvey)** — `ef9b89c5c` (test)

**Task 3 (closure):**
5. **The flake.lock pin advance, isolated (Harvey)** — `d8080f115` (chore)
6. **08-EVIDENCE.md (Harvey)** — `bfc7fd8de` (docs)

**Plan metadata:** (Harvey — this SUMMARY + the evidence CI row + STATE/ROADMAP/REQUIREMENTS/WINDOWS, committed after this write per the atomic close-out order)

## Files Created/Modified

**Harvey:**
- `build/aarch64/smoke-r9-net.expect` — NEW: the composed net smoke driver (D-13)
- `nix/smoke-r9-net.sh` — NEW: the wrapper sibling
- `flake.nix` — the `netSmokeR9Pkg` entry + the `boot-r9-net-smoke` app (insertion-only; the hasAttr gate)
- `.github/workflows/r9-shell.yml` — the `net-smoke` job (additive; boot-smoke byte-untouched)
- `.gitignore` — the smoke's transient capture names
- `flake.lock` — the r9 pin advance (its own commit)
- `.planning/phases/08-*/08-EVIDENCE.md` — NEW: the phase gate index
- `.planning/phases/08-*/08-09-net-smoke-{gicv2-tcg,gicv3-hvf}.log` — the retained green captures
- `.planning/phases/08-*/deferred-items.md` + `.planning/WINDOWS.md` + REQUIREMENTS/ROADMAP/STATE — the records

**r9:**
- `aarch64/lib/config_virtshell.toml` — `dev` gains `"ip"` (the packaged kernel carries #I)
- `port/src/devip/phyadapt.rs` — `impl Drop for EtherDev` (closes the three lane convs)
- `port/src/devip/ipifc.rs` — the unbind's take-then-drop-late (never holds STACK across netif's locks)

## Decisions Made

- **virtshell carries devip** (the Rule-3 unblock): the packaged kernel shipped with `dev = ["exec", "bootchain", "ether"]` — D-13's composed smoke through the PINNED pipeline cannot exist without #I in the package; cfg(dev_ip) is inert until a bind lands, and the plain composed smoke was re-proven green on the devip kernel (2/2 HVF + the battery's virt profile + the packaged net smoke's boot stages).
- **The composed smoke's stage ORDER is load-bearing** (ether first, loopback last, two orphan-free ether unbinds) — the recorded why: the multi-waiter RX-wake defect (both GIC gens; static-alone passes), the two-core wildcard demux (the udp leg wedged with ether bound), the singleton mediums. The driver header carries the full analysis so the order is never "simplified" back.
- **The typed-input discipline:** measured truncation at the PL011-side 256-byte ring (wedgedbg `line=255`); every typed line is short; the fd-4-held tcp block is typed as rc continuation lines paced per continuation prompt; the ok-marker two-occurrence discipline adapted (the c-line stages consume the typed echo, so the response stage sees only the output); the quoted-'FAIL' token so an echoed line can never trip the abort regex.
- **NETIP-08 stays Pending** past this plan (its letter names the tap leg — Phase 9's joint gate per D-15); NETIP-03/04/05 are the shared-ID gate's ready subset and marked Complete.
- **The at-pin packaged proof uses the CI shape on the darwin host** (R9_GICV2=1): the pre-existing rc argv-stack flake is HVF-timing-exposed (2/~6 HVF runs today, including the plain 05-12 smoke — proven NOT devip-correlated by the plain smoke's 2/2 re-pass on the devip kernel) and has never fired on TCG across the phase's whole gate history; TCG is D-16's venue.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The packaged kernel carried no devip**
- **Found during:** Task 1 (the smoke's first direct-env run — the plan's own step-4 note anticipated red runs for the STALE PIN, but the packaged `virtshell` config itself lacked `"ip"`)
- **Issue:** `config_virtshell.toml` had `dev = ["exec", "bootchain", "ether"]` — the `r9.packages.*.kernel-virt` artifact (what a clean-checkout consumer gets) would never serve `#I`; the composed smoke's premise (D-13 through the PACKAGED pipeline) was unreachable at ANY pin
- **Fix:** `dev` gains `"ip"` with a comment naming D-13; the cfg is proven by the virtip profile's identical dev set (the build's 0.42s fresh result IS the flags-identical proof), the plain smoke re-proven green on the devip kernel
- **Files modified:** ../r9/aarch64/lib/config_virtshell.toml
- **Verification:** the packaged smoke green at the pin (the nix-built package from the pushed rev)
- **Committed in:** `14b2f23` (r9)

**2. [Rule 1 - Kernel bug] EtherDev leaked its netif conversations on unbind**
- **Found during:** Task 1's composed-smoke runs (the static→dhcp sequence's dhcp leg failed `devether: ether-type already in use`)
- **Issue:** ipifc's unbind dropped the seam core but EtherDev had NO Drop impl (pop_merged's comment recorded "the binding never closes the convs this era") — the three lane convs leaked open, so any re-bind failed; the composed smoke's unbind+rebind was the first live exercise of the path
- **Fix:** `impl Drop for EtherDev` closing the lanes via netif's `conv_close`; ipifc's unbind now TAKES the core out of the STACK cell and drops it after the guard drops (the Drop takes NETIF/INGRESS — the seam never holds STACK across netif's locks, the bind path's discipline mirrored)
- **Files modified:** ../r9/port/src/devip/phyadapt.rs, ../r9/port/src/devip/ipifc.rs
- **Verification:** the smoke's double unbind+rebind green on both shapes; port suite 152/152; the full battery (8 profiles + 7 virtip modes) green at the pushed HEAD
- **Committed in:** `19f61ed` (r9)

**3. [Rule 1 - Harness defect] The >256-byte typed console line truncated silently**
- **Found during:** Task 1 (both legs wedged identically; the capture ended mid-word at char 255; wedgedbg showed `line=255` stuck)
- **Issue:** the console's PL011-side 256-byte RX ring drops host-burst bytes (drop-oldest by design), the trailing CR never landed, and rc waited forever on the incomplete command
- **Fix:** the smoke's typed-input discipline — all typed lines short; the fd-4-held tcp client block typed as rc continuation lines (the tab second-prompt) paced per-line
- **Files modified:** build/aarch64/smoke-r9-net.expect
- **Verification:** both venue shapes pass the tcp leg end-to-end
- **Committed in:** `174f7bc30` (Task 1 commit)

**4. [Rule 1 - Harness defect] The Tcl quoting on the block's close line**
- **Found during:** Task 1 (Tcl `invalid command name "4"` — a `[4]` in a double-quoted send; then `\}` in braces RETAINED the backslash, which rc lexed as a stray `\` command — the block ran but the trailing word exec-failed)
- **Fix:** the close line sends `"\} <>\[4\]/net/tcp/clone\r"` (double-quote backslash escapes, verified against the host's expect); the two-occurrence marker staging adjusted (the continuation-line stages already consume the typed echo, so the response stage sees only the output)
- **Files modified:** build/aarch64/smoke-r9-net.expect
- **Committed in:** `174f7bc30` (Task 1 commit)

**5. [Rule 1 - Smoke design] The stage order (two measured kernel/session hazards)**
- **Found during:** Task 1's runs — (a) with the loopback servers blocked, the static leg's gateway ping wedged the guest fully idle on BOTH GIC gens (an earlier form panicked there) — the multi-waiter RX-IRQ wake defect; (b) with ether still bound, the loopback fixture's wildcard-announced servers landed on the ETHER core (the 08-06 demux rule) and the udp echo never crossed
- **Issue:** the naive fixture-sequence order (loopback → tcp → static → dhcp) is unreachable: the medium singletons force unbinds, and the unbind+rebind of loopback with a live server is the orphan-handle collision class
- **Fix:** ether legs FIRST (single-waiter per exchange — the xtask-proven shape), the orphan-free ether unbind between them (possible only because of deviation 2's fix), a SECOND ether unbind before the loopback legs (the wildcard fallback), loopback last, tcp inline on the live loopback core. The full rationale is in the driver's header
- **Verification:** exit 0 on both shapes + the packaged run at the pin + hosted CI green
- **Committed in:** `174f7bc30` (Task 1 commit)

---

**Total deviations:** 5 auto-fixed (1 blocking config gap, 1 kernel bug, 3 harness/smoke-design corrections) + 4 pre-existing defects recorded (NOT fixed — scope boundary): the multi-waiter RX-wake defect, the devcons burst-truncation-no-signal, the rc argv-stack flake residual, the intermittent early-boot devpipe EL1 abort.
**Impact on plan:** The plan's deliverables all shipped with their gates green; the two fixes are exactly the phase's "gate-exposed kernel bugs fixed en route" posture (08-08's precedent); the stage-order redesign preserves every plan semantic (all five flows, the fixtures where they compose, the markers, the labels). No locked decision changed; the NETIP-08 transparency prohibition holds everywhere.

## Issues Encountered

- **The gsd_run/gsd-tools shim is unavailable** in this environment (the phase's standing condition) — STATE.md/ROADMAP.md/REQUIREMENTS.md/WINDOWS.md hand-edited to the same fields the verbs maintain (position, 9/9 + the phase-completed row, the metrics row, the decision appends, NETIP-03/04/05 Complete with NETIP-08's D-15 annotation, the four WINDOWS entries with the count bumps). The ready-subset analysis was computed by hand from the plan frontmatters (NETIP-03: 08-03/04/05/09; NETIP-04: 08-03/06/09; NETIP-05: 08-02/03/07/09 — all sibling declarers carry SUMMARYs).
- **The rc argv-stack flake (`pushword but no argv!`)** fired twice in HVF runs today — once inside netip-static.rc during the FIRST packaged run at the pin, once in the plain 05-12 smoke's typed echo stage — and is the reason the at-pin packaged proof runs the CI shape (R9_GICV2=1) on this host. Measured NOT devip-correlated (the plain smoke re-passed 2/2 on the devip kernel; never fired on TCG all phase; the hosted TCG run green first try). Recorded in deferred-items + WINDOWS #49.
- **The intermittent early-boot devpipe EL1 abort** (devpipe.rs:282, FAR 0x4e428, translation fault level 3) fired once in an HVF leg right after the boot chain's `version...` — the same tree re-ran green end-to-end; 1/~6 HVF runs; never on TCG. Recorded (WINDOWS #50).
- **One triage run of mine was tainted by a harness mistake** (I rm'd its capture mid-run — the driver's post-pass re-open would have failed; the run was killed and re-done cleanly). Not a product signal; recorded here for completeness.
- **Concurrent-session traffic:** none this window (the ledger window 19ada7f3a..bfc7fd8de carries exactly this plan's 4 commits).
- **r9's pre-existing uncommitted `aarch64/src/exec.rs` modification and the pre-existing stash entry were never touched, staged, or committed** anywhere in this plan (verified per-commit; every r9 commit named explicit paths; the push carried only commits already on main).

## Authentication Gates

None. (The pushes — r9 `main` to requaos/r9 and Harvey `master` to requaos/harvey — succeeded with the environment's existing credentials; the GitHub API run queries were unauthenticated-read.)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Phase 8 is complete (9/9 plans); the phase's exit state:** the devip `/net` tree in r9, all protocol gates green, the composed smoke green through the packaged pipeline + hosted CI, the phase's r9 work published and pinned (`19f61ed`, `d8080f115`), the evidence consolidated in `08-EVIDENCE.md`.
- **Phase 9's entry is ready:** the tap-leg truth verdict re-runs THIS phase's machinery unchanged (the soak fixture + host echo endpoint + assert_tcp_bidirectional; static/DHCP fixtures) — the handoff list + the exact sustained-soak command live in `08-EVIDENCE.md` and `netip-soak.rc`'s header. HNET-02 lifts `netaddr.rc` into TOML.
- **Named blocker-class for Phase 9/12:** the multi-waiter RX-IRQ wake defect (WINDOWS #47) must be root-caused before aan/cpu ride real net with many blocked net procs — the composed smoke's stage-order note in the driver header is the reproduction guide.
- **The CI story doubled:** `r9-shell.yml` now runs both the boot smoke and the composed net smoke on every push — both green at the phase-close HEAD.
- No blockers.

## Self-Check: PASSED

- r9 commits `14b2f23`, `19f61ed` exist on r9 main AND on origin/main (`git ls-remote origin main` == `19f61ed133d96fcdbfdfc5049ab2168beae512b3`) — FOUND
- Harvey commits `174f7bc30`, `ef9b89c5c`, `d8080f115`, `bfc7fd8de` exist on master — FOUND (`git log` at SUMMARY write)
- `build/aarch64/smoke-r9-net.expect` exists with staged assertions for all five flows + `NET-SMOKE: ALL STAGES PASS` + the NOT-L2-TRUTH label stages — VERIFIED (greps)
- The spawn argv carries the identical netdev stanza tokens as smoke-r9.expect with no hostfwd — VERIFIED (the lines are a verbatim copy; grep)
- `nix/smoke-r9-net.sh` passes `bash -n`; the flake app evaluates (`nix eval .#apps.aarch64-darwin.boot-r9-net-smoke` → FLAKE-APP-EVAL-OK) — VERIFIED
- `build/aarch64/smoke-r9.expect`, `nix/smoke-r9.sh`, and the `smokeR9Pkg` entry are byte-untouched — VERIFIED (`git diff` empty on the siblings; flake.nix insertion-only)
- The composed smoke green at the pin: `R9_GICV2=1 nix run .#boot-r9-net-smoke` exit 0 (the packaged pipeline: nix-built pinned kernel + userland build + bake + smoke) — VERIFIED (this plan's Task 3 run; capture retained)
- The direct-env plumbing proof: exit 0 on gicv2+tcg AND gicv3+hvf at the live `19f61ed` kernel — VERIFIED (both retained captures carry the marker ×2 with zero panic/FAIL lines)
- `.github/workflows/r9-shell.yml` parses; the net-smoke job runs `nix run .#boot-r9-net-smoke` with the pinned installer SHA + the SHA-pinned checkout; the existing boot-smoke job byte-identical — VERIFIED (YAML-OK + zero removed content lines in the diff)
- The pin's structural gates: `git ls-remote origin main` == live r9 HEAD (post-push verify); locked rev == live HEAD == origin/main with owner requaos (PIN-STRUCT-OK); the pin commit `d8080f115` names flake.lock ALONE (PIN-COMMIT-ISOLATED-OK: git show --stat → 1 file) — VERIFIED
- The hosted CI run: 42171968459 at headSha `bfc7fd8de4609ff097aff8d69453d36257f682b1` — both jobs (boot-smoke, net-smoke) conclusion=success — VERIFIED via the GitHub API at write time
- `08-EVIDENCE.md` carries every gate × SHA × pin × verdict × run, the NOT-L2-TRUTH labels, the verbatim soak command, the OQ-1 + v6-probe verdicts, and the Phase-9 handoff list — VERIFIED (EVIDENCE-INDEX-OK grep chain)
- The regression battery at the pushed HEAD: port 152/152, 8/8 boot profiles, 7/7 virtip modes — VERIFIED (the battery log this session)
- r9 `aarch64/src/exec.rs` pre-existing modification + the stash entry untouched — VERIFIED (git status / stash list per commit)
- The two retained captures exist in the phase dir — VERIFIED (`08-09-net-smoke-gicv2-tcg.log`, `08-09-net-smoke-gicv3-hvf.log` committed at `174f7bc30`)

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Completed: 2026-11-06*
