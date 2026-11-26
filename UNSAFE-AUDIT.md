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
the five gated crates — aarch64, x86_64, port, crypto, ds) enforces this
mechanically — never the honor system.  Sites the lint cannot see are
documented per-file below (test-only blocks, unsafe fns, extern blocks, and
code behind cfgs no gate config compiles).

**Scope line (explicit, not silent):** elimination covers the aarch64, x86_64,
port, crypto and ds crates — the five gated crates, each opted into the
workspace lints via `[lints] workspace = true` in its own manifest (x86_64
joined at Phase 5's CORR-05, 2026-11-26, once its last undocumented unsafe
block carried a concrete SAFETY comment).  `crypto/` and `ds/` are host-tested and
cross-checked only: no kernel crate depends on them in Phase 3, so neither is
linked into a kernel yet (CONTEXT D-21).  **Phase 4 update (2026-11-24):** that
Phase 3 statement is kept as history; `port` now depends on `crypto/` and
`ds/` (CONTEXT D-22), so both crates' unsafe code is linked into the kernel
images.  riscv64 is NEVER compiled
by the gates: it is classified by per-file count only, gets NO elimination
and NO lint opt-in (the Phase 7 sync is its audit horizon).

**The uniform policy (CORR-05, D-02, D-03):** `crypto/` is crate-level
`#![forbid(unsafe_code)]` over r9's own code (the pinned crates it wraps are
the third-party record below).  `ds/` is per-module forbidden: exactly two
places may hold `unsafe` — `ds/src/tpht/` and one reserved Roaring SIMD
module (no code until MEM-17); a third needs owner approval (Phase 3 D-09).
`port/` is per-module forbidden with the three named floor modules
`globalalloc`, `mcslock` and `devip_waker`; no fourth without owner approval
(IG-D07 amended, D-02).  The workspace lint `clippy::undocumented_unsafe_blocks
= deny` applies in all five opted-in crates, so every unsafe block in
`ds/src/tpht/`, `aarch64/src` and `x86_64/src` (and everywhere else in those
crates) must carry a concrete `// SAFETY:` comment — enforced mechanically on
every config a clippy leg compiles, and static-scanned tree-wide below.

## Reconciliation (mechanical)

