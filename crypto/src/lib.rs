#![cfg_attr(not(any(test)), no_std)]
#![forbid(unsafe_code)]

//! crypto — r9's cryptographic substrate: HMAC-SHA256 and HKDF-SHA256,
//! ChaCha20 in the RFC 8439 layout, keyed BLAKE3 and the key hierarchy
//! (CRYP-01 to CRYP-04).
//!
//! Scope note (D-18): callers never name the upstream crates.  Every public
//! function takes and returns r9's own byte arrays, slices and error type, so
//! the pinned crates stay an implementation detail behind this API.  The
//! known-answer pins in `crypto/tests/` import `crypto` only, never an
//! upstream crate, and prove the wrapper rather than the crate beneath it.
//!
//! The crate-level `#![forbid(unsafe_code)]` (D-10) covers r9's own code in
//! this crate.  It does not and cannot cover the pinned upstream crates this
//! crate wraps: their `unsafe` is recorded in `UNSAFE-AUDIT.md` by version
//! and feature set, not enumerated site by site.

pub mod kdf;
pub mod page;

/// Errors returned by crypto's fallible operations.
///
/// Length and counter-overflow failures are reported rather than panicking
/// (03-RESEARCH.md Pattern 1): a kernel caller must not be able to abort the
/// system by asking for an impossible output length or stream position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// An HKDF expand asked for more than 255 × hash-length bytes
    /// (RFC 5869 §2.3).
    OutputTooLong,
    /// A ChaCha20 seek would pass the RFC 8439 32-bit block counter limit.
    CounterOverflow,
    /// A tag did not verify against the expected value.
    TagMismatch,
    /// A tag had the wrong length for the primitive.
    BadTagLength,
}

/// crypto's result alias.
pub type Result<T> = core::result::Result<T, Error>;
