# Requirements: r9

**Defined:** 2026-11-22
**Core Value:** The OS boots to rio on QEMU q35 at every commit.

Source tags in parentheses point at `.planning/intel/constraints.md` entries (IG = implementation guide, AC = architecture compendium, RFC, PT = permissions and test plan) or at owner direction. "Phase N" always means the GSD phase number in ROADMAP.md; where the source documents use their own label it is written "guide Phase N". Open questions W1, W2, W4 and W7 to W10 were settled as recorded in PROJECT.md Key Decisions; the Phase 4 wire set (W1, W2, W4) closed on 2026-11-24.

Standing constraint on every requirement below: the golden invariant (boots to rio on QEMU q35) holds at every commit, and the verification discipline of IG-V01 applies (RED pin, implementation, GREEN twice, pin advance).

## Carried forward

**Carried forward:** 2026-11-25, after milestone v3.0 closed with 38 of 38 requirements
complete. The shipped set (HARN, DSGN, CRYP, DS, LINT, WIRE) is archived at
`.planning/milestones/v3.0-REQUIREMENTS.md`. This file carries the requirements for Phases 5-16 as rescoped on 2026-11-25
for Phases 5-16, the deferred v2 set, and the out-of-scope table.

**Milestone v3.1 (started 2026-11-25, rescoped the same day):** Early SMP and Cluster Fabric covers
Phase 5 (CORR-01 to CORR-05, DEBT-01 to DEBT-05, SMP-01 to SMP-04, SMP-07 to SMP-09, MEM-07, MEM-08,
MEM-10), Phase 6 (DIST-01 to DIST-12) and Phase 7 (PROC-04, RV-01, RV-02): 35 requirements.

**Rescope of 2026-11-25.** The owner's v3.0 documents (`docs/plan_r9_architecture_code_compendium_v3_0.md`,
`docs/r9_implementation_guide_for_agentic_developers_v3_0.md`) dropped software cryptographic memory,
moved `FidTable` to a sharded generational slab, pulled SMP forward and removed the Rust port of the
C compilers. Requirements retired by that rescope are listed under "Retired" below with their reason;
their IDs are not reused. Source tags "v3.0 guide" and "v3.0 compendium" point at those two documents.

## v1 Requirements

### Early SMP, Course Correction and Memory Pools (v3.0 guide Phase 3 and the Phase 0-2 delta)

Course correction (v3.0 guide section 1):

