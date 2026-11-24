//! page — ChaCha20 in the RFC 8439 layout (CRYP-02, D-06).
//!
//! Layout (RFC 8439 §2.3): a 256-bit key, a 32-bit little-endian block
//! counter and a 96-bit nonce.  The caller supplies both the nonce and the
//! counter; this module never chooses, defaults or generates either, and no
//! function takes fewer parameters (T-03-10).  The caller must never reuse a
//! `(key, nonce, counter)` triple for two different messages: doing so XORs
//! the same keystream twice and reveals the plaintext.
//!
//! How the 8-byte wire sequence nonce is extended to the 12-byte ChaCha20
//! nonce is Phase 4's rule, not this module's (D-06).
//!
//! The pins are the RFC 8439 vectors: the block function (§2.3.2) and the
//! 114-byte encryption example (§2.4.2), in `crypto/tests/rfc8439.rs`.
//! Their results are recorded in `evidence/PINS-03-03.md`.
//!
//! # Limit per key and nonce
//!
//! This module produces block indices 0 to `u32::MAX - 1` and never block
//! index `u32::MAX`, so one key and nonce cover at most 2^32 - 1 blocks,
//! 274,877,906,880 bytes (256 GiB less 64 bytes) from counter 0.  RFC 8439
//! allows one block more (2^32 blocks).  The difference is the pinned
//! `chacha20 =0.10.2` crate, which refuses the last counter value so its
//! counter can never wrap.  A call that would need block index `u32::MAX`
//! returns `Err(Error::CounterOverflow)` and writes nothing.

/// The ChaCha20 key length in bytes (RFC 8439 §2.3: 256 bits).
pub const CHACHA20_KEY_LEN: usize = 32;

/// The ChaCha20 nonce length in bytes (RFC 8439 §2.3: 96 bits).
pub const CHACHA20_NONCE_LEN: usize = 12;

/// The ChaCha20 block (keystream) length in bytes (RFC 8439 §2.3: 512 bits).
pub const CHACHA20_BLOCK_LEN: usize = 64;

/// The number of blocks one key and nonce can produce through this module:
/// 2^32 - 1, covering block indices 0 to `u32::MAX - 1`.  See the module
/// documentation's "Limit per key and nonce" section.
pub const CHACHA20_MAX_BLOCKS: u64 = u32::MAX as u64;

/// The ChaCha20 block function (RFC 8439 §2.3): the 64-byte serialized
/// keystream block for `counter`.
///
/// The caller supplies the 32-bit counter and the 96-bit nonce explicitly.
/// Block index `u32::MAX` is never produced: that call returns
/// `Err(Error::CounterOverflow)` and no block.
pub fn chacha20_block(key: &[u8; 32], nonce: &[u8; 12], counter: u32) -> crate::Result<[u8; 64]> {
    // RED skeleton (plan 03-03): the surface compiles and the pin fails on
    // its assertion, not on a compile error.
    let _ = (key, nonce, counter);
    Ok([0u8; CHACHA20_BLOCK_LEN])
}

/// XOR the ChaCha20 keystream into `buf` (RFC 8439 §2.4), starting at block
/// `counter`.
///
/// The caller supplies the 32-bit counter and the 96-bit nonce explicitly,
/// and owns the no-reuse duty for `(key, nonce, counter)`.  A zero-length
/// buffer is `Ok` for every counter value.  A buffer that would need block
/// index `u32::MAX` returns `Err(Error::CounterOverflow)` and leaves `buf`
/// untouched.
pub fn chacha20_xor(
    key: &[u8; 32],
    nonce: &[u8; 12],
    counter: u32,
    buf: &mut [u8],
) -> crate::Result<()> {
    // RED skeleton (plan 03-03): returns Ok without touching a byte.
    let _ = (key, nonce, counter, buf);
    Ok(())
}
