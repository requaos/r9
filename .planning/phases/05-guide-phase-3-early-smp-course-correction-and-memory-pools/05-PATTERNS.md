# Phase 5: Guide Phase 3 - Early SMP, Course Correction and Memory Pools - Pattern Map

**Mapped:** 2026-11-25
**Files analyzed:** 38
**Analogs found:** 35 / 38

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `ds/src/slab.rs` | model / data-structure | lock-free concurrent CRUD | `ds/src/tpht/flattened.rs` + `ds/src/clientset.rs` | exact |
| `ds/src/setops_simd.rs` | utility / SIMD kernel | batch transform | `aarch64/src/fpu.rs` + `x86_64/src/fpu.rs` + `ds/src/setops.rs` | role-match |
| `ds/src/setops.rs` (mod) | utility / dispatch seam | batch transform | `ds/src/setops.rs` | exact |
| `ds/src/lib.rs` (mod) | config / module root | N/A | `ds/src/lib.rs` + `port/src/lib.rs` | exact |
| `ds/src/tpht/flattened.rs` (mod) | model / hash table | lock-free CRUD | `ds/src/tpht/flattened.rs` | exact |
| `port/src/fid.rs` (mod) | service / handle table | lock-free read + sharded CRUD | `port/src/fid.rs` | exact |
| `port/src/rfork.rs` (mod) | config / flag schema | transform | `port/src/rfork.rs` | exact |
| `port/src/mcslock.rs` (mod) | middleware / synchronization | lock / spin-poll | `port/src/mcslock.rs` | exact |
| `port/src/smp.rs` | service / SMP coordinator | event-driven / IPI | `port/src/mcslock.rs` + `x86_64/src/fpu.rs` | partial |
| `port/src/sched.rs` | service / run queues | concurrent queue CRUD | `port/src/fid.rs` + `port/src/mcslock.rs` | role-match |
| `port/src/palloc.rs` | service / frame pools | set-algebra CRUD + batch | `port/src/bitmapalloc.rs` + `ds/src/roaring.rs` | role-match |
| `port/src/superpage.rs` | model / atomic bitset | atomic CRUD | `ds/src/tpht/flattened.rs` (`TphtBucketHeader`) | exact |
| `port/src/pagecache.rs` | service / page cache | concurrent CRUD + eviction | `port/src/fid.rs` (TPHT wrapper) + `ds/src/tpht/flattened.rs` | exact |
| `port/src/aead_loop.rs` | service / loopback server | request-response | `port/tests/common/aead_link.rs` | exact |
| `port/src/lib.rs` (mod) | config / module root | N/A | `port/src/lib.rs` | exact |
| `port/src/devip/stack.rs` (mod) | service / wait queues | event-driven | `port/src/devip/stack.rs` + `port/src/mcslock.rs` | exact |
| `port/src/netif.rs` (mod) | service / RX queue | event-driven | `port/src/devip/stack.rs` + `port/src/mcslock.rs` | exact |
| `aarch64/src/smp.rs` | service / arch SMP bring-up | hardware I/O / IPI | `aarch64/src/platform/virt/gic/gicv3.rs` + `aarch64/src/fpu.rs` | role-match |
| `aarch64/src/l.S` (mod) | boot assembly | hardware bring-up | `aarch64/src/l.S` | exact |
| `aarch64/src/trap.S` (mod) | trap assembly | event-driven | `aarch64/src/trap.S` | exact |
| `aarch64/src/platform/virt/gic/gicv2.rs` (mod) | driver / interrupt controller | hardware I/O / IPI | `aarch64/src/platform/virt/gic/gicv2.rs` | exact |
| `aarch64/src/platform/virt/gic/gicv3.rs` (mod) | driver / interrupt controller | hardware I/O / IPI | `aarch64/src/platform/virt/gic/gicv3.rs` | exact |
| `aarch64/src/fault.rs` (mod) | controller / demand pager | trap request-response | `aarch64/src/fault.rs` | exact |
| `aarch64/src/vm.rs` (mod) | model / page tables | hardware I/O | `aarch64/src/vm.rs` | exact |
| `aarch64/src/usermem.rs` (mod) | middleware / user-slice guard | RAII pin guard | `aarch64/src/usermem.rs` | exact |
| `x86_64/src/smp.rs` | service / arch SMP bring-up | hardware I/O / IPI | `x86_64/src/platform/q35/lapic.rs` + `x86_64/src/dat.rs` | role-match |
| `x86_64/src/ap_trampoline.S` | boot assembly (16→32→64) | hardware bring-up | None (real-mode SIPI trampoline is new) | none |
| `x86_64/src/platform/q35/acpi.rs` (mod) | utility / ACPI parser | transform | `x86_64/src/platform/q35/acpi.rs` | exact |
| `x86_64/src/platform/q35/lapic.rs` (mod) | driver / interrupt controller | hardware I/O / IPI | `x86_64/src/platform/q35/lapic.rs` | exact |
| `x86_64/src/dat.rs` (mod) | model / per-CPU `Mach` | in-memory state | `x86_64/src/dat.rs` | exact |
| `x86_64/src/trap.rs` (mod) | controller / trap & IPI entry | event-driven | `x86_64/src/trap.rs` | exact |
| `x86_64/src/fpu.rs` (mod) | service / AVX & XSAVE state | hardware I/O | `aarch64/src/fpu.rs` + `x86_64/src/fpu.rs` | exact |
| `x86_64/src/fault.rs` | controller / demand pager | trap request-response | `aarch64/src/fault.rs` | exact |
| `x86_64/src/exec.rs` (mod) | service / image loader | file-I/O | `aarch64/src/exec.rs` + `x86_64/src/exec.rs` | exact |
| `x86_64/src/usermem.rs` (mod) | middleware / user-slice guard | RAII pin guard | `aarch64/src/usermem.rs` + `x86_64/src/usermem.rs` | exact |
| `xtask/src/main.rs` & `lintcontract.rs` & `rforkh.rs` & `gate.rs` (mod) | tooling / test & lint runner | batch | `xtask/src/main.rs` + `xtask/src/lintcontract.rs` | exact |
| `gates/aarch64/{smp-stress,mem-pressure}.expect` | test / boot gate | request-response | `gates/aarch64/smoke-r9.expect` | exact |
| `gates/x86_64/{smp-stress,mem-pressure}.expect` | test / boot gate | request-response | `gates/x86_64/smoke-r9.expect` | exact |

