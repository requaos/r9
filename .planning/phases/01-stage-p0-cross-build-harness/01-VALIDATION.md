---
phase: "1"
slug: "stage-p0-cross-build-harness"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-22"
validated: "2026-11-23"
pass2_seeded: "2026-11-23"
pass2_validated: "2026-11-23"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
>
> **Two-pass scope (D-22).** Phase 1 was planned in two passes. Pass 1 (plans 01-01 to 01-07)
> answered the spike questions S1–S6 and ended at an owner decision checkpoint; it implemented
> **no HARN requirement**. The pass-1 rows below are historical evidence and were deliberately
> manual-only, as recorded by the pass-1 audit. Pass 2 (plans 01-08 to 01-34) implements
> HARN-01 to HARN-08, and this revision records that implementation's verification.
>
> **This revision (2026-11-23, pass 2).** Every pass-2 row was re-audited against the code,
> the tests and the committed witness evidence. The agent ran every command it could run on the
> Mac (host unit tests, image/reference comparisons, devshell/lock checks, static gate checks,
> the whole aarch64 quick pass and three aarch64 boot profiles). Commands that are KVM-only by
> design (D-13: the owner's x86_64 Linux host is the required witness) are recorded as manual
> with their witness evidence paths — never as green from an unrun command. No test was written
> against duplicated logic; no requirement was weakened.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in tests through `cargo xtask test` (port + aarch64 host legs) and `cargo test -p xtask` (the harness itself); expect/Tcl boot gates; shell gate scripts; nix flake evaluation checks |
| **Config file** | `xtask/manifests/<amd64,arm64>/*.toml` (the image/manifest fixtures); no test config file |
| **Quick run command** | `nix develop -c cargo test -p xtask && nix develop -c cargo xtask test` |
| **Full suite command** | quick, then `cargo xtask clippy` and `cargo xtask check`; every image through `cargo xtask image … --compare-reference`; the aarch64 quick pass; the owner's KVM quick gate (D-13) |
| **Estimated runtime** | measured 2026-11-23: `cargo test -p xtask` 185 tests ≈ 4 s; `cargo xtask test` 395 tests ≈ 2 s; Mac boot gates 13–58 s each (five aarch64 gates ≈ 3 min); KVM quick gate minutes (stop-E ran two gate runs in 394 s) |

**Why the quick command has two halves.** `cargo xtask test` runs the `port` and `aarch64`
host legs only; it does **not** run the `xtask` crate's own 185 tests (the resolver, image
engine, drift/provenance and gate logic). The pass-2 plans' `<automated>` blocks use
`cargo test -p xtask …` explicitly, so no coverage was lost in execution — but the suite
command must include it, and this row is a recorded warning (see W-1).

---

## Sampling Rate

- **After every task commit:** `nix develop -c cargo test -p xtask` and `nix develop -c cargo xtask test`
- **After every plan wave:** the full suite (quick + `clippy` + `check`), plus the aarch64 quick pass on the Mac where the change touches the aarch64 supply/gates
- **Before `/gsd-verify-work`:** full suite green, and the owner's KVM quick gate green in both directions (positive and `--force-no-rio`) at the phase-boundary commit
- **Owner witness cadence (D-12/D-13):** the local x86_64 KVM run is the witness at each owner stop (A/A2, B/B2/B3, C/C2/C3, E) and at every Harvey pin bump; hosted CI is the best-effort second witness
- **Max feedback latency:** measured host suite ≈ 4 s; Mac boot gates ≤ 60 s each; KVM gate minutes (stop-E: 394 s for the two positive gate runs)

---

## Per-Task Verification Map

### Pass 1 (plans 01-01 to 01-07; historical, D-22)

