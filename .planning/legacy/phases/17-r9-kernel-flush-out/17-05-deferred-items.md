# 17-05 Deferred Items

## Deferred Items

- The tier1 gate's `date` check passes on a pre-existing display quirk, not a real epoch
  status: open
  **What:** `hdate -n` prints `0` on every r9-x64 kernel this era (`/dev/bintime` is absent — 9k devcons serves it, r9's does not; libc `nsec()` returns 0 on the failed open) and the sweep's `*20[0-9][0-9]*` expectation is met only because `ctime(0)` under the `EST` timezone data displays "Sun Feb  7 01:28:16 EST 2006". Measured live on BOTH sides of 17-05: identical at r9 63bfda9 (pre) and r9 59d28fa (post) — `hdate -n` = 0 at both. 17-05's regression touched only the timezone env's existence (the broken intermediate pin printed "Jan 1 1970 GMT" and reded the gate); the nomount fix restored the status quo ante, byte-for-byte display included. Root fix is Phase-14's recorded era shape (no RTC; the `nsec_now` seam awaits an epoch clock) — the honest fix is either r9 devcons serving `bintime` or the sweep's expectation keyed to `hdate -n`'s raw value, and it belongs to whichever plan first owns the clock seam. Not chased here: pre-existing, out of 17-05's scope (the discovery, not a regression).
