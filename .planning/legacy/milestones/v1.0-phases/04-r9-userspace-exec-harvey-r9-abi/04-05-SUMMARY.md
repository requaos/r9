---
phase: 04-r9-userspace-exec-harvey-r9-abi
plan: 05
subsystem: kernel/vm
tags: [r9, aarch64, exec, el0-entry, devroot, trap-return, initrd, demand-paging, wr-01-closure]

requires:
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 01)
    provides: initrd delivery (generic loader + fdtput-patched DTB — D-01 AMENDED), mkpaqfs host recipe, D-14 measured syscall set {51 pwrite, 14 open, 50 pread, 4 close, 8 exits}
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 02)
    provides: port::paqfs no_std reader (stateless-per-call, trap-context-safe)
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 03)
    provides: TrapFrame ≡ Harvey Ureg (published contract, const asserts + ureg-check)
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 04)
    provides: proc/exec/fault machinery (consumed unchanged — new_user_proc, exec_into, user_fault, EC split)
provides:
  - build/aarch64/init.c + stage-build.sh → gitignored build/aarch64/out/{init,hello} (R_MAGIC-verified staged binaries)
  - cargo xtask initrd — target/bootfs.paq assembly via mkpaqfs with sha256 evidence print + build-if-missing/stale hook on qemu/boot-test
  - port/src/devroot.rs — the initrd paqfs as the root file tree (D-15); exec sources /init through it
  - trap.S trap_return — the exported shared restore tail + Rust-callable first-EL0 entry (the ONLY way EL0 is entered)
  - main9 dev_exec orchestration: devroot mount → /init exec → D-10 TTBR0 teardown (WR-01 closed) → trap_return into EL0
  - D-12 removal: test_sysexit + the raspi user-table experiment deleted from main.rs
  - first-EL0 serial evidence: init's first svc (51=pwrite, D-14-traced) arriving as the loud-unimplemented endpoint
affects:
  - 04-06 (syscall dispatch replaces the EC-0x15 placeholder; the fd 0/1/2 devcons preopen seam; tick-print gating pattern already landed)
  - 04-07 (scheduler: EL0 IRQs already dispatch correctly; per-proc kstacks + PROC_TABLE in place)
  - 04-08 (notes: the trap_return boundary is the delivery point)

actuals:
  tokens: 13500
  tasks: 2
  commits: 6
  plan_head_before_r9: b14c3e81a0b2478416c8131ae0419909e200e88e
  plan_head_after_r9: 0598716d26bb5a5962dcfa76e10962af8782a74c
  plan_head_before_harvey: eac513e968abf14ac4884bc32636c79e5bd26594

tech-stack:
  added: []
  patterns:
    - "trap entry without a stack switch (the 9front arm64 shape): the frame goes on the current kernel stack — EL1h traps on the interrupted stack, EL0 traps on SP_EL1 (the trapping proc's kstack top). A switch needs a scratch register BEFORE the first store, which is how the old entry poisoned every frame's x0"
    - "EL0 entry only via trap_return with a seeded forkret-shaped frame; swtch never enters EL0"
    - "staged-binary supply chain: stage-build.sh (lima namespace 7c/7l) → build/aarch64/out/ (gitignored, R_MAGIC-verified) → cargo xtask initrd (mkpaqfs) → target/bootfs.paq → loader+patched-DTB → devroot → exec"
    - "evidence-run hygiene: rm the serial log before launching qemu (a stale log short-circuits the marker-wait loop and kills the fresh guest)"

