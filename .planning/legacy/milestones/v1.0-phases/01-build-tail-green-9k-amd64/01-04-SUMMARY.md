---
phase: 01-build-tail-green-9k-amd64
plan: 04
subsystem: infra
tags: [plan9, minimega, qemu, kvm, tcg, serial-console, boot-harness, amd64, 9k, nix, lp64, 6c, 6l]

requires:
  - phase: 01-build-tail-green-9k-amd64
    provides: "01-01: green build producing amd64/9k10cpu.elf (this plan's precondition and boot target); 01-03: KERNDATE plumbing in nix/build.sh + nix/vm-build.sh (carried a latent shellcheck/date-format regression this plan fixed)"

provides:
  - "build/mm/boot-9k10cpu.mm — committed minimega boot script (six vm commands, KERNEL_ELF placeholder, amd64/KVM-only boundary comment, TCG fallback documented in-file)"
  - "build/mm/boot-9k10cpu.sh — capture/assert wrapper: probes /dev/kvm, honors MM_TCG/MM_QEMU/MM_BOOT_BUDGET/MM_LOG, splits the .mm at `vm start` so the socat serial capture attaches BEFORE the kernel runs, greps the captured log for the `Plan 9` banner, kills/flushes the VM on exit"
  - "BUILD-05 demonstration: TCG-mode boot of 9k10cpu.elf on the linux-aarch64 lima guest with boot-9k10cpu.log capturing the `Plan 9` banner + full MMU/APIC/PCI/memory init — explicitly accepted by the human (2026-10-31) as this phase's BUILD-05 proof"
  - "LP64 wide-constant truncation fixes in the amd64 toolchain (6c/swt.c zaddr, 6l/span.c oclass, 6c/cgen.c immconst) — the load-bearing discovery: without these the 9k10cpu kernel was silently miscompiled on LP64 hosts"
  - "KERNDATE derivation hardened: shellcheck-clean if-block + numeric-validated `date -n`/`date +%s` fallback in nix/build.sh and nix/vm-build.sh (01-03 latent regression, fixed here)"

affects:
  - "Phase 2+ (every later kernel build gets a one-command boot proof via build/mm/boot-9k10cpu.sh)"
  - "Phase 2 (deferred: dormant-arch toolchain siblings 4c/9c/9a/4l/9l carry the same LP64 truncation idiom — audit before trusting their output; deferred-items.md #1)"
  - "Phase 5 (deferred: post-banner boot-process pool panic means boot-to-userspace is not yet healthy; must be investigated before boot-to-shell is a verification signal; deferred-items.md #2)"

actuals:
  tokens: 5345      # chars/4 over the realized diff (21378 diff chars, base..HEAD at SUMMARY write)
  tasks: 2
  commits: 3        # MEASURED: git rev-list --count 0d4a1a248..HEAD at SUMMARY write (final docs commit lands after)
plan_head_before: 0d4a1a248c5c7cfb675c81ffa345c8d508ba4002
plan_head_after: 52bbb4c5b96e2f195eb2bd406a0e839cd5624277

tech-stack:
  added:
    - "minimega 3.3.0 operational use via the pre-existing committed derivation nix/minimega.nix (hash-pinned fetchFromGitHub + buildGoModule, vendored deps — zero registry fetches)"
  patterns:
    - "Boot-proof-by-serial-banner: no in-guest agent, no qemu stdio scraping — socat attaches to minimega's per-VM serial0 unix socket BEFORE `vm start`, and a `grep -q 'Plan 9'` on the captured log is the sole assertion (banner grep, not the timer, is the source of truth)"
    - "Split-.mm ordering: the .mm is split at its `vm start` line into launch/start fragments (awk) so capture attaches between launch and start — qemu always launches paused (-S) and the COM1 banner prints within the first milliseconds"
    - "Acceleration probing, never assuming: `test -w /dev/kvm` selects KVM; TCG requires explicit MM_TCG=1 and injects BOTH `vm config qemu <path>` and `vm config cpu qemu64` (minimega's default CPU 'host' is rejected by qemu without KVM)"
    - "minimega -e exit status is untrustworthy (exits 0 on command failure) — launch success is gated on the serial socket appearing, and the VM id is parsed from `vm info` by header name, never by column position"

