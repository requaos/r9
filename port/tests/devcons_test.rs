//! devcons read-path tests (05-08, D-06): the cooked line discipline
//! (erase 0x08 / kill-line 0x15 / CR→LF / line completes with its
//! newline / ^D ends without buffering), DEL (0x7f) interception (never
//! buffered — flushes the partial line, posts "interrupt" to the
//! readers' note groups, fails the read -1/"interrupted"), raw mode
//! (unprocessed bytes, no DEL interception), consctl switching + mode
//! readback, the 4 KiB drop-oldest overrun bound (T-05-15), and the
//! block-until-input shape.
//!
//! The scripted-hooks idiom follows devpipe_test.rs: the set-once
//! ConsHooks are installed by ONE test; the block hook plays the host
//! typing into the scripted RX queue while "we" are parked (the real
//! wake is the tick seeing the ring non-empty — the test's feed IS the
//! wake condition).

use std::collections::VecDeque;
use std::sync::Mutex;

use port::dat::OREAD;
use port::devcons::{self, ConsHooks};

const TEST_PID: u64 = 7;

/// The host-side "RX ring" (what pl011::cons_rx_drain would drain).
static RX: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());
/// Bytes the block hook feeds into RX per park (the host typing while
/// the reader sleeps).
static SCRIPT: Mutex<VecDeque<Vec<u8>>> = Mutex::new(VecDeque::new());
/// Recorded note-group posts: (reader pids, message).
static POSTS: Mutex<Vec<(Vec<u64>, Vec<u8>)>> = Mutex::new(Vec::new());
static BLOCKS: Mutex<u32> = Mutex::new(0);

fn test_current() -> u64 {
    TEST_PID
}

fn test_drain_rx(buf: &mut [u8]) -> usize {
    let mut rx = RX.lock().unwrap();
    let mut n = 0;
    while n < buf.len() {
        match rx.pop_front() {
            Some(b) => {
                buf[n] = b;
                n += 1;
            }
            None => break,
        }
    }
    n
}

fn test_block() {
    *BLOCKS.lock().unwrap() += 1;
    let step = SCRIPT.lock().unwrap().pop_front().expect("block without a scripted feed");
    RX.lock().unwrap().extend(step);
}

fn test_post_group(readers: &[u64], msg: &[u8]) {
    POSTS.lock().unwrap().push((readers.to_vec(), msg.to_vec()));
}

fn cons_chan() -> port::dat::Chan {
    let root = devcons::attach();
    let c = devcons::walk(&root, b"cons").expect("walk cons");
    devcons::open(&c, OREAD).expect("open cons")
}

fn consctl_chan(mode: u32) -> port::dat::Chan {
    let root = devcons::attach();
    let c = devcons::walk(&root, b"consctl").expect("walk consctl");
    devcons::open(&c, mode).expect("open consctl")
}

fn read_cons(buf: &mut [u8]) -> port::Result<u64> {
    let mut c = cons_chan();
    devcons::read(&mut c, buf, u64::MAX)
}

