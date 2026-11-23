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

## Stop B2 — plan 01-33, x86_64 KVM

> **Superseded:** the gate, probe, flipback and kernel FAILs below were resolved
> by stop B3 (see "Stop B3" at the end). This section is kept as history; the
> current verdicts are the stop-B3 ones.

- **Return:** `witness-src/01-33` at `9ec1d7c2aac92735cb5c74644802130172c07e48`;
  return directory `witness/return-01-33/` in that commit (the trimmed evidence is
  copied under `evidence/stop-b2/`; the return branch is not merged).
- **Source HEAD:** `9af05f8b71e57a422c36b3a23e168399bb1c88b6` (the Task 1/2 fix
  commit this plan produced).
- **Host:** `teeniebox`, x86_64-linux, nix 2.35.6, `DEV_KVM=writable`.
- **Session:** 2026-11-23T16:17:30Z → 2026-11-23T16:29:51Z.
- **Summary:** `gate_neg`, `profiles` and `cpuauth_ref` PASS. `gate`, `probe`,
  `flipback` and `kernel` are non-zero (7 selected steps: 3 PASS, 4 non-zero).
  The gate's positive rio leg itself passed every stage in both runs; the probe
  and flipback failures are a DIFFERENT defect from the gate teardown (their
  historical clones carry the pre-01-33 driver). The kernel step now reaches the
  boot-iso fixed-output check and finds a hash mismatch. No KVM verdict is claimed
  for HARN-01/HARN-04 in this pass; the stop-B requirement stays open and no later
  plan started.

### Per-step verdicts

VERDICT gate-kvm: FAIL
EVIDENCE gate-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/gate.log

VERDICT gate-neg-kvm: PASS
EVIDENCE gate-neg-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/gate-neg.log

VERDICT probe-kvm: FAIL
EVIDENCE probe-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/probe.log

VERDICT flipback-kvm: FAIL
EVIDENCE flipback-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/flipback.log

VERDICT profiles-kvm: PASS
EVIDENCE profiles-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/profiles.log

VERDICT cpuauth-ref-kvm: PASS
EVIDENCE cpuauth-ref-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/cpuauth-ref.log

VERDICT kernel-kvm: FAIL
EVIDENCE kernel-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/kernel.log

### Decisive lines and root causes

**gate (FAIL) — teardown defect, assertions green.** Both runs printed the D-28
pair and `GATE-PASS rio-serving`, then all three post-pass ok lines:

```
/srv/rio.glenda.74
/srv/riowctl.glenda.74
% GATE-PASS rio-serving
drawterm-gate: post-pass ok — zero cpurc: FAIL in the capture
drawterm-gate: post-pass ok — zero kernel panic markers (drawterm's own sysfatal exit pair excluded)
drawterm-gate: post-pass ok — the per-bake DEMO password redacted from the retained capture (T-01-58)
GATE-FAIL rio-leg-failed (leg rio; capture .../154850-1795450687/rio.log)
GATE-RUN 1 exit=nonzero
```

No `ALL STAGES PASS` line appears in either run (grep count 0). The final
`cleanup`'s identity sweep ran `pgrep -f 'qemu-system|Xvfb|drawterm'`: the
driver's own argv (`drawterm-gate.expect`) and the run tag in its environment
matched, so the sweep SIGTERMed the expect process before the PASS print. Fixed
in this plan: the sweep excludes its own caller chain, cleanup guards every pid
read and catches the sweep/redaction, and the final success cleanup is caught
(a teardown warning cannot mask the verdict). A targeted rerun is pending.

**gate_neg (PASS).** The failing direction reached its assertion:

```
drawterm-gate: GATE-FAIL rio-not-serving — no rio.<user>.<pid> + riowctl.<user>.<pid> pair ...
GATE-NEG-EXPECTED-FAIL
```

**probe (FAIL) — historical clone's driver, not the teardown.** The probe build
and image assembly succeeded, then the clone at `PROBE_FLIP_SHA` (064747f4)
aborted in its own pre-01-33 driver:

```
probe: 96 flat bytes -> 136 a.out bytes .../ps.aout
GATE-FAIL ... (leg rio ...)
can't read "spawn_id": no such variable
"expect_before -i $spawn_id { ... (file .../gates/x86_64/drawterm-gate.expect line 336)
```

