//! devproc tests (05-09, D-09's ps(1) data source + init's ctl write).
//!
//! The /proc root's directory read carries one stat(5) wire entry per
//! live pid, named by the decimal pid — ps's `chdir("/proc")` +
//! `open(".")` + dirreadall shape (sys/src/cmd/ps.c:33-45).  The per-pid
//! directory listing carries args/ctl/note/status.  The status read
//! emits ≥12 whitespace-separated fields in devproc.c's order
//! (ps.c:75-87's tokenizer contract).  The args read serves the exact
//! argv recorded at exec (EMPTY for a never-exec'd pid, never garbage).
//! The ctl write parses + records "pri N" (an unknown verb errors
//! loudly).  And the 04-08 note path is pinned unregressed (Pitfall 5 —
//! postnote(PNPROC) must keep working byte-identically).
//!
//! The scripted-hooks idiom follows devcons_test.rs/devpipe_test.rs:
//! the set-once hooks are installed by ONE test; the fake proc table is
//! a host-side Vec the hooks read/write (the arch side's PROC_TABLE
//! stands in — the dependency-inversion idiom keeps the port crate
//! arch-free).

use std::sync::Mutex;

use port::dat::{OREAD, OWRITE};
use port::devproc::{self, ProcHooks, ProcInfo};

/// The fake proc table (the arch PROC_TABLE's stand-in).  `argv` is the
/// exec-recorded NUL-joined shape (proc.rs's Proc.argv); pid 4 never
/// exec'd, so its argv is EMPTY.
struct FakeProc {
    pid: u64,
    name: &'static str,
    state: &'static str,
    argv: Vec<u8>,
    pri: u32,
}
static PROCS: Mutex<Vec<FakeProc>> = Mutex::new(Vec::new());
/// Recorded note posts (the 04-08 path's unregression pin).
static NOTES: Mutex<Vec<(u64, Vec<u8>)>> = Mutex::new(Vec::new());

fn hook_list() -> Vec<u64> {
    PROCS.lock().unwrap().iter().map(|p| p.pid).collect()
}

fn hook_info(pid: u64) -> Option<ProcInfo> {
    PROCS.lock().unwrap().iter().find(|p| p.pid == pid).map(|p| ProcInfo {
        name: p.name.as_bytes().to_vec(),
        state: p.state,
        args: p.argv.clone(),
        pri: p.pri,
    })
}

fn hook_set_pri(pid: u64, pri: u32) -> port::Result<()> {
    let mut t = PROCS.lock().unwrap();
    let Some(p) = t.iter_mut().find(|p| p.pid == pid) else {
        return Err("devproc: no such process");
    };
    p.pri = pri;
    Ok(())
}

fn hook_set_private(pid: u64) -> port::Result<()> {
    // The 15-03 "private" verb: recorded-only this era (no /proc mem
    // files to exclude yet) — the test hook accepts for the live pid.
    let t = PROCS.lock().unwrap();
    if t.iter().any(|p| p.pid == pid) {
        Ok(())
    } else {
        Err("devproc: no such process")
    }
}

fn hook_post(pid: u64, msg: &[u8]) -> port::Result<()> {
    NOTES.lock().unwrap().push((pid, msg.to_vec()));
    Ok(())
}

/// Parse a directory read's stat(5) wire records into (name, qid path,
/// mode) triples (libc dirread's dirpackage shape: records are
/// self-delimiting via the size prefix — m = 2 + GBIT16(buf)).
/// Wire layout (dat.rs StatWriter): size[2] type[2] dev[4]
/// qid.typ[1] qid.vers[4] qid.path[8] mode[4] atime[4] mtime[4]
/// length[8] then FOUR length-prefixed strings (name, uid, gid, muid —
/// statcheck walks FOUR; a three-string record fails dirreadall with
/// "empty directory" — the 05-09 gate's catch, pinned here).
fn parse_dir(buf: &[u8]) -> Vec<(String, u64, u32)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 2 <= buf.len() {
        let sz = u16::from_le_bytes([buf[i], buf[i + 1]]) as usize;
        if sz == 0 {
            break;
        }
        let rec = &buf[i..i + 2 + sz];
        let qid_path = u64::from_le_bytes(rec[13..21].try_into().unwrap());
        let mode = u32::from_le_bytes(rec[21..25].try_into().unwrap());
        // Walk the four strings; the record must END exactly after them
        // (statcheck's `buf != ebuf` rejection).
        let mut p = 41usize;
        let mut names = Vec::new();
        for _ in 0..4 {
            let nlen = u16::from_le_bytes([rec[p], rec[p + 1]]) as usize;
            p += 2;
            names.push(String::from_utf8(rec[p..p + nlen].to_vec()).unwrap());
            p += nlen;
        }
        assert_eq!(p, 2 + sz, "the record ends exactly after FOUR strings (statcheck)");
        out.push((names[0].clone(), qid_path, mode));
        i += 2 + sz;
    }
    out
}

/// The whole 05-09 surface (one test — the hooks are set-once).  Order
/// matters: the ctl arm re-reads status to see the recorded priority.
#[test]
fn devproc_root_status_args_ctl_and_note_surface() {
    // Seed the fake table + register the hooks (set-once — one test).
    PROCS.lock().unwrap().extend([
        FakeProc { pid: 1, name: "init", state: "Blocked", argv: b"init\0".to_vec(), pri: 0 },
        FakeProc { pid: 2, name: "rc", state: "Running", argv: b"rc\0-i\0".to_vec(), pri: 0 },
        FakeProc {
            pid: 3,
            name: "paqfs",
            state: "Blocked",
            argv: b"paqfs\0-iv\0/dev/rootfs\0".to_vec(),
            pri: 0,
        },
        // Never exec'd (a fork child pre-exec): args must read EMPTY.
        FakeProc { pid: 4, name: "fork", state: "Ready", argv: Vec::new(), pri: 0 },
    ]);
    devproc::set_proc_hooks(ProcHooks {
        list: hook_list,
        info: hook_info,
        set_pri: hook_set_pri,
        set_private: hook_set_private,
    });
    devproc::set_post_hook(hook_post);

    // -- Root dir read: one stat(5) entry per live pid, named by the
    //    decimal pid (ps's dirreadall of open(".") after chdir /proc).
    let root = devproc::attach();
    let mut root_rd = devproc::open(&root, OREAD).expect("the root dir opens OREAD");
    let mut buf = [0u8; 4096];
    let n = devproc::read(&mut root_rd, &mut buf, u64::MAX).expect("the root listing reads");
    let entries = parse_dir(&buf[..n as usize]);
    let names: Vec<&str> = entries.iter().map(|e| e.0.as_str()).collect();
    assert_eq!(names, ["1", "2", "3", "4"], "one entry per live pid, decimal-named");
    for (name, path, mode) in &entries {
        assert!(mode & 0x8000_0000 != 0, "a pid entry is a directory (DMDIR)");
        assert_eq!(*path, name.parse::<u64>().unwrap() * 8, "the dir qid encoding (pid*8)");
    }
    // EOF: a second read at the advanced streaming offset returns 0
    // (dirreadall's loop terminator).
    let n = devproc::read(&mut root_rd, &mut buf, u64::MAX).expect("root EOF read");
    assert_eq!(n, 0, "the listing ends at EOF");

    // -- A pid dir lists its files (the plan's args/ctl/note/status).
    let d = devproc::walk(&root, b"2").expect("walk pid 2");
    let mut d = devproc::open(&d, OREAD).expect("the pid dir opens OREAD");
    let n = devproc::read(&mut d, &mut buf, u64::MAX).expect("the pid dir reads");
    let names: Vec<String> = parse_dir(&buf[..n as usize]).into_iter().map(|e| e.0).collect();
    assert_eq!(names, ["args", "ctl", "note", "status"], "the pid dir's file set");

    // -- status: ≥12 whitespace-separated fields in devproc.c's order
    //    (ps.c's tokenizer map: [0] name, [1] user, [2] state,
    //    [3..=8] six times, [9] memory, [10] basepri, [11] pri).
    let s = devproc::walk(&d, b"status").expect("walk status");
    let mut s = devproc::open(&s, OREAD).expect("status opens OREAD");
    let n = devproc::read(&mut s, &mut buf, u64::MAX).expect("status reads");
    let text = std::str::from_utf8(&buf[..n as usize]).unwrap();
    let fields: Vec<&str> = text.split_whitespace().collect();
    assert!(fields.len() >= 12, "ps tokenizes ≥12 fields (got {})", fields.len());
    assert_eq!(fields[0], "rc", "field 0 is the proc name");
    assert_eq!(fields[1], "glenda", "field 1 is the user (D-02's sane default)");
    assert_eq!(fields[2], "Running", "field 2 is the state");
    for f in &fields[3..12] {
        assert!(f.parse::<u64>().is_ok(), "numeric field: {f}");
    }

    // -- args: the exact argv recorded at exec (NUL-joined on the proc,
    //    space-joined at the read — the donor procargs shape); a
    //    never-exec'd pid reads EMPTY (ps.c:135's Noargs arm).
    let a = devproc::walk(&d, b"args").expect("walk args");
    let mut a = devproc::open(&a, OREAD).expect("args opens OREAD");
    let n = devproc::read(&mut a, &mut buf, u64::MAX).expect("args reads");
    assert_eq!(&buf[..n as usize], b"rc -i", "the exact argv recorded at exec");
    let d4 = devproc::walk(&root, b"4").expect("walk pid 4");
    let a4 = devproc::walk(&d4, b"args").expect("walk pid 4 args");
    let mut a4 = devproc::open(&a4, OREAD).expect("open pid 4 args");
    let n = devproc::read(&mut a4, &mut buf, u64::MAX).expect("never-exec'd args read");
    assert_eq!(n, 0, "a never-exec'd pid's args are EMPTY, not garbage");

    // -- ctl: "pri N" parses + RECORDS (init.c:52's write); the status
    //    read then reports it; unknown verbs error loudly.
    let c = devproc::walk(&d, b"ctl").expect("walk ctl");
    let mut c = devproc::open(&c, OWRITE).expect("ctl opens OWRITE");
    let n = devproc::write(&mut c, b"pri 10").expect("the pri write is accepted");
    assert_eq!(n, 6, "the write takes the message's bytes");
    assert_eq!(
        PROCS.lock().unwrap().iter().find(|p| p.pid == 2).unwrap().pri,
        10,
        "the priority is RECORDED on the proc"
    );
    let mut s2 = devproc::open(&devproc::walk(&d, b"status").unwrap(), OREAD).unwrap();
    let n = devproc::read(&mut s2, &mut buf, u64::MAX).expect("status re-reads");
    let text = std::str::from_utf8(&buf[..n as usize]).unwrap();
    let fields: Vec<&str> = text.split_whitespace().collect();
    assert_eq!(fields[10], "10", "basepri reports the recorded value");
    assert_eq!(fields[11], "10", "pri reports the recorded value");
    let err = devproc::write(&mut c, b"frob 1").expect_err("an unknown ctl verb errors loudly");
    assert!(err.contains("ctl"), "the error names the ctl path: {err}");

    // -- note: the 04-08 path is byte-identical (Pitfall 5) — a write
    //    posts the message to the addressed pid via the post hook.
    let nn = devproc::walk(&d, b"note").expect("walk note");
    let mut nn = devproc::open(&nn, OWRITE).expect("note opens OWRITE");
    let n = devproc::write(&mut nn, b"interrupt").expect("the note write posts");
    assert_eq!(n, 9);
    assert_eq!(
        NOTES.lock().unwrap().as_slice(),
        &[(2u64, b"interrupt".to_vec())],
        "the note posts to the ADDRESSED pid (T-04-17)"
    );
}
