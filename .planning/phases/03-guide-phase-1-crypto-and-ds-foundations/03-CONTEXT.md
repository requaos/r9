# Phase 3: Guide Phase 1 - crypto/ and ds/ Foundations - Context

**Gathered:** 2026-11-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Two new workspace crates exist, pinned and host-tested. `crypto/` provides HKDF-SHA256 (on HMAC-SHA256), scalar ChaCha20, BLAKE3 keyed and tree modes, the key hierarchy and the `KeyCapability` typestate. `ds/` provides TPHT (`FlattenedTphtShard`, `ChainedTphtOverflow`, Feistel quotienting, `deref_tiny`), a Roaring bitmap and `ClientSet`. The lint and audit contract for both crates is settled and enforced by `cargo xtask check`, `clippy` and `test`.

Requirements: CRYP-01 to CRYP-04, DS-01 to DS-07, LINT-01. Open items settled here: W3, W5, W6, N1, and N2 as far as it concerns the two new crates.

Not in this phase: any wire format or `FidTable` (Phase 4), page sealing, the vector fast path or `segcrypt.rs` (Phase 5), linking either crate into a kernel, any `port/src` edit, any kernel byte change, warnings W1, W2, W4 and W7 to W10.

</domain>

<decisions>
## Implementation Decisions

### Carried in (not re-asked)
- **D-01:** Roaring is the integer set algebra (IG-D10, locked 2026-11-23).
- **D-02:** Every requirement follows IG-V01: committed RED pin, implementation, GREEN twice, pin advance.
- **D-03:** Owner standing rules apply unchanged: no push without approval, `port/src` edits need the diff shown first, no AI attribution in commits. The compendium's appendix listings (9.3, 9.4 and the rest) are non-normative sketches and are not copied as-is (INGEST-CONFLICTS I5).
- **D-04:** External crates are pinned from crates.io with the committed `Cargo.lock` as the supply-chain record, and each gets a provenance comment in `Cargo.toml` (the `smoltcp` precedent in `port/Cargo.toml`).

### MAC and ChaCha20 construction (W3, N1)
- **D-05 (W3 settled):** Keyed BLAKE3 is the only MAC in r9: wire packets, page sealing and capability tags. Poly1305 is not implemented. The "/ Poly1305" wording in guide section 6.3 and compendium sections 1.2 and 3.2 is recorded as corrected. — **Reversibility:** costly — the 32-byte MAC field of the Phase 4 wire format and the Phase 5 page tags are built on this choice.
- **D-06 (N1 settled):** ChaCha20 uses the RFC 8439 layout: 32-bit block counter, 96-bit nonce. The pins are the RFC 8439 vectors (block function, section 2.3.2; encryption, section 2.4.2). The RFC's "64-bit block counters and 96-bit nonces" is an error. How the 8-byte wire sequence nonce is extended to 12 bytes is decided in Phase 4, not here. — **Reversibility:** costly — the counter and nonce layout is part of the wire and page formats once Phases 4 and 5 use it.
- **D-07:** "HMAC-BLAKE3" (RFC sections 5 and 5.x, guide section 6.2, the `StatelessCap` tag) is read as BLAKE3's native keyed mode with the output truncated. No HMAC construction is built over BLAKE3. HMAC exists in `crypto/` only over SHA-256, for HKDF.
- **D-08:** The corrections are recorded in two places: PROJECT.md (W3 and N1 leave Open Questions and become dated Key Decisions rows naming the superseded wording), and `.planning/rfc-9p2000-aead.md` itself is amended: section 3.1 counter and nonce wording per D-06, and the Poly1305 reading removed so the RFC states Keyed BLAKE3 only. The guide and compendium texts are not edited. STATE.md's open-question table is updated to match.

