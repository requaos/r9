# Requirements: r9

**Defined:** 2026-11-22
**Core Value:** The OS boots to rio on QEMU q35 at every commit.

Source tags in parentheses point at `.planning/intel/constraints.md` entries (IG = implementation guide, AC = architecture compendium, RFC, PT = permissions and test plan) or at owner direction. "Phase N" always means the GSD phase number in ROADMAP.md; where the source documents use their own label it is written "guide Phase N". Open questions W1, W2, W4 and W7 to W10 were settled as recorded in PROJECT.md Key Decisions; the Phase 4 wire set (W1, W2, W4) closed on 2026-11-24.

Standing constraint on every requirement below: the golden invariant (boots to rio on QEMU q35) holds at every commit, and the verification discipline of IG-V01 applies (RED pin, implementation, GREEN twice, pin advance).

## Carried forward

**Carried forward:** 2026-11-25, after milestone v3.0 closed with 38 of 38 requirements
complete. The shipped set (HARN, DSGN, CRYP, DS, LINT, WIRE) is archived at
`.planning/milestones/v3.0-REQUIREMENTS.md`. This file carries the 89 remaining requirements (94 with the five DEBT follow-ups added for v3.1)
for Phases 5-16, the deferred v2 set, and the out-of-scope table.

**Milestone v3.1 (started 2026-11-25):** Secure Memory and Cluster Fabric covers MEM-01 to MEM-13,
DIST-01 to DIST-12, PROC-01 to PROC-05 and DEBT-01 to DEBT-05 (35 requirements). Seven MEM
requirements were amended on 2026-11-25 from the Phase 5 discussion; the decisions are in
`.planning/phases/05-guide-phase-3-cryptographic-memory-and-zero-copy-dma-paging/05-CONTEXT.md`.

## v1 Requirements

### Cryptographic Memory and Zero-Copy DMA Paging (guide Phase 3)

