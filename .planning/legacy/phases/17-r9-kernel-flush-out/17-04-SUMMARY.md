---
phase: 17-r9-kernel-flush-out
plan: 04
subsystem: kernel
tags: [r9, virtio, pci, cap-walk, wr-01, wr-02, wr-03, wr-04, wr-07, wr-08, device-input, fod-rewire, host-pins]

requires:
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: the WR-01..08 finding bodies this plan closes (fixed or dispositioned)
  - phase: 17-r9-kernel-flush-out
    provides: the closure pipeline proven on the CR-01 exemplar (17-01), the churn/profile gates (17-02), the WaitSet/used-id classes + pin-advance lineage at 658001b (17-03)
provides:
  - WR-01..04 CLOSED — every cap-walk admission hole is a loud bind-time Err in the '(device fault)' family (zero-length extents, end edge, common/isr/notify minimums, devcfg 8, BAR type nibble, natural alignment, parse alignment/short-notify)
  - WR-07/WR-08 dispositioned — the 07 ledger's native deferred state carrying the full D-15 3-part rationale (WHERE/WHY/Reopener:)
  - the 07 ledger's WR sweep COMPLETE — 8 rows resolved (6 fixed, 2 deferred), only the 5 IN rows open (Pitfall 10)
  - FLUSH-03 COMPLETE (WaitSet sizing + used-id liveness closed by 17-03; the cap-walk class + dispositions by this plan)
  - the FOD re-wire lineage advanced 658001b -> 63bfda9 (Harvey pin 02218e88c, lock-only) with the two-derivation byte-identity proof in three-way agreement
affects: [17-05..17-07, VERIFICATION (walks 17-FLUSHOUT.md), the services venue owner (the deferred dns-stage trap)]

actuals:
  tokens: 15500   # chars/4 over the realized diff: r9 23072 + Harvey ~39500 (ledger/FLUSHOUT/deferred/evidence/SUMMARY), /4
  tasks: 3
  commits: 2      # measured: git rev-list --count 20ed9e7d0..ef7b6a04c (pin advance + ledger closure; the SUMMARY docs commit follows; the r9 side carries cc22223 + 63bfda9 pushed)
plan_head_before: 20ed9e7d083f18084ccce3dd29a3f923bbc0a946
plan_head_after: ef7b6a04c   # the last production/state commit; this SUMMARY's own docs commit follows (the 17-03 convention)

tech-stack:
  added: []
  patterns: ["the lying-cap admission pins mirror the LIVE qemu layout (a 0x4000 BAR tiling common/isr/device/notify 0x1000 each) — the live-shape pin (cap_extents_admit_the_live_q35_shape) is the standing guard against over-tightening the admission checks", "bind-path validation extracted as pure cfg(any(test, dev_ether)) predicates beside their volatile callers (check_cap_extents, is_64bit_mem_bar_nibble, notify_reachable) — the host battery pins the logic, the battery re-runs prove the wire"]

key-files:
  created: [.planning/phases/17-r9-kernel-flush-out/17-04-deferred-items.md, .planning/phases/17-r9-kernel-flush-out/evidence/17-04-t{1,2}-red-{evidence,raw-libtest}.json]
  modified: [../r9/port/src/virtio.rs, ../r9/port/src/virtio_tests.rs, ../r9/x86_64/src/platform/q35/pci.rs, ../r9/flake.nix, flake.lock, .planning/milestones/v2.0-phases/07-r9-pci-ecam-virtio-net-devether/07-REVIEW-DISPOSITION.md, .planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md]

