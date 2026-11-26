# Roadmap: r9

## Overview

r9 is already a Rust kernel that boots Harvey's C userland to a shell, CPU server and rio on QEMU q35. This roadmap takes it from there to a self-hosting system with Harvey archived. v3.0 built the cross-build harness (Stage P0), the design lock, `crypto/` and `ds/`, and the `9P2000.aead` wire. From here the order follows the owner's v3.0 documents of 2026-11-25: a minimal two-CPU SMP bring-up first (opened by five course-correction tasks), then the distributed 9P extensions and discovery, process lifecycle and the riscv64 sync, SMP scale-out, the native `std` (`libstd_r9`), the Harvey userland port (Track P), storage, graphics and the self-hosting toolchain, and a last phase that archives Harvey.

**Rescope of 2026-11-25.** The owner's documents `docs/plan_r9_architecture_code_compendium_v3_0.md` and `docs/r9_implementation_guide_for_agentic_developers_v3_0.md` supersede the earlier guide and compendium. Software cryptographic memory is dropped (hardware MMU isolation only); `FidTable` moves to a sharded generational slab; SMP is pulled forward; Harvey's C compilers and classic APE are not ported to Rust. No phase may reintroduce software RAM encryption, `PKEY`/`WRPKRU` page-fault hooks, or Rust rewrites of `6c`/`6a`/`6l`; a proposed deviation needs a recorded owner decision first. The planning written for the old Phase 5 is archived under `.planning/milestones/superseded/`.

Phase order is the owner's plan and overrides any order inferred from the source documents. GSD phase numbers are sequential integers and keep their slots across the rescope; each phase title keeps a source label so it traces back. "Guide Phase N" now means the v3.0 guide's numbering.

| GSD phase | Source label (v3.0 guide) | | GSD phase | Source label |
|-----------|---------------------------|-|-----------|--------------|
| 1 | Stage P0 | | 9 | Milestone STD |
| 2 | Guide Phase 0 | | 10 | Track P, Stage P1 |
| 3 | Guide Phase 1 | | 11 | Track P, Stage P2 |
| 4 | Guide Phase 2 | | 12 | Track P, Stage P3 |
| 5 | Guide Phase 3 (Early SMP) and the Phase 0-2 delta | | 13 | Guide Phase 6 |
| 6 | Guide Phase 4 | | 14 | Guide Phase 7 |
| 7 | Guide Phase 5 | | 15 | Guide Phase 8 |
| 8 | SMP scale-out (owner, 2026-11-25) | | 16 | Final (Harvey retirement) |

Structure note: the standard granularity band (4-6 phases) would compress this, but the owner's plan fixes the 16-phase structure. Phase 10 (Track P kernel devices) may run concurrently with Phases 6 and 7; Phases 11 and 12 wait for `libstd_r9`. Phases 10 to 12 are expected to split into many plans.

## Milestones

- ✅ **v3.0 Cross-Build Harness and Wire Foundations** - Phases 1-4 (shipped 2026-11-25)
- 🚧 **v3.1 Early SMP and Cluster Fabric** - Phases 5-7 (in progress, started 2026-11-25; rescoped the same day)
- **v3.2 SMP Scale-Out and Native std** - Phases 8-9 (planned)
- **v3.3 Harvey Userland Port (Track P)** - Phases 10-12 (planned)
- **v3.4 Storage and Graphics** - Phases 13-14 (planned)
- **v4.0 Self-Hosting and Harvey Retirement** - Phases 15-16 (planned)

Milestone numbering continues from the closed Harvey-rooted milestone v2.1.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): planned milestone work
- Decimal phases (2.1, 2.2): urgent insertions (marked INSERTED)

<details>
<summary>✅ v3.0 Cross-Build Harness and Wire Foundations (Phases 1-4) — SHIPPED 2026-11-25</summary>

**Milestone Goal:** The golden invariant is an r9-owned gate, Harvey is a read-only supply of C artifacts that Rust replaces one at a time, the design decisions are locked, and the `crypto/`, `ds/` and `9P2000.aead` wire foundations exist and are pinned by host tests.

- [x] **Phase 1: Stage P0 - Cross-Build Harness** - 34/34 plans — completed 2026-11-23
- [x] **Phase 2: Guide Phase 0 - Design Lock and rfork Flag Reservation** - 6/6 plans — completed 2026-11-24
- [x] **Phase 3: Guide Phase 1 - crypto/ and ds/ Foundations** - 13/13 plans — completed 2026-11-24
- [x] **Phase 4: Guide Phase 2 - 9P2000.aead Transport and FidTable** - 25/25 plans — completed 2026-11-25

**Archive:** [ROADMAP](milestones/v3.0-ROADMAP.md) · [REQUIREMENTS](milestones/v3.0-REQUIREMENTS.md) · [AUDIT](milestones/v3.0-MILESTONE-AUDIT.md)

78 plans, 180 tasks, 38/38 requirements complete. UAT: 112/112 on Phase 4, 3/3 on Phases 2 and 3. Phase 1 was verified by its own report and the owner witness stops (infrastructure phase, no UAT).

