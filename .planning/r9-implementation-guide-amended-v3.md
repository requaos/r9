# r9 Implementation Guide (Amended Specification v3)

> **Architectural Roadmap, Landing Sites, and Audited Implementation Blueprint for a Secure, Pure-Rust Port of Plan 9 and Harvey**  
> *Targeting `requaos/r9` (commit `c787ce9`) and `requaos/harvey` (commit `a6b7e6a9`)*  
> *November 20, 2026*

---

## 1. Scope and How to Use This Guide

This document is the **authoritative implementation specification** for the `r9` operating system. It translates the design decisions and theoretical models of the *Plan r9 Architecture Compendium (v2.1)* into actionable repository landing sites, audited data structure definitions, prioritized engineering phases, and verification exit criteria.

### The Live Target Repositories

All paths, module names, system call bitflags, and crate layouts in this guide are pinned against the live donor trees:
1. **`requaos/r9` at `c787ce9`:** The active pure-Rust kernel port. Boots Harvey's `amd64` userspace on QEMU `q35` with an interactive shell, CPU server, and `rio` window manager.
2. **`requaos/harvey` at `a6b7e6a9`:** The 64-bit Plan 9 4th Edition derivative in C. Provides verified source truth for `sys/src/9k/` kernel drivers, 35 core libraries, and userland binaries.

---

## 2. Reconciled Decisions & Architectural Principles

The following architectural decisions are reconciled and non-negotiable across the implementation:

| Architectural Dimension | Reconciled Decision | Implementation Landing Site | Rationale & Operational Invariant |
| :--- | :--- | :--- | :--- |
| **Build Priority** | **Elevated 9P & `ds/` Upfront** | `ds/`, `crypto/`, `port/src/fid.rs`, `port/src/chan.rs` | Plan 9 is fundamentally defined as a network file fabric. Core data structures (`ds/`) and `9P2000.aead` wire handling are built in **Phase 1 & Phase 2**. |
| **Executable Binary Format** | **Plan 9 `a.out` exclusively** | `x86_64/src/exec.rs`, `xtask/src/elf2aout.rs` | `a.out` is the sole binary format loaded by the kernel. Contrib code and 9front utilities run unmodified. ELF emitted by linkers is converted via in-tree tooling. |
| **System Call Interface** | **Strictly Closed Call List** | `sys/include/libc.h`, `sys/src/syscall.rs` | No POSIX emulation calls or `mmap` extensions are permitted. Software adapts to Plan 9 primitives (`rfork`, `bind`, `mount`, 9P). Crypto extensions add 3 flag bits to `rfork` and zero syscalls. |
| **Native Rust Runtime** | **r9 `std` Crate (`sys/src/libstd_r9/`)** | Milestone STD (Post-Milestone S, Pre-Track P) | Full Rust `std` implementation wrapping Plan 9 syscalls (`/net`, `rfork(RFMEM)`, 9P file ops). Eliminates `#![no_std]` requirement for Harvey userland and native applications. |
| **C Toolchain & Compilers** | **Ported Plan 9 C Compilers (`6c`, `6a`, `6l`)** | Track P (Stage P3) | Plan 9 C compilers are ported to Rust to ensure native contrib code remains buildable directly on r9. Harvey's C compilers serve as bootstrap tools until parity is proven. |
| **Dual C Runtime Model** | **Segregated Classic APE & APEX** | `sys/src/ape` (APE) and `vendor/apex` (APEX) | Classic APE (compiled via `6c`) services Plan 9 contrib software (demonstrated by Ghostscript). Vendored APEX (compiled via Clang) services LLVM/C++ tooling. The two runtimes never mix in a single binary. |
| **Kernel Safety Boundary** | **Port Crate Safe Invariant** | `port/` (`#![forbid(unsafe_code)]`) | Low-level hardware operations (`WRPKRU`, `MOVNTDQ`, `CLFLUSHOPT`, inline assembly, raw atomics) are restricted to `x86_64/`, `aarch64/`, and `ds/src/tpht/` behind audited `// SAFETY:` blocks. |
| **Memory Isolation Model** | **Independent CR3 + Intra-Process PKEYs** | `x86_64/src/mmu.rs`, `port/src/segcrypt.rs` | Sibling processes operate under distinct hardware page-table roots (CR3), preventing PKEY capacity exhaustion. Hardware PKEYs enforce read-only base permissions locally within ~20 cycles. |
| **Primary Concurrent Hashing** | **Succinct TPHT (Flattened & Chained)** | `ds/src/tpht/` | Replaces Funnel Hashing in the Page Cache, 9P FidTable, and Venti Index. Single cache-line hits via Feistel quotienting; 1-byte tiny pointers yield single-read dereferencing (`deref_tiny`). |
| **Integer Set Algebra** | **Hybrid Roaring Bitmaps** | `ds/src/roaring.rs` | Vectorized frame tracking, $O(1)$ 2MB superpage promotion detection via `SuperpageResidency`, 9P priority tag scheduling (`P0`–`P2`), deadline wheels, and exact union layer filters. |
| **Distributed Wire Security** | **Native `9P2000.aead` Transit** | `port/src/chan.rs`, `port/src/fid.rs`, `crypto/src/session.rs` | Authenticated Associated Data (AAD) envelope with `ExtMask`, monotonic sequence nonces, hybrid `FidHandle`, and four new message pairs (`Tcompound`, `Tstream`, `Tlease_revoke`, `Treclaim`). Cleartext fallbacks use pinned bounce buffers. |
| **3D Graphics Architecture** | **Plan 9 `#G` Synthetic Device** | `port/src/devgpu.rs`, `kern/src/gpu.rs` | VirtIO-GPU Venus transport serializing Vulkan streams over `/dev/gpu/` file nodes (`clone`, `ctl`, `cmd`, `blob`, `fence`). Zero-copy window presentation via `draw(3)` host texture blits. |
| **Cluster Service Discovery** | **Multicast Beacons & Staged `/srv` Registration** | `sys/src/cmd/discover/`, `sys/src/cmd/beacon/`, `port/src/chan.rs` (`#M`) | Structured descriptors (`class:instance[+attrs]`) over dual-stack IPv4/IPv6 multicast (`DiscoveryBeaconV2`). Two-tier degradation (30s `unhealthy` / 5m `dead`) shields `#M` replica routing without premature connection drops. |

