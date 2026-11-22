---
phase: 05-shell-boot-one-command-integration
plan: 05
subsystem: kernel
tags: [r9, aarch64, devenv, env-device, per-pgrp, rfork, RFENVG, RFCENVG, kernel-seed, devroot, rootfs.img, two-views, tdd, qemu]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-04)
    provides: the namespace core (namec registry + union walk + pgrp mount table) devenv registers into; the provisional /rootfs.img node this plan hardens; the pipe-wall gate this plan re-runs; the build-order lesson (the gated config builds LAST)
  - phase: 05-shell-boot-one-command-integration (05-02)
    provides: bootr9.c's method table (embed arg = the literal /rootfs.img) + the baked bootfs.paq (ships the /env mount-point dir)
provides:
  - port::devenv — the per-pgrp environment device (D-07): EnvGroup (create/write/trunc/read/deep_copy, donor devenv.c semantics) + the #e surface (attach/walk/open/read/write/create/stat) resolving the OPERATING proc's group per op via the injected hook; registered as rune 'e' in the device registry
  - rfork env-group semantics per fork(2)/sysproc.c: neither=share, RFENVG=deep copy, RFCENVG=empty; the non-RFPROC caller split (newns's rfork(RFENVG|RFCNAMEG) shape)
  - the kernel-seeded ROOT env group before the first exec (cputype=aarch64, nobootprompt=embed, readparts=0) + the 'devenv: seeded …' gate line; pid 1 runs it; the 05-04 transitional argv hint is RETIRED (argv = [boot])
  - devroot /rootfs.img hardened: mount-time name-collision refusal (named error, nothing published) + named read-only/exec refusals + host tests
  - evidence: .planning/phases/05-shell-boot-one-command-integration/05-05-boot-r9.log (the env-driven gate run)
affects: [05-06 (sys_create wires devenv's create op; the paqfs server opens /rootfs.img for its live read exercise; newns's rfork(RFENVG|RFCNAMEG) gets its live env-split exercise past the pipe wall), 05-08 (newns's $var expansion reads #e/<var>; getenv's seek-to-end sizing needs SEEK(39) — deferred), all later plans (rc's var export to children rides the share-by-default group semantics)]

# Actuals (#2632) — pairs with the plan's estimate (80000 tokens, confidence: low).
actuals:
  tokens: 27500    # MEASURED chars/4: r9 diff 82889 + harvey deferred-items 1164 + this SUMMARY 26105 = 110158 chars; the estimate (80k) overshot ~2.9x.  (The committed 98961-byte gate log is bulk evidence, excluded per the 05-04 convention.)
  tasks: 2
  commits: 4       # MEASURED: r9 git rev-list --count 1344eba..628d612 = 4; harvey 0 pre-metadata (the docs commit rides on top, per the 05-04 convention)
plan_head_before: {r9: "1344eba320e72ffe14e7df1a513d68f876edc180", harvey: "8743f2328f14a93055cd44d89a61c72d8ee9c25f"}
plan_head_after: {r9: "628d612", harvey: "8743f2328f14a93055cd44d89a61c72d8ee9c25f"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies anywhere (T-05-SC held: no package installs)
  patterns:
    - "per-pgrp env group as Arc<Lock<EnvGroup>> on the Proc (the pgrp field's exact shape); every #e op resolves the OPERATING proc's group through a dependency-injected current-group hook (devproc's set_post_hook idiom) — the donor envgrp(c) dynamic shape, never captured at attach"
    - "kernel-seeded root env group via the one-time-publish static + explicit pid-1 assignment (the slash/dot seeding's twin) — RESEARCH Pattern 3's ONE mechanism replacing #ec and the argv hint"
    - "rfork flag semantics verified against fork(2)+sysproc.c BEFORE test-writing when a plan bullet names flag semantics (the 05-04 inversion recurred; the donor governs)"
    - "device create as a strict Eexist-on-dup op; the create-on-existing → OTRUNC-open fallback belongs to namec's Acreate contract (05-06's sys_create), never the device"

key-files:
  created:
    - ../r9/port/src/devenv.rs
    - ../r9/port/tests/devenv_test.rs
    - ../r9/port/tests/devroot_rawimg_test.rs
    - ../r9/port/tests/05-05-red-evidence.json
    - .planning/phases/05-shell-boot-one-command-integration/05-05-boot-r9.log
    - .planning/phases/05-shell-boot-one-command-integration/deferred-items.md
  modified:
    - ../r9/port/src/lib.rs
    - ../r9/port/src/dat.rs
    - ../r9/port/src/namec.rs
    - ../r9/port/src/devroot.rs
    - ../r9/aarch64/src/proc.rs
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/main.rs
    - ../r9/HowItWorks.md

key-decisions:
  - "rfork env semantics follow fork(2) + 9k sysproc.c — the plan's truth bullet inverted RFENVG/RFCENVG (neither=share, RFENVG=deep copy, RFCENVG=empty); the 05-04 RFNAMEG precedent governs, and D-07's operative clause ('behave exactly like Plan 9') + newns's rfork(RFENVG|RFCNAMEG) are only coherent under the donor reading (a shared newns env would leak per-login user=/home= back into init's group)"
  - "devenv resolves the OPERATING proc's group per op via the injected hook (donor envgrp(c) dynamic resolution), never a group captured at attach — a chan walked by one proc resolves in the group of the proc using it (host-pinned: the same walked chan reads group B's value after the hook target swaps)"
  - "the kernel seeds the root env group (cputype=aarch64, nobootprompt=embed, readparts=0 — the mandatory minimum; service/user left to init/newns per the donor flow) BEFORE the first exec; the argv hint is retired (argv=[boot])"
  - "/rootfs.img name collision = loud mount_initrd refusal (named error, nothing published) — the 05-04 provisional's silent real-entry-wins replaced per the plan's defensive mandate; reads keep the paqfs read_at clamp convention (EOF=0 at/past the end — the plan's 'errstr on overrun' read against the donor convention: no Plan 9 device errstrs a past-end read)"

patterns-established:
  - "env devices are flat trees keyed by name; qid paths are per-group creation sequences (renumbered on envcpy) so cross-group chan use resolves the k-th entry of the OPERATING group"
  - "the #ec config-env mechanism is deliberately absent: the attach spec validates (empty only) and errors loudly; boot's bind(\"#ec\", …) is the designed fail-soft path"

requirements-completed: [INT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "devenv (D-07): the per-pgrp env device with donor devenv.c semantics — seeded read-back, unset-key open failure (boot's fail-soft contract), create + write + OTRUNC-open replace (the libc setenv flow), grow-at-offset writes, Maxenvsize/name bounds, two-group isolation, and the full #e device surface over the OPERATING proc's group (dynamic envgrp resolution, registry 'e', #ec loud refusal, stat shapes, the dir listing)"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — port/tests/devenv_test.rs 6/6 (seeded_keys_read_back_and_unset_is_absent, write_replace_and_create_round_trip, rfork_env_semantics_share_copy_empty, two_groups_hold_same_key_independently, env_bounds_and_name_validation, devenv_device_flow_over_dynamic_groups) + the full suite 44/44"
        status: pass
      - kind: integration
        ref: "builds virtshell/virtexec/default/raspi4b + clippy virtexec (no warnings on the touched files) + all five boot-test profiles PASS + ureg-check 35/35"
        status: pass
    human_judgment: false
  - id: D2
    description: "The env-driven boot gate (05-05 must_haves truth 3): with argv reduced to [boot] (the 05-04 hint REMOVED), the kernel seeds the root group before the first exec ('devenv: seeded cputype=aarch64 nobootprompt=embed readparts=0'), boot's rootserver reads #e/nobootprompt and selects embed with NO outin prompt (a devenv break would hang there and time out loudly), and the flow reaches connectembed's 'paqfs...' and stops at the loud PIPE wall — panic-free"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: ".planning/phases/05-shell-boot-one-command-integration/05-05-boot-r9.log (committed): 'devenv: seeded cputype=aarch64 nobootprompt=embed readparts=0' + 'exec: entering EL0 … argv=[\"boot\"]' + two 'syscall 39 unimplemented' (getenv's seeks AFTER open(\"/env/readparts\") succeeded through the union-mounted /env — accidental proof of the union round-trip) + 'paqfs...' + 'syscall 21 unimplemented' + 'process 1 exited' + 'all user processes exited'; zero 'panic|Unhandled interrupt' lines"
        status: pass
      - kind: integration
        ref: "all five boot-test profiles PASS post-change (the seed print lands in every dev_exec profile's log without breaking marker assertions)"
        status: pass
    human_judgment: false
  - id: D3
    description: "devroot /rootfs.img hardened (the D-04 two-views counterpart): the synthesized node resolves at the root, stats as a 0444 regular file whose length is the image size, reads proxy the image slice bounds-checked (clamp + EOF=0 per the paqfs read_at convention), write/exec are refused with NAMED errors, and an image shipping its own rootfs.img root entry fails mount_initrd loudly with nothing published"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — port/tests/devroot_rawimg_test.rs 3/3 (rawimg_node_walk_stat_read_and_bounds, rawimg_node_refuses_write_and_exec_loudly, rawimg_name_collision_fails_mount_loudly incl. the nested-namesake control)"
        status: pass
      - kind: e2e
        ref: "the boot gate stays green with the change inert for the walked tree (connectembed's dirstat(\"/rootfs.img\") stats the node — 'paqfs...' printed; zero panics); the cross-plan literal grep holds (devroot.rs + bootr9.c both carry rootfs.img)"
        status: pass
    human_judgment: false
  - id: D4
    description: "rfork env-group wiring (D-07's RFENVG/RFCENVG arms): the proc.rs field + branch wiring (share/copy/empty + the non-RFPROC caller split) with the primitives host-pinned"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — rfork_env_semantics_share_copy_empty pins the group primitives the wiring mirrors (share=Arc clone visibility, copy=invisible+prior-values-intact, empty=fresh)"
        status: pass
    human_judgment: true
    rationale: "The flag→branch wiring in proc.rs/syscall.rs has no EL0 caller until init's newns (rfork(RFENVG|RFCNAMEG)) — past the pipe wall, first exercised by 05-06+'s deeper boot. The primitives are host-pinned and the wiring compiles + mirrors the 05-04 namespace wiring reviewed the same way, but no execution evidence exercises the env arms this plan (the plan's own gate stops at PIPE by design)."

# Metrics
duration: 26 min
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 05: devenv (D-07) + kernel-seeded env + the hardened /rootfs.img Summary

**The environment is real: r9's `#e` is a per-pgrp device (donor devenv.c semantics — dynamic `envgrp(c)` resolution through an injected current-group hook, rfork share/copy/empty per fork(2)), the kernel seeds the root group before the first exec (`cputype=aarch64`, `nobootprompt=embed`, `readparts=0`), and boot's method pick is env-driven end-to-end — the 05-04 argv hint is retired (`argv=["boot"]`) and rootserver still reaches `paqfs...` without a prompt. devroot's `/rootfs.img` raw-image node is hardened (mount-time name-collision refusal, named read-only/exec errors) — the paqfs server's byte source is contracted and collision-proof.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-11-03T06:07:14Z
- **Completed:** 2026-11-03T06:33:25Z
- **Tasks:** 2
- **Files modified:** 14 (8 r9 source + 3 r9 test/evidence files created, HowItWorks.md, 2 harvey planning artifacts)

## Accomplishments

- **devenv landed TDD-style (D-07).** RED: 6 host tests + the compiling skeleton, evidence verified `RED_EVIDENCE_OK` (target `seeded_keys_read_back_and_unset_is_absent` failed on the planned assertion — `left: None, right: Some("aarch64")` — exit 101, 6/6 failing) at `port/tests/05-05-red-evidence.json` (r9 `40bab8e`). GREEN (r9 `3bdb410`): `EnvGroup` (create = strict Eexist-on-dup, grow-at-offset writes, Maxenvsize bound, OTRUNC trunc, envcpy renumbering deep copy) + the `#e` surface (attach/walk/open/read/write/create/stat) registered as rune `'e'` in the 05-04 registry. Every op resolves the OPERATING proc's group via the injected hook — the donor's `envgrp(c)` → `up->egrp` dynamic shape, host-pinned by a test that swaps the hook's group and re-reads the same walked chan. The tree is flat; the root dir serves the group's stat-wire listing (init's `/env` union read + later `ls /env` work).
- **The kernel seeds the root env group; the argv hint is gone.** exec_bootstrap registers the hook + seeds `cputype=aarch64`, `nobootprompt=embed`, `readparts=0` (the mandatory minimum — service/user are init's/newns's to set per the donor flow) and prints `devenv: seeded …` BEFORE the first exec; pid 1 gets the root group explicitly (the slash/dot seeding's twin). `argv` is back to `[boot]` with a comment naming Pattern 3. The gate proves the contract: `argv=["boot"]` and boot STILL reaches `paqfs...` — rootserver's `#e/nobootprompt` read selected embed with no `outin()` hang (the fail-loudly design: a devenv break times the gate out).
- **rfork env semantics per fork(2) — with the plan's inversion corrected (Deviation 1).** Neither flag shares (Arc clone — rc's var export to children), RFENVG deep-copies (newns's per-login overrides never leak back), RFCENVG empties; the non-RFPROC branch re-groups the CALLER the same way (newns's `rfork(RFENVG|RFCNAMEG)` shape). Host tests pin the matrix; the wiring mirrors the reviewed 05-04 namespace branches.
- **The `/rootfs.img` node hardened (Task 2, D-04's two-views counterpart).** mount_initrd now REFUSES an image shipping its own rootfs.img root entry — a named error, nothing published (one view never shadows the other silently; the 05-04 provisional's real-entry-wins is replaced). Write-side and exec opens carry named refusals (`/rootfs.img is read-only`, `cannot exec the raw image`). Reads keep the paqfs `read_at` clamp convention (EOF=0 at/past the end — see Deviation 4). Three host tests pin the stat shape (0444, length = image size), the bounds, root-only synthesis, and the collision refusal (with a nested-namesake control). The gate exercises the node's stat live (connectembed's `dirstat(paqfile)`) and stays green.
- **Incident proof of the union-mounted /env round-trip:** the gate log's two `syscall 39 unimplemented` walls are `getenv("readparts")` SEEKing — which only happens AFTER `open("/env/readparts")` SUCCEEDED through the union (bindenvsrv's `#e` bind now lands). The seeded key resolved through the mounted union exactly as the must_have's "round-trip under the union-mounted /env" names. (SEEK itself is the 05-03-traced set for a later plan — deferred-items.md.)

## Task Commits

Each task was committed atomically (two-repo convention — r9 commits land on `main` of `requaos/r9`):

1. **Task 1 RED: failing devenv tests + RED evidence record** — r9 `40bab8e` (test)
2. **Task 1 GREEN: devenv + registry 'e' + rfork env semantics + kernel seed + argv-hint removal** — r9 `3bdb410` (feat)
3. **Task 2: devroot /rootfs.img hardening + host tests** — r9 `cf174ca` (feat)
4. **HowItWorks.md material-change section (the header's mandate)** — r9 `628d612` (docs)

No REFACTOR commit — the GREEN code was fresh-written and clippy-clean; nothing needed a cleanup pass (tdd.md: commit only on change). The RED-phase test bug (asserted 11 bytes for the 10-byte replacement value) was fixed inline during GREEN and rides the GREEN commit — the test was wrong, not the implementation (tdd.md error handling).

**Plan metadata:** (the harvey docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS + the evidence log + deferred-items)

## Files Created/Modified

- `../r9/port/src/devenv.rs` — the per-pgrp env device: EnvGroup + the `#e` op surface + the group hook + root-group seeding
- `../r9/port/src/namec.rs` — registry 'e' + the DEV_ENV walk/open/stat dispatch legs (devenv's open gets the full mode — OTRUNC honored)
- `../r9/port/src/dat.rs` — DEV_ENV = 3 device index
- `../r9/port/src/lib.rs` — `pub mod devenv;`
- `../r9/port/src/devroot.rs` — the mount-time collision refusal + named raw-node refusals + the two-views module documentation
- `../r9/aarch64/src/proc.rs` — the `env_group` field + rfork branches (share/copy/empty) + `rfork_env_split` (non-RFPROC caller split) + `current_env_group` (the hook)
- `../r9/aarch64/src/syscall.rs` — DEV_ENV pread/pwrite legs (offsets honored); RFENVG|RFCENVG in SUPPORTED + the non-RFPROC env split
- `../r9/aarch64/src/main.rs` — hook registration + root-group seed + the `devenv: seeded …` line before the first exec; pid 1 runs the root group; argv = [boot] (the 05-04 hint retired)
- `../r9/HowItWorks.md` — the 05-05 section (devenv + seeding + rawimg hardening)
- `../r9/port/tests/devenv_test.rs` + `devroot_rawimg_test.rs` + `05-05-red-evidence.json` — the TDD suites + RED evidence
- `.planning/phases/05-shell-boot-one-command-integration/05-05-boot-r9.log` — the committed gate evidence
- `.planning/phases/05-shell-boot-one-command-integration/deferred-items.md` — the phase's deferred register (new)

## Decisions Made

- **The donor governs the rfork env flags** (Deviation 1 — the 05-04 inversion pattern's second instance; see Deviations).
- **Dynamic group resolution** (the injected hook) over capture-at-attach — donor-exact `envgrp(c)`; the divergence surface (cross-group chan use) is pinned as a design property by test.
- **Minimal seed** (the three mandatory keys); `service=terminal`/`user=glenda` deliberately left to init's setenv/newns (the donor's flow — the seed is the KERNEL's contract, not the login's).
- **#ec rejected loudly** at attach (non-empty spec) — the seeded root group replaces the config-env mechanism (Pattern 3); boot's `#ec` bind is the designed fail-soft path (05-04 proved it).
- **Device create is strict Eexist-on-dup**; the create-on-existing → OTRUNC-open fallback is namec's Acreate contract (9k chan.c), landing with sys_create in 05-06 — the op slots in unchanged.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan text bug] the truth bullet's RFENVG/RFCENVG semantics are inverted vs fork(2)**
- **Found during:** Task 1 RED test design (cross-checking the bullet against sys/man/2/fork and 9k sysproc.c — the 05-04 RFNAMEG/RFCNAMEG precedent made the check routine)
- **Issue:** the must_have truth + action text say "RFENVG shares, RFCENVG copies". fork(2) (sys/man/2/fork: "RFENVG: the environment variables are COPIED; otherwise the two processes SHARE… RFCENVG: the new process starts with an EMPTY environment") and 9k sysproc.c (RFENVG → envcpy; RFCENVG → fresh empty Egrp; neither → incref-share) say the opposite. D-07's operative clause ("behave exactly like Plan 9") and newns's `rfork(RFENVG|RFCNAMEG)` are only coherent under the donor reading (a shared newns env would leak per-login user=/home= into init's group).
- **Fix:** implemented + tested the donor/fork(2) semantics (neither=share, RFENVG=copy, RFCENVG=empty; the non-RFPROC caller split matches); the inversion is recorded in the test file's header, the proc.rs/syscall.rs comments, HowItWorks.md, and WINDOWS.md entry 15.
- **Files modified:** `../r9/aarch64/src/proc.rs`, `../r9/aarch64/src/syscall.rs`, `../r9/port/tests/devenv_test.rs` (header note)
- **Verification:** `rfork_env_semantics_share_copy_empty` pins the matrix; the boot gate exercises the seeded env end-to-end
- **Committed in:** `3bdb410`

**2. [Rule 3 - Blocking] sys_pread/sys_pwrite need DEV_ENV dispatch legs (the plan's "add nothing, verify only" was wrong)**
- **Found during:** Task 1 design — the plan's action step 5 says syscall.rs gets "no new handlers… add nothing, verify only", but reads/writes dispatch by `chan.dev`: without DEV_ENV legs, boot's `readfile("#e/cputype")` would open fine (namec) then fail the READ with "unknown device" — readfile fails soft to an EMPTY string, rootserver's nobootprompt read misses, and the gate hangs at outin()
- **Fix:** DEV_ENV legs in sys_pread/sys_pwrite (offsets honored — donor envwrite's grow-at-offset + the streaming convention). The plan's files_modified already named syscall.rs; the registry/namec twins got the walk/open/stat legs the same way
- **Files modified:** `../r9/aarch64/src/syscall.rs`, `../r9/port/src/namec.rs`
- **Verification:** the gate's `paqfs...` marker proves the #e reads landed; host tests pin the read/write ops
- **Committed in:** `3bdb410`

**3. [Rule 3 - Blocking] the plan's verify build order would gate the wrong kernel (the 05-04 lesson recurred)**
- **Found during:** Task 1 verify prep — the plan's build legs run virtshell FIRST and raspi4b LAST; all aarch64 configs share `target/aarch64-unknown-none-elf/debug/aarch64`, so the gate would boot the raspi kernel (05-04 deviation 2's exact trap, which this plan's verify text did not absorb)
- **Fix:** ran the legs virtexec → default → raspi4b → clippy, then virtshell LAST so the gate's ELF is the virtshell kernel; all four legs + clippy green
- **Files modified:** none (ordering only)
- **Verification:** the gate booted the virtshell kernel (banner + the full chain)
- **Committed in:** n/a (ordering)

**4. [Rule 1 - Convention] the plan's "errstr on overrun" read against the donor convention**
- **Found during:** Task 2 design — the action text says the raw node's reads are "bounds-checked, errstr on overrun"; the paqfs reader's `read_at` (and every Plan 9 device read — envread included) CLAMPS: at/past the end is EOF = 0, never an error
- **Fix:** kept the 05-04 clamp (donor-consistent; the acceptance criterion's "bounds checks" is the operative phrase); the paqfs server reading the image gets EOF at the end exactly like reading any file. The "errstr" energy went where the plan's acceptance criteria put it: named write/exec refusals
- **Files modified:** none (behavior kept; documented in the devroot.rs RAWIMG block + here)
- **Verification:** `rawimg_node_walk_stat_read_and_bounds` pins EOF=0 at/past the end + the spanning-read clamp
- **Committed in:** n/a (no change)

---

**Total deviations:** 4 (2 Rule 1, 2 Rule 3) — plus one RED-phase test bug fixed inline (byte-count assertion; rides the GREEN commit, not counted as a plan deviation)
**Impact on plan:** The flag-semantics correction is the phase's second plan-text inversion — the donor-governs precedent held cleanly. The dispatch legs were strictly necessary for the gate (the plan's files_modified anticipated them). No scope creep: nothing beyond the plan's named surface was implemented (create stays device-side until 05-06's sys_create, per the plan's own ordering).

## Issues Encountered

- **qemu/fdtput are devshell-provided, not ambient-PATH** — the plan's gate command presumes the r9 devshell PATH (05-04's documented pattern: "`nix develop` from ../r9, then the gate command from this repo root"); ran the verbatim command inside `nix develop -c sh -c '…'`. Environment setup, not a plan change.
- **05-04 never got its HowItWorks.md section** (the namespace core is material; the doc's header mandates updates) — out of this plan's scope; logged in the phase's new `deferred-items.md` alongside the getenv/SEEK note.

## User Setup Required

None - no external service configuration required. (The gate driver needs qemu-system-aarch64 + fdtput on PATH — the r9 devshell provides both: `nix develop` from ../r9, then the gate command from this repo root.)

## Known Stubs

- **CREATE(22) stays loud-unimplemented until 05-06 wires sys_create** — plan-mandated ordering ("CREATE rides the sys_create handler — 05-06 lands create"); devenv's create op is host-tested and slots in unchanged (init's setenv/cpenv get their live exercise past the pipe wall). WINDOWS.md entry 14.
- **`#ec` attach errors loudly by design** (the seeded root group replaces the config-env mechanism this era) — boot's `bind("#ec", …)` is the designed fail-soft path, not a stub.
- **The RFENVG/RFCENVG wiring has no EL0 caller until newns** (post-pipe-wall, 05-06+) — primitives host-pinned; live exercise arrives with init's namespace rebuild. (Coverage D4 carries the judgment note.)
- 05-04's known stubs are unchanged (cons reads -1 until 05-08; devproc dir/status/args until 05-08/05-09).

## Threat Flags

(None new — the register held verbatim: T-05-08's per-pgrp isolation is host-pinned (share/copy/empty + two-group isolation) and the seeded set carries no secrets (cputype/method names only); T-05-09's raw node is read-only with bounds-checked reads, named write/exec refusals, and the new mount-time collision refusal is a hardening GAIN; T-05-SC: zero package installs. The devenv write path's bounded heap growth (MAXENVSIZE=16300 per key) is inside T-05-08's component and validated-buffer-gated (sys_pwrite's T-04-11 validate_user_range runs before dispatch); the dynamic-resolution design keeps cross-group fd use donor-exact, host-pinned.)

## Next Phase Readiness

- **05-06** (devpipe/devdup + the paqfs server exec — the pipe wall): boot's connectembed will traverse pipe→fork→dup→exec; the paqfs server then opens `/rootfs.img` for the raw node's live read exercise, and sys_create wires devenv's create op (init's setenv/cpenv run for real). Everything the wall needs is in place: the env-driven method pick, the raw byte source, the union-mounted /env.
- **05-08**: cons reads + the namespace-file re-mount stanza; newns's `$var` expansion reads `#e/<var>` — the seeded group serves it (and getenv's seek-to-end sizing wants SEEK(39) — deferred-items).
- r9 HEAD: `628d612` (main). Harvey HEAD: `8743f2328` + the metadata commit.
- The staged-profile fan is green through devenv (all five profiles); the seed print joins every dev_exec log harmlessly (marker-presence discipline holds).

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: r9 port/src/devenv.rs, port/tests/{devenv_test.rs,devroot_rawimg_test.rs,05-05-red-evidence.json}, HowItWorks.md; harvey .planning/phases/05-shell-boot-one-command-integration/{05-05-SUMMARY.md,05-05-boot-r9.log,deferred-items.md} — all FOUND on disk and (r9 side) committed.
- Commits: r9 40bab8e (RED test), 3bdb410 (GREEN feat), cf174ca (Task 2 feat), 628d612 (docs) — all FOUND in the r9 git log; RED precedes GREEN (TDD gate sequence holds).
- Plan-level <verification>: (1) `cargo test -p port` 44/44 (6 devenv + 3 rawimg new); (2) builds virtshell/virtexec/default/raspi4b + clippy virtexec green (virtshell built LAST — the 05-04 shared-artifact lesson); (3) the boot gate (05-05-boot-r9.log): 'devenv: seeded cputype=aarch64 nobootprompt=embed readparts=0' + 'argv=["boot"]' + 'paqfs...' + 'syscall 21 unimplemented' + clean exit, zero 'panic|Unhandled interrupt'; the cross-plan literal grep holds (devroot.rs + bootr9.c); all five boot-test profiles PASS + ureg-check 35/35.
- Acceptance criteria re-run at final state: Task 1 (seeded keys served through #e with host-tested per-group semantics; RFENVG/RFCENVG real; env-only method selection; all green) — PASS. Task 2 (/rootfs.img resolves, stats to image size, bounds-checked reads, named write refusal, loud mount-time collision refusal, gate green, host tests cover read/stat/bounds) — PASS.
