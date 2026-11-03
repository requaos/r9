#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::too_long_first_doc_paragraph)]
#![cfg_attr(not(any(test)), no_std)]
#![feature(allocator_api)]
#![feature(step_trait)]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod allocator;
pub mod bitmapalloc;
pub mod dat;
pub mod devcons;
pub mod devenv;
pub mod devdup;
pub mod devmnt;
pub mod devpipe;
pub mod devproc;
pub mod devsrv;
pub mod devroot;
pub mod fdgrp;
pub mod fdt;
pub mod maths;
pub mod mcslock;
pub mod mem;
pub mod namec;
pub mod pagealloc;
pub mod paqfs;
pub mod pgrp;

pub type Result<T> = core::result::Result<T, &'static str>;
