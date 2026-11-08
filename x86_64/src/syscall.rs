//! SYSCALL dispatch + the syscall handler set (the aarch64 module's
//! port).  The wire ABI is frozen (Harvey `amd64/ABI.md`): the syscall
//! number arrives in BP (`frame.bp` — the 6c RARG), arguments sit at
//! user `SP+8` in 8-byte slots (SP+0 is the stub's return-address slot;
//! the stub's spilled arg0 lands exactly at SP+8), and the result is
//! written to `frame.ax` before the return tail's SYSRETQ carries it
//! back to user AX.
//!
//! Invariants (carried from the aarch64 header, re-keyed): the
//! implemented set is exactly the measured boot-chain set — every other
//! number is loud-unimplemented (a named unimplemented line + errstr +
//! -1), never wedged.  EVERY user-VA argument validates against the
//! calling proc's segment map through the 14-03 usermem surface BEFORE
//! the kernel reads or writes it, and the STACK RANGE at the captured
//! SP is validated BEFORE the argument copy from SP+8 (the 9k donor's
//! order, syscall.c:248-251 — T-14-06).  Locking: the devcons write
//! path takes the Console MCS lock from syscall context — safe only
//! because IRQ-context code never contends (the timer tick's println is
//! cfg(not(dev_exec))-gated, Pitfall 8) and syscall context runs
//! IRQ-masked.

use crate::dat::Ureg;
use crate::proc;

// The frozen syscall numbers (Harvey sys/src/libc/9syscall/sys.h —
// the shared table 0..54, identical membership on every arch).
// The amd64 stub quirks (ABI.md's frozen rules): nsec (53) emits NO
// arg0 spill — dispatch it with no stack read; seek/exits arrive under
// their _seek/_exits TEXT names with UNCHANGED numbers.
const BIND: u64 = 2;
const CHDIR: u64 = 3;
const CLOSE: u64 = 4;
const DUP: u64 = 5;
const ALARM: u64 = 6;
const EXEC: u64 = 7;
const EXITS: u64 = 8;
const OPEN: u64 = 14;
const SLEEP: u64 = 17;
const RFORK: u64 = 19;
const PIPE: u64 = 21;
const CREATE: u64 = 22;
const FD2PATH: u64 = 23;
const BRK_: u64 = 24;
const NOTIFY: u64 = 28;
const NOTED: u64 = 29;
const RENDEZVOUS: u64 = 34;
const UNMOUNT: u64 = 35;
const SEMACQUIRE: u64 = 37;
const SEMRELEASE: u64 = 38;
const SEEK: u64 = 39;
const FVERSION: u64 = 40;
const ERRSTR: u64 = 41;
const STAT: u64 = 42;
const FSTAT: u64 = 43;
const MOUNT: u64 = 46;
const AWAIT: u64 = 47;
const PREAD: u64 = 50;
const PWRITE: u64 = 51;
const NSEC: u64 = 53;

/// Handler shape: the live Ureg (arg source + result sink) and the
/// calling proc.
type Handler = fn(&mut Ureg, &mut proc::Proc) -> u64;

/// The dispatch table — Harvey sys.h 0-54 verbatim as Option<fn>
/// entries.  None -> loud unimplemented at dispatch time.
static DISPATCH: [Option<Handler>; 55] = {
    let mut t: [Option<Handler>; 55] = [None; 55];
    let _ = (
        &BIND, &CHDIR, &CLOSE, &DUP, &ALARM, &EXEC, &EXITS, &OPEN, &SLEEP, &RFORK, &PIPE,
        &CREATE, &FD2PATH, &BRK_, &NOTIFY, &NOTED, &RENDEZVOUS, &UNMOUNT, &SEMACQUIRE,
        &SEMRELEASE, &SEEK, &FVERSION, &ERRSTR, &STAT, &FSTAT, &MOUNT, &AWAIT, &PREAD,
        &PWRITE, &NSEC,
    );
    t
};

/// The dispatch: number from frame.bp, handler result to frame.ax.
pub fn dispatch(frame: &mut Ureg) {
    let Some(mut p) = proc::current() else {
        panic!("syscall dispatch with no current process (n={})", frame.bp);
    };
    dispatch_proc(frame.bp, frame, &mut p);
}

