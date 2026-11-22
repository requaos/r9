---
phase: 07-r9-pci-ecam-virtio-net-devether
plan: 05
subsystem: testing
tags: [r9, xtask, virtio-net, devether, pcap, arp, net-gate, tdd, aarch64, slirp, filter-dump]

# Dependency graph
requires:
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: 07-01 (harvey: initrd channel + netdev stanza + pcap parser), 07-02 (PCI discovery prints), 07-04 (baked payload: net-sweep.rc FAIL markers + netblast + arp.req + dd in bootfs.paq)
provides:
  - config_virtnet.toml — the NETDRV-03/04 gate profile (dev exec+bootchain+ether, gicv2 virt, harvey:-prefixed initrd at 0x48000000)
  - BootProfile::Net + the Net stage machine (Banner -> boot-chain markers -> % prompt -> typed sweep line -> SweepRun fast-fail/NET-DONE) selected by config name, dispatched before the exec arm
  - R9_NET_SWEEP_MODE (arp|blast, default arp) validated pre-spawn
  - pcap::assert_arp_pingpong / pcap::assert_blast + the serial-side stats/ifstats extraction suite, unit-proven on synthetic fixtures (13 new tests)
  - the negative scan's stray-IRQ arm (Pitfall N10) + the Net profile's surgical fauth exemption
  - cfg(dev_ether) in the workspace check-cfg list
affects: [07-06, 07-07, 07-08, phase-8]

# Actuals (#2632) — pairs with the plan's estimate (tokens 55000, confidence: low).
actuals:
  tokens: 11533     # chars/4 over the realized r9 diff (230127f..993be67)
  tasks: 2
  commits: 3        # MEASURED: git rev-list --count 230127f..993be67 (r9); the harvey metadata docs commit rides on top

plan_head_before: 230127fe5d32a9b5f2f5360094c9b0e07143afeb   # r9 repo (code commits)
plan_head_after: 993be67456a9543e93f45b4d4a2ca5c1cae8f547    # r9 repo

# Tech tracking
tech-stack:
  added: []   # zero new crate dependencies (T-07-SC vacuous; the assertion suite is std-only xtask code)
  patterns:
    - "net-gate profile shape: a config-name-selected BootProfile whose dev list deliberately overlaps another family (exec) — precedence in the Banner dispatch is the load-bearing detail"
    - "typed-line sweep driving: the harness types one rc line at the real chain's % prompt; assertions key on marker prefixes the typed line can never contain, so cooked echo can't false-match"
    - "fast-fail gate contract: a named FAIL marker aborts the run immediately (never a deadline hang); the failure message carries the sweep's own stage line"
    - "assertion-half discipline: every pcap/serial assertion is unit-proven on synthetic fixtures BEFORE a live capture can reach it; the guest MAC is parsed from the bind line, never hardcoded"

key-files:
  created:
    - r9:aarch64/lib/config_virtnet.toml
  modified:
    - r9:xtask/src/main.rs
    - r9:xtask/src/pcap.rs
    - r9:Cargo.toml

key-decisions:
  - "07-05: the Net profile's negative scan exempts exactly the D-02 designed fauth line (`syscall 10 unimplemented`, printed twice by the real Harvey chain's boot(8) auth handshake) — any OTHER unimplemented-syscall line still fails the run; without the exemption the plan's own red path could never name the sweep (the scan would fire first)"
  - "07-05: task 1 ships the assert_arp_pingpong/assert_blast call sites in finish() against placeholder Err bodies (compile-necessity, unreachable on the red path); task 2's TDD cycle replaces the bodies — the plan's 'call sites exist / Task 2 wires the functions' made literal"
  - "07-05: the serial-side stats/ifstats extraction helpers live in pcap.rs (the plan's either-way placement rule) — the whole D-06 assertion half is one self-contained module over frames + capture, named in the module header"

