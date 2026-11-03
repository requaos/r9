//! SVC dispatch + the measured syscall handler set (04-06 — R9-02's
//! dispatch half; D-14 measure-first).
//!
//! The wire ABI is frozen (Harvey `aarch64/ABI.md`): the syscall number
//! arrives in R0 (`frame.x0`), arguments sit at user `SP+8` in 8-byte
//! slots (`SP+0` is the reserved LR slot the stub's `MOV R0, 0(FP)`
//! materializes), the result is written back to R0 before the shared
//! trap-return tail erets into EL0.
//!
//! The implemented set is EXACTLY the 04-01 donor trace
//! (04-01-d14-trace.txt: 51 pwrite, 14 open, 50 pread, 4 close, 8 exits)
//! plus D-14's named SC#4 core members EXEC(7), RFORK(19), AWAIT(47) and
//! the 04-08 notes pair NOTIFY(28)/NOTED(29) — zero guessed syscalls.  The
//! trace lists NO brk_(24)/segbrk(12) (the D-14 fmt/pool watch item settled
//! by evidence), so they stay None.  Every other number prints
//! `syscall N unimplemented` and returns -1 — loud, never wedged (D-14).
//!
//! T-04-11: EVERY user-VA argument (the Sargs block itself, buffers, path
//! strings, argv arrays) is validated against the calling proc's segment
//! map via `Proc::validate_user_range` BEFORE the kernel reads or writes
//! it; invalid → proc.errstr + -1, never a kernel dereference of an
//! unchecked pointer.  (The segment-level caveat — an in-segment but
//! not-yet-faulted VA would abort at EL1h loudly — is documented on
//! validate_user_range; the measured binaries' buffers are always resident
//! by call time.)
//!
//! Locking (04-RESEARCH.md Pitfall 3): the devcons write path takes the
//! Console MCS lock from syscall context.  That is safe because the timer
//! tick's println is cfg(not(dev_exec))-gated — no IRQ-context code ever
//! contends for the lock once syscalls can print.  Syscall/trap context
//! runs IRQ-masked, so the PL011 RX ring's IRQ writer cannot interleave
//! with the console-read drainer either.  05-08 extends the rule: the
//! devcons INPUT queue lock (the read path's cooked/raw state) is
//! syscall/trap context ONLY — IRQ context never takes it; the PL011 RX
//! IRQ handler only pushes its lock-free ring, and the ring→queue drain
//! runs in the reader's own syscall context (a blocked reader is readied
//! by the tick, timer.rs → proc::cons_input_wake — the reschedule()
//! PROC_TABLE-from-trap-context precedent).

use alloc::vec::Vec;

use port::dat::{Chan, DEV_CONS, DEV_DUP, DEV_ENV, DEV_MNT, DEV_PIPE, DEV_PROC, DEV_ROOT, DEV_SRV, OEXEC, OTRUNC};
use port::mcslock::LockNode;
use port::namec::{self, NamecMode};
use port::pgrp::{MAFTER, MBEFORE, MMASK, MORDER};
use port::println;

use crate::exec;
use crate::fault;
use crate::proc::{self, ByteBacking, ERRMAX, Proc, ProcState};
use crate::trap;
use crate::trap::TrapFrame;

// The frozen syscall numbers (Harvey sys/src/libc/9syscall/sys.h).
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
const UNMOUNT: u64 = 35;
const SEEK: u64 = 39;
const FVERSION: u64 = 40;
const ERRSTR: u64 = 41;
const STAT: u64 = 42;
const FSTAT: u64 = 43;
const MOUNT: u64 = 46;
const AWAIT: u64 = 47;
const PREAD: u64 = 50;
const PWRITE: u64 = 51;

// rfork flags (Harvey sys/include/libc.h — frozen values; 04-07).
// pub(crate) since 05-04: proc.rs's rfork_clone reads RFNAMEG/RFCNAMEG
// (the D-03 namespace group semantics) — one mirror, one provenance.
// RFENVG/RFCENVG joined the pub(crate) set in 05-05 (D-07 env groups);
// RFFDG/RFCFDG in 05-06 (the fd-group arms).
pub(crate) const RFNAMEG: u64 = 1 << 0;
pub(crate) const RFENVG: u64 = 1 << 1;
pub(crate) const RFFDG: u64 = 1 << 2;
/// pub(crate) since 05-08: proc.rs's rfork_clone/rfork_noteg_split read it
/// (D-06's note groups — the console DEL target scoping).
pub(crate) const RFNOTEG: u64 = 1 << 3;
const RFPROC: u64 = 1 << 4;
const RFMEM: u64 = 1 << 5;
pub(crate) const RFNOWAIT: u64 = 1 << 6;
pub(crate) const RFCNAMEG: u64 = 1 << 10;
pub(crate) const RFCENVG: u64 = 1 << 11;
pub(crate) const RFCFDG: u64 = 1 << 12;
const RFREND: u64 = 1 << 13;

/// Handler shape (04-RESEARCH.md Pattern 2 / Code Examples): the live trap
/// frame (arg source + result sink) and the calling proc.
type Handler = fn(&mut TrapFrame, &mut Proc) -> u64;

/// The dispatch table — Harvey sys.h 0–54 verbatim as Option<fn> entries.
/// None → loud unimplemented at dispatch time (D-14).  The implemented set
/// is the 04-01 measured trace {pwrite(51), open(14), pread(50), close(4),
/// exits(8)} plus D-14's named SC#4 core members EXEC(7), the 04-07 process
/// pair RFORK(19)/AWAIT(47), and 04-08's notes pair NOTIFY(28)/NOTED(29),
/// grown by 05-04 with the 05-03 real-chain trace's namespace members
/// BIND(2), CHDIR(3), UNMOUNT(35) (unmount ×1 — boot's settime /tmp
/// handling) and its Task-3 additions: the measured stat family STAT(42)/
/// FSTAT(43) and BRK_(24) (×28 — malloc pool growth; boot's dirstat
/// mallocs make it gate-load-bearing per the plan dispatch note); and by
/// 05-06 with the connectembed family DUP(5)/PIPE(21)/CREATE(22)/
/// FD2PATH(23) (boot's pipe+fork+dup+exec of the paqfs server +
/// srvcreate) and SEEK(39) (measured ×72 — paqfs's bio Bseek; surfaced
/// by the 05-06 gate as the trace-settled gap the loud wall named); and by
/// 05-07 with D-03's mount half FVERSION(40)/MOUNT(46) — the devmnt
/// session negotiation + the mount-table insertion (FAUTH(10) stays None:
/// nsinit's fall-through to an unauthenticated mount is the designed D-02
/// path); and by 05-08 with the trace-measured timer pair ALARM(6)
/// (init's alarm(0) disarm) + SLEEP(17) (×42 in the 05-03 trace — init's
/// for(;;) restart loop sleeps 1s between rc generations) riding the
/// generic timer (timer.rs ticks; expiry via the tick's tick_timeouts).
/// The 04-01 trace carries no
/// brk_/segbrk(12) on the hello path; segbrk stays None.
static DISPATCH: [Option<Handler>; 55] = {
    let mut t: [Option<Handler>; 55] = [None; 55];
    t[BIND as usize] = Some(sys_bind);
    t[CHDIR as usize] = Some(sys_chdir);
    t[CLOSE as usize] = Some(sys_close);
    t[DUP as usize] = Some(sys_dup);
    t[ALARM as usize] = Some(sys_alarm);
    t[EXEC as usize] = Some(sys_exec);
    t[EXITS as usize] = Some(sys_exits);
    t[OPEN as usize] = Some(sys_open);
    t[SLEEP as usize] = Some(sys_sleep);
    t[RFORK as usize] = Some(sys_rfork);
    t[PIPE as usize] = Some(sys_pipe);
    t[CREATE as usize] = Some(sys_create);
    t[FD2PATH as usize] = Some(sys_fd2path);
    t[NOTIFY as usize] = Some(sys_notify);
    t[NOTED as usize] = Some(sys_noted);
    t[BRK_ as usize] = Some(sys_brk_);
    t[UNMOUNT as usize] = Some(sys_unmount);
    t[SEEK as usize] = Some(sys_seek);
    t[FVERSION as usize] = Some(sys_fversion);
    t[ERRSTR as usize] = Some(sys_errstr);
    t[STAT as usize] = Some(sys_stat);
    t[FSTAT as usize] = Some(sys_fstat);
    t[MOUNT as usize] = Some(sys_mount);
    t[AWAIT as usize] = Some(sys_await);
    t[PREAD as usize] = Some(sys_pread);
    t[PWRITE as usize] = Some(sys_pwrite);
    t
};

/// The EC 0x15 entry (trap.rs routes here): number from frame.x0, dispatch,
/// result to frame.x0 (the restore tail's `ldp x0` pop propagates it to
/// user R0 — the mechanism exists since Phase 3).
pub fn dispatch(frame: &mut TrapFrame) {
    let proc_ptr = proc::current_proc();
    if proc_ptr.is_null() {
        // An SVC with no current process is a kernel-orchestration bug
        // (dev_exec execs /init before any EL0 exists), never a user error.
        panic!("syscall dispatch with no current process (n={})", frame.x0);
    }
    let proc = unsafe { &mut *proc_ptr };
    let n = frame.x0;
    let result = match DISPATCH.get(n as usize) {
        Some(Some(handler)) => handler(frame, proc),
        _ => {
            println!("syscall {n} unimplemented");
            proc.set_errstr("syscall not implemented");
            -1i64 as u64
        }
    };
    // NOTED's success path rewrote the live frame (the restored Ureg IS
    // the result) — the donor dosyscall special case (9front sysproc.c
    // `if(scallnr == NOTED) ... return 1`): dispatch returns WITHOUT the
    // frame.x0 writeback.  NOTED's ERROR returns (-1) left the frame
    // untouched and DO write back; NDFLT diverged inside the handler.
    if n == NOTED && result != -1i64 as u64 {
        return;
    }
    // EXITS never reaches here (it parks divergent); every other handler's
    // result rides frame.x0 back to user R0.
    frame.x0 = result;
}

/// Read the argument slots from user memory at SP+8 (the D-07 published
/// formula: `(Sargs*)(frame.sp + 8)`; SP+0 is the reserved LR slot).  The
/// arg block is user memory too, so it is validated first (T-04-11).
///
/// Width discipline (04-07, donor-faithful): every slot is 8 bytes, but
/// int/uint-width arguments are stored by 7c with a 32-bit store that
/// leaves the slot's upper half as stack garbage — the donor's
/// `va_arg(list, int/uint)` reads only the slot's low half
/// (9front@350266104f arm64/include/u.h: `va_arg(list, mode)` with
/// sizeof(mode) == 4 reads `((mode*)list)[-2]`), so handlers must
/// likewise mask int-width args to 32 bits (await's buflen, errstr's
/// n, rfork's flag, the fd args, open's mode).  Pointer/vlong args
/// occupy the full slot.  **`long` is 32-bit in Plan 9 C on every arch
/// (7c SZ_LONG=4; only `vlong` is 64-bit) — pread/pwrite's `long n` is
/// a 32-bit arg on the wire and must be masked too** (05-01: the
/// stacksmoke deep recursion left pointer garbage in the reused stack
/// pages, so an unmasked n read as a huge value and the write failed
/// silently — shallow stacks never saw it because fresh demand-paged
/// pages are zeroed).  (Arg0 is always clean: the stub materializes it
/// from R0, whose upper half the caller's 32-bit op already zeroed.)
fn sargs(frame: &mut TrapFrame, proc: &mut Proc, arity: usize) -> Option<[u64; 5]> {
    let base = frame.sp + 8;
    if proc.validate_user_range(base, 8 * arity as u64, false).is_err() {
        return None;
    }
    let sargs = base as *const u64;
    let mut out = [0u64; 5];
    for (i, slot) in out.iter_mut().enumerate().take(arity) {
        *slot = unsafe { sargs.add(i).read() };
    }
    Some(out)
}

/// Copy a NUL-terminated user string into a kernel Vec, validating each
/// byte's page membership BEFORE reading it (T-04-11) and bounding the
/// length at `max` (ERRMAX-ish — Plan 9's string ceiling).  The returned
/// bytes EXCLUDE the terminator.
fn copy_user_str(proc: &Proc, va: u64, max: usize) -> Result<Vec<u8>, &'static str> {
    let mut out = Vec::new();
    if va == 0 {
        return Err("null string pointer");
    }
    loop {
        if out.len() >= max {
            return Err("string too long");
        }
        proc.validate_user_range(va + out.len() as u64, 1, false)?;
        let b = unsafe { ((va + out.len() as u64) as *const u8).read() };
        if b == 0 {
            return Ok(out);
        }
        out.push(b);
    }
}

/// Error return: set the proc's errstr, return -1 on the wire.
fn err(proc: &mut Proc, msg: &'static str) -> u64 {
    proc.set_errstr(msg);
    -1i64 as u64
}

/// A validated user buffer as a kernel-readable slice.  The calling proc's
/// TTBR0 is live (we are in its syscall), so user VAs are directly
/// dereferenceable; `validate_user_range` has already run.
unsafe fn user_slice(va: u64, n: u64) -> &'static [u8] {
    unsafe { core::slice::from_raw_parts(va as *const u8, n as usize) }
}

/// The writable twin of user_slice.
unsafe fn user_slice_mut(va: u64, n: u64) -> &'static mut [u8] {
    unsafe { core::slice::from_raw_parts_mut(va as *mut u8, n as usize) }
}

/// PWRITE(51): fd, buf, n, offset.  Offset -1 (u64::MAX) is the streaming
/// channel offset (Harvey libc write() = pwrite(fd, buf, n, -1LL) — Pitfall
/// 7); the console ignores offsets (writes append), devroot is read-only,
/// devenv honors them (05-05: donor envwrite's grow-at-offset + streaming
/// advance), devpipe ignores them (05-06: donor qwrite — pipes are
/// streaming, there are no offsets).  n is a `long` — 32-bit in Plan 9 C
/// (SZ_LONG=4), so the wire slot's upper half is 7c garbage and must be
/// masked (the width discipline above).
///
/// 05-06: the fd is CHECKED OUT (fd_take) for the op and put back after
/// (fd_store) — a blocking pipe write never holds the fd-group lock
/// (fdgrp.rs's checked-out-chan pattern).
fn sys_pwrite(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "pwrite: bad argument block");
    };
    let (fd, buf, n, off) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff, args[3]); // fd is an int
    if proc.validate_user_range(buf, n, false).is_err() {
        return err(proc, "pwrite: bad user buffer");
    }
    // 05-06 (the cold-buffer class): force the read source resident
    // BEFORE the device reads it — a brk_-grown/stack-growth page that
    // no EL0 code has touched aborts at EL1h on a kernel read (04-08's
    // explicit-residency pattern generalized; T-05-06: never a nested
    // fault).
    fault::ensure_user_resident(proc, buf as usize, (buf + n) as usize);
    let Some(mut chan) = proc.fd_take(fd) else {
        return err(proc, "pwrite: bad fd");
    };
    let result = match chan.dev {
        DEV_CONS => {
            // 05-04 Task 3: the device routes by qid — cons writes
            // byte-exact, null discards, hostowner stores (D-02), user is
            // read-only (the whole per-file contract lives in the device).
            port::devcons::write(&mut chan, unsafe { user_slice(buf, n) })
        }
        DEV_ROOT => Err("devroot: read-only filesystem"),
        DEV_PROC => {
            // devproc-lite (04-08): the note append is the whole write —
            // the chan's qid names the target pid; offsets don't apply.
            port::devproc::write(&mut chan, unsafe { user_slice(buf, n) })
        }
        DEV_ENV => {
            // devenv (05-05, D-07): the write lands in the OPERATING
            // proc's env group (the device resolves it via the
            // registered hook — donor envgrp(c)); offsets per envwrite
            // (streaming offset honored, grow-at-offset).
            port::devenv::write(&mut chan, unsafe { user_slice(buf, n) }, off)
        }
        DEV_PIPE => {
            // devpipe (05-06): end i writes q[1-i]; a full queue is a
            // named error (never partial, never silent); blocked readers
            // wake after the bytes land.
            port::devpipe::write(&mut chan, unsafe { user_slice(buf, n) }, off)
        }
        DEV_DUP => Err("devdup: not writable (open #d/N to reach the fd)"),
        DEV_SRV => {
            // devsrv (05-06): writing a decimal fd number to a service
            // file registers it (boot's srvcreate — FATAL on failure, so
            // this must actually work).
            port::devsrv::write(&mut chan, unsafe { user_slice(buf, n) }, off)
        }
        DEV_MNT => {
            // devmnt (05-07): paqfs is a read-only server; the early
            // client-side error is clearer than the server's Erdonly.
            Err("devmnt: the mounted tree is read-only this era")
        }
        _ => Err("pwrite: unknown device"),
    };
    proc.fd_store(fd, chan);
    match result {
        Ok(wrote) => wrote,
        Err(msg) => err(proc, msg),
    }
}

/// PREAD(50): fd, buf, n, offset.  devroot chans read via the paqfs
/// reader's byte-granular read_at (offset -1 uses + advances the chan's
/// streaming offset); devcons chans route by qid (cons reads are the loud
/// 05-08 -1); devenv honors offsets; devpipe reads the end's queue —
/// blocking on empty (05-06, the 04-07 block/wake machinery via the
/// injected hooks).  n is a `long` — 32-bit in Plan 9 C, so the wire
/// slot's upper half is masked (same discipline as sys_pwrite).
///
/// 05-06: the fd is CHECKED OUT (fd_take) for the op and put back after
/// (fd_store) — a blocking pipe read never holds the fd-group lock.
fn sys_pread(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "pread: bad argument block");
    };
    let (fd, buf, n, off) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff, args[3]); // fd is an int
    if proc.validate_user_range(buf, n, true).is_err() {
        return err(proc, "pread: bad user buffer");
    }
    // 05-06: force the write target resident BEFORE the device fills it
    // (the cold-buffer class — paqfs's emalloc'd heap buffers are
    // demand-paged and first-touched by this kernel write).
    fault::ensure_user_resident(proc, buf as usize, (buf + n) as usize);
    let Some(mut chan) = proc.fd_take(fd) else {
        return err(proc, "pread: bad fd");
    };
    let result = match chan.dev {
        DEV_CONS => {
            // 05-04 Task 3: the device routes by qid — the root dir serves
            // the stat-wire listing, null reads EOF, user reads glenda,
            // hostowner reads the stored value, and cons reads get the
            // loud "05-08" -1 (never a wedge).  The streaming offset
            // advances inside the device (the pread convention).
            port::devcons::read(&mut chan, unsafe { user_slice_mut(buf, n) }, off)
        }
        DEV_ROOT => {
            let Some(root) = port::devroot::root() else {
                proc.fd_store(fd, chan);
                return err(proc, "devroot: not mounted");
            };
            root.read(&mut chan, unsafe { user_slice_mut(buf, n) }, off)
        }
        DEV_PROC => Err("devproc: note files are write-only"),
        DEV_ENV => {
            // devenv (05-05, D-07): the read comes from the OPERATING
            // proc's env group (hook-resolved — donor envgrp(c)); the
            // root dir serves the group's stat-wire listing, a key serves
            // its value (clamped, EOF = 0); streaming offset honored.
            port::devenv::read(&mut chan, unsafe { user_slice_mut(buf, n) }, off)
        }
        DEV_PIPE => {
            // devpipe (05-06): read the end's queue — drains available
            // bytes, blocks on empty (wake on write/close), EOF on
            // hangup.  Never touches the chan offset (streaming).
            port::devpipe::read(&mut chan, unsafe { user_slice_mut(buf, n) }, off)
        }
        DEV_DUP => {
            // devdup (05-06): the root dir listing of the CALLING proc's
            // open fds (rcmain's `. -i '#d/0'` opens the fd file — its
            // reads dispatch to the REFERENCED chan's device, not here).
            port::devdup::read(&mut chan, unsafe { user_slice_mut(buf, n) }, off)
        }
        DEV_SRV => {
            // devsrv (05-06): the root dir listing of registered
            // services (a service file's reads flow through its posted
            // chan — open returns it, the donor srvopen).
            port::devsrv::read(&mut chan, unsafe { user_slice_mut(buf, n) }, off)
        }
        DEV_MNT => {
            // devmnt (05-07): chunked Tread RPCs over the mounted tree's
            // session (blocks mid-syscall — the pipe read schedules the
            // server proc; the fd is checked out, never the group lock).
            port::devmnt::read(&mut chan, unsafe { user_slice_mut(buf, n) }, off)
        }
        _ => Err("pread: unknown device"),
    };
    proc.fd_store(fd, chan);
    match result {
        Ok(read) => read,
        Err(msg) => err(proc, msg),
    }
}

/// NOTIFY(28): handler VA (a full-width pointer arg).  nil clears the
/// handler (libc atnotify's notify(0)); otherwise the VA must be 4-aligned
/// and land inside the proc's OWN text segment (T-04-17: a process can
/// never install a handler pointing outside its image — the EL0 handler
/// redirect trusts only validated segment-internal VAs).  Stored on the
/// proc; trap.rs's deliver_notes redirects into it at the return-to-EL0
/// boundary.
fn sys_notify(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "notify: bad argument block");
    };
    let handler = args[0];
    if handler == 0 {
        proc.notify_handler = None;
        return 0;
    }
    if handler & 3 != 0 {
        return err(proc, "notify: misaligned handler");
    }
    let in_text = proc
        .segments
        .as_ref()
        .is_some_and(|s| s.text.0.contains(&(handler as usize)));
    if !in_text {
        return err(proc, "notify: handler outside text segment");
    }
    proc.notify_handler = Some(handler);
    0
}

/// NOTED(29): action (int-width arg — width discipline).  NCONT(0)/NRSTR(3)
/// restore the saved Ureg (the 280 bytes deliver_notes pushed — the frozen
/// sizeof, NOT r9's 288 stride) over the live frame in place; the success
/// return takes dispatch's no-writeback special path (the donor dosyscall
/// NOTED case).  NDFLT(1) exits with the delivered note message.  NSAVE(2)
/// is loud-unimplemented (the smoke's notejmp path uses NCONT).
fn sys_noted(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "noted: bad argument block");
    };
    let action = args[0] & 0xffff_ffff; // noted's action is an int
    match action {
        trap::NCONT | trap::NRSTR => {
            // The saved ureg is kernel-tracked (the donor's up->noteureg):
            // libc's noted(action) passes NO pointer.
            let nureg = proc.note_ureg;
            if nureg == 0 {
                return err(proc, "noted: not notified");
            }
            if proc.validate_user_range(nureg, trap::UREG_SIZE as u64, false).is_err() {
                return err(proc, "noted: bad saved ureg");
            }
            // Validate the resume target BEFORE committing the restore
            // (the donor noted()'s okaddr(pc)+okaddr(sp)+alignment checks,
            // pre-copy so a bad saved frame leaves the live frame
            // untouched and the -1 writeback is truthful).
            let saved_sp = unsafe { ((nureg + 248) as *const u64).read() }; // ureg->sp
            let saved_pc = unsafe { ((nureg + 256) as *const u64).read() }; // ureg->pc
            let in_text = proc
                .segments
                .as_ref()
                .is_some_and(|s| s.text.0.contains(&(saved_pc as usize)));
            if saved_pc & 3 != 0 || saved_sp & 7 != 0 || !in_text {
                return err(proc, "noted: bad resume frame");
            }
            if proc.validate_user_range(saved_sp, 8, true).is_err() {
                return err(proc, "noted: bad resume stack");
            }
            // Restore: the saved 280 bytes over the live frame's 35 fields.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    nureg as *const u8,
                    frame as *mut TrapFrame as *mut u8,
                    trap::UREG_SIZE,
                );
            }
            // WR-04: the copied psr came from user-writable memory (the
            // notejmp contract lets the handler edit the saved Ureg) —
            // force the privilege bits to EL0t so a handler-edited M field
            // can never steer trap.S's eret to EL1 (a user-triggerable
            // kernel panic).  The saved `type` field stays
            // informational-only (aarch64/ABI.md documents it as such).
            frame.force_psr_el0t();
            // Allow the next note (the donor's up->notified = 0).
            proc.note_ureg = 0;
            0
        }
        trap::NDFLT => {
            // Exit with the delivered note's message (the donor pexit
            // shape).  Diverges.
            let msg = proc.last_note;
            let len = proc.last_note_len;
            exit_tail(proc, &msg[..len]);
        }
        trap::NSAVE => {
            println!("noted: NSAVE unimplemented");
            err(proc, "noted: NSAVE unimplemented")
        }
        _ => err(proc, "noted: bad action"),
    }
}

/// OPEN(14): path, mode.  05-04 (D-03, the one-way door): EVERY path
/// resolves through the namespace core — namec + the proc's mount table —
/// and the resolved chan opens via the per-device dispatch
/// (namec::open_chan).  The Phase-4 scaffolding that routed paths by
/// literal name matching is RETIRED (the plan's prohibition: it must not
/// be extended for new devices nor kept as a fallback).
/// Installed in the lowest free fd slot.
fn sys_open(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "open: bad argument block");
    };
    let (path_va, mode) = (args[0], (args[1] & 0xffff_ffff) as u32); // mode is an int (width discipline)
    let path = match copy_user_str(proc, path_va, ERRMAX) {
        Ok(path) => path,
        Err(msg) => return err(proc, msg),
    };

    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let guard = pg.lock(&node);
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "open: no namespace (pre-exec)");
    };
    let c = match namec::namec(&path, slash, dot, &guard, NamecMode::Open) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let opened = match namec::open_chan(&c, mode) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    drop(guard);

    match proc.fd_install_lowest(opened) {
        Some(slot) => slot,
        None => err(proc, "open: no free fd"),
    }
}

/// BIND(2): name, old, flag — 05-04 (D-03; measured ×22 in the 05-03
/// trace).  Both path strings cross the trust boundary: sargs +
/// validate_user_range BEFORE any kernel deref (T-04-11 discipline,
/// verbatim).  The source resolves with the union-head redirect (donor
/// namec's Abind domount); the target NEVER redirects (Amount — stacked
/// binds attach to the original directory) and must EXIST (Pitfall-8:
/// MCREATE permits later creation IN the mounted dir; it never conjures
/// the mount target).  The dir/file agreement + order-bit semantics are
/// cmount's, ported in pgrp::bind.
fn sys_bind(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "bind: bad argument block");
    };
    let (name_va, old_va, flag) = (args[0], args[1], (args[2] & 0xffff_ffff) as u32); // flag is an int
    // 9k bindmount: unknown bits and BEFORE|AFTER together are Ebadarg.
    if flag & !MMASK != 0 || (flag & MORDER) == (MBEFORE | MAFTER) {
        return err(proc, "bind: bad flags");
    }
    let name = match copy_user_str(proc, name_va, ERRMAX) {
        Ok(s) => s,
        Err(msg) => return err(proc, msg),
    };
    let old = match copy_user_str(proc, old_va, ERRMAX) {
        Ok(s) => s,
        Err(msg) => return err(proc, msg),
    };

    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let mut guard = pg.lock(&node);
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "bind: no namespace (pre-exec)");
    };
    let src = match namec::namec(&name, slash, dot, &guard, NamecMode::BindSource) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let tgt = match namec::namec(&old, slash, dot, &guard, NamecMode::MountTarget) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    match guard.bind(src, &tgt, flag) {
        Ok(_) => 0,
        Err(msg) => err(proc, msg),
    }
}

/// CHDIR(3): path — 05-04 (measured ×2: ps's /proc chdir, newns's cd).
/// 9k syschdir: namec with Atodir semantics (NO mount-point redirect —
/// "directories are left before the mount point, so one may mount on /
/// or . and see the effect"), the target must be a directory, then
/// up->dot = c.
fn sys_chdir(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "chdir: bad argument block");
    };
    let path = match copy_user_str(proc, args[0], ERRMAX) {
        Ok(s) => s,
        Err(msg) => return err(proc, msg),
    };
    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let guard = pg.lock(&node);
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "chdir: no namespace (pre-exec)");
    };
    let c = match namec::namec(&path, slash, dot, &guard, NamecMode::ToDir) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    if c.qid.typ & port::dat::QTDIR == 0 {
        return err(proc, "chdir: not a directory");
    }
    drop(guard);
    proc.dot = Some(c);
    0
}

/// UNMOUNT(35): name, old — 05-04 (measured ×1: boot's settime /tmp
/// handling).  9k sysunmount + cunmount: the mount point resolves WITHOUT
/// the redirect (Amount semantics); old == nil removes the whole mount
/// point, otherwise only the chain entry whose mounted chan matches old's
/// identity.  "Not a mount point" / "not in the union" are loud errors.
fn sys_unmount(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "unmount: bad argument block");
    };
    let (name_va, old_va) = (args[0], args[1]);
    let name = match copy_user_str(proc, name_va, ERRMAX) {
        Ok(s) => s,
        Err(msg) => return err(proc, msg),
    };

    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let mut guard = pg.lock(&node);
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "unmount: no namespace (pre-exec)");
    };
    let tgt = match namec::namec(&name, slash, dot, &guard, NamecMode::MountTarget) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let from = if old_va == 0 {
        None
    } else {
        let old = match copy_user_str(proc, old_va, ERRMAX) {
            Ok(s) => s,
            Err(msg) => return err(proc, msg),
        };
        match namec::namec(&old, slash, dot, &guard, NamecMode::BindSource) {
            Ok(c) => Some(c),
            Err(msg) => return err(proc, msg),
        }
    };
    match guard.unmount(&tgt.path, from.as_ref()) {
        Ok(()) => 0,
        Err(msg) => err(proc, msg),
    }
}

/// FVERSION(40): fd, msize, version, nversion — 05-07 (D-03's mount half;
/// the 05-06 gate's FVERSION wall).  Donor sysfversion (9k sysauth.c):
/// `int fversion(int fd, u32int msize, char *version, int nversion)` —
/// negotiate the 9P session over the fd's chan (devmnt::fversion, the
/// mntversion port) and write back the negotiated version string,
/// returning its length.  The fd must be a 9P-capable connection open
/// ORDWR (the connectembed pipe end).  T-04-11: the version buffer is
/// validated + forced resident BEFORE the kernel reads/writes it (the
/// 05-06 cold-buffer class); the donor's Ebadarg NUL check is ported
/// (the buffer carries the REQUESTED version string — "" means the
/// "9P2000" default; anything else is rejected, the donor's
/// "bad 9P version specification").
fn sys_fversion(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "fversion: bad argument block");
    };
    // fd, msize, nversion are int-width (the width discipline); version is
    // a full pointer.
    let (fd, msize, ver_va, nver) =
        (args[0] & 0xffff_ffff, (args[1] & 0xffff_ffff) as u32, args[2], args[3] & 0xffff_ffff);
    if proc.validate_user_range(ver_va, nver, true).is_err() {
        return err(proc, "fversion: bad version buffer");
    }
    fault::ensure_user_resident(proc, ver_va as usize, (ver_va + nver) as usize);
    if nver == 0 {
        return err(proc, "fversion: zero-length version buffer");
    }
    let buf = unsafe { user_slice(ver_va, nver) };
    let Some(nul) = buf.iter().position(|&b| b == 0) else {
        return err(proc, "fversion: bad version specification");
    };
    let requested = &buf[..nul];
    if !requested.is_empty() && requested != port::devmnt::VERSION9P {
        return err(proc, "fversion: bad 9P version specification");
    }
    let Some(mut chan) = proc.fd_take(fd) else {
        return err(proc, "fversion: bad fd");
    };
    let result = {
        let out = unsafe { user_slice_mut(ver_va, nver) };
        port::devmnt::fversion(&mut chan, msize, out)
    };
    proc.fd_store(fd, chan);
    match result {
        Ok(len) => len,
        Err(msg) => err(proc, msg),
    }
}

/// MOUNT(46): fd, afd, old, flag, aname — 05-07 (D-03's mount half;
/// nsinit's `mount(fd, -1, "/root", MREPL|MCREATE, rp)` and settime's
/// second mount onto /tmp via #s/boot).  Donor sysmount + bindmount's
/// ismount arm (9k sysfile.c): validate the flags (bindmount's Ebadarg
/// shape), the fd must be a 9P connection (auto-version when fversion
/// never ran — the donor mntattach's Enoversion fallback), Tattach over
/// the session with afid = NOFID (afd ≠ -1 is loud-unsupported — D-02's
/// no-auth era), then the mount-table insertion at the resolved target
/// with the frozen flag semantics (pgrp::bind — MountTarget resolution,
/// dir/file agreement, MREPL/MBEFORE/MAFTER order, MCREATE recorded).
fn sys_mount(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 5) else {
        return err(proc, "mount: bad argument block");
    };
    // fd, afd, flag are int-width (the width discipline); old/aname are
    // full pointers.
    let (fd, afd, old_va, flag, aname_va) = (
        args[0] & 0xffff_ffff,
        (args[1] & 0xffff_ffff) as u32 as i32,
        args[2],
        (args[3] & 0xffff_ffff) as u32,
        args[4],
    );
    // 9k bindmount: unknown bits and BEFORE|AFTER together are Ebadarg.
    if flag & !MMASK != 0 || (flag & MORDER) == (MBEFORE | MAFTER) {
        return err(proc, "mount: bad flags");
    }
    if afd != -1 {
        return err(proc, "mount: auth fd unsupported this era (D-02)");
    }
    let old = match copy_user_str(proc, old_va, ERRMAX) {
        Ok(s) => s,
        Err(msg) => return err(proc, msg),
    };
    let aname = match copy_user_str(proc, aname_va, ERRMAX) {
        Ok(s) => s,
        Err(msg) => return err(proc, msg),
    };

    // The fd's session — checked out only for the lookup (the RPC IO rides
    // the session's own chan reference; a minimal checkout window).
    let Some(mut chan) = proc.fd_take(fd) else {
        return err(proc, "mount: bad fd");
    };
    if chan.dev != DEV_PIPE || chan.mode & port::dat::OMASK != port::dat::ORDWR {
        proc.fd_store(fd, chan);
        return err(proc, "mount: fd is not a 9P connection");
    }
    if chan.mnt_session().is_none() {
        // The donor's mntattach auto-versions when fversion never ran.
        if let Err(msg) = port::devmnt::fversion(&mut chan, 0, &mut []) {
            proc.fd_store(fd, chan);
            return err(proc, msg);
        }
    }
    let Some(mnt) = chan.mnt_session() else {
        proc.fd_store(fd, chan);
        return err(proc, "mount: no 9P session after version");
    };
    proc.fd_store(fd, chan);

    // Tattach (blocks mid-syscall — the pipe read schedules the server
    // proc; NO pgrp lock is held across the RPC).
    let mut mnt_chan = match port::devmnt::attach(&mnt, &aname) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };

    // Resolve the target (MountTarget — never redirect; stacked mounts
    // attach to the original directory) and insert the entry.
    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let mut guard = pg.lock(&node);
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "mount: no namespace (pre-exec)");
    };
    let tgt = match namec::namec(&old, slash, dot, &guard, NamecMode::MountTarget) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    mnt_chan.path = tgt.path.clone();
    match guard.bind(mnt_chan, &tgt, flag) {
        Ok(_) => 0,
        Err(msg) => err(proc, msg),
    }
}

/// STAT(42): path, buf, n — 05-04 Task 3 (measured ×51 in the 05-03
/// trace).  namec with Aaccess semantics (the final component DOES cross
/// to the union head), then the device's stat op writes the stat(5) wire
/// bytes; the return is the byte count (the donor sysstat contract).
/// T-04-11: the path string AND the output buffer (writable) validate
/// before any kernel deref.  The donor's dirsetname fix-up is a no-op for
/// every device this era (each serves the canonical name already) and is
/// deliberately omitted.
fn sys_stat(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "stat: bad argument block");
    };
    let (path_va, buf_va, n) = (args[0], args[1], args[2] & 0xffff_ffff); // n is an int (width discipline)
    let path = match copy_user_str(proc, path_va, ERRMAX) {
        Ok(s) => s,
        Err(msg) => return err(proc, msg),
    };
    if proc.validate_user_range(buf_va, n, true).is_err() {
        return err(proc, "stat: bad user buffer");
    }
    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let guard = pg.lock(&node);
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "stat: no namespace (pre-exec)");
    };
    let c = match namec::namec(&path, slash, dot, &guard, NamecMode::Open) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    drop(guard);
    let sb = unsafe { user_slice_mut(buf_va, n) };
    match namec::stat_chan(&c, sb) {
        Ok(count) => count,
        Err(msg) => err(proc, msg),
    }
}

/// FSTAT(43): fd, buf, n — 05-04 Task 3 (measured ×3).  The open fd's
/// chan stats through the same device dispatch (the donor sysfstat shape
/// — no namec, no mount crossing: the chan already resolved at open).
/// devroot's stat reads the chan's paqfs entry, so DEV_ROOT stats go
/// through the LIVE fd slot; the other devices' stats read qid/dev only.
fn sys_fstat(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "fstat: bad argument block");
    };
    let (fd, buf_va, n) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff); // fd, n are ints
    if proc.validate_user_range(buf_va, n, true).is_err() {
        return err(proc, "fstat: bad user buffer");
    }
    let Some(chan) = proc.fd_take(fd) else {
        return err(proc, "fstat: bad fd");
    };
    let sb = unsafe { user_slice_mut(buf_va, n) };
    let result = namec::stat_chan(&chan, sb);
    proc.fd_store(fd, chan);
    match result {
        Ok(count) => count,
        Err(msg) => err(proc, msg),
    }
}

/// BRK_(24): addr — 05-04 Task 3 (measured ×28 in the 05-03 trace; the
/// plan's dispatch note binds it: malloc's pool growth rides it —
/// sbrkalloc's sbrk→brk_ — and boot's dirstat mallocs make it
/// gate-load-bearing).  Donor sysbrk_ (9k sysseg.c): ibrk(addr, BSEG) —
/// set the bss segment's top; libc's sbrk needs 0/-1.  Growth pages
/// demand-fault as zero pages (the Bss class covers the extended range);
/// a shrink only lowers the recorded break (mapped pages stay mapped —
/// the donor mfreeseg's them; malloc never shrinks this era).  The break
/// never crosses USTOP (the stack-growth bound — the donor's Esoverlap
/// check's segment).
fn sys_brk_(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "brk: bad argument block");
    };
    let addr = args[0] as usize; // a full-width pointer arg (no masking)
    let Some(segs) = &mut proc.segments else {
        return err(proc, "brk: no segments (pre-exec)");
    };
    if addr == 0 {
        return 0; // no-op (libc never passes 0; defensive)
    }
    if addr < segs.bss.0.start {
        return err(proc, "brk: address below the heap base");
    }
    if addr > exec::USTOP {
        return err(proc, "brk: out of memory");
    }
    segs.bss.0.end = addr;
    0
}

/// EXEC(7): path, argv — 2 syscall args (libc `exec(char *name,
/// char *argv[])`).  Donor semantics: returns ONLY on failure (-1/errstr,
/// old image intact); on success exec::exec_replace rewrites the live trap
/// frame (R0 = tos, SP → new seed stack, PC = new entry, PSR = 0) and the
/// dispatch return erets into the new image.
///
/// Both args are user memory crossing the trust boundary (T-04-11): the
/// path string and the argv block + every pointed-to string are validated
/// against the proc's segment map before copying, with argc bounded at 32
/// and each arg at 128 bytes.  Everything is COPIED to kernel memory before
/// exec_replace runs — the replacement frees the old image's pages, which
/// is where the arguments lived.
fn sys_exec(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    const MAX_EXEC_ARGS: usize = 32;
    const MAX_EXEC_ARG_LEN: usize = 128;
    /// The whole-file staging ceiling for a MOUNTED binary (05-07: read
    /// eagerly at exec because the trap-context pager never RPCs) — far
    /// past every subset binary (rc is ~250 KB), bounded so an untrusted
    /// server can never stream the kernel into an unbounded allocation.
    const EXEC_STAGE_CAP: usize = 16 * 1024 * 1024;
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "exec: bad argument block");
    };
    let (path_va, argv_va) = (args[0], args[1]);
    let path = match copy_user_str(proc, path_va, ERRMAX) {
        Ok(path) => path,
        Err(msg) => return err(proc, msg),
    };
    let mut argv: Vec<Vec<u8>> = Vec::new();
    if argv_va != 0 {
        loop {
            if argv.len() >= MAX_EXEC_ARGS {
                return err(proc, "exec: too many args");
            }
            let slot_va = argv_va + 8 * argv.len() as u64;
            if proc.validate_user_range(slot_va, 8, false).is_err() {
                return err(proc, "exec: bad argv block");
            }
            let ptr = unsafe { (slot_va as *const u64).read() };
            if ptr == 0 {
                break;
            }
            match copy_user_str(proc, ptr, MAX_EXEC_ARG_LEN) {
                Ok(s) => argv.push(s),
                Err(msg) => return err(proc, msg),
            }
        }
    }

    // 05-07 Task 2 (the wiring walk): resolve through the NAMESPACE —
    // namec + the proc's mount table — so an exec path crosses mounts
    // (execinit's /aarch64/init through the union-mounted /; init's
    // /bin/rc through the post-newns /bin union into the mounted tree).
    // The Phase-4 devroot-only prefix walk is retired (D-03's one-way
    // door: resolution lives in the namespace core).
    let pg = proc.pgrp.clone();
    let c = {
        let node = LockNode::new();
        let guard = pg.lock(&node);
        let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
            return err(proc, "exec: no namespace (pre-exec)");
        };
        match namec::namec(&path, slash, dot, &guard, NamecMode::Open) {
            Ok(c) => c,
            Err(msg) => return err(proc, msg),
        }
    };
    // open + stage OUTSIDE the pgrp lock: a devmnt open/read RPCs and
    // BLOCKS mid-syscall (the pipe read schedules the server proc).
    let opened = match namec::open_chan(&c, OEXEC) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let backing = match opened.dev {
        DEV_ROOT => {
            let Some(root) = port::devroot::root() else {
                return err(proc, "devroot: not mounted");
            };
            match root.entry(&opened) {
                Ok(bin) => ByteBacking::Paq { dir: bin, image: root.image() },
                Err(msg) => return err(proc, msg),
            }
        }
        DEV_MNT => {
            // A mounted binary is read WHOLE here (devmnt::read_all) —
            // syscall context, where the 9P client's blocking is by
            // design; the trap-context pager must never RPC, so the
            // staged bytes become the demand-page byte source.
            match port::devmnt::read_all(&opened, EXEC_STAGE_CAP) {
                Ok(bytes) => ByteBacking::Mnt { file: alloc::sync::Arc::new(bytes) },
                Err(msg) => return err(proc, msg),
            }
        }
        _ => return err(proc, "exec: path is not an executable source"),
    };
    // The exec chan is a transient (never fd-installed): close-dispatch it
    // (devmnt Tclunks the fid — the staged bytes now own the content).
    port::namec::close_chan(&opened);
    let leaf = path.split(|&b| b == b'/').filter(|c| !c.is_empty()).last().unwrap_or(&path[..]);
    let name = core::str::from_utf8(leaf).unwrap_or("?");
    let arg_slices: Vec<&[u8]> = argv.iter().map(|a| a.as_slice()).collect();
    match exec::exec_replace(proc, backing, name, &arg_slices, frame) {
        // dispatch writes frame.x0 = tos_va — the same value exec_replace
        // seeded into the frame's x0, so the writeback agrees by
        // construction and the eret enters the new image with R0 = tos.
        Ok(tos_va) => tos_va,
        Err(msg) => err(proc, msg),
    }
}

/// CLOSE(4): drop the fd slot.
fn sys_close(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "close: bad argument block");
    };
    let fd = args[0] & 0xffff_ffff; // fd is an int (width discipline)
    match proc.fd_take(fd) {
        Some(c) => {
            // 05-06: the close dispatch (the donor's devclose) — a pipe
            // end's close decrements the open count and the last one
            // hangs the peer's read queue up (the EOF path).
            port::namec::close_chan(&c);
            0
        }
        None => err(proc, "close: bad fd"),
    }
}

/// PIPE(21): fd-array-ptr — 05-06 (D-04's connectembed substrate;
/// measured ×1 in the 05-03 trace — the ONE connectembed pipe).  Donor
/// syspipe (9k sysfile.c): attach a FRESH pipe ("#|" — one Pipe per
/// attach, never shared across attaches), walk data/data1, open both
/// ORDWR, install the pair at the two lowest free slots (found BEFORE
/// anything installs — a one-slot table fails with a named errstr and
/// consumes nothing), and write the two fds back through the validated
/// user array (two 32-bit ints — the width discipline).
///
/// The ends reference the ONE Arc-shared Pipe (Pitfall 4): writes on end
/// i land in the queue end 1-i reads; fork/dup clone the Arc, never the
/// bytes.
fn sys_pipe(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "pipe: bad argument block");
    };
    let arr_va = args[0];
    if proc.validate_user_range(arr_va, 8, true).is_err() {
        return err(proc, "pipe: bad fd array");
    }
    fault::ensure_user_resident(proc, arr_va as usize, (arr_va + 8) as usize); // 05-06 cold-buffer class
    // The pipe pair (the registry '|' twin — a fresh attach per call).
    let dir = port::devpipe::attach();
    let c0 = match port::devpipe::walk(&dir, b"data") {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let c1 = match port::devpipe::walk(&dir, b"data1") {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let Some(pair) = proc.fd_free_pair() else {
        return err(proc, "pipe: no free fd pair");
    };
    let e0 = match namec::open_chan(&c0, port::dat::ORDWR) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let e1 = match namec::open_chan(&c1, port::dat::ORDWR) {
        Ok(c) => c,
        Err(msg) => {
            // Balance e0's open before failing (the close dispatch).
            port::namec::close_chan(&e0);
            return err(proc, msg);
        }
    };
    proc.fd_install_pair(pair, e0, e1);
    let out = unsafe { user_slice_mut(arr_va, 8) };
    out[0..4].copy_from_slice(&(pair.0 as u32).to_le_bytes());
    out[4..8].copy_from_slice(&(pair.1 as u32).to_le_bytes());
    0
}

/// DUP(5): oldfd, newfd — 05-06 (measured ×9 in the 05-03 trace;
/// connectembed's child dups the pipe ends onto 0/1).  Donor sysdup
/// (9k sysfile.c): the old fd's chan is ref-cloned (an open pipe end's
/// count bumps — the Arc is shared, never the bytes); newfd == -1
/// installs at the lowest free slot; an occupied newfd is EVICTED and
/// close-dispatched AFTER the install ("close after dup'ing, so
/// date > #d/1 works").  Returns newfd.
fn sys_dup(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "dup: bad argument block");
    };
    let (old, new) = (args[0] & 0xffff_ffff, (args[1] & 0xffff_ffff) as u32 as i32); // both ints
    let Some(c) = proc.fd_ref_clone(old) else {
        return err(proc, "dup: bad old fd");
    };
    if new == -1 {
        return match proc.fd_install_lowest(c) {
            Some(slot) => slot,
            None => err(proc, "dup: no free fd"),
        };
    }
    if new < 0 {
        return err(proc, "dup: bad new fd");
    }
    match proc.fd_install_at(new as u64, c) {
        Some(evicted) => {
            if let Some(old_chan) = evicted {
                port::namec::close_chan(&old_chan);
            }
            new as u64
        }
        None => err(proc, "dup: bad new fd"),
    }
}

/// CREATE(22): path, mode, perm — 05-06 Task 2 (measured ×67 in the
/// 05-03 trace: every env-var set + boot's srvcreate).  Donor syscreate +
/// chan.c's Acreate: resolve the PARENT (open-style union redirect), then
/// if the final component EXISTS open it OTRUNC (unless OEXCL → Eexist),
/// else dispatch the device's create op (devenv/devsrv are this phase's
/// creators; a device without a create op errors loudly).  The donor's
/// create-then-fail retry (the create/create race) is unreachable this
/// era: syscalls are serialized.  Directory creates (perm&DMDIR) have no
/// measured caller — loud error, never a silent fake.
fn sys_create(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "create: bad argument block");
    };
    let (path_va, mode, perm) = (args[0], (args[1] & 0xffff_ffff) as u32, (args[2] & 0xffff_ffff) as u32); // mode/perm are ints
    if perm & 0x8000_0000 != 0 {
        // DMDIR
        return err(proc, "create: directories unsupported this era");
    }
    let path = match copy_user_str(proc, path_va, ERRMAX) {
        Ok(path) => path,
        Err(msg) => return err(proc, msg),
    };
    // Split parent/final at the last '/': "#s/boot" → ("#s", "boot");
    // "/env/x" → ("/env", "x"); "/x" → ("/", "x"); bare "x" → the dot
    // (no measured bare caller this era; the dot redirect below keeps it
    // correct).
    let split = path.iter().rposition(|&b| b == b'/');
    let final_name: &[u8] = match split {
        Some(i) => &path[i + 1..],
        None => &path[..],
    };
    if final_name.is_empty() {
        return err(proc, "create: empty final component");
    }

    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let guard = pg.lock(&node);
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "create: no namespace (pre-exec)");
    };
    let parent = match split {
        Some(0) => namec::namec(b"/", slash, dot, &guard, NamecMode::Open),
        Some(i) => namec::namec(&path[..i], slash, dot, &guard, NamecMode::Open),
        None => {
            // The dot itself, with the open-mode final redirect (a dot
            // covered by a mount point creates in the union head).
            let mut d = dot.dup();
            if let Some(chain) = guard.chain_at(&d.path) {
                let Some(head) = chain.first() else {
                    return err(proc, "create: empty union chain");
                };
                let mut nc = head.to.dup();
                nc.path = d.path.clone();
                d = nc;
            }
            Ok(d)
        }
    };
    let parent = match parent {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    if parent.qid.typ & port::dat::QTDIR == 0 {
        return err(proc, "create: parent not a directory");
    }
    let opened = match namec::walk_one(&parent, final_name) {
        Ok(existing) => {
            // The donor's Acreate existing-file arm: OEXCL → Eexist;
            // otherwise open OTRUNC (the create-on-existing fallback).
            if mode & port::dat::OEXCL != 0 {
                return err(proc, "create: file exists");
            }
            match namec::open_chan(&existing, mode | OTRUNC) {
                Ok(c) => c,
                Err(msg) => return err(proc, msg),
            }
        }
        Err(_) => {
            match namec::create_chan(&parent, final_name, mode & !(port::dat::OEXCL | port::dat::OCEXEC), perm) {
                Ok(c) => c,
                Err(msg) => return err(proc, msg),
            }
        }
    };
    drop(guard);
    match proc.fd_install_lowest(opened) {
        Some(slot) => slot,
        None => err(proc, "create: no free fd"),
    }
}

/// FD2PATH(23): fd, buf, n — 05-06 Task 2 (measured ×3 in the 05-03
/// trace).  Donor sysfd2path: the fd's chan's walked path (chanpath)
/// into the validated user buffer, NUL-terminated and truncated to n
/// (Harvey libc: fd2path(int, char*, int) — n is int-width, masked).
/// Returns 0.  The path bookkeeping is the walk-maintained one
/// (fd2path's raison d'être) — never the errstr side channel.
fn sys_fd2path(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "fd2path: bad argument block");
    };
    let (fd, buf_va, n) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff); // fd, n are ints
    if proc.validate_user_range(buf_va, n, true).is_err() {
        return err(proc, "fd2path: bad user buffer");
    }
    fault::ensure_user_resident(proc, buf_va as usize, (buf_va + n) as usize); // 05-06 cold-buffer class
    let Some(path) = proc.fd_path(fd) else {
        return err(proc, "fd2path: bad fd");
    };
    let dst = unsafe { user_slice_mut(buf_va, n) };
    if dst.is_empty() {
        return 0;
    }
    let copy = path.len().min(dst.len() - 1);
    dst[..copy].copy_from_slice(&path[..copy]);
    dst[copy] = 0;
    0
}

/// RFORK(19): flag.  Donor sysrfork flag validation (9front@350266104f
/// sys/src/9/port/sysproc.c): (RFFDG|RFCFDG), (RFNAMEG|RFCNAMEG) and
/// (RFENVG|RFCENVG) are each mutually exclusive; RFMEM/RFNOWAIT require
/// RFPROC.  Supported this era: the address space is ALWAYS eagerly copied
/// page-for-page (no COW, no shared writable pages — T-04-15; the donor's
/// RFMEM share-semantics is not honored, which the smoke does not depend
/// on); RFNOWAIT is honored (EXITS posts no wait record); RFREND is a
/// no-op (no rendezvous this era); RFNOTEG has REAL note-group semantics
/// since 05-08 (D-06: Proc.noteid — a fresh group per construction,
/// inherited without RFNOTEG, fresh with it; the console DEL posts
/// "interrupt" to the readers' groups).  05-04 (D-03) adds
/// the namespace group semantics per fork(2): RFPROC+neither shares the
/// parent's pgrp, RFNAMEG deep-copies, RFCNAMEG starts clean — and the
/// non-RFPROC split (boot(8)'s `rfork(RFNAMEG)`): the CALLER's group
/// becomes a fresh copy (RFNAMEG) or fresh empty (RFCNAMEG).  05-05
/// (D-07) adds the ENV group semantics, the same donor shape (fork(2):
/// "RFENVG: the environment variables are COPIED; otherwise the two
/// processes SHARE"; "RFCENVG: the new process starts with an EMPTY
/// environment"): RFPROC+neither shares the parent's env group, RFENVG
/// deep-copies (envcpy), RFCENVG empties; the non-RFPROC env split
/// (newns's `rfork(RFENVG|RFCNAMEG)`) re-groups the CALLER (copy on
/// RFENVG, empty on RFCENVG).  05-06 (D-03's fd half) adds the FD group
/// semantics, again per fork(2)/sysrfork ("RFFDG: the invoker's file
/// descriptor table is COPIED; otherwise the two processes SHARE a
/// single table"): RFPROC+neither SHARES the Arc<Lock<FdGroup>> handle,
/// RFFDG deep-COPIES slot-by-slot (dupfgrp — pipe chans clone their
/// Arc), RFCFDG empties (dupfgrp(nil)).  libc fork() =
/// rfork(RFPROC|RFFDG|RFREND) = COPY — connectembed's child dup/close
/// choreography depends on it.  (The 05-06 plan's bullet inverts RFFDG
/// vs default — the phase's third flag inversion; the donor governs.)
/// The non-RFPROC fd-group split (RFFDG/RFCFDG) has no measured caller
/// and stays a loud -1 — never silent partial semantics.  Every other
/// bit combination is a loud -1 errstr.
fn sys_rfork(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "rfork: bad argument block");
    };
    let flag = args[0] & 0xffff_ffff; // rfork's flag is an int (width discipline)
    if flag & (RFFDG | RFCFDG) == (RFFDG | RFCFDG)
        || flag & (RFNAMEG | RFCNAMEG) == (RFNAMEG | RFCNAMEG)
        || flag & (RFENVG | RFCENVG) == (RFENVG | RFCENVG)
    {
        return err(proc, "rfork: bad flags");
    }
    const SUPPORTED: u64 = RFPROC
        | RFMEM
        | RFFDG
        | RFNOWAIT
        | RFREND
        | RFNOTEG
        | RFNAMEG
        | RFCNAMEG
        | RFENVG
        | RFCENVG
        | RFCFDG;
    if flag & !SUPPORTED != 0 {
        return err(proc, "rfork: unsupported flags");
    }
    if flag & RFPROC == 0 {
        // Donor non-RFPROC branch: RFMEM/RFNOWAIT are Ebadarg; the fd-group
        // split is unsupported this era (loud); the namespace group split
        // (RFNAMEG copy / RFCNAMEG wipe) re-groups the CALLER, as does
        // the env group split (05-05: RFENVG copy / RFCENVG empty —
        // newns's rfork(RFENVG|RFCNAMEG)).
        if flag & (RFMEM | RFNOWAIT | RFFDG | RFCFDG) != 0 {
            return err(proc, "rfork: bad flags for non-RFPROC");
        }
        if flag & (RFNAMEG | RFCNAMEG) != 0 {
            proc::rfork_namespace_split(proc, flag & RFNAMEG != 0);
        }
        if flag & (RFENVG | RFCENVG) != 0 {
            proc::rfork_env_split(proc, flag & RFENVG != 0);
        }
        // 05-08 (D-06): non-RFPROC rfork(RFNOTEG) re-groups the CALLER
        // into a fresh note group (9k sysproc.c:82) — init's fexec child
        // does this before exec'ing rc, so console DELs target the
        // shell's group and never init.
        if flag & RFNOTEG != 0 {
            proc::rfork_noteg_split(proc);
        }
        return 0;
    }
    match proc::rfork_clone(proc, frame, flag) {
        // The parent's wire result is the child pid; the child's copied
        // frame carries x0 = 0 (donor forkchild).
        Ok(pid) => pid,
        Err(msg) => err(proc, msg),
    }
}

/// SEEK(39): rv, fd, offset, base — 05-06 Task 3 (the gate's
/// trace-settled gap: paqfs's libbio Bseek — measured ×72 in the 05-03
/// trace; the loud wall named it), CORRECTED 05-07 (the hinge gate's
/// iterate-discipline): **the aarch64 syscall ABI is FOUR arguments —
/// `_seek(vlong *rv, int fd, vlong offset, int base)`** (the generated
/// stub's shape, sys/src/libc/9syscall/mkfile's `j=_seek` arm + the
/// aarch64 `_seek.c` wrapper; the donor sysfile.c sysseek's own comment:
/// "The system call actually has 4 arguments... the first argument is
/// where the offset is returned").  The result offset is WRITTEN through
/// the validated rv pointer; the wire result is 0 (error: -1 + errstr,
/// rv untouched — the wrapper then yields -1LL).  Donor sseek semantics:
/// base 0 absolute, base 1 relative (negative deltas allowed, never below
/// 0), directories error (Eisdir), SEEK_END loud-unsupported.
///
/// (The 05-06 version read fd/offset/base from slots 0/1/2 and returned
/// the offset on the wire — wrong for this ABI; its gate never caught it
/// because every exercised caller's seek was bio's in-buffer fast path.
/// The hinge gate's paqfs blockRead seeks for real and checks.)
fn sys_seek(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "seek: bad argument block");
    };
    // rv is a full pointer; fd/base are ints (the width discipline);
    // offset is a vlong.
    let (rv, fd, off, base) =
        (args[0], args[1] & 0xffff_ffff, args[2] as i64, (args[3] & 0xffff_ffff) as u32 as i32);
    if proc.validate_user_range(rv, 8, true).is_err() {
        return err(proc, "seek: bad result pointer");
    }
    fault::ensure_user_resident(proc, rv as usize, (rv + 8) as usize); // 05-06 cold-buffer class
    let Some(mut chan) = proc.fd_take(fd) else {
        return err(proc, "seek: bad fd");
    };
    let result = (|chan: &mut Chan| {
        if chan.qid.typ & port::dat::QTDIR != 0 {
            return Err("seek: is a directory");
        }
        let new = match base {
            0 => off,
            1 => match (chan.offset as i64).checked_add(off) {
                Some(v) => v,
                None => return Err("seek: offset overflow"),
            },
            _ => return Err("seek: SEEK_END unsupported this era"),
        };
        if new < 0 {
            return Err("seek: negative offset");
        }
        chan.offset = new as u64;
        Ok(new as u64)
    })(&mut chan);
    proc.fd_store(fd, chan);
    match result {
        Ok(v) => {
            // The donor's *rv write-back (the wrapper's real return).
            unsafe { (rv as *mut u64).write(v) };
            0
        }
        Err(msg) => err(proc, msg),
    }
}

