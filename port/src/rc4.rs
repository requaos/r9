//! rc4 — the rc4_256 key class + the SSL-format framing arithmetic (DESK-04,
//! 19-04).  A donor-verbatim port of libsec's `sys/src/libsec/port/rc4.c`
//! (the donor of record for the bytes: `setupRC4state` + the `rc4` stream
//! op — no re-derivation, no modernization) plus the two framing
//! primitives 9k's `sys/src/9k/port/devssl.c` computes inline (the
//! max/maxpad record-bound arithmetic at devssl.c:1189-1195) and the
//! `dec64` secret codec from `sys/src/libc/port/u64.c` the ctl plane
//! decodes secrets with (devssl.c:1196-1210).  Donor line cites below
//! name the semantics being ported; this is not a literal translation
//! (the project's D-06: port the donor, never the phrase).
//!
//! WHY hand-rolled from the donor and nothing else: interop.  devssl's
//! peer is drawterm's vendored `kern/devssl.c`, and both arms of the
//! cpu session (`sys/src/cmd/cpu.c` p9auth/srvp9auth) derive the same
//! `mksecret`-shaped inputs — rc4 bytes must be byte-exact with the
//! donor's algorithm on BOTH ends or auth succeeds and then every read
//! garbles silently (19-RESEARCH Pitfall 11).  No external crate
//! (T-19-SC); the SHA1 half is NOT here — `hmacsha1.rs`'s existing
//! block is the one SHA-1 this tree carries (its `sha1()` one-shot
//! composes the donor's streaming secret‖message‖msgid hash exactly).
//!
//! Scoping (recorded per the plan): the NOSPOOKS subset pushssl's
//! default `rc4_256 sha1` exercises is ALL this module carries — the
//! rc4_256 key class (initRC4key: setupRC4state over the full secret
//! length, no masking), never rc4_40/rc4_128/DES (out of measure);
//! dec64 only (enc64 is out — nothing in the kernel arm encodes).
//! rc4_256/sha1 is the accepted era shape: known-weak by modern
//! standards and PROTOCOL-blocked from stronger crypto per DESK-04's
//! requirement text (T-19-08 disposition: accept, documented; the
//! hardening milestone owns the upgrade).

extern crate alloc;

/// dec64's INVAL marker (donor u64.c: `INVAL = 255`).
const INVAL: u8 = 255;

/// The donor's RC4 key schedule (libsec rc4.c setupRC4state): the
/// identity permutation, then 256 swap steps keyed by the rotating key
/// bytes — `index2 = (*p + t + index2) & 255`, swap `state[i]`/`
/// `state[index2]`, `p` wraps at the key end.  The donor key classes
/// only differ in how they cap `n` (initRC4key passes the secret whole
/// — the rc4_256 class).
///
/// Panics on an empty key (the donor's C reads `*p` off a zero-length
/// range — UB it never reaches because devssl gates dec64 length > 0;
/// the port makes the unreachable loud instead).
pub fn setup_rc4_state(key: &[u8]) -> Rc4state {
    assert!(!key.is_empty(), "rc4: empty key (dec64's t>0 gate makes this unreachable)");
    let mut st = Rc4state { x: 0, y: 0, state: [0; 256] };
    for i in 0..256 {
        st.state[i] = i as u8;
    }
    let mut index2: usize = 0;
    let mut p = 0usize;
    for i in 0..256 {
        let t = st.state[i];
        index2 = (key[p] as usize + t as usize + index2) & 255;
        st.state[i] = st.state[index2];
        st.state[index2] = t;
        p += 1;
        if p >= key.len() {
            p = 0;
        }
    }
    st
}

/// The donor's RC4state (libsec rc4.h shape): the two stream indices +
/// the 256-byte permutation.
pub struct Rc4state {
    x: u8,
    y: u8,
    state: [u8; 256],
}

