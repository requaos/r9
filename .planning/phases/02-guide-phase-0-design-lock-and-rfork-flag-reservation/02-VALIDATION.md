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
| **Quick run command** | `nix develop -c cargo xtask test` (after the wiring: header staleness check, `port` tests, both aarch64 legs); add `nix develop -c cargo test -p xtask rfork` when xtask changes |
| **Full suite command** | `nix develop -c cargo xtask test && nix develop -c cargo test -p xtask && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy` (clippy in the default-arch form hosted CI runs; run in plan 02-05 Task 2) |
| **Estimated runtime** | quick: under 60 seconds once built; full: several minutes |

`cargo xtask test` does not run the xtask crate's own tests; `cargo test -p xtask` does. The x86_64 host test leg is skipped in the nix devshell on the Mac and compiles for the first time on the owner's x86_64 Linux host (after the xtask host-triple fallback fix). `cargo xtask clippy --arch aarch64` fails at HEAD with 4 errors and is not a pass criterion for this phase.

**Required dispatch mode (set before `/gsd-execute-phase 2`):** `workflow.use_worktrees` = `false` in `.planning/config.json`, set with `node "$HOME/.claude/gsd-core/bin/gsd-tools.cjs" query config-set workflow.use_worktrees false`, or set `"use_worktrees": false` under `"workflow"` in `.planning/config.json` by hand. The key is unset and defaults to true, which dispatches each plan into an agent worktree. With it off, every plan runs in sequence on the main working tree. With it on: plan 02-06 fails its main-tree check on first dispatch and stops (it merges and pushes nothing); 02-01 Task 4 and 02-02 Task 3 stop if the continuation worktree lacks the first agent's commits; 02-03's edits to ROADMAP.md and STATE.md, which are plan content, meet the worktree executor's instruction not to update those files. Each of those stops names this setting as the remedy; spawning another continuation agent without changing it repeats the stop.

---

## Sampling Rate

- **After every task commit:** Run the quick run command
- **After every plan wave:** Run the full suite command
- **Before `/gsd-verify-work`:** Full suite green on the Mac; byte evidence recorded for four configs with `.text` equal; aarch64 quick pass green; then the single owner stop on the x86_64 Linux host (branch push approval, x86_64 host tests, `outputHash` commit, build and `--rebuild`, KVM quick gate once, TCG gate, full gate set; hash build, `--rebuild` and gates all at one commit)
- **Max feedback latency:** 60 seconds for the quick command

---

## Per-Task Verification Map

