---
phase: 08-r9-tcp-ip-path-devip-compatible-net
reviewed: 2026-11-06T00:00:00Z
depth: standard
files_reviewed: 53
files_reviewed_list:
  - /Users/neilskinner/code/r9/Cargo.toml
  - /Users/neilskinner/code/r9/HowItWorks.md
  - /Users/neilskinner/code/r9/UNSAFE-AUDIT.md
  - /Users/neilskinner/code/r9/aarch64/lib/config_virtip.toml
  - /Users/neilskinner/code/r9/aarch64/lib/config_virtshell.toml
  - /Users/neilskinner/code/r9/aarch64/src/fault.rs
  - /Users/neilskinner/code/r9/aarch64/src/main.rs
  - /Users/neilskinner/code/r9/aarch64/src/platform/virt/devcons.rs
  - /Users/neilskinner/code/r9/aarch64/src/platform/virt/pci.rs
  - /Users/neilskinner/code/r9/aarch64/src/platform/virt/timer.rs
  - /Users/neilskinner/code/r9/aarch64/src/proc.rs
  - /Users/neilskinner/code/r9/aarch64/src/syscall.rs
  - /Users/neilskinner/code/r9/docs/devip-contract.md
  - /Users/neilskinner/code/r9/port/Cargo.toml
  - /Users/neilskinner/code/r9/port/src/dat.rs
  - /Users/neilskinner/code/r9/port/src/devcons.rs
  - /Users/neilskinner/code/r9/port/src/devether.rs
  - /Users/neilskinner/code/r9/port/src/devip.rs
  - /Users/neilskinner/code/r9/port/src/devip/arp.rs
  - /Users/neilskinner/code/r9/port/src/devip/conv.rs
  - /Users/neilskinner/code/r9/port/src/devip/ipifc.rs
  - /Users/neilskinner/code/r9/port/src/devip/medium.rs
  - /Users/neilskinner/code/r9/port/src/devip/netlog.rs
  - /Users/neilskinner/code/r9/port/src/devip/phyadapt.rs
  - /Users/neilskinner/code/r9/port/src/devip/proto.rs
  - /Users/neilskinner/code/r9/port/src/devip/route.rs
  - /Users/neilskinner/code/r9/port/src/devip/stack.rs
  - /Users/neilskinner/code/r9/port/src/devip/waker.rs
  - /Users/neilskinner/code/r9/port/src/devip_tests.rs
  - /Users/neilskinner/code/r9/port/src/fdgrp.rs
  - /Users/neilskinner/code/r9/port/src/lib.rs
  - /Users/neilskinner/code/r9/port/src/namec.rs
  - /Users/neilskinner/code/r9/port/src/netif.rs
  - /Users/neilskinner/code/r9/port/src/rendez.rs
  - /Users/neilskinner/code/r9/xtask/src/main.rs
  - /Users/neilskinner/code/r9/xtask/src/pcap.rs
  - /Users/neilskinner/code/harvey/.github/workflows/r9-shell.yml
  - /Users/neilskinner/code/harvey/build/aarch64/mkimage.sh
  - /Users/neilskinner/code/harvey/build/aarch64/netaddr.rc
  - /Users/neilskinner/code/harvey/build/aarch64/smoke-r9-net.expect
  - /Users/neilskinner/code/harvey/build/aarch64/subset-build.sh
  - /Users/neilskinner/code/harvey/build/aarch64/tcpblast.c
  - /Users/neilskinner/code/harvey/build/aarch64/tcpecho.c
  - /Users/neilskinner/code/harvey/build/aarch64/root-tree/lib/netip-aan-drop.rc
  - /Users/neilskinner/code/harvey/build/aarch64/root-tree/lib/netip-aan.rc
  - /Users/neilskinner/code/harvey/build/aarch64/root-tree/lib/netip-dhcp.rc
  - /Users/neilskinner/code/harvey/build/aarch64/root-tree/lib/netip-loopback.rc
  - /Users/neilskinner/code/harvey/build/aarch64/root-tree/lib/netip-soak.rc
  - /Users/neilskinner/code/harvey/build/aarch64/root-tree/lib/netip-static.rc
  - /Users/neilskinner/code/harvey/build/aarch64/root-tree/lib/netip-tcp.rc
  - /Users/neilskinner/code/harvey/build/aarch64/root-tree/lib/netip-v6.rc
  - /Users/neilskinner/code/harvey/flake.nix
  - /Users/neilskinner/code/harvey/nix/smoke-r9-net.sh
