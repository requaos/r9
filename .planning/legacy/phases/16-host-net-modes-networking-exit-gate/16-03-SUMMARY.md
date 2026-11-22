---
phase: 16-host-net-modes-networking-exit-gate
plan: 03
subsystem: net
tags: [net-doctor, preflight, tap, bridge, host-state, read-only, remediation, nix-flake-app, hnet-05, oq-3]

requires:
  - phase: 16-host-net-modes-networking-exit-gate
    provides: 16-02's tap_ seam keys (tap_gwv4/blastport) — the doctor's lazy tap-mode address source
provides:
  - nix/net-doctor.sh — the read-only host-state doctor (HNET-05/D-14): per-mode PASS/FAIL/WARN/INFO checks, NixOS-first exact remediations, deterministic exit (0 iff zero FAILs, DOCTOR-OK)
  - the net-doctor flake app (linux-gated in its own optionalAttrs; consumes no r9 attrs) — the D-03 one-implementation-two-front-ends artifact, ready for 16-04's R9_NET_DOCTOR inline consumption
affects: [16-04 boot script tap leg (D-03 inline doctor), 16-05 soak gate (doctor preflight), 16-06 CI legs]

actuals:
  tokens: 4637      # chars/4 over the realized diff (git diff db3391660..HEAD -- nix/net-doctor.sh flake.nix)
  tasks: 2
  commits: 2        # MEASURED: git rev-list --count db3391660..HEAD
plan_head_before: db33916608d723e21c061bf0e838977f4701ba0c
plan_head_after: c4cf19d27a2709282c7645f50b8a656d022b5d71

tech-stack:
  added: []          # no new registry packages (T-16-SC held: iproute2/iptables/procps/coreutils/gnugrep/gnused are pre-existing nixpkgs attrs behind the flake lock)
  patterns:
    - "read-only diagnose+print checker: probes in if-guards, every mutation-shaped remediation line is heredoc print payload only (D-14's hard rule, audited by reading)"
    - "OQ-3 exit semantics: WARN class never gates the exit; the root-gated masquerade read degrades to a loud WARN carrying the exact sudo command — never a silent pass, never a FAIL"
    - "lazy seam parsing: tap_gwv4/blastport grep-extracted+eval'd inside the tap block only — slirp/vmnet preflights are seam-independent (wave-1 ordering independence)"

key-files:
  created:
    - nix/net-doctor.sh
  modified:
    - flake.nix

key-decisions:
  - "OQ-3 landed as committed semantics: exit 0 iff zero FAILs; every slirp check fatal, every tap check fatal EXCEPT masquerade (WARN + sudo iptables-save / sudo nft list ruleset with the seam-derived subnet)"
  - "No address literal in the doctor: the tap subnet/ports interpolate seam-derived values ($BRIDGE, tap_gwv4, TAP_SUBNET from tap_gwv4's /24, blastport) into the remediation text — the seam is the single address source host-side too; probe output prints on detail lines only, never inside copied commands (T-16-05)"
  - "The app gate is its OWN linux optionalAttrs — no hasAttr line added anywhere; the frozen kernel-x64/iso-builder list is untouched (research Pitfall 6), verified by diff assertion"
  - "runtimeInputs carry the checks' own tools (qemu/iproute2/iptables/procps + coreutils/gnugrep/gnused); unixtools evaluated NOT needed — nothing doctor-side diverges beyond these (documented in the flake comment)"

patterns-established:
  - "PASS/FAIL/WARN/INFO line vocabulary with indented venue-chosen remediation blocks (NixOS via /run/current-system marker, T-16-07) — the first diagnose+print checker in either repo, now the template for any future host-state preflight"
  - "App-per-gate without gate-list widening: an app that consumes no r9 attrs gets a bare isLinux optionalAttrs, keeping eval lazy and the frozen list frozen"

requirements-completed: [HNET-05]

coverage:
  - id: D1
    description: "nix/net-doctor.sh — the check implementation: all 12 committed checks (slirp 3, tap 8, vmnet 1) with the PASS/FAIL/WARN/INFO vocabulary, NixOS-first remediations, lazy seam parsing, deterministic exit"
    requirement: HNET-05
    verification:
      - kind: command
        ref: "--mode slirq -> exit 1 + 'want slirp|tap|vmnet|all (the valid modes)' usage line (/tmp/doc16u.log)"
        status: pass
      - kind: command
        ref: "--mode slirp -> exit 0, 0 FAIL lines, DOCTOR-OK present (/tmp/doc16s.log)"
        status: pass
      - kind: command
        ref: "--mode tap -> exit 1 with FAIL bridge-conf-allow + environment.etc remediation + WARN masquerade with sudo commands, on the live-probed host state (/tmp/doc16tap.log)"
        status: pass
      - kind: command
        ref: "sh -n SYNTAX-OK; every mutation-shaped line (ip/nft/sysctl/chmod/echo>>) read-audited as heredoc print payload (lines 192-227 of remedy_generic)"
        status: pass
    human_judgment: false
  - id: D2
    description: "the net-doctor flake app — linux-gated in its own optionalAttrs, no r9 attrs consumed, frozen gate list untouched"
    requirement: HNET-05
    verification:
      - kind: command
        ref: "nix run .#net-doctor -- --mode slirp -> exit 0, DOCTOR-OK (shellcheck-clean build); bare script vs app byte-identical on slirp AND tap (diff empty, clean tree)"
        status: pass
      - kind: command
        ref: "nix eval .#apps.x86_64-linux.net-doctor.program OK; diff adds no hasAttr line (NO-HASATTR-ADDED); no added line extends [ \"kernel-x64\" \"iso-builder\" ]"
        status: pass
    human_judgment: false

