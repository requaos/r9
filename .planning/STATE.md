---
gsd_state_version: "1.0"
milestone: v3.1
milestone_name: Early SMP and Cluster Fabric
current_phase: 05
current_phase_name: Guide Phase 3 - Early SMP, Course Correction and Memory Pools
status: executing
stopped_at: Phase 5 context gathered
last_updated: "2026-11-26T04:17:55.589Z"
last_activity: 2026-11-25
last_activity_desc: Phase 05 execution started
state_head: 0d08e6cb86af0bee94f6ab67b4e126692773ecd7
progress:
  total_phases: 3
  completed_phases: 0
  total_plans: 27
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-11-25)

**Core value:** The OS boots to rio on QEMU q35 at every commit.
**Current focus:** Phase 05 — Guide Phase 3 - Early SMP, Course Correction and Memory Pools

## Current Position

Phase: 05 (Guide Phase 3 - Early SMP, Course Correction and Memory Pools) — EXECUTING
Plan: 1 of 27
Status: Executing Phase 05
Last activity: 2026-11-25 — Phase 05 execution started


## Performance Metrics

**Velocity:**
- Total plans completed: 78

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 34 | - | - |
| 02 | 6 | - | - |
| 03 | 13 | - | - |
| 04 | 25 | - | - |

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P03 | 3 min | 2 tasks | 8 files |
| Phase 01 P04 | 6 min | 3 tasks | 17 files |
| Phase 01 P05 | 5 min | 2 tasks | 8 files |
| Phase 01 P01 | 13 min | 3 tasks | 8 files |
| Phase 01 P02 | 9min | 3 tasks | 15 files |
| Phase 01 P06 | 80min | 3 tasks | 19 files |
| Phase 01 P07 | 17min | 3 tasks | 5 files |
| Phase 01 P08 | 4min | 2 tasks | 8 files |
| Phase 01 P09 | 4min | 3 tasks | 7 files |
| Phase 02 P01 | 7 min | 4 tasks | 10 files |
| Phase 02 P03 | 5 min | 3 tasks | 5 files |
| Phase 02 P04 | 2 min | 3 tasks | 6 files |
| Phase 02 P05 | 10 min | 3 tasks | 4 files |
| Phase 02 P06 | 138 min | 3 tasks | 3 files |
| Phase 03 P01 | 2 min | 2 tasks | 11 files |
| Phase 03 P12 | 2min | 2 tasks | 12 files |
| Phase 03 P02 | 3min | 2 tasks | 7 files |
| Phase 03 P03 | 2 min | 2 tasks | 7 files |
| Phase 03 P04 | 2min | 3 tasks | 7 files |
| Phase 03 P05 | 4 min | 3 tasks | 4 files |
| Phase 03 P06 | 25 | 2 tasks | 5 files |
| Phase 03 P13 | 13 min | 1 tasks | 2 files |
| Phase 03 P07 | 32 min | 2 tasks | 2 files |
| Phase 03 P08 | 51 min | 2 tasks | 5 files |
| Phase 03 P09 | 1 min | 2 tasks | 5 files |
| Phase 03 P10 | 15 min | 3 tasks | 6 files |
| Phase 03 P11 | 92 min | 3 tasks | 14 files |
| Phase 04 P14 | 23 min | 3 tasks | 8 files |
| Phase 04 P15 | 1h 6m | 3 tasks | 2 files |
| Phase 04 P23 | 48 min | 2 tasks | 3 files |
| Phase 04 P24 | 11 min | 2 tasks | 6 files |
| Phase 04 P25 | 1h 4m | 3 tasks | 2 files |

## Accumulated Context

### Decisions

Full log is the PROJECT.md Key Decisions table and the archived phase summaries under
`.planning/milestones/v3.0-phases/`. Carried forward:

- [v3.0 shipped 2026-11-25] 38/38 requirements complete across 4 phases; all four phase
  verifications `passed`; UAT 112/112 on Phase 4 and 3/3 on Phases 2 and 3; audit `tech_debt`
  with no blockers.
- [v3.0] The wire contract is the 28-byte header with the MAC over every preceding byte,
  per-connection per-direction keys from a transcript-bound HKDF, the parameter-block-plus-page
  ciphertext body, the six `ExtMask` bits with `EXT_STATELESS_CAP`, and `DECISION P9`’s
  self-identifying in-band re-key exchange (`NOTAG` + reserved afid).
