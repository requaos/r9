# Phase 4: Guide Phase 2 - 9P2000.aead Transport and FidTable - Pattern Map

**Mapped:** 2026-11-24
**Files analyzed:** 27 (17 new, 10 modified; document edits listed separately)
**Analogs found:** 25 / 27

All analog paths below are git-tracked (`git ls-files` checked). Line numbers are at HEAD `e5db75b`.

## Precedence notes for the planner

Where `04-RESEARCH.md` and `04-CONTEXT.md` disagree, CONTEXT wins. The points that change what code is copied:

| Topic | RESEARCH text | Operative (CONTEXT) |
|-------|---------------|---------------------|
| Header length | user-constraints block still says 24 | **28 bytes** (D-06 corrected); offsets 0, 4, 5, 7, 8, 16, 24, 26 |
| Data messages | `Twrite`, `Rread`, `Tstream` | `Twrite`, `Rread`, **`Rstream`** (D-09) |
| Exchange bytes | "ride inside `Tauth` / `Rauth`" | standard `Tauth` / `Rauth`; bytes travel as `Tread` / `Twrite` on the auth fid (D-16, D-17) |
| Volatility class | WIRE-10 "`qtype` bits" | its own byte; never written into `dat::Qid.typ` (D-27) |
| Opcode set | RFC registry | plus `Rerror` 107, `Tflush` / `Rflush` 108 / 109 (D-28) |
| Byte evidence | "`.text` identical" | the replacement evidence set of RESEARCH F1 (D-29) |
| x64 TCG gate venue | amd64 container | natively on the owner's x86_64 Linux host (D-25) |

RESEARCH proposals tagged `[ASSUMED]` (TLV record format, `info` layout, opcodes 138/139, keystream counter placement, `ds` API additions) are discretion proposals, not locked. This map names the analog to copy from for each; it does not lock them.

## File Classification

### New files

| New file | Role | Data flow | Closest analog | Match quality |
|----------|------|-----------|----------------|---------------|
| `crypto/src/session.rs` | service (key derivation, per-direction seal/open primitives) | transform | `crypto/src/key.rs` | exact |
| `crypto/tests/session.rs` | test (KAT pins) | transform | `crypto/tests/keys.rs` | exact |
| `port/src/chan.rs` | module root (constants, re-exports) | n/a | `port/src/devip.rs` + `port/src/lib.rs` forbid note | role-match |
| `port/src/chan/wire.rs` | utility (header codec, opcode registry, `Unverified` / `Verified`) | transform | `port/src/devmnt.rs` codec (lines 88-231) + `frame_one` (491-534) | role-match |
| `port/src/chan/tlv.rs` | utility (`ExtMask`, TLV parse) | transform | `port/src/devmnt.rs` `RepCur` (166-231) | role-match |
| `port/src/chan/msg.rs` | utility (message body encode/decode) | transform | `port/src/devmnt.rs` `pw*` / `tmsg` / `RepCur` and the per-RPC bodies (141-164, 690-703, 863-876) | exact for the base set |
| `port/src/chan/transport.rs` | service (per-direction state, strict receive, re-key, teardown, `SessionExchange`) | streaming, request-response | `port/src/devssl.rs` (`OneWay` / `InDir` / `OutDir`, `sslbread`, `sslput_record`) | role-match |
| `port/src/fid.rs` | model + store (`StatefulHandle`, `StatelessCap`, `FidEntry`, `FidShard`, `FidTable`, `VolatilityClass`) | CRUD | `ds/src/tpht/flattened.rs` (API it wraps) + `crypto/src/key.rs` (fixed-width LE packing) | partial (no table-over-TPHT exists) |
| `port/tests/common/aead_link.rs` | test fixture (in-memory duplex link, fake clock) | streaming | `port/tests/devmnt_test.rs` `FakeServer` / `FakeHalf` (183-458) | role-match |
| `port/tests/aead_wire.rs` | test | transform | `port/tests/devmnt_test.rs` helpers (35-107) | role-match |
| `port/tests/aead_tlv.rs` | test | transform | `port/tests/devmnt_test.rs` | role-match |
| `port/tests/aead_transport.rs` | test | streaming | `port/tests/devmnt_test.rs` rejection pins (587-620) | role-match |
| `port/tests/aead_fid.rs` | test | CRUD | `ds/src/tpht/flattened.rs` tests (`CountingProbe` 533-570, DS-03 pin 1161-1199) | exact for the probe pin |
| `port/tests/aead_negotiate.rs` | test | request-response | `port/tests/devmnt_test.rs` `session()` and negotiate pins (460-476, 578-594) | exact |
| `xtask/src/test9p.rs` | config / build step | batch | `xtask/src/miri.rs` | exact |
| phase `evidence/kernel-bytes/before.sha256`, `KERNEL-BYTES.md` | evidence | file-I/O | `.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/kernel-bytes/` | exact |
| phase `evidence/PINS-04-NN.md` | evidence | file-I/O | `.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-04.md` | exact |

### Modified files

| Modified file | Role | Data flow | Analog (in-file precedent) | Match quality |
|---------------|------|-----------|----------------------------|---------------|
| `crypto/src/lib.rs` | module list, `Error` enum | n/a | itself, lines 19-43 | exact |
| `port/src/devmnt.rs` | service (`negotiate` branch) | request-response | itself, `negotiate` 670-716 | exact |
| `port/src/lib.rs` | module list | n/a | itself, lines 20-104 | exact |
| `port/Cargo.toml` | config | n/a | itself; comment style of `crypto/Cargo.toml` | exact |
| `ds/src/tpht/flattened.rs` | store (slot clearing on `remove`, `slot_id`, `read_slot`) | CRUD | itself, `remove` 382-415, `write_value` 463-469, `slot_index` 448-452 | exact |
| `xtask/src/lintcontract.rs` | lint rule R4 + fixtures | batch | itself, 335-378 and 733-767 | exact |
| `xtask/src/lib.rs` | module list | n/a | line 24 `pub mod miri;` | exact |
| `xtask/src/main.rs` | subcommand registration | n/a | `miri` at 217-232, 332, 4928-4940 | exact |
| `UNSAFE-AUDIT.md` | audit record | n/a | "Phase 3 delta" paragraph, lines 59-68 | exact |
| `flake.nix` | config (`boot-iso` `outputHash`, line 983) | n/a | owner-only edit; procedure in `02-OWNER-STOP.md` | exact |

Documents edited under D-10 (not code; pattern in "Shared Patterns / Dated amendment notes"): `.planning/rfc-9p2000-aead.md`, `.planning/PROJECT.md`, `.planning/STATE.md`, `.planning/REQUIREMENTS.md`, `.planning/r9-architecture-and-code-v4.md` (listing 9.9 at line 1293).

## Pattern Assignments

### `crypto/src/session.rs` (service, transform)

**Analog:** `crypto/src/key.rs`

This file must hold everything that touches raw key bytes, because `Key::bytes` is `pub(crate)` (key.rs:76-80) and `port` cannot key a hasher or cipher itself.

**Module doc and imports** (key.rs:1-46): a `//!` header naming the requirement IDs, the landing-site decision, the layout, and what the module does not decide. Imports are the upstream crates directly, which only `crypto/` may name:

```rust
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

/// The key length in bytes: one SHA-256 HKDF output, 256 bits.
pub const KEY_LEN: usize = 32;
```

**Key storage** (key.rs:53-81): private zeroizing bytes, `Clone`, no `Debug`, no public accessor. `DirectionKeys` follows this: private fields, no accessor for `k_enc`, `k_mac` or the 4-byte prefix.

```rust
#[derive(Clone)]
pub struct Key {
    bytes: Zeroizing<[u8; KEY_LEN]>,
}
    pub(crate) fn bytes(&self) -> &[u8; KEY_LEN] {
        &self.bytes
    }
```

**Core derivation pattern: fixed-width little-endian `info`, expand into `Zeroizing`, wrap** (key.rs:90-101). Copy this shape for the 18-byte session `info` (label, direction, purpose, generation):

```rust
pub fn k_file(k_user: &Key, qid_path: u64, qid_vers: u32) -> Key {
    let mut info = [0u8; 12];
    info[..8].copy_from_slice(&qid_path.to_le_bytes());
    info[8..].copy_from_slice(&qid_vers.to_le_bytes());

    let mut out = Zeroizing::new([0u8; KEY_LEN]);
    crate::kdf::hkdf_expand(k_user.bytes(), &info, &mut out[..])
        .expect("a 32-byte HKDF-SHA256 expand is within the 255 x 32 RFC 5869 limit");
    Key { bytes: out }
}
```

Note `Key { bytes: out }` is a private-field construction, available to `session.rs` only through `Key::from_bytes` (key.rs:60) unless a `pub(crate)` constructor is added. The extract step is `crate::kdf::hkdf_extract(salt, ikm) -> [u8; 32]` (kdf.rs:48-53; argument order is contract: salt first).

**Typestate over key holders that seal, verify and apply keystream** (key.rs:152-190). The per-direction key type copies these three method shapes; the bodies show exactly how to reach `mac` and `page` from a `Key`:

```rust
    pub fn seal(&self, msg: &[u8]) -> [u8; crate::mac::BLAKE3_OUT_LEN] {
        crate::mac::blake3_keyed(self.state.k_mac.bytes(), msg)
    }
    pub fn verify(&self, msg: &[u8], tag: &[u8]) -> crate::Result<()> {
        crate::mac::verify_keyed(self.state.k_mac.bytes(), msg, tag)
    }
    pub fn apply_keystream(&self, nonce: &[u8; 12], counter: u32, buf: &mut [u8]) -> crate::Result<()> {
        crate::page::chacha20_xor(self.state.k_dec.bytes(), nonce, counter, buf)
    }
```

**Incremental keyed MAC** (`crypto/src/mac.rs` 48-72): `PacketMac` wraps `Blake3::new_keyed(key)`, `update(&[u8])`, `finalize(&self) -> [u8; 32]`. Three `update` calls over header, TLVs and ciphertext give the D-07 slice with no copy.

**Constant-time verify** (mac.rs:101-108). There is no incremental constant-time verify today; `PacketMac::verify` and the 8-byte capability tag check copy this fold (also `Key::ct_eq`, key.rs:68-74):

```rust
    let mut acc = 0u8;
    for (computed_byte, tag_byte) in computed.as_bytes().iter().zip(tag) {
        acc |= computed_byte ^ tag_byte;
    }
    if acc == 0 { Ok(()) } else { Err(crate::Error::TagMismatch) }
```

`verify_keyed` (mac.rs:81-109) already accepts a 1 to 32 byte tag, so `cap_verify(&Key, &[u8], &[u8; 8])` is a one-line wrapper calling it with `key.bytes()`.

**Error handling** (`crypto/src/lib.rs` 24-43): fallible operations return `crate::Result<T>` over the `Error` enum (`OutputTooLong`, `CounterOverflow`, `TagMismatch`, `BadTagLength`); never panic on a caller-controlled length. `expect` is used only where the type makes failure impossible, with the reason as the message. Add any new variant here, not a second error type.

**Nonce and counter duty** (`crypto/src/page.rs` 65-96): `chacha20_xor(key, nonce, counter, buf)` takes both explicitly and returns `Err(CounterOverflow)` without writing. `session.rs` builds `nonce = prefix(4) || seq.to_le_bytes()` (D-08) and passes the block counter through.

**In-crate unit test for zeroization and raw derived bytes:** key bytes are unreadable outside the crate, so the byte-level assertions go in a `#[cfg(test)] mod tests` at the bottom of `session.rs` (the layout `ds/src/tpht/flattened.rs` 518-526 uses). `crypto/` has no in-crate test module today; this is the first.

**Registration:** add `pub mod session;` to `crypto/src/lib.rs` lines 19-22 (alphabetical: after `page`).

---

### `crypto/tests/session.rs` (test, transform)

**Analog:** `crypto/tests/keys.rs`

**Header and imports** (keys.rs:1-8): a `//!` header stating what is pinned and how keys are compared; import `crypto` only, never an upstream crate.

```rust
use crypto::key::{Key, derive_labelled, k_file};
```

**Fixed inputs and hex helper** (keys.rs:10-29): `k_user()` builds bytes `0x00..=0x1f`; `hex32` decodes a 64-digit literal.

**Known-answer pin with independent provenance** (keys.rs:31-49):

```rust
/// The known answer for `k_user()`, `KAT_PATH`, `KAT_VERS`, computed
/// independently of this crate (Python `hmac` + `hashlib`, HKDF-Expand per
/// RFC 5869 §2.3 with `info` = path little-endian then vers little-endian).
/// The command and its output are recorded in `evidence/PINS-03-04.md`
/// (`KAT k_file`).
const KAT_EXPECTED: &str = "ce90009aebce083580333aeaf1420104c25b12b6a1151dfe8fd5b2420cf10afe";

#[test]
fn k_file_known_answer() {
    let derived = k_file(&k_user(), KAT_PATH, KAT_VERS);
    assert!(
        derived.ct_eq(&Key::from_bytes(hex32(KAT_EXPECTED))),
        "K_file must equal the independently computed vector {KAT_EXPECTED}"
    );
}
```

The session pin copies this: expected bytes computed by a Python `hmac` / `hashlib` script, the command and output recorded in the phase's `evidence/PINS-04-NN.md`, compared through `Key::ct_eq`. The nonce prefix is not a `Key`; pin it through an observable (a keystream block or a MAC under the derived keys) or in the in-crate unit test.

