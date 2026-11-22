---
phase: 04-r9-userspace-exec-harvey-r9-abi
plan: 04
subsystem: kernel/vm
tags: [r9, aarch64, proc, exec, demand-paging, fault-handler, a.out, icache, trap-ec-split]

requires:
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 01)
    provides: initrd delivery (loader + patched DTB), D-14 measured syscall set
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 02)
    provides: port::paqfs no_std reader (the fault handler's byte source), KZERO+PA runtime-map convention
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 03)
    provides: TrapFrame ≡ Harvey Ureg (35×u64, stride 288, offsets/arg-read formula frozen in aarch64/ABI.md + pinned by xtask ureg-check)
provides:
  - aarch64/src/proc.rs — Proc/ProcState/PROC_TABLE (Lock<ProcTable> over [Option<Proc>; 8])/new_user_proc/current_proc; per-process 16 KiB kstacks + root tables with recursive slot-511 self-entry
  - aarch64/src/exec.rs — a.out parse + donor-grade validation + exec_into + seed_exec_frame (forkret shape); r9 user-VA policy constants (D-11): USTACKTOP/USTKSIZE(16 MiB)/USTOP
  - aarch64/src/fault.rs — user_fault + flush_icache_user_range (CTR_EL0-derived D-09 sequence)
  - aarch64/src/registers.rs — EsrEl1IssDataAbort (wnr bit 6) + DataFaultStatusCode + LowerEl constructors + tests
  - aarch64/src/trap.rs — EC 0x20/0x24 → fault::user_fault and RETURN; EC 0x15 prints the wire number frame.x0; other EL0-origin sync → faultnote-shaped kill
affects:
  - 04-05 (runtime wiring: boot_init → exec_into, devroot byte source, D-10 TTBR0 switch, first eret — consumes this machinery unchanged)
  - 04-06 (syscall dispatch replaces the EC-0x15 placeholder; EXITS owns the all-exited clean halt)
  - 04-07 (scheduler iterates PROC_TABLE; swtch Context + per-proc kstacks are in place)
  - 04-08 (note_queue/notify_handler slots filled)

actuals:
  tokens: 14208
  tasks: 2
  commits: 4
  plan_head_before_r9: 2303cf93bd002de3f246ef94ada5ba8d6d364408
  plan_head_after_r9: b14c3e81a0b2478416c8131ae0419909e200e88e

tech-stack:
  added: []
  patterns:
    - "per-process root tables: pagealloc'd page mapped at KZERO+PA in the kernel table, zeroed, recursive slot-511 self-entry written through the KZERO-offset physical alias (init_empty_root_page_table's pointer shape) — never the USER_PAGETABLE singleton"
    - "exec-time content writes to inactive user tables go through the pages' KZERO+PA kernel aliases; ACTIVE-table demand mapping rides map_to's temp-recursive dance and content writes go through the user VA (Pitfall 4 discipline)"
    - "fault-handler kill path: print marker + ProcState::Moribund + wfi park (single-process era); kernel-context FAR or null current-proc panics loudly (T-04-09)"
    - "already-mapped-page check via the proc's mapping_log before any demand page-in (donor fault() existence-check port)"

key-files:
  created:
    - ../r9/aarch64/src/proc.rs
    - ../r9/aarch64/src/exec.rs
    - ../r9/aarch64/src/fault.rs
  modified:
    - ../r9/aarch64/src/registers.rs
    - ../r9/aarch64/src/trap.rs
    - ../r9/aarch64/src/vm.rs
    - ../r9/aarch64/src/main.rs
    - ../r9/aarch64/Cargo.toml
    - ../r9/port/src/dat.rs
    - ../r9/HowItWorks.md

key-decisions:
  - "Kernel-stack VA allocation rides vm::next_free_device_page4k (the kernel-VA bump allocator): 4 contiguous VA pages per proc, each a fresh pagealloc page, contiguity asserted loudly — r9 has no general kernel-VA allocator yet"
  - "exec maps the two seed pages into the proc's INACTIVE root table via map_to's temp-recursive dance and writes contents via KZERO+PA kernel aliases; the fault handler maps demand pages into the ACTIVE user table and writes through the user VA — the two edit paths are distinct by table liveness"
  - "fault handler kills on already-mapped-page faults (mapping_log lookup) — remapping a live page would silently zero its contents; this is the donor fault() existence check"
  - "TrapFrame seeding API lives in trap.rs (zeroed/seed_el0_entry/pc getter) — 04-03 left the fields private; exec.rs composes it (04-03's EsrEl1::read() precedent for plan-named missing methods)"
  - "r9 user address-space policy constants landed in exec.rs (D-11): USTACKTOP=0x0000_8000_0000_0000, USTKSIZE=16 MiB, USTOP=USTACKTOP-16MiB"

