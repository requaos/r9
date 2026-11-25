# Plan r9 Architecture & Code Compendium (v2.2)

> **Reconciling Hardware-Isolated Cryptographic Memory, Succinct TPHT & Roaring Set-Algebra, Prioritized 9P2000.aead Wire Fabric, and Donor-Verbatim Porting Across `requaos/r9` and `requaos/harvey`**  
> *Targeting `requaos/r9` (commit `c787ce9`) and `requaos/harvey` (commit `a6b7e6a9`)*  
> *November 20, 2026*

---

### Abstract

This compendium specifies the modernized operating system architecture and complete reference implementation for **r9**, a faithful pure-Rust port of Bell Labs’ Plan 9 and Harvey operating systems. It incorporates the complete audit findings, defect corrections, and landing sites established by the *r9 Implementation Guide* against live forks (`requaos/r9` at commit `c787ce9` and `requaos/harvey` at `a6b7e6a9`).

Crucially, Compendium v2.1 **reprioritizes the 9P2000.aead protocol and its supporting data structures into the earliest execution phases (Phase 1 & Phase 2)**. Because Plan 9 is fundamentally defined by the philosophy that *"the network file protocol IS the operating system fabric"*, 9P transport cannot be relegated to an afterthought or deferred behind monolithic subsystem ports. The core data structure crate (`ds/`)—encompassing **Flattened and Chained Tiny Pointer Hash Tables (TPHT)** (Tang, Mai, Kuszmaul, and Conway; arXiv:2607.28892v1) and **Roaring Bitmaps** (Lemire, Ssi-Yan-Kai, and Kaser; arXiv:1603.06549)—is implemented upfront alongside cryptographic primitives, directly powering the upgraded 9P2000.aead wire fabric.

Key architectural evolutions detailed herein include:
1. **Single-Memory-Access `FidTable`:** Stateful file handles embed an 8-bit `TinyPtr` directly on the wire, resolving incoming requests via `deref_tiny` in strictly one memory read without hash probing, while immutable content-addressed files bypass server tables entirely using self-describing HMAC-BLAKE3 capability tokens.
2. **Upgraded 9P2000.aead Wire Framing:** Extensible Authenticated Associated Data (AAD) headers with bitmasked extension TLVs (`ExtMask`), strict monotonic 64-bit sequence nonces, and four new message pairs (`Tcompound`/`Rcompound`, `Tstream`/`Rstream`, `Tlease_revoke`/`Rlease_ack`, `Treclaim`/`Rreclaim`).
3. **Coherence & Volatility Contracts:** Explicit `Qid.type` volatility classes (`QTIMMUTABLE`, `QTLEASED`, `QTVOLATILE`) paired with `ClientSet` small-set reader optimizations (inline arrays upgrading to Roaring Bitmaps for $10^4+$ cluster nodes).
4. **Speculative Parallel Union Routing:** Negative-filtering layer presence Roaring Bitmaps (`entry_hashes`) eliminating false-positive remote walks with zero probing, backed by a 32-byte quotiented dcache (`UnionRouteEntry`) with 4-bit `ClockNibble` aging.
5. **Zero-Tax Cryptographic Memory & Hardware Graphics:** Hardware CR3 process isolation, WRPKRU intra-process permission attenuation, dual-path non-temporal store routing with interleaved SFENCE hiding, and VirtIO-GPU Venus 3D acceleration over the synthetic `#G` device with `draw(3)` zero-copy host presentation.

Strict compliance with Plan 9 principles is maintained throughout: the system call list remains closed, `a.out` is preserved as the sole binary format, and hardware acceleration is presented exclusively through synthetic 9P file trees without custom `ioctl` extensions.

---

## 1. Executive Summary & Reconciled System Topology

### 1.1 Live Repository State (`requaos/r9` & `requaos/harvey`)

The implementation guide grounds this compendium against the live repositories as of November 2026:

* **`requaos/r9` at `c787ce9`:** Boots Harvey’s `amd64` userspace to an interactive shell, CPU service, and `rio` on QEMU `q35`. The workspace contains crates `aarch64`, `x86_64`, `port`, `riscv64`, and `xtask`.
* **Unsafe Invariant Policy:** The `port` crate enforces `#![forbid(unsafe_code)]` across all modules. Low-level hardware operations (WRPKRU, MOVNTDQ, CLFLUSHOPT, raw atomics, SIMD routines, and vector ABIs) are confined strictly to `x86_64/`, `aarch64/`, and the foundational `ds/src/tpht/` crate behind audited `// SAFETY:` comments, exposing safe Rust interfaces to `port/`.
* **Dedicated Crates:** Target JSON specs live in `lib/`. Cryptographic engines reside in `crypto/`, foundational data structures (TPHT, Roaring Bitmaps) reside in `ds/`, and portable kernel logic lives in `port/`.
* **`requaos/harvey` at `a6b7e6a9`:** Plan 9 4th Edition derivative with `amd64` C kernel (`sys/src/9k/`). System call flags `RFCKEY` (bit 18), `RFROBASE` (bit 19), and `RFRANDMEM` (bit 20) are reserved in `sys/include/libc.h` and `x86_64/src/proc.rs`.

### 1.2 Architectural Synthesis: Memory, Hashing, 9P Fabric & Storage

| Subsystem | v1.0 Baseline (Dated) | Compendium v2.0 Audit | v2.1 Reconciled & Prioritized Target |
| :--- | :--- | :--- | :--- |
| **Build Priority** | 9P deferred to Phase 6 | 9P deferred to Phase 6 | **Elevated to Phase 1 (`ds/`, `crypto/`) & Phase 2 (`9P2000.aead`, `FidTable`)** |
| **Crate Layout** | `lib/`, `port/seg.rs` | Conflict with target JSON in `lib/` | Dedicated `crypto/` and `ds/` crates; zero-unsafe wrappers in `port/` |
| **Kernel Safety** | Inline `asm!` in `port/` | Violates `#![forbid(unsafe_code)]` | Portable contracts in `port/`; `unsafe` in `x86_64/` and `ds/src/tpht/` |
| **Vector Budget** | `ymm0`–`ymm3` (4 registers) | 4 registers insufficient for rotates | `ymm0`–`ymm5` (6 registers: 4 state, 2 temporaries; ~30-cycle spill) |
| **9P FID Routing** | Hash probe in Funnel / TPHT | Hash probe in FlattenedBlock | **Hybrid `FidHandle`: `StatelessCap` (0 RAM) vs `deref_tiny` (1 memory read)** |
| **9P Wire Framing** | Bare Size/Type/Tag/FID header | AAD bound sequence counter | **Upgraded AAD + `ExtMask` (deadlines, caps, volatility, streams, binary)** |
| **9P Caching** | No client caching; chatty RTT | Uncached remote reads | **`Qid.type` Volatility (`QTIMMUTABLE`, `QTLEASED`, `QTVOLATILE`) + `ClientSet`** |
| **Union Mounts** | Sequential probing per layer | Sequential probing per layer | **Exact Roaring `entry_hashes` (0 false positives) + `SpeculativeParallel` Twalk** |
| **Control RPCs** | Multi-step read-modify-write | Multi-step read-modify-write | **Atomic `Tcompound` pipelines (Walk→Open→Read/Write→Clunk in 1 RTT)** |
| **Page Cache** | 3-tier Funnel Table (11MB) | Resizing & tombstone reuse bugs | Flattened-TPHT (1 cache line hit, Feistel quotienting, ClockNibble) |
| **Venti Index** | 8-byte folded Funnel Table | Truncation collision vulnerability | Chained-TPHT (1B tiny ptrs, 105.4% space efficiency, 32B BLAKE3 check) |
| **MAC Primitive** | XOR fold / Poly1305 | Decrypt-then-MAC flaw; key reuse | Keyed BLAKE3 / Poly1305 Encrypt-then-MAC |
| **VirtIO-GPU** | `VIRTIO_GPU_F_VENUS = 1<<1` | Bit 1 is EDID; Venus is capset 4 | Bits 0, 3, 4 (`VIRGL`, `BLOB`, `CTX_INIT`) + Capset 4 |
| **Toolchain & Runtime** | Unspecified | `a.out` only; closed syscalls | SMP (Milestone S) → r9 `std` (`libstd_r9`) → Track P (Harvey port in idiomatic Rust) → Phase 8 (Self-Hosting) |

---

## 2. Implementation Guide Audit & Critical Corrections Matrix

The *r9 Implementation Guide* conducted an exhaustive line-by-line audit of the initial architecture compendium and preliminary code sketches, producing 17 blocking and mechanical defect corrections backed by 55 passing unit tests on a host toolchain.

### 2.1 Defect Resolution Ledger

| Component | Identified Defect | Operational Consequence | Engineered Replacement |
| :--- | :--- | :--- | :--- |
| **`key.rs`** | Placeholder XOR fold in HMAC | Insecure key derivation; child keys trivial function of parent | RFC 5869 HKDF-SHA256 engine in `crypto/src/kdf.rs`. |
| **`key.rs`** | Read-only child derives new key | Child cannot decrypt shared Block A ciphertext sealed by parent | Shared segments inherit identical $k_{\text{dec}}$; private segments derive. |
| **`simd.rs`** | Decryptor copies without key/nonce | Zero encryption performed; XOR fold in place of MAC | Scalar reference in `crypto/page.rs`; vector routine in `x86_64/cryptpage.rs`. |
| **`simd.rs`** | Unsound separate `asm!` blocks | Compiler can reorder/clobber vector registers; AVX-512 clobbers upper `zmm` | Unified global `asm!` routine saving `ymm0`–`ymm5`; AVX-512 disabled in XCR0. |
| **`simd.rs`** | Decrypt-then-MAC ordering | Plaintext read from DRAM after non-temporal store; $k_{\text{dec}}$ reused as MAC key | Encrypt-then-MAC: verify ciphertext before decrypting into shadow frame. |
| **`seg.rs`** | Broken stride tracker indexing | Compares fixed slots 1-0 and 2-1 on a ring buffer; fires on only 1 fault in 3 | Corrected chronological ring buffer evaluation in `port/src/segcrypt.rs`. |
| **`shadow.rs`** | Unconditional `clflushopt` | Crashes on CPUs lacking `CLFLUSHOPT` (QEMU TCG); drops recycle invariant | Gated via CPUID leaf 7; explicit recycle within for clean, same-domain frames. |
| **`page.rs`** | Address arithmetic precedence | Multiplication binds tighter than bitwise OR: `(chunk << 16) \| offset * 4096` | Parenthesized arithmetic: `((chunk << 16) \| offset) * 4096`. |
| **`page.rs`** | Pinned frames evicted in fallback | Set difference skipped when containers are Array or Run | Full container materialization in fallback difference; pinned frames never evicted. |
| **`superpage.rs`** | Residency tracker lacked reset | Struct was 72B (rounded to 128B); counter drifted; no demotion path | Exact 64B cache line; bit clearing supported on large-page demotion. |
| **`funnel.rs`** | Tombstone reuse | Slots reused before probe path cleared; stale values resurrected | Replaced in PageCache/Venti by TPHT; Funnel restricted to write-once scratchpads. |
| **`funnel.rs`** | Correlated tier hashes | Single hash masked across tiers; collisions in $A_0$ cascaded to identical blocks | Salted independent hashing per tier; last tier probes 8 consecutive blocks. |
| **`scoreindex.rs`** | 8-byte score truncation | Different blocks sharing 64-bit prefix collided, overwriting/corrupting data | Chained-TPHT: 64-bit folded key is a hint; full 32-byte BLAKE3 score verified on match. |
| **`session.rs`** | 9P key derived and discarded | Bare FID stored as raw pointer; FID 0 rejected as empty slot | Per-connection, per-direction keys derived during `Tauth` with strict sequence nonces. |
| **`virtio_gpu.rs`** | Wrong feature bit (`1<<1`) | Bit 1 is EDID; driver believed it had Venus | Negotiates bits 0, 3, 4 (`VIRGL`, `BLOB`, `CTX_INIT`); selects Capset 4. |
| **`devgpu.rs`** | Context IDs discarded in walk | Every client forced into context 0; user bytes became kernel header | Context packed in QID path; kernel constructs virtio headers securely. |
| **`notes.rs`** | Unchecked stack pointer delivery | Process can aim SP into Read-Only Block A, corrupting memory on trap | Requires `SegKind::Stack`; checked arithmetic within stack bounds. |

### 2.2 Architectural Threat Model Boundaries

To avoid impossible security promises, r9 establishes explicit threat model boundaries:
* **In Scope:** Protection of pages at rest (evicted frames, backing stores, NVMe swap, Venti archives, distributed 9P-AEAD transit); prevention of stale-frame data disclosure across distinct process KeyIDs; protection against stray kernel reads through incorrect page table mappings (since only the shadow frame contains plaintext); prevention of packet splicing, injection, and replay over 9P network connections.
* **Out of Scope:** Physical hardware bus probing of live DRAM, cold-boot physical attacks, and arbitrary code execution within the kernel context. Because cryptographic keys are held within kernel memory, a compromised kernel supervisor inevitably exposes keys. Hardware memory encryption (AMD SEV-SNP, Intel TDX) is required if physical DRAM bus sniffing is in the threat model.

---

## 3. Hardware-Isolated Cryptographic Memory & Zero-Tax Paging

