// The kernel-side churn driver on aarch64 virt (20-09, D-08's arm64
// churn gate): the x86_64 17-02 driver (x86_64/src/platform/q35/
// churngate.rs) ported — fork+exec churn at WINDOWS #44's repro shape
// (~1400 exec iterations of the staged churnmark binary, baked into
// the r9 initrd beside /init by `cargo xtask initrd`) under the SAME
// heap_stats cursor-slope CEILING the x86_64 gate asserts.  The pure
// halves below (the slope logic, the ceiling derivation, the marker
// shapes) are the x86_64 file's verbatim; what differs is the
// bootstrap (the initrd image, not mb2 facts), the binary paths (the
// r9 initrd is flat: /churnmark, /churnsem), the tick rate (virt's
// generic timer ticks at 1000 Hz, q35's at 100 Hz — the watchdogs keep
// their wall-clock budgets) and the scheduler seam (aarch64's
// pick_next trap_returns a fresh exec'd proc into its seeded frame, so
// the children need no first-resume seeding).
//
// THE SLOPE DOCTRINE (port/src/allocator.rs:574-583): the bump cursor
// only ever grows (freed blocks recycle through the quick/misc lists),
// so absolute free bytes prove nothing — the PER-EXEC cursor delta is
// the leak signal.  The gate samples the cursor before the loop, after
// ONE exec, after the baseline leg (BASELINE_N execs), and at the end,
// then asserts the steady-leg slope against CEILING_BYTES_PER_EXEC.
// Moribund-husk accumulation is NOT chased as a leak: slots are never
// reaped this era, the husks pin PROC_TABLE slots (static bytes, not
// heap), and their heap payloads are dropped at exit (exit_tail).
//
// THE CEILING: the x86_64 gate's number, unchanged (T-20-23: a ceiling
// tuned to the aarch64 reading would let a leak report clean).  It was
// derived from x86_64's first measured-green baseline as
// max(3 x steady_per_exec, 1024) + 512; the driver asserts against the
// CONST (fail-closed kernel-side) and prints the slope line the xtask
// post-pass re-derives from.
//
// THE CONTENTION LEG: every SEM_EVERY-th spawn is a churnsem
// generation (RFMEM children contending semacquire/semrelease on a
// shared Data word) spawned WITHOUT waiting, so a generation's
// contenders overlap the execs that follow it.  Presence-marked: the
// generation's own `churngate: semgen` line (printed by the churnsem
// parent from EL0) and the driver's semgens= count in the terminal
// marker.
//
// Marker vocabulary (identical to x86_64's; the xtask consts mirror
// them for the virtchurn profile):
//   churngate: heap <tag> <used> <total>     per-sample cursor line
//   churngate: slope steady=<s> measured=<m> ceiling=<c>
//   churngate: DONE iterations=<n> semgens=<k>   the terminal marker
//   churngate: FAIL <reason>                 fail-closed; DONE withheld
//
// Concurrency: the driver runs as a REAL kernel-context proc (never
// EL0) — bootstrapped by bootstrap_and_run from the dev_exec boot
// window, seeded into the scheduler through its saved swtch context
// (proc::seed_kernel_context), and always IRQ-masked (kernel context on
// aarch64 runs with DAIF.I set; the sched loop's idle_wait unmasks only
// around its own wfi), so its prints need no unmask window.  It blocks
// via proc::sched() — woken by the child's exit-path wait record or by
// tick_timeouts on its sleep_until watchdog — so a wedge fails the gate
// loudly inside the xtask budget, never a bare hang.

#[cfg(dev_churn)]
use crate::platform::timer;

#[cfg(dev_churn)]
use alloc::sync::Arc;
#[cfg(dev_churn)]
use core::cell::SyncUnsafeCell;
#[cfg(dev_churn)]
use port::mcslock::Lock;
#[cfg(dev_churn)]
use port::println;

#[cfg(any(test, dev_churn))]
use alloc::string::String;

/// Total exec iterations (the #44 repro shape: ~1400 fork+exec).
pub const CHURN_N: u32 = 1400;
/// The baseline leg's exec count: samples ride before/after this leg,
/// and the per-exec steady delta derived from it is the ceiling's
/// input (the first exec pays one-time warmup the steady slope must
/// exclude).
pub const BASELINE_N: u32 = 40;
/// Every SEM_EVERY-th spawn is a churnsem contention generation (the
/// driver does not wait for it — the contenders overlap the execs
/// that follow).  1400 / 35 = 40 generations.
pub const SEM_EVERY: u32 = 35;
/// The per-exec cursor-delta ceiling (bytes of bump-cursor growth per
/// exec, steady leg) — the x86_64 gate's const, carried unchanged
/// (20-09's rule: the aarch64 threshold is the x86_64 rule, never a
/// number tuned to the aarch64 result).  Its derivation, from x86_64's
/// first measured-green baseline as max(3 x steady, 1024) + 512:
///
///   first measured-green run (KVM, q35churn, 17-02, post-#44-fix):
///   steady = 420 bytes/exec over the 2..=BASELINE_N leg -> ceiling =
///   max(3 x 420, 1024) + 512 = 1772 bytes/exec.
pub const CEILING_BYTES_PER_EXEC: u64 = 1772;

/// Watchdog for one waited exec child (timer ticks: virt's generic
/// timer runs at 1000 Hz, so 3000 ticks is the x86_64 driver's 3 s of
/// wall clock; a churnmark cycle completes far inside it even under
/// TCG, so expiry names a wedge, never slowness).
#[cfg(dev_churn)]
const WATCH_TICKS: u64 = 3000;
/// The final drain's budget (30 s at 1000 Hz, x86_64's wall clock):
/// every churnsem generation's contenders finish inside it or the gate
/// names them.
#[cfg(dev_churn)]
const DRAIN_TICKS: u64 = 30000;

/// The per-exec cursor delta over `n` execs, saturating (a shrinking
/// cursor is impossible by the doctrine; 0 means flat — no leak).
#[cfg(any(test, dev_churn))]
fn per_exec_delta(before: usize, after: usize, n: u32) -> u64 {
    if n == 0 {
        return 0;
    }
    after.saturating_sub(before) as u64 / n as u64
}

/// The gate verdict: the measured steady-leg per-exec delta against
/// the ceiling.  Err names both numbers — the FAIL line is the whole
/// diagnosis (a leak this gate exists to catch, not a gate bug).
#[cfg(any(test, dev_churn))]
fn slope_ok(measured: u64, ceiling: u64) -> Result<(), String> {
    if measured <= ceiling {
        return Ok(());
    }
    Err(alloc::format!(
        "per-exec cursor delta {measured} over ceiling {ceiling} (unbounded drain — the #44 shape)"
    ))
}

/// The ceiling derivation (documented at CEILING_BYTES_PER_EXEC):
/// max(3 x steady, 1024) + 512.  Pinned as a pure fn so the committed
/// const can be audited against the derivation in one place.
#[cfg(any(test, dev_churn))]
#[cfg_attr(dev_churn, allow(dead_code))]
fn derive_ceiling(steady: u64) -> u64 {
    steady.saturating_mul(3).max(1024) + 512
}

/// One per-sample cursor line.  `tag` names the sample point
/// (start/one/base/end) — the xtask post-pass pairs the base and end
/// samples with the slope line's numbers.
#[cfg(any(test, dev_churn))]
fn heap_marker(tag: &str, used: usize, total: usize) -> String {
    alloc::format!("churngate: heap {tag} {used} {total}")
}

/// The slope line: the steady baseline, the measured steady-leg delta,
/// and the ceiling — every number the post-pass re-derives the verdict
/// from, on the wire.
#[cfg(any(test, dev_churn))]
fn slope_marker(steady: u64, measured: u64, ceiling: u64) -> String {
    alloc::format!("churngate: slope steady={steady} measured={measured} ceiling={ceiling}")
}

/// The terminal marker (presence: the gate's DONE).
#[cfg(any(test, dev_churn))]
fn done_marker(iterations: u32, semgens: u32) -> String {
    alloc::format!("churngate: DONE iterations={iterations} semgens={semgens}")
}

/// Presence-based terminal emission: DONE lands ONLY on a green gate —
/// any failed stage has already printed its churngate: FAIL line and
/// the terminal marker is WITHHELD (fail-closed: the gate reds on the
/// missing DONE, never a silent pass).
#[cfg(any(test, dev_churn))]
fn terminal_marker(gate_ok: bool, iterations: u32, semgens: u32) -> String {
    if gate_ok { done_marker(iterations, semgens) } else { String::new() }
}

/// One fail-closed failure line.
#[cfg(any(test, dev_churn))]
fn fail_marker(reason: &str) -> String {
    alloc::format!("churngate: FAIL {reason}")
}

// --- the kernel driver (dev_churn only; the host battery compiles the
//     pure halves above) ------------------------------------------------

/// The churnmark binary's initrd path (the r9 initrd is flat — the
/// staged binaries sit at the root beside /init).
#[cfg(dev_churn)]
const MARK_PATH: &[u8] = b"/churnmark";
/// The churnsem binary's initrd path.
#[cfg(dev_churn)]
const SEM_PATH: &[u8] = b"/churnsem";

/// The bootstrap handoff: what bootstrap_and_run stashes for
/// churn_main (single writer — the boot context, before the scheduler
/// handoff; single reader — the driver's first resume; the
/// SCHED_CONTEXT seed-once discipline).
#[cfg(dev_churn)]
struct Inputs {
    img: &'static [u8],
    /// The churnmark entry (walked once; PaqDir is a value).
    mark: port::paqfs::PaqDir,
    /// The churnsem entry.
    sem: port::paqfs::PaqDir,
    /// The root attach chan — each child dups it for slash/dot.
    slash: port::dat::Chan,
    /// The seeded ROOT env group (exec_bootstrap's twin).
    root_env: Arc<Lock<port::devenv::EnvGroup>>,
}

#[cfg(dev_churn)]
static INPUTS: SyncUnsafeCell<Option<Inputs>> = SyncUnsafeCell::new(None);

/// Sample the heap cursor and print the tagged line.
#[cfg(dev_churn)]
fn sample(tag: &str) -> usize {
    let (used, total) = crate::allocator::heap_stats();
    println!("{}", heap_marker(tag, used, total));
    used
}

/// The live/pooled census lines: `churngate: census <tag> live <blocks>
/// <bytes>; <size>x<count> ...` + the pooled twin — live classes that
/// climb name the leak; a climbing cursor with flat live classes names
/// fragmentation.
#[cfg(dev_churn)]
fn print_census(tag: &str) {
    let (live, pooled) = crate::allocator::heap_census();
    for (kind, census) in [("live", &live), ("pooled", &pooled)] {
        let blocks: usize = census.iter().map(|&(_, n)| n).sum();
        let bytes: usize = census.iter().map(|&(s, n)| s * n).sum();
        let classes: alloc::vec::Vec<alloc::string::String> =
            census.iter().map(|&(s, n)| alloc::format!("{s}x{n}")).collect();
        println!(
            "churngate: census {tag} {kind} {blocks} blocks {bytes} bytes; {}",
            classes.join(" ")
        );
    }
}

/// The dev_churn boot path (main9's dispatch: dev_exec without
/// dev_bootchain routes here instead of exec_bootstrap).  Everything
/// up to and including the devroot mount is exec_bootstrap's own
/// shared prologue (crate::exec_prologue); this walk-stage splits to the
/// churn entries, then builds the DRIVER proc (a kernel-context
/// orchestrator that never enters EL0, seeded into the scheduler through
/// its saved context) and parks the boot context into the scheduler
/// loop forever.
#[cfg(dev_churn)]
pub fn bootstrap_and_run(initrd_img: Option<&'static [u8]>) -> ! {
    let pro = crate::exec_prologue(initrd_img);
    let root = pro.root;
    let walk = |path: &'static [u8]| -> port::paqfs::PaqDir {
        let mut chan = pro.slash.dup();
        for comp in path.split(|&b| b == b'/').filter(|c| !c.is_empty()) {
            chan = match root.walk(&chan, comp) {
                Ok(c) => c,
                Err(err) => {
                    panic!("churngate: walk {:?}: {err}", core::str::from_utf8(path).unwrap_or("?"))
                }
            };
        }
        let open = match root.open(&chan, port::devroot::OEXEC) {
            Ok(c) => c,
            Err(err) => {
                panic!("churngate: open {:?}: {err}", core::str::from_utf8(path).unwrap_or("?"))
            }
        };
        match root.entry(&open) {
            Ok(bin) => bin,
            Err(err) => {
                panic!("churngate: {:?} entry: {err}", core::str::from_utf8(path).unwrap_or("?"))
            }
        }
    };
    let mark = walk(MARK_PATH);
    let sem = walk(SEM_PATH);

    let mut driver = crate::proc::new_user_proc("churndrv")
        .unwrap_or_else(|err| panic!("churngate: driver proc: {err}"));
    driver.slash = Some(pro.slash.dup());
    driver.dot = Some(pro.slash.dup());
    driver.env_group = pro.root_env.clone();
    // The kernel entry (the SCHED_CONTEXT seeding shape re-keyed onto
    // a proc): the sched loop's swtch resumes into churn_main on the
    // driver's own kstack — never EL0, no seeded TrapFrame.
    crate::proc::seed_kernel_context(&mut driver, churn_main);
    // SAFETY: single writer (this boot context), before the scheduler
    // handoff below; the driver is the only reader.
    unsafe {
        INPUTS.get().write(Some(Inputs {
            img: pro.img,
            mark,
            sem,
            slash: pro.slash,
            root_env: pro.root_env,
        }));
    }
    println!("churngate: driver readied (kernel-context orchestrator)");
    crate::proc::enter_sched_loop_from_boot()
}

/// A tiny argv helper: the binary's leaf name as its own argv[0]
/// (exec_bootstrap's shape).
#[cfg(dev_churn)]
fn leaf_of(path: &[u8]) -> &[u8] {
    let mut leaf = path;
    if let Some(pos) = path.iter().rposition(|&b| b == b'/') {
        leaf = &path[pos + 1..];
    }
    leaf
}

/// The driver body — entered via the seeded context on the driver's
/// kstack, as the current proc, IRQ-masked.  Runs the exec loop, the
/// contention generations, the drain, the slope verdict, and exits
/// through the normal exit tail (the all-exited park prints + halts —
/// the gate reads the capture).
#[cfg(dev_churn)]
fn churn_main() -> ! {
    // SAFETY: the single reader of the bootstrap's one-shot stash.
    let inp = unsafe { (*INPUTS.get()).take().expect("churngate: inputs unstashed") };
    let mark_leaf = leaf_of(MARK_PATH);
    let sem_leaf = leaf_of(SEM_PATH);

    let fail = |reason: &str| -> ! {
        println!("{}", fail_marker(reason));
        finish(false, 0, 0)
    };

    // The sample ladder: start (0 execs), one (1 exec), base
    // (BASELINE_N execs) — the derivation edge measured, not assumed.
    sample("start");
    print_census("start");
    if spawn_and_wait(&inp, false, mark_leaf).is_err() {
        fail("first exec spawn/wait failed");
    }
    let s_one = sample("one");
    print_census("one");
    for _ in 2..=BASELINE_N {
        if spawn_and_wait(&inp, false, mark_leaf).is_err() {
            fail("baseline-leg exec spawn/wait failed");
        }
    }
    let s_base = sample("base");
    let steady = per_exec_delta(s_one, s_base, BASELINE_N - 1);
    // The leak-hunt census at the baseline boundary: which size classes
    // are outstanding after the warmup — compared against the end-of-
    // run census, the accumulating classes name the leak.
    print_census("base");

    // The churn loop proper: iterations BASELINE_N+1 ..= CHURN_N, a
    // churnsem generation on every SEM_EVERY boundary (spawned without
    // waiting — its contenders overlap the execs that follow).
    let mut semgens: u32 = 0;
    for i in (BASELINE_N + 1)..=CHURN_N {
        let sem_gen = i % SEM_EVERY == 0;
        if sem_gen {
            semgens += 1;
            if let Err(err) = spawn_and_wait(&inp, true, sem_leaf) {
                fail(err);
            }
        } else if spawn_and_wait(&inp, false, mark_leaf).is_err() {
            fail("churn-leg exec spawn/wait failed");
        }
    }

    // The drain: every nowait generation's contenders must be gone
    // before the end sample means anything.
    if let Err(err) = drain_stragglers() {
        fail(err);
    }
    let s_end = sample("end");
    print_census("end");
    let measured = per_exec_delta(s_base, s_end, CHURN_N - BASELINE_N);
    println!("{}", slope_marker(steady, measured, CEILING_BYTES_PER_EXEC));
    match slope_ok(measured, CEILING_BYTES_PER_EXEC) {
        Ok(()) => finish(true, CHURN_N, semgens),
        Err(msg) => {
            // The fail-closed verdict: the FAIL line names both numbers
            // (the xtask fail-fast reds on it instantly — DONE alone
            // missing would only surface as a slow budget expiry).
            println!("{}", fail_marker(&msg));
            finish(false, CHURN_N, semgens)
        }
    }
}

/// The terminal tail: the presence-based DONE (or its withholding —
/// the FAIL line already fired), then the driver's own exit through
/// the shared exit path (last-non-Moribund: the all-exited park
/// follows, the clean halt the gate's teardown observes).
#[cfg(dev_churn)]
fn finish(gate_ok: bool, iterations: u32, semgens: u32) -> ! {
    let term = terminal_marker(gate_ok, iterations, semgens);
    if !term.is_empty() {
        println!("{term}");
    }
    let mut me = crate::proc::current().expect("churngate: driver lost its current proc");
    crate::syscall::exit_tail(&mut me, if gate_ok { b"" } else { b"churngate-failed" })
}

/// Spawn one exec child and (unless a contention generation) wait for
/// its exit record.  The child is built exactly like exec_bootstrap's
/// first proc — new_user_proc + slash/dot + the ROOT env group +
/// exec_into — and given THIS driver as its parent.  new_user_proc
/// inserts it Ready (blocked_in_kernel clear), so once exec_into has
/// seeded its frame the sched loop's pick_next trap_returns it into
/// EL0 like any fork child; a contention generation is spawned nowait
/// (no record ever posts; the drain waits out its contenders by table
/// state).
#[cfg(dev_churn)]
fn spawn_and_wait(inp: &Inputs, sem_gen: bool, leaf: &[u8]) -> Result<(), &'static str> {
    let bin = if sem_gen { &inp.sem } else { &inp.mark };
    let name = if sem_gen { "churnsem" } else { "churnmark" };
    let mut child = crate::proc::new_user_proc(name).map_err(|_| "proc table/PMM exhausted")?;
    child.slash = Some(inp.slash.dup());
    child.dot = Some(inp.slash.dup());
    child.env_group = inp.root_env.clone();
    let argv: &[&[u8]] = &[leaf];
    crate::exec::exec_into(&mut child, inp.img, bin, name, argv)
        .map_err(|_| "exec_into failed (staging/seed)")?;
    child.parent = crate::proc::current_proc();
    child.nowait = sem_gen;
    if !sem_gen {
        let mut me = crate::proc::current().expect("churngate: no current proc");
        me.children_live += 1;
        // The AWAIT discipline: block until the child's exit record
        // posts, with a sleep_until watchdog so a wedge fails loud.
        let start = timer::ticks();
        loop {
            if me.wait_records.pop_front().is_some() {
                me.children_live = me.children_live.saturating_sub(1);
                return Ok(());
            }
            if timer::ticks().wrapping_sub(start) > WATCH_TICKS {
                return Err("exec child did not exit inside the budget");
            }
            me.sleep_until = timer::ticks().wrapping_add(WATCH_TICKS);
            crate::proc::sched();
            me.sleep_until = 0;
        }
    }
    Ok(())
}

/// Wait (bounded, sleep-driven) until no non-Moribund proc other than
/// the driver remains — every contention generation's contenders gone
/// before the end sample.
#[cfg(dev_churn)]
fn drain_stragglers() -> Result<(), &'static str> {
    let my_pid = crate::proc::current_pid();
    let start = timer::ticks();
    let mut me = crate::proc::current().expect("churngate: no current proc");
    loop {
        if !crate::proc::any_other_active_procs(my_pid) {
            return Ok(());
        }
        if timer::ticks().wrapping_sub(start) > DRAIN_TICKS {
            return Err("contention contenders did not drain inside the budget");
        }
        me.sleep_until = timer::ticks().wrapping_add(WATCH_TICKS);
        crate::proc::sched();
        me.sleep_until = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A sample series with a FLAT live-allocation delta passes the
    // ceiling.
    #[test]
    fn flat_delta_passes_ceiling() {
        let s0 = 0x10_0000;
        let s1 = s0 + 512; // one exec's warmup
        let s2 = s1; // the baseline leg adds nothing steady
        assert_eq!(per_exec_delta(s1, s2, BASELINE_N), 0, "flat leg has zero slope");
        assert_eq!(per_exec_delta(s0, s0, CHURN_N), 0);
        assert_eq!(per_exec_delta(s0, s0, 0), 0, "zero execs guards the divide");
        assert_eq!(slope_ok(0, CEILING_BYTES_PER_EXEC), Ok(()));
    }

    // ...and a series whose per-exec delta exceeds the ceiling fails
    // it, naming both numbers (the FAIL line is the diagnosis).
    #[test]
    fn over_ceiling_delta_fails() {
        let ceiling = 4096u64;
        assert_eq!(slope_ok(ceiling, ceiling), Ok(()), "the boundary itself passes");
        let verdict = slope_ok(ceiling + 1, ceiling);
        assert!(verdict.is_err(), "one byte over the ceiling must fail");
        let msg = verdict.unwrap_err();
        assert!(msg.contains("4097"), "the FAIL line names the measured delta: {msg}");
        assert!(msg.contains("4096"), "the FAIL line names the ceiling: {msg}");
    }

    // The derivation formula the committed ceiling documents — pinned
    // so the const can be audited against it mechanically.
    #[test]
    fn derive_ceiling_covers_steady_and_jitter() {
        assert_eq!(derive_ceiling(0), 1024 + 512, "a zero baseline still leaves the jitter floor");
        assert_eq!(derive_ceiling(100), 1024 + 512, "the floor still wins at 3x100 < 1024");
        assert_eq!(derive_ceiling(400), 3 * 400 + 512, "3x crosses the floor at 342");
        assert_eq!(derive_ceiling(5000), 3 * 5000 + 512);
        assert!(derive_ceiling(10_000) > 3 * 10_000 - 512);
        // monotone in the baseline
        assert!(derive_ceiling(7) < derive_ceiling(7000));
        // the committed ceiling IS the x86_64 gate's derivation of its
        // first measured-green baseline (420 bytes/exec, 17-02's KVM
        // run) — 20-09 carries it unchanged, never re-derived from an
        // aarch64 reading
        assert_eq!(CEILING_BYTES_PER_EXEC, derive_ceiling(420));
        assert_eq!(CEILING_BYTES_PER_EXEC, 1772, "the x86_64 driver's const");
    }

    // The marker shapes the xtask constants grep and the post-pass
    // parses — pinned exactly, both sides key off them.
    #[test]
    fn marker_shapes_are_exact() {
        assert_eq!(done_marker(1400, 40), "churngate: DONE iterations=1400 semgens=40");
        assert_eq!(heap_marker("start", 100, 200), "churngate: heap start 100 200");
        assert_eq!(slope_marker(1, 2, 3), "churngate: slope steady=1 measured=2 ceiling=3");
        assert_eq!(
            fail_marker("exec child did not exit"),
            "churngate: FAIL exec child did not exit"
        );
    }

    // Presence-based terminal emission: the terminal marker lands ONLY
    // on a green gate — on failure it is withheld (the FAIL line
    // already fired) and the gate reds on the missing DONE.
    #[test]
    fn terminal_marker_withheld_on_failure() {
        assert_eq!(terminal_marker(true, 1400, 40), "churngate: DONE iterations=1400 semgens=40");
        assert_eq!(terminal_marker(false, 1400, 40), "", "DONE is withheld on a failed gate");
    }
}
