---
phase: 16-host-net-modes-networking-exit-gate
plan: 04
subsystem: netdev-infra
tags: [netdev, flake-pin, r9-pin, boot-app, net-modes, slirp, tap, hostfwd, emitter-substitution, net-doctor, x86_64]
duration: 14 min
completed: 2026-11-11
requires:
  - "16-01-complete the pushed r9 netdev-args emitter (r9 d1c4849dfa1d769240630c29b9b03b88d5819558 — the pin target)"
  - "16-03-complete the harvey-net-doctor app (the D-03 tap gate's implementation, consumed via R9_NET_DOCTOR)"
provides:
  - "the r9 pin advanced af52fee -> d1c4849dfa1d769240630c29b9b03b88d5819558 as its OWN lock-only commit (45d4436f8), every existing x64 app eval-green at the new pin"
  - "the boot app's net surface: --net=slirp|tap|vmnet (the = form ONLY) over R9_NET over the slirp default — unknown mode and cross-venue requests are parse-time usage errors naming the valid set; no silent downgrade anywhere"
  - "the healed launch block: the qemu netdev+device argv is command substitution of R9_NETDEV_EMITTER --mode $NET_MODE (word-split) — zero netdev literals left in nix/boot-r9-x64.sh; the D-04 hostfwd pair rides every default boot's echoed argv"
  - "the D-03 tap gate: --net=tap runs R9_NET_DOCTOR --mode tap FIRST, aborts on any FAIL with the per-issue remediation blocks inline + the 'nix run .#net-doctor' pointer — proven live on this host (bridge-conf-allow FAIL direction)"
  - "the flake exports R9_NETDEV_EMITTER + R9_NET_DOCTOR on boot-r9-x64, and the x64 apps hasAttr gate widened to [kernel-x64 iso-builder netdev-args] in the same commit set as the pin advance (Pitfall 6)"
affects:
  - "16-05 (consumes R9_NETDEV_EMITTER and R9_NET_DOCTOR through the same app pattern — the success criterion)"
  - "16-06 (HNET-01's hosted-CI half rides the now-emitter-backed default argv)"
  - "Phases 18/19 (drawterm/auth docs cite nix run .#boot-r9-x64 -- --net=<mode> and the 127.0.0.1:5670->567 remap)"
tech-stack:
  added: []
  patterns:
    - "flag-over-env-over-default machine-contract selection extended from R9_ACCEL to the net modes: a case ladder against the closed three-name set per venue, every failure a named usage error — never a passthrough into qemu"
    - "argv-as-emitted-data in shell: netdev+device tokens arrive by command substitution of the pinned r9 emitter and are word-split unquoted (shellcheck-disabled deliberately) — the script assembles no netdev text"
key-files:
  created:
    - .planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-04-boot-slirp.log
  modified:
    - flake.lock
    - flake.nix
    - nix/boot-r9-x64.sh
key-decisions:
  - "The tap leg's doctor invocation precedes the emitter call (acceptance-order honored literally): contract checks for both paths first, then the doctor run, then the emitter capture — a FAIL never reaches the emitter, let alone qemu"
  - "R9_NET_DOCTOR is contract-checked (-f/-x) only inside the tap leg: slirp/vmnet boots stay independent of the doctor's presence (16-03's own mode-independence discipline mirrored boot-side)"
  - "The doctor runs with HARVEY passed explicitly from the boot script's resolved root, so the seam lookup (tap_gwv4/blastport) never depends on the caller's cwd"
  - "Task 2's tap verify ran with R9_NETDEV_EMITTER/R9_NET_DOCTOR manually exported to the exact paths Task 3 wires (the pinned github rev's netdev-args build + the doctor app program) — the flake exports could not exist before Task 3's own commit; re-proven through the flake app alone after 116fd7f2f"
patterns-established:
  - "Emitter substitution in launch blocks: the echoed argv line is the gate-visible proof surface for data-sourced tokens (grep hostfwd= on the echo, not on the script)"
