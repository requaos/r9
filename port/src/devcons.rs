use crate::Result;
use crate::mcslock::{Lock, LockNode};
use core::fmt;

const fn ctrl(b: u8) -> u8 {
    b - b'@'
}

#[allow(dead_code)]
const BACKSPACE: u8 = ctrl(b'H');
#[allow(dead_code)]
const DELETE: u8 = 0x7F;
#[allow(dead_code)]
const CTLD: u8 = ctrl(b'D');
#[allow(dead_code)]
const CTLP: u8 = ctrl(b'P');
#[allow(dead_code)]
const CTLU: u8 = ctrl(b'U');

pub trait Uart {
    fn putb(&self, b: u8);
}

static CONS: Lock<Option<&'static mut dyn Uart>> = Lock::new("cons", None);

/// Console is what should be used in almost all cases, as it ensures threadsafe
/// use of the console.
pub struct Console;

impl Console {
    pub fn set_uart<F>(uart_fn: F)
    where
        F: FnOnce() -> Result<&'static mut dyn Uart>,
    {
        let node = LockNode::new();
        let mut cons = CONS.lock(&node);
        *cons = uart_fn().ok();
    }

    pub fn putstr(&mut self, s: &str) {
        // XXX: Just for testing.

        let node = LockNode::new();
        let mut uart_guard = CONS.lock(&node);
        if let Some(uart) = uart_guard.as_deref_mut() {
            for b in s.bytes() {
                putb(uart, b);
            }
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.putstr(s);
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    // XXX: Just for testing.
    use fmt::Write;
    let mut cons: Console = Console {};
    cons.write_fmt(args).unwrap();
}

/// Write bytes VERBATIM to the console uart under the CONS MCS lock — the
/// devcons DEVICE write path, called from syscall context (04-06, D-15).
/// Unlike the kernel print path there is no `\n`→`\r\n` cooking: device
/// writes are byte-exact (the 9front devcons conswrite shape), which is
/// what lets user-visible marker strings originate from the EL0 binary's
/// own pwrite path rather than from a kernel print on its behalf.
///
/// Locking (04-RESEARCH.md Pitfall 3): this shares the kernel print path's
/// lock, so no IRQ-context code may ever take it once syscalls can write —
/// the timer tick's println is cfg(not(dev_exec))-gated off for exactly
/// that reason.  Returns the number of bytes written.
pub fn write_bytes(buf: &[u8]) -> usize {
    let node = LockNode::new();
    let mut uart_guard = CONS.lock(&node);
    let Some(uart) = uart_guard.as_deref_mut() else {
        // Console never initialised (pre-devcons::init) — the write goes
        // nowhere, but a syscall-era caller always has a registered uart.
        return 0;
    };
    for &b in buf {
        uart.putb(b);
    }
    buf.len()
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($args:tt)*) => {{
        $crate::devcons::print(format_args!($($args)*))
    }};
}

fn putb(uart: &mut dyn Uart, b: u8) {
    if b == b'\n' {
        uart.putb(b'\r');
    } else if b == BACKSPACE {
        uart.putb(b);
        uart.putb(b' ');
    }
    uart.putb(b);
}

// ---------------------------------------------------------------------------
// The registry-facing device surface (05-04, D-03) — attach/walk/open/
// read/write/stat as free functions, the established port idiom
// (devroot.rs's header explains why the dat.rs Dev trait is not
// implemented).  Task 2 landed the minimal cons surface; Task 3 grows the
// file set the boot chain touches: cons (opencons's three opens), null,
// user (init's readenv("#c/user")), hostowner (glenda's guarded write).
//
// Qid encoding (collision-free): root = 0 (QTDIR), cons = 1 (QTFILE — the
// fd 0/1/2 preopen convention, proc.rs), null = 2, user = 3, hostowner = 4.
// ---------------------------------------------------------------------------

/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// Qid path values for the devcons files.
pub(crate) const QID_ROOT: u64 = 0;
pub(crate) const QID_CONS: u64 = 1;
pub(crate) const QID_NULL: u64 = 2;
pub(crate) const QID_USER: u64 = 3;
pub(crate) const QID_HOSTOWNER: u64 = 4;
/// 05-07 Task 2: the time file (settime's OWRITE target).
pub(crate) const QID_TIME: u64 = 5;

/// attach: the devcons root chan (one console tree this era — the attach
/// spec is empty, the devroot attach precedent).
pub fn attach() -> crate::dat::Chan {
    crate::dat::Chan::new(
        crate::dat::DEV_CONS,
        crate::dat::Qid { path: QID_ROOT, vers: 0, typ: QTDIR },
        0,
    )
}

/// The hostowner string (D-02's sane-default contract): the glenda no-op's
/// guarded `#c/hostowner` write stores here; reads return the stored bytes
/// (empty until the first write — EOF).  The mcslock static-publish idiom;
/// syscall context only.
static HOSTOWNER: Lock<alloc::vec::Vec<u8>> = Lock::new("hostowner", alloc::vec::Vec::new());

/// 05-07 Task 2: the `#c/time` file joins the attach surface (the donor
/// devcons.c Qtime).  settime's boot flow opens it OWRITE and writes the
/// root's atime as decimal seconds — r9 has NO RTC this phase, so the
/// semantics are accept-and-record-the-attempt: the write STORES the
/// bytes (comment-documented, never a failure — a failure would only warn
/// in settime, but silence here keeps the boot log clean for marker
/// assertions) and a later read returns the recorded value (or a
/// plausible fixed epoch before any write — the donor's time file always
/// has an answer).
static TIME: Lock<alloc::vec::Vec<u8>> = Lock::new("time", alloc::vec::Vec::new());

/// The pre-write answer for a `#c/time` read (a plausible fixed epoch —
/// 2023-11-14; the value is informational only until settime's write).
const TIME_DEFAULT: &[u8] = b"1700000000\n";

/// The per-file static metadata (name, qid, mode) — the walk table and the
/// directory listing share it.  lengths are computed at read/stat time
/// (hostowner's and time's vary).
const FILES: [(&[u8], u64, u32); 5] = [
    (b"cons", QID_CONS, 0o666),
    (b"null", QID_NULL, 0o666),
    (b"user", QID_USER, 0o444),
    (b"hostowner", QID_HOSTOWNER, 0o664),
    (b"time", QID_TIME, 0o664),
];

/// Walk a single name component from the devcons root.
pub fn walk(c: &crate::dat::Chan, name: &[u8]) -> crate::Result<crate::dat::Chan> {
    if c.qid.typ != QTDIR || c.qid.path != QID_ROOT {
        return Err("devcons: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devcons: walk takes a single name component");
    }
    let Some((_, qid, _)) = FILES.iter().find(|(n, _, _)| *n == name) else {
        return Err("devcons: name not found");
    };
    let mut nc =
        crate::dat::Chan::new(crate::dat::DEV_CONS, crate::dat::Qid { path: *qid, vers: 0, typ: QTFILE }, 0);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}

/// Open a walked devcons chan.  The root dir opens OREAD only; user is
/// read-only (0444); cons/null/hostowner open for read or write.
pub fn open(c: &crate::dat::Chan, mode: u32) -> crate::Result<crate::dat::Chan> {
    if c.qid.typ == QTDIR && mode != crate::dat::OREAD {
        return Err("devcons: directory opens OREAD only");
    }
    if c.qid.path == QID_USER && mode != crate::dat::OREAD {
        return Err("devcons: user is read-only");
    }
    let mut nc = crate::dat::Chan::new(c.dev, c.qid, mode);
    nc.path = c.path.clone();
    Ok(nc)
}

/// The hostowner file's current length (for stat/dir listings).
fn hostowner_len() -> u64 {
    let node = LockNode::new();
    HOSTOWNER.lock(&node).len() as u64
}

/// The time file's current length (05-07): the recorded write's length,
/// or the fixed default's before any write.
fn time_len() -> u64 {
    let node = LockNode::new();
    let stored = TIME.lock(&node);
    if stored.is_empty() { TIME_DEFAULT.len() as u64 } else { stored.len() as u64 }
}

/// The "user" file's content (D-02's sane default: glenda — the auth stub
/// never runs a real handshake this phase).  NO trailing newline (the
/// donor's Quser read is `readstr(up->user)` — the bare name): init's
/// readenv("#c/user") feeds newns's `$user` → `/usr/$user` verbatim, so a
/// newline would poison the home path.
const USER_CONTENT: &[u8] = b"glenda";

/// One stat(5) wire record for a file (the shared StatWriter shape; the
/// 41-byte fixed part + three length-prefixed strings).
fn write_stat_record(sb: &mut [u8], name: &[u8], qid: u64, mode: u32, length: u64) -> crate::Result<u64> {
    let need = 41 + (2 + name.len()) + (2 + 6) + (2 + 6); // uid/gid = "glenda"
    if sb.len() < need {
        return Err("devcons: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); // type: unused this era
    w.u32(crate::dat::DEV_CONS as u32);
    w.u8(if mode & 0x8000_0000 != 0 { QTDIR } else { QTFILE });
    w.u32(0); // vers
    w.u64(qid);
    w.u32(mode);
    w.u32(0); // atime
    w.u32(0); // mtime
    w.u64(length);
    w.string(name);
    w.string(b"glenda");
    w.string(b"glenda");
    Ok(w.finish())
}

/// stat of a devcons chan (sys_stat/sys_fstat's dispatch).
pub fn stat(c: &crate::dat::Chan, sb: &mut [u8]) -> crate::Result<u64> {
    if c.qid.typ == QTDIR {
        // The root dir's entry (name "#c" per devattach convention).
        return write_stat_record(sb, b"#c", QID_ROOT, 0x8000_0000 | 0o555, 0);
    }
    let Some((name, qid, mode)) = FILES.iter().find(|(_, q, _)| *q == c.qid.path) else {
        return Err("devcons: stat of unknown file");
    };
    let length = if *qid == QID_HOSTOWNER {
        hostowner_len()
    } else if *qid == QID_TIME {
        time_len()
    } else {
        0
    };
    write_stat_record(sb, name, *qid, *mode, length)
}

/// Read from a devcons chan.  `offset` u64::MAX uses + advances the chan's
/// streaming offset (the pread convention).  cons reads are 05-08's — an
/// open for read succeeds but reads get a loud -1 rather than wedging
/// (the plan's explicit contract); null reads EOF; user reads "glenda\n";
/// hostowner reads the stored bytes; the root dir serves the stat-wire
/// listing (partinit's readparts consumes it — no sd* entries exist).
pub fn read(c: &mut crate::dat::Chan, buf: &mut [u8], offset: u64) -> crate::Result<u64> {
    let off = if offset == u64::MAX { c.offset } else { offset };
    let n = match c.qid.path {
        QID_ROOT => {
            // The directory listing: the four files' stat records
            // concatenated, rebuilt per call (static content; hostowner's
            // length field may shift after a write — acceptable).
            let mut listing = alloc::vec::Vec::new();
            for (name, qid, mode) in FILES {
                let start = listing.len();
                let length = if qid == QID_HOSTOWNER {
                    hostowner_len()
                } else if qid == QID_TIME {
                    time_len()
                } else {
                    0
                };
                listing.resize(start + 41 + (2 + name.len()) + 2 * (2 + 6), 0);
                write_stat_record(&mut listing[start..], name, qid, mode, length)?;
            }
            let start = (off as usize).min(listing.len());
            let avail = &listing[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        }
        QID_CONS => return Err("devcons: cons reads arrive with 05-08"),
        QID_NULL => 0, // null reads EOF
        QID_USER => {
            let start = (off as usize).min(USER_CONTENT.len());
            let avail = &USER_CONTENT[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        }
        QID_HOSTOWNER => {
            let node = LockNode::new();
            let stored = HOSTOWNER.lock(&node);
            let start = (off as usize).min(stored.len());
            let avail = &stored[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        }
        QID_TIME => {
            // 05-07 Task 2: the recorded settime write, or the fixed
            // default before any write (the donor's time file always has
            // an answer).
            let node = LockNode::new();
            let stored = TIME.lock(&node);
            let content: &[u8] = if stored.is_empty() { TIME_DEFAULT } else { &stored };
            let start = (off as usize).min(content.len());
            let avail = &content[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        }
        _ => return Err("devcons: read of unknown file"),
    };
    if offset == u64::MAX {
        c.offset += n;
    }
    Ok(n)
}

/// Write to a devcons chan: cons writes byte-exact through the Console MCS
/// lock (the 04-06 device-write path, untouched); null discards; user is
/// read-only; hostowner STORES the bytes (replace semantics — the glenda
/// no-op writes once); the root dir refuses.
pub fn write(c: &mut crate::dat::Chan, buf: &[u8]) -> crate::Result<u64> {
    match c.qid.path {
        QID_CONS => Ok(write_bytes(buf) as u64),
        QID_NULL => Ok(buf.len() as u64),
        QID_USER => Err("devcons: user is read-only"),
        QID_HOSTOWNER => {
            let node = LockNode::new();
            let mut stored = HOSTOWNER.lock(&node);
            stored.clear();
            stored.extend_from_slice(buf);
            Ok(buf.len() as u64)
        }
        QID_TIME => {
            // 05-07 Task 2: accept-and-record-the-attempt (no RTC this
            // phase — the write SUCCEEDS and stores the bytes; settime's
            // contract is only that the write never fails).
            let node = LockNode::new();
            let mut stored = TIME.lock(&node);
            stored.clear();
            stored.extend_from_slice(buf);
            Ok(buf.len() as u64)
        }
        _ => Err("devcons: write of a directory"),
    }
}
