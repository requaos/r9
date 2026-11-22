---
phase: 16-host-net-modes-networking-exit-gate
plan: 05
subsystem: net
tags: [netip-08, soak-gate, tap, host-echo, poll, expect-driver, flake-app, slirp-refusal, netsoak, d-11, d-12, d-13]

requires:
  - phase: 16-host-net-modes-networking-exit-gate
    provides: "16-02's tap_ seam keys + /lib/netip-tap.rc fixture (the driven sweep) + the bake coverage; 16-03's net-doctor (the preflight); 16-04's R9_NETDEV_EMITTER/R9_NET_DOCTOR exports + the pin advance (d1c4849) carrying netdev-args"
provides:
  - "the committed host echo peer (nix/host-echo.c -> flake hostEchoX64Pkg bin/host-echo): poll()-concurrent, gate-owned markers, SIGTERM teardown — the NETIP-08 far end (D-12)"
  - "the soak gate app boot-r9-net-soak-x64 (nix/soak-r9-net-x64.sh): --net/R9_NET parse + the hard slirp refusal before any bake/boot, doctor preflight FIRST, seam-parsed peer bind, emitter tap argv + retained pcap, tier build -> net bake -> iso compose -> expect driver"
  - "the expect driver build/x86_64/netsoak-r9.expect: committed gate data (baseline 2048 / burst 8 / burst rounds 512), one typed fixture launch, output-only marker stages, the counting post-pass (tcpblast: done == 9, zero netip: FAIL, ARP + venue label), wedge-loud per-stage + overall timeouts with the capture tail"
affects: [16-07 tap docs (the gate names the remediation path), 16-08 (RUNS the gate: green-twice, the kernel-fix loop, evidence + r9 SHA close-out)]

actuals:
  tokens: 11227    # chars/4 over git diff 6d0e0d374..c787fafea (estimate was 60000 — wiring and data, not invention)
  tasks: 2
  commits: 2       # MEASURED: git rev-list --count 6d0e0d374..HEAD
plan_head_before: 6d0e0d374c7007d8fe191f874b55a9d6ee7d50a9
plan_head_after: c787fafea7ca07aa419fd2a638ae322c722e0f6d

tech-stack:
  added: []        # no new registry packages; host-echo is committed C built by stdenv cc (T-16-SC)
  patterns:
    - "poll()-concurrent host peer as a gate-owned child: listener + every accepted fd serviced in one loop (the anti-Pitfall-7 shape the in-guest tcpecho and the r9 HostEcho precedent deliberately do NOT have)"
    - "venue-honesty refusal at parse time: the soak app rejects slirp (including the hermetic default) with 'slirp evidence not accepted' BEFORE the doctor preflight — the doctor aborts any current wrong-allow host, so a doctor-first order would make the refusal unreachable on exactly the hosts that need it; both still precede every bake/boot/listener"
    - "expect driver argv-as-split-data: the spawn expands {*}$env(R9_NETDEV_ARGV) (the emitter's one-line output) — no netdev literal in the driver (D-05 carried driver-side)"
    - "marker-COUNT post-pass over the whole capture: count_occurrences on 'tcpblast: done' asserting exactly 1 + burst_count — never an order assertion among burst legs; teeth proven by an 8-count negative control"
    - "fake-qemu marker-stream self-test: the driver is provable end-to-end without a tap host (spawn/argv-split/stages/post-passes all exercise under a scripted marker emitter)"

key-files:
  created:
    - nix/host-echo.c
    - nix/soak-r9-net-x64.sh
    - build/x86_64/netsoak-r9.expect
  modified:
    - flake.nix

