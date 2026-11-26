# Phase 5: Guide Phase 3 - Early SMP, Course Correction and Memory Pools - Context

**Gathered:** 2026-11-25
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers three things, in this order:

1. **Course correction and audit follow-ups.** `main` is green and matches the v3.0 documents: CI repaired, `boot-iso` a pure derivation, `FidTable` on a sharded generational slab, the three cryptographic `rfork` bits retired with `RFNOLEGACY` at bit 18, the unsafe policy stated the same way everywhere, and the five v3.0 audit follow-ups closed (CORR-01 to CORR-05, DEBT-01 to DEBT-05).
2. **Two-CPU SMP.** r9 schedules on two CPUs on x86_64 q35 and aarch64 virt with per-CPU state, fine-grained SMP-safe locking, interrupt-safe locks and TLB shootdown, proven by a two-CPU stress gate (SMP-01 to SMP-04, SMP-07 to SMP-09, SMP-11 to SMP-13).
3. **Memory pools, non-cryptographic.** Roaring clean, dirty and pinned frame pools, a pager that drops re-readable pages, demand loading on both architectures, a TPHT image page cache, SIMD set kernels, and pin guards (MEM-07, MEM-08, MEM-10, MEM-14 to MEM-18).

Out of this phase: more than two CPUs, the full arm64 HVF series and the WINDOWS items (Phase 8); any software encryption of memory, `PKEY` hooks or a backing store for dirty anonymous pages; any distributed 9P extension (Phase 6). The in-kernel loopback server of D-28 is gate wiring, not a served feature.

The discussion on 2026-11-25 enlarged this phase beyond its roadmap entry at the owner's choice (D-13 to D-19, D-22, D-23, D-27, D-28). The owner was shown the size and chose to keep one phase and one planning pass (D-31). New requirement rows MEM-14 to MEM-18 and SMP-11 to SMP-13 record the additions (D-35).

</domain>

<decisions>
## Implementation Decisions