Plan-level rows for pass 1 (the plans were spike investigations; a per-task automated map is a
pass-2 artifact).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01/T1-T3 | 01-01 | 1 | HARN-07, HARN-02 | — | Read-only Harvey tree; no sudo | manual (spike) | none — pass 1 | n/a | ✅ completed (evidence: `spikes/S1-FINDINGS.md`) |
| 01-02/T1-T3 | 01-02 | 2 | HARN-07, HARN-06 | — | Read-only Harvey tree; no sudo | manual (spike) | none — pass 1 | n/a | ✅ completed (`spikes/S2-FINDINGS.md`, `S4-FINDINGS.md`) |
| 01-03/T1-T2 | 01-03 | 1 | HARN-08 | — | Read-only Harvey tree; no sudo | manual (spike) | none — pass 1 | n/a | ✅ completed (`spikes/S5-FINDINGS.md`) |
| 01-04/T1-T3 | 01-04 | 1 | HARN-04 | — | No protected-path edits | manual (spike) | none — pass 1 | n/a | ✅ completed (`spikes/S6-FINDINGS.md`) |
| 01-05/T1-T2 | 01-05 | 1 | HARN-01, HARN-03 | — | Redacted auth captures; no sudo | manual (spike) | none — pass 1 | n/a | ✅ completed (`spikes/S3-FINDINGS.md`) |
| 01-06/T1-T3 | 01-06 | 2 | HARN-01, HARN-04, HARN-07, HARN-08 | — | Root-refusing, sudo-free witness driver; credential scan | manual (witness) | none — pass 1 | n/a | ✅ completed (`spikes/linux-witness/`) |
| 01-07/T1-T3 | 01-07 | 3 | HARN-07, HARN-08, HARN-04, HARN-01 | — | No pushes; prohibition audit | manual (closing) | none — pass 1 | n/a | ✅ completed (`spikes/SPIKE-RESULTS.md`) |

*Status: ⬜ pending · ✅ green · ❌ red · 📋 manual · ⚠️ caveat*

### Pass 2 (plans 01-08 to 01-34; implementation and owner witness)

