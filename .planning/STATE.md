---
gsd_state_version: "1.0"
milestone: v3.0
milestone_name: Cross-Build Harness and Wire Foundations (In Progress)
current_phase: 01
current_phase_name: Stage P0 - Cross-Build Harness
status: executing
stopped_at: Completed 01-06-PLAN.md
last_updated: "2026-11-23T02:46:03.307Z"
last_activity: 2026-11-23
last_activity_desc: Phase 01 plan 06 (Linux KVM witness) completed
state_head: 8ba927ed5f6cc6baacb863bb1592e4f8e0d43c24
progress:
  total_phases: 4
  completed_phases: 0
  total_plans: 7
  completed_plans: 6
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-11-22)

**Core value:** The OS boots to rio on QEMU q35 at every commit.
**Current focus:** Phase 01 — Stage P0 - Cross-Build Harness

## Current Position

Phase: 01 (Stage P0 - Cross-Build Harness) — EXECUTING
Plan: 7 of 7
Status: Ready to execute
Last activity: 2026-11-23 — Plan 02 (S2/A9/S4) completed

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**
- Total plans completed: 2

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 2 | 7 | 5 min |

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P03 | 3 min | 2 tasks | 8 files |
| Phase 01 P04 | 6 min | 3 tasks | 17 files |
| Phase 01 P05 | 5 min | 2 tasks | 8 files |
| Phase 01 P01 | 13 min | 3 tasks | 8 files |
| Phase 01 P02 | 9min | 3 tasks | 15 files |
| Phase 01 P06 | 80min | 3 tasks | 19 files |

## Accumulated Context

### Decisions

Full log is the PROJECT.md Key Decisions table. Recent:

- [Owner 2026-11-22] `RFNOLEGACY` is a fourth `rfork` flag, `1 << 21`, assigned in guide Phase 0 (GSD Phase 2) with `RFCKEY` 18, `RFROBASE` 19, `RFRANDMEM` 20. Resolves warning W11; the guide's "3 flag bits" wording is corrected to four. Reserved in r9 only (`x86_64/src/proc.rs`, `aarch64/src/syscall.rs`); Harvey is not edited. Recorded deviation from the guide's "reserve bits 18-20 in `libc.h`". Phase 6 implements the behaviour.
- [Owner 2026-11-22] The r9 nix flake supplies the read-only Harvey tree to `cargo xtask` through the `nix develop` devshell (HARN-07, Phase 1); xtask keeps no built-in default path.
- [Owner 2026-11-22] Phase 1 flips flake ownership (HARN-08): r9's flake imports Harvey and produces the composed outputs Harvey's flake owns today; Harvey's flake stops importing r9. Harvey's `flake.nix` is edited for this; the Harvey commit and pushes need owner approval.
- [Owner 2026-11-22] Phase 1 is planned in two passes (CONTEXT D-22). Pass 1 (plans 01-01 to 01-07) covers spikes S1 to S6 and an owner checkpoint only. Coverage gates overridden for pass 1 on that basis: HARN-05 and decisions D-02, D-06, D-07, D-09, D-10, D-12, D-14, D-15 are not yet in any plan, and no HARN requirement is implemented until pass 2. Re-run `/gsd-plan-phase 1` after the spike results to write pass 2.
- [Roadmap] Phase order is the owner's migration plan; Stage P0 is Phase 1; "Milestone S" and "Milestone STD" are source labels for Phases 8 and 9.
- [Ingest] Precedence IG > AC > RFC > PT. Nothing is locked; the guide's "non-negotiable" decisions are constraints pending confirmation in Phase 2.
- [Phase 01]: S5 kernel provenance: the Mac copy of Harvey's 9k10cpu.elf has sha256 ecdcdad... and an unknown source commit; the KVM witness pins the sha256, not a commit.
- [Phase 01]: S5 harness: one boot-q35.sh serves the Mac TCG matrix and the owner's KVM witness; KVM without a writable /dev/kvm exits S5-ERROR kvm-unavailable, never a silent TCG downgrade (D-13).
- [Phase 01]: S5 D-20 minimum scope: the evidence supports a Harvey app that builds and boots to the root is from prompt; an rc prompt via Harvey's own u9fs on TCP 564 is the next increment.
- [Phase 01]: S5 pc baseline reproduces the recorded 2026-11-05 behaviour, not the exact artifact; the kernel was rebuilt on 2026-11-18 and no string proves its commit.
- [Phase 01]: S6 D-17 target is `ps`, not D-17's prose example `echo`: ps is staged in every chain-family x86_64 image, absent from the boot path, and has two gate sites; `echo` is not an rc builtin (`sys/src/cmd/rc/plan9.c:35-48`), so every bare echo in cpurc and the smoke gates would run the replacement.
- [Phase 01]: S6 arm64 swap proven end to end under HVF: control, probe marker and flip-back all PASS, and the flip-back bootfs.paq is byte-identical to control (`746f1dfe...`). The x86_64 KVM leg is scripted and statically checked but PENDING-OWNER for Plan 06.
- [Phase 01]: S6's aarch64 bakes needed a scratch-only mkpaqfs shim (pin mtimes, header time and trailer SHA1) because Harvey's aarch64 mkimage has no KERNDATE pin and the prebuilt darwin mkpaqfs predates it; Harvey stayed untouched.
- [Phase 01]: S3 rio mode waits on the serial console rc prompt + a rio budget, not a drawterm shell prompt: the cpu profile ends in `exec rio`, so rc never prints one and rio writes nothing to the pty; the driver then types `ls /srv` on the serial console.
- [Phase 01]: S3 dials as glenda (S3_DT_USER), not the creds-file probe user norbert: only /usr/glenda/lib/profile is staged, rcmain sources a profile only for a readable $home/lib/profile, and the DEMO domain shares one password so the parsed password authenticates glenda.
- [Phase 01]: The S3 flake re-exports Harvey's locked r9 116bcc2 packages and resolves them at run time with `nix build path:$S3_FLAKE#<attr>`: r9's kernel derivation has an import-from-derivation (rust-src Cargo.lock) that cannot build on aarch64-darwin, so eval-time interpolation would fail the plan's Mac check.
- [Phase 01]: determinism.sh refuses a first bake whose log records a KERNDATE other than the requested one; Plan 06 passes KERNDATE=1795382101 to both gate runs so an all-DIFFERENT result cannot be blamed on the clock.
- [Phase 01]: S3 static verdicts: A5 PASS (kernel-wide static srv table at devsrv.rs:59; #s bound at /srv for init's console rc and the cpu session) and A6 PASS (drawterm.nix builds Make.unix GUI=x11); the three KVM legs and A12 stay PENDING-OWNER for Plan 06.
- [Phase 01]: S1 venue: the owner chose the existing lima VM harvey; unshare -Ur -m is refused on it (nested uid_map EPERM inside the nix sandbox, Ubuntu 24.04 AppArmor userns restriction outside it) and the same guest script passes through proot, so A1 is REFUTED for unshare and D-03 stays viable on the proot venue.
- [Phase 01]: S1 host toolchain: nix/setup.sh + nix/toolchain.sh port to a sandbox with four adjustments - store-bash shebang for the 9l shim, pkgs.ed (mkcname), plan9port last on PATH (its 9p mount must not shadow util-linux mount), and GNU sed shadowing for 7l/BUILD's malformed _SYSRM line at the pin.
- [Phase 01]: S1 measurement: the amd64 libc tier builds in 2 s inside the sandbox (259 members, 601170 bytes) and nix store verify --no-trust is clean on the output; A8's tens-of-minutes belongs to the full tier build, not the library tier.
- [Phase 01]: S1 flake: pins Harvey bd7a698 (flake = false) and r9's nixpkgs rev 1c89083e, exposes s1-nsprobe, s1-nsprobe-proot, s1-hostbin and s1-libc-amd64 for x86_64-linux and aarch64-linux; the x86_64 witness legs stay PENDING-OWNER for Plan 06.
- [Phase 01]: S2: both source modes preserve the 23 case-colliding paths; the pass-2 tree derivation should fetch Harvey on the builder through a fixed-output fetch (rev + SRI) so the source never round-trips through APFS
- [Phase 01]: S2 copy-back: the Mac system store refuses nix copy --from as the owner's user (require-sigs=true, trusted-users=root); the case mechanism itself is sound (same-volume chroot store verify clean, 23/23 blobs), so pass 2 needs a remote-builder config or an owner-approved store-policy change
- [Phase 01]: A9: a context-discarded outPath in the darwin devshell is lazy (no build sent) and equals the later build path, so HARN-07's supply can meet D-05
- [Phase 01]: S4: the amd64 tier 0 (27 libs, 6 s) and tier 1 (40 cmds, 19 s, all S_MAGIC 00008a97) build on the aarch64-linux builder with two venue ports: /rc/bin on PATH for membername and an explicit INCLUDE list for a latent 6c heap-layout failure under proot; A3/A4 CONFIRMED, A8 REFUTED
- [Phase 01]: On APFS a bare [ -f ] lookup resolves the case twin; Mac-side re-checks must enumerate exact names, and tree outputs read by xtask must not contain colliding names
- [Phase 01]: Plan 01-06 witness verdicts: PASS s1 rung2 (proot libc 601170 B/259 members), s3 control, s5 q35 KVM (kernel ecdcdad matched), s6 x86_64 (control/probe/flipback sha-equal=yes; probe db94057f matched); FAIL s1 rung1 (unshare /nix/store bind, exit 32), s3 rio (tr-ok then password-feed timeout, no S3-RIO-SERVING), s3 determinism (bake2 aborted, hsecstored missing).
- [Phase 01]: A1 scoped: unshare is REFUTED on both the lima aarch64 builder and the x86_64 witness, but proot rung 2 PASSes on both; pass 2's tree derivation must use proot and not assume unshare.
- [Phase 01]: A12 REFUTED: the second mkimage bake aborts at the auth bake (hsecstored missing from the computed auth-suite set), so no image reached a determinism comparison; resolve the auth-suite resolver before planning byte-diff tests.
- [Phase 01]: A5/A6/A7/A11 CONFIRMED: the -G control's serial ls /srv shows no rio entry, drawterm runs under Xvfb and authenticates, the amd64 a.out probe prints R9-RUST-AOUT-PROBE-OK, and Harvey's kernel reaches the q35 KVM prompt with no signature.
- [Phase 01]: Gate teardown finding: drawterm PID 88800 survived the rio-gate timeout; ps shows the store path, not the r9s3riogate-dt-* argv0 tag, so pass 2 must reap by scratch/dial identity or the pty child pid.
- [Phase 01]: HARN-01/04/07/08 stay pending in REQUIREMENTS.md: pass 1 is spikes only under the D-22 override; the 01-06 summary lists them for traceability but mark-complete was not run.

### Open Questions (ten warnings, approved to be carried, not resolved)

Settle each when its phase is planned. Detail: `.planning/INGEST-CONFLICTS.md`. Eight roadmapper notes (N1-N8) are in PROJECT.md.

| ID | Settle in | Question |
|----|-----------|----------|
| W3 | Phase 3 | MAC primitive: Keyed BLAKE3 only, or where Poly1305 applies (also read by Phase 4) |
| W5 | Phase 3 | `unsafe` in `ds/src/tpht/` versus `forbid(unsafe_code)` on `ds/` |
| W6 | Phase 3 | TPHT home block 4 versus 16 slots |
| W1 | Phase 4 | Wire header layout and AAD contents (`Pad`, `ExtTLVs Length`) |
| W2 | Phase 4 | 24-byte `StatelessCap` in an 8-byte `FidHandle` field |
| W4 | Phase 4 | Session key scope: per FID versus per connection and direction |
| W7 | Phase 6 | `DiscoveryBeaconV2` 192 versus 202 bytes |
| W8 | Phase 13 | `/adm/users` UID 0 `adm` versus persona `glenda` |
| W9 | Phase 13 | Test 3.2 needs `alice` absent from `/adm/users` |
| W10 | Phase 13 | Suite 3 cross-VLAN mounts with no route |

### Pending Todos

None yet.

### Blockers/Concerns

- Three commits are unpushed by the owner's choice: r9 `5b8b70a` (planning documents plus `legacy/`, 53 MB) and `6ee45b5` (handoff note), and Harvey `25d97dc4d` (freeze note). Confirmed by git at roadmap time. The orchestrator's planning-bootstrap commit is also local until the owner approves a push. The owner has not decided whether `legacy/` stays in r9 history.
- Owed checks at checkpoint r9 `116bcc2`: the owner's linux `nix build .#boot-iso --rebuild` (expected `sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=`), and hosted CI on r9 `116bcc2` and Harvey `bd7a6982a`.
- Phase 20 of the Harvey-rooted work never ran `/gsd-secure-phase` (no SECURITY.md).
- Docs pin r9 at `c787ce9`; HEAD is 60 commits later with no `crypto/`, `ds/`, `kern/`, `sys/`, `venti/`. Re-verify every landing-site path when its phase is planned.
- S6 x86_64 KVM leg: scripted and statically checked, PENDING-OWNER. Plan 06 must run `spikes/s6/linux-x86_64.sh` on the x86_64 Linux host (writable `/dev/kvm`, the S3 scratch Harvey tree, the r9 clone at `116bcc2`) before the pass-1 checkpoint.
- Mac store copy-back: nix copy --from ssh-ng://lima-harvey as the owner's user is refused (require-sigs=true, trusted-users=root). Pass 2 needs a remote-builder configuration or an owner-approved store-policy change; the case names themselves round-trip cleanly in a same-volume chroot store.

### Standing Owner Rules

- Pushes in either repo need explicit owner approval; check what is unpushed and push only the approved commits.
- Edits under r9 `port/src` need owner approval (show the diff first). `aarch64/`, `x86_64/`, `xtask/` edits do not.
- x86_64 kernel byte change: re-derive and commit the `boot-iso` `outputHash`, run the x64 TCG gates in an amd64 Docker container (clean up afterwards), and get the owner's linux boot-iso check.
- aarch64 kernel byte change: quick pass now (tier-1 on HVF and TCG; smoke, net-smoke, cpu once on HVF); full HVF series at Phase 8. Kernel builds are reproducible, so compare kernel sha256 to prove a pin move changes nothing.
- Kernels are built without FP/SIMD; kernel SIMD goes through `fpu::kernel_begin()` / `with_simd()`; user FP state is per process (save at switch, copy on fork, reset on exec); arm64 is the reference for x86_64.
- No AI attribution in commits or PRs. Never stage the 11 pre-modified troff/postscript font files in Harvey (a darwin case-collision artifact).

### Practical Notes

`gh` is not installed (`nix run nixpkgs#gh -- ...`). GNU sed is on PATH. macOS Unix socket paths cap at 104 bytes: run boot tests from a short path. The lima VM `harvey` is the Harvey namespace build venue (`core.ignorecase=false` on purpose). r9 exec caps each argument at 128 bytes; the PL011 console buffer is 256 bytes. Debug history: `.planning/legacy/debug/knowledge-base.md` (frozen).

## Deferred Items

Items carried over from the legacy-final checkpoint (HANDOFF.md):

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| Verification | Full arm64 HVF series (smoke 1753, net-smoke 36, tier-1 10, cpu 36, plus 3 TCG each; about 4.4 h) | Deferred by owner to Phase 8 (SMP-05) | legacy-final | v3.2 |
| Defect | WINDOWS #70: aarch64 scheduler resume skips note delivery | Deferred to Phase 8 (SMP-06) | legacy-final | v3.2 |
| Defect | WINDOWS #49: I-cache, root-caused and fixed | Left open until the full series runs (SMP-06) | legacy-final | v3.2 |
| Defect | WINDOWS #66: x86_64 triple fault under 8-way soak | Fix on an x86_64 host venue (SMP-06) | legacy-final | v3.2 |
| Test gap | `#!` recursion cap has no host or live test | Unscheduled; add a test when exec is next touched | legacy-final | - |
| Prerequisite | SMP: aarch64 per-CPU current frame, kernel stack, proc pointers; x86_64 per-CPU `Mach` mapping (FP ownership already per-CPU) | Phase 8 (SMP-02) | legacy-final | v3.2 |

The WINDOWS ledger (51 open, 4 waived, 17 fixed, 72 total) is frozen at `.planning/legacy/WINDOWS.md`; re-home open entries to a live file before updating them.

## Session Continuity

Last session: 2026-11-23T02:46:03.284Z
Stopped at: Completed 01-06-PLAN.md
Resume file: None