key-files:
  created:
    - build/aarch64/init.c
    - build/aarch64/stage-build.sh
    - ../r9/port/src/devroot.rs
    - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md
  modified:
    - .gitignore (build/aarch64/out/)
    - build/aarch64/README.md (stage-build.sh roster row)
    - ../r9/xtask/src/main.rs (initrd subcommand + ensure_initrd_image hook)
    - ../r9/xtask/src/uregcheck.rs (harvey_dir pub(crate))
    - ../r9/port/src/lib.rs (pub mod devroot)
    - ../r9/aarch64/src/trap.S (trap_return factoring + no-switch unified entry)
    - ../r9/aarch64/src/trap.rs (trap_return binding, sp()/psr() getters, EL0-origin IRQ acceptance)
    - ../r9/aarch64/src/platform/virt/timer.rs (tick print cfg(not(dev_exec))-gated — Pitfall 3)
    - ../r9/aarch64/src/main.rs (exec_bootstrap + D-12 removal + initrd image capture)
    - ../r9/aarch64/src/fault.rs (kernel-context panic prints pc/sp/psr)
    - ../r9/aarch64/lib/config_virtexec.toml (dev=["exec"], initrd=target/bootfs.paq)
    - ../r9/HowItWorks.md (file-header mandate)

key-decisions:
  - "Trap-entry stack-switch removed entirely (Rule 1 bug fix): the old entry clobbered x0 with the interrupt-stack address before the first store (every frame's x0 = interruptstackbase+64KiB — poisoning the SVC number and any EL0-context x0) and never restored the interrupted EL1 sp on return. The no-switch 9front shape fixes both with less asm; the 04-05 gate exposed it as an EL0 permission fault on the interrupt-stack top"
  - "EL0 is entered ONLY through trap_return with the seeded forkret frame (swtch never enters EL0); the restore tail is the single exported path shared by the trap macro and exec's first entry"
  - "config_virtexec.toml's initrd file is 'target/bootfs.paq' (workspace-relative), not the plan's literal '../../target/bootfs.paq' — the plan's own instruction was to match apply_qemu_config's dtb emission base, which is workspace-relative (qemu runs with cwd=workspace; prepare_initrd_dtb prepends the workspace path)"
  - "devroot implements the device OPERATIONS as inherent methods, not the dat.rs Dev trait: the trait skeleton is undispatched and non-constructible as written (Walkqid's unsized _qids; static attach cannot see a mounted image). Plan 9 open-mode values carried locally (OREAD=0/OEXEC=3) — dat.rs's Mode bitflags have wrong values (deferred-items.md #1)"
  - "Timer tick println is cfg(not(dev_exec))-gated NOW (04-06's planned edit pulled forward): with exec configs, main9 prints after unmask and the tick's IRQ-context println can deadlock against the interrupted Console-lock holder (Pitfall 3). The tick keeps re-arming in every config (D-13 machinery stays live); the Phase-3 virt profile keeps its tick markers"

patterns-established:
  - "devroot chain as the one exec byte-source mechanism: mount_initrd → attach_root → walk(component) → open(OEXEC) → entry() (D-15)"
  - "boot evidence chain for the exec profile: banner → initrd → paqfs entries → ctr_el0 → devroot open → exec: entering EL0 → exec: ttbr0 switched, console alive → fault page-in markers → syscall N unimplemented"

requirements-completed: [R9-03]