Venue: **Mac** = the executor's aarch64 Mac; **OWNER-LINUX** = the owner's x86_64 KVM host
(never verifiable from the Mac). Commands are abbreviated; each plan's `<verify>` block holds
the exact form. The **Audit** column says what was actually done in this revision: `ran` = the
agent executed the command and it passed; `evidence` = the committed witness log is the
authority; `manual` = owner-only by design, with the evidence path.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Venue | Audit / Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------|----------------|
| 01-08/T1 | 01-08 | 4 | HARN-03, HARN-06, HARN-07 | T-01-27 | Staging under `target/` only; path validation | unit + boot | `cargo test -p xtask image::` (46); `cargo xtask boot-test --arch aarch64 --config virtexec --accel hvf` | Mac | ✅ **ran**: 46 tests pass; virtexec PASS (13 s) |
| 01-08/T2 | 01-08 | 4 | HARN-02, HARN-07 | T-01-28, T-01-29 | No default path; named error; realise checks printed path | unit + grep | `cargo test -p xtask harvey::` (14); direct-env-read grep == 0; `cargo xtask ureg-check` named error | Mac | ✅ **ran**: 14 tests pass; no-env ureg-check exit 1 naming all three variables; source invariant test passes |
| 01-09/T1-T3 | 01-09 | 4 | HARN-01 | T-01-30..33 | Password never on argv/logs; root-refusing driver | static | `shellcheck` on `rio-derisk/*.sh`; Tcl `info complete`; argv grep == 0 | Mac | ✅ **ran**: shellcheck 0 findings; Tcl 1; `DRAWTERM.*password=` count 0; the rio leg itself is 📋 manual (stop-A FAIL → stop-A2 PASS) |
| 01-10/T1 | 01-10 | 4 | HARN-07, HARN-08 | T-01-34 | Only pushed Harvey revs locked | lock + build | `jq .nodes.harvey.locked.rev flake.lock`; `nix build .#checks.aarch64-darwin.harvey-harness`; both `harvey-drawterm.drvPath` evals | Mac | ✅ **ran**: `c697666ba315…`; harness check builds (`/nix/store/v1anrpz…-harness-0.1.0`); both drawterm drvPaths evaluate; no `r9` node and no `116bcc2` |
| 01-10/T2 | 01-10 | 4 | HARN-07 | T-01-35, T-01-36 | Builder key root-only, loopback alias | doc | doc-key loop (seven builder keys) | Mac | ✅ **ran**: `DOC-KEYS-OK` (ssh-ng, substitutes, determinate auth, override-input, no-write-lock-file, ls-remote, r9-builder-probe) |
| 01-11/T1-T2 | 01-11 | 5 | HARN-03, HARN-05 | T-01-37..39 | Two-way drift; Rust provenance; symlink escape | unit + compare | `cargo test -p xtask image::`; `cargo xtask image --arch x86-64 --image <n> --compare-reference` | Mac | ✅ **ran**: drift/provenance/symlink tests pass; reference compare passes (negative compare exits 1 naming the changed member) |
| 01-12/T1-T2 | 01-12 | 5 | HARN-01, HARN-07 | T-01-40..42 | Redacted evidence; stop on rio FAIL | witness | builder probe (Mac) + stop-A steps | Mac + OWNER-LINUX | 📋 **manual**: `witness/evidence/stop-a/summary.tsv` — mac-builder PASS, `rio` exit 1 (the recorded FAIL), control/a13 PASS; closed by 01-32/A2 |
| 01-13/T1 | 01-13 | 6 | HARN-07, HARN-02, HARN-06 | T-01-43..46 | FOD source; case-clean tree; lazy export | build + boot | `nix build .#packages.aarch64-linux.harvey-tree-arm64`; store verify; lazy path match; virtexec boot | Mac | ✅ **ran**: devshell export == eval outPath; `nix store verify` clean; virtexec PASS with nothing set by hand |
| 01-13/T2 | 01-13 | 6 | HARN-07 | — | — | doc | contract-doc key grep | Mac | ✅ **ran**: the seven contract keys present (3–7 occurrences each across `HowItWorks.md` and `docs/harvey-supply.md`) |
| 01-14/T1-T2 | 01-14 | 7 | HARN-07, HARN-03 | T-01-47..49 | No key material in store; case-clean | build + unit | `cargo xtask image --arch x86-64 --libraries`; `cargo xtask ureg-check`; tree verify | Mac | ✅ **ran**: 27-row library table OK, ureg-check 35/26 fields, tree store-verify clean; tree build evidence `01-14-SUMMARY` (TREE-AMD64 27/27…118/118) |
| 01-15/T1-T2 | 01-15 | 8 | HARN-03, HARN-05 | T-01-50, T-01-51 | Reference output credential-free | build + compare | `cargo xtask image --image marker\|chain\|substrate --compare-reference` | Mac | ✅ **ran**: 30 / 49 / 27 members, all match |
| 01-16/T1-T2 | 01-16 | 9 | HARN-03, HARN-01, HARN-06 | T-01-52..54 | 0600 creds; no password in output | unit + compare | `cargo test -p xtask image::authdomain*`; `cargo xtask image --image cpu\|auth --compare-reference` | Mac / OWNER-LINUX | ✅ **unit ran** (4 authdomain tests); comparison is 🟢 **evidence**: `stop-e/cpuauth-ref.log` cpu 4442 / auth 95 match. Mac cannot run `authdomain-gen` (Linux-only) and fails with the named `R9_AUTHDOMAIN_GEN is not set` error — an environment limit, not a defect |
| 01-17/T1-T3 | 01-17 | 9 | HARN-03, HARN-05, HARN-06, HARN-07 | T-01-55..57 | Case-clean arm64 tree | build + compare + boot | `cargo xtask image --arch aarch64 --image bootfs --compare-reference`; `cargo xtask image --arch aarch64 --libraries`; `cargo xtask boot-test --config virtnet\|virtip --accel hvf` | Mac | ✅ **ran**: bootfs 151 members match; 35 libraries OK; virtnet and virtip PASS (plus virtexec) |
| 01-18/T1 | 01-18 | 10 | HARN-01, HARN-08 | T-01-58, T-01-59 | Password by 0600 file path only | static | `shellcheck`; Tcl; `nix eval .#apps.x86_64-linux.drawterm-gate-x64.program` | Mac | ✅ **ran**: shellcheck/Tcl clean; app evals to a store path |
| 01-18/T2 | 01-18 | 10 | HARN-01, HARN-02 | T-01-60..62 | No silent TCG downgrade; tree-unmodified check | unit + venue | `cargo test -p xtask gate::` (10); `cargo xtask gate --arch x86-64` on the Mac must refuse | Mac | ✅ **ran**: 10 tests pass; Mac run exits 1 with `GATE-VENUE not-linux`; the KVM positive/negative runs are 📋 `stop-e/gate.log` + `gate-neg.log` |
| 01-19/T1-T2 | 01-19 | 10 | HARN-03, HARN-05 | T-01-63, T-01-64 | No compilation in the assembler | compare | `cargo xtask image --image net\|services\|tier\|churn --compare-reference` | Mac | ✅ **ran**: 64 / 4517 / 91 / 52 members, all match |
| 01-29/T1 | 01-29 | 10 | HARN-06 | T-01-93 | Profiles re-pointed only at reference-matched images | compare + build | `cargo xtask image --image marker\|chain\|substrate --compare-reference`; `cargo xtask build --config <five>`; `grep 'module = "image:'` | Mac | ✅ **ran**: three compares match; five builds exit 0; every profile names `image:` and no `harvey:` module remains |
| 01-20/T1-T3 | 01-20 | 11 | HARN-04, HARN-05 | T-01-65, T-01-66 | Provenance rule; probe removed | unit + member diff | `cargo test -p xtask image::` (rust/provenance tests); flip diff == `/amd64/bin/ps` only; flip-back `cmp` identical; remnant grep == 0 | Mac + OWNER-LINUX | ✅ **unit ran** (provenance/rust tests pass; no `source = "rust"` entry remains); the end-to-end flip is 📋 **manual**: `stop-b3/probe.log`, `flipback.log`, `summary.tsv` |
| 01-21/T1-T3 | 01-21 | 11 | HARN-08, HARN-06 | T-01-67..69 | No writes under `$HARVEY` | static + boot | `shellcheck`; Tcl; `nix run .#boot-r9-tier1`; `R9_ACCEL=tcg nix run .#boot-r9-tier1`; `nix run .#boot-r9-smoke` | Mac | ✅ **ran**: shellcheck/Tcl clean; tier-1 HVF PASS, tier-1 TCG PASS, smoke PASS |
| 01-22/T1-T3 | 01-22 | 12 | HARN-01, HARN-04, HARN-06, HARN-02, HARN-07 | T-01-70..73 | Verdicts only from returned rows; boot-iso FOD | witness | witness `steps trees, kernel, gate×2, gate_neg, probe, flipback, profiles` | OWNER-LINUX | 📋 **manual**: stop-B/B2 failed and were superseded; **stop-B3** is green for gate, gate-neg, probe, flipback, kernel (`stop-b3/summary.tsv`, all exit 0); profiles/cpuauth PASS in `stop-b2` |
| 01-30/T1-T2 | 01-30 | 12 | HARN-08, HARN-06 | T-01-94..96 | No writes under `$HARVEY` | static + boot | `shellcheck`; Tcl; five aarch64 app evals; the five-app quick pass | Mac | ✅ **ran**: shellcheck/Tcl clean; five apps eval; **all five gates PASS** today (smoke, tier1 HVF+TCG, net-smoke, cpu) |
| 01-23/T1-T2 | 01-23 | 13 | HARN-08 | T-01-74..76 | — | static | `shellcheck`; Tcl; four app evals | Mac | ✅ **ran**: shellcheck/Tcl clean; the four apps eval; the KVM stage-list match is 📋 `stop-e/full.tsv` |
| 01-24/T1-T2 | 01-24 | 14 | HARN-08 | T-01-78, T-01-79 | Net gates write only to `R9_GATE_OUT`; host-echo verbatim | static | `shellcheck`; Tcl; four net app evals; `cmp` host-echo against the pin | Mac | ✅ **ran**: clean; four apps eval; `HOST-ECHO-VERBATIM`; soak/stress KVM is 📋 `stop-e/soak.log`, `stress.log` |
| 01-25/T1-T2 | 01-25 | 14 | HARN-07, HARN-08 | T-01-80, T-01-81 | Actions pinned by sha | lint | `nix run nixpkgs#actionlint` on both workflows; no Harvey checkout in CI | Mac | ✅ **ran**: actionlint exit 0; 0 unpinned actions; 0 `R9_HARVEY_DIR`/Harvey checkout in `xtask.yml`; 6 relocated legs + 1 TCG quick gate. ⚠️ hosted `quick-gate-tcg` red (best-effort, D-13) |
| 01-26/T1-T2 | 01-26 | 15 | HARN-08, HARN-02 | T-01-82..85 | Owner checkout untouched | scratch validation | scratch lock has no r9; no forbidden outputs; q35 `--no-build` prompt; `HARVEY-UNTOUCHED` | Mac / OWNER-LINUX | 📋 **manual**: `01-26-SUMMARY` + `harvey-flip/HARVEY-FLIP.md`; the scratch q35 run is `Q35-RESULT banner=yes prompt=yes sigs=none` (TCG); owner checkout guard held |
| 01-31/T1-T2 | 01-31 | 15 | HARN-08 | T-01-77, T-01-97, T-01-98 | Creds by 0600 file path; password never on argv or in env by value | static | `shellcheck`; Tcl; by-value password grep == 0; all 13 x86_64 gate/helper app evals; 15 drivers present | Mac | ✅ **ran**: shellcheck/Tcl clean; by-value grep 0; all 13 apps eval; 15 expect drivers complete; KVM boot/auth are 📋 `stop-e/full.tsv` |
| 01-32/T1-T3 | 01-32 | 6 | HARN-01 | — | Diag separates login from rio startup | witness | stop-A2 steps (`rio-diag`, `rio`) | OWNER-LINUX | 📋 **manual**: `stop-a2/summary.tsv` both exit 0; `rio-diag.log` `RIO-DIAG login-ok` + `RIO-SERVING`; `serial-tail.log` carries `/srv/rio.glenda.76` + `/srv/riowctl.glenda.76` |
| 01-33/T1-T3 | 01-33 | 15b | HARN-01, HARN-04, HARN-06, HARN-07 | — | Driver registration; kernel/reference preconditions | unit + witness | `cargo test -p xtask gate::tests::driver_fast_fail_registers_after_the_qemu_spawn`; `…driver_final_teardown_cannot_mask_a_passing_leg` | Mac + OWNER-LINUX | ✅ **ran**: both tests pass; stop-B3 gate both directions green (📋 `stop-b3/gate.log`, `gate-neg.log`) |
| 01-34/T1-T3 | 01-34 | 17b | HARN-08, HARN-02 | — | Correct per-app capture dirs; tap venue; fpprobe fix | unit + witness | `cargo test -p xtask gate::tests::full_tier_output_dirs_match_the_scripts_and_witness_map`; `…full_capture_loop_covers_only_the_full_tier` | Mac + OWNER-LINUX | ✅ **ran**: both tests pass; stop-C2 full 8/8, stop-C3 soak/equiv/kernel/store PASS (📋 `stop-c2/full.log`, `stop-c3/equiv/report.tsv`). ⚠️ the x86_64 `sleep_and_alarm…` test has no runnable command (W-2) |
| 01-27/T1-T3 | 01-27 | 16 | HARN-08, HARN-02 | T-01-86..89 | Explicit-path staging; no attribution; no push | witness | stop-C steps (`full, soak, stress, equiv, store_verify`) | OWNER-LINUX + Mac | 📋 **manual**: stop-C FAILED and is superseded by 01-34's C2/C3 (recorded in `WITNESS-LOG.md`); no Harvey change was made before approval |
| 01-28/T1-T3 | 01-28 | 17 | HARN-08, HARN-01, HARN-02, HARN-06, HARN-07 | T-01-90..92 | Lock only pushed revs; lock-only commit | lock + compare + witness | `jq .nodes.harvey.locked.rev` (c697666); no nested `r9`; all images compare; stop-E steps; audit R1–R7 | Mac + OWNER-LINUX | ✅ **ran**: lock/commit/lock-graph checks pass; images compare (7/9 live + cpu/auth KVM evidence); **stop-E 11/11 PASS** (📋 `stop-e/summary.tsv`, `WITNESS-LOG.md` R1–R7 audit) |

