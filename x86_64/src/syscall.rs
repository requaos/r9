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

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

use port::dat::{
    Chan, DEV_CONS, DEV_DUP, DEV_ENV, DEV_ETHER, DEV_IP, DEV_MNT, DEV_PIPE, DEV_PROC, DEV_ROOT,
    DEV_SRV, OEXEC, OTRUNC,
};
use port::mcslock::{Lock, LockNode};
use port::namec::{self, NamecMode};
use port::pgrp::{MAFTER, MBEFORE, MMASK, MORDER};
use port::println;

use crate::dat::Ureg;
use crate::exec;
use crate::proc::{self, ByteBacking, Proc, ProcState, SegKind, ERRMAX};
use crate::usermem::{UserPtr, UserSlice, UserSliceMut, UserString};

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

// rfork flags live in proc.rs (the process machinery owns them).
use proc::{RFCFDG, RFCENVG, RFCNAMEG, RFENVG, RFMEM, RFNAMEG, RFNOTEG, RFNOWAIT, RFFDG, RFPROC, RFREND};

/// Plan 9 note actions (Harvey sys/include/libc.h — frozen values).
const NCONT: u64 = 0;
const NDFLT: u64 = 1;
const NSAVE: u64 = 2;
const NRSTR: u64 = 3;

/// The frozen sizeof(Ureg) on amd64: 184 — and on amd64 the kernel
/// frame IS the ABI frame (no aarch64-style 280-vs-288 stride
/// mismatch), so the note machinery copies the whole frame.
const UREG_SIZE: usize = core::mem::size_of::<Ureg>();

/// Handler shape: the live Ureg (arg source + result sink) and the
/// calling proc.
type Handler = fn(&mut Ureg, &mut Proc) -> u64;

/// The dispatch table — Harvey sys.h 0-54 verbatim as Option<fn>
/// entries.  None -> loud unimplemented at dispatch time.  The
/// implemented set is the measured boot-chain set (the aarch64
/// membership carried verbatim — the same shared sys.h table serves
/// every arch): the fd pair PREAD/PWRITE over the port devices, the
/// namespace family OPEN/BIND/CHDIR/UNMOUNT/STAT/FSTAT/MOUNT/FVERSION,
/// the process family EXEC/RFORK/AWAIT/EXITS/NOTIFY/NOTED, the
/// connectembed family DUP/PIPE/CREATE/FD2PATH, the memory pair
/// BRK_/SEMACQUIRE/SEMRELEASE, the timer pair ALARM/SLEEP, plus
/// SEEK/ERRSTR/RENDEZVOUS/NSEC.  REMOVE/WSTAT/FWSTAT/OSEEK/FAUTH stay
/// loud-None (the measured evidence ratified the loud gap, not a
/// silent one).  The kernel's syscall attack surface is exactly the
/// measured set.
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

/// The dispatch: number from frame.bp, handler result to frame.ax (the
/// restore tail's `movq 0*8(%rsp), %rax` propagates it to user AX).
pub fn dispatch(frame: &mut Ureg) {
    let Some(mut p) = proc::current() else {
        // A SYSCALL with no current process is a kernel-orchestration
        // bug (the exec bootstrap execs before any ring 3 exists),
        // never a user error.
        panic!("syscall dispatch with no current process (n={})", frame.bp);
    };
    dispatch_proc(frame.bp, frame, &mut p);
}

/// The entry the 14-02 asm calls (trap.rs's `syscall` stub body):
/// dispatch, then the note-delivery boundary — every return to ring 3
/// is the delivery point.
pub fn entry(_number: u64, frame: &mut Ureg) {
    dispatch(frame);
    deliver_notes(frame);
}

/// The dispatch core (everything `dispatch` does after the
/// current-proc fetch — the module's host-test seam: tests substitute
/// a table-resident proc for the global).
pub(crate) fn dispatch_proc(n: u64, frame: &mut Ureg, proc: &mut Proc) {
    let result = match DISPATCH.get(n as usize) {
        Some(Some(handler)) => handler(frame, proc),
        _ => {
            println!("syscall {n} unimplemented");
            proc.set_errstr("syscall not implemented");
            -1i64 as u64
        }
    };
    // NOTED's success path rewrote the live frame (the restored Ureg IS
    // the result) — the donor dosyscall special case: dispatch returns
    // WITHOUT the frame.ax writeback.  NOTED's ERROR returns (-1) left
    // the frame untouched and DO write back; NDFLT diverged inside the
    // handler.
    if n == NOTED && result != -1i64 as u64 {
        return;
    }
    // EXITS never reaches here (it parks divergent); every other
    // handler's result rides frame.ax back to user AX.
    frame.ax = result;
}

/// Read the argument slots from user memory at SP+8 (the frozen ABI
/// formula: the stub's spilled arg0 lands at SP+8; SP+0 is the
/// return-address slot).  THE STACK RANGE AT THE CAPTURED SP IS
/// VALIDATED FIRST — the 9k donor's order (syscall.c:248-251): the
/// window [sp, sp+8+8*arity) covers the return-address slot through
/// the arity slots, validated WITHOUT forcing residency (the arg block
/// lives on the live user stack, which the stub wrote through to make
/// the call — resident by construction).  Only then are the slots read
/// upward from SP+8.
///
/// Width discipline (donor-faithful, carried from the aarch64 port):
/// every slot is 8 bytes, but int/uint-width arguments are stored by
/// 6c with a 32-bit store that leaves the slot's upper half as stack
/// garbage — handlers must likewise mask int-width args to 32 bits
/// (await's buflen, errstr's n, rfork's flag, the fd args, open's
/// mode).  Pointer/vlong args occupy the full slot.  **`long` is
/// 32-bit in Plan 9 C on every arch (6c SZ_LONG=4; only `vlong` is
/// 64-bit) — pread/pwrite's `long n` is a 32-bit arg on the wire and
/// must be masked too.**  (Arg0 is always clean: the stub materializes
/// it from BP, whose upper half the caller's 32-bit op already
/// zeroed.)
fn sargs(frame: &mut Ureg, proc: &mut Proc, arity: usize) -> Option<[u64; 5]> {
    let base = frame.sp;
    let block = UserSlice::validated_only(proc, base, 8 + 8 * arity as u64).ok()?;
    let mut out = [0u64; 5];
    for (i, slot) in out.iter_mut().enumerate().take(arity) {
        *slot = block.read_u64(8 + 8 * i)?;
    }
    Some(out)
}

