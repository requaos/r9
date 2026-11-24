# Phase 3: Guide Phase 1 - crypto/ and ds/ Foundations - Research

**Researched:** 2026-11-24
**Domain:** `no_std` Rust cryptographic wrappers and concurrent data structures; cargo workspace, xtask, nix devshell, Miri
**Confidence:** HIGH for the build, toolchain and wiring findings (every one was built or run this session); MEDIUM for the TPHT design guidance (a Python model, not the Rust measurement D-15 calls for)

All probes ran in a scratch directory outside the repo with the repo's pinned toolchain (`nix develop /Users/neilskinner/code/r9 -c ...`, rustc `1.95.0-nightly (842bd5be2 2026-01-29)`, host `aarch64-apple-darwin`). Nothing in the repo was edited; `git status` is unchanged. One side effect outside the scratch directory: `cargo miri setup` wrote its sysroot cache to `~/Library/Caches/org.rust-lang.miri`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried in (not re-asked)
- **D-01:** Roaring is the integer set algebra (IG-D10, locked 2026-11-23).
- **D-02:** Every requirement follows IG-V01: committed RED pin, implementation, GREEN twice, pin advance.
- **D-03:** Owner standing rules apply unchanged: no push without approval, `port/src` edits need the diff shown first, no AI attribution in commits. The compendium's appendix listings (9.3, 9.4 and the rest) are non-normative sketches and are not copied as-is (INGEST-CONFLICTS I5).
- **D-04:** External crates are pinned from crates.io with the committed `Cargo.lock` as the supply-chain record, and each gets a provenance comment in `Cargo.toml` (the `smoltcp` precedent in `port/Cargo.toml`).

#### MAC and ChaCha20 construction (W3, N1)
- **D-05 (W3 settled):** Keyed BLAKE3 is the only MAC in r9: wire packets, page sealing and capability tags. Poly1305 is not implemented. The "/ Poly1305" wording in guide section 6.3 and compendium sections 1.2 and 3.2 is recorded as corrected. — **Reversibility:** costly — the 32-byte MAC field of the Phase 4 wire format and the Phase 5 page tags are built on this choice.
- **D-06 (N1 settled):** ChaCha20 uses the RFC 8439 layout: 32-bit block counter, 96-bit nonce. The pins are the RFC 8439 vectors (block function, section 2.3.2; encryption, section 2.4.2). The RFC's "64-bit block counters and 96-bit nonces" is an error. How the 8-byte wire sequence nonce is extended to 12 bytes is decided in Phase 4, not here. — **Reversibility:** costly — the counter and nonce layout is part of the wire and page formats once Phases 4 and 5 use it.
- **D-07:** "HMAC-BLAKE3" (RFC sections 5 and 5.x, guide section 6.2, the `StatelessCap` tag) is read as BLAKE3's native keyed mode with the output truncated. No HMAC construction is built over BLAKE3. HMAC exists in `crypto/` only over SHA-256, for HKDF.
- **D-08:** The corrections are recorded in two places: PROJECT.md (W3 and N1 leave Open Questions and become dated Key Decisions rows naming the superseded wording), and `.planning/rfc-9p2000-aead.md` itself is amended: section 3.1 counter and nonce wording per D-06, and the Poly1305 reading removed so the RFC states Keyed BLAKE3 only. The guide and compendium texts are not edited. STATE.md's open-question table is updated to match.

#### Unsafe and lint contract (W5, N2)
- **D-09 (W5 settled, as amended later in the discussion):** `ds/` is not crate-level `forbid(unsafe_code)`. It uses per-module forbids, the same shape as `port/` at HEAD. Exactly two places in `ds/` may hold `unsafe`: `ds/src/tpht/` (in this phase) and one small, isolated Roaring SIMD module (reserved; no code in this phase, see D-17). Every other `ds/` module carries `forbid(unsafe_code)`. A third unsafe-holding module in `ds/` needs owner approval. The guide's section 11 crate-level forbid wording for `ds/` is recorded as corrected.
- **D-10:** `crypto/` is crate-level `#![forbid(unsafe_code)]`. Any `unsafe` in `crypto/` itself needs owner approval. Because of D-18 this forbid covers r9's own code in `crypto/`, not the third-party crates it wraps.
- **D-11 (N2 settled for this phase):** `crypto/` and `ds/` take `[lints] workspace = true`, so `undocumented_unsafe_blocks = deny` applies to both. `x86_64` stays opted out; extending the lint and audit to it is Phase 5's question when `shadow.rs` and `cryptpage.rs` land. `UNSAFE-AUDIT.md` gains a `ds/` section listing each `tpht` unsafe site, and a `crypto/` line recording zero sites.
- **D-12:** No owner diff stop for `tpht` unsafe. `ds/` is treated like the arch crates: every site has a `// SAFETY:` comment (lint-enforced), is listed in `UNSAFE-AUDIT.md`, and is covered by the phase code review.
- **D-13:** The concurrent and unsafe parts of TPHT (seqlock readers, the two-phase `WRITING` publication, unchecked reads) are verified by Miri plus multi-threaded stress tests. Miri runs as its own xtask step so the ordinary `cargo xtask test` run stays fast. No `loom`.

#### TPHT shape and Funnel (W6, DS-07, IG-D09)
- **D-14 (W6 settled):** The home block header keeps the 16-slot layout of guide section 5.2 and compendium section 4.2 (`occupied_mask` bits 0..15, `quotients [u16; 16]`, `tiny_ptrs [u8; 16]`, 64 bytes, `align(64)`). The "4-slot capacity" phrase in guide section 5.3 is not taken as the layout. The spill threshold (how many home slots fill before entries go to the dereference banks) is measured, not assumed.
- **D-15:** Claude runs the spill-threshold measurement on the host, chooses the threshold that meets over 90% home-block hits at the highest occupancy, fixes it as one named constant, and records the measurement as evidence. There is no owner stop; the owner reviews the evidence at phase verification. The success-criterion tests pin against the chosen constant.
- **D-16 (DS-07):** Funnel Hashing is not built. DS-07 is met by absence. A short note in the `ds/` crate documentation records the restriction (write-once ephemeral scratchpads only; never the page cache, `FidTable` or a Venti index) for whoever adds it later.
- **D-17:** IG-D09 is **locked on 2026-11-24** (owner), amended: succinct TPHT is the primary concurrent hashing and Funnel Hashing is scratchpad only; `ds/src/tpht/` is an unsafe-holding module under D-09; the home block header has 16 slots. The spill threshold is not part of the lock; it is fixed by D-15. The lock follows the Phase 2 rule (owner-only change; a change is a new dated row). The plan writes the PROJECT.md Key Decisions row with this date and removes "Not locked; deferred to Phase 3". — **Reversibility:** costly — `FidTable`, the page cache and the Venti index are all built on TPHT in later phases.

#### Crate sourcing and kernel reach
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

### Deferred Ideas (OUT OF SCOPE)
- **Roaring SIMD kernels** (AVX2, NEON) in the reserved isolated module: Phase 5, with the pager and `with_simd()`. The unsafe allowance is already approved (D-09, D-20).
- **Funnel Hashing** for write-once scratchpads: built in whichever phase first has a scratchpad consumer (D-16).
- **Extending the deny lint and audit to `x86_64`**: Phase 5 (the rest of note N2).
- **The 8-byte wire sequence nonce to 12-byte ChaCha20 nonce rule**: Phase 4, with W1 and W4.
- **Linking `crypto/` and `ds/` into the kernels**: Phases 4 and 5, with the kernel byte-change procedure.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CRYP-01 | RFC 5869 HKDF-SHA256 on HMAC-SHA256, RFC 5869 and RFC 4231 vectors | `hkdf =0.13.0` + `hmac =0.13.0` + `sha2 =0.11.1` built and passed RFC 5869 TC1 and RFC 4231 TC1, TC2 on the host; cross-built for both kernel specs (Findings F1, F2) |
| CRYP-02 | Scalar ChaCha20 in `crypto/src/page.rs`, RFC 8439 layout | `chacha20 =0.10.2` with feature `cipher` passed RFC 8439 2.3.2 and 2.4.2; its `soft` backend is what the kernel specs select (F1, F3) |
| CRYP-03 | BLAKE3 keyed and tree modes, 32-byte output, official vectors | `blake3 =1.8.7` with `pure` and the `no_*` features passed all 35 official cases for `hash` and `keyed_hash` (inputs to 102,400 bytes, so multi-chunk trees) (F1, F4) |
| CRYP-04 | Key hierarchy, `KeyCapability` typestate, `Zeroizing<[u8; 32]>` | `zeroize =1.9.2` builds `no_std`; `Zeroizing<Z: Zeroize + Clone>` is `Clone`; `Hkdf::from_prk` + `expand` give expand-only derivation; the read-only guarantee is type-level only (Pitfall 8) |
| DS-01 | `FlattenedTphtShard`, 64-byte header, seqlock, over 90% home hits | Header layout sums to 64 bytes with atomic fields (compiled, asserted); seqlock orderings tested under Miri (F7); hit-rate model and the index-bits finding (F9) |
| DS-02 | Feistel quotienting bijective | Round-trip confirmed for the one-round construction in a model; `f(R)` is elided in the sources and must be completed (Open Question 2) |
| DS-03 | `deref_tiny` exactly one memory access, model-based test | Counting-probe pattern (Architecture Pattern 4) |
| DS-04 | `ChainedTphtOverflow`, 255-node banks, `WRITING` 0xFF, over 95% fill | Listing 9.4 read; its `&T as *const T as *mut T` write is undefined behaviour and must not be copied (Pitfall 6) |
| DS-05 | Roaring matches a reference model on randomized inputs | `roaring` crate works as a host dev-dependency oracle (F8); `BTreeSet` is in `std` |
| DS-06 | `ClientSet`, zero heap to four clients | Variant and size assertions; a counting allocator would be a third unsafe site (Pitfall 9) |
| DS-07 | Funnel Hashing scratchpad only | Met by absence (D-16); a crate-doc note |
| LINT-01 | Lint contract, `UNSAFE-AUDIT.md`, xtask legs | xtask wiring read and exercised against a repo copy with both crates added (F5, F6) |
</phase_requirements>

