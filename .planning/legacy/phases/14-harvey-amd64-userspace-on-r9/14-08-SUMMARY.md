---
phase: 14-harvey-amd64-userspace-on-r9
plan: 08
subsystem: tier-machinery
tags: [tiers, subset-build, mkimage, k10t1, donor-venue, r9, x86_64, abi-witness, xexec-04, sse, eager-stack, flake]
duration: 70 min active across two executors (wall ~7h — the Task-1→2 gap is a host usage-limit kill; continuation 14:54Z→15:23Z)
completed: 2026-11-09
requires:
  - "14-07-complete r9 pin e5def72 (the compose pipeline + smoke app + iso-builder export — the tier gate composes through the same machinery)"
provides:
  - "the amd64 tier machinery end-to-end (XEXEC-04): build/x86_64/tiers/*.txt allowlists + the five objtype-gated mk filter arms (Task 1), the tier-driven subset-build.sh (T0-ART/T1-ART/TIER-BUILD-OK, D-07 fresh-archive, link-only singles), mkimage's TIER bake (bootfs-tier.paq: chain tree + 37 h* binaries computed from the tier data + /lib/tier1-sweep.rc + sentinel + nm-input.6), and the k10t1 venue extended with the r9-x64 execution leg"
  - "THE LIVE ABI WITNESS: ONE /amd64/bin set (the 38-member sha256 manifest build-9k10t1's r9 leg pins; mkimage's tier bake digest-verifies — T-14-15 structural) executes on BOTH kernels: k10t1 T1-VENUE-PASS ok=36 skip=24 fail=0 AND r9-x64 T1-R9X64-PASS ok=36 skip=2 fail=0, with IDENTICAL 36-marker T1-OK sets proven at gate time (diff-empty donor-vs-gate captures)"
  - "the r9-x64 tier gate as a flake app: build/x86_64/tier1-r9.expect + nix/tier1-r9-x64.sh + boot-r9-tier1-x64 — GREEN TWICE CONSECUTIVELY via the full pipeline (tier build → tier bake → r9's iso-builder → expect), expectations COMPUTED from the committed tier data + sweep (36 OK + 2 SKIP, never a second hardcoded list)"
  - "build/x86_64/tier1.ledger — the 37 r9-x64-venue execution-marker lines (35 T1-OK:r9x64 + the ipconfig/factotum SKIPs with the 06-07 measured reasons), sha256 naming the gate-run /amd64/bin artifacts (the witness-manifest bytes); both venue captures retained in the phase evidence dir"
  - "TWO r9 kernel gaps root-caused and fixed (r9@e417660, push-before-pin, pin advanced lock-only in 5294f27b8): (1) user SSE never armed — CR4.OSFXSR/OSXMMEXCPT at both l.S cr4 sites + fp_reset_for_exec (fninit+ldmxcsr 0x1f80 per exec, the donor's fpstate=FPinit equivalent); (2) the 32-page eager stack too shallow for cmp's 2×64KiB frame — STACK_EAGER_PAGES 32→64"
  - "the tier-0 closure completed by the gate discipline: libString (du), libthread + lib9p (factotum's dat.h autolibs) appended append-only with provenance — the tier lists grew exactly where the build gate caught the gaps"
affects:
  - "../r9/x86_64/src/{l.S,exec.rs} (the two kernel fixes)"
  - "build/mm/{build-9k10t1.sh,boot-9k10t1-sweep.sh}, build/x86_64/{mkimage.sh,subset-build.sh,tier1-sweep.rc,tier-termrc,hello.c,nm-input.6,tier1.ledger,tier1-r9.expect}, nix/tier1-r9-x64.sh, flake.nix, flake.lock, .gitignore, build/x86_64/tiers/tier0-libs.txt"
  - "14-09 (the net smoke consumes the same compose machinery; Phase 15's fuller tiers append onto the tier data + the ledger)"
