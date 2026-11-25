# Plan r9 Architecture & Code Compendium (v3.0)

**Target Repositories:** `requaos/r9` (HEAD `900f58b`) and `requaos/harvey` (lock `c697666`)  
**Supersedes:** Plan r9 Architecture & Code Compendium (v2.1)  
**Date:** November 25, 2026

---

## 1. Architectural Mandate & v3.0 Direction Changes

**r9** is a pure-Rust reimplementation of Bell Labs' Plan 9 and Harvey operating systems built around Plan 9's core thesis: *"the network file protocol IS the operating system fabric."* Version 3.0 locks five strategic refinements based on Phase 1–4 implementation experience:

| Subsystem | v2.1 Baseline | v3.0 Authoritative Target |
| :--- | :--- | :--- |
| **Memory Isolation** | Software RAM encryption on `#PF` (`cryptpage.rs`, `ShadowFrame`, `ymm0-ymm5` asm, `PKEYs`) | **Dropped.** Standard hardware MMU isolation (`CR3`/`TTBR0`/`satp` + PTE `RO`/`RW`) uniformly across `x86_64`, `aarch64`, and `riscv64`. |
| **9P `FidTable`** | `FlattenedTphtShard<u32, FidEntry>` with `TinyPtr` | **Sharded Generational Slab** (`FidShard` with direct slot indexing + `RoaringBitmap` active/reclaimed sets). Zero bucket clustering; deterministic allocation. |
| **SMP Bring-Up** | Deferred to `Milestone S` (after Phase 3 & Track P) | **Pulled forward to Phase 3.** Minimal 2-CPU SMP (`q35 -smp 2`, `virt -smp 2`) lands *before* distributed 9P extensions and storage. |
| **Native Toolchain** | Port `6c`/`6a`/`6l` to Rust + dual Classic APE / APEX | **`rustc` + `rust-lld` + vendored APEX + `elf2aout`** as primary native toolchain. Harvey C compilers remain a bootstrap/compatibility layer only (no Rust rewrite). |
| **`ds/` Unsafe Policy** | Contradictory (`forbid` vs. `tpht` exception) | **Per-module `#![forbid(unsafe_code)]` layout** matching commit `25af196`. Only `ds/src/tpht/` permits audited `unsafe`. `crypto/` and `port/` are crate-wide `#![forbid(unsafe_code)]`. |

### Impact Summary: Dropping Software Cryptographic Memory

* **Eliminated:** Demand-paged ChaCha20/BLAKE3 `#PF` decryption (~4.4 $\mu\text{s}$ per cold fault on KVM), `ymm0-ymm5` restricted vector assembly, `MOVNTDQ` dual-path stores, interleaved `SFENCE` hiding, `CLFLUSHOPT` shadow-frame scrubbing, out-of-band tag slabs, `ShadowFrame`/`BackingStore` split paging, `StrideTracker` crypto prefetching, and `rfork(RFCKEY | RFROBASE | RFRANDMEM)` key-derivation machinery.
* **Retained:**
  * Uniform hardware MMU isolation (`CR3`/`TTBR0`/`satp` + PTE `RO`/`RW`) across `x86_64`, `aarch64`, and `riscv64`.
  * Pure-Rust `crypto/` powering `9P2000.aead` **network transit security** (HKDF-SHA256, ChaCha20, Keyed BLAKE3, Factotum `Tauth`, monotonic nonces, AAD + `ExtMask`, `StatelessCap` MACs).
  * Succinct `ds/` primitives (Flattened-TPHT, Chained-TPHT, Roaring Bitmaps, Sharded Slab) and cluster 9P extensions (`Tcompound`, `Tstream`, leases, union mount routing, `RFNOLEGACY`).
  * Checked `SegKind::Stack` bounds enforcement in `notes.rs` for safe note delivery.

### Unchanged Non-Negotiable Invariants

