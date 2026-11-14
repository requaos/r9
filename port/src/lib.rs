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
// hmacsha1 (18-02, AUTH-03): hand-rolled SHA-1+HMAC in pure safe Rust
// (the devcons ChaCha20 no-new-crate precedent) — devcap's donor cap
// format hashes with it.  The devcap module itself joins in Task 2.
#[forbid(unsafe_code)]
pub mod hmacsha1;
#[forbid(unsafe_code)]
pub mod devroot;
#[forbid(unsafe_code)]
pub mod fdgrp;
#[forbid(unsafe_code)]
pub mod fdt;
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
