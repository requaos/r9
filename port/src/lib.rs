#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_long_first_doc_paragraph)]
#![cfg_attr(not(any(test)), no_std)]
#![feature(allocator_api)]
#![feature(step_trait)]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// 261103-p64 (the regression-lock rollout): every port module carries
// #[forbid(unsafe_code)] EXCEPT the two named floor modules —
// `allocator` (the GlobalAlloc shim; its QuickFit internals are the
// wave-3 redesign target) and `mcslock` (the lock/cell primitives: the
// MCS queue's raw node pointers + BusyCell + the MntCore Send proof).
// The forbid is a compile-time regression lock, not a promise: no new
// unsafe can enter any other port module without deleting its forbid
// attribute first.
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

pub type Result<T> = core::result::Result<T, &'static str>;
