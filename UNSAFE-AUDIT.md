# UNSAFE-AUDIT.md — r9 unsafe-surface classification (261103-p64)

The maximal unsafe-elimination contract's canonical inventory: every unsafe
block/fn/impl/extern in the tree, classified `eliminated | encapsulated |
floor`, with the floor justification and the 05-review-fix overlap flag.

**Baseline (r9 HEAD d8496d3, plan-measured):** ~221 unsafe blocks / 15 unsafe
fn / 6 unsafe impl / 6 unsafe extern tree-wide; 18 mutable statics; 25 asm!
sites; exactly 1 SAFETY comment.  aarch64+port carried ≈ 260 unsafe-bearing
lines (per-file: syscall.rs 34, proc.rs 34, port/allocator.rs 30, vminit.rs 29,
vm.rs 18, main.rs 14, util.rs 13, devmnt.rs 13, kmem.rs 11, fdt.rs 11,
trap.rs 10, pl011.rs 9, mcslock.rs 7, fault.rs 6, exec.rs 6, gic.rs 4,
io.rs 3, the rest ≤ 2).

**Counting rule (the honesty contract):** a *site* is one `unsafe {…}` block,
`unsafe fn`, `unsafe impl`, or `unsafe extern` token.  SAFETY comments sit
one-per-site on every block/impl the lint can see; the lint
(`clippy::undocumented_unsafe_blocks = "deny"`, workspace lints, opted into by
the four gated crates only) enforces this mechanically — never the honor
system.  Sites the lint cannot see are documented per-file below (test-only
blocks, unsafe fns, extern blocks).

**Scope line (explicit, not silent):** elimination covers the aarch64, port,
crypto and ds crates — the four gated crates (boot-test is aarch64-only;
`cargo test -p port` is port-only; `cargo test -p crypto` / `-p ds`, the
soft-backend `crypto` leg and the two `lib/*-unknown-none-elf.json` cross legs
cover the two new crates).  `crypto/` and `ds/` are host-tested and
cross-checked only: no kernel crate depends on them in Phase 3, so neither is
linked into a kernel yet (CONTEXT D-21).  x86_64 + riscv64 are NEVER compiled
by the gates: they are classified by per-file count only, get NO elimination
and NO lint opt-in.  Extending the lint and this audit to `x86_64` is Phase 5's
question, when `shadow.rs` and `cryptpage.rs` land (CONTEXT D-11); it is not
done here.

## Reconciliation (mechanical)