requirements-completed: [HNET-01, HNET-02]
actuals:
  tokens: 2985    # chars/4 over git diff 9934c1634..116fd7f2f (flake.lock + flake.nix + nix/boot-r9-x64.sh; estimate was 50000 — the plan was wiring, not invention)
  tasks: 3
  commits: 3      # measured: git rev-list --count 9934c1634..116fd7f2f
plan_head_before: 9934c163450889388be612ebf94fa75ff383ca57
plan_head_after: 116fd7f2f4de36b1ad8513baedbd7ca8ccc3ed7d
r9_pin_before: af52feef7620137c43c5e04d077819fbcf3ddb82
r9_pin_after: d1c4849dfa1d769240630c29b9b03b88d5819558
status: complete
coverage:
  - id: D1
    description: "The r9 pin advance as its OWN commit: flake.lock-only diff (rev+narHash+lastModified of the r9 node alone), every existing x64 app eval-green at the new pin"
    requirement: HNET-02
    verification:
      - kind: other
        ref: "PIN-COMMIT-CLEAN (HEAD touches only flake.lock; no + line carries af52fee or the nixpkgs rev 1d060086) + nix eval boot-r9-x64/net-doctor/smoke-x64/tier1-x64/net-smoke-x64/services-x64 programs all green; subject names d1c4849dfa1d769240630c29b9b03b88d5819558; pinned rev equality asserted"
        status: pass
    human_judgment: false
  - id: D2
    description: "The --net/R9_NET surface: = form only, flag > env > slirp default, unknown mode + cross-venue rejections with the named valid-mode vocabulary"
    requirement: HNET-02
    verification:
      - kind: other
        ref: "R9_NET=bogus nix run .#boot-r9-x64 -> exit 1 + '--net/R9_NET=bogus: want slirp|tap|vmnet'; --net=vmnet on linux -> exit 1 + cross-venue line naming 'linux's valid net modes: slirp tap'; '--net tap' (no =) -> usage error naming the accepted form"
        status: pass
    human_judgment: false
  - id: D3
    description: "The healed launch block: emitter-substituted netdev+device argv carrying the D-04 hostfwd pair, no netdev literal in the script"
    requirement: HNET-02
    verification:
      - kind: other
        ref: "grep -c 'netdev user' nix/boot-r9-x64.sh == 0; the captured boot's argv echo contains hostfwd=tcp:127.0.0.1:17010-:17010 and hostfwd=tcp:127.0.0.1:5670-:567 (evidence/16-04-boot-slirp.log)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The D-03 doctor-inline tap gate: doctor FIRST, abort on FAIL with per-issue remediation blocks embedded + the nix run .#net-doctor pointer, never a fall-through"
    requirement: HNET-02
    verification:
      - kind: other
        ref: "--net=tap -> exit 1 with the doctor's full tap report embedded (bridge-conf-allow FAIL + NixOS remediation block + masquerade WARN) + 'full report: nix run .#net-doctor -- --mode tap'; re-proven through the flake app alone after 116fd7f2f (FLAKE-WIRED-TAP-GATE-OK)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The hermetic default boot proof (HNET-01's local half): unprivileged full-pipeline default boot to the rc prompt with the loopback hostfwd data in the echoed argv"
    requirement: HNET-01
    verification:
      - kind: other
        ref: "timeout-bounded captured run: banner 'r9 from the Internet' + '% ' prompt + both hostfwd tokens in the argv echo + zero '0.0.0.0' in the capture (BOOT-PROOF-OK, LOOPBACK-ONLY-OK); evidence/16-04-boot-slirp.log (25584 bytes) retained"
        status: pass
    human_judgment: false
---

# Phase 16 Plan 04: r9 pin advance + the boot app's net surface Summary

**The r9 pin advanced to the 16-01 pushed SHA (d1c4849, lock-only own commit) and the one-command boot gained its roadmap interface — `--net=slirp|tap|vmnet` over `R9_NET` over the hermetic slirp default — with the hardcoded netdev line healed into pinned-emitter substitution (D-04 hostfwd pair in every echoed argv) and the tap leg doctor-gated inline (D-03); the default boot reached the `%` prompt unprivileged in a captured run.**

## PENDING FIRST PUSH (the carried honesty convention)

