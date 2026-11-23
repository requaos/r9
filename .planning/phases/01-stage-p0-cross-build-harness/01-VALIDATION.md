---
phase: "1"
slug: "stage-p0-cross-build-harness"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-22"
validated: "2026-11-23"
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

---

## Wave 0 Requirements

- [x] S1: userns and chroot build venue inside a nix sandbox (Linux witness, then builder) — **FAIL on the witness** (unshare bind of `/nix/store`); **PASS** with proot on both builders. `spikes/S1-FINDINGS.md`
- [x] S2: Mac builder path with the real Harvey input and its 23 case-colliding paths — **PASS** (23/23 blobs); copy-back into the Mac system store refused by signature policy. `spikes/S2-FINDINGS.md`
- [x] S3: headless rio proof on KVM (Xvfb, X11 drawterm, serial `ls /srv`) — **control PASS, positive leg FAIL** (headless auth never completed); determinism **FAIL** (bake 2, `hsecstored`). `spikes/S3-FINDINGS.md`
- [x] S4: amd64 tier build on an aarch64-linux host — **PASS** (tier 0 27/27, tier 1 40/40). `spikes/S4-FINDINGS.md`
- [x] S5: Harvey own-kernel q35 launch to a prompt (read-only recipe) — **PASS** on Mac TCG and x86_64 KVM. `spikes/S5-FINDINGS.md`
- [x] S6: Rust `a.out` probe boots and prints under the r9 loader — **PASS** (arm64 HVF, amd64 static, x86_64 KVM). `spikes/S6-FINDINGS.md`
- [ ] `xtask/src/harvey.rs` and `xtask/src/image.rs` tests, manifest fixtures, reference-bake hash fixtures — **pass 2**, after the implementation exists

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
