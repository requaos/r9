---
phase: 04-r9-userspace-exec-harvey-r9-abi
verified: 2026-11-02T19:44:43Z
status: passed
score: 16/17 must-haves verified
covered_files:
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-01-PLAN.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-02-PLAN.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-03-PLAN.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-04-PLAN.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-05-PLAN.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-06-PLAN.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-07-PLAN.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-08-PLAN.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-01-SUMMARY.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-02-SUMMARY.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-03-SUMMARY.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-04-SUMMARY.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-05-SUMMARY.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-06-SUMMARY.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-07-SUMMARY.md
  - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-08-SUMMARY.md
  - aarch64/ABI.md
  - build/aarch64/init.c
  - build/aarch64/smokefork.c
  - build/aarch64/smokenote.c
  - build/aarch64/stage-build.sh
  - build/aarch64/mkpaqfs-host.sh
  - build/aarch64/tracehello.c
  - build/aarch64/trace-syscalls.py
  - build/aarch64/README.md
  - .gitignore
covered_digest: "v4:sha256:b21e65061df579261fd6ae2524e08320b3779409f2de407abdb67df5e53e5212"
# Cross-repo note (#4155 limitation): the r9-side implementation files escape the
# project root and cannot be fingerprinted by the tool. r9-side coverage is pinned
# by the D-13 mechanism instead: 04-EVIDENCE.md records r9 HEAD
# b4f858abe015a66f403290580970c94a5e57a835, verified equal to the live checkout
# (`git -C ../r9 rev-parse HEAD`) at verification time.
behavior_unverified: 1
behavior_unverified_items:
  - truth: "Stack accesses below the seed page fault and grow the stack segment within the 16 MiB bound (04-04 must-have; ROADMAP SC#3's 'growing the user stack')"
    test: "Stage a binary that descends >4 KiB below the seeded stack page (deep recursion or a large stack array) under a boot-test profile; assert 'fault: stack growth va=0x' markers appear. Negative leg: fault below USTACKTOP-16MiB and confirm the loud unresolvable-kill path, not growth."
    expected: "Stack pages map on demand as zeroed pages inside [USTACKTOP-16MiB, USTACKTOP); faults below the bound kill the process loudly with no kernel panic."
    why_human: "No evidence log shows a 'fault: stack growth' marker — the shipped binaries' stacks fit the 4 KiB seed page, so the growth trigger never fired. Presence checks prove the machinery (Segments::classify → page_in_zeroed; the bound IS the stack segment range) and the shared zero-page map path is runtime-proven via the 'fault: bss page-in' markers, but the stack-specific classification→growth transition and the bound rejection are unexercised."
coincidental_reliance_items:
  - truth: "SC#4 notes smoke: the parent's postnote lands only after the child's notify() is installed"
    reason: incidental-ordering
    harden: "init.c orders handler-install-before-postnote with a 50M-iteration CPU spin (D-16 era has no rendezvous primitive; RFREND is validated by sys_rfork but unwired as a rendezvous). A violation fails LOUDLY (handlerless-note kill or 'note-timeout' → gate failure), so no silent pass is possible — but a Phase-5 rendezvous primitive would replace the timing assumption with an enforced order."
human_verification:
  - test: "Review the committed 04-06-sc2-boot.log (planner-deferred UAT, 04-06 Task 2): confirm the serial log shows banner → initrd discovery → exec entry → 'init: r9 EL0 alive' → 'hello aarch64 harvey' (printed by the committed Phase-2 binary via the real EL0 svc path) → clean all-exited, with zero panic lines."
    expected: "The ordered marker chain is present; the log is consistent with a real EL0 execution of hello.7.out (sha256 1c89e099…)."
    why_human: "Planner-deferred end-of-phase UAT. Verifier note: this gate was RE-RUN live at the pinned r9 HEAD during verification (`cargo xtask boot-test --config virtexec` → PASS, 64940 bytes captured), so the human review confirms the committed artifact, not a stale one."
  - test: "Review the committed 04-08-sc4-boot.log (planner-deferred UAT, 04-08 Task 2): confirm the D-16 narrative ran end-to-end from EL0 — rfork'd children, note posted through /proc/N/note, delivery at the child's return boundary, noted(NCONT) resume, await records — under visible preemption (7 'sched: switch' lines), zero panic/note-timeout/unimplemented-syscall lines. Confirm the smoke suite satisfies 'rfork, notes, basic fd ops from EL0 without kernel panic' before Phase 5 planning."
    expected: "The full marker set is present (smokefork fd ok / child running / await, smokenote got note / resumed, init reaped ×3, smoke complete, all-exited); negative scans are empty."
    why_human: "Planner-deferred end-of-phase UAT. Verifier note: this gate was RE-RUN live at the pinned r9 HEAD during verification (`cargo xtask boot-test --config virtnotes` → PASS, 64940 bytes — byte-count identical to the committed log), so the human review confirms a reproducible artifact."
  - test: "Stack-growth exercise (behavior-unverified truth #9): decide whether the present-and-wired stack-growth machinery (runtime-proven shared zero-page path via bss page-ins) is sufficient for Phase 4 closure, or require an exercised demonstration (a staged binary that faults below the seed page) before Phase 5 relies on deep-stack binaries."
    expected: "Human judgment: accept the machinery as proven-by-shared-path, or schedule a stack-growth smoke."
    why_human: "The growth trigger never fired in any evidence run — the shipped binaries fit the seed page. A kernel-side unit test cannot exercise the trap-context fault path on the host."
