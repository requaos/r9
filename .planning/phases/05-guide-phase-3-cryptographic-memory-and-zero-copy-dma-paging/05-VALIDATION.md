---
phase: "5"
slug: "guide-phase-3-cryptographic-memory-and-zero-copy-dma-paging"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-25"
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from `05-RESEARCH.md` § Validation Architecture. The per-task map is filled from the plans.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in tests driven by `cargo xtask`; Miri for `ds/src/tpht/`; QEMU boot profiles through `cargo xtask boot-test`; composed gates through `cargo xtask gate` |
| **Config file** | workspace `Cargo.toml`; `.planning/config.json` (`build_command`, `test_command`); profile TOMLs under `x86_64/lib` and `aarch64/lib` |
| **Quick run command** | `nix develop -c cargo test -p port --lib <module>` (or `-p crypto`, `-p ds`) |
| **Full suite command** | `nix develop -c cargo xtask test && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy --arch x86-64 --config q35shell && nix develop -c cargo xtask clippy --arch aarch64 --config virtshell` |
| **Conditional** | `nix develop -c cargo xtask miri` when `ds/src/tpht/` changes (DEBT-04) |
| **Estimated runtime** | quick: seconds once built; full suite: not measured for this phase |

After DEBT-01, `cargo xtask test` includes `test-9p` and `cargo test -p xtask`.

### Test layers

| Layer | Venue | What it can prove |
|-------|-------|-------------------|
| L1 host unit and integration (`crypto`, `ds`, `port`) | any host | derivation vectors, seal and open round trips, tag binding, pools, pager policy, stride, superpage, in-flight, scatter receive, retained key lifetime |
| L2 arch-crate host tests | host of the same architecture | `fpu` format selection, restricted-section register preservation, vector equals scalar. aarch64 on the Mac; x86_64 only on an x86_64 host |
| L3 lint and static | any host | lint contract as amended, clippy `undocumented_unsafe_blocks` on all gated crates, kernel-spec cross legs |
| L4 aarch64 boot profiles | this host (TCG, HVF) | sealed fault, eviction, tamper kill, NEON preservation on a booted kernel |
| L5 x86_64 boot profiles, TCG | Linux x86_64 or amd64 container | default boots unchanged on `qemu64` (FXSAVE path); `q35lowmem`, `q35crypt` on `qemu64` (scalar) and on the AVX2 model (vector, XSAVE) |
| L6 x86_64 KVM | owner's host | XSAVE on real hardware, MEM-13, boot-to-rio gate, `boot-iso` |
| L7 evidence scans | any host with the built ELFs | kernel bytes before and after, vector instructions only in allow-listed symbols, `.text` under 2 MiB |

---

## Sampling Rate

- **After every task commit:** the one test file the task touches, then `nix develop -c cargo xtask test`
- **After every plan wave:** the full suite; the aarch64 sealed profiles on this host; Miri if `tpht/` changed
- **Before each owner stop:** full suite green twice; kernel-bytes evidence with the symbol allow-list scan; aarch64 quick pass; the x86_64 TCG gates on a Linux venue
- **Before `/gsd-verify-work`:** all of the above plus the owner's KVM runs
- **Max feedback latency:** one task commit for host-testable work; one wave for boot profiles; the next owner stop for anything that needs KVM

---

## Per-Requirement Verification Map

"File Exists" is the state at HEAD before this phase. File and test names are proposals from research; the plans fix them. Task IDs are filled in once the plans exist.