**Separation pins** (keys.rs:51-79): the bit-flip loops over `path` and `vers` are the model for "the two directions' keys differ", "generation changes every key" (RESEARCH Pitfall 5) and "one transcript bit changes the keys".

---

### `port/src/chan/wire.rs` and `port/src/chan/msg.rs` (utility, transform)

**Analog:** `port/src/devmnt.rs`

**Imports** (devmnt.rs:75-86):

```rust
extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_MNT, DEV_PIPE, DEV_SSL, OMASK, ORDWR, OREAD, QTDIR};
use crate::mcslock::{BusyCell, Lock, LockNode};
use crate::Result;
```

**Opcode constants** (devmnt.rs:92-124): `const TVERSION: u8 = 100;` ... `const RSTAT: u8 = 125;`, plus `pub const NOTAG: u16 = 0xFFFF;`, `pub const NOFID: u32 = 0xFFFF_FFFF;`, `pub const VERSION9P: &[u8] = b"9P2000";`. These are private to `devmnt` and cover a client subset only (no `Tauth` 102/103, `Tflush` 108/109, `Twstat` 126/127). `chan` defines the full registry itself (100 to 127 with 106 rejected, 130 to 137, D-28); do not widen `devmnt`'s constants.

**Little-endian writers** (devmnt.rs:141-164):

```rust
fn pw16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pw32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn pw64(v: &mut Vec<u8>, x: u64) {
    v.extend_from_slice(&x.to_le_bytes());
}
/// convS2M's pstring: u16 length prefix + the bytes.
fn pwstr(v: &mut Vec<u8>, s: &[u8]) {
    pw16(v, s.len() as u16);
    v.extend_from_slice(s);
}
/// Wrap a body in the 9P header: size[4] type[1] tag[2] + body.
fn tmsg(typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(7 + body.len());
    pw32(&mut v, (7 + body.len()) as u32);
    v.push(typ);
    pw16(&mut v, tag);
    v.extend_from_slice(body);
    v
}
```

**Checked decode cursor that must consume the message exactly** (devmnt.rs:170-231). This is the decode pattern for every header field, TLV and message body: bounds-checked `get`, `from_le_bytes`, static error string, no panic, `done()` at the end.

```rust
struct RepCur<'a> {
    b: &'a [u8],
    p: usize,
}
    fn u32(&mut self) -> Result<u32> {
        let Some(s) = self.b.get(self.p..self.p + 4) else {
            return Err("devmnt: malformed reply (truncated)");
        };
        self.p += 4;
        Ok(u32::from_le_bytes(s.try_into().unwrap()))
    }
    /// convM2S's gqid: type[1] vers[4] path[8].
    fn qid(&mut self) -> Result<Qid> {
        let typ = self.u8()?;
        let vers = self.u32()?;
        let path = self.u64()?;
        Ok(Qid { path, vers, typ })
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let Some(s) = self.b.get(self.p..self.p + n) else {
            return Err("devmnt: malformed reply (count overruns the message)");
        };
        self.p += n;
        Ok(s)
    }
    /// The whole message must be consumed (convM2S's exact-size check).
    fn done(self) -> Result<()> {
        if self.p != self.b.len() {
            return Err("devmnt: malformed reply (trailing garbage)");
        }
        Ok(())
    }
```

The 13-byte `Qid` wire order (`type[1] vers[4] path[8]`, devmnt.rs:205-210) is the WIRE-10 codec. `dat::Qid` is `{ path: u64, vers: u32, typ: u8 }` (`port/src/dat.rs` 405-410) and `QTDIR = 0x80` (dat.rs:73). Under D-27 the volatility class never goes into `typ`.

**Framing: size bounds before the body is read** (devmnt.rs:499-534). The sealed receive path keeps this discipline with the lower bound changed from 7 to `HEADER_LEN + MAC_LEN` (28 + 32); `Size` is the only field read before MAC verification.

```rust
        let size = u32::from_le_bytes(hdr) as usize;
        // mntchk: the claimed size is untrusted until it passes
        // BOTH bounds — never read (or trust) a body the claim doesn't
        // cover, never read more than the negotiated msize.
        if size < 7 {
            println!("devmnt: runt reply message ({size} bytes)");
            return Err("devmnt: runt reply message");
        }
        if size > msize as usize {
            println!("devmnt: reply claims {size} bytes over msize {msize}");
            return Err("devmnt: reply exceeds negotiated msize");
        }
```

**Base message bodies** to copy byte for byte: `Tversion` (devmnt.rs:690-693), `Rversion` decode (697-703), `Tattach` (735-740), `Twrite` and `Rwrite` (863-876):

```rust
        let mut body = Vec::new();
        pw32(&mut body, fid);
        pw64(&mut body, off);
        pw32(&mut body, data.len() as u32);
        body.extend_from_slice(data);
        let msg = tmsg(TWRITE, tag, &body);
```

In the sealed dialect the `Twrite` / `Rread` / `Rstream` body becomes parameters, then data zero-padded to a multiple of 4096 with the true count in the parameters (D-09); the classic fixed fields above are the parameter block.

**Error convention:** `crate::Result<T>` is `core::result::Result<T, &'static str>` (`port/src/lib.rs` 151). Every `devmnt` error is a static string prefixed with the module name ("devmnt: ..."). RESEARCH's sketch returns `Result<Verified, Closed>`; if a typed `Closed` is used, keep `"chan: ..."` strings at the boundary where `chan` meets `devmnt`.

---

### `port/src/chan/tlv.rs` (utility, transform)

**Analog:** `port/src/devmnt.rs` `RepCur` (170-231), as above.

No TLV or bitmask-record parser exists in `port/`. Copy the cursor discipline (bounds-checked reads, exact consumption through `done()`, static error strings). The record rules (ascending type, each once, fixed `len` per type, zero padding, set of types equals the mask bits) are RESEARCH Pattern 3 and are a discretion proposal. `bitflags = "2.5"` is already a `port` dependency (`port/Cargo.toml` line 7) if a flags type is wanted for `ExtMask`; RESEARCH proposes plain `u16` constants.

---

### `port/src/chan/transport.rs` (service, streaming)

**Analog:** `port/src/devssl.rs` (the existing sealed record transport in `port/`: per-direction state, a per-record ordinal both ends count, MAC check before the payload is handed up).

**Per-direction state** (devssl.rs:158-213). Copy the split: one struct per direction owning that direction's secret material and counter, so a send and a receive never share state.

```rust
struct OneWay {
    secret: Vec<u8>,
    /// Donor `w->mid`: the per-record message id hashed into every
    /// digest — the replay/ordering ordinal both ends count in lockstep.
    mid: u32,
    rc4: Option<rc4::Rc4state>,
}

struct InDir {
    io: Option<Chan>,
    way: OneWay,
    processed: Vec<u8>,
    unprocessed: Vec<u8>,
}

struct OutDir {
    io: Option<Chan>,
    way: OneWay,
}
```

