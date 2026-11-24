---
phase: 02-guide-phase-0-design-lock-and-rfork-flag-reservation
plan: 02
subsystem: docs
tags: [threat-model, security, documentation, xtask-test]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: the xtask crate and `nix develop` devshell the document test runs in
provides:
  - "`docs/threat-model.md`: scope boundary (AC 2.2 verbatim), assets, attacker classes, trust boundaries, protections with delivering phase, out-of-scope remedy, open points"
  - "`xtask/tests/threat_model.rs`: six document checks run by `cargo test -p xtask threat_model`"
  - "PROJECT.md notes N9, N10, N11: three source contradictions that were untracked"
affects: [phase-03, phase-04, phase-05, phase-06, phase-07, phase-13, phase-14]

actuals:
  tokens: 14000
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "Document test that reads the document and its source at run time, so the quoted boundary cannot drift from the compendium"
    - "A draft crosses an owner stop as a committed record with its sha256 in a ledger; the final file is committed only when byte-equal to it"

key-files:
  created:
    - docs/threat-model.md
    - xtask/tests/threat_model.rs
    - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/PINS-threat-model.md
    - .planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/evidence/threat-model-draft.md
  modified:
    - .planning/PROJECT.md
    - .planning/STATE.md

key-decisions:
  - "Citation tags: `[AC n.n]`, `[RFC n.n]`, `[IG n.n]` / `[IG n row k]`, `[PT §n.n]` / `[PT Test n.n]`; the mark for unsourced statements is `**[new]**`"
  - "Bullets are written one per line, because the test checks the line that starts the bullet"
  - "The protection table's first column is headed \"Protection, as specified\" so present-tense rows cannot be read as existing behaviour"
  - "Owner direction: every source contradiction in the document must be tracked as an open question; three untracked ones became notes N9 to N11"

patterns-established:
  - "Threat-model rows carry a source tag or the new mark; a later phase that delivers a protection changes its status cell and the test"

requirements-completed: [DSGN-01]

coverage:
  - id: D1
    description: "docs/threat-model.md is committed, quotes the AC 2.2 boundary word for word, names SEV-SNP and TDX, and states that no protection is implemented"
    requirement: "DSGN-01"
    verification:
      - kind: integration
        ref: "nix develop -c cargo test -p xtask threat_model (6 tests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every citation says what the document claims, and no statement overstates protection"
    requirement: "DSGN-01"
    verification: []
    human_judgment: true
    rationale: "A test can check that a tag is present, not that the cited section supports the row; the owner reviewed the draft"
  - id: D3
    description: "The committed document is byte-equal to the draft record the owner's approval refers to"
    requirement: "DSGN-01"
    verification:
      - kind: other
        ref: "cmp -s docs/threat-model.md evidence/threat-model-draft.md && git diff --quiet HEAD -- both"
        status: pass
    human_judgment: false

duration: 75min
completed: 2026-11-23
status: complete
---

# Phase 2 Plan 02: Threat Model Document Summary

**`docs/threat-model.md` is committed: a full analysis whose boundary is the compendium's section 2.2 word for word, every row cited or marked new, and every protection marked not implemented.**

## How it was run

The owner asked for this one plan to be run on Fable 5.2 ahead of the rest of the phase. GSD's dispatch guard refused to start an executor in the main checkout while `workflow.use_worktrees` is unset, and the plan needs the main checkout, so at the owner's direction the orchestrating session executed the plan's tasks itself. No separate executor agent was used. Plans 02-01 and 02-03 to 02-06 have not run.

## Accomplishments

- RED pin: `xtask/tests/threat_model.rs` committed with six tests, all failing with no document (`7cb7e66`).
- Draft written against the test and the four source documents: 18 assets, 13 attacker classes, 13 trust boundaries, 25 protections with delivering phase, 8 source contradictions and 9 gaps.
- Draft committed as a record with its sha256 (`0dd9a2d`), reviewed by the owner, revised once, and committed at `docs/threat-model.md` (`647be60`).
- GREEN twice, pin advanced (`397baef`).
- Three contradictions that no warning or note tracked are now PROJECT.md notes N9 (stale-frame mechanism), N10 (plaintext in legacy bounce frames), N11 (where the wire MAC is verified) (`43da67b`).

## Owner checkpoint (Task 2)

- Draft shown: sha256 `c6896d531acd2994da4613ccab3e6f24316beb874b51c2222e8d7fc4fd69574d`, commit `0dd9a2d`.
- Owner's reply, verbatim: "ensure all the contradictions a listed as open questions and then approve"
- Action taken: notes N9 to N11 added to PROJECT.md; the three draft bullets that read "No warning tracks this yet **[new]**." now read "Tracked as note N9." / "N10." / "N11.", and the Sources line reads "notes N1 to N11". Four lines changed.
- Revised draft record: sha256 `a8f28c3595b0833b6ce6841515fed4b3171e229ab6fcdcd2529fd3a79707bfb0`, commit `ed54dc5`. The committed document equals this record.

## Verification results

| Check | Result |
|-------|--------|
| `nix develop -c cargo test -p xtask threat_model` with no document | 0 passed, 6 failed (RED) |
| Same, on the first draft | 5 passed, 1 failed; then 6 passed after the bullet fix below |
| Same, on the committed document, run twice | 6 passed, 0 failed, both runs |
| Task 1 gates (draft passes when placed, `not implemented` count 27, `locked` count 0, ledger sha equals draft sha) | all pass |
| Task 3 gates (6 tests ok, document commit subject, byte-equality with the draft record, one ADVANCED line) | all pass |
| `git status --porcelain -- docs xtask/tests` | empty |

## Deviations from the plan

1. **Executed in the orchestrating session**, not by an executor agent (owner's choice after the dispatch guard blocked the main-checkout executor).
2. **Bullet format.** The first draft wrapped bullets across lines; the test checks the bullet's first line, so 15 bullets failed. The document was changed to one line per bullet. The test was not changed.
3. **Protection column header and one sentence** added so present-tense rows are not read as existing behaviour (acceptance criterion: no sentence says a protection exists today).
4. **Revise and approve in one reply.** The plan returns to the checkpoint after a revision. The owner's reply asked for one specific change and approval together, so the revised record was committed and then the document, without a second stop. The owner has not seen the four changed lines rendered; they are listed above.
5. **PROJECT.md and STATE.md edited** (notes N9 to N11 and the note count). These files are not in the plan's `files_modified`; the edit is the owner's instruction.
6. **Draft not printed in full at the checkpoint.** The plan says to print it; the owner was given the path and a summary of the new-marked statements and contradictions instead, because the tables are too wide for a terminal.
7. **Four citation corrections before the first draft commit**, after re-reading the cited passages: the integrity-tag row now cites AC 6.2 for tag slabs; the `factotum` boundary no longer says it "hands the kernel" the secret; the key-exchange options are described as examples; the permissions row matches what PT Tests 1.1 and 1.3 assert.

## Known limits

- The test proves a tag is present on each row, not that the cited section supports the row. Citations were checked by reading; one reader.
- The attacker classes, trust boundaries, and the pairing of protections to attackers and phases are new in this document and have no source.
- The document quotes "only the shadow frame contains plaintext" unchanged although the legacy gateway contradicts it (note N10).
- 02-03 will edit PROJECT.md note N3 and the Key Decisions table; this plan's PROJECT.md change is three added lines after N8 and does not touch those.

## Next

Plans 02-01, 02-03, 02-04, 02-05, 02-06 remain. Set `workflow.use_worktrees` to `false` before `/gsd-execute-phase 2`.
