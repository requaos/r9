#![cfg_attr(not(any(test)), no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// Every ds module carries #[forbid(unsafe_code)] EXCEPT the two named
// modules allowed by Phase 3 CONTEXT D-09: `tpht` (the concurrent hash
// table, whose seqlock publication and unchecked reads arrive in plan
// 03-06) and one small, isolated Roaring SIMD module reserved for Phase 5
// (D-20; no code in this phase).  The forbid is a compile-time regression
// lock, not a promise: no new unsafe can enter any other ds module without
// deleting its forbid attribute first.  A third unsafe-holding module in
// ds/ needs the owner's approval.
//
// The reserved SIMD module cannot live under `roaring` once that module is
// forbidden — a forbid reaches submodules — so it will be mounted at the
// crate root with #[path], the same way port/src/lib.rs mounts
// devip/waker.rs.
pub mod tpht;
#[forbid(unsafe_code)]
pub mod roaring;
#[forbid(unsafe_code)]
mod setops;
#[forbid(unsafe_code)]
pub mod clientset;
