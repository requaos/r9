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
