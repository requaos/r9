# Phase 7: r9 PCI/ECAM + virtio-net + devether - Pattern Map

**Mapped:** 2026-11-04
**Files analyzed:** 20 (17 r9-repo, 3 Harvey-repo)
**Analogs found:** 20 / 20 (two are contract-port targets whose *semantics* come from Harvey C sources, noted per-file)

**Repo key:** `r9:` = sibling repo `/Users/neilskinner/code/r9` (all paths verified git-tracked there); unprefixed = this Harvey repo (verified git-tracked). No gitignored mirrors cited.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `r9:aarch64/src/platform/virt/pci.rs` (NEW) | platform driver/probe | DTB-discovery + boot-time MMIO config walk | `r9:aarch64/src/platform/virt/pl011.rs` + `gic/gicv2.rs` | role-match |
| `r9:aarch64/src/platform/virt/mod.rs` (MOD) | platform glue | discovery serial prints | itself (`print_machine_info`) | exact |
| `r9:aarch64/src/platform/virt/gic/gicv2.rs` (MOD) | IRQ controller driver | MMIO RMW register programming | itself (`enable()`) | exact |
| `r9:aarch64/src/trap.rs` (MOD) | IRQ dispatch | event-driven (IRQ) | itself (`dispatch_irq`) | exact |
| `r9:aarch64/src/io.rs` (MOD) | volatility floor | MMIO volatile r/w | itself (`read_reg`/`write_reg`) | exact |
| `r9:aarch64/src/main.rs` (MOD) | boot orchestration | sequential boot-time wiring | itself (IRQ-arming block + hook registrations) | exact |
| `r9:port/src/fdt.rs` (MOD) | utility (parser) | batch (DTB blob traversal) | itself (`property_range_iter` shape) | exact |
| `r9:port/src/virtio.rs` (NEW) | driver/transport | event-driven ring state machine (unsafe-free index model) | `r9:port/src/allocator.rs` (pattern) + `sys/src/9/pc/ethervirtio.c` (ring semantics) | pattern-match + cross-repo semantics |
| `r9:port/src/virtio_tests.rs` (NEW) | test | host-side unit test | `r9:port/src/allocator_tests.rs` | exact |
| `r9:port/src/devether.rs` (NEW) | device (`#l`) | request-response syscall surface + IRQ-fed RX | `r9:port/src/devcons.rs` + `devpipe.rs` (structure); `sys/src/9/pc/devether.c` + `sys/src/9k/port/netif.{h,c}` (contract) | role-match + cross-repo contract |
| `r9:port/src/netif.rs` (NEW) | model (Netif/Netfile) | request-response state machine | `r9:port/src/devpipe.rs` (Pipe/PipeQueue shape); `sys/src/9k/port/netif.{h,c}` (contract) | role-match + cross-repo contract |
| `r9:port/src/lib.rs` (MOD) | crate root | module registry | itself | exact |
| `r9:aarch64/lib/config_virt*.toml` (MOD) | config | build/qemu parameters | `r9:aarch64/lib/config_virtexec.toml` / `config_virtshell.toml` | exact |
| `r9:aarch64/lib/virt-gicv2-lowecam.{dts,dtb}` (NEW) | config/fixture | committed DTB pair | `r9:aarch64/lib/virt-gicv2.{dts,dtb}` | exact |
| `r9:Cargo.toml` (MOD) | workspace config | lint/check-cfg declarations | itself (`[workspace.lints.rust]`) | exact |
| `r9:xtask/src/main.rs` (MOD) | test harness | qemu spawn + serial/pcap assertion | itself (BootTestStep machinery) | exact |
| `build/aarch64/root-tree/lib/test/arp.req` (NEW) | fixture | committed binary payload (60 B) | `build/aarch64/hello.7.out` (committed-fixture discipline) | role-match |
| `build/aarch64/netblast.c` (NEW) | in-guest smoke binary | TX-bulk generator | `build/aarch64/stacksmoke.c` | exact |
| `build/aarch64/subset-build.sh` (MOD) | build script | namespace guest compile + out/ harvest feeding mkimage.sh's baked bootfs.paq | itself (per-target install line + stacksmoke compile/harvest flow) | exact |
| `build/aarch64/mkimage.sh` (MOD) | build script | bootfs bake + payload staging + manifest guard | itself (SUBSET staging loop + manifest loop + rodata guard) | exact |