* **Binary Format:** Plan 9 `a.out` exclusively (static ELF emitted by `rust-lld` is converted via in-tree `elf2aout`).
* **Closed Syscall Table:** No POSIX `mmap`, `ioctl`, or `io_uring` syscalls. Shared rings and GPU transport are exposed as synthetic file trees (`#M`, `#G`) and `segattach`.
* **Multi-Architecture Parity:** Portable kernel logic lives strictly in `port/`; architecture crates (`x86_64/`, `aarch64/`, `riscv64/`) implement clean traits without leaking x86-only assumptions into `port/`.
* **Single Source of Truth:** Planning, Nix composition, and CI gate legs live in `requaos/r9` (`gates/`, `nix/`, `.planning/`).

---

## 2. Workspace Crate & Safety Boundaries

```text
r9/
├── crypto/         # #![forbid(unsafe_code)] — HKDF-SHA256, ChaCha20, Keyed BLAKE3, 9P session/cap tokens
├── ds/             # Per-module #![forbid(unsafe_code)]:
│   ├── src/roaring.rs      # #![forbid(unsafe_code)] — RoaringBitmap, SuperpageResidency, ClientSet
│   ├── src/slab.rs         # #![forbid(unsafe_code)] — Sharded generational slab for authoritative tables
│   └── src/tpht/           # Audited unsafe (deny undocumented_unsafe_blocks) — Flattened & Chained TPHT
├── port/           # #![forbid(unsafe_code)] — Portable kernel: 9P2000.aead, FidTable, leases, union, SMP traits, devgpu
├── x86_64/         # Audited unsafe — AP startup (SIPI), LAPIC/IPI, MMU, traps, VirtIO drivers
├── aarch64/        # Audited unsafe — PSCI CPU_ON, GICv2/v3 IPI, MMU, traps
├── riscv64/        # Audited unsafe — SBI HSM, PLIC/CLINT, MMU, typed MMIO
├── sys/include/    # Generated ABI headers (e.g., rfork.h from port::rfork)
├── xtask/          # Build, host tests, test-9p, boot-test, ureg-check, elf2aout
├── gates/ & nix/   # Relocated Harvey-composed boot/net/auth/rio gate legs
└── UNSAFE-AUDIT.md # Mandatory ledger for every unsafe block in ds/src/tpht/ and arch crates
```

---

## 3. Data Structure Selection Matrix (`ds/` Crate)

Each data structure is matched strictly to its failure-mode semantics:

| Kernel / Storage Subsystem | Data Structure | Rationale |
| :--- | :--- | :--- |
| **9P `FidTable`** (authoritative) | **Sharded Generational Slab** (`ds/src/slab.rs`) | Cannot evict live FIDs or fail on hash clustering; $O(1)$ direct index lookup via server-assigned `(shard_id, slot_idx, gen)`. |
| **Page Cache (`palloc.hash`)** | **Flattened-TPHT** (`ds/src/tpht/flattened.rs`) | Cache semantics tolerate `ClockNibble` eviction; single 64B cache-line hit via boot-salted Feistel quotienting. |
| **Union Mount Dcache (`UnionRouteEntry`)** | **Flattened-TPHT** (`ds/src/tpht/flattened.rs`) | Read-heavy cache with 4-bit `ClockNibble` second-chance eviction. |
| **Venti Score Index** | **Chained-TPHT** (`ds/src/tpht/chained.rs`) | Succinct 1-byte tiny pointers + 64-bit hint verified against full 32-byte BLAKE3 score on match. |
| **Frame Pools, Superpages, Deadlines, Union Filters, Leases, Fossil GC** | **RoaringBitmap** & **`ClientSet`** (`ds/src/roaring.rs`) | $O(1)$ 2MB superpage check (`SuperpageResidency`), SIMD set algebra ($\text{Dirty} \setminus (\text{Pinned} \cup \text{Active})$), zero-heap `ClientSet` for 1–4 clients. |

---

