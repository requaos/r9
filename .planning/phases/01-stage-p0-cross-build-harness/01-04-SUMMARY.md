---
phase: 01-stage-p0-cross-build-harness
plan: 04
subsystem: cross-build-harness-spike
tags: [plan9, aout, rust-lld, qemu, hvf, kvm, harvey, d-17, d-18, s6]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "D-17/D-18, the Harvey pin 25d97dc4d, and the S5 scratch-boot venue"

provides:
  - "no_std amd64/arm64 a.out probes plus a wrapper/checker that mirrors r9's parse_aout"
  - "arm64 swap proven end to end under HVF: control, probe marker, flip-back with identical image bytes"
  - "D-17 target recommendation: ps (echo is not an rc builtin and drives cpurc's marker ladder)"
  - "owner-run x86_64 KVM script and expect driver for Plan 06"

affects: [pass-2 manifest and assembler, Phase 9 elf2aout, Plan 06 KVM witness]

actuals:
  tokens: 19217
  tasks: 3
  commits: 3

plan_head_before: 0fde99cba2c99d11b4d6258b8c21881d2fbabb4e
plan_head_after: 9f2ac3c8755b4945b3c20617ea4bd9e09e2ac516

tech-stack:
  added: []
  patterns:
    - "Flat rust-lld --oformat=binary payload linked at INITTEXT, prefixed with the 40-byte big-endian Plan 9 a.out header"
    - "Scratch-only bake determinism shim: pin mtimes, pin the paq header time, recompute the trailer SHA1"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-amd64.rs
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-amd64.ld
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-arm64.rs
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-arm64.ld
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/aout.py
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/build-probe.sh
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/target-analysis.md
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/run-arm64-hvf.sh
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/virt-probe.expect
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/linux-x86_64.sh
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/q35shell-probe.expect
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/.gitignore
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/evidence/amd64-static.txt
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/evidence/arm64-hvf-control.log
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/evidence/arm64-hvf-probe.log
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/evidence/arm64-hvf-flipback.log
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/S6-FINDINGS.md
  modified: []

key-decisions:
  - "D-17 target is ps, not D-17's prose example echo: ps is staged in every chain-family image, has zero boot-path invocations and two gate sites; echo is not an rc builtin (sys/src/cmd/rc/plan9.c:35-48), so every bare echo in cpurc and the smoke gates execs the replacement."
  - "The arm64 leg uses cargo xtask qemu rather than boot-test: qemu leaves the -serial mon:stdio console interactive so the expect driver can type each leg's own command; boot-test drives a fixed probe string."
  - "The aarch64 bakes are made deterministic in the scratch copy only: Harvey's aarch64 mkimage.sh has no KERNDATE/mtime pin and this checkout's prebuilt darwin mkpaqfs predates KERNDATE, so run-arm64-hvf.sh wraps mkpaqfs to pin mtimes/header time and recompute the trailer SHA1. Harvey itself is untouched."
  - "HARN-04 is not marked complete in pass 1: S6 exercises the mechanism but the manifest and assembler land in pass 2 (D-22)."

patterns-established:
  - "Probe build: plain rustc from the devshell, relocation-model=static, -T<script> and --oformat=binary, never added to the cargo workspace"
  - "Swap proof: a scratch Harvey copy, Harvey's own mkimage/mkpaqfs, control/probe/flip-back legs, and image sha256 equality as the flip-back witness"

requirements-completed: []

