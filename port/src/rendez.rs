//! rendez — the rendezvous group: 9k's Rgrp (portdat.h) behind the
//! mcslock idiom, shared via `Arc<Lock<RendezGroup>>` on the Proc (the
//! fdgrp/pgrp/env_group field shape).  The group's only contents are the
//! SLEEPER table: procs parked in rendezvous(34) waiting for a tag peer.
//!
//! Semantics per 9k sysrendezvous (sysproc.c:853) — measured into the
//! dispatch set by the 06-07 tier-1 trace (hfactotum's libthread start):
//!
//!   - rendezvous(tag, value): if a sleeper already waits on `tag`, the
//!     caller hands it ITS value (the sleeper wakes and returns it) and
//!     returns the SLEEPER's stored value — a symmetric exchange, the
//!     first caller sleeping.
//!   - No sleeper on `tag`: the caller records (tag, value, pid) and
//!     blocks mid-syscall until a peer's call takes the entry.
//!
//! rfork's RFREND arm (9k sysproc.c: the RFPROC child arm and the
//! non-RFPROC caller re-group): WITH RFREND the child (or, non-RFPROC,
//! the caller) gets a FRESH empty group; WITHOUT it the child SHARES the
//! parent's handle (Arc clone).
//!
//! Pure data + a Leaf-lock consumer — no unsafe, no device access.  The
//! wake half (pid → Proc, rendval write, Blocked → Ready) lives
//! aarch64-side in proc.rs (the PROC_TABLE idiom).

extern crate alloc;

use alloc::vec::Vec;

/// A proc parked in rendezvous: its tag, the value it offered, and its
/// pid (the waker's rendez_wake target).
#[derive(Debug, Clone, Copy)]
pub struct RendezSleeper {
    pub tag: u64,
    pub value: u64,
    pub pid: u64,
}

/// The rendezvous group — the sleeper table, lock-held only briefly in
/// syscall context (a Leaf: never from IRQ context, never across a
/// blocking op; the syscall handler drops it BEFORE sched()).
pub struct RendezGroup {
    pub sleepers: Vec<RendezSleeper>,
}

impl RendezGroup {
    /// A fresh, empty group (Proc construction; the rfork RFREND arms).
    pub fn new() -> RendezGroup {
        RendezGroup { sleepers: Vec::new() }
    }

    /// Take the sleeper waiting on `tag` (removes it — the waker owns the
    /// wake).  None when no sleeper holds the tag.
    pub fn take_sleeper(&mut self, tag: u64) -> Option<RendezSleeper> {
        let i = self.sleepers.iter().position(|s| s.tag == tag)?;
        Some(self.sleepers.remove(i))
    }

    /// Park the caller on `tag` with `value` (the first-caller arm).
    pub fn park(&mut self, tag: u64, value: u64, pid: u64) {
        self.sleepers.push(RendezSleeper { tag, value, pid });
    }

    /// Is `pid` still parked (its entry unconsumed)?  sys_rendezvous's
    /// post-sched() re-verification: r9's wake model is pid-keyed and
    /// FLAT (Blocked→Ready), not channel-keyed like the donor's Rendez,
    /// so a stale waiter-set entry elsewhere (a devip net-waiter
    /// registration left behind by an already-completed op, drained by
    /// a later tick/RX sweep) can ready a parked proc without any peer
    /// pairing — the entry's PRESENCE is the pairing truth.
    pub fn is_parked(&self, pid: u64) -> bool {
        self.sleepers.iter().any(|s| s.pid == pid)
    }

    /// Drop every entry naming `pid` — the exit-time sweep (a killed or
    /// exited sleeper must never linger as a wake target; 9k's pexit
    /// removes the proc from the rendezvous hash the same way).
    pub fn remove_pid(&mut self, pid: u64) {
        self.sleepers.retain(|s| s.pid != pid);
    }
}

impl Default for RendezGroup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn park_take_exchange_and_remove() {
        let mut g = RendezGroup::new();
        // first caller parks; no sleeper on the tag yet
        g.park(0x11, 0xaaaa, 7);
        assert!(g.take_sleeper(0x22).is_none());
        // second caller takes the entry: the exchange's pieces
        let s = g.take_sleeper(0x11).expect("the parked sleeper");
        assert_eq!(s.value, 0xaaaa);
        assert_eq!(s.pid, 7);
        // the table is empty again
        assert!(g.take_sleeper(0x11).is_none());
    }

    #[test]
    fn remove_pid_sweeps_only_that_pid() {
        let mut g = RendezGroup::new();
        g.park(1, 10, 7);
        g.park(2, 20, 8);
        g.park(3, 30, 7);
        g.remove_pid(7);
        assert!(g.take_sleeper(1).is_none());
        assert!(g.take_sleeper(3).is_none());
        assert_eq!(g.take_sleeper(2).expect("pid 8's entry").pid, 8);
    }
}