---

## Wave 0 Requirements

- [x] S1: userns and chroot build venue inside a nix sandbox — **FAIL on the witness** (unshare bind of `/nix/store`); **PASS** with proot. `spikes/S1-FINDINGS.md`
- [x] S2: Mac builder path with the real Harvey input and its 23 case-colliding paths — **PASS** (23/23 blobs); copy-back refused by signature policy. `spikes/S2-FINDINGS.md`
- [x] S3: headless rio proof on KVM — **control PASS, positive leg FAIL**; determinism **FAIL** (bake 2, `hsecstored`). `spikes/S3-FINDINGS.md`
- [x] S4: amd64 tier build on an aarch64-linux host — **PASS** (tier 0 27/27, tier 1 40/40). `spikes/S4-FINDINGS.md`
- [x] S5: Harvey own-kernel q35 launch to a prompt — **PASS** on Mac TCG and x86_64 KVM. `spikes/S5-FINDINGS.md`
- [x] S6: Rust `a.out` probe boots and prints under the r9 loader — **PASS** (arm64 HVF, amd64 static, x86_64 KVM). `spikes/S6-FINDINGS.md`
- [x] `xtask/src/harvey.rs` (14 tests) and `xtask/src/image.rs` (46 tests), the 13 per-image manifests, and the reference member lists — **pass 2 delivered**: the lists come from the `harvey-reference-amd64` / `harvey-reference-arm64` derivations and are compared live (D-29), not committed hash fixtures. Measured 2026-11-23: all compares pass except cpu/auth, which are Linux-only and carry owner KVM evidence.