| Set | Sites |
|-----|-------|
| aarch64 unsafe blocks+impls (lint-visible) | 129 |
| port unsafe blocks+impls, non-test, production (lint-visible) | 23 |
| crypto unsafe blocks+impls (lint-visible) | 0 |
| ds unsafe blocks+impls, non-test, production (lint-visible) | 1 |
| **SAFETY annotations (comment immediately preceding a site)** | **153** |
| port test-only blocks (cfg(test) crate-root test module — the pin/stress arena scaffolding; not lint-gated, never shipped) | 10 |
| unsafe fns (documented by `# Safety` / fn-level docs, not lint-checked) | 16 |
| unsafe extern blocks (not lint-checked; items doc'd individually) | 6 |

129 + 23 + 1 + 0 = 153: the site count reconciles with the SAFETY-annotation
count exactly (measured by script: each unsafe block/impl immediately preceded by
a SAFETY comment).  The 10 test-only blocks are the moved pin/stress tests'
arena setups + raw write/read backs (`port/src/allocator_tests.rs`).

The `aarch64` figure of 129 above is kept exactly as it was measured; Phase 3
research found 7 undocumented blocks in `aarch64/` under `cargo xtask clippy
--arch aarch64 --config virtshell` (`platform/virt/rtc.rs:75`, `proc.rs:439`,
`470`, `1823`, `2753`, `textcopy.rs:158`, `172`), so that figure may be stale.
It was not re-derived here because it is outside Phase 3.

**Phase 3 delta (crypto/ and ds/ join, 2026-11-24):** two workspace crates join
the contract (CONTEXT D-09, D-10, D-11, D-12).  `ds/` is per-module forbidden
with `ds/src/tpht/` as its one unsafe-holding module; it adds **1** lint-visible
block (`FlattenedTphtShard::deref_tiny`'s unchecked arena read,
`ds/src/tpht/flattened.rs:440`), **0** unsafe impls, **0** unsafe fns, **0**
unsafe extern blocks and **0** test-only blocks.  `crypto/` is crate-level
`#![forbid(unsafe_code)]` and adds **0** sites of every kind.  SAFETY
annotations 152 → 153; ds lint-visible 0 → 1, crypto 0 → 0; the reconciliation
holds.  Neither crate is linked into a kernel in this phase, so the kernel-byte
evidence is unchanged (D-21).

## ds/ and crypto/ (Phase 3)

Both crates take `[lints] workspace = true`, so
`clippy::undocumented_unsafe_blocks = "deny"` applies to each (CONTEXT D-11);
`cargo xtask clippy` (the default form hosted CI runs) rejects an undocumented
`unsafe` block in either.  The probe that proves it is recorded in
`.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/closing-suite.md`
(`VERDICT clippy-rejects-undocumented-unsafe`).

### Module states (the regression locks)

| Module | State |
|---|---|
| `ds/src/tpht/` | **the named unsafe-holding module this phase** — 1 block (`flattened.rs:440`, `deref_tiny`); `tpht/mod.rs` and `tpht/chained.rs` hold none |
| `ds/src/tpht/feistel.rs` | unsafe-free, `#![forbid(unsafe_code)]` (the safe submodule inside `tpht`) |
| `ds` reserved Roaring SIMD module | **approved by D-09 and D-20, NO CODE** until Phase 5; it will be mounted at the crate root with `#[path]` because the forbid on `roaring` reaches submodules |
| `ds/src/roaring.rs` | unsafe-free, `#[forbid(unsafe_code)]` |
| `ds/src/setops.rs` | unsafe-free, `#[forbid(unsafe_code)]` (the D-20 kernel seam) |
| `ds/src/clientset.rs` | unsafe-free, `#[forbid(unsafe_code)]` |
| `crypto/` | crate-level `#![forbid(unsafe_code)]`, **0 sites** |

A third unsafe-holding module in `ds/` needs the owner's approval (D-09).  The
`crypto/` forbid covers r9's own code, not the pinned crates it wraps (D-10).

### Classification — `ds/src/tpht/`

| File + identifier | Kind | Why it is needed | Invariant |
|---|---|---|---|
| `ds/src/tpht/flattened.rs:440` `FlattenedTphtShard::deref_tiny` | block ×1 (`get_unchecked` on the arena) | DS-03 requires one arena slot read with no header read; the bound check is elided on that path.  The equivalent safe index (`&self.arena[slot]`) is what every other reader uses (`read_value`) | `slot_index` masks the slot with `self.slot_mask`, and `with_probe` allocates the arena with exactly `slot_mask + 1` slots (`HOME_SLOTS * num_blocks`).  Both quantities are fixed in the constructor and never mutated, so `slot < self.arena.len()` for every block index and every tiny pointer, including a pointer that runs past the arena end |

There is no `unsafe fn`, `unsafe impl`, `unsafe extern` block or test-only
`unsafe` block anywhere in `ds/`, and no `unsafe` site at all in `crypto/`.

### Third-party dependency record (D-10 discretion item)

`crypto/` wraps six pinned crates.io crates and `ds/` takes one dev-dependency
oracle; every one of them is compiled from upstream sources that contain
`unsafe` of their own.  **Upstream sites are not enumerated or audited here**:
the r9 contract governs r9's own code, and the pins, the committed `Cargo.lock`
and the provenance comments are the supply-chain record (CONTEXT D-04, D-10).
The `unsafe` counts below are raw `grep -ow unsafe` counts over each crate's
upstream `src/` directory (`03-RESEARCH.md` F10; the counts for the crates F10
does not list were taken the same way at audit time).  They include code for
backends r9 never selects — SIMD, assembly and C paths that the `pure` and
`no_*` features or the soft-float kernel specs switch off.

Direct dependencies of `crypto/` (all `default-features = false`, exact `=`
pins, one provenance comment each in `crypto/Cargo.toml`):

| Crate | Version | Features r9 enables | Contains upstream `unsafe`? | Raw `unsafe` tokens |
|---|---|---|---|---|
| `sha2` | `=0.11.1` | none (no default features) | yes — software and hardware SHA paths | 39 |
| `hmac` | `=0.13.0` | none | no | 0 |
| `hkdf` | `=0.13.0` | none | no | 0 |
| `chacha20` | `=0.10.2` | `cipher` | yes — NEON/AVX2/SSE2 backends | 66 |
| `blake3` | `=1.8.7` | `pure`, `no_sse2`, `no_sse41`, `no_avx2`, `no_avx512`, `no_neon` | yes — SIMD intrinsics and C/assembly backends | 238 |
| `zeroize` | `=1.9.2` | none | yes — the volatile-write primitive | 21 |

Transitive crates the six above bring into `Cargo.lock` (version only; r9 sets
no feature on any of them):

| Crate | Version | Contains upstream `unsafe`? | Raw `unsafe` tokens |
|---|---|---|---|
| `digest` | `0.11.3` | no | 0 |
| `cipher` | `0.5.2` | no | 0 |
| `inout` | `0.2.6` | yes | 31 |
| `crypto-common` | `0.2.2` | no | 0 |
| `block-buffer` | `0.12.1` | yes | 22 |
| `hybrid-array` | `0.4.15` | yes | 43 |
| `typenum` | `1.20.1` | no | 0 |
| `cpufeatures` | `0.3.1` | yes — CPUID / `mrs` feature detection | 11 |
| `cfg-if` | `1.0.5` | no | 0 |
| `arrayvec` | `0.7.8` | yes | 58 |
| `constant_time_eq` | `0.4.2` | yes | 54 |
| `ctutils` | `0.4.8` | yes | 1 |
| `cmov` | `0.5.4` | yes | 28 |
| `libc` | `0.2.191` | yes — pulled by `cpufeatures` on the hosts that use it | 810 |
| `cc` | `1.11.3` | yes — `blake3`'s build dependency | 36 |
| `shlex` | `2.0.1` | yes — `cc`'s dependency | 7 |
| `find-msvc-tools` | `0.1.15` | yes — `cc`'s dependency | 55 |

`roaring =0.11.5` is a **dev-dependency of `ds/` only**: it is a test oracle,
it never links into a kernel, and it never reaches a `cargo build` or a
bare-metal cross leg (CONTEXT D-19; `Cargo.lock` records it with no
dependencies at this feature set).  It is not part of the audited surface and
is listed here only so the record is complete.

**08-02 delta (the phase-08 waker bridge):** `port/src/devip/waker.rs` — the
smoltcp socket-waker → r9 wake bridge (D-05) — is the ONE named
unsafe-bearing devip module, mounted at the crate root as `devip_waker` via
`#[path]` because the `#[forbid(unsafe_code)]` on the `devip` module
propagates into submodules (the named exception would be uncompilable as
`devip::waker`).  It adds 2 lint-visible blocks (the RawWaker data-pointer
deref in the vtable's shared wake path; the `Waker::from_raw` construction)
and 4 unsafe fns (the RawWakerVTable's clone/wake/wake_by_ref/drop —
`# Safety`-doc'd; `RawWakerVTable::new` is a SAFE const fn on the pinned
nightly-2026-01-30 toolchain).  Port lint-visible 21 → 23, SAFETY 150 →
152, unsafe fns 12 → 16; the reconciliation holds.

**Wave-3 deltas:** `port/allocator.rs` is UNSAFE-FREE (forbidden) — the
QuickFit internals are the index/arena model (side-table headers as
Option<Idx> chains, arena-offset blocks, no raw-pointer free-list surgery);
the GlobalAlloc shim (`port/globalalloc.rs`) holds the remaining unsafe
(the trait surface, the arena-edge Block constructor, realloc + its copy,
the QuickFit Sync proof).  The 05 review-fix semantics are preserved as
index-model equivalents: unlink's head-preserving mid-list removal +
no-match-returns-unchanged (index-chain splices), the cycle tripwire (loud
panic — the new test splices a cycle via a safe cfg(test) hook),
align_prefix's clamp-not-assert, free_misc's emergency-header fallback
(reserve header slots), BumpAlloc's (size, align) order + overflow checks,
realloc's same-size fast path + copy-then-free order.  The carve-from-the-
freed-block fallback variant is gone (a freed misc block now frees WHOLE —
the audit's sanctioned index-model equivalent).  asm! consolidation: asm!
exists ONLY in the named floor modules — trap.rs (trap vector interface +
halt/idle/debug-brk primitives), swtch.rs (context switch), vm.rs
(MMU-register access, TLB/barrier, the D-09 cache maintenance + the ctr_el0
read), pre_mmu/{vminit,util}.rs (the pre-MMU sequences), main.rs (the boot
entry global_asm! only).  fault.rs/proc.rs/syscall.rs/exec.rs carry no asm!.

### port module states (the regression locks)

| Module | State |
|---|---|
| `allocator` | **UNSAFE-FREE, `#[forbid(unsafe_code)]`** (the index/arena model) |
| `globalalloc` | the named floor (the GlobalAlloc shim) — 8 blocks + 7 fns, all SAFETY/doc'd |
| `mcslock` | the named floor (Lock + BusyCell primitives + the MntCore proof) — 12 blocks |
| `devip_waker` (`devip/waker.rs`) | the named 08-02 bridge (the smoltcp waker RawWaker vtable) — 2 blocks + 4 fns, all SAFETY/doc'd; mounted via `#[path]` so the `devip` parent's forbid cannot reach it |
| every other port module | unsafe-free, `#[forbid(unsafe_code)]` (the devip subtree carries per-module forbids: `devip.rs` shell + `stack`/`phyadapt`) |

The `#[forbid(unsafe_code)]` attributes are the regression locks: a new
unsafe site in a forbidden module fails the build — the honesty rule is
mechanical (T-p64-04).

### Wave-2 deltas (kept for the wave history)

devmnt's 4 sites (2 accessor blocks + 2 impls) are GONE — the session core
rides mcslock's `BusyCell` primitive (the try-enter, never-spin sibling of
Lock: an MCS lock held across the blocking 9P receive would spin-deadlock an
IRQ-masked contender) with msize/version as frozen set-once plain fields;
mcslock gained BusyCell's blocks/impls + the MntCore Send proof (the floor
module holds the primitive AND its proof — the `MntIo: Send` supertrait
would have worked but breaks the unmodified host tests' Rc fid-table
sharing).  fdt's 3 sites are GONE — `from_usize`'s two-pass raw-pointer
dance moved to the boot callers (aarch64 main9/vminit, riscv64 main) as
safe `header_totalsize` + `new`; fdt is unsafe-free.

### Typed MMIO (wave-2a)

`aarch64/src/regblock.rs` is the typed register-block wrapper over a mapped
VirtRange: drivers access registers by offset through `read`/`write`/
`write_or`; the volatile ops live only in `io.rs`'s primitives.  Migrated:
virt pl011 (PL011_RANGE now publishes the RegBlock; the lock-free IRQ echo
path is byte-identical in behavior), gic, raspi3b mailbox (the MMIO register
half — the DMA request buffer's volatile ops stay floor), raspi3b uartmini +
uartpl011 (tree-wide: no driver touches io.rs directly anymore).  The
`virtnotes` DEL-echo profile is the IRQ-path behavior proof.

## Eliminated (wave 1 — these sites no longer exist)

| Site (file + identifier) | Was | Elimination |
|---|---|---|
| aarch64/proc.rs `static mut CURRENT_PROC` | static mut | `AtomicPtr<Proc>` — data-race-free by construction |
| aarch64/proc.rs `static mut CURKSTACK` / `CURFRAME` | static mut | `SyncUnsafeCell<u64>` (repr(transparent) — same symbol names + layout for trap.S; T-p64-05) |
| aarch64/proc.rs `static mut SCHED_CONTEXT` / `SCHED_SEEDED` | static mut | `SyncUnsafeCell` |
| aarch64/vm.rs `static mut USER_PAGETABLE` + `user_pagetable()` + `init_user_page_tables()` | static mut + 2 fns | DELETED (dead since the 04-05 per-process root tables — zero callers) |
| aarch64/allocator.rs `static mut HEAP` / `ALLOC` | static mut | `SyncUnsafeCell` statics + the const-init chain |
| aarch64/syscall.rs `user_slice` / `user_slice_mut` free fns + all 21 call-site blocks | raw slice ctors | usermem module (UserSlice/UserSliceMut fused constructors) |
| aarch64/syscall.rs `sargs`' slot-read block, `copy_user_str`'s byte-read loop, sys_noted's 3 sites, sys_fversion's 2, sys_exec's argv read, sys_seek's write | raw ptr IO | usermem (UserSlice::validated_only / UserPtr / UserString / write_u64) + `TrapFrame::restore_from_saved` |
| aarch64/syscall.rs + trap.rs `deliver_notes` + fault.rs ×2 + proc.rs ×9 current-proc derefs | `&mut *current_proc()` | `proc::current()` → `ProcGuard` (one guarded accessor; T-p64-01's twin for the proc pointer) |
| port/devmnt.rs 11 ad-hoc `&mut *self.core.get()` sites | UnsafeCell derefs | `Mnt::core_mut(&Busy)` / `core_ref()` — two accessors behind the busy-token discipline |
| port/fdt.rs 8 sites (4 byte-extractors, transmute, property_value_contains, inline_str, parse_token) | MaybeUninit dance | internal representation is now `&[u8]` — the DTB is always initialized memory |
| aarch64 `&'static mut` signatures: `root_page_table`, `kernel_pagetable`, `new_user_proc`, `allocate_virtpage`, `recursive_root_page_table`; port/devcons `Lock<Option<&'static mut dyn Uart>>` + the Uart factory | 'static-mut refs | guard newtypes (`RootTableGuard`, `ProcGuard`, `VirtPage4KGuard`), shared `&'static` refs |

## Remaining sites — classification (aarch64 + port)

(The two Phase 3 crates are classified in `## ds/ and crypto/ (Phase 3)` above.)

### `encapsulated` — unsafe lives only inside a named safe-ish wrapper

| File + identifier | Kind | Wrapper / invariant |
|---|---|---|
| aarch64/usermem.rs `UserSlice::as_slice`, `read_u64`; `UserSliceMut::as_slice_mut`, `as_slice`, `write_u64`; `UserPtr::read_u64`; `UserString::copy_from` (7 blocks) | block ×7 | THE single user-memory materialization point (T-p64-01): constructors fuse validate_user_range + fault::ensure_user_resident; the calling proc's TTBR0 is live in its syscall context.  **05-06 overlap: yes** (the cold-buffer class is now un-forgettable) |
| aarch64/trap.rs `TrapFrame::copy_to_saved` / `restore_from_saved` (2 blocks) | block ×2 | the frame↔280-byte-Ureg reinterpretation pair, homed (syscall.rs/trap.rs carry none) |
| aarch64/vm.rs `RootTableGuard::deref/deref_mut` (2 blocks) | block ×2 | TTBR-resolved root-table handle; single CPU, boot/IRQ-masked context |
| aarch64/pagealloc.rs `VirtPage4KGuard::deref/deref_mut` (2 blocks) | block ×2 | freshly mapped page's handle |
| aarch64/proc.rs `ProcGuard::deref/deref_mut` (2 blocks) | block ×2 | PROC_TABLE-resident proc handle (slots never freed this era) |
| aarch64/proc.rs `read_curframe`/`write_curframe`/`write_curkstack` (3 blocks) | block ×3 | SyncUnsafeCell IO on the trap.S ABI symbols — the module-header write discipline |
| aarch64/proc.rs `sched_context` (1 block) | block ×1 | seed-once cell discipline, IRQ-masked single CPU |
| aarch64/platform/virt/pl011.rs `Pl011Uart::new` + `init_rx` + `rx_irq_cons_buffer` ×2 + `cons_rx_drain` + `cons_rx_pending` + `cons_rx_fifo_pending` + `cons_rx_ring_pos` + `rx_irq_echo` (9 blocks) | block ×9 | PL011_RANGE (publish-once before unmask) + CONS_RX (single-writer-IRQ / single-reader-syscall ring) cell disciplines — the research-P8 lock-free IRQ path |
| aarch64/platform/virt/gic.rs `init`/`enable`/`ack`/`eoi` (4 blocks) | block ×4 | GIC write-once cell, lock-free IRQ reads (research P8) |
| aarch64/platform/{virt,raspi3b}/devcons.rs `init` (1 block each) | block ×2 | write-once UART cell at platform init |
| port/devmnt.rs — NONE (wave 2) | — | the session core rides mcslock::BusyCell; msize/version are frozen set-once plain fields.  **05-09 overlap: preserved** (the four-string stat + one-outstanding-RPC discipline are byte-identical semantics) |
| aarch64/main.rs `main9`'s DTB two-pass + initrd `from_raw_parts`; `exec_bootstrap`'s `&*proc.frame` (3 blocks) | block ×3 | vminit-live DTB mapping (the header_totalsize + new two-step, wave 2); the reserved+mapped initrd; the seeded frame |
| aarch64/vmdebug.rs `recursive_root_page_table` + `print_table_at_level` (2 blocks) | block ×2 | read-only recursive-mapping walk (print path) |

### `floor` — the provably irreducible kernel floor

| File + identifier | Kind | Justification |
|---|---|---|
| aarch64/trap.rs `init` (vbar_el1), `unmask_irq` (DAIFClr), `trap_unsafe` (the trap.S entry ABI), `trap`'s 2 wfi blocks; `unsafe extern trap_return` | block ×4 + extern ×1 | the trap-vector interface: asm entry/exit, vector-base install, IRQ mask, clean-halt parks.  trap.S byte-untouched (gate-diffed) |
| aarch64/swtch.rs `unsafe extern swtch` | extern ×1 | the context-switch ABI (swtch.S) |
| aarch64/vm.rs `PhysPage4K::clear`; `next_mut` ×2; `init_empty_root_page_table` (fn+block); `ttbr0_el1`/`ttbr1_el1`; `switch` (fn+block); `invalidate_all_tlb_entries` (fn+block); `write_entry`; `write_recursive_entry` | fn ×3, block ×9 | page-table machinery: TTBR/TLB/barrier asm, volatile hardware-visible entry writes, recursive-mapping resolution.  The MMU floor |
| aarch64/kmem.rs extern block + 10 link-sym fns | extern ×1, block ×10 | kernel.ld boundary symbols — only the zero-sized markers' addresses are read (never contents) |
| aarch64/pre_mmu/vminit.rs extern block + 24 blocks (12 link-syms, isb/tlbi asm, DTB parse, pre-MMU identity-map page-table construction, EarlyPageAllocator) | extern ×1, block ×24 | pre-MMU boot: identity-map derefs of freshly allocated pages, the architected MMU-enable sequence.  **05-review overlap: no** |
| aarch64/pre_mmu/util.rs `write_or_volatile`/`write_and_volatile` (fns+blocks) + `init_early_uart` ×3 + `init_early_uart_putc` ×2 | fn ×2, block ×7 | pre-driver early console: fixed-PA MMIO, identity-mapped pre-MMU |
| aarch64/io.rs `read_reg`/`write_reg`/`write_or_reg` | block ×3 | the MMIO-volatility primitives (drivers' only volatile access; wave 2's regblock rides these) |
| aarch64/platform/raspi3b/mailbox.rs `request` ×2 | block ×2 | the mailbox's mapped request page (MMIO volatility) |
| aarch64/main.rs `print_stacks` extern+2 blocks; `brk #0`; the idle `wfi`; `ctr_el0` read; `vm::switch` call; `trap_return` call | extern ×1, block ×7 | link-syms, the deliberate dev_panic trigger, the idle/halt asm, the cache-register read, the boot-time TTBR0 switch + first EL0 entry (the ABI boundary) |
| aarch64/proc.rs `interrupt_stack_top` extern+block; `free_user_mappings`/`new_user_proc`/`rfork_copy` ×3 map_to/copy blocks; `reschedule`/`pick_next` `vm::switch` calls; `sched`/`sched_from_moribund`/`sched_loop` ×3 swtch/trap_return/wfi blocks; `post_wait_record` parent deref | extern ×1, block ×11 | the process machinery's floor: link-syms, the KZERO+PA-alias table/page writes, the switch/resume ABI calls, the locked parent-proc post |
| aarch64/fault.rs `page_in_from_image`/`page_in_zeroed` fills, `flush_icache_user_range` asm, `kill_current_process` wfi | block ×4 | user-page writes through the live TTBR0 (the demand-pager); the D-09 cache-maintenance sequence; the halt park |
| aarch64/exec.rs 6 blocks (data/staging writes, seed-stack/argv construction, map_one_user_page, frame seed) | block ×6 | exec's KZERO+PA-alias page writes + the seed-frame ABI |
| aarch64/syscall.rs `exit_tail`'s wfi | block ×1 | the all-exited clean halt |
| aarch64/allocator.rs the arena Block construction in the SyncUnsafeCell init | block ×1 | the global allocator's arena — one-time static construction |
| port/allocator.rs — NONE (wave 3: `#[forbid(unsafe_code)]`) | — | the index/arena model — internals carry no raw-pointer free-list surgery; the pin tests pass with byte-verbatim bodies (pointer identity holds: base+offset materialization) |
| port/globalalloc.rs — the GlobalAlloc shim (wave-3 split) | fn ×7, block ×8 | THE named allocator floor: the trait's unsafe fn surface, `Block::new_from_raw_parts` (the arena edge), `QuickFit::realloc` + `copy_live_bytes`, `unsafe impl Sync for QuickFit`, the `unsafe impl Allocator for BumpAlloc`, with_allocator's swap trick |
| port/mcslock.rs — the sync-primitives floor: `lock`/`unlock`/`Lock::lock` ×2/`drop` + Send/Sync impls (7 sites), plus wave-2's BusyCell (`enter` guard's Deref/DerefMut blocks, its Send/Sync impls) + the `unsafe impl Send for devmnt::MntCore` proof (5 sites) | impl ×5, block ×7 | the MCS queue's raw node pointers + the try-enter cell are the synchronization protocol; the MntCore proof is the BusyCell discipline's other half |
| port/devip/waker.rs (mounted as `devip_waker`) — the 08-02 smoltcp waker bridge: the RawWaker vtable (clone/wake/wake_by_ref/drop fns + the shared wake path's slot deref block) + `Waker::from_raw` in `waker_for` | fn ×4, block ×2 | constructing a `core::task::Waker` is irreducibly unsafe (no safe constructor exists).  **Invariant: the wakee-pid slot is armed (written) before registration, read at wake, and never outlives the conv** (`&'static WakeSlot` makes it mechanical — slots live in conv-table/test statics).  The vtable wake records into the seam's pending-wake set only — never the wake hook directly (wake-after-drop, stack.rs's header).  Host pins (a)/(b) in `port/src/devip_tests.rs` exercise the wake path. |
| port/fdt.rs — NONE (wave 2) | — | `from_usize` removed; `header_totalsize` + `new` are fully safe (the raw-pointer two-pass lives at the aarch64/riscv64 boot callers) |

### Out of scope (never compiled by the gates — classified by count only)

| Crate | File | unsafe tokens |
|---|---|---|
| x86_64 | allocator.rs 1, cpu.rs 15, dat.rs 2, devcons.rs 1, main.rs 4, node0.rs 29, pio.rs 6, proc.rs 1, syscall.rs 3, trap.rs 9, uart16550.rs 1, vsvm.rs 9 | **81** |
| riscv64 | allocator.rs 1, main.rs 1, nezha/devcons.rs 1, virt/devcons.rs 1, runtime.rs 1, sbi.rs 2, uart16550.rs 4 | **11** (+ 1 `static mut` declaration in nezha/devcons.rs, an additional non-token site) |

Rationale: the gates (five aarch64 boot profiles + `cargo test -p port` +
virtshell build/clippy) never compile these crates; elimination work there
would be unverifiable — recorded, not touched.

## asm! consolidation note (wave 3 — landed)

asm! exists ONLY in the named floor modules (aarch64): trap.rs (the vector
interface: vbar install, DAIFClr, + the consolidated halt/idle/debug-brk
primitives `park_forever`/`idle_wait`/`debug_sync_trap`), swtch.rs (the
context-switch global_asm!), vm.rs (MMU-register reads ttbr0/ttbr1/ctr_el0,
the switch + TLB/barrier sequences, the D-09 cache maintenance),
pre_mmu/vminit.rs (isb + tlbi in the MMU-enable), main.rs (the boot entry's
global_asm!(l.S) only).  Everything formerly ad-hoc — fault.rs's cache
sequence + park, proc.rs's sched idle arm, syscall.rs's all-exited halt,
main.rs's brk/wfi/ctr — now calls those primitives.  trap.S/swtch.S/l.S
remain byte-untouched (the gate's diff check).

## Descopes

None at wave-1 completion.  (Any wave-2/3 descope records here as
"attempted, reverted, reason" — never landed broken.)