---

## 3. Implementation Guide Audit & Critical Corrections Ledger

The audit conducted across live repositories identified 17 critical defects that must remain permanently corrected in all subsequent PRs:

| Subsystem Component | Identified Defect in Initial Sketches | Operational Consequence | Engineered & Verified Replacement |
| :--- | :--- | :--- | :--- |
| **`crypto/src/kdf.rs`** | Placeholder XOR fold in HMAC routine | Insecure derivation; child keys were trivial, reversible functions of parent seeds | RFC 5869–compliant HKDF-SHA256 engine in `crypto/src/kdf.rs` pinned against RFC 4231 vectors. |
| **`crypto/src/key.rs`** | Read-only child derived a new $k_{\text{dec}}$ on `rfork` | Child could not decrypt shared Block A ciphertext sealed by the parent | Sibling processes inheriting shared read-only base code retain the parent's $k_{\text{dec}}$ directly; private segments derive isolated keys. |
| **`crypto/src/simd.rs`** | Decryptor copied ciphertext; MAC was an XOR fold | Zero encryption performed; data remained exposed in plaintext | Scalar reference in `crypto/src/page.rs`; vector fast path in `x86_64/src/cryptpage.rs` implementing verified ChaCha20. |
| **`crypto/src/simd.rs`** | Unsound separate `asm!` blocks without clobbers | Compiler could reorder/clobber vector registers; AVX-512 VEX writes zeroed upper ZMM registers | Single unified `global_asm!` routine saving `ymm0`–`ymm5`; AVX-512 state is explicitly disabled in XCR0. |
| **`crypto/src/simd.rs`** | Decrypt-then-MAC operation ordering | Plaintext read from DRAM after non-temporal stores; $k_{\text{dec}}$ reused as MAC key | Encrypt-then-MAC: verify ciphertext in cache before decrypting into shadow frames; dedicated MAC keys. |
| **`port/src/seg.rs`** | Broken chronological stride tracking | Ring buffer compared fixed slots (1-0 and 2-1); sequential streams detected on only 1 fault in 3 | Corrected chronological ring buffer evaluation in `port/src/segcrypt.rs` comparing oldest, middle, and newest entries. |
| **`port/src/shadow.rs`** | Unconditional `CLFLUSHOPT` instruction | Kernel crashed under QEMU TCG lacking `CLFLUSHOPT`; skipped same-domain frame recycling | CPUID leaf 7 EBX bit 23 probe; explicit `recycle_within` path for clean, same-domain physical frames. |
| **`port/src/page.rs`** | Address arithmetic operator precedence bug | Multiplication bound tighter than bitwise OR: `(chunk << 16) \| offset * 4096` corrupted addresses | Fully parenthesized expression: `(((chunk as usize) << 16) \| (offset as usize)) * 4096`. |
| **`port/src/page.rs`** | Pinned frames returned as eviction candidates | Pinned DMA and crypto buffers could be unmapped and overwritten mid-flight | Comprehensive container materialization in `fallback_difference`; pinned frames are mathematically excluded. |
| **`port/src/superpage.rs`** | Residency tracker lacked reset and demotion | Structure was 72B (padded to 128B); counter drifted; no demotion path existed | Exact 64B cache line; bit clearing and atomic counter reset supported on large-page demotion. |
| **`ds/src/funnel.rs`** | Tombstone reuse and correlated tier hashing | Dead slots resurrected stale values; single hash across tiers induced massive collisions at 65% load | Replaced in primary paths by TPHT; Funnel Hashing is restricted to write-once ephemeral scratchpads. |
| **`ds/src/scoreindex.rs`**| 8-byte score truncation in Venti index | Different blocks sharing a 64-bit prefix collided, corrupting the content-addressed store | Chained-TPHT: 64-bit folded key serves as a table hint; full 32-byte BLAKE3 score is verified on candidate match. |
| **`crypto/src/session.rs`**| 9P key derived and discarded | Bare FID stored as raw pointer; FID 0 rejected as empty slot; zero wire authentication | Per-connection, per-direction keys derived during `Tauth` with strict 64-bit monotonic sequence nonces. |
| **`kern/src/gpu.rs`** | Incorrect VirtIO feature bit (`1 << 1`) | Bit 1 is EDID; driver incorrectly assumed it had negotiated Venus | Negotiates bits 0, 3, and 4 (`VIRGL`, `BLOB`, `CONTEXT_INIT`); selects Venus capability set 4 on context creation. |
| **`port/src/devgpu.rs`** | Context IDs discarded in directory walk | Every client forced into context 0; raw user bytes became kernel VirtIO headers | Context ID packed into upper 24 bits of QID path; kernel constructs and validates all VirtIO packet headers. |
| **`kern/src/notes.rs`** | Unchecked user stack pointer on note delivery | Malicious processes could aim SP into read-only Block A, corrupting memory on trap | Delivery requires `SegKind::Stack`; checked arithmetic guarantees frame lies strictly within stack bounds. |
| **`port/src/access.rs`** | `UserSlice` duplicated existing arch abstractions | Duplicated existing `x86_64/src/usermem.rs` while bypassing memory residency checks | Folded into existing `UserSlice` / `UserSliceMut` behind `UserAddressSpace` trait; residency enforced on demand. |