*(Note: Pure documentation/CI config edits — `flake.nix`, `.github/workflows/{gates,xtask}.yml`, `gates/x86_64/drawterm-gate.expect`, `UNSAFE-AUDIT.md`, `docs/threat-model.md`, `xtask/tests/threat_model.rs`, `04-VALIDATION.md`, `KERNEL-BYTES-GAPS.md` — modify existing files in-place using their own local conventions.)*

---

## Pattern Assignments

### `ds/src/slab.rs` (model / data-structure, lock-free concurrent CRUD)

**Analogs:** `ds/src/tpht/flattened.rs` (Boehm seqlock + 64-byte cache-line assert + `Probe` hook) and `ds/src/clientset.rs` (`#![forbid(unsafe_code)]`, `alloc::boxed::Box`, `RoaringBitmap`, `SplitMix64` host tests)

**Module safety & imports pattern** (`ds/src/lib.rs` lines 26-32, `ds/src/tpht/flattened.rs` lines 58-60):
```rust
// In ds/src/lib.rs:
#[forbid(unsafe_code)]
pub mod slab;

// In ds/src/slab.rs:
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering, fence};
use crate::roaring::RoaringBitmap;
```

**Cache-line struct & compile-time layout assertion pattern** (`ds/src/tpht/flattened.rs` lines 99-118):
```rust
#[repr(C, align(64))]
pub struct TphtBucketHeader {
    seq_lock: AtomicU32,
    // ...
}

const _: () = {
    assert!(core::mem::size_of::<TphtBucketHeader>() == 64);
    assert!(core::mem::align_of::<TphtBucketHeader>() == 64);
};
```
Copy this exact `const _: ()` size/align assertion pattern for `AtomicSlabSlot<7>` (`size_of == 64`, `align_of == 64`).

**Boehm seqlock fence-pairing & zero-cost `Probe` pattern** (`ds/src/tpht/flattened.rs` lines 8-18, 138-144):
```rust
// - writer: set WRITING bit (Relaxed); fence(Release); store words (Relaxed);
//   store published state (Release).
// - reader: load state (Acquire); spin if WRITING; check OCCUPIED & gen;
//   load words (Relaxed); fence(Acquire); reload state (Relaxed); retry if !=.
pub struct FlattenedTphtShard<const WORDS: usize, P: Probe = NoProbe> {
    headers: Box<[TphtBucketHeader]>,
    arena: Box<[[AtomicU64; WORDS]]>,
    probe: P,
    // ...
}
```

**Deterministic host-test RNG pattern (no external crates)** (`ds/src/clientset.rs` lines 196-226):
```rust
const SEEDS: [u64; 8] = [
    0x0000_0000_0000_0001, 0x5DEE_CE66_D1CE_4B1D,
    0x1234_5678_9ABC_DEF0, 0xDEAD_BEEF_CAFE_F00D,
    0x0F1E_2D3C_4B5A_6978, 0xA5A5_5A5A_1234_4321,
    0xFFFF_FFFF_0000_0000, 0x9E37_79B9_7F4A_7C15,
];

struct SplitMix64 { state: u64 }
impl SplitMix64 {
    fn new(seed: u64) -> Self { Self { state: seed } }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}
```

---

### `ds/src/setops_simd.rs` & `ds/src/setops.rs` (utility / SIMD kernel, batch transform)

**Analogs:** `ds/src/setops.rs` (exact function signatures & naive reference tests), `aarch64/src/fpu.rs` (`.arch_extension simd` inline asm + `SAFETY:` comments), `x86_64/src/fpu.rs` (x86 inline asm + `options(att_syntax, ...)`), `ds/src/lib.rs` (crate-root mount for reserved unsafe module)