In `chan` the secret is a `crypto::session` per-direction key value, never a `Vec<u8>`, and the ordinal is the 64-bit wire sequence number.

**Receive: frame, check, then advance the ordinal** (devssl.rs:711-764). Structure to copy: `ensure` enough bytes, take the record, authenticate, only then stage plaintext and bump the counter:

```rust
            check_digest(dir, p.diglen, &body)?;
            body.drain(..p.diglen as usize);
            plen -= p.diglen as usize;
        ...
    dir.processed = body;
    dir.way.mid += 1;
    Ok(())
```

**Do not copy** devssl's digest comparison (devssl.rs:697): `if digest[..diglen] != body[..diglen]` is an early-exit slice compare, and devssl decrypts before it checks. `chan` verifies first, through `crypto::session` (constant-time), and on any failure zeroizes both directions and closes (WIRE-03); devssl only returns `Err("bad digest")`.

**Send: build the record, seal, write, advance** (devssl.rs:775-799):

```rust
    let io = dir.io.as_mut().ok_or("inappropriate use of fd")?;
    io_write(io, &rec)?;
    dir.way.mid += 1;
    Ok(())
```

D-21 requires sealing and sending under one per-direction lock; in devssl the direction's `BusyCell` guard is that lock (`claim_out`, devssl.rs:285).

**Lock primitives** (`port/src/mcslock.rs`): `Lock::new(name, data)` (line 114) and `lock(&node)` with a stack `LockNode::new()` (lines 26, 118) for brief, never-parking state; `BusyCell::new(data)` / `enter() -> Option<BusyGuard>` (lines 184, 192) for ownership that spans a blocking read. Usage shape (devmnt.rs:317-320):

```rust
        let node = LockNode::new();
        *self.reply.lock(&node) = Some(msg);
```

Do not add `spin`; do not write `unsafe impl Send/Sync` (the module is forbidden).

**Transport seam** (devmnt.rs:245-259). The engine reads and writes through the existing trait, so the same in-memory fakes serve it:

```rust
pub trait MntIo {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64>;
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64>;
    fn close_chan(&mut self) {}
}
```

`SessionExchange` (the D-16 seam trait) follows this shape: a small object-safe trait in `port`, implemented by test structs.

**Clock for the D-20 timeout.** RESEARCH F4 recommends sans-IO (`now: u64` passed in). The in-tree precedent for a clock a kernel caller registers later is the hook idiom (`port/src/devcap.rs` 109-123); nothing is wired this phase (D-23):

```rust
static TICK_HOOK: Lock<Option<fn() -> u64>> = Lock::new("captick", None);

pub fn set_tick_hook(hook: fn() -> u64) {
    let node = LockNode::new();
    let mut guard = TICK_HOOK.lock(&node);
    assert!(guard.is_none(), "devcap: tick hook registered twice");
    *guard = Some(hook);
}

fn ticks_now() -> u64 {
    let node = LockNode::new();
    TICK_HOOK.lock(&node).map_or(0, |f| f())
}
```

---

### `port/src/fid.rs` (model + store, CRUD)

**Analog:** `ds/src/tpht/flattened.rs` (the API being wrapped); no existing table is built on TPHT, so this is a partial match and RESEARCH Pattern 7 supplies the design.

**Shard construction: home-only, optional probe** (flattened.rs:159-167, 193-198):

```rust
    pub fn with_banks(num_blocks: usize, num_banks: usize) -> Result<Self, TphtError> {
        Self::with_banks_and_probe(num_blocks, num_banks, NoProbe)
    }
    pub fn with_banks_and_probe(
        num_blocks: usize,
        num_banks: usize,
        probe: P,
    ) -> Result<Self, TphtError> {
```

`num_banks == 0` gives a home-only shard that returns `TphtError::HomeFull` when a block's 16 slots are used. The type is `FlattenedTphtShard<8, P>` for a 64-byte `FidEntry` (two const/type parameters; the compendium's three-parameter form does not exist).

**Insert, and why shards must be home-only** (flattened.rs:250-255, 305, 315):

```rust
    pub fn insert(
        &self,
        key: u64,
        value: [u64; WORDS],
        same: impl Fn(&[u64; WORDS]) -> bool,
    ) -> Result<Option<TinyPtr>, TphtError> {
```

`Ok(None)` means the entry spilled and has no `TinyPtr`, so it cannot become a `StatefulHandle`. Errors: `TphtError::{BadBlockCount, Exists, HomeFull, BanksFull}` (`ds/src/tpht/mod.rs` 66-80).

**Resolve in one arena read** (flattened.rs:430-446): `deref_tiny(&self, key: u64, ptr: TinyPtr) -> [u64; WORDS]`. Its documented contract: "A stale pair returns whatever the slot now holds". `fid.rs` therefore validates `fid_key`, `client_id`, `epoch` and a `LIVE` flag after the read.

**`TinyPtr` is non-zero** (`ds/src/tpht/mod.rs` 84-97), which D-12 relies on:

```rust
pub struct TinyPtr(core::num::NonZeroU8);
    pub fn new(value: u8) -> Option<TinyPtr> {
        core::num::NonZeroU8::new(value).map(TinyPtr)
    }
    pub fn get(self) -> u8 {
        self.0.get()
    }
```

**Counting probe seam** (`ds/src/tpht/mod.rs` 52-64): `pub trait Probe { fn header_read(&self) {} fn arena_read(&self) {} }` and `pub struct NoProbe;`. `FidShard<P: Probe = NoProbe>` threads it through.

**Sweep set algebra** (`ds/src/roaring.rs` 181-189; `insert` 71, `remove` 99, `contains` 118, `iter` 136; values are `u32`):

```rust
    /// Difference `self \ other`.
    pub fn difference(&self, other: &Self) -> Self {
        self.combine(other, SetOp::Difference)
    }
    /// In place: `self` becomes `self \ other`.
    pub fn andnot(&mut self, other: &Self) {
        *self = self.difference(other);
    }
```

Bitmaps sit behind `crate::mcslock::Lock<RoaringBitmap>` (see transport section for the lock idiom).

**Fixed-width little-endian field packing** (`crypto/src/key.rs` 93-95) is the pattern for the 8-byte handle, 24-byte capability and `FidEntry` to `[u64; 8]` conversions, in safe code:

```rust
    let mut info = [0u8; 12];
    info[..8].copy_from_slice(&qid_path.to_le_bytes());
    info[8..].copy_from_slice(&qid_vers.to_le_bytes());
```

No `transmute`, no `repr(packed)`, no `unsafe impl` (compendium listing 9.6 is not copied; D-04).

