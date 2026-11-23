---
phase: "2"
slug: "guide-phase-0-design-lock-and-rfork-flag-reservation"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-23"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `02-RESEARCH.md`, section "Validation Architecture".

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in tests: `port` (via `cargo xtask test`), `xtask` (via `cargo test -p xtask`), `aarch64` host leg (via `cargo xtask test`), `x86_64` host leg (hosted CI or an x86_64 rustup host only); shell/expect gates for boot |
| **Config file** | none (workspace `Cargo.toml`; `.planning/config.json` sets `nix develop -c cargo xtask check` / `test`) |
| **Quick run command** | `nix develop -c cargo test -p xtask rfork && nix develop -c cargo test -p port --test rfork_test` |
| **Full suite command** | `nix develop -c cargo test -p xtask && nix develop -c cargo xtask test && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy` |
| **Estimated runtime** | quick: under 60 seconds once built; full: several minutes |

`cargo xtask test` does not run the xtask crate's own tests; `cargo test -p xtask` does. The x86_64 host test leg is skipped in the nix devshell on the Mac.

---

## Sampling Rate

- **After every task commit:** Run the quick run command
- **After every plan wave:** Run `nix develop -c cargo test -p xtask && nix develop -c cargo xtask test && nix develop -c cargo xtask check`
- **Before `/gsd-verify-work`:** Full suite green, kernel-byte evidence recorded, quick gate green on the owner's x86_64 Linux KVM host, aarch64 quick pass green
- **Max feedback latency:** 60 seconds for the quick command

---

## Per-Task Verification Map

Task IDs are filled in by the planner; the rows below are the per-requirement contract.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | DSGN-01 | — | Threat-model doc states in-scope and out-of-scope boundaries exactly, names SEV-SNP/TDX, states no protection is implemented yet, every statement cited or marked new | doc check + owner review | `nix develop -c cargo test -p xtask threat_model` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DSGN-02 | — | `sys/include/rfork.h` equals the rendering of the Rust flag table; a mutated copy is rejected | unit + CLI | `nix develop -c cargo xtask rfork-header --check`; `nix develop -c cargo test -p xtask rfork` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DSGN-03 | — | Sixteen flags defined once in the `port/` module; no definition left in the arch files; both arch crates build | port test + source-invariant test + build | `nix develop -c cargo test -p port --test rfork_test`; `nix develop -c cargo xtask check` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DSGN-04 | — | Four values pinned; disjoint single bits; bit 14 is `RFNOMNT`; none of the five in either supported mask; mask value unchanged | unit (port; aarch64 on Mac; x86_64 in CI/Linux) | `nix develop -c cargo test -p port --test rfork_test`; `nix develop -c cargo xtask test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DSGN-04 (behaviour unchanged) | — | Kernel code unchanged around the move for both arches; boot still reaches rio | build comparison + gate | sha256 comparison per `02-RESEARCH.md` "Kernel Bytes"; owner's `nix develop -c cargo xtask gate --arch x86-64`; aarch64 quick pass | procedure | ⬜ pending |
| TBD | TBD | TBD | DSGN-05 | — | PROJECT.md records each of IG-D01..D13 as locked, locked-amended, or not-locked-deferred; no "confirm in Phase 2" left | doc check + owner checkpoint | `grep -c "confirm in Phase 2" .planning/PROJECT.md` returns 0 | manual confirmation | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `port/src/rfork.rs` and `port/tests/rfork_test.rs` — DSGN-03, DSGN-04 (owner diff approval before commit)
- [ ] xtask header generator module with unit tests and negative control; `rfork-header` subcommand — DSGN-02
- [ ] Supported-mask hoist and a test in each arch `syscall.rs` — DSGN-04
- [ ] `sys/include/rfork.h` (generated) and `docs/threat-model.md` — DSGN-02, DSGN-01
- [ ] Before-kernels (both arches) built and hashed from the pre-edit commit, kept for the D-22 comparison
- [ ] Framework install: none

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Owner approves the `port/src` diff before commit | DSGN-03 | Standing owner rule | Show the diff; wait for approval |
| Owner reviews the threat-model draft | DSGN-01 | CONTEXT D-15 | Present the draft with source marks; wait for approval |
| Owner confirms the thirteen-decision table | DSGN-05 | CONTEXT D-03 | Present the table with proposed statuses; record the ruling in PROJECT.md |
| Quick gate to rio on the x86_64 Linux KVM host | DSGN-04 | The gate refuses to run on the Mac | Owner runs `nix develop -c cargo xtask gate --arch x86-64` |
| x86_64 kernel byte-change procedure, if bytes change | DSGN-04 | Standing owner rule; needs the Linux host | Re-derive `boot-iso` `outputHash`, amd64-container TCG gates, owner's linux boot-iso check |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