coverage:
  - id: T1
    description: "staged-init pipeline: build/aarch64/init.c (Plan 9 C: tabs, nil, exits; prints exactly 'init: r9 EL0 alive' then exits(nil)); stage-build.sh compiles it in the lima guest namespace with the committed 7c/7l wiring into gitignored build/aarch64/out/ (idempotent, mirrors subset-build.sh, R_MAGIC-verified, hello.7.out copied verbatim); cargo xtask initrd assembles target/bootfs.paq from out/{init,hello} via the 04-01 mkpaqfs host build and prints its sha256; missing mkpaqfs fails with a named instruction"
    requirement: "R9-03"
    verification:
      - kind: automated
        ref: "nix develop -c sh build/aarch64/stage-build.sh → STAGE-BUILD-OK + PASS line; test -s out/init && test -s out/hello && od first-4-bytes(out/init)=00008c47 && od(out/hello)=00008c47 — PASS; cargo xtask initrd --arch aarch64 --config virtexec → 'initrd: …/target/bootfs.paq sha256 cd609137c927d0c156b2b11583194eaee3bc1b0b0bcded5387732574908fbc4c' (evidence-run bake; per-bake digest — mkpaqfs embeds a time field), test -s target/bootfs.paq PASS; mkpaqfs-precondition negative (binary renamed away) → 'mkpaqfs host binary missing at … — build it in the Harvey checkout with `nix develop -c sh build/aarch64/mkpaqfs-host.sh` (the 04-01 recipe)'; cargo xtask build --config virtexec green; cargo test -p xtask 17/17 PASS; git check-ignore confirms out/ ignored"
        status: pass
    human_judgment: false
  - id: T2
    description: "devroot + trap_return export + main9 exec orchestration + D-12 removal → first-SVC boot proof: banner → initrd discovered → /init opened through devroot → a.out parsed → exec: entering EL0 → D-10 TTBR0 teardown with console-liveness proof (WR-01 closed) → demand-fault page-in markers → init's first SVC (51=pwrite, in the D-14 measured set) arriving as the loud-unimplemented endpoint; zero panic/Unhandled lines; Phase-3 virt/virtpanic profiles + raspi3b/raspi4b builds green"
    requirement: "R9-03"
    verification:
      - kind: integration
        ref: "raw-qemu evidence (D-01-amended delivery: -device loader,file=target/bootfs.paq,addr=0x48000000 + fdtput-patched target/initrd-virt-gicv2.dtb — see Deviation 1): target/boot-logs/virtexec-el0.log (gitignored) carries 'r9 from the Internet' → 'initrd: start=0x48000000 end=0x480094d2 size=38098' → 'paqfs: init'/'paqfs: hello' → 'ctr_el0: 0x8444c004' → 'devroot: /init open (37164 bytes)' → 'exec: entering EL0 pc=0x10044 sp=0x7fffffffffd8' → 'exec: ttbr0 switched, console alive' → 'fault: text page-in va=0x10000' + va=0x11000 + 'fault: data page-in va=0x20000' + va=0x14000 → 'syscall 51 unimplemented'; all 8 plan greps PASS; first svc number 51 found in 04-01-d14-trace.txt ('^51 pwrite'); acceptance greps PASS (devroot mod line, 'fn trap_return' in trap.S, dev_exec + vm::switch in main.rs, zero test_sysexit/init_user_page_tables hits — D-12 gone incl. comments); cargo xtask boot-test --config virt PASS (banner/≥3 ticks/echo — the rewritten entry on the IRQ path) + --config virtpanic PASS (banner/start marker/≥30 register rows/kernel halted) + build default(raspi3b)/raspi4b green; clippy virtexec zero unused/error diagnostics; cargo xtask test 12/12 PASS; ureg-check green inside both boot-tests"
        status: pass
    human_judgment: false

duration: 31min
completed: 2026-11-02
status: complete
---

# Phase 04 Plan 05: First-EL0-Entry Runtime Gate Summary

**SC#3's mechanics are execution-proven end-to-end: the staged Harvey aarch64 `init` rides the initrd supply chain (stage-build.sh → cargo xtask initrd → bootfs.paq → loader+patched-DTB), r9 mounts it as devroot, execs `/init` through the 04-04 machinery, tears down the TTBR0 identity map (WR-01 closed, console alive on TTBR1), erets into EL0 via the new shared `trap_return` tail, and demand-faults its way (text×3 + data, CTR_EL0-derived icache flushes) to init's first syscall — `syscall 51 unimplemented`, the D-14-traced pwrite, the deliberate loud endpoint. The gate exposed and fixed a real latent trap-entry bug (the stack switch poisoned every frame's x0); the D-12 experiments are gone.**

## Performance

- **Duration:** 31min
- **Started:** 2026-11-02T13:27:32Z
- **Completed:** 2026-11-02
- **Tasks:** 2/2
- **Commits:** 3 r9 + 3 Harvey (below)

## Accomplishments