---

## 4. Hardware-Isolated Cryptographic Memory & Zero-Tax Paging

The memory isolation subsystem enforces process separation through cryptographic capabilities, eliminates redundant memory zeroing, and protects CPU cache hierarchies from pollution during demand-paging traps.

### 4.1 Compile-Time Typestates & Key Capabilities

Memory segments in `port/src/segcrypt.rs` are governed by compile-time capability tokens wrapping sensitive key material in self-zeroizing structures:

```rust
use zeroize::Zeroizing;

pub const KEY_LEN: usize = 32;

#[derive(Clone)]
pub enum KeyCapability {
    ReadOnly {
        k_dec: Zeroizing<[u8; KEY_LEN]>,
        blake3_verify: Zeroizing<[u8; KEY_LEN]>,
    },
    ReadWrite {
        k_dec: Zeroizing<[u8; KEY_LEN]>,
        k_mac: Zeroizing<[u8; KEY_LEN]>,
    },
}
```

### 4.2 Dual-Path Store Routing & Pipeline Hiding

When the #PF handler decrypts a 4KB page, writing all 64 cache lines using non-temporal streaming stores (`MOVNTDQ`) incurs an immediate 40–100ns stall when user execution resumes, because the specific 64-byte line referenced by the instruction pointer or stack pointer must be refetched from DRAM.

The kernel implements **Dual-Path Store Routing**:
1. The 64-byte line containing the exact faulting address (`fault_addr & !0x3F`) is written using standard cached stores (`VMOVAPS`), ensuring it sits hot in L1/L2 cache when user space resumes.
2. The remaining 4,032 bytes (63 cache lines) are written using non-temporal stores (`MOVNTDQ`), bypassing L1/L2 and avoiding cache pollution.
3. **Interleaved SFENCE Hiding:** An `SFENCE` is issued immediately following the store loop. The CPU drains write-combining buffers concurrently in the background while the scalar core computes the BLAKE3 tree-MAC root and restores vector registers, absorbing write-drain latency completely.

### 4.3 Restricted Vector ABI (`ymm0`–`ymm5`)

Invoking the standard kernel `XSAVE`/`XRSTOR` sequence costs ~300 CPU cycles. The vector cryptographic fast path in `x86_64/src/cryptpage.rs` defines a hand-rolled vector ABI restricted strictly to 6 registers:
* `ymm0`–`ymm3`: Hold the four 256-bit ChaCha20 state rows.
* `ymm4`–`ymm5`: Provide the two temporary registers required for quarter-round lane rotates (12-bit and 7-bit shifts) and lane-splitting.
* Prior to vector execution, `ymm0`–`ymm5` are spilled to the kernel execution stack via `VMOVUPS` (~192 bytes, ~15 cycles).
* Upon completion, registers are zeroed with `VPXOR` to eliminate speculative data leakage and restored from the stack (~15 cycles), yielding a total round-trip context switch cost of **~30 cycles**.
* **AVX-512 Disabled:** AVX-512 state is explicitly disabled in XCR0. This prevents VEX-encoded 256-bit operations from zeroing the upper halves of 512-bit ZMM registers.

### 4.4 Allocator Bypass: `UninitializedFrame`

Standard operating system physical memory allocators zero memory prior to returning frames to prevent inter-process information disclosure. This induces a **Write-Write Tax**: writing 4,096 zeros to DRAM (~1,000 cycles), followed immediately by ChaCha20 writing 4,096 bytes of decrypted plaintext into the same memory (~1,500 cycles).

r9 eliminates this redundancy via the `UninitializedFrame` primitive:
* Physical memory is popped directly from free pools without scrubbing.
* The hardware Page Table Entry (PTE) remains strictly unmapped (`PROT_NONE`) while the frame is in an uninitialized state.
* ChaCha20 streams decrypted plaintext across all 4,096 bytes from index 0 to 4,095, completely and unconditionally overwriting all residual stale data.
* Only after BLAKE3 integrity verification passes is the frame wrapped as an immutable `ShadowFrame` and published to the hardware page tables.

---

## 5. Succinct TPHT & Roaring Set-Algebra Engine (`ds/` Crate)

To reconcile high-throughput concurrency, bounded lookup latency, and minimal memory overhead, the `ds/` crate implements **Succinct and Fast Tiny Pointer Hash Tables (TPHT)** alongside **Roaring Bitmaps**.

### 5.1 One-Round Feistel Quotienting Routine

Traditional hash tables store full 64-bit keys in every bucket. In `ds/src/tpht/flattened.rs`, key storage is eliminated entirely using a bijective one-round Feistel permutation:
$$F(L, R) = (R,\; L \oplus f(R))$$
where $L$ and $R$ represent the upper and lower 32 bits of the 64-bit key, and $f(R)$ is a fast non-linear mixing function:
$$f(R) = (R \oplus (R \gg 16)) \cdot \text{0x85EBCA6B} \oplus \dots$$

Given only the home block index $q$ and the stored remainder $r$, the original 64-bit key is reconstructed losslessly in 3–4 ALU cycles:
$$R = P \pmod{2^{32}}, \quad L' = \lfloor P / 2^{32} \rfloor, \quad L = L' \oplus f(R), \quad \text{Key} = (L \ll 32) \mid R$$
This saves ~20–24 bits per entry in the Kernel Page Cache, completely offsetting metadata and pointer overhead.

