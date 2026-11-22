---
phase: 02-aarch64-toolchain-libc-9front-donor
plan: "05"
subsystem: infra
tags: [aarch64, 9front, toolchain, tc-03, execution-gate, qemu, tcg, gicv3, bootfs, devroot, abi-bisect-point]

requires:
  - phase: 02-aarch64-toolchain-libc-9front-donor (plan 02-03)
    provides: committed hello.7.out built by Harvey's vendored 7c/7l with frozen-ABI SVC stubs
  - phase: 02-aarch64-toolchain-libc-9front-donor (plan 02-04)
    provides: donor kernel (pin 35026610) built in-donor-tree by 7a/7c/7l + boot-9qemu.sh TCG harness proven to a live rc shell
provides:
  - build/aarch64/run-gate.sh — the TC-03 execution gate: bootdir-embeds the committed hello.7.out into a disposable guest donor rebake, boots it, runs /boot/hello at the rc prompt, asserts marker/status/fault-scan, appends evidence — idempotent (three consecutive PASS blocks)
  - build/aarch64/gate-evidence.log — committed execution proof: 9front banner, term% prompt, "hello aarch64 harvey" marker after the prompt, clean exit, sha256 equality between executed and committed binary
  - build/aarch64/README.md — harness documentation: D-10 pin record, per-script contracts, TCG/GICv3 probe evidence, bake-vs-ufs resolution, license credits
  - vendored libc/aarch64 memset/memmove reconciled to Harvey's ulong-n prototypes (binary rebuilt; hash 1c89e099…)
affects: [02-06 (scaffolding now unblocked), 02-07 (shell-subset gate reuses run-gate.sh), phase-04 (hello.7.out is the known-good ABI bisect point), phase-03 (GICv3/HVF forward note)]

actuals:
  tokens: 11180   # chars/4 over the realized diff (44721 chars text; binary hello.7.out diff placeholder negligible)
  tasks: 2
  commits: 3      # measured: git rev-list --count cdd0e9245..HEAD (ledger gsd-plan-head-before-02-05)
  plan_head_before: cdd0e9245121909340b993377fa9eb7f10ba2c40
  plan_head_after: d275b1ab9

tech-stack:
  added: []
  patterns:
    - "execution gate before scaffolding: marker-after-prompt + sha256-pair evidence committed before any aarch64/ arch dir (riscv64 vaporware rule, ROADMAP SC#3)"
    - "CONF bootdir → devroot embed (guest-scratch donor rebake) as the deterministic binary-delivery path; bootfs.paq is gone at the rc prompt (unmounted by bootrc)"
    - "paced echo-verified serial sends with wait-for-idle: the console input path drops burst-leading bytes while the shell is busy"

key-files:
  created:
    - build/aarch64/run-gate.sh
    - build/aarch64/README.md
  modified:
    - build/aarch64/gate-evidence.log
    - build/aarch64/hello.7.out
    - sys/src/libc/aarch64/memset.s
    - sys/src/libc/aarch64/memmove.s
    - sys/src/libc/aarch64/README

key-decisions:
  - "TC-03 gate SATISFIED and human-approved: the committed hello.7.out executes on the 9front donor kernel (banner → term% prompt → 'hello aarch64 harvey' → clean exit, sha256 1c89e099… both sides) — three consecutive PASS blocks; scaffolding (02-06) unblocked"
  - "Guest path resolution: the CONF=qemu bootdir embed lands at /boot/hello (devroot exposes only boot/); bootfs.paq is unmounted before the shell, so the paq path is NOT runnable at the rc prompt — mkpaqfs is not re-run"
  - "README records bake-vs-ufs resolution (research Open Question 2): bake for gates (deterministic), ufs-over-tcp for iteration (ethervirtio10 + ip stack; srv9.py already demonstrates host 9P2000/tcp to this guest)"

patterns-established:
  - "Evidence log contract: every gate run appends banner/prompt/marker/status + host+guest sha256 pair + fault-scan verdict; idempotent reruns append, never overwrite"
  - "Tracer feedback gate: a failing tracer never proceeds to expansion — halt with the guest serial log as bisect surface (toolchain-first diagnostic rule)"

requirements-completed: [TC-03]

coverage:
  - id: D1
    description: "TC-03 execution gate: committed hello.7.out executes on the 9front donor kernel under qemu -M virt TCG, marker line over serial, sha256-identical to the committed binary, evidence committed — precedes all aarch64/ scaffolding (ROADMAP SC#3)"
    requirement: TC-03
    verification:
      - kind: e2e
        ref: "sh build/aarch64/run-gate.sh — three consecutive PASS blocks in build/aarch64/gate-evidence.log (2026-11-01T04:28–04:29Z); sha256 grep gate + fault-scan grep gate green"
        status: pass
    human_judgment: true
    rationale: "The plan carries an explicit <human-check> (the vaporware-gate proof). APPROVED by the user at the tracer feedback gate, 2026-11-01 — the three PASS blocks in gate-evidence.log were accepted as TC-03 execution evidence."
  - id: D2
    description: "Harness README: D-10 pin record (35026610…), per-script contracts, TCG-only rationale with qemu probe evidence, GICv3/HVF Phase-3 forward note, bake-vs-ufs resolution, license credits"
    verification:
      - kind: unit
        ref: "grep gates: pin SHA, tcg, nsrun.sh, boot-9qemu.sh, GICv3, ufs — all present (Task 2 <verify> automated, PASS)"
        status: pass
    human_judgment: false