---

## Manual-Only Verifications

These behaviours are manual **by design**, not because a test is missing. D-13 makes the owner's
x86_64 KVM host the required witness for the quick gate, and D-18 removes the swap probe after
its proof. Every row carries committed evidence; a row is manual, not green-by-assertion.

| Behavior | Requirement | Why Manual | Evidence (committed) |
|----------|-------------|------------|----------------------|
| Quick gate boots q35 x86_64 to rio and exits 0 only then; `--force-no-rio` exits non-zero with `GATE-FAIL rio-not-serving` | HARN-01 | KVM + GRUB ISO; no KVM/grub on the Mac (D-13) | `witness/evidence/stop-e/gate.log` (2 runs, `GATE-PASS rio-serving` + `GATE-PASS control-no-rio`, both exit 0) and `stop-e/gate-neg.log` (`GATE-FAIL rio-not-serving` → `GATE-NEG-EXPECTED-FAIL`) |
| Headless rio login/feed fixed; the D-28 pair present | HARN-01 (D-28/D-30) | X11 drawterm login inside the KVM guest | `witness/evidence/stop-a2/rio-diag.log` (`RIO-DIAG login-ok`, `RIO-SERVING`), `stop-a2/serial-tail.log` (`/srv/rio.glenda.76` + `/srv/riowctl.glenda.76`) |
| One manifest entry swaps to Rust, the gate passes at the flip commit, and flip-back restores the Harvey bytes | HARN-04 (D-17/D-18) | The probe was removed after the proof; no Rust entry remains | `witness/evidence/stop-b3/probe.log`, `flipback.log`, `summary.tsv`; `PROBE-FLIP-SHA 064747f…`, `PROBE-FLIPBACK-SHA af7df34…`; member-list proof in `01-20-SUMMARY.md`; unit provenance tests remain |
| Nine x86_64 q35 boot-test profiles pass through the reconciled mechanism | HARN-06 | KVM-only; profiles need the Linux ISO path | `witness/evidence/stop-e/profiles.log` (nine `exit=0`: q35, q35lowmcfg, q35net, q35exec, q35churn, q35shell, q35churnshell, q35cpu, q35auth) |
| Full moved x86_64 set (8 apps), soak, stress and equivalence 10/10 from r9 | HARN-08 | KVM + the owner's tap venue | `witness/evidence/stop-e/full.tsv` (8/8), `soak.log` (`ALL STAGES PASS`, tap), `stress.log` (tap), `equiv/report.tsv` (all 10 rows match) |
| Harvey q35 own-kernel command at the pushed commit reaches `root is from` | HARN-08 (D-20/D-26) | Owner's Harvey checkout / lima / sudo | `witness/evidence/stop-e/harvey-q35.log` (`Q35-KERNEL sha256=ecdcdadd…`, `Q35-PROMPT root is from (tcp)[tcp]:`, `Q35-SIGS none`) |
| The one owner-approved Harvey commit and both pushes | HARN-08 | Owner-only push | `harvey-flip/HARVEY-FLIP.md`; Harvey `c697666ba315f1…` (36 paths, +285/−9972, no font path); r9 lock bump `626c80c` (flake.lock only) |
| Hosted CI as the best-effort second witness | HARN-07/HARN-08 (D-13) | Runs on GitHub, after the owner pushes | `witness/evidence/stop-e/ci.txt` (`xtask.yml` all green; `gates.yml` green except `quick-gate-tcg`, which is best-effort; native TCG PASS at stop C3) |
| Mac lima builder probe | HARN-07 (D-04/D-24) | Owner-registered remote builder | `witness/evidence/stop-a/mac-builder-probe.txt` (fresh aarch64-linux derivation built through `ssh-ng://r9-lima-builder`, imported to the Mac store) |