## 4. Reference Implementation: Sharded Generational Slab `FidTable` (`port/src/fid.rs`)

Replacing `FlattenedTphtShard` for `FidTable` eliminates both bucket-clustering allocation failures and the serialized insert lock workaround introduced in commit `6fd201c`.

* **Wire Handle Size:** `StatefulHandle` remains **exact 8 bytes**, preserving the `9P2000.aead` AAD layout and 4KB payload alignment.
* **Capacity:** $256 \text{ shards} \times 65{,}536 \text{ slots per shard} = 16{,}777{,}216$ concurrent stateful FIDs per server.
* **Lookup Cost:** **1 direct array read** (`shards[shard_id].slots[slot_idx]`) with generation and epoch verification—zero hashing, zero bucket probing.
* **Allocation Guarantees:** `alloc()` pops the lowest free slot from a shard-local free-list. It can never spuriously fail due to hash collisions; if a shard is full, the caller falls back to the next shard round-robin.
* **Safety:** 100% safe Rust (`#![forbid(unsafe_code)]`).

```rust
// port/src/fid.rs — Hybrid Stateless capabilities + Sharded Generational Slab Stateful handles
#![forbid(unsafe_code)]

extern crate alloc;
use alloc::vec::Vec;
use crate::ds::roaring::RoaringBitmap;
use crate::sync::RwLock; // SMP-safe spin/sleep RwLock

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StatelessCap {
    pub qid_path: u64,
    pub qid_vers: u32,
    pub qid_type: u8,
    pub mode: u8,
    pub epoch: u16,
    pub mac_tag: [u8; 8], // Truncated Keyed BLAKE3 MAC
} // 24 bytes — zero server RAM for QTIMMUTABLE files & static walks

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StatefulHandle {
    pub slot_idx: u16,  // Direct index into FidShard::slots (0..65535)
    pub gen: u16,       // Monotonic per-slot generation (ABA / UAF protection)
    pub shard_id: u8,   // Target shard (0..N_SHARDS-1)
    pub flags: u8,      // Handle capability flags (e.g., stream / lease / bypass)
    pub epoch: u16,     // Server restart epoch
} // 8 bytes exact — fits directly in 9P2000.aead AAD FidHandle field

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FidHandle {
    Stateless(StatelessCap),
    Stateful(StatefulHandle),
}

#[repr(C, align(64))]
#[derive(Copy, Clone, Debug)]
pub struct FidEntry {
    pub client_id: u32,
    pub qid_vers: u32,
    pub qid_path: u64,
    pub offset: u64,
    pub iounit: u32,
    pub lease_gen: u32,
    pub ring_slot: u32,
    pub dev_handler: u16,
    pub qid_type: u8,
    pub open_mode: u8,
    pub flags: u16,
    pub _reserved: [u8; 22],
} // 64 bytes exact (1 cache line)

struct SlabSlot {
    gen: u16,
    occupied: bool,
    entry: Option<FidEntry>,
}

struct ShardInner {
    slots: Vec<SlabSlot>,
    free_list: Vec<u16>,
    active_slots: RoaringBitmap,
    reclaimed_slots: RoaringBitmap,
}

#[repr(align(64))] // Prevent false sharing across SMP cores
pub struct FidShard {
    shard_id: u8,
    server_epoch: u16,
    max_slots: u16,
    inner: RwLock<ShardInner>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum FidError {
    ShardFull,
    StaleEpoch,
    InvalidHandle,
    SlotOccupied,
}

impl FidShard {
    pub fn new(shard_id: u8, server_epoch: u16, initial_cap: usize, max_slots: u16) -> Self {
        let mut slots = Vec::with_capacity(initial_cap);
        let mut free_list = Vec::with_capacity(initial_cap);
        for i in (0..initial_cap).rev() {
            slots.push(SlabSlot { gen: 1, occupied: false, entry: None });
            free_list.push(i as u16);
        }
        Self {
            shard_id,
            server_epoch,
            max_slots,
            inner: RwLock::new(ShardInner {
                slots,
                free_list,
                active_slots: RoaringBitmap::new(),
                reclaimed_slots: RoaringBitmap::new(),
            }),
        }
    }

    /// Deterministic O(1) allocation: never fails due to hash clustering.
    pub fn alloc(&self, entry: FidEntry, flags: u8) -> Result<StatefulHandle, FidError> {
        let mut g = self.inner.write();
        let slot_idx = if let Some(idx) = g.free_list.pop() {
            idx
        } else {
            let next = g.slots.len();
            if next >= self.max_slots as usize {
                return Err(FidError::ShardFull);
            }
            g.slots.push(SlabSlot { gen: 1, occupied: false, entry: None });
            next as u16
        };

        let slot = &mut g.slots[slot_idx as usize];
        slot.occupied = true;
        slot.entry = Some(entry);
        let gen = slot.gen;
        g.active_slots.insert(slot_idx as u32);

        Ok(StatefulHandle {
            slot_idx,
            gen,
            shard_id: self.shard_id,
            flags,
            epoch: self.server_epoch,
        })
    }

    /// O(1) direct-indexed read with epoch + generation validation.
    #[inline]
    pub fn resolve(&self, h: StatefulHandle) -> Result<FidEntry, FidError> {
        if h.epoch != self.server_epoch || h.shard_id != self.shard_id {
            return Err(FidError::StaleEpoch);
        }
        let g = self.inner.read();
        let slot = g.slots.get(h.slot_idx as usize).ok_or(FidError::InvalidHandle)?;
        if !slot.occupied || slot.gen != h.gen {
            return Err(FidError::InvalidHandle);
        }
        slot.entry.ok_or(FidError::InvalidHandle)
    }

    /// Clunks a stateful handle, incrementing generation to invalidate stale wire handles.
    pub fn clunk(&self, h: StatefulHandle) -> Result<FidEntry, FidError> {
        if h.epoch != self.server_epoch || h.shard_id != self.shard_id {
            return Err(FidError::StaleEpoch);
        }
        let mut g = self.inner.write();
        let slot = g.slots.get_mut(h.slot_idx as usize).ok_or(FidError::InvalidHandle)?;
        if !slot.occupied || slot.gen != h.gen {
            return Err(FidError::InvalidHandle);
        }
        slot.occupied = false;
        slot.gen = slot.gen.wrapping_add(1).max(1); // Skip 0
        let old = slot.entry.take().ok_or(FidError::InvalidHandle)?;
        g.active_slots.remove(h.slot_idx as u32);
        g.free_list.push(h.slot_idx);
        Ok(old)
    }

    /// Post-failover grace sweep: one SIMD ANDNOT removes all un-reclaimed handles.
    pub fn sweep_unreclaimed(&self) -> RoaringBitmap {
        let mut g = self.inner.write();
        let orphaned = &g.active_slots - &g.reclaimed_slots;
        for idx in orphaned.iter() {
            if let Some(slot) = g.slots.get_mut(idx as usize) {
                if slot.occupied {
                    slot.occupied = false;
                    slot.gen = slot.gen.wrapping_add(1).max(1);
                    slot.entry = None;
                    g.free_list.push(idx as u16);
                }
            }
        }
        let reclaimed = core::mem::take(&mut g.reclaimed_slots);
        g.active_slots &= &reclaimed;
        orphaned
    }
}
```