**Crate-root unsafe-module mount pattern** (`ds/src/lib.rs` lines 22-30; mirrors `port/src/lib.rs` lines 49-50):
```rust
// Mounted at crate root (NOT under #[forbid(unsafe_code)] mod roaring/setops)
// because a forbid propagates to submodules.
mod setops_simd;
#[forbid(unsafe_code)]
mod setops;
```
Update `xtask/src/lintcontract.rs` lines 263-309 (`check_ds_token_and_forbid_rules`) to allow `setops_simd.rs` alongside `tpht/`.

**Seam signatures to preserve unchanged** (`ds/src/setops.rs` lines 17-65):
```rust
pub(crate) const BITMAP_WORDS: usize = 1024;

pub(crate) fn bitmap_and(a: &[u64; BITMAP_WORDS], b: &[u64; BITMAP_WORDS], out: &mut [u64; BITMAP_WORDS]) -> u32
pub(crate) fn bitmap_or(a: &[u64; BITMAP_WORDS], b: &[u64; BITMAP_WORDS], out: &mut [u64; BITMAP_WORDS]) -> u32
pub(crate) fn bitmap_andnot(a: &[u64; BITMAP_WORDS], b: &[u64; BITMAP_WORDS], out: &mut [u64; BITMAP_WORDS]) -> u32
pub(crate) fn array_intersect(a: &[u16], b: &[u16], out: &mut Vec<u16>)
```

**AArch64 inline NEON asm pattern (soft-float safe, avoids `#[target_feature]`)** (`aarch64/src/fpu.rs` lines 91-96, 223-230):
```rust
unsafe {
    core::arch::asm!(
        ".arch_extension simd\n",
        // NEON instructions: ld1, and/orr/bic, st1, cnt, addv/addlv
        dst = in(reg) dst,
        options(nostack, preserves_flags),
    );
}
```

**x86_64 inline AVX2 asm pattern (soft-float safe)** (`x86_64/src/fpu.rs` lines 50-54, 184-190):
```rust
unsafe {
    core::arch::asm!(
        // AVX2 instructions: vmovdqu, vpand/vpor/vpandn, vmovdqu, vzeroupper
        dst = in(reg) dst,
        options(nostack, preserves_flags),
    );
}
```

**Equivalence test pattern** (`ds/src/setops.rs` lines 127-188):
Reuse `naive_bitmap`, `SEEDS`, and `random_words` to pin exact word-for-word and cardinality identity between scalar and SIMD paths on every seed.

---

### `port/src/fid.rs` (service / handle table, lock-free read + sharded CRUD)

**Analog:** `port/src/fid.rs` (existing file — refactor internals onto `ds::slab` while preserving `StatelessCap`, `FidHandle`, `VolatilityClass`, `qid_encode`/`qid_decode`, `gpu_qid_path`, and `port::Result` error conventions)

**Wire codec & zero-rejection pattern** (`port/src/fid.rs` lines 160-184, 255-272):
```rust
pub fn to_wire(&self) -> [u8; STATEFUL_HANDLE_LEN] {
    let mut out = [0u8; STATEFUL_HANDLE_LEN];
    // D-10 wire order: slot_idx(2) gen(2) shard_id(1) flags(1) epoch(2), LE
    out[0..2].copy_from_slice(&self.slot_idx.to_le_bytes());
    out[2..4].copy_from_slice(&self.gen.to_le_bytes());
    out[4] = self.shard_id;
    out[5] = self.flags;
    out[6..8].copy_from_slice(&self.epoch.to_le_bytes());
    out
}

pub fn from_wire(bytes: &[u8; STATEFUL_HANDLE_LEN]) -> crate::Result<StatefulHandle> {
    let gen = u16::from_le_bytes([bytes[2], bytes[3]]);
    if gen == 0 {
        return Err("fid: StatefulHandle gen is zero (D-10: never all zero)");
    }
    // ...
}
```

**Safe field-by-field word packing pattern (`#![forbid(unsafe_code)]`, zero transmute)** (`port/src/fid.rs` lines 327-345, 399-416):
```rust
fn words_from_bytes(bytes: &[u8; 56]) -> [u64; 7] {
    let mut words = [0u64; 7];
    for (word, chunk) in words.iter_mut().zip(bytes.chunks_exact(8)) {
        *word = u64::from_le_bytes([
            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
        ]);
    }
    words
}
```
Adapt the existing 64-byte (`[u64; 8]`) encoder to 56 bytes (`[u64; 7]`), dropping `FID_LIVE` (replaced by slot `OCCUPIED` per D-11) while keeping `client_id` check (D-08) and `client_id != 0` guard.

---

### `port/src/mcslock.rs` & `port/src/smp.rs` & `port/src/sched.rs` (middleware & service, SMP synchronization & scheduling)

**Analogs:** `port/src/mcslock.rs` (MCS `Lock` / `LockNode` / `LockGuard` + `SAFETY:` conventions), `x86_64/src/fpu.rs` / `aarch64/src/fpu.rs` (`#[cfg(test)] thread_local!` per-CPU simulation pattern)

