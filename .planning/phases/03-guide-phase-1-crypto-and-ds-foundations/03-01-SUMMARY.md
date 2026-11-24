---
phase: 03-guide-phase-1-crypto-and-ds-foundations
plan: 01
subsystem: crypto
tags: [crypto, hmac-sha256, rfc-4231, sha2, no_std, xtask, kernel-byte-evidence]

# Dependency graph
requires: []
provides:
  - "crypto/ workspace crate with six exact pins, each with a provenance comment"
  - "crypto::kdf::hmac_sha256 behind crypto's own API (no upstream type in the signature)"
  - "RFC 4231 section 4.2 case 1 pin running on the host default backend and the scalar backend"
  - "cargo xtask test legs for crypto: host default and the two soft cfgs"
  - "kernel sha256 before/after evidence: both kernels IDENTICAL on one venue"
affects: [03-12, 03-10, phase-4, phase-5]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan's estimate.
actuals:
  tokens: 6187
  tasks: 2
  commits: 5
  plan_head_before: 4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1
  plan_head_after: b40171118b2cc969dea8c26692d2eadcb56e7767

tech-stack:
  added:
    - "sha2 =0.11.1 (default-features = false)"
    - "hmac =0.13.0 (default-features = false)"
    - "hkdf =0.13.0 (default-features = false)"
    - "chacha20 =0.10.2 (default-features = false, cipher)"
    - "blake3 =1.8.7 (default-features = false, pure + five no_* features)"
    - "zeroize =1.9.2 (default-features = false)"
  patterns:
    - "Thin wrapper: upstream crates never appear in a crypto/ public signature (D-18)"
    - "Scalar-backend test leg: per-leg --config build.rustflags, never .cargo/config.toml"
    - "IG-V01: committed RED pin, GREEN twice, pin advance in a ledger"

key-files:
  created:
    - crypto/Cargo.toml
    - crypto/src/lib.rs
    - crypto/src/kdf.rs
    - crypto/tests/rfc4231.rs
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-01.md
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/kernel-bytes/before.sha256
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/kernel-bytes/after-03-01.sha256
    - .planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/kernel-bytes/KERNEL-BYTES.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - xtask/src/main.rs

key-decisions:
  - "hmac_sha256 is a thin wrapper over hmac::Hmac<sha2::Sha256>; no upstream type appears in a public signature (D-18)"
  - "The scalar-backend test leg runs as a second crypto leg with per-leg --config build.rustflags; the two artifact variants coexist under target/, so no --target-dir was needed (run 2 recompiled nothing)"
  - "zeroize =1.9.2 is kept as researched; the provenance comment records that it was seven days old at research, that 1.9.1 is the older alternative, and that the choice goes to the owner through plan 03-09's review rows and plan 03-10's checklist"
  - "Both kernel ELF sha256 values are unchanged, so D-21's stop condition is not met and the phase proceeds"

requirements-completed: [CRYP-01, LINT-01]

coverage:
  - id: D1
    description: "crypto/ is a workspace member with six exact pins, a provenance comment each, crate-level #![forbid(unsafe_code)], Error/Result and kdf::hmac_sha256"
    requirement: CRYP-01
    verification:
      - kind: integration
        ref: "cargo xtask test (rc 0; includes cargo test -p crypto --lib --tests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "RFC 4231 section 4.2 case 1 pin passes on the host default backend and on the scalar backend the kernels select"
    requirement: CRYP-01
    verification:
      - kind: unit
        ref: "crypto/tests/rfc4231.rs#hmac_sha256_pin_rfc4231_case1 (2 ok lines in `cargo xtask test`)"
        status: pass
      - kind: other
        ref: "nm evidence SOFT-LEG: sha2::sha256::soft::unroll::compress present in the soft leg's binary, absent from the default leg's"
        status: pass
    human_judgment: false
  - id: D3
    description: "Both kernels are byte-identical on one venue after crypto/ and its six pins joined the workspace"
    requirement: LINT-01
    verification:
      - kind: automated_ui
        ref: "evidence/kernel-bytes/KERNEL-BYTES.md: VERDICT elf-virtshell@03-01 and elf-q35churnshell@03-01 IDENTICAL"
        status: pass
    human_judgment: false
  - id: D4
    description: "The pin ledger records RED before GREEN, GREEN twice, and the advance commit"
    requirement: CRYP-01
    verification:
      - kind: other
        ref: "evidence/PINS-03-01.md: RED, GREEN-1, GREEN-2, ADVANCED 8be63c3"
        status: pass
    human_judgment: false
  - id: D5
    description: "The zeroize =1.9.2 pin (seven days old at research, 1.9.1 the older alternative) is recorded for owner review"
    requirement: CRYP-01
    verification: []
    human_judgment: true
    rationale: "Owner-only decision: plan 03-09 carries the review row and plan 03-10 the end-of-phase checklist; the executor keeps the researched pin and does not decide it"