patterns-established:
  - "ifstats reconciliation: used==lastused (drained ring) + nintr in [1, frames] on a queue that moved frames (vq0=RX rides stats in, vq1=TX rides stats out)"
  - "every assertion failure names the field with got/want values; absence of bind line / stats / ifstats / markers is a loud Err (T-07-07), never a silent pass"

requirements-completed: [NETDRV-03]   # copied verbatim from plan frontmatter; shared with 07-01/04/07/08 — the ready-ids gate holds it open until the last declaring plan finishes

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The virtnet profile boots the REAL Harvey boot chain under xtask (harvey: initrd channel), drives the typed sweep line at the % prompt, and fast-fails naming the sweep stage — the live RED-path proof (no #l device until 07-07): exit 1, `netsweep: FAIL bind` and `stacksmoke: depth 64 ok` in the retained log, typed-line echo visible"
    requirement: NETDRV-03
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch aarch64 --config virtnet; test $? -ne 0 && grep -q 'netsweep: FAIL' + 'stacksmoke: depth 64 ok' target/boot-test-virtnet.log → RED-PATH-MACHINERY-OK — run THREE times (task gate, tracer feedback re-run, task-2 re-run RED-PATH-STILL-HONEST)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Regression invisibility: the seven existing aarch64 virt profiles (virt virtlowecam virtexec virtexec3 virtmulti virtnotes virtpanic) stay green with the stage-machine additions and the extended negative scan (stray-IRQ arm trips none); check + clippy legs green"
    requirement: NETDRV-03
    verification:
      - kind: e2e
        ref: "for p in <seven>; do cargo xtask boot-test --arch aarch64 --config $p; done → ALL-SEVEN-GREEN, twice (task gate + tracer re-run); cargo xtask check/clippy exit 0"
        status: pass
      - kind: unit
        ref: "xtask/src/main.rs#tests — negative_scan_hits_stray_irq_line + negative_scan_net_exempts_fauth_only + the pre-existing scan suite, green"
        status: pass
    human_judgment: false
  - id: D3
    description: "The D-06/D-08 assertion suite (assert_arp_pingpong / assert_blast + stats/ifstats extraction): arp golden (N=4 data-derived), arp negatives (count mismatch / wrong tpa / extra frame vs 2N / missing bind line / missing stats), blast golden (256 pure-TX), blast negatives (sent!=pcap / oper=2 present / stats in!=0), ifstats negatives (nintr=0 moved queue / undrained ring), header-only empty parse — guest MAC always parsed from the bind line, never a constant"
    requirement: NETDRV-03
    verification:
      - kind: unit
        ref: "xtask/src/pcap.rs#pcap::tests — 13 new tests; full xtask suite 46/46 via cargo test -p xtask (exit 0); RED first: exit 101, 12 failed, RED_EVIDENCE_OK (07-05-RED-task2.json)"
        status: pass
    human_judgment: false
  - id: D4
    description: "cfg(dev_ether) declared in the workspace check-cfg list (the virtnet config's dev emission is the first appearance; the first code use lands in 07-06)"
    verification:
      - kind: other
        ref: "grep -c 'dev_ether' ../r9/Cargo.toml == 2; workspace check/clippy green with the virtnet config emitting --cfg dev_ether"
        status: pass
    human_judgment: false

# Metrics
duration: 12min
completed: 2026-11-05
status: complete
---

# Phase 07 Plan 05: The virtnet Net-Gate Profile + the pcap/Serial Assertion Suite Summary

**The complete xtask-side net gate (D-06's assertion half) exists and is honest: `cargo xtask boot-test --config virtnet` boots the real Harvey chain, types `rc /lib/net-sweep.rc <mode>` at the % prompt, and — with no #l device yet — fails NON-ZERO naming `netsweep: FAIL bind` (proven live three times), while the arp ping-pong and TX-blast pcap/stats/ifstats assertions it will run are already unit-proven against 13 synthetic fixtures; all seven pre-existing profiles stay green twice.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-11-05T00:26:16Z
- **Completed:** 2026-11-05T00:38:10Z
- **Tasks:** 2 (1 tracer + 1 TDD)
- **Files modified:** 4 (all r9-side; zero Harvey source changes — the SUMMARY/STATE/RED-evidence docs commit rides on top here)

## Accomplishments

- `aarch64/lib/config_virtnet.toml` — the NETDRV-03/04 gate profile: dev = ["exec", "bootchain", "ether"], `virt,gic-version=2`, the committed virt-gicv2.dtb, and the Harvey-baked `bootfs.paq` via the 07-01 `harvey:` channel at 0x48000000 (the image carrying 07-04's payload: fixture, sweep, netblast, dd)
- `BootProfile::Net` + the Net stage machine in xtask: selected by config name (the virtmulti/virtnotes precedent) and dispatched FIRST at the Banner stage (its dev list contains "exec" — without precedence the exec arm would mis-select); Banner → NetBoot (`paqfs...` + `stacksmoke: depth 64 ok`) → NetPrompt (`% `) → typed `rc /lib/net-sweep.rc <mode>\r` → NetSweep (fast-fail the instant a `netsweep: FAIL` line appears — the error carries the sweep's own stage line; completion gated on `netsweep: NET-DONE`)
- `R9_NET_SWEEP_MODE` (arp|blast, default arp), validated BEFORE qemu spawns — a typo'd mode is a harness bug, never a wasted TCG boot
- finish() Net arm: parses `target/boot-test-virtnet.pcap` (07-01's module) and calls the mode's assertion behind the mode switch
- `pcap::assert_arp_pingpong` / `pcap::assert_blast` + the serial-side extraction suite (guest-MAC bind line, `in:`/`out:` stats, `vq<q>:` ifstats lines) in pcap.rs — every failure names the field with got/want values; absence is a loud Err (T-07-07); the guest MAC is always parsed, never hardcoded
- The always-on negative scan gains the stray-IRQ line's lowercase shape (`unhandled irq`, Pitfall N10) and a surgical Net-profile exemption for exactly the D-02 designed fauth line; unit-pinned both ways
- `cfg(dev_ether)` declared in the workspace check-cfg list (first appearance; first code use lands in 07-06)
- Red-path live proof ×3: exit 1, failure names `netsweep: FAIL bind`; log carries the boot-chain markers, the `% ` prompt, the typed line's echo; the two designed `syscall 10 unimplemented` fauth lines correctly did NOT trip the scan
- Regression: all seven existing profiles green TWICE (task gate + tracer feedback re-run); `cargo test -p xtask` 46/46; check + clippy green

## Task Commits

Each task was committed atomically (r9 repo, branch main — the dual-repo contract):

1. **Task 1 (tracer): virtnet profile + BootProfile::Net stage machine + typed sweep + red-path live gate** — `7dc8c20` (feat)
2. **Task 2 RED: net-gate assertion suite failing behavior tests** — `1f9fc95` (test)
3. **Task 2 GREEN: pcap + serial-counter assertion suite** — `993be67` (feat)

**Plan metadata:** docs commit rides on top in the Harvey repo (SUMMARY.md + STATE.md + ROADMAP.md + the RED evidence record), per the dual-repo contract.

_TDD gate compliance (Task 2, tdd="true"):_ RED `1f9fc95` (exit 101, 12 assertion tests failed on behavior assertions — goldens on `.expect`, negatives on the placeholder Err naming no field/class; `RED_EVIDENCE_OK` per `gsd_run check tdd-red-evidence`, record at `07-05-RED-task2.json` in this phase dir) → GREEN `993be67` (46/46) → REFACTOR not needed (the GREEN bodies are the minimal helpers; nothing to clean). Tracer feedback gate (after Task 1): interactive + end-of-phase + automated-only verify → re-ran all three verify legs end-to-end post-commit (check+clippy, red-path, seven-profile regression) — TRACER-GATE-ALL-GREEN; expanded per the row-3 auto-continue.

## Files Created/Modified

- `r9:aarch64/lib/config_virtnet.toml` (NEW) — the gate profile; header names the NETDRV-03/04 purpose and the 07-01/07-04 supply chain
- `r9:xtask/src/main.rs` (+~200) — Net markers, BootStage NetBoot/NetPrompt/NetSweep, BootProfile::Net, `net_sweep_mode()`, `line_containing()`, the profile-aware `negative_scan_hit_for` (stray-IRQ arm + fauth exemption), the Banner dispatch precedence, the finish() Net arm; test-module import fix + two new scan tests
- `r9:xtask/src/pcap.rs` (+~430) — the assertion suite: `assert_arp_pingpong` / `assert_blast`, helpers (`capture_lines`, `guest_mac`, `stat_value`, `netblast_sent`, `ifstats_lines`, `reconcile_ifstats`), 13 new fixture tests
- `r9:Cargo.toml` (+4) — `cfg(dev_ether)` in the check-cfg list with the declaration-rule comment

## Decisions Made

1. **The fauth exemption (documented deviation, below).** The plan's "scan stays live" and "the red path names the sweep" are mutually inconsistent without it — the real chain prints `syscall 10 unimplemented` twice per boot by design. The exemption is surgical (that exact line only, Net profile only); every other pattern — and every other unimplemented syscall — still fails the run.
2. **Placeholder-then-TDD sequencing.** Task 1's finish() call sites compile against placeholder Err bodies (unreachable on the red path); Task 2's RED fixtures fail against them and GREEN replaces them. This is the plan's "this task's call sites exist behind the same mode switch / Task 2 wires the functions" made literal — no stub survives the plan.
3. **Helpers in pcap.rs.** The plan allowed main.rs or pcap.rs placement; one self-contained module keeps the whole assertion half unit-testable in one place. Named in the module header per the plan's rule.
4. **src_mac asserted on requests.** devether's write path rewrites the ethernet source to the NIC's MAC (RESEARCH Q7), so the assertion pins src_mac == sha == guest MAC for requests — a stronger gate that consumes the last otherwise-test-only ArpFrame field.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The stock negative scan makes the plan's red path unreachable**
- **Found during:** Task 1 pre-implementation ground-truthing (the retained packaged-smoke log `build/aarch64/smoke-r9.log` shows `syscall 10 unimplemented` at lines 782/807 — boot(8)'s fauth calls, loud-None by D-02 design)
- **Issue:** the always-on scan keys on "unimplemented"; the Net profile rides the REAL Harvey chain, which prints that line twice per boot by design. Unmodified, the scan fires during boot — the run fails as "negative scan hit" and never reaches the sweep, violating the plan's primary contract (the failure must name `netsweep: FAIL bind`).
- **Fix:** `negative_scan_hit_for(capture, net_profile)` exempts exactly the `syscall 10 unimplemented` line for the Net profile, line-wise; any other unimplemented syscall still fails. Unit-pinned (`negative_scan_net_exempts_fauth_only`).
- **Files modified:** r9:xtask/src/main.rs
- **Verification:** the live red path runs past both fauth lines to the sweep's named FAIL (×3); the scan's other arms trip as before (unit suite)
- **Committed in:** `7dc8c20`

**2. [Rule 3 - Blocking] Plan's boot-test verify commands omit `--arch aarch64`**
- **Found during:** Task 1 verification (the 07-01/07-02 class — `Arch::from` defaults to x86_64)
- **Issue:** `cargo xtask boot-test --config virtnet` as written looks for `x86_64/lib/config_virtnet.toml` (absent); every historical invocation carries the flag.
- **Fix:** ran all boot-test invocations as `cargo xtask boot-test --arch aarch64 --config <p>`.
- **Files modified:** none
- **Verification:** red path + seven profiles green, ×2 each
- **Committed in:** n/a

**3. [Rule 1 - Bug] The negative_scan_hit → negative_scan_hit_for rename broke the existing test-module import**
- **Found during:** Task 2 RED (first `cargo test -p xtask` run — E0432 unresolved import at main.rs:2117)
- **Issue:** my Task-1 refactor renamed the scan entry point; the crate's own test module still imported the old name.
- **Fix:** updated the import/call sites and extended the module with the stray-IRQ + fauth-exemption pins (passing coverage for Task 1's committed behavior).
- **Files modified:** r9:xtask/src/main.rs (test module only)
- **Verification:** full suite green (34 pre-existing passing incl. the scan tests)
- **Committed in:** `1f9fc95` (the RED commit — test-only file)

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 bug). **Impact on plan:** the fauth exemption is a plan-inconsistency resolution preserving the plan's primary contract; the rest are the familiar plan-command/self-inflicted classes. Zero scope creep — every truth in the plan's must_haves holds as measured.

## Issues Encountered

- None beyond the deviations above. The red path passed on the FIRST live run (boot → prompt → typed line → named FAIL), and GREEN passed 46/46 on the first implementation pass. The transient Task-1 dead-code warnings on pcap.rs's helpers (surfaced by removing the module's `allow(dead_code)`) were consumed by Task 2's GREEN as predicted — no warning survives the plan.

## User Setup Required

None - no external service configuration required. (R9_HARVEY_DIR is the established cross-repo channel; the verify commands set it explicitly. R9_NET_SWEEP_MODE defaults to arp; set `blast` for the TX-bulk leg once 07-07 lands the driver.)

## Next Phase Readiness

- **07-06 (devether skeleton):** `cfg(dev_ether)` is declared and emitted by the virtnet config; the negative scan already trips on any stray NIC IRQ (`unhandled irq`), so a bind-ordering bug reds the gate immediately upon arming the IRQ source.
- **07-07 (driver + live gates):** every assertion the green path will run is unit-proven today; the only remaining variable is the kernel, exactly the plan's intent. `cargo xtask boot-test --arch aarch64 --config virtnet` (arp) and `R9_NET_SWEEP_MODE=blast …` (load leg) are the two commands that turn green.
- The virtnet profile is a permanent red-path leg until then: any regression in the boot chain, the typed-line pipeline, or the marker contract fails it loudly by name.
- Requirements note: NETDRV-03 is shared by 07-01/04/05/07/08; the ready-ids gate correctly holds it open (07-07/07-08 unfinished). This plan's portion (the gate machinery) is complete.

## Self-Check: PASSED

- Files: `r9:aarch64/lib/config_virtnet.toml` tracked (`git ls-files` lists it); `r9:Cargo.toml`, `r9:xtask/src/main.rs`, `r9:xtask/src/pcap.rs` modified
- Commits (r9): `7dc8c20`, `1f9fc95`, `993be67` all FOUND in `git log`; measured count `git rev-list --count 230127f..993be67` = 3 ✓
- Acceptance re-run at summary time: `dev_ether` ×2 in Cargo.toml; `BootProfile::Net` ×3; `net-sweep.rc` ×2 + `R9_NET_SWEEP_MODE` ×4 in main.rs; `assert_arp_pingpong|assert_blast` ×16 in pcap.rs; the net_profile arm (line 1332) precedes the exec arm (line 1336); the guest-MAC string `52:54:00:12:34:56` appears ONLY inside the test fixture (pcap.rs:765, `#[cfg(test)]`)
- Gates re-run: `cargo test -p xtask` 46/46 (exit 0); check + clippy exit 0; red-path `RED-PATH-STILL-HONEST` post-GREEN; seven profiles green ×2; RED evidence `RED_EVIDENCE_OK`

---
*Phase: 07-r9-pci-ecam-virtio-net-devether*
*Completed: 2026-11-05*