- [ ] **MEM-01**: `UninitializedFrame` is popped from the free pools with its PTE unmapped (`PROT_NONE`); a frame reused within the same KeyID is not scrubbed, and a frame reused across key domains is zeroed and flushed first (MEM-06). ChaCha20 overwrites all 4,096 bytes, and only after BLAKE3 verification is it wrapped as an immutable `ShadowFrame` and published (IG-M04; amended 2026-11-25 by Phase 5 CONTEXT D-11)
- [ ] **MEM-02**: Page sealing is Encrypt-then-MAC with dedicated MAC keys: the MAC is computed over the ciphertext, and the frame is published only after it verifies; on failure the frame is zeroed and the process is killed. No plaintext is read back from DRAM to compute the MAC (IG-A01 item 5; amended 2026-11-25 by Phase 5 CONTEXT D-15)
- [ ] **MEM-03**: On `#PF` decrypt the 64-byte faulting line uses cached stores and the remaining 4,032 bytes use non-temporal stores, with `SFENCE` issued after the loop and overlapped with the scalar tree-MAC root and register restore (IG-M02)
- [ ] **MEM-04**: The vector fast path in `x86_64/src/cryptpage.rs` uses only `ymm0`-`ymm5` in one `global_asm!` routine. It runs in a restricted `fpu` section that spills those six registers to the kernel stack, scrubs them with `VPXOR` and restores them (about 30 cycles); no `XSAVE`/`XRSTOR` runs on the fault path or the eviction seal path. AVX-512 is disabled in XCR0. Scalar and vector paths produce identical output. Enabling AVX moves per-process FP state from FXSAVE to XSAVE at process switch only, using `XSAVEOPT` where available. aarch64 has a scalar path and a NEON path under its own restricted section (IG-M03, IG-A01 items 3-4, RFC-P10; note N7 settled; amended 2026-11-25 by Phase 5 CONTEXT D-12, D-13, D-23)
- [ ] **MEM-05**: `StrideTracker` evaluates a chronological stride ring buffer comparing oldest, middle and newest entries (IG-A01 item 6)
- [ ] **MEM-06**: Frame recycling skips zero and flush for same-KeyID reuse (`recycle_within`). Cross-domain reassignment zeroes the frame and then flushes it with `CLFLUSHOPT`, gated on CPUID leaf 7 EBX bit 23. A key domain is one process, with shared text under a per-image read-only key (IG-A01 item 7, RFC section 11; note N9 settled; amended 2026-11-25 by Phase 5 CONTEXT D-10, D-11)
- [ ] **MEM-07**: The page allocator keeps clean, dirty and pinned Roaring pools over 256MB chunks with the address expression `(((chunk as usize) << 16) | (offset as usize)) * 4096`, full-container materialization in `fallback_difference`, pinned frames excluded from eviction, `Evictable = Dirty \ (Pinned | Active)`, and a High-Water Pager holding about 15% pre-cleared frames; the Roaring set kernels have AVX2 and NEON implementations behind the `setops` seam, reached through `with_simd()` (IG-A01 items 8-9, IG-D10, IG-V02; amended 2026-11-25 by Phase 5 CONTEXT D-16)
- [ ] **MEM-08**: `SuperpageResidency` is exactly 64 bytes and supports bit clearing and atomic counter reset on large-page demotion (IG-A01 item 10)
- [ ] **MEM-09**: A `9P2000.aead` data page is read from the transport straight into an `UninitializedFrame` in the destination segment's `BackingStore`, with no intermediate kernel buffer and no decrypt on receive. A keyed BLAKE3 page tag is computed at receipt, and the demand fault verifies it and decrypts correctly with one frame per page. Copies inside the TCP stack remain and are stated as the limit. The path serves exec image reads and page-aligned whole-page `read(2)` (IG section 8 gate, AC-P01; amended 2026-11-25 by Phase 5 CONTEXT D-17, D-19, D-21)
- [ ] **MEM-10**: `UserSlice` / `UserSliceMut` enforce residency on demand behind `UserAddressSpace`, and kernel code that holds user memory across sleep points (for example devmnt reads) pins those frames against the evictor (IG-A01 item 17, IG-V02 risk 2)
- [ ] **MEM-11**: Boot profiles `q35lowmem` (forces eviction) and `q35crypt` (tamper kill and vector-register preservation probes), with aarch64 counterparts, exist in `cargo xtask boot-test` and pass. Sealed paging runs only in these profiles. `x86_64` is lint-opted-in as a whole crate and `UNSAFE-AUDIT.md` reconciles every site, including the new pager and vector modules on both architectures (IG-V01, note N2; amended 2026-11-25 by Phase 5 CONTEXT D-08, D-23, D-25)
- [ ] **MEM-12**: Concurrent faults on shared Block A code are de-duplicated through a write-once Funnel scratchpad (`port/src/inflight.rs`) (IG-M08)
- [ ] **MEM-13**: A KVM benchmark records cycles per cold fault (scalar versus vector) and eviction cost, against the guide's expectation of about 4.4 microseconds per cold fault (IG-V01, IG-V02 risk 1)

### Distributed 9P Extensions and Service Discovery (guide Phase 4)

