---
phase: 07-r9-pci-ecam-virtio-net-devether
plan: 04
subsystem: testing
tags: [aarch64, devether, arp-fixture, rc, netblast, bootfs, mkimage, subset-build, dd, tx-bulk]

requires:
  - phase: 05-shell-boot-one-command-integration
    provides: build/aarch64 subset-build.sh + mkimage.sh bake harness (root-tree skeleton, out/ supply chain, manifest/rodata guards)
  - phase: 02 (v1.0 milestone)
    provides: bake-vs-ufs resolution (bake-into-bootfs for gates) and the aarch64 7c/7l toolchain
provides:
  - build/aarch64/root-tree/lib/test/arp.req — the committed 60-byte ARP-request fixture (D-07 + ETHERMINTU errata)
  - build/aarch64/root-tree/lib/net-sweep.rc — the sourced two-mode sweep (arp | blast) with netsweep: FAIL <stage>/NET-DONE markers
  - build/aarch64/netblast.c — the D-08 TX-bulk generator (pure-TX ARP to 10.0.2.99), baked at /aarch64/bin/netblast
  - dd in the baked image (aarch64/bin/dd via dd.install + SUBSET staging)
  - a green bake with all four payload paths manifest-asserted
affects: [07-05-virtnet-profile, 07-06, 07-07-driver-gates, phase-08-devip]

actuals:
  tokens: 2940
  tasks: 2
  commits: 2
  plan_head_before: 1de01b4bda3b75bba918f9fc9724c6e483b79fc1

tech-stack:
  added: []
  patterns:
    - "committed binary fixture discipline (hello.7.out precedent) extended to protocol payloads"
    - "sourced rc sweep with named FAIL stages via the || idiom per command (never typed)"
    - "insertion-only edits to shared build scripts (zero-deletion diffs) for parallel-phase collision avoidance"
    - "host-tool shadowing defense: extract bytes with od -j (no Plan 9 od exists to shadow it) — the D-02 class"

key-files:
  created:
    - build/aarch64/root-tree/lib/test/arp.req
    - build/aarch64/root-tree/lib/net-sweep.rc
    - build/aarch64/netblast.c
  modified:
    - build/aarch64/subset-build.sh
    - build/aarch64/mkimage.sh

key-decisions:
  - "Sweep dd syntax is the dash-pair form (dd -if <fixture> -of <datafile>) — the baked sys/src/cmd/dd.c takes -if/-of pairs, not if=/of= (plan prose corrected; in-tree convention per build/scripts/linux-build)"
  - "dd/netblast join via appended statements (SUBSET=\"$SUBSET dd\", separate mk dd.install and manifest assert lines) so the acceptance's zero-deletion constraint and Phase-6 collision avoidance hold literally"
  - "net-sweep.rc's connect step pre-checks test -e on the ctl file: a redirect-open failure is an uncatchable rc Xerror; the pre-check preserves the named-FAIL contract for the open-failure class, || on the echo covers the write-failure class"
  - "mkimage.sh trailer magic re-extracted with od -j — Harvey's host dd shadows POSIX dd on the devshell PATH (D-02 hostbin-shadowing class) and its dash-pair syntax left the trailer empty (bake red)"

patterns-established:
  - "net payload in bootfs: fixture under root-tree/lib/test/, sweep under root-tree/lib/, blaster compiled into out/ and staged to aarch64/bin — all manifest-asserted"
  - "sweep marker vocabulary: netsweep: arp sent <i> / netsweep: NET-DONE / netsweep: FAIL <bind|clone|connect|dd|blast|stats>; blaster: netblast: sent <n> frames"

requirements-completed: [NETDRV-03]