duration: 9min
completed: 2026-11-11
status: complete
---

# Phase 16 Plan 03: the net-doctor (read-only host-state checker + flake app) Summary

**HNET-05/D-14 delivered: nix/net-doctor.sh probes the per-mode host state read-only (slirp 3 checks, tap 8, vmnet 1) printing PASS/FAIL/WARN/INFO lines with exact NixOS-first-then-generic remediation blocks, exits 0 iff zero FAILs (the OQ-3 WARN-never-gates semantics), and ships as the linux-gated net-doctor flake app — byte-identical to the bare script on both the slirp green path and the tap failing path, ready for 16-04's R9_NET_DOCTOR inline consumption.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-11-11T12:47:54Z
- **Completed:** 2026-11-11T12:56:50Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- The doctor's live run on the primary host nails the plan's expected verdicts exactly: slirp mode all-green (qemu present, both D-04 hostfwd ports free) → DOCTOR-OK exit 0; tap mode fails on EXACTLY the live-probed wrong (bridge.conf carries `allow virbr0` only) with the environment.etc declarative remediation, while bridge exists/UP/addr, ip_forward, the setuid helper (mode 4511), and blastport-free all PASS and the root-gated masquerade read degrades to a loud WARN naming both sudo commands with the seam-derived 192.168.122.0/24
- The read-only discipline is audited by reading: every mutation-shaped remediation line (ip link add/set, ip addr add, sysctl -w, nft add table/chain/rule, chmod u+s, echo>>bridge.conf) lives inside `cat <<EOF` print blocks in remedy_generic — nothing executes; probe output prints on detail lines only, never interpolated into copied commands (T-16-05's mitigation as committed)
- The flake app lands without touching the frozen gate: netDoctorPkg beside the x64 pkgs, apps.net-doctor in its own `optionalAttrs isLinux` block, zero hasAttr additions (asserted against the diff), and the build runs writeShellApplication's shellcheck checkPhase clean

## Task Commits

Each task was committed atomically:

1. **Task 1: the net-doctor.sh check implementation** - `912bb8ee8` (feat)
2. **Task 2: the net-doctor flake app** - `c4cf19d27` (feat)

**Plan metadata:** (see final docs commit)

## Files Created/Modified
- `nix/net-doctor.sh` - the read-only doctor: argv `--mode slirp|tap|vmnet|all` (fail-loud usage idiom), named host constants (BRIDGE/HELPER/BRIDGE_CONF/QEMU_BIN/hostfwd ports), lazy seam_init (grep+eval of tap_gwv4/blastport, tap mode only), the 12 checks, remedy dispatch (NixOS /run/current-system marker → declarative snippets; generic otherwise), FAIL counting with DOCTOR-OK/exit semantics
- `flake.nix` - netDoctorPkg (mkScriptApp, empty extraHeader, runtimeInputs bash/qemu/iproute2/iptables/procps/coreutils/gnugrep/gnused) + the apps.net-doctor linux-only optionalAttrs block

## Decisions Made
- OQ-3's exit-code contract committed in code: WARN-class findings never gate the exit — the CI preflight is deterministic; the masquerade check tries iptables-save then nft, treats ANY unreadable ruleset as WARN (the doctor cannot know), and only a SUCCESSFUL read with no matching rule FAILs
- The tap subnet is derived from the seam at runtime (`${tap_gwv4%.*}.0/24`) and interpolated into remediations — zero address literal in the script; the bridge NAME (br0) is a named constant cross-referenced to the r9 TOML's br= (a host object name, not a venue address)
- Port checks are word-bounded exact matches (`ss -tln` + `[:.]PORT\b`) so 17010 never matches 170100; the occupied-FAIL remediation names the holder lookup (`ss -tlnp`) rather than guessing
- vmnet prints an INFO n/a line on linux pointing at the darwin manual docs (the check never gates anything); the darwin branch checks the socket_vmnet socket path informationally

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] port-check polarity inversion in the first draft**
- **Found during:** Task 1 (the plan's slirp verify step)
- **Issue:** the first `port_free` helper returned grep's status directly — 0 when a LISTEN socket MATCHED (port held) — so the caller's `if port_free` branch read held-as-free and free-as-held: the healthy host failed both hostfwd checks with empty holder lines
- **Fix:** inverted to `port_busy` (true when a listener matches) with the branches swapped; the slirp verify then passed exactly as the plan expects (3 PASS + DOCTOR-OK, exit 0)
- **Files modified:** nix/net-doctor.sh
- **Verification:** verify 2 green (0 FAIL lines, DOCTOR-OK); tap mode's blastport-free PASSes against the same helper
- **Committed in:** 912bb8ee8 (part of task commit; found and fixed pre-commit)

---

**Total deviations:** 1 auto-fixed (1 Rule 1)
**Impact on plan:** None structural — a first-draft logic slip caught by the plan's own verify step before the task commit; the committed artifact never carried it

## Live Evidence (Task 1 acceptance — the current-host FAIL retained)

The tap-mode run on the primary host (retained at /tmp/doc16tap.log; NixOS marker present, so remediations render declarative-first):

```
net-doctor: PASS bridge-exists: br0 present
net-doctor: PASS bridge-up: br0 flags carry UP
net-doctor: PASS bridge-addr: 192.168.122.1/24 present on br0 (the seam's tap_gwv4)
net-doctor: PASS ip-forward: net.ipv4.ip_forward=1
net-doctor: WARN masquerade: cannot read the netfilter ruleset unprivileged — verify with: sudo iptables-save | grep '192.168.122.0/24' (or: sudo nft list ruleset | grep '192.168.122.0/24'); want a MASQUERADE rule for 192.168.122.0/24 out the default route
net-doctor: PASS helper-wrapper: /run/wrappers/bin/qemu-bridge-helper present + setuid (mode 4511)
net-doctor: FAIL bridge-conf-allow: /etc/qemu/bridge.conf has no exact 'allow br0' line (allow lines now:allow virbr0 )
    remediation (NixOS — declarative; add to the host/colmena config, then rebuild):
    environment.etc."qemu/bridge.conf".text = ''
      allow br0
      allow virbr0  # keep ONLY while libvirt's virbr0 must stay allowed (append semantics)
    '';
    # the EXACT per-bridge line only — "allow all" is forbidden (the ACL is the control point)
net-doctor: PASS blastport-free: port 7777 free (no LISTEN holder)
net-doctor: 1 FAIL(s) — apply the printed remediations (the doctor is read-only: it diagnoses, the human applies)
```

exit=1 — exactly the plan's expected current-state verdict (FAIL bridge-conf-allow on the wrong-allow file, WARN masquerade on the root-gated read, everything else green). The slirp run (/tmp/doc16s.log): 3 PASS + DOCTOR-OK, exit 0. The all-modes run (/tmp/doc16all.log): 9 PASS + 1 WARN + 1 FAIL + 1 INFO, exit 1. The unknown-mode run (/tmp/doc16u.log): exit 1 with the usage line naming the valid modes.

## Verification Notes
- Front-end identity proven on a clean tree: bare `sh nix/net-doctor.sh` vs `nix run .#net-doctor` produce byte-identical output for both slirp and tap (diff empty) — the D-03 one-implementation contract; the earlier "Git tree is dirty" line in the app logs was nix's own warning on the pre-commit tree and vanished after the Task 2 commit
- The app build exercises writeShellApplication's shellcheck checkPhase — the script is lint-clean including the two intentional `# shellcheck disable=SC2312` sites (the masquerade read and the ip-forward sysctl read, where masking the substitution's status IS the design)
- Host-state probes confirming the expectations were taken before implementation: bridge.conf=`allow virbr0` only, br0 UP at 192.168.122.1/24, ip_forward=1, helper mode 4511, 17010/5670/7777 free, iptables-save unprivileged → "Permission denied (you must be root)", nft absent from PATH

