# Phase 5: Guide Phase 3 - Early SMP, Course Correction and Memory Pools - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-25
**Phase:** 5-Guide Phase 3 - Early SMP, Course Correction and Memory Pools
**Areas discussed:** Slab handle and FID mapping, Pools with no backing store, SMP shape and lock audit, Order, stops and review

---

## Slab handle and FID mapping

### Fid mapping

| Option | Description | Selected |
|--------|-------------|----------|
| Server-assigned only (Recommended) | The handle is the only name; no lookup by client fid number, no duplicate-fid check; the all-shard probe and `insert_lock` go away | ✓ |
| Per-connection fid map | Each connection maps client fid number to handle; "fid in use" detected per connection | |
| You decide | Research compares against RFC section 5 and Phase 6 needs | |

**User's choice:** Server-assigned only

### Client check

| Option | Description | Selected |
|--------|-------------|----------|
| Keep the client check (Recommended) | `resolve`, `clunk`, `mark_reclaimed` compare the connection's `client_id` with the entry's | ✓ |
| Follow the compendium exactly | Only epoch, shard and generation are checked | |

**User's choice:** Keep the client check

### Slab home

| Option | Description | Selected |
|--------|-------------|----------|
| ds/src/slab.rs, generic (Recommended) | Generic slab in `ds/` under forbid; `port/src/fid.rs` wraps it | ✓ |
| Embedded in port/src/fid.rs | As in the compendium section 4 listing | |

**User's choice:** ds/src/slab.rs, generic

### Non-zero rule

| Option | Description | Selected |
|--------|-------------|----------|
| Generation is never zero (Recommended) | Starts at 1, skips 0 on wrap; `from_wire` rejects `gen == 0` | ✓ |
| An always-set flags bit | Bit 0 of flags is "stateful", always 1 | |
| Both | Two independent guards | |

**User's choice:** Generation is never zero

### Shard lock

| Option | Description | Selected |
|--------|-------------|----------|
| New safe-Rust RwLock (Recommended) | Spinning reader-writer lock on atomics per shard | |
| Existing MCS Lock per shard | No new primitive; resolves on one shard take turns | |
| Keep resolve lock-free | Atomics read without a lock, as today | |
| You decide | Research measures the three | |

**User's choice:** Free text (a fourth design), reproduced verbatim:

