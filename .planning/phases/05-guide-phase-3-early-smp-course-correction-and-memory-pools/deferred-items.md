# Deferred items — Phase 5 (plan 05-08)

Out-of-scope discoveries made during plan 05-08, not fixed by it:

- **devether.rs's block loops keep a pre-existing 2-CPU check/register gap.**
  `read_data` / `write_data` do `netif::pop_frame` (or `tx_post`) in one
  INGRESS acquisition, drop the lock, then `register_rx_waiter` /
  `register_tx_waiter` in a SECOND acquisition, then `block()`.  On two
  CPUs an IRQ on the sibling can land between the empty/full check and the
  registration: it queues the frame, drains an empty waiter set, and sends
  no wake — the reader parks with a queued frame.  Closing it needs the
  check and the registration in ONE critical section (a structural change,
  Rule 4), plus the same `prepare_block()` arm the plan gave
  devpipe/devcons/devip-conv.  Plan 05-08's audit scope was explicitly
  `devpipe.rs`, `devcons.rs` and `devip/conv.rs`, and netif.rs's BusyCell →
  IrqSpinLock conversion (this plan) does not alter the window's shape.
  Owner: the SMP-03 arch-side plans (05-10/05-12) that wire
  `wake_pending` into `block()`/`wake()`, or a follow-up port/ plan.
  status: acknowledged

# Plan 05-22 (2026-11-26) — residual from the -smp 2 gate proof:

- **aarch64 -smp 2 under HVF (gicv3, -cpu host) has a residual
  fork/exec-load corruption that only real-hardware timing exposes.**
  With the 05-22 fix chain committed (ceb6ecc), the TCG gicv2 -smp 2
  smoke leg is twice-consecutive green and R9_SMP=1 is solid (2/2), but
  the HVF -smp 2 leg passes only ~50% of full smoke runs.  The failure
  signature is always in the fork/exec/exit sweep: either garbage bytes
  on the serial wire right after "init: starting /bin/rc" (a torn
  print/copy — e.g. `sched: switch` interleaved with 8-bit garbage), or
  a stall mid page-in sweep, or (pre-fix-chain) qemu's
  `hvf.c:2447 assert(isv)` — an ISV=0 MMIO write (a wild pointer to a
  device PA).  This is the WINDOWS #49 class (fork-child corruption,
  HVF-timing-exposed, TCG the truth venue) AMPLIFIED by two-CPU fork
  concurrency — the measured rate (~50%/boot over the smoke's ~20 forks)
  is far above #49's hardened residual (~1.8e-5/fork), so it is a NEW
  SMP-specific instance, not the #49 residual itself.  Not fixed here
  (fix-attempt limit): the next slice should start from a pagealloc
  poison-on-free tripwire (the corruption looks like a page reused while
  still referenced) and re-check TextCopy's cross-CPU icache/dcache
  discipline under real HVF.  Owner: the phase's SMP-08 stress gate or a
  dedicated debug session.
  status: FIXED (2026-11-26, commit 339036f, debug session
  aarch64-smp2-hvf-corruption)

  **Root cause (not the pagealloc/icache guesses above — both were
  eliminated):** two kernel-stack ownership violations that only real
  (HVF) interrupt timing exposes on 2 CPUs.  (1) `reschedule()`
  published the preempted proc Ready+enqueued while the CPU still ran on
  its kernel stack, then spent the long `sched: switch` console print
  there (every UART byte is an MMIO exit under HVF): a sibling CPU could
  pop the pid and trap_return into it, taking traps on the same 16 KiB
  kstack — the qemu monitor showed a CPU with SP_EL1 inside the other
  CPU's kstack, both CPUs then spinning in the CONS console lock's MCS
  wait loop (the wire stops mid-line, e.g. `paqfs...s`).  `block()` had
  the same shape (publish Blocked, drop the lock, then swtch).
  (2) `secondary_main` unmasked IRQs before
  `enter_sched_loop_from_boot`, so the sched loop ran interruptible: an
  IRQ between `pick_next`'s `write_curframe(p.frame)` and
  `trap_return`'s reload overwrote PerCpu.cur_frame with an EL1h frame
  from the scheduler stack and the tail restored that instead of the
  picked proc's frame.
  **Fix:** `Proc::on_cpu` (the kstack-ownership token) + the sched
  loop's `release_left_proc` publish step (reschedule parks with swtch
  and the proc becomes pickable only after the CPU is off its kstack);
  `pick_next` refuses an on_cpu proc and carries a debug-build tripwire
  (panics when another CPU's cur_kstack still names the candidate);
  `sched_loop` masks IRQs on entry and `secondary_main` no longer
  unmasks before it; `dispatch_irq` EOIs before `preempt` (which now
  parks the CPU).
  **Verified:** HVF gicv3 -smp 2 smoke 8/8 consecutive (6/6 then 2/2 on
  the committed tree; pre-fix 6/13), TCG gicv2 -smp 2 2/2, R9_SMP=1
  2/2, `cargo xtask check` and `cargo xtask test` green.
  **WINDOWS #12** closed by the same commit.  Follow-up: the x86_64
  twin (05-24/05-25) should adopt the same handoff rule when it gains
  AP bring-up — its reschedule already parks via swtch (14-06), but it
  publishes Ready before the swtch and has no kstack-ownership token, so
  the same class opens the moment a second CPU runs procs there.

# Plan 05-12 (2026-11-26) — residuals for the x86_64 -smp 2 boot proof
# (05-14, Owner Stop 2):

- **x86_64's scheduler carries the 05-22 shape BEFORE its hardening.**
  05-12 ported the 05-10 migration shape (the plan's designated mirror):
  `reschedule()` marks the preempted proc Ready + enqueues it under
  PROC_TABLE and only then swtch'es off its kernel stack, and
  `sched_from_moribund` frees the kstack before the swtch ("nothing
  allocates until the hand-off" — false under a sibling CPU).  The
  05-22-proven fixes (the `on_cpu` kstack-ownership token +
  `release_left_proc` publish-off-the-kstack + the deferred moribund
  reap) are NOT ported: 05-12's verification is compile/clippy/test
  only (x86_64 cannot boot on the aarch64 dev host), and the phase
  assigns the live -smp 2 boot proof to 05-14 (CI TCG + owner KVM).
  05-12 did port the cheap half: `proc::preempt(interrupted_user)` gates
  on the trap frame's ring-3 origin (the stale-current double-run tear
  is closed from the start).  Owner: Plan 05-14 (the x86_64 -smp 2 boot
  proof) — enter with `on_cpu` + `release_left_proc` + the deferred
  reap queued from 05-22's fix list (ceb6ecc) before the stress legs.
  status: fixed (2026-11-26, follow-up commit c9d43c8 — the 339036f +
  665c94e port): x86_64 now has `Proc::on_cpu` set by
  pick_next/activate_first and cleared only by the owning CPU's
  sched-loop `release_left_proc` (owner-scoped via the
  `Mach.current_proc` consume), the park-only `reschedule`, the
  pick_next refusal + `owned_pick_give_up` cycle guard + the
  debug-build tripwire reading `Mach.cur_kstack` cross-CPU (the TSS
  rsp0 is never the witness — it is not retracted on the park), the
  deferred moribund reap (`Mach.moribund_reap` +
  `defer_root_kstack_reap`/`reap_moribund`), the IRQ-masked sched-loop
  entry (`trap::mask_irq` — the dev_churn boot path reached it with
  IF=1), and the EOI-before-preempt `dispatch_irq`.  The
  `sched: switch` marker is emitted from pick_next via
  `Mach.sched_from`.  Verified by compile/clippy/host-test/build only
  (no x86_64 boot on this host): `cargo xtask check`, clippy
  x86-64/q35cpu + aarch64/virtshell, `cargo xtask test`,
  `cargo test -p xtask lintcontract`, all nine q35* config builds — all
  exit 0.  The two new x86_64 host pins run on CI's x86_64 runners
  (the crate harness is x86_64-host-only — the WINDOWS #13/#14
  constraint; their logic passed a standalone extraction run here).
  The live -smp 2 boot proof remains 05-14's (CI TCG + owner KVM).

# Plan 05-14 (2026-11-26) — pre-existing net-smoke failure found during
# the aarch64 quick pass:

- **aarch64 net-smoke panics during bss page-in, at both -smp 1 and
  -smp 2.**  `nix run .#boot-r9-net-smoke` (HVF, default -smp 2) and
  `R9_SMP=1 nix run .#boot-r9-net-smoke` both fail with an identical
  truncated panic (`panicked at` — the PL011 console buffer caps the
  message) after the bss page-in sweep.  The panic is NOT SMP-related
  (identical at both CPU counts) and NOT caused by 05-14's changes
  (x86_64 gate scripts + CI YAML only).  The network stack files
  (devip/*, netif.rs) were converted to SMP-safe locking in 05-08;
  this may be a regression from that conversion or a pre-existing
  issue that predates Phase 5.  All other aarch64 quick pass items
  (smoke, cpu, tier-1 HVF+TCG, xtask test, all clippy legs) pass green.
  Owner: needs a debug session to capture the full panic message (the
  truncated console output hides the source location).
  status: open