**Capability tag:** computed and verified by `crypto` functions taking `&Key` (see `session.rs` above); the domain secret is a caller-supplied `crypto::key::Key` (D-13).

**Volatility class (D-27):** a separate one-byte type with values 0x20, 0x40, 0x80 and a mutual-exclusion check. Do not add these constants to `port/src/dat.rs` beside `QTDIR` (dat.rs:73-74), and do not OR them into `dat::Qid.typ`.

---

### `ds/src/tpht/flattened.rs` (modified: slot clearing, `slot_id`, `read_slot`)

**Analog:** the file itself. This is the one unsafe-holding `ds` module, so `cargo xtask miri` is re-run and the audit line reference re-measured (see `UNSAFE-AUDIT.md` below).

**Where the clear goes** (`remove`, flattened.rs:382-415). The header fields are cleared under the block seqlock; the arena slot is not written:

```rust
            header.tiny_ptrs[slot].store(0, Ordering::Release);
            header.quotients[slot].store(0, Ordering::Release);
            Self::clear_spilled(header, slot);
            header.occupied_mask.store(occupied & !(1u16 << slot), Ordering::Release);
            removed = true;
            break;
```

**How a slot is written under the lock** (flattened.rs:463-469), the call to reuse with a zero value:

```rust
    fn write_value(&self, slot: usize, value: &[u64; WORDS]) {
        for (word, out) in value.iter().enumerate() {
            self.arena[slot][word].store(*out, Ordering::Release);
        }
    }
```

**Slot arithmetic for `slot_id` / `read_slot`** (flattened.rs:448-461): `slot_index(block, ptr) = ((block << 4) + (ptr.get() as usize - 1)) & self.slot_mask`; `read_value(slot)` is the safe bounds-checked read (indexing `self.arena[slot]`), which is the body for a public `read_slot` returning `Option`. The private `split_key` (228-237) gives `block` from `key`.

**Test pattern** (flattened.rs:1161-1199): sizes through `miri_scaled(native, under_miri)` (`ds/src/tpht/mod.rs` 117-120); no test is gated out of Miri.

---

### `port/src/devmnt.rs` (modified: the `negotiate` branch)

**Analog:** the function itself, lines 662-716.

```rust
    pub fn negotiate(io_r: Box<dyn MntIo>, io_w: Box<dyn MntIo>, offer: u32) -> Result<Mnt> {
        let offer = if offer == 0 { DEFAULT_MSIZE } else { offer };
        if !(MSIZE_MIN..=MSIZE_MAX).contains(&offer) {
            return Err("devmnt: nonsense msize in fversion");
        }
        ...
        let mut body = Vec::new();
        pw32(&mut body, offer);
        pwstr(&mut body, VERSION9P);
        let msg = tmsg(TVERSION, NOTAG, &body);
        let reply = mnt.rpc(&msg, RVERSION, NOTAG)?;
        let (msize, version) = {
            let mut cur = RepCur::new(&reply[7..]);
            let msize = cur.u32()?;
            let version = cur.string()?;
            cur.done()?;
            (msize, version)
        };
        if msize > offer {
            return Err("devmnt: server tried to increase msize in fversion");
        }
        if !(MSIZE_MIN..=MSIZE_MAX).contains(&msize) {
            return Err("devmnt: nonsense msize in fversion");
        }
        if version != VERSION9P {
            return Err("devmnt: bad 9P version returned from server");
        }
        mnt.msize = msize;
        mnt.version = version;
        Ok(mnt)
    }
```

Constraints from the surrounding code:
- The signature `negotiate(io_r, io_w, offer) -> Result<Mnt>` stays exactly as is. Its one kernel caller is `fversion` (devmnt.rs:1022: `Arc::new(Mnt::negotiate(Box::new(io_r), Box::new(io_w), offer)?)`) and there are 14 call sites in `port/tests/devmnt_test.rs`. The general form is a new function that `negotiate` delegates to with the version fixed to `VERSION9P`.
- The exact `Tversion` (`msg`) and `Rversion` (`reply`) bytes are both in scope at lines 693-696; these are the first transcript bytes (D-17).
- The plain-path version check at line 710 stays; the new branch compares against the dialect string and treats a plain answer to an aead offer as an error (RESEARCH Pattern 8).
- `Mnt.version` is replayed by `fversion` (devmnt.rs:990-1002, "an established session answers from the stored negotiation"); the plain path's stored value must stay `b"9P2000"`.

---

### `port/src/lib.rs` (modified: module list)

**Analog:** the file itself. Add two lines in the pattern of lines 20-21, 46-47, each with the attribute:

```rust
#[forbid(unsafe_code)]
pub mod dat;
```

`chan` sits after `bitmapalloc`; `fid` after `fdt` (the list is roughly alphabetical). The header comment (lib.rs:10-19) states the rule: every module carries the forbid except `globalalloc`, `mcslock` and `devip_waker`. A forbid on a parent reaches its submodules (lib.rs:36-45 explains this for `devip`), which is wanted for `chan/`. Later-phase additions carry a short dated provenance comment above the attribute (lib.rs:54-76); follow that for `chan` and `fid`.

Crate-root test modules (`#[cfg(test)] mod devssl_tests;`, lib.rs:106-149) exist only for tests that need crate-private access. The Phase 4 pins are integration tests under `port/tests/`, so no new root test module is needed.

---

### `port/Cargo.toml` (modified: the dependency edge)

**Analog:** the file itself (lines 6-40) and `crypto/Cargo.toml` (provenance comment style, lines 7-17).

Path dependencies in this workspace are written `port = { path = "../port" }` (`aarch64/Cargo.toml` 12, `x86_64/Cargo.toml` 12, `riscv64/Cargo.toml` 10). Add under `[dependencies]`, with a comment in the existing style stating the phase, the decision (D-22, D-25) and that the lint contract's R4 changed in the same commit:

```toml
crypto = { path = "../crypto" }
ds = { path = "../ds" }
```

This edit and the R4 change in `xtask/src/lintcontract.rs` must be one commit: `cargo xtask test` runs the lint contract before any leg (`xtask/src/main.rs` 3577).

---

### `xtask/src/lintcontract.rs` (modified: rule R4 and fixtures)

**Analog:** the file itself.

**The rule** (lintcontract.rs:335-378). The kernel list and both message sites change so that `port` may name `crypto` and `ds` and the three arch crates may not:

```rust
fn check_kernel_dependencies(workspace: &Path, problems: &mut Vec<String>) {
    for kernel in ["port", "aarch64", "x86_64", "riscv64"] {
        let display = format!("{kernel}/Cargo.toml");
        ...
            let key = code.split('=').next().unwrap_or("").trim().trim_matches('"');
            if key == "crypto" || key == "ds" {
                problems.push(format!(
                    "{display}:{}: kernel crate dependency `{key}`; no kernel crate may depend on \
                     crypto/ or ds/ in Phase 3 (D-21). Phases 4 and 5 add this dependency under \
                     the kernel byte-change procedure and must change this rule then",
                    n + 1
                ));
            }
```

