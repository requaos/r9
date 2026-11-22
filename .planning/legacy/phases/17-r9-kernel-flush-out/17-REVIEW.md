---
phase: 17-r9-kernel-flush-out
reviewed: 2026-11-13T06:59:46Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - ../r9/port/src/devip/conv.rs
  - ../r9/port/src/devip/ipifc.rs
  - ../r9/port/src/devip/netlog.rs
  - ../r9/port/src/devip/proto.rs
  - ../r9/port/src/devip/stack.rs
  - ../r9/port/src/devip_tests.rs
  - ../r9/port/src/devip/waker.rs
  - ../r9/port/src/namec.rs
  - ../r9/port/src/namec_tests.rs
  - ../r9/port/src/netif.rs
  - ../r9/port/src/virtio.rs
  - ../r9/port/src/virtio_tests.rs
  - ../r9/x86_64/lib/config_q35churn.toml
  - ../r9/x86_64/src/exec.rs
  - ../r9/x86_64/src/platform/q35/churngate.rs
  - ../r9/x86_64/src/platform/q35/pci.rs
  - ../r9/x86_64/src/proc.rs
  - ../r9/x86_64/src/syscall.rs
  - ../r9/xtask/src/main.rs
  - build/x86_64/root-tree/lib/netstress.rc
  - build/x86_64/stress-r9.expect
  - flake.lock
  - flake.nix
  - nix/stress-r9-net-x64.sh
  - sys/src/cmd/rc/exec.c
  - sys/src/cmd/rc/plan9.c
  - sys/src/cmd/rc/simple.c
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
  necessity: 0
status: issues_found
---

# Phase 17: Code Review Report

**Reviewed:** 2026-11-13T06:59:46Z
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

## Summary

Adversarial pass over the r9 Kernel Flush-Out phase's 28 declared files: the devip stack (conv/ipifc/netlog/proto/stack/waker + host pins), namec's union-consult/nomount work, netif/virtio hardening, the x86_64 proc/exec/syscall OOM-release and sem machinery, the churngate leak gate and its xtask post-pass, the tiered stress gate (netstress.rc, stress-r9.expect, nix wrapper, flake wiring), and the three Plan 9 rc corner fixes. The flake.lock diff (routine r9 rev bump `208ceba` → `8f3ff11`, narHash updated) is structurally sound.

Overall this is an unusually disciplined changeset: the OOM-release unwinds in `pml4_map_user`/`new_user_proc`/`rfork_copy` balance page-for-page, the used-ring slot-liveness reject (WR-06/17-03) and the cap-walk admission holes (zero-length extents, minimum common/device lengths, 64-bit BAR nibble trust) are correctly implemented and pinned, the `NetWaiterScope` deregistration counterpart genuinely covers every exit path, the sem registry consume/re-register discipline is sound, and the three rc C fixes (`Xqdol`/`Xcount` pushlist guards, `Execute` first-candidate errstr, checked `Dup`) are correct against their donors. No Critical findings.

Three Warnings survived verification: (1) `sys_brk_`'s private growth arm re-maps pages that a prior shrink left recorded, producing duplicate `mapping_log` entries whose first-match consumers (`sem_word_pa`) key on a stale, eventually-freed physical page; (2) `conv_close`'s pending-incall sweep resets convs unconditionally, zeroing an in-use conv when its files were opened directly by id; (3) the accept-pool pop leaks the popped member socket back into the seam on two of its own error arms. Five Info items cover a capacity-comment falsehood under `dev_churn`, wrong generation-count arithmetic in churngate comments, a watchdog-expiry accounting slip, an unapplied `nomount` bit in `sys_create`'s bare-path arm, and a committed yacc artifact with developer-machine absolute paths riding the same diff (outside the declared file list).

Out-of-scope observations for the record (files NOT in the declared list, so unreviewed in depth): `sys/src/cmd/eqn/eqn.c` (see IN-05), and `port/src/devip.rs` — whose `tcp_listen_pop` error arms are the reachable half of WR-03.

## Critical Issues

(None.)

## Warnings

### WR-01: sys_brk_ regrowth over shrink-recorded pages creates duplicate mapping_log records and a stale semaphore PA key