**Aarch64 quick pass is now automated, not manual.** The five gates were re-run live for this
audit: `nix run .#boot-r9-smoke`, `.#boot-r9-tier1` (HVF and `R9_ACCEL=tcg`), `.#boot-r9-net-smoke`
and `.#boot-r9-cpu` — all `ALL STAGES PASS`, exit 0.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — pass-1 rows are historical spike evidence; every pass-2 row has an automated command, or a manual row with committed witness evidence
- [x] Sampling continuity: no 3 consecutive tasks without automated verify — the owner stops are isolated manual checkpoints (A/A2, B/B2/B3, C/C2/C3, E), each bracketed by automated plans
- [x] Wave 0 covers all MISSING references — S1–S6 answered, with FAILs documented; the pass-2 fixtures (`harvey.rs`/`image.rs` tests, 13 manifests, reference derivations) exist and are exercised
- [x] No watch-mode flags
- [x] Feedback latency measured and recorded — host suite ≈ 4 s; Mac boot gates 13–58 s; KVM gate minutes
- [x] `nyquist_compliant: true` set in frontmatter — 8/8 HARN requirements covered by an automated behavioural test that ran, or by an explicit owner-witness row with evidence; no requirement is unverified

**Approval:** validated 2026-11-23 — pass 1 (spikes, D-22) + pass 2 (implementation, owner stops);
HARN-01…08 covered; warnings W-1…W-5 recorded below, none of which leaves a requirement unverified.