Evidence files under `.planning/phases/07-*/` and the `flake.lock` pin advance carry no code pattern (evidence standard is CONTEXT.md's, pin advance is its own commit).

## Pattern Assignments

### `r9:aarch64/src/platform/virt/pci.rs` (NEW — ECAM walk, cap parse, BAR assign, SPI route)

**Analogs:** `r9:aarch64/src/platform/virt/pl011.rs` (DTB→map→publish-once), `r9:aarch64/src/platform/virt/gic/gicv2.rs` (multi-regblock DTB device), `r9:aarch64/src/deviceutil.rs` (MMIO mapping), `r9:aarch64/src/main.rs` (Normal-memory mapping idiom for ring pages).

**Imports pattern** (pl011.rs:1-10):
```rust
use core::cell::SyncUnsafeCell;

use crate::deviceutil::map_device_register;
use crate::regblock::RegBlock;
use crate::registers::{UART0_CR, UART0_DR, UART0_FR, UART0_ICR, UART0_IMSC, UART0_LCRH};
use crate::vm;
use port::Result;
use port::devcons::Uart;
use port::fdt::DeviceTree;
use port::mem::PhysRange;

#[cfg(not(test))]
use port::println;
```

**DTB discovery → PhysRange idiom** (pl011.rs:59-66 — the chain pci.rs reuses against `find_device_type("pci")` / `find_compatible("pci-host-ecam-generic")`):
```rust
fn find_pl011_physrange(dt: &DeviceTree) -> Result<PhysRange> {
    dt.find_compatible("arm,pl011")
        .next()
        .and_then(|uart| dt.property_translated_reg_iter(uart).next())
        .and_then(|reg| reg.regblock())
        .map(|reg| PhysRange::from(&reg))
        .ok_or("can't find pl011")
}
```

**Map + publish-once for IRQ context** (pl011.rs:38-57 — the cell discipline the mapped BAR4/capability blocks ride into the IRQ handler):
```rust
let pl011_virtrange =
    match map_device_register("pl011", pl011_physrange, vm::PageSize::Page4K) {
        Ok(pl011_virtrange) => pl011_virtrange,
        Err(msg) => {
            println!("can't map pl011 {:?}", msg);
            return Err("can't create pl011");
        }
    };

let uart = Pl011Uart { regs: RegBlock::new(pl011_virtrange) };
// SAFETY: the publish-once discipline — this write runs at driver
// construction (boot context, before trap::unmask_irq), so no IRQ
// reader can observe a torn write; the cell is never written again.
unsafe { *PL011_RANGE.get() = Some(uart.regs.clone()) };
```

**MMIO mapping primitive** (deviceutil.rs:9-34 — ECAM window + BAR4 map through this; note it maps `Entry::rw_device()`):
```rust
pub fn map_device_register(
    id: &'static str,
    physrange: PhysRange,
    page_size: vm::PageSize,
) -> Result<VirtRange> {
    let page_physrange = physrange.round(page_size.size());
    let mut physpage_allocator = PhysPageAllocator {};
    let mut vmtrait_impl = VmTraitImpl {};
    if let Ok(vr) = vm::kernel_pagetable().map_phys_range(
        &mut physpage_allocator,
        &mut vmtrait_impl,
        id,
        &page_physrange,
        vm::next_free_device_page4k(),
        vm::Entry::rw_device(),
        page_size,
        vm::RootPageTableType::Kernel,
    ) { ... }
}
```

**Ring-memory mapping anti-pattern guard** (main.rs:255-268 — descriptor rings/frame buffers are NORMAL memory at KZERO+PA, never `alloc_device_page`'s `rw_device()`; this is the initrd convention to copy):
```rust
if let Err(err) = vm::kernel_pagetable().map_phys_range(
    &mut physpage_allocator,
    &mut vmtrait_impl,
    "initrd",
    &physrange,
    vm::VaMapping::Offset(KZERO),
    vm::Entry::rw_kernel_data(),
    vm::PageSize::Page4K,
    vm::RootPageTableType::Kernel,
) {
    panic!("initrd: map failed: {err:?}");
}
```
Physical pages come from `pagealloc::allocate_physpage()` (pagealloc.rs:63-78 — "Try to allocate a physical page. Note that this is NOT mapped."), then map as above. The DTB's `dma-coherent` on root+pci nodes means no cache maintenance on qemu virt.

**Discovery print style** (virt/mod.rs:17-35 — pci discovery lines join this exact shape; "Discovery flows through port::fdt — no hardcoded virt address tables" is the module's own doc line):
```rust
pub fn print_machine_info(dt: &DeviceTree) {
    println!("Machine information:");
    if let Some(mem) = dt
        .find_device_type("memory")
        .flat_map(|m| dt.property_translated_reg_iter(m).flat_map(|r| r.regblock()))
        .next()
    {
        println!("  Memory:\t{:#x} .. +{:#x}", mem.addr, mem.len.unwrap_or(0));
    }
    for compat in ["arm,pl011", "arm,cortex-a15-gic", "arm,gic-v3", "arm,gic-400", "arm,armv8-timer"]
    {
        if dt.find_compatible(compat).next().is_some() {
            println!("  found: {compat}");
        }
    }
}
```

---

### `r9:aarch64/src/platform/virt/gic/gicv2.rs` (MODIFIED — add ICFGR level-trigger programming)

**Analog:** itself. The ICFGR write mirrors `enable()`'s RMW shape exactly.

**The RMW pattern to copy** (gicv2.rs:90-113):
```rust
pub fn enable(&self, intid: u32, prio: u8) {
    let intid = intid as usize;

    // ISENABLERn: one bit per interrupt, 32 per word (read-modify-write).
    let isenabler_off = GICD_ISENABLER0 + 4 * (intid / 32);
    let isenabler = self.gicd.read(isenabler_off);
    self.gicd.write(isenabler_off, isenabler | (1 << (intid % 32)));

    // IPRIORITYRn: one byte per interrupt, 4 per word (probe used 0xA0).
    let byte_shift = (intid % 4) * 8;
    let ipriorityr_off = GICD_IPRIORITYR0 + 4 * (intid / 4);
    let ipriorityr = self.gicd.read(ipriorityr_off);
    self.gicd
        .write(ipriorityr_off, (ipriorityr & !(0xff << byte_shift)) | ((prio as u32) << byte_shift));

    // ITARGETSRn: SPIs need an explicit cpu target ... PPIs (intid < 32) skipped.
    if intid >= 32 {
        let itargetsr_off = GICD_ITARGETSR0 + 4 * (intid / 4);
        let itargetsr = self.gicd.read(itargetsr_off);
        self.gicd
            .write(itargetsr_off, (itargetsr & !(0xff << byte_shift)) | (0x01 << byte_shift));
    }
}
```
ICFGR addition: offset 0xC00, 2 bits per interrupt, 16 per word; write it BEFORE ISENABLER (per RESEARCH §GICv2 gap 1). Register-offset constants live at gicv2.rs:22-31 — `GICD_ICFGR0: usize = 0xC00` joins that block. The facade `gic/mod.rs:53-61` (`enable` dispatch over the V2/V3 enum, lock-free cell read) needs no change if ICFGR folds into `Gicv2::enable`.

---

### `r9:aarch64/src/trap.rs` (MODIFIED — virtio SPI dispatch arm)

**Analog:** itself — the virtio INTID arm joins this match verbatim in shape.

**Dispatch pattern** (trap.rs:371-394):
```rust
#[cfg(platform = "virt")]
fn dispatch_irq(frame: &TrapFrame) -> bool {
    if frame.interrupt_type != IRQ_INVALID_EL1T
        && frame.interrupt_type != IRQ_INVALID_EL1H
        && frame.interrupt_type != IRQ_INVALID_EL0_64
    {
        return false;
    }

    let intid = platform::gic::ack();
    match intid {
        // ARM generic timer CNTP (PPI 14 -> INTID 16+14)
        30 => platform::timer::tick(),
        // PL011 UART RX (SPI 1 -> INTID 32+1): the devcons read-buffer
        // feed under dev_exec
        33 => platform::pl011::rx_irq(),
        platform::gic::SPURIOUS_INTID => {} // spurious: acked, no EOI
        _ => println!("unhandled IRQ {intid}"),
    }
    if intid != platform::gic::SPURIOUS_INTID {
        platform::gic::eoi(intid);
    }
    true
}
```
The arm shape per PL011 precedent: a bare-INTID match arm calling a platform-level handler (`platform::virt::pci::...` or a `devether`-bound entry per RESEARCH Q11). The handler MUST be lock-free IRQ context (pl011's CONS_RX discipline — see Shared Patterns) and MUST read the virtio ISR byte FIRST (read-to-ack deasserts the level line; EOI then completes the GIC handshake — the existing ack→handler→eoi flow is already correct).

---

### `r9:aarch64/src/io.rs` (MODIFIED — u8/u16/u64 volatile accessors)

**Analog:** itself — the new widths are twins of the u32 trio, each carrying its own SAFETY comment (the workspace lint `undocumented_unsafe_blocks = "deny"` makes the comment mechanical, not optional).

**The pattern** (io.rs:21-50):
```rust
/// Write val into the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
pub fn write_reg(range: &VirtRange, offset: usize, val: u32) {
    let dst = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: dst lies inside the driver's mapped MMIO VirtRange (bounds-
    // checked by offset_addr); the volatile write is the MMIO-volatility
    // floor — hardware semantics, never elided.
    unsafe { write_volatile(dst as *mut u32, val) }
}

/// Read from the reg RegBlock at offset from reg.addr.
/// Panics if offset is outside any range specified by reg.len.
pub fn read_reg(range: &VirtRange, offset: usize) -> u32 {
    let src = range.offset_addr(offset).expect("offset outside bounds");
    // SAFETY: write_reg's contract — a volatile read of the mapped
    // register (the read may have hardware side effects, e.g. PL011 DR).
    unsafe { read_volatile(src as *const u32) }
}
```
The u8 ISR-byte reader's SAFETY comment must name the read-to-ack side effect (the PL011 DR comment at io.rs:48-49 is the in-file precedent for documenting read side effects). Typed-offset access then rides `regblock.rs`'s `RegBlock` idiom (regblock.rs:34-52) — drivers never do ad-hoc pointer arithmetic.

---

### `r9:aarch64/src/main.rs` (MODIFIED — driver bind in the IRQ-arming window + devether hook registration)

**Analog:** itself, two sites.

**Site 1 — bind window** (main.rs:354-402; the pci probe → virtio init → devether attach → `gic::enable(spi, 0xA0)` chain lands inside this block, after `gic::init`/timer/PL011 and strictly before `trap::unmask_irq()` / `exec_bootstrap`'s unmask):
```rust
// Virt interrupt bring-up: arm the GIC, the generic timer
// and the UART RX path, and only then unmask IRQs.  Any earlier unmask
// would hit trap()'s sync-path wedge with no handler armed.
#[cfg(platform = "virt")]
{
    platform::gic::init(&dt);
    // CNTP generic timer: PPI 14 -> INTID 16+14 (SPIs need ITARGETSR,
    // PPIs don't -- enable() applies that rule itself).
    platform::gic::enable(30, 0xA0);
    platform::timer::init(&dt);
    // PL011 RX: SPI 1 -> INTID 32+1 -- an SPI, so enable() writes its
    // ITARGETSR cpu0 target.  Arm the UART's RXIM|RTIM path first.
    platform::pl011::init_rx();
    platform::gic::enable(33, 0xA0);
    ...
    #[cfg(not(dev_exec))]
    trap::unmask_irq();
    #[cfg(dev_exec)]
    exec_bootstrap(initrd_img);
```

**Site 2 — hook registration shape** (main.rs:474-504 — devether's `set_hooks(EtherHooks { current, block, wake })` registers alongside these in `exec_bootstrap`):
```rust
// devpipe's block/wake hooks — a read on an empty pipe
// parks the caller via the sched machinery; a write or close
// wakes the queue's waiters.  Registered before the first exec ...
port::devpipe::set_hooks(port::devpipe::PipeHooks {
    current: proc::pipe_current_pid,
    block: proc::pipe_block_current,
    wake: proc::pipe_wake,
});
...
#[cfg(platform = "virt")]
port::devcons::set_cons_hooks(port::devcons::ConsHooks {
    current: proc::current_pid,
    drain_rx: platform::pl011::cons_rx_drain,
    block: proc::cons_block_current,
    post_group: proc::post_note_group_of_members,
    echo: cons_echo,
});
```
Also copy the bind-print shape from Harvey `sys/src/9/pc/devether.c:434-448` (`#l%d: %s: %dMbps ... : %2.2ux…\n`) — RESEARCH Q11 names `#l0: virtio: 1000Mbps: 52:54:00:12:34:56` as the target serial line.

---

### `r9:port/src/fdt.rs` (MODIFIED — phandle lookup + `#interrupt-cells` reader + interrupt-map iterator)

**Analog:** itself. Three additions, all matching existing shapes:

**Iterator shape to copy** (fdt.rs:244-298, `property_range_iter` — `core::iter::from_fn` over a byte cursor with cell-count stride; the interrupt-map iterator is this same shape with a 10-cell tuple):
```rust
pub fn property_range_iter(&self, node: Node) -> impl Iterator<Item = Range> + '_ {
    let parent = self.parent(&node);
    let (parent_address_cells, _) = self.node_address_size_cells(parent);
    let (address_cells, size_cells) = self.node_address_size_cells(Some(node));

    let prop = self.property(&node, "ranges");
    let (value_start, value_len) = prop.map_or((0, 0), |p| (p.value_start, p.value_len));
    let mut value_i = value_start;
    let value_end = value_start + value_len;
    ...
    core::iter::from_fn(move || {
        ...
        let child_bus_addr = self.consume_cells(value_i, address_cells)?;
        value_i += address_size;
        ...
        Some(Range::Translated(RangeMapping { child_bus_addr, parent_bus_addr, len }))
    })
}
```

**Cell-count reader to mirror for `#interrupt-cells`** (fdt.rs:184-195 — currently private, address/size only):
```rust
fn node_address_size_cells(&self, node: Option<Node>) -> (usize, usize) {
    let address_cells = node
        .and_then(|n| self.property(&n, "#address-cells"))
        .and_then(|p| self.property_value_as_u32(&p))
        .unwrap_or(2) as usize;
    ...
}
```

**`find_phandle` shape** (fdt.rs:389-399, `find_compatible` — the `nodes()` scan + property compare; phandle lookup is the same scan comparing the `phandle` property's u32):
```rust
pub fn find_compatible(&'a self, comp: &'a str) -> impl Iterator<Item = Node> + 'a {
    self.nodes().filter(|n| {
        if let Some(comp_prop) = self.property(n, "compatible") {
            return self.property_value_contains(&comp_prop, comp);
        }
        false
    })
}
```
The decode-table unit test (RESEARCH Q2: feed the committed `virt-gicv2.dtb` bytes as a fixture) rides the crate-root test placement — see virtio_tests below.

---

### `r9:port/src/virtio.rs` (NEW — transport + split-ring index model, `#[forbid(unsafe_code)]`)

**Pattern analog:** `r9:port/src/allocator.rs` — the unsafe-free index/arena model. **Semantics analog:** `sys/src/9/pc/ethervirtio.c` (ring semantics ONLY — legacy-transitional negotiation is explicitly discarded per D-03).

**The module-registration discipline** (lib.rs:18-57 — the new module lands in this list WITH the forbid attribute; the comment at lib.rs:10-17 is the contract):
```rust
// Every port module carries
// #[forbid(unsafe_code)] EXCEPT the two named floor modules —
// `globalalloc` ... and `mcslock` ...  The forbid is a compile-time
// regression lock, not a promise: no new unsafe can enter any other port
// module without deleting its forbid attribute first.
#[forbid(unsafe_code)]
pub mod allocator;
```

**The index-model idiom** (allocator.rs:146-160 — side-table metadata + typed index, no intrusive pointers):
```rust
const HEADER_CAP: usize = 4096;
const RESERVE_CAP: usize = 1024;
const HEADER_TOTAL: usize = HEADER_CAP + RESERVE_CAP;

/// A header slot index into the side table (u16: HEADER_TOTAL << 65536).
type Idx = u16;

/// A block's metadata — a SIDE-TABLE entry (headers never
/// live inside blocks anymore; the intrusive-pointer era's `repr(C,
/// align(64))` Header is gone).  `next` links the slot's current chain —
/// a quick/misc/hash chain when used, a free-slot chain when free.
```
Apply to virtqueues: descriptor alloc = index pool; avail/used positions = u16 wrap counters; raw memory touched only through arch-side io.rs/RegBlock wrappers; `core::sync::atomic::fence(Release)` (→ `dmb ish`, verified on the pinned toolchain) between descriptor writes and the avail-idx volatile store; `fence(Acquire)` (→ `dmb ishld`) after reading used-idx before trusting buffer contents. Fences are NOT unsafe and NOT asm! — the UNSAFE-AUDIT asm-consolidation posture is preserved (zero new asm! sites).

**Ring semantics to re-express (ethervirtio.c):**

Two-descriptor chains + static avail pre-linking (ethervirtio.c:204-216 TX; 278-290 RX — RX both flagged `Dwrite`, TX neither):
```c
for(i = 0; i < q->qsize/2; i++){
    j = i << 1;
    q->desc[j].addr = PADDR(header);
    q->desc[j].len = VheaderSize;
    q->desc[j].next = j | 1;
    q->desc[j].flags = Dnext;
    q->availent[i] = q->availent[i + q->qsize/2] = j;
    j |= 1;
    q->desc[j].next = 0;
    q->desc[j].flags = 0;
}
q->avail->flags &= ~Rnointerrupt;
```

The barrier point (ethervirtio.c:249-253 — `coherence()` immediately before `avail->idx++`; our `fence(Release)` sites):
```c
blocks[i] = b;
j = (i << 1) | 1;
q->desc[j].addr = PADDR(b->rp);
q->desc[j].len = BLEN(b);
coherence();
q->avail->idx++;
vqnotify(ctlr, Vtxq);
```

Used-retire + slot recovery (ethervirtio.c:226-234):
```c
while((i = q->lastused) != q->used->idx){
    u = &q->usedent[i & q->qmask];
    i = (u->id & q->qmask) >> 1;
    if(blocks[i] == nil)
        break;
    freeb(blocks[i]);
    blocks[i] = nil;
    q->lastused++;
}
```

RX replenish-until-caught-up then ONE notify (ethervirtio.c:299-312):
```c
do {
    i = q->avail->idx & (q->qmask >> 1);
    if(blocks[i] != nil)
        break;
    if((b = iallocb(ETHERMAXTU)) == nil)
        break;
    blocks[i] = b;
    j = (i << 1) | 1;
    q->desc[j].addr = PADDR(b->rp);
    q->desc[j].len = BALLOC(b);
    coherence();
    q->avail->idx++;
} while(q->avail->idx != q->used->idx);
vqnotify(ctlr, Vrxq);
```

Interrupt handler + IRQ-accounting counters (ethervirtio.c:393-412 — ISR-byte read first, per-queue `nintr`; the `nnote`/`nintr` pair is NETDRV-04's accounting surface):
```c
static void
interrupt(Ureg*, void* arg)
{
    ...
    if(inb(ctlr->port+Qisr) & 1){
        for(i = 0; i < ctlr->nqueue; i++){
            q = &ctlr->queue[i];
            if(vhasroom(q)){
                q->nintr++;
                wakeup(q);
            }
        }
    }
}
```

**Security note for the ring code (RESEARCH Security Domain):** used-ring `id`/`len` are device-written — validate `id < qsize`, `len ≤ buffer len`, `len ≥ 12` before indexing (the device is the first untrusted input source; index-model bounds checks are the mitigation).

---

### `r9:port/src/virtio_tests.rs` (NEW — host-side ring state machine tests)

**Analog:** `r9:port/src/allocator_tests.rs` + its crate-root placement (lib.rs:59-65).

**Placement contract** (lib.rs:59-65):
```rust
// The allocator's tests live at the crate root (NOT under the
// forbidden allocator module) so the pin tests' arena constructions keep
// their unsafe blocks — ...
#[cfg(test)]
mod allocator_tests;
```

**Test shape** (allocator_tests.rs:16-42 — a heap `Vec<u8>` arena fixture, then behavior assertions; 123-145 for the `#[should_panic]` loud-tripwire idiom; 119-124's header comment documents WHY the test is at crate root):
```rust
#[test]
fn tail_alloc_big_nonpow2_free_and_realloc() {
    let mut arena = alloc::vec![0u8; 1024 * 1024];
    let start = arena.as_mut_ptr();
    let len = arena.len();
    let tail = BumpAlloc::new(unsafe { Block::new_from_raw_parts(start, len) });
    let mut qf = QuickFit::new(tail);
    let layout = Layout::from_size_align(200_000, 1).unwrap();
    let p = qf.malloc(layout);
    assert!(!p.is_null(), "a 200 KB non-pow2 tail allocation succeeds");
    ...
}
```
For virtio: the fake backing arena is a `Vec<u8>` carved into desc/avail/used regions; tests pin descriptor alloc, avail/used index advance, 2^16 wrap, chain assembly, used-retire (`lastused != used.idx`), and barrier *placement* (a mock volatile-writer can record operation order). Name tests after the regression they pin, with a header comment telling the wedge story (allocator_tests.rs:44-53 is the model).

---

### `r9:port/src/devether.rs` (NEW — `#l` device + virtio-net binding)

**Structural analogs:** `r9:port/src/devcons.rs` (static file table + free-function device surface) and `r9:port/src/devpipe.rs` (per-instance id-tagged qids + blocking read hooks). **Contract sources (semantics, not letter):** `sys/src/9k/port/netif.h`, `sys/src/9k/port/netif.c`, `sys/src/9/pc/devether.c`.

**Module header discipline** (devpipe.rs:1-41 — copy this shape verbatim in intent: name the donor, name the deltas, document the qid scheme, state the lock invariants):
```rust
//! devpipe — the pipe device `#|` (the connectembed substrate;
//! semantics ported from 9k's devpipe.c + qio.c).
//! ...
//! Qid scheme (9k's PIPEQID): path = (pipe_id << 5) | tag where tag is 0
//! (the dir), 1 (`data`, end 0), 2 (`data1`, end 1).
```
NOTE: devpipe's `(id << 5) | tag` qid is the SAME shape as netif.h's `NETQID(i,t) = ((i)<<5)|(t)` — the id/tag bit split (low 5 bits = tag) is precedented in-tree.

**Qid constants + static file table** (devcons.rs:134-201 — the per-file metadata table the walk/stat/listing share):
```rust
/// Plan 9 Qid type bytes (sys/include/libc.h).
const QTDIR: u8 = 0x80;
const QTFILE: u8 = 0x00;

/// Qid path values for the devcons files.
pub(crate) const QID_ROOT: u64 = 0;
pub(crate) const QID_CONS: u64 = 1;
...
/// The per-file static metadata (name, qid, mode) — the walk table and the
/// directory listing share it.  lengths are computed at read/stat time ...
const FILES: [(&[u8], u64, u32); 7] = [
    (b"cons", QID_CONS, 0o666),
    (b"null", QID_NULL, 0o666),
    (b"user", QID_USER, 0o444),
    ...
];
```
devether's table realizes the netif.h:5-19 enum (`Ncloneqid=1, Naddrqid, N2ndqid, N3rdqid, Ndataqid, Nctlqid, Nstatqid, Ntypeqid, Nifstatqid`) as the three-level tree: `#l` root → `ether0/` (`clone` 0666, `addr` 0666, `stats` 0444, `ifstats` 0444 + numbered conv dirs) → `N/` (`data`, `ctl`, `stats`, `type`, `ifstats`).

**attach/walk/open free functions + mode discipline** (devcons.rs:161-167, 249-279 — devether's attach parses the controller number from spec per Harvey `etherattach`):
```rust
pub fn attach() -> crate::dat::Chan {
    crate::dat::Chan::new(
        crate::dat::DEV_CONS,
        crate::dat::Qid { path: QID_ROOT, vers: 0, typ: QTDIR },
        0,
    )
}

pub fn open(c: &crate::dat::Chan, mode: u32) -> crate::Result<crate::dat::Chan> {
    if c.qid.typ == QTDIR && mode != crate::dat::OREAD {
        return Err("devcons: directory opens OREAD only");
    }
    ...
}
```

**Registration** (namec.rs:58-72 — devether joins as a new arm; DEV_ETHER constant joins dat.rs:20-33; dispatch arms join dev_walk/open_chan/stat_chan/close_chan at namec.rs:100-114, 314-339, 344-357, 389-399):
```rust
fn registry_attach(rune: char, spec: &[u8]) -> Result<Chan> {
    match rune {
        'c' => Ok(crate::devcons::attach()),
        'e' => crate::devenv::attach(spec),
        'p' => Ok(crate::devproc::attach()),
        '|' => Ok(crate::devpipe::attach()),
        'd' => Ok(crate::devdup::attach()),
        's' => Ok(crate::devsrv::attach()),
        _ => Err("namec: unknown # device"),
    }
}
```

**Data-file write path contract** (devether.c:283-298 — the minmtu gate the baked ARP must satisfy: bake 60 bytes; the kernel REWRITES the source MAC):
```c
if(n > ether->mtu)
    error(Etoobig);
if(n < ether->minmtu)
    error(Etoosmall);

bp = allocb(n);
...
memmove(bp->rp, buf, n);
memmove(bp->rp+Eaddrlen, ether->ea, Eaddrlen);
```

**Stats file format — byte-for-byte target** (netif.c:221-235):
```c
j = snprint(p, READSTR, "in: %llud\n", nif->inpackets);
j += snprint(p+j, READSTR-j, "link: %d\n", nif->link);
j += snprint(p+j, READSTR-j, "out: %llud\n", nif->outpackets);
j += snprint(p+j, READSTR-j, "crc errs: %d\n", nif->crcs);
j += snprint(p+j, READSTR-j, "overflows: %d\n", nif->overflows);
j += snprint(p+j, READSTR-j, "soft overflows: %d\n", nif->soverflows);
j += snprint(p+j, READSTR-j, "framing errs: %d\n", nif->frames);
j += snprint(p+j, READSTR-j, "buffer errs: %d\n", nif->buffs);
j += snprint(p+j, READSTR-j, "output errs: %d\n", nif->oerrs);
j += snprint(p+j, READSTR-j, "prom: %d\n", nif->prom);
j += snprint(p+j, READSTR-j, "mbps: %d\n", nif->mbps);
j += snprint(p+j, READSTR-j, "addr: ");
for(i = 0; i < nif->alen; i++)
    j += snprint(p+j, READSTR-j, "%2.2ux", nif->addr[i]);
snprint(p+j, READSTR-j, "\n");
```
`addr` file = `%2.2ux` × alen, no separators (netif.c:239-248). `ifstats` routes to the driver hook — keep ethervirtio.c:459-462's per-queue line shape (`vq%d … avail->idx %d used->idx %d lastused %hud nintr %ud nnote %ud`) — that line IS NETDRV-04's reconciliation surface.

**Clone semantics** (netif.c:175-178 — opening `clone` allocates a conversation id and rewrites the chan's qid to `NETQID(id, Nctlqid)`):
```c
case Ncloneqid:
    id = openfile(nif, -1);
    c->qid.path = NETQID(id, Nctlqid);
    break;
```

---

### `r9:port/src/netif.rs` (NEW — Netif/Netfile model, qid scheme, stats)

**Structural analog:** `r9:port/src/devpipe.rs`'s `Pipe`/`PipeQueue` split — a shared state object with per-end queues, waiter pid lists, and open-ref counts, serialized by an mcslock `Lock` that is a documented LEAF lock.

**The shared-state shape** (devpipe.rs:67-96):
```rust
/// One direction's byte queue: the buffered bytes, the hangup/close
/// flags (donor qhangup/qclose), and the pids blocked reading it.
pub struct PipeQueue {
    buf: VecDeque<u8>,
    hungup: bool,
    closed: bool,
    /// Pids blocked reading this queue (woken by write/close).
    waiters: Vec<u64>,
}

/// The shared pipe object (9k's `Pipe`): two cross-connected queues +
/// the open-end counts per side (donor qref).  Carried by the chans as
/// `Arc<Lock<Pipe>>`; the Lock serializes queue ops from syscall
/// context (a LEAF lock: never held across block()/sched(), never taken
/// from IRQ context, never held while taking PROC_TABLE).
pub struct Pipe {
    q: [PipeQueue; 2],
    qref: [usize; 2],
    id: u64,
}
```
Map to: `Netfile` (inuse/mode/type/prom + `iq` input queue — netif.h:32-49) and `Netif` (name/nfile/f[]/limit/alen/mbps/minmtu/maxmtu/mtu/addr/bcast + the counter set inpackets/outpackets/misses/soverflows — netif.h:65-110). The IRQ-context RX path must NOT take this lock (pl011 lock-free discipline — see Shared Patterns); RX retire pushes onto a per-conversation queue and wakes readers via injected hooks.

**Hook injection idiom** (devpipe.rs:98-123 — devether/netif's `EtherHooks { current, block, wake }` copies this exactly, registered once from main9/exec_bootstrap):
```rust
#[derive(Clone, Copy)]
pub struct PipeHooks {
    pub current: fn() -> u64,
    pub block: fn(),
    pub wake: fn(&[u64]),
}

static HOOKS: Lock<Option<PipeHooks>> = Lock::new("devpipe", None);

/// Register the arch-side hooks.  Called once during boot before any
/// user process exists; a second registration is a boot bug.
pub fn set_hooks(hooks: PipeHooks) {
    let node = LockNode::new();
    let mut guard = HOOKS.lock(&node);
    assert!(guard.is_none(), "devpipe: hooks registered twice");
    *guard = Some(hooks);
}
```

**Blocking-read loop** (devpipe.rs:273-303 — the data-file read's block/wake shape; the register-under-lock → drop-lock → block() invariant in the module header at devpipe.rs:19-22 is load-bearing and must be restated for devether):
```rust
let mut h: Option<PipeHooks> = None;
loop {
    let node = LockNode::new();
    {
        let mut g = pipe.lock(&node);
        let q = &mut g.q[side];
        if !q.buf.is_empty() {
            let n = q.buf.len().min(buf.len());
            for (i, b) in q.buf.drain(..n).enumerate() {
                buf[i] = b;
            }
            return Ok(n as u64);
        }
        if q.hungup {
            return Ok(0); // EOF — the write side is fully closed
        }
        ...
        let pid = (h.unwrap().current)();
        if !q.waiters.contains(&pid) {
            q.waiters.push(pid);
        }
    }
    (h.unwrap().block)(); // park mid-syscall; a write/close wakes us
}
```
Delta for devether: reads return WHOLE frames (Qmsg mode — one frame per read), never partial byte drains. RESEARCH Open Question 1's recommendation: no Phase-7 gate blocks on a data read (stats + pcap carry RX proof), so the block hook can land minimal-but-correct.

---

### `r9:port/src/lib.rs` (MODIFIED — module adds)

**Analog:** itself (18-57 + 59-65). Add `virtio`, `devether`, `netif` each with `#[forbid(unsafe_code)]`; add `#[cfg(test)] mod virtio_tests;` at the crate root next to `allocator_tests`.

---

### `r9:aarch64/lib/config_virt*.toml` (MODIFIED — `dev` list gains `"ether"`)

**Analogs:** `r9:aarch64/lib/config_virtexec.toml:19` (`dev = ["exec"]`), `r9:aarch64/lib/config_virtshell.toml:17` (`dev = ["exec", "bootchain"]` with a comment block naming the profile's purpose). The TOML→cfg emission is automatic (config.rs:217-223 — each `dev = [...]` entry emits `--cfg dev_<name>`). raspi/x86_64/riscv64 configs never gain the flag.

---

### `r9:aarch64/lib/virt-gicv2-lowecam.{dts,dtb}` (NEW — highmem-ecam=off perturbation pair)

**Analog:** `r9:aarch64/lib/virt-gicv2.{dts,dtb}` — same committed-pair convention, same provenance header.

**Provenance header pattern** (virt-gicv2.dts:1-29 — the low-ecam pair's header names the machine flag `virt,gic-version=2,highmem-ecam=off` in place of the base recipe):
```
/*
 * virt-gicv2.dts -- qemu `-M virt,gic-version=2 -cpu cortex-a72` machine DTB.
 * Committed source of aarch64/lib/virt-gicv2.dtb (referenced by
 * aarch64/lib/config_virt.toml [qemu].dtb), ...
 *
 * Provenance (qemu drift policy, phase-03 research P11): dumped from
 * qemu-system-aarch64 11.1.2 (nixpkgs pin, see flake.lock) and round-tripped
 * through dtc 1.8.1. ...
 *
 * Regenerate (from the r9 repo root, inside `nix develop`):
 *   qemu-system-aarch64 -M virt,gic-version=2,dumpdtb=target/virt-gicv2-raw.dtb \
 *       -cpu cortex-a72
 *   dtc -I dtb -O dts -o aarch64/lib/virt-gicv2.dts target/virt-gicv2-raw.dtb
 *   # review the diff, then restore this header comment
 *   dtc -I dts -O dtb -o aarch64/lib/virt-gicv2.dtb aarch64/lib/virt-gicv2.dts
 * ...
 */
```
The new profile's config TOML needs NO xtask schema change: `[qemu].machine = "virt,gic-version=2,highmem-ecam=off"` is a free-form string passed to `-M` (config.rs:337-339), and `dtb = "aarch64/lib/virt-gicv2-lowecam.dtb"` rides the existing key.

---

### `r9:Cargo.toml` (MODIFIED — check-cfg gains `cfg(dev_ether)`)

**Analog:** itself (25-31 — the comment at 14-21 states the rule: "each family member must be declared here when code first uses it"):
```toml
[workspace.lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = [
    'cfg(platform, values("virt", "raspi3b", "raspi4b"))',
    'cfg(dev_panic)',
    'cfg(dev_exec)',
    'cfg(dev_bootchain)',
] }
```

---

### `r9:xtask/src/main.rs` (MODIFIED — netdev+filter-dump args, Net boot-test profile, pcap assertion)

**Analog:** itself. Four integration points:

**1. qemu argv emission** (main.rs:671-672 — the riscv64 leg's netdev shape; CONTEXT.md explicitly notes the riscv64 `virtio-net-device` MMIO leg is NOT a device-model precedent — copy only the argv emission idiom `cmd.arg("-netdev").arg(...)`):
```rust
cmd.arg("-netdev").arg("type=user,id=net0");
cmd.arg("-device").arg("virtio-net-device,netdev=net0");
```
For aarch64 virt per D-09 the stanza is `-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0` + `-object filter-dump,id=f0,netdev=n0,file=target/boot-test-<profile>.pcap`, added to the aarch64 virt path (BootTestStep's qemu spawn at main.rs:1064-1077 shares `apply_to_qemu_step_with_overrides`; the stanza appends after it, same as the `-chardev`/`-serial` additions at 1069-1074).

**2. Marker constants** (main.rs:756-828 — named `const &str` with a doc comment telling the boot story; net markers join this block):
```rust
/// Assertion markers, named so every profile reuses them.
const BOOT_BANNER_MARKER: &str = "r9 from the Internet";
...
/// Notes-profile additions: the virtnotes
/// config boots the SAME kernel+initrd as virtexec/virtmulti, but the
/// staged init also rforks a third child (smokenote), ...
const BOOT_NOTE_GOT_MARKER: &str = "smokenote: got note";
```

**3. The negative scan + assertion loop** (main.rs:838-851, 1180-1308 — the Net profile extends the `BootStage` enum + stage machine; presence-based `contains_marker`, count-based `count_marker_lines`; the scan keys `panic|unhandled interrupt|note-timeout|unimplemented`):
```rust
fn negative_scan_hit(capture: &[u8]) -> Option<&'static str> {
    let lower: Vec<u8> = capture.iter().map(|b| b.to_ascii_lowercase()).collect();
    if contains_marker(&lower, b"panic") {
        Some("panic")
    } else if contains_marker(&lower, b"unhandled interrupt") {
        Some("Unhandled interrupt")
    } ...
}
```
CAUTION (Pitfall N10): a stray NIC IRQ prints `unhandled IRQ 36` which does NOT trip this scan — treat any such line as a bind-ordering bug.

**4. INITRD payload list** (main.rs:1699-1759 — the blaster binary joins `INITRD_FILES`; the `lib/test/arp.req` file rides the tree assembly the way `proc/` does at line 1746 — `create_dir_all` + copy):
```rust
const INITRD_FILES: [&str; 6] = ["init", "hello", "smokefork", "smokenote", "stacksmoke", "smokesleep"];
```

**5. pcap parser** — no code analog exists, but the DISCIPLINE analog is the socket-serial boot-test's zero-new-crates stance (main.rs:741-744: "Zero new crate dependencies: std::os::unix::net::UnixStream + std::process only"). ~100 lines of std-only Rust: skip the 24-byte global header, iterate 16-byte record headers `{le32 ts_sec, ts_usec, caplen, len}`, match ethertype 0x0806 + ARP body fields (oper=1 request, spa=10.0.2.15, tpa=10.0.2.2, sha=guest MAC). Teardown safety is free: qemu's filter-dump writes are unbuffered `writev()` per packet, so `kill_process_group` (main.rs:965-978) loses nothing.

---

### `build/aarch64/root-tree/lib/test/arp.req` (NEW — 60-byte padded ARP request fixture)

**Analog:** `build/aarch64/hello.7.out` — a committed binary fixture carried verbatim through the pipeline (stage-build.sh:137 `cp ... hello.7.out "$OUT/hello"`; mkimage.sh's root-tree tar copy at line 87 carries committed skeleton files byte-exact). The 60-byte shape (42-byte ARP + 18 zero pad) is mandated by devether's `Etoosmall` gate (ETHERMINTU=60, sys/src/9k/k10/etherif.h:4 + sys/src/9/pc/devether.c:285-286); contents: dst=ff:ff:ff:ff:ff:ff, src=52:54:00:12:34:56 (the qemu default — devether rewrites it anyway), ethertype 0x0806, ARP htype=1 ptype=0x0800 hlen=6 plen=4 oper=1, spa=10.0.2.15, tha=0, tpa=10.0.2.2 (slirp answers ONLY for its own addresses).

---

### `build/aarch64/netblast.c` (NEW — in-guest TX-bulk generator)

**Analog:** `build/aarch64/stacksmoke.c` (full file is the template — 40 lines):

**The smoke-binary shape** (stacksmoke.c:1-40):
```c
#include <u.h>
#include <libc.h>

/*
 * stacksmoke — the staged stack-growth smoke: force multiple
 * demand-paged stack page-ins on r9 before rc's parser ever depends on
 * stack growth.  ...  The checksum accumulates into a plain
 * static so the unwind prints one marker line through the real pwrite
 * path.  No notes, no alarm, no sleeps.
 */

enum { DEPTH = 64, FRAMESZ = 256 };

static ulong sum;
...
void
main(int, char*[])
{
	recur(DEPTH);
	print("stacksmoke: depth 64 ok, checksum %lud\n", sum);
	exits(nil);
}
```
Copy: Plan 9 C (`#include <u.h>` + `<libc.h>`, tabs, `print(...)` marker lines through the real pwrite path, `exits(nil)`), a header comment naming the gate the binary serves, and a marker vocabulary the xtask profile asserts. The blaster loops `write` to `/net/ether0/0/data` as fast as the ring drains (wedge-hunting per D-08), printing counters on completion.

---

### `build/aarch64/subset-build.sh` (MODIFIED — `dd.install` + the netblast compile/harvest block)

**Analog:** itself. Correction to the map-time suggestion: the virtnet gate boots mkimage.sh's Harvey-baked bootfs.paq, whose supply chain is subset-build.sh — NOT stage-build.sh, whose STAGE_SRCS/smoke-out chain feeds r9's auto-baked bootfs.paq (a different image the gate never boots; 07-04's subset-build.sh + mkimage.sh wiring is the authoritative one). Three touch points, all mechanical:
- The single-file per-target install line (subset-build.sh:131-135): `objtype=aarch64 mk cat.install cp.install date.install … wc.install` gains `dd.install`, alphabetical — dd.c is single-file, u.h+libc.h only, so no new libs enter the closure.
- The netblast compile/harvest block mirrors the stacksmoke one (subset-build.sh:164-166):
```sh
(cd "$HARVEY/build/aarch64" && objtype=aarch64 7c -FTVw stacksmoke.c \
	&& objtype=aarch64 7l -o "$HARVEY/build/aarch64/out/stacksmoke" stacksmoke.7 \
	&& rm -f stacksmoke.7) || fail "stacksmoke"
```
  with `netblast.c`/`out/netblast` substituted. The stacksmoke handling the plan follows exactly spans two more sites: the darwin-leg out/ rsync-back list (subset-build.sh:218-223) gains `out/netblast`, and the host-side R_MAGIC assert loop (subset-build.sh:239-243) gains the entry.
- mkimage.sh then stages `out/netblast` beside `out/stacksmoke` and carries the new manifest lines — see the mkimage.sh section below.

---

### `build/aarch64/mkimage.sh` (MODIFIED — SUBSET gains dd, netblast staging, new manifest lines)

**Analog:** itself. Three additive touches, all riding existing loops: the `SUBSET` variable gains `dd` (the `for f in $SUBSET` staging loop and its per-file manifest re-check then cover dd automatically); `out/netblast` stages beside the `out/stacksmoke` cp; and the pre-bake manifest loop (mkimage.sh:113-117) gains `lib/test/arp.req`, `lib/net-sweep.rc`, and `aarch64/bin/netblast`:
```sh
for p in boot/boot boot/paqfs aarch64/init aarch64/bin/stacksmoke \
	rc/lib/rcmain rc/bin/termrc lib/namespace adm/timezone/local \
	usr/glenda/lib/profile; do
	[ -f "$STAGE/$p" ] || err "manifest: $p missing from staged tree"
done
```
The committed-skeleton flow (mkimage.sh:85-87, tar copy of `root-tree/`) means adding `root-tree/lib/test/arp.req` and `root-tree/lib/net-sweep.rc` flows automatically; the manifest lines make absence loud. CONTEXT notes the real-init rodata guard (mkimage.sh:127-130) applies unchanged — the payload additions don't perturb it.

---

## Shared Patterns

### Publish-once cells into IRQ context (SyncUnsafeCell)
**Source:** `r9:aarch64/src/platform/virt/pl011.rs:26,50-55` + `r9:aarch64/src/platform/virt/gic/mod.rs:30,41-45`
**Apply to:** pci.rs (mapped BAR/capability blocks), the virtio SPI handler's view of the device
```rust
static PL011_RANGE: SyncUnsafeCell<Option<RegBlock>> = SyncUnsafeCell::new(None);
...
// SAFETY: the publish-once discipline — this write runs at driver
// construction (boot context, before trap::unmask_irq), so no IRQ
// reader can observe a torn write; the cell is never written again.
unsafe { *PL011_RANGE.get() = Some(uart.regs.clone()) };
```
IRQ readers take NO lock: `let range = unsafe { &*PL011_RANGE.get() };` with a SAFETY comment citing the write-once discipline (pl011.rs:120-123).

### IRQ-context lock-free discipline
**Source:** `r9:aarch64/src/platform/virt/pl011.rs:100-140` (CONS_RX ring) + `r9:port/src/devpipe.rs:87-96` (LEAF-lock rule)
**Apply to:** the virtio IRQ handler (ISR read → retire → replenish → wake) and devether's RX path
- Handler: single-writer lock-free structures only; no `println!`, no Console lock, no syscall-context locks.
- RX retire pushes frames onto per-conversation queues and wakes via injected hooks (`EtherHooks { wake }`); syscall-context readers take the Netif mcslock (a documented LEAF lock: never held across block()/sched(), never taken from IRQ context).

### Dependency-injected hooks (port cannot see arch)
**Source:** `r9:port/src/devpipe.rs:98-123`, `r9:port/src/devcons.rs:685-710`, registration sites `r9:aarch64/src/main.rs:474-504`
**Apply to:** devether's block/wake/current hooks
Shape: `#[derive(Clone, Copy)] pub struct XHooks { ... }` + `static HOOKS: Lock<Option<XHooks>>` + `set_hooks()` asserting single registration + a lazy `hooks()?` accessor returning `Err("devX: ... hooks not registered")`.

### `port::Result<T>` error convention
**Source:** `r9:port/src/lib.rs:67` (`pub type Result<T> = core::result::Result<T, &'static str>;`) + `r9:port/src/namec.rs:17-18` header
**Apply to:** all new port modules
Errors are `"devX: ..."`-prefixed static strings (e.g. `"devcons: walk of a non-directory"`, devcons.rs:251). Loud-unimplemented for out-of-era ctl verbs per CONTEXT's ctl-depth discretion (devpipe.rs:292's "loud rather than a silent EOF" is the posture).

### stat(5) wire records (StatWriter, FOUR strings)
**Source:** `r9:port/src/dat.rs:182-223` (StatWriter) + `r9:port/src/devcons.rs:313-333` (write_stat_record)
**Apply to:** devether's stat/read-dir paths
```rust
let mut w = crate::dat::StatWriter::new(sb);
w.u16(0); // type: unused this era
w.u32(crate::dat::DEV_CONS as u32);
w.u8(if mode & 0x8000_0000 != 0 { QTDIR } else { QTFILE });
w.u32(0); // vers
w.u64(qid);
w.u32(mode);
w.u32(0); // atime
w.u32(0); // mtime
w.u64(length);
w.string(name);
w.string(b"glenda");
w.string(b"glenda");
w.string(b"glenda"); // muid — the fourth string statcheck walks
Ok(w.finish())
```
FOUR length-prefixed strings are mandatory (libc's statcheck rejects short records). Directory listings rebuild per call (devcons.rs:368-394, devpipe.rs:234-259).

### Barrier discipline for rings (verified codegen)
**Source:** 07-RESEARCH.md §"aarch64 Barriers from Rust" (disassembly-verified on the pinned toolchain)
**Apply to:** every virtio.rs ring post/retire
- `core::sync::atomic::fence(Ordering::Release)` → `dmb ish` — between descriptor/ring volatile writes and the avail-idx volatile store; again before the suppression check → notify.
- `core::sync::atomic::fence(Ordering::Acquire)` → `dmb ishld` — after reading used-idx, before trusting descriptor/buffer contents.
- Ring accesses themselves must be VOLATILE (io.rs wrappers) so the compiler never reorders around the fences. No new `asm!`; fences are safe core intrinsics.

### DTB-driven everything (NETDRV-01's rule)
**Source:** `r9:aarch64/src/platform/virt/mod.rs:14-16` ("Discovery flows through port::fdt -- no hardcoded virt address tables") + `r9:aarch64/src/platform/virt/timer.rs:44-65` (runtime re-discovery with documented fallback constant)
**Apply to:** pci.rs (ECAM window + interrupt-map), gicv2 ICFGR flags
Zero hardcoded IRQs/addresses. `0x4010000000` and INTID 36 appear in docs/test expectations, never as code constants. timer.rs's `discover_intid` + warn-on-mismatch shape (timer.rs:48-73) is the in-tree idiom for "re-discover at runtime, keep a documented fallback only where the committed DTB always carries the node" — for PCI, NO fallback: absence = bind failure (the perturbation gate depends on it).

### Boot-test evidence discipline
**Source:** `r9:xtask/src/main.rs:830-851` (negative scan), 853-861 (presence/count helpers), 1050-1062 (stale-log destruction pre-spawn), 763-765 (one total timeout budget, `R9_BOOT_TEST_TIMEOUT`)
**Apply to:** the Net profile + pcap assertion
Timing-tolerant presence assertions, never interleaving order; capture-before-start via `server=on,wait=on`; one wall-clock budget; SIGKILL process-group teardown; evidence log + pcap committed to `.planning/phases/07-*/` with the r9 commit SHA recorded.

## No Analog Found

None — every file in the phase has an in-tree analog or a named cross-repo semantics source. The two partial cases, with the substitution the planner should use:

| File | Gap | Substitute |
|------|-----|------------|
| `r9:xtask/src/main.rs` pcap parser | No byte-format parser exists in xtask (marker scans are the closest) | The zero-new-crates discipline (main.rs:741-744) + std-only byte iteration (`contains_marker`/`count_marker_lines` at 853-861) — the pcap+ARP parse is ~100 lines of the same style |
| `r9:port/src/virtio.rs` PCI capability walk + negotiation | No PCI or virtio code exists in either repo that is modern-only | RESEARCH.md's spec-verified sketches (§"Code Examples": cap walk, §3.1.1 sequence, barrier sandwich, IRQ handler) + Harvey `ethervirtio.c` ring semantics only (its negotiation is legacy — explicitly forbidden by D-03) |

## Metadata

**Analog search scope:** `r9:port/src/` (all 23 modules listed; devcons/devpipe/devroot/dat/namec/lib/allocator/allocator_tests/fdt read in full), `r9:aarch64/src/` (pl011, timer, gicv2, gic/mod, virt/mod, io, regblock, deviceutil, pagealloc read; trap.rs/main.rs targeted reads), `r9:xtask/src/` (main.rs + config.rs targeted reads), `r9:Cargo.toml`, `r9:aarch64/Cargo.toml`, `r9:aarch64/lib/` (config TOMLs + virt-gicv2.dts header), `r9:UNSAFE-AUDIT.md` (targeted); Harvey: `sys/src/9/pc/ethervirtio.c` + `devether.c` (targeted), `sys/src/9k/port/netif.h` (full) + `netif.c` (targeted), `build/aarch64/` (mkimage.sh, stage-build.sh, stacksmoke.c full; root-tree layout), `flake.nix` (targeted).

**Tracked-source gate:** every cited r9 path verified via `git -C /Users/neilskinner/code/r9 ls-files`; every cited Harvey path via `git ls-files`. No gitignored mirrors named.

**Files scanned:** ~35
**Pattern extraction date:** 2026-11-04