findings:
  critical: 7
  warning: 6
  info: 7
  total: 20
status: issues_found
---

# Phase 08: Code Review Report

**Reviewed:** 2026-11-06
**Depth:** standard (full per-file reads of the phase's kernel/arch/harness changes; smoltcp 0.14.0 crate source consulted to confirm panic/error semantics; donor `sys/src/9k/ip/` consulted for tcpgc/devip lifecycle parity)
**Files Reviewed:** 53 (two repos: r9 `a5b1b42..19f61ed`, harvey 08-scoped commits in `e3de28fd2..HEAD`)
**Status:** issues_found

## Summary

The phase builds a devip-compatible `/net` over smoltcp 0.14 in r9 (two-core seam, accept pool, waker bridge, RFMEM + semaphore pair, DTB-seeded ChaCha20 DRBG) plus Harvey gate fixtures and a composed CI smoke. The engineering discipline is visibly high: locking orders are documented and consistent (TABLES → STACK nesting, wake-after-guard-drop), the unsafe surface is one audited RawWaker module, panic-boundary thinking (P-F) is applied at most smoltcp edges, and the harness fixtures are loud-on-failure with sane bounds-checked parsing.

The review nonetheless found seven defects in the block/wake/close lifecycle class — the exact class this phase's own evidence flags as fragile. Three are user-reachable hangs or kernel panics (CR-02, CR-03, CR-04), two are state aliasing bugs (CR-05, CR-06), and one silently breaks TCP close semantics on the primary close path (CR-01). All phase gates pass with these defects present — the gates' single-connection, teardown-by-qemu-kill shapes mask them (analysis under CR-01).

## Critical Issues

### CR-01: conv_close drops the FIN — `close()` then `remove()` before any poll

**File:** `/Users/neilskinner/code/r9/port/src/devip/conv.rs:593-603` (drive at 617-619)
**Issue:** On the last close of a tcp conv, the code runs `s.close()` (smoltcp 0.14: only transitions state to FinWait1, FIN queued internally — verified against `socket/tcp.rs:1084`) and then **immediately** `core.sockets.remove(h)` (smoltcp 0.14 `socket_set.rs:124`: destroys the socket, emits nothing). Only afterwards (line 617-619) does `drive_stack()` run — but `poll_egress` can only transmit for sockets still in the `SocketSet`, and this socket is gone. The comment at 589-592 ("the removal follows immediately — the FIN leaves at the drive below") is factually wrong: the FIN never leaves. Consequences: (a) the peer never receives FIN → never sees EOF — any peer blocked in a read stays blocked forever; (b) any TX data queued but not yet emitted is silently dropped. This is the *primary* close path: Harvey programs close fds on exit; the `ctl close`/`hangup` verbs are a different (working) path. The gates are blind to it: every tcp fixture is single-connection, asserts on the echoed bytes, and the guest is torn down by qemu kill — tcpecho's `while(read>0)` EOF loop (build/aarch64/tcpecho.c:40) wedges after the client exits, invisibly. Also note `tcp_close_hangup_hangupxmit_wire_shapes` (devip_tests.rs:1485) pins only the *verb* path, which keeps the socket until conv_close.
**Fix:** Don't remove a tcp socket at close() time. Sequence: `s.close()`, `drive_stack()` with the socket still in the set, and defer `remove()` until the socket reaches `State::Closed` (a reap pass in `drive_stack`, or on the next conv op) — the donor keeps the tcb alive until the FIN is acked/aged out. At minimum, emit the FIN before removal: close-then-drive-then-remove fixes FIN delivery but still drops the retransmit/ACK-of-FIN tail; the lingering-socket design is the correct fix.

### CR-02: Permanent smoltcp send errors wedge the writing proc forever (icmp/udp/icmpv6 data writes)

**File:** `/Users/neilskinner/code/r9/port/src/devip/proto.rs:715-742` (icmp), `1316-1345` (udp), `1677-1706` (icmpv6)
**Issue:** All three write loops treat **every** `send_slice` error as transient "TX buffer full → register/drive/block". smoltcp 0.14 collapses two permanent failures into the same `SendError::BufferFull`/`Unaddressable` returns (verified: `socket/udp.rs:298-331`, `socket/icmp.rs:331-343`, `storage/packet_buffer.rs:80-83`):
  1. **Oversized payload** — `PacketBuffer::enqueue` returns `Full` when `payload_ring.capacity() < size`, *forever* (the 64 KiB `ICMP_BUF`/`UDP_BUF`). A user write >64 KiB to `/net/{icmp,udp,icmpv6}/N/data` can never succeed. On a quiet connection nothing is ever enqueued, no send-waker ever fires, and `poll_at` disarms the deadline — the proc blocks with no wake source: unkillable wedge. On a busy connection it live-locks (wake → retry → Full → block).
  2. **Unaddressable endpoint** — a write to an *announced* (never-connected) udp/icmp conv reaches `send_slice` with dst `0.0.0.0`/port 0 (`udp_write_data`'s non-headers arm uses the conv's cleared raddr/rport) → `SendError::Unaddressable` → same infinite block loop. Writing to a listener's data file is ordinary user behavior.
  The donor never wedges here (the write succeeds and the packet drops unroutable, or errors Emsgsize-class). tcp's write path is unaffected (chunked `send_slice` with `Ok(0)`/Err → error return).
**Fix:** Match on the error: `Unaddressable` → return `Err("...")` immediately; `BufferFull` → first check `payload.len() > BUF_CAP` → return a sized error (or the donor's succeed-and-drop); only block when the packet can ever fit. Add the same guard at the top of each write (`n` vs the socket buffer capacity).

### CR-03: semacquire loses wakeups — a woken waiter that loses the unit race re-blocks unregistered

**File:** `/Users/neilskinner/code/r9/aarch64/src/syscall.rs:1088-1095` + `/Users/neilskinner/code/r9/aarch64/src/proc.rs:379-402` (`sem_wake`)
**Issue:** `sys_semacquire` registers the waiter **once** before the loop:
```rust
proc::sem_register_waiter(pa, proc.pid);
loop { if sem_try_acquire(&mut word) { break; } proc::sched(); }
proc::sem_deregister_waiter(pa, proc.pid);
```
but `sem_wake` **consumes** registrations (`waiters.remove(i)`) when it picks up to `delta` waiters. Sequence: word=0, procs A and B blocked; R does `semrelease(+1)` → `sem_wake` removes A, readies it; before A is scheduled, R (still running in user mode, single CPU) re-acquires the unit (fast path); A runs, `sem_try_acquire` fails, `sched()` re-blocks — **unregistered**. Future releases never find A: permanent wedge. `sem_wake`'s own comment claims "a woken waiter that loses the unit race re-registers on its next block" — the syscall loop never re-registers; the comment describes code that doesn't exist. Reachable with ≥2 waiters plus any interleaved acquirer — exactly the libc `lock()` shape RFMEM was built to serve (multi-proc libthread contention).
**Fix:** Move `proc::sem_register_waiter(pa, proc.pid)` inside the loop before each `sched()` (register → check → block per iteration, the same wakeup-loss-free shape used everywhere else in this phase), or make `sem_wake` non-destructive (mark-and-sweep on acquire). Re-check the comment against the code after fixing.

### CR-04: `ipifc add` of a v4-multicast or limited-broadcast address panics the kernel (P-F violation)

**File:** `/Users/neilskinner/code/r9/port/src/devip/ipifc.rs:355-357`
**Issue:** The P-F absorption guard covers `unspecified` (0.0.0.0/::) and v6 multicast (`!is_v4 && ip[0]==0xff`) only. A v4 multicast add (`add 224.0.0.1 255.0.0.0`) or the v4 limited broadcast (`add 255.255.255.255 ...`) falls through to `core.iface.update_ip_addrs(...)`; smoltcp 0.14's `check_ip_addrs` (iface/interface/mod.rs:896-902) **panics** on any address that is neither unicast nor unspecified — `224/4` fails `x_is_unicast` (`wire/ipv4.rs:58-60`), as does `255.255.255.255` (`is_broadcast`). Result: an unprivileged ctl write panics the kernel — the exact outcome the module header's panic-boundary rule exists to prevent. (`add6` is safe: its EUI-64 product of an ff00:: prefix is absorbed by `addr[0]==0xff` at line 637.)
**Fix:** Extend the absorption/rejection predicate: `let mcast4 = is_v4(&ip) && (v4[0] & 0xf0) == 0xe0; let bcast4 = v4 == [255,255,255,255];` — absorb (record-only) or reject with `"bad ip address syntax"`; never push either to `update_ip_addrs`.

### CR-05: unbind drops a live seam core — re-bind turns stale conv socket handles into panics/aliasing

**File:** `/Users/neilskinner/code/r9/port/src/devip/ipifc.rs:248-281` (`unbind`)
**Issue:** `unbind` takes the medium's `StackCore` out of the seam cell and drops it, with no guard for convs still holding `sock: Some(handle)` + `sel` naming that core (they can be owned by *other* processes — nothing ties conv lifetime to the binding conv). After a later re-bind, the fresh core's `SocketSet` reuses handle indices from zero; the orphaned conv's next op (`tcp_read_data`, `conv_close`'s `get_mut` at conv.rs:597, etc.) calls `core.sockets.get_mut::<tcp::Socket>(h)` on a stale handle → smoltcp panics (`socket_set.rs:112-117`: "handle does not refer to a valid socket" / wrong-type downcast) or, worse, silently **aliases a different live conversation's socket** (same slot index, right type). Pre-rebind the ops error cleanly ("no bound medium"); the poison window is after re-bind. The composed smoke sidesteps this by construction ("orphan-free by construction", smoke-r9-net.expect:62-85) — but the kernel defect is unrecorded in deferred-items.md and reachable by any user script: bind ether; connect a tcp conv; hold it open; unbind; re-bind; touch the old conv → panic or cross-conversation corruption.
**Fix:** On unbind, walk the conv tables (TABLES → STACK order already documented) and orphan every conv whose `sel` names the dying core *before* dropping the core: clear `sock`, set `hangup`, wake its waiters (the closeconv wake shape). Refusing `unbind` while referenced is the cheaper stopgap but diverges from the donor.

### CR-06: tcp_gc steals an in-use conv — cross-conversation aliasing + refcount corruption

**File:** `/Users/neilskinner/code/r9/port/src/devip/proto.rs:2625-2650` (`tcp_gc`), called from `/Users/neilskinner/code/r9/port/src/devip/conv.rs:368-372`
**Issue:** `conv_alloc` only calls `tcp_gc` after the scan confirms **every** slot has `inuse > 0`. `tcp_gc` then picks a conv whose socket is `Closed` — which necessarily has `inuse > 0` (open fds) — removes its socket, resets it, and hands the slot to the cloner with `inuse = 1`. The victim's open fds (qid still names conv N) now alias the cloner's fresh conversation: the victim's next read/write operates on the new owner's socket, and the victim's eventual `conv_close` decrements the *new* conv's `inuse` — at 0 it closeconv-resets a live conversation out from under its owner (socket dropped mid-connection). The donor's `tcpgc` (sys/src/9k/ip/tcp.c:3372-3402) never does this: it `localclose`s timed-out convs (hangup semantics, slot ownership unchanged) and `Fsprotoclone` still requires `inuse == 0` for reuse — the gc only makes *closed* convs eligible, it never hands an in-use slot away. Note a conv with `inuse>0` and a Closed socket is *common* (a process that read EOF and hasn't closed the fd yet), so 16-open-conv pressure reaches this reliably.
**Fix:** Never steal an `inuse > 0` slot. Map the donor faithfully: under pressure, force-hangup stale convs (`hangup = Some(...)`, wake waiters — they error out and close) and *retry the `inuse == 0` scan*; if still none, `Err("no free devices")`. The reset-and-hand-out arm must go.

