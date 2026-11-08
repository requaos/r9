//! User-memory access — the SINGLE validated entry point for every kernel
//! read or write of the calling proc's user address space.
//!
//! Every constructor FUSES the two checks the cold-buffer class made
//! per-site (and therefore forgettable): the segment-membership proof
//! (`validate_user_range`) and explicit zero-page-in
//! (`ensure_user_resident`) so an in-segment but never-touched demand
//! page can never fault the kernel mid-access.  After this module, a
//! call site cannot forget residency — the constructor will not hand
//! out anything dereferenceable without both steps.
//!
//! The ONLY raw-pointer/slice materializations of user VAs in the kernel
//! live in this file, each behind its own SAFETY comment.  The invariant
//! they share (the aarch64 sibling's TTBR0-live justification, re-keyed
//! to x86_64): the calling proc's CR3 is loaded — its user half is
//! visible in kernel context through the per-process PML4 — so its user
//! VAs are directly dereferenceable by the kernel.  This is PROJECT.md's
//! locked per-process live-user-memory model: NO Linux-style copy layer.
//!
//! Zero-behavior-change migration rule: two sites in
//! the measured set historically validated WITHOUT forcing residency — the
//! syscall argument block at SP+8 (always resident by construction: the
//! stub wrote through it to make the call) and NOTED's saved-ureg window
//! (forced resident at delivery time by deliver_notes).  Those use the
//! documented `validated_only` constructors so their exact shape survives.
//!
//! The validation predicate is the `UserAddressSpace` trait — the exact
//! aarch64 `Proc::validate_user_range(va, len, writable)` shape — so this
//! module compiles ahead of the proc port; 14-04's `Proc` implements it
//! (its segment-map walk plugs in without interface churn here).

use alloc::vec::Vec;

/// The fused-constructor predicate: the calling proc's user-address-space
/// checks.  Mirrors the aarch64 pair `Proc::validate_user_range` /
/// `fault::ensure_user_resident(proc, ...)` signature-for-signature; the
/// 14-04 proc port implements this for the real `Proc` (segments from
/// exec, mapping log for residency).
pub trait UserAddressSpace {
    /// Prove `[va, va+len)` lies inside one of the proc's segments (the
    /// segment-membership proof).  `writable` selects the write-intent
    /// flag.  Errors are loud named `&'static str`s.
    fn validate_user_range(&self, va: u64, len: u64, writable: bool) -> Result<(), &'static str>;

    /// Force every page in `[start, end)` resident (explicit zero-page-in
    /// of never-touched demand pages) before anything is dereferenceable.
    fn ensure_user_resident(&mut self, start: usize, end: usize);
}

/// A validated, resident, read-only view of a byte range in the calling
/// proc's user space.  Holds the raw VA + length; `as_slice` materializes
/// the slice on demand so no `&[u8]` with a fabricated lifetime is stored.
pub struct UserSlice {
    ptr: *const u8,
    len: usize,
}

impl UserSlice {
    /// The fused constructor: validate the range against the
    /// proc's segment map, then force its pages resident, before anything
    /// dereferenceable exists.
    pub fn new(
        proc: &mut impl UserAddressSpace,
        va: u64,
        len: u64,
    ) -> Result<UserSlice, &'static str> {
        proc.validate_user_range(va, len, false)?;
        proc.ensure_user_resident(va as usize, va.wrapping_add(len) as usize);
        Ok(UserSlice { ptr: va as *const u8, len: len as usize })
    }

    /// Validate-only variant (documented; zero-behavior-change): for ranges
    /// whose residency is guaranteed by construction — the syscall arg
    /// block at SP+8 (the stub wrote through it) and NOTED's saved-ureg
    /// window (forced resident at delivery).  Never use for a cold buffer.
    pub fn validated_only(
        proc: &impl UserAddressSpace,
        va: u64,
        len: u64,
    ) -> Result<UserSlice, &'static str> {
        proc.validate_user_range(va, len, false)?;
        Ok(UserSlice { ptr: va as *const u8, len: len as usize })
    }

    /// The materialized read-only slice.  Valid only for the duration of
    /// the owning syscall (the proc's CR3 is live in its syscall
    /// context).
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: the constructor validated [ptr, ptr+len) against the
        // calling proc's segment map and (except validated_only, whose
        // residency holds by construction) forced every page resident; the
        // proc's CR3 is live for the syscall's duration — its user half
        // walks the per-process PML4 — and its user pages are never
        // unmapped under a live syscall (unmapping happens only at
        // EXEC/EXITS of THIS proc, which diverges).
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Read a single `u64` at byte offset `off` within the range.
    /// Callers bounds-check via the Option (the arg-slot walk's shape).
    pub fn read_u64(&self, off: usize) -> Option<u64> {
        if off.checked_add(8)? > self.len {
            return None;
        }
        // SAFETY: off+8 <= len is checked above; the range was validated +
        // resident at construction; the proc's CR3 is live (as_slice's
        // invariant).  User memory is ordinary WB-cacheable on x86_64, so
        // an unaligned-unchecked u64 read via read_unaligned is exact.
        Some(unsafe { self.ptr.add(off).cast::<u64>().read_unaligned() })
    }
}

