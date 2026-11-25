//! chan — the `9P2000.aead` envelope: the fixed header, the packet MAC and
//! the sealed body (WIRE-01, WIRE-03, WIRE-05; D-06, D-07, D-08, D-22).
//!
//! # The header (D-06)
//!
//! 28 bytes, in the RFC section 4.1 order and all multi-byte fields little
//! endian: `Size(4) Type(1) Tag(2) Pad(1) FidHandle(8) SeqNonce(8)
//! ExtMask(2) ExtTLVs Length(2)`.  The field list is the decision; the figure
//! "24" it was first written with was an arithmetic slip.  The offsets below
//! pin every field, and [`HEADER_LEN`] pins the total.
//!
//! # What the MAC covers (D-07)
//!
//! Every byte of the packet before the MAC, as one contiguous slice from
//! byte 0 of `Size` to the last ciphertext byte.  `Pad` and `ExtTLVs Length`
//! are therefore authenticated, and [`open_packet`] verifies before it acts
//! on any field but `Size` (which it reads only to bound the packet).
//!
//! # What is not here yet
//!
//! This is the thinnest real path: one sealed packet, one open.  TLVs,
//! message bodies, the transport state machine and re-keying are later plans
//! in this phase (D-22); `port/src/fid.rs` is plan 04-09's.

extern crate alloc;

use alloc::vec::Vec;

use crypto::session::DirectionKeys;

/// The fixed envelope header length (D-06).
pub const HEADER_LEN: usize = 28;

/// The packet MAC length in bytes (D-01, D-06).
pub const MAC_LEN: usize = 32;

/// The dialect version string (D-17).
pub const VERSION_AEAD: &[u8] = b"9P2000.aead";

/// `Size` offset: the whole packet length, little-endian (D-06).
pub const OFF_SIZE: usize = 0;
/// `Type` offset: the 9P message type (D-06).
pub const OFF_TYPE: usize = 4;
/// `Tag` offset: the 9P tag, little-endian (D-06).
pub const OFF_TAG: usize = 5;
/// `Pad` offset: must be zero (D-06).
pub const OFF_PAD: usize = 7;
/// `FidHandle` offset: the 8-byte handle (D-06).
pub const OFF_FIDHANDLE: usize = 8;
/// `SeqNonce` offset: the per-direction sequence number (D-06, D-08).
pub const OFF_SEQ: usize = 16;
/// `ExtMask` offset: the extension bitmask (D-06).
pub const OFF_EXTMASK: usize = 24;
/// `ExtTLVs Length` offset: the TLV area length (D-06).
pub const OFF_EXTLEN: usize = 26;

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
    let size = HEADER_LEN + body.len() + MAC_LEN;
    if size > u32::MAX as usize {
        return Err("chan: packet exceeds the 32-bit size field");
    }
    let mut packet = alloc::vec![0u8; size];
    packet[OFF_SIZE..OFF_SIZE + 4].copy_from_slice(&(size as u32).to_le_bytes());
    packet[OFF_TYPE] = typ;
    packet[OFF_TAG..OFF_TAG + 2].copy_from_slice(&tag.to_le_bytes());
    // Pad is zero (D-06); the buffer was zero-filled, so it is already set.
    packet[OFF_FIDHANDLE..OFF_FIDHANDLE + 8].copy_from_slice(&fid_handle);
    packet[OFF_SEQ..OFF_SEQ + 8].copy_from_slice(&seq.to_le_bytes());
    // ExtMask and ExtTLVs Length are zero: no TLV travels in this plan.
    packet[HEADER_LEN..HEADER_LEN + body.len()].copy_from_slice(body);

    keys.keystream(seq, 0, &mut packet[HEADER_LEN..HEADER_LEN + body.len()])
        .map_err(|_| "chan: keystream counter overflow")?;

    let mut mac = keys.mac();
    mac.update(&packet[..size - MAC_LEN]);
    let tag_bytes = mac.finalize();
    packet[size - MAC_LEN..].copy_from_slice(&tag_bytes);
    Ok(packet)
}

/// Open one packet: bounds, MAC, sequence, then decrypt (D-07, WIRE-03).
///
/// `Size` is the only field read before the MAC verifies, and only to bound
/// the packet; every other field is acted on only after verification
/// (T-04-04).  The returned body is the decrypted plaintext.
pub fn open_packet(
    keys: &DirectionKeys,
    expected_seq: u64,
    msize: u32,
    packet: &[u8],
) -> crate::Result<(u8, u16, [u8; 8], Vec<u8>)> {
    if packet.len() < HEADER_LEN + MAC_LEN {
        return Err("chan: runt packet");
    }
    let size = u32::from_le_bytes(
        packet[OFF_SIZE..OFF_SIZE + 4]
            .try_into()
            .expect("the Size slice is exactly 4 bytes"),
    ) as usize;
    // mntchk's discipline: both bounds before any body is trusted.
    if size < HEADER_LEN + MAC_LEN {
        return Err("chan: packet shorter than the header and MAC");
    }
    if size > msize as usize {
        return Err("chan: packet exceeds the negotiated msize");
    }
    if size != packet.len() {
        return Err("chan: packet length disagrees with its size field");
    }

    // Verify before acting on any field but Size (D-07).
    let mut mac = keys.mac();
    mac.update(&packet[..size - MAC_LEN]);
    let tag: [u8; MAC_LEN] = packet[size - MAC_LEN..]
        .try_into()
        .expect("the tag slice is exactly MAC_LEN bytes");
    mac.verify(&tag).map_err(|_| "chan: packet MAC does not verify")?;

    // Only now read the sequence number and compare it (WIRE-04).
    let seq = u64::from_le_bytes(
        packet[OFF_SEQ..OFF_SEQ + 8]
            .try_into()
            .expect("the SeqNonce slice is exactly 8 bytes"),
    );
    if seq != expected_seq {
        return Err("chan: unexpected sequence number");
    }

    let typ = packet[OFF_TYPE];
    let tag_field = u16::from_le_bytes(
        packet[OFF_TAG..OFF_TAG + 2]
            .try_into()
            .expect("the Tag slice is exactly 2 bytes"),
    );
    let fid_handle: [u8; 8] = packet[OFF_FIDHANDLE..OFF_FIDHANDLE + 8]
        .try_into()
        .expect("the FidHandle slice is exactly 8 bytes");

    let mut body = packet[HEADER_LEN..size - MAC_LEN].to_vec();
    keys.keystream(seq, 0, &mut body)
        .map_err(|_| "chan: keystream counter overflow")?;
    Ok((typ, tag_field, fid_handle, body))
}