Also update the module header's R4 line (lintcontract.rs:18-19) and the summary message in `check` (lines 49-52), which name the Phase 3 decisions.

**Fixture and test pattern** (lintcontract.rs:526-562, 733-767). `fixture(name)` writes a minimal passing tree; each test overwrites the one file its rule reads and asserts on the error text:

```rust
    #[test]
    fn lintcontract_rejects_kernel_dependency() {
        for kernel in ["port", "aarch64", "x86_64", "riscv64"] {
            for dep in ["crypto", "ds"] {
                let dir = fixture(&format!("kernel-dep-{kernel}-{dep}"));
                write(
                    &dir,
                    &format!("{kernel}/Cargo.toml"),
                    &format!(
                        "[package]\nname = \"x\"\n\n[dependencies]\n{dep} = {{ path = \"../{dep}\" }}\n"
                    ),
                );
                let text = check(&dir)
                    .expect_err("a kernel crate depending on crypto or ds must be rejected")
                    .to_string();
```

Three things in this test change: the `port` case moves from rejected to accepted; the arch-crate cases stay rejected; and the trailing `kernel-dep-dev` case (lines 756-766, `port` with a `[dev-dependencies]` `ds`) needs a decision, since it currently asserts rejection. `lintcontract_workspace_passes` (573-577) checks the real tree and is the pin that the manifest and the rule agree.

---

### `xtask/src/test9p.rs` (build step, batch)

**Analog:** `xtask/src/miri.rs`

**Module shape** (miri.rs:1-37): a `//!` header saying what the step runs and why it is separate; `use crate::Result;`; a `CommandSpec` value type so the command is inspectable without spawning.

```rust
use std::path::{Path, PathBuf};

use crate::Result;

/// The exact command the step runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    ...
}
```

**Pure command builder** (miri.rs:92-108): `pub fn command_spec(...) -> CommandSpec` with no environment access and no spawn.

**Refusal when the filter would match nothing** (miri.rs:110-126):

```rust
    let tests = tpht_test_count(workspace)?;
    if tests == 0 {
        return Err("miri: no tests under ds/src/tpht (the filter would match nothing)".into());
    }
```

For `test-9p` the equivalent is refusing when any listed test file is missing. Errors carry the step-name prefix (`"miri: ..."`).

**Spawn** (miri.rs:134-144): `std::process::Command::new(&spec.program).args(&spec.args).current_dir(workspace)....status()`, mapping a spawn error and a non-zero status to prefixed errors. `miri.rs` uses the Miri toolchain's cargo; `test-9p` uses the ordinary one. The `cargo()` helper is private to `xtask/src/main.rs` (line 344), not in the library, so either the step struct in `main.rs` passes the program in or the module resolves `CARGO` itself.

**Unit tests** (miri.rs:147-257): `scratch(name)` temp tree, `write(root, rel, text)`, one test per behaviour, named with the step prefix (`miri_refuses_when_tpht_has_no_tests`, `miri_command_uses_the_toolchain_cargo`). These run under `cargo test -p xtask`, which `cargo xtask test` does not run.

**Soft-backend leg** (`xtask/src/main.rs` 3629-3641). The scalar-backend flags ride in the leg's own argument vector, never in `.cargo/config.toml` (lint rule R5):

```rust
        all_cmd_args.push((
            vec![
                "test".to_string(),
                "--package".to_string(),
                "crypto".to_string(),
                "--lib".to_string(),
                "--tests".to_string(),
                "--config".to_string(),
                "build.rustflags=[\"--cfg\",\"chacha20_backend=\\\"soft\\\"\",\"--cfg\",\"sha2_backend=\\\"soft\\\"\"]"
                    .to_string(),
            ],
            None,
        ));
```

**Existing legs already run the new files** (main.rs:3582-3604): `test --package port --lib --tests` and `test --package crypto --package ds --lib --tests`. The soft-backend leg covers `crypto` only; the `port` wire pins on the scalar backend exist only if `test-9p` adds that leg.

---

### `xtask/src/lib.rs` and `xtask/src/main.rs` (modified: registration)

**Analog:** the `miri` subcommand, four sites.

1. `xtask/src/lib.rs` line 24: `pub mod miri;` — add `pub mod test9p;`.
2. Subcommand definition (main.rs:217-232):

```rust
        .subcommand(
            clap::Command::new("miri")
                .about(
                    "Run Miri over ds/src/tpht with the second toolchain derivation \
                     (D-13): ... refuses when ds/src/tpht holds no \
                     #[test].  Not part of `cargo xtask test`: the ordinary run \
                     stays fast",
                )
                .args(&[clap::Arg::new("seeds")
                    .long("seeds")
                    ...
```

3. Dispatch arm (main.rs:332): `Some(("miri", m)) => MiriStep::new(m).run(),`
4. Step struct (main.rs:4928-4940):

```rust
struct MiriStep {
    seeds: u32,
}

impl MiriStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        Self { seeds: *matches.get_one::<u32>("seeds").unwrap_or(&64) }
    }

    fn run(self) -> Result<()> {
        xtask::miri::run(&workspace(), self.seeds)
    }
}
```

`workspace()` is at main.rs:4942-4944. The subcommand name is `test-9p` (kebab-case, as `rfork-header` and `ureg-check` at main.rs:330-331).

---

### `port/tests/common/aead_link.rs` and the five `port/tests/aead_*.rs` files (test)

**Analog:** `port/tests/devmnt_test.rs`

**Header and imports** (devmnt_test.rs:1-33): a `//!` header describing the rig and the knobs; `std` types; the crate under test through its public paths.

```rust
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use port::devmnt::{self, Mnt, MntIo};
use port::mcslock::{Lock, LockNode};
use port::Result;
```

**Test-side wire helpers built independently of the codec under test** (devmnt_test.rs:35-107): `pu8` / `pu16` / `pu32` / `pu64` / `pstr` / `pqid` / `frame` and a panicking `Cur` cursor. The comment states the reason: "built test-side so the codec under test is exercised through real bytes". The WIRE-01 hand-written header vector and the WIRE-06 registry table follow this rule; expected bytes are never produced by the encoder being tested.

```rust
fn pqid(v: &mut Vec<u8>, typ: u8, vers: u32, path: u64) {
    pu8(v, typ);
    pu32(v, vers);
    pu64(v, path);
}
/// Wrap a body in the 9P header: size[4] type[1] tag[2] + body.
fn frame(typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
```

