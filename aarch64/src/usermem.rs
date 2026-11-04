//! User-memory access — the SINGLE validated entry point for every kernel
//! read or write of the calling proc's user address space.
//!
//! Every constructor FUSES the two checks the cold-buffer class made
//! per-site (and therefore forgettable): `Proc::validate_user_range` (the
//! segment-membership proof) and `fault::ensure_user_resident`
//! (explicit zero-page-in so an in-segment but never-touched demand page
//! can never abort the kernel at EL1h mid-access).  After this module, a
//! call site cannot forget residency — the constructor will not hand out
//! anything dereferenceable without both steps.
//!
//! The ONLY raw-pointer/slice materializations of user VAs in the kernel
//! live in this file, each behind its own SAFETY comment.  The invariant
//! they share: the calling proc's TTBR0 is live (we are inside its
//! syscall), so its user VAs are directly dereferenceable by the kernel —
//! the justification the old `user_slice` comment carried, generalized.
//!
//! Zero-behavior-change migration rule: two sites in
//! the measured set historically validated WITHOUT forcing residency — the
//! syscall argument block at SP+8 (always resident by construction: the
//! stub wrote through it to make the call) and NOTED's saved-ureg window
//! (forced resident at delivery time by deliver_notes).  Those use the
//! documented `validated_only` constructors so their exact shape survives.

use alloc::vec::Vec;

use crate::fault;
use crate::proc::Proc;

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
    pub fn new(proc: &mut Proc, va: u64, len: u64) -> Result<UserSlice, &'static str> {
        proc.validate_user_range(va, len, false)?;
        fault::ensure_user_resident(proc, va as usize, va.wrapping_add(len) as usize);
        Ok(UserSlice { ptr: va as *const u8, len: len as usize })
    }

    /// Validate-only variant (documented; zero-behavior-change): for ranges
    /// whose residency is guaranteed by construction — the syscall arg
    /// block at SP+8 (the stub wrote through it) and NOTED's saved-ureg
    /// window (forced resident at delivery).  Never use for a cold buffer.
    pub fn validated_only(proc: &Proc, va: u64, len: u64) -> Result<UserSlice, &'static str> {
        proc.validate_user_range(va, len, false)?;
        Ok(UserSlice { ptr: va as *const u8, len: len as usize })
    }

    /// The materialized read-only slice.  Valid only for the duration of
    /// the owning syscall (the proc's TTBR0 is live in its syscall
    /// context).
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: the constructor validated [ptr, ptr+len) against the
        // calling proc's segment map and (except validated_only, whose
        // residency holds by construction) forced every page resident; the
        // proc's TTBR0 is live for the syscall's duration and its user
        // pages are never unmapped under a live syscall (unmapping happens
        // only at EXEC/EXITS of THIS proc, which diverges).
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Read a single `u64` at byte offset `off` within the range.
    /// Callers bounds-check via the Option (the arg-slot walk's shape).
    pub fn read_u64(&self, off: usize) -> Option<u64> {
        if off.checked_add(8)? > self.len {
            return None;
        }
        // SAFETY: off+8 <= len is checked above; the range was validated +
        // resident at construction; the proc's TTBR0 is live (as_slice's
        // invariant).  User memory is ordinary Normal-cacheable, so an
        // unaligned-unchecked u64 read via read_unaligned is exact.
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
    pub fn new(proc: &mut Proc, va: u64, len: u64) -> Result<UserSliceMut, &'static str> {
        proc.validate_user_range(va, len, true)?;
        fault::ensure_user_resident(proc, va as usize, va.wrapping_add(len) as usize);
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
        // construction; the proc's TTBR0 is live (as_slice_mut's
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
    pub fn validated_only(proc: &Proc, va: u64) -> Result<UserPtr, &'static str> {
        proc.validate_user_range(va, 8, false)?;
        Ok(UserPtr { ptr: va as *const u8 })
    }

    /// Read the pointed-to `u64`.
    pub fn read_u64(&self) -> u64 {
        // SAFETY: the constructor validated [ptr, ptr+8) against the
        // proc's segment map; the argv block's pages are resident by
        // construction (EL0 wrote the slots); the proc's TTBR0 is live.
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
    pub fn copy_from(proc: &Proc, va: u64, max: usize) -> Result<Vec<u8>, &'static str> {
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
            // string); the proc's TTBR0 is live in its syscall context.
            let b = unsafe { ((va + out.len() as u64) as *const u8).read() };
            if b == 0 {
                return Ok(out);
            }
            out.push(b);
        }
    }
}
