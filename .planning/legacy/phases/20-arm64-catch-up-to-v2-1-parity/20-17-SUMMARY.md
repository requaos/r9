---
phase: 20-arm64-catch-up-to-v2-1-parity
plan: 17
subsystem: aarch64 host-test battery, r9 + Harvey CI lanes, the final group C pin
tags: [arm64, aarch64, host-tests, xtask, ci, pin, flake, ARM-03, ARM-05, ARM-11, ARM-12]
status: complete
requires: ["20-16"]
provides:
  - "cargo xtask test runs the aarch64 crate on an aarch64 host under nix (host-triple fallback), at default and virtshell cfg"
  - "host pins: shebang_pass 6, valid_stat_wire + remove/wstat routing 8, parse_bootconf_pairs 6, check_cap_extents 5"
  - "r9 CI lanes build-aarch64-virtcpu and test-aarch64-host (ubuntu-24.04-arm), green at e6e5220"
  - "Harvey CI job cpu-smoke (nix run .#boot-r9-cpu, TCG), green at 3f16b791d"
  - "repeat-gate.sh accepts boot-r9-cpu (capture R9_CPU_LOG)"
  - "N-HVF boot-r9-cpu = 36; N-HVF boot-r9-tier1 = 10"
  - "Harvey pin r9 821db85 -> e6e5220 (97cb3c7bb, lock-only); confidence pass green"
affects: []
tech-stack:
  added: []
  patterns:
    - "a host-test leg under a kernel profile reuses the build step's own cfg source (apply_platform_config) via build.rustflags, so the tests see exactly the profile's cfgs"
    - "kernel-byte equivalence after a test-only change: compare .text/.rodata/.data per section with llvm-objcopy --dump-section; panic Location line fields are the only expected .data deltas"
key-files:
  created:
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-17-SUMMARY.md
  modified:
    - ../r9/aarch64/src/exec.rs
    - ../r9/aarch64/src/syscall.rs
    - ../r9/aarch64/src/main.rs
    - ../r9/aarch64/src/platform/mod.rs
    - ../r9/aarch64/src/platform/virt/bootconf.rs
    - ../r9/aarch64/src/platform/virt/pci.rs
    - ../r9/xtask/src/main.rs
    - ../r9/xtask/src/config.rs
    - ../r9/.github/workflows/xtask.yml
    - build/aarch64/repeat-gate.sh
    - .github/workflows/r9-shell.yml
    - flake.lock
    - .planning/STATE.md
    - .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md
key-decisions:
  - "Owner: full 20-06-shape HVF series stopped at e6e5220 and deferred to the upcoming SMP work (not phase end); the quick confidence pass stands in for it"
  - "Route A fix: the raspi3b glob's test arm is narrowed to all(test, not(platform = \"virt\")); the textcopy selftest call is gated not(test) like its callee"
  - "The xtask host-triple fallback is aarch64-only; the x86_64 and riscv64 legs keep rustup-only selection"
  - "The shebang recursion cap has no host pin and no live gate (known gap, recorded)"
requirements-completed: [ARM-03, ARM-05, ARM-11, ARM-12]
metrics:
  duration: "about 3h (2026-11-22T15:30Z to 18:40Z), including the owner checkpoint, the stopped series and hosted CI"
  completed: 2026-11-22
actuals:
  tokens: 14393
  tasks: 4
  commits: 5
plan_head_before: 8ca3998705d84e31730fd05de7c371971ea80aa2
plan_head_after: 3f16b791dcba30be4c85b349aea6a9998239a9b0
---

# Phase 20 Plan 17: aarch64 host-test battery and group C at the pin Summary

`cargo xtask test` now runs the aarch64 crate's host tests under nix, at both the default and the virtshell cfg. They pin the four cpu-port pure functions with 25 new tests. r9 `e6e5220` is pushed with two new CI lanes, and Harvey is pinned to it lock-only. The quick confidence pass is green at the pin. Both repos' hosted CI is green, including the new `test-aarch64-host` and `cpu-smoke` jobs. The owner stopped the full HVF series after a green partial run and deferred it to the SMP work.

## Tasks

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | aarch64 host-test battery | r9 `02c3d28` | exec.rs, syscall.rs, main.rs, platform/mod.rs, virt/bootconf.rs, virt/pci.rs, xtask main.rs + config.rs |
| 2 | CI lanes, r9 battery, push r9 | r9 `e6e5220` (pushed by the coordinator on owner approval) | .github/workflows/xtask.yml |
| 3.2 | repeat-gate accepts boot-r9-cpu | `204b7f6d3` | build/aarch64/repeat-gate.sh |
| 3.3 | battery + N recorded | `0151ef255` | 20-EVIDENCE.md |
| 3.5 | cpu-smoke CI job | `5c3f2c239` | .github/workflows/r9-shell.yml |
| 3.1 | lock-only pin move | `97cb3c7bb` | flake.lock |
| 3.6 | records | `3f16b791d` | STATE.md, 20-EVIDENCE.md |
| 3.7 | Harvey push | `8ca399870..3f16b791d` | — |
| 4 | owner checks | approved | — |

## Results