> Put a per-slot seqlock + generation word (AtomicU64) directly inside each 64-byte slab slot and back the shard with a non-reallocating slot array.
>
> That gives you the exact fast path you have today—zero locks and one 64-byte cache-line fetch on FidTable::resolve—while guaranteeing a strictly consistent snapshot under concurrent clunk, alloc, and in-place update operations, all inside #![forbid(unsafe_code)]
>
> In the sharded slab, FidEntry only uses 42 bytes of live payload (the other 22 bytes were padding to reach 64 bytes). By reclaiming 8 of those padding bytes for a per-slot state: AtomicU64, the version check and the entire payload live in the exact same 64-byte CPU cache line:
>
> No reallocation lock: slots is a fixed Box<[AtomicSlabSlot]> per shard (or a fixed 64-entry spine of lazily initialized 1,024-slot chunks), so slots[slot_idx] never moves and readers never touch a lock.
>
> No RwLock on readers: Only alloc, clunk, and sweep_unreclaimed acquire a shard-local Mutex<AllocMeta> (protecting free_list, active_slots, and reclaimed_slots). resolve() never touches AllocMeta.
>
> Consistent snapshots: Separating gen: u16 (bumped only on clunk) from seq: u46 (bumped on every write) lets in-place updates (e.g., offset or lease_gen) trigger a clean reader retry without invalidating the client's wire StatefulHandle.
>
> 64-Byte Slot Layout & State Bit-Packing
> Bit 0      : WRITING  (1 = writer active, 0 = stable)
> Bit 1      : OCCUPIED (1 = live FID, 0 = free / clunked)
> Bits 2..17 : gen: u16 (1..=65535; bumped only on clunk; matched against StatefulHandle.gen)
> Bits 18..63: seq: u46 (monotonic write counter; bumped on alloc, update, and clunk)
> Total slot size: 8B (state) + 56B ([AtomicU64; 7]) = 64 bytes exact (1 cache line).
>
> Lock-Free Consistent-Snapshot Implementation (#![forbid(unsafe_code)])
> This uses the standard Boehm seqlock fence pairing over AtomicU64 words, so it has zero Rust data-race UB and is sound on weakly ordered aarch64 and riscv64 as well as x86_64:

```rust
#![forbid(unsafe_code)]

use core::sync::atomic::{fence, AtomicU64, Ordering};

const WRITING: u64 = 1 << 0;
const OCCUPIED: u64 = 1 << 1;
const GEN_SHIFT: u32 = 2;
const GEN_MASK: u64 = 0xFFFF << GEN_SHIFT;
const SEQ_SHIFT: u32 = 18;
const SEQ_INC: u64 = 1 << SEQ_SHIFT;

#[inline]
fn decode_gen(state: u64) -> u16 {
    ((state & GEN_MASK) >> GEN_SHIFT) as u16
}

#[repr(C, align(64))]
struct AtomicSlabSlot {
    state: AtomicU64,
    words: [AtomicU64; 7], // 56 bytes — packs the 42-byte FidEntry
}

impl AtomicSlabSlot {
    /// Lock-free, consistent-snapshot read (1 cache line on the uncontended path).
    #[inline]
    fn resolve(&self, expected_gen: u16) -> Option<FidEntry> {
        loop {
            let s1 = self.state.load(Ordering::Acquire);
            if (s1 & WRITING) != 0 {
                core::hint::spin_loop();
                continue;
            }
            if (s1 & OCCUPIED) == 0 || decode_gen(s1) != expected_gen {
                return None;
            }

            // Relaxed atomic loads: UB-free even if a writer races concurrently.
            let mut raw = [0u64; 7];
            for (dst, src) in raw.iter_mut().zip(self.words.iter()) {
                *dst = src.load(Ordering::Relaxed);
            }

            // Pairs with writer's fence(Release) so payload loads cannot be
            // reordered after the validation load on aarch64/riscv64.
            fence(Ordering::Acquire);
            let s2 = self.state.load(Ordering::Relaxed);

            if s1 == s2 {
                return Some(FidEntry::from_words(raw));
            }
            // A concurrent update or clunk raced with our read; retry.
        }
    }

    /// Writer helper (called under shard AllocMeta mutex for alloc/clunk,
    /// or after winning a CAS on WRITING for in-place updates).
    fn publish_alloc(&self, entry: FidEntry) -> u16 {
        let cur = self.state.load(Ordering::Relaxed);
        let gen = decode_gen(cur).max(1);
        let writing = (cur & !OCCUPIED) | WRITING;

        self.state.store(writing, Ordering::Relaxed);
        fence(Ordering::Release);

        for (dst, val) in self.words.iter().zip(entry.to_words().into_iter()) {
            dst.store(val, Ordering::Relaxed);
        }

        let next_seq = cur.wrapping_add(SEQ_INC) & !(GEN_MASK | WRITING | OCCUPIED);
        let published = next_seq | ((gen as u64) << GEN_SHIFT) | OCCUPIED;
        self.state.store(published, Ordering::Release);
        gen
    }

    /// Clunks a live slot, bumping both `seq` and `gen` to invalidate stale handles.
    fn publish_clunk(&self, expected_gen: u16) -> Option<FidEntry> {
        let cur = self.state.load(Ordering::Relaxed);
        if (cur & OCCUPIED) == 0 || decode_gen(cur) != expected_gen {
            return None;
        }

        self.state.store(cur | WRITING, Ordering::Relaxed);
        fence(Ordering::Release);

        let mut old = [0u64; 7];
        for (dst, src) in old.iter_mut().zip(self.words.iter()) {
            *dst = src.load(Ordering::Relaxed);
            src.store(0, Ordering::Relaxed); // Scrub slot payload on clunk
        }

        let next_gen = expected_gen.wrapping_add(1).max(1); // Skip 0
        let next_seq = cur.wrapping_add(SEQ_INC) & !(GEN_MASK | WRITING | OCCUPIED);
        let cleared = next_seq | ((next_gen as u64) << GEN_SHIFT); // OCCUPIED = 0, WRITING = 0
        self.state.store(cleared, Ordering::Release);

        Some(FidEntry::from_words(old))
    }
}
```

> How FidShard Wires It Together

```rust
#[repr(align(64))]
pub struct FidShard {
    shard_id: u8,
    server_epoch: u16,
    /// Immutable slice — readers index directly with zero locks.
    slots: Box<[AtomicSlabSlot]>,
    /// Writer-only metadata (free_list + active_slots + reclaimed_slots).
    meta: Mutex<AllocMeta>,
}

impl FidShard {
    #[inline]
    pub fn resolve(&self, h: StatefulHandle) -> Result<FidEntry, FidError> {
        if h.epoch != self.server_epoch || h.shard_id != self.shard_id {
            return Err(FidError::StaleEpoch);
        }
        let slot = self.slots.get(h.slot_idx as usize).ok_or(FidError::InvalidHandle)?;
        slot.resolve(h.gen).ok_or(FidError::InvalidHandle)
    }
}
```

> Two Subtle Edge Cases to Guard Against
> In-place update() vs. clunk() concurrency: If you add an in-place update(h, new_entry) method, either take self.meta.lock() (simplest, still zero impact on resolve()) or CAS WRITING from 0 -> 1 while verifying OCCUPIED and gen == h.gen so two writers cannot race on the same slot.
>
> Memory footprint if a shard rarely fills to 65,536: A flat 65,536-slot Box<[AtomicSlabSlot]> is 4 MB per shard. If you want small shards that still grow without moving existing slots, replace Box<[AtomicSlabSlot]> with a fixed [OnceLock<Box<[AtomicSlabSlot; 1024]>>; 64] (or AtomicPtr in ds/). Readers still do a lock-free chunks[idx >> 10].get()?.get(idx & 1023) with zero reallocation and zero RwLock.

**Notes (checked against the tree):** the tree's `FidEntry` has 49 live bytes, not 42 (it also carries `fid_key`, `epoch` and `volatility`); it still fits 56. `OnceLock` is `std`-only and neither `ds/` nor `port/` has a once cell; `AtomicPtr` needs `unsafe` to dereference. `ds/` has no mutex; the MCS `Lock` is in `port/`.

### Shard size

| Option | Description | Selected |
|--------|-------------|----------|
| Flat array, caller-sized (Recommended) | One `Box<[AtomicSlabSlot]>` per shard sized at construction | |
| Chunked spine, safe once cell | 64 lazy 1,024-slot chunks if a safe `no_std` once cell exists | |
| Chunked spine, audited unsafe | Chunks through `AtomicPtr` in a new audited unsafe module in `ds/` | |

**User's choice:** Free text, reproduced verbatim:

> Flat Now, Geometric Chunks When Needed
> For Phase 2–3 bring-up (immediate refactor): Start with a flat, configurable Box<[AtomicSlabSlot]> sized as a power of two at shard construction. It requires zero no_std OnceBox plumbing, stays 100% inside #![forbid(unsafe_code)], and gives you strictly 1 memory read on resolve():
>
> Kernel synthetic mounts (#c, #e, #p): 256 slots/shard (16 KB/shard) × 4 shards = 64 KB (1,024 stateful FIDs).
>
> Default workstation / QEMU q35 boot: 1,024 slots/shard (64 KB/shard) × 8 shards = 512 KB (8,192 stateful FIDs).
>
> Cluster / heavy file server: 4,096–16,384 slots/shard (256 KB–1 MB/shard) × 16–32 shards = 4–32 MB (65K–512K stateful FIDs).
>
> For unbounded growth without wasting RAM (Phase 4+): Switch the shard backing store to geometric power-of-two chunks (the same layout used by Rust's sharded-slab and boxcar):
>
> Chunk sizes: 64, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768 (11 chunks totaling all 65,536 u16 slots).
>
> Starting cost: Chunk 0 is 64 slots = 4 KB (exactly 1 physical page) per shard—even 16 shards only consume 64 KB at boot.
>
> Index math (1 CLZ instruction, zero division):

```rust
#[inline]
fn split_idx(slot_idx: u16) -> (usize, usize) {
    let x = slot_idx as usize;
    if x < 64 {
        (0, x)
    } else {
        let bit = (usize::BITS - 1 - x.leading_zeros()) as usize; // 6..=15
        (bit - 5, x - (1 << bit)) // chunk 1..=10, offset 0..(1 << bit)
    }
}
```

> no_std safety boundary: Because std::sync::OnceLock is unavailable in no_std, back the 11-entry spine with a tiny OnceBox<[AtomicSlabSlot]> (using AtomicPtr with Release/Acquire) housed in an audited module (or spin::Once if already in the workspace), keeping ds/src/slab.rs and port/src/fid.rs #![forbid(unsafe_code)].
>
> Two Companion Sizing Rules
> Shard count (num_shards): Size to (num_cpus * 2).next_power_of_two().clamp(4, 256) (e.g., 4 shards for Phase 3's 2-CPU SMP bring-up, 16–32 for multi-core servers). Route alloc() to the calling CPU's shard (cpu_id & shard_mask) to keep writer locks core-local, falling back round-robin only if a shard hits its cap.
>
> Roaring metadata cost: Because every shard's slot_idx space is u16 (0..65,535), both active_slots and reclaimed_slots fit inside a single RoaringBitmap container—4 bytes as a Run container when contiguous, and capped at 8 KB worst-case when maximally fragmented.

**Notes:** the flat array is this phase; the geometric spine is deferred. "Phase 2-3" and "Phase 4+" above are v3.0 guide numbers (roadmap Phases 5 and 6 onward).

### Rewording of WIRE-07 to WIRE-09

| Option | Description | Selected |
|--------|-------------|----------|
| Same three places as Phase 4 (Recommended) | RFC section 5 dated notes, PROJECT.md Key Decisions rows, CORR-03 live; compendium v3.0 section 4 dated note | ✓ |
| RFC and PROJECT.md only | The `docs/` compendium is not edited | |
| You decide | Planner chooses | |

**User's choice:** Same three places as Phase 4

---

## Pools with no backing store

### Eviction

| Option | Description | Selected |
|--------|-------------|----------|
| Free-frame pools only (Recommended) | Pools describe free and held frames; no mapped page is taken | |
| Drop re-readable pages | Also unmap unmodified file-backed pages under pressure | ✓ |
| RAM-backed store seam | Evicted pages copied to kernel RAM behind a store interface | |

**User's choice:** Drop re-readable pages

### Droppable pages

| Option | Description | Selected |
|--------|-------------|----------|
| Read-only text only (Recommended) | Text mapped read-only; only text droppable; no dirty tracking | |
| Text plus untouched data | Also initialised-data pages not yet written; needs dirty tracking | ✓ |
| You decide | Research sizes both | |

**User's choice:** Text plus untouched data

### x86_64 pager

| Option | Description | Selected |
|--------|-------------|----------|
| Re-read on fault only (Recommended) | Exec stays eager; fault handler re-reads a dropped page | |
| Full demand loading | x86_64 exec loads every image page at first touch, as aarch64 | ✓ |
| You decide | Planner picks the smaller sound change | |

**User's choice:** Full demand loading

### Trigger and proof

| Option | Description | Selected |
|--------|-------------|----------|
| Live always, forced in a gate (Recommended) | Kernel-context forced drops in a `-smp 2` gate | |
| Live always, small-memory gate | Boot with small RAM until real pressure | |
| Off by default, boot switch | Dropping only when a switch enables it | |

**User's choice:** Free text: "Live always, forced in a small-memory gate with a special workload designed to deterministically trigger pressure drops"

### Page cache

| Option | Description | Selected |
|--------|-------------|----------|
| TPHT page cache now (Recommended) | Image page cache on `FlattenedTphtShard`; DEBT-04 first | ✓ |
| Simple per-image table now | Plain array per image under a lock | |
| You decide | Planner picks | |

**User's choice:** TPHT page cache now

### Roaring SIMD

| Option | Description | Selected |
|--------|-------------|----------|
| Scalar now, SIMD later (Recommended) | Scalar behind the `setops.rs` seam | |
| SIMD now, both architectures | AVX2 and NEON kernels in the reserved `ds` module | ✓ |
| NEON only now | aarch64 NEON; x86_64 scalar | |

**User's choice:** SIMD now, both architectures

### x86_64 vector level

| Option | Description | Selected |
|--------|-------------|----------|
| SSE2 only (Recommended) | 128-bit, runs on `qemu64`, FXSAVE unchanged | |
| AVX2 with XSAVE, CPUID-chosen | Enable AVX where present, XSAVE there; FXSAVE elsewhere | ✓ |
| SSE2 now, AVX2 deferred | SSE2 kernels now | |

**User's choice:** AVX2 with XSAVE, CPUID-chosen

### Pinning

| Option | Description | Selected |
|--------|-------------|----------|
| Guard on every sleeping path (Recommended) | Pin guard with per-frame count; all sleeping `port/` paths audited | ✓ |
| Guard built, devmnt only | Other paths in a ledger | |
| You decide | Planner sets the cut | |

**User's choice:** Guard on every sleeping path

---

## SMP shape and lock audit

### Lock model

| Option | Description | Selected |
|--------|-------------|----------|
| Coarse lock first, then narrow (Recommended) | Big kernel lock first, then hot structures leave it | |
| Fine-grained from the start | No big lock; every structure locked or per-CPU before CPU 1 schedules | ✓ |
| Coarse lock only | Big lock is the whole answer for this phase | |

**User's choice:** Fine-grained from the start

### Run queue

| Option | Description | Selected |
|--------|-------------|----------|
| One shared queue (Recommended) | Single queue under one `IrqSpinLock` | |
| Per-CPU queues, no stealing | A process stays on its CPU | |
| Per-CPU queues with stealing | An idle CPU takes work from the other | ✓ |

**User's choice:** Per-CPU queues with stealing

### Gate CPU count

| Option | Description | Selected |
|--------|-------------|----------|
| Move all to -smp 2, keep one -smp 1 leg (Recommended) | `-smp 2` becomes the golden invariant | ✓ |
| Existing stay -smp 1, new at -smp 2 | The letter of SMP-09 | |
| Run every gate at both | Doubles gate time | |

**User's choice:** Move all to -smp 2, keep one -smp 1 leg

### Lead architecture

| Option | Description | Selected |
|--------|-------------|----------|
| aarch64 first, shared code in port/ (Recommended) | HVF gives two real cores; x86_64 follows | ✓ |
| x86_64 first, shared code in port/ | Golden invariant first; TCG interleaves CPUs here | |
| Both together | Each step lands on both | |

**User's choice:** aarch64 first, shared code in port/

### Stress gate form

| Option | Description | Selected |
|--------|-------------|----------|
| Kernel workers, boot switch (Recommended) | Kernel-context workers pinned per CPU | |
| User workload over 9P | User processes drive opens, reads, clunks through a mount, plus exec churn | ✓ |
| Both | Kernel workers and a user workload | |

**User's choice:** User workload over 9P

### Server end for the workload

| Option | Description | Selected |
|--------|-------------|----------|
| In-kernel loopback test server (Recommended) | Boot-switch server end on the real `FidTable`, loopback link, injected secret | ✓ |
| Host-side server over the network | `FidTable` runs on the host; does not meet SMP-08 | |
| Kernel workers for FidTable after all | User workload only for the page cache | |

**User's choice:** In-kernel loopback test server

### Lock ordering

| Option | Description | Selected |
|--------|-------------|----------|
| Ranked locks + watchdog (Recommended) | Rank table, runtime rank assertions in debug and gate builds, per-CPU watchdog | ✓ |
| Documented order + watchdog | No runtime rank check | |
| You decide | Planner picks | |

**User's choice:** Ranked locks + watchdog

---

## Order, stops and review

### Planning form

| Option | Description | Selected |
|--------|-------------|----------|
| One phase, three planned stages (Recommended) | Each stage planned after the previous is verified | |
| Split into 5, 5.1 and 5.2 | Three roadmap phases | |
| One phase, one planning pass | Everything planned at once | ✓ |

**User's choice:** One phase, one planning pass
**Notes:** asked after stating how far the earlier choices had grown the phase.

### port/src rule

| Option | Description | Selected |
|--------|-------------|----------|
| Waived, review at owner stops (Recommended) | Cumulative diff at each stop and before any push | ✓ |
| Waived except fid.rs and rfork.rs | Those two shown before commit | |
| Rule stands | Every plan stops to show its diff | |

**User's choice:** Waived, review at owner stops

### Owner stops

| Option | Description | Selected |
|--------|-------------|----------|
| Three stops (Recommended) | After course correction; after SMP; phase end | |
| Four stops | As above, plus one after the AVX/XSAVE change | ✓ |
| Two stops | After course correction; phase end | |

**User's choice:** Four stops

### x86_64 venue

| Option | Description | Selected |
|--------|-------------|----------|
| CI plus your KVM host at stops (Recommended) | CI per commit; KVM runs on the owner's host at stops 2, 3 and 4 | ✓ |
| CI only | Green CI is the whole check | |
| KVM in CI if available, else your host | Research checks `/dev/kvm` on runners | |

**User's choice:** CI plus your KVM host at stops

### Traceability

| Option | Description | Selected |
|--------|-------------|----------|
| Amend requirements and roadmap first (Recommended) | New and reworded rows before planning | ✓ |
| CONTEXT.md decisions only | Plans cite D-numbers | |
| You decide | Planner chooses per item | |

**User's choice:** Amend requirements and roadmap first

---

## Claude's Discretion

- The `ds/src/slab.rs` and `port/src/fid.rs` split (where the meta mutex lives).
- Whether the entry keeps `fid_key` and `epoch`.
- The non-AVX x86_64 set-kernel path (scalar required, SSE2 optional).
- Dirty-tracking mechanism per architecture, low-water mark, small-memory gate size, pressure workload form.
- Behaviour when a page-in cannot get a frame.
- How `boot-iso` becomes pure; CI wiring for `test-9p` and the xtask tests.
- Lock rank table contents; watchdog period.
- Source of the stress workload's user programs.

## Deferred Ideas

- Geometric chunk spine for slab shards with an audited `OnceBox` (owner design; guide Phase 4 or later).
- A backing store for dirty anonymous pages.
- More than two CPUs, the 8-way soak, the full arm64 HVF series, WINDOWS #49, #66, #70 (Phase 8).
- Serving `9P2000.aead` from the kernel in a default boot (Phase 6).
