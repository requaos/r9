//! transport — sealing and opening one packet (WIRE-01, WIRE-03, WIRE-05;
//! D-06, D-07, D-08).
//!
//! The tracer's two functions from plan 04-02 live here, re-exported by
//! `port::chan`, so `port/tests/aead_transport.rs` compiles unchanged.  This
//! plan's GREEN step re-expresses them through the canonical codec in
//! [`super::wire`].
//!
//! The transport state machine, strict sequencing, re-keying and teardown are
//! later plans in this phase (D-22).

use alloc::vec::Vec;

use crypto::session::DirectionKeys;

use super::{HEADER_LEN, MAC_LEN, OFF_FIDHANDLE, OFF_SEQ, OFF_SIZE, OFF_TAG, OFF_TYPE};

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
        packet[OFF_SIZE..OFF_SIZE + 4].try_into().expect("the Size slice is exactly 4 bytes"),
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
    let tag: [u8; MAC_LEN] =
        packet[size - MAC_LEN..].try_into().expect("the tag slice is exactly MAC_LEN bytes");
    mac.verify(&tag).map_err(|_| "chan: packet MAC does not verify")?;

    // Only now read the sequence number and compare it (WIRE-04).
    let seq = u64::from_le_bytes(
        packet[OFF_SEQ..OFF_SEQ + 8].try_into().expect("the SeqNonce slice is exactly 8 bytes"),
    );
    if seq != expected_seq {
        return Err("chan: unexpected sequence number");
    }

    let typ = packet[OFF_TYPE];
    let tag_field = u16::from_le_bytes(
        packet[OFF_TAG..OFF_TAG + 2].try_into().expect("the Tag slice is exactly 2 bytes"),
    );
    let fid_handle: [u8; 8] = packet[OFF_FIDHANDLE..OFF_FIDHANDLE + 8]
        .try_into()
        .expect("the FidHandle slice is exactly 8 bytes");

    let mut body = packet[HEADER_LEN..size - MAC_LEN].to_vec();
    keys.keystream(seq, 0, &mut body)
        .map_err(|_| "chan: keystream counter overflow")?;
    Ok((typ, tag_field, fid_handle, body))
}