- [v3.0] The booted kernel still speaks plain 9P2000; the Aead engine is selected only at mount
  negotiation by a peer that offers it (D-16/D-23). The golden invariant is the r9-owned
  `cargo xtask gate`, proven at `0dd1037` under KVM, native TCG and the full gate set.
- [v3.0] Harness follow-ups are the v3.1 backlog: wire `test-9p` and `cargo test -p xtask` into
  CI, reconcile `04-VALIDATION.md`, correct the boot-iso evidence claim, settle the TPHT
  bank-sizing precondition.

### Rescope of 2026-11-25

The owner's `docs/plan_r9_architecture_code_compendium_v3_0.md` and `docs/r9_implementation_guide_for_agentic_developers_v3_0.md` supersede the earlier guide and compendium where they differ: software cryptographic memory is dropped, `FidTable` becomes a sharded generational slab, two-CPU SMP is Phase 5, and the C compilers are not ported. The planning written earlier that day for the old Phase 5 (context D-01 to D-38, research, pattern map) is archived under `.planning/milestones/superseded/`; no plan was written or executed from it. The new Phase 5 opens with the guide's five course-correction tasks and the audit follow-ups. Whether CI is red at `900f58b` has not been checked.

### Open Questions (four warnings, approved to be carried, not resolved)

Settle each when its phase is planned. Detail: `.planning/INGEST-CONFLICTS.md`. Eleven notes (N1-N11) are in PROJECT.md; N7 to N11 are void or reduced since the 2026-11-25 rescope (see PROJECT.md); N9 to N11 were source contradictions the Phase 2 threat model found untracked (stale-frame mechanism, plaintext in legacy bounce frames, where the wire MAC is verified).

| ID | Settle in | Question |
|----|-----------|----------|
| W7 | Phase 6 | `DiscoveryBeaconV2` 192 versus 202 bytes |
| - | Phase 5 | Settled 2026-11-25 at the phase discussion (05-CONTEXT.md D-10, D-14, D-18, D-19): eviction drops re-readable pages; the TPHT page cache is built now; the Roaring set kernels get SIMD; the slab handle's generation is never zero |
| W8 | Phase 13 | `/adm/users` UID 0 `adm` versus persona `glenda` |
| W9 | Phase 13 | Test 3.2 needs `alice` absent from `/adm/users` |
| W10 | Phase 13 | Suite 3 cross-VLAN mounts with no route |

### Pending Todos

None yet.

### Blockers/Concerns

- **v3.0 is shipped and archived.** Phase directories now live under
  `.planning/milestones/v3.0-phases/`; the roadmap, requirements and audit are archived beside
  them. Phase 1-3 verification reports read `stale` only because later phases changed shared
  files they covered; the owner accepted that override at the close.
- **Push state:** `origin/main` = `bb68d988752ccb616a980b943ea86aecca8f25be` (the owner’s second
  approved push). Local `main` is ahead only by v3.0 close-out commits — the second-stop verdict
  commit `4d420c9`, the 04-25 SUMMARY, `04-UAT.md`, the audit, the archive, and this close — all
  docs/evidence, held local until a later approved push. `origin/phase-04/aead-wire-gaps` =
  `194c71d` and `origin/phase-04/aead-wire` = `405984d`; both transport branches can be deleted.
- **v3.1 backlog from the v3.0 audit (no blockers):** `cargo xtask test-9p` and
  `cargo test -p xtask` are not in CI or `cargo xtask test` (INT-03/INT-04); the boot-iso
  evidence does not support the “fixes moved the linked kernel bytes” claim (INT-02); the TPHT
  >90% home-hit property is measured with fixture-sized banks and the Phase 4 FidShard uses 0
  banks (INT-05); `04-VALIDATION.md` is `draft` (run `/gsd-validate-phase 4`);
  `docs/threat-model.md:181` still frames rfork bits 18-20 as unreserved (WR-04).
