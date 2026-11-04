//! devcons read-path tests (05-08, D-06): the cooked line discipline
//! (erase 0x08 / kill-line 0x15 / CR→LF / line completes with its
//! newline / ^D ends without buffering), DEL (0x7f) interception (never
//! buffered — flushes the partial line, posts "interrupt" to the
//! readers' note groups, fails the read -1/"interrupted"), raw mode
//! (unprocessed bytes, no DEL interception), consctl switching + mode
//! readback, the 4 KiB drop-oldest overrun bound (T-05-15), and the
//! block-until-input shape.
//!
//! 261104-b4p (items 1+2): the cooked-mode ECHO arms (typed bytes echo
//! verbatim at drain time; a BACKSPACE that pops echoes BS-SP-BS; CR/LF
//! line completers echo CRLF; ^U/^D/DEL stay silent; raw mode echoes
//! NOTHING; a "rawon" flush never retro-echoes the partial line) and the
//! #c/pid file pin (the 05-09 implementation, pinned: the hook's pid as
//! decimal ASCII, offset/streaming reads, the OWRITE refusal, and stat
//! length tracking).
//!
//! The scripted-hooks idiom follows devpipe_test.rs: the set-once
//! ConsHooks are installed by ONE test; the block hook plays the host
//! typing into the scripted RX queue while "we" are parked (the real
//! wake is the tick seeing the ring non-empty — the test's feed IS the
//! wake condition).  The pid pin registers set_pid_hook — a SEPARATE
//! set-once static, safe beside it in the same test process (neither
//! ever registers twice).

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
/// 261104-b4p: the recorded echo stream (what the wire would carry).
static ECHO: Mutex<Vec<u8>> = Mutex::new(Vec::new());

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

fn test_echo(bytes: &[u8]) {
    ECHO.lock().unwrap().extend_from_slice(bytes);
}

