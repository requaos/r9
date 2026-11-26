#![allow(non_upper_case_globals)]

/// Recursive page table implementation for aarch64.
/// Note that currently there are a lot of assumptions that we're dealing with
/// 4KiB tables here, although it supports various sizes of pages.
use crate::{
    kmem::{from_ptr_to_physaddr_offset_from_kzero, physaddr_as_ptr_mut_offset_from_kzero},
    pagealloc,
    param::KZERO,
};
use bitstruct::bitstruct;
use core::{fmt, sync::atomic::Ordering};
use core::{ptr::write_volatile, sync::atomic::AtomicUsize};
use num_enum::{FromPrimitive, IntoPrimitive};
use port::mcslock::{IrqSpinLock, LockNode};
use port::smp::LockRank;
use port::{
    mem::{PAGE_SIZE_1G, PAGE_SIZE_2M, PAGE_SIZE_4K, PhysAddr, PhysRange, VirtRange},
    pagealloc::PageAllocError,
};

/// Serialize Kernel-pgtype `map_to` across CPUs (SMP-03, D-22): the
/// slot-511 recursive dance is a single shared mutable cell on the ONE
/// kernel root table — two CPUs mapping into it concurrently (proc
/// kstack construction runs on any CPU post-SMP; device frames map on
/// the secondary's bring-up CPU) would interleave the
/// save/replace-walk/restore and write PTEs into the wrong physical
/// page.  Rank 25 < PageAlloc (70): `next_mut` allocates the walk's
/// intermediate table pages while the guard is held; every call site
/// holds no other lock.  User-pgtype walks stay unlocked: their dance
/// writes the ACTIVE user table's slot (per-CPU TTBR0).
static KERNEL_VM: IrqSpinLock<()> = IrqSpinLock::new("kernel_vm", LockRank::KernelVm, ());

#[cfg(not(test))]
use port::println;

// The KERNEL_PAGETABLE/USER_PAGETABLE statics are GONE (the
// static-mut elimination).  The kernel root table is never a r9 static —
// vminit's earlyvm page tables own it and ttbr1 names it, so
// kernel_pagetable() resolves through root_page_table()'s TTBR read (the
// guard newtype below, next to it).  The per-process era made
// USER_PAGETABLE dead: user root tables are pagealloc'd per proc;
// user_pagetable()/init_user_page_tables() had zero callers and are
// deleted with the static.

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    Page4K,
    Page2M,
    Page1G,
}

impl PageSize {
    pub const fn size(&self) -> usize {
        match self {
            PageSize::Page4K => PAGE_SIZE_4K,
            PageSize::Page2M => PAGE_SIZE_2M,
            PageSize::Page1G => PAGE_SIZE_1G,
        }
    }
}

#[repr(C, align(4096))]
pub struct PhysPage4K([u8; PAGE_SIZE_4K]);

impl PhysPage4K {
    pub fn clear(&mut self) {
        // SAFETY: self is a live, aligned 4 KiB page (the type's own
        // repr(C, align(4096)) contract); volatile zeroing is the page-
        // table semantics (the write must not be elided even if the page
        // is never read before handoff).
        unsafe {
            core::intrinsics::volatile_set_memory(&mut self.0, 0u8, 1);
        }
    }
}

#[repr(C, align(4096))]
pub struct VirtPage4K(pub [u8; PAGE_SIZE_4K]);

impl VirtPage4K {}

#[derive(Debug, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Mair {
    #[num_enum(default)]
    Normal = 0,
    Device = 1,
}

#[derive(Debug, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum AccessPermission {
    #[num_enum(default)]
    PrivRw = 0,
    AllRw = 1,
    PrivRo = 2,
    AllRo = 3,
}

#[derive(Debug, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Shareable {
    #[num_enum(default)]
    Non = 0, // Non-shareable (single core)
    Unpredictable = 1, // Unpredictable!
    Outer = 2,         // Outer shareable (shared across CPUs, GPU)
    Inner = 3,         // Inner shareable (shared across CPUs)
}

bitstruct! {
    /// AArch64 supports various granule and page sizes.  We assume 48-bit
    /// addresses.  This is documented in the 'Translation table descriptor
    /// formats' section of the Arm Architecture Reference Manual.
    /// The virtual address translation breakdown is documented in the 'Translation
    /// Process' secrtion of the Arm Architecture Reference Manual.
    #[derive(Copy, Clone, PartialEq)]
    #[repr(transparent)]
    pub struct Entry(pub u64) {
        pub valid: bool = 0;
        pub page_or_table: bool = 1;
        pub mair_index: Mair = 2..5;
        pub non_secure: bool = 5;
        pub access_permission: AccessPermission = 6..8;
        pub shareable: Shareable = 8..10;
        pub accessed: bool = 10; // Was accessed by code
        pub addr: u64 = 12..48;
        pub pxn: bool = 53; // Privileged eXecute Never
        pub uxn: bool = 54; // Unprivileged eXecute Never
    }
}

impl Entry {
    pub const fn empty() -> Entry {
        Entry(0)
    }

