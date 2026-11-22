# Phase 3: r9 qemu-virt Platform Bring-up - Pattern Map

**Mapped:** 2026-11-01
**Files analyzed:** 28 (19 create, 9 modify — ALL in sibling repo `/Users/neilskinner/code/r9` except evidence copies)
**Analogs found:** 25 / 28

**Codebase note:** Every code analog below lives in the **r9 repo** (`/Users/neilskinner/code/r9`, HEAD `33a9273`), not Harvey. All cited paths are git-tracked (verified via `git ls-files`). Harvey's only analog contribution is `flake.nix` (nix conventions template per D-08). Paths below are written relative to the r9 repo root unless prefixed `harvey:`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `aarch64/src/platform/mod.rs` (NEW) | module root | compile-time cfg dispatch | `riscv64/src/platform/mod.rs` | exact |
| `aarch64/src/platform/virt/mod.rs` (NEW) | platform module | init orchestration | `riscv64/src/platform/virt/mod.rs` + `nezha/mod.rs` | exact |
| `aarch64/src/platform/virt/devcons.rs` (NEW) | console init | DTB-driven device init | `riscv64/src/platform/virt/devcons.rs` | exact |
| `aarch64/src/platform/virt/pl011.rs` (NEW) | device driver | MMIO polled-TX + IRQ-RX | `aarch64/src/uartpl011.rs` | exact (strip raspi init) |
| `aarch64/src/platform/virt/gic.rs` (NEW) | device driver | MMIO IRQ controller | none (greenfield) — idioms from `aarch64/src/io.rs`, `deviceutil.rs` | none |
| `aarch64/src/platform/virt/timer.rs` (NEW) | device driver | sysreg + periodic IRQ | none (greenfield) — `aarch64-cpu` crate idiom | none |
| `aarch64/src/platform/raspi3b/{mod,devcons,mailbox,uartmini}.rs` (NEW, moved) | platform module | MMIO/mailbox | the existing `aarch64/src/{devcons,mailbox,uartmini}.rs` themselves | exact (move) |
| `aarch64/src/platform/raspi4b/…` (NEW, moved) | platform module | MMIO/mailbox | same as raspi3b | exact (move) |
| `aarch64/lib/config_virt.toml` (NEW) | build config | declarative TOML | `aarch64/lib/config_default.toml` + `riscv64/lib/config_default.toml` | exact |
| `aarch64/lib/config_virtpanic.toml` (NEW) | build config | declarative TOML | same + `xtask/src/config.rs:149-157` dev-cfg mechanism | exact |
| `aarch64/lib/virt-gicv2.{dts,dtb}` (NEW) | data artifact | build-time generated, committed | `aarch64/lib/bcm2710-rpi-3-b.{dts,dtb}` | role-match (convention) |
| `flake.nix` (NEW) | nix config | declarative devshell+apps | `harvey:flake.nix` | role-match (different stack) |
| `aarch64/src/main.rs` (MOD) | kernel entry | sequential init orchestration | itself + `riscv64/src/main.rs` (platform dispatch) | exact |
| `aarch64/src/trap.rs` (MOD) | exception dispatch | event-driven (IRQ/sync) | itself (return path exists in `trap.S`) | exact |
| `aarch64/src/l.S` (MOD) | entry asm | boot flow | itself | exact |
| `aarch64/src/pre_mmu/util.rs` (MOD) | early debug uart | MMIO polled I/O, pre-MMU | itself | exact |
| `aarch64/src/pre_mmu/vminit.rs` (MOD) | VM init | page-table construction | itself | exact |
| `aarch64/src/kmem.rs` (MOD) | kernel memory map | address math | itself | exact |
| `aarch64/lib/config_default.toml` (MOD) | build config | declarative | `riscv64/lib/config_default.toml` (platform key) | exact |
| `aarch64/lib/config_raspi4b.toml` (MOD) | build config | declarative | same | exact |
| `aarch64/lib/kernel.ld` (likely UNCHANGED) | linker script | `${LOAD-ADDRESS}` substitution | itself (substitution already generic) | n/a |
| `aarch64/Cargo.toml` (MOD) | crate config | declarative | `riscv64/Cargo.toml` (`[lints.rust]` block) | exact |
| `xtask/src/config.rs` (MOD) | build tooling | TOML → cargo/qemu argv | itself (`Qemu` struct + `apply_qemu_config`) | exact |
| `xtask/src/main.rs` (MOD) | build tooling | subprocess orchestration | itself (`BuildStep`/`DistStep`/`QemuStep`/`TestStep`) | exact |
| `xtask/src/main.rs` — `boot-test` subcommand (NEW arm) | acceptance harness | spawn qemu, socket capture, assert | `TestStep`/`QemuStep` in same file | role-match |
| `README.md` (MOD) | docs | table row | itself (qemu invocation table) | exact |
| `HowItWorks.md` (MOD) | docs | design tour | itself (header mandates updates) | exact |
| `harvey:.planning/phases/03-*/` evidence copies (NEW) | acceptance evidence | captured serial logs | `harvey:.planning/phases/01-*/01-VERIFICATION.md` convention | role-match |

## Pattern Assignments

### `aarch64/src/platform/mod.rs` (NEW — module root, cfg dispatch)

**Analog:** `riscv64/src/platform/mod.rs` (exact — the D-01 precedent, full file lines 1-9)

```rust
#[cfg(platform = "nezha")]
pub mod nezha;
#[cfg(platform = "nezha")]
pub use crate::platform::nezha::*;

#[cfg(any(test, platform = "virt", not(platform = "nezha")))]
pub mod virt;
#[cfg(any(test, platform = "virt", not(platform = "nezha")))]
pub use crate::platform::virt::*;
```

