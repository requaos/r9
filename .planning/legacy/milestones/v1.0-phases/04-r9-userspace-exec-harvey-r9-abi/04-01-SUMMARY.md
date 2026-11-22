---
phase: 04-r9-userspace-exec-harvey-r9-abi
plan: 01
subsystem: infra
tags: [r9, aarch64, qemu, initrd, paqfs, syscall-trace]

requires:
  - phase: 02-aarch64-toolchain-libc-9front-donor
    provides: lima donor-kernel harness (run-gate.sh/drive9.py/srv9.py/gdbpoke9.py), vendored 7c/7l compile path, hello.7.out ABI bisect point
  - phase: 03-r9-qemu-virt-platform-bring-up
    provides: r9 virt boot-test harness, config-TOML-as-data convention, ELF+-dtb boot contract, evidence-ordering convention (D-13)
provides:
  - r9 kernel initrd discovery — /chosen linux,initrd-start/end read, RAM-range validation (T-04-02), PhysRange reservation before pagealloc (T-04-01)
  - xtask initrd delivery — [qemu] initrd={file,addr} TOML table → generic-loader -device + fdtput-patched DTB copy (target/, gitignored)
  - build/aarch64/mkpaqfs-host.sh — in-tree mkpaqfs host build recipe (plan9port 9c/9l)
  - ../r9/port/tests/fixtures/probe-tree.paq — committed, byte-verified paqfs fixture (NoEnc + DeflateEnc from one image)
  - 04-01-d14-trace.txt — measured hello-path syscall set {pwrite, open, pread, close, exits}
affects: [04-02 (paqfs reader gate), 04-06/04-07/04-08 (syscall implementation set), 04-03]

actuals:
  tokens: 13051
  tasks: 3
  commits: 5

tech-stack:
  added: [fdtput (dtc, devshell-supplied) as an xtask-invoked host tool, mkpaqfs host binary via plan9port 9c/9l]
  patterns:
    - "initrd delivery: TOML data (file+PA) → generic-loader -device → patched-DTB /chosen handoff (target/ artifact, base DTB untouched)"
    - "qemu-side file staging via CONF bootdir carries text scripts byte-identical (mkrootall strips only file(1)-identified executables)"
    - "donor syscall tracing: startsyscall + /proc/N/syscall loop with the rc write as the while condition"

key-files:
  created:
    - ../r9/aarch64/lib/config_virtexec.toml (probe profile: virt keys + initrd table)
    - ../r9/aarch64/lib/probe-initrd.txt (delivery-probe bytes, 402B)
    - ../r9/port/tests/fixtures/probe-tree.paq + probe-tree/{hello.txt,dir/nested.txt}
    - build/aarch64/mkpaqfs-host.sh
    - build/aarch64/tracehello.c
    - build/aarch64/trace-syscalls.py
    - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-01-d14-trace.txt
  modified:
    - ../r9/port/src/fdt.rs (pub property_value_as_u64)
    - ../r9/aarch64/src/main.rs (main9 /chosen discovery + validation + reservation)
    - ../r9/aarch64/src/pre_mmu/mod.rs (pub mod util for the pre-console panic print)
    - ../r9/xtask/src/config.rs (Initrd struct, loader emission, prepare_initrd_dtb)
    - ../r9/xtask/src/main.rs (prepare_initrd_dtb wired into QemuStep + BootTestStep)

key-decisions:
  - "D-01 AMENDED (user decision A, blocking-human decision checkpoint): qemu 11.1.2 never delivers -initrd for ELF kernels (hw/arm/boot.c: ELF ⇒ is_linux=0 ⇒ initrd never loaded into guest RAM, /chosen fixup never runs) — plan assumption A3 falsified. Delivery is now qemu generic loader (-device loader,file=...,addr=<TOML-data PA>) + xtask-produced fdtput-patched DTB copy carrying /chosen linux,initrd-start/end. Kernel-side /chosen discovery/validation/reservation code ships unchanged; ELF boot preserved per D-05."
  - "fdtput byte encoding: dtc 1.8.1 has NO 8-byte property type (l is a 4-byte size modifier and silently truncates u64) — each u64 is written as two 32-bit cells, high word first (byte-identical to one BE u64, which is what property_value_as_u64 parses). /chosen already exists in the committed virt-gicv2.dtb; fdtput -c errors FDT_ERR_EXISTS on it, so no create flag."
  - "Initrd probe PA 0x48000000: 128 MiB into the 1 GiB guest, clear of the kernel (0x40080000) and DTB (0x40000000) placements."
  - "D-14 measured hello-path syscall set (donor pin 350266104f, primary startsyscall method): 51 pwrite, 14 open (#c/pid), 50 pread, 4 close, 8 exits — fmt/pool pulls NO brk_/segbrk on this path (the D-14 watch item is settled by evidence)."
  - "Plan 9 rc has no break builtin: the trace loop's while CONDITION is the startsyscall write itself (ends on Eprocdied)."