/// Drain the recorded echo stream (each arm asserts its own slice).
fn take_echo() -> Vec<u8> {
    core::mem::take(&mut *ECHO.lock().unwrap())
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
        echo: test_echo,
    });

    // -- Cooked: a split feed blocks twice, CR→LF, the line includes its
    //    newline.  ECHO: the typed bytes echo verbatim as drained
    //    ("ec" then "ho r9shell7"), the CR completer echoes CRLF.
    SCRIPT.lock().unwrap().extend([b"ec".to_vec(), b"ho r9shell7\r".to_vec()]);
    let mut buf = [0u8; 64];
    let n = read_cons(&mut buf).expect("cooked read returns the line");
    assert_eq!(&buf[..n as usize], b"echo r9shell7\n", "whole line with trailing newline, CR→LF");
    assert_eq!(*BLOCKS.lock().unwrap(), 2, "blocked once per partial feed");
    assert_eq!(take_echo(), b"echo r9shell7\r\n", "cooked echo: bytes verbatim + CRLF completer");

    // -- Bytes already queued never block.  An LF completer echoes CRLF
    //    too (the terminal must advance — write_bytes is byte-exact).
    RX.lock().unwrap().extend(b"cat\n");
    let n = read_cons(&mut buf).expect("queued line reads");
    assert_eq!(&buf[..n as usize], b"cat\n");
    assert_eq!(*BLOCKS.lock().unwrap(), 2, "no block with input queued");
    assert_eq!(take_echo(), b"cat\r\n", "LF completer echoes CRLF");

    // -- Erase (0x08) edits the in-assembly line; the POP echoes the
    //    BS-SP-BS triple (putb's shape).
    RX.lock().unwrap().extend(b"ab\x08c\n");
    let n = read_cons(&mut buf).expect("erase-edited line");
    assert_eq!(&buf[..n as usize], b"ac\n", "0x08 erased the last byte");
    assert_eq!(take_echo(), b"ab\x08 \x08c\r\n", "a popping BS echoes BS-SP-BS");

    // -- A stray BACKSPACE on an EMPTY line pops nothing and echoes
    //    NOTHING.
    RX.lock().unwrap().extend(b"\x08x\n");
    let n = read_cons(&mut buf).expect("stray-BS line");
    assert_eq!(&buf[..n as usize], b"x\n", "a stray BS on an empty line is a no-op");
    assert_eq!(take_echo(), b"x\r\n", "a non-popping BS echoes nothing");

    // -- Kill-line (0x15) discards the in-assembly line and echoes
    //    NOTHING for the kill (the typed bytes already echoed verbatim).
    RX.lock().unwrap().extend(b"ab\x15c\n");
    let n = read_cons(&mut buf).expect("kill-edited line");
    assert_eq!(&buf[..n as usize], b"c\n", "0x15 killed the partial line");
    assert_eq!(take_echo(), b"abc\r\n", "^U is silent on the wire");

    // -- ^D ends the line without being buffered — and echoes NOTHING
    //    (the silent EOF path).
    RX.lock().unwrap().extend(b"hi\x04");
    let n = read_cons(&mut buf).expect("caret-D-terminated line");
    assert_eq!(&buf[..n as usize], b"hi", "^D sent the line without a newline");
    assert_eq!(take_echo(), b"hi", "^D echoes nothing");

    // -- WR-03: ^D on an EMPTY line completes a zero-length line — the
    //    read returns 0 (EOF — rc's exit-on-^D at a fresh prompt; the
    //    donor's qwrite(lineq, kbd.line, 0)); the NEXT line still reads.
    //    Both ^Ds stay silent on the wire.
    RX.lock().unwrap().extend(b"\x04");
    let n = read_cons(&mut buf).expect("empty ^D completes a line");
    assert_eq!(n, 0, "an empty ^D line reads as EOF");
    assert_eq!(take_echo(), b"", "an empty ^D echoes nothing");
    RX.lock().unwrap().extend(b"after\n");
    let n = read_cons(&mut buf).expect("the line after the EOF");
    assert_eq!(&buf[..n as usize], b"after\n", "the EOF did not wedge the reader");
    assert_eq!(take_echo(), b"after\r\n");
    // Bytes queued BEFORE the ^D read first; the EOF follows (the donor's
    // block order).
    RX.lock().unwrap().extend(b"first\n\x04");
    let n = read_cons(&mut buf).expect("the queued line reads first");
    assert_eq!(&buf[..n as usize], b"first\n");
    let n = read_cons(&mut buf).expect("then the EOF");
    assert_eq!(n, 0, "the queued ^D completes after the queued bytes");
    assert_eq!(take_echo(), b"first\r\n", "the queued ^D echoed nothing");

    // -- DEL: never buffered; flushes the partial; posts "interrupt" to
    //    the reader's group; the read fails "interrupted"; the shell
    //    survives to answer the next line (the gate's shape).  ECHO: the
    //    typed partial echoed verbatim as drained; the DEL itself is
    //    SILENT (no retro-erasure of the discarded partial).
    SCRIPT.lock().unwrap().extend([b"fnord xyz".to_vec(), b"\x7f".to_vec()]);
    let err = read_cons(&mut buf).expect_err("DEL interrupts the read");
    assert_eq!(err, "interrupted");
    assert_eq!(take_echo(), b"fnord xyz", "DEL echoes nothing (the partial stays on the terminal line)");
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
    assert_eq!(take_echo(), b"echo r9delok\r\n");

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
    //    only while empty — and echo NOTHING (raw mode never echoes).
    SCRIPT.lock().unwrap().extend([b"a\x7f\x08b".to_vec()]);
    let n = read_cons(&mut buf).expect("raw read");
    assert_eq!(&buf[..n as usize], b"a\x7f\x08b", "raw passes DEL/BS verbatim");
    assert!(POSTS.lock().unwrap().len() == 1, "raw DEL posts no note");
    assert_eq!(take_echo(), b"", "raw mode echoes NOTHING");

    // -- rawoff back to cooked; cooked editing + echo resume.
    devcons::write(&mut cc, b"rawoff").expect("rawoff write");
    RX.lock().unwrap().extend(b"z\x08q\n");
    let n = read_cons(&mut buf).expect("cooked again");
    assert_eq!(&buf[..n as usize], b"q\n");
    assert_eq!(take_echo(), b"z\x08 \x08q\r\n", "cooked echo resumes after rawoff");

    // -- WR-01: the natural `echo rawon >#c/consctl` form — the word
    //    arrives NEWLINE-TERMINATED and must still switch (the donor's
    //    strncmp prefix compare); the whole-word compare silently
    //    discarded it.
    devcons::write(&mut cc, b"rawon\n").expect("echo rawon write");
    let n = devcons::read(&mut consctl_chan(OREAD), &mut wbuf, u64::MAX).expect("consctl reads raw");
    assert_eq!(&wbuf[..n as usize], b"raw\n", "newline-terminated rawon switches the mode");
    devcons::write(&mut cc, b"rawoff\n").expect("echo rawoff write");
    let n = devcons::read(&mut consctl_chan(OREAD), &mut wbuf, u64::MAX).expect("consctl reads cooked");
    assert_eq!(&wbuf[..n as usize], b"cooked\n", "newline-terminated rawoff switches back");

    // -- 261104-b4p: a "rawon" flush never RETRO-echoes the partial line.
    //    "abc\ndef" drains in one iteration: "abc\n" completes (echoed
    //    "abc\r\n"), "def" assembles as the in-assembly partial (echoed
    //    verbatim as typed).  The rawon write completes the partial
    //    SILENTLY — no completer was typed, so no CRLF may appear.
    RX.lock().unwrap().extend(b"abc\ndef");
    let n = read_cons(&mut buf).expect("the completed line reads first");
    assert_eq!(&buf[..n as usize], b"abc\n");
    assert_eq!(take_echo(), b"abc\r\ndef", "the partial's bytes echoed as typed");
    devcons::write(&mut cc, b"rawon").expect("rawon flushes the pending partial");
    assert_eq!(take_echo(), b"", "the rawon flush echoes NOTHING (no retro CRLF)");
    devcons::write(&mut cc, b"rawoff").expect("rawoff");
    let n = read_cons(&mut buf).expect("the flushed line reads back");
    assert_eq!(&buf[..n as usize], b"def", "the flushed partial completes WITHOUT an added newline");
    assert_eq!(take_echo(), b"", "the flushed line's return echoes nothing");

    // -- Overrun (T-05-15): the input queue is bounded at 4096 with
    //    drop-oldest; the reader still gets a (capped) completed line.
    //    Echo: only bytes that LAND on the line echo (the LINE_CAP-dropped
    //    excess never re-writes the wire).
    let flood: Vec<u8> = core::iter::repeat(b'a').take(5000).chain([b'\n']).collect();
    RX.lock().unwrap().extend(flood);
    let mut big = [0u8; 2048];
    let n = read_cons(&mut big).expect("overrun read");
    assert_eq!(n, 1025, "the assembled line is LINE_CAP content + newline");
    assert_eq!(big[n as usize - 1], b'\n');
    let echoed = take_echo();
    assert_eq!(echoed.len(), 1024 + 2, "only the landed bytes echo, plus the completer CRLF");
    assert!(echoed[..1024].iter().all(|b| *b == b'a') && &echoed[1024..] == b"\r\n");
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

