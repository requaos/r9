//! transport — sealing and opening one packet (WIRE-01, WIRE-03, WIRE-05;
//! D-06, D-07, D-08).
//!
//! The tracer's two functions from plan 04-02 live here, re-exported by
//! `port::chan`, so `port/tests/aead_transport.rs` compiles unchanged.  They
//! are expressed through the canonical codec in [`super::wire`]: `seal_packet`
//! writes the header with `Header::encode` and sizes the packet with
//! `packet_len`; `open_packet` bounds the packet with `frame_size` and reads
//! the header with `Header::decode` only after the MAC verifies.
//!
//! The transport state machine, strict sequencing, re-keying and teardown are
//! later plans in this phase (D-22).

use alloc::vec::Vec;

use crypto::session::DirectionKeys;

use super::wire::{Header, Unverified, Verified, check_header, packet_len};
use super::{HEADER_LEN, MAC_LEN};

/// Seal one packet: header, encrypted body, MAC (D-06, D-07).
///
/// The header is written field by field at the pinned offsets, the body is
/// encrypted in place from block counter 0, and the MAC covers the one
/// contiguous slice `[0, size - MAC_LEN)` and is appended last.
pub fn seal_packet(
    keys: &DirectionKeys,
    seq: u64,
    typ: u8,
    tag: u16,
    fid_handle: [u8; 8],
    body: &[u8],
) -> crate::Result<Vec<u8>> {
    // No TLV area travels in this plan, and no negotiated msize is known at
    // this layer, so only the 32-bit `Size` bound applies here.
    let size = packet_len(0, body.len(), u32::MAX)? as usize;
    let header = Header {
        size: size as u32,
        typ,
        tag,
        pad: 0,
        fid_handle,
        seq,
        ext_mask: 0,
        ext_len: 0,
    };
    let mut packet = alloc::vec![0u8; size];
    header.encode((&mut packet[..HEADER_LEN]).try_into().expect("the header slice is 28 bytes"));
    packet[HEADER_LEN..HEADER_LEN + body.len()].copy_from_slice(body);

    keys.keystream(seq, 0, &mut packet[HEADER_LEN..HEADER_LEN + body.len()])
        .map_err(|_| "chan: keystream counter overflow")?;

    let mut mac = keys.mac();
    mac.update(&packet[..size - MAC_LEN]);
    let tag_bytes = mac.finalize();
    packet[size - MAC_LEN..].copy_from_slice(&tag_bytes);
    Ok(packet)
}

/// Open one packet: bounds, MAC, sequence, header rules, then decrypt
/// (D-07, WIRE-03).
///
/// `Size` is the only field read before the MAC verifies, and only to bound
/// the packet; every other field is acted on only after verification
/// (T-04-11, T-04-12).  The returned body is the decrypted plaintext.
pub fn open_packet(
    keys: &DirectionKeys,
    expected_seq: u64,
    msize: u32,
    packet: &[u8],
) -> crate::Result<(u8, u16, [u8; 8], Vec<u8>)> {
    // The receive path decrypts in place and the public signature is
    // read-only (the tracer passes a shared slice), so one copy is made
    // here.  It is not a second parse: `Unverified::new` applies the bounds
    // to the copy first, exactly as `frame_one` does for plain 9P.
    let mut buf = packet.to_vec();
    let unverified = Unverified::new(&mut buf, msize)?;
    let size = unverified.len();

    // Verify before acting on any field but Size (D-07).
    let mut mac = keys.mac();
    mac.update(&buf[..size - MAC_LEN]);
    let tag: [u8; MAC_LEN] =
        buf[size - MAC_LEN..].try_into().expect("the tag slice is exactly MAC_LEN bytes");
    mac.verify(&tag).map_err(|_| "chan: packet MAC does not verify")?;

    // Only now read the header, check every field, and compare the sequence
    // number (RESEARCH Pattern 2 steps 3 and 4; WIRE-04).
    let header = Header::decode(
        (&buf[..HEADER_LEN]).try_into().expect("the header slice is exactly HEADER_LEN bytes"),
    );
    if header.seq != expected_seq {
        return Err("chan: unexpected sequence number");
    }
    check_header(&header, size)?;

    // Decrypt the body only: the TLV area travels cleartext and is covered by
    // the MAC.  Then hand the split packet to the decoder (WIRE-03).
    let body_start = HEADER_LEN + header.ext_len as usize;
    keys.keystream(header.seq, 0, &mut buf[body_start..size - MAC_LEN])
        .map_err(|_| "chan: keystream counter overflow")?;
    let verified = Verified::new_verified(header, &buf)?;
    Ok((header.typ, header.tag, header.fid_handle, verified.body().to_vec()))
}
