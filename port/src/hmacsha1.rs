//! hmacsha1 — hand-rolled SHA-1 (RFC 3174) + HMAC (RFC 2104) in pure
//! safe Rust.  The 18-02 AUTH-03 substrate: devcap's `#¤` capability
//! format hashes with `hmac_sha1(data = "from@to", key = the cap's
//! base64 random part)` — the DONOR's userspace contract
//! (sys/src/cmd/auth/as.c mkcap:143 and factotum util.c's twin), not a
//! crypto choice: SHA-1 here is what the donor cap format specifies,
//! exactly as ChaCha20 is what the DRBG specifies (the no-new-crate
//! kernel discipline — devcons.rs's hand-rolled ChaCha20 is the style
//! and precedent; devcap.c links libsec's hmac_sha1 for the same
//! format on the donor kernel side).
//!
//! The exported call shape mirrors the donor's
//! `hmac_sha1(from, strlen(from), key, strlen(key), hash, nil)` with a
//! nil HMAC state: one shot, fixed 20-byte digest.
//!
//! Scope note (deliberate): this is a keyed-hash construction for
//! capability-token matching, not a collision-resistant signature
//! surface — the donor's threat model and ours (T-18-05) rest on the
//! known-answer vectors pinned below (RFC 2202 cases 1-2 + the FIPS
//! `abc` vector) and the devcap one-shot/FIFO semantics, never on
//! SHA-1's collision resistance.

extern crate alloc;

/// SHA-1's round constants (RFC 3174 §5): one per 20-round stage.
const K: [u32; 4] = [0x5A82_7999, 0x6ED9_EBA1, 0x8F1B_BCDC, 0xCA62_C1D6];

/// The SHA-1 block size in bytes (RFC 3174: 512-bit blocks) — HMAC's
/// ipad/opad width (RFC 2104 §2).
const BLOCK: usize = 64;

/// The SHA-1 digest length in bytes — the donor's `SHA1dlen` / devcap's
/// `Hashlen`.
pub const SHA1DLEN: usize = 20;

