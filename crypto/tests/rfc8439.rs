//! Host-side pins for ChaCha20 (CRYP-02, RFC 8439).
//!
//! The expected values are copied from the RFC text at rfc-editor.org —
//! section 2.3.2 (the block function, counter 1) and section 2.4.2 (the
//! 114-byte "sunscreen" encryption, initial counter 1) — and never
//! re-derived from the implementation under test (03-RESEARCH.md Pitfall 10).
//! This file imports only `crypto`'s own API, never an upstream crate, so the
//! pins prove the wrapper (D-18).

use crypto::page::*;
use crypto::Error;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// RFC 8439 §2.3.2 key: 00:01:02:…:1f (32 octets, no structure before the
/// ChaCha state setup).
const RFC_KEY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
    0x1e, 0x1f,
];

/// RFC 8439 §2.3.2 nonce: 00:00:00:09:00:00:00:4a:00:00:00:00.
const RFC_BLOCK_NONCE: [u8; 12] =
    [0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00];

/// RFC 8439 §2.4.2 nonce: 00:00:00:00:00:00:00:4a:00:00:00:00.
const RFC_CIPHER_NONCE: [u8; 12] =
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4a, 0x00, 0x00, 0x00, 0x00];

/// RFC 8439 §2.3.2: the ChaCha20 block function with the RFC key, nonce and
/// block count 1 must serialize to the RFC's 64-byte block.
#[test]
fn chacha20_pin_rfc8439_2_3_2() {
    let block = chacha20_block(&RFC_KEY, &RFC_BLOCK_NONCE, 1).expect("counter 1 is in range");
    assert_eq!(
        hex(&block),
        "10f1e7e4d13b5915500fdd1fa32071c4\
         c7d1f4c733c068030422aa9ac3d46c4e\
         d2826446079faa0914c2d705d98b02a2\
         b5129cd1de164eb9cbd083e8a2503c4e",
        "ChaCha20 RFC 8439 §2.3.2 serialized block must match"
    );
}

/// RFC 8439 §2.4.2: encrypting the 114-byte "sunscreen" plaintext with the
/// RFC key, nonce and initial counter 1 must give the RFC ciphertext, and a
/// second call with the same parameters must restore the plaintext.
#[test]
fn chacha20_pin_rfc8439_2_4_2() {
    let plaintext = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
    let expected = "6e2e359a2568f98041ba0728dd0d6981\
                    e97e7aec1d4360c20a27afccfd9fae0b\
                    f91b65c5524733ab8f593dabcd62b357\
                    1639d624e65152ab8f530c359f0861d8\
                    07ca0dbf500d6a6156a38e088a22b65e\
                    52bc514d16ccf806818ce91ab7793736\
                    5af90bbf74a35be6b40b8eedf2785e42\
                    874d";

    let mut buf = *plaintext;
    chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, 1, &mut buf).expect("counter 1 is in range");
    assert_eq!(
        hex(&buf),
        expected,
        "ChaCha20 RFC 8439 §2.4.2 ciphertext must match"
    );

    chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, 1, &mut buf).expect("counter 1 is in range");
    assert_eq!(
        &buf[..],
        &plaintext[..],
        "applying chacha20_xor twice with the same parameters restores the plaintext"
    );
}

/// RFC 8439 §2.4: a keystream is block-aligned, so encrypting 63, 64 and 65
/// bytes gives prefixes of one keystream, and a split at a 64-byte boundary
/// with counters `n` and `n + 1` equals one call with counter `n`.
#[test]
fn chacha20_block_boundaries() {
    let counter = 7u32;

    let mut keystream = [0u8; 128];
    chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, counter, &mut keystream)
        .expect("two blocks from counter 7 is in range");

    for len in [63usize, 64, 65] {
        let mut buf = vec![0u8; len];
        chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, counter, &mut buf)
            .expect("a short buffer from counter 7 is in range");
        assert_eq!(
            &buf[..],
            &keystream[..len],
            "{len} bytes must be a prefix of the counter-{counter} keystream"
        );
    }

    let mut one_call = [0u8; 128];
    chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, counter, &mut one_call)
        .expect("two blocks from counter 7 is in range");
    let mut split_call = [0u8; 128];
    chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, counter, &mut split_call[..64])
        .expect("block 7 is in range");
    chacha20_xor(
        &RFC_KEY,
        &RFC_CIPHER_NONCE,
        counter + 1,
        &mut split_call[64..],
    )
    .expect("block 8 is in range");
    assert_eq!(
        one_call, split_call,
        "a split at 64 bytes with counters n and n + 1 must equal one call with counter n"
    );
}

