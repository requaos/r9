//! SVC dispatch + the syscall handler set.  The wire ABI is frozen
//! (Harvey `aarch64/ABI.md`): the syscall number arrives in R0
//! (`frame.x0`), arguments sit at user `SP+8` in 8-byte slots (`SP+0` is
//! the reserved LR slot the stub's `MOV R0, 0(FP)` materializes), the
//! result is written back to R0 before the shared trap-return tail erets
//! into EL0.
//!
//! Invariants: the implemented set is exactly the measured boot-chain
//! set — every other number is loud-unimplemented, never wedged.  EVERY
//! user-VA argument (the Sargs block itself, buffers, path strings, argv
//! arrays) validates against the calling proc's segment map via
//! `Proc::validate_user_range` BEFORE the kernel reads or writes it;
//! invalid → proc.errstr + -1, never a kernel dereference of an
//! unchecked pointer.  Locking: the devcons write path takes the Console
//! MCS lock from syscall context — safe because IRQ-context code never
//! contends (the timer tick's println is cfg(not(dev_exec))-gated) and
//! syscall/trap context runs IRQ-masked; the devcons INPUT queue lock is
//! syscall/trap context ONLY (the PL011 RX IRQ handler pushes its
//! lock-free ring; the ring→queue drain runs in the reader's own syscall
//! context via timer.rs → proc::cons_input_wake).

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

use port::dat::{
    Chan, DEV_CONS, DEV_DUP, DEV_ENV, DEV_ETHER, DEV_IP, DEV_MNT, DEV_PIPE, DEV_PROC, DEV_ROOT,
    DEV_SRV, OEXEC, OTRUNC,
};
use port::fdgrp::FdGroup;
use port::mcslock::{Lock, LockNode};
use port::namec::{self, NamecMode};
use port::pgrp::{MAFTER, MBEFORE, MMASK, MORDER};
use port::println;

use crate::exec;
use crate::proc::{self, ByteBacking, ERRMAX, Proc, ProcState, MAX_FDS};
use crate::trap;
use crate::trap::TrapFrame;
use crate::usermem::{UserPtr, UserSlice, UserSliceMut, UserString};

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

// rfork flags (Harvey sys/include/libc.h — frozen values).
// pub(crate): proc.rs's rfork_clone reads RFNAMEG/RFCNAMEG
// (the namespace group semantics) — one mirror, one provenance.
// RFENVG/RFCENVG are likewise pub(crate) (the env groups);
// RFFDG/RFCFDG (the fd-group arms).
pub(crate) const RFNAMEG: u64 = 1 << 0;
pub(crate) const RFENVG: u64 = 1 << 1;
pub(crate) const RFFDG: u64 = 1 << 2;
/// pub(crate): proc.rs's rfork_clone/rfork_noteg_split read it
/// (the note groups — the console DEL target scoping).
pub(crate) const RFNOTEG: u64 = 1 << 3;
const RFPROC: u64 = 1 << 4;
/// pub(crate): proc.rs's rfork_copy reads it (the RFMEM share arm —
/// the Data/Bss page sharing per the donor dupseg shape).
pub(crate) const RFMEM: u64 = 1 << 5;
pub(crate) const RFNOWAIT: u64 = 1 << 6;
pub(crate) const RFCNAMEG: u64 = 1 << 10;
pub(crate) const RFCENVG: u64 = 1 << 11;
pub(crate) const RFCFDG: u64 = 1 << 12;
/// pub(crate): proc.rs's rfork_clone reads it (the rendezvous-group
/// arm — the rgrp share/split semantics).
pub(crate) const RFREND: u64 = 1 << 13;

/// Handler shape: the live trap
/// frame (arg source + result sink) and the calling proc.
type Handler = fn(&mut TrapFrame, &mut Proc) -> u64;

/// The dispatch table — Harvey sys.h 0–54 verbatim as Option<fn> entries.
/// None → loud unimplemented at dispatch time.  The implemented set is
/// the measured boot-chain donor trace {pwrite, open, pread, close,
/// exits} plus the named core members EXEC(7), RFORK(19), AWAIT(47),
/// NOTIFY(28)/NOTED(29) and the trace-measured namespace members
/// BIND(2), CHDIR(3), UNMOUNT(35), STAT(42)/FSTAT(43), BRK_(24), the
/// connectembed family DUP(5)/PIPE(21)/CREATE(22)/FD2PATH(23), SEEK(39),
/// the mount half FVERSION(40)/MOUNT(46), and the timer pair ALARM(6) +
/// SLEEP(17) riding the generic timer (timer.rs ticks; expiry via the
/// tick's tick_timeouts).  The trace carries no brk_/segbrk(12) on the
/// hello path; segbrk stays None.  FAUTH(10) stays None (the designed
/// no-auth fall-through — nsinit/newns mount unauthenticated on its -1);
/// REMOVE(25), WSTAT(44), FWSTAT(45) and OSEEK(16) stay loud-None: the
/// 06-07 tier-1 trace DID measure remove/wstat/fwstat — but only inside
/// the sweep's donor-venue guard (hrm/hmk/hyacc/hed/hchmod/hmv/htouch),
/// the write-flow set r9 cannot serve this era (no writable fs), so the
/// measured evidence RATIFIED the existing loud-None rather than growing
/// it.  The 06-07 tier-1 measure-first trace (Harvey phase 06 evidence,
/// 8581 entry fragments over 201 pids) grew exactly two names, both with
/// r9-runnable callers: NSEC(53) (Harvey libc time() = nsec()/1e9 —
/// hdate/hipconfig) and RENDEZVOUS(34) (hfactotum's libthread start).
/// 08-04 grew the semaphore pair SEMACQUIRE(37)/SEMRELEASE(38): RFMEM
/// (08-04's rfork Data/Bss sharing) makes libc's CONTENDED lock() path
/// reachable for the first time — ping's rfork(RFPROC|RFMEM|RFFDG)
/// reader/sender pair shares its request list under listlock, and the
/// loser's `while(semacquire(...) < 0)` retry spins forever on a
/// loud-None.  TSEMACQUIRE(52) stays loud-None (no measured caller).
/// The kernel's syscall attack surface is exactly the measured set.
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
    t[RENDEZVOUS as usize] = Some(sys_rendezvous);
    t[BRK_ as usize] = Some(sys_brk_);
    t[UNMOUNT as usize] = Some(sys_unmount);
    t[SEMACQUIRE as usize] = Some(sys_semacquire);
    t[SEMRELEASE as usize] = Some(sys_semrelease);
    t[SEEK as usize] = Some(sys_seek);
    t[FVERSION as usize] = Some(sys_fversion);
    t[ERRSTR as usize] = Some(sys_errstr);
    t[STAT as usize] = Some(sys_stat);
    t[FSTAT as usize] = Some(sys_fstat);
    t[MOUNT as usize] = Some(sys_mount);
    t[AWAIT as usize] = Some(sys_await);
    t[PREAD as usize] = Some(sys_pread);
    t[PWRITE as usize] = Some(sys_pwrite);
    t[NSEC as usize] = Some(sys_nsec);
    t
};

/// The EC 0x15 entry (trap.rs routes here): number from frame.x0, dispatch,
/// result to frame.x0 (the restore tail's `ldp x0` pop propagates it to
/// user R0).
pub fn dispatch(frame: &mut TrapFrame) {
    let Some(mut proc) = proc::current() else {
        // An SVC with no current process is a kernel-orchestration bug
        // (dev_exec execs /init before any EL0 exists), never a user error.
        panic!("syscall dispatch with no current process (n={})", frame.x0);
    };
    let n = frame.x0;
    let result = match DISPATCH.get(n as usize) {
        Some(Some(handler)) => handler(frame, &mut proc),
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

/// Read the argument slots from user memory at SP+8 (the frozen ABI
/// formula: `(Sargs*)(frame.sp + 8)`; SP+0 is the reserved LR slot).  The
/// arg block is user memory too, so it is validated first.
///
/// Width discipline (donor-faithful): every slot is 8 bytes, but
/// int/uint-width arguments are stored by 7c with a 32-bit store that
/// leaves the slot's upper half as stack garbage — the donor's
/// `va_arg(list, int/uint)` reads only the slot's low half
/// (9front@350266104f arm64/include/u.h: `va_arg(list, mode)` with
/// sizeof(mode) == 4 reads `((mode*)list)[-2]`), so handlers must
/// likewise mask int-width args to 32 bits (await's buflen, errstr's
/// n, rfork's flag, the fd args, open's mode).  Pointer/vlong args
/// occupy the full slot.  **`long` is 32-bit in Plan 9 C on every arch
/// (7c SZ_LONG=4; only `vlong` is 64-bit) — pread/pwrite's `long n` is
/// a 32-bit arg on the wire and must be masked too** (a deep-recursion
/// smoke left pointer garbage in the reused stack pages, so an unmasked
/// n read as a huge value and the write failed silently — shallow stacks
/// never saw it because fresh demand-paged pages are zeroed).  (Arg0 is
/// always clean: the stub materializes it from R0, whose upper half the
/// caller's 32-bit op already zeroed.)
fn sargs(frame: &mut TrapFrame, proc: &mut Proc, arity: usize) -> Option<[u64; 5]> {
    let base = frame.sp + 8;
    // The arg block rides the usermem validated-only variant — it lives
    // on the live user stack, which the stub wrote through to make the
    // call (resident by construction; validated without forcing
    // residency).
    let block = UserSlice::validated_only(proc, base, 8 * arity as u64).ok()?;
    let mut out = [0u64; 5];
    for (i, slot) in out.iter_mut().enumerate().take(arity) {
        *slot = block.read_u64(8 * i)?;
    }
    Some(out)
}

/// Copy a NUL-terminated user string into a kernel Vec, validating each
/// byte's page membership BEFORE reading it and bounding the
/// length at `max` (ERRMAX-ish — Plan 9's string ceiling).  The returned
/// bytes EXCLUDE the terminator.  The loop is homed in
/// usermem::UserString; this wrapper keeps the call sites' shape.
fn copy_user_str(proc: &Proc, va: u64, max: usize) -> Result<Vec<u8>, &'static str> {
    UserString::copy_from(proc, va, max)
}

/// Error return: set the proc's errstr, return -1 on the wire.
fn err(proc: &mut Proc, msg: &'static str) -> u64 {
    proc.set_errstr(msg);
    -1i64 as u64
}

/// PWRITE(51): fd, buf, n, offset.  Offset -1 (u64::MAX) is the streaming
/// channel offset (Harvey libc write() = pwrite(fd, buf, n, -1LL));
/// the console ignores offsets (writes append), devroot is read-only,
/// devenv honors them (donor envwrite's grow-at-offset + streaming
/// advance), devpipe ignores them (donor qwrite — pipes are
/// streaming, there are no offsets).  n is a `long` — 32-bit in Plan 9 C
/// (SZ_LONG=4), so the wire slot's upper half is 7c garbage and must be
/// masked (the width discipline above).
///
/// The fd is held by a transient OP-REFERENCE (fd_op_ref/fd_op_done —
/// fdgrp.rs): the slot keeps its chan across a blocking write (devip's
/// buffer-full park, the pipe queue), so a shared-group sibling's
/// concurrent op on the same fd never sees an emptied slot — the
/// libthread interleave (aan: a blocked reader proc + the writer main
/// on one netfd) is exactly that shape; the pre-08-08 take/store
/// pattern's "pwrite: bad fd" wedge was its failure.
fn sys_pwrite(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "pwrite: bad argument block");
    };
    let (fd, buf, n, off) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff, args[3]); // fd is an int
    // Validation + cold-buffer residency are FUSED
    // in the usermem constructor — the discipline can no longer be
    // forgotten at a call site.
    let src = match UserSlice::new(proc, buf, n) {
        Ok(s) => s,
        Err(_) => return err(proc, "pwrite: bad user buffer"),
    };
    let Some(mut chan) = proc.fd_op_ref(fd) else {
        return err(proc, "pwrite: bad fd");
    };
    let result = match chan.dev {
        DEV_CONS => {
            // The device routes by qid — cons writes
            // byte-exact, null discards, hostowner stores, user is
            // read-only (the whole per-file contract lives in the device).
            port::devcons::write(&mut chan, src.as_slice())
        }
        DEV_ROOT => Err("devroot: read-only filesystem"),
        DEV_PROC => {
            // devproc-lite: the note append is the whole write —
            // the chan's qid names the target pid; offsets don't apply.
            port::devproc::write(&mut chan, src.as_slice())
        }
        DEV_ENV => {
            // devenv: the write lands in the OPERATING
            // proc's env group (the device resolves it via the
            // registered hook — donor envgrp(c)); offsets per envwrite
            // (streaming offset honored, grow-at-offset).
            port::devenv::write(&mut chan, src.as_slice(), off)
        }
        DEV_PIPE => {
            // devpipe: end i writes q[1-i]; a full queue is a
            // named error (never partial, never silent); blocked readers
            // wake after the bytes land.
            port::devpipe::write(&mut chan, src.as_slice(), off)
        }
        DEV_DUP => Err("devdup: not writable (open #d/N to reach the fd)"),
        DEV_SRV => {
            // devsrv: writing a decimal fd number to a service
            // file registers it (boot's srvcreate — FATAL on failure, so
            // this must actually work).
            port::devsrv::write(&mut chan, src.as_slice(), off)
        }
        DEV_ETHER => {
            // devether: the data file's write is the netif-faithful TX
            // path (bounds, src-MAC overwrite, ring post, block-on-full);
            // the ctl file takes the verb set (connect/promiscuous/…).
            port::devether::write(&mut chan, src.as_slice(), off)
        }
        DEV_IP => {
            // devip: the stack-debt drain FIRST (contract §12.1's idle
            // corner — a deadline that fired while nobody was blocked
            // is driven here, on the next devip-dispatched syscall),
            // then the /net file contract: ctl verb dispatch, the
            // proto data writes (icmp), ndb/arp/iproute/log.
            port::devip::stack::drain_debt_if_set();
            port::devip::write(&mut chan, src.as_slice(), off)
        }
        DEV_MNT => {
            // devmnt: paqfs is a read-only server; the early
            // client-side error is clearer than the server's Erdonly.
            Err("devmnt: the mounted tree is read-only this era")
        }
        _ => Err("pwrite: unknown device"),
    };
    proc.fd_op_done(fd, chan);
    match result {
        Ok(wrote) => wrote,
        Err(msg) => err(proc, msg),
    }
}

/// PREAD(50): fd, buf, n, offset.  devroot chans read via the paqfs
/// reader's byte-granular read_at (offset -1 uses + advances the chan's
/// streaming offset); devcons chans route by qid (cons reads are the loud
/// -1); devenv honors offsets; devpipe reads the end's queue —
/// blocking on empty (the block/wake machinery via the
/// injected hooks).  n is a `long` — 32-bit in Plan 9 C, so the wire
/// slot's upper half is masked (same discipline as sys_pwrite).
///
/// The fd is held by a transient OP-REFERENCE (fd_op_ref/fd_op_done —
/// fdgrp.rs): the slot keeps its chan across the block, so a
/// shared-group sibling's concurrent op on the same fd (libthread's
/// shape — aan's reader/writer procs ride one netfd) never sees an
/// emptied slot; the fd-group lock is never held across the block.
fn sys_pread(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "pread: bad argument block");
    };
    let (fd, buf, n, off) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff, args[3]); // fd is an int
    // The writable target's validation + cold-buffer
    // residency are fused in the usermem constructor.
    let mut dst = match UserSliceMut::new(proc, buf, n) {
        Ok(s) => s,
        Err(_) => return err(proc, "pread: bad user buffer"),
    };
    let Some(mut chan) = proc.fd_op_ref(fd) else {
        return err(proc, "pread: bad fd");
    };
    let result = match chan.dev {
        DEV_CONS => {
            // The device routes by qid — the root dir serves
            // the stat-wire listing, null reads EOF, user reads glenda,
            // hostowner reads the stored value, and cons reads get the
            // loud -1 (never a wedge).  The streaming offset
            // advances inside the device (the pread convention).
            port::devcons::read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_ROOT => {
            let Some(root) = port::devroot::root() else {
                proc.fd_op_done(fd, chan);
                return err(proc, "devroot: not mounted");
            };
            root.read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_PROC => {
            // devproc: the root/pid-dir listings, the
            // per-pid status/args reads (the ps data source); note
            // reads stay write-only-loud.
            port::devproc::read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_ENV => {
            // devenv: the read comes from the OPERATING
            // proc's env group (hook-resolved — donor envgrp(c)); the
            // root dir serves the group's stat-wire listing, a key serves
            // its value (clamped, EOF = 0); streaming offset honored.
            port::devenv::read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_PIPE => {
            // devpipe: read the end's queue — drains available
            // bytes, blocks on empty (wake on write/close), EOF on
            // hangup.  Never touches the chan offset (streaming).
            port::devpipe::read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_DUP => {
            // devdup: the root dir listing of the CALLING proc's
            // open fds (rcmain's `. -i '#d/0'` opens the fd file — its
            // reads dispatch to the REFERENCED chan's device, not here).
            port::devdup::read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_SRV => {
            // devsrv: the root dir listing of registered
            // services (a service file's reads flow through its posted
            // chan — open returns it, the donor srvopen).
            port::devsrv::read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_ETHER => {
            // devether: the netif file contract — dir listings, the
            // blocking whole-frame data read (Qmsg; the gates never
            // block on it — OQ-1), ctl's conversation id, and the live
            // stats/ifstats/addr/type content.
            port::devether::read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_IP => {
            // devip: the stack-debt drain FIRST (contract §12.1's idle
            // corner), then the /net file contract: dir listings, the
            // blocking proto data reads (Qmsg — the seam's
            // register→drop→block), ctl's conv number, and the live
            // status/stats/local/remote/ipselftab content.
            port::devip::stack::drain_debt_if_set();
            port::devip::read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_MNT => {
            // devmnt: chunked Tread RPCs over the mounted tree's
            // session (blocks mid-syscall — the pipe read schedules the
            // server proc; the fd is checked out, never the group lock).
            port::devmnt::read(&mut chan, dst.as_slice_mut(), off)
        }
        _ => Err("pread: unknown device"),
    };
    proc.fd_op_done(fd, chan);
    match result {
        Ok(read) => read,
        Err(msg) => err(proc, msg),
    }
}

/// NOTIFY(28): handler VA (a full-width pointer arg).  nil clears the
/// handler (libc atnotify's notify(0)); otherwise the VA must be 4-aligned
/// and land inside the proc's OWN text segment (a process can
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
            // The saved-ureg window rides the usermem validated-only
            // variant — deliver_notes forced its pages resident at
            // delivery time and nothing unmaps them while the handler
            // runs (validated without re-forcing residency).
            let saved = match UserSlice::validated_only(proc, nureg, trap::UREG_SIZE as u64) {
                Ok(s) => s,
                Err(_) => return err(proc, "noted: bad saved ureg"),
            };
            // Validate the resume target BEFORE committing the restore
            // (the donor noted()'s okaddr(pc)+okaddr(sp)+alignment checks,
            // pre-copy so a bad saved frame leaves the live frame
            // untouched and the -1 writeback is truthful).
            let saved_sp = saved.read_u64(248).expect("ureg->sp in range"); // ureg->sp
            let saved_pc = saved.read_u64(256).expect("ureg->pc in range"); // ureg->pc
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
            // Restore: the saved 280 bytes over the live frame's 35 fields
            // (the usermem slice is the source; the frame-side byte copy
            // lives in TrapFrame::restore_from_saved).
            frame.restore_from_saved(saved.as_slice());
            // The copied psr came from user-writable memory (the
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

/// OPEN(14): path, mode.  EVERY path
/// resolves through the namespace core — namec + the proc's mount table —
/// and the resolved chan opens via the per-device dispatch
/// (namec::open_chan).  The retired scaffolding routed paths by literal
/// name matching; the namespace core is the only resolver (no
/// name-matching fallback, never extended for new devices).
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

    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "open: no namespace (pre-exec)");
    };
    // NO pgrp lock is held across resolution or the open — namec
    // consults the mount table under per-consult snapshots, and
    // open_chan's Topen RPC blocks mid-syscall (sys_mount's documented
    // discipline, applied uniformly).
    let c = match namec::namec(&path, slash, dot, &proc.pgrp, NamecMode::Open) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let opened = match namec::open_chan(&c, mode) {
        Ok(c) => c,
        Err(msg) => {
            // The resolved chan never reaches an fd
            // on the error exit — close-dispatch it (a walked DEV_MNT
            // chan holds a server-side fid; clunk_if_last's count check
            // makes other devices no-ops).
            port::namec::close_chan(&c);
            return err(proc, msg);
        }
    };

    match proc.fd_install_lowest(opened) {
        Some(slot) => slot,
        None => err(proc, "open: no free fd"),
    }
}

/// BIND(2): name, old, flag.  Both path strings cross the trust boundary:
/// sargs + validate_user_range BEFORE any kernel deref.  The source
/// resolves with the union-head redirect (donor
/// namec's Abind domount); the target NEVER redirects (Amount — stacked
/// binds attach to the original directory) and must EXIST
/// (MCREATE permits later creation IN the mounted dir; it never conjures
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

    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "bind: no namespace (pre-exec)");
    };
    // Both resolves run WITHOUT the pgrp lock held (namec's
    // per-consult snapshots; a mount-crossing walk is a devmnt Twalk RPC
    // that blocks mid-syscall); the lock covers the mount-table INSERT
    // only.
    let src = match namec::namec(&name, slash, dot, &proc.pgrp, NamecMode::BindSource) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let tgt = match namec::namec(&old, slash, dot, &proc.pgrp, NamecMode::MountTarget) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let mut guard = pg.lock(&node);
    let result = guard.bind(src, &tgt, flag);
    drop(guard);
    // The resolved TARGET chan is never installed
    // anywhere — close-dispatch it (a mount-crossing resolution can leave
    // a last-ref DEV_MNT fid on it; the SOURCE is table-pinned on success
    // by design).
    port::namec::close_chan(&tgt);
    match result {
        Ok(_) => 0,
        Err(msg) => err(proc, msg),
    }
}

/// CHDIR(3): path (callers: ps's /proc chdir, newns's cd).
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
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "chdir: no namespace (pre-exec)");
    };
    // Resolution runs WITHOUT the pgrp lock held (namec's
    // per-consult snapshots).
    let c = match namec::namec(&path, slash, dot, &proc.pgrp, NamecMode::ToDir) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    if c.qid.typ & port::dat::QTDIR == 0 {
        // The resolved chan is dropped on the error
        // exit — close-dispatch it first (a walked DEV_MNT fid's only
        // ref is this local).
        port::namec::close_chan(&c);
        return err(proc, "chdir: not a directory");
    }
    // The REPLACED dot's fid ref leaves with it —
    // close-dispatch the old dot (clunk_if_last's count check protects a
    // shared/rfork-dup'd dot).
    if let Some(old) = proc.dot.replace(c) {
        port::namec::close_chan(&old);
    }
    0
}

/// UNMOUNT(35): name, old (boot's settime /tmp
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

    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "unmount: no namespace (pre-exec)");
    };
    // Both resolves run WITHOUT the pgrp lock held (namec's
    // per-consult snapshots); the lock covers the mount-table REMOVAL
    // only.
    let tgt = match namec::namec(&name, slash, dot, &proc.pgrp, NamecMode::MountTarget) {
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
        match namec::namec(&old, slash, dot, &proc.pgrp, NamecMode::BindSource) {
            Ok(c) => Some(c),
            Err(msg) => return err(proc, msg),
        }
    };
    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let mut guard = pg.lock(&node);
    let result = guard.unmount(&tgt.path, from.as_ref());
    drop(guard);
    // The resolved chans are never installed —
    // close-dispatch them (clunk_if_last's count check protects any
    // shared fid; the table's REMOVED entries stay table-lifetime-pinned,
    // the documented bounded-by-mount-count deferral).
    port::namec::close_chan(&tgt);
    if let Some(f) = &from {
        port::namec::close_chan(f);
    }
    match result {
        Ok(()) => 0,
        Err(msg) => err(proc, msg),
    }
}

/// FVERSION(40): fd, msize, version, nversion.  Donor sysfversion
/// (9k sysauth.c):
/// `int fversion(int fd, u32int msize, char *version, int nversion)` —
/// negotiate the 9P session over the fd's chan (devmnt::fversion, the
/// mntversion port) and write back the negotiated version string,
/// returning its length.  The fd must be a 9P-capable connection open
/// ORDWR (the connectembed pipe end).  The version buffer is
/// validated + forced resident BEFORE the kernel reads/writes it (the
/// cold-buffer class); the donor's Ebadarg NUL check is ported
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
    if nver == 0 {
        return err(proc, "fversion: zero-length version buffer");
    }
    // One fused constructor covers the read of the
    // requested version string AND the negotiated write-back (the buffer
    // is read+written, so the writable variant covers both).
    let mut verbuf = match UserSliceMut::new(proc, ver_va, nver) {
        Ok(b) => b,
        Err(_) => return err(proc, "fversion: bad version buffer"),
    };
    let requested = {
        let buf = verbuf.as_slice();
        let Some(nul) = buf.iter().position(|&b| b == 0) else {
            return err(proc, "fversion: bad version specification");
        };
        &buf[..nul]
    };
    if !requested.is_empty() && requested != port::devmnt::VERSION9P {
        return err(proc, "fversion: bad 9P version specification");
    }
    let Some(mut chan) = proc.fd_take(fd) else {
        return err(proc, "fversion: bad fd");
    };
    let result = port::devmnt::fversion(&mut chan, msize, verbuf.as_slice_mut());
    proc.fd_store(fd, chan);
    match result {
        Ok(len) => len,
        Err(msg) => err(proc, msg),
    }
}

/// MOUNT(46): fd, afd, old, flag, aname
/// (nsinit's `mount(fd, -1, "/root", MREPL|MCREATE, rp)` and settime's
/// second mount onto /tmp via #s/boot).  Donor sysmount + bindmount's
/// ismount arm (9k sysfile.c): validate the flags (bindmount's Ebadarg
/// shape), the fd must be a 9P connection (auto-version when fversion
/// never ran — the donor mntattach's Enoversion fallback), Tattach over
/// the session with afid = NOFID (afd ≠ -1 is loud-unsupported — the
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
    // attach to the original directory) and insert the entry.  The
    // resolve runs WITHOUT the pgrp lock held (namec's per-consult
    // snapshots — a mount-crossing walk RPCs); the lock covers the
    // mount-table INSERT only.
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "mount: no namespace (pre-exec)");
    };
    let tgt = match namec::namec(&old, slash, dot, &proc.pgrp, NamecMode::MountTarget) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    mnt_chan.path = tgt.path.clone();
    let pg = proc.pgrp.clone();
    let node = LockNode::new();
    let mut guard = pg.lock(&node);
    let result = guard.bind(mnt_chan, &tgt, flag);
    drop(guard);
    // The resolved TARGET chan is never installed —
    // close-dispatch it (the mount-root chan itself is table-pinned on
    // success by design; on failure it dies inside bind).
    port::namec::close_chan(&tgt);
    match result {
        Ok(_) => 0,
        Err(msg) => err(proc, msg),
    }
}

/// STAT(42): path, buf, n.  namec with Aaccess semantics (the final
/// component DOES cross
/// to the union head), then the device's stat op writes the stat(5) wire
/// bytes; the return is the byte count (the donor sysstat contract).
/// The path string AND the output buffer (writable) validate
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
    // The output buffer's validation + residency
    // (the cold-buffer class) are fused in the constructor.
    let mut sb = match UserSliceMut::new(proc, buf_va, n) {
        Ok(b) => b,
        Err(_) => return err(proc, "stat: bad user buffer"),
    };
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "stat: no namespace (pre-exec)");
    };
    // Resolution runs WITHOUT the pgrp lock held (namec's
    // per-consult snapshots; a mount-crossing walk RPCs mid-syscall).
    let c = match namec::namec(&path, slash, dot, &proc.pgrp, NamecMode::Open) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    // The resolved chan is never fd-installed here —
    // close-dispatch it after the stat (a walked DEV_MNT chan holds a
    // server-side fid; its only ref is this local).
    let result = namec::stat_chan(&c, sb.as_slice_mut());
    port::namec::close_chan(&c);
    match result {
        Ok(count) => count,
        Err(msg) => err(proc, msg),
    }
}

/// FSTAT(43): fd, buf, n.  The open fd's
/// chan stats through the same device dispatch (the donor sysfstat shape
/// — no namec, no mount crossing: the chan already resolved at open).
/// devroot's stat reads the chan's paqfs entry, so DEV_ROOT stats go
/// through the LIVE fd slot; the other devices' stats read qid/dev only.
fn sys_fstat(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "fstat: bad argument block");
    };
    let (fd, buf_va, n) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff); // fd, n are ints
    // The output buffer's validation + residency
    // (the cold-buffer class) are fused in the constructor.
    let mut sb = match UserSliceMut::new(proc, buf_va, n) {
        Ok(b) => b,
        Err(_) => return err(proc, "fstat: bad user buffer"),
    };
    let Some(chan) = proc.fd_take(fd) else {
        return err(proc, "fstat: bad fd");
    };
    let result = namec::stat_chan(&chan, sb.as_slice_mut());
    proc.fd_store(fd, chan);
    match result {
        Ok(count) => count,
        Err(msg) => err(proc, msg),
    }
}

/// BRK_(24): addr (malloc's pool growth rides it —
/// sbrkalloc's sbrk→brk_ — and boot's dirstat mallocs depend on it).
/// Donor sysbrk_ (9k sysseg.c): ibrk(addr, BSEG) —
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
    if proc.segments.is_none() {
        return err(proc, "brk: no segments (pre-exec)");
    }
    if addr == 0 {
        return 0; // no-op (libc never passes 0; defensive)
    }
    let (bss_start, old_end) = {
        let segs = proc.segments.as_ref().unwrap();
        (segs.bss.0.start, segs.bss.0.end)
    };
    if addr < bss_start {
        return err(proc, "brk: address below the heap base");
    }
    if addr > exec::USTOP {
        return err(proc, "brk: out of memory");
    }
    match proc.rfmem_family {
        // The ordinary private shape: move the caller's break only;
        // growth pages fault in lazily on touch.
        None => {
            proc.segments.as_mut().unwrap().bss.0.end = addr;
            0
        }
        // The RFMEM family shape (the donor's shared-segment break):
        // every member's break moves together and growth pages are born
        // shared — ping's sender-malloc'd request nodes are readable by
        // the receiver.  proc.rs's rfmem_brk_set does the maps and the
        // end moves (with a full unwind on failure).
        Some(family) => match proc::rfmem_brk_set(family, old_end, addr) {
            Ok(()) => 0,
            Err(msg) => err(proc, msg),
        },
    }
}

/// Read the semaphore word (a user `int` — 4 bytes, little-endian on
/// aarch64, the native word order).
fn sem_word_read(word: &UserSliceMut) -> i32 {
    i32::from_le_bytes(word.as_slice()[..4].try_into().expect("sem word is 4 bytes"))
}

/// Write the semaphore word.
fn sem_word_write(word: &mut UserSliceMut, val: i32) {
    word.as_slice_mut()[..4].copy_from_slice(&val.to_le_bytes());
}

/// The donor's canacquire: decrement a positive word, else fail.
/// Single CPU + IRQ-masked syscall context (no preemption mid-handler)
/// makes the load/store atomic against other procs; the RFMEM-shared
/// page is the cross-proc synchronization point.
fn sem_try_acquire(word: &mut UserSliceMut) -> bool {
    let val = sem_word_read(word);
    if val <= 0 {
        return false;
    }
    sem_word_write(word, val - 1);
    true
}

/// Translate a resident user VA to its physical address via the proc's
/// mapping_log.  The semaphore's wait-queue key is the word's PHYSICAL
/// address: RFMEM family members map the same PA (one queue per WORD),
/// while unrelated procs reusing the VA name different PAs and never
/// meet.
fn sem_word_pa(proc: &Proc, va: u64) -> Option<u64> {
    let va = va as usize;
    proc.mapping_log
        .iter()
        .find(|m| m.va <= va && va < m.va + m.size)
        .map(|m| m.phys as u64 + (va - m.va) as u64)
}

/// SEMACQUIRE(37): addr, block.  The donor's syssemacquire (9k
/// sysproc.c): a counting semaphore whose WORD lives in user memory
/// (libc Lock.sem — ping's listlock is the 08-04 gate's caller).  Fast
/// path: decrement a positive word and return 1.  Contended path
/// (block != 0): register the caller on the word's PA and block in
/// sched(), re-checking on wake (spurious-allowed: a woken waiter that
/// loses the unit race re-blocks and stays registered).  Returns 1 on
/// acquire, 0 when block == 0 and the word is exhausted.  Era limit:
/// the donor's note-interruptible sleep is NOT reproduced — r9's block
/// is uninterruptible (a note queued mid-wait delivers at the syscall's
/// return), so libc's `while(semacquire(...) < 0)` retry shape never
/// fires here.
fn sys_semacquire(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "semacquire: bad argument block");
    };
    let addr = args[0]; // a full-width pointer arg (no masking)
    let block = args[1] & 0xffff_ffff; // block is an int (width discipline)
    // The donor's entry validation (syssemacquire): evenaddr for an int,
    // a writable in-segment word, a never-negative value.
    if addr & 3 != 0 {
        return err(proc, "bad arg in system call");
    }
    let mut word = match UserSliceMut::new(proc, addr, 4) {
        Ok(w) => w,
        Err(_) => return err(proc, "bad arg in system call"),
    };
    if sem_word_read(&word) < 0 {
        return err(proc, "bad arg in system call");
    }
    // The fast path — the uncontended acquire (a private-memory lock is
    // never contended at all; an RFMEM lock usually isn't either).
    if sem_try_acquire(&mut word) {
        return 1;
    }
    if block == 0 {
        return 0;
    }
    // The word is resident (the constructor forced it), so the PA lookup
    // cannot fail.
    let Some(pa) = sem_word_pa(proc, addr) else {
        return err(proc, "bad arg in system call");
    };
    proc::sem_register_waiter(pa, proc.pid);
    loop {
        if sem_try_acquire(&mut word) {
            break;
        }
        proc::sched();
    }
    proc::sem_deregister_waiter(pa, proc.pid);
    1
}

/// SEMRELEASE(38): addr, delta.  The donor's syssemrelease: add `delta`
/// to the word, then wake up to `delta` waiters registered on the
/// word's PA (registration order, the donor semwakeup's list walk).
/// Returns the new value (the donor's `value+delta` wire result).
fn sys_semrelease(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "semrelease: bad argument block");
    };
    let addr = args[0]; // a full-width pointer arg (no masking)
    let delta = (args[1] & 0xffff_ffff) as i32; // delta is an int
    if addr & 3 != 0 {
        return err(proc, "bad arg in system call");
    }
    let mut word = match UserSliceMut::new(proc, addr, 4) {
        Ok(w) => w,
        Err(_) => return err(proc, "bad arg in system call"),
    };
    let value = sem_word_read(&word);
    if delta < 0 || value < 0 {
        return err(proc, "bad arg in system call");
    }
    let Some(pa) = sem_word_pa(proc, addr) else {
        return err(proc, "bad arg in system call");
    };
    let new = value + delta;
    sem_word_write(&mut word, new);
    proc::sem_wake(pa, delta as u64);
    new as u64
}

/// EXEC(7): path, argv — 2 syscall args (libc `exec(char *name,
/// char *argv[])`).  Donor semantics: returns ONLY on failure (-1/errstr,
/// old image intact); on success exec::exec_replace rewrites the live trap
/// frame (R0 = tos, SP → new seed stack, PC = new entry, PSR = 0) and the
/// dispatch return erets into the new image.
///
/// Both args are user memory crossing the trust boundary: the
/// path string and the argv block + every pointed-to string are validated
/// against the proc's segment map before copying, with argc bounded at 32
/// and each arg at 128 bytes.  Everything is COPIED to kernel memory before
/// exec_replace runs — the replacement frees the old image's pages, which
/// is where the arguments lived.
fn sys_exec(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    const MAX_EXEC_ARGS: usize = 32;
    const MAX_EXEC_ARG_LEN: usize = 128;
    /// The whole-file staging ceiling for a MOUNTED binary (read
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
            // The argv slot block rides
            // usermem's validated-only pointer — the slots are user
            // stack/data the process itself wrote to set up the call
            // (resident by construction).
            let slot = match UserPtr::validated_only(proc, slot_va) {
                Ok(p) => p,
                Err(_) => return err(proc, "exec: bad argv block"),
            };
            let ptr = slot.read_u64();
            if ptr == 0 {
                break;
            }
            match copy_user_str(proc, ptr, MAX_EXEC_ARG_LEN) {
                Ok(s) => argv.push(s),
                Err(msg) => return err(proc, msg),
            }
        }
    }

    // Resolve through the NAMESPACE —
    // namec + the proc's mount table — so an exec path crosses mounts
    // (execinit's /aarch64/init through the union-mounted /; init's
    // /bin/rc through the post-newns /bin union into the mounted tree).
    // The old devroot-only prefix walk is retired (resolution lives in
    // the namespace core).
    // NO pgrp lock is held across resolution, open, or stage —
    // namec consults under per-consult snapshots; a devmnt open/read RPCs
    // and BLOCKS mid-syscall (the pipe read schedules the server proc).
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "exec: no namespace (pre-exec)");
    };
    let c = match namec::namec(&path, slash, dot, &proc.pgrp, NamecMode::Open) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let opened = match namec::open_chan(&c, OEXEC) {
        Ok(c) => c,
        Err(msg) => {
            // The walked chan is walked-past on the
            // error exit too — close-dispatch it (clunk_if_last's count
            // check protects non-DEV_MNT chans).
            port::namec::close_chan(&c);
            return err(proc, msg);
        }
    };
    // The walked chan is walked-past once the open
    // SHARES its fid's Arc — close-dispatch it now so the fid's last ref
    // (the opened chan's, below) Tclunks exactly once.  Without this the
    // shared count strands the fid at close time (a per-command leak).
    port::namec::close_chan(&c);
    let backing = match opened.dev {
        DEV_ROOT => {
            let Some(root) = port::devroot::root() else {
                port::namec::close_chan(&opened);
                return err(proc, "devroot: not mounted");
            };
            match root.entry(&opened) {
                Ok(bin) => ByteBacking::Paq { dir: bin, image: root.image() },
                Err(msg) => {
                    port::namec::close_chan(&opened);
                    return err(proc, msg);
                }
            }
        }
        DEV_MNT => {
            // A mounted binary is read WHOLE here (devmnt::read_all) —
            // syscall context, where the 9P client's blocking is by
            // design; the trap-context pager must never RPC, so the
            // staged bytes become the demand-page byte source.
            match port::devmnt::read_all(&opened, EXEC_STAGE_CAP) {
                Ok(bytes) => ByteBacking::Mnt { file: alloc::sync::Arc::new(bytes) },
                Err(msg) => {
                    port::namec::close_chan(&opened);
                    return err(proc, msg);
                }
            }
        }
        _ => {
            port::namec::close_chan(&opened);
            return err(proc, "exec: path is not an executable source");
        }
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
            // The close dispatch (the donor's devclose) — a pipe
            // end's close decrements the open count and the last one
            // hangs the peer's read queue up (the EOF path).
            port::namec::close_chan(&c);
            0
        }
        None => err(proc, "close: bad fd"),
    }
}

/// PIPE(21): fd-array-ptr (the connectembed substrate — the ONE
/// connectembed pipe).  Donor
/// syspipe (9k sysfile.c): attach a FRESH pipe ("#|" — one Pipe per
/// attach, never shared across attaches), walk data/data1, open both
/// ORDWR, install the pair at the two lowest free slots (found BEFORE
/// anything installs — a one-slot table fails with a named errstr and
/// consumes nothing), and write the two fds back through the validated
/// user array (two 32-bit ints — the width discipline).
///
/// The ends reference the ONE Arc-shared Pipe: writes on end
/// i land in the queue end 1-i reads; fork/dup clone the Arc, never the
/// bytes.
fn sys_pipe(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "pipe: bad argument block");
    };
    let arr_va = args[0];
    // The fd array's validation + residency are
    // fused in the constructor (the cold-buffer class).
    let mut arr = match UserSliceMut::new(proc, arr_va, 8) {
        Ok(a) => a,
        Err(_) => return err(proc, "pipe: bad fd array"),
    };
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
    let out = arr.as_slice_mut();
    out[0..4].copy_from_slice(&(pair.0 as u32).to_le_bytes());
    out[4..8].copy_from_slice(&(pair.1 as u32).to_le_bytes());
    0
}

/// DUP(5): oldfd, newfd (connectembed's child dups the pipe ends onto
/// 0/1).  Donor sysdup
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

/// CREATE(22): path, mode, perm (callers: every env-var set + boot's
/// srvcreate).  Donor syscreate +
/// chan.c's Acreate: resolve the PARENT (open-style union redirect), then
/// if the final component EXISTS open it OTRUNC (unless OEXCL → Eexist),
/// else dispatch the device's create op (devenv/devsrv are this era's
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

    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "create: no namespace (pre-exec)");
    };
    // Resolution runs WITHOUT the pgrp lock held — namec consults
    // under per-consult snapshots, and the dot-redirect consult below
    // snapshots under a scoped lock, dropping it BEFORE the fid-clone (a
    // devmnt zero-name Twalk RPC).  walk_one/open_chan/create_chan run
    // lock-free (a DEV_MNT parent's walk/create RPCs mid-syscall).
    let parent = match split {
        Some(0) => namec::namec(b"/", slash, dot, &proc.pgrp, NamecMode::Open),
        Some(i) => namec::namec(&path[..i], slash, dot, &proc.pgrp, NamecMode::Open),
        None => {
            // The dot itself, with the open-mode final redirect (a dot
            // covered by a mount point creates in the union head).
            // A DEV_MNT head is fid-CLONED, never bare-dup'd (the
            // mount-table aliasing fix — the redirect's open/close must
            // not touch the table's server-side fid).
            let mut d = dot.dup();
            let chain = {
                let node = LockNode::new();
                let guard = proc.pgrp.lock(&node);
                guard.chain_snapshot(&d.path)
            };
            if let Some(chain) = chain {
                let Some(head) = chain.first() else {
                    return err(proc, "create: empty union chain");
                };
                let mut nc = match port::namec::redirect_clone(&head.to) {
                    Ok(c) => c,
                    Err(msg) => return err(proc, msg),
                };
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
        // The resolved parent (or the manual
        // dot-redirect clone) holds a server-side fid on EVERY exit —
        // sweep it (clunk_if_last's count check protects the shared dot
        // dup case).
        port::namec::close_chan(&parent);
        return err(proc, "create: parent not a directory");
    }
    let opened = match namec::walk_one(&parent, final_name) {
        Ok(existing) => {
            // The donor's Acreate existing-file arm: OEXCL → Eexist;
            // otherwise open OTRUNC (the create-on-existing fallback).
            if mode & port::dat::OEXCL != 0 {
                port::namec::close_chan(&existing);
                port::namec::close_chan(&parent);
                return err(proc, "create: file exists");
            }
            match namec::open_chan(&existing, mode | OTRUNC) {
                Ok(c) => c,
                Err(msg) => {
                    port::namec::close_chan(&existing);
                    port::namec::close_chan(&parent);
                    return err(proc, msg);
                }
            }
        }
        Err(_) => {
            match namec::create_chan(&parent, final_name, mode & !(port::dat::OEXCL | port::dat::OCEXEC), perm) {
                Ok(c) => c,
                Err(msg) => {
                    port::namec::close_chan(&parent);
                    return err(proc, msg);
                }
            }
        }
    };
    // The parent's fid (the dot-redirect clone's, on
    // the bare-name arm) is walked-past on success too — sweep it.  (The
    // walk-probe `existing` needs no sweep on success: the open SHARES
    // its fid's Arc and `opened` is now the owning ref.)
    port::namec::close_chan(&parent);
    match proc.fd_install_lowest(opened) {
        Some(slot) => slot,
        None => err(proc, "create: no free fd"),
    }
}

/// FD2PATH(23): fd, buf, n.  Donor sysfd2path: the fd's chan's walked
/// path (chanpath)
/// into the validated user buffer, NUL-terminated and truncated to n
/// (Harvey libc: fd2path(int, char*, int) — n is int-width, masked).
/// Returns 0.  The path bookkeeping is the walk-maintained one
/// (fd2path's raison d'être) — never the errstr side channel.
fn sys_fd2path(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "fd2path: bad argument block");
    };
    let (fd, buf_va, n) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff); // fd, n are ints
    // The fd2path buffer's validation + residency
    // (the cold-buffer class) are fused in the constructor.
    let mut dst = match UserSliceMut::new(proc, buf_va, n) {
        Ok(b) => b,
        Err(_) => return err(proc, "fd2path: bad user buffer"),
    };
    let Some(path) = proc.fd_path(fd) else {
        return err(proc, "fd2path: bad fd");
    };
    let dst = dst.as_slice_mut();
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
/// RFPROC.  Supported this era: the address space follows the donor's
/// dupseg shape (sys/src/9/port/segment.c) — WITHOUT RFMEM every user
/// page is eagerly copied page-for-page (no COW); WITH RFMEM the
/// Data/Bss pages SHARE the parent's physical pages (refcount-gated
/// free at the last sharer's exit/exec; proc.rs's SHARED_PAGES) while
/// Stack/Tos/Text stay fresh copies (SG_STACK is never shared — note
/// delivery writes the proc's own stack).  Known limit: pages brk_-grown
/// or demand-faulted by one sharer AFTER the fork are per-proc (the
/// donor's shared-segment pte map would show them to both; ping's
/// reader/sender pair never crosses this — its shared request list
/// lives in fork-time-present pages); full demand-page sharing is a
/// later refinement.  RFNOWAIT is honored (EXITS posts no wait record);
/// RFREND splits the rendezvous group (the RENDEZVOUS(34) handler's
/// group semantics —
/// measured into the dispatch set by the 06-07 tier-1 trace); RFNOTEG
/// has REAL note-group semantics
/// (Proc.noteid — a fresh group per construction,
/// inherited without RFNOTEG, fresh with it; the console DEL posts
/// "interrupt" to the readers' groups).
/// The namespace group semantics per fork(2): RFPROC+neither shares the
/// parent's pgrp, RFNAMEG deep-copies, RFCNAMEG starts clean — and the
/// non-RFPROC split (boot(8)'s `rfork(RFNAMEG)`): the CALLER's group
/// becomes a fresh copy (RFNAMEG) or fresh empty (RFCNAMEG).
/// The ENV group semantics, the same donor shape (fork(2):
/// "RFENVG: the environment variables are COPIED; otherwise the two
/// processes SHARE"; "RFCENVG: the new process starts with an EMPTY
/// environment"): RFPROC+neither shares the parent's env group, RFENVG
/// deep-copies (envcpy), RFCENVG empties; the non-RFPROC env split
/// (newns's `rfork(RFENVG|RFCNAMEG)`) re-groups the CALLER (copy on
/// RFENVG, empty on RFCENVG).
/// The FD group semantics, again per fork(2)/sysrfork ("RFFDG: the
/// invoker's file
/// descriptor table is COPIED; otherwise the two processes SHARE a
/// single table"): RFPROC+neither SHARES the Arc<Lock<FdGroup>> handle,
/// RFFDG deep-COPIES slot-by-slot (dupfgrp — pipe chans clone their
/// Arc), RFCFDG empties (dupfgrp(nil)).  libc fork() =
/// rfork(RFPROC|RFFDG|RFREND) = COPY — connectembed's child dup/close
/// choreography depends on it.
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
        // the env group split (RFENVG copy / RFCENVG empty —
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
        // Non-RFPROC rfork(RFNOTEG) re-groups the CALLER
        // into a fresh note group (9k sysproc.c:82) — init's fexec child
        // does this before exec'ing rc, so console DELs target the
        // shell's group and never init.
        if flag & RFNOTEG != 0 {
            proc::rfork_noteg_split(proc);
        }
        // Non-RFPROC rfork(RFREND) re-groups the CALLER into a fresh
        // rendezvous group (9k sysproc.c:69-72).
        if flag & RFREND != 0 {
            proc::rfork_rendez_split(proc);
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

/// SEEK(39): rv, fd, offset, base (paqfs's libbio Bseek):
/// **the aarch64 syscall ABI is FOUR arguments —
/// `_seek(vlong *rv, int fd, vlong offset, int base)`** (the generated
/// stub's shape, sys/src/libc/9syscall/mkfile's `j=_seek` arm + the
/// aarch64 `_seek.c` wrapper; the donor sysfile.c sysseek's own comment:
/// "The system call actually has 4 arguments... the first argument is
/// where the offset is returned").  The result offset is WRITTEN through
/// the validated rv pointer; the wire result is 0 (error: -1 + errstr,
/// rv untouched — the wrapper then yields -1LL).  Donor sseek semantics:
/// base 0 absolute, base 1 relative (negative deltas allowed, never below
/// 0); base 2 (SEEK_END) is offset-from-end via the chan's stat length
/// (9k sysfile.c case 2: `c->dev->stat` → convM2D → dir.length) — the
/// 06-07 tier-1 gate measured its need (tail -1's length probe,
/// join.c:148's random-access check).  Stream devices (pipes) always
/// error (the donor sseek's `dc == L'|'` → Eisstream check — tail.c's
/// isseekable probe keys on it).  Directories error (Eisdir);
/// base > 2 is a bad-args error.
///
/// (An earlier version read fd/offset/base from slots 0/1/2 and returned
/// the offset on the wire — wrong for this ABI; every early caller's
/// seek was bio's in-buffer fast path, so nothing exercised it.  paqfs's
/// blockRead seeks for real.)
fn sys_seek(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "seek: bad argument block");
    };
    // rv is a full pointer; fd/base are ints (the width discipline);
    // offset is a vlong.
    let (rv, fd, off, base) =
        (args[0], args[1] & 0xffff_ffff, args[2] as i64, (args[3] & 0xffff_ffff) as u32 as i32);
    // The result pointer's validation + residency
    // (the cold-buffer class) are fused in the constructor.
    let mut rv_slot = match UserSliceMut::new(proc, rv, 8) {
        Ok(s) => s,
        Err(_) => return err(proc, "seek: bad result pointer"),
    };
    let Some(mut chan) = proc.fd_take(fd) else {
        return err(proc, "seek: bad fd");
    };
    let result = (|chan: &mut Chan| {
        // The donor sseek's stream-device rejection (9front sysfile.c:
        // `devtab[c->type]->dc == L'|' → Eisstream`) — a pipe end is never
        // seekable.  tail.c's isseekable probe (seek(fd, 0, 1) >= 0) keys
        // on this failure to choose its read-forward path; without the
        // rejection the probe "succeeds" and tail reads a pipe backward.
        if chan.dev == port::dat::DEV_PIPE {
            return Err("seek: stream device");
        }
        if chan.qid.typ & port::dat::QTDIR != 0 {
            return Err("seek: is a directory");
        }
        let new = match base {
            0 => off,
            1 => match (chan.offset as i64).checked_add(off) {
                Some(v) => v,
                None => return Err("seek: offset overflow"),
            },
            2 => {
                // SEEK_END: the chan's stat gives the length (the donor's
                // case-2 shape).  The stat wire's fixed prefix carries
                // length as a u64 LE at byte offset 33 (size[2] type[2]
                // dev[4] qid[13] mode[4] atime[4] mtime[4] precede it) —
                // the prefix layout is stat(5)/convM2D-stable.
                let mut sb = [0u8; 512];
                let n = namec::stat_chan(chan, &mut sb)?;
                if n < 41 {
                    return Err("seek: short stat record");
                }
                let length = u64::from_le_bytes(sb[33..41].try_into().map_err(|_| "seek: stat length")?);
                match (length as i64).checked_add(off) {
                    Some(v) => v,
                    None => return Err("seek: offset overflow"),
                }
            }
            _ => return Err("seek: bad base"),
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
            rv_slot.write_u64(0, v);
            0
        }
        Err(msg) => err(proc, msg),
    }
}

/// ERRSTR(41): buf, n — copy the proc's errstr (NUL-terminated) into the
/// user buffer, return 0.  libc's `%r` verb reads the errstr this way
/// (rerrstr calls it twice) — the smoke's failure prints are its measured
/// caller.
fn sys_errstr(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "errstr: bad argument block");
    };
    let (buf, n) = (args[0], args[1] & 0xffff_ffff); // n is a uint (width discipline)
    // The errstr buffer's validation + residency
    // (the cold-buffer class) are fused in the constructor.
    let mut dst = match UserSliceMut::new(proc, buf, n) {
        Ok(b) => b,
        Err(_) => return err(proc, "errstr: bad user buffer"),
    };
    let cap = (n as usize).min(ERRMAX);
    if cap == 0 {
        return 0;
    }
    let src = proc.errstr;
    let len = src.iter().position(|&b| b == 0).unwrap_or(ERRMAX).min(cap - 1);
    let dst = dst.as_slice_mut();
    dst[..len].copy_from_slice(&src[..len]);
    dst[len] = 0;
    0
}

/// SLEEP(17): millisecs (init's for(;;) restart loop sleeps 1s between
/// rc generations).  Donor syssleep (9k sysproc.c): ms<=0 yields (give up
/// the CPU if another process is waiting); ms>0 tsleeps until the tick
/// count passes now+ms (1 tick = 1 ms this era — timer.rs
/// TICKS_PER_SEC=1000, so ms IS the tick delta).  The block rides the
/// sched machinery; the tick's tick_timeouts readies expired
/// sleepers.  Register-deadline-then-block cannot lose the wake: syscall
/// context is IRQ-masked, so the tick can only fire after sched()
/// swtch'es (devpipe's documented invariant).  The ms arg is a `long` —
/// 32-bit in Plan 9 C (the width discipline).
///
/// The donor's note-interruptible tsleep: a pending-note PRE-CHECK fails
/// the sleep immediately (the donor tsleep's notepending shape — never
/// arm-then-lose), and tick_timeouts readies a sleeper whose note_queue
/// is non-empty.  On resume, a LOCAL deadline copy discriminates: an
/// early wake (ticks() < deadline) is always a posted note this era (the
/// waker inventory is enumerable — tick_timeouts's expiry+note arms;
/// cons_input_wake keys on cons_waiting; pipe_wake keys on waiter pids —
/// neither can ready a sleeping proc), so the syscall fails -1 with
/// errstr "interrupted"; expiry (ticks() >= deadline) returns 0.  A note
/// that arrives AT the expiry tick still takes the 0 exit (the expiry
/// arm ran first) and delivers at the return boundary — the donor's
/// shape.  The note itself delivers at the return-to-EL0 boundary,
/// untouched.
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
        // sleep(long) is SIGNED — the donor syssleep yields when
        // n <= 0 (9k sysproc.c).  Read as i32 so sleep(-1) (a computed
        // negative duration, arriving as 0xffffffff) yields instead of
        // sleeping 4294967295 ticks (~49.7 days at 1 kHz, no wake path
        // other than expiry).
        if (ms as i32) <= 0 {
            proc::preempt(); // the donor's yield()
            return 0;
        }
        // The donor tsleep's notepending pre-check —
        // a note queued BEFORE the arm fails the sleep immediately.
        if !proc.note_queue.is_empty() {
            return err(proc, "interrupted");
        }
        let deadline = crate::platform::timer::ticks() + ms;
        proc.sleep_until = deadline;
        proc::sched();
        // An early wake is always a posted note this era (see the doc
        // comment's waker inventory); expiry returns 0.
        if crate::platform::timer::ticks() < deadline {
            return err(proc, "interrupted");
        }
        0
    }
    #[cfg(not(dev_exec))]
    {
        let _ = ms;
        err(proc, "sleep: no timer in this profile")
    }
}

/// ALARM(6): millisecs (init's alarm(0)
/// disarm at main start).  Donor sysalarm (9k sysproc.c) + procalarm
/// (9k alarm.c): arm a per-proc alarm whose expiry posts the "alarm" note
/// to the proc (tick_timeouts's expiry arm; delivery rides the
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

/// The %q quoting of a wait-record status (libc fmtquote's rule): quote
/// with SINGLE quotes when the string is empty or carries a char ≤ ' '
/// (whitespace/control) or a quote; internal quotes double.  Otherwise
/// bare.  libc wait()'s tokenize (qtoken) strips exactly this shape.
fn quote_status(msg: &str) -> alloc::string::String {
    let needs_quote = msg.is_empty() || msg.bytes().any(|b| b <= b' ' || b == b'\'');
    if !needs_quote {
        return msg.into();
    }
    let mut out = alloc::string::String::with_capacity(msg.len() + 2);
    out.push('\'');
    for c in msg.chars() {
        if c == '\'' {
            out.push('\'');
        }
        out.push(c);
    }
    out.push('\'');
    out
}

/// AWAIT(47): buf, buflen.  Donor sysawait shape: block (the
/// swtch-based sched path) until a child's exit record exists, then write
/// the Plan-9-shaped wait message `{pid} 0 0 0 {status}` (utime/stime/
/// realtime zeros this era; the status is %q-quoted — SINGLE quotes,
/// only when needed — because libc wait()'s tokenize strips exactly
/// that shape; anything else arrives as literal bytes and rc's status
/// branches invert) NUL-terminated into
/// the validated user buffer and return the record length.  No children
/// ever → loud -1 errstr (donor pwait's "no children").
fn sys_await(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "await: bad argument block");
    };
    let (buf, buflen) = (args[0], args[1] & 0xffff_ffff); // buflen is a uint (width discipline)
    // The await buffer's validation + residency
    // (the cold-buffer class) are fused in the constructor.
    let mut dst = match UserSliceMut::new(proc, buf, buflen) {
        Ok(b) => b,
        Err(_) => return err(proc, "await: bad user buffer"),
    };
    let cap = buflen as usize;
    if cap == 0 {
        return err(proc, "await: zero-length buffer");
    }
    loop {
        if let Some(rec) = proc.wait_records.pop_front() {
            proc.children_live = proc.children_live.saturating_sub(1);
            let msg = core::str::from_utf8(&rec.msg[..rec.msg_len]).unwrap_or("?");
            // The status field is %q-quoted (the donor's waitmsg shape —
            // SINGLE quotes, only when the status is empty or carries a
            // whitespace/control/quote char, internal quotes doubled).
            // libc's wait() tokenize strips single quotes; a double-quoted
            // (or always-quoted) status arrives as literal quote bytes and
            // rc's `||`/`&&` read a successful child as FAILED (07-07's
            // net-sweep bind gate exposed this).
            let qmsg = quote_status(msg);
            let text = alloc::format!("{} 0 0 0 {}", rec.pid, qmsg);
            let bytes = text.as_bytes();
            let n = bytes.len().min(cap - 1);
            let dst = dst.as_slice_mut();
            dst[..n].copy_from_slice(&bytes[..n]);
            dst[n] = 0;
            return n as u64;
        }
        if proc.children_live == 0 {
            return err(proc, "await: no children");
        }
        // Block mid-syscall: mark Blocked + swtch to the scheduler
        // context; resumes here when a child's EXITS posts a record and
        // wakes us.  The trap frame on this proc's own kstack survives
        // other processes' traps meanwhile.
        proc::sched();
    }
}

/// NSEC(53): no arguments (the libc stub passes none — there is no user
/// memory to validate).  Return the epoch-nanosecond clock.  9k's
/// sysnsec is todget(nil): time-of-day ns, EPOCH-based — libc's time()
/// is nsec()/1e9, so the value must name the real epoch (the tier-1
/// sweep's date check keys on a 20xx year).  The composition lives in
/// platform::rtc (the PL031 epoch-seconds read anchored against the
/// generic timer's monotonic count); a profile with no clock errors
/// loudly.  Measured into the dispatch set by the 06-07 tier-1 trace
/// (hdate, hipconfig — hmk/htouch/hcpp ride the donor-only guard).
fn sys_nsec(_frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    #[cfg(dev_exec)]
    {
        match crate::platform::rtc::nsec_epoch() {
            Some(ns) => ns,
            None => err(proc, "nsec: no RTC"),
        }
    }
    #[cfg(not(dev_exec))]
    {
        err(proc, "nsec: no clock in this profile")
    }
}

/// RENDEZVOUS(34): tag, value — the user rendezvous (9k sysrendezvous,
/// sysproc.c:853).  The proc's rendezvous group (rfork's RFREND arm;
/// libthread's procs share one) is searched for a sleeper on `tag`.
/// Found: the sleeper is handed OUR value and readied, and we return the
/// SLEEPER's stored value — the symmetric first-sleeps exchange.
/// Absent: we record (tag, value, pid) and block mid-syscall (the sched()
/// shape — the group lock drops BEFORE the block, and register-then-
/// block is atomic against wakers because syscalls serialize).  On wake
/// we return the value the waker left in proc.rendval.  Both args are
/// full-width pointers (no masking).  Measured ×1 by hfactotum in the
/// 06-07 tier-1 trace (libthread's start sync).
fn sys_rendezvous(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "rendezvous: bad argument block");
    };
    let (tag, value) = (args[0], args[1]);
    let rgrp = proc.rgrp.clone();
    let sleeper = {
        let node = LockNode::new();
        let mut g = rgrp.lock(&node);
        match g.take_sleeper(tag) {
            Some(s) => Some(s),
            None => {
                g.park(tag, value, proc.pid);
                None
            }
        }
    };
    match sleeper {
        Some(s) => {
            // The peer was parked mid-syscall: hand it our value and
            // ready it; our return is its stored offer.
            proc::rendez_wake(s.pid, value);
            s.value
        }
        None => {
            // The first caller sleeps until a peer takes the entry.
            // The proc::sched() wake is pid-keyed and FLAT
            // (Blocked→Ready), not channel-keyed like the donor's
            // Rendez — a STALE waiter-set registration (e.g. a devip
            // net-waiter entry left behind by an already-completed
            // listen/read, drained by a later tick/RX sweep) readies a
            // parked proc with NO peer pairing (the 08-08 aan gate: the
            // server main's listen-open registration survived into its
            // libthread runthread park, and the spurious ready made
            // runthread read a nil q->head — a wild-EL0-pointer kill).
            // Every other blocking primitive re-checks its condition
            // after sched() (semacquire/await/sleep/devpipe/devip all
            // do) — rendezvous does it here: our entry still present =
            // the wake was spurious (re-block; the entry stays for the
            // real peer); entry gone = a peer's take_sleeper consumed
            // it in the same serialized syscall window that readied us
            // (rendval is valid).
            proc::sched();
            loop {
                let paired = {
                    let node = LockNode::new();
                    !rgrp.lock(&node).is_parked(proc.pid)
                };
                if paired {
                    break;
                }
                proc::sched();
            }
            proc.rendval
        }
    }
}
/// EXITS(8): copy the optional message (nil = empty), free the proc's user
/// pages + root-table page + kstack via the mapping_log / recorded PAs,
/// mark Moribund, print the exit line, post the wait record to the parent
/// and wake an AWAITing parent, then hand off to the scheduler;
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

/// The shared exit tail: sys_exits's user-message path and the
/// note machinery's kernel-message kills (deliver_notes's no-handler kill,
/// NOTED's NDFLT) converge here.  Frees the user mappings, marks Moribund,
/// prints the exit line, posts + wakes the parent, frees the root-table
/// page and the kstack LAST (the handler is still running on it — safe
/// because the swtch save writes only the PROC_TABLE-resident context),
/// then the all-exited halt or the sched-loop hand-off.  Diverges.
///
/// The FIRST step is the fd-group sweep (the donor pexit's closefgrp).
/// The proc's group is DETACHED (replaced by a fresh
/// empty Arc — the Moribund proc holds the empty one), and only when the
/// detached Arc's count is 1 (this proc was the group's LAST sharer) are
/// its occupied slots drained and close-dispatched (namec::close_chan —
/// devpipe's open-count decrement, devmnt's clunk_if_last).  A fork child
/// SHARING its parent's group (RFPROC without RFFDG/RFCFDG) never
/// triggers the sweep (count > 1 — the sharers keep the group).
///
/// Ordering: the sweep runs BEFORE free_user_mappings because close_chan
/// may RPC-block for DEV_MNT (the Tclunk's pipe read schedules the server
/// proc — the sys_mount blocking precedent) and an RPC needs the address
/// space live.  The drain runs under the group lock; the close dispatch
/// runs AFTER the lock drops (never under it — the fdgrp leaf-lock rule;
/// a close can block).
///
/// Structural exclusions (BY DESIGN, never swept): devsrv's registered
/// clone is registry-held (an Arc ref outside any fd slot — it survives
/// exits so later boots mounts of #s/boot keep working), and the parked
/// paqfs io child never exits (a server self-Tclunking during its own
/// exit would deadlock its own RPC — impossible this era).
pub(crate) fn exit_tail(proc: &mut Proc, msg: &[u8]) -> ! {
    // Detach + last-sharer sweep (see the doc comment).  The replacement
    // is the SHARED empty group (proc.rs's empty_fdgroup): a fresh 32-slot
    // table per exit was ~6 KB of heap retained per husk, and the tier-1
    // sweep's ~200 fork+exit cycles measured the arena wall — a dead proc
    // never touches its group, so the sharing is never observed.
    let detached = core::mem::replace(&mut proc.fd_group, proc::empty_fdgroup());
    if Arc::strong_count(&detached) == 1 {
        let drained = {
            let node = LockNode::new();
            detached.lock(&node).drain_all()
        };
        for c in &drained {
            port::namec::close_chan(c);
        }
    }
    // count > 1: sharers keep the group; the detached Arc just drops.
    // A parked rendezvous entry never outlives its proc (9k's pexit
    // removes the proc from the rendezvous hash the same way).
    proc::rendez_remove(proc);
    proc.free_user_mappings();
    // Drop the husk's remaining heap payloads (the slot itself is never
    // reaped this era — its Vec/Arc payloads must not ride along):
    // mapping_log/segments/text_source went with free_user_mappings; the
    // exec-recorded argv and the wait-record/note queues go here.
    proc.argv = Vec::new();
    proc.wait_records = VecDeque::new();
    proc.note_queue = VecDeque::new();
    proc.state = ProcState::Moribund;
    let pid = proc.pid;
    println!("process {pid} exited: \"{}\"", core::str::from_utf8(msg).unwrap_or("<bad utf8>"));
    // Post the wait record to the parent and wake it BEFORE the
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
        // Clean halt: park in wfi.  IRQs stay masked (trap
        // context never unmasks them), so this is a wedge-free stop.
        trap::park_forever();
    }
    // The sched loop picks the next Ready proc (the
    // woken parent, or a sibling).  Diverges.
    proc::sched_from_moribund();
}
