---
phase: 19-cpu-service-drawterm-rio-desktop
plan: 02
subsystem: kernel
tags: [smoltcp, devip, tcp, accept-pool, nagle, delayed-ack, tick_check, q35, lapic-timer, r9, x86_64]

# Dependency graph
requires:
  - phase: 18-in-guest-p9sk1-auth-domain
    provides: the 18-07 tcp_socket donor-shape fix (Nagle off, ACK immediate) this plan mirrors at the accept-pool site, and the CR-03/WR-04 reopener record naming the fix site and pin legs
  - phase: 19-cpu-service-drawterm-rio-desktop/19-01
    provides: the q35cpu tracer boot gate re-greened here (the tracer still holds after the TCP/timer changes)
provides:
  - r9 AcceptPool members carry the donor TCP shape for life (Nagle off, no delayed ACK, hop limit 255 unchanged) — announced connections no longer park >2048-byte bulk writes (the CR-03 class closed)
  - the WR-04 pin legs (pool-member options pin, accepted-conv identity pin, >POOL_SOCK_BUF bulk-drain leg) with RED evidence classified RED_EVIDENCE_OK
  - the q35 tick()'s cfg(dev_ip) tick_check call (D-10 — x86_64's timer-mediated TCP progress under park, the aarch64 virt precedent verbatim)
  - the port-level park-timer pin (a REAL parked socket waiter wakes through tick_check alone)
  - a devip test-harness seam fix (fresh_scripted_stack clears the one-shot ON_BLOCK action)