## Summary

The owner's choice of pinned upstream crates (D-18) holds: every primitive builds `no_std` for both of r9's real kernel target specs and passes its published vectors on the host, so the hand-written fallback is not needed. The exact pins are `sha2 =0.11.1`, `hmac =0.13.0`, `hkdf =0.13.0`, `chacha20 =0.10.2` (feature `cipher`), `blake3 =1.8.7` (features `pure`, `no_sse2`, `no_sse41`, `no_avx2`, `no_avx512`, `no_neon`) and `zeroize =1.9.2`, all with `default-features = false`. A probe binary linked for `lib/x86_64-unknown-none-elf.json` and `lib/aarch64-unknown-none-elf.json` contains zero FP or SIMD register instructions.

Three findings change how the plan must be written. First, the built-in `aarch64-unknown-none` target that D-21 names has NEON enabled: the same probe linked for it contains 590 FP/SIMD instructions, so it proves nothing about an FP-free build. The proof has to use r9's own spec files with `-Z build-std=core,alloc`, which works and is what the kernel build already does. Second, Miri is not in the devshell, and adding the `miri` component to `rust-toolchain.toml` changes the toolchain's store path, which is embedded over a thousand times in each kernel; that is a kernel byte change and a D-21 stop. Miri must come from a separate toolchain derivation exported to xtask through an environment variable; that route was built and run. Third, Miri's weak-memory emulation fails the seqlock orderings used in compendium listing 9.3 in 18 of 64 seeds, while 50 native stress runs of the same code all pass; the listing's plain `u16` header fields are also a data race Miri reports as undefined behaviour. The header fields must be atomics and the reader must use the Boehm ordering.

Adding `crypto` and `ds` as workspace members, with all the new crates in `Cargo.lock`, leaves both kernels byte-identical on a fresh build (sha256 compared before and after). `cargo xtask check`, `clippy` and `test` still pass with the members added.

**Primary recommendation:** Pin the six crates as above; add host test legs, a soft-backend test leg for `crypto`, and cross-check legs against `lib/*-unknown-none-elf.json`; supply Miri as a second toolchain derivation behind `R9_MIRI_TOOLCHAIN`; build the TPHT header from atomics with the Boehm seqlock ordering; and take the home block index from the mixed half of the Feistel output.

## Findings That Contradict or Qualify CONTEXT.md

| # | CONTEXT item | Finding | Evidence |
|---|--------------|---------|----------|
| C1 | D-21: compiling for `aarch64-unknown-none` "proves the pinned crates build in portable form" | It does not. That built-in target has NEON on. The probe linked for it has 590 FP/SIMD instructions (363 with soft backends forced); linked for `lib/aarch64-unknown-none-elf.json` it has 0. Keep the D-21 legs if wanted, but the proof is the r9 spec files. | [VERIFIED: probe build + `llvm-objdump -d` scan, this session] |
| C2 | D-18: "built with default features off" | True for all six, but `chacha20` with defaults off exposes no cipher type; feature `cipher` must be turned back on. `blake3` needs `pure` plus five `no_*` features. | [VERIFIED: compile error `could not find ChaCha20 in chacha20 ... gated behind the cipher feature`, then green with the feature] |
| C3 | Code context: "Whether that toolchain ships the Miri component is not yet known" | The devshell has no Miri (`error: no such command: miri`). The rust-overlay manifest has it for this nightly on aarch64-darwin, x86_64-linux and aarch64-linux. Adding `miri` to `rust-toolchain.toml` moves the toolchain store path from `xdba9fkgy...` to `h2vrk7sa...`, and that path occurs 1042 times in the aarch64 kernel and 1036 times in the x86_64 kernel. | [VERIFIED: `nix eval`, `nix build`, `strings` on built kernels] |
| C4 | IG-M05 / compendium 4.1: home block index is "the lower log2(num_blocks) bits of P" | With one Feistel round, the low 32 bits of P are the key's low 32 bits unchanged. Keys that differ only in their upper half all land in one block. In a model, `PageKey`-style keys (64 images, sequential pages) get 25% home hits and `id << 32` keys get 0%, against 94% for random keys. Taking the index from the mixed half gives at least 94% for every distribution tried. This conflicts with DS-01's over-90% criterion for structured keys. | [VERIFIED: formula read in both source documents; consequence shown by `scripts/tpht_model.py` in scratch. The model is Python, not the D-15 measurement] |
| C5 | Success criterion 4: "`cargo xtask check` and clippy pass" | `cargo xtask clippy` (default arch) passes at HEAD and with both crates added. `cargo xtask clippy --arch aarch64 --config virtshell` already fails at HEAD with 7 undocumented unsafe blocks in `aarch64/` (`platform/virt/rtc.rs:75`, `proc.rs:439`, `470`, `1823`, `2753`, `textcopy.rs:158`, `172`). This is outside Phase 3, but the plan must say which clippy command the criterion means, and `UNSAFE-AUDIT.md`'s aarch64 count of 129 may be stale. | [VERIFIED: ran both commands on an unmodified copy of the working tree] |
| C6 | CRYP-04: `ReadOnly { k_dec, blake3_verify }` "cannot produce a valid integrity tag" | Keyed BLAKE3 is symmetric: the key that verifies a tag also computes it. The property can only be enforced by the type (no sealing method on `ReadOnly`, key bytes private), not by the cryptography. | [ASSUMED: follows from keyed BLAKE3 being a symmetric MAC; no source document addresses it] |

## Architectural Responsibility Map

This phase has no client, server or database tiers. The tiers that matter are crates and build targets.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| HKDF, HMAC, ChaCha20, BLAKE3, key types | `crypto/` (library, host-tested) | upstream crates behind it | D-18: callers never name upstream crates |
| TPHT, Roaring, `ClientSet` | `ds/` (library, host-tested) | — | D-19 |
| Test, check, clippy, Miri legs | `xtask/` | `flake.nix` devshell (supplies the Miri toolchain path) | xtask owns all gate commands; the flake owns toolchains |
| Lint enforcement | workspace `Cargo.toml` + each crate's `[lints]` | clippy | Existing contract |
| Unsafe inventory | `UNSAFE-AUDIT.md` | — | Existing contract |
| Decision records | `.planning/PROJECT.md`, `STATE.md`, `REQUIREMENTS.md`, `rfc-9p2000-aead.md` | — | D-08, D-17 |
| Kernels (`port`, `aarch64`, `x86_64`) | untouched | — | D-21: no dependency on the new crates this phase |

## Standard Stack

### Core (`crypto/` dependencies)

All versions and features below were resolved and built this session. Crate names are confirmed in each upstream project's own README and passed `gsd-tools query package-legitimacy check --ecosystem crates` with verdict `OK`. [VERIFIED: crates.io registry + upstream README + legitimacy seam]

| Library | Version | Published | Features | Purpose |
|---------|---------|-----------|----------|---------|
| `sha2` | `=0.11.1` | 2026-10-12 | `default-features = false` | SHA-256 for HMAC and HKDF |
| `hmac` | `=0.13.0` | 2026-03-29 | `default-features = false` | HMAC-SHA256 (RFC 4231 pins) |
| `hkdf` | `=0.13.0` | 2026-03-30 | `default-features = false` | HKDF-SHA256 (RFC 5869 pins) |
| `chacha20` | `=0.10.2` | 2026-08-27 | `default-features = false`, `features = ["cipher"]` | RFC 8439 ChaCha20 (`ChaCha20` type: 12-byte nonce, 32-bit counter) |
| `blake3` | `=1.8.7` | 2026-08-20 | `default-features = false`, `features = ["pure", "no_sse2", "no_sse41", "no_avx2", "no_avx512", "no_neon"]` | Keyed and unkeyed tree hash, 32-byte output |
| `zeroize` | `=1.9.2` | 2026-11-17 | `default-features = false` | `Zeroizing<[u8; 32]>` |

Declared minimum Rust versions: `sha2 0.11.1` 1.88; `hmac`, `hkdf`, `chacha20`, `zeroize` 1.85. All are at or below the pinned 1.95 nightly and all compiled on it. [VERIFIED: crates.io API `rust_version` fields + successful builds]

`chacha20 0.10.0` and `0.10.1` are yanked; `0.10.2` is the only non-yanked 0.10 release. [VERIFIED: crates.io API]

`zeroize 1.9.2` was published seven days before this research. If the owner prefers an older release, `1.9.1` (2026-10-06) has the same feature list. [VERIFIED: crates.io API; 1.9.1 not built]

Transitive crates that enter `Cargo.lock` through the six above: `digest 0.11.3`, `cipher 0.5.2`, `inout 0.2.6`, `crypto-common 0.2.2`, `block-buffer 0.12.1`, `hybrid-array 0.4.15`, `typenum 1.20.1`, `cpufeatures 0.3.1`, `cfg-if 1.0.5`, `arrayvec 0.7.8`, `constant_time_eq 0.4.2`, `ctutils 0.4.8`, `cmov 0.5.4`, `libc 0.2.191` (a `cpufeatures` dependency on some hosts), and the `blake3` build dependency `cc 1.11.3` with `shlex 2.0.1` and `find-msvc-tools 0.1.15`. All returned `OK` from the legitimacy seam. [VERIFIED: `cargo tree`, legitimacy seam]

### Supporting (`ds/` dev-dependency)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `roaring` | `=0.11.5` (2026-08-12) | Host-only test oracle (D-19) | `[dev-dependencies]` of `ds/` only |

