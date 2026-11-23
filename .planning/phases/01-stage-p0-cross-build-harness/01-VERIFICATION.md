---
phase: 01-stage-p0-cross-build-harness
verified: 2026-11-23T03:06:43Z
status: gaps_found
score: 35/42 must-haves verified
covered_files:
  - .planning/phases/01-stage-p0-cross-build-harness/01-01-PLAN.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-01-SUMMARY.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-02-PLAN.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-02-SUMMARY.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-03-PLAN.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-03-SUMMARY.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-04-PLAN.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-04-SUMMARY.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-05-PLAN.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-05-SUMMARY.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-06-PLAN.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-06-SUMMARY.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-07-PLAN.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-07-SUMMARY.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-CONTEXT.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-DISCUSSION-LOG.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-RESEARCH.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-SECURITY.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-UI-REVIEW.md
  - .planning/phases/01-stage-p0-cross-build-harness/01-VALIDATION.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/S1-FINDINGS.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/S2-FINDINGS.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/S3-FINDINGS.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/S4-FINDINGS.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/S5-FINDINGS.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/S6-FINDINGS.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/SPIKE-RESULTS.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/.gitignore
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/README.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/evidence/env.txt
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/evidence/summary.tsv
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/make-bundle.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/run-all.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/run-from-repo.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/.gitignore
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/evidence/builder-setup.txt
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/evidence/rung1-builder.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/evidence/rung2-builder.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/evidence/witness-x86_64-linux.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/flake.lock
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/flake.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/hostbin.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/venue.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/.gitignore
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/collisions.txt
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/evidence/casecheck-builder.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/evidence/lazy-devshell.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/flake.lock
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s2/flake.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/.gitignore
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/determinism.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/evidence/determinism-kvm-linux.txt
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/evidence/norio-control-kvm-linux.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/evidence/rio-kvm-linux.ATTRIBUTION-NOTE.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/evidence/rio-kvm-linux.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/evidence/static-analysis.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/flake.lock
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/flake.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/rio-gate.expect
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s3/rio-gate.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/.gitignore
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/evidence/tier-builder.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/flake.lock
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/flake.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/hostbin.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/tier.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s4/venue.nix
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/.gitignore
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/9k10cpu.elf
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/boot-q35.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/build-path-analysis.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/kernel-provenance.txt
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/pc-baseline-tcg-mac.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/q35-kvm-linux.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/q35-nonic-tcg-mac.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s5/evidence/q35-virtio-tcg-mac.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/.gitignore
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/aout.py
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/build-probe.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/evidence/amd64-static.txt
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/evidence/arm64-hvf-control.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/evidence/arm64-hvf-flipback.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/evidence/arm64-hvf-probe.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/evidence/x86_64-kvm-linux.log
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/linux-x86_64.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-amd64.ld
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-amd64.rs
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-arm64.ld
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/probe-arm64.rs
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/q35shell-probe.expect
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/run-arm64-hvf.sh
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/target-analysis.md
  - .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/virt-probe.expect
covered_digest: "v4:sha256:25667821eb44ffc16146f8d724856c13e79758021ec67cd3735766983c950207"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "One documented `cargo xtask` command boots r9 on QEMU q35 x86_64 to a rio session using only the r9 kernel and the pre-built Harvey tree, and exits non-zero when rio is not reached."
    status: failed
    reason: "Pass-2 scope (CONTEXT D-22, owner-approved). Pass 1 implements no HARN requirement. xtask has no rio/gate subcommand and no rio assertion; boot-test asserts banner/ticks/probe markers only. There is also no pre-built Harvey tree supplied by the flake."
    artifacts:
      - path: "xtask/src/main.rs"
        issue: "Subcommands are build, expand, kasm, boot-test, qemu, check, test, clippy, initrd, ureg-check, etc. No rio or golden-gate command; no rio assertion anywhere in xtask/src."
      - path: "flake.nix"
        issue: "No Harvey input or tree derivation to boot from."
    missing:
      - "The `cargo xtask` quick gate that boots to rio and exits non-zero without it, with its negative control (SPIKE-RESULTS pass-2 HARN-01 row)."
      - "Resolve the generated DEMO password/credential path and the headless drawterm auth/feed before the rio marker is trusted (SPIKE-RESULTS Decision 5)."
  - truth: "Harvey is located through one resolver that honours the existing `R9_HARVEY_DIR` and the approved alternatives (`HARVEY_SRC`, `vendor/harvey/`), a missing tree fails with a message naming the variable, and after a full gate run the Harvey tree is byte-for-byte unmodified."
    status: failed
    reason: "Partial pre-existing implementation only. `uregcheck::harvey_dir()` reads R9_HARVEY_DIR, errors with the variable name, and the in-tree consumers route through it; but `HARVEY_SRC` and `vendor/harvey/` have zero references, and no gate run proves the tree unmodified."
    artifacts:
      - path: "xtask/src/uregcheck.rs"
        issue: "Only R9_HARVEY_DIR is read; no HARVEY_SRC or vendor/harvey precedence."
      - path: ".github/workflows/xtask.yml"
        issue: "CI still checks Harvey out manually and sets R9_HARVEY_DIR."
    missing:
      - "Resolver precedence over R9_HARVEY_DIR / HARVEY_SRC / vendor/harvey, per-architecture variables, and the after-gate byte-for-byte unmodified proof."
  - truth: "The xtask Harvey fallback manifest lists every C library object and userland binary in the image per architecture, and the harness fails when the image and the manifest disagree in either direction."
    status: failed
    reason: "No manifest exists in xtask and no drift detection exists. HARN-05 (the two-way drift requirement) is claimed by no pass-1 plan and is explicitly deferred by name in SPIKE-RESULTS."
    artifacts:
      - path: "xtask/src/"
        issue: "No manifest type, no per-arch manifest data, no xtask/src/image.rs, no drift errors."
    missing:
      - "Per-architecture manifests of every image file plus the tier-0 library table with a Harvey/Rust source per entry."
      - "Named drift errors in both directions and the Rust-provenance rule (HARN-05)."
  - truth: "Flipping one manifest entry to a Rust replacement swaps that single artifact into the image, the gate still passes, and flipping it back restores the Harvey artifact."
    status: failed
    reason: "The mechanism was proven as a spike (S6: arm64 HVF and x86_64 KVM, control/probe/flip-back, sha-equal=yes) but no in-tree manifest, assembler or swap exists in r9, so nothing exercises it in the build."
    artifacts:
      - path: ".planning/phases/01-stage-p0-cross-build-harness/spikes/s6/"
        issue: "Proof only; probe and swap live under the spikes directory and are deliberately not wired into r9."
      - path: "xtask/src/"
        issue: "No image assembler or manifest swap."
    missing:
      - "The in-tree swap probe, flip and flip-back in the gate, then its removal after the proof (D-18)."
  - truth: "The existing x86_64 q35 boot-test profiles and the aarch64 virt quick pass still pass through the reconciled mechanism."
    status: failed
    reason: "The profiles exist, but there is no reconciled mechanism (single resolver plus manifest-assembled images) and no pass-1 run of them through it. Pass 1 did not change the existing mechanism, so this is unimplemented rather than regressed."
    artifacts:
      - path: "x86_64/lib/config_q35*.toml"
        issue: "Profiles exist; images still come from Harvey's baked tree via R9_HARVEY_DIR."
      - path: "aarch64/lib/config_virt*.toml"
        issue: "Quick-pass profiles exist; not routed through a reconciled mechanism."
    missing:
      - "Route the existing x86_64 q35 profiles and the aarch64 virt quick pass through the resolver/assembler and record their results."
  - truth: "Inside `nix develop`, with nothing set by hand, `cargo xtask` commands find the Harvey tree: the r9 flake pins Harvey and the devshell exports its read-only location, and that tree contains the pre-built artifacts the harness needs."
    status: failed
    reason: "r9's flake has no Harvey input, no built-tree derivation, and its devshell exports no R9_HARVEY_DIR* variable (zero `R9_HARVEY` matches in flake.nix)."
    artifacts:
      - path: "flake.nix"
        issue: "Inputs are nixpkgs and rust-overlay only; devShell shellHook exports only OBJCOPY and banner text."
    missing:
      - "`inputs.harvey`, the harvey-tree-amd64/arm64 and mkpaqfs derivations, devshell exports of R9_HARVEY_DIR_AMD64/ARM64, CI on the flake supply, and the documented Mac builder."
  - truth: "Flake ownership is flipped: r9's flake imports Harvey and exposes the composed boot and gate outputs that Harvey's flake used to own, Harvey's flake has no `r9` input, and each moved gate passes when run from r9."
    status: failed
    reason: "The pre-flip arrangement is unchanged. Harvey's flake still declares `inputs.r9` and owns the boot-r9*/drawterm-gate/net-doctor outputs; r9's flake has no Harvey import and no composed boot/gate outputs."
    artifacts:
      - path: "flake.nix"
        issue: "No Harvey input and no boot-r9*/drawterm-gate apps."
      - path: "../harvey/flake.nix"
        issue: "Still has `inputs.r9.url = \"github:requaos/r9\"` and 43 boot-r9/gate references."
    missing:
      - "Composed apps in r9, copied and rewired gate scripts and expect drivers, relocated CI legs, the owner-approved Harvey commit, and the lock-only pin bump."
---

# Phase 1: Stage P0 - Cross-Build Harness Verification Report

**Phase Goal:** The owner can build and boot r9 to rio on QEMU q35 from the r9 repo with `cargo xtask`, drawing every C library object and userland binary from a pre-built, read-only Harvey tree, and can replace those C artifacts with Rust ones one at a time without ever breaking the boot.
**Verified:** 2026-11-23T03:06:43Z
**Status:** gaps_found
**Re-verification:** No — initial verification

## Deliverable classification (read this first)

Phase 1 was deliberately planned in two passes (CONTEXT **D-22**, owner-approved; STATE.md decision log; `01-07-SUMMARY.md` line 211). The classification below keeps the two questions separate:

| Question | Answer | Evidence |
|---|---|---|
| Are the **pass-1 spike deliverables** complete? | **YES** | 6/6 spikes ANSWERED; 25/25 legs have a PASS/FAIL verdict with committed evidence; 35/35 pass-1 plan truths verified. |
| Is the **full phase goal** achieved? | **NO** | 0/7 ROADMAP success criteria; 0/8 HARN requirements implemented. No rio gate, no manifest, no resolver extensions, no flake flip. |

**Pass-2 prerequisite (plainly):** the phase is not finished. The documented next action is **`/gsd-plan-phase 1`** to write planning pass 2 (STATE.md: "re-run /gsd-plan-phase 1 for pass 2"), **not** a gap-closure cycle. The structured `gaps:` above are the pass-2 scope, not defects in pass-1 execution. Pass 2 must implement HARN-01 through HARN-08; the owner's seven closing-checkpoint decisions (SPIKE-RESULTS "Owner decisions") are the inputs.

## Goal Achievement

### Observable Truths

#### A. ROADMAP contract (the full phase goal)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| R1 | One documented `cargo xtask` command boots r9 on QEMU q35 x86_64 to a rio session and exits non-zero when rio is not reached | ✗ FAILED | `xtask` subcommands contain no rio/gate command; zero rio assertions in `xtask/src`. Only the S3 spike gate exercises a rio marker, and its positive leg is FAIL. |
| R2 | One resolver honours `R9_HARVEY_DIR`, `HARVEY_SRC`, `vendor/harvey/`; a missing tree names the variable; a full gate leaves the Harvey tree unmodified | ✗ FAILED | `xtask/src/uregcheck.rs:76-84` reads only `R9_HARVEY_DIR`; `HARVEY_SRC` and `vendor/harvey` have zero references in the repo. The named-error half exists; the alternatives and the after-gate proof do not. |
| R3 | The xtask fallback manifest lists every C library object and userland binary per architecture, and the harness fails on two-way image/manifest drift | ✗ FAILED | No manifest symbols or files in `xtask/`; `xtask/src/image.rs` does not exist. HARN-05 has no plan claiming it. |
| R4 | Flipping one manifest entry swaps that single Rust artifact into the image, the gate passes, and flipping back restores the Harvey artifact | ✗ FAILED | Proven only as an S6 spike (control/probe/flip-back, sha-equal=yes); no in-tree manifest/assembler/swap. |
| R5 | The existing x86_64 q35 boot-test profiles and the aarch64 virt quick pass still pass through the reconciled mechanism | ✗ FAILED | Profiles exist (`x86_64/lib/config_q35*.toml`, `aarch64/lib/config_virt*.toml`), but no reconciled mechanism exists and no pass-1 run is recorded through it. |
| R6 | Inside `nix develop`, with nothing set by hand, `cargo xtask` finds the Harvey tree; the flake pins Harvey and exports the read-only tree | ✗ FAILED | `flake.nix` has no Harvey input and no `R9_HARVEY*` export; grep count in `flake.nix` = 0. |
| R7 | Flake ownership is flipped: r9 imports Harvey and owns the composed gates; Harvey has no `r9` input; each moved gate passes from r9 | ✗ FAILED | `flake.nix` has no Harvey import; `../harvey/flake.nix` still has `inputs.r9` and owns boot-r9*/drawterm-gate/net-doctor. |

#### B. Pass-1 plan must-haves (D-22 scope)

| # | Truth (plan) | Status | Evidence |
|---|--------------|--------|----------|
| 01-01.1 | Owner chose the builder venue before any machine change; recorded on `BUILDER-VENUE`, and it does not settle D-04 | ✓ VERIFIED | `spikes/S1-FINDINGS.md:10` `BUILDER-VENUE: lima (owner, 2026-11-22)`; `s1/evidence/builder-setup.txt` shows pre-bring-up read-only state (FlakeHub logged-out); D-04 is settled only in SPIKE-RESULTS Decision 1. |
| 01-01.2 | S1 rung 1 has a PASS/FAIL verdict on each chosen builder with the SANDBOX-EVIDENCE block | ✓ VERIFIED | `S1-FINDINGS.md:19` FAIL, `:42` PASS (proot); `:58-62` SANDBOX-EVIDENCE block. |
| 01-01.3 | S1 rung 2 (host toolchain + amd64 libc, no sudo) has a verdict with artifact facts or failing lines | ✓ VERIFIED | `S1-FINDINGS.md:93` PASS; 601170-byte libc, 259 members, 2 s on lima and 4 s on the witness; `nix store verify` clean. |
| 01-01.4 | The S1 flake evaluates for x86_64-linux and aarch64-linux | ✓ VERIFIED | `s1/flake.nix:22-23` lists both systems and exposes the probe packages; the witness built the same derivations. |
| 01-01.5 | S1-FINDINGS says what each verdict means for the pass-2 tree derivation (D-03) and names any fallback rung | ✓ VERIFIED | `S1-FINDINGS.md:204-229` "What this means for planning pass 2"; proot fallback named and measured. |
| 01-02.1 | S2 answers both source modes for the 23 case-colliding Harvey paths | ✓ VERIFIED | `S2-FINDINGS.md:21` PASS (Mac-fetched input), `:36` PASS (builder-side FOD); `s2/collisions.txt` is 23 lines, and an independent `git ls-tree` recomputation matched it exactly. |
| 01-02.2 | S2 answers Mac store copy-back (accept, verify, re-check contents) | ✓ VERIFIED | `S2-FINDINGS.md:49` FAIL (`nix copy --from` system-store signature policy), `:68` PASS (same-volume chroot store, 23/23 blobs); decisive evidence in `casecheck-builder.log`. |
| 01-02.3 | A9 answers whether a devshell can export a Linux `outPath` lazily and whether it matches the later build path | ✓ VERIFIED | `s2/evidence/lazy-devshell.log`: path invalid before and after the shell, printed path == evaluated path == built path. |
| 01-02.4 | S4 answers tier 0 (27 libs) and tier 1 (40 commands) on aarch64-linux with counts, wall time and S_MAGIC | ✓ VERIFIED | `S4-FINDINGS.md:23,34` PASS; `s4/evidence/tier-builder.log:82-83` 27/27 and 40/40, 6 s and 19 s; tier files at the pin contain exactly 27 and 40 non-comment entries. |
| 01-02.5 | S2/S4 findings carry verdicts with evidence and D-04/D-05/pass-2 implications | ✓ VERIFIED | Both findings files have VERDICT/EVIDENCE lines and "What this means for planning pass 2" sections. |
| 01-03.1 | Harvey's own `9k10cpu.elf` boots under q35 TCG with virtio-net and with no NIC, with prompt or failure point | ✓ VERIFIED | `S5-FINDINGS.md:32,35` PASS; both logs end `S5-RESULT ... prompt=yes sigs=none` at `root is from (tcp)[tcp]:`. |
| 01-03.2 | A `-M pc` baseline shows whether the Mac reproduces the recorded earlier TCG result | ✓ VERIFIED | `S5-FINDINGS.md:38` PASS; `pc-baseline-tcg-mac.log:16` prompt=yes, and the findings state the behaviour (not artifact) reproduction. |
| 01-03.3 | `boot-q35.sh` runs the same classification on Mac (TCG) and Linux (KVM) and refuses to downgrade KVM silently | ✓ VERIFIED | `boot-q35.sh:58-60` exits `kvm-unavailable` rather than fall back; `q35-kvm-linux.log:19` PASS. |
| 01-03.4 | `build-path-analysis.md` answers how Harvey builds 9k10cpu without r9, sudo/namespace needs, prompt needs, and D-20 options | ✓ VERIFIED | `s5/evidence/build-path-analysis.md` sections 1-5, 256 lines, file:line citations and Option A/B. |
| 01-03.5 | S5-FINDINGS has PASS/FAIL for every Mac leg, a PENDING-OWNER verdict for KVM, and D-08/D-20 implications | ✓ VERIFIED | Three Mac PASS verdicts; the KVM leg was PENDING-OWNER at Plan 03 and is now PASS (designed Plan 06 progression); implications at `S5-FINDINGS.md:127`. |
| 01-04.1 | A no_std amd64 a.out passes every parse_aout rule, with disassembly showing PWRITE(51) and EXITS(8) | ✓ VERIFIED | `s6/evidence/amd64-static.txt`: AOUT-OK, magic 0x8a97, text 96, entry 0x200028, `mov ebp,0x33`/`0x8` + syscall sequences, marker bytes present. |
| 01-04.2 | The arm64 technique works end to end: probe replaces `echo`, r9 boots under HVF, `/bin/echo` prints the marker, flip-back restores bytes | ✓ VERIFIED | `S6-FINDINGS.md:42` PASS; control `R9-ECHO-CONTROL`, probe `R9-RUST-AOUT-PROBE-OK`, flip-back control marker; image sha 746f1dfe… equal on control/flip-back. |
| 01-04.3 | `target-analysis.md` records boot/gate invocation of ls, cat, echo, ps, rm, test and recommends the D-17 target | ✓ VERIFIED | `s6/target-analysis.md` (251 lines) evaluates six candidates and recommends `ps`; echo disqualified with a source citation. |
| 01-04.4 | The x86_64 KVM leg is scripted and statically checked, PENDING-OWNER for Plan 06 | ✓ VERIFIED | `s6/linux-x86_64.sh` exists; `bash -n`, shellcheck and Tcl checks recorded; the leg is now PASS on the witness (`S6-FINDINGS.md:99`), superseding PENDING-OWNER as designed. |
| 01-04.5 | S6-FINDINGS has PASS/FAIL for both Mac legs and D-17/D-18 implications | ✓ VERIFIED | Two Mac PASS verdicts plus x86_64 PASS; implications at `S6-FINDINGS.md:161-196`. |
| 01-05.1 | A runnable rewired drawterm gate drives non `-G` drawterm under Xvfb and asserts both rio srv entries with named failures | ✓ VERIFIED | `s3/rio-gate.sh` (323 lines) and `rio-gate.expect` (412 lines); `S3-FAIL rio-not-serving`, `drawterm-exited`, `no-session`, `control-saw-rio` present; serial `ls /srv` parser at `rio-gate.expect:321-358`. |
| 01-05.2 | A `control-noG` mode must see no rio srv entry, proving the assertion discriminates | ✓ VERIFIED | `S3_MODE=control-noG` at `rio-gate.sh:110-112`, `S3-CONTROL-NO-RIO` at expect; the witness control PASSed with only `/srv/boot` and `/srv/factotum`. |
| 01-05.3 | The scratch S3 flake evaluates on the Mac for x86_64-linux over Harvey bd7a698's locked r9 pin and drawterm package | ✓ VERIFIED | `s3/flake.nix` pins bd7a698, follows Harvey nixpkgs, resolves r9 packages at run time; `01-05-SUMMARY.md:78` records `nix eval --raw ...#apps.x86_64-linux.s3-rio-gate.program` returning a store path. |
| 01-05.4 | `determinism.sh` can show per-image whether two bakes with pinned KERNDATE are byte-identical | ✓ VERIFIED | `s3/determinism.sh` (194 lines) bakes twice, hashes all nine images, records IDENTICAL/DIFFERENT, auth/cpu member-list split; witness run recorded the bake-2 FAIL with the decisive `hsecstored` error. |
| 01-05.5 | Static A5/A6 answers have PASS/FAIL verdicts, and the three KVM legs are PENDING-OWNER | ✓ VERIFIED | `S3-FINDINGS.md:19,33` static PASS; the three KVM legs were PENDING-OWNER and are now FAIL/PASS/FAIL (designed Plan 06 progression). |
| 01-06.1 | One self-contained checksummed bundle runs every Linux-only leg in one session with no sudo/root and no access to the owner's checkouts | ✓ VERIFIED | `linux-witness/make-bundle.sh` + `run-from-repo.sh` + `run-all.sh`; root refusal, no sudo calls (grep), `MANIFEST.sha256` verification; `summary.tsv` records all four steps. |
| 01-06.2 | The owner has an exact copy-pasteable command sequence and an explicit list of outputs behind a blocking human checkpoint | ✓ VERIFIED | `linux-witness/README.md:40-84` command sequence, `:191-245` return paths and what not to send; `01-06-PLAN.md:169` is a `checkpoint:human-action` with `gate="blocking-human"`. |
| 01-06.3 | Every PENDING-OWNER verdict is replaced by PASS/FAIL with committed evidence, and no credential appears | ✓ VERIFIED | 25/25 verdicts in SPIKE-RESULTS; redacted evidence; credential scan of committed logs found zero password values; `rio-kvm-linux.ATTRIBUTION-NOTE.md` corrects the tr-ok attribution. |
| 01-06.4 | A step the owner did not run is never given a verdict | ✓ VERIFIED | `summary.tsv` has one row per executed step (s1/s5/s3/s6); `01-06-PLAN.md:213` requires a targeted rerun rather than a verdict for a missing step; no step is missing. |
| 01-07.1 | Every spike has a PASS/FAIL verdict with committed evidence, and no leg is left PENDING-OWNER | ✓ VERIFIED | 25/25 matrix rows; all six findings files have verdicts; one stale prose mention remains in `s3/evidence/static-analysis.md:52` (warning below), but it is not a verdict line. |
| 01-07.2 | SPIKE-RESULTS shows verdicts by venue, A1-A16 status, and decisions with options and evidence-based recommendations | ✓ VERIFIED | Verdict matrix (25 rows), 16 ASSUMPTION lines, seven decisions each with Evidence/Options/Recommendation. |
| 01-07.3 | The owner chose D-04 and confirmed or overrode every recommendation, recorded for pass 2 | ✓ VERIFIED | SPIKE-RESULTS "Owner decisions (closing checkpoint)" quotes the reply verbatim and records DECISION 1-7 plus `PASS-2 READY: yes`. |
| 01-07.4 | Every machine change is listed with its revert command, and any requested revert is done | ✓ VERIFIED | Six-row machine-change ledger with revert commands; "No revert requested"; the lima VM/guest config remain deliberately. |
| 01-07.5 | Pass-1 prohibitions held: Harvey untouched, r9 protected paths unchanged, nothing pushed | ✓ VERIFIED (warning) | `HARVEY-UNTOUCHED` reproduced now (HEAD 25d97dc4d, status sha 041d2929…); protected-path diff and status vs the phase baseline are both 0 lines. One documented owner-approved push by Plan 06 (c92a3f4) is the exception to the blanket no-push wording; Plan 07 pushed nothing. |
| 01-07.6 | Every unimplemented requirement ID and CONTEXT decision is listed in the deferred-to-pass-2 section | ✓ VERIFIED | All eight HARN IDs and D-01 through D-23 are listed; HARN-05 explicitly says "no spike bears on it". |

**Score:** 35/42 truths verified (0 present-but-behavior-unverified; 7 failed = the ROADMAP contract).

### Deferred Items

None. The seven failed truths are **not** deferred to a later milestone phase: Phase 2 and later depend on the harness but their roadmaps do not implement it. They are deferred to **planning pass 2 of this phase** (D-22), which is why they remain `gaps_found` rather than being filtered out here.

### Advisory (New Scope, Unevidenced)

Not applicable — this is an initial verification, not a re-verification.

### Required Artifacts

All 23 artifacts declared in the pass-1 PLAN frontmatter exist, are substantive, and are wired to the findings/evidence they claim.

| Plan | Artifact | Status | Details |
|------|----------|--------|---------|
| 01-01 | `spikes/S1-FINDINGS.md` | ✓ VERIFIED | 229 lines; BUILDER-VENUE, 6 verdicts, evidence lines, assumptions, pass-2 implications. |
| 01-01 | `spikes/s1/flake.nix` | ✓ VERIFIED | Pins bd7a698; both Linux systems; imports venue/hostbin. |
| 01-01 | `spikes/s1/venue.nix` | ✓ VERIFIED | 237 lines; unshare and proot venues; SANDBOX-EVIDENCE and `/usr/bin/env` guard. |
| 01-01 | `spikes/s1/hostbin.nix` | ✓ VERIFIED | 120 lines; plan9port bootstrap with the four sandbox adjustments. |
| 01-02 | `spikes/S2-FINDINGS.md` | ✓ VERIFIED | 203 lines; four verdicts + A9; D-04/D-05 implications. |
| 01-02 | `spikes/S4-FINDINGS.md` | ✓ VERIFIED | 154 lines; tier 0/1 counts, timings, S_MAGIC, venue ports. |
| 01-02 | `spikes/s2/collisions.txt` | ✓ VERIFIED | Exactly 23 `<blob> <path>` lines; independently recomputed from Harvey at bd7a698 and matched. |
| 01-02 | `spikes/s4/tier.nix` | ✓ VERIFIED | 185 lines; per-entry build loops; references `tier1-cmds.txt`. |
| 01-03 | `spikes/s5/boot-q35.sh` | ✓ VERIFIED | 219 lines; capture-before-start, classification, KVM refusal, kill/reap trap. |
| 01-03 | `spikes/S5-FINDINGS.md` | ✓ VERIFIED | 153 lines; three Mac PASS + KVM PASS; A11; D-08/D-20 implications. |
| 01-03 | `spikes/s5/evidence/build-path-analysis.md` | ✓ VERIFIED | 256 lines; build chain, sudo/namespace needs, prompt analysis, D-20 options. |
| 01-04 | `spikes/s6/aout.py` | ✓ VERIFIED | 137 lines; wrap/check; `0x8A97` and `0x8C47`; parse_aout rules. |
| 01-04 | `spikes/s6/probe-amd64.rs` | ✓ VERIFIED | no_std probe with PWRITE/EXITS and the marker. |
| 01-04 | `spikes/s6/linux-x86_64.sh` | ✓ VERIFIED | 174 lines; q35shell control/probe/flip-back; refusals and trap. |
| 01-04 | `spikes/S6-FINDINGS.md` | ✓ VERIFIED | 200 lines; three PASS verdicts, D-17/D-18 implications. |
| 01-05 | `spikes/s3/rio-gate.sh` | ✓ VERIFIED | 323 lines; S3_MODE rio/control-noG; Xvfb `-nolisten tcp`; named failures. |
| 01-05 | `spikes/s3/rio-gate.expect` | ✓ VERIFIED | 412 lines; serial `ls /srv` assertion for `rio.*` + `riowctl.*`; control negative. |
| 01-05 | `spikes/s3/flake.nix` | ✓ VERIFIED | 64 lines; Harvey bd7a698, Mac-evaluable app, locked runtime packages. |
| 01-05 | `spikes/S3-FINDINGS.md` | ✓ VERIFIED (superseded pattern) | 205 lines; five verdicts + A5/A6. The PLAN's `PENDING-OWNER` pattern no longer matches because Plan 06 replaced it with the stronger FAIL/PASS/FAIL record — the designed progression, not a stub. |
| 01-06 | `spikes/linux-witness/run-all.sh` | ✓ VERIFIED | 382 lines; root refusal, `R9_SPIKES_ONLY`, per-step isolation, secret scan. |
| 01-06 | `spikes/linux-witness/make-bundle.sh` | ✓ VERIFIED | 144 lines; `MANIFEST.sha256` over every file, verified from a fresh extraction. |
| 01-06 | `spikes/linux-witness/evidence/summary.tsv` | ✓ VERIFIED | Four rows (s1/s5/s3/s6) with exit codes and durations from the owner's run. |
| 01-07 | `spikes/SPIKE-RESULTS.md` | ✓ VERIFIED | 434 lines; verdict matrix, A1-A16, seven decisions, machine-change ledger, prohibition audit, deferred list, owner decisions. |

Supporting evidence not in PLAN frontmatter (also verified): all S1-S6 evidence logs, the S6 arm64 control/probe/flip-back logs, the witness-return logs, the kernel copy (`s5/9k10cpu.elf`, sha256 ecdcdad…), and the witness bundle scripts/README.

### Key Link Verification

All nine declared plan key links are wired. One gsd-tool result is a literal-match false negative: it searched for the string `^EVIDENCE s1-rung1-` (with the caret) and missed the actual `EVIDENCE s1-rung1-lima-aarch64:` lines at `S1-FINDINGS.md:20,43`.

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `s1/flake.nix` | `s1/venue.nix` | imports to build s1-nsprobe/s1-libc-amd64 | WIRED | `import ./venue.nix` present. |
| `s1/flake.nix` | `s1/hostbin.nix` | hostbin first on PATH in the libc guest | WIRED | `import ./hostbin.nix` present. |
| `S1-FINDINGS.md` | `s1/evidence/rung1-builder.log` | EVIDENCE line per rung-1 leg | WIRED | Lines 20 and 43; tool false negative on the caret. |
| `s2/flake.nix` | `s2/collisions.txt` | casecheck derivations read the blob list | WIRED | Pattern present. |
| `s4/tier.nix` | `s4/venue.nix` | tier build runs in a copy of the S1 venue | WIRED | Pattern present. |
| `S5-FINDINGS.md` | `s5/evidence/kernel-provenance.txt` | kernel sha256 for the KVM leg | WIRED | Pattern present; sha256 matches the committed kernel. |
| `s6/build-probe.sh` | `s6/aout.py` | wrap then check the flat payload | WIRED | Both subcommands invoked. |
| `s6/run-arm64-hvf.sh` | `s6/virt-probe.expect` | each leg boots under HVF via expect | WIRED | Pattern present. |
| `s3/flake.nix` | `s3/rio-gate.sh` | writeShellApplication wraps the gate | WIRED | `readFile ./rio-gate.sh` present. |
| `s3/rio-gate.sh` | `s3/rio-gate.expect` | step 5 spawns the expect driver | WIRED | Pattern present. |
| `linux-witness/run-all.sh` | `s1/flake.nix` | builds x86_64-linux S1 derivations | WIRED | `x86_64-linux.s1-` present. |
| `linux-witness/run-all.sh` | `s3/flake.nix` | runs s3-rio-gate in rio then control-noG | WIRED | Pattern present. |
| `linux-witness/run-all.sh` | `s6/linux-x86_64.sh` | reuses the S3 scratch Harvey copy | WIRED | Pattern present. |
| `SPIKE-RESULTS.md` | `S2-FINDINGS.md` | D-04 recommendation cites S2 copy-back | WIRED | Pattern present. |
| `SPIKE-RESULTS.md` | `S1-FINDINGS.md` | machine-change revert list comes from S1 | WIRED | Pattern present. |

**HARN harness links (expected by the full goal) — all absent:**

| From | To | Via | Status |
|------|----|-----|--------|
| `xtask` | rio session | boot + drawterm/serial assertion + non-zero exit | ✗ NOT_WIRED — no rio code in `xtask/src` |
| `xtask` manifest | image assembly | per-file Harvey/Rust source selection | ✗ NOT_WIRED — no manifest or assembler |
| `flake.nix` | Harvey tree | `inputs.harvey` + tree derivation | ✗ NOT_WIRED — no Harvey input |
| `flake.nix` devshell | `xtask` | `R9_HARVEY_DIR_AMD64/ARM64` exports | ✗ NOT_WIRED — zero exports |
| `flake.nix` | Harvey's composed gates | moved `boot-r9*`/`drawterm-gate` apps | ✗ NOT_WIRED — ownership not flipped |

### Data-Flow Trace (Level 4)

Not applicable. Pass 1 produces documentation, spike flakes, shell/expect scripts and evidence logs; it renders no dynamic data and has no UI or data-source path. The pass-2 harness data flow (manifest → per-file source selection → `.paq` bake → boot) does not exist yet; it is part of gaps R3/R4.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| S5 kernel copy is the recorded artifact | `shasum -a 256 spikes/s5/9k10cpu.elf` | `ecdcdaddea8…` matches `S5-FINDINGS.md` and `kernel-provenance.txt` | ✓ PASS |
| S4 tier counts are the pinned Harvey data | `git show bd7a698:build/x86_64/tiers/{tier0-libs,tier1-cmds}.txt` filtered for non-comment lines | 27 and 40, matching `S4-FINDINGS.md` | ✓ PASS |
| The 23 case collisions are the real set at the pin | independent `git ls-tree` + case-fold duplicate scan vs `s2/collisions.txt` | 23/23 exact match (`COLLISIONS-MATCH`) | ✓ PASS |
| All committed shell scripts parse | `bash -n` over every non-work `*.sh` under `spikes/` | no syntax errors | ✓ PASS |
| `aout.py` is importable/compilable | `python3 -m py_compile spikes/s6/aout.py` | success | ✓ PASS |
| All three expect drivers are complete Tcl scripts | `tclsh` + `info complete` | `1` for rio-gate, virt-probe, q35shell-probe | ✓ PASS |
| The amd64 probe contains the marker and syscall sequence | grep + disassembly evidence | marker present; `mov ebp,0x33` and `0x8` with syscall | ✓ PASS |
| Harvey checkout is untouched by the phase | `git -C ../harvey rev-parse HEAD` + status-sha guard | `HARVEY-UNTOUCHED` (25d97dc4d, 041d2929…) | ✓ PASS |
| No protected r9 path changed | `git diff --name-only ce51503..HEAD -- flake.nix flake.lock xtask x86_64 aarch64 riscv64 port lib .github .cargo Cargo.toml Cargo.lock rust-toolchain.toml` | 0 lines; `git status --porcelain` also 0 | ✓ PASS |
| No credential value leaked into evidence | password/secret grep over committed logs | zero unredacted values; redaction markers present | ✓ PASS |
| Every SUMMARY-cited commit exists | `git cat-file -t` over sampled Task/plan hashes | all commits | ✓ PASS |
| The full-goal machinery is absent (negative check) | `xtask` subcommand list + `grep -n "manifest\|rio" xtask/src` + flake input/export grep | no rio/manifest command; no Harvey input; no `R9_HARVEY` export | ✓ CONFIRMS FAILED (R1-R7) |

### Probe Execution

Not applicable. No `probe-*.sh` is declared by the phase, and the S6 "probe" is a Rust a.out artifact under `spikes/s6/`, not a Step-7c probe script. It is covered by the static and evidence checks above.

### Requirements Coverage

All eight HARN requirements are accounted for: seven appear in at least one pass-1 PLAN frontmatter, and **HARN-05 is ORPHANED** — no pass-1 plan claims it. It is nevertheless explicitly named in `SPIKE-RESULTS.md` "Deferred to planning pass 2" ("none; no spike bears on it, so it is planned from research alone") and in `01-VALIDATION.md` as deferred. No pass-1 requirement was implemented, by design (D-22); `REQUIREMENTS.md` correctly keeps all eight unchecked/Pending.

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|----------------|-------------|--------|----------|
| HARN-01 | 01-05, 01-06, 01-07 | One `cargo xtask` command boots to rio and exits non-zero otherwise | ✗ BLOCKED | No rio command/assertion in `xtask/src`; S3 gate is a spike and its positive leg FAILed. |
| HARN-02 | 01-01, 01-02, 01-06, 01-07 | Single read-only resolver over `R9_HARVEY_DIR`/`HARVEY_SRC`/`vendor/harvey/`, all consumers, named error, unmodified after gate | ✗ BLOCKED | `uregcheck::harvey_dir()` is a partial pre-existing base (R9_HARVEY_DIR + named error, consumers routed); no `HARVEY_SRC`/`vendor/harvey`, no after-gate proof. |
| HARN-03 | 01-05, 01-07 | Fallback manifest of every C library object and userland binary per architecture | ✗ BLOCKED | No manifest in `xtask/`; no `xtask/src/image.rs`. |
| HARN-04 | 01-04, 01-06, 01-07 | Swap one manifest entry to Rust, gate passes, swap back | ✗ BLOCKED | Mechanism proven only in S6; no in-tree manifest/assembler/swap. |
| HARN-05 | **none (ORPHANED)** | Named fail-fast on two-way manifest/image drift | ✗ BLOCKED | No drift detection; explicitly deferred by name in SPIKE-RESULTS, no plan claims it. |
| HARN-06 | 01-02 | Existing x86_64 q35 profiles and aarch64 virt quick pass through the reconciled mechanism | ✗ BLOCKED | Profiles exist; no reconciled mechanism and no recorded run through it. |
| HARN-07 | 01-01, 01-02, 01-06, 01-07 | r9 flake pins Harvey and devshell exports per-arch read-only tree | ✗ BLOCKED | `flake.nix` has no Harvey input, no tree derivation, no `R9_HARVEY*` export; CI still checks Harvey out by hand. |
| HARN-08 | 01-03, 01-06, 01-07 | Flake ownership flipped; r9 owns composed gates; Harvey has no r9 input | ✗ BLOCKED | `../harvey/flake.nix` still has `inputs.r9` and owns boot-r9*/drawterm-gate; r9 has no composed outputs. |

### Anti-Patterns Found

No debt markers (`TBD`/`FIXME`/`XXX`), no `TODO`/`HACK`/`PLACEHOLDER`, no empty implementations and no hard-coded empty stubs were found in the phase's changed files. The `XXXXXX` hits are `mktemp` templates, not debt markers. The following are documentation-consistency warnings, not blockers.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `01-01-SUMMARY.md` | 52 | `requirements-completed: [HARN-07, HARN-02]` while the body says "Requirements checkboxes stay Pending" and REQUIREMENTS.md keeps them Pending | ⚠️ WARNING | A consumer reading only SUMMARY frontmatter could wrongly mark HARN-07/02 complete; pass 2 is the closing plan. |
| `01-03-SUMMARY.md` | 53 | `requirements-completed: [HARN-08]` while the body records the work as a read-only feasibility spike | ⚠️ WARNING | Same risk for HARN-08. |
| `01-06-SUMMARY.md` | 72 | `requirements-completed: [HARN-01, HARN-04, HARN-07, HARN-08]` while line 65/170 says the checkboxes stay Pending | ⚠️ WARNING | Same risk for four IDs; the plan's own body and STATE.md say none is implemented. |
| `spikes/s3/evidence/static-analysis.md` | 52 | Stale prose "The runtime confirmation is the Plan 06 KVM leg (`PENDING-OWNER`)" after Plan 06 recorded the leg as FAIL | ⚠️ WARNING | Pass 2 could read this as an unrun leg; the authoritative verdict in S3-FINDINGS is FAIL. |
| `spikes/SPIKE-RESULTS.md` | 333-339 | The pass-wide audit records that Plan 06's owner-approved commits were pushed (`c92a3f4`), while the plan prohibition said "MUST NOT push" | ⚠️ WARNING | The "nothing pushed" truth holds only with this documented owner-approved exception; no Harvey push and no protected-path change. |

### Human Verification Required

**None for pass-1.** The pass-1 deliverables are documentary and their evidence is committed; no visual, real-time or external-service behavior needs a human test to verify *this pass*. The owner already supplied the closing-checkpoint decisions, and no revert was requested.

For **pass 2** (not a verification item here): the owner's Linux KVM host is the required witness for the shipped quick gate (D-13), the generated DEMO password/headless-auth path must be resolved and the rio leg re-run with `control-noG` unchanged (Decision 5), and the Harvey commit plus both pushes need owner approval (HARN-08).

### Gaps Summary

Pass 1 did exactly what D-22 scoped it to do, and did it well: the six spikes are answered (25 legs, 20 PASS, 5 evidence-backed FAILs, 0 unanswered), the assumptions are classified, the owner's seven decisions are recorded verbatim, the machine-change ledger and prohibition audit are complete, and every pass-1 artifact is substantive. I independently reproduced the Harvey guard, the protected-path zero diff, the 23-case collision set, the 27/40 tier counts, the kernel sha256, shell/Python/Tcl syntax, and the credential scan; all agree with the summaries.

But the **phase goal itself is not achieved**. The repository has no boot-to-rio gate, no fallback manifest, no assembler or swap, no `HARVEY_SRC`/`vendor/harvey` resolver, no flake-supplied Harvey tree, and no flake ownership flip. `REQUIREMENTS.md` correctly lists every HARN requirement as Pending, and `01-VALIDATION.md` itself records "8 gaps found … HARN-01..08 all unimplemented by design in pass 1". The next step is therefore to **write pass 2** with `/gsd-plan-phase 1`, using the owner's decisions and the pass-2 backlog in SPIKE-RESULTS as its inputs — not to close these gaps as if pass-1 execution had failed.

---

_Verified: 2026-11-23T03:06:43Z_
_Verifier: the agent (gsd-verifier)_
