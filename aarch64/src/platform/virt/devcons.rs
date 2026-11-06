// Racy to start.

use core::cell::SyncUnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::pl011::Pl011Uart;
use port::devcons::Console;
use port::fdt::DeviceTree;
#[cfg(not(test))]
use port::println;

pub fn init(dt: &DeviceTree) {
    Console::set_uart(|| {
        let uart = Pl011Uart::new(dt);

        // Return a statically initialised Pl011Uart.  If that couldn't be
        // done for some reason, return Err and hope that things work out
        // regardless.
        match uart {
            Ok(uart) => {
                static UART: SyncUnsafeCell<MaybeUninit<Pl011Uart>> =
                    SyncUnsafeCell::new(MaybeUninit::uninit());
                // SAFETY: set_uart runs once at platform init (single CPU,
                // pre-IRQ-unmask); the cell is written before its address
                // escapes, and the published &'static dyn Uart only ever
                // reads it.
                unsafe {
                    let cons = &mut *UART.get();
                    cons.write(uart);
                    let cons: &'static Pl011Uart = cons.assume_init_mut();
                    Ok(cons)
                }
            }
            Err(msg) => {
                println!("can't initialise uart: {msg:?}");
                Err("can't initialise uart")
            }
        }
    });
}

/// devcons device write: the syscall PWRITE path for devcons
/// chans (fd 1/2, opened "cons").  Byte-verbatim through the Console MCS
/// lock + PL011 polled TX — shares the kernel print path's lock safely
/// because the timer tick never prints under dev_exec (timer.rs
/// gates it), so no IRQ-context code contends while this runs.
pub fn cons_write(buf: &[u8]) -> u64 {
    port::devcons::write_bytes(buf) as u64
}

/// devcons device read: drain the PL011 RX ring into `buf`,
/// returning the byte count — 0 on empty (no blocking console-read this
/// phase).  Under cfg(not(dev_exec)) no RX ring exists (bytes echo
/// instead), and no EL0 reader can exist either — return 0.
#[cfg(dev_exec)]
pub fn cons_read(buf: &mut [u8]) -> u64 {
    super::pl011::cons_rx_drain(buf) as u64
}

#[cfg(not(dev_exec))]
pub fn cons_read(_buf: &mut [u8]) -> u64 {
    0
}

// ---------------------------------------------------------------------------
// The D-17 random device's arch halves (08-06): the DTB rng-seed read and
// the generic-timer jitter sample.  main9 records the DTB's virtual
// address at boot (the DTB is reserved in pagealloc and its KZERO+PA
// mapping is never torn down), so the seed property is re-parsed LAZILY —
// at the first /dev/random read, via port::fdt, exactly the plan's
// "seeded at first read from the DTB /chosen/rng-seed property via
// fdt.rs".
// ---------------------------------------------------------------------------

/// The DTB's virtual address, recorded by main9 (0 = not yet recorded).
static DTB_VA: AtomicUsize = AtomicUsize::new(0);

/// Record the DTB's virtual address for the lazy rng-seed parse.  Called
/// once from main9 (single CPU, pre-IRQ-unmask).
pub fn set_dtb_va(va: usize) {
    DTB_VA.store(va, Ordering::Relaxed);
}

/// devcons's rng_seed hook: copy /chosen/rng-seed's bytes into `buf`,
/// returning the count (0 when absent — the jitter mixing then carries
/// the whole seed, the plan's documented fallback).  The DTB is immutable
/// post-boot, so a re-parse at first read sees the same bytes main9 saw.
pub fn rng_seed_read(buf: &mut [u8]) -> usize {
    let va = DTB_VA.load(Ordering::Relaxed);
    if va == 0 {
        return 0;
    }
    // SAFETY: va names the DTB mapped at KZERO+PA by vminit (still live —
    // the TTBR1 kernel map is never torn down) and reserved in pagealloc;
    // the header prefix sits inside the first mapped page and the full
    // totalsize range is inside the mapping (main9's own two-pass shape).
    let dt = unsafe {
        let prefix = core::slice::from_raw_parts(va as *const u8, port::fdt::FDT_HEADER_SIZE);
        let Ok(total) = port::fdt::DeviceTree::header_totalsize(prefix) else {
            return 0;
        };
        let Ok(dt) = port::fdt::DeviceTree::new(core::slice::from_raw_parts(va as *const u8, total))
        else {
            return 0;
        };
        dt
    };
    let Some(chosen) = dt.find_by_path("/chosen") else {
        return 0;
    };
    let Some(prop) = dt.property(&chosen, "rng-seed") else {
        return 0;
    };
    let Some(bytes) = dt.property_value_bytes(&prop) else {
        return 0;
    };
    let n = bytes.len().min(buf.len());
    buf[..n].copy_from_slice(&bytes[..n]);
    n
}

/// devcons's jitter hook: CNTFRQ ^ CNTPCT ^ the 1 ms tick count.  CNTFRQ
/// is the plan's named term (the runtime-read frequency word — 62.5 MHz
/// TCG / 24 MHz HVF, a host-supplied boot constant); CNTPCT is the term
/// that actually varies between back-to-back reads within one tick; the
/// tick count folds in wall time.
pub fn random_jitter() -> u64 {
    use aarch64_cpu::registers::{CNTFRQ_EL0, CNTPCT_EL0, Readable};
    CNTFRQ_EL0.get() ^ CNTPCT_EL0.get() ^ super::timer::ticks()
}
