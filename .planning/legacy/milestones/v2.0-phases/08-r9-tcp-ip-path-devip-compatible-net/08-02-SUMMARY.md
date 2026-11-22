---
phase: 08-r9-tcp-ip-path-devip-compatible-net
plan: 02
subsystem: infra
tags: [smoltcp, devip, tcp, udp, waker, event-driven, seam, no_std, cargo, aarch64, r9, dhcp]

# Dependency graph
requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net (plan 08-01)
    provides: the contract doc (docs/devip-contract.md — the design authority the seam implements), the smoltcp 0.14.0 pin + committed Cargo.lock, cfg(dev_ip) predeclare
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: netif's BusyCell/INGRESS handoff discipline + EtherHooks shape + the devether_tests host-pin precedent
provides:
  - ../r9/port/src/devip.rs + devip/{stack,waker,phyadapt}.rs — the D-05 event-driven seam: BusyCell single ownership of Interface+SocketSet+Device, drive_stack() (RX_DRIVE_CAP-bounded ingress + egress + maintenance + poll_at arming), NET_DEADLINE (tick units) + STACK_DEBT + drain_debt_if_set(), DevipHooks + set_clock, the RawWaker bridge (the phase's ONE new unsafe site), LoopbackDev (Medium::Ip, MTU 16384, bounded), ScriptedDev test device
  - ../r9/port/src/devip_tests.rs — the six host pins (a)(b)(c)(d)(g)(h) + seam smoke, 7/7 green
  - AcceptPool (announce/take_connected/refill) — the no-backlog listen bridge (contract §12.3)
  - the OQ-1 verdict recorded in the contract doc §12.6: PASS-path via explicit UdpMetadata.local_address (no wire-layer shim; D-09 dhcpv4 fallback stays unwired)
  - UNSAFE-AUDIT.md reconciled: port lint-visible 21→23, SAFETY 150→152, fns 12→16
affects: [08-03 (payloads), 08-04 (the device surface consumes the seam: drive_stack at file-ops, tick_check wiring into timer.rs, drain_debt_if_set call sites, DevipHooks/clock registration in main.rs), 08-05 (tcp proto consumes AcceptPool), 08-06 (EtherDev joins the StackDev enum; the OQ-1 bridge rule governs the DHCP conv's sends), 08-07 (the DHCP gate rides the pinned exchange shape)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as estimateTokens.
actuals:
  tokens: 24500    # (85534-char r9 diff + ~12000-char this SUMMARY + ~700-char STATE/ROADMAP edits) / 4
  tasks: 2
  commits: 5       # MEASURED: 4 r9 (ae4d82a, ee65bfc, c28e6f4, 63a3ab7) + 1 Harvey metadata commit (this file + STATE/ROADMAP)
plan_head_before: b446f83e2   # Harvey repo ledger base (#3968) — 08-01's docs commit
plan_head_after: b7e4616bc    # Harvey HEAD at SUMMARY write — 0 Harvey commits yet by design (dual-repo plan: production commits landed in r9; the single Harvey metadata commit follows this write per the atomic close-out order)

# Tech tracking
tech-stack:
  added: [smoltcp `async` feature (zero-dep waker-plumbing gate — no executor/runtime; Cargo.lock closure unchanged)]
  patterns: [event-driven seam (BusyCell single ownership + three drive legs + wake-after-drop pending set), RawWaker-vtable-as-named-unsafe-exception (#[path] mounted outside the forbidden subtree), poll_at-armed one-shot deadline over the existing 1ms tick sweep + stack-debt drain, accept-pool listen bridge over a no-backlog stack, host-pinned smoltcp mapping BEFORE any guest gate]

key-files:
  created: [../r9/port/src/devip.rs, ../r9/port/src/devip/stack.rs, ../r9/port/src/devip/waker.rs, ../r9/port/src/devip/phyadapt.rs, ../r9/port/src/devip_tests.rs]
  modified: [../r9/port/src/lib.rs, ../r9/port/Cargo.toml, ../r9/UNSAFE-AUDIT.md, ../r9/docs/devip-contract.md]

key-decisions:
  - "OQ-1 RESOLVED PASS-path (pin h, empirical): the udp socket sources 0.0.0.0 on the wire when the send metadata carries local_address: Some(0.0.0.0); the naive shape is SILENTLY DROPPED (get_source_address -> None on an address-less interface) — the bridge MUST set local_address from the conv's recorded laddr; no wire-layer shim; D-09 dhcpv4 fallback stays unwired"
  - "smoltcp 'async' feature enabled (Rule 3 blocking fix): 0.14.0 gates ALL socket waker methods + internal wake() calls behind #[cfg(feature = \"async\")] — the D-05 waker bridge cannot exist without it; async = [] pulls in zero deps (no executor); the research's 'async outdated' note was self-contradictory (its D-05 section requires wakers); corrected in contract §12.2"
  - "devip/waker.rs mounted at the crate root as crate::devip_waker via #[path]: Rust's forbid(unsafe_code) PROPAGATES into submodules (probe-proven), so the plan's acceptance criteria 1+3 (devip.rs forbidden AND devip/waker.rs unsafe-bearing) are mutually exclusive under a naive module tree — the #[path] mount satisfies every literal criterion and the safety contract"
  - "The waker bridge RECORDS into a pending-wake set; drive_stack drains it after the STACK cell drops — the plan's own stack.rs header mandate (netif's wake-after-drop discipline) governs over the waker.rs action text's 'calls the hooks' literal"
  - "The seam's clock is its own registration point (set_clock) — DevipHooks keeps the devpipe/devether {current, block, wake} shape verbatim; port cannot see the arch's timer::ticks"
  - "AcceptPool::announce/take_connected/refill enter the STACK cell themselves (short acquisitions; never hold the cell across a pool op); mid-announce failure tears down the partial pool"

patterns-established:
  - "Seam host-pin shape: crate-root test module + Once-registered stub hooks + advanceable stub clock (TEST_TICKS) + SEAM_TEST_LOCK serialization + fresh stack per pin (install_test_stack resets deadline/debt/wake sets); poison-tolerant harness mutexes"
  - "Wire scripting via smoltcp's own wire codecs (Ipv4Repr/UdpRepr/TcpRepr emit + parse); sequence numbers PARSED from the seam's captured SYN, never hardcoded"
  - "RDT-style pins for event machinery: pin (c) pins deadline==poll_at in tick units; pin (g) pins the debt drain drives the missed retransmit"

requirements-completed: []  # NETIP-02/NETIP-05 are shared with pending plans (08-04/05/06 and 08-07) — the shared-ID gate (#2388) blocks completion marking until the last declaring plan finishes; no REQUIREMENTS.md change this plan

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The D-05 event-driven seam modules (stack/waker/phyadapt + devip shell): BusyCell single ownership, drive_stack with RX_DRIVE_CAP-bounded ingress, NET_DEADLINE/STACK_DEBT/debt-drain, DevipHooks + clock registration, the RawWaker bridge (one unsafe site, SAFETY-documented), LoopbackDev (Medium::Ip MTU 16384 bounded), no periodic polling anywhere"
    requirement: NETIP-02
    verification:
      - kind: unit
        ref: "cargo test -p port devip -- --nocapture → 7 passed, 0 failed (smoke + pins a/b/c/d/g/h)"
        status: pass
      - kind: other
        ref: "cargo xtask check --verbose && cargo xtask clippy --verbose → both exit 0; no devip-named warnings beyond the pre-existing duplicated-attribute class shared with devether/netif/virtio"
        status: pass
      - kind: other
        ref: "acceptance grep chain (forbid in devip.rs; drive_stack/NET_DEADLINE/STACK_DEBT/DevipHooks/set_hooks/drain_debt_if_set/RX_DRIVE_CAP in stack.rs; RawWaker in waker.rs; LoopbackDev/Medium::Ip/16384 in phyadapt.rs; no smoltcp source in tree) → all OK"
        status: pass
    human_judgment: false
  - id: D2
    description: "The six host pins settle the seam's behavior contracts incl. OQ-1: waker fires on rx/close/buffer-space exactly once per registration (a); register→wake→re-register never loses a wake, quiet drives keep the waker (b); deadline arms from poll_at at 1000 ticks and re-arms across a retransmit (c); two SYNs demux onto two DISTINCT accept-pool members + refill (d); deadline-with-no-waiters sets STACK_DEBT and the drain drives the missed retransmit (g); DHCP-shaped exchange sources 0.0.0.0 on the wire via explicit local_address with the naive shape silently dropped (h)"
    requirement: NETIP-05
    verification:
      - kind: unit
        ref: "cargo test -p port devip -- --nocapture → 7 passed, 0 failed"
        status: pass
      - kind: other
        ref: "grep chain: OQ-1 verdict line in docs/devip-contract.md §12.6 + waker entry in UNSAFE-AUDIT.md → PINS-AND-AUDIT-OK"
        status: pass
    human_judgment: false

# Metrics
duration: 26 min
completed: 2026-11-05
status: complete
---

# Phase 8 Plan 02: The Event-Driven Seam + Host Pins Summary

**The D-05 event-driven seam landed host-side BEFORE any boot gate depends on it: single-ownership BusyCell stack (Interface+SocketSet+Device adapters) driven only by RX-wake/poll_at-armed-deadline/file-op legs, the RawWaker waker bridge as the phase's ONE new unsafe site (#[path]-mounted outside the forbidden devip subtree), the fixed loopback Device — with all six behavioral pins green, including the OQ-1 verdict (PASS-path: the udp socket sources 0.0.0.0 on the wire via explicit `local_address`; no wire-layer shim needed) recorded in the contract doc and the UNSAFE-AUDIT reconciled.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-11-05T19:12:27Z
- **Completed:** 2026-11-05T19:38:55Z
- **Tasks:** 2
- **Files modified:** 9 r9-side (+1 Harvey planning file set: this SUMMARY + STATE + ROADMAP)

## Accomplishments

- **The seam (Task 1):** `devip.rs` module shell (donor-named header; device surface deferred to 08-04 by design) + `devip/stack.rs` (StackState behind a fail-fast BusyCell; `drive_stack()` = RX_DRIVE_CAP-bounded `poll_ingress_single` loop + `poll_egress` + `poll_maintenance` + `poll_at`→`NET_DEADLINE` arming in tick units; `STACK_DEBT` + `drain_debt_if_set()`; `DevipHooks{current,block,wake}` verbatim the devpipe/devether shape + a separate `set_clock` for the Instant bridge; fixed 16-pid WaitSets; pending-wake drain AFTER the cell drop) + `devip/waker.rs` (the RawWaker vtable bridge — arm-slot-before-register, re-register after EVERY wake, spurious-allowed) + `devip/phyadapt.rs` (`LoopbackDev` Medium::Ip MTU 16384 bounded 64-packet queue — never smoltcp's per-packet-allocating `phy::Loopback`; cfg(test) `ScriptedDev` RX/TX capture; the `StackDev` token-unifying enum EtherDev joins in 08-06)
- **The pins (Task 2), 7/7 green** (`cargo test -p port devip`): (h) OQ-1 DHCP-shaped exchange FIRST — naive send silently dropped (0 frames asserted), bridge-shaped send (`local_address: Some(0.0.0.0)`) leaves with src 0.0.0.0/dst 255.255.255.255/udp 68→67/payload intact, broadcast Offer accepted back with no configured address; (a) waker fires on RX data / peer close / freed send-buffer, exactly once per registration, per-direction pids; (b) three register→wake→re-register cycles lose nothing and a quiet drive keeps the waker; (c) deadline arms to exactly 1000 ticks from poll_at (smoltcp RTO floor) and re-arms strictly later after the retransmit fires; (d) two scripted SYNs land on TWO DISTINCT accept-pool members (zero still-Listen), refill restores all-Listen; (g) deadline-with-zero-waiters sets STACK_DEBT and the drain drives the missed retransmit
- **The records:** contract doc §12.6 carries the dated OQ-1 PASS-path verdict + the bridge rule (a conv with the absorbed unspecified-laddr marker passes `local_address` on every send); §12.2 carries the async-feature correction + the wake-after-drop discipline; UNSAFE-AUDIT.md gains the waker-bridge floor entry with its invariant (slot armed before registration, read at wake, never outlives the conv) and the reconciliation holds (129+23=152)
- **Regression:** full `cargo test -p port` 124/124 green (117 pre-existing + 7 new); raspi3b/raspi4b/x86_64/riscv64 stay green by construction (xtask check compiles all arch bins + port; no cfg-gated shared-code changes this plan)

## Task Commits

Each task was committed atomically (r9 repo — this plan's files are all r9-side):

1. **Task 1: seam modules (stack/waker/phyadapt) + module shell** — `ae4d82a` (feat)
2. **Task 2 RED: host pins (a)(b)(c)(d)(g)(h)** — `ee65bfc` (test) — RED evidence: `error[E0433]: could not find AcceptPool in stack` (pin (d) asserts the Task-1-absent helper; the plan's sanctioned RED shape)
3. **Task 2 GREEN: AcceptPool + pin-(g) time-discipline fix** — `c28e6f4` (feat)
4. **Task 2 docs: OQ-1 verdict + §12.2 correction + UNSAFE-AUDIT entry** — `63a3ab7` (docs)

**Plan metadata:** (Harvey repo — this SUMMARY + STATE/ROADMAP, committed after this write per the atomic close-out order)

## TDD Gate Compliance

- RED commit `ee65bfc` precedes GREEN `c28e6f4` — the RED is a compile error (the planned `AcceptPool` API absent), which is the Rust test-first RED shape when the asserted API does not exist; tdd.md's INVALID_RED categories (syntax errors / fixture crashes / unrelated assertions) do not apply — the failure names exactly the planned-behavior surface. The plan's own commit-order note sanctions this: "pins fail against a not-yet-wired seam behavior only where a pin asserts Task-1-absent behavior — most pins pass against Task 1".
- The five other pins are characterization pins of Task 1's seam — expected green at GREEN, and the plan said so.
- One REAL red-glint inside the GREEN loop: pin (g) initially failed (the pin, not the seam, had the wrong time discipline — the drain's drive must run at/after the deadline; the seam correctly refused to retransmit early). Fixed in-loop and recorded in the GREEN commit body.
- `workflow.tdd_mode` is absent from config.json, so the runtime gate (`check tdd-red-evidence`) is inactive; the plan's commit-order text governed.

## Files Created/Modified

- `../r9/port/src/devip.rs` — NEW: module shell, `#![forbid(unsafe_code)]`, donor-naming header, the module-mount explanation
- `../r9/port/src/devip/stack.rs` — NEW: THE SEAM (drive_stack/NET_DEADLINE/STACK_DEBT/DevipHooks/set_clock/WaitSets/AcceptPool + cfg(test) install/enter seams)
- `../r9/port/src/devip/waker.rs` — NEW: the RawWaker bridge (2 lint-visible unsafe blocks + 4 unsafe fns, all SAFETY/`# Safety`-documented)
- `../r9/port/src/devip/phyadapt.rs` — NEW: LoopbackDev + ScriptedDev + StackDev/StackRx/StackTx
- `../r9/port/src/devip_tests.rs` — NEW: the pin suite + harness
- `../r9/port/src/lib.rs` — `pub mod devip;` + `#[path] pub mod devip_waker;` + `mod devip_tests;` + the forbid-comment update (two floor modules → + one named bridge)
- `../r9/port/Cargo.toml` — smoltcp `async` feature + the why comment (Cargo.lock unchanged — `async = []` adds no packages)
- `../r9/UNSAFE-AUDIT.md` — reconciliation rows (21→23 / 150→152 / 12→16) + module-states row + the per-site floor entry
- `../r9/docs/devip-contract.md` — §12.6 OQ-1 verdict line + §12.2 async correction + wake-after-drop note

## Decisions Made

- **OQ-1: PASS-path** via explicit `UdpMetadata.local_address = Some(0.0.0.0)`; the naive path's silent drop is pinned as the reason the bridge MUST set it; no shim; dhcpv4 fallback stays unwired (recorded §12.6)
- **smoltcp `async` feature enabled** — zero-dep waker-plumbing gate; without it the plan's mandated waker bridge does not exist (recorded §12.2)
- **waker module mounted as `crate::devip_waker` via `#[path]`** — forbid propagates into submodules (probe-proven); the named exception cannot be a child of the forbidden parent
- **Deferred wake** — the vtable records into the pending set; `drive_stack` drains post-drop (wake-after-drop)
- **Pool ops self-enter the STACK cell**; mid-announce failure tears down the partial pool

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] smoltcp `async` feature required — the locked feature set made the plan's own waker bridge uncompilable**
- **Found during:** pre-Task-1 source verification (the 0.14.0 crate source, before writing any seam code)
- **Issue:** every `register_recv_waker`/`register_send_waker` method AND every internal `wake()` call in smoltcp 0.14.0 is `#[cfg(feature = "async")]`; the locked feature set excluded `async` and the research's "outdated" list dismissed it — while the same research's D-05 section and the plan's Task 1/pins (a)/(b) mandate the waker bridge. The contradiction survived to execution because the 08-01 compile probe never exercised the waker methods.
- **Fix:** enabled `async` in `port/Cargo.toml` with a full comment; verified `async = []` (zero dependencies, no executor/runtime; Cargo.lock byte-unchanged); recorded the correction in contract §12.2. Within the agent's explicit discretion envelope (CONTEXT: "smoltcp feature set within the locked shape ... is the agent's discretion"); every locked prohibition respected (no dhcpv4, no cubic, no slaac, no segmentation-offload).
- **Files modified:** ../r9/port/Cargo.toml, ../r9/docs/devip-contract.md
- **Verification:** pins (a)/(b) green (the bridge fires and re-registers); check/clippy exit 0; Cargo.lock unchanged
- **Committed in:** `ae4d82a` (Task 1), doc in `63a3ab7`

**2. [Rule 1 - Bug] Plan's acceptance criteria 1+3 are mutually exclusive under Rust's forbid propagation**
- **Found during:** pre-Task-1 probe (empirical rustc probe in a scratch crate)
- **Issue:** criterion 1 demands `devip.rs` carry `#![forbid(unsafe_code)]`; criterion 3 demands `devip/waker.rs` carry unsafe; Rust's `forbid` PROPAGATES into submodules (probe: the child's unsafe errors with "lint level defined here → devip.rs:1")
- **Fix:** waker.rs mounted at the crate root via `#[path = "devip/waker.rs"] pub mod devip_waker;` — the FILE path (must_haves), criterion 1 (devip.rs forbidden), criterion 3 (waker.rs unsafe-bearing, no forbid), and item 3's lib.rs-comment mandate all satisfied; the safety contract enforced structurally (negative control in the probe: unsafe in stack.rs fails)
- **Files modified:** ../r9/port/src/lib.rs, ../r9/port/src/devip.rs, ../r9/port/src/devip/waker.rs (mount shape)
- **Verification:** cargo xtask check/clippy exit 0; the probe's negative control fails as required
- **Committed in:** `ae4d82a` (Task 1)

**3. [Rule 1 - Bug] Pin (g)'s time discipline (my test, not the seam)**
- **Found during:** Task 2 GREEN loop
- **Issue:** the pin never advanced the stub clock, so the debt-draining drive ran at t=0 and (correctly) did not retransmit before the 1000-tick deadline — the seam was right, the pin was wrong
- **Fix:** `set_ticks(1000)` before the drain; added a comment explaining tick_check takes the observed tick count as a parameter while poll_maintenance reads the clock
- **Files modified:** ../r9/port/src/devip_tests.rs
- **Verification:** pin (g) green
- **Committed in:** `c28e6f4` (GREEN)

**4. [Rule 2 - Missing Critical] Test-harness failure isolation**
- **Found during:** Task 2 GREEN loop (one pin's panic poisoned SEAM_TEST_LOCK/WAKE_LOG, cascading PoisonError into its siblings and masking the root failure)
- **Fix:** poison-tolerant mutex acquisition (`unwrap_or_else(|e| e.into_inner())`) — a fresh install resets the whole seam state anyway; each pin now reports its own verdict
- **Files modified:** ../r9/port/src/devip_tests.rs
- **Committed in:** `c28e6f4` (GREEN)

**5. [Rule 1 - Bug] Compile fixes in my own new code**
- `Instant::from_millis(ticks() as i64)` (u64 has no Into<i64>); `enter_stack` returns `BusyGuard<'static, StackState>` (the cell is a static); `RawWakerVTable::new` is a SAFE const fn on the pinned nightly (dropped the unnecessary unsafe block — the audit entry records the corrected site count: 2 blocks + 4 fns, not 3+4); test-only scaffolding (`enter_stack`, SLOT_RX/TX, STUB_PID_TX) moved out of Task 1 to keep its clippy leg clean of dead-code warnings; two import typos (`ChecksumCapabilities` is phy::, `UdpMetadata` is socket::udp::); unused `mut` on SocketSet::get_mut bindings; TcpLeg trimmed to asserted fields
- **Committed in:** `ae4d82a`, `c28e6f4`

---

**Total deviations:** 5 auto-fixed (1 blocking feature-gate, 3 plan-text/my-code bugs, 1 missing test-harness robustness)
**Impact on plan:** the two substantive ones (async feature, #[path] mount) RESOLVE internal contradictions in the plan+research package in the direction the LOCKED decisions (D-05's waker bridge; the unsafe-posture contract) require; both are recorded in the r9-side records (contract doc §12.2/§12.6, UNSAFE-AUDIT.md, lib.rs comment). No scope creep; no locked decision changed.

## Issues Encountered

- **`git stash` hazard (self-inflicted, recovered):** a defensive `git stash -q` during an audit-recount step swept up the pre-existing uncommitted `aarch64/src/exec.rs` modification plus my uncommitted docs edits. Recovered intact via `git stash show --name-only` verification + `git stash pop` of exactly `stash@{0}` (an older pre-existing stash@{1} left untouched). No data lost; exec.rs remains uncommitted and out of every commit as instructed. The verification flow no longer uses stash.
- **Pre-existing r9 warnings** (devenv_test unused-mut, devroot_rawimg unused-import, virtio/devether/netif clippy style notes) observed again — unchanged by this plan; the 08-01 deferred-items.md record stands. My modules add ONE clippy note of the same pre-existing class: `duplicated attribute` on devip.rs's inner forbid (identical to devether.rs:79/netif.rs:71/virtio.rs:54 — the acceptance criterion requires the inner attribute, the lib.rs convention the outer one).
- **gsd_run/gsd-tools shim unavailable** in this environment — STATE.md/ROADMAP.md were hand-edited to the same fields the verbs maintain (position, metric row, decisions, progress count; roadmap plan flip + count), and `requirements.mark-complete` was correctly a no-op anyway: NETIP-02/NETIP-05 are shared with pending plans (shared-ID gate #2388).

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **08-04 unblocked:** the seam exports everything the device-surface plan needs — `drive_stack()` for the three legs, `tick_check(now)` for the timer.rs sweep arm, `drain_debt_if_set()` for the syscall-arm drain, `register_net_waiter` + `DevipHooks`/`set_clock` for main.rs registration (beside devether's), `StackCore::new_loopback` for `bind loopback`, and the waker bridge (`WakeSlot` + `waker_for`) for the conv read/write paths. The OQ-1 bridge rule (conv laddr → `local_address`) is recorded for 08-07's DHCP gate.
- **08-05 unblocked:** `AcceptPool` (announce/take_connected/refill, ACCEPT_POOL_DEPTH=128, 2048/2048 buffers, hop-limit 255) is the announced-conv machinery.
- **08-06 note:** EtherDev joins the `StackDev` enum; `StackState` grows the second core with the §12.4 address-class demux.
- r9's pre-existing `aarch64/src/exec.rs` modification was left untouched and out of every commit, as instructed (one stash scare, recovered intact — see Issues).
- No blockers.

## Self-Check: PASSED

- `../r9/port/src/devip.rs` exists, carries `#![forbid(unsafe_code)]` + the donor-naming header — VERIFIED
- `../r9/port/src/devip/stack.rs` contains `drive_stack`, `NET_DEADLINE`, `STACK_DEBT`, `DevipHooks`, `set_hooks`, `drain_debt_if_set`, `RX_DRIVE_CAP` — VERIFIED (grep chain)
- `../r9/port/src/devip/waker.rs` contains the RawWaker vtable; 2 unsafe blocks each immediately SAFETY-preceded (clippy deny lint green); no forbid attribute — VERIFIED
- `../r9/port/src/devip/phyadapt.rs` contains `LoopbackDev` with `Medium::Ip` + MTU 16384; `phy::Loopback` appears only in the why-not comment — VERIFIED
- `../r9/port/src/lib.rs` names `pub mod devip;` and `mod devip_tests;` (+ the `#[path]` devip_waker mount) — VERIFIED
- No periodic timer/poll thread anywhere in the seam — VERIFIED (grep; the deadline is armed data, the sweep is the pre-existing tick)
- r9 commits `ae4d82a`, `ee65bfc`, `c28e6f4`, `63a3ab7` exist on r9 main (`git log --oneline 669c561..HEAD`) — FOUND
- `cargo test -p port devip` = 7 passed / 0 failed; full port suite 124/124; `cargo xtask check`/`clippy` exit 0 — VERIFIED at execution time
- `../r9/docs/devip-contract.md` §12.6 carries the dated OQ-1 PASS-path verdict; `../r9/UNSAFE-AUDIT.md` reconciles (129+23=152) — VERIFIED
- No smoltcp source vendored/patched/forked anywhere in the tree; Cargo.lock unchanged by the feature flag — VERIFIED
- r9 `aarch64/src/exec.rs` pre-existing modification untouched and uncommitted — VERIFIED (git status)

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Completed: 2026-11-05*
