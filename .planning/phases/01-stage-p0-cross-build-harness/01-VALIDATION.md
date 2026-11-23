---
phase: "1"
slug: "stage-p0-cross-build-harness"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-22"
validated: "2026-11-23"
pass2_seeded: "2026-11-23"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
>
> **Pass-1 scope (D-22).** Phase 1 is planned in two passes. Pass 1 (plans 01-01 to 01-07) answers the
> spike questions S1–S6 and ends at an owner decision checkpoint; it implements **no HARN requirement**.
> Pass 2 is written after the spike results. The coverage gates are therefore overridden for pass 1:
> every HARN row below is manual-only / deferred to pass 2, and no test is generated against
> unimplemented behavior.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in tests through `cargo xtask test` (host); expect and shell gates for boot behaviour |
| **Config file** | none (workspace `Cargo.toml`; xtask is a std crate) |
| **Quick run command** | `nix develop -c cargo xtask test` |
| **Full suite command** | `nix develop -c cargo xtask test`, then `cargo xtask clippy` and `cargo xtask check` |
| **Estimated runtime** | full `cargo xtask test` measured at ~1 s host time (87 + 71 + 5 tests, 2026-11-23); boot gates need the Linux KVM witness |

---

## Sampling Rate

- **After every task commit:** Run `nix develop -c cargo xtask test`
- **After every plan wave:** Run the full suite; the quick gate on the Linux KVM witness once it exists
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** measured: host suite ~1 s; boot gates minutes (Linux KVM)

---

## Per-Task Verification Map

Plan-level rows for pass 1 (the plans are spike investigations; a per-task automated map is a pass-2 artifact).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01/T1-T3 | 01-01 | 1 | HARN-07, HARN-02 | — | Read-only Harvey tree; no sudo | manual (spike) | none — pass 1 | n/a | ⬜ deferred (D-22) |
| 01-02/T1-T3 | 01-02 | 2 | HARN-07, HARN-06 | — | Read-only Harvey tree; no sudo | manual (spike) | none — pass 1 | n/a | ⬜ deferred (D-22) |
| 01-03/T1-T2 | 01-03 | 1 | HARN-08 | — | Read-only Harvey tree; no sudo | manual (spike) | none — pass 1 | n/a | ⬜ deferred (D-22) |
| 01-04/T1-T3 | 01-04 | 1 | HARN-04 | — | No protected-path edits | manual (spike) | none — pass 1 | n/a | ⬜ deferred (D-22) |
| 01-05/T1-T2 | 01-05 | 1 | HARN-01, HARN-03 | — | Redacted auth captures; no sudo | manual (spike) | none — pass 1 | n/a | ⬜ deferred (D-22) |
| 01-06/T1-T3 | 01-06 | 2 | HARN-01, HARN-04, HARN-07, HARN-08 | — | Root-refusing, sudo-free witness driver; credential scan | manual (witness) | none — pass 1 | n/a | ⬜ deferred (D-22) |
| 01-07/T1-T3 | 01-07 | 3 | HARN-07, HARN-08, HARN-04, HARN-01 | — | No pushes; prohibition audit | manual (closing) | none — pass 1 | n/a | ✅ completed |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Pass 2 (plans 01-08 to 01-31; seeded at planning, not yet validated)

