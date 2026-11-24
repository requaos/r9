---
phase: "3"
slug: "guide-phase-1-crypto-and-ds-foundations"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-24"
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `03-RESEARCH.md`, "Validation Architecture". Test file and function names are proposals until the plans fix them.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in tests through `cargo xtask`; Miri (`cargo miri test`) for `ds/src/tpht/` |
| **Config file** | none; workspace `Cargo.toml` and `.planning/config.json` (`build_command`, `test_command`) |
| **Quick run command** | `nix develop -c cargo xtask test` |
| **Full suite command** | `nix develop -c cargo xtask test && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy && nix develop -c cargo xtask miri` |
| **Estimated runtime** | quick: seconds once built; a 64-seed Miri run of one seqlock test took about 10 s in research |

---

## Sampling Rate

- **After every task commit:** Run `nix develop -c cargo xtask test`
- **After every plan wave:** Run `test`, `check`, `clippy`, and `cargo xtask miri` when `ds/src/tpht/` changed
- **Before `/gsd-verify-work`:** Full suite green twice (IG-V01), and the kernel sha256 comparison recorded
- **Max feedback latency:** about 120 seconds (first build excluded)

---

## Per-Task Verification Map

Task IDs are filled in when the plans exist; rows are keyed by requirement until then.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | CRYP-01 | — | HMAC-SHA256 matches RFC 4231 TC1 to TC7 | unit (vectors) | `cargo test -p crypto --test rfc4231` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CRYP-01 | — | HKDF-SHA256 matches RFC 5869 TC1 to TC3; over-long output rejected | unit (vectors) | `cargo test -p crypto --test rfc5869` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CRYP-01 | — | Same vectors on the scalar backend the kernel runs | unit (vectors) | soft-cfg `crypto` leg of `cargo xtask test` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CRYP-02 | — | ChaCha20 block function, RFC 8439 §2.3.2 | unit (vector) | `cargo test -p crypto --test rfc8439` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CRYP-02 | — | Encryption RFC 8439 §2.4.2; decrypt round trip; counter overflow is an error | unit | `cargo test -p crypto --test rfc8439` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CRYP-03 | — | BLAKE3 `hash` and `keyed_hash`, all 35 official cases, 32-byte output | unit (vectors) | `cargo test -p crypto --test blake3_vectors` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CRYP-03 | — | Truncated keyed tag verifies; one flipped bit is rejected | unit | `cargo test -p crypto --test blake3_vectors` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CRYP-04 | — | `K_file` known answer; a different `Qid.path` or `Qid.vers` gives a different key | unit | `cargo test -p crypto --test keys` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CRYP-04 | — | `ReadWrite` seals and verifies; `ReadOnly` verifies and has no seal | unit + `compile_fail` doctest | `cargo test -p crypto --test keys`; `cargo test -p crypto --doc` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | CRYP-04 | — | Read-only sibling keeps the parent's `k_dec`; keys are `Zeroizing` | unit | `cargo test -p crypto --test keys` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-01 | — | Header is 64 bytes, 64-aligned | unit | `cargo test -p ds tpht::` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-01 | — | Home-hit fraction over 0.90 at target occupancy, per key distribution | unit (measurement pin) | `cargo test -p ds tpht::hit_rate` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-01 | — | Seqlock readers never see a torn entry | Miri + native stress | `cargo xtask miri`; `cargo test -p ds tpht::stress` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-02 | — | Feistel round trip; known answer for `f` | unit | `cargo test -p ds tpht::feistel` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-03 | — | `deref_tiny`: one arena read, zero header reads; agrees with a model | unit (model-based) | `cargo test -p ds tpht::deref_tiny` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-04 | — | Banks over 95% fill before the first failed claim; full-key verification | unit | `cargo test -p ds tpht::chained` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-04 | — | `WRITING` (0xFF) publication never exposes a partial node | Miri + native stress | `cargo xtask miri` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-05 | — | Union, intersection, difference, `andnot` equal the `BTreeSet` model and the `roaring` crate | unit (differential) | `cargo test -p ds roaring::` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-06 | — | `Empty` → `Single` → `Inline` to four, `Roaring` at five; contiguous ids give one run container | unit | `cargo test -p ds clientset::` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | DS-07 | — | No Funnel module; the crate docs carry the restriction | static check | `test ! -e ds/src/funnel.rs && grep -q -i funnel ds/src/lib.rs` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | LINT-01 | — | Both crates opt in; clippy rejects an undocumented unsafe block | lint | `nix develop -c cargo xtask clippy` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | LINT-01 | — | Both crates compile for both r9 kernel target specs | cross compile | `nix develop -c cargo xtask check` (new legs) | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | LINT-01 | — | `crypto/` has no `unsafe`; `ds/` has it only under `src/tpht/` | static check | `! grep -rnw unsafe crypto/src; ! grep -rnw unsafe ds/src --exclude-dir=tpht` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crypto/Cargo.toml`, `crypto/src/lib.rs`, `ds/Cargo.toml`, `ds/src/lib.rs`; workspace `members`
- [ ] xtask: `crypto` and `ds` test legs, the soft-cfg `crypto` leg, a `--doc` leg if doctests are used, host and cross check legs against `lib/*-unknown-none-elf.json`, the Miri subcommand
- [ ] `flake.nix`: a second toolchain derivation with Miri, exported as `R9_MIRI_TOOLCHAIN` (never added to `rust-toolchain.toml`, which would change kernel bytes)
- [ ] `crypto/tests/vectors/blake3_test_vectors.json` (the official file)
- [ ] A RED pin for every row above, each committed before its implementation
- [ ] Baseline kernel sha256 values recorded before the first workspace change

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `UNSAFE-AUDIT.md` lists every `tpht` unsafe site | LINT-01 | The audit is a hand-kept ledger reconciled by count | Compare the audit's `ds/` section against `grep -rnw unsafe ds/src/tpht` |
| Kernel bytes unchanged (D-21) | LINT-01 | Evidence step with recorded hashes on one venue | Compare the sha256 of the aarch64 `virtshell` and x86_64 `q35churnshell` kernels before and after |
| The nix-sandboxed kernel derivations build with the enlarged `Cargo.lock` | LINT-01 | Cannot be built on the Mac | One build on the owner's Linux host or in CI |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