### Unsafe and lint contract (W5, N2)
- **D-09 (W5 settled, as amended later in the discussion):** `ds/` is not crate-level `forbid(unsafe_code)`. It uses per-module forbids, the same shape as `port/` at HEAD. Exactly two places in `ds/` may hold `unsafe`: `ds/src/tpht/` (in this phase) and one small, isolated Roaring SIMD module (reserved; no code in this phase, see D-17). Every other `ds/` module carries `forbid(unsafe_code)`. A third unsafe-holding module in `ds/` needs owner approval. The guide's section 11 crate-level forbid wording for `ds/` is recorded as corrected.
- **D-10:** `crypto/` is crate-level `#![forbid(unsafe_code)]`. Any `unsafe` in `crypto/` itself needs owner approval. Because of D-18 this forbid covers r9's own code in `crypto/`, not the third-party crates it wraps.
- **D-11 (N2 settled for this phase):** `crypto/` and `ds/` take `[lints] workspace = true`, so `undocumented_unsafe_blocks = deny` applies to both. `x86_64` stays opted out; extending the lint and audit to it is Phase 5's question when `shadow.rs` and `cryptpage.rs` land. `UNSAFE-AUDIT.md` gains a `ds/` section listing each `tpht` unsafe site, and a `crypto/` line recording zero sites.
- **D-12:** No owner diff stop for `tpht` unsafe. `ds/` is treated like the arch crates: every site has a `// SAFETY:` comment (lint-enforced), is listed in `UNSAFE-AUDIT.md`, and is covered by the phase code review.
- **D-13:** The concurrent and unsafe parts of TPHT (seqlock readers, the two-phase `WRITING` publication, unchecked reads) are verified by Miri plus multi-threaded stress tests. Miri runs as its own xtask step so the ordinary `cargo xtask test` run stays fast. No `loom`.

### TPHT shape and Funnel (W6, DS-07, IG-D09)
- **D-14 (W6 settled):** The home block header keeps the 16-slot layout of guide section 5.2 and compendium section 4.2 (`occupied_mask` bits 0..15, `quotients [u16; 16]`, `tiny_ptrs [u8; 16]`, 64 bytes, `align(64)`). The "4-slot capacity" phrase in guide section 5.3 is not taken as the layout. The spill threshold (how many home slots fill before entries go to the dereference banks) is measured, not assumed.
- **D-15:** Claude runs the spill-threshold measurement on the host, chooses the threshold that meets over 90% home-block hits at the highest occupancy, fixes it as one named constant, and records the measurement as evidence. There is no owner stop; the owner reviews the evidence at phase verification. The success-criterion tests pin against the chosen constant.
- **D-16 (DS-07):** Funnel Hashing is not built. DS-07 is met by absence. A short note in the `ds/` crate documentation records the restriction (write-once ephemeral scratchpads only; never the page cache, `FidTable` or a Venti index) for whoever adds it later.
- **D-17:** IG-D09 is **locked on 2026-11-24** (owner), amended: succinct TPHT is the primary concurrent hashing and Funnel Hashing is scratchpad only; `ds/src/tpht/` is an unsafe-holding module under D-09; the home block header has 16 slots. The spill threshold is not part of the lock; it is fixed by D-15. The lock follows the Phase 2 rule (owner-only change; a change is a new dated row). The plan writes the PROJECT.md Key Decisions row with this date and removes "Not locked; deferred to Phase 3". — **Reversibility:** costly — `FidTable`, the page cache and the Venti index are all built on TPHT in later phases.

### Crate sourcing and kernel reach
- **D-18:** The cryptographic primitives come from pinned crates.io crates behind `crypto/`'s own API: SHA-256, HMAC and HKDF, ChaCha20, BLAKE3 and `zeroize`. They are built with default features off, in portable form: `no_std`, no SIMD or FP code paths, since kernels are built without FP/SIMD. `crypto/` exposes r9's own types and functions; callers never name the upstream crates. The vector pins (RFC 5869, RFC 4231, RFC 8439, the official BLAKE3 vectors) run against `crypto/`'s API.
- **D-19:** The Roaring bitmap is hand-written in `ds/src/roaring.rs` (array, bitmap and run containers). It is tested against two oracles on randomized inputs: a `BTreeSet` reference model and the crates.io `roaring` crate as a dev-dependency. The `roaring` crate never links into a kernel. TPHT and `ClientSet` are also written here.
- **D-20:** The owner expects Roaring to need SIMD for r9's use (the pager's set difference, guide section 5.4). Phase 3 ships scalar container operations with the hot loops (bitmap-container and, or, andnot; array intersection) behind one internal seam that a SIMD module can replace. The `unsafe` allowance for that module is approved now (D-09). No SIMD code and no `unsafe` lands in Roaring in this phase; the SIMD kernels arrive in Phase 5, when the pager needs them and they can be reached through `fpu::kernel_begin()` / `with_simd()`.
- **D-21:** Both crates are `no_std` (with `alloc` where needed). `cargo xtask check` also compiles them for `x86_64-unknown-none` and `aarch64-unknown-none`, which proves the pinned crates build in portable form. No kernel crate (`port`, `x86_64`, `aarch64`) depends on `crypto/` or `ds/` in this phase, so kernel bytes do not change and the `boot-iso` `outputHash` procedure does not apply. Phases 4 and 5 add the dependency. If any step of this phase is found to change kernel bytes, that is a stop.
- **D-22:** The key hierarchy (`K_file`, transient and session key derivation) and the `KeyCapability` typestate live in `crypto/`, host-tested there. Phase 5's `port/src/segcrypt.rs` imports the type instead of defining it; the guide's landing site (section 4.1) is recorded as moved. No `port/src` edit in this phase.

