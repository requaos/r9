---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 13
subsystem: arm64 boot configuration (DTB bootconf), aarch64 virtio PCI bind
tags: [r9, aarch64, bootconf, dtb, fdtput, devenv, virtio, pci, hardening, ARM-10, ARM-09]
status: complete
requires: ["20-12"]
provides:
  - "BOOTCONF-CARRIER: bootargs (measured on hvf, tcg and gicv2; owner-confirmed chosen-bootargs)"
  - "Harvey env contract R9_BOOTARGS / R9_BOOTARGS_PROP (default bootargs) in nix/smoke-r9.sh and nix/boot-r9.sh; unset means the DTB copy is byte-identical"
  - "r9 aarch64/src/platform/virt/bootconf.rs: read_bootargs, BootArgs::from_raw, parse_bootconf_pairs, Bootconf, BootconfError"
  - "arm64 ROOT env seed = the three compiled pairs + the DTB loader pairs, printed as 'devenv: bootconf k=v ...'"
  - "r9 aarch64 check_cap_extents in the virtio bind (x86_64's four rules)"
affects: [20-14, 20-15, 20-16, 20-17]
tech-stack:
  added: []
  patterns:
    - "opt-in launcher seam: one extra fdtput only when the env var is non-empty, so the default DTB copy is byte-identical"
    - "read the DTB boot string after devcons::init, so a named panic for an over-long string reaches the console"
    - "the smoke driver's fast-fail on 'panic' cuts off the second line of the panic; prove a named panic through nix run .#boot-r9 (plain qemu on stdout)"
key-files:
  created:
    - ../r9/aarch64/src/platform/virt/bootconf.rs
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-13-SUMMARY.md
  modified:
    - nix/smoke-r9.sh
    - nix/boot-r9.sh
    - ../r9/aarch64/src/platform/virt/mod.rs
    - ../r9/aarch64/src/main.rs
    - ../r9/aarch64/src/platform/virt/pci.rs
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
key-decisions:
  - "Owner: the bootconf carrier is /chosen/bootargs (chosen-bootargs). The launchers keep R9_BOOTARGS_PROP=bootargs, and the reader takes that one property"
  - "main9 reads the boot string at the exec_bootstrap call site (DeviceTree in scope, after devcons::init), not before the console is up beside the initrd discovery, so the CmdlineTooLong panic is visible"
  - "The seed merge lives in the shared exec prologue (exec_prologue_with_bootargs). The churn driver keeps the unchanged exec_prologue(initrd_img) entry and seeds only the compiled pairs, so churngate.rs is untouched"
  - "BootArgs::from_raw holds the NUL cut and the 256-byte reject as a pure function beside the unchanged parse_bootconf_pairs, so 20-17 can pin both on the host"
requirements-completed: [ARM-10]
metrics:
  duration: "about 4 h 9 min wall (2026-11-21T09:17Z to 13:26Z), including the Task 2 checkpoint wait"
  completed: 2026-11-21
actuals:
  tokens: 6600
  tasks: 4
  commits: 2
plan_head_before: be06dae6ca6f432d74641c5eebe065a24a3cbc2e
plan_head_after: 3bc95787b23f559a93da5b3535d8b8058ef5a6ae
---

# Phase 20 Plan 13: arm64 boot config through DTB /chosen/bootargs (opt-in launcher fdtput, r9 reader merged after the three compiled seed pairs, named panic when malformed) plus x86_64's virtio capability-extent check Summary

Boot configuration now reaches the arm64 ROOT env seed. The launchers write `R9_BOOTARGS` onto `/chosen/bootargs` of the DTB copy, and the kernel reads it, parses it with a copy of the x86_64 pair parser, and appends the pairs after the three compiled ones. A bad string panics at first boot with a named message. The aarch64 virtio bind now has x86_64's capability-extent check, and it still binds the real qemu NIC on HVF and TCG.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Launcher seam + DTB delivery measurement | Harvey `da89798ec`, `3bc95787b` | nix/smoke-r9.sh, nix/boot-r9.sh, 20-EVIDENCE.md |
| 2 | Owner confirms the carrier | (decision: chosen-bootargs) | 20-EVIDENCE.md |
| 3 | Reader, parser and seed merge | r9 `12bce96` | aarch64/src/platform/virt/bootconf.rs, platform/virt/mod.rs, main.rs |
| 4 | Capability-extent hardening | r9 `de03bf7` | aarch64/src/platform/virt/pci.rs |

## Evidence

The full record is in `20-EVIDENCE.md` under `## Bootconf delivery`.

- **Carrier:** I ran eight measured boots with a print that was never committed. A `bootargs` that `fdtput` wrote arrives intact on hvf and tcg, NUL included (17 bytes for `p20probe=one two`). qemu adds no `bootargs` when none is written (gicv3 hvf/tcg and gicv2). `r9,bootconf` also survives, and a 300-byte string arrives whole. Result: `BOOTCONF-CARRIER: bootargs`. At the pin with the seam committed and `R9_BOOTARGS` unset, the TCG smoke still prints `ALL STAGES PASS`.
- **Merge:** with the probe string, the smoke capture shows the seed line unchanged, then `devenv: bootconf p20probe=one two`, and the gate passes. With no string there is no bootconf line and exactly one seed line.
- **Panics:**
  - `=bad` gives `bootconf: DTB bootargs parse failed: InvalidBootconfToken`.
  - The 300-byte string gives `...: CmdlineTooLong`.
