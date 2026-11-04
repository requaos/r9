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
the two gated crates only) enforces this mechanically — never the honor
system.  Sites the lint cannot see are documented per-file below (test-only
blocks, unsafe fns, extern blocks).

**Scope line (explicit, not silent):** elimination covers the aarch64 + port
crates — the gated crates (boot-test is aarch64-only; `cargo test -p port` is
port-only).  x86_64 + riscv64 are NEVER compiled by the gates: they are
classified by per-file count only, get NO elimination and NO lint opt-in.

## Reconciliation (mechanical)

| Set | Sites |
|-----|-------|
| aarch64 unsafe blocks+impls (lint-visible) | 134 |
| port unsafe blocks+impls, non-test (lint-visible) | 36 |
| **SAFETY annotations (comment immediately preceding a site)** | **170** |
| port test-only blocks (cfg(test), not lint-gated, documented here) | 4 |
| unsafe fns (documented by `# Safety` / fn-level docs, not lint-checked) | 11 |
| unsafe extern blocks (not lint-checked; items doc'd individually) | 6 |

134 + 36 = 170: the site count reconciles with the SAFETY-annotation count
exactly (measured by script: each unsafe block/impl immediately preceded by
a SAFETY comment).  The 4 test-only blocks are the allocator pin tests'
arena constructions (`Block::new_from_raw_parts` over a Vec's buffer) —
host-test scaffolding, never shipped code.

**Wave-2 deltas:** devmnt's 4 sites (2 accessor blocks + 2 impls) are GONE —
the session core rides mcslock's new `BusyCell` primitive (the try-enter,
never-spin sibling of Lock: an MCS lock held across the blocking 9P receive
would spin-deadlock an IRQ-masked contender) with msize/version as frozen
set-once plain fields; mcslock gained BusyCell's 4 blocks/impls + the
MntCore Send proof (the floor module holds the primitive AND its proof —
the `MntIo: Send` supertrait would have worked but breaks the unmodified
host tests' Rc fid-table sharing).  fdt's 3 sites are GONE — `from_usize`'s
two-pass raw-pointer dance moved to the boot callers (aarch64 main9/vminit,
riscv64 main) as safe `header_totalsize` + `new`; fdt is unsafe-free.  port's
lib.rs now carries `#[forbid(unsafe_code)]` on EVERY module except the two
named floors (allocator, mcslock) — the regression lock is compile-time.

### Typed MMIO (wave-2a)

`aarch64/src/regblock.rs` is the typed register-block wrapper over a mapped
VirtRange: drivers access registers by offset through `read`/`write`/
`write_or`; the volatile ops live only in `io.rs`'s primitives.  Migrated:
virt pl011 (9 cell sites unchanged — PL011_RANGE now publishes the RegBlock;
the lock-free IRQ echo path is byte-identical in behavior), gic (4),
raspi3b mailbox (the MMIO register half — the DMA request buffer's volatile
ops stay floor), raspi3b uartmini + uartpl011 (tree-wide: no driver touches
io.rs directly anymore).  The `virtnotes` DEL-echo profile is the IRQ-path
behavior proof.

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
| port/allocator.rs — QuickFit's internals (25 block/impl sites + 6 unsafe fns; 4 of the blocks are test-only) | fn ×6, impl ×2, block ×25 | **wave-3 target** (the index/arena redesign): the intrusive free-list surgery is the allocator floor today; the `global` module's GlobalAlloc impl is THE named floor that survives the redesign |
| port/mcslock.rs — the sync-primitives floor: `lock`/`unlock`/`Lock::lock` ×2/`drop` + Send/Sync impls (7 sites), plus wave-2's BusyCell (`enter` guard's Deref/DerefMut blocks, its Send/Sync impls) + the `unsafe impl Send for devmnt::MntCore` proof (5 sites) | impl ×5, block ×7 | the MCS queue's raw node pointers + the try-enter cell are the synchronization protocol; the MntCore proof is the BusyCell discipline's other half |
| port/fdt.rs — NONE (wave 2) | — | `from_usize` removed; `header_totalsize` + `new` are fully safe (the raw-pointer two-pass lives at the aarch64/riscv64 boot callers) |

### Out of scope (never compiled by the gates — classified by count only)

| Crate | File | unsafe tokens |
|---|---|---|
| x86_64 | allocator.rs 1, cpu.rs 15, dat.rs 2, devcons.rs 1, main.rs 4, node0.rs 29, pio.rs 6, proc.rs 1, syscall.rs 3, trap.rs 9, uart16550.rs 1, vsvm.rs 9 | **81** |
| riscv64 | allocator.rs 1, main.rs 1, nezha/devcons.rs 1, virt/devcons.rs 1, runtime.rs 1, sbi.rs 2, uart16550.rs 4 | **11** (+ 1 `static mut` declaration in nezha/devcons.rs, an additional non-token site) |

Rationale: the gates (five aarch64 boot profiles + `cargo test -p port` +
virtshell build/clippy) never compile these crates; elimination work there
would be unverifiable — recorded, not touched.

## asm! consolidation note (wave-3's map)

asm! sites at wave-1 end (aarch64): trap.rs 8 (vector init, DAIFClr, 2 wfi
halt parks + macro-expanded arms), vm.rs 10 (TTBR reads/writes, TLB/barrier),
fault.rs 7 (the D-09 cache sequence + the park), proc.rs 2 (the idle-arm
DAIFClr/wfi/DAIFSet), main.rs 4 (brk, wfi, ctr_el0 + the global_asm!), swtch.rs
2 (global_asm), vminit.rs 2 (isb, tlbi), syscall.rs 1 (exit halt wfi).  Wave 3
consolidates everything outside the named floor modules (trap interface,
swtch, TLB/barrier, MMU-register access, cache maintenance) behind safe fns.

## Descopes

None at wave-1 completion.  (Any wave-2/3 descope records here as
"attempted, reverted, reason" — never landed broken.)
