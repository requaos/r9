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

### gate-tcg

VERDICT gate-tcg: PASS
EVIDENCE gate-tcg: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-b3/gate-tcg.txt
The owner ran `cargo xtask gate --arch x86-64 --accel tcg` on the x86_64 host and
reported `GATE-PASS control-no-rio`. The control leg runs only after the rio leg,
so the TCG gate completed green; the amd64 TCG check in the standing kernel-byte
procedure is satisfied.

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

## Stop C — plan 01-27, x86_64 KVM

STATUS stopc: FAILED — return `witness-src/01-27` at
`6346c68cd67264f6a58fd72369c0dc8523cc9ffe`; the owner did not approve the
Harvey diff and no Harvey commit was made.

- **Return:** `witness-src/01-27` at `6346c68…`; return directory
  `witness/return-01-27/`; source HEAD `3f717f7…`; host `teeniebox`
  (x86_64-linux, nproc 16, 30 GiB RAM, nix 2.35.6, `DEV_KVM=writable`);
  session 2026-11-23T17:37:29Z → 18:32:12Z.
- **Summary:** 7/8 full apps PASS; `boot-r9-tier1-x64` FAIL (`T1-FAIL
  fpprobe`); `soak` and `stress` FAIL on the tap precondition (the `HARVEY`
  seam was not set); `equiv` FAIL (six app captures absent from the full
  runner's copy, plus `boot-r9-x64` markers 0/1); `store_verify` PASS.
- **Harvey:** NOT APPROVED — no change, staging or commit was made; HEAD is
  still `25d97dc4d…` and the status hash still `041d2929…`.

### Per-step verdicts

VERDICT stopc-full: FAIL
EVIDENCE stopc-full: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/full.log
EVIDENCE stopc-full: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/full.tsv

VERDICT stopc-app-drawterm-gate-x64: PASS
VERDICT stopc-app-boot-r9-x64: PASS
VERDICT stopc-app-boot-r9-smoke-x64: PASS
VERDICT stopc-app-boot-r9-tier1-x64: FAIL
VERDICT stopc-app-boot-r9-net-smoke-x64: PASS
VERDICT stopc-app-boot-r9-services-x64: PASS
VERDICT stopc-app-boot-r9-churn-x64: PASS
VERDICT stopc-app-boot-r9-auth-x64: PASS
EVIDENCE stopc-apps: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/full.tsv

VERDICT stopc-soak: FAIL
EVIDENCE stopc-soak: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/soak.log

VERDICT stopc-stress: FAIL
EVIDENCE stopc-stress: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/stress.log

VERDICT stopc-equiv: FAIL
EVIDENCE stopc-equiv: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/equiv/report.tsv
EVIDENCE stopc-equiv: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/equiv/rows.tsv

VERDICT stopc-store-verify: PASS
EVIDENCE stopc-store-verify: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/store_verify.log

EVIDENCE stopc-env: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/env.txt
EVIDENCE stopc-summary: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c/summary.tsv

### Why stop C failed (decisive lines)

**`full` — 7/8, `boot-r9-tier1-x64` FAIL.** `full.tsv` records PASS for
`drawterm-gate-x64`, `boot-r9-x64`, `boot-r9-smoke-x64`,
`boot-r9-net-smoke-x64`, `boot-r9-services-x64`, `boot-r9-churn-x64` and
`boot-r9-auth-x64`, and FAIL for `boot-r9-tier1-x64`. The tier-1 driver's
post-pass names the missing marker; the serial capture shows the FPU note
leg never delivered its handlers:

```
fpprobe: BAD note-spin handler ran 0 times
fpprobe: BAD note-regs handler ran 0 times
fpprobe: BAD note child 129: bad
user trap: unhandled vector 1 pc=0x2027d9 addr=0x0
fpprobe: FAIL note
T1-FAIL fpprobe
T1-DONE
tier1-r9-x64: post-pass FAILED — missing T1-OK fpprobe
GATE-FAIL full-run — apps failed: boot-r9-tier1-x64 (results: target/gate/347497-1795455456/full.tsv)
```

The six `FULL-NO-CAPTURE` lines (`boot-r9-smoke-x64`, `boot-r9-tier1-x64`,
`boot-r9-net-smoke-x64`, `boot-r9-services-x64`, `boot-r9-churn-x64`,
`boot-r9-auth-x64`) are a **capture-copy gap in the new full runner**, not gate
failures: those apps write to `target/gates/<short-name>/` (`smoke-r9-x64`,
`tier1-r9-x64`, `smoke-r9-net-x64`, `services-r9-x64`, `churn-r9-x64`,
`smoke-r9-auth-x64`) while `step_full` copies `target/gates/<app-name>/`.
`drawterm-gate-x64` and `boot-r9-x64` are the two names where the two agree;
only the drawterm capture contains a `.log`, which is why the copy loop did not
name `boot-r9-x64` but the equivalence count still saw an empty directory.

**`soak` / `stress` — precondition, not a measured venue.** Both stopped in
2 s / 0 s before any bake or boot:

```
error: tap mode needs the netaddr seam: /home/req/code/r9/build/x86_64/root-tree/lib/netaddr.rc missing — run from the harvey repo root or set HARVEY (the tap checks key on its tap_gwv4/blastport)
```

`step_soak`/`step_stress` ran `nix run .#net-doctor -- --mode tap` from the r9
root with no `HARVEY`, so net-doctor defaulted to `$PWD` and looked for the
seam at `<r9>/build/…`. The tap venue itself was never exercised. The two
flake apps already export the correct seam (`HARVEY_NETADDR_SEAM=…/harvey-tree-amd64/build/x86_64/root-tree/lib/netaddr.rc`,
flake.nix:1405/1418), so the defect is the witness step's doctor invocation,
not the gates.

**`equiv` — capture gaps plus `boot-r9-x64` 0/1.** `report.tsv` has
`drawterm-gate-x64 match`; every other row is `differ`. The `-1` rows are the
six capture-copy gaps above. `boot-r9-x64` is `0/1` for
`q35: paqfs root mounted from module`, `init: starting /bin/rc` and `^% `;
note that those exact markers **do** appear in the full-run log (lines 552,
565, 585: `q35: paqfs root mounted from module (389600 bytes)`,
`init: starting /bin/rc`, `% `), while the terminal-mode boot app writes no
`target/gates/boot-r9-x64/*.log` for `count_in_dir` to read. Treat this row as
a capture-path defect to confirm before calling it a boot regression.

**`store_verify` — PASS.** Both tree paths verify:
`/nix/store/arrvrysn8l6bagwv95nbzlng8s1y7wyf-harvey-tree-amd64-0` and
`/nix/store/5rq6h22qxhifg6v78q7pf4bbphc9mnjp-harvey-tree-arm64-0`.

### What a green rerun needs

1. `step_soak`/`step_stress` must point net-doctor's `HARVEY` at the r9-built
   amd64 tree (the store path of `.#harvey-tree-amd64`) so the tap preflight
   reads the real seam; the soak/stress apps already carry
   `HARVEY_NETADDR_SEAM` themselves.
2. `step_full` must copy each app's actual `R9_GATE_OUT` directory (the
   short-name map above), and `step_equiv` must count the same path.