</details>

### 🚧 v3.1 Early SMP and Cluster Fabric (In Progress)

**Milestone Goal:** `main` is green and aligned with the v3.0 documents, the kernel schedules on two CPUs on both architectures with its shared structures proven under real concurrency, the cluster can share leased state and discover its own services, and note delivery is stack-safe with riscv64 brought back in step.

- [ ] **Phase 5: Guide Phase 3 - Early SMP, Course Correction and Memory Pools** - five course-correction tasks and the audit follow-ups first, then two-CPU bring-up on virt and q35 with fine-grained locking, per-CPU run queues and TLB shootdown, then demand loading, the Roaring page pools with a page-dropping evictor, the TPHT page cache, SIMD set kernels, and the 2-CPU stress gate
- [ ] **Phase 6: Guide Phase 4 - Distributed 9P Extensions and Service Discovery** - compound, streams, leases, union mounts, legacy gateway, beacon and discover with 30s/5m lifecycle
- [ ] **Phase 7: Guide Phase 5 - Process Lifecycle, Stack-Safe Notes and riscv64 Sync** - checked stack-bound note delivery, riscv64 at parity with `port::rfork` and the SMP traits

#### Phase 5: Guide Phase 3 - Early SMP, Course Correction and Memory Pools
**Goal**: `main` is green and matches the v3.0 documents, and r9 schedules on two CPUs on x86_64 q35 and aarch64 virt with per-CPU state, interrupt-safe locking and TLB shootdown, so that every concurrent structure is exercised under real multi-core interrupts before the distributed and storage work is built on it.
**Depends on**: Phase 4
**Requirements**: CORR-01, CORR-02, CORR-03, CORR-04, CORR-05, DEBT-01, DEBT-02, DEBT-03, DEBT-04, DEBT-05, SMP-01, SMP-02, SMP-03, SMP-04, SMP-07, SMP-08, SMP-09, SMP-11, SMP-12, SMP-13, MEM-07, MEM-08, MEM-10, MEM-14, MEM-15, MEM-16, MEM-17, MEM-18
**Success Criteria** (what must be TRUE):
  1. CI on `main` is green; `boot-iso` builds as a pure derivation with no `outputHash` to maintain; `cargo xtask test` runs `test-9p` and the xtask tests; and the policy documents, the lint contract and `UNSAFE-AUDIT.md` describe the unsafe boundary as it is in the tree, with `x86_64` under the deny lint.
  2. `FidTable` is a sharded generational slab in safe Rust on a generic `ds/src/slab.rs`, with a server-assigned 8-byte `StatefulHandle` that is never all zero, lock-free seqlock slots whose `resolve` reads one cache line and returns a consistent snapshot, and the owning-client check kept; no `FlattenedTphtShard`, no table-wide insert lock and no all-shard probe remain in it; every `cargo xtask test-9p` leg is green; `ds/src/tpht/` is untouched. `RFCKEY`, `RFROBASE` and `RFRANDMEM` no longer exist and `RFNOLEGACY` is bit 18 in `port::rfork` and the generated header.
  3. Under `-smp 2`, both CPUs reach the scheduler loop on QEMU virt (aarch64, brought up first) and q35 (x86_64), each with its own per-CPU state and its own run queue, an idle CPU steals work from the other, no big kernel lock exists, every lock has a rank that debug and gate builds enforce, and every existing gate runs green at `-smp 2` with one `-smp 1` smoke leg kept per architecture.
  4. A two-CPU stress gate runs a user workload on both CPUs through a `9P2000.aead` mount served by the real `FidTable` (an in-kernel loopback server started only by a boot switch) plus exec churn through the TPHT page cache, with timer interrupts across both CPUs, and finishes with no deadlock; a watchdog would name a stuck CPU and lock; a TLB shootdown reaches the other CPU; every new kernel boot gate runs with `-smp 2` on both architectures.
  5. The page allocator keeps Roaring clean, dirty and pinned pools in which a pinned frame is never an eviction candidate, `SuperpageResidency` is exactly 64 bytes, and every `port/` path that holds user memory across a sleep pins its frames through a guard; all of it safe under two CPUs and none of it cryptographic.
  6. Both architectures load image pages on demand, map user text read-only and track dirty pages; the evictor, live in every boot, drops text and unwritten initialised-data pages under pressure and they fault back intact through the TPHT page cache, shown by a small-memory `-smp 2` gate on both architectures; the Roaring set kernels run AVX2 or NEON where the CPU has it with outputs identical to the scalar path; and x86_64 saves user FP state with XSAVE where AVX is enabled and with FXSAVE elsewhere.
**Plans:** 27 plans

Plans:
**Wave 1**
- [ ] 05-01-PLAN.md — Repair CI (`drawterm-gate.expect` `global env`, unmask all 7 gate jobs), pure `boot-iso` derivation, and `cargo xtask test` wiring (`CORR-01`, `CORR-02`, `DEBT-01`)