### CR-07: [tracked — phase-deferred] Multi-waiter RX-IRQ wake wedges the guest

**File:** `/Users/neilskinner/code/r9/port/src/devip/stack.rs:294-304` (`note_rx_pending`), `265-279` (`tick_check`)
**Issue:** With >1 registered devip waiter, the first NIC-RX wake never reaches the blocked reader (measured on gicv2+tcg and gicv3+hvf; an earlier form panicked mid-print). Root cause unpinned by the phase. **Disposition:** already recorded blocker-class in `deferred-items.md` (2026-11-06 entry) with the composed-smoke stage-order workaround; named as a must-fix before Phase 9 tap legs / Phase 12 cpu. Logged here for completeness of this review's record — no new fix action this cycle, but it must not silently age out.
**Fix:** (Phase 9/12 scope) Root-cause candidates recorded in deferred-items.md: the multi-pid wake hook path, a woken waiter's re-probe driving both cores and consuming the ARP reply before the ping reader re-probes, or IRQ-context BusyCell contention.

## Warnings

### WR-01: udp listen/accept spawned convs silently discard writes with Ok(n)

**File:** `/Users/neilskinner/code/r9/port/src/devip/proto.rs:1248-1254` (the `sock == None` arm of `udp_write_data`)
**Issue:** `fs_new_call` (conv.rs:397-430) materializes spawned udp convs Connected with a full quad but `sock: None`. A udp server using the classic `listen`/`accept` shape then writes its reply on the accepted conv's data file → hits the "never-connected conv: fire-and-forget" arm → **returns success while dropping the datagram**. The donor sends on such convs (Connected, quad known). Silent data loss with a success status is the worst failure shape; the gate escapes it only because `udpecho` replies in headers mode on the *announcing* conv.
**Fix:** Route spawned-conv writes through the announcer's socket with per-write metadata (`UdpMetadata{ endpoint: (raddr,rport), local_address: laddr }` — the headers-mode write already demonstrates the shape), or error loudly ("udp: no socket on spawned conv") until then.