`roaring 0.12.0` (2026-10-28) also works, but it adds `fearless_simd 1.2.0`, `fearless_simd_macros 0.1.0` (a proc macro) and `libm` to the lock. `0.11.5` needs only `bytemuck` and `byteorder`, and `byteorder` is already in `Cargo.lock`. For a test oracle the smaller, older release is the better pin. Both passed a union, intersection and difference smoke test on the pinned nightly. [VERIFIED: both versions built and tested; dependency lists from the crates.io API]

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `sha2 0.11` generation | `sha2 0.10.9` + `hmac 0.12` + `hkdf 0.12` | 0.10.9 has a `force-soft` Cargo feature instead of a `--cfg`; older, not built here. Not needed: see F2 |
| `chacha20` crate | hand-written ChaCha20 | Only the discretion fallback; the crate builds portable, so D-18 stands |
| `proptest` or `rand` for randomized tests | a seeded SplitMix64 or xorshift64* in test code | No new dependency, deterministic seeds, fast under Miri. `port/src/devip/conv.rs:756` already hand-rolls xorshift64* [VERIFIED: grep hit; line not opened with Read] |
| `subtle` for constant-time compare | `blake3::Hash`'s `PartialEq` | `blake3::Hash` equality already calls `constant_time_eq::constant_time_eq_32`; no extra crate for 32-byte tags [VERIFIED: `blake3-1.8.7/src/lib.rs:351-354`] |

**Installation (two new manifests; the provenance comment is required by D-04):**

```toml
# crypto/Cargo.toml
[dependencies]
sha2     = { version = "=0.11.1", default-features = false }
hmac     = { version = "=0.13.0", default-features = false }
hkdf     = { version = "=0.13.0", default-features = false }
chacha20 = { version = "=0.10.2", default-features = false, features = ["cipher"] }
blake3   = { version = "=1.8.7",  default-features = false, features = [
    "pure", "no_sse2", "no_sse41", "no_avx2", "no_avx512", "no_neon",
] }
zeroize  = { version = "=1.9.2", default-features = false }

[lints]
workspace = true
```

```toml
# ds/Cargo.toml
[dev-dependencies]
roaring = "=0.11.5"

[lints]
workspace = true
```

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `sha2` | crates.io | 10 yrs | 32.2M/wk | github.com/RustCrypto/hashes | OK | Approved |
| `hmac` | crates.io | 10 yrs | 19.6M/wk | github.com/RustCrypto/MACs | OK | Approved |
| `hkdf` | crates.io | 11 yrs | 10.0M/wk | github.com/RustCrypto/KDFs | OK | Approved |
| `chacha20` | crates.io | 10 yrs | 17.8M/wk | github.com/RustCrypto/stream-ciphers | OK | Approved |
| `blake3` | crates.io | 7 yrs | 5.2M/wk | github.com/BLAKE3-team/BLAKE3 | OK | Approved |
| `zeroize` | crates.io | 8 yrs | 21.8M/wk | github.com/RustCrypto/utils | OK | Approved |
| `roaring` (dev only) | crates.io | 11 yrs | 1.5M/wk | github.com/RoaringBitmap/roaring-rs | OK | Approved |

Transitive packages (19 checked, listed under Standard Stack): all `OK`. The two youngest are `ctutils` (first published 2025-12-18, 8.6M/wk) and `find-msvc-tools` (2025-08-29, 26.6M/wk); both are pulled by the approved crates, not chosen.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

`blake3` has a `build.rs` and a build dependency on `cc`. With feature `pure` the script selects the Rust implementations and compiles no C or assembly. [VERIFIED: `blake3-1.8.7/build.rs` lines 345-353 read: `is_pure()` routes to `build_sse2_sse41_avx2_rust_intrinsics()`; the devshell does have a `cc` on PATH, so the build alone does not prove this]

## Findings in Detail

### F1. All six crates build and pass their vectors on the pinned nightly

A scratch crate (`#![no_std]`, `#![forbid(unsafe_code)]`) wrapping the six crates passed, on the host: RFC 4231 test cases 1 and 2; RFC 5869 test case 1 (PRK and 42-byte OKM, plus rejection of an output longer than 255 × 32 bytes); RFC 8439 section 2.3.2 (block function, counter 1) and 2.4.2 (114-byte encryption, counter 1); and all 35 cases of the official BLAKE3 file for `hash` and `keyed_hash`, both one-shot and split across two `update` calls. [VERIFIED: `cargo test`, 6 passed]

API names confirmed in the vendored sources:
- `hkdf::Hkdf::<Sha256>::extract`, `from_prk`, `expand`, `expand_multi_info` [VERIFIED: `hkdf-0.13.0/src/lib.rs:104-174`]
- `hmac::Hmac<Sha256>` with `KeyInit::new_from_slice`, `Mac::update`, `finalize`, `verify_slice`, `verify_truncated_left` [VERIFIED: `digest-0.11.3/src/mac.rs:25-97`]
- `chacha20::ChaCha20` with `cipher::{KeyIvInit, StreamCipher, StreamCipherSeek}`; `try_seek` and `try_apply_keystream` return errors instead of panicking [VERIFIED: `cipher-0.5.2/src/stream.rs:128-247`]
- `blake3::hash`, `keyed_hash`, `Hasher::new_keyed`, `Hasher::finalize`, `OUT_LEN = 32`, `KEY_LEN = 32`, and the `hazmat` module (`merge_subtrees_root`, `merge_subtrees_non_root`, `left_subtree_len`) for explicit subtree work in Phase 5 [VERIFIED: `blake3-1.8.7/src/lib.rs:150-1398`, `src/hazmat.rs:298-560`]

The RFC 8439 counter is set by seeking to `counter * 64` bytes. [VERIFIED: both RFC 8439 vectors pass with this mapping]

### F2. On r9's kernel specs the crates select scalar code with no rustflags

r9's kernel specs, quoted from the files:

`lib/x86_64-unknown-none-elf.json:10-21`:
```
"os": "none",
"features": "-mmx,-sse,+soft-float",
"rustc-abi": "x86-softfloat"
```
`lib/aarch64-unknown-none-elf.json:2-7`:
```
"abi": "softfloat",
"features": "+strict-align,-neon,-fp-armv8",
```
[VERIFIED: both files opened with Read this session]

`cpufeatures 0.3.1` turns runtime CPU detection off on freestanding targets, so `sha2`, `chacha20` and `blake3` fall back to compile-time target features, which these specs disable:

`cpufeatures-0.3.1/src/x86.rs:17-22`:
```
#[cfg(not(any(target_env = "sgx", target_os = "none", target_os = "uefi")))]
$body
// CPUID is not available on SGX. Freestanding and UEFI targets
// do not support SIMD features with default compilation flags.
#[cfg(any(target_env = "sgx", target_os = "none", target_os = "uefi"))]
false
```
`cpufeatures-0.3.1/src/aarch64.rs:175-183`: `// On other targets, runtime CPU feature detection is unavailable` followed by a macro that expands to `false` for every target that is not Apple, Linux or Android. [VERIFIED: both read from the vendored registry source]

Measured on a `no_std`, `no_main` probe binary that calls every wrapper and is linked with `rust-lld`, built with `-Z build-std=core,alloc`:

| Target | Soft cfgs | Instructions | FP/SIMD register instructions | `cpuid`/`xgetbv` |
|--------|-----------|--------------|-------------------------------|------------------|
| `lib/x86_64-unknown-none-elf.json` | no | 55,449 | 0 | 0 |
| `lib/x86_64-unknown-none-elf.json` | yes | 55,449 | 0 | 0 |
| `lib/aarch64-unknown-none-elf.json` | no | 13,094 | 0 | — |
| `lib/aarch64-unknown-none-elf.json` | yes | 13,088 | 0 | — |
| built-in `x86_64-unknown-none` | no | 50,721 | 0 | 0 |
| built-in `aarch64-unknown-none` | no | 12,627 | **590** | — |
| built-in `aarch64-unknown-none` | yes | 11,162 | **363** | — |

[VERIFIED: `llvm-objdump -d` scans; x86 pattern `%[xyz]mm`, `%st(`, `%mm`; aarch64 pattern `v`/`q`/`d`/`s`/`z` registers]

Two things in the aarch64 binary look alarming and are not:
- Two `mrs x19, DIT` instructions come from `constant_time_eq::dit`. Without its `std` feature the crate decides by `cfg!(target_feature = "dit")`, which is false here, so those paths are present but not taken. [VERIFIED: `constant_time_eq-0.4.2/src/dit.rs:77-95` read]
- On x86_64 the binary still contains `blake3::sse2::*`, `sse41::*` and `avx2::*` symbols. They are compiled as scalar integer code (the soft-float target has no vector registers) and are never selected. They cost size: the x86_64 probe is 298 KB of text against 70 KB on aarch64. That matters when Phase 4 or 5 links `crypto/` into the x86_64 kernel (4.19 MB of text today); it does not matter in Phase 3. [VERIFIED: symbol table and `llvm-size`]

### F3. The soft-backend cfgs are optional for the kernel but useful in two places

