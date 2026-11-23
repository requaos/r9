# Pass-2 witness ledger

Stop A's rio de-risk evidence is `../rio-derisk/RIO-DERISK.md`; the driver contract and
the return paths are in `README.md`. This ledger records the pass-2 verdicts, stop by
stop.

## Stop B — plan 01-22, x86_64 KVM

- **Return:** `witness-returns/01-22` at `9100fcea4855109961ef91fc8b0f031a4b3da65e`;
  return directory `spikes/linux-witness/return-01-22/` (fetched read-only, not merged).
- **Source HEAD:** `8417a847cede1af06631b480188aaa55142b5989` (the Task 1 driver
  commit, which the owner checked out).
- **Host:** `teeniebox`, x86_64-linux, nix 2.35.6, `DEV_KVM=writable`.
- **Session:** 2026-11-23T15:12:45Z → 2026-11-23T15:24:06Z.
- **Summary:** `trees` passed; every other selected step failed. The plan's stop rule
  applies: gate, gate_neg and the two probe legs did not reach their assertions, so
  **no KVM verdict is claimed for HARN-01 or HARN-04**.

### Per-check verdicts

VERDICT stopb-trees: PASS
EVIDENCE stopb-trees: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/trees.log

VERDICT stopb-kernel-bootiso: FAIL
EVIDENCE stopb-kernel-bootiso: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/kernel.log

VERDICT stopb-gate-1: FAIL
EVIDENCE stopb-gate-1: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/gate.log

VERDICT stopb-gate-2: FAIL
EVIDENCE stopb-gate-2: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/gate.log

VERDICT stopb-gate-neg: FAIL
EVIDENCE stopb-gate-neg: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/gate-neg.log

VERDICT stopb-probe-flip: FAIL
EVIDENCE stopb-probe-flip: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/probe.log

VERDICT stopb-probe-flipback: FAIL
EVIDENCE stopb-probe-flipback: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/flipback.log

VERDICT stopb-profile-q35: PASS
EVIDENCE stopb-profile-q35: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log

VERDICT stopb-profile-q35lowmcfg: PASS
EVIDENCE stopb-profile-q35lowmcfg: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log

VERDICT stopb-profile-q35net: PASS
EVIDENCE stopb-profile-q35net: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log

VERDICT stopb-profile-q35exec: PASS
EVIDENCE stopb-profile-q35exec: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log

VERDICT stopb-profile-q35churn: PASS
EVIDENCE stopb-profile-q35churn: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log

VERDICT stopb-profile-q35shell: PASS
EVIDENCE stopb-profile-q35shell: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log

VERDICT stopb-profile-q35churnshell: FAIL
EVIDENCE stopb-profile-q35churnshell: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/q35churnshell.log

VERDICT stopb-profile-q35cpu: PASS
EVIDENCE stopb-profile-q35cpu: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log

VERDICT stopb-profile-q35auth: PASS
EVIDENCE stopb-profile-q35auth: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log

### Per-step roll-up (the selected R9_WITNESS_STEPS)

VERDICT trees-kvm: PASS
EVIDENCE trees-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/trees.log

VERDICT kernel-kvm: FAIL
EVIDENCE kernel-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/kernel.log

VERDICT gate-kvm: FAIL
EVIDENCE gate-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/gate.log

VERDICT gate-neg-kvm: FAIL
EVIDENCE gate-neg-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/gate-neg.log

VERDICT probe-kvm: FAIL
EVIDENCE probe-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/probe.log

VERDICT flipback-kvm: FAIL
EVIDENCE flipback-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/flipback.log

VERDICT profiles-kvm: FAIL
EVIDENCE profiles-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/profiles.log

VERDICT cpuauth-ref-kvm: FAIL
EVIDENCE cpuauth-ref-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b/cpuauth-ref.log

### Decisive lines and root causes

**trees (PASS).** Both native x86_64-linux trees built and verified:

```
TREES-STORE /nix/store/arrvrysn8l6bagwv95nbzlng8s1y7wyf-harvey-tree-amd64-0
TREES-VERIFY-OK /nix/store/arrvrysn8l6bagwv95nbzlng8s1y7wyf-harvey-tree-amd64-0
TREES-STORE /nix/store/5rq6h22qxhifg6v78q7pf4bbphc9mnjp-harvey-tree-arm64-0
TREES-VERIFY-OK /nix/store/5rq6h22qxhifg6v78q7pf4bbphc9mnjp-harvey-tree-arm64-0
```

Both stamps carry `rev=bd7a6982a3e4f465f8ed3da5e0c7290521514ff0` and the matching
`narHash`.

**gate (both runs) — moved-driver defect.** Both runs assembled `cpu.paq`, built the
ISO, and stopped before the rio assertion:

```
can't read "spawn_id": no such variable
    while executing
"expect_before -i $spawn_id {
...
    (file ".../gates/x86_64/drawterm-gate.expect" line 336)
GATE-FAIL rio-leg-failed (leg rio; ...)
GATE-RUN 1 exit=nonzero
GATE-RUN 2 exit=nonzero
```