- [ ] **DIST-01**: `Tcompound` / `Rcompound` run atomic multi-op scripts using the `CURRENT_FID` register and `NOFID_CURRENT` (0xFFFFFFFE), stop on first error, roll back where applicable, and report `num_completed` (RFC-P07)
- [ ] **DIST-02**: `Tstream` / `Rstream` use sliding-window credit via `EXT_STREAM_PRIO` (senders never exceed granted credit; window updates are empty `Rstream` frames) and tag scheduling across P0_CONTROL, P1_METADATA and P2_BULK pops the next stream in O(1) (RFC-P07, RFC-P04, IG-D10)
- [ ] **DIST-03**: Server `LeaseRecord` (64-byte, in a `FlattenedTphtShard`) tracks readers with `ClientSet`; `Tlease_revoke` goes to exactly the other sharers; the server blocks the conflicting write until `Rlease_ack` arrives or a 5-second timeout revokes unilaterally and drops the client channel; clients flush `dirty_blocks & revoked_range` (AC-P02, RFC-P07, RFC-P10)
- [ ] **DIST-04**: Clients honor volatility classes: `QTIMMUTABLE` cached indefinitely with no revalidation, `QTLEASED` cached until revoked, `QTVOLATILE` never cached, read ahead, or directory-cached (RFC-P08)
- [ ] **DIST-05**: `Treclaim` / `Rreclaim` work with an incremented epoch in `Rversion` and a 30-second grace period; restored FIDs enter `reclaimed_fids` and unreclaimed handles are freed and their keys zeroized at window close (RFC-P07)
- [ ] **DIST-06**: Union mounts (`port/src/union.rs`) keep an exact Roaring `entry_hashes` per layer, probe dynamic layers with speculative parallel `Twalk`, and commit the highest-priority hit to a 32-byte `UnionRouteEntry` (AC-P04)
- [ ] **DIST-07**: The legacy gateway negotiates `9P2000`, `9P2000.u`, `9P2000.e` and `9P2000.L` via `Tversion`, handles plaintext only in pre-pinned non-pageable bounce frames zeroized after use, and `rfork(RFNOLEGACY)` (bit 21, reserved in Phase 2) forbids binding, mounting or walking `CHAN_UNTRUSTED_WIRE` channels; 9front interoperability verified (IG-P03, AC-P05, RFC-P09)
- [ ] **DIST-08**: Ring-bypass nodes under `#M` (`/dev/mnt/<conn>/ring`) can be mapped with `segattach(SG_SHARED, ...)` for lock-free transfer between application buffers and the NIC (AC-P01); placement in this phase is the roadmapper's judgement, as the guide's Phase 4 row does not name it
- [ ] **DIST-09**: `DiscoveryBeaconV2` is emitted by `cmd/beacon` every 5 seconds over IPv4 `239.9.9.9!9999` (TTL 2) and IPv6 `[ff02::9:9]!9999`, at the size settled by W7 (IG-P04, AC-P03)
- [ ] **DIST-10**: `cmd/discover` registers `/srv/{hostname}_{class}_{instance}` from `class:instance[+attrs]` descriptors, listens on both stacks, reads the 52-byte UDP header, and authenticates peers through Factotum (IG-P04, AC-P03)
- [ ] **DIST-11**: The staged lifecycle uses wall-clock time: Healthy 0-30s; Unhealthy 30-330s (`.status` reads `unhealthy`, `healthy_replicas` bit cleared, TCP socket kept open, any heartbeat restores Healthy); Dead beyond 330s (descriptor closed, `/srv` entries removed, peer reaped) (IG-P04, IG-D13)
- [ ] **DIST-12**: `#M` exposes the `healthy_replicas` Roaring bitmap and invalidates it on every state transition (IG-D13, IG-V01)

### Process Lifecycle and Notes (guide Phase 5)

- [ ] **PROC-01**: `rfork(RFCKEY)` is live: private segments derive isolated keys and each sibling runs with an independent CR3 (IG-D08, IG-A01 item 2)
- [ ] **PROC-02**: `rfork(RFROBASE)` is live: after it, a write into Block A (`SG_TEXT`) traps via PKEY 1 / `WRPKRU` in about 20 cycles without touching DRAM (IG-D08, PT-T04 test 4.2)
- [ ] **PROC-03**: `rfork(RFRANDMEM)` is live with semantics specified at planning time (note N8) (IG section 8)
- [ ] **PROC-04**: Note delivery requires `SegKind::Stack` and uses checked arithmetic that keeps the frame strictly within stack bounds; forged non-stack frames are refused (IG-A01 item 16)
- [ ] **PROC-05**: PT Suite 4 passes: a sibling dereferencing another sibling's private heap takes an MMU trap, direct-frame reads fail the BLAKE3 MAC check, and the kernel kills it with the note `sys: crypto integrity violation`; `q35crypt` covers sibling isolation and note spoofing (PT-T04, IG-V01)

### v3.0 Audit Follow-ups (v3.1 backlog)

