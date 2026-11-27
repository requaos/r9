//! aead_loop — the boot-switch-gated in-kernel `9P2000.aead` loopback server
//! (SMP-13, D-28).
//!
//! When (and only when) the kernel command line carries the exact token
//! `aeadloop=1`, this module builds:
//!
//! * a real [`FidTable`] at the synthetic tier (`4 shards x 256 slots`, D-12),
//! * a real [`Session`](crate::chan::transport::Session) server end finished
//!   with an injected gate-only test secret, and
//! * a duplex in-kernel link ([`LoopLink`]) carrying whole 9P messages between
//!   the kernel's client engine and the server end.
//!
//! The kernel's own [`Mnt`](crate::devmnt::Mnt) client negotiates
//! `Dialect::Aead` over that link, finishes the handshake, and mounts the
//! server; the resulting `DEV_MNT` root chan is published behind the `#M`
//! registry arm.  A default boot allocates nothing, starts no server, and the
//! `#M` attach returns the donor `Enodev` shape (`namec: unknown # device`).
//!
//! Safety: `#[forbid(unsafe_code)]`.  The host fixture's `Rc<RefCell<...>>` is
//! replaced with `alloc::sync::Arc` plus the crate's own `Lock`
//! (`LockRank::DevTop` for the server core); every shared structure is plain
//! safe Rust.
//!
//! Wire shape: the client's [`Mnt`](crate::devmnt::Mnt) speaks plain `9P2000`
//! to a sealed [`MntIo`] adapter ([`SealedWriteIo`] / [`SealedReadIo`]) that
//! seals each frame with the client `Session` and tracks the server-assigned
//! 8-byte fid handles (the sealed header's `fid_handle` is separate from the
//! base body's `fid`).  The server end opens the sealed packets, dispatches
//! through the real `FidTable`, and answers sealed.

#![allow(dead_code)]

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crypto::key::Key;

use crate::chan::msg::{self, Msg};
use crate::chan::transport::{
    Handshake, Role, Session, TransportConfig,
};
use crate::chan::{self, wire, PAGE_LEN};
use crate::dat::{Chan, Qid, QTDIR, QTFILE};
use crate::devmnt::{Dialect, Mnt, MntIo, Negotiated};
use crate::fid::{FidEntry, FidTable, StatefulHandle};
use crate::mcslock::{Lock, LockNode};
use crate::smp::LockRank;
use crate::Result;

/// The negotiated msize the synthetic loopback agrees on (the transport
/// default, equal to the epoch the fixture's canonical handshake uses).
const MSIZE: u32 = 8192;

/// The loopback's wall clock reading: the gate-only server never re-keys and
/// no deadline is armed, so a constant zero is the fixture's fake clock.
const NOW: u64 = 0;

/// The synthetic mount root's qid path.
const ROOT_PATH: u64 = 0x1000;

/// The deterministic payload files: `data0`..`data7`, 4 KiB each.
const DATA_FILES: usize = 8;
const FILE_LEN: usize = 4096;
const FILE_NAMES: [&[u8]; DATA_FILES] = [
    b"data0", b"data1", b"data2", b"data3", b"data4", b"data5", b"data6", b"data7",
];

/// The client's opaque exchange bytes for the handshake transcript (D-16).
const CLIENT_EXCHANGE: &[u8] = b"kernel aeadloop client exchange";
/// The server's opaque exchange bytes for the handshake transcript (D-16).
const SERVER_EXCHANGE: &[u8] = b"kernel aeadloop server exchange";

/// The auth fid the handshake exchange rides (D-16).
const AFID: u32 = 7;

/// The gate-only server's owning client id (0 is reserved, D-08).
const CLIENT_ID: u32 = 1;

/// Whether the kernel command line carries the exact whitespace-delimited
/// token `aeadloop=1` (SMP-13, D-28).
pub fn parse_boot_switch(cmdline: &str) -> bool {
    cmdline.split_whitespace().any(|tok| tok == "aeadloop=1")
}

/// The deterministic byte of file `file` at offset `off` (a fixed pattern the
/// host tests verify byte for byte).
fn file_byte(file: usize, off: usize) -> u8 {
    (file as u8)
        .wrapping_mul(37)
        .wrapping_add((off as u8).wrapping_mul(3))
        .wrapping_add(1)
}

/// The deterministic body for `file`'s qid path and `[offset, offset+count)`.
fn file_bytes(path: u64, offset: u64, count: u32) -> Vec<u8> {
    if path == ROOT_PATH || path < ROOT_PATH + 1 {
        return Vec::new();
    }
    let idx = (path - ROOT_PATH - 1) as usize;
    if idx >= DATA_FILES {
        return Vec::new();
    }
    let start = offset.min(FILE_LEN as u64) as usize;
    let end = (start + count as usize).min(FILE_LEN);
    (start..end).map(|off| file_byte(idx, off)).collect()
}

fn file_index(name: &[u8]) -> Option<usize> {
    FILE_NAMES.iter().position(|candidate| *candidate == name)
}