### 5.2 64-Byte Hardware-Aligned `TphtBucketHeader` & `FlattenedTphtShard`

The primary table structure in `ds/src/tpht/flattened.rs` packs home buckets into exact 64-byte hardware cache lines (`#[repr(C, align(64))]`):
* `seq_lock` (`AtomicU32`, 4 bytes): Odd indicates an active writer; even indicates a stable generation for lock-free optimistic reads.
* `occupied_mask` (`u16`, 2 bytes): Bitmask of active slots in the bucket (bits 0..15).
* `spilled_mask` (`u16`, 2 bytes): Bitmask of slots whose tiny pointer exceeds the home window (`ptr > 16`).
* `quotients` (`[u16; 16]`, 32 bytes): 16-bit Feistel quotients packed for parallel SIMD comparison.
* `tiny_ptrs` (`[u8; 16]`, 16 bytes): 8-bit displacements (`1..=255`, 0 = empty) addressing slots in the flat value arena.
* `clock_nibbles` (`AtomicU64`, 8 bytes): Sixteen 4-bit clock/aging counters packed into a single 64-bit atomic integer.

**Lookup Fast Path:** >90% of lookups resolve in the home block. When querying with `(key, tiny_ptr)` via `deref_tiny`, the lookup completes in **strictly one memory access** with zero header probes.

### 5.3 Two-Choice Dereference Banks (`ChainedTphtOverflow`)

When a `FlattenedBlock` exceeds its 4-slot capacity, entries spill into secondary **Dereference Banks** in `ds/src/tpht/chained.rs`:
* Each bank is an arena of 255 compact overflow nodes indexable by a 1-byte tiny pointer.
* **Two-Choice Load Balancing:** Each home block maps to two candidate bank pairs ($\text{Bank}_0$ and $\text{Bank}_1$). New overflow nodes are allocated in the bank with greater free capacity, preventing localized slot exhaustion without global table reorganization.
* **Linearizable Publication:** Slots are claimed atomically using a two-phase publication protocol with a `WRITING` sentinel (`0xFF`), ensuring concurrent readers never observe partially initialized entries.

### 5.4 Roaring Bitmaps for Integer Set Algebra

Roaring Bitmaps (Lemire et al., arXiv:1603.06549) govern integer set algebra in `ds/src/roaring.rs`:
* **Physical Frame Allocator:** Tracks free clean, dirty, and pinned pools across 256MB DRAM chunks.
* **SIMD Eviction Harvesting:** The asynchronous High-Water Pager computes eviction candidates via vectorized set difference in <200 cycles: $\text{Evictable} = \text{Dirty} \setminus (\text{Pinned} \mid \text{Active})$.
* **$O(1)$ 2MB Superpage Promotion:** The `SuperpageResidency` structure maps 512 sub-pages (4KB each) into an 8-word (64-byte) cache-line aligned bitset. Transparent 2MB PMD promotion triggers in ~5 cycles when all 512 bits are set.
* **Small-Set `ClientSet` Optimization:** Readers on `QTLEASED` files use an inline enum for 1–4 clients, upgrading to `RoaringBitmap` for large reader cohorts ($10^4+$ nodes).

---

## 6. Distributed 9P Protocol, VFS Dispatch, QIDs/FIDs & Graphics

### 6.1 9P QID Architecture & Volatility Classes

A 9P resource is identified by a 13-byte `Qid` (`path: u64`, `vers: u32`, `qtype: u8`). In r9, synthetic device drivers multiplex operational state directly into the 64-bit `Qid.path`:
* **Synthetic `#G` GPU Routing:**
  $$\mathtt{Qid.path} = ((\text{ctx\_id} \ \& \ \mathtt{0xFFFFFF}) \ll 8) \mid (\text{node\_type} \ \& \ \mathtt{0xFF})$$
* **Cryptographic Object Binding:** When writing persistent files, the kernel derives an isolated object key bound to resource identity:
  $$K_{\text{file}} = \text{HKDF-Expand}(K_{\text{user}}, \mathtt{Qid.path} \parallel \mathtt{Qid.vers}, 32)$$

`Qid.qtype` extends mode bits with **three explicit Volatility Classes**:
1. **`QTIMMUTABLE` (`0x20`):** Content-addressed files (Venti blocks, shared binaries). Cached indefinitely in local DRAM/NVMe with zero revalidation round-trips.
2. **`QTLEASED` (`0x40`):** Shared mutable files (Fossil file trees). Cached locally until server pushes an asynchronous `Tlease_revoke`.
3. **`QTVOLATILE` (`0x80`):** Live synthetic files (`/dev/cons`, `/proc`, `#G`). Always routed directly to device handlers.

### 6.2 Hybrid `FidTable` & Single-Read Wire Resolution (`port/src/fid.rs`)

`FidHandle` splits 9P file identifiers into two wire representations:
* **`StatelessCap` (24 bytes):** Used for `QTIMMUTABLE` files and static directory walks. Self-contained HMAC-BLAKE3 token requiring **zero server RAM**.
* **`StatefulHandle` (8 bytes):** Used for mutable files, pipes, `/net`, and `#G` contexts. Embeds the server's 8-bit `TinyPtr`, `shard_id`, and `epoch`.
* **Single-Read `deref_tiny`:** Incoming requests resolve `FidEntry` in **strictly one memory read without hash probing**.
* **Failover Sweep:** Post-failover, un-reclaimed descriptors are swept in one SIMD ANDNOT operation: $\mathtt{Orphaned} = \mathtt{active\_fids} \setminus \mathtt{reclaimed\_fids}$.

