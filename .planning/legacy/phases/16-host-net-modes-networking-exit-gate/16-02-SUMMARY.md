---
phase: 16-host-net-modes-networking-exit-gate
plan: 02
subsystem: net
tags: [tap, bridge, netaddr-seam, netip, mkimage, bake-assert, netip-tap, l2-truth, netip-08]

requires:
  - phase: 14-harvey-amd64-userspace-on-r9
    provides: the 14-09 net bake section + netaddr.rc seam + the netip-static fixture skeleton
  - phase: 08-r9-tcp-ip-path-devip-compatible-net (v2.0)
    provides: the aarch64 netip-soak.rc shapes (argv switches, blast leg, /net/tcp/stats reconciliation)
provides:
  - netaddr.rc tap_ leg keys (tap_guestv4/tap_maskv4/tap_gwv4 — the adopted br0 192.168.122.0/24, host .1 guest .15; OQ-1's committed bridge-subnet decision)
  - /lib/netip-tap.rc — the NETIP-08 tap-leg truth fixture (static ipconfig + ARP L2-truth assert + baseline/burst soak legs + stats reconciliation, argv [rounds] [burstn])
  - mkimage.sh's bake-time seam assert covering BOTH venue prefixes (slirp + tap), teeth-proven by a doctored-input negative control
  - dc staged into the net image (the burst leg's mandated dc backquote counter)
affects: [16-03 net-doctor bridge-address check, 16-05 soak gate (expect-driven launch + host echo peer bind), 16-06 CI legs]

actuals:
  tokens: 4021      # chars/4 over the realized diff (git diff plan_head_before..HEAD)
  tasks: 2
  commits: 2        # MEASURED: git rev-list --count ae1c479d8..HEAD
plan_head_before: ae1c479d8d4d53d9d143952b810c2c53c819a516
plan_head_after: d28134991deca49afcc9ba7c6494a29f7a388b56

tech-stack:
  added: []          # no new packages (dc is an in-tree tier-1 binary newly staged into the net image)
  patterns:
    - "one seam grep per venue prefix, one err per stray class (the D-06 mechanical extension of 14-09 D-11)"
    - "rc burst leg: N backgrounded tcpblast + dc backquote counter + bare wait + a wait-returned marker; marker-COUNT assert owned by the host-side expect post-pass (rc cannot collect per-child statuses after wait)"

key-files:
  created:
    - build/x86_64/root-tree/lib/netip-tap.rc
  modified:
    - build/x86_64/root-tree/lib/netaddr.rc
    - build/x86_64/mkimage.sh

key-decisions:
  - "OQ-1 adopted: the host's existing br0 192.168.122.0/24 becomes the committed tap seam (host .1, guest .15) — collision-free with slirp's 10.0.2.0/24 and the host LAN"
  - "OQ-4 verdict recorded in the seam header: v6 was evaluated NOT free to verify (D-09) and stays deferred to the phase that first needs it — the stale join-with-Phase-16 anticipation is re-keyed so it cannot be re-read as a promise"
  - "dc joins the net bake utility set (Rule 3): the plan-mandated dc backquote burst counter needs the binary in the guest image; additive list extension only, tier-1-built"

patterns-established:
  - "Venue-literal bake assert scales by duplication: one _stray grep + one err per venue prefix over root-tree/lib, seam excluded by name — comment prose counts (15-07 lesson held: netaddr.rc is the only file carrying the literal)"
  - "The tap fixture's L2-truth label is the asserted inverse of the slirp NOT-L2-TRUTH label — venue honesty rides the gate log in both directions"

requirements-completed: [HNET-03, NETIP-08]

coverage:
  - id: D1
    description: "netaddr.rc tap_ seam keys (additive; slirp_ keys and blastport byte-unchanged; v6 note re-keyed with the not-free-to-verify verdict)"
    requirement: HNET-03
    verification:
      - kind: command
        ref: "grep -c tap keys = 3; git diff '-' lines touching slirp_/blastport = empty (SLIRP-KEYS-BYTE-UNCHANGED)"
        status: pass
    human_judgment: false
  - id: D2
    description: "/lib/netip-tap.rc fixture — static-ipconfig skeleton on tap_ keys, ARP L2-truth assert, baseline + dc-counted burst + bare-wait + stats legs, flipped venue label, zero venue literal outside the seam"
    requirement: NETIP-08
    verification:
      - kind: command
        ref: "grep -c '192\\.168\\.122\\.' netip-tap.rc = 0 (NO-TAP-LITERAL); markers netsoak: burst wait returned / L2-TRUTH tap venue / netip: NETIP-DONE present; bind/ipconfig/ping/tcpblast/bare-wait shapes present; grep -c '^if(!' = 0"
        status: pass
      - kind: command
        ref: "bake manifest line 'net manifest: /lib/netip-tap.rc staged' in /tmp/b16.log and /tmp/green16.log"
        status: pass
    human_judgment: false
  - id: D3
    description: "mkimage.sh seam assert extended to the tap prefix + netip-tap.rc/dc staged; teeth proven by doctored-input negative control"
    requirement: HNET-03
    verification:
      - kind: command
        ref: "doctored '# doctored 192.168.122.77' in netip-static.rc -> mkimage exit 1 with 'venue address literal outside netaddr.rc' naming file+line; git checkout revert -> all 5 images baked green with the netip-tap manifest line (/tmp/red16.log, /tmp/green16.log)"
        status: pass
    human_judgment: false

duration: 8min
completed: 2026-11-11
status: complete
---

# Phase 16 Plan 02: the tap_ seam keys + the netip-tap fixture + the bake-assert extension Summary

**Guest-side addressing half of the tap leg: netaddr.rc gains the additive tap_ keys (br0 192.168.122.0/24, host .1 guest .15 — OQ-1 adopted), netip-static's skeleton is re-venued as netip-tap.rc with the /net/arp L2-truth assert and the netip-soak legs (baseline tcpblast + dc-counted N-way burst behind a bare wait + /net/tcp/stats reconciliation), and mkimage's single-seam assert now greps both venue prefixes — teeth-proven by a doctored-input negative control that reds the bake.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-11-11T12:28:41Z
- **Completed:** 2026-11-11T12:36:33Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- The netaddr seam carries the tap leg as three additive keys; the slirp leg and blastport are byte-unchanged (git diff proves zero removed lines touch them), and the header's stale v6 anticipation is re-keyed with OQ-4's not-free-to-verify verdict
- The new /lib/netip-tap.rc re-vends the proven x64 dialect (|| FAIL ladders, word-wise ~ asserts, switch validation for the two-argv contract) onto tap_ values: real ipconfig static flow, gateway ping, the ARP entry assert with the flipped `netip: L2-TRUTH tap venue` label, baseline tcpblast at $rounds, the $burstn-way 512-round burst backgrounded with & and dc-counted, bare wait + `netsoak: burst wait returned`, and the InSegs/OutSegs non-zero reconciliation — zero venue literal anywhere in the file
- The bake mechanically enforces the single seam for BOTH venue prefixes; the net bake stages netip-tap.rc (manifest line proven) and dc (the burst counter's dependency); bootfs-net.paq rebuilt green alongside all four sibling images

## Task Commits

Each task was committed atomically:

1. **Task 1: the tap_ seam keys + the netip-tap.rc fixture** - `0ddaa88a2` (feat)
2. **Task 2: the bake assert extension + manifest, teeth-proven** - `d28134991` (feat)

**Plan metadata:** (see final docs commit)

## Files Created/Modified
- `build/x86_64/root-tree/lib/netaddr.rc` - additive tap_ leg block (guest/mask/gw keys + the D-06/D-08/D-09 venue-properties comment); v6 note re-keyed with the OQ-4 verdict
- `build/x86_64/root-tree/lib/netip-tap.rc` - the NETIP-08 tap-leg truth fixture (created)
- `build/x86_64/mkimage.sh` - 10b second _stray grep for the tap prefix; netip-tap.rc in 10a presence + 10h manifest; dc in 10a/10f/10h; comment blocks updated to name both

## Decisions Made
- OQ-1 adopted as committed data: the primary host's existing br0 192.168.122.0/24 is the tap subnet (host .1, guest .15) — the 16-03 doctor's bridge check and the 16-05 peer's bind consume the same seam values
- OQ-4 recorded in the seam itself: v6 is NOT free to verify; no v6 key joins this phase (the note now states this verdict, so the old anticipation cannot be re-read as a promise)
- Marker-count ownership: per-child statuses are not collectible after rc's bare `wait` — the fixture prints `netsoak: burst wait returned` and 16-05's expect post-pass owns counting `tcpblast: done` occurrences

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking dependency] dc added to the net bake utility set**
- **Found during:** Task 2 (mkimage edits), grounded during Task 1 analysis
- **Issue:** The plan mandates the burst-leg counter "via the dc backquote shape from research Pattern 5" (`i=`{echo $i 1 + p | dc}``), but dc was not in the 14-09 net image's utility set (grep sed dd wc sha1sum test bind) — in-guest the backquote would fail and the burst loop would never terminate (the counter cannot advance), a time bomb for 16-05's launch
- **Fix:** dc (an in-tree tier-1 binary, `cmd/dc` in tiers/tier1-cmds.txt, present at amd64/bin/dc) joined the net bake additively: the 10a input check, the 10f staging cp, and the 10h manifest — additive list extensions only, no slirp-fixture perturbation
- **Files modified:** build/x86_64/mkimage.sh
- **Verification:** `net manifest: /amd64/bin/dc staged` in the bake log; bootfs-net.paq green; negative control unaffected
- **Committed in:** d28134991 (part of task commit)

**2. [Rule 2 - Documentation honesty] mkimage comment blocks updated alongside the list edits**
- **Found during:** Task 2
- **Issue:** The file-header net-image contract and section-10's comment still described "the four netip-*.rc sweeps" and the utility set without dc/tap — comments that lie about the payload are the 15-07 defect class
- **Fix:** Both comment blocks extended to name netip-tap.rc, the five-fixture set, dc, and the both-prefix seam assert
- **Files modified:** build/x86_64/mkimage.sh
- **Verification:** visual + the acceptance greps still pass
- **Committed in:** d28134991 (part of task commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 3, 1 Rule 2)
**Impact on plan:** None structural — both are additive extensions the plan's own "additive list extensions only" rule contemplates; the slirp gates' fixtures and behavior are untouched

## Negative Control Record (Task 2 acceptance)

Both control directions were run and recorded:
- **RED:** `printf '# doctored 192.168.122.77\n' >> build/x86_64/root-tree/lib/netip-static.rc` then `sh build/x86_64/mkimage.sh` → **exit 1** with `error: venue address literal outside netaddr.rc (the D-11 seam violated): .../root-tree/lib/netip-static.rc:135:# doctored 192.168.122.77` (/tmp/red16.log) — the assert has teeth, names the file and line, and fires at bake time before the net image is written
- **GREEN:** `git checkout -- build/x86_64/root-tree/lib/netip-static.rc` then re-bake → **exit 0**, all five images baked, `net manifest: /lib/netip-tap.rc staged` present (/tmp/green16.log)

## Verification Notes

- The full bake verify ran as `nix develop -c sh -c 'sh build/x86_64/subset-build.sh > /tmp/b16.log 2>&1 && sh build/x86_64/mkimage.sh >> /tmp/b16.log 2>&1'` — the plan's literal command plus the devshell wrapper mkimage's own usage header mandates ("inside the nix devshell"); subset-build was incremental (tree unchanged since the Phase-15 builds)
- This host's grep treats a mid-pattern `$` as an anchor, so the acceptance greps for `ip/ipconfig -g $tap_gwv4` and `tcpblast $tap_gwv4 $blastport` need fixed-string matching (`grep -F`) — both substrings are present (the file text is exactly as planned); noted so future verifiers don't misread a 0 count

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- 16-05's soak gate can bake and launch /lib/netip-tap.rc with nothing left to define guest-side (fixture + seam + bake coverage all landed; the host echo peer binds $tap_gwv4:$blastport)
- 16-03's doctor consumes the same tap_ seam values for its bridge-address check (host-side consumer of the committed 192.168.122.0/24 decision)
- HNET-03 and NETIP-08 remain open across the phase (shared-ID gate: other Phase 16 plans declare them; the soak's green-twice verdict is 16-05's)

## Self-Check: PASSED

- Files: netaddr.rc, netip-tap.rc, mkimage.sh all present (FOUND)
- Commits: 0ddaa88a2, d28134991 both ancestors of HEAD (FOUND)
- Commits measured from the plan ledger: 2 (no uncommitted code changes)

---
*Phase: 16-host-net-modes-networking-exit-gate*
*Completed: 2026-11-11*