/// Copy a NUL-terminated user string into a kernel Vec (the usermem
/// loop's call-site wrapper).
fn copy_user_str(proc: &Proc, va: u64, max: usize) -> Result<Vec<u8>, &'static str> {
    UserString::copy_from(proc, va, max)
}

/// Error return: set the proc's errstr, return -1 on the wire.
fn err(proc: &mut Proc, msg: &'static str) -> u64 {
    proc.set_errstr(msg);
    -1i64 as u64
}

/// EXEC's argv staging: the path string + the NUL-terminated argv
/// block out of user memory, everything COPIED to kernel memory before
/// exec_replace runs (the replacement frees the old image's pages,
/// which is where the arguments lived).  Order-stable by construction
/// (the walk ascends the argv array; each string copies in turn).
pub(crate) fn copy_exec_args(
    proc: &Proc,
    path_va: u64,
    argv_va: u64,
) -> Result<(Vec<u8>, Vec<Vec<u8>>), &'static str> {
    const MAX_EXEC_ARGS: usize = 32;
    const MAX_EXEC_ARG_LEN: usize = 128;
    let path = UserString::copy_from(proc, path_va, ERRMAX)?;
    let mut argv: Vec<Vec<u8>> = Vec::new();
    if argv_va != 0 {
        loop {
            if argv.len() >= MAX_EXEC_ARGS {
                return Err("exec: too many args");
            }
            let slot_va = argv_va + 8 * argv.len() as u64;
            // The argv slot block rides usermem's validated-only
            // pointer — the slots are user stack/data the process
            // itself wrote to set up the call (resident by
            // construction).
            let slot = UserPtr::validated_only(proc, slot_va)?;
            let ptr = slot.read_u64();
            if ptr == 0 {
                break;
            }
            argv.push(UserString::copy_from(proc, ptr, MAX_EXEC_ARG_LEN)?);
        }
    }
    Ok((path, argv))
}

/// PWRITE(51): fd, buf, n, offset.  Offset -1 (u64::MAX) is the
/// streaming channel offset (Harvey libc write() = pwrite(fd, buf, n,
/// -1LL)); the console ignores offsets, devroot is read-only, devenv
/// honors them, devpipe ignores them.  n is a `long` — 32-bit in
/// Plan 9 C, so the wire slot's upper half is 6c garbage and must be
/// masked (the width discipline above).
///
/// The fd is held by a transient OP-REFERENCE (fd_op_ref/fd_op_done):
/// the slot keeps its chan across a blocking write, so a shared-group
/// sibling's concurrent op on the same fd never sees an emptied slot.
fn sys_pwrite(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "pwrite: bad argument block");
    };
    let (fd, buf, n, off) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff, args[3]); // fd/n are ints
    let src = match UserSlice::new(proc, buf, n) {
        Ok(s) => s,
        Err(_) => return err(proc, "pwrite: bad user buffer"),
    };
    let Some(mut chan) = proc.fd_op_ref(fd) else {
        return err(proc, "pwrite: bad fd");
    };
    let result = match chan.dev {
        DEV_CONS => port::devcons::write(&mut chan, src.as_slice()),
        DEV_ROOT => Err("devroot: read-only filesystem"),
        DEV_PROC => port::devproc::write(&mut chan, src.as_slice()),
        DEV_ENV => port::devenv::write(&mut chan, src.as_slice(), off),
        DEV_PIPE => port::devpipe::write(&mut chan, src.as_slice(), off),
        DEV_DUP => Err("devdup: not writable (open #d/N to reach the fd)"),
        DEV_SRV => port::devsrv::write(&mut chan, src.as_slice(), off),
        DEV_ETHER => port::devether::write(&mut chan, src.as_slice(), off),
        DEV_IP => {
            port::devip::stack::drain_debt_if_set();
            port::devip::write(&mut chan, src.as_slice(), off)
        }
        DEV_MNT => port::devmnt::write(&mut chan, src.as_slice(), off),
        _ => Err("pwrite: unknown device"),
    };
    proc.fd_op_done(fd, chan);
    match result {
        Ok(wrote) => wrote,
        Err(msg) => err(proc, msg),
    }
}

/// PREAD(50): fd, buf, n, offset — the device dispatch over the
/// op-referenced fd.  n is a `long` (32-bit — the width discipline).
fn sys_pread(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "pread: bad argument block");
    };
    let (fd, buf, n, off) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff, args[3]); // fd/n are ints
    let mut dst = match UserSliceMut::new(proc, buf, n) {
        Ok(s) => s,
        Err(_) => return err(proc, "pread: bad user buffer"),
    };
    let Some(mut chan) = proc.fd_op_ref(fd) else {
        return err(proc, "pread: bad fd");
    };
    let result = match chan.dev {
        DEV_CONS => port::devcons::read(&mut chan, dst.as_slice_mut(), off),
        DEV_ROOT => {
            let Some(root) = port::devroot::root() else {
                proc.fd_op_done(fd, chan);
                return err(proc, "devroot: not mounted");
            };
            root.read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_PROC => port::devproc::read(&mut chan, dst.as_slice_mut(), off),
        DEV_ENV => port::devenv::read(&mut chan, dst.as_slice_mut(), off),
        DEV_PIPE => port::devpipe::read(&mut chan, dst.as_slice_mut(), off),
        DEV_DUP => port::devdup::read(&mut chan, dst.as_slice_mut(), off),
        DEV_SRV => port::devsrv::read(&mut chan, dst.as_slice_mut(), off),
        DEV_ETHER => port::devether::read(&mut chan, dst.as_slice_mut(), off),
        DEV_IP => {
            port::devip::stack::drain_debt_if_set();
            port::devip::read(&mut chan, dst.as_slice_mut(), off)
        }
        DEV_MNT => port::devmnt::read(&mut chan, dst.as_slice_mut(), off),
        _ => Err("pread: unknown device"),
    };
    proc.fd_op_done(fd, chan);
    match result {
        Ok(read) => read,
        Err(msg) => err(proc, msg),
    }
}

