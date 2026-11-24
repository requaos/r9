//! rfork(2) flag bits: the one definition both arch crates import.
//!
//! Bits 0-6 and 10-14 are Harvey's frozen values (Harvey
//! `sys/include/libc.h:613-624`).  Bits 18-21 are reserved by r9 and
//! exist in no Harvey header.  `RFNOMNT` is defined by Harvey but is not
//! implemented by r9; it and the four reserved flags are in neither
//! arch's `RFORK_SUPPORTED` mask, so rfork rejects them with
//! "rfork: unsupported flags".
//!
//! This file is also compiled into xtask (`#[path]`) to generate
//! `sys/include/rfork.h`, so it must use nothing outside `core` and no
//! `crate::` path.

macro_rules! rfork_flags {
    ($( $(#[$doc:meta])* $name:ident = 1 << $bit:literal; )*) => {
        $( $(#[$doc])* pub const $name: u64 = 1 << $bit; )*
        /// Every flag as (C name, value), in declaration order.
        pub const ALL: &[(&str, u64)] = &[ $( (stringify!($name), $name), )* ];
    };
}

rfork_flags! {
    RFNAMEG = 1 << 0;
    RFENVG = 1 << 1;
    RFFDG = 1 << 2;
    RFNOTEG = 1 << 3;
    RFPROC = 1 << 4;
    RFMEM = 1 << 5;
    RFNOWAIT = 1 << 6;
    RFCNAMEG = 1 << 10;
    RFCENVG = 1 << 11;
    RFCFDG = 1 << 12;
    RFREND = 1 << 13;
    /// Defined by Harvey, not implemented by r9.
    RFNOMNT = 1 << 14;
    /// Reserved (Phase 7).
    RFCKEY = 1 << 18;
    /// Reserved (Phase 7).
    RFROBASE = 1 << 19;
    /// Reserved (Phase 7).
    RFRANDMEM = 1 << 20;
    /// Reserved (Phase 6).
    RFNOLEGACY = 1 << 21;
}

/// Index into `ALL` of the first flag Harvey does not define.
pub const FIRST_R9_ONLY: usize = 12;
