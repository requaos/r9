---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "13"
subsystem: docs-decision-record
tags: [9p2000-aead, rfc-amendment, compendium-listing, project-md, requirements-md, threat-model, ig-d11-lock, coverage, documents-only]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-01)
    provides: "04-WIRE-DECISIONS.md — the eight ACCEPTED P1 to P8 values and DECISION IG-D11: lock-amended (2026-11-25)"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-03, 04-04, 04-05, 04-06, 04-07, 04-08, 04-09)
    provides: "the shipped code and the eight independent pins the amendments cite and copy their literals from"
provides:
  - "RFC section 3.2, 4.1, 4.3, 4.4, 5, 6.1, 6.2 and 11 each end with a dated Phase 4 amendment note quoting the superseded wording"
  - "compendium listing 9.9 rewritten in place with a dated note; section 5.3 points at amended RFC 4.3"
  - "PROJECT.md: W1, W2 and W4 settled and out of the Open Questions table; N1 fully, N5 (Phase 4) and N11 (half) annotated; eleven dated Key Decisions rows including the IG-D11 lock; the lock-count sentence updated"
  - "REQUIREMENTS.md: WIRE-02 six bits with EXT_STATELESS_CAP, WIRE-10 class in its own byte, WIRE-06 note naming Rstream"
  - "docs/threat-model.md: the six Phase 4 wire protections marked implemented and host-tested, not used by a booted kernel; peer authentication not provided; W4 note settled"
  - "the phase's COVERAGE.md no-external-API declaration"
affects: [04-14 (phase-end code review and the STATE.md W1/W2/W4 row removal), phase 05 (page-level check, DMA body shape), phase 06 (legacy gateway, real key exchange)]

actuals:
  tokens: 14124
  tasks: 2
  commits: 2
  plan_head_before: 5e1cf705e265b9c05798bd8ce6c9509b5fa062a7
  plan_head_after: 038ef15

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "dated amendment notes placed at the end of the section they amend, quoting the superseded wording rather than deleting it"
    - "every keyed or placed formula names the independent test that computes the expected bytes outside the function under test (`Pinned by:`)"
    - "a settled open question leaves the table and becomes a dated Key Decisions row; a superseded decision keeps its old row, marked superseded, never silently edited"
    - "the compendium appendix listing is a non-normative sketch; the RFC as amended and the code are normative"

key-files:
  created:
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/COVERAGE.md
  modified:
    - .planning/rfc-9p2000-aead.md
    - .planning/r9-architecture-and-code-v4.md
    - .planning/PROJECT.md
    - .planning/REQUIREMENTS.md
    - docs/threat-model.md

key-decisions:
  - "The eight RFC notes quote the superseded wording and state the operative reading; the five keyed/placed notes end with `Pinned by:` naming the independent test."
  - "The IG-D11 row is written from the DECISION line (lock-amended, 2026-11-25, no wording changes); the old not-locked row stays, marked superseded."
  - "The Open Questions table drops W1, W2 and W4 and the three become dated Key Decisions rows, following the Phase 3 pattern for W3, W5 and W6."
  - "Only the six wire protections this phase actually built are marked implemented in the threat model; the capability-revocation and failover-reclaim rows stay not implemented."

requirements-completed: [WIRE-01, WIRE-02, WIRE-10]