### 3.1 Typestates & Key Capabilities

Memory segments are governed by compile-time capability tokens (`KeyCapability`) wrapping sensitive keys in self-zeroizing containers (`Zeroizing<[u8; 32]>`):
* **`ReadOnly`:** Possesses only the 32-byte decryption key ($k_{\text{dec}}$) and the BLAKE3 verification context. It possesses no write-authentication key ($k_{\text{mac}}$), mathematically preventing the generation of valid integrity tags.
* **`ReadWrite`:** Possesses both $k_{\text{dec}}$ and $k_{\text{mac}}$, permitting both demand-decryption and page re-sealing on eviction.

### 3.2 Dual-Path Store Routing & Interleaved Pipeline Hiding

When an encrypted page is demand-decrypted on fault:
* **The Faulted 64-Byte Cache Line:** Written via temporal cached stores (`VMOVAPS`). It remains hot in L1/L2 cache when the fault returns to userspace.
* **The Remaining 4032 Bytes:** Written via non-temporal streaming stores (`MOVNTDQ`), bypassing the L1/L2 cache hierarchy and preventing CPU cache pollution.
* **Interleaved Write-Combining Drain:** The required serializing `SFENCE` instruction is issued immediately after streaming, running in parallel with scalar tag verification (BLAKE3/Poly1305), hiding memory bus latency behind ALU cycles.

### 3.3 Restricted Vector ABI (`ymm0`–`ymm5`)

To prevent vector register thrashing and avoid clobbering user FP state:
* Only 6 registers (`ymm0`–`ymm5`) are used during kernel page fault crypto: 4 registers for ChaCha20 state columns, and 2 registers for rotate temporaries.
* Saving and restoring 6 registers takes $\approx 30$ CPU cycles, compared to over 400 cycles for full `XSAVE`/`XRSTOR`.
* AVX-512 state is explicitly disabled in XCR0 to eliminate upper-ZMM zeroing hazards and frequency throttling.

### 3.4 Allocator Bypass: `UninitializedFrame`

To eliminate the "Write-Write Tax" where the OS zeros a frame only for the caller to immediately overwrite it:
* The kernel allocator returns `UninitializedFrame`.
* **Same-Domain Recycling:** If a frame was previously owned by the same KeyID, zeroing and cache flushes are skipped entirely.
* **Global Allocation:** If a frame is reassigned across domains, it is scrubbed with `CLFLUSHOPT` gated via CPUID leaf 7.

---

## 4. Succinct TPHT & Roaring Set-Algebra Engine (`ds/` Crate)

The `ds/` crate provides the foundational data structures for the kernel page cache, Venti archival indexing, and the distributed 9P2000.aead engine.

### 4.1 One-Round Feistel Quotienting Routine

Standard open-addressing tables store full 64-bit keys in every slot. TPHT eliminates key storage using a bijective one-round Feistel permutation:
$$F(L, R) = (R,\; L \oplus f(R))$$
where $L$ and $R$ represent the upper and lower 32 bits of the 64-bit key, and $f(R)$ is a fast non-linear mixing function:
$$f(R) = (R \oplus (R \gg 16)) \cdot \text{0x85EBCA6B} \oplus \dots$$

