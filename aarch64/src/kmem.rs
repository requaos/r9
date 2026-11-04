use crate::param::KZERO;
use port::mem::{PhysAddr, PhysRange};

// These map to definitions in kernel.ld
unsafe extern "C" {
    #[cfg(platform = "virt")]
    static boottext_pa: [u64; 0];
    static eboottext_pa: [u64; 0];
    static text_pa: [u64; 0];
    static etext_pa: [u64; 0];
    static rodata_pa: [u64; 0];
    static erodata_pa: [u64; 0];
    static data_pa: [u64; 0];
    static edata_pa: [u64; 0];
    static bss_pa: [u64; 0];
    static ebss_pa: [u64; 0];
}

fn base_physaddr() -> PhysAddr {
    // On virt the kernel loads at PA 0x40080000 (qemu -kernel); starting
    // kernel ranges at PA 0 would map >1 GiB as Normal-cacheable kernel text,
    // covering the GIC (0x08000000) and PL011 (0x09000000) MMIO windows -- a
    // speculative-fetch hazard.  Raspi deliberately
    // keeps base 0 (the vminit TODO contemplates an unmapped page 0).
    #[cfg(platform = "virt")]
    {
        boottext_physaddr()
    }
    #[cfg(not(platform = "virt"))]
    {
        PhysAddr::new(0)
    }
}

#[cfg(platform = "virt")]
fn boottext_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(boottext_pa.as_ptr().addr() as u64) }
}

fn eboottext_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(eboottext_pa.as_ptr().addr() as u64) }
}

fn text_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(text_pa.as_ptr().addr() as u64) }
}

fn etext_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(etext_pa.as_ptr().addr() as u64) }
}

fn rodata_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(rodata_pa.as_ptr().addr() as u64) }
}

fn erodata_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(erodata_pa.as_ptr().addr() as u64) }
}

fn data_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(data_pa.as_ptr().addr() as u64) }
}

fn edata_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(edata_pa.as_ptr().addr() as u64) }
}

fn bss_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(bss_pa.as_ptr().addr() as u64) }
}

fn ebss_physaddr() -> PhysAddr {
    // SAFETY: kernel.ld defines this zero-sized marker at exactly the
    // boundary it names; only the symbol's ADDRESS is read (never
    // its contents) — the kmem link-sym class.
    unsafe { PhysAddr::new(ebss_pa.as_ptr().addr() as u64) }
}

pub fn boottext_physrange() -> PhysRange {
    PhysRange::new(base_physaddr(), eboottext_physaddr())
}

pub fn text_physrange() -> PhysRange {
    PhysRange::new(text_physaddr(), etext_physaddr())
}

pub fn rodata_physrange() -> PhysRange {
    PhysRange::new(rodata_physaddr(), erodata_physaddr())
}

pub fn data_physrange() -> PhysRange {
    PhysRange::new(data_physaddr(), edata_physaddr())
}

pub fn bss_physrange() -> PhysRange {
    PhysRange::new(bss_physaddr(), ebss_physaddr())
}

pub fn total_kernel_physrange() -> PhysRange {
    PhysRange::new(base_physaddr(), ebss_physaddr())
}

/// Transform the physical address to a virtual address, under the assumption that
/// the virtual address is the physical address offset from KZERO.
pub const fn physaddr_as_ptr_mut_offset_from_kzero<T>(pa: PhysAddr) -> *mut T {
    (pa.addr() as usize).wrapping_add(KZERO) as *mut T
}

/// Given a virtual address, return the physical address.  Makes a massive assumption
/// that the code is mapped offset to KZERO, so should be used with extreme care.
/// TODO: Remove this altogether, otherwise it'll get used when it shouldn't.
fn from_virt_to_physaddr(va: usize) -> PhysAddr {
    debug_assert!(va >= KZERO, "from_virt_to_physaddr: va {va} must be >= KZERO ({KZERO})");
    PhysAddr::new((va - KZERO) as u64)
}

/// Given an address, return the physical address.  Makes a massive assumption
/// that the code is mapped offset to KZERO, so should be used with extreme care.
pub fn from_ptr_to_physaddr_offset_from_kzero<T>(a: *const T) -> PhysAddr {
    from_virt_to_physaddr(a.addr())
}