- **Builds:** all 13 aarch64 configs build with no new warnings, and there is no `port/src` diff.
- **Extents:** smoke and net-smoke both exit 0 on HVF and on TCG (`r9-kernel-virt-de03bf7`). All four print the NIC bind line and the live caps line, `notify@0x3000` in a 0x4000 BAR.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The read moved after the console comes up.**
- **Found during:** Task 3.
- **Issue:** the plan put the `read_bootargs` call beside the initrd discovery. That code runs before `devcons::init`, so the `CmdlineTooLong` panic would not reach the console.
- **Fix:** the read happens at the `exec_bootstrap(initrd_img, read_bootargs(&dt))` call site in `main9`, where the DeviceTree is still in scope and the console is live. The 300-byte boot shows the named panic.
- **Commit:** r9 `12bce96`.

**2. [Rule 3 - Blocking] The seed lives in the shared prologue, not in `exec_bootstrap` itself.**
- **Found during:** Task 3.
- **Issue:** the three-pair seed is in `exec_prologue`, which churngate's `bootstrap_and_run` also calls.
- **Fix:** the body became `exec_prologue_with_bootargs(initrd_img, bootargs)`, and `exec_bootstrap` passes `bootargs.as_bytes()`. A wrapper `exec_prologue(initrd_img)`, gated `all(dev_exec, dev_churn)`, keeps churngate's call unchanged and passes an empty string.
- **Effect:** the churn-driver profile (dev_churn without bootchain) does not honor a boot string; it has no boot(8) that would read one. bootconf.rs allows dead code in that cfg, and virtchurn builds clean. The commit lists exactly the three planned r9 files.

**3. [Rule 3 - Blocking] The smoke driver hides the named panic.**
- **Found during:** Task 3, the third verify check.
- **Issue:** `smoke-r9.expect` fast-fails on the substring `panic` and kills qemu at `panicked at ...:`, before the message line arrives. The check therefore printed `exit=1 named=0`. The gate does fail, as it should, but its capture lacks the name.
- **Fix:** none to the harness. I proved the same boot through `nix run .#boot-r9` with the output captured (`.build/p20-13-t3-bad-hvf-direct.out`), which shows the full two-line panic. If 20-16/20-17 want the name inside a gate capture, the expect panic branch needs a short drain of the rest of the line before cleanup.

**4. [Rule 2 - Correctness] Added a pure `BootArgs::from_raw`.**
- **Why:** the plan's parser behaviors include "more than 256 bytes is CmdlineTooLong" and "bytes after the NUL are ignored". On x86_64 those live in `parse_cmdline`, not in `parse_bootconf_pairs`.
- **Fix:** I kept `parse_bootconf_pairs` identical and put both rules in `BootArgs::from_raw`, so 20-17 can pin them without a DTB.

### Scope notes

- Both scratch host tests (the parser and `check_cap_extents`) used the route-A one-line scratch edit to `platform/mod.rs` and were reverted. The real pins and the real fix land in 20-17 Task 1.

## Known Stubs

None.

## Threat Flags

None beyond the register:
- **T-20-33:** bounded copy plus named panic; proven for both the parse and the length panic.
- **T-20-35:** `port::fdt` accessors only; no `unsafe` added.
- **T-20-41:** `check_cap_extents` runs before any cap is trusted.
- **T-20-34:** accepted. Duplicate keys are appended after the compiled pairs, and their override behavior is unchanged.
- **T-20-SC:** no package added.

## Notes

- `commits` is the measured Harvey count `be06dae6c..HEAD` when this was written: `da89798ec` and `3bc95787b`. The final docs commit (this SUMMARY, the evidence for Tasks 2 to 4, STATE, ROADMAP, REQUIREMENTS) adds one more. The r9 commits are `12bce96` and `de03bf7` on r9 `main`, local and unpushed (r9 `origin/main` is still `fa38402`).
- The pin did not move. Harvey's `flake.lock` still names r9 `fa38402`, so the four existing arm64 apps boot the unchanged `kernel-virt` without an override. Every kernel-byte change in this plan ran only through `--override-input`. A pin move needs the 20-06-shape HVF series first and is the owner's call. No x86_64 file changed.
- Nothing was pushed. The 11 pre-existing font/postscript modifications were left unstaged.
- Requirements: ARM-10 is marked complete. ARM-09 is only half done here (the `check_cap_extents` half); its devcap/devssl half is 20-15's, which owns `requirements: [ARM-09]`, so it stays pending.
- `actuals.tokens` is chars/4 over the added lines in the r9 diff, the two launchers and the evidence additions.

## Self-Check: PASSED

- FOUND: `../r9/aarch64/src/platform/virt/bootconf.rs` (tracked in `12bce96`).
- FOUND (ancestors of HEAD): Harvey `da89798ec`, `3bc95787b`; r9 `12bce96`, `de03bf7`.
- `## Bootconf delivery` has exactly one `BOOTCONF-CARRIER: bootargs`, and `grep -c R9_BOOTARGS` gives 10 per launcher.
- `git -C ../r9 status --porcelain` is empty, and `git -C ../r9 diff --stat origin/main..HEAD -- port/src` is empty.
- In `pci.rs`, `check_cap_extents` appears twice outside comments.
