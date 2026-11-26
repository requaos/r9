#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_long_first_doc_paragraph)]
#![cfg_attr(not(any(test)), no_std)]
#![feature(allocator_api)]
#![feature(step_trait)]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// Every port module carries
// #[forbid(unsafe_code)] EXCEPT the two named floor modules —
// `globalalloc` (the GlobalAlloc shim: the trait's unsafe fn surface, the
// arena-edge Block constructor, realloc, the QuickFit Sync proof) and
// `mcslock` (the lock/cell primitives: the MCS queue's raw node pointers +
// BusyCell + the MntCore Send proof) — plus the ONE named phase-08 bridge
// module `devip_waker` (the smoltcp RawWaker vtable + Waker::from_raw
// construction — irreducibly unsafe; see its entry below).  The forbid is
// a compile-time regression lock, not a promise: no new unsafe can enter
// any other port module without deleting its forbid attribute first.
#[forbid(unsafe_code)]
pub mod allocator;
#[forbid(unsafe_code)]
pub mod bitmapalloc;
// chan (Phase 4, D-22): the `9P2000.aead` envelope — the 28-byte header, the
// keyed BLAKE3 packet MAC and the per-direction session keys.  The `fid`
// module and its declaration land in plan 04-09 (Phase 4 D-22).
#[forbid(unsafe_code)]
pub mod chan;
#[forbid(unsafe_code)]
pub mod dat;
#[forbid(unsafe_code)]
pub mod devcons;
#[forbid(unsafe_code)]
pub mod devenv;
#[forbid(unsafe_code)]
pub mod devdup;
#[forbid(unsafe_code)]
pub mod devether;
#[forbid(unsafe_code)]
pub mod devip;
// devip_waker (the file is port/src/devip/waker.rs) — the phase-08 ONE
// named unsafe site in the devip work: the smoltcp socket-waker → r9
// wake bridge (D-05).  It is mounted at the crate root via #[path]
// precisely BECAUSE the forbid on `devip` above propagates into
// submodules — as `devip::waker` the named exception would be
// uncompilable.  Every other devip module carries the forbid.  (Beside
// globalalloc/mcslock, the named floor modules — this one is a named
// bridge, not floor; the UNSAFE-AUDIT.md entry carries the site counts.)
#[path = "devip/waker.rs"]
pub mod devip_waker;
#[forbid(unsafe_code)]
pub mod devmnt;
#[forbid(unsafe_code)]
pub mod devpipe;
#[forbid(unsafe_code)]
pub mod devproc;
#[forbid(unsafe_code)]
pub mod devsrv;
// devcap (18-02, AUTH-03) — the `#¤` capability device, a semantics
// port of 9k's devcap.c.  Compiles everywhere (aarch64 included — the
// v2.0-pin freeze bars aarch64 SOURCE changes, not port additions);
// the namec registry arm is cfg(dev_cap)-gated so no frozen config
// ever declares the device.
#[forbid(unsafe_code)]
pub mod devcap;
// hmacsha1 (18-02, AUTH-03): hand-rolled SHA-1+HMAC in pure safe Rust
// (the devcons ChaCha20 no-new-crate precedent) — devcap's donor cap
// format hashes with it.
#[forbid(unsafe_code)]
pub mod hmacsha1;
// rc4 (19-04, DESK-04): the donor-verbatim rc4_256 key class + the
// SSL-format framing arithmetic + dec64 — devssl's algorithm half
// (libsec rc4.c / devssl.c / libc u64.c as the donors of record).
#[forbid(unsafe_code)]
pub mod rc4;
// devssl (19-04, DESK-04) — the `#D` device pushssl hard-codes, a
// semantics port of 9k's devssl.c (the devcap skeleton).  Compiles
// everywhere (aarch64 included — the v2.0-pin freeze bars aarch64
// SOURCE changes, not port additions).
#[forbid(unsafe_code)]
pub mod devssl;
#[forbid(unsafe_code)]
pub mod devroot;
#[forbid(unsafe_code)]
pub mod fdgrp;
#[forbid(unsafe_code)]
pub mod fdt;
// fid (Phase 4, 04-09, D-22; the slab table lands in Phase 5, 05-06, CORR-03):
// the `9P2000.aead` hybrid handle layouts — `StatefulHandle`, `StatelessCap`,
// `FidHandle`, the 56-byte `FidEntry`, the volatility class and the qid codec
// (WIRE-07, WIRE-10, amended by Phase 5 D-07..D-13).  Safe code only: the
// entry is packed field by field with `to_le_bytes`, never transmuted, and the
// `FidTable` wraps the `ds::slab` Boehm seqlock slab under the forbid
// (COMPENDIUM listing 9.6's `PodSlot` + `transmute` is not copied).
#[forbid(unsafe_code)]
pub mod fid;
pub mod globalalloc;
#[forbid(unsafe_code)]
pub mod maths;
pub mod mcslock;
#[forbid(unsafe_code)]
pub mod mem;
#[forbid(unsafe_code)]
pub mod namec;
#[forbid(unsafe_code)]
pub mod netif;
#[forbid(unsafe_code)]
pub mod pagealloc;
#[forbid(unsafe_code)]
pub mod paqfs;
#[forbid(unsafe_code)]
pub mod pgrp;
#[forbid(unsafe_code)]
pub mod rendez;
#[forbid(unsafe_code)]
pub mod rfork;
// sched (Phase 5, 05-08, SMP-11, D-23/D-25/D-26): the portable per-CPU
// work-stealing scheduler run queues — cache-line-aligned
// IrqSpinLock<VecDeque<u64>> queues at LockRank::RunQueue (60), local FIFO
// pop, victim steal via try_lock (never two RunQueue locks held), and the
// idle-cpu bitmask whose enqueue-time consult names the reschedule-IPI
// target.  Host-tested multi-threaded; both arch crates consume it (plans
// 05-09..12).
#[forbid(unsafe_code)]
pub mod sched;
// smp (Phase 5, 05-07, SMP-02/SMP-07/SMP-12, D-25/D-26): the portable SMP
// core — the SmpArch per-CPU hook boundary (registered once by the arch
// crate before CPU 1 starts), the documented LockRank table with
// debug/gate rank assertions and the per-cpu progress watchdog, and the
// lock-free TLB shootdown coordinator.  Unsafe-free: the one pointer
// bridge (the publish-once cell behind register_arch) lives in the
// mcslock floor module.
#[forbid(unsafe_code)]
pub mod smp;
#[forbid(unsafe_code)]
pub mod virtio;

// The allocator's tests live at the crate root (NOT under the
// forbidden allocator module) so the pin tests' arena constructions keep
// their unsafe blocks — the test bodies are byte-verbatim from the
// pre-redesign module; the wave-3 additions pin the tripwire/exhaustion/
// realloc-stress behavior of the index/arena model.
#[cfg(test)]
mod allocator_tests;
// The virtio transport's tests live at the crate root for the same
// reason: the Vec-backed mock arenas and the op-order log need alloc/std
// while port::virtio itself keeps its unsafe-free forbid attribute.
#[cfg(test)]
mod virtio_tests;
// The devether/netif pins live at the crate root for the same reason:
// the frame-store drive and the stub wake log need alloc/std while
// port::devether/port::netif keep their unsafe-free forbid attributes.
#[cfg(test)]
mod devether_tests;
// The RX chain slice-arithmetic pins live at the crate root for the
// same reason: the Vec-backed ring arena and the device-write
// simulation need alloc/std while port::virtio keeps its forbid.
#[cfg(test)]
mod rx_chain_tests;
// The devip seam pins live at the crate root for the same reason: the
// scripted-device drives and the stub wake log need alloc/std while
// port::devip keeps its unsafe-free forbid attribute (and the waker
// bridge keeps its lone documented exception).
#[cfg(test)]
mod devip_tests;
// The namec union pins (17-05, WINDOWS #45) live at the crate root for
// the same reason: the synthetic paqfs image and the stub env-group
// hook need alloc/std while port::namec keeps its unsafe-free forbid
// attribute.
#[cfg(test)]
mod namec_tests;
// The devcap pins (18-02) live at the crate root for the same reason:
// the stub user/tick hooks and the FIFO fixtures need alloc/std while
// port::devcap keeps its unsafe-free forbid attribute.
#[cfg(test)]
mod devcap_tests;
// The devssl pins (19-04) live at the crate root for the same reason:
// the devcons user-seam stub + the pipe loopback fixtures need
// alloc/std while port::devssl keeps its unsafe-free forbid attribute.
#[cfg(test)]
mod devssl_tests;

pub type Result<T> = core::result::Result<T, &'static str>;

/// The shared fixed capacity of every kernel waiter set (WR-05, 17-03):
/// one waiter slot per possible proc.  The port crate cannot import the
/// arch crates' `MAX_PROCS` (they depend on port), so the value is
/// defined HERE, once, and must track the arch tables it mirrors —
/// `x86_64/src/proc.rs` (`MAX_PROCS: usize = 512`) and
/// `aarch64/src/proc.rs` (512 since the v2.0 pin).  Any arch bump of
/// MAX_PROCS must bump this in the same change; the waiter sets' loud
/// full arm (counted + reported, never a silent drop) is the guard
/// that fires if the two ever drift apart under load.  (The dev_churn
/// gate profile raises x86_64's fork BUDGET to 2048 for the churn
/// loop's ~1600 procs; those are not net waiters — and a waiter set
/// that ever fills there reports loudly instead of dropping silently.)
pub const MAX_WAITERS: usize = 512;