/// A validated, resident, writable view of a byte range in the calling
/// proc's user space (the kernel's write targets: pread buffers, errstr,
/// await records, fd2path, pipe's fd array, stat wire bytes).
pub struct UserSliceMut {
    ptr: *mut u8,
    len: usize,
}

impl UserSliceMut {
    /// The fused constructor (writable variant): validate with the
    /// writable flag, then force resident.
    pub fn new(
        proc: &mut impl UserAddressSpace,
        va: u64,
        len: u64,
    ) -> Result<UserSliceMut, &'static str> {
        proc.validate_user_range(va, len, true)?;
        proc.ensure_user_resident(va as usize, va.wrapping_add(len) as usize);
        Ok(UserSliceMut { ptr: va as *mut u8, len: len as usize })
    }

    /// The materialized writable slice (valid for the syscall's duration —
    /// UserSlice::as_slice's invariant).
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        // SAFETY: UserSlice::as_slice's invariant, plus the range validated
        // writable at construction.
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    /// A read-only view of the same range (fversion's pre-write inspect).
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: as_slice_mut's invariant.
        unsafe { core::slice::from_raw_parts(self.ptr.cast_const(), self.len) }
    }

    /// Write a single `u64` at byte offset `off` within the range (seek's
    /// `*rv` write-back; pipe's fd-pair write uses as_slice_mut).
    pub fn write_u64(&mut self, off: usize, val: u64) -> Option<()> {
        if off.checked_add(8)? > self.len {
            return None;
        }
        // SAFETY: off+8 <= len is checked; validated writable + resident at
        // construction; the proc's CR3 is live (as_slice_mut's
        // invariant).
        unsafe { self.ptr.add(off).cast::<u64>().write_unaligned(val) };
        Some(())
    }
}

/// A validated pointer to a single typed value in user memory (exec's argv
/// slot walk).  Read-only; residency is the caller's documented variant.
pub struct UserPtr {
    ptr: *const u8,
}

impl UserPtr {
    /// Validate-only constructor for one `u64`-sized read (the argv slot
    /// walk: the argv block is user stack/data the process itself wrote —
    /// the same residency-by-construction shape as the arg block).
    pub fn validated_only(proc: &impl UserAddressSpace, va: u64) -> Result<UserPtr, &'static str> {
        proc.validate_user_range(va, 8, false)?;
        Ok(UserPtr { ptr: va as *const u8 })
    }

    /// Read the pointed-to `u64`.
    pub fn read_u64(&self) -> u64 {
        // SAFETY: the constructor validated [ptr, ptr+8) against the
        // proc's segment map; the argv block's pages are resident by
        // construction (ring 3 wrote the slots); the proc's CR3 is live —
        // the user half of its PML4 is visible in kernel context.
        unsafe { self.ptr.cast::<u64>().read_unaligned() }
    }
}

/// Bounded NUL-terminated string copy out of user memory (the
/// `copy_user_str` loop, homed here): each byte's page membership is
/// validated BEFORE the byte is read, the length is bounded at
/// `max`, and the returned bytes EXCLUDE the terminator.  Validate-only
/// per byte — the historical shape (path/arg strings live on pages the
/// process itself wrote).
pub struct UserString;

