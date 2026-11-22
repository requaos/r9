# Requirements: r9

**Defined:** 2026-11-22
**Core Value:** The OS boots to rio on QEMU q35 at every commit.

Source tags in parentheses point at `.planning/intel/constraints.md` entries (IG = implementation guide, AC = architecture compendium, RFC, PT = permissions and test plan) or at owner direction. "Phase N" always means the GSD phase number in ROADMAP.md; where the source documents use their own label it is written "guide Phase N". Open questions (W1-W10) are not resolved here; each requirement that depends on one names it.

Standing constraint on every requirement below: the golden invariant (boots to rio on QEMU q35) holds at every commit, and the verification discipline of IG-V01 applies (RED pin, implementation, GREEN twice, pin advance).

## v1 Requirements

### Cross-Build Harness (Stage P0)

- [ ] **HARN-01**: One documented `cargo xtask` command boots r9 on QEMU q35 x86_64 to a rio session using only the r9 kernel and the pre-built Harvey tree, and exits non-zero if rio is not reached (the golden-invariant gate, runnable per commit) (owner direction; today only Harvey's flake gates exercise rio)
- [ ] **HARN-02**: Harvey is consumed strictly read-only from one resolved location; a single resolver reconciles the existing `R9_HARVEY_DIR` contract with `HARVEY_SRC` and `vendor/harvey/`, every existing consumer (boot-test, ureg-check, initrd, pcap netaddr seam, `harvey:` module paths in configs) goes through it, a missing tree fails with a message naming the variable, and a gate run leaves the Harvey tree unmodified (owner direction; PROJECT.md note N4)
- [ ] **HARN-03**: A Harvey fallback manifest in xtask enumerates every C library object and userland binary the image draws from Harvey, per architecture, with the source of each entry (Harvey or Rust) (owner direction)
- [ ] **HARN-04**: Switching one manifest entry to its Rust replacement swaps that single artifact into the assembled image and removes it from the Harvey fallback set, and the golden gate passes at that commit; switching it back restores the Harvey artifact (owner direction; IG-V02 risk 4)
- [ ] **HARN-05**: The harness fails fast with a named error when the image contains a Harvey artifact not in the manifest, or when an entry marked Rust still resolves to the Harvey artifact (the manifest and the image cannot drift)
- [ ] **HARN-06**: Existing x86_64 q35 boot-test profiles and the aarch64 virt quick pass (tier-1 on HVF and TCG, smoke, net-smoke and cpu once on HVF) still pass through the reconciled mechanism
- [ ] **HARN-07**: The r9 nix flake is the supplier of the read-only Harvey tree: the flake pins Harvey, and the default devshell (`nix develop`) exports the resolved tree location (`R9_HARVEY_DIR`) so every `cargo xtask` command run inside the devshell finds it with no manual setup; the tree it names is read-only and contains the pre-built artifacts the harness draws from; xtask itself keeps no built-in default path (outside the devshell an unset variable is still a named error); the flake apps and CI use the same supply (owner direction 2026-11-22)

### Design Lock and Flag Reservation (guide Phase 0)

- [ ] **DSGN-01**: A threat model document is committed stating in-scope protections (pages at rest, stale-frame disclosure across KeyIDs, stray kernel reads, packet splicing, injection, replay) and out-of-scope attacks (DRAM bus probing, cold boot, kernel-context code execution; SEV-SNP or TDX named as the remedy) (AC-T01)
- [ ] **DSGN-02**: The corrected C prototypes called for by the guide's Phase 0 are committed inside r9 (Harvey is not edited), with their landing location decided against the repository layout at HEAD (IG section 8)
- [ ] **DSGN-03**: Four `rfork` flag bits are reserved as named constants — `RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21 — in r9's rfork flag definitions only (`x86_64/src/proc.rs` and `aarch64/src/syscall.rs`); Harvey is not changed, a recorded deviation from the guide's "reserve bits 18-20 in `libc.h`"; verified not defined anywhere at HEAD, despite AC section 1.1 (IG-D03; owner decisions 2026-11-22: W11 resolved, `RFNOLEGACY` = `1 << 21`, r9-only reservation)
- [ ] **DSGN-04**: A host test pins the four reserved values and asserts they overlap no existing `rfork` flag (highest bit in use today is 14), and the reservation changes no `rfork` behaviour (behaviour arrives in Phase 6 for `RFNOLEGACY` and Phase 7 for the other three)
- [ ] **DSGN-05**: The maintainer confirms the guide's reconciled decisions (IG-D01 to IG-D13) and PROJECT.md Key Decisions records which are locked (IG section 8 gate)

### Crypto Foundations (guide Phase 1)

- [ ] **CRYP-01**: `crypto/` provides RFC 5869 HKDF-SHA256 built on HMAC-SHA256, passing RFC 5869 and RFC 4231 vectors, replacing the placeholder XOR fold (IG-A01 item 1)
- [ ] **CRYP-02**: `crypto/` provides a scalar ChaCha20 reference (`crypto/src/page.rs`) passing published vectors, with the counter and nonce construction fixed first (PROJECT.md note N1) (IG-A01 item 3, RFC-P01)
- [ ] **CRYP-03**: `crypto/` provides BLAKE3 keyed hash and tree-hash modes with 32-byte output passing the official test vectors, applying the MAC-primitive answer to W3 (RFC-P01, IG-D11)
- [ ] **CRYP-04**: The key hierarchy (`K_file = HKDF-Expand(K_user, Qid.path || Qid.vers, 32)`, transient and session keys) and the `KeyCapability` typestate exist: `ReadOnly { k_dec, blake3_verify }` cannot produce a valid integrity tag, `ReadWrite { k_dec, k_mac }` can, all key material is `Zeroizing<[u8; 32]>`, and siblings inheriting shared read-only base code keep the parent's `k_dec` (IG-M01, IG-A01 item 2, IG-P01)

### Data Structures (guide Phase 1)

- [ ] **DS-01**: `ds/` provides `FlattenedTphtShard` with a 64-byte `#[repr(C, align(64))]` `TphtBucketHeader`, seqlock, quotiented slots at the capacity settled by W6, and a host test showing over 90% of lookups resolve in the home block at target occupancy (IG-M06, IG-D09)
- [ ] **DS-02**: Feistel quotienting is bijective: key reconstruction from home block index and remainder round-trips for all tested keys (IG-M05)
- [ ] **DS-03**: `deref_tiny(key, tiny_ptr)` performs exactly one memory access with zero header probes, shown by a model-based test (IG-M06)
- [ ] **DS-04**: `ChainedTphtOverflow` implements two-choice dereference banks of 255 overflow nodes with a two-phase `WRITING` (0xFF) publication, reaching over 95% fill without allocation failure and supporting full-key verification on candidate match (IG-M07, IG-A01 item 12)
- [ ] **DS-05**: `ds/src/roaring.rs` provides a Roaring bitmap (array, bitmap and run containers) whose union, intersection, difference and `andnot` match a reference model on randomized inputs (IG-D10)
- [ ] **DS-06**: `ClientSet` is `Empty | Single | Inline{count, ids[4]} | Roaring`, uses zero heap for up to four clients, upgrades beyond four, and collapses contiguous ids into run containers (IG-M08, RFC-P08)
- [ ] **DS-07**: Funnel Hashing, if present, serves only write-once ephemeral scratchpads; no page cache, FidTable, or Venti index uses it (IG-D09, IG-A01 item 11)

### Lint and Audit (guide Phase 1)

- [ ] **LINT-01**: `crypto/` and `ds/` join the workspace under the lint contract settled by W5 and note N2; every unsafe site carries a `// SAFETY:` comment, `UNSAFE-AUDIT.md` lists the new sites, and `cargo xtask test`, `check` and `clippy` run both crates on the host toolchain (IG-D07, IG-V01)

### 9P2000.aead Wire and FidTable (guide Phase 2)

- [ ] **WIRE-01**: One canonical wire header byte layout is fixed (resolving W1, including whether `Pad` and `ExtTLVs Length` are in the AAD), implemented, and round-trip tested; AC listing 9.9 is corrected (RFC-P03, IG-P03)
- [ ] **WIRE-02**: `ExtMask` and TLV parsing handle all five extension bits (`EXT_DEADLINE`, `EXT_CAP_TOKEN`, `EXT_QID_VOLATILITY`, `EXT_STREAM_PRIO`, `EXT_OBINARY`) zero-copy over 64-bit aligned records, with `ExtTLVs Length` zero when `ExtMask` is zero (RFC-P03, RFC-P04)
- [ ] **WIRE-03**: On MAC failure the receiver terminates the connection and zeroes channel state without parsing any byte of the packet (RFC-P03)
- [ ] **WIRE-04**: Sequence nonces are strictly monotonic 64-bit counters per connection and direction, a packet that is not the expected next value is rejected, and the channel re-keys in band before 2^64 - 1 (RFC-P03, RFC-P10)
- [ ] **WIRE-05**: `Tauth` precedes `Tattach` and yields per-connection per-direction session keys (scope settled by W4; key-exchange sourcing per note N5), keys are zeroized on teardown (IG-A01 item 13, RFC-P02)
- [ ] **WIRE-06**: Encode and decode round-trip every base 9P2000 message and the opcode registry including reserved 130-137, with ChaCha20 ciphertext 4KB-aligned in `Twrite`, `Rread` and `Tstream` and the MAC per W3 (RFC-P06, RFC-P03)
- [ ] **WIRE-07**: `FidHandle` supports `StatefulHandle` (8 bytes) and `StatelessCap` (24 bytes) with the on-wire form of the capability settled by W2, and `FidEntry` is 64 bytes `align(64)` (IG-P02, RFC-P05)
- [ ] **WIRE-08**: `FidTable` and `FidShard` use TPHT; `deref_tiny` resolves a `FidEntry` in exactly one memory read; `StatelessCap` files use zero server FID memory and any replica holding the domain secret verifies in O(1) (IG-P02, IG-D01)
- [ ] **WIRE-09**: The `FidShard` failover sweep yields exactly `active_fids \ reclaimed_fids` in one bitmap and-not (IG-P02, RFC-P07)
- [ ] **WIRE-10**: `Qid` is 13 bytes and carries the three volatility classes `QTIMMUTABLE` 0x20, `QTLEASED` 0x40, `QTVOLATILE` 0x80 as mutually exclusive `qtype` bits, with the `#G` path formula `((ctx_id & 0xFFFFFF) << 8) | (node_type & 0xFF)` (IG-P01)
- [ ] **WIRE-11**: Anti-splicing pins pass: a replayed `Twrite` with a stale nonce is dropped, and a payload moved to another channel by editing the cleartext FID fails the AAD MAC (RFC-P10, PT-T02 test 2.3)
- [ ] **WIRE-12**: `Tversion "9P2000.aead"` selects the native engine while peers offering plain 9P2000 keep using the existing devmnt path unchanged, so the golden invariant holds (RFC-P09, owner invariant)
- [ ] **WIRE-13**: `cargo xtask test-9p` exists and runs the wire pins (round trips, `ExtMask` parsing, `StatelessCap` MAC verification, `FidShard` sweep) on the host toolchain (IG-V01)

### Cryptographic Memory and Zero-Copy DMA Paging (guide Phase 3)

- [ ] **MEM-01**: `UninitializedFrame` is popped without scrubbing, its PTE stays unmapped (`PROT_NONE`), ChaCha20 overwrites all 4,096 bytes, and only after BLAKE3 verification is it wrapped as an immutable `ShadowFrame` and published (IG-M04)
- [ ] **MEM-02**: Page sealing is Encrypt-then-MAC: ciphertext is verified in cache before decrypting into a shadow frame, with dedicated MAC keys (IG-A01 item 5)
- [ ] **MEM-03**: On `#PF` decrypt the 64-byte faulting line uses cached stores and the remaining 4,032 bytes use non-temporal stores, with `SFENCE` issued after the loop and overlapped with the scalar tree-MAC root and register restore (IG-M02)
- [ ] **MEM-04**: The vector fast path in `x86_64/src/cryptpage.rs` uses only `ymm0`-`ymm5` in one `global_asm!` routine, AVX-512 is disabled in XCR0, registers are scrubbed with `VPXOR` before returning to user mode, scalar and vector paths produce identical output, and the design is reconciled with `fpu::kernel_begin()` / `with_simd()` (note N7) (IG-M03, IG-A01 items 3-4, RFC-P10)
- [ ] **MEM-05**: `StrideTracker` evaluates a chronological stride ring buffer comparing oldest, middle and newest entries (IG-A01 item 6)
- [ ] **MEM-06**: Frame recycling skips zero and flush for same-KeyID reuse (`recycle_within`) and scrubs cross-domain reassignment with `CLFLUSHOPT`, gated on CPUID leaf 7 EBX bit 23 (IG-A01 item 7, IG-M04)
- [ ] **MEM-07**: The page allocator keeps clean, dirty and pinned Roaring pools over 256MB chunks with the address expression `(((chunk as usize) << 16) | (offset as usize)) * 4096`, full-container materialization in `fallback_difference`, pinned frames excluded from eviction, `Evictable = Dirty \ (Pinned | Active)`, and a High-Water Pager holding about 15% pre-cleared frames (IG-A01 items 8-9, IG-D10, IG-V02)
- [ ] **MEM-08**: `SuperpageResidency` is exactly 64 bytes and supports bit clearing and atomic counter reset on large-page demotion (IG-A01 item 10)
- [ ] **MEM-09**: A 9P2000.aead data packet DMAs directly into an `UninitializedFrame` in the destination segment's `BackingStore` with no kernel copy or decrypt on receive; demand `#PF` decrypts it correctly with zero memory amplification (IG section 8 gate, AC-P01)
- [ ] **MEM-10**: `UserSlice` / `UserSliceMut` enforce residency on demand behind `UserAddressSpace`, and kernel code that holds user memory across sleep points (for example devmnt reads) pins those frames against the evictor (IG-A01 item 17, IG-V02 risk 2)
- [ ] **MEM-11**: Boot profiles `q35lowmem` (forces eviction) and `q35crypt` (tamper kill and vector-register preservation probes) exist in `cargo xtask boot-test` and pass; `x86_64` is lint-opted-in and `UNSAFE-AUDIT.md` reconciles `shadow.rs` and `cryptpage.rs` (IG-V01, note N2)
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
- [ ] **RETIRE-04**: Every composed gate previously hosted in Harvey's flake (boot-r9-x64 smokes, net smoke, auth, drawterm, rio) runs from the r9 repo without Harvey, and the golden-invariant gate is green (derived: Harvey cannot be archived while its flake owns the gates; note N4)
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
| HARN-01 | Phase 1 | Pending |
| HARN-02 | Phase 1 | Pending |
| HARN-03 | Phase 1 | Pending |
| HARN-04 | Phase 1 | Pending |
| HARN-05 | Phase 1 | Pending |
| HARN-06 | Phase 1 | Pending |
| HARN-07 | Phase 1 | Pending |
| DSGN-01 | Phase 2 | Pending |
| DSGN-02 | Phase 2 | Pending |
| DSGN-03 | Phase 2 | Pending |
| DSGN-04 | Phase 2 | Pending |
| DSGN-05 | Phase 2 | Pending |
| CRYP-01 | Phase 3 | Pending |
| CRYP-02 | Phase 3 | Pending |
| CRYP-03 | Phase 3 | Pending |
| CRYP-04 | Phase 3 | Pending |
| DS-01 | Phase 3 | Pending |
| DS-02 | Phase 3 | Pending |
| DS-03 | Phase 3 | Pending |
| DS-04 | Phase 3 | Pending |
| DS-05 | Phase 3 | Pending |
| DS-06 | Phase 3 | Pending |
| DS-07 | Phase 3 | Pending |
| LINT-01 | Phase 3 | Pending |
| WIRE-01 | Phase 4 | Pending |
| WIRE-02 | Phase 4 | Pending |
| WIRE-03 | Phase 4 | Pending |
| WIRE-04 | Phase 4 | Pending |
| WIRE-05 | Phase 4 | Pending |
| WIRE-06 | Phase 4 | Pending |
| WIRE-07 | Phase 4 | Pending |
| WIRE-08 | Phase 4 | Pending |
| WIRE-09 | Phase 4 | Pending |
| WIRE-10 | Phase 4 | Pending |
| WIRE-11 | Phase 4 | Pending |
| WIRE-12 | Phase 4 | Pending |
| WIRE-13 | Phase 4 | Pending |
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

**Coverage:**
- v1 requirements: 126 total
- Mapped to phases: 126
- Unmapped: 0

---
*Requirements defined: 2026-11-22*
*Last updated: 2026-11-22 after roadmap creation*