3. The tier-1 `fpprobe` note-leg failure is investigated and fixed below
   (the q35 millisecond/tick conversion); the KVM stop-C2 rerun is the
   proof that the real handlers now run.

### fpprobe note-delivery — root cause, fix and next action (plan 01-34)

**Finding: the interrupt-return note boundary is intact; the q35 port
interpreted alarm/sleep MILLISECONDS as 100 Hz timer TICKS, so every
alarm arrived ten times late.** `fpprobe`'s two ring-3 spin rounds
(`note-spin`: 200M volatile adds; `note-regs`: `gprspin`'s 2^31 asm
decrements) finish before that late alarm on a fast KVM host, so their
handlers ran 0 times; the same rounds pass under the far slower TCG
container where the 500 ms alarm always lands mid-spin.

Evidence, and why it is this and not a missing boundary:

- The failing lines are exactly the two legs that need delivery on the
  INTERRUPT return:
  `fpprobe: BAD note-spin handler ran 0 times`,
  `fpprobe: BAD note-regs handler ran 0 times`, then
  `fpprobe: BAD note child 129: bad`, `fpprobe: FAIL note`,
  `T1-FAIL fpprobe` (full.log). `note-sync`, `note-sleep` and
  `note-nsave` did not report BAD — those deliver at the SYSCALL return
  (and `note-sleep` additionally proves the tick's timeout sweep posts
  the alarm note and wakes the Blocked sleeper).
- The `switch` and `preempt` legs passed: `preempt` runs the same
  ring-3 integer spin for four rounds and requires the timer tick to
  preempt it, so ticks fire and the IRQ path runs under KVM.
- `user trap: unhandled vector 1 pc=0x2027d9 addr=0x0`, followed by
  `process 138 exited: "sys: trap"`, is the flags-`tf` sub-leg's
  DESIGNED outcome: TF is set across a syscall and the single-step #DB
  must arrive in ring 3 and kill the child with `sys: trap` (fpprobe
  requires exactly that string). It is not a note-delivery fault.
- `x86_64/src/trap.rs`'s IRQ-return
  `deliver_notes(trap_frame)` call (vectors >= 32, `cs & 3 == 3`) has
  been present since `0ffcec6` (the 2026-11-22 x86_64 async-note fix,
  part of `e9e5365`); no later commit touched that block. The same code
  was verified green under TCG (`fpprobe: ok`, tier-1 39 T1-OK) on
  2026-11-22. The prior session's "no IRQ-return delivery" mutant
  produced these SAME two lines, which is why the serial evidence alone
  was ambiguous; the code and venue timeline disambiguate it.
- The defect: `TICKS_PER_SEC = 100` on q35 (`TICK_MS = 10`), while
  `sys_alarm`/`sys_sleep` inherited aarch64's `now + ms` — harmless
  there because the aarch64 tick is 1 ms, so a millisecond and a tick
  are the same number; ten times late here. `alarm(50)` armed at
  500 ms, and `alarm`'s "remaining ms" return was ten times small.
  `gprspin` is sub-second on the witness's KVM host (`-cpu host`, 16
  cores) and loses the race; under TCG it runs for many seconds and
  wins. That is why stop C's KVM host is the first venue to see it.
- WINDOWS #70 (aarch64 scheduler resume skips note delivery, deferred
  to Phase 8/SMP-06) is a DIFFERENT defect on the other architecture;
  the x86_64 async path was fixed in `e9e5365` and is present here.
  Neither the arm64 deferral nor its acceptance covers this timing
  bug, and nothing here re-scopes it.

**Fix (in scope, small):** `x86_64/src/syscall.rs` converts
milliseconds to ticks with a ceiling (`ms_to_ticks`, `TICK_MS = 10`)
for `sleep` and `alarm`, and converts `alarm`'s remaining-tick return
back to milliseconds; `x86_64/src/platform/q35/timer.rs` exposes
`TICK_MS` `pub(crate)`. A host unit test pins the conversion
(`sleep_and_alarm_convert_milliseconds_to_the_100hz_slice`: 1..9 ms = 1
tick, 10 = 1, 11 = 2, 50 = 5, 1000 = 100). The aarch64 twin is
unchanged (identity conversion there by construction).

