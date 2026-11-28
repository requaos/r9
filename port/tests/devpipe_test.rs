//! Host-side tests for the 05-06 devpipe device (D-04's connectembed
//! substrate): the Arc-shared cross-connected queue pair (9k devpipe.c +
//! qio.c), the blocking read with wake-on-write, and the
//! EOF-on-last-write-close semantics.
//!
//! Semantics are pinned against the 9k donor (sys/src/9k/port/devpipe.c:
//! pipeattach/pipeopen/pipeclose/piperead/pipewrite + qio.c's
//! qread/qwrite/qhangup/qclose/qreopen).  The plan's "single shared
//! queue" prose names the shared PIPE STATE both ends reference — the
//! donor's two-queue direction pair is what paqfs's stdio flow (io(1)
//! reads AND writes fd 1) requires; a single FIFO would let a server
//! consume its own replies.
//!
//! The blocking idiom under test: the arch hooks are scripted test-side
//! (a single-threaded "the other proc acts while we sleep" harness) —
//! the kernel side wires current-pid/sched/wake-pids (04-07 machinery).

use std::sync::Mutex;
use std::collections::VecDeque;

use port::dat::{Chan, ORDWR, OREAD};
use port::devpipe::{self, PipeHooks, PIPE_Q_CAP};
use port::namec;

/// A fresh open pipe pair (data = end 0, data1 = end 1), both ORDWR —
/// the sys_pipe shape.
fn pipe_pair() -> (Chan, Chan) {
    let dir = devpipe::attach();
    let c0 = devpipe::walk(&dir, b"data").expect("walk data");
    let c1 = devpipe::walk(&dir, b"data1").expect("walk data1");
    let e0 = namec::open_chan(&c0, ORDWR).expect("open data");
    let e1 = namec::open_chan(&c1, ORDWR).expect("open data1");
    (e0, e1)
}

/// Behavior bullet 1: a write on one end is read on the other; the two
/// ends share ONE queue per direction — bytes are consumed EXACTLY once
/// (the Pitfall-4 property: a value-copied pipe would give each end its
/// own byte stream and connectembed's 9P exchange would never cross).
#[test]
fn pipe_pair_write_read_round_trip() {
    let (mut e0, mut e1) = pipe_pair();

    let n = devpipe::write(&mut e0, b"hello", u64::MAX).expect("write end0");
    assert_eq!(n, 5);
    let mut buf = [0u8; 16];
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("read end1");
    assert_eq!(&buf[..n as usize], b"hello", "a write on end0 is readable on end1");

    // The reverse direction (paqfs's replies flow end1 → end0).
    devpipe::write(&mut e1, b"ack", u64::MAX).expect("write end1");
    let n = devpipe::read(&mut e0, &mut buf, u64::MAX).expect("read end0");
    assert_eq!(&buf[..n as usize], b"ack", "a write on end1 is readable on end0");

    // Consume-once: a second reader never re-sees prior bytes.
    devpipe::write(&mut e0, b"abcdef", u64::MAX).expect("write");
    let n = devpipe::read(&mut e1, &mut buf[..2], u64::MAX).expect("first read");
    assert_eq!(&buf[..n as usize], b"ab");
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("second read");
    assert_eq!(&buf[..n as usize], b"cdef", "prior bytes are consumed exactly once");
}

/// The bounded queue under the DONOR's blocking write (19-08): a
/// write into a full queue PARKS (whole-message atomic — nothing lands
/// partially) until a reader drains room; the drain WAKES the parked
/// writer; the queued bytes drain intact.  (The old error-on-full
/// shape killed the root paqfs under the tag-mux's concurrent rpcs —
/// the tier1 gate's measured `paqfs: mount write' root death.)
#[test]
fn pipe_full_write_blocks_until_a_reader_drains() {
    let _script = script_guard();
    ensure_hooks();
    let (mut e0, mut e1) = pipe_pair();
    let chunk = [0xABu8; 1024];
    for _ in 0..(PIPE_Q_CAP / 1024) {
        devpipe::write(&mut e0, &chunk, u64::MAX).expect("write within capacity");
    }
    // The overflowing write parks; the scripted step (the PEER end is
    // e1, the reader of q[1]) drains the whole queue, which wakes the
    // parked writer — the write then completes whole.
    *PEER_END.lock().unwrap() = Some(e1);
    SCRIPT.lock().unwrap().push_back(Step::Read(PIPE_Q_CAP));
    WAKES.lock().unwrap().clear();
    devpipe::write(&mut e0, b"x", u64::MAX).expect("the parked write completes after the drain");
    assert_eq!(WAKES.lock().unwrap().as_slice(), &[vec![TEST_PID]], "the drain woke the parked writer");

    let mut e1 = PEER_END.lock().unwrap().take().expect("the scripted peer end");
    let mut buf = [0u8; 8];
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("read the landed byte");
    assert_eq!(&buf[..n as usize], b"x", "the parked write landed WHOLE after the drain — nothing partial, nothing lost");
}