---

# Phase 4: r9 Userspace-Exec + Harvey↔r9 ABI Verification Report

**Phase Goal:** r9 can load and run Harvey aarch64 userland binaries — the cross-repo syscall ABI is designed, implemented on both sides, and mismatch is a build failure, not a runtime mystery. Work spans BOTH repos: kernel side in `/Users/neilskinner/code/r9`, user side anchored to `/Users/neilskinner/code/harvey`'s Phase-2 imports.
**Verified:** 2026-11-02T19:44:43Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Verification Method Note

This phase's claims are **boot-gated**, so presence checks were not treated as sufficient: in addition to code/artifact inspection in both repos, the verifier **re-ran the phase's own gates live at the pinned r9 HEAD** (`b4f858abe015a66f403290580970c94a5e57a835` — equal to 04-EVIDENCE.md's recorded SHA, D-13 satisfied):

| Gate (all live at HEAD) | Command | Result |
|---|---|---|
| SC#1 layer 2 | `cargo xtask ureg-check` (standalone, live Harvey tree) | PASS — "35 fields match aarch64/include/ureg.h" |
| SC#1 negative control | drifted ureg.h (`psr`→`psrX`) via R9_HARVEY_DIR | PASS — non-zero exit, named-field diff "field 33: expected psr, found psrX" |
| paqfs host tests | `cargo test -p port` | PASS 26/26 (incl. 5 fixture tests against the committed mkpaqfs image) |
| xtask host tests | `cargo test -p xtask` | PASS 19/19 (incl. 5 uregcheck tests) |
| raspi3b build | `cargo xtask build --arch aarch64` | green |
| raspi4b build | `cargo xtask build --arch aarch64 --config raspi4b` | green |
| virt profile | `cargo xtask boot-test --config virt` | PASS (banner, ≥3 ticks, echo) |
| virtpanic profile | `cargo xtask boot-test --config virtpanic` | PASS (banner, start, ≥30 register rows, clean halt) |
| **virtexec (SC#2)** | `cargo xtask boot-test --config virtexec` | PASS (exec entry → init → hello → all-exited) |
| virtmulti | `cargo xtask boot-test --config virtmulti` | PASS (smoke markers + ≥1 sched switch) |
| **virtnotes (SC#4)** | `cargo xtask boot-test --config virtnotes` | PASS (full D-16 marker set + preemption + all-exited; 64940 bytes, byte-count identical to the committed 04-08-sc4-boot.log) |

Every boot-test run also exercised the ureg-check precondition in-process (observed "ureg-check: 35 fields match…" before qemu spawn) — SC#1's layer 2 is live on every profile, as designed.

## Goal Achievement

### Observable Truths

| # | Truth (source) | Status | Evidence |
|---|---|---|---|
| 1 | **SC#1a:** The Harvey↔r9 syscall ABI contract is written down in one canonical place | ✓ VERIFIED | `aarch64/ABI.md` carries the frozen wire contract (number R0, args SP+8, SVC, result R0; stub shape; syscall table 0–54; D-17 exclusions) plus the appended "Kernel-side enforcement (Phase 4 appendix)". Git: appendix commit 90fa37493 is **append-only (62 insertions, 0 deletions)** on the frozen Phase-2 text; `aarch64/include/ureg.h` untouched since Phase-2 commit a7945d8b5. r9's HowItWorks.md points at the appendix as canonical. |
| 2 | **SC#1b:** r9's Rust trap frame matches ureg.h field-for-field; mismatch is a build failure | ✓ VERIFIED | `r9:aarch64/src/trap.rs` TrapFrame = 35×u64 (x0@0 … x28@224, x29@232, link@240, sp@248, pc@256, psr@264, type@272), `align(16)` stride 288. **Layer 1:** 8 always-on const asserts (offset_of! per pinned offset + size_of == 288) — fail `cargo build`. **Layer 2:** `cargo xtask ureg-check` parses the sibling `aarch64/include/ureg.h` (R9_HARVEY_DIR, default ../harvey); passes live, and the drifted-header negative control fails with a named-field diff. Wired as a BootTestStep precondition (xtask/src/main.rs:983) — observed running ahead of qemu in the live virtnotes boot. |
| 3 | **04-03:** one unified trap.S frame serves all 16 vectors — per-origin SP capture, conditional SP_EL0 restore on `psr & 0xF == 0`, ESR/FAR read live (never frame fields) | ✓ VERIFIED | `trap.S`: single `handle_interrupt type, from_el1` macro; EL0 stubs `mrs x30, sp_el0` post-switch (line 109), EL1 stubs capture live entry-sp pre-switch (line 106); restore tail `tst x10, #0xf` → conditional `msr sp_el0` (174-177), unconditional ELR/SPSR restore (180-181). No ESR/FAR stores in the frame; trap() reads them live and passes them as dispatcher args (trap.rs:394+). Behavior proof: virtpanic profile (EL1h sync path, ≥30 `x<N>:` rows) and virt profile (IRQ path) both PASS live at HEAD — the reshape is behavior-preserving for every pre-existing trap class. |
| 4 | **SC#2:** Phase 2's hello-world executes on r9 under qemu virt and prints via the real syscall path (EL0 entry/return + svc dispatch) | ✓ VERIFIED | Committed `04-06-sc2-boot.log` (git-tracked): `exec: entering EL0 pc=0x10068 sp=0x7fffffffffd8` → `init: r9 EL0 alive` → `hello aarch64 harvey` → `all user processes exited`, zero panic/Unhandled lines. Executing binary is byte-identical to the pinned Phase-2 bisect binary: `shasum -a 256 build/aarch64/hello.7.out` = `1c89e099…` (matches 04-EVIDENCE.md). Evidence honesty: the marker string is in the binary's own rodata (`strings hello.7.out` → "hello aarch64 harvey"), and no r9 kernel source prints it (grep clean). **Reproduced live:** virtexec boot-test PASS at HEAD. |
| 5 | **04-06:** SVC EC 0x15 dispatch — number from R0, args read at user SP+8 with per-access validation, result written to R0; gaps loud-unimplemented, never wedged | ✓ VERIFIED | `syscall.rs`: `dispatch()` reads `frame.x0`, indexes `static DISPATCH: [Option<Handler>; 55]`, None → `println!("syscall {n} unimplemented")` + errstr + -1 (D-14). `sargs()` reads `(frame.sp + 8)` slots after `proc.validate_user_range` on the arg block; every user pointer (buffers, path strings, argv array + each pointed-to string) is validated before dereference (T-04-11). Result → `frame.x0` (the trap.S `ldp x0` pop propagates to user R0); NOTED's special no-writeback return path present (donor dosyscall special case). |
| 6 | **04-06:** the implemented set is exactly the measured trace ∪ D-14's named core — nothing else | ✓ VERIFIED | DISPATCH entries: CLOSE(4), EXEC(7), EXITS(8), OPEN(14), RFORK(19), NOTIFY(28), NOTED(29), ERRSTR(41), AWAIT(47), PREAD(50), PWRITE(51) — numbers cross-checked against Harvey's frozen `sys/src/libc/9syscall/sys.h` (all 11 match). The measured trace (`04-01-d14-trace.txt`: pwrite/open/pread/close/exits) is a subset; ERRSTR(41) was promoted from a measured caller (libc `%r`, observed in the first virtmulti run — documented in 04-EVIDENCE.md); no brk_(24)/segbrk(12) (the trace settles the watch item — both stay None). Zero `syscall N unimplemented` lines in the SC#4 log — no measured-set gap. |
| 7 | **SC#3a:** r9's first-process path loads a Harvey aarch64 init from an embedded root-fs or equivalent delivery | ✓ VERIFIED | D-01-as-amended delivery (loader + fdtput-patched DTB, per the 04-01 user checkpoint): `xtask initrd` bakes `target/bootfs.paq` from staged `out/{init,hello,smokefork,smokenote}` via Harvey's in-tree mkpaqfs host build; qemu generic loader places it at 0x48000000; `/chosen linux,initrd-start/end` patched into a target/ DTB copy. Kernel side: `main.rs` /chosen discovery → RAM-range validation (T-04-02, `initrd: invalid range` panic) → PhysRange reservation before pagealloc (T-04-01) → serial marker `initrd: start=0x48000000 end=… size=77301` (SC#4 log line 14; size matches the live target/bootfs.paq exactly). `devroot` serves the paqfs as root (attach/walk/open OREAD+OEXEC/read/stat); exec sources /init through it — one mechanism (D-15). |
| 8 | **SC#3b:** user text is faulted in on demand, with icache maintenance after text load | ✓ VERIFIED | Nothing mapped at exec but the seed stack page + zeroed Tos page (exec.rs `install_image` → `seed_user_pages`; D-09/D-11 prohibition honored — proven behaviorally: eager loading would produce zero page-in faults, but the logs show them). SC#2 log: `fault: text page-in va=0x10000/0x11000/0x14000`, `fault: data page-in va=0x20000`; SC#4 log: 15 text + 4 data + 5 bss page-ins. `flush_icache_user_range` runs after every text page-in (fault.rs:162) with the verbatim D-09 sequence — CTR_EL0 probed (`mrs ctr_el0`; boot evidence `ctr_el0: 0x8444c004`, never hardcoded), per-line `dc cvau` → `dsb ish` → per-line `ic ivau` → `dsb ish` → `isb`. Trap EC split routes EC 0x20/0x24 (lower-EL aborts) into `fault::user_fault` (trap.rs:433-439). |
| 9 | **SC#3c / 04-04:** stack accesses below the seed page fault and grow the stack segment within the 16 MiB bound; bss/stack/tos faults map zero pages | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Present + wired: `Segments::classify` (proc.rs:97) routes stack VAs to `page_in_zeroed` (fault.rs:196) with marker `fault: stack growth va=…`; the stack segment range `[USTACKTOP-16MiB, USTACKTOP)` IS the bound (out-of-bound classifies None → loud unresolvable kill). The shared zero-page machinery IS runtime-proven — 6 `fault: bss page-in va=0x21000` markers across the evidence logs exercise the same function. **But** no `fault: stack growth` marker appears in any evidence log — the shipped binaries' stacks fit the 4 KiB seed page (note delivery's `ensure_stack_resident` never needed a new page either). The growth trigger and the bound rejection are unexercised → human verification item 3. |
| 10 | **04-05 / D-10:** the TTBR0 identity map is replaced by the first process's user root table at first exec; console print proves MMIO is TTBR1-side (closes Phase-3 WR-01) | ✓ VERIFIED | `main.rs` `exec_bootstrap`: fully-built per-proc root table (recursive slot-511 self-entry) → `vm::switch(&*proc.root_table, RootPageTableType::User)` → immediate `exec: ttbr0 switched, console alive` print (Pitfall-11 order). Marker present in both evidence logs (SC#2 line 456, SC#4 line 467) followed by live console output — a dead console would be an instant visible failure (T-04-10). |
| 11 | **SC#4a:** rfork + basic fd ops run from EL0 without kernel panic, under preemption | ✓ VERIFIED | Committed `04-08-sc4-boot.log`: `smokefork: fd ok` (OPEN/PREAD/CLOSE of /hello's R_MAGIC header through devroot from EL0), `smokefork: child running`, `smokefork: await: 5 0 0 0 "kid"`, 7 `sched: switch` lines, zero panic/Unhandled lines. RFORK validates donor-invalid flag combos, eagerly copies pages via KZERO aliases into the child's own root table (no COW/shared writable pages), dup's the fd table, child frame x0=0 (donor forkchild). AWAIT blocks via `proc::sched()` (swtch) and returns the Plan-9-shaped record; EXITS posts records and wakes. **Reproduced live:** virtmulti PASS at HEAD. |
| 12 | **SC#4b:** notes run from EL0 without kernel panic — libc postnote unmodified, delivery at the return-to-EL0 boundary, noted(NCONT) resumes | ✓ VERIFIED | SC#4 log: `smokenote: got note smoke` → `smokenote: resumed` → `init: reaped 4 0 0 0 "note-ok"`; zero `note-timeout`, zero unimplemented-syscall lines. `devproc-lite` write appends to the addressed pid's queue; `deliver_notes` (trap.rs:332) runs at BOTH return-to-EL0 boundaries (post-dispatch line 428, post-IRQ line 409), pushes the 280-byte saved Ureg (frozen sizeof, NOT the 288 stride) + note bytes on the user stack with SP+8/SP+16 arg homes after writability validation (T-04-16); NOTED NCONT/NRSTR restore, NDFLT exit, NSAVE loud-unimplemented, no-writeback special return. Harvey libc untouched since Phase 2 (git log empty for sys/src/libc/) — postnote ran unmodified. **Reproduced live:** virtnotes PASS at HEAD. |
| 13 | **04-07:** every trap entry lands on the CURRENT process's kernel stack (CURKSTACK); the timer tick preempts — IRQ-from-EL0 returns to a DIFFERENT process | ✓ VERIFIED | `trap.S` records the built frame into CURFRAME at entry (line 124-128); the `trap_return` tail reloads the frame base from CURFRAME (168) and loads SP_EL1 from CURKSTACK on EL0t-bound returns (207). `proc::preempt` from tick → `reschedule()` swaps CURKSTACK + CURFRAME + TTBR0 under the PROC_TABLE lock with a `sched: switch {old}->{new}` marker (dev_exec only — marker-not-order, D-13). Evidence: 7 switch lines in the SC#4 log; the note was delivered on exactly such a boundary. Live: virtmulti + virtnotes PASS at HEAD. |
| 14 | **04-08 / D-16:** devproc-lite exposes ONLY /proc/N/note (walk/open/write) — no ctl/status/notepg files | ✓ VERIFIED | `port/src/devproc.rs`: walk accepts a numeric pid component then the literal "note"; every other name errors; open is write-only; write resolves only the addressed pid against PROC_TABLE (no broadcast/notepg). Scope guard clean: `grep -vE '^\s*//' devproc.rs | grep -c 'ctl\|status\|notepg'` → 0. |
| 15 | **04-01 / D-14:** the donor-kernel trace artifact names the hello-path syscall set with zero guessed entries | ✓ VERIFIED | `04-01-d14-trace.txt` (git-tracked): header pins date, donor pin 350266104f, target /boot/tracehello, method startsyscall+/proc/N/syscall (primary — fallback not needed); set = {51 pwrite, 14 open, 50 pread, 4 close, 8 exits}. Every name resolves to a `sys.h` #define (verified). No brk_/segbrk — the fmt/pool watch item settled by evidence. |
| 16 | **04-01/04-02:** the paqfs delivery substrate — initrd discovery/validation/reservation + a host-verified no_std reader | ✓ VERIFIED | `fdt.rs` `pub property_value_as_u64` (line 173); main.rs validates start≥0x40000000 / start<end / end≤RAM-top and reserves the PhysRange before pagealloc. Reader: `port/src/paqfs.rs` parses header magic 0x529AB12B / trailer 0x6B46E688 / block magic 0x198A1CBF, NoEnc + DeflateEnc (miniz_oxide raw inflate hard-capped at blocksize, adler32 seed-0 via `Adler32::from_checksum(0)`), bounds-checked throughout, typed Errs on hostile input. Host tests PASS 26/26 incl. the committed fixture (sha256 `27b7482e…` — matches, git-tracked in r9) and corrupt-input negatives. Genuine TDD: `04-02-red-evidence.json` records 5/5 named RED failures on the stub API before GREEN. |
| 17 | **04-08:** phase evidence consolidated in 04-EVIDENCE.md; recorded r9 SHA matches the live r9 HEAD | ✓ VERIFIED | 04-EVIDENCE.md records r9 HEAD `b4f858abe015a66f403290580970c94a5e57a835`; `git -C /Users/neilskinner/code/r9 rev-parse HEAD` returns exactly that SHA (D-13 gate satisfied). The SC#1–4 → evidence mapping table, the measured syscall set, the Wave-0 falsification results, and the regression matrix are all present; the two docs commits on top of the gate tree (`dac3b33` docs, `b4f858a` whitespace-only style) change no compiled code, and the verifier re-ran the full matrix green at that HEAD anyway (see Verification Method Note). |

**Score:** 16/17 truths verified (1 present, behavior-unverified — truth #9 stack growth)

### Deferred Items

None. Nothing failed or was left unmet with a later-phase owner: truth #9 (stack growth) is Phase 4's own SC#3 contract and is routed to human verification, not deferred. (Phase 5's real shell will naturally exercise deep stacks and bss-before-data access orders, but that is not a coverage transfer — the criterion belongs to this phase.)

### Review-Finding Interaction (04-REVIEW.md — 17 open findings)

The verifier independently confirmed the two load-bearing findings against the live code and weighed each against the must-have truths:

- **CR-01 (critical, open): demand pager's shared-page classification.** Confirmed real in code: `exec.rs:142-148` records `text_end` page-rounded but `data_end` byte-exact with `bss = [data_end, bss_end)`; `fault.rs:93-103` classifies by faulting *address* and pages whole pages, so a bss-first touch on a page shared with the data tail maps it zeroed and the file bytes at `[page_va, data_end)` are never loaded. **Interaction with must-haves: none falsified.** The phase's exercised paths (text/data/bss page-ins, all smokes, both gates, live re-runs at HEAD) provably work; CR-01 is an ordering-dependent latent defect the shipped binaries don't trigger (their access order touches the data tail first). It does not negate "binaries load and run" as evidenced — but it is a silent-corruption-class bug in phase-shipped code that Phase 5's real binaries (rc, ls, cat — larger bss/data layouts) may trip. Tracked open in 04-REVIEW-DISPOSITION.md; **recommend Phase 5 planning treat CR-01 as a blocking input, not an advisory.**
- **WR-08 (warning, open): user wild top-half pointer panics the kernel.** Confirmed: `fault.rs:69-76` panics on `va >= KZERO` using the *user's* FAR — an ordinary EL0 wild-pointer bug becomes a kernel halt. Latent (unreachable from the measured binaries), tracked open. Does not touch any evidenced truth (zero panic lines in all gates stands as evidence for the exercised paths).
- **WR-01..WR-07 (warnings, open):** vm mapper error-path restore, rfork error-path lifecycle, RFNOWAIT/AWAIT pivot, NOTED psr sanitize, double-note delivery guard, fault-kill lifecycle bypass, exec_replace post-free failure. Each was spot-checked against the named file:line and found accurately reported. None are exercised by the shipped smokes; all are tracked open in the disposition file.
- **IN-01..IN-08 (info, open):** comment rot and harness robustness items; IN-03's formatting slip (`fault.rs:196` brace jam) confirmed present. No truth interaction.

**Net assessment:** the review findings describe the *unexercised edges* of a phase whose *exercised contract* is fully green. No finding negates a must-have truth; CR-01 and WR-08 are honest latent-defect disclosures that travel with the phase into Phase 5 planning.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `../r9/aarch64/src/trap.rs` | TrapFrame ≡ Ureg + const asserts | ✓ VERIFIED | 35 fields, 8 const asserts, live ESR/FAR, UREG_SIZE=280 for the note path |
| `../r9/aarch64/src/trap.S` | unified frame asm + trap_return + CURKSTACK/CURFRAME | ✓ VERIFIED | per-origin SP capture, conditional SP_EL0 restore, frame reload tail, exported `trap_return`/`trap_return_exec` |
| `../r9/xtask/src/uregcheck.rs` + subcommand | cross-repo ureg.h gate | ✓ VERIFIED | standalone PASS + drift negative control PASS + BootTestStep precondition wired (main.rs:983) |
| `aarch64/ABI.md` | kernel-side appendix (append-only) | ✓ VERIFIED | 62+/0- diff; pinned offsets, SP+8 formula, two-layer check, type-field informational note |
| `../r9/aarch64/src/proc.rs` | Proc scaffolding + scheduler + rfork_clone | ✓ VERIFIED | per-proc kstack/root table/fd table/mapping log; PROC_TABLE mcslock; CURKSTACK/CURFRAME; preempt/reschedule/sched; eager rfork copy |
| `../r9/aarch64/src/exec.rs` | a.out loader + exec_replace | ✓ VERIFIED | magic 0x8C47/HDR_MAGIC 40-byte parse, donor validation set, USTACKTOP/USTOP policy constants, two eager pages only, forkret frame seed, parse-before-free ordering |
| `../r9/aarch64/src/fault.rs` | demand-fault handler + icache | ✓ VERIFIED | DFSC/IFSC class routing, text/data from paqfs window via user VA, CTR_EL0-derived icache sequence, zero-page path (bss runtime-proven), 16 MiB stack bound structural |
| `../r9/aarch64/src/syscall.rs` | dispatch table + handlers | ✓ VERIFIED | [Option<Handler>; 55], 11 implemented entries matching sys.h, SP+8 arg reads, validate_user_range on every user VA, loud-unimplemented default, NOTED special return |
| `../r9/port/src/paqfs.rs` | no_std paqfs reader | ✓ VERIFIED | both encodings, bounds-checked, typed Errs, 26/26 host tests |
| `../r9/port/src/devroot.rs` | initrd root tree device | ✓ VERIFIED | mount_initrd/attach/walk/open/read/stat over the paqfs reader |
| `../r9/port/src/devproc.rs` | /proc/N/note only | ✓ VERIFIED | scope guard clean; pid-resolved note append |
| `../r9/aarch64/src/main.rs` | exec orchestration + D-12 removal | ✓ VERIFIED | `exec_bootstrap` under dev_exec; zero `test_sysexit`/`init_user_page_tables` hits (vm.rs retains the Phase-3 helper, unt called); D-10 teardown with console-alive proof |
| `../r9/aarch64/lib/config_virt{exec,multi,notes}.toml` | three exec-era profiles | ✓ VERIFIED | all present; virtnotes carries dev=["exec"] + initrd={file=target/bootfs.paq, addr=0x48000000} |
| `build/aarch64/{init,smokefork,smokenote}.c` | staged init + smokes | ✓ VERIFIED | substantive Plan 9 C (tabs, nil, exits, %r diagnostics); init rforks 3 children, posts the note via unmodified libc postnote, awaits all |
| `build/aarch64/stage-build.sh` + `mkpaqfs-host.sh` | supply chain | ✓ VERIFIED | lima guest compile with vendored 7c/7l, R_MAGIC verification on every staged binary, gitignored out/; in-tree mkpaqfs via plan9port 9c/9l with $PLAN9/bin LAST |
| `04-01-d14-trace.txt` / `04-06-sc2-boot.log` / `04-08-sc4-boot.log` / `04-EVIDENCE.md` | evidence artifacts | ✓ VERIFIED | all git-tracked; logs carry the full marker chains with clean negative scans; EVIDENCE SHA == live r9 HEAD |
| `build/aarch64/hello.7.out` | committed ABI bisect binary | ✓ VERIFIED | sha256 `1c89e099…` matches the pin; marker string present in rodata |
| `../r9/port/tests/fixtures/probe-tree.paq` | committed paqfs fixture | ✓ VERIFIED | sha256 `27b7482e…` matches the pin; git-tracked in r9 |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| qemu delivery | guest RAM → kernel | generic loader `-device loader,file=…,addr=0x48000000` + fdtput-patched DTB `/chosen` → fdt.rs `property_value_as_u64` → main9 validation + reservation | ✓ WIRED | serial marker `initrd: start=0x48000000 end=0x48012df5 size=77301` in SC#4 log; size matches live target/bootfs.paq |
| initrd paqfs | exec | devroot mount → walk "init" → open OEXEC → parse_aout → seed → vm::switch(User) → trap_return → EL0 | ✓ WIRED | main.rs exec_bootstrap (lines 400-460); log chain `devroot: /init open` → `exec: entering EL0` → `ttbr0 switched` |
| first instruction fetch | demand pager | InstructionAbortLowerEl (EC 0x20) → fault::user_fault → paqfs read_at window → map → write via user VA → icache → eret | ✓ WIRED | trap.rs:433-439 routing; `fault: text page-in va=0x10000` markers in both logs |
| user SVC | dispatch | EC 0x15 → syscall::dispatch(frame) → DISPATCH[n] → frame.x0 result → trap_return `ldp x0` → user R0 | ✓ WIRED | trap.rs:423; smoke markers exist only via this path (kernel-print prohibition clean) |
| parent postnote | child note delivery | libc postnote → OPEN "/proc/N/note" → devproc-lite write → note_queue → deliver_notes at return boundary → frame.pc=handler → EL0 handler → noted(NCONT) → saved-Ureg restore → resume | ✓ WIRED | smokenote markers + `init: reaped 4 … "note-ok"`; delivery observed on an IRQ-return boundary (preemption evidence) |
| timer tick | preemption | INTID 30 IRQ-from-EL0 → CURKSTACK entry → tick → preempt → locked CURKSTACK/CURFRAME/TTBR0 swap → tail returns to the other proc | ✓ WIRED | 7 `sched: switch` lines; IRQ-from-EL0 accepted since 04-05's dispatch_irq fix |
| xtask boot-test | ureg-check | BootTestStep precondition before qemu spawn | ✓ WIRED | main.rs:983 `uregcheck::check_for_boot_test()`; observed "ureg-check: 35 fields match…" in live runs |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| devroot reads | file bytes | paqfs `read_at` over the memory-resident initrd image (host mkpaqfs output) | yes — smokefork's R_MAGIC check reads real bytes from EL0 | ✓ FLOWING |
| console output | user print strings | PWRITE handler → devcons Console lock + PL011 polled TX | yes — markers originate in the EL0 binaries' rodata (rodata grep + kernel-print prohibition clean) | ✓ FLOWING |
| syscall args | sargs slots | user stack at frame.sp+8, validated per access | yes — wrong-base reads would garbage-fail the smokes; they pass | ✓ FLOWING |
| wait records | await buffer | EXITS-posted records → AWAIT formats `{pid} 0 0 0 "{msg}"` into the user buffer | yes — `init: reaped 4 0 0 0 "note-ok"` carries the child's exits() string | ✓ FLOWING |
| paqfs host tests | fixture bytes | `include_bytes!("../tests/fixtures/probe-tree.paq")` (sha256 pinned) | yes — byte-exact reads against probe-tree sources | ✓ FLOWING |

No static fallbacks, hollow props, or mock-sourced values found on any rendered/observed path.

### Behavioral Spot-Checks

All run live at r9 HEAD `b4f858abe015a66f403290580970c94a5e57a835` (the 04-EVIDENCE.md pin — D-13 gate PASS):

| Behavior | Command | Result | Status |
|---|---|---|---|
| SC#1 cross-repo gate | `cargo xtask ureg-check` (R9_HARVEY_DIR=/Users/neilskinner/code/harvey) | "ureg-check: 35 fields match aarch64/include/ureg.h", exit 0 | ✓ PASS |
| SC#1 negative control | drifted ureg.h (psr→psrX) | non-zero exit, "field 33: expected psr, found psrX" | ✓ PASS |
| paqfs reader correctness | `cargo test -p port` | 26/26 ok (12 unit + 9 fdt + 5 paqfs incl. committed-fixture + negatives) | ✓ PASS |
| xtask/uregcheck unit tests | `cargo test -p xtask` | 19/19 ok | ✓ PASS |
| no-regression: raspi3b | `cargo xtask build --arch aarch64` | green | ✓ PASS |
| no-regression: raspi4b | `cargo xtask build --arch aarch64 --config raspi4b` | green | ✓ PASS |
| no-regression: virt profile | `cargo xtask boot-test --config virt` | PASS (banner, ≥3 ticks, echo) | ✓ PASS |
| no-regression: virtpanic profile | `cargo xtask boot-test --config virtpanic` | PASS (≥30 register rows, clean halt) | ✓ PASS |
| **SC#2 gate** | `cargo xtask boot-test --config virtexec` | PASS (exec entry → init → hello → all-exited) | ✓ PASS |
| SC#4 rfork/fd-ops | `cargo xtask boot-test --config virtmulti` | PASS (smoke markers + sched switch) | ✓ PASS |
| **SC#4 gate (full)** | `cargo xtask boot-test --config virtnotes` | PASS (full D-16 set + 64940-byte capture, byte-count identical to the committed log) | ✓ PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes are declared by this phase, and no PLAN references a probe script. The phase's probe-equivalents are the Wave-0 falsification boots (04-01: /chosen discovery + mkpaqfs fixture + D-14 trace) and the five xtask boot-test profiles — all of which were executed live by the verifier and reported in the Behavioral Spot-Checks table above. The committed probe artifacts (`04-01-d14-trace.txt`, `probe-tree.paq` sha256-pinned, both boot logs) were each re-validated against their pins.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| R9-02 | 04-01, 04-04, 04-06, 04-07, 04-08 | r9 process model + svc syscall dispatch (EL0 entry/return, trap frame matching Plan 9 ureg conventions) | ✓ SATISFIED | Dispatch table over the frozen sys.h 0–54 (truths 5, 6); unified trap frame ≡ ureg.h (truths 2, 3); per-proc kstacks + preemptive scheduler + RFORK/AWAIT/NOTIFY/NOTED (truths 11–13); EL0 entry only via trap_return; all exercised live at HEAD (virtexec/virtmulti/virtnotes PASS) |
| R9-03 | 04-01, 04-02, 04-04, 04-05 | first-process path loads a Harvey aarch64 init (embedded root-fs or equivalent), faulting in user pages | ✓ SATISFIED | D-01-amended delivery + /chosen discovery/validation/reservation (truth 7); paqfs reader + devroot (truths 7, 16); demand paging with icache (truth 8); D-10 teardown (truth 10); one sub-element (stack growth, truth 9) present-but-behavior-unverified → human item 3 |
| R9-04 | 04-03 | ABI contract documented + enforced by static checks (cross-repo layout match at build time) | ✓ SATISFIED | ABI.md appendix (append-only) + const asserts + ureg-check with proven negative control + boot-test precondition (truths 1, 2) |

**Orphaned requirements:** none. REQUIREMENTS.md maps exactly {R9-02, R9-03, R9-04} to Phase 4; the union of the eight plans' `requirements:` fields is the same set. INT-01..04 belong to Phase 5.

**Flagged-assumption note (per plan headers):** R9-02/R9-03/R9-04 acceptance was derived from ROADMAP SC#1–4 + CONTEXT D-decisions under the spec-less probe fallback. The verifier treated the probe rows as flagged assumptions and anchored acceptance to the ROADMAP success criteria themselves, as instructed. All four SCs are evidenced (three fully machine-verified live at HEAD; SC#3's stack-growth clause routed to human).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| build/aarch64/stage-build.sh | 51 | `mktemp -d "…/stage-build.XXXXXX"` | ℹ️ Info | false positive — mktemp template, not a debt marker |
| build/aarch64/mkpaqfs-host.sh | 29 | `mktemp -d .build/mkpaqfs.XXXXXX` | ℹ️ Info | false positive — mktemp template, not a debt marker |

No TBD/FIXME/XXX debt markers, no placeholder text, no empty implementations, no hardcoded-empty data flowing to output, no console.log-only handlers in any phase-modified file across both repos. The DISPATCH table's `None` entries are the D-14 deliberate loud-unimplemented design (print + -1), not stubs. The 04-REVIEW.md findings (1 critical, 8 warning, 8 info) are tracked in 04-REVIEW-DISPOSITION.md and analyzed under "Review-Finding Interaction" above — none rise to verification blockers for the exercised contract, but CR-01 is flagged as a Phase-5 planning input.

### Human Verification Required

#### 1. SC#2 gate log review (planner-deferred UAT, 04-06)

**Test:** Review committed `04-06-sc2-boot.log` — banner → initrd discovery → exec entry → `init: r9 EL0 alive` → `hello aarch64 harvey` (printed by the committed Phase-2 binary via the real EL0 svc path) → clean all-exited, zero panic lines. Confirm the recorded r9 SHA chain matches the log's provenance.
**Expected:** The ordered marker chain present and consistent with a real EL0 execution of hello.7.out (sha256 1c89e099…).
**Why human:** Planner-deferred end-of-phase UAT. Verifier strengthening: the gate was re-run live at the pinned HEAD during verification (virtexec PASS), so the human confirms a reproducible artifact rather than a one-shot log.

#### 2. SC#4 gate log review (planner-deferred UAT, 04-08)

**Test:** Review committed `04-08-sc4-boot.log` — the D-16 narrative end-to-end from EL0 (rfork'd children; note posted through /proc/N/note; delivery at the child's return boundary; noted(NCONT) resume; await records) under visible preemption (7 `sched: switch` lines), zero panic/note-timeout/unimplemented-syscall lines. Confirm "rfork, notes, basic fd ops from EL0 without kernel panic" is satisfied before Phase 5 planning.
**Expected:** Full marker set present; negative scans empty.
**Why human:** Planner-deferred end-of-phase UAT. Verifier strengthening: re-run live at the pinned HEAD (virtnotes PASS, 64940-byte capture — byte-count identical to the committed log).

#### 3. Stack-growth path — behavior-unverified truth #9

**Test:** Decide whether the present-and-wired stack-growth machinery suffices for Phase 4 closure, or require an exercised demonstration: stage a binary that descends >4 KiB below the seed page (deep recursion / large stack array) under a boot-test profile and assert `fault: stack growth va=0x` markers; negative leg — fault below USTACKTOP−16 MiB and confirm the loud unresolvable-kill path.
**Expected:** Stack pages map on demand within the bound; out-of-bound faults kill loudly without kernel panic.
**Why human:** No evidence log contains a `fault: stack growth` marker — the shipped binaries' stacks fit the 4 KiB seed page, so the trigger never fired. The machinery is present, wired, and its shared zero-page map path is runtime-proven via the 6 bss page-in markers; the stack-specific classify→grow transition and the bound rejection are unexercised, and the trap-context fault path cannot be host-tested.

### Gaps Summary

No gaps in the goal-blocking sense: no truth FAILED, no artifact missing/stub/orphaned, no key link unwired, all three requirement IDs satisfied with live evidence at the pinned r9 HEAD. The phase's four success criteria are green, three of them machine-reproduced during this verification; SC#3's stack-growth clause is present-and-wired but unexercised and is routed to human judgment (item 3). Status is therefore **human_needed**, not gaps_found.

Honest carry-forwards for Phase 5 planning (tracked elsewhere, not verification gaps): CR-01 (demand-pager shared-page ordering — real in code, latent, silent-corruption class) and WR-01..WR-08 are open in 04-REVIEW-DISPOSITION.md; deferred-items.md carries the exec_bootstrap IRQ-masking heisenbug containment (#4, exposure eliminated by construction, root cause unisolated) and the validate_user_range residency caveat (#3).

---

_Verified: 2026-11-02T19:44:43Z_
_Verifier: the agent (gsd-verifier)_

## Acknowledged Gaps

Acknowledged at UAT close (2026-11-02, user-confirmed proceed) — deferred-items.md, non-blocking:

- **#2 (open)** — Orphaned qemu processes / stale serial-log rerun hazard in the smoke harnesses; hygiene cleanup candidate.
- **#3 (open)** — `validate_user_range` is segment-membership only; cold in-segment syscall pointers abort loudly (never silently). Nested demand-paging from syscall context is Phase-5+ territory.
- **#4 (open)** — virtmulti heisenbug not root-caused; exposure eliminated by construction (exec_bootstrap IRQ-masked). Revisit first if a future phase adds an unmasked-EL1h window with live computation.