coverage:
  - id: D1
    description: "Committed 60-byte ARP-request fixture (dst broadcast, src 52:54:00:12:34:56, ethertype 0x0806, spa 10.0.2.15, tpa 10.0.2.2, 18-byte zero pad) at build/aarch64/root-tree/lib/test/arp.req"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "wc -c == 60; od -An -tx1 -N14 == ffffffffffff5254001234560806 (plan verify gates 1+4)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Two-mode sourced sweep build/aarch64/root-tree/lib/net-sweep.rc: arp mode (bind/clone/connect 2054/n×dd/stats/NET-DONE) and blast mode (clone/netblast/stats/NET-DONE), every failure a named netsweep: FAIL <stage> with non-nil exit"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "host plan9port-rc stub harness: arp 2, arp (default 4), blast 5, blast (default), bad-mode, bad-count ×2, no-args, clone-missing → FAIL clone, ctl-missing → FAIL connect, dd-fail → FAIL dd, netblast-fail → FAIL blast — all correct markers and exit statuses"
        status: pass
      - kind: other
        ref: "grep gates: netsweep: FAIL ≥ 4 (11), NET-DONE ≥ 1 (2), tabs-only indentation, no leading-space lines"
        status: pass
    human_judgment: false
  - id: D3
    description: "dd joins the baked image self-containedly (dd.install per-target install; SUBSET staging; host-side assert) — no new libs in the closure"
    requirement: NETDRV-03
    verification:
      - kind: integration
        ref: "sh build/aarch64/subset-build.sh → SUBSET-BUILD-OK; aarch64/bin/dd present (47210 bytes); mkimage.sh '== PASS: baked'"
        status: pass
    human_judgment: false
  - id: D4
    description: "netblast TX-bulk generator (D-08): argv conv id + optional count (default 256), one static 60-byte ARP frame with tpa 10.0.2.99 (pure TX, zero RX noise), per-write ==60 checks, netblast: sent <n> frames marker, exits(nil)"
    requirement: NETDRV-03
    verification:
      - kind: integration
        ref: "subset-build.sh compiled+linked out/netblast (7c/7l, R_MAGIC 00008c47 asserted); mkimage.sh staged aarch64/bin/netblast with manifest assert; bake '== PASS: baked' (1061067 bytes)"
        status: pass
      - kind: other
        ref: "rodata grep on the built binary: netblast: sent + netblast: write failed both present; source greps: exits( ×4, no bare exit(, 0x0a,0x00,0x02,0x63 tpa bytes present"
        status: pass
    human_judgment: false
  - id: D5
    description: "Bake-supply-chain resilience: mkimage.sh trailer magic survives the devshell PATH (Harvey host dd shadowing fixed via od -j re-extraction)"
    verification:
      - kind: other
        ref: "od -An -tx1 -j $((sz-28)) -N 4 == 6b46e688 against the baked image; == PASS: baked line present"
        status: pass
    human_judgment: false

duration: 19 min
completed: 2026-11-05
status: complete
---

# Phase 07 Plan 04: net test payload (ARP fixture + net-sweep + netblast + dd) Summary

**The guest half of the D-07/D-08 gates is baked and byte-proven: a committed 60-byte ARP fixture, a two-mode sourced rc sweep with named FAIL stages, the netblast TX-bulk generator compiled into the image beside stacksmoke, and dd joining the binary set — all four payload paths manifest-asserted in a green bake.**

## Performance

- **Duration:** 19 min (plus one guest-toolchain settle wait, see Issues)
- **Started:** 2026-11-04T23:43:45Z
- **Completed:** 2026-11-05T00:02:50Z
- **Tasks:** 2 (1 tracer + 1 auto)
- **Files modified:** 5 (3 created, 2 modified)