The witness step now overlays this checkout's fixed driver into the throwaway
clone; the image and the swapped `ps` stay at the exact SHA. A targeted rerun is
pending.

**flipback (FAIL) — same historical-clone driver defect.** The flip-back clone
assembled the 4442-member cpu image, then hit the same line-336 abort at
`PROBE_FLIPBACK_SHA` (af7df344). The fixed-driver overlay applies here too; a
targeted rerun is pending.

**profiles (PASS).** All nine profiles exited 0:

```
PROFILE q35 exit=0
PROFILE q35lowmcfg exit=0
PROFILE q35net exit=0
PROFILE q35exec exit=0
PROFILE q35churn exit=0
PROFILE q35shell exit=0
PROFILE q35churnshell exit=0
PROFILE q35cpu exit=0
PROFILE q35auth exit=0
```

`q35churnshell` PASS is the 01-33 registration fix working under KVM (the
real-chain fauth line exempted; the chain stages asserted through the `% `
prompt).

**cpuauth_ref (PASS).** The realised reference path was used for both compares:

```
CPUAUTH-REF-REALISED /nix/store/mxx5qyijchxaff7cg61y4nwvjvjbbmi4-harvey-reference-amd64-0
image: x86_64/cpu ... members 4442
image: x86_64/cpu matches the reference member list (4442 members)
CPUAUTH-REF cpu exit=0
image: x86_64/auth ... members 95
image: x86_64/auth matches the reference member list (95 members)
CPUAUTH-REF auth exit=0
```

**kernel (FAIL) — stale boot-iso fixed-output hash.** The plain build now runs
(the 01-33 precondition fix worked), and the fixed-output check found a mismatch:

```
KERNEL-STORE /nix/store/imb1n1k67mpiv0v2cn0wr0vj8nw1pcc0-r9-kernel-x64-9af05f8
4228a1bf1408a7f06d942fa847b6ca25d7bfbd5a400ce9190b602d3b365f32e1  .../kernel-x64.elf
error: hash mismatch in fixed-output derivation '...-r9-boot-iso-9af05f8.drv':
         specified: sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=
            got:    sha256-xFt7GUshPv+CQNry48fBOAiCsCIuqGZfeC0eX11/8mc=
KERNEL-BOOTISO-BUILD-FAIL
```

This is the long-owed Linux boot-iso kernel-byte check, not a harness defect.
No option is chosen here; the decision is the owner's (below).

### HARN status (stop B2)

- **HARN-01** — not witnessed: gate is non-zero on its teardown (assertions
  green; fix pending rerun); gate_neg PASSED.
- **HARN-04** — not witnessed: probe/flipback aborted in their historical
  clones' pre-01-33 driver (overlay fix pending rerun).
- **HARN-06 (x86_64 half)** — PASS: 9/9 profiles, `q35churnshell` included.
- **HARN-07** — both trees remained green at stop B; the boot-iso fixed-output
  check is still red on the stale x86_64 hash.

No later plan (01-26 and later) was started.

### Boot-iso decision (owner, pending)

The fixed-output hash must be resolved before the kernel step can pass; no option
is chosen by this plan:

- **(a)** Re-derive and commit the x86_64 `boot-iso` outputHash from the owner's
  x86_64-linux host (the standing kernel-byte-change procedure), then rerun the
  kernel step.
- **(b)** Investigate the cross-builder reproducibility difference first.
- **(c)** Re-scope the Linux boot-iso fixed-output check.

## Stop B3 — plan 01-33, x86_64 KVM

- **Return:** `witness-src/01-33` at `02b1b5d5b126901969b83cb47f7dcdc77a1b9831`;
  return directory `witness/return-01-33/` in that commit (trimmed evidence copied
  under `evidence/stop-b3/`).
- **Source HEAD:** `4139fdf86ef3cd3f2455c5b1359e80c27d48d098` (the boot-iso
  outputHash re-derivation commit).
- **Host:** `teeniebox`, x86_64-linux, nix 2.35.6, `DEV_KVM=writable`.
- **Session:** 2026-11-23T16:44:41Z → 2026-11-23T17:00:16Z.
- **Summary:** every selected step PASS (kernel, gate, gate_neg, probe, flipback).
  `profiles` (9/9, q35churnshell included) and `cpuauth_ref` (cpu 4442, auth 95)
  are carried from stop B2. The only outstanding check is the amd64 TCG gate on
  the owner's x86_64 host (PENDING below).