- **Task 1 — the staged-init supply chain exists end-to-end.** `build/aarch64/init.c` (the D-04 staged minimal init) compiles via `build/aarch64/stage-build.sh` in the lima guest's Plan 9 namespace (committed 7c/7l thestring=aarch64 wiring, idempotent libc `mk install`, mirrors subset-build.sh's vm-run.sh flow) into the gitignored `build/aarch64/out/`, joined by the committed `hello.7.out` verbatim; every staged binary is R_MAGIC (`00 00 8c 47` BE) verified before the rsync-back is trusted. r9's `cargo xtask initrd` assembles `target/bootfs.paq` from `out/{init,hello}` via the 04-01 mkpaqfs host build (staging tree at `target/initrd-tree/`, sha256 printed per bake; a missing mkpaqfs or missing staged binary fails with the producing command named). `qemu`/`boot-test` invocations whose config initrd lives under `target/` get a build-if-missing-or-stale call — never a silent boot without userland.
- **Task 2 — the first EL0 entry, proven on serial.** `port/src/devroot.rs` serves the initrd paqfs as the root tree (mount_initrd → attach → single-component walk → open OREAD/OEXEC → read_at-backed read → minimal Plan 9-wire stat); exec sources `/init` through exactly this path (D-15, one mechanism). `trap.S`'s restore tail is factored into the exported `trap_return` label — the macro branches to it, and exec's first entry rides it via `trap_return_exec` (x0 = seeded frame → sp → branch in): the ONLY way EL0 is ever entered. `main9` under `dev_exec` (after the preserved GIC → timer → PL011 RX → unmask arming order) prints the probed `ctr_el0: 0x8444c004`, mounts devroot, opens `/init` (37164 bytes), `new_user_proc` + `exec_into`, prints `exec: entering EL0 pc=0x10044 sp=0x7fffffffffd8`, switches TTBR0 to the proc's root table (D-10 — Pitfall-11 order: fully-built table → switch → immediate `exec: ttbr0 switched, console alive` proof), sets the current proc, and erets into EL0. Demand paging then does the rest: `fault: text page-in va=0x10000`, `va=0x11000`, `fault: data page-in va=0x20000`, `va=0x14000`, … → `syscall 51 unimplemented` — zero panic lines.
- **The gate caught a real latent bug (fixed inline, Rule 1).** The old trap entry switched to the interrupt stack using x0 as scratch BEFORE saving it — every frame's x0 slot held `interruptstackbase+64KiB`, silently poisoning the SVC number and any EL0-context x0 (invisible pre-EL0: the wfi idle loop never reads x0). init died on the evidence run with an EL0 permission fault at exactly that address. The entry no longer switches stacks at all (the 9front arm64 shape): the frame goes on the current kernel stack — EL1h traps on the interrupted stack, EL0 traps on SP_EL1 (the trapping proc's kstack top). This also fixes the latent never-restored interrupted-EL1-sp defect on IRQ return. The Phase-3 virt (ticks+echo) and virtpanic (register dump) profiles both PASS on the rewritten entry.
- **D-12 done:** `test_sysexit` and the raspi user-table-singleton experiment are deleted from main.rs, source and comments alike (acceptance greps count zero); git history preserves them.
- **WR-01 closed:** the TTBR0 identity map is gone from first exec onward; it now exists only for the pre-MMU→MMU transition.

## Task Commits

| Task | Repo | Commit | Files |
|------|------|--------|-------|
| 1 | r9 | `80f8580` | xtask/src/main.rs (initrd subcommand + ensure_initrd_image), xtask/src/uregcheck.rs (harvey_dir pub(crate)) |
| 1 | harvey | `84592640d` | build/aarch64/init.c, build/aarch64/stage-build.sh, .gitignore |
| 2 | r9 | `b8f1dfe` | port/src/devroot.rs (new), port/src/lib.rs, aarch64/src/trap.S, aarch64/src/trap.rs, aarch64/src/platform/virt/timer.rs, aarch64/src/main.rs, aarch64/src/fault.rs, aarch64/lib/config_virtexec.toml |
| 2 | r9 (docs) | `0598716` | HowItWorks.md (file-header mandate) |
| 2 | harvey (docs) | `b8972aad6` | build/aarch64/README.md (stage-build.sh roster row) |
| metadata | harvey | (final commit) | .planning SUMMARY/STATE/ROADMAP/REQUIREMENTS |

