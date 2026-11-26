---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 03
subsystem: safety
tags: [unsafe, clippy, undocumented_unsafe_blocks, aarch64, x86_64, safety-comments, corr-05]

requires:
  - phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools plan 01
    provides: green CI harness and pure boot-iso (CORR-01/CORR-02) that later Phase 5 plans inherit
provides:
  - aarch64 virtshell clippy 100% green (0 undocumented unsafe blocks; was 7)
  - 24 of 32 x86_64 undocumented unsafe blocks documented (allocator, cpu, dat, pio, seg)
  - SMP-03 audit markers on the proc.rs sites whose invariants rest on single-CPU IRQ-masked execution
affects: [phase-05 plan 04 (remaining 8 x86_64 sites, lint opt-in, UNSAFE-AUDIT.md sync), smp-03 audits]

actuals:
  tokens: 4707
  tasks: 2
  commits: 2
  plan_head_before: bf9e7d0eed4c0dc657cec06ba5bf2fad2bf9fd20
  plan_head_after: 4d5ec89c64f6f786d2debf4d2ab9243537e74654

tech-stack:
  added: []
  patterns:
    - "clippy::undocumented_unsafe_blocks requires the `// SAFETY:` prefix verbatim in the contiguous comment block immediately above the unsafe block — a prose comment mentioning SAFETY mid-paragraph (proc.rs:1823) or a parenthesized variant 'SAFETY (both map_to calls):' (proc.rs:2753) does NOT count"
    - "for cfg-gated unsafe statements (`#[cfg(not(test))] unsafe { ... }` in pio.rs), the SAFETY comment goes between the attribute and the unsafe keyword — the attribute line breaks clippy's comment walk"
    - "single-CPU-dependent unsafe invariants are annotated '(an SMP-03 audit site)' so the SMP-03 sweep finds them mechanically"

key-files:
  created: []
  modified:
    - aarch64/src/platform/virt/rtc.rs
    - aarch64/src/proc.rs
    - aarch64/src/textcopy.rs
    - x86_64/src/allocator.rs
    - x86_64/src/cpu.rs
    - x86_64/src/dat.rs
    - x86_64/src/pio.rs
    - x86_64/src/seg.rs

key-decisions:
  - "proc.rs:1823's SAFETY justification moved from a mid-paragraph prose comment to a dedicated comment immediately above the unsafe block (lint mechanics; justification text preserved verbatim)"
  - "proc.rs:2753's 'SAFETY (both map_to calls):' reworded to the lint-recognized 'SAFETY:' prefix, content unchanged"
  - "pio.rs SAFETY comments sit between the #[cfg(not(test))] attribute and the unsafe block — the only position clippy's comment walk accepts on a cfg-gated statement"

patterns-established:
  - "x86_64 privileged-instruction SAFETY shape: CPL 0 precondition + operand/clobber contract + no-memory-touched note (reusable by plan 05-04 for trap.rs/main.rs)"

requirements-completed: []
requirements-progress:
  - "CORR-05 Part A complete (this plan): 7/7 aarch64 sites documented and virtshell clippy green; 24/32 x86_64 sites documented. Part B lands in plan 05-04 (remaining 8 x86_64 sites, [lints] workspace = true opt-in, UNSAFE-AUDIT.md reconciliation, policy docs) — CORR-05 is marked complete there, not here."

coverage:
  - id: D1
    description: "All 7 undocumented unsafe blocks in aarch64/src/{platform/virt/rtc.rs,proc.rs,textcopy.rs} carry concrete // SAFETY: comments and virtshell clippy passes cleanly"
    requirement: CORR-05
    verification:
      - kind: other
        ref: "nix develop -c cargo xtask clippy --arch aarch64 --config virtshell → exit 0, zero undocumented_unsafe_blocks diagnostics (baseline before edits: exit 1 with exactly 7 errors at the plan's named lines)"
        status: pass
    human_judgment: false
  - id: D2
    description: "All 24 undocumented unsafe blocks across x86_64/src/{allocator.rs,cpu.rs,dat.rs,pio.rs,seg.rs} carry concrete // SAFETY: comments"
    requirement: CORR-05
    verification:
      - kind: other
        ref: "nix develop -c cargo clippy -p x86_64 --bins -- -D clippy::undocumented_unsafe_blocks → the 24 targeted sites no longer flagged; exactly the 8 plan-05-04 sites remain (trap.rs:69,136; uart16550.rs:14; proc.rs:852,873; main.rs:131,338,374)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Comment-only edits; no runtime semantics changed"
    requirement: CORR-05
    verification:
      - kind: other
        ref: "nix develop -c cargo xtask check → exit 0; git diff shows 92 insertions / 0 deletions in x86_64 and +35/-10 comment lines in aarch64, every added line a // comment"
        status: pass
    human_judgment: false

duration: 6min
completed: 2026-11-26
status: complete
---

# Phase 05 Plan 03: Unsafe Policy — Document 7 aarch64 + 24 x86_64 Unsafe Blocks (CORR-05 Part A) Summary