tech-stack:
  added: []
  patterns:
    - "a both-kernels execution witness must pin its input set structurally, not incidentally: the venue kernel's build writes a digest manifest of the exact /amd64/bin set it embedded, and the other venue's image bake VERIFIES that manifest before staging — a drifted member between builds reds the bake, never the eye"
    - "ldmxcsr takes a MEMORY operand only (m32) — the register form is invalid; the reset value rides the stack and the inline asm dereferences it (asm_sub_register points the way but {reg} alone cannot work)"
    - "Plan 9 objects embed the ABSOLUTE source path in their symbol table — a fixture object is only byte-reproducible when compiled from the same path (nm-input.6's documented 6c command)"
    - "the 9k kernel build on a passwordless host needs more of the tree than userspace builds: /lib/keyboard (portmkfile's mklatinkbd→latin1.h rule) and the nvram rootdir stub (build/scripts/build:48-49's rule) — the full in-OS build creates both as untracked artifacts; a subset-only host must stage them or mk dies 'don't know how to make'"
    - "a dedicated venue image beats a typed-line gate for sweep symmetry: the tier bake's termrc override runs the sweep from the boot flow exactly as the donor's k10t1.rc does — both venues then need no devcons reads and the harness can drive plain -serial file:"
key-files:
  created:
    - build/x86_64/subset-build.sh
    - build/x86_64/tier1-sweep.rc
    - build/x86_64/tier-termrc
    - build/x86_64/hello.c
    - build/x86_64/nm-input.6
    - build/x86_64/tier1.ledger
    - build/x86_64/tier1-r9.expect
    - nix/tier1-r9-x64.sh
    - ../r9: no new files (l.S + exec.rs modified)
  modified:
    - build/mm/build-9k10t1.sh
    - build/mm/boot-9k10t1-sweep.sh
    - build/x86_64/mkimage.sh
    - build/x86_64/tiers/tier0-libs.txt
    - flake.nix
    - flake.lock
    - .gitignore
    - ../r9/x86_64/src/l.S
    - ../r9/x86_64/src/exec.rs
key-decisions:
  - "The witness is STRUCTURAL: build-9k10t1's r9 leg pins a 38-member sha256 manifest of the exact /amd64/bin set the k10t1 kernel embedded; mkimage's tier bake verifies every digest before staging the r9 image — the identical-6.outs claim is enforced at build time, and the gate-time marker parity (identical 36-marker T1-OK sets, diff-proven) is the execution half of the proof"
  - "The tier image runs the sweep from the boot flow (tier-termrc staged as rc/bin/termrc), mirroring the donor's k10t1.rc — NO typed line (the aarch64 gate's one-typed-line accommodation retired for the dedicated image); the gates assert markers only, and the % prompt is asserted AFTER T1-DONE (the sweep's child rc never kills init's shell)"
  - "The sweep's SKIP expectations are extracted VERBATIM from the committed tier1-sweep.rc's own `echo T1-SKIP` lines (the aarch64 r9-gate rule, carried) and the OK set from the tier data minus SKIP-named members plus float-d01 — computed at classify time in all three consumers (mm r9 arm, nix driver, expect post-pass), so a stale baked sweep reds the converse assertion"
  - "Two r9 fixes landed as one push-before-pin cycle (r9@e417660 → push → 5294f27b8 lock-only): arming user SSE is a two-line l.S change BUT the donor-mirroring per-exec FP reset (fninit+ldmxcsr) rides with it so each image starts at power-on defaults — the era limit (no per-proc FXSAVE/XRSTOR; correct while the kernel stays SSE-free and FP users never interleave) is recorded in exec.rs and below"
  - "The donor venue build gained the passwordless venue (the 14-05 userns chroot scaffold wrapping the SAME nuke-sandwich core — behavior-identical, only the namespace entry differs), plus the nvram stub and /lib binds the full in-OS build normally provides; the 9k arm's default behavior (no r9 env set) is byte-preserved"
  - "nm-input.6 is a committed fixture (the nm-input.7 discipline) — gitignore-negated against the tree-wide *.6 rule, with the byte-reproduction command documented in the sweep header (the absolute-path 6c compile)"
requirements-completed: [XEXEC-04]
commits: 4
plan_head_before: eda2e70edf38c6b4968337cad325e578a09c8b21   # harvey repo; r9 ledger e5def72
plan_head_after: b50a0e1ca252386b19c874e26f9f18a1f27e5762   # harvey repo; r9 e417660
actuals:
  tokens: 35000   # chars/4 over the realized diff (harvey 135229 + r9 5489 chars, evidence captures included)
  tasks: 3
  commits: 5      # harvey (7b515dcee 5294f27b8 bce0f69f1 b50a0e1ca) + r9 (e417660) + this docs commit
status: complete
coverage:
  - deliverable: "Task 1 — the amd64 tier data + the five mk filter arms (tracer, prior executor)"
    verification:
      - kind: command
        ref: "test -f both tier files; awk keep-rule over tier1-cmds.txt yields 40 entries; grep -c 'x86_64/tiers' = 3/2/2/2/2 across the five mkfile sites; git diff --stat 7b515dcee..HEAD -- build/aarch64/ empty (re-verified at resume)"
        status: pass
    human_judgment: false
  - deliverable: "Task 2 — subset-build.sh + the k10t1 venue r9 leg: both-kernels sweep green"
    verification:
      - kind: command
        ref: "sh build/x86_64/subset-build.sh: exit 0, TIER-BUILD-OK, 15 T0-ART + 40 T1-ART markers, sys/src git-clean after (droppings targeted); sh build/mm/boot-9k10t1.sh: 9k10cpu + 9k10t1 built, embedded root files: 43, k10 tree git-clean, 38-member witness manifest written; mkimage tier bake: witness manifest verified, 37 h* binaries, magic 529ab12b/6b46e688"
        status: pass
      - kind: command
        ref: "the both-halves run: 9k leg T1-VENUE-PASS ok=36 skip=22+2libtest fail=0 done=yes (36 T1-OK lines in the harness log — the D-16 self-print); r9 leg T1-R9X64-PASS ok=36 skip=2 fail=0 done=yes; diff of the two captures' T1-OK name sets: EMPTY (36 identical markers)"
        status: pass
      - kind: command
        ref: "acceptance: grep -c 'T1-ART' subset-build.sh = 5; '00008a97|fresh' = 5; grep -ci 'r9' boot-9k10t1-sweep.sh = 91 (baseline 3 at 7b515dcee, recorded); git diff --stat HEAD -- build/aarch64/subset-build.sh empty"
        status: pass
    human_judgment: false
  - deliverable: "Task 3 — the r9-x64 tier gate: green twice, markers + ledger committed"
    verification:
      - kind: command
        ref: "THE GATE: nix run .#boot-r9-tier1-x64 TWICE CONSECUTIVELY — both runs ALL STAGES PASS (banner/paqfs/version → float-canary → class markers → T1-DONE → prompt; post-pass: 36 expected T1-OK present, T1-SKIP set EQUAL to the sweep's declared 2, zero T1-FAIL, zero panic); capture retained at build/x86_64/tier1-r9-x64.log (grep -q T1-OK ✓) and copied to the phase evidence dir"
        status: pass
      - kind: command
        ref: "witness parity at gate time: donor capture vs gate capture T1-OK sets diff-empty (36=36); the 37 ledger lines' sha256 values match the witness manifest's /amd64/bin digests"
        status: pass
      - kind: command
        ref: "the r9 regression battery at the fix commit: q35exec, q35shell, q35, q35net, aarch64 virt all PASS; 101 port tests; clippy clean (exec.rs zero warnings); acceptance greps (T1-DONE|T1-OK ×16 in the expect, boot-r9-tier1-x64 ×2 in flake, driver exists, aarch64 tier1-r9.expect + nix/tier1-r9.sh untouched)"
        status: pass
    human_judgment: false
---

# Phase 14 Plan 8: The Tier Machinery on amd64 — Both-Kernels Execution Proof Summary

Re-keyed the whole tier machinery to objtype=amd64 and proved it on BOTH kernels: the append-only allowlists + five objtype-gated mk filter arms as data, the tier-driven subset-build driver with its artifact-marker discipline (15 T0-ART + 40 T1-ART, TIER-BUILD-OK), the k10t1 donor venue extended with an r9-x64 execution leg, and the in-guest r9 tier gate as the `boot-r9-tier1-x64` flake app — GREEN TWICE CONSECUTIVELY. The ABI witness is live and structural: ONE /amd64/bin set (the sha256 manifest the venue-kernel build pins and the tier bake verifies) executed on Harvey's own 9k kernel (T1-VENUE-PASS ok=36) and on r9-x86_64 (T1-R9X64-PASS ok=36) with IDENTICAL 36-marker T1-OK sets — D-01's both-kernels consequence made executable proof, not a document. The gate did its job before it went green: the first r9 sweep surfaced TWO real kernel defects as classified T1-FAIL captures (never silent skips, never hangs) — user SSE was never armed (CR4.OSFXSR absent; awk's float canary and seq's memcpy SSE tail #UD'd) and the 32-page eager stack was 392 bytes too shallow for cmp's 2×64KiB frame — both fixed donor-faithfully in r9@e417660 with the full Phase-13/14 regression battery green, push-before-pin held.

**Duration:** 70 min active across two executors (wall ~7h; the Task-1→Task-2 gap is a host usage-limit kill — this continuation ran 14:54Z→15:23Z)
**Tasks:** 3/3 complete (Task 1 tracer by the prior executor, re-verified at resume)
**Files:** 8 created + 7 modified (harvey) + 2 modified (r9) + 2 evidence captures

## Commits

| Repo | Hash | Message |
|------|------|---------|
| harvey | 7b515dcee | feat(14-08): the amd64 tier data + the five objtype-gated mk filter arms (XEXEC-04) — Task 1 (prior executor) |
| r9 | e417660 | fix(14-08): arm user SSE + grow the eager stack — the tier-1 sweep's two kernel gaps |
| harvey | 5294f27b8 | chore(14-08): advance the r9 pin to e417660 (user SSE + eager-stack fixes — push-before-pin) |
| harvey | bce0f69f1 | feat(14-08): the x64 subset driver + the both-kernels venue legs — the live ABI witness (XEXEC-04) |
| harvey | b50a0e1ca | feat(14-08): the r9-x64 tier gate — tier1-r9-x64 green twice, execution markers committed (XEXEC-04) |
| harvey | (docs) | docs(14-08): this file + state |

## Accomplishments

### Task 1: the amd64 tier data + mk filter arms (tracer — prior executor, re-verified)

- `build/x86_64/tiers/{tier0-libs,tier1-cmds}.txt` re-keyed from the aarch64 headers: the append-only discipline documented in-file, the membership anchored to the k10t1-proven 35-command embed set, the exclusions (write-flow / build-chain / toolchain classes) documented transparently for Phase 15 appends.
- The five one-line filter arms (`sys/src/mkfile`, `sys/src/cmd/mkfile`, `cmd/ip`, `cmd/auth`, `cmd/auth/factotum`) reading `$HARVEY/build/x86_64/tiers/*.txt` through the same awk keep-rule, COVERAGE_SWEEP escape preserved, aarch64 byte-untouched.
- Resume verification: all Task-1 acceptance criteria re-run green (tracer verified end-to-end before expansion).

### Task 2: subset-build.sh + the both-kernels venue legs

- **subset-build.sh** re-keyed complete: the tier-file loop with the D-07 fresh-archive rule, T0-ART/T1-ART sha256 markers, TIER-BUILD-OK (mk exit never trusted), the 14-07 venue lesson carried (singles link via `mk 6.<cmd>` + driver cp — never the $BIN recipe that stages `._cp` in the userns chroot), targeted droppings cleanup (rc/awk/grep/paqfs/ipconfig/factotum + units' yacc products + 6.init), and the bind additive block (the donor embed's member, link-only).
- **The tier-0 closure grew exactly where the gate caught it**: run 1 died at du (`6l: cannot open libString.a`) → libString appended; run 2 died at factotum (`cannot open libthread.a`) → libthread + lib9p appended (dat.h's `<thread.h>`/`<9p.h>` autolibs — the same pair the aarch64 v1.0 closure names); run 3 green with the full 40-marker T1 set.
- **build-9k10t1.sh** extended additively: the nuke-sandwich core factored to a shared script wrapped by BOTH namespace venues (RUN_BUILD_IN_PRIVATE_NAMESPACE under passwordless sudo; the 14-05 userns chroot scaffold otherwise — plus the nvram stub rule and the /lib bind the full in-OS build normally provides), and the r9 leg writing the 38-member witness manifest.
- **boot-9k10t1-sweep.sh** gained the r9 arm (`T1_R9_ISO` / `T1_LEG=r9`): the marker contract COMPUTED from the x64 tier data, the SKIP set extracted from the sweep's own lines, boundary-anchored regexes, the converse assertion, the failure digest — and green runs now self-print the marker digest (D-16; the plan's own verify greps the harness log). The 9k arm's default behavior is byte-preserved.
- **mkimage.sh section 9** — the TIER bake: the chain tree + 37 h* binaries (computed from the tier data — never a hardcoded staging list) + `/lib/tier1-sweep.rc` + the byte-pinned sentinel + nm-input.6, the tier-termrc override, the witness-manifest digest verification, KERNDATE determinism, magic asserts.
- **BOTH VENUES GREEN, PARITY PROVEN**: k10t1 ok=36 skip=24 fail=0; r9-x64 ok=36 skip=2 fail=0; the two captures' T1-OK name sets diff EMPTY.

### Task 3: the r9-x64 tier gate — green twice

- **tier1-r9.expect**: the staged boot → the sweep from the boot flow (float canary FIRST) → T1-DONE → the % prompt → the whole-file post-pass (computed OK set, SKIP-set equality, zero FAIL/panic).
- **nix/tier1-r9-x64.sh + the `boot-r9-tier1-x64` app**: the full pipeline (tier build → bake → r9's iso-builder → expect) with computed expectations (36 OK + 2 SKIP), capture hygiene, the R9_RUN_TAG survivor sweep.
- **`nix run .#boot-r9-tier1-x64` green twice consecutively**; the capture retained at build/x86_64/tier1-r9-x64.log and in the phase evidence dir.
- **The 37 ledger lines** (35 T1-OK:r9x64 + ipconfig/factotum SKIPs) with sha256 values matching the witness manifest's artifacts.

## The two r9 kernel defects the first sweep surfaced (and the fixes)

**1. User SSE was never armed (r9@e417660, l.S + exec.rs).** The first r9 sweep run classified 4 T1-FAILs — awk, float-d01 (awk again), seq: `user trap: unhandled vector 6` (#UD) at the same SSE-bearing pcs. r9's l.S wrote only PGE|PAE to CR4; without OSFXSR every SSE instruction #UDs at decode — 6c's libc memcpy.s uses movdqu tails and the fmt family prints doubles through SSE. The tier1 data planted the canary for exactly this ("Pitfall 7: user SSE under r9 needs CR4.OSFXSR ownership proven, not assumed") and the canary did its job. Fix: OSFXSR|OSXMMEXCPT at both l.S cr4 sites (boot + AP), plus fp_reset_for_exec — fninit + ldmxcsr 0x1f80 at every install, so each image starts at FP power-on defaults (the donor's fpstate=FPinit equivalent; ldmxcsr is memory-operand-only, the value rides the stack). Era limit recorded: no per-proc FXSAVE/XRSTOR yet — correct while the kernel stays SSE-free and FP users never interleave (the sequential sweep); real FP context switching lands with the pager era.

**2. The eager stack was 392 bytes too shallow for cmp (r9@e417660, exec.rs).** cmp.c declares `uchar buf1[65536], buf2[65536]` — a 128KiB+ frame against the 14-06-era 32-page (128KiB) seed: `vector 14` at USTKTOP-0x10188, just past the seed's bottom edge. Fix: STACK_EAGER_PAGES 32→64 (256KiB, 2× headroom over the deepest current consumer); the 16MiB USTKSIZE policy bound and the pager follow-up unchanged.

Both fixes proven by the full battery at the fix commit: q35exec, q35shell, q35, q35net, aarch64 virt, 101 port tests, clippy clean.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing-critical] The tier-0 closure was incomplete (three libs)**
- **Found during:** Task 2's subset-build runs 1-2
- **Issue:** The Task-1 seed missed libString (du.c's autolib) and libthread + lib9p (factotum's dat.h autolibs) — the build gate caught each at link time.
- **Fix:** Appended all three to tier0-libs.txt with provenance comments (append-only held; the aarch64 v1.0 closure names the same factotum pair).
- **Files modified:** build/x86_64/tiers/tier0-libs.txt
- **Commit:** harvey@bce0f69f1

**2. [Rule 1 - Latent kernel defects] User SSE unarmed + eager stack too shallow**
- **Found during:** Task 2's first r9 sweep (4 classified T1-FAILs — the flagged assumption's exact class, surfaced as captures never silent skips)
- **Fix:** See "The two r9 kernel defects" — CR4 arming + per-exec FP reset + STACK_EAGER_PAGES 64; full regression battery green; push-before-pin (e417660 → 5294f27b8 lock-only).
- **Files modified:** ../r9/x86_64/src/{l.S,exec.rs}, flake.lock
- **Commit:** r9@e417660, harvey@5294f27b8

**3. [Rule 3 - Blocking venue] The donor build needs full-build artifacts the subset host lacks**
- **Found during:** Task 2's build-9k10t1 runs (nvram: "don't know how to make '../root/nvram'"; then /lib/keyboard: same class)
- **Issue:** The full in-OS build (`nix run .#build`) creates sys/src/9k/root/nvram (build/scripts/build:48-49's zero-file rule) and serves /lib/keyboard (portmkfile's mklatinkbd rule); a host that only ran the subset drivers has neither, and this host has no passwordless sudo for RUN_BUILD_IN_PRIVATE_NAMESPACE.
- **Fix:** The nvram stub rule (head -c 512 — Plan 9 dd's dash syntax is shadowed, the D-02 class), the /lib bind in the chroot scaffold, and the passwordless venue itself (the shared nuke-sandwich core wrapped in the 14-05 userns scaffold).
- **Files modified:** build/mm/build-9k10t1.sh
- **Commit:** harvey@bce0f69f1

**4. [Rule 3 - Blocking] mkimage.sh needed the tier bake section + the committed fixtures needed a gitignore negation**
- **Found during:** Task 2 design (the r9 leg consumes "the 14-07 compose pipeline's image variant" — the variant had to exist)
- **Fix:** mkimage section 9 (the tier bake with witness verification); hello.c + nm-input.6 committed as fixtures (nm-input.6 gitignore-negated against the tree-wide *.6 rule — the nm-input.7 discipline; byte-reproduction documented).
- **Files modified:** build/x86_64/mkimage.sh, .gitignore, build/x86_64/{hello.c,nm-input.6}
- **Commit:** harvey@bce0f69f1

**5. [Rule 1 - Hygiene] Two droppings classes the WIP cleanup missed**
- **Found during:** Task 2's reruns (units.c/units.tab.h from the yacc single; 6.init from stage/init's install)
- **Fix:** Targeted rm additions (never a blanket clean — jpg's committed tables).
- **Files modified:** build/x86_64/subset-build.sh
- **Commit:** harvey@bce0f69f1

**6. [Rule 3 - Verify shape] Green sweep runs printed no markers to the harness log**
- **Found during:** Task 2's plan-verify run (`grep -c 'T1-OK' k10t1sweep.log` = 0 on a PASS — only failure digests printed)
- **Fix:** Green classifiers self-print the marker digest (D-16's convention — runner logs are the visible evidence); both arms.
- **Files modified:** build/mm/boot-9k10t1-sweep.sh
- **Commit:** harvey@bce0f69f1

**Total deviations:** 6 auto-fixed (1 missing-critical closure, 2 latent kernel defects, 3 blocking/venue/hygiene). **Impact:** both kernel fixes are permanent, regression-proven improvements; the closure appends are exactly the append-only growth the discipline prescribes; the plan's terminal truth (both venues green, gate twice-green, parity proven) is met.

## Authentication Gates

None.

## Known Stubs

None in the shipped code. Two recorded era limits (documented, not stubs): (a) r9 has no per-proc FXSAVE/XRSTOR — FP registers survive context switches in place, correct for the sequential-sweep era and recorded in exec.rs + r9@e417660's message; real FP context switching lands with the pager era. (b) The donor venue's SKIP set (24) vs r9's (2) differs because the two venues read different tier data files (aarch64's full list vs this phase's x64 core scope) — the parity claim covers the shared membership (the 35 + float-d01, all T1-OK on both), which is exactly the embed-set witness; the write-flow/build-chain classes join the x64 data only in Phase 15. The flagged assumption's residual-gap check: the sweep's watchdog (budgets), converse assertions, and failure digests all carried to the x64 arms — the first run's 4 T1-FAILs were classified captures with digests, never silent skips or hangs (14-EVIDENCE.md does not exist yet at plan time; this SUMMARY + the r9 sources carry the record for it).

## Issues Encountered

None open — the two kernel defects are fixed, regression-proven, and pushed; both repos clean.

## TDD Gate Compliance

N/A — no task carries tdd="true".

## Next Plan Readiness

READY for 14-09 (the net smoke composes through the same apps/bake; the tier gate's driver shape is its direct template) — and for Phase 15's fuller tiers (the append-only lists + the ledger + the computed-expectation gates are the growth surface).

## Self-Check: PASSED

- files exist: subset-build.sh, tier1-sweep.rc, tier-termrc, hello.c, nm-input.6, tier1.ledger, tier1-r9.expect, nix/tier1-r9-x64.sh ✓; mm scripts + mkimage.sh + flake.nix modified in harvey@{bce0f69f1,b50a0e1ca} ✓; r9 l.S/exec.rs @e417660 ✓
- commits are ancestors of HEAD: harvey {7b515dcee, 5294f27b8, bce0f69f1, b50a0e1ca} ✓; r9 e417660 ✓ (both repos' working trees clean outside the .planning research cache)
- plan-level verification green: tier data parses + five filter arms + aarch64 frozen (Task 1); subset-build TIER-BUILD-OK + 15/40 markers + both venue sweeps green with identical 36-marker sets (Task 2); the gate twice-green with committed markers, ledger, and parity (Task 3)
