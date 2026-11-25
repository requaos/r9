# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v3.0 — Cross-Build Harness and Wire Foundations

**Shipped:** 2026-11-25
**Phases:** 4 | **Plans:** 78 | **Tasks:** 180 | **Sessions:** not instrumented

### What Was Built

- An r9-owned cross-build harness: `cargo xtask gate` boots to rio on q35 from a read-only Harvey tree, with per-arch fallback manifests, two-way drift/provenance checks, and every composed gate moved out of Harvey's flake.
- The design lock: a committed threat model and all sixteen `rfork` flags in one `port::rfork` module, with `sys/include/rfork.h` generated and drift-checked.
- `crypto/` and `ds/`: HKDF, ChaCha20, keyed BLAKE3, the key hierarchy, TPHT with its seqlock and spill threshold, Roaring and `ClientSet`, all safe code under lint contracts and pinned by RFC vectors, independent Python KATs and Miri.
- `9P2000.aead`: the 28-byte MAC'd header, per-connection per-direction keys, in-band re-key with strict cutover, parameter-plus-page ciphertext bodies, six extension bits with the stateless-capability TLV, and the hybrid handle on TPHT with a failover sweep.
- Review-driven gap closure: 12 blocking review findings fixed and pinned, closing evidence re-taken from a from-nothing build, and a second owner stop with eleven PASS verdicts.

### What Worked

- **Returned-log verdicts.** Every owner-stop step ran through `run_logged`, so each log began with `HEAD=` and ended with `EXIT=`, and every verdict was decided from a specific line. This made a 585-commit milestone auditable after the fact.
- **Coverage blocks in SUMMARY frontmatter.** They deterministically auto-passed 91 of 112 Phase 4 UAT checkpoints, leaving 21 genuine owner judgements instead of 112 prompts.
- **Two independent reviewers.** The code review and the verifier agreed on the same Phase 4 blocking gaps, which made the stop-and-defer decision (then the fix loop) unambiguous.
- **RED/GREEN evidence ledgers.** `PINS-*` and `RESULTS.md` with `VERDICT` lines made the milestone audit's 3-source requirement cross-reference mechanical.

### What Was Inefficient

- **Phase 1 took 34 plans and two planning passes.** The rio de-risk needed stops A, A2, B, B3, C and E; the Miri suite grew to 230–402 s against a 120 s target.
- **The requirements file was roadmap-wide.** The standard close assumes milestone-scoped requirements, so this close needed a custom split to keep 89 future requirements live.
- **Verification staleness at the boundary.** Later phases editing shared files (`flake.nix`, `xtask`, `x86_64/src/syscall.rs`) marked earlier phases `stale`, requiring an explicit override at close.
- **The boot-iso expectation drifted from the evidence.** The second stop page said the hash must change; the returned logs built and rebuilt green on the unchanged hash. The audit caught it, not the phase.
- **UAT session completion raced its own last row.** The first completion ran before the final `pass` was persisted, producing a `partial` status and a second commit.

### Patterns Established

- One `test-9p` leg per listed `port/tests` file, each with a scalar-backend twin, plus a preflight and a zero-passed-result guard.
- Owner-stop pages that name the host, the gate venue, the hash commit rule, and a returned-log verdict table with `HEAD=` stamps.
- `coverage:` blocks in plan summaries: each deliverable names its covering tests, and `uat classify-coverage` turns that into auto-passed vs human checkpoints.
- Evidence sets named by phase (`@04-14`, `@04-24`) kept beside a page that marks the superseded set.

### Key Lessons

1. **Any kernel-byte commit re-stales the `boot-iso` fixed-output hash.** Budget the re-derive at phase close and state explicitly in the stop page whether the hash must change; then make the verdict read that line.
2. **Wire a new test command into CI in the phase that creates it.** `test-9p` and `cargo test -p xtask` are manual-only, so the scalar twins and the threat-model guards depend on someone remembering.
3. **Keep requirements milestone-scoped, or plan the split at close.** A roadmap-wide requirements file makes the standard archive step destructive.
4. **Persist a UAT row before completing the session.** A completion that runs ahead of the last write flips the report to `partial` and costs a second commit.
5. **A by-design limit must be stated in the requirement's own terms.** The Aead engine is foundation-only (D-16/D-23); saying so up front prevented it from reading as a gap at the audit.

### Cost Observations

- Model mix: not instrumented; the planner and plan-checker were pinned to `fable` by project config, and the integration checker ran on the orchestrator model after the configured sonnet alias was unavailable.
- Sessions: not instrumented.
- Notable: four calendar days for 585 commits and 271k insertions; the expensive parts were owner stops and Miri, not planning.

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v3.0 | not instrumented | 4 | First milestone with structured `coverage:` blocks, a security review, a code review, and a milestone audit |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v3.0 | 14 wire legs × 2 backends plus the host suites; Miri over 64 seeds | Phase UAT 112/112, 3/3, 3/3; requirements 38/38 | `crypto/` and `ds/` add only pinned crates |

### Top Lessons (Verified Across Milestones)

1. Kernel-byte work needs a named venue and returned logs; a Mac-only claim does not survive the next phase.
2. Evidence that is not mechanically checkable drifts from the claim it was meant to support.