## Accomplishments
- `build/aarch64/root-tree/lib/test/arp.req` — committed 60-byte fixture (D-07 + Pitfall N2 errata: 42-byte ARP + 18 zero pad to ETHERMINTU=60); byte-verified (60 bytes; first 14 = ff×6 + 525400123456 + 0806)
- `build/aarch64/root-tree/lib/net-sweep.rc` — the sourced two-mode sweep: `arp [n]` (bind #l → clone → connect 2054 → n dd writes of the fixture, default 4 → stats + ifstats → NET-DONE) and `blast [n]` (clone → netblast → stats → NET-DONE); every failure prints `netsweep: FAIL <stage>` and exits non-nil; proven on host plan9port rc across both modes and all failure paths
- `build/aarch64/netblast.c` — the D-08 TX-bulk generator in the stacksmoke shape: argv-driven conv id + count (default 256), one static 60-byte ARP frame targeting tpa 10.0.2.99 (no slirp responder → pure TX), every write checked == 60, `netblast: sent <n> frames` completion marker
- dd in the image: `dd.install` via cmd's per-target rule (single-file, u.h+libc.h only — no new libs), SUBSET staging, host-side assert; netblast compiled/harvested via the stacksmoke-shaped block and staged to aarch64/bin/netblast
- Supply chain green end-to-end: subset-build SUBSET-BUILD-OK + mkimage `== PASS: baked` (1061067 bytes, header 529ab12b trailer 6b46e688), with the payload manifest-asserted
- All edits insertion-only (numstat 0 deletions on both scripts) — no collision surface for the parallel Phase 6 tier machinery

## Task Commits

1. **Task 1: arp.req fixture + net-sweep.rc + dd into the image (D-07)** — `a288badb6` (feat) — 4 files, 132 insertions
2. **Task 2: netblast.c TX-bulk generator (D-08) + staging wiring + compile gate** — `9fb015a57` (feat) — 3 files, 86 insertions

_Tracer feedback gate (after Task 1): interactive + end-of-phase + automated-only verify → re-ran all four verify commands end-to-end (fixture bytes, subset-build, mkimage, od header) — all green; expanded per the row-3 auto-continue._

## Files Created/Modified
- `build/aarch64/root-tree/lib/test/arp.req` — the committed 60-byte ARP-request fixture (baked at /lib/test/arp.req)
- `build/aarch64/root-tree/lib/net-sweep.rc` — the two-mode sweep (baked at /lib/net-sweep.rc)
- `build/aarch64/netblast.c` — the TX-bulk blaster source (baked at /aarch64/bin/netblast)
- `build/aarch64/subset-build.sh` — dd.install line + host assert; netblast compile/harvest block + rsync-back entry + R_MAGIC assert
- `build/aarch64/mkimage.sh` — SUBSET append (dd), out/netblast input check + staging, manifest lines for lib/test/arp.req, lib/net-sweep.rc, aarch64/bin/netblast; trailer magic via od -j

## Decisions Made
- **dd syntax corrected to the dash-pair form** (`dd -if <fixture> -of <datafile>`): the baked sys/src/cmd/dd.c parses `-key value` pairs (iskey requires the dash), so the plan prose's `if=/of=` form would fail on the guest with "dd: bad arg". In-tree convention confirmed at build/scripts/linux-build:36.
- **Insertion-only placement over alphabetical-in-list**: the action prose's "alphabetical into the existing list" conflicts with the acceptance criteria's zero-deleted-lines/insertions-only constraint (an in-list edit is a delete+insert). The checkable criteria govern: dd joins via `SUBSET="$SUBSET dd"` and a separate `mk dd.install`; netblast via additive blocks. Runtime effect identical.
- **connect step gets a `test -e` pre-check**: a failed redirect open is an uncatchable rc Xerror (aborts the script without the named FAIL line — proven on host rc); the pre-check covers the open-failure class, the `||` on the echo covers the write-failure (Ebadctl) class.
- **sweep FAIL-stage vocabulary extended with `stats`** for the epilogue cats (plan enumerated bind|clone|connect|dd|blast); "every failure named" covers the stats dump too.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] dd argument syntax corrected to dash-pairs**
- **Found during:** Task 1 (net-sweep.rc authoring + host-rc harness run)
- **Issue:** The plan's action text specifies `dd if=/lib/test/arp.req of=/net/ether0/$conv/data`. Harvey's baked dd (sys/src/cmd/dd.c) parses dash-prefixed key/value PAIRS (`iskey` = key[0]=='-'), so the classic form exits "dd: bad arg" — the sweep's dd step would fail on the real guest.
- **Fix:** Sweep calls `dd -if /lib/test/arp.req -of /net/ether0/$conv/data` (the in-tree invocation convention).
- **Files modified:** build/aarch64/root-tree/lib/net-sweep.rc
- **Verification:** host plan9port-rc harness: arp mode drives the dd stub per-iteration; Harvey host dd confirmed rejecting `if=` with "bad arg" while accepting `-if` pairs
- **Committed in:** a288badb6

**2. [Rule 3 - Blocking] mkimage.sh trailer check re-extracts via od -j (host-dd shadowing)**
- **Found during:** Task 1 verify (`sh build/aarch64/mkimage.sh` failed: "bad trailer magic (want 6b46e688)")
- **Issue:** Pre-existing, environment-exposed: the trailer check's `dd if=... bs=1 skip=... count=...` resolves to Harvey's host dd because the devshell PATH puts .build/hostbin-* first; the dash-pair-only dd rejects the classic syntax (stderr nulled by the check), leaving trailer empty and the bake red. Same D-02 hostbin-shadowing class as the grep fix in 877cdae11.
- **Fix:** Insertion-only shadow assignment right after the existing line: `trailer=$(od -An -tx1 -j $((sz - 28)) -N 4 "$IMAGE" | tr -d ' \n')`. od is never shadowed (Plan 9 has no od — xd is its od).
- **Files modified:** build/aarch64/mkimage.sh
- **Verification:** bake green twice since (1041466 then 1061067 bytes; trailer 6b46e688 both times)
- **Committed in:** a288badb6