/// A write larger than the queue itself (2026-11-28): the whole-fit test
/// could never pass for it, so the writer parked for good and the reader
/// waited for bytes that never landed (`hsort` flushes 8 KiB stdio blocks
/// into a 4 KiB pipe).  Such a write now lands in pieces, each as big as
/// the room the reader leaves: one park per full piece, one wake per
/// drain, and every byte crosses in order exactly once.
#[test]
fn a_write_larger_than_the_queue_lands_in_pieces_as_the_reader_drains() {
    let _script = script_guard();
    ensure_hooks();
    let (mut e0, e1) = pipe_pair();
    *PEER_END.lock().unwrap() = Some(e1);
    let payload: Vec<u8> = (0..2 * PIPE_Q_CAP + 100).map(|i| (i % 251) as u8).collect();
    // Piece 1 fills the queue and the writer parks; the peer drains it;
    // piece 2 fills it again; the peer drains; the tail lands at once.
    let mut script = SCRIPT.lock().unwrap();
    script.push_back(Step::Collect(PIPE_Q_CAP));
    script.push_back(Step::Collect(PIPE_Q_CAP));
    drop(script);
    let n = devpipe::write(&mut e0, &payload, u64::MAX).expect("the oversized write completes");
    assert_eq!(n as usize, payload.len(), "the call reports the whole write");
    assert_eq!(*BLOCKS.lock().unwrap(), 2, "parked once per full piece, not for the tail");
    assert_eq!(WAKES.lock().unwrap().as_slice(), &[vec![TEST_PID], vec![TEST_PID]], "each drain woke the writer");

    let mut e1 = PEER_END.lock().unwrap().take().expect("the scripted peer end");
    let mut tail = vec![0u8; payload.len()];
    let got = devpipe::read(&mut e1, &mut tail, u64::MAX).expect("read the tail");
    let mut crossed = DRAINED.lock().unwrap().clone();
    crossed.extend_from_slice(&tail[..got as usize]);
    assert_eq!(crossed, payload, "every byte crossed, in order, exactly once");
}

/// The oversized write cut short: the read end closes while the writer is
/// parked behind a landed piece.  The call reports the piece that landed
/// — a count, not an error that would hide it — and never parks again.
#[test]
fn an_oversized_write_cut_by_the_readers_close_reports_what_landed() {
    let _script = script_guard();
    ensure_hooks();
    let (mut e0, e1) = pipe_pair();
    *PEER_END.lock().unwrap() = Some(e1);
    let payload = vec![0x5Au8; PIPE_Q_CAP + 500];
    SCRIPT.lock().unwrap().push_back(Step::Close);
    let n = devpipe::write(&mut e0, &payload, u64::MAX).expect("a short write is not an error");
    assert_eq!(n as usize, PIPE_Q_CAP, "the piece that landed, and no more");
    assert_eq!(*BLOCKS.lock().unwrap(), 1, "parked behind the first piece only");
    assert_eq!(WAKES.lock().unwrap().as_slice(), &[vec![TEST_PID]], "the close woke the parked writer");
    // What follows is an error: the queue is closed now.
    PEER_END.lock().unwrap().take();
    assert_eq!(devpipe::write(&mut e0, b"x", u64::MAX), Err("devpipe: write on closed pipe"));
}

/// A zero-length write is whole by definition: it lands nothing, parks
/// for nothing and reports zero.
#[test]
fn a_zero_length_write_reports_zero_without_parking() {
    let (mut e0, mut e1) = pipe_pair();
    assert_eq!(devpipe::write(&mut e0, b"", u64::MAX), Ok(0));
    let mut buf = [0u8; 16];
    devpipe::write(&mut e0, b"after", u64::MAX).expect("write");
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("read");
    assert_eq!(&buf[..n as usize], b"after", "the empty write left no trace");
}

/// Behavior bullet 3 (drain arm): closing the write end makes the reader
/// drain the remaining bytes and then see EOF (a 0 read) — never a hang.
#[test]
fn pipe_reader_eof_after_drain_on_write_close() {
    let (mut e0, mut e1) = pipe_pair();
    devpipe::write(&mut e0, b"tail", u64::MAX).expect("write");
    namec::close_chan(&e0); // the last write-end close for q[1]
    let mut buf = [0u8; 8];
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("drain");
    assert_eq!(&buf[..n as usize], b"tail", "a hung-up queue drains first");
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("EOF read");
    assert_eq!(n, 0, "closing the write end makes the reader see EOF, not a hang");
}