# Metrics
duration: 2min
completed: 2026-11-24
status: complete
---

# Phase 3 Plan 01: Tracer — RFC 4231 pin through `crypto/` and the test gate, kernels byte-identical

**`crypto/` joins the workspace with six exact pins and HMAC-SHA256 behind its own API; the RFC 4231 case 1 pin runs RED then GREEN twice on the host default and scalar backends, and both kernels stay byte-identical.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-11-24T14:21:39Z
- **Completed:** 2026-11-24T14:24:08Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- `crypto/` is a workspace member: `sha2 =0.11.1`, `hmac =0.13.0`, `hkdf =0.13.0`, `chacha20 =0.10.2` (`cipher`), `blake3 =1.8.7` (`pure` plus five `no_*`) and `zeroize =1.9.2`, every one exact with `default-features = false` and a four-part provenance comment. `Cargo.lock` gained 176 lines and removed none.
- `crypto::kdf::hmac_sha256` wraps `hmac::Hmac<sha2::Sha256>`; `crypto/src/lib.rs` carries `#![forbid(unsafe_code)]` (D-10), `Error`, `Result`, and the API callers will use.
- The RFC 4231 section 4.2 case 1 vector (copied from rfc-editor.org at execution time) is pinned in `crypto/tests/rfc4231.rs` and runs on two `cargo xtask test` legs: the host default and the scalar backend the kernels select (`--cfg sha2_backend="soft"`, `--cfg chacha20_backend="soft"`). The pin printed `ok` twice, the `sha256::soft` symbol is present in the soft leg's binary, and the default leg's binary does not contain it.
- Both kernels rebuilt from nothing (`target/aarch64-unknown-none-elf` and `target/x86_64-unknown-none-elf` moved aside first) are byte-identical to the baseline: `virtshell` `9218b998…` and `q35churnshell` `c9dcef0b…` before and after. No protected path changed and `Cargo.lock` lost no existing entry.

## Task Commits

Each task was committed atomically:

1. **Task 1, Step A: kernel sha256 baseline** — `eaf5552` (evidence)
2. **Task 1, Step B: RED pin** — `bb57121` (test)
3. **Task 1, Step C: implementation and xtask legs** — `8be63c3` (feat)
4. **Task 1, Step C: pin advance** — `ba6efec` (evidence)
5. **Task 2: kernel bytes after the workspace change** — `b401711` (evidence)

**Plan metadata:** see the final docs commit for this SUMMARY, STATE.md and ROADMAP.md.

## Files Created/Modified

