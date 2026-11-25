//! msg — the base 9P2000 message body codecs (WIRE-06, D-28, P4).
//!
//! The base bodies are the 9P2000 bodies unchanged (`Harvey` `convS2M`), so
//! the `fid[4]` in a body is the client-scoped fid number; the server-issued
//! handle travels in the envelope header (P4).  Every multi-byte field is
//! little-endian.  Strings are a 16-bit little-endian byte length followed by
//! raw bytes, with no UTF-8 validation or normalization (as in `devmnt`).
//!
//! The decoder uses a private checked cursor and requires exact consumption;
//! every error is a static `"chan: ..."` string and no wire length is trusted
//! (T-04-14).
//!
//! The new pairs 130 to 139 are encode and decode only: their semantics are
//! Phase 6 and nothing here may imply they work.  The three data messages
//! (`Twrite`, `Rread`, `Rstream`) carry the D-09 two-part body: a parameter
//! block, then file data padded with zeros to a whole number of
//! [`PAGE_LEN`](super::PAGE_LEN)-byte pages, with the true length in the
//! parameters.  Parameters encrypt from ChaCha20 block counter 0 and data page
//! `i` from `1 + 64 * i` (P5); every parameter block fits one 64-byte block.

use alloc::vec::Vec;

use crate::dat::Qid;

use super::{PAGE_LEN, wire};

/// The 9P2000 maximum number of path elements in a `Twalk` / `Rwalk`
/// (`fcall.h` `MAXWELEM`).
pub const MAXWELEM: usize = 16;

/// The largest parameter block of a D-09 data body: one ChaCha20 block (P5).
pub const MAX_PARAMS_LEN: usize = 64;

/// Re-key request kind (P3).
pub const REKEY_REQUEST: u8 = 0;
/// Re-key cutover marker kind (P3).
pub const REKEY_MARKER: u8 = 1;
/// Re-key acknowledgement kind (P3).
pub const REKEY_ACK: u8 = 2;

/// One 9P message body, in the `fcall.h` field order.
///
/// `Qid` is `crate::dat::Qid`; equality is implemented by hand because that
/// type predates this module and does not derive `PartialEq` (`dat.rs` is
/// read-only to this plan).
#[derive(Clone, Debug)]
pub enum Msg {
    // ---- base 9P2000 set, 100 to 127 (D-28) ------------------------------
    Tversion {
        msize: u32,
        version: Vec<u8>,
    },
    Rversion {
        msize: u32,
        version: Vec<u8>,
    },
    Tauth {
        afid: u32,
        uname: Vec<u8>,
        aname: Vec<u8>,
    },
    Rauth {
        aqid: Qid,
    },
    Tattach {
        fid: u32,
        afid: u32,
        uname: Vec<u8>,
        aname: Vec<u8>,
    },
    Rattach {
        qid: Qid,
    },
    Rerror {
        ename: Vec<u8>,
    },
    Tflush {
        oldtag: u16,
    },
    Rflush,
    Twalk {
        fid: u32,
        newfid: u32,
        wname: Vec<Vec<u8>>,
    },
    Rwalk {
        wqid: Vec<Qid>,
    },
    Topen {
        fid: u32,
        mode: u8,
    },
    Ropen {
        qid: Qid,
        iounit: u32,
    },
    Tcreate {
        fid: u32,
        name: Vec<u8>,
        perm: u32,
        mode: u8,
    },
    Rcreate {
        qid: Qid,
        iounit: u32,
    },
    Tread {
        fid: u32,
        offset: u64,
        count: u32,
    },
    Rread {
        data: Vec<u8>,
    },
    Twrite {
        fid: u32,
        offset: u64,
        data: Vec<u8>,
    },
    Rwrite {
        count: u32,
    },
    Tclunk {
        fid: u32,
    },
    Rclunk,
    Tremove {
        fid: u32,
    },
    Rremove,
    Tstat {
        fid: u32,
    },
    Rstat {
        stat: Vec<u8>,
    },
    Twstat {
        fid: u32,
        stat: Vec<u8>,
    },
    Rwstat,