### 6.3 Upgraded `9P2000.aead` Wire Envelope & Message Pairs (`port/src/chan.rs`)

Distributed transactions encapsulate data within an authenticated encryption envelope:
$$\text{Packet} = \underbrace{\text{Size (4B)} \parallel \text{Type (1B)} \parallel \text{Tag (2B)} \parallel \text{FidHandle (8B)} \parallel \text{SeqNonce (8B)} \parallel \text{ExtMask (2B)} \parallel \text{ExtTLVs}}_{\text{Authenticated Associated Data (AAD)}} \parallel \underbrace{\text{Ciphertext (ChaCha20)}}_{\text{4KB Aligned}} \parallel \underbrace{\text{MAC (32B)}}_{\text{BLAKE3 / Poly1305}}$$

* **Extension Bitmask (`ExtMask`):** Encodes `EXT_DEADLINE`, `EXT_CAP_TOKEN`, `EXT_QID_VOLATILITY`, `EXT_STREAM_PRIO`, and `EXT_OBINARY`.
* **Four New Message Pairs:**
  1. `Tcompound` / `Rcompound`: Atomic multi-operation pipelines via `CURRENT_FID`.
  2. `Tstream` / `Rstream`: Bidirectional and push streaming over `FidHandle`.
  3. `Tlease_revoke` / `Rlease_ack`: Asynchronous lease revocation with Roaring invalidation sets.
  4. `Treclaim` / `Rreclaim`: Post-failover descriptor recovery.
* **Legacy Gateway:** Mediated via non-pageable, pre-pinned kernel bounce buffers zeroized immediately after transmission. Sandboxed via `rfork(RFNOLEGACY)`.

### 6.4 Dynamic Cluster Service Discovery & Staged `/srv` Lifecycle (`cmd/discover`, `cmd/beacon`)

To eliminate manual network configuration (`/lib/ndb/local`) when adding heterogeneous compute, storage, and GPU nodes to a cluster, r9 implements autonomous dynamic service discovery integrated directly into Plan 9's `/net` and `/srv` abstractions:

* **Structured Resource Specifiers:** Replaces flat service strings with structured descriptors:
  $$\text{Resource Specifier} = \underbrace{\text{Class}}_{\mathtt{gpu},\,\mathtt{fs},\,\mathtt{venti}} \;:\; \underbrace{\text{Instance / Unit ID}}_{\mathtt{0},\,\mathtt{1},\,\mathtt{compute},\,\mathtt{main}} \;[\;+\; \underbrace{\text{Capabilities / Attributes}}_{\mathtt{venus,vram=24G,headless}}\;]$$
* **Canonical `/srv` Registration:** Nodes are exposed deterministically as `/srv/{hostname}_{class}_{instance}` (e.g., `/srv/gpu02_gpu_0` for display and `/srv/gpu02_gpu_1` for compute).
* **Dual-Stack Multicast Discovery (`DiscoveryBeaconV2`):**
  * Wire format: 192-byte packed binary struct (`magic: b"R9DSC2"`, sequence nonce, 16-byte native IPv6 or IPv4-mapped IPv6, port, family, attributes).
  * Multicast groups: IPv4 `239.9.9.9!9999` (TTL: 2 hops); IPv6 link-local `[ff02::9:9]!9999` (VLAN/L2 switch scope).
  * Plan 9 dial string format: Strictly enforces Plan 9 delimiter grammar: `connect [{ipv6}]!{port}\n` and `connect {ipv4}!{port}\n`.
* **Two-Tier Degradation Lifecycle & `#M` Invalidation:**
  1. **`Healthy` (0–30s Silence):** Registered in `/srv/{name}`, status file `/srv/{name}.status` contains `healthy\n`, asserted in `#M` client-side `healthy_replicas` Roaring Bitmap. Speculative parallel walks and hedged reads route to this node normally.
  2. **`Unhealthy` (30s–330s / +5m Silence):** `/srv/{name}.status` set to `unhealthy\n`; cleared from `#M`'s `healthy_replicas` bitmap, immediately halting new speculative walks, hedged reads, or load-balanced requests. The underlying TCP socket in `/srv/{name}` **remains open** to allow in-flight transactions to drain gracefully. Heartbeat arrival immediately restores the node to `Healthy`.
  3. **`Dead` (>330s Total Silence):** Connection descriptor closed, `/srv/{name}` and `/srv/{name}.status` unlinked via `remove()`, peer reaped from memory.

---

## 7. Storage Stack: Venti & Fossil Modernization

### 7.1 Venti Parallel BLAKE3 Content-Addressed Storage

* **Cryptographic Digests:** Replaces SHA-1 with parallel BLAKE3 Merkle-tree digests, providing 128-bit collision resistance and $>10$ GB/s tree hashing per core.
* **`VentiTphtIndex`:** Scores resolve to arena offsets via Chained-TPHT, where a 16MB head array sits permanently in L3 cache to service lookups wait-free.
* **Hash-Before-Compression Invariant:** Scores derive strictly from uncompressed blocks: $\text{Score} = \text{BLAKE3}(\text{Plaintext})$. Deduplication operates identically across nodes regardless of local compression, and Merkle tree roots remain stable across background re-encoding.

### 7.2 On-Disk Super-Blocks & Out-of-Band Tag Slabs

* **64KB Data Super-Blocks:** 16 contiguous 4KB pages aligned to 64KB flash erase boundaries.
* **1KB Tag Slabs:** Adjacent 1KB slabs containing sixteen 64-byte cache-line aligned `TagEntry` structures. Scanning metadata prefetches adjacent tags automatically, achieving 98.9% DRAM efficiency without pointer chasing.
* **Hardware Layout:** Records `blake3_tag: [u8; 32]`, `offset: u32`, `compressed_size: u16`, `original_size: u16`, `block_type: u8`, and `compression_algo: u8` (0=None, 1=Zstd).

