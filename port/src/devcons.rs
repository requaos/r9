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
// The registry-facing device surface (05-04, D-03) — attach/walk/open as
// free functions, the established port idiom (devroot.rs's header explains
// why the dat.rs Dev trait is not implemented).  Task 2 lands the minimal
// surface the namespace core's registry + union tests resolve through
// (cons); the full file set (null/user/hostowner) is this plan's Task 3.
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

/// attach: the devcons root chan (one console tree this era — the attach
/// spec is empty, the devroot attach precedent).
pub fn attach() -> crate::dat::Chan {
    crate::dat::Chan::new(
        crate::dat::DEV_CONS,
        crate::dat::Qid { path: QID_ROOT, vers: 0, typ: QTDIR },
        0,
    )
}

/// Walk a single name component from the devcons root (05-04 Task 2's
/// surface: "cons" only; Task 3 grows the file set).
pub fn walk(c: &crate::dat::Chan, name: &[u8]) -> crate::Result<crate::dat::Chan> {
    if c.qid.typ != QTDIR || c.qid.path != QID_ROOT {
        return Err("devcons: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devcons: walk takes a single name component");
    }
    match name {
        b"cons" => {
            let mut nc = crate::dat::Chan::new(
                crate::dat::DEV_CONS,
                crate::dat::Qid { path: QID_CONS, vers: 0, typ: QTFILE },
                0,
            );
            nc.path = c.path.clone();
            nc.path.push(b'/');
            nc.path.extend_from_slice(name);
            Ok(nc)
        }
        _ => Err("devcons: name not found"),
    }
}

/// Open a walked devcons chan.  The root dir opens OREAD only; cons opens
/// any access mode (Plan 9's cons is read/write; the read path itself is
/// 05-08's — an open for read succeeds and reads get the cons arm's
/// answer).
pub fn open(c: &crate::dat::Chan, mode: u32) -> crate::Result<crate::dat::Chan> {
    if c.qid.typ == QTDIR && mode != crate::dat::OREAD {
        return Err("devcons: directory opens OREAD only");
    }
    let mut nc = crate::dat::Chan::new(c.dev, c.qid, mode);
    nc.path = c.path.clone();
    Ok(nc)
}
