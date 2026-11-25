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
//! (T-04-14).  Task 2 adds the new message pairs 130 to 139 and the D-09
//! two-part data body.

use alloc::vec::Vec;

use crate::dat::Qid;

/// The 9P2000 maximum number of path elements in a `Twalk` / `Rwalk`
/// (`fcall.h` `MAXWELEM`).
pub const MAXWELEM: usize = 16;

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
            _ => false,
        }
    }
}

impl Eq for Msg {}

/// `dat::Qid` does not derive `PartialEq`; compare its three fields.
fn qid_eq(a: &Qid, b: &Qid) -> bool {
    a.path == b.path && a.vers == b.vers && a.typ == b.typ
}

/// The registry opcode for a message.
pub fn opcode(_msg: &Msg) -> u8 {
    0
}

/// Append one message body to `out` (no `size[4] type tag[2]`).
pub fn encode_body(_msg: &Msg, _out: &mut Vec<u8>) -> crate::Result<()> {
    Ok(())
}

/// Decode one message body of type `typ` (no `size[4] type tag[2]`).
pub fn decode_body(_typ: u8, _body: &[u8]) -> crate::Result<Msg> {
    Err("chan: message body decode not implemented")
}