**Wave 2** *(blocked on Wave 1 completion)*
- [ ] 05-02-PLAN.md — `rfork` bit cleanup (`RFNOLEGACY = 1<<18`), v3.0 threat model sync, and Phase 4 validation/evidence closure (`CORR-04`, `DEBT-02`, `DEBT-03`, `DEBT-05`)
- [ ] 05-03-PLAN.md — Document all 7 `aarch64` and first 24 `x86_64` undocumented `unsafe` sites (`CORR-05` Part A)
- [ ] 05-05-PLAN.md — Generic `#![forbid(unsafe_code)]` Boehm seqlock slab in `ds/src/slab.rs` and `DEBT-04` TPHT production bank sizing (`CORR-03`, `DEBT-04`)

**Wave 3** *(blocked on Wave 2 completion)*
- [ ] 05-04-PLAN.md — Complete `x86_64` workspace deny-lint opt-in, `UNSAFE-AUDIT.md` reconciliation, `lintcontract` R6, and CI clippy legs (`CORR-05` Part B)

**Wave 4** *(blocked on Wave 3 completion)*
- [ ] 05-06-PLAN.md — Refactor `port/src/fid.rs` onto `ds::slab`, update `aead_link.rs`, D-13 three-place amendments, `CORR-05` policy doc sync, and **Owner Stop 1** (`CORR-03`, `CORR-05`)

**Wave 5** *(blocked on Wave 4 completion)*
- [ ] 05-07-PLAN.md — Portable SMP primitives (`port/src/{smp,mcslock,globalalloc}.rs` & arch `allocator.rs`): `IrqSpinLock`, `LockRank` table, `#[cfg(debug_assertions)]` watchdog, spin-poll `ShootdownCoordinator`, and SMP-safe `GlobalQuickAlloc` (`SMP-02`, `SMP-03`, `SMP-07`, `SMP-12`)

**Wave 6** *(blocked on Wave 5 completion)*
- [ ] 05-08-PLAN.md — Per-CPU work-stealing `RunQueues` (`port/src/sched.rs`), `FidTable` CPU-ID routing, and `port/` `BusyCell` & lost-wakeup fixes (`SMP-03`, `SMP-11`)

**Wave 7** *(blocked on Wave 6 completion)*
- [ ] 05-09-PLAN.md — `aarch64` 2-CPU hardware bring-up: `TPIDR_EL1` `PerCpu`, `mod smp`, `l.S` `secondary_start`, `trap.S` migration, PSCI `CPU_ON` (incl. `INVALID_PARAMETERS` `-smp 1` handling), and GICv2/v3 SGIs (`SMP-02`, `SMP-04`, `SMP-07`)

**Wave 8** *(blocked on Wave 7 completion)*
- [ ] 05-10-PLAN.md — `aarch64` single-CPU static & IRQ-shared audit (`proc`, `syscall`, `fpu`, `pagealloc`, `pl011` `CONS_RX`, `pci` `VIRTIO_NET`) and `RunQueues` + `wake_pending` wiring (`SMP-02`, `SMP-03`, `SMP-11`)
- [ ] 05-23-PLAN.md — Regenerate committed `aarch64/lib/virt-gicv{2,3,2-lowecam}.{dts,dtb}` with `-smp 2` so `/cpus` declares `cpu@0` and `cpu@1` (`SMP-04`, `SMP-09`)

**Wave 9** *(blocked on Wave 8 completion)*
- [ ] 05-22-PLAN.md — `aarch64` CPU 1 PSCI bring-up, per-CPU timers, SGI 0/1 dispatch, `R9_SMP`-aware `smoke-r9.expect`, and `-smp 2` HVF+TCG boot (`SMP-04`, `SMP-07`, `SMP-09`, `SMP-12`)

**Wave 10** *(blocked on Wave 9 completion)*
- [ ] 05-11-PLAN.md — `x86_64` 2-CPU hardware bring-up: ACPI MADT type-0/9, dual xAPIC/x2APIC ICR (`mfence; lfence`), `ap_trampoline.S`, `mod smp`, `%gs:0` `Mach`, and canonical `intrspecial` (`SMP-01`, `SMP-02`, `SMP-07`)

**Wave 11** *(blocked on Wave 10 completion)*
- [ ] 05-12-PLAN.md — `x86_64` single-CPU static & IRQ-shared audit (`proc`, `syscall`, `fpu`, `pagealloc`, `pci` `0xCF8/0xCFC` + `VIRTIO_NET`, `uart16550`), `RunQueues` + `wake_pending`, `ap_main` init, per-CPU LAPIC timer, and boot shootdown self-check (`SMP-01`, `SMP-02`, `SMP-03`, `SMP-07`, `SMP-11`, `SMP-12`)

**Wave 12** *(blocked on Wave 11 completion)*
- [ ] 05-13-PLAN.md — Migrate remaining `aarch64` gates, both `nix/gates/boot-r9*.sh` launchers, and first 3 `x86_64` smoke gates to `-smp 2` default with `R9_SMP` override (`SMP-01`, `SMP-04`, `SMP-09`)
- [ ] 05-24-PLAN.md — Migrate 7 core `x86_64/lib/config_q35*.toml` profiles (incl. CI `boot-test-x86-64` configs) to `smp = 2` (`SMP-01`, `SMP-09`)
- [ ] 05-25-PLAN.md — Migrate remaining 2 `x86_64/lib/config_q35*.toml` and first 5 `aarch64/lib/config_virt*.toml` profiles to `smp = 2` (`SMP-01`, `SMP-04`, `SMP-09`)
- [ ] 05-26-PLAN.md — Migrate remaining 7 `aarch64/lib/config_virt*.toml` profiles to `smp = 2` (`SMP-04`, `SMP-09`)