impl UserString {
    /// Copy the NUL-terminated string at `va` into an owned Vec, excluding
    /// the terminator.  Errors: null pointer, string longer than `max`, or
    /// a byte outside the proc's segments.
    pub fn copy_from(
        proc: &impl UserAddressSpace,
        va: u64,
        max: usize,
    ) -> Result<Vec<u8>, &'static str> {
        let mut out = Vec::new();
        if va == 0 {
            return Err("null string pointer");
        }
        loop {
            if out.len() >= max {
                return Err("string too long");
            }
            proc.validate_user_range(va + out.len() as u64, 1, false)?;
            // SAFETY: this byte's page membership was just validated; the
            // byte is resident by construction (the process wrote the
            // string); the proc's CR3 is live in its syscall context —
            // the user half of its PML4 is visible in kernel context.
            let b = unsafe { ((va + out.len() as u64) as *const u8).read() };
            if b == 0 {
                return Ok(out);
            }
            out.push(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock user address space: ONE contiguous "user segment" backed by
    /// REAL host memory (the buffer's own address range), so the module's
    /// raw materializations genuinely dereference bytes the test wrote —
    /// the aarch64 tests' mock-proc/segments shape, mirrored ahead of
    /// 14-04's Proc port (whose segment walk satisfies the real
    /// predicate).  `resident_calls` records the forced-residency demands
    /// so the fused-vs-validated_only split is asserted, not assumed.
    struct MockUserSpace {
        buf: [u8; 64],
        base: usize,
        resident_calls: Vec<(usize, usize)>,
    }

    impl MockUserSpace {
        fn new() -> MockUserSpace {
            MockUserSpace { buf: [0u8; 64], base: 0, resident_calls: Vec::new() }
        }

        fn va(&self, off: usize) -> u64 {
            (self.base + off) as u64
        }
    }

    impl UserAddressSpace for MockUserSpace {
        fn validate_user_range(
            &self,
            va: u64,
            len: u64,
            _writable: bool,
        ) -> Result<(), &'static str> {
            let Some(end) = va.checked_add(len) else {
                return Err("user range overflows");
            };
            if va >= self.base as u64 && end <= (self.base + self.buf.len()) as u64 {
                Ok(())
            } else {
                Err("bad user address")
            }
        }

        fn ensure_user_resident(&mut self, start: usize, end: usize) {
            self.resident_calls.push((start, end));
        }
    }

    #[test]
    fn fused_constructor_validates_and_forces_residency() {
        let mut mock = MockUserSpace::new();
        // Pin the buffer's address AFTER construction; never move the
        // struct afterwards (the mock's segment IS this memory).
        mock.buf[0..8].copy_from_slice(&0x1122_3344_5566_7788u64.to_le_bytes());
        let mock_ref = &mut mock;
        mock_ref.base = mock_ref.buf.as_ptr() as usize;

        // In-segment: accepted, and residency was forced for the whole
        // range — the fused contract.
        let va0 = mock_ref.va(0);
        let sl = UserSlice::new(mock_ref, va0, 8).expect("in-segment accepted");
        assert_eq!(mock_ref.resident_calls, vec![(mock_ref.base, mock_ref.base + 8)]);
        assert_eq!(sl.read_u64(0), Some(0x1122_3344_5566_7788u64));

        // Out-of-segment: rejected BEFORE any residency demand.
        let before = mock_ref.resident_calls.len();
        let (va64, va60) = (mock_ref.va(64), mock_ref.va(60));
        assert!(UserSlice::new(mock_ref, va64, 8).is_err());
        assert!(UserSlice::new(mock_ref, va60, 8).is_err());
        assert_eq!(mock_ref.resident_calls.len(), before);
    }

    #[test]
    fn validated_only_validates_without_residency() {
        let mut mock = MockUserSpace::new();
        mock.base = mock.buf.as_ptr() as usize;
        let r = &mock;
        // In-segment: accepted with NO forced residency (the documented
        // residency-by-construction variant).
        let sl = UserSlice::validated_only(r, r.va(0), 16).expect("in-segment accepted");
        assert!(r.resident_calls.is_empty());
        assert_eq!(sl.as_slice().len(), 16);

        // Out-of-segment still rejected — validated_only validates.
        assert!(UserSlice::validated_only(r, r.va(63), 2).is_err());
        assert!(UserSlice::validated_only(r, 0x1000_0000, 8).is_err());
    }

    #[test]
    fn read_u64_bounds_are_option_checked() {
        let mut mock = MockUserSpace::new();
        mock.buf[0..8].copy_from_slice(&42u64.to_le_bytes());
        mock.base = mock.buf.as_ptr() as usize;
        let sl = UserSlice::validated_only(&mock, mock.va(0), 12).unwrap();
        assert_eq!(sl.read_u64(0), Some(42));
        assert_eq!(sl.read_u64(4), Some(0)); // bytes 4..12 are zero
        // off+8 > len → None, for every out-of-bounds shape.
        assert_eq!(sl.read_u64(5), None);
        assert_eq!(sl.read_u64(12), None);
        assert_eq!(sl.read_u64(usize::MAX), None);
    }

    #[test]
    fn user_slice_mut_writes_and_bounds() {
        let mut mock = MockUserSpace::new();
        mock.base = mock.buf.as_ptr() as usize;
        // Writable variant: the mock records the writable flag as the same
        // acceptance; the write path is what this test exercises.
        let va16 = mock.va(16);
        let mut wm = UserSliceMut::new(&mut mock, va16, 16).expect("in-segment accepted");
        assert_eq!(mock.resident_calls, vec![(mock.base + 16, mock.base + 32)]);
        assert!(wm.write_u64(0, 0xdead_beef_cafe_f00d).is_some());
        assert_eq!(wm.write_u64(9, 1), None); // off+8 > len
        assert!(wm.write_u64(8, 1).is_some()); // exactly at the end fits
        assert_eq!(wm.as_slice()[0..8], 0xdead_beef_cafe_f00du64.to_le_bytes());
        wm.as_slice_mut().fill(0xaa);
        assert_eq!(wm.as_slice(), &[0xaa; 16]);
    }

    #[test]
    fn user_ptr_validated_read() {
        let mut mock = MockUserSpace::new();
        mock.buf[24..32].copy_from_slice(&0x0bad_c0de_0bad_c0deu64.to_le_bytes());
        mock.base = mock.buf.as_ptr() as usize;
        let p = UserPtr::validated_only(&mock, mock.va(24)).expect("in-segment accepted");
        assert_eq!(p.read_u64(), 0x0bad_c0de_0bad_c0de);
        // One byte short of the u64 → the range leaves the segment → Err.
        assert!(UserPtr::validated_only(&mock, mock.va(57)).is_err());
        assert!(UserPtr::validated_only(&mock, mock.va(64)).is_err());
    }

    #[test]
    fn user_string_copies_until_nul_excluding_it() {
        let mut mock = MockUserSpace::new();
        let s = b"/bin/rc\0trailing";
        mock.buf[..s.len()].copy_from_slice(s);
        mock.base = mock.buf.as_ptr() as usize;
        let out = UserString::copy_from(&mock, mock.va(0), 64).expect("string copies");
        assert_eq!(out, b"/bin/rc");
    }

    #[test]
    fn user_string_null_pointer_and_max_bound() {
        let mut mock = MockUserSpace::new();
        mock.base = mock.buf.as_ptr() as usize;
        // Null pointer: loud named error before any validation.
        assert_eq!(UserString::copy_from(&mock, 0, 64), Err("null string pointer"));
        // No NUL within `max`: the bound fires.
        mock.buf[..32].fill(b'x');
        assert_eq!(UserString::copy_from(&mock, mock.va(0), 32), Err("string too long"));
    }

    #[test]
    fn user_string_validates_each_byte_before_reading() {
        let mut mock = MockUserSpace::new();
        // Fill the WHOLE segment with non-NUL bytes: the string walks to
        // the segment end and the next byte's validation fails — the
        // validate-each-byte loop must error BEFORE reading out of the
        // segment (never a wild read, never a panic).
        mock.buf.fill(b'z');
        mock.base = mock.buf.as_ptr() as usize;
        // max=100 sits above the segment so the LENGTH bound cannot fire
        // first — the boundary byte's validation is what must reject.
        assert_eq!(
            UserString::copy_from(&mock, mock.va(0), 100),
            Err("bad user address")
        );
        // Same shape one byte before the end: 63 z's then the boundary.
        assert_eq!(
            UserString::copy_from(&mock, mock.va(1), 100),
            Err("bad user address")
        );
        // A NUL at the last byte succeeds at the boundary.
        mock.buf[63] = 0;
        let out = UserString::copy_from(&mock, mock.va(0), 64).expect("terminator at the edge");
        assert_eq!(out, &mock.buf[0..63]);
    }
}
