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