**Wave 13** *(blocked on Wave 12 completion)*
- [ ] 05-14-PLAN.md — Migrate remaining 7 `x86_64` gates to `-smp 2`, wire 1 `-smp 1` smoke leg/arch in CI, run `aarch64` quick pass, and **Owner Stop 2** (`SMP-01`, `SMP-04`, `SMP-09`)

**Wave 14** *(blocked on Wave 13 completion)*
- [ ] 05-15-PLAN.md — `x86_64` CPUID-gated AVX/XSAVE (832B `FpState`, wired into BSP & AP in `main.rs`) + `qemu64` FXSAVE fallback, CI TCG AVX2 leg, and **Owner Stop 3** (`MEM-18`)

**Wave 15** *(blocked on Wave 14 completion)*
- [ ] 05-16-PLAN.md — Portable Roaring `PageAlloc` pools + ~15% High-Water Pager, 64B `SuperpageResidency`, and Flattened-TPHT `palloc.hash` `PageCache` (`MEM-07`, `MEM-08`, `MEM-16`)

**Wave 16** *(blocked on Wave 15 completion)*
- [ ] 05-17-PLAN.md — Roaring AVX2 & NEON inline-asm kernels (`ds/src/setops_simd.rs`) behind `ds/src/setops.rs` + `port::setops` R4 boundary + `fpu::with_simd()` + `lintcontract` R3 (`MEM-17`)

**Wave 17** *(blocked on Wave 16 completion)*
- [ ] 05-18-PLAN.md — `aarch64` RO shared text via `palloc.hash`, `pagealloc.rs` Roaring pools, write-protect dirty tracking, live evictor, and `UserSlice`/`UserSliceMut` RAII `PinGuard` with EL1 promotion (`MEM-07`, `MEM-10`, `MEM-15`, `MEM-16`)

**Wave 18** *(blocked on Wave 17 completion)*
- [ ] 05-19-PLAN.md — `x86_64` full demand loading (`#PF` pager), `pagealloc.rs` Roaring pools, RO shared text, race-free `PTE_D` atomic-swap evictor, and `UserSlice`/`UserSliceMut` RAII `PinGuard` (`MEM-07`, `MEM-10`, `MEM-14`, `MEM-15`, `MEM-16`)

**Wave 19** *(blocked on Wave 18 completion)*
- [ ] 05-20-PLAN.md — Boot-switch `aeadloop=1` in-kernel `9P2000.aead` loopback server on real `FidTable` (`SMP-13`)

**Wave 20** *(blocked on Wave 19 completion)*
- [ ] 05-27-PLAN.md — Author `-smp 2` stress (`SMP-08`) & small-memory pressure (`MEM-15`) expect scripts + `nix/gates/*.sh` wrappers (`fdtput` `R9_BOOTARGS` on `aarch64`, `r9-iso-builder --append` on `x86_64`) (`SMP-08`, `SMP-09`, `MEM-10`, `MEM-15`, `MEM-16`, `MEM-17`)

**Wave 21** *(blocked on Wave 20 completion)*
- [ ] 05-21-PLAN.md — Wire stress & pressure Nix apps, `xtask`, and CI jobs on both arches, run `aarch64` HVF+TCG twice consecutively, and **Owner Stop 4** (`SMP-08`, `SMP-09`, `MEM-10`, `MEM-15`, `MEM-16`, `MEM-17`)
**Open questions**: Settled at the phase discussion on 2026-11-25; the decisions are D-01 to D-35 in `.planning/phases/05-guide-phase-3-early-smp-course-correction-and-memory-pools/05-CONTEXT.md`. Order: course correction and audit follow-ups, then SMP on aarch64 and x86_64, then the memory work. The phase is planned in one pass with four owner stops (after the course correction; after SMP; after the AVX/XSAVE change; at phase end); the `port/src` diff-first rule is waived for this phase with the cumulative diff reviewed at each stop. CI is the per-commit check for x86_64 and the SMP and stress gates also run under KVM on the owner's x86_64 Linux host at stops 2 to 4. Still to verify before planning: what is actually red at `900f58b`. `boot-iso` becoming a pure derivation changes the owner's byte-change rule: once it lands, green CI on the commit is the check for an x86_64 kernel byte change and no hash is committed; the old rule holds until then. An aarch64 kernel byte change gets the quick pass. For research to confirm: that the fence-paired seqlock is sound in safe Rust on all three architectures; how a dropped shared text page is unmapped from every address space; which `port/` paths sleep while holding user memory; whether CI can run an AVX2 CPU model under TCG in acceptable time. From the superseded Phase 5 research, to re-verify: `x86_64/` has 32 undocumented `unsafe` blocks in 9 files and aarch64 has 7; the x86_64 table in `UNSAFE-AUDIT.md` names files that no longer exist; `xtask/src/test9p.rs` pins the leg count; neither architecture tracks dirty pages; the page-in paths panic on allocation failure; `UserSlice` holds nothing that could release a pin. More than two CPUs, the full arm64 HVF series and the WINDOWS items are Phase 8. Deferred: the geometric chunk spine for slab shards (owner design, Phase 6 or later).