---

## Validation Audit

| Metric | Value |
|--------|-------|
| Requirement-level gaps (no test and no evidence) | **0** |
| Requirements covered (automated + manual) | **8/8** (HARN-01…08) |
| Pass-2 rows (this revision) | **31** |
| Pass-2 rows green, primary automated command run in this audit (`ran`) | **24** |
| Pass-2 rows green with a Linux-only leg from committed evidence | **1** (01-16 cpu/auth compare) |
| Pass-2 rows mixed: unit tests ran + owner witness for the end-to-end leg | **1** (01-20 swap) |
| Pass-2 rows manual-only (owner witness, evidence path) | **5** (01-12, 01-22, 01-26, 01-27, 01-32) |
| Requirements whose KVM leg is manual-only | 4 (HARN-01 rio gate, HARN-04 swap, HARN-06 x86_64 profiles, HARN-08 moved-set equivalence) |
| Warnings | 5 (W-1…W-5) |
| Escalated | 0 |

### Commands run for this audit (2026-11-23, aarch64 Mac)

| # | Command | Result |
|---|---------|--------|
| 1 | `nix develop -c cargo test -p xtask` | **185 passed** (lib 92 + netdev bin 1 + main 89 + doctests 3), 0 failed |
| 2 | `cargo test -p xtask harvey::` / `image::` / `gate::` / `config::` / `uregcheck::` | 14 / 46 / 10 / 23 / 11 passed, 0 failed |
| 3 | `nix develop -c cargo xtask test` | **395 passed** (port 168 + integration 69 + aarch64 71 + aarch64 virtshell 87), 0 failed |
| 4 | `nix develop -c cargo xtask clippy`; `… check` | exit 0 / exit 0 (warnings only) |
| 5 | `nix develop -c cargo xtask ureg-check` | 35 arm64 fields + 26 amd64 fields match; negative control fired |
| 6 | `env -u R9_HARVEY_DIR -u …_AMD64 -u …_ARM64 -u HARVEY_SRC cargo xtask ureg-check` | **exit 1**, names all three variables and "no built-in default" (HARN-02 named error) |
| 7 | `cargo xtask image --arch x86-64 --image {marker,chain,substrate,net,services,tier,churn} --compare-reference` | 30 / 49 / 27 / 64 / 4517 / 91 / 52 members, all match |
| 8 | `cargo xtask image --arch aarch64 --image bootfs --compare-reference` | 151 members, match |
| 9 | `cargo xtask image --arch {x86-64,aarch64} --libraries` | tier-0 list OK (27 / 35) |
| 10 | Negative compare: tampered `marker.members` (README mode 644→645) | **exit 1**, `changed: README.md (mode 644 != reference 645)` |
| 11 | `cargo xtask image --arch x86-64 --image cpu\|auth --compare-reference` on the Mac | named `R9_AUTHDOMAIN_GEN is not set` (Linux-only generator); KVM evidence: cpu 4442, auth 95 |
| 12 | `nix develop -c cargo xtask boot-test --arch aarch64 --config {virtexec,virtnet,virtip} --accel hvf` | all **PASS** (virtexec 13 s) |
| 13 | `nix run .#boot-r9-smoke`; `.#boot-r9-tier1` (HVF); `R9_ACCEL=tcg .#boot-r9-tier1`; `.#boot-r9-net-smoke`; `.#boot-r9-cpu` | all **PASS**, `ALL STAGES PASS` (49 s, 52 s, 14 s, 19 s, 58 s) |
| 14 | `cargo xtask gate --arch x86-64` on the Mac | **exit 1**, `GATE-VENUE not-linux … never on the Mac (D-13)` |
| 15 | `nix store verify --no-trust` on both trees and both reference derivations (after the gates) | all clean — **HARN-02 after-gate proof** |
| 16 | `nix develop -c sh -c 'printf %s "$R9_HARVEY_DIR_ARM64"'` etc. | equals the eval outPath; `R9_HARVEY_DIR` unset; `R9_MKPAQFS`/`R9_AUTHDOMAIN_GEN` contract as designed |
| 17 | `jq .nodes.harvey.locked.rev flake.lock`; `jq .nodes.harvey.inputs`; `grep -c 116bcc2` | `c697666…`; `{nixpkgs}`; 0 |
| 18 | `git show c697666:flake.nix` on Harvey | no `r9` input; 0 `boot-r9`/`drawterm-gate`/`net-doctor` outputs; lock root inputs `{nixpkgs}` |
| 19 | `nix eval .#apps.x86_64-linux` and `.#apps.aarch64-darwin`; `nix build .#checks.aarch64-darwin.harvey-harness`; both `harvey-drawterm.drvPath` evals; doc-key loop | 20 apps (x86_64) including all 14 `boot-r9*`, `drawterm-gate-x64`, `net-doctor`, `drawterm`; 5 aarch64 gate apps; all `.program` paths evaluate; harness builds; `DOC-KEYS-OK` |
| 20 | `nix run nixpkgs#actionlint -- .github/workflows/{xtask,gates}.yml` | exit 0; 0 unpinned actions; 0 Harvey checkouts; 6 relocated legs; 1 TCG quick-gate job |
| 21 | `shellcheck nix/gates/*.sh witness/run-witness.sh rio-derisk/*.sh`; Tcl `info complete` over all 15 `gates/**/*.expect` | 0 findings; 15/15 complete |
| 22 | `cmp` `nix/gates/host-echo.c` vs Harvey `bd7a698:nix/host-echo.c`; by-value password grep | `HOST-ECHO-VERBATIM`; 0 |
| 23 | `cargo xtask build --arch x86-64 --config {q35exec,q35churn,q35shell,q35churnshell,q35auth}` | all exit 0; every profile names `image:`, no `harvey:` module |
| 24 | Witness evidence inspection | stop-A/A2/B3/C2/C3/E verdicts and summaries as cited above; 76 `VERDICT` lines in `WITNESS-LOG.md`; R1–R7 audit PASS |

