---
phase: 03-guide-phase-1-crypto-and-ds-foundations
reviewed: 2026-11-24T18:52:36Z
depth: standard
files_reviewed: 30
files_reviewed_list:
  - Cargo.toml
  - Cargo.lock
  - crypto/Cargo.toml
  - crypto/src/lib.rs
  - crypto/src/kdf.rs
  - crypto/src/page.rs
  - crypto/src/mac.rs
  - crypto/src/key.rs
  - crypto/tests/rfc4231.rs
  - crypto/tests/rfc5869.rs
  - crypto/tests/rfc8439.rs
  - crypto/tests/blake3_vectors.rs
  - crypto/tests/keys.rs
  - crypto/tests/vectors/blake3_test_vectors.json
  - ds/Cargo.toml
  - ds/src/lib.rs
  - ds/src/roaring.rs
  - ds/src/setops.rs
  - ds/src/clientset.rs
  - ds/src/tpht/mod.rs
  - ds/src/tpht/feistel.rs
  - ds/src/tpht/flattened.rs
  - ds/src/tpht/chained.rs
  - ds/src/tpht/measure.rs
  - xtask/src/lib.rs
  - xtask/src/main.rs
  - xtask/src/lintcontract.rs
  - xtask/src/miri.rs
  - flake.nix
  - UNSAFE-AUDIT.md
findings:
  critical: 0
  warning: 7
  info: 7
  total: 14
  necessity: 1
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-11-24T18:52:36Z
**Depth:** standard
**Files Reviewed:** 30
**Status:** issues_found

## Summary

The phase delivers crypto/ (HMAC-SHA256, HKDF-SHA256, ChaCha20/RFC 8439, keyed
BLAKE3, key hierarchy) and ds/ (hand-written Roaring, setops, ClientSet,
succinct TPHT with chained overflow, Feistel quotienting, D-15 measurement),
plus the static lint contract, the Miri step and the flake toolchain
derivations.

**What held up under adversarial checking.** All known-answer constants were
recomputed independently of the implementation under test and match: RFC 4231
cases 1–7, RFC 5869 A.1–A.3, RFC 8439 §2.3.2 and §2.4.2, the BLAKE3 vector
file's documented sha256 (`dcb91e…f624`, 31,922 bytes) and first case, the four
`mix32` KATs, and the `k_file` KAT (`ce9000…0afe`, recomputed with Python
`hmac` + RFC 5869 §2.3). The ChaCha20 counter limit
(`page.rs:73-84`) matches the pinned crate's `remaining_blocks =
u32::MAX - block_pos` exactly, including the 0-length-buffer and
buffer-untouched promises. The Roaring container algebra (run insert/remove
merging, array/bitmap/run pairings, cardinality bookkeeping, conversion sites)
was re-implemented from the source and differentially fuzzed against a set
model with no divergence. The one `unsafe` site
(`flattened.rs:440`) is sound: `slot_mask == arena.len() - 1` and
`arena.len() == 16 * num_blocks` is a power of two, so the masked index is
always in bounds; every reader-touched word is an atomic and the seqlock
follows the documented Boehm ordering. No functional bug was found in the
crypto or TPHT hot paths.

**What did not hold up.** The mechanical gates have holes: the lint contract's
dependency rules can be evaded with valid Cargo syntax (renamed or dotted-key
dependencies), and the Miri step — the phase's evidence for the seqlock and
bank protocols — is not invoked by any automated path. Two public data types
(`ClientSet`, `ChainedTphtOverflow`) expose unvalidated constructors/indices
that panic, and `verify_keyed`'s 1-byte tag floor lets a truncated slice
silently weaken authentication. Key-material zeroization stops at `Key` and
does not cover the HKDF PRK or the HMAC/HKDF working state. The rest are API
preconditions (`feistel::split`), seqlock counter arithmetic, one necessity
item, and documentation precision in the unsafe audit.

**Environment note.** No Rust toolchain was available in this review
environment (`cargo`/`rustc` not on PATH, no Nix development shell), so the
test suite was not executed; vector verification was done with independent
Python implementations and the source was checked statically.

## Warnings

### WR-01: Lint contract can be evaded by renamed and dotted-key dependencies

**File:** `xtask/src/lintcontract.rs:367-375` (R4), `xtask/src/lintcontract.rs:470-477` (R6), helper at `xtask/src/lintcontract.rs:312-329`
**Issue:** Both dependency rules identify a dependency by the raw text before `=` (`code.split('=').next()`). Two forms that Cargo fully accepts define a real dependency yet are not matched:

* **Renamed dependency:** `crypto_impl = { package = "crypto", path = "../crypto" }` (or `ds_impl`, `bitmap = { package = "roaring", … }`). The key is `crypto_impl`, so R4 reports nothing while the kernel crate does depend on `crypto`.
* **Dotted key:** `crypto.workspace = true` / `roaring.version = "=0.11.5"` inside `[dependencies]`. TOML builds the nested table `crypto = { workspace = true }`, a valid Cargo dependency entry, but the parsed key is `crypto.workspace`, which is neither `crypto` nor `ds`/`roaring`.

R4 exists to enforce D-21 ("no kernel crate may depend on crypto/ or ds/ in Phase 3") and R6 to keep `roaring` out of `[dependencies]` (D-19). A kernel crate can satisfy its dependency through either form and `cargo xtask test` will still print the contract as green. R5 has the same class of blind spot: it scans only `.cargo/config.toml`, while Cargo also reads a legacy `.cargo/config`.

**Fix:** Parse the dependency entries rather than raw lines — `toml` is already an xtask dependency — and check the resolved package name. Minimally:

```rust
// key side: take the first segment of a dotted key
let key = code.split('=').next().unwrap_or("").trim().trim_matches('"');
let name = key.split('.').next().unwrap_or("");
let value = code.split_once('=').map(|(_, v)| v).unwrap_or("");
let renamed_to = ["crypto", "ds", "roaring"]
    .into_iter()
    .find(|p| value.replace(' ', "").contains(&format!("package=\"{p}\"")));
if matches!(name, "crypto" | "ds") || renamed_to == Some("crypto") || /* … */ {
    // flag, using the resolved package name in the message
}
```

and add fixture tests for `crypto_alias = { package = "crypto" }`, `crypto.workspace = true`, and `roaring.version = …` under `[dependencies]`.

### WR-02: The Miri gate is not invoked by any automated path

**File:** `xtask/src/miri.rs:112-145`; entry points at `xtask/src/main.rs:217-232` and `xtask/src/main.rs:332`
**Issue:** `cargo xtask miri` is the phase's evidence for the seqlock, bank-claim and two-phase-publication protocols (D-13), and `miri.rs:4-10` deliberately keeps it out of `cargo xtask test`. But nothing else runs it: no job in `.github/workflows/gates.yml` or `.github/workflows/xtask.yml` mentions miri, the flake's `checks` set (`flake.nix:996-1005`) contains only `harvey-harness` and `harvey-drawterm`, and `TestStep::run` (`xtask/src/main.rs:3570-3698`) has no miri leg. The only caller of `miri::run` is the manually typed subcommand. A gate that no pipeline invokes cannot detect a regression (a newly introduced data race in `tpht/` would pass every green run), and the "no test is gated out of Miri" claim in `tpht/mod.rs:18-20` is only reproducible by hand.
**Fix:** Add a CI job (`nix develop -c cargo xtask miri`, since the devshell exports `R9_MIRI_TOOLCHAIN`), or a `checks` attribute in `flake.nix`, or a `cargo xtask test --miri` opt-in that the gate job uses. Keeping the default fast is fine; leaving it unreachable is not.

### WR-03: `ClientSet`'s canonical-form invariant is not enforced by its type

**File:** `ds/src/clientset.rs:32-38`, `:64-72`, `:102-118`, `:128`
**Issue:** `ClientSet` is a `pub enum` whose `Inline { count: u8, ids: [u32; 4] }` fields are public, so any dependent crate can construct a non-canonical value. Every method then trusts `count` and sortedness:

* `insert` and `contains` do `ids[..*count as usize]` and `iter` slices `ids[..count]` — `Inline { count: 5, .. }` (or anything larger) panics with an out-of-bounds slice; in a kernel `panic = "abort"` this halts the system.
* `contains`/`remove` use `binary_search` on the slice, so unsorted ids return wrong membership answers instead of an error.
* An `Inline { count: 1, .. }`, which `remove` itself would produce if it ever saw one, removes its last id into `Inline { count: 0 }` (`:111-116` checks only `count == 1` → `Single`) rather than `Empty`, so the documented canonical forms (`Empty`/`Single`/`Inline` 2–4/`Roaring`) are violated.

The module documentation calls these forms "canonical … fixed here", and Phase 4 kernel code will be the first external constructor.

**Fix:** Keep the representation private and expose the canonical states through methods:

```rust
pub struct ClientSet { repr: Repr }

