---
phase: 08-r9-tcp-ip-path-devip-compatible-net
fixed_at: 2026-11-06T13:44:39Z
review_path: .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW.md
iteration: 1
findings_in_scope: 13
fixed: 12
skipped: 1
status: partial
---

# Phase 08: Code Review Fix Report

**Fixed at:** 2026-11-06T13:44:39Z
**Source review:** .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 13 (fix_scope = critical_warning: CR-01..07, WR-01..06; Info findings excluded)
- Fixed: 12 (two flagged "requires human verification" — logic fixes in host-unreachable paths)
- Skipped: 1 (CR-07 — tracked blocker-class deferred to Phase 9/12 per the phase's own record)

All fixes landed in the **r9** repo (`/Users/neilskinner/code/r9`, main: `19f61ed..0a4ea82`), 23 atomic commits (12 fix + 11 test). Harvey-side: only the flake.lock pin advance (`97522aebe`, flake.lock alone). No Harvey source files were modified by this run; r9's pre-existing uncommitted `aarch64/src/exec.rs` modification and stash entry were never touched.

## Fixed Issues

### CR-01: conv_close drops the FIN — `close()` then `remove()` before any poll

**Files modified:** `port/src/devip/conv.rs`, `port/src/devip/stack.rs` (fix); `port/src/devip_tests.rs` (pin)
**Commits:** `bd4db36` (fix), `7300113` (test)
**Applied fix:** the lingering-socket design from the finding's preferred arm. The closed conv's tcp socket stays IN the core's SocketSet on a new per-core `linger` list (close() sequences the FIN; the poll legs and the poll_at-armed deadline keep driving it — the donor's tcb surviving closeconv); `drive_stack` reaps the entry once the socket reaches `State::Closed`. The list lives INSIDE `StackCore` (not `StackState`), so it dies with its core — a lingered handle can never name a successor core's SocketSet (kills the cross-core staleness class by construction). Pin `tcp_conv_close_lingers_the_socket_until_closed`: RED-proven (pre-fix the last close emitted no FIN — a wire-level probe failed); GREEN: FinWait1 after conv_close, TimeWait after the peer's FIN-ACK, reaped after the stub-clock jumps smoltcp's CLOSE_DELAY (10 s).

### CR-02: Permanent smoltcp send errors wedge the writing proc forever (icmp/udp/icmpv6)

**Files modified:** `port/src/devip/proto.rs` (fix); `port/src/devip_tests.rs` (pins)
**Commits:** `904af8f` (fix), `b9d3977` (test)
**Applied fix:** exactly the diagnosis. Each of the three write loops now guards the oversize arm at the top (`payload > the socket's TX payload buffer capacity` → the donor's Etoobig string "read or write too large" — smoltcp's enqueue capacity arm returns Full forever, so this was a permanent wedge with no wake source) and matches the SendError: `Unaddressable` → immediate `Err("devip: unaddressable endpoint")`; `BufferFull` (now provably transient) keeps the register → drop → block shape. Pins (udp oversize + unaddressable, icmp oversize + unaddressable, icmpv6 oversize): RED-proven vividly — pre-fix the udp oversize pin WEDGED (test-runner's "running for over 60 seconds" is the review's defect made visible); GREEN: immediate errors, and BLOCK_COUNT proves no write ever parked.

### CR-03: semacquire loses wakeups — a woken waiter that loses the unit race re-blocks unregistered

**Files modified:** `aarch64/src/syscall.rs`, `aarch64/src/proc.rs` (fix); `aarch64/src/proc.rs` (pins)
**Commits:** `75970f0` (fix), `b48e809` (test)
**Status:** fixed: requires human verification
**Applied fix:** `proc::sem_register_waiter(pa, proc.pid)` moved INSIDE the loop — register → check → block per iteration (the wakeup-loss-free shape used everywhere else in the phase; single CPU, IRQ-masked syscall context, so no releaser can run between the check and sched()). `sem_wake` stays destructive (consume-on-pick); the registration is simply restored before every re-block. The three doc comments that claimed this shape already (sys_semacquire, sem_register_waiter, sem_wake) were re-checked against the code and corrected. Pins (`proc::sem_tests`, 4): the registry contract the fix relies on — consume-in-order, idempotent re-register, PA keying, re-registration found by the next release. Honest limits: the syscall LOOP itself is not host-reachable (needs Proc + user memory); the fix is a race/lifecycle logic change — flagged for human verification; the guest RFMEM contention gates cover it end-to-end.