- **Phase 5 open defects (2026-11-27, owner chose fix-now):** (1) aarch64 live evictor reclaims nothing under pressure — `fault: phys page alloc failed` kills the caller and the mem-pressure counters stay 0/0/0 (debug session `aarch64-palloc-evictor` active; locally reproducible); (2) the mem-pressure gate's `pinned_skipped > 0` criterion is structurally unreachable from this image and is being replaced; (3) x86_64 `-smp 2` under real KVM resets right after ring-3 entry with no `smp: cpu 1 online` (pre-dates 05-19; SIPI-retry fix did not cure it — needs the QEMU exception-vector log); (4) x86_64 `-smp 1` cpu-chain hangs at `cpu-keyfs-ok` (new since 5a3d5e4). Evidence: `kvm-evidence-20261127T015715Z`. Phase 5 stays at 26/27 until these are green or the owner waives them. **Dedicated debug task prepared (not yet run):** `.planning/debug/x86_64-ap-bringup.md` carries the marker legend, eliminated hypotheses and the experiment plan; `scripts/kvm-ap-debug.sh` runs one AP cycle on an x86_64 Linux KVM host (`--push` sends the capture on an `ap-debug-<ts>` branch). Run it on that host; Owner Stop 4 waits on the result.
- **x86_64 AP bring-up debug task — run 2026-11-27, awaiting the owner's verify.** Defect (3) is
  root-caused and fixed on branch `debug/x86_64-ap-bringup` (seven local commits `b4ec82f..4342436`,
  the last two repairing the `x86_64` crate's host-test leg; `main` untouched at `0e783a0`; nothing pushed): the SIPI trampoline's 32-bit data segment was
  expand-down (`SegWRITE` is the expand-down bit for a DATA descriptor), which TCG does not enforce
  and KVM does; behind it, `91187e8` started the APs twice on the dev_exec path, the bring-up
  delays collapsed on the periodic timer venue (TCG: `tsc_delta` was 0 there), and `pick_next`'s
  kstack tripwire compared kstack VAs that are the same window VA for every proc. On KVM the
  composed smoke gate now passes `smp-online`, `paqfs` and `version` with `smp: cpu 1 online` and
  `smp: tlb shootdown ok`; on TCG the two lines print at +12 ms/+13 ms from ring 3. CI's single-CPU
  TCG leg (`R9_SMP=1`) passes 31 stages on the branch and then stops at `cpu-keyfs-ok` — defect (4)
  below — identical on `main` (0e783a0, its own kernel build, R9_SMP=1 R9_ACCEL=tcg): the same 31 stages pass and the run ends in the same `TIMEOUT at stage: cpu-keyfs-ok` — the branch does not change the single-CPU leg. `cargo xtask test` exits 0 on the branch (it did not compile at `main`). The
  `-smp 2` wedge behind it (MCS lock nodes on the calling CPU's kernel stack, every proc's kstack at
  one VA under its own PML4) is fixed on the next branch, below. Note: "TCG passes" never held for two CPUs — CI's x86_64 smoke job pins `R9_SMP=1`
  (`gates.yml:148-158`). Record: `.planning/debug/x86_64-ap-bringup.md`.
- **x86_64 two-CPU fix — run 2026-11-27, awaiting the owner's verify.** Branch
  `fix/x86_64-smp2-kstack` (stacked on `debug/x86_64-ap-bringup`; local, nothing pushed): `667b545` reserves one arena of
  contiguous 16 KiB kernel-stack slots at boot, reached at the KZERO alias every page table carries, so a lock hand-off between
  two CPUs writes a node both can see; `9a2b5a3` holds the console lock across a whole line; `3ff89ab` makes the scheduler
  context run on the boot page table — a CPU used to idle, and the scheduler used to run, on the page-table root of the process
  it had left, and once that process exited the root went back to the allocator and was zeroed under it (a triple fault the churn
  gate reached 600 cats in, at two CPUs only; 14 stops in 25 workload runs before, 64 of 64 complete after). On the KVM host at
  the default `-smp 2`, on `3ff89ab`: churn and net-smoke pass; smoke passes the chain leg and the CPU ladder to `cpu-ether-ok`
  and stops at `cpu-keyfs-ok` (defect (4)); tier-1 and services end where `main`
  ends at `-smp 1` (controls run), on `fpprobe` (see the next bullet) and services' `srv` and `exportfs`
  (`SVC-SKIP-WHY no-rc-lib-rcmain-the-srv-child-rc-dies`; not investigated); auth fails on `hmntgen` (defect (4)); `main` at `-smp 1` fails
  each of those the same way (controls run). The tier-1, services, churn and auth drivers hard-code `-smp 2`; only smoke reads
  `R9_SMP`. `cargo xtask test` exits 0 (1254), clippy exits 0. Record: `.planning/debug/x86_64-smp2-mcs-kstack.md`.
- **Two findings of that run, for the owner.** (a) Defect (4) is not a hang: the keyfs rung's helper `hmntgen` calls `fauth`
  (syscall 10), which the dispatch table leaves unimplemented by record (`x86_64/src/syscall.rs`: "OSEEK/FAUTH stay
  loud-None"), and then faults at pc=0; `cpurc: FAIL keyfs` cascades down the ladder and the gate's 600 s wait for
  `cpurc: KEYFS-OK` is what looks like a hang. `boot`, `init` and `hfactotum` call it too. (b) tier-1's `fpprobe` fails its
  flags-nt sub-leg on `main` at `-smp 1` as well: `sys_sleep` blocked once and reported the first wake as "interrupted", and a
  child's exit wakes its parent whatever it is blocked on. Fixed on the stacked branch `fix/x86_64-sleep-wake` (its one commit
  sits on the branch above): the sleep ends only at its deadline or on a note; tier-1 then passes at `-smp 1` and at `-smp 2`.
- **Scale targets (owner, 2026-11-27) — audited; the memory model is built, paths next.** Future systems: more than
  100 GiB of RAM, hundreds of terabytes of storage, file paths exceeding 2048 characters. None of
  the three is stated anywhere in the plans, and the verification plan's VMs are 2 GiB. Audit with
  sources in `.planning/SCALE-AUDIT.md`. Headline findings: the x86_64 kernel reaches RAM only
  through one 1 GiB page, and measured on this host 2 GiB and 4 GiB boot to the prompt while 8 GiB
  hangs before its first line of output (a panic before the console exists); the frame allocator
  keeps one 4-byte pin count per frame of the machine's address span in a fixed 32 MiB kernel heap;
  device memory at or above 4 GiB panics; every path argument is capped at 128 bytes; no storage
  stack exists yet and the page cache's page index is 32-bit. Seven planning items are proposed in
  the audit's section 5.
- **Scale targets, memory model — built 2026-11-28, awaiting the owner's review.** Branch
  `feat/x86_64-memory-model` (stacked on the two branches above; local, nothing pushed), four
  commits. `0098eed` (`port/src`, its own commit so the diff can be read by the rule above) keeps
  a pin count only for pinned frames and caps the clean target and the low-water mark at 64 MiB
  and 16 MiB. `1e4ebe7` maps the RAM above the first gibibyte into the PML4 slots the boot tables
  leave free (a direct map built at boot from the firmware map; `phys_ptr_mut` reaches every
  frame; every process PML4 copies the whole kernel half) and raises the firmware-map cap to 128
  entries and the boot-information stash to 32 KiB. `4a395e1` replaces the demand pager's per-page
  `Vec` on the kernel heap (found on the way: a process mapping 1 GiB exhausted the heap, after a
  quarter of a million linear scans) with a radix tree keyed by address, paged from the frame
  allocator. `ae04cd9` makes the per-fault console line opt-in (`faulttrace`) and prints a failed
  break's reason. Measured on the KVM host: guests of 8, 32, 100 and 128 GiB boot to the prompt
  with the whole RAM managed (first output in 0.6 to 2.2 s; before, 8 GiB panicked in the
  seeding), and a process writes and reads back 1 GiB through frames above 4 GiB with the kernel
  heap under 3 MiB (100 GiB and 128 GiB need a sparse file-backed guest; a 1 TiB guest does not
  start on this host). At 1 GiB on the same tree at the default `-smp 2` under KVM: tier-1 (at
  `-smp 1` too), churn and net-smoke pass; smoke passes the chain leg and the CPU ladder to
  `cpu-ether-ok` and stops at `cpu-keyfs-ok`, as `main` does at one CPU (the single-CPU TCG leg
  ends the same way, 31 stages); services and auth end as before.
  `cargo xtask test` exits 0 (1281); clippy exits 0 on q35churnshell and q35cpu. Still open from
  the audit: the kernel heap (one fixed 32 MiB arena; nothing in it now grows with RAM), the rest
  of the caps, device memory at or above 4 GiB on firmware that places BARs high, and two limits
  outside the kernel, recorded in the audit's section 7 (the user library's malloc caps a process
  near 3.8 GiB; exec takes 32 arguments of 128 bytes). The three targets are recorded as SCALE-01
  to SCALE-03 in `.planning/REQUIREMENTS.md`; SCALE-01 is ticked. Record: `.planning/SCALE-AUDIT.md`
  section 7.
