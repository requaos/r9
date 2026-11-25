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

use super::wire;

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
            p16(out, wname.len() as u16);
            for name in wname {
                pstr(out, name)?;
            }
        }
        Msg::Rwalk { wqid } => {
            if wqid.len() > MAXWELEM {
                return Err("chan: Rwalk with more than 16 qids");
            }
            p16(out, wqid.len() as u16);
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
            out.extend_from_slice(data);
        }
        Msg::Twrite { fid, offset, data } => {
            p32(out, *fid);
            p64(out, *offset);
            p32(out, data_count(data.len())?);
            out.extend_from_slice(data);
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
    }
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

    fn done(&self) -> crate::Result<()> {
        if self.p == self.b.len() {
            Ok(())
        } else {
            Err("chan: trailing bytes after the body")
        }
    }
}

/// `count[4] data[count]` — the plain 9P2000 data body (Task 1).
fn take_data_plain(cur: &mut Cur<'_>) -> crate::Result<Vec<u8>> {
    let n = usize::try_from(cur.u32()?).map_err(|_| "chan: data count too large")?;
    Ok(cur.bytes(n)?.to_vec())
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
            data: take_data_plain(&mut cur)?,
        },
        wire::TWRITE => {
            let fid = cur.u32()?;
            let offset = cur.u64()?;
            let data = take_data_plain(&mut cur)?;
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
        _ => return Err("chan: unknown opcode"),
    };
    cur.done()?;
    Ok(msg)
}