patterns-established:
  - "Host-patched-DTB pattern: base DTB committed, per-profile patched copy derived into target/ at qemu-launch time — never commit the patched blob"
  - "Trace-target shaping: leading pure-compute spin (no syscalls possible) sized by trial so the tracer provably arms before the first syscall"

requirements-completed: [R9-02, R9-03]

coverage:
  - id: D1
    description: "initrd delivery proven end-to-end on the ELF+-dtb boot path: generic-loader bytes land in guest RAM, kernel prints 'initrd: start=0x48000000 end=0x48000192 size=402' parsed from /chosen, range validated against the DTB memory node and reserved in pagealloc's used_ranges; 'initrd: none' backward-compat intact (config_virt.toml); serial log zero invalid-range, zero panic"
    requirement: "R9-03"
    verification:
      - kind: integration
        ref: "cargo test -p xtask (12/12, incl. qemu_initrd_emits_loader_and_patched_dtb); raw qemu -device loader + fdtput-patched DTB serial-marker chain (banner + initrd marker + size=402 + zero invalid-range + zero panic); cargo xtask qemu --config virtexec smoke (shipped prepare_initrd_dtb path) — target/boot-logs/virtexec-probe.log, virtexec-xtask-smoke.log"
        status: pass
    human_judgment: false
  - id: D2
    description: "platform no-regression: Phase-3 virt boot-test profile PASS (banner/ticks/echo), raspi3b + raspi4b builds green"
    requirement: "R9-03"
    verification:
      - kind: integration
        ref: "cargo xtask boot-test --arch aarch64 --config virt; cargo xtask build --arch aarch64; cargo xtask build --arch aarch64 --config raspi4b"
        status: pass
    human_judgment: false
  - id: D3
    description: "host mkpaqfs recipe + committed byte-verified paqfs fixture (header magic 0x529AB12B @0, trailer magic 0x6B46E688 @ EOF-28), baked WITHOUT -u so one fixture exercises NoEnc + DeflateEnc (982B from 3048B source)"
    requirement: "R9-03"
    verification:
      - kind: integration
        ref: "sh build/aarch64/mkpaqfs-host.sh in the nix devshell + od byte assertions (trailer via od -j — see deviation 2) + git ls-files --error-unmatch on fixture + tree"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-14 measured syscall artifact: 04-01-d14-trace.txt carries header (donor pin, target, method) + 'NUMBER name' lines, includes pwrite + exits, every name resolves against the frozen sys.h 0-54 table"
    requirement: "R9-02"
    verification:
      - kind: integration
        ref: "plan's Task-3 verify command verbatim (test -s + pwrite/exits greps + shape grep + per-name sys.h resolution loop) — PASS; serial evidence .build/9front-dist/trace-run.log (gitignored)"
        status: pass
    human_judgment: false

