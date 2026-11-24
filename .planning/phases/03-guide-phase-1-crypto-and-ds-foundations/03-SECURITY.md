---
phase: "03"
slug: "guide-phase-1-crypto-and-ds-foundations"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-24"
---

# Phase 3 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register authored at plan time across all 13 PLAN.md `<threat_model>` blocks. ASVS level 1, block-on high.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| crates.io → workspace | Third-party source enters the build through `Cargo.lock` | Pinned crate source and versions |
| host test build → kernel build | The host may link a different backend from the one a kernel runs | SHA/ChaCha backend code paths |
| Mac build → evidence | A hash is a claim about a build on one venue | Kernel ELF sha256 values |
| Mac → remote | A push publishes commits | Git history, evidence |
| owner Linux host → evidence | Verdicts are decided from lines the owner returns | Exit codes, kernel hashes, suite output |
| planning artifacts → implementation | A recorded value can drift from the code | Constants, versions, decision rows |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| 03-01 T-03-01 | Tampering | Kernel bytes changed by workspace/lock change | high | mitigate | Before/after sha256 at `@03-01`, `@03-12`, `@03-10`; Linux tip/base hashes equal | closed |
| 03-01 T-03-02 | Tampering | Host vectors pin a hardware backend the kernel never runs | medium | mitigate | Scalar-backend test leg + `SOFT-LEG` nm evidence (`PINS-03-01.md`) | closed |
| 03-01 T-03-03 | Denial of service | Vector/FP code compiled into an FP-free crate | medium | mitigate | `default-features = false`; blake3 `pure` + five `no_*`; cross legs (`PINS-03-12.md`) | closed |
| 03-01 T-03-04 | Tampering | Vector transcribed wrongly | medium | mitigate | RFC section/case cited; committed RED before GREEN (`PINS-03-01.md`) | closed |
| 03-01 T-03-SC | Tampering | Dependency installs | high | mitigate | Research audit (7 direct/19 transitive, OK); exact `=` pins; lock additions only; zeroize review row | closed |
| 03-02 T-03-05 | Tampering | Kernel bytes moved by adding Miri to the toolchain | high | mitigate | `toolchainMiri` derivation; lint rule R5; `rust-toolchain.toml` unchanged | closed |
| 03-02 T-03-06 | Elevation of privilege | `unsafe` or kernel dependency outside approved places | medium | mitigate | Lint-contract rules R2-R4 run before every leg; forbid attributes | closed |
| 03-02 T-03-07 | Repudiation | Miri run that executed no test | medium | mitigate | `tpht_test_count` refusal; `MIRI-REFUSAL` line (`PINS-03-02.md`) | closed |
| 03-02 T-03-08 | Spoofing | `R9_MIRI_TOOLCHAIN` pointing at an arbitrary directory | low | accept | Developer's own shell trust; nix-store default; documented (`03-02-SUMMARY.md`) | closed |
| 03-02 T-03-SC | Tampering | Dependency installs | low | accept | No crate added; Miri from locked rust-overlay | closed |
| 03-03 T-03-09 | Information disclosure | ChaCha20 counter wrap / keystream reuse | high | mitigate | Block budget checked before write; `CounterOverflow`; both test legs (`PINS-03-03.md`) | closed |
| 03-03 T-03-10 | Information disclosure | Nonce/counter reuse by a caller | medium | transfer | No default nonce; caller duty documented; Phase 4/5 own nonce rules | closed |
| 03-03 T-03-11 | Tampering | HKDF over-long output | medium | mitigate | `OutputTooLong` before output touched; `hkdf_expand_length_limits` | closed |
| 03-03 T-03-12 | Tampering | Wrong vector hiding a wrong implementation | medium | mitigate | RFC citations; scalar-backend leg runs the same pins | closed |
| 03-03 T-03-SC | Tampering | Dependency installs | low | accept | No package added; `Cargo.lock` unchanged | closed |
| 03-04 T-03-13 | Information disclosure | Tag comparison timing leak | medium | mitigate | `blake3::Hash` equality; no-early-exit fold for truncated tags; no slice `==` | closed |
| 03-04 T-03-14 | Information disclosure | Key bytes printed/returned | medium | mitigate | `Zeroizing<[u8;32]>`; no `Debug`/`Display`; accessor `pub(crate)` | closed |
| 03-04 T-03-15 | Elevation of privilege | `ReadOnly` holder computes a tag | medium | mitigate | No `seal` on `ReadOnly`; private fields; `compile_fail` doctest + twin; symmetric-MAC residual documented | closed |
| 03-04 T-03-16 | Tampering | Vector file altered/truncated | low | mitigate | Upstream sha256/size; 35-case assertion (`PINS-03-04.md`) | closed |
| 03-04 T-03-17 | Tampering | Two derivations collide through ambiguous `info` | medium | mitigate | Fixed 12-byte `K_file` info; labelled-expand pin binds every bit | closed |
| 03-04 T-03-18 | Tampering | MAC key reused as cipher key | medium | mitigate | Separate `k_dec`/`k_mac`; `mac_key_is_separate_from_cipher_key` | closed |
| 03-04 T-03-SC | Tampering | Dependency installs | low | accept | No package added; vector file sha-gated | closed |
| 03-05 T-03-19 | Tampering | Wrong set-algebra result | medium | mitigate | Differential tests vs BTreeSet + `roaring` oracle, all nine pairings | closed |
| 03-05 T-03-20 | Elevation of privilege | SIMD/`unsafe` entering Roaring early | medium | mitigate | Per-module forbids; lint rule R3; no `core::arch`/intrinsics in `ds/src` | closed |
| 03-05 T-03-21 | Denial of service | Per-insert rewrite cost for huge cohorts | low | accept | One container (≤8 KiB) per insert; documented; revisit Phase 6 | closed |
| 03-05 T-03-SC | Tampering | Dependency installs | low | accept | No dependency added; `roaring` pinned in 03-12 and audited | closed |
| 03-06 T-03-22 | Tampering | Torn/racing read from another generation | high | mitigate | All reader fields atomic; Boehm ordering; Miri 64 seeds; native stress | closed |
| 03-06 T-03-24 | Denial of service | Structured keys collapsing into one home block | medium | mitigate | Home index from mixed Feistel half; measurement in 03-08 | closed |
| 03-06 T-03-25 | Denial of service | Chosen keys flooding one home block | low | accept | No kernel links `ds/`; per-table seed deferred to Phase 4 | closed |
| 03-06 T-03-SC | Tampering | Dependency installs | low | accept | No package added | closed |
| 03-07 T-03-26 | Tampering | Reader returns a partially written node | high | mitigate | `WRITING` two-phase publication; payload before publish; Miri 64 seeds; stress | closed |
| 03-07 T-03-27 | Tampering | Two inserts claim the same node | high | mitigate | Compare-exchange claim; stress asserts every key and fill count | closed |
| 03-07 T-03-28 | Tampering | UB copied from listing 9.4 | medium | mitigate | All-atomic nodes; no `unsafe` in `chained.rs`; Miri | closed |
| 03-07 T-03-29 | Denial of service | Bank exhaustion | low | accept | `Err(BanksFull)`, never panic/drop; caller sizes banks | closed |
| 03-07 T-03-SC | Tampering | Dependency installs | low | accept | No package added | closed |
| 03-08 T-03-30 | Repudiation | Threshold chosen to pass, no basis | medium | mitigate | Rule committed before run (`b479d28`); raw tables (`4150faf`); constant tied to evidence | closed |
| 03-08 T-03-31 | Tampering | Header and chain from different generations | high | mitigate | One seqlock attempt for header + chain; Miri 64 seeds; stress with spill | closed |
| 03-08 T-03-32 | Denial of service | Structured keys defeat the home block | medium | mitigate | Five distributions in grid + pin; mixed-half index | closed |
| 03-08 T-03-SC | Tampering | Dependency installs | low | accept | No package added | closed |
| 03-09 T-03-33 | Repudiation | Planning choice recorded as an owner decision | medium | mitigate | Nine `for owner review` rows; eight `Decided (owner)` rows; gates counted | closed |
| 03-09 T-03-34 | Tampering | Later phase implements superseded wording | medium | mitigate | Dated RFC amendment; PROJECT rows name superseded wording | closed |
| 03-09 T-03-35 | Repudiation | Recorded value differs from code | low | mitigate | W6 row tied to `SPILL-THRESHOLD.md`; `SPILL_THRESHOLD = 16` in code | closed |
| 03-09 T-03-SC | Tampering | Dependency installs | low | accept | Documents only; no package | closed |
| 03-10 T-03-36 | Repudiation | `unsafe` site missing from the audit | medium | mitigate | Mechanical scan; `AUDIT-COUNT tpht source=1 audit=1`; one site at `flattened.rs:440` | closed |
| 03-10 T-03-37 | Tampering | Kernel bytes changed during the phase | high | mitigate | Fresh-target rebuild at `1be121c`, both IDENTICAL; Linux comparison PASS | closed |
| 03-10 T-03-38 | Tampering | Nix-sandboxed kernels fail/change with enlarged lock | medium | mitigate | Owner checklist §§3-5; Linux builds, four-hash comparison, ISO build/rebuild PASS | closed |
| 03-10 T-03-39 | Elevation of privilege | Checklist command escalates/writes outside work dir | medium | mitigate | Allowlist only (`git`, `nix build`, `nix develop -c cargo xtask`, `sha256sum`); dedicated `r9-p03-stop` dir | closed |
| 03-10 T-03-40 | Repudiation | Verdict written without a run | medium | mitigate | Verdicts only from returned `HEAD=`/`EXIT=` lines; 12/12 logs present | closed |
| 03-10 T-03-SC | Tampering | Dependency installs | low | accept | No package added | closed |
| 03-11 T-03-41 | Elevation of privilege | Push without owner approval / more than approved | high | mitigate | Two `blocking-human` checkpoints; fixed commands; unpushed list printed; `origin/main` checked | closed |
| 03-11 T-03-42 | Repudiation | Result measured at the wrong commit | medium | mitigate | Every log has one `HEAD=`/`EXIT=`; tip logs `95fe66a`, base logs `4beb5c8` | closed |
| 03-11 T-03-43 | Tampering | Linux kernel bytes differ although Mac identical | medium | mitigate | Tip/base sha256 equal; ISO build/rebuild exit 0, no hash mismatch | closed |
| 03-11 T-03-44 | Information disclosure | Credential in a returned log | low | mitigate | Checklist forbids credentials; no credential string appeared | closed |
| 03-11 T-03-SC | Tampering | Dependency installs | low | accept | No package added; sandboxed builds vendor audited pins | closed |
| 03-12 T-03-45 | Denial of service | Vector/FP/`std` compiled into kernel crates | medium | mitigate | Cross legs `-Z build-std=core,alloc --lib`; four `CROSS-LEG rc=0` lines | closed |
| 03-12 T-03-46 | Tampering | Kernel bytes changed by `ds/`/`roaring` lock | high | mitigate | Fresh-target rebuild at `25af196`, both IDENTICAL + byte `cmp` | closed |
| 03-12 T-03-47 | Elevation of privilege | `roaring` oracle linked into a kernel | medium | mitigate | `[dev-dependencies]` only; cross legs without `--tests`; lint rules R4/R6 | closed |
| 03-12 T-03-SC | Tampering | Dependency installs | high | mitigate | Research audit; exact pin; lock additions only (0 removed) | closed |
| 03-13 T-03-23 | Tampering | UB from an unchecked arena read | medium | mitigate | Index masked by `slot_mask`; arena `slot_mask + 1`; SAFETY comment; Miri 64 seeds | closed |
| 03-13 T-03-SC | Tampering | Dependency installs | low | accept | No package added | closed |