/// ERRSTR(41): buf, n — copy the proc's errstr (NUL-terminated) into the
/// user buffer, return 0.  libc's `%r` verb reads the errstr this way
/// (rerrstr calls it twice) — the smoke's failure prints are its measured
/// caller (04-07: observed in the first virtmulti run; D-14's
/// measured-caller rule is satisfied by the smoke, not guessed).
fn sys_errstr(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "errstr: bad argument block");
    };
    let (buf, n) = (args[0], args[1] & 0xffff_ffff); // n is a uint (width discipline)
    if proc.validate_user_range(buf, n, true).is_err() {
        return err(proc, "errstr: bad user buffer");
    }
    fault::ensure_user_resident(proc, buf as usize, (buf + n) as usize); // 05-06 cold-buffer class
    let cap = (n as usize).min(ERRMAX);
    if cap == 0 {
        return 0;
    }
    let src = proc.errstr;
    let len = src.iter().position(|&b| b == 0).unwrap_or(ERRMAX).min(cap - 1);
    let dst = unsafe { user_slice_mut(buf, n) };
    dst[..len].copy_from_slice(&src[..len]);
    dst[len] = 0;
    0
}

/// SLEEP(17): millisecs — 05-08 (trace-gated: sleep ×42 in the 05-03
/// real-chain trace — init's for(;;) restart loop sleeps 1s between rc
/// generations).  Donor syssleep (9k sysproc.c): ms<=0 yields (give up
/// the CPU if another process is waiting); ms>0 tsleeps until the tick
/// count passes now+ms (1 tick = 1 ms this era — timer.rs
/// TICKS_PER_SEC=1000, so ms IS the tick delta).  The block rides the
/// 04-07 sched machinery; the tick's tick_timeouts readies expired
/// sleepers.  Register-deadline-then-block cannot lose the wake: syscall
/// context is IRQ-masked, so the tick can only fire after sched()
/// swtch'es (devpipe's documented invariant).  The ms arg is a `long` —
/// 32-bit in Plan 9 C (the width discipline).  Era limit (documented):
/// the donor's tsleep is interruptible by notes; this sleeper wakes only
/// on expiry (a posted note queues and delivers at the syscall return —
/// no measured caller interrupts a sleep this era).
///
/// Yield note: sleep(0)'s preempt() runs the tick's own entry from
/// syscall context — safe here because dispatch completes on the OLD
/// proc's kstack before the trap tail restores the NEW proc's frame, and
/// the post-dispatch note boundary still targets the frame that owns the
/// note (the OLD proc's — delivery before its next EL0 entry is the
/// correct ordering).
fn sys_sleep(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "sleep: bad argument block");
    };
    let ms = args[0] & 0xffff_ffff; // long is 32-bit (the width discipline)
    #[cfg(dev_exec)]
    {
        if ms == 0 {
            proc::preempt(); // the donor's yield()
            return 0;
        }
        proc.sleep_until = crate::platform::timer::ticks() + ms;
        proc::sched();
        0
    }
    #[cfg(not(dev_exec))]
    {
        let _ = ms;
        err(proc, "sleep: no timer in this profile")
    }
}

/// ALARM(6): millisecs — 05-08 (trace-gated: alarm ×1 — init's alarm(0)
/// disarm at main start).  Donor sysalarm (9k sysproc.c) + procalarm
/// (9k alarm.c): arm a per-proc alarm whose expiry posts the "alarm" note
/// to the proc (tick_timeouts's expiry arm; delivery rides the 04-08
/// return-to-EL0 boundary); successive calls reset the clock; a zero arg
/// clears it.  The return is the previously-REMAINING ms (0 = none
/// armed).  The ms arg is an `unsigned long` — 32-bit (the width
/// discipline).
fn sys_alarm(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "alarm: bad argument block");
    };
    let ms = args[0] & 0xffff_ffff; // unsigned long is 32-bit (the width discipline)
    #[cfg(dev_exec)]
    {
        let now = crate::platform::timer::ticks();
        let old = if proc.alarm_at != 0 { proc.alarm_at.saturating_sub(now) } else { 0 };
        proc.alarm_at = if ms == 0 { 0 } else { now + ms };
        old
    }
    #[cfg(not(dev_exec))]
    {
        let _ = ms;
        err(proc, "alarm: no timer in this profile")
    }
}

/// AWAIT(47): buf, buflen.  Donor sysawait shape: block (the 04-07
/// swtch-based sched path) until a child's exit record exists, then write
/// the Plan-9-shaped wait message `{pid} 0 0 0 "{msg}"` (utime/stime/
/// realtime zeros this era — CONTEXT.md discretion) NUL-terminated into
/// the validated user buffer and return the record length.  No children
/// ever → loud -1 errstr (donor pwait's "no children").
fn sys_await(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "await: bad argument block");
    };
    let (buf, buflen) = (args[0], args[1] & 0xffff_ffff); // buflen is a uint (width discipline)
    if proc.validate_user_range(buf, buflen, true).is_err() {
        return err(proc, "await: bad user buffer");
    }
    fault::ensure_user_resident(proc, buf as usize, (buf + buflen) as usize); // 05-06 cold-buffer class
    let cap = buflen as usize;
    if cap == 0 {
        return err(proc, "await: zero-length buffer");
    }
    loop {
        if let Some(rec) = proc.wait_records.pop_front() {
            proc.children_live = proc.children_live.saturating_sub(1);
            let msg = core::str::from_utf8(&rec.msg[..rec.msg_len]).unwrap_or("?");
            let text = alloc::format!("{} 0 0 0 \"{}\"", rec.pid, msg);
            let bytes = text.as_bytes();
            let n = bytes.len().min(cap - 1);
            let dst = unsafe { user_slice_mut(buf, buflen) };
            dst[..n].copy_from_slice(&bytes[..n]);
            dst[n] = 0;
            return n as u64;
        }
        if proc.children_live == 0 {
            return err(proc, "await: no children");
        }
        // Block mid-syscall (04-07): mark Blocked + swtch to the scheduler
        // context; resumes here when a child's EXITS posts a record and
        // wakes us.  The trap frame on this proc's own kstack survives
        // other processes' traps meanwhile (T-04-14's whole point).
        proc::sched();
    }
}

/// EXITS(8): copy the optional message (nil = empty), free the proc's user
/// pages + root-table page + kstack via the mapping_log / recorded PAs,
/// mark Moribund, print the exit line, post the wait record to the parent
/// and wake an AWAITing parent (04-07), then hand off to the scheduler;
/// when no non-Moribund proc remains, print the all-exited marker and park
/// in the clean wfi halt.  Diverges: never returns to the exited image.
fn sys_exits(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let msg_va = match sargs(frame, proc, 1) {
        Some(args) => args[0],
        None => 0, // exits never fails — a bad arg block exits msgless
    };
    // Copy the message BEFORE freeing anything (it lives in user memory).
    // exits(nil) carries a zero pointer; an invalid string is not an error
    // either — exits must never be wedged by its own diagnostics.
    let msg = if msg_va == 0 {
        Vec::new()
    } else {
        copy_user_str(proc, msg_va, ERRMAX).unwrap_or_default()
    };
    exit_tail(proc, &msg);
}

/// The shared exit tail (04-08): sys_exits's user-message path and the
/// note machinery's kernel-message kills (deliver_notes's no-handler kill,
/// NOTED's NDFLT) converge here.  Frees the user mappings, marks Moribund,
/// prints the exit line, posts + wakes the parent, frees the root-table
/// page and the kstack LAST (the handler is still running on it — safe
/// because the swtch save writes only the PROC_TABLE-resident context),
/// then the all-exited halt or the sched-loop hand-off.  Diverges.
pub(crate) fn exit_tail(proc: &mut Proc, msg: &[u8]) -> ! {
    proc.free_user_mappings();
    proc.state = ProcState::Moribund;
    let pid = proc.pid;
    println!("process {pid} exited: \"{}\"", core::str::from_utf8(msg).unwrap_or("<bad utf8>"));
    // 04-07: post the wait record to the parent and wake it BEFORE the
    // final frees (the record is kernel memory in the parent's table slot;
    // RFNOWAIT children post nothing, donor-style).
    if !proc.nowait {
        proc::post_wait_record(proc.parent, pid, msg);
    }
    // Last frees: the root-table page and the kstack pages.  The handler is
    // STILL running on the kstack and TTBR0 still names the root table —
    // safe because nothing allocates until the hand-off below (single CPU,
    // IRQ-masked syscall context) and the swtch save writes only the
    // PROC_TABLE-resident context struct, never the freed stack.
    proc.free_root_table_and_kstack();

    if !proc::any_active_procs() {
        println!("all user processes exited");
        // Clean halt (SC#5 shape): park in wfi.  IRQs stay masked (trap
        // context never unmasks them), so this is a wedge-free stop.
        loop {
            unsafe { core::arch::asm!("wfi") };
        }
    }
    // 04-07's reschedule: the sched loop picks the next Ready proc (the
    // woken parent, or a sibling).  Diverges.
    proc::sched_from_moribund();
}