    pub fn rw_kernel_data() -> Self {
        Entry(0)
            .with_access_permission(AccessPermission::PrivRw)
            .with_shareable(Shareable::Inner)
            .with_accessed(true)
            .with_uxn(true)
            .with_pxn(true)
            .with_mair_index(Mair::Normal)
            .with_valid(true)
    }

    pub fn ro_kernel_data() -> Self {
        Entry(0)
            .with_access_permission(AccessPermission::PrivRo)
            .with_shareable(Shareable::Inner)
            .with_accessed(true)
            .with_uxn(true)
            .with_pxn(true)
            .with_mair_index(Mair::Normal)
            .with_valid(true)
    }

    pub fn ro_kernel_text() -> Self {
        Entry(0)
            .with_access_permission(AccessPermission::PrivRo)
            .with_shareable(Shareable::Inner)
            .with_accessed(true)
            .with_uxn(true)
            .with_pxn(false)
            .with_mair_index(Mair::Normal)
            .with_valid(true)
    }

    pub fn rw_device() -> Self {
        Entry(0)
            .with_access_permission(AccessPermission::PrivRw)
            .with_shareable(Shareable::Inner)
            .with_accessed(true)
            .with_uxn(true)
            .with_pxn(true)
            .with_mair_index(Mair::Device)
            .with_valid(true)
    }

    pub fn rw_user_text() -> Self {
        Entry(0)
            .with_access_permission(AccessPermission::AllRw)
            .with_shareable(Shareable::Inner)
            .with_accessed(true)
            .with_uxn(false)
            .with_pxn(true)
            .with_mair_index(Mair::Normal)
            .with_valid(true)
    }

    pub fn rw_user_data() -> Self {
        Entry(0)
            .with_access_permission(AccessPermission::AllRw)
            .with_shareable(Shareable::Inner)
            .with_accessed(true)
            .with_uxn(true)
            .with_pxn(true)
            .with_mair_index(Mair::Normal)
            .with_valid(true)
    }

    pub const fn with_phys_addr(self, pa: PhysAddr) -> Self {
        Entry(self.0).with_addr(pa.addr() >> 12)
    }

    pub fn phys_addr(self) -> PhysAddr {
        PhysAddr::new(self.addr() << 12)
    }

    pub fn is_table(self, level: Level) -> bool {
        self.page_or_table() && level != Level::Level3
    }
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entry: {:#x} ", self.addr() << 12)?;
        if self.valid() {
            write!(f, " Valid")?;
        } else {
            write!(f, " Invalid")?;
        }
        if self.page_or_table() {
            write!(f, " Page/Table")?;
        } else {
            write!(f, " Block")?;
        }
        write!(f, " {:?}", self.mair_index())?;
        if self.non_secure() {
            write!(f, " NonSecure")?;
        } else {
            write!(f, " Secure")?;
        }
        write!(f, " {:?} {:?}", self.access_permission(), self.shareable())?;
        if self.accessed() {
            write!(f, " Accessed")?;
        }
        if self.pxn() {
            write!(f, " PXN")?;
        }
        if self.uxn() {
            write!(f, " UXN")?;
        }
        Ok(())
    }
}

/// Levels start at the lowest number (most significant) and increase from
/// there.  Four levels would support (for example) 4kiB granules with 4KiB
/// pages using Level0 - Level3, while three would support 2MiB pages with the
/// same size granules, using only Level0 - Level2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Level {
    Level0,
    Level1,
    Level2,
    Level3,
}

impl Level {
    /// Returns the next level to translate
    pub fn next(&self) -> Option<Level> {
        match self {
            Level::Level0 => Some(Level::Level1),
            Level::Level1 => Some(Level::Level2),
            Level::Level2 => Some(Level::Level3),
            Level::Level3 => None,
        }
    }

    pub fn depth(&self) -> usize {
        match self {
            Level::Level0 => 0,
            Level::Level1 => 1,
            Level::Level2 => 2,
            Level::Level3 => 3,
        }
    }
}

pub fn va_index(va: usize, level: Level) -> usize {
    match level {
        Level::Level0 => (va >> 39) & 0x1ff,
        Level::Level1 => (va >> 30) & 0x1ff,
        Level::Level2 => (va >> 21) & 0x1ff,
        Level::Level3 => (va >> 12) & 0x1ff,
    }
}