**File:** `../r9/x86_64/src/syscall.rs:814-827` (with `../r9/x86_64/src/proc.rs:1375-1392`)
**Issue:** `sys_brk_`'s private arm lowers `bss.0.end` on a shrink without unmapping or dropping the still-recorded pages (`proc.segments.as_mut().unwrap().bss.0.end = addr;` — `mapping_log` untouched). A later growth back through the same range calls `proc.map_user_page(va, SegKind::Bss)` for VAs that are still present in `mapping_log`; `map_user_page` allocates a fresh page and `pml4_map_user` silently REPLACES the present leaf (its `was_present` → `invlpg` path), then appends a SECOND record for the same VA. Consequences: (a) the shrunken page stays allocated until exit (pinned, not leaked at exit — both records free exactly once); (b) first-match consumers read the STALE record — `sem_word_pa` (`syscall.rs:862-868`) returns the old physical page, which is freed at exit (or at any later full unmap) while the semaphore word is still live; after the allocator recycles that page, two unrelated procs' semaphore words can key the same PA in `SEM_WAITERS`, cross-waking each other's wait queues. Note the RFMEM bss-sweep in `rfork_copy` (`proc.rs:2603`) DOES guard with `parent.mapping_log.iter().any(|m| m.va == va)` — the private arm is missing the same check. Reachable only via a heap shrink followed by regrowth with a semaphore placed in the regrown page (rc's malloc does not shrink), hence Warning rather than Critical.
**Fix:** Mirror the RFMEM sweep's liveness check in the private growth arm:

```rust
let mut va = grown_start;
while va < grown_end {
    if proc.mapping_log.iter().any(|m| m.va == va) {
        va += page;
        continue;                       // still mapped from before the shrink
    }
    proc.map_user_page(va, SegKind::Bss)?;
    va += page;
}
proc.segments.as_mut().unwrap().bss.0.end = addr;
```

### WR-02: conv_close's pending-incall sweep resets in-use convs unconditionally (donor closeconv decrements)

**File:** `../r9/port/src/devip/conv.rs:606-611`
**Issue:** On the announcer's last close, the sweep runs `for nid in incall { table[nid].close_reset(); }` with no `inuse` check or decrement. `fs_new_call`/`fs_new_call_tcp` set `inuse = 1` (the queue's own reference), and `devip::open`'s QCTL/QDATA/QERR arm (`conv_open`, `conv.rs:533-547`) allows opening an ALLOCATED conv directly by id — `walk /net/udp/N/ctl` on a still-queued spawned conv bumps it to 2. The announcer's close then zeroes that conv under live fds: the victim's open chans now point at a fresh Idle conv, and its own eventual `conv_close` finds `inuse == 0` and no-ops — the reference that the direct open took is silently destroyed. The 9k donor's `closeconv` (which this code cites: "closeconv's incall sweep") decrements `c->inuse` and only resets at zero, so a directly-opened incall conv survives until its own close. The inline comment's premise "idle tables, no fds reference them" is the unverified assumption that makes this a bug.
**Fix:** Decrement and reset at zero, matching the cited donor semantics:

```rust
for nid in incall {
    let c = &mut table[nid];
    if c.inuse > 0 {
        c.inuse -= 1;
    }
    if c.inuse == 0 {
        c.close_reset();
    }
}
```

### WR-03: Accept-pool pop leaks the member socket on two of its own error arms

**File:** `../r9/port/src/devip/stack.rs:754-763` (consumer at `../r9/port/src/devip.rs:860-871`, adjacent to the declared scope)
**Issue:** `AcceptPool::take_connected()` removes the member from the pool and returns the raw `SocketHandle`, transferring ownership to the caller. The consumer (`tcp_listen_pop` in devip.rs) aborts+removes the handle only on the table-full arm; on the `Err("devip: no bound medium")` arm and on the v6-quad arm (`return Err("bad ip address syntax")`) the popped handle is dropped WITHOUT `abort()`/`sockets.remove()` — a live socket left permanently in the seam's `SocketSet`, owned by neither pool nor conv (it lingers as an open listener until the whole core is unbound). The v6 arm is reachable by network input: `spawn_listener` (`stack.rs:814-821`) calls `s.listen(port)` with an address-less `IpListenEndpoint`, which accepts BOTH families, so a v6 SYN to the announced port occupies a member whose quad then fails the pop's v4-only destructure. Each such event leaks one socket into the set. (devip.rs is outside the declared file list; the ownership contract that makes this footgun possible lives on `take_connected` in the reviewed stack.rs, so the finding is reported here with the caller cited.)
**Fix:** In `tcp_listen_pop`'s error arms, abort and remove the popped handle exactly like the table-full arm; or push the member back into the pool when validation fails. Longer term, have `take_connected` return the member only after the caller validates (e.g. pass a validator closure) so the pool's "a dropped pool NEVER leaves live sockets behind" invariant cannot be violated from outside.

## Info

### IN-01: dev_churn's MAX_PROCS=2048 falsifies the "WaitSet sized to the live proc table (512 on both arches)" capacity claim