---

## 5. 9P2000.aead Wire Fabric & Cluster Extensions

With software cryptographic memory removed, **9P2000.aead** is cleanly decoupled from `#PF` paging:

* **Wire Envelope (unchanged):**
  $$\text{Packet} = \underbrace{\text{Size (4B)} \parallel \text{Type (1B)} \parallel \text{Tag (2B)} \parallel \text{FidHandle (8B)} \parallel \text{SeqNonce (8B)} \parallel \text{ExtMask (2B)} \parallel \text{ExtTLVs}}_{\text{AAD (Keyed BLAKE3)}} \parallel \underbrace{\text{Ciphertext (ChaCha20)}}_{4\text{KB Aligned}} \parallel \underbrace{\text{MAC (32B)}}_{\text{BLAKE3}}$$
* **Receive / Transmit Path:** Incoming `9P2000.aead` frames DMA into pinned kernel RX buffers, verify the BLAKE3 MAC (Encrypt-then-MAC), and decrypt via pure-Rust `crypto/` directly into standard kernel page-cache frames (or user-mapped `#M` ring buffers)—with zero `#PF` decrypt traps.
* **Distributed Extensions (Phase 4):** `Tcompound` atomic pipelines, `Tstream` flow control + Roaring P0–P2 priority/deadline wheel, `Qid` volatility (`QTIMMUTABLE`, `QTLEASED`, `QTVOLATILE`) + `ClientSet` leases, `UnionMountTable` with exact Roaring layer filters + `SpeculativeParallel` `Twalk`, legacy 9front/Linux gateway, and `rfork(RFNOLEGACY)`.

