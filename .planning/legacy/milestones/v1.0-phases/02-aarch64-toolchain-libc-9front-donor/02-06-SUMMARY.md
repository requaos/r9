---
phase: 02-aarch64-toolchain-libc-9front-donor
plan: "06"
subsystem: infra
tags: [aarch64, objtype-wiring, pcc, mkfile-proto, mkdirs, gitignore, abi-freeze, ureg.h, syscall-contract, 9front-donor, tc-04, libc-build]

requires:
  - phase: 02-aarch64-toolchain-libc-9front-donor
    plan: "05"
    provides: TC-03 execution gate passed and human-approved — committed hello.7.out executes on the 9front donor kernel; the proof that unblocks all aarch64/ arch wiring (riscv64 vaporware rule)
  - phase: 02-aarch64-toolchain-libc-9front-donor
    plan: "03"
    provides: pre-gate libc build with guest-scratch scaffolding — this plan replaces that injection with the committed wiring and re-proves the same build end-to-end
provides:
  - aarch64/mkfile — the 5-line objtype wiring (mkfile.proto include; CC=7c LD=7l AS=7a O=7), byte-pattern from the donor's arm64/mkfile
  - aarch64/include/u.h + aarch64/include/ureg.h — byte-verbatim pinned-donor headers (sha256-gated); ureg.h is the frozen trap frame Phase 4 statically checks r9 against
  - aarch64/ABI.md — the canonical Harvey↔r9 user-side syscall contract (number in R0, args in 8-byte slots at user SP+8 with SP+0 the reserved LR slot, SVC, result in R0; Harvey sys.h 0-54 incl. NMOUNT=54; D-15/D-16/D-17)
  - sys/src/cmd/pcc.c aarch64 objtype row + '7' in allos — pcc dispatches aarch64 compiles to 7c/7l and accepts .7 objects
  - sys/src/mkfile.proto CPUS += aarch64 (LAST, post-gate) and OS += 7 — mkone/mkmany/clean [$OS] regexes sweep .7 objects (deferred-items #3 resolved)
  - build/mkdirs ./aarch64 tree (bin set + lib + lib/ape + bbin)
  - .gitignore *.7 + anchored /aarch64/{9*,*bin,init,lib} (build/aarch64/ scripts and the committed hello.7.out stay trackable)
  - sys/src/cmd/cc/mkfile CURCC += 7c (the agent's-discretion item, resolved post-build-proof)
affects: [02-07 (shell-subset build + build_aarch64 activation per assumption A1), phase-03 (r9), phase-04 (r9 svc dispatch + static trap-frame checks consume ABI.md + ureg.h)]

actuals:
  tokens: 2389    # chars/4 over the realized diff (9558 chars)
  tasks: 2
  commits: 2      # measured: git rev-list --count 9ff90d663..HEAD (ledger gsd-plan-head-before-02-06)
  plan_head_before: 9ff90d663cfb58ce7ee7709e9e5f0cb8678cc4e8
  plan_head_after: 0f8ea566f86c6ccc5b740feccb4277501cab6976

tech-stack:
  added: []
  patterns:
    - "execution-gated scaffolding: aarch64/ arch dir lands only after the 02-05 execution-gate commits — git history enforces the riscv64 vaporware rule"
    - "sha256-gated vendor fidelity: u.h/ureg.h imported byte-verbatim and gated against the pinned donor — the Phase 4 field-for-field static check depends on zero drift"
    - "anchored gitignore artifact patterns (/aarch64/lib vs unanchored per-arch convention) so harness scripts and committed gate evidence under build/aarch64/ stay trackable"

key-files:
  created:
    - aarch64/mkfile
    - aarch64/include/u.h
    - aarch64/include/ureg.h
    - aarch64/ABI.md
  modified:
    - sys/src/cmd/pcc.c
    - sys/src/mkfile.proto
    - build/mkdirs
    - .gitignore
    - sys/src/cmd/cc/mkfile

key-decisions:
  - "aarch64/ABI.md is the single canonical Harvey↔r9 user-side contract: args in 8-byte slots at user SP+8 (SP+0 reserved link-register slot — the research-verified correction to D-15's original SP+0 wording), number and result in R0, SVC trap; kernel-side dispatch style (Harvey 9k's void sysxxx(Ar0*, va_list) vs 9front's void sysxxx(Ar0)) explicitly does not cross the wire (D-17)"
  - "NMOUNT=54 documented as Harvey-only: r9 implements the table lazily and NMOUNT may stay unimplemented until something calls it; the 5-arg mount is MOUNT=46, wire-identical to 9front (D-16)"
  - "APE cc.c deliberately NOT synced with the new pcc.c row (APE on aarch64 out of scope this phase) — recorded as a dated note on pcc.c's line-13 sync comment, per the plan's prohibition"
  - "pcc functional proof runs in the lima guest (the phase's canonical Linux build host): pcc.c execs /bin/cpp and /bin/7c by absolute path, and darwin's /bin is SIP-sealed — the host can build pcc but cannot run it (pre-existing, affects amd64 identically)"

patterns-established:
  - "Guest-as-Linux-locus for /bin-absolute tools: any proof that must exec through pcc's hardcoded /bin paths runs in the lima guest, with /bin/<tool> installed from the guest hostbin via nix/build.sh's own sudo-cp mechanism"

requirements-completed: [TC-04]

coverage:
  - id: D1
    description: "aarch64/ arch dir landed: mkfile (CC=7c LD=7l AS=7a O=7) + donor byte-verbatim u.h/ureg.h (sha256 equality with the pinned checkout) + ABI.md carrying every contract element (SP+8, R0, SVC, NMOUNT, ureg.h) and none of the pre-correction SP+0 wording — all strictly after the 02-05 gate commits"
    requirement: TC-04
    verification:
      - kind: other
        ref: "head/grep gates on aarch64/mkfile; sha256sum uniq-count gates for u.h + ureg.h vs .build/9front-donor/arm64/include/; grep gates on aarch64/ABI.md (incl. zero-count 'SP+0 is where'); git log ordering"
        status: pass
    human_judgment: false
  - id: D2
    description: "objtype wiring: pcc.c {aarch64,7c,7l,7,7.out} row + allos '7'; mkfile.proto CPUS+=aarch64 (last) + OS+=7; mkdirs ./aarch64 set + bbin; .gitignore *.7 + anchored /aarch64 set; cc/mkfile CURCC+=7c"
    requirement: TC-04
    verification:
      - kind: other
        ref: "plan verify gate 1: all grep gates + mkdirs sandbox (mktemp dir gains aarch64/{bin,lib,bbin} and every amd64-parity subdir); git check-ignore confirms build/aarch64/* NOT ignored"
        status: pass
    human_judgment: false
  - id: D3
    description: "pcc drives 7c end-to-end for objtype=aarch64: host cmd BUILD rebuilds pcc with the row (rc=0); guest functional proof objtype=aarch64 pcc -c t.c produces t.7 (PCC-PROOF-PASS)"
    requirement: TC-04
    verification:
      - kind: integration
        ref: "nix develop cmd BUILD rc=0 + strings check (aarch64 row, 056789kqvij) in installed host pcc; vm-run.sh guest run: /bin/7{a,c,l} install + guest pcc rebuild + pcc -c t.c && test -s t.7 → PCC-PROOF-PASS"
        status: pass
    human_judgment: false
  - id: D4
    description: "namespace libc rebuild for aarch64 through the COMMITTED wiring (no scratch injection, vs 02-03): cd /sys/src/libc && objtype=aarch64 mk nuke && mk install && mk clean exits 0; /aarch64/lib/libc.a built (259 members); clean phase visibly sweeps .[568ijqv7] including .7"
    requirement: TC-04
    verification:
      - kind: integration
        ref: "sh build/aarch64/vm-run.sh '...' rc=0; guest /aarch64/lib/libc.a 779312 bytes, ar t = 259 members"
        status: pass
    human_judgment: false

duration: 6 min
completed: 2026-11-01
status: complete
---

# Phase 02 Plan 06: objtype=aarch64 Wiring + ABI Freeze Summary

**Landed the aarch64/ arch dir (donor-verbatim mkfile + sha256-gated u.h/ureg.h headers + the canonical ABI.md Harvey↔r9 syscall contract), wired objtype=aarch64 through pcc/mkfile.proto/mkdirs/.gitignore/CURCC, and proved the committed wiring end-to-end: host+guest pcc rebuilds with the new row, pcc drove 7c to a .7 object in the guest, and the full libc build (nuke→install→clean) ran green in the namespace with zero scratch injection.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-11-01T15:08:06Z
- **Completed:** 2026-11-01T15:13:59Z
- **Tasks:** 2/2
- **Files created/modified:** 9 (4 created, 5 modified)

## Accomplishments

- **aarch64/ arch dir committed** (first plan allowed to — the 02-05 execution gate precedes it in git history): `aarch64/mkfile` is the donor arm64/mkfile byte-pattern (`</sys/src/mkfile.proto`, `CC=7c LD=7l AS=7a O=7`); `aarch64/include/{u.h,ureg.h}` are byte-verbatim against the pinned donor (sha256 equality gated: u.h `dd1bd611…`, ureg.h `04910dfc…`); `aarch64/ABI.md` freezes the user-side wire contract with the research-corrected SP+8 wording and the full sys.h 0–54 table including Harvey-only NMOUNT=54.
- **pcc knows aarch64:** `{"aarch64", "7c", "7l", "7", "7.out"}` row + `7` in `allos` (`056789kqvij`) — without which pcc rejects `.7` files as wrong-architecture at pcc.c:153. Proven functionally: `objtype=aarch64 pcc -c t.c` produced `t.7` (guest run; see Deviation 1).
- **mkfile.proto:** `CPUS` gains `aarch64` LAST (post-gate, per the vaporware rule); `OS` gains `7` (`568ijqv7`) so mkone/mkmany/clean regexes handle `.7` — visibly exercised by the libc rebuild's clean phase (`rm -f *.[568ijqv7]`); resolves deferred-items #3.
- **build/mkdirs + .gitignore:** the `./aarch64` tree (bin set + lib + lib/ape + bbin) follows the amd64 block; `*.7` plus anchored `/aarch64/{9*,*bin,init,lib}` keep `build/aarch64/` scripts and the committed `hello.7.out` trackable (check-ignore verified).
- **CURCC gains 7c** — the agent's-discretion item resolved: 7c has compiled natively under 6c since 02-01's sweep check, so it joins `mk everything`.
- **Committed-wiring libc build green:** `cd /sys/src/libc && objtype=aarch64 mk nuke && mk install && mk clean` in the lima namespace exits 0 with no guest-scratch injection anywhere (02-03's scaffolding was disposable; this is the committed replacement) — `/aarch64/lib/libc.a` 779,312 bytes, 259 members.
- **APE boundary held:** `/sys/src/ape/cmd/cc.c` untouched; the pcc.c line-13 sync comment carries a dated note that the aarch64 row is deliberately unmirrored (plan prohibition).

## Task Commits

Each task was committed atomically:

1. **Task 1: Land aarch64/ — mkfile, verbatim headers, and the ABI freeze document** — `a7945d8b5` (feat)
2. **Task 2: pcc/mkfile.proto/mkdirs/.gitignore wiring + committed-path integration proof** — `0f8ea566f` (feat)

**Plan metadata:** see the final docs commit (SUMMARY/STATE/ROADMAP/REQUIREMENTS + deferred-items).

## Files Created/Modified

- `aarch64/mkfile` — objtype wiring, donor byte-pattern (CC=7c LD=7l AS=7a O=7)
- `aarch64/include/u.h` — per-arch types, byte-verbatim from the pinned donor
- `aarch64/include/ureg.h` — the frozen trap frame (r0..r28, sb note on r28, r29, r30/link union, sp, pc, psr, type); Phase 4's static-check target against r9's Rust trap frame
- `aarch64/ABI.md` — the canonical Harvey↔r9 user-side syscall contract (SP+8 arg base, R0 number/result, SVC, sys.h 0–54 incl. the 48–49 gap, NSEC=53 zero-arg register-return, NMOUNT=54 Harvey-only; D-17 dispatch-style exclusion; stub shape reference; provenance line)
- `sys/src/cmd/pcc.c` — aarch64 objtype row + allos '7' + dated APE-out-of-scope note on the line-13 sync comment
- `sys/src/mkfile.proto` — `CPUS=… aarch64`, `OS=568ijqv7`
- `build/mkdirs` — `./aarch64` directory set + `./aarch64/bbin`
- `.gitignore` — `*.7`, anchored `/aarch64/{9*,*bin,init,lib}`, `/sys/src/cmd/pcc` build leftover
- `sys/src/cmd/cc/mkfile` — `CURCC=5c 6c 7c 8c qc vc`
- `.planning/phases/02-aarch64-toolchain-libc-9front-donor/deferred-items.md` — #3 marked resolved (OS+=7), #9 added (libc/mkfile `everything` glob literal)

## Decisions Made

- **ABI.md is the one canonical place** for the Harvey↔r9 user-side contract, with the corrected freeze wording: syscall number in R0; arguments in 8-byte slots at user SP+8 with SP+0 the reserved link-register slot (the stub's `MOV R0, 0(FP)` materializes arg0 into the first kernel-read slot; the kernel reads `(Sargs*)(ureg->sp+BY2WD)`, BY2WD=8 — 9front trap.c:196, arm64/mem.h:92); SVC; result in R0. Kernel-side dispatch style does not cross the wire (D-17). One-way rated per the plan — Phase 4 re-freezes both repos if this changes.
- **pcc proofs run where pcc can run:** pcc.c execs `/bin/cpp` and `/bin/7c` by absolute path, so the functional proof locus is the lima guest (Linux, writable /bin, `/bin/cpp` already installed by the nix/build.sh mechanism). The host rebuild half of the plan's verify ran on darwin as written.
- **Assumption A1 recorded:** "config section literally named aarch64" (TC-04) is interpreted as the `arch=(...)` lists + `build_$i` dispatch in `build/scripts/{funcs,linux-build}`; activation is plan 02-07's work (activating before the shell subset builds would break the default build). If another config surface was meant, the correction is one line — all wiring sites are enumerated in 02-PATTERNS.md.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] pcc functional proof relocated to the lima guest (darwin /bin is SIP-sealed)**
- **Found during:** Task 2, verify (a)
- **Issue:** the plan's host proof (`objtype=aarch64 pcc -c t.c` on darwin) cannot work: pcc.c execs `/bin/cpp` and `/bin/7c` by absolute path, and macOS /bin is read-only under SIP. Proven pre-existing — the unmodified committed pcc fails identically with `objtype=amd64` (`pcc: exec failed`), so this is a latent plan assumption (Linux-style host), not a regression from this task.
- **Fix:** ran the functional half in the lima guest — the phase's canonical Linux build host — via `build/aarch64/vm-run.sh`: installed `/bin/7a /bin/7c /bin/7l` from the guest hostbin using nix/build.sh's own sudo-cp mechanism, rebuilt the guest pcc from the rsynced (new) pcc.c via `cd /sys/src/cmd && bash BUILD`, then `objtype=aarch64 pcc -c t.c && test -s t.7` → PCC-PROOF-PASS. The plan's literal host half — `(cd sys/src/cmd && bash BUILD)` — ran on the host as written (rc=0; the installed host pcc carries the `aarch64` row and `056789kqvij`). The guest /bin install is guest-side state only; nothing extra is committed.
- **Files modified:** none beyond plan (guest VM state only)
- **Verification:** t.7 produced and non-empty; host pcc binary strings contain the row + allos
- **Committed in:** 0f8ea566f (Task 2 commit)

**2. [Rule 3 - Blocking] namespace libc proof command gained the mkdirs prefix**
- **Found during:** Task 2, verify (b) — first run failed at `ar: cannot create /aarch64/lib/libc.a: No such file or directory`
- **Issue:** git cannot track the empty `aarch64/lib/` directory, so the rsynced guest copy lacked it; `vm-run.sh` (unlike `nix/build.sh`) does not run `build/mkdirs` first. The plan's literal verify command omits the mkdirs step that its own mkdirs edit exists to provide.
- **Fix:** prefixed the proof with `cd "$HARVEY" && bash build/mkdirs` (exactly nix/build.sh's ordering) — this exercised the task's committed mkdirs aarch64 set as intended; the rerun exited 0.
- **Files modified:** none beyond plan
- **Verification:** full `mk nuke && mk install && mk clean` rc=0; `/aarch64/lib/libc.a` 259 members
- **Committed in:** 0f8ea566f (Task 2 commit)

**3. [Rule 2 - Hygiene] .gitignore gained `/sys/src/cmd/pcc`**
- **Found during:** Task 2 (running `(cd sys/src/cmd && bash BUILD)` on the host leaves the compiled `pcc` binary in-tree, untracked)
- **Issue:** generated host-build output not covered by .gitignore (siblings `/sys/src/cmd/7a/7a`, `7c/7c`, `7l/7l`, `nm` already are)
- **Fix:** added the entry adjacent to the nm line
- **Files modified:** .gitignore
- **Verification:** `git status` clean of the binary
- **Committed in:** 0f8ea566f (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 hygiene) — all preserve the plan's gates; the two command-shape amendments ran the plan's own committed mechanisms (vm-run.sh, build/mkdirs) rather than inventing alternates.
**Impact on plan:** No scope creep. Every planned file landed exactly as specified; both integration proofs pass with the committed wiring.

## Issues Encountered

- None beyond the deviations above. The pcc-on-darwin limitation is environmental and pre-existing (documented as a decision: guest is the /bin-absolute-tool locus).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 02-07 (shell subset + TC-05):** all wiring it needs is committed — `objtype=aarch64 mk` works in the namespace end-to-end (libc green), pcc dispatches, mkdirs lays out the tree, and the dormant `build_aarch64` can now be activated (`arch=(amd64 aarch64)` in linux-build per assumption A1). The D-09 NMOUNT nm gate uses the host nm proven in 02-03.
- **Phase 4 (r9 svc dispatch):** consumes `aarch64/ABI.md` + `aarch64/include/ureg.h` as the frozen contract; `build/aarch64/hello.7.out` remains the known-good ABI bisect point.
- **Phase 3 forward note (unchanged):** HVF returns when r9 speaks GICv3; Apple 16K pages constrain guest granules; cache maintenance via `CTR_EL0`.
- No blockers.

## Self-Check: PASSED

- All 4 created files verified on disk: `aarch64/mkfile`, `aarch64/include/u.h`, `aarch64/include/ureg.h`, `aarch64/ABI.md`
- All 5 modified files carry the gated content (grep gates re-run post-commit: pcc row + allos 7 + dated APE note, CPUS/OS, CURCC 7c, gitignore entries — all PASS)
- Both task commits found in git history: `a7945d8b5`, `0f8ea566f`
- Plan-level verification re-run at plan end: Task 1 sha256/mkfile/ABI.md gates PASS; Task 2 grep gates + mkdirs sandbox PASS; guest pcc proof PASS (t.7); namespace libc rebuild rc=0 (libc.a 259 members)
- Acceptance criterion "git log shows 02-05's gate-evidence commit strictly before the first aarch64/ commit" holds: `62351f81f` (02-05 gate) precedes `a7945d8b5` (first aarch64/ commit)
- `/sys/src/ape/cmd/cc.c` untouched (`git status` clean for the path); guest `/bin/7{a,c,l}` install is guest-side state only
- No stubs: every artifact is wired end-to-end; deferred-items #9 records the plan-declared libc/mkfile `everything`-glob deferral

---
*Phase: 02-aarch64-toolchain-libc-9front-donor*
*Completed: 2026-11-01*
