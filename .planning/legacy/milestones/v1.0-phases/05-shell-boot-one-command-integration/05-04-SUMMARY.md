---
phase: 05-shell-boot-one-command-integration
plan: 04
subsystem: kernel
tags: [r9, aarch64, namespace, namec, pgrp, bind, mount-table, union-walk, device-registry, devcons, devproc, devroot, stat, brk, rfork, boot, tdd, qemu]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-01)
    provides: the hardened boot-test harness (fresh log, process-group kill) and the green staged profiles this plan runs through the new machinery
  - phase: 05-shell-boot-one-command-integration (05-02)
    provides: out/{boot,init,paqfs,stacksmoke} + the baked bootfs.paq + run-r9.sh (the gate driver) + the /rootfs.img contract literal
  - phase: 05-shell-boot-one-command-integration (05-03)
    provides: 05-03-trace.txt — the measured 26-syscall set this plan implements against (bind/chdir/unmount/stat/fstat/brk_ from it) + the RFCNAMEG-wipe finding
provides:
  - the D-03 namespace core on r9: port::namec (path resolution, union walk, mount stitching, rune-aware #x parse) + port::pgrp (per-target ordered union chains, cunmount, pgrpcpy) + the hand-built rune→attach registry — sys_open's prefix routing is RETIRED
  - syscall handlers: sys_bind, sys_chdir, sys_unmount, sys_stat, sys_fstat, sys_brk_ (all T-04-11 validation-first)
  - rfork namespace semantics per fork(2): share (neither flag) / copy (RFNAMEG) / wipe (RFCNAMEG), incl. the non-RFPROC split (boot's call)
  - devcons registry attach serving cons/null/user/hostowner; devproc attachable as #p; devroot's /rootfs.img raw-image node
  - exec's cold-syscall-arg fix: data+bss mapped eagerly (phase-04 deferred #3 lands); text/stack/tos stay demand-paged
  - config_virtshell.toml (dev = ["exec","bootchain"], no xtask initrd) + the proven end-to-end slice: Harvey-baked image → r9 execs /boot/boot as pid 1 → EL0 boot code to the loud PIPE wall
  - evidence: .planning/phases/05-shell-boot-one-command-integration/05-04-boot-r9.log (the gate run)
affects: [05-05 (devenv seeds the env-driven method pick, replacing the argv hint; hardens /rootfs.img), 05-06 (devpipe/devdup + the paqfs server exec), 05-07 (devmnt mount rides this plan's mount table), 05-08 (cons reads + the namespace-file re-mount stanza), 05-09 (smoke), all later device plans (the registry is where they attach)]

# Actuals (#2632) — pairs with the plan's estimate (140000 tokens, confidence: low).
actuals:
  tokens: 33460    # MEASURED chars/4: r9 diff 102387 + harvey diff 853 + this SUMMARY 30603 = 133843 chars; the estimate (140k) overshot ~4.2x
  tasks: 3
  commits: 6       # MEASURED: r9 git rev-list --count d70e6c7..1344eba = 5; harvey a58bbf47c..2a53066a2 = 1; the metadata commit rides on top
plan_head_before: {r9: "d70e6c7d16d6042b865fac33b34af5132072437c", harvey: "a58bbf47c3ef6e847f5c24629b44c0c9789e8c33"}
plan_head_after: {r9: "1344eba320e72ffe14e7df1a513d68f876edc180", harvey: "2a53066a26eb93ef89d269c108920e3da69bc237"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies anywhere (T-05-SC held: no package installs)
  patterns:
    - "the D-03 namespace core shape: hand-built rune→attach registry (no generated devtab), per-target ordered union chains (9k cmount order: MBEFORE latest-first, underlying, MAFTER bind-order), mount-point redirect per amode (Aopen redirects, Amount/Atodir do not)"
    - "syscall-argument residency: data+bss map EAGERLY at exec (7c literals/statics are passed to syscalls with no prior EL0 touch); validate-then-deref stays the syscall discipline; text/stack/tos keep D-09's demand model (SC#3's pinned wording)"
    - "kernel namespace semantics host-tested: synthetic paqfs image builder + Arc<Lock<Pgrp>> group handles in port/tests/ — semantics pinned before any boot"
    - "staged gates establish their own namespace the donor way (bind #p at /proc in the staged init) — kernel-side routing scaffolds never return"
    - "two-repo gate choreography: all aarch64 configs share target/aarch64-unknown-none-elf/debug/aarch64 — the config under gate builds LAST"

key-files:
  created:
    - ../r9/port/src/namec.rs
    - ../r9/port/src/pgrp.rs
    - ../r9/port/tests/namespace_test.rs
    - ../r9/port/tests/05-04-red-evidence.json
    - ../r9/aarch64/lib/config_virtshell.toml
    - .planning/phases/05-shell-boot-one-command-integration/05-04-boot-r9.log
  modified:
    - ../r9/aarch64/src/main.rs
    - ../r9/aarch64/src/exec.rs
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/proc.rs
    - ../r9/port/src/dat.rs
    - ../r9/port/src/lib.rs
    - ../r9/port/src/devcons.rs
    - ../r9/port/src/devproc.rs
    - ../r9/port/src/devroot.rs
    - ../r9/aarch64/Cargo.toml
    - ../r9/xtask/src/main.rs
    - build/aarch64/init.c

key-decisions:
  - "exec maps data+bss eagerly (05-04; phase-04 deferred #3 lands): the tracer proved real binaries pass cold syscall-arg pages (7c literals in never-touched data pages); T-05-06's no-nested-demand-paging rule holds; D-09's pinned wording (faulting in user TEXT, growing the user STACK) is untouched"
  - "rfork namespace semantics follow fork(2)/9k sysrfork — the plan's behavior bullet inverted RFNAMEG/RFCNAMEG: the donor governs (RFPROC+neither shares the Arc<Lock<Pgrp>>, RFNAMEG deep-copies, RFCNAMEG wipes); the 05-03 RFCNAMEG-wipe finding is the in-phase binding evidence"
  - "the staged init binds #p at /proc itself (connectembed's fatal-bind shape) — the namespace era retires the routing scaffold the staged gates relied on; the xtask initrd tree ships the empty proc/ dir (Pitfall-8)"
  - "devroot synthesizes /rootfs.img pulled forward from 05-05 Task 2 — this plan's gate needs dirstat(paqfile) to succeed; a real image entry always wins (no shadowing); 05-05 hardens"
  - "sys_brk_ landed here per the orchestrator's dispatch note (brk_ ×28 must work) — the plan's Task-3 action text named only the stat family, but boot's dirstat mallocs are gate-load-bearing"

patterns-established:
  - "device registry era: namec owns all path resolution; devices expose attach/walk/open/read/write/stat as free functions dispatched by chan.dev; the '#x' parse is rune-aware ('#æ', '#κ' decode to clean unknown-device errors)"
  - "mount entries carry the frozen flag bits + a mount id; the underlying directory joins the union as a chain entry (mountid 0) iff the first bind at the point was not MREPL"
  - "the stat(5) wire writer lives in dat.rs (StatWriter) and every device serves the same shape; stat returns the byte count (convD2M contract)"

requirements-completed: [INT-01]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "TRACER slice: the Harvey-baked image's /boot/boot execs as pid 1 on r9 (two-component devroot walk), runs EL0 through close×3 to the loud BIND wall, zero panic lines; the evidence print names the path and argv"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "R9_KERNEL=<virtshell ELF> R9_DTB=... sh build/aarch64/run-r9.sh --log build/aarch64/boot-r9.log on the Task-1 tree: 'r9 from the Internet' + 'devroot: /boot/boot open (163116 bytes)' + 'exec: entering EL0 pc=0x1510c ... path=/boot/boot argv=[\"boot\"]' + 'syscall 2 unimplemented' ×4; zero 'panic|Unhandled interrupt' lines (the log was then superseded by the Task-3 gate run in the same file — the tracer's marker sequence is quoted in Accomplishments)"
        status: pass
      - kind: integration
        ref: "cargo xtask build --config virtshell + --config default + --config raspi4b (three legs green); boot-test virt/virtexec/virtmulti/virtnotes/virtpanic all PASS on the Task-1 tree"
        status: pass
    human_judgment: false
  - id: D2
    description: "The namespace core: namec (registry + union walk + mount stitching) + pgrp (mount table, cunmount, pgrpcpy) + rfork RFNAMEG/RFCNAMEG/share semantics, host-tested; sys_open's prefix routing retired (comment-stripped scan); BIND/CHDIR/UNMOUNT implemented with validated user-VAs"
    requirement: INT-01
    verification:
      - kind: unit
        ref: "cargo test -p port — port/tests/namespace_test.rs 9/9 (registry resolution, two-component walk, MBEFORE/MREPL/MAFTER union order, bind-target existence, share/copy/wipe, unmount) + all pre-existing suites green"
        status: pass
      - kind: integration
        ref: "builds virtshell/virtexec/default/raspi4b + clippy virtexec (no new warnings) + boot-test fan all PASS through namec + ureg-check 35/35"
        status: pass
    human_judgment: false
  - id: D3
    description: "GATE: boot runs on r9 through opencons (bind #c→/dev MBEFORE + three /dev/cons opens through the union), fail-soft bindenvsrv, argv-hinted embed pick, glenda auth no-op (#c/hostowner write stores), rfork(RFNAMEG), connectembed's dirstats (/boot/paqfs + /rootfs.img through devroot stat) + fatal binds #c/#p — the serial shows 'paqfs...' and stops at the loud PIPE wall (syscall 21), never a hang, never a panic"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: ".planning/phases/05-shell-boot-one-command-integration/05-04-boot-r9.log (committed): 'paqfs...' + 'syscall 21 unimplemented' + 'boot: pipe: syscall not implemented' (boot's own fatal print on the union-opened fd 2) + 'process 1 exited' + 'all user processes exited'; zero panic/Unhandled-interrupt; zero 'can't exec paqfs'"
        status: pass
      - kind: integration
        ref: "boot-test all five profiles PASS post-gate; raspi4b + default builds green"
        status: pass
    human_judgment: false
  - id: D4
    description: "exec cold-arg fix: data+bss eager at exec (syscall-argument residency class); the staged gates' binaries unregressed (behavior identical, residency earlier)"
    requirement: INT-01
    verification:
      - kind: integration
        ref: "the full five-profile boot-test fan green on the final tree (twice: after the fix and after the namespace core); stacksmoke depth-64 marker intact (demand stack growth still execution-evidenced)"
        status: pass
    human_judgment: false

# Metrics
duration: 55 min
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 04: The namespace core (D-03) + the boot(8)-to-PIPE-wall gate Summary

**r9 resolves paths for real now — namec + pgrp mount table + union walk + the hand-built #c/#p device registry, with sys_open's Phase-4 prefix routing retired (the D-03 one-way door walked) — and Harvey's real boot(8) runs on r9 as pid 1 through opencons (the union-mounted console), the fail-soft env/srv binds, the argv-hinted embed pick, the glenda auth no-op, rfork(RFNAMEG), and connectembed's dirstats + fatal binds, printing `paqfs...` and stopping loudly at the unimplemented PIPE syscall — panic-free, with all five staged boot-test profiles green through the new machinery. The tracer-first shape paid off immediately: it surfaced the cold-syscall-argument class (phase-04 deferred #3), fixed by mapping data+bss eagerly at exec while text/stack/tos keep D-09's demand model.**

## Performance

- **Duration:** 55 min
- **Started:** 2026-11-03T05:01:20Z
- **Completed:** 2026-11-03T05:56:38Z
- **Tasks:** 3
- **Files modified:** 18 (16 r9 + 2 harvey, counting the host-test suite, the RED evidence record, and the committed gate log)

## Accomplishments

- **The tracer slice proved the end-to-end chain before the namespace work started.** Task 1's gate log showed the full slice: banner → `devroot: /boot/boot open (163116 bytes)` → `exec: entering EL0 pc=0x1510c sp=0x7fffffffffd0 path=/boot/boot argv=["boot"]` → demand page-ins → close×3 → `syscall 2 unimplemented` ×4 (opencons's bind + bindenvsrv's three) → further walls (brk_ ×2, semrelease) → clean exit — zero panic lines. The two-component devroot walk generalizes the entry path; `config_virtshell.toml` (dev = ["exec","bootchain"], no xtask initrd — the D-10/D-11 shape emerges) selects /boot/boot while every pre-existing profile keeps /init. **The tracer immediately did its job**: the first run died at an EL1h DataAbort on `open("/dev/cons")`'s path string — a 7c string literal in a data page no EL0 code had ever touched (phase-04 deferred #3's cold-pointer class, previously masked by libc fmt always reading the measured binaries' buffers first).
- **exec maps data+bss eagerly (the cold-arg fix).** install_image now maps the data+bss pages at exec (zero-fill + the 04-06 byte-exact content clamp through the inactive-table KZERO+PA alias path, logged SegKind::Data/Bss so rfork-copy/exit-free cover them). Text, stack and Tos stay demand-paged — D-09's pinned wording ("faulting in user text and growing the user stack") is untouched, and T-05-06's no-nested-demand-paging rule holds everywhere. All five profiles stayed green through the change; stacksmoke's depth-64 marker (demand stack growth) still fires.
- **The namespace core landed TDD-style.** RED: 9 host tests + compiling skeletons, evidence record verified `RED_EVIDENCE_OK` (target_test_failed; exit 101; 8 of 9 failing) at `port/tests/05-04-red-evidence.json` (r9 commit 68f2692). GREEN: `port::namec` (rune-aware '#x' registry parse — '#æ'/'#κ' decode to clean unknown-device errors; absolute/relative walks from slash/dot; mount-point crossing + union fallback in 9k chain order; the final-component redirect per the donor's amode) and `port::pgrp` (per-target ordered union chains: MREPL replaces, the first non-MREPL bind joins the underlying directory, MBEFORE head-inserts latest-first, MAFTER appends; cunmount by whole-point or named-entry; pgrpcpy deep copy). All 9 tests pass, plus every pre-existing port suite. sys_open routes everything through namec (the comment-stripped grep scan proves no routing remnants); sys_bind/sys_chdir/sys_unmount follow the validation-first handler contract.
- **rfork's namespace semantics per fork(2)** — the plan's behavior bullet inverted RFNAMEG/RFCNAMEG (see Deviations): RFPROC with neither flag shares the `Arc<Lock<Pgrp>>` handle; RFNAMEG deep-copies; RFCNAMEG wipes; and the non-RFPROC split re-groups the CALLER (boot's `rfork(RFNAMEG)` — "leave existing subprocs in own namespace").
- **The gate.** The committed evidence log (05-04-boot-r9.log) shows the full designed sequence: exec entry `path=/boot/boot argv=["boot", "embed"]` → opencons (bind #c→/dev MBEFORE + three /dev/cons opens through the union — boot's own prints flow on the reopened fds) → bindenvsrv's #ec/#e/#s binds fail soft (no registry entries) → `#e/cputype` read fails soft → kbmap/usbinit no-ops → rootserver's argv-hinted embed pick → the #æ/#S fail-soft binds → partinit's readparts over the devcons root listing (no sd* — vacuous) → the glenda no-op (`#c/hostowner` write stores "glenda") → rfork(RFNAMEG) → connectembed's dirstats (`/boot/paqfs` via devroot stat; `/rootfs.img` via the synthesized raw node) → `paqfs...` → fatal binds #c/#p MREPL → **the loud PIPE wall** (`syscall 21 unimplemented`) → `boot: pipe: syscall not implemented` (boot's fatal print on fd 2) → `process 1 exited` → `all user processes exited`. Zero `panic`/`Unhandled interrupt` lines; zero `can't exec paqfs` (exec is correctly never attempted before pipe returns).

## Task Commits

Each task was committed atomically (two-repo convention — r9 commits land on `main` of `requaos/r9`):

1. **exec cold-arg fix (found by the tracer)** — r9 `17a5898` (fix)
2. **Task 1: TRACER — /boot/boot execs as pid 1 to the BIND wall** — r9 `244359e` (feat)
3. **Task 2 RED: failing namespace-core host tests + evidence record** — r9 `68f2692` (test)
4. **Task 2 GREEN: namespace core (namec + pgrp + registry + BIND/CHDIR/UNMOUNT + rfork semantics) + xtask initrd proc/ dir** — r9 `0cd1e20` (feat)
5. **Task 2 GREEN, harvey side: staged init binds #p at /proc** — harvey `2a53066a2` (feat)
6. **Task 3: GATE — stat family + brk_ + device attaches + the pipe wall** — r9 `1344eba` (feat)

No REFACTOR commit — the GREEN code was fresh-written and clippy-clean; nothing needed a cleanup pass (tdd.md: commit only on change).

**Plan metadata:** (the harvey docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS + the evidence log)

## Files Created/Modified

- `../r9/port/src/namec.rs` — the namespace core: registry, component walk, union fallback, final-redirect amode, open/stat dispatch
- `../r9/port/src/pgrp.rs` — the mount table (ordered union chains) + cunmount + pgrpcpy
- `../r9/port/src/dat.rs` — frozen M* flag values + MountEntry + the shared StatWriter + QTDIR/QTFILE
- `../r9/port/src/lib.rs` — `pub mod namec; pub mod pgrp;`
- `../r9/port/src/devcons.rs` — the registry surface: attach/walk/open/read/write/stat for cons/null/user/hostowner + the root dir listing
- `../r9/port/src/devproc.rs` — stat (the dispatch's total coverage; the note surface unchanged)
- `../r9/port/src/devroot.rs` — the /rootfs.img raw-image node + bounds-checked slice reads + stat byte count
- `../r9/aarch64/src/syscall.rs` — sys_bind/sys_chdir/sys_unmount/sys_stat/sys_fstat/sys_brk_; sys_open rewritten through namec; sys_rfork's group semantics; dispatch growth
- `../r9/aarch64/src/proc.rs` — pgrp/slash/dot fields + rfork flag wiring + the non-RFPROC split
- `../r9/aarch64/src/main.rs` — the entry-path walk + cfg selection + slash/dot seeding + the transitional argv hint + the self-describing exec-entry print
- `../r9/aarch64/src/exec.rs` — eager data+bss (the cold-arg fix)
- `../r9/aarch64/lib/config_virtshell.toml` — the shell-chain kernel build config
- `../r9/aarch64/Cargo.toml` — check-cfg predeclares cfg(dev_bootchain)
- `../r9/xtask/src/main.rs` — the initrd tree ships the empty proc/ dir (Pitfall-8)
- `../r9/port/tests/namespace_test.rs` + `05-04-red-evidence.json` — the TDD suite + RED evidence
- `build/aarch64/init.c` — the staged init's #p bind (the donor shape)
- `.planning/phases/05-shell-boot-one-command-integration/05-04-boot-r9.log` — the committed gate evidence

## Decisions Made

- **Cold-arg residency is the loader's job, not the syscall layer's** (D-09-letter vs T-05-06 reconciliation): eager data+bss at exec; the demand model stays for text/stack/tos; nested demand-paging from syscall context stays forbidden. Deferred #3 is closed by construction for the argument class (literals/statics/bss buffers).
- **fork(2) governs rfork** (the plan-text inversion resolved to the donor — see Deviations 3).
- **The staged init establishes its namespace in userland** (bind #p at /proc) rather than the kernel pre-binding — the donor's shape; the staged initrd ships the empty target dir.
- **/rootfs.img lives at the devroot root, synthesized only when the image ships no real entry** (no shadowing; 05-05 Task 2's collision-hardening remains meaningful).
- **brk_ is the bss-break set** (9k's ibrk/BSEG): growth pages demand-fault as Bss zero pages; shrink lowers the recorded break without unmapping (malloc never shrinks this era — documented approximation); the break never crosses USTOP.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] exec maps data+bss eagerly (the tracer's cold-pointer finding)**
- **Found during:** Task 1 (the tracer's first gate run died at an EL1h DataAbort: `open("/dev/cons")`'s path literal sat in a never-touched data page, far=0x369c8)
- **Issue:** the plan's zero-panic tracer gate (and Task 3's gate harder — sys_bind reads the same cold literals) cannot pass with membership-only validate_user_range, and T-05-06 forbids nested demand-paging from syscall context. Phase-04 deferred #3 explicitly earmarked Phase 5 as where this lands ("if real binaries ever pass cold pointers" — boot(8) does).
- **Fix:** install_image maps data+bss pages at exec (zero-fill + byte-exact content clamp via the KZERO+PA alias; logged for rfork/exit). Text/stack/tos keep demand paging. D-09's SC#3-pinned wording (text fault-in + stack growth) is untouched.
- **Files modified:** `../r9/aarch64/src/exec.rs` (+ comment updates in main.rs)
- **Verification:** tracer gate PASS with zero panic lines; all five boot-test profiles green through the change (twice); ureg-check 35/35
- **Committed in:** `17a5898`

**2. [Rule 3 - Blocking] the plan's verify build order would gate the wrong kernel**
- **Found during:** Task 1 (first gate run produced a zero-byte serial log: the three build legs share `target/aarch64-unknown-none-elf/debug/aarch64`, so the raspi4b leg — last in the plan's verify — overwrote the virtshell ELF, and a raspi kernel has no console on the virt machine)
- **Fix:** the three-leg regression evidence stands (all green), then virtshell rebuilds LAST so the gate's `$(ls …/debug/aarch64)` is the virtshell kernel. Recorded for 05-05+: any plan whose verify builds multiple configs before a gate run must rebuild the gated config last.
- **Files modified:** none (ordering only)
- **Verification:** the re-run gate PASSed with the virtshell ELF
- **Committed in:** n/a (ordering)

**3. [Rule 1 - Plan text bug] the behavior bullet's RFNAMEG/RFCNAMEG semantics are inverted vs fork(2)**
- **Found during:** Task 2 RED test design (cross-checking the bullet against sys/include/libc.h's frozen values and 9k sysproc.c)
- **Issue:** the bullet says "RFNAMEG shares, RFCNAMEG deep-copies"; fork(2) says "RFNAMEG: the new process inherits a COPY of the parent's name space; otherwise [neither] it SHARES; RFCNAMEG: a CLEAN name space". 9k sysrfork implements copy/wipe/share exactly so, and the phase's own binding 05-03 finding ("newns's RFCNAMEG wipe discards boot's mount table") is empirical proof. boot's `rfork(RFNAMEG)` comment ("leave existing subprocs in own namespace") only makes sense under the donor reading.
- **Fix:** implemented and tested the donor/fork(2) semantics (share on neither, copy on RFNAMEG, wipe on RFCNAMEG, plus the non-RFPROC re-group); the inversion is recorded in the test file's header and here.
- **Files modified:** `../r9/port/src/pgrp.rs`, `../r9/aarch64/src/proc.rs`, `../r9/aarch64/src/syscall.rs`, `../r9/port/tests/namespace_test.rs` (header note)
- **Verification:** the share/copy/wipe host test pins the semantics; the Task-3 gate exercises rfork(RFNAMEG) live (boot's call)
- **Committed in:** `0cd1e20`

**4. [Rule 3 - Blocking, pre-existing wave-2 regression] the r9 boot-test supply chain was poisoned by 05-02's out/ overwrite**
- **Found during:** Task 2 verify leg 4 (virtexec FAILED: `syscall 6 unimplemented` at pid-1 startup — `out/init` was the REAL init (05-02's subset-build wrote it last; it calls `alarm(0)`), and the boot-test initrd baked it)
- **Fix:** re-ran stage-build.sh (the documented restore path — "last writer wins"); then, because the namespace era retired the literal-path routing the staged init relied on for `/proc/N/note`, the staged init gained the donor shape: `bind("#p", "/proc", MREPL)` before postnote (fatal on failure), and the xtask initrd tree ships the empty `proc/` dir (Pitfall-8). The gates now exercise the real namespace machinery.
- **Files modified:** `build/aarch64/init.c`, `../r9/xtask/src/main.rs`
- **Verification:** all five profiles PASS (virtnotes proves the note path through bind+namec+devproc)
- **Committed in:** `0cd1e20` (xtask) + `2a53066a2` (init.c)

**5. [Rule 3 - Blocking] /rootfs.img pulled forward from 05-05 Task 2**
- **Found during:** Task 3 design (embed.c gates `paqfs...` on `dirstat(paqfile)` succeeding; bootr9.c's method arg is the literal `/rootfs.img`; the plan's files_modified omits devroot.rs while its gate requires the node)
- **Fix:** devroot synthesizes the raw-image node at the root when the image ships no real entry (a real entry always wins — no shadowing): walk + open OREAD + stat (mode 0444, length = image size) + bounds-checked slice reads; write refused loudly. 05-05 Task 2's hardening (collision handling, gate exercise when the paqfs server opens it) remains meaningful.
- **Files modified:** `../r9/port/src/devroot.rs`
- **Verification:** the gate log shows `paqfs...` (both dirstats succeeded)
- **Committed in:** `1344eba`

**6. [Rule 2 - Missing Critical] sys_brk_ (BRK_=24) implemented in Task 3**
- **Found during:** Task 3 design — dirstat's `malloc` grows the pool via sbrk→brk_; with brk_ unimplemented, malloc returns nil → dirstat fails → connectembed returns -1 before `paqfs...`. The tracer's log already showed two `syscall 24 unimplemented` walls from boot's soft paths. The orchestrator's dispatch notes bind brk_ ("brk_ ×28 must work — rc malloc growth"); the 05-03 trace measured it ×28.
- **Fix:** sys_brk_ = the bss-break set (9k ibrk/BSEG): bounds-checked (bss base, USTOP cap), growth pages demand-fault as Bss zero pages, 0/-1 on the wire per libc's sbrk contract.
- **Files modified:** `../r9/aarch64/src/syscall.rs`
- **Verification:** the gate's dirstats succeeded (malloc works); all profiles green
- **Committed in:** `1344eba`

**7. [Rule 3 - Blocking] devcons.rs touched in Task 2 (plan assigns its surface to Task 3)**
- **Found during:** Task 2 GREEN (the registry's 'c' → devcons::attach target must exist for namec to resolve `#c/cons` — the Task-2 test's first bullet)
- **Fix:** Task 2 landed the minimal cons-only attach/walk/open; Task 3 completed the file set (null/user/hostowner + the root listing) per the plan's Task-3 action.
- **Files modified:** `../r9/port/src/devcons.rs` (both commits)
- **Verification:** the Task-2 registry test and the Task-3 gate both pass
- **Committed in:** `0cd1e20` + `1344eba`

---

**Total deviations:** 7 auto-fixed (2 Rule 1/2 correctness classes, 5 Rule 3 blocking — one of them pre-existing wave-2 fallout)
**Impact on plan:** Every deviation is the tracer/gate shape doing its job — each was caught by its own evidence. The D-09-letter adjustment (eager data+bss) is the one judgment call with architectural flavor; it preserves both binding constraints (T-05-06's rule and SC#3's pinned wording) and closes phase-04 deferred #3 for the syscall-argument class. No scope creep: nothing beyond the measured set + the dispatch note was implemented.

## Issues Encountered

- **The plan's verify build order shares one artifact path across configs** (Deviation 2) — fixed by rebuilding the gated config last; noted for later plans.
- **out/ choreography reminder for 05-05+:** after this plan's stage-build.sh re-run, `out/init` is the STAGED init again; any plan re-baking the image must re-run subset-build.sh first (05-02's documented "last writer wins" + mkimage.sh's manifest assert names the fix).
- The tracer's Task-1 gate log was superseded by the Task-3 gate run in the same file (run-r9.sh's fresh-log hygiene); the tracer's marker sequence is recorded in Accomplishments and the committed 05-04-boot-r9.log is the final (superset) gate evidence.

## User Setup Required

None - no external service configuration required. (Driving run-r9.sh by hand needs qemu-system-aarch64 + fdtput on PATH — the r9 devshell provides both; `nix develop` from ../r9, then the gate command from this repo root.)

## Known Stubs

- **devcons cons reads return a loud -1 ("cons reads arrive with 05-08")** — deliberate, plan-mandated ("an open for read succeeds but reads return a loud -1 'not yet' errstr rather than wedging"); the open succeeds so fd wiring is real; 05-08 (full devcons) wires the PL011 RX ring through the device (the arch-side `cons_read` drainer is preserved for it).
- **devproc directory reads / status / args do not exist** — outside this plan's scope (the trace's ps surface); 05-08/05-09 grow them (D-09's sweep needs `/proc/*/status`).
- **#e/#s registry entries absent by design** — bindenvsrv's binds are the fail-soft paths they exercise; 05-05 (devenv) and a later devsrv plan fill them.
- **brk_ shrink only lowers the recorded break** (mapped pages stay mapped; the donor mfreeseg's them) — malloc never shrinks this era; documented approximation, revisited if a measured caller shrinks.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: syscall_surface | ../r9/aarch64/src/syscall.rs | sys_brk_ takes a full-width user address (not a path string — outside T-05-06's named components): bounds-checked against the bss base and USTOP before the segment map moves; growth pages are demand-zeroed; no nested paging. Mitigation is inline and complete this era — flagged for the phase audit's visibility. |

(T-05-06 held verbatim: every path string/buffer in the new handlers validates via sargs + validate_user_range before any deref — the eager data+bss change is what makes that discipline sufficient for real binaries. T-05-07: the union/mount semantics are the literal 9k port with host tests pinning them. T-05-SC: zero package installs.)

## Next Phase Readiness

- **05-05** (devenv + seeded root group): the argv hint's removal point is commented in main.rs (`cfg!(dev_bootchain)` arm); the env-driven method pick (nobootprompt=embed) lands there; /rootfs.img hardening is its Task 2.
- **05-06** (devpipe/devdup + the paqfs server exec): the pipe wall is the next frontier — PIPE(21) stays loud-None by design this plan; boot's connectembed will traverse pipe→fork→dup→exec when those land.
- **05-08**: the namespace-file re-mount stanza finding (05-03) applies to the image's scoped /lib/namespace there; cons reads wire the preserved RX-ring drainer through the device.
- The staged-profile supply chain is healthy again (staged init restored); the r9 boot-test fan is green through the namespace machinery.
- r9 HEAD: `1344eba` (main). Harvey HEAD: `2a53066a2` + the metadata commit.

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: r9 port/src/{namec.rs,pgrp.rs,dat.rs,lib.rs,devcons.rs,devproc.rs,devroot.rs}, aarch64/src/{main.rs,exec.rs,syscall.rs,proc.rs}, aarch64/lib/config_virtshell.toml, aarch64/Cargo.toml, xtask/src/main.rs, port/tests/{namespace_test.rs,05-04-red-evidence.json}; harvey build/aarch64/init.c + .planning/phases/05-shell-boot-one-command-integration/05-04-boot-r9.log — spot-checked FOUND on disk and committed.
- Commits: r9 17a5898, 244359e, 68f2692, 0cd1e20, 1344eba; harvey 2a53066a2 — all FOUND in git log.
- Plan-level <verification>: (1) tracer — Task-1 gate PASS on its tree (markers quoted in Accomplishments); (2) core — `cargo test -p port` 9/9 + all suites, builds ×4 + clippy green, grep legs PASS; (3) gate — 05-04-boot-r9.log shows paqfs... → syscall 21 wall, zero panic, zero "can't exec paqfs"; all five boot-test profiles PASS; ureg-check 35/35.