/// 261104-b4p (item 2 — pin-and-flip; the implementation landed in 05-09):
/// `#c/pid` serves the OPERATING proc's pid as decimal ASCII via the
/// injected current-pid hook (libc getpid's read — init's #p/N/ctl pri
/// write names the real pid).  Pins: the read content, a mid-content
/// offset serving the tail, the streaming read advancing the chan's
/// offset, the OWRITE refusal, and the stat/listing length tracking the
/// content.  Registers set_pid_hook — a SEPARATE set-once static from
/// ConsHooks, so this test is safe beside the scripted suite in one
/// process (neither registration ever runs twice).
#[test]
fn pid_file_serves_the_callers_pid() {
    /// The hook's answer (multi-digit so the offset-tail arm is real).
    const HOOK_PID: u64 = 1049;
    fn test_pid() -> u64 {
        HOOK_PID
    }
    devcons::set_pid_hook(test_pid);

    let root = devcons::attach();
    let c = devcons::walk(&root, b"pid").expect("walk pid");
    assert!(devcons::open(&c, port::dat::OWRITE).is_err(), "pid is read-only (OWRITE rejected)");
    let mut c = devcons::open(&c, OREAD).expect("open pid OREAD");

    // The read returns the hook's pid as decimal ASCII.
    let mut buf = [0u8; 32];
    let n = devcons::read(&mut c, &mut buf, 0).expect("read pid");
    assert_eq!(&buf[..n as usize], b"1049", "the pid reads as decimal ASCII");

    // A read at a mid-content offset serves the tail.
    let n = devcons::read(&mut c, &mut buf, 2).expect("read at offset 2");
    assert_eq!(&buf[..n as usize], b"49", "an offset read serves the tail");

    // A streaming read (offset u64::MAX) uses + advances the chan offset.
    let n = devcons::read(&mut c, &mut buf, u64::MAX).expect("streaming read");
    assert_eq!(&buf[..n as usize], b"1049");
    assert_eq!(c.offset, 4, "the streaming read advanced the chan offset");
    let n = devcons::read(&mut c, &mut buf, u64::MAX).expect("streaming read at end");
    assert_eq!(n, 0, "the advanced offset reads EOF");

    // stat's length tracks the content length (4 ASCII bytes).  The
    // stat(5) wire layout's length field sits at offset 33 (size[2]
    // type[2] dev[4] qid[13] mode[4] atime[4] mtime[4], then length[8]).
    let mut sb = [0u8; 128];
    let n = devcons::stat(&c, &mut sb).expect("stat pid") as usize;
    assert!(n >= 41, "a full stat record");
    let length = u64::from_le_bytes(sb[33..41].try_into().unwrap());
    assert_eq!(length, 4, "stat length tracks the pid content length");

    // The root-dir listing's pid record carries the same length.
    let mut rd = devcons::open(&root, OREAD).expect("open root dir");
    let mut listing = [0u8; 1024];
    let n = devcons::read(&mut rd, &mut listing, u64::MAX).expect("dir read") as usize;
    let mut i = 0usize;
    let mut found = false;
    while i + 2 <= n {
        let sz = u16::from_le_bytes([listing[i], listing[i + 1]]) as usize;
        if sz == 0 {
            break;
        }
        let rec = &listing[i..i + 2 + sz];
        let name_len = u16::from_le_bytes([rec[41], rec[42]]) as usize;
        if &rec[43..43 + name_len] == b"pid" {
            let rlen = u64::from_le_bytes(rec[33..41].try_into().unwrap());
            assert_eq!(rlen, 4, "the listing's pid record carries the content length");
            found = true;
        }
        i += 2 + sz;
    }
    assert!(found, "the root listing carries a pid record");
}