coverage:
  - id: D1
    description: "amd64 no_std probe linked flat at 0x200028 and wrapped in the 40-byte a.out header, with every x86_64 parse_aout rule checked statically and both syscalls visible in the disassembly"
    requirement: HARN-04
    verification:
      - kind: integration
        ref: "sh .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/build-probe.sh amd64 && python3 -I .../aout.py check --arch amd64 .../work/probe-amd64.aout"
        status: pass
      - kind: other
        ref: "grep 'AOUT-OK amd64' and both syscall sequences in s6/evidence/amd64-static.txt"
        status: pass
    human_judgment: false
  - id: D2
    description: "arm64 swap end to end under HVF: control marker, probe marker from the replaced echo, flip-back marker, and identical bootfs.paq sha256"
    requirement: HARN-04
    verification:
      - kind: integration
        ref: "sh .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/run-arm64-hvf.sh -> S6-ARM64 control=PASS probe=PASS flipback=PASS sha-equal=yes"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-17 target analysis with every boot-path and gate invocation listed and a recommendation"
    verification:
      - kind: other
        ref: "s6/target-analysis.md (11 ## sections; citation inventory)"
        status: pass
    human_judgment: true
    rationale: "The target choice is a judgment over the cited invocation inventory; the verifier should confirm that ps is acceptable to the owner for the pass-2 swap."
  - id: D4
    description: "x86_64 KVM leg scripted and statically checked for Plan 06"
    requirement: HARN-04
    verification:
      - kind: other
        ref: "nix run nixpkgs#shellcheck -- s6/linux-x86_64.sh s6/run-arm64-hvf.sh s6/build-probe.sh; bash -n; tcl info complete = 1"
        status: pass
    human_judgment: true
    rationale: "The KVM witness itself must run on the owner's x86_64 Linux host in Plan 06 (PENDING-OWNER); the Mac can only prove the script is well-formed."
  - id: D5
    description: "S6-FINDINGS verdicts and the D-17/D-18 pass-2 recipe"
    verification: []
    human_judgment: true
    rationale: "Prose findings and the pass-2 recommendation are reviewed by the owner at the pass-1 checkpoint."

# Metrics
duration: 6 min
completed: 2026-11-23
status: complete
---

# Phase 1 Plan 4: S6 minimum Rust-to-Plan-9-a.out probe Summary

**Rust-to-a.out swap proven on arm64 HVF with byte-identical flip-back; ps recommended for D-17 and the owner-run x86_64 KVM leg scripted for Plan 06**

## Performance

- **Duration:** 6 min
- **Started:** 2026-11-23T00:47:07Z
- **Completed:** 2026-11-23T00:52:50Z
- **Tasks:** 3
- **Files modified:** 17 (16 created, 1 modified)

## Accomplishments

- Built a `no_std` Rust probe for amd64 and arm64 with plain `rustc`/`rust-lld`
  (`--oformat=binary`, static relocation model, flat payload at INITTEXT) and
  a stdlib-only `aout.py` that prepends the 40-byte big-endian header and
  asserts every rule of r9's `parse_aout`/`read_aout`. The amd64 disassembly
  at 0x200028 starts at `_start` and shows `mov ebp, 0x33` + `syscall` and
  `mov ebp, 0x8` + `syscall`; the header is 0x8A97/96 bytes/entry 0x200028.
- Proved the swap end to end on this Mac under HVF: a scratch copy of
  Harvey's arm64 tree re-baked `bootfs.paq` with Harvey's own `mkimage` and
  `mkpaqfs`, the probe replaced `arm64/bin/echo`, and typing `/bin/echo x`
  printed `R9-RUST-AOUT-PROBE-OK`. The control and flip-back legs printed
  `R9-ECHO-CONTROL`, and the flip-back image is byte-identical to control
  (`746f1dfe...`).
- Analysed every `ls cat echo ps rm test` invocation on the x86_64 boot path
  and gates. `echo` is not an rc builtin, so it drives cpurc and the smoke
  markers; `ps` is staged everywhere, absent from the boot path and used by
  only two gate sites. Recommended `ps` for D-17.
- Scripted the owner-run x86_64 KVM leg (`linux-x86_64.sh` +
  `q35shell-probe.expect`): re-bake `bootfs-chain.paq` with KERNDATE,
  boot q35shell under KVM, and run control/probe/flip-back with
  restore-and-reap traps. Statically checked on the Mac (bash, shellcheck,
  Tcl `info complete`); the run itself is PENDING-OWNER in Plan 06.

## Task Commits

Each task was committed atomically:

1. **Task 1: amd64 probe, wrapper/checker, target analysis** - `f961eaf` (feat)
2. **Task 2: arm64 HVF end-to-end swap** - `5536639` (feat)
3. **Task 3: x86_64 KVM leg script and pass-2 recipe** - `9f2ac3c` (feat)

**Plan metadata:** pending (docs: complete plan)

## Files Created/Modified

- `spikes/s6/probe-amd64.rs`, `probe-amd64.ld` — amd64 `_start` (PWRITE then EXITS) and the flat-text linker script at 0x200028.
- `spikes/s6/probe-arm64.rs`, `probe-arm64.ld` — arm64 twin at 0x10028 with the X0/SP+8/SVC ABI.
- `spikes/s6/aout.py` — `wrap`/`check` for the 40-byte header; mirrors r9's parse rules for both magics.
- `spikes/s6/build-probe.sh` — plain-rustc build from the devshell plus wrap/check.
- `spikes/s6/target-analysis.md` — per-tool invocation inventory and the `ps` recommendation.
- `spikes/s6/run-arm64-hvf.sh`, `virt-probe.expect` — scratch Harvey copy, deterministic bakes, three HVF legs.
- `spikes/s6/linux-x86_64.sh`, `q35shell-probe.expect` — owner-run x86_64 KVM leg for Plan 06.
- `spikes/s6/evidence/amd64-static.txt` — header bytes, AOUT-OK, disassembly, sha256.
- `spikes/s6/evidence/arm64-hvf-{control,probe,flipback}.log` — 199-line console excerpts.
- `spikes/S6-FINDINGS.md` — verdicts, commands, determinism note, assumption A7, pass-2 recipe.

## Decisions Made

- **D-17 target: `ps`.** It is in every chain-family x86_64 image, is absent
  from the boot path, and has two gate sites. `echo` is disqualified because
  it is not an rc builtin (`sys/src/cmd/rc/plan9.c:35-48`): every bare echo in
  `cpurc` and the typed smoke markers would run the replacement.
- **Boot command: `cargo xtask qemu`, not `boot-test`.** qemu leaves the
  `-serial mon:stdio` console interactive so the expect driver can type each
  leg's command; `boot-test` drives its own fixed probe string.
- **Determinism is scratch-only.** Harvey's aarch64 `mkimage.sh` has no
  KERNDATE/mtime pin and the checkout's prebuilt darwin `mkpaqfs` predates
  KERNDATE, so the run script wraps `mkpaqfs` in the scratch copy to pin
  mtimes, header time and the trailer SHA1. The real checkout is untouched.
- **HARN-04 is not marked complete.** S6 proves the mechanism; the manifest
  and assembler are pass-2 work (D-22).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Deterministic aarch64 bakes needed a scratch-only mkpaqfs shim**
- **Found during:** Task 2 (arm64 HVF leg)
- **Issue:** two bakes of the same arm64 tree differed (21 bytes: header time
  and the trailer SHA1), so the planned flip-back sha256 equality could not
  hold. Harvey's aarch64 mkimage has no KERNDATE/mtime pin, and this
  checkout's prebuilt darwin `mkpaqfs` predates the KERNDATE feature.
- **Fix:** `run-arm64-hvf.sh` installs a wrapper at the scratch copy's
  `.build/bin/mkpaqfs` that pins staged mtimes, runs Harvey's real binary
  (`mkpaqfs.real`) and then pins the header time to 1795382101 and recomputes
  the trailer digest. No Harvey file is changed.
- **Files modified:** `spikes/s6/run-arm64-hvf.sh` (shim embedded)
- **Verification:** two bakes of the control tree are byte-identical;
  control and flip-back images both hash to `746f1dfe...`.
- **Committed in:** `5536639` (Task 2 commit)

**2. [Rule 1 - Bug] arm64 linker ALIGN(16) put `_start` after the entry VA**
- **Found during:** Task 2 (first arm64 build)
- **Issue:** the linker script's `ALIGN(16)` on the output section placed the
  payload at 0x10030 while the header entry stayed 0x10028, so the loader
  would have entered 8 bytes of padding.