enum Repr {           // stays private
    Empty,
    Single(u32),
    Inline { count: u8, ids: [u32; 4] },
    Roaring(Box<RoaringBitmap>),
}
```

(or `#[non_exhaustive]` plus private fields, which also blocks struct-literal construction), and make `remove`'s `count == 1` branch cover `count == 0` → `Empty` defensively.

### WR-04: `verify_keyed` accepts 1-byte tags, so a truncated slice silently weakens authentication to 8 bits

**File:** `crypto/src/mac.rs:81-108`; forwarded unchanged by `crypto/src/key.rs:176-178` and `:222-224`
**Issue:** The only verification entry point accepts any tag length in `1..=BLAKE3_OUT_LEN` (`mac.rs:84-86`). The RFC's `StatelessCap` tag is 8 bytes and the wire/page MAC is 32, but nothing in the API distinguishes them: `KeyCapability::verify` passes the caller's slice straight through. A caller that passes `&tag[..3]` (a truncated read, a copy-paste from the 8-byte example, a length taken from an untrusted header) gets 24-bit authentication and `Ok(())`; a 1-byte tag gives 8 bits. The constant-time comparison itself is correct; the length *policy* is the hole, and no test pins a minimum above 1.
**Fix:** Make the tag's length part of the type, or restrict the accepted lengths:

```rust
/// The two tags r9 produces: the 32-byte MAC and the 8-byte capability tag.
pub fn verify_keyed(key: &[u8; BLAKE3_KEY_LEN], msg: &[u8], tag: &[u8]) -> crate::Result<()> {
    if tag.len() != BLAKE3_OUT_LEN && tag.len() != 8 {
        return Err(crate::Error::BadTagLength);
    }
    // … existing constant-time compare
}
```

with an explicit constant for the capability-tag length and tests for 1/7/9-byte rejection.

### WR-05: The HKDF PRK and derivation working state are not zeroized

**File:** `crypto/src/kdf.rs:48-53` (bare PRK return), `:29-36`/`:73-78`; `crypto/src/key.rs:116-122`; `crypto/Cargo.toml:20-26`
**Issue:** `Key` is careful (`key.rs:53-81`: `Zeroizing<[u8; 32]>`, no byte accessor, no `Debug`), but the derivation path is not:

* `hkdf_extract` returns the PRK as a plain `[u8; 32]` on the caller's stack, and `hkdf_expand` takes a plain `&[u8; 32]` — the secret intermediate is never wiped. This is the natural Phase 4 API for deriving `K_user`/`K_session`.
* `k_file`/`derive_labelled` build an `Hmac`/`Hkdf` from the parent key; the HMAC key schedule (ipad/opad) and `Hkdf`'s stored PRK are dropped without zeroizing.
* `Blake3::new_keyed` (`mac.rs:58-61`) copies the key into `blake3::Hasher`, which offers no zeroization path at all.
* `crypto/Cargo.toml:20-26` says "hmac has no feature this crate needs", but `hmac 0.13` exposes `zeroize = ["digest/zeroize"]` (and `sha2 0.11` the same). The comment is inaccurate even though `digest`'s feature only covers `CtOutput`/fixed-buffer wrappers and would not wipe `HmacCore`'s pads.

**Fix:** Return `Zeroizing<[u8; 32]>` from `hkdf_extract` and accept `&Zeroizing<[u8; 32]>` (or a `Key`) in `hkdf_expand`; correct the manifest comment to record why the upstream `zeroize` feature is (or is not) sufficient; and either enable `hmac`/`sha2` `zeroize` or document explicitly that the key-derivation working state is not wiped, with a hand-rolled `Zeroizing` expand loop if the kernel threat model requires it. Also record the `Blake3` incremental-hasher limitation beside `Blake3::new_keyed`.