Task IDs name the plan and task (`02-01-T1` is plan 02-01, Task 1); the rows below are the per-requirement contract.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-02-T1, 02-02-T3 | 02-02 | 1 | DSGN-01 | T-02-06, T-02-07, T-02-08, T-02-25 | Threat-model doc states in-scope and out-of-scope boundaries exactly, names SEV-SNP/TDX, states no protection is implemented yet, every statement cited or marked new | doc check + owner review | `nix develop -c cargo test -p xtask threat_model` | ❌ W0 | ⬜ pending |
| 02-01-T2, 02-01-T4, 02-04-T3 | 02-01, 02-04 | 1, 2 | DSGN-02 | T-02-02, T-02-14 | `sys/include/rfork.h` equals the rendering of the Rust flag table; a mutated copy is rejected | unit + CLI | `nix develop -c cargo xtask rfork-header --check`; `nix develop -c cargo test -p xtask rfork` | ❌ W0 | ⬜ pending |
| 02-01-T2, 02-01-T4, 02-04-T1, 02-04-T2, 02-03-T1 | 02-01, 02-04, 02-03 | 1, 2 | DSGN-03 | T-02-01, T-02-04, T-02-13 | Sixteen flags defined once in the `port/` module; no definition left in the arch files; both arch crates build | port test + source-invariant test + build | `nix develop -c cargo test -p port --test rfork_test`; `nix develop -c cargo xtask check` | ❌ W0 | ⬜ pending |
| 02-01-T2, 02-01-T4, 02-04-T1, 02-04-T2, 02-06-T1, 02-06-T2 | 02-01, 02-04, 02-06 | 1, 2, 4 | DSGN-04 | T-02-03, T-02-12, T-02-15 | Four values pinned; disjoint single bits; bit 14 is `RFNOMNT`; none of the five in either supported mask; mask value unchanged | unit (port; aarch64 on Mac; x86_64 in CI/Linux) | `nix develop -c cargo test -p port --test rfork_test`; `nix develop -c cargo xtask test` | ❌ W0 | ⬜ pending |
| 02-01-T1, 02-05-T1, 02-05-T2, 02-05-T3, 02-06-T1, 02-06-T2 | 02-01, 02-05, 02-06 | 1, 3, 4 | DSGN-04 (behaviour unchanged) | T-02-16, T-02-23, T-02-26 | Kernel code unchanged around the move for both arches; boot still reaches rio | build comparison + gate | `.text` sha256 before/after on four configs per `02-RESEARCH.md` "Kernel Bytes" (a `.text` difference stops the phase); owner's `nix develop -c cargo xtask gate --arch x86-64 --accel kvm` once at phase end; aarch64 quick pass | procedure | ⬜ pending |
| 02-03-T1, 02-03-T2, 02-03-T3 | 02-03 | 1 | DSGN-05 | T-02-09, T-02-10 | PROJECT.md records each of IG-D01..D13 as locked, locked-amended, or not-locked-deferred; no "confirm in Phase 2" left | doc check + owner checkpoint | `grep -c "confirm in Phase 2" .planning/PROJECT.md` returns 0 | manual confirmation | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `port/src/rfork.rs` and `port/tests/rfork_test.rs` — DSGN-03, DSGN-04 (owner diff approval before commit)
- [ ] xtask header generator module with unit tests and negative control; `rfork-header` subcommand — DSGN-02
- [ ] Supported-mask hoist and a test in each arch `syscall.rs` — DSGN-04
- [ ] `sys/include/rfork.h` (generated) and `docs/threat-model.md` — DSGN-02, DSGN-01
- [ ] xtask host-triple fallback widened to x86_64 hosts so the x86_64 host tests run on the owner's Linux host — D-24
- [ ] Before-kernels for four configs (`q35churnshell`, `q35cpu`, `virtshell`, `virtcpu`) built from the pre-edit commit and copied out of `target/` — D-23
- [ ] Framework install: none

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Owner approves the `port/src` diff before commit | DSGN-03 | Standing owner rule | Show the diff; wait for approval |
| Owner reviews the threat-model draft | DSGN-01 | CONTEXT D-15 | Present the draft with source marks; wait for approval |
| Owner confirms the thirteen-decision table | DSGN-05 | CONTEXT D-03 | Present the table with proposed statuses; record the ruling in PROJECT.md |
| Branch push for transport to the Linux host | DSGN-04 | Standing owner rule; CONTEXT D-26 | Show the unpushed commits; owner approves pushing a branch (not main) |
| Quick gate to rio on the x86_64 Linux KVM host, once, plus the full gate set | DSGN-04 | The gate refuses to run on the Mac; CONTEXT D-25, D-27 | Owner runs `nix develop -c cargo xtask gate --arch x86-64 --accel kvm` once, then the full set |
| x86_64 host tests on the Linux host | DSGN-04 | Not runnable on the Mac; CONTEXT D-24 | Owner runs `nix develop -c cargo xtask test --verbose` after the fallback fix |
| x86_64 kernel byte-change procedure (planned work, CONTEXT D-23, D-28) | DSGN-04 | Standing owner rule; needs the Linux host | Owner re-derives the `boot-iso` `outputHash` and commits it on the host before the gate run, runs the TCG gate and the linux boot-iso check |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