- [ ] **CORR-01**: CI on `main` is green: the workflow failing at `900f58b` is repaired before any refactor lands (v3.0 guide task 1)
- [ ] **CORR-02**: `boot-iso` in `flake.nix` is a standard pure derivation built from workspace inputs, with no fixed-output `outputHash` to maintain (v3.0 guide task 2)
- [ ] **CORR-03**: `FidTable` is a sharded generational slab in safe Rust: an 8-byte `StatefulHandle` (`slot_idx`, `gen`, `shard_id`, `flags`, `epoch`), up to 256 shards of 65,536 slots, lookup by one direct indexed read with generation and epoch checks, allocation that cannot fail on hash clustering, and Roaring active and reclaimed sets for the failover sweep. `FlattenedTphtShard` is no longer used by `FidTable`, the serialized-insert workaround of `6fd201c` is reverted, `ds/src/tpht/` stays intact for later consumers, and every `cargo xtask test-9p` leg stays green (v3.0 guide task 3, v3.0 compendium section 4; supersedes the TPHT-backed table of WIRE-07 to WIRE-09)
- [ ] **CORR-04**: `RFCKEY`, `RFROBASE` and `RFRANDMEM` are removed from `port::rfork`, `RFNOLEGACY` is `1 << 18`, bits 19 to 21 are free, and `sys/include/rfork.h` is regenerated; runtime behaviour is unchanged (v3.0 guide task 4; bit position by owner decision 2026-11-25)
- [ ] **CORR-05**: The unsafe policy is stated the same way in `docs/`, the lint contract and `UNSAFE-AUDIT.md`, and matches the tree: `crypto/` crate-wide `forbid(unsafe_code)`; `ds/` per-module forbid with `ds/src/tpht/` the audited exception; `port/` per-module forbid with the three named floor modules (`globalalloc`, `mcslock`, `devip_waker`); `undocumented_unsafe_blocks = deny` on `ds/src/tpht/` and the arch crates, with `x86_64` opted in and its sites, and the 7 open aarch64 sites, documented and audited (v3.0 guide task 5 and rule 3; the guide's "crate-wide" wording for `port/` is corrected by owner decision 2026-11-25; carries the lint half of the retired MEM-11, note N2)

v3.0 audit follow-ups:

- [ ] **DEBT-01**: `cargo xtask test-9p` and `cargo test -p xtask` run in CI and as part of `cargo xtask test` (v3.0 audit INT-03, INT-04)
- [ ] **DEBT-02**: `04-VALIDATION.md` is reconciled from `draft` to a completed validation record (v3.0 audit)
- [ ] **DEBT-03**: The Phase 4 boot-iso evidence text no longer claims the fixes moved the linked kernel bytes (v3.0 audit INT-02)
- [ ] **DEBT-04**: The TPHT home-hit property is measured at production bank sizes and the bank-sizing rule is written down before the first default-bank TPHT consumer is built (v3.0 audit INT-05; `FidTable` no longer uses TPHT after CORR-03, so the consumers are the union dcache and any page cache)
- [ ] **DEBT-05**: `docs/threat-model.md` matches the rescope: software protection of pages at rest and the stale-frame, shadow-frame and sibling-key items are removed from scope, hardware MMU isolation is stated as the memory boundary, and the `rfork` text says bits 18 to 20 were retired and `RFNOLEGACY` is bit 18 (v3.0 review WR-04, widened by the rescope)

Two-CPU SMP:

- [ ] **SMP-01**: r9 boots on QEMU q35 x86_64 with `-smp 2`: the second CPU is started by INIT-SIPI-SIPI from the ACPI MADT, both CPUs reach the scheduler loop, and the boot-to-rio gate stays green (v3.0 guide Phase 3, v3.0 compendium section 6; amended from the multi-CPU wording, whose scale-out part is SMP-10)
- [ ] **SMP-02**: Per-CPU state exists on both architectures: an x86_64 per-CPU `Mach` mapping, and aarch64 per-CPU current frame, kernel stack and proc pointers; FP ownership stays per-CPU (`CpuFp`) (HANDOFF SMP prerequisites)
- [ ] **SMP-03**: Every single-CPU spinlock invariant in the kernel is audited and replaced by SMP-safe locking, including the allocator, page tables and segment and page-cache structures (memory locking) (IG section 8, owner direction)
- [ ] **SMP-04**: aarch64 virt boots with `-smp 2` under HVF and TCG: the second CPU is started by PSCI `CPU_ON` and both reach the scheduler loop (v3.0 guide Phase 3, v3.0 compendium section 6)
- [ ] **SMP-07**: Locks taken in interrupt context use an interrupt-disabling guard (`IrqSpinLock`), and a TLB shootdown IPI (x86_64 LAPIC vector, aarch64 GIC SGI) reaches the other CPU (v3.0 compendium section 6)
- [ ] **SMP-08**: A two-CPU stress gate runs concurrent `FidTable` and `FlattenedTphtShard` workloads with timer interrupts across both CPUs and completes with zero deadlocks, as a CI gate (v3.0 guide Phase 3 exit gate)
- [ ] **SMP-09**: Every new kernel boot gate runs with `-smp 2` on QEMU q35 and virt (v3.0 guide rule 2)

Memory pools (moved here by the owner on 2026-11-25, without their cryptographic parts):

- [ ] **MEM-07**: The page allocator keeps clean, dirty and pinned Roaring pools over 256MB chunks with the address expression `(((chunk as usize) << 16) | (offset as usize)) * 4096`, full-container materialization in `fallback_difference`, pinned frames excluded from eviction, `Evictable = Dirty \ (Pinned | Active)`, and a High-Water Pager holding about 15% pre-cleared frames (IG-A01 items 8-9, IG-D10; v3.0 compendium section 3; moved to Phase 5 without its cryptographic parts by the owner on 2026-11-25; whether the set kernels get SIMD and what eviction means with no backing store are settled at phase discussion)
- [ ] **MEM-08**: `SuperpageResidency` is exactly 64 bytes and supports bit clearing and atomic counter reset on large-page demotion (IG-A01 item 10)
- [ ] **MEM-10**: `UserSlice` / `UserSliceMut` enforce residency on demand behind `UserAddressSpace`, and kernel code that holds user memory across sleep points (for example devmnt reads) pins those frames against the evictor (IG-A01 item 17, IG-V02 risk 2)

### Distributed 9P Extensions and Service Discovery (guide Phase 4)

- [ ] **DIST-01**: `Tcompound` / `Rcompound` run atomic multi-op scripts using the `CURRENT_FID` register and `NOFID_CURRENT` (0xFFFFFFFE), stop on first error, roll back where applicable, and report `num_completed` (RFC-P07)
- [ ] **DIST-02**: `Tstream` / `Rstream` use sliding-window credit via `EXT_STREAM_PRIO` (senders never exceed granted credit; window updates are empty `Rstream` frames) and tag scheduling across P0_CONTROL, P1_METADATA and P2_BULK pops the next stream in O(1) (RFC-P07, RFC-P04, IG-D10)
- [ ] **DIST-03**: Server `LeaseRecord` (64-byte, in a `FlattenedTphtShard`) tracks readers with `ClientSet`; `Tlease_revoke` goes to exactly the other sharers; the server blocks the conflicting write until `Rlease_ack` arrives or a 5-second timeout revokes unilaterally and drops the client channel; clients flush `dirty_blocks & revoked_range` (AC-P02, RFC-P07, RFC-P10)
- [ ] **DIST-04**: Clients honor volatility classes: `QTIMMUTABLE` cached indefinitely with no revalidation, `QTLEASED` cached until revoked, `QTVOLATILE` never cached, read ahead, or directory-cached (RFC-P08)
- [ ] **DIST-05**: `Treclaim` / `Rreclaim` work with an incremented epoch in `Rversion` and a 30-second grace period; restored FIDs enter `reclaimed_fids` and unreclaimed handles are freed and their keys zeroized at window close (RFC-P07)
- [ ] **DIST-06**: Union mounts (`port/src/union.rs`) keep an exact Roaring `entry_hashes` per layer, probe dynamic layers with speculative parallel `Twalk`, and commit the highest-priority hit to a 32-byte `UnionRouteEntry` (AC-P04)
- [ ] **DIST-07**: The legacy gateway negotiates `9P2000`, `9P2000.u`, `9P2000.e` and `9P2000.L` via `Tversion`, handles plaintext only in pre-pinned non-pageable bounce frames zeroized after use, and `rfork(RFNOLEGACY)` (bit 18 after CORR-04) forbids binding, mounting or walking `CHAN_UNTRUSTED_WIRE` channels; 9front interoperability verified (IG-P03, AC-P05, RFC-P09)
- [ ] **DIST-08**: Ring-bypass nodes under `#M` (`/dev/mnt/<conn>/ring`) can be mapped with `segattach(SG_SHARED, ...)` for lock-free transfer between application buffers and the NIC (AC-P01); placement in this phase is the roadmapper's judgement, as the guide's Phase 4 row does not name it
- [ ] **DIST-09**: `DiscoveryBeaconV2` is emitted by `cmd/beacon` every 5 seconds over IPv4 `239.9.9.9!9999` (TTL 2) and IPv6 `[ff02::9:9]!9999`, at the size settled by W7 (IG-P04, AC-P03)
- [ ] **DIST-10**: `cmd/discover` registers `/srv/{hostname}_{class}_{instance}` from `class:instance[+attrs]` descriptors, listens on both stacks, reads the 52-byte UDP header, and authenticates peers through Factotum (IG-P04, AC-P03)
- [ ] **DIST-11**: The staged lifecycle uses wall-clock time: Healthy 0-30s; Unhealthy 30-330s (`.status` reads `unhealthy`, `healthy_replicas` bit cleared, TCP socket kept open, any heartbeat restores Healthy); Dead beyond 330s (descriptor closed, `/srv` entries removed, peer reaped) (IG-P04, IG-D13)
- [ ] **DIST-12**: `#M` exposes the `healthy_replicas` Roaring bitmap and invalidates it on every state transition (IG-D13, IG-V01)

### Process Lifecycle, Stack-Safe Notes and riscv64 Sync (v3.0 guide Phase 5)

- [ ] **PROC-04**: Note delivery requires `SegKind::Stack` and uses checked arithmetic that keeps the frame strictly within stack bounds; forged non-stack frames are refused (IG-A01 item 16)
- [ ] **RV-01**: The riscv64 kernel takes its `rfork` flags from `port::rfork` and implements the SMP traits the other architectures implement, with no architecture-specific assumption added to `port/` (v3.0 guide Phase 5, v3.0 compendium section 1)
- [ ] **RV-02**: The riscv64 kernel builds and passes a boot smoke on QEMU (v3.0 guide Phase 5 exit gate)

### SMP Scale-Out and Deferred Verification (Phase 8)

- [ ] **SMP-05**: The full arm64 HVF series (smoke 1753, net-smoke 36, tier-1 10, cpu 36, plus 3 TCG each) runs and is green; the owner deferred it to this milestone (HANDOFF)
- [ ] **SMP-06**: WINDOWS #70 (aarch64 scheduler resume skips note delivery) and #66 (x86_64 triple fault under 8-way soak) are fixed and #49 (I-cache) is closed after the full series, with the open-entry ledger re-homed to a live file because the original is frozen under `.planning/legacy/` (HANDOFF)
- [ ] **SMP-10**: r9 boots on QEMU q35 x86_64 with more than two CPUs, all online and scheduling, reaches rio repeatedly, and shows zero contention deadlocks under an 8-way soak (the scale-out part of the original SMP-01)

### Native Standard Library (Milestone STD)

- [ ] **STD-01**: A Rust program built for an r9 userland target is converted to `a.out` by `xtask/src/elf2aout.rs` and loaded by the kernel with no ELF loader (IG-D02, IG-D04)
- [ ] **STD-02**: `std::fs` works over 9P: open, read, write, seek, stat, remove, `ReadDir` via `dirread(2)`, permission bits `0o777`, `DMDIR`, `DMAPPEND`, `DMEXCL` (IG-L01)
- [ ] **STD-03**: `std::net` works over `/net/tcp` and `/net/udp` (clone, ctl, data, listen; UDP `headers` 52-byte frames) with IPv6 `[fe80::1]!564` and IPv4 `10.0.0.1!564` dial forms (IG-L01)
- [ ] **STD-04**: `std::thread` and `std::sync` work: `rfork(RFPROC | RFMEM | RFFDG)` threads with `segattach` stacks and guard pages, TLS via `%gs` / `FS_BASE`, uncontended CAS in user space and contended sleep on `rendezvous` (IG-L01)
- [ ] **STD-05**: `std::process`, `std::env`, `std::io` and stdio work: `rfork` plus `exec(a.out)`, exit status from `#c/wait` or `/proc/<pid>/wait`, `pipe(2)` stdio, `/env/<name>` 1:1, fd 0, 1, 2 (IG-L01)
- [ ] **STD-06**: `std::alloc` and `std::time` work: `sbrk(2)` heap with large buffers via `segattach(2)`, `Instant` from calibrated `RDTSC`, `SystemTime` from `#c/bintime` (IG-L01)
- [ ] **STD-07**: A `std` test suite passes on r9 under multi-CPU, and idiomatic Rust binaries run without `#![no_std]` (IG section 8 gate)

### Track P Stage P1: Kernel Devices

- [ ] **KDEV-01**: A device-parity inventory comparing Harvey's devtab with r9 `port/src` is committed for `#c`, `#d`, `#e`, `#s`, `#|`, `#I`, `#p`, with every gap closed (several already exist in Rust at HEAD)
- [ ] **KDEV-02**: `#c`, `#d`, `#e`, `#s` and `#|` expose the files and semantics Harvey userland relies on (including `rendezvous`, `bintime`, `wait`), shown by Harvey tier sweeps staying green
- [ ] **KDEV-03**: `#I` exposes the `/net` conversation interface (clone, ctl, data, listen, UDP headers) as `libstd_r9` and Harvey userland use it
- [ ] **KDEV-04**: `#p` exposes the `/proc/<pid>` files (ctl, status, note, wait) that process tools use
- [ ] **KDEV-05**: These seven devices have no `unsafe` in device logic (module-level `forbid(unsafe_code)`), with any floor exceptions documented

### Track P Stage P2: Base CLI

- [ ] **CLI-01**: `rc` runs as a Rust binary and executes Harvey's boot and rc scripts unmodified (IG-D05)
- [ ] **CLI-02**: `bind` runs as a Rust binary with Harvey namespace semantics
- [ ] **CLI-03**: `mount` runs as a Rust binary with Harvey semantics
- [ ] **CLI-04**: `cat` runs as a Rust binary
- [ ] **CLI-05**: `echo` runs as a Rust binary
- [ ] **CLI-06**: `ls` runs as a Rust binary
- [ ] **CLI-07**: Each ported tool is removed from the xtask Harvey fallback manifest in the same commit that swaps it in, and the golden gate is green at every such commit (owner direction; HARN-04)

### Track P Stage P3: Complex Userland

- [ ] **USER-01**: `sam` runs as a Rust binary
- [ ] **USER-02**: `rio` runs as a Rust binary and the golden gate reaches a rio session on q35 with it
- [ ] **USER-03**: `upas` (mail send, filesystem and transport pieces needed by the system) runs as Rust binaries
- [ ] **USER-05**: `auth/factotum` runs as a Rust binary (AC places it in Track P; Phase 4 depends on factotum, note N5)
- [ ] **USER-06**: All remaining Harvey userland commands and the 35 libraries are ported or explicitly retired with a recorded reason, and the fallback manifest holds no C userland binary or library object except the named bootstrap set (Harvey's `6c`, `6a`, `6l` and classic APE); the system boots to rio (Track P gate, amended 2026-11-25 by the v3.0 documents)

### Storage Engine (guide Phase 6)

- [ ] **STOR-01**: Venti scores are `BLAKE3(plaintext)` over uncompressed blocks (hash-before-compression), using parallel BLAKE3 Merkle digests, so dedup and Merkle roots stay stable across compression and re-encoding (IG-S01)
- [ ] **STOR-02**: `VentiTphtIndex` is a Chained-TPHT with a 16MB head array and wait-free lookups; a 64-bit folded key is only a hint and the full 32-byte BLAKE3 score is verified on candidate match (IG-S01, IG-A01 item 12)
- [ ] **STOR-03**: On-disk layout uses 64KB data super-blocks (16 contiguous 4KB pages, aligned to flash erase boundaries) and 1KB tag slabs of sixteen 64-byte `TagEntry` records with bounds-checked `TagTable`; the `nonce` and `_reserved` fields (AC superset) are decided at planning (IG-S02)
- [ ] **STOR-04**: Compression is two-tier Zstd: level 3 in flight and in the background; `COMPRESS_NONE` when `compressed_size >= original_size`; decompression scratch sized from `original_size`; re-encoding to Zstd 9-15 only during consolidation of arenas below 40% occupancy (IG-S03)
- [ ] **STOR-05**: Fossil commits send surviving BLAKE3 roots to Venti with daily (Son) and weekly (Father) snapshots retained at level 3 (IG-S04, IG-S03)
- [ ] **STOR-06**: Mark-and-sweep GC computes `ReferencedBlocks` as a Roaring bitmap and `PurgeSet = ArenaPresence \ ReferencedBlocks`, and issues NVMe Dataset Management (TRIM) for contiguous purged runs (IG-S04)
- [ ] **STOR-07**: Venti block deduplication is green: identical content written twice is stored once and reads back by score (IG section 8 gate)

### Cluster Permission Suites (PT)

- [ ] **PERM-01**: A minimega testbed (12 nodes, VLANs 10, 20, 30) and the persona and `/adm/users` fixtures exist, with fixture defects W8, W9 and W10 settled (PT-C01, PT-C02)
- [ ] **PERM-02**: Suite 1 (discretionary file sharing: asymmetric pair sharing, symmetric team collaboration, group leadership) passes (PT-T01)
- [ ] **PERM-03**: Suite 2 (concurrent read leases with inline `ClientSet` and zero network packets on repeat reads, `Tlease_revoke` flow, anti-splicing and replay on the wire) passes (PT-T02)
- [ ] **PERM-04**: Suite 3 (multi-domain federation: two-key factotum negotiation with distinct transient keys, and unregistered cross-domain attenuation to UID 9999) passes (PT-T03)
- [ ] **PERM-05**: Suite 5 (mail and chat visibility: direct message isolation and dual visibility) passes, once `chat-rs` is defined or this suite is deferred (note N6) (PT-T05)

### Graphics (guide Phase 7)

- [ ] **GFX-01**: A 2D framebuffer and kernel `devdraw` let rio draw natively on q35 without a drawterm client (IG section 8)
- [ ] **GFX-02**: VirtIO-GPU negotiates feature bits 0, 3, 4 (`VIRGL`, `BLOB`, `CONTEXT_INIT`) and selects Venus capset 4 (IG-A01 item 14, AC-G01)
- [ ] **GFX-03**: `#G` exposes `/dev/gpu/clone`, `<id>/ctl`, `cmd`, `blob`, `fence`, with the context id packed into the QID path and every VirtIO packet header built and validated by the kernel (IG-D12, IG-A01 item 15)
- [ ] **GFX-04**: The `draw(3)` bridge binds a window to a host resource (`Image.gpu_res_id`) and composites in host VRAM via host blit when both images have resource ids, falling back to CPU memdraw otherwise (AC-G01)
- [ ] **GFX-05**: A hardware-rendered triangle appears in a rio window on QEMU (IG section 8 gate)

### Self-Hosting Toolchain (guide Phase 8)

- [ ] **SELF-01**: `rustc` runs natively on r9, with its `memmap2` metadata loader falling back to whole-file read buffers so no new kernel mapping call is added (IG-T01)
- [ ] **SELF-02**: `rust-lld` runs natively on r9 and its static ELF output is wrapped into `a.out` by the in-tree conversion tool (IG-T01, IG-D02)
- [ ] **SELF-03**: APEX (Harvey-OS/apex, MIT, musl-based, no `mmap`) is vendored at `vendor/apex/` with syscall stubs generated from `sys/src/libc/9syscall/sys.h` (IG-D06)
- [ ] **SELF-04**: Modern C++/LLVM tooling builds via Clang against `vendor/apex/` and runs on r9 (IG-D06, IG-T01)
- [ ] **SELF-05**: Harvey's `6c`/`6a`/`6l` and classic APE run on r9 as carried bootstrap and compatibility binaries and still build native Plan 9 C software; they are not rewritten in Rust, and the two C runtimes are never mixed in one binary (IG-D06; amended 2026-11-25 by the v3.0 compendium section 7)
- [ ] **SELF-06**: r9 compiles its own kernel and userland on r9, and the result boots to rio (IG section 8 gate)

### Harvey Retirement (Final)

- [ ] **RETIRE-01**: Every device in the devtab is safe Rust, confirmed by an inventory scan covering devices beyond the seven in Stage P1 (owner deprecation checklist)
- [ ] **RETIRE-02**: The root image contains no Harvey C binary other than the named bootstrap set (`6c`, `6a`, `6l` and classic APE), confirmed by the fallback manifest and an image scan (owner deprecation checklist, amended 2026-11-25)
- [ ] **RETIRE-03**: With the Harvey tree absent (not merely unused), `rustc` and `rust-lld` run natively, modern C++/LLVM builds via Clang against `vendor/apex/`, and the bootstrap set r9 carries still builds native Plan 9 C software (owner deprecation checklist, amended 2026-11-25)
- [ ] **RETIRE-04**: Every composed gate previously hosted in Harvey's flake (boot-r9-x64 smokes, net smoke, auth, drawterm, rio) runs from the r9 repo without Harvey, and the golden-invariant gate is green (derived; the gates themselves move to r9's flake in Phase 1 under HARN-08, so what remains here is running them with no Harvey tree at all; note N4)
- [ ] **RETIRE-05**: `vendor/harvey/` is removed, the Harvey repository is archived read-only, and the r9 README carries the banner (owner deprecation checklist)


## Retired by the 2026-11-25 rescope

Not built. IDs are kept so that older documents still resolve.

| Requirement | What it was | Why retired |
|-------------|-------------|-------------|
| MEM-01, MEM-02, MEM-03, MEM-04 | `UninitializedFrame` and `ShadowFrame`, Encrypt-then-MAC page sealing, dual-path stores, the `ymm0`-`ymm5` vector path | Software cryptographic memory dropped (v3.0 compendium section 1) |
| MEM-05, MEM-06 | `StrideTracker` crypto prefetch, KeyID frame recycling and `CLFLUSHOPT` scrub | Same |
| MEM-09 | `9P2000.aead` pages placed in frames undecrypted, decrypted at fault | Frames are verified and decrypted on receive (v3.0 compendium section 5) |
| MEM-11 | `q35lowmem` and `q35crypt` profiles; x86_64 lint opt-in | Profiles dropped with sealed paging; the lint opt-in is carried by CORR-05 |
| MEM-12, MEM-13 | In-flight fault de-duplication, cold-fault benchmark | No fault-time decrypt to de-duplicate or measure |
| PROC-01, PROC-02, PROC-03 | `RFCKEY`, `RFROBASE`, `RFRANDMEM` live | Flags removed (CORR-04) |
| PROC-05 | PT Suite 4, crypto sibling isolation | Depends on the removed flags |
| USER-04 | `6c`, `6a`, `6l` as Rust binaries | No Rust rewrite of the C compilers (v3.0 guide rule 4); they stay as bootstrap binaries |

## v2 Requirements

Deferred. Tracked but not in the current roadmap.

### Discovery and Wire Extensions

- **SITE-01**: IPv6 site-local discovery group `[ff05::9:9]!9999` for routed datacenter fabrics (AC-only extension; INGEST-CONFLICTS I2)
- **OBIN-01**: Producers of `EXT_OBINARY` aligned binary structures from `/proc`, `#G`, `/net` and `telescope` (the wire bit itself is parsed in WIRE-02)

### Storage

- **TELE-01**: `telescope`, batch-built immutable full-text indexes over Venti snapshots using Elastic Hashing (IG-M08)


## Out of Scope

| Feature | Reason |
|---------|--------|
| POSIX emulation, `mmap` extensions, any new syscall | The system call list is closed (IG-D03) |
| Kernel ELF loading | `a.out` is the sole executable format (IG-D02) |
| Physical DRAM bus probing, cold boot, kernel-context code execution | Outside the threat model (AC-T01); SEV-SNP or TDX is the remedy |
| Funnel Hashing on primary paths | Replaced by TPHT; scratchpad only (IG-D09) |
| Mixing APE and APEX in one binary | Dual C runtime isolation (IG-D06) |
| Writing Harvey userland in `#![no_std]` first | Double rewrite that Milestone STD before Track P avoids (IG section 9.1) |
| Copying AC appendix listings verbatim | Non-normative; several contradict the guide (INGEST-CONFLICTS I5) |
| Raspberry Pi hardware | No phase targets it (riscv64 on QEMU is back in scope, Phase 7) |
| Software RAM encryption, `PKEY`/`WRPKRU` page-fault hooks, `rfork` crypto flags | Dropped by the v3.0 documents; hardware MMU isolation only; a deviation needs a recorded owner decision first |
| Rust rewrites of `6c`/`6a`/`6l` and of classic APE | Harvey's C compilers and APE stay as bootstrap and compatibility binaries (v3.0 documents) |
| Editing the Harvey tree | Read-only reference; the Phase 2 flag reservation is r9-only (owner decision 2026-11-22) |


## Traceability

Which phases cover which requirements. Each requirement maps to exactly one phase.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CORR-01 | Phase 5 | Pending |
| CORR-02 | Phase 5 | Pending |
| CORR-03 | Phase 5 | Pending |
| CORR-04 | Phase 5 | Pending |
| CORR-05 | Phase 5 | Pending |
| DEBT-01 | Phase 5 | Pending |
| DEBT-02 | Phase 5 | Pending |
| DEBT-03 | Phase 5 | Pending |
| DEBT-04 | Phase 5 | Pending |
| DEBT-05 | Phase 5 | Pending |
| SMP-01 | Phase 5 | Pending |
| SMP-02 | Phase 5 | Pending |
| SMP-03 | Phase 5 | Pending |
| SMP-04 | Phase 5 | Pending |
| SMP-07 | Phase 5 | Pending |
| SMP-08 | Phase 5 | Pending |
| SMP-09 | Phase 5 | Pending |
| MEM-07 | Phase 5 | Pending |
| MEM-08 | Phase 5 | Pending |
| MEM-10 | Phase 5 | Pending |
| DIST-01 | Phase 6 | Pending |
| DIST-02 | Phase 6 | Pending |
| DIST-03 | Phase 6 | Pending |
| DIST-04 | Phase 6 | Pending |
| DIST-05 | Phase 6 | Pending |
| DIST-06 | Phase 6 | Pending |
| DIST-07 | Phase 6 | Pending |
| DIST-08 | Phase 6 | Pending |
| DIST-09 | Phase 6 | Pending |
| DIST-10 | Phase 6 | Pending |
| DIST-11 | Phase 6 | Pending |
| DIST-12 | Phase 6 | Pending |
| PROC-04 | Phase 7 | Pending |
| RV-01 | Phase 7 | Pending |
| RV-02 | Phase 7 | Pending |
| SMP-05 | Phase 8 | Pending |
| SMP-06 | Phase 8 | Pending |
| SMP-10 | Phase 8 | Pending |
| STD-01 | Phase 9 | Pending |
| STD-02 | Phase 9 | Pending |
| STD-03 | Phase 9 | Pending |
| STD-04 | Phase 9 | Pending |
| STD-05 | Phase 9 | Pending |
| STD-06 | Phase 9 | Pending |
| STD-07 | Phase 9 | Pending |
| KDEV-01 | Phase 10 | Pending |
| KDEV-02 | Phase 10 | Pending |
| KDEV-03 | Phase 10 | Pending |
| KDEV-04 | Phase 10 | Pending |
| KDEV-05 | Phase 10 | Pending |
| CLI-01 | Phase 11 | Pending |
| CLI-02 | Phase 11 | Pending |
| CLI-03 | Phase 11 | Pending |
| CLI-04 | Phase 11 | Pending |
| CLI-05 | Phase 11 | Pending |
| CLI-06 | Phase 11 | Pending |
| CLI-07 | Phase 11 | Pending |
| USER-01 | Phase 12 | Pending |
| USER-02 | Phase 12 | Pending |
| USER-03 | Phase 12 | Pending |
| USER-05 | Phase 12 | Pending |
| USER-06 | Phase 12 | Pending |
| STOR-01 | Phase 13 | Pending |
| STOR-02 | Phase 13 | Pending |
| STOR-03 | Phase 13 | Pending |
| STOR-04 | Phase 13 | Pending |
| STOR-05 | Phase 13 | Pending |
| STOR-06 | Phase 13 | Pending |
| STOR-07 | Phase 13 | Pending |
| PERM-01 | Phase 13 | Pending |
| PERM-02 | Phase 13 | Pending |
| PERM-03 | Phase 13 | Pending |
| PERM-04 | Phase 13 | Pending |
| PERM-05 | Phase 13 | Pending |
| GFX-01 | Phase 14 | Pending |
| GFX-02 | Phase 14 | Pending |
| GFX-03 | Phase 14 | Pending |
| GFX-04 | Phase 14 | Pending |
| GFX-05 | Phase 14 | Pending |
| SELF-01 | Phase 15 | Pending |
| SELF-02 | Phase 15 | Pending |
| SELF-03 | Phase 15 | Pending |
| SELF-04 | Phase 15 | Pending |
| SELF-05 | Phase 15 | Pending |
| SELF-06 | Phase 15 | Pending |
| RETIRE-01 | Phase 16 | Pending |
| RETIRE-02 | Phase 16 | Pending |
| RETIRE-03 | Phase 16 | Pending |
| RETIRE-04 | Phase 16 | Pending |
| RETIRE-05 | Phase 16 | Pending |