affects: [19-03 (#! exec), 19-04 (WSTAT/FWSTAT/REMOVE + devssl ride this kernel tree), 19-05 (pin advance over these kernel bytes — f610473+df9ada8), 19-07 (seventh contract), 19-08 (the -G interop gate and long-lived encrypted sessions — D-10's named consumer)]

# Actuals (#2632) — pairs with the plan's estimate (45000 tokens, low confidence).
# chars/4 over the realized r9 diff (16,066 chars across stack.rs/proto.rs/devip_tests.rs/timer.rs).
actuals:
  tokens: 4016
  tasks: 3
  commits: 2   # MEASURED: git rev-list --count e491ed5..df9ada8 (r9 only; the harvey docs commit follows separately)

# Tech tracking
tech-stack:
  added: []   # zero packages — smoltcp unchanged at the 0.14.0 pin (T-19-SC honored)
  patterns:
    - "the fix-site mirror: a socket-option fix lands as the donor shape at the SPAWN site, not at accept time — pool members are connections for life, so options set where the member is created cover the connection end-to-end"
    - "compile-shape pin by symbol contrast: a cfg-gated call is proven present by the dev_ip profile exporting the symbol (nm) and absent from a non-dev_ip profile — the cfg key shown load-bearing without a host-testable tick()"

key-files:
  created: []   # evidence artifacts beside this SUMMARY: evidence/19-02-t1-red-evidence.json + 19-02-t1-red-raw-libtest.json
  modified:
    - ../r9/port/src/devip/stack.rs
    - ../r9/port/src/devip/proto.rs
    - ../r9/port/src/devip_tests.rs
    - ../r9/x86_64/src/platform/q35/timer.rs

key-decisions:
  - "The WR-04 bulk-write leg's writer is the CLIENT (connect side): the ACK that drains a writer's tx buffer comes from the PEER, so the pool member's delayed ACK gates a client bulk write into the announced connection (the secstore-PUT/exportfs direction); a server-side writer drains on the client's already-fixed immediate ACKs and passes pre-fix — the first draft had the wrong side and exposed exactly that asymmetry"
  - "Task 2's park-timer pin is green-on-arrival by the plan's own branch ('if the leg already exists on the port level, pin that it runs on the x86_64 host target too'): the 08-04 machinery is pinned at port level since Phase 8 — the NEW named pin re-proofs the wake explicitly; the wiring itself is proven by the nm symbol contrast + the q35shell boot (the live dev_ip venue), with 19-08's long-lived session as D-10's named live consumer"
  - "Panic-safe ConvGuard on popped listen convs in the new pins: a failed pin must not leak the accepted conv's table slot into the gc pin's full-table arithmetic (found live: the RED panic cascaded a false gc failure before the guard)"

patterns-established:
  - "Park bound as RED discriminator: a >2048-byte write that would park under stock defaults is converted into the pin's own planned failure by a one-shot ON_BLOCK panic — the guest stall made loud and deterministic (the 18-07 two_write discipline, extended to the bulk-drain class)"

requirements-completed: []   # CPU-04 stays open: co-declared by 19-08 (the end-to-end remoteside flow) — the shared-ID gate blocks it; this plan delivered the TCP/timer substrate CPU-04 rides

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "CR-03 closed at its named fix site: spawn_listener mirrors tcp_socket's set_nagle_enabled(false)+set_ack_delay(None) between hop limit and listen (nothing else changes); pool members — the accepted connection's socket for life — carry the donor TCP shape end-to-end"
    requirement: CPU-04
    verification:
      - kind: unit
        ref: "r9 port lib (cargo test -p port --lib): tcp_pool_members_carry_the_donor_tcp_shape + tcp_accepted_connection_keeps_the_donor_shape_for_life — green in the 151-pass suite, twice"
        status: pass
    human_judgment: false
  - id: D2
    description: "WR-04 pin legs with RED-proven evidence: the pool-member options pin, the accepted-conv identity pin, and the >POOL_SOCK_BUF (3000-byte) bulk-drain round-trip over a real announced-accepted loopback pair (client writer, harness park bound armed)"
    requirement: CPU-04
    verification:
      - kind: unit
        ref: "cargo test -p port --lib: tcp_announced_connection_bulk_write_round_trips — green; RED evidence evidence/19-02-t1-red-evidence.json classified RED_EVIDENCE_OK (17 tests, 3 planned failures pre-fix, 14 siblings green incl. the 18-07 connect-path pin as negative control)"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-10 wiring: the q35 tick()'s dev_exec block carries the #[cfg(dev_ip)] port::devip::stack::tick_check(prev + 1) call with the aarch64-verbatim invariant comment (no poll/alloc in IRQ context; woken SYSCALL contexts drive the stack); aarch64 untouched"
    verification:
      - kind: unit
        ref: "cargo test -p port --lib: parked_socket_waiter_wakes_at_the_poll_at_deadline_via_tick_check (the park-timer pin — tick_check alone wakes a real parked waiter, no syscall-context poll)"
        status: pass
      - kind: other
        ref: "compile-shape nm contrast: q35shell (dev_ip) kernel exports tick_check (T @ ffffffff8027e420); q35cpu (no dev_ip) contains zero tick_check symbols; q35shell boot-test PASS (the live dev_ip venue booting with the call in its tick path)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Battery + boot legs green twice + push: check/clippy/port-lib battery clean twice (151 port tests); q35net KVM x2 + TCG x1; q35auth, q35cpu (the 19-01 tracer: cpurc RAN), q35shell green; r9 pushed origin/main e491ed5..df9ada8 (PUSHED-CLEAN)"
    verification:
      - kind: integration
        ref: "boot-test verdicts: q35net PASS x2 KVM + PASS TCG (X64-NET-DONE in each serial log), q35auth PASS, q35cpu PASS (cpurc: RAN), q35shell PASS; git log origin/main..HEAD empty after push"
        status: pass
    human_judgment: false

# Metrics
duration: 20 min
completed: 2026-11-14T23:35:21Z
status: complete
commits_note: "r9 plan_head_before=e491ed5 plan_head_after=df9ada8 (both == origin/main)"
---

# Phase 19 Plan 02: CR-03 AcceptPool TCP fix + q35 tick_check wiring Summary

**r9's announce-path sockets now carry the donor TCP shape for life (Nagle off, no 10ms delayed ACK at spawn_listener — >2048-byte bulk writes into announced connections complete on their own drives, RED-proven then green) and the q35 tick drives devip's deadline sweep under dev_ip (the aarch64 precedent verbatim, closing x86_64's never-fired park timers)**

## Performance

- **Duration:** 20 min
- **Started:** 2026-11-14T23:14:42Z
- **Completed:** 2026-11-14T23:35:21Z
- **Tasks:** 3
- **Files modified:** 4 (all in r9; 2 evidence artifacts beside this SUMMARY)

## Accomplishments

- **Task 1 (TDD, RED->GREEN):** the CR-03 fix — `spawn_listener` mirrors `tcp_socket`'s two `set_*` calls exactly (between hop limit and listen; buffers/hop limit/listen order untouched). Pool members are the accepted connection's socket FOR LIFE (`take_connected` hands the handle to the listen-file conv), so the member shape IS every announced connection's shape. Three WR-04 pin legs landed as ONE class commit with the RED evidence captured first (`evidence/19-02-t1-red-evidence.json`, `RED_EVIDENCE_OK`: 17 tests, exactly the 3 planned failures — pool-member Nagle assert, accepted-conv Nagle assert, the bulk-write park bound — 14 siblings green including the 18-07 connect-path two-write pin as the negative control). Post-fix: 151 port tests green.
- **Task 2 (TDD):** the D-10 wiring — q35 `tick()`'s `dev_exec` block gains `#[cfg(dev_ip)] port::devip::stack::tick_check(prev + 1)` between the timeout sweep and preemption, the aarch64 `platform/virt/timer.rs` precedent verbatim including the invariant comment (poll NEVER in IRQ/tick context; the venue's 100 Hz slice named honestly — the sweep is deadline-driven, not rate-driven). New port-level park-timer pin: a REAL parked connect waiter wakes through `tick_check` alone (no syscall-context poll anywhere), then the WOKEN context drives to its verdict. Compile-shape proven by symbol contrast: the dev_ip kernel exports `tick_check`, the non-dev_ip kernel contains zero references. No aarch64 file touched (`git diff --stat` clean of aarch64 paths).
- **Task 3:** full battery green twice (`cargo xtask check` + `clippy` + `test` — 151 port tests); boot legs green: q35net KVM x2 + TCG x1, q35auth, q35cpu (the 19-01 tracer still holds — `cpurc: RAN` asserted), q35shell (the live dev_ip venue booting with the call in its tick path); r9 pushed `e491ed5..df9ada8` (PUSHED-CLEAN).

## Task Commits

Each task was committed atomically (cross-repo: all code lands in r9 `main`, push-before-pin; the harvey flake.lock advance is 19-05's, never here):

1. **Task 1: AcceptPool fix + WR-04 pin legs (one class commit per the plan's Pitfall-5-style prohibition — RED evidence captured before, pins and fix together)** - r9 `f610473` (fix)
2. **Task 2: q35 tick_check wiring + park-timer pin** - r9 `df9ada8` (feat)
3. **Task 3: battery + boot legs + push** - no separate commit (verification-only task; the two commits above are the pushed bytes)

**Plan metadata:** (this SUMMARY + STATE/ROADMAP commit, below)

## Files Created/Modified

- `../r9/port/src/devip/stack.rs` - spawn_listener: the two set_* calls + the 19-02 doc comment (the fix site)
- `../r9/port/src/devip/proto.rs` - tcp_socket's 18-07 doc comment tail records the pool-site mirror
- `../r9/port/src/devip_tests.rs` - the three WR-04 pin legs + the park-timer pin + the ON_BLOCK seam fix in fresh_scripted_stack
- `../r9/x86_64/src/platform/q35/timer.rs` - the cfg(dev_ip) tick_check call + invariant comment
- `evidence/19-02-t1-red-evidence.json` + `evidence/19-02-t1-red-raw-libtest.json` - the RED evidence record (classified) + the raw libtest JSON stream

## Decisions Made

- **The WR-04 leg's writer is the client** (see key-decisions): the draining ACK comes from the peer, so the pool member's delayed ACK gates CLIENT bulk writes into the announced connection — the exportfs/secstore direction 18-07's record names. A server-side writer passes even pre-fix (the client's 18-07-fixed socket ACKs immediately), which the first draft's unexpected green exposed; the leg was flipped before the final RED capture.
- **Task 2's pin rides the plan's already-exists branch**: the 08-04 deadline machinery has been port-level-pinned since Phase 8; the new pin names the park-wake explicitly (the etimedout leg uses the wake incidentally), and the wiring is proven by the nm symbol contrast + the q35shell boot, with 19-08's long-lived encrypted session as D-10's named live consumer.
- **CPU-04 is NOT marked complete here**: co-declared by 19-08 (the end-to-end `cpu -R` remoteside flow); the shared-ID gate correctly blocks it. This plan delivered the TCP/timer substrate that flow rides.

## TDD Gate Compliance

Task 1 is `tdd="true"` and RED-proven per the format contract: the libtest JSON of the pre-fix run (raw stream preserved) was mechanically converted to TAP and classified — `RED_EVIDENCE_OK`, reason `target_test_failed` (17 discovered / 14 passed / exactly the 3 planned failures). Semantic assessment in the evidence record: each options pin failed its own `nagle_enabled` assertion through the real announce/SYN/listen-pop paths, and the bulk-write leg's park bound fired inside `tcp_write_data`'s send arm with the CR-03 mechanism named. Per the plan's own action text, the pins and fix landed as ONE class commit (`f610473`) — the Pitfall-5-style prohibition on splitting `test()` from the fix when they are one behavior (the 17-04 precedent); no separate `test(19-02)` commit exists by design. Task 2 is `tdd="true"` in the plan's already-exists branch: its named pin re-proofs existing port machinery (green on arrival — the pin is a re-proof, not a new-behavior RED; the genuinely-new behavior, the tick wiring, is compile-shape + boot proven because `tick()` is hardware-gated and not host-testable). Plan-level `workflow.tdd_mode` is false (config).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Stale one-shot ON_BLOCK action leaked across pins**
- **Found during:** Task 1 (the first RED capture)
- **Issue:** A pin whose park bound never fires (its flow completes without parking — e.g. the green two_write pin) leaves its panic action armed in the global ON_BLOCK static; the NEXT pin whose real path blocks through `stub_block` inherits the stale panic — a scheduling-dependent false failure. Latent since 18-07; exposed when the new pins changed the SEAM_TEST_LOCK queue order (two_write locked before full_exchange in the capture run and full_exchange falsely failed with two_write's message).
- **Fix:** `fresh_scripted_stack` clears the one-shot (`*ON_BLOCK.lock()... = None`) — the function's own stated contract ("each gets a clean seam") extended to the one-shot; one line + comment.
- **Files modified:** ../r9/port/src/devip_tests.rs
- **Verification:** the final RED capture is clean of collateral failures (14 siblings green); the full 151-test suite green twice after.
- **Committed in:** r9 f610473 (part of Task 1's class commit)

**2. [Rule 3 - Blocking] Task-3 verify venue adaptations (three)**
- **Found during:** Task 3
- **Issue:** (a) `cargo xtask boot-test` requires `R9_HARVEY_DIR` (no sibling default since 05-CONTEXT D-10) — the plan's bare command aborts at the precondition; (b) the plan's `grep -q 'ALL STAGES PASS' target/boot-test-q35net.log` names the harvey-side expect-gate vocabulary — the r9 boot-test writes its `boot-test: PASS` verdict to stdout and stage markers (e.g. `X64-NET-DONE`) to the serial log; (c) `R9_ACCEL=tcg` alone leaves the TOML `cpu=host` — qemu refuses `-cpu host` without KVM and the leg dies at 0 captured bytes.
- **Fix:** (a) `export R9_HARVEY_DIR=/home/req/code/harvey` on every leg; (b) the green assert greps the driver's `^boot-test: PASS` verdict line plus the profile's terminal stage marker in the serial log (q35net: `X64-NET-DONE`; q35cpu: `cpurc: RAN`); (c) `R9_ACCEL=tcg R9_CPU=qemu64` — the documented no-silent-downgrade env pair (the q35auth config comment's own idiom).
- **Files modified:** none (verification venue only)
- **Verification:** all legs green under the adapted commands (Q35NET-GREEN-1/2, Q35NET-TCG-GREEN, Q35AUTH-GREEN, Q35CPU-GREEN, Q35SHELL-GREEN).
- **Committed in:** (venue adaptation — no code change)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking-venue)
**Impact on plan:** The harness fix is load-bearing for every future devip pin run; the venue adaptations changed only how the greens were asserted, not what was asserted. No scope creep.

## Issues Encountered

- The first WR-04 draft (server-side writer) unexpectedly passed pre-fix: the ACK that drains a writer's tx buffer comes from the PEER, and the client peer already ACKs immediately (18-07). Diagnosed, the writer flipped to the client (the pool member's delayed ACK is the hold — the exportfs/secstore direction), and the final RED capture failed for exactly the planned reason. The asymmetry is recorded in key-decisions.
- During RED capture, the planned accept-shape panic cascaded a false `tcp_gc...` failure (the popped conv's table slot leaked past the failing assert). Fixed by making the new pins close their popped convs through panic-safe ConvGuards — a failed pin now leaks nothing into its siblings' arithmetic.

## Known Stubs

None - no stubs, placeholders, or unwired data paths were introduced. (devssl/#D, WSTAT/FWSTAT/REMOVE, and `#!` exec are 19-03/19-04's planned work, not stubs of this plan.)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The CR-03/WR-04 record is closed at its named fix site with its named pin legs, and the D-10 reopener is closed on x86_64 — the TCP/timer substrate the cpu/exportfs/drawterm legs ride is in place and pushed (r9 at `df9ada8` == origin/main)
- 19-03/19-04 build on this tree; 19-05 owns the flake.lock pin advance over these kernel bytes (never advanced here); 19-08's long-lived encrypted session is D-10's named live proof
- Kernel bytes changed: the boot-iso FOD hash re-stales structurally (the standing 13-06/14-07 discipline) — 19-05's re-wire-then-pin pass covers it
- The boot-iso FOD/witness machinery is untouched; q35net/q35auth/q35cpu/q35shell gates all green at the new bytes (twice where required)

## Self-Check: PASSED

- Modified files exist on disk: ../r9/port/src/devip/stack.rs, proto.rs, devip_tests.rs, ../r9/x86_64/src/platform/q35/timer.rs FOUND
- Task commits exist and are pushed: r9 f610473, df9ada8 (== origin/main, PUSHED-CLEAN) FOUND
- RED evidence exists and classified: evidence/19-02-t1-red-evidence.json (RED_EVIDENCE_OK) FOUND
- All Task 1/2/3 acceptance criteria re-verified (see coverage block): spawn_listener's two calls before listen; the pins present and passing (151 twice); no aarch64 paths in the diff; battery + four profiles green (q35net x2 KVM + x1 TCG); r9 pushed

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-14*
