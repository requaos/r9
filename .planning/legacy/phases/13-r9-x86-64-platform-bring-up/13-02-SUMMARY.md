---
phase: 13-r9-x86-64-platform-bring-up
plan: 02
subsystem: kernel
tags: [x86_64, q35, multiboot2, mb2-info-structure, parser, pagealloc, platform-tree, old-gen-retirement, boot-test, markers, l.S]

# Dependency graph
requires:
  - phase: 13-r9-x86-64-platform-bring-up (13-01)
    provides: the q35 SeaBIOS -> GRUB2 -> mb2 boot channel with the banner gate, the mb2 tagged header in l.S, and the %rsi mb2-info hand-off this plan's kernel half consumes
provides:
  - The main9-x64 kernel arc on x86_64 — mb2 info parsed bounds-checked (fdt.rs discipline), page allocator seeded from type-1 ranges only, console via platform/q35/devcons, banner + one q35: fact line per mb2 range + the ACPI root address, clean masked-IRQ park
  - x86_64::platform tree (mod.rs cfg dispatch; q35/{mod,mb2,devcons}.rs) — the orchestrator slot (print_machine_info) later plans extend (acpi 13-04, lapic/ioapic/timer 13-03, pci 13-04/05)
  - The l.S mb2-info .bss stash (GRUB 2.14 lands the structure inside the Mach-zeroing mebibyte) and the boot-PML4 execution model (node0's cr3 switch retired; identity map stays live)
  - The old-generation x86_64 locals retired (proc/syscall/node0/vsvm/devcons gone; Gdt/Idt/Tss relocated to seg.rs; dat.rs untangled)
  - The q35 boot-test gate's extended marker set: banner then >= 1 q35: memory fact line, order-anchored past the banner byte offset
affects: [13-03, 13-04, 13-05, 13-06, "Phase 14 XEXEC (syscall/star substrate kept with doc'd allows)"]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 19957   # chars/4 over git diff b3fe837..b964a62 in ../r9 (79829 chars; plan estimated 58000, confidence low — the overage bought the l.S stash + seg relocation the plan did not scope)
  tasks: 2
  commits: 1      # MEASURED harvey-repo docs commit per the plan ledger (ledger b45adf572 unchanged before it). Production code: 3 commits in the sibling r9 repo (e9d3f24 RED, d8d14db GREEN/Task 1, b964a62 Task 2) per the dual-repo execution context.
plan_head_before: b45adf572a6a9ba3d252f9bf02e42f3f5a009590
plan_head_after: b45adf572a6a9ba3d252f9bf02e42f3f5a009590   # harvey-side: all production commits live in ../r9 (b3fe837..b964a62)

# Tech tracking
tech-stack:
  added: []        # zero new crates — the from-scratch discipline holds
  patterns:
    - "Firmware-data stash: early asm copies the loader-handed structure into .bss BEFORE the boot-time zeroing of the region it may occupy — the pointer GRUB hands can name memory the kernel itself is about to reclaim"
    - "Boot-PML4 execution: the port-based system runs on l.S's hand-crafted tables with the identity map live until the port-side vm layer exists; no node0-style cr3 switch in between"
    - "Ordered-presence marker anchoring: marker_position() anchors a stage's marker count strictly past the previous marker's byte offset, so out-of-order or pre-banner lines never satisfy the gate"

key-files:
  created:
    - ../r9/x86_64/src/platform/mod.rs
    - ../r9/x86_64/src/platform/q35/mod.rs
    - ../r9/x86_64/src/platform/q35/mb2.rs
    - ../r9/x86_64/src/platform/q35/devcons.rs
    - ../r9/x86_64/src/pagealloc.rs
    - ../r9/x86_64/src/seg.rs
  modified:
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/src/dat.rs
    - ../r9/x86_64/src/cpu.rs
    - ../r9/x86_64/src/pio.rs
    - ../r9/x86_64/src/l.S
    - ../r9/x86_64/Cargo.toml
    - ../r9/xtask/src/main.rs
  deleted:
    - ../r9/x86_64/src/proc.rs
    - ../r9/x86_64/src/syscall.rs
    - ../r9/x86_64/src/node0.rs
    - ../r9/x86_64/src/vsvm.rs
    - ../r9/x86_64/src/devcons.rs

key-decisions:
  - "GRUB 2.14's mb2 info structure landed at 0x16dc08 — INSIDE the second mebibyte l.S zeroes for the Mach (hex-dump verified live): l.S now stashes the structure into a .bss buffer (clamped to 4 KiB) BEFORE the zeroing, and warp64 passes the stash address; parsing the original pointer would read zeroed memory"
  - "warp64 keeps the boot PML4 (identity map of the first 4 GiB live) — node0's init0/cr3-switch retirement means the port-based kernel runs on the hand-crafted tables until a later plan's vm layer rebuilds the proper map; the mb2 stash and Mach stay reachable thereby"
  - "mb2 tag SIZES are legitimately non-8-multiples (live: empty-cmdline size 9, boot-loader-name 18, APM 28) — the parser validates tag positions/total_size/fixed-body bounds and walks 8-aligned per spec; a strict size%8 error would reject GRUB's own structure and break the banner gate (Test 3 re-keyed accordingly, live shape PINNED as a test)"
  - "The mb2 ACPI tags 14/15 embed the RSDP structure ITSELF (spec 3.9.6/3.9.7), not a pointer — a naive pointer read yields the 'RSD ' signature bytes (observed live); the parser signature-validates and extracts the RSDT/XSDT address (live: q35: acpi rsdt 0x3ffe2345 via tag 14 on BIOS boots), handing 13-04 the root table with no EBDA scan"
  - "The q35 gate's fact assertion is order-anchored: >= 1 'q35: memory' line counted strictly past the banner's byte offset (marker_position anchor), presence-not-timing, negative panic scan on"

patterns-established:
  - "Host-testable firmware parsers in x86_64: #[cfg(test)] modules over synthetic byte fixtures run via cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu (the crate's default none-target otherwise blocks host tests)"
  - "Page-allocator seeding as the exact type-1 union: sorted available ranges form the span; the gap complement plus the kernel-image range is the used set — type-3/2/4/5 and undeclared holes never enter the free pool"

requirements-completed: [X64-01]

# Coverage metadata (#1602) — one entry per shipped deliverable.
coverage:
  - id: D1
    description: "The mb2 info-structure parser in the port/fdt.rs discipline (typed ParseError, checked extraction, two-pass total_size, aligned tag walk, validated mmap, signature-validated ACPI root)"
    requirement: X64-01
    verification:
      - kind: unit
        ref: "cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu mb2: golden_memory_map_parses_ranges_with_types, end_tag_stops_walk_and_cut_total_size_is_loud, alignment_and_header_violations_are_loud, acpi_tags_yield_root_table_and_absence_yields_none, every_truncation_errors_loudly (5 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The platform/q35 tree: cfg dispatch in platform/mod.rs (q35 + not(q35) host-test arm), print_machine_info (one q35: fact line per range + the acpi root, loud absence), devcons (COM1 16550 with tolerant scratch-register probe via pio::inb)"
    requirement: X64-01
    verification:
      - kind: integration
        ref: "live serial (13-02-evidence-q35-serial.log): 9 q35: memory lines + q35: acpi rsdt 0x3ffe2345 after the banner; gate green"
        status: pass
    human_judgment: false
  - id: D3
    description: "The main9-x64 rewrite: l.S mb2 stash before Mach zeroing + warp64 on the boot PML4; main parses the stash, seeds pagealloc from type-1 only (Used 0x800000 / Total 0x3ffdf000 live), fills and loads the Mach GDT/IDT/TSS, banner + facts + masked-IRQ park"
    requirement: X64-01
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch x86-64 --config q35: PASS — banner marker then >= 1 q35 memory-map fact line observed in order (multiple consecutive runs)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Old-gen retirement with every leg green: proc/syscall/node0/vsvm/devcons deleted, Gdt/Idt/Tss relocated to seg.rs, dat.rs vsvm re-export gone, toy swtch/jumpback gone; ureg-check 35 fields, aarch64 virt boot-test, xtask test (101 port + 83 xtask), check, clippy all green"
    requirement: X64-01
    verification:
      - kind: integration
        ref: "acceptance greps (all pass: vsvm count 0 in dat.rs, no swtch/jumpback in main.rs, retired files absent) + the five workspace legs re-run on the final tree"
        status: pass
    human_judgment: false
  - id: D5
    description: "The q35 gate's extended marker set: BootStage::Q35Facts asserting >= 1 'q35: memory' line strictly past the banner byte offset (marker_position anchor), presence+order only"
    requirement: X64-01
    verification:
      - kind: unit
        ref: "xtask test q35_memory_facts_count_only_after_the_banner (83 passed total)"
        status: pass
      - kind: e2e
        ref: "gate run PASS + grep -c 'q35:' target/boot-test-q35.log >= 1"
        status: pass
    human_judgment: false

# Metrics
duration: 45 min
completed: 2026-11-07
status: complete
---

# Phase 13 Plan 02: r9 x86_64 Platform Bring-Up Summary

**The main9-x64 rewrite boots the q35 kernel from GRUB's mb2 data — a bounds-checked parser in the fdt.rs discipline, the page allocator seeded from type-1 ranges only, the platform/q35 tree printing one loud fact line per range plus the ACPI root address, and the old-generation locals (proc/syscall/node0/vsvm) retired — every commit still green on the 13-01 channel, now gated by an ordered banner-to-memory-map marker set.**

## Performance

- **Duration:** 45 min
- **Started:** 2026-11-07T00:24:34Z
- **Completed:** 2026-11-07T01:09:42Z
- **Tasks:** 2 (Task 1 tracer + Task 2 auto/tdd)
- **Files modified:** 16 in ../r9 (7 created, 7 modified, 5 deleted — the vsvm->seg rename counts both ways) + this plan's harvey docs

## Accomplishments
- The kernel half of X64-01's DTB-free protocol: the mb2 info structure is parsed bounds-checked at boot and EVERY fact is printed loud on serial (9 memory ranges + the ACPI root on the live channel) — loud-by-absence holds for the RSDP tags too
- The ground-truth dump (uncommitted scratch payload) settled three spec-vs-reality questions before any parser line was written: GRUB 2.14 emits non-8-multiple tag sizes (9/18/28), places the info structure inside the Mach-zeroing mebibyte (0x16dc08), and embeds the RSDP structure (not a pointer) in tags 14/15
- The page allocator is seeded ONLY from mb2 type-1 entries (live: Used 0x800000 / Total 0x3ffdf000 — the free set is exactly the type-1 union minus the kernel image); type-3 ACPI-reclaimable ranges stay reserved for 13-04
- The old generation is gone (proc/syscall/node0/vsvm/devcons; the swtch/jumpback toy) with Gdt/Idt/Tss relocated to seg.rs and the Mach's descriptor tables filled+loaded from main — and all five workspace legs stayed green through every retirement step
- The q35 boot-test gate grew its kernel-half assertion: banner, then the memory-map facts, order-anchored and timing-tolerant

## Task Commits

Each task was committed atomically (r9 repo, `main`), in TDD order:

1. **Task 2's RED (the mb2 behavior tests, written first)** - `e9d3f24` (test)
2. **Task 1: main9-x64 rewrite + platform/q35 tree + mb2 parse + old-gen retired (GREEN)** - `d8d14db` (feat)
3. **Task 2: mb2 parser host tests landed green + q35 gate marker set** - `b964a62` (feat)

**Plan metadata:** (this harvey docs commit — see below)

## TDD Gate Compliance

RED -> GREEN -> REFACTOR executed per the dispatch's TDD-applicable contract, re-sequenced so the tests precede the implementation (the plan's task order — Task 1 implements the parser, Task 2 tests it — inverted would make RED impossible; the test(13-02) commit e9d3f24 precedes and is an ancestor of both feat(13-02) commits):
- **RED:** all five behavior tests failed on their planned assertions against the loud stub (target `platform::q35::mb2::tests::golden_memory_map_parses_ranges_with_types` panicked at the planned golden-parse expect); evidence record with the real command/exit/output at `13-02-red-evidence.json` in this directory. Rust's libtest console has no classifier adapter and `workflow.tdd_mode` is false in config — the record is on disk for review rather than machine-classified (the documented Rust gap, tdd.md #4379)
- **GREEN:** the real parser landed with Task 1 (commit d8d14db); `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu mb2` — 5 passed, 0 failed
- **REFACTOR:** not needed — the GREEN tree is clippy-clean in every new file (the two is_multiple_of and module_inception lints were fixed inline before commit)

