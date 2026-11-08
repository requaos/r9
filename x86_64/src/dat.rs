pub use crate::seg::{Gdt, Idt, Tss};

use bitstruct::bitstruct;
use port::dat as portdat;
use zerocopy::FromZeros;

pub const KCPUZERO: usize = 0xffff_ff00_0000_0000;
pub const KMACH: usize = KCPUZERO + 0x0010_0000;

// Slot indices into the 184-byte Ureg (the D-01 frozen layout):
// the 15 GP qwords occupy slots 0-14, the packed u16 segment
// quartet shares slot 15 (one qword at byte 120), and `type`
// sits at slot 16 (byte 128) with cs at slot 19 (byte 152).
pub const UREG_TRAPNO_OFFSET: usize = 16 * core::mem::size_of::<u64>();
pub const UREG_CS_OFFSET: usize = 19 * core::mem::size_of::<u64>();

// The SYSCALL entry asm's remaining slot offsets (trap.rs, 14-02) —
// computed from the struct like the two above, so the asm's literals
// are witnessed against the layout itself, not a restated number.
pub const UREG_BP_OFFSET: usize = core::mem::offset_of!(Ureg, bp);
pub const UREG_DS_OFFSET: usize = core::mem::offset_of!(Ureg, ds);
pub const UREG_SP_OFFSET: usize = core::mem::offset_of!(Ureg, sp);

/// The user register and trap frame structure.
///
/// This stores both user state during system calls, and
/// and trap frames for any kind of interrupt, exception,
/// or fault.
///
/// Phase 14 D-01: this layout is byte-verbatim Harvey's
/// `amd64/include/ureg.h` (published as `amd64/ABI.md`) —
/// 184 bytes, no translation anywhere, so the same 6.outs
/// run on both the 9k and r9 kernels.  Drift between this
/// struct and the header is a build failure (the const
/// asserts below plus the xtask ureg-check), never a
/// runtime mystery.
///
/// Warning: The format of this structure is (necessarily)
/// known to assembly language.  Caveat emptor.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct Ureg {
    // Pushed by software.  ax/bp/sp are pub(crate) — the SYSCALL wire
    // slots the dispatch reads/writes (result in ax, the number in bp,
    // the argument block at sp+8 — amd64/ABI.md's contract).
    pub(crate) ax: u64,
    bx: u64,
    cx: u64,
    dx: u64,
    si: u64,
    di: u64,
    pub(crate) bp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,

    // The x86 segmentation registers.  Aside from %fs and
    // %gs these are effectively unused in long mode, but
    // they exist, so we save and restore them (some kernels
    // do this, some do not; %fs and %gs are special).
    //
    // ureg.h declares them u16, packed into ONE qword at
    // offset 120 — ds@120, es@122, fs@124, gs@126 — with
    // no padding before `type` (120+8 == 128).
    ds: u16,
    es: u16,
    fs: u16,
    gs: u16,

    // ureg.h names this field `type`; it carries the trap
    // vector (the stub-encoded return address on the IDT
    // path).  `type` is a Rust keyword, so the field is
    // `type_` here — the ABI table uses the header's name.
    type_: u64,

    // Sometimes pushed by hardware.
    pub ecode: u64,

    // Pushed by hardware.  pc/cs/flags/ss are pub(crate) for the note
    // machinery (the handler redirect rewrites pc; the post-restore
    // privilege force rewrites cs/ss/flags — syscall.rs's
    // force_user_return).
    pub pc: u64,
    pub(crate) cs: u64,
    pub(crate) flags: u64,
    pub(crate) sp: u64,
    pub(crate) ss: u64,
}