key-files:
  created:
    - build/mm/boot-9k10cpu.mm
    - build/mm/boot-9k10cpu.sh
  modified:
    - sys/src/cmd/6c/swt.c
    - sys/src/cmd/6c/cgen.c
    - sys/src/cmd/6l/span.c
    - nix/build.sh
    - nix/vm-build.sh
    - .gitignore
    - .planning/phases/01-build-tail-green-9k-amd64/deferred-items.md

key-decisions:
  - "HUMAN DECISION (2026-10-31, blocking checkpoint): TCG-degraded serial capture ACCEPTED as BUILD-05's demonstration for this phase — the dev fleet has no Linux KVM host (darwin-aarch64 host; lima guest lacks /dev/kvm). The KVM path remains documented and runnable on any Linux x86_64 host: `nix build .#minimega && sudo build/mm/boot-9k10cpu.sh`. No KVM-host run is required for phase close."
  - "Post-banner `bad magic` pool panic stays deferred (deferred-items.md #2): BUILD-05's banner criterion is met; the boot-to-userspace handoff is explicitly out of this plan's scope and is flagged as a Phase 2 verification-signal risk"
  - "Serial capture attaches BEFORE `vm start` via the launch/start .mm split — a capture started after start loses the early COM1 banner (chardev output with no client connected is discarded); this is the plan's Flagged Assumption resolved by construction"
  - "The LP64 truncation fixes change only the comparison/cast sites (zaddr, oclass, immconst) — no object-format or ABI changes; dormant-arch siblings with the same idiom were inventoried and deferred rather than fixed (amd64-only green gate scope)"

patterns-established:
  - "See tech-stack.patterns"

requirements-completed: [BUILD-05]

coverage:
  - id: D1
    description: "build/mm/boot-9k10cpu.mm + build/mm/boot-9k10cpu.sh committed: six vm commands in launch order with KERNEL_ELF placeholder and boundary comment; wrapper probes /dev/kvm, honors MM_TCG/MM_QEMU/MM_BOOT_BUDGET, attaches serial capture before vm start, greps `Plan 9`, kills/flushes on exit"
    requirement: BUILD-05
    verification:
      - kind: static
        ref: "plan verify gates: bash -n clean on the wrapper; grep gates for 'vm config kernel', 'serial-ports 1', 'vm launch kvm', 'vm start' in the .mm and 'Plan 9' in the wrapper — all pass"
        status: pass
    human_judgment: false
  - id: D2
    description: "nix build .#minimega produces a runnable minimega binary on linux (committed derivation, tag 3.3.0 pinned)"
    requirement: BUILD-05
    verification:
      - kind: e2e
        ref: "lima guest (linux-aarch64): nix build .#minimega exits 0; result/bin/minimega executable and used by the harness run below"
        status: pass
    human_judgment: false
  - id: D3
    description: "Boot demonstration: 9k10cpu.elf boots under minimega with the `Plan 9` banner captured in boot-9k10cpu.log; execution mode (TCG-degraded) explicitly accepted as this phase's BUILD-05 proof"
    requirement: BUILD-05
    verification:
      - kind: manual_procedural
        ref: "checkpoint:human-verify (blocking) — TCG run on the lima guest produced boot-9k10cpu.log (4003 bytes, banner at line 4); human verdict 2026-10-31: 'Accept TCG demonstration'"
        status: pass
    human_judgment: true
    rationale: "The plan gates BUILD-05 on a human accepting the execution mode (KVM host vs TCG-degraded) — the dev fleet has no KVM host, so mode acceptance is inherently a human judgment call, exercised at the blocking checkpoint."

duration: 60min active + ~9.7h checkpoint wait (blocking human-verify)
completed: 2026-10-31
status: complete
---

# Phase 01 Plan 04: minimega Smoke Harness (BUILD-05) Summary