- **Pre-existing and carried:** `cargo xtask clippy --arch aarch64 --config virtshell` fails on
  7 undocumented `unsafe` blocks in `aarch64/`; repo-wide `cargo fmt --all -- --check` is red;
  the docs still pin r9 at `c787ce9` while `crypto/`, `ds/` and the wire foundation now exist —
  re-verify every landing-site path when its phase is planned.

### Standing Owner Rules

- Pushes in either repo need explicit owner approval; check what is unpushed and push only the approved commits.
- Edits under r9 `port/src` need owner approval (show the diff first). `aarch64/`, `x86_64/`, `xtask/` edits do not.
- x86_64 kernel byte change, until `boot-iso` is a pure derivation (CORR-02): re-derive and commit the `boot-iso` `outputHash`, run the x64 TCG gates in an amd64 Docker container (clean up afterwards), and get the owner's linux boot-iso check.
- x86_64 kernel byte change, once CORR-02 has landed: green CI on the commit is the check; no hash, no owner boot-iso check (owner, 2026-11-25).
- No phase reintroduces software RAM encryption, `PKEY`/`WRPKRU` page-fault hooks, or Rust rewrites of `6c`/`6a`/`6l` without a recorded owner decision first (v3.0 guide rule 4).
- Phase 5 only: the `port/src` diff-first rule is waived; the owner reviews the cumulative `port/src` diff at four owner stops and before any push (05-CONTEXT.md D-30, D-32).
- aarch64 kernel byte change: quick pass now (tier-1 on HVF and TCG; smoke, net-smoke, cpu once on HVF); full HVF series at Phase 8. Kernel builds are reproducible, so compare kernel sha256 to prove a pin move changes nothing.
- Kernels are built without FP/SIMD; kernel SIMD goes through `fpu::kernel_begin()` / `with_simd()`; user FP state is per process (save at switch, copy on fork, reset on exec); arm64 is the reference for x86_64.
- No AI attribution in commits or PRs. Never stage the 11 pre-modified troff/postscript font files in Harvey (a darwin case-collision artifact).