### CR-04: `ipifc add` of a v4-multicast or limited-broadcast address panics the kernel (P-F violation)

**Files modified:** `port/src/devip/ipifc.rs` (fix); `port/src/devip_tests.rs` (pin)
**Commits:** `ac6370c` (fix), `2017360` (test)
**Applied fix:** extended the absorption predicate with `mcast4` (224/4) and `bcast4` (255.255.255.255) — both absorb record-only (the lifc + devip-visible tables stand; `update_ip_addrs` never sees them), choosing the finding's absorb arm over reject for consistency with the existing unspecified/mcast6 shape. The netlog note now names what was absorbed. Pin `ipifc_add_v4_multicast_and_broadcast_absorb_without_panic`: RED-proven — pre-fix the pin panics inside smoltcp's check_ip_addrs (`iface/interface/mod.rs:899`, "IP address 224.0.0.1 is not unicast"), the guest-kernel panic path made visible; GREEN: both adds absorb, the iface address list stays empty, both records remove cleanly.

### CR-05: unbind drops a live seam core — re-bind turns stale conv socket handles into panics/aliasing

**Files modified:** `port/src/devip/ipifc.rs` (fix); `port/src/devip_tests.rs` (pin)
**Commits:** `0f602d8` (fix), `42b1745` (test)
**Applied fix:** the finding's primary arm. `unbind` now walks all five conv tables before the core drops and orphans every conv whose `sel` names the dying core: `sock`/`pool` references cleared (the sockets die with the core's drop), the hangup marker set ("devip: medium unbound" — the read/write paths error "i/o on hungup channel" / the udp sibling), the err queue carries the reason, and blocked waiters are woken after every guard drops (the closeconv wake shape). Pin `ipifc_unbind_orphans_convs_on_the_dying_core`: RED-proven (pre-fix the stale reference survives unbind); GREEN: both a tcp and a udp conv orphaned at unbind, and — the poison window — after a RE-bind (fresh SocketSet, indices reused from zero) touching the orphans errors cleanly, never panics or aliases.

### CR-06: tcp_gc steals an in-use conv — cross-conversation aliasing + refcount corruption

**Files modified:** `port/src/devip/proto.rs`, `port/src/devip/conv.rs` (fix); `port/src/devip_tests.rs` (pin)
**Commits:** `79517b1` (fix), `27d8ec1` (test)
**Applied fix:** the donor mapping, verbatim in spirit. `tcp_gc` no longer resets/hands out the victim: it finds tcp convs whose socket is fully Closed (the era's "tcb is done" mapping) and force-HANGS-UP them — the donor's `localclose(c, Etimedout)`: hangup marker + errq + wake, slot ownership unchanged. `conv_alloc` retries the `inuse == 0` scan (new `first_free` helper) after a successful gc and returns Enodev ("no free devices") when still full; the gc's wakes now run AFTER the TABLES guard drops (the wake-after-drop discipline the old code also violated). Pin `tcp_gc_hangs_up_stale_convs_but_never_steals_an_in_use_slot`: RED-proven (pre-fix conv_alloc stole slot 0 and returned Ok); GREEN: full table → Enodev, victims keep state/quad with the hangup recorded, and a slot freed by its owner's close is the one the next clone gets.

### WR-01: udp listen/accept spawned convs silently discard writes with Ok(n)

**Files modified:** `port/src/devip/proto.rs` (fix); `port/src/devip_tests.rs` (pin + one pin-hygiene repair)
**Commits:** `3dec2e9` (fix), `764d0d1` (test)
**Applied fix:** the finding's primary arm (not the loud-error stopgap): a spawned conv's data write rides the ANNOUNCER's socket with the conv's recorded quad as per-write `UdpMetadata` (the headers-mode write's shape; the announced lport holds via the socket's bind). Guard against the recycled-slot corner: the announcer must still be Announced with the same lport, else the write errors "udp: no socket on spawned conv" — never a wrong-port send on an unrelated socket. Pin `udp_spawned_conv_writes_ride_the_announcers_socket`: RED-proven (pre-fix Ok(4) with no TX frame); GREEN: the reply leaves with src GUEST:7 → PEER:5000, payload verbatim; the dead-announcer arm errors loudly. The test commit also repairs a pre-existing pin-hygiene bug the new pin exposed: `udp_fsnewcall_spawns_on_new_remote_quad`'s `let _ = nc;` dropped the listen-popped Chan WITHOUT closing the conv, leaking a spawned conv across pins (a later pin announcing the same port+quad matched the stale conv in the drain's per-quad demux instead of spawning fresh).