`sha2 0.11.1` and `chacha20 0.10.2` have no Cargo feature to force scalar code; they read `--cfg sha2_backend="soft"` and `--cfg chacha20_backend="soft"`. [VERIFIED: `sha2-0.11.1/src/sha256.rs:1-4`, `chacha20-0.10.2/src/lib.rs:82-84`, both crates' `check-cfg` lists]

F2 shows the kernel does not need them. They are still worth using in two xtask legs:

1. **A second host test leg for `crypto`.** On this Mac the default test build links `chacha20`'s NEON backend and `sha2`'s hardware SHA path; an x86_64 host would link AVX2 and SHA-NI. The vectors therefore pin a different code path from the one the kernel will run. With the two cfgs the host test links `sha256::soft` and no NEON backend, and all six vector tests still pass. [VERIFIED: `nm` on both test binaries; soft run 6 passed]
2. **The aarch64 cross-check leg.** Without the cfgs, checking `crypto` for `lib/aarch64-unknown-none-elf.json` prints `the following packages contain code that will be rejected by a future version of Rust: core v0.0.0 ..., sha2 v0.11.1`. With the cfgs `sha2` drops off that list (`core` stays; it is there for the kernel builds today too). The toolchain is pinned, so this is a warning, not a failure. [VERIFIED: both runs]

The flags must be passed per leg with `--config build.rustflags=[...]`, not in `.cargo/config.toml`. A global entry would also reach the kernel builds, whose flags xtask sets through the same key (`xtask/src/config.rs:738-745`: `cmd.arg(format!("build.rustflags='{flat}'"))`, with the comment `!! this overrides the build.rustflags from the target Cargo.toml !!`). [VERIFIED: file read. ASSUMED: that a global rustflags entry would change kernel bytes; it was not tried because it is not needed]

`blake3` needs no cfg. Its `pure` and `no_*` features are Cargo features, so the host tests already run the portable code (`blake3::portable` is the only backend symbol in both host test binaries). [VERIFIED: `nm`]

### F4. BLAKE3 vectors: source, identity and shape

- File: `test_vectors/test_vectors.json` in `github.com/BLAKE3-team/BLAKE3`. sha256 `dcb91ea8accc77e6d6e632af7cdc1a99a9f3ae78cf648da595c7d064db32f624`, 31,922 bytes, identical at tags `1.0.0` and `1.8.7` and on `master`. [VERIFIED: three downloads hashed]
- Shape: top-level `key` (`whats the Elvish word for friend`), `context_string`, and 35 `cases` with `input_len`, `hash`, `keyed_hash`, `derive_key`. Each output is 131 bytes of extended output in hex (262 characters); the first 32 bytes are the default-length output. Input byte `i` is `i % 251`. Lengths: 0 to 8, 63 to 65, 127 to 129, 1023 to 1025, 2048, 2049, 3072, 3073, 4096, 4097, 5120, 5121, 6144, 6145, 7168, 7169, 8192, 8193, 16384, 31744, 102400. [VERIFIED: parsed the file]
- "Tree mode" needs no separate API: any input over 1024 bytes is hashed as a tree. Cases from 1025 bytes up are the tree-mode pins; `keyed_hash` over those lengths is the keyed tree mode. [CITED: github.com/BLAKE3-team/BLAKE3 README; VERIFIED that those cases pass]
- Licence of the BLAKE3 repository: `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`. [VERIFIED: upstream `Cargo.toml` at tag 1.8.7]

**How to commit it:** copy the JSON byte-for-byte to `crypto/tests/vectors/blake3_test_vectors.json`, record the sha256 and upstream tag in a header comment of the test file and in the phase's evidence file, and read it with `include_str!`. A 12-line extractor for the file's fixed shape was enough in the probe; no JSON crate is needed. Assert the case count is 35 so a truncated file cannot pass.

### F5. How `cargo xtask test`, `check` and `clippy` work today, and what changes

All from `xtask/src/main.rs`, opened with Read this session.

**`TestStep::run` (lines 3553-3627).** It first runs `xtask::rforkh::check(&workspace())?`, then builds a list of cargo argument vectors. The only library leg is:
```
"test", "--package", "port", "--lib", "--tests"
```
(lines 3562-3568). An arch-crate leg follows when a host target with `std` is found. Each leg runs through one loop that returns `Err("test failed".into())` on the first failure.

**`CheckStep::run` (lines 3686-3760).** A fixed list of four legs; the library one is:
```
"check", "--package", "port", "--lib", "--tests", "--benches"
```
(lines 3709-3716). The `--tests`/`--benches` legs for arch crates are skipped in the nix devshell because `RustupState` finds no rustup.

**`ClippyStep::run` (lines 3647-3667).** `cargo clippy --workspace` with the other architectures excluded by `exclude_other_arches` (lines 4791-4806). Because it uses `--workspace`, new members are linted with no xtask change. [VERIFIED: with both crates added to a repo copy, removing a `// SAFETY:` comment in `ds/` made the workspace clippy run fail with `error: unsafe block missing a safety comment` and `could not compile ds`]

**`BuildStep::run` (lines 433-459, read through the shell).** `cargo build --workspace --exclude xtask` plus the other-arch excludes and `-Z build-std=core,alloc`. So once `crypto` and `ds` are members, every kernel build also compiles them for the real kernel spec. [VERIFIED: the build log of the repo copy shows `Compiling` lines for `crypto`, `ds`, `blake3`, `sha2` and `chacha20` under both kernel builds]

**Kernel bytes with the members added.** A copy of the working tree was built twice at baseline (identical), then `crypto` and `ds` were added as members with the pins above (and `roaring 0.12.0` as the `ds` dev-dependency), `target/` was moved away, and both kernels were rebuilt from nothing:

| Kernel (`cargo xtask build`) | sha256 before | sha256 after |
|---|---|---|
| `--arch aarch64 --config virtshell` | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | identical |
| `--arch x86-64 --config q35churnshell` | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | identical |

`Cargo.lock` gained entries only; no existing entry changed (`diff` shows zero removed lines). [VERIFIED: this session]

These hashes are from a devshell build of a copy at a different path; they are evidence that the change is byte-neutral, not reference values for the repo. The plan must repeat the comparison in the repo (D-21's evidence) and again after any later dependency edit.

**With the members added, unchanged xtask:** `cargo xtask check` rc 0, `cargo xtask clippy` rc 0, `cargo xtask test` rc 0. [VERIFIED]

**Legs that worked when run by hand against the copy (all rc 0):**
```
cargo test  --package crypto --package ds --lib --tests
cargo check --package crypto --package ds --lib --target x86_64-unknown-none
cargo check --package crypto --package ds --lib --target aarch64-unknown-none
cargo check --package crypto --package ds --lib -Z build-std=core,alloc --target lib/x86_64-unknown-none-elf.json
cargo check --package crypto --package ds --lib -Z build-std=core,alloc --target lib/aarch64-unknown-none-elf.json
cargo clippy --package crypto --package ds --lib --tests -- -D warnings
```

**What the plan must add to xtask:**
1. `TestStep`: legs for `crypto` and `ds` (`--lib --tests`), a `crypto` leg with the two soft cfgs (F3), and a `--doc` leg for `crypto` if compile-fail doctests carry the typestate proof (Pitfall 8).
2. `CheckStep`: host legs for both crates, and cross legs. Use the two r9 spec files with `-Z build-std=core,alloc` as the proof; keep the two built-in targets as well if D-21 is to be met to the letter.
3. A new subcommand for Miri (F6).
4. `BuildStep`: no change is needed. The alternative is `--exclude crypto --exclude ds` so kernel builds compile exactly what they do today; that also works but gives up the free real-spec compile on every kernel build. Either way the kernels are byte-identical.

CI (`.github/workflows/xtask.yml`) runs `cargo xtask check`, `clippy` and `test` on `ubuntu-latest` through rustup, not nix. `rust-toolchain.toml` already lists `rust-src` and both built-in bare-metal targets, so the new check and test legs need no CI change. A Miri job there would need the `miri` component added in the job. [VERIFIED: workflow file lines 1-30 and `rust-toolchain.toml` read. ASSUMED: that rustup installs the listed targets on the runner as it does today]

### F6. Miri: how to supply it

- The devshell toolchain is `rust-bin.fromRustupToolchainFile ./rust-toolchain.toml` (`flake.nix:32-33`), and the devshell's `packages` are `[ toolchain pkgs.qemu pkgs.dtc ]` plus Linux-only tools (`flake.nix:1056`). [VERIFIED: file read]
- `(toolchainFor system).override { extensions = [ "rustfmt" "rust-src" "clippy" "llvm-tools" "miri" ]; }` evaluates and builds to a separate store path. Building it fetched only the `miri-preview` component. [VERIFIED: `nix build`]
- With that path first on `PATH`, `cargo miri setup` built its sysroot in 9 seconds and `cargo miri test --package ds --lib` ran inside the repo copy's workspace, including from a shell where `CARGO` pointed at the devshell cargo. [VERIFIED]
- The two toolchains cannot both be in the devshell's `packages`: each provides `cargo` and `rustc`. Export the path instead.

**Recommended shape:**
```nix
# flake.nix, devShells: a second derivation; rust-toolchain.toml is not edited
toolchainMiri = toolchain.override {
  extensions = [ "rustfmt" "rust-src" "clippy" "llvm-tools" "miri" ];
};
# shellHook:
if [ -z "''${R9_MIRI_TOOLCHAIN:-}" ]; then
  export R9_MIRI_TOOLCHAIN="${toolchainMiri}"
fi
```
`cargo xtask miri` (name is Claude's discretion) then runs `$R9_MIRI_TOOLCHAIN/bin/cargo miri test --package ds --lib tpht` with that `bin` directory prepended to `PATH` and `MIRIFLAGS=-Zmiri-many-seeds=0..N`, and fails with a named error when the variable is unset, the same way the Harvey resolver does. The kernel package derivations keep `toolchainFor`, so their inputs do not change.

`cargo miri setup` compiles `std` from `rust-src` and caches it under the user's cache directory. It worked here with network access; inside a nix sandbox it was not tried. [ASSUMED: a sandboxed flake check would need the sysroot dependencies vendored; keep Miri a devshell step]

### F7. Miri and the seqlock: what the probe showed

A probe crate defined a 64-byte header from atomics and three tests.

| Variant | Miri, 64 seeds | Native, 50 runs |
|---|---|---|
| Listing 9.3 orderings on atomic fields: data stores `Relaxed`, reader loads `seq` with `Acquire` before and after | **18 seeds fail** (torn read: quotient from one generation, tiny pointer from the previous) | 50 pass |
| Boehm ordering: data stores `Release`; reader loads data `Relaxed`, then `fence(Acquire)`, then reloads `seq` | 64 pass, about 10 s wall | — |
| Listing 9.3 literally: plain `u16` field written under the seqlock, read by a concurrent reader | `Undefined Behavior: Data race detected between (1) non-atomic write ... and (2) non-atomic read` | — |

[VERIFIED: `cargo miri test`, `MIRIFLAGS=-Zmiri-many-seeds=0..64`]

Consequences for the plan:
- Every header field a reader touches must be an atomic. The layout still sums to 64 bytes: `AtomicU32` + `AtomicU16` + `AtomicU16` + `[AtomicU16; 16]` + `[AtomicU8; 16]` + `AtomicU64` = 4 + 2 + 2 + 32 + 16 + 8. A `const` assertion on `size_of` and `align_of` compiled. [VERIFIED]
- Native stress tests on this machine did not catch the ordering bug once in 50 runs. D-13's Miri step is the only check that does; it is not optional.
- Under Miri, keep each test to a few hundred operations and scale loop counts with `cfg!(miri)`.

### F8. `roaring` as a host-only oracle

`roaring` as a `[dev-dependencies]` entry of a `no_std` crate builds and runs under `cargo test`; the crate root uses `#![cfg_attr(not(test), no_std)]` as `port/src/lib.rs:3` does (`#![cfg_attr(not(any(test)), no_std)]`). Dev-dependencies are not compiled by `cargo build` or by `cargo check --lib`, so the oracle never reaches a kernel or a cross-check leg. [VERIFIED: the kernel build log has no `roaring` line; the cross legs passed with the dev-dependency present]

The oracle also compiles and runs under Miri, but the Miri step should filter to `tpht` so Roaring's randomized tests do not run there. [VERIFIED: one oracle test ran under Miri in 0.17 s]

### F9. TPHT hit rate: what a host measurement needs, and a model of the answer

**What "home block" means in the design.** A header tracks up to 16 entries. A tiny pointer `p` in 1..=255 addresses arena slot `((b << 4) + (p - 1)) & slot_mask`: pointers 1..=16 are the block's own 16 slots, 17..=255 reach into the slots of the following blocks (`spilled_mask`). The arena has 16 slots per block. Entries that do not fit in the header go to the dereference banks. [VERIFIED: guide 5.2, compendium 4.2 and 4.3 read through the shell; quoted formula `slot = ((b << 4) + (ptr.get() - 1)) & slot_mask`]

**A workable definition for the D-15 measurement:**
- Occupancy = entries inserted / (16 × number of blocks).
- A lookup of a present key is a home-block hit when it resolves from the home header (one header line, one arena slot) without touching a bank.
- Spill threshold `T` = the number of header slots a block may use before further keys for that block go to the banks, `T` ≤ 16.
- Report, for each `T` and each occupancy: home-hit fraction for present keys, and the fraction of absent-key lookups that end at the header.
- Vary the key distribution. At least: uniform random 64-bit; sequential; `PageKey`-style `(image_id << 32) | page_offset`; keys that vary only in the upper 32 bits.

**Model (Python, a planning aid; not the D-15 evidence).** Home-hit fraction for random keys, by threshold and occupancy:

| T | 0.50 | 0.70 | 0.80 | 0.85 | 0.90 | 0.95 | 1.00 |
|---|------|------|------|------|------|------|------|
| 4 | 0.493 | 0.357 | 0.312 | 0.294 | 0.278 | 0.263 | 0.250 |
| 8 | 0.860 | 0.692 | 0.617 | 0.583 | 0.553 | 0.525 | 0.499 |
| 12 | 0.984 | 0.913 | 0.856 | 0.826 | 0.794 | 0.763 | 0.732 |
| 14 | 0.996 | 0.964 | 0.928 | 0.906 | 0.881 | 0.855 | 0.828 |
| 15 | 0.998 | 0.978 | 0.952 | 0.934 | 0.914 | 0.892 | 0.867 |
| 16 | 0.999 | 0.987 | 0.969 | 0.956 | 0.940 | 0.922 | 0.901 |

[VERIFIED: `E[min(X, T)] / λ` for `X ~ Poisson(16 × occupancy)`, and a simulation with 2^14 blocks that agrees to three places]

Expect the measurement to choose `T = 16`: it is the only threshold over 90% at 0.95 occupancy, and `T = 14` or less fails at 0.90. The model also confirms D-14: a 4-slot capacity gives 28% at 0.90.

**Index bits (the C4 finding).** Simulated with `T = 16`, 2^14 blocks, at 0.90 occupancy:

| Key distribution | Index = low bits of P (as written) | Index = low bits of the mixed half L' |
|---|---|---|
| random u64 | 0.940 | 0.941 |
| sequential 0..n | 1.000 | 0.940 |
| `PageKey`, 64 images × sequential pages | **0.250** | 0.940 |
| `id << 32` | **0.000** | 1.000 |
| `(id << 32) \| 7` | **0.000** | 1.000 |

[VERIFIED: simulation; `f` taken as MurmurHash3's 32-bit finalizer, see Open Question 2]

The permutation stays bijective whichever bits are used as the index, so DS-02 is unaffected. See Open Question 1.

### F10. Lint and audit contract as it stands

- Workspace `Cargo.toml:3`: `members = ["aarch64", "x86_64", "port", "riscv64", "xtask"]`; lines 11-12: `[workspace.lints.clippy]` / `undocumented_unsafe_blocks = "deny"`; the comment on lines 5-10 says `Only the gated crates (aarch64 + port) opt in` and must be updated. [VERIFIED: file read]
- Opt-in is `[lints]` / `workspace = true` (`port/Cargo.toml:44-45`). [VERIFIED: file read]
- A crate with `[lints] workspace = true` cannot also carry its own `[lints.rust]` table; the workspace `Cargo.toml` comment says so (`cargo forbids a crate-level [lints.rust] override alongside [lints] workspace = true`). If `crypto/` ever tests a custom `cfg` in its own code, the name goes in the workspace `check-cfg` list. [VERIFIED: comment read through the shell; not tried]
- `UNSAFE-AUDIT.md`: counting rule at lines 15-21 (`a site is one unsafe {…} block, unsafe fn, unsafe impl, or unsafe extern token`), scope line at 23-26 (`elimination covers the aarch64 + port crates`), reconciliation table at 30-37 (aarch64 129, port 23, SAFETY annotations 152, port test-only blocks 10, unsafe fns 16, unsafe extern blocks 6), port module-state table at 78-84. [VERIFIED: file read]
- The lint sees `unsafe {}` blocks and `unsafe impl`; it does not check `unsafe fn` declarations, and the default clippy run does not lint test code (`--workspace` without `--tests`). The audit already records both classes separately. [VERIFIED: audit lines 17-21, 35-37; ClippyStep read]
- A `#[forbid(unsafe_code)]` on a module reaches its submodules, which is why `port` mounts `devip/waker.rs` at the crate root with `#[path]` (`port/src/lib.rs`, audit lines 44-48). For `ds/` this means `tpht` carries no forbid, and any safe submodule inside it (for example the Feistel functions) can carry its own. [VERIFIED: audit text read; `port/src/lib.rs` read through the shell]

Upstream `unsafe` token counts in the pinned sources, for the audit's third-party record (D-10, discretion item): `blake3` 238, `chacha20` 66, `arrayvec` 58, `constant_time_eq` 54, `hybrid-array` 43, `sha2` 39, `cmov` 28, `block-buffer` 22, `zeroize` 21, `cpufeatures` 11, `ctutils` 1; `hmac`, `hkdf`, `digest`, `crypto-common`, `typenum`, `cfg-if` 0. These are raw `grep` counts of the word in `src/`, including code for backends r9 never selects. [VERIFIED: grep over the vendored sources]

## Architecture Patterns

### System Architecture Diagram

```
                       host (cargo xtask test)                      kernel specs (cargo xtask check)
                       ───────────────────────                      ────────────────────────────────
 RFC 5869 / 4231 ─┐
 RFC 8439 2.3.2,  ├─► crypto/tests/*  ──►  crypto/ public API ──►  sha2 / hmac / hkdf / chacha20 / blake3 / zeroize
 2.4.2            │   (default leg and     kdf.rs  page.rs          │
 blake3 JSON ─────┘    soft-cfg leg)       mac.rs  key.rs           └─► lib/{x86_64,aarch64}-unknown-none-elf.json
                                                                        -Z build-std=core,alloc   (compile only)

 seeded PRNG ─► ds tests ─► ds/ public API
                 │           ├─ tpht/ (unsafe allowed) ── flattened.rs ── chained.rs ── feistel
                 │           ├─ roaring.rs (forbid)  ◄── compared with ──► BTreeSet model
                 │           │                        ◄── compared with ──► roaring crate (dev-dep)
                 │           └─ clientset.rs (forbid)
                 │
                 └─► cargo xtask miri ─► $R9_MIRI_TOOLCHAIN/bin/cargo miri test -p ds --lib tpht
                                          (separate toolchain derivation; many seeds)

 cargo xtask clippy ─► --workspace ─► undocumented_unsafe_blocks = deny on crypto, ds, port
 UNSAFE-AUDIT.md ◄── ds/ section (each tpht site) + crypto/ line (zero sites) + dependency record

 kernels (port, aarch64, x86_64): no edge to crypto/ or ds/ in this phase; sha256 before == after
```

### Recommended Project Structure

```
crypto/
├── Cargo.toml            # pins + provenance comment (D-04), [lints] workspace = true
├── src/
│   ├── lib.rs            # #![cfg_attr(not(test), no_std)]  #![forbid(unsafe_code)]
│   ├── kdf.rs            # HMAC-SHA256, HKDF extract/expand, K_file, labelled expand
│   ├── page.rs           # ChaCha20 (RFC 8439 counter and nonce)
│   ├── mac.rs            # keyed BLAKE3: 32-byte tag, truncated tag, verify
│   └── key.rs            # KeyCapability typestate, Zeroizing key newtypes
└── tests/
    ├── rfc5869.rs  rfc4231.rs  rfc8439.rs  blake3_vectors.rs  keys.rs
    └── vectors/blake3_test_vectors.json      # byte-for-byte upstream

ds/
├── Cargo.toml            # [dev-dependencies] roaring; [lints] workspace = true
├── src/
│   ├── lib.rs            # #![cfg_attr(not(test), no_std)]  extern crate alloc; per-module forbids
│   ├── tpht/             # the one unsafe-holding module this phase (no forbid on it)
│   │   ├── mod.rs
│   │   ├── feistel.rs    # #![forbid(unsafe_code)] inside is allowed
│   │   ├── flattened.rs
│   │   └── chained.rs
│   ├── roaring.rs        # #[forbid(unsafe_code)]; container ops behind one seam (D-20)
│   └── clientset.rs      # #[forbid(unsafe_code)]
└── tests/                # or src/*_tests.rs, as port does
```

`crypto/src/kdf.rs`, `crypto/src/page.rs`, `ds/src/tpht/flattened.rs`, `ds/src/tpht/chained.rs` and `ds/src/roaring.rs` are named by the requirements. The other file names are suggestions (Claude's discretion).

### Pattern 1: Thin wrapper, upstream types never public

**What:** Every `crypto/` function takes and returns byte arrays, slices or r9 types. No upstream type appears in a public signature (D-18).
**Example (compiled and vector-tested in the probe):**
```rust
// Source: probe crate built this session against the pinned versions
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};

pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut m = <Hmac<Sha256> as KeyInit>::new_from_slice(key).expect("any key len");
    m.update(msg);
    m.finalize().into_bytes().into()
}

pub fn hkdf_expand(prk: &[u8], info: &[u8], okm: &mut [u8]) -> bool {
    match hkdf::Hkdf::<Sha256>::from_prk(prk) {
        Ok(h) => h.expand(info, okm).is_ok(),
        Err(_) => false,
    }
}

/// RFC 8439: 32-bit block counter, 96-bit nonce.
pub fn chacha20_xor(key: &[u8; 32], nonce: &[u8; 12], counter: u32, buf: &mut [u8]) {
    let mut c = chacha20::ChaCha20::new(key.into(), nonce.into());
    c.seek(counter as u64 * 64);
    c.apply_keystream(buf);
}

pub fn blake3_keyed(key: &[u8; 32], msg: &[u8]) -> [u8; 32] {
    *blake3::keyed_hash(key, msg).as_bytes()
}
```
In the real crate use `try_seek` and `try_apply_keystream` and return an error: `seek` and `apply_keystream` panic when the 32-bit counter would wrap (256 GiB of keystream). [VERIFIED: both method pairs exist in `cipher-0.5.2/src/stream.rs`. ASSUMED: the panic-on-overflow wording, from the trait's naming convention]

### Pattern 2: Header of atomics with the Boehm seqlock ordering

**What:** Readers never take a lock. Writers make `seq` odd, store data with `Release`, make `seq` even. Readers load `seq`, load data `Relaxed`, issue `fence(Acquire)`, reload `seq`, and retry on change or odd.
**Example (the variant that passed 64 Miri seeds):**
```rust
// Source: Miri probe run this session
use core::sync::atomic::{fence, AtomicU16, AtomicU32, AtomicU64, AtomicU8, Ordering};

#[repr(C, align(64))]
pub struct TphtBucketHeader {
    seq_lock: AtomicU32,
    occupied_mask: AtomicU16,
    spilled_mask: AtomicU16,
    quotients: [AtomicU16; 16],
    tiny_ptrs: [AtomicU8; 16],
    clock_nibbles: AtomicU64,
}
const _: () = assert!(core::mem::size_of::<TphtBucketHeader>() == 64);

// writer
let s = h.seq_lock.load(Ordering::Relaxed);
h.seq_lock.compare_exchange(s, s + 1, Ordering::Acquire, Ordering::Relaxed).unwrap();
h.quotients[i].store(q, Ordering::Release);
h.tiny_ptrs[i].store(p, Ordering::Release);
h.seq_lock.store(s + 2, Ordering::Release);

// reader
let s1 = h.seq_lock.load(Ordering::Acquire);
let q = h.quotients[i].load(Ordering::Relaxed);
let p = h.tiny_ptrs[i].load(Ordering::Relaxed);
fence(Ordering::Acquire);
let stable = s1 & 1 == 0 && h.seq_lock.load(Ordering::Relaxed) == s1;
```
The field names are the guide's (section 5.2). [CITED: Boehm, "Can Seqlocks Get Along With Programming Language Memory Models?", MSPC 2012, for the ordering; VERIFIED under Miri here]

### Pattern 3: Roaring container ops behind one seam (D-20)

**What:** One private module holds the four hot loops as plain functions over slices (`bitmap_and`, `bitmap_or`, `bitmap_andnot` over `&[u64; 1024]`, `array_intersect` over `&[u16]`). Containers call only those. Phase 5 swaps the function bodies; no caller changes. No `unsafe` in this phase.

### Pattern 4: Counting probe for "exactly one memory access" (DS-03)

**What:** Route every arena and header read through a small trait with a zero-sized no-op implementation in production and a counting implementation in tests. The test asserts, after one `deref_tiny`, `arena_reads == 1 && header_reads == 0`. Define "one access" as one arena slot (a value may be several words inside that slot). Compare `deref_tiny` against a `BTreeMap` model over a randomized operation sequence so the test is model-based as DS-03 requires.

### Anti-Patterns to Avoid

- **Copying listing 9.3 or 9.4.** 9.3 reads plain `u16` fields under the seqlock (a data race) and uses orderings that fail under Miri. 9.4 writes through `&node.entry as *const PackedEntry as *mut PackedEntry`, which is undefined behaviour for a shared reference without interior mutability, and its `unpack_tiny_ptr` gives 7 slot bits (127) against a 255-node bank. D-03 already forbids copying them. [VERIFIED: listings read]
- **Adding `miri` to `rust-toolchain.toml`.** Changes kernel bytes (C3).
- **Putting the soft cfgs in `.cargo/config.toml`.** They would reach the kernel builds (F3).
- **Naming an upstream crate in a `crypto/` public signature.** Breaks D-18.
- **`#[derive(Debug)]` on key types.** Prints key bytes.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SHA-256, HMAC, HKDF | own compression function or HMAC padding | `sha2`, `hmac`, `hkdf` | D-18; the guide's ledger records that the first sketch had an XOR fold here |
| ChaCha20 | own quarter-round code | `chacha20` | D-18; builds portable, so the fallback does not apply |
| BLAKE3 | own tree hashing | `blake3` | D-18; chunk and parent chaining is easy to get subtly wrong |
| Constant-time 32-byte tag compare | byte loop with early exit | `blake3::Hash` equality | Already constant-time; no new crate |
| Zeroize on drop | manual `Drop` with a write loop | `zeroize::Zeroizing` | The compiler may remove a plain write loop |
| JSON parsing for the vectors | a JSON crate | `include_str!` + a fixed-shape extractor | One file, fixed shape, no new dependency |
| Property-test framework | `proptest`, `rand` | seeded SplitMix64 in test code | No dependency; reproducible seeds; quick under Miri |

Hand-written by decision, not by default: Roaring, TPHT, `ClientSet` (D-19).

**Key insight:** the risk in this phase is not the primitives, which upstream gets right, but the build configuration around them and the memory-ordering details of the hand-written table. Both are only checkable by running the tools, which is why the Miri step and the real-spec cross legs carry the weight.

## Common Pitfalls

### Pitfall 1: Treating the built-in aarch64 target as FP-free
**What goes wrong:** The cross-check passes but would also pass for NEON code.
**Why:** `aarch64-unknown-none` is a hard-float target (590 FP/SIMD instructions in the probe).
**How to avoid:** Check against `lib/aarch64-unknown-none-elf.json` with `-Z build-std=core,alloc`.
**Warning sign:** A cross leg with no `-Z build-std`.

### Pitfall 2: Changing kernel bytes through the toolchain
**What goes wrong:** `rust-toolchain.toml` gains `miri`; both kernels change in over a thousand places.
**How to avoid:** Second toolchain derivation, `R9_MIRI_TOOLCHAIN`.
**Warning sign:** Any diff to `rust-toolchain.toml`.

### Pitfall 3: Kernel bytes drifting through feature unification
**What goes wrong:** Kernel builds use `--workspace`. A future dependency of `crypto/` or `ds/` that `port` also uses (for example `byteorder`, which `smoltcp` pulls) could switch on a feature for the kernel's copy.
**How to avoid:** Re-run the before/after kernel sha256 comparison after every `Cargo.toml` dependency edit in this phase, not only once. With the pins above there is no drift (F5).
**Warning sign:** An existing `Cargo.lock` entry changes, or a `Compiling` line for a `port` dependency appears with a new feature set. [ASSUMED mechanism; the current pins are VERIFIED byte-neutral]

### Pitfall 4: Seqlock orderings that pass every native test
**What goes wrong:** Listing 9.3's orderings tear under the language memory model.
**How to avoid:** Pattern 2; Miri with many seeds on every `tpht` change.
**Warning sign:** A reader that reloads `seq` with no `fence(Acquire)` before it, or data stored `Relaxed`.

### Pitfall 5: Host tests pinning a backend the kernel never runs
**What goes wrong:** Vectors pass on NEON or AVX2 code; the scalar path is untested.
**How to avoid:** The soft-cfg `crypto` test leg (F3).

### Pitfall 6: Undefined behaviour copied from listing 9.4
**What goes wrong:** Writing through a pointer cast from `&T`.
**How to avoid:** Make every field written after publication an atomic, or wrap the slot in `UnsafeCell` with a `// SAFETY:` argument tied to the `WRITING` claim. Miri reports the cast version.

### Pitfall 7: Structured keys collapsing into one block
**What goes wrong:** Over-90% home hits on random keys, near zero on `PageKey` or `id << 32` keys.
**How to avoid:** Index from the mixed half (Open Question 1); include structured distributions in the D-15 evidence.

### Pitfall 8: Claiming a cryptographic read-only guarantee
**What goes wrong:** A test or comment says `ReadOnly` "cannot" forge a tag. With a symmetric MAC it can if it can read the key.
**How to avoid:** State the guarantee as type-level: `ReadOnly` has `verify` and no `seal`; the key field is private; no accessor returns the bytes. Prove it with a `compile_fail` doctest (needs a `--doc` test leg, since `--lib --tests` does not run doctests) or with a test of the public surface. Record the limit in the crate docs.

### Pitfall 9: A third unsafe site through test scaffolding
**What goes wrong:** Proving "no heap for up to four clients" with a counting `#[global_allocator]` needs `unsafe impl GlobalAlloc`, outside `ds/src/tpht/`. D-09 allows exactly two unsafe-holding places.
**How to avoid:** Assert the variant (`Inline` after four inserts, `Roaring` after the fifth) and `size_of::<ClientSet>() <= 24` (the RFC's "Occupies 24 bytes inline", section 7.2). The `Inline` variant holds no pointer, so no allocation is possible in it. If the owner wants an allocator-level proof, that is an approval request.

### Pitfall 10: Vectors transcribed from memory
**What goes wrong:** A wrong hex digit makes a RED pin that can never go green, or a matching wrong implementation.
**How to avoid:** Copy every RFC vector from the RFC text (rfc-editor.org) at execution time and cite section and test-case number beside it. Only RFC 5869 TC1, RFC 4231 TC1 and TC2, and RFC 8439 2.3.2 and 2.4.2 were exercised in this research; the remaining cases (RFC 5869 TC2 and TC3; RFC 4231 TC3 to TC7, where TC5 is a 128-bit truncation) were not.

### Pitfall 11: The aarch64 clippy leg is already red
**What goes wrong:** Phase verification runs `cargo xtask clippy --arch aarch64 --config virtshell`, sees failures, and blames this phase.
**How to avoid:** Name the clippy command in the plan (`cargo xtask clippy`, the CI one). Record the seven pre-existing sites as out of scope, or raise them with the owner.

## Code Examples

### Soft-backend test leg (verified command form)
```bash
# Source: run this session; 6 vector tests passed, test binary links sha256::soft and no NEON backend
cargo test --package crypto --lib --tests \
  --config 'build.rustflags=["--cfg","chacha20_backend=\"soft\"","--cfg","sha2_backend=\"soft\""]'
```

### Real-spec cross leg (verified command form)
```bash
# Source: run this session against a repo copy; rc 0 for both specs
cargo check --package crypto --package ds --lib \
  -Z build-std=core,alloc --target lib/aarch64-unknown-none-elf.json
```

### Miri step (verified command form)
```bash
# Source: run this session
PATH="$R9_MIRI_TOOLCHAIN/bin:$PATH" MIRIFLAGS="-Zmiri-many-seeds=0..64" \
  "$R9_MIRI_TOOLCHAIN/bin/cargo" miri test --package ds --lib tpht
```

### BLAKE3 vector file, fixed-shape read (verified)
```rust
// Source: probe test, 35 cases passed for hash and keyed_hash
fn field<'a>(obj: &'a str, name: &str) -> &'a str {
    let k = format!("\"{name}\": ");
    let i = obj.find(&k).unwrap() + k.len();
    let rest = obj[i..].trim_start_matches('"');
    let end = rest.find(|c| c == '"' || c == ',' || c == '\n').unwrap();
    &rest[..end]
}
// let j = include_str!("vectors/blake3_test_vectors.json");
// key = field(j, "key"); each case: split on '{' after "cases"; input byte i = i % 251;
// expected 32-byte output = first 64 hex characters of "hash" / "keyed_hash".
```

### Kernel byte-neutrality check
```bash
# Source: procedure run this session on a copy
nix develop -c cargo xtask build --arch aarch64 --config virtshell
nix develop -c cargo xtask build --arch x86-64 --config q35churnshell
shasum -a 256 target/aarch64-unknown-none-elf/debug/aarch64 target/x86_64-unknown-none-elf/debug/x86_64
# record, make the change, move target/ aside, rebuild, compare
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `sha2 0.10` Cargo feature `force-soft` | `sha2 0.11` `--cfg sha2_backend="soft"` | 0.11.0, 2026-03 | Forcing scalar code is a rustflag, not a feature |
| `chacha20 0.9` `--cfg chacha20_force_soft` | `chacha20 0.10` `--cfg chacha20_backend="soft"` | 0.10, 2026 | New cfg name |
| `blake3` runtime detection only with `std` | `blake3 1.8.7` uses `cpufeatures` in `no_std` too | by 1.8.7 (upstream `Cargo.toml` comment) | On hosted `no_std` targets SIMD can now be selected at run time; `cpufeatures` still returns false on `os = "none"` |
| `roaring` optional `simd` feature | `roaring 0.12.0` depends on `fearless_simd` unconditionally | 0.12.0, 2026-10-28 | Reason to pin 0.11.5 for an oracle |

[VERIFIED: crates.io feature lists per version; vendored sources]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A global `build.rustflags` entry would change kernel bytes | F3 | Low: the recommendation avoids it either way |
| A2 | A sandboxed nix build could not run `cargo miri setup` without vendoring | F6 | Low: Miri is planned as a devshell step |
| A3 | `seek`/`apply_keystream` panic on counter overflow | Pattern 1 | Low: use the `try_` forms regardless |
| A4 | Feature unification is the mechanism by which a later dependency could move kernel bytes | Pitfall 3 | Low: the check catches drift whatever the cause |
| A5 | The read-only guarantee can only be type-level | C6, Pitfall 8 | Medium: if the owner intends a cryptographic guarantee, `blake3_verify` must be something other than the MAC key, which is a design change |
| A6 | `f(R)` completes to MurmurHash3's 32-bit finalizer | F9, Open Question 2 | Low for correctness (any `f` is bijective); changes measured hit rates slightly |
| A7 | `Qid.path` and `Qid.vers` are encoded little-endian in the HKDF `info` | Open Question 4 | Medium: the encoding becomes part of every derived file key |
| A8 | The nix-sandboxed kernel derivations (`kernel-x64`, `kernel-virt`) build with the enlarged `Cargo.lock` | F5 | Medium: they vendor from `Cargo.lock` with `importCargoLock` and could not be built on this Mac. Needs one Linux or CI build |
| A9 | rustup on the CI runner installs the targets and `rust-src` that `rust-toolchain.toml` lists | F5 | Low |
| A10 | RFC test vectors may be reproduced in the repo | F4 | Low |

## Open Questions

1. **Which bits of the Feistel output are the home block index?**
   - Known: the compendium says the lower bits of `P = (L' << 32) | R`; those are the key's own low bits. Structured keys collapse (C4, F9).
   - Unclear: whether the owner regards the compendium wording as binding. It is not part of the IG-D09 lock as D-17 states it.
   - Recommendation: index = low bits of `L'` (the mixed half); remainder = `R` and the remaining bits of `L'`. Record it as a correction to compendium 4.1 in PROJECT.md beside the D-08 and D-17 rows, and show both columns in the D-15 evidence. Surface it to the owner at phase verification with that evidence.

2. **What is the rest of `f(R)`?**
   - Known: both sources give `f(R) = (R xor (R >> 16)) * 0x85EBCA6B xor ...` and stop.
   - Recommendation: MurmurHash3's 32-bit finalizer, whose first constant is `0x85EBCA6B` (`h ^= h >> 16; h *= 0x85EBCA6B; h ^= h >> 13; h *= 0xC2B2AE35; h ^= h >> 16`). Pin it with a known-answer test.

3. **How does DS-02's "reconstruct the key from index and remainder" square with a 16-bit `quotients` field?**
   - Known: a 64-bit key minus `log2(blocks)` index bits leaves far more than 16 bits. The header's 16-bit value can only be a fingerprint; the sketch verifies candidates with a caller closure.
   - Recommendation: test DS-02 at the function level (index and full-width remainder round-trip to the key) and treat the header field as a fingerprint of that remainder. Say so in the module docs.

4. **Byte encoding of `Qid.path || Qid.vers` in `K_file`'s `info`.**
   - Recommendation: 8 bytes little-endian then 4 bytes little-endian, matching 9P's wire order; pin with a known-answer vector computed once from the RFC-verified HKDF.

5. **How far do "transient and session keys" go in this phase?**
   - Known: the RFC's formula is `HKDF-Expand(S_session, "9P-AEAD-FID" || fid, 32)`, but whether keys are per FID or per connection and direction is W4, settled in Phase 4.
   - Recommendation: provide a labelled expand (`label`, `context`, 32-byte output) and the `K_file` binding; do not fix the session `info` layout here.

6. **Where does a home block keep its overflow chain head?**
   - Known: the 64-byte header has no spare byte; listing 9.5 reads a `block.overflow_chain` field that the header in 9.3 does not have.
   - Recommendation: a parallel array of atomic chain heads indexed by block, touched only on the spill path. This keeps the header at the locked layout.

7. **What does "over 95% fill" in DS-04 measure?**
   - Recommendation: nodes in use / (255 × banks) at the first insert that finds both candidate banks full, over several seeds; "allocation failure" means that event, not a heap failure.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| nix devshell (`nix develop`) | all commands | yes | — | — |
| rustc / cargo | everything | yes | 1.95.0-nightly (842bd5be2 2026-01-29) / cargo 1.95.0-nightly (efcd9f586 2026-01-23) | — |
| `rust-src` | `-Z build-std` cross legs, Miri sysroot | yes | in the toolchain | — |
| built-in targets `x86_64-unknown-none`, `aarch64-unknown-none` | D-21 legs | yes | in the toolchain | — |
| `llvm-tools` (`llvm-objdump`, `llvm-size`) | optional disassembly evidence | yes | in the toolchain | — |
| clippy | LINT-01 | yes | in the toolchain | — |
| Miri | D-13 | **no** in the devshell | available as `miri-preview-1.95.0-nightly-2026-01-30` through rust-overlay | add `toolchainMiri` + `R9_MIRI_TOOLCHAIN` (F6) |
| crates.io network access | first resolve of the new pins | yes (this machine) | — | — |
| Linux host or CI | one nix-sandboxed kernel build with the new lock (A8) | not on this Mac | — | owner's Linux host or hosted CI |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** Miri (supply through the flake as in F6).

## Validation Architecture

`workflow.nyquist_validation` is absent from `.planning/config.json`, so validation is enabled. [VERIFIED: file read through the shell]

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in tests through `cargo xtask`; Miri (`cargo miri test`) for `ds/src/tpht/` |
| Config file | none; workspace `Cargo.toml`, `.planning/config.json` (`build_command`: `nix develop -c cargo xtask check`, `test_command`: `nix develop -c cargo xtask test`) |
| Quick run command | `nix develop -c cargo xtask test` |
| Full suite command | `nix develop -c cargo xtask test && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy && nix develop -c cargo xtask miri` |
| Estimated runtime | quick: seconds once built (the probe's vector tests ran in under 0.01 s; a 64-seed Miri run of one seqlock test took about 10 s) |

### Phase Requirements → Test Map

Test file and function names are proposals; none exists yet.

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CRYP-01 | HMAC-SHA256 matches RFC 4231 TC1 to TC7 | unit (vectors) | `cargo test -p crypto --test rfc4231` (inside `cargo xtask test`) | no, Wave 0 |
| CRYP-01 | HKDF-SHA256 matches RFC 5869 TC1 to TC3; output over 8160 bytes rejected | unit (vectors) | `cargo test -p crypto --test rfc5869` | no, Wave 0 |
| CRYP-01 | Same vectors on the scalar backend | unit (vectors) | the soft-cfg `crypto` leg of `cargo xtask test` | no, Wave 0 (xtask leg) |
| CRYP-02 | ChaCha20 block function, RFC 8439 2.3.2 | unit (vector) | `cargo test -p crypto --test rfc8439` | no, Wave 0 |
| CRYP-02 | ChaCha20 encryption, RFC 8439 2.4.2; decrypt round trip; counter-overflow returns an error | unit | same | no, Wave 0 |
| CRYP-03 | BLAKE3 `hash` and `keyed_hash`, all 35 official cases, 32-byte output, one-shot and incremental | unit (vectors) | `cargo test -p crypto --test blake3_vectors` | no, Wave 0 |
| CRYP-03 | Truncated keyed tag (D-07): verify accepts the right tag, rejects one flipped bit | unit | same | no, Wave 0 |
| CRYP-04 | `K_file` known answer; different `Qid.path` or `Qid.vers` gives a different key | unit | `cargo test -p crypto --test keys` | no, Wave 0 |
| CRYP-04 | `ReadWrite` seals and verifies; `ReadOnly` verifies; `ReadOnly` has no seal | unit + `compile_fail` doctest | `cargo test -p crypto --test keys`; `cargo test -p crypto --doc` | no, Wave 0 |
| CRYP-04 | Read-only sibling keeps the parent's `k_dec`; key types are `Zeroizing` | unit | `cargo test -p crypto --test keys` | no, Wave 0 |
| DS-01 | `size_of::<TphtBucketHeader>() == 64`, `align_of == 64` | unit (const assert + test) | `cargo test -p ds tpht::` | no, Wave 0 |
| DS-01 | Home-hit fraction over 0.90 at the target occupancy with the chosen threshold constant, for each key distribution | unit (measurement pin) | `cargo test -p ds tpht::hit_rate` | no, Wave 0 |
| DS-01 | Seqlock readers never see a torn entry | Miri, many seeds; native multi-thread stress | `cargo xtask miri`; `cargo test -p ds tpht::stress` | no, Wave 0 |
| DS-02 | Feistel round trip for random, sequential and boundary keys; known answer for `f` | unit | `cargo test -p ds tpht::feistel` | no, Wave 0 |
| DS-03 | `deref_tiny`: one arena read, zero header reads; agrees with a `BTreeMap` model | unit (model-based, counting probe) | `cargo test -p ds tpht::deref_tiny` | no, Wave 0 |
| DS-04 | Banks reach over 95% fill before the first failed claim; full-key verification on a fingerprint match | unit | `cargo test -p ds tpht::chained` | no, Wave 0 |
| DS-04 | `WRITING` (0xFF) publication: a reader never sees a partial node | Miri, many seeds; native stress | `cargo xtask miri` | no, Wave 0 |
| DS-05 | Union, intersection, difference, `andnot` equal the `BTreeSet` model and the `roaring` crate over seeded random inputs covering array, bitmap and run containers and their conversions | unit (differential) | `cargo test -p ds roaring::` | no, Wave 0 |
| DS-06 | `Empty` → `Single` → `Inline` up to four, `Roaring` at five; `size_of::<ClientSet>() <= 24`; ids 100..=4096 give one run container | unit | `cargo test -p ds clientset::` | no, Wave 0 |
| DS-07 | No Funnel module exists in `ds/`; the crate docs carry the restriction | static check | `test ! -e ds/src/funnel.rs && grep -q -i funnel ds/src/lib.rs` | no, Wave 0 |
| LINT-01 | Both crates opt in; clippy rejects an undocumented unsafe block | lint | `nix develop -c cargo xtask clippy` | xtask exists; manifests do not |
| LINT-01 | Both crates compile for both kernel specs | cross compile | `nix develop -c cargo xtask check` (new legs) | no, Wave 0 (xtask legs) |
| LINT-01 | `crypto/` has no `unsafe`; `ds/` has it only under `src/tpht/` | static check | `! grep -rnw unsafe crypto/src; ! grep -rnw unsafe ds/src --exclude-dir=tpht` | no, Wave 0 |
| LINT-01 | `UNSAFE-AUDIT.md` lists every `tpht` site | manual count against `grep -rnw unsafe ds/src/tpht`, as the audit does today | — | manual |
| D-21 | Kernel sha256 unchanged for aarch64 `virtshell` and x86_64 `q35churnshell` | evidence script | see Code Examples | manual step with recorded hashes |

### Sampling Rate

- **Per task commit:** `nix develop -c cargo xtask test`
- **Per wave merge:** `nix develop -c cargo xtask test`, `check`, `clippy`, and `cargo xtask miri` when `ds/src/tpht/` changed
- **Phase gate:** all four green twice (IG-V01), the kernel sha256 comparison recorded, before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crypto/Cargo.toml`, `crypto/src/lib.rs`, `ds/Cargo.toml`, `ds/src/lib.rs`; workspace `members`
- [ ] xtask: `crypto` and `ds` test legs, the soft-cfg `crypto` leg, a `--doc` leg if doctests are used, host and cross check legs, the Miri subcommand
- [ ] `flake.nix`: `toolchainMiri` and the `R9_MIRI_TOOLCHAIN` export
- [ ] `crypto/tests/vectors/blake3_test_vectors.json` (sha256 `dcb91ea8...f624`)
- [ ] RED pins for every row above (D-02), each committed before its implementation
- [ ] Baseline kernel sha256 values recorded in the repo before the first workspace change

## Security Domain

`security_enforcement` is absent from `.planning/config.json`, so it is enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No authentication flow in this phase (`Tauth` is Phase 4) |
| V3 Session Management | no | Session keys are derived in Phase 4 |
| V4 Access Control | partly | `KeyCapability` typestate: `ReadOnly` has no sealing method (type-level; Pitfall 8) |
| V5 Input Validation | yes | Fixed-size array parameters for keys, nonces and tags; HKDF output length and ChaCha20 counter overflow return errors |
| V6 Cryptography | yes | Vetted upstream implementations only (D-18); published vectors as pins; constant-time tag comparison; `Zeroizing` key storage; no hand-rolled primitive |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Tag comparison that leaks timing | Information disclosure | `blake3::Hash` equality (constant-time) for 32-byte tags; a constant-time compare for truncated tags, never `==` on slices |
| Key bytes left in memory or printed | Information disclosure | `Zeroizing<[u8; 32]>`; no `Debug` on key types; no accessor returning key bytes from `ReadOnly` |
| Nonce or counter reuse under one key | Tampering, information disclosure | Out of this phase's control (Phase 4 and 5 own nonce rules); the API takes the nonce and counter explicitly and documents the caller's duty |
| MAC key reused as cipher key | Tampering | Separate key fields (`k_dec`, `k_mac`); the guide's ledger item 5 requires dedicated MAC keys |
| Compromised or typosquatted dependency | Tampering | Exact `=` pins, committed `Cargo.lock`, provenance comments (D-04), legitimacy audit above |
| Vector code running in a kernel without FP state saved | Denial of service, information disclosure (corrupts user FP state) | Real-spec cross legs; F2's measurement; `blake3` `no_*` features |
| Data race in a lock-free table | Tampering | Atomics for all reader-visible fields; Miri with many seeds |
| Hash-flooding of the home blocks by chosen keys | Denial of service | A per-table seed in the Feistel mixing function is worth considering; not required by any Phase 3 requirement [ASSUMED] |

## Sources

### Primary (HIGH confidence)
- Scratch builds and runs with the repo's pinned toolchain this session: vector tests, eight linked cross-target probes with disassembly scans, kernel before/after sha256, xtask commands against a repo copy, Miri probes.
- Vendored upstream sources under `~/.cargo/registry/src/`: `cpufeatures-0.3.1`, `sha2-0.11.1`, `chacha20-0.10.2`, `blake3-1.8.7`, `hkdf-0.13.0`, `cipher-0.5.2`, `digest-0.11.3`, `constant_time_eq-0.4.2`, `zeroize-1.9.2`.
- Repo files opened with Read: `xtask/src/main.rs` (128-172, 320-409, 3470-3789, 4789-4806), `xtask/src/config.rs` (736-747), `UNSAFE-AUDIT.md`, `flake.nix` (1005-1116), `Cargo.toml` (1-12), `rust-toolchain.toml`, both `lib/*-unknown-none-elf.json`, `port/Cargo.toml` (40-46), `03-CONTEXT.md`, `REQUIREMENTS.md`, `STATE.md`.
- crates.io API: versions, publish dates, yanked flags, `rust_version`, feature lists, dependency lists.
- `github.com/BLAKE3-team/BLAKE3`: `test_vectors/test_vectors.json` at tags 1.0.0 and 1.8.7 and on master; `Cargo.toml` at 1.8.7.
- `nix eval` and `nix build` against the repo's locked `rust-overlay` and `nixpkgs`.

### Secondary (MEDIUM confidence)
- Source documents read through the shell: guide sections 2 to 6, 8, 11; compendium sections 4.1 to 4.6 and listings 9.3 to 9.6; RFC sections 3.1, 3.2, 5, 7.2; `intel/constraints.md` entries IG-M05 to IG-M08, IG-D07, IG-D09, IG-A01, IG-V01.
- Upstream READMEs (RustCrypto hashes, MACs, KDFs, stream-ciphers, utils; roaring-rs; BLAKE3) for crate names.
- The Python hit-rate model (`scripts/tpht_model.py` in scratch).

### Tertiary (LOW confidence)
- Boehm's seqlock paper, cited from memory for the ordering recipe; the recipe itself was verified under Miri.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. Every pin was resolved, built for four targets and tested against vectors.
- xtask, flake, lint and audit wiring: HIGH. Read and exercised against a copy with both crates added.
- Miri supply and seqlock ordering: HIGH. Built and run.
- TPHT design guidance (threshold, index bits): MEDIUM. A model in Python; the Rust measurement is D-15's job.
- Nix-sandboxed kernel build with the new lock: LOW. Not buildable on this Mac (A8).

**Research date:** 2026-11-24
**Valid until:** 2026-12-24 for the wiring and toolchain findings (the toolchain is pinned). Re-check crates.io for yanks of the six pins on the day they are committed.