**File:** `../r9/x86_64/src/proc.rs:119-120` (claims at `../r9/port/src/devip/stack.rs:172-178`, `../r9/port/src/netif.rs:330-335`, `../r9/port/src/devip/conv.rs:119-125`)
**Issue:** The WaitSet/WaitPids capacity comments ("sized by the shared MAX_WAITERS const to the LIVE proc table — 512 on both arches", WR-05/17-03's premise) hold only while `MAX_PROCS == MAX_WAITERS == 512`. The same phase introduced `#[cfg(dev_churn)] pub const MAX_PROCS: usize = 2048`, so under the churn profile the fixed sets are 4x smaller than the table and the 513rd distinct devip/netif waiter would hit the loud full arm — precisely the invariant mismatch the 17-03 fix existed to close. Unreachable today (dev_churn's dev list is `["exec","churn"]` — no devip/netif activity), but any future `dev_churn` + `dev_ip`/`dev_ether` build silently loses the sizing guarantee the comments promise.
**Fix:** Either derive `MAX_WAITERS` from (or assert against) the per-arch `MAX_PROCS`, or note the dev_churn exception in the three capacity comments.

### IN-02: churngate generation-count arithmetic in comments is wrong (39 generations, not 40)

**File:** `../r9/x86_64/src/platform/q35/churngate.rs:77-79` (also `../r9/x86_64/lib/config_q35churn.toml:11-12`)
**Issue:** `SEM_EVERY`'s comment says "1400 / 35 = 40 generations", but the loop `for i in (BASELINE_N + 1)..=CHURN_N` (41..=1400) first hits a multiple of 35 at 70, yielding (1400-70)/35+1 = 39 generations (and 1321 churnmark execs, not 1320). The kernel counts at runtime and the xtask post-pass counts from the wire, so no functional defect — but the comment, the `done_marker(1400, 40)` shape fixture (`churngate.rs:507`), and the config comment all encode the wrong expectation and will mislead the next person auditing the exact-count contract.
**Fix:** Correct the comments to 39 (or move the first generation boundary so the arithmetic matches the comment).

### IN-03: churngate spawn_and_wait's watchdog-expiry path leaves children_live elevated and a stale wait record possible

**File:** `../r9/x86_64/src/platform/q35/churngate.rs:411-427`
**Issue:** The waited-exec arm does `me.children_live += 1` before the wait loop, but the watchdog-expiry `return Err("exec child did not exit inside the budget")` never decrements it. If the child exits after the driver gave up, its wait record posts to a `children_live` that no longer matches reality; a subsequent wait in the same run would pop a record for the wrong child. Contained in practice — expiry routes straight to `fail()`/`finish(false)`/`exit_tail`, so the driver never waits again — but the accounting is one-sided on the error path.
**Fix:** On the expiry path, `me.children_live = me.children_live.saturating_sub(1)` and drain any record that later arrives (or simply document that the driver exits on expiry).

### IN-04: sys_create's bare-path arm derives nomount but does not apply it to the dot-redirect consult

**File:** `../r9/x86_64/src/syscall.rs:1161-1185`
**Issue:** `nomount` is computed from `path[0] == b'#'` and correctly threaded into `walk_one_union`/`create_chan_union`, but the `split == None` arm (a path with no '/', which includes a bare `"#e"`-style device path) resolves the parent as `dot` with an UNCONDITIONAL union-chain consult on `d.path` — the same empty-key/root-union collision class the nomount bit was added to prevent (the tier1 timezone regression). Impact is confined to an inconsistent error surface: both the donor and r9 ultimately error for `create("#e")` (the donor errors on the empty final component at the devenv root; r9 errors "namec: device has no create" after consulting the root union), and real callers always use `"#e/name"`.
**Fix:** Gate the None-arm's chain consult on `!nomount` for consistency (skip the redirect when `path[0] == b'#'`), or assert the shape is unreachable and document it.

### IN-05: Regenerated yacc output committed with developer-machine absolute paths (outside declared scope, present in the phase diff)

**File:** `sys/src/cmd/eqn/eqn.c:59` (254-line diff vs `d5894b9e`; not in the declared review file list)
**Issue:** Riding the same phase diff is a regenerated `eqn.c`: `#line` directives rewritten from `/sys/src/cmd/eqn/eqn.y` to `/home/req/code/harvey/sys/src/cmd/eqn/eqn.y` and tables changed from `short yyact[]` to `static const short yyact[]`. This is a build artifact that leaked into the commit with machine-specific absolute paths baked in — it will show as spurious drift for every other checkout and re- regenerate on the next `yacc` run. Flagged here because it is part of the phase's changeset even though it sits outside the files this review was scoped to.
**Fix:** Revert `sys/src/cmd/eqn/eqn.c` (or regenerate with the canonical `/sys/src/...` paths from inside the namespace build) and keep generated parser output out of source commits.

---

_Reviewed: 2026-11-13T06:59:46Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