/// Return the virtual address for the page table at level `level` for the
/// given virtual address, assuming the use of recursive page tables.
fn recursive_table_addr(pgtype: RootPageTableType, va: usize, level: Level) -> usize {
    let indices_mask = 0x0000_ffff_ffff_f000;
    let indices = va & indices_mask;
    let shift = match level {
        Level::Level0 => 36,
        Level::Level1 => 27,
        Level::Level2 => 18,
        Level::Level3 => 9,
    };
    let recursive_indices = match level {
        Level::Level0 => (511 << 39) | (511 << 30) | (511 << 21) | (511 << 12),
        Level::Level1 => (511 << 39) | (511 << 30) | (511 << 21),
        Level::Level2 => (511 << 39) | (511 << 30),
        Level::Level3 => 511 << 39,
    };
    let msbits = match pgtype {
        RootPageTableType::Kernel => 0xffff_0000_0000_0000,
        RootPageTableType::User => 0x0000_0000_0000_0000,
    };
    msbits | recursive_indices | ((indices >> shift) & indices_mask)
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum PageTableError {
    AllocationFailed(PageAllocError),
    EntryIsNotTable,
    PhysRangeIsZero,
    PhysRangeIsNotOnPageBoundary,
    MappingRecursiveIndex,
}

impl From<PageAllocError> for PageTableError {
    fn from(err: PageAllocError) -> PageTableError {
        PageTableError::AllocationFailed(err)
    }
}

#[repr(C, align(4096))]
pub struct Table {
    pub entries: [Entry; 512],
}

impl Table {
    /// Return a mutable entry from the table based on the virtual address and
    /// the level.  (It uses the level to extract the index from the correct
    /// part of the virtual address).
    pub fn entry_mut(&mut self, level: Level, va: usize) -> Result<&mut Entry, PageTableError> {
        let idx = va_index(va, level);
        Ok(&mut self.entries[idx])
    }

    /// Return the next table in the walk.  If it doesn't exist, create it.
    fn next_mut<A: PageAllocator, B: VmTrait>(
        &mut self,
        page_allocator: &mut A,
        vmtrait_impl: &mut B,
        pgtype: RootPageTableType,
        level: Level,
        va: usize,
    ) -> Result<&mut Table, PageTableError> {
        // Try to get a valid page table entry.
        let index = va_index(va, level);
        let mut entry = self.entries[index];

        if entry.valid() && !entry.is_table(level) {
            println!("error:vm:next_mut:entry is not a valid table entry:{entry:?} {level:?}");
            return Err(PageTableError::EntryIsNotTable);
        }

        if entry.valid() {
            // Return the address of the next table
            let next_level = level.next().unwrap();
            // SAFETY: the entry is a valid table entry (checked above);
            // resolve_entry_mut's recursive-address math names the child
            // table, which is live and mapped (this walk created it).
            return Ok(unsafe {
                &mut *vmtrait_impl.resolve_entry_mut::<Table>(entry, pgtype, va, next_level)
            });
        }

        // Create a new page table and write the entry into the parent table
        let page_pa = page_allocator.alloc_physpage();
        let page_pa = match page_pa {
            Ok(p) => p,
            Err(err) => {
                println!("error:vm:next_mut:can't allocate physpage");
                return Err(PageTableError::AllocationFailed(err));
            }
        };
        entry = Entry::rw_kernel_data().with_phys_addr(page_pa).with_page_or_table(true);
        vmtrait_impl.write_entry(&mut self.entries[index], entry);

        // Clear out the new page and return it as the next table
        let next_level = level.next().unwrap();
        let page = vmtrait_impl.resolve_entry_mut::<PhysPage4K>(entry, pgtype, va, next_level);
        // SAFETY: page names the page just allocated + linked above (live,
        // owned by this table, 4 KiB); zeroing it and viewing it as a Table
        // (same size/alignment) is the construction.
        return Ok(unsafe {
            (*page).clear();
            &mut *(page as *mut Table)
        });
    }
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:x}", (self as *const Self).addr())
    }
}

pub enum VaMapping {
    Addr(usize),   // Map to exact virtual address
    Offset(usize), // Map to offset of physical address
}

impl VaMapping {
    pub fn map(&self, pa: PhysAddr) -> usize {
        match self {
            Self::Addr(va) => *va,
            Self::Offset(offset) => (pa.addr() as usize).wrapping_add(*offset),
        }
    }
}

pub type RootPageTable = Table;

impl RootPageTable {
    pub const fn empty() -> RootPageTable {
        RootPageTable { entries: [Entry::empty(); 512] }
    }

    /// Ensure there's a mapping from va to entry, creating any intermediate
    /// page tables that don't already exist.  If a mapping already exists,
    /// replace it.
    /// root_page_table should be a direct va - not a recursive va.
    /// (pub for the exec/fault machinery: exec maps the two seed pages
    /// into a not-yet-active proc table via the temp-recursive dance, and
    /// the fault handler maps demand pages into the ACTIVE user table.)
    pub fn map_to<A: PageAllocator, B: VmTrait>(
        &mut self,
        page_allocator: &mut A,
        vmtrait_impl: &mut B,
        entry: Entry,
        va: usize,
        page_size: PageSize,
        pgtype: RootPageTableType,
    ) -> Result<(), PageTableError> {
        // The Kernel-pgtype dance's cross-CPU serialization (KERNEL_VM's
        // doc above).  The Option guard keeps the lock to end of fn; User
        // walks skip it (per-CPU active table).
        let node = LockNode::new();
        let _kernel_vm_guard =
            matches!(pgtype, RootPageTableType::Kernel).then(|| KERNEL_VM.lock(&node));
        // We change the last entry of the root page table to the address of
        // self for the duration of this method.  This allows us to work with
        // this hierarchy of pagetables even if it's not the current translation
        // table.  We *must* return it to its original state on exit.
        // TODO Only do this if self != kernel_root()
        let temp_recursive_entry = vmtrait_impl.generate_temp_recursive_address(self);
        let old_recursive_entry =
            vmtrait_impl.replace_recursive_entry(pgtype, temp_recursive_entry);

        // Walk + write the destination entry.  The recursive slot-511
        // restore below runs on EVERY exit, success or walk failure:
        // an early Err return that skipped it left the ACTIVE table's slot
        // pointing at this (other) root table, and each later map_to read
        // the corrupted slot as its "old" entry and restored the corruption
        // — the recursive self-map stayed permanently wrong outside the
        // dance.
        let result: Result<(), PageTableError> = (|| {
            let dest_entry = match page_size {
                PageSize::Page4K => self
                    .next_mut(page_allocator, vmtrait_impl, pgtype, Level::Level0, va)
                    .and_then(|t1| {
                        t1.next_mut(page_allocator, vmtrait_impl, pgtype, Level::Level1, va)
                    })
                    .and_then(|t2| {
                        t2.next_mut(page_allocator, vmtrait_impl, pgtype, Level::Level2, va)
                    })
                    .and_then(|t3| t3.entry_mut(Level::Level3, va)),
                PageSize::Page2M => self
                    .next_mut(page_allocator, vmtrait_impl, pgtype, Level::Level0, va)
                    .and_then(|t1| {
                        t1.next_mut(page_allocator, vmtrait_impl, pgtype, Level::Level1, va)
                    })
                    .and_then(|t2| t2.entry_mut(Level::Level2, va)),
                PageSize::Page1G => self
                    .next_mut(page_allocator, vmtrait_impl, pgtype, Level::Level0, va)
                    .and_then(|t1| t1.entry_mut(Level::Level1, va)),
            };
            let dest_entry = match dest_entry {
                Ok(e) => e,
                Err(err) => {
                    println!(
                        "error:vm:map_to:couldn't find page table entry. va:{:#x} err:{:?}",
                        va, err
                    );
                    return Err(err);
                }
            };

            // Entries at level 3 should have the page flag set
            let entry =
                if page_size == PageSize::Page4K { entry.with_page_or_table(true) } else { entry };

            vmtrait_impl.write_entry(dest_entry, entry);
            Ok(())
        })();
        vmtrait_impl.write_recursive_entry(pgtype, old_recursive_entry);
        result
    }

    /// Map the physical range using the requested page size.
    /// This aligns on page size boundaries, and rounds the requested range so
    /// that both the alignment requirements are met and the requested range are
    /// covered.
    /// TODO Assuming some of these requests are dynamic, but should not fail,
    /// we should fall back to the smaller page sizes if the requested size fails.
    pub fn map_phys_range<A: PageAllocator, B: VmTrait>(
        &mut self,
        page_allocator: &mut A,
        vmtrait_impl: &mut B,
        debug_name: &str,
        range: &PhysRange,
        va_mapping: VaMapping,
        entry: Entry,
        page_size: PageSize,
        pgtype: RootPageTableType,
    ) -> Result<VirtRange, PageTableError> {
        if !range.start().is_multiple_of(page_size.size() as u64)
            || !range.end().is_multiple_of(page_size.size() as u64)
        {
            println!(
                "error:vm:map_phys_range:range not on page boundary. debug_name:{debug_name} range:{range} page_size:{page_size:?}",
            );
            return Err(PageTableError::PhysRangeIsNotOnPageBoundary);
        }

        let mut startva = None;
        let mut endva = 0;
        let mut currva = 0;
        for pa in range.step_by_rounded(page_size.size()) {
            if startva.is_none() {
                currva = va_mapping.map(pa);
                startva = Some(currva);
            } else {
                currva += page_size.size();
            }
            endva = currva + page_size.size();
            self.map_to(
                page_allocator,
                vmtrait_impl,
                entry.with_phys_addr(pa),
                currva,
                page_size,
                pgtype,
            )?;
        }
        let mapped_virtrange = startva.map(|startva| VirtRange(startva..endva));
        mapped_virtrange.ok_or(PageTableError::PhysRangeIsZero)
    }
}

// TODO this needs to be a real virtual address allocator...
static next_free_device_page_va: AtomicUsize = AtomicUsize::new(KZERO + 0x100000000000);
pub fn next_free_device_page4k() -> VaMapping {
    next_free_device_page_va
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current + PageSize::Page4K.size())
        })
        .map(VaMapping::Addr)
        .expect("next_free_device_page4k: unable to return new va")
}