### WR-02: udp spawned convs' `dataq` is unbounded

**Files modified:** `port/src/devip/proto.rs` (fix); `port/src/devip_tests.rs` (pin)
**Commits:** `8cfc44f` (fix), `106e939` (test)
**Applied fix:** the queue is capped at the socket buffer's own parity — UDP_RX_META (8) datagrams AND UDP_BUF (64 KiB) bytes, covering both the many-small and few-huge flood shapes (the donor's per-conv rq is a 64 KiB Qmsg queue). Past the cap the datagram drops and counts into InErrors (the boundary-drop discipline), and no wake fires (nothing was queued). Pin `udp_spawned_conv_dataq_is_capped`: RED-proven (pre-fix the queue grew to 13 under the flood); GREEN: the count cap holds at 8, the byte cap drops the second 48 KiB datagram, and InErrors moves with the drops.

### WR-03: RFMEM failure paths leak physical pages (brk family arm unwind; rfork bss-sweep mid-failure)

**Files modified:** `aarch64/src/proc.rs`
**Commit:** `9035bb2` (fix)
**Status:** fixed: requires human verification
**Applied fix:** both arms, mirroring the module's own per-member failure hygiene. (1) `rfmem_brk_set`'s `!completed_all` unwind now honors `release_user_page`'s verdict — the last release (count → 0) deallocates; previously every completed page of a failed growth leaked. (2) The rfork bss sweep: a parent-map failure now frees the fresh page; a child-map failure unmaps the parent's UNTRACKED mapping (the record push never ran for it) and frees the page — previously the page leaked AND the parent's exit walk couldn't see it. No new pin: the leak arms are OOM-only unsafe page-table paths (not host-pinnable); the refcount-verdict contract the fix consumes IS already host-pinned (`rfmem_tests::born_shared_frees_at_the_last_release` et al.). Flagged for human verification as a logic fix. Verified: `cargo check -p aarch64` (real target) clean; aarch64 host pins 18/18.

### WR-04: ipifc `bind` doesn't reject a conv already bound to another medium (comment claims it does)

**Files modified:** `port/src/devip/ipifc.rs` (fix); `port/src/devip_tests.rs` (pin)
**Commits:** `c6040b7` (fix), `4791dc0` (test)
**Applied fix:** exactly the diagnosis — `bind` now rejects when `guard.for_conv(id).is_some()` ("interface already bound", the same string as the per-slot arm), realizing the donor's `ifc->medium != nil` check the comment claimed. The check runs BEFORE `bind_medium` (never allocates/half-binds on the reject path). Pin `ipifc_bind_rejects_a_conv_already_bound_to_another_medium`: RED-proven (pre-fix the cross-medium re-bind proceeded to the ether medium's bind action, erroring "devether: no ether0" on the host — never the donor's string); GREEN: same-medium and cross-medium re-binds both error correctly, and bind-after-unbind still works.