    // ---- new pairs 130 to 139 (WIRE-06; RFC section 6.2, P3) -------------
    //
    // These are encode and decode only: their semantics are Phase 6 and
    // nothing here may imply they work.  The leading `size[4] type tag[2]` is
    // the envelope's; a `fid_handle[8]` field stays in the body as written.
    Tcompound {
        num_ops: u16,
        op_stream: Vec<u8>,
    },
    Rcompound {
        num_completed: u16,
        status: u16,
        result_stream: Vec<u8>,
    },
    Tstream {
        fid_handle: [u8; 8],
        stream_id: u32,
        flags: u16,
        offset: u64,
    },
    Rstream {
        stream_id: u32,
        chunk_seq: u64,
        data: Vec<u8>,
    },
    TleaseRevoke {
        qid_path: u64,
        lease_gen: u64,
        target_mode: u8,
        num_blocks: u32,
        block_ranges: Vec<u8>,
    },
    RleaseAck {
        qid_path: u64,
        lease_gen: u64,
        client_status: u8,
    },
    Treclaim {
        old_handle: [u8; 8],
        target_epoch: u16,
        client_token: [u8; 32],
    },
    Rreclaim {
        new_handle: [u8; 8],
        recovery_status: u16,
    },
    Trekey {
        kind: u8,
        generation: u32,
        confirm: [u8; 32],
    },
    Rrekey {
        kind: u8,
        generation: u32,
        confirm: [u8; 32],
    },
}