duration: 11h 18m wall (includes the human-verify checkpoint wait; active execution ~2h across two sessions)
completed: 2026-11-01
status: complete
---

# Phase 02 Plan 05: TC-03 Execution Gate — hello-world runs on the 9front donor kernel Summary

**Harvey's vendored 7c/7l hello-world EXECUTES on the 9front donor kernel under qemu TCG — banner, rc prompt, "hello aarch64 harvey", clean exit, sha256-identical binary, three consecutive PASS blocks committed — closing the TC-03 vaporware gate before any aarch64/ scaffolding exists, and leaving a self-documenting harness (README with the D-10 pin, TCG/GICv3 probe evidence, and bake-vs-ufs resolution).**

## Performance

- **Duration:** 11h 18m wall (03:47:50Z → 15:05:25Z; includes the human-verify checkpoint wait — the tracer gate was approved by the user between sessions)
- **Started:** 2026-11-01T03:47:50Z
- **Completed:** 2026-11-01T15:05:25Z
- **Tasks:** 2/2
- **Files created/modified:** 7

## Accomplishments

- **TC-03 gate closed, human-approved**: the committed `hello.7.out` (sha256 `1c89e0990e386ae503d9f513d3a6c1985222f772d1edb40016c19aa3881abb04`) ran on the 9front donor kernel as `/boot/hello`, printed `hello aarch64 harvey` after the `term% ` prompt, exited clean (`$status` empty), zero panic/trap lines — appended as **three consecutive PASS blocks** to `build/aarch64/gate-evidence.log`; the guest-staged binary's sha256 matches the committed one (evidence integrity, T-02-08)
- **ROADMAP Phase 2 SC#3 satisfied**: this execution commit precedes every aarch64/ arch-dir, libc-scaffolding, CPUS-entry, and pcc.c-entry commit — `git ls-files | grep ^aarch64/` is empty and no commit to date touches an `aarch64/` tree path
- **Phase 4's ABI bisect point established** (D-06 rationale): if this binary later fails on r9, the bug is in r9 — the frozen syscall ABI (R0/SP+0 slots/SVC) is proven against the reference kernel
- **Harness self-documenting**: `build/aarch64/README.md` records the D-10 pin (`350266104f1fed81c78c3d05fa44f523bec0ae4d`), every script's contract, the TCG-only pin with qemu 11.1.2 probe rejections (HVF+GICv2, HVF+cortex-a72, HVF+GICv3) and the donor trapinit GICv3 truth, the HVF-returns-with-GICv3 forward note plus D-03's 16K-page/CTR_EL0 notes for Phase 3, the bake-vs-ufs resolution, and license credits

## Task Commits

1. **Task 1: Tracer — hello-world executes on the 9front donor kernel** — `c4ee50c79` (fix: memset/memmove n-arg reconcile), `62351f81f` (feat: run-gate.sh + evidence)
2. **Task 2: Harness documentation (D-10 pin + constraints + iteration paths)** — `d275b1ab9` (docs)

**Plan metadata:** recorded below (final docs commit with SUMMARY/STATE/ROADMAP).

## Files Created/Modified

- `build/aarch64/run-gate.sh` — the TC-03 gate driver (sha256 record → guest bootdir staging → targeted rebake → boot → execute → assert → evidence append); header documents the flagged-assumption resolution (devroot vs bootfs.paq)
- `build/aarch64/gate-evidence.log` — three PASS blocks (banner, bootfs stamp, fingerprint, prompt, command, marker, exit status, fault scan, both sha256s)
- `build/aarch64/README.md` — harness documentation (pin, contracts, TCG rationale, GICv3 forward note, bake-vs-ufs, credits)
- `sys/src/libc/aarch64/memset.s`, `memmove.s` — n-arg width reconciled to Harvey's `ulong` prototypes (32-bit MOVW loads); README notes the reconcile
- `build/aarch64/hello.7.out` — rebuilt with the fixed asm (the hash recorded in the evidence log)

## Decisions Made