### WR-05: self-table broadcast/network entries aren't refcounted — removing one address strips shared entries

**Files modified:** `port/src/devip.rs`, `port/src/devip/ipifc.rs` (fix); `port/src/devip_tests.rs` (pin)
**Commits:** `4ee61c8` (fix), `85adba5` (test)
**Applied fix:** the finding's refcount arm, placed inside the self-table itself (`SelfEntry.refs`) — self-contained (selftab_add/remove are called only from ipifc's add/remove_address). Add bumps the implier count (the donor's exists check becomes the share count); remove decrements and drops only at zero; the null address's accept-all side effect now clears only with its last implier. Pin `ipifc_remove_keeps_shared_selftab_entries_for_surviving_lifcs`: RED-proven (pre-fix the first remove stripped the subnet broadcast while the second lifc implied it); GREEN: removed address's unicast entry goes, the shared family survives, the last implier's remove strips it. The pin also records the family's exact entry forms (donor addselfcache arithmetic: network entries keep the v4-mapped prefix; broadcast entries carry 0xff prefix bytes).

### WR-06: default-route mirror does not "land at bind" (comment/code mismatch; functional gap)

**Files modified:** `port/src/devip/route.rs`, `port/src/devip/ipifc.rs` (fix); `port/src/devip_tests.rs` (pin)
**Commits:** `f70d98b` (fix), `0a4ea82` (test)
**Applied fix:** the finding's replay arm (not the comment deletion): new `route::replay_default_routes()` re-mirrors the route forest's recorded default routes into the fresh core at bind time (the mirror's own target rule — ether when bound, else loopback; smoltcp replaces on re-add, so the replay is idempotent). Called from `ipifc::bind` with the IFC guard held (the established IFC → ROUTES → STACK nesting). A replay failure never fails the bind (the record stands; the add-time path stays the loud one) but lands in netlog (D-07). Pin `iproute_default_recorded_before_bind_lands_at_bind`: RED-proven (pre-bind record never reached smoltcp's Routes); GREEN: the bind delivers it.

## Skipped Issues

### CR-07: [tracked — phase-deferred] Multi-waiter RX-IRQ wake wedges the guest

**File:** `/Users/neilskinner/code/r9/port/src/devip/stack.rs:294-304` (`note_rx_pending`), `265-279` (`tick_check`)
**Reason:** tracked blocker-class deferred to Phase 9/12 per the phase's own record — `deferred-items.md` (2026-11-06 entry, root-cause candidates recorded) and WINDOWS.md #50 name it as a must-fix before Phase 9 tap legs / Phase 12 cpu; the review itself logs it "for completeness of this review's record — no new fix action this cycle". No minimal provably-correct fix was apparent (the root cause is unpinned), and the phase precedent forbids a speculative redesign. It must not silently age out.
**Original issue:** With >1 registered devip waiter, the first NIC-RX wake never reaches the blocked reader (measured on gicv2+tcg and gicv3+hvf; an earlier form panicked mid-print). Root cause unpinned by the phase.

## Verification

**Where the gates ran (#2825):** all verification ran in the MAIN r9 checkout (`/Users/neilskinner/code/r9`, darwin-aarch64 host, `nix develop` devshell) — `workflow.use_worktrees=false` in `.planning/config.json`, so no isolated worktree was created; the numbers are reproducible from the tree the reader is looking at. Guest gates needed `R9_HARVEY_DIR=/Users/neilskinner/code/harvey`.

**Per-fix:** every fix read the actual source first, applied with targeted edits, and re-verified. Every r9-side fix except WR-03 carries a host pin in `port/src/devip_tests.rs` (the phase's pin home), each RED-proven against the pre-fix tree (revert → fail/wedge/panic → restore → pass) before the fix commit landed:
- CR-01 RED: no FIN on the wire after the last close (behavioral probe failed).
- CR-02 RED: the oversized udp write WEDGED the pin (the review's defect, live).
- CR-04 RED: pin panicked inside smoltcp's check_ip_addrs (the kernel-panic path).
- CR-05 RED: the stale socket reference survived unbind (no orphan sweep).
- CR-06 RED: conv_alloc stole the in-use slot and returned Ok.
- WR-01 RED: Ok(4) with no TX frame (silent drop).
- WR-02 RED: the queue grew past the cap (13 > 8).
- WR-04 RED: the cross-medium re-bind ran the medium's bind action (wrong error string).
- WR-05 RED: the first remove stripped the shared broadcast entry.
- WR-06 RED: the pre-bind default record never mirrored.
- CR-03: the syscall loop is not host-reachable — the registry contract it relies on is pinned (4 new `sem_tests` pins); the loop change is flagged for human verification.
- WR-03: OOM-only unsafe arms, not host-pinnable; the consumed refcount contract was already pinned; flagged for human verification.

**Full regression battery (all at final r9 HEAD `0a4ea82`):**
- `cargo test -p port`: **164/164 pass** (baseline 152 + 12 new devip pins).
- `cargo test -p xtask`: **74/74 pass**.
- `cargo test -p aarch64 --target aarch64-apple-darwin`: **18/18 pass** (baseline 14 + 4 new sem pins).
- `cargo xtask check`: green.
- Clippy: `cargo clippy -p port` — **0 errors** (27 pre-existing warnings, unchanged). `cargo xtask clippy --arch aarch64` — RED, but **identically red at the pre-fix baseline `19f61ed`** (A/B-verified via a scratch detached worktree): the same 4 pre-existing `undocumented_unsafe_blocks` errors (proc.rs:427/458/1685/2394 at baseline → same blocks, line-shifted +8 by the WR-03 insert) and 27 pre-existing warnings. A pre-existing host/toolchain divergence, not introduced by these fixes; parity holds exactly. Left alone as out-of-scope for this review's findings.
- Guest gates (`cargo xtask boot-test --arch aarch64 --config virtip`, `R9_NETIP_SWEEP` per mode): **loopback PASS** (NETIP-03), **tcp PASS**, **static PASS** (NETIP-04, pcap-proven: gratuitous ARP + 3 ICMP echo request/replies, NOT-L2-TRUTH label intact).

**Operational notes (transparent record):**
- Early in the run, two `cargo test` invocations hung/emptied immediately after checkout-or-`cp` file swaps without an intervening `touch`; after the files stabilized, the identical suites passed 10+ consecutive times (parallel and single-threaded) and the final battery + guest gates are green at the committed HEAD. Attributed to a stale/mixed incremental test binary across the rapid revert-restore cycles (the hanging test's behavior matched the PRE-fix binary exactly); not observed since. Recorded here so a future intermittent-wedge sighting can be compared.
- The WR-01 pin exposed a pre-existing cross-pin conv leak in `udp_fsnewcall_spawns_on_new_remote_quad` (`let _ = nc;` dropped the Chan without closing the conv); repaired in the WR-01 test commit.

**Push-before-pin (the phase's locked discipline):** r9 main pushed (`19f61ed..0a4ea82` → `origin/main`, github.com:requaos/r9). Harvey's flake.lock r9 pin advanced as its OWN isolated commit `97522aebe` naming flake.lock ALONE — verified locked rev `0a4ea828932b1ac32110da28ef87b04130f966c4` == live HEAD == origin/main, owner=requaos. The composed gate re-run at the new pin: `nix run .#boot-r9-net-smoke` — **ALL STAGES PASS** (boot, ping 127.0.0.1, udp echo, tcp/7 echo, ipconfig static, ipconfig dhcp — both NOT-L2-TRUTH labeled — completion marker, pid-warning-absent, nic-bind).

---

_Fixed: 2026-11-06T13:44:39Z_
_Fixer: the agent (gsd-code-fixer)_
_Iteration: 1_