### Claude's Discretion
- **Crate build fallback:** if research or the cross-compile shows a pinned crate cannot build for the kernel targets in portable, `no_std`, FP/SIMD-free form, that one primitive is hand-written in `crypto/` and pinned to the same vectors, and the summary reports it. The rest of D-18 stands.
- **Audit treatment of third-party code:** `UNSAFE-AUDIT.md` records each new dependency with its version, feature set and the fact that it contains upstream `unsafe`; it does not enumerate upstream sites.
- The rule and evidence format for the spill-threshold measurement (D-15): thresholds tried, occupancies, key distributions.
- Module layout inside both crates beyond the names the requirements give (`crypto/src/kdf.rs`, `crypto/src/page.rs`, `ds/src/tpht/flattened.rs`, `ds/src/tpht/chained.rs`, `ds/src/roaring.rs`); the shape of the Roaring seam; the name of the Miri xtask step.
- Exact crate versions and feature flags; how `cargo xtask test`, `check` and `clippy` are extended to the two crates; the order of work and commits.
- Constant-time tag comparison and zeroization details inside `crypto/`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` (Phase 3 section, line 169) — goal, four success criteria, open questions W3, W5, W6, notes N1, N2
- `.planning/REQUIREMENTS.md` (lines 31-50) — CRYP-01 to CRYP-04, DS-01 to DS-07, LINT-01
- `.planning/PROJECT.md` — Constraints (safety, FP/SIMD, verification discipline), Open Questions table, notes N1 and N2, Key Decisions rows for IG-D07, IG-D09, IG-D10; the rows D-08 and D-17 rewrite
- `.planning/STATE.md` — standing owner rules; the open-question table D-08 updates
- `.planning/INGEST-CONFLICTS.md` (lines 27-45) — full text of W3, W5, W6

### Source documents
- `.planning/r9-implementation-guide-amended-v3.md` §2 (line 21, decisions table), §3 (line 43, the 17-correction ledger: `kdf.rs`, `simd.rs`, `funnel.rs`, `scoreindex.rs` rows), §4.1 (line 73, `KeyCapability`), §5.1 to §5.4 (lines 125-165, Feistel, header layout, dereference banks, Roaring and `ClientSet`), §6.1 (line 171, `K_file` derivation), §8 Phase 1 row (line 259), §11 (line 335, verification matrix and lint contracts)
- `.planning/r9-architecture-and-code-v4.md` §4.2 and §4.3 (lines 143-160, header layout, seqlock, neighbourhood displacement), listings §9.3 and §9.4 (lines 753, 901; non-normative sketches, not to be copied)
- `.planning/rfc-9p2000-aead.md` §3.1 (line 63, primitives; amended by D-08), §3.2 (lines 78-85, key derivations), §5 (lines 185-243, `StatelessCap` and "HMAC-BLAKE3"), §11 (line 541, nonce reuse), §13 (line 566, references)
- `.planning/intel/constraints.md` — IG-D07 (line 47), IG-D09 (57), IG-A01 (82), IG-M05 to IG-M08 (125-140), IG-V01 (195), RFC-P01 (260), RFC-P08 (295)

### External standards (vector sources)
- RFC 5869 (HKDF test vectors), RFC 4231 (HMAC-SHA256 vectors), RFC 8439 §2.3.2 and §2.4.2 (ChaCha20 vectors), the official BLAKE3 `test_vectors.json` (keyed and tree modes), Lemire et al. arXiv:1603.06549 (Roaring)

### Workspace, lint and audit at HEAD
- `Cargo.toml` — workspace members (add `crypto`, `ds`), `[workspace.lints.clippy] undocumented_unsafe_blocks = "deny"` and the scope comment that must be updated
- `port/Cargo.toml` — the `[lints] workspace = true` opt-in and the `smoltcp` provenance comment that D-04 follows
- `UNSAFE-AUDIT.md` — counting rule, scope line and reconciliation table that D-11 extends
- `xtask/src/main.rs` (test legs near line 3555, check legs near line 3685) — where `port` is the only library crate tested and checked today
- `.planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-CONTEXT.md` — D-06 to D-08, the lock rule and the IG-D07 amendment this phase builds on

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `port/` is the model for a `no_std` library crate that is host-tested (`port/src/*_tests.rs`), lint-opted-in and per-module `forbid(unsafe_code)` with named floor modules.
- `xtask` already has per-package test, check and clippy legs and a host-triple fallback for the nix devshell (no rustup); the new crates join those lists.
- `.planning/config.json` sets the build and test commands: `nix develop -c cargo xtask check` and `nix develop -c cargo xtask test`.

### Established Patterns
- At HEAD there is no `crypto/`, `ds/` or `vendor/` directory, and no crypto crate, `zeroize`, `roaring`, `proptest` or `rand` in `Cargo.lock`. Everything in this phase is new.
- Only `aarch64` and `port` opt into the workspace deny lint; `x86_64` and `riscv64` do not. The workspace `Cargo.toml` comment and `UNSAFE-AUDIT.md` scope line both say so and need updating.
- The pinned toolchain is `nightly-2026-01-30`, supplied by the nix devshell. Whether that toolchain ships the Miri component is not yet known.
- Kernel builds are reproducible; a sha256 comparison of both kernels before and after proves D-21's "no kernel byte change".

### Integration Points
- Workspace `Cargo.toml` `members`; `xtask/src/main.rs` test, check and clippy legs, plus a new Miri step and the two bare-metal cross-compile checks.
- `UNSAFE-AUDIT.md` (new `ds/` and `crypto/` entries); PROJECT.md, STATE.md, REQUIREMENTS.md and the RFC for the recorded decisions.
- Later consumers, not touched now: Phase 4 `FidTable` on TPHT and the wire MAC; Phase 5 `port/src/segcrypt.rs` importing `KeyCapability`, the pager using Roaring.

### Research must confirm
- For each pinned crate (SHA-256, HMAC, HKDF, ChaCha20, BLAKE3, `zeroize`): the version, the exact features or cfg flags that give a portable, `no_std`, FP/SIMD-free build on `x86_64-unknown-none` and `aarch64-unknown-none` with the pinned nightly, and that the API exposes what D-06 needs (RFC 8439 counter and nonce) and what CRYP-03 needs (BLAKE3 keyed and tree modes, 32-byte output).
- That Miri is available in the nix devshell for the pinned nightly, or how to supply it.
- That the `roaring` crate works as a host-only dev-dependency oracle.

</code_context>

<specifics>
## Specific Ideas

- The owner's words on Roaring: hand-written in `ds/src/roaring.rs`, "but I think it will need SIMD to fit our usecase so make allowances for small isolated unsafe usage. Use `roaring` crate as a dev-dependency test oracle in addition to the BTreeSet".
- The owner chose pinned upstream crates for the cryptographic primitives over hand-written ones, and a hand-written Roaring over the upstream crate.
- The owner chose to lock IG-D09 now, before code exists, and to let Claude pick the spill threshold from measurement without a stop.
- The owner wants the RFC text itself corrected for W3 and N1, not only PROJECT.md.

</specifics>

<deferred>
## Deferred Ideas

- **Roaring SIMD kernels** (AVX2, NEON) in the reserved isolated module: Phase 5, with the pager and `with_simd()`. The unsafe allowance is already approved (D-09, D-20).
- **Funnel Hashing** for write-once scratchpads: built in whichever phase first has a scratchpad consumer (D-16).
- **Extending the deny lint and audit to `x86_64`**: Phase 5 (the rest of note N2).
- **The 8-byte wire sequence nonce to 12-byte ChaCha20 nonce rule**: Phase 4, with W1 and W4.
- **Linking `crypto/` and `ds/` into the kernels**: Phases 4 and 5, with the kernel byte-change procedure.

</deferred>

---

*Phase: 3-Guide Phase 1 - crypto/ and ds/ Foundations*
*Context gathered: 2026-11-24*