#### Phase 6: Guide Phase 4 - Distributed 9P Extensions and Service Discovery
**Goal**: Cluster nodes can run atomic compound operations, stream data with flow control, cache shared files under leases, mount unions in parallel, interoperate with legacy 9P peers, and find each other's services dynamically.
**Depends on**: Phase 5
**Requirements**: DIST-01, DIST-02, DIST-03, DIST-04, DIST-05, DIST-06, DIST-07, DIST-08, DIST-09, DIST-10, DIST-11, DIST-12
**Success Criteria** (what must be TRUE):
  1. A `Tcompound` script (walk, open, read) runs atomically and stops at the first error with the correct completed count; a `Tstream` sender never exceeds its granted credit.
  2. Three readers cache a leased file with no heap-allocated client set and generate zero network packets on repeat reads; a writer causes `Tlease_revoke` to exactly the other readers, the server blocks until acknowledgements or the 5-second timeout, and the next read sees the write; the revocation test passes under two CPUs.
  3. A 9front client mounts through the legacy bounce gateway, and a process created with `rfork(RFNOLEGACY)` (bit 18) cannot bind, mount or walk legacy-gateway channels.
  4. A beacon on one node populates `/srv/{hostname}_{class}_{instance}` on another over both IPv4 and IPv6.
  5. After 30 seconds of silence the peer's `.status` reads `unhealthy`, its `healthy_replicas` bit clears and its socket stays open; a heartbeat restores it; beyond 330 seconds its `/srv` entries are removed.
**Plans**: TBD
**Open questions**: W7 (`DiscoveryBeaconV2` stated 192 bytes, fields sum to 202). Discovery stays in this phase although the v3.0 guide's row does not name it (owner, 2026-11-25). Note N5 (`cmd/discover` authenticates via Factotum). Ring bypass (DIST-08) is placed here by the roadmapper. The v3.0 compendium says received `9P2000.aead` frames are verified and decrypted on receive into ordinary page-cache frames or `#M` ring buffers; there is no fault-time decrypt. Landing sites `sys/src/cmd/beacon/`, `sys/src/cmd/discover/`, `port/src/union.rs`, `port/src/legacy_9p.rs` do not exist at HEAD. Edits under `port/src` need owner approval.

#### Phase 7: Guide Phase 5 - Process Lifecycle, Stack-Safe Notes and riscv64 Sync
**Goal**: Notes are delivered only onto a genuine stack, and the riscv64 kernel is back in step with the portable layer.
**Depends on**: Phase 6
**Requirements**: PROC-04, RV-01, RV-02
**Success Criteria** (what must be TRUE):
  1. A note aimed at a frame that is not on a stack is refused, and stack-spoofing probes trap while frames stay within stack bounds.
  2. The riscv64 kernel uses `port::rfork` and implements the SMP traits the other two architectures implement, with no architecture-specific assumption leaking into `port/`.
  3. The riscv64 kernel builds and passes a boot smoke on QEMU.
**Plans**: TBD
**Open questions**: `RFNOLEGACY` enforcement is DIST-07 in Phase 6; the v3.0 guide lists it under both phases, and it is not repeated here. The `rfork` crypto flags and sibling key isolation (old PROC-01, PROC-02, PROC-03, PROC-05 and PT Suite 4) are retired with software cryptographic memory. Which riscv64 machine and gate the smoke uses, and whether riscv64 joins the deny lint and `UNSAFE-AUDIT.md` here, are planning decisions. Landing site `kern/src/notes.rs` does not exist at HEAD. Edits under `port/src` need owner approval.

### v3.2 SMP Scale-Out and Native std (Planned)

**Milestone Goal:** The kernel scales past two CPUs with the deferred verification closed, and Rust programs written with ordinary `std` run on r9.

- [ ] **Phase 8: SMP Scale-Out and Deferred Verification** - more than two CPUs under soak, the full arm64 HVF series, WINDOWS items closed
- [ ] **Phase 9: Milestone STD - libstd_r9 Native Standard Library** - `std` fs, net, thread, sync, process, alloc, time, env over Plan 9 syscalls

#### Phase 8: SMP Scale-Out and Deferred Verification
**Goal**: r9 runs on more than two CPUs without deadlock or fault, and the verification the owner deferred to the SMP work is closed.
**Depends on**: Phase 5, Phase 7
**Requirements**: SMP-05, SMP-06, SMP-10
**Success Criteria** (what must be TRUE):
  1. r9 boots on QEMU q35 x86_64 with more than two CPUs, all online and scheduling, reaches rio repeatedly, and an 8-way soak shows no deadlock and no triple fault.
  2. The full arm64 HVF series passes (smoke 1753, net-smoke 36, tier-1 10, cpu 36, plus 3 TCG each).
  3. WINDOWS #70 and #66 are fixed and #49 is closed, with the open-entry ledger kept in a live file.