**Verification:** the conversion is pinned at COMPILE TIME in every
x86_64 kernel build — `const` asserts beside `ms_to_ticks` are carried
by `cargo xtask check`, `clippy` and `build`, so a tick-rate or formula
change cannot silently restore the ten-times-late alarm — and repeated
by a host unit test on the x86_64 leg
(`sleep_and_alarm_convert_milliseconds_to_the_100hz_slice`).
`cargo xtask test|clippy|check` pass on the Mac (the aarch64 host leg);
the x86_64 host-test leg needs an x86_64 Linux host or an amd64
container (a Docker attempt here is blocked by Docker's seccomp policy,
`unable to load seccomp BPF`, so the owner's host carries that leg).
The boot-level proof is stop C2: `boot-r9-tier1-x64` must print
`fpprobe: ok` with all six legs, `note-spin` and `note-regs` included.
Nothing about the gate changes: `T1-OK fpprobe` stays a required marker
and stop C remains FAIL until that rerun is green.

A TCG rerun **cannot discriminate this finding**: the pre-fix kernel was
already green under TCG on 2026-11-22 (the same two rounds passed), and
it will stay green after the fix — TCG's spin loops outlast the 500 ms
late alarm, so the race never fires there. The KVM venue is the
discriminator, and stop C2 runs there; spending a TCG cycle would
re-measure the venue that never exposed the bug. (The x86_64 gate does
run under the amd64 Docker + `nix develop` shape, so the command is
available for any future TCG need; it is recorded below.)

**If stop C2 still reports `handler ran 0 times`:** the next action is
the trap.rs IRQ-boundary isolation (the 2026-11-22 no-IRQ-delivery
mutant) — instrument `deliver_notes` at the vector>=32 return to print
`pid`, `note_queue.len()` and `notified` per tick — not a gate
reclassification. `note-sleep` passing while `note-spin`/`note-regs`
fail would then implicate the tick's `preempt()` park/resume path
(the proc resumes through `swtch` after the boundary ran), which is the
only code between the tick and the boundary; the fix stays in the note
path, never in the acceptance.

**TCG-only reproduction command** (an x86_64 Linux host or the amd64
container; the Mac has no x86_64 devshell):

    nix develop -c env R9_ACCEL=tcg cargo xtask gate --arch x86-64 --full --accel tcg

or the single app through the flake: `R9_ACCEL=tcg nix run
.#boot-r9-tier1-x64` (x86_64-linux only; the app exports its own
supply). The KVM rerun (stop C2) is the authoritative proof.

**Stop-C2 result: CONFIRMED.** The KVM rerun printed `fpprobe: ok` and
`T1-OK fpprobe`; the tier-1 post-pass reports all 39 expected T1-OK
present, zero T1-FAIL, zero panic (evidence:
`evidence/stop-c2/full.log`). The two ring-3 note rounds now deliver.

## Stop C2 — plan 01-34, x86_64 KVM

STATUS stopc2: HALTED — return `witness-src/01-34` at
`2bc54786da3895e4c30f8ec8e7141b4ccd99b99f` on the fixed tree
(`2a5cbed`). The moved gates themselves are green: the full set is 8/8
including `fpprobe`, stress PASSes on the tap venue, and both trees
verify. The run is not fully green: the `full` step exited 1 on a
capture-check bug, `soak` refused slirp, `equiv` compares against stale
baselines, and the `kernel` step has not verified the new boot-iso hash.

- **Return:** `witness-src/01-34` at `2bc5478`; return directory
  `witness/return-01-34/`; source HEAD `2a5cbed`; host `teeniebox`
  (x86_64-linux, nproc 16, 30 GiB RAM, nix 2.35.6, `DEV_KVM=writable`);
  session 2026-11-23T19:22:04Z → 19:31:19Z; survivors none; credential
  scan clean.
- **Steps selected:** `full soak stress equiv store_verify` (the owner
  did not select `kernel`).
- **Harvey:** NOT APPROVED — no change, staging or commit was made; the
  prepared diff remains unapplied.

### Per-step verdicts

VERDICT stopc2-full: PASS
EVIDENCE stopc2-full: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/full.tsv
EVIDENCE stopc2-full: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/full.log

VERDICT stopc2-soak: FAIL
EVIDENCE stopc2-soak: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/soak.log

VERDICT stopc2-stress: PASS
EVIDENCE stopc2-stress: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/stress.log

VERDICT stopc2-equiv: FAIL
EVIDENCE stopc2-equiv: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/equiv/report.tsv
EVIDENCE stopc2-equiv: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/equiv/rows.tsv

VERDICT stopc2-store-verify: PASS
EVIDENCE stopc2-store-verify: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/store-verify.log

VERDICT stopc2-kernel: PENDING
EVIDENCE stopc2-kernel: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/kernel.log

EVIDENCE stopc2-env: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/env.txt
EVIDENCE stopc2-summary: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/summary.tsv
EVIDENCE stopc2-secret-scan: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/secret-scan.txt
EVIDENCE stopc2-survivors: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c2/survivors.txt

### What the run proves

- **full — 8/8, `GATE PASS full x86_64-linux run=405781-1795461728`.**
  `boot-r9-tier1-x64` PASS: `fpprobe: ok`, `T1-OK fpprobe`, every
  expected T1-OK present (39 names), zero T1-FAIL. The fpprobe
  ms→tick finding is confirmed on the KVM venue.
- **The 01-34 capture fixes worked.** Every app's real `R9_GATE_OUT`
  directory was copied (no `-1` from the full set), and
  `boot-r9-x64` now has `boot-r9.log`: its three markers count 1/1.
  `drawterm-gate-x64`, `boot-r9-smoke-x64`, `boot-r9-net-smoke-x64`,
  `boot-r9-services-x64` and `boot-r9-x64` all `match`.
- **stress — PASS on the tap venue.** `netstress: venue tap`,
  `tcpblast: done x 9`, zero `netstress: FAIL`, ALL STAGES PASS.
- **store_verify — PASS** on both tree paths.
- **The tap preflight fix worked:** the doctor read
  `…/harvey-tree-amd64-0/build/x86_64/root-tree/lib/netaddr.rc` and
  PASSed all eight checks.

### Why the run is halted (and the fixes committed after the return)

1. **`full` step exit 1 was the capture check, not a gate.** The step
   printed `GATE PASS full` and then `FULL-NO-CAPTURE
   boot-r9-net-soak-x64` / `boot-r9-stress-x64`: the copy loop walked
   the whole app→dir map, which carries the two tap-venue rows that
   only the separate soak/stress steps produce. Fixed: `step_full`
   iterates the eight `FULL_APPS` and resolves each dir through
   `app_out_dir`; a static test
   (`full_capture_loop_covers_only_the_full_tier`) pins it.
2. **`soak` refused slirp.** The doctor PASSed, then the app exited in
   the venue-honesty check: `slirp evidence not accepted … want
   --net=tap / R9_NET=tap`. `venue_gate` exported `HARVEY` and the seam
   but not `R9_NET`. Fixed: both the doctor preflight and the app run
   now carry `R9_NET=tap` (the stress app's default happened to be tap;
   it is explicit now too).
3. **`equiv` compared against stale baselines.** No `-1` rows remain
   from the capture gap; the four `differ` rows are expectation drift:
   `T1-OK 39/36`, `AUTH-OK 10/5`, `CATS-DONE 1400 1/2` and the tap
   run's absent `RACE-STRESS slirp venue` label (0/1). `equivalence.tsv`
   now records the stop-C2 expectations with a per-row rationale and a
   `recorded` source token; the comparator treats `recorded` as an
   owner-approved expectation (no fresh baseline, no stale capture
   count). The gates themselves are unchanged.
4. **`kernel` is PENDING.** The return's `kernel.log` is the stale
   4139fdf package. The branch's `flake.nix` (commit `2bc5478`) carries
   the new boot-iso `outputHash`
   `sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=`, but no
   `kernel` step has built/`--rebuild`-checked it.

### Remaining owner command

On the x86_64 host, at the fixed commit, the four unproven/red steps
only (the full and stress captures are already green — no full rerun):

    R9_WITNESS_STEPS="soak equiv kernel store_verify" \
      bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh

and the amd64 TCG leg:

    nix develop -c cargo xtask gate --arch x86-64 --accel tcg

Expected: `soak` PASS on the tap venue; `equiv` all rows `match`;
`kernel` `KERNEL-BOOTISO-BUILD-OK` + `KERNEL-BOOTISO-REBUILD-OK` on the
new hash; `store_verify` PASS. The Harvey diff remained unapplied until
the green C3 run and the explicit APPROVE — both are now recorded in the
Stop C3 section.

**Resolved by stop C3 — all four were green.** See the Stop C3 section
below.

## Stop C3 — plan 01-34, x86_64 KVM

STATUS stopc3: PASS for the four selected steps. With stop C2's full
8/8 and tap stress, every witness step is now green, and the three
owner items that followed are resolved: the standalone amd64 TCG gate
is `GATE-PASS` (owner report), the HARN-08 equivalence expectations are
APPROVED (2026-11-23 owner), and the Harvey diff is APPROVED, applied
and PUSHED as Harvey `c697666ba` on `origin/master` (owner push
2026-11-23, verified). The diff is no longer unapplied-awaiting-approval.

- **Return:** `witness-src/01-34` at
  `d6106dbfcdbfa16be02a14b1d40f95adcbc2226e` on our halted-summary tip
  (`678a88c`); return directory `witness/return-01-34/`; source HEAD
  `678a88c`; host `teeniebox` (x86_64-linux, nproc 16, 30 GiB RAM, nix
  2.35.6, `DEV_KVM=writable`); session 2026-11-23T19:39:17Z →
  19:44:52Z; survivors none; credential scan clean.
- **Steps selected:** `soak kernel equiv store_verify` (the four red/
  unproven C2 steps; no full rerun).
- **Harvey:** APPROVED 2026-11-23 by the owner and committed as
  `c697666ba315f137ca2fd11768f835d04bd9f018`; not pushed.

### Per-step verdicts

VERDICT stopc3-soak: PASS
EVIDENCE stopc3-soak: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/soak.log

VERDICT stopc3-equiv: PASS
EVIDENCE stopc3-equiv: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/equiv/report.tsv
EVIDENCE stopc3-equiv: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/equiv/rows.tsv

VERDICT stopc3-kernel: PASS
EVIDENCE stopc3-kernel: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/kernel.log

VERDICT stopc3-store-verify: PASS
EVIDENCE stopc3-store-verify: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/store-verify.log

VERDICT stopc3-tcg: PASS
EVIDENCE stopc3-tcg: owner report 2026-11-23 — standalone `nix develop -c cargo xtask gate --arch x86-64 --accel tcg` on the x86_64 host printed `GATE-PASS` (no log file committed; the GATE-PASS control-no-rio lines in the full/stress captures are the drawterm legs, not this run)

EVIDENCE stopc3-env: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/env.txt
EVIDENCE stopc3-summary: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-c3/summary.tsv

### The combined witness set is green

- **full — 8/8** (stop C2, carried): `GATE PASS full`, `fpprobe: ok`,
  `T1-OK fpprobe`, 39 expected T1-OK, zero T1-FAIL.
- **stress — PASS** (stop C2, carried): tap venue label, 9 tcpblast,
  zero FAIL.
- **store_verify — PASS** (stop C2 carried and C3 repeated): both trees
  `STORE-VERIFY-OK`.
- **soak — PASS**: `netip: L2-TRUTH tap venue`,
  `netsoak-r9-x64: ALL STAGES PASS` (static + ARP L2-truth + venue
  label; baseline 2048 + burst 8x512; counted 9 tcpblast: done; zero
  `netip: FAIL`).
- **equiv — PASS**: all 10 gate rows `match`, including the four
  recorded expectations — tier1 `T1-OK=39/39`, auth `AUTH-OK=10/10`,
  churn `CATS-DONE 1400=1/1`, stress `netstress: venue tap=1/1` — plus
  soak `NETIP-DONE=1/1; soak ok=1/1`. The `recorded stop-C2 expectation
  (owner approval pending)` label is the run's own provenance; the
  owner's approval is the remaining HARN-08 item.
- **kernel — PASS**: `r9-kernel-x64-678a88c`, kernel ELF sha256
  `2377dc2f…`, boot-iso `KERNEL-BOOTISO-BUILD-OK` and
  `KERNEL-BOOTISO-REBUILD-OK` on the new
  `sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=`.

### Approvals and the Harvey commit (2026-11-23)

1. **Standalone amd64 TCG gate — PASS (owner report).**
   `nix develop -c cargo xtask gate --arch x86-64 --accel tcg` printed
   `GATE-PASS` on the x86_64 host. The `GATE-PASS control-no-rio` lines
   in the full/stress captures are the drawterm legs, not this run.
2. **HARN-08 equivalence — APPROVED (owner, 2026-11-23).** The four
   recorded expectations are accepted: tier1 `T1-OK` 39, auth `AUTH-OK`
   10, churn `CATS-DONE 1400` 1, stress `netstress: venue tap` 1. Stop
   C3 verified each 1:1 and all 10 equivalence rows match.
3. **Harvey diff — APPROVED (owner, 2026-11-23), APPLIED and PUSHED.**
   From base `25d97dc4d4976bfb7411f01970755acf6261c81b`, `git apply` plus
   an explicit 36-path `git add --` produced Harvey commit
   `c697666ba315f137ca2fd11768f835d04bd9f018` on `master`:
   36 files, +285/−9972, zero `sys/lib/(postscript|troff)` or
   `devpost.add` paths; the 11 pre-modified font files were never
   staged. The owner pushed `master c697666ba315f137ca2fd11768f835d04bd9f018`
   on 2026-11-23; `origin/master` carries it (and the pre-existing
   `25d97dc4d` freeze commit), verified via `git ls-remote`.

### Stop C closure

All six witness steps are green (full 8/8 with `fpprobe: ok`, soak,
stress, equiv 10/10, store_verify twice, kernel `b6Oc` build+rebuild),
the standalone amd64 TCG gate is `GATE-PASS`, the four HARN-08
equivalence expectations are approved, and the Harvey flip is applied
and pushed at `c697666ba315f137ca2fd11768f835d04bd9f018`. Plan 01-34
is complete; 01-28 is the orchestrator's next dispatch. Harvey received
no further change; the 11 pre-modified font paths remain untouched and
unstaged.

### The prepared Harvey review packet

**APPROVED 2026-11-23, applied and pushed as Harvey
`c697666ba315f137ca2fd11768f835d04bd9f018` to `origin/master`.** The
packet below is the reviewed record; the owner push completed the flip.

- **Full diff:** `$HOME/r9-harvey-flip/harvey-flip.diff` — 511797 bytes,
  sha256 `f7b0272d23bc6c131c2f63b37cde1330c0bebddd25b905c22040829232cd7f22`.
- **Base:** Harvey `25d97dc4d4976bfb7411f01970755acf6261c81b` (the owner's
  `master`; the planning-freeze commit is this commit's parent and was
  published by the same owner push).
- **Shape:** 36 paths, +285 / −9972 — 32 deletions, 1 new file
  (`nix/q35.sh`), 3 modified (`flake.nix`, `flake.lock`,
  `.github/workflows/phase6.yml`).
- **Deletions:** 17 under `nix/` (16 moved gate scripts + `host-echo.c`),
  9 under `build/x86_64/`, 5 under `build/aarch64/`, and
  `.github/workflows/r9-shell.yml` (whole).
- **Kept exception:** `build/x86_64/drawterm-gate.expect` stays — it is not
  in the diff; the retained `nix/drawterm-x64.sh` names it as the source of
  its `-s ''` pin (the plan's expected exception).
- **Protected set:** `grep -cE 'sys/lib/(postscript|troff)|devpost\.add'`
  over the diff → **0**; the 11 pre-modified font paths are not touched.
- **D-19:** the deletion set's intersection with r9's tree/reference
  read-set paths is empty (the recorded 102-path check → no output; an
  independent re-extraction of the three `nix/harvey/*.nix` files →
  0 intersecting path-like tokens). `grep -c '\.expect'` over the read set →
  0; no `nix/*.sh` appears in any read set.
- **Preparation checks (Mac, read-only):** the diff sha256 matches
  HARVEY-FLIP.md; the scratch clone's
  `git diff --stat HEAD -- . ':(exclude)sys/**'` reproduces 36 files /
  +285 / −9972; `git -C /Users/neilskinner/code/harvey apply --check
  "$HOME/r9-harvey-flip/harvey-flip.diff"` → APPLY-CHECK-OK; the owner's
  checkout is unchanged (HEAD `25d97dc4d…`, status-porcelain sha256
  `041d2929…`), and its 11 font paths still read ` M`.

### What the owner reruns (KVM host, two hours or more)

Part 1, on the Mac — push the r9 source the witness runs from (after the
harness fixes above land):

```sh
git -C /Users/neilskinner/code/r9 push origin HEAD:main
# or, to leave main alone:
git -C /Users/neilskinner/code/r9 push origin HEAD:refs/heads/witness-src/01-27
```

Part 2, on the x86_64 KVM host with the tap/bridge venue up:

```sh
git -C <r9> fetch origin
git -C <r9> checkout <pushed commit>   # or FETCH_HEAD after fetching witness-src/01-27
cd <r9>
R9_WITNESS_STEPS="full soak stress equiv store_verify" \
  bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
```

`step_full` exports `R9_ACCEL=kvm` for every child app and keeps each app's
capture set under `out/full/apps/`; `soak` and `stress` preflight
`nix run .#net-doctor -- --mode tap` and refuse a slirp substitute; `equiv`
writes `out/equiv/report.tsv` against `witness/equivalence.tsv`; `store_verify`
runs last.

Part 2b, return the results (never a creds file):

- **Path A (preferred):** from the KVM host, push branch
  `witness-returns/01-27` with the `out/` tree and `WITNESS-RETURN.md` under
  `.planning/phases/01-stage-p0-cross-build-harness/witness/return-01-27/`.
- **Path B:** paste the full `$HOME/r9-witness-work/WITNESS-RETURN.md`.

Part 3, on the Mac — review the full diff:

```sh
HF=$HOME/r9-harvey-flip
shasum -a 256 "$HF/harvey-flip.diff"
git -C "$HF/harvey" status
git -C "$HF/harvey" diff HEAD --stat -- . ':(exclude)sys/**'
less "$HF/harvey-flip.diff"
```

### The approval wording

**Harvey commit (reply together with the stop-C return):**

> stop C: pushed <ref>, returned via A <sha> | B; Harvey diff APPROVE

or

> stop C: pushed <ref>, returned via A <sha> | B; Harvey diff REJECT: <changes you want>

APPROVE authorises the executor to apply exactly
`$HOME/r9-harvey-flip/harvey-flip.diff` (sha256 `f7b0272d…`) to
`/Users/neilskinner/code/harvey` and make the single commit
`refactor(flake): move r9 composition to the r9 repo; add the q35 own-kernel launcher`
— no push, no AI attribution, and none of the 11 font files staged.

**Owner push (Task 3, after the Harvey commit exists):**

> Harvey pushed: <branch> <sha>

after `git -C ~/code/harvey push origin HEAD:<branch>`. The new commit's parent
is `25d97dc4d`; the owner's 2026-11-23 push published both (D-23, recorded in
HARVEY-FLIP.md).

### Guard rails for the continuation

- No Harvey write, staging or commit before **both** an all-PASS stop C and
  the owner's explicit APPROVE.
- Stage by explicit path from the diff's own list; never `git add -A`, never
  `git commit -a`, never a directory argument covering `sys/**`.
- No push from the executor to either repository; the owner pushes Harvey
  (Task 3) and r9 (Part 1).
- A REJECT or any FAIL ends the pass with no Harvey change; that is what
  happened here. The scratch clone can absorb requested changes before a
  green rerun of this stop.

## Stop E — plan 01-28, x86_64 KVM at the lock-only bump

STATUS stope: PASS for all eleven selected steps. The owner checked out the
bump commit `af46615` on `teeniebox` and returned the full D-14 set through
`witness-returns/01-28` at `95af737c4a38665fa2a60862207ff4fbf391f994`; the
return directory is `witness/return-01-28/` (fetched read-only, not merged).
The lock pins Harvey `c697666ba315f137ca2fd11768f835d04bd9f018` — the pushed
flip commit — and every step below ran against that tree.

- **Source HEAD:** `af4661596d3e259de990baef41a9cbdf0512ce77` (r9 main; the
  lock-only bump `626c80c`, merged as `af46615`).
- **Host:** `teeniebox`, x86_64-linux, nproc 16, 30 GiB RAM, nix 2.35.6,
  `DEV_KVM=writable`.
- **Session:** 2026-11-23T20:59:49Z → 2026-11-23T21:25:45Z.
- **Steps selected:** `trees soak kernel gate gate_neg profiles cpuauth_ref
  full stress equiv store_verify`.
- **Summary:** `out/summary.tsv` — eleven rows, every exit 0.
- **`env.txt` note:** its `harvey_pin=bd7a6982…` is the driver's
  `legacy-final` baseline pin for the Harvey-hosted equivalence captures
  (`run-witness.sh:142`), not the r9 lock. `flake.lock` pins `c697666`.
- **`full` note:** the return carries three `full.tsv` snapshots. The first
  (`out/full/347497-1795455456/full.tsv`) records `boot-r9-tier1-x64 FAIL`;
  the second (`405781-1795461728`) and the final (`623461-1795468943`) are
  8/8. The verdict below is the final run, whose `full.log` ends
  `GATE PASS full x86_64-linux run=623461-1795468943`; the earlier snapshot
  remains in the return tree and is disclosed here rather than dropped.

### Per-step verdicts

VERDICT stope-trees: PASS
EVIDENCE stope-trees: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/trees.log
— `harvey-tree-amd64` and `harvey-tree-arm64` built at `rev=c697666…`; both
`TREES-VERIFY-OK`.

VERDICT stope-soak: PASS
EVIDENCE stope-soak: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/soak.log
— `netip: L2-TRUTH tap venue`; `netsoak-r9-x64: ALL STAGES PASS`; baseline
2048 + burst 8x512; `tcpblast: done x 9`; zero `netip: FAIL`.

VERDICT stope-kernel: PASS
EVIDENCE stope-kernel: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/kernel.log
— `r9-kernel-x64-af46615`, ELF sha256 `2377dc2f…`; `KERNEL-BOOTISO-BUILD-OK`
and `KERNEL-BOOTISO-REBUILD-OK`.

VERDICT stope-gate: PASS
EVIDENCE stope-gate: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/gate.log
— both runs exit 0; `GATE-PASS rio-serving` and `GATE-PASS control-no-rio`;
`ALL STAGES PASS` on both legs.

VERDICT stope-gate-neg: PASS
EVIDENCE stope-gate-neg: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/gate-neg.log
— `GATE-FAIL rio-not-serving` then `GATE-NEG-EXPECTED-FAIL` (the driver
asserts the non-zero exit and the named marker).

VERDICT stope-profiles: PASS
EVIDENCE stope-profiles: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/profiles.log
— nine `PROFILE … exit=0` lines: q35, q35lowmcfg, q35net, q35exec,
q35churn, q35shell, q35churnshell, q35cpu, q35auth.

VERDICT stope-cpuauth-ref: PASS
EVIDENCE stope-cpuauth-ref: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/cpuauth-ref.log
— `cpu` 4442 members and `auth` 95 members, each `matches the reference
member list`; both exit 0.

VERDICT stope-full: PASS
EVIDENCE stope-full: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/full.tsv
EVIDENCE stope-full: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/full.log
— final run 8/8: drawterm-gate-x64, boot-r9-x64, boot-r9-smoke-x64,
boot-r9-tier1-x64, boot-r9-net-smoke-x64, boot-r9-services-x64,
boot-r9-churn-x64, boot-r9-auth-x64.

VERDICT stope-stress: PASS
EVIDENCE stope-stress: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/stress.log
— tap venue, `tcpblast: done x 9`, zero `netstress: FAIL`,
`ALL STAGES PASS`.

VERDICT stope-equiv: PASS
EVIDENCE stope-equiv: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/equiv/report.tsv
— all 10 rows `match`, including tier1 `T1-OK=39/39`, auth `AUTH-OK=10/10`,
churn `CATS-DONE 1400=1/1`, stress `netstress: venue tap=1/1`.

VERDICT stope-store-verify: PASS
EVIDENCE stope-store-verify: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/store-verify.log
— both trees `STORE-VERIFY-OK` after the full step set.

VERDICT stope-harvey-q35: PASS
EVIDENCE stope-harvey-q35: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/harvey-q35.log
— `Q35-KERNEL … sha256=ecdcdadd… bytes=2030226`; `Q35-PROMPT root is from
(tcp)[tcp]:`; `Q35-SIGS none`; `Q35-RESULT machine=q35 accel=tcg banner=yes
prompt=yes sigs=none`. The kernel sha equals the S5 artifact and the 01-08
provenance record.

VERDICT stope-ci-best-effort: INFO (recorded; not a phase gate)
EVIDENCE stope-ci-best-effort: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/ci.txt
— `xtask.yml` all jobs green; `gates.yml` all green except `quick-gate-tcg`
(red). D-13 makes hosted CI the best-effort second witness; the owner's
native amd64 TCG gate is `GATE-PASS` at stop C3.

EVIDENCE stope-env: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/env.txt
EVIDENCE stope-summary: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/summary.tsv
EVIDENCE stope-secret-scan: .planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/secret-scan.txt

## Phase boundary audit

**Scope and point of the audit.** This maps the ROADMAP's seven phase truths
(R1-R7, numbered as in `01-VERIFICATION.md` "A. ROADMAP contract") to the
phase's concrete evidence, and re-checks every phase prohibition at
`af46615` (2026-11-23). The check is evidence-backed: each row names the
verdict ids, commits or file paths that satisfy it, and the prohibition
table records the command and its result. It is not a restatement of the
plan.

### R1-R7 evidence map

| # | ROADMAP truth (abridged) | Verdict | Evidence (verdict ids, commits, paths) |
|---|---|---|---|
| R1 | One documented `cargo xtask` command boots r9 on QEMU q35 x86_64 to a rio session and exits non-zero when rio is not reached | PASS | `VERDICT stope-gate` — two runs, `GATE-RUN 1/2 exit=0`, `GATE-PASS rio-serving` + `GATE-PASS control-no-rio`, ALL STAGES PASS (`witness/evidence/stop-e/gate.log`); `VERDICT stope-gate-neg` — `GATE-FAIL rio-not-serving` then `GATE-NEG-EXPECTED-FAIL` (`stop-e/gate-neg.log`); the command and its exit contract are in `cargo xtask --help` ("exits 0 only then"); plan 01-18 Task 2 `ed52b47` and the 01-33 driver fix `b8575463`; run at `af46615`. |
| R2 | One resolver honours `R9_HARVEY_DIR` and the approved alternatives, a missing tree names the variable, and a full gate leaves the Harvey tree unmodified | PASS | `xtask/src/harvey.rs` implements precedence `R9_HARVEY_DIR` > `R9_HARVEY_DIR_ARM64/AMD64` > `HARVEY_SRC`, with `R9_HARVEY_DIR=vendor/harvey` resolving relative to the workspace and no built-in default; the named-error test lists every variable and `no built-in default`. `cargo test harvey::tests` at `af46615`: 14/14 pass (2026-11-23). `VERDICT stope-store-verify` and `stope-trees` — both Nix trees `STORE-VERIFY-OK` / `TREES-VERIFY-OK` after the eleven-step run; plan 01-08 `a297253`. The `vendor/harvey` and `HARVEY_SRC` names are the alias/relative forms of the one resolver, not implicit fallbacks (D-01/D-21). |
| R3 | The xtask fallback manifest lists every C library object and userland binary per architecture, and the harness fails on two-way image/manifest drift | PASS | `xtask/manifests/amd64/{marker,chain,substrate,cpu,auth,net,services,tier,churn,libraries,fonts-bit}` and `xtask/manifests/arm64/{bootfs,smoke,libraries}`; `check_drift` in `xtask/src/image.rs:1226`; tests `drift_reports_a_missing_entry` and `drift_reports_an_unlisted_staged_file` pass at `af46615` (2026-11-23); the bump's `stope-cpuauth-ref` matches the reference lists (cpu 4442, auth 95) and `stope-full` assembles and boots all eight x86_64 images; plans 01-08 `a297253`, 01-11 `88d1e85`, 01-15/01-16/01-19. |
| R4 | Flipping one manifest entry swaps that single Rust artifact into the image, the gate passes, and flipping back restores the Harvey artifact | PASS | `PROBE-FLIP-SHA 064747f42c7f3eff6cd677a8820e6b4b1fff95e3` and `PROBE-FLIPBACK-SHA af7df344b9c3702100d4408e85044a722e861a67`; `VERDICT stopb3-probe` and `stopb3-flipback` PASS with the serial-check `/bin/ps=R9-RUST-AOUT-PROBE-OK` gate and the byte-identical restored member list (`evidence/stop-b3/probe.log`, `evidence/stop-b3/flipback.log`); the member-list proof is recorded in 01-20 (`witness/evidence/stop-b3/summary.tsv` rows probe/flipback 0); no Rust entry remains in any manifest (`grep -rc rust xtask/manifests` = 0). |
| R5 | The existing x86_64 q35 boot-test profiles and the aarch64 virt quick pass still pass through the reconciled mechanism | PASS (x86_64 at the bump; aarch64 at 01-30 plus the bump report) | `VERDICT stope-profiles` — all nine q35 profiles exit 0 (`stop-e/profiles.log`); `VERDICT stope-full` — 8/8 through the reconciled gate apps (`stop-e/full.tsv`); aarch64: 01-30 Task 2 `5dd9cf5` ran the whole quick pass on the Mac (`AARCH64-QUICK-PASS`, `target/aarch64-quick-pass.log` in the 01-30 record) and the Task 1 bump re-run reported it green; the aarch64 devshell kernel still hashes `76e6e153…`, the 01-08 baseline. |
| R6 | Inside `nix develop`, with nothing set by hand, `cargo xtask` finds the Harvey tree; the flake pins Harvey and exports the read-only tree | PASS | `flake.nix:17` `inputs.harvey = "github:requaos/harvey"`; the devshell exports `R9_HARVEY_DIR_AMD64` / `R9_HARVEY_DIR_ARM64` lazily (`flake.nix:1071-1107`) and never the generic `R9_HARVEY_DIR`; the bump's devshell banners show `harvey supply: R9_HARVEY_DIR_AMD64=…` and the trees carry `rev=c697666`; `cargo xtask --help` at `af46615` prints the resolver contract; plans 01-10 `56063cf`, 01-13 `1d66e2b`, 01-14 `fab8b36`. |
| R7 | Flake ownership is flipped: r9 imports Harvey and owns the composed gates, Harvey has no `r9` input, and each moved gate passes from r9 | PASS | `flake.nix` exposes `boot-r9*`, `drawterm-gate-x64` and `net-doctor` (`nix eval .#apps.x86_64-linux` at `af46615` lists all 20 apps); Harvey `c697666` has `flake.lock` root inputs `{nixpkgs}` only and `flake.nix` has no `r9` input; the bump's `stope-gate`, `stope-gate-neg`, `stope-profiles`, `stope-full`, `stope-soak` and `stope-stress` all pass from r9 at `af46615`; plans 01-26 `c34edeb`, 01-34 `83d00ca`, bump `626c80c`. |

### Prohibition audit

| # | Prohibition | Command | Result |
|---|---|---|---|
| P1 | No unpushed Harvey pin (D-23) | `git ls-remote https://github.com/requaos/harvey refs/heads/master` vs `jq -r '.nodes.harvey.locked.rev' flake.lock` | `c697666ba315f137ca2fd11768f835d04bd9f018` both; PASS. |
| P2 | No font-file staging; Harvey gained exactly one executor commit | `git -C ~/code/harvey rev-list --count 25d97dc4d..HEAD`; `git show --name-only --format= c697666`; `git status --porcelain --untracked-files=no`; `git status --porcelain \| shasum -a 256` | 1 commit (`c697666`), 36 paths, `grep -cE 'sys/lib/(postscript\|troff)\|devpost\.add'` = 0; the 11 pre-modified font paths remain ` M`; status hash `041d29292e50…` equals the recorded `041d2929…`; PASS. |
| P3 | The bump commit changes `flake.lock` only (D-06) | `git show --name-only --format= 626c80c` | `flake.lock` only; PASS. |
| P4 | flake.lock locks only the pushed Harvey commit; no nested r9 node | `jq -r '.nodes.harvey.inputs \| keys[]' flake.lock`; `jq -r '.nodes \| keys[]' flake.lock \| grep -c '^r9'`; `grep -c 116bcc23c8052a1728992cdbc7b625143260d683 flake.lock` | `nixpkgs` only; 0 `r9` nodes; 0 `116bcc2`; PASS. |
| P5 | The devshell never exports `R9_HARVEY_DIR` (the generic override stays the user's) | `grep -cE 'export R9_HARVEY_DIR=' flake.nix` | 0; PASS. |
| P6 | xtask has no built-in Harvey path | `nix develop -c cargo test --manifest-path xtask/Cargo.toml no_other_source_reads_a_harvey_tree_variable` | 1 passed, 0 failed; the resolver docs say no `../harvey`, no implicit `vendor/harvey`; PASS. |
| P7 | No `port/src` edit without approval | `git diff --name-only 6a64287~1 HEAD -- port/src` | no output (0 paths); PASS. |
| P8 | No gate weakening | `git diff --numstat 7619dd4~1 HEAD -- xtask/src/gate.rs gates/x86_64/drawterm-gate.expect`; `VERDICT stope-gate` + `stope-gate-neg` | 971/0 and 777/0 (additions only, zero deleted assertions); the positive pair and the failing direction both ran at the bump; PASS. |
| P9 | No marker weakening (D-28 pair) | `grep -n 'riowctl\\\.' gates/x86_64/drawterm-gate.expect`; `VERDICT stope-gate` / `stope-gate-neg` | the pair `rio.<user>.<pid>` + `riowctl.<user>.<pid>` is the only pass condition; `GATE-PASS rio-serving` and `GATE-FAIL rio-not-serving` at `af46615`; PASS. |
| P10 | No credential committed in pass 2 | `git diff "$(sed -n 's/^plan_head_before: //p' 01-08-SUMMARY.md)" HEAD \| grep '^+' \| grep -E 'password=[^<\`"[:space:]]'`; `git ls-files \| grep -cE '\.creds$\|auth-creds'`; `grep -rE 'auth-creds\|[Pp]assword=' witness/evidence/stop-e` | at `af46615` (before this audit's own text) the literal regex printed 20 added lines, all benign: parser contract (`grep -q '^password='`/`sed 's/^password=//p'`), runtime creds writers (`printf 'password=%s'`), synthetic `FAKEPW123451` test fixtures, and doc refs — no credential value; 0 tracked creds files; 0 matches in the stop-E evidence; PASS on the intent (no credential), with the plan's literal `EXPECT==0` regex noted as over-broad. |
| P11 | No manifest has a rust entry | `grep -rc rust xtask/manifests` | 0; PASS. |
| P12 | Both trees verify unmodified after the full set | `VERDICT stope-store-verify`; `stope-trees` | `STORE-VERIFY-OK` on the amd64 and arm64 trees; `TREES-VERIFY-OK`; PASS. |
| P13 | Kernel bytes unchanged at the bump | `VERDICT stope-kernel`; stop-C3 `kernel.log`; `cargo xtask build --arch aarch64 --config virtshell` + `shasum`; `cargo xtask build --arch x86-64 --config q35churnshell` + `shasum` | stop-E ELF `2377dc2f…` equals stop-C3 `2377dc2f…`, boot-iso BUILD+REBUILD OK; aarch64 `76e6e153…` equals the 01-08 baseline; x86_64 `8f2d485e…` (devshell, reproduced twice) supersedes the 01-08 `50b01c9a…` because the approved 01-33/01-34 x86_64 fixes (`9af05f8`, `2a5cbed`) changed x86_64 source; the bump itself changed no kernel source; PASS with the baseline supersession recorded. |

### Residual risks and dispositions

1. **Hosted CI `quick-gate-tcg` is red.** `xtask.yml` is fully green and
   `gates.yml` is green except that leg. D-13 makes CI the best-effort second
   witness; the owner's native amd64 TCG gate is `GATE-PASS` at stop C3, and
   the stop-E gate/gate-neg ran on KVM. Disposition: informational, not a
   phase gate.
2. **The first stop-E `full` iteration recorded `boot-r9-tier1-x64 FAIL`.**
   The owner re-ran; the second and final iterations are 8/8, and stop C3's
   `full` was 8/8. Disposition: transient, final verdict PASS; the earlier
   `full.tsv` is retained in `return-01-28/full/347497-1795455456/`.
3. **The R4 swap proof is the stop-B3 KVM run** at `PROBE-FLIP-SHA` /
   `PROBE-FLIPBACK-SHA`, before the 01-34 `gate.rs` fix. That fix changed the
   driver's spawn/registration, not the swap assertion; the manifests have no
   Rust entry, so a re-run would require re-flipping. Disposition: accepted,
   recorded.
4. **No bump-time aarch64 quick-pass log is in the return.** The aarch64
   half of R5 rests on 01-30's Mac log and the Task 1 bump re-run (reported
   green, no retained file); the stop-E set is x86_64 KVM. Disposition:
   accepted, recorded; the aarch64 kernel hash still matches the 01-08
   baseline.
5. **The 01-08 x86_64 kernel baseline is superseded** by the approved
   01-33/01-34 x86_64 fixes; the audit compares at the bump (unchanged
   package ELF from stop C3), not to the stale 01-08 literal. Disposition:
   documented deviation from the plan's stale acceptance line.