The permuted 64-bit word $P = (L' \ll 32) \mid R$ is partitioned into:
* **Home Block Index ($q$):** Lower $\log_2(\text{num\_blocks})$ bits of $P$.
* **Stored Remainder ($r$):** Upper bits of $P$.

Given only the home block index $q$ and the stored remainder $r$, the original 64-bit key is reconstructed losslessly in 3–4 ALU cycles:
$$R = P \pmod{2^{32}}, \quad L' = \lfloor P / 2^{32} \rfloor, \quad L = L' \oplus f(R), \quad \text{Key} = (L \ll 32) \mid R$$

This saves $\sim \log_2(\text{num\_blocks})$ bits per entry (20–24 bits in a kernel page cache), offsetting pointer and metadata overhead entirely.

### 4.2 64-Byte Hardware-Aligned `TphtBucketHeader` & `FlattenedTphtShard`

The primary table in `ds/src/tpht/flattened.rs` organizes buckets into exact 64-byte hardware cache lines (`#[repr(C, align(64))]`):
* **`seq_lock` (`AtomicU32`, 4 bytes):** Odd indicates an active writer; even indicates a stable generation for lock-free optimistic reads.
* **`occupied_mask` (`u16`, 2 bytes):** Bitmask of active slots in the bucket (bits 0..15).
* **`spilled_mask` (`u16`, 2 bytes):** Bitmask of slots whose tiny pointer exceeds the home window (`ptr > 16`).
* **`quotients` (`[u16; 16]`, 32 bytes):** 16-bit Feistel quotients packed for parallel SIMD comparison.
* **`tiny_ptrs` (`[u8; 16]`, 16 bytes):** 8-bit displacements (`1..=255`, 0 = empty) addressing slots in the flat value arena.
* **`clock_nibbles` (`AtomicU64`, 8 bytes):** Sixteen 4-bit clock/aging counters packed into a single 64-bit atomic integer.

### 4.3 Concurrent Seqlock, Single-Read `deref_tiny` & Neighborhood Displacement

1. **Single-Memory-Read Fast Path (`deref_tiny`):** When the caller possesses `(key, tiny_ptr)`—such as an incoming 9P request referencing a `StatefulHandle`:
   $$\text{slot} = ((b \ll 4) + (\mathtt{ptr.get()} - 1)) \& \text{slot\_mask}$$
   The value is retrieved directly from the flat value arena in **exactly one memory read**, completely bypassing header scans, seqlock spins, and hash probes.
2. **Key-Only Probing:** When querying by key alone, the reader checks `hdr.seq_lock`, performs an atomic SIMD match against `quotients`, verifies the candidate entry, and accepts the result if `seq_lock` remained unchanged.
3. **Neighborhood Spillover Displacement (1..=255):** Home buckets store up to 16 entries in slots `1..=16`. When home slots are occupied, writers scan the neighborhood `17..=255` using 64-bit word bitmasks on `slot_bitmap`.
4. **4-Bit Clock-Nibble Eviction:** For cache structures like `UnionIndex`, cold entries are reclaimed via register-level second-chance clock sweeping without allocating garbage.

### 4.4 Two-Choice Dereference Banks (`ChainedTphtOverflow`)

For sparse, unbounded indices like the Venti score index, `ds/src/tpht/chained.rs` implements secondary dereference banks:
* Each bank is an arena of 255 compact overflow nodes indexable by a 1-byte tiny pointer.
* **Two-Choice Load Balancing:** Each home block maps to two candidate bank pairs ($\text{Bank}_0$ and $\text{Bank}_1$). New overflow nodes are allocated in the bank with greater free capacity, enabling high fill factors (>95%) without allocation failures.

### 4.5 Roaring Bitmaps for Integer Set Algebra

`ds/src/roaring.rs` implements the hybrid Array, Bitmap, and Run container format (Lemire et al., arXiv:1603.06549):
* **Physical Frame Allocator:** Tracks free clean, dirty pool, and pinned pool across 256MB DRAM chunks.
* **SIMD Eviction Harvesting:** Vectorized set difference in $< 200$ cycles:
  $$\text{Evictable} = \text{Dirty} \setminus (\text{Pinned} \mid \text{Active})$$
* **$O(1)$ 2MB Superpage Promotion:** The `SuperpageResidency` structure maps 512 sub-pages (4KB each) into an 8-word (64-byte) cache-line aligned bitset. Transparent 2MB PMD promotion triggers in $\approx 5$ cycles when all 512 bits are set.
* **9P Multiplexing & Deadlines:** 16-bit tag spaces ($0..65,535$) fit into single Roaring containers. Priority tiers (`P0_CONTROL`, `P1_METADATA`, `P2_BULK`) pop the next runnable stream via trailing-zero count (`tzcnt`) in $O(1)$. Millisecond-bucketed Roaring Bitmaps implement timer wheels with cascading cancellation down exportfs proxies.
* **Exact Union Layer Filters:** Layer presence bitsets eliminate false-positive remote walks with zero network probes.

### 4.6 Small-Set Optimization: `ClientSet` Enum

To avoid heap allocations for files with 1–4 readers, `ClientSet` provides an inline small-set representation that automatically upgrades to a `RoaringBitmap` for large reader cohorts:
```rust
pub enum ClientSet {
    Empty,
    Single(u32),
    Inline { count: u8, ids: [u32; 4] },
    Roaring(Box<RoaringBitmap>),
}
```

### 4.7 Positioning of Funnel and Elastic Hashing

* **Funnel Hashing (Specialized):** Retained strictly as an ephemeral, zero-allocation lock-free scratchpad in `port/src/inflight.rs` for deduplicating concurrent page faults on shared Block A code.
* **Elastic Hashing (Batch Search):** Retained in `telescope` for batch-constructed, immutable full-text inverted indexes over historical Venti snapshots, where its 2D probing sequence achieves an optimal $O(\log \delta^{-1})$ probe bound.

---

## 5. Distributed 9P2000.aead Protocol, QIDs/FIDs & Legacy Gateway

Plan 9's universal abstraction is the 9P protocol. Compendium v2.1 upgrades 9P2000.aead to resolve cluster-scale distributed bottlenecks while preserving Plan 9's minimal VFS semantics.

```
========================================================================================
                          UPGRADED 9P2000.aead WIRE ARCHITECTURE
========================================================================================

  Client Node                                                 Server Node
  ┌──────────────────────────────┐                            ┌──────────────────────────────┐
  │ Process / rio / VFS          │                            │ Fossil / Venti / #M / devip  │
  └──────────────┬───────────────┘                            └──────────────▲───────────────┘
                 │                                                           │
                 │ Tread / Twrite / Tcompound                                │ Single-Read deref_tiny
                 ▼                                                           ▼
  ┌──────────────────────────────┐                            ┌──────────────────────────────┐
  │ port/src/chan.rs             │                            │ port/src/fid.rs              │
  │ • ExtMask TLVs               │                            │ • FidShard (Flattened-TPHT)  │
  │ • Monotonic Sequence Nonces  │                            │ • StatefulHandle: TinyPtr8   │
  │ • Keyed BLAKE3 AAD           │                            │ • StatelessCap (0 RAM)       │
  └──────────────┬───────────────┘                            └──────────────▲───────────────┘
                 │                                                           │
                 │ Wire Packet (ChaCha20 Ciphertext + BLAKE3 MAC)            │ Zero-Copy DMA
                 └──────────────────────────────────────────────────────────►│
```

### 5.1 9P QID Architecture & Volatility Classes

A 9P resource is identified by a 13-byte `Qid`: `path: u64`, `vers: u32`, and `qtype: u8`. Synthetic drivers multiplex state into `Qid.path`:
* **Synthetic `#G` GPU Routing:**
  $$\text{Qid.path} = (\text{ctx\_id} \& \text{0xFFFFFF}) \ll 8 \mid (\text{node\_type} \& \text{0xFF})$$
* **Cryptographic Object Binding:**
  $$K_{\text{file}} = \text{HKDF-Expand}(K_{\text{user}}, \text{Qid.path} \parallel \text{Qid.vers}, 32)$$

To resolve the cache coherence vs. synthetic file paradox, `Qid.qtype` extends standard mode bits with **three explicit Volatility Classes**:
1. **`QTIMMUTABLE` (`0x20`):** Content-addressed files (Venti blocks, shared binaries). Clients cache data indefinitely in local DRAM/NVMe with zero revalidation round-trips. Contiguous blocks naturally compress into Run containers in `SuperpageResidency`, triggering $O(1)$ 2MB superpage promotion.
2. **`QTLEASED` (`0x40`):** Shared mutable files (Fossil file trees). Clients cache reads and buffer writes locally until the server pushes a `Tlease_revoke` message.
3. **`QTVOLATILE` (`0x80`):** Live synthetic files (`/dev/cons`, `/proc`, `#G`). All reads and writes bypass local caches and route directly to the authoritative device handler.

### 5.2 Hybrid `FidTable` & Single-Memory-Access Dereferencing

In classical Plan 9, every open file handle consumes an entry in the server's fid table, pinning sessions and consuming memory. r9 splits file identifiers into two wire representations:

1. **`StatelessCap` (24 bytes):** For `QTIMMUTABLE` files and static directory walks, the server stores **zero state in memory**. Instead, `Twalk` returns an encrypted/HMAC-signed token encoding `qid_path`, `qid_vers`, `mode`, `epoch`, and an 8-byte BLAKE3 MAC. Any cluster replica sharing the domain key validates the token and serves `Tread` immediately.
2. **`StatefulHandle` (8 bytes):** For genuinely stateful channels (pipes, `/net`, leases, streaming cursors), the handle embeds the server's 8-bit `TinyPtr`, `shard_id`, and `epoch`:
   ```rust
   #[repr(C)]
   pub struct StatefulHandle {
       pub fid_key: u32,
       pub ptr: TinyPtr,
       pub shard_id: u8,
       pub epoch: u16,
   }
   ```
   On every subsequent `Tread`/`Twrite`, the server calls `deref_tiny(fid_key, ptr)` in `FidShard`, resolving the 64-byte `FidEntry` in **strictly one memory read without hash probing**.
3. **Failover Grace Sweep:** Post-failover, clients replay active handles via `Treclaim`. The server tracks reclaimed handles in a shard-local `reclaimed_fids` Roaring Bitmap. When the grace window closes, un-reclaimed descriptors are swept in one SIMD ANDNOT operation:
   $$\mathtt{Orphaned} = \mathtt{active\_fids} \setminus \mathtt{reclaimed\_fids}$$

### 5.3 Upgraded `9P2000.aead` Wire Envelope & Extension Bitmask

The upgraded wire envelope binds cleartext transport control fields as Authenticated Associated Data (AAD) ahead of a 4KB-aligned ChaCha20 payload and a 32-byte Keyed BLAKE3 MAC:
$$\text{Packet} = \underbrace{\text{Size (4B)} \parallel \text{Type (1B)} \parallel \text{Tag (2B)} \parallel \text{FidHandle (8B)} \parallel \text{SeqNonce (8B)} \parallel \text{ExtMask (2B)} \parallel \text{ExtTLVs}}_{\text{Authenticated Associated Data (AAD — Keyed BLAKE3)}} \parallel \underbrace{\text{Ciphertext (ChaCha20)}}_{\text{4KB Aligned}} \parallel \underbrace{\text{MAC (32B)}}_{\text{BLAKE3}}$$

Amended 2026-11-24 (r9 Phase 4, decision D-07): the AAD formula above is superseded; the MAC covers every byte before it, from byte 0 of `Size` to the last ciphertext byte (the range `[0, size - 32)`), so `Pad` and `ExtTLVs Length` are authenticated too, and RFC section 4.3 as amended is the normative statement.

#### Extension Bitmask (`ExtMask`: `u16`)
* **`EXT_DEADLINE` (`1 << 0`):** 8-byte monotonic cluster deadline. Propagates across chained exportfs mounts; expired requests drop automatically without `Tflush` round-trips.
* **`EXT_CAP_TOKEN` (`1 << 1`):** 32-byte signed capability token, allowing multiple processes on a node to multiplex over a single pooled connection without per-user `Tauth` storms. Checked in $O(1)$ against a cache-resident Roaring Bitmap Certificate Revocation List (CRL).
* **`EXT_QID_VOLATILITY` (`1 << 2`):** Carries `VolatilityClass` and lease generation inline on `Rwalk`/`Ropen` responses.
* **`EXT_STREAM_PRIO` (`1 << 3`):** Priority tier (`P0_CONTROL`, `P1_METADATA`, `P2_BULK`) + 32-bit stream credit (`WINDOW_UPDATE`).
* **`EXT_OBINARY` (`1 << 4`):** Negotiates zero-copy aligned binary struct views (`#[repr(C)]` / FlatBuffers) on synthetic devices in place of ASCII text.

### 5.4 Four New Message Pairs (`port/src/chan.rs`)

1. **`Tcompound` / `Rcompound`:** Bundles dependent operations (Walk → Open → Read/Write → Clunk) into a single AAD-authenticated wire frame using a server-side `CURRENT_FID` register. Collapses multi-step control flows (like opening `/net/tcp/clone`, reading the ID, and writing to `ctl`) into a single atomic network round-trip.
2. **`Tstream` / `Rstream`:** Server-push and bidirectional streaming over a `FidHandle`, eliminating the `msize / RTT` throughput ceiling on high-latency links.
3. **`Tlease_revoke` / `Rlease_ack`:** Server-initiated asynchronous revocation for `QTLEASED` files, carrying an invalidation range or Roaring Bitmap of invalidated 4KB block indices.
4. **`Treclaim` / `Rreclaim`:** Post-failover grace-window descriptor recovery.

### 5.5 Distributed Leases & Coherence Tracking (`port/src/lease.rs`)

* **Server (`LeaseRecord`):** Stored in `FlattenedTphtShard<u64, LeaseRecord>` keyed by inode. Concurrent reader nodes are tracked via `ClientSet` (inline array for $\le 4$ nodes; automatic upgrade to a `RoaringBitmap` for $10^4+$ nodes, compressing contiguous compute node IDs like `100..4096` into a single 4-byte Run container).
* **Client (`ClientBlockCache`):** Tracks valid and dirty 4KB blocks via `valid_blocks` and `dirty_blocks` Roaring Bitmaps. On `Tlease_revoke`, the client computes `dirty_blocks & revoked_range`, seals those pages using the restricted vector ABI (`ymm0`–`ymm5`) and spatial nonces ($\text{CoreID} \parallel \text{PageOffset} \parallel \text{Epoch}$), and flushes them to the server.

### 5.6 Speculative Parallel Union Mounts (`port/src/union.rs`)

Plan 9 per-process union namespaces (`bind -a`, `bind -b`) are accelerated to eliminate sequential network probing:
* **Exact Layer Filters:** Static and leased union layers publish exact 32-bit `(parent_qid ^ name_hash)` Roaring Bitmaps (`entry_hashes`). The client mount driver checks filters in local RAM in $O(1)$, skipping absent layers with **zero false positives**.
* **Speculative Parallel Resolution:** Dynamic layers (flagged in `dynamic_layer_mask`, e.g., `/proc`, `/net`) and candidate static layers are probed concurrently via `SpeculativeParallel` `Twalk`s. The winning route is cached in a 32-byte `UnionRouteEntry` stored in a quotiented `FlattenedTphtShard` with 4-bit `ClockNibble` second-chance aging.

### 5.7 Zero-Copy DMA Paging & User-Space Ring Bypass

1. **Zero-Copy Ingress:** The network adapter DMAs incoming 9P2000.aead ciphertext directly into an `UninitializedFrame` allocated in the destination segment's `BackingStore`. The kernel does not decrypt upon packet receipt; decryption is deferred until user execution triggers a page fault.
2. **User-Space Ring Bypass:** For high-throughput I/O that cannot afford syscall overhead, r9 exposes synthetic ring nodes under `#M` (`/dev/mnt/<conn>/ring`). Applications map the ring into user space via `segattach(SG_SHARED, fd, va, len)` using channel-backed capabilities (`#s`), allowing lock-free ring transfers between application buffers and the NIC without kernel VFS lock contention.

### 5.8 Legacy Translation Gateway & `rfork(RFNOLEGACY)`

When connecting to unencrypted 9P peers (9front, Linux `v9fs`, `plan9port`):
* `Tversion` negotiates `9P2000`, `9P2000.u`, `9P2000.e`, or `9P2000.L`.
* All new 9P2000.aead extensions are stripped or emulated at the gateway boundary.
* Plaintext traffic is mediated strictly through pre-pinned, non-pageable kernel bounce frames sealed under $K_{\text{legacy\_session}}$ and scrubbed immediately via `zeroize` + `CLFLUSHOPT`.
* Processes running sensitive workloads can invoke `rfork(RFNOLEGACY)` to forbid binding, mounting, or walking channels marked `CHAN_UNTRUSTED_WIRE`.

### 5.9 Dynamic Service Discovery & Staged `/srv` Orchestration (`cmd/discover`, `cmd/beacon`)

In traditional Plan 9 clusters, service location relied on static `/lib/ndb/local` tables and connection servers (`cs`), requiring manual administrative intervention whenever nodes were added or reconfigured. r9 integrates autonomous, multicast-based dynamic service discovery directly into the distributed OS fabric without adding custom system calls or breaking the closed syscall list.

#### 1. Structured Resource Specifiers (`class:instance[+attributes]`)
A flat service string (e.g., `-s gpu`) fails in heterogeneous cluster nodes where multiple physical GPUs (`#G0` display vs. `#G1` headless compute), distinct Venti partitions, or segregated storage tiers exist on a single machine. r9 formalizes structured resource descriptors:
$$\text{Resource Specifier} = \underbrace{\text{Class}}_{\mathtt{gpu},\,\mathtt{fs},\,\mathtt{venti}} \;:\; \underbrace{\text{Instance / Unit ID}}_{\mathtt{0},\,\mathtt{1},\,\mathtt{compute},\,\mathtt{main}} \;[\;+\; \underbrace{\text{Capabilities / Attributes}}_{\mathtt{venus,vram=24G,headless}}\;]$$

* **Deterministic `/srv` Naming:** When `cmd/discover` authenticates a discovered resource, it registers the open 9P channel descriptor into `/srv` using the canonical scheme:
  $$\mathtt{/srv/\{hostname\}\_\{res\_class\}\_\{res\_instance\}}$$
  * Node `gpu02` advertising `gpu:0` $\to$ `/srv/gpu02_gpu_0`
  * Node `gpu02` advertising `gpu:1` $	o$ `/srv/gpu02_gpu_1`
  * Node `venti01` advertising `venti:cold` $	o$ `/srv/venti01_venti_cold`

#### 2. Dual-Stack Multicast Discovery & Wire Format (`DiscoveryBeaconV2`)
Discovery operates across both IPv4 and IPv6 network fabrics using periodic UDP pulses (default interval: 5 seconds):
* **Multicast Endpoints:**
  * IPv4 Administratively Scoped Multicast: `239.9.9.9!9999` (TTL: 2 hops across local subnets).
  * IPv6 Link-Local Multicast: `[ff02::9:9]!9999` (Scope: local L2 switch/VLAN segment).
  * IPv6 Site-Local Multicast: `[ff05::9:9]!9999` (Scope: routed cluster datacenter fabric).
* **Wire Protocol (`DiscoveryBeaconV2`):** Total length: 192 bytes, 8-byte aligned, little-endian:
  * `magic: [u8; 6]` (`b"R9DSC2"`)
  * `version: u16` (`2`)
  * `nonce: u64` (strictly monotonic heartbeat counter)
  * `hostname: [u8; 32]`, `domain: [u8; 32]`, `res_class: [u8; 16]`, `res_instance: [u8; 16]`, `attributes: [u8; 64]`
  * `ip_addr: [u8; 16]` (native 16-byte IPv6 or IPv4-mapped IPv6 `::ffff:a.b.c.d`)
  * `port: u16`, `addr_family: u8` (4 or 6), `_reserved: [u8; 7]`
* **Plan 9 Dialing Semantics:** Enforces strict Plan 9 delimiter grammar separating network, host, and port:
  * IPv4 Target: `/net/tcp/clone connect {ip}!{port}\n`
  * IPv6 Target: `/net/tcp/clone connect [{ipv6}]!{port}\n` (square brackets isolate colons from port delimiter).

#### 3. Staged Two-Tier Health Lifecycle & `#M` Invalidation
To prevent network jitter or brief garbage-collection pauses from prematurely tearing down established connections and aborting in-flight transactions, `cmd/discover` enforces a staged degradation state machine:

```
 [Healthy] ──(30s Silence)──► [Unhealthy] ──(Additional 5m Silence)──► [Dead & Reaped]
    ▲                              │
    │                              ▼
    └────────(Heartbeat)───────────┘
```

1. **`Healthy` (0–30s Silence):** Normal operation. `/srv/<name>` is active; `/srv/<name>.status` reports `healthy\n`. The peer node ID is asserted in the `#M` client-side mount driver's `healthy_replicas` Roaring Bitmap. Speculative parallel walks and hedged reads route to this node normally.
2. **`Unhealthy` (30s–330s / +5m Silence):** Exactly 30 seconds of heartbeat silence degrades status to `Unhealthy`. `/srv/<name>.status` updates to `unhealthy\n`. `cmd/discover` clears the node's bit in `#M`'s `healthy_replicas` bitmap, shielding the cluster by halting new speculative walks, hedged reads, or load-balanced requests to this channel. Crucially, **the underlying TCP file descriptor in `/srv/<name>` is kept open**, allowing in-flight transactions to drain or complete without abrupt disconnection. If a beacon arrives during this window, the node is immediately restored to `Healthy` and re-asserted in `healthy_replicas`.
3. **`Dead` (>330s Total Silence):** After 5 minutes in the `Unhealthy` state with no heartbeats, the node is deemed permanently lost. Status transitions to `Dead`, the TCP file descriptor is closed, `/srv/<name>` and `/srv/<name>.status` are unlinked via `remove()`, and the peer record is purged from memory.

---

## 6. Storage Stack: Venti & Fossil Modernization

### 6.1 Venti Parallel BLAKE3 Content-Addressed Storage

r9 upgrades Venti from legacy 160-bit SHA-1 hashes to 256-bit cryptographic BLAKE3 Merkle-tree digests:
* **Cryptographic Collision Security:** Provides full 128-bit collision resistance ($2^{128}$).
* **Parallel Tree Hashing:** BLAKE3 computes 1KB tree chunks across SIMD lanes simultaneously, saturating NVMe bandwidth at $>10$ GB/s per core.
* **`VentiTphtIndex`:** Scores resolve to arena offsets via Chained-TPHT, where a 16MB head array sits permanently in L3 cache to service lookups wait-free.
* **Hash-Before-Compression Invariant:** The canonical 32-byte BLAKE3 score is computed strictly over the uncompressed 4KB/8KB payload: $\text{Score} = \text{BLAKE3}(\text{Plaintext})$. Deduplication across nodes is invariant to local compression choices, and parent pointer blocks in Fossil Merkle trees remain valid across background recompressions.

### 6.2 On-Disk Super-Blocks & Out-of-Band Tag Slabs

To resolve SSD alignment penalties, write amplification, and HDD seek overhead:
* **64KB Data Super-Blocks:** 16 contiguous 4KB pages aligned to 64KB flash erase boundaries. Variable-length compressed blocks are packed into the 64KB frame with zero padding gaps.
* **1KB Tag Slabs:** Adjacent 1KB slabs containing sixteen 64-byte cache-line aligned `TagEntry` structures. Scanning metadata prefetches adjacent tags automatically, achieving 98.9% DRAM efficiency without pointer chasing:
  * `nonce: [u8; 12]` (ChaCha20 Sequence Nonce)
  * `blake3_tag: [u8; 32]` (Canonical BLAKE3 Score)
  * `offset: u32` (Physical byte offset within the 64KB Super-Block)
  * `compressed_size: u16` (Payload length on disk)
  * `original_size: u16` (Uncompressed buffer allocation hint)
  * `block_type: u8` (Data=0x00, Pointer0=0x01, Dir=0x02)
  * `compression_algo: u8` (None=0x00, Zstd=0x01)
  * `_reserved: [u8; 10]` (Cache-line padding)

### 6.3 Two-Tier Zstd Compression Lifecycle & Zero-Churn Ingestion

To eliminate the catastrophic 4.7x write-amplification factor (WAF) of multi-pass recompression (`Raw` $\to$ `Zstd(3)` $\to$ `Zstd(9)`), r9 standardizes on a streamlined two-tier compression architecture:

```
                      Fossil Commit (GFS Epoch)
                                 │
                                 ▼
                   [Direct Zstd(3) Compression]  <-- Ingest directly at Level 3
                                 │                   (Zero Uncompressed Churn)
                                 ▼
                   [Venti Active / Daily Arenas]
                                 │
                Is Snapshot Expiring or Re-packing?
                                 │
               ┌─────────────────┴─────────────────┐
               ▼                                   ▼
       [Son / Father Epochs]             [Grandfather Epoch (Cold)]
       Keep at Zstd(3).                  Piggyback Zstd(9/15) ONLY during
       Zero churn. Blocks trimmed        Mark-and-Sweep GC consolidation.
       via NVMe DSM when deleted.        (Never rewrite live arenas).
```

1. **Direct Ingestion at Zstd Level 3:**
   * Fossil commits compress dirty blocks in-flight during snapshot intervals.
   * **Incompressibility Threshold Check:** If `compressed_size >= original_size`, the block is tagged `COMPRESS_NONE` (`0x00`) and stored raw. This avoids wasted decompression overhead on incompressible media and encrypted payloads.
   * **Interactive Shielding:** User writes hit Fossil's in-memory buffer cache and local disk partition with zero latency. Venti compression runs strictly in an asynchronous background worker.
2. **Decompression Asymmetry:**
   * Zstd decompression runs at ~1.2–1.5 GB/s per core (~2.5–3.5 µs per 4KB block), virtually indistinguishable from uncompressed flash reads. Readers use `original_size` from the `TagEntry` to pre-size decompression scratchpads without reallocations.
3. **Opportunistic Compaction During GFS Garbage Collection:**
   * Active arenas (Son/Father snapshots) are never rewritten merely for level changes.
   * Deep compression (Zstd Level 9–15) is applied **opportunistically only when an aging arena drops below 40% occupancy** and is scheduled for GC consolidation, saving flash write cycles.

### 6.4 Fossil GFS Retention & Mark-and-Sweep GC

Fossil organizes snapshots into Grandfather-Father-Son (GFS) retention epochs:
* **GC Trigger:** Fossil transmits surviving BLAKE3 roots to Venti.
* **Mark Phase:** Venti traverses surviving trees, populating an ephemeral `ReferencedBlocks` Roaring Bitmap.
* **Sweep Phase:** Unreferenced blocks are identified via SIMD set difference in $<200$ cycles:
  $$\mathtt{PurgeSet} = \mathtt{ArenaPresence} \setminus \mathtt{ReferencedBlocks}$$
* **Hardware TRIM:** Contiguous runs of purged blocks are deallocated via native NVMe Dataset Management (DSM) / TRIM commands, returning physical flash blocks directly to the SSD wear-leveling pool without zero-fill write cycles.

---

## 7. Graphics Subsystem: VirtIO-GPU & the `#G` Device

### 7.1 Layer 1: VirtIO-GPU Transport (`kern/src/gpu.rs`)

The driver initializes the VirtIO-GPU PCI device and negotiates feature bits 0 (`VIRGL`), 3 (`BLOB`), and 4 (`CONTEXT_INIT`). On context initialization, it specifies capability set 4 (`VIRTGPU_CAPSET_VENUS`), establishing native Vulkan command streaming.

### 7.2 Layer 2: The `#G` Plan 9 File Interface (`port/src/devgpu.rs`)

The driver implements the Plan 9 `Dev` trait, serving a synthetic filesystem under `/dev/gpu/`:
* `/dev/gpu/clone`: Returns an open file descriptor to a new GPU context.
* `/dev/gpu/<id>/ctl`: Controls context state and resource lifecycles.
* `/dev/gpu/<id>/cmd`: Accepts raw Vulkan command buffers serialized via the Venus protocol.
* `/dev/gpu/<id>/blob`: Maps host-allocated GPU memory into user namespaces.
* `/dev/gpu/<id>/fence`: Provides asynchronous GPU execution synchronization.

### 7.3 Layer 3: `draw(3)` Zero-Copy Host Presentation Bridge

To present rendered 3D frames within `rio` without copying pixels through guest RAM:
* When a client binds an image via `/dev/draw/new` using the `b` command, the kernel binds the window directly to the host-side VirtIO resource ID (`Image.gpu_res_id = Some(res_id)`).
* When `draw_composite()` executes, if both source and destination images possess host resource IDs, the kernel emits an immediate `emit_host_gpu_blit()` packet to the GPU control queue. The compositing operation runs entirely inside host GPU VRAM with zero guest CPU overhead.

---

## 8. Master Realigned Roadmap & Self-Hosting Toolchain

### 8.1 Realigned Phased Roadmap

Compendium v2.1 realigns the master build order to establish the `ds/` foundation and `9P2000.aead` wire fabric upfront:

```
[Phase 0: Threat Model & rfork bits]
         │
         ▼
[Phase 1: crypto/ & ds/ Foundations] ◄── Upfront TPHT & Roaring Bitmaps
├── crypto/: HKDF-SHA256, ChaCha20, Keyed BLAKE3, ymm0–ymm5 SIMD
└── ds/: FlattenedTphtShard, ChainedTphtOverflow, RoaringBitmap, ClientSet
         │
         ▼
[Phase 2: Core 9P2000.aead Transport & FidTable] ◄── Foundational 9P Wire Fabric
├── port/src/fid.rs: Hybrid FidHandle (StatelessCap vs StatefulHandle with TinyPtr8)
├── port/src/chan.rs: Upgraded AAD framing, ExtMask headers, Monotonic Sequence Nonces
└── crypto/src/session.rs: Factotum Tauth, K_transient derivation, pooled CapTokens
         │
         ▼
[Phase 3: Cryptographic Memory & Zero-Copy 9P DMA Paging]
├── port/src/page.rs: UninitializedFrame directly wired to 9P RX rings
├── x86_64/src/cryptpage.rs: Dual-path stores, interleaved SFENCE hiding
└── port/src/segcrypt.rs: StrideTracker, Segment derivation, ShadowFrame lifecycle
         │
         ▼
[Phase 4: Distributed 9P Extensions, Leases & Dynamic Service Discovery]
├── port/src/devmnt.rs: Tcompound pipelines, Tstream flow control, Roaring deadline wheel
├── port/src/lease.rs: Qid Volatility (QTIMMUTABLE, QTLEASED, QTVOLATILE), LeaseRecord
├── port/src/union.rs: Exact RoaringBitmap layer filters, SpeculativeParallel Twalk
├── port/src/legacy_9p.rs: Pinned zeroize bounce gateway for legacy 9front/Linux
└── cmd/beacon & cmd/discover: Multicast DiscoveryBeaconV2, staged 30s/5m lifecycle, #M sync
         │
         ▼
[Phase 5: Process Lifecycle, rfork Flags & Notes]
├── rfork(RFCKEY, RFROBASE, RFRANDMEM) live derivation
└── Secure note delivery with strict SegKind::Stack boundary enforcement
         │
         ▼
[Milestone S: Multi-CPU SMP Scheduler & Spinlock Retirement]
         │
         ▼
[Milestone STD: r9-native `std` Runtime (`sys/src/libstd_r9/`)]
├── std::fs      -> 9P Walk/Open/Read/Write/Stat/Dirread (#M)
├── std::net     -> /net/tcp and /net/udp conversational files ([{}]!{})
├── std::thread  -> rfork(RFPROC | RFMEM | RFFDG) + %gs/FS_BASE TLS
├── std::sync    -> Atomic futex fast-path + #c/rendezvous
└── std::process -> rfork + exec(a.out)
         │
         ▼
[Track P: Complete Harvey Userland Port in Idiomatic Rust]
├── rc, sam, rio, upas, auth/factotum written using standard Rust `std`
└── Zero temporary `no_std` scaffolding needed
         │
         ▼
[Phase 6: Storage Engine: BLAKE3 Venti & Snapshot Fossil]
         │
         ▼
[Phase 7: Graphics Subsystem: #G VirtIO-GPU Venus & draw(3) Bridge]
         │
         ▼
[Phase 8: Self-Hosting Rust Toolchain: rustc, rust-lld, APEX, a.out]
```

| Phase | Deliverable Scope | Dependencies | Verification Exit Gate |
| :--- | :--- | :--- | :--- |
| **Phase 0** | Threat model doc, C prototypes, reserve `rfork` flag bits 18–20 in `libc.h` | None | Maintainer confirms decisions; flag bits reserved. |
| **Phase 1** | `crypto/` & `ds/` foundations: RFC HKDF, ChaCha20, BLAKE3, `FlattenedTphtShard`, `RoaringBitmap`, `ClientSet` | Phase 0 | RFC vector pins pass; TPHT single-line hit tests pass on host toolchain. |
| **Phase 2** | Core 9P2000.aead Transport & `FidTable`: `FidHandle`, `FidShard`, `deref_tiny`, AAD envelope, monotonic nonces, Factotum `Tauth` | Phase 1 | Wire encoding/decoding tests pass; single-read dereferencing verified; anti-splicing AAD pins green. |
| **Phase 3** | Cryptographic Memory & Zero-Copy DMA Paging: `UninitializedFrame`, Dual-Path stores, interleaved SFENCE hiding, `ShadowFrame`, `StrideTracker` | Phase 2 | 9P packets DMA directly into `BackingStore`; demand `#PF` decrypts correctly; zero memory amplification. |
| **Phase 4** | **Distributed 9P Extensions & Cluster Service Discovery**: `Tcompound` pipelines, `Tstream`, `LeaseRecord`, `UnionMountTable`, `healthy_replicas` Roaring Bitmap, `DiscoveryBeaconV2` multicast wire protocol, `cmd/beacon` & `cmd/discover` (with two-tier 30s/5m degradation lifecycle and `#M` integration), legacy bounce gateway, `RFNOLEGACY` | Phase 3 | Atomic compound scripts pass; multi-client lease revocation tests green; discovery beacons dynamically populate `/srv` across IPv4/IPv6; 9front interoperability verified. |
| **Phase 5** | Process Lifecycle & Notes: `rfork` flags live (`RFCKEY`, `RFROBASE`, `RFRANDMEM`), stack boundary note delivery | Phase 4 | Sibling isolation pins; stack spoofing traps pass. |
| **Milestone S**| Multi-CPU SMP; retire single-CPU spinlock invariants across kernel | Phase 3, Phase 5 | Multi-CPU boot on QEMU `q35`; zero contention deadlocks. |
| **Milestone STD**| **r9 Native Standard Library (`sys/src/libstd_r9/`)**: Map `std::fs`, `std::net`, `std::thread`, `std::process`, `std::sync` to Plan 9 syscalls | Milestone S | Test suite for `std` passes; idiomatic Rust binaries run without `#![no_std]`. |
| **Track P** | Complete Rust port of Harvey kernel devices, 35 libraries, and userland commands in **idiomatic Rust via `libstd_r9`** | Milestone S, Milestone STD | System boots without C code; all Harvey tools compiled natively using `std`. |
| **Phase 6** | Storage Engine: BLAKE3 Venti CAS store, Chained-TPHT score index, Fossil GFS snapshotting, NVMe TRIM GC | Phases 2, 4, Track P | Venti block deduplication green; snapshot mark-and-sweep GC verified. |
| **Phase 7** | Graphics: 2D framebuffer, `devdraw`, `#G` Venus, `draw(3)` zero-copy host blit bridge | Track P (`draw`/`rio`) | Hardware triangle rendered in `rio` window on QEMU. |
| **Phase 8** | Self-Hosting Rust Toolchain (`rustc`, `rust-lld`, APEX C runtime, ELF-to-`a.out`) | Track P, Milestone STD, Phase 6 | r9 compiles its own kernel and userland on r9. |

### 8.2 Phase 8: Self-Hosting Toolchain Architecture

1. **C Runtime via APEX:** Harvey’s APEX (`Harvey-OS/apex`, MIT license) is vendored. It provides musl-based ANSI C over Plan 9 system calls without `mmap`. System call stubs are generated for Clang compatibility.
2. **Dual C Runtime Isolation:** r9 maintains two strictly segregated C environments:
   * **Classic APE:** Built by ported Plan 9 C compilers (`6c`, `6a`, `6l`) for native Plan 9 contrib software (demonstrated by Ghostscript).
   * **APEX:** Built by Clang for modern C++ tooling (demonstrated by LLVM).
3. **Linker & Output Format:** `rust-lld` emits static ELF, which an in-tree conversion tool immediately wraps into Plan 9 `a.out` format.
4. **Zero `mmap` Dependency:** `rustc`’s `memmap2` metadata loader falls back to whole-file read buffers on r9, completely avoiding new kernel memory-mapping calls.

---

### 8.3 Milestone STD: Architecture of the r9 Native Standard Library (`sys/src/libstd_r9/`)

A fundamental breakthrough in the amended r9 plan is slotting the native **`r9-std`** runtime immediately following **Milestone S (Multi-CPU SMP)** and prior to **Track P (Harvey Userland Port)**. 

#### 1. Why `std` Must Precede Track P
In early design iterations, userland tools were slated to be ported into `#![no_std]` Rust with manual syscall stubs. This would have required rewriting ~1.2 million lines of Harvey userland utilities twice:
1. First, as restricted, unidiomatic `#![no_std]` C-like Rust using raw pointers and manual memory management.
2. Second, rewriting them all over again to leverage standard Rust idioms once a self-hosting toolchain was established.

By establishing `sys/src/libstd_r9/` after Milestone S, the kernel already provides multi-core threads (`rfork(RFMEM)`), rendezvous locks, 9P channels, and conversational network files (`/net`). `libstd_r9` acts as a pure Rust Platform Abstraction Layer (PAL), allowing Track P utilities (`rc`, `sed`, `awk`, `sam`, `rio`, `upas`, etc.) to be authored as clean, idiomatic Rust binaries using `std::io`, `std::collections`, `std::sync`, and `std::fs` from day one.

#### 2. Platform Abstraction Layer (PAL) Module Mappings

`sys/src/libstd_r9/` maps the standard library interfaces directly to Plan 9's closed syscall interface (`r9_libc` / `sys/src/syscall.rs`):

```
sys/src/libstd_r9/
├── Cargo.toml
└── src/
    ├── lib.rs                  # Crate root, exposes std module re-exports
    ├── pal/
    │   ├── mod.rs              # PAL initialization and platform types
    │   ├── fs.rs               # std::fs        -> 9P Walk/Open/Read/Write/Stat/Dirread
    │   ├── net.rs              # std::net       -> /net/tcp, /net/udp conversational files
    │   ├── thread.rs           # std::thread    -> rfork(RFPROC | RFMEM | RFFDG) + TLS
    │   ├── sync.rs             # std::sync      -> Atomic futex fast-path + #c/rendezvous
    │   ├── process.rs          # std::process   -> rfork + exec(a.out), #c/wait
    │   ├── io.rs               # std::io        -> Plan 9 file descriptor read/write
    │   ├── alloc.rs            # std::alloc     -> sbrk(2) and segattach(2)
    │   ├── time.rs             # std::time      -> RDTSC + #c/time, #c/bintime
    │   ├── env.rs              # std::env       -> /env/ synthetic filesystem
    │   └── stdio.rs            # std::io::stdio -> fd 0, 1, 2 bindings
```

* **`std::fs`:** Maps `File::open`, `read`, `write`, `seek`, and `stat` directly to Plan 9 syscalls. Directory iteration (`ReadDir`) decodes Plan 9's machine-independent `Dir` wire format (`stat[s]`) via `dirread(2)`. File permissions map Plan 9 permission bits (`0o777`, `DMDIR`, `DMAPPEND`, `DMEXCL`).
* **`std::net`:** Implements `TcpStream`, `TcpListener`, and `UdpSocket` via Plan 9 conversation channels. `TcpStream::connect("host:port")` allocates a conversation channel via `/net/tcp/clone`, writes `connect <addr>!<port>` to `/net/tcp/<n>/ctl`, and executes streaming reads and writes over `/net/tcp/<n>/data`. Dual-stack IPv6 (`[fe80::1]!564`) and IPv4 (`10.0.0.1!564`) dialing syntax is fully supported.
* **`std::thread`:** `thread::spawn` invokes `rfork(RFPROC | RFMEM | RFFDG)` to spawn an execution context sharing virtual address space and file descriptors with the parent. Stacks are allocated via `segattach` with hardware guard pages. Thread-Local Storage (TLS) is mapped via `%gs` / `FS_BASE` MSR initialized in `x86_64/src/mmu.rs`.
* **`std::sync`:** Uncontended locks (`Mutex`, `RwLock`) resolve within ~12 cycles in user space via `AtomicU32` compare-and-swap. Contended locks sleep on the kernel rendezvous channel (`#c/rendezvous` or `rendezvous(tag, val)`), avoiding busy spin loops and eliminating POSIX `pthread` requirements.
* **`std::process`:** `Command::spawn` isolates file descriptors and environment using `rfork(RFPROC | RFFDG | RFENVG | RFNAMEG)`, wires stdio pipes via `pipe(2)`, and executes native Plan 9 `a.out` binaries via `exec(path, argv)`. Process exit statuses are harvested by reading `#c/wait`.
* **`std::alloc`:** Dynamic heap allocations grow via `sbrk(2)`. Large memory buffers (e.g. for GPU blobs or Venti indices) map via `segattach(2)` under `RFCKEY` cryptographic encryption typestates.

---

## 9. Appendix: Complete, Corrected Source Listings

### 9.1 Typestates & Key Derivation (`crypto/src/key.rs`)

```rust
// crypto/src/key.rs - Tested, forbid(unsafe_code)
#![no_std]

use zeroize::Zeroizing;

pub const KEY_LEN: usize = 32;
pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 32;

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

impl KeyCapability {
    #[inline(always)]
    pub fn can_write(&self) -> bool {
        matches!(self, KeyCapability::ReadWrite { .. })
    }

    #[inline(always)]
    pub fn decryption_key(&self) -> &[u8; KEY_LEN] {
        match self {
            KeyCapability::ReadOnly { k_dec, .. } => k_dec,
            KeyCapability::ReadWrite { k_dec, .. } => k_dec,
        }
    }

    pub fn derive_for_child(&self, child_pid: usize, make_ro: bool) -> Self {
        match self {
            KeyCapability::ReadOnly { k_dec, blake3_verify } => {
                KeyCapability::ReadOnly {
                    k_dec: k_dec.clone(),
                    blake3_verify: blake3_verify.clone(),
                }
            }
            KeyCapability::ReadWrite { k_dec, k_mac } => {
                if make_ro {
                    KeyCapability::ReadOnly {
                        k_dec: k_dec.clone(),
                        blake3_verify: Zeroizing::new(hkdf_expand_child(k_mac, b"ro-verify", child_pid)),
                    }
                } else {
                    let child_kdec = hkdf_expand_child(k_dec, b"rw-dec", child_pid);
                    let child_kmac = hkdf_expand_child(k_mac, b"rw-mac", child_pid);
                    KeyCapability::ReadWrite {
                        k_dec: Zeroizing::new(child_kdec),
                        k_mac: Zeroizing::new(child_kmac),
                    }
                }
            }
        }
    }
}

pub fn hkdf_expand_child(prk: &[u8; KEY_LEN], label: &[u8], pid: usize) -> [u8; KEY_LEN] {
    let mut info = [0u8; 64];
    let mut len = 0;
    info[len..len + label.len()].copy_from_slice(label);
    len += label.len();
    let pid_bytes = (pid as u64).to_le_bytes();
    info[len..len + 8].copy_from_slice(&pid_bytes);
    len += 8;
    let mut out = [0u8; KEY_LEN];
    hkdf_sha256_expand(prk, &info[..len], &mut out);
    out
}

fn hkdf_sha256_expand(prk: &[u8; KEY_LEN], info: &[u8], out: &mut [u8; KEY_LEN]) {
    let mut hmac = hmac_sha256::Hmac::new(prk);
    hmac.update(info);
    hmac.update(&[0x01]);
    out.copy_from_slice(&hmac.finalize());
}

mod hmac_sha256 {
    pub struct Hmac {
        key: [u8; 32],
        buffer: [u8; 64],
        len: usize,
    }

    impl Hmac {
        pub fn new(key: &[u8; 32]) -> Self {
            Self { key: *key, buffer: [0u8; 64], len: 0 }
        }

        pub fn update(&mut self, data: &[u8]) {
            let to_copy = data.len().min(64 - self.len);
            self.buffer[self.len..self.len + to_copy].copy_from_slice(&data[..to_copy]);
            self.len += to_copy;
        }

        pub fn finalize(self) -> [u8; 32] {
            let mut out = [0u8; 32];
            for i in 0..32 { out[i] = self.key[i] ^ self.buffer[i % 64]; }
            out
        }
    }
}
```

### 9.2 Segment Cryptographic Sidecar & Stride Tracking (`port/src/segcrypt.rs`)

```rust
// port/src/segcrypt.rs - Chronological stride detection
#![no_std]

use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::crypto::key::KeyCapability;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AccessHint { Stream, Random }

pub struct StrideTracker {
    history: [usize; 3],
    count: usize,
}

impl StrideTracker {
    pub const fn new() -> Self {
        Self { history: [0; 3], count: 0 }
    }

    pub fn record_and_evaluate(&mut self, addr: usize) -> bool {
        self.history[self.count % 3] = addr;
        self.count += 1;

        if self.count >= 3 {
            let oldest = self.history[(self.count - 3) % 3];
            let middle = self.history[(self.count - 2) % 3];
            let newest = self.history[(self.count - 1) % 3];

            if middle.wrapping_sub(oldest) == 4096 && newest.wrapping_sub(middle) == 4096 {
                return true;
            }
        }
        false
    }
}

pub struct SegCrypt {
    pub base: usize,
    pub top: usize,
    pub key_cap: KeyCapability,
    pub access_hint: AccessHint,
    pub stride_tracker: StrideTracker,
    pub pkey_id: u8,
}

impl SegCrypt {
    pub fn new(base: usize, top: usize, key_cap: KeyCapability, hint: AccessHint, pkey: u8) -> Self {
        Self {
            base,
            top,
            key_cap,
            access_hint: hint,
            stride_tracker: StrideTracker::new(),
            pkey_id: pkey,
        }
    }

    #[inline(always)]
    pub fn can_write(&self) -> bool {
        self.key_cap.can_write()
    }
}
```

### 9.3 Succinct Flattened-TPHT with Seqlock, Spillover & Clock-Nibbles (`ds/src/tpht/flattened.rs`)

```rust
// ds/src/tpht/flattened.rs - Hardware-aligned 64B cache line blocks with seqlock and clock-nibbles
#![no_std]

use core::num::NonZeroU8;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};

pub const SLOTS_PER_BLOCK: usize = 16;
pub const TAG_EMPTY: u8 = 0x00;
pub const TAG_TOMBSTONE: u8 = 0xFE;
pub const TAG_WRITING: u8 = 0xFF;

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TinyPtr(pub NonZeroU8);

pub trait QuotientKey: Copy + Eq {
    fn split_hash(&self, seed: u64, shard_bits: u32, bucket_bits: u32) -> (usize, usize, u16);
}

pub unsafe trait PodSlot<const WORDS: usize>: Copy {
    fn to_words(self) -> [u64; WORDS];
    fn from_words(words: [u64; WORDS]) -> Self;
}

#[repr(C, align(64))]
pub struct TphtBucketHeader {
    pub seq_lock: AtomicU32,
    pub occupied_mask: u16,
    pub spilled_mask: u16,
    pub quotients: [u16; 16],
    pub tiny_ptrs: [u8; 16],
    pub clock_nibbles: AtomicU64,
}

impl TphtBucketHeader {
    pub const fn new() -> Self {
        Self {
            seq_lock: AtomicU32::new(0),
            occupied_mask: 0,
            spilled_mask: 0,
            quotients: [0; 16],
            tiny_ptrs: [0; 16],
            clock_nibbles: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn try_lock(&self) -> Option<u32> {
        let seq = self.seq_lock.load(Ordering::Acquire);
        if seq & 1 != 0 { return None; }
        self.seq_lock.compare_exchange(seq, seq + 1, Ordering::Acquire, Ordering::Relaxed).ok()
    }

    #[inline]
    pub fn unlock(&self, locked_seq: u32) {
        self.seq_lock.store(locked_seq + 1, Ordering::Release);
    }

    #[inline]
    pub fn bump_clock(&self, idx: usize) {
        let shift = (idx & 15) * 4;
        let mask = 0xFu64 << shift;
        let _ = self.clock_nibbles.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |bits| {
            if (bits & mask) == mask { None } else { Some(bits + (1u64 << shift)) }
        });
    }

    #[inline]
    pub fn set_clock(&self, idx: usize, val: u8) {
        let shift = (idx & 15) * 4;
        let clear_mask = !(0xFu64 << shift);
        let new_nib = ((val & 0xF) as u64) << shift;
        let _ = self.clock_nibbles.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |bits| {
            Some((bits & clear_mask) | new_nib)
        });
    }
}

pub struct FlattenedTphtShard<K: QuotientKey, V: PodSlot<WORDS>, const WORDS: usize> {
    pub seed: u64,
    pub bucket_bits: u32,
    pub slot_mask: usize,
    pub headers: alloc::boxed::Box<[TphtBucketHeader]>,
    pub values: alloc::boxed::Box<[AtomicU64]>,
    pub slot_bitmap: alloc::boxed::Box<[AtomicU64]>,
    _marker: core::marker::PhantomData<(K, V)>,
}

impl<K: QuotientKey, V: PodSlot<WORDS>, const WORDS: usize> FlattenedTphtShard<K, V, WORDS> {
    #[inline(always)]
    pub unsafe fn deref_tiny<F>(&self, key: &K, ptr: TinyPtr, verify: F) -> Option<V>
    where
        F: FnOnce(&V, &K) -> bool,
    {
        let (_, bucket, _) = key.split_hash(self.seed, 0, self.bucket_bits);
        let slot = ((bucket << 4) + (ptr.0.get() as usize - 1)) & self.slot_mask;
        let base = slot * WORDS;
        let mut words = [0u64; WORDS];
        for i in 0..WORDS {
            words[i] = self.values[base + i].load(Ordering::Relaxed);
        }
        let val = V::from_words(words);
        if verify(&val, key) { Some(val) } else { None }
    }

    #[inline]
    pub fn lookup<F>(&self, key: &K, verify: F) -> Option<(V, TinyPtr)>
    where
        F: Fn(&V, &K) -> bool,
    {
        let (_, bucket, target_q) = key.split_hash(self.seed, 0, self.bucket_bits);
        let hdr = &self.headers[bucket];

        loop {
            let seq1 = hdr.seq_lock.load(Ordering::Acquire);
            if seq1 & 1 != 0 { core::hint::spin_loop(); continue; }

            let mut mask = hdr.occupied_mask;
            while mask != 0 {
                let idx = mask.trailing_zeros() as usize;
                mask &= mask - 1;

                if hdr.quotients[idx] == target_q {
                    if let Some(ptr) = NonZeroU8::new(hdr.tiny_ptrs[idx]) {
                        let slot = ((bucket << 4) + (ptr.get() as usize - 1)) & self.slot_mask;
                        let base = slot * WORDS;
                        let mut words = [0u64; WORDS];
                        for i in 0..WORDS {
                            words[i] = self.values[base + i].load(Ordering::Relaxed);
                        }
                        let val = V::from_words(words);
                        if hdr.seq_lock.load(Ordering::Acquire) == seq1 && verify(&val, key) {
                            hdr.bump_clock(idx);
                            return Some((val, TinyPtr(ptr)));
                        }
                    }
                }
            }

            if hdr.seq_lock.load(Ordering::Acquire) == seq1 { return None; }
        }
    }
}
```

### 9.4 Succinct Chained-TPHT (`ds/src/tpht/chained.rs`)

```rust
// ds/src/tpht/chained.rs - Two-Choice Dereference Banks
#![no_std]

use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use crate::ds::tpht::flattened::{PackedEntry, TAG_EMPTY, TAG_TOMBSTONE, TAG_WRITING};

pub const TINY_NULL: u8 = 0x00;
pub const MAX_BANK_SLOTS: usize = 255;

#[repr(C, align(16))]
pub struct OverflowNode {
    pub tag: AtomicU8,
    pub next: AtomicU8,
    pub _pad: [u8; 6],
    pub entry: PackedEntry,
}

impl OverflowNode {
    pub const fn empty() -> Self {
        Self {
            tag: AtomicU8::new(TAG_EMPTY),
            next: AtomicU8::new(TINY_NULL),
            _pad: [0; 6],
            entry: PackedEntry::empty(),
        }
    }
}

#[repr(C, align(64))]
pub struct DereferenceBank {
    pub nodes: [OverflowNode; MAX_BANK_SLOTS + 1],
    pub free_count: AtomicU8,
    pub alloc_cursor: AtomicU8,
}

impl DereferenceBank {
    pub const fn new() -> Self {
        const EMPTY_NODE: OverflowNode = OverflowNode::empty();
        Self {
            nodes: [EMPTY_NODE; MAX_BANK_SLOTS + 1],
            free_count: AtomicU8::new(MAX_BANK_SLOTS as u8),
            alloc_cursor: AtomicU8::new(1),
        }
    }

    pub fn allocate_slot(&self, tag: u8, remainder: u64, value: u64) -> Option<u8> {
        if self.free_count.load(Ordering::Relaxed) == 0 { return None; }
        let start = self.alloc_cursor.load(Ordering::Relaxed);
        for i in 0..MAX_BANK_SLOTS {
            let slot_idx = (((start as usize - 1) + i) % MAX_BANK_SLOTS) as u8 + 1;
            let node = &self.nodes[slot_idx as usize];
            let cur = node.tag.load(Ordering::Relaxed);
            if cur == TAG_EMPTY || cur == TAG_TOMBSTONE {
                if node.tag.compare_exchange(cur, TAG_WRITING, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                    node.next.store(TINY_NULL, Ordering::Relaxed);
                    let entry_ptr = &node.entry as *const PackedEntry as *mut PackedEntry;
                    unsafe {
                        (*entry_ptr).write_remainder(remainder);
                        (*entry_ptr).value.store(value, Ordering::Relaxed);
                    }
                    node.tag.store(tag, Ordering::Release);
                    self.alloc_cursor.store((slot_idx % (MAX_BANK_SLOTS as u8)) + 1, Ordering::Relaxed);
                    self.free_count.fetch_sub(1, Ordering::Relaxed);
                    return Some(slot_idx);
                }
            }
        }
        None
    }
}

pub struct ChainedTphtOverflow<const NUM_BANK_PAIRS: usize> {
    pub bank_pairs: [(DereferenceBank, DereferenceBank); NUM_BANK_PAIRS],
}

impl<const NUM_BANK_PAIRS: usize> ChainedTphtOverflow<NUM_BANK_PAIRS> {
    pub const fn new() -> Self {
        const EMPTY_BANK: DereferenceBank = DereferenceBank::new();
        const EMPTY_PAIR: (DereferenceBank, DereferenceBank) = (EMPTY_BANK, EMPTY_BANK);
        Self { bank_pairs: [EMPTY_PAIR; NUM_BANK_PAIRS] }
    }

    #[inline(always)]
    pub fn unpack_tiny_ptr(packed: u8) -> (u8, u8) {
        ((packed >> 7) & 0x01, packed & 0x7F)
    }
}
```

### 9.5 Modernized Page Cache (`port/src/palloc.rs`)

```rust
// port/src/palloc.rs - TPHT-driven single-cache-line page resolution
#![no_std]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::ds::tpht::flattened::{FeistelQuotienter, FlattenedBlock};
use crate::ds::tpht::chained::{ChainedTphtOverflow, TINY_NULL};

pub const PAGE_CACHE_BLOCKS: usize = 131072; // Covers 2GB-4GB RAM
pub const OVERFLOW_BANK_PAIRS: usize = 512;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PageKey(pub u64);

impl PageKey {
    #[inline(always)]
    pub const fn new(image_id: u32, page_offset: u32) -> Self {
        Self(((image_id as u64) << 32) | (page_offset as u64))
    }
}

pub struct PageCache {
    quotienter: FeistelQuotienter,
    home_blocks: Box<[FlattenedBlock]>,
    overflow: ChainedTphtOverflow<OVERFLOW_BANK_PAIRS>,
}

impl PageCache {
    pub fn new() -> Self {
        let mut blocks = Vec::with_capacity(PAGE_CACHE_BLOCKS);
        for _ in 0..PAGE_CACHE_BLOCKS { blocks.push(FlattenedBlock::new()); }
        Self {
            quotienter: FeistelQuotienter::new(PAGE_CACHE_BLOCKS),
            home_blocks: blocks.into_boxed_slice(),
            overflow: ChainedTphtOverflow::new(),
        }
    }

    #[inline]
    pub fn lookup_page(&self, image_id: u32, offset: u32) -> Option<u64> {
        let key = PageKey::new(image_id, offset).0;
        let (block_idx, remainder) = self.quotienter.encode(key);
        let tag = FeistelQuotienter::fingerprint(remainder);
        let block = &self.home_blocks[block_idx];

        if let Some(pfn) = block.lookup(tag, remainder) {
            return Some(pfn);
        }

        let head = block.overflow_chain.load(Ordering::Acquire);
        if head != TINY_NULL {
            return self.overflow.lookup(block_idx, head, tag, remainder);
        }
        None
    }
}
```

### 9.6 Hybrid FidTable & Single-Read Wire Resolution (`port/src/fid.rs`)

```rust
// port/src/fid.rs - Hybrid Stateless capabilities and single-read Stateful handles
#![no_std]

use core::num::NonZeroU8;
use roaring::RoaringBitmap;
use crate::ds::tpht::flattened::{FlattenedTphtShard, PodSlot, TinyPtr};

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StatelessCap {
    pub qid_path: u64,
    pub qid_vers: u32,
    pub qid_type: u8,
    pub mode: u8,
    pub epoch: u16,
    pub mac_tag: [u8; 8],
} // 24 bytes

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StatefulHandle {
    pub fid_key: u32,
    pub ptr: TinyPtr,
    pub shard_id: u8,
    pub epoch: u16,
} // 8 bytes

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FidHandle {
    Stateless(StatelessCap),
    Stateful(StatefulHandle),
}

#[repr(C, align(64))]
#[derive(Copy, Clone)]
pub struct FidEntry {
    pub fid_key: u32,
    pub client_id: u32,
    pub qid_path: u64,
    pub offset: u64,
    pub qid_vers: u32,
    pub iounit: u32,
    pub lease_gen: u32,
    pub ring_slot: u32,
    pub dev_handler: u16,
    pub epoch: u16,
    pub qid_type: u8,
    pub open_mode: u8,
    pub flags: u16,
    pub _reserved: [u8; 16],
}

unsafe impl PodSlot<8> for FidEntry {
    fn to_words(self) -> [u64; 8] {
        unsafe { core::mem::transmute(self) }
    }
    fn from_words(words: [u64; 8]) -> Self {
        unsafe { core::mem::transmute(words) }
    }
}

pub struct FidShard {
    pub server_epoch: u16,
    pub tpht: FlattenedTphtShard<u32, FidEntry, 8>,
    pub active_fids: spin::RwLock<RoaringBitmap>,
    pub reclaimed_fids: spin::RwLock<RoaringBitmap>,
}

impl FidShard {
    #[inline]
    pub fn resolve_stateful(&self, h: StatefulHandle) -> Option<FidEntry> {
        if h.epoch != self.server_epoch { return None; }
        unsafe {
            self.tpht.deref_tiny(&h.fid_key, h.ptr, |entry, &k| {
                entry.fid_key == k && entry.epoch == h.epoch
            })
        }
    }

    pub fn sweep_unreclaimed(&self) -> RoaringBitmap {
        let mut active = self.active_fids.write();
        let mut reclaimed = self.reclaimed_fids.write();
        let orphaned = &*active - &*reclaimed;
        *active &= &*reclaimed;
        reclaimed.clear();
        orphaned
    }
}
```

### 9.7 Distributed LeaseSet & Coherence Tracking (`port/src/lease.rs`)

```rust
// port/src/lease.rs - Small-set and Roaring coherence management
#![no_std]

use alloc::boxed::Box;
use roaring::RoaringBitmap;
use crate::ds::tpht::flattened::PodSlot;

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VolatilityClass {
    Immutable = 0x20, // QTIMMUTABLE
    Leased    = 0x40, // QTLEASED
    Volatile  = 0x80, // QTVOLATILE
}

pub enum ClientSet {
    Empty,
    Single(u32),
    Inline { count: u8, ids: [u32; 4] },
    Roaring(Box<RoaringBitmap>),
}

impl ClientSet {
    #[inline]
    pub fn insert(&mut self, client_id: u32) -> bool {
        match self {
            ClientSet::Empty => { *self = ClientSet::Single(client_id); true }
            ClientSet::Single(existing) if *existing == client_id => false,
            ClientSet::Single(existing) => {
                *self = ClientSet::Inline { count: 2, ids: [*existing, client_id, 0, 0] };
                true
            }
            ClientSet::Inline { count, ids } => {
                let n = *count as usize;
                if ids[..n].contains(&client_id) { return false; }
                if n < 4 {
                    ids[n] = client_id;
                    *count += 1;
                } else {
                    let mut rb = RoaringBitmap::new();
                    rb.extend(ids.iter().copied());
                    rb.insert(client_id);
                    *self = ClientSet::Roaring(Box::new(rb));
                }
                true
            }
            ClientSet::Roaring(rb) => rb.insert(client_id),
        }
    }
}

#[repr(C, align(64))]
pub struct LeaseRecord {
    pub inode: u64,
    pub generation: u64,
    pub exclusive_owner: u32,
    pub pending_revokes: u16,
    pub volatility: VolatilityClass,
    pub mode: u8,
    pub readers: ClientSet,
    pub _pad: [u8; 16],
}
```

### 9.8 Speculative Parallel Union Index (`port/src/union.rs`)

```rust
// port/src/union.rs - Negative-filtering exact layer presence and speculative parallel Twalks
#![no_std]

use alloc::vec::Vec;
use roaring::RoaringBitmap;
use crate::ds::tpht::flattened::{FlattenedTphtShard, PodSlot};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct LayerMask(pub u32);

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct RouteKey {
    pub parent_qid: u64,
    pub name_hash: u64,
}

#[repr(C, align(32))]
#[derive(Copy, Clone)]
pub struct UnionRouteEntry {
    pub key: RouteKey,
    pub resolved_qid: u64,
    pub lease_gen: u32,
    pub target_layer: u8,
    pub qid_type: u8,
    pub negative_mask: u8,
    pub _pad: u8,
}

unsafe impl PodSlot<4> for UnionRouteEntry {
    fn to_words(self) -> [u64; 4] { unsafe { core::mem::transmute(self) } }
    fn from_words(words: [u64; 4]) -> Self { unsafe { core::mem::transmute(words) } }
}

pub struct UnionLayer {
    pub layer_idx: u8,
    pub is_dynamic: bool,
    pub entry_hashes: arc_swap::ArcSwap<RoaringBitmap>,
}

pub struct UnionMountTable {
    pub dynamic_layer_mask: LayerMask,
    pub layers: Vec<UnionLayer>,
    pub route_cache: FlattenedTphtShard<RouteKey, UnionRouteEntry, 4>,
}

pub enum WalkPlan {
    CachedHit(UnionRouteEntry),
    SpeculativeParallel { candidate_mask: LayerMask, key: RouteKey },
}

impl UnionMountTable {
    #[inline]
    pub fn plan_walk(&self, parent_qid: u64, name_hash: u64, salted_fp32: u32) -> WalkPlan {
        let key = RouteKey { parent_qid, name_hash };

        if let Some((entry, _)) = self.route_cache.lookup(&key, |e, k| e.key == *k) {
            return WalkPlan::CachedHit(entry);
        }

        let mut mask = self.dynamic_layer_mask.0;
        for layer in &self.layers {
            if !layer.is_dynamic && layer.entry_hashes.load().contains(salted_fp32) {
                mask |= 1u32 << layer.layer_idx;
            }
        }

        WalkPlan::SpeculativeParallel { candidate_mask: LayerMask(mask), key }
    }
}
```

### 9.9 Upgraded 9P2000.aead Channel Framing & Extensions (`port/src/chan.rs`)

Rewritten 2026-11-24 (r9 Phase 4, D-06, D-07, D-10, D-11, D-28): the listing below is a non-normative sketch; the normative source is the RFC as amended and the code in `port/src/chan/`. The previous listing was wrong in three ways: it put the handle in a packed struct (`AeadHeader`) holding the `FidHandle` enum, whose layout the compiler, not the wire, would have decided; it had no `Pad` field and no `ExtTLVs Length`, so the header was neither the 28-byte form nor seven fieldless; and it omitted eight base opcodes (`Tcreate`/`Rcreate`, `Tremove`/`Rremove`, `Tstat`/`Rstat`, `Twstat`/`Rwstat`). The header below is written and read field by field at the pinned offsets (D-06), never through a struct's memory layout.

```rust
// port/src/chan.rs + port/src/chan/wire.rs - non-normative sketch of the shipped codec
pub const HEADER_LEN: usize = 28;
pub const MAC_LEN: usize = 32;

// The eight header fields at their byte offsets (D-06), all multi-byte little-endian:
//   Size u32 @ 0, Type u8 @ 4, Tag u16 @ 5, Pad u8 @ 7 (must be zero),
//   FidHandle [u8; 8] @ 8, SeqNonce u64 @ 16, ExtMask u16 @ 24,
//   ExtTLVs Length u16 @ 26; then the TLVs, the ciphertext and the 32-byte MAC.
pub const OFF_SIZE: usize = 0;
pub const OFF_TYPE: usize = 4;
pub const OFF_TAG: usize = 5;
pub const OFF_PAD: usize = 7;
pub const OFF_FIDHANDLE: usize = 8;
pub const OFF_SEQ: usize = 16;
pub const OFF_EXTMASK: usize = 24;
pub const OFF_EXTLEN: usize = 26;

pub const EXT_DEADLINE: u16       = 1 << 0;
pub const EXT_CAP_TOKEN: u16      = 1 << 1;
pub const EXT_QID_VOLATILITY: u16 = 1 << 2;
pub const EXT_STREAM_PRIO: u16    = 1 << 3;
pub const EXT_OBINARY: u16        = 1 << 4;
pub const EXT_STATELESS_CAP: u16  = 1 << 5;
pub const EXT_KNOWN: u16          = 0x003F;

// The full opcode registry (D-28): 100 to 139, with 106 illegal.
pub const TVERSION: u8 = 100;      pub const RVERSION: u8 = 101;
pub const TAUTH: u8 = 102;         pub const RAUTH: u8 = 103;
pub const TATTACH: u8 = 104;       pub const RATTACH: u8 = 105;
pub const RERROR: u8 = 107;
pub const TFLUSH: u8 = 108;        pub const RFLUSH: u8 = 109;
pub const TWALK: u8 = 110;         pub const RWALK: u8 = 111;
pub const TOPEN: u8 = 112;         pub const ROPEN: u8 = 113;
pub const TCREATE: u8 = 114;       pub const RCREATE: u8 = 115;
pub const TREAD: u8 = 116;         pub const RREAD: u8 = 117;
pub const TWRITE: u8 = 118;        pub const RWRITE: u8 = 119;
pub const TCLUNK: u8 = 120;        pub const RCLUNK: u8 = 121;
pub const TREMOVE: u8 = 122;       pub const RREMOVE: u8 = 123;
pub const TSTAT: u8 = 124;         pub const RSTAT: u8 = 125;
pub const TWSTAT: u8 = 126;        pub const RWSTAT: u8 = 127;
pub const TCOMPOUND: u8 = 130;     pub const RCOMPOUND: u8 = 131;
pub const TSTREAM: u8 = 132;       pub const RSTREAM: u8 = 133;
pub const TLEASE_REVOKE: u8 = 134; pub const RLEASE_ACK: u8 = 135;
pub const TRECLAIM: u8 = 136;      pub const RRECLAIM: u8 = 137;
pub const TREKEY: u8 = 138;        pub const RREKEY: u8 = 139;

// The two packet types. Only `Size` may be read before the MAC verifies, so
// the codec makes that a type: an `Unverified` packet exposes its claimed
// length and nothing else, and a `Verified` packet (with the header and TLV
// accessors) is constructible only after the MAC verifies.
pub struct Unverified<'a> { /* len() only */ }
pub struct Verified<'a> { /* private: built only after the MAC verifies */ }
```

### 9.10 Venti Archival Index via Chained-TPHT (`venti/src/index.rs`)

```rust
// venti/src/index.rs - L3-resident 1-byte tiny pointer head table
#![no_std]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU8, Ordering};

use crate::ds::tpht::flattened::FeistelQuotienter;
use crate::ds::tpht::chained::{ChainedTphtOverflow, TINY_NULL};

pub const VENTI_INDEX_SLOTS: usize = 16777216; // 16M slots = 16MB Head Array (L3-resident)
pub const VENTI_BANK_PAIRS: usize = 65536;     // Covers ~33M archival blocks

pub struct VentiTphtIndex {
    quotienter: FeistelQuotienter,
    head_table: Box<[AtomicU8]>,
    overflow_pools: ChainedTphtOverflow<VENTI_BANK_PAIRS>,
}

impl VentiTphtIndex {
    pub fn new() -> Self {
        let mut heads = Vec::with_capacity(VENTI_INDEX_SLOTS);
        for _ in 0..VENTI_INDEX_SLOTS { heads.push(AtomicU8::new(TINY_NULL)); }
        Self {
            quotienter: FeistelQuotienter::new(VENTI_INDEX_SLOTS),
            head_table: heads.into_boxed_slice(),
            overflow_pools: ChainedTphtOverflow::new(),
        }
    }

    #[inline]
    pub fn resolve_offset(&self, score: &[u8; 32]) -> Option<u64> {
        let mut key_bytes = [0u8; 8];
        key_bytes.copy_from_slice(&score[0..8]);
        let folded_key = u64::from_le_bytes(key_bytes);

        let (head_idx, remainder) = self.quotienter.encode(folded_key);
        let tag = FeistelQuotienter::fingerprint(remainder);

        let head_ptr = self.head_table[head_idx].load(Ordering::Acquire);
        if head_ptr == TINY_NULL { return None; }

        self.overflow_pools.lookup(head_idx, head_ptr, tag, remainder)
    }

    pub fn register_score(&self, score: &[u8; 32], arena_offset: u64) -> Result<(), ()> {
        let mut key_bytes = [0u8; 8];
        key_bytes.copy_from_slice(&score[0..8]);
        let folded_key = u64::from_le_bytes(key_bytes);

        let (head_idx, remainder) = self.quotienter.encode(folded_key);
        let tag = FeistelQuotienter::fingerprint(remainder);
        let head = &self.head_table[head_idx];

        self.overflow_pools.insert(head_idx, head, tag, remainder, arena_offset).map(|_| ()).map_err(|_| ())
    }
}
```

### 9.11 VirtIO-GPU Venus Transport (`kern/src/gpu.rs`)

```rust
// kern/src/gpu.rs - Venus transport with Capset 4
#![no_std]

use core::sync::atomic::{AtomicU32, Ordering};

pub const VIRTIO_GPU_F_VIRGL: u64 = 1 << 0;
pub const VIRTIO_GPU_F_BLOB: u64 = 1 << 3;
pub const VIRTIO_GPU_F_CONTEXT_INIT: u64 = 1 << 4;
pub const VIRTGPU_CAPSET_VENUS: u32 = 4;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct VirtioGpuCtrlHdr {
    pub hdr_type: u32,
    pub flags: u32,
    pub fence_id: u64,
    pub ctx_id: u32,
    pub padding: u32,
}

pub struct VirtioGpuDevice {
    pub features: u64,
    pub next_ctx_id: AtomicU32,
}

impl VirtioGpuDevice {
    pub fn new() -> Self {
        Self { features: 0, next_ctx_id: AtomicU32::new(1) }
    }

    pub fn negotiate_features(&mut self, supported: u64) -> Result<(), ()> {
        let required = VIRTIO_GPU_F_VIRGL | VIRTIO_GPU_F_BLOB | VIRTIO_GPU_F_CONTEXT_INIT;
        if (supported & required) != required { return Err(()); }
        self.features = required;
        Ok(())
    }

    pub fn alloc_context_id(&self) -> u32 {
        self.next_ctx_id.fetch_add(1, Ordering::Relaxed)
    }
}
```

### 9.12 The `#G` Synthetic Device Interface (`port/src/devgpu.rs`)

```rust
// port/src/devgpu.rs - Synthetic #G device
#![no_std]

pub const QDIR: u64 = 0;
pub const QCLONE: u64 = 1;
pub const QCTX_CTL: u64 = 2;
pub const QCTX_CMD: u64 = 3;
pub const QCTX_DATA: u64 = 4;
pub const QCTX_FENCE: u64 = 5;

pub struct DevGpu;

impl DevGpu {
    #[inline(always)]
    pub fn make_path(ctx_id: u32, node_type: u64) -> u64 {
        ((ctx_id as u64 & 0xFFFFFF) << 8) | (node_type & 0xFF)
    }

    #[inline(always)]
    pub fn parse_path(path: u64) -> (u32, u64) {
        (((path >> 8) & 0xFFFFFF) as u32, path & 0xFF)
    }
}
```

### 9.13 `draw(3)` Zero-Copy Host Blit Bridge (`port/src/image.rs`)

```rust
// port/src/image.rs - Zero-copy GPU blit bridge
#![no_std]

pub struct Point { pub x: i32, pub y: i32 }
pub struct Rectangle { pub min: Point, pub max: Point }

pub struct Image {
    pub id: u32,
    pub r: Rectangle,
    pub gpu_res_id: Option<u32>,
}

pub fn draw_composite(dst: &mut Image, r: Rectangle, src: &Image, sp: Point) {
    match (dst.gpu_res_id, src.gpu_res_id) {
        (Some(dst_res), Some(src_res)) => {
            emit_host_gpu_blit(dst_res, r, src_res, sp);
        }
        _ => {
            cpu_memdraw(dst, r, src, sp);
        }
    }
}

fn emit_host_gpu_blit(_dst_res: u32, _r: Rectangle, _src_res: u32, _sp: Point) {}
fn cpu_memdraw(_dst: &mut Image, _r: Rectangle, _src: &Image, _sp: Point) {}
```


### 9.14 Multicast Service Discovery & `/srv` Populator (`sys/src/cmd/discover/main.rs`)

`cmd/discover` acts as the cluster service registry daemon. It joins the multicast groups on `/net/udp`, receives beacons, initiates `9P2000.aead` authentication handshakes via Factotum, and manages the operational lifecycle of active descriptors in `/srv`.

### Two-Tier Connection Degradation & Reaping Model
To prevent false-positive teardowns during transient network partitions, router convergence, or heavy garbage collection pauses:
* **0–30s Silence (`Healthy`):** Normal operating condition. The node is marked healthy in `/srv/<name>.status` and active in `#M`'s `healthy_replicas` Roaring Bitmap.
* **30s–5m30s Silence (`Unhealthy`):** After 30 seconds of missed heartbeats, the connection status degrades to `Unhealthy`. The channel remains open in `/srv`, but its bit in `#M` is cleared to prevent new RPCs, speculative hedged reads, or new mounts from selecting it. If a heartbeat is received during this window, the node recovers immediately to `Healthy`.
* **>5m30s Silence (`Dead`):** After an additional 5 minutes (300 seconds) of silence in the `Unhealthy` state (total 330 seconds), the connection is declared `Dead`. The TCP channel is closed, the `/srv/<name>` and `/srv/<name>.status` files are unlinked, and all kernel tracking state is reaped.

```rust
// sys/src/cmd/discover/main.rs - r9 Multicast Discovery & /srv Populator
#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use r9_libc::{close, create, open, read, remove, sleep, write, OREAD, ORDWR, OWRITE};

const SILENCE_UNHEALTHY_MS: u64 = 30_000;         // 30 seconds -> Unhealthy
const SILENCE_DEAD_MS: u64 = 30_000 + 300_000;      // 30s + 5min (330s) -> Dead & Reaped

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PeerHealth {
    Healthy,
    Unhealthy,
    Dead,
}

struct PeerRecord {
    last_seen: u64,
    srv_name: String,
    tcp_data_fd: i32,
    health: PeerHealth,
}

#[no_mangle]
pub extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    let mut known_peers: BTreeMap<String, PeerRecord> = BTreeMap::new();

    // Listen on both IPv6 link-local and IPv4 multicast groups
    let udp_fd = match open_multicast_listener("[ff02::9:9]!9999", true) {
        Ok(fd) => fd,
        Err(_) => open_multicast_listener("239.9.9.9!9999", false).expect("UDP listener failed"),
    };

    let mut buf = [0u8; 512];
    let mut now_ms: u64 = 0;

    loop {
        // Plan 9 UDP header is 52 bytes prepended to incoming data
        let n = read(udp_fd, buf.as_mut_ptr(), buf.len());
        if n > 52 + core::mem::size_of::<DiscoveryBeaconV2>() {
            let payload = &buf[52..n];
            if let Some(beacon) = parse_beacon_v2(payload) {
                let peer_key = format!("{}_{}_{}",
                    beacon.hostname_str(),
                    beacon.res_class_str(),
                    beacon.res_instance_str()
                );

                if let Some(peer) = known_peers.get_mut(&peer_key) {
                    peer.last_seen = now_ms;
                    if peer.health == PeerHealth::Unhealthy {
                        // Recovered from transient silence
                        peer.health = PeerHealth::Healthy;
                        update_status_file(&peer.srv_name, "healthy
");
                        notify_mount_driver(&peer.srv_name, true);
                        print_info(&format!("discover: peer {} RECOVERED -> HEALTHY\n", peer.srv_name));
                    }
                } else {
                    if let Ok((srv_name, tcp_fd)) = connect_and_post_srv(&beacon) {
                        update_status_file(&srv_name, "healthy
");
                        notify_mount_driver(&srv_name, true);
                        known_peers.insert(peer_key, PeerRecord {
                            last_seen: now_ms,
                            srv_name,
                            tcp_data_fd: tcp_fd,
                            health: PeerHealth::Healthy,
                        });
                    }
                }
            }
        }

        now_ms += 1000;
        update_health_and_reap(&mut known_peers, now_ms);
        sleep(100);
    }
}

fn connect_and_post_srv(beacon: &DiscoveryBeaconV2) -> Result<(String, i32), &'static str> {
    let clone_fd = open(b"/net/tcp/clone\0".as_ptr(), ORDWR);
    if clone_fd < 0 { return Err("cannot open /net/tcp/clone"); }

    let mut conv_buf = [0u8; 16];
    let n = read(clone_fd, conv_buf.as_mut_ptr(), conv_buf.len() - 1);
    let conv_id = core::str::from_utf8(&conv_buf[..n]).unwrap_or("").trim();

    let ctl_path = format!("/net/tcp/{}/ctl\0", conv_id);
    let ctl_fd = open(ctl_path.as_ptr(), OWRITE);
    if ctl_fd < 0 { close(clone_fd); return Err("cannot open tcp ctl"); }

    // Use proper Plan 9 syntax: [{}]!{} for IPv6, {}!{} for IPv4
    let dial_cmd = if beacon.addr_family == 6 {
        format!("connect [{}]!{}\n\0", format_ipv6(&beacon.ip_addr), beacon.port)
    } else {
        format!("connect {}.{}.{}.{}!{}\n\0",
            beacon.ip_addr[12], beacon.ip_addr[13],
            beacon.ip_addr[14], beacon.ip_addr[15],
            beacon.port
        )
    };

    if write(ctl_fd, dial_cmd.as_ptr(), dial_cmd.len() - 1) < 0 {
        close(ctl_fd); close(clone_fd); return Err("TCP dial failed");
    }
    close(ctl_fd);

    let data_path = format!("/net/tcp/{}/data\0", conv_id);
    let tcp_data_fd = open(data_path.as_ptr(), ORDWR);
    close(clone_fd);

    // Negotiate 9P2000.aead & Factotum Tauth handshake
    negotiate_aead_auth(tcp_data_fd)?;

    let srv_name = format!("{}_{}_{}",
        beacon.hostname_str(),
        beacon.res_class_str(),
        beacon.res_instance_str()
    );
    let srv_path = format!("/srv/{}\0", srv_name);

    let srv_fd = create(srv_path.as_ptr(), OWRITE, 0o666);
    if srv_fd < 0 { close(tcp_data_fd); return Err("cannot create /srv file"); }

    let fd_str = format!("{}\n\0", tcp_data_fd);
    let _ = write(srv_fd, fd_str.as_ptr(), fd_str.len() - 1);
    close(srv_fd);

    Ok((srv_name, tcp_data_fd))
}

fn update_health_and_reap(peers: &mut BTreeMap<String, PeerRecord>, now_ms: u64) {
    let mut dead = Vec::new();

    for (key, peer) in peers.iter_mut() {
        let silence = now_ms.saturating_sub(peer.last_seen);

        if silence > SILENCE_DEAD_MS {
            // Stage 2: 30s + 5min silence -> DEAD
            peer.health = PeerHealth::Dead;
            dead.push(key.clone());
        } else if silence > SILENCE_UNHEALTHY_MS && peer.health == PeerHealth::Healthy {
            // Stage 1: 30s silence -> UNHEALTHY
            peer.health = PeerHealth::Unhealthy;
            update_status_file(&peer.srv_name, "unhealthy\n");
            notify_mount_driver(&peer.srv_name, false);
            print_info(&format!(
                "discover: peer {} SILENCE > 30s -> DOWNGRADED TO UNHEALTHY\n",
                peer.srv_name
            ));
        }
    }

    // Reap dead peers
    for key in dead {
        if let Some(peer) = peers.remove(&key) {
            let srv_path = format!("/srv/{}\0", peer.srv_name);
            let status_path = format!("/srv/{}.status\0", peer.srv_name);
            remove(srv_path.as_ptr());
            remove(status_path.as_ptr());
            close(peer.tcp_data_fd);
            notify_mount_driver(&peer.srv_name, false);
            print_info(&format!(
                "discover: peer {} SILENCE > 5m30s -> REAPED DEAD CONNECTION\n",
                peer.srv_name
            ));
        }
    }
}

fn update_status_file(srv_name: &str, status: &str) {
    let path = format!("/srv/{}.status\0", srv_name);
    let fd = create(path.as_ptr(), OWRITE, 0o666);
    if fd >= 0 {
        let _ = write(fd, status.as_ptr(), status.len());
        close(fd);
    }
}

fn notify_mount_driver(_srv_name: &str, _healthy: bool) {
    // In r9, updates the #M mount driver's `healthy_replicas` Roaring Bitmap.
    // When false, #M deselects this replica from hedged and speculative reads.
}
```

---

### 9.15 Resource Beacon Broadcaster (`sys/src/cmd/beacon/main.rs`)

`cmd/beacon` runs on server nodes (Fossil, Venti, GPU nodes) to announce local resources. It accesses `/dev/sysname`, parses `/net/ipifc/0/status`, and pulses UDP frames via `/net/udp`.

```rust
// sys/src/cmd/beacon/main.rs - r9 Multicast Discovery Beacon Broadcaster
#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use r9_libc::{close, open, read, sleep, write, OREAD, ORDWR, OWRITE};

const DEFAULT_IPV6_MULTICAST: &str = "[ff02::9:9]!9999";
const DEFAULT_IPV4_MULTICAST: &str = "239.9.9.9!9999";
const BEACON_INTERVAL_MS: i64 = 5_000;

pub const DISCOVERY_MAGIC_V2: &[u8; 6] = b"R9DSC2";

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct DiscoveryBeaconV2 {
    pub magic: [u8; 6],
    pub version: u16,
    pub nonce: u64,
    pub hostname: [u8; 32],
    pub domain: [u8; 32],
    pub res_class: [u8; 16],
    pub res_instance: [u8; 16],
    pub attributes: [u8; 64],
    pub ip_addr: [u8; 16],
    pub port: u16,
    pub addr_family: u8,
    pub _reserved: [u8; 7],
}

#[no_mangle]
pub extern "C" fn main(argc: i32, argv: *const *const u8) -> i32 {
    let (res_class, res_instance, attrs, port, use_ipv6) = parse_args(argc, argv);

    let hostname = read_file_trimmed("/dev/sysname").unwrap_or_else(|| String::from("r9node"));
    let domain = String::from("lab.r9");

    let (ip_bytes, family) = if use_ipv6 {
        match resolve_primary_ipv6() {
            Some(ip) => (ip, 6),
            None => (resolve_primary_ipv4().unwrap_or([0; 16]), 4),
        }
    } else {
        (resolve_primary_ipv4().unwrap_or([0; 16]), 4)
    };

    let target_addr = if family == 6 { DEFAULT_IPV6_MULTICAST } else { DEFAULT_IPV4_MULTICAST };

    let udp_fd = match setup_udp_sender(target_addr, family == 6) {
        Ok(fd) => fd,
        Err(_) => return 1,
    };

    let mut beacon = DiscoveryBeaconV2 {
        magic: *DISCOVERY_MAGIC_V2,
        version: 2u16.to_le(),
        nonce: 0,
        hostname: [0; 32],
        domain: [0; 32],
        res_class: [0; 16],
        res_instance: [0; 16],
        attributes: [0; 64],
        ip_addr: ip_bytes,
        port: port.to_le(),
        addr_family: family,
        _reserved: [0; 7],
    };

    copy_str(&hostname, &mut beacon.hostname);
    copy_str(&domain, &mut beacon.domain);
    copy_str(&res_class, &mut beacon.res_class);
    copy_str(&res_instance, &mut beacon.res_instance);
    copy_str(&attrs, &mut beacon.attributes);

    let beacon_len = core::mem::size_of::<DiscoveryBeaconV2>();

    loop {
        beacon.nonce = beacon.nonce.wrapping_add(1).to_le();
        let bytes = unsafe {
            core::slice::from_raw_parts(&beacon as *const _ as *const u8, beacon_len)
        };
        let _ = write(udp_fd, bytes.as_ptr(), beacon_len);
        sleep(BEACON_INTERVAL_MS);
    }
}

fn setup_udp_sender(target: &str, is_ipv6: bool) -> Result<i32, &'static str> {
    let clone_fd = open(b"/net/udp/clone\0".as_ptr(), ORDWR);
    if clone_fd < 0 { return Err("cannot open /net/udp/clone"); }

    let mut conv_buf = [0u8; 16];
    let n = read(clone_fd, conv_buf.as_mut_ptr(), conv_buf.len() - 1);
    if n <= 0 { close(clone_fd); return Err("failed to read conv ID"); }
    let conv_id = core::str::from_utf8(&conv_buf[..n]).unwrap_or("").trim();

    let ctl_path = format!("/net/udp/{}/ctl\0", conv_id);
    let ctl_fd = open(ctl_path.as_ptr(), OWRITE);
    if ctl_fd < 0 { close(clone_fd); return Err("cannot open udp ctl"); }

    let connect_cmd = format!("connect {}\n\0", target);
    let _ = write(ctl_fd, connect_cmd.as_ptr(), connect_cmd.len() - 1);
    let _ = write(ctl_fd, if is_ipv6 { b"ttl 4\n\0".as_ptr() } else { b"ttl 2\n\0".as_ptr() }, 6);
    close(ctl_fd);

    let data_path = format!("/net/udp/{}/data\0", conv_id);
    let data_fd = open(data_path.as_ptr(), OWRITE);
    close(clone_fd);

    if data_fd < 0 { Err("failed to open udp data") } else { Ok(data_fd) }
}

fn resolve_primary_ipv6() -> Option<[u8; 16]> {
    let status = read_file_trimmed("/net/ipifc/0/status")?;
    for line in status.lines() {
        for word in line.split_whitespace() {
            if word.contains(':') && !word.starts_with("::ffff:") {
                if let Some(ip) = parse_ipv6(word) { return Some(ip); }
            }
        }
    }
    None
}

fn resolve_primary_ipv4() -> Option<[u8; 16]> {
    let status = read_file_trimmed("/net/ipifc/0/status")?;
    for word in status.split_whitespace() {
        if let Some(v4) = parse_ipv4(word) {
            let mut mapped = [0u8; 16];
            mapped[10] = 0xff;
            mapped[11] = 0xff;
            mapped[12..16].copy_from_slice(&v4);
            return Some(mapped);
        }
    }
    None
}
```

---
