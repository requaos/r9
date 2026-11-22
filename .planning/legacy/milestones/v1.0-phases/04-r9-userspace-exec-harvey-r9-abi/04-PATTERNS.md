# Phase 4: r9 Userspace-Exec + Harvey↔r9 ABI - Pattern Map

**Mapped:** 2026-11-01
**Files analyzed:** 26 (12 create, 14 modify — nearly ALL in sibling repo `/Users/neilskinner/code/r9`; Harvey-side: `aarch64/ABI.md` appendix + staged-init/smoke C sources + host-mkpaqfs recipe + evidence)
**Analogs found:** 26 / 26 (6 are 9front-donor *reference-analogs* — semantics sources, never edit targets)

**Codebase note:** All code analogs live in the **r9 repo** (`/Users/neilskinner/code/r9`), HEAD `55ca7c4`. Phase 3's map was written at HEAD `33a9273`; **drift since: Phase 3 fully landed** — the platform tree (`platform/virt/{gic,timer,pl011,devcons}.rs`), `trap::unmask_irq()` split, `dispatch_irq()`, the `boot-test` unix-socket harness (`BootTestStep`), the `dev_panic`/virtpanic profile, and the D-06 qemu override chain all now exist and are the *current* patterns this phase extends. Every r9/Harvey path below is git-tracked (verified via `git ls-files` in each repo). The 9front donor tree exists locally at `harvey/.build/9front-donor/` but is **gitignored** (`git ls-files .build` = 0 files) — per the tracked-source gate it is cited ONLY as upstream reference paths `9front@350266104f:sys/src/9/...`, never as local paths. Paths are r9-root-relative unless prefixed `harvey:`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `aarch64/src/trap.rs` (MOD) | exception dispatch + syscall entry | event-driven (sync/IRQ/SVC) | itself (`trap.rs:37-74` TrapFrame, `:138-156` dispatch_irq) | exact |
| `aarch64/src/trap.S` (MOD) | entry/exit asm | trap save/restore | itself (`trap.S:51-115`) + `9front@350266104f:sys/src/9/arm64/l.s` (vtrap0/vtrap1 restore asymmetry) | exact + reference |
| `aarch64/src/vm.rs` (MOD) | VM service | page-table CRUD + TTBR0 switch | itself (`vm.rs:168-188` user Entry builders, `:587-613` switch) | exact |
| `aarch64/src/main.rs` (MOD) | kernel entry orchestration | sequential init | itself (`main.rs:110-260` main9, `:132-141` used_ranges) | exact |
| `aarch64/src/registers.rs` (MOD) | register decode | bitfield decode | itself (`registers.rs:84-213` EsrEl1/IFSC idiom) | exact |
| `aarch64/src/platform/virt/timer.rs` (MOD) | device driver → scheduler tick | periodic IRQ | itself (`timer.rs:54-85`) | exact |
| `aarch64/src/platform/virt/devcons.rs` (MOD → devcons *device*) | device shim | syscall-context console write | itself + `pl011.rs:93-107` lock-free idiom + `port/src/devcons.rs:24-50` Console lock | exact |
| `aarch64/src/proc.rs` (NEW) | process model | state machine + scheduler | `aarch64/src/swtch.rs` (Context) + `port/src/mcslock.rs` (Lock) + `9front@…:sys/src/9/port/proc.c` (reference) | role-match + reference |
| `aarch64/src/syscall.rs` (NEW) | syscall dispatch + handlers | request-response (measured set) | `trap.rs:138-156` (dispatch shape) + `9front@…:sys/src/9/arm64/trap.c:186-207` (reference) | role-match + reference |
| `aarch64/src/exec.rs` (NEW) | a.out loader | file parse + frame seeding | `main.rs:264-322` (test_sysexit allocation idiom) + `9front@…:trap.c:408-422` execregs (reference) | role-match + reference |
| `aarch64/src/fault.rs` (NEW) | user-fault handler | demand paging | `registers.rs:147-213` (decode idiom) + `vm.rs:411-462` (map_to) + `9front@…:trap.c:273-332` faultarm64 (reference) | role-match + reference |
| `port/src/paqfs.rs` (NEW) | fs reader (no_std, host-testable) | byte-buffer parse + typed block decode | `port/src/fdt.rs` (BE slice-parse idiom, cfg(test) host tests) + `harvey:sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c,paqfs.c}` (format truth) | role-match + reference |
| `port/src/devroot.rs` (NEW) | device (root file tree) | walk/open/read over paqfs | `port/src/dat.rs:59-77` (Dev trait skeleton) | role-match |
| `port/src/devproc.rs` (NEW) | device (lite: `/proc/N/note` only) | walk/open/write | `port/src/dat.rs:59-77` + `harvey:sys/src/libc/9sys/postnote.c` (caller) + `9front@…:sys/src/9/port/devproc.c` (reference) | role-match + reference |
| `port/src/dat.rs` (MOD) | kernel data scaffolding | Proc/Chan/Dev grow real fields | itself (`dat.rs:10-101`) | exact |
| `port/src/lib.rs` (MOD) | module root | mod declarations | itself (10-line file) | exact |
| `port/Cargo.toml` (MOD) | crate config | + miniz_oxide/adler2 deps | `aarch64/Cargo.toml` (deps + `[lints.rust]` block shape) | exact |
| `xtask/src/config.rs` (MOD) | build tooling | TOML → qemu argv + cross-repo check | itself (`config.rs:73-96` Qemu struct, `:294-322` apply_qemu_config, `:362-449` emission tests) | exact |
| `xtask/src/main.rs` (MOD) | build tooling | initrd assembly + boot-test profiles | itself (`main.rs:713-1063` BootTestStep, `:487-700` QemuStep) | exact |
| `aarch64/lib/config_virtexec.toml` + smoke configs (NEW) | build config | declarative TOML | `aarch64/lib/config_virtpanic.toml` / `config_virt.toml` | exact |
| `HowItWorks.md` / `README.md` (MOD) | docs | design tour / pointer line | themselves (HowItWorks header mandates updates) | exact |
| `harvey:aarch64/ABI.md` (MOD) | canonical ABI doc | D-07 kernel-side appendix | itself (existing frozen doc; appendix after the wire-contract sections) | exact |
| `harvey:build/aarch64/init.c` + smoke C sources (NEW; dir per OQ-5 discretion) | test binaries (7c/7l) | syscall exercisers | `harvey:build/aarch64/hello.c` (full file, 9 lines) | exact |
| `harvey:build/aarch64/` mkpaqfs host recipe (NEW or extend existing script) | host tooling | plan9port 9c/9l compile | `harvey:build/aarch64/donor-ns-build.sh:240-243` (proven recipe) | exact |
| `harvey:.planning/phases/04-*/` evidence (NEW) | acceptance evidence | captured serial + marker logs | Phase 1–3 phase-dir convention (`01-VERIFICATION.md` shape) | role-match |

## Pattern Assignments

### `aarch64/src/trap.rs` (MOD — TrapFrame reshape D-05/D-08, SVC dispatch, note boundary, fault dispatch)

**Analog:** itself at HEAD `55ca7c4`.

**Current TrapFrame** (lines 36-74 — the D-05 reshape target; tail swaps *sources* not *offsets*):

```rust
/// Register frame at time interrupt was taken
#[repr(C, align(16))]
pub struct TrapFrame {
    x0: u64, /* … x1..x28 … */
    frame_pointer: u64, // x29
    link_register: u64, // x30
    esr_el1: EsrEl1,        // @248 → becomes sp    (source: SP_EL0)
    elr_el1: u64,           // @256 → becomes pc    (source: ELR_EL1, same source today)
    far_el1: u64,           // @264 → becomes psr   (source: SPSR_EL1)
    interrupt_type: u64,    // @272 stays           (Harvey ureg->type)
}
```

Offsets are already the frozen ones — the reshape renames/resources the tail (`esr_el1`→`sp`, `far_el1`→`psr`), drops in-frame ESR/FAR (read live at trap entry, passed as separate `trap()` args per D-05), and adds the D-06 layer-1 const asserts verbatim from RESEARCH.md Pattern 1:

```rust
const _: () = assert!(core::mem::offset_of!(TrapFrame, x0) == 0);
const _: () = assert!(core::mem::offset_of!(TrapFrame, x28) == 224);
const _: () = assert!(core::mem::offset_of!(TrapFrame, link_register) == 240);
const _: () = assert!(core::mem::offset_of!(TrapFrame, sp_el0) == 248);
const _: () = assert!(core::mem::offset_of!(TrapFrame, pc) == 256);
const _: () = assert!(core::mem::offset_of!(TrapFrame, psr) == 264);
const _: () = assert!(core::mem::offset_of!(TrapFrame, interrupt_type) == 272);
const _: () = assert!(core::mem::size_of::<TrapFrame>() == 288);
```

**Dispatch shape to extend** (lines 138-156 — the existing IRQ dispatch is the in-repo template for `syscall.rs`'s table dispatch; note the return-`true`-means-handled convention and the spurious/unknown-INTID loud-but-EOI'd policy, which maps to D-14's loud-unimplemented-syscall policy):

```rust
#[cfg(platform = "virt")]
fn dispatch_irq(frame: &TrapFrame) -> bool {
    if frame.interrupt_type != IRQ_INVALID_EL1T && frame.interrupt_type != IRQ_INVALID_EL1H {
        return false;
    }
    let intid = platform::gic::ack();
    match intid {
        30 => platform::timer::tick(),
        33 => platform::pl011::rx_irq_echo(),
        platform::gic::SPURIOUS_INTID => {} // spurious: acked, no EOI
        _ => println!("unhandled IRQ {intid}"),
    }
    if intid != platform::gic::SPURIOUS_INTID {
        platform::gic::eoi(intid);
    }
    true
}
```

**Current EC 0x15 stub to replace wholesale** (lines 170-182): `println!("Syscall {syscallid}")` + wedge → becomes EC split: 0x15 → `syscall::dispatch`, 0x20/0x24 (Instruction/DataAbortLowerEl — `ExceptionClass` already has both, `registers.rs:129,132`) → `fault::user_fault`, other EL0 sync → fault note (donor `faultnote` shape), EL1 → panic dump.

**Keep untouched:** the `Debug` impl's bare `x<N>:` row names (lines 108-112 comment — the boot-test panic profile counts these; ESR/FAR move to separately-printed `trap()` args per D-08 so the row-count gate survives); `init()`/`unmask_irq()` split (lines 11-34).

---

### `aarch64/src/trap.S` (MOD — unified frame asm, D-05/D-08, per-process kstacks)

**Analog:** itself (lines 51-115 `handle_interrupt`) + `9front@350266104f:sys/src/9/arm64/l.s` (restore asymmetry precedent).

**Current save macro** (lines 51-91 — the insertion points):

```asm
.macro handle_interrupt type
	// Switch to the interrupt stack
	ldr	x0, =interruptstackbase
	add	x0, x0, #INTERRUPTSTACKSZ
	mov	sp, x0

	sub 	sp, sp, #288
	stp 	x0, x1, [sp, #16 * 0]
	// … x2..x29 pairs …
	stp	x28, x29, [sp, #16 * 14]
	// LR, ESR_EL1
	mrs	x0, esr_el1
	stp	x30, x0, [sp, #16 * 15]      // ← slot 15 hi word becomes sp (SP_EL0), not ESR
	// ELR_EL1, FAR_EL1
	mrs	x1, elr_el1
	mrs	x2, far_el1
	stp	x1, x2, [sp, #16 * 16]      // ← slot 16: pc (ELR, stays), psr (SPSR_EL1, replaces FAR)
	// Interrupt type
	ldr	x3, =\type
	str	x3, [sp, #16 * 17]
	mov	x0, sp
	bl	trap_unsafe
```

D-05/D-08 edits: after the stack switch, `mrs x?, sp_el0` into slot 15-hi (@248) and `mrs x?, spsr_el1` into slot 16-hi (@264); ESR/FAR are read live in Rust at `trap()` entry (aarch64-cpu 11.2 has both). **EL1h-origin caveat (RESEARCH Pitfall 1):** on EL1h vectors `MRS SP_EL0` yields stale user SP — EL1-origin stubs must capture the live entry-`sp` *before* the stack switch into the frame's sp slot instead. The 16 vector stubs (lines 147-193) already know their origin class via `\type` — the same dispatch point selects the capture source.

**Restore asymmetry precedent** (`9front@350266104f:sys/src/9/arm64/l.s:518-552` noteret — EL0-bound, writes SP_EL0; vs `:555-579` vtrap1 — EL1-bound, does NOT):

```asm
TEXT noteret(SB), 1, $-4			// EL0-bound return
	MOV	264(RSP), R1	// sp
	MOV	272(RSP), R2	// pc
	MOV	280(RSP), R3	// psr
	MSR	R1, SP_EL0			// ← SP_EL0 written
	MSR	R2, ELR_EL1
	MSR	R3, SPSR_EL1
	…
TEXT vtrap1(SB), 1, $-4			// EL1h return tail
	MOV	272(RSP), R2	// pc
	MOV	280(RSP), R3	// psr
	MSR	R2, ELR_EL1			// ← ELR/SPSR only, NEVER SP_EL0
	MSR	R3, SPSR_EL1
```

r9 equivalent: at the restore point test `frame.psr & 0xF == 0` (EL0t); write SP_EL0 only then (RESEARCH Pitfall 2). The existing restore (trap.S:94-114) already propagates frame `x0` → user R0 (the syscall-result mechanism — unchanged).

**Per-process kernel stacks (D-13 derived):** the `=interruptstackbase` switch (lines 52-55) becomes a load of `current_proc.kstack_top` — the frame then lives on the blocking process's own stack (RESEARCH Pitfall 10).

---

### `aarch64/src/vm.rs` (MOD — user-fault support, per-process root tables, D-10 ungate)

**Analog:** itself. Three existing assets the phase builds directly on:

**User Entry builders** (lines 168-188 — copy for fault-in mappings; text is UXN-clear/PXN-set, data is both-set):

```rust
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
```

**`map_to`'s temp-recursive-entry dance** (lines 411-462) — already handles editing the *active* user table; for not-yet-active process tables at setup time, write via KZERO-offset physical pointers (`from_ptr_to_physaddr_offset_from_kzero` / `physaddr_as_ptr_mut_offset_from_kzero`, imported at lines 6-10) exactly as `init_empty_root_page_table` does (lines 542-549):

```rust
unsafe fn init_empty_root_page_table(root_page_table: &mut RootPageTable) {
    unsafe {
        let entry = Entry::rw_kernel_data()
            .with_phys_addr(from_ptr_to_physaddr_offset_from_kzero(root_page_table))
            .with_page_or_table(true);
        write_volatile(&mut root_page_table.entries[511], entry);   // recursive slot 511
    }
}
```

Phase 4 allocates a root table *page per process* (pagealloc) instead of sharing the `USER_PAGETABLE` static singleton (line 24) — RESEARCH anti-pattern: one singleton = all processes share an address space.

**`switch()`** (lines 587-613) — the D-10 teardown call; the raspi gate is at the *call site* in `main.rs:193-201`, not in `vm.rs`, so ungating on virt is a main.rs edit:

```rust
match pgtype {
    RootPageTableType::User => {
        core::arch::asm!(
            "msr ttbr0_el1, {pt_phys}",
            "tlbi vmalle1is", "dsb ish", "isb",
            pt_phys = in(reg) pt_phys);
    } …
}
```

Note `write_entry` (lines 668-674) does a full `tlbi vmalle1is` per write — heavy but correct; optimization is Phase 5+ (RESEARCH Pattern 3 step 2).

---

### `aarch64/src/main.rs` (MOD — main9: initrd → paqfs → staged-init exec; D-12 removal)

**Analog:** itself. Three touchpoints:

**(a) initrd reservation (RESEARCH Pitfall 9)** — extend the existing `used_ranges` array (lines 132-141):

```rust
let mut physranges = [
    dtb_physrange.round(PageSize::Page4K.size()),
    boottext_physrange().add(&text_physrange()),
    rodata_physrange(),
    data_physrange().add(&bss_physrange()),
    // + initrd_physrange (from /chosen linux,initrd-start/end) — reserve or
    //   pagealloc hands userland bytes out later
];
```

**(b) D-12 removal** — delete the raspi-gated experiment block (lines 191-212: `init_user_page_tables`/`switch(User)` experiment + `vmdebug` user-table walk) and `test_sysexit()` (lines 264-322). The *idiom inside test_sysexit* is the closest in-repo analog for the real exec path's page allocation (keep the pattern, delete the experiment):

```rust
// main.rs:269-297 — allocate_virtpage + VaMapping::Addr + Entry::rw_user_text/data
let user_text = pagealloc::allocate_virtpage(
    page_table, "usertext", Entry::rw_user_text(),
    VaMapping::Addr(0x1000), RootPageTableType::User,
).expect("couldn't allocate user_text");
```

(The real path differs: demand-fault maps pages lazily — D-09 forbids this eager allocation at exec; the idiom is cited for the `allocate_virtpage` call shape only.)

**(c) exec orchestration tail** — replaces the virt idle `wfi` loop (lines 250-252) after IRQ arming (keep the lines 221-245 arming order — GIC → timer → PL011 RX → unmask — it's the Phase 4 ordering precedent): `/chosen` parse → paqfs mount → devroot → exec `/init` → the *first* `vm::switch(user_table, User)` (D-10) → eret into EL0 via the seeded frame (forkret shape, Pattern: exec.rs below).

---

### `aarch64/src/registers.rs` (MOD — data-abort ISS decode)

**Analog:** itself — the `EsrEl1IssInstructionAbort` bitstruct + `from_esr_el1` + `TryFromPrimitive` enum idiom (lines 147-213) is the exact template for a new `EsrEl1IssDataAbort` (adds `wnr: bool = 6` — write-not-read per RESEARCH Pattern 3) and a `DataFaultStatusCode` enum (same values 0-49 as `InstructionFaultStatusCode`, lines 174-213 — the DFSC/IFSC encodings are shared):

```rust
bitstruct! {
    #[derive(Copy, Clone)]
    pub struct EsrEl1IssInstructionAbort(pub u32) {
        ifsc: u8 = 0..6;
        s1ptw: bool = 7;
        ea: bool = 9;
        fnv: bool = 10;
        set: u8 = 11..13;
    }
}
// New: same shape + wnr: bool = 6; and from_esr_el1 filters
// ExceptionClass::{Instruction,Data}AbortLowerEl (already in the enum,
// registers.rs:129,132) instead of InstructionAbortSameEl (line 163).
```

Existing `#[cfg(test)]` tests (lines 215+) are the test pattern for the new decoders.

---

### `aarch64/src/platform/virt/timer.rs` (MOD — tick → scheduler tick, D-13)

**Analog:** itself. `init`/`arm`/CNTFRQ-runtime-read (lines 54-72) stay. `tick()` (lines 81-85) changes: the `println!("tick {n}")` **must go away** once syscalls print (RESEARCH Pitfall 3 — devcons syscall writes hold the Console MCS lock; IRQ-context println against it self-deadlocks; the file's own lines 77-80 comment documents the invariant). The tick becomes reschedule-only (or trylock-with-drop). Rate (`TICKS_PER_SEC = 1`, line 25) becomes the round-robin time-slice — discretion.

---

### `aarch64/src/platform/virt/devcons.rs` (MOD — devcons *device* for EL0 fds, D-15)

**Analog:** itself (Console install, lines 12-35) + the lock disciplines below.

The device shim's write path runs in syscall context and shares the Console MCS lock (`port/src/devcons.rs:24`, `:43-50`) — safe *there*, but see timer.rs above for the IRQ-side invariant it breaks. fd 0/1/2 preopen at exec per Plan 9 convention (D-15). Console read: the PL011 RX path (`pl011.rs:93-107`) is the lock-free raw-DR idiom to reuse for the read buffer feed:

```rust
// pl011.rs:100-106 — lock-free drain; never println!/Console from IRQ context
while read_reg(range, UART0_FR) & 0x10 == 0 {
    let b = read_reg(range, UART0_DR) as u8;
    while read_reg(range, UART0_FR) & (1 << 5) != 0 {}
    write_reg(range, UART0_DR, b as u32);
}
write_reg(range, UART0_ICR, 0x50);
```

For devcons the IRQ handler feeds a per-console read buffer instead of echoing; echo stays for the Phase-3 profile only if OQ-4 keeps it.

---

### `aarch64/src/proc.rs` (NEW — Proc table, scheduler, per-process kstacks, D-13)

**Analogs:** `aarch64/src/swtch.rs` (kernel-context switch — reused as-is), `port/src/mcslock.rs` (table lock), `9front@350266104f:sys/src/9/port/proc.c` (semantics reference only).

**swtch Context** (swtch.rs:6-33 — kernel-context switches ride this; EL0 entry/exit never does — EL0 is always the trap-return path):

```rust
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Context {
    pub x19: u64, /* … x20..x29 … */
    pub x30: u64, // Link register (return address)
    pub sp: u64,
    pub spsr: u64,
}
// swtch.rs:56-58
unsafe extern "C" { pub(crate) fn swtch(from: *mut *mut Context, to: &Context); }
```

**Lock idiom** (`port/src/mcslock.rs:84-101`): `static TABLE: Lock<ProcTable> = Lock::new("proc", …)` + `let node = LockNode::new(); let g = TABLE.lock(&node);` — the same shape as `CONS` in `port/src/devcons.rs:24,43-44`. **Do not hold across the fault handler** (trap context, IRQs off — RESEARCH anti-pattern on locking in trap context).

Proc fields (discretion): state, kernel stack, root page table, fd table (devroot/devcons chans), note queue, wait queue, pid/name, saved Ureg pointer. First entry = forkret shape (seed TrapFrame on the new kstack, let the normal trap-return `eret` into EL0) — see exec.rs below.

---

### `aarch64/src/syscall.rs` (NEW — dispatch table 0–54, measured set, D-14)

**Analogs:** `trap.rs:138-156` (dispatch-shape idiom, quoted above) + `9front@350266104f:sys/src/9/arm64/trap.c:186-207` (the loop semantics — reference):

```c
void
syscall(Ureg *ureg)
{
	scallnr = ureg->r0;
	if(dosyscall(scallnr, (Sargs*)(ureg->sp+BY2WD), &ureg->r0))
		returnto(noteret);      /* noted() rewrote the ureg */
	if(up->procctl || up->nnote)
		donotify(ureg);          /* notes at the return-to-EL0 boundary */
	if(up->delaysched)
		sched();
}
```

r9 shape: number = `frame.x0`; args read per-handler from user `frame.sp_el0 + 8` (8-byte slots, D-07 formula) after segment-range validation (V5: never trust `SP+8` contents); result → `frame.x0`; `NOTED` special-cased (handler rewrote the frame — no result writeback); then the note-delivery check; then maybe-reschedule. Table: `[Option<fn(&mut TrapFrame, &mut Proc) -> u64>; 55]`, `None` → print number + return -1 loudly (D-14). Handler exemplar (PWRITE offset -1) is RESEARCH.md "Code Examples" verbatim — Harvey libc `write()` is `pwrite(fd,…,-1)` (PWRITE=51), not `_WRITE`(20).

---

### `aarch64/src/exec.rs` (NEW — a.out parse + segment setup + frame seeding)

**Analogs:** `main.rs:269-297` (allocate_virtpage idiom, quoted above) + `9front@350266104f` references:

**execregs contract** (`9front@350266104f:sys/src/9/arm64/trap.c:408-422` — the crt0 seeding r9 must reproduce; Harvey `main9.s` expects R0=tos, SP→[argc][argv0]…[nil], PC=entry, LR=0):

```c
uintptr
execregs(uintptr entry, int argc, char *argv[], Tos *tos)
{
	uintptr *sp = (void*)argv;
	Ureg *ureg;
	*--sp = argc;
	ureg = up->dbgreg;
	ureg->sp = (uintptr)sp;
	ureg->pc = entry;
	ureg->link = 0;
	return (uintptr)tos;		/* becomes R0 */
}
```

PSR = 0 (EL0t) per `9front@350266104f:sys/src/9/arm64/l.s:237-243` touser (`MSR R2, SPSR_EL1` with R2=0). The r9-side `seed_exec_frame` sketch is RESEARCH.md "Code Examples" verbatim (frame at `kstack_top - 288`; `frame.psr = 0`).

**a.out parse:** 8×BE u32 + BE u64 entry @32 (HDR_MAGIC); reject non-`0x8C47` loudly; text VA fixed by 7l at `0x10000+40`; data at the 64K-aligned VA above text but **file offset `40+textsize`, not page-aligned** (RESEARCH Pitfall 5). Validation checks ported from `9front@350266104f:sys/src/9/port/sysproc.c:298+` sysexec (entry ≥ text start, sizes bounded). Share a tiny `be32/be16` helper with `port/src/paqfs.rs` (RESEARCH anti-pattern: both formats are BE-on-disk, aarch64 is LE).

---

### `aarch64/src/fault.rs` (NEW — user-fault handler, D-09 core VM deliverable)

**Analogs:** `registers.rs:147-213` (decode idiom, above) + `vm.rs:411-462` map_to (above) + `9front@350266104f:sys/src/9/arm64/trap.c:273-332` faultarm64 (reference — the fault-class routing table):

```c
	addr = getfar();
	read = (ureg->type & (1<<6)) == 0;
	switch((u32int)ureg->type & 0x3F){
	case  4: case  5: case  6: case  7:	// Translation fault.
	case  8: case  9: case 10: case 11:	// Access flag fault.
	case 12: case 13: case 14: case 15:	// Permission fault.
	case 48:				// tlb conflict fault.
		if(fault(addr, ureg->pc, read) == 0)
			break;
		/* wet floor */
	default:
		if(!user){ dumpregs(ureg); panic("kernel fault: …"); }
		faultnote("fault", read? "read": "write", addr);
	}
```

r9 handler algorithm (RESEARCH Pattern 3): classify VA by the process's segment map (text/data/bss/stack/tos — D-11 policy) → text/data: pagealloc + map_to + paqfs read_at byte-window + write via the **user VA** (the faulting process's TTBR0 is live — no aliasing, no PAN on cortex-a72) + **D-09 icache sequence after every text page-in** (`DC CVAU` per CTR_EL0.DminLine, `DSB ISH`, `IC IVAU` per IminLine, `DSB ISH`, `ISB` — RESEARCH.md D-09 deliverable has the verbatim Rust; CTR_EL0 read via inline `asm!`, aarch64-cpu 11.2 lacks it) → bss/stack/tos: zero page, bounded stack growth → unresolvable: fault note (kills handlerless process loudly). Runs with IRQs off, touches only pre-mapped kernel memory + the user VA being installed (V5 threat table).

---

### `port/src/paqfs.rs` (NEW — no_std paqfs reader, D-02)

**Analogs:** `port/src/fdt.rs` (the in-repo byte-buffer-parser idiom) + `harvey:sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c,paqfs.c}` (format truth — producer+reference reader, git-tracked Harvey sources).

**fdt.rs parse idiom to mirror** (BE reads over `MaybeUninit` slices, Option-chaining, zero heap):

```rust
// fdt.rs:30-34 — the BE u64 helper shape (currently private; the pattern to copy)
fn bytes_to_u64(bytes: &[mem::MaybeUninit<u8>]) -> Option<u64> {
    let maybe_uninit_bytes = bytes.get(..8)?;
    let init_bytes = unsafe { maybe_uninit_bytes.assume_init_ref() };
    Some(u64::from_be_bytes(init_bytes.try_into().unwrap()))
}
```

The paqfs reader is *stateless-per-call* over the memory-resident image slice (qemu loads the initrd into RAM — "block read" = slice indexing), making it reentrant-by-construction for the trap-context fault handler (RESEARCH anti-pattern: no shared reader state, no lock). Validation per V5: magic/type/size ≤ blocksize, adler32 of decoded block, all offsets bounds-checked. Deflate via `miniz_oxide::inflate::decompress_to_vec_with_limit` (raw RFC 1951 — never the zlib path); adler32 via `adler2`. **Host-testable:** the `port` crate's `#[cfg(test)]`-std-tests pattern (`port/src/lib.rs:3` `#![cfg_attr(not(any(test)), no_std)]`; precedent tests in `vm.rs:709-863` and `fdt.rs`) — Wave-0 feeds real `mkpaqfs`-produced fixture bytes through the reader under `cargo test` (D-02's byte-level verification).

**Format spec:** fully pinned in RESEARCH.md "D-02 Deliverable" (44-byte header magic `0x529AB12B`, 28-byte trailer magic `0x6B46E688` + root offset, 12-byte block headers magic `0x198A1CBF`, DirBlock/DataBlock/PointerBlock types, NoEnc always blocksize-on-disk, PaqDir entry layout, pointer-block chase with 0-offset termination). Reader must support NoEnc regardless of producer flags and reject unknown encodings loudly (`paqfs.c:963-965` precedent).

---

### `port/src/devroot.rs` + `port/src/devproc.rs` (NEW — devices over the `Dev` trait, D-15/D-16)

**Analog:** `port/src/dat.rs:59-77` — the existing `Dev` trait skeleton is the interface both implement:

```rust
pub trait Dev {
    fn reset();
    fn init();
    fn attach(spec: &[u8]) -> Chan;
    fn walk(&self, c: &Chan, nc: &mut Chan, name: &[&[u8]]) -> Walkqid;
    fn stat(&self, c: &Chan, sb: &mut [u8]) -> Result<(), Error>;
    fn open(&mut self, c: &Chan, mode: u32) -> Chan;
    fn close(&mut self, c: Chan);
    fn read(&mut self, c: &mut Chan, buf: &mut [u8], offset: u64) -> Result<u64, Error>;
    fn write(&mut self, c: &mut Chan, buf: &[u8], offset: u64) -> Result<u64, Error>;
    // … create/bread/bwrite/remove/wstat/power/config
}
```

`Chan` (dat.rs:10-38), `Qid` (90-94), `Mode` bitflags (47-57 — includes `OEXEC`, the exec-open mode per donor convention) already exist; dat.rs grows real fields this phase. **devroot:** walk/open/read/stat over the paqfs reader; exec sources binaries through this same path (one mechanism, D-15). **devproc-lite scope (hard, D-16/RESEARCH Pitfall 12):** ONLY `/proc/N/note` walk/open/write — the caller contract is `harvey:sys/src/libc/9sys/postnote.c` (PNPROC → `/proc/%d/note`, open OWRITE + write + close; verified no POSTNOTE syscall exists). Write handler = append to target pid's note queue + mark pending; delivery at the target's next return-to-EL0 (trap.rs boundary). No ctl/status/notepg.

---

### `port/src/dat.rs` + `port/src/lib.rs` (MOD — scaffolding grows real fields; mod declarations)

**Analogs:** themselves. dat.rs: `Proc {}` (line 101 — currently empty) gains the D-13 field set (see proc.rs above); `Error {}` (line 8) may gain the errstr-string carrier. lib.rs (10 lines of `pub mod`): append `pub mod paqfs; pub mod devroot; pub mod devproc;` in the existing alphabetical order.

---

### `port/Cargo.toml` (MOD — miniz_oxide + adler2 for paqfs)

**Analog:** `aarch64/Cargo.toml` (deps block + the `[lints.rust] unexpected_cfgs` check-cfg block shape). Per RESEARCH.md "Standard Stack":

```toml
[dependencies]
bitflags = "2.5"        # existing
miniz_oxide = { version = "0.9", default-features = false, features = ["with-alloc"] }
adler2 = "2"
```

(paqfs lives in `port`, so the deps land here — RESEARCH's "aarch64 crate Cargo.toml additions" line predates the port-placement decision in its own structure sketch; planner confirms placement. If the reader lands in the aarch64 crate instead, the deps follow it.)

---

### `xtask/src/config.rs` (MOD — `[qemu] initrd` key, harvey-path glue D-03, ureg.h check D-06 L2)

**Analog:** itself. New `Qemu` field following the existing Option-field style (lines 73-96) + one emission block in `apply_qemu_config` (lines 294-322 — the exact pattern to replicate):

```rust
// config.rs:317-320 — the per-key shape to copy for initrd
if let Some(dtb) = &config.dtb {
    cmd.arg("-dtb");
    cmd.arg(dtb);
}
```

(one edit covers both `QemuStep` and `BootTestStep` — both call `apply_to_qemu_step_with_overrides`.) New emission test copied from `qemu_full_keys_emit_all_flags` (lines 376-406 — TOML doc in, exact argv vec out). The Harvey-tree path glue (D-03) follows the existing env-override helper (`main.rs` `env_or`) and/or a new TOML key — discretion; the ureg.h cross-repo parse check is plain text parsing, no crate (RESEARCH Pattern 1).

---

### `xtask/src/main.rs` (MOD — initrd assembly step + smoke boot-test profiles)

**Analog:** itself. Three reusable idioms:

**(a) Step idiom** — `struct XStep` + `fn new(matches)` + `fn run(self) -> Result<()>`, clap registration, `annotated_status` for subprocesses (e.g. the mkpaqfs invocation: host-built `mkpaqfs -o <paq> <dir>` over the Harvey output dir; pin artifact under `target/`, print sha256 for evidence — D-03/RESEARCH Pattern 7).

**(b) BootTestStep marker machinery** (lines 713-1063) — new smoke profiles (e.g. `config_virtexec.toml`, `config_virtnotes.toml`) reuse everything: socket spawn with `server=on,wait=on` capture-before-start (lines 899-911), `count_marker_lines`/`contains_marker` helpers (749-755), the `BootStage` state machine (787-794) with per-profile stage selection — the existing profile switch (lines 966-971) keys off `[config] dev` list contents:

```rust
// main.rs:966-971 — profile selection idiom to extend for smoke profiles
let panic_profile = self
    .config
    .config
    .as_ref()
    .and_then(|c| c.dev.as_ref())
    .is_some_and(|devs| devs.iter().any(|d| d == "panic"));
```

**(c) D-13 marker-not-order discipline:** smoke assertions assert marker *presence* (and per-process marker *counts*), never interleaving order — preemption makes ordering nondeterministic. Failure report idiom: `fail()` (lines 1055-1062) prints the awaited stage + 2000-byte serial tail.

---

### `aarch64/lib/config_virtexec.toml` + smoke configs (NEW — TOML-as-data)

**Analog:** `aarch64/lib/config_virtpanic.toml` (the dev-cfg profile precedent) + `config_virt.toml` (base virt keys). New keys land as TOML data per Phase 3's TOML-as-data convention: `[qemu] initrd = "target/bootfs.paq"`, `[config] dev = ["exec"]`-style profile selectors matching the BootTestStep profile switch above.

---

### `HowItWorks.md` / `README.md` (MOD — docs)

**Analogs:** themselves. HowItWorks.md's header mandates update with material changes — add the userspace-exec tour (trap frame = Ureg, demand paging, paqfs initrd, devroot/cons/proc-lite) + a pointer line to `harvey:aarch64/ABI.md`'s new appendix (D-07). Match existing prose style (terse design tour; Phase 3's commit `55ca7c4` is the most recent exemplar of the update shape).

---

### `harvey:aarch64/ABI.md` (MOD — D-07 kernel-side appendix; the ONLY Harvey repo doc change)

**Analog:** itself. The existing doc structure (head matter: "frozen … Phase 4 builds r9's svc dispatch and its build-time static checks against this document") already anticipates the appendix. Append a new "Kernel-side enforcement (Phase 4 appendix)" section — frozen wire-contract text untouched. Contents per D-07: pinned frame-offset table (`r0`@0 … `r30`/`link`@240, `sp`@248, `pc`@256, `psr`@264, `type`@272; size 280, align 16 → 288), the arg-read formula (`(Sargs*)(ureg->sp+8)`; `SP+0` reserved LR slot), the two-layer check description (r9 const asserts + xtask ureg.h parse), and r9's `type`-field encoding as kernel-side informational (OQ-3 — flag at plan checkpoint). Ground truth: `harvey:aarch64/include/ureg.h` (full file verified: 35 × u64int, the union occupying one slot).

---

### `harvey:build/aarch64/init.c` + smoke C sources (NEW — staged init D-04, SC#4 smoke binaries)

**Analog:** `harvey:build/aarch64/hello.c` (exact — full file, the minimal 7c binary shape):

```c
#include <u.h>
#include <libc.h>

void
main(int, char*[])
{
	print("hello aarch64 harvey\n");
	exits(nil);
}
```

Staged `init` and smoke binaries grow from this skeleton: Plan 9 C style (tabs, `nil`, `exits`, no auto-init at declaration) — these are Harvey-tree files so Harvey conventions apply (the repo-boundary rule cuts the other way only for r9). Smoke set per D-14 trace + D-16 shape (rfork → child notify+spin → parent postnote → delivery → noted → exits → await). Location: OQ-5 discretion (keep OUT of the donor-bootdir staging area); build wiring rides the existing `aarch64/mkfile` toolchain (`CC=7c LD=7l AS=7a O=7`) or a small script beside `subset-build.sh`.

---

### `harvey:build/aarch64/` mkpaqfs host recipe (NEW or extend — D-02 producer)

**Analog:** `harvey:build/aarch64/donor-ns-build.sh:240-243` (the Phase-2-proven plan9port recipe):

```sh
# mkpaqfs (plan9port 9c + flate/mp/sec)
(export PATH="$P9PATH"; cd "$SHIM" && "$PLAN9/bin/9c" -O0 -I"$DONOR/sys/src/cmd/paqfs" \
	-c "$DONOR/sys/src/cmd/paqfs/mkpaqfs.c" &&
	"$PLAN9/bin/9l" -o "$SCRATCH/mkpaqfs" mkpaqfs.o \
	-lflate -lsec -lmp -lbio -l9)
```

Harvey-side variant compiles `sys/src/cmd/paqfs/mkpaqfs.c` (in-tree, tracked) with `-I sys/src/cmd/paqfs`; `$PLAN9/bin` LAST in PATH per project rules. The `-u` flag decision (NoEnc-only images vs deflate-capable reader) is a deliberate xtask invocation flag (RESEARCH D-02 recommendation: reader supports both via miniz_oxide; `-u` is the Wave-0 descope valve).

---

### `harvey:.planning/phases/04-*/` evidence (NEW)

**Analog:** Phase 1-3 convention — captured serial logs + marker assertions consolidated in the phase dir with the r9 commit SHA recorded (Phase 3 D-13); `01-VERIFICATION.md` captured-log + human-checkpoint structure is the shape. New this phase: the D-14 measured-syscall-list artifact (donor `startsyscall` + `/proc/N/syscall` rc-loop output, driven by `harvey:build/aarch64/drive9.py` — the existing send-on-match serial harness) lands here BEFORE any r9 handler is written.

## Shared Patterns

### DTB device/path discovery

**Source:** `port/src/fdt.rs` — `find_compatible` (:389), `find_device_type` (:402), `find_by_path` (:352-386), `property` (:152-154), `property_value_as_u32_iter` (:170), `bytes_to_u64` (:30-34, private)
**Apply to:** `main.rs` (`/chosen` → `linux,initrd-start/end` — D-01; expose a `property_value_as_u64` beside the u32 one, same `property_value_bytes().and_then(bytes_to_u64)` shape as lines 165-168), any new DTB consumers

### Locking discipline (the phase's #1 cross-cutting hazard)

**Source:** `port/src/mcslock.rs:84-101` (Lock/LockNode), `port/src/devcons.rs:24,43-50` (Console), `aarch64/src/platform/virt/timer.rs:77-80` (the documented invariant), `pl011.rs:93-107` (lock-free IRQ idiom)
**Apply to:** proc table, devcons device, syscall handlers, fault handler

Rules: (1) IRQ-context code never takes the Console lock (raw-DR idiom only); (2) syscall-context console writes hold it — therefore the timer tick stops printing (Pitfall 3); (3) trap-context fault handler takes NO locks — paqfs reader is stateless-per-call, pagealloc's MCS lock is fine only because the handler is short and non-blocking (RESEARCH Pattern 3 step 2).

### Error handling

**Source:** r9 kernel: `port::Result<T> = Result<T, &'static str>` (`port/src/lib.rs:19`), recoverable device-discovery failures `println!("can't …")` + `Err` and continue (`platform/virt/devcons.rs:29-32`), hard panics for VM/memory invariants (`main.rs:139-141`). xtask: `Result<T, Box<dyn Error>>` + `annotated_status` + `Err("step failed".into())`.
**Apply to:** all new Rust files. Phase-4-specific: unimplemented syscalls/faults/encodings fail LOUDLY (print + error return or panic), never silently (D-14, D-02, a.out parse). Harvey-side C: `-1`+errstr / `sysfatal` / `exits("usage")` per `sys/man/6/style`.

### BE-on-disk parsing

**Source:** `port/src/fdt.rs:16-41` (`bytes_to_u32/u64`, `from_be_bytes` + bounds-checked `get(..n)?`)
**Apply to:** `port/src/paqfs.rs` AND `aarch64/src/exec.rs` — both formats are big-endian; share one tiny `be16/be32/be64` helper (RESEARCH anti-pattern: "kernel-side big-endian mistakes"). Host tests against real fixture bytes for both.

### Architectural discovery (never hardcode geometry)

**Source:** `aarch64/src/platform/virt/timer.rs:66-72` (CNTFRQ read at runtime, never hardcoded — the Phase 3 precedent), RESEARCH D-09 (CTR_EL0 inline `asm!` for cache line sizes)
**Apply to:** the icache sequence in the fault path (D-09), any future cache/page maintenance. Print probed CTR_EL0 once at boot so evidence logs carry the geometry (Phase 2 D-03).

### Platform-tree + TOML-as-data placement

**Source:** `aarch64/src/platform/virt/mod.rs` (submodule declarations + DTB-driven init), `aarch64/lib/config_virt*.toml` + `xtask/src/config.rs` (new behavior lands as TOML data where feasible)
**Apply to:** new machinery placement (port vs aarch64 per the Phase 3 conventions; RESEARCH structure sketch), smoke-profile configs, initrd key, harvey-path glue.

## Reference-Analogs (9front donor — semantics sources, NEVER edit targets, NOT git-tracked locally)

The donor tree lives at gitignored `harvey/.build/9front-donor/` (pin-verified `350266104f1fed81c78c3d05fa44f523bec0ae4d` this session); cite upstream paths only. Key excerpts are quoted inline above; the full set:

| Upstream path @350266104f | What it pins | Used by |
|---|---|---|
| `sys/src/9/arm64/trap.c:186-207` | syscall loop: number=r0, args `(Sargs*)(sp+BY2WD)`, NOTED special-case, notes-at-boundary, delaysched | `syscall.rs` |
| `sys/src/9/arm64/trap.c:209-238` | `notify()` user-stack frame construction verbatim (sp−256−sizeof(Ureg), align, okaddr checks, arg homes at SP+8/SP+16) | `trap.rs` note delivery |
| `sys/src/9/arm64/trap.c:240-271` | `noted()` NCONT/NRSTR restore, NSAVE re-push, alignment checks | `syscall.rs` NOTED handler |
| `sys/src/9/arm64/trap.c:273-332` | `faultarm64` IFSC/DFSC routing (4-7/8-11/12-15/48 → demand page; else note/panic), WnR = type bit 6 | `fault.rs` |
| `sys/src/9/arm64/trap.c:396-406` | `forkchild` — copied Ureg with `r0 = 0` on the child | `proc.rs` rfork |
| `sys/src/9/arm64/trap.c:408-422` | `execregs` — SP→[argc][argv…], pc=entry, link=0, R0=tos | `exec.rs` |
| `sys/src/9/arm64/l.s:237-243` | `touser` — SPSR=0 (EL0t), SP_EL0/ELR/SPSR writes + ERET | `exec.rs` frame seeding |
| `sys/src/9/arm64/l.s:498-552` vs `:555-579` | vtrap0/noteret (EL0: SP_EL0 written) vs vtrap1 (EL1h: SP_EL0 never written) — restore asymmetry | `trap.S` |
| `sys/src/9/arm64/mem.h` | `BY2WD 8` (the +8 arg offset), user-layout reference only (D-11: r9 owns its VA policy) | ABI appendix |
| `sys/src/9/port/sysproc.c:298-357+` | sysexec validation (validaddr, OEXEC open, evenaddr) | `exec.rs` checks |
| `sys/src/9/port/devproc.c` | `startsyscall` ctl + `/proc/N/syscall` — the D-14 zero-rebuild trace mechanism | Wave-0 measurement (harness-side, not r9 code) |
| `sys/src/9/port/proc.c` | pexit/postnote/pushnote semantics reference | `proc.rs`, `devproc.rs` |

**Caveat (RESEARCH Pitfall 6):** donor `TRAPFRAMESIZE`=304 with Ureg at `RSP+16` ≠ r9's 288-at-sp+0 ≠ Harvey's sizeof(Ureg)=280. Only FIELD OFFSETS cross repos; frame discipline stays r9's own (D-06 checks pin it).

## No Analog Found

None — every file has at least a role-match in-repo plus a donor reference-analog. The closest calls: `proc.rs`/`syscall.rs`/`exec.rs`/`fault.rs`/`paqfs.rs`/`devroot.rs`/`devproc.rs` are greenfield *as r9 files* (marked role-match + reference); their semantics references are the donor sources tabled above and their structural idioms are the quoted in-repo patterns. Unlike Phase 3 (gic/timer were true greenfield), this phase's hard problems are sequencing problems with a working reference implementation at the pin (RESEARCH "Key insight").

## Metadata

**Analog search scope:** `/Users/neilskinner/code/r9` (aarch64/src/, aarch64/src/platform/virt/, port/src/, xtask/src/, both Cargo.tomls, aarch64/lib/), `/Users/neilskinner/code/harvey` (aarch64/, build/aarch64/, sys/src/cmd/paqfs/, sys/src/libc/9sys/postnote.c, .planning/phases/01-03), 9front donor at pin 350266104f (gitignored local clone, read-only reference)
**r9 HEAD drift:** Phase 3 map written at `33a9273`; current HEAD `55ca7c4` — Phase 3 deliverables all landed as mapped; no structural surprises. Phase 3 map remains accurate for everything it cited; this map re-verified every path it extends.
**Files scanned:** 22 read in full or targeted sections this session (r9: trap.rs, trap.S, vm.rs, main.rs, registers.rs, swtch.rs, fdt.rs, dat.rs, devcons.rs, mcslock.rs, mem.rs, lib.rs, timer.rs, devcons.rs(virt), pl011.rs, mod.rs(virt), pagealloc.rs(API), xtask/main.rs, xtask/config.rs, both Cargo.tomls; Harvey: ureg.h, ABI.md head, hello.c, donor-ns-build.sh recipe, subset-exec.sh header, paqfs dir listing; donor: trap.c, l.s, sysproc.c, mem.h targeted sections)
**Tracked-source check:** all cited r9/Harvey paths verified via `git ls-files --error-unmatch` (r9 HEAD `55ca7c4`, Harvey working tree); donor tree confirmed 0-tracked-files and demoted to upstream-path reference citations
**Pattern extraction date:** 2026-11-01