duration: 33min (this continuation; plus a prior executor segment that implemented+proved Task 1's kernel side before the A3 decision checkpoint)
completed: 2026-11-02
status: complete
---

# Phase 04 Plan 01: Wave-0 Falsification Spikes Summary

**All three load-bearing assumptions converted to this-repo evidence: initrd delivery proven on serial via generic-loader+patched-DTB (D-01 amended after A3 falsified), mkpaqfs host producer built in-tree with a committed byte-verified paqfs fixture, and the hello-path syscall set measured on the donor kernel — {pwrite, open, pread, close, exits}, zero guessed entries.**

## Performance
- **Duration:** 33min this continuation (+ prior segment to the decision checkpoint)
- **Started:** 2026-11-02T12:00:27Z (continuation)
- **Completed:** 2026-11-02
- **Tasks:** 3
- **Files modified:** 14 (10 created, 4 modified across both repos)

## Accomplishments
- **D-01 amended + proven.** The checkpoint decision (user: Option A) replaced qemu `-initrd` (falsified on qemu 11.1.2 for ELF kernels) with the generic loader at a TOML-data PA plus an xtask-produced fdtput-patched DTB. The kernel-side discovery (fdt.rs `property_value_as_u64`, main9 /chosen validation against the DTB memory node, PhysRange reservation before pagealloc) shipped unchanged and proved on serial: `initrd: start=0x48000000 end=0x48000192 size=402` + banner, zero invalid-range, zero panics; the no-initrd path (`initrd: none`) stays intact. xtask emits `-dtb target/initrd-virt-gicv2.dtb -device loader,file=...,addr=0x48000000` from `initrd = { file, addr }` TOML data; `prepare_initrd_dtb` runs in both QemuStep and BootTestStep.
- **D-02 producer + fixture.** `build/aarch64/mkpaqfs-host.sh` builds Harvey's in-tree `sys/src/cmd/paqfs/mkpaqfs.c` with plan9port 9c/9l ($PLAN9/bin LAST) into gitignored `.build/bin/mkpaqfs`. Baked `../r9/port/tests/fixtures/probe-tree.paq` (committed, sha256 **27b7482ed68562c891a87ee6a3f4ca31633570b9f61de6d1a902eb61ccd21300**), header magic 0x529AB12B @0, trailer 0x6B46E688 @ EOF-28, baked without `-u` so 04-02's reader meets NoEnc + DeflateEnc in one fixture.
- **D-14 measured set.** `04-01-d14-trace.txt` (committed): **51 pwrite, 14 open, 50 pread, 4 close, 8 exits** — first-use order, deduplicated, every name resolved against Harvey's frozen `sys.h`. Method: **primary** startsyscall + /proc/N/syscall (no donor rebuild; the dosyscall print-patch fallback was not needed). The D-14 watch item is settled: fmt/pool pulls **no** brk_/segbrk on the hello path. The open/pread/close triplet is the exits path resolving its own pid via `#c/pid`.
- **Platform no-regression:** Phase-3 virt boot-test PASS (banner/ticks/echo), raspi3b + raspi4b builds green at the final kernel state.

## Task Commits
1. **Task 1: TRACER initrd delivery (D-01 amended)** — `267dbe81c1acadd919ae96448bede4f4ffc8d5f3` (feat) **[r9 repo]**
2. **Task 2: paqfs fixture** — `04e6dcb9cef22eac68a357bfbc3cec2ace005bb0` (feat) **[r9 repo]**
3. **Task 2: mkpaqfs host recipe** — `498945a1e544cd28336e2592a5e1c8be0bf0b196` (feat)
4. **Task 3: D-14 trace** — `cb1784a7f9e1b0b0b1100fe1d4580fe3d6db54f8` (feat)

**r9 HEAD at evidence time (D-13 ordering — r9 commits landed first):** `04e6dcb9cef22eac68a357bfbc3cec2ace005bb0`

## Files Created/Modified
- `../r9/port/src/fdt.rs` — `pub property_value_as_u64` (8-byte BE property reads)
- `../r9/aarch64/src/main.rs` — /chosen discovery + RAM-range validation + pagealloc reservation + deferred marker print
- `../r9/aarch64/src/pre_mmu/mod.rs` — `pub mod util` (pre-console panic print path)
- `../r9/xtask/src/config.rs` — `Initrd` struct, loader emission, `prepare_initrd_dtb` (fdtput driver)
- `../r9/xtask/src/main.rs` — prepare wired into both qemu steps
- `../r9/aarch64/lib/config_virtexec.toml`, `probe-initrd.txt` — probe profile + bytes
- `../r9/port/tests/fixtures/probe-tree.paq` + `probe-tree/` — committed 04-02 reader fixture
- `build/aarch64/mkpaqfs-host.sh` — in-tree producer recipe
- `build/aarch64/tracehello.c` — spin-first trace target (2e9 volatile iterations ≈ 15s TCG, calibrated by trial)
- `build/aarch64/trace-syscalls.py` — compile/stage/rebake/boot/trace/parse orchestrator
- `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-01-d14-trace.txt` — the measured set

## Decisions Made
See key-decisions in frontmatter: the **D-01 amendment** (loader + patched DTB, DTB-mediated, ELF boot preserved per D-05) is the headline; plus the fdtput two-cell u64 encoding, the 0x48000000 probe PA, and the measured D-14 set.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — plan written against falsified assumption A3] qemu -initrd mechanism replaced**
- **Found during:** Task 1 (prior segment falsified A3; checkpoint resolved: user decision A)
- **Issue:** plan's emission (`-initrd`), acceptance grep `" -initrd"`, and verify leg 2's qemu invocation all targeted the never-delivering flag
- **Fix:** loader + host-patched-DTB rework per decision A; acceptance criterion `grep -q '"-initrd"' config.rs` treated as satisfied by the equivalent loader-emission assertion (`pub initrd: Option<Initrd>`, `-device`/`loader,file=` emission, `qemu_initrd_emits_loader_and_patched_dtb` test); verify leg 2 adapted to the loader device + patched DTB (serial-marker greps unchanged, all PASS)
- **Files modified:** ../r9/xtask/src/{config.rs,main.rs}, ../r9/aarch64/lib/{config_virtexec.toml,probe-initrd.txt}
- **Commit:** 267dbe8 (r9)