**r9 HEAD at evidence time (D-13 ordering — all r9 commits landed first):** `0598716d26bb5a5962dcfa76e10962af8782a74c` (docs-only on top of the code-evidence commit `b8f1dfe`).
**Evidence-run bootfs.paq sha256:** `cd609137c927d0c156b2b11583194eaee3bc1b0b0bcded5387732574908fbc4c` (per-bake digest — mkpaqfs embeds a time field).
**Probed CTR_EL0 (D-09 geometry evidence):** `0x8444c004`.
**Serial evidence log:** r9 `target/boot-logs/virtexec-el0.log` (gitignored `target/`, per phase convention).

## Files Created/Modified

**Harvey:** `build/aarch64/init.c` (new), `build/aarch64/stage-build.sh` (new), `.gitignore` (`build/aarch64/out/`), `build/aarch64/README.md` (roster row), `.planning/…/deferred-items.md` (new).
**r9:** `port/src/devroot.rs` (new), `port/src/lib.rs`, `xtask/src/main.rs`, `xtask/src/uregcheck.rs`, `aarch64/src/trap.S`, `aarch64/src/trap.rs`, `aarch64/src/platform/virt/timer.rs`, `aarch64/src/main.rs`, `aarch64/src/fault.rs`, `aarch64/lib/config_virtexec.toml`, `HowItWorks.md`.

## Decisions Made