key-decisions:
  - "The end edge was re-derived against the live caps (the flagged assumption falsified): qemu sizes the modern BAR pow2ceil(notify.offset + notify.size) with notify.size = notify_off_multiplier * VIRTIO_QUEUE_MAX = 0x1000 at 0x3000, so the LIVE notify cap ends exactly AT the 0x4000 boundary — a >=-style edge would reject the real device and redden every net gate. The closed form (the plan's own fails_when prescription): the > edge stays, the zero-length reject closes the named offset==bar_size length==0 hole, and the live-shape pin guards the edge against future over-tightening"
  - "The TDD single-commit shape: RED evidence captured and classified BEFORE implementation (RED_EVIDENCE_OK x2), but the pins rode the ONE class commit per the plan's Pitfall-5 prohibition — no separate test() commit exists for 17-04 (see TDD Gate Compliance)"
  - "WR-07/WR-08 closed as the ledger's native 'deferred' carrying the D-15 3-part rationale in Source (the D-13 'dispositioned' token mapped onto the ledger's own vocabulary — the flagged assumption, same class as 17-02's resolved-vs-fixed mapping)"
  - "The services dns-stage trap is pre-existing (A/B-proven identical at pin b02408cad) — deferred, not fixed (close-what-you-touch)"

patterns-established:
  - "Device-input admission pins are shaped by the live qemu source, not the spec's letter: read the device model (virtio-pci.c) when an admission edge could over-reject, and pin the live shape as the no-over-tightening guard"
  - "Extract-then-pin: a cfg-gated bind-path validation becomes a pure predicate beside its volatile caller, RED-proven against the extracted-but-unfixed semantics (the extraction is a behavior-preserving move riding the same class commit)"

requirements-completed: [FLUSH-03]

coverage:
  - id: D1
    description: "WR-04: Caps::parse rejects misaligned cap pointers and short notify caps (shared, both arches)"
    requirement: FLUSH-03
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- caps = 5/5 (caps_parse_rejects_misaligned_capability_pointers all three residue classes + caps_parse_rejects_short_notify_capabilities, both RED-proven — evidence/17-04-t1-*.json, RED_EVIDENCE_OK; the well-formed fixture pin unchanged); battery 114/114 (baseline 112)"
        status: pass
    human_judgment: false
  - id: D2
    description: "WR-01/WR-02: the q35 bind-time extent admission — zero-length, end edge, minimum extents (common 0x40, isr 1 subsumed, notify_off x mult), devcfg 8"
    requirement: FLUSH-03
    verification:
      - kind: unit
        ref: "cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu = 76/76: cap_extents_admit_the_live_q35_shape + reject_zero_length + reject_extents_past_the_bar_end (incl. the u32-wrap guard) + reject_undersized_common_and_device_configs (6/7 rejected, 8/0x40 admitted) + notify_offsets_must_reach_inside_the_notify_cap; the zero-length/undersized/alignment pins RED-proven — evidence/17-04-t2-*.json"
        status: pass
      - kind: integration
        ref: "boot-r9-net-smoke-x64 exit 0 + boot-r9-smoke-x64 exit 0 (nic-bind stage) at pin 02218e88c — the live virtio-net bind ran every new admission check and bound (no over-rejection)"
        status: pass
    human_judgment: false
  - id: D3
    description: "WR-03: size_bar64 type-nibble validation + bar_assignment natural alignment"
    requirement: FLUSH-03
    verification:
      - kind: unit
        ref: "only_64bit_memory_nibbles_are_sizing_trusted + bar_assignment_rejects_naturally_misaligned_window_bases (misaligned None, aligned Some, size-0 None without division; the misalignment pin RED-proven); battery 76/76 (baseline 69)"
        status: pass
      - kind: integration
        ref: "smoke + net-smoke gates green at pin 02218e88c — the live device's nibble 0b100 still sizes and the firmware assignment arm is untouched"
        status: pass
    human_judgment: false
  - id: D4
    description: "The discipline half: push-before-pin, FOD re-wire with the two-derivation proof, lock-only pin advance, the battery family at the new pin"
    verification:
      - kind: integration
        ref: "R9-CLEAN-PUSHED (r9 cc22223..63bfda9 pushed, unpushed=0); PIN-COMMIT-CLEAN (02218e88c is flake.lock-only); FOD proof: kernel A plain + kernel B wr04fix-proof-b override -> byte-identical kernel content (sha256-U1qaBWsh…) and ISOs (file sha256 f50320fa…, NAR sha256-9gK1DV8… = f602b50d…), in three-way agreement with the FOD's own got-value; nix build .#boot-iso + --rebuild green; battery at pin 02218e88c: smoke=0 net-smoke=0 tier1=0 (38 T1-OK, zero SKIP) churn=0 (16 bytes/exit <= 1772) net-soak=0 (R9_NET=tap, 2048 baseline + 8x512 burst, 9x tcpblast: done, zero netip: FAIL); services=1 — pre-existing dns-stage trap, A/B-proven identical at pin b02408cad (deferred)"
        status: pass
    human_judgment: false
  - id: D5
    description: "WR-07/WR-08 3-part dispositions + the 07 ledger's WR sweep complete + the six FLUSHOUT rows"
    verification:
      - kind: integration
        ref: "ledger gates: WR-01..04 disposition: fixed, WR-07/08 disposition: deferred (YAML), open count exactly 5 (the IN rows), WR-07/WR-08 table rows grep 'Reopener:' (D15-PARTS-PRESENT x2); FLUSHOUT-WR-FILLED (all six qualified rows, no pending)"
        status: pass
    human_judgment: false