**MCS spin-wait & guard Drop pattern** (`port/src/mcslock.rs` lines 48-66, 118-155):
```rust
while node.locked.load(Ordering::Acquire) {
    crate::smp::poll_shootdown_if_pending(); // SMP-07 deadlock prevention
    hint::spin_loop();
}
```
```rust
pub struct LockGuard<'a, T: ?Sized + 'a> {
    lock: &'a UnsafeCell<MCSLock>,
    node: &'a LockNode,
    data: &'a mut T,
}
impl<T: ?Sized> Drop for LockGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: the guard's existence proves the lock is held by this context.
        unsafe { &mut *self.lock.get() }.unlock(self.node);
    }
}
```
`IrqSpinLock<T>` mirrors `Lock<T>` directly: saves & masks IRQs before acquiring `MCSLock`, records rank/watchdog in `#[cfg(any(debug_assertions, feature = "lockdebug"))]`, and pops rank + unlocks + restores IRQs in `Drop for IrqLockGuard`.

**Host-testable per-CPU abstraction pattern** (`aarch64/src/fpu.rs` lines 201-210; identical in `x86_64/src/fpu.rs` lines 164-173):
```rust
#[cfg(test)]
fn this_cpu() -> &'static CpuFp {
    std::thread_local! {
        static CPU: &'static CpuFp = Box::leak(Box::new(CpuFp::new()));
    }
    CPU.with(|c| *c)
}
```
Use this exact `#[cfg(test)] thread_local!` pattern in `port/src/smp.rs` so lock-rank stacks, watchdog slots, shootdown acks, and `port/src/sched.rs` work-stealing run queues run multi-threaded on the host under `cargo test -p port` with zero arch dependencies.

**Module registration pattern** (`port/src/lib.rs` lines 20-30, 95-118):
```rust
#[forbid(unsafe_code)]
pub mod smp;
#[forbid(unsafe_code)]
pub mod sched;
```
Keep `port/src/mcslock.rs` as one of the three named floor modules (D-02); `smp.rs` and `sched.rs` stay strictly `#[forbid(unsafe_code)]`.

---

### `port/src/devip/stack.rs` & `port/src/netif.rs` (service / wait queues, event-driven)

**Analogs:** `port/src/devip/stack.rs` (existing wait sets) + `port/src/mcslock.rs` (`IrqSpinLock`)

**Single-CPU `BusyCell` sites to convert to ranked `IrqSpinLock` / `Lock`** (`port/src/devip/stack.rs` lines 247-278):
```rust
// BEFORE (single-CPU only — panics/fails on 2-CPU contention):
static NET_WAITERS: BusyCell<WaitSet> = BusyCell::new(WaitSet::new());
static PENDING_WAKES: BusyCell<WaitSet> = BusyCell::new(WaitSet::new());

// AFTER (SMP-03 — ranked IrqSpinLock at Rank 90 NET_IRQ):
static NET_WAITERS: IrqSpinLock<WaitSet> = IrqSpinLock::new("net_waiters", LockRank::NetIrq, WaitSet::new());
static PENDING_WAKES: IrqSpinLock<WaitSet> = IrqSpinLock::new("pending_wakes", LockRank::NetIrq, WaitSet::new());
```
Apply the same replacement to `STACK` (`port/src/devip/stack.rs:523`, Rank 14) and `INGRESS` (`port/src/netif.rs:387`, Rank 90). Keep `BusyCell` only on `devmnt::Mnt` where `contention_wait()` retries across blocking RPCs.

---

### `port/src/palloc.rs`, `port/src/superpage.rs`, `port/src/pagecache.rs` (service & model, memory pools & page cache)

**Analogs:** `port/src/bitmapalloc.rs` (allocator API & `PageAllocError`), `ds/src/roaring.rs` (`RoaringBitmap::difference` / `union`), `ds/src/tpht/flattened.rs` (`FlattenedTphtShard` & 64-byte `const _:` asserts), `port/src/fid.rs` (TPHT-backed cache with `Lock` protection)

**Allocator imports & error type pattern** (`port/src/bitmapalloc.rs` lines 11-14, 101-106; `port/src/pagealloc.rs` lines 1-9):
```rust
use crate::{
    mem::{PhysAddr, PhysRange},
    pagealloc::PageAllocError,
};

pub fn free_unused_ranges(
    &mut self,
    available_mem: PhysRange,
    used_ranges: impl Iterator<Item = PhysRange>,
) -> Result<(), PageAllocError>
```
`port/src/palloc.rs` adopts `PhysAddr`, `PhysRange`, and `PageAllocError`, replacing the static byte-bitmap with `RoaringBitmap` pools (`clean`, `dirty`, `active`, `pinned`) indexed by `pfn = (pa >> 12) as u32` and physical address reconstruction `(((chunk as usize) << 16) | (offset as usize)) * 4096`.

**Exact 64-byte atomic bitset pattern for `port/src/superpage.rs`** (`ds/src/tpht/flattened.rs` lines 99-131):
```rust
#[repr(C, align(64))]
pub struct SuperpageResidency {
    bits: [AtomicU64; 8],
}
const _: () = {
    assert!(core::mem::size_of::<SuperpageResidency>() == 64);
    assert!(core::mem::align_of::<SuperpageResidency>() == 64);
};
```