/// The dispatch core (everything `dispatch` does after the
/// current-proc fetch — the module's host-test seam: tests substitute a
/// table-resident proc for the global).  (RED skeleton — degenerate.)
pub(crate) fn dispatch_proc(n: u64, frame: &mut Ureg, proc: &mut proc::Proc) {
    let _ = (n, frame, proc);
}

/// NSEC(53)'s clock source seam (the aarch64 platform::rtc role).
/// Returns None -> the loud "no RTC" errstr.
pub(crate) fn nsec_now() -> Option<u64> {
    #[cfg(test)]
    {
        let v = tests::TEST_NSEC.load(core::sync::atomic::Ordering::Relaxed);
        if v != u64::MAX {
            return Some(v);
        }
        None
    }
    #[cfg(not(test))]
    {
        None
    }
}

/// EXEC's argv staging (path + the NUL-terminated argv block) out of
/// user memory.  (RED stub.)
pub(crate) fn copy_exec_args(
    proc: &proc::Proc,
    path_va: u64,
    argv_va: u64,
) -> Result<(alloc::vec::Vec<u8>, alloc::vec::Vec<alloc::vec::Vec<u8>>), &'static str> {
    let _ = (proc, path_va, argv_va);
    Err("exec: stub")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proc::{MappingRecord, ProcGuard, SegKind, Segments};
    use alloc::vec::Vec;
    use port::mem::{PAGE_SIZE_4K, VirtRange};

    /// The test clock hook (nsec's seam — u64::MAX = unset).
    pub(crate) static TEST_NSEC: core::sync::atomic::AtomicU64 =
        core::sync::atomic::AtomicU64::new(u64::MAX);

    /// A table-resident proc whose "user stack" is a REAL leaked host
    /// buffer: the segments' stack range IS the buffer's address range
    /// (so the usermem constructors' raw materializations genuinely hit
    /// these bytes), and the mapping_log covers the page so
    /// ensure_user_resident never page-ins (the 14-03 mock doctrine on
    /// a real Proc — no live CR3, no Mach).
    struct MockStack {
        guard: ProcGuard,
        buf: &'static mut [u8; 256],
    }

    impl MockStack {
        fn new(name: &'static str) -> MockStack {
            let mut guard = crate::proc::new_user_proc(name).expect("construction succeeds");
            let buf: &'static mut [u8; 256] =
                alloc::boxed::Box::leak(alloc::boxed::Box::new([0u8; 256]));
            let base = buf.as_ptr() as usize;
            let empty = || VirtRange::with_len(0x1000_0000, 0);
            guard.segments = Some(Segments {
                text: empty(),
                data: empty(),
                bss: empty(),
                stack: VirtRange::with_end(base, base + buf.len()),
                tos: empty(),
            });
            guard.mapping_log.push(MappingRecord {
                va: base & !(PAGE_SIZE_4K - 1),
                phys: base,
                size: PAGE_SIZE_4K,
                kind: SegKind::Stack,
            });
            MockStack { guard, buf }
        }

        /// The buffer's base VA (the mock's "user" address).
        fn base(&self) -> u64 {
            self.buf.as_ptr() as u64
        }

        /// Write a u64 slot at byte offset `off` in the mock stack.
        fn put(&mut self, off: usize, v: u64) {
            self.buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
        }

        fn proc(&mut self) -> &mut crate::proc::Proc {
            &mut self.guard
        }
    }

    /// ERRSTR(41) routed from frame.bp: args materialize from SP+8
    /// UPWARD (never SP+0 — the poison qword proves the slot source),
    /// the result lands in the AX slot, and the errstr bytes land in
    /// the user buffer.
    #[test]
    fn dispatch_routes_args_from_sp8_upward_into_ax() {
        let mut mock = MockStack::new("t1");
        let mut frame = Ureg::zeroed();
        frame.bp = ERRSTR; // the wire number's slot
        frame.sp = mock.base();
        // SP+0: the return-address slot — POISON.  A generic reader
        // that started here would use 0xdead as the buffer pointer and
        // the call would fail with a bad-buffer error.
        mock.put(0, 0xdead);
        // SP+8: arg0 = the errstr buffer's VA (well inside the stack).
        mock.put(8, mock.base() + 64);
        // SP+16: arg1 = n.
        mock.put(16, 32);
        mock.proc().set_errstr("boom");
        dispatch_proc(frame.bp, &mut frame, mock.proc());
        // Result in the AX slot (byte 0 of the frame), nowhere else —
        // bp carries the number, unchanged.
        assert_eq!(frame.ax, 0);
        assert_eq!(frame.bp, ERRSTR);
        // The user buffer holds the errstr, NUL-terminated.
        assert_eq!(&mock.buf[64..69], b"boom\0");
    }

    /// The arg-block window [sp, sp+8+8*arity) is validated BEFORE any
    /// slot is copied: a window that overflows the segment fails the
    /// whole call with the named bad-argument-block error.
    #[test]
    fn arg_block_window_validates_before_copy() {
        let mut mock = MockStack::new("t2");
        let mut frame = Ureg::zeroed();
        frame.bp = ERRSTR;
        // sp sits 8 bytes before the segment end: even a one-arg call
        // needs [sp, sp+24) — outside.  The handler must fail before
        // touching any slot.
        frame.sp = mock.base() + 256 - 8;
        mock.put(0, 0);
        mock.put(8, mock.base() + 64);
        mock.put(16, 8);
        dispatch_proc(frame.bp, &mut frame, mock.proc());
        assert_eq!(frame.ax, -1i64 as u64);
        let e = &mock.proc().errstr;
        let n = e.iter().position(|&b| b == 0).unwrap_or(crate::proc::ERRMAX);
        assert_eq!(
            core::str::from_utf8(&e[..n]).unwrap(),
            "errstr: bad argument block"
        );
    }

    /// An unimplemented number (9 = _FSESSION, in-table None) is LOUD:
    /// the named unimplemented errstr plus -1 in the result slot —
    /// never a silent 0.
    #[test]
    fn unimplemented_number_is_loud() {
        let mut mock = MockStack::new("t3");
        let mut frame = Ureg::zeroed();
        frame.bp = 9; // _FSESSION — outside the measured set
        frame.sp = mock.base();
        dispatch_proc(frame.bp, &mut frame, mock.proc());
        assert_eq!(frame.ax, -1i64 as u64, "the wire result is -1");
        let e = &mock.proc().errstr;
        let n = e.iter().position(|&b| b == 0).unwrap_or(crate::proc::ERRMAX);
        assert_eq!(
            core::str::from_utf8(&e[..n]).unwrap(),
            "syscall not implemented"
        );
        assert_eq!(frame.bp, 9, "the number slot is not the result slot");
    }

    /// NSEC(53) is served from the frame with NO user-stack read: a
    /// seeded frame whose SP+0 would poison a generic reader still
    /// returns the clock — zero-arg register-return (Pitfall 5).
    #[test]
    fn nsec_reads_no_user_stack() {
        TEST_NSEC.store(1_700_000_000_123_456_789, core::sync::atomic::Ordering::Relaxed);
        let mut mock = MockStack::new("t4");
        let mut frame = Ureg::zeroed();
        frame.bp = NSEC;
        frame.sp = mock.base();
        mock.put(0, 0xdead_beef); // SP+0 poison — must never be read
        mock.put(8, 0xdead_beef);
        dispatch_proc(frame.bp, &mut frame, mock.proc());
        assert_eq!(frame.ax, 1_700_000_000_123_456_789u64);
        // The errstr side channel is untouched (no failed arg block).
        assert_eq!(mock.proc().errstr[0], 0);
    }

    /// copy_exec_args preserves argv ORDER verbatim (the pointers walk
    /// the user argv array ascending; each string copies in order).
    #[test]
    fn exec_arg_staging_preserves_order() {
        let mut mock = MockStack::new("t5");
        // The argv block: three pointers then nil; the strings sit at
        // the buffer's tail; the path string at +208.
        let base = mock.base();
        let (s0, s1, s2) = (base + 160, base + 176, base + 192);
        mock.put(0, s0);
        mock.put(8, s1);
        mock.put(16, s2);
        mock.put(24, 0);
        mock.buf[160..168].copy_from_slice(b"one\0aaaa");
        mock.buf[176..184].copy_from_slice(b"two\0bbbb");
        mock.buf[192..200].copy_from_slice(b"three\0cc");
        mock.buf[208..212].copy_from_slice(b"pth\0");
        let (path, argv) = copy_exec_args(mock.proc(), base + 208, base).expect("staging");
        assert_eq!(path, b"pth");
        let got: Vec<&[u8]> = argv.iter().map(|a| a.as_slice()).collect();
        assert_eq!(got, vec![b"one".as_slice(), b"two".as_slice(), b"three".as_slice()]);
    }

    /// The EXEC engine seeds the user stack with the argv block
    /// ORDER-STABLE: [argc][argv0..N][nil] with the pointers naming the
    /// strings in argument order, byte-verbatim (the ordering edge the
    /// plan's behavior block pins — the aarch64 seed layout: strings
    /// packed top-down from the seed page's top, the pointer array
    /// 16-aligned below them).
    #[test]
    fn exec_replace_seeds_argv_order_verbatim() {
        use crate::exec::{HDR_SIZE, INITTEXT, S_MAGIC_AMD64, USTACKTOP};
        use crate::proc::ByteBacking;
        use alloc::sync::Arc;

        let mut mock = MockStack::new("t6");
        // A synthetic 6l-shaped binary: magic, text, data, bss, syms,
        // entry = INITTEXT.
        let mut bin = alloc::vec![0u8; HDR_SIZE + 0x1000 + 0x100];
        bin[0..4].copy_from_slice(&S_MAGIC_AMD64.to_be_bytes());
        bin[4..8].copy_from_slice(&0x1000u32.to_be_bytes());
        bin[8..12].copy_from_slice(&0x100u32.to_be_bytes());
        bin[12..16].copy_from_slice(&0x80u32.to_be_bytes());
        bin[32..40].copy_from_slice(&(INITTEXT as u64).to_be_bytes());
        let backing = ByteBacking::Mnt { file: Arc::new(bin) };
        let mut frame = Ureg::zeroed();
        let args: Vec<&[u8]> = vec![b"/amd64/init".as_slice(), b"-e".as_slice()];
        let tos_va = crate::exec::exec_replace(mock.proc(), backing, "init", &args, &mut frame)
            .expect("exec_replace succeeds");
        assert_ne!(tos_va, 0);
        // The seeded user stack: the Stack mapping's page, read through
        // the seam (the seed page is host memory in tests).
        let rec = mock
            .proc()
            .mapping_log
            .iter()
            .find(|m| m.kind == SegKind::Stack)
            .expect("a stack seed page")
            .clone();
        let base = USTACKTOP - PAGE_SIZE_4K;
        let page = crate::proc::phys_ptr_mut(rec.phys as u64) as *const u8;
        let user_sp = frame.sp as usize;
        assert!(user_sp > base && user_sp < base + PAGE_SIZE_4K);
        // SAFETY: `page` names the seam-allocated seed page; every read
        // below is inside its 4 KiB.
        unsafe {
            let at = |va: usize| -> u64 { page.add(va - base).cast::<u64>().read_unaligned() };
            // The pointer array: [argc][argv0][argv1][nil].
            assert_eq!(at(user_sp), 2, "argc");
            let a0 = at(user_sp + 8);
            let a1 = at(user_sp + 16);
            assert_eq!(at(user_sp + 24), 0, "the nil terminator");
            // The strings, in argument order, byte-verbatim.
            let s0 = page.add(a0 as usize - base);
            assert_eq!(core::slice::from_raw_parts(s0, 13), b"/amd64/init\0");
            let s1 = page.add(a1 as usize - base);
            assert_eq!(core::slice::from_raw_parts(s1, 3), b"-e\0");
            // The seed packs the strings top-down in ARGUMENT order:
            // argv[0] sits above argv[1], both below the page top.
            assert!(a1 < a0);
            assert!(a0 as usize + 13 <= base + PAGE_SIZE_4K);
        }
        // The live frame was rewritten to the new image's entry.
        assert_eq!(frame.pc as usize, INITTEXT);
        // bp carries the seeded Tos (the RARG first-arg convention).
        assert_eq!(frame.bp, tos_va);
    }
}