### 7.3 Two-Tier Zstd Compression Lifecycle & Flash Health

* **Direct Zstd(3) Ingestion:** Fossil commits compress dirty blocks in-flight at Level 3. If `compressed_size >= original_size`, storage falls back to `COMPRESS_NONE` with zero CPU overhead. User writes hit Fossil's in-memory cache and return with zero compression latency.
* **Asymmetric Decompression:** Decompression executes at ~1.2–1.5 GB/s per core (~2.5–3.5 µs per 4KB block). Decompression scratchpads are pre-sized from `original_size` without memory reallocations.
* **Zero Rewrites for Live Arenas:** Daily (Son) and weekly (Father) snapshots stay at Level 3 for their lifespan. Re-encoding to Zstd Level 9–15 occurs **strictly during GFS Mark-and-Sweep consolidation** when an arena falls below 40% occupancy, eliminating flash write amplification.

### 7.4 Fossil GFS Retention & Mark-and-Sweep GC

* **GC Trigger:** Fossil transmits surviving BLAKE3 roots to Venti.
* **Mark Phase:** Venti traverses surviving Merkle trees, populating an ephemeral `ReferencedBlocks` Roaring Bitmap.
* **Sweep Phase:** Unreferenced blocks are identified via SIMD set difference in <200 cycles: $\mathtt{PurgeSet} = \mathtt{ArenaPresence} \setminus \mathtt{ReferencedBlocks}$.
* **Hardware TRIM:** Contiguous runs of purged blocks are deallocated via native NVMe Dataset Management (DSM) / TRIM commands, preserving SSD endurance.

---

## 8. Multi-Track Engineering Roadmap

The amended engineering order coordinates the foundational crates, 9P wire fabric, SMP, Harvey Rust port, and the self-hosting toolchain:

| Phase / Milestone | Deliverable Scope | Dependencies | Verification Exit Gate |
| :--- | :--- | :--- | :--- |
| **Phase 0** | Threat model doc, corrected C prototypes, reserve `rfork` flag bits 18–20 in `libc.h` | Milestone v2.1 closed | Maintainer confirms decisions; flag bits reserved. |
| **Phase 1** | `crypto/` & `ds/` foundations: RFC HKDF, ChaCha20, BLAKE3, `FlattenedTphtShard`, `RoaringBitmap`, `ClientSet` | Phase 0 | RFC vector pins pass; TPHT single-line hit tests pass on host toolchain. |
| **Phase 2** | Core 9P2000.aead Transport & `FidTable`: `FidHandle`, `FidShard`, `deref_tiny`, AAD envelope, monotonic nonces, Factotum `Tauth` | Phase 1 | Wire encoding/decoding tests pass; single-read dereferencing verified; anti-splicing AAD pins green. |
| **Phase 3** | Cryptographic Memory & Zero-Copy DMA Paging: `UninitializedFrame`, Dual-Path stores, interleaved SFENCE hiding, `ShadowFrame`, `StrideTracker` | Phase 2 | 9P packets DMA directly into `BackingStore`; demand `#PF` decrypts correctly; zero memory amplification. |
| **Phase 4** | **Distributed 9P Extensions & Cluster Service Discovery**: `Tcompound` pipelines, `Tstream`, `LeaseRecord`, `UnionMountTable`, `healthy_replicas` Roaring Bitmap, `DiscoveryBeaconV2` multicast wire protocol, `cmd/beacon` & `cmd/discover` (with two-tier 30s/5m degradation lifecycle and `#M` integration), legacy bounce gateway, `RFNOLEGACY` | Phase 3 | Atomic compound scripts pass; multi-client lease revocation tests green; discovery beacons dynamically populate `/srv` across IPv4/IPv6; 9front interoperability verified. |
| **Phase 5** | Process Lifecycle & Notes: `rfork` flags live (`RFCKEY`, `RFROBASE`, `RFRANDMEM`), stack boundary note delivery | Phase 4 | Sibling isolation pins; stack spoofing traps pass. |
| **Milestone S** | Multi-CPU SMP; retire single-CPU spinlock invariants across kernel | Phase 3, Phase 5 | Multi-CPU boot on QEMU `q35`; zero contention deadlocks. |
| **Milestone STD** | **r9 Native Standard Library (`sys/src/libstd_r9/`)**: Map `std::fs`, `std::net`, `std::thread`, `std::process`, `std::sync` to Plan 9 syscalls | Milestone S | Test suite for `std` passes; idiomatic Rust binaries run without `#![no_std]`. |
| **Track P** | Complete Rust port of Harvey kernel devices, 35 libraries, and userland commands in **idiomatic Rust via `libstd_r9`** | Milestone S, Milestone STD | System boots without C code; all Harvey tools compiled natively using `std`. |
| **Phase 6** | Storage Engine: BLAKE3 Venti CAS store, Chained-TPHT score index, Fossil GFS snapshotting, NVMe TRIM GC | Phases 2, 4, Track P | Venti block deduplication green; snapshot mark-and-sweep GC verified. |
| **Phase 7** | Graphics: 2D framebuffer, kernel `devdraw`, `#G` Venus, `draw(3)` zero-copy host blit bridge | Track P (`draw`/`rio`) | Hardware triangle rendered in `rio` window on QEMU. |
| **Phase 8** | Self-Hosting Rust Toolchain (`rustc`, `rust-lld`, APEX C runtime, ELF-to-`a.out`) | Track P, Milestone STD, Phase 6 | r9 compiles its own kernel and userland on r9. |

