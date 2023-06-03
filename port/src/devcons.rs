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

/// LockingConsole is the what should be used in almost all cases, as it ensures
/// threadsafe use of the console.
pub struct LockingConsole;

impl LockingConsole {
    /// Create a locking console.  Assumes at this point we can use atomics.
    pub fn new<F>(uart_fn: F) -> Self
    where
        F: FnOnce() -> &'static mut dyn Uart,
    {
        static mut NODE: LockNode = LockNode::new();
        let mut cons = CONS.lock(unsafe { &NODE });
        *cons = Some(uart_fn());
        Self
    }

    pub fn putstr(&mut self, s: &str) {
        // XXX: Just for testing.

        static mut NODE: LockNode = LockNode::new();
        let mut uart_guard = CONS.lock(unsafe { &NODE });
        let uart = uart_guard.as_deref_mut().unwrap();
        for b in s.bytes() {
            putb(uart, b);
        }
    }
}

impl fmt::Write for LockingConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.putstr(s);
        Ok(())
    }
}

/// EarlyConsole should only be used in the very early stages of booting, when
/// we're not sure we can use locks.  This can be particularly useful for
/// implementing an early panic handler.
pub struct EarlyConsole<T>
where
    T: Uart,
{
    uart: T,
}

impl<T> EarlyConsole<T>
where
    T: Uart,
{
    pub fn new(uart: T) -> Self {
        Self { uart }
    }

    pub fn putstr(&mut self, s: &str) {
        // XXX: Just for testing.

        for b in s.bytes() {
            putb(&mut self.uart, b);
        }
    }
}

impl<T> fmt::Write for EarlyConsole<T>
where
    T: Uart,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.putstr(s);
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    // XXX: Just for testing.
    use fmt::Write;
    let mut cons: LockingConsole = LockingConsole {};
    cons.write_fmt(args).unwrap();
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