- **Fix:** removed the alignment (0x10028 is already 8-byte aligned) and
  verified `_start` is the first symbol/instruction.
- **Files modified:** `spikes/s6/probe-arm64.ld`
- **Verification:** `llvm-nm` shows `_start` at 0x10028; the probe boots.
- **Committed in:** `5536639`

**3. [Rule 1 - Bug] Intel-syntax absolute immediate rejected by LLVM**
- **Found during:** Task 1 (first amd64 build)
- **Issue:** `mov qword ptr [rsp + 24], marker_len` failed with "invalid
  operand for instruction" (LLVM Intel syntax does not take the symbol as a
  memory immediate).
- **Fix:** compute the length at runtime as `marker_end - marker`; the arm64
  twin uses the same label arithmetic.
- **Files modified:** `spikes/s6/probe-amd64.rs`, `spikes/s6/probe-arm64.rs`
- **Verification:** both probes build; the marker length is 22 in the debug
  ELFs.
- **Committed in:** `f961eaf` (amd64), `5536639` (arm64)

**4. [Rule 1 - Bug] shellcheck SC1007 on the CDPATH assignment**
- **Found during:** Task 2 (shellcheck pass)
- **Issue:** `here=$(CDPATH= cd -- ...)` warns "remove space after =".
- **Fix:** `CDPATH='' cd -- ...` in `build-probe.sh`, `run-arm64-hvf.sh` and
  the embedded shim.
- **Files modified:** `spikes/s6/build-probe.sh`, `spikes/s6/run-arm64-hvf.sh`
- **Verification:** shellcheck clean on all three scripts.
- **Committed in:** `5536639`

**5. [Rule 3 - Blocking] llvm-objdump cannot read the flat payload directly**
- **Found during:** Task 1 (disassembly evidence)
- **Issue:** `llvm-objdump -D` on a raw binary fails with "file was not
  recognized as a valid object file".
- **Fix:** wrap the flat payload byte-verbatim with `llvm-objcopy -I binary
  -O elf64-x86-64 --change-section-address .data=0x200028`, then
  `llvm-objdump -D`; the bytes and VMAs are the flat payload's.
- **Files modified:** `spikes/s6/evidence/amd64-static.txt` (method note)
- **Verification:** the disassembly starts at 0x200028 with `_start` and
  shows both syscalls.
- **Committed in:** `f961eaf`

---

**Total deviations:** 5 auto-fixed (3 Rule 1 bugs, 2 Rule 3 blockers).
**Impact on plan:** all fixes were needed for the probe to build and for the
flip-back byte-equality proof to be meaningful; no scope creep and no
changes to Harvey, r9 sources outside `.planning`, or any protected path.

## Issues Encountered

- The arm64 HVF console logs contain r9's demand-page `fault:` chatter; the
  probe marker, the two control markers and the prompts are all present, so
  no gate requirement is affected.
- The x86_64 KVM leg could not run on this Mac (no `/dev/kvm`; grub2/xorriso
  are Linux-devshell only). It is scripted and statically checked, and is
  PENDING-OWNER for Plan 06 as the plan anticipated.

## Pending Owner Work

- **Plan 06:** run `s6/linux-x86_64.sh` on the x86_64 Linux host with the S3
  scratch Harvey tree and the r9 clone at
  `116bcc23c8052a1728992cdbc7b625143260d683`; expected
  `S6-X86 control=PASS probe=PASS flipback=PASS sha-equal=yes`.

## User Setup Required

None — no external service configuration; all work ran in the local devshell
and scratch directories.

## Next Phase Readiness

- Pass-2 planning can use the proven recipe: plain rustc + flat link +
  40-byte header, with `ps` as the D-17 swap target.
- The x86_64 KVM witness is the one open S6 leg; Plan 06's run-all should
  close it before the pass-1 checkpoint.
- Harvey remains at `25d97dc4d` with its pre-existing status hash; r9's
  protected paths and `.planning`-only diff are unchanged by this plan.

---
*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED
