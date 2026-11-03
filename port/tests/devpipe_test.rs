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

use port::dat::{Chan, ORDWR};
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

/// The bounded queue: a write that would exceed the 4 KiB bound errors
/// with a NAMED errstr and lands NOTHING (atomic — a partial 9P message
/// would corrupt the stream); the queued bytes drain intact.
#[test]
fn pipe_full_is_a_named_atomic_error() {
    let (mut e0, mut e1) = pipe_pair();
    let chunk = [0xABu8; 1024];
    for _ in 0..(PIPE_Q_CAP / 1024) {
        devpipe::write(&mut e0, &chunk, u64::MAX).expect("write within capacity");
    }
    let err = devpipe::write(&mut e0, b"x", u64::MAX).expect_err("an overflowing write must error");
    assert!(err.contains("full"), "the named overflow errstr, got: {err}");

    let mut buf = vec![0u8; PIPE_Q_CAP];
    let n = devpipe::read(&mut e1, &mut buf, u64::MAX).expect("drain");
    assert_eq!(n as usize, PIPE_Q_CAP, "nothing was lost or added by the refused write");
    assert!(buf.iter().all(|&b| b == 0xAB));
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
}

static SCRIPT: Mutex<VecDeque<Step>> = Mutex::new(VecDeque::new());
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
    }
}

fn test_wake(pids: &[u64]) {
    WAKES.lock().unwrap().push(pids.to_vec());
}

/// Behavior bullet 2: block-on-empty + wake-on-write, then the
/// reader-wakes-on-close arm (a close while parked ends the read at EOF).
#[test]
fn pipe_blocks_on_empty_and_write_wakes_with_bytes() {
    devpipe::set_hooks(PipeHooks { current: test_current, block: test_block, wake: test_wake });

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