/// Reserve a `len`-byte span (4 KiB-rounded up) of the device VA arena —
/// the multi-page counterpart of next_free_device_page4k.  A multi-page
/// map (e.g. the 1 MiB ECAM window, the 64 KiB GICD frame) must reserve
/// its WHOLE span: bumping one page per map lets the next map alias the
/// previous one's tail (the ECAM/GICD overlap the virtio-net bind hit).
pub fn next_free_device_va_span(len: usize) -> VaMapping {
    let len = len.next_multiple_of(PageSize::Page4K.size());
    next_free_device_page_va
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| Some(current + len))
        .map(VaMapping::Addr)
        .expect("next_free_device_va_span: unable to return new va")
}

/// Return the root user or kernel level page table.
///
/// Returns a guard newtype, never a `&'static mut` — the
/// Deref/DerefMut impls hold the raw-alias deref behind the documented
/// invariant (the TTBR-named table is live and KZERO-aliased; no second
/// handle into the same table is live across a caller's use — SMP-03:
/// the user table read is THIS CPU's current TTBR0 (per-CPU by
/// construction), and kernel-table (TTBR1) writes serialize through the
/// KERNEL_VM IrqSpinLock, 05-22).
pub fn root_page_table(pgtype: RootPageTableType) -> RootTableGuard {
    let page_table_pa = match pgtype {
        RootPageTableType::User => ttbr0_el1(),
        RootPageTableType::Kernel => ttbr1_el1(),
    };
    RootTableGuard(physaddr_as_ptr_mut_offset_from_kzero::<RootPageTable>(page_table_pa))
}

/// The kernel root table's guard (the TTBR1-resolution shorthand).
pub fn kernel_pagetable() -> RootTableGuard {
    root_page_table(RootPageTableType::Kernel)
}

/// A guard handle to a live root page table.  Obtained only
/// from root_page_table()/kernel_pagetable(); Deref/DerefMut reach the
/// table so existing method-call sites (`guard.map_phys_range(...)`,
/// `guard.entries[i]`) compile unchanged.
pub struct RootTableGuard(*mut RootPageTable);

impl core::ops::Deref for RootTableGuard {
    type Target = RootPageTable;

    fn deref(&self) -> &RootPageTable {
        // SAFETY: the pointer comes from root_page_table()'s TTBR read —
        // the named root table is live (kernel: vminit's earlyvm tables;
        // user: the current proc's pagealloc'd root) and its KZERO+PA
        // alias is mapped in the kernel table.  The read is this CPU's
        // own TTBR-named table (per-CPU current, or the KERNEL_VM-
        // serialized kernel table); the caller context (boot /
        // IRQ-masked trap/syscall) cannot be preempted into a
        // conflicting access ON THIS CPU.
        unsafe { &*self.0 }
    }
}

impl core::ops::DerefMut for RootTableGuard {
    fn deref_mut(&mut self) -> &mut RootPageTable {
        // SAFETY: Deref's invariant; the guard is short-lived at each call
        // site (no two guards into the same table are live at once).
        unsafe { &mut *self.0 }
    }
}

/// Given an empty, statically allocated page table.  We need to write a
/// recursive entry in the last entry.  To do this, we need to know the physical
/// address, but all we have is the virtual address.  (pub for
/// proc.rs: per-process root tables are pagealloc'd pages written through
/// their KZERO-offset physical alias, which is exactly the pointer shape
/// this function's `from_ptr_to_physaddr_offset_from_kzero` expects.)
pub unsafe fn init_empty_root_page_table(root_page_table: &mut RootPageTable) {
    // SAFETY: the caller contract (a live, writable, KZERO-aliased root
    // table page); the volatile write is the page-table semantics (no
    // elision) — the recursive slot-511 self-entry.
    unsafe {
        let entry = Entry::rw_kernel_data()
            .with_phys_addr(from_ptr_to_physaddr_offset_from_kzero(root_page_table))
            .with_page_or_table(true);
        write_volatile(&mut root_page_table.entries[511], entry);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RootPageTableType {
    User,
    Kernel,
}

/// Return the root user-level page table physical address
fn ttbr0_el1() -> PhysAddr {
    #[cfg(not(test))]
    {
        let mut addr: u64;
        // SAFETY: reading TTBR0_EL1 is a pure register read at EL1 — no
        // memory, no side effects (the MMU-register floor).
        unsafe {
            core::arch::asm!("mrs {value}, ttbr0_el1", value = out(reg) addr);
        }
        PhysAddr::new(addr)
    }
    #[cfg(test)]
    PhysAddr::new(0)
}

/// Return the root kernel page table physical address
fn ttbr1_el1() -> PhysAddr {
    #[cfg(not(test))]
    {
        let mut addr: u64;
        // SAFETY: reading TTBR1_EL1 is a pure register read at EL1 (the
        // MMU-register floor).
        unsafe {
            core::arch::asm!("mrs {value}, ttbr1_el1", value = out(reg) addr);
        }
        PhysAddr::new(addr)
    }
    #[cfg(test)]
    PhysAddr::new(0)
}

// TODO this should just call invalidate_all_tlb_entries afterwards?
#[allow(unused_variables)]
pub unsafe fn switch(page_table: &RootPageTable, pgtype: RootPageTableType) {
    #[cfg(not(test))]
    // SAFETY: the caller contract (page_table names a fully-built root
    // table whose recursive self-entry was written at construction — the
    // switch is the TTBR write + the architected TLB/barrier sequence,
    // no memory accesses beyond the register).
    unsafe {
        let pt_phys = from_ptr_to_physaddr_offset_from_kzero(page_table).addr();
        // https://forum.osdev.org/viewtopic.php?t=36412&p=303237
        //
        // Invariant: the TTBR write must be context-synchronized (isb)
        // BEFORE the tlbi.  Until an isb, the new TTBR value is not
        // guaranteed to be in effect, so table walks in that window —
        // speculative ones included — can still use the outgoing root
        // table, and a TLB entry filled after the tlbi has executed
        // survives the trailing isb.  r9 uses no ASIDs, so the incoming
        // process then runs on the outgoing process's translations for
        // every VA the two share: a fork child ran on its parent's
        // physical pages and rc aborted with `pushword but no argv`
        // (WINDOWS #49; the r9 Harvey phase 20-02 root cause).  Real
        // cores under HVF walk speculatively and hit it; qemu TCG never
        // fills its software TLB speculatively, so TCG never fired.
        //
        // The leading dsb ishst makes every prior page-table store
        // (the incoming process's tables, built by rfork_copy or a
        // fault handler, possibly in pages just freed by another
        // process and reallocated) visible to the table walker before
        // the new TTBR can be used.  Without it a walk may read the
        // stale contents of a reused table page.  The isb fix alone
        // left a residual: a fork child faulted on a corrupt rc argv
        // word under HVF (r9 Harvey phase 20-06, net-smoke run 21).
        match pgtype {
            RootPageTableType::User => {
                core::arch::asm!(
                    "dsb ishst",      // page-table stores visible to the walker
                    "msr ttbr0_el1, {pt_phys}",
                    "isb",            // make the new TTBR0 visible to the walker
                    "tlbi vmalle1is", // invalidate all TLB entries
                    "dsb ish",      // ensure write has completed
                    "isb",          // synchronize context and ensure that no instructions
                                    // are fetched using the old translation
                    pt_phys = in(reg) pt_phys);
            }
            RootPageTableType::Kernel => {
                core::arch::asm!(
                    "dsb ishst",      // same ordering as the TTBR0 arm above
                    "msr ttbr1_el1, {pt_phys}",
                    "isb",            // same ordering as the TTBR0 arm above
                    "tlbi vmalle1is", // invalidate all TLB entries
                    "dsb ish",      // ensure write has completed
                    "isb",          // synchronize context and ensure that no instructions
                                    // are fetched using the old translation
                    pt_phys = in(reg) pt_phys);
            }
        }
    }
}

#[allow(unused_variables)]
pub unsafe fn invalidate_all_tlb_entries() {
    #[cfg(not(test))]
    // SAFETY: dsb ishst + tlbi vmalle1is + dsb ish + isb is the architected
    // full-TLB maintenance sequence — pure barrier and maintenance ops, no
    // memory operands (the TLB/barrier floor; safe at any EL1 point).
    unsafe {
        // https://forum.osdev.org/viewtopic.php?t=36412&p=303237
        // The dsb ishst orders the caller's page-table store before the
        // tlbi, so no walk after the invalidation can refill the old entry.
        core::arch::asm!(
            "dsb ishst",      // the page-table store completes first
            "tlbi vmalle1is", // invalidate all TLB entries
            "dsb ish",        // ensure write has completed
            "isb"             // synchronize context and ensure that no instructions
                              // are fetched using the old translation
        );
    }
}

/// Read CTR_EL0 (the cache-type register) — the ONE home for this
/// system-register read (the
/// register-access floor), previously inline at main.rs + fault.rs.
pub fn ctr_el0() -> u64 {
    #[cfg(not(test))]
    {
        let ctr: u64;
        // SAFETY: reading CTR_EL0 is a pure register read at EL1 — no
        // memory, no side effects.
        unsafe { core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr) };
        ctr
    }
    #[cfg(test)]
    0
}

/// Icache/dcache maintenance after a text page-in, on the USER VA:
/// clean D to the Point of
/// Unification per CTR_EL0.DminLine, DSB ISH, invalidate I to PoU per
/// IminLine, DSB ISH, ISB.  Geometry is ALWAYS discovered from CTR_EL0 —
/// never hardcode 64 (the architectural-discovery rule).
/// Homed here (the TLB/barrier/cache
/// floor) — fault.rs's page-in calls it.
#[allow(unused_variables)]
pub fn flush_icache_user_range(start: usize, end: usize) {
    #[cfg(not(test))]
    // SAFETY: the maintenance sequence on the USER VA range the
    // caller just wrote (mapped r/w); dc cvau / ic ivau / dsb / isb are
    // pure maintenance ops — no memory operands beyond the named range.
    unsafe {
        let ctr = ctr_el0();
        let dline = 4usize << ((ctr >> 16) & 0xf); // DminLine
        let iline = 4usize << (ctr & 0xf); // IminLine
        let mut p = start & !(dline - 1);
        while p < end {
            // clean dcache to Point of Unification
            core::arch::asm!("dc cvau, {}", in(reg) p);
            p += dline;
        }
        core::arch::asm!("dsb ish"); // drains the cleans (Inner-Shareable)
        let mut p = start & !(iline - 1);
        while p < end {
            // invalidate icache to Point of Unification
            core::arch::asm!("ic ivau, {}", in(reg) p);
            p += iline;
        }
        core::arch::asm!("dsb ish");
        core::arch::asm!("isb"); // discard any stale prefetched instructions
    }
}

/// Clean the data cache to the point of unification over [start,
/// start+len) by VA (any mapping of the range — rfork's copy uses the
/// KZERO alias of the destination page), line size from CTR_EL0.DminLine,
/// then `dsb ish` so the cleans complete before any I-cache maintenance.
#[allow(unused_variables)]
pub fn clean_dcache_to_pou(start: usize, len: usize) {
    #[cfg(not(test))]
    // SAFETY: dc cvau + dsb are pure cache maintenance on a range the
    // caller has mapped (no memory operands beyond the named range).
    unsafe {
        let dline = 4usize << ((ctr_el0() >> 16) & 0xf);
        let mut p = start & !(dline - 1);
        while p < start + len {
            core::arch::asm!("dc cvau, {}", in(reg) p);
            p += dline;
        }
        core::arch::asm!("dsb ish");
    }
}

/// Invalidate every instruction cache to the point of unification
/// (Inner Shareable), then synchronize: `ic ialluis; dsb ish; isb`.
/// Needed after instructions are written through a data mapping into
/// physical pages that may still have I-cache lines from a previous use
/// (CTR_EL0.DIC = 0).
pub fn invalidate_icache_all() {
    #[cfg(not(test))]
    // SAFETY: ic ialluis + dsb + isb are pure maintenance/barrier ops —
    // no memory operands; always executable at EL1.
    unsafe {
        core::arch::asm!("ic ialluis", "dsb ish", "isb");
    }
}

pub trait PageAllocator {
    fn alloc_physpage(&mut self) -> Result<PhysAddr, PageAllocError>;
}

#[derive(Clone, Copy)]
pub struct PhysPageAllocator {}

impl PageAllocator for PhysPageAllocator {
    fn alloc_physpage(&mut self) -> Result<PhysAddr, PageAllocError> {
        pagealloc::allocate_physpage()
    }
}

/// I guess we need a better name.  This is to wrap all things that would break if run in a
/// test, or if run before we have the MMU set up.
pub trait VmTrait {
    fn write_entry(&self, dest_entry: &mut Entry, entry: Entry);

    fn replace_recursive_entry(&mut self, pgtype: RootPageTableType, entry: Entry) -> Entry;

    fn write_recursive_entry(&mut self, pgtype: RootPageTableType, entry: Entry);

    fn generate_temp_recursive_address(&self, table: &Table) -> Entry;

    /// Resolve the next table or page from an entry.  In the kernel, this uses recursive
    /// page table addressing.  In tests, it uses the physical address directly
    /// from the entry (which is a valid virtual address in the test environment).
    fn resolve_entry_mut<T>(
        &self,
        entry: Entry,
        pgtype: RootPageTableType,
        va: usize,
        level: Level,
    ) -> *mut T;
}

#[derive(Clone, Copy)]
pub struct VmTraitImpl {}

impl VmTrait for VmTraitImpl {
    fn write_entry(&self, dest_entry: &mut Entry, entry: Entry) {
        // SAFETY: dest_entry is a live entry in a mapped table (the walk's
        // invariant); volatile so the hardware-visible write is never
        // elided, then the TLB maintenance (its own SAFETY comment).
        unsafe {
            write_volatile(dest_entry, entry);
            invalidate_all_tlb_entries();
        }
    }

    fn replace_recursive_entry(&mut self, pgtype: RootPageTableType, entry: Entry) -> Entry {
        let page_table = root_page_table(pgtype);
        let old_recursive_entry = page_table.entries[511];
        self.write_recursive_entry(pgtype, entry);
        old_recursive_entry
    }

    fn write_recursive_entry(&mut self, pgtype: RootPageTableType, entry: Entry) {
        let mut page_table = root_page_table(pgtype);
        // SAFETY: the recursive slot write goes through the guard's
        // DerefMut (root_page_table's invariant); invalidate_all_tlb_entries
        // is the TLB/barrier floor (its own SAFETY comment).
        unsafe {
            // Return the recursive entry to its original state
            write_volatile(&mut page_table.entries[511], entry);
            // TODO Need to invalidate the single cache entry (+ optionally the recursive entry)
            invalidate_all_tlb_entries();
        }
    }

    fn generate_temp_recursive_address(&self, table: &Table) -> Entry {
        let pa = from_ptr_to_physaddr_offset_from_kzero(table);
        Entry::rw_kernel_data().with_phys_addr(pa).with_page_or_table(true)
    }

    fn resolve_entry_mut<T>(
        &self,
        _entry: Entry,
        pgtype: RootPageTableType,
        va: usize,
        level: Level,
    ) -> *mut T {
        recursive_table_addr(pgtype, va, level) as *mut T
    }
}

#[cfg(test)]
mod tests {
    use crate::vmdebug::va_indices;

    use super::*;

    #[test]
    fn can_break_down_va() {
        assert_eq!(va_indices(0xffff8000049fd000), (256, 0, 36, 509));
    }

    #[test]
    fn test_to_use_for_debugging_vaddrs() {
        // assert_eq!(va_indices(0xffffffffffe00000), (256, 0, 36, 509));
        // assert_eq!(va_indices(0xfffffffffff00000), (256, 0, 36, 509));
        // assert_eq!(va_indices(0xffffffffe0000000), (256, 0, 36, 509));
        // assert_eq!(va_indices(0x1000), (0, 0, 0, 1));
    }

    #[test]
    fn test_recursive_table_addr() {
        assert_eq!(va_indices(0xffff800008000000), (256, 0, 64, 0));
        assert_eq!(
            va_indices(recursive_table_addr(
                RootPageTableType::Kernel,
                0xffff800008000000,
                Level::Level0
            )),
            (511, 511, 511, 511)
        );
        assert_eq!(
            va_indices(recursive_table_addr(
                RootPageTableType::Kernel,
                0xffff800008000000,
                Level::Level1
            )),
            (511, 511, 511, 256)
        );
        assert_eq!(
            va_indices(recursive_table_addr(
                RootPageTableType::Kernel,
                0xffff800008000000,
                Level::Level2
            )),
            (511, 511, 256, 0)
        );
        assert_eq!(
            va_indices(recursive_table_addr(
                RootPageTableType::Kernel,
                0xffff800008000000,
                Level::Level3
            )),
            (511, 256, 0, 64)
        );
        assert_eq!(
            va_indices(recursive_table_addr(
                RootPageTableType::Kernel,
                0xffff800008000000,
                Level::Level3
            )),
            (511, 256, 0, 64)
        );
    }

    struct TestPageAllocator {
        free_pages: [PhysPage4K; 4],
        next_page_idx: usize,
    }

    impl TestPageAllocator {
        fn new() -> Self {
            Self {
                free_pages: core::array::from_fn(|_| PhysPage4K([0u8; PAGE_SIZE_4K])),
                next_page_idx: 0,
            }
        }
    }

    impl PageAllocator for TestPageAllocator {
        fn alloc_physpage(&mut self) -> Result<PhysAddr, PageAllocError> {
            if self.next_page_idx < self.free_pages.len() {
                let next_page = &self.free_pages[self.next_page_idx];
                self.next_page_idx += 1;
                Ok(PhysAddr::new(next_page as *const PhysPage4K as u64))
            } else {
                Err(PageAllocError::OutOfSpace)
            }
        }
    }

    struct TestVmTrait {
        root_page_table: RootPageTable,
    }

    impl TestVmTrait {
        fn new() -> Self {
            Self { root_page_table: RootPageTable::empty() }
        }
    }

    impl VmTrait for TestVmTrait {
        fn write_entry(&self, dest_entry: &mut Entry, entry: Entry) {
            *dest_entry = entry;
        }

        fn replace_recursive_entry(&mut self, _pgtype: RootPageTableType, entry: Entry) -> Entry {
            let old_entry = self.root_page_table.entries[511];
            self.root_page_table.entries[511] = entry;
            old_entry
        }

        fn write_recursive_entry(&mut self, _pgtype: RootPageTableType, entry: Entry) {
            self.root_page_table.entries[511] = entry;
        }

        fn generate_temp_recursive_address(&self, table: &Table) -> Entry {
            Entry::rw_kernel_data()
                .with_phys_addr(PhysAddr(table as *const _ as u64))
                .with_page_or_table(true)
        }

        fn resolve_entry_mut<T>(
            &self,
            entry: Entry,
            _pgtype: RootPageTableType,
            _va: usize,
            _level: Level,
        ) -> *mut T {
            // In tests, the "physical" address is actually a valid virtual address
            entry.phys_addr().addr() as *mut T
        }
    }

    #[test]
    fn map_phys_range_test() {
        let mut table = RootPageTable::empty();
        let mut test_page_allocator = TestPageAllocator::new();
        let mut test_vmtrait_impl = TestVmTrait::new();

        let mapped_virtrange = table
            .map_phys_range(
                &mut test_page_allocator,
                &mut test_vmtrait_impl,
                "test",
                &PhysRange::with_end(4096, 8 * 4096),
                VaMapping::Offset(KZERO),
                Entry::rw_kernel_data(),
                PageSize::Page4K,
                RootPageTableType::Kernel,
            )
            .expect("error:init:mapping failed");

        assert_eq!(mapped_virtrange, VirtRange::with_end(KZERO + 4096, KZERO + 8 * 4096));
    }
}