See key-decisions in the frontmatter: the no-switch trap entry (the headline Rule-1 fix), trap_return as the only EL0 entry, the workspace-relative initrd path resolution, devroot's inherent-API shape vs the non-constructible Dev-trait skeleton, and the early tick-print gating.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] trap.S entry clobbered x0 (and never restored the interrupted EL1 sp)**
- **Found during:** Task 2 first evidence boot — init died with an EL0 permission fault at `0xffff800040a41000` = exactly `interruptstackbase + INTERRUPTSTACKSZ`, with `pc=0x10638 sp=0x7ffffffffd08 psr=0x60000000` (instrumented per deviation 4)
- **Issue:** the entry macro switched to the interrupt stack using x0 as scratch BEFORE saving it (`ldr x0, =interruptstackbase` → `stp x0, x1, …`), so every frame's x0 slot held the interrupt-stack top — poisoning the SVC number and any EL0-context x0 on restore. Separately, the EL1h return never restored the interrupted sp (invisible pre-EL0: only the wfi loop resumed at EL1)
- **Fix:** removed the stack switch entirely (the 9front arm64 shape): the frame goes on the current kernel stack — EL1h traps on the interrupted stack, EL0 traps on SP_EL1 (the trapping proc's kstack top, left there by trap_return). No scratch register is needed before the first store, every slot stays truthful, and `add sp, #288` on the tail reproduces the interrupted EL1 sp exactly
- **Files modified:** ../r9/aarch64/src/trap.S (entry rewrite; header comment documents the history), HowItWorks.md
- **Verification:** the full evidence chain passes; Phase-3 virt + virtpanic boot-tests PASS on the rewritten entry
- **Commit:** b8f1dfe (r9)

**2. [Rule 1 — Plan written against the falsified pre-amendment initrd mechanism] Task-2 raw-qemu verify leg adapted to D-01-amended delivery**
- **Found during:** Task 2 verify (before first boot)
- **Issue:** the plan's leg-1 command used `-initrd target/bootfs.paq` + the base DTB — qemu 11.1.2 never delivers `-initrd` for ELF kernels (04-01 falsified A3; D-01 AMENDED to loader + fdtput-patched DTB), so the leg as written could only produce `initrd: none` → a loud dev_exec panic
- **Fix:** same command shape and the plan's grep chain byte-unchanged, but delivery via `-device loader,file=target/bootfs.paq,addr=0x48000000` + `-dtb target/initrd-virt-gicv2.dtb` produced by the same fdtput two-cell encoding as xtask's prepare_initrd_dtb. Recorded in WINDOWS.md as entry #8, marked fixed (04-06+ inherit the amended mechanism)
- **Files modified:** none (verification invocation only)
- **Commit:** — (evidence in the coverage block)

**3. [Rule 1 — Acceptance-letter] ac2's awk anchor `/pub fn main9/` never matches**
- **Found during:** Task 2 acceptance greps
- **Issue:** main9's signature is `pub extern "C" fn main9` (pre-existing since Phase 3) — the plan's anchor matches nothing, so the criterion could never pass
- **Fix:** ran the intent-equivalent check with the corrected anchor (`/fn main9/`): main9's body onward contains `dev_exec` ✓ and `vm::switch` ✓
- **Files modified:** none

**4. [Rule 2 — Missing critical] T-04-09 kernel-context panic now prints pc/sp/psr**
- **Found during:** Task 2 debugging (the deviation-1 bug hunt needed the faulting instruction's pc)
- **Issue:** fault.rs's loudest kernel-bug alarm named the class and address but not the faulting pc/sp/psr — the minimum triage context
- **Fix:** added `TrapFrame::sp()`/`psr()` getters (04-04's `pc()` precedent — the conformance-preserving minimal surface; fields stay private) and print all three in the panic
- **Files modified:** ../r9/aarch64/src/fault.rs, ../r9/aarch64/src/trap.rs
- **Commit:** b8f1dfe (r9)

**5. [Rule 2 — Missing critical] dispatch_irq accepts EL0-origin IRQs; tick print gated under dev_exec**
- **Found during:** Task 2 pre-boot review (the gate would have been nondeterministic without both)
- **Issue:** (a) dispatch_irq filtered IRQs to EL1-origin types only — a timer tick landing while init runs at EL0 would have fallen into the "user trap: unhandled" kill path instead of returning to EL0; (b) the tick's IRQ-context `println!` can deadlock against the interrupted Console-lock holder once main9 prints post-unmask (Pitfall 3 — 04-06's planned gating mechanism, needed by the first exec config NOW)
- **Fix:** (a) dispatch_irq also accepts type 9 (IRQ from EL0-64) — inert elsewhere (with no EL0 in existence the vector never fires); (b) tick's println is `cfg(not(dev_exec))`-gated (the tick keeps re-arming in every config; the Phase-3 virt profile keeps its tick markers and PASSed)
- **Files modified:** ../r9/aarch64/src/trap.rs, ../r9/aarch64/src/platform/virt/timer.rs
- **Commit:** b8f1dfe (r9)

**Plan-supporting additions (in-plan intent):** `config_virtexec.toml` carries `initrd = { file = "target/bootfs.paq", … }` (workspace-relative) instead of the plan's literal `../../target/bootfs.paq` — the plan's own instruction was to confirm the emission's base dir against apply_qemu_config's dtb handling and use the same one; the dtb handling is workspace-relative (qemu's cwd is the workspace; prepare_initrd_dtb prepends the workspace path), so `../../target/…` would have escaped the workspace and never been found. `trap.rs` gained the `trap_return` extern binding (the plan names trap.S only, but the asm symbol needs a Rust-side declaration — trap.rs is where trap.S's bindings live). `uregcheck::harvey_dir` went `pub(crate)` so the initrd step resolves the Harvey checkout the same D-03/D-06 way (no second resolver).

**Total deviations:** 5 auto-fixed + 3 plan-supporting additions. No Rule 4 (architectural) items.
**Impact on plan:** none to intent — all must-have truths, acceptance criteria (with the anchor correction documented), and plan-level verification items pass at the final state.

## Known Stubs

Plan-mandated seams with named owning plans (none prevent this plan's goal — the runtime gate passed):

- **EC 0x15 loud-unimplemented endpoint** (../r9/aarch64/src/trap.rs) — prints the wire number and parks; real dispatch is 04-06 per the plan (this IS the plan's deliberate endpoint, recorded here only for the register's visibility).
- **fd 0/1/2 = None at exec** (../r9/aarch64/src/proc.rs, 04-04's seam) — 04-06's devcons device supplies the Chan values (D-15 preopen).
- **note_queue/notify_handler/wait_record/context fields** carried unfilled — 04-07/04-08 own them, per plan.
- **dat.rs `Dev` trait / `Walkqid` / `Mode` skeleton** — devroot implements the device operations as inherent methods instead; trait conformance lands with the devtab dispatch era (04-06+). The `Mode` value drift is logged in `deferred-items.md` #1.
- (Ledger note: the WINDOWS.md append for the deviation-of-record succeeded as entry #8 and is marked fixed; cross-repo (`../r9/…`) paths remain unrecordable there, so the seams above are recorded here, matching 04-04's note.)

## Issues Encountered

- The stale-serial-log race: rerun scripts must `rm -f` the log before launching qemu, else the marker-wait loop matches the old log and kills the fresh guest before it opens the file. Cost two debugging cycles; logged in `deferred-items.md` #2.
- One executor slip (`struct UregCheckStep {}` clobbered during an xtask edit) was caught by compile immediately and restored in the same commit.
- One WINDOWS.md probe entry appended while checking the CLI output shape — waived in place (entry #9); the real deviation entry is #8 (fixed).

## Threat Flags

None. Both register mitigations landed as designed: T-04-10 (Pitfall-11 teardown order — fully-built table → `vm::switch(User)` → immediate console-liveness print; the proof line is in the evidence) and T-04-SC (no package installs this plan — mkpaqfs is the 04-01-audited host build; the sha256 evidence print shells out to coreutils/shasum precisely to avoid a new crate). The D-10 switch changes which translations exist mid-boot, but the mmio-is-TTBR1 proof is printed at every exec-profile boot by construction.

## User Setup Required

None.

## Next Phase Readiness

- **04-06** starts with a running binary whose PWRITE/EXITS path is the only missing piece: replace the EC-0x15 placeholder with the measured dispatch table ({pwrite(51), open(14), pread(50), close(4), exits(8)} ∪ the D-14 named core); the fd 0/1/2 devcons preopen seam is documented in `new_user_proc`; the tick-print gating pattern already landed (extend as the smoke profiles arrive); the exec-era profiles assert markers only, never tick lines (OQ-4 resolution).
- **04-07** gets PROC_TABLE iteration + per-proc kstacks + EL0-origin IRQ dispatch already returning correctly to userland; `swtch` Context slots are in place.
- **04-08** delivers notes at the `trap_return` boundary.
- The initrd iteration loop is now `sh build/aarch64/stage-build.sh && (cd ../r9 && nix develop -c cargo xtask initrd)` — userland changes never relink the kernel (D-01/D-03).
- r9 HEAD for downstream pinning (D-13): `0598716d26bb5a5962dcfa76e10962af8782a74c`.
- Flagged assumption carried from the plan: R9-03 remains probe-unclassified — acceptance derived from ROADMAP SC#3 + CONTEXT D-04/D-09/D-10/D-11/D-12/D-15; the verifier should treat the probe row as a flagged assumption, not a resolved edge.

## Self-Check: PASSED

- Key files exist: Harvey `build/aarch64/init.c` ✓ `stage-build.sh` ✓ `.gitignore` entry ✓ `README.md` row ✓ `deferred-items.md` ✓; r9 `port/src/devroot.rs` ✓ `port/src/lib.rs` mod line ✓ `trap.S` trap_return ✓ `config_virtexec.toml` dev=["exec"] ✓ — all verified on disk at write time.
- Commits: r9 `git log --grep="04-05"` → 80f8580 (Task 1), b8f1dfe (Task 2), 0598716 (docs) — ≥1 `04-05` commit ✓; Harvey → 84592640d (Task 1), b8972aad6 (docs) + the metadata commit carrying this file ✓.
- Acceptance criteria re-run at close-out at the final HEADs: Task-1 greps (R_MAGIC bytes on out/init + out/hello, mkpaqfs precondition named-instruction path) PASS; Task-2 greps (`pub mod devroot;` ✓, `fn trap_return` in trap.S ✓, dev_exec + vm::switch in main.rs ✓ anchor-corrected, zero test_sysexit/init_user_page_tables hits ✓) PASS.
- Plan-level verification at final r9 HEAD: the full leg-1 evidence chain re-run fresh (all 8 greps PASS, zero panic/Unhandled lines) + first-svc 51 ∈ D-14 trace ✓ + boot-test virt PASS + virtpanic PASS + raspi3b/raspi4b builds green + clippy clean + xtask 17/17 and port 12/12 host tests + ureg-check green inside both boot-tests ✓.

---
*Phase: 04-r9-userspace-exec-harvey-r9-abi*
*Completed: 2026-11-02*
