//! devproc — the per-process device (04-08's note slice, extended by
//! 05-09: D-09's ps(1) data source + init's ctl write).  The tree:
//! /proc (the root, a directory) → /proc/<pid>/ (a directory per live
//! pid) → the per-pid files note (04-08), status, args, and ctl.
//!
//! 04-08 scope (UNCHANGED — Pitfall 5): /proc/<pid>/note writes append
//! the message bytes (bounded to ERRMAX with a NUL-terminated copy) to
//! the addressed process's note queue via the injected post hook — the
//! write resolves the addressed pid and no other target (T-04-17: no
//! broadcast, no group semantics).  libc's `postnote(PNPROC, pid, msg)`
//! works unmodified (there is no postnote syscall — the note arrives by
//! a device write); delivery rides the 04-08 return-to-EL0 boundary.
//!
//! 05-09 scope (D-09, RESEARCH Pitfall 5 — the ps(1) contract): ps does
//! `chdir("/proc")`, `open(".")`, and dirreadall — a DIRECTORY read of
//! the root carrying one stat(5) wire entry per live pid, each named by
//! its decimal pid (sys/src/cmd/ps.c:33-45) — then reads
//! `<pid>/status` and `<pid>/args` per entry:
//!   * status emits ≥12 whitespace-separated fields in devproc.c's
//!     order (ps.c:80-87's tokenizer map: [0] name, [1] user, [2]
//!     state, [3..=8] six time fields, [9] memory, [10] basepri,
//!     [11] pri).  The field COUNT and order are the contract ps
//!     parses; r9 has no analog for the time/memory fields this era,
//!     so they are zero-filled (the plan's sanction), while name/state
//!     are real and basepri/pri report the recorded ctl value.
//!   * args serves the argv the kernel recorded AT EXEC (proc.rs's
//!     Proc.argv — truthful, never reconstructed from user memory at
//!     read time, T-05-17), space-joined per the donor's procargs
//!     (the %q quoting is a no-measured-caller era limit).  A pid that
//!     never exec'd reads EMPTY (ps prints just the name — ps.c:135's
//!     Noargs arm), never garbage.
//!   * ctl accepts `pri N` writes (init.c:52's `#p/N/ctl` write of
//!     "pri 10") — parsed and RECORDED (accepting + storing is the
//!     honest degradation the plan names: no scheduler priority exists
//!     this era).  An unknown ctl verb errors LOUDLY.
//!
//! The proc-table bookkeeping lives in the arch crate, so the data
//! crosses via injected hooks — the same dependency inversion as the
//! 04-08 note-post hook (and devcons's `Console::set_uart`): the arch
//! side registers a ProcHooks set once at boot, and the reads/writes
//! call it.
//!
//! Like devroot, the dat.rs `Dev` trait is NOT implemented here (its
//! skeleton is non-constructible as written — see devroot.rs's header);
//! the device operations the plan names (attach/walk/open/read/write/
//! stat) are free functions, and trait conformance lands with the
//! devtab dispatch era.
//!
//! Qid encoding (05-09 RE-ENCODED — the never-collide rule holds): the
//! root is path 0; a pid's directory is `pid*8`; its files are
//! `pid*8 + slot` with note = 1, status = 2, args = 3, ctl = 4.  Pids
//! start at 1 and slots are < 8, so the kinds never collide (path/8 is
//! the pid, path%8 the kind).  The 04-08 `pid*2` / `pid*2+1` encoding
//! is superseded: qids are transient kernel internals (vers pinned 0),
//! so the re-encoding changes no observable behavior — the note path's
//! walk+write semantics are byte-identical (Pitfall 5).

use crate::dat::{Chan, Qid, DEV_PROC, OREAD, OWRITE};
use crate::mcslock::{Lock, LockNode};
use crate::Result;

/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// The per-pid file slots (the module header's qid encoding).
const SLOT_NOTE: u64 = 1;
const SLOT_STATUS: u64 = 2;
const SLOT_ARGS: u64 = 3;
const SLOT_CTL: u64 = 4;

/// The pid-dir listing's file table (name, slot, mode) — alphabetical,
/// the donor devproc.c dirtab's ordering style.  note is 04-08's
/// write-only file; ctl is 05-09's write-only control; status/args are
/// read-only (ps's data).
const PID_FILES: [(&[u8], u64, u32); 4] = [
    (b"args", SLOT_ARGS, 0o444),
    (b"ctl", SLOT_CTL, 0o666),
    (b"note", SLOT_NOTE, 0o666),
    (b"status", SLOT_STATUS, 0o444),
];

/// The arch-side note-post hook signature (04-08): validate the pid,
/// append to that process's note queue.
type PostHook = fn(u64, &[u8]) -> Result<()>;

/// The arch-side note-post hook.  Registered once (single writer, pre-EL0;
/// read from syscall context thereafter) — the mcslock idiom guards the
/// one-time publish, mirroring devroot's ROOT_IMG.
static POST_HOOK: Lock<Option<PostHook>> = Lock::new("devproc", None);

/// Register the arch-side note-post hook.  Called once during boot before
/// any user process exists; a second registration is a boot bug.
pub fn set_post_hook(hook: PostHook) {
    let node = LockNode::new();
    let mut guard = POST_HOOK.lock(&node);
    assert!(guard.is_none(), "devproc: post hook registered twice");
    *guard = Some(hook);
}

/// The per-pid status/args data the arch side serves (05-09): the proc
/// name (KNAMELEN-bounded bytes), the donor-statename state string, the
/// exec-recorded argv (NUL-joined; empty for a never-exec'd proc), and
/// the recorded ctl priority.
pub struct ProcInfo {
    pub name: alloc::vec::Vec<u8>,
    pub state: &'static str,
    pub args: alloc::vec::Vec<u8>,
    pub pri: u32,
}

/// The arch-side proc-table hooks (05-09 — the note-post hook's
/// siblings): `list` returns the LIVE (non-Moribund) pids (the root
/// listing's content); `info` returns a pid's status/args data (None =
/// no such live process); `set_pri` records a ctl priority write.
#[derive(Clone, Copy)]
pub struct ProcHooks {
    pub list: fn() -> alloc::vec::Vec<u64>,
    pub info: fn(u64) -> Option<ProcInfo>,
    pub set_pri: fn(u64, u32) -> Result<()>,
}

/// The arch-side proc hooks.  Same one-time-publish idiom as POST_HOOK.
static PROC_HOOKS: Lock<Option<ProcHooks>> = Lock::new("devproc-hooks", None);

/// Register the arch-side proc-table hooks.  Called once during boot
/// before any user process exists; a second registration is a boot bug.
pub fn set_proc_hooks(hooks: ProcHooks) {
    let node = LockNode::new();
    let mut guard = PROC_HOOKS.lock(&node);
    assert!(guard.is_none(), "devproc: proc hooks registered twice");
    *guard = Some(hooks);
}

/// The registered hooks, or the loud not-registered error (a pre-exec
/// read/write is a boot-orchestration bug — the hooks land before the
/// first exec).
fn proc_hooks() -> Result<ProcHooks> {
    let node = LockNode::new();
    (*PROC_HOOKS.lock(&node)).ok_or("devproc: proc hooks not registered")
}

/// The Qid path encoding (the module header's scheme): the root is 0; a
/// pid's directory is `pid*8`; a pid's file is `pid*8 + slot`.
fn dir_qid(pid: u64) -> Qid {
    Qid { path: pid * 8, vers: 0, typ: QTDIR }
}
fn file_qid(pid: u64, slot: u64) -> Qid {
    Qid { path: pid * 8 + slot, vers: 0, typ: QTFILE }
}

/// A pid rendered as decimal ASCII (the root listing's entry names and
/// the pid dir's stat name).
fn dec(mut v: u64) -> alloc::vec::Vec<u8> {
    let mut nbuf = alloc::vec::Vec::new();
    if v == 0 {
        nbuf.push(b'0');
    }
    while v > 0 {
        nbuf.insert(0, b'0' + (v % 10) as u8);
        v /= 10;
    }
    nbuf
}

/// attach: the `/proc` root chan (r9 serves one proc tree this era, so the
/// attach spec is empty — the same shape as devroot's attach_root).
pub fn attach() -> Chan {
    Chan::new(DEV_PROC, Qid { path: 0, vers: 0, typ: QTDIR }, 0)
}

/// Walk a single name component from `c`.  Under the root the component
/// must be all ASCII digits (a pid); under a pid directory the component
/// must be one of the PID_FILES names (note/status/args/ctl).  Anything
/// else is a loud walk error — the tree has no other names.  Pid
/// EXISTENCE is checked at read/write time (the walk is the syntactic
/// half; the data ops are the semantic half).
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR {
        return Err("devproc: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devproc: walk takes a single name component");
    }
    if c.qid.path == 0 {
        // Root → per-process directory: the component must parse as a pid.
        if !name.iter().all(|b| b.is_ascii_digit()) {
            return Err("devproc: not a numeric process name");
        }
        let mut pid: u64 = 0;
        for &b in name {
            pid = pid.checked_mul(10).and_then(|p| p.checked_add(u64::from(b - b'0')))
                .ok_or("devproc: process name overflows")?;
        }
        if pid == 0 {
            return Err("devproc: not a numeric process name");
        }
        let mut nc = Chan::new(DEV_PROC, dir_qid(pid), 0);
        nc.path = c.path.clone();
        nc.path.push(b'/');
        nc.path.extend_from_slice(name);
        Ok(nc)
    } else {
        // Pid directory → one of the per-pid files, and nothing else.
        let Some((_, slot, _)) = PID_FILES.iter().find(|(n, _, _)| *n == name) else {
            return Err("devproc: unknown per-process name");
        };
        let pid = c.qid.path / 8;
        let mut nc = Chan::new(DEV_PROC, file_qid(pid, *slot), 0);
        nc.path = c.path.clone();
        nc.path.push(b'/');
        nc.path.extend_from_slice(name);
        Ok(nc)
    }
}

/// Open `c`: the directories open OREAD (ps's `open(".")` + a pid dir's
/// listing); status/args open OREAD; note/ctl open OWRITE (the 04-08
/// note discipline, extended to ctl).  Every other mode errors loudly —
/// the scope guard is the open path.
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    if c.qid.typ == QTDIR {
        if mode != OREAD {
            return Err("devproc: directories open OREAD only");
        }
    } else {
        let slot = c.qid.path % 8;
        let ok = match slot {
            SLOT_STATUS | SLOT_ARGS => mode == OREAD,
            SLOT_NOTE | SLOT_CTL => mode == OWRITE,
            _ => false,
        };
        if !ok {
            return Err("devproc: bad per-process file mode");
        }
    }
    let mut nc = Chan::new(c.dev, c.qid, mode);
    nc.path = c.path.clone();
    Ok(nc)
}

/// stat of a devproc chan (05-04 Task 3 — the namec stat dispatch's
/// devproc leg; the wire shape is the shared StatWriter).  Directories
/// stat with DMDIR|0555; the per-pid files stat by slot (note/ctl 0666,
/// status/args 0444) with the status/args lengths computed from the
/// live data (they vary with the name/argv).
pub fn stat(c: &Chan, sb: &mut [u8]) -> Result<u64> {
    if c.qid.path == 0 {
        return write_stat_record(sb, b"proc", c.qid, 0x8000_0000 | 0o555, 0);
    }
    if c.qid.typ == QTDIR {
        // The per-pid directory's name is the pid (decimal).
        let pid = c.qid.path / 8;
        return write_stat_record(sb, &dec(pid), c.qid, 0x8000_0000 | 0o555, 0);
    }
    let pid = c.qid.path / 8;
    let slot = c.qid.path % 8;
    let Some((name, _, mode)) = PID_FILES.iter().find(|(_, s, _)| *s == slot) else {
        return Err("devproc: stat of unknown file");
    };
    let length = match slot {
        SLOT_STATUS => status_line(pid)?.len() as u64,
        SLOT_ARGS => args_text(pid)?.len() as u64,
        _ => 0,
    };
    write_stat_record(sb, name, c.qid, *mode, length)
}

/// One stat(5) wire record (devcons's shape; uid/gid/muid glenda per
/// D-02).  FOUR strings — name, uid, gid, muid: libc's dirreadall
/// (ps's directory read) runs statcheck on every record, which walks
/// FOUR length-prefixed strings and rejects a record that ends early
/// (convM2D.c) — a three-string record fails the whole directory read
/// with "empty directory" (the 05-09 gate's catch).
fn write_stat_record(
    sb: &mut [u8],
    name: &[u8],
    qid: crate::dat::Qid,
    mode: u32,
    length: u64,
) -> Result<u64> {
    let need = 41 + (2 + name.len()) + 3 * (2 + 6);
    if sb.len() < need {
        return Err("devproc: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type: unused this era
    w.u32(crate::dat::DEV_PROC as u32);
    w.u8(qid.typ);
    w.u32(qid.vers);
    w.u64(qid.path);
    w.u32(mode);
    w.u32(0); // atime
    w.u32(0); // mtime
    w.u64(length);
    w.string(name);
    w.string(b"glenda");
    w.string(b"glenda");
    w.string(b"glenda"); // muid — the fourth string statcheck walks
    Ok(w.finish())
}

/// The status line's bytes (05-09; shared by the status read and stat):
/// ≥12 whitespace-separated fields in devproc.c's order — name, user,
/// state, six zero-filled time fields, zero-filled memory (KB), basepri,
/// pri (ps.c's tokenizer contract; the zero-filled fields are the
/// plan-sanctioned no-analog arms).  The line ends with a newline
/// (whitespace to the tokenizer).
fn status_line(pid: u64) -> Result<alloc::vec::Vec<u8>> {
    let hooks = proc_hooks()?;
    let Some(info) = (hooks.info)(pid) else {
        return Err("devproc: no such process");
    };
    let mut line = alloc::vec::Vec::new();
    line.extend_from_slice(&info.name);
    line.extend_from_slice(b" glenda ");
    line.extend_from_slice(info.state.as_bytes());
    line.extend_from_slice(b" 0 0 0 0 0 0 0 ");
    line.extend_from_slice(&dec(u64::from(info.pri)));
    line.push(b' ');
    line.extend_from_slice(&dec(u64::from(info.pri)));
    line.push(b'\n');
    Ok(line)
}

/// The args file's bytes (05-09): the exec-recorded argv (NUL-joined on
/// the proc) space-joined at the read — the donor procargs's shape (the
/// %q quoting is a no-measured-caller era limit).  Empty for a
/// never-exec'd pid (ps.c's Noargs arm prints just the name).
fn args_text(pid: u64) -> Result<alloc::vec::Vec<u8>> {
    let hooks = proc_hooks()?;
    let Some(info) = (hooks.info)(pid) else {
        return Err("devproc: no such process");
    };
    let mut out = alloc::vec::Vec::new();
    for (i, arg) in info.args.split(|&b| b == 0).filter(|a| !a.is_empty()).enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        out.extend_from_slice(arg);
    }
    Ok(out)
}

/// Serve `content` at `off` into `buf` (the pread convention; EOF = 0 at
/// or past the end) — the dir listings and file contents share it.
fn serve_at(content: &[u8], buf: &mut [u8], off: u64) -> u64 {
    let start = (off as usize).min(content.len());
    let avail = &content[start..];
    let n = avail.len().min(buf.len());
    buf[..n].copy_from_slice(&avail[..n]);
    n as u64
}

/// Read from a devproc chan (05-09).  The root dir serves one stat(5)
/// wire entry per LIVE pid (named by the decimal pid — ps's dirreadall
/// of `open(".")`); a pid dir serves its PID_FILES listing; status
/// serves the ≥12-field line; args serves the exec-recorded argv.  note
/// is write-only (04-08) and ctl is write-only — reads of either error
/// loudly.  `offset` u64::MAX uses + advances the chan's streaming
/// offset (the pread convention).
pub fn read(c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64> {
    let off = if offset == u64::MAX { c.offset } else { offset };
    let n = if c.qid.path == 0 {
        // The root listing: one entry per live pid (the list hook),
        // rebuilt per call (the table may change between reads).
        let hooks = proc_hooks()?;
        let mut listing = alloc::vec::Vec::new();
        for pid in (hooks.list)() {
            let start = listing.len();
            listing.resize(start + 41 + (2 + dec(pid).len()) + 3 * (2 + 6), 0);
            write_stat_record(
                &mut listing[start..],
                &dec(pid),
                dir_qid(pid),
                0x8000_0000 | 0o555,
                0,
            )?;
        }
        serve_at(&listing, buf, off)
    } else if c.qid.typ == QTDIR {
        // The pid dir's file listing (args/ctl/note/status).
        let pid = c.qid.path / 8;
        let mut listing = alloc::vec::Vec::new();
        for (name, slot, mode) in PID_FILES {
            let length = match slot {
                SLOT_STATUS => status_line(pid)?.len() as u64,
                SLOT_ARGS => args_text(pid)?.len() as u64,
                _ => 0,
            };
            let start = listing.len();
            listing.resize(start + 41 + (2 + name.len()) + 3 * (2 + 6), 0);
            write_stat_record(&mut listing[start..], name, file_qid(pid, slot), mode, length)?;
        }
        serve_at(&listing, buf, off)
    } else {
        match c.qid.path % 8 {
            SLOT_STATUS => serve_at(&status_line(c.qid.path / 8)?, buf, off),
            SLOT_ARGS => serve_at(&args_text(c.qid.path / 8)?, buf, off),
            SLOT_NOTE => return Err("devproc: note files are write-only"),
            SLOT_CTL => return Err("devproc: ctl files are write-only"),
            _ => return Err("devproc: read of unknown file"),
        }
    };
    if offset == u64::MAX {
        c.offset += n;
    }
    Ok(n)
}

/// Write `buf` to the chan `c` names (04-08's note path + 05-09's ctl):
/// decode the pid and file kind from the chan's qid.
///   * note (kind 1): bound the message to ERRMAX (the note buffer's
///     size, NUL-padded) and hand it to the arch-side post hook —
///     unchanged from 04-08 (Pitfall 5).
///   * ctl (kind 4): parse `pri N` and record it via the arch-side
///     hooks (accepting + storing is the plan's honest degradation — no
///     scheduler priority exists this era); an unknown verb errors
///     loudly.
///
/// Returns the byte count taken.
pub fn write(c: &mut Chan, buf: &[u8]) -> Result<u64> {
    if c.qid.typ == QTDIR {
        return Err("devproc: write of a directory");
    }
    if c.mode != OWRITE {
        return Err("devproc: per-process file not open for writing");
    }
    let pid = c.qid.path / 8;
    match c.qid.path % 8 {
        SLOT_NOTE => {
            let hook = {
                let node = LockNode::new();
                *POST_HOOK.lock(&node)
            };
            let Some(hook) = hook else {
                return Err("devproc: note post hook not registered");
            };
            hook(pid, buf)?;
            Ok(buf.len() as u64)
        }
        SLOT_CTL => {
            // 05-09: the "pri N" control (init.c:52's write of "pri 10"
            // to #p/<getpid>/ctl).  Parse + RECORD via the arch hooks —
            // accepting and storing is the plan's honest degradation (no
            // scheduler priority exists this era).  An unknown verb (or
            // a malformed write) errors LOUDLY (the donor procctl's
            // Ebadarg shape).
            let text = core::str::from_utf8(buf).map_err(|_| "devproc: ctl write is not utf8")?;
            let mut it = text.split_whitespace();
            let (verb, arg, extra) = (it.next(), it.next(), it.next());
            if verb != Some("pri") {
                return Err("devproc: unknown ctl verb");
            }
            if extra.is_some() {
                return Err("devproc: malformed ctl write");
            }
            let Some(arg) = arg else {
                return Err("devproc: ctl pri needs a decimal number");
            };
            let v: u32 = arg.parse().map_err(|_| "devproc: ctl pri needs a decimal number")?;
            let hooks = proc_hooks()?;
            (hooks.set_pri)(pid, v)?;
            Ok(buf.len() as u64)
        }
        _ => Err("devproc: write of a read-only file"),
    }
}