Venue column: **Mac** = the executor's aarch64 Mac; **OWNER-LINUX** = the owner's x86_64 KVM host (never verified from the Mac). Commands are abbreviated; each plan's `<verify>` blocks hold the exact form and its `<fails_when>`.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | Venue | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------|--------|
| 01-08/T1 (tracer) | 01-08 | 4 | HARN-03, HARN-06, HARN-07 | T-01-27 | Staging under target/ only; path validation | unit + boot | `cargo test -p xtask image::`; `cargo xtask boot-test --arch aarch64 --config virtexec --accel hvf` (override supply) | Mac | ⬜ |
| 01-08/T2 | 01-08 | 4 | HARN-02, HARN-07 | T-01-28, T-01-29 | No default path; named error; realise checks printed path | unit + grep | `cargo test -p xtask harvey::`; direct-env-read grep == 0; `cargo xtask ureg-check` named error | Mac | ⬜ |
| 01-09/T1-T3 | 01-09 | 4 | HARN-01 | T-01-30..33 | Password never on argv/logs; root-refusing driver | static | shellcheck; Tcl `info complete`; `nix eval` of rio-gate; argv grep == 0 | Mac | ⬜ |
| 01-10/T1 | 01-10 | 4 | HARN-07, HARN-08 | T-01-34 | Only pushed Harvey revs locked | lock + build | `jq .nodes.harvey.locked.rev`; `nix build .#checks.aarch64-darwin.harvey-harness`; drawterm drv eval | Mac | ⬜ |
| 01-10/T2 | 01-10 | 4 | HARN-07 | T-01-35, T-01-36 | Builder key root-only, loopback alias | doc | doc-key grep | Mac | ⬜ |
| 01-11/T1-T2 | 01-11 | 5 | HARN-03, HARN-05 | T-01-37..39 | Two-way drift; Rust provenance; symlink escape | unit | `cargo test -p xtask image::`; `cargo xtask image --arch aarch64 --image smoke` | Mac | ⬜ |
| 01-12/T1-T2 | 01-12 | 5 | HARN-01, HARN-07 | T-01-40..42 | Redacted evidence; stop on rio FAIL | witness | builder probe derivation (Mac); witness steps rio, control, a13 | Mac + OWNER-LINUX | ⬜ |
| 01-13/T1 | 01-13 | 6 | HARN-07, HARN-02, HARN-06 | T-01-43..46 | FOD source; case-clean tree; lazy export | build + boot | `nix build .#packages.aarch64-linux.harvey-tree-arm64`; store verify; lazy path match; virtexec boot with nothing set | Mac | ⬜ |
| 01-13/T2 | 01-13 | 6 | HARN-07 | — | — | doc | contract-doc key grep | Mac | ⬜ |
| 01-14/T1-T2 | 01-14 | 7 | HARN-07, HARN-03 | T-01-47..49 | No key material in store; case-clean | build + unit | `TREE-AMD64 tier<N>` counts; `cargo xtask ureg-check` in devshell; `cargo xtask image --libraries` | Mac | ⬜ |
| 01-15/T1-T2 | 01-15 | 8 | HARN-03, HARN-05 | T-01-50, T-01-51 | Reference output credential-free | build + compare | 9 reference member lists; creds/password scan == 0; compare-reference marker/chain/substrate | Mac | ⬜ |
| 01-16/T1-T2 | 01-16 | 9 | HARN-03, HARN-01, HARN-06 | T-01-52..54 | 0600 creds; no password in output | unit + compare | `cargo test -p xtask image::`; compare-reference cpu/auth; creds mode 600; q35cpu harvey: module grep == 0 | Mac (or OWNER-LINUX if authdomain-gen is Linux-only) | ⬜ |
| 01-17/T1-T3 | 01-17 | 9 | HARN-03, HARN-05, HARN-06, HARN-07 | T-01-55..57 | Case-clean arm64 tree | build + compare + boot | `TREE-ARM64 built=`; compare-reference bootfs; virtnet/virtip boot HVF | Mac | ⬜ |
| 01-18/T1 | 01-18 | 10 | HARN-01, HARN-08 | T-01-58, T-01-59 | Password by 0600 file path only | static | shellcheck; Tcl; `nix eval .#apps.x86_64-linux.drawterm-gate-x64.program` | Mac | ⬜ |
| 01-18/T2 | 01-18 | 10 | HARN-01, HARN-02 | T-01-60..62 | No silent TCG downgrade; tree-unmodified check | unit + venue | `cargo test -p xtask gate::`; Mac gate run must print GATE-VENUE | Mac | ⬜ |
| 01-19/T1-T2 | 01-19 | 10 | HARN-03, HARN-05 | T-01-63, T-01-64 | No compilation in the assembler | compare | compare-reference net/services/tier/churn | Mac | ⬜ |
| 01-29/T1 | 01-29 | 10 | HARN-06 | T-01-93 | Profiles re-pointed only at reference-matched images | compare + grep + build | compare-reference marker/chain/substrate; five profiles on `image:` (per-file loop); x86_64 harvey: module grep == 0; `cargo xtask build` for the five | Mac | ⬜ |
| 01-20/T1-T3 | 01-20 | 11 | HARN-04, HARN-05 | T-01-65, T-01-66 | Provenance rule; probe removed | unit + member diff | `cargo test -p xtask probe::`; flip diff == /amd64/bin/ps only; flip-back `cmp` identical; remnant grep == 0 | Mac | ⬜ |
| 01-21/T1-T3 | 01-21 | 11 | HARN-08, HARN-06 | T-01-67..69 | No writes under $HARVEY | static + boot | shellcheck; Tcl; tier-1 HVF+TCG, smoke HVF; three app evals; store verify | Mac | ⬜ |
| 01-22/T1-T3 | 01-22 | 12 | HARN-01, HARN-04, HARN-06, HARN-02, HARN-07 | T-01-70..73 | Verdicts only from returned rows; boot-iso FOD | witness | steps trees, kernel, gate×2, gate_neg, probe, flipback, profiles | OWNER-LINUX | ⬜ |
| 01-30/T1-T2 | 01-30 | 12 | HARN-08, HARN-06 | T-01-94..96 | No writes under $HARVEY | static + boot | shellcheck; Tcl; five aarch64 app evals; five aarch64 drivers; aarch64 quick pass (tier-1 HVF+TCG, smoke, net-smoke, cpu HVF); store verify | Mac | ⬜ |
| 01-23/T1-T2 | 01-23 | 13 | HARN-08 | T-01-74..76 | — | static | shellcheck; Tcl; four app evals | Mac | ⬜ |
| 01-24/T1-T2 | 01-24 | 14 | HARN-08 | T-01-78, T-01-79 | Net gates write only to R9_GATE_OUT; host-echo verbatim | static | shellcheck; Tcl; four net app evals; host-echo `cmp` against the pin | Mac | ⬜ |
| 01-25/T1-T2 | 01-25 | 14 | HARN-07, HARN-08 | T-01-80, T-01-81 | Actions pinned by sha | lint | `nix run nixpkgs#actionlint` on both workflows; no Harvey checkout in CI | Mac | ⬜ |
| 01-26/T1-T2 | 01-26 | 15 | HARN-08, HARN-02 | T-01-82..85 | Owner checkout untouched | scratch validation | scratch lock has no r9; flake show has no boot-r9*; q35 `--no-build` reaches prompt (TCG); HARVEY-UNTOUCHED | Mac | ⬜ |
| 01-31/T1-T2 | 01-31 | 15 | HARN-08 | T-01-77, T-01-97, T-01-98 | Creds by 0600 file path; password never on argv or in env by value | static | shellcheck; Tcl; by-value password grep == 0; all twelve x86_64 app evals; 15 drivers present | Mac | ⬜ |
| 01-27/T1-T3 | 01-27 | 16 | HARN-08, HARN-02 | T-01-86..89 | Explicit-path staging; no attribution; no push | witness + commit guard | steps full, soak, stress, equiv, store_verify; staged set == diff set; font-file guard | OWNER-LINUX + Mac | ⬜ |
| 01-28/T1-T3 | 01-28 | 17 | HARN-08, HARN-01, HARN-02, HARN-06, HARN-07 | T-01-90..92 | Lock only pushed revs; lock-only commit | lock + compare + witness | `has("r9") == false`; all 11 compare-reference; aarch64 quick pass; stop-E steps; audit R1-R7 | Mac + OWNER-LINUX | ⬜ |