The pin-advance commit's hosted-CI observation is **PENDING FIRST PUSH** — Harvey's three production commits of this plan (45d4436f8, 209d29cdd, 116fd7f2f) are local to `master` until the phase's Harvey commits push (16-08's wave). The local eval proof at the new pin is the standing evidence until then; the fresh hosted-CI observation lands with the push per the carried push-before-pin discipline.

## Performance

- **Duration:** 14 min
- **Started:** 2026-11-11T13:04:58Z
- **Completed:** 2026-11-11T13:19:18Z
- **Tasks:** 3/3
- **Files modified:** 4 (flake.lock, flake.nix, nix/boot-r9-x64.sh + the evidence capture)

## Accomplishments

- The r9 input advanced af52fee → d1c4849dfa1d769240630c29b9b03b88d5819558 via `nix flake lock --update-input r9` — the diff touched ONLY the r9 node (rev/narHash/lastModified; the transitive nixpkgs_2/rust-overlay nodes unmoved, 16-01 having never touched r9's own lock); committed alone with the SHA in the subject
- Every existing x64 app (boot-r9-x64, net-doctor, smoke/tier1/net-smoke/services-x64) evals green at the advanced pin
- `nix/boot-r9-x64.sh` parses its first-ever arguments: exactly `--net=<mode>`, validated per venue (linux: slirp+tap; darwin: slirp+vmnet) — unknown modes, cross-venue requests, duplicate flags, and any other argument are named usage errors, never a silent downgrade
- The tap leg runs the 16-03 doctor FIRST and aborts on any FAIL with the per-issue remediation blocks inline (live proof on this host: `bridge-conf-allow` FAIL — `/etc/qemu/bridge.conf` still allows only `virbr0` — with the NixOS declarative remediation printed and `nix run .#net-doctor` named)
- The qemu netdev+device argv is now the pinned r9 emitter's output (`R9_NETDEV_EMITTER --mode $NET_MODE`, word-split unquoted); `grep -c 'netdev user' nix/boot-r9-x64.sh` == 0
- The flake app exports `R9_NETDEV_EMITTER` (r9 `packages.netdev-args` bin) and `R9_NET_DOCTOR` (the doctor app) in the `R9_ISO_BUILDER` precedent shape, and the x64 apps hasAttr gate widened to include `netdev-args` — in a commit AFTER the pin advance, with the Pitfall-6 coupling documented at the gate
- The HNET-01 local proof: unprivileged default boot through the full pipeline (build → bake → compose → qemu, KVM) reached the rc prompt with both loopback hostfwd tokens (`tcp:127.0.0.1:17010-:17010`, `tcp:127.0.0.1:5670-:567`) in the echoed argv and zero `0.0.0.0` anywhere in the capture — `evidence/16-04-boot-slirp.log` retained in the phase dir

## Task Commits

Each task committed atomically:

1. **Task 1: the r9 pin advance — its OWN commit** — `45d4436f8` (chore)
2. **Task 2: the --net/R9_NET surface + emitter substitution + doctor-inline tap leg** — `209d29cdd` (feat)
3. **Task 3: flake exports + the hermetic default boot proof** — `116fd7f2f` (feat)

**Plan metadata:** this SUMMARY + STATE/ROADMAP/REQUIREMENTS updates + the evidence capture.

## Files Created/Modified

- `flake.lock` — the r9 node advanced to the 16-01 pushed SHA (rev+narHash+lastModified only)
- `nix/boot-r9-x64.sh` — step 0c (venue/parse/validation + emitter+doctor contracts + the D-03 gate), the healed launch block, the header net-discipline/env-contract/usage documentation
- `flake.nix` — bootR9X64Pkg's two new exports; the x64 apps gate list widened to `netdev-args` with the Pitfall-6 coupling comment
- `.planning/phases/16-.../evidence/16-04-boot-slirp.log` — the captured default-boot proof

## Decisions Made

See key-decisions in the frontmatter. The load-bearing ones:

1. **Doctor-before-emitter inside the tap leg** — the acceptance criterion is ordering-literal; both contract checks (-f/-x) run first, the doctor gate second, the emitter capture last, so a FAIL never spends an emitter exec and certainly never reaches qemu.
2. **R9_NET_DOCTOR checked only in the tap leg** — a slirp boot stays runnable even where the doctor app is absent, mirroring 16-03's own mode-independence rule.
3. **Verify-3 mechanics under the task split** — Task 2's tap proof needed the two contract env vars a day before Task 3's flake edit could export them; the vars were supplied manually at the exact paths Task 3 wires, then the same proof re-ran through the flake app alone after 116fd7f2f.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 2's tap verify required Task 3's not-yet-existing flake exports**
- **Found during:** Task 2 (verify 3)
- **Issue:** the plan's `nix run .#boot-r9-x64 -- --net=tap` fires the D-03 doctor gate only if `R9_NETDEV_EMITTER`/`R9_NET_DOCTOR` are in the app env — but those exports are Task 3's flake.nix edit, which commits after Task 2
- **Fix:** ran Task 2's verify 3 with both vars manually exported to exactly what Task 3 wires (`github:requaos/r9/d1c4849...#netdev-args`'s bin + the net-doctor app program path) — same binaries, same paths, no script changes; after Task 3's commit the identical command re-ran through the flake app ALONE and hit the doctor gate identically
- **Files modified:** none (verify mechanics only)
- **Verification:** `/tmp/b16p3.log` (manual env) and `/tmp/b16p3b.log` (flake app alone) both: exit 1, `net-doctor: FAIL` blocks embedded, `nix run .#net-doctor` named
- **Committed in:** n/a (no code change; both proofs green)