- [ ] **DEBT-01**: `cargo xtask test-9p` and `cargo test -p xtask` run in CI and as part of `cargo xtask test` (v3.0 audit INT-03, INT-04)
- [ ] **DEBT-02**: `04-VALIDATION.md` is reconciled from `draft` to a completed validation record (v3.0 audit)
- [ ] **DEBT-03**: The Phase 4 boot-iso evidence text no longer claims the fixes moved the linked kernel bytes (v3.0 audit INT-02)
- [ ] **DEBT-04**: The TPHT home-hit property is measured at production bank sizes, and the bank-sizing precondition is settled before any default-bank `FidShard` exists (v3.0 audit INT-05)
- [ ] **DEBT-05**: `docs/threat-model.md` states that `rfork` bits 18-21 are reserved (v3.0 review WR-04)

### Multi-CPU SMP (Milestone S)

- [ ] **SMP-01**: r9 boots on QEMU q35 x86_64 with multiple CPUs, all online and scheduling, reaches rio repeatedly, and shows zero contention deadlocks under soak (IG section 8 gate)
- [ ] **SMP-02**: Per-CPU state exists on both architectures: an x86_64 per-CPU `Mach` mapping, and aarch64 per-CPU current frame, kernel stack and proc pointers; FP ownership stays per-CPU (`CpuFp`) (HANDOFF SMP prerequisites)
- [ ] **SMP-03**: Every single-CPU spinlock invariant in the kernel is audited and replaced by SMP-safe locking, including the allocator, page tables and segment and page-cache structures (memory locking) (IG section 8, owner direction)
- [ ] **SMP-04**: aarch64 virt boots multi-CPU under HVF and TCG
- [ ] **SMP-05**: The full arm64 HVF series (smoke 1753, net-smoke 36, tier-1 10, cpu 36, plus 3 TCG each) runs and is green; the owner deferred it to this milestone (HANDOFF)
- [ ] **SMP-06**: WINDOWS #70 (aarch64 scheduler resume skips note delivery) and #66 (x86_64 triple fault under 8-way soak) are fixed and #49 (I-cache) is closed after the full series, with the open-entry ledger re-homed to a live file because the original is frozen under `.planning/legacy/` (HANDOFF)

### Native Standard Library (Milestone STD)

- [ ] **STD-01**: A Rust program built for an r9 userland target is converted to `a.out` by `xtask/src/elf2aout.rs` and loaded by the kernel with no ELF loader (IG-D02, IG-D04)
- [ ] **STD-02**: `std::fs` works over 9P: open, read, write, seek, stat, remove, `ReadDir` via `dirread(2)`, permission bits `0o777`, `DMDIR`, `DMAPPEND`, `DMEXCL` (IG-L01)
- [ ] **STD-03**: `std::net` works over `/net/tcp` and `/net/udp` (clone, ctl, data, listen; UDP `headers` 52-byte frames) with IPv6 `[fe80::1]!564` and IPv4 `10.0.0.1!564` dial forms (IG-L01)
- [ ] **STD-04**: `std::thread` and `std::sync` work: `rfork(RFPROC | RFMEM | RFFDG)` threads with `segattach` stacks and guard pages, TLS via `%gs` / `FS_BASE`, uncontended CAS in user space and contended sleep on `rendezvous` (IG-L01)
- [ ] **STD-05**: `std::process`, `std::env`, `std::io` and stdio work: `rfork` plus `exec(a.out)`, exit status from `#c/wait` or `/proc/<pid>/wait`, `pipe(2)` stdio, `/env/<name>` 1:1, fd 0, 1, 2 (IG-L01)
- [ ] **STD-06**: `std::alloc` and `std::time` work: `sbrk(2)` heap with large buffers via `segattach(2)` under `RFCKEY`, `Instant` from calibrated `RDTSC`, `SystemTime` from `#c/bintime` (IG-L01)
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

### Track P Stage P3: Complex Userland and Compilers