key-decisions:
  - "The slirp refusal sits at parse time, ahead of the doctor preflight: the doctor FAILs on this host today (bridge-conf-allow), and the refusal must be reachable on exactly such hosts — the doctor still gates before any bake, boot, or listener (T-16-14 ordering kept)"
  - "The driver self-test (fake qemu emitting the full marker stream) is a permanent proof layer for gate drivers — it caught a real expect defect (the -timeout-before-block silent no-match) before any boot, and the 8-count negative control proves the count post-pass is not vacuous"
  - "The evidence scratch dir (pcap + hostecho.log) is RETAINED past the run — a dedicated mktemp dir, deliberately NOT the ISO scratch TMPD the trap removes; 16-08's consolidation copies it into the phase evidence dir"
  - "HARVEY_NETADDR_SEAM exported from the flake (the committed seam's store path) with the doctor's $HARVEY-relative shape as fallback — one seam host-side too, parsed exactly the doctor's way (grep-eval of tap_gwv4/blastport)"
  - "procps joined the soak app's runtimeInputs so the trap's pkill sweep is hermetic (the sibling gates resolve pkill from the host PATH)"

patterns-established:
  - "Gate drivers can be self-tested against a scripted marker emitter before any venue exists — stage ordering, argv splitting, and post-pass counting all prove out without br0"
  - "Host-tool behavior is a measurement surface like the guest's: this host's coreutils head emits NOTHING when timeout-killed mid-count, and expect's pty stays canonical for children that don't set raw mode themselves (real qemu does) — client harnesses must read exact byte counts, not over-ask and kill"

requirements-completed: [NETIP-08, HNET-03]

coverage:
  - id: D1
    description: "The committed host echo peer — poll()-concurrent echo server (binds the given address, never the wildcard), gate-owned markers, dies with its parent; proven by the three local behavior tests"
    requirement: NETIP-08
    verification:
      - kind: command
        ref: "Task 1 local test 1: cc -O2 -Wall clean; /tmp/host-echo16 127.0.0.1 17777 -> 'hostecho: listening 127.0.0.1:17777' + dial/send 'ping16'/read -> byte-exact 'ping16' back (reply=[ping16], LISTENING-MARKER-OK)"
        status: pass
      - kind: command
        ref: "Task 1 local tests 2+3: 8 held-open concurrent dials -> all 8 distinct replies (burst1..burst8 each exactly once) in 2s wall (a serialized peer queues 16s+, capped by the 10s timeouts -> count < 8); kill parent -> re-dial refused (CONCURRENCY-OK + NO-ORPHAN-OK, server-gone-OK)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The armed soak gate: flake app boot-r9-net-soak-x64 (refusal + doctor preflight + seam + emitter + peer lifecycle + bake/compose pipeline), the expect driver (committed gate data, marker stages, counting post-pass, wedge-loud timeouts), flake packaging + registration in the existing x64 gate block"
    requirement: NETIP-08
    verification:
      - kind: command
        ref: "R9_NET=slirp nix run .#boot-r9-net-soak-x64 -> exit 1 + 'slirp evidence not accepted' present (/tmp/soakref16.log)"
        status: pass
      - kind: command
        ref: "shape asserts: r9x64soak- tag, R9_NETDEV_ARGV split in the driver, committed 2048, tcpblast: done counting post-pass, hostEchoX64Pkg/host-echo in flake.nix -> SHAPE-OK; nix eval .#apps.x86_64-linux.boot-r9-net-soak-x64.program -> APP-EVAL-OK"
        status: pass
      - kind: command
        ref: "tap path through the app: R9_NET=tap -> exit 1 with the doctor's real findings inline (WARN masquerade + FAIL bridge-conf-allow, this host's allow virbr0 state) + remediation pointer + evidence-scratch retention note (/tmp/soaktap16.log)"
        status: pass
      - kind: command
        ref: "pinned emitter probe: r9-netdev-args --mode tap --pcap <p> -> '-netdev tap,id=n0,br=br0,helper=/run/wrappers/bin/qemu-bridge-helper -device virtio-net-pci-non-transitional,netdev=n0 -object filter-dump,id=f0,netdev=n0,file=<p>' (the exact argv the driver splits)"
        status: pass
      - kind: command
        ref: "driver self-test vs a fake marker-stream qemu: exit 0, all stages in order, post-pass 'tcpblast: done x 9 (1 baseline + 8 burst)', zero netip: FAIL, ARP + venue label asserted; negative control (8 done lines) -> exit 1 'COUNT FAILED — got 8, want exactly 9'"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-11-11
status: complete
---

# Phase 16 Plan 05: the NETIP-08 soak machinery (host echo peer + soak app + expect driver) Summary