**`palloc.hash` TPHT page cache pattern for `port/src/pagecache.rs`** (`ds/src/tpht/flattened.rs` lines 157-176):
```rust
// Built AFTER DEBT-04 verifies production bank sizing on FlattenedTphtShard:
let shard = FlattenedTphtShard::<2>::with_banks(num_blocks, banks_for_production)?;
// Key: ((image_id as u64) << 32) | (page_idx as u64)
// Value: [pa, meta_word]
```
All three modules (`palloc`, `superpage`, `pagecache`) are declared with `#[forbid(unsafe_code)]` in `port/src/lib.rs`.

---

### `port/src/aead_loop.rs` (service / loopback server, request-response)

**Analog:** `port/tests/common/aead_link.rs` (in-memory duplex `c2s`/`s2c` queues, `TestExchange`, `Handshake`, `Session`, `FidTable`, `devmnt::Mnt`)

**Imports & session-exchange pattern** (`port/tests/common/aead_link.rs` lines 23-35, 164-208):
```rust
use crypto::key::Key;
use crate::chan::msg::{self, Msg};
use crate::chan::transport::{
    ExchangeStep, Handshake, Role, Session, SessionExchange, TransportConfig,
};
use crate::chan::wire;
use crate::dat::Qid;
use crate::devmnt::{Dialect, Mnt, MntIo};
use crate::fid::{FidEntry, FidHandle, FidTable, StatefulHandle};
```
Replace host-only `Rc<RefCell<...>>` from `aead_link.rs` with `alloc::sync::Arc` + `crate::mcslock::Lock` (or `IrqSpinLock`), keeping `#[forbid(unsafe_code)]`. Gate startup strictly behind bootconf/cmdline `aeadloop=1` (SMP-13, D-28).

---

### `aarch64/src/smp.rs`, `aarch64/src/l.S`, `aarch64/src/trap.S`, `aarch64/src/platform/virt/gic/{gicv2,gicv3}.rs` (arch SMP bring-up & IPIs)

**Analogs:** `aarch64/src/l.S` (EL2→EL1 drop, MMU enable), `aarch64/src/trap.S` (register save/restore), `aarch64/src/platform/virt/gic/gicv2.rs` & `gicv3.rs` (MMIO `RegBlock`, GICR wake, `ICC_*_EL1` system registers), `aarch64/src/fpu.rs` (`MAXMACH` & `MPIDR_EL1` indexing)

**Per-CPU `TPIDR_EL1` migration in `aarch64/src/trap.S`** (lines 136-138, 191-193, 236-239):
```asm
// BEFORE (global single-CPU CURFRAME):
	mov	x9, sp
	ldr	x10, =CURFRAME
	str	x9, [x10]
// AFTER (offset 0 of PerCpu in TPIDR_EL1):
	mov	x9, sp
	mrs	x10, tpidr_el1
	str	x9, [x10, #0]
```
Apply the same `mrs x9, tpidr_el1` substitution in `trap_return` (line 191) and `trap_return_exec` (line 237). Enforce `const_assert!(offset_of!(PerCpu, cur_frame) == 0)` in `aarch64/src/smp.rs`.

**EL2→EL1 + FPU enable pattern for `secondary_start` in `aarch64/src/l.S`** (lines 74-108):
```asm
	mrs	x0, CurrentEL
	and	x0, x0, CURRENTEL_EL
	lsr	x0, x0, #2
	cmp	x0, #1
	beq	1f
	// Drop EL2 -> EL1 with HCR_EL2_RW + CPACR_EL1_FPEN, then load published
	// MAIR_EL1, TCR_EL1, TTBR0_EL1, TTBR1_EL1, enable SCTLR_EL1.(M|C|I),
	// set VBAR_EL1 = exception_vectors, TPIDR_EL1 = &PER_CPU[cpu_id].
```

**GICv3 redistributor wake & SGI pattern** (`aarch64/src/platform/virt/gic/gicv3.rs` lines 125-180):
```rust
// Scan GICR RD frames (stride 2 * 64 KiB) matching MPIDR_EL1 affinity:
let waker = rd.read(GICR_WAKER);
rd.write(GICR_WAKER, waker & !GICR_WAKER_PROCESSOR_SLEEP);
for _ in 0..1_000_000 {
    if rd.read(GICR_WAKER) & GICR_WAKER_CHILDREN_ASLEEP == 0 { break; }
}
```
Store `gicr_physrange` in `Gicv3` so CPU 1 wakes its own RD frame, enables PPI 30 + SGI 0/1 in its own `GICR_ISENABLER0`, configures `ICC_SRE_EL1`/`ICC_PMR_EL1`/`ICC_IGRPEN1_EL1`, and sends SGIs via `msr ICC_SGI1R_EL1`. In `Gicv2` (`gicv2.rs` lines 81-84, 105-115), CPU 1 writes banked `GICC_PMR`/`GICC_CTLR`/`GICD_ISENABLER0`, and `send_sgi` writes `GICD_SGIR` (`0xF00`).