impl Rc4state {
    /// The donor's stream op (libsec rc4.c `rc4`): XOR `p` with the
    /// keystream, advancing `x`/`y` — `tx = state[++x&255]`,
    /// `y = (y+tx)&255`, swap, output `state[(tx+ty)&255]`.  In place,
    /// length-agnostic (the donor's `rc4(key, p, len)`).
    pub fn rc4(&mut self, p: &mut [u8]) {
        let mut x = self.x as usize;
        let mut y = self.y as usize;
        for b in p.iter_mut() {
            x = (x + 1) & 255;
            let tx = self.state[x];
            y = (y + tx as usize) & 255;
            let ty = self.state[y];
            self.state[x] = ty;
            self.state[y] = tx;
            *b ^= self.state[(tx as usize + ty as usize) & 255];
        }
        self.x = x as u8;
        self.y = y as u8;
    }
}

/// The SSL-format unpadded record bound (devssl.c:1189-1195): with a
/// stream cipher (`blocklen == 1`, the rc4_256 shape) `max` is
/// `(1<<15) - diglen - 1`; with a block cipher it is the same value
/// rounded DOWN to a block multiple.  The device composes this instead
/// of re-deriving — a cleaner port of the arithmetic breaks wire
/// interop silently (Pitfall 11), so the formula is pinned, not
/// restyled.
pub fn ssl_max(blocklen: u16, diglen: u16) -> u32 {
    let max = 32768 - diglen as u32 - 1;
    if blocklen != 1 {
        max - max % blocklen as u32
    } else {
        max
    }
}

/// The SSL-format padded record bound (devssl.c:1192-1195): stream
/// ciphers share `max`; block ciphers use `(1<<14) - diglen - 1`
/// rounded down to a block multiple.  rc4_256 never pads — the value
/// exists because the framing arithmetic is one pinned whole.
pub fn ssl_maxpad(blocklen: u16, diglen: u16) -> u32 {
    if blocklen != 1 {
        let maxpad = 16384 - diglen as u32 - 1;
        maxpad - maxpad % blocklen as u32
    } else {
        ssl_max(blocklen, diglen)
    }
}

/// The donor's dec64 decode table (u64.c `t64d`): standard base64
/// alphabet — 'A'-'Z' 0-25, 'a'-'z' 26-51, '0'-'9' 52-61, '+' 62,
/// '/' 63, everything else INVAL.  Built as a const fn (the donor's
/// 256-entry literal, provably the same table, byte-validated by the
/// known-answer pins below against independently computed vectors).
const fn t64d() -> [u8; 256] {
    let mut t = [INVAL; 256];
    let mut i = 0;
    while i < 26 {
        t[b'A' as usize + i] = i as u8;
        t[b'a' as usize + i] = (i + 26) as u8;
        i += 1;
    }
    i = 0;
    while i < 10 {
        t[b'0' as usize + i] = (i + 52) as u8;
        i += 1;
    }
    t[b'+' as usize] = 62;
    t[b'/' as usize] = 63;
    t
}

/// dec64 (donor u64.c): base64-decode `input` into `out`, returning the
/// decoded byte count.  THE QUIRK THAT IS LOAD-BARING: invalid chars —
/// including '=' — are SKIPPED, never terminal (donor `continue`s), and
/// the tail split counts only valid chars (i==2 tail emits 1 byte,
/// i==3 emits 2).  The secrets the ctl plane feeds this are
/// mksecret-shaped 20-char uppercase-hex ASCII (cpu.c `sprint(t,
/// "%2.2ux" x10)`), NOT base64 of the key: dec64 yields 15 bytes for
/// 20 chars where a hex decode would yield 10 — both session arms run
/// the same donor codec, so the pseudo-random 15 bytes ARE the rc4
/// secret (never hex-decode: the must-have's exact words).
pub fn dec64(out: &mut [u8], input: &[u8]) -> usize {
    const T64D: [u8; 256] = t64d();
    let mut b24: u32 = 0;
    let mut i: i32 = 0;
    let mut w = 0usize;
    for &ch in input {
        let c = T64D[ch as usize];
        if c == INVAL {
            continue;
        }
        match i {
            0 => b24 = (c as u32) << 18,
            1 => b24 |= (c as u32) << 12,
            2 => b24 |= (c as u32) << 6,
            _ => {
                b24 |= c as u32;
                if w + 3 > out.len() {
                    return w; // exhausted (the donor's goto)
                }
                out[w] = (b24 >> 16) as u8;
                out[w + 1] = (b24 >> 8) as u8;
                out[w + 2] = b24 as u8;
                w += 3;
                i = -1;
            }
        }
        i += 1;
    }
    match i {
        2 => {
            if w + 1 > out.len() {
                return w;
            }
            out[w] = (b24 >> 16) as u8;
            w += 1;
        }
        3 => {
            if w + 2 > out.len() {
                return w;
            }
            out[w] = (b24 >> 16) as u8;
            out[w + 1] = (b24 >> 8) as u8;
            w += 2;
        }
        _ => {}
    }
    w
}

