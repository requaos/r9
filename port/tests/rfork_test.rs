//! Host-side pins for the rfork flag table (DSGN-03, DSGN-04).
//!
//! `port::rfork` is the one definition of the sixteen rfork flags: the
//! eleven Harvey values r9 implements, `RFNOMNT` (defined by Harvey, not
//! implemented by r9), and the four bits r9 reserves (`RFCKEY`,
//! `RFROBASE`, `RFRANDMEM`, `RFNOLEGACY`).  These tests pin the four
//! reserved values, pin the twelve Harvey values literally and in order,
//! and prove all sixteen flags are disjoint single bits.
//!
//! The arch-side half — that `RFNOMNT` and the four reserved bits stay
//! outside each kernel's `RFORK_SUPPORTED` mask — lives in each arch
//! crate's `syscall.rs` (`syscall::rfork_mask_tests`), because the masks
//! stay with the kernels.

use port::rfork::*;

#[test]
fn rfork_reserved_values_are_pinned() {
    assert_eq!(RFCKEY, 1 << 18);
    assert_eq!(RFROBASE, 1 << 19);
    assert_eq!(RFRANDMEM, 1 << 20);
    assert_eq!(RFNOLEGACY, 1 << 21);
}

#[test]
fn rfork_harvey_values_are_pinned() {
    let harvey: [(&str, u64); 12] = [
        ("RFNAMEG", 1 << 0),
        ("RFENVG", 1 << 1),
        ("RFFDG", 1 << 2),
        ("RFNOTEG", 1 << 3),
        ("RFPROC", 1 << 4),
        ("RFMEM", 1 << 5),
        ("RFNOWAIT", 1 << 6),
        ("RFCNAMEG", 1 << 10),
        ("RFCENVG", 1 << 11),
        ("RFCFDG", 1 << 12),
        ("RFREND", 1 << 13),
        ("RFNOMNT", 1 << 14),
    ];
    assert_eq!(&ALL[..FIRST_R9_ONLY], &harvey[..]);
    assert_eq!(RFNOMNT, 1 << 14);
    assert_eq!(ALL.len(), 16);
    assert_eq!(FIRST_R9_ONLY, 12);
}

#[test]
fn rfork_flags_do_not_overlap() {
    let mut seen = 0u64;
    for (name, bit) in ALL {
        assert!(bit.is_power_of_two(), "{name} is not a single bit");
        assert_eq!(seen & bit, 0, "{name} overlaps an earlier flag");
        assert!(*bit <= u32::MAX as u64, "{name} does not fit rfork's int argument");
        seen |= bit;
    }
    assert_eq!(seen, 0x3c7c7f);
}