# Metrics
duration: 96 min
completed: 2026-11-12
status: complete
---

# Phase 17 Plan 04: The Cap-Walk Admission Holes (WR-01..04) + the WR-07/08 Dispositions Summary

**All four cap-walk admission holes closed as loud bind-time '(device fault)' rejects on the live q35 path — one RED-proven class commit (r9 cc22223, 9 new host pins, batteries 114/114 + 76/76), FOD re-wired with the two-derivation proof, pinned lock-only (02218e88c), the battery family green except a proven-pre-existing services trap — and the 07 ledger's WR sweep completed (8 rows resolved, only the 5 IN rows open).**

## Performance

- **Duration:** 96 min (the battery family incl. the tap soak is the designed cost — Pitfall 5)
- **Started:** 2026-11-12T22:23:11Z
- **Completed:** 2026-11-13T00:05:00Z
- **Tasks:** 3/3
- **Files modified:** 9 (r9 4 + Harvey 5, excluding plan metadata)

## The Four Fixes (one class commit, r9 cc22223)

**WR-04 — shared `Caps::parse`** (`port/src/virtio.rs`): a `p & 3` reject at the walk's top (before ANY read at the pointer — a misaligned pointer was an unaligned volatile u32 on the real ECAM seam, UB/a Device-memory fault), and the notify arm rejects `cap_len < CAP_NOTIFY_LEN` (the multiplier was read out of the NEXT cap's bytes). Pins: all three residue classes + the short-notify shape, both RED-proven.

**WR-01 — the extent admission** (`x86_64/src/platform/q35/pci.rs`): the T-07-11 loop extracted as the pure `check_cap_extents` (called at the exact bind site) with zero-length rejects (closing the exact `offset == bar_size, length == 0` hole), the end edge, and the minimums (common >= 0x40 through CFG_QUEUE_DEVICE+8; isr >= 1 subsumed by the zero-length reject). `bring_up_queue` additionally requires `queue_notify_off x multiplier` to address a u16 inside the notify cap (`notify_reachable`).

**WR-02 — the device-config minimum rises 6 -> 8** with the comment naming the IRQ handler's `mmio_read16(state.devcfg + 6)` link-status read: lengths 6/7 previously admitted a config the handler over-reads on the first config-change interrupt.

**WR-03 — the BAR path**: `size_bar64` validates the type nibble (memory, bit 0 clear + 64-bit, bits 2:1 == 0b10 — `is_64bit_mem_bar_nibble`) BEFORE the all-ones probe is trusted (returns Option; the bind maps None to a '(device fault)' Err), and `bar_assignment` adds the natural-alignment check beside the unchanged window-END check (`checked_rem` — size 0 is a loud None, never a division).

## TDD Gate Compliance

Both fix tasks carried `tdd="true"` — RED captured, classified, and ONLY then implemented:

| Task | RED | Evidence | GREEN |
|------|-----|----------|-------|
| 1 (WR-04 parse) | working tree, pre-implementation | RED_EVIDENCE_OK (`evidence/17-04-t1-*.json` — both pins failed on their planned assertions: the misaligned cap was WALKED, the short-notify parsed Ok) | r9 `cc22223` |
| 2 (WR-01/02/03 q35) | working tree, pre-implementation (after the behavior-preserving extraction of `check_cap_extents`, so the pins exercised the EXISTING admission logic — assertion failures, not missing symbols) | RED_EVIDENCE_OK (`evidence/17-04-t2-*.json` — 3 pins RED: zero-length admitted, undersized common/device-6/7 admitted, misaligned window assigned; 3 controls green incl. the live shape) | r9 `cc22223` |

**Plan-directed deviation from the standard commit split:** the plan's own prohibition (must_haves + Task 1's action + Task 2's acceptance criteria — Pitfall 5) mandates ONE device-input class commit carrying pins AND fixes, so no separate `test(17-04)` commit exists and the GREEN commit carries `fix(...)` (the 17-03 message-family precedent). The RED evidence records and the classifier verdicts above are the audit trail; the mechanical gate (`git log --grep '^test\(17-04\)'` finds nothing) is satisfied by this section, per the tdd gate's SUMMARY contract.

## The Pin-Advance Lineage (658001b -> 63bfda9 -> Harvey 02218e88c)

1. r9 `cc22223` fix(17-04): the ONE class commit — parse hardening + all four q35 holes + all 9 pins
2. r9 `63bfda9` fix(17-04): **FOD re-wire** — the stale wiring measured `got: sha256-9gK1DV8…`; kernel A (plain, cc22223) + kernel B (never-before-used `wr04fix-proof-b` override) built independently, byte-identical kernel content (`sha256-U1qaBWsh…`); ISOs through the non-FOD `lib.bootIso` byte-identical (file sha256 `f50320fa6dae382c035e9578e0eb66d8d13e33297f4b9372622ea76662b39448`, NAR `sha256-9gK1DV8rQHPfU76lboc+0DWquRkQelBvvaT7OUQ/AJ4=` = `f602b50d5f2b4073df53bea56e873ed035aab919107a506fbda4fb39443f009e`) — three-way agreement with the FOD's own got-value; `nix build .#boot-iso` + `--rebuild` green
3. r9 pushed `658001b..63bfda9` BEFORE the Harvey advance — push-before-pin held (R9-CLEAN-PUSHED, unpushed=0)
4. Harvey `02218e88c` chore(17-04): pin -> 63bfda9, **flake.lock only** (PIN-COMMIT-CLEAN)
5. Harvey `ef7b6a04c` docs(17-04): the 07-ledger rows + FLUSHOUT cells + deferred item + RED evidence

## Gates at the New Pin (the family, D-04's designed cost)

