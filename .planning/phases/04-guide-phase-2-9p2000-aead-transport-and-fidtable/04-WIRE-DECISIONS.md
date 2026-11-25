# Phase 4 wire decisions

This file records the eight wire-fixing proposals research made under Claude's Discretion (P1 to
P8) and the owner's answers to them, plus the owner's answer on the IG-D11 lock. Plans 04-02 to
04-13 read this file before they pin any wire byte: where a `DECISION Pn` value differs from the
`PROPOSAL Pn` value it replaces it everywhere, and where the two agree the proposal value stands.
The proposals were drafted in plan 04-01 Task 1 and answered by the owner in Task 2 (P1 to P8) and
Task 3 (IG-D11).

PROPOSAL P1: sixth `ExtMask` bit and TLV record: `EXT_STATELESS_CAP = 1 << 5`; record is `type[1]` (the bit index 0 to 5), `reserved[1]` zero, `len[2]` little-endian value length unpadded, value, zero padding to a multiple of 8; ascending type order, each type at most once, set of types equals the mask bits, fixed `len` per type (8, 32, 10, 4, 0, 24), any unknown bit or broken rule closes the connection (Pattern 3).
PROPOSAL P2: session derivation: `prk = HKDF-Extract(salt = transcript, ikm = secret)`; `info` is 18 bytes: `"9P2000.aead"` (11) then `0x00`, direction (`0x01` client to server, `0x02` server to client), purpose (`0x01` cipher key 32 bytes, `0x02` MAC key 32 bytes, `0x03` nonce prefix 4 bytes), generation as 4 bytes little-endian; transcript is unkeyed BLAKE3 over every plain-framed handshake message whole, in wire order; across a re-key the next transcript is unkeyed BLAKE3 of the previous transcript followed by the exchange messages of that re-key only (the in-band `Tauth`, `Rauth` and every afid `Twrite`, `Rwrite`, `Tread` and `Rread`), each re-framed in its standard 9P2000 form, whole, in the order they cross the wire: `size[4] type[1] tag[2]` then the standard body, with `size` counting those bytes and the data unpadded (`Twrite` body `fid[4] offset[8] count[4] data[count]`, `Rread` body `count[4] data[count]`); the sealed bytes, the 28-byte header, the TLVs, the page padding and the MAC are not hashed, and neither is `Trekey` or `Rrekey` of any kind nor any other traffic sealed during the exchange (Pattern 5; the form of the re-key exchange messages is not in research, it was stated in planning and is the form plan 04-08 pins and plan 04-13 writes into RFC section 3.2).
PROPOSAL P3: re-key messages: opcodes `Trekey = 138` (client) and `Rrekey = 139` (server), tag `NOTAG`, zero `FidHandle`, body `kind[1] generation[4] confirm[32]` with kind 0 REQUEST, 1 MARKER, 2 ACK; `confirm` is keyed BLAKE3 under the new MAC key of the direction cut over, over `"r9 rekey ack"` then the generation; defaults `REKEY_AFTER_PACKETS = 1 << 48`, `REKEY_ACK_TIMEOUT_MS = 5000`, sequence `u64::MAX` never sent (Pattern 4).
PROPOSAL P4: message bodies: base messages keep their 9P2000 bodies byte for byte; the header `FidHandle` carries the server-issued handle of the message's primary fid and R-messages that create a fid return the new handle there; the four new pairs use the RFC section 6.2 formats minus `size[4] type tag[2]`; data messages are parameters then zero-padded 4096-byte pages with the true count in the parameters (Pattern 6).
PROPOSAL P5: keystream placement: parameters and non-data bodies from ChaCha20 block counter 0; data page `i` from block counter `1 + 64 * i`; parameter blocks fit one 64-byte block (Pattern 6).
PROPOSAL P6: `blake3` `zeroize` feature: off; the limit (a per-packet hasher holds a key copy that is dropped unscrubbed) is stated in the `session.rs` module doc (Pitfall 6).
PROPOSAL P7: sequence violation: a packet whose sequence number is not the expected next value is dropped and the connection is closed and zeroized, not dropped alone (Pattern 2, A6).
PROPOSAL P8: `StatelessCap` tag: first 8 bytes of keyed BLAKE3 under the domain secret over the label `r9 StatelessCap1` then the capability's first 16 bytes; `epoch` is the table's current epoch; a capability that fails verification closes the connection (Pattern 7, A11).

DECISION P1: accepted (2026-11-25)
DECISION P2: accepted (2026-11-25)
DECISION P3: accepted (2026-11-25)
DECISION P4: accepted (2026-11-25)
DECISION P5: accepted (2026-11-25)
DECISION P6: accepted (2026-11-25)
DECISION P7: accepted (2026-11-25)
DECISION P8: accepted (2026-11-25)
DECISION IG-D11: lock-amended (2026-11-25) — locked with the proposed amended wording, no wording changes