/// One stat(5) wire record for the synthetic root directory or a data file
/// (the shared `StatWriter` shape: the 41-byte fixed part plus FOUR
/// length-prefixed strings — name/uid/gid/muid, which libc's `statcheck`
/// walks).  The reply to Tstat and each record in the root's directory read
/// share this shape.
fn stat_record(path: u64, name: &[u8]) -> Vec<u8> {
    let (mode, length) = if path == ROOT_PATH {
        (0x8000_0000 | 0o555, 0)
    } else {
        (0o644, FILE_LEN as u64)
    };
    let need = 41 + (2 + name.len()) + 3 * (2 + 6); // uid/gid/muid = "glenda"
    let mut buf = alloc::vec![0u8; need];
    let n = {
        let mut w = crate::dat::StatWriter::new(&mut buf);
        w.u16(0); // type: unused this era
        w.u32(0); // dev: the synthetic server owns no device number
        w.u8(if path == ROOT_PATH { QTDIR } else { QTFILE });
        w.u32(0); // vers
        w.u64(path);
        w.u32(mode);
        w.u32(0); // atime
        w.u32(0); // mtime
        w.u64(length);
        w.string(name);
        w.string(b"glenda");
        w.string(b"glenda");
        w.string(b"glenda"); // muid — the fourth string statcheck walks
        w.finish() as usize
    };
    buf.truncate(n);
    buf
}

/// The root directory's byte listing: one stat(5) record per data file, in
/// name order, honoring the standard 9P offset/count window (a directory is
/// read in pieces; `ls` walks it sequentially).  This is what `ls '#M'`
/// enumerates — the loopback root is a real read-only directory, not just
/// an attach target.
fn directory_bytes(offset: u64, count: u32) -> Vec<u8> {
    let mut listing = Vec::new();
    for (idx, name) in FILE_NAMES.iter().enumerate() {
        listing.extend_from_slice(&stat_record(ROOT_PATH + 1 + idx as u64, name));
    }
    let start = (offset as usize).min(listing.len());
    let end = (start + count as usize).min(listing.len());
    listing[start..end].to_vec()
}

fn qid_for(path: u64) -> Qid {
    Qid {
        path,
        vers: 0,
        typ: if path == ROOT_PATH { QTDIR } else { 0 },
    }
}

fn entry(fid_key: u32, qid_path: u64, qid_type: u8) -> FidEntry {
    FidEntry {
        fid_key,
        client_id: 0,
        qid_path,
        offset: 0,
        qid_vers: 0,
        iounit: 0,
        lease_gen: 0,
        ring_slot: 0,
        dev_handler: 0,
        epoch: 0,
        qid_type,
        open_mode: 0,
        flags: 0,
        volatility: 0,
        _reserved: [0u8; 7],
    }
}

/// The injected gate-only test secret (never exposed on a default boot).
fn test_secret() -> Key {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = 0x10u8.wrapping_add(i as u8);
    }
    Key::from_bytes(bytes)
}

/// The capability domain secret of the loopback's `FidTable` (distinct from
/// the session secret; used only for the stateless-cap signing key, which this
/// gate-only server issues no caps under).
fn domain_key() -> Key {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = 0xA0u8.wrapping_add(i as u8);
    }
    Key::from_bytes(bytes)
}

// ---------------------------------------------------------------------------
// The duplex link and its plaintext framing helpers
// ---------------------------------------------------------------------------

/// The in-kernel duplex byte link: client-to-server and server-to-client
/// queues.  One whole message per `write_msg`; `read_some` drains available
/// bytes.  Unranked `Lock`s (the byte queues never nest under the ranked
/// server core).
struct LoopLink {
    c2s: Lock<VecDeque<u8>>,
    s2c: Lock<VecDeque<u8>>,
}

impl LoopLink {
    fn new() -> LoopLink {
        LoopLink {
            c2s: Lock::new("aeadloop-c2s", VecDeque::new()),
            s2c: Lock::new("aeadloop-s2c", VecDeque::new()),
        }
    }

    fn push(&self, to_server: bool, buf: &[u8]) {
        let node = LockNode::new();
        let queue = if to_server { &self.c2s } else { &self.s2c };
        let mut guard = queue.lock(&node);
        guard.extend(buf.iter().copied());
    }

    fn drain(&self, from_server: bool, buf: &mut [u8]) -> usize {
        let node = LockNode::new();
        let queue = if from_server { &self.s2c } else { &self.c2s };
        let mut guard = queue.lock(&node);
        let n = guard.len().min(buf.len());
        for slot in buf.iter_mut().take(n) {
            *slot = guard.pop_front().unwrap_or(0);
        }
        n
    }
}

fn read_exact(link_or_io: &mut dyn MntIo, buf: &mut [u8]) -> Result<()> {
    let mut got = 0usize;
    while got < buf.len() {
        let n = link_or_io.read_some(&mut buf[got..])? as usize;
        if n == 0 {
            return Err("aeadloop: loopback eof before a message completed");
        }
        got += n;
    }
    Ok(())
}

fn read_link_exact(link: &LoopLink, from_server: bool, buf: &mut [u8]) -> Result<()> {
    let mut got = 0usize;
    while got < buf.len() {
        let n = link.drain(from_server, &mut buf[got..]);
        if n == 0 {
            return Err("aeadloop: loopback eof before a message completed");
        }
        got += n;
    }
    Ok(())
}

/// Frame one plain 9P2000 message: `size[4] type[1] tag[2] body`.
fn plain_frame(typ: u8, tag: u16, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(7 + body.len());
    out.extend_from_slice(&((7 + body.len()) as u32).to_le_bytes());
    out.push(typ);
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(body);
    out
}

