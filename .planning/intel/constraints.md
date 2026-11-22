# Constraints (from SPEC documents)

All four ingested documents are classified SPEC, none locked. Precedence between them was set by the owner's manifest: 0 = implementation guide, 1 = architecture compendium, 2 = RFC, 3 = permissions and test plan. Lower number wins on contradiction. Entries are grouped by source in that order. Section references are to the source documents so a consumer can read the full text.

Source keys used below:
- IG = /Users/neilskinner/code/r9/.planning/r9-implementation-guide-amended-v3.md (precedence 0)
- AC = /Users/neilskinner/code/r9/.planning/r9-architecture-and-code-v4.md (precedence 1)
- RFC = /Users/neilskinner/code/r9/.planning/rfc-9p2000-aead.md (precedence 2)
- PT = /Users/neilskinner/code/r9/.planning/r9-permissions-and-test-plan.md (precedence 3)

Open conflicts touching an entry are cross-referenced as [see W#] (warnings) or [see I#] (info) in /Users/neilskinner/code/r9/.planning/INGEST-CONFLICTS.md.

---

# Group 1: Implementation Guide (precedence 0)

## IG-D01: Build priority, 9P and ds/ upfront
- source: IG section 2
- type: nfr
- content: Core data structures (`ds/`) and `9P2000.aead` wire handling are built in Phase 1 and Phase 2. Landing sites: `ds/`, `crypto/`, `port/src/fid.rs`, `port/src/chan.rs`. Rationale given: Plan 9 is defined as a network file fabric. IG section 2 states the decisions in that table are "reconciled and non-negotiable".

## IG-D02: Executable format is Plan 9 a.out exclusively
- source: IG section 2
- type: nfr
- content: `a.out` is the sole binary format the kernel loads. Landing sites `x86_64/src/exec.rs`, `xtask/src/elf2aout.rs`. Linker-emitted ELF is converted by in-tree tooling. Contrib code and 9front utilities run unmodified.

## IG-D03: Strictly closed system call list
- source: IG section 2
- type: api-contract
- content: No POSIX emulation calls and no `mmap` extensions. Software adapts to Plan 9 primitives (`rfork`, `bind`, `mount`, 9P). Crypto extensions add 3 flag bits to `rfork` and zero syscalls. Landing sites `sys/include/libc.h`, `sys/src/syscall.rs`. [see W11 for the RFNOLEGACY flag]

## IG-D04: Native Rust runtime, r9 std crate
- source: IG section 2, section 9
- type: nfr
- content: Full Rust `std` implemented as `sys/src/libstd_r9/` wrapping Plan 9 syscalls (`/net`, `rfork(RFMEM)`, 9P file ops). Sequenced as Milestone STD, after Milestone S and before Track P. Removes the `#![no_std]` requirement for Harvey userland and native applications.

## IG-D05: C toolchain and compilers
- source: IG section 2
- type: nfr
- content: Plan 9 C compilers (`6c`, `6a`, `6l`) are ported to Rust (Track P, Stage P3). Harvey's C compilers are bootstrap tools until parity is proven.

## IG-D06: Dual C runtime model
- source: IG section 2, section 10
- type: nfr
- content: Classic APE (`sys/src/ape`, compiled via `6c`) for Plan 9 contrib software (Ghostscript as demonstration). Vendored APEX (`vendor/apex`, compiled via Clang) for LLVM/C++ tooling. The two runtimes never mix in one binary. APEX is Harvey-OS/apex, MIT license, musl-based ANSI C over Plan 9 syscalls without `mmap`; syscall stubs generated from `sys/src/libc/9syscall/sys.h`.

## IG-D07: Kernel safety boundary
- source: IG section 2, section 11
- type: nfr
- content: `port/` is `#![forbid(unsafe_code)]`. Low-level operations (`WRPKRU`, `MOVNTDQ`, `CLFLUSHOPT`, inline assembly, raw atomics) are restricted to `x86_64/`, `aarch64/`, and `ds/src/tpht/` behind audited `// SAFETY:` blocks. Section 11 lint contracts: `undocumented_unsafe_blocks = deny` on arch crates; `#![forbid(unsafe_code)]` on `crypto/`, `ds/`, and `port/`. [see W5 for the ds/ inconsistency]

## IG-D08: Memory isolation model
- source: IG section 2
- type: nfr
- content: Independent CR3 per sibling process plus intra-process PKEYs. Sibling processes use distinct page-table roots, which avoids PKEY capacity exhaustion. PKEYs enforce read-only base permissions in about 20 cycles. Landing sites `x86_64/src/mmu.rs`, `port/src/segcrypt.rs`.

## IG-D09: Primary concurrent hashing is succinct TPHT
- source: IG section 2, section 5
- type: schema
- content: Succinct TPHT (flattened and chained) in `ds/src/tpht/` replaces Funnel Hashing in the Page Cache, 9P FidTable and Venti Index. Funnel Hashing is restricted to write-once ephemeral scratchpads. Single cache-line hits via Feistel quotienting; 1-byte tiny pointers give single-read dereference (`deref_tiny`).

## IG-D10: Integer set algebra is hybrid Roaring Bitmaps
- source: IG section 2, section 5.4
- type: schema
- content: `ds/src/roaring.rs` (Lemire et al., arXiv:1603.06549). Uses: physical frame allocator (clean, dirty, pinned pools over 256MB DRAM chunks); eviction `Evictable = Dirty \ (Pinned | Active)` in under 200 cycles; `SuperpageResidency` (512 sub-pages in an 8-word, 64-byte bitset, 2MB promotion in about 5 cycles); 9P priority tag scheduling P0-P2; deadline wheels; exact union layer filters; `ClientSet` inline enum for 1-4 clients upgrading to `RoaringBitmap` for 10^4+ nodes.

## IG-D11: Distributed wire security is native 9P2000.aead
- source: IG section 2, section 6.3
- type: protocol
- content: AAD envelope with `ExtMask`, monotonic sequence nonces, hybrid `FidHandle`, four new message pairs (`Tcompound`, `Tstream`, `Tlease_revoke`, `Treclaim`). Cleartext fallbacks use pinned bounce buffers. Landing sites `port/src/chan.rs`, `port/src/fid.rs`, `crypto/src/session.rs`. Detailed protocol is in the RFC group below. [see W1, W2, W3, W4]

## IG-D12: 3D graphics via Plan 9 #G synthetic device
- source: IG section 2
- type: api-contract
- content: VirtIO-GPU Venus transport serializing Vulkan streams over `/dev/gpu/` nodes `clone`, `ctl`, `cmd`, `blob`, `fence`. Zero-copy window presentation via `draw(3)` host texture blits. Landing sites `port/src/devgpu.rs`, `kern/src/gpu.rs`.

## IG-D13: Cluster service discovery
- source: IG section 2, section 6.4
- type: protocol
- content: Multicast beacons plus staged `/srv` registration. Landing sites `sys/src/cmd/discover/`, `sys/src/cmd/beacon/`, `port/src/chan.rs` (`#M`). Descriptors `class:instance[+attrs]` over dual-stack multicast (`DiscoveryBeaconV2`). Two-tier degradation (30s unhealthy, 5m dead). Detail under IG-P04 and AC-P03.

## IG-A01: Audit corrections ledger (17 permanent corrections)
- source: IG section 3
- type: nfr
- content: "17 critical defects that must remain permanently corrected in all subsequent PRs". Each is a required property of the final code.
  1. `crypto/src/kdf.rs`: RFC 5869 HKDF-SHA256 engine pinned against RFC 4231 vectors (replaces placeholder XOR fold in HMAC).
  2. `crypto/src/key.rs`: sibling processes inheriting shared read-only base code keep the parent's `k_dec`; private segments derive isolated keys.
  3. `crypto/src/simd.rs`: scalar reference in `crypto/src/page.rs`; vector fast path in `x86_64/src/cryptpage.rs` implementing verified ChaCha20.
  4. `crypto/src/simd.rs`: single unified `global_asm!` routine saving `ymm0`-`ymm5`; AVX-512 state disabled in XCR0.
  5. `crypto/src/simd.rs`: Encrypt-then-MAC. Verify ciphertext in cache before decrypting into shadow frames; dedicated MAC keys.
  6. `port/src/seg.rs`: chronological stride ring buffer evaluation in `port/src/segcrypt.rs` comparing oldest, middle, newest entries.
  7. `port/src/shadow.rs`: CPUID leaf 7 EBX bit 23 probe before `CLFLUSHOPT`; explicit `recycle_within` path for clean same-domain frames.
  8. `port/src/page.rs`: address expression `(((chunk as usize) << 16) | (offset as usize)) * 4096`.
  9. `port/src/page.rs`: full container materialization in `fallback_difference`; pinned frames are excluded from eviction candidates.
  10. `port/src/superpage.rs`: residency tracker is exactly 64B; supports bit clearing and atomic counter reset on large-page demotion.
  11. `ds/src/funnel.rs`: replaced in primary paths by TPHT; Funnel restricted to write-once ephemeral scratchpads.
  12. `ds/src/scoreindex.rs`: Chained-TPHT; 64-bit folded key is a table hint, full 32-byte BLAKE3 score verified on candidate match.
  13. `crypto/src/session.rs`: per-connection, per-direction keys derived during `Tauth`, strict 64-bit monotonic sequence nonces.
  14. `kern/src/gpu.rs`: negotiate VirtIO feature bits 0, 3, 4 (`VIRGL`, `BLOB`, `CONTEXT_INIT`); select Venus capability set 4 on context creation (bit 1 is EDID).
  15. `port/src/devgpu.rs`: context ID packed into the QID path; kernel constructs and validates all VirtIO packet headers.
  16. `kern/src/notes.rs`: note delivery requires `SegKind::Stack`; checked arithmetic keeps the frame strictly within stack bounds.
  17. `port/src/access.rs`: fold into existing `UserSlice` / `UserSliceMut` behind the `UserAddressSpace` trait (existing `x86_64/src/usermem.rs`); residency enforced on demand.
  [see I4 for differences from the AC ledger, I5 for appendix listings that contradict items 1, 12 and the port unsafe rule]

## IG-M01: KeyCapability typestate
- source: IG section 4.1; AC section 3.1, appendix 9.1
- type: schema
- content: In `port/src/segcrypt.rs`, `KeyCapability` is an enum. `ReadOnly { k_dec, blake3_verify }` and `ReadWrite { k_dec, k_mac }`, all `Zeroizing<[u8; 32]>` (`KEY_LEN = 32`). ReadOnly holds no write-authentication key, so it cannot generate valid integrity tags. ReadWrite holds both and can re-seal on eviction.

## IG-M02: Dual-path store routing and SFENCE hiding
- source: IG section 4.2; AC section 3.2; RFC section 9.1
- type: nfr
- content: On `#PF` decrypt of a 4KB page, the 64-byte line at `fault_addr & !0x3F` is written with cached stores (`VMOVAPS`). The remaining 4,032 bytes (63 lines) use non-temporal stores (`MOVNTDQ`). `SFENCE` is issued immediately after the store loop; the write-combining drain overlaps scalar BLAKE3 tree-MAC root computation and vector register restore.

## IG-M03: Restricted vector ABI ymm0-ymm5
- source: IG section 4.3; AC section 3.3; RFC section 11 item 2
- type: nfr
- content: Vector crypto fast path in `x86_64/src/cryptpage.rs` uses only `ymm0`-`ymm5`: `ymm0`-`ymm3` hold the four ChaCha20 state rows, `ymm4`-`ymm5` are temporaries for rotates and lane splitting. Spill via `VMOVUPS` (about 192 bytes, about 15 cycles), zero with `VPXOR` after use, restore (about 15 cycles); about 30 cycles total round trip. AVX-512 disabled in XCR0. RFC requires the register scrub (`VPXOR`) before returning to user mode. [see I1 for the XSAVE cost discrepancy]

## IG-M04: UninitializedFrame allocator bypass
- source: IG section 4.4; AC section 3.4
- type: nfr
- content: Frames are popped from free pools without scrubbing. The PTE stays unmapped (`PROT_NONE`) while the frame is uninitialized. ChaCha20 overwrites all 4,096 bytes from index 0 to 4,095. Only after BLAKE3 verification passes is the frame wrapped as an immutable `ShadowFrame` and published to page tables. AC adds: same-KeyID recycling skips zeroing and flush; cross-domain reassignment scrubs with `CLFLUSHOPT` gated by CPUID leaf 7.

## IG-M05: Feistel quotienting
- source: IG section 5.1; AC section 4.1
- type: schema
- content: `ds/src/tpht/flattened.rs`. One-round bijective Feistel `F(L, R) = (R, L xor f(R))` over the upper and lower 32 bits of a 64-bit key, with `f(R) = (R xor (R >> 16)) * 0x85EBCA6B xor ...` (the source elides the remaining terms). Key reconstruction from home block index `q` and remainder `r`: `R = P mod 2^32`, `L' = P >> 32`, `L = L' xor f(R)`, `Key = (L << 32) | R`, in 3-4 ALU cycles. Saves about 20-24 bits per entry in the kernel page cache. AC adds: `P = (L' << 32) | R`; home block index is the lower log2(num_blocks) bits of P; stored remainder is the upper bits.

## IG-M06: TphtBucketHeader and FlattenedTphtShard layout
- source: IG section 5.2; AC section 4.2, appendix 9.3
- type: schema
- content: `#[repr(C, align(64))]`, exactly 64 bytes. `seq_lock: AtomicU32` (odd = writer active, even = stable); `occupied_mask: u16`; `spilled_mask: u16` (tiny pointer exceeds home window, `ptr > 16`); `quotients: [u16; 16]` (32 B); `tiny_ptrs: [u8; 16]` (displacements 1..=255, 0 = empty); `clock_nibbles: AtomicU64` (sixteen 4-bit counters). Over 90% of lookups resolve in the home block. `deref_tiny` with `(key, tiny_ptr)` is exactly one memory access with zero header probes: `slot = ((b << 4) + (ptr.get() - 1)) & slot_mask`. AC adds: key-only probing uses seqlock plus SIMD match on `quotients`; home bucket holds up to 16 entries in slots 1..=16, spill scans 17..=255 via 64-bit word bitmasks on `slot_bitmap`; 4-bit clock-nibble second-chance eviction for caches such as `UnionIndex`. [see W6 for the 4-slot versus 16-slot statement]

## IG-M07: ChainedTphtOverflow, two-choice dereference banks
- source: IG section 5.3; AC section 4.4, appendix 9.4
- type: schema
- content: `ds/src/tpht/chained.rs`. Each bank is an arena of 255 overflow nodes indexed by a 1-byte tiny pointer. Each home block maps to two candidate bank pairs; new nodes go to the bank with more free capacity. Slots are claimed with a two-phase publication using a `WRITING` sentinel (`0xFF`) so readers never see partial entries. AC: enables fill above 95% without allocation failures; used for sparse unbounded indices such as the Venti score index.

## IG-M08: Roaring-based structures in the kernel
- source: IG section 5.4; AC section 4.5, 4.6, 4.7
- type: schema
- content: See IG-D10. AC adds: 16-bit tag space fits single Roaring containers; priority tiers `P0_CONTROL`, `P1_METADATA`, `P2_BULK` pop the next runnable stream via `tzcnt` in O(1); millisecond-bucketed Roaring bitmaps implement timer wheels with cascading cancellation down exportfs proxies. `ClientSet` enum is `Empty | Single(u32) | Inline { count: u8, ids: [u32; 4] } | Roaring(Box<RoaringBitmap>)`. Funnel Hashing is retained only as a lock-free scratchpad in `port/src/inflight.rs` (dedup of concurrent faults on shared Block A code). Elastic Hashing is retained only in `telescope` for batch-built immutable full-text indexes over Venti snapshots.

## IG-P01: Qid layout and volatility classes
- source: IG section 6.1; AC section 5.1; RFC section 7
- type: protocol
- content: `Qid` is 13 bytes (`path: u64`, `vers: u32`, `qtype: u8`). `#G` routing: `Qid.path = ((ctx_id & 0xFFFFFF) << 8) | (node_type & 0xFF)`. Object key: `K_file = HKDF-Expand(K_user, Qid.path || Qid.vers, 32)`. Three mutually exclusive volatility classes in `Qid.qtype`: `QTIMMUTABLE 0x20` (content-addressed, cache indefinitely, no Tstat/Twalk revalidation), `QTLEASED 0x40` (shared mutable, cached until server `Tlease_revoke`), `QTVOLATILE 0x80` (live synthetic files, never cached, routed to the device handler). [see I11 for the "upper 24 bits" wording]

## IG-P02: FidHandle, FidTable and failover sweep
- source: IG section 6.2; AC section 5.2, appendix 9.6; RFC section 5
- type: protocol
- content: `StatelessCap` (24 bytes) for `QTIMMUTABLE` files and static directory walks: self-contained HMAC-BLAKE3 token, zero server RAM, any replica with the domain secret verifies in O(1). `StatefulHandle` (8 bytes) for mutable files, pipes, `/net`, `#G` contexts, leases: embeds 8-bit `TinyPtr`, `shard_id`, `epoch`. `deref_tiny` resolves `FidEntry` in strictly one memory read. Failover sweep `Orphaned = active_fids \ reclaimed_fids` in one SIMD ANDNOT. Landing site `port/src/fid.rs`. [see W2]

## IG-P03: 9P2000.aead envelope and message pairs (summary)
- source: IG section 6.3; AC section 5.3, 5.4
- type: protocol
- content: Packet = Size(4B) || Type(1B) || Tag(2B) || FidHandle(8B) || SeqNonce(8B) || ExtMask(2B) || ExtTLVs as AAD, then 4KB-aligned ChaCha20 ciphertext, then 32-byte MAC. ExtMask bits: `EXT_DEADLINE`, `EXT_CAP_TOKEN`, `EXT_QID_VOLATILITY`, `EXT_STREAM_PRIO`, `EXT_OBINARY`. Four new pairs: `Tcompound`/`Rcompound` (atomic multi-op via `CURRENT_FID`), `Tstream`/`Rstream`, `Tlease_revoke`/`Rlease_ack` (Roaring invalidation sets), `Treclaim`/`Rreclaim`. Legacy gateway uses non-pageable pre-pinned bounce buffers zeroized after transmission; sandboxed via `rfork(RFNOLEGACY)`. Field-level detail is in RFC-P01 to RFC-P10. [see W1, W3]

## IG-P04: Dynamic cluster service discovery and staged /srv lifecycle
- source: IG section 6.4; AC section 5.9, appendix 9.14, 9.15
- type: protocol
- content: Resource specifier `Class:Instance[+Attributes]` (class examples `gpu`, `fs`, `venti`; instance examples `0`, `1`, `compute`, `main`; attributes example `venus,vram=24G,headless`). Canonical registration `/srv/{hostname}_{class}_{instance}` (for example `/srv/gpu02_gpu_0` display, `/srv/gpu02_gpu_1` compute). `DiscoveryBeaconV2`: 192-byte packed struct, `magic b"R9DSC2"`, sequence nonce, 16-byte IPv6 or IPv4-mapped address, port, family, attributes. Multicast: IPv4 `239.9.9.9!9999` (TTL 2); IPv6 link-local `[ff02::9:9]!9999`. Dial strings `connect [{ipv6}]!{port}\n` and `connect {ipv4}!{port}\n`. Lifecycle: Healthy (0-30s silence; `/srv/{name}.status` = `healthy\n`; bit set in `#M` `healthy_replicas`), Unhealthy (30s to 330s; status `unhealthy\n`; bit cleared; TCP socket stays open so in-flight work drains; any heartbeat restores Healthy), Dead (over 330s total; descriptor closed, `/srv/{name}` and `/srv/{name}.status` removed, peer reaped). [see W7, I2, I6]

## IG-S01: Venti parallel BLAKE3 content-addressed storage
- source: IG section 7.1; AC section 6.1
- type: schema
- content: SHA-1 replaced by parallel BLAKE3 Merkle digests (128-bit collision resistance, over 10 GB/s per core). `VentiTphtIndex`: Chained-TPHT, 16MB head array resident in L3, wait-free lookups. Hash-before-compression invariant: `Score = BLAKE3(Plaintext)` over uncompressed blocks, so dedup and Merkle roots are stable across local compression and background re-encoding.

## IG-S02: On-disk super-blocks and out-of-band tag slabs
- source: IG section 7.2; AC section 6.2
- type: schema
- content: 64KB data super-blocks (16 contiguous 4KB pages aligned to 64KB flash erase boundaries). 1KB tag slabs holding sixteen 64-byte `TagEntry` structures (98.9% DRAM efficiency claimed). IG fields: `blake3_tag: [u8; 32]`, `offset: u32`, `compressed_size: u16`, `original_size: u16`, `block_type: u8`, `compression_algo: u8` (0 None, 1 Zstd). AC fields are a superset: adds `nonce: [u8; 12]` (ChaCha20 sequence nonce) and `_reserved: [u8; 10]`; `block_type` Data=0x00, Pointer0=0x01, Dir=0x02; `offset` is the physical byte offset within the 64KB super-block. AC also states variable-length compressed blocks pack into the 64KB frame with no padding gaps.

## IG-S03: Two-tier Zstd compression and flash health
- source: IG section 7.3; AC section 6.3
- type: nfr
- content: Fossil commits compress dirty blocks in flight at Zstd level 3. If `compressed_size >= original_size`, store as `COMPRESS_NONE`. User writes hit Fossil's in-memory cache with zero compression latency (AC: compression runs in an asynchronous background worker). Decompression about 1.2-1.5 GB/s per core (about 2.5-3.5 microseconds per 4KB block); scratchpads pre-sized from `original_size`. Daily (Son) and weekly (Father) snapshots stay at level 3. Re-encoding to Zstd 9-15 happens only during GFS mark-and-sweep consolidation when an arena falls below 40% occupancy; live arenas are never rewritten.

## IG-S04: Fossil GFS retention and mark-and-sweep GC
- source: IG section 7.4; AC section 6.4
- type: nfr
- content: Fossil sends surviving BLAKE3 roots to Venti. Mark: Venti traverses surviving Merkle trees into an ephemeral `ReferencedBlocks` Roaring bitmap. Sweep: `PurgeSet = ArenaPresence \ ReferencedBlocks` by SIMD set difference in under 200 cycles. Contiguous purged runs are deallocated with NVMe Dataset Management (DSM) / TRIM.

## IG-L01: libstd_r9 module mapping
- source: IG section 9.2; AC section 8.3
- type: api-contract
- content: `sys/src/libstd_r9/` is a Platform Abstraction Layer on the closed syscall interface, no POSIX layer. Files under `src/pal/`: `fs.rs` (std::fs to 9P open/read/write/stat/remove, `Twalk` via `#M`, `dirread(2)`, permission bits `0o777`, `DMDIR`, `DMAPPEND`, `DMEXCL`), `net.rs` (std::net via `/net/tcp/clone`, `ctl`, `data`, `listen`; UDP `headers` gives 52-byte framed packets; native IPv6 `[fe80::1]!564` and IPv4 `10.0.0.1!564`), `thread.rs` (`rfork(RFPROC | RFMEM | RFFDG)`, stacks via `segattach` with guard pages, TLS via `%gs` / `FS_BASE` set in `x86_64/src/mmu.rs`), `sync.rs` (uncontended CAS about 12 cycles; contended sleep on `#c/rendezvous` or `rendezvous(tag, val)`), `process.rs` (`rfork(RFPROC | RFFDG | RFENVG | RFNAMEG)` plus `exec(a.out)`, exit status via `#c/wait` or `/proc/<pid>/wait`, `pipe(2)` stdio), `alloc.rs` (`sbrk(2)` heap; large buffers via `segattach(2)` under `RFCKEY`), `time.rs` (`RDTSC` for Instant; `#c/bintime` for SystemTime), `env.rs` (`/env/<name>` 1:1), `io.rs`, `stdio.rs` (fd 0, 1, 2).

## IG-T01: Self-hosting toolchain (Phase 8)
- source: IG section 10; AC section 8.2
- type: nfr
- content: APEX vendored for C runtime; dual C runtime isolation (IG-D06); `rust-lld` emits static ELF that an in-tree tool wraps into `a.out`; `rustc`'s `memmap2` metadata loader falls back to whole-file read buffers so no new kernel mmap call is needed.

## IG-V01: Verification discipline and matrix
- source: IG section 11
- type: nfr
- content: Every phase follows: a committed RED pin, the implementation, GREEN verification twice, then pin advance. Harnesses: `cargo xtask test` (crypto vectors, TPHT single-line hit models, `deref_tiny`, Roaring SIMD set algebra, `StrideTracker`, `TagTable` bounds, page sealing round trips); `cargo xtask test-9p` (wire round trips, `ExtMask` parsing, `Tcompound` sequencing, `StatelessCap` MAC verification, `FidShard` failover sweep, `DiscoveryBeaconV2` dual-stack multicast decode, 30s/5m degradation timing, `healthy_replicas` invalidation); `cargo xtask check` plus clippy (lint contracts); `UNSAFE-AUDIT.md` reconciling every unsafe site across `x86_64/src/shadow.rs`, `cryptpage.rs`, `ds/src/tpht/flattened.rs`; `cargo xtask boot-test` (`q35lowmem` forces eviction; `q35crypt` probes tamper kills, sibling isolation, note spoofing, vector register preservation); Harvey flake apps (`boot-r9-x64`, `net`, `auth`, `drawterm`, `rio`); KVM benchmarks (cycles per fault scalar vs vector, eviction cost, TPHT hit rate at 90%+ occupancy, 9P IOPS under ring bypass).

## IG-V02: Technical risks and mitigations
- source: IG section 11
- type: nfr
- content: (1) Crypto fault cost about 4.4 microseconds per cold fault on KVM; mitigation: High-Water Pager keeps a 15% pre-cleared clean frame buffer and schedules background encryption in idle cycles. (2) Kernel code that holds user memory across sleep points (for example `devmnt` reads into user buffers) must pin those frames against the evictor. (3) If the soft-float kernel target's assembler rejects AVX mnemonics in `global_asm!`, isolate the vector routine in a standalone assembly file (`l.S`). (4) Track P is about 1.2 million lines of C; the Stage P0 cross-build path enables one-at-a-time binary replacement so the system boots and passes tests at every intermediate commit.

---

# Group 2: Architecture Compendium (precedence 1)

Only content not already carried above, or where AC adds detail, is listed here.

## AC-T01: Threat model boundaries
- source: AC section 2.2
- type: nfr
- content: In scope: protection of pages at rest (evicted frames, backing stores, NVMe swap, Venti archives, 9P-AEAD transit); prevention of stale-frame disclosure across process KeyIDs; protection against stray kernel reads via wrong page-table mappings (only the shadow frame holds plaintext); prevention of packet splicing, injection and replay. Out of scope: physical DRAM bus probing, cold-boot attacks, arbitrary code execution in kernel context (a compromised kernel exposes keys). AMD SEV-SNP or Intel TDX is required if DRAM bus sniffing is in scope.

## AC-P01: Zero-copy DMA paging and user-space ring bypass
- source: AC section 5.7; RFC section 9
- type: protocol
- content: NIC DMAs received 9P2000.aead ciphertext into an `UninitializedFrame` in the destination segment's `BackingStore`; kernel does not decrypt on receive; decrypt is deferred to the page fault. Ring bypass: synthetic ring nodes under `#M` (`/dev/mnt/<conn>/ring`); applications map them via `segattach(SG_SHARED, fd, va, len)` with channel-backed capabilities (`#s`) for lock-free transfers between application buffers and the NIC.

## AC-P02: Distributed leases and coherence tracking
- source: AC section 5.5, appendix 9.7; RFC section 7.2
- type: protocol
- content: Server `LeaseRecord` in `FlattenedTphtShard<u64, LeaseRecord>` keyed by inode; readers tracked with `ClientSet` (inline up to 4; Roaring beyond; contiguous IDs such as 100..4096 collapse to one 4-byte run container). Client `ClientBlockCache` tracks `valid_blocks` and `dirty_blocks` bitmaps. On `Tlease_revoke` the client computes `dirty_blocks & revoked_range`, seals those pages with the restricted vector ABI and spatial nonces (`CoreID || PageOffset || Epoch`) and flushes them to the server. `LeaseRecord` is `#[repr(C, align(64))]` with fields `inode`, `generation`, `exclusive_owner`, `pending_revokes`, `volatility`, `mode`, `readers`, `_pad`.

## AC-P03: Discovery details (beacon struct, interval, groups)
- source: AC section 5.9, appendix 9.14, 9.15
- type: protocol
- content: Beacon interval default 5 seconds (UDP). Field list for `DiscoveryBeaconV2` (little-endian, 8-byte aligned, stated total 192 bytes): `magic [u8;6]`, `version u16 = 2`, `nonce u64` (monotonic heartbeat), `hostname [u8;32]`, `domain [u8;32]`, `res_class [u8;16]`, `res_instance [u8;16]`, `attributes [u8;64]`, `ip_addr [u8;16]`, `port u16`, `addr_family u8` (4 or 6), `_reserved [u8;7]`. AC additionally lists an IPv6 site-local group `[ff05::9:9]!9999` (routed datacenter fabric). Beacon source sets IPv6 TTL 4 and IPv4 TTL 2; domain defaults to `lab.r9`. Daemon `cmd/discover` reads Plan 9 UDP header of 52 bytes before payload and authenticates via Factotum `Tauth`. Constants `SILENCE_UNHEALTHY_MS = 30_000`, `SILENCE_DEAD_MS = 30_000 + 300_000`. [see W7, I2, I6]

## AC-P04: Union mounts, speculative parallel walks
- source: AC section 5.6, appendix 9.8; RFC section 8
- type: protocol
- content: `port/src/union.rs`. Each static or leased layer publishes an exact Roaring bitmap `entry_hashes` of salted 32-bit hashes `(parent_qid ^ name_hash)`, giving O(1) local membership tests with zero false positives. Layers flagged in `dynamic_layer_mask` (for example `/proc`, `/net`) plus matching static layers are probed concurrently with speculative parallel `Twalk`; the earliest positive response from the highest-priority layer commits to a 32-byte `UnionRouteEntry` in a quotiented `FlattenedTphtShard` with 4-bit clock-nibble aging, and remaining probes abort.

## AC-P05: Legacy translation gateway
- source: AC section 5.8; RFC section 10
- type: protocol
- content: `Tversion` negotiates `9P2000`, `9P2000.u`, `9P2000.e` or `9P2000.L` for legacy peers; AEAD extensions are stripped or emulated at the gateway. Plaintext only through pre-pinned non-pageable bounce frames sealed under `K_legacy_session`, scrubbed with `zeroize` plus `CLFLUSHOPT`. `rfork(RFNOLEGACY)` forbids binding, mounting or walking channels marked `CHAN_UNTRUSTED_WIRE`. Landing site `port/src/legacy_9p.rs`.

## AC-G01: VirtIO-GPU and #G layers
- source: AC section 7, appendix 9.11, 9.12, 9.13
- type: api-contract
- content: Layer 1 `kern/src/gpu.rs`: negotiate bits 0, 3, 4, then capset 4 (`VIRTGPU_CAPSET_VENUS`). Layer 2 `port/src/devgpu.rs` implements the `Dev` trait; files `/dev/gpu/clone`, `/dev/gpu/<id>/ctl`, `cmd` (Venus-serialized Vulkan command buffers), `blob` (host GPU memory into user namespaces), `fence`. Layer 3 `draw(3)` bridge: `/dev/draw/new` `b` command binds a window to a host VirtIO resource (`Image.gpu_res_id = Some(res_id)`); `draw_composite()` emits `emit_host_gpu_blit()` to the GPU control queue when both images have host resource IDs, so compositing stays in host VRAM; otherwise CPU memdraw. [see I11]

## AC-R01: Appendix source listings (non-normative)
- source: AC section 9 (listings 9.1 to 9.15)
- type: schema
- content: AC presents 15 source listings as "Complete, Corrected Source Listings": 9.1 `crypto/src/key.rs`, 9.2 `port/src/segcrypt.rs` (StrideTracker), 9.3 `ds/src/tpht/flattened.rs`, 9.4 `ds/src/tpht/chained.rs`, 9.5 `port/src/palloc.rs`, 9.6 `port/src/fid.rs`, 9.7 `port/src/lease.rs`, 9.8 `port/src/union.rs`, 9.9 `port/src/chan.rs`, 9.10 `venti/src/index.rs`, 9.11 `kern/src/gpu.rs`, 9.12 `port/src/devgpu.rs`, 9.13 `port/src/image.rs`, 9.14 `sys/src/cmd/discover/main.rs`, 9.15 `sys/src/cmd/beacon/main.rs`. Several listings conflict with the IG ledger or with stated invariants; for those the IG wins and the listing is reference material only. [see I5, I6, I11]

## AC-R02: Realigned roadmap (see context.md)
- source: AC section 8.1
- type: nfr
- content: The AC phase table is identical to the IG phase table except for Phase 0 dependency and a Phase 7 wording difference; both are reproduced in `context.md` under "Roadmap as specified". [see I3]

---

# Group 3: RFC 9P2000.aead (precedence 2)

## RFC-P01: Scope and cryptographic primitives
- source: RFC sections 1, 3.1
- type: protocol
- content: Addresses six scaling bottlenecks: synchronous RPC amplification, no cache coherence, throughput capped by `msize/RTT`, transport multiplexing head-of-line blocking (16-bit tags, 65,535 ceiling), stateful FID memory exhaustion, sequential union traversal. Adds no custom system calls and does not break the closed call list. Data-plane primitives: ChaCha20 (64-bit block counters, 96-bit nonces) for confidentiality; Keyed BLAKE3 in tree-hashing mode for integrity and authenticity. Key words per BCP 14 (RFC 2119, RFC 8174). [see W3]

## RFC-P02: Key hierarchy and derivation
- source: RFC section 3.2
- type: protocol
- content: A process MUST NEVER transmit its master identity seed `S_root` or derived private heap keys. The client sends `Tauth` before `Tattach`; local `factotum` runs an authenticated key exchange (Noise handshake or mutual DP9IK ticket exchange) with the remote `factotum`, yielding `S_session`. Transient key `K_transient = HKDF-Expand(S_session, "9P-AEAD-FID" || fid, 32)` (RFC 5869 HKDF-SHA256), described as bound to the connection and direction. Object key `K_file = HKDF-Expand(K_user, Qid.path || Qid.vers, 32)`. On `Tclunk` of a channel both endpoints drop `K_transient` and zeroize (`Zeroizing<[u8; 32]>`). [see W4]

## RFC-P03: Wire envelope and field definitions
- source: RFC sections 4.1, 4.2, 4.3
- type: protocol
- content: Field order in the diagram: Size (4B, LE, total length including Size, header, TLVs, ciphertext and MAC); Type (1B); Tag (2B LE; `0xFFFF` NOTAG reserved for `Tversion`/`Rversion`); Pad (1B); FidHandle (8B); Monotonic Sequence Nonce (8B LE); ExtMask (2B LE); ExtTLVs Length (2B LE, MUST be 0 when ExtMask is 0); ExtTLVs payload (variable, 64-bit aligned records parsed zero-copy); ciphertext payload (ChaCha20; MUST be 4KB-aligned in `Twrite`, `Rread`, `Tstream`); Keyed BLAKE3 MAC (32 bytes). Sequence rule: senders increment by exactly 1 per packet per connection and direction; receivers MUST reject any packet that does not match the expected next value. AAD for the MAC (section 4.3) is `Size || Type || Tag || FidHandle || SeqNonce || ExtMask || ExtTLVs`. On MAC failure the receiver MUST terminate the connection and zero channel state without parsing any bytes. [see W1]

## RFC-P04: ExtMask bits and TLV definitions
- source: RFC section 4.4
- type: protocol
- content: `EXT_DEADLINE 1<<0` (8-byte monotonic nanosecond deadline; server cancels expired work without `Tflush`), `EXT_CAP_TOKEN 1<<1` (32-byte HMAC-BLAKE3 capability token for pooled connections; O(1) validation against a Roaring Bitmap CRL), `EXT_QID_VOLATILITY 1<<2` (emitted on `Rwalk`/`Ropen`/`Rcreate`: `volatility_class` 1B with 0x20/0x40/0x80, `lease_generation` 8B, `granted_mode` 1B with 0 None, 1 SharedRead, 2 ExclusiveWrite), `EXT_STREAM_PRIO 1<<3` (`priority_tier` 2 bits P0_CONTROL=0, P1_METADATA=1, P2_BULK=2; `flow_credit` 30 bits in bytes), `EXT_OBINARY 1<<4` (aligned zero-copy binary structures from `/proc`, `#G`, `/net`, `telescope` instead of ASCII). [see I9]

## RFC-P05: FidHandle wire layouts and FidEntry
- source: RFC section 5
- type: schema
- content: "On the wire, every `FidHandle` occupies exactly 8 bytes." `StatefulHandle` (`#[repr(C)]`, 8B): `fid_key u32`, `ptr TinyPtr` (NonZeroU8 displacement), `shard_id u8`, `epoch u16`. `StatelessCap` (24B): `qid_path u64`, `qid_vers u32`, `qid_type u8`, `mode u8`, `epoch u16`, `mac_tag [u8;8]` (truncated keyed BLAKE3). RFC comment: "represented via token indirection or wire capability". `FidEntry` (`#[repr(C, align(64))]`, 64B): `fid_key u32`, `client_id u32`, `qid_path u64`, `offset u64`, `qid_vers u32`, `iounit u32`, `lease_gen u32`, `ring_slot u32`, `dev_handler u16`, `epoch u16`, `qid_type u8`, `open_mode u8`, `flags u16`, `_reserved [u8;16]`. Stateless files use zero server FID memory. `slot = ((b << 4) + (ptr.get() - 1)) & slot_mask`. [see W2]

## RFC-P06: Opcode registry
- source: RFC section 6.1
- type: protocol
- content: T/R opcodes: Tversion 100/101, Tauth 102/103, Tattach 104/105, Twalk 110/111, Topen 112/113, Tcreate 114/115, Tread 116/117, Twrite 118/119, Tclunk 120/121, Tremove 122/123, Tstat 124/125, Twstat 126/127, Tcompound 130/131, Tstream 132/133, Tlease_revoke 134/135 (response Rlease_ack), Treclaim 136/137. Base 9P2000 set preserved. [see I10]

## RFC-P07: New message formats and semantics
- source: RFC section 6.2
- type: protocol
- content: `Tcompound`: `size[4] Tcompound tag[2] num_ops[2] op_stream[...]`; `Rcompound`: `size[4] Rcompound tag[2] num_completed[2] status[2] result_stream[...]`. Server keeps a `CURRENT_FID` register; sub-op FID `0xFFFFFFFE` (`NOFID_CURRENT`) substitutes the result of the previous op. Sequential evaluation; on first error stop, roll back where applicable, return error status and `num_completed`. `Tstream`: `size[4] Tstream tag[2] fid_handle[8] stream_id[4] flags[2] offset[8]`; `Rstream`: `size[4] Rstream tag[2] stream_id[4] chunk_seq[8] count[4] data[...]`. Sliding-window flow control via `EXT_STREAM_PRIO`; senders MUST NOT exceed granted credit; window updates are empty `Rstream` frames carrying credits. `Tlease_revoke`: `size[4] Tlease_revoke tag[2] qid_path[8] lease_gen[8] target_mode[1] num_blocks[4] [block_ranges]`; `Rlease_ack`: `size[4] Rlease_ack tag[2] qid_path[8] lease_gen[8] client_status[1]`. Clients MUST purge or flush the 4KB blocks, suspend speculation on the revoked ranges, and emit `Rlease_ack`; server blocks the conflicting write until all acks arrive or the lease timeout expires. `Treclaim`: `size[4] Treclaim tag[2] old_handle[8] target_epoch[2] client_token[32]`; `Rreclaim`: `size[4] Rreclaim tag[2] new_handle[8] recovery_status[2]`. After reboot the server advertises an incremented epoch in `Rversion` and opens a grace period (default 30 seconds); clients replay `StatefulHandle`s via `Treclaim`; restored FIDs go into the shard-local `reclaimed_fids` bitmap; at window close `OrphanedHandles = active_fids \ reclaimed_fids` are freed and keys zeroized.

## RFC-P08: Volatility rules and ClientSet
- source: RFC sections 7.1, 7.2
- type: protocol
- content: `QTIMMUTABLE`: clients MAY cache indefinitely and MUST NOT issue `Tstat`/`Twalk` revalidation; contiguous immutable blocks collapse into Run containers in `SuperpageResidency`. `QTLEASED`: read lease allows local cached reads; exclusive write lease allows buffering writes in uncommitted shadow frames; revoked asynchronously on conflict. `QTVOLATILE`: clients MUST NOT cache data, read-ahead or directory lookups. `ClientSet` inline phase occupies 24 bytes inside the 64-byte `LeaseRecord` with zero heap use; above 4 clients converts to Roaring; write-lease upgrade removes the writer (`sharers.remove(writer_id)`) to get the exact recipient list for targeted `Tlease_revoke`.

## RFC-P09: Legacy gateway negotiation and RFNOLEGACY
- source: RFC section 10
- type: protocol
- content: Server inspects `Tversion`: `"9P2000.aead"` selects the native engine; `"9P2000"`, `"9P2000.u"`, `"9P2000.e"`, `"9P2000.L"` route to the legacy gateway (`port/src/legacy_9p.rs`). Ingress: read into a bounce frame, encrypt under a synthetic session key derived via `factotum`, commit to `BackingStore`, sanitize bounce frame with `Zeroize` plus `CLFLUSHOPT`. Egress: decrypt from `BackingStore` into the bounce frame, transmit, scrub immediately. `rfork(RFNOLEGACY)` forbids mounting, binding or walking channels mediated by the gateway.

## RFC-P10: Security considerations and IANA
- source: RFC sections 11, 12
- type: nfr
- content: Keystream reuse: senders MUST keep a strictly monotonic 64-bit sequence counter; near 2^64 - 1 the channel MUST re-key in band before sending more. Side channels: all kernel crypto in the fault path MUST restore and scrub `ymm0`-`ymm5` via `VPXOR`; frames reassigned across `KeyID` domains MUST be hardware-zeroized and flushed with `CLFLUSHOPT`. Header fields, FIDs, tags and sequence numbers MUST be authenticated as AAD; MAC failures MUST be rejected without partial parsing. Lease DoS: servers MUST enforce a revocation timeout (default 5 seconds); on missing `Rlease_ack` the server revokes unilaterally and drops the client channel. IANA: dialect string `"9P2000.aead"`, default TCP/UDP port 564, discovery endpoints IPv4 `239.9.9.9` port 9999 and IPv6 `ff02::9:9` port 9999 (link-local). Normative references: RFC 2119, 8174, 5869, 8439. Informative: 9P2000, BLAKE3, TPHT-2026 (arXiv:2607.28892v1), Roaring-2016, OpenAddressing-2025 (arXiv:2501.02305v2).

---

# Group 4: Permissions, Cluster Topology and Test Specification (precedence 3)

## PT-C01: minimega testbed and node roles
- source: PT sections 1, 2
- type: nfr
- content: Verification harness uses `minimega` (QEMU/KVM) with declarative `vm config` templates, `tap`/`mesh` VLANs, and `vm cc` command-and-control. 12 nodes: 1 Fossil server `fossil01` (4096 MB, 4 vCPU, dual-homed VLAN 10 and 20, authoritative `/`, `/adm/users`, `QTLEASED` lease handling, GFS snapshots); 3 Venti CAS nodes `venti01`-`venti03` (2048 MB, 2 vCPU, VLAN 10, NVMe arena, BLAKE3 arenas, Chained-TPHT index, 64KB super-blocks and 1KB tag slabs); 6 stateless diskless compute nodes `cpu01`-`cpu06` (2048 MB, 2 vCPU, VLAN 20, netboot kernel `r9-9load` with initrd `r9-bootproto.cpio`, mount Fossil at `/` via 9P2000.aead); 2 terminal nodes `term01`-`term02` (VLAN 20 and 30, `virtio-vga-gl`, `rio`, `libdraw`, `chat-rs`, `#G` Venus). VLAN 10 storage fabric, VLAN 20 9P2000.aead grid fabric, VLAN 30 terminal display fabric. Script saved at `/opt/r9/minimega/cluster_topology.mm`; all nodes use `-cpu host -machine q35,accel=kvm`. [see W10]

## PT-C02: User registry and group hierarchy
- source: PT section 3
- type: schema
- content: Persona table (UID, name, node, default group): 1001 larry (cpu01, term01, dev), 1002 curly (cpu02, dev), 1003 moe (cpu03, term02, stooges; leader of stooges), 1004 bob (cpu04, audit; read-only auditor), 1005 eve (cpu05, none; adversary), 2001 alice (cpu06, arch-team; federated peer via `archive.r9`), 0 glenda (fossil01, sys; administrator), 9999 none (anonymous world). `/adm/users` format `uid:name:leader:members` on `fossil01` (domain `lab.r9`), entries: `0:adm:adm:glenda,sys`, `1:sys:sys:glenda`, `9999:none:none:`, `1001:larry:larry:`, `1002:curly:curly:`, `1003:moe:moe:`, `1004:bob:bob:`, `1005:eve:eve:`, `2001:alice:alice:`, `1010:stooges:moe:larry,curly,moe`, `1020:pair_ab:larry:larry,curly`, `1030:dev:glenda:larry,curly,moe,bob`, `1040:audit:glenda:bob,glenda`. Groups are users that contain member names, governed by a leader. [see W8, W9]

## PT-T01: Test Suite 1, discretionary file sharing (Fossil VFS)
- source: PT section 4, Tests 1.1 to 1.3
- type: nfr
- content: 1.1 Asymmetric pairwise sharing: larry creates `/usr/larry/shared_ab/spec.txt` containing `Confidential_AB_Payload_9481`, `chgrp pair_ab`, dir mode 750, file mode 640. Assert curly read succeeds with exact string; curly write fails `permission denied`; moe list and read fail `Eperm`; eve read fails `Eperm`. 1.2 Symmetric team collaboration: moe creates `/usr/moe/collab/plan.txt`, group `stooges`, dir 770, file 660; larry and curly appends succeed and all three signatures appear in chronological order; bob read fails `Eperm`. 1.3 Group leadership: only a leader may change membership; larry (leader of `pair_ab`) adding bob via `fossil/conf` succeeds; curly removing larry from `pair_ab` and adding eve to `stooges` fail with `not group leader`.

## PT-T02: Test Suite 2, distributed leases and cache coherence
- source: PT section 4, Tests 2.1 to 2.3
- type: nfr
- content: 2.1 Concurrent read leases: cpu01-03 read `/usr/moe/collab/plan.txt`; `fossil01` registers node IDs in `LeaseRecord.readers`; `ClientSet` stays `Inline { count: 3, ids: [...] }` with no heap allocation; later reads served from local DRAM with 0 network packets. 2.2 `Tlease_revoke`: curly opens for write; `fossil01` upgrades to `ExclusiveWrite` and sends `Tlease_revoke` to cpu01 and cpu03; cpu01 clears the block in its `valid_blocks` bitmap; larry's next read triggers an on-demand fetch reflecting curly's modification. 2.3 Anti-splicing and replay: tap on VLAN 20 duplicates a `Twrite`; replay with stale nonce fails the `SeqNonce <= last_seen` check and is dropped; a payload moved between channels by editing the cleartext FID fails Keyed BLAKE3 AAD MAC verification.

## PT-T03: Test Suite 3, multi-domain federation (lab.r9 vs archive.r9)
- source: PT section 4, Tests 3.1, 3.2
- type: nfr
- content: 3.1 Factotum multi-key negotiation: larry loads two `p9sk1` keys (`dom=lab.r9`, `dom=archive.r9`), mounts `/net/tcp!fossil01!564` at `/n/lab` and `/net/tcp!venti03!564` at `/n/archive`; assert separate 9P2000.aead links with distinct transient keys and concurrent access without credential collision. 3.2 Unregistered cross-domain attenuation: alice (from `archive.r9`) mounts `fossil01` on `lab.r9`; ticket authenticates but no local `/adm/users` record exists, so the channel maps to UID 9999 `none` and private files return `Eperm`. [see W9, W10]

## PT-T04: Test Suite 4, kernel memory micro-isolation
- source: PT section 4, Tests 4.1, 4.2
- type: nfr
- content: 4.1 Sibling cryptographic partitioning: harness spawns two children with `rfork(RFPROC | RFMEM | RFCKEY)`; child 2 dereferences child 1's private heap; assert MMU trap via independent CR3, direct-frame read under `K_RW_B2` yields random bytes that fail the BLAKE3 MAC check, and the kernel terminates child 2 with note `sys: crypto integrity violation`. 4.2 Read-only base enforcement: after `rfork(RFROBASE)`, a write into `Block A` (`SG_TEXT`) traps via hardware PKEY 1 / `WRPKRU` in about 20 cycles without touching DRAM.

## PT-T05: Test Suite 5, mail and chat visibility (chat-rs)
- source: PT section 4, Tests 5.1, 5.2
- type: nfr
- content: 5.1 Direct message isolation: larry to curly via `upas/send curly` appears in curly's transcript under `[USERS] -> larry` and is absent from moe, bob, eve spools. 5.2 Dual visibility: moe sends to larry with CC `stooges` (`upas/send larry stooges`); larry sees it in `[USERS] -> moe` (badge `(to you, cc: stooges)`) and in `[PERSONAL GROUPS] -> stooges`; curly sees it in `[PERSONAL GROUPS] -> stooges` (badge `moe -> stooges (to: larry)`); bob and eve see nothing.

## PT-T06: In-guest script and cluster driver
- source: PT sections 5, 6
- type: nfr
- content: `/sys/src/tests/perm_test.rc` (rc) defines `assert_ok` and `assert_fail` and covers Suites 1 and 2 only (uses `rfork u; user=<name>` to act as each persona; payload strings differ from the matrix, `Payload_AB_7721` and `Plan_Alpha`). Host driver `/opt/r9/scripts/run_cluster_tests.sh` (bash, `set -euo pipefail`) reads the topology, sleeps 15s, pings `venti01`-`venti03` from `fossil01` with `venti/ping`, copies `/opt/r9/conf/users` to `/adm/users`, runs `perm_test.rc` on `cpu01` and `federation_test.rc` on `cpu06`. `federation_test.rc` is referenced but not defined in the source. [see I12]
