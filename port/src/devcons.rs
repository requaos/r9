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

/// The registered uart is a SHARED &'static dyn Uart (putb is
/// a &self method) — no `&'static mut` ever enters the console.
static CONS: Lock<Option<&'static dyn Uart>> = Lock::new("cons", None);

/// Console is what should be used in almost all cases, as it ensures threadsafe
/// use of the console.
pub struct Console;

impl Console {
    pub fn set_uart<F>(uart_fn: F)
    where
        F: FnOnce() -> Result<&'static dyn Uart>,
    {
        let node = LockNode::new();
        let mut cons = CONS.lock(&node);
        *cons = uart_fn().ok();
    }

    pub fn putstr(&mut self, s: &str) {
        // XXX: Just for testing.

        let node = LockNode::new();
        let uart_guard = CONS.lock(&node);
        if let Some(uart) = uart_guard.as_deref() {
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
/// devcons DEVICE write path, called from syscall context.
/// Unlike the kernel print path there is no `\n`→`\r\n` cooking: device
/// writes are byte-exact (the 9front devcons conswrite shape), which is
/// what lets user-visible marker strings originate from the EL0 binary's
/// own pwrite path rather than from a kernel print on its behalf.
///
/// Locking: this shares the kernel print path's
/// lock, so no IRQ-context code may ever take it once syscalls can write —
/// the timer tick's println is cfg(not(dev_exec))-gated off for exactly
/// that reason.  Returns the number of bytes written.
pub fn write_bytes(buf: &[u8]) -> usize {
    let node = LockNode::new();
    let uart_guard = CONS.lock(&node);
    let Some(uart) = uart_guard.as_deref() else {
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

fn putb(uart: &dyn Uart, b: u8) {
    if b == b'\n' {
        uart.putb(b'\r');
    } else if b == BACKSPACE {
        uart.putb(b);
        uart.putb(b' ');
    }
    uart.putb(b);
}

// ---------------------------------------------------------------------------
// The registry-facing device surface — attach/walk/open/
// read/write/stat as free functions, the established port idiom
// (devroot.rs's header explains why the dat.rs Dev trait is not
// implemented).  The file set is what the boot chain touches: cons
// (opencons's three opens), null, user (init's readenv("#c/user")),
// hostowner (glenda's guarded write), time (settime's OWRITE target),
// consctl (raw switching), pid (libc getpid's read, so init's #p/N/ctl
// pri write names the real pid), and the cons READ path — the bounded
// input queue, the cooked line discipline, and the DEL→interrupt note
// post.
//
// Qid encoding (collision-free): root = 0 (QTDIR), cons = 1 (QTFILE — the
// fd 0/1/2 preopen convention, proc.rs), null = 2, user = 3, hostowner = 4,
// time = 5, consctl = 6, pid = 7, random = 8, urandom = 9.
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
/// The time file (settime's OWRITE target).
pub(crate) const QID_TIME: u64 = 5;
/// The consctl file — "rawon"/"rawoff" writes flip the
/// cooked/raw input mode; reads report the current mode word (the donor's
/// consctl is write-only 0220; here reads report the
/// current mode word).
pub(crate) const QID_CONSCTL: u64 = 6;
/// The pid file
/// (the donor devcons.c's Qpid) — a read returns the OPERATING proc's
/// pid as decimal ASCII via the injected current-pid hook (the
/// devproc-style dependency inversion; proc.rs's current_pid).  libc's
/// getpid() reads it (`open("#c/pid")` + atol), which is what lets
/// init's `#p/%d/ctl` "pri 10" write name the REAL pid instead of 0.
pub(crate) const QID_PID: u64 = 7;
/// The random file (D-17) — a read serves fresh bytes from the
/// DTB-seeded ChaCha20 DRBG (the random device section at the bottom of
/// this file).  Read-only (0444): no write-to-stir surface this era.
pub(crate) const QID_RANDOM: u64 = 8;
/// The urandom file (D-17) — identical behavior to random this era
/// (both are non-blocking reads from the same DRBG; Phase 11 hardens
/// and splits the semantics).  Read-only (0444).
pub(crate) const QID_URANDOM: u64 = 9;

/// attach: the devcons root chan (one console tree this era — the attach
/// spec is empty, the devroot attach precedent).
pub fn attach() -> crate::dat::Chan {
    crate::dat::Chan::new(
        crate::dat::DEV_CONS,
        crate::dat::Qid { path: QID_ROOT, vers: 0, typ: QTDIR },
        0,
    )
}

/// The hostowner string (the sane-default contract): the glenda no-op's
/// guarded `#c/hostowner` write stores here; reads return the stored bytes
/// (empty until the first write — EOF).  The mcslock static-publish idiom;
/// syscall context only.
static HOSTOWNER: Lock<alloc::vec::Vec<u8>> = Lock::new("hostowner", alloc::vec::Vec::new());

/// The `#c/time` file (the donor
/// devcons.c Qtime).  settime's boot flow opens it OWRITE and writes the
/// root's atime as decimal seconds — r9 has NO RTC, so the
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
const FILES: [(&[u8], u64, u32); 9] = [
    (b"cons", QID_CONS, 0o666),
    (b"null", QID_NULL, 0o666),
    (b"user", QID_USER, 0o444),
    (b"hostowner", QID_HOSTOWNER, 0o664),
    (b"time", QID_TIME, 0o664),
    (b"consctl", QID_CONSCTL, 0o664),
    (b"pid", QID_PID, 0o444),
    (b"random", QID_RANDOM, 0o444),
    (b"urandom", QID_URANDOM, 0o444),
];

/// The arch-side current-pid hook for the pid file (the devproc
/// set_post_hook idiom — port cannot see PROC_TABLE).  Registered once
/// at boot before the first exec (main.rs, dev_exec — NOT virt-gated, so
/// #c/pid answers on every dev_exec platform); a read before
/// registration errors loudly (a pre-exec getpid() is a boot bug).
static PID_HOOK: Lock<Option<fn() -> u64>> = Lock::new("conspid", None);

/// Register the arch-side current-pid hook.  A second registration is a
/// boot bug.
pub fn set_pid_hook(hook: fn() -> u64) {
    let node = LockNode::new();
    let mut guard = PID_HOOK.lock(&node);
    assert!(guard.is_none(), "devcons: pid hook registered twice");
    *guard = Some(hook);
}

/// The pid file's content: the operating proc's pid as decimal ASCII
/// (getpid's atol parses it; the donor's fixed-width readnum padding is
/// cosmetic and omitted).
fn pid_content() -> crate::Result<alloc::vec::Vec<u8>> {
    let hook = {
        let node = LockNode::new();
        *PID_HOOK.lock(&node)
    };
    let Some(hook) = hook else {
        return Err("devcons: pid hook not registered");
    };
    let mut pid = hook();
    let mut out = alloc::vec::Vec::new();
    if pid == 0 {
        out.push(b'0');
    }
    while pid > 0 {
        out.insert(0, b'0' + (pid % 10) as u8);
        pid /= 10;
    }
    Ok(out)
}

/// The pid file's current length (for stat/dir listings) — 0 before the
/// hook registers (pre-exec; no #c listing happens then).
fn pid_len() -> u64 {
    pid_content().map(|c| c.len() as u64).unwrap_or(0)
}

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

/// Open a walked devcons chan.  The root dir opens OREAD only; user, pid,
/// random and urandom are read-only (0444); cons/null/hostowner open for
/// read or write.
pub fn open(c: &crate::dat::Chan, mode: u32) -> crate::Result<crate::dat::Chan> {
    if c.qid.typ == QTDIR && mode != crate::dat::OREAD {
        return Err("devcons: directory opens OREAD only");
    }
    if (c.qid.path == QID_USER
        || c.qid.path == QID_PID
        || c.qid.path == QID_RANDOM
        || c.qid.path == QID_URANDOM)
        && mode != crate::dat::OREAD
    {
        return Err("devcons: user/pid/random/urandom are read-only");
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

/// The time file's current length: the recorded write's length,
/// or the fixed default's before any write.
fn time_len() -> u64 {
    let node = LockNode::new();
    let stored = TIME.lock(&node);
    if stored.is_empty() { TIME_DEFAULT.len() as u64 } else { stored.len() as u64 }
}

/// The consctl mode word's current length: "raw\n" or "cooked\n".
fn consctl_len() -> u64 {
    let node = LockNode::new();
    let inq = INPUT.lock(&node);
    if inq.raw { 4 } else { 7 }
}

/// The "user" file's content (the sane default: glenda — the auth stub
/// never runs a real handshake).  NO trailing newline (the
/// donor's Quser read is `readstr(up->user)` — the bare name): init's
/// readenv("#c/user") feeds newns's `$user` → `/usr/$user` verbatim, so a
/// newline would poison the home path.
const USER_CONTENT: &[u8] = b"glenda";

/// One stat(5) wire record for a file (the shared StatWriter shape; the
/// 41-byte fixed part + FOUR length-prefixed strings — name, uid, gid,
/// muid: libc's statcheck walks FOUR and rejects a record that ends early,
/// convM2D.c; uid/gid/muid glenda).
fn write_stat_record(sb: &mut [u8], name: &[u8], qid: u64, mode: u32, length: u64) -> crate::Result<u64> {
    let need = 41 + (2 + name.len()) + 3 * (2 + 6); // uid/gid/muid = "glenda"
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
    w.string(b"glenda"); // muid — the fourth string statcheck walks
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
    } else if *qid == QID_CONSCTL {
        consctl_len()
    } else if *qid == QID_PID {
        pid_len()
    } else {
        0
    };
    write_stat_record(sb, name, *qid, *mode, length)
}

/// Read from a devcons chan.  `offset` u64::MAX uses + advances the chan's
/// streaming offset (the pread convention).  cons reads are the read
/// path (cons_read: cooked line discipline / raw, blocking until bytes are
/// available); null reads EOF; user reads "glenda"; hostowner reads the
/// stored bytes; consctl reads the mode word; the root dir serves the
/// stat-wire listing (partinit's readparts consumes it — no sd* entries
/// exist).
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
                } else if qid == QID_CONSCTL {
                    consctl_len()
                } else if qid == QID_PID {
                    pid_len()
                } else {
                    0
                };
                listing.resize(start + 41 + (2 + name.len()) + 3 * (2 + 6), 0);
                write_stat_record(&mut listing[start..], name, qid, mode, length)?;
            }
            let start = (off as usize).min(listing.len());
            let avail = &listing[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        }
        // The real console read — cooked/raw, blocks until
        // bytes are available (see the input engine below).  Cons is a
        // streaming device: the offset is ignored (the pread -1
        // convention's advance is meaningless here, like devpipe).
        QID_CONS => return cons_read(buf),
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
            // The recorded settime write, or the fixed
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
        QID_CONSCTL => {
            // Report the current mode word (the donor's consctl is
            // write-only).
            let node = LockNode::new();
            let inq = INPUT.lock(&node);
            let word: &[u8] = if inq.raw { b"raw\n" } else { b"cooked\n" };
            let start = (off as usize).min(word.len());
            let avail = &word[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        }
        QID_PID => {
            // The operating proc's pid as decimal ASCII (libc
            // getpid's read — init's #p/N/ctl write names the real pid).
            let content = pid_content()?;
            let start = (off as usize).min(content.len());
            let avail = &content[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        }
        // /dev/random + /dev/urandom (D-17): fresh bytes from the
        // ChaCha20 DRBG on every read — streaming devices, the offset
        // ignored (the cons/devpipe streaming precedent).  Both files
        // behave identically this era (the module comment at the bottom
        // records the minimality + the Phase-11 supersession).
        QID_RANDOM | QID_URANDOM => return Ok(random_read(buf)),
        _ => return Err("devcons: read of unknown file"),
    };
    if offset == u64::MAX {
        c.offset += n;
    }
    Ok(n)
}

/// Write to a devcons chan: cons writes byte-exact through the Console MCS
/// lock; null discards; user is
/// read-only; hostowner STORES the bytes (replace semantics — the glenda
/// no-op writes once); consctl flips the input mode; the root dir
/// refuses.
pub fn write(c: &mut crate::dat::Chan, buf: &[u8]) -> crate::Result<u64> {
    match c.qid.path {
        QID_CONS => Ok(write_bytes(buf) as u64),
        QID_NULL => Ok(buf.len() as u64),
        QID_USER => Err("devcons: user is read-only"),
        QID_RANDOM | QID_URANDOM => Err("devcons: random/urandom are read-only"),
        QID_HOSTOWNER => {
            let node = LockNode::new();
            let mut stored = HOSTOWNER.lock(&node);
            stored.clear();
            stored.extend_from_slice(buf);
            Ok(buf.len() as u64)
        }
        QID_TIME => {
            // Accept-and-record-the-attempt (no RTC — the write SUCCEEDS
            // and stores the bytes; settime's
            // contract is only that the write never fails).
            let node = LockNode::new();
            let mut stored = TIME.lock(&node);
            stored.clear();
            stored.extend_from_slice(buf);
            Ok(buf.len() as u64)
        }
        QID_CONSCTL => {
            // The donor conswrite Qconsctl scan —
            // space-separated words, each PREFIX-compared (the donor's
            // strncmp(a, "rawon", 5)): "rawon" switches to raw mode (no
            // editing, no DEL interception) and completes a pending
            // partial cooked line (the donor's ch=0 flush marker);
            // "rawoff" switches back.  The prefix compare is what makes
            // the natural `echo rawon >#c/consctl` work — the word is
            // "rawon\n", which strncmp matches.  Unknown words are ignored
            // (the donor's scan shape; ctlpon/ctlpoff stay deferred with
            // Ctrl-P).
            let node = LockNode::new();
            let mut inq = INPUT.lock(&node);
            for w in buf.split(|&b| b == b' ') {
                if w.starts_with(b"rawon") {
                    inq.raw = true;
                    if !inq.line.is_empty() {
                        let line = core::mem::take(&mut inq.line);
                        inq.lines.extend(line);
                    }
                } else if w.starts_with(b"rawoff") {
                    inq.raw = false;
                }
            }
            Ok(buf.len() as u64)
        }
        _ => Err("devcons: write of a directory"),
    }
}

// ---------------------------------------------------------------------------
// The console INPUT path — the userland-read half of
// devcons (earlier binaries only wrote; this is the interactive
// half).  Semantics ported from 9k's devcons.c consread Qcons arm +
// conswrite Qconsctl arm ("port
// the semantics, not the letter"):
//
//   * Cooked mode (the default): bytes accumulate into a line; 0x08
//     erases the last byte, 0x15 (^U) kills the line, '\r' is accepted as
//     Enter (the wire's CRLF cooking means either arrives — the donor's
//     kbdcr2nl converts at IRQ time; the conversion rides the assembly
//     here instead, same effect), and a completed line INCLUDES its
//     trailing '\n' (Plan 9 cooked reads return whole lines).  0x04 (^D)
//     ends the line WITHOUT being buffered (the donor's send-without-append
//     arm); on an EMPTY line it completes a zero-length line, which reads
//     as a 0-length read — the EOF that lets rc exit on ^D at a fresh
//     prompt (the donor's qwrite(lineq, kbd.line, 0)).
//     PREAD on /dev/cons blocks until a complete line is available.
//   * DEL (0x7f) is NEVER buffered: it discards the in-assembly partial
//     line and posts the "interrupt" note to the console's FOREGROUND
//     GROUP (below), then the reading syscall fails -1/"interrupted"
//     (the donor's qread-under-a-note behavior — rc's Eintr path
//     discards its parse state and re-prompts).
//   * Raw mode (consctl "rawon"): bytes return unprocessed — no editing,
//     no DEL interception; a read returns what is queued, blocking only
//     while empty.  "rawoff" switches back.  "rawon" completes a pending
//     partial cooked line first (the donor's ch=0 flush marker).
//
// The foreground-group SCOPING: the
// donor's DEL post targets a note group via /proc/notepg machinery r9
// does not grow this era.  The note posts to the
// note GROUPS of the proc(s) reading the console — the blocked waiters
// plus the current reader (the procs conversing with the console right
// now).  r9 grows real per-proc note-group ids for this (Proc
// noteid + RFNOTEG's real semantics), so init's rfork(RFNOTEG)'d shell
// children are interrupted WITHOUT init itself getting the note — the
// donor's pgrpnote shape over the readers' groups.
//
// Documented deltas from the letter of devcons.c: (a) input ECHO fires
// at DRAIN time in the reader's syscall context: the cooked processing
// loop collects
// echo bytes into a bounded Vec (ECHO_CAP, drop-excess) and
// calls the arch's echo hook AFTER the INPUT lock drops (the
// INPUT→Console non-nesting rule; debug_dump_input's precedent).  A
// blocked reader's keystrokes echo ≤ one tick later (the tick's
// cons_input_wake readies it); with no blocked reader, typed-ahead bytes
// sit in the PL011 ring un-echoed until the next read — the terminal-era
// IRQ echo stays out of scope.  Raw mode never echoes; DEL/^U/^D stay
// silent; BACKSPACE echoes the BS-SP-BS triple only when it actually
// pops; CR/LF line completers echo CRLF.  (b) Ctrl-P/debugger entry
// stays deferred (pl011.rs); (c) the donor's multi-queue
// kbdqs merge is single-console here.
//
// LOCKING (the syscall.rs header rule, extended): the INPUT queue lock
// is taken ONLY from syscall/trap context — IRQ context NEVER takes it.
// The PL011 RX IRQ handler pushes bytes into its lock-free ring
// (pl011.rs rx_irq_cons_buffer, unchanged); the ring→queue DRAIN happens
// here, in the reader's own syscall context.  A blocked reader is readied
// by the TICK (timer.rs's cons_rx_pending check → proc::cons_input_wake)
// — the tick's PROC_TABLE take from trap context is the reschedule()
// precedent; the reader then resumes, drains, and processes in ITS OWN
// syscall context.  register-waiter → drop-lock → block cannot lose a
// wake: syscalls are serialized (single CPU, IRQ-masked), so the tick can
// only fire after block() swtch'es (devpipe's documented invariant).
//
// DoS bound: the input queue is bounded at 4 KiB; on overrun the
// OLDEST bytes are dropped with a one-line console warning — the IRQ
// handler never blocks and never grows unbounded (its ring is the fixed
// 256-byte PL011-side one; this queue is the syscall-side aggregate).
// ---------------------------------------------------------------------------

/// The console input queue bound.
const INPUT_CAP: usize = 4096;
/// The cooked line-assembly bound (the donor's `char line[1024]`).
const LINE_CAP: usize = 1024;
/// The per-drain echo buffer's bound — at most
/// INPUT_CAP drained bytes, each echoed verbatim (≤ INPUT_CAP), plus a
/// BS-SP-BS triple per erase (≤ 3*LINE_CAP pops) plus a CRLF pair per
/// completer (≤ the drained count, under the 2*INPUT_CAP arm).  Excess
/// is DROPPED, never grown unbounded — the input queue's overrun
/// discipline's twin.
const ECHO_CAP: usize = 2 * INPUT_CAP + 3 * LINE_CAP;

/// Append to the bounded echo buffer (drop-excess — ECHO_CAP).
fn echo_push(echo: &mut alloc::vec::Vec<u8>, bytes: &[u8]) {
    let room = ECHO_CAP.saturating_sub(echo.len());
    let n = bytes.len().min(room);
    echo.extend_from_slice(&bytes[..n]);
}

/// The console input state (syscall/trap context only — the locking rule
/// above).  `input` is the unprocessed byte queue (the donor's kbdq),
/// `line` the in-assembly partial (the donor's kbd.line), `lines` the
/// completed-lines queue (the donor's lineq); `waiters` are the pids
/// blocked in a cons read (the DEL target set's base).  `pending_eof`
/// counts zero-length completed lines — ^D on an EMPTY in-assembly line:
/// the donor's qwrite(lineq, kbd.line, 0) queues a zero-length
/// block whose qread returns 0 (the EOF that lets rc exit on ^D at a
/// fresh prompt); a flat byte queue cannot represent it.  Consumed only
/// when `lines` is empty, preserving the donor's block order (bytes
/// queued before the ^D read first).
struct ConsInput {
    input: alloc::collections::VecDeque<u8>,
    line: alloc::vec::Vec<u8>,
    lines: alloc::collections::VecDeque<u8>,
    pending_eof: usize,
    raw: bool,
    waiters: alloc::vec::Vec<u64>,
    overruns: usize,
}

impl ConsInput {
    const fn new() -> ConsInput {
        ConsInput {
            input: alloc::collections::VecDeque::new(),
            line: alloc::vec::Vec::new(),
            lines: alloc::collections::VecDeque::new(),
            pending_eof: 0,
            raw: false,
            waiters: alloc::vec::Vec::new(),
            overruns: 0,
        }
    }
}

static INPUT: Lock<ConsInput> = Lock::new("consin", ConsInput::new());

/// gatedbg: dump the cons input state to
/// the wire (the wedge autopsy — called from the scheduler's idle-arm
/// tripwire).  INPUT-brief; println takes Console afterwards (INPUT →
/// Console is not a nested hold: the lock drops before the print).
pub fn debug_dump_input() {
    let node = LockNode::new();
    let (waiters, input_len, line_len, lines_len, pending_eof, raw) = {
        let inq = INPUT.lock(&node);
        (
            inq.waiters.clone(),
            inq.input.len(),
            inq.line.len(),
            inq.lines.len(),
            inq.pending_eof,
            inq.raw,
        )
    };
    crate::println!(
        "wedgedbg: cons input waiters={:?} input={} line={} lines={} pending_eof={} raw={}",
        waiters,
        input_len,
        line_len,
        lines_len,
        pending_eof,
        raw
    );
}

/// The arch-side hooks the read path rides (the devproc.rs
/// set_post_hook idiom — port cannot see the proc table or the PL011
/// ring): registered once before the first exec (main.rs, dev_exec).
/// `current` names the calling proc (the reader/waiter identity);
/// `drain_rx` drains the PL011 RX ring into the buffer (the lock-free
/// ring read, invoked only from this module's syscall-context sections);
/// `block` parks the caller mid-syscall until the tick wakes it
/// (arch: proc::cons_block_current); `post_group` posts a note to the
/// note GROUPS of the given reader pids (arch: the pgrpnote port);
/// `echo` writes the cooked-mode echo bytes to the wire (arch: the
/// device-write path — devcons::write_bytes, the syscall-context
/// Console-lock precedent).  The echo hook keeps the
/// port arch-blind (the post_group idiom).
#[derive(Clone, Copy)]
pub struct ConsHooks {
    pub current: fn() -> u64,
    pub drain_rx: fn(&mut [u8]) -> usize,
    pub block: fn(),
    pub post_group: fn(&[u64], &[u8]),
    pub echo: fn(&[u8]),
}

static CONS_HOOKS: Lock<Option<ConsHooks>> = Lock::new("conshooks", None);

/// Register the arch-side read-path hooks.  Called once during boot
/// before any user process exists; a second registration is a boot bug.
pub fn set_cons_hooks(hooks: ConsHooks) {
    let node = LockNode::new();
    let mut guard = CONS_HOOKS.lock(&node);
    assert!(guard.is_none(), "devcons: cons hooks registered twice");
    *guard = Some(hooks);
}

fn cons_hooks() -> crate::Result<ConsHooks> {
    let node = LockNode::new();
    CONS_HOOKS
        .lock(&node)
        .ok_or("devcons: cons read path not initialized (no hooks)")
}

/// The cooked-mode special bytes (the donor's switch; BACKSPACE/DELETE/
/// CTLU/CTLD constants live at the top of this file).
const CR: u8 = b'\r';
const LF: u8 = b'\n';

/// The /dev/cons read: cooked line discipline + raw mode +
/// blocking.  The waiters registration is held from the first block until
/// the read returns, so a DEL processed after a wake still targets this
/// reader's group (the waiters ∪ {current} set is computed at DEL time).
fn cons_read(buf: &mut [u8]) -> crate::Result<u64> {
    let hooks = cons_hooks()?;
    let pid = (hooks.current)();
    let node = LockNode::new();
    let mut registered = false;
    loop {
        let mut dropped = 0usize;
        let mut interrupted = false;
        let mut readers: alloc::vec::Vec<u64> = alloc::vec::Vec::new();
        // The cooked-mode echo bytes for THIS drain
        // (bounded, ECHO_CAP) — assembled under the INPUT lock, emitted
        // AFTER it drops (the INPUT→Console non-nesting rule;
        // debug_dump_input's drop-then-print precedent).
        let mut echo: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
        let ready: Option<u64> = {
            let mut inq = INPUT.lock(&node);
            // 1. Drain the RX ring into the bounded input queue
            //    (drop-oldest on overrun, counted + warned once below).
            let mut tmp = [0u8; 256];
            loop {
                let n = (hooks.drain_rx)(&mut tmp);
                if n == 0 {
                    break;
                }
                let room = INPUT_CAP.saturating_sub(inq.input.len());
                if n > room {
                    let d = n - room;
                    inq.input.drain(..d);
                    inq.overruns += d;
                    dropped += d;
                }
                inq.input.extend(&tmp[..n]);
                if n < tmp.len() {
                    break; // the ring is drained
                }
            }
            // 2. Process per mode.  Raw: bytes return unprocessed (no
            //    editing, no DEL interception) — return what is queued.
            //    Cooked: assemble lines with erase/kill editing; DEL is
            //    never buffered — it discards the partial line and posts
            //    the interrupt note.
            if inq.raw {
                // Raw mode echoes NOTHING (the window-system era owns the
                // display; a typed byte is never re-written here).
                if !inq.input.is_empty() {
                    let n = inq.input.len().min(buf.len());
                    for slot in buf[..n].iter_mut() {
                        *slot = inq.input.pop_front().unwrap_or(0);
                    }
                    Some(n as u64)
                } else {
                    None
                }
            } else {
                while let Some(b) = inq.input.pop_front() {
                    match b {
                        DELETE => {
                            // The interrupt path stays SILENT on the wire
                            // (no retro-erasure of the discarded partial).
                            inq.line.clear();
                            interrupted = true;
                        }
                        BACKSPACE => {
                            // Echo the erasure ONLY when it actually pops
                            // a byte (the BS-SP-BS triple — putb's shape);
                            // a stray BS on an empty line is silent.
                            if inq.line.pop().is_some() {
                                echo_push(&mut echo, &[BACKSPACE, b' ', BACKSPACE]);
                            }
                        }
                        CTLU => {
                            // Kill-line stays silent (the donor echoes
                            // nothing for ^U).
                            inq.line.clear();
                        }
                        CR | LF | CTLD => {
                            // CR→LF (the donor kbdcr2nl's !raw arm); the
                            // completed line INCLUDES the newline; ^D
                            // ends the line without being buffered.  The
                            // line completers echo CRLF (the terminal
                            // must advance — write_bytes is byte-exact,
                            // so both bytes ride); ^D (the silent EOF)
                            // echoes NOTHING.
                            if b != CTLD {
                                inq.line.push(LF);
                                echo_push(&mut echo, &[CR, LF]);
                            }
                            let line = core::mem::take(&mut inq.line);
                            if line.is_empty() && b == CTLD {
                                // ^D on an EMPTY line still
                                // completes a (zero-length) line — the
                                // donor's qwrite(lineq, kbd.line, 0) —
                                // whose read returns 0: the EOF that lets
                                // rc exit on ^D at a fresh prompt.
                                inq.pending_eof += 1;
                            } else {
                                inq.lines.extend(line);
                            }
                        }
                        _ => {
                            // A regular byte echoes VERBATIM — but only
                            // when it actually lands on the line (a
                            // byte dropped at LINE_CAP never echoes).
                            if inq.line.len() < LINE_CAP {
                                inq.line.push(b);
                                echo_push(&mut echo, &[b]);
                            }
                        }
                    }
                }
                if interrupted {
                    // The DEL target set: the blocked cons readers plus
                    // this reader (the console's foreground group this
                    // era), dedup'd — the arch side posts to their note
                    // GROUPS.
                    for p in inq.waiters.iter().chain(core::iter::once(&pid)) {
                        if !readers.contains(p) {
                            readers.push(*p);
                        }
                    }
                    None
                } else if !inq.lines.is_empty() {
                    let n = inq.lines.len().min(buf.len());
                    for slot in buf[..n].iter_mut() {
                        *slot = inq.lines.pop_front().unwrap_or(0);
                    }
                    Some(n as u64)
                } else if inq.pending_eof > 0 {
                    // A queued zero-length line (^D on empty)
                    // reads as EOF once — only once `lines` has drained
                    // (the donor's block order).
                    inq.pending_eof -= 1;
                    Some(0)
                } else {
                    None
                }
            }
        };
        // Emit the drain's echo AFTER the INPUT lock
        // dropped (NEVER under it — INPUT→Console non-nesting; the
        // write_bytes path takes the Console lock).  Echo fires at drain
        // time in the reader's syscall context — ≤ one tick after the
        // keystroke when a reader is blocked (the tick's cons_input_wake
        // readies it); with NO blocked reader, typed-ahead bytes sit in
        // the PL011 ring un-echoed until the next read (the terminal-era
        // IRQ echo stays out of scope).
        if !echo.is_empty() {
            (hooks.echo)(&echo);
        }
        if dropped > 0 {
            // The overrun's one-line warning (post-lock: Console is a
            // separate MCS lock; nothing nests Console → INPUT).
            crate::println!("devcons: console input overrun — dropped {dropped} oldest bytes");
        }
        if interrupted {
            (hooks.post_group)(&readers, b"interrupt");
            if registered {
                let mut inq = INPUT.lock(&node);
                inq.waiters.retain(|p| *p != pid);
            }
            return Err("interrupted");
        }
        if let Some(n) = ready {
            if registered {
                let mut inq = INPUT.lock(&node);
                inq.waiters.retain(|p| *p != pid);
            }
            return Ok(n);
        }
        // 3. Nothing to return — register as a waiter and block until
        //    the tick readies us (input pending).
        {
            let mut inq = INPUT.lock(&node);
            if !inq.waiters.contains(&pid) {
                inq.waiters.push(pid);
            }
            registered = true;
        }
        (hooks.block)();
    }
}

// ---------------------------------------------------------------------------
// /dev/random + /dev/urandom — the D-17 minimal DTB-seeded CSPRNG.
//
// D-17 (08-CONTEXT, 2026-11-05): the MINIMAL random device lands in Phase 8
// because ipconfig's init() calls srand(truerand()) UNCONDITIONALLY and
// libc's truerand() sysfatals without /dev/random (research OQ-2 / Pitfall
// P-I); r9 had no random file.  Scope is deliberately minimal: the two
// read-only (0444) files only — no devcap, no auth adjacency, no
// virtio-rng, no write-to-stir surface.  **Phase 11 (AUTH-02) HARDENS and
// EXPANDS this implementation** (a real entropy pool, reseed discipline, a
// write surface, hardware RNG sources); the construction below is
// superseded there.
//
// The CSPRNG: a ChaCha20-based DRBG in pure safe Rust (no new crate, no
// unsafe — this module keeps its forbid attribute).  Counter-mode
// keystream: a 32-byte key + an 8-byte counter (DJB's original layout:
// state = constants ‖ key ‖ counter64 ‖ nonce64=0), 20 rounds with the
// feed-forward add.  The DRBG is seeded LAZILY at the first read from the
// DTB's /chosen/rng-seed property (qemu supplies real entropy there when
// loading the blob — the committed virt-gicv2.dts header comment documents
// the injection), read via port::fdt from the arch hook, mixed with
// CNTFRQ+CNTPCT+tick jitter at seed time AND on every read.  The counter
// increments per 64-byte block and the key is jitter-mixed per read, so no
// two reads ever produce the same bytes; the read path never errors (a
// truerand() caller must never fall back to a weaker path — T-08-11).
//
// `urandom` is the non-blocking read from the pool; `random` is IDENTICAL
// this era (the pool never blocks) — the split is Phase 11's.
//
// Entropy hooks: the arch registers { rng_seed, jitter } once at boot
// (main.rs, platform = "virt" this era — the only dev_exec platform).  A
// read before registration (or on a platform with no registration) seeds
// from the block constants alone — deterministic bytes; the file still
// never errors, and the entropy guarantee is documented to require the
// registered hooks (every gate platform has them).
// ---------------------------------------------------------------------------

/// The ChaCha20 state constants — "expand 32-byte k" (RFC 8439 §2.3).
const CHACHA20_CONST: [u32; 4] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574];

/// The ChaCha20 quarter-round (RFC 8439 §2.1): add-xor-rotate on the
/// four named state words.
fn quarter_round(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(12);
    s[a] = s[a].wrapping_add(s[b]);
    s[d] = (s[d] ^ s[a]).rotate_left(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_left(7);
}

/// The ChaCha20 block function (RFC 8439 §2.3): constants ‖ key ‖
/// counter64 ‖ nonce64(=0), 20 rounds (10 double rounds: columns then
/// diagonals), then the feed-forward add of the input state.  Counter
/// mode over a 64-bit counter — the plan's "32-byte key + 8-byte
/// counter" shape.
fn chacha20_block(key: &[u32; 8], counter: u64, out: &mut [u32; 16]) {
    let mut w = [0u32; 16];
    w[0..4].copy_from_slice(&CHACHA20_CONST);
    w[4..12].copy_from_slice(key);
    w[12] = counter as u32;
    w[13] = (counter >> 32) as u32;
    // w[14..16] = nonce = 0 (fixed this era — the key+counter pair is
    // never reused, which is what the nonce exists to protect).
    let orig = w;
    for _ in 0..10 {
        quarter_round(&mut w, 0, 4, 8, 12);
        quarter_round(&mut w, 1, 5, 9, 13);
        quarter_round(&mut w, 2, 6, 10, 14);
        quarter_round(&mut w, 3, 7, 11, 15);
        quarter_round(&mut w, 0, 5, 10, 15);
        quarter_round(&mut w, 1, 6, 11, 12);
        quarter_round(&mut w, 2, 7, 8, 13);
        quarter_round(&mut w, 3, 4, 9, 14);
    }
    for i in 0..16 {
        out[i] = w[i].wrapping_add(orig[i]);
    }
}

/// The DRBG state: the 32-byte key (as 8 little-endian words), the
/// 64-bit block counter, and the seeded latch.
struct Drbg {
    key: [u32; 8],
    counter: u64,
    seeded: bool,
}

impl Drbg {
    const fn new() -> Drbg {
        Drbg { key: [0; 8], counter: 0, seeded: false }
    }
}

/// The DRBG cell.  Syscall context only (the file read path); the mcslock
/// static-publish idiom.  Lock order: DRBG → Console is the one-way order
/// (the seeded-marker print happens under the DRBG guard; no path takes
/// DRBG while holding Console — the console write/print paths never touch
/// the DRBG).
static DRBG: Lock<Drbg> = Lock::new("randomdrbg", Drbg::new());

/// The arch-side entropy hooks — the dependency-inversion idiom (port
/// cannot read the DTB or the aarch64 generic-timer registers).
/// Registered once at boot before the first exec (main.rs); a second
/// registration is a boot bug.
#[derive(Clone, Copy)]
pub struct RandomHooks {
    /// Copy the DTB's /chosen/rng-seed property bytes into `buf`,
    /// returning the byte count (0 when the property is absent — the
    /// CNTFRQ/ticks mixing then carries the whole seed, the plan's
    /// documented fallback).
    pub rng_seed: fn(&mut [u8]) -> usize,
    /// A jitter sample: CNTFRQ ^ CNTPCT ^ the 1 ms tick count (the
    /// plan's "CNTFRQ + tick jitter"; CNTPCT is the term that varies
    /// between back-to-back reads within one tick).
    pub jitter: fn() -> u64,
}

static RANDOM_HOOKS: Lock<Option<RandomHooks>> = Lock::new("randomhooks", None);

/// Register the arch-side entropy hooks.  A second registration is a
/// boot bug.
pub fn set_random_hooks(hooks: RandomHooks) {
    let node = LockNode::new();
    let mut guard = RANDOM_HOOKS.lock(&node);
    assert!(guard.is_none(), "devcons: random hooks registered twice");
    *guard = Some(hooks);
}

/// The once-at-seed marker guard (the ATTACH_PRINTED idiom in devip.rs):
/// `devcons: random seeded` prints exactly once, at the first read's
/// seed.
static RANDOM_SEEDED_PRINTED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// Mix one jitter sample-pair into the key (two samples, decorrelated
/// across different key words with a rotate — CNTPCT advances between
/// the two hook calls).
fn drbg_mix_jitter(drbg: &mut Drbg, hooks: &RandomHooks) {
    let j = (hooks.jitter)();
    drbg.key[0] ^= j as u32;
    drbg.key[1] ^= (j >> 32) as u32;
    let j2 = (hooks.jitter)();
    drbg.key[2] ^= (j2 as u32).rotate_left(11);
    drbg.key[3] ^= ((j2 >> 32) as u32).rotate_left(11);
}

/// Seed the DRBG: fold the DTB rng-seed bytes into the key
/// (add-and-rotate, little-endian byte order, wrapping across the 8
/// words), mix the boot jitter, then compress with the block function
/// itself (key ← lo(E(key,0)) + hi(E(key,0)) — the one-way fold that
/// keeps the raw seed bytes unrecoverable from the later keystream).
fn drbg_seed(drbg: &mut Drbg, hooks: &RandomHooks) {
    let mut seed_bytes = [0u8; 64];
    let n = (hooks.rng_seed)(&mut seed_bytes).min(seed_bytes.len());
    for (i, &b) in seed_bytes[..n].iter().enumerate() {
        let w = (i / 4) % 8;
        let shift = (i % 4) * 8;
        drbg.key[w] = drbg.key[w].wrapping_add((b as u32) << shift).rotate_left(7);
    }
    drbg_mix_jitter(drbg, hooks);
    let mut out = [0u32; 16];
    chacha20_block(&drbg.key, 0, &mut out);
    for i in 0..8 {
        drbg.key[i] = out[i].wrapping_add(out[i + 8]);
    }
    drbg.counter = 1;
    drbg.seeded = true;
}

/// A kernel-internal seed draw: 8 bytes from the DRBG (smoltcp's
/// Config.random_seed at the ether-core bind — 08-06).  Seeds the DRBG
/// lazily on the first call (the once-marker prints then, wherever the
/// first draw happens to come from).
pub fn random_u64() -> u64 {
    let mut b = [0u8; 8];
    random_read(&mut b);
    u64::from_le_bytes(b)
}

/// Serve a /dev/random (== /dev/urandom this era) read: seed lazily at
/// the first call, mix fresh jitter on every call, then emit counter-mode
/// keystream blocks (the counter increments per block — never the same
/// bytes twice).  Never errors.
fn random_read(buf: &mut [u8]) -> u64 {
    let hooks = {
        let node = LockNode::new();
        *RANDOM_HOOKS.lock(&node)
    };
    let node = LockNode::new();
    let mut drbg = DRBG.lock(&node);
    if !drbg.seeded {
        match hooks {
            Some(h) => {
                drbg_seed(&mut drbg, &h);
                // The seeded marker (D-17's gate-visible proof) — once,
                // under the DRBG guard; DRBG → Console is the one-way
                // lock order (the INPUT → Console precedent).
                if !RANDOM_SEEDED_PRINTED.swap(true, core::sync::atomic::Ordering::Relaxed) {
                    crate::println!("devcons: random seeded");
                }
            }
            None => {
                // No hooks registered (a non-gate platform): seed from
                // the constants alone — deterministic bytes, documented
                // in the module header; the file still never errors.
                drbg.counter = 1;
                drbg.seeded = true;
            }
        }
    }
    if let Some(h) = hooks {
        drbg_mix_jitter(&mut drbg, &h);
    }
    let mut written = 0usize;
    let mut block = [0u32; 16];
    while written < buf.len() {
        chacha20_block(&drbg.key, drbg.counter, &mut block);
        drbg.counter = drbg.counter.wrapping_add(1);
        for &w in &block {
            for &b in &w.to_le_bytes() {
                if written < buf.len() {
                    buf[written] = b;
                    written += 1;
                }
            }
        }
    }
    written as u64
}