**Plans**: TBD
**Open questions**: Two-CPU bring-up, per-CPU state and the lock audit moved to Phase 5 on 2026-11-25; this phase keeps what needs more CPUs or long venue time. The WINDOWS ledger is frozen under `.planning/legacy/WINDOWS.md` (51 open, 4 waived, 17 fixed, 72 total); re-home open entries to a live file before updating. The ~4.4 hour full HVF series is owner-run venue time. Edits under `port/src` need owner approval.

#### Phase 9: Milestone STD - libstd_r9 Native Standard Library
**Goal**: A Rust program using ordinary `std` builds for r9, runs as an `a.out` without `#![no_std]`, and uses files, networking, threads, processes, time and environment through Plan 9 primitives.
**Depends on**: Phase 5
**Requirements**: STD-01, STD-02, STD-03, STD-04, STD-05, STD-06, STD-07
**Success Criteria** (what must be TRUE):
  1. A Rust binary written with `std` builds for an r9 userland target, is converted to `a.out` by `xtask/src/elf2aout.rs`, and runs on r9 under q35.
  2. `std::fs` opens, reads, writes, stats, lists and removes files over 9P, and `std::net` connects and accepts over `/net/tcp` using both IPv4 and IPv6 dial strings.
  3. `std::thread` spawns and joins threads across CPUs, and `Mutex` contention sleeps and wakes through `rendezvous`.
  4. The `std` test suite passes on r9 under multi-CPU, with `std::process`, `env`, `time` and `alloc` covered.
**Plans**: TBD
**Open questions**: How `std` is built for the r9 target (custom target JSON with `-Zbuild-std` against the pinned nightly versus another mechanism) is a planning decision. `elf2aout` lands here, not in Phase 15, because the Phase 9 gate cannot be met without it; Phase 15 runs the same tool natively. Landing site `sys/src/libstd_r9/` does not exist at HEAD.

### v3.3 Harvey Userland Port - Track P (Planned)

**Milestone Goal:** The Harvey kernel devices, base CLI and complex userland run as idiomatic Rust, and the system boots to rio with Harvey C binaries present only as the named bootstrap set (the C compilers and classic APE).

- [ ] **Phase 10: Stage P1 - Track P Kernel Devices** - `#c #d #e #s #| #I #p` at Harvey parity, safe Rust
- [ ] **Phase 11: Stage P2 - Track P Base CLI** - `rc`, `bind`, `mount`, `cat`, `echo`, `ls` in Rust, removed from the fallback manifest
- [ ] **Phase 12: Stage P3 - Track P Complex Userland** - `sam`, `rio`, `upas`, `factotum` and the remaining userland and libraries; no compiler port

#### Phase 10: Stage P1 - Track P Kernel Devices
**Goal**: The kernel devices `#c`, `#d`, `#e`, `#s`, `#|`, `#I` and `#p` are complete, safe Rust at parity with Harvey, so Harvey userland needs nothing from a C device.
**Depends on**: Phase 5 (also the Phase 1 harness); may run concurrently with Phases 6 and 7
**Requirements**: KDEV-01, KDEV-02, KDEV-03, KDEV-04, KDEV-05
**Success Criteria** (what must be TRUE):
  1. A committed parity inventory compares Harvey's devtab with r9's devices for the seven devices, with no open gaps.
  2. Harvey's tier sweeps and the boot-to-rio gate pass with these devices, with no regression at any commit.
  3. `#I` serves the `/net` conversation interface (clone, ctl, data, listen, UDP headers) and `#p` serves the `/proc/<pid>` files that process tools read.
  4. Device logic in these seven devices contains no `unsafe`, with any floor exceptions documented.
**Plans**: TBD
**Open questions**: Several of these devices already exist in Rust under `port/src/` at HEAD (devcons, devdup, devenv, devsrv, devpipe, devip, devproc), so this phase is the parity gap, measured at planning time. Edits under `port/src` need owner approval.

#### Phase 11: Stage P2 - Track P Base CLI
**Goal**: The everyday command line (`rc`, `bind`, `mount`, `cat`, `echo`, `ls`) runs as Rust binaries and the Harvey versions leave the image.
**Depends on**: Phase 9, Phase 10
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, CLI-06, CLI-07
**Success Criteria** (what must be TRUE):
  1. The system boots to rio with `rc`, `bind`, `mount`, `cat`, `echo` and `ls` running as Rust binaries from the image and no Harvey copies of them present.
  2. Harvey's boot and rc scripts run unmodified under the Rust `rc`, and the Harvey tier sweeps that exercise these tools pass.
  3. The fallback manifest no longer lists the six tools, and each swap was its own commit with the golden gate green.
**Plans**: TBD
**Open questions**: None recorded.