- **Guest path resolution (plan flagged assumption)**: the CONF=qemu bootdir embed lands at `/boot/hello` — devroot's root directory exposes only `boot/`, and the bootfs.paq is unmounted by bootrc before the shell, so a paq-baked binary would be gone at the rc prompt. mkpaqfs is not re-run; the embed goes through `mkrootall` + relink + aout2uimage into `9qemu-gate.u` (the proven `9qemu.u` untouched).
- **Bake-for-gates, ufs-for-iteration** (research Open Question 2 resolved in the README): bake is deterministic and zero-config for gates; ufs-over-tcp documented as the faster iteration path (donor CONF has ethervirtio10 + full ip stack; `srv9.py` already proves host 9P2000/tcp to this guest every boot).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] memset/memmove n-arg width reconciled with Harvey's ulong prototype**
- **Found during:** Task 1 (hello-world run produced wrong output/crash behavior)
- **Issue:** vendored 9front arm64 memset/memmove load `n` as 64-bit; Harvey's libc.h declares `ulong` n — on Harvey's LP64-ish calling convention the asm must consume the width Harvey's caller actually passes
- **Fix:** 32-bit MOVW n loads in both files; provenance note added to `sys/src/libc/aarch64/README`; `hello.7.out` rebuilt (new sha256 1c89e099…)
- **Files modified:** sys/src/libc/aarch64/memset.s, memmove.s, README; build/aarch64/hello.7.out
- **Verification:** marker line printed correctly on the donor kernel; sha256 recorded in evidence log matches the committed binary
- **Committed in:** `c4ee50c79`

**2. [Rule 3 - Blocking] Console input drops burst-leading bytes while the shell is busy**
- **Found during:** Task 1 (commands sent at the rc prompt intermittently lost their first bytes)
- **Issue:** the donor console input path drops burst-leading bytes when the shell is busy processing prior input
- **Fix:** run-gate.sh drives the prompt with paced, echo-verified sends and wait-for-idle between commands
- **Files modified:** build/aarch64/run-gate.sh
- **Verification:** three consecutive PASS runs with `/boot/hello` and the status echo both arriving intact
- **Committed in:** `62351f81f`

**3. [Rule 1 - Bug] send_verified truthiness fall-through in the serial driver**
- **Found during:** Task 1
- **Issue:** a truthiness fall-through in the send-verification helper could report success on an unverified send
- **Fix:** explicit status handling
- **Files modified:** build/aarch64/run-gate.sh
- **Verification:** PASS blocks show echo-verified command lines
- **Committed in:** `62351f81f`

**4. [Rule 3 - Blocking] devshell `date` resolves to the broken hostbin Plan 9 build**
- **Found during:** Task 1 (evidence timestamps)
- **Issue:** the nix devshell PATH surfaced a non-functional Plan 9 `date` from hostbin
- **Fix:** `/bin/date` pinned in the harness
- **Files modified:** build/aarch64/run-gate.sh
- **Verification:** evidence blocks carry correct UTC timestamps
- **Committed in:** `62351f81f`

**5. [Rule 3 - Blocking] guest-script `$HOME=/root` in the root namespace phase**
- **Found during:** Task 1 (guest rebake step)
- **Issue:** the root-namespace re-entry phase saw `$HOME=/root`, breaking relative donor paths
- **Fix:** explicit exports in the guest script
- **Files modified:** build/aarch64/run-gate.sh
- **Verification:** rebake produces `9qemu-gate.u` with hello embedded; gate boots and passes
- **Committed in:** `62351f81f`

---

**Total deviations:** 5 auto-fixed (2 Rule 1 bug-class, 3 Rule 3 blocking-class), plus the flagged-assumption resolution (devroot `/boot/hello` path) recorded as a decision.
**Impact on plan:** All auto-fixes necessary for gate correctness or task completion; no donor source edits (D-12 held — staging is guest-scratch only). No scope creep.

## Issues Encountered

- The plan's flagged assumptions resolved cleanly at execution time: boot-prompt response table from 02-04 held unchanged; bootdir staging mechanics discovered from the donor CONF/mkfile as expected. Scratch bisect probes lived in /tmp and were never committed (per instructions).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Plan 02-06 (arch scaffolding) is unblocked**: the execution commit precedes all scaffolding; 02-06 commits the identical `aarch64/mkfile` + include files used pre-gate and asserts sha256 equality
- **Plan 02-07 (shell subset)** reuses `run-gate.sh`'s bake-and-execute pattern (D-08: the harness doubles as the TC-05 acceptance gate)
- **Phase 4** inherits `hello.7.out` as the known-good ABI bisect point
- **Phase 3 forward note** (in README): HVF returns when r9 speaks GICv3; Apple 16K pages constrain guest granule choices; cache maintenance via `CTR_EL0` discovery

---
*Phase: 02-aarch64-toolchain-libc-9front-donor*
*Completed: 2026-11-01*

## Self-Check: PASSED

All 7 key files exist on disk; all 3 plan-scoped commits (`c4ee50c79`, `62351f81f`, `d275b1ab9`) present in git history. Plan-level verification at completion: Task 2 grep gates green (pin SHA, tcg, nsrun.sh, boot-9qemu.sh, GICv3, ufs all present); `test ! -e aarch64` ✓ and `git ls-files | grep -c ^aarch64/` = 0 ✓ (SC#3 ordering holds); evidence log carries three PASS blocks with sha256 equality ✓; tracer human-check approved by user 2026-11-01 ✓. No stubs: run-gate.sh is fully wired end-to-end; README documents real committed scripts only.