**3. [Rule 2 - Missing Critical] connect step pre-checks ctl existence (rc Xerror class)**
- **Found during:** Task 1 (host-rc harness: `/net/ether0/3/ctl: rc: can't open` aborted the script with no FAIL line)
- **Issue:** A redirect-open failure in rc raises Xerror and aborts the script — `if(! echo ... > ctl)` / `echo ... || ...` can never print the named FAIL line for the open-failure class, violating the sweep's "every failure prints netsweep: FAIL <stage>" contract.
- **Fix:** `test -e /net/ether0/$conv/ctl || { FAIL connect }` before the redirect; the `||` on the echo itself still covers write failures (Ebadctl).
- **Files modified:** build/aarch64/root-tree/lib/net-sweep.rc
- **Verification:** harness failure path "ctl missing" → prints `netsweep: FAIL connect`, exit 1
- **Committed in:** a288badb6

**4. [Rule 1 - Bug] First fixture generation had two wrong MAC bytes**
- **Found during:** Task 1 (the plan's own od verify gate)
- **Issue:** octal slips (\020/\034 instead of \022/\064) produced src 52:54:00:10:1c:56 — the header gate expects 525400123456.
- **Fix:** regenerated with corrected octal; re-verified (60 bytes; first-14 exact; full dump matches the D-07 field table).
- **Files modified:** build/aarch64/root-tree/lib/test/arp.req
- **Verification:** `od -An -tx1 -N14` == ffffffffffff5254001234560806
- **Committed in:** a288badb6

---

**Total deviations:** 4 auto-fixed (2 blocking, 1 missing-critical, 1 bug)
**Impact on plan:** All four were necessary for the plan's own gates to be passable on the real guest/host; the deliverable contract (fixture bytes, sweep modes/markers, blaster behavior, manifest-guarded bake) is exactly as planned. No scope creep.

## Issues Encountered
- **Lima guest toolchain races (twice):** two Task-2 gate runs failed at nm.BUILD with `ar: .../unix/9cc/obj.linux/lib9.a|libmach.a: No such file or directory`. Root-caused while debugging: (a) vm-run.sh's ensure sentinel is a RELATIVE path (`$GUEST_DIR/.build/...`) checked after `cd $GUEST_DIR`, so it never exists and the full ~minutes toolchain re-runs on EVERY invocation (this is deferred item 02-4's actual mechanism); (b) every vm-run.sh push rsyncs with `--delete`, wiping the guest's gitignored `obj.linux` (a darwin host has only obj.darwin); (c) toolchain.sh lacks `set -e`, so a raced nm.BUILD failure doesn't stop the chain — a zombie build kept mutating the shared guest while my next push wiped obj.linux under it. After the zombie settled (guest toolchain complete: 7c/ar/nm + lib9/libbio/libmach all present), the gate re-ran green. Not caused by this plan's changes; recorded as a sharpened finding in the phase deferred-items.md.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- 07-05 (virtnet profile + synthetic-fixture assertion units) can source /lib/net-sweep.rc and rely on the marker vocabulary (`netsweep: arp sent <i>` / `netsweep: NET-DONE` / `netsweep: FAIL <stage>`; `netblast: sent <n> frames`).
- 07-06/07-07 (devether/virtio-net driver + live gates): the guest payload is baked and manifest-guarded — the only remaining variable is the kernel, exactly the plan's intent.
- Requirements note: NETDRV-03 is shared by 07-01/04/05/07/08; the shared-ID gate correctly held it open (07-05/07-07/08 unfinished). This plan's portion (the payload) is complete.

## Self-Check: PASSED
- Files: arp.req, net-sweep.rc, netblast.c, SUMMARY.md all present on disk
- Commits: a288badb6 (Task 1), 9fb015a57 (Task 2) both in git log
- Plan verification: fixture 60 bytes + header ffffffffffff5254001234560806; subset-build SUBSET-BUILD-OK (with dd.install and the netblast compile leg in the log); mkimage '== PASS: baked' ×2 (payload manifest-asserted); numstat 0 deletions on both scripts; rodata/factotum guard greps unchanged (2, 7)