---

### `aarch64/src/fault.rs`, `aarch64/src/vm.rs`, `aarch64/src/usermem.rs` (demand pager, RO text, dirty tracking, pinning)

**Analogs:** `aarch64/src/fault.rs`, `aarch64/src/vm.rs`, `aarch64/src/usermem.rs` (in-place evolution)

**RO page entry constructors in `aarch64/src/vm.rs`** (lines 169-190):
```rust
pub fn rw_user_text() -> Self {
    Entry(0)
        .with_access_permission(AccessPermission::AllRw)
        .with_shareable(Shareable::Inner)
        .with_accessed(true)
        .with_uxn(false)
        .with_pxn(true)
        .with_mair_index(Mair::Normal)
        .with_valid(true)
}
```
Add `Entry::ro_user_text()` (`AccessPermission::AllRo`, `uxn: false`, `pxn: true`) and `Entry::ro_user_data()` (`AccessPermission::AllRo`, `uxn: true`, `pxn: true`). Fill image frames through the kernel `KZERO + pa` alias before installing the RO PTE.

**Demand fault & write-protect dirty-promotion pattern** (`aarch64/src/fault.rs` lines 49-55, 91-116, 224-242):
- In `user_fault`: when `already_mapped && access == "write" && segments.classify(va) == Some(SegKind::Data)` and the page is clean (`AllRo`), upgrade PTE to `Entry::rw_user_data()`, execute `tlbi vae1is`, mark dirty, and remove from droppable set.
- In `ensure_user_resident(proc, start, end, writable)`: when `writable == true`, also promote any already-mapped clean `SegKind::Data` page to `Entry::rw_user_data()` so EL1 kernel writes through `UserSliceMut` never take a permission fault (Pitfall 2).

**RAII `PinGuard` in `aarch64/src/usermem.rs` & `x86_64/src/usermem.rs`** (`aarch64/src/usermem.rs` lines 33-46, 87-99; `x86_64/src/usermem.rs` lines 53-70, 116-132):
```rust
pub struct UserSlice {
    ptr: *const u8,
    len: usize,
    _pin: PinGuard, // inline [u32; 4] fast path + Option<Vec<u32>> overflow; Drop unpins each PFN
}
```
`UserSlice::new` and `UserSliceMut::new` pin every resolved PFN via `port::palloc::pin_frame(pfn)`; `validated_only` constructs an empty `PinGuard::empty()` (non-sleeping paths).

---

### `x86_64/src/smp.rs`, `x86_64/src/platform/q35/{acpi,lapic}.rs`, `x86_64/src/dat.rs`, `x86_64/src/trap.rs` (x86_64 SMP bring-up, `Mach` GS base, IPIs)

**Analogs:** `x86_64/src/platform/q35/acpi.rs` (bounds-checked MADT walker), `x86_64/src/platform/q35/lapic.rs` (dual xAPIC/x2APIC register driver), `x86_64/src/dat.rs` (`Mach` layout with `me` at `%gs:0`), `x86_64/src/trap.rs` (`intrspecial` GS check & IRQ vector dispatch)

**MADT entry parsing pattern** (`x86_64/src/platform/q35/acpi.rs` lines 284-325):
```rust
match ty {
    0 => { /* Processor Local APIC: elen >= 8, acpi_id = bytes[off+2], apic_id = bytes[off+3], flags = le_u32(off+4) */ }
    1 => { /* existing IOAPIC */ }
    2 => { /* existing IntSrcOverride */ }
    9 => { /* Processor Local x2APIC: elen >= 16, x2apic_id = le_u32(off+4), flags = le_u32(off+8) */ }
    _ => {}
}
```

**Dual xAPIC / x2APIC ICR IPI pattern** (`x86_64/src/platform/q35/lapic.rs` lines 162-200):
- xAPIC: write `ICR_HI (0x310) = (apic_id << 24)` then `ICR_LO (0x300) = delivery_word`; poll delivery status bit 12.
- x2APIC: execute `mfence; lfence` BEFORE `wrmsr(0x830, ((apic_id as u64) << 32) | (delivery_word as u64))` (SDM Vol 3A §10.12.9 — `WRMSR` to `IA32_X2APIC_ICR` is not serializing).

**Per-CPU `Mach` access via `%gs:0` & canonical `intrspecial` check** (`x86_64/src/dat.rs` line 367; `x86_64/src/trap.rs` lines 287-297):
```asm
// x86_64/src/trap.rs intrspecial — replace hardcoded `cmpq $KMACH, %rax`
// with canonical kernel-half sign test (every kernel Mach VA has bit 63 = 1):
        movl ${MSR_GS_BASE}, %ecx;
        rdmsr;
        shlq $32, %rdx;
        orq %rdx, %rax;
        testq %rax, %rax;
        popq %rdx;
        popq %rcx;
        popq %rax;
        js 1f;
        swapgs;
1:
```
Replace hardcoded `*(dat::KMACH as *const Mach)` in `x86_64/src/fpu.rs:161` and `proc.rs` with `mach_ptr()` (`mov %gs:0, %rax`).

---

### `x86_64/src/fpu.rs` (service / AVX & XSAVE state, hardware I/O)

**Analogs:** `x86_64/src/fpu.rs` (existing FXSAVE driver + `CpuFp` + `with_simd()`) and `aarch64/src/fpu.rs` (reference contract per D-06)

**Layout & `FpState` upgrade pattern** (`x86_64/src/fpu.rs` lines 65-90, 180-211):
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C, align(64))] // 64-byte alignment required by XSAVE64/XRSTOR64 (also satisfies FXSAVE64's 16-byte req)
pub struct FpState {
    pub area: [u8; 832], // 512B legacy + 64B XSAVE header + 256B YMM_Hi128 (XCR0 = 0x7)
}
const _: () = assert!(core::mem::size_of::<FpState>() == 832);
const _: () = assert!(core::mem::align_of::<FpState>() == 64);
```
Dispatch `save_raw`/`restore_raw` on the boot-detected `FpFormat` (`XSaveOpt`, `XSave`, `FxSave`) set by CPUID (`ECX[26] && ECX[28]`), emitting one deterministic boot line (`fpu: format=xsaveopt avx2=on` / `format=fxsave avx2=off`).

---

### `x86_64/src/fault.rs` & `x86_64/src/exec.rs` (controller / demand pager, trap request-response)

**Analogs:** `aarch64/src/fault.rs` (exact structural template for `user_fault`, `page_in_from_image`, `page_in_zeroed`, `ensure_user_resident`), `x86_64/src/trap.rs` lines 826-850 (ring-3 `#PF` entry), `x86_64/src/exec.rs` lines 269-290 (eager mapping removal)

**`x86_64/src/trap.rs` ring-3 `#PF` routing pattern** (lines 826-850):
```rust
#[cfg(dev_exec)]
{
    if trap_frame.cs & 3 == 3 {
        let cr2 = /* mov %cr2, %rax */;
        if vector == 14 {
            crate::fault::user_fault(trap_frame, cr2);
            return 0;
        }
        // Other ring-3 traps: print + exit_tail(&mut p, b"sys: trap")
    }
}
```

**`x86_64/src/exec.rs` demand-loading change** (lines 273-280):
Remove `map_text_eager(proc, &source, aout)?;` and `map_data_bss_eager(proc, &source, aout)?;`, keeping `proc.text_source = Some(source);` and `seed_user_pages(proc, args)` — matching `aarch64/src/exec.rs`.

**`x86_64/src/fault.rs` structure** — mirror `aarch64/src/fault.rs` lines 36-250:
- Decode `#PF` error code (`trap_frame.err`: bit 0 `P`, bit 1 `W/R`, bit 2 `U/S`, bit 4 `I/D`).
- Reject `cr2 >= KZERO` -> `unresolvable`.
- Look up `segments.classify(va)`:
  - `SegKind::Text`: look up `port::pagecache` (`palloc.hash`), map read-only (`PTE_P | PTE_U`, no `PTE_W`, no `PTE_NX`).
  - `SegKind::Data`: fill via `phys_ptr_mut(pa)`, map `PTE_P | PTE_U | PTE_W | PTE_NX` with HW `PTE_D = 0`, register in droppable list.
  - `SegKind::Bss | SegKind::Tos | SegKind::Stack`: allocate clean frame (evict-and-retry on `OutOfSpace`), map `PTE_P | PTE_U | PTE_W | PTE_NX`.
- Export `ensure_user_resident` and `ensure_stack_resident` matching `aarch64/src/fault.rs` lines 201-242.

---

### `gates/aarch64/{smp-stress,mem-pressure}.expect` & `gates/x86_64/{smp-stress,mem-pressure}.expect` (test / boot gate, request-response)

**Analogs:** `gates/aarch64/smoke-r9.expect` (lines 61-120) and `gates/x86_64/smoke-r9.expect` (lines 105-120)

**Expect gate boilerplate pattern** (`gates/aarch64/smoke-r9.expect` lines 61-120):
```tcl
set timeout 600
match_max 65536
# Required env check -> exit 2 on missing env
foreach v {R9_KERNEL R9_DTB R9_INITRD R9_SMOKE_LOG} { ... }
set run_tag [expr {[info exists env(R9_RUN_TAG)] ? $env(R9_RUN_TAG) : "r9gate-[pid]-[clock seconds]"}]
exec rm -f $env(R9_SMOKE_LOG)
spawn $QEMU -M $R9_MACHINE -cpu $R9_CPU -smp 2 -m $R9_MEM ... -name $run_tag
set spawn_pid [exp_pid]
log_file $env(R9_SMOKE_LOG)
proc cleanup {} {
    global spawn_pid
    catch { exec kill -KILL -- -$spawn_pid 2>/dev/null }
    catch { exec kill -KILL $spawn_pid 2>/dev/null }
}
```
Note: Keep `log_user` ON (expect 5.45 writes nothing to `log_file` if `log_user 0`), and ALWAYS declare `global env` inside any proc touching `env(...)` (the CORR-01 root cause in `drawterm-gate.expect:319`).