---

## Wave 0 Requirements

- [x] S1: userns and chroot build venue inside a nix sandbox (Linux witness, then builder) — **FAIL on the witness** (unshare bind of `/nix/store`); **PASS** with proot on both builders. `spikes/S1-FINDINGS.md`
- [x] S2: Mac builder path with the real Harvey input and its 23 case-colliding paths — **PASS** (23/23 blobs); copy-back into the Mac system store refused by signature policy. `spikes/S2-FINDINGS.md`
- [x] S3: headless rio proof on KVM (Xvfb, X11 drawterm, serial `ls /srv`) — **control PASS, positive leg FAIL** (headless auth never completed); determinism **FAIL** (bake 2, `hsecstored`). `spikes/S3-FINDINGS.md`
- [x] S4: amd64 tier build on an aarch64-linux host — **PASS** (tier 0 27/27, tier 1 40/40). `spikes/S4-FINDINGS.md`
- [x] S5: Harvey own-kernel q35 launch to a prompt (read-only recipe) — **PASS** on Mac TCG and x86_64 KVM. `spikes/S5-FINDINGS.md`
- [x] S6: Rust `a.out` probe boots and prints under the r9 loader — **PASS** (arm64 HVF, amd64 static, x86_64 KVM). `spikes/S6-FINDINGS.md`
- [ ] `xtask/src/harvey.rs` and `xtask/src/image.rs` tests, manifest fixtures, reference-bake hash fixtures — **pass 2**, after the implementation exists. Plans 01-08 and 01-11 write the tests TDD-first with temp-dir fixtures; the reference member lists come from the `harvey-reference-*` derivations (01-15, 01-17), not committed hash fixtures, because D-29 chose member lists and A12 was refuted.