/// The SHA-1 compression function: one 64-byte block into the 5-word
/// state (RFC 3174 §6.1: message schedule + 80 rounds in four
/// 20-round stages, the feed-forward add).  `us` is the partial-block
/// tail scratch the streaming driver owns; both are fixed-size arrays
/// so the whole construction stays copy-free and const-friendly.
fn sha1_block(state: &mut [u32; 5], block: &[u8; BLOCK]) {
    let mut w = [0u32; 80];
    for (i, chunk) in block.chunks_exact(4).enumerate() {
        w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *state;
    for (i, &wi) in w.iter().enumerate() {
        let (f, k) = match i / 20 {
            0 => ((b & c) | (!b & d), K[0]),
            1 => (b ^ c ^ d, K[1]),
            2 => ((b & c) | (b & d) | (c & d), K[2]),
            _ => (b ^ c ^ d, K[3]),
        };
        let t = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(wi);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = t;
    }
    *state = [
        state[0].wrapping_add(a),
        state[1].wrapping_add(b),
        state[2].wrapping_add(c),
        state[3].wrapping_add(d),
        state[4].wrapping_add(e),
    ];
}

/// One-shot SHA-1 over `data` (RFC 3174): the padding (0x80, zeros, the
/// 64-bit big-endian bit length), then the block loop.  No streaming
/// surface — every caller this era (HMAC's inner/outer hashes, the
/// pins) hashes short bounded inputs (caps are at most a few hundred
/// bytes; the length counter is byte-count, not bit-shifted per byte).
pub fn sha1(data: &[u8]) -> [u8; SHA1DLEN] {
    let mut state = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
    // The padded message: data ‖ 0x80 ‖ zeros ‖ be64(bit length).  The
    // tail block always exists (8 bytes of length + 1 of 0x80 leave
    // the last block 0..=55 bytes of data when a fresh block opens).
    let bitlen = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % BLOCK != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bitlen.to_be_bytes());
    for block in padded.chunks_exact(BLOCK) {
        let mut b = [0u8; BLOCK];
        b.copy_from_slice(block);
        sha1_block(&mut state, &b);
    }
    let mut out = [0u8; SHA1DLEN];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// One-shot HMAC-SHA1 (RFC 2104 §2): `H(K ⊕ opad, H(K ⊕ ipad, text))`
/// with the donor's nil-state call shape —
/// `hmac_sha1(from, strlen(from), key, strlen(key), hash, nil)` (the
/// donor libsec entry devcap.c consumes).  Keys longer than the block
/// width are hashed first (RFC 2104 §3); shorter keys are zero-padded.
pub fn hmac_sha1(data: &[u8], key: &[u8]) -> [u8; SHA1DLEN] {
    // Key material into one block: hash oversized keys, zero-pad the
    // rest (RFC 2104 step 1-3).
    let mut kblock = [0u8; BLOCK];
    if key.len() > BLOCK {
        kblock[..SHA1DLEN].copy_from_slice(&sha1(key));
    } else {
        kblock[..key.len()].copy_from_slice(key);
    }
    // ipad 0x36 / opad 0x5c (RFC 2104 §2's constants).
    let mut inner = [0u8; BLOCK];
    let mut outer = [0u8; BLOCK];
    for i in 0..BLOCK {
        inner[i] = kblock[i] ^ 0x36;
        outer[i] = kblock[i] ^ 0x5c;
    }
    // H(K ⊕ ipad ‖ text)
    let mut inner_msg = inner.to_vec();
    inner_msg.extend_from_slice(data);
    let inner_hash = sha1(&inner_msg);
    // H(K ⊕ opad ‖ inner hash)
    let mut outer_msg = outer.to_vec();
    outer_msg.extend_from_slice(&inner_hash);
    sha1(&outer_msg)
}

// ---------------------------------------------------------------------------
// Known-answer pins (T-18-05): the FIPS 180 `abc` vector + RFC 2202
// HMAC-SHA1 cases 1 and 2, pinned BESIDE the implementation (the
// hand-rolled ChaCha20 discipline: vectors from the primary RFCs, never
// re-derived from the implementation under test).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// FIPS 180-1 / RFC 3174 §7.3 example B: SHA-1("abc").
    #[test]
    fn hmac_sha1_pin_fips_abc_vector() {
        assert_eq!(
            hex(&sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d",
            "SHA-1 of \"abc\" must match the RFC 3174 test vector"
        );
    }

    /// RFC 2202 §1 test case 1: key = 0x0b × 20, data = "Hi There".
    #[test]
    fn hmac_sha1_pin_rfc2202_case1() {
        let key = [0x0bu8; 20];
        assert_eq!(
            hex(&hmac_sha1(b"Hi There", &key)),
            "b617318655057264e28bc0b6fb378c8ef146be00",
            "HMAC-SHA1 RFC 2202 case 1 must match"
        );
    }

    /// RFC 2202 §1 test case 2: key = "Jefe",
    /// data = "what do ya want for nothing?".
    #[test]
    fn hmac_sha1_pin_rfc2202_case2() {
        assert_eq!(
            hex(&hmac_sha1(b"what do ya want for nothing?", b"Jefe")),
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79",
            "HMAC-SHA1 RFC 2202 case 2 must match"
        );
    }

    /// RFC 2202 §1 test case 3 (oversized key): key = 0xaa × 20 — same
    /// width as the digest, still under the block size; pins the
    /// short-key padding arm.
    #[test]
    fn hmac_sha1_pin_rfc2202_case3() {
        let key = [0xaau8; 20];
        assert_eq!(
            hex(&hmac_sha1(
                b"\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd\xdd",
                &key
            )),
            "125d7342b9ac11cd91a39af48aa17b4f63f175d3",
            "HMAC-SHA1 RFC 2202 case 3 must match"
        );
    }

    /// RFC 2202 §1 test case 6 (oversized key): key = 0xaa × 131 — over
    /// the block width, hashed first per RFC 2104 §3; pins the
    /// long-key arm.
    #[test]
    fn hmac_sha1_pin_rfc2202_case6_long_key() {
        let key = [0xaau8; 131];
        assert_eq!(
            hex(&hmac_sha1(
                b"Test Using Larger Than Block-Size Key - Hash Key First",
                &key
            )),
            "90d0dace1c1bdc957339307803160335bde6df2b",
            "HMAC-SHA1 RFC 2202 case 6 must match"
        );
    }
}