## Files Created/Modified
- `../r9/x86_64/src/platform/q35/mb2.rs` - the parser: ParseError (9 named variants), MemKind/MemRange/Facts/AcpiRoot, two-pass header_total_size + parse, aligned tag walk, validated mmap + ACPI-root extraction; the five #[cfg(test)] behavior tests
- `../r9/x86_64/src/platform/q35/mod.rs` - print_machine_info: one `q35: ` fact line per range + the acpi root (loud absence print for missing RSDP tags)
- `../r9/x86_64/src/platform/q35/devcons.rs` - COM1 16550 console registration, tolerant scratch-register probe
- `../r9/x86_64/src/platform/mod.rs` - cfg dispatch (q35 + the not(q35) host-test arm, the aarch64 shape re-keyed for one board)
- `../r9/x86_64/src/pagealloc.rs` - BitmapPageAlloc<32, 4K> seeded as the exact type-1 union minus the kernel image (span + gap-complement, heap-less)
- `../r9/x86_64/src/main.rs` - the main9-x64 arc: mb2 stash parse (bounds vs the stash size), pagealloc seed, console, banner, facts, memory-usage print, masked-IRQ park; the Mb2InfoStash + KZERO/kernel_phys_end
- `../r9/x86_64/src/seg.rs` - Gdt/Idt/Tss relocated from vsvm.rs (verbatim modulo the retired init dance + the new Gdt::init; inner `seg` module renamed `descr` to satisfy clippy's module_inception)
- `../r9/x86_64/src/l.S` - the mb2 stash copy (clamped, before the Mach zeroing) + warp64 on the boot PML4 (init0 call + cr3 switch removed)
- `../r9/x86_64/src/dat.rs` - `pub use crate::seg::{Gdt, Idt, Tss}` (vsvm untangled); idt/gdt pub (main fills them); node0's HPA/KMACHTSS vocabulary removed
- `../r9/x86_64/src/cpu.rs` / `pio.rs` - pio::inb added (the file's documented extension shape); doc'd allows on the retirement-orphaned wrmsr/wrgsbase/fmask/MSR_KERNEL_GS_BASE (13-03/14 substrate)
- `../r9/x86_64/Cargo.toml` - crate-local check-cfg predeclare for platform="q35" (the crate is not workspace-lint-opted-in)
- `../r9/xtask/src/main.rs` - BootStage::Q35Facts + marker_position anchor + BOOT_Q35_MEMORY_MARKER + the X64 profile/doc/PASS-message updates + the anchor unit test
- Deleted: `../r9/x86_64/src/{proc,syscall,node0,vsvm,devcons}.rs`

## Decisions Made
- The three ground-truth findings and their fixes (see key-decisions): the .bss stash, the boot-PML4 execution model, the alignment-rule and ACPI-tag re-keying — all verified against the live channel before the parser was written, in the project's confirm-empirically style
- trap.rs kept as-is: it is already "the stub 13-03 rebuilds" (the IDT stubs + trap() print the new main loads for early-exception diagnostics); the spl/sti/cli warning family it carries is the pre-existing baseline set (verified by building the 13-01 commit in a throwaway worktree), not retirement fallout
- X64-01 left Pending in REQUIREMENTS.md deliberately: its text requires "banner, timer ticks, serial input echo", and ticks/echo are 13-03's timer deliverable — this plan completed the requirement's kernel/protocol halves (with 13-01), and the traceability row should flip when 13-03's stages land

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The mb2 info structure is destroyed before main can read it**
- **Found during:** Task 1 (the ground-truth dump)
- **Issue:** The plan's design ("parse the mb2 info at the mbdata physical address, identity-mapped by l.S's boot PML4s") has two fatal gaps on the real channel: GRUB 2.14 lands the structure at 0x16dc08 — inside the second mebibyte l.S zeroes for the Mach — and warp64 switched cr3 to node0's tables (dropping the identity map) before main
- **Fix:** l.S stashes the structure into a 4 KiB .bss buffer (word copy, clamped) BEFORE the zeroing; warp64 keeps the boot PML4 and passes the stash's linked address; main bounds-checks total_size against the stash size before forming the slice
- **Files modified:** ../r9/x86_64/src/l.S, ../r9/x86_64/src/main.rs
- **Verification:** live boot prints the full fact set parsed from the stash; the gate is green
- **Committed in:** d8d14db

**2. [Rule 1 - Bug] Test 3's "tag size not 8-aligned is an error" would reject GRUB's own structure**
- **Found during:** Task 1 (the ground-truth dump)
- **Issue:** The live structure carries tag sizes 9 (empty cmdline), 18 ("GRUB 2.14"), 28 (APM) — the spec pads tag POSITIONS, not size fields; the planned strict check breaks Task 1's own banner gate
- **Fix:** The parser validates positions/total_size/fixed-body bounds and walks 8-aligned (spec-correct); Test 3 pins tag-header-too-small and misaligned-total_size as loud errors AND pins the live unaligned-size shape as a passing walk
- **Files modified:** ../r9/x86_64/src/platform/q35/mb2.rs
- **Verification:** the live-shape test + the green gate on the real structure
- **Committed in:** e9d3f24 (test) + d8d14db (parser)

**3. [Rule 1 - Bug] Test 4's "RSDP physical pointer" misread the ACPI tag format**
- **Found during:** Task 1 (first live boot printed rsdp 0x20445352 = ASCII "RSD ")
- **Issue:** mb2 tags 14/15 embed the RSDP structure itself (spec 3.9.6/3.9.7), not a pointer to it
- **Fix:** parse_acpi_root signature-validates "RSD PTR " and extracts the RSDT (v1/tag 14) or XSDT (v2/tag 15) address into AcpiRoot; Facts.acpi_root replaces the pointer field; Test 4 re-keyed to the structure-in-tag reality with a bad-signature loud-error case
- **Files modified:** ../r9/x86_64/src/platform/q35/mb2.rs, ../r9/x86_64/src/platform/q35/mod.rs
- **Verification:** live print `q35: acpi rsdt 0x3ffe2345`; the re-keyed Test 4 passes
- **Committed in:** d8d14db

**4. [Rule 3 - Blocking] The plan's test command cannot run as written**
- **Found during:** Task 2 verify
- **Issue:** `cargo test -p x86_64 mb2` targets the crate's default-target (x86_64-unknown-none — no std/test); host tests need the explicit target override
- **Fix:** the working form is `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu mb2 -- --nocapture` (exactly the cargo-xtask test leg's shape; in the rustup-less devshell that leg self-skips, so the explicit form is also the only way the suite covers this crate here)
- **Files modified:** none (command-form note; later plans' verify commands should use it)
- **Verification:** 5 passed
- **Committed in:** n/a

**5. [Rule 3 - Blocking] devcons's tolerant arm needs a port read**
- **Found during:** Task 1
- **Issue:** pio.rs ships out* only; the plan's "aarch64 tolerant shape (error-printing arm)" for devcons has nothing real to fail on without an input primitive
- **Fix:** pio::inb added in the file's own documented extension shape; devcons probes the 16550 scratch register and prints loudly + leaves the console unset on readback mismatch
- **Files modified:** ../r9/x86_64/src/pio.rs, ../r9/x86_64/src/platform/q35/devcons.rs
- **Verification:** live boot's console comes up (all prints reach serial); the probe passes on qemu's 16550
- **Committed in:** d8d14db

---

**Total deviations:** 5 auto-fixed (3 Rule 1 bugs grounded in live-channel evidence, 2 Rule 3 blockers)
**Impact on plan:** All fixes protect the plan's own gates (the banner gate and the loudness doctrine) against the loader's real behavior; no scope creep — pio.rs/Cargo.toml/seg.rs/pagealloc.rs were the minimal seams the plan's actions implied but its file list did not enumerate.

## Issues Encountered
- The plan's file list omitted four files its own actions require (l.S for the entry-path change, seg.rs/pagealloc.rs as the relocation/seeding homes, Cargo.toml for the check-cfg predeclare) — handled as the natural seams and documented above
- The x86_64 crate carries a pre-existing dead-code warning family (spl/sti/cli/flags, present at the 13-01 baseline — verified by building that commit in a throwaway worktree); retirement-orphaned substrate (wrmsr/wrgsbase/fmask/MSR_KERNEL_GS_BASE/Flags::empty/bits) got doc'd allows instead, so the tree adds no new warnings

## Verification Log

On the final committed tree (r9 `b964a62`):
- `cargo xtask boot-test --arch x86-64 --config q35` — **PASS**: "banner marker then >= 1 q35 memory-map fact line observed in order" (run twice; the gate log now carries `q35:` evidence — grep count 1 at kill time, the full serial in `13-02-evidence-q35-serial.log`)
- Full live serial: banner, 9 `q35: memory` ranges (0x0..0x9fc00 available through 0xfffc0000..0x100000000 reserved, matching the ground-truth dump exactly), `q35: acpi rsdt 0x3ffe2345`, Memory usage Used 0x800000 / Total 0x3ffdf000
- `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu mb2` — 5 passed, 0 failed
- `cargo xtask ureg-check` — 35 fields match; `cargo xtask boot-test --arch aarch64 --config virt` — PASS (banner + >= 3 ticks + echo); `cargo xtask test` — 101 (port) + 83 (xtask, +1 new) passed; `cargo xtask check` and `cargo xtask clippy` — green, no new warnings in any touched file
- Acceptance greps: platform/q35/mb2.rs exists with 39 ParseError mentions; print_machine_info/cfg(platform) counts 2/2; proc/syscall/node0/vsvm gone; dat.rs vsvm count 0; main.rs swtch/jumpback count 0

## Known Stubs
None — the parked payload is replaced by the real arc; the masked-IRQ halt at the end of main is this plan's designed end state (sti and the timer are 13-03's deliverable, per the plan's own ordering rule).

## Authentication Gates

None - no authentication was required.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- 13-03 (LAPIC/IOAPIC + TSC-deadline timer) inherits: the platform/q35 slot, the masked-IRQ park to replace with the arming window, the filled+loaded IDT/GDT/TSS, the identity-mapped execution model, and the gate's ordered-marker machinery (add tick/echo stages to the X64 profile)
- 13-04 (ACPI/ECAM) starts from `Facts.acpi_root` — the RSDT address arrives loader-mediated on every BIOS boot (no EBDA scan needed on the gate channel); type-3 ranges are already kept out of the free pool
- The kernel/protocol halves of X64-01 are complete; its REQUIREMENTS row flips when 13-03 lands ticks/echo
- No blockers.

## Self-Check: PASSED

- All 14 r9 files exist on disk (created+modified, `[ -f ]` each: FOUND); the 5 deleted files confirmed absent
- r9 commits verified: e9d3f24, d8d14db, b964a62 are ancestors of r9 `main` HEAD (`git merge-base --is-ancestor`)
- TDD gates: test(13-02) precedes and is an ancestor of both feat(13-02) commits; RED evidence at `13-02-red-evidence.json`
- Production code lives in the sanctioned sibling repo `/home/req/code/r9` per the dual-repo execution context; the harvey-side commit for this plan is the docs commit carrying this SUMMARY/STATE/ROADMAP

---
*Phase: 13-r9-x86-64-platform-bring-up*
*Completed: 2026-11-07*
