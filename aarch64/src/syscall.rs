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
//! plus D-14's named core member EXEC(7, Task 2 wires it) — zero guessed
//! syscalls.  The trace lists NO brk_(24)/segbrk(12) (the D-14 fmt/pool
//! watch item settled by evidence), so they stay None.  Every other number
//! prints `syscall N unimplemented` and returns -1 — loud, never wedged
//! (D-14).
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
//! with the console-read drainer either.

use alloc::vec::Vec;

use port::dat::{Chan, DEV_CONS, DEV_ROOT, OEXEC, OMASK, OREAD, OTRUNC};
use port::println;

use crate::exec;
use crate::platform;
use crate::proc::{self, Proc, ProcState, ERRMAX};
use crate::trap::TrapFrame;

// The frozen syscall numbers (Harvey sys/src/libc/9syscall/sys.h).
const CLOSE: u64 = 4;
const EXEC: u64 = 7;
const EXITS: u64 = 8;
const OPEN: u64 = 14;
const PREAD: u64 = 50;
const PWRITE: u64 = 51;

/// Handler shape (04-RESEARCH.md Pattern 2 / Code Examples): the live trap
/// frame (arg source + result sink) and the calling proc.
type Handler = fn(&mut TrapFrame, &mut Proc) -> u64;

/// The dispatch table — Harvey sys.h 0–54 verbatim as Option<fn> entries.
/// None → loud unimplemented at dispatch time (D-14).  The implemented set
/// is exactly the 04-01 measured trace {pwrite(51), open(14), pread(50),
/// close(4), exits(8)} plus D-14's named core member EXEC(7) — nothing
/// else; the trace carries no brk_(24)/segbrk(12), so they stay None.
static DISPATCH: [Option<Handler>; 55] = {
    let mut t: [Option<Handler>; 55] = [None; 55];
    t[CLOSE as usize] = Some(sys_close);
    t[EXEC as usize] = Some(sys_exec);
    t[EXITS as usize] = Some(sys_exits);
    t[OPEN as usize] = Some(sys_open);
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
    // EXITS never reaches here (it parks divergent); every other handler's
    // result rides frame.x0 back to user R0.
    frame.x0 = result;
}

/// Read the argument slots from user memory at SP+8 (the D-07 published
/// formula: `(Sargs*)(frame.sp + 8)`; SP+0 is the reserved LR slot).  The
/// arg block is user memory too, so it is validated first (T-04-11).
fn sargs(frame: &mut TrapFrame, proc: &mut Proc, arity: usize) -> Option<[u64; 4]> {
    let base = frame.sp + 8;
    if proc.validate_user_range(base, 8 * arity as u64, false).is_err() {
        return None;
    }
    let sargs = base as *const u64;
    let mut out = [0u64; 4];
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
/// 7); the console ignores offsets (writes append), devroot is read-only.
fn sys_pwrite(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "pwrite: bad argument block");
    };
    let (fd, buf, n, _off) = (args[0], args[1], args[2], args[3]);
    if proc.validate_user_range(buf, n, false).is_err() {
        return err(proc, "pwrite: bad user buffer");
    }
    let dev = match proc.fd(fd) {
        Some(chan) => chan.dev,
        None => return err(proc, "pwrite: bad fd"),
    };
    match dev {
        DEV_CONS => platform::devcons::cons_write(unsafe { user_slice(buf, n) }),
        DEV_ROOT => err(proc, "devroot: read-only filesystem"),
        _ => err(proc, "pwrite: unknown device"),
    }
}

/// PREAD(50): fd, buf, n, offset.  devroot chans read via the paqfs
/// reader's byte-granular read_at (offset -1 uses + advances the chan's
/// streaming offset); devcons chans drain the console RX ring and return 0
/// on empty (no blocking console-read this phase).
fn sys_pread(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 4) else {
        return err(proc, "pread: bad argument block");
    };
    let (fd, buf, n, off) = (args[0], args[1], args[2], args[3]);
    if proc.validate_user_range(buf, n, true).is_err() {
        return err(proc, "pread: bad user buffer");
    }
    let dev = match proc.fd(fd) {
        Some(chan) => chan.dev,
        None => return err(proc, "pread: bad fd"),
    };
    match dev {
        DEV_CONS => {
            let read = platform::devcons::cons_read(unsafe { user_slice_mut(buf, n) });
            if let Some(chan) = proc.fd(fd) {
                chan.offset += read;
            }
            read
        }
        DEV_ROOT => {
            let Some(root) = port::devroot::root() else {
                return err(proc, "devroot: not mounted");
            };
            let Some(chan) = proc.fd(fd) else {
                return err(proc, "pread: bad fd");
            };
            match root.read(chan, unsafe { user_slice_mut(buf, n) }, off) {
                Ok(read) => read,
                Err(msg) => err(proc, msg),
            }
        }
        _ => err(proc, "pread: unknown device"),
    }
}