---

## 9. Milestone STD: r9 Native Standard Library (`sys/src/libstd_r9/`)

### 9.1 Strategic Re-sequencing: Why `std` Precedes Track P
Originally, Track P proposed porting Harvey’s userland tools (`rc`, `sed`, `awk`, `sam`, `rio`, `upas`) under `#![no_std]` with manual syscall wrappers, deferring a proper `std` target until Phase 8. This introduced a severe engineering anti-pattern: rewriting ~1.2 million lines of userland tools twice (first in low-level `#![no_std]` Rust, then modernizing them into standard Rust).

By slotting **Milestone STD** immediately following **Milestone S (Multi-CPU SMP)**:
1. **Kernel Primitives Are Ready:** Milestone S completes multi-core thread scheduling, memory locking, `/net` device operations, and 9P channels. All foundational kernel syscalls are fully operational under SMP.
2. **Idiomatic Userland Implementation:** Track P userland tools are authored directly in clean, idiomatic Rust from day one, leveraging `std::io::BufReader`, `std::collections::HashMap`, `std::sync::Arc`, and `std::fs::File`.
3. **Ecosystem Compatibility:** Standard crates from `crates.io` that require `std` (parsers, formatters, linear algebra, networking) compile cleanly for r9 userland without manual `#![no_std]` patching.

---

### 9.2 Exact Module Mapping for `sys/src/libstd_r9/`

`libstd_r9` acts as the Platform Abstraction Layer (PAL), implementing standard Rust interfaces directly on Plan 9’s closed system call interface without any POSIX translation layer:

```
sys/src/libstd_r9/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Crate root, exposes std module re-exports
    ├── pal/
    │   ├── mod.rs              # PAL initialization and platform types
    │   ├── fs.rs               # std::fs        -> 9P file ops (open, read, write, stat, remove)
    │   ├── net.rs              # std::net       -> /net/tcp, /net/udp conversational files
    │   ├── thread.rs           # std::thread    -> rfork(RFPROC | RFMEM | RFFDG)
    │   ├── process.rs          # std::process   -> rfork + exec(a.out), #c/wait
    │   ├── sync.rs             # std::sync      -> Atomic futex fast-path + #c/rendezvous
    │   ├── io.rs               # std::io        -> Plan 9 file descriptor read/write
    │   ├── alloc.rs            # std::alloc     -> sbrk(2) and segattach(2)
    │   ├── time.rs             # std::time      -> RDTSC + #c/time, #c/bintime
    │   ├── env.rs              # std::env       -> /env/ synthetic filesystem
    │   └── stdio.rs            # std::io::stdio -> fd 0, 1, 2 bindings
```

#### Detailed Subsystem Specifications

| Standard Module | Target Interface | Underlying Plan 9 / r9 Kernel Primitives | Operational Mechanics |
| :--- | :--- | :--- | :--- |
| **`std::fs`** | `File`, `OpenOptions`, `Metadata`, `ReadDir`, `DirEntry` | `open(2)`, `read(2)`, `write(2)`, `seek(2)`, `remove(2)`, `stat(2)`, `wstat(2)` | • Path traversal maps directly to 9P `Twalk` via `#M`.<br>• `Metadata` decodes Plan 9's machine-independent `Dir` wire format (`stat[s]`).<br>• `ReadDir` iteratively parses directory streams via `dirread(2)`.<br>• Access permissions map Plan 9 permission bits (`0o777`, `DMDIR`, `DMAPPEND`, `DMEXCL`). |
| **`std::net`** | `TcpStream`, `TcpListener`, `UdpSocket` | `/net/tcp`, `/net/udp` conversation directories | • **Connection:** `TcpStream::connect()` opens `/net/tcp/clone`, reads conversation ID `<n>`, and writes `connect <addr>!<port>` to `/net/tcp/<n>/ctl`. I/O reads/writes `/net/tcp/<n>/data`.<br>• **Listening:** `TcpListener::bind()` writes `announce <addr>!<port>` and `listen` to `ctl`, blocking on `/net/tcp/<n>/listen` to accept incoming conversation IDs.<br>• **Dual-Stack:** Native IPv6 (`[fe80::1]!564`) and IPv4 (`10.0.0.1!564`) parsing.<br>• **UDP:** Writes `headers` to `ctl` to enable 52-byte framed packet reads/writes on `data`. |
| **`std::thread`** | `Thread`, `Builder::spawn`, `JoinHandle` | `rfork(RFPROC \| RFMEM \| RFFDG)` | • Spawns a lightweight kernel process sharing virtual memory and file descriptor tables with parent.<br>• Thread stack is allocated via `segattach` with hardware guard pages.<br>• Thread-Local Storage (TLS) is addressed via `%gs` / `FS_BASE` MSR initialized in `x86_64/src/mmu.rs`.<br>• Thread termination calls `exits(0)` and signals parent atomic join handle. |
| **`std::sync`** | `Mutex`, `RwLock`, `Condvar`, `Once` | Atomic operations (`AtomicU32`) + `rendezvous(2)` / `#c/rendezvous` | • **Uncontended:** Lock acquisition completes in user space within ~12 cycles via `compare_exchange`.<br>• **Contended:** Yields execution by sleeping on `#c/rendezvous` or kernel `rendezvous(tag, val)` system call, avoiding busy spin loops and eliminating POSIX `pthread_mutex` overhead. |
| **`std::process`** | `Command`, `Child`, `ExitStatus` | `rfork(RFPROC \| RFFDG \| RFENVG \| RFNAMEG)` + `exec(2)` | • Process isolation: `rfork` forks descriptors, environment, and namespace according to builder configuration.<br>• Spawns Plan 9 `a.out` binaries via `exec(path, argv)`.<br>• Parent reads termination exit strings from `#c/wait` or `/proc/<pid>/wait`.<br>• Anonymous pipes (`pipe(2)`) wire stdin, stdout, and stderr. |
| **`std::alloc`** | `GlobalAlloc`, `System` | `sbrk(2)` and `segattach(2)` | • Primary heap pool expands memory dynamically using `sbrk(2)`.<br>• Large virtual buffers (e.g. GPU blobs, Venti indices) attach directly via `segattach(2)` with `RFCKEY` cryptographic encryption typestates. |
| **`std::time`** | `Instant`, `SystemTime`, `Duration` | Hardware `RDTSC` + `#c/time`, `#c/bintime` | • `Instant::now()` samples CPU timestamp counter (`RDTSC`) calibrated during kernel boot.<br>• `SystemTime::now()` reads nanoseconds since UNIX epoch from `#c/bintime`. |
| **`std::env`** | `args()`, `var()`, `set_var()`, `current_dir()` | `/env/` synthetic file tree & argument vectors | • Environment variables correspond 1:1 with files in `/env/<name>`. Reading `var("PATH")` reads `/env/path`. Setting `set_var("USER", "larry")` writes to `/env/user`.<br>• `args()` decodes the Plan 9 process stack frame populated during `exec(2)`. |
| **`std::io`** | `stdin()`, `stdout()`, `stderr()` | Pre-allocated file descriptors `0`, `1`, `2` | Direct unbuffered and buffered reads/writes over standard Plan 9 stream channels. |