### WR-06: `feistel::split`/`join` do not enforce their `index_bits`/`index` preconditions

**File:** `ds/src/tpht/feistel.rs:59-62`, `:67-74`, `:78-83`
**Issue:** `low_mask` is documented "for `index_bits <= 32`", but `split` and `join` are `pub` inside `pub mod tpht` and accept any `u32`:

* `index_bits >= 64` makes `1u64 << index_bits` an arithmetic-overflow panic in debug/test builds, and in release the shift is masked, so e.g. `split(key, 65)` drops `L'`'s top bit from the remainder and `join` returns a *different* key — silent key corruption in a hash table.
* `join` masks `index` to `index_bits` bits, so an out-of-range index silently produces a different key instead of an error.

All internal callers derive the width from `index_bits()` (≤ 32), so the current code is safe; the public API has no guard.

**Fix:**

```rust
pub fn split(key: u64, index_bits: u32) -> (usize, u64) {
    assert!(index_bits <= 32, "a TPHT home index is at most 32 bits");
    // …
}

pub fn join(index: usize, remainder: u64, index_bits: u32) -> u64 {
    assert!(index_bits <= 32, "a TPHT home index is at most 32 bits");
    debug_assert!((index as u64) < (1u64 << index_bits));
    // …
}
```

(or make both `pub(crate)` and keep the assertion).

### WR-07: Seqlock generation arithmetic can overflow in `unlock`

**File:** `ds/src/tpht/flattened.rs:493-495` (and `:480-485`)
**Issue:** `lock` returns an even generation; `unlock` stores `generation + 2`. After `2^31` lock/unlock cycles on one block, `generation == u32::MAX - 1` and `generation + 2` overflows — a panic in any build with overflow checks (the dev/test profile; `[profile.dev]` does not disable them), and a wrap to 0 in release. The release wrap is benign for the even/odd invariant but leaves the reader's `seq_lock != first` equality check ABA-prone across the wrap. The likelihood is low (2^31 writes to one block), but a concurrency primitive should not have a panicking release path, and the fix is one word.
**Fix:**

```rust
fn unlock(header: &TphtBucketHeader, generation: u32) {
    // The generation is modulo 2^32; equality in `lookup` is a
    // "no write in this window" test, not a lifetime-unique stamp.
    header.seq_lock.store(generation.wrapping_add(2), Ordering::Release);
}
```

Use `wrapping_add(1)` in `lock`'s CAS for symmetry.

## Info

### IN-01: `derive_labelled`'s unframed `label || context` allows cross-context key collisions