**Copy shape exactly**, substituting aarch64 platforms: `raspi3b` / `raspi4b` / `virt`. Note the riscv64 default-fallback idiom (`not(platform = "nezha")` → virt) — the aarch64 equivalent should make **raspi3b the default** (`not(any(platform = "virt", platform = "raspi4b"))`) so existing bare `cargo xtask build --arch aarch64` keeps working before `config_default.toml` gains its platform key. The wildcard re-export (`pub use …::*`) is what lets `main.rs` write `use crate::platform::{devcons, platform_init};` platform-agnostically.

**Companion change (mandated pair):** `aarch64/Cargo.toml` must gain the check-cfg declaration or nightly-2026-01-30 emits `unexpected_cfgs` warnings (research P10). Analog `riscv64/Cargo.toml`:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = [
    'cfg(platform, values("nezha"))',
    'cfg(platform, values("virt"))',
] }
```

Extend the values list with `"raspi3b"`, `"raspi4b"`, plus `'cfg(dev_panic)'` if the D-12 panic uses the `dev` cfg route.

---

### `aarch64/src/platform/virt/mod.rs` (NEW — platform module root)

**Analog:** `riscv64/src/platform/virt/mod.rs` (exact, full file lines 1-3) and `riscv64/src/platform/nezha/mod.rs` (identical shape)

```rust
pub mod devcons;

pub fn platform_init() {}
```

aarch64/virt's version declares its real submodules and DTB-driven init entry points:

```rust
pub mod devcons;
pub mod gic;
pub mod pl011;
pub mod timer;

// plus per D-03: print_machine_info(dt: &DeviceTree) — see RESEARCH.md "DTB-derived platform info" sketch
```

Keep `platform_init()` as the named orchestration hook (riscv64 convention, consumed by `main.rs` via the wildcard re-export).

---

### `aarch64/src/platform/virt/devcons.rs` (NEW — console init, DTB-driven)

**Analog:** `riscv64/src/platform/virt/devcons.rs` (exact, full file lines 1-29)

```rust
// Racy to start.

use core::cell::SyncUnsafeCell;
use core::mem::MaybeUninit;

use crate::uart16550::Uart16550;
use port::{devcons::Console, fdt::DeviceTree};

pub fn init(dt: &DeviceTree) {
    let ns16550a_reg = dt
        .find_compatible("ns16550a")
        .next()
        .and_then(|uart| dt.property_translated_reg_iter(uart).next())
        .and_then(|reg| reg.regblock())
        .unwrap();

    Console::set_uart(|| {
        let mut uart = Uart16550::new(ns16550a_reg);
        uart.init(115_200);

        static CONS: SyncUnsafeCell<MaybeUninit<Uart16550>> =
            SyncUnsafeCell::new(MaybeUninit::uninit());
        unsafe {
            let cons = &mut *CONS.get();
            cons.write(uart);
            Ok(cons.assume_init_mut())
        }
    });
}
```

**This is the file to copy nearly verbatim**, with three substitutions: `Uart16550` → the virt `Pl011Uart`, compatible string `"ns16550a"` → `"arm,pl011"`, and the constructor becomes DTB-driven per the pl011.rs pattern below. The `SyncUnsafeCell<MaybeUninit<T>>` + `Console::set_uart` closure is the current r9-wide console-install idiom.

⚠️ **Drift warning:** `riscv64/src/platform/nezha/devcons.rs` (lines 16-26) calls `Console::new(|| …)` — that API **no longer exists** in `port/src/devcons.rs` (only `set_uart`, lines 31-38). nezha is cfg-gated out of default builds and has bit-rotted. **Do not copy nezha's devcons**; copy virt's.

The current aarch64 raspi console (`aarch64/src/devcons.rs` lines 30-54) shows the same idiom with error-tolerant variant (returns `Err` instead of unwrap) — copy its `match` shape into the moved `platform/raspi*/devcons.rs`:

```rust
// aarch64/src/devcons.rs:31-53 — error-tolerant variant of the same pattern
Console::set_uart(|| {
    let uart = MiniUart::new_with_map_ranges(dt);
    match uart {
        Ok(uart) => {
            uart.init();
            static UART: SyncUnsafeCell<MaybeUninit<MiniUart>> =
                SyncUnsafeCell::new(MaybeUninit::uninit());
            unsafe {
                let cons = &mut *UART.get();
                cons.write(uart);
                Ok(cons.assume_init_mut())
            }
        }
        Err(msg) => {
            println!("can't initialise uart: {msg:?}");
            Err("can't initialise uart")
        }
    }
});
```

---

### `aarch64/src/platform/virt/pl011.rs` (NEW — PL011 driver, virt-flavored)

**Analog:** `aarch64/src/uartpl011.rs` (exact — same device; reuse register-level code, bypass raspi GPIO/mailbox init per CONTEXT D-02/research Pattern 3)

**Imports + struct + DTB discovery** (lines 1-11, 16-20, 60-67 — keep, minus GPIO):

```rust
use crate::deviceutil::map_device_register;
use crate::io::{read_reg, write_reg};
use port::Result;
use port::devcons::Uart;
use port::fdt::DeviceTree;
use port::mem::{PhysRange, VirtRange};

pub struct Pl011Uart {
    pl011_virtrange: VirtRange,   // gpio_virtrange field DROPPED on virt
}

fn find_pl011_physrange(dt: &DeviceTree) -> Result<PhysRange> {
    dt.find_compatible("arm,pl011")
        .next()
        .and_then(|uart| dt.property_translated_reg_iter(uart).next())
        .and_then(|reg| reg.regblock())
        .map(|reg| PhysRange::from(&reg))
        .ok_or("can't find pl011")
}
```

**Register-mapping via `map_device_register`** (lines 38-47 — the idiom for all virt MMIO drivers):

```rust
let pl011_physrange = Self::find_pl011_physrange(dt)?;
let pl011_virtrange =
    match map_device_register("pl011", pl011_physrange, vm::PageSize::Page4K) {
        Ok(pl011_virtrange) => pl011_virtrange,
        Err(msg) => {
            println!("can't map pl011 {:?}", msg);
            return Err("can't create pl011");
        }
    };