### WR-02: udp spawned convs' `dataq` is unbounded

**File:** `/Users/neilskinner/code/r9/port/src/devip/proto.rs:1436` (`dataq.push_back(pkt)`)
**Issue:** The donor's per-conv `rq` is a bounded Qmsg queue (64 KiB); r9's per-spawned-conv `dataq: VecDeque<Vec<u8>>` grows one Vec per datagram with no cap. A fast sender to an announced port whose spawned conv is never read grows kernel heap without limit — kernel memory exhaustion driven by network input. (The icmp/udp smoltcp sockets are bounded; only this devip-side queue is not.)
**Fix:** Cap dataq by bytes or datagrams (64 KiB / 8-msg parity with `UDP_BUF`/`UDP_RX_META`); drop + count past the cap (the `dropped`-counter discipline used everywhere else in this phase).

### WR-03: RFMEM failure paths leak physical pages (brk family arm unwind; rfork bss-sweep mid-failure)

**File:** `/Users/neilskinner/code/r9/aarch64/src/proc.rs:466-478` and `2400-2425`
**Issue:** Two OOM-path leaks:
  1. `rfmem_brk_set`'s `!completed_all` unwind calls `release_user_page(done_pa)` per member but **ignores the return** — the final release (count → 0, returns true = "caller must free") never `deallocate_physpage`s. Each failed growth leaks every page it completed.
  2. The rfork bss sweep maps the page into the *parent* first (line ~2406) and only then the child (~2418); on child-map failure the error return leaves the parent's table with an **untracked mapping** (the `mapping_log.push`/`share_new_user_page` never ran) — the page is leaked and the parent's exit walk (`free_user_mappings`) can't see it to free it.