*Status: closed for all 60 register entries; 0 open at or above the `high` block threshold.*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward threats_open.*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party).*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-03-01 | 03-02 T-03-08 | `R9_MIRI_TOOLCHAIN` is set by the developer's own shell (same trust as `PATH`); the devshell default is a nix-store path | Owner (plan disposition) | 2026-11-24 |
| AR-03-02 | 03-02 T-03-SC | No crate added by the Miri step; the component comes from the locked `rust-overlay` input | Owner (plan disposition) | 2026-11-24 |
| AR-03-03 | 03-03 T-03-10 | Nonce/counter reuse is the caller's duty; the API has no default nonce and Phases 4/5 own the wire nonce rules (transfer, not ignored) | Owner (plan disposition) | 2026-11-24 |
| AR-03-04 | 03-03 T-03-SC | No package added; `Cargo.lock` unchanged | Owner (plan disposition) | 2026-11-24 |
| AR-03-05 | 03-04 T-03-SC | No package added; the BLAKE3 vector file is data, verified by sha256 | Owner (plan disposition) | 2026-11-24 |
| AR-03-06 | 03-05 T-03-21 | One container (≤8 KiB) scanned per insert in `ClientSet`; accepted for the lease path's read-heavy pattern; revisit in Phase 6 | Owner (plan disposition) | 2026-11-24 |
| AR-03-07 | 03-05 T-03-SC | No dependency added by this plan; `roaring` was pinned in 03-12 and audited | Owner (plan disposition) | 2026-11-24 |
| AR-03-08 | 03-06 T-03-25 | No kernel links `ds/` in this phase and no peer-chosen key reaches the table; per-table seed is a Phase 4 question | Owner (plan disposition) | 2026-11-24 |
| AR-03-09 | 03-06 T-03-SC | No package added | Owner (plan disposition) | 2026-11-24 |
| AR-03-10 | 03-07 T-03-29 | `Err(BanksFull)` is returned, never a panic or silent drop; bank sizing is the caller's decision | Owner (plan disposition) | 2026-11-24 |
| AR-03-11 | 03-07 T-03-SC | No package added | Owner (plan disposition) | 2026-11-24 |
| AR-03-12 | 03-08 T-03-SC | No package added | Owner (plan disposition) | 2026-11-24 |
| AR-03-13 | 03-09 T-03-SC | Documents only; no package added | Owner (plan disposition) | 2026-11-24 |
| AR-03-14 | 03-10 T-03-SC | No package added | Owner (plan disposition) | 2026-11-24 |
| AR-03-15 | 03-11 T-03-SC | No package added; sandboxed builds vendor the audited pins | Owner (plan disposition) | 2026-11-24 |
| AR-03-16 | 03-13 T-03-SC | No package added | Owner (plan disposition) | 2026-11-24 |

Notes (non-blocking, no threat status change):
- T-03-15's residual (symmetric MAC means a `ReadOnly` holder that also knows the MAC key could compute a tag) is documented in `crypto/src/key.rs` and `03-04-SUMMARY.md`; the type-level control is the delivered mitigation.
- T-03-39: the owner-stop checklist names `~/r9-p03-stop/`, while the returned logs show the runner's dedicated `.git/r9-p03-stop/` directory. The work stayed inside a dedicated `r9-p03-stop` directory with only allowlisted commands, so the mitigation holds; recorded for fidelity.
- Miri latency (230-402 s vs the 120 s feedback target) is a budget deviation accepted with coverage unchanged; it is not a security mitigation gap.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-24 | 60 | 60 | 0 | gsd-security-auditor (ASVS L1) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-24
