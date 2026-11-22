---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 15
subsystem: arm64 devcap and devssl device arms, the virtcpu profile and kernel-virt-cpu package
tags: [r9, aarch64, devcap, devssl, sslprobe, mount, virtcpu, kernel-virt-cpu, flake, ARM-09]
status: complete
requires: ["20-14"]
provides:
  - "CAP-COMPILES-UNGATED: yes; SSL-FVERSION: unrestricted; PORT-EDIT-NEEDED: no; CAP-WRITER: unreached (source: boot user passes the eve gate); ARMS-ROUTE: aarch64-only"
  - "r9 505494e: aarch64 err_dyn; DEV_CAP and DEV_SSL arms in sys_pread and sys_pwrite; sys_mount accepts DEV_SSL; devcap tick hook under cfg(dev_cap); devssl fd-resolve hook ungated"
  - "r9 821db85: aarch64/lib/config_virtcpu.toml (virtshell + cap) and flake package kernel-virt-cpu (kernel-virt.elf + virt-gicv{2,3}.dtb)"
  - "sslprobe: ok roundtrip and sslprobe: ok mount on HVF and TCG (were FAIL pushssl0)"
  - "devcap live on virtcpu: ls lists caphash/capuse, caphash write as the boot user succeeds, bogus capuse gives 'invalid capability glenda@bogus@key', rm caphash hides it"
  - "CAP-PROFILE: virtcpu"
affects: [20-16, 20-17]
tech-stack:
  added: []
  patterns:
    - "a device whose write error carries formatted bytes completes the syscall inside its arm (fd_op_done, then map CapErr), outside the &'static str funnel"
    - "a scratch dev-key edit to a tracked TOML is visible to the git+file override, so a cap kernel can be measured live without a commit"
    - "macOS BSD xargs -I refuses long replacement commands ('command line cannot be assembled, too long'); run the store-path check without xargs"
key-files:
  created:
    - ../r9/aarch64/lib/config_virtcpu.toml
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-15-SUMMARY.md
  modified:
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/main.rs
    - ../r9/flake.nix
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
key-decisions:
  - "Owner: proceed-aarch64-only at the Task 2 checkpoint; no port/src edit; Task 3 step 4 (remove_chan devcap arm) and step 6 (sys_fversion) recorded as already satisfied"
  - "kernel-virt-cpu duplicates the kernel-virt derivation rather than factoring a shared helper, so the kernel-virt expression is untouched (its ELF stays byte-identical)"
requirements-completed: [ARM-09]
metrics:
  duration: "about 55 min of execution (2026-11-22T15:20Z to 16:15Z), plus the Task 2 owner checkpoint"
  completed: 2026-11-22
actuals:
  tokens: 7300
  tasks: 4
  commits: 3
plan_head_before: 90e30378ce000ae6dbd242bf13a6b670da328088
plan_head_after: a4f5393279ad9e91b4d4bf256da728c4e5424690
---

# Phase 20 Plan 15: arm64 devcap and devssl arms, and the virtcpu kernel that serves devcap Summary

devssl now works on arm64: `sslprobe` and `sslprobe -m` pass on HVF and TCG, where 20-12 recorded `FAIL pushssl0: pread: unknown device`. A new r9 profile `virtcpu` and package `kernel-virt-cpu` carry devcap. On that kernel the five devcap probes return the device's own answers. Every change is under `../r9/aarch64/` plus one flake derivation. Nothing under `port/src`, the x86_64 bytes or the boot-iso hash changed.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Measure the devcap and devssl unknowns on a scratch `cap` kernel | Harvey 925958864 | 20-EVIDENCE.md |
| 2 | Owner route checkpoint | answered `proceed-aarch64-only` | — |
| 3 | devcap and devssl arms and hooks | r9 505494e; Harvey 99914f7ab | aarch64/src/syscall.rs, aarch64/src/main.rs, 20-EVIDENCE.md |
| 4 | virtcpu profile and kernel-virt-cpu package | r9 821db85; Harvey a4f539327 | aarch64/lib/config_virtcpu.toml, flake.nix, 20-EVIDENCE.md |

All r9 commits are local and unpushed on `c94bc9b`. Harvey commits are local.

## What was measured (Task 1)

- `port::devcap` and `port::devssl` are declared without a cfg. `dev_cap` gates only namec's `'¤'` arm and the tick hook. Without the hook, devcap records 0 ticks; there is no panic.
- aarch64 `sys_fversion` has no device restriction. 917685c's fversion half lives in port (`devmnt.rs:1009`, and `ChanIo` at 274-282).
- On the scratch `cap` kernel the smoke gate passed on HVF and TCG. Every read or write reached the default `unknown device` arms. `rm '#¤/caphash'` already worked through `port::namec::remove_chan` (20-14).
- The scratch edit was reverted, and r9 was clean before Task 3.

## Inventory tick-off (Task 3)