impl PartialEq for Msg {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Msg::Tversion {
                    msize: am,
                    version: av,
                },
                Msg::Tversion {
                    msize: bm,
                    version: bv,
                },
            ) => am == bm && av == bv,
            (
                Msg::Rversion {
                    msize: am,
                    version: av,
                },
                Msg::Rversion {
                    msize: bm,
                    version: bv,
                },
            ) => am == bm && av == bv,
            (
                Msg::Tauth {
                    afid: aa,
                    uname: au,
                    aname: an,
                },
                Msg::Tauth {
                    afid: ba,
                    uname: bu,
                    aname: bn,
                },
            ) => aa == ba && au == bu && an == bn,
            (Msg::Rauth { aqid: a }, Msg::Rauth { aqid: b }) => qid_eq(a, b),
            (
                Msg::Tattach {
                    fid: af,
                    afid: aa,
                    uname: au,
                    aname: an,
                },
                Msg::Tattach {
                    fid: bf,
                    afid: ba,
                    uname: bu,
                    aname: bn,
                },
            ) => af == bf && aa == ba && au == bu && an == bn,
            (Msg::Rattach { qid: a }, Msg::Rattach { qid: b }) => qid_eq(a, b),
            (Msg::Rerror { ename: a }, Msg::Rerror { ename: b }) => a == b,
            (Msg::Tflush { oldtag: a }, Msg::Tflush { oldtag: b }) => a == b,
            (Msg::Rflush, Msg::Rflush) => true,
            (
                Msg::Twalk {
                    fid: af,
                    newfid: an,
                    wname: aw,
                },
                Msg::Twalk {
                    fid: bf,
                    newfid: bn,
                    wname: bw,
                },
            ) => af == bf && an == bn && aw == bw,
            (Msg::Rwalk { wqid: a }, Msg::Rwalk { wqid: b }) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| qid_eq(x, y))
            }
            (
                Msg::Topen { fid: af, mode: am },
                Msg::Topen { fid: bf, mode: bm },
            ) => af == bf && am == bm,
            (
                Msg::Ropen {
                    qid: aq,
                    iounit: ai,
                },
                Msg::Ropen {
                    qid: bq,
                    iounit: bi,
                },
            ) => qid_eq(aq, bq) && ai == bi,
            (
                Msg::Tcreate {
                    fid: af,
                    name: an,
                    perm: ap,
                    mode: am,
                },
                Msg::Tcreate {
                    fid: bf,
                    name: bn,
                    perm: bp,
                    mode: bm,
                },
            ) => af == bf && an == bn && ap == bp && am == bm,
            (
                Msg::Rcreate {
                    qid: aq,
                    iounit: ai,
                },
                Msg::Rcreate {
                    qid: bq,
                    iounit: bi,
                },
            ) => qid_eq(aq, bq) && ai == bi,
            (
                Msg::Tread {
                    fid: af,
                    offset: ao,
                    count: ac,
                },
                Msg::Tread {
                    fid: bf,
                    offset: bo,
                    count: bc,
                },
            ) => af == bf && ao == bo && ac == bc,
            (Msg::Rread { data: a }, Msg::Rread { data: b }) => a == b,
            (
                Msg::Twrite {
                    fid: af,
                    offset: ao,
                    data: ad,
                },
                Msg::Twrite {
                    fid: bf,
                    offset: bo,
                    data: bd,
                },
            ) => af == bf && ao == bo && ad == bd,
            (Msg::Rwrite { count: a }, Msg::Rwrite { count: b }) => a == b,
            (Msg::Tclunk { fid: a }, Msg::Tclunk { fid: b }) => a == b,
            (Msg::Rclunk, Msg::Rclunk) => true,
            (Msg::Tremove { fid: a }, Msg::Tremove { fid: b }) => a == b,
            (Msg::Rremove, Msg::Rremove) => true,
            (Msg::Tstat { fid: a }, Msg::Tstat { fid: b }) => a == b,
            (Msg::Rstat { stat: a }, Msg::Rstat { stat: b }) => a == b,
            (
                Msg::Twstat { fid: af, stat: as_ },
                Msg::Twstat { fid: bf, stat: bs },
            ) => af == bf && as_ == bs,
            (Msg::Rwstat, Msg::Rwstat) => true,
            (
                Msg::Tcompound {
                    num_ops: an,
                    op_stream: ao,
                },
                Msg::Tcompound {
                    num_ops: bn,
                    op_stream: bo,
                },
            ) => an == bn && ao == bo,
            (
                Msg::Rcompound {
                    num_completed: ac,
                    status: as_,
                    result_stream: ar,
                },
                Msg::Rcompound {
                    num_completed: bc,
                    status: bs,
                    result_stream: br,
                },
            ) => ac == bc && as_ == bs && ar == br,
            (
                Msg::Tstream {
                    fid_handle: ah,
                    stream_id: ai,
                    flags: af,
                    offset: ao,
                },
                Msg::Tstream {
                    fid_handle: bh,
                    stream_id: bi,
                    flags: bf,
                    offset: bo,
                },
            ) => ah == bh && ai == bi && af == bf && ao == bo,
            (
                Msg::Rstream {
                    stream_id: ai,
                    chunk_seq: ac,
                    data: ad,
                },
                Msg::Rstream {
                    stream_id: bi,
                    chunk_seq: bc,
                    data: bd,
                },
            ) => ai == bi && ac == bc && ad == bd,
            (
                Msg::TleaseRevoke {
                    qid_path: aq,
                    lease_gen: ag,
                    target_mode: am,
                    num_blocks: an,
                    block_ranges: ar,
                },
                Msg::TleaseRevoke {
                    qid_path: bq,
                    lease_gen: bg,
                    target_mode: bm,
                    num_blocks: bn,
                    block_ranges: br,
                },
            ) => aq == bq && ag == bg && am == bm && an == bn && ar == br,
            (
                Msg::RleaseAck {
                    qid_path: aq,
                    lease_gen: ag,
                    client_status: ac,
                },
                Msg::RleaseAck {
                    qid_path: bq,
                    lease_gen: bg,
                    client_status: bc,
                },
            ) => aq == bq && ag == bg && ac == bc,
            (
                Msg::Treclaim {
                    old_handle: ah,
                    target_epoch: ae,
                    client_token: at,
                },
                Msg::Treclaim {
                    old_handle: bh,
                    target_epoch: be,
                    client_token: bt,
                },
            ) => ah == bh && ae == be && at == bt,
            (
                Msg::Rreclaim {
                    new_handle: ah,
                    recovery_status: ar,
                },
                Msg::Rreclaim {
                    new_handle: bh,
                    recovery_status: br,
                },
            ) => ah == bh && ar == br,
            (
                Msg::Trekey {
                    kind: ak,
                    generation: ag,
                    confirm: ac,
                },
                Msg::Trekey {
                    kind: bk,
                    generation: bg,
                    confirm: bc,
                },
            ) => ak == bk && ag == bg && ac == bc,
            (
                Msg::Rrekey {
                    kind: ak,
                    generation: ag,
                    confirm: ac,
                },
                Msg::Rrekey {
                    kind: bk,
                    generation: bg,
                    confirm: bc,
                },
            ) => ak == bk && ag == bg && ac == bc,
            _ => false,
        }
    }
}

impl Eq for Msg {}

/// `dat::Qid` does not derive `PartialEq`; compare its three fields.
fn qid_eq(a: &Qid, b: &Qid) -> bool {
    a.path == b.path && a.vers == b.vers && a.typ == b.typ
}

/// The parameter/data split of a D-09 data-message body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataLayout {
    /// The number of leading parameter bytes before the page-aligned data.
    pub params_len: usize,
    /// The number of whole [`PAGE_LEN`](super::PAGE_LEN)-byte data pages.
    pub pages: usize,
}

/// The parameter/data split of a data-message body, or `None` for a
/// non-data opcode (D-09, P5).
///
/// The transport uses this to place the keystream (parameters from block
/// counter 0, page `i` from `1 + 64 * i`); Phase 5 uses it to split control
/// bytes from whole pages.
pub fn data_layout(typ: u8, body: &[u8]) -> Option<DataLayout> {
    let params_len = match typ {
        wire::TWRITE => 16,
        wire::RREAD => 4,
        wire::RSTREAM => 16,
        _ => return None,
    };
    let data_len = body.len().saturating_sub(params_len);
    Some(DataLayout {
        params_len,
        pages: data_len / PAGE_LEN,
    })
}

/// The registry opcode for a message.
pub fn opcode(msg: &Msg) -> u8 {
    match msg {
        Msg::Tversion { .. } => wire::TVERSION,
        Msg::Rversion { .. } => wire::RVERSION,
        Msg::Tauth { .. } => wire::TAUTH,
        Msg::Rauth { .. } => wire::RAUTH,
        Msg::Tattach { .. } => wire::TATTACH,
        Msg::Rattach { .. } => wire::RATTACH,
        Msg::Rerror { .. } => wire::RERROR,
        Msg::Tflush { .. } => wire::TFLUSH,
        Msg::Rflush => wire::RFLUSH,
        Msg::Twalk { .. } => wire::TWALK,
        Msg::Rwalk { .. } => wire::RWALK,
        Msg::Topen { .. } => wire::TOPEN,
        Msg::Ropen { .. } => wire::ROPEN,
        Msg::Tcreate { .. } => wire::TCREATE,
        Msg::Rcreate { .. } => wire::RCREATE,
        Msg::Tread { .. } => wire::TREAD,
        Msg::Rread { .. } => wire::RREAD,
        Msg::Twrite { .. } => wire::TWRITE,
        Msg::Rwrite { .. } => wire::RWRITE,
        Msg::Tclunk { .. } => wire::TCLUNK,
        Msg::Rclunk => wire::RCLUNK,
        Msg::Tremove { .. } => wire::TREMOVE,
        Msg::Rremove => wire::RREMOVE,
        Msg::Tstat { .. } => wire::TSTAT,
        Msg::Rstat { .. } => wire::RSTAT,
        Msg::Twstat { .. } => wire::TWSTAT,
        Msg::Rwstat => wire::RWSTAT,
        Msg::Tcompound { .. } => wire::TCOMPOUND,
        Msg::Rcompound { .. } => wire::RCOMPOUND,
        Msg::Tstream { .. } => wire::TSTREAM,
        Msg::Rstream { .. } => wire::RSTREAM,
        Msg::TleaseRevoke { .. } => wire::TLEASE_REVOKE,
        Msg::RleaseAck { .. } => wire::RLEASE_ACK,
        Msg::Treclaim { .. } => wire::TRECLAIM,
        Msg::Rreclaim { .. } => wire::RRECLAIM,
        Msg::Trekey { .. } => wire::TREKEY,
        Msg::Rrekey { .. } => wire::RREKEY,
    }
}

// ---------------------------------------------------------------------------
// Little-endian writers (`convS2M`'s PBIT*)
// ---------------------------------------------------------------------------

fn p16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn p32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn p64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}
/// `convS2M`'s `pstring`: a 16-bit little-endian length then the raw bytes.
fn pstr(out: &mut Vec<u8>, s: &[u8]) -> crate::Result<()> {
    let n = u16::try_from(s.len()).map_err(|_| "chan: string longer than 65535 bytes")?;
    p16(out, n);
    out.extend_from_slice(s);
    Ok(())
}
/// `convS2M`'s `pqid`: type[1] vers[4] path[8].
fn pqid(out: &mut Vec<u8>, q: &Qid) {
    out.push(q.typ);
    p32(out, q.vers);
    p64(out, q.path);
}

/// Append one message body to `out` (no `size[4] type tag[2]`).
pub fn encode_body(msg: &Msg, out: &mut Vec<u8>) -> crate::Result<()> {
    match msg {
        Msg::Tversion { msize, version } => {
            p32(out, *msize);
            pstr(out, version)?;
        }
        Msg::Rversion { msize, version } => {
            p32(out, *msize);
            pstr(out, version)?;
        }
        Msg::Tauth { afid, uname, aname } => {
            p32(out, *afid);
            pstr(out, uname)?;
            pstr(out, aname)?;
        }
        Msg::Rauth { aqid } => pqid(out, aqid),
        Msg::Tattach {
            fid,
            afid,
            uname,
            aname,
        } => {
            p32(out, *fid);
            p32(out, *afid);
            pstr(out, uname)?;
            pstr(out, aname)?;
        }
        Msg::Rattach { qid } => pqid(out, qid),
        Msg::Rerror { ename } => pstr(out, ename)?,
        Msg::Tflush { oldtag } => p16(out, *oldtag),
        Msg::Rflush => {}
        Msg::Twalk { fid, newfid, wname } => {
            if wname.len() > MAXWELEM {
                return Err("chan: Twalk with more than 16 names");
            }
            p32(out, *fid);
            p32(out, *newfid);
            p16(
                out,
                u16::try_from(wname.len()).map_err(|_| "chan: too many walk names")?,
            );
            for name in wname {
                pstr(out, name)?;
            }
        }
        Msg::Rwalk { wqid } => {
            if wqid.len() > MAXWELEM {
                return Err("chan: Rwalk with more than 16 qids");
            }
            p16(
                out,
                u16::try_from(wqid.len()).map_err(|_| "chan: too many walk qids")?,
            );
            for q in wqid {
                pqid(out, q);
            }
        }
        Msg::Topen { fid, mode } => {
            p32(out, *fid);
            out.push(*mode);
        }
        Msg::Ropen { qid, iounit } => {
            pqid(out, qid);
            p32(out, *iounit);
        }
        Msg::Tcreate {
            fid,
            name,
            perm,
            mode,
        } => {
            p32(out, *fid);
            pstr(out, name)?;
            p32(out, *perm);
            out.push(*mode);
        }
        Msg::Rcreate { qid, iounit } => {
            pqid(out, qid);
            p32(out, *iounit);
        }
        Msg::Tread { fid, offset, count } => {
            p32(out, *fid);
            p64(out, *offset);
            p32(out, *count);
        }
        Msg::Rread { data } => {
            p32(out, data_count(data.len())?);
            write_padded(out, data);
        }
        Msg::Twrite { fid, offset, data } => {
            p32(out, *fid);
            p64(out, *offset);
            p32(out, data_count(data.len())?);
            write_padded(out, data);
        }
        Msg::Rwrite { count } => p32(out, *count),
        Msg::Tclunk { fid } => p32(out, *fid),
        Msg::Rclunk => {}
        Msg::Tremove { fid } => p32(out, *fid),
        Msg::Rremove => {}
        Msg::Tstat { fid } => p32(out, *fid),
        Msg::Rstat { stat } => pstr(out, stat)?,
        Msg::Twstat { fid, stat } => {
            p32(out, *fid);
            pstr(out, stat)?;
        }
        Msg::Rwstat => {}
        Msg::Tcompound { num_ops, op_stream } => {
            p16(out, *num_ops);
            out.extend_from_slice(op_stream);
        }
        Msg::Rcompound {
            num_completed,
            status,
            result_stream,
        } => {
            p16(out, *num_completed);
            p16(out, *status);
            out.extend_from_slice(result_stream);
        }
        Msg::Tstream {
            fid_handle,
            stream_id,
            flags,
            offset,
        } => {
            out.extend_from_slice(fid_handle);
            p32(out, *stream_id);
            p16(out, *flags);
            p64(out, *offset);
        }
        Msg::Rstream {
            stream_id,
            chunk_seq,
            data,
        } => {
            p32(out, *stream_id);
            p64(out, *chunk_seq);
            p32(out, data_count(data.len())?);
            write_padded(out, data);
        }
        Msg::TleaseRevoke {
            qid_path,
            lease_gen,
            target_mode,
            num_blocks,
            block_ranges,
        } => {
            p64(out, *qid_path);
            p64(out, *lease_gen);
            out.push(*target_mode);
            p32(out, *num_blocks);
            out.extend_from_slice(block_ranges);
        }
        Msg::RleaseAck {
            qid_path,
            lease_gen,
            client_status,
        } => {
            p64(out, *qid_path);
            p64(out, *lease_gen);
            out.push(*client_status);
        }
        Msg::Treclaim {
            old_handle,
            target_epoch,
            client_token,
        } => {
            out.extend_from_slice(old_handle);
            p16(out, *target_epoch);
            out.extend_from_slice(client_token);
        }
        Msg::Rreclaim {
            new_handle,
            recovery_status,
        } => {
            out.extend_from_slice(new_handle);
            p16(out, *recovery_status);
        }
        Msg::Trekey {
            kind,
            generation,
            confirm,
        } => encode_rekey(out, *kind, *generation, confirm)?,
        Msg::Rrekey {
            kind,
            generation,
            confirm,
        } => encode_rekey(out, *kind, *generation, confirm)?,
    }
    Ok(())
}

/// Append zero padding so the data area is a whole number of 4096-byte pages.
///
/// The padding is written as zeros before encryption, never left stale
/// (T-04-15).
fn write_padded(out: &mut Vec<u8>, data: &[u8]) {
    out.extend_from_slice(data);
    let rem = data.len() % PAGE_LEN;
    if rem != 0 {
        out.resize(out.len() + (PAGE_LEN - rem), 0);
    }
}

/// The 37-byte re-key body: `kind[1] generation[4] confirm[32]` (P3).
fn encode_rekey(
    out: &mut Vec<u8>,
    kind: u8,
    generation: u32,
    confirm: &[u8; 32],
) -> crate::Result<()> {
    if kind > REKEY_ACK {
        return Err("chan: unknown re-key kind");
    }
    out.push(kind);
    p32(out, generation);
    out.extend_from_slice(confirm);
    Ok(())
}

/// A buffer length as the wire's 32-bit count, checked, never narrowed.
fn data_count(n: usize) -> crate::Result<u32> {
    u32::try_from(n).map_err(|_| "chan: data length exceeds 32 bits")
}

// ---------------------------------------------------------------------------
// A checked little-endian reader with exact-consumption
// ---------------------------------------------------------------------------

struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cur<'a> {
    fn new(b: &'a [u8]) -> Cur<'a> {
        Cur { b, p: 0 }
    }

    fn bytes(&mut self, n: usize) -> crate::Result<&'a [u8]> {
        let end = self
            .p
            .checked_add(n)
            .ok_or("chan: body offset overflows")?;
        let s = self.b.get(self.p..end).ok_or("chan: truncated body")?;
        self.p = end;
        Ok(s)
    }

    fn u8(&mut self) -> crate::Result<u8> {
        let s = self.bytes(1)?;
        Ok(s[0])
    }

    fn u16(&mut self) -> crate::Result<u16> {
        let a: [u8; 2] = self.bytes(2)?.try_into().map_err(|_| "chan: truncated body")?;
        Ok(u16::from_le_bytes(a))
    }

    fn u32(&mut self) -> crate::Result<u32> {
        let a: [u8; 4] = self.bytes(4)?.try_into().map_err(|_| "chan: truncated body")?;
        Ok(u32::from_le_bytes(a))
    }

    fn u64(&mut self) -> crate::Result<u64> {
        let a: [u8; 8] = self.bytes(8)?.try_into().map_err(|_| "chan: truncated body")?;
        Ok(u64::from_le_bytes(a))
    }

    fn string(&mut self) -> crate::Result<Vec<u8>> {
        let n = usize::from(self.u16()?);
        Ok(self.bytes(n)?.to_vec())
    }

    fn qid(&mut self) -> crate::Result<Qid> {
        let typ = self.u8()?;
        let vers = self.u32()?;
        let path = self.u64()?;
        Ok(Qid { path, vers, typ })
    }

    /// A fixed-size byte array (a token or a key confirmation).
    fn array<const N: usize>(&mut self) -> crate::Result<[u8; N]> {
        self.bytes(N)?.try_into().map_err(|_| "chan: truncated body")
    }

    /// Everything left in the body, consuming it (the variable opaque tail).
    fn rest(&mut self) -> Vec<u8> {
        let out = self.b.get(self.p..).unwrap_or(&[]).to_vec();
        self.p = self.b.len();
        out
    }

    fn done(&self) -> crate::Result<()> {
        if self.p == self.b.len() {
            Ok(())
        } else {
            Err("chan: trailing bytes after the body")
        }
    }
}