- `crypto/Cargo.toml` — the six exact pins, a provenance comment each (six `Provenance:` lines), and `[lints] workspace = true`
- `crypto/src/lib.rs` — `no_std` crate root, `#![forbid(unsafe_code)]`, `Error` (`OutputTooLong`, `CounterOverflow`, `TagMismatch`, `BadTagLength`), `Result`
- `crypto/src/kdf.rs` — `SHA256_LEN` and `hmac_sha256`, a thin wrapper, no upstream type public
- `crypto/tests/rfc4231.rs` — the RFC 4231 §4.2 case 1 pin, importing only `crypto::kdf::*`
- `xtask/src/main.rs` — two `TestStep` legs after `port`: `crypto` (`--lib --tests`) and the scalar-backend leg with the two soft cfgs in its own argument vector
- `Cargo.toml` — `"crypto"` added to `members`; nothing else
- `Cargo.lock` — 176 added lines, 0 removed
- `evidence/PINS-03-01.md` — RED, GREEN-1, GREEN-2, SOFT-LEG, ADVANCED `8be63c3`
- `evidence/kernel-bytes/before.sha256` — `base 4beb5c8…`, `venue Darwin arm64`, both ELF hashes
- `evidence/kernel-bytes/after-03-01.sha256` — `after ba6efec…`, same venue, both ELF hashes
- `evidence/kernel-bytes/KERNEL-BYTES.md` — before/after table and the two `@03-01` verdicts

## Decisions Made

- `hmac_sha256` is a thin wrapper over `hmac::Hmac<sha2::Sha256>`; the upstream keyed state never appears in a public signature (D-18).
- The scalar leg's rustflags live in the leg's own argument vector (`--config build.rustflags=[…]`), never in `.cargo/config.toml`, so the kernel builds' own `build.rustflags` are unaffected. No `--target-dir` was added: the default and soft artifact variants coexist under `target/`, and the second `cargo xtask test` run recompiled nothing (the plan's conditional was not triggered).
- `zeroize =1.9.2` is kept as researched, with the 1.9.1 alternative and the owner-review route (plan 03-09 rows, plan 03-10 checklist) recorded in its provenance comment.
- The workspace lint scope comment (naming the opted-in crates) is deliberately left for plan 03-12, which owns that rewrite when `ds/` lands.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## Known Stubs

None. The RED skeleton was replaced by the GREEN implementation in the same task; no placeholder values, TODOs or unwired data sources remain in the plan's files.

## Threat Flags

None beyond the plan's `<threat_model>`: the new surface is the six pinned crates (T-03-SC) and the host-vs-kernel backend split (T-03-02), both mitigated as planned.

## Optional for the owner

OPTIONAL, not a stop and not a pass criterion. RESEARCH assumption A8 (the nix-sandboxed `kernel-x64` and `kernel-virt` derivations build with the enlarged `Cargo.lock`) is otherwise first tested at the end-of-phase owner stop (plan 03-11). The owner may test it earlier by running `nix build .#kernel-x64` and `nix build .#kernel-virt` on the Linux host at any commit from this plan's last one onward; the lock has its final Phase 3 shape after plan 03-12, which adds the `roaring` dev-dependency. The executor pushes nothing for this and waits for nothing: getting the commits to the Linux host is the owner's own action under the standing push rule. Later plans proceed whether or not it is run. If the owner reports a result, record it as one line `A8-EARLY <sha> kernel-x64=<rc> kernel-virt=<rc>` in `evidence/kernel-bytes/KERNEL-BYTES.md`; a reported build failure is returned to the orchestrator as a checkpoint. The required check stays the single stop in plan 03-11.

## Next Phase Readiness

- The build shape every later `crypto/` plan depends on is proved: exact pins, provenance comments, the two-backend test gate, and byte-neutral kernel builds.
- Plan 03-12 runs next: `ds/` joins, `cargo xtask check` gains the host and kernel-spec legs, and the workspace scope comment is rewritten for the four opted-in crates.
- `zeroize =1.9.2` remains flagged for the owner's review at phase verification (plan 03-09 rows, plan 03-10 checklist); nothing in this plan depends on resolving it earlier.

## Self-Check: PASSED

All nine claimed files exist on disk and all five task commits are ancestors of HEAD. `gsd_run check evaluation-scope --plan 03-01 --commits-only` resolved five task commits for the plan. Plan-level verification: `cargo xtask test` rc 0; the RFC pin printed `ok` twice; two `@03-01` kernel verdicts are IDENTICAL.