**Every one of the 7 undocumented `unsafe` blocks in `aarch64/` and the first 24 of 32 in `x86_64/` now carries a concrete `// SAFETY:` justification — `cargo xtask clippy --arch aarch64 --config virtshell` is fully green, and `x86_64/` is ready for the workspace lint opt-in in plan 05-04.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-11-26T04:29:10Z
- **Completed:** 2026-11-26T04:35:30Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- **Task 1 (aarch64, 7 sites):** `rtc.rs:75` (PL031 MMIO read: mapped rw_device page, page-aligned base plus 4-byte register offset, volatile load contract); `proc.rs:439` and `:470` (brk-growth failure unwinds: PROC_TABLE-resident member pointers, exclusive `&mut` under single-CPU IRQ-masked syscall context, per-write TLB invalidate — both annotated as SMP-03 audit sites); `proc.rs:1823` (parent-Proc deref — the SAFETY text existed but sat mid-paragraph lines above the block where the lint cannot see it; moved to the site); `proc.rs:2753` (rfork sweep map_to pair — reworded `SAFETY (both map_to calls):` to the lint-recognized `SAFETY:` prefix); `textcopy.rs:158` (selftest parent-page emit through the KZERO alias) and `:172` (control-arm `isb` — context-synchronization event, no operands/clobbers).
- **Task 2 (x86_64, 24 sites):** `allocator.rs:12` (static heap arena Block: valid pointer, exclusive one-time static init, GlobalHeap alignment); `cpu.rs` 13 sites (`flags`, `sti`, `cli`, `ltr`, `lgdt`, `lidt`, `_rdmsr`, `cpuid`, `rdtsc`, `rdrand`, `rdseed`, `wrmsr`, `wrgsbase` — CPL 0 preconditions plus per-instruction operand/clobber/memory contracts); `dat.rs:433` (Mach GDT/IDT/TSS loads on the exclusively borrowed, initialized Mach); `pio.rs` 6 sites (CPL 0 port-I/O invariants for `inb`/`inw`/`inl`/`outb`/`outw`/`outl`, comments placed between the `#[cfg(not(test))]` attribute and the unsafe block — the only position the lint accepts there); `seg.rs` 3 sites (GDT/IDT/TSS `load`: architecturally valid tables per the fn contracts, CPL 0, `ltr`'s busy-bit write lands in the live GDT).

## Task Commits

Each task was committed atomically:

1. **Task 1: Document all 7 undocumented unsafe blocks in aarch64/ (CORR-05)** — `832382f` (fix)
2. **Task 2: Document 24 undocumented unsafe blocks in x86_64 core files (CORR-05)** — `4d5ec89` (docs)

**Plan metadata:** this SUMMARY commit (docs: complete plan)

## Files Created/Modified

- `aarch64/src/platform/virt/rtc.rs` — +6 lines (SAFETY comment at the RTCDR volatile read)
- `aarch64/src/proc.rs` — 4 sites documented (+21/-8 lines: two inserted comments, one moved, one reworded)
- `aarch64/src/textcopy.rs` — 2 sites documented (+8 lines)
- `x86_64/src/allocator.rs` — 1 site (+6 lines)
- `x86_64/src/cpu.rs` — 13 sites (+51 lines)
- `x86_64/src/dat.rs` — 1 site (+6 lines)
- `x86_64/src/pio.rs` — 6 sites (+18 lines)
- `x86_64/src/seg.rs` — 3 sites (+11 lines)

## Decisions Made

- **Lint mechanics drove two micro-fixes beyond bare insertion:** clippy's `undocumented_unsafe_blocks` only recognizes a `// SAFETY:`-prefixed comment in the contiguous comment block immediately above the unsafe token. `proc.rs:1823`'s existing justification (embedded mid-paragraph) and `proc.rs:2753`'s `SAFETY (both map_to calls):` variant were invisible to it; both were re-homed/re-prefixed with their content preserved.
- **pio.rs comment placement:** on a cfg-gated statement the attribute line breaks clippy's backward comment walk, so each SAFETY comment sits between `#[cfg(not(test))]` and `unsafe {`.
- **SMP-03 markers:** the three `proc.rs` sites whose invariants rest on single-CPU IRQ-masked execution say so explicitly (`an SMP-03 audit site`), per the plan's instruction that the SMP-03 audit catch them accurately.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- The harness's worktree path guard rejected absolute edit paths (the hook cwd named a sibling worktree); switched to relative paths per `worktree-path-safety.md`. No plan impact.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Comment-only edits; no code, data flow, or UI touched.

## Threat Flags

None. No new network endpoints, auth paths, file access patterns, or schema changes — comment-only edits. T-05-05's mitigation (every unsafe precondition documented) is exactly what this plan delivers; T-05-SC is N/A (`Cargo.lock` untouched, zero packages installed).

## Next Phase Readiness

- Plan 05-04 can proceed directly: the remaining 8 x86_64 sites (`trap.rs:69,136`, `uart16550.rs:14`, `proc.rs:852,873`, `main.rs:131,338,374`) are the only `undocumented_unsafe_blocks` diagnostics left under the forced lint, so the `[lints] workspace = true` opt-in there will turn green the moment those are documented.
- The pre-existing `aarch64` clippy failure recorded in STATE.md's deferred items is cleared at this commit (`UNSAFE-AUDIT.md` count reconciliation still lands in 05-04 per the plan).
- Kernel byte-change procedure (D-05): comment-only edits leave `.text` unchanged (line metadata only); with CORR-02 landed in 05-01, green CI on the commit is the check.

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-26*

## Self-Check: PASSED

- All 9 plan files (SUMMARY + 8 modified source files) exist on disk.
- Both task commits (`832382f`, `4d5ec89`) are ancestors of HEAD.
- Verifications re-run at HEAD: `cargo xtask clippy --arch aarch64 --config virtshell` exit 0 with zero `undocumented_unsafe_blocks` diagnostics; `cargo xtask check` exit 0; forced-lint x86_64 clippy flags exactly the 8 plan-05-04 sites and none of this plan's 24.
- Measured commits from ledger: 2 task commits (this SUMMARY commit is the third).
- Working tree carries no untracked files; Harvey untouched.
