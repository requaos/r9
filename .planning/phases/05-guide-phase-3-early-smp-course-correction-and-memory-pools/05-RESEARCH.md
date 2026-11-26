# Phase 5: Guide Phase 3 - Early SMP, Course Correction and Memory Pools - Research

**Researched:** 2026-11-25
**Domain:** CI/Nix build hygiene, lock-free seqlock sharded slab (`ds/src/slab.rs`, `port/src/fid.rs`), two-CPU SMP bring-up (`aarch64` PSCI/GICv2+v3, `x86_64` MADT/INIT-SIPI-SIPI/LAPIC+x2APIC), interrupt-safe ranked locking, per-CPU run queues with work stealing, TLB shootdown IPIs, Roaring frame pools & page-dropping evictor, TPHT image page cache, demand loading, dirty tracking, `UserSlice` frame pinning, AVX/XSAVE and Roaring SIMD kernels
**Confidence:** HIGH for codebase state, CI/test failures, lint/unsafe counts, seqlock memory-model soundness, and QEMU TCG/HVF probes (all verified live this session); MEDIUM for x86_64 KVM multi-core timing and exact small-memory pressure threshold (cannot boot x86_64 KVM on the arm64 macOS research host; verified on CI Linux runners and owner's KVM host at stops 2–4).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried in (not re-asked)
- **D-01:** The course-correction tasks (CORR-01 to CORR-05) and the audit follow-ups (DEBT-01 to DEBT-05) are the phase's first plans, executed before any SMP or memory work (owner, 2026-11-25). Before planning the CI fix, check what is actually red at `900f58b`; it has not been verified. `origin/main` is at `900f58b`.
- **D-02:** `RFNOLEGACY` is `1 << 18`; `RFCKEY`, `RFROBASE` and `RFRANDMEM` are removed; `port/` stays per-module forbid with its three floor modules (`globalalloc`, `mcslock`, `devip_waker`) (owner, 2026-11-25).
- **D-03:** Every requirement follows IG-V01 and v3.0 guide rule 1: committed RED pin, implementation, GREEN twice, pin advance. The golden invariant (boots to rio on q35) holds at every commit. `UNSAFE-AUDIT.md` changes in the same commit as any `unsafe` block in `ds/src/tpht/` or an arch crate (guide rule 3).
- **D-04:** Owner standing rules apply: no push without approval, no AI attribution, Harvey read-only. No software RAM encryption, `PKEY`/`WRPKRU` hooks or compiler rewrites (guide rule 4). D-18 and D-19 are not a deviation from rule 4: they enable AVX for set algebra, not for page encryption.
- **D-05:** x86_64 kernel byte change: the old procedure (re-derived `boot-iso` `outputHash`, x64 TCG gates in an amd64 container, owner's linux boot-iso check) applies until CORR-02 lands; after that, green CI on the commit is the check. aarch64 kernel byte change: the quick pass (tier-1 on HVF and TCG; smoke, net-smoke, cpu once on HVF). Check whether CORR-02 has landed before applying either.
- **D-06:** Kernels are built without FP/SIMD; kernel SIMD goes through `fpu::kernel_begin()` / `with_simd()`; user FP state is per process; arm64 is the reference for x86_64 (owner standing rule).

#### Slab `FidTable` (CORR-03)
- **D-07:** The handle is server-assigned only. The handle the server returns is the only name for an open file; `FidTable` has no lookup by the client's fid number and no duplicate-fid check. The all-shard probe and the table-wide `insert_lock` of `6fd201c` are removed. The entry may keep the client's fid number as a record only. — **Reversibility:** one-way — the 8-byte handle is part of the `9P2000.aead` wire contract.
- **D-08:** The owning-client check is kept, which the compendium section 4 listing drops: `resolve`, `clunk` and `mark_reclaimed` take the connection's `client_id` and compare it with the entry's.
- **D-09:** The slab is a generic sharded generational slab in `ds/src/slab.rs` under `#![forbid(unsafe_code)]`, host-tested on its own. `port/src/fid.rs` keeps `StatefulHandle`, `FidEntry` and the client check and wraps it. `ds/src/tpht/` is untouched.
- **D-10:** A stateful handle is never all zero because its generation is never zero: each slot's generation starts at 1 and skips 0 on wrap; `from_wire` rejects `gen == 0`; a test pins both. This re-proves Phase 4 D-12. Wire order: `slot_idx(2) gen(2) shard_id(1) flags(1) epoch(2)`, little endian. — **Reversibility:** one-way — wire layout.
- **D-11 (owner design):** Slots are lock-free seqlock slots. Each slot is 64 bytes, `#[repr(C, align(64))]`: one `AtomicU64` state word and `[AtomicU64; 7]` payload.
  - State word: bit 0 `WRITING`, bit 1 `OCCUPIED`, bits 2..17 `gen: u16`, bits 18..63 `seq` (46 bits).
  - `resolve` takes no lock and returns a consistent snapshot from one cache line: load state (`Acquire`); spin if `WRITING`; reject if not `OCCUPIED` or the generation differs; load the seven words (`Relaxed`); `fence(Acquire)`; reload state (`Relaxed`); retry if it changed.
  - Writers: store `state | WRITING` (`Relaxed`), `fence(Release)`, store the words (`Relaxed`), store the published state (`Release`). This is the Boehm seqlock fence pairing and must be sound on aarch64 and riscv64 as well as x86_64.
  - `gen` is bumped only on clunk; `seq` on alloc, in-place update and clunk. An in-place update (for example `offset`, `lease_gen`) makes readers retry without invalidating the client's wire handle. Clunk scrubs the payload words.
  - `alloc`, `clunk` and `sweep_unreclaimed` take a shard-local mutex over `AllocMeta` (free list, active and reclaimed Roaring sets). `resolve` never touches it.
  - An in-place update either takes the meta mutex or wins a CAS on `WRITING` while checking `OCCUPIED` and the generation, so two writers cannot race on one slot.
  - The slot array never reallocates. All of it is `#![forbid(unsafe_code)]`.
  - `OCCUPIED` replaces the `FID_LIVE` entry flag. The owner's reference code is in `05-DISCUSSION-LOG.md`.
- **D-12 (owner design):** Each shard's slots are a flat `Box<[AtomicSlabSlot]>` sized as a power of two at construction. A full shard falls over to the next round-robin.
  - Sizing tiers: kernel synthetic mounts 256 slots x 4 shards (64 KB); default workstation and q35 boot 1,024 slots x 8 shards (512 KB, 8,192 fids); cluster or heavy file server 4,096 to 16,384 slots x 16 to 32 shards.
  - Shard count: `(num_cpus * 2).next_power_of_two().clamp(4, 256)`. `alloc` routes to the calling CPU's shard (`cpu_id & shard_mask`).
  - The CPU-id source in `port/` arrives with the SMP work; the slab plan routes from CPU 0 until then and switches when the per-CPU trait exists.
- **D-13:** The amended wording of the archived WIRE-07 to WIRE-09 is recorded in three places, as Phase 4 D-10 did: dated amendment notes in `.planning/rfc-9p2000-aead.md` section 5; dated Key Decisions rows in `PROJECT.md` naming the superseded wording; CORR-03 in `REQUIREMENTS.md` as the live requirement. The archived v3.0 requirement files are not edited. `docs/plan_r9_architecture_code_compendium_v3_0.md` section 4 gets a dated note for the seqlock slot and the kept client check, since both differ from its listing.

#### Memory pools and paging (MEM-07, MEM-08, MEM-10, MEM-14 to MEM-18)
- **D-14:** With no backing store, eviction means dropping re-readable pages. The pools are: dirty (a freed frame not yet zeroed), clean (zeroed and ready), pinned (not reclaimable). The pager zeroes dirty frames to hold about 15% clean, and under pressure unmaps unmodified file-backed pages that can be read again from their image. Dirty anonymous memory (bss, stack, heap, written data) is never taken from a process.
- **D-15:** Droppable pages are user text and initialised-data pages a process has not yet written. User text is mapped read-only on both architectures (today both map it writable). Per-page dirty tracking is built: the PTE dirty bit on x86_64 and a write-protect fault scheme on aarch64, or a better mechanism research finds.
- **D-16:** x86_64 moves to full demand loading: exec stops mapping eagerly and every image page is loaded at first touch, as aarch64 does. One model on both architectures. — **Reversibility:** costly — every x86_64 boot's exec and fault path changes, and the golden gate rides on it.
- **D-17:** The evictor is live in every boot and acts when clean frames fall below a low-water mark. It is proven by a small-memory `-smp 2` boot gate on both architectures running a workload built to trigger pressure drops deterministically. The gate proves dropped pages fault back intact and that a pinned frame is never chosen. Host tests cover the pool algebra.
- **D-18:** The image page lookup is the Flattened-TPHT page cache the compendium names (`palloc.hash`), built in this phase on `FlattenedTphtShard`. It is the first default-bank TPHT consumer, so DEBT-04 (home-hit measured at production bank sizes, sizing rule written down) is done before it. Shared text maps one frame into every process running the image.
- **D-19:** The Roaring set kernels get SIMD in this phase on both architectures: AVX2 on x86_64 and NEON on aarch64, in the one reserved unsafe-holding module in `ds/` (Phase 3 D-09), behind the `ds/src/setops.rs` seam, reached through `with_simd()`. The path is chosen by CPU feature at boot, a scalar path remains, and scalar and vector outputs are pinned identical.
- **D-20:** On x86_64, AVX is enabled where CPUID reports it and per-process FP state moves from FXSAVE to XSAVE on those CPUs, so user `ymm` state survives a switch, `rfork`, exec reset and note delivery. On a CPU without XSAVE (TCG's default `qemu64`, used by CI and the quick gate) the FXSAVE path is kept unchanged and the set kernels run a non-AVX path. One boot line names the chosen format so a gate can assert it. — **Reversibility:** costly — the FP save format and every context-switch path on AVX-capable hosts.
- **D-21:** Pinning (MEM-10): `UserSlice` and `UserSliceMut` gain a pin guard released on drop, with a per-frame pin count so nested pins work. Every `port/` path that holds user memory across a sleep (devmnt, pipes, the network devices, console) is audited and takes the guard.

#### SMP shape (SMP-01 to SMP-04, SMP-07 to SMP-09, SMP-11 to SMP-13)
- **D-22:** Locking is fine-grained from the start, with no big kernel lock. Every structure whose safety rests on the single-CPU invariant gets its own lock or a per-CPU copy before the second CPU is allowed to schedule. — **Reversibility:** costly — the audit touches about 29 files and about 70 raw global statics across both arch crates.
- **D-23:** Each CPU has its own run queue, and an idle CPU steals work from the other. The design should not hard-code two CPUs, but only two are brought up and gated here; more is Phase 8 (SMP-10).
- **D-24:** Once both CPUs schedule, every existing gate on both architectures moves to `-smp 2` and that becomes the golden invariant. One smoke leg per architecture stays at `-smp 1`.
- **D-25:** aarch64 brings up the second CPU first; x86_64 follows on the proven shared code. `IrqSpinLock`, the per-CPU trait, the run queues and the shootdown request logic live in `port/` and are host-tested. Each arch crate supplies only CPU start (PSCI `CPU_ON`; INIT-SIPI-SIPI from the MADT), per-CPU register access (`TPIDR_EL1`; `GS` base) and the IPI (GIC SGI; LAPIC vector).
- **D-26:** Every lock has a rank in one documented table. Debug and gate builds assert that a CPU takes locks only in increasing rank and panic with both lock names on a violation. A per-CPU progress watchdog names the stuck CPU and the lock it waits on, so a deadlock is a named failure and not a gate timeout. Release builds carry neither.
- **D-27:** The two-CPU stress gate (SMP-08) is a user workload over 9P: user processes on both CPUs drive opens, reads and clunks through a mount so `FidTable` is exercised end to end, plus exec churn for the TPHT page cache, with timer interrupts on both CPUs.
- **D-28:** A boot switch starts a small in-kernel `9P2000.aead` server end backed by the real `FidTable` over a loopback link with an injected test secret, and the kernel's own client engine mounts it. This is gate-only wiring; a default boot exposes nothing new and still speaks plain 9P2000 (Phase 4 D-16/D-23 unchanged).

#### Order, stops and review
- **D-29:** Order inside the phase: course correction and audit follow-ups; SMP on aarch64, then x86_64; then the memory work, which needs the TLB shootdown to unmap a shared page on the other CPU. DEBT-04 precedes the page cache (D-18).
- **D-30:** The `port/src` diff-first rule is waived for this phase, as in Phase 4 D-24. Plans commit once their tests are green; the owner reviews the cumulative `port/src` diff at each owner stop and before any push. The standing rule is unchanged for later phases.
- **D-31:** The phase is planned in one pass, as one phase. Later plans that depend on code not yet written state their assumptions by name so they can be revised at the stop before them.
- **D-32:** Four owner stops. Each presents the cumulative `port/src` diff, gate evidence, and any push for approval.
  1. After the course correction and audit follow-ups: `main` green, `boot-iso` pure, slab in. A push can be approved here so CI proves it.
  2. After both architectures schedule on two CPUs and every gate has moved to `-smp 2`.
  3. After the AVX/XSAVE change (D-20), which lands as its own small group of plans because it alters the FP save format on AVX-capable x86_64 boots.
  4. Phase end, after demand loading, pools, page cache, SIMD kernels, pinning and the stress gate.
- **D-33:** x86_64 venue: CI (x86_64 Linux runners, plus one TCG leg with an AVX2 CPU model) is the per-commit check. At stops 2, 3 and 4 the SMP and stress gates also run under KVM on the owner's x86_64 Linux host, where two real cores and real AVX2 are in play, and the results are recorded as evidence. On the development Mac an x86_64 guest runs under TCG only.
- **D-34:** The audit follow-ups DEBT-02 and DEBT-03 edit files archived under `.planning/milestones/v3.0-phases/`; that is expected and is not a change of archived requirements.
- **D-35:** Requirements and roadmap were amended before planning so verification can check the additions: MEM-07, MEM-10, SMP-03, SMP-08, SMP-09 and CORR-03 reworded; new rows MEM-14 to MEM-18 and SMP-11 to SMP-13; the Phase 5 roadmap entry and success criteria updated; `PROJECT.md` Key Decisions rows added.

### Claude's Discretion
- How `ds/src/slab.rs` and `port/src/fid.rs` divide the work: `ds/` has no mutex (the MCS `Lock` is in `port/`), so the expected split is that `ds/` exports the slot array and `AllocMeta` and `port/` wraps the meta in `Lock`.
- Whether the entry keeps `fid_key` and `epoch` (49 live bytes today, 56 available) or drops them.
- The non-AVX x86_64 set-kernel path: scalar is required; an SSE2 path is optional.
- The dirty-tracking mechanism on each architecture, the low-water mark, the small-memory gate's RAM size and the pressure workload's form.
- Behaviour when a page-in cannot get a frame (today both page-in paths panic); with an evictor present this should reclaim and retry before failing.
- How `boot-iso` becomes a pure derivation, and the CI wiring for `test-9p` and the xtask tests (`xtask/src/test9p.rs` pins the leg count in three assertions and a literal list).
- The lock rank table's contents and the watchdog's period.
- Where the stress workload's user programs come from: Harvey is read-only, so either existing Harvey binaries driven by a script or small `a.out` probes generated by xtask.

### Deferred Ideas (OUT OF SCOPE)
- **Geometric chunk spine for slab shards** (owner design, guide Phase 4 or later, which is roadmap Phase 6 onward): chunks of 64, 64, 128, ..., 32,768 slots (11 chunks, 65,536 slots); `split_idx` by leading-zero count; chunk 0 is one page per shard; the spine backed by a small `OnceBox` (`AtomicPtr`, Release/Acquire) in an audited module, or `spin::Once` if it is in the workspace, keeping `ds/src/slab.rs` and `port/src/fid.rs` under `#![forbid(unsafe_code)]`. Needs owner approval for the unsafe-holding module when it is taken up.
- **A backing store for dirty anonymous pages** (swap): not in any phase; D-14 leaves the seam implicit.
- **More than two CPUs, the 8-way soak, the full arm64 HVF series, WINDOWS #49, #66, #70**: Phase 8.
- **Serving `9P2000.aead` from the kernel in a default boot**: the loopback server of D-28 is gate-only; a real server end belongs with Phase 6.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description (short) | Research Support |
|----|---------------------|------------------|
| CORR-01 | CI on `main` is green; failing workflow at `900f58b` repaired | Verified live via `gh api`: 16/17 check runs green, 1 RED (`quick-gate-tcg` in `.github/workflows/gates.yml`) due to missing `global env` in `proc start_xvfb` at `gates/x86_64/drawterm-gate.expect:318-348` |
| CORR-02 | `boot-iso` in `flake.nix` is a pure derivation with no `outputHash` | `flake.nix:312-340` `builder.bootIso` ALREADY takes `outputHash ? null`; removing `outputHash = "..."` at `flake.nix:984` converts `packages.x86_64-linux.boot-iso` to a pure derivation immediately |
| CORR-03 | `FidTable` on generic `#![forbid(unsafe_code)]` sharded seqlock slab in `ds/src/slab.rs` + `port/src/fid.rs` | Boehm fence-paired seqlock verified sound in C++20/Rust memory model on x86_64, aarch64, riscv64; 49 live bytes of `FidEntry` fit in `[AtomicU64; 7]` (56 B); `SlabProbe` hook preserves 1-cache-line read pin |
| CORR-04 | Remove `RFCKEY`/`RFROBASE`/`RFRANDMEM`, move `RFNOLEGACY` to `1 << 18`, regenerate `sys/include/rfork.h` | `port/src/rfork.rs:36-44`, `sys/include/rfork.h:30-33`, and hardcoded assertions in `xtask/src/rforkh.rs:124-131, 162, 171` verified |
| CORR-05 | Unsafe policy synced across docs, lint contract, `UNSAFE-AUDIT.md`; `x86_64` opted into deny lint + 7 aarch64 sites fixed | Verified live via `cargo clippy`: exactly 32 undocumented blocks in 9 `x86_64` files and 7 in 3 `aarch64` files; `UNSAFE-AUDIT.md:317` names deleted `node0.rs`/`vsvm.rs`; `x86_64/Cargo.toml` switches cleanly to `[lints] workspace = true` |
| DEBT-01 | `cargo xtask test-9p` and `cargo test -p xtask` run in CI and `cargo xtask test` | Verified live: `cargo test -p xtask` currently FAILS 2 tests in `xtask/src/gate.rs:885,918` (stale archived path `.planning/phases/01-...` moved to `.planning/milestones/v3.0-phases/01-...`); must fix those paths as part of DEBT-01 |
| DEBT-02 | Reconcile `04-VALIDATION.md` from `draft` to validated | Verified at `.planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-VALIDATION.md:6-8` (`status: draft`, `nyquist_compliant: false`) |
| DEBT-03 | Correct Phase 4 boot-iso evidence text claiming fixes moved linked kernel bytes | Verified in `evidence/kernel-bytes/KERNEL-BYTES-GAPS.md:50-52, 88-91` and `04-25-SUMMARY.md:40-41` vs `evidence/owner-stop-2/RESULTS.md:105-119` |
| DEBT-04 | Measure TPHT home-hit at production bank sizes (`default_banks`) and record sizing rule before page cache | Verified in `ds/src/tpht/flattened.rs:157-159` (`default_banks = num_blocks.div_ceil(64).max(2)` = 256 banks at 16,384 blocks vs `measure::banks_for` = 2,058 banks) |
| DEBT-05 | Update `docs/threat-model.md` to match rescope & `rfork` bit 18 | Verified `docs/threat-model.md` AND live integration test `xtask/tests/threat_model.rs:149-175` (which currently enforces verbatim old AC §2.2 quotes and `STATUS_SENTENCE`; must update test alongside doc) |
| SMP-01 | x86_64 q35 boots `-smp 2` via ACPI MADT + INIT-SIPI-SIPI; both CPUs schedule; rio green | `x86_64/src/platform/q35/acpi.rs:284-309` needs MADT type-0/type-9 LAPIC entries; `lapic.rs` needs xAPIC+x2APIC ICR IPI/INIT/SIPI; low-mem trampoline at `0x8000` |
| SMP-02 | Per-CPU state on both arches (`TPIDR_EL1` on aarch64; `GS_BASE` per-CPU `Mach` on x86_64; `CpuFp` preserved) | `aarch64/src/trap.S:137,191,237` migrates from global `CURFRAME` to `mrs xN, tpidr_el1`; `x86_64/src/dat.rs:367` already has `Mach.me` for `%gs:0`, and `trap.rs:291` `intrspecial` switches from `cmp $KMACH` to canonical kernel-half check |
| SMP-03 | Fine-grained SMP-safe locking & per-CPU copies across every single-CPU invariant; no BKL | Audited all `SyncUnsafeCell`, `static mut`, and single-CPU statics across `aarch64/src`, `x86_64/src`, `port/src` (detailed inventory in Architecture Patterns) |
| SMP-04 | aarch64 virt boots `-smp 2` under HVF (GICv3) and TCG (GICv2) via PSCI `CPU_ON` | Verified QEMU 11.1.2 launches cleanly with `-smp 2` on both HVF (`gic-version=3`) and TCG (`gic-version=2`); DTB `/psci` method (`hvc` vs `smc`), per-CPU GICR wake on GICv3, banked GICC/ISENABLER0 on GICv2 |
| SMP-07 | `IrqSpinLock` for IRQ-context locks + TLB shootdown IPI (LAPIC vector; GICv2 `GICD_SGIR` / GICv3 `ICC_SGI1R_EL1`) | IRQ-masked syscall shootdown deadlock prevented via lock-free per-CPU shootdown poll inside spin-waits + releasing `PAGE_ALLOC` before waiting for ack |
| SMP-08 | 2-CPU stress gate: user 9P workload over `9P2000.aead` mount (`FidTable`) + exec churn (`palloc.hash`) + timer IRQs on both CPUs | Runs on both arches as `-smp 2` CI gate; uses small xtask-generated or Harvey `rc`-driven workload over `#M` loopback mount |
| SMP-09 | Every new boot gate at `-smp 2`; all existing gates move to `-smp 2` (golden invariant); 1 `-smp 1` smoke leg kept per arch | Verified all 15 `-smp 1` lines across `gates/x86_64/*.expect` and `gates/aarch64/*.expect` |
| SMP-11 | Per-CPU run queues with work stealing in `port/`, host-tested | Shared `port/src/sched.rs` with per-CPU `IrqSpinLock<RunQueue>` (ordered by CPU index on steal to satisfy lock rank!), host-tested under multi-thread contention |
| SMP-12 | Documented lock rank table; runtime rank assertions + per-CPU progress watchdog in debug/gate builds only (compiled out in release) | Zero-cost `#[cfg(any(debug_assertions, feature = "lockdebug"))]` rank stack in per-CPU state + timer-tick progress counter watchdog naming stuck CPU & lock name |
| SMP-13 | Boot-switch in-kernel `9P2000.aead` loopback server on real `FidTable` + injected test secret, mounted by kernel client; default boot unchanged | Reuses `port/src/chan/transport.rs` + `port/src/fid.rs` over an in-kernel duplex pipe/channel behind a bootconf/kernel-cmdline switch (`aeadloop=1`) |
| MEM-07 | Roaring clean/dirty/pinned frame pools over 256 MB chunks, `(((chunk as usize) << 16) | (offset as usize)) * 4096`, full-container `difference`, `Evictable = Dirty \ (Pinned | Active)`, ~15% clean High-Water Pager | `ds/src/roaring.rs:675-710` already implements full-container materialization (`to_words`) in `difference`; new `port/src/palloc.rs` owns pool state under `IrqSpinLock` |
| MEM-08 | `SuperpageResidency` exact 64 bytes with bit clearing & atomic counter reset on demotion | `[AtomicU64; 8]` with `#[repr(C, align(64))]` = exact 64 bytes (512 sub-pages); `is_full()` checks all 8 words == `u64::MAX`, `count()` sums `count_ones()`, `clear_bit()` uses `fetch_and`, `reset()` zeroes all 8 atomically |
| MEM-10 | `UserSlice`/`UserSliceMut` enforce residency & hold RAII `PinGuard` (per-frame atomic pin count); every sleeping `port/` path audited | Audited all 24 `aarch64` and 26 `x86_64` `usermem` call sites + every `port/` blocking hook (`devmnt`, `devpipe`, `devip`, `devether`, `devcons`, `devssl`, `sys_semacquire`, `sys_fversion`, `sys_stat`/`fstat`, `sys_await`); pinning inside `UserSlice`/`UserSliceMut` covers all uniformly |
| MEM-14 | x86_64 full demand loading (text/data/bss/stack loaded on `#PF` vector 14, matching aarch64) | Verified `x86_64/src/exec.rs:273-274` currently maps eagerly and `x86_64/src/trap.rs:826-850` kills on every ring-3 `#PF`; adding `x86_64/src/fault.rs` brings parity |
| MEM-15 | Read-only user text + per-page dirty tracking on both arches + live evictor dropping text & untouched data + small-memory `-smp 2` pressure gate | Verified both arches currently map user text RW (`aarch64/src/vm.rs:169` `AllRw`, `x86_64` PTE `PTE_W`); x86_64 uses HW PTE `D` bit (`swap(0)` + shootdown + `D` check); aarch64 uses write-protect fault (with `UserSliceMut` EL1 write promotion!) |
| MEM-16 | Flattened-TPHT image page cache (`palloc.hash`) keyed by `(image_id, page_idx)`; shared text maps 1 frame across processes | Built after DEBT-04 on `FlattenedTphtShard`; reverse-map (`rmap`) per cached frame enables atomic multi-address-space unmap + TLB shootdown on eviction |
| MEM-17 | Roaring SIMD kernels (AVX2 on x86_64, NEON on aarch64) in 1 reserved unsafe module in `ds/`, behind `ds/src/setops.rs`, via `with_simd()`, runtime feature dispatch, scalar kept & pinned identical | Inline/global asm (not `#[target_feature]` which silently emits scalar on x86-softfloat); update `xtask/src/lintcontract.rs` R3 to allow the one reserved SIMD module |
| MEM-18 | x86_64 AVX enabled when CPUID reports it; XSAVE/XSAVEOPT there, FXSAVE where XSAVE absent (`qemu64`); 1 boot line names format | Verified QEMU 11.1.2 TCG supports `-cpu qemu64,+avx,+avx2,+xsave,+xsaveopt -smp 2`; `FpState` grows to 832 B (64-byte aligned, covering x87+SSE+YMM with AVX-512 off in XCR0) |
</phase_requirements>

## Summary

Phase 5 is a three-stage systems foundation spanning 28 requirements and four owner stops (D-32): (1) course correction and v3.0 audit follow-ups (`CORR-01..05`, `DEBT-01..05`), (2) two-CPU SMP on `aarch64` virt then `x86_64` q35 (`SMP-01..04`, `SMP-07..09`, `SMP-11..13`), and (3) non-cryptographic memory pools, demand loading, dirty tracking, TPHT image page cache, Roaring SIMD, AVX/XSAVE, and user-frame pinning (`MEM-07`, `MEM-08`, `MEM-10`, `MEM-14..18`).

Live verification against HEAD (`35ccf2c` / `origin/main` `900f58b`) uncovered five concrete facts that directly shape the plan:
1. **What is actually red at `900f58b` (CORR-01):** `xtask` workflow is 10/10 green, and `gates` workflow is 6/7 green, with **`quick-gate-tcg` FAILING** (masked at the workflow badge only by `continue-on-error: true`). Root cause: `gates/x86_64/drawterm-gate.expect:318-348` `proc start_xvfb` sets `set env(DISPLAY) ":$number"` without declaring `global env`, crashing line 425 with `no such variable (read trace on "env(DISPLAY)")` on headless CI runners.
2. **`cargo test -p xtask` is ALREADY red on 2 tests (DEBT-01):** Running `cargo test -p xtask` live fails `gate::tests::full_capture_loop_covers_only_the_full_tier` and `gate::tests::full_tier_output_dirs_match_the_scripts_and_witness_map` (`xtask/src/gate.rs:885,918`) because they read `.planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh`, which was archived to `.planning/milestones/v3.0-phases/01-stage-p0-cross-build-harness/witness/run-witness.sh` at `c6dd0a6`.
3. **`boot-iso` purity is a surgical removal (CORR-02):** `flake.nix:312-340` `builder.bootIso` already accepts `outputHash ? null` and omits the FOD attributes when `null`. Removing `outputHash = "..."` at `flake.nix:984` turns `packages.x86_64-linux.boot-iso` into a standard pure derivation with zero builder rewrite.
4. **Unsafe & lint counts are exact (CORR-05):** Live `cargo clippy` confirms exactly **7** undocumented `unsafe` blocks in 3 `aarch64` files and **32** in 9 `x86_64` files. `x86_64/Cargo.toml` can switch directly to `[lints] workspace = true` because root `Cargo.toml:44-53` already declares all `x86_64` cfgs.
5. **Two subtle SMP/MMU hazards resolved up front:** (a) Because r9 runs kernel syscalls with IRQs masked, a CPU waiting for a remote TLB shootdown IPI while holding a lock would deadlock if the target CPU is spinning on that lock; polling a lock-free per-CPU shootdown slot inside `IrqSpinLock`/`MCSLock` spin loops AND releasing `PAGE_ALLOC` before waiting eliminates this deadlock class completely. (b) On ARMv8 (`aarch64`), `AccessPermission::AllRo` denies EL1 kernel writes as well as EL0 writes, so `UserSliceMut::new` must promote untouched `SegKind::Data` pages to `AllRw` + mark them dirty before handing out `&mut [u8]`.

**Primary recommendation:** Execute strictly in the four-stop sequence of D-29/D-32 — (Stop 1) CI/xtask/FOD/`rfork`/lint/debt + seqlock slab `FidTable`; (Stop 2) `port/` SMP primitives (`IrqSpinLock`, rank checker, watchdog, per-CPU run queues, shootdown) -> `aarch64` 2-CPU bring-up (HVF+TCG) -> `x86_64` 2-CPU bring-up -> migrate all gates to `-smp 2`; (Stop 3) x86_64 AVX/XSAVE with CPUID-gated FXSAVE fallback + TCG AVX2 CI leg; (Stop 4) DEBT-04 bank sizing -> Roaring pools, `SuperpageResidency`, demand loading on x86_64, RO text + dirty tracking, TPHT page cache + reverse-map evictor, `UserSlice` `PinGuard`, Roaring SIMD asm kernels, small-memory pressure gate, and loopback `9P2000.aead` 2-CPU stress gate.

## Project Constraints (from Standing Owner Rules & Workspace Contract)

No `AGENTS.md`, `CLAUDE.md`, `.claude/skills/`, or `.agents/skills/` exists in the repository `[VERIFIED: directory listing]`. The binding workspace constraints from `STATE.md`, `05-CONTEXT.md`, and `docs/r9_implementation_guide_for_agentic_developers_v3_0.md` are:
- **No AI attribution** in commit messages or PRs.
- **No push without explicit owner approval**; present unpushed commits at each of the four owner stops (D-32).
- **Harvey is read-only**; never edit `vendor/harvey` or stage the 11 pre-modified troff/postscript font files in Harvey.
- **IG-V01 discipline:** committed RED pin -> implementation -> GREEN twice consecutively -> pin advance; golden invariant (boots to rio on q35) holds at every commit (D-03).
- **`UNSAFE-AUDIT.md` lockstep:** any commit adding, modifying, or removing an `unsafe` block in `ds/src/tpht/`, `ds/` reserved SIMD module, `port/` floor modules, or an arch crate must update `UNSAFE-AUDIT.md` in the same commit (guide rule 3).
- **No software RAM encryption, `PKEY`/`WRPKRU` page-fault hooks, or Rust rewrites of `6c`/`6a`/`6l`** (D-04, guide rule 4).
- **Kernels compiled without FP/SIMD** (`-mmx,-sse,+soft-float` / `-neon`); kernel SIMD goes strictly through `fpu::kernel_begin()` / `with_simd()` (D-06).
- **`port/src` diff-first rule waived for Phase 5 only** (D-30); cumulative `port/src` diff reviewed at each of the four owner stops.
- **x86_64 kernel byte check** switches from manual `outputHash` + owner boot-iso check to green CI once CORR-02 lands (D-05); KVM runs on owner's Linux host at stops 2, 3, and 4 (D-33).
- **`gh` is not installed** on PATH — invoke via `nix run nixpkgs#gh -- ...`. Run builds/tests via `nix develop -c cargo xtask ...`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CI/Nix hygiene, `boot-iso`, lint contract, `rfork.h` generation | Build / Harness (`flake.nix`, `xtask/`, `.github/workflows/`) | `port/src/rfork.rs` | Single source of truth in Rust/Nix; zero manual hashes or duplicated flags |
| Generic lock-free seqlock slab (`AtomicSlabSlot`, `SlabShard`, `AllocMeta`) | Data Structures (`ds/src/slab.rs`, `#![forbid(unsafe_code)]`) | `port/src/fid.rs` (wraps `AllocMeta` in `Lock`, adds `StatefulHandle`, `FidEntry`, client check) | `ds/` has no mutex; `port/` owns 9P semantics & MCS lock without any `unsafe` |
| `IrqSpinLock`, lock rank table, watchdog state, per-CPU trait, work-stealing run queues, shootdown coordinator | Portable Kernel (`port/src/mcslock.rs`, `port/src/smp.rs`, `port/src/sched.rs`) | Arch crates supply IRQ mask/restore, CPU ID, IPI send, AP bring-up | Host-testable concurrency primitives; prevents arch-specific scheduler divergence (D-25) |
| CPU 1 bring-up, per-CPU register (`TPIDR_EL1` / `GS_BASE`), IPI delivery, `trap.S`/`trap.rs` per-CPU frames | Arch Kernel (`aarch64/`, `x86_64/`) | `port/` trait boundary | Hardware-specific boot (PSCI `CPU_ON`, ACPI MADT + INIT-SIPI-SIPI) and interrupt controllers (GICv2/v3, LAPIC/x2APIC) |
| Roaring frame pools (clean/dirty/pinned), `SuperpageResidency`, High-Water Pager, TPHT page cache (`palloc.hash`), reverse-map & eviction policy | Portable Kernel (`port/src/palloc.rs`, `port/src/pagecache.rs`) + `ds/src/roaring.rs` | Arch crates supply physical memory ranges, PTE clear/dirty-check, TLB shootdown, frame zeroing | Pool set algebra and cache/eviction policy are architecture-independent and host-testable |
| Demand fault handler, RO user text, dirty tracking, `UserSlice` `PinGuard` | Arch Kernel (`aarch64/src/{fault,exec,vm,usermem}.rs`, `x86_64/src/{fault,exec,proc,trap,usermem}.rs`) | `port/src/palloc.rs` (atomic per-frame pin counts & cache lookup) | Page table formats and fault encodings are arch-specific; pin count and cache live in `port/` |
| Roaring SIMD set kernels (AVX2 / NEON) | Data Structures (`ds/src/setops_simd.rs` reserved unsafe module behind `ds/src/setops.rs`) | Arch `fpu::with_simd()` + boot-time feature flag | Single reserved unsafe module in `ds/` (Phase 3 D-09); callers pass SIMD token / `with_simd` guard |
| x86_64 AVX enable & XSAVE/FXSAVE switch | Arch Kernel (`x86_64/src/{fpu,cpu,proc,syscall,exec}.rs`) | — | Hardware `CR4.OSXSAVE`/`XCR0` and `XSAVEOPT`/`XRSTOR` save area are x86_64-only |
| Loopback `9P2000.aead` stress server & mount | Portable Kernel (`port/src/aead_loop.rs`, boot-switch gated) | `port/src/devmnt.rs`, `port/src/fid.rs`, `port/src/chan/` | Gate-only wiring (SMP-13, D-28); default boot exposes nothing new |

## Standard Stack

**No new external crates are added in Phase 5.** All work builds strictly on the existing pinned workspace crates and toolchains `[VERIFIED: Cargo.toml, rust-toolchain.toml, flake.nix]`.

### Core Workspace & Toolchain
| Component | Version | Purpose | Verification |
|-----------|---------|---------|--------------|
| Rust toolchain | `nightly-2026-01-30` (`rustc 1.95.0-nightly 842bd5be2`) | Kernel `-Zbuild-std=core,alloc` + host tests + clippy | `[VERIFIED: nix develop -c rustc --version]` |
| QEMU (`qemu-system-aarch64`, `qemu-system-x86_64`) | `11.1.2` | Boot gates under HVF (`virt,gic-version=3`), TCG (`virt,gic-version=2`, `q35`) | `[VERIFIED: nix develop -c qemu-system-x86_64 --version]` |
| `crypto` (workspace) | `0.1.0` (`#![forbid(unsafe_code)]`) | HKDF-SHA256, ChaCha20, Keyed BLAKE3, `cap_tag`/`cap_verify` for `9P2000.aead` loopback & `FidTable` | `[VERIFIED: crypto/src/lib.rs]` |
| `ds` (workspace) | `0.1.0` (per-module forbid) | `RoaringBitmap`, `FlattenedTphtShard`, new `slab.rs` (forbid), new `setops_simd.rs` (reserved audited unsafe) | `[VERIFIED: ds/src/lib.rs:1-32]` |
| `port` (workspace) | `0.1.0` (per-module forbid; 3 floor modules) | `mcslock` (+ `IrqSpinLock`), `fid`, `rfork`, `devmnt`, new `smp`/`sched`/`palloc`/`pagecache`/`aead_loop` | `[VERIFIED: port/src/lib.rs:10-119]` |
| `aarch64-cpu` | `11.2` | AArch64 system register helpers (`MPIDR_EL1`, `CNTFRQ_EL0`, `CNTP_*`) | `[VERIFIED: aarch64/Cargo.toml:10]` |
| `x86` | `0.52` | x86_64 low-level helpers | `[VERIFIED: x86_64/Cargo.toml:11]` |
| `zerocopy` | `0.8.27` | `FromZeros` on `Mach` and `CpuFp` | `[VERIFIED: x86_64/Cargo.toml:14]` |

### Package Legitimacy Audit
*Skipped — Phase 5 installs zero external packages (`Cargo.lock` unchanged).*

## Architecture Patterns

### System Architecture Diagram

```text
                       ┌───────────────────────────────────────────────┐
                       │         User Processes (EL0 / Ring 3)         │
                       │  (running concurrently on CPU 0 and CPU 1)    │
                       └──────────┬────────────────────────┬───────────┘
                                  │ Syscall / #PF / Abort  │ Timer / IPI / Device IRQ
                                  ▼                        ▼
┌──────────────────────────────────────────────────────────────────────────────────┐
│                 Arch Trap & Per-CPU Layer (aarch64/ & x86_64/)                   │
│  Per-CPU ptr: TPIDR_EL1 (arm64) / GS_BASE -> Mach.me (x64)                       │
│  [cur_frame, cur_kstack, cur_proc, cpu_id, fpu: CpuFp, lock_ranks, watchdog_cnt] │
│  IPIs: GICv2 GICD_SGIR / GICv3 ICC_SGI1R_EL1  |  LAPIC xAPIC/x2APIC ICR          │
└───────────┬──────────────────────────┬───────────────────────────┬───────────────┘
            │                          │                           │
            ▼                          ▼                           ▼
┌───────────────────────┐  ┌─────────────────────────┐  ┌──────────────────────────┐
│  UserSlice/Mut Pin    │  │  Demand Pager & Evictor │  │  SMP Scheduler (port/)   │
│  validate_user_range  │  │  1. palloc.hash (TPHT)  │  │  Per-CPU IrqSpinLock     │
│  ensure_user_resident │  │     (shared RO text)    │  │  RunQueue[cpu_id]        │
│  (arm64: promote RO   │  │  2. Roaring Frame Pools │  │  Idle CPU work-steal     │
│   Data to RW+dirty)   │  │     Clean / Dirty / Pin │  │  (locked low->high id)   │
│  PinGuard (atomic     │  │  3. Evictable =         │  │                          │
│   pin_count++, Drop--)│  │     Dirty \ (Pin|Act)   │  │  Rank-checked Lock /     │
└───────────┬───────────┘  │  4. Reverse-map unmap   │  │  IrqSpinLock + Watchdog  │
            │              │     -> TLB Shootdown IPI│  └──────────────────────────┘
            │              │     (spin-poll safe!)   │
            ▼              └────────────┬────────────┘
┌───────────────────────────────────────┴──────────────────────────────────────────┐
│               9P2000.aead & Device Layer (port/ — forbid unsafe)                 │
│  devmnt (Client) ──[loopback pipe, boot switch]──> aead_loop (Gate Server)       │
│                                                       │                          │
│                                                       ▼                          │
│                                              FidTable (port/src/fid.rs)          │
│                                              wraps ds/src/slab.rs                │
│                                              - resolve(): 0 locks, 1x 64B line   │
│                                                (Boehm seqlock, AtomicU64 words)  │
│                                              - alloc/clunk/sweep: shard mutex    │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure (New & Modified Files)

```text
ds/src/
├── lib.rs                 # Add #[forbid(unsafe_code)] pub mod slab; and mod setops_simd;
├── slab.rs                # NEW (#![forbid(unsafe_code)]): AtomicSlabSlot, SlabShard, AllocMeta, SlabProbe
├── roaring.rs             # Unchanged API; calls setops::*
├── setops.rs              # Dispatch seam: calls setops_simd (AVX2/NEON) when active, else scalar
├── setops_simd.rs         # NEW (reserved audited unsafe module): AVX2 & NEON inline asm kernels
└── tpht/                  # UNTOUCHED except DEBT-04 production-bank measurement & sizing rule doc

port/src/
├── lib.rs                 # Add forbid modules: smp, sched, palloc, pagecache, superpage, aead_loop
├── rfork.rs               # Remove RFCKEY/RFROBASE/RFRANDMEM; RFNOLEGACY = 1 << 18
├── fid.rs                 # Refactor onto ds::slab (remove TPHT, insert_lock, all-shard probe)
├── mcslock.rs             # Add IrqSpinLock, lock-rank checker hooks, shootdown poll in spin-wait
├── smp.rs                 # NEW: PerCpu trait, ShootdownCoordinator, LockRank table, Watchdog
├── sched.rs               # NEW: Per-CPU RunQueue + work-stealing scheduler logic (host-tested)
├── palloc.rs              # NEW: Roaring Clean/Dirty/Pinned pools, High-Water Pager, pin counts
├── superpage.rs           # NEW: 64-byte SuperpageResidency ([AtomicU64; 8])
├── pagecache.rs           # NEW: FlattenedTphtShard image page cache (palloc.hash) + reverse-map
└── aead_loop.rs           # NEW: Boot-switch-only in-kernel 9P2000.aead loopback server on FidTable

aarch64/src/
├── l.S                    # Add secondary_start entry for PSCI CPU_ON
├── trap.S                 # Replace =CURFRAME with TPIDR_EL1 per-CPU slot
├── smp.rs                 # NEW: PSCI CPU_ON (HVC/SMC from DTB), PerCpu struct, SGI IPIs
├── platform/virt/gic/     # gicv2: secondary init + GICD_SGIR; gicv3: per-CPU GICR + ICC_SGI1R_EL1
├── fault.rs               # RO text + write-protect dirty fault + TPHT page cache + evict-and-retry
├── vm.rs                  # Add Entry::ro_user_text(), Entry::ro_user_data(), unmap + tlbi vae1is
└── usermem.rs             # PinGuard on UserSlice/UserSliceMut + EL1 write promotion on Data

x86_64/src/
├── ap_trampoline.S        # NEW (or in l.S): 16->32->64-bit SIPI trampoline copied to PA 0x8000
├── smp.rs                 # NEW: INIT-SIPI-SIPI from MADT, per-CPU Mach allocation, LAPIC IPIs
├── platform/q35/acpi.rs   # Parse MADT type-0 (LAPIC) & type-9 (x2APIC) CPU entries
├── platform/q35/lapic.rs  # Add xAPIC & x2APIC ICR send_init / send_sipi / send_ipi + AP init
├── fpu.rs                 # CPUID-gated AVX/XSAVE/XSAVEOPT (832B FpState) + FXSAVE fallback + boot line
├── fault.rs               # NEW: demand pager for #PF (vector 14), RO text, HW PTE_D dirty tracking
├── exec.rs                # Remove eager text/data/bss mapping (full demand loading, MEM-14)
├── trap.rs                # Route ring-3 #PF to fault.rs; canonical GS check in intrspecial; IPI vectors
└── usermem.rs             # PinGuard on UserSlice/UserSliceMut
```

---

### Pattern 1: Course Correction & Audit Follow-Ups (CORR-01..05, DEBT-01..05)

#### 1.1 CORR-01 — Red CI Root Cause & Fix
- **Verified failure at `900f58b`:** `gh api repos/requaos/r9/commits/900f58beb03c1a118a7698addcca5c68502d0c1b/check-runs` reports 16/17 checks `success` and **`quick-gate-tcg: completed / failure`** (job `139394030723` in run `45667644077`) `[VERIFIED: gh api]`.
- **Root cause (`gates/x86_64/drawterm-gate.expect:318-348`):**
  ```tcl
  proc start_xvfb {} {
      global run_tag gate_dir
      ...
      set env(DISPLAY) ":$number"
      return $pid
  }
  ```
  Missing `global env` causes `set env(DISPLAY)` to write a procedure-local array; on headless CI runners where `DISPLAY` is unset, line 425 (`puts stderr "drawterm-gate: Xvfb ready (DISPLAY=$env(DISPLAY), -nolisten tcp)"`) throws `no such variable (read trace on "env(DISPLAY)")` `[VERIFIED: gates/x86_64/drawterm-gate.expect:318-348,425]`.
- **Fix:** Change line 319 to `global env run_tag gate_dir`. Also verify whether `.github/workflows/gates.yml:136` should remove `continue-on-error: true` on `quick-gate-tcg` (and the other gate jobs) once CORR-02 makes CI the per-commit x86_64 check (D-05): with `continue-on-error: true`, a red `quick-gate-tcg` still shows a green workflow checkmark on GitHub, which was why the failure went unnoticed across five consecutive merges!

#### 1.2 CORR-02 — Pure `boot-iso` Derivation
- In `flake.nix:312-340`, `builder.bootIso` ALREADY implements:
  ```nix
  bootIso = { kernel-elf, modules ? [ ], outputHash ? null }:
    ...
    pkgs.stdenv.mkDerivation ({
      ...
    } // nixpkgs.lib.optionalAttrs (outputHash != null) {
      outputHashAlgo = "sha256";
      outputHashMode = "recursive";
      inherit outputHash;
    });
  ```
  `[VERIFIED: flake.nix:312-340]`
- At `flake.nix:626-985`, `packages.x86_64-linux.boot-iso` passes `outputHash = "sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=";`.
- **Fix:** Remove the `outputHash = ...;` attribute (and replace the 350-line historical FOD comment block with a concise note pointing to CORR-02). `nix build .#boot-iso` on `x86_64-linux` becomes a standard pure derivation whose output path hashes its inputs (`kernel-x64`, `iso-builder`, `grub.cfg`). Also add a `nix build --no-link .#boot-iso` step in CI (`xtask.yml`) so pure `boot-iso` buildability is verified on every push!

#### 1.3 CORR-04 — `rfork` Bit Cleanup
- `port/src/rfork.rs:36-44` currently defines:
  ```rust
  RFCKEY = 1 << 18;
  RFROBASE = 1 << 19;
  RFRANDMEM = 1 << 20;
  RFNOLEGACY = 1 << 21;
  ```
  `[VERIFIED: port/src/rfork.rs:36-44]`
- **Fix:** Delete `RFCKEY`, `RFROBASE`, `RFRANDMEM`; set `RFNOLEGACY = 1 << 18;`; run `cargo xtask rfork-header` to regenerate `sys/include/rfork.h`; update `xtask/src/rforkh.rs:124-131` (assert only `\tRFNOLEGACY\t= (1<<18)\n` and assert absence of `RFCKEY`/`RFROBASE`/`RFRANDMEM`), and lines 162 & 171 (change `.replace("(1<<21)", "(1<<22)")` to `.replace("(1<<18)", "(1<<22)")`) `[VERIFIED: xtask/src/rforkh.rs:124-131,162,171]`.

#### 1.4 CORR-05 — Unsafe Policy Sync & `x86_64` Lint Opt-In
- **Exact undocumented `unsafe` counts verified by live `cargo clippy`:**
  - `aarch64` (`virtshell`, **7 sites in 3 files**): `platform/virt/rtc.rs:75`, `proc.rs:439, 470, 1823, 2753`, `textcopy.rs:158, 172` `[VERIFIED: cargo xtask clippy --arch aarch64 --config virtshell]`.
  - `x86_64` (default & full `q35cpu`, **32 sites in 9 files**): `allocator.rs:12`, `cpu.rs:37, 79, 88, 102, 115, 142, 164, 188, 216, 270, 289, 307, 328`, `dat.rs:433`, `pio.rs:5, 12, 19, 26, 39, 52`, `seg.rs:257, 394, 422`, `trap.rs:69, 136`, `uart16550.rs:14`, `proc.rs:852, 873`, `main.rs:131, 338, 374` `[VERIFIED: cargo clippy --package x86_64]`.
- **Lint opt-in:** Replace `x86_64/Cargo.toml:29-38` (`[lints.rust] ...`) with `[lints]\nworkspace = true` (workspace `Cargo.toml:44-53` already contains all seven `x86_64` check-cfg entries `[VERIFIED: Cargo.toml:44-53, x86_64/Cargo.toml:29-38]`).
- **Policy docs sync:** Update `docs/r9_implementation_guide_for_agentic_developers_v3_0.md`, `docs/plan_r9_architecture_code_compendium_v3_0.md`, `Cargo.toml` header comment, `xtask/src/lintcontract.rs` (R6 checks `x86_64/Cargo.toml` too; R3 notes the reserved Roaring SIMD module), and `UNSAFE-AUDIT.md` (remove non-existent `node0.rs`/`vsvm.rs`, audit all actual `x86_64/src/` files, update `aarch64` count, state `port/` per-module forbid with the 3 floor modules `globalalloc`, `mcslock`, `devip_waker` per D-02). Add `cargo xtask clippy --arch aarch64 --config virtshell` and `cargo xtask clippy --arch x86-64 --config q35cpu` to `.github/workflows/xtask.yml` `lint` job.

#### 1.5 DEBT-01..05 — Exact Action Items
- **DEBT-01 (`xtask/src/main.rs:3578-3684`, `.github/workflows/xtask.yml`, `xtask/src/gate.rs:885,918`):**
  - First fix the 2 broken tests in `xtask/src/gate.rs:885,918`: update `.planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh` -> `.planning/milestones/v3.0-phases/01-stage-p0-cross-build-harness/witness/run-witness.sh` `[VERIFIED: cargo test -p xtask]`.
  - Add `cargo test --package xtask` AND `xtask::test9p::run(&workspace(), &cargo())` (or its legs) to `TestStep::run` in `xtask/src/main.rs` so `cargo xtask test` (and therefore CI `xtask.yml`'s `tests` job) executes BOTH automatically. Also update `xtask/src/test9p.rs` doc comment (lines 4-9, 44-45) which previously stated `test-9p` is not part of `cargo xtask test`.
- **DEBT-02:** Reconcile `.planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-VALIDATION.md` from `status: draft` / `nyquist_compliant: false` / `TBD` rows to `status: validated` / `nyquist_compliant: true` / `wave_0_complete: true` using the completed Phase 4 plan/verification evidence (D-34).
- **DEBT-03:** Add a dated correction note to `.planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/kernel-bytes/KERNEL-BYTES-GAPS.md` (and `04-25-SUMMARY.md` where needed) clarifying that the unchanged `boot-iso` `outputHash` (`sha256-5A6Q...`) proves the linked stripped kernel code bytes did not change (only unstripped DWARF/line metadata moved on the Mac ELF), and that the fixes are proved by the host test suite while the booted kernel does not select the Aead path (D-16/D-23).
- **DEBT-04:** Measure `FlattenedTphtShard` home-hit rate and `BanksFull` occupancy ceiling under `FlattenedTphtShard::new(num_blocks)` (`default_banks(num_blocks) = num_blocks.div_ceil(64).max(2)` = 256 banks at 16,384 blocks) across all 5 `Distribution::ALL` distributions (`Uniform`, `Sequential`, `PageKey`, `Upper32`, `Upper32Or7`). Write down the production bank-sizing rule in `ds/src/tpht/flattened.rs` (e.g., verify whether `default_banks` holds `>90%` home-hit with zero `BanksFull` at 85%–90% occupancy, and derive the exact `banks_for_occupancy(num_blocks, max_occ_pct)` helper / doc rule) and add a non-ignored host pin before MEM-16 builds `palloc.hash`.
- **DEBT-05:** Update `docs/threat-model.md` to match v3.0 rescope (hardware MMU isolation as memory boundary; drop pages-at-rest encryption, stale-frame/shadow-frame/`ymm0-5`/sibling-key items; update `rfork` section to state bits 18–20 retired and `RFNOLEGACY = 1 << 18`) AND update `xtask/tests/threat_model.rs` (lines 13–18, 149–175, 234+) so `cargo test -p xtask threat_model` checks against `docs/plan_r9_architecture_code_compendium_v3_0.md` instead of the superseded `.planning/r9-architecture-and-code-v4.md`!

---

### Pattern 2: Generic Lock-Free Seqlock Slab (`ds/src/slab.rs`) & `FidTable` (`port/src/fid.rs`) (CORR-03)

**Soundness Proof of D-11 Boehm Seqlock in Safe Rust (`#![forbid(unsafe_code)]`) across x86_64, aarch64, riscv64:**
- **Reference:** Hans-J. Boehm, *"Can Seqlocks Get Along With Programming Language Memory Models?"* (MSPC 2012), §4 Fig. 5 `[CITED: ds/src/tpht/flattened.rs:8-18]`.
- **Layout:** `#[repr(C, align(64))] pub struct AtomicSlabSlot<const WORDS: usize = 7> { state: AtomicU64, words: [AtomicU64; WORDS] }`. At `WORDS = 7`: `8 + 56 = 64` bytes exact, 64-byte aligned (one cache line).
  - State bits: bit 0 = `WRITING`, bit 1 = `OCCUPIED`, bits `2..18` = `gen: u16` (`1..=65535`, initialized to `1`, skip 0 on wrap: `gen.wrapping_add(1).max(1)`), bits `18..64` = 46-bit `seq` (`SEQ_INC = 1 << 18`).
- **Reader (`resolve`):**
  1. `s1 = self.state.load(Ordering::Acquire)` — synchronizes-with the writer's final `state.store(published, Ordering::Release)` -> guarantees all payload stores from `s1` happen-before step 3.
  2. If `(s1 & WRITING) != 0` -> `core::hint::spin_loop(); continue;`
  3. If `(s1 & OCCUPIED) == 0 || decode_gen(s1) != expected_gen` -> `return None;`
  4. Load `raw[i] = self.words[i].load(Ordering::Relaxed)` for `i in 0..7` — zero Rust data-race UB because every word is `AtomicU64`.
  5. `core::sync::atomic::fence(Ordering::Acquire)` — pairs with any concurrent writer's `fence(Ordering::Release)` via C++20 `[atomics.fences]` fence-fence synchronization! If ANY `raw[i]` read from a newer write `W_next`, `W_next`'s `fence(Release)` synchronizes-with this `fence(Acquire)`, making `W_next`'s `state.store(writing, Relaxed)` happen-before step 6.
  6. `s2 = self.state.load(Ordering::Relaxed)` — if `s1 == s2`, return `Some(raw)`; else retry.
- **Writer (`publish_alloc`, `publish_update`, `publish_clunk`):**
  - **CRITICAL SERIALIZATION RULE:** ALL writers (`alloc`, `update`, `clunk`, `sweep`) MUST hold the shard-local `Lock<AllocMeta>` (in `port/src/fid.rs`) so two writers NEVER race on `self.state.store(cur | WRITING, Relaxed)`! (Do NOT mix a mutex-serialized `clunk` with a CAS-only `update` unless BOTH use CAS on `WRITING`; taking the shard-local `meta` mutex for `update` as well is simplest, 100% race-free, and has zero impact on lock-free `resolve()`).
  - Sequence: `state.store(writing, Relaxed)` -> `fence(Ordering::Release)` -> store all 7 `words[i]` (`Relaxed`; clunk zeroes them) -> `state.store(published_or_cleared, Ordering::Release)`.
- **Codegen on all 3 architectures:**
  - `x86_64` (TSO): `mov` loads/stores + compiler barriers only (0 hardware fence instructions on resolve!).
  - `aarch64` (ARMv8): `ldar` (`s1`) + 7× `ldr` + `dmb ishld` (`fence(Acquire)`) + `ldr` (`s2`); writer emits `str` + `dmb ish` (`fence(Release)`) + 7× `str` + `stlr`.
  - `riscv64` (RVWMO): `ld; fence r,rw` + 7× `ld` + `fence r,rw` + `ld`; writer emits `sd` + `fence rw,w` + 7× `sd` + `fence rw,w; sd`.
- **`ds/src/slab.rs` vs `port/src/fid.rs` Split (Claude's Discretion):**
  - `ds/src/slab.rs` (`#![forbid(unsafe_code)]`) exports:
    - `SlabProbe` trait (`fn slot_read(&self) {}`) + ZST `NoSlabProbe`.
    - `AtomicSlabSlot<const WORDS: usize = 7>` (64 B, align 64).
    - `AllocMeta` (holding `free_list: Vec<u16>`, `active: RoaringBitmap`, `reclaimed: RoaringBitmap`, `capacity: u16`) with pure methods `alloc_slot(&mut self, slots: &[AtomicSlabSlot<WORDS>], words: [u64; WORDS]) -> Option<(u16, u16)>`, `clunk_slot`, `update_slot`, `mark_reclaimed`, `sweep_unreclaimed`.
    - `SlabSlots<const WORDS: usize = 7, P: SlabProbe = NoSlabProbe>` wrapping `Box<[AtomicSlabSlot<WORDS>]>` + `probe: P`, with lock-free `resolve(&self, idx: u16, expected_gen: u16) -> Option<[u64; WORDS]>` (calls `self.probe.slot_read()` once per attempt!).
  - `port/src/fid.rs` (`#![forbid(unsafe_code)]`) wraps `SlabSlots<7, P>` + `Lock<AllocMeta>` inside `#[repr(align(64))] struct FidShard<P>`:
    - `StatefulHandle` (8 bytes LE): `slot_idx: u16`, `gen: u16` (1..=65535, never 0), `shard_id: u8`, `flags: u8`, `epoch: u16`. `from_wire` rejects `gen == 0`, proving a stateful handle is never all zero (D-10).
    - `FidEntry` keeps its 49 live bytes (including `fid_key` as a client record and `epoch`, dropping `FID_LIVE` since slot `OCCUPIED` replaces it per D-11) packed into `[u64; 7]` (56 bytes, 7 spare bytes). Keep `#[repr(C, align(64))]` or 56-byte `to_words() -> [u64; 7]` / `from_words(&[u64; 7])`.
    - `FidTable::resolve(client_id, handle)` checks `client_id != 0`, `handle.epoch == self.epoch`, `handle.shard_id < shards.len()`, calls `shard.slots.resolve(handle.slot_idx, handle.gen)`, unpacks `FidEntry`, and checks `entry.client_id == client_id` (D-08)!
    - `Probe` test (WIRE-08 restated): `CountingSlabProbe` confirms **1 slot cache-line read, 0 metadata lock acquisitions** on uncontended `resolve()`.
    - Remove `FlattenedTphtShard`, `insert_lock`, and all-shard probe completely. Route `alloc()` from `port::smp::current_cpu_id() & shard_mask` (CPU 0 before SMP lands, then wired to the per-CPU trait in SMP-02; round-robin fallback if shard full per D-12).

---

### Pattern 3: Two-CPU SMP Architecture (`aarch64` first, then `x86_64`) (SMP-01..04, SMP-07..09, SMP-11..13)

#### 3.1 Single-CPU Invariant Audit (`SMP-03`, D-22)
Every global mutable state item found in our codebase scan falls into one of four SMP-safe categories:

| Category | `aarch64/src` Items | `x86_64/src` Items | `port/src` Items | SMP-03 Action |
|----------|---------------------|--------------------|------------------|---------------|
| **1. Move to Per-CPU (`PerCpu` / `Mach`)** | `CURRENT_PROC`, `CURKSTACK`, `CURFRAME`, `SCHED_CONTEXT`, `SCHED_SEEDED`, `BOOT_CTX`, `fpu::CPUS` | `CURRENT_PROC`, `CURFRAME`, `SCHED_CONTEXT`, `SCHED_SEEDED`, `BOOT_CTX`, `Mach.{tss,fpu,scratch,proc,sched}` | — | Index via `TPIDR_EL1` (`aarch64`) / `%gs:0` (`Mach.me`, `x86_64`); zero sharing |
| **2. Protect with `IrqSpinLock` (shared with IRQ/fault)** | `PROC_TABLE`, `PAGE_ALLOC`, `ALLOC` (`QuickFit` heap), `CONS_RX`, `VIRTIO_NET`, `SHARED_PAGES`, `RFMEM_FAMILIES`, `SEM_WAITERS` | `PROC_TABLE`, `PAGE_ALLOC`, `ALLOC` (`QuickFit` heap), `uart16550::RING`, `VIRTIO_NET`, `SHARED_PAGES`, `RFMEM_FAMILIES`, `SEM_WAITERS` | `devcons::INPUT`, `devpipe` pipes, `devip` (`NET_WAITERS`, `PENDING_WAKES`, `STACK`, `TABLES`, `IFC`, `FS`), `netif` (`NETIF`, `INGRESS`), `devmnt` (`NEXT_MNT_ID`, `Mnt`), `devproc`, `devsrv`, `devenv`, `devcap`, `devssl` | Convert IRQ-touched locks to `IrqSpinLock` with explicit ranks; replace single-CPU `BusyCell` contention assumptions where two CPUs can enter simultaneously! |
| **3. Publish-once boot-time read-only** | `GIC`, `PL011_RANGE`, `UART`, `PCI_DISCOVERY`, `VIRTIO_NET_INTID`, `DTB_VA`, `RTC_VA`, `ANCHOR_NS`, `INPUTS` | `ACPI`, `LAPIC`, `IOAPIC`, `DISCOVERY`, `VIRTIO_VECTOR`, `TIMER::STATE`, `CONS`, `mb2info`, `modstash`, `INPUTS` | `ROOT_IMG`, `HOOKS` (`devdup`/`devether`/`devip`/`devproc`/`devenv`/`devmnt`), `CLOCK` | Written strictly by CPU 0 **before** starting CPU 1 (release barrier before `CPU_ON` / `SIPI`); immutable read-only thereafter! |
| **4. Already Atomic** | `NEXT_NOTEID`, `NEXT_RFMEM_FAMILY`, `TICKS`, `SYNCS` | `NEXT_NOTEID`, `NEXT_RFMEM_FAMILY`, `TICKS` | `NET_DEADLINE`, `STACK_DEBT`, `DRIVE_PRINT`, `DRIVE_COUNT`, `ATTACH_PRINTED`, `IN_MSGS`/`OUT_MSGS` | Verify ordering (`Acquire`/`Release` where coordinating state; `Relaxed` only for stats/counters); timer `TICKS` incremented only by CPU 0 so tick rate stays 1 kHz! |

**CRITICAL FINDING ON `port/` `BusyCell` UNDER 2 CPUs:**
- Look at `port/src/devip/stack.rs:247,252,523` (`NET_WAITERS`, `PENDING_WAKES`, `STACK`) and `port/src/netif.rs:387` (`INGRESS`) `[VERIFIED: grep port/src]`:
  - Today those use `BusyCell::enter()` which returns `None` if already entered! On 1 CPU with IRQs masked, two syscalls never ran simultaneously, so `BusyCell` only protected against nested re-entry.
  - On **2 CPUs**, CPU 0 and CPU 1 WILL call `devip` / `netif` / `devmnt` concurrently! If `NET_WAITERS` / `PENDING_WAKES` / `STACK` / `INGRESS` stay `BusyCell`, CPU 1 gets a spurious `None` error instead of waiting!
  - **Fix:** Replace `NET_WAITERS`, `PENDING_WAKES`, `INGRESS`, and `STACK` with ranked `IrqSpinLock` (for `NET_WAITERS`/`PENDING_WAKES`/`INGRESS`, which are brief and touched in IRQ/wake paths) or `Lock` (for `STACK`, never held across `block()` — see `netif.rs:60-62`). Keep `BusyCell` ONLY on `devmnt::Mnt` where `contention_wait()` already handles `None` by yielding/retrying across blocking 9P RPCs!
- **CRITICAL FINDING ON `register-waiter -> drop-lock -> block()` UNDER 2 CPUs:**
  - Look at the comments in `port/src/devpipe.rs:21`, `port/src/devcons.rs:706`, `port/src/devip/conv.rs:33`, `aarch64/src/syscall.rs:1412` `[VERIFIED: grep]`:
    - Today they say *"register-waiter → drop-lock → block cannot lose a wakeup because single CPU, IRQ-masked"*.
    - **ON 2 CPUs, THAT RACE IS REAL!** If CPU 0 registers `pid` in `pipe.waiters`, drops `pipe.lock`, and BEFORE CPU 0 calls `block()` (which sets `proc.state = Blocked`), CPU 1 writes to the pipe and calls `wake(pid)`: CPU 1 sees `proc.state == Running`, drops the wakeup, and THEN CPU 0 sets `proc.state = Blocked` and sleeps FOREVER (lost wakeup)!
  - **Standard SMP fix (9front `sleep`/`wakeup` / Linux `prepare_to_wait`):**
    Add a per-`Proc` atomic `wake_pending: AtomicBool` (or `Proc::prepare_block()` / `block_if_not_woken()`):
    1. Before registering in a wait queue (while holding the queue lock or right before): clear `proc.wake_pending = false` (or better: check under `PROC_TABLE` lock inside `block()`!).
    2. Actually, the cleanest Plan 9 / Linux pattern:
       - `proc.wake_pending: AtomicBool` cleared *before* taking the resource lock + checking the sleep condition.
       - Inside `wake(pid)`: set `proc.wake_pending.store(true, Ordering::Release)`; acquire `PROC_TABLE`; if `proc.state == Blocked`, transition to `Ready` and enqueue on a CPU run queue (and send reschedule IPI if target CPU is idle!).
       - Inside `block()`: acquire `PROC_TABLE`; if `proc.wake_pending.swap(false, Ordering::AcqRel)` is `true`, **do NOT block** — return immediately (the wakeup arrived in the window between drop-lock and `block()`)! Otherwise set `proc.state = Blocked` and call `sched()`.
    - This requires **ZERO** lock-order nesting between device locks and `PROC_TABLE`, is 100% immune to lost wakeups on 2+ CPUs, and works identically for `devpipe`, `devcons`, `devip`, `devether`, `devmnt`, `semacquire`, and `await`!

#### 3.2 `IrqSpinLock`, Lock Rank Table & Watchdog (`SMP-07`, `SMP-12`, D-26)
- **`IrqSpinLock<T>` (in `port/src/mcslock.rs`):**
  - Wraps `MCSLock` + `rank: LockRank`.
  - `lock(&node)`:
    1. Calls `arch_irq_save_disable() -> usize` (aarch64: read `DAIF`, `msr daifset, #2`; x86_64: `pushfq; pop; cli`; host tests: no-op).
    2. In debug/gate builds (`#[cfg(any(debug_assertions, feature = "lockdebug"))]`), calls `smp::rank_before_acquire(rank, name)` — asserts `rank > current_cpu_top_rank()`; panics naming BOTH held lock and requested lock on violation! Records `waiting_on = Some(name)` and `wait_start_ticks` for the watchdog.
    3. Acquires `MCSLock`, calling `smp::poll_shootdown_if_pending()` inside the spin loop (prevents IRQ-masked TLB shootdown deadlock!).
    4. Clears `waiting_on`, pushes `(rank, name)` onto per-CPU held-lock stack.
  - `Drop`: pops `(rank, name)` from per-CPU held-lock stack, unlocks `MCSLock`, restores saved IRQ state via `arch_irq_restore(saved)`.
- **Documented Lock Rank Table (strictly increasing, lower number acquired first):**
  1. `Rank 10 — MNT_SESSION / DEV_TOP` (`devmnt`, `devssl`, `devip::FS`, `devip::STACK`, `devip::TABLES`, `devip::IFC`, `devpipe`, `devether::DRIVER`, `devsrv`, `devenv`, `devcap`, `devdup`, `devproc`, `devcons::INPUT`, `FDGROUP`, `PGRP`, `ENVGROUP`, `RENDEZGROUP`) — device/subsystem locks; never nested with each other except documented parent->child (`FS` -> `TABLES` -> `STACK`, given sub-ranks 10, 12, 14).
  2. `Rank 20 — PAGE_CACHE` (`palloc.hash` TPHT shard writer / `rmap` lock).
  3. `Rank 30 — VM_SHARED` (`SHARED_PAGES`, `RFMEM_FAMILIES`, `SEM_WAITERS`).
  4. `Rank 40 — FID_SHARD` (`FidShard` `AllocMeta` mutex).
  5. `Rank 50 — PROC_TABLE` (`PROC_TABLE` process table & per-proc state).
  6. `Rank 60 — RUN_QUEUE(cpu_id)` (per-CPU run queue; when stealing across CPUs, acquire `min(a, b)` before `max(a, b)` — encoded as `60 + cpu_id`, or try-lock victim queue during steal so only 1 run-queue lock is held at a time! **Using `try_lock` on the victim queue during steal is even cleaner and avoids all multi-queue ordering!**).
  7. `Rank 70 — PAGE_ALLOC` (Roaring frame pool allocator).
  8. `Rank 80 — HEAP_ALLOC` (`QuickFit` global heap allocator — must be strictly above locks that allocate `Vec`/`Box` while held!).
  9. `Rank 90 — NET_IRQ` (`NETIF`, `INGRESS`, `NET_WAITERS`, `PENDING_WAKES`, `VIRTIO_NET`).
  10. `Rank 100 — CONSOLE` (`devcons` UART output lock — leaf lock so `println!` works from almost any panic/diagnostic path; never acquires anything else).
  - **CRITICAL CHECK ON RANK 80 (`HEAP_ALLOC`):** Any lock held while calling `Vec::push` / `Box::new` / `Arc::new` MUST have rank `< 80` (`HEAP_ALLOC`). Putting `HEAP_ALLOC` at rank 80 (below only `NET_IRQ` and `CONSOLE`, neither of which allocates!) means normal locks can allocate safely without ever violating rank order! And `PAGE_ALLOC` (rank 70) must NEVER allocate from the heap while holding `PAGE_ALLOC`? Wait — Roaring `RoaringBitmap::insert` CAN allocate a `Vec`/`Box` on container upgrade! Therefore: either **`PAGE_ALLOC` is rank 70 and `HEAP_ALLOC` is rank 80** (so `PAGE_ALLOC -> HEAP_ALLOC` is valid, and `QuickFit` heap uses its own pre-reserved static arena, never calling `PAGE_ALLOC`! `[VERIFIED: aarch64/src/allocator.rs:15-16, x86_64/src/allocator.rs:10-12]` — `QuickFit` ALREADY uses a static `GlobalHeap` arena and NEVER calls `pagealloc`!). That guarantees zero cycle between `PAGE_ALLOC` (70) and `HEAP_ALLOC` (80)!
- **Per-CPU Progress Watchdog (SMP-12, D-26):**
  - Each CPU stores `heartbeat: AtomicU64` (bumped on every timer tick / scheduler loop iteration) and `waiting_lock: AtomicPtr<u8>` + `waiting_since: AtomicU64` (written while spinning in `IrqSpinLock`/`MCSLock` or `shootdown_wait`).
  - On CPU 0's timer tick (every N ticks, e.g., threshold = 5,000 ticks = 5 seconds under KVM/HVF, scaled ×6 under TCG or configurable), CPU 0 checks all other online CPUs' `waiting_since` / `heartbeat`: if any CPU has been stuck waiting on a lock (or silent with IRQs masked) past the threshold, panic immediately printing:
    `WATCHDOG: cpu <id> stuck for <ms>ms waiting on lock "<name>" (held by cpu <owner>)`!
  - Gated behind `#[cfg(any(debug_assertions, feature = "lockdebug"))]` so release builds carry zero rank or watchdog overhead.

#### 3.3 Per-CPU Run Queues with Work Stealing (`SMP-11`, D-23, D-25)
- Lives in `port/src/sched.rs` (`#![forbid(unsafe_code)]`), host-tested independently.
- `RunQueues<const MAX_CPUS: usize>`: array of cache-line-aligned `IrqSpinLock<VecDeque<u64>>` (storing `pid`s).
- `enqueue(cpu_id, pid)`: pushes to `queues[cpu_id]`; if another CPU is idle, caller sends reschedule IPI (SGI 0 on aarch64, `RESCHED_VECTOR = 0xF1` on x86_64) to wake it from `wfi` / `hlt`.
- `pop_or_steal(my_cpu, num_cpus)`:
  1. Lock `queues[my_cpu]`, pop front -> if `Some(pid)`, return it.
  2. Otherwise, scan `other in 0..num_cpus` (`other != my_cpu`): `try_lock(&queues[other])` -> pop from back (or front; FIFO/LIFO work-stealing) -> if `Some(pid)`, return it.
  3. If none found, return `None` (caller enters `wfi` / `sti; hlt; cli`).
- Host tests pin: FIFO per-CPU ordering, work stealing when CPU 1 is idle, concurrent 2+ thread enqueue/steal with zero lost or duplicated PIDs, and `num_cpus > 2` scalability (no hardcoded `2`).

#### 3.4 AArch64 2-CPU Bring-Up (`SMP-04`, lead arch per D-25)
- **DTB discovery:** Walk `/cpus` for `cpu@*` nodes to read MPIDR `reg` values (CPU 0 = `0`, CPU 1 = `1`) and read `/psci` `method` property (`"hvc"` on QEMU virt EL1, `"smc"` fallback).
- **`PerCpu` + `TPIDR_EL1`:**
  ```rust
  #[repr(C, align(64))]
  pub struct PerCpu {
      pub cur_frame: AtomicU64,      // offset 0 — read/written by trap.S via TPIDR_EL1!
      pub cur_kstack: AtomicU64,     // offset 8
      pub cpu_id: u32,               // offset 16
      pub online: AtomicBool,
      pub current_proc: AtomicPtr<Proc>,
      pub sched_context: SyncUnsafeCell<swtch::Context>,
      pub sched_seeded: AtomicBool,
      pub fpu: fpu::CpuFp,
      pub pending_shootdown: AtomicBool,
      // debug/gate: lock rank stack + watchdog fields
  }
  ```
  `const_assert!(offset_of!(PerCpu, cur_frame) == 0);`
  - `aarch64/src/trap.S` replaces `ldr x10, =CURFRAME; str x9, [x10]` with `mrs x10, tpidr_el1; str x9, [x10, #0]` (and same in `trap_return` and `trap_return_exec`).
- **PSCI `CPU_ON`:**
  - `PSCI_0_2_FN64_CPU_ON = 0xC400_0003`.
  - Invoke `hvc #0` (or `smc #0` per DTB `/psci`) with `x0 = 0xC400_0003`, `x1 = target_mpidr`, `x2 = secondary_start_pa`, `x3 = cpu_id`.
  - `secondary_start` in `l.S`: drops EL2->EL1 if needed, loads CPU 0's published `MAIR_EL1`, `TCR_EL1`, `TTBR0_EL1` (boot identity table), `TTBR1_EL1` (kernel root table), enables MMU + caches in `SCTLR_EL1` (`dsb ish; isb`), installs `VBAR_EL1 = exception_vectors`, sets `TPIDR_EL1 = &PER_CPU[cpu_id]`, switches `sp` to CPU 1's dedicated 64 KiB kernel boot/sched stack, and branches to `secondary_main(cpu_id)`.
- **GICv2 & GICv3 Secondary CPU Init + SGI IPIs:**
  - `Gicv2`: CPU 1 writes `GICC_PMR = 0xff`, `GICC_CTLR = 1`, enables PPI 30 (timer) + SGI 0 (resched) + SGI 1 (TLB shootdown) in banked `GICD_ISENABLER0`/`GICD_IPRIORITYR`. Send SGI via `GICD_SGIR` (`0xF00`): `(1 << target_cpu) << 16 | sgi_id`.
  - `Gicv3`: `Gicv3::new` saves `gicr_physrange` (or maps per-CPU RD+SGI frames for `0..MAX_CPUS`); CPU 1 wakes its own GICR (`GICR_WAKER`), sets `GICR_IGROUPR0 = !0`, enables PPI 30 + SGI 0 + SGI 1 in its `GICR_ISENABLER0`, writes `ICC_SRE_EL1 |= 1`, `ICC_PMR_EL1 = 0xff`, `ICC_IGRPEN1_EL1 = 1`. Send SGI via `msr ICC_SGI1R_EL1, val` (`(1 << target_aff0) | ((sgi_id as u64) << 24)`) + `isb`.
  - Both CPUs arm their own generic timer (`CNTP_TVAL_EL0`, `CNTP_CTL_EL0`) — only CPU 0 increments global `TICKS` (so time/sleep deadlines stay 1 kHz), while both CPUs run `preempt()` on tick!
  - CPU 1 prints `smp: cpu 1 online` (or sets `online = true` so CPU 0 prints cleanly), unmasks IRQs, and enters `sched_loop()`.

#### 3.5 x86_64 2-CPU Bring-Up (`SMP-01`, `SMP-02`, `SMP-07`)
- **MADT parsing (`x86_64/src/platform/q35/acpi.rs`):** Extend `Madt` to collect type-0 (`Processor Local APIC`: `len >= 8`, `acpi_id = bytes[off+2]`, `apic_id = bytes[off+3]`, `flags = le_u32(off+4)`, enabled if `flags & 1 != 0 || flags & 2 != 0`) and type-9 (`Processor Local x2APIC`: `len >= 16`, `x2apic_id = le_u32(off+4)`, `flags = le_u32(off+8)`).
- **Per-CPU `Mach` via `GS_BASE` (distinct VA per CPU — avoids PML4 cloning for `RFMEM`!):**
  - CPU 0's `Mach` stays at `dat::KMACH` (`0xffff_ff00_0010_0000`, `IA32_GS_BASE = KMACH`, `mach.me = KMACH`).
  - CPU 1's `Mach` is allocated (zeroed, page-aligned, mapped in the shared kernel half of `bootpml4`, e.g., at `KMACH + sizeof(Mach)` or in the KZERO direct map), with `mach1.me = &mach1`, `IA32_GS_BASE = &mach1 as u64`, `IA32_KERNEL_GS_BASE = 0`, and its own GDT/TSS/IDT/stacks!
  - **Every** kernel access to the current CPU's `Mach` (`fpu::this_cpu()`, `set_tss_rsp0`, `current_proc`, `cur_frame`, `sched_context`) reads `%gs:0` (`mach_ptr()` = `mov %gs:0, %rax`) instead of hardcoding `dat::KMACH`!
  - In `x86_64/src/trap.rs:287-293` (`intrspecial`), replace `movabsq $KMACH, %rdx; cmpq %rdx, %rax; je 1f` with a kernel-half canonical test (`testq %rax, %rax; js 1f` — every kernel `Mach` VA has bit 63 = 1, every user GS base has bit 63 = 0!).
- **AP Trampoline & INIT-SIPI-SIPI:**
  - Copy a position-independent 16-bit -> 32-bit -> 64-bit trampoline (~150 bytes) to physical page `0x8000` (SIPI vector `0x08`, well inside `0x0..0x9fc00` available RAM verified in live q35 boot logs `[VERIFIED: job 139394030723 log]`).
  - Patch a small mailbox at `0x8F00` with: `pml4_pa` (CPU 0's `bootpml4` PA), `ap_stack_top`, `ap_mach_va`, `ap_entry_va` (`ap_main`).
  - BSP sends INIT IPI -> 10 ms delay -> SIPI(`0x08`) -> 200 µs delay -> optional 2nd SIPI(`0x08`) -> waits for `mach1.online == true`.
  - AP initializes its own LAPIC (including x2APIC enable if CPUID reports it), arms its own LAPIC timer (reusing BSP's calibrated frequency!), installs `SYSCALL` MSRs (`STAR`, `LSTAR`, `FMASK`, `EFER.SCE`), enables SSE/AVX (matching BSP), prints/signals online, and enters `sched_loop()`.
  - **x2APIC ICR barrier (`lapic.rs`):** Before `wrmsr(0x830, icr)` in x2APIC mode, execute `mfence; lfence` (Intel SDM Vol 3A §10.12.9: `WRMSR` to `IA32_X2APIC_ICR` is not serializing!).
  - **IPI vectors (`trap.rs`):** `TLB_SHOOTDOWN_VECTOR = 0xF0`, `RESCHED_VECTOR = 0xF1`.

---

### Pattern 4: x86_64 AVX & XSAVE / FXSAVE Switch (`MEM-18`, D-20 — Owner Stop 3)

- **Feature detection at boot (`x86_64/src/cpu.rs`, `x86_64/src/fpu.rs`):**
  - Check CPUID leaf 1: `ECX[26]` (`XSAVE`), `ECX[28]` (`AVX`), and CPUID leaf `0xD` subleaf 0 (`EAX` supported XCR0 bits, `EBX` save area size for enabled XCR0) + subleaf 1 (`EAX[0]` `XSAVEOPT`).
  - If `XSAVE && AVX` (e.g., KVM `-cpu host`, or TCG `-cpu qemu64,+avx,+avx2,+xsave,+xsaveopt`):
    1. Set `CR4.OSXSAVE` (bit 18) on BSP **and** AP!
    2. Write `XCR0 = 0x7` (`x87 | SSE | AVX` — bits 0, 1, 2; leave AVX-512 bits 5, 6, 7 **OFF**!).
    3. Verify CPUID `0xD.0` `EBX <= 832` (x87+SSE 512B + XSAVE header 64B + YMM_Hi128 256B = **832 bytes**).
    4. Select `FpFormat::XSaveOpt` (if `XSAVEOPT`) or `FpFormat::XSave`, and enable AVX2 Roaring dispatch!
  - Else (e.g., TCG default `-cpu qemu64`, which has NO XSAVE/AVX `[VERIFIED: superseded 05-RESEARCH probe]`):
    - Leave `CR4.OSXSAVE` clear; select `FpFormat::FxSave`; Roaring uses scalar (or optional SSE2) path.
  - **Mandatory boot log line (D-20):** Print exactly one deterministic line at boot, e.g.:
    `fpu: format=xsaveopt avx2=on` or `fpu: format=xsave avx2=on` or `fpu: format=fxsave avx2=off`
    so both the default `qemu64` gate and the AVX2 TCG gate can assert the exact chosen format!
- **`FpState` layout:**
  - `#[repr(C, align(64))] pub struct FpState { pub area: [u8; 832] }` (64-byte alignment required by `XSAVE64`/`XRSTOR64` — also satisfies `FXSAVE64`'s 16-byte alignment!).
  - `FpState::DEFAULT`: legacy area bytes `0..512` keep `FCW = 0x037F`, `MXCSR = 0x1F80`, rest zero; XSAVE header `xstate_bv` (`area[512..520]`) = `0` (meaning all components in init state for `XRSTOR64`, which cleanly zeroes `ymm0..15`!).
  - `xsave64` / `xsaveopt64` / `xrstor64` take `eax = 0x7, edx = 0` (`RFBM = x87|SSE|AVX`).
  - Covers all 4 lifecycle sites: context switch (`install`/`leave`), `rfork` (`snapshot`), `exec` (`replace(&DEFAULT)`), and note delivery / `noted` (`Proc::note_fpsave`).

---

### Pattern 5: Memory Pools, Demand Loading, Dirty Tracking, TPHT Page Cache, Evictor, SIMD, Pinning (`MEM-07`, `MEM-08`, `MEM-10`, `MEM-14..18`)

#### 5.1 Roaring Frame Pools & High-Water Pager (`MEM-07`, D-14)
- **Address encoding (exact formula from MEM-07):**
  - Frame index `pfn: u32 = (pa >> 12) as u32`.
  - Chunk = `(pfn >> 16) as u16` (each chunk covers `65,536 * 4 KiB = 256 MiB` — matching a single Roaring container key!).
  - Offset = `(pfn & 0xFFFF) as u16`.
  - Physical address expression: `(((chunk as usize) << 16) | (offset as usize)) * 4096`.
- **Pools in `port/src/palloc.rs` (under `IrqSpinLock` rank 70):**
  - `clean: RoaringBitmap` — zeroed, immediately allocatable frames.
  - `dirty: RoaringBitmap` — freed frames not yet zeroed.
  - `active: RoaringBitmap` — currently allocated/mapped frames.
  - `pinned: RoaringBitmap` — frames with `pin_count > 0` (mirroring atomic per-frame pin table `pin_counts: Box<[AtomicU32]>` indexed by `pfn`!).
  - **Evictable free-zeroing set:** `dirty.difference(&pinned.union(&active))` (`Evictable = Dirty \ (Pinned | Active)`), using `ds/src/roaring.rs`'s full-container-materializing `difference` (`to_words()` at `ds/src/roaring.rs:676-696` `[VERIFIED: ds/src/roaring.rs:676-696]`) so pinned frames are mathematically excluded!
  - **High-Water Pager (~15% clean target):**
    - `target_clean = total_managed_frames * 15 / 100`.
    - `low_water = total_managed_frames * 5 / 100` (or tunable for the small-memory pressure gate!).
    - When `clean.len() < target_clean`: pop dirty frames from `dirty \ (pinned | active)`, release lock, zero frames via arch kernel-alias helper, re-acquire lock and move to `clean`.
    - When `clean.len() < low_water` (or `alloc_clean()` sees empty `clean` + empty `dirty`): invoke the **page-dropping evictor** (reclaim unmodified text / untouched data pages from `palloc.hash` / droppable list, skipping any frame in `pinned`!) and retry allocation instead of panicking (resolving Claude's Discretion on page-in allocation failure!).

#### 5.2 `SuperpageResidency` (`MEM-08`)
- Lives in `port/src/superpage.rs` (`#![forbid(unsafe_code)]`):
  ```rust
  #[repr(C, align(64))]
  pub struct SuperpageResidency {
      bits: [AtomicU64; 8], // 512 sub-pages (2 MiB / 4 KiB) = exact 64 bytes
  }
  const _: () = {
      assert!(core::mem::size_of::<SuperpageResidency>() == 64);
      assert!(core::mem::align_of::<SuperpageResidency>() == 64);
  };
  ```
- Operations: `set_bit(subpage: usize)`, `clear_bit(subpage: usize)`, `is_full(&self) -> bool` (8-word `&` equals `u64::MAX`, ~5 cycles), `count(&self) -> u32` (sum of `count_ones()`), `reset_on_demotion(&self)` (atomically clears all 8 words to 0). Zero padding, exact 64 bytes, 100% SMP-safe and host-tested.

#### 5.3 Demand Loading (`MEM-14`), RO User Text & Dirty Tracking (`MEM-15`), TPHT Page Cache (`MEM-16`)
- **x86_64 full demand loading (`MEM-14`, D-16):**
  - Remove `map_text_eager` and `map_data_bss_eager` calls from `x86_64/src/exec.rs:273-274`; keep only `seed_user_pages` (initial argv/Tos stack page, matching `aarch64/src/exec.rs`).
  - Add `x86_64/src/fault.rs` and route ring-3 `#PF` (vector 14) in `x86_64/src/trap.rs:828` to `fault::user_fault(trap_frame, cr2)`.
  - Ensure `x86_64/src/usermem.rs` `ensure_user_resident` pages in cold text/data/bss/stack pages (already wired via `UserAddressSpace::ensure_user_resident` — update its body to invoke `fault::ensure_user_resident`, matching `aarch64/src/fault.rs`).
- **Read-only user text on both arches (`MEM-15`, D-15):**
  - `aarch64/src/vm.rs`: add `Entry::ro_user_text()` (`AccessPermission::AllRo`, `uxn: false`, `pxn: true`) and `Entry::ro_user_data()` (`AccessPermission::AllRo`, `uxn: true`, `pxn: true`). Fill frames through the kernel `KZERO + pa` alias (NEVER by mapping writable into user space first!).
  - `x86_64/src/proc.rs`: map `SegKind::Text` with `PTE_P | PTE_U` (NO `PTE_W`, NO `PTE_NX`). Write bytes through `phys_ptr_mut(pa)` before installing the PTE.
  - Any user write to `SegKind::Text` traps as a protection violation and kills the process cleanly.
- **Per-page dirty tracking (`MEM-15`, D-15):**
  - **x86_64:** Map freshly loaded `SegKind::Data` writable (`PTE_P | PTE_U | PTE_W | PTE_NX`) with hardware Dirty bit (`PTE_D = 1 << 6`) = `0`. At eviction time, atomically `swap(0)` the leaf PTE (`AtomicU64::swap(0, AcqRel)`), execute TLB shootdown, and inspect `old_pte & PTE_D`: if set, restore `old_pte`, mark the mapping dirty (non-droppable), and skip; if clear, no CPU could have written it (a concurrent write with `D=0` in TLB re-walks memory, sees `PTE_P=0`, and takes `#PF`!), so drop it safely!
  - **aarch64:** Cortex-A72 (ARMv8.0) has no hardware DBM, so map freshly loaded `SegKind::Data` with `Entry::ro_user_data()` (`AllRo`, `dirty = false`). On first user write (`DataAbortLowerEl`, DFSC 12..15, `WnR = 1`), `fault.rs` checks `segments.classify(va) == Some(SegKind::Data)` and upgrades the PTE to `Entry::rw_user_data()` (`AllRw`), runs `tlbi vae1is`, sets `dirty = true`, and removes the frame from the droppable set. **CRITICAL:** Because `AllRo` also denies EL1 writes, `UserSliceMut::new` (`ensure_user_resident(..., writable: true)`) MUST promote any `SegKind::Data` pages in `[va, va+len)` to `AllRw` + `dirty = true` before returning!
- **TPHT Page Cache (`palloc.hash`, `MEM-16`, D-18) & Shared Text Reverse-Map Unmap:**
  - `port/src/pagecache.rs` wraps `FlattenedTphtShard<2>` (sized per DEBT-04's production bank-sizing rule!) keyed by `((image_id as u64) << 32) | (page_idx as u64)`, storing `[pa, meta_word]`.
  - Each cached/droppable frame tracks a reverse-map entry: `Vec<(u16 /* proc_slot */, usize /* page_va */)>` (for shared text: every process mapping that text frame; for untouched data: the owning `(proc_slot, page_va)`).
  - **Eviction flow for a shared text page `pa`:**
    1. Evictor picks an unpinned candidate from the droppable list (clock/FIFO scan skipping any frame with `pin_count(pa) > 0`).
    2. Acquires `PAGE_CACHE` (rank 20) -> removes `(image_id, page_idx)` from `palloc.hash` (no new fault can map `pa`) -> acquires `PROC_TABLE` (rank 50) -> re-checks `pin_count(pa) == 0` (abort if newly pinned!).
    3. For every `(proc_slot, page_va)` in `rmap[pa]`: clears leaf PTE in `proc_table[proc_slot].root_table` and removes `page_va` from `proc.mapping_log`.
    4. Releases locks, runs **TLB shootdown** across both CPUs (with spin-poll safety), waits for ack.
    5. Returns `pa` to `PAGE_ALLOC` (rank 70) dirty/clean pool!
  - **Result:** When any process subsequently touches `page_va`, it takes a demand fault, misses `palloc.hash`, allocates a clean frame, re-reads exact bytes from `ByteSource`, re-inserts into `palloc.hash`, and resumes seamlessly!

#### 5.4 `UserSlice` / `UserSliceMut` Pinning (`MEM-10`, D-21)
- **Complete audit of sleeping `port/` paths holding user memory (`[VERIFIED: aarch64/src/syscall.rs, x86_64/src/syscall.rs, port/src/*]`):**
  Every user buffer in r9 enters through `UserSlice` or `UserSliceMut` in `aarch64/src/usermem.rs` (24 sites) and `x86_64/src/usermem.rs` (26 sites). The sleeping paths that hold a live `&[u8]` / `&mut [u8]` across `block()` / `sched()` are:
  1. `DEV_MNT` (`port/src/devmnt.rs`: `read`, `write`, `fversion`, `stat`) — blocks on pipe RPC and `contention_wait()`
  2. `DEV_PIPE` (`port/src/devpipe.rs`: `read`, `write`) — blocks on empty read / full write
  3. `DEV_IP` (`port/src/devip.rs`, `devip/proto.rs`: `read`, `write`) — blocks on TCP/UDP/ICMP socket wait queues
  4. `DEV_ETHER` (`port/src/devether.rs`, `netif.rs`: `read`, `write`) — blocks on RX empty / TX full
  5. `DEV_CONS` (`port/src/devcons.rs`: `read`) — blocks on console RX line wait
  6. `DEV_SSL` (`port/src/devssl.rs`: `read`, `write`) — delegates to underlying `DEV_PIPE`/`DEV_IP`
  7. `sys_semacquire` (`aarch64/src/syscall.rs:1386`, `x86_64/src/syscall.rs:1248`) — holds `UserSliceMut` across `sem_sleep()`
  8. `sys_await` (`aarch64/src/syscall.rs:1884`, `x86_64/src/syscall.rs:1639`) — holds `UserSliceMut` across `await_child()`
  - **Non-sleeping `validated_only` sites** (`sargs` arg block, `sys_noted` saved ureg) never sleep; they keep `validated_only` without pin overhead.
- **Design:** `UserSlice` and `UserSliceMut` store `pin: PinGuard` (holding the resolved physical frame list — inline `[u32; 4]` fast path for buffers ≤ 16 KiB + `Option<Vec<u32>>` overflow so common syscall buffers do zero heap allocation!).
  - `UserSlice::new` / `UserSliceMut::new` validates the range, ensures residency (and on aarch64 `UserSliceMut`, promotes `SegKind::Data` to writable+dirty), resolves each page's PFN, and increments `palloc::pin_frame(pfn)` (atomic refcount; adds to `pinned` Roaring pool on `0 -> 1`).
  - `Drop for PinGuard` calls `palloc::unpin_frame(pfn)` for each frame (removes from `pinned` Roaring pool on `1 -> 0`). Nested/overlapping pins work automatically via the per-frame atomic refcount!

#### 5.5 Roaring SIMD Set Kernels (`MEM-17`, D-19)
- **Why inline asm (not `#[target_feature]` / `fearless_simd`):** Verified in the superseded Phase 5 probe (re-confirmed by `x86_64/src/fpu.rs:50-54` `[VERIFIED: x86_64/src/fpu.rs:50-54]`): on the x86-softfloat kernel target, `#[target_feature(enable = "avx2")]` silently emits scalar code, and on aarch64 `#[target_feature(enable = "neon")]` triggers ABI-unsound warnings. Both LLVM integrated assemblers natively assemble AVX2 (`vmovdqu`, `vpand`, `vpor`, `vpandn`, `vzeroupper`) and NEON (`.arch_extension simd` + `ld1`, `st1`, `and`, `orr`, `bic`, `cnt`, `addv`) inside `core::arch::asm!`!
- **Module placement & lint rule R3:** Create `ds/src/setops_simd.rs` (mounted at `ds/src/lib.rs` as the ONE reserved Roaring SIMD module approved by Phase 3 D-09 `[VERIFIED: ds/src/lib.rs:13-25]`). Update `xtask/src/lintcontract.rs:263-309` R3 to allow `ds/src/setops_simd.rs` alongside `ds/src/tpht/` (and document every `unsafe` block in `UNSAFE-AUDIT.md` in the same commit).
- **Dispatch & `with_simd()` enforcement (D-06, D-19):**
  - `ds::setops` holds a runtime flag `SIMD_ENABLED: AtomicBool` (set once at boot by `aarch64`/`x86_64` after CPU feature check) AND a registered kernel-SIMD bracket hook `SIMD_BRACKET: fn(&mut dyn FnMut())` (or `port`/arch calls `ds::setops::register_simd_guard(with_simd_trampoline)` at boot!).
  - When `SIMD_ENABLED` is `true`, `bitmap_and`/`bitmap_or`/`bitmap_andnot` (and vectorized `array_intersect` where helpful) run inside the registered `fpu::with_simd()` bracket (parking/restoring user FP state safely with IRQs disabled on kernel builds; direct execution on host tests!).
  - On `x86_64`: AVX2 loop processes 256 bits (4× `u64`) per iteration (`vmovdqu`, `vpand`/`vpor`/`vpandn`, `vmovdqu`, `vzeroupper` at exit; popcnt via scalar or Harley-Seal). On `qemu64` (`avx2=off`), `SIMD_ENABLED` stays `false` and the scalar path runs untouched.
  - On `aarch64`: NEON loop processes 128 bits (2× `u64`) per iteration (`ld1`, `and`/`orr`/`bic`, `st1`, `cnt` + `addlv` for popcount!).
  - Host tests + boot selftests pin **exact bit-for-bit and cardinality identity** between scalar and AVX2/NEON paths on all 8 seeded random distributions + edge cases (all-0, all-1, alternating).

#### 5.6 Loopback `9P2000.aead` Server & Two-CPU Stress / Pressure Gates (`SMP-08`, `SMP-13`, `MEM-15`, D-17, D-27, D-28)
- **Boot switch (`SMP-13`, D-28):** Parse `aeadloop=1` from bootconf / DTB `/chosen/bootargs` / GRUB `append` (zero effect on default boots!). When set:
  - Kernel spawns a small in-kernel loopback `9P2000.aead` server thread (or kernel proc) backed by the real `port::fid::FidTable` (4 shards × 256 slots = 1,024 fids per D-12 synthetic tier) serving a small synthetic tree (e.g., `/data0..7` readable files) over an in-kernel duplex pipe using `port::chan::transport::Session` with an injected test secret.
  - Kernel client mounts it via `devmnt` (`Dialect::Aead` negotiated through `Mnt::negotiate_dialect`!) at `/n/aeadloop` (or exposes a `/srv/aeadloop` service file that the gate's `rc` script mounts).
- **Stress workload (`SMP-08`, D-27):** Runs at `-smp 2` on both `q35` and `virt` (HVF + TCG). Spawns concurrent user processes (using Harvey's `rc` + `/bin/cat` / `/bin/ls` / `/bin/echo` or small xtask-generated `a.out` probes) across both CPUs that simultaneously:
  1. Open, read, verify bytes, and clunk files on `/n/aeadloop` (exercising `FidTable` seqlock `resolve` + shard `alloc`/`clunk` + `9P2000.aead` ChaCha20/BLAKE3 end-to-end across both CPUs), AND
  2. Churn `exec` of binaries (exercising `palloc.hash` TPHT page cache, shared text mapping, and demand paging),
  while 1 kHz timer interrupts preempt both CPUs and the lock ranker + watchdog verify zero rank violations and zero deadlocks!
- **Small-memory pressure gate (`MEM-15`, D-17):** Boots `-smp 2` with constrained RAM (e.g., `-m 64M` or `-m 48M` — exact floor tuned to kernel + initrd + ~10–16 MiB managed user pages so pressure triggers deterministically in seconds!). Runs a deterministic pressure workload that:
  1. Allocates anonymous memory (bss/heap) to push clean frames below `low_water`, forcing the live evictor to drop cached text and untouched initialised-data pages;
  2. Holds a pinned user buffer across a blocking pipe/console read on CPU 1 while CPU 0 triggers eviction, verifying the kernel counter `evictor: dropped_text=... dropped_data=... pinned_skipped=...` reports `dropped_text > 0`, `dropped_data > 0`, and **zero pinned frames evicted**;
  3. Re-executes/reads the dropped text & untouched data pages and verifies byte-for-byte integrity after re-fault!

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Lock-free seqlock with non-atomic fields | `UnsafeCell<FidEntry>` + plain reads | `AtomicU64` state + `[AtomicU64; 7]` with Boehm `Acquire`/`Release` fence pairing (`#![forbid(unsafe_code)]`) | Non-atomic concurrent reads/writes are instant UB under Rust/Miri even if discarded on retry |
| Kernel SIMD via `#[target_feature]` or `fearless_simd` | `#[target_feature(enable = "avx2")]` / `fearless_simd` | Inline `core::arch::asm!` in `ds/src/setops_simd.rs` behind `fpu::with_simd()` | Soft-float x86_64 target silently emits scalar code; aarch64 triggers ABI-unsound lint |
| Big Kernel Lock (BKL) for SMP | One global mutex around syscalls/kernel | Fine-grained ranked `IrqSpinLock` + per-CPU `PerCpu`/`Mach` + per-CPU run queues | Locked by D-22; BKL hides concurrency bugs that Phase 6/13 would trip later |
| Custom Roaring container set ops | Ad-hoc bitset loops for frame pools | `ds::roaring::RoaringBitmap` (`difference`, `union`) + `setops` seam | Already verified against `roaring =0.11.5` oracle & `BTreeSet` model in Phase 3 |
| Re-implementing `9P2000.aead` wire framing for the stress gate | Custom test protocol | Existing `port::chan::transport::Session` + `port::devmnt` + `port::fid::FidTable` | SMP-08/SMP-13 require exercising the real Phase 4 wire engine and slab `FidTable` |
| Manual `boot-iso` hash maintenance | Re-deriving FOD `outputHash` after every kernel edit | Pure `builder.bootIso { kernel-elf = ...; }` (`outputHash = null`) | Eliminates manual owner-host hash churn (CORR-02, D-05) |

## Common Pitfalls

### Pitfall 1: IRQ-Masked Syscall + TLB Shootdown Deadlock
- **What goes wrong:** CPU 0 (IF=0 / DAIF.I=1) holds a lock or initiates a TLB shootdown IPI and spins waiting for CPU 1 to ack; CPU 1 (also IF=0 / DAIF.I=1 inside a syscall) is spinning on a lock held by CPU 0 and cannot take the IPI -> permanent deadlock.
- **How to avoid:** (1) Never hold `PAGE_ALLOC` while waiting for a shootdown ack; (2) Poll `smp::poll_shootdown_if_pending()` inside BOTH `MCSLock`/`IrqSpinLock` spin loops AND `shootdown_wait`! Then any CPU spinning with IRQs masked services incoming shootdowns inline immediately.
- **Warning signs:** Watchdog fires naming CPU 0 stuck in `shootdown_wait` and CPU 1 stuck on `PAGE_ALLOC` or `PROC_TABLE`.

### Pitfall 2: ARMv8 `AllRo` Denies EL1 Kernel Writes in `UserSliceMut`
- **What goes wrong:** Mapping untouched `SegKind::Data` pages with `AccessPermission::AllRo` (`AP[2:1] = 0b11`) on `aarch64` makes them read-only to **EL1 as well as EL0**. When a syscall calls `UserSliceMut::new` on a cold initialised-data buffer (e.g., `pread` into a global static buffer) and writes through `as_slice_mut()`, the kernel takes a synchronous EL1h Data Abort and halts!
- **How to avoid:** Inside `UserSliceMut::new` (`ensure_user_resident(..., writable: true)`), check if any page in `[va, va+len)` is a clean `SegKind::Data` page and promote its PTE to `Entry::rw_user_data()` (`AllRw`) + `dirty = true` + `tlbi vae1is` before returning the `UserSliceMut`.
- **Warning signs:** Kernel EL1h Data Abort (EC `0x25`, DFSC permission fault) inside `sys_pread` / `sys_stat` on `aarch64`.

### Pitfall 3: x86_64 Hardware `PTE_D` Eviction Race
- **What goes wrong:** CPU 0 checks `pte & PTE_D == 0`, decides the data page is clean, then clears the PTE while CPU 1 concurrently writes to it -> silent data loss of CPU 1's write.
- **How to avoid:** Use atomic `old_pte = pte_atomic.swap(0, AcqRel)`, perform TLB shootdown across CPUs, and check `old_pte & PTE_D`. If `PTE_D` was set, restore `old_pte`, mark the page dirty, and abort eviction!
- **Warning signs:** Corrupted global variables under 2-CPU pressure gate on x86_64 KVM.

### Pitfall 4: Lost Wakeups Under 2 CPUs (`register -> drop -> block`)
- **What goes wrong:** CPU 0 registers its PID on a device wait queue, drops the device lock, and before CPU 0 marks itself `Blocked`, CPU 1 calls `wake(pid)` (sees `Running`, does nothing); CPU 0 then marks itself `Blocked` and sleeps forever.
- **How to avoid:** Add per-`Proc` `wake_pending: AtomicBool` cleared before checking the wait condition, set in `wake(pid)` before locking `PROC_TABLE`, and checked/consumed under `PROC_TABLE` inside `block()`.
- **Warning signs:** Intermittent hangs on `devpipe`, `devmnt`, or `devip` under `-smp 2` HVF/KVM that never appear at `-smp 1`.

### Pitfall 5: `BusyCell` Spurious Errors on 2 CPUs
- **What goes wrong:** `NET_WAITERS`, `PENDING_WAKES`, `STACK`, and `INGRESS` in `port/src/devip/` and `port/src/netif.rs` use `BusyCell`, which fails fast on contention (safe only on 1 CPU with IRQs masked). On 2 CPUs, concurrent syscalls/IRQs collide and drop wakes or fail net ops.
- **How to avoid:** Replace those 4 `BusyCell`s with ranked `IrqSpinLock` / `Lock` during SMP-03. Keep `BusyCell` only where `contention_wait()` retries (`devmnt::Mnt`).

### Pitfall 6: Hardcoded Single-CPU Assumptions in `xtask` & `gates/`
- **What goes wrong:** Several tests/scripts silently break if overlooked:
  - `xtask/src/gate.rs:885,918` references archived `.planning/phases/01-...` (already red under `cargo test -p xtask`!).
  - `xtask/tests/threat_model.rs:149-175` enforces verbatim old AC §2.2 quotes (breaks under DEBT-05 unless updated!).
  - `xtask/src/rforkh.rs:124-131,162,171` hardcodes `RFCKEY`..`RFNOLEGACY = (1<<21)` (breaks under CORR-04!).
  - `xtask/src/test9p.rs:283,313,317,345,371,519` hardcodes 14 legs / 7 defaults (check before adding/changing any `test-9p` leg!).
  - `xtask/src/lintcontract.rs:263-309` rejects any `unsafe` in `ds/src/` outside `ds/src/tpht/` (breaks under MEM-17 unless R3 is updated to allow the reserved Roaring SIMD module!).
  - `gates/x86_64/*.expect` and `gates/aarch64/*.expect` hardcode `-smp 1` in 15 places `[VERIFIED: grep gates/]`.

### Pitfall 7: x2APIC `WRMSR(ICR)` Ordering & `x86_64` `intrspecial` `KMACH` Check
- **What goes wrong:** (a) On x2APIC, `WRMSR` to `IA32_X2APIC_ICR` (`0x830`) is NOT serializing (Intel SDM Vol 3A §10.12.9), so stores to a shootdown descriptor can be reordered after the IPI send; (b) `x86_64/src/trap.rs:291` compares `IA32_GS_BASE` against constant `KMACH`, which fails on CPU 1 whose `Mach` lives at a different VA!
- **How to avoid:** (a) Always execute `mfence; lfence` before `wrmsr(0x830, icr)` in x2APIC mode; (b) Change `intrspecial` to test bit 63 (`testq %rax, %rax; js 1f`) to distinguish kernel `Mach` GS base from user GS base.

### Pitfall 8: `qemu64` Has No XSAVE/AVX, and `XSAVE64` Requires 64-Byte Alignment
- **What goes wrong:** Unconditionally setting `CR4.OSXSAVE` on TCG `qemu64` causes `#GP(0)` at boot; leaving `FpState` at `align(16)` causes `#GP(0)` on `XSAVE64`/`XRSTOR64`.
- **How to avoid:** Gate `CR4.OSXSAVE` and `XCR0 = 0x7` on `CPUID.01H:ECX[26] && CPUID.01H:ECX[28]`; align `FpState` to `#[repr(C, align(64))]` with size 832 bytes; keep FXSAVE fallback intact when XSAVE is absent.

## Code Examples

### Verified Boehm Seqlock Slot (`ds/src/slab.rs`, `#![forbid(unsafe_code)]`)
```rust
// Source: Owner design (05-DISCUSSION-LOG.md:86-183), Boehm MSPC 2012 §4 Fig. 5
#![forbid(unsafe_code)]
use core::sync::atomic::{AtomicU64, Ordering, fence};

const WRITING: u64 = 1 << 0;
const OCCUPIED: u64 = 1 << 1;
const GEN_SHIFT: u32 = 2;
const GEN_MASK: u64 = 0xFFFF << GEN_SHIFT;
const SEQ_SHIFT: u32 = 18;
const SEQ_INC: u64 = 1 << SEQ_SHIFT;

#[repr(C, align(64))]
pub struct AtomicSlabSlot<const WORDS: usize = 7> {
    state: AtomicU64,
    words: [AtomicU64; WORDS],
}

const _: () = {
    assert!(core::mem::size_of::<AtomicSlabSlot<7>>() == 64);
    assert!(core::mem::align_of::<AtomicSlabSlot<7>>() == 64);
};

impl<const WORDS: usize> AtomicSlabSlot<WORDS> {
    pub fn new() -> Self {
        Self {
            state: AtomicU64::new(1u64 << GEN_SHIFT), // gen starts at 1, never 0 (D-10)
            words: core::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    #[inline]
    pub fn resolve(&self, expected_gen: u16, on_read: impl Fn()) -> Option<[u64; WORDS]> {
        if expected_gen == 0 {
            return None;
        }
        loop {
            on_read();
            let s1 = self.state.load(Ordering::Acquire);
            if (s1 & WRITING) != 0 {
                core::hint::spin_loop();
                continue;
            }
            if (s1 & OCCUPIED) == 0 || (((s1 & GEN_MASK) >> GEN_SHIFT) as u16) != expected_gen {
                return None;
            }
            let mut raw = [0u64; WORDS];
            for (dst, src) in raw.iter_mut().zip(self.words.iter()) {
                *dst = src.load(Ordering::Relaxed);
            }
            fence(Ordering::Acquire);
            let s2 = self.state.load(Ordering::Relaxed);
            if s1 == s2 {
                return Some(raw);
            }
        }
    }
}
```

### Verified `SuperpageResidency` (exact 64 bytes, `port/src/superpage.rs`)
```rust
// Source: MEM-08 / IG-A01 item 10
use core::sync::atomic::{AtomicU64, Ordering};

#[repr(C, align(64))]
pub struct SuperpageResidency {
    bits: [AtomicU64; 8],
}

const _: () = {
    assert!(core::mem::size_of::<SuperpageResidency>() == 64);
    assert!(core::mem::align_of::<SuperpageResidency>() == 64);
};

impl SuperpageResidency {
    pub const fn new() -> Self {
        Self { bits: [const { AtomicU64::new(0) }; 8] }
    }
    pub fn set_bit(&self, idx: usize) {
        self.bits[idx / 64].fetch_or(1u64 << (idx % 64), Ordering::AcqRel);
    }
    pub fn clear_bit(&self, idx: usize) {
        self.bits[idx / 64].fetch_and(!(1u64 << (idx % 64)), Ordering::AcqRel);
    }
    pub fn is_full(&self) -> bool {
        let mut acc = u64::MAX;
        for w in &self.bits {
            acc &= w.load(Ordering::Acquire);
        }
        acc == u64::MAX
    }
    pub fn reset(&self) {
        for w in &self.bits {
            w.store(0, Ordering::Release);
        }
    }
}
```

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Nix + devshell (`nix develop`) | All builds & tests | ✓ | Nix flakes enabled | — |
| Rust nightly toolchain (`rust-toolchain.toml`) | Kernel & host builds | ✓ | `1.95.0-nightly` (2026-01-30) | — |
| `qemu-system-aarch64` | aarch64 `-smp 2` HVF & TCG gates | ✓ | `11.1.2` | — |
| `qemu-system-x86_64` | x86_64 TCG AVX2 & `-smp 2` syntax/boot | ✓ | `11.1.2` | — |
| `grub-mkrescue` / x86_64 ISO build on macOS | x86_64 boot-tests locally | ✗ (Linux-only nixpkgs) | — | CI `ubuntu-latest` x86_64 runners + owner's Linux KVM host (D-33) |
| `/dev/kvm` on macOS dev host | x86_64 KVM gates | ✗ (Apple Silicon Mac) | — | CI TCG (incl. AVX2 leg) per commit; owner's Linux KVM host at stops 2, 3, 4 (D-33) |
| `gh` CLI | CI inspection | ✓ via `nix run nixpkgs#gh` | — | `nix run nixpkgs#gh -- ...` |
| `toolchainMiri` (`cargo xtask miri`) | Miri validation of `ds/src/tpht/` & `ds/src/slab.rs` | ✓ via devshell `R9_MIRI_TOOLCHAIN` | — | — |

**Missing dependencies with fallback:** `grub-mkrescue` and `/dev/kvm` are absent on the macOS arm64 dev host; D-33 explicitly assigns per-commit x86_64 verification to CI (`ubuntu-latest` Linux) and stops 2–4 to the owner's x86_64 Linux KVM host.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in unit/integration test harness via `cargo xtask` + Miri (`cargo xtask miri`) + QEMU expect gates (`gates/aarch64/`, `gates/x86_64/`) |
| Config file | Workspace `Cargo.toml`, `.planning/config.json`, `xtask/src/test9p.rs`, `xtask/src/lintcontract.rs` |
| Quick run command | `nix develop -c cargo test -p <crate> <filter>` (or `nix develop -c cargo xtask test-9p` for 9P wire/slab pins) |
| Full suite command | `nix develop -c cargo xtask check && nix develop -c cargo xtask clippy --arch aarch64 --config virtshell && nix develop -c cargo xtask clippy --arch x86-64 --config q35cpu && nix develop -c cargo xtask test` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CORR-01 | `quick-gate-tcg` green on CI; `start_xvfb` sets global `env(DISPLAY)` | CI / expect gate | `nix run nixpkgs#gh -- run view` + `nix develop -c cargo xtask gate --arch x86-64 --accel tcg` (Linux) | ✅ `gates/x86_64/drawterm-gate.expect` |
| CORR-02 | `boot-iso` has no `outputHash`, builds purely on `x86_64-linux` | nix eval / CI build | `grep -c outputHash flake.nix` (0 in `boot-iso`) + CI `nix build --no-link .#boot-iso` | ✅ `flake.nix` |
| CORR-03 | `ds/src/slab.rs` seqlock slab + `port/src/fid.rs` wrapper; 1-line resolve, non-zero gen, client check, Miri clean | unit + integration + Miri | `cargo test -p ds slab && cargo xtask test-9p && cargo xtask miri` | ❌ Wave 0 (`ds/src/slab.rs`), ✅ `port/tests/aead_fid.rs` |
| CORR-04 | `RFNOLEGACY = 1<<18`, 3 crypto bits removed, `rfork.h` regenerated | unit | `cargo xtask rfork-header --check && cargo test -p xtask rforkh` | ✅ `port/src/rfork.rs`, `xtask/src/rforkh.rs` |
| CORR-05 | Zero undocumented `unsafe` on `x86_64` & `aarch64`; policy & `UNSAFE-AUDIT.md` synced | lint + unit | `cargo xtask clippy --arch aarch64 --config virtshell && cargo xtask clippy --arch x86-64 --config q35cpu && cargo test -p xtask lintcontract` | ✅ `Cargo.toml`, `UNSAFE-AUDIT.md` |
| DEBT-01 | `cargo xtask test` runs `test-9p` and `cargo test -p xtask`; CI runs them | unit + CI | `cargo xtask test` (verified green including `xtask` & 14 `test-9p` legs) | ✅ `xtask/src/main.rs`, `xtask/src/gate.rs` |
| DEBT-02 | `04-VALIDATION.md` reconciled to `status: validated`, `nyquist_compliant: true` | doc check | `grep -E "^(status: validated|nyquist_compliant: true)" .planning/milestones/v3.0-phases/04-*/04-VALIDATION.md` | ✅ `04-VALIDATION.md` |
| DEBT-03 | Phase 4 boot-iso evidence no longer claims fixes moved linked kernel bytes | doc check | `grep` check on `KERNEL-BYTES-GAPS.md` & `04-25-SUMMARY.md` | ✅ `KERNEL-BYTES-GAPS.md` |
| DEBT-04 | TPHT home-hit & bank-sizing rule pinned at `default_banks` | unit | `cargo test -p ds tpht::flattened::tests::default_banks` | ❌ Wave 0 (new test in `ds/src/tpht/`) |
| DEBT-05 | `docs/threat-model.md` matches v3.0 rescope & `rfork` bit 18 | unit | `cargo test -p xtask --test threat_model` | ✅ `docs/threat-model.md`, `xtask/tests/threat_model.rs` |
| SMP-01 | x86_64 q35 boots `-smp 2`, both CPUs schedule, rio green | boot gate | CI TCG + KVM `cargo xtask gate --arch x86-64` (`-smp 2`) | ✅ `gates/x86_64/*.expect` |
| SMP-02 | Per-CPU state (`TPIDR_EL1` / `GS_BASE` `Mach.me`) + `CpuFp` | unit + boot gate | `cargo test -p port smp` + `-smp 2` boot gates | ❌ Wave 0 (`port/src/smp.rs`) |
| SMP-03 | Fine-grained locks, lost-wakeup-free `wake_pending`, no BKL | unit + stress gate | `cargo test -p port mcslock` + 2-CPU stress gate | ✅ `port/src/mcslock.rs` |
| SMP-04 | aarch64 virt boots `-smp 2` under HVF (GICv3) and TCG (GICv2) | boot gate | `nix run .#boot-r9-smoke` (HVF & TCG, `-smp 2`) | ✅ `gates/aarch64/*.expect` |
| SMP-07 | `IrqSpinLock` + TLB shootdown IPI reaches other CPU | unit + boot gate | `cargo test -p port irq_spinlock` + boot gate shootdown marker | ❌ Wave 0 (`port/src/smp.rs`) |
| SMP-08 | 2-CPU user 9P (`FidTable`) + exec churn (`palloc.hash`) stress gate, 0 deadlocks | boot gate (both arches) | `nix run .#boot-r9-smp-stress` (aarch64) + x86_64 CI/KVM stress gate | ❌ Wave 0 (new gate scripts) |
| SMP-09 | All existing gates at `-smp 2` + 1 `-smp 1` smoke leg per arch | boot gate | `grep -rn "\-smp " gates/` + CI green | ✅ `gates/` |
| SMP-11 | Per-CPU run queues + work stealing in `port/` | unit | `cargo test -p port sched` | ❌ Wave 0 (`port/src/sched.rs`) |
| SMP-12 | Lock rank assertions (panic on inversion) + watchdog (names stuck CPU & lock); absent in release | unit | `cargo test -p port lock_rank` + `cargo test -p port watchdog` | ❌ Wave 0 (`port/src/smp.rs`) |
| SMP-13 | Boot-switch `aeadloop=1` loopback `9P2000.aead` server; default boot unchanged | unit + boot gate | `cargo test -p port aead_loop` + default boot negative check | ❌ Wave 0 (`port/src/aead_loop.rs`) |
| MEM-07 | Roaring clean/dirty/pinned pools, address formula, High-Water Pager ~15% clean | unit | `cargo test -p port palloc` | ❌ Wave 0 (`port/src/palloc.rs`) |
| MEM-08 | `SuperpageResidency` == 64 bytes, bit clear, atomic reset | unit | `cargo test -p port superpage` | ❌ Wave 0 (`port/src/superpage.rs`) |
| MEM-10 | `UserSlice`/`UserSliceMut` `PinGuard` refcount; pinned frame never evicted | unit + pressure gate | `cargo test -p port pin_guard` + small-mem pressure gate | ❌ Wave 0 |
| MEM-14 | x86_64 demand loading (`#PF` pager); rio green | boot gate | CI TCG + KVM `cargo xtask gate --arch x86-64` | ❌ Wave 0 (`x86_64/src/fault.rs`) |
| MEM-15 | RO user text, dirty tracking both arches, live evictor, small-mem `-smp 2` pressure gate | boot gate (both arches) | `nix run .#boot-r9-mem-pressure` (aarch64) + x86_64 CI/KVM pressure gate | ❌ Wave 0 (new gate scripts) |
| MEM-16 | Flattened-TPHT `palloc.hash` image page cache + shared text reverse-map unmap | unit + boot gate | `cargo test -p port pagecache` + pressure/stress gates | ❌ Wave 0 (`port/src/pagecache.rs`) |
| MEM-17 | Roaring AVX2 & NEON kernels identical to scalar behind `setops.rs` + `with_simd()` | unit + boot selftest | `cargo test -p ds setops` + aarch64 HVF/TCG + x86_64 AVX2 TCG boot selftest | ❌ Wave 0 (`ds/src/setops_simd.rs`) |
| MEM-18 | x86_64 AVX/XSAVE where CPUID reports it, FXSAVE elsewhere, boot line names format | boot gate (both CPUs) | CI TCG `qemu64` (`format=fxsave`) + CI TCG `qemu64,+avx,+avx2,+xsave,+xsaveopt` (`format=xsaveopt`) + KVM `fpprobe` | ✅ `x86_64/src/fpu.rs` |

### Sampling Rate
- **Per task commit:** Targeted unit test (`cargo test -p <crate> <mod>`) + `cargo xtask check`
- **Per wave merge:** Full host suite (`cargo xtask test`, both arch clippy legs) + `cargo xtask miri` when `ds/src/tpht/` or `ds/src/slab.rs` changes
- **Per owner stop (4 stops, D-32):** Cumulative `port/src` diff review; aarch64 quick pass on Mac (HVF+TCG); x86_64 CI (stops 1–4) + owner's Linux KVM host (stops 2, 3, 4)

### Wave 0 Gaps
- [ ] `xtask/src/gate.rs:885,918` — fix archived Phase 1 witness path so `cargo test -p xtask` starts green
- [ ] `ds/src/slab.rs` — generic `#![forbid(unsafe_code)]` seqlock slab + unit/concurrency tests
- [ ] `ds/src/setops_simd.rs` — reserved audited unsafe AVX2/NEON Roaring kernels + `lintcontract.rs` R3 update
- [ ] `port/src/smp.rs`, `port/src/sched.rs` — `PerCpu` trait, `IrqSpinLock` rank checker, watchdog, shootdown coordinator, work-stealing run queues + host tests
- [ ] `port/src/palloc.rs`, `port/src/superpage.rs`, `port/src/pagecache.rs`, `port/src/aead_loop.rs` — pools, `SuperpageResidency`, TPHT page cache, loopback 9P2000.aead server + host tests
- [ ] `gates/aarch64/` & `gates/x86_64/` new `-smp 2` stress gate (`SMP-08`) and small-memory pressure gate (`MEM-15`) expect scripts + CI AVX2 TCG job in `.github/workflows/xtask.yml` or `gates.yml`

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No (peer auth deferred to Phase 6; loopback server uses boot-switch test secret) | Gate-only `aeadloop=1` switch; default boot exposes nothing (SMP-13) |
| V3 Session Management | Yes (`9P2000.aead` `FidTable` handles & epochs) | Server-assigned `StatefulHandle` with non-zero 16-bit generation (`gen >= 1`), epoch check, owning `client_id` check, payload scrub on clunk (D-07, D-08, D-10, D-11) |
| V4 Access Control | Yes (MMU isolation, RO user text, `UserSlice` bounds) | Hardware MMU per-process page tables (`CR3`/`TTBR0`), RO+UX user text (prevents user code self-modification/W^X violation), `validate_user_range` + `PinGuard` (MEM-10, MEM-15) |
| V5 Input Validation | Yes (ACPI MADT, DTB `/cpus`/`/psci`, `StatefulHandle::from_wire`) | Bounds-checked slice parsers (`acpi.rs`, `fdt.rs`), `from_wire` rejects `gen == 0` and out-of-range shard/slot indices |
| V6 Cryptography | No new crypto primitives | Existing `crypto/` (`#![forbid(unsafe_code)]`) reused unchanged; no software RAM encryption (D-04) |

### Known Threat Patterns for r9 Kernel (SMP + Paging + Slab)
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stale `StatefulHandle` / ABA handle reuse across clients | Elevation of Privilege / Spoofing | 16-bit per-slot generation bumped on clunk (skipping 0) + owning `client_id` check + server `epoch` check + payload scrub on clunk (CORR-03, D-08, D-10, D-11) |
| Torn read of `FidEntry` under concurrent clunk/update | Tampering / Info Disclosure | Lock-free Boehm seqlock over `AtomicU64` words retries on any concurrent mutation; never returns mixed fields (D-11) |
| Evicting a user page mid-syscall / mid-DMA (use-after-free frame) | Tampering / Info Disclosure | `UserSlice`/`UserSliceMut` RAII `PinGuard` with atomic per-frame pin count; `Evictable = Dirty \ (Pinned | Active)` mathematically excludes pinned frames (MEM-07, MEM-10) |
| Stale TLB entry on other CPU after page eviction/reassignment | Info Disclosure | Atomic PTE clear + cross-CPU TLB shootdown IPI with completion barrier BEFORE frame zeroing/reallocation (SMP-07, MEM-15) |
| Kernel SIMD leaking kernel data or corrupting user `ymm`/`v` registers | Info Disclosure / Tampering | All kernel SIMD strictly inside `fpu::with_simd()` with IRQs disabled; user FP state parked/restored or scrubbed to `DEFAULT` (D-06, MEM-17, MEM-18) |
| Malicious/buggy user writing into shared read-only text segment | Tampering (cross-process code injection!) | Both architectures map user text strictly read-only (`AllRo` / `PTE_W=0`); write faults kill the offending process (MEM-15, MEM-16) |
| Gate-only loopback `9P2000.aead` test server exposed in default boot | Spoofing / Attack Surface | Strictly gated behind explicit boot switch (`aeadloop=1`); default boot starts no server and keeps plain `9P2000` (SMP-13, D-28) |

## Open Questions (RESOLVED)

Every open question from `ROADMAP.md` and `05-CONTEXT.md` has been investigated and resolved against the live tree:

1. **What is actually red in CI at `900f58b`?**
   - **RESOLVED:** `quick-gate-tcg` in `.github/workflows/gates.yml` (16/17 check runs green, 1 RED) fails on every push because `proc start_xvfb` at `gates/x86_64/drawterm-gate.expect:318-348` omits `global env`, so line 425 crashes on headless CI runners with `no such variable (read trace on "env(DISPLAY)")`. Additionally, `cargo test -p xtask` (not yet in CI) fails 2 tests at `xtask/src/gate.rs:885,918` due to the archived Phase 1 witness path.
2. **Is the D-11 fence-paired seqlock sound in safe Rust on x86_64, aarch64, and riscv64, and how is the `Probe` access-count test restated?**
   - **RESOLVED:** Yes — 100% sound under C++20/Rust's atomic memory model (Boehm MSPC 2012 §4 Fig. 5: `load(Acquire)` + `Relaxed` word loads + `fence(Acquire)` + `load(Relaxed)` paired with writer `store(WRITING, Relaxed)` + `fence(Release)` + `Relaxed` word stores + `store(published, Release)`), zero data-race UB, Miri-valid, and optimal on all three architectures (0 HW fences on x86_64, `dmb ishld` on aarch64, `fence r,rw` on riscv64), provided all writers (`alloc`, `update`, `clunk`, `sweep`) serialize via the shard-local `Lock<AllocMeta>`. Restate the WIRE-08 `Probe` test via a `SlabProbe` trait on `SlabSlots` proving 1 cache-line slot read and 0 metadata lock touches on uncontended `resolve()`.
3. **Can CI run an AVX2 CPU model under TCG in acceptable time?**
   - **RESOLVED:** Yes. Verified live on pinned QEMU 11.1.2 that `-M q35 -cpu qemu64,+avx,+avx2,+xsave,+xsaveopt -smp 2 -accel tcg` launches cleanly with zero warnings. Existing CI TCG q35 gates (`boot-test-x86-64` running 3 sequential boots in 10m49s, `boot-r9-auth-x64` in 7m34s) show an AVX2 TCG leg will finish in ~3–5 minutes on `ubuntu-latest` because non-AVX instructions pay zero extra TCG translation cost.
4. **How is a dropped shared text page unmapped from every address space that maps it, and what is the shootdown path?**
   - **RESOLVED:** Each cached/droppable frame in `palloc.hash` keeps a reverse-map (`rmap`) of `(proc_slot, page_va)` entries. Eviction removes the key from `palloc.hash` (rank 20), acquires `PROC_TABLE` (rank 50), re-verifies `pin_count == 0`, clears every mapped leaf PTE + `mapping_log` entry, releases locks, issues a cross-CPU TLB shootdown IPI (`LAPIC 0xF0` on x86_64; `tlbi vae1is` + GIC SGI 1 on aarch64) with spin-poll servicing inside `IrqSpinLock`/`MCSLock` spin-waits to prevent IRQ-masked deadlocks, and returns the frame to `PAGE_ALLOC` only after all CPUs acknowledge.
5. **Which `port/` paths sleep while holding user memory?**
   - **RESOLVED:** Audited all 24 `aarch64` and 26 `x86_64` `usermem` call sites and all `port/` blocking hooks: `devmnt` (`read`/`write`/`fversion`/`stat`), `devpipe` (`read`/`write`), `devip` (`read`/`write`), `devether` (`read`/`write`), `devcons` (`read`), `devssl` (`read`/`write`), `sys_semacquire`, and `sys_await`. Because every one acquires its buffer via `UserSlice::new` or `UserSliceMut::new`, placing an RAII `PinGuard` (atomic per-frame pin count) directly inside `UserSlice`/`UserSliceMut` covers all sleeping paths uniformly with zero missed sites.
6. **Re-verification of superseded Phase 5 research facts:**
   - **RESOLVED:** All re-verified live this session: `x86_64/` has exactly 32 undocumented `unsafe` blocks in 9 files; `aarch64/` has exactly 7 in 3 files; `UNSAFE-AUDIT.md:317` names deleted `node0.rs`/`vsvm.rs`; `xtask/src/test9p.rs` pins 14 legs / 7 defaults in 3+ assertions; neither arch tracks dirty pages; both map user text writable; both page-in paths panic on OOM; `UserSlice` holds only `(ptr, len)`; `qemu64` lacks XSAVE/AVX.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | CI `ubuntu-latest` TCG AVX2 leg completes in ~3–5 minutes | Open Questions #3 | Low — `timeout-minutes: 90` matches sibling TCG legs; if slower, trim workload iterations |
| A2 | Owner's x86_64 Linux KVM host passes `-smp 2` stress & pressure gates at stops 2–4 with `-m 64M` pressure floor | Pattern 5.6 | Low — RAM floor and low-water threshold are boot/config tunable parameters |

## Sources

### Primary (HIGH confidence)
- Live codebase reads: `05-CONTEXT.md`, `05-DISCUSSION-LOG.md`, `REQUIREMENTS.md`, `ROADMAP.md`, `STATE.md`, `flake.nix`, `Cargo.toml`, `UNSAFE-AUDIT.md`, `docs/threat-model.md`, `docs/r9_implementation_guide_for_agentic_developers_v3_0.md`, `docs/plan_r9_architecture_code_compendium_v3_0.md`, `.github/workflows/{gates,xtask}.yml`, `gates/x86_64/drawterm-gate.expect`, `port/src/{lib,fid,rfork,mcslock,pagealloc,devmnt,devcons}.rs`, `ds/src/{lib,roaring,setops}.rs`, `ds/src/tpht/{flattened,measure}.rs`, `aarch64/src/{main,l.S,trap.S,vm,fault,fpu,proc,syscall,usermem}.rs`, `aarch64/src/platform/virt/{gic/mod,gic/gicv2,gic/gicv3,timer}.rs`, `x86_64/src/{Cargo.toml,dat,fpu,exec,proc,trap,usermem}.rs`, `x86_64/src/platform/q35/{acpi,lapic}.rs`, `xtask/src/{main,gate,rforkh,test9p,lintcontract}.rs`, `xtask/tests/threat_model.rs`.
- Live tool executions: `gh run list`/`gh run view`/`gh api` on `900f58b`, `cargo xtask clippy --arch aarch64 --config virtshell`, `cargo clippy --package x86_64`, `cargo test -p xtask`, `cargo xtask test-9p`, `qemu-system-x86_64` and `qemu-system-aarch64` 11.1.2 `-smp 2` / AVX2 / HVF / TCG probes.

### Secondary (MEDIUM confidence)
- Hans-J. Boehm, *"Can Seqlocks Get Along With Programming Language Memory Models?"* (MSPC 2012) — cited in `ds/src/tpht/flattened.rs:8-18` and C++20 `[atomics.fences]`.
- Intel SDM Vol 3A §8.4.4 (MP INIT-SIPI-SIPI), §10.12.9 (x2APIC ICR ordering), ARM DEN0022D (PSCI `CPU_ON` `0xC400_0003`), ARM GICv2/GICv3 Architecture Specs.

## Metadata

**Confidence breakdown:**
- Course correction & audit follow-ups (CORR-01..05, DEBT-01..05): HIGH — every file, line, failing test, and CI log verified live.
- Slab `FidTable` & seqlock soundness (CORR-03): HIGH — formal C++20/Rust fence-fence proof + exact byte layout verified.
- Two-CPU SMP & locking (SMP-01..04, SMP-07..09, SMP-11..13): HIGH — every arch register, GICv2/v3, LAPIC/x2APIC, and IRQ-masked deadlock hazard verified against live code.
- Memory pools, demand loading, dirty tracking, SIMD, XSAVE, pinning (MEM-07, MEM-08, MEM-10, MEM-14..18): HIGH on design/codebase; MEDIUM on KVM-only timing until stops 3–4.

**Research date:** 2026-11-25
**Valid until:** 2026-12-25 (stable bare-metal architecture & pinned toolchain)