**2. [Rule 3 — blocking] Task-2 verify's dd pipe unusable in the devshell**
- **Found during:** Task 2 verify
- **Issue:** the nix devshell prepends Harvey's `.build/hostbin-darwin-aarch64/bin` to PATH, whose Plan 9 host `dd` rejects Unix `if=` syntax (`dd: bad arg`) — the plan's trailer-magic check produced empty output and failed
- **Fix:** substituted `od -An -tx1 -j $((sz-28)) -N4` (same bytes at the same offset; od is nix coreutils, unshadowed) — assertion semantics identical
- **Files modified:** none (verification invocation only)

**3. [Rule 1 — bug] CONF bootdir patch appended unindented rows**
- **Found during:** Task 3 (first trace run: /boot/tracehello "file does not exist")
- **Issue:** CONF section rows must be TAB-indented; the initial awk appended bare lines, silently terminating the bootdir section — mkrootall's embed list never gained the entries
- **Fix:** `add_line` now inserts `\t<path>\t<name>` with an idempotency check on the exact row; guest CONF's junk lines cleaned (guest-scratch only, never committed)
- **Files modified:** build/aarch64/trace-syscalls.py
- **Commit:** cb1784a7f

**4. [Rule 1 — bug] rc `break` does not exist + entry-fragment pc regex**
- **Found during:** Task 3 (second/third runs)
- **Issue:** (a) Plan 9 rc has no `break` builtin — the loop ended via command-error noise; (b) the donor's `%p` prints the pc as bare hex (`14918`), so the `0x`-anchored ENTRY_RE parsed zero fragments
- **Fix:** (a) the while CONDITION is now the startsyscall write itself (clean Eprocdied termination + `__GSD_TRACE_LOOP_DONE__`); (b) ENTRY_RE relaxed to bare hex; parse validated against the captured log, then a clean full end-to-end run produced the artifact
- **Files modified:** build/aarch64/trace-syscalls.py
- **Commit:** cb1784a7f

**5. [Rule 2 — missing critical] trace robustness guards**
- **Found during:** Task 3
- **Fix:** `echo stop > ctl || echo __GSD_TRACE_STOP_FAILED__` guard (a too-short spin surfaces as an explicit empty-set failure, not a hung script); spin calibrated 4e8 → 2e9 iterations by trial in the guest (plan-mandated calibration; measured TCG ≈ 800 MIPS here)
- **Files modified:** build/aarch64/trace-syscalls.py, build/aarch64/tracehello.c
- **Commit:** cb1784a7f

**Total deviations:** 5 auto-fixed (one per the user's checkpoint decision)
**Impact on plan:** all plan truths met; the D-01 amendment is recorded as a key decision and the falsified A3 is replaced by serial evidence of the amended mechanism.

## Issues Encountered
- A3 falsification consumed the prior segment; the rework itself verified first-try after the fdtput byte-encoding was pinned empirically (dtc 1.8.1 `-t lx` truncates u64 to 4 bytes — use `-t x` with hi/lo cells).
- Task 3 took three guest iterations (CONF indentation, rc break, pc regex) — each failure was harness-side; the donor kernel and tracehello behaved correctly throughout.

## User Setup Required
None — no external service configuration required.

## Next Phase Readiness
- **04-02** unblocked: committed `probe-tree.paq` fixture (sha256 above) + the `mkpaqfs-host.sh` producer recipe; the fixture covers NoEnc + DeflateEnc from one image.
- **04-06/07/08** syscall set pinned: measured {pwrite(51), open(14), pread(50), close(4), exits(8)} ∪ D-14's named core {RFORK, NOTIFY, NOTED, AWAIT, EXITS, EXEC, OPEN, CLOSE, PREAD, PWRITE}.
- **04-03** inherits the virtexec profile shape (TOML-data initrd) for any exec-path boot profiles.
- The `virtexec` profile's probe-initrd.txt is delivery-machinery probe bytes; 04-02 replaces it with the paqfs image per plan.

## Self-Check: PASSED
- All 10 key files exist (checked with `[ -f ]`).
- Commits: r9 `git log --grep="04-01"` → 267dbe8, 04e6dcb; Harvey → 498945a1e, cb1784a7f (+ metadata commit).
- Acceptance greps re-run at final state: property_value_as_u64 pub ✓, linux,initrd-start/end in main.rs ✓, loader emission in config.rs ✓ (adapted), config_virtexec.toml initrd key ✓, fixture git-tracked ✓, trace artifact shape + sys.h resolution ✓.
- Plan-level verification: serial markers ✓ (leg 2 + xtask smoke), fixture byte magics ✓, artifact contents ✓, platform no-regression ✓ (virt boot-test PASS; raspi3b/raspi4b builds green — kernel code unchanged since those runs; the later r9 commit adds only inert fixture data).

---
*Phase: 04-r9-userspace-exec-harvey-r9-abi*
*Completed: 2026-11-02*
