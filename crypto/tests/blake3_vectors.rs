//! Host-side pins for BLAKE3 against the official vector file (CRYP-03).
//!
//! Upstream: the BLAKE3 repository, `github.com/BLAKE3-team/BLAKE3`, file
//! `test_vectors/test_vectors.json` at tag `1.8.7` (the same file at tag
//! `1.0.0` and on `master`).  sha256
//! `dcb91ea8accc77e6d6e632af7cdc1a99a9f3ae78cf648da595c7d064db32f624`,
//! 31,922 bytes.  Licence of that repository:
//! `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`.
//!
//! The file is committed byte for byte and embedded at compile time
//! (`include_str!`), so the pins cannot be edited without changing this
//! test's fixture.  The file has a fixed shape — a top-level `key`, a
//! `context_string` and 35 `cases`, each with `input_len`, `hash`,
//! `keyed_hash` and `derive_key` — so a small fixed-shape extractor is
//! enough and no JSON crate enters the tree (03-RESEARCH.md F4).
//!
//! Every case's input is the repeating sequence `0, 1, ..., 250, 0, 1, ...`
//! (byte `i` is `i % 251`); each output field is 131 bytes of extended
//! output in hex.  Only the first 32 bytes (the default-length output) are
//! pinned here.  The cases from 1025 bytes up are the tree-mode pins, and
//! `keyed_hash` over them is the keyed tree mode.

use crypto::Error;
use crypto::mac::{BLAKE3_KEY_LEN, Blake3, blake3_hash, blake3_keyed, verify_keyed};

/// The official vector file, byte for byte (see the module documentation
/// for the upstream tag, sha256 and licence).
static VECTORS: &str = include_str!("vectors/blake3_test_vectors.json");

/// The number of cases in the official file: 35, asserted so a truncated
/// file cannot pass.
const CASE_COUNT: usize = 35;

/// One case of the official file.
struct Case<'a> {
    input_len: usize,
    hash: &'a str,
    keyed_hash: &'a str,
}

/// Read one quoted string field of a fixed-shape object.
fn string_field<'a>(obj: &'a str, name: &str) -> &'a str {
    let needle = format!("\"{name}\": \"");
    let start = obj.find(&needle).unwrap_or_else(|| panic!("case has no {name} field")) + needle.len();
    let rest = &obj[start..];
    let end = rest.find('"').expect("closing quote");
    &rest[..end]
}

/// Read one unsigned integer field of a fixed-shape object.
fn number_field(obj: &str, name: &str) -> usize {
    let needle = format!("\"{name}\": ");
    let start = obj.find(&needle).unwrap_or_else(|| panic!("case has no {name} field")) + needle.len();
    let rest = &obj[start..];
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().expect("integer field")
}

/// Split the file's `cases` array into its case objects, in file order.
fn cases(file: &str) -> Vec<Case<'_>> {
    let array = file
        .find("\"cases\": [")
        .expect("the official file has a cases array");
    let mut out = Vec::new();
    for chunk in file[array..].split('{').skip(1) {
        let Some(body) = chunk.split('}').next() else {
            continue;
        };
        if !body.contains("\"input_len\"") {
            continue;
        }
        out.push(Case {
            input_len: number_field(body, "input_len"),
            hash: string_field(body, "hash"),
            keyed_hash: string_field(body, "keyed_hash"),
        });
    }
    out
}

/// The file's `key` field (the 32-byte ASCII key of `keyed_hash`).
fn file_key(file: &str) -> &str {
    string_field(file, "key")
}

/// Decode the first 32 bytes of a 262-character extended-output field.
fn first32(hex: &str) -> [u8; 32] {
    assert!(hex.len() >= 64, "an output field holds at least 32 bytes of hex");
    let bytes = hex.as_bytes();
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        let pair = core::str::from_utf8(&bytes[2 * i..2 * i + 2]).expect("hex is ascii");
        *byte = u8::from_str_radix(pair, 16).expect("hex digits");
    }
    out
}