### Per-step verdicts

VERDICT gate-kvm: PASS
EVIDENCE gate-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/gate.log

VERDICT gate-neg-kvm: PASS
EVIDENCE gate-neg-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/gate-neg.log

VERDICT probe-kvm: PASS
EVIDENCE probe-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/probe.log

VERDICT flipback-kvm: PASS
EVIDENCE flipback-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/flipback.log

VERDICT kernel-kvm: PASS
EVIDENCE kernel-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/kernel.log

VERDICT profiles-kvm: PASS
EVIDENCE profiles-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/profiles.log

VERDICT cpuauth-ref-kvm: PASS
EVIDENCE cpuauth-ref-kvm: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b2/cpuauth-ref.log

### Decisive lines

**gate (PASS).** Both runs exited 0 and BOTH legs printed `ALL STAGES PASS`:

```
% GATE-PASS rio-serving
drawterm-gate: ALL STAGES PASS — cpu boot to cpurc: DONE, encrypted drawterm session, serial /srv shows rio + riowctl (GATE-PASS rio-serving)
% GATE-PASS control-no-rio
drawterm-gate: ALL STAGES PASS — cpu boot to cpurc: DONE, -G control session, serial /srv shows neither rio entry (GATE-PASS control-no-rio)
GATE-RUN 1 exit=0
...
GATE-RUN 2 exit=0
```

**gate_neg (PASS).** `GATE-FAIL rio-not-serving` then `GATE-NEG-EXPECTED-FAIL`.

**probe (PASS).** The probe build produced the 136-byte a.out and the gate's
serial check observed the marker after `/bin/ps`; both legs printed ALL STAGES PASS:

```
probe: 96 flat bytes -> 136 a.out bytes .../ps.aout
% GATE-PASS rio-serving
/bin/ps
R9-RUST-AOUT-PROBE-OK
drawterm-gate: stage ok — serial-check 'R9-RUST-AOUT-PROBE-OK' present after '/bin/ps'
drawterm-gate: ALL STAGES PASS — ... rio-serving
```

**flipback (PASS).** The flip-back clone assembled the 4442-member cpu image;
both legs printed `ALL STAGES PASS` and the wrapper exited 0.

**kernel (PASS).** The re-derived boot-iso hash now matches and `--rebuild`
succeeds:

```
KERNEL-STORE /nix/store/y56rpqda7cpglr41yyr0h5qqrm20ra8y-r9-kernel-x64-4139fdf
4228a1bf1408a7f06d942fa847b6ca25d7bfbd5a400ce9190b602d3b365f32e1  .../kernel-x64.elf
KERNEL-BOOTISO-STORE /nix/store/bx1bbayxf2y99xysqcn7p1kxzjrkifwb-r9-boot-iso-4139fdf
KERNEL-BOOTISO-BUILD-OK
KERNEL-BOOTISO-REBUILD-OK
```

**Boot-iso hash re-derivation.** Commit `4139fdf` re-derived the x86_64
`boot-iso` outputHash from the owner's x86_64-linux host (option (a), the
standing kernel-byte-change procedure). The stop-B2 mismatch
(`specified TIzw…` vs `got xFt7…`) is resolved: the committed hash now names
`sha256-xFt7GUshPv+CQNry48fBOAiCsCIuqGZfeC0eX11/8mc=`, and the kernel ELF sha
stays `4228a1bf1408a7f06d942fa847b6ca25d7bfbd5a400ce9190b602d3b365f32e1`.

### PENDING

PENDING gate-tcg: the amd64 TCG gate (`cargo xtask gate --arch x86-64 --accel
tcg` on the owner's x86_64 host) has been requested; no PASS/FAIL line has been
reported. It is NOT claimed as PASS.

### HARN status (stop B3)

- **HARN-01** — PASS: gate green twice (rio + control legs, ALL STAGES PASS) and
  gate_neg reached the expected failing direction.
- **HARN-04** — PASS: the probe's serial `/bin/ps` marker observed, and the
  flipback leg green.
- **HARN-06 (x86_64 half)** — PASS: 9/9 profiles, q35churnshell included.
- **HARN-07** — PASS: both trees green at stop B and the boot-iso fixed-output
  check now builds and re-checks (`KERNEL-BOOTISO-REBUILD-OK`).

The amd64 TCG gate remains the one pending check before phase close-out; no
later plan (01-26 and later) was started.