```

**`putb` polling TX** (lines 126-131 — unchanged; verified safe against qemu's PL011 model):

```rust
impl Uart for Pl011Uart {
    fn putb(&self, b: u8) {
        // Wait for UART to become ready to transmit.
        while read_reg(&self.pl011_virtrange, UART0_FR) & (1 << 5) != 0 {}
        write_reg(&self.pl011_virtrange, UART0_DR, b as u32);
    }
}
```

**DO NOT copy** the raspi `init()` (lines 69-100: GPIO pull config, `mailbox::set_clock_rate`, baud divisor math) — qemu pre-configures the PL011 and ignores divisors (research Pattern 3, empirically verified). The virt init is only: `ICR = 0x7ff`, `LCRH = 0x70` (8N1 + FIFO enable), `IMSC = 0x50` (RXIM|RTIM for SC#4 IRQ RX), `CR = 0x301` (UARTEN|TXE|RXE). The RX-drain/echo IRQ handler is new code; it must use a **lock-free raw `putb` path** (research P8: `Console::putstr` takes the mcslock → self-deadlock if the IRQ lands mid-print). Register-offset constants (`UART0_DR/FR/LCRH/IMSC/ICR/CR`) already exist in `aarch64/src/registers.rs:15-22` — reuse, don't redefine.

---

### `aarch64/src/platform/virt/gic.rs` (NEW — GICv2 driver) — **NO ANALOG**

No interrupt-controller driver exists anywhere in r9 (grep-confirmed: riscv64 virt/nezha have no PLIC/CLINT driver; x86_64 trap code is PIC-less). **Planner: use RESEARCH.md "Pattern 1" verified register sequence as the source of truth.** Build it from in-repo idioms:

- **MMIO map:** `map_device_register("gicd", …)` / `map_device_register("gicc", …)` — same shape as `uartpl011.rs:29-46` above. DTB reg tuples (GICD `0x08000000`/0x10000, GICC `0x08010000`/0x10000) come from `find_compatible("arm,cortex-a15-gic")` **falling back to** `"arm,gic-400"` (research correction to D-04 — qemu gicv2's actual compatible is the a15 string) → `property_translated_reg_iter` yields both regblocks in order.
- **Register access:** `read_reg`/`write_reg` from `aarch64/src/io.rs:40-43, 21-24` — bounds-checked volatile wrappers over `VirtRange`:

```rust
// aarch64/src/io.rs:21-24, 40-43
pub fn write_reg(range: &VirtRange, offset: usize, val: u32) {
    let dst = range.offset_addr(offset).expect("offset outside bounds");
    unsafe { write_volatile(dst as *mut u32, val) }
}

pub fn read_reg(range: &VirtRange, offset: usize) -> u32 {
    let src = range.offset_addr(offset).expect("offset outside bounds");
    unsafe { read_volatile(src as *const u32) }
}
```

- **API shape suggestion** (mirrors research): `init(dt) -> Result<Gic>`; `enable(intid, prio)` doing ISENABLER/IPRIORITYR math (+ ITARGETSR **only for SPIs ≥32** — research P6: PPIs are hardwired per-CPU, SPIs undelivered without it); `ack() -> u32` (GICC_IAR 0x00C); `eoi(intid)` (GICC_EOIR 0x010); spurious 1023 → no EOI.
- **Constant style:** follow `aarch64/src/registers.rs:15-22` — `pub const GICD_CTLR: usize = 0x000; // comment` — rather than a register-block crate.

---

### `aarch64/src/platform/virt/timer.rs` (NEW — ARM generic timer) — **NO ANALOG**

No timer code exists in r9 (grep-confirmed). **Planner: RESEARCH.md "Pattern 2" has the verified sequence.** The register access idiom is the `aarch64-cpu` 11.2 crate — already a dependency (`aarch64/Cargo.toml:10`: `aarch64-cpu = "11.2"`), using tock-registers style:

```rust
use aarch64_cpu::registers::{CNTFRQ_EL0, CNTP_CTL_EL0, CNTP_TVAL_EL0};
// CNTFRQ_EL0.get() — read, never hardcode (62.5 MHz TCG / 24 MHz HVF, both measured)
// CNTP_TVAL_EL0.set(freq / ticks_per_sec)
// CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET)
```

In-repo precedent for `aarch64_cpu` register reads: `aarch64/src/registers.rs` wraps `MidrEl1` (used in `main.rs:136` as `registers::MidrEl1::read()`); `trap.rs:13-18` shows raw `core::arch::asm!` for VBAR/DAIF when the crate isn't used. Prefer the crate. DTB discovery: `find_compatible("arm,armv8-timer")` → `interrupts` property → `property_value_as_u32_iter` (`port/src/fdt.rs:170`) → CNTP is the `<0x01 0x0e …>` triple → PPI 14 → INTID 30.

---

### `aarch64/src/platform/raspi3b/` + `raspi4b/` (NEW — moved raspi modules)

**Analog:** the existing files themselves — this is a move, not a rewrite.

