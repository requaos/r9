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
// 05-07 Task 2: the time file (settime's OWRITE target).  05-08 (D-06):
// the cons READ path — the bounded input queue, the cooked line
// discipline, consctl raw switching, and the DEL→interrupt note post.
// 05-09 (D-09): the pid file (libc getpid's read — the deferred item's
// fix, so init's #p/N/ctl pri write names the real pid).
//
// Qid encoding (collision-free): root = 0 (QTDIR), cons = 1 (QTFILE — the
// fd 0/1/2 preopen convention, proc.rs), null = 2, user = 3, hostowner = 4,
// time = 5 (05-07), consctl = 6 (05-08), pid = 7 (05-09).
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
/// 05-08 (D-06): the consctl file — "rawon"/"rawoff" writes flip the
/// cooked/raw input mode; reads report the current mode word (the donor's
/// consctl is write-only 0220; the plan extends it: "reads report the
/// current mode word").
pub(crate) const QID_CONSCTL: u64 = 6;
/// 05-09 (D-09's wiring gap — the phase deferred item): the pid file
/// (the donor devcons.c's Qpid) — a read returns the OPERATING proc's
/// pid as decimal ASCII via the injected current-pid hook (the
/// devproc-style dependency inversion; proc.rs's current_pid).  libc's
/// getpid() reads it (`open("#c/pid")` + atol), which is what lets
/// init's `#p/%d/ctl` "pri 10" write name the REAL pid instead of 0.
pub(crate) const QID_PID: u64 = 7;

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
const FILES: [(&[u8], u64, u32); 7] = [
    (b"cons", QID_CONS, 0o666),
    (b"null", QID_NULL, 0o666),
    (b"user", QID_USER, 0o444),
    (b"hostowner", QID_HOSTOWNER, 0o664),
    (b"time", QID_TIME, 0o664),
    (b"consctl", QID_CONSCTL, 0o664),
    (b"pid", QID_PID, 0o444),
];

/// 05-09: the arch-side current-pid hook for the pid file (the devproc
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

/// Open a walked devcons chan.  The root dir opens OREAD only; user and
/// pid are read-only (0444); cons/null/hostowner open for read or write.
pub fn open(c: &crate::dat::Chan, mode: u32) -> crate::Result<crate::dat::Chan> {
    if c.qid.typ == QTDIR && mode != crate::dat::OREAD {
        return Err("devcons: directory opens OREAD only");
    }
    if (c.qid.path == QID_USER || c.qid.path == QID_PID) && mode != crate::dat::OREAD {
        return Err("devcons: user/pid are read-only");
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

/// The consctl mode word's current length (05-08): "raw\n" or "cooked\n".
fn consctl_len() -> u64 {
    let node = LockNode::new();
    let inq = INPUT.lock(&node);
    if inq.raw { 4 } else { 7 }
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
/// streaming offset (the pread convention).  cons reads are the 05-08 read
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
                listing.resize(start + 41 + (2 + name.len()) + 2 * (2 + 6), 0);
                write_stat_record(&mut listing[start..], name, qid, mode, length)?;
            }
            let start = (off as usize).min(listing.len());
            let avail = &listing[start..];
            let n = avail.len().min(buf.len());
            buf[..n].copy_from_slice(&avail[..n]);
            n as u64
        }
        // 05-08 (D-06): the real console read — cooked/raw, blocks until
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
        QID_CONSCTL => {
            // 05-08: report the current mode word (the plan's extension —
            // the donor's consctl is write-only).
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
            // 05-09: the operating proc's pid as decimal ASCII (libc
            // getpid's read — init's #p/N/ctl write names the real pid).
            let content = pid_content()?;
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
/// no-op writes once); consctl flips the input mode (05-08); the root dir
/// refuses.
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
        QID_CONSCTL => {
            // 05-08 (D-06): the donor conswrite Qconsctl scan —
            // space-separated words; "rawon" switches to raw mode (no
            // editing, no DEL interception) and completes a pending
            // partial cooked line (the donor's ch=0 flush marker);
            // "rawoff" switches back.  Unknown words are ignored (the
            // donor's scan shape; ctlpon/ctlpoff stay deferred with
            // Ctrl-P per CONTEXT.md).
            let node = LockNode::new();
            let mut inq = INPUT.lock(&node);
            for w in buf.split(|&b| b == b' ') {
                if w == b"rawon" {
                    inq.raw = true;
                    if !inq.line.is_empty() {
                        let line = core::mem::take(&mut inq.line);
                        inq.lines.extend(line);
                    }
                } else if w == b"rawoff" {
                    inq.raw = false;
                }
            }
            Ok(buf.len() as u64)
        }
        _ => Err("devcons: write of a directory"),
    }
}

