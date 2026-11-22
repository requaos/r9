---
phase: 14-harvey-amd64-userspace-on-r9
plan: 09
subsystem: net-gates
tags: [net, ipconfig, dhcp, slirp, loopback, tcp-echo, expect, flake, x86_64, r9, xexec-05, venue-honesty, evidence]
duration: 14 min
completed: 2026-11-09
requires:
  - "14-08-complete harvey b50a0e1ca + the r9 pin e417660 (the tier machinery + compose pipeline; ipconfig built via the ip container's amd64 filter arm)"
provides:
  - "the x64 /net gate stack (XEXEC-05, D-07): build/x86_64/root-tree/lib/netaddr.rc (the D-11 seam re-keyed — lo_/slirp_/blastport) + netip-{loopback,static,dhcp,tcp}.rc (the fixture discipline verbatim: sourced-not-typed, test -e guards, per-step netip: FAIL + exit, ok markers + NETIP-DONE, no sleeps)"
  - "the net-binary bake: subset-build's net section (ping/udpecho/dd link-only from the ip container + cmd, the 14-07 ._cp lesson; netblast/tcpblast/tcpecho as the 6c C-helper re-keys into out/) + mkimage section 10 — bootfs-net.paq (chain tree + /amd64/bin/ip/{ipconfig,ping,udpecho} + the C helpers + the utility set grep/sed/dd/wc/sha1sum/test/bind + the /lib fixtures; KERNDATE-deterministic ffdccd6c, 31-line named-input manifest)"
  - "the composed net gate: build/x86_64/smoke-r9-net.expect + nix/smoke-r9-net-x64.sh + the boot-r9-net-smoke-x64 flake app — GREEN TWICE CONSECUTIVELY (45 stage-oks each), both captures retained in the phase evidence dir"
  - "the degenerate-input edges as NAMED stages: netgate: DUPBIND-REJECTED (the equal-address ipconfig vs the bound ether — devip's own 'interface already bound' asserted on the wire) + netgate: NOADDR-REJECTED (the empty ping target) — visible rejections, never silent merges or wedges"
  - "the D-11 discipline asserted MECHANICALLY at bake time (T-14-17): mkimage greps root-tree/lib for 10.0.2. literals outside netaddr.rc and a stray FAILS the bake"
  - "14-EVIDENCE.md — the phase bundle: all five XEXEC truths with their gate commands, twice-green records, capture paths, the flagged-assumption outcomes, the witness-parity record, the discretion/OQ resolutions, and the venue-honesty record"
affects:
  - "build/x86_64/{root-tree/lib/*,netblast.c,tcpblast.c,tcpecho.c,smoke-r9-net.expect,mkimage.sh,subset-build.sh}, nix/smoke-r9-net-x64.sh, flake.nix, .gitignore, .planning/.../14-EVIDENCE.md + evidence/"
  - "Phase 16 (NETIP-08's tap legs re-key the seam + the fixtures; the multi-waiter RX-wake record names the path), Phase 15 (the tier sweep's ipconfig SKIP check flips once ipconfig joins the swept set)"
tech-stack:
  added: []
  patterns:
    - "the D-11 seam discipline becomes a BAKE-TIME assert, not a review-time rule: the literal-address grep lives in mkimage's net section, so a future fixture edit that hardcodes a venue address reds the bake (the gate can fail)"
    - "degenerate-input backstop edges are named gate stages with a marker prefix DISTINCT from the fixtures' fast-fail arm (netgate: vs netip: FAIL) — the deliberate rejections are assertable evidence without tripping the abort regex"
    - "expect stage order must follow WIRE ARRIVAL, not logical grouping: the dup-bind leg's sysfatal prints between the typed echo and the ||-arm marker, so the error stage sits between the two marker stages"
