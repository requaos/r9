---
phase: "1"
slug: "stage-p0-cross-build-harness"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-22"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in tests through `cargo xtask test` (host); expect and shell gates for boot behaviour |
| **Config file** | none (workspace `Cargo.toml`; xtask is a std crate) |
| **Quick run command** | `nix develop -c cargo test -p xtask` |
| **Full suite command** | `nix develop -c cargo xtask test`, then `cargo xtask clippy` and `cargo xtask check` |
| **Estimated runtime** | not yet measured (unit tests run on the Mac; boot gates need the Linux KVM witness) |

---

## Sampling Rate

- **After every task commit:** Run `nix develop -c cargo test -p xtask`
- **After every plan wave:** Run the full suite; the quick gate on the Linux KVM witness once it exists
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** not yet measured

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| (filled by the planner; requirement-to-command map is in `01-RESEARCH.md`, "Validation Architecture") | | | | | | | | | |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] S1: userns and chroot build venue inside a nix sandbox (Linux witness, then builder)
- [ ] S2: Mac builder path with the real Harvey input and its 23 case-colliding paths
- [ ] S3: headless rio proof on KVM (Xvfb, X11 drawterm, serial `ls /srv`)
- [ ] S4: amd64 tier build on an aarch64-linux host
- [ ] S5: Harvey own-kernel q35 launch to a prompt (read-only recipe)
- [ ] S6: Rust `a.out` probe boots and prints under the r9 loader
- [ ] `xtask/src/harvey.rs` and `xtask/src/image.rs` tests, manifest fixtures, reference-bake hash fixtures

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Quick gate and moved gates green on KVM | HARN-01, HARN-06, HARN-08 | The required witness is the owner's x86_64 Linux KVM host; it cannot run on the aarch64 Mac | Owner runs the gate command on the Linux host and records the result |

*If none: "All phase behaviors have automated verification."*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency measured and recorded
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** {pending / approved YYYY-MM-DD}