- **Host-test battery.** The aarch64 leg used to run 0 tests. It now runs 71 at the default cfg and 87 at the virtshell cfg. The verify reads `shebang=12 stat_wire=16 bootconf=6 cap_extents=5 bad=0`. Every `<behavior>` has a test.
- **Ported pins with no host equivalent.** The x86_64 pins built on MockStack / `new_user_proc`, plus the devcap eve-hook half, are listed in 20-EVIDENCE.md `## Host-test battery`, each with the live `boot-r9-cpu` leg that covers it.
- **No kernel behavior change.** At `e6e5220`, both kernels have `.text` and `.rodata` identical to `821db85`. `.data` differs in 13 bytes, each +3: the panic line fields after three comment lines were added in main.rs. `boot-r9-cpu` passed on HVF with the r9 override.
- **r9 pre-push battery.** Green: check, clippy, test, the virtshell/virtcpu/virtchurn/default builds, ureg-check, and `nix build .#kernel-virt .#kernel-virt-cpu`. The `port/src` and `flake.nix` diff stats are empty.
- **N values.** `N-HVF boot-r9-cpu = 36`: 17 forks per boot gives ceil(30.55) = 31, so the floor of 36 decides. `N-HVF boot-r9-tier1 = 10`: 189 forks per boot gives 3, so the floor of 10 decides.
- **Series at the pin.** It was stopped by the owner. The partial run was all green: tier-1 HVF 10/10 (3890 forks) and net-smoke HVF 12/12 (852 forks). This is recorded as `SERIES-AT-PIN: deferred-to-SMP-work (owner)`.
- **Confidence pass at `e6e5220`.** Tier-1 passed on HVF and TCG, at 44/23/0 with the seven r9 probe markers once each. Smoke, net-smoke and `boot-r9-cpu` passed on HVF, and `boot-r9-cpu` on TCG. Every run exited 0. The cpu capture has the seven markers once each and 0 `cpurc: FAIL`.
- **Hosted CI.**
  - r9 at `e6e5220` is green per the owner, including `test-aarch64-host` and `build-aarch64-virtcpu`.
  - Harvey at `3f16b791d` is green across r9-shell (`boot-smoke`, `net-smoke`, `cpu-smoke`, `net-smoke-x64`, `auth-smoke-x64`), phase6-userspace (`tier-r9`, `tier-9k`, `tier-2`, `x86_64-poolpanic`, `wave0-host`) and harvey (`build`).
  - The tier-9k capture shows `T1-VENUE-PASS ok=36 skip=22+2libtest fail=0` and has 0 r9-only marker lines.
- **Linux boot-iso.** No x86_64 byte has changed since `c94bc9b`, where the owner's check passed.

## Deviations from Plan

**1. [Rule 3 - Blocking] The textcopy selftest call had no gate.** A second compile break in the virtshell-cfg test build (E0425 at `main.rs:441`) appeared after 20-12 measured the route. The call is now gated `not(test)`, as its callee already was. Commit: r9 `02c3d28`.

**2. The battery was re-cut before the push.** `f94cf12` was re-made as `02c3d28` with one added dispatch-slot pin (x86_64 `cpu03_slots_carry_named_handlers`), using `git reset --mixed` with no hard reset. No non-test byte changed between the two.

**3. The cpu-smoke job got its own commit (`5c3f2c239`).** The plan put it in the records commit. I committed it early so it would survive the checkpoint.

**4. The series was replaced by the confidence pass (owner).** See the results above. The owed series is listed with its N values in STATE.md and in the evidence file.

**5. No new pure-function gate was needed.** `bootconf` and `check_cap_extents` were already gated `any(test, …)` since 20-13, and `shebang_pass` and `valid_stat_wire` are ungated.

**6. The `<behavior>` argv text is read against the donor.** The behavior line wrote `[rc, -x, f, a]`, but the function's argv[0] is the script's own name, per the donor and x86_64. The test pins the interpreter `/bin/rc` and the argv `[f, -x, f, a]`.

REQUIREMENTS.md ARM-10 is untouched, because the carrier answer was `chosen-bootargs`. The 11 pre-existing font/postscript modifications were never staged. No r9 `port/src` edit was made.

## Known Stubs

None.

## Known gaps (recorded, not stubs)

- The `#!` recursion cap (`MAX_SHEBANG_DEPTH`) has no host pin and no live gate.
- The full HVF series is deferred to the SMP work. It owes smoke 1753, net-smoke 36, tier-1 10 and cpu 36.

## Threat Flags

None. T-20-46: the pin rev equals `origin/main` and the pin commit is lock-only. T-20-47: no x86_64 change; boot-iso passed at `c94bc9b`. T-20-48: explicit-path staging, with the 11 font files untouched. T-20-45: section-compare plus a cpu gate re-run after the gating. T-20-SC: no package added, and the new CI jobs reuse the pinned action SHAs.

## Self-Check: PASSED

- FOUND: build/aarch64/repeat-gate.sh, .github/workflows/r9-shell.yml (cpu-smoke), ../r9/.github/workflows/xtask.yml (2 new jobs)
- FOUND commits: 204b7f6d3, 0151ef255, 5c3f2c239, 97cb3c7bb, 3f16b791d (Harvey); 02c3d28, e6e5220 (r9, on origin/main)
