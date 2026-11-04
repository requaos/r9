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
// BusyCell + the MntCore Send proof).  The forbid is a compile-time
// regression lock, not a promise: no new unsafe can enter any other port
// module without deleting its forbid attribute first.
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
pub mod devmnt;
#[forbid(unsafe_code)]
pub mod devpipe;
#[forbid(unsafe_code)]
pub mod devproc;
#[forbid(unsafe_code)]
pub mod devsrv;
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
pub mod pagealloc;
#[forbid(unsafe_code)]
pub mod paqfs;
#[forbid(unsafe_code)]
pub mod pgrp;
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

pub type Result<T> = core::result::Result<T, &'static str>;