| Set | Sites |
|-----|-------|
| aarch64 unsafe blocks+impls (lint-visible, non-test production) | 191 |
| x86_64 unsafe blocks+impls (lint-visible, non-test production) | 156 |
| port unsafe blocks+impls, non-test, production (lint-visible) | 32 |
| crypto unsafe blocks+impls (lint-visible) | 0 |
| ds unsafe blocks+impls, non-test, production (lint-visible) | 1 |
| **SAFETY annotations (comment immediately preceding a site)** | **397** |
| aarch64 test-only blocks (cfg(test) modules — not lint-gated, never shipped) | 6 |
| x86_64 test-only blocks (cfg(test) modules — not lint-gated, never shipped) | 8 |
| port test-only blocks (cfg(test) crate-root test module — the pin/stress arena scaffolding; not lint-gated, never shipped) | 11 |
| unsafe fns (documented by `# Safety` / fn-level docs, not lint-checked) — aarch64 9, x86_64 30, port 11 | 50 |
| unsafe extern blocks (not lint-checked; items doc'd individually) — aarch64 7, x86_64 2 | 9 |

Re-measured 2026-11-26 (CORR-05) by a comment-stripping static scan over every
tracked `.rs` file: production lint-visible sites 186 + 158 + 22 + 1 + 0 = 367,
and zero of them lacks a SAFETY comment — the scan found no undocumented
production site in any opted-in crate, and the gate configs prove the lint side
mechanically (`cargo xtask clippy --arch aarch64 --config virtshell` and
`--arch x86-64 --config q35cpu` both exit 0 under the workspace
`undocumented_unsafe_blocks = deny`).  The annotation total (384) exceeds the
production-site total because cfg(test) modules and a few doubly-commented
sites carry SAFETY comments too.  Code behind cfgs no gate config compiles (the
dev_ether arms of both `pci.rs` drivers, the churn/cap/net gate files) inherits
the same deny lint the moment a config builds it.

The aarch64 row replaces the stale 129, measured at Phase 3 against the
pre-Phase-4 tree and already flagged here as suspect.  The growth is real code
that post-dates that measurement: `platform/virt/pci.rs`,
`platform/virt/gic/gicv3.rs`, `fpu.rs`, `textcopy.rs`, `churngate.rs` and the
proc.rs rfork/brk machinery.  The 7 undocumented aarch64 blocks the Phase 3
research found (`platform/virt/rtc.rs:75`, `proc.rs:439`, `470`, `1823`,
`2753`, `textcopy.rs:158`, `172`) were documented in 05-03; virtshell clippy is
green.  The x86_64 row is new (CORR-05): the crate was classified by count only
until this phase, and the legacy table's entries for files deleted with the
old x86_64 generation are gone.

**Phase 3 delta (crypto/ and ds/ join, 2026-11-24):** two workspace crates join
the contract (CONTEXT D-09, D-10, D-11, D-12).  `ds/` is per-module forbidden
with `ds/src/tpht/` as its one unsafe-holding module; it adds **1** lint-visible
block (`FlattenedTphtShard::deref_tiny`'s unchecked arena read,
`ds/src/tpht/flattened.rs:440`; re-measured `ds/src/tpht/flattened.rs:473` on
2026-11-24), **0** unsafe impls, **0** unsafe fns, **0**
unsafe extern blocks and **0** test-only blocks.  `crypto/` is crate-level
`#![forbid(unsafe_code)]` and adds **0** sites of every kind.  SAFETY
annotations 152 → 153; ds lint-visible 0 → 1, crypto 0 → 0; the reconciliation
holds.  Neither crate is linked into a kernel in this phase, so the kernel-byte
evidence is unchanged (D-21).

**Phase 4 delta (port links crypto/ and ds/, 2026-11-24):** `port` gains its
dependency on the two crates and the engine lands in the guide's sites
(CONTEXT D-22, D-25).  `port::chan` (with its submodules) and `port::fid` join
`port` as `#[forbid(unsafe_code)]` modules, so nothing in the new wire code can
hold unsafe; no new unsafe-holding module exists, so no IG-D07 approval was
needed.  Re-measured at this commit with the Phase 3 method (lint-visible
unsafe blocks + impls, non-test production): `port` **22** (14 blocks + 8
impls; the module breakdown is `globalalloc` 8, `mcslock` 12, `devip_waker` 2 =
22, one below the 23 the reconciliation table carries from Phase 3, whose own
module breakdown also sums to 22 — the current measured value is 22), `crypto`
**0** (crate-level `#![forbid(unsafe_code)]`), and `ds` **1**
(`FlattenedTphtShard::deref_tiny`'s unchecked arena read, re-measured at
`ds/src/tpht/flattened.rs:473`; the Phase 3 delta's `flattened.rs:440` above is
the old line).  The reversed link is now the fact: `port` depends on `crypto/`
and `ds/`, so the pinned upstream crates' unsafe code is reachable from both
kernel images, including the BLAKE3 SSE2, SSE4.1 and AVX2 routines that are
compiled as scalar code for the soft-float target and never selected.  No new
unsafe site of any kind was added in this phase, and the third-party record is
unchanged.

**Phase 5 delta (x86_64 joins, CORR-05, 2026-11-26):** `x86_64` opts into
`[lints] workspace = true`, becoming the fifth gated crate.  05-03 documented
the 7 aarch64 virtshell sites and the first 24 x86_64 sites (allocator, cpu,
dat, pio, seg); 05-04 documented the remaining 8 (`trap.rs` ×2, `uart16550.rs`,
`proc.rs` ×2 — SMP-03 audit sites, `main.rs` ×3) and flipped the manifest.
Comments and one manifest line only: no unsafe site was added or removed, and
kernel `.text` is unchanged.  The x86_64 per-file inventory below replaces the
legacy table's deleted-file row, and the static lint
contract's R6 now pins the opt-in on both arch manifests with both arch clippy
legs in CI.

**Plan 05-07 delta (SMP-02/03/07/12, 2026-11-26):** `port/src/smp.rs` lands
`#[forbid(unsafe_code)]` (the SmpArch hook boundary, LockRank table, debug
rank stack + watchdog, ShootdownCoordinator — zero unsafe of its own); the one
pointer bridge it needs, the publish-once `OnceStatic<SmpArch>` cell behind
`register_arch`, lives in the `mcslock` floor.  `mcslock` also gains
`IrqSpinLock`/`IrqLockGuard`, `MCSLock::try_lock`, `Lock::new_ranked`,
`Lock::try_lock` and the rank/watchdog hooks (Task 2).  Port 22 → 32,
mcslock 12 → 22, SAFETY 384 → 394 across the plan's commits.  Task 3
(SMP-03): `GlobalQuickAlloc` drops the single-CPU `AtomicPtr::swap` panic
hazard for an `IrqSpinLock<*mut QuickFit>` at `LockRank::HeapAlloc` (80),
with `arena_stats`/`split_census` serialized under it and the census tally
running into fixed stack arrays (no allocation under the lock);
`x86_64/allocator.rs` loses its `static mut` HEAP/ALLOC to SyncUnsafeCell
and both arch allocators lose their unlocked diagnostic reads (aarch64
186 → 184, x86_64 158 → 156, SAFETY 394 → 390); `riscv64/allocator.rs`
keeps its `static mut` init (never gate-compiled; Phase 7 audit horizon)
and only tracks the constructor.

**Plan 05-09 delta (SMP-02/04/07, 2026-11-26):** `aarch64/src/smp.rs` lands
the per-CPU `PerCpu` state (TPIDR_EL1), DTB `/cpus` + `/psci` discovery,
PSCI `CPU_ON`, IRQ mask/restore, TLB flush and the `SmpArch` registration
(10 blocks: the TPIDR_EL1 mrs/deref pair, the BSP TPIDR_EL1 msr, the
write-once cpu_id store, the hvc/smc PSCI call pair, the secondary_start
link-sym read, the DAIF save/restore pair, the two local_tlb_flush arms;
plus 1 unsafe extern block for the `secondary_start` link-sym declaration).
`aarch64/src/main.rs` gains the temporary `secondary_main` spin stub (1
block: the wfi park) and the init_bsp + publish_boot_state boot ordering.
The GIC SGI IPI delivery and secondary-CPU init land:
`platform/virt/gic/mod.rs` gains the `send_sgi` and `init_secondary_cpu`
dispatches (2 blocks: the write-once cell reads), `gicv2.rs` the
`GICD_SGIR` SGI write (1 block: the dsb ishst barrier) and the banked
GICC/ISENABLER0/IPRIORITYR0 secondary init (no new blocks — the existing
RegBlock writes carry the unsafe), `gicv3.rs` the `ICC_SGI1R_EL1` SGI
emission (3 blocks: the dsb ishst + isb barriers and the msr write helper)
and the per-CPU GICR RD wake + SGI frame secondary init (no new blocks —
the existing ICC helpers and RegBlock reads/writes carry the unsafe).
aarch64 184 → 202 blocks, 6 → 7 extern blocks, SAFETY 390 → 408; the
reconciliation holds.

**Plan 05-10 delta (SMP-02/03/11, 2026-11-26):** the aarch64 single-CPU
statics migrate to per-CPU/locked form.  `aarch64/src/proc.rs`: the
`CURKSTACK`/`CURFRAME` no_mangle statics and their three accessor blocks
(`read_curframe`/`write_curframe`/`write_curkstack`) are GONE — trap.S
already reads/writes `PerCpu::cur_frame` via `[tpidr_el1, #0]` (05-09),
and the scheduler's swap now stores atomically into the owning CPU's
`PerCpu` slots (safe code); the `SCHED_CONTEXT`/`SCHED_SEEDED` statics
and the churn profile's `BOOT_CTX` static move into `PerCpu`
(`sched_context`/`sched_seeded`/`park_context`/`sched_stack_top`), with
`sched_context()`'s seed-once block now carrying a per-CPU SAFETY
discipline; `PROC_TABLE` becomes `IrqSpinLock` at `LockRank::ProcTable`
(50) and `SHARED_PAGES`/`RFMEM_FAMILIES`/`SEM_WAITERS` at
`LockRank::VmShared` (30); `Proc::wake_pending: AtomicBool` plus the one
`wake_locked` critical section (flag publish + Blocked→Ready + run-queue
enqueue under PROC_TABLE) and `block()`'s flag-consume close Pitfall 4;
`pick_next` pops `port::sched::RunQueues` via `pop_or_steal` and
`enqueue_ready` wires every Ready transition.  `aarch64/src/fpu.rs`:
`this_cpu()` drops the MPIDR_EL1-indexed `CPUS` static (its `mrs` asm
block is gone) for `&smp::this_cpu().fpu`.  `aarch64/src/pagealloc.rs`:
`PAGE_ALLOC` becomes `IrqSpinLock` at `LockRank::PageAlloc` (70).
aarch64 202 → 198 blocks, SAFETY 408 → 404; the reconciliation holds.

**Plan 05-10 Task 2 delta (SMP-03, 2026-11-26):** the two IRQ-shared
mutable `SyncUnsafeCell` globals in `aarch64/src/platform/virt/` take
ranked `IrqSpinLock`s.  `pl011.rs`: `CONS_RX` is `IrqSpinLock<ConsRx>` at
`LockRank::Console` (100) — the RX IRQ writer, the `cons_rx_drain`
syscall reader, the tick's FIFO poll and the gatedbg peeks all serialize
through it (the four raw cell derefs are gone).  `pci.rs`: `VIRTIO_NET`
is `IrqSpinLock<Option<VirtioNetState>>` at `LockRank::NetIrq` (90) — the
bind publish, `virtnet_tx_post`, `virtnet_ifstats` and the IRQ handler's
ISR/TX/RX/replenish phases run under it, and the handler NEVER holds the
guard across the port-side wake/ingress callbacks (devether
`rx_ingress`/`tx_retired`, devip `note_rx_pending` — their own NetIrq-90
locks would equal-rank-nest, a D-26 violation): RX frames copy to a
2 KiB stack buffer under the guard and ingress after it drops.  The
publish-once cells (`PL011_RANGE`, `PCI_DISCOVERY`, `VIRTIO_NET_INTID`)
stay `SyncUnsafeCell` — write-once-at-boot/read-never-written-again is
not IRQ-shared-mutable state.  pl011.rs 9 → 5, pci.rs 10 → 7;
aarch64 198 → 191 blocks, SAFETY 404 → 397; the reconciliation holds.

**Plan 05-22 delta (SMP-04/07/12, 2026-11-26):** the aarch64 two-CPU
boot wiring lands.  `main.rs`: the 05-09 `secondary_main` wfi spin stub
(1 block) is REPLACED by the real bring-up tail (`gic::init_secondary_cpu`
→ `timer::init_secondary` → `mark_online` → `unmask_irq` →
`enter_sched_loop_from_boot`) — all safe calls, so the block leaves with
the stub; the new code (the `register_port_smp` call, the BSP-side SGI
0/1 enables, `start_secondaries_and_self_check`, `wait_cpu_online`, the
exec_bootstrap DT threading) adds zero unsafe.  `trap.rs`: gains the
`current_sp` primitive (1 block: the mov-from-sp read — the secondary's
sched-stack seed), placed in the trap floor per the wave-3 asm!
consolidation; the SGI 0/1 dispatch arms call safe fns only.
`smp.rs::start_secondaries` now returns the started count (safe code;
counts unchanged).  `proc.rs::enter_sched_loop_from_boot` drops its
`#[cfg(dev_churn)]` gate so the secondary entry can call it (an
attribute change; counts unchanged).  `churngate.rs::bootstrap_and_run`
takes the DT and calls `start_secondaries_and_self_check` after the
prologue (safe code; the churn profile's 2 blocks are unchanged), so
every virt config starts secondaries uniformly.  `platform/virt/timer.rs` gains
`init_secondary`, `raw_counter`/`raw_counter_freq` and the CPU-0-only
TICKS + per-CPU watchdog tick split — all safe (the register accesses
ride the pinned aarch64-cpu crate's safe wrappers).  main.rs 7 → 6,
trap.rs 8 → 9; aarch64 191 → 191 blocks, SAFETY 397 → 397; the
reconciliation holds.

**Plan 05-22 Task 2 delta (SMP-04/09, 2026-11-26):** the `-smp 2` boot
proof surfaced a chain of latent defects, each fixed at the cause.
`main.rs`: `secondary_main` gains the CPACR_EL1.FPEN=TrapNothing write
(safe tock-registers wrapper, mirroring vminit's BSP fix-up — PSCI
CPU_ON enters the secondary with FPEN at its trap-all reset value, so
the first EL0 process scheduled onto CPU 1 trapped EC=0x07) plus ONE
unsafe block (the `isb` that must follow the FPEN write), so main.rs
6 → 7; `exec_bootstrap` starts secondaries only AFTER
`activate_first` published pid 1 Running (claim-then-start — a
secondary in sched_loop would otherwise steal pid 1 inside
exec_into's Ready→Running window and double-run it) — safe code.
`gicv3.rs::init_secondary_cpu`'s redistributor frame selection gains
the HVF fallback: a nonzero GICR_TYPER affinity match stays
authoritative (TCG's userspace GICv3), an all-zero TYPER (the HVF
in-kernel vGIC does not model RD-frame TYPER/WAKER) selects by qemu
virt's cpu-order frame numbering (frame == MPIDR Aff0) — safe code,
counts unchanged.  `trap.rs`: `dispatch_irq` dispatches on the masked
INTID field (`raw & 0x3ff`) and EOIs with the raw IAR — GICv2's
GICC_IAR carries an SGI's source CPUID at [12:10], so a CPU-1 SGI 0
read as 0x400 and fell to "unhandled" (dropping every cross-CPU
resched; qemu's own EOIR masks to 0x3ff at arm_gic.c:1742);
`preempt`/`tick` gain the EL0-origin gate (an idle CPU's stale PerCpu
`current` can name a proc now Running on a sibling — a flagless
Running check rescheduled it off the sibling mid-run); the
synchronous-return note boundary reloads `cur_frame` like the IRQ
path (sys_sleep's yield switches procs mid-dispatch — delivering into
the entry frame wrote the CURRENT proc's note context over the
yielder's frame).  `proc.rs`: `preempt(interrupted_user)` gates on
the trap origin; `reschedule` picks from the RUN QUEUES (pop_or_steal
+ the Ready check), never a raw table scan that could pick a
mid-construction rfork/exec child (Ready-in-table, null frame — the
-smp 2 wild pc=0); exit's root-table + kstack frees move to the
deferred moribund reap (`defer_root_kstack_reap` 1 block + the sched
loop's `reap_moribund` 1 block — under a sibling CPU the old
"nothing allocates until the hand-off" invariant is gone, and a
just-freed kstack page was re-handed-out and written while the
exiting CPU still ran on it), so proc.rs 26 → 28.  `vm.rs`: the
kernel root table's slot-511 recursive dance is serialized across
CPUs by the new `KERNEL_VM` IrqSpinLock at `LockRank::KernelVm` (25 —
below PageAlloc 70, whose allocation runs inside the walk) — two CPUs
building procs concurrently interleaved the save/replace-walk/restore
and wrote PTEs into wrong physical pages (the wild MMIO writes);
`map_to` gains zero unsafe (the lock guard is safe code).
`smp.rs`: PerCpu gains the `moribund_reap` SyncUnsafeCell slot (no new
blocks).  `port/src/smp.rs`: `LockRank::KernelVm = 25` + the table
row (no unsafe).  `aarch64/lib/kernel.ld`: `.bootdata` moves from the
`.text.boot` output section to `.data` — vminit maps boottext+text
with `Entry::ro_kernel_text()` (RX), so 05-09's `publish_boot_state`
writes to the SECONDARY_* globals faulted on a read-only mapping
before VBAR_EL1 was installed (a silent wedge predating this plan,
latent since 05-09); the linker script carries no unsafe tokens,
counts unchanged.  `syscall.rs`: the yield's `preempt(true)` + the
exit_tail deferral call — safe code, counts unchanged.  aarch64
191 → 194 blocks, SAFETY 397 → 400; the reconciliation holds.

**05-23 debug-session delta (2026-11-26, the HVF `-smp 2` fork-sweep
corruption — aarch64-smp2-hvf-corruption):** the scheduler's kernel-stack
handoff became explicit.  `Proc` gains `on_cpu` (a safe AtomicBool: the
kstack-ownership token) and the sched loop's new `release_left_proc`
derefs the per-CPU `current_proc` pointer to read its pid (the ONE new
proc.rs block — the pointer names a PROC_TABLE-resident Proc, slots are
never freed this era, read on the owning CPU under the table lock);
`reschedule` parks this CPU's kernel context with `swtch` (its
`vm::switch` block becomes the `swtch` block — 1:1) instead of
publishing the preempted proc while still on its stack, and `pick_next`
refuses an `on_cpu` proc and carries a debug-build tripwire that panics
when another CPU's `cur_kstack` still names the candidate's stack.
`trap.rs` gains `mask_irq` (+1 block: DAIFSet #2, the sched loop's
entry discipline) and `dispatch_irq` defers `preempt` until after the
GIC EOI (safe reordering — the park makes the pre-EOI call leave the
line active while the preempted proc is off-CPU).  `main.rs`: the
secondary's premature `unmask_irq` before `enter_sched_loop_from_boot`
is gone (safe code, counts unchanged); `platform/virt/timer.rs`'s `tick`
loses its preempt call (safe code).  `port/src/sched.rs`: the new
`RunQueues::any_enqueued` probe (safe code) and its host test.
aarch64 194 → 196 blocks, SAFETY 400 → 402 (proc.rs 28 → 29, trap.rs
9 → 10); the reconciliation holds.

**Plan 05-11 Task 1 delta (SMP-01/07, 2026-11-26):** the x86_64 dual
xAPIC/x2APIC IPI delivery and AP LAPIC init land.  `acpi.rs`: the MADT
walk records enabled/online-capable processors (type-0 LAPIC and type-9
x2APIC entries) into `Madt::cpus` (capped at `MAX_CPUS` with a counted
`dropped_cpus`) — pure bounds-checked parsing, zero new unsafe.
`lapic.rs`: gains `send_icr` (xAPIC ICR_HI/ICR_LO MMIO pair with the
delivery-status poll; x2APIC `mfence; lfence; wrmsr(0x830)` per Intel
SDM Vol 3A §10.12.9 — T-05-19's mitigation) (+1 block), the
hardware-derived `current_apic_id` (+2 blocks: the IA32_APIC_BASE read
and the EXTD-gated IA32_X2APIC_ID read), and `init_ap` (+2 blocks: the
AP-side IA32_APIC_BASE read-modify-write and the CPUID-gated
IA32_TSC_DEADLINE arm from the BSP's published calibration), plus
`send_init`/`send_sipi`/`send_ipi` over the existing wrappers (safe
code), so lapic.rs 8 → 13.  `timer.rs`: the new `state()` getter reads
the publish-once STATE cell (+1 block), so timer.rs 5 → 6.  x86_64
156 → 162 blocks, SAFETY 402 → 408; the reconciliation holds.  (The
x86_64 per-file inventory header below was stale at 158 — the rows
summed to 156 since 05-07's allocator change; the header now reads the
measured 162.)

**Plan 05-11 Task 2 delta (SMP-01/02/07, 2026-11-26):** the x86_64
SIPI trampoline and SMP bring-up module land.  `ap_trampoline.S` is
position-independent assembly (no unsafe tokens — the file is data to
the linked kernel).  `smp.rs` (new): the trampoline link-sym read and
the `copy_nonoverlapping` into PA 0x8000 (2 blocks), the AP Mach
`alloc_zeroed` + zero-valid reborrow + the five volatile mailbox word
writes (3 blocks), the registry slot reads in `num_online_cpus` and
`apic_id_of` (2 blocks), the IRQ save/restore asm pair (2 blocks), and
the `invlpg`/CR3-reload TLB flush arms (2 blocks) — 11 blocks plus ONE
unsafe extern block (the trampoline link-syms).  `dat.rs`: `mach_ptr`'s
`mov %gs:0` read and `this_mach`'s deref (2 blocks); the Mach layout
itself gains the per-CPU SMP fields with `me: u64` pinned at offset 0
(safe code — `SyncUnsafeCell` doesn't implement zerocopy's FromZeros,
so Mach drops the marker derive; no caller used it, and both
construction sites (l.S zeroing + `Mach::init`, `alloc_zeroed` +
`init_ap_identity`) stand on their own SAFETY text).  `main.rs`: the
`mod smp` mount and the temporary `ap_main` stub (2 blocks: the online
handshake store and the cli/hlt park).  (Baseline correction, not new
code: the main.rs inventory row read 9 — measured 10 at the plan's
base; the row now reads 12.)  x86_64 162 → 178 blocks, extern 2 → 3,
SAFETY 408 → 424 (the +1 past the +15 delta is the same main.rs
correction); the reconciliation holds.

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
| `mcslock` | the named floor (Lock + BusyCell + OnceStatic + IrqSpinLock primitives + the MntCore proof) — 22 blocks |
| `devip_waker` (`devip/waker.rs`) | the named 08-02 bridge (the smoltcp waker RawWaker vtable) — 2 blocks + 4 fns, all SAFETY/doc'd; mounted via `#[path]` so the `devip` parent's forbid cannot reach it |
| every other port module | unsafe-free, `#[forbid(unsafe_code)]` (the devip subtree carries per-module forbids: `devip.rs` shell + `stack`/`phyadapt`; Phase 4 adds the wire modules `chan` and its submodules, and `fid`, all forbidden) |

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
| aarch64/proc.rs `sched_context` (1 block) | block ×1 | per-CPU seed-once cell discipline (the owning CPU's PerCpu slot, IRQ-masked) — 05-10: the CURFRAME/CURKSTACK accessor trio is GONE (PerCpu atomic stores are safe code) |
| aarch64/platform/virt/pl011.rs `Pl011Uart::new` + `init_rx` + `rx_irq_cons_buffer`'s range read + `cons_rx_fifo_pending` + `rx_irq_echo` (5 blocks) | block ×5 | PL011_RANGE (publish-once before unmask) — the research-P8 lock-free IRQ path; 05-10: CONS_RX's four raw ring derefs are GONE (the ring is an IrqSpinLock at LockRank::Console 100 — safe code) |
| aarch64/platform/virt/gic/mod.rs `init`/`enable`/`ack`/`eoi` (4 blocks) + gic/gicv3.rs redistributor wake, ICC system-register access and SGI emission (7 blocks) | block ×11 | GIC write-once cell, lock-free IRQ reads (research P8); the v3 redistributor/SGI paths are the same MMIO-volatility floor |
| aarch64/platform/{virt,raspi3b}/devcons.rs `init` (1 block each) | block ×2 | write-once UART cell at platform init |
| port/devmnt.rs — NONE (wave 2) | — | the session core rides mcslock::BusyCell; msize/version are frozen set-once plain fields.  **05-09 overlap: preserved** (the four-string stat + one-outstanding-RPC discipline are byte-identical semantics) |
| aarch64/main.rs `main9`'s DTB two-pass + initrd `from_raw_parts`; `exec_bootstrap`'s `&*proc.frame` (3 blocks) | block ×3 | vminit-live DTB mapping (the header_totalsize + new two-step, wave 2); the reserved+mapped initrd; the seeded frame |
| aarch64/vmdebug.rs `recursive_root_page_table` + `print_table_at_level` (2 blocks) | block ×2 | read-only recursive-mapping walk (print path) |

### `floor` — the provably irreducible kernel floor

| File + identifier | Kind | Justification |
|---|---|---|
| aarch64/trap.rs `init` (vbar_el1), `unmask_irq` (DAIFClr), `trap_unsafe` (the trap.S entry ABI), `trap`'s 2 wfi blocks, `current_sp` (05-22's mov-from-sp) | block ×5 + extern ×1 | the trap-vector interface: asm entry/exit, vector-base install, IRQ mask, clean-halt parks, the secondary sched-stack seed read.  trap.S byte-untouched (gate-diffed); the extern is `unsafe extern trap_return` |
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
| aarch64/allocator.rs the arena Block construction in the SyncUnsafeCell init | block ×1 | the global allocator's arena — one-time static construction (05-07 removed the two unlocked diagnostic-read blocks; heap_stats/heap_census now delegate through the IrqSpinLock-guarded GlobalQuickAlloc) |
| port/allocator.rs — NONE (wave 3: `#[forbid(unsafe_code)]`) | — | the index/arena model — internals carry no raw-pointer free-list surgery; the pin tests pass with byte-verbatim bodies (pointer identity holds: base+offset materialization).  05-07 adds the non-allocating `split_census_snapshot` (fixed-capacity tally under the heap lock — SMP-03) |
| port/globalalloc.rs — the GlobalAlloc shim (wave-3 split) | fn ×7, block ×8 | THE named allocator floor: the trait's unsafe fn surface, `Block::new_from_raw_parts` (the arena edge), `QuickFit::realloc` + `copy_live_bytes`, `unsafe impl Sync for QuickFit`, the `unsafe impl Allocator for BumpAlloc`, with_allocator's IrqSpinLock-guard deref (05-07: the single-CPU AtomicPtr swap trick is gone; `IrqSpinLock` at `LockRank::HeapAlloc` serializes the heap across all cpus — SMP-03) |
| port/mcslock.rs — the sync-primitives floor: `lock`/`unlock`/`Lock::lock` ×2/`Lock::try_lock` ×2/`drop` + Send/Sync impls (7 sites), plus wave-2's BusyCell (`enter` guard's Deref/DerefMut blocks, its Send/Sync impls) + the `unsafe impl Send for devmnt::MntCore` proof (5 sites), plus 05-07's OnceStatic (`get`'s published-pointer deref — 1 site), plus 05-07's IrqSpinLock (`lock`/`try_lock` UnsafeCell protocol entries + guard data borrows + `IrqLockGuard::drop`'s unlock — 7 blocks) and its Send/Sync impls (2) | impl ×7, block ×15 | the MCS queue's raw node pointers + the try-enter cell are the synchronization protocol; the MntCore proof is the BusyCell discipline's other half; OnceStatic is the publish-once boot cell behind smp::register_arch (SMP-02); IrqSpinLock is the IRQ-safe ranked lock (SMP-07/SMP-12) — its UnsafeCell entries are the same protocol, with the IRQ mask/restore closing same-cpu IRQ re-entry |
| port/devip/waker.rs (mounted as `devip_waker`) — the 08-02 smoltcp waker bridge: the RawWaker vtable (clone/wake/wake_by_ref/drop fns + the shared wake path's slot deref block) + `Waker::from_raw` in `waker_for` | fn ×4, block ×2 | constructing a `core::task::Waker` is irreducibly unsafe (no safe constructor exists).  **Invariant: the wakee-pid slot is armed (written) before registration, read at wake, and never outlives the conv** (`&'static WakeSlot` makes it mechanical — slots live in conv-table/test statics).  The vtable wake records into the seam's pending-wake set only — never the wake hook directly (wake-after-drop, stack.rs's header).  Host pins (a)/(b) in `port/src/devip_tests.rs` exercise the wake path. |
| port/fdt.rs — NONE (wave 2) | — | `from_usize` removed; `header_totalsize` + `new` are fully safe (the raw-pointer two-pass lives at the aarch64/riscv64 boot callers) |

## Per-file inventories (measured 2026-11-26, CORR-05)

Every tracked `.rs` file with a production `unsafe` token, counted by the
comment-stripping static scan (blocks / unsafe fns / unsafe extern blocks;
cfg(test) module contents are excluded and totalled separately).  Both arch
crates are lint-opted-in, so every listed block carries a `// SAFETY:` comment
— clippy `undocumented_unsafe_blocks = deny` is green on the virtshell and
q35cpu gate configs, and the scan finds zero undocumented production sites in
cfg-gated code as well.

### aarch64/src — 196 blocks, 9 unsafe fns, 7 extern blocks (test-only: 6 blocks)

| File | Blocks | fns | ext | Surface |
|---|---|---|---|---|
| `allocator.rs` | 1 | 0 | 0 | the global allocator's arena static init (05-07: the unlocked diagnostic reads are gone; heap_stats/heap_census delegate through the IrqSpinLock-guarded GlobalQuickAlloc) |
| `exec.rs` | 6 | 0 | 0 | KZERO-alias image-page writes and seed-stack/argv construction |
| `fault.rs` | 2 | 0 | 0 | the demand pager's user-page fills through the live TTBR0 |
| `fpu.rs` | 10 | 2 | 0 | FP/SIMD (q0-q31/FPCR/FPSR) save/restore, per-CPU `CpuFp` in the PerCpu slot (TPIDR_EL1, 05-10), `with_simd` (+1 test-only block) |
| `io.rs` | 9 | 0 | 0 | the MMIO-volatility primitives every driver rides |
| `kmem.rs` | 9 | 0 | 1 | kernel.ld boundary symbols (addresses only, never contents) |
| `main.rs` | 7 | 0 | 1 | link-syms, DTB two-pass, the idle wfi, the boot-time TTBR0 switch + first EL0 entry (05-22: the secondary_main wfi park is gone — the real bring-up tail is safe calls only; Task 2 adds the secondary's CPACR_EL1.FPEN write + its trailing `isb` block) |
| `pagealloc.rs` | 2 | 0 | 0 | `VirtPage4KGuard` deref pair |
| `platform/raspi3b/devcons.rs` | 1 | 0 | 0 | write-once UART cell at platform init |
| `platform/raspi3b/mailbox.rs` | 2 | 0 | 0 | the mailbox's mapped request page |
| `platform/virt/churngate.rs` | 2 | 0 | 0 | gate marker cells (single-writer boot / single-reader) |
| `platform/virt/devcons.rs` | 2 | 0 | 0 | write-once UART cell at platform init |
| `platform/virt/gic/gicv2.rs` | 1 | 0 | 0 | the GICD_SGIR SGI write's dsb ishst barrier (05-09) |
| `platform/virt/gic/gicv3.rs` | 10 | 0 | 0 | redistributor wake, ICC system registers, SGI emission + the ICC_SGI1R_EL1 write (05-09) |
| `platform/virt/gic/mod.rs` | 6 | 0 | 0 | GIC write-once cell, lock-free IRQ ack/eoi reads + the send_sgi and init_secondary_cpu dispatches (05-09) |
| `platform/virt/pci.rs` | 7 | 0 | 0 | DTB/ECAM discovery and virtio-net bind: publish-once cells (PCI_DISCOVERY, VIRTIO_NET_INTID) + MMIO; VIRTIO_NET's cell derefs are gone (05-10: IrqSpinLock at NetIrq 90, never held across the port wake/ingress callbacks) |
| `platform/virt/pl011.rs` | 5 | 0 | 0 | PL011_RANGE publish-once (05-10: CONS_RX is IrqSpinLock at Console 100 — the ring's four raw derefs are gone) |
| `platform/virt/rtc.rs` | 1 | 0 | 0 | PL031 MMIO read on the mapped rw_device page |
| `pre_mmu/util.rs` | 7 | 2 | 0 | pre-driver early console: fixed-PA MMIO, identity-mapped pre-MMU |
| `pre_mmu/vminit.rs` | 23 | 0 | 1 | pre-MMU boot: identity-map page-table construction, isb/tlbi, DTB parse |
| `proc.rs` | 29 | 0 | 1 | the process machinery floor: link-syms, KZERO+PA table/page writes, switch/resume ABI, ProcGuard + the per-CPU sched_context seed (05-10: the CURFRAME/CURKSTACK accessor trio moved to safe PerCpu atomic stores; PROC_TABLE/SHARED_PAGES/RFMEM_FAMILIES/SEM_WAITERS are ranked IrqSpinLocks; `wake_pending` + `wake_locked`/`block()` close Pitfall 4; `RunQueues` drives `pick_next`; 05-22 Task 2: the deferred moribund reap's two PerCpu-slot blocks; 05-23: `release_left_proc`'s current-proc pid read — the kstack-ownership release) |
| `smp.rs` | 10 | 0 | 1 | per-CPU PerCpu (TPIDR_EL1), PSCI CPU_ON (hvc/smc), DTB /cpus+/psci, DAIF save/restore, TLB flush, SmpArch registration (05-09) |
| `swtch.rs` | 0 | 0 | 1 | the context-switch ABI (swtch.S); one host-test twin fn |
| `textcopy.rs` | 8 | 2 | 0 | rfork text-copy emit/call and the I-cache selftest (RO text under KZERO aliases) (+5 test-only blocks) |
| `trap.rs` | 10 | 0 | 1 | the trap-vector interface: VBAR install, DAIFClr/DAIFSet (05-23 adds `mask_irq`, the sched loop's entry discipline), trap.S entry ABI, wfi parks, the `current_sp` primitive (05-22's secondary sched-stack seed read) |
| `usermem.rs` | 7 | 0 | 0 | THE single user-memory materialization point (UserSlice/UserSliceMut/UserPtr/UserString) |
| `vm.rs` | 16 | 3 | 0 | page-table machinery: TTBR/TLB/barrier asm, entry writes, recursive-mapping resolution |
| `vmdebug.rs` | 2 | 0 | 0 | read-only recursive-mapping walk (print path) |

### x86_64/src — 178 blocks, 30 unsafe fns, 3 extern blocks (test-only: 8 blocks, 2 fns)

| File | Blocks | fns | ext | Surface |
|---|---|---|---|---|
| `allocator.rs` | 1 | 0 | 0 | the global-allocator arena's one-time static construction (05-07: `static mut` HEAP/ALLOC became SyncUnsafeCell, and the unlocked diagnostic reads moved under the IrqSpinLock guard) |
| `cpu.rs` | 13 | 6 | 0 | CPL-0 instruction primitives: `ltr`/`lgdt`/`lidt`, `_rdmsr`/`wrmsr`, `wrgsbase` (the fns); flags/sti/cli/cpuid/rdtsc/rdrand/rdseed blocks |
| `dat.rs` | 6 | 1 | 0 | Ureg↔saved-frame copies, the Mach's CpuFp cell write, `Mach::init`'s GDT/IDT/TSS load, `mach_ptr`'s `%gs:0` read + `this_mach`'s deref (05-11) |
| `exec.rs` | 5 | 0 | 0 | KZERO-alias writes into just-mapped image pages and the seed page (+2 test-only blocks) |
| `fpu.rs` | 11 | 2 | 0 | FXSAVE64/FXRSTOR64 save/restore asm (`save_raw`/`restore_raw`), per-CPU `CpuFp`, `with_simd` (+1 test-only block) |
| `main.rs` | 12 | 1 | 1 | boot descriptor arming, the mb2 stash slice, `arm_syscall_entry`'s MSR program, `halt`'s cli/hlt, the temporary `ap_main` stub's online store + park (05-11); extern: the l.S link-syms.  (The row read 9 at CORR-05 — measured 10 at the 05-11 base; the +1 is a stale-row correction, not new code.) |
| `pio.rs` | 6 | 6 | 0 | the port-I/O primitives (`inb`/`inw`/`inl`/`outb`/`outw`/`outl`, CPL 0) |
| `platform/q35/acpi.rs` | 1 | 0 | 0 | bounds-checked volatile byte reads of firmware tables |
| `platform/q35/churngate.rs` | 2 | 0 | 0 | the churn gate's one-shot stash cells (single writer at boot / single reader) |
| `platform/q35/devcons.rs` | 2 | 0 | 0 | COM1 port I/O + the write-once console cell at platform init |
| `platform/q35/ioapic.rs` | 5 | 0 | 0 | IOAPIC select/data MMIO window (bounds-checked base) |
| `platform/q35/lapic.rs` | 13 | 0 | 0 | identity-mapped LAPIC MMIO and x2APIC register MSRs; the ICR IPI write (xAPIC ICR pair + delivery-status poll; x2APIC `mfence; lfence; wrmsr(0x830)`, SDM §10.12.9); `current_apic_id`'s hardware-derived ID read; `init_ap`'s AP-side enable + deadline arm |
| `platform/q35/mod.rs` | 2 | 0 | 0 | the publish-once ACPI cell (write at print_machine_info, lock-free reads) |
| `platform/q35/netgate.rs` | 2 | 0 | 0 | the net gate driver's `hlt` park loops |
| `platform/q35/pci.rs` | 28 | 0 | 0 | MCFG/ECAM MMIO walks, 0xCF8/0xCFC config mechanism, publish-once discovery/bind cells, virtio-net IRQ path |
| `platform/q35/timer.rs` | 6 | 0 | 0 | TSC-deadline/CPUID check, publish-once timer state cells (05-11: +1 for `state()`'s cell read — the AP LAPIC-timer arming input) |
| `proc.rs` | 30 | 1 | 1 | the process machinery floor: `first_entry` ABI, pml4 map/unmap through the KZERO alias, brk/rfork unwind (SMP-03 audit sites), link-syms (+3 test-only blocks, 1 fn) |
| `seg.rs` | 3 | 3 | 0 | GDT/IDT/TSS `load` (the fns) on architecturally valid tables |
| `smp.rs` | 11 | 0 | 1 | the SIPI trampoline copy into PA 0x8000, the AP Mach `alloc_zeroed` + mailbox volatile writes, the IRQ save/restore asm pair, `invlpg`/CR3-reload flush, the Mach-registry reads (05-11); extern: the trampoline link-syms |
| `swtch.rs` | 0 | 1 | 0 | `swtch` context-switch ABI (+1 host-test twin fn) |
| `syscall.rs` | 1 | 0 | 0 | the `trap_return` tail call on the seeded frame (+2 test-only blocks) |
| `trap.rs` | 6 | 9 | 0 | the trap-vector interface: `intr_stubs` emitter, `as_thunk`, `intrspecial`/`intrregular`/`intrcommon`, `syscallentry`/`syscallreturn`, `trap_return` |
| `uart16550.rs` | 5 | 0 | 0 | COM1 port I/O and the cons_rx single-writer/single-reader ring cells |
| `usermem.rs` | 7 | 0 | 0 | the single validated user-memory entry point (UserSlice/UserSliceMut) |

### Out of scope (never compiled by the gates — classified by count only)

| Crate | File | unsafe tokens |
|---|---|---|
| riscv64 | allocator.rs 1, main.rs 1, nezha/devcons.rs 1, virt/devcons.rs 1, runtime.rs 1, sbi.rs 2, uart16550.rs 4 | **11** (+ 1 `static mut` declaration in nezha/devcons.rs, an additional non-token site) |

Rationale: the gates never compile riscv64; elimination work there would be
unverifiable — recorded, not touched.  x86_64 left this table at CORR-05
(2026-11-26): it is lint-opted-in and inventoried above.  The legacy row's two
entries named files deleted with the old x86_64 generation and are dropped.
riscv64's audit horizon is the Phase 7 sync.

## asm! consolidation note (wave 3 — landed)

asm! exists ONLY in the named floor modules (aarch64): trap.rs (the vector
interface: vbar install, DAIFClr, + the consolidated halt/idle/debug-brk/
sp-read primitives `park_forever`/`idle_wait`/`debug_sync_trap`/
`current_sp`), swtch.rs (the
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