### Practical Notes

`gh` is not installed (`nix run nixpkgs#gh -- ...`). GNU sed is on PATH. macOS Unix socket paths cap at 104 bytes: run boot tests from a short path. The lima VM `harvey` is the Harvey namespace build venue (`core.ignorecase=false` on purpose). r9 exec caps each argument at 128 bytes; the PL011 console buffer is 256 bytes. Debug history: `.planning/legacy/debug/knowledge-base.md` (frozen).

## Deferred Items

Items carried over from the legacy-final checkpoint (HANDOFF.md):

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| Verification | Full arm64 HVF series (smoke 1753, net-smoke 36, tier-1 10, cpu 36, plus 3 TCG each; about 4.4 h) | Deferred by owner to Phase 8 (SMP-05) | legacy-final | v3.2 |
| Defect | WINDOWS #70: aarch64 scheduler resume skips note delivery | Deferred to Phase 8 (SMP-06) | legacy-final | v3.2 |
| Defect | WINDOWS #49: I-cache, root-caused and fixed | Left open until the full series runs (SMP-06) | legacy-final | v3.2 |
| Defect | WINDOWS #66: x86_64 triple fault under 8-way soak | Fix on an x86_64 host venue (SMP-06) | legacy-final | v3.2 |
| Test gap | `#!` recursion cap has no host or live test | Unscheduled; add a test when exec is next touched | legacy-final | - |
| Prerequisite | SMP: aarch64 per-CPU current frame, kernel stack, proc pointers; x86_64 per-CPU `Mach` mapping (FP ownership already per-CPU) | Phase 5 (SMP-02) | legacy-final | v3.2 |
| Lint | `cargo xtask clippy --arch aarch64 --config virtshell` fails at HEAD on 7 undocumented unsafe blocks in `aarch64/` (`platform/virt/rtc.rs:75`, `proc.rs:439`, `470`, `1823`, `2753`, `textcopy.rs:158`, `172`); `UNSAFE-AUDIT.md`'s aarch64 count of 129 may be stale | Pre-existing; found by Phase 3 research; outside Phase 3 | Phase 3 | - |
| Design | TPHT writer assigns tiny pointers 1..=16 only; compendium 4.3 neighbourhood displacement (17..=255) and clock-nibble eviction are not built | Not a Phase 3 requirement; for owner review | Phase 3 | - |
| Deferred item | 03/deferred-items.md: repo-wide `cargo fmt --all -- --check` is red (pre-existing formatting drift) | acknowledged | 2026-11-25 | v3.0 |

The WINDOWS ledger (51 open, 4 waived, 17 fixed, 72 total) is frozen at `.planning/legacy/WINDOWS.md`; re-home open entries to a live file before updating them.

## Session Continuity

Last session: 2026-11-26T00:56:30.804Z
Stopped at: Phase 5 context gathered
Resume file: .planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-CONTEXT.md

## Operator Next Steps

- Plan Phase 5 with /gsd-plan-phase 5