/// The whole read-path behavior suite (one test — the hooks are
/// set-once).  Order matters: each arm builds on the previous state.
#[test]
fn cons_read_path_cooked_raw_del_and_consctl() {
    devcons::set_cons_hooks(ConsHooks {
        current: test_current,
        drain_rx: test_drain_rx,
        block: test_block,
        post_group: test_post_group,
    });

    // -- Cooked: a split feed blocks twice, CR→LF, the line includes its
    //    newline.
    SCRIPT.lock().unwrap().extend([b"ec".to_vec(), b"ho r9shell7\r".to_vec()]);
    let mut buf = [0u8; 64];
    let n = read_cons(&mut buf).expect("cooked read returns the line");
    assert_eq!(&buf[..n as usize], b"echo r9shell7\n", "whole line with trailing newline, CR→LF");
    assert_eq!(*BLOCKS.lock().unwrap(), 2, "blocked once per partial feed");

    // -- Bytes already queued never block.
    RX.lock().unwrap().extend(b"cat\n");
    let n = read_cons(&mut buf).expect("queued line reads");
    assert_eq!(&buf[..n as usize], b"cat\n");
    assert_eq!(*BLOCKS.lock().unwrap(), 2, "no block with input queued");

    // -- Erase (0x08) edits the in-assembly line.
    RX.lock().unwrap().extend(b"ab\x08c\n");
    let n = read_cons(&mut buf).expect("erase-edited line");
    assert_eq!(&buf[..n as usize], b"ac\n", "0x08 erased the last byte");

    // -- Kill-line (0x15) discards the in-assembly line.
    RX.lock().unwrap().extend(b"ab\x15c\n");
    let n = read_cons(&mut buf).expect("kill-edited line");
    assert_eq!(&buf[..n as usize], b"c\n", "0x15 killed the partial line");

    // -- ^D ends the line without being buffered.
    RX.lock().unwrap().extend(b"hi\x04");
    let n = read_cons(&mut buf).expect("caret-D-terminated line");
    assert_eq!(&buf[..n as usize], b"hi", "^D sent the line without a newline");

    // -- DEL: never buffered; flushes the partial; posts "interrupt" to
    //    the reader's group; the read fails "interrupted"; the shell
    //    survives to answer the next line (the gate's shape).
    SCRIPT.lock().unwrap().extend([b"fnord xyz".to_vec(), b"\x7f".to_vec()]);
    let err = read_cons(&mut buf).expect_err("DEL interrupts the read");
    assert_eq!(err, "interrupted");
    {
        let posts = POSTS.lock().unwrap();
        assert_eq!(posts.len(), 1, "one note post for the DEL");
        assert_eq!(posts[0].0, &[TEST_PID], "the note targets the reader (group resolution is arch-side)");
        assert_eq!(posts[0].1, b"interrupt");
    }
    // The partial "fnord xyz" was flushed: the fresh line reads clean.
    SCRIPT.lock().unwrap().extend([b"echo r9delok\n".to_vec()]);
    let n = read_cons(&mut buf).expect("post-DEL read");
    assert_eq!(&buf[..n as usize], b"echo r9delok\n", "the partial line did NOT corrupt the fresh command");

    // -- consctl: the mode word reads back; rawon switches (and flushes a
    //    pending partial per the donor's ch=0 marker).
    let mut wbuf = [0u8; 16];
    let n = devcons::read(&mut consctl_chan(OREAD), &mut wbuf, u64::MAX).expect("consctl reads");
    assert_eq!(&wbuf[..n as usize], b"cooked\n");
    let mut cc = consctl_chan(port::dat::OWRITE);
    devcons::write(&mut cc, b"rawon").expect("rawon write");
    let n = devcons::read(&mut consctl_chan(OREAD), &mut wbuf, u64::MAX).expect("consctl reads raw");
    assert_eq!(&wbuf[..n as usize], b"raw\n");

    // -- Raw: bytes pass UNPROCESSED (DEL and 0x08 are data), blocking
    //    only while empty.
    SCRIPT.lock().unwrap().extend([b"a\x7f\x08b".to_vec()]);
    let n = read_cons(&mut buf).expect("raw read");
    assert_eq!(&buf[..n as usize], b"a\x7f\x08b", "raw passes DEL/BS verbatim");
    assert!(POSTS.lock().unwrap().len() == 1, "raw DEL posts no note");

    // -- rawoff back to cooked; cooked editing resumes.
    devcons::write(&mut cc, b"rawoff").expect("rawoff write");
    RX.lock().unwrap().extend(b"z\x08q\n");
    let n = read_cons(&mut buf).expect("cooked again");
    assert_eq!(&buf[..n as usize], b"q\n");

    // -- Overrun (T-05-15): the input queue is bounded at 4096 with
    //    drop-oldest; the reader still gets a (capped) completed line.
    let flood: Vec<u8> = core::iter::repeat(b'a').take(5000).chain([b'\n']).collect();
    RX.lock().unwrap().extend(flood);
    let mut big = [0u8; 2048];
    let n = read_cons(&mut big).expect("overrun read");
    assert_eq!(n, 1025, "the assembled line is LINE_CAP content + newline");
    assert_eq!(big[n as usize - 1], b'\n');
}

/// CR-01: the stat(5) wire record is FOUR length-prefixed strings (name,
/// uid, gid, muid — libc's statcheck walks four and rejects a record that
/// ends early, convM2D.c).  Pin the shape devproc_test's parse_dir way:
/// walk all four strings and require the record to END exactly after them
/// (statcheck's `buf != ebuf` rejection).  Runs hook-free (stat/listing
/// never touch the RX path), so it is safe beside the scripted suite.
#[test]
fn stat_records_carry_four_strings() {
    /// Walk one record's four strings; return them.
    fn strings_of(rec: &[u8]) -> Vec<String> {
        let mut p = 41usize;
        let mut out = Vec::new();
        for _ in 0..4 {
            let slen = u16::from_le_bytes([rec[p], rec[p + 1]]) as usize;
            p += 2;
            out.push(String::from_utf8(rec[p..p + slen].to_vec()).unwrap());
            p += slen;
        }
        assert_eq!(p, rec.len(), "the record ends exactly after FOUR strings (statcheck)");
        out
    }

    let root = devcons::attach();
    let mut sb = [0u8; 128];

    let n = devcons::stat(&root, &mut sb).expect("stat root") as usize;
    assert_eq!(strings_of(&sb[..n]), ["#c", "glenda", "glenda", "glenda"], "root stat strings");

    let c = devcons::walk(&root, b"cons").expect("walk cons");
    let n = devcons::stat(&c, &mut sb).expect("stat cons") as usize;
    assert_eq!(strings_of(&sb[..n]), ["cons", "glenda", "glenda", "glenda"], "file stat strings");

    // The root directory listing's records are the same 4-string shape
    // (libc dirread's statcheck runs per record).
    let mut rd = devcons::open(&root, OREAD).expect("open root dir");
    let mut listing = [0u8; 1024];
    let n = devcons::read(&mut rd, &mut listing, u64::MAX).expect("dir read") as usize;
    let mut i = 0usize;
    let mut recs = 0;
    while i + 2 <= n {
        let sz = u16::from_le_bytes([listing[i], listing[i + 1]]) as usize;
        if sz == 0 {
            break;
        }
        strings_of(&listing[i..i + 2 + sz]);
        recs += 1;
        i += 2 + sz;
    }
    assert_eq!(i, n, "the listing parses to the last byte");
    assert!(recs >= 4, "the listing holds the files' records");
}