/// NOTIFY(28): handler VA (a full-width pointer arg).  nil clears the
/// handler; otherwise the VA must land inside the proc's OWN text
/// segment (a process can never install a handler pointing outside its
/// image — the ring-3 handler redirect trusts only validated
/// segment-internal VAs).  Stored on the proc; the note-delivery
/// boundary redirects into it.
fn sys_notify(frame: &mut Ureg, proc: &mut Proc) -> u64 {
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

/// NOTED(29): action (int-width arg — width discipline).  NCONT(0)/
/// NRSTR(3) restore the saved Ureg over the live frame in place; the
/// success return takes dispatch's no-writeback special path.  NDFLT(1)
/// exits with the delivered note message.  NSAVE(2) is
/// loud-unimplemented.
fn sys_noted(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "noted: bad argument block");
    };
    let action = args[0] & 0xffff_ffff; // noted's action is an int
    match action {
        NCONT | NRSTR => {
            // The saved ureg is kernel-tracked (the donor's
            // up->noteureg): libc's noted(action) passes NO pointer.
            let nureg = proc.note_ureg;
            if nureg == 0 {
                return err(proc, "noted: not notified");
            }
            // The saved-ureg window rides the usermem validated-only
            // variant — delivery forced its pages resident and nothing
            // unmaps them while the handler runs.
            let saved = match UserSlice::validated_only(proc, nureg, UREG_SIZE as u64) {
                Ok(s) => s,
                Err(_) => return err(proc, "noted: bad saved ureg"),
            };
            // Validate the resume target BEFORE committing the restore
            // (the donor noted()'s okaddr checks, pre-copy so a bad
            // saved frame leaves the live frame untouched and the -1
            // writeback is truthful).  Saved-Ureg byte offsets: sp@168,
            // pc@144 (the frozen table).
            let saved_sp = saved.read_u64(168).expect("ureg->sp in range");
            let saved_pc = saved.read_u64(144).expect("ureg->pc in range");
            let in_text = proc
                .segments
                .as_ref()
                .is_some_and(|s| s.text.0.contains(&(saved_pc as usize)));
            if saved_sp & 7 != 0 || !in_text {
                return err(proc, "noted: bad resume frame");
            }
            if proc.validate_user_range(saved_sp, 8, true).is_err() {
                return err(proc, "noted: bad resume stack");
            }
            // Restore: the saved 184 bytes over the live frame (the
            // usermem slice is the source; the frame-side byte copy
            // lives on the frame's own module).
            frame.restore_from_saved(saved.as_slice());
            // The restored cs/ss/flags came from user-writable memory
            // (the notejmp contract lets the handler edit the saved
            // Ureg) — force the user return state (the flat selectors,
            // IOPL cleared, IF set) so a handler-edited frame can never
            // steer SYSRETQ's RFLAGS load (Ureg::force_user_return's
            // doc).  The saved `type` field stays
            // informational-only (ABI.md documents it as such).
            frame.force_user_return();
            // Allow the next note (the donor's up->notified = 0).
            proc.note_ureg = 0;
            0
        }
        NDFLT => {
            // Exit with the delivered note's message (the donor pexit
            // shape).  Diverges.
            let msg = proc.last_note;
            let len = proc.last_note_len;
            exit_tail(proc, &msg[..len]);
        }
        NSAVE => {
            println!("noted: NSAVE unimplemented");
            err(proc, "noted: NSAVE unimplemented")
        }
        _ => err(proc, "noted: bad action"),
    }
}

/// OPEN(14): path, mode.  EVERY path resolves through the namespace
/// core — namec + the proc's mount table — and the resolved chan opens
/// via the per-device dispatch.  Installed in the lowest free fd slot.
fn sys_open(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "open: bad argument block");
    };
    let (path_va, mode) = (args[0], (args[1] & 0xffff_ffff) as u32); // mode is an int
    let path = match copy_user_str(proc, path_va, ERRMAX) {
        Ok(path) => path,
        Err(msg) => return err(proc, msg),
    };

    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "open: no namespace (pre-exec)");
    };
    // NO pgrp lock is held across resolution or the open — namec
    // consults the mount table under per-consult snapshots, and
    // open_chan's Topen RPC blocks mid-syscall.
    let c = match namec::namec(&path, slash, dot, &proc.pgrp, NamecMode::Open) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let opened = match namec::open_chan(&c, mode) {
        Ok(c) => c,
        Err(msg) => {
            // The resolved chan never reaches an fd on the error exit —
            // close-dispatch it.
            port::namec::close_chan(&c);
            return err(proc, msg);
        }
    };

    match proc.fd_install_lowest(opened) {
        Some(slot) => slot,
        None => err(proc, "open: no free fd"),
    }
}

/// BIND(2): name, old, flag.  The source resolves with the union-head
/// redirect; the target NEVER redirects (Amount — stacked binds attach
/// to the original directory) and must EXIST.
fn sys_bind(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "bind: bad argument block");
    };
    let (name_va, old_va, flag) = (args[0], args[1], (args[2] & 0xffff_ffff) as u32); // flag is an int
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
    // The resolved TARGET chan is never installed — close-dispatch it.
    port::namec::close_chan(&tgt);
    match result {
        Ok(_) => 0,
        Err(msg) => err(proc, msg),
    }
}

/// CHDIR(3): path.  Atodir semantics (NO mount-point redirect), the
/// target must be a directory, then up->dot = c.
fn sys_chdir(frame: &mut Ureg, proc: &mut Proc) -> u64 {
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
    let c = match namec::namec(&path, slash, dot, &proc.pgrp, NamecMode::ToDir) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    if c.qid.typ & port::dat::QTDIR == 0 {
        port::namec::close_chan(&c);
        return err(proc, "chdir: not a directory");
    }
    if let Some(old) = proc.dot.replace(c) {
        port::namec::close_chan(&old);
    }
    0
}

/// UNMOUNT(35): name, old.  The mount point resolves WITHOUT the
/// redirect; old == nil removes the whole mount point, otherwise only
/// the chain entry whose mounted chan matches old's identity.
fn sys_unmount(frame: &mut Ureg, proc: &mut Proc) -> u64 {
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
    port::namec::close_chan(&tgt);
    if let Some(f) = &from {
        port::namec::close_chan(f);
    }
    match result {
        Ok(()) => 0,
        Err(msg) => err(proc, msg),
    }
}

/// FVERSION(40): fd, msize, version, nversion.  Negotiate the 9P
/// session over the fd's chan and write back the negotiated version
/// string, returning its length.
fn sys_fversion(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "fversion: bad argument block");
    };
    let (fd, msize, ver_va, nver) =
        (args[0] & 0xffff_ffff, (args[1] & 0xffff_ffff) as u32, args[2], args[3] & 0xffff_ffff);
    if nver == 0 {
        return err(proc, "fversion: zero-length version buffer");
    }
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

/// MOUNT(46): fd, afd, old, flag, aname.  Validate the flags, the fd
/// must be a 9P connection (auto-version when fversion never ran),
/// Tattach with afid = NOFID (afd != -1 is loud-unsupported — the
/// no-auth era), then the mount-table insertion at the resolved target.
fn sys_mount(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 5) else {
        return err(proc, "mount: bad argument block");
    };
    let (fd, afd, old_va, flag, aname_va) = (
        args[0] & 0xffff_ffff,
        (args[1] & 0xffff_ffff) as u32 as i32,
        args[2],
        (args[3] & 0xffff_ffff) as u32,
        args[4],
    );
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

    // Tattach (blocks mid-syscall — NO pgrp lock is held across the
    // RPC).
    let mut mnt_chan = match port::devmnt::attach(&mnt, &aname) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };

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
    port::namec::close_chan(&tgt);
    match result {
        Ok(_) => 0,
        Err(msg) => err(proc, msg),
    }
}

/// STAT(42): path, buf, n.  Aaccess resolution, then the device's stat
/// op writes the stat(5) wire bytes; the return is the byte count.
fn sys_stat(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "stat: bad argument block");
    };
    let (path_va, buf_va, n) = (args[0], args[1], args[2] & 0xffff_ffff); // n is an int
    let path = match copy_user_str(proc, path_va, ERRMAX) {
        Ok(s) => s,
        Err(msg) => return err(proc, msg),
    };
    let mut sb = match UserSliceMut::new(proc, buf_va, n) {
        Ok(b) => b,
        Err(_) => return err(proc, "stat: bad user buffer"),
    };
    let (Some(slash), Some(dot)) = (proc.slash.as_ref(), proc.dot.as_ref()) else {
        return err(proc, "stat: no namespace (pre-exec)");
    };
    let c = match namec::namec(&path, slash, dot, &proc.pgrp, NamecMode::Open) {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let result = namec::stat_chan(&c, sb.as_slice_mut());
    port::namec::close_chan(&c);
    match result {
        Ok(count) => count,
        Err(msg) => err(proc, msg),
    }
}

/// FSTAT(43): fd, buf, n — the open fd's chan stats through the same
/// device dispatch.
fn sys_fstat(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "fstat: bad argument block");
    };
    let (fd, buf_va, n) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff); // fd, n are ints
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

/// BRK_(24): addr (malloc's pool growth rides it).  Donor sysbrk_:
/// set the bss segment's top; libc's sbrk needs 0/-1.  Growth pages
/// demand-fault as zero pages (the Bss class covers the extended
/// range); a shrink only lowers the recorded break.  The break never
/// crosses USTOP.
fn sys_brk_(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "brk: bad argument block");
    };
    let addr = args[0] as usize; // a full-width pointer arg (no masking)
    if proc.segments.is_none() {
        return err(proc, "brk: no segments (pre-exec)");
    }
    if addr == 0 {
        return 0; // no-op (defensive)
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
        // The ordinary private shape: move the caller's break and map
        // the growth pages NOW — the 14-06 x86_64 deviation from
        // aarch64's lazy fault-in (ensure_user_resident pages them
        // from syscall context there): this era has no trap-context
        // pager, so a lazily-grown page's first touch fault-kills the
        // proc (boot(8)'s first malloc died exactly there — the chain
        // gate surfaced it).  Same policy as map_text_eager/
        // map_data_bss_eager: the mapping is eager, the demand pager
        // is the follow-up.
        None => {
            let page = port::mem::PAGE_SIZE_4K;
            let grown_start = exec::round_up_pow2(old_end, page);
            let grown_end = exec::round_up_pow2(addr, page);
            let mut va = grown_start;
            while va < grown_end {
                if proc.map_user_page(va, SegKind::Bss).is_err() {
                    return err(proc, "brk: growth page map failed");
                }
                va += page;
            }
            proc.segments.as_mut().unwrap().bss.0.end = addr;
            0
        }
        // The RFMEM family shape: every member's break moves together
        // and growth pages are born shared.
        Some(family) => match proc::rfmem_brk_set(family, old_end, addr) {
            Ok(()) => 0,
            Err(msg) => err(proc, msg),
        },
    }
}

/// Read the semaphore word (a user `int` — 4 bytes, little-endian).
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
/// mapping_log — the wait-queue key is the word's PHYSICAL address.
fn sem_word_pa(proc: &Proc, va: u64) -> Option<u64> {
    let va = va as usize;
    proc.mapping_log
        .iter()
        .find(|m| m.va <= va && va < m.va + m.size)
        .map(|m| m.phys as u64 + (va - m.va) as u64)
}

/// SEMACQUIRE(37): addr, block.  Fast path: decrement a positive word
/// and return 1.  Contended path (block != 0): register the caller on
/// the word's PA BEFORE EACH block and block in sched(), re-checking
/// on wake.  Returns 1 on acquire, 0 when block == 0 and the word is
/// exhausted.
fn sys_semacquire(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "semacquire: bad argument block");
    };
    let addr = args[0]; // a full-width pointer arg (no masking)
    let block = args[1] & 0xffff_ffff; // block is an int
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
    if sem_try_acquire(&mut word) {
        return 1;
    }
    if block == 0 {
        return 0;
    }
    let Some(pa) = sem_word_pa(proc, addr) else {
        return err(proc, "bad arg in system call");
    };
    loop {
        // Register BEFORE every block: sem_wake CONSUMES registrations
        // when it picks its wakers, so a woken waiter that loses the
        // unit race must RE-register before re-blocking.
        proc::sem_register_waiter(pa, proc.pid);
        if sem_try_acquire(&mut word) {
            break;
        }
        proc::sched();
    }
    proc::sem_deregister_waiter(pa, proc.pid);
    1
}

/// SEMRELEASE(38): addr, delta.  Add `delta` to the word, then wake up
/// to `delta` waiters registered on the word's PA.  Returns the new
/// value.
fn sys_semrelease(frame: &mut Ureg, proc: &mut Proc) -> u64 {
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

/// EXEC(7): path, argv — 2 syscall args.  Donor semantics: returns
/// ONLY on failure (-1/errstr, old image intact); on success
/// exec::exec_replace rewrites the live Ureg and the dispatch return
/// SYSRETQs into the new image.
fn sys_exec(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    /// The whole-file staging ceiling for a MOUNTED binary — bounded
    /// so an untrusted server can never stream the kernel into an
    /// unbounded allocation.
    const EXEC_STAGE_CAP: usize = 16 * 1024 * 1024;
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "exec: bad argument block");
    };
    let (path_va, argv_va) = (args[0], args[1]);
    let (path, argv) = match copy_exec_args(proc, path_va, argv_va) {
        Ok(v) => v,
        Err(msg) => return err(proc, msg),
    };

    // Resolve through the NAMESPACE — namec + the proc's mount table —
    // so an exec path crosses mounts.  NO pgrp lock is held across
    // resolution, open, or stage.
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
            port::namec::close_chan(&c);
            return err(proc, msg);
        }
    };
    // The walked chan is walked-past once the open SHARES its fid's
    // Arc — close-dispatch it now so the fid's last ref Tclunks exactly
    // once.
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
            // A mounted binary is read WHOLE here — syscall context,
            // where the 9P client's blocking is by design.
            match port::devmnt::read_all(&opened, EXEC_STAGE_CAP) {
                Ok(bytes) => ByteBacking::Mnt { file: Arc::new(bytes) },
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
    // The exec chan is a transient (never fd-installed):
    // close-dispatch it.
    port::namec::close_chan(&opened);
    let leaf = path.split(|&b| b == b'/').rfind(|c| !c.is_empty()).unwrap_or(&path[..]);
    let name = core::str::from_utf8(leaf).unwrap_or("?");
    let arg_slices: Vec<&[u8]> = argv.iter().map(|a| a.as_slice()).collect();
    match exec::exec_replace(proc, backing, name, &arg_slices, frame) {
        // dispatch writes frame.ax = tos_va — alongside the seeded bp
        // (the RARG slot carries the same Tos by seed_exec_entry).
        Ok(tos_va) => tos_va,
        Err(msg) => err(proc, msg),
    }
}

/// CLOSE(4): drop the fd slot.
fn sys_close(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "close: bad argument block");
    };
    let fd = args[0] & 0xffff_ffff; // fd is an int
    match proc.fd_take(fd) {
        Some(c) => {
            // The close dispatch — a pipe end's close decrements the
            // open count and the last one hangs the peer's read queue
            // up (the EOF path).
            port::namec::close_chan(&c);
            0
        }
        None => err(proc, "close: bad fd"),
    }
}

/// PIPE(21): fd-array-ptr.  Attach a FRESH pipe, walk data/data1, open
/// both ORDWR, install the pair at the two lowest free slots, and
/// write the two fds back through the validated user array (two 32-bit
/// ints — the width discipline).
fn sys_pipe(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "pipe: bad argument block");
    };
    let arr_va = args[0];
    let mut arr = match UserSliceMut::new(proc, arr_va, 8) {
        Ok(a) => a,
        Err(_) => return err(proc, "pipe: bad fd array"),
    };
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
            // Balance e0's open before failing.
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

/// DUP(5): oldfd, newfd.  The old fd's chan is ref-cloned; newfd == -1
/// installs at the lowest free slot; an occupied newfd is EVICTED and
/// close-dispatched AFTER the install.  Returns newfd.
fn sys_dup(frame: &mut Ureg, proc: &mut Proc) -> u64 {
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

/// CREATE(22): path, mode, perm.  Resolve the PARENT (open-style union
/// redirect), then if the final component EXISTS open it OTRUNC
/// (unless OEXCL), else dispatch the device's create op.  DMDIR rides
/// the perm word to the device (mkdir through a mount — keyfs's user
/// creates — is the measured caller, 15-03); a device that cannot make
/// directories errors its own loud error.
fn sys_create(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "create: bad argument block");
    };
    let (path_va, mode, perm) = (
        args[0],
        (args[1] & 0xffff_ffff) as u32,
        (args[2] & 0xffff_ffff) as u32,
    ); // mode/perm are ints

    let path = match copy_user_str(proc, path_va, ERRMAX) {
        Ok(path) => path,
        Err(msg) => return err(proc, msg),
    };
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
    let parent = match split {
        Some(0) => namec::namec(b"/", slash, dot, &proc.pgrp, NamecMode::Open),
        Some(i) => namec::namec(&path[..i], slash, dot, &proc.pgrp, NamecMode::Open),
        None => {
            // The dot itself, with the open-mode final redirect.
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
        port::namec::close_chan(&parent);
        return err(proc, "create: parent not a directory");
    }
    let opened = match namec::walk_one(&parent, final_name) {
        Ok(existing) => {
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
        Err(_) => match namec::create_chan(
            &parent,
            final_name,
            mode & !(port::dat::OEXCL | port::dat::OCEXEC),
            perm,
        ) {
            Ok(c) => c,
            Err(msg) => {
                port::namec::close_chan(&parent);
                return err(proc, msg);
            }
        },
    };
    port::namec::close_chan(&parent);
    match proc.fd_install_lowest(opened) {
        Some(slot) => slot,
        None => err(proc, "create: no free fd"),
    }
}

/// FD2PATH(23): fd, buf, n.  The fd's chan's walked path into the
/// validated user buffer, NUL-terminated and truncated to n.  Returns
/// 0.
fn sys_fd2path(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 3) else {
        return err(proc, "fd2path: bad argument block");
    };
    let (fd, buf_va, n) = (args[0] & 0xffff_ffff, args[1], args[2] & 0xffff_ffff); // fd, n are ints
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

/// RFORK(19): flag.  Donor flag validation: (RFFDG|RFCFDG),
/// (RFNAMEG|RFCNAMEG) and (RFENVG|RFCENVG) are each mutually
/// exclusive; RFMEM/RFNOWAIT require RFPROC.  The address space
/// follows the donor dupseg shape (the proc.rs port's RFMEM share
/// machinery).  The non-RFPROC splits re-group the CALLER.
fn sys_rfork(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "rfork: bad argument block");
    };
    let flag = args[0] & 0xffff_ffff; // rfork's flag is an int
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
        // Donor non-RFPROC branch (sysproc.c:50-57): RFMEM/RFNOWAIT are
        // Ebadarg; RFFDG privatizes a COPY of the fd group in place and
        // RFCFDG empties it (dupfgrp(ofg)/dupfgrp(nil)); the namespace
        // and env group splits re-group the CALLER, as do the note and
        // rendezvous group splits.  The in-place RFFDG is load-bearing
        // for lib9p's _postmountsrv (post.c): after forking the server
        // child (which SHARES the fd group), the parent rfork(RFFDG)s
        // to keep its srvfd private before the child's close(s->srvfd)
        // lands in the SHARED table — without it every postmountsrv
        // mount dies "mount: bad fd" (15-03's services sweep finding,
        // measured on mntgen/cs/dns/factotum/keyfs).
        if flag & (RFMEM | RFNOWAIT) != 0 {
            return err(proc, "rfork: bad flags for non-RFPROC");
        }
        if flag & RFFDG != 0 {
            let node = LockNode::new();
            let copy = proc.fd_group.lock(&node).deep_copy();
            proc.fd_group = Arc::new(Lock::new("fdgrp", copy));
        } else if flag & RFCFDG != 0 {
            proc.fd_group = proc::empty_fdgroup();
        }
        if flag & (RFNAMEG | RFCNAMEG) != 0 {
            proc::rfork_namespace_split(proc, flag & RFNAMEG != 0);
        }
        if flag & (RFENVG | RFCENVG) != 0 {
            proc::rfork_env_split(proc, flag & RFENVG != 0);
        }
        if flag & RFNOTEG != 0 {
            proc::rfork_noteg_split(proc);
        }
        if flag & RFREND != 0 {
            proc::rfork_rendez_split(proc);
        }
        return 0;
    }
    match proc::rfork_clone(proc, frame, flag) {
        // The parent's wire result is the child pid; the child's copied
        // frame carries ax = 0 (donor forkchild — the amd64 result
        // slot).
        Ok(pid) => pid,
        Err(msg) => err(proc, msg),
    }
}

/// SEEK(39): rv, fd, offset, base — the FOUR-argument shape (the
/// generated stub's `_seek(vlong *rv, int fd, vlong offset, int
/// base)`; the amd64 _seek TEXT rename carries the same number).  The
/// result offset is WRITTEN through the validated rv pointer; the wire
/// result is 0.  Base 0 absolute, 1 relative, 2 from-end via the
/// chan's stat length.  Stream devices (pipes) always error.
fn sys_seek(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "seek: bad argument block");
    };
    let (rv, fd, off, base) = (
        args[0],
        args[1] & 0xffff_ffff,
        args[2] as i64,
        (args[3] & 0xffff_ffff) as u32 as i32,
    );
    let mut rv_slot = match UserSliceMut::new(proc, rv, 8) {
        Ok(s) => s,
        Err(_) => return err(proc, "seek: bad result pointer"),
    };
    let Some(mut chan) = proc.fd_take(fd) else {
        return err(proc, "seek: bad fd");
    };
    let result = (|chan: &mut Chan| {
        // The donor sseek's stream-device rejection — a pipe end is
        // never seekable (tail.c's isseekable probe keys on this
        // failure).
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
                // SEEK_END: the chan's stat gives the length — the u64
                // LE at stat-wire byte offset 33 (stat(5)-stable).
                let mut sb = [0u8; 512];
                let n = namec::stat_chan(chan, &mut sb)?;
                if n < 41 {
                    return Err("seek: short stat record");
                }
                let length =
                    u64::from_le_bytes(sb[33..41].try_into().map_err(|_| "seek: stat length")?);
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

/// ERRSTR(41): buf, n — copy the proc's errstr (NUL-terminated) into
/// the user buffer, return 0.
fn sys_errstr(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "errstr: bad argument block");
    };
    let (buf, n) = (args[0], args[1] & 0xffff_ffff); // n is a uint
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

/// SLEEP(17): millisecs (an int — the width discipline).  ms<=0
/// yields; ms>0 blocks until the tick count passes now+ms.  The
/// note-interruptible shape: a pending-note PRE-CHECK fails the sleep
/// immediately, an early wake returns -1 "interrupted", expiry returns
/// 0.
fn sys_sleep(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "sleep: bad argument block");
    };
    let ms = args[0] & 0xffff_ffff; // long is 32-bit
    #[cfg(dev_exec)]
    {
        // sleep(long) is SIGNED — the donor syssleep yields when
        // n <= 0.
        if (ms as i32) <= 0 {
            proc::preempt(); // the donor's yield()
            return 0;
        }
        // The donor tsleep's notepending pre-check.
        if !proc.note_queue.is_empty() {
            return err(proc, "interrupted");
        }
        let deadline = crate::platform::q35::timer::ticks() + ms;
        proc.sleep_until = deadline;
        proc::sched();
        // An early wake is always a posted note this era; expiry
        // returns 0.
        if crate::platform::q35::timer::ticks() < deadline {
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

/// ALARM(6): millisecs (an int).  Arm a per-proc alarm whose expiry
/// posts the "alarm" note; a zero arg clears it.  The return is the
/// previously-REMAINING ms.
fn sys_alarm(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "alarm: bad argument block");
    };
    let ms = args[0] & 0xffff_ffff; // unsigned long is 32-bit
    #[cfg(dev_exec)]
    {
        let now = crate::platform::q35::timer::ticks();
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

/// The %q quoting of a wait-record status (libc fmtquote's rule).
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

/// AWAIT(47): buf, buflen.  Block until a child's exit record exists,
/// then write the Plan-9-shaped wait message `{pid} 0 0 0 {status}`
/// (the status %q-quoted) NUL-terminated into the validated user
/// buffer and return the record length.  No children ever -> loud -1
/// errstr.
fn sys_await(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "await: bad argument block");
    };
    let (buf, buflen) = (args[0], args[1] & 0xffff_ffff); // buflen is a uint
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
        // Block mid-syscall: the frame on this proc's own kstack
        // survives other processes' traps meanwhile.
        proc::sched();
    }
}