coverage:
  - id: D1
    description: "Eight dated Phase 4 amendment notes in the RFC text (sections 3.2, 4.1, 4.3, 4.4, 5, 6.1, 6.2, 11), each quoting superseded wording; five name their independent pin"
    requirement: WIRE-01
    verification:
      - kind: automated
        ref: "grep -c 'Amended 2026-11-24 (r9 Phase 4' .planning/rfc-9p2000-aead.md == 8"
        status: pass
      - kind: unit
        ref: "the eight named pins (session_derive_known_answer, session_packet_mac_is_keyed_blake3_under_the_mac_key, session_rekey_confirm_is_keyed_blake3_of_label_and_generation, session_cap_tag_is_truncated_keyed_blake3_of_label_and_body, aead_fid_stateless_tag_is_keyed_blake3_of_label_and_wire_body, aead_transport_wire_data_pages_decrypt_alone_at_their_block_counter, aead_transport_wire_tag_is_packet_mac_of_all_bytes_before_it, aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation) print ok"
        status: pass
    human_judgment: true
    rationale: "The notes are prose; gates confirm count and that the pins exist and pass, but that each note's wording matches the code needs a reader."
  - id: D2
    description: "Compendium listing 9.9 rewritten in place with a dated note; section 5.3 points at amended RFC 4.3 for the MAC input"
    requirement: WIRE-01
    verification:
      - kind: automated
        ref: "grep -c 'Rewritten 2026-11-24 (r9 Phase 4' .planning/r9-architecture-and-code-v4.md == 1"
        status: pass
    human_judgment: true
    rationale: "Whether the sketch is accurate and short enough is a reading judgment; the automated gate only proves the note is present."
  - id: D3
    description: "PROJECT.md, REQUIREMENTS.md and the threat model record the settled wire: W1/W2/W4 out of Open Questions, eleven Key Decisions rows, the IG-D11 lock, the WIRE-02/06/10 amendments, and the Phase 4 wire-protection statuses"
    requirement: WIRE-02
    verification:
      - kind: automated
        ref: "grep: no W1/W2/W4 table rows; 'Locked amended 2026-11-25 (owner)' == 1; EXT_STATELESS_CAP == 1; Rstream >= 2; 'implemented and host-tested in Phase 4' == 6; 'Peer authentication is not provided' == 1"
        status: pass
    human_judgment: true
    rationale: "Decision-record prose: the gates prove the rows and phrases exist, not that each rationale reads correctly."
  - id: D4
    description: "COVERAGE.md holds the single no-external-API declaration and no table rows"
    verification:
      - kind: automated
        ref: "grep -c '^No external API integration: ' COVERAGE.md == 1"
        status: pass
    human_judgment: false

# Metrics
duration: 28min
completed: 2026-11-24
status: complete
---

# Phase 4 Plan 13: Record the settled wire in the documents Summary

**Eight dated RFC amendment notes, a rewritten compendium listing 9.9, W1/W2/W4 settled out of PROJECT.md's open questions, the IG-D11 lock row, and the WIRE-02/06/10 and threat-model amendments — documents only, no source touched.**

## Performance

- **Duration:** 28 min
- **Started:** 2026-11-24T00:00:00Z (approx)
- **Completed:** 2026-11-24
- **Tasks:** 2
- **Files modified:** 5 modified, 1 created

## Accomplishments

- The RFC now carries eight `Amended 2026-11-24 (r9 Phase 4, ...)` notes, one at the end of sections 3.2, 4.1, 4.3, 4.4, 5, 6.1, 6.2 and 11, each quoting the wording it supersedes; the five keyed or placed notes end with `Pinned by:` naming their independent test, and the section 3.2 and 11 notes also name `aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation`.
- Compendium listing 9.9 is rewritten in place under a dated note: the 28-byte header with offsets and `HEADER_LEN`/`MAC_LEN`, the six `ExtMask` constants, the full 100–139 opcode list, and the two packet types; the old packed-struct-with-enum handle, the missing `Pad`/`ExtTLVs Length` and the eight missing base opcodes are named as the old errors. Section 5.3 points at amended RFC 4.3.
- PROJECT.md: W1, W2 and W4 leave the Open Questions table and become dated summaries plus eleven dated Key Decisions rows (header/MAC, nonce, data body, capability TLV, key scope, exchange seam, re-key, class byte, opcode set, kernel linking, and the IG-D11 lock); N1 is fully settled, N5 settled for this phase, N11 half-settled; the lock-count sentence now reads eight locked as written, four locked amended, one not locked.
- REQUIREMENTS.md: WIRE-02 names six extension bits including `EXT_STATELESS_CAP`; WIRE-10 says the class is its own byte; WIRE-06 gains a note naming `Rstream` as the stream message that carries page data.
- The threat model marks the six Phase 4 wire protections implemented and host-tested, not used by a booted kernel, and states that peer authentication is not provided; the W4 note is settled. The phase `COVERAGE.md` declares no external API.

## Task Commits

Each task was committed atomically:

1. **Task 1: Amend the RFC text and rewrite compendium listing 9.9** - `32613bb` (docs)
2. **Task 2: PROJECT.md, REQUIREMENTS.md, the threat model, COVERAGE.md** - `038ef15` (docs)

**Plan metadata (SUMMARY):** committed after this file, as `docs(04-13): complete ... plan`.

## Files Created/Modified

- `.planning/rfc-9p2000-aead.md` — eight dated Phase 4 amendment notes.
- `.planning/r9-architecture-and-code-v4.md` — listing 9.9 rewritten; dated sentence in section 5.3.
- `.planning/PROJECT.md` — W1/W2/W4 settled, N1/N5/N11 annotated, eleven Key Decisions rows, IG-D11 lock row, lock-count sentence.
- `.planning/REQUIREMENTS.md` — WIRE-02, WIRE-06 and WIRE-10 dated amendments.
- `docs/threat-model.md` — six Phase 4 wire protections implemented and host-tested; peer authentication not provided; W4 settled.
- `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/COVERAGE.md` — the no-external-API declaration.

## IG-D11 row as written

The new row in PROJECT.md's Key Decisions table reads, cell for cell:

- **Decision:** Native `9P2000.aead` is the distributed wire security, amended by the MAC scope of D-07, the key scope of D-15, the capability TLV of D-11, the nonce rule of D-08 and the class byte of D-27. Outside the lock stay the real key exchange (N5), the page-level check at fault time (N11, second half) and how a qid carries both a class and `QTDIR` (Phase 6).
- **Rationale:** IG-D11; answer given in plan 04-01's checkpoint (`DECISION IG-D11: lock-amended`, 2026-11-25, no wording changes)
- **Outcome:** Locked amended 2026-11-25 (owner)

The old row (`| Native 9P2000.aead is the distributed wire security | IG-D11; details open at W1-W4 | Superseded 2026-11-25 by the amended lock row below; was Not locked, deferred to Phase 4 (W1-W4) |`) is kept and marked superseded.

## Decisions Made

- The RFC notes quote the superseded wording rather than deleting it, and place each note at the end of its section, per the PATTERNS "Dated amendment notes" form.
- The settled warnings leave the Open Questions table and become dated Key Decisions rows, following the Phase 3 treatment of W3, W5 and W6; the three summaries immediately under the table keep the W1/W2/W4 wording traceable.
- Only the six wire protections this phase actually built are marked implemented in the threat model; the capability-revocation and failover-reclaim rows stay not implemented, so the document does not overclaim.
- The compendium's rewritten listing stays a non-normative sketch and names the RFC and `port/src/chan/` as normative, per D-04/D-10.

## Deviations from Plan

None - plan executed exactly as written.

(The eight independent pins were run first, as the plan's rules require; all eight printed `ok` before any note was written.)

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The decision record matches what was built: W1, W2 and W4 are closed in PROJECT.md; WIRE-01's "AC listing 9.9 is corrected" holds; the IG-D11 row says exactly what the owner answered.
- The STATE.md half of D-10 (the W1, W2 and W4 rows leaving STATE.md's open-question table) is plan 04-14 Task 1, which runs on the main working tree because a worktree executor must not edit STATE.md or ROADMAP.md.
- Plans 04-14 and 04-15 must not be dispatched in worktrees; see the owner action below.

## Owner action before wave 10

Before continuing `/gsd-execute-phase 4`, set `workflow.use_worktrees` to `false` in `.planning/config.json` (or through `/gsd-config`), then set it back to `true` after plan 04-15. Plans 04-14 and 04-15 run only on the main working tree; plan 04-14 stops at once with `## CHECKPOINT REACHED` if it is dispatched in a worktree. The executor did not change the setting itself.

## Self-Check: PASSED

Verified after writing this file: both task commits (`32613bb`, `038ef15`) are ancestors of HEAD; the six changed/created files exist; `.planning/STATE.md` and `.planning/ROADMAP.md` are unchanged by this plan; no file under `port/`, `crypto/`, `ds/`, `xtask/`, `aarch64/`, `x86_64/` or `riscv64/` was committed under the `(04-13)` scope.