#### Phase 12: Stage P3 - Track P Complex Userland
**Goal**: The remaining Harvey userland, including `sam`, `rio`, `upas` and `factotum`, runs as Rust, and the system boots to rio with no Harvey C binary in the image other than the named bootstrap set.
**Depends on**: Phase 11
**Requirements**: USER-01, USER-02, USER-03, USER-05, USER-06
**Success Criteria** (what must be TRUE):
  1. `sam`, `rio`, `upas` and `factotum` run as Rust binaries and the golden gate reaches a rio session on q35 with the Rust `rio`.
  2. The fallback manifest holds no C userland binary and no library object except the named bootstrap set (Harvey's `6c`, `6a`, `6l` and classic APE), and the root image boots to rio.
  3. Every Harvey command or library that was not ported has a recorded retirement reason.
**Plans**: TBD
**Open questions**: The Rust port of `6c`/`6a`/`6l` (old USER-04) is out of scope by the v3.0 documents; Harvey's C compilers and classic APE stay as bootstrap and compatibility binaries. The exact contents of the bootstrap set are fixed at planning. Stage P3 is large and is likely to need subdividing when planned. If `factotum` is pulled forward for Phase 6 (note N5), this phase inherits its completion.

### v3.4 Storage and Graphics (Planned)

**Milestone Goal:** Content-addressed storage with snapshots and garbage collection runs on the cluster and passes the permission suites, and rio renders hardware-accelerated 3D.

- [ ] **Phase 13: Guide Phase 6 - Storage Engine and Cluster Permission Suites** - BLAKE3 Venti, Chained-TPHT score index, Fossil snapshots, GC with TRIM, permission suites on minimega
- [ ] **Phase 14: Guide Phase 7 - Graphics (#G Venus and draw(3) Host Blit)** - native `devdraw`, VirtIO-GPU Venus, host blit, hardware triangle in rio

#### Phase 13: Guide Phase 6 - Storage Engine and Cluster Permission Suites
**Goal**: Storage is content-addressed by BLAKE3 with deduplication, snapshots and garbage collection, and the cluster's permission, lease and federation behaviour is verified on a 12-node testbed.
**Depends on**: Phase 5, Phase 6, Phase 12
**Requirements**: STOR-01, STOR-02, STOR-03, STOR-04, STOR-05, STOR-06, STOR-07, PERM-01, PERM-02, PERM-03, PERM-04, PERM-05
**Success Criteria** (what must be TRUE):
  1. Writing identical content twice stores one block and reads back by score, the score is the same before and after compression, and round trips across compression tiers return identical bytes.
  2. A score lookup verifies the full 32-byte score, so a collision on the folded 64-bit hint never returns the wrong data.
  3. A Fossil snapshot sends its surviving roots to Venti, mark-and-sweep reclaims unreferenced blocks, and TRIM is issued for contiguous purged runs.
  4. PT Suites 1, 2 and 3 pass on the minimega testbed: pair and team file sharing with group leadership, lease coherence with anti-splicing, and two-domain federation with unregistered users mapped to UID 9999.
  5. PT Suite 5 (mail and chat visibility) passes, or has been explicitly deferred.
**Plans**: TBD
**Open questions**: W8 (UID 0 `adm` versus `glenda`), W9 (`alice` present but test needs her absent), W10 (Suite 3 mounts `venti03` across VLANs with no route; `archive.r9` server unnamed). Note N6 (`chat-rs` is defined in no document, so Suite 5 needs a definition or deferral). PT Suite 4 (crypto sibling isolation) is retired with software cryptographic memory. Snapshot GC is verified under SMP. The `TagEntry` nonce field (AC superset) is decided at planning. Landing site `venti/` does not exist at HEAD.

#### Phase 14: Guide Phase 7 - Graphics (#G Venus and draw(3) Host Blit)
**Goal**: rio draws natively on q35 and a Vulkan program renders on the host GPU into a rio window without CPU copies.
**Depends on**: Phase 12
**Requirements**: GFX-01, GFX-02, GFX-03, GFX-04, GFX-05
**Success Criteria** (what must be TRUE):
  1. rio draws natively on q35 through the kernel `devdraw` and 2D framebuffer, with no drawterm client needed to see a window.
  2. VirtIO-GPU negotiates features 0, 3 and 4 and selects Venus capset 4, and `/dev/gpu/clone` yields a context whose `cmd`, `blob` and `fence` nodes work.
  3. Compositing two images that both have host resource ids stays in host VRAM through a host blit, and falls back to CPU memdraw otherwise.
  4. A hardware-rendered triangle appears in a rio window on QEMU.
**Plans**: TBD
**Open questions**: The `#G` QID path wording ("upper 24 bits") is governed by the formula `((ctx_id & 0xFFFFFF) << 8) | (node_type & 0xFF)`. Landing sites `kern/src/gpu.rs` and `port/src/devgpu.rs` do not exist at HEAD. Edits under `port/src` need owner approval.

### v4.0 Self-Hosting and Harvey Retirement (Planned)

**Milestone Goal:** r9 compiles itself and Harvey is archived.

- [ ] **Phase 15: Guide Phase 8 - Self-Hosting Toolchain** - rustc, rust-lld, vendored APEX, Clang, elf2aout, r9 builds r9
- [ ] **Phase 16: Final - Harvey Retirement** - deprecation checklist passes with Harvey absent, then Harvey is archived

#### Phase 15: Guide Phase 8 - Self-Hosting Toolchain
**Goal**: r9 builds its own kernel and userland on r9, with Rust, C and C++ toolchains running natively.
**Depends on**: Phase 9, Phase 12, Phase 13
**Requirements**: SELF-01, SELF-02, SELF-03, SELF-04, SELF-05, SELF-06
**Success Criteria** (what must be TRUE):
  1. `rustc` compiles a Rust program on r9, `rust-lld` links it to a static ELF, the in-tree tool wraps it into `a.out`, and it runs.
  2. Modern C++/LLVM tooling builds with Clang against the vendored `vendor/apex/` runtime and runs on r9.
  3. Harvey's `6c`/`6a`/`6l` and classic APE run on r9 as carried bootstrap binaries and still build native Plan 9 C software, and the two C runtimes are never mixed in one binary.
  4. r9 compiles its own kernel and userland on r9, and the resulting system boots to rio.
**Plans**: TBD
**Open questions**: `vendor/apex/` does not exist at HEAD. The APEX syscall stubs are generated from `sys/src/libc/9syscall/sys.h`, a Harvey path, so the source of that header is settled at planning time.

#### Phase 16: Final - Harvey Retirement
**Goal**: Harvey is no longer needed and is archived read-only.
**Depends on**: Phase 14, Phase 15 (and transitively every earlier phase)
**Requirements**: RETIRE-01, RETIRE-02, RETIRE-03, RETIRE-04, RETIRE-05
**Success Criteria** (what must be TRUE):
  1. Every device in the devtab is safe Rust, confirmed by an inventory scan that covers more than the seven Stage P1 devices.
  2. The root image contains no Harvey C binary other than the named bootstrap set (`6c`, `6a`, `6l` and classic APE), confirmed by the fallback manifest and an image scan.
  3. With the Harvey tree absent, `rustc` and `rust-lld` run natively, modern C++/LLVM builds via Clang against `vendor/apex/`, and the bootstrap set r9 carries (Harvey's C compilers and classic APE, as binaries) still builds native Plan 9 C software.
  4. Every composed gate that Harvey's flake hosted (boot-r9-x64 smokes, net smoke, auth, drawterm, rio) runs from the r9 repo without Harvey, and the golden-invariant gate is green.
  5. `vendor/harvey/` is removed, the Harvey repository is archived read-only, and the r9 README carries the deprecation banner.
**Plans**: TBD
**Open questions**: The checklist was amended on 2026-11-25: the C compilers and classic APE are not ported, so they remain as carried bootstrap binaries and "zero Harvey C binaries" no longer applies to them; where r9 keeps those binaries once `vendor/harvey/` is removed is settled at planning. Note N4 (the composed gates are the main hidden dependency). Archiving the Harvey repository and any push need explicit owner approval.

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → … → 16 (decimal insertions, if any, run between their surrounding integers)

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Stage P0 - Cross-Build Harness | v3.0 | 34/34 | Complete | 2026-11-23 |
| 2. Guide Phase 0 - Design Lock and rfork Flag Reservation | v3.0 | 6/6 | Complete | 2026-11-24 |
| 3. Guide Phase 1 - crypto/ and ds/ Foundations | v3.0 | 13/13 | Complete | 2026-11-24 |
| 4. Guide Phase 2 - 9P2000.aead Transport and FidTable | v3.0 | 25/25 | Complete | 2026-11-25 |
| 5. Guide Phase 3 - Early SMP, Course Correction and Memory Pools | v3.1 | 0/22 | Planned | - |
| 6. Guide Phase 4 - Distributed 9P Extensions and Service Discovery | v3.1 | 0/TBD | Not started | - |
| 7. Guide Phase 5 - Process Lifecycle, Stack-Safe Notes and riscv64 Sync | v3.1 | 0/TBD | Not started | - |
| 8. SMP Scale-Out and Deferred Verification | v3.2 | 0/TBD | Not started | - |
| 9. Milestone STD - libstd_r9 Native Standard Library | v3.2 | 0/TBD | Not started | - |
| 10. Stage P1 - Track P Kernel Devices | v3.3 | 0/TBD | Not started | - |
| 11. Stage P2 - Track P Base CLI | v3.3 | 0/TBD | Not started | - |
| 12. Stage P3 - Track P Complex Userland | v3.3 | 0/TBD | Not started | - |
| 13. Guide Phase 6 - Storage Engine and Cluster Permission Suites | v3.4 | 0/TBD | Not started | - |
| 14. Guide Phase 7 - Graphics (#G Venus and draw(3) Host Blit) | v3.4 | 0/TBD | Not started | - |
| 15. Guide Phase 8 - Self-Hosting Toolchain | v4.0 | 0/TBD | Not started | - |
| 16. Final - Harvey Retirement | v4.0 | 0/TBD | Not started | - |