---

**Total deviations:** 1 auto-fixed (1× Rule 3 verify-mechanics)
**Impact on plan:** None on artifacts — every plan file, commit boundary, and acceptance criterion landed exactly as written; the deviation is confined to the intermediate-proof mechanics forced by the plan's own Task-2/Task-3 split.

## Issues Encountered

- The captured boot run used a stdin feeder (`sleep 900 | timeout 900 nix run ...`) so the interactive qemu console never sees EOF mid-boot; once the prompt markers were in the log the run was ended by an exact-PID SIGTERM (the plan's timeout-bound shape, ended early with the capture complete). The log's final lines show the `%` prompt then the termination notice — no assertion reads past the prompt.
- `init: can't open #c/sysname` + `syscall 10 unimplemented` appear in the capture — pre-existing pinned-kernel behavior on the boot path (visible before this plan; the boot proceeds to the prompt). Out of scope here; no r9 kernel change is in this plan's remit.
- No other issues: all three failure shapes, the eval battery, and the boot proof were green on first attempt.

## User Setup Required

None - no external service configuration required. (Host-side tap remediation — the `allow br0` bridge.conf line — is the human's NixOS/colmena change whenever the tap leg is actually wanted; the doctor prints it; 16-07 documents it.)

## Next Phase Readiness

- 16-05 can consume `R9_NETDEV_EMITTER` and `R9_NET_DOCTOR` through the same mkScriptApp extraHeader pattern — both are proven wired end-to-end (the soak app's tap boot rides them)
- 16-06's hosted-CI leg boots the now-emitter-backed default argv; the PENDING FIRST PUSH observation above lands with the phase's push
- The parallel-boot constraint is now live for the interactive boot too: two simultaneous default boots collide on 127.0.0.1:17010/5670 (the hostfwd listeners bind at netdev init)

## Self-Check: PASSED

- Modified files exist and carry the changes: flake.lock (r9 rev d1c4849...), nix/boot-r9-x64.sh (step 0c + healed launch block), flake.nix (both exports + widened gate), evidence/16-04-boot-slirp.log (25584 bytes)
- All 3 task commits (45d4436f8, 209d29cdd, 116fd7f2f) are ancestors of HEAD, each staged exactly its task's files
- Verify battery green: PIN-COMMIT-CLEAN, EVAL-OK (6 apps), 3/3 failure shapes with exact vocabulary, BOOT-PROOF-OK, LOOPBACK-ONLY-OK, pin-before-gate ordering asserted

---
*Phase: 16-host-net-modes-networking-exit-gate*
*Completed: 2026-11-11*