key-files:
  created:
    - build/x86_64/root-tree/lib/netaddr.rc
    - build/x86_64/root-tree/lib/netip-loopback.rc
    - build/x86_64/root-tree/lib/netip-static.rc
    - build/x86_64/root-tree/lib/netip-dhcp.rc
    - build/x86_64/root-tree/lib/netip-tcp.rc
    - build/x86_64/netblast.c
    - build/x86_64/tcpblast.c
    - build/x86_64/tcpecho.c
    - build/x86_64/smoke-r9-net.expect
    - nix/smoke-r9-net-x64.sh
    - .planning/phases/14-harvey-amd64-userspace-on-r9/14-EVIDENCE.md
    - .planning/phases/14-harvey-amd64-userspace-on-r9/evidence/14-09-net-smoke-run1.log
    - .planning/phases/14-harvey-amd64-userspace-on-r9/evidence/14-09-net-smoke-run2.log
  modified:
    - build/x86_64/mkimage.sh
    - build/x86_64/subset-build.sh
    - flake.nix
    - .gitignore
key-decisions:
  - "The x64 net gate re-key went GREEN ON THE FIRST BOOT — no kernel defects surfaced: the port devip stack + virtio-net carried from Phase 13 held under the real userspace drivers, and the aarch64 gate's load-bearing STAGE ORDER (ether legs first, ether unbind before loopback, singleton mediums) was carried verbatim as the plan demanded"
  - "The degenerate-input edges (adjacency/empty-input, the backstop truths) are named stages: netgate: DUPBIND-REJECTED stages TWO-OCCURRENCE (typed echo, then the ||-arm output — a silent merge prints nothing and fails the second occurrence) plus devip's own 'interface already bound' asserted in wire-arrival order; netgate: NOADDR-REJECTED the same shape — the marker prefix deliberately distinct from the fixtures' netip: FAIL fast-fail arm"
  - "The tier sweep's committed `T1-SKIP ipconfig` reason ('r9 has no devrandom this era') is STALE for x86_64 — the port devcons serves /dev/random there (no entropy hooks registered this era: deterministic-seed boot-to-boot, distinct bytes per read via counter advance); XEXEC-05's gates prove ipconfig green in-guest, and the sweep check flips with Phase 15's tier append (never a quiet edit of the committed sweep) — recorded in 14-EVIDENCE.md"
  - "The D-11 single-seam discipline is enforced MECHANICALLY: mkimage's net section greps root-tree/lib for 10.0.2. literals outside netaddr.rc (a stray reds the bake), and the plan-level verify re-runs the same grep — even the fixtures' own 'NO hardcoded...' comment prose had to be reworded to keep the mechanical check clean"
  - "The net bake rides the committed skeleton (the fixtures live in root-tree/lib, so the skeleton tar carries them into every bake) with named-input manifest asserts + the ip container's binaries at /amd64/bin/ip + the C helpers and utility set at /amd64/bin — the aarch64 staging list re-keyed to /amd64 paths"
requirements-completed: [XEXEC-05]
commits: 4
plan_head_before: 7177ffa05de12ddb9018bc0a307f1807982b9071   # harvey repo; no r9 changes (the pin stays e417660)
plan_head_after: e49f5c00a5cb2ba7303d38abf8a10a71a3ec99ba   # harvey repo (the docs commit follows this file)
actuals:
  tokens: 31000   # chars/4 over the realized diff (119770 chars code/docs + this file)
  tasks: 3
  commits: 4      # MEASURED: git rev-list --count 7177ffa05..HEAD = 3 + this docs commit