/// NSEC(53): NO arguments (the amd64 stub emits no arg0 spill — a
/// generic SP+8 read would consume the return-address slot, Pitfall
/// 5).  Return the epoch-nanosecond clock, or the loud no-RTC error.
/// x86_64 has no epoch clock this era (the LAPIC tick count is
/// monotonic-only); the seam is ready for one.
fn sys_nsec(_frame: &mut Ureg, proc: &mut Proc) -> u64 {
    match nsec_now() {
        Some(ns) => ns,
        None => err(proc, "nsec: no RTC"),
    }
}

/// NSEC's clock source seam (the aarch64 platform::rtc role).
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

/// RENDEZVOUS(34): tag, value — the user rendezvous.  Found: the
/// sleeper is handed OUR value and readied, and we return the SLEEPER's
/// stored value.  Absent: we park and block; on wake we return the
/// value the waker left.  Both args are full-width pointers.
fn sys_rendezvous(frame: &mut Ureg, proc: &mut Proc) -> u64 {
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
            proc::rendez_wake(s.pid, value);
            s.value
        }
        None => {
            // The first caller sleeps until a peer takes the entry.
            // The sched wake is pid-keyed and FLAT — a STALE waiter-set
            // registration could ready a parked proc with NO peer
            // pairing, so the wake is re-checked: our entry still
            // present = the wake was spurious (re-block); entry gone =
            // a peer consumed it in the same serialized syscall window.
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

/// EXITS(8): copy the optional message (nil = empty), free the proc's
/// user pages + PML4 + kstack, mark Moribund, print the exit line,
/// post the wait record to the parent, then hand off to the scheduler;
/// when no non-Moribund proc remains, print the all-exited marker and
/// park in the clean halt.  Diverges: never returns to the exited
/// image.  (The libc stub's TEXT name is `_exits`; the NUMBER is
/// unchanged — ABI.md's frozen quirk.)
fn sys_exits(frame: &mut Ureg, proc: &mut Proc) -> u64 {
    let msg_va = match sargs(frame, proc, 1) {
        Some(args) => args[0],
        None => 0, // exits never fails — a bad arg block exits msgless
    };
    // Copy the message BEFORE freeing anything (it lives in user
    // memory).  exits(nil) carries a zero pointer; an invalid string is
    // not an error either — exits must never be wedged by its own
    // diagnostics.
    let msg = if msg_va == 0 {
        Vec::new()
    } else {
        copy_user_str(proc, msg_va, ERRMAX).unwrap_or_default()
    };
    exit_tail(proc, &msg);
}

/// The shared exit tail: sys_exits's user-message path and the note
/// machinery's kernel-message kills (the no-handler kill, NOTED's
/// NDFLT, the post-free exec kill) converge here.  Frees the user
/// mappings, marks Moribund, prints the exit line, posts + wakes the
/// parent, frees the PML4 and the kstack pages LAST (the handler is
/// still running on it — safe because the swtch save writes only the
/// PROC_TABLE-resident context), then the all-exited halt or the
/// sched-loop hand-off.  Diverges.
///
/// The FIRST step is the fd-group sweep (the donor pexit's closefgrp):
/// the group is DETACHED, and only when this proc was the LAST sharer
/// are its occupied slots drained and close-dispatched.
pub(crate) fn exit_tail(proc: &mut Proc, msg: &[u8]) -> ! {
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
    proc::rendez_remove(proc);
    proc.free_user_mappings();
    // Drop the husk's remaining heap payloads (the slot itself is never
    // reaped this era).
    proc.argv = Vec::new();
    proc.wait_records = VecDeque::new();
    proc.note_queue = VecDeque::new();
    proc.state = ProcState::Moribund;
    let pid = proc.pid;
    println!("process {pid} exited: \"{}\"", core::str::from_utf8(msg).unwrap_or("<bad utf8>"));
    // 17-02 (dev_churn): the per-exit heap marker — the composed churn
    // venue's memory-side probe.  The kernel-side driver samples the
    // cursor at its own four points; this line gives the composed gate
    // (whose venue boots the real Harvey chain, not the driver) the
    // same vocabulary at per-exit granularity, so its expect driver
    // bounds the churn window's cursor delta from the kernel's own
    // emitted numbers.  Sampled AFTER the mapping/Vec frees — the
    // residue the slope doctrine names is what is left here.
    #[cfg(dev_churn)]
    {
        let (used, total) = crate::allocator::heap_stats();
        println!("churngate: heap exit {used} {total}");
    }
    // Post the wait record to the parent and wake it BEFORE the final
    // frees.
    if !proc.nowait {
        proc::post_wait_record(proc.parent, pid, msg);
    }
    // Last frees: the PML4 and the kstack pages.  The handler is STILL
    // running on the kstack and CR3 still names the PML4 — safe
    // because nothing allocates until the hand-off below and the
    // swtch save writes only the PROC_TABLE-resident context struct.
    proc.free_root_table_and_kstack();

    if !proc::any_active_procs() {
        println!("all user processes exited");
        // Clean halt: park with IRQs masked — a wedge-free stop.
        crate::trap::park_forever();
    }
    // The sched loop picks the next Ready proc.  Diverges.
    proc::sched_from_moribund();
}

/// Note delivery at the return-to-ring-3 boundary.  Called on every
/// return to ring 3 — after syscall dispatch AND after a handled IRQ —
/// with the frame the return path is about to restore.  Pops the
/// current proc's pending note and redirects the frame into the
/// notify()-installed handler, donor notify() verbatim re-keyed (9k
/// k10/syscall.c:182-192): the handler's Ureg* arg0 rides ureg->bp
/// (RARG — ABI.md's note-delivery convention), and the pushed Ureg
/// copy is the whole 184-byte frame.
///
/// The donor okaddr port: the WHOLE write window is validated
/// user-writable BEFORE any write and its pages are forced resident;
/// an unwritable/misaligned target kills the process loudly — never a
/// partial write, never a kernel write to a bad user VA.  No handler
/// installed -> print the note + kill via the exits path.
///
/// In-handler guard (the donor donotify `up->notified` check): a
/// second note is NOT delivered while a handler is running; the note
/// stays queued, noted() clears note_ureg and the next boundary
/// retries.
pub(crate) fn deliver_notes(frame: &mut Ureg) {
    let Some(mut proc) = proc::current() else {
        return;
    };
    // Leave the note queued while a handler is active (note_ureg
    // doubles as the donor's up->notified flag).
    if proc.note_ureg != 0 {
        return;
    }
    let Some(note) = proc.note_queue.pop_front() else { return };
    proc.last_note = note.buf;
    proc.last_note_len = note.len;
    let buf = note.buf; // owned NUL-padded copy
    let msg = &buf[..note.len];

    let Some(handler) = proc.notify_handler else {
        println!("note: pid {} has no notify handler", proc.pid);
        exit_tail(&mut proc, msg);
    };
    if handler & 3 != 0 {
        println!("note: pid {} misaligned notify handler", proc.pid);
        exit_tail(&mut proc, msg);
    }

    // The donor stack construction (verbatim arithmetic; wrapping_sub
    // so a pathological frame.sp can't panic before the validation):
    // sp = ureg->sp - 256 (debug preserve) - sizeof(Ureg), aligned ~7;
    // then the message and the three arg-home slots below it.
    let nureg = frame.sp.wrapping_sub(256 + UREG_SIZE as u64) & !7u64;
    let msg_va = nureg.wrapping_sub(8 + proc::ERRMAX as u64);
    let final_sp = msg_va.wrapping_sub(3 * 8);
    let window_end = nureg + UREG_SIZE as u64;

    // Validate the whole write window user-WRITABLE before any write,
    // then force its stack pages resident.
    if proc.validate_user_range(final_sp, window_end - final_sp, true).is_err() {
        println!("note: pid {} undeliverable (bad user stack window)", proc.pid);
        exit_tail(&mut proc, msg);
    }
    proc.ensure_stack_resident(final_sp as usize, window_end as usize);

    // The user-memory writes ride a single usermem window over
    // [final_sp, window_end).
    let mut window = crate::usermem::UserSliceMut::new(&mut *proc, final_sp, window_end - final_sp)
        .expect("note window validated + resident above");
    let w = window.as_slice_mut();
    let nureg_off = (nureg - final_sp) as usize;
    frame.copy_to_saved(&mut w[nureg_off..nureg_off + UREG_SIZE]);
    let msg_off = (msg_va - final_sp) as usize;
    w[msg_off..msg_off + proc::ERRMAX].copy_from_slice(&buf);
    // The arg homes: [sp+8] = nureg (arg0), [sp+16] = &msg (arg1).
    window.write_u64(8, nureg);
    window.write_u64(16, msg_va);
    // The donor's redirect: arg0 through ureg->bp (RARG — the 6c
    // first-arg register), the handler as the resume ip, the shrunk
    // sp.
    frame.bp = nureg;
    frame.sp = final_sp;
    frame.pc = handler;
    // The donor's up->noteureg: NOTED's NCONT/NRSTR restore source.
    proc.note_ureg = nureg;
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
            assert_eq!(core::slice::from_raw_parts(s0, 12), b"/amd64/init\0");
            let s1 = page.add(a1 as usize - base);
            assert_eq!(core::slice::from_raw_parts(s1, 3), b"-e\0");
            // The seed packs the strings top-down in ARGUMENT order:
            // argv[0] sits above argv[1], both below the page top.
            assert!(a1 < a0);
            assert!(a0 as usize + 12 <= base + PAGE_SIZE_4K);
        }
        // The live frame was rewritten to the new image's entry.
        assert_eq!(frame.pc as usize, INITTEXT);
        // ax carries the seeded Tos (the exec-return contract — the
        // dispatch result writeback lands exec_replace's return, which
        // IS the Tos pointer; main9.s stores exactly that AX into
        // _tos).  bp (the 6c first-arg register) stays whatever the
        // caller had — argc arrives on the seeded stack.
        assert_eq!(frame.ax, tos_va);
    }
}