fn encoded_body(message: &Msg) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    msg::encode_body(message, &mut body)?;
    Ok(body)
}

fn tauth_frame() -> Result<Vec<u8>> {
    Ok(plain_frame(
        wire::TAUTH,
        1,
        &encoded_body(&Msg::Tauth {
            afid: AFID,
            uname: b"none".to_vec(),
            aname: Vec::new(),
        })?,
    ))
}

fn twrite_frame(data: &[u8]) -> Result<Vec<u8>> {
    Ok(plain_frame(
        wire::TWRITE,
        2,
        &encoded_body(&Msg::Twrite {
            fid: AFID,
            offset: 0,
            data: data.to_vec(),
        })?,
    ))
}

fn tread_frame() -> Result<Vec<u8>> {
    Ok(plain_frame(
        wire::TREAD,
        3,
        &encoded_body(&Msg::Tread {
            fid: AFID,
            offset: 0,
            count: 65536,
        })?,
    ))
}

fn read_plain_reply(io: &mut dyn MntIo) -> Result<Vec<u8>> {
    let mut hdr = [0u8; 4];
    read_exact(io, &mut hdr)?;
    let size = u32::from_le_bytes(hdr) as usize;
    if size < 7 {
        return Err("aeadloop: runt plain reply");
    }
    let mut frame = Vec::with_capacity(size);
    frame.extend_from_slice(&hdr);
    let mut body = alloc::vec![0u8; size - 4];
    read_exact(io, &mut body)?;
    frame.extend_from_slice(&body);
    Ok(frame)
}

fn plain_step(
    io_w: &mut dyn MntIo,
    io_r: &mut dyn MntIo,
    handshake: &mut Handshake,
    frame: &[u8],
) -> Result<()> {
    handshake.absorb_plain(frame);
    let n = io_w.write_msg(frame)? as usize;
    if n != frame.len() {
        return Err("aeadloop: short plain write on the loopback");
    }
    let reply = read_plain_reply(io_r)?;
    handshake.absorb_plain(&reply);
    Ok(())
}

// ---------------------------------------------------------------------------
// The server end: real FidTable + real Session over the link
// ---------------------------------------------------------------------------

/// One opened sealed request, decoded.
struct Opened {
    msg: Msg,
    tag: u16,
    handle: [u8; 8],
}

/// The server end of the loopback: the real `FidTable`, the real `Session`,
/// and the plain-handshake state, all behind one ranked `Lock` (DevTop).
struct LoopServer {
    link: Arc<LoopLink>,
    handshake: Option<Handshake>,
    session: Option<Session>,
    secret: Key,
    config: TransportConfig,
    table: Arc<FidTable>,
    client_id: u32,
}

impl LoopServer {
    fn new(link: Arc<LoopLink>, table: Arc<FidTable>, client_id: u32, secret: Key) -> LoopServer {
        LoopServer {
            link,
            handshake: Some(Handshake::new(Role::Server, TransportConfig::default())),
            session: None,
            secret,
            config: TransportConfig::default(),
            table,
            client_id,
        }
    }

    /// Answer one plain-framed client message during negotiation (Tversion,
    /// Tauth, the afid Twrite/Tread); the Tread finishes the server handshake.
    fn receive_plain(&mut self, frame: &[u8]) -> Result<()> {
        if frame.len() < 7 {
            return Err("aeadloop: runt plain client frame");
        }
        let typ = frame[4];
        let tag = u16::from_le_bytes([frame[5], frame[6]]);
        if typ == wire::TVERSION {
            let Msg::Tversion { msize: offer, .. } = msg::decode_body(typ, &frame[7..])? else {
                return Err("aeadloop: malformed Tversion");
            };
            let answer = offer.min(MSIZE);
            self.config.msize = answer;
            self.handshake = Some(Handshake::new(Role::Server, self.config));
            self.absorb(frame);
            let reply = plain_frame(
                wire::RVERSION,
                tag,
                &encoded_body(&Msg::Rversion {
                    msize: answer,
                    version: chan::VERSION_AEAD.to_vec(),
                })?,
            );
            self.absorb(&reply);
            self.link.push(false, &reply);
            return Ok(());
        }
        self.absorb(frame);
        let reply = match typ {
            wire::TAUTH => plain_frame(
                wire::RAUTH,
                tag,
                &encoded_body(&Msg::Rauth {
                    aqid: Qid { path: 0, vers: 0, typ: 0 },
                })?,
            ),
            wire::TWRITE => {
                let Msg::Twrite { data, .. } = msg::decode_body(typ, &frame[7..])? else {
                    return Err("aeadloop: malformed afid Twrite");
                };
                plain_frame(
                    wire::RWRITE,
                    tag,
                    &encoded_body(&Msg::Rwrite { count: data.len() as u32 })?,
                )
            }
            wire::TREAD => plain_frame(
                wire::RREAD,
                tag,
                &encoded_body(&Msg::Rread { data: SERVER_EXCHANGE.to_vec() })?,
            ),
            _ => return Err("aeadloop: unexpected plain message"),
        };
        self.absorb(&reply);
        self.link.push(false, &reply);
        if typ == wire::TREAD {
            let handshake = self.handshake.take().ok_or("aeadloop: no running handshake")?;
            self.session = Some(handshake.finish(self.secret.clone()));
        }
        Ok(())
    }

    fn absorb(&mut self, frame: &[u8]) {
        if let Some(handshake) = &mut self.handshake {
            handshake.absorb_plain(frame);
        }
    }

    /// Open one sealed request from the client-to-server queue and answer it.
    fn handle_one(&mut self) -> Result<()> {
        let opened = self.recv_sealed()?;
        self.serve(opened)
    }

    fn recv_sealed(&mut self) -> Result<Opened> {
        let link = Arc::clone(&self.link);
        let mut prefix = [0u8; 4];
        read_link_exact(&link, false, &mut prefix)?;
        let len = {
            let session = self.session.as_mut().ok_or("aeadloop: no server session")?;
            session.frame_len(&prefix).map_err(|_| "aeadloop: bad sealed frame length")?
        };
        let mut packet = alloc::vec![0u8; len];
        packet[..4].copy_from_slice(&prefix);
        read_link_exact(&link, false, &mut packet[4..])?;
        let (typ, tag, handle, body) = {
            let session = self.session.as_mut().ok_or("aeadloop: no server session")?;
            let verified = session
                .receive(&mut packet, NOW)
                .map_err(|_| "aeadloop: server receive failed")?;
            let header = *verified.header();
            (header.typ, header.tag, header.fid_handle, verified.body().to_vec())
        };
        Ok(Opened { msg: msg::decode_body(typ, &body)?, tag, handle })
    }

    fn resolve(&self, handle: &[u8; 8]) -> Result<FidEntry> {
        let parsed = StatefulHandle::from_wire(handle)?;
        self.table.resolve(self.client_id, &parsed)
    }

    fn serve(&mut self, opened: Opened) -> Result<()> {
        match opened.msg {
            Msg::Tattach { fid, .. } => {
                let fresh = self
                    .table
                    .alloc(self.client_id, entry(fid, ROOT_PATH, QTDIR))
                    .map_err(|_| "aeadloop: fid table full on attach")?;
                self.reply(
                    opened.tag,
                    fresh.to_wire(),
                    &Msg::Rattach { qid: qid_for(ROOT_PATH) },
                )
            }
            Msg::Twalk { newfid, wname, .. } => {
                let source = match self.resolve(&opened.handle) {
                    Ok(source) => source,
                    Err(_) => return self.rerror(opened.tag),
                };
                let next_path = if wname.is_empty() {
                    source.qid_path
                } else {
                    if source.qid_path != ROOT_PATH {
                        return self.rerror(opened.tag);
                    }
                    match file_index(&wname[0]) {
                        Some(idx) => ROOT_PATH + 1 + idx as u64,
                        None => return self.rerror(opened.tag),
                    }
                };
                let qid_type = if next_path == ROOT_PATH { QTDIR } else { 0 };
                let fresh = match self.table.alloc(
                    self.client_id,
                    entry(newfid, next_path, qid_type),
                ) {
                    Ok(fresh) => fresh,
                    Err(_) => return self.rerror(opened.tag),
                };
                let wqid = if wname.is_empty() { Vec::new() } else { alloc::vec![qid_for(next_path)] };
                self.reply(opened.tag, fresh.to_wire(), &Msg::Rwalk { wqid })
            }
            Msg::Topen { .. } => match self.resolve(&opened.handle) {
                Ok(source) => self.reply(
                    opened.tag,
                    opened.handle,
                    &Msg::Ropen { qid: qid_for(source.qid_path), iounit: 0 },
                ),
                Err(_) => self.rerror(opened.tag),
            },
            Msg::Tread { offset, count, .. } => match self.resolve(&opened.handle) {
                Ok(source) => {
                    let data = if source.qid_path == ROOT_PATH {
                        directory_bytes(offset, count)
                    } else {
                        file_bytes(source.qid_path, offset, count)
                    };
                    self.reply(opened.tag, opened.handle, &Msg::Rread { data })
                }
                Err(_) => self.rerror(opened.tag),
            },
            Msg::Tstat { .. } => match self.resolve(&opened.handle) {
                Ok(source) => {
                    let record = if source.qid_path == ROOT_PATH {
                        Some(stat_record(ROOT_PATH, b"#M"))
                    } else {
                        source
                            .qid_path
                            .checked_sub(ROOT_PATH + 1)
                            .map(|idx| idx as usize)
                            .filter(|idx| *idx < DATA_FILES)
                            .map(|idx| stat_record(source.qid_path, FILE_NAMES[idx]))
                    };
                    match record {
                        Some(stat) => self.reply(opened.tag, opened.handle, &Msg::Rstat { stat }),
                        None => self.rerror(opened.tag),
                    }
                }
                Err(_) => self.rerror(opened.tag),
            },
            Msg::Tclunk { .. } => {
                let parsed = match StatefulHandle::from_wire(&opened.handle) {
                    Ok(parsed) => parsed,
                    Err(_) => return self.rerror(opened.tag),
                };
                if self.table.clunk(self.client_id, &parsed).is_err() {
                    return self.rerror(opened.tag);
                }
                self.reply(opened.tag, [0u8; 8], &Msg::Rclunk)
            }
            _ => self.rerror(opened.tag),
        }
    }

    fn rerror(&mut self, tag: u16) -> Result<()> {
        self.reply(
            tag,
            [0u8; 8],
            &Msg::Rerror { ename: b"aeadloop: request refused".to_vec() },
        )
    }

    fn reply(&mut self, tag: u16, handle: [u8; 8], message: &Msg) -> Result<()> {
        let body = encoded_body(message)?;
        let typ = msg::opcode(message);
        let packet = {
            let session = self.session.as_mut().ok_or("aeadloop: no server session")?;
            session
                .seal(typ, tag, handle, &crate::chan::tlv::Exts::default(), &body, NOW)
                .map_err(|_| "aeadloop: server seal failed")?
        };
        self.link.push(false, &packet);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The raw duplex MntIo (used for negotiation and for the sealed adapters)
// ---------------------------------------------------------------------------

/// The raw in-kernel `MntIo` half over [`LoopLink`].  A write pushes the whole
/// message onto the client-to-server queue and drives the server
/// synchronously (the loopback's coroutine shape); a read drains the
/// server-to-client queue.
struct LoopMntIo {
    link: Arc<LoopLink>,
    server: Arc<Lock<LoopServer>>,
}

impl MntIo for LoopMntIo {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        let node = LockNode::new();
        let mut server = self.server.lock(&node);
        if server.session.is_some() {
            // Sealed: the packet must enter the server's framed queue.
            self.link.push(true, buf);
            server.handle_one()?;
        } else {
            // Plain negotiation: the frame is handled in place, never queued
            // (queueing it would desynchronise the sealed stream, which reads
            // whole sealed frames off the same queue).
            server.receive_plain(buf)?;
        }
        Ok(buf.len() as u64)
    }

    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        Ok(self.link.drain(true, buf) as u64)
    }
}

// ---------------------------------------------------------------------------
// The client sealed adapter: plaintext Mnt <-> sealed 9P2000.aead
// ---------------------------------------------------------------------------

/// The client's sealed state: the real `Session`, the client-fid to
/// server-handle map, and the tags awaiting a new-fid reply.
struct ClientSealed {
    session: Lock<Session>,
    handles: Lock<Vec<(u32, [u8; 8])>>,
    pending: Lock<Vec<(u16, u32)>>,
}

impl ClientSealed {
    fn new(session: Session) -> ClientSealed {
        ClientSealed {
            session: Lock::new("aeadloop-client-session", session),
            handles: Lock::new("aeadloop-client-handles", Vec::new()),
            pending: Lock::new("aeadloop-client-pending", Vec::new()),
        }
    }

    fn handle_for(&self, fid: u32) -> [u8; 8] {
        let node = LockNode::new();
        let guard = self.handles.lock(&node);
        guard
            .iter()
            .find(|(key, _)| *key == fid)
            .map(|(_, handle)| *handle)
            .unwrap_or([0u8; 8])
    }

    fn record_pending(&self, tag: u16, fid: u32) {
        let node = LockNode::new();
        self.pending.lock(&node).push((tag, fid));
    }

    fn take_pending(&self, tag: u16) -> Option<u32> {
        let node = LockNode::new();
        let mut guard = self.pending.lock(&node);
        let pos = guard.iter().position(|(pending, _)| *pending == tag)?;
        Some(guard.remove(pos).1)
    }

    fn set_handle(&self, fid: u32, handle: [u8; 8]) {
        let node = LockNode::new();
        let mut guard = self.handles.lock(&node);
        if let Some(slot) = guard.iter_mut().find(|(key, _)| *key == fid) {
            slot.1 = handle;
        } else {
            guard.push((fid, handle));
        }
    }
}

/// Rebuild the plaintext `9P2000` body a base client expects from a decoded
/// message.  The sealed wire pads a data message's data area to whole pages
/// (`msg::write_padded`), but `devmnt::Mnt` is a *plain* client and requires
/// `Rread.count == the remaining body` exactly, so an `Rread` is re-emitted
/// with just `count[4] data[count]`.  Every other reply encodes unpadded.
fn plain_reply_body(message: &Msg) -> Result<Vec<u8>> {
    match message {
        Msg::Rread { data } => {
            let mut body = Vec::with_capacity(4 + data.len());
            body.extend_from_slice(&(data.len() as u32).to_le_bytes());
            body.extend_from_slice(data);
            Ok(body)
        }
        _ => encoded_body(message),
    }
}

/// Pad a client plaintext data body to the whole-page shape the sealed wire
/// carries (the mirror of `msg::write_padded` for `Twrite`/`Tstream`).
fn pad_client_body(typ: u8, body: &[u8]) -> Vec<u8> {
    let params_len = match typ {
        wire::TWRITE => 16,
        wire::TSTREAM => 16,
        _ => return body.to_vec(),
    };
    if body.len() < params_len {
        return body.to_vec();
    }
    let mut out = Vec::with_capacity(body.len() + PAGE_LEN);
    out.extend_from_slice(&body[..params_len]);
    let data = &body[params_len..];
    out.extend_from_slice(data);
    let rem = data.len() % PAGE_LEN;
    if rem != 0 {
        out.resize(out.len() + (PAGE_LEN - rem), 0);
    }
    out
}

/// The plaintext-to-sealed write half the client `Mnt` sends through.
struct SealedWriteIo {
    inner: alloc::boxed::Box<dyn MntIo>,
    sealed: Arc<ClientSealed>,
}

impl MntIo for SealedWriteIo {
    fn write_msg(&mut self, buf: &[u8]) -> Result<u64> {
        if buf.len() < 7 {
            return Err("aeadloop: runt client frame");
        }
        let typ = buf[4];
        let tag = u16::from_le_bytes([buf[5], buf[6]]);
        let body = &buf[7..];
        let fid = if body.len() >= 4 {
            Some(u32::from_le_bytes([body[0], body[1], body[2], body[3]]))
        } else {
            None
        };
        let newfid = if typ == wire::TWALK && body.len() >= 8 {
            Some(u32::from_le_bytes([body[4], body[5], body[6], body[7]]))
        } else {
            None
        };
        let handle = fid.map(|f| self.sealed.handle_for(f)).unwrap_or([0u8; 8]);
        let wire_body = pad_client_body(typ, body);
        let packet = {
            let node = LockNode::new();
            let mut session = self.sealed.session.lock(&node);
            session
                .seal(typ, tag, handle, &crate::chan::tlv::Exts::default(), &wire_body, NOW)
                .map_err(|_| "aeadloop: client seal failed")?
        };
        if typ == wire::TATTACH {
            if let Some(fid) = fid {
                self.sealed.record_pending(tag, fid);
            }
        }
        if let Some(newfid) = newfid {
            self.sealed.record_pending(tag, newfid);
        }
        let written = self.inner.write_msg(&packet)? as usize;
        if written != packet.len() {
            return Err("aeadloop: short sealed write on the loopback");
        }
        Ok(buf.len() as u64)
    }

    fn read_some(&mut self, _buf: &mut [u8]) -> Result<u64> {
        Err("aeadloop: the sealed write half never reads")
    }
}

/// The sealed-to-plaintext read half the client `Mnt` receives through.
struct SealedReadIo {
    inner: alloc::boxed::Box<dyn MntIo>,
    sealed: Arc<ClientSealed>,
    plain: VecDeque<u8>,
}

impl SealedReadIo {
    /// Pull exactly one sealed reply off the wire, open it, and enqueue the
    /// reconstructed plaintext 9P frame (recording a new-fid reply's handle).
    fn pull_one(&mut self) -> Result<()> {
        let mut prefix = [0u8; 4];
        read_exact(&mut *self.inner, &mut prefix)?;
        let len = {
            let node = LockNode::new();
            let mut session = self.sealed.session.lock(&node);
            session
                .frame_len(&prefix)
                .map_err(|_| "aeadloop: bad sealed reply length")?
        };
        let mut packet = alloc::vec![0u8; len];
        packet[..4].copy_from_slice(&prefix);
        read_exact(&mut *self.inner, &mut packet[4..])?;
        let (typ, tag, handle, body) = {
            let node = LockNode::new();
            let mut session = self.sealed.session.lock(&node);
            let verified = session
                .receive(&mut packet, NOW)
                .map_err(|_| "aeadloop: client receive failed")?;
            let header = *verified.header();
            (header.typ, header.tag, header.fid_handle, verified.body().to_vec())
        };
        if let Some(fid) = self.sealed.take_pending(tag) {
            self.sealed.set_handle(fid, handle);
        }
        let message = msg::decode_body(typ, &body)?;
        let plain_body = plain_reply_body(&message)?;
        let mut frame = Vec::with_capacity(7 + plain_body.len());
        frame.extend_from_slice(&((7 + plain_body.len()) as u32).to_le_bytes());
        frame.push(typ);
        frame.extend_from_slice(&tag.to_le_bytes());
        frame.extend_from_slice(&plain_body);
        self.plain.extend(frame);
        Ok(())
    }
}

impl MntIo for SealedReadIo {
    fn write_msg(&mut self, _buf: &[u8]) -> Result<u64> {
        Err("aeadloop: the sealed read half never writes")
    }

    fn read_some(&mut self, buf: &mut [u8]) -> Result<u64> {
        while self.plain.is_empty() {
            self.pull_one()?;
        }
        let n = self.plain.len().min(buf.len());
        for slot in buf.iter_mut().take(n) {
            *slot = self.plain.pop_front().unwrap_or(0);
        }
        Ok(n as u64)
    }
}

// ---------------------------------------------------------------------------
// Build, publish and the #M registry arm
// ---------------------------------------------------------------------------

/// A live in-kernel loopback mount (only ever built behind `aeadloop=1`).
pub struct Loopback {
    root: Chan,
    table: Arc<FidTable>,
}

impl Loopback {
    /// A fresh fid-cloned `DEV_MNT` root chan for the `#M` registry arm.  The
    /// clone is a real server-side fid (a zero-name Twalk), never a bare dup,
    /// so `namec`'s `clunk_if_last` sweep can never retire the stored root.
    pub fn attach_root(&self, spec: &[u8]) -> Result<Chan> {
        if !spec.is_empty() {
            return Err("aeadloop: #M takes no specification");
        }
        crate::devmnt::clone_chan(&self.root)
    }

    pub(crate) fn table(&self) -> &Arc<FidTable> {
        &self.table
    }
}