/// The donor's "write on closed pipe": once the READ end is fully
/// closed, writes error loudly (the note-posting half is 05-08's era).
#[test]
fn pipe_write_on_closed_reader_end_errors() {
    let (mut e0, e1) = pipe_pair();
    namec::close_chan(&e1); // the last read-end close for q[1]
    let err = devpipe::write(&mut e0, b"x", u64::MAX).expect_err("write with no readers must error");
    assert!(err.contains("closed"), "the donor's 'write on closed pipe', got: {err}");
}

/// The donor's qreopen: with BOTH ends closed the queues reset, and the
/// pipe object (still referenced by the dir chan) serves a fresh pair —
/// stale bytes and hangups do not survive the reopen.
#[test]
fn pipe_reopen_after_full_close() {
    let dir = devpipe::attach();
    let (mut e0, e1) = {
        let c0 = devpipe::walk(&dir, b"data").unwrap();
        let c1 = devpipe::walk(&dir, b"data1").unwrap();
        (namec::open_chan(&c0, ORDWR).unwrap(), namec::open_chan(&c1, ORDWR).unwrap())
    };
    devpipe::write(&mut e0, b"stale", u64::MAX).expect("write before close");
    namec::close_chan(&e0);
    namec::close_chan(&e1);

    let c0 = devpipe::walk(&dir, b"data").unwrap();
    let c1 = devpipe::walk(&dir, b"data1").unwrap();
    let mut e0b = namec::open_chan(&c0, ORDWR).expect("reopen data");
    let mut e1b = namec::open_chan(&c1, ORDWR).expect("reopen data1");
    devpipe::write(&mut e0b, b"fresh", u64::MAX).expect("the reopened pipe accepts writes");
    let mut buf = [0u8; 8];
    let n = devpipe::read(&mut e1b, &mut buf, u64::MAX).expect("the reopened pipe reads");
    assert_eq!(&buf[..n as usize], b"fresh", "qreopen: the stale bytes are gone");
}

/// CR-01: the stat(5) wire record is FOUR length-prefixed strings (name,
/// uid, gid, muid — libc's statcheck walks four and rejects a record that
/// ends early, convM2D.c).  Pin the shape devproc_test's parse_dir way:
/// walk all four strings and require the record to END exactly after them
/// (statcheck's `buf != ebuf` rejection).  Hook-free (stat/listing never
/// block), so it is safe beside the scripted blocking suite.
#[test]
fn pipe_stat_records_carry_four_strings() {
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

    let dir = devpipe::attach();
    let mut sb = [0u8; 128];
    let n = devpipe::stat(&dir, &mut sb).expect("stat root") as usize;
    assert_eq!(strings_of(&sb[..n]), ["#|", "glenda", "glenda", "glenda"], "root stat strings");

    let c0 = devpipe::walk(&dir, b"data").expect("walk data");
    let n = devpipe::stat(&c0, &mut sb).expect("stat data") as usize;
    assert_eq!(strings_of(&sb[..n]), ["data", "glenda", "glenda", "glenda"], "end stat strings");

    // The directory listing parses to the last byte in 4-string records
    // (libc dirread's statcheck runs per record).
    let mut rd = namec::open_chan(&dir, OREAD).expect("open root dir");
    let mut listing = [0u8; 512];
    let n = devpipe::read(&mut rd, &mut listing, u64::MAX).expect("dir read") as usize;
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
    assert_eq!(recs, 2, "the listing holds data and data1");
}

// ---------------------------------------------------------------------------
// The blocking idiom (behavior bullet 2): a read on an empty open pipe
// registers the caller as a waiter and blocks; a later write wakes it
// with the bytes; a write-end close wakes it into EOF.  The hooks are
// scripted: the test's block hook plays the peer proc (writing or
// closing) while "we" sleep.  Single test installs the set-once hooks.
// ---------------------------------------------------------------------------

/// What the scripted peer does while the reader is parked.
enum Step {
    /// Write these bytes to the peer's end (end 0).
    Write(Vec<u8>),
    /// Close the peer's end (end 0) — the write side of the reader's queue.
    Close,
    /// Drain `n` bytes off the peer end's read queue (the scripted
    /// reader for 19-08's blocking write: a parked writer completes
    /// only after the drain makes room).
    Read(usize),
    /// As `Read`, keeping what was drained (the scripted reader for an
    /// oversized write: every piece must cross, in order).
    Collect(usize),
}