---

## 10. Phase 8: Self-Hosting Toolchain Architecture

1. **C Runtime via APEX:** Harvey’s APEX (`Harvey-OS/apex`, MIT license) is vendored. It provides musl-based ANSI C over Plan 9 system calls without `mmap`. System call stubs are generated from `sys/src/libc/9syscall/sys.h` for Clang compatibility.
2. **Dual C Runtime Isolation:** r9 maintains two strictly segregated C environments:
   * **Classic APE:** Built by ported Plan 9 C compilers (`6c`, `6a`, `6l`) for native Plan 9 contrib software (demonstrated by Ghostscript).
   * **APEX:** Built by Clang for modern C++ tooling (demonstrated by LLVM).
3. **Linker & Output Format:** `rust-lld` emits static ELF binaries, which an in-tree conversion tool immediately wraps into Plan 9 `a.out` format.
4. **Zero `mmap` Dependency:** `rustc`'s `memmap2` metadata loader falls back to whole-file read buffers on r9, completely avoiding new kernel memory-mapping calls.

---

## 11. Testing, Verification Matrix & Risk Governance

Every phase adheres to strict repository verification discipline: a committed RED pin, the engineered implementation, GREEN verification twice, followed by pin advance.

| Verification Layer | Execution Harness | Target Coverage in Amended Plan |
| :--- | :--- | :--- |
| **Host Unit Pins** | `cargo xtask test` | `crypto/` vectors; `ds/` TPHT single-line hit models; `deref_tiny` single-read validation; `RoaringBitmap` SIMD set algebra; `StrideTracker` chronological sequence; `TagTable` bounds; page sealing round-trips. |
| **9P Protocol Pins** | `cargo xtask test-9p` | `9P2000.aead` wire round-trips; `ExtMask` parsing; `Tcompound` sequence validation; `StatelessCap` MAC verification; `FidShard` failover sweep (`active - reclaimed`); `DiscoveryBeaconV2` dual-stack multicast decode; staged degradation timing (30s `unhealthy` / 5m `dead`) and `#M` `healthy_replicas` bitmap invalidation. |
| **Lint Contracts** | `cargo xtask check`, `clippy` | `undocumented_unsafe_blocks = deny` on arch crates; `#![forbid(unsafe_code)]` on `crypto/`, `ds/`, and `port/`. |
| **Unsafe Audit** | `UNSAFE-AUDIT.md` | Reconciles every unsafe site across `x86_64/src/shadow.rs`, `cryptpage.rs`, and `ds/src/tpht/flattened.rs`. |
| **Boot Profiles** | `cargo xtask boot-test` | `q35lowmem` forcing eviction; `q35crypt` probing tamper kills, sibling isolation, note spoofing, and vector register preservation. |
| **Composed Gates** | Harvey flake apps | `boot-r9-x64` smokes, `net` smoke, `auth`, `drawterm`, and `rio`. |
| **Performance Benchmarks** | KVM dev host | Cycles per fault (scalar vs vector), eviction cost, TPHT hit rates at 90%+ occupancy, 9P IOPS under ring bypass. |

### Technical Risks & Mitigations

* **Cryptographic Fault Overhead:** Cryptographic operations consume ~4.4 µs per cold fault on KVM. If eviction is too aggressive, interactive performance degrades. **Mitigation:** The High-Water Pager maintains a 15% pre-cleared clean frame buffer, scheduling background encryption during idle cycles.
* **Pinned User Frames:** Kernel operations holding user memory across sleep points (e.g., `devmnt` network reads into user buffers) must explicitly pin those frames against the evictor to prevent re-encryption and frame reuse mid-flight.
* **Soft-Float Target Assembler:** If the soft-float kernel target's assembler rejects AVX mnemonics in `global_asm!`, the vector routine is isolated into a standalone assembly file (`l.S`).
* **Toolchain Scale:** Track P encompasses ~1.2 million lines of C across Harvey's libraries and commands. The cross-build path established in Stage P0 enables one-at-a-time binary replacement, ensuring the system boots and passes tests at every intermediate commit.