/// Build the full loopback: the server (real `FidTable` + real `Session`)
/// over the duplex link, the client negotiation, the remaining plain
/// handshake, the sealed adapters, and the mounted `DEV_MNT` root chan.
fn build_with(secret: Key, table: Arc<FidTable>, client_id: u32) -> Result<Loopback> {
    let link = Arc::new(LoopLink::new());
    let server = Arc::new(Lock::new_ranked(
        "aeadloop-server",
        LockRank::DevTop,
        LoopServer::new(Arc::clone(&link), Arc::clone(&table), client_id, secret.clone()),
    ));

    let io_r: alloc::boxed::Box<dyn MntIo> = alloc::boxed::Box::new(LoopMntIo {
        link: Arc::clone(&link),
        server: Arc::clone(&server),
    });
    let io_w: alloc::boxed::Box<dyn MntIo> = alloc::boxed::Box::new(LoopMntIo {
        link: Arc::clone(&link),
        server: Arc::clone(&server),
    });

    let negotiated = Mnt::negotiate_dialect(io_r, io_w, 0, Dialect::Aead)?;
    let Negotiated::Aead(handoff) = negotiated else {
        return Err("aeadloop: an aead negotiation returned a plain session");
    };
    let msize = handoff.msize;
    let (mut handshake, mut raw_r, mut raw_w) =
        handoff.into_handshake(Role::Client, TransportConfig::default());

    plain_step(&mut *raw_w, &mut *raw_r, &mut handshake, &tauth_frame()?)?;
    plain_step(
        &mut *raw_w,
        &mut *raw_r,
        &mut handshake,
        &twrite_frame(CLIENT_EXCHANGE)?,
    )?;
    plain_step(&mut *raw_w, &mut *raw_r, &mut handshake, &tread_frame()?)?;
    let session = handshake.finish(secret);

    let sealed = Arc::new(ClientSealed::new(session));
    let sealed_r: alloc::boxed::Box<dyn MntIo> = alloc::boxed::Box::new(SealedReadIo {
        inner: raw_r,
        sealed: Arc::clone(&sealed),
        plain: VecDeque::new(),
    });
    let sealed_w: alloc::boxed::Box<dyn MntIo> =
        alloc::boxed::Box::new(SealedWriteIo { inner: raw_w, sealed });

    let mnt = Arc::new(Mnt::sealed_session(sealed_r, sealed_w, msize));
    let root = crate::devmnt::attach(&mnt, b"")?;
    Ok(Loopback { root, table })
}

/// Build the loopback when (and only when) the boot switch is active.
pub fn start(cmdline: &str) -> Option<Loopback> {
    if !parse_boot_switch(cmdline) {
        return None;
    }
    let table = Arc::new(FidTable::synthetic(domain_key(), 1));
    build_with(test_secret(), table, CLIENT_ID).ok()
}

/// The published loopback root (None on every default boot).
static ROOT: Lock<Option<Loopback>> = Lock::new("aeadloop-root", None);

/// Start and publish the loopback behind the boot switch; `true` when active.
pub fn maybe_start_from_cmdline(cmdline: &str) -> bool {
    let Some(loopback) = start(cmdline) else {
        return false;
    };
    let node = LockNode::new();
    *ROOT.lock(&node) = Some(loopback);
    true
}