| Gate | Exit | Note |
|------|------|------|
| `boot-r9-smoke-x64` | 0 | ALL STAGES PASS incl. nic-bind — the live virtio-net bind ran every new admission check |
| `boot-r9-net-smoke-x64` | 0 | ALL STAGES PASS — static+dhcp ipconfig, DUPBIND/NOADDR rejects, ping/udp/tcp echo |
| `boot-r9-tier1-x64` | 0 | ALL STAGES PASS — 38 T1-OK, zero SKIP, float canary |
| `boot-r9-services-x64` | **1** | TIMEOUT at dns-verdict: kernel PF at pc=0x0 after `devmnt: server error: cs:` — **pre-existing** (below) |
| `boot-r9-churn-x64` | 0 | ALL STAGES PASS — 1400 cat iterations, 16 bytes/exit <= 1772 |
| `boot-r9-net-soak-x64` (R9_NET=tap) | 0 | ALL STAGES PASS — 2048 baseline + 8x512 burst rounds, 9x `tcpblast: done`, zero `netip: FAIL` (the soak refuses the slirp venue at parse time per 16-05 D-11 — the tap leg is the verdict's venue) |

The live caps print re-proven at the pin: `pci: bar4 … size 0x4000` / `caps common@0x0+0x1000 notify@0x3000 isr@0x1000 device@0x2000 mult 4` — the real device binds under the strictest admission checks the path has carried.

## The Services Trap (pre-existing, A/B-proven, deferred)

The dns stage faults: kernel NULL-call (PF at pc=0x0, cs=0x8, sp in the kstack window) immediately after `devmnt: server error: cs: file does not exist`. **A/B discriminator**: the IDENTICAL trap frame (same pc/sp/r13/…; `ax` differs only by 0xF0 — the link-layout shift of the 17-04 code, the same static symbol) fires at Harvey pin `b02408cad` (r9 658001b, pre-17-04) run from a clean worktree — not a 17-04 regression. Services was last green at the 15-08 pin era and not run in 17-01..17-03, so the introducing commit sits in that window (suspect orbit: the 17-02 exit_tail/fd-group drain fixes or kernel-x64's q35churnshell re-point — undetermined). Deferred: `17-04-deferred-items.md` + a WINDOWS deviation row; the root-cause belongs to the next plan touching the services venue. Per the scope boundary this plan does not fix it.

## Task Commits

1. **The class commit + FOD + pin** - r9 `cc22223` (fix) + r9 `63bfda9` (fix, FOD re-wire) + Harvey `02218e88c` (chore, lock-only)
2. **Ledger closure** - Harvey `ef7b6a04c` (docs: 07-REVIEW-DISPOSITION WR-01..04 fixed + WR-07/08 deferred, the six FLUSHOUT rows, the deferred item, the RED evidence)
3. **Plan metadata** - this SUMMARY + STATE/ROADMAP/REQUIREMENTS (docs)

## Ledger Closure (D-13/D-15/D-16)

- **07-REVIEW-DISPOSITION.md**: WR-01..04 flip `open -> fixed` (YAML + table) with Source cells citing r9 cc22223 + the pin test names; WR-07/WR-08 flip to `deferred` carrying the full 3-part rationale — WR-07 WHERE: the superseded aarch64-era net-sweep.rc fixture (leading-zero counts the arp loop's string match can never equal), WHY: the committed tier-sweep machinery replaced it + the aarch64 freeze, Reopener: the fixture returns to a live venue (or aarch64 reactivation); WR-08 WHERE: the frozen aarch64 DTB/ICFGR trigger-flag path, WHY: the aarch64 freeze policy (committed DTBs are level-high — latent only), Reopener: aarch64 reactivation (plumb flags into gic::enable or reject non-level routes at discovery). `open: 11 -> 5` — exactly the IN rows remain (Pitfall 10; the verify gate's open-count <= 5 holds)
- **17-FLUSHOUT.md**: the six qualified rows filled (four `fixed` with pins/commits, two `dispositioned (deferred)` with ledger refs) — FLUSHOUT-WR-FILLED
- The D-13 literal token "dispositioned" is not a state the ledger's header enumerates; the plan's flagged assumption mapped its semantics onto the native `deferred` (the hand-set value whose documented home for the reason IS the Source cell) — recorded here as the vocabulary-mapping decision

## Decisions Made

- The end edge (the flagged assumption, falsified at execution): see the front matter and Deviations — the live qemu notify cap ends exactly at the bar boundary by construction, so the strict edge would reject the real device
- The extent pins mirror the live qemu layout (from `hw/virtio/virtio-pci.c`: notify.size = mult x VIRTIO_QUEUE_MAX; BAR = pow2ceil(notify.offset + notify.size)) rather than an invented one — the live-shape pin is the no-over-tightening guard
- The notify_off x mult check lives in `bring_up_queue` (queue_notify_off is a common-config read, only available after the region is trusted — the check stays a bind-time Err in the '(device fault)' family)
- The net-soak verdict leg ran on tap (`R9_NET=tap`) — the soak app's slirp refusal is 16-05's designed venue honesty, not a failure

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Falsified flagged assumption] The end edge re-derived against the live caps**
- **Found during:** Task 2 (before writing the edge)
- **Issue:** The plan's prescribed ">=-style edge" (an extent ending at the boundary rejected) is unsatisfiable together with the battery-green truth: qemu 11.1.2's notify cap length is `notify_off_multiplier * VIRTIO_QUEUE_MAX` = 4 x 1024 = 0x1000 at offset 0x3000, and the BAR is sized `pow2ceil(notify.offset + notify.size)` = 0x4000 — the live notify cap ends EXACTLY at the boundary. The plan itself anticipated this (the battery `fails_when` names over-rejection and prescribes "the edge shape must be re-derived against the live caps")
- **Fix:** The `>` edge stays; the new zero-length reject alone closes the named `offset == bar_size, length == 0` hole (any offset >= bar_size is covered by zero-length + `>`); u64-widened sum unchanged. The live-shape pin (`cap_extents_admit_the_live_q35_shape`) pins the re-derived edge
- **Files modified:** ../r9/x86_64/src/platform/q35/pci.rs
- **Verification:** battery 76/76 + the full family green at the pin (the live bind ran the check)
- **Committed in:** r9 cc22223

**2. [Rule 1 - Bug] My nibble pin's literals mis-encoded**
- **Found during:** Task 2 GREEN (the battery's one failure)
- **Issue:** The first `only_64bit_memory_nibbles_are_sizing_trusted` pinned the type bits shifted one place (`0b100 << 1`); the predicate was correct, the pin's literals were not
- **Fix:** Corrected literals (nibble 0b100 = the live shape); predicate untouched
- **Files modified:** ../r9/x86_64/src/platform/q35/pci.rs
- **Verification:** battery 76/76
- **Committed in:** r9 cc22223

---

**Total deviations:** 2 auto-fixed (1 falsified assumption via the plan's own prescribed path, 1 test-literal bug)
**Impact on plan:** No scope creep; the end-edge re-derivation is the plan's own contingency branch, executed with the discriminating evidence (qemu source + live log) recorded.

## Issues Encountered

- The services-x64 dns-stage trap — pre-existing (A/B-proven identical at the pre-17-04 pin), deferred (see above)
- r9 carries ~439 pre-existing `cargo fmt --check` drift sites (no fmt gate in its CI — check+clippy only); this plan kept its own added lines fmt-clean and added zero clippy warnings (stash-diff verified)
- The TAP conversion of libtest JSON followed 17-03's precedent (lossless transform; raw JSON preserved beside each evidence record)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- FLUSH-03 is COMPLETE (17-03 + this plan closed all its named classes: WaitSet sizing, used-id liveness, the cap-walk holes, and the two dispositions)
- 17-05..17-07 proceed at pin 02218e88c (r9 63bfda9) on the hardest admission-checked discovery path the tree has carried
- The services dns-stage trap is the open thread for whichever plan next touches that venue (or a dedicated fix); the 07 ledger has only the 5 IN rows left open by policy

## Self-Check: PASSED

- Files exist on disk: the 3 r9 sources + r9 flake.nix (committed, tree clean/pushed); Harvey flake.lock + the ledger + FLUSHOUT + deferred + evidence (committed)
- Commits verified: r9 cc22223/63bfda9 on origin/main (R9-CLEAN-PUSHED); Harvey 02218e88c/ef7b6a04c ancestors of HEAD; the pin commit is flake.lock-only
- Plan-level verification re-run post-commit: caps filter 5/5; port battery 114/114; x86_64 battery 76/76; ledger gates (fixed x4, deferred x2, open count 5, D15-PARTS-PRESENT x2); FLUSHOUT-WR-FILLED; battery family smoke=0 net=0 tier1=0 churn=0 netsoak(tap)=0, services=1 pre-existing (A/B-proven)

---
*Phase: 17-r9-kernel-flush-out*
*Completed: 2026-11-12*