Root cause: `gates/x86_64/drawterm-gate.expect:336` registers
`expect_before -i $spawn_id` at top level, before the qemu `spawn` at the later subtree
assigns `spawn_id`. This is a defect introduced by the 01-18 move, not a
`rio-not-serving` result. The gate never ran the rio pair or the D-28 control.

**gate_neg — wrong failure from the same defect.** The force-no-rio leg also stopped at
line 336:

```
can't read "spawn_id": no such variable
GATE-FAIL force-no-rio-leg-failed (leg force-no-rio; ...)
GATE-NEG-WRONG-FAILURE (GATE-FAIL rio-not-serving absent)
```

The failing direction was not exercised; the marker required by HARN-01 is absent.

**probe — probe build OK, gate unwitnessed.** The flip leg cloned at
`PROBE-FLIP-SHA` and rebuilt the a.out, then hit the same driver line:

```
probe: 96 flat bytes -> 136 a.out bytes at .../r9-probe-flip/target/probe/x86_64/ps.aout
gate: image cpu .../r9-probe-flip/target/images/x86_64/cpu.paq ... members 4442
can't read "spawn_id": no such variable
GATE-FAIL rio-leg-failed (leg rio; ...)
```

The probe artifact and the 4442-member image assembly succeeded; the serial
`/bin/ps=R9-RUST-AOUT-PROBE-OK` check never ran.

**flipback — assembly OK, gate unwitnessed.** The flip-back clone assembled the
4442-member cpu image and hit the same driver line:

```
gate: image cpu .../r9-probe-flipback/target/images/x86_64/cpu.paq ... members 4442
can't read "spawn_id": no such variable
GATE-FAIL rio-leg-failed (leg rio; ...)
```

**profiles — 8/9 pass, q35churnshell fails.** `profiles.log` records eight exits at 0
and one non-zero. The q35churnshell serial tail:

```
process 2 exited: ""
churngate: heap exit 96512 33554432
syscall 10 unimplemented
boot-test: >= 3 lines beginning "tick" not observed (negative scan hit "unimplemented syscall")
```

This is a separate kernel/churn-gate defect. The other eight profiles (q35,
q35lowmcfg, q35net, q35exec, q35churn, q35shell, q35cpu, q35auth) passed under KVM.

**kernel — boot-iso check did not run.** The kernel package built:

```
KERNEL-STORE /nix/store/2279qlywyhk9zfya9qjaqn004vj95jl6-r9-kernel-x64-8417a84
3535848bdab363f226360c172caf95dc699a493e2c7a9ebf6af68ad10cb77ad4  .../kernel-x64.elf
```

Then `--rebuild` refused because the derivation had never been built or substituted:

```
error: some outputs of '...-r9-boot-iso-8417a84.drv' are not valid, so checking is not possible
       Hint: --rebuild and --check error if the derivation was not previously built and cannot be substituted.
KERNEL-BOOTISO-REBUILD-FAIL
```

The fixed-output kernel-byte check did not run. The recorded package ELF sha256
`3535848b…` differs from the local-build 01-08 baseline `50b01c9a…`; 01-21 recorded the
same package-vs-local difference for aarch64 (the sandbox's `--remap-path-prefix`), so
no kernel-byte equality is claimed either way from this run.

**cpuauth_ref — both compares failed on the lazy reference path.** The authdomain
generator ran and paired for each image, and both images assembled (cpu 4442, auth 95
members), but the reference directory has no member lists:

```
image: authdomain pairing verified over the staged copies for "cpu"
image: creds .../cpu.creds (mode 0600)
reference member list /nix/store/mxx5qyijchxaff7cg61y4nwvjvjbbmi4-harvey-reference-amd64-0/cpu.members: No such file or directory (os error 2)
CPUAUTH-REF cpu exit=nonzero
image: x86_64/auth ... members 95
reference member list /nix/store/mxx5qyijchxaff7cg61y4nwvjvjbbmi4-harvey-reference-amd64-0/auth.members: No such file or directory (os error 2)
CPUAUTH-REF auth exit=nonzero
```

The step did not realize `harvey-reference-amd64` before reading it; the devshell
export is a lazy context-discarded path (01-15/A9). Both compares were reached; both
failed on the missing member list.

### HARN status

- **HARN-01** — not witnessed: gate and gate_neg failed in the driver before the rio
  pair and before the failing-direction assertion.
- **HARN-04** — not witnessed: both probe legs failed in the driver before the serial
  `ps` check; the probe build and both image assemblies themselves completed.
- **HARN-06 (x86_64 half)** — 8 of 9 profiles passed; q35churnshell failed on
  `syscall 10 unimplemented`, a separate kernel/churn-gate defect.
- **HARN-07** — both trees built natively and store-verified; the boot-iso fixed-output
  check did not run.

No later plan (01-23 to 01-28, 01-31) was started.