// Pinned byte offsets from Harvey's amd64/include/ureg.h, the
// D-01 frozen source of truth (published as amd64/ABI.md).
// Always on — any layout drift fails `cargo build`, no Harvey
// checkout needed (the xtask ureg-check covers the header itself).const _: () = assert!(core::mem::offset_of!(Ureg, ax) == 0);
const _: () = assert!(core::mem::offset_of!(Ureg, bp) == 48);
const _: () = assert!(core::mem::offset_of!(Ureg, r15) == 112);
const _: () = assert!(core::mem::offset_of!(Ureg, ds) == 120);
const _: () = assert!(core::mem::offset_of!(Ureg, es) == 122);
const _: () = assert!(core::mem::offset_of!(Ureg, fs) == 124);
const _: () = assert!(core::mem::offset_of!(Ureg, gs) == 126);
const _: () = assert!(core::mem::offset_of!(Ureg, type_) == 128);
const _: () = assert!(core::mem::offset_of!(Ureg, ecode) == 136);
const _: () = assert!(core::mem::offset_of!(Ureg, pc) == 144);
const _: () = assert!(core::mem::offset_of!(Ureg, cs) == 152);
const _: () = assert!(core::mem::offset_of!(Ureg, flags) == 160);
const _: () = assert!(core::mem::offset_of!(Ureg, sp) == 168);
const _: () = assert!(core::mem::offset_of!(Ureg, ss) == 176);
const _: () = assert!(core::mem::size_of::<Ureg>() == 184);

impl Ureg {
    /// The all-zero frame (the aarch64 TrapFrame::zeroed idiom — the
    /// exec seed's starting state before the entry fields are planted).
    pub const fn zeroed() -> Ureg {
        Ureg {
            ax: 0,
            bx: 0,
            cx: 0,
            dx: 0,
            si: 0,
            di: 0,
            bp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            ds: 0,
            es: 0,
            fs: 0,
            gs: 0,
            type_: 0,
            ecode: 0,
            pc: 0,
            cs: 0,
            flags: 0,
            sp: 0,
            ss: 0,
        }
    }

    /// Seed the exec entry state — the 9k donor's sysexecregs (k10
    /// syscall.c: `ureg->sp`, `ureg->ip`, `ureg->type = 64` — the
    /// "fiction for acid"; the returned Tos address rides bp, the 6c
    /// RARG/first-arg register that is the aarch64 x0 analog) plus the
    /// ring-3 context fields 14-02's touser/syscallreturn consume on
    /// entry: the flat user selectors (cs/ss = utext/udata|RPL3, the
    /// packed quartet = udata|RPL3) and flags = If — the user enters
    /// with IF=1, the unmask-at-handoff discipline.  Everything else
    /// stays zeroed.
    pub fn seed_exec_entry(&mut self, tos_va: u64, user_sp: u64, entry: u64) {
        const IF: u64 = 0x200; // If — touser's R11 plant
        self.bp = tos_va;
        self.sp = user_sp;
        self.pc = entry;
        self.flags = IF;
        self.cs = crate::trap::USER_TEXT_SEL as u64;
        self.ss = crate::trap::USER_DATA_SEL as u64;
        self.ds = crate::trap::USER_DATA_SEL;
        self.es = crate::trap::USER_DATA_SEL;
        self.fs = crate::trap::USER_DATA_SEL;
        self.gs = crate::trap::USER_DATA_SEL;
        self.type_ = 64;
    }

    /// The note-delivery frame push: the live frame's whole 184 bytes
    /// (the ABI sizeof — on amd64 the kernel frame IS the wire frame,
    /// no stride mismatch like aarch64's 280-vs-288) into the
    /// validated, resident user window.  Homed on the frame's own
    /// module so callers carry no frame-reinterpretation unsafe.
    pub(crate) fn copy_to_saved(&self, dst: &mut [u8]) {
        assert_eq!(dst.len(), core::mem::size_of::<Ureg>(), "saved ureg size");
        // SAFETY: self is a live Ureg; exactly 184 bytes are read;
        // dst is validated + resident (usermem's constructor
        // invariant).  Overlap is impossible (src kernel, dst user).
        unsafe {
            core::ptr::copy_nonoverlapping(
                self as *const Ureg as *const u8,
                dst.as_mut_ptr(),
                core::mem::size_of::<Ureg>(),
            );
        }
    }

    /// NOTED's NCONT/NRSTR restore: overwrite the live frame with the
    /// saved Ureg's 184 bytes — the whole frame, since the sizes agree
    /// on amd64.
    pub(crate) fn restore_from_saved(&mut self, src: &[u8]) {
        assert_eq!(src.len(), core::mem::size_of::<Ureg>(), "saved ureg size");
        // SAFETY: self is a live Ureg; the copy writes exactly 184
        // bytes inside it; src is validated + resident and read for
        // exactly its length.  Overlap is impossible.
        unsafe {
            core::ptr::copy_nonoverlapping(
                src.as_ptr(),
                self as *mut Ureg as *mut u8,
                core::mem::size_of::<Ureg>(),
            );
        }
    }

    /// Force the restored return state to user privilege — the amd64
    /// analog of aarch64's force_psr_el0t.  The saved frame crosses
    /// from USER-WRITABLE memory (the notejmp contract lets a handler
    /// edit it), and although SYSRETQ derives CS/SS from STAR (not the
    /// frame — kernel selectors cannot leak in that way), the frame's
    /// cs/ss are still what userureg-style consumers see, and RFLAGS
    /// rides R11 straight into the CPU: a handler-edited IOPL would
    /// grant the user I/O privilege.  Force the flat user selectors,
    /// clear IOPL, and set IF (the unmask-at-handoff discipline).
    pub(crate) fn force_user_return(&mut self) {
        const IF: u64 = 0x200;
        const IOPL: u64 = 3 << 12;
        self.cs = crate::trap::USER_TEXT_SEL as u64;
        self.ss = crate::trap::USER_DATA_SEL as u64;
        self.flags = (self.flags & !IOPL) | IF;
    }
}

/// A structured representation of a trap, as derived
/// from a trap number.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Trap {
    DE,
    DB,
    NMI,
    BP,
    OF,
    BR,
    UD,
    NM,
    DF,
    CoSegOvr,
    TS,
    NP,
    SS,
    GP,
    PF,
    Res15,
    MF,
    AC,
    MC,
    XM,
    VE,
    CP,
    #[allow(unused)]
    Reserved(u8),
    #[allow(unused)] // XXX: remove soon.
    Intr(u8),
}

impl From<u8> for Trap {
    fn from(trapno: u8) -> Trap {
        match trapno {
            0 => Trap::DE,
            1 => Trap::DB,
            2 => Trap::NMI,
            3 => Trap::BP,
            4 => Trap::OF,
            5 => Trap::BR,
            6 => Trap::UD,
            7 => Trap::NM,
            8 => Trap::DF,
            9 => Trap::CoSegOvr,
            10 => Trap::TS,
            11 => Trap::NP,
            12 => Trap::SS,
            13 => Trap::GP,
            14 => Trap::PF,
            15 => Trap::Res15,
            16 => Trap::MF,
            17 => Trap::AC,
            18 => Trap::MC,
            19 => Trap::XM,
            20 => Trap::VE,
            21 => Trap::CP,
            22..=31 => Trap::Reserved(trapno),
            _ => Trap::Intr(trapno),
        }
    }
}

/// A label is a place where we can `swtch` to.
///
/// This is essentially a structured representation
/// of the ABI-defined callee-saved general purpose
/// registers.
///
/// Floating-point state is maintained separately.
#[derive(Clone, Debug, Default, FromZeros)]
#[repr(C)]
pub struct Label {
    pub pc: u64,
    pub sp: u64,
    pub fp: u64,
    /// The remaining callee-saved registers — pub(crate): the swtch
    /// module's naked asm addresses these slots and witnesses the
    /// offsets against the struct (swtch.rs's const asserts).
    pub(crate) rbx: u64,
    pub(crate) r12: u64,
    pub(crate) r13: u64,
    pub(crate) r14: u64,
    pub(crate) r15: u64,
}

impl Label {
    pub const fn new() -> Label {
        Label { pc: 0, sp: 0, fp: 0, rbx: 0, r12: 0, r13: 0, r14: 0, r15: 0 }
    }
}

/// The machine structure, which describes a CPU.
///
/// The small stacks, for specific exceptions, come first and
/// occupy the first 64KiB of the Mach.  These have guard pages
/// immediately _after_ their data areas, which serve as guards
/// for the subsequent data.  Since the Mach is mapped in a
/// per-CPU portion of virtual address space that is offset from
/// a 2MiB boundary, we know that the first stack is de facto
/// guarded.
///
/// The main kstack for the scheduler comes next, and occupies
/// the next 64KiB.  It is guarded by the NMI stack's guard page.
///
/// Thus, stacks and their guards take up the first 128KiB of
/// virtual space in the Mach.
///
/// Next, we have the smaller Mach data itself; this includes the
/// space required for the structures used in the pseudo-recursive
/// page table implementation.
///
/// Warning: the layout of this structure is known to assembly
/// language.
#[derive(FromZeros)]
#[repr(C, align(65536))]
pub struct Mach {
    pub debug_stack: ExStack, // Stack for debug exceptions
    pub bp_stack: ExStack,    // Stack for breakpoint exceptions
    pub df_stack: ExStack,    // Stack for double faults
    pub nmi_stack: ExStack,   // Stack for NMIs
    pub zero: Page,           // Mapped to (per-node) read-only zeroed page
    pub stack: KStack,        // Kernel stack for scheduler

    me: *mut Mach,            // %gs:0 is a `*mut Mach` pointing to this `Mach`.
    scratch: usize,           // A scratch word used on entry to kernel
    splpc: usize,             // PC of last caller to ` k`.  Cleared by `spllo`.
    proc: *mut portdat::Proc, // Current process on this process.

    machno: u32,  // Logical ID of CPU.
    cpuno: u32,   // Physical ID of CPU.
    online: bool, // Is this CPU online?
    cpuhz: u64,

    // Various stats that the kernel keeps track of
    ticks: u64,
    tlbfaults: u64,
    ulbpurges: u64,
    pfaults: u64,
    syscalls: u64,
    mmuflushes: u64,

    sched: Label,

    // All preceding data after the stack fits within a single 4KiB page.
    // Paging structures that follow are sized in page multiples and aligned.
    pml4: PTable,  // PML4 root page table for this Mach
    pml3: PTable,  // PML3 for rec 512GiB (root of all subtrees)
    pml2: PTable,  // PML2 for rec 1GiB
    pml1: PTable,  // PML1 for rec 2MiB (PML4 and all PML3s)
    mpml3: PTable, // PML3 for mapping region
    mpml2: PTable, // PML2 for mapping region
    mpml1: PTable, // PML1 for mapping region

    // Architecturally defined data.
    pub tss: Tss, // Truly per-CPU
    pub idt: Idt, // Mapped to per-node IDT
    pub gdt: Gdt, // Mapped to per-CPU GDT; padded to 64k with zeros
}
static_assertions::const_assert_eq!(core::mem::offset_of!(Mach, pml4), 65536 * 2 + 4096);
static_assertions::const_assert_eq!(core::mem::offset_of!(Mach, stack), 65536);

/// The byte offset of the Mach's entry scratch word — the SYSCALL
/// entry's user-sp stash (the field's documented purpose: "a scratch
/// word used on entry to kernel"; the 14-02 entry asm addresses it
/// gs-relative, and the field is private, so the computed offset is
/// exported instead of the pin).
pub(crate) const MACH_SCRATCH_OFFSET: usize = core::mem::offset_of!(Mach, scratch);

impl Mach {
    pub unsafe fn init(&mut self) {
        use crate::trap;
        self.me = self;
        self.tss.init(&mut [
            (0, &mut self.stack),
            (trap::NMI_TRAPNO, &mut self.nmi_stack),
            (trap::DEBUG_TRAPNO, &mut self.debug_stack),
            (trap::BREAKPOINT_TRAPNO, &mut self.bp_stack),
            (trap::DOUBLE_FAULT_TRAPNO, &mut self.df_stack),
        ]);
        unsafe {
            self.gdt.load();
            self.idt.load();
            self.tss.load();
        }
    }
}

#[repr(u8)]
pub enum MachMode {
    Kernel,
    Ring1,
    Ring2,
    User,
}

impl From<u8> for MachMode {
    fn from(raw: u8) -> Self {
        match raw {
            0b00 => MachMode::Kernel,
            0b01 => MachMode::Ring1,
            0b10 => MachMode::Ring2,
            0b11 => MachMode::User,
            _ => panic!("invalid machine mode: {raw:x}"),
        }
    }
}

impl From<MachMode> for u8 {
    fn from(mode: MachMode) -> u8 {
        match mode {
            MachMode::Kernel => 0b00,
            MachMode::Ring1 => 0b01,
            MachMode::Ring2 => 0b10,
            MachMode::User => 0b11,
        }
    }
}

bitstruct! {
    #[derive(Clone, Copy, Debug)]
    #[repr(transparent)]
    pub struct Flags(u64) {
        pub carry: bool = 0;
        mb1: bool = 1;
        pub parity: bool = 2;
        pub aux_carry: bool = 4;
        pub zero: bool = 6;
        pub sign: bool = 7;
        pub trap: bool = 8;
        pub intr: bool = 9;
        pub dir: bool = 10;
        pub overflow: bool = 11;
        pub iopl: MachMode = 12..=13;
        pub nestedt: bool = 14;
        pub resume: bool = 16;
        pub virt8086: bool = 17;
        pub access: bool = 18;
        pub virt_intr: bool = 19;
        pub virt_intr_pending: bool = 20;
        pub id_flag: bool = 21;
    }
}

impl Flags {
    // empty/bits feed cpu::fmask (the Phase-14 syscall FMASK value); the
    // family rides the pre-existing dead spl/syscall chains until
    // 13-03/14 re-arm them.
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self(0)
    }

    pub fn new(raw: u64) -> Self {
        Self(raw).with_mb1(true)
    }

    #[allow(dead_code)]
    pub fn bits(self) -> u64 {
        self.0
    }
}

#[derive(FromZeros)]
#[repr(C, align(4096))]
pub struct PTable([u64; 512]);

impl PTable {
    pub fn array_mut(&mut self) -> &mut [u64; 512] {
        &mut self.0
    }
}

/// The smallest basic page type.
#[derive(FromZeros)]
#[repr(C, align(4096))]
pub struct Page([u8; 4096]);

impl Page {
    pub const fn empty() -> Page {
        Page([0; 4096])
    }

    pub const fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }

    pub const fn as_mut_ptr(&mut self) -> *mut u8 {
        self.0.as_mut_ptr()
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl Default for Page {
    fn default() -> Page {
        Page::empty()
    }
}

/// A trait for retrieving the top of various kinds of stacks.
pub trait Stack {
    fn len(&self) -> usize;
    fn top(&self) -> *const u8;
    fn top_mut(&mut self) -> *mut u8;
}

/// A small stack that we can use for exception handlers
/// that require their own stack (NMI, Breakpoint, Debug,
/// and Double Fault).
#[derive(FromZeros)]
#[repr(C, align(8192))]
pub struct ExStack(Page);

impl Stack for ExStack {
    fn top_mut(&mut self) -> *mut u8 {
        self.0.as_mut_ptr().wrapping_add(self.0.len())
    }

    fn top(&self) -> *const u8 {
        self.0.as_ptr().wrapping_add(self.0.len())
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

/// A kernel stack for a proc.
#[derive(FromZeros)]
#[repr(C, align(65536))]
pub struct KStack([Page; 16]);

impl Stack for KStack {
    fn top_mut(&mut self) -> *mut u8 {
        let lastpage = &mut self.0[self.0.len() - 1];
        lastpage.as_mut_ptr().wrapping_add(lastpage.len())
    }

    fn top(&self) -> *const u8 {
        let lastpage = &self.0[self.0.len() - 1];
        lastpage.as_ptr().wrapping_add(lastpage.len())
    }

    fn len(&self) -> usize {
        self.0.len() * self.0[0].len()
    }
}
