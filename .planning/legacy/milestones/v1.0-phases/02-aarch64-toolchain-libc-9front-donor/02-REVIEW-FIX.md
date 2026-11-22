---
phase: 02-aarch64-toolchain-libc-9front-donor
fixed_at: 2026-11-01T19:10:47Z
review_path: .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-REVIEW.md
iteration: 1
findings_in_scope: 11
fixed: 11
skipped: 0
status: all_fixed
---

# Phase 02: Code Review Fix Report

**Fixed at:** 2026-11-01T19:10:47Z
**Source review:** .planning/phases/02-aarch64-toolchain-libc-9front-donor/02-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 11 (fix_scope=critical_warning: 1 critical + 10 warnings; 13 Info findings not in scope)
- Fixed: 11
- Skipped: 0

**Verification environment:** all fixes were applied, verified, and committed in the main working tree (`/Users/neilskinner/code/harvey`, branch `master`) — orchestrator-directed sequential mode, no worktree. Verification tiers used: Tier 1 re-read of every modified region (all findings); Tier 2 syntax checks — `sh -n`/`bash -n` for shell scripts, `python3 -m py_compile` for drive9.py and for the extracted embedded-driver heredocs (gatedrive9.py, subsetdrive9.py); plus targeted live checks — WR-01 guard executed (exit 2 with the documented message on this post-02-06 tree), WR-04 regex/probe logic validated against simulated console byte streams (healthy boot, echo-only, and the 5-byte-drop attack), WR-07 `find` exclusion dry-run against the local pinned donor checkout, WR-09 `set -e`-safety harness for the script's `&&`/`||` idioms, WR-10 functional run of `bash build/mkdirs` in a scratch dir. `build/aarch64/gate-evidence.log` was deliberately NOT modified — the CR-01 fix is script-only (the committed log's 3 human-approved PASS blocks stand); no gate was re-run as part of these fixes.

## Fixed Issues

### CR-01: run-gate.sh's "scaffolding-free" assertion is stale — the TC-03 gate now always fails

**Files modified:** `build/aarch64/run-gate.sh`
**Commit:** 064c6a185
**Applied fix:** Deleted the stale `test -e "$HARVEY/aarch64"` assertion (its protective purpose ended when plan 02-06 legitimately committed the arch dir — a7945d8b5), replacing it with a NOTE comment recording why the check is gone. The TC-03 gate is idempotent again per the documented README contract (consecutive runs append consecutive PASS blocks). The committed `gate-evidence.log` was not touched. Verified: Tier 1 re-read + `sh -n`.

### WR-01: pre-gate-build.sh carries the same stale assertion and is obsolete-but-undocumented

**Files modified:** `build/aarch64/pre-gate-build.sh`, `build/aarch64/README.md`
**Commit:** 2b30d900e
**Applied fix:** Per the review's recommendation, guarded the script explicitly as historical: it now exits 2 with "pre-gate-build.sh is obsolete post-02-06 (aarch64/ is committed); use objtype=aarch64 mk in the namespace" when `aarch64/mkfile` exists (i.e., on every current tree), placed right after the checkout sanity check so it precedes any side effects. Also fixed the second defect named in the finding — the `#!/bin/sh` shebang over the bashism `printf '%q'` — by declaring `#!/usr/bin/env bash` (and updating the usage comment), and added the obsolescence note to the script header and the harness README script-table row. The pre-gate step-6 assertion is retained: it remains the correct check for the pre-02-06 trees where the script can still run. Verified: Tier 1 + `bash -n` + live run (exit 2 with the documented message).

### WR-02: nsrun.sh root mode runs the entire bind/staging sequence without `set -e`

**Files modified:** `build/aarch64/nsrun.sh`
**Commit:** 872d84824
**Applied fix:** Added `set -e` immediately after the root-mode argument parsing (with a comment explaining the partial-namespace hazard). The deliberate `set +e` around the `su` status capture at the tail is unchanged, as is the EXIT trap unwinding the staged /lib. Verified: Tier 1 + `bash -n`; confirmed all commands in the covered region are either simple (fail-hard desired) or already guarded (`if`, `|| true`).

### WR-03: gdbpoke9.py's 120s default deadline is far below the 600–1500s boot budgets

**Files modified:** `build/aarch64/boot-9qemu.sh`, `build/aarch64/run-gate.sh`, `build/aarch64/subset-exec.sh`
**Commit:** f494b7727
**Applied fix:** Each of the three callers now passes `--deadline "$BOOT_BUDGET"` to gdbpoke9.py (600s boot-9qemu/run-gate, 1500s subset-exec; BOOT_BUDGET is defined ahead of every call site), so the wake poke's watch window scales with the boot budget instead of false-failing healthy-but-slow TCG boots. gdbpoke9.py itself is unchanged (the 120s default stays for standalone use). Verified: Tier 1 + `sh -n` on all three scripts.

### WR-04: drive9.py's "stronger" prompt proof is satisfied by console echo alone; SHELL_RE is one 5-byte drop away from a false positive