**The NETIP-08 machinery half landed and is proven ARMED: a committed poll()-concurrent host echo peer (the anti-Pitfall-7 far end, flake-built, dies with the gate), and a dedicated tap-leg soak app whose pipeline runs slirp-refusal -> doctor preflight -> seam-parsed peer bind -> emitter tap argv with retained pcap -> bake -> compose -> an expect driver carrying the committed gate data (baseline 2048 + burst 8x512), output-only fixture staging, and a whole-capture counting post-pass (exactly 9 `tcpblast: done`, zero FAIL, ARP + L2-TRUTH venue label) — refusal, abort path, eval, shape, and the driver itself all proven live/on-this-host; the RUNS (green-twice + kernel-fix loop + evidence close-out) are 16-08's.**

## Performance

- **Duration:** 20 min
- **Started:** 2026-11-11T13:32:25Z
- **Completed:** 2026-11-11T13:52:45Z
- **Tasks:** 2/2
- **Files modified:** 4 (3 created, 1 modified)

## Accomplishments
- `nix/host-echo.c` + flake `hostEchoX64Pkg`: the tcpecho semantics as a host POSIX C program made CONCURRENT (one poll() loop for the listener and every accepted fd — the r9 HostEcho precedent's sequential accept deliberately not copied); binds the literal address it is given (the seam's tap_gwv4 — never the wildcard, T-16-11), loud perror exits, SIGTERM/SIGINT stop-flag teardown, byte-accounting markers
- `nix/soak-r9-net-x64.sh` = flake app `boot-r9-net-soak-x64`: the boot script's exact --net/R9_NET parse extended with the hard slirp refusal (the default included), the 16-03 doctor preflight before any work, the seam parsed the doctor's way for the peer bind, the pinned emitter's tap argv (with the retained filter-dump pcap) exported as R9_NETDEV_ARGV, the peer as a trap-torn-down child waited for its listening marker, then the unchanged 14-09 build/bake/compose contract and the expect driver — with a NEW r9x64soak- run tag so the four x64 gates' survivor sweeps never cross-kill
- `build/x86_64/netsoak-r9.expect`: the smoke-r9-net discipline carried (header warnings incl. the multi-waiter RX-IRQ record the burst leg WILL load, CRLF-tolerant substring markers, panic + netip: FAIL fast-fail arms, fresh-capture hygiene, process-group teardown) + the soak additions: committed gate data as named variables, the single typed launch `rc /lib/netip-tap.rc 2048 8`, the counting post-pass, and per-stage + OVERALL wall-clock budgets that fail loud with a capture tail (a stall IS the finding)
- Proven without needing a remediated host: the driver green end-to-end against a scripted marker emitter (including the argv split via `{*}` expansion) and the count post-pass red under an 8-count doctored stream; the real pinned emitter's tap+pcap output captured token-for-token; the app's tap leg aborts at the doctor with this host's real findings inline — exactly the state 16-07/16-08 pick up from

## Task Commits

Each task was committed atomically:

1. **Task 1: the committed host echo peer (concurrent, gate-owned lifecycle)** — `9e83e6220` (feat)
2. **Task 2: the soak app + expect driver (doctor preflight, slirp refusal, tap boot, tiered soak)** — `c787fafea` (feat)

**Plan metadata:** this SUMMARY + STATE/ROADMAP/REQUIREMENTS updates.

## Files Created/Modified
- `nix/host-echo.c` — the concurrent host echo peer (created; Task 1)
- `nix/soak-r9-net-x64.sh` — the soak gate pipeline script (created; Task 2)
- `build/x86_64/netsoak-r9.expect` — the expect driver (created; Task 2)
- `flake.nix` — hostEchoX64Pkg + netSoakR9X64Pkg (exports R9_KERNEL_X64_PKG/R9_ISO_BUILDER/R9_NETDEV_EMITTER/R9_NET_DOCTOR/HOST_ECHO_X64/HARVEY_NETADDR_SEAM; runtimeInputs expect/qemu/plan9port/bash/procps + the linux util set) + `boot-r9-net-soak-x64` registered in the EXISTING x64 optionalAttrs block (no new gate; its list already carried netdev-args)