| Current file | Moves to |
|--------------|----------|
| `aarch64/src/devcons.rs` (54 lines) | `platform/raspi3b/devcons.rs` (shared by raspi4b via re-export, or duplicated per riscv64's per-platform `devcons.rs` convention — riscv64 has one devcons per platform dir) |
| `aarch64/src/uartmini.rs` (158 lines) | `platform/raspi3b/uartmini.rs` |
| `aarch64/src/mailbox.rs` (330 lines) | `platform/raspi3b/mailbox.rs` |
| `aarch64/src/uartpl011.rs` (raspi variant) | `platform/raspi3b/uartpl011.rs` if raspi keeps the PL011 option; the virt copy is independent |

Import paths inside moved files change from `crate::deviceutil::…` to `crate::deviceutil::…` (unchanged — deviceutil/io/vm stay at crate root per research structure sketch). Only `main.rs`'s `mod` declarations and the platform `mod.rs` re-exports change. The mailbox's lock-init idiom (`mailbox.rs:21-36`: `static MAILBOX: Lock<Option<Mailbox>>` + `init(dt)`) is the pattern D-03 gates behind `#[cfg(platform = "raspi3b")]` etc. in `main9` — see main.rs assignment below.

raspi4b vs raspi3b divergence today is only DTB + qemu machine (both live in config TOMLs) — the riscv64 tree shows per-platform dirs even when near-identical (`virt/` vs `nezha/` both carry a `devcons.rs`). Follow that: two dirs, near-identical content, rather than one `raspi/` with inner cfg — matching the D-01 wording (`raspi3b/`, `raspi4b/`).

---

### `aarch64/lib/config_virt.toml` + `config_virtpanic.toml` (NEW — build config)

**Analogs:** `aarch64/lib/config_default.toml` (full file, 14 lines) for shape; `riscv64/lib/config_default.toml` (full file, 13 lines) for the `platform` key placement.

aarch64 current:

```toml
[build]
target = "lib/aarch64-unknown-none-elf.json"
buildflags = ["-Z", "build-std=core,alloc"]

[link]
# linker script to use
script = 'aarch64/lib/kernel.ld'

# kernel load address to insert into kernel.ld
load-address = '0xffff800000100000 - 0x80000'

[qemu]
machine = "raspi3b"
dtb = "aarch64/lib/bcm2710-rpi-3-b.dtb"
```

riscv64 (note `[config] platform` — currently **absent from both aarch64 TOMLs**, required by D-01):

```toml
[config]
platform = "virt"
```

New `config_virt.toml` per RESEARCH.md sketch (already reconciled with findings): `[link] load-address = '0xffff800040080000'`, `[config] platform = "virt"`, `[qemu] machine = "virt,gic-version=2"` + D-07's new `cpu`/`smp`/`memory`/`accel` keys, `dtb = "aarch64/lib/virt-gicv2.dtb"` (the `-dtb` fix for research P1: ELF boot gets x0=0 and no auto-DTB). `kernel.ld` needs **no edit** — the `${LOAD-ADDRESS}` substitution (`xtask/src/config.rs:226`) and `AT()` physical-address emission already generalize.

`config_virtpanic.toml` (D-12) adds only `dev = ["panic"]` under `[config]` — zero xtask schema change: the existing generic mechanism at `xtask/src/config.rs:149-157` maps each `dev` entry to `--cfg dev_<key>`:

```rust
// xtask/src/config.rs:149-157
if let Some(devices) = &config.dev {
    // get all [config] 'dev' settings
    for dev in devices {
        rustflags.push("--cfg".into());
        // prefix the setting
        rustflags.push(format!("dev_{dev}"));
    }
}
```

Then `main.rs` gets `#[cfg(dev_panic)] platform::panic_at_end_of_main9();` (or equivalent). **Modified files** `config_default.toml` / `config_raspi4b.toml` each gain `[config] platform = "raspi3b"` / `"raspi4b"` — add the section between `[link]` and `[qemu]` to match riscv64 ordering.

---

### `aarch64/lib/virt-gicv2.dts` + `.dtb` (NEW — committed DTB artifacts)

**Analog:** `aarch64/lib/bcm2710-rpi-3-b.dts` + `.dtb` pair (role-match — the established convention: human-reviewable `.dts` committed alongside the `dtc`-compiled `.dtb`; config TOML references the `.dtb`). Generation procedure (research P11/Open Question 1): `qemu-system-aarch64 -M virt,gic-version=2 -machine dumpdtb=virt-gicv2.dtb` → `dtc` round-trip → commit both. The xtask `qemu` subcommand already has a `--dump_dtb` flag pattern (`xtask/src/main.rs:204-205`, consumed riscv64-side at lines 507-512) — its `-machine virt,dumpdtb=…` form is the in-tree precedent for producing this artifact.

---

### `flake.nix` (NEW — r9 repo root)

**Analog:** `harvey:flake.nix` (role-match — D-08 says "mirror Harvey's app style"; the rust stack differs, RESEARCH.md "flake.nix" section has the full sketch).

Conventions to copy from Harvey's flake (lines 1-111):

```nix
# harvey:flake.nix:6-10 — systems + forAllSystems idiom
outputs = { self, nixpkgs }:
  let
    systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
    forAllSystems = nixpkgs.lib.genAttrs systems;
    pkgsFor = system: nixpkgs.legacyPackages.${system};
```

```nix
# harvey:flake.nix:37-72 — apps as writeShellApplication/writeShellScript wrappers,
# exposed via { type = "app"; program = …; }
mkApp = pkg: { type = "app"; program = "${pkg}/bin/${pkg.name}"; };
```

```nix
# harvey:flake.nix:79-108 — devShells.default via pkgs.mkShell with an
# informative shellHook (echo tool versions + usage hint)
```

r9 deltas (per RESEARCH.md): systems list drops `x86_64-darwin` (out of project scope); adds `rust-overlay` input with `inputs.nixpkgs.follows`; toolchain via `pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml` (D-09 — `rust-toolchain.toml` stays single source of truth: nightly-2026-01-30, components `rustfmt rust-src clippy llvm-tools`, three targets — file is 8 lines, verified); devshell packages `[ toolchain pkgs.qemu pkgs.dtc ]`; `shellHook` exports `OBJCOPY` from the toolchain's rustlib dir (xtask already honors `$OBJCOPY` first — `xtask/src/main.rs:246-271`); apps `build` and `boot-virt` wrap `cargo xtask …`. **Do not** add `packages.*` outputs (deferred to Phase 5 per D-08).

---

### `aarch64/src/main.rs` (MOD — kernel entry orchestration)

**Analog:** itself + `riscv64/src/main.rs` (exact for the platform-dispatch shape, full file 37 lines).

riscv64's main9 shows the target end-state of platform-agnostic orchestration (lines 16-26):

```rust
use crate::platform::{devcons, platform_init};
use port::fdt::DeviceTree;

#[unsafe(no_mangle)]
pub extern "C" fn main9(hartid: usize, dtb_ptr: usize) -> ! {
    let dt = unsafe { DeviceTree::from_usize(dtb_ptr).unwrap() };
    crate::devcons::init(&dt);
    platform_init();
    …
}
```

Current aarch64 `main9` (`main.rs:110-190`) hardcodes raspi steps. The modification keeps its overall shape (trap init → DTB → pagealloc → devcons → banner/info → VM → loop) and gates the raspi couplings:

- **`mod` declarations** (lines 10-25): `mod mailbox; mod uartmini; mod uartpl011;` move under `mod platform;` (per D-01); add `mod platform;`.
- **`mailbox::init(&dt)` (line 131) + `print_board_info()` (line 142, defined 80-93)** — gate to raspi platforms (D-03): `#[cfg(any(platform = "raspi3b", platform = "raspi4b"))]`.
- **`devcons::init(&dt)` (line 130)** — becomes `platform::devcons::init(&dt)` via the wildcard re-export; the virt variant discovers PL011 from DTB.
- **DTB pointer source (line 111 param `dtb_va`)** — on virt, `l.S` may pass 0 (research P1); virt path substitutes the fixed PA `0x40000000` mapped via KZERO. Keep `print_binary_sections`/`print_memory_info`/`print_stacks` (platform-agnostic).
- **`test_sysexit()` (line 179, defined 194-251)** — research Open Question 3 recommends gating to raspi configs (userspace-exec is Phase 4); planner decides per CONTEXT.md line 46.
- **New virt tail:** GIC init → timer arm → UART RX arm → **then** unmask IRQs → banner + `wfi` tick loop. Ordering constraint from research P5: `trap::init` currently sets VBAR *and* unmasks (`DAIFClr #2`) — must be split before virt can enable IRQs safely (see trap.rs assignment).
- **`#[cfg(dev_panic)]` hook** at end of main9 for the D-12 panic config.

Banner `"r9 from the Internet"` (line 134) is the SC#2 assertion marker the boot-test harness greps — **do not reword**.

---

### `aarch64/src/trap.rs` (MOD — exception/IRQ dispatch)

**Analog:** itself — the return path already exists in `trap.S`; only `trap()` never returns.

Current state (lines 9-20, 109-122):

```rust
pub fn init() {
    #[cfg(not(test))]
    unsafe {
        // Set up a vector table for any exception that is taken to EL1, then enable IRQ
        core::arch::asm!(
            "adr {tmp}, exception_vectors",
            "msr vbar_el1, {tmp}",
            "msr DAIFClr, #2",      // ← unmasks IRQs with no handler behind them (P5)
            tmp = out(reg) _,
        );
    }
}
```

```rust
fn trap(frame: &mut TrapFrame) {
    if frame.esr_el1.ec() == 0x15 {
        // Syscall
        let syscallid = frame.esr_el1.iss();
        println!("Syscall {syscallid}");
    } else {
        println!("{:#?}", frame);
        println!("Unhandled interrupt");
    }

    loop {                          // ← every IRQ wedges here today
        core::hint::spin_loop();
    }
}
```

Modification per RESEARCH.md Pattern 4: (1) split `init()` into VBAR-setup + a separate `unmask_irq()` called by main9 only after GIC+timer+UART are armed; (2) add an IRQ branch on `frame.interrupt_type` matching `IRQ_EL1h`/`IRQ_EL1t` (the `.equ` constants live in `trap.S:17-35` — `IRQ_INVALID_EL1h = 5`; VBAR+0x280 is the EL1h IRQ vector) that does `gic.ack()` → dispatch INTID 30 → `timer::tick()`, INTID 33 → PL011 RX drain/echo, 1023 spurious → no EOI → `gic.eoi(intid)` → **return** (the `eret` in `trap.S`'s `handle_interrupt` macro, lines 93-114, then resumes the `wfi` loop); (3) keep the sync/syscall branches spinning (or clean-halt) as today — the sync branch's `{:#?}` frame dump is also the D-12 panic-dump foundation (`TrapFrame` + its `Debug` impl at lines 22-102 — reuse as-is for the `brk #0` panic route per RESEARCH.md D-12 recommendation).

`trap.S` itself likely needs **no structural change** — vector table, frame save/restore, and `eret` are complete (lines 38-115). The interrupt-stack switch at `trap.S:51-55` already isolates IRQ context.

---

### `aarch64/src/l.S` (MOD — entry asm)

**Analog:** itself (163 lines). Two virt touchpoints:

1. **Early UART call** (line 122: `bl init_early_uart_rpi4` + line 124-125 putc) — raspi-specific; needs a virt counterpart (early PL011 at `0x09000000`, zero-init per D-02/research Pattern 3) selected at assemble/link time (cfg-gated Rust `extern "C"` symbols with the same names is the cleanest — the asm call sites stay untouched if the per-platform `pre_mmu/util.rs` exports matching `#[unsafe(no_mangle)]` symbols).
2. **DTB pointer** (line 49: `mov x27, x0`; line 127-128: passed to `init_vm`; lines 152-155: KZERO-offset passed to `main9`) — virt ELF boot delivers x0=0 (research P1); the Rust side (vminit/main9) applies the fixed-`0x40000000` fallback, keeping asm unchanged is preferred. If asm-side patching is chosen, the x27 cache is the single touchpoint.

Also note line 62's stale comment ("Qemu starts in EL3") — research verified EL1 entry on qemu 11.1.2; comment-only tidy, optional.

---

### `aarch64/src/pre_mmu/util.rs` (MOD — early-boot debug UART)

**Analog:** itself (141 lines). The existing structure is exactly the per-platform shape D-02 wants: `init_early_uart_rpi4` (lines 30-79, raw `write_volatile` to fixed PA constants), `init_early_uart_putc` (lines 82-90, poll LSR bit5 → write data reg), `putstr`/`putu64h` helpers (92-108) built on putc.

The virt variant mirrors this with PL011 at fixed PA `0x09000000` and **zero init** (qemu pre-configures; DR writes work even with UARTEN clear — research Pattern 3). putc polls PL011 FR bit5 (TXFF) — identical poll shape to lines 84-88:

```rust
// pre_mmu/util.rs:83-90 — the poll-then-write shape to mirror
pub extern "C" fn init_early_uart_putc(b: u8) {
    unsafe {
        while read_volatile(AUX_MU_LSR as *const u32) & (1 << 5) == 0 {
            core::hint::spin_loop();
        }
        write_volatile(AUX_MU_IO as *mut u32, b as u32);
    }
}
```

Per-platform selection: keep one symbol name (`init_early_uart` / `init_early_uart_putc`) with `#[cfg(platform = …)]` choosing the body, so `l.S` call sites don't fork.

---

### `aarch64/src/pre_mmu/vminit.rs` + `aarch64/src/kmem.rs` (MOD — base_physaddr + identity map)

**Analogs:** themselves — targeted fixes at known locations (research P3/P4, line-verified).

`kmem.rs` (lines 17-19, 57-59, 77-79):

```rust
fn base_physaddr() -> PhysAddr {
    PhysAddr::new(0)     // ← on virt this makes boottext_physrange() = 0..0x40090000
}
```

`vminit.rs` (lines 36-38 duplicate the same `base_physaddr()`; lines 84-86 build `boottext_physrange` from it; the kernel-text map at 122-131 consumes it):

```rust
// vminit.rs:36-38 — same poisoned base
fn base_physaddr() -> PhysAddr {
    PhysAddr::new(0)
}
```

Fix per research P3: start kernel ranges at the linker's `boottext_pa` (already `PROVIDE`d by `kernel.ld:18`, already declared `static boottext_pa` — wait, only `eboottext_pa` is currently declared in both files' `unsafe extern "C"` blocks; add `boottext_pa` import). Raspi may deliberately keep 0 (the `vminit.rs:116` TODO contemplates unmapped page 0) — platform-parameterize.

Identity map (`vminit.rs:197-205`) currently installs a single 1 GiB block at PA 0:

```rust
physicalpt3.entries[0] = Entry::empty()
    .with_phys_addr(PhysAddr::new(0))
    .with_valid(true)
    .with_page_or_table(false)
    .with_accessed(true)
    .with_access_permission(AccessPermission::PrivRw)
    .with_shareable(Shareable::Inner)
    .with_uxn(true)
    .with_mair_index(Mair::Normal);
```

virt kernel at PA `0x40080000` is outside that window → MMU-on jump faults (P4). Copy this exact `Entry` builder chain into a conditional `physicalpt3.entries[1]` with `PhysAddr::new(0x40000000)` (or platform-parameterized base) — same flags, block mapping.

---

### `aarch64/Cargo.toml` (MOD — crate config)

**Analog:** `riscv64/Cargo.toml` (exact — add the `[lints.rust] unexpected_cfgs` block shown in the `platform/mod.rs` assignment above). No dependency changes this phase (research: kernel needs zero new deps; `aarch64-cpu = "11.2"` already present at line 10).

---

### `xtask/src/config.rs` (MOD — D-07 `[qemu]` schema extension)

**Analog:** itself. Extend the `Qemu` struct (lines 73-80) with `cpu`/`smp`/`memory`/`accel` Option fields following the existing `machine`/`dtb` style:

```rust
// xtask/src/config.rs:73-80 — current shape
#[derive(Debug, Serialize, Deserialize)]
pub struct Qemu {
    /// Machine (`-M`) value for qemu: raspi3b, raspi4b, etc.
    pub machine: Option<String>,

    /// Filepath of DTB file relative to crate
    pub dtb: Option<String>,
}
```

and mirror each in `apply_qemu_config` (lines 258-269), whose current per-key `if let Some(…)` → `cmd.arg(…)` shape is the pattern to replicate:

```rust
// xtask/src/config.rs:258-269
fn apply_qemu_config(cmd: &mut Command, config: &Configuration) {
    if let Some(config) = &config.qemu {
        if let Some(machine) = &config.machine {
            cmd.arg("-M");
            cmd.arg(machine);
        }
        if let Some(dtb) = &config.dtb {
            cmd.arg("-dtb");
            cmd.arg(dtb);
        }
    }
}
```

(`-accel`/`-cpu`/`-smp`/`-m` map the same way; `smp` is numeric — existing TOML handling elsewhere is stringly-typed, so `smp = 1` as integer + `.to_string()` or string — planner's pick, serde handles either.)

---

### `xtask/src/main.rs` (MOD — DistStep/QemuStep platform-awareness + boot-test)

**Analog:** itself — three distinct work sites, all following existing in-file patterns.

**(a) DistStep platform-aware (D-05):** current aarch64 arm (lines 351-390) unconditionally objcopy→gzip. It must become config-aware (skip for ELF platforms like virt) — meaning `DistStep` gains a `config` field, constructed exactly like `BuildStep::new` does (lines 296-303):

```rust
// xtask/src/main.rs:296-303 — the ctor idiom every Step uses
fn new(matches: &clap::ArgMatches) -> Self {
    let arch = Arch::from(matches);
    let config = load_config(arch, matches);
    let profile = Profile::from(matches);
    let verbose = verbose(matches);
    Self { arch, config, profile, verbose }
}
```

Note the subcommand dispatch (lines 215-228) currently runs `s1.run().and_then(|_| s2.run())` for `dist`/`qemu` — DistStep's new() signature stays the same, so dispatch is untouched.

**(b) QemuStep aarch64 arm (D-06/D-07, research P7/P12):** current raspi-specific serial routing (lines 469-501):

```rust
// xtask/src/main.rs:475-492
// TODO Choose UART at cmdline
// If using UART0 (PL011), this enables serial
cmd.arg("-nographic");

// If using UART1 (MiniUART), this enables serial
cmd.arg("-serial");
cmd.arg("null");
cmd.arg("-serial");
cmd.arg("mon:stdio");
…
cmd.arg("-kernel");
cmd.arg(format!("target/{target}/{dir}/aarch64-qemu.gz"));
```

On virt: `-serial mon:stdio` only (PL011 is serial0), `-kernel` points at the raw ELF `target/{target}/{dir}/aarch64` (no `.gz`), and accel/cpu/smp/memory come from the TOML via the extended `apply_to_qemu_step` (already wired in at line 473 — it just gets new keys). The riscv64 arm (lines 502-539) shows the simpler `-serial mon:stdio` + `-smp`/`-m` shape to converge toward; its `--dump_dtb` handling (lines 507-512) is the precedent for qemu-flag-from-config plumbing.

**(c) NEW `boot-test` subcommand (D-11):** register in the clap builder (pattern: lines 128-209 — copy the `qemu` subcommand's arg set: `--release/--debug/--arch/--config/--verbose`); dispatch in the `match matches.subcommand()` (lines 211-230); implement as a `BootTestStep` struct with the same `new()`/`run() -> Result<()>` shape as `TestStep` (lines 659-715). The qemu spawn + assert logic is new code per RESEARCH.md "Boot-test harness" (zero-dep unix-socket design: `-chardev socket,id=cons,path=<tmp>.sock,server=on,wait=on` + `-serial chardev:cons` + `std::os::unix::net::UnixStream`; capture-before-start via `wait=on` is the Phase 1 lesson). Process-spawn/assert idiom to copy: `annotated_status(&mut cmd)?` + `if !status.success() { return Err("…".into()) }` (e.g. lines 328-331) — though boot-test will want `Child` (spawn/stdout pipes/kill) rather than `status()`, so it diverges deliberately there; `Command::new(qemu_system)` + `apply_to_qemu_step` reuse stays. Evidence log written out per D-13 (canonical copies land in this Harvey phase dir; r9 commit SHA + r9-side evidence paths recorded here).

---

### `README.md` + `HowItWorks.md` (MOD — docs)

**Analogs:** themselves. README: add the virt row to the qemu invocation table (current table at README lines ~40-46: `|aarch64|raspi3b|cargo xtask qemu --arch aarch64 --verbose|` etc.) and fix the "Currently only Raspberry Pi 3 is supported" line (~line 16). HowItWorks.md: its header mandates "Please update with any material code change!" — the aarch64 early-init section (describing `l.S` as raspi-focused) gains the virt platform tree + ELF/`-dtb` contract. Match existing prose style (terse, design-tour).

---

### `harvey:.planning/phases/03-r9-qemu-virt-platform-bring-up/` evidence copies (NEW)

**Analog:** Harvey phase-dir convention — Phase 1 and 2 phase dirs carry `0X-VERIFICATION.md` + per-plan `SUMMARY.md` files; evidence (captured serial logs, asserted markers, human-checkpoint record) lands alongside per D-13. No code pattern; the planner should create an evidence subdirectory convention consistent with `01-VERIFICATION.md`'s captured-log + checkpoint structure (boot/execution-test with captured serial evidence — carried-forward proof standard).

## Shared Patterns

### DTB device discovery (D-04) — the dominant cross-cutting idiom

**Source:** `riscv64/src/platform/virt/devcons.rs:10-15`; also `aarch64/src/uartpl011.rs:60-67`, `aarch64/src/uartmini.rs:80-108`
**Apply to:** `platform/virt/{devcons,pl011,gic,timer}.rs`, virt `print_machine_info`

```rust
let node_reg = dt
    .find_compatible("arm,pl011")                       // port/src/fdt.rs:389
    .next()
    .and_then(|dev| dt.property_translated_reg_iter(dev).next())  // fdt.rs:300
    .and_then(|reg| reg.regblock())
    .unwrap();                                          // or .ok_or("can't find x")?
```

Variants in-tree: `.ok_or("can't find pl011")` (uartpl011.rs:66) for `Result` propagation; `.or_else(|| dt.find_compatible("brcm,bcm2711-gpio").next())` (uartmini.rs:81-83) for multi-compatible fallback — **the gic.rs driver needs exactly this or_else shape** for `"arm,cortex-a15-gic"` vs `"arm,gic-400"` (research correction to D-04). Interrupt-cells parsing uses `property_value_as_u32_iter` (`fdt.rs:170`); memory size uses `find_device_type("memory")` (`fdt.rs:402`).

### Console installation (`Console::set_uart` + SyncUnsafeCell)

**Source:** `riscv64/src/platform/virt/devcons.rs:17-28` (canonical), `aarch64/src/devcons.rs:31-53` (error-tolerant variant)
**Apply to:** `platform/virt/devcons.rs`, moved `platform/raspi*/devcons.rs`

`Console::set_uart(|| …)` takes a closure returning `Result<&'static mut dyn Uart>` (`port/src/devcons.rs:31-38`); the `static X: SyncUnsafeCell<MaybeUninit<T>>` + `assume_init_mut()` dance is how the 'static reference is manufactured. `Uart` trait is **output-only** (`putb`, `port/src/devcons.rs:20-22`) — SC#4 input is a new path, and IRQ-context output must **bypass** the Console lock (research P8; lock at `port/src/devcons.rs:24,40-50`).

### Bounds-checked MMIO register access

**Source:** `aarch64/src/io.rs:21-43`
**Apply to:** all virt drivers (pl011, gic)

`write_reg(&VirtRange, offset, val)` / `read_reg(&VirtRange, offset) -> u32` / `write_or_reg` — volatile, panics on out-of-bounds offset. Devices get their `VirtRange` from `map_device_register(id, physrange, PageSize::Page4K)` (`aarch64/src/deviceutil.rs:9-34`), which maps via the kernel page table at `vm::next_free_device_page4k()` with `Entry::rw_device()`. Register offset constants follow `aarch64/src/registers.rs:15-22` style (`pub const UART0_DR: usize = 0x00; // comment`).

### Platform cfg gating

**Source:** `riscv64/src/platform/mod.rs:1-9` (module gating), `xtask/src/config.rs:144-147` (`platform = "x"` → `--cfg platform="x"`), `riscv64/Cargo.toml` (`[lints.rust] unexpected_cfgs`)
**Apply to:** `platform/mod.rs`, `main.rs` (raspi-only mailbox/board-info/test_sysexit), `pre_mmu/util.rs` (early uart body), `Cargo.toml` (check-cfg list)

### Error handling

**Source:** `aarch64/src/uartpl011.rs:27-49`, `aarch64/src/mailbox.rs:25-36`, `port/src/devcons.rs` (Result)
**Apply to:** all new Rust files

r9 kernel style: `port::Result<T>` = `Result<T, &'static str>`; recoverable init failures `println!("can't …")` + `Err("…")` and the kernel continues (mailbox init tolerates failure, devcons returns Err); unrecoverable invariant violations `panic!()`. New platform init should follow the tolerant style for device discovery (DTB absence = print + continue), hard panic only for VM/memory setup (as `main.rs:126-128` pagealloc does).

### Panic path (D-12)

**Source:** `aarch64/src/runtime.rs:13-19` (panic handler prints + spins), `aarch64/src/trap.rs:62-102` (`TrapFrame` Debug — the ready-made register dump)
**Apply to:** `config_virtpanic.toml` plumbing + the panic-at-end-of-main9 hook

Research recommends the `brk #0` route (deliberate sync exception → real trap path → existing TrapFrame dump + clean-halt marker + `wfi` loop). The `runtime.rs` handler's `println!("{}\n", info); loop {}` is the fallback shape if the `panic!()` route is chosen — it would need extension for register capture.

### xtask step idiom

**Source:** `xtask/src/main.rs` — `struct XStep { … }` + `fn new(matches) -> Self` + `fn run(self) -> Result<()>`, registered in the clap `main()` (lines 122-235); `env_or("VAR", "default")` env-override helper (lines 237-240); `load_config(arch, matches)` resolves `lib/config_<name>.toml` (lines 273-282); `annotated_status` + `Err("step failed".into())` on non-zero exit.
**Apply to:** the new `BootTestStep`, and the modified DistStep/QemuStep.

`Result<T> = std::result::Result<T, Box<dyn std::error::Error>>` (lines 13-14) — xtask-side error handling, distinct from kernel `port::Result`.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `aarch64/src/platform/virt/gic.rs` | device driver | MMIO IRQ controller | No interrupt-controller driver exists anywhere in r9 (riscv64 virt/nezha have no PLIC/CLINT code; x86_64 trap is PIC-less). Use RESEARCH.md Pattern 1 (verified register sequence) + shared MMIO/DTB idioms above. |
| `aarch64/src/platform/virt/timer.rs` | device driver | sysreg + periodic IRQ | No timer code in r9. Use RESEARCH.md Pattern 2 + `aarch64-cpu` crate (already a dep). |
| `xtask boot-test` socket-capture internals | acceptance harness | spawn/capture/assert | xtask has no child-process *interaction* today (all steps use `status()`). The unix-socket + `UnixStream` + marker-assert design is specified in RESEARCH.md "Boot-test harness"; struct/registration idiom copies `TestStep`. |
| `flake.nix` rust-overlay wiring | nix config | declarative | Harvey's flake has no rust toolchain (different stack); the `fromRustupToolchainFile` mechanism is specified in RESEARCH.md (cited from oxalica/rust-overlay docs). Shape/conventions copy Harvey's flake. |

## Metadata

**Analog search scope:** `/Users/neilskinner/code/r9` (riscv64/src/platform/, aarch64/src/, aarch64/lib/, xtask/src/, port/src/, riscv64/lib/, both Cargo.tomls, rust-toolchain.toml, README.md, HowItWorks.md), `/Users/neilskinner/code/harvey` (flake.nix, .planning/phases/01-*, 02-*)
**Files scanned:** 31 read in full or targeted sections
**Tracked-source check:** all cited analog paths verified via `git ls-files` in their respective repos (r9 HEAD `33a9273`; no capability-mirror paths involved)
**Pattern extraction date:** 2026-11-01