/// `count[4]`, then the D-09 page-aligned data area.
///
/// The receiver requires `data_len` to be a whole number of pages,
/// `count <= data_len < count + PAGE_LEN`, and every padding byte to be zero
/// (T-04-15, T-04-16).  The true bytes are returned; the padding is dropped.
fn take_data(cur: &mut Cur<'_>) -> crate::Result<Vec<u8>> {
    let count = usize::try_from(cur.u32()?).map_err(|_| "chan: data count too large")?;
    let data = cur.rest();
    if !data.len().is_multiple_of(PAGE_LEN) {
        return Err("chan: data area is not a whole number of pages");
    }
    if count > data.len() {
        return Err("chan: count exceeds the data area");
    }
    let bound = count
        .checked_add(PAGE_LEN)
        .ok_or("chan: data count overflows")?;
    if data.len() >= bound {
        return Err("chan: data area is more than one page beyond the count");
    }
    if data[count..].iter().any(|&b| b != 0) {
        return Err("chan: nonzero data padding");
    }
    Ok(data[..count].to_vec())
}

/// Decode one message body of type `typ` (no `size[4] type tag[2]`).
pub fn decode_body(typ: u8, body: &[u8]) -> crate::Result<Msg> {
    let mut cur = Cur::new(body);
    let msg = match typ {
        wire::TVERSION => Msg::Tversion {
            msize: cur.u32()?,
            version: cur.string()?,
        },
        wire::RVERSION => Msg::Rversion {
            msize: cur.u32()?,
            version: cur.string()?,
        },
        wire::TAUTH => Msg::Tauth {
            afid: cur.u32()?,
            uname: cur.string()?,
            aname: cur.string()?,
        },
        wire::RAUTH => Msg::Rauth { aqid: cur.qid()? },
        wire::TATTACH => Msg::Tattach {
            fid: cur.u32()?,
            afid: cur.u32()?,
            uname: cur.string()?,
            aname: cur.string()?,
        },
        wire::RATTACH => Msg::Rattach { qid: cur.qid()? },
        wire::RERROR => Msg::Rerror {
            ename: cur.string()?,
        },
        wire::TFLUSH => Msg::Tflush {
            oldtag: cur.u16()?,
        },
        wire::RFLUSH => Msg::Rflush,
        wire::TWALK => {
            let fid = cur.u32()?;
            let newfid = cur.u32()?;
            let n = usize::from(cur.u16()?);
            if n > MAXWELEM {
                return Err("chan: Twalk with more than 16 names");
            }
            let mut wname = Vec::with_capacity(n);
            for _ in 0..n {
                wname.push(cur.string()?);
            }
            Msg::Twalk { fid, newfid, wname }
        }
        wire::RWALK => {
            let n = usize::from(cur.u16()?);
            if n > MAXWELEM {
                return Err("chan: Rwalk with more than 16 qids");
            }
            let mut wqid = Vec::with_capacity(n);
            for _ in 0..n {
                wqid.push(cur.qid()?);
            }
            Msg::Rwalk { wqid }
        }
        wire::TOPEN => Msg::Topen {
            fid: cur.u32()?,
            mode: cur.u8()?,
        },
        wire::ROPEN => Msg::Ropen {
            qid: cur.qid()?,
            iounit: cur.u32()?,
        },
        wire::TCREATE => Msg::Tcreate {
            fid: cur.u32()?,
            name: cur.string()?,
            perm: cur.u32()?,
            mode: cur.u8()?,
        },
        wire::RCREATE => Msg::Rcreate {
            qid: cur.qid()?,
            iounit: cur.u32()?,
        },
        wire::TREAD => Msg::Tread {
            fid: cur.u32()?,
            offset: cur.u64()?,
            count: cur.u32()?,
        },
        wire::RREAD => Msg::Rread {
            data: take_data(&mut cur)?,
        },
        wire::TWRITE => {
            let fid = cur.u32()?;
            let offset = cur.u64()?;
            let data = take_data(&mut cur)?;
            Msg::Twrite { fid, offset, data }
        }
        wire::RWRITE => Msg::Rwrite { count: cur.u32()? },
        wire::TCLUNK => Msg::Tclunk { fid: cur.u32()? },
        wire::RCLUNK => Msg::Rclunk,
        wire::TREMOVE => Msg::Tremove { fid: cur.u32()? },
        wire::RREMOVE => Msg::Rremove,
        wire::TSTAT => Msg::Tstat { fid: cur.u32()? },
        wire::RSTAT => Msg::Rstat {
            stat: cur.string()?,
        },
        wire::TWSTAT => Msg::Twstat {
            fid: cur.u32()?,
            stat: cur.string()?,
        },
        wire::RWSTAT => Msg::Rwstat,
        wire::TCOMPOUND => Msg::Tcompound {
            num_ops: cur.u16()?,
            op_stream: cur.rest(),
        },
        wire::RCOMPOUND => Msg::Rcompound {
            num_completed: cur.u16()?,
            status: cur.u16()?,
            result_stream: cur.rest(),
        },
        wire::TSTREAM => Msg::Tstream {
            fid_handle: cur.array()?,
            stream_id: cur.u32()?,
            flags: cur.u16()?,
            offset: cur.u64()?,
        },
        wire::RSTREAM => Msg::Rstream {
            stream_id: cur.u32()?,
            chunk_seq: cur.u64()?,
            data: take_data(&mut cur)?,
        },
        wire::TLEASE_REVOKE => Msg::TleaseRevoke {
            qid_path: cur.u64()?,
            lease_gen: cur.u64()?,
            target_mode: cur.u8()?,
            num_blocks: cur.u32()?,
            block_ranges: cur.rest(),
        },
        wire::RLEASE_ACK => Msg::RleaseAck {
            qid_path: cur.u64()?,
            lease_gen: cur.u64()?,
            client_status: cur.u8()?,
        },
        wire::TRECLAIM => Msg::Treclaim {
            old_handle: cur.array()?,
            target_epoch: cur.u16()?,
            client_token: cur.array()?,
        },
        wire::RRECLAIM => Msg::Rreclaim {
            new_handle: cur.array()?,
            recovery_status: cur.u16()?,
        },
        wire::TREKEY => {
            let kind = cur.u8()?;
            if kind > REKEY_ACK {
                return Err("chan: unknown re-key kind");
            }
            Msg::Trekey {
                kind,
                generation: cur.u32()?,
                confirm: cur.array()?,
            }
        }
        wire::RREKEY => {
            let kind = cur.u8()?;
            if kind > REKEY_ACK {
                return Err("chan: unknown re-key kind");
            }
            Msg::Rrekey {
                kind,
                generation: cur.u32()?,
                confirm: cur.array()?,
            }
        }
        _ => return Err("chan: unknown opcode"),
    };
    cur.done()?;
    Ok(msg)
}