**Committed minimega harness (`boot-9k10cpu.mm` + KVM-probing capture/assert wrapper) boots the Phase-1 `9k10cpu.elf` to the `Plan 9` banner with the serial log captured from minimega's per-VM socket — demonstrated under TCG on the lima guest and explicitly accepted by the human (2026-10-31) as BUILD-05's proof; the load-bearing discovery en route was an LP64 wide-constant truncation bug class in 6c/6l that was silently miscompiling the kernel on LP64 hosts.**

## Performance

- **Duration:** ~60 min active execution + ~9.7 h blocked at the human-verify checkpoint awaiting the mode verdict
- **Started:** 2026-10-31T02:20Z (after 01-03 close at 02:15Z)
- **Completed:** 2026-10-31T12:56Z (checkpoint verdict applied, plan closed)
- **Tasks:** 2/2 (Task 1 auto; Task 2 checkpoint:human-verify — approved)
- **Files modified:** 2 created + 7 modified (392 insertions, 17 deletions across 3 task commits)

## Accomplishments

- `build/mm/boot-9k10cpu.mm` (39 lines): the six-command minimega boot script with the `KERNEL_ELF` placeholder, amd64/KVM-only BUILD-05 boundary comment, and an in-file documented TCG fallback (`vm config qemu` + `vm config cpu qemu64`)
- `build/mm/boot-9k10cpu.sh` (247 lines, shellcheck-clean): probes `/dev/kvm`, honors `MM_TCG`/`MM_QEMU`/`MM_BOOT_BUDGET`/`MM_LOG`/`MM_BASE`, splits the .mm at `vm start` so the socat capture attaches before the kernel runs, waits an env-overridable budget (60s KVM / 600s TCG), asserts `grep -q 'Plan 9'`, and kills/flushes the VM on exit
- Boot demonstrated for real: TCG mode on the linux-aarch64 lima guest produced `boot-9k10cpu.log` with the banner plus clean MMU/APIC/PCI/memory init (`508M memory: 1936K+230M kernel, 276M user`) — see Demonstrated Evidence
- The harness surfaced and the plan fixed the phase's deepest bug: LP64 wide-constant truncation across `6c/swt.c` (zaddr), `6l/span.c` (oclass), `6c/cgen.c` (immconst) — without these, the 9k10cpu kernel was being silently miscompiled
- 01-03's latent KERNDATE regression repaired (shellcheck SC2015 broke fresh `.#build` app builds; plan9port `date +%s` fallback returned garbage) — `nix run .#build` is green end-to-end again
- KVM path documented for any future Linux x86_64 host: `nix build .#minimega && sudo build/mm/boot-9k10cpu.sh`

## Task Commits

Each task was committed atomically:

1. **Task 1: Commit the minimega boot script + serial capture/assert wrapper; verify package builds on linux** — `52bbb4c5b` (feat; declared files: build/mm/boot-9k10cpu.mm, build/mm/boot-9k10cpu.sh, .gitignore) — plus two enabling Rule-1 fix commits made during the task: `a63a6a90a` (fix: LP64 wide-constant truncation across 6c/6l) and `df615a33a` (fix: shellcheck-clean + numerically-validate KERNDATE derivation)
2. **Task 2: Boot demonstration checkpoint (human-verify, blocking)** — no code commit; evidence artifact `boot-9k10cpu.log` is gitignored by design. Human verdict 2026-10-31: **"Accept TCG demonstration."**

**Plan metadata:** committed after this summary (docs: complete minimega smoke harness plan)

## Demonstrated Evidence (BUILD-05)

**Mode:** TCG-degraded (explicitly accepted). Host reality per RESEARCH Open Question 1: the dev fleet has no Linux KVM host — primary dev machine is darwin-aarch64 and the lima guest lacks `/dev/kvm`. Run: lima guest, `MM_TCG=1 MM_QEMU=$(command -v qemu-system-x86_64) MM_BOOT_BUDGET=600 bash build/mm/boot-9k10cpu.sh`, minimega 3.3.0 from `nix build .#minimega`, kernel = Phase-1-built `amd64/9k10cpu.elf`.

**Captured serial log** (`boot-9k10cpu.log`, gitignored runtime artifact; regenerate via the wrapper):

```
Booting from ROM..
Plan 9
mmuinit: vmstart 0xfffffffff0000000 vmunused 0xfffffffff02f4000 vmunmapped 0xfffffffff0400000 vmend 0xfffffffffe000000
sys->pd 0x108003 0x108023
...
apic0: hz 990642660 max 9906426 min 99064 div 1
pcirouting: BIOS workaround: PCI.0.1.3 at pin 1 link 96 irq 10 -> 9
508M memory: 1936K+230M kernel, 276M user
```

The assertion target — the string `Plan 9` printed unconditionally on COM1 at kernel main (`sys/src/9k/k10/main.c:191`, console enabled at `:182`) — appears at line 4. MMU, APIC, PCI routing, and memory sizing all initialize correctly.

**Post-banner state (deferred, not a plan failure):** the embedded-root `boot` process then reports `bad magic` and dies in a pool panic (`panic: pool panic`, `boot 1: suicide: sys: trap: fault read addr=0x0 pc=0x20ac98`). This is out of BUILD-05's scope (the banner criterion is met) and is recorded as deferred-items.md #2 with the full context; it flags that boot-to-userspace is not yet healthy and must be investigated before Phase 2 relies on boot-to-shell as a verification signal.

**KVM path (documented, unexercised this phase):** on any Linux x86_64 host with `/dev/kvm` — `nix build .#minimega`, ensure `amd64/9k10cpu.elf` exists, `sudo build/mm/boot-9k10cpu.sh`; expect exit 0 within ~60 s. minimega never emits `-enable-kvm`, so the same harness runs both modes; the wrapper probes and selects.

## Decisions Made

- **Human decision at the blocking checkpoint (2026-10-31): "Accept TCG demonstration."** The TCG-captured banner satisfies BUILD-05's demonstration requirement for this phase; no KVM-host run is required for phase close. The checkpoint record (this section + D3 in coverage) states the mode and the acceptance, per the plan's acceptance criteria.
- Post-banner pool panic stays deferred rather than chased inside this plan — BUILD-05's letter is the harness + banner capture; scope discipline keeps the boot-to-userspace investigation visible (deferred-items.md #2) instead of absorbed.
- LP64 fixes kept minimal: repair the truncation at the three comparison/cast sites in the amd64 toolchain only; the same idiom in dormant-arch tools (4c/9c/9a/4l/9l) was inventoried and deferred (deferred-items.md #1) — amd64-only green gate scope held.
- No flake app registration and no CI wiring this phase (plan-explicit deferrals).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Repaired LP64 wide-constant truncation across 6c/6l**
- **Found during:** Task 1 (harness bring-up: the booted kernel's early behavior exposed toolchain miscompilation on LP64 hosts)
- **Issue:** three sites truncated `vlong` through `long` on LP64 hosts — `6c/swt.c` (zaddr case-value range test), `6l/span.c` (oclass 64-bit immediate fit test), `6c/cgen.c` (immconst) — silently miscompiling wide constants in the kernel
- **Fix:** corrected the comparison/cast handling at each site so 64-bit values survive on LP64 hosts; no object-format or ABI change
- **Files modified:** sys/src/cmd/6c/swt.c, sys/src/cmd/6c/cgen.c, sys/src/cmd/6l/span.c
- **Verification:** kernel rebuilt with the fixed toolchain boots to the banner with correct MMU/memory init (evidence above); same-idiom dormant-arch siblings inventoried into deferred-items.md #1
- **Committed in:** `a63a6a90a`

**2. [Rule 1 - Bug] shellcheck-clean + numerically-validated KERNDATE derivation**
- **Found during:** Task 1 (fresh `nix run .#build` exercise — 01-03's verification used direct namespace runs, so the app-level failure was invisible until now)
- **Issue:** 01-03's KERNDATE line in nix/build.sh + nix/vm-build.sh tripped writeShellApplication's shellcheck (SC2015), failing any fresh `.#build` app build; and the `date +%s` fallback resolves to plan9port's date (hostbin first on PATH), which prints garbage for GNU-style format args — the guest's no-git fallback produced a date STRING, not an epoch
- **Fix:** restructured as an if-block (shellcheck-clean) and numerically validated the fallback (`date -n` in-idiom, `date +%s` only when it yields a number)
- **Files modified:** nix/build.sh, nix/vm-build.sh
- **Verification:** `nix run .#build` green end-to-end; recorded as deferred-items.md #3 (latent-regression note, fixed)
- **Committed in:** `df615a33a`

### Plan-sanctioned fallback exercised

**3. [Checkpoint fallback] TCG-degraded demonstration accepted in lieu of a KVM host**
- **Found during:** Task 2 (blocking human-verify checkpoint)
- **Issue:** BUILD-05's letter prefers a Linux KVM host demonstration; the dev fleet has none (RESEARCH Open Question 1 anticipated exactly this and recommended the TCG fallback)
- **Resolution:** TCG run executed per the plan's fallback path; human explicitly accepted the TCG-degraded capture as this phase's BUILD-05 proof on 2026-10-31; KVM path documented for any future Linux x86_64 host

---

**Total deviations:** 2 auto-fixed (both Rule 1 bugs) + 1 plan-sanctioned checkpoint fallback
**Impact on plan:** The LP64 toolchain fixes were the load-bearing discovery of the plan — the harness immediately paid for itself by surfacing a miscompiled kernel. No scope creep: dormant-arch siblings deferred, post-banner panic deferred, both recorded.

## Issues Encountered

- The dev fleet has no KVM host (known from RESEARCH OQ-1): resolved by the plan's own TCG fallback + human acceptance — not a surprise, but worth restating because every later phase's boot proof inherits this constraint until a KVM host exists.
- minimega's `-e` exits 0 even when commands fail, and qemu launches paused — both handled by construction (socket-appearance gating, launch/start split) per RESEARCH Pitfall 6.

## Known Stubs

None. All harness behavior is fully wired; `boot-9k10cpu.log` is an intentionally gitignored runtime artifact, not a stub.

## Threat Flags

None. The plan's threat register holds: T-01-05 (minimega-as-root) accepted with the harness documented for dedicated KVM hosts/dev VMs; T-01-06 (log disclosure) accepted — the boot log contains no secrets and is gitignored; T-01-SC (package legitimacy) mitigated — no new packages; minimega 3.3.0 was already a hash-pinned, vendored-deps derivation. No new network endpoints, auth paths, or trust-boundary surface introduced.

## User Setup Required

None outstanding. The plan's `user_setup` item (linux-kvm-host access) was resolved at the checkpoint by the human's explicit acceptance of the TCG demonstration — no KVM host is required for phase close. Any future Linux x86_64 host can exercise the KVM path with `nix build .#minimega && sudo build/mm/boot-9k10cpu.sh`.

## Next Phase Readiness

- **BUILD-05 complete → Phase 1's five requirements (BUILD-01…05) are all satisfied**; the smoke harness gives every later kernel build a one-command boot proof.
- Phase 2 entry points ready: the `aarch64` naming quarantine (01-02) and the green amd64 factory (01-01) are the donors; the toolchain app path is proven.
- Watch-items handed forward: deferred-items.md #1 (dormant-arch LP64 idiom audit — relevant the moment Phase 2 vendors 7c/7l), #2 (post-banner pool panic — investigate before boot-to-shell becomes a verification signal).
- Guest store carries a realized minimega 3.3.0 — subsequent harness runs in the lima guest start warm.

---
*Phase: 01-build-tail-green-9k-amd64*
*Completed: 2026-10-31*

## Self-Check: PASSED

- SUMMARY.md exists at .planning/phases/01-build-tail-green-9k-amd64/01-04-SUMMARY.md
- All three Task 1 commits found in git history (a63a6a90a, df615a33a, 52bbb4c5b)
- Both declared harness files exist (build/mm/boot-9k10cpu.mm, build/mm/boot-9k10cpu.sh)
- Accepted evidence artifact boot-9k10cpu.log present with the `Plan 9` banner at line 4 (gitignored, regenerable via the wrapper)
- Commit count measured from on-disk ledger: `git rev-list --count 0d4a1a248..HEAD` = 3 at SUMMARY write (docs commit lands after)