**File:** `crypto/src/key.rs:103-123`; pinned by `crypto/tests/keys.rs:110-125`
**Issue:** The concatenation is deliberate (the RFC's `"9P-AEAD-FID" || fid` shape) and the test pins it, but the API cannot enforce the "fixed-length label" convention it relies on: the first Phase 4 caller that uses a variable-length label makes `("ab","c")` and `("a","bc")` derive the same key. `k_file` and `derive_labelled` also share the same parent/expand with no domain tag, so a caller that reuses one `Key` for both can hit an `info` collision.
**Fix:** Length-prefix the label (or make it a fixed-size `[u8; N]` / const generic), or prepend a per-function domain label; at minimum, record the fixed-label rule as a type-level invariant the first caller must satisfy.

### IN-02: `KeyCapability::new` does not reject `k_dec == k_mac`

**File:** `crypto/src/key.rs:152-156`
**Issue:** T-03-18's separation (a tag sealed under `k_mac` must not verify under `k_dec`) is only tested for *different* keys (`crypto/tests/keys.rs:192-207`). `KeyCapability::new(k, k.clone())` is accepted, reusing one 256-bit key for ChaCha20 and keyed BLAKE3 — exactly the key reuse the test is guarding against.
**Fix:** `debug_assert!(!k_dec.ct_eq(&k_mac), "k_dec and k_mac must be distinct (T-03-18)")` (or return a `Result` and reject in release too), or derive both keys from a labelled expand inside the constructor.

### IN-03: `cargo xtask miri --seeds 0` is accepted

**File:** `xtask/src/main.rs:226-231`; `xtask/src/miri.rs:93-107`
**Issue:** The step already fails closed when the `tpht` filter would match nothing (`miri.rs:122-125`), but it does not reject an empty seed range: `--seeds 0` yields `-Zmiri-many-seeds=0..0`, which either errors obscurely or runs no seeds. A gate should refuse a vacuous run for the same reason it refuses an empty test filter.
**Fix:** `if seeds == 0 { return Err("miri: --seeds 0 would run no seeds".into()); }` (and consider a minimum of 2).

### IN-04: The unsafe-token scan strips `//` anywhere and ignores block comments

**File:** `xtask/src/lintcontract.rs:98-103`, `:222-232`, `:262-308`
**Issue:** `strip_line_comment` truncates at the first `//`, including inside a string literal, and block comments are never stripped. A line such as `let u = "http://x"; unsafe { … }` outside `ds/src/tpht` would pass the token scan (the per-module `#[forbid(unsafe_code)]` is the real lock, so the failure would surface at compile time rather than in the contract check); conversely `/* unsafe */` is a false positive. R3's forbid rule also reads only `ds/src/lib.rs`, so a module declared in any other file carries no compile-time lock and is not checked.
**Fix:** strip `/* … */` spans and skip string literals in the scanner (or accept the documentation's "plain text scan" limitation explicitly and rely on the forbids); scan module declarations for the forbid rule in every non-`tpht` file, not just the crate root.

### IN-05: `spilled_mask` maintenance is write-only

**File:** `ds/src/tpht/flattened.rs:97`, `:303`, `:405`, `:500-515`
**Category:** necessity
**Issue:** `set_spilled` runs on every home insert and `clear_spilled` on every remove, updating `spilled_mask`; no code ever reads the field (its only loads are inside those two functions). The state they guard — a home tiny pointer above the home window — cannot occur: the sole writer assigns `slot + 1` with `slot < HOME_SLOTS` (`flattened.rs:299`), and `spilled_mask` is otherwise initialised to zero. The per-insert maintenance therefore cannot affect any observation. (Counter-consideration: the field itself is part of the mandated 64-byte header layout, so keeping it zero-initialised is reasonable; the *maintenance* is the unnecessary part, and `clock_nibbles` is in the same position.)
**Fix:** Drop the two calls (keep the field and its `new()` initialisation), or keep them and state in the module docs that no reader consumes the mask in Phase 3 and the maintenance exists only for the future neighbourhood-displacement writer.

### IN-06: The `deref_tiny` safety justification names one of four constructors

**File:** `ds/src/tpht/flattened.rs:434-440`; `UNSAFE-AUDIT.md:98`
**Issue:** The SAFETY comment (and the audit row) justify the unchecked read with "`with_probe` allocates the arena with exactly `slot_mask + 1` slots". `deref_tiny` is reachable on shards built by `new`, `with_banks`, `with_banks_and_probe` and `with_probe`; the argument remains valid because all four delegate to `with_banks_and_probe` (`flattened.rs:165-167`, `:189-191`, `:194-223`), but an auditor checking only `with_probe` is being pointed at one of four constructors. (The bound itself is sound: `slot_mask == arena.len() - 1` and `arena.len() == 16 * num_blocks` is a power of two, so the mask is a no-op bound.)
**Fix:** Say "every constructor delegates to `with_banks_and_probe`, which allocates `HOME_SLOTS * num_blocks` slots, and `slot_mask` is `slots - 1` because that product is a power of two" in both the comment and the audit row.

### IN-07: `ChainedTphtOverflow` is public with an unvalidated `block` index

**File:** `ds/src/tpht/chained.rs:170-179`, `:186-233`, `:237-269`
**Issue:** `candidate_banks`, `insert`, `lookup` and `remove` index `self.heads[block]` (and the selected banks) without a bounds check. The type is `pub` in `pub mod tpht`, so a caller passing `block >= num_blocks` gets an out-of-bounds panic (kernel abort). The flattened shard always derives `block` from `feistel::split` with the validated width, so the fix is visibility rather than validation.
**Fix:** Make `ChainedTphtOverflow` `pub(crate)` (its tests are in-crate), or return `TphtError::BadBlockCount` for an out-of-range block.

---

_Reviewed: 2026-11-24T18:52:36Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
