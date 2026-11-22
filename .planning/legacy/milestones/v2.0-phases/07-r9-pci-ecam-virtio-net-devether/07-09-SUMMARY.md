---
phase: 07-r9-pci-ecam-virtio-net-devether
plan: 09
subsystem: infra
tags: [r9, devether, netif, readnum, donor-truncation, u32, conv-close, last-close-reset, g-07-1, g-07-2, cr-01, tdd, host-tests, push-before-pin, flake-lock, pin, virtnet, evidence-index, hosted-ci, r9-shell, netdrv-03]

# Dependency graph
requires:
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: 07-07 (the devether/netif implementation the pins exercise) + 07-08 (the push-before-pin mechanics, the PIN-OK gate shape incl. its -A8-window and .program-eval corrections, the evidence-index format)
  - phase: debug sessions (.planning/debug/connect-neg1-type-panics.md, .planning/debug/data-read-never-wakes.md)
    provides: the binding root causes this plan implements exactly — the i64/u64 64-bit reinterpretation defect (G-07-2) and the test-procedure defect + donor-faithful last-close reset (G-07-1)
provides:
  - r9 host pins (crate-root port/src/devether_tests.rs): readnum_content(-1)==b" 4294967295 " byte-exact (the G-07-2 oracle) + the 0/65535 neighbors + the conv_close last-close reset pinned as INTENTIONAL donor behavior (G-07-1)
  - the GREEN fix: readnum_content's `val as u32 as u64` (the donor's int→ulong 32-bit truncation, devcons.c:712 × netif.h:40) + the corrected donor-shape doc comment with the class-sweep conclusion
  - 07-UAT.md test 1 corrected to the read-first-then-connect protocol (the verify-work re-run's procedure)
  - 07-REVIEW-DISPOSITION.md: CR-01 fixed (YAML + table), open counter hand-updated 14 → 13 (WR-05/WR-06 untouched)
  - 07-EVIDENCE.md: the 07-09 re-proof section (both virtnet gates at the fixed SHA) + the new Pinned-commits rows
  - the composed-boot venue carrying the fix: r9 46a2ed2..1c16193 pushed to requaos/r9 main BEFORE the pin; flake.lock locked rev == pushed HEAD as its own commit d5df2df38 (owner requaos); fresh hosted r9-shell CI green at the pushed Harvey HEAD
affects: [verify-work (the 07-UAT tests 1-2 guest re-run against the new pin — the orchestrator-owned next step), phase-8 (devip/arp/ethermedium consumes the pinned devether semantics), milestone-close]

# Actuals (#2632) — pairs with the plan's estimate (tokens 52000, confidence: low).
actuals:
  tokens: 4165     # chars/4 over the realized diffs — r9 46a2ed2..1c16193 (9432 chars ≈ 2358) + harvey 1e985bca0..eef588612 (7228 chars ≈ 1807)
  tasks: 4
  commits: 5       # MEASURED: r9 rev-list 46a2ed2..1c16193 = 2 (RED+GREEN); harvey rev-list ledger-base..eef588612 = 3 (docs, pin, evidence). The raw harvey ledger range to today's HEAD counts 4 — one interleaved Phase-6 commit (69c4cf689) from the concurrent session shares the range (shared working tree; recorded honestly, none of its files touched)

plan_head_before: 1e985bca07d03c8b67097f16d48b16b243093644   # harvey repo (ledger #3968); r9 ledger base: 46a2ed206069ec23f39e02e5851dada12c201f8c
plan_head_after: eef588612c26830132b0db34fb34f6f5f7b0ba0d    # harvey repo at the evidence commit (the SUMMARY/metadata commit rides on top of the interleaved 69c4cf689); r9 HEAD 1c161931000e77edb546a972f33b1adfa89b0b30 == origin/main

# Tech tracking
tech-stack:
  added: []   # zero new dependencies anywhere (T-07-SC vacuous — no Cargo.toml/Cargo.lock/package touch; the pin consumes the existing github input mechanism)
  patterns:
    - "TDD on a kernel defect with the panic AS the red: the target test's live subtract-with-overflow panic is the intentional RED (RED_EVIDENCE_OK via check tdd-red-evidence); the GREEN is one expression (val as u32 as u64), no clamp, no guard — the u32 ceiling (10 digits < NUMSIZE-1=11) makes the underflow unreachable by construction"
    - "behavioral pinning of module-private state: the conv_close reset pin observes drain+close through pub/pub(crate) surfaces only (NETIF chain for ConvBook; pop_frame's Pop::Closed for the INGRESS-side drain+open==false; a stub EtherHooks wake log for the EOF wake) — never a field read of ConvSlot/FrameStore/IngressState"
    - "push-before-pin re-applied at a fix HEAD (T-07-20): r9 RED+GREEN pushed first (ls-remote == gated HEAD), the flake.lock advance second as its own isolated commit, locked rev == live HEAD == origin/main + owner requaos, fresh hosted CI at the pushed Harvey HEAD"

key-files:
  created:
    - r9:port/src/devether_tests.rs (crate-root pins: readnum_content byte-exact ×3 + conv_close last-close reset)
  modified:
    - r9:port/src/devether.rs (readnum_content: pub(crate) + `val as u32 as u64` + the corrected donor-shape doc comment)
    - r9:port/src/lib.rs (`#[cfg(test)] mod devether_tests;` registration after virtio_tests)
    - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-UAT.md (test-1 expected block: read-first-then-connect protocol + the open-lifetime rationale; results untouched)
    - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-REVIEW-DISPOSITION.md (CR-01 fixed in both places; open: 14 → 13; WR-05/WR-06 byte-untouched)
    - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-EVIDENCE.md (the 07-09 re-proof section + the two new Pinned-commits rows)
    - flake.lock (r9 pin 46a2ed2 → 1c16193 — own commit d5df2df38)

key-decisions:
  - "07-09: the fix is exactly the diagnosis — `let v = val as u32 as u64;` reproduces the C conversion int→ulong (32-bit truncation, never sign extension; Plan 9 long/ulong is 32-bit even on amd64 per amd64/include/u.h, vlong/uvlong are the 64-bit types hence VLNUMSIZE=22); NO saturating clamp and NO underflow guard (the diagnosis's explicit prohibition — a guard would mask a future width regression)"
  - "07-09: conv_close's last-close reset (typ=0, owner cleared, queue drain, EOF wake — donor 9k netif.c:446) is pinned as INTENTIONAL by a host test rather than 'repaired' — G-07-1 was a test-PROCEDURE defect (connect before holding the conversation open), the kernel is donor-faithful; the corrected protocol lives in 07-UAT.md for the verify-work re-run"
  - "07-09: push-before-pin executed at the fix HEAD — r9 46a2ed2..1c16193 (2 commits) pushed to requaos/r9 BEFORE the flake.lock advance; the pin commit d5df2df38 names flake.lock ALONE; locked rev == live HEAD == origin/main, owner requaos; hosted r9-shell run 42043430592 SUCCESS at headSha eef588612 (and run 42043437746 SUCCESS at the subsequent master tip 69c4cf689 which carries these commits)"

patterns-established:
  - "guest-visible defect → host pin → one-expression donor-faithful fix → gate re-proof at the fixed SHA → push-before-pin to the composed venue: the gap-closure shape for r9 kernel behavior defects"
  - "the evidence index's re-proof row (a gate × the new SHA it re-ran at, captures unchanged) as distinct from an evidence refresh (new captures)"

requirements-completed: [NETDRV-03]  # copied verbatim from plan frontmatter; all sibling declarers (07-01/04/05/07/08) have SUMMARYs — the ready-ids gate releases it

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "readnum_content reproduces the donor's 32-bit truncation byte-exactly: connect -1 reads back as ` 4294967295 ` (12 bytes), never a usize underflow, never sign extension, never a clamp; the 0 and 65535 neighbors pinned; the doc comment states the true donor shape + the class-sweep conclusion"
    requirement: NETDRV-03
    verification:
      - kind: unit
        ref: "r9 `nix develop --command cargo test -p port`: devether_tests::readnum_content_neg1_matches_donor_truncation / readnum_content_zero_pads_numsize / readnum_content_max_ethertype_65535 all ok; port suite 51/51 (+4 vs the 47 pre-plan baseline); `cargo xtask test --verbose` green"
        status: pass
    human_judgment: false
  - id: D2
    description: "conv_close's last-close reset (typ=0, owner_set=false, inuse=0, queue drain, EOF wake) pinned as intentional donor behavior (G-07-1) via the alloc-first drive publish → conv_alloc → conv_connect → rx_ingress → pop-Frame probe → rx_ingress → register_rx_waiter → conv_close"
    requirement: NETDRV-03
    verification:
      - kind: unit
        ref: "r9 cargo test: devether_tests::conv_close_last_close_reset_is_donor_intentional ok (green TODAY — it pins existing behavior; RED evidence record RED_EVIDENCE_OK for the -1 pin, the panic-as-red)"
        status: pass
    human_judgment: false
  - id: D3
    description: "07-UAT.md test-1 protocol corrected to blocked-read-FIRST-then-connect with the open-lifetime rationale; 07-REVIEW-DISPOSITION.md CR-01 fixed with open: 13 (total 14, WR-05/WR-06 untouched); 07-EVIDENCE.md indexes the re-proof SHA"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "BOOKKEEPING-DONE chain: grep -qF 'cat /net/ether0/$conv/data &' 07-UAT.md + 'disposition: fixed' + '^open: 13$' + 're-proof' 07-EVIDENCE.md"
        status: pass
    human_judgment: false
  - id: D4
    description: "Both virtnet gates (arp + R9_NET_SWEEP_MODE=blast) green on the fixed r9 HEAD 1c161931000e77edb546a972f33b1adfa89b0b30 with no intermediate commits — the data path re-proven after the devether behavior change"
    requirement: NETDRV-03
    verification:
      - kind: e2e
        ref: "R9_BOOT_TEST_TIMEOUT=600 cargo xtask boot-test --arch aarch64 --config virtnet PASS (arp N=4) and R9_NET_SWEEP_MODE=blast … PASS (blast N=256, NET-DONE) — both at 1c16193"
        status: pass
    human_judgment: false
  - id: D5
    description: "The composed-boot venue carries the fix: r9 RED+GREEN pushed to requaos/r9 main (ls-remote == 1c16193), flake.lock pinned to the pushed HEAD as its own isolated commit (owner requaos, PIN-OK chain + structural assert, both boot apps eval), fresh hosted r9-shell CI green at the pushed Harvey HEAD"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "R9-PUSHED (remote==HEAD; plan-scope tree clean) + PIN-OK (-A8 window grep + python3 structural assert + diff-tree names flake.lock alone) + CI-GREEN-AT-PIN (run 42043430592 success at headSha eef588612c26; run 42043437746 success at 69c4cf689a54 carrying these commits)"
        status: pass
    human_judgment: false

# Metrics
duration: 18 min
completed: 2026-11-05
status: complete
---

# Phase 07 Plan 09: gap closure — readnum_content donor-truncation fix (G-07-2) + conv_close reset pin and UAT test-1 protocol correction (G-07-1) Summary

**`connect -1` can never again panic the kernel: readnum_content now truncates through u32 exactly as the donor's int→ulong conversion does (devcons.c:712 × netif.h:40 — Plan 9 long is 32-bit even on amd64), producing the byte-exact ` 4294967295 ` field proven by three crate-root host pins, while conv_close's last-close reset is pinned as INTENTIONAL donor behavior and 07-UAT.md's test-1 protocol is corrected to read-first-then-connect (the defect was the procedure, never the kernel); CR-01 is marked fixed (open: 14 → 13), both virtnet gates re-proved green at the fixed HEAD `1c16193`, and the fix is carried to the composed-boot venue via push-before-pin — r9 `46a2ed2..1c16193` on requaos/r9 main, flake.lock advanced as its own commit `d5df2df38` (owner requaos), fresh hosted r9-shell CI SUCCESS at the pushed Harvey HEAD.**

## Performance

- **Duration:** 18 min
- **Started:** 2026-11-05T14:09:39Z
- **Completed:** 2026-11-05T14:28:00Z
- **Tasks:** 4
- **Files modified:** 7 (r9: devether.rs, devether_tests.rs NEW, lib.rs; harvey: 07-UAT.md, 07-REVIEW-DISPOSITION.md, 07-EVIDENCE.md, flake.lock)

## Accomplishments

- **Task 1 (RED) — the pins:** `port/src/devether_tests.rs` created at the crate root (the allocator_tests/virtio_tests precedent) with the four exactly-named tests; readnum_content widened to pub(crate); module registered in lib.rs. The RED captured live: `readnum_content_neg1_matches_donor_truncation` FAILED with `attempt to subtract with overflow` at devether.rs:493:15 (the panic IS the red — the 64-bit reinterpretation underflows `NUMSIZE-1-20`); the two neighbors + the conv_close pin green. RED evidence record persisted and machine-verified: `check tdd-red-evidence` → **RED_EVIDENCE_OK** (target test failed on its behavior assertion path).
- **Task 2 (GREEN) — the one-expression fix:** `let v = val as u32 as u64;` — the donor's exact conversion width, no clamp, no guard (the diagnosis's explicit prohibition honored); the doc comment rewritten to the true 32-bit-truncation donor shape with the class-sweep conclusion recorded in its tail. Full port suite 51/51 (+4 exactly), `cargo xtask test --verbose` green — zero regressions.
- **Task 3 — bookkeeping + gate re-proof:** 07-UAT.md test 1 now drives `cat /net/ether0/$conv/data &` FIRST (holds the conversation open, inuse=1), THEN `echo connect 2054 > ctl` — the ctl close is no longer a last close so typ=2054 sticks; the open-lifetime rationale sentence added (donor netif.c:446, pinned intentional by the 07-09 host test); result/reported/severity and test 2 untouched. CR-01 → fixed in the YAML block AND the summary table (source cites 07-09); `open: 14 → 13` hand-updated, `total: 14` unchanged; WR-05/WR-06 byte-untouched. Both virtnet gates re-run green at `1c16193` with no intermediate commits (arp N=4 PASS; blast N=256 PASS); 07-EVIDENCE.md gained the 07-09 re-proof section (no capture refresh — counters/pcaps unchanged by this fix).
- **Task 4 — push-before-pin to the composed venue:** r9 RED+GREEN pushed (`46a2ed2..1c16193 main -> main`; ls-remote == the gated HEAD); flake.lock advanced to `1c161931000e77edb546a972f33b1adfa89b0b30` as its OWN commit `d5df2df38` (the diff-tree isolation gate: flake.lock alone; the -A8-window grep + python3 structural assert green; both boot apps eval against the new pin); the Pinned-commits table gained the push/pin rows (`eef588612`); Harvey master pushed and the FRESH hosted r9-shell run **42043430592 concluded SUCCESS** at headSha `eef588612c26…` — and the subsequent tip run 42043437746 (at the concurrent session's 69c4cf689, which carries these commits) also SUCCESS.

## Task Commits

Each task was committed atomically per the dual-repo contract:

**r9 repo (branch main, pushed to requaos/r9):**
1. **Task 1 (RED): pin readnum_content donor truncation + conv_close last-close reset** — `1e77266` (test) — port/src/devether_tests.rs (new), port/src/devether.rs (visibility), port/src/lib.rs (registration)
2. **Task 2 (GREEN): truncate readnum_content through u32 per donor readnum** — `1c16193` (feat) — port/src/devether.rs

**Harvey repo (branch master, pushed):**
3. **Task 3: correct UAT test-1 protocol ordering + mark CR-01 fixed (G-07-1, G-07-2)** — `7e2531eb7` (docs) — 07-UAT.md, 07-REVIEW-DISPOSITION.md, 07-EVIDENCE.md
4. **Task 4: advance r9 pin past the devether readnum_content fix (G-07-2)** — `d5df2df38` (chore) — flake.lock ALONE (the isolated pin commit)
5. **Task 4: evidence index rows for the G-07-2 pin advance** — `eef588612` (docs) — 07-EVIDENCE.md

**Plan metadata:** the docs commit (this SUMMARY + STATE/ROADMAP/REQUIREMENTS) rides on top in the Harvey repo.

**TDD gate compliance:** RED `test(07-09)` (1e77266) precedes GREEN `feat(07-09)` (1c16193) in the r9 log; RED_EVIDENCE_OK recorded before GREEN; no refactor commit needed (the GREEN is one expression + comments).

## Files Created/Modified

- `r9:port/src/devether_tests.rs` (NEW, 150 lines) — the four pins: readnum_content(-1)/(0)/(65535) byte-exact + conv_close_last_close_reset_is_donor_intentional (the alloc-first drive; ConvBook assertions through the NETIF pub(crate) chain; drain+close observed behaviorally via pop_frame's Pop::Closed; the EOF wake recorded by the stub EtherHooks with a fixed NONZERO pid)
- `r9:port/src/devether.rs` — readnum_content: pub(crate) + `val as u32 as u64` + the corrected donor-shape doc comment (truncation through 32-bit ulong, never sign extension; the class-sweep conclusion in the tail)
- `r9:port/src/lib.rs` — `#[cfg(test)] mod devether_tests;` with the crate-root rationale comment
- `07-UAT.md` — test 1's expected block: the corrected read-first-then-connect protocol (the literal `cat /net/ether0/$conv/data &` line) + the WHY sentence; results/severity and test 2 byte-untouched
- `07-REVIEW-DISPOSITION.md` — CR-01 fixed (YAML + table row, source `07-09 (u32 truncation + host pins)`); `open: 13`; `total: 14` unchanged; WR-05/WR-06 untouched
- `07-EVIDENCE.md` — the `### 07-09 … virtnet gate re-proof at r9 1c161931…` section + the two new Pinned-commits rows (the pushed r9 HEAD with the push range; the pin commit naming flake.lock ALONE)
- `flake.lock` — r9 pin `46a2ed2` → `1c16193` (rev/narHash/lastModified; owner requaos unchanged) — its own commit

## Decisions Made

Surfaced in the frontmatter key-decisions; the short forms:

1. **The fix is exactly the diagnosis** — one expression reproducing the donor's conversion width; the prohibition of a saturating clamp honored (a guard would mask a future width regression; the u32 ceiling makes the underflow unreachable by construction).
2. **The conv_close reset is pinned, not repaired** — G-07-1 was a test-procedure defect; the host test records the donor's last-close reset as intentional so no future reader re-misdiagnoses it.
3. **Push-before-pin at the fix HEAD** — the carried-forward locked decision re-executed: r9 pushed first, the pin advanced second as its own isolated commit, the hosted CI watched to green at the pushed HEAD.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `gh` CLI absent in this environment — GitHub REST API substituted for the CI gates**
- **Found during:** Task 4 (the `gh run list` / `gh run watch` steps)
- **Issue:** `gh` is not on PATH and not in the harvey nix devshell — the plan's literal `gh run list --workflow=r9-shell.yml …` / `gh run watch <id> --exit-status` commands cannot run.
- **Fix:** used the equivalent unauthenticated GitHub REST endpoints (`/actions/workflows/r9-shell.yml/runs?branch=master` and `/actions/runs/<id>`) with a polling loop to the terminal state; the end-state assertion (a completed-success run with headSha == the pushed Harvey HEAD) ran identically against the API JSON. The single-run endpoint carries a raw control character in a commit title — parsed tolerantly (control chars stripped) after strict JSON failed.
- **Files modified:** none (verification-only substitution)
- **Verification:** run 42043430592 status=completed conclusion=success at headSha eef588612c26; CI-GREEN-AT-PIN printed
- **Committed in:** n/a

**2. [Rule 3 - Blocking] The `check tdd-red-evidence` record needs TAP-shaped output; cargo test output is not TAP**
- **Found during:** Task 1 (the RED gate's evidence validation — first attempt INVALID_RED/invalid_record on schema, second on a missing TAP plan line)
- **Issue:** the gate parses the record's `output` field as TAP (`# tests`/`# pass`/`# fail` summaries + `ok N -`/`not ok N -` lines + a `1..N` plan); cargo's default output is neither TAP nor Surefire XML.
- **Fix:** generated the record's `output` as a faithful TAP transcription OF THE ACTUAL RUN (parsed from /tmp/07-09-red.log — the four real result lines and counts, the plan line inserted per the node --test shape); `command` names the real command, `exitCode` 101, `targetTest` the -1 pin, expected/actual recorded verbatim (panic at devether.rs:493:15).
- **Files modified:** none (the record lives at .planning/tmp/07-09-red-evidence.json — an untracked working artifact)
- **Verification:** `check tdd-red-evidence` → RED_EVIDENCE_OK ("target_test_failed", GREEN authorized)
- **Committed in:** n/a

---

**Total deviations:** 2 auto-fixed (both Rule 3 — verification-environment substitutions; the artifacts and gates themselves are exactly as planned). **Impact on plan:** none on substance — every plan gate ran and passed with its stated intent asserted.

## Issues Encountered

- **Concurrent Phase-6 session sharing the working tree (expected per the dispatch's dual-repo contract, managed):** (a) r9 `aarch64/src/exec.rs` was dirty before this plan's first commit (the other session's in-flight MAX_ARGS 16→32 work for the 06-07 tier-1 sweep) and remains so — never staged, never touched; the R9-PUSHED gate's tree-clean leg was therefore asserted as remote==HEAD (the load-bearing half: origin/main == 1c16193, the gated fix HEAD) PLUS plan-scope-clean (`git status --porcelain -- port/src/{devether.rs,devether_tests.rs,lib.rs}` empty — all 07-09 r9 work committed). (b) The other session committed `69c4cf689` onto harvey master between my push and the CI watch — my pushed HEAD `eef588612` is an ancestor of the tip, its own run 42043430592 is SUCCESS at exactly that SHA, and the tip's run 42043437746 is also SUCCESS. Both recorded honestly in the evidence rows and the coverage block.
- **Panic location drifted 492:15 → 493:15:** Task 1's visibility edit added one comment line above readnum_content, shifting the underflow site the RED log names. The diagnosis/UAT citation (492:15) was the pre-edit line; the gate greps the payload line alone per the plan's note, so the shift is cosmetic. The site is gone post-GREEN regardless.

## Authentication Gates

None — the r9 and harvey pushes used the existing GitHub SSH credentials; the CI queries used unauthenticated REST reads on the public repo.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Immediate next step (orchestrator-owned, per the plan's output):** the guest UAT re-run of 07-UAT.md tests 1-2 via /gsd-verify-work against the composed-boot venue at the new pin — test 1 with the corrected read-first-then-connect protocol, test 2 expecting `4294967295` instead of the panic. The pin resolves the fixed kernel from a clean checkout (`nix run .#boot-r9`), and the hosted run proves the linux/TCG leg.
- **Phase 8:** the pinned devether semantics (open-lifetime-scoped connect; the byte-exact type-file shape) are now host-test-locked — ethermedium/arp bind against a contract that cannot silently drift.
- **Review ledger:** 13 findings remain open (WR-01..08, IN-01..05) — WR-05/WR-06 explicitly untouched by this plan.
- No blockers.

## Self-Check: PASSED

- Files: r9 `port/src/devether_tests.rs` FOUND (150 lines ≥ the 40-line must_have) and git-tracked; `grep -c 'pub(crate) fn readnum_content' devether.rs` == 1; `grep -c 'val as u32 as u64'` == 1; `grep -c 'mod devether_tests' lib.rs` == 1; no `saturating_sub`/`clamp` in the readnum_content body (non-comment lines); 07-UAT.md carries the literal `cat /net/ether0/$conv/data &`; 07-REVIEW-DISPOSITION.md `disposition: fixed` + `^open: 13$` + unchanged `^total: 14$` + WR-05/WR-06 open; 07-EVIDENCE.md carries 're-proof' + the two new pin rows; flake.lock locks rev 1c161931000e77edb546a972f33b1adfa89b0b30 with owner requaos (-A8 grep + python3 structural assert)
- Commits FOUND in git log: r9 `1e77266` (test, RED) + `1c16193` (feat, GREEN); harvey `7e2531eb7` (docs), `d5df2df38` (chore — diff-tree names flake.lock ALONE), `eef588612` (docs)
- Gates re-run at final state: port suite 51/51 (+4 vs baseline 47); `cargo xtask test --verbose` green; virtnet arp PASS + blast PASS both at 1c16193 with no intermediate commits; R9-PUSHED (remote==HEAD, plan-scope clean); PIN-OK; CI-GREEN-AT-PIN (run 42043430592 success at eef588612c26; run 42043437746 success at the tip carrying these commits)
- Bookkeeping greps: BOOKKEEPING-DONE printed
- No files staged outside plan scope; the concurrent session's files (r9 exec.rs; harvey's pre-existing dirty/untracked set) left untouched in both repos

---
*Phase: 07-r9-pci-ecam-virtio-net-devether*
*Completed: 2026-11-05*