patterns-established:
  - "demand-page evidence markers: `fault: text page-in va=…` / `fault: data page-in va=…` / `fault: stack growth va=…` / `fault: bss page-in va=…` — the 04-05 exec profile asserts these as proof of demand paging"
  - "not-yet-wired machinery compiles into all configs behind module-level #[allow(dead_code)] on `mod exec;`/`mod proc;` with a comment naming the plan that removes it (04-05)"

requirements-completed: [R9-02, R9-03]

coverage:
  - id: T1
    description: "Proc scaffolding + per-process root tables + vm.rs visibility + cfg(dev_exec) registration: Proc carries pid/name/state/kstack_top/root_table/fd_table/mapping_log/segments/frame/context/note_queue/errstr; new_user_proc() builds a root table with the recursive slot-511 self-entry; dat.rs Chan carries qid/mode/offset with the Dev trait unchanged; no USER_PAGETABLE use for process tables"
    requirement: "R9-02"
    verification:
      - kind: automated
        ref: "cargo xtask build --arch aarch64 --config {virt,virtexec} + default(raspi3b) + raspi4b all green; cargo xtask clippy --arch aarch64 --config virtexec exit 0; grep chain (cfg(dev_exec) in aarch64/Cargo.toml, pub struct Proc/kstack_top/root_table/mapping_log/Lock<ProcTable> in proc.rs) PASS; USER_PAGETABLE grep of proc.rs = 0 hits"
        status: pass
    human_judgment: false
  - id: T2
    description: "a.out loader (magic 0x8C47/HDR_MAGIC 40-byte/entry-in-text/sizes-vs-USTOP validation), exactly two eager pages (seed stack + zeroed Tos) + forkret frame at kstack_top-288; demand-fault handler (translation/access-flag/permission/TLB-conflict → demand; 16 MiB stack bound; text written via user VA; full CTR_EL0-derived icache sequence per text page-in); registers.rs data-abort decode with cfg(test) tests; trap.rs EC split (0x20/0x24 → user_fault + return, 0x15 prints frame.x0, other EL0 sync → faultnote-shaped kill, EL1 dump unchanged)"
    requirement: "R9-03"
    verification:
      - kind: automated
        ref: "four-config builds + clippy green; grep chain (EsrEl1IssDataAbort/wnr/parse_aout/0x8C47/flush_icache_user_range/ctr_el0/dc cvau/ic ivau/InstructionAbortLowerEl/DataAbortLowerEl) PASS; cargo test -p aarch64 --bins --target aarch64-apple-darwin 8/8 PASS incl. both new decoder tests; cargo xtask test (port) 12/12 PASS; ureg-check green (35 fields match — frame seeding conforms to the published ABI)"
        status: pass
    human_judgment: false
  - id: T3
    description: "No-regression: Phase-3 virt boot-test profile PASS after the trap.rs EC-split edits (the new branches are unreachable with no EL0 — a failure would mean the dispatch edit broke IRQ handling); raspi3b/raspi4b builds green"
    requirement: "R9-03"
    verification:
      - kind: automated
        ref: "cargo xtask boot-test --arch aarch64 --config virt → PASS (banner + ≥3 ticks + echo; grep 'r9 from the Internet' target/boot-test-virt.log PASS); cargo xtask boot-test --arch aarch64 --config virtpanic → PASS (banner → start marker → ≥30 rows → kernel halted) — extra evidence beyond the plan's leg since trap.rs's dump path was restructured"
        status: pass
    human_judgment: false

duration: 17min
completed: 2026-11-02
status: complete
---

# Phase 04 Plan 04: proc/exec/fault Spine Summary

**R9-03's machinery half landed compile-verified: per-process kernel stacks and root page tables (recursive slot-511 self-entry, never the USER_PAGETABLE singleton), the Plan 9 a.out loader with the donor sysexec validation set, the demand-fault handler with the CTR_EL0-derived icache sequence on the user VA, and the trap EC split routing lower-EL aborts into it — green on all four build configs + clippy, with the Phase-3 virt and virtpanic boot profiles both still passing.**

## Performance

- **Duration:** 17min
- **Started:** 2026-11-02T13:09:07Z
- **Completed:** 2026-11-02
- **Tasks:** 2/2
- **Commits:** 3 r9 + 1 Harvey metadata (below)

## Accomplishments

- **Task 1 (r9 `cd4bd88`): Proc scaffolding.** `aarch64/src/proc.rs` — the full `Proc` field set (pid/name/state/kstack_top/root_table/fd_table[16]/mapping_log/segments/text_source/frame/context/note_queue/notify_handler/wait_record/errstr), `PROC_TABLE: Lock<ProcTable>` over a fixed `[Option<Proc>; 8]` with the mcslock idiom (never held across the fault handler or pagealloc), `current_proc()` raw-pointer accessor with the single-CPU/IRQ-masked invariant documented, and `new_user_proc()` allocating pid + 16 KiB kernel stack (4 contiguous kernel-VA pages off the bump allocator, contiguity asserted) + a per-process root table (pagealloc'd page mapped at KZERO+PA, zeroed, recursive slot-511 self-entry via the now-`pub` `init_empty_root_page_table`). `port/src/dat.rs` grew real `Chan` fields (qid/mode/offset/dev/path + crate-private paqfs `PaqDir` slot) with the `Dev` trait signature untouched; `aarch64/Cargo.toml` registers `cfg(dev_exec)` for 04-05's config change.
- **Task 2 (r9 `2136c93`): loader + fault handler + EC split.** `exec.rs` parses/validates the BE a.out header (magic `0x8C47`, 40-byte HDR_MAGIC form, BE u64 entry @32, entry-in-text, text+data+bss < USTOP) and seeds EXACTLY two eager pages (seed stack + zeroed Tos — D-09/D-11) written via their KZERO+PA aliases with the 9front execregs `[argc][argv0]…[nil]` layout, then seeds the forkret-shaped `TrapFrame` at `kstack_top-288` (R0=tos, SP→user stack, PC=entry, PSR=0 EL0t, LR=0). `fault.rs` routes translation/access-flag/permission/TLB-conflict classes to demand paging (faultarm64's table), pages text/data in from the paqfs byte source at byte granularity (Pitfall-5 offsets), writes through the user VA (Pitfall 4), runs the verbatim D-09 `DC CVAU → DSB ISH → IC IVAU → DSB ISH → ISB` sequence with CTR_EL0-derived geometry after every text page-in, zero-fills bss/stack/tos within the 16 MiB stack bound, kills loudly on unresolvable or already-mapped faults, and panics loudly on kernel-context FARs (T-04-09). `registers.rs` gained `EsrEl1IssDataAbort` (wnr bit 6) + `DataFaultStatusCode` + LowerEl constructors with tests (8/8 host tests pass). `trap.rs` routes EC 0x20/0x24 into `fault::user_fault` and RETURNS, prints the wire syscall number from `frame.x0` (was the ISS), and gives other EL0-origin sync the faultnote-shaped kill while the EL1 panic dump stays byte-identical (virtpanic gate PASS).
- **Docs (r9 `b14c3e8`):** HowItWorks.md gained the userspace-exec machinery section (its header mandates material-change updates).

## Task Commits

| Task | Repo | Commit | Files |
|------|------|--------|-------|
| 1 | r9 | `cd4bd88` | aarch64/src/proc.rs (new), aarch64/Cargo.toml, port/src/dat.rs, aarch64/src/vm.rs, aarch64/src/main.rs |
| 2 | r9 | `2136c93` | aarch64/src/exec.rs (new), aarch64/src/fault.rs (new), aarch64/src/registers.rs, aarch64/src/trap.rs, aarch64/src/main.rs |
| docs | r9 | `b14c3e8` | HowItWorks.md |
| metadata | harvey | (this commit) | .planning SUMMARY/STATE/ROADMAP/REQUIREMENTS |

**r9 HEAD at evidence time (D-13 ordering — r9 commits landed first):** `b14c3e81a0b2478416c8131ae0419909e200e88e`

## Files Created/Modified

All in the r9 repo (the plan's only Harvey-side artifact is this planning evidence):

- `aarch64/src/proc.rs` (new, ~330 lines) — Proc/ProcState/SegKind/Segments/ByteSource/MappingRecord/Note/WaitRecord/ProcTable/PROC_TABLE/current_proc/set_current_proc/new_user_proc
- `aarch64/src/exec.rs` (new, ~290 lines) — AoutHeader/parse_aout/exec_into/seed_exec_frame + the D-11 policy constants (USTACKTOP/USTKSIZE/USTOP) + INITTEXT/TEXT_MAP_BASE
- `aarch64/src/fault.rs` (new, ~210 lines) — user_fault/page_in_from_image/page_in_zeroed/map_active_user/flush_icache_user_range/unresolvable/kill_current_process
- `aarch64/src/registers.rs` — EsrEl1IssDataAbort + DataFaultStatusCode + LowerEl filters + 2 new cfg(test) tests
- `aarch64/src/trap.rs` — TrapFrame::zeroed/seed_el0_entry/pc(); trap() EC split
- `aarch64/src/vm.rs` — map_to + init_empty_root_page_table go pub (visibility only, no structural change)
- `aarch64/src/main.rs` — mod exec/fault/proc registrations (module-level dead-code allowance on exec/proc, naming 04-05 as the remover)
- `aarch64/Cargo.toml` — cfg(dev_exec) check-cfg registration
- `port/src/dat.rs` — Chan real fields + Qid real fields + Error static-str carrier
- `HowItWorks.md` — machinery section

## Decisions Made

See key-decisions in the frontmatter: the kernel-VA bump allocator as the kstack VA source, the two distinct table-edit paths by table liveness (KZERO+PA alias for inactive, user-VA writes for active), the already-mapped-page kill guard, the TrapFrame seeding API placement, and the landed D-11 policy constants.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] proc.rs double mutable borrow of the proc table**
- **Found during:** Task 1 first build
- **Issue:** `table.procs.iter_mut().find()` held the table mutably while `table.next_pid` was read and bumped
- **Fix:** bumped `next_pid` before taking the slot iterator
- **Files modified:** ../r9/aarch64/src/proc.rs
- **Commit:** cd4bd88 (r9)

**2. [Rule 1 — Bug] exec.rs usize→u64 mismatch at the seed_exec_frame call**
- **Found during:** Task 2 first build
- **Issue:** VA math is usize; the frame API takes u64 (the header's entry type)
- **Fix:** cast at the call boundary (`user_sp as u64, tos_va as u64`)
- **Files modified:** ../r9/aarch64/src/exec.rs
- **Commit:** 2136c93 (r9)

**3. [Rule 1 — Bug] fault.rs read the private TrapFrame.pc field**
- **Found during:** Task 2 first build
- **Issue:** 04-03 left all frame fields private; the plan's unresolvable-fault print (`pc=0x<pc:x>`) needs the frame's pc
- **Fix:** added a `pub fn pc()` getter to trap.rs alongside the seeding API (the conformance-preserving minimal surface; fields stay private)
- **Files modified:** ../r9/aarch64/src/trap.rs, ../r9/aarch64/src/fault.rs
- **Commit:** 2136c93 (r9)

**4. [Rule 1 — Acceptance-letter] USER_PAGETABLE literal in a proc.rs doc comment**
- **Found during:** Task 1 acceptance greps
- **Issue:** acceptance requires zero USER_PAGETABLE hits in proc.rs; a doc comment named the anti-pattern by its identifier
- **Fix:** reworded to "the vm.rs user-table static singleton" — guidance preserved, letter satisfied
- **Files modified:** ../r9/aarch64/src/proc.rs
- **Commit:** cd4bd88 (r9)

**5. [Rule 2 — Missing critical] already-mapped-page guard in the demand path**
- **Found during:** Task 2 implementation
- **Issue:** a permission/access-flag fault on an already-mapped page (e.g. a fetch from a data page — UXN-set) would have remapped a fresh zero page over live contents, silently corrupting the process
- **Fix:** the handler consults the proc's mapping_log first; an already-mapped page_va takes the unresolvable kill path (the donor fault() existence check)
- **Files modified:** ../r9/aarch64/src/fault.rs
- **Commit:** 2136c93 (r9)

**6. [Rule 2 — Missing critical] zero-fill before text/data page-in**
- **Found during:** Task 2 implementation
- **Issue:** pagealloc'd phys pages carry garbage; the file window reads short at segment tails — without a pre-zero, text/data pages would execute garbage tails
- **Fix:** every demand-mapped page (text/data/bss/stack/tos) is zero-filled through the user VA before any content read
- **Files modified:** ../r9/aarch64/src/fault.rs
- **Commit:** 2136c93 (r9)

**Plan-supporting additions (in-plan intent):** `TrapFrame::zeroed()` + `seed_el0_entry()` (the plan's exec frame seeding requires writing frame fields from exec.rs; 04-03's reshape made them private — same precedent as 04-03's `EsrEl1::read()` plan-named-method addition); `Chan::new()` constructor (the real fields need one for 04-05/04-06); `EsrEl1IssInstructionAbort::from_esr_el1` now also accepts `InstructionAbortLowerEl` (the plan's "extend the constructor family" step).

**Total deviations:** 6 auto-fixed + 3 plan-supporting additions. No Rule 4 (architectural) items.
**Impact on plan:** none to intent — all must-have truths, acceptance criteria, and plan-level verification items pass at the final state.

## Known Stubs

Plan-mandated seams with named owning plans (none prevent this plan's goal — compile-verified machinery):

- **fd 0/1/2 = None at exec** (../r9/aarch64/src/proc.rs `new_user_proc`) — the D-15 devcons preopen seam; 04-06's devcons device supplies the Chan values. (The plan explicitly defers this: "nothing reads fds until dispatch exists.")
- **EC 0x15 loud-unimplemented placeholder** (../r9/aarch64/src/trap.rs) — now prints the wire number from frame.x0; real dispatch is 04-06 per the plan.
- **note_queue/notify_handler/wait_record/context fields** are carried but unfilled — 04-08 (notes) and 04-07 (scheduler/await) own them, per plan.
- (Ledger note: `.planning/WINDOWS.md` append was attempted for the fd seam but the ledger rejects cross-repo (`../r9/…`) paths with `windows_invalid_file`; the seams are recorded here instead.)

## Issues Encountered

None beyond the deviations above — two compile-error rounds (Task 1 borrow, Task 2 type/privacy), each fixed inline on the first retry; all verification legs green on the first full run after that.

## Threat Flags

None. Both threat-register mitigations landed as designed: T-04-08 (the donor sysexec check set is ported verbatim into `parse_aout` — magic/HDR_MAGIC-40/entry-in-text/sizes-vs-USTOP, all loud Errs before any mapping exists) and T-04-09 (handler IRQs-off, touches only pre-mapped kernel memory + the user VA being installed, kernel-context FAR panics loudly, pagealloc's MCS lock taken briefly and never under PROC_TABLE). T-04-SC: no package installs this plan (no new crates).

## User Setup Required

None.

## Next Phase Readiness

- **04-05** starts from known-good machinery and only debugs the runtime wiring: the devroot byte source, `boot_init` → `exec_into` call path, the D-10 TTBR0 switch (ungate at the main.rs call site), and the first eret into EL0 from the seeded frame. The demand-page evidence markers (`fault: text page-in va=…` etc.) are the serial proof points.
- **04-06** replaces the EC-0x15 placeholder with the measured dispatch table {pwrite(51), open(14), pread(50), close(4), exits(8)} ∪ the D-14 named core; the fd 0/1/2 preopen seam is documented in `new_user_proc`.
- **04-07** gets PROC_TABLE iteration + per-proc kstacks + swtch Context slots for free; **04-08** fills note_queue/notify_handler.
- Frame seeding conforms to the published ABI by construction — `ureg-check` stayed green at every boot-test (35 fields match).
- r9 HEAD for downstream pinning (D-13): `b14c3e81a0b2478416c8131ae0419909e200e88e`.
- Flagged assumption carried from the plan: R9-03 remains probe-unclassified — acceptance here derived from ROADMAP SC#3 + CONTEXT D-04/D-09/D-10/D-11/D-12/D-15; the verifier should treat the probe row as a flagged assumption, not a resolved edge.

## Self-Check: PASSED

- Key files exist: r9 `aarch64/src/proc.rs` ✓, `aarch64/src/exec.rs` ✓, `aarch64/src/fault.rs` ✓, plus modified registers.rs/trap.rs/vm.rs/main.rs/Cargo.toml/dat.rs/HowItWorks.md — all verified on disk.
- Commits: `git -C ../r9 log --grep="04-04"` → cd4bd88 (Task 1), 2136c93 (Task 2), b14c3e8 (docs) — ≥1 `04-04` commit in r9 ✓; Harvey metadata commit below (the plan's only Harvey-side artifacts are planning files) ✓.
- Acceptance criteria re-run at close-out: Task-1 grep chain (cfg(dev_exec), pub struct Proc, kstack_top, root_table, mapping_log, Lock<ProcTable) PASS; USER_PAGETABLE grep of proc.rs = 0 hits ✓; Task-2 grep chain (EsrEl1IssDataAbort, wnr, parse_aout, 0x8C47, flush_icache_user_range, ctr_el0, dc cvau, ic ivau, InstructionAbortLowerEl, DataAbortLowerEl) PASS ✓.
- Plan-level verification at final r9 HEAD (`b14c3e8`): four-config builds + clippy green ✓; `cargo xtask boot-test --config virt` PASS + banner grep ✓ (plus virtpanic PASS — extra evidence); 8/8 aarch64 host tests + 12/12 port tests ✓; ureg-check green at every boot-test ✓.

---
*Phase: 04-r9-userspace-exec-harvey-r9-abi*
*Completed: 2026-11-02*