**Fix:** (1) `if release_user_page(done_pa) { deallocate_physpage(...) }` in the unwind; (2) reorder the sweep to record-or-unwind the parent mapping on child-map failure (unmap + free, mirroring the per-member failure arm above it).

### WR-04: ipifc `bind` doesn't reject a conv already bound to another medium (comment claims it does)

**File:** `/Users/neilskinner/code/r9/port/src/devip/ipifc.rs:199-242`
**Issue:** The comment at 209-210 says "a re-bind of a bound conv errors the same way — the donor's `ifc->medium != nil` check", but the code only checks the *target medium's* slot (`guard.slot_mut(kind).is_some()`). One conv can `bind loopback` then `bind ether`, owning both ifcs; `for_conv`'s `[&loopback, &ether]` iteration order then silently resolves all later `add`/`unbind`/status ops to the loopback ifc — state confusion with no error.
**Fix:** Before `bind_medium`, reject when `guard.for_conv(id).is_some()` ("interface already bound"), matching the comment.

### WR-05: self-table broadcast/network entries aren't refcounted — removing one address strips shared entries

**File:** `/Users/neilskinner/code/r9/port/src/devip/ipifc.rs:891-903` (`remove_address`) vs `864-888` (`add_address`)
**Issue:** `add_address` collapses duplicates (selftab_add's exists check), so two v4 lifcs on the same subnet share the net/bcast/defmask and `255.255.255.255` entries. `remove_address` deletes them unconditionally — removing one lifc strips the broadcast self-recognition entries the *other* lifc still needs.
**Fix:** Refcount the shared entries (or recompute-and-prune: on remove, re-add any entry still implied by a surviving lifc).

### WR-06: default-route mirror does not "land at bind" (comment/code mismatch; functional gap)

**File:** `/Users/neilskinner/code/r9/port/src/devip/route.rs:135-137`
**Issue:** `mirror_default_add` returns early with "the record stands; the mirror lands at bind" — but nothing in `medium::bind_medium`/`ipifc::bind` replays recorded default routes into the new core's `Routes`. A `add 0.0.0.0 0.0.0.0 <gw>` written before the ether bind (legal ordering) never reaches smoltcp; the guest has a default route on file and no gateway on the wire.
**Fix:** Replay default routes into the core at bind time (or delete the comment and document the ordering constraint); the current state is a silent divergence.

## Info

### IN-01: dead statement in parse_v6
**File:** `/Users/neilskinner/code/r9/port/src/devip.rs:396`
**Issue:** `let _ = &mut out;` — leftover no-op after the `match tail` block.
**Fix:** Delete.

### IN-02: duplicated section header
**File:** `/Users/neilskinner/code/r9/port/src/devip.rs:1418-1424`
**Issue:** The `// Small helpers` banner appears twice back-to-back.
**Fix:** Delete one.

### IN-03: add6's `plen < 0` check is unreachable
**File:** `/Users/neilskinner/code/r9/port/src/devip/ipifc.rs:533,541-543`
**Issue:** `atoi` returns `u32` (≥0 always; `-5` parses to 0), so `if plen < 0` can never fire — the donor's "negative ipv6 prefix length" error is dead. Also `plen.min(128)` silently clamps where the donor errors (>128).
**Fix:** Drop the dead arm (or detect a leading `-` and error verbatim); consider erroring on plen > 128 instead of clamping.

### IN-04: netlog `only` with a malformed address silently clears the restriction
**File:** `/Users/neilskinner/code/r9/port/src/devip/netlog.rs:207-211`
**Issue:** `parse_ip(fields[1])` → `None` → `only = None` — a typo clears the filter with success instead of erroring.
**Fix:** `let addr = parse_ip(fields[1]).ok_or("bad ip address syntax")?;` (a literal `only 0`-style clear stays available via the zero address).

### IN-05: sys_semrelease `value + delta` can overflow i32
**File:** `/Users/neilskinner/code/r9/aarch64/src/syscall.rs:1107-1112`
**Issue:** `value >= 0` and `delta >= 0` are checked, but `value + delta` overflows i32 (wraps in release, panics in a debug build) — a wrapped-negative word then fails every future semacquire's "never-negative" entry validation.
**Fix:** `let Some(new) = value.checked_add(delta) else { return err(proc, "bad arg in system call") };`

### IN-06: listen-open's returned chan keeps the listen file's path
**File:** `/Users/neilskinner/code/r9/port/src/devip.rs:769` and `884-885`
**Issue:** `nc.path = c.path.clone()` — the chan for the *new conv's ctl* reports the listener's `/net/<proto>/<N>/listen` path (fd2path/devproc output misnames it).
**Fix:** Build the path as the new conv's ctl path (e.g. parent dir + `/<nid>/ctl`).

### IN-07: WaitPids/WaitSet full-case silently drops a registration
**File:** `/Users/neilskinner/code/r9/port/src/devip/conv.rs:128-136` (same shape in stack.rs:186-194)
**Issue:** "The full case is impossible" is asserted by comment only; a violated invariant (MAX_PROCS change, a registration leak) degrades to a silently-never-woken waiter — the CR-03/CR-07 failure shape.
**Fix:** `debug_assert!` on the full arm (host pins catch it), or count-and-report via netlog.

---

_Reviewed: 2026-11-06_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