*Pass 1 answers the six spikes; the pass-2 fixtures are not built yet.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Harvey resolver precedence + named error, consumers use it | HARN-02 | Not implemented in pass 1 (D-22) | Pass 2: `cargo test -p xtask harvey::` and the grep gate |
| Manifest parse + tier-0 classification | HARN-03 | Not implemented in pass 1 (D-22) | Pass 2: `cargo test -p xtask image::manifest` |
| Probe swap changes only the entry member and flips back | HARN-04 | Mechanism proven in S6; implementation in pass 2 | Pass 2: `cargo test -p xtask image::swap`, `cargo xtask gate --arch x86-64` |
| Negative controls: stray/missing/Rust-into-tree fail named | HARN-05 | Not in any pass-1 plan (D-22) | Pass 2: `cargo test -p xtask image::drift` |
| Existing profiles pass via resolver + assembler | HARN-06 | Profiles exist; images are pass 2 | Pass 2: 3 CI profiles on Mac + `q35shell`/`q35cpu` on Linux |
| Read-only Harvey tree supplied through the devshell | HARN-07 | Venue spikes only in pass 1 | Pass 2: `nix develop -c cargo xtask ureg-check`, unset env error |
| Moved apps build and pass from r9; no flake cycle | HARN-08 | Feasibility only in pass 1 | Pass 2: `nix flake show`, app evals, Harvey lock has no r9 input |
| Quick gate green on the Linux KVM witness | HARN-01 | Fixtures land in pass 2; S3 positive leg unproven (headless auth) | Re-run the rio gate after the DEMO password/headless auth fix |

**Pass-2 owner-run (KVM-only) verifications** — batched at four owner stops; each is a `verification: backstop` truth in its plan:

| Behavior | Requirement | Stop (plan) | Witness steps |
|----------|-------------|-------------|---------------|
| Fixed headless rio leg and unchanged control-noG | HARN-01 (D-28) | A (01-12) | rio, control, a13 |
| Quick gate green twice; failing direction; swap at flip and flip-back shas; nine x86_64 profiles; x86_64-linux trees; boot-iso FOD | HARN-01, HARN-04, HARN-06, HARN-07 | B (01-22) | trees, kernel, gate, gate_neg, probe, flipback, profiles |
| Full moved x86_64 set, soak, stress, equivalence, tree verify | HARN-08, HARN-02 | C (01-27) | full, soak, stress, equiv, store_verify |
| Same set at the lock-only pin bump; Harvey q35 own-kernel command; hosted CI | HARN-08, HARN-01, HARN-06 | E (01-28) | trees … store_verify; owner's `nix run .#q35`; CI report |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — pass-1 tasks are spike evidence, not implementation; the plan-level audit is recorded above
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify — deferred to pass 2 with the implementation
- [x] Wave 0 covers all MISSING references — S1–S6 answered, with FAILs documented
- [x] No watch-mode flags
- [x] Feedback latency measured and recorded — host suite ~1 s
- [ ] `nyquist_compliant: true` set in frontmatter — remains false until pass 2 lands

**Approval:** validated 2026-11-23 — pass-1 spike phase, no requirement implemented (D-22)

---

## Validation Audit

| Metric | Value |
|--------|-------|
| Gaps found | 8 (HARN-01..08 all unimplemented by design in pass 1) |
| Resolved | 0 (no tests generated against unimplemented behavior) |
| Escalated | 8 (manual-only / deferred to planning pass 2, D-22) |

## Validation Audit 2026-11-23

| Metric | Count |
|---|---|
| Gaps found | 8 |
| Resolved | 0 |
| Escalated | 8 |