**Scripted fake with malformation knobs and shared observation handles** (devmnt_test.rs:186-208):

```rust
struct FakeServer {
    outq: VecDeque<u8>,
    fids: Rc<RefCell<HashMap<u32, u64>>>, // fid → node qid path
    clunked: Rc<RefCell<Vec<u32>>>,
    server_msize: u32,
    /// Malformation knobs (T-05-13's rejection proofs):
    inflate_msize: bool, // answer msize HIGHER than the offer
    size_boost: u32,     // add to EVERY reply's size field (no bytes added)
    wrong_tag: bool,     // echo tag+1 on every reply
    replies: usize,
}
```

The aead link's knobs (flip byte N, replay last packet, reflect, alter `Rversion` msize, withhold the re-key ack) and its decode counter follow this shape: plain public fields set before the exchange, `Rc<RefCell<..>>` handles for what the test observes.

**In-memory link implementing the transport trait** (devmnt_test.rs:427-458):

```rust
impl MntIo for FakeHalf {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        let reply = self.0.borrow_mut().handle(buf);
        self.0.borrow_mut().outq.extend(reply);
        Ok(buf.len() as u64)
    }
    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        let n = {
            let mut f = self.0.borrow_mut();
            let n = f.outq.len().min(buf.len());
            for b in buf.iter_mut().take(n) {
                *b = f.outq.pop_front().unwrap();
            }
            n
        };
        Ok(n as u64)
    }
}

struct FakeHalf(Rc<RefCell<FakeServer>>);

impl FakeServer {
    fn split(self) -> (Box<FakeHalf>, Box<FakeHalf>) {
        let rc = Rc::new(RefCell::new(self));
        (Box::new(FakeHalf(Rc::clone(&rc))), Box::new(FakeHalf(rc)))
    }
}
```

**Session builder and negotiate pins** (devmnt_test.rs:462-467, 578-594), the model for `aead_negotiate.rs` and the "plain path unchanged" assertions:

```rust
fn session() -> (Mnt, Rc<RefCell<HashMap<u32, u64>>>) {
    let (fake, fids, _clunked) = FakeServer::new();
    let (r, w) = fake.split();
    let mnt = Mnt::negotiate(r, w, 0).expect("the version handshake negotiates");
    (mnt, fids)
}

#[test]
fn devmnt_rejects_server_msize_increase() {
    let (mut fake, _fids, _cl) = FakeServer::new();
    fake.inflate_msize = true; // the server answers HIGHER than the offer
    let (r, w) = fake.split();
    let err = Mnt::negotiate(r, w, 0).expect_err("a raised msize is a protocol error");
    assert!(err.contains("msize"), "the error names the msize violation: {err}");
}
```

Conventions to keep: test names prefixed with the module under test; every `expect` / `assert!` carries a sentence saying what must hold; rejection tests assert on the error text.

**Single-read proof for `aead_fid.rs`** (`ds/src/tpht/flattened.rs` 533-570 and 1161-1199). `CountingProbe` there is private to `ds`'s test module, so the `port` test defines its own against the public `ds::tpht::Probe` trait:

```rust
    #[derive(Clone, Default)]
    struct CountingProbe {
        header_reads: Arc<AtomicUsize>,
        arena_reads: Arc<AtomicUsize>,
    }
    impl Probe for CountingProbe {
        fn header_read(&self) {
            self.header_reads.fetch_add(1, Ordering::Relaxed);
        }
        ...
    }

        probe.reset();
        assert_eq!(shard.deref_tiny(0x5EED, ptr), [...], "...");
        assert_eq!(
            (probe.arena_reads(), probe.header_reads()),
            (1, 0),
            "deref_tiny must make exactly one arena read and zero header reads (DS-03)"
        );
```

**Shared fixture module:** `port/tests/` has no `common/` directory today (every existing test file is self-contained, and `devmnt_test.rs` keeps its fake in-file). A shared `port/tests/common/aead_link.rs` is the first; it needs `port/tests/common/mod.rs` with `pub mod aead_link;` and `mod common;` in each test file (or a `#[path]` attribute), otherwise Cargo builds it as its own test target.

---

### `UNSAFE-AUDIT.md` (modified)

**Analog:** the "Phase 3 delta" paragraph, lines 59-68.

```
**Phase 3 delta (crypto/ and ds/ join, 2026-11-24):** two workspace crates join
the contract (CONTEXT D-09, D-10, D-11, D-12).  `ds/` is per-module forbidden
with `ds/src/tpht/` as its one unsafe-holding module; it adds **1** lint-visible
block (`FlattenedTphtShard::deref_tiny`'s unchecked arena read,
`ds/src/tpht/flattened.rs:440`), **0** unsafe impls, **0** unsafe fns, **0**
unsafe extern blocks and **0** test-only blocks.  ...  Neither crate is linked
into a kernel in this phase, so the kernel-byte evidence is unchanged (D-21).
```

Write a dated "Phase 4 delta" paragraph in this form: counts per kind for each crate, the decisions cited, and the kernel-link statement, which now reverses (`port` links both crates). The `flattened.rs:440` reference is re-measured if that file is edited. The port module-states row at line 197 ("every other port module | unsafe-free, `#[forbid(unsafe_code)]`") already covers `chan` and `fid`; the sentence at line 28 ("no kernel crate depends on them in Phase 3") needs the matching update.

---

### Phase evidence files

**Analog:** `.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/`