- [ ] **USER-01**: `sam` runs as a Rust binary
- [ ] **USER-02**: `rio` runs as a Rust binary and the golden gate reaches a rio session on q35 with it
- [ ] **USER-03**: `upas` (mail send, filesystem and transport pieces needed by the system) runs as Rust binaries
- [ ] **USER-04**: `6c`, `6a` and `6l` run as Rust binaries and demonstrate parity with Harvey's compilers on a defined reference corpus (method set at planning); until proven, Harvey's compilers stay in the fallback manifest (IG-D05)
- [ ] **USER-05**: `auth/factotum` runs as a Rust binary (AC places it in Track P; Phase 4 depends on factotum, note N5)
- [ ] **USER-06**: All remaining Harvey userland commands and the 35 libraries are ported or explicitly retired with a recorded reason, and the fallback manifest holds no C userland binary or library object; the system boots to rio with zero Harvey C binaries (Track P gate)

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
- [ ] **SELF-05**: Native Plan 9 C software builds with the ported `6c`/`6a`/`6l` over classic APE (`sys/src/ape`), Ghostscript as the demonstration, and the two C runtimes are never mixed in one binary (IG-D06)
- [ ] **SELF-06**: r9 compiles its own kernel and userland on r9, and the result boots to rio (IG section 8 gate)

### Harvey Retirement (Final)

- [ ] **RETIRE-01**: Every device in the devtab is safe Rust, confirmed by an inventory scan covering devices beyond the seven in Stage P1 (owner deprecation checklist)
- [ ] **RETIRE-02**: The root image contains zero Harvey C binaries, confirmed by an empty fallback manifest and an image scan (owner deprecation checklist)
- [ ] **RETIRE-03**: With the Harvey tree absent (not merely unused), native Plan 9 C software builds with ported 6c/6l over classic APE, modern C++/LLVM builds via Clang against `vendor/apex/`, and `rustc` and `rust-lld` run natively (owner deprecation checklist)
- [ ] **RETIRE-04**: Every composed gate previously hosted in Harvey's flake (boot-r9-x64 smokes, net smoke, auth, drawterm, rio) runs from the r9 repo without Harvey, and the golden-invariant gate is green (derived; the gates themselves move to r9's flake in Phase 1 under HARN-08, so what remains here is running them with no Harvey tree at all; note N4)
- [ ] **RETIRE-05**: `vendor/harvey/` is removed, the Harvey repository is archived read-only, and the r9 README carries the banner (owner deprecation checklist)


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
| riscv64 and Raspberry Pi hardware | No phase targets them |
| Editing the Harvey tree | Read-only reference; the Phase 2 flag reservation is r9-only (owner decision 2026-11-22) |


## Traceability

Which phases cover which requirements. Each requirement maps to exactly one phase.

| Requirement | Phase | Status |
|-------------|-------|--------|
| MEM-01 | Phase 5 | Pending |
| MEM-02 | Phase 5 | Pending |
| MEM-03 | Phase 5 | Pending |
| MEM-04 | Phase 5 | Pending |
| MEM-05 | Phase 5 | Pending |
| MEM-06 | Phase 5 | Pending |
| MEM-07 | Phase 5 | Pending |
| MEM-08 | Phase 5 | Pending |
| MEM-09 | Phase 5 | Pending |
| MEM-10 | Phase 5 | Pending |
| MEM-11 | Phase 5 | Pending |
| MEM-12 | Phase 5 | Pending |
| MEM-13 | Phase 5 | Pending |
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
| PROC-01 | Phase 7 | Pending |
| PROC-02 | Phase 7 | Pending |
| PROC-03 | Phase 7 | Pending |
| PROC-04 | Phase 7 | Pending |
| PROC-05 | Phase 7 | Pending |
| DEBT-01 | Phase 5 | Pending |
| DEBT-02 | Phase 5 | Pending |
| DEBT-03 | Phase 5 | Pending |
| DEBT-04 | Phase 5 | Pending |
| DEBT-05 | Phase 5 | Pending |
| SMP-01 | Phase 8 | Pending |
| SMP-02 | Phase 8 | Pending |
| SMP-03 | Phase 8 | Pending |
| SMP-04 | Phase 8 | Pending |
| SMP-05 | Phase 8 | Pending |
| SMP-06 | Phase 8 | Pending |
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
| USER-04 | Phase 12 | Pending |
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