status: complete
coverage:
  - deliverable: "Task 1 — the x64 net fixtures + net-binary bake (tracer)"
    verification:
      - kind: command
        ref: "sh build/x86_64/subset-build.sh: exit 0, TIER-BUILD-OK, 15 T0-ART + 40 T1-ART + the net section (ping/udpecho/dd link-only + the three 6c helpers, all magic 00008a97)"
        status: pass
      - kind: command
        ref: "sh build/x86_64/mkimage.sh: all four bakes PASS incl. bootfs-net.paq (839085 bytes, header 529ab12b/trailer 6b46e688); the net manifest prints 31 lines incl. /amd64/bin/ip/ipconfig + /lib/netip-loopback.rc; determinism: sha256 ffdccd6c on consecutive bakes; the witness manifest verified (38 members)"
        status: pass
      - kind: command
        ref: "the D-11 mechanical grep: ! grep -rn '10\\.0\\.2\\.' build/x86_64/root-tree/lib/ | grep -v netaddr.rc | grep -q . — clean (and now a bake-time assert); git diff --stat HEAD -- build/aarch64/{netaddr.rc,root-tree/lib/} empty"
        status: pass
      - kind: command
        ref: "tracer gate end-to-end re-run (the full verify suite above) green — ⚡ Tracer verified end-to-end — expanding"
        status: pass
    human_judgment: false
  - deliverable: "Task 2 — the composed net gate, green twice (XEXEC-05)"
    verification:
      - kind: command
        ref: "THE GATE: R9_NET_LOG=…run1.log nix run .#boot-r9-net-smoke-x64 then …run2.log — BOTH exit 0, ALL STAGES PASS (45 stage-oks each): chain boot → seam → static (ok/arp/NOT-L2-TRUTH/DONE) → dupbind (echoed/error/rejected) → unbind → dhcp (ok/NOT-L2-TRUTH/DONE) → unbind → loopback (random/ping/udp/DONE) → noaddr (echoed/rejected) → the inline tcp/7 leg → NET-SMOKE completion → pid-warning-absent + nic-bind; both captures retained (evidence/14-09-net-smoke-run{1,2}.log, 7045/7062 bytes)"
        status: pass
      - kind: command
        ref: "the degenerate markers in BOTH captures: netgate: DUPBIND-REJECTED ×2 (typed echo + output), 'interface already bound' ×1, netgate: NOADDR-REJECTED ×2; the venue labels netip: NOT-L2-TRUTH slirp venue ×2 (static + dhcp)"
        status: pass
      - kind: command
        ref: "acceptance greps: NOT-L2-TRUTH ×4 in the expect; -cdrom present; -dtb/device-loader tokens ABSENT; boot-r9-net-smoke-x64 ×2 in flake.nix (baseline 0); nix eval .#apps.x86_64-linux.boot-r9-net-smoke-x64.type = \"app\"; aarch64 smoke-r9-net.expect + nix/smoke-r9-net.sh untouched"
        status: pass
    human_judgment: false
  - deliverable: "Task 3 — 14-EVIDENCE.md, the phase bundle"
    verification:
      - kind: command
        ref: "test -s 14-EVIDENCE.md && XEXEC-01 + XEXEC-05 + NOT-L2-TRUTH present; XEXEC-0[1-5] mentions total 17 (≥5); the witness-parity + flagged-assumption + discretion/OQ sections present; nothing claimed green that was not run (the stale-SKIP honesty note included; r9 push state verified: origin/main..HEAD = 0)"
        status: pass
    human_judgment: false
---

# Phase 14 Plan 9: The x86_64 /Net Gates + the Phase Evidence Bundle Summary

