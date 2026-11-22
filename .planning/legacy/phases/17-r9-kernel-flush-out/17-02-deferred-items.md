# 17-02 Deferred Items

Findings surfaced by this plan's gates that belong to later plans/rows.

- rc's userland malloc pool corrupts under sustained backquote churn (~428+ iterations of `while(! ~ ...)` + a dc backquote counter per iteration): "bad arena size / pool sbrkmem block <addr> / hdr ... deadbeef ..." then "panic: pool panic".  Measured live on the composed churn venue (17-02, twice, deterministic threshold); the kernel side held flat through the corrupting runs (the per-exit heap markers never moved).  The composed churn leg now drives a C churn driver (churncats.c) so the gate's subject stays the kernel's exec path.  **Status:** open — for 17-03's rc rows (#46/#49/#65 class) to root-cause; the repro shape is committed in build/x86_64/root-tree/lib/churn-cats.rc (the original fixture) under `rc /lib/churn-cats.rc` at the composed prompt.
  status: open
  **What:** rc or libc malloc frees-then-uses under per-iteration backquote allocation.
- r9's exit tail is not 9P-safe (a devmnt clunk is a blocking RPC; dispatching one from mid-exit measured userland pool corruption in the mounted venue — the sweep now drops chans without the close dispatch, leaving the husk's server-side fid to leak bounded until the server dies).  The real fix (exit-path 9P safety, or a deferred-clunk queue) is follow-up work.
  status: open
  **What:** devmnt clunk-from-exit-context; recorded in the sweep_husk_payloads doc comment (r9 proc.rs).
- sys_sleep stores MILLISECONDS into sleep_until but compares against 100 Hz TICKS (syscall.rs sys_sleep: `deadline = ticks() + ms`) — sleep(N) waits ~10x longer than asked.  Benign today (its only users wake early via the alarm note), observed while building the churn driver's bounded waits (which use tick units directly).
  status: open
  **What:** a units slip in sys_sleep's deadline arithmetic.
