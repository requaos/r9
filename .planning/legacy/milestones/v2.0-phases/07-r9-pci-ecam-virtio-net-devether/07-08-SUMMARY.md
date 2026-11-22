---
phase: 07-r9-pci-ecam-virtio-net-devether
plan: 08
subsystem: infra
tags: [r9, flake-lock, pin, push-before-pin, virtio-net, devether, netdev-stanza, d-10, boot-r9, smoke-r9, nic-bind, gicv3, hvf, hosted-ci, r9-shell, evidence-index, netdrv-03]

# Dependency graph
requires:
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: 07-01..07-07 — the full r9-side networking stack at HEAD 2afa3d6 (PCI/ECAM discovery, virtio transport, driver bind, devether #l, both payoff gates green) plus the committed evidence captures
  - phase: 05-shell-boot-one-command-integration (v1.0)
    provides: the pin mechanics (05-11: push-first, locked-rev==live-HEAD gate, owner metadata check), the evidence-index shape + PENDING FIRST PUSH convention (05-EVIDENCE.md), the r9-shell.yml hosted CI leg (05-13)
provides:
  - r9 main pushed to requaos/r9 (e72abfe..2afa3d6, 19 commits) — the phase's r9 work resolves from a clean clone
  - flake.lock pin advanced to 2afa3d6ebfde3a6a5fcc123d2cfaf89fd6dc0466 (owner requaos) as its OWN commit 06380f2bc — the compose pipeline now delivers the dev-ether virtshell kernel
  - the D-10 end state: `-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0` on all three compose paths (nix/boot-r9.sh, build/aarch64/run-r9.sh, build/aarch64/smoke-r9.expect) — the standard boot speaks Ethernet
  - the smoke's nic-bind whole-capture post-pass (`#l0: virtio: 1000Mbps: ` presence; absence fails naming the stage)
  - a green composed smoke at the new pin on the default darwin leg (gicv3+hvf — the first live NIC-on-GICv3 composed boot)
  - a FRESH hosted CI run green at the new pin: r9-shell run 41955568449 SUCCESS (no substitution needed)
  - 07-EVIDENCE.md — the phase gate index mapping every gate to the SHA it ran at
affects: [phase-8 (devip/arp/ethermedium bind to #l on the standard boot), phase-9 (HNET-02 single-sources this exact stanza shape into platform TOML), milestone-close]

# Actuals (#2632) — pairs with the plan's estimate (tokens 55000, confidence: low).
actuals:
  tokens: 5160     # chars/4 over this plan's three harvey commits (20642 chars); r9-side was push-only (0 new commits)
  tasks: 2
  commits: 3       # this plan's harvey commits (06380f2bc, 15739d108, fcd92f0bf); raw ledger range count is 5 — two interleaved Phase-6 commits (fa19c15f7, 90224e2e8) from the concurrent session share the range

plan_head_before: 2d2fa572546f017f94395614d32e126c91b8fcfa   # harvey repo (ledger #3968); r9: push-only this plan (e72abfe..2afa3d6 went to origin)
plan_head_after: fcd92f0bf   # harvey repo at evidence commit (the SUMMARY/metadata commit rides on top); r9 HEAD 2afa3d6ebfde3a6a5fcc123d2cfaf89fd6dc0466 == origin/main

# Tech tracking
tech-stack:
  added: []   # zero new packages anywhere (T-07-SC vacuous — the pin consumes the existing github input mechanism)
  patterns:
    - "push-before-pin, executed: r9 main to origin FIRST (ls-remote gate), the flake.lock advance SECOND as its own commit, locked-rev==live-HEAD + owner==requaos as the supply-chain gate (T-07-17)"
    - "the always-on NIC stanza as static argv tokens appended via the scripts' existing set --/spawn-list shapes — no shell interpolation of variable content (T-07-18); the flag shape is the one Phase 9's HNET-02 single-sources"
    - "whole-capture post-pass assertions in the expect smoke: the pid-warning-absent scan's shape inverted for a PRESENCE requirement (nic-bind), stage-named on failure"

key-files:
  created:
    - .planning/phases/07-r9-pci-ecam-virtio-net-devether/07-EVIDENCE.md
  modified:
    - flake.lock (r9 pin e72abfe -> 2afa3d6 — own commit)
    - nix/boot-r9.sh (the netdev stanza + D-10 comment in the composed interactive boot's argv)
    - build/aarch64/smoke-r9.expect (the stanza in the spawn argv + the nic-bind post-pass)
    - build/aarch64/run-r9.sh (the stanza + D-10 comment in the dev launcher's argv)

key-decisions:
  - "07-08: push-before-pin executed — r9 e72abfe..2afa3d6 (19 commits) pushed to requaos/r9 BEFORE the flake.lock advance; the pin commit 06380f2bc names flake.lock ALONE; locked rev == live HEAD == origin/main, owner=requaos (the 05-11 gate shape, T-07-17)"
  - "07-08: D-10's end state landed — the identical netdev stanza on all three compose paths and the smoke's nic-bind post-pass; the composed smoke green on the default darwin leg (gicv3+hvf) = the first live NIC-on-GICv3 composed boot; R9_GICV2=1 remains the bisection hatch, never the gate default"
  - "07-08: hosted CI requirement satisfied for real — r9-shell run 41955568449 SUCCESS at headSha 15739d108 (the pushed pin+compose HEAD); the 05-13 PENDING FIRST PUSH substitution was NOT needed; legacy harvey.yml's red streak recorded as pre-existing and unrelated (red on every master push since 2026-11-04T12:01Z, incl. pre-phase pushes)"

patterns-established:
  - "the phase evidence index (07-EVIDENCE.md): every gate x the SHA it ran at x the pin commit x the smoke verdict x the CI run reference — the v1.0 D-13 evidence ordering carried into v2.0"
  - "pin-amend discipline for the close-out class: a pin advance is always its own commit with a trivially auditable diff; compose-script changes never ride it"

requirements-completed: [NETDRV-03]  # copied verbatim from plan frontmatter; NETDRV-03 is shared with 07-01/04/05/07 — this is the LAST declaring plan, so the ready-ids gate releases it here

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "Push-before-pin + the pin advance as its own commit: r9 main pushed to requaos/r9 (e72abfe..2afa3d6), flake.lock locked rev == live HEAD == origin/main, owner=requaos, the pin commit touching only flake.lock"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "PIN-OK chain (r9 tree clean + ls-remote origin main == 2afa3d6… + flake.lock locked rev == live HEAD + owner requaos via the -A8 window + the python structural assert) printed PIN-OK / PIN-STRUCT-OK; git show --stat 06380f2bc names only flake.lock"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-10 end state: the identical stanza tokens on all three compose paths and the composed smoke green with the NIC bound — every pre-existing stage assertion unchanged PLUS the nic-bind whole-capture post-pass, on the default darwin leg (gicv3+hvf)"
    requirement: NETDRV-03
    verification:
      - kind: e2e
        ref: "nix run .#boot-r9-smoke exit 0 (shape virt,gic-version=3 accel=hvf cpu=host, kernel store path r9-kernel-virt-2afa3d6); build/aarch64/smoke-r9.log line 990 '#l0: virtio: 1000Mbps: 52:54:00:12:34:56'; ALL STAGES PASS incl. nic-bind; COMPOSE-NIC-OK grep chain"
        status: pass
    human_judgment: false
  - id: D3
    description: "A fresh hosted CI run covers the new pin — r9-shell.yml exercising boot-r9-smoke on ubuntu-latest"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "GitHub Actions run 41955568449 (https://github.com/requaos/harvey/actions/runs/41955568449): conclusion success, headSha 15739d108595bde0f17b88081dfcc25f2c271848, 2026-11-05T02:29:42Z→02:37:05Z"
        status: pass
    human_judgment: false
  - id: D4
    description: "07-EVIDENCE.md — the phase gate index: every gate (discovery prints, moved-window perturbation, transport unit tests, payload bake, gate machinery red→green, driver bind on both GIC generations, arp ping-pong + blast with committed pcaps) with its r9 SHA, the pin commit, the smoke verdict, the CI run reference"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "EVIDENCE-INDEX-OK (test -s + committed-pcap reference + 40-char SHA presence); 90 lines >= the 30-line must_have"
        status: pass
    human_judgment: false

# Metrics
duration: 20 min
completed: 2026-11-05
status: complete
---

# Phase 07 Plan 08: phase close-out — push-before-pin, the D-10 standard-boot NIC, green smoke + hosted CI, and the evidence index Summary

**r9's Phase-7 networking work is published and pinned: 19 commits pushed to requaos/r9 main (`e72abfe..2afa3d6`) BEFORE Harvey's flake.lock advanced to the same rev as its own isolated commit (locked rev == live HEAD == origin/main, owner requaos) — and the standard composed boot now speaks Ethernet by default: all three compose paths carry the identical `-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0` stanza, the packaged smoke gained a nic-bind whole-capture post-pass, and `nix run .#boot-r9-smoke` is green at the new pin on the default darwin leg (`gicv3+hvf` — the first live NIC-on-GICv3 composed boot, bind line `#l0: virtio: 1000Mbps: 52:54:00:12:34:56` at capture line 990), with a FRESH hosted CI run green at the pushed HEAD (r9-shell run 41955568449, success, ~7.5 min on ubuntu-latest) and the phase's evidence index (07-EVIDENCE.md) mapping every gate to the SHA it ran at.**

## Performance

- **Duration:** 20 min
- **Started:** 2026-11-05T02:21:58Z
- **Completed:** 2026-11-05T02:42:19Z
- **Tasks:** 2
- **Files modified:** 5 (flake.lock, nix/boot-r9.sh, build/aarch64/smoke-r9.expect, build/aarch64/run-r9.sh, 07-EVIDENCE.md created)

## Accomplishments

- **Task 1 — push-before-pin + the isolated pin commit:** r9 working tree confirmed clean at the 07-07-gated HEAD `2afa3d6`; pushed `e72abfe..2afa3d6` (19 commits — the whole phase: 07-01 netdev/pcap parser, 07-02 PCI/ECAM + moved-window, 07-03 virtio transport, 07-05 gate machinery, 07-06 driver bind, 07-07 devether + both gates) to `requaos/r9` main; `nix flake lock --update-input r9` advanced the pin to the just-pushed HEAD; the PIN-OK chain verified tree-clean + remote==HEAD + locked-rev==HEAD + owner==requaos; committed flake.lock ALONE (`06380f2bc` — 1 file, 3/3 lines; `git show --stat` gate); the flake evaluates both boot apps against the new pin.
- **Task 2 — D-10 end state + gates + evidence:** the identical netdev stanza landed on all three compose paths with D-10 comments (no filter-dump object — capture stays a gate feature); smoke-r9.expect gained the nic-bind whole-capture post-pass (presence, stage-named on failure) with every pre-existing stage assertion byte-untouched; `nix run .#boot-r9-smoke` green at the new pin on the default darwin leg (gicv3+hvf — the first live NIC-on-GICv3 composed boot; the bind line at capture line 990); Harvey master pushed (`…→15739d108`) and the hosted r9-shell run **41955568449 concluded SUCCESS** at the pin+compose HEAD; 07-EVIDENCE.md (90 lines) consolidates the phase: every gate × the r9 SHA it ran at × the pin commit × the smoke verdict × the CI run reference.

## Task Commits

Each task was committed atomically per the dual-repo contract (r9: push-only this plan — the phase's r9 work was already committed at 07-07's HEAD):

**r9 repo (branch main):** no new commits — the push `e72abfe..2afa3d6` (19 commits) published the already-gated HEAD to origin; verified `git ls-remote origin main` == `2afa3d6ebfde3a6a5fcc123d2cfaf89fd6dc0466` post-push.

**Harvey repo (branch master):**
1. **Task 1: advance the r9 flake.lock pin past the Phase-7 networking commits** — `06380f2bc` (chore) — flake.lock ALONE (the isolated pin commit per the locked D-decision)
2. **Task 2: the standard boot speaks Ethernet — always-on virtio-net NIC on all three compose paths (D-10 end state)** — `15739d108` (feat) — nix/boot-r9.sh, build/aarch64/smoke-r9.expect, build/aarch64/run-r9.sh
3. **Task 2: 07-EVIDENCE.md — the phase gate index** — `fcd92f0bf` (docs) — 1 file created

**Plan metadata:** the docs commit (this SUMMARY + STATE/ROADMAP/REQUIREMENTS) rides on top in the Harvey repo.

## Files Created/Modified

- `flake.lock` — the r9 input advanced `e72abfe` → `2afa3d6` (rev/narHash/lastModified; owner requaos unchanged)
- `nix/boot-r9.sh` — the netdev stanza + one-line D-10 comment in the composed interactive boot's qemu argv (header argv sketch updated to match)
- `build/aarch64/smoke-r9.expect` — the stanza in the spawn argv; the nic-bind whole-capture post-pass (the pid-warning-absent scan inverted to a PRESENCE check, failing as its own named stage); the header stage-list updated
- `build/aarch64/run-r9.sh` — the stanza + D-10 comment in the dev launcher's argv (header argv doc updated)
- `.planning/phases/07-r9-pci-ecam-virtio-net-devether/07-EVIDENCE.md` (NEW, 90 lines) — the phase gate index

## Decisions Made

Surfaced in the frontmatter key-decisions; the short forms:

1. **Push-before-pin, executed in order** — push first, pin second, pin isolated; the three supply-chain gates (remote==HEAD, locked-rev==HEAD, owner==requaos) green before the commit.
2. **D-10 end state as static argv tokens** — no filter-dump on the compose paths; the exact flag shape Phase 9's HNET-02 will single-source into platform TOML.
3. **The hosted CI requirement satisfied for real** — the push was planned and authorized, the run is green, nothing recorded as pending (the 05-13 substitution path not taken). The legacy harvey.yml failure on the same push is pre-existing (red on every master push since 2026-11-04T12:01Z, including pushes before this phase's commits existed) and unrelated — recorded in 07-EVIDENCE.md for transparency.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] PIN-OK gate command's grep window too small for the real lock shape**
- **Found during:** Task 1 verification (`grep -A4 '"r9"' flake.lock | grep -q '"owner": "requaos"'` — stage 5 failed while stages 1–4 passed)
- **Issue:** the r9 node's `inputs` block (nixpkgs + rust-overlay follows) pushes the `locked` opener 8 lines below the `"r9": {` node opener, so the plan's `-A4` context window never reaches the `"owner": "requaos"` line. The artifact was correct; the check's window was written against a smaller node shape.
- **Fix:** re-ran the gate with `-A8` (reaches the owner field) PLUS a structural python assert of the locked block (owner/repo/rev/type — the 05-11 gate shape). Both green; PIN-OK printed by the corrected chain.
- **Files modified:** none (verification-only correction)
- **Verification:** OWNER-OK, PIN-STRUCT-OK, PIN-OK all printed
- **Committed in:** n/a

**2. [Rule 1 - Bug] Acceptance command names a nonexistent flake attribute**
- **Found during:** Task 1 acceptance (`nix eval .#apps.aarch64-darwin.boot-r9.name` → "does not provide attribute …")
- **Issue:** the flake's `mkApp` emits `{ type = "app"; program = …; }` — apps have no `name` attribute, so the plan's literal command cannot succeed against this flake shape.
- **Fix:** realized the command's stated intent ("the flake evaluates with the advanced pin — no IFD surprise") via `.program` on BOTH boot apps (`boot-r9`, `boot-r9-smoke`) — the attribute that forces the `R9_KERNEL_PKG` store-path interpolation against the advanced pin. Both resolved to store paths.
- **Files modified:** none (verification-only correction)
- **Verification:** both evals printed store paths; EVAL-OK
- **Committed in:** n/a

---

**Total deviations:** 2 auto-fixed (both Rule 1 — verification-command bugs against the real artifact shapes; the artifacts themselves were correct). **Impact on plan:** none on substance — every plan truth bullet holds as measured; the corrected checks assert the plan's stated intent more faithfully than the literal commands.

## Issues Encountered

- **Concurrent Phase-6 session interleaving on master (expected, managed):** two Phase-6 commits (`fa19c15f7`, `90224e2e8`) landed between my pin and compose commits — the dispatch's warned-about concurrent session. No action needed: my commits stayed scoped to plan files, and the master push carried the interleaved commits along (shared-branch reality; the CI run's headSha `15739d108` names MY compose commit, which sits on top of them — recorded exactly in the evidence index).
- **Background CI poller's nested-quote python parse printed empty statuses:** cosmetic harness bug in my polling loop (zsh quoting); the direct API query gave the verdict immediately (completed/success). No plan impact.
- **Legacy `harvey.yml` red on the push:** pre-existing failure streak (runs 4–9 since 2026-11-04T12:01Z, incl. pre-phase pushes); its pipeline (amd64 9k ISO donor-VM) touches nothing this plan changed. Logged to the phase's transparency note in 07-EVIDENCE.md; out of scope per the scope-boundary rule.

## User Setup Required

None - no external service configuration required. (The pushes used the existing GitHub SSH credentials — no auth gate triggered.)

## Next Phase Readiness

- **Phase 07 is COMPLETE** — all 8 plans summarized. NETDRV-01..04 all satisfied with execution evidence (NETDRV-03's shared gate releases here, this being the last declaring plan).
- **Phase 8 (devip/arp/ethermedium):** the `#l` file contract it binds to is live, proven, and now present in the STANDARD composed boot — the loopback-first staging can bind ethermedium to devether on any `nix run .#boot-r9`. Two named omissions remain its to consume or close (07-07): no TX loopback/broadcast feedback (etheroq→etheriq) and the gate-unexercised blocking Qmsg read path.
- **Phase 9 (HNET-02):** the netdev stanza shape it single-sources into platform TOML is now identical on all three compose paths (`-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0`).
- **Milestone-level:** the evidence index lets a future reader reproduce every Phase-7 gate at its recorded SHA from clean checkouts.
- No blockers.

## Self-Check: PASSED

- Files: `07-EVIDENCE.md` FOUND on disk (90 lines ≥ 30) and git-tracked; the three compose scripts carry the stanza (`grep -c 'netdev user,id=n0'` = 2/1/2 — each ≥ 1; `virtio-net-pci-non-transitional` present in all three); smoke-r9.expect's pre-existing stage names unchanged (grep -c = 6) PLUS the nic-bind post-pass
- Commits (harvey): `06380f2bc` (pin — `git show --stat` names ONLY flake.lock), `15739d108` (compose), `fcd92f0bf` (evidence) — all FOUND in `git log`; ledger-measured range count 5 with 2 interleaved Phase-6 commits (recorded honestly in actuals)
- r9: push verified — `git ls-remote origin main` == `2afa3d6ebfde3a6a5fcc123d2cfaf89fd6dc0466` == live HEAD; r9 tree clean
- Plan-level <verification> re-run at final state: PIN-OK (corrected window) PASS; COMPOSE-NIC-OK PASS; EVIDENCE-INDEX-OK PASS
- Acceptance criteria: Task 1 (PIN-OK printed; pin commit contains only flake.lock; both boot apps eval against the new lock) PASS. Task 2 (stanza tokens on all three paths each ≥ 1; stage names unchanged + post-pass added; smoke log carries the full stage sequence PLUS the bind line on the default shape; evidence index with per-gate SHAs + pin reference + CI URL/verdict; Harvey main carries the pin commit and the compose+evidence commits separately, pin first) PASS
- Gates: `nix run .#boot-r9-smoke` exit 0 at the 2afa3d6 pin (gicv3+hvf; nic-bind stage ok; capture line 990); hosted CI run 41955568449 SUCCESS at headSha `15739d108…`
- No files staged outside plan scope; the concurrent Phase-6 session's files and the pre-existing dirty/untracked files in both repos left untouched

---
*Phase: 07-r9-pci-ecam-virtio-net-devether*
*Completed: 2026-11-05*