/// The case input: byte `i` is `i % 251` (the file's documented pattern).
fn case_input(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

#[test]
fn blake3_vector_file_is_the_official_one() {
    assert_eq!(
        cases(VECTORS).len(),
        CASE_COUNT,
        "the official BLAKE3 file has exactly {CASE_COUNT} cases"
    );
    assert_eq!(
        file_key(VECTORS),
        "whats the Elvish word for friend",
        "the official file's key field must be the documented 32-byte ASCII key"
    );
    assert_eq!(
        file_key(VECTORS).len(),
        BLAKE3_KEY_LEN,
        "the file's key is exactly one BLAKE3 key"
    );
}

#[test]
fn blake3_pin_official_hash_all_cases() {
    let parsed = cases(VECTORS);
    assert_eq!(parsed.len(), CASE_COUNT, "35 cases parsed");
    for case in &parsed {
        let input = case_input(case.input_len);
        assert_eq!(
            input.len(),
            case.input_len,
            "input_len {} is the generated length",
            case.input_len
        );
        assert_eq!(
            blake3_hash(&input),
            first32(case.hash),
            "BLAKE3 hash, official case input_len={}",
            case.input_len
        );
    }
}

#[test]
fn blake3_pin_official_keyed_hash_all_cases() {
    let parsed = cases(VECTORS);
    assert_eq!(parsed.len(), CASE_COUNT, "35 cases parsed");
    let key: [u8; BLAKE3_KEY_LEN] = file_key(VECTORS).as_bytes().try_into().expect("32-byte key");
    for case in &parsed {
        let input = case_input(case.input_len);
        assert_eq!(
            blake3_keyed(&key, &input),
            first32(case.keyed_hash),
            "BLAKE3 keyed_hash, official case input_len={}",
            case.input_len
        );
    }
}

#[test]
fn blake3_incremental_matches_one_shot() {
    let parsed = cases(VECTORS);
    assert_eq!(parsed.len(), CASE_COUNT, "35 cases parsed");
    let key: [u8; BLAKE3_KEY_LEN] = file_key(VECTORS).as_bytes().try_into().expect("32-byte key");
    for case in &parsed {
        let input = case_input(case.input_len);
        let midpoint = case.input_len / 2;

        let mut plain = Blake3::new();
        plain.update(&input[..midpoint]);
        plain.update(&input[midpoint..]);
        assert_eq!(
            plain.finalize(),
            blake3_hash(&input),
            "incremental BLAKE3 equals the one-shot result, input_len={}",
            case.input_len
        );
        assert_eq!(
            plain.finalize(),
            first32(case.hash),
            "incremental BLAKE3 equals the official hash, input_len={}",
            case.input_len
        );

        let mut keyed = Blake3::new_keyed(&key);
        keyed.update(&input[..midpoint]);
        keyed.update(&input[midpoint..]);
        assert_eq!(
            keyed.finalize(),
            blake3_keyed(&key, &input),
            "incremental keyed BLAKE3 equals the one-shot result, input_len={}",
            case.input_len
        );
        assert_eq!(
            keyed.finalize(),
            first32(case.keyed_hash),
            "incremental keyed BLAKE3 equals the official keyed_hash, input_len={}",
            case.input_len
        );
    }
}

#[test]
fn blake3_keyed_verify_accepts_and_rejects() {
    let key: [u8; BLAKE3_KEY_LEN] = file_key(VECTORS).as_bytes().try_into().expect("32-byte key");
    let msg = b"the 8-byte capability tag is a truncated keyed BLAKE3 tag";
    let tag = blake3_keyed(&key, msg);

    assert_eq!(verify_keyed(&key, msg, &tag), Ok(()), "the full 32-byte tag verifies");
    assert_eq!(
        verify_keyed(&key, msg, &tag[..8]),
        Ok(()),
        "the 8-byte prefix verifies (the RFC's truncated capability tag)"
    );

    let mut short = [0u8; 8];
    short.copy_from_slice(&tag[..8]);
    for bit in 0..64 {
        short[bit / 8] ^= 1 << (bit % 8);
        assert_eq!(
            verify_keyed(&key, msg, &short),
            Err(Error::TagMismatch),
            "flipping bit {bit} of the 8-byte tag must fail"
        );
        short[bit / 8] ^= 1 << (bit % 8);
    }

    let mut full = tag;
    for bit in 0..256 {
        full[bit / 8] ^= 1 << (bit % 8);
        assert_eq!(
            verify_keyed(&key, msg, &full),
            Err(Error::TagMismatch),
            "flipping bit {bit} of the 32-byte tag must fail"
        );
        full[bit / 8] ^= 1 << (bit % 8);
    }

    assert_eq!(
        verify_keyed(&key, msg, &[]),
        Err(Error::BadTagLength),
        "a zero-length tag is a length error"
    );
    assert_eq!(
        verify_keyed(&key, msg, &[0u8; 33]),
        Err(Error::BadTagLength),
        "a 33-byte tag is a length error"
    );
}