**Files modified:** `build/aarch64/drive9.py`, `build/aarch64/run-gate.sh` (embedded gatedrive9.py), `build/aarch64/subset-exec.sh` (embedded subsetdrive9.py)
**Commit:** 795fd9de6
**Status:** fixed: requires human verification
**Applied fix:** Both weaknesses fixed in all three driver copies, keeping the change minimal per the harness README contracts. (1) `PROMPT_RE` is now anchored to a bare prompt line (`(?m)^term% $`) — the echoed assignment line `prompt='term% '` contains the substring but cannot match — and the proof is order-dependent: a buffer `base` is recorded before the assignment, the probe's `^__GSD_PROMPT_SET__$` OUTPUT line must appear from `base` first, and only then is the anchored prompt accepted from `base` (drive9.py gained the forks' `search_from`; the forks' step-5 wait order was restructured output-first, prompt-second). (2) The liveness probe now sends the marker split by rc's `^` concatenation operator (`echo __GSD_SHELL_^OK__`), so the echoed command line never contains the contiguous marker and a `^__GSD_SHELL_OK__$` line can only be rc's output — the review's 5-byte-drop attack now additionally requires the mid-marker caret to vanish. Header/comments updated in all three copies so the proof is not over-trusted. Verified: Tier 1 + `py_compile` (drive9.py and both extracted embedded forks) + `sh -n` on both shell scripts + a simulated-console-stream test of the new regexes (healthy boot matches; bare echo, echoed-assignment, and 5-byte-dropped echo all correctly fail to match). Human verification = the next real gate run (TC-03/TC-05 under TCG), which exercises the new probe and wait ordering end to end.

### WR-05: 9cc/BUILD has no `set -e` — mid-build failures can exit 0 against stale archives

**Files modified:** `sys/src/cmd/unix/9cc/BUILD`
**Commit:** fd95a0336
**Applied fix:** Added `set -e` after the OSTYPE case (with a comment naming the silent-staleness class), matching the phase's other BUILDs (7a/7c/7l, nm.BUILD). The existing `&&`-chained per-library subshells now abort the script on the first failure instead of letting the iar link succeed against a stale archive. Verified: Tier 1 + `sh -n`.

### WR-06: 9cc README's executable.c provenance omits the LP64 commonllp64 rewrite

**Files modified:** `sys/src/cmd/unix/9cc/README`
**Commit:** d448617ed
**Applied fix:** Corrected the libmach/executable.c bullet: the R_MAGIC exectab entry is no longer described as donor-verbatim — it carries the on-disk `(8*4)+8` hsize literal (replacing host-dependent `sizeof(Exec)+8`) — and the Harvey `commonllp64` u32-view LP64 decode fix (executable.c:401-446, with the `hswal(sizeof(Exec)/sizeof(long))` scramble rationale, 02-03) is now recorded. Both claims verified against the current executable.c before writing (line 213 literal + comment; commonllp64 body). Verified: Tier 1 (doc-only change).

### WR-07: donor-vm-build.sh's stale-object sweep deletes two tracked donor files

**Files modified:** `build/aarch64/donor-vm-build.sh`
**Commit:** d41a23220
**Applied fix:** The `*.7` arm of the guest-donor sweep now excludes `*/cmd/awk/test/*`, protecting the donor's tracked awk test fixtures `p.7`/`expected.p.7` (verified against the pinned checkout at `.build/9front-donor`: they are the only two tracked `*.7` files under `sys/src` — a dry-run of both the old and new find commands shows the old sweep would delete exactly those two and the new sweep deletes neither). The comment above the sweep now records both exclusion sets (7.* arm: 7.out.h/7.c; *.7 arm: awk fixtures). Verified: Tier 1 + `sh -n` + find dry-run.

### WR-08: vm-build.sh reports success when the guest build fails

**Files modified:** `nix/vm-build.sh`
**Commit:** d663bab7b
**Applied fix:** The guest-build ssh is now explicitly status-checked: on failure the script prints `error: guest build failed (rc=N) — refusing to rsync artifacts back` and exits with the guest's status, before the artifact pull and the "build complete" line. Verified: Tier 1 + `bash -n`.

### WR-09: nix/build.sh lacks `set -e`; sudo-cp installs and the namespace build both fail softly

**Files modified:** `nix/build.sh`
**Commit:** 566677c94
**Applied fix:** Added `set -e` near the top (sudo-cp installs and mkfile fixups now fail loudly instead of surfacing later as confusing pcc/cpp exec errors), and the namespace build's status is captured explicitly with an `|| { RC=$?; ...; exit $RC; }` guard before the preflight-arm tail — the tail's `|| exit 0` can no longer mask a failed build. The `set -e`-safety of the script's existing idioms (`[ cond ] && cmd` arch/tool loops, `|| KERNDATE=` cascade, `|| SUDO=sudo` fallback, regex checks) was verified with a `bash -e` harness before committing. Verified: Tier 1 + `bash -n` + idiom harness.

### WR-10: build/mkdirs declares `#!/bin/sh` but is bash (arrays)

**Files modified:** `build/mkdirs`
**Commit:** b4e652801
**Applied fix:** Changed the shebang to `#!/usr/bin/env bash` (the review's recommendation; the repo's host-side scripts are bash — build/build.bash), matching the body's use of `lst=( ... )` / `"${lst[@]}"`. All in-tree callers already invoke `bash build/mkdirs` explicitly, so this removes the latent broken `sh build/mkdirs` invitation under dash. Verified: Tier 1 + `bash -n` + functional run in a scratch dir (dirs created correctly).

---

_Fixed: 2026-11-01T19:10:47Z_
_Fixer: the agent (gsd-code-fixer)_
_Iteration: 1_