---

## Shared Patterns

### 1. Crate-Root Per-Module `#[forbid(unsafe_code)]` & Lint Contract
**Source:** `ds/src/lib.rs` lines 13-32; `port/src/lib.rs` lines 10-50; `xtask/src/lintcontract.rs` lines 261-309
**Apply to:** Every new module in `ds/` and `port/` (`ds/src/slab.rs`, `port/src/{smp,sched,palloc,superpage,pagecache,aead_loop}.rs`), plus the single reserved unsafe SIMD module `ds/src/setops_simd.rs`
```rust
// Every non-floor module carries #[forbid(unsafe_code)] on the line above `pub mod ...;`.
// Any reserved unsafe-holding module (ds/src/setops_simd.rs) MUST be mounted at crate root
// (never inside a forbidden parent module) and allow-listed in xtask/src/lintcontract.rs.
```

### 2. Documented `// SAFETY:` & `UNSAFE-AUDIT.md` Lockstep
**Source:** `port/src/mcslock.rs` lines 54-58, 92-95, 105-111; `aarch64/src/fpu.rs` lines 214-230; `x86_64/Cargo.toml` (`[lints] workspace = true`)
**Apply to:** Every `unsafe` block in `ds/src/setops_simd.rs`, `port/src/mcslock.rs`, `aarch64/src/`, and `x86_64/src/`
```rust
// Every `unsafe` block carries an immediate `// SAFETY: ...` comment (enforced by
// workspace clippy::undocumented_unsafe_blocks = "deny") AND updates UNSAFE-AUDIT.md
// in the same commit (guide rule 3, D-03).
// Never justify unsafe with "single CPU, IRQ-masked" once SMP lands (SMP-03).
```

### 3. Error Propagation (`port::Result<T>` & Named `&'static str`)
**Source:** `port/src/lib.rs` line 165; `port/src/fid.rs` lines 172-183; `port/src/pagealloc.rs` lines 1-9
**Apply to:** All `port/` services (`fid`, `smp`, `sched`, `palloc`, `pagecache`, `aead_loop`) and arch `usermem` / `exec` / `fault`
```rust
pub type Result<T> = core::result::Result<T, &'static str>;
// Return descriptive, named &'static str errors (or typed enums like PageAllocError / ParseError).
// Never silently ignore errors or panic on user-reachable invalid inputs.
```

### 4. Kernel SIMD Guard & No-FP Kernel Rule (D-06)
**Source:** `aarch64/src/fpu.rs` lines 31-55; `x86_64/src/fpu.rs` lines 31-55
**Apply to:** `ds/src/setops.rs`, `ds/src/setops_simd.rs`, `x86_64/src/fpu.rs`, `aarch64/src/fpu.rs`
```rust
// Kernels compile with soft-float (-mmx,-sse / -neon).
// Never use #[target_feature(enable = ...)] for kernel SIMD.
// All vector instructions live in inline asm executed strictly inside fpu::with_simd()
// with IRQs masked, never sleeping or switching inside the guard.
```

### 5. `xtask` Test Registration & Lint Gates
**Source:** `xtask/src/main.rs` lines 3578-3684
**Apply to:** `DEBT-01`, `CORR-04`, `CORR-05`
```rust
// TestStep::run runs static contracts first (rforkh::check, lintcontract::check),
// then pushes each cargo test leg onto all_cmd_args (port, crypto+ds, crypto --doc,
// crypto soft-backend, xtask, arch host legs, plus test-9p).
```

---

## No Analog Found

Files with no close match in the codebase (planner should use `05-RESEARCH.md` patterns directly):

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `x86_64/src/ap_trampoline.S` | boot assembly (16→32→64) | hardware bring-up | Existing `x86_64/src/l.S` starts in 32-bit protected mode from Multiboot 2; a 16-bit real-mode SIPI entry copied to PA `0x8000` with an `0x8F00` mailbox is new in Phase 5 (see `05-RESEARCH.md` Pattern 3.5). |
| `port/src/smp.rs` (watchdog & rank stack parts) | service / diagnostics | event-driven | No runtime lock-rank assertion stack or timer-driven per-CPU progress watchdog exists yet in `port/` (see `05-RESEARCH.md` Pattern 3.2; use `thread_local!` host-test pattern from `fpu.rs`). |
| `x86_64/src/smp.rs` (INIT-SIPI-SIPI sequence) | service / arch SMP bring-up | hardware I/O | Existing `lapic.rs` and `acpi.rs` cover BSP init only; the INIT → 10 ms → SIPI → 200 µs → AP online handshake is new (see `05-RESEARCH.md` Pattern 3.5). |

## Metadata

**Analog search scope:** `ds/src/`, `port/src/`, `port/tests/`, `aarch64/src/`, `x86_64/src/`, `xtask/src/`, `gates/`, `flake.nix`, `.github/workflows/`
**Files scanned:** 45 tracked source files verified via `git ls-files`
**Pattern extraction date:** 2026-11-25