// ---------------------------------------------------------------------------
// Host pins (the 19-04 Task-1 classes): every expected byte/number below
// was computed OUTSIDE this implementation (python's hashlib/base64 over
// the donor algorithms, cross-checked against the donor's own dec64
// emulation) and embedded as literals — a port bug cannot self-confirm.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02X}")).collect()
    }

    /// Pin 1 — the classic RC4 known vector (key "Key", plaintext
    /// "Plaintext" -> BBF316E8D940AF0AD3; the canonical Saferite-era
    /// vector every RC4 must reproduce) plus the donor-shaped 32-byte
    /// key class round-trip (rc4_256 = the unmasked full-length key:
    /// stream a buffer, stream it back through a FRESH state from the
    /// same key, bytes equal).
    #[test]
    fn rc4_known_vector_and_32byte_key_round_trip() {
        let ct = {
            let mut buf = *b"Plaintext";
            setup_rc4_state(b"Key").rc4(&mut buf);
            buf
        };
        assert_eq!(hex(&ct), "BBF316E8D940AF0AD3", "the classic RC4 vector");

        let key: [u8; 32] = core::array::from_fn(|i| i as u8);
        let pt: Vec<u8> = (0..100u32).map(|i| (i.wrapping_mul(7) + 3) as u8).collect();
        let mut ct2 = pt.clone();
        setup_rc4_state(&key).rc4(&mut ct2);
        assert_ne!(ct2, pt, "the stream must change the bytes");
        let mut back = ct2.clone();
        setup_rc4_state(&key).rc4(&mut back);
        assert_eq!(back, pt, "a fresh state from the same key streams back");
    }

    /// Pin 2 — state independence + position sensitivity: two states
    /// from the same key material are independent copies (same
    /// plaintext -> identical ciphertext; advancing one leaves the
    /// other untouched), and once their stream offsets differ the
    /// ciphertexts diverge — the property that makes the per-direction
    /// in/out states (and their mid-counted records) safe.
    #[test]
    fn rc4_states_are_independent_position_sensitive_copies() {
        let key = b"independent-state-key-material";
        let pt: Vec<u8> = (0..64u32).map(|i| (i * 31 + 7) as u8).collect();

        let mut a1 = pt.clone();
        setup_rc4_state(key).rc4(&mut a1);
        let mut a2 = pt.clone();
        setup_rc4_state(key).rc4(&mut a2);
        assert_eq!(a1, a2, "fresh states from one key agree");

        // Advance s2 by 16 bytes; the SAME plaintext now encrypts
        // differently under s2, while s1 still reproduces a1.
        let mut s2 = setup_rc4_state(key);
        let mut scratch = [0u8; 16];
        s2.rc4(&mut scratch);
        let mut b2 = pt.clone();
        s2.rc4(&mut b2);
        assert_ne!(b2, a1, "a stream advanced past its offset diverges");

        let mut again = pt.clone();
        setup_rc4_state(key).rc4(&mut again);
        assert_eq!(again, a1, "an untouched sibling state is unaffected");
    }

    /// Pin 3 — the max/maxpad framing arithmetic on the donor formulas
    /// at the boundary inputs: stream shape (blocklen 1) at diglen 0
    /// (the fd-verb default) and diglen 20 (SHA1dlen, the rc4_256+sha1
    /// live shape); block edges at blocklen 8 (the DES-class formula
    /// leg) at both digest widths.  Literals from the donor formulas,
    /// hand-evaluated: 32767/32747; 32760/16376; 32744/16360.
    #[test]
    fn framing_max_maxpad_match_the_donor_formulas() {
        assert_eq!(ssl_max(1, 0), 32767, "fd-verb default: (1<<15)-0-1");
        assert_eq!(ssl_maxpad(1, 0), 32767, "stream maxpad == max");
        assert_eq!(ssl_max(1, 20), 32747, "the rc4_256+sha1 live bound");
        assert_eq!(ssl_maxpad(1, 20), 32747, "stream maxpad == max");
        assert_eq!(ssl_max(8, 0), 32760, "32767 - 32767%8");
        assert_eq!(ssl_maxpad(8, 0), 16376, "16383 - 16383%8");
        assert_eq!(ssl_max(8, 20), 32744, "32747 - 32747%8");
        assert_eq!(ssl_maxpad(8, 20), 16360, "16363 - 16363%8");
    }

    /// Pin 4 — dec64 on the exact pushssl inputs: the mksecret-shaped
    /// 20-char uppercase-hex strings (derived here from the real
    /// composition — sha1 of the 16-byte session key, hex of each
    /// 10-byte half, the p9auth/srvp9auth shape) decode to 15 bytes
    /// each, NEVER the 10 a hex decode would yield; the tail splits at
    /// the boundaries (18 chars -> 13 bytes with a 2-char tail, 19 ->
    /// 14 with a 3-char tail); and the donor's skip-not-stop '=' quirk
    /// (TQ==TQ== -> 4D04D0, where standard base64 would give MM).
    #[test]
    fn dec64_decodes_mksecret_shapes_with_the_donor_tail_splits() {
        // mksecret(sha1("0123456789abcdef")[0..10]) = "FE5567E8D76955085218"
        // mksecret(sha1("0123456789abcdef")[10..20]) = "2CDF69D74BB16DFF8E29"
        // (python hashlib over the donor composition; dec64 vectors via
        // python base64 cross-checked against the donor's own codec.)
        for (input, want) in [
            (&b"FE5567E8D76955085218"[..], hex(&[
                0x14, 0x4E, 0x79, 0xEB, 0xB1, 0x3C, 0x0F, 0xBE, 0xBD, 0xE7, 0x9D, 0x3C, 0xE7, 0x6D, 0x7C,
            ])),
            (&b"2CDF69D74BB16DFF8E29"[..], hex(&[
                0xD8, 0x20, 0xC5, 0xEB, 0xD0, 0xFB, 0xE0, 0x10, 0x75, 0xE8, 0x31, 0x45, 0xF0, 0x4D, 0xBD,
            ])),
        ] {
            let mut out = [0u8; 31]; // the donor's (strlen*3)/2+1 bound
            let n = dec64(&mut out, input);
            assert_eq!(n, 15, "a 20-char mksecret shape decodes to 15 bytes, never hex's 10");
            assert_eq!(hex(&out[..n]), want, "byte-exact against the donor codec");
        }

        // Tail splits: 2-char tail -> 13 bytes for 18 chars; 3-char
        // tail -> 14 bytes for 19 chars (donor tail arithmetic).
        let mut out = [0u8; 31];
        assert_eq!(dec64(&mut out, b"0123456789ABCDEF01"), 13, "18-char tail split");
        assert_eq!(
            hex(&out[..13]),
            "D35DB7E39EBBF3D001083105D3",
            "the 18-char vector byte-exact"
        );
        assert_eq!(dec64(&mut out, b"0123456789ABCDEF012"), 14, "19-char tail split");

        // The skip-not-stop quirk: '=' is skipped and later chars still
        // feed groups (standard base64 would decode "TQ==TQ==" as MM).
        let mut q = [0u8; 8];
        let n = dec64(&mut q, b"TQ==TQ==");
        assert_eq!(n, 3, "the quirk's group count");
        assert_eq!(hex(&q[..3]), "4D04D0", "the donor's skip semantics, not standard's");
    }
}