## Issues Encountered
None

## User Setup Required
None - no external service configuration required. (Applying the printed remediations on the primary host — the `allow br0` bridge.conf line and the masquerade rule — is 16-04/16-05's runtime prerequisite and the human's choice of when, per D-14: the doctor never applies anything.)

## Next Phase Readiness
- 16-04's boot script tap leg can consume the doctor via R9_NET_DOCTOR (D-03) knowing both front-ends print byte-identical blocks; the boot app merely embeds them in its failure path and names `nix run .#net-doctor`
- 16-05's soak gate has its doctor-preflight primitive: tap-mode green requires the human to first apply the environment.etc `allow br0` remediation (plus a masquerade rule the WARN cannot verify unprivileged) — the preflight exit code is the deterministic gate signal
- HNET-05 is complete: diagnose + exact remediation + deterministic preflight exit delivered; the sole remaining Phase-16 requirement-family items live in 16-04..16-08's plans

## Self-Check: PASSED

- Files: nix/net-doctor.sh present (committed 912bb8ee8), flake.nix modified (committed c4cf19d27)
- Commits: 912bb8ee8, c4cf19d27 both ancestors of HEAD (FOUND)
- Commits measured from the plan ledger: 2 (no uncommitted code changes; only pre-existing .planning/research/.cache files remain untracked)

---
*Phase: 16-host-net-modes-networking-exit-gate*
*Completed: 2026-11-11*