static SCRIPT: Mutex<VecDeque<Step>> = Mutex::new(VecDeque::new());
static DRAINED: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static PEER_END: Mutex<Option<Chan>> = Mutex::new(None);
static BLOCKS: Mutex<u32> = Mutex::new(0);
static WAKES: Mutex<Vec<Vec<u64>>> = Mutex::new(Vec::new());

const TEST_PID: u64 = 7;

fn test_current() -> u64 {
    TEST_PID
}

fn test_block() {
    *BLOCKS.lock().unwrap() += 1;
    let step = SCRIPT.lock().unwrap().pop_front().expect("block without a scripted step");
    let mut guard = PEER_END.lock().unwrap();
    let peer = guard.as_mut().expect("peer end not installed");
    match step {
        Step::Write(bytes) => {
            devpipe::write(peer, &bytes, u64::MAX).expect("scripted peer write");
        }
        Step::Close => {
            namec::close_chan(peer);
        }
        Step::Read(n) => {
            let mut scratch = vec![0u8; n];
            let got = devpipe::read(peer, &mut scratch, u64::MAX).expect("scripted peer read");
            assert!(got as usize <= n, "the scripted drain stays in bounds");
        }
        Step::Collect(n) => {
            let mut scratch = vec![0u8; n];
            let got = devpipe::read(peer, &mut scratch, u64::MAX).expect("scripted peer read");
            assert!(got as usize <= n, "the scripted drain stays in bounds");
            DRAINED.lock().unwrap().extend_from_slice(&scratch[..got as usize]);
        }
    }
}

fn test_wake(pids: &[u64]) {
    WAKES.lock().unwrap().push(pids.to_vec());
}

/// Behavior bullet 2: block-on-empty + wake-on-write, then the
/// reader-wakes-on-close arm (a close while parked ends the read at EOF).
/// Register the scripted hooks exactly once per process (set_hooks
/// is once-only by assert — the boot's own discipline).
fn ensure_hooks() {
    HOOKS_ONCE.call_once(|| {
        devpipe::set_hooks(PipeHooks {
            current: test_current,
            block: test_block,
            wake: test_wake,
            note_pending: || false,
        });
    });
}
static HOOKS_ONCE: std::sync::Once = std::sync::Once::new();

/// Serialize the scripted tests (the script state is process-global
/// and cargo runs tests in parallel): each scripted test holds this
/// guard for its duration, starting from a clean slate.
fn script_guard() -> std::sync::MutexGuard<'static, ()> {
    static SCRIPT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let g = SCRIPT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    *BLOCKS.lock().unwrap() = 0;
    WAKES.lock().unwrap().clear();
    SCRIPT.lock().unwrap().clear();
    DRAINED.lock().unwrap().clear();
    *PEER_END.lock().unwrap() = None;
    g
}

#[test]
fn pipe_blocks_on_empty_and_write_wakes_with_bytes() {
    let _script = script_guard();
    ensure_hooks();

    // Arm A: the read blocks; the scripted peer write wakes it with bytes.
    let (e0, mut e1) = pipe_pair();
    *PEER_END.lock().unwrap() = Some(e0);
    SCRIPT.lock().unwrap().push_back(Step::Write(b"wakeup".to_vec()));
    let mut buf = [0u8; 16];
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("the woken read returns");
    assert_eq!(&buf[..n as usize], b"wakeup", "the write's bytes crossed to the blocked reader");
    assert_eq!(*BLOCKS.lock().unwrap(), 1, "the read blocked exactly once");
    assert_eq!(WAKES.lock().unwrap().as_slice(), &[vec![TEST_PID]], "the write woke the reader");

    // Arm B: a fresh pipe; the read blocks; the scripted peer CLOSE wakes
    // it into EOF (reader-wakes-on-close).
    let (e0b, mut e1b) = pipe_pair();
    *PEER_END.lock().unwrap() = Some(e0b);
    SCRIPT.lock().unwrap().push_back(Step::Close);
    let n = devpipe::read(&mut e1b, &mut buf, u64::MAX).expect("the close-woken read returns");
    assert_eq!(n, 0, "the write end's close wakes the reader into EOF");
    assert_eq!(*BLOCKS.lock().unwrap(), 2, "the second read blocked once");
    assert_eq!(
        WAKES.lock().unwrap().len(),
        2,
        "the close woke the waiter (a hang would strand it Blocked)"
    );
}
