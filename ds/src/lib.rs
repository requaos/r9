#![cfg_attr(not(any(test)), no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

//! `ds/` — data-structure foundations for r9: succinct TPHT, Roaring bitmaps
//! and `ClientSet`.
//!
//! Funnel Hashing is not built in this crate (DS-07; IG-D09, locked
//! 2026-11-24).  Succinct TPHT is the primary concurrent hashing.  If Funnel
//! Hashing is added later it serves write-once ephemeral scratchpads only and
//! never the page cache, `FidTable` or a Venti index.
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
// The CORR-03 slab also carries `#![forbid(unsafe_code)]` as an inner
// attribute at the top of slab.rs itself (Phase 5 D-09).
#[forbid(unsafe_code)]
pub mod slab;