### Carried in (not re-asked)
- **D-01:** The course-correction tasks (CORR-01 to CORR-05) and the audit follow-ups (DEBT-01 to DEBT-05) are the phase's first plans, executed before any SMP or memory work (owner, 2026-11-25). Before planning the CI fix, check what is actually red at `900f58b`; it has not been verified. `origin/main` is at `900f58b`.
- **D-02:** `RFNOLEGACY` is `1 << 18`; `RFCKEY`, `RFROBASE` and `RFRANDMEM` are removed; `port/` stays per-module forbid with its three floor modules (`globalalloc`, `mcslock`, `devip_waker`) (owner, 2026-11-25).
- **D-03:** Every requirement follows IG-V01 and v3.0 guide rule 1: committed RED pin, implementation, GREEN twice, pin advance. The golden invariant (boots to rio on q35) holds at every commit. `UNSAFE-AUDIT.md` changes in the same commit as any `unsafe` block in `ds/src/tpht/` or an arch crate (guide rule 3).
- **D-04:** Owner standing rules apply: no push without approval, no AI attribution, Harvey read-only. No software RAM encryption, `PKEY`/`WRPKRU` hooks or compiler rewrites (guide rule 4). D-18 and D-19 are not a deviation from rule 4: they enable AVX for set algebra, not for page encryption.
- **D-05:** x86_64 kernel byte change: the old procedure (re-derived `boot-iso` `outputHash`, x64 TCG gates in an amd64 container, owner's linux boot-iso check) applies until CORR-02 lands; after that, green CI on the commit is the check. aarch64 kernel byte change: the quick pass (tier-1 on HVF and TCG; smoke, net-smoke, cpu once on HVF). Check whether CORR-02 has landed before applying either.
- **D-06:** Kernels are built without FP/SIMD; kernel SIMD goes through `fpu::kernel_begin()` / `with_simd()`; user FP state is per process; arm64 is the reference for x86_64 (owner standing rule).

### Slab `FidTable` (CORR-03)
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

### Memory pools and paging (MEM-07, MEM-08, MEM-10, MEM-14 to MEM-18)
- **D-14:** With no backing store, eviction means dropping re-readable pages. The pools are: dirty (a freed frame not yet zeroed), clean (zeroed and ready), pinned (not reclaimable). The pager zeroes dirty frames to hold about 15% clean, and under pressure unmaps unmodified file-backed pages that can be read again from their image. Dirty anonymous memory (bss, stack, heap, written data) is never taken from a process.
- **D-15:** Droppable pages are user text and initialised-data pages a process has not yet written. User text is mapped read-only on both architectures (today both map it writable). Per-page dirty tracking is built: the PTE dirty bit on x86_64 and a write-protect fault scheme on aarch64, or a better mechanism research finds.
- **D-16:** x86_64 moves to full demand loading: exec stops mapping eagerly and every image page is loaded at first touch, as aarch64 does. One model on both architectures. — **Reversibility:** costly — every x86_64 boot's exec and fault path changes, and the golden gate rides on it.
- **D-17:** The evictor is live in every boot and acts when clean frames fall below a low-water mark. It is proven by a small-memory `-smp 2` boot gate on both architectures running a workload built to trigger pressure drops deterministically. The gate proves dropped pages fault back intact and that a pinned frame is never chosen. Host tests cover the pool algebra.
- **D-18:** The image page lookup is the Flattened-TPHT page cache the compendium names (`palloc.hash`), built in this phase on `FlattenedTphtShard`. It is the first default-bank TPHT consumer, so DEBT-04 (home-hit measured at production bank sizes, sizing rule written down) is done before it. Shared text maps one frame into every process running the image.
- **D-19:** The Roaring set kernels get SIMD in this phase on both architectures: AVX2 on x86_64 and NEON on aarch64, in the one reserved unsafe-holding module in `ds/` (Phase 3 D-09), behind the `ds/src/setops.rs` seam, reached through `with_simd()`. The path is chosen by CPU feature at boot, a scalar path remains, and scalar and vector outputs are pinned identical.
- **D-20:** On x86_64, AVX is enabled where CPUID reports it and per-process FP state moves from FXSAVE to XSAVE on those CPUs, so user `ymm` state survives a switch, `rfork`, exec reset and note delivery. On a CPU without XSAVE (TCG's default `qemu64`, used by CI and the quick gate) the FXSAVE path is kept unchanged and the set kernels run a non-AVX path. One boot line names the chosen format so a gate can assert it. — **Reversibility:** costly — the FP save format and every context-switch path on AVX-capable hosts.
- **D-21:** Pinning (MEM-10): `UserSlice` and `UserSliceMut` gain a pin guard released on drop, with a per-frame pin count so nested pins work. Every `port/` path that holds user memory across a sleep (devmnt, pipes, the network devices, console) is audited and takes the guard.

### SMP shape (SMP-01 to SMP-04, SMP-07 to SMP-09, SMP-11 to SMP-13)
- **D-22:** Locking is fine-grained from the start, with no big kernel lock. Every structure whose safety rests on the single-CPU invariant gets its own lock or a per-CPU copy before the second CPU is allowed to schedule. — **Reversibility:** costly — the audit touches about 29 files and about 70 raw global statics across both arch crates.
- **D-23:** Each CPU has its own run queue, and an idle CPU steals work from the other. The design should not hard-code two CPUs, but only two are brought up and gated here; more is Phase 8 (SMP-10).
- **D-24:** Once both CPUs schedule, every existing gate on both architectures moves to `-smp 2` and that becomes the golden invariant. One smoke leg per architecture stays at `-smp 1`.
- **D-25:** aarch64 brings up the second CPU first; x86_64 follows on the proven shared code. `IrqSpinLock`, the per-CPU trait, the run queues and the shootdown request logic live in `port/` and are host-tested. Each arch crate supplies only CPU start (PSCI `CPU_ON`; INIT-SIPI-SIPI from the MADT), per-CPU register access (`TPIDR_EL1`; `GS` base) and the IPI (GIC SGI; LAPIC vector).
- **D-26:** Every lock has a rank in one documented table. Debug and gate builds assert that a CPU takes locks only in increasing rank and panic with both lock names on a violation. A per-CPU progress watchdog names the stuck CPU and the lock it waits on, so a deadlock is a named failure and not a gate timeout. Release builds carry neither.
- **D-27:** The two-CPU stress gate (SMP-08) is a user workload over 9P: user processes on both CPUs drive opens, reads and clunks through a mount so `FidTable` is exercised end to end, plus exec churn for the TPHT page cache, with timer interrupts on both CPUs.
- **D-28:** A boot switch starts a small in-kernel `9P2000.aead` server end backed by the real `FidTable` over a loopback link with an injected test secret, and the kernel's own client engine mounts it. This is gate-only wiring; a default boot exposes nothing new and still speaks plain 9P2000 (Phase 4 D-16/D-23 unchanged).

### Order, stops and review
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` — Phase 5 entry: goal, requirements, success criteria, open questions
- `.planning/REQUIREMENTS.md` — CORR-01 to CORR-05, DEBT-01 to DEBT-05, SMP-01 to SMP-04, SMP-07 to SMP-09, SMP-11 to SMP-13, MEM-07, MEM-08, MEM-10, MEM-14 to MEM-18; the Retired table
- `.planning/STATE.md` — standing owner rules, carried items, practical notes
- `.planning/PROJECT.md` — Key Decisions table; notes N1 to N11 (N7 to N11 void or reduced)

### Authoritative v3.0 documents (outrank older sources where they differ)
- `docs/r9_implementation_guide_for_agentic_developers_v3_0.md` — section 1 (five course-correction tasks), section 2 (Phase 3 exit gate), section 3 (operating rules 1 to 4)
- `docs/plan_r9_architecture_code_compendium_v3_0.md` — section 2 (crate safety boundaries), section 3 (data structure matrix), section 4 (slab listing; amended by D-07, D-08, D-11, D-12), section 6 (two-CPU SMP architecture)

### Wire contract and prior decisions
- `.planning/rfc-9p2000-aead.md` — section 5 `FidHandle`, `StatefulHandle`, `FidEntry` (amended by D-13); section 4 envelope and AAD
- `.planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-CONTEXT.md` — D-10 (three-place amendment pattern), D-11, D-12 (all-zero discriminator), D-13, D-14, D-16, D-23, D-24 (waiver precedent), D-27
- `.planning/milestones/v3.0-phases/03-guide-phase-1-crypto-and-ds-foundations/03-CONTEXT.md` — D-09 to D-13 (unsafe and lint contract; the one reserved Roaring SIMD module)
- `.planning/milestones/v3.0-MILESTONE-AUDIT.md` — INT-02 to INT-05 and WR-04, the source of DEBT-01 to DEBT-05
- `.planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-VALIDATION.md` — the draft DEBT-02 reconciles

### Superseded material (record only; mine for facts, not decisions)
- `.planning/milestones/superseded/README.md` — which findings of the old Phase 5 research are still true of the tree
- `.planning/milestones/superseded/05-guide-phase-3-cryptographic-memory-and-zero-copy-dma-paging/05-RESEARCH.md` — x86_64 exec and fault paths, FXSAVE/XSAVE, `qemu64` has no XSAVE or AVX, `fearless_simd` probe result, unsafe-site counts. Re-verify each finding before relying on it.
- `.planning/milestones/superseded/05-guide-phase-3-cryptographic-memory-and-zero-copy-dma-paging/05-CONTEXT.md` — D-13, D-30 (AVX and XSAVE by CPUID), D-16, D-38 (SIMD kernels and lint rule R3), D-34 (generated probes). Their cryptographic context is void.

### Policy and audit
- `UNSAFE-AUDIT.md` — the unsafe ledger; its x86_64 table names files that no longer exist
- `docs/threat-model.md` — amended by DEBT-05
- `.planning/legacy/WINDOWS.md` — frozen ledger; SMP-related entries are Phase 8

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `port/src/fid.rs` (939 lines): `StatefulHandle`, `StatelessCap`, `FidHandle`, 64-byte `FidEntry`, `FidShard`, `FidTable` on `FlattenedTphtShard`; the `Probe` hook that counts memory accesses in tests; `issue_cap` / `verify_cap` need no table memory and are unaffected.
- `port/src/mcslock.rs`: the MCS `Lock`, used in about 170 places; one of the three `port/` floor modules. There is no interrupt-disabling variant and no reader-writer lock.
- `ds/src/roaring.rs`, `ds/src/setops.rs` (the seam SIMD kernels go behind), `ds/src/clientset.rs`; `ds/src/tpht/` (`flattened.rs`, `chained.rs`, `feistel.rs`, `measure.rs`).
- `port/src/bitmapalloc.rs`, `port/src/allocator.rs`, `port/src/pagealloc.rs` (9 lines, an error enum only): the pools are close to new code.
- `aarch64/src/fpu.rs`, `x86_64/src/fpu.rs`: per-CPU FP ownership (`CpuFp`) already exists; `kernel_begin()` / `with_simd()`.
- `x86_64/src/platform/q35/acpi.rs`, `lapic.rs`, `ioapic.rs`: ACPI and LAPIC code exists for one CPU. `x86_64/src/dat.rs` has a `Mach` struct.
- `xtask/src/config.rs` already has a `-smp` CPU-count setting.

### Established Patterns
- About 29 files in `x86_64/src`, `aarch64/src` and `port/src` justify `unsafe` or unlocked access with "single CPU, IRQ-masked" in `// SAFETY:` comments (heaviest in both `proc.rs` files, `x86_64/src/platform/q35/pci.rs`, both `allocator.rs`, both `fpu.rs`, `aarch64/src/vm.rs`). Each is an SMP-03 audit site and an `UNSAFE-AUDIT.md` update.
- The arch crates hold about 70 `static mut` / `UnsafeCell` globals between them; `port/src` has 10. Each arch crate has its own `sched()` (`x86_64/src/proc.rs`, `aarch64/src/proc.rs`).
- `ds/src/lib.rs` puts `#[forbid(unsafe_code)]` on each module except `tpht` and one reserved Roaring SIMD module mounted at the crate root with `#[path]`.
- x86_64 has 32 undocumented `unsafe` blocks in 9 files and aarch64 has 7 (superseded research; confirm).
- x86_64 exec maps text, data and bss eagerly; aarch64 pages in on fault; both map user text writable; neither tracks dirty pages; the page-in paths panic on allocation failure; `UserSlice` holds nothing that could release a pin.

### Integration Points
- Every gate script under `gates/aarch64/` and `gates/x86_64/` hard-codes `-smp 1` (D-24 changes them).
- `.github/workflows/gates.yml` and `xtask.yml`: CI legs; `test-9p` and `cargo test -p xtask` are not in them (DEBT-01).
- `flake.nix`: `boot-iso` (CORR-02). `port/src/rfork.rs` and `sys/include/rfork.h` (CORR-04).
- `aarch64/src/usermem.rs`, `x86_64/src/usermem.rs`: `UserSlice` (D-21). `aarch64/src/fault.rs`, `x86_64/src/trap.rs`, both `exec.rs`: demand loading and page-in (D-15, D-16).
- The Aead client engine and the test server end used by `port/tests/common/aead_link.rs` are the pieces D-28 pairs inside the kernel.

### Research must confirm
- What is red in CI at `900f58b`.
- That the fence-paired seqlock of D-11 is sound in safe Rust on all three architectures, and how the `Probe` access-count test (WIRE-08) is restated for it.
- Whether the CI runners can run an AVX2 CPU model under TCG in acceptable time.
- How a dropped shared text page is unmapped from every address space that maps it, and the shootdown path for it.
- Which `port/` paths sleep while holding user memory.

</code_context>

<specifics>
## Specific Ideas

- The owner wrote the slot layout and the reference seqlock code for D-11 (`AtomicSlabSlot` with `resolve`, `publish_alloc`, `publish_clunk`; `FidShard` with an immutable slot slice and a meta mutex). It is preserved verbatim in `05-DISCUSSION-LOG.md`; treat it as the intended shape, not as text to paste.
- The owner's two edge cases for D-11: in-place update against clunk (meta mutex or CAS on `WRITING`), and the memory cost of a flat 65,536-slot shard (4 MB), which is why D-12 sizes shards at construction.
- Roaring metadata cost per shard: the slot space is `u16`, so the active and reclaimed sets each fit one container (4 bytes as a run container when contiguous, at most 8 KB).
- The pressure gate uses "a special workload designed to deterministically trigger pressure drops" (owner's words).

</specifics>

<deferred>
## Deferred Ideas

- **Geometric chunk spine for slab shards** (owner design, guide Phase 4 or later, which is roadmap Phase 6 onward): chunks of 64, 64, 128, ..., 32,768 slots (11 chunks, 65,536 slots); `split_idx` by leading-zero count; chunk 0 is one page per shard; the spine backed by a small `OnceBox` (`AtomicPtr`, Release/Acquire) in an audited module, or `spin::Once` if it is in the workspace, keeping `ds/src/slab.rs` and `port/src/fid.rs` under `#![forbid(unsafe_code)]`. Needs owner approval for the unsafe-holding module when it is taken up.
- **A backing store for dirty anonymous pages** (swap): not in any phase; D-14 leaves the seam implicit.
- **More than two CPUs, the 8-way soak, the full arm64 HVF series, WINDOWS #49, #66, #70**: Phase 8.
- **Serving `9P2000.aead` from the kernel in a default boot**: the loopback server of D-28 is gate-only; a real server end belongs with Phase 6.

</deferred>

---

*Phase: 5-Guide Phase 3 - Early SMP, Course Correction and Memory Pools*
*Context gathered: 2026-11-25*