- [x] pwrite `DEV_CAP` (inline; `fd_op_done` runs before the `return match`; `CapErr::Static` maps to `err`, `InvalidCapability` to `err_dyn`)
- [x] pwrite `DEV_SSL`
- [x] pread `DEV_CAP`
- [x] pread `DEV_SSL`
- [x] `sys_mount` accepts `DEV_PIPE` or `DEV_SSL` (ORDWR still required)
- [x] `err_dyn`
- [x] `#[cfg(dev_cap)] port::devcap::set_tick_hook(crate::platform::timer::ticks)`; `port::devssl::set_fd_resolve_hook(proc::current_fd_resolve)` (ungated)
- [x] Step 4, the `remove_chan` devcap arm: **already satisfied** by r9 `9173af0` (20-14). `remove_chan` is `port::namec::remove_chan`, whose `DEV_CAP` arm is at `namec.rs:467`.
- [x] Step 6, `sys_fversion`: **already satisfied, unchanged**. It has no restriction to widen.
- stat, fstat, seek and close have no `DEV_CAP`/`DEV_SSL` arm on x86_64. They route through port (`namec::stat_chan`, `close_chan`).

## Verification

- `cargo xtask build --arch aarch64 --config virtshell`: rc 0, 25 warnings (the pre-existing set). The `virtcpu` build: rc 0.
- `cargo test -p aarch64 --bins --target aarch64-apple-darwin`: 57/57.
- `sslprobe: ok roundtrip` and `sslprobe: ok mount` on HVF and TCG.
- HVF gates on `505494e` (override): smoke 0, net-smoke 0, tier1 0 (44 OK / 23 SKIP / 0 FAIL).
- `nix build .#kernel-virt-cpu` from `821db85`: `/nix/store/31bjaknv19fmbj22sxmvzh56iy83pzhg-r9-kernel-virt-cpu-821db85`, holding the ELF and both DTBs.
- `nix build .#kernel-virt` still builds, and its ELF is byte-identical to the `505494e` kernel, so the three existing gates were not re-run on Task 4 (the plan's virtcpu branch).
- `git diff origin/main..HEAD -- port/src` is empty. The flake diff adds 28 lines and removes 0, with nothing about `outputHash`, `kernel-x64`, grub2 or xorriso. `config_virtshell.toml` has no diff in the range.
- Probes after the port, on virtcpu (HVF and TCG identical):
  - `ls -l '#¤'` lists caphash and capuse.
  - `cat caphash` gives `devcap: permission denied`.
  - The 20-byte caphash write succeeds.
  - The bogus capuse write gives `invalid capability glenda@bogus@key`.
  - `rm caphash` succeeds, and caphash is then hidden.

## Deviations from Plan

### Plan corrections (owner-confirmed at Task 2)

**1. Task 3 step 4 was already done.** The `remove_chan` devcap arm came from 20-14 (r9 `9173af0`), so no edit was made.

**2. Task 3 step 6 needs no change.** `SSL-FVERSION: unrestricted`.

### Venue notes

**2a. The Task 3 count check finds 4 arms, not 5.** `grep -cE 'DEV_(CAP|SSL) =>'` on `aarch64/src/syscall.rs` prints 4 (the two pread and two pwrite arms), below the plan's `EXPECT>=5`. The fifth arm the plan expected was a `DEV_CAP` arm in `remove_chan`. That arm lives in port (`namec.rs:467`), and aarch64 `remove_chan` reaches it through `port::namec::remove_chan`, per correction 1. This is the same owner-confirmed correction, not a missing arm.

**3. The Task 4 verify command's `xargs -I` fails on macOS.** BSD xargs printed `command line cannot be assembled, too long` for the `sh -c` replacement. I ran the same checks without xargs: the store path holds `kernel-virt.elf` and `virt-gicv3.dtb` (`CAP-KERNEL-OK`). This is a planning-time defect in the check, not in the build. If 20-16 or 20-17 reuse that verify, it needs rewriting.

**4. The 20-14 evidence text under `## Group C ports` now has a sub-heading.** I inserted the 20-15 block directly under the heading, which the owner allowed, so the Task 3 and 4 greps (`-A120` / `-A140`) can see it. The existing 20-14 text now sits under a new `### devmnt remove and #! (20-14)` sub-heading. Its content is unchanged.

**5. Gate runs used the plan's own capture names.** The Task 3 HVF gate runs wrote `.build/p20-15-t1-boot-r9-*.out`, as the plan's verify command specifies, even though they belong to Task 3.

Otherwise the plan ran as written. No auth gates.

## Known Stubs

None.

## Threat Flags

None. T-20-39: the devcap arm only forwards to `port::devcap::write`, so the eve gate, the hash check and the one-use consume stay in port. T-20-40: the existing `proc::current_fd_resolve` is registered unchanged. T-20-42: `fd_op_done` runs on every path of every arm touched; the devcap arm calls it before its `return match`.

## Still owed (not this plan)

- The r9 push of `505494e` and `821db85`, and the Harvey pin move, need an owner checkpoint (20-17). At that pin move run only the confidence pass: tier-1 once on HVF and once on TCG, then smoke and net-smoke once each on HVF. Record "full series deferred to phase end by owner".
- No x86_64 bytes changed, so no boot-iso re-derive or amd64 container run is owed for this plan.
- The devcap gate proof (`boot-r9-cpu` on `kernel-virt-cpu`) belongs to 20-16.

## Self-Check: PASSED

- FOUND: ../r9/aarch64/lib/config_virtcpu.toml; ../r9/aarch64/src/syscall.rs (err_dyn, DEV_CAP/DEV_SSL arms); ../r9/aarch64/src/main.rs (both hooks); ../r9/flake.nix (kernel-virt-cpu)
- FOUND commits: r9 505494e, 821db85; Harvey 925958864, 99914f7ab, a4f539327
