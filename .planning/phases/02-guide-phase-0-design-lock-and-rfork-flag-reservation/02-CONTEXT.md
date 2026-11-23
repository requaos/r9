# Phase 2: Guide Phase 0 - Design Lock and rfork Flag Reservation - Context

**Gathered:** 2026-11-23
**Status:** Ready for planning

<domain>
## Phase Boundary

The design decisions that Phases 3 to 7 build on are confirmed by the owner and recorded in PROJECT.md; a threat model document and an r9-owned C header for the `rfork` interface are committed; and four `rfork` flag bits (`RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21) are reserved as named constants in r9. Runtime behaviour does not change: `rfork` with any reserved bit fails exactly as it does today.

Requirements: DSGN-01 to DSGN-05. Not in this phase: any flag behaviour (`RFNOLEGACY` is Phase 6, the other three are Phase 7), any crypto or data-structure code (Phase 3), settling warnings W1 to W10, any edit to Harvey.

</domain>

<decisions>
## Implementation Decisions

### Carried in (owner, 2026-11-22; not re-asked)
- **D-01:** Four flags, not three: `RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21. Reserved in r9 only; Harvey is never edited. The guide's "reserve bits 18-20 in `libc.h`" is a recorded deviation.
- **D-02:** Warnings W1 to W10 stay open and are settled in the phase each affects. This phase does not pull any of them forward.

### Locking the guide's thirteen decisions (DSGN-05)
- **D-03:** Confirmation is in bulk with exceptions only. The phase presents all thirteen (IG-D01 to IG-D13) in one table with a proposed status each; the owner confirms the table at one checkpoint and rules individually only on the exceptions below. The result is written to the PROJECT.md Key Decisions table.
- **D-04:** Proposed for lock as written: IG-D01, IG-D02, IG-D04, IG-D05, IG-D06, IG-D08, IG-D10, IG-D12.
- **D-05:** IG-D03 (closed syscall list) is locked with its wording corrected from "3 flag bits" to four, per D-01.
- **D-06:** IG-D07 (kernel safety boundary) is locked amended to what is true at HEAD: `port/` uses per-module `forbid(unsafe_code)` with three named floor modules that hold `unsafe` (`globalalloc`, `mcslock`, `devip_waker`); no new floor module without owner approval. The guide's "crate-level forbid" wording is not locked. How lint and audit scope grow when new `unsafe` lands (note N2) stays for Phases 3 and 5, and the `ds/src/tpht/` part of the wording stays open under W5.
- **D-07:** IG-D09 (TPHT as primary hashing; open at W5, W6), IG-D11 (native `9P2000.aead`; open at W1 to W4) and IG-D13 (cluster discovery; open at W7) are **not locked** in this phase. They remain carried constraints, and the lock is deferred until their W items are settled in Phases 3, 4 and 6. PROJECT.md must record them as "not locked, deferred to Phase N" so the phase gate is explicit about which are locked and which are not.
- **D-08:** "Locked" means owner-only change. Planning and execution treat a locked decision as fixed. If a later phase finds one wrong, work stops and the owner is asked. A change is recorded as a new dated row that supersedes the old one, never a silent edit.

### C prototypes (DSGN-02)
- **D-09:** No source document defines the "corrected C prototypes". The owner's reading: the C statement of the whole `rfork` interface, that is every `RF*` flag (Harvey's frozen values, `RFNOMNT` included, plus the four new ones) and the `rfork()` declaration.
- **D-10:** The landing location is a new top-level `sys/include/` directory, matching the guide's own landing sites (`sys/include/libc.h`, `sys/src/...`), so the file does not move when Phases 9 to 12 fill in `sys/`.
- **D-11:** The file is a small r9-owned `sys/include/rfork.h` holding only the `RF*` enum and the `rfork()` declaration, with a comment naming the Harvey `libc.h` lines it mirrors (`sys/include/libc.h:613-624` for the enum, `:684` for `extern int rfork(int);`). It is not a copy of Harvey's `libc.h` and is not named `libc.h`.
- **D-12:** `rfork.h` is generated, not hand-maintained: an xtask step writes it from the Rust flag constants, and a check fails when the committed file is stale. The Rust constants are the single source of truth.

### Threat model document (DSGN-01)
- **D-13:** The document is `docs/threat-model.md`, beside `docs/devip-contract.md` and `docs/harvey-supply.md`.
- **D-14:** Depth is a full analysis: assets, attacker classes and their capabilities, trust boundaries, and a protection table. It is more than a restatement of AC section 2.2.
- **D-15:** Every statement either cites its source (compendium section, RFC section, guide row) or is visibly marked as new in this document. The in-scope and out-of-scope boundaries are stated exactly as AC section 2.2 / AC-T01 (pages at rest, stale-frame disclosure across KeyIDs, stray kernel reads, packet splicing, injection and replay in scope; DRAM bus probing, cold boot and kernel-context code execution out of scope; SEV-SNP or TDX as the remedy). The owner reviews the draft at a checkpoint before it is committed as final.
- **D-16:** The document is a living document and nothing in it is locked, the boundary included. Later phases may revise it as designs firm up. It is therefore not part of the D-08 lock set. None of its protections exists in code yet, and it must say so.

### Reservation and test shape (DSGN-03, DSGN-04)
- **D-17:** Owner amendment to the roadmap: the `rfork` flags get **one definition in the `port/` crate**, not two arch copies. ROADMAP Phase 2 success criterion 2 and DSGN-03 both name `x86_64/src/proc.rs` and `aarch64/src/syscall.rs` as the home; the plan must amend both texts (and the matching lines in PROJECT.md and STATE.md) to say the constants live in one `port/` module that both arch crates import. This is a `port/src` edit, so the diff is shown to the owner before it is committed.
- **D-18:** All `rfork` flags move: the eleven existing flags (bits 0-6, 10-13) and the four new ones live in the one `port/` module; the duplicate constants are deleted from `x86_64/src/proc.rs:171-181` and `aarch64/src/syscall.rs:92-108`, which import them. No re-export shims.
- **D-19:** `RFNOMNT` = 1<<14 is added to the same module as a named constant that stays outside both `SUPPORTED` masks, exactly like the four reserved flags. `rfork(RFNOMNT)` keeps failing as it does today. The module therefore defines sixteen flags.
- **D-20:** "Behaviour unchanged" is pinned by a host test: none of the four reserved bits (nor `RFNOMNT`) is in either `sys_rfork` `SUPPORTED` mask, so `rfork` with any of them still returns "rfork: unsupported flags". Phases 6 and 7 must change that test on purpose when they make a flag live. No live boot check is added for this.
- **D-21:** The host test of DSGN-04 also pins the four values and proves they overlap no existing flag, with bit 14 covered through the real `RFNOMNT` constant.
- **D-22:** Kernel bytes follow the standing rules: compare both kernels' sha256 before and after the move. Moving constants is expected to change nothing; if x86_64 bytes do change, the `boot-iso` `outputHash`, amd64-container TCG gates and owner linux check apply, and an aarch64 change gets the quick pass.

### Decisions after research (owner, 2026-11-23; source: `02-RESEARCH.md`, "Kernel Bytes" and "Open Questions")
- **D-23:** Supersedes the expectation in D-22. Research measured that shifting source lines changes x86_64 kernel bytes (panic line numbers in `.rodata`; `.text` identical in the experiment) and the aarch64 ELF (debug line tables). The owner chose to accept the change: edit the kernel sources naturally, with no line-count balancing, and run the full x86_64 byte-change procedure as planned work in this phase (re-derive and commit the `boot-iso` `outputHash` on the owner's x86_64 Linux host, the x64 TCG gates, the owner's linux boot-iso check). aarch64 gets the quick pass. The before/after sha256 of both kernels and of their `.text` sections is still recorded as evidence; a `.text` difference is a stop, because the phase must change no executable code.
- **D-24:** The x86_64 host test that the reserved bits are outside x86_64's supported mask is witnessed on the owner's x86_64 Linux host, at the same stop as the KVM gate. If the devshell skips the x86_64 host tests there too, the plan includes the small xtask fix that makes them run. Hosted CI is not the witness and no push is required for verification.
- **D-25:** The boot-to-rio gate (`cargo xtask gate`, owner's x86_64 Linux KVM host) runs once, at the end of the phase, after all work is committed. No per-commit or mid-phase gate run in this phase.

### Claude's Discretion
- Name and file of the `port/` module that holds the flags.
- How the generator reads the Rust constants (xtask depending on `port`, or another route) and the name of the xtask step and its staleness check; whether the check runs inside `cargo xtask check` or `test`.
- Where the host tests sit (the `port` crate for values and overlap; the arch crates for the `SUPPORTED` masks, which stay where they are).
- Layout and section order of `docs/threat-model.md`, and the form of the "new in this document" mark.
- The layout of the thirteen-decision table presented at the owner checkpoint, and the order of work within the phase.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` (Phase 2 section, line 141) — goal, five success criteria, the W11 note; criterion 2 is amended by D-17
- `.planning/REQUIREMENTS.md` (DSGN-01 to DSGN-05, lines 25-29) — the phase requirements; DSGN-03 is amended by D-17
- `.planning/PROJECT.md` — Key Decisions table (where the locks are recorded), Constraints, Open Questions W1-W10, roadmapper notes N2 and N3
- `.planning/STATE.md` — standing owner rules (push approval, `port/src` diff approval, kernel byte-change procedures)

### Source documents for the design lock
- `.planning/r9-implementation-guide-amended-v3.md` §2 — the "Reconciled Decisions" table (IG-D01 to IG-D13 in order); §3 the 17-correction audit ledger; the phase table row for Phase 0 (line 258)
- `.planning/intel/constraints.md` (lines 17-80) — IG-D01 to IG-D13 as carried constraints, with source wording
- `.planning/intel/decisions.md` — why nothing was locked at ingest
- `.planning/INGEST-CONFLICTS.md` — detail of W1 to W11, which decide what D-07 defers

### Threat model sources
- `.planning/r9-architecture-and-code-v4.md` §2.2 (lines 84-88) — the threat model boundaries, the text D-15 keeps exact
- `.planning/intel/constraints.md` (AC-T01, line 211) — the same boundaries as an extracted constraint
- `.planning/rfc-9p2000-aead.md` — the wire protections (splicing, injection, replay) the analysis must cite
- `.planning/r9-architecture-and-code-v4.md` §1.1 (line 35) — the false claim that bits 18-20 are already reserved (note N3)

### rfork flags and header
- `x86_64/src/proc.rs` (lines 167-181) — current x86_64 flag constants
- `aarch64/src/syscall.rs` (lines 87-108) — current aarch64 flag constants
- `x86_64/src/syscall.rs` (lines 1643-1672) — `sys_rfork`, the `SUPPORTED` mask and the "unsupported flags" rejection
- `aarch64/src/syscall.rs` (lines 1961-1980) — the aarch64 `SUPPORTED` mask
- `../harvey/sys/include/libc.h` (lines 613-624, 684) — the frozen `RF*` values and the `rfork` declaration; read-only reference, never edited

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `xtask/src/uregcheck.rs` — an existing xtask check that compares r9 Rust against a C header; a model for the `rfork.h` staleness check.
- `port/` host tests (`port/src/*_tests.rs`) — the port crate already builds and tests on the host, so flag-value tests can live there.
- `#[cfg(test)]` modules already exist in `x86_64/src/proc.rs` and `aarch64/src/syscall.rs` for arch-side pins.

### Established Patterns
- Both `sys_rfork` copies mask the flag to 32 bits and reject any bit outside a local `SUPPORTED` constant. Leaving the new constants out of those masks is what keeps behaviour unchanged.
- Flag values are documented as "Harvey `sys/include/libc.h` — frozen values" in both arch files; the new module keeps that provenance comment.
- Kernel builds are reproducible, so a sha256 comparison proves a source move changed no bytes.
- `config.json` sets the build and test commands: `nix develop -c cargo xtask check` and `nix develop -c cargo xtask test`.

### Integration Points
- `x86_64/src/syscall.rs:92` imports the flags from `proc`; that import and the definitions in `x86_64/src/proc.rs` and `aarch64/src/syscall.rs` change to the `port/` module. `aarch64/src/proc.rs` also reads several flags through `syscall.rs`.
- `sys/include/` and `docs/threat-model.md` are new; r9 has no `sys/` directory today.
- PROJECT.md Key Decisions table currently marks IG decisions as "Carried constraint; confirm in Phase 2"; those rows become locked, amended, or deferred.

</code_context>

<specifics>
## Specific Ideas

- The owner wants `rfork.h` to be a complete, self-standing statement of the `rfork` interface, not only the four new flags.
- The owner prefers one list of flags with no copies over keeping the roadmap's two named arch files.
- Decisions with open details are left unlocked on purpose; a partial "direction only" lock was offered and declined.

</specifics>

<deferred>
## Deferred Ideas

- **Implement `RFNOMNT` (bit 14) in r9's kernel.** The owner asked for it during the discussion. It is a kernel behaviour change, which this phase excludes, so it needs its own phase or a place in an existing one. D-19 leaves the constant defined so the implementing phase only adds it to `SUPPORTED` and writes the behaviour.
- Locking IG-D09, IG-D11 and IG-D13: deferred by D-07 to Phases 3, 4 and 6 respectively, once W5/W6, W1-W4 and W7 are settled.

</deferred>

---

*Phase: 2-Guide Phase 0 - Design Lock and rfork Flag Reservation*
*Context gathered: 2026-11-23*