/// OPEN(14): path, mode.  The literal names "cons" and "#c/cons" yield a
/// devcons chan; absolute or bare names walk devroot component-wise from
/// the root (D-15's one mechanism).  Installed in the lowest free fd slot.
fn sys_open(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let Some(args) = sargs(frame, proc, 2) else {
        return err(proc, "open: bad argument block");
    };
    let (path_va, mode) = (args[0], args[1] as u32);
    let path = match copy_user_str(proc, path_va, ERRMAX) {
        Ok(path) => path,
        Err(msg) => return err(proc, msg),
    };
    let access = mode & OMASK;

    let chan = if path == b"cons" || path == b"#c/cons" {
        if access == OEXEC {
            return err(proc, "cons: cannot exec-open the console");
        }
        // Qid path 1 = the cons file (the preopen convention, proc.rs).
        Chan::new(DEV_CONS, port::dat::Qid { path: 1, vers: 0, typ: 0 }, access)
    } else {
        // devroot: the initrd root tree is read-only — any write-side
        // intent (OWRITE/ORDWR access, OTRUNC) fails.
        if access != OREAD && access != OEXEC {
            return err(proc, "devroot: read-only filesystem");
        }
        if mode & OTRUNC != 0 {
            return err(proc, "devroot: read-only filesystem");
        }
        let Some(root) = port::devroot::root() else {
            return err(proc, "devroot: not mounted");
        };
        let mut c = match root.attach_root() {
            Ok(c) => c,
            Err(msg) => return err(proc, msg),
        };
        for comp in path.split(|&b| b == b'/').filter(|c| !c.is_empty()) {
            c = match root.walk(&c, comp) {
                Ok(c) => c,
                Err(_) => return err(proc, "open: path does not exist"),
            };
        }
        match root.open(&c, access) {
            Ok(c) => c,
            Err(msg) => return err(proc, msg),
        }
    };

    let Some(slot) = proc.fd_table.iter().position(|s| s.is_none()) else {
        return err(proc, "open: no free fd");
    };
    proc.fd_table[slot] = Some(chan);
    slot as u64
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

    // Resolve the target through devroot (the one exec byte-source
    // mechanism, D-15): absolute or bare path, component-wise walk, OEXEC.
    let Some(root) = port::devroot::root() else {
        return err(proc, "devroot: not mounted");
    };
    let mut c = match root.attach_root() {
        Ok(c) => c,
        Err(msg) => return err(proc, msg),
    };
    let mut last = &path[..];
    for comp in path.split(|&b| b == b'/').filter(|c| !c.is_empty()) {
        last = comp;
        c = match root.walk(&c, comp) {
            Ok(c) => c,
            Err(_) => return err(proc, "exec: path does not exist"),
        };
    }
    let Ok(c) = root.open(&c, OEXEC) else {
        return err(proc, "exec: cannot exec-open");
    };
    let Ok(bin) = root.entry(&c) else {
        return err(proc, "exec: chan carries no paqfs entry");
    };
    let name = core::str::from_utf8(last).unwrap_or("?");
    let arg_slices: Vec<&[u8]> = argv.iter().map(|a| a.as_slice()).collect();
    match exec::exec_replace(proc, root.image(), &bin, name, &arg_slices, frame) {
        // dispatch writes frame.x0 = tos_va — the same value exec_replace
        // seeded into the frame's x0, so the writeback agrees by
        // construction and the eret enters the new image with R0 = tos.
        Ok(tos_va) => tos_va,
        Err(msg) => err(proc, msg),
    }
}

/// CLOSE(4): drop the fd slot.
fn sys_close(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {    let Some(args) = sargs(frame, proc, 1) else {
        return err(proc, "close: bad argument block");
    };
    let fd = args[0] as usize;
    match proc.fd_table.get_mut(fd) {
        Some(slot) if slot.is_some() => {
            *slot = None;
            0
        }
        _ => err(proc, "close: bad fd"),
    }
}

/// EXITS(8): copy the optional message (nil = empty), free the proc's user
/// pages + root-table page + kstack via the mapping_log / recorded PAs,
/// mark Moribund, print the exit line; when no non-Moribund proc remains,
/// print the all-exited marker and park in the clean wfi halt
/// (single-process era — 04-07 turns this into a reschedule).  Diverges:
/// never returns to the exited image.
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

    proc.free_user_mappings();
    proc.state = ProcState::Moribund;
    let pid = proc.pid;
    println!(
        "process {pid} exited: \"{}\"",
        core::str::from_utf8(&msg).unwrap_or("<bad utf8>")
    );
    // Last frees: the root-table page and the kstack pages.  The handler is
    // STILL running on the kstack and TTBR0 still names the root table —
    // safe this era because nothing allocates afterwards (the all-exited
    // halt follows immediately); documented on free_root_table_and_kstack.
    proc.free_root_table_and_kstack();

    if !proc::any_active_procs() {
        println!("all user processes exited");
    }
    // Clean halt (SC#5 shape): park in wfi.  IRQs stay masked (trap context
    // never unmasks them), so this is a wedge-free stop.
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