/// The `#M` registry arm: a fresh cloned root chan when active, `Err` when not
/// (the donor `Enodev` shape, `namec: unknown # device`).
pub fn attach_root(spec: &[u8]) -> Result<Chan> {
    if !spec.is_empty() {
        return Err("aeadloop: #M takes no specification");
    }
    let root = {
        let node = LockNode::new();
        let guard = ROOT.lock(&node);
        match guard.as_ref() {
            Some(loopback) => loopback.root.dup(),
            None => return Err("namec: unknown # device"),
        }
    };
    crate::devmnt::clone_chan(&root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dat::OREAD;

    fn expected_byte(file: usize, off: usize) -> u8 {
        file_byte(file, off)
    }

    fn expected_file(file: usize) -> Vec<u8> {
        (0..FILE_LEN).map(|off| expected_byte(file, off)).collect()
    }

    fn active_fids(table: &FidTable) -> u64 {
        (0..table.shard_count() as u8)
            .filter_map(|shard| table.bitmap_lens(shard))
            .map(|(active, _reclaimed)| active)
            .sum()
    }

    fn read_file(root: &Chan, idx: usize) -> Vec<u8> {
        let name = alloc::format!("data{idx}");
        let walked = crate::namec::walk_one(root, name.as_bytes()).expect("walk data file");
        assert_eq!(
            walked.qid.path,
            ROOT_PATH + 1 + idx as u64,
            "the walked file carries its deterministic qid path"
        );
        let mut opened = crate::namec::open_chan(&walked, OREAD).expect("open data file");
        let mut buf = alloc::vec![0u8; FILE_LEN];
        let n = crate::devmnt::read(&mut opened, &mut buf, 0).expect("read data file") as usize;
        assert_eq!(n, FILE_LEN, "the whole deterministic file is returned");
        // The syscall shape: the opened chan is the surviving reference (the
        // walked chan is dropped at install), so its close is the fid's last
        // reference and dispatches the Tclunk.
        drop(walked);
        crate::namec::close_chan(&opened);
        drop(opened);
        buf.truncate(n);
        buf
    }

    fn global_guard() -> std::sync::MutexGuard<'static, ()> {
        static G: std::sync::Mutex<()> = std::sync::Mutex::new(());
        G.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    fn reset_global() {
        let node = LockNode::new();
        *ROOT.lock(&node) = None;
    }

    #[test]
    fn aead_loop_parse_boot_switch_is_exact_token() {
        assert!(!parse_boot_switch("console=ttyS0"));
        assert!(!parse_boot_switch(""));
        assert!(!parse_boot_switch("aeadloop=0"));
        assert!(!parse_boot_switch("aeadloop=11"));
        assert!(!parse_boot_switch("xaeadloop=1"));
        assert!(!parse_boot_switch("aeadloop=1x"));
        assert!(parse_boot_switch("aeadloop=1"));
        assert!(parse_boot_switch("console=ttyS0 aeadloop=1 root=/dev/vda"));
    }

    #[test]
    fn aead_loop_dormant_default_boot_exposes_nothing() {
        let _g = global_guard();
        reset_global();
        assert!(
            !maybe_start_from_cmdline("console=ttyS0 root=/dev/vda"),
            "a default boot must not start the loopback server"
        );
        assert_eq!(
            attach_root(b"").err(),
            Some("namec: unknown # device"),
            "the dormant #M attach is the donor Enodev shape"
        );
        assert!(start("console=ttyS0").is_none());
    }

    #[test]
    fn aead_loop_enabled_serves_deterministic_files() {
        let _g = global_guard();
        reset_global();
        assert!(maybe_start_from_cmdline("aeadloop=1"), "the switch activates the loopback");
        let root = attach_root(b"").expect("the active #M attach returns the root chan");
        assert_eq!(root.qid.typ & QTDIR, QTDIR, "the root is a directory");
        for idx in 0..DATA_FILES {
            let got = read_file(&root, idx);
            assert_eq!(got, expected_file(idx), "file {idx} reads back its exact pattern");
        }
        drop(root);
        reset_global();
    }

    #[test]
    fn aead_loop_root_enumerates_and_stats() {
        let _g = global_guard();
        reset_global();
        assert!(maybe_start_from_cmdline("aeadloop=1"), "the switch activates the loopback");
        let root = attach_root(b"").expect("the active #M attach returns the root chan");

        // Tstat of the root: a stat(5) record carrying its four strings.
        let mut sb = [0u8; 512];
        let n = crate::devmnt::stat(&root, &mut sb).expect("stat the loopback root") as usize;
        assert!(n > 41, "the root stat is a stat(5) record with four strings");

        // A directory read enumerates data0..data7 (what `ls '#M'` prints).
        let mut opened = crate::namec::open_chan(&root, OREAD).expect("open the root directory");
        let mut buf = [0u8; 1024];
        let mut listing = Vec::new();
        loop {
            let offset = listing.len() as u64;
            let got =
                crate::devmnt::read(&mut opened, &mut buf, offset).expect("read the root directory")
                    as usize;
            if got == 0 {
                break;
            }
            listing.extend_from_slice(&buf[..got]);
        }
        for idx in 0..DATA_FILES {
            let name = alloc::format!("data{idx}");
            assert!(
                listing.windows(name.len()).any(|window| window == name.as_bytes()),
                "the root listing names {name}"
            );
        }

        drop(root);
        crate::namec::close_chan(&opened);
        drop(opened);
        reset_global();
    }

    #[test]
    fn aead_loop_two_threads_share_one_table_without_leaks() {
        let table = Arc::new(FidTable::synthetic(domain_key(), 1));
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let mut threads = Vec::new();
        for client_id in [1u32, 2u32] {
            let shared = Arc::clone(&table);
            let sync = Arc::clone(&barrier);
            threads.push(std::thread::spawn(move || {
                let loopback = build_with(test_secret(), Arc::clone(&shared), client_id)
                    .expect("build loopback");
                // Both loopbacks exist before either drives traffic, so the
                // shared table's baseline is the same on both sides.
                sync.wait();
                let baseline = active_fids(&shared);
                // The baseline capture itself must be quiet too: without this
                // barrier a fast thread starts its read traffic (allocating a
                // scratch fid) while the slower thread is still reading the
                // baseline, so the baseline counts a transient fid and the
                // final compare fails 3 != 2. Three waits, one reusable
                // barrier: both built, both baselined, both finished.
                sync.wait();
                for round in 0..25 {
                    for idx in 0..DATA_FILES {
                        let got = read_file(&loopback.root, idx);
                        assert_eq!(
                            got,
                            expected_file(idx),
                            "client {client_id} round {round} file {idx} payload"
                        );
                    }
                }
                // No thread is mid-operation after the second barrier, so a
                // shared counter read here is exact: every clunk has landed.
                sync.wait();
                assert_eq!(
                    active_fids(&shared),
                    baseline,
                    "client {client_id} leaked no fids across 25 rounds"
                );
            }));
        }
        for thread in threads {
            thread.join().expect("concurrent loopback thread");
        }

        // The table keeps the owning-client check: a handle issued to client 1
        // never resolves for client 2 (D-08).
        let handle = table
            .alloc(1, entry(1, ROOT_PATH, QTDIR))
            .expect("alloc for client 1");
        assert!(
            table.resolve(2, &handle).is_err(),
            "a client-1 handle is refused to client 2"
        );
        assert!(table.resolve(1, &handle).is_ok());
        table.clunk(1, &handle).expect("clunk the probe handle");
    }
}