Re-keyed the whole in-guest /net gate stack to x86_64 and closed the phase: the netaddr seam + the four netip fixtures under the D-11 single-seam discipline (now asserted MECHANICALLY at bake time — a stray venue literal reds the bake), the net binaries built (ping/udpecho/dd link-only from the ip container, netblast/tcpblast/tcpecho as the 6c C-helper re-keys) and baked into bootfs-net.paq, and the composed net gate — the aarch64 Phase-8 stage vocabulary re-keyed onto the -cdrom/q35 compose with the load-bearing STAGE ORDER carried verbatim — GREEN TWICE CONSECUTIVELY on the FIRST boot (45 stage-oks each, both captures retained): static ipconfig + DHCP DORA + loopback ping/udp/tcp echo through the REAL binaries, both NOT-L2-TRUTH venue labels asserted as stages, and the two degenerate-input backstop edges as named markers (the equal-address bind rejected visibly with devip's own error on the wire; the empty ping target rejected loudly). 14-EVIDENCE.md consolidates all five XEXEC truths with their gate commands, twice-green records, captures, the witness-parity proof, the flagged-assumption outcomes, and the discretion/OQ resolutions — the phase closes evidenced.

**Duration:** 14 min (2026-11-09T15:35:22Z → 15:49:36Z)
**Tasks:** 3/3 complete (Task 1 tracer, verified end-to-end before expansion)
**Files:** 13 created + 4 modified (all harvey-side; no r9 changes — the pin stays e417660)

## Commits

| Repo | Hash | Message |
|------|------|---------|
| harvey | 4bf559e6c | feat(14-09): the x64 net fixtures + net-binary bake — netaddr seam, netip-*.rc, binaries in the image (XEXEC-05) |
| harvey | 187984ac1 | feat(14-09): the composed net gate — smoke-r9-net-x64 green twice (XEXEC-05) |
| harvey | e49f5c00a | docs(14-09): 14-EVIDENCE.md — the phase's five truths evidenced (XEXEC-01..05) |
| harvey | (docs) | docs(14-09): this file + state |

## Accomplishments

### Task 1: the x64 net fixtures + net-binary bake (tracer)

- **netaddr.rc** re-keyed into `build/x86_64/root-tree/lib/` — the same lo_/slirp_/blastport keys; the v6 keys are honestly NOT carried (no x64 v6 gate this phase; they join with Phase 16's NETIP-08 legs). The ONLY place a venue address may live.
- **netip-{loopback,static,dhcp,tcp}.rc** re-keyed with the fixture discipline verbatim; two honest re-keys: the random leg keeps the D-17 shape but calls the plain `sha1sum` (the x64 net image stages it unprefixed) and documents the no-hooks era (the port devcons serves /dev/random on x86_64 — never errors, deterministic seed, distinct bytes per read); the "devcons: random seeded" marker assert is not carried (that print rides the registered-hooks arm).
- **subset-build's net section** (the bind precedent's shape — never tier members): ping + udpecho link-only from the ip container (`mk 6.ping`/`mk 6.udpecho` — the mkmany $BIN-recipe ._cp class avoided), dd link-only (the fixtures' one-read discipline; write-flow-excluded from the tier data but needed here), and the three C helpers 6c/6l-compiled into out/.
- **mkimage section 10** — the NET bake: bootfs-net.paq (chain tree + the ip binaries at /amd64/bin/ip + the helpers + the utility set + the fixtures riding the skeleton), named-input asserts with remediation, the D-11 literal grep as a BAKE-TIME assert, factotum absence, rcmain cmp, KERNDATE determinism (ffdccd6c twice).

### Task 2: the composed net gate — green twice, first boot

- **smoke-r9-net.expect**: the aarch64 gate re-keyed stage-for-stage; the header documents the load-bearing STAGE ORDER verbatim (ether first — the multi-waiter RX-wake defect; ether unbind before loopback — the two-core wildcard demux; singleton mediums — why the tcp leg rides inline).
- **The degenerate legs** (the plan's backstop edges): `netgate: DUPBIND-REJECTED` (equal-address ipconfig against the bound ether — devip's `interface already bound` asserted between the typed echo and the ||-arm marker, in wire-arrival order) and `netgate: NOADDR-REJECTED` (empty ping target) — a marker prefix distinct from the fixtures' `netip: FAIL` fast-fail arm so the deliberate rejections are assertable without aborting.
- **nix/smoke-r9-net-x64.sh + the boot-r9-net-smoke-x64 app**: the tier-gate driver shape re-keyed (tier+net build → net bake → r9's iso-builder → expect; the r9x64net- survivor-sweep tag distinct from the sibling gates).
- **Both runs green consecutively** — 45 stage-oks each; the first boot needed zero kernel fixes (the port stack held under the real userspace drivers).

### Task 3: 14-EVIDENCE.md — the phase bundle

All five XEXEC truths with their exact gate commands, the twice-green records (q35exec, q35shell ×4, the composed smoke, the tier gate, the net gate), the retained capture paths, the witness-parity record (the 38-member structural manifest + the identical 36-marker sets), the flagged-assumption outcomes, the discretion/OQ resolutions, the venue-honesty record, and the stale-SKIP honesty note. r9's push state verified at consolidation (0 unpushed; the pin e417660 = origin/main).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The net binaries needed a build home the plan's files list omitted**
- **Found during:** Task 1 (the bake's named inputs had no producer)
- **Issue:** the plan lists mkimage.sh (staging) and the C sources, but no build driver for ping/udpecho/dd/the helpers — the tier build alone produces only ipconfig.
- **Fix:** subset-build.sh's net section (the bind precedent's non-tier shape): link-only ping/udpecho in the ip container + dd in cmd, the 6c compiles for the three helpers, host-side magic spot-checks.
- **Files modified:** build/x86_64/subset-build.sh
- **Commit:** harvey@4bf559e6c

**2. [Rule 3 - Hygiene] The fixtures' own comment prose tripped the mechanical D-11 grep**
- **Found during:** Task 1's verify (the literal-address grep FAILED)
- **Issue:** the comment "NO hardcoded 10.0.2.x outside that seam" contains the literal `10.0.2.` pattern the gate greps for — prose, not an address, but the gate is mechanical by design.
- **Fix:** reworded to "NO venue address literal outside that seam"; the grep then passed and became a bake-time assert so the class stays dead.
- **Files modified:** build/x86_64/root-tree/lib/{netip-static,netip-dhcp}.rc
- **Commit:** harvey@4bf559e6c

**3. [Rule 3 - Venue] Standalone build/bake runs need PLAN9 + plan9port on PATH; the flake app needs git-tracked scripts**
- **Found during:** Task 1's runs (mk not found in the chroot; nix refused the untracked script)
- **Issue:** the 14-07 documented standalone contract (the wrapper provides PLAN9; nix flakes only see tracked files).
- **Fix:** exported PLAN9 + PATH for the standalone runs (execution-only); git-added the new script before `nix run` (standard staging, committed in Task 2's commit).
- **Files modified:** none (execution-only) / staging only
- **Commit:** n/a

**Total deviations:** 3 auto-fixed (1 missing build home, 1 comment-prose hygiene, 1 venue/nix re-key). **Impact:** none on scope or guarantees — the D-11 discipline got STRONGER (a bake-time assert), and every plan verify leg ran and passed.

## Authentication Gates

None.

## Known Stubs

None — no stubs, placeholders, or unwired data paths were introduced. Two honest notes recorded in 14-EVIDENCE.md: the tier sweep's stale ipconfig-SKIP reason (the net gates prove ipconfig green on r9-x64; the sweep check flips with Phase 15's append) and the no-entropy-hooks /dev/random era on x86_64 (deterministic boot-to-boot, distinct per read; Phase 11-era hardening owns the quality).

## Issues Encountered

None open — both repos clean, the pin unchanged (e417660, fully pushed).

## TDD Gate Compliance

N/A — no task carries tdd="true".

## Next Plan Readiness

Phase 14 is COMPLETE (9/9 plans, all five XEXEC truths evidenced in 14-EVIDENCE.md) — ready for the phase verification and Phase 15 planning (the fuller tiers append onto the committed tier data; the tier sweep's ipconfig SKIP check flips once the net-proven binary joins the swept set). Phase 16's NETIP legs re-key the seam + the fixtures onto the tap venue (the multi-waiter RX-wake record in the gate header names the path they will hit).

## Self-Check: PASSED

- files exist: the 8 fixture/C sources + smoke-r9-net.expect + nix/smoke-r9-net-x64.sh + 14-EVIDENCE.md + both evidence captures ✓ (13/13 FOUND)
- commits are ancestors of HEAD: 4bf559e6c, 187984ac1, e49f5c00a ✓
- plan-level verification green: the bake carries the net binaries + fixtures with zero venue literals outside the seam (Task 1); the composed net gate green twice with the venue-label stage present and the aarch64 gate byte-untouched (Task 2); 14-EVIDENCE.md names all five truths with evidence (Task 3)