| Requirement | Behaviour | Test Type | Automated Command | File Exists | Status |
|-------------|-----------|-----------|-------------------|-------------|--------|
| DEBT-01 | `cargo xtask test` runs `test-9p` and `cargo test -p xtask` and fails if either fails | unit + smoke | `cargo test -p xtask test_step`; `nix develop -c cargo xtask test` | ❌ W0 | ⬜ pending |
| DEBT-02 | archived `04-VALIDATION.md` is not `draft` and has no pending rows | doc check | grep count of `status: draft`, pending markers and `TBD` equals 0 | ❌ W0 | ⬜ pending |
| DEBT-03 | Phase 4 evidence text carries the dated correction | doc check | grep for the correction note in the archived owner-stop evidence | ❌ W0 | ⬜ pending |
| DEBT-04 | home-hit above 0.90 at the production bank count | unit | `cargo test -p ds tpht::measure`; Miri | partial | ⬜ pending |
| DEBT-05 | threat model names bits 18-21 as reserved | doc test | `cargo test -p xtask threat_model` | partial | ⬜ pending |
| MEM-01 | a frame is never mapped before its tag verifies; same-KeyID reuse skips scrub, cross-domain does not | unit | `cargo test -p port --lib palloc`, `segcrypt` with a recording `FrameOps` fake | ❌ W0 | ⬜ pending |
| MEM-01 | same, on a booted kernel | boot | `q35crypt` and aarch64 counterpart markers | ❌ W0 | ⬜ pending |
| MEM-02 | flipped ciphertext byte or tag: frame zeroed, fault returns kill; MAC key differs from cipher key | unit | `cargo test -p port --lib segcrypt tamper` | ❌ W0 | ⬜ pending |
| MEM-02 | tamper kills the process; kernel and siblings live | boot | `cargo xtask boot-test --arch x86-64 --config q35crypt` | ❌ W0 | ⬜ pending |
| MEM-03 | one cached line, 63 non-temporal, `SFENCE` after the loop | static scan + unit | disassembly check of the cipher symbol; output equality for every fault-line index 0..63 | ❌ W0 | ⬜ pending |
| MEM-04 | vector output equals scalar | arch host test | `cargo test -p x86_64 cryptpage` (x86_64 host); `cargo test -p aarch64 cryptpage` | ❌ W0 | ⬜ pending |
| MEM-04 | six registers preserved for the interrupted context and scrubbed inside | arch host test + boot self-test | `fpu` restricted-section tests; boot line in `q35crypt` under the AVX2 model | ❌ W0 | ⬜ pending |
| MEM-04 | user `ymm` survives switch, note, `rfork`; reset on exec | boot (generated ring-3 probe, D-34) | `q35crypt` leg under the AVX2 TCG model and under KVM | ❌ W0 | ⬜ pending |
| MEM-04 | no `XSAVE`/`XRSTOR` reachable from the fault or seal path; AVX-512 off | static scan | symbol allow-list scan; boot line printing `XCR0` | ❌ W0 | ⬜ pending |
| MEM-05 | oldest, middle, newest comparison detects a sequential stream on every fault | unit | `cargo test -p port --lib segcrypt stride` | ❌ W0 | ⬜ pending |
| MEM-06 | recycle rule and the flush gate | unit | `cargo test -p port --lib palloc recycle`; CPUID decision helpers | ❌ W0 | ⬜ pending |
| MEM-07 | address expression; pinned never evictable; `Evictable = Dirty \ (Pinned \| Active)` against a model; about 15% clean | unit | `cargo test -p port --lib palloc` | ❌ W0 | ⬜ pending |
| MEM-07 | vector set kernels equal scalar | unit | `cargo test -p ds setops` on aarch64 host (NEON) and x86_64 host (AVX2) | scalar only | ⬜ pending |
| MEM-08 | `size_of == 64`, `align_of == 64`; clear and reset | unit | `cargo test -p port --lib superpage` | ❌ W0 | ⬜ pending |
| MEM-09 | scatter receive leaves data pages as ciphertext in frames; tag at receipt; fault decrypts to the sent bytes | integration | `cargo xtask test-9p` with a new `port/tests/aead_ingress.rs` | ❌ W0 | ⬜ pending |
| MEM-09 | retained key unreachable by the packet path after cutover; zeroized when the last page goes | integration | `cargo test -p port --test aead_ingress retained_key` | ❌ W0 | ⬜ pending |
| MEM-09 | loopback probe: a user process faults an ingress page | boot | `q35crypt` probe leg | ❌ W0 | ⬜ pending |
| MEM-10 | a pinned frame is never in the eviction set; drop releases; overlapping pins count | unit | `cargo test -p port --lib pager pin`; `usermem` tests in both arch crates | partial | ⬜ pending |
| MEM-11 | four profiles pass; default boots print no sealed-paging marker | boot | `cargo xtask boot-test` for `q35lowmem`, `q35crypt` and the aarch64 pair, plus the existing profiles | ❌ W0 | ⬜ pending |
| MEM-11 | `x86_64` under the lint; audit counts reconcile | lint + doc check | `cargo xtask clippy --arch x86-64 --config q35shell`; a count check for `UNSAFE-AUDIT.md` | ❌ (32 failures today) | ⬜ pending |
| MEM-12 | one decrypt per page for duplicate requests; write-once | unit | `cargo test -p port --lib inflight` | ❌ W0 | ⬜ pending |
| MEM-13 | figures recorded | manual, owner's KVM host | benchmark profile; report under the phase evidence directory | manual | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] DEBT-01 itself, so that later RED pins in `xtask` tests and `test-9p` are part of the default run
- [ ] `x86_64` lint opt-in (32 comments) and the 7 aarch64 comments, so new modules land under the lint
- [ ] `port/tests/aead_ingress.rs` and its entry in the `test-9p` test list
- [ ] A recording fake `FrameOps` and a fake `BackingStore` under `port/tests/common/`
- [ ] Profile TOMLs, `BootProfile` variants and marker constants for the four sealed profiles
- [ ] A generated ring-3 `ymm` probe and its manifest entries (amd64); the aarch64 equivalent for NEON
- [ ] The symbol allow-list scan that replaces the Phase 4 "FP/SIMD scan equal to baseline" verdict
- [ ] A written procedure for the Linux x86_64 venue (container or host) used by L2 and L5

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Cycles per cold fault (scalar versus vector) and eviction cost | MEM-13 | Needs KVM on the owner's x86_64 Linux host | Run the benchmark profile under KVM at owner stop 2; record the report under the phase evidence directory, with cipher time and MAC time separated |
| `XSAVEOPT` cost at process switch against FXSAVE | MEM-04 (D-13, D-37 assumption) | Not measurable under TCG, which always writes every component | Measure under KVM at owner stop 1; revise later plans if it contradicts the assumption |
| `boot-iso` `outputHash` re-derivation and boot-to-rio under KVM | golden invariant, D-26 | Owner's host and owner's check by standing rule | At both owner stops: re-derive and commit the hash, run the x64 TCG gates, owner's boot-iso check |
| Gate-time cost of the AVX2 TCG CPU model | MEM-04 (D-14) | Needs an x86_64 boot | Time the AVX2 TCG leg against an ordinary boot-test at owner stop 1 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency within the bounds stated under Sampling Rate
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