---

## 6. Early 2-CPU SMP Architecture (Phase 3)

Pulling minimal 2-CPU SMP ahead of distributed 9P, Track P, and storage ensures every concurrent primitive is verified under real multi-core interrupts:

* **Per-CPU State (`Mach`):** Dedicated `Mach` struct per core (`GS`-base on `x86_64`, `TPIDR_EL1` on `aarch64`) holding current `Proc`, IRQ nesting depth, and per-CPU scheduler runqueue pointer.
* **Locking Discipline:** Retire all single-CPU spinlock shortcuts. Locks acquired in interrupt context must use IRQ-disabling spinlock guards (`IrqSpinLock`) to prevent same-CPU IRQ deadlocks; process-context locks use ticket/MCS spinlocks or sleep-capable `RwLock`.
* **AP Bring-Up:**
  * `x86_64`: Parse ACPI MADT, send INIT-SIPI-SIPI via LAPIC, bring up AP1 in QEMU `q35 -smp 2`, wire TLB shootdown IPI vector.
  * `aarch64`: Bring up CPU1 via PSCI `CPU_ON` in QEMU `virt -smp 2`, wire GICv2/v3 SGI for reschedule and TLB shootdown.
* **Cross-Core Validation:** Run concurrent `FlattenedTphtShard` and `FidTable` stress legs + timer interrupts across both CPUs as a mandatory CI gate.

---

## 7. Storage, Graphics & Self-Hosting Toolchain Targets

* **Storage (Phase 6):** BLAKE3 Venti CAS store backed by Chained-TPHT score index (32-byte BLAKE3 verification on match); Fossil snapshotting with RoaringBitmap mark-and-sweep GC and NVMe TRIM; two-tier Zstd compression (ingest at Zstd(3), cold Grandfather re-pack at Zstd(9/15) only during GC).
* **Graphics (Phase 7):** 2D framebuffer, `devdraw`, `#G` VirtIO-GPU Venus (`VIRTIO_GPU_F_VIRGL` bit 0, `BLOB` bit 3, `CONTEXT_INIT` bit 4, Capset 4; context ID packed in upper 24 bits of QID path), and `draw(3)` host-blit bridge.
* **Primary Native Toolchain (Phase 8):**
  1. **`rustc` + `rust-lld`:** Primary compiler and linker. `rustc`'s metadata loader uses whole-file reads (zero `mmap` dependency).
  2. **APEX (vendored):** Musl-based ANSI C runtime over Plan 9 syscalls for LLVM/C++ tooling.
  3. **`elf2aout`:** In-tree converter wrapping static ELF output into Plan 9 `a.out`.
  4. **Harvey C Compilers (`6c`/`6a`/`6l`) & Classic APE:** Retained strictly as a bootstrap/compatibility binary layer; **removed from Track P Rust porting scope**.