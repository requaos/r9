---
phase: 05-shell-boot-one-command-integration
verified: 2026-11-04T12:31:26Z
status: passed
score: 5/5 must-haves verified
covered_files:
  - .github/workflows/r9-shell.yml
  - .planning/debug/DEBUG-G-05-1.md
  - .planning/phases/05-shell-boot-one-command-integration/05-01-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-01-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-02-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-02-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-03-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-03-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-04-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-04-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-05-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-05-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-06-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-06-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-07-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-07-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-08-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-08-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-09-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-09-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-10-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-10-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-11-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-11-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-12-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-12-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-13-PLAN.md
  - .planning/phases/05-shell-boot-one-command-integration/05-13-SUMMARY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-EVIDENCE.md
  - .planning/phases/05-shell-boot-one-command-integration/05-REVIEW-DISPOSITION.md
  - .planning/phases/05-shell-boot-one-command-integration/05-REVIEW-FIX.md
  - .planning/phases/05-shell-boot-one-command-integration/05-REVIEW.md
  - .planning/phases/05-shell-boot-one-command-integration/05-SECURITY.md
  - .planning/phases/05-shell-boot-one-command-integration/05-UAT.md
  - build/REPRODUCIBILITY.md
  - build/aarch64/drive-r9.expect
  - build/aarch64/init.c
  - build/aarch64/mkimage.sh
  - build/aarch64/nsrun.sh
  - build/aarch64/run-r9.sh
  - build/aarch64/smoke-r9.expect
  - build/aarch64/smoke-r9.log
  - build/aarch64/stacksmoke.c
  - build/aarch64/stage-build.sh
  - build/aarch64/subset-build.sh
  - build/harness/Cargo.lock
  - build/harness/Cargo.toml
  - build/harness/src/console.rs
  - build/harness/src/drive9.rs
  - build/harness/src/gdbpoke9.rs
  - build/harness/src/main.rs
  - build/harness/src/mkfs9.rs
  - build/harness/src/regex.rs
  - build/harness/src/srv9.rs
  - build/harness/src/trace.rs
  - flake.lock
  - flake.nix
  - nix/boot-r9.sh
  - nix/smoke-r9.sh
  - sys/src/9k/boot/bootr9.c
  - sys/src/9k/boot/mkfile.aarch64
covered_digest: "v4:sha256:ee506a564447fa788c02b67be871e764ed6ac23dad829a654e7f761f0578c283"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 4/5
  gaps_closed:
    - "SC#3 CI leg: hosted GH Actions run 41825728636 is GREEN (conclusion=success, headSha 68c49ebcc — ancestor of HEAD, zero code drift since) — verified via gh API this run"
    - "G-05-1 (CI failure): root-caused (/bin/rc absent in fresh-host namespace), fixed d758c2071, diagnosis committed in .planning/debug/DEBUG-G-05-1.md, fix verified present in nsrun.sh:103-105"
    - "Live-interactive sanity: user-confirmed 2026-11-04 (UAT test 2) — echo/ls/ps answer at the live % prompt, exit restarts rc; no-echo era delta accepted"
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "flake.lock still pins r9 at 9549cee — 22 commits behind r9 HEAD (e4a50c3); the 05 review-fix series (a244ba2..d8496d3) and the 261103-p64 unsafe elimination are gated green in r9 main but are NOT in the pinned kernel"
    category: other
    reason: "Intentional pin discipline (D-13 dev-flow override for local gates; the pin moves deliberately and the CI-green evidence at run 41825728636 is pin-specific — a pin bump invalidates it and needs a fresh hosted run). No success criterion requires pin freshness; SC#5 requires a pinned git rev, which holds. Track the pin bump as a milestone-completion decision alongside the already-triaged CR-01/CR-02 closure."
    evidence_status: "verified facts (pin value, commit counts, gate logs); the bump decision itself is project judgment"
  - finding: "Quick-261103-p64's harvey-side smoke gate (D8: 'three consecutive nix run .#boot-r9-smoke') names no --override-input, and /nix/store carries no realized r9-kernel-virt package for 286bb4d/e4a50c3 — so those runs most plausibly exercised the flake-PINNED kernel (9549cee), proving harvey-pipeline non-regression rather than the shell chain on the redesigned allocator"
    category: other
    reason: "Recorded for honesty only. The redesigned allocator IS behaviorally proven by gates that build the local r9 tree (verifier-run cargo test -p port green on e4a50c3 incl. the byte-verbatim pin tests; five qemu boot profiles attested), and the pre-refactor fixed kernel's real shell chain was proven by the review-fix era's override-smoke 3/3 at d8496d3 (store paths r9-kernel-virt-{065a358,36dcc39,ec8833c,d8496d3} realized). The phase-05 criteria concern the pinned integration and are unaffected either way."
    evidence_status: "store-path enumeration + plan text; the runs' actual invocation is not recorded"
---

# Phase 5: Shell Boot + One-Command Integration — Verification Report (Re-verification)

**Phase Goal:** The Core Value is delivered — one command, on either host OS, from clean checkouts of both repos, boots r9 plus the Harvey aarch64 userland to an interactive `/bin/rc` shell. Work spans both repos plus the flake glue.
**Verified:** 2026-11-04T12:31:26Z
**Status:** passed
**Re-verification:** Yes — prior report (2026-11-03T17:30:00Z, 4/5, human_needed on the SC#3 CI leg) went stale: the G-05-1 fix landed, the hosted CI ran green, the 05 review-fix series and two quick tasks (261103-giq harness rewrite, 261103-p64 unsafe elimination) changed covered sources, and UAT completed 2/2.

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A single command (`nix run` app or `cargo xtask`) boots r9 + Harvey aarch64 userland to an interactive `/bin/rc` prompt over the PL011 console in qemu | ✓ VERIFIED | Pipeline intact on current tree: `flake.nix:99-125` (bootR9Pkg/smokeR9Pkg, `R9_KERNEL_PKG` from `r9.packages.${system}.kernel-virt`), `nix/boot-r9.sh` compose. Boot proven three independent ways on current code: (a) hosted CI run 41825728636 GREEN (gh API, this verification); (b) UAT test 2 — user typed `echo hi`/`ls /bin`/`ps`/`exit` at the live `% ` prompt 2026-11-04, commands answered, exit restarted rc; (c) committed captures `05-12-smoke.log` / `05-REVIEW-FIX-smoke.log` / `build/aarch64/smoke-r9.log` each carry banner → paqfs → `init: starting /bin/rc` → stacksmoke → `% `. |
| 2 | At the prompt, rc builtins and external binaries (`ls`, `cat`, `echo`, `ps`) execute correctly | ✓ VERIFIED | `build/aarch64/smoke-r9.expect` asserts each by name (unchanged). Current-tree post-refactor capture `build/aarch64/smoke-r9.log` (1117 lines) byte-verified: `r9shell7` echo round-trip (:935), `/bin/cat /bin/echo /bin/ls /bin/ps /bin/rc` in the ls listing (:970-980), four `glenda` ps rows incl init/paqfs/rc (:1016-1017+), `bind -c #e /env` from `cat /lib/namespace` (:1049), `r9delok` DEL-safety answer (:1069), restart `init: starting /bin/rc` (:1074, second occurrence; boot occurrence :816). |
| 3 | Expect-style smoke passes on darwin-aarch64 and linux hosts — CI green (per 05-CONTEXT D-13: CI = linux leg) | ✓ VERIFIED | **The prior report's sole UNCERTAIN component is now closed.** Hosted leg: `gh run view 41825728636 --repo requaos/harvey` → `{"conclusion":"success","headSha":"68c49ebcc7367d410a157f78b0b3e38910260544","status":"completed"}` (verifier-run this session). `git merge-base --is-ancestor 68c49ebcc HEAD` → YES; `git diff 68c49ebcc..HEAD --stat` → 3 planning docs only (05-EVIDENCE/05-SECURITY/05-UAT), zero code/config drift — the green run covers HEAD's code exactly, including the giq harness rewrite and the G-05-1 fix. darwin leg: two consecutive green runs (05-12 gate, committed log). Native aarch64-linux leg: `05-13-linux-clean-clone.log` (byte-verified prior report). EVIDENCE.md slot filled with the real run values (:97-104). |
| 4 | From clean checkouts of both repos, the documented commands reproduce the boot with pinned toolchains/qemu (Phase 1 KERNDATE policy) | ✓ VERIFIED | `build/REPRODUCIBILITY.md:86-146` INT-03 section intact (process-level policy, KERNDATE from checkout HEAD, named pins, NOT-claimed bit-identity, per-host derivation). Clean-checkout proofs: `05-13-darwin-clean-clone.log` + `05-13-linux-clean-clone.log` (pristine clones @ dc8299575, both rc=0) — and the hosted run 41825728636 is itself a clean-checkout reproduction of the CURRENT code on x86_64-linux (GH runner fresh clone @ 68c49ebcc ≡ HEAD's code), closing the sub-case the prior report folded into its human item. flake.lock pins verified current: r9 `9549cee…` + narHash, nixpkgs `31ecbaca1bde`, rust-overlay `c85ccd343087`. |
| 5 | Harvey's `flake.nix` consumes r9 as a flake input (pinned git rev) — no `../r9` relative-path or submodule glue (INT-04) | ✓ VERIFIED | `flake.nix:11` `inputs.r9.url = "github:requaos/r9"` (unchanged); `flake.lock` locks r9 rev `9549cee63d86a0c6d9b4d04ed8860b2efabcd3e6` + narHash (json-parsed this session). No `.gitmodules` exists. `grep -vE '^\s*#' nix/boot-r9.sh \| grep -c '../r9'` → 0; same for `nix/smoke-r9.sh` → 0. Kernel/DTB arrive only via the `R9_KERNEL_PKG` store path (`flake.nix:100,110`). The giq harness addition (`flake.nix:42-46` buildRustPackage, committed zero-dep Cargo.lock) introduced no sibling-path glue. Pin-vs-HEAD delta: see advisory #1. |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified)

### Staleness-Cause Audit (what changed since the prior run — each verified goal-preserving)

1. **G-05-1 fix (harvey `d758c2071`).** `build/aarch64/nsrun.sh:103-105` verified present: fail-hard `$PLAN9/bin/rc` executability check, touch-mountpoint, `mount --bind "$PLAN9/bin/rc" /bin/rc` — the Dockerfile:42 precedent inside the private namespace. Diagnosis committed (`DEBUG-G-05-1.md`, status resolved): raw CI log showed the real failure (membername `#!/bin/rc` shebang ENOENT → empty libboot.a7 → 7l undefineds → SUBSET-BUILD-FAIL boot(8)); pre-fix byte-identical repro in an emulated x86_64 container, post-fix full ALL STAGES PASS, aarch64 lima non-regression. The hosted re-run (41825728636) is the external proof.
2. **Hosted CI leg GREEN.** See SC#3. First hosted run 41750071242 failed (the G-05-1 trigger), was diagnosed, fixed, and the re-run is green. EVIDENCE.md + UAT.md record it; gh API confirms it independently.
3. **Review-fix series (r9 `a244ba2..d8496d3` + harvey `0f70588e7`).** All 10 in-scope findings verified in the CURRENT r9 tree (e4a50c3): CR-01 four-string stat records (`devcons.rs:315` `need = 41 + (2 + name.len()) + 3 * (2 + 6)`, `:332` muid fourth string); CR-02 stat/fstat cold-buffer residency — now STRUCTURAL: `sys_stat`/`sys_fstat` (`syscall.rs:840,871`) build their output buffers via `UserSliceMut::new`, the usermem fused constructor that runs `validate_user_range` + `fault::ensure_user_resident` (`usermem.rs:44-46`) — the refactor made the review fix un-forgettable, comments cite T-05-06; WR-03 `pending_eof` (`devcons.rs:602`); WR-04 `chain_snapshot` (`pgrp.rs:202`, consulted at `namec.rs:163,222` — no pgrp lock across devmnt RPCs); WR-07 per-run `-name` tag (`nix/smoke-r9.sh:137` `R9_RUN_TAG="r9smoke-$$-$(date +%s)"`, bracket-fixed pkill keyed per-run); WR-08 devroot dir listings (`devroot.rs` read_dir/write_entry, /rootfs.img synthesized node). Gate-surfaced latent fixes verified present: QuickFit unlink (ec8833c) and the tick's direct FIFO poll (d8496d3) — `timer.rs` tick() calls `pl011::cons_rx_poll()` under `#[cfg(dev_exec)]` with the 05-REVIEW-FIX comment before the wake check. Gate capture `05-REVIEW-FIX-smoke.log` (113,162 bytes) byte-verified: boot+restart rc (:817,:1106), r9shell7 (:967), r9delok (:1101), zero wedgedbg/tripwire/panic lines.
4. **Quick 261103-giq (Python → Rust harness).** `build/harness/` exists (Cargo.toml + committed zero-dep Cargo.lock + 8 std-only src modules incl. the CPS regex engine and the 9P2000 srv9); `flake.nix:42-46` wires `packages.harness`; the five `.py` tools are deleted (`git ls-files 'build/**/*.py'` → empty); the four shell callers rewired. Not on the boot-r9/smoke-r9 path (dev-time donor tooling) — and the hosted green run includes these commits, proving the rewired tree end-to-end.
5. **Quick 261103-p64 (maximal unsafe elimination, r9 `32babaf..e4a50c3`).** Current-tree facts: `static mut` in `port/src` + `aarch64/src` → **0** (grep); `aarch64/src/usermem.rs` exists with the fused validate+reside constructors; `port/src/lib.rs` carries `#[forbid(unsafe_code)]` on every module except the two named floor modules (globalalloc + mcslock); `UNSAFE-AUDIT.md` reconciliation table verified (129 aarch64 + 21 port lint-visible = 150 SAFETY annotations; 10 test-only blocks; 12 unsafe fns documented separately). **Behavioral gate run by THIS verifier:** `nix develop --command cargo test -p port` on r9 HEAD e4a50c3 ran to completion green — cargo aborts at the first failing target by default, and the run reached the final suites (namespace 9/9, paqfs 5/5, doc-tests) with `0 failed` throughout. Smoke capture `build/aarch64/smoke-r9.log` (the retained p64-era run) byte-verified per SC#2. Kernel-provenance nuance of that gate: advisory #2.
6. **UAT 2/2.** `05-UAT.md` frontmatter `status: complete`, total 2 / passed 2; test 1 resolved with the run URL + headSha; test 2 resolved with the user's 2026-11-04 live-prompt confirmation (no-echo era delta judged acceptable for the milestone headline).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `flake.nix` (harvey) | inputs.r9 + boot-r9/boot-r9-smoke apps + harness pkg | ✓ VERIFIED | lines 11, 42-46, 99-125 |
| `flake.lock` (harvey) | r9 rev+narHash pin | ✓ VERIFIED | 9549cee… (json-parsed); unchanged since 05-11 (`git log --follow` shows the pin set once, in 58d1b1077) |
| `nix/boot-r9.sh` / `nix/smoke-r9.sh` | compose + packaged smoke | ✓ VERIFIED | WR-07 per-run -name tag present; zero `../r9` |
| `build/aarch64/smoke-r9.expect` / `drive-r9.expect` | sweep drivers | ✓ VERIFIED | unchanged, markers proven in captures |
| `build/aarch64/nsrun.sh` | G-05-1 /bin/rc bind | ✓ VERIFIED | lines 103-105 |
| `build/aarch64/stage-build.sh` | smoke-out/ split (4150a3034) | ✓ VERIFIED | smoke-out/{init,hello,smokefork,smokenote,stacksmoke} staged separately from out/ |
| `build/harness/` (Rust, std-only) | giq replacement for 5 Python tools | ✓ VERIFIED | 8 modules; .py files deleted; flake-wired |
| `sys/src/9k/boot/bootr9.c` + `mkfile.aarch64` | aarch64 boot build | ✓ VERIFIED | 1470 / 2041 bytes, substantive |
| `build/REPRODUCIBILITY.md` | INT-03 policy + pins | ✓ VERIFIED | lines 86-146 |
| `.github/workflows/r9-shell.yml` | linux CI leg | ✓ VERIFIED, NOW RUN GREEN | unchanged since dc8299575; run 41825728636 success |
| `../r9/port/src/*.rs` device set | namespace/pipe/9P/env/cons/proc | ✓ VERIFIED | all 10 present post-refactor (namec, pgrp, devenv, devpipe, devdup, devsrv, devmnt, devproc, devroot, fdgrp) |
| `../r9/aarch64/src/syscall.rs` handlers | measured-set dispatch | ✓ VERIFIED | 26 `fn sys_*` handlers; stat/fstat on the fused usermem constructor |
| `../r9/aarch64/src/usermem.rs` | p64 single entry point | ✓ VERIFIED | UserSlice/UserSliceMut fused validate+reside |
| `../r9/UNSAFE-AUDIT.md` | reconciled inventory | ✓ VERIFIED | 150 SAFETY + 10 test-only; counting rule + scope line explicit |
| `../r9/flake.nix` packages.kernel-virt | kernel store-path contract | ✓ VERIFIED | lines 81-99; realized store paths for every gated rev present |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| harvey flake | r9 flake input | `inputs.r9` → `kernel-virt` → `R9_KERNEL_PKG` | ✓ WIRED | store path realized; CI green consumes it |
| boot-r9.sh | bake | mkimage.sh → host mkpaqfs → bootfs.paq | ✓ WIRED | bake PASS lines in clean-clone logs; D-02 factotum-absence assert at mkimage.sh:73 intact |
| smoke-r9.expect | composed pipeline | qemu stdio markers | ✓ WIRED | current-tree capture carries every asserted marker |
| r9 kernel | Harvey userland | paqfs initrd → devroot/exec → /boot/boot | ✓ WIRED | all committed captures: `init: starting /bin/rc` |
| GH workflow | the pipeline | `nix run .#boot-r9-smoke` on ubuntu-latest | ✓ WIRED, RUN GREEN | run 41825728636 (the prior report's one NOT-WIRED link — now closed) |
| nsrun.sh | fresh-host /bin/rc | bind-mount of $PLAN9/bin/rc | ✓ WIRED | G-05-1 fix; hosted x86_64 run is the external proof |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| boot-r9.sh kernel path | `R9_KERNEL_PKG` | flake-input store path | ✓ real ELF per gated rev in /nix/store | ✓ FLOWING |
| smoke markers | expect buffer | live qemu serial | ✓ 3 committed captures byte-verified | ✓ FLOWING |
| ps output | devproc PROC_TABLE | live kernel | ✓ 4 glenda rows in current capture | ✓ FLOWING |
| ls /bin | devmnt → paqfs → baked image | Harvey's own bake | ✓ 17-entry listing in current capture | ✓ FLOWING |
| GH run conclusion | Actions API | hosted execution | ✓ gh API this session | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Hosted CI green | `gh run view 41825728636 --json conclusion,headSha,status,url` | success / 68c49ebcc… / completed | ✓ PASS |
| headSha covers HEAD's code | `git merge-base --is-ancestor` + `git diff 68c49ebcc..HEAD --stat` | ancestor; 3 docs files only | ✓ PASS |
| G-05-1 fix present | read nsrun.sh:103-105 | bind + fail-hard present | ✓ PASS |
| flake.lock pin parses | python3 json load | 9549cee… + narHash | ✓ PASS |
| No `../r9` glue | grep -c on shipped scripts | 0, 0; no .gitmodules | ✓ PASS |
| static mut eliminated | grep `static mut` in r9 port/src + aarch64/src | 0 | ✓ PASS |
| CR-01 4-string stat | grep devcons.rs | `3 * (2 + 6)` + muid write | ✓ PASS |
| CR-02 residency (new shape) | read syscall.rs:829-884 + usermem.rs:44-46 | UserSliceMut::new fuses validate+reside | ✓ PASS |
| WR-04 no-lock-across-RPC | grep chain_snapshot | pgrp.rs:202, consulted namec.rs:163/222 | ✓ PASS |
| Tick FIFO drain (d8496d3) | read timer.rs tick() | `pl011::cons_rx_poll()` under cfg(dev_exec) | ✓ PASS |
| p64 port gate on current r9 | `nix develop --command cargo test -p port` | full suite green (run to completion; cargo aborts on first failing target — final suites reached, 0 failed) | ✓ PASS |
| Current-tree smoke capture markers | grep build/aarch64/smoke-r9.log | all markers incl. restart + bind line | ✓ PASS |
| Five qemu boot-test profiles | — | not re-run (TCG minutes each; r9-side committed gates + the verifier-run port suite + 3 byte-verified smoke captures attest) | ? SKIP |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| INT-01 | 05-11 | One command boots to interactive /bin/rc over PL011 | ✓ SATISFIED | SC#1 row (CI + UAT + captures) |
| INT-02 | 05-12, 05-13 | Expect smoke on darwin + linux | ✓ SATISFIED | SC#3 row — all three legs green incl. hosted |
| INT-03 | 05-13 | Clean-checkout reproducibility, pinned toolchains | ✓ SATISFIED | SC#4 row |
| INT-04 | 05-10, 05-11 | flake input (pinned rev), no relative-path/submodule glue | ✓ SATISFIED | SC#5 row |

No orphaned requirements: REQUIREMENTS.md maps exactly INT-01..04 to Phase 5, all marked Complete; plans claim exactly INT-01..04.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `nix/smoke-r9.sh:130`, `stage-build.sh:63` | — | `mktemp -d …XXXXXX` template matched the XXX grep | ℹ️ False positive | mktemp placeholder syntax, not a debt marker |
| `../r9/aarch64/src/platform/raspi3b/mailbox.rs` | 86, 330 | two `FIXME` comments | ℹ️ Info | Pre-existing since phase 03-02 (count 2 before and after the review/refactor series — `git show a244ba2~1:…` == HEAD); raspi3b platform code outside the p64 elimination scope; not phase-05 debt |
| `../r9/port/src/devcons.rs` | 43, 63 | `// XXX: Just for testing.` | ⚠️ Warning (tracked) | IN-10 in 05-REVIEW-DISPOSITION.md — open info-tier finding with an auditable ledger entry; not untracked debt |

No blockers. No new debt markers introduced by the G-05-1 fix, review-fix series, or either quick task.

### Deferred Items

None applicable — no gaps to defer. (Phase-era `deferred-items.md` remains as the milestone backlog; two of its open entries — the pgrp-lock hazard and the 3-string stat records — were since FIXED by WR-04/CR-01 in the review series. The file not being re-synced is cosmetic; the disposition ledger carries current state.)

### Advisory (New Scope / Honesty Record)

See frontmatter `advisory:` — (1) the r9 flake pin at 9549cee vs r9 HEAD e4a50c3 (22 commits: review fixes + unsafe elimination gated in r9 main, not yet pinned; pin bump is a milestone-completion decision that will need a fresh hosted CI run); (2) the p64 harvey-side smoke gate's kernel provenance (most plausibly the pinned kernel — integration non-regression — with the redesigned allocator proven by the r9-local gates). Neither affects any success criterion.

### Human Verification Required

None. Both prior human items are resolved and recorded in 05-UAT.md (2/2 pass): the hosted CI run is green (independently confirmed via gh API this session) and the live-interactive prompt check was user-confirmed on 2026-11-04.

### Gaps Summary

No gaps. All five success criteria hold on the CURRENT tree, byte-verified against committed evidence and live external state:

1. The one-command boot pipeline is intact and proven — by the hosted CI run, by the user's live-prompt session, and by three committed serial captures.
2. The command sweep (echo/ls/cat/ps, rc builtins, DEL safety, exit-restart) is asserted by the smoke and present in the current-tree capture.
3. The CI leg the prior report flagged as never-run is now GREEN (run 41825728636, gh-API-confirmed, headSha covering HEAD's code exactly).
4. Clean-checkout reproducibility stands — now additionally backed by the hosted run itself being a clean-checkout execution of current code.
5. The flake-input contract is untouched by the refactor waves: pinned git rev, zero relative-path or submodule glue.

The phase goal — the project's Core Value — is delivered and verified end to end.

---

_Verified: 2026-11-04T12:31:26Z_
_Verifier: the agent (gsd-verifier)_