### Warnings (caveats, not uncovered requirements)

| # | Warning | Impact / disposition |
|---|---------|----------------------|
| W-1 | `cargo xtask test` **does not run the `xtask` crate's own 185 tests**; it runs only `port` and `aarch64`. | The plans' `<automated>` blocks used `cargo test -p xtask …`, so execution coverage was not lost. The quick command above includes both halves. A future change should either add the xtask leg to `TestStep` (implementation work, outside this audit) or keep the two-command quick gate. |
| W-2 | `x86_64/src/syscall.rs#sleep_and_alarm_convert_milliseconds_to_the_100hz_slice` has **no runnable command**: the crate's `default-target` is `x86_64-unknown-none`, and `cargo test -p x86_64` fails to compile on the host (`#[panic_handler] function required`; 1907 errors). | The 01-34 claim "host leg" is overstated. The behaviour is covered end-to-end by the KVM fpprobe leg (`stop-c2/full.log`: `T1-OK fpprobe`, 39 expected, 0 `T1-FAIL`) and by the compile-time asserts in every kernel build. Recorded, not silently green. |
| W-3 | The HARN-04 swap proof is the **stop-B3** run, which predates the 01-34 `gate.rs` fix (240 added lines). | 01-28's audit records this as a residual risk. The fix changed driver spawn/registration, not the swap assertion; no manifest has a Rust entry, so re-flipping would be a new probe. The member-list/provenance proof remains valid. |
| W-4 | Hosted CI `quick-gate-tcg` is **red**; `gates.yml`'s other jobs are green. | D-13 makes hosted CI best-effort. The required witness is the owner's local KVM run (`stop-e/gate.log` + `gate-neg.log`), and the owner's native amd64 TCG gate is `GATE-PASS` at stop C3. |
| W-5 | No **bump-time aarch64 quick-pass log** is in the stop-E return; the first stop-E `full` iteration recorded one `boot-r9-tier1-x64 FAIL` (final run 8/8). | The aarch64 half of HARN-06 rests on 01-30's Mac log and was re-run live for this audit (all five gates PASS today). The transient tier1 failure is disclosed in `WITNESS-LOG.md`; the final 8/8 is the verdict. |

### Residual owner actions

None required for validation. The phase's remaining dispositions are the recorded warnings
above; no requirement is left unverified. The next phase (Phase 2) may begin against the
validated harness.