/// The pinned `chacha20 =0.10.2` rule: block indices 0 to `u32::MAX - 1` are
/// usable and `u32::MAX` is refused.  Every refusal is
/// `Err(Error::CounterOverflow)` and leaves the buffer untouched.
#[test]
fn chacha20_counter_overflow_is_an_error() {
    let last = u32::MAX - 1;

    let mut at_last = [0x5au8; 64];
    assert_eq!(
        chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, last, &mut at_last),
        Ok(()),
        "block index u32::MAX - 1 is the last usable block"
    );

    let mut past_last = [0x5au8; 65];
    assert_eq!(
        chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, last, &mut past_last),
        Err(Error::CounterOverflow),
        "a 65-byte buffer at block index u32::MAX - 1 needs block u32::MAX"
    );
    assert!(
        past_last.iter().all(|&b| b == 0x5a),
        "an overflow refusal must leave the buffer untouched"
    );

    let two_before = u32::MAX - 2;
    let mut two_blocks = [0x5au8; 128];
    assert_eq!(
        chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, two_before, &mut two_blocks),
        Ok(()),
        "block indices u32::MAX - 2 and u32::MAX - 1 are usable"
    );
    let mut three_blocks = [0x5au8; 129];
    assert_eq!(
        chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, two_before, &mut three_blocks),
        Err(Error::CounterOverflow),
        "a 129-byte buffer at block index u32::MAX - 2 needs block u32::MAX"
    );
    assert!(
        three_blocks.iter().all(|&b| b == 0x5a),
        "an overflow refusal must leave the buffer untouched"
    );

    for len in [1usize, 64] {
        let mut buf = vec![0xa5u8; len];
        assert_eq!(
            chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, u32::MAX, &mut buf),
            Err(Error::CounterOverflow),
            "block index u32::MAX is never produced ({len}-byte buffer)"
        );
        assert!(
            buf.iter().all(|&b| b == 0xa5),
            "an overflow refusal must leave the buffer untouched"
        );
    }

    let mut empty: [u8; 0] = [];
    assert_eq!(
        chacha20_xor(&RFC_KEY, &RFC_CIPHER_NONCE, u32::MAX, &mut empty),
        Ok(()),
        "a zero-length buffer is Ok even at counter u32::MAX"
    );

    assert_eq!(
        chacha20_block(&RFC_KEY, &RFC_BLOCK_NONCE, u32::MAX),
        Err(Error::CounterOverflow),
        "chacha20_block never produces block index u32::MAX"
    );
    assert!(
        chacha20_block(&RFC_KEY, &RFC_BLOCK_NONCE, last).is_ok(),
        "chacha20_block produces block index u32::MAX - 1"
    );
}

/// The module's stated per-key-and-nonce limit: `CHACHA20_MAX_BLOCKS` is
/// 2^32 - 1 blocks, i.e. 274,877,906,880 bytes from counter 0.
#[test]
fn chacha20_last_block_constant_matches() {
    assert_eq!(
        CHACHA20_MAX_BLOCKS,
        u32::MAX as u64,
        "CHACHA20_MAX_BLOCKS is 2^32 - 1 blocks"
    );
    assert_eq!(
        CHACHA20_MAX_BLOCKS * CHACHA20_BLOCK_LEN as u64,
        274_877_906_880,
        "one key and nonce cover 274,877,906,880 bytes from counter 0"
    );
}

/// RFC 8439 §2.3: the block counter selects the block, so counters 1 and 2
/// give different keystream blocks for the same key and nonce.
#[test]
fn chacha20_counter_selects_the_block() {
    let first = chacha20_block(&RFC_KEY, &RFC_BLOCK_NONCE, 1).expect("counter 1 is in range");
    let second = chacha20_block(&RFC_KEY, &RFC_BLOCK_NONCE, 2).expect("counter 2 is in range");
    assert_ne!(
        first, second,
        "counters 1 and 2 must give different keystream blocks"
    );
}