`kernel-bytes/before.sha256` format (four lines, taken at the phase's base commit on one venue):

```
base 4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1
venue Darwin arm64
virtshell elf 9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e
q35churnshell elf c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b
```

Phase 3 kept `after-03-NN.sha256` per plan and a `KERNEL-BYTES.md` verdict page in the same directory. Under D-29 this phase's `KERNEL-BYTES.md` records the RESEARCH F1 evidence set (whole-ELF hashes, defined text symbol byte totals, FP/SIMD scan counts, x86_64 text total under 2,097,152), not a ".text identical" verdict.

Pin ledger format (`evidence/PINS-03-04.md` lines 10-27), one block per requirement, for D-03 / IG-V01:

```
PIN blake3-official-vectors RED
  command: nix develop -c cargo test -p crypto --test blake3_vectors
  result: test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
  failing line: ...
PIN blake3-official-vectors GREEN-1
  command: nix develop -c cargo xtask test (rc 0)
  result: ...
PIN blake3-official-vectors GREEN-2
  same command, second run (rc 0): ...
PIN blake3-official-vectors ADVANCED 83e5b26

KAT k_file command: python3 -c "import hmac, hashlib, struct; ..." => info=... k_file=...
  independent of the code under test: ...
```

The RED line records a behavioural failure (a wrong value), not a compile error: stubs with real signatures land first (RESEARCH Pitfall 11).

## Shared Patterns

### No key bytes outside `crypto/`
**Source:** `crypto/src/key.rs` 48-81 (private `Zeroizing` storage, `pub(crate) fn bytes`)
**Apply to:** `port/src/chan/*`, `port/src/fid.rs`, every `port/tests/aead_*.rs`
`port` holds `crypto::key::Key` and `crypto::session` values and calls methods on them. No `Key::expose`, no accessor returning a MAC key. Tests compare keys through `Key::ct_eq` or through observables.

### Untrusted wire input: bounds first, checked cursor, exact consumption, static error strings
**Source:** `port/src/devmnt.rs` 170-231 (`RepCur`), 499-534 (`frame_one`)
**Apply to:** `chan/wire.rs`, `chan/tlv.rs`, `chan/msg.rs`, `fid.rs` wire forms
```rust
        let Some(s) = self.b.get(self.p..self.p + n) else {
            return Err("devmnt: malformed reply (count overruns the message)");
        };
```
No indexing or `unwrap` on a length that came off the wire; the only `unwrap` is `try_into` on a slice whose length was just checked.

### Error types
**Source:** `port/src/lib.rs` 151 and `crypto/src/lib.rs` 29-43
**Apply to:** all new code
`port`: `pub type Result<T> = core::result::Result<T, &'static str>;` with `"<module>: ..."` strings. `crypto`: `Result<T, crypto::Error>` with a `Copy` enum. `ds`: `TphtError` (`ds/src/tpht/mod.rs` 66-80). Convert at the crate boundary; do not leak `crypto::Error` or `TphtError` through `devmnt`'s `Result`.

### Constant-time comparison
**Source:** `crypto/src/mac.rs` 101-108; `crypto/src/key.rs` 68-74
**Apply to:** packet MAC verify, capability tag verify, the re-key `confirm` value
Accumulator fold with no early exit, inside `crypto/`. Never `==` on tag slices (devssl.rs:697 is the counter-example, not a model).

### Per-module `forbid(unsafe_code)`
**Source:** `port/src/lib.rs` 10-21; `crypto/src/lib.rs` 2; `ds/src/lib.rs` 13-32
**Apply to:** `port::chan` (and submodules), `port::fid`, `crypto::session`
`crypto` is crate-level forbidden, so `session.rs` is covered with no attribute. In `port`, the attribute goes on the `pub mod` line in `lib.rs`. A new unsafe-holding `port` module needs owner approval (IG-D07); none is proposed.

### Module documentation headers
**Source:** `crypto/src/key.rs` 1-39, `crypto/src/page.rs` 1-25, `port/src/devmnt.rs` 1-73, `xtask/src/miri.rs` 1-17
**Apply to:** every new source file
A `//!` block stating the requirement and decision IDs, the layout or contract, what is pinned and where, and what the module deliberately does not decide. Function docs state the contract and the failure behaviour; inline comments cite the decision or threat ID that motivates a check.

### Thin wrappers over pinned crates
**Source:** `crypto/src/kdf.rs` 29-36, 48-53, 60-78; `crypto/src/mac.rs` 31-43
**Apply to:** `crypto/src/session.rs`
Public signatures take and return r9's own arrays, slices and `crate::Result`; upstream types never appear in a signature (D-18 of Phase 3).

### Locks
**Source:** `port/src/mcslock.rs` 100-128 (`Lock`), 170-192 (`BusyCell`); usage `port/src/devmnt.rs` 310-326, 477-489
**Apply to:** `chan/transport.rs` (per-direction send lock, D-21), `fid.rs` (bitmaps)
`Lock::new("name", data)` with `let node = LockNode::new(); let mut g = x.lock(&node);` for short sections; `BusyCell` where ownership spans a blocking read.

### Dated amendment notes in planning documents (D-10)
**Source:** `.planning/rfc-9p2000-aead.md` line 70; `.planning/PROJECT.md` lines 100, 124
**Apply to:** the RFC, PROJECT.md, STATE.md, REQUIREMENTS.md, compendium listing 9.9
RFC form: a paragraph beginning `Amended 2026-11-24 (r9 Phase 4, decisions D-NN to D-NN):` that quotes the previous wording and states the operative reading. PROJECT.md form: `**Settled ... on 2026-11-24** (Phase 4 CONTEXT D-NN): ...` appended to the open item, and a Key Decisions table row ending `| Locked amended 2026-11-24 (owner) |`. A superseded decision gets a new dated row, never a silent edit (PROJECT.md line 114). The IG-D11 lock row is written only after the owner's checkpoint answer.

### RED, GREEN twice, pin advance (D-03)
**Source:** `.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/evidence/PINS-03-04.md`
**Apply to:** every WIRE requirement

## No Analog Found

| File | Role | Data flow | Reason |
|------|------|-----------|--------|
| `port/src/chan/transport.rs` (re-key state machine only: marker, ack with `confirm`, deadline, fail-closed zeroize of both directions) | service | event-driven | `devssl` has per-direction ordinals and a digest check but no re-keying, no deadline and no zeroize-on-failure. Use RESEARCH Patterns 2 and 4 and CONTEXT D-18 to D-21. |
| `port/src/fid.rs` (`FidTable` / `FidShard` composition, shard fall-over on `HomeFull`, sweep) | store | CRUD | Nothing at HEAD composes `FlattenedTphtShard` with a `RoaringBitmap`; `ds/src/clientset.rs` uses Roaring alone. Use RESEARCH Pattern 7 with the `ds` API excerpts above. |

Partial gaps inside files that do have an analog:
- `port/src/chan/tlv.rs`: no TLV or aligned-record parser exists; only the cursor discipline transfers.
- `Unverified` / `Verified` packet typestate (`chan/wire.rs`): the nearest precedent is the `KeyCapability<ReadWrite>` / `KeyCapability<ReadOnly>` typestate with a `compile_fail` doctest (`crypto/src/key.rs` 148-236), which shows how this repo proves "this call does not compile". Doctests run in their own leg (`test --package crypto --doc`, `xtask/src/main.rs` 3610-3618); there is no `port --doc` leg today.
- `port/tests/common/`: no shared integration-test module exists in any crate.

## Metadata

**Analog search scope:** `port/src/`, `port/tests/`, `crypto/src/`, `crypto/tests/`, `ds/src/`, `xtask/src/`, `UNSAFE-AUDIT.md`, `flake.nix`, Phase 2 and Phase 3 evidence directories
**Files scanned:** 24 read in full or in targeted ranges
**Knowledge graph:** `graphify query` was tried first; the graph is flagged stale and returned no useful transport or codec nodes, so analogs were located from the file lists in CONTEXT and RESEARCH and by direct outline. The graph was not refreshed (read-only task).
**Project instructions:** no `./CLAUDE.md`; no `.claude/skills` or `.agents/skills` in the repository
**Pattern extraction date:** 2026-11-24