// ---------------------------------------------------------------------------
// 05-08 (D-06): the console INPUT path — the userland-read half of
// devcons (Phase-4 binaries only wrote; this is the plan's "interactive
// half").  Semantics ported from 9k's devcons.c consread Qcons arm +
// conswrite Qconsctl arm (the RESEARCH Don't-Hand-Roll mandate — "port
// the semantics, not the letter"):
//
//   * Cooked mode (the default): bytes accumulate into a line; 0x08
//     erases the last byte, 0x15 (^U) kills the line, '\r' is accepted as
//     Enter (the wire's CRLF cooking means either arrives — the donor's
//     kbdcr2nl converts at IRQ time; the conversion rides the assembly
//     here instead, same effect), and a completed line INCLUDES its
//     trailing '\n' (Plan 9 cooked reads return whole lines).  0x04 (^D)
//     ends the line WITHOUT being buffered (the donor's send-without-append
//     arm).  PREAD on /dev/cons blocks until a complete line is available.
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
// The foreground-group SCOPING (the plan's sanctioned delta): the
// donor's DEL post targets a note group via /proc/notepg machinery r9
// does not grow this era.  Scoped per the plan: the note posts to the
// note GROUPS of the proc(s) reading the console — the blocked waiters
// plus the current reader (the procs conversing with the console right
// now).  r9 grows real per-proc note-group ids for this (05-08: Proc
// noteid + RFNOTEG's real semantics), so init's rfork(RFNOTEG)'d shell
// children are interrupted WITHOUT init itself getting the note — the
// donor's pgrpnote shape over the readers' groups.
//
// Documented deltas from the letter of devcons.c: (a) no input ECHO —
// the typed bytes are never written back to the wire (echo belongs to
// the window/kbdfs era; nothing in D-06 or the plan names it, and its
// absence keeps the interactive gate's output markers unambiguous);
// (b) Ctrl-P/debugger entry stays deferred (pl011.rs, CONTEXT.md);
// (c) the donor's multi-queue kbdqs merge is single-console here.
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
// T-05-15 (DoS): the input queue is bounded at 4 KiB; on overrun the
// OLDEST bytes are dropped with a one-line console warning — the IRQ
// handler never blocks and never grows unbounded (its ring is the fixed
// 256-byte PL011-side one; this queue is the syscall-side aggregate).
// ---------------------------------------------------------------------------

/// The console input queue bound (T-05-15).
const INPUT_CAP: usize = 4096;
/// The cooked line-assembly bound (the donor's `char line[1024]`).
const LINE_CAP: usize = 1024;

/// The console input state (syscall/trap context only — the locking rule
/// above).  `input` is the unprocessed byte queue (the donor's kbdq),
/// `line` the in-assembly partial (the donor's kbd.line), `lines` the
/// completed-lines queue (the donor's lineq); `waiters` are the pids
/// blocked in a cons read (the DEL target set's base).
struct ConsInput {
    input: alloc::collections::VecDeque<u8>,
    line: alloc::vec::Vec<u8>,
    lines: alloc::collections::VecDeque<u8>,
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
            raw: false,
            waiters: alloc::vec::Vec::new(),
            overruns: 0,
        }
    }
}

static INPUT: Lock<ConsInput> = Lock::new("consin", ConsInput::new());

/// The arch-side hooks the read path rides (the devproc.rs
/// set_post_hook idiom — port cannot see the proc table or the PL011
/// ring): registered once before the first exec (main.rs, dev_exec).
/// `current` names the calling proc (the reader/waiter identity);
/// `drain_rx` drains the PL011 RX ring into the buffer (the lock-free
/// ring read, invoked only from this module's syscall-context sections);
/// `block` parks the caller mid-syscall until the tick wakes it
/// (arch: proc::cons_block_current); `post_group` posts a note to the
/// note GROUPS of the given reader pids (arch: the pgrpnote port).
#[derive(Clone, Copy)]
pub struct ConsHooks {
    pub current: fn() -> u64,
    pub drain_rx: fn(&mut [u8]) -> usize,
    pub block: fn(),
    pub post_group: fn(&[u64], &[u8]),
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

/// The /dev/cons read (05-08): cooked line discipline + raw mode +
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
                            inq.line.clear();
                            interrupted = true;
                        }
                        BACKSPACE => {
                            inq.line.pop();
                        }
                        CTLU => {
                            inq.line.clear();
                        }
                        CR | LF | CTLD => {
                            // CR→LF (the donor kbdcr2nl's !raw arm); the
                            // completed line INCLUDES the newline; ^D
                            // ends the line without being buffered.
                            if b != CTLD {
                                inq.line.push(LF);
                            }
                            let line = core::mem::take(&mut inq.line);
                            inq.lines.extend(line);
                        }
                        _ => {
                            if inq.line.len() < LINE_CAP {
                                inq.line.push(b);
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
                } else {
                    None
                }
            }
        };
        if dropped > 0 {
            // T-05-15's one-line warning (post-lock: Console is a
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