## Decisions Made

See key-decisions in the frontmatter. The load-bearing ones:

1. **Refusal-before-doctor ordering** — the refusal is parse-time; the doctor preflight follows (still before any contract spend, bake, boot, or listener). On a wrong-allow host (this one today) a doctor-first order would make the refusal string unreachable — and the venue-honesty control must be reachable exactly there. The doctor still gates everything that touches the host (T-16-14 kept).
2. **The fake-qemu driver self-test** — a marker-stream emitter proves the whole driver (spawn argv split, stage order, both fast-fail arms' absence, all four post-passes) without br0; it caught the expect `-timeout`-before-block defect pre-commit and carries forward as the cheap regression harness for driver edits.
3. **Retained evidence scratch** — the pcap + peer log live in their own mktemp dir the trap does NOT remove (the ISO scratch TMPD is removed); the header documents that 16-08's consolidation owns the copy into the phase evidence dir.
4. **burst_rounds as data twin** — the fixture's burst leg hardcodes its own 512 (16-02's committed fixture); the driver's `burst_rounds` names the same number at the gate-data block so the count contract reads from one place.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The plan's verify-2 client shape is unsatisfiable on this host**
- **Found during:** Task 1 (verify 2)
- **Issue:** two measured host behaviors break the literal command: (a) the bare `wait` also waits on the backgrounded SERVER child, so the one-liner hangs; (b) this host's coreutils `head` emits NOTHING when timeout-killed mid-count (measured: `head -c 9` with 8 bytes available, killed at 3s -> empty file, exit 124) — so `head -c 7` after 6-byte payloads can never capture the replies it demonstrably received
- **Fix:** wait on the eight dial PIDs explicitly; read EXACT counts (`head -c 6`), then hold each connection open 2s (`sleep 2` before `exec 3<&-`) — the hold preserves the plan's discrimination (8 simultaneously-open connections: a sequential peer queues ~2s each = 16s+ and the 10s timeouts starve the tail dials -> count < 8; the poll peer served all 8 in 2s wall)
- **Files modified:** none (verify mechanics only; the plan's intent — anti-serialization + orphan check — fully preserved)
- **Verification:** 8/8 distinct replies each exactly once, 2s wall; kill -> re-dial refused (CONCURRENCY-OK + NO-ORPHAN-OK)
- **Committed in:** n/a (verify mechanics)

**2. [Rule 1 - Bug] `expect -timeout N {block}` silently breaks braced-block matching**
- **Found during:** Task 2 (driver self-test — pre-commit)
- **Issue:** with `-timeout` before the single-brace pattern-action block, expect returned immediately without matching anything (no stage ok, no timeout, no error — bisected against a plain inline block which matched fine)
- **Fix:** the stage proc sets the GLOBAL `timeout` variable per call (the sibling gates' mechanism — smoke-r9-net.expect sets `set timeout 600` and passes no option); noted in the driver's comment so the next driver does not rediscover it
- **Files modified:** build/x86_64/netsoak-r9.expect
- **Verification:** self-test green end-to-end (all stages + post-passes) after the fix
- **Committed in:** c787fafea (part of the task commit — the defect and fix both landed inside Task 2's window)

**3. [Rule 2 - Missing critical] procps added to the soak app's runtimeInputs**
- **Found during:** Task 2 (flake packaging)
- **Issue:** the trap's qemu-survivor sweep needs `pkill` (procps); the sibling gate apps resolve it from the host PATH — a host without procps would silently skip the sweep (`|| true`), violating the dies-with-the-gate/never-a-survivor posture on exactly the hosts nobody watches
- **Fix:** `pkgs.procps` joined the always-on runtimeInputs list
- **Files modified:** flake.nix
- **Verification:** app eval green; the acceptance grep finds both the pkill pattern and the peer kill in the trap
- **Committed in:** c787fafea (part of the task commit)

**4. [Rule 3 - Blocking] A dedicated retained scratch dir for the pcap + peer log**
- **Found during:** Task 2 (script structure)
- **Issue:** the plan's `--pcap <tmpdir>/netsoak.pcap` inside the smoke pipeline's TMPD would die in the trap's `rm -rf "$TMPD"` — "captures retained per the evidence convention" could not survive the run
- **Fix:** a second mktemp dir (EVIDD) for netsoak.pcap + hostecho.log, deliberately NOT removed by the trap; the trap prints its path and the header names 16-08's consolidation as the consumer
- **Files modified:** nix/soak-r9-net-x64.sh
- **Verification:** the tap-abort run's trap output names the retained dir (/tmp/soaktap16.log)
- **Committed in:** c787fafea (part of the task commit)

---

**Total deviations:** 4 auto-fixed (2 Rule 3, 1 Rule 1, 1 Rule 2)
**Impact on plan:** None structural — every named artifact, marker string, committed datum, and acceptance criterion landed exactly as written; the fixes are host-behavior adaptations around the plan's edges (verify mechanics, teardown hermeticity, evidence retention)

## Verification Notes

- **Driver self-test harness** (not a repo artifact): `/tmp/fakeqemu16.sh` emits the full marker stream (banner/paqfs/version/prompt, the fixture markers, 1+8 `tcpblast: done` interleaved with round lines, the stats tail) with its stdio set RAW (real qemu does this itself for the -nographic serial; without it the spawned pty stays canonical and a \r-terminated typed line never completes a child's read — the first self-test hung exactly there). The negative control is the same script with the burst loop cut to 7.
- **The slirp-refusal verify doubles as an app-build proof**: `nix run` built harvey-boot-r9-net-soak-x64 — whose extraHeader interpolates `${hostEchoX64Pkg}/bin/host-echo` — so hostEchoX64Pkg building under nix is proven by the same run.
- **Requirement checkboxes stay pending by the shared-ID gate**: NETIP-08/HNET-03 are declared by 16-02 (done), 16-05 (this plan), 16-07, and 16-08 — they may not read Complete until every declarer has a SUMMARY; `requirements.ready-ids` returned the empty ready subset for this plan.
- The panic fast-fail truncation quirk (15-08 deferred item) is carried as-is in the driver; the fail_tail capture print mitigates the localization gap host-side.

## Issues Encountered

None at the repo level. Two host-behavior findings are recorded in patterns-established (killed-head-emits-nothing; pty canonical mode for non-raw children) because 16-08's run loop will build client-side harnesses on this host too. One process-hygiene lesson re-learned during debugging: an unbracketed `pkill -f` pattern matches the invoking shell's own argv — the gate scripts' `[q]emu` bracket idiom exists for exactly this and is now carried in the soak trap verbatim.

## User Setup Required

None for this plan's machinery. (16-08's RUNS need the host-side tap remediation the doctor prints — the exact `allow br0` bridge.conf line etc.; 16-07 documents it, the human applies it.)

## Next Phase Readiness
- The gate is ARMED end-to-end: once the host carries the doctor's tap remediations, `nix run .#boot-r9-net-soak-x64` (or `-- --net=tap`) runs the whole pipeline — nothing is left to define in 16-08 except executing it, the expected kernel-fix loop (the multi-waiter RX-IRQ defect loads under the burst leg — first runs are expected to have fix headroom, not be green on attempt one), and the green-twice evidence + r9 SHA close-out
- 16-07's docs cite the app/doctor surface that now exists verbatim; 16-06's CI legs are untouched (hosted CI keeps the slirp legs by D-13)
- The parallel-run constraint widens once more: the soak's hostfwd-free tap leg does not collide with slirp boots' 17010/5670, but two simultaneous SOAKS would collide on the peer's blastport bind and one br0 leg

## Self-Check: PASSED

- Files: nix/host-echo.c, nix/soak-r9-net-x64.sh, build/x86_64/netsoak-r9.expect present; flake.nix carries hostEchoX64Pkg + netSoakR9X64Pkg + the app registration (FOUND)
- Commits: 9e83e6220, c787fafea both ancestors of HEAD; commits measured from the plan ledger = 2 (no uncommitted code changes)

---
*Phase: 16-host-net-modes-networking-exit-gate*
*Completed: 2026-11-11*
