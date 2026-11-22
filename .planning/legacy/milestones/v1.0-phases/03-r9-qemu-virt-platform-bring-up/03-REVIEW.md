---
phase: 03-r9-qemu-virt-platform-bring-up
reviewed: 2026-11-01T00:00:00Z
depth: standard
files_reviewed: 31
files_reviewed_list:
  - aarch64/Cargo.toml
  - aarch64/lib/config_default.toml
  - aarch64/lib/config_raspi4b.toml
  - aarch64/lib/config_virt.toml
  - aarch64/lib/config_virtpanic.toml
  - aarch64/lib/virt-gicv2.dts
  - aarch64/src/kmem.rs
  - aarch64/src/l.S
  - aarch64/src/main.rs
  - aarch64/src/platform/mod.rs
  - aarch64/src/platform/raspi3b/devcons.rs
  - aarch64/src/platform/raspi3b/mailbox.rs
  - aarch64/src/platform/raspi3b/mod.rs
  - aarch64/src/platform/raspi3b/uartmini.rs
  - aarch64/src/platform/raspi3b/uartpl011.rs
  - aarch64/src/platform/raspi4b/mod.rs
  - aarch64/src/platform/virt/devcons.rs
  - aarch64/src/platform/virt/gic.rs
  - aarch64/src/platform/virt/mod.rs
  - aarch64/src/platform/virt/pl011.rs
  - aarch64/src/platform/virt/timer.rs
  - aarch64/src/pre_mmu/util.rs
  - aarch64/src/pre_mmu/vminit.rs
  - aarch64/src/trap.rs
  - flake.nix
  - HowItWorks.md
  - README.md
  - xtask/src/config.rs
  - xtask/src/main.rs
  - flake.lock
findings:
  critical: 0
  warning: 5
  info: 10
  total: 15
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-11-01
**Depth:** standard
**Files Reviewed:** 31 (30 source files line-reviewed; `flake.lock` verified committed and consistent with flake.nix's inputs — nodes `nixpkgs` + `rust-overlay` match the declared inputs, per instructions no line review)
**Status:** issues_found

## Summary

Reviewed the virt platform bring-up diff (base 33a9273) file-by-file, tracing the phase delta with `git diff` per file and cross-checking called context outside the listed files (trap.S vector layout vs. `IRQ_INVALID_EL1T/H` constants, `Entry::rw_device` MAIR index, kernel.ld section layout vs. the `bss_pa..ebss_pa` reservation covering `.earlyvm_pagetables`, TrapFrame stack layout vs. asm save order — all verified consistent). The core virt path is sound: the GICv2 register sequences, PL011 IMSC/ICR/LCRH/CR values, CNTP arming, P5 ordering (GIC → timer → UART RX → unmask), the x0==0 DTB fallback twins in `init_vm`/`main9`, and the boot-test marker harness all check out against the hardware contracts. No blocker-level defects found.

The five warnings concentrate on (a) the never-torn-down TTBR0 identity map undercutting the phase's own P3 MMIO-hazard mitigation on virt, (b) `boot-test` accepting non-virt configs into a guaranteed 300 s failure, (c) a pre-existing wrong-MMIO-window bug in the raspi PL011 GPIO helper exposed by the file move, (d) a pre-existing error-swallowing loop in `map_phys_range` that the new virt mapping path depends on, and (e) DTB-rediscovered timer INTID being advisory-only while enable/dispatch stay hardcoded.

Known-and-accepted context (raspi3b-on-qemu-11.1.2 deferral, IRQ-context println discipline under the wfi-only idle loop, virtpanic as a deliberate debug artifact) was treated as out of scope per the review brief and is not flagged.

## Warnings

### WR-01: TTBR0 identity map keeps GIC/PL011 MMIO Normal-cacheable (and EL1-executable) forever on virt

**File:** `aarch64/src/pre_mmu/vminit.rs:254-262, 269-280` (also `aarch64/src/kmem.rs:19-33`)
**Issue:** The phase's P3 mitigation moves the higher-half kernel ranges to start at `boottext_pa` (0x40080000) so "the GIC (0x08000000) and PL011 (0x09000000) MMIO windows" never become Normal-cacheable kernel text (kmem.rs comment). But the *other* mapping of those windows survives: `physicalpt3.entries[0]` identity-maps PA 0–1 GiB as `Mair::Normal`, `PrivRw`, `Shareable::Inner`, with `uxn` set but **PXN never set** — i.e. the GIC/PL011 MMIO window remains Normal-cacheable and EL1-executable through TTBR0. The comment at vminit.rs:243-244 claims "once we've jumped to the higher half, this will no longer be used," yet nothing ever switches or clears TTBR0_EL1 on virt: raspi's `vm::init_user_page_tables()`/`vm::switch(...User)` experiment (which replaces ttbr0) is `#[cfg(not(platform = "virt"))]`-gated in main.rs:193-201, so on virt the identity map is live for the entire post-boot lifetime — exactly the speculative-fetch exposure the phase set out to remove, plus an EL1-executable low window. Benign under qemu TCG today; a real hazard on stricter hardware or once more code runs.
**Fix:**
```rust
// After the higher-half jump (e.g. first thing in main9 on virt), tear the
// identity root down — point TTBR0 at a minimal empty root table, or at
// minimum invalidate the two block entries and `tlbi vmalle1`/`dsb`/`isb`.
// Cheaper interim: build entries[0]/entries[1] with .with_pxn(true) and,
// for the MMIO-covered entries[0], Mair::Device.
```

### WR-02: `boot-test` accepts non-virt configs into a guaranteed 300 s failure

**File:** `xtask/src/main.rs:850-921` (guard at 851-853; kernel path at 858)
**Issue:** `BootTestStep::run` only rejects non-aarch64 arches. With the default `--config default` (raspi3b) the harness (1) boots the raw ELF `target/{target}/{dir}/aarch64` (line 858) while raspi's qemu contract needs the objcopy+gzip flat image (`aarch64-qemu.gz` — and DistStep is never run in the boot-test chain), (2) wires the single `-serial chardev:cons` socket to the raspi machine's PL011 while the raspi console driver is the miniuart, and (3) waits for tick/echo markers that exist only on the virt platform. Every raspi-config invocation therefore burns the full `BOOT_TEST_TIMEOUT_SECS` (300 s) and fails with a misleading "banner marker not observed" tail dump. `cargo xtask boot-test --arch aarch64` with no flags — the most discoverable invocation — hits exactly this.
**Fix:**
```rust
let platform = self.config.config.as_ref().and_then(|c| c.platform.as_deref());
if platform != Some("virt") {
    return Err(format!(
        "boot-test markers (banner/ticks/echo) exist only on platform=virt; \
         config {:?} selects {:?}", self.config_name, platform).into());
}
```

### WR-03: raspi PL011 `gpiosetpull` writes GPIO registers through the PL011 MMIO window

**File:** `aarch64/src/platform/raspi3b/uartpl011.rs:113-126` (`gpiosetpull`)
**Issue:** All six writes use `&self.pl011_virtrange` but the offsets (`GPPUD` = 0x94, `GPPUDCLK0` = 0x98, from registers.rs:11-12) are GPIO-block offsets — the struct carries `gpio_virtrange` precisely for this. As written, `gpiosetpull` pokes offsets 0x94/0x98 in the PL011 register space (past the defined PL011 register block) and never touches the real GPIO pull-up/down registers. Pre-existing (the file moved with only import changes this phase) and currently latent because raspi devcons always picks `MiniUart` and `Pl011Uart` is `#[allow(dead_code)]` — but the driver is one call-site away from live use, and this failure mode (silent writes into the wrong peripheral) is exactly the kind that costs hours to find on real hardware.
**Fix:** In `gpiosetpull`, change every `write_reg(&self.pl011_virtrange, …)` to `write_reg(&self.gpio_virtrange, …)` (the GPPUD/GPPUDCLK writes; nothing in that helper touches PL011 registers).

### WR-04: `map_phys_range` swallows mid-range `map_to` failures and returns `Ok` for a partial map

**File:** `aarch64/src/pre_mmu/vminit.rs:427-443`
**Issue:** Inside the `for pa in range.step_by_rounded(...)` loop, a `map_to` failure is only reported via early-UART `putstr` — iteration continues and the function ultimately returns `Ok(start..end)` spanning the full requested range. The one caller (`init_vm`, lines 216-227) panics on `Err` only, so a mid-range failure (e.g. `EarlyPageAllocator` exhaustion on the larger virt mapping footprint) leaves silent holes in the kernel map and boot continues into a fault elsewhere. Pre-existing, but the new virt path (4K text pages → ~10x more `map_to` calls than raspi's 2 MiB path) leans on it much harder.
**Fix:**
```rust
if let Err(err) = map_to(root_page_table, page_allocator, entry.with_phys_addr(pa), currva, page_size) {
    putstr("error:vminit:map_phys_range:map_to failed: ");
    putstr(debug_name);
    putstr("\n");
    return Err(err); // fail the whole range instead of returning a partial map as Ok
}
```

### WR-05: DTB-rediscovered timer INTID is advisory-only; enable and dispatch stay hardcoded at 30

**File:** `aarch64/src/platform/virt/timer.rs:33-58`, `aarch64/src/main.rs:224-227`, `aarch64/src/trap.rs:145-146`
**Issue:** `timer::init` re-discovers CNTP's INTID from the DTB and prints a warning if it ever differs from `CNTP_INTID` (30) — but the discovered value is then discarded: main9 already ran `platform::gic::enable(30, 0xA0)` with a literal, and `dispatch_irq` matches a literal `30`. On a hypothetical DTB where CNTP is not PPI 14, the warning prints and the system then silently never delivers the timer interrupt (GIC never enables the real INTID; dispatcher wouldn't recognize it). This also sits awkwardly next to the phase's "no hardcoded virt address tables" design rule (virt/mod.rs:14). Low likelihood — CNTP's PPI 14 assignment is architecturally fixed — so warning not blocker, but the discovery code creates false confidence.
**Fix:** Have `timer::init` (or a small `timer::intid()` accessor built in `init`) own the discovered INTID; call it before `gic::enable` in main9 (`let intid = platform::timer::init(&dt); platform::gic::enable(intid, 0xA0);`) and match `dispatch_irq` against it instead of the literal 30 — or delete the rediscovery and keep the documented constant, one source of truth either way.

## Info

### IN-01: Dead `mrs x0, elr_el1` before the higher-half jump

**File:** `aarch64/src/l.S:141`
**Issue:** `mrs x0, elr_el1` reads ELR into x0, which is never consumed — the next two instructions load x20 with `higher_half` and branch. Leftover from an earlier jump scheme.
**Fix:** Delete the line (or comment why ELR is sampled).

### IN-02: x28 "entrypoint" cached from x4 but never used

**File:** `aarch64/src/l.S:46, 50`
**Issue:** `mov x28, x4` caches an entrypoint-offset that nothing reads — not in l.S, not in `init_vm` (x0 only), not in `main9` (dtb_va only). On qemu's ELF entry x4 is architecturally undefined, so the cached value is noise on virt anyway. Vestigial state in a register the comment says "shouldn't be used by any code in this file."
**Fix:** Drop the cache (and the comment), or document the boot-stub contract that consumes it if one is planned.

### IN-03: earlyvm page tables zeroed four times

**File:** `aarch64/src/l.S:109-120`, `aarch64/src/pre_mmu/vminit.rs:557-561, 572`
**Issue:** l.S clears `bss_pa..ebss_pa` (which already spans `.earlyvm_pagetables` because `ebss_pa = LOADADDR(.end)` sits after it in kernel.ld), then clears `earlyvm_pagetables_pa..eearlyvm_pagetables_pa` again; `EarlyPageAllocator::new` clears all 32 pages a third time; `alloc_physpage` clears each page a fourth time at handout. All harmless, all redundant.
**Fix:** Keep the l.S bss clear and the per-page clear in `alloc_physpage`; drop the other two.

### IN-04: `apply_link` panics on a `[link]` table missing `script`; redundant nested `if let`

**File:** `xtask/src/config.rs:236, 249-258`
**Issue:** `link["script"].clone()` indexes the HashMap directly — a config TOML with `[link]` but no `script` key crashes xtask with a bare "no entry found for key" panic instead of a named diagnostic. Inside the block, `if let Some(link) = &config.link` re-tests and shadows the binding from the outer `if let` that already established `Some`.
**Fix:** `let Some(filename) = link.get("script") else { eprintln!("config {filename}: [link] table requires a 'script' key"); exit(1); };` and drop the inner re-binding.

### IN-05: `-Z build-std=core,alloc` passed twice on every aarch64 build

**File:** `xtask/src/main.rs:364` vs. `aarch64/lib/config_*.toml` `[build] buildflags`
**Issue:** `BuildStep::run` appends `-Z build-std=core,alloc` unconditionally, and every aarch64 config TOML supplies the identical flag via `apply_build`. Cargo accepts the duplication; it's noise and a drift risk if one copy ever changes.
**Fix:** Keep the TOML `buildflags` as the single source and delete the hardcoded push (or vice versa).

### IN-06: Reference-to-temporary in `dump_dtb` default

**File:** `xtask/src/main.rs:506-511`
**Issue:** `.unwrap_or(&"".to_string()).clone()` takes a reference to a temporary `String`; it compiles only because the temporary lives to the end of the statement. Fragile idiom — a future refactor that splits the expression becomes a borrow error.
**Fix:** `.cloned().unwrap_or_default()` (or `.map(String::as_str).unwrap_or("").to_string()`).

### IN-07: `--config` name interpolated into a filesystem path unsanitized

**File:** `xtask/src/main.rs:313-322` (`load_config`)
**Issue:** `format!("{}/{}/lib/config_{}.toml", …)` with a CLI-supplied name permits `../` traversal (`--config ../../x`). Dev-tool self-attack surface only, but a one-character fix.
**Fix:** Reject names not matching `[A-Za-z0-9_-]+` before formatting the path.

### IN-08: "logged once" comment vs. log-every-IRQ implementation

**File:** `aarch64/src/trap.rs:132-136, 150`
**Issue:** The `dispatch_irq` doc comment says an unknown INTID "is logged once"; the `_ => println!("unhandled IRQ {intid}")` arm logs *every* occurrence. A stuck level-triggered stray line would flood the console from IRQ context (the same lock-free discipline the file documents for tick/echo).
**Fix:** Correct the comment to "logged (rate-unlimited)", or add a per-INTID once-cell if the flood matters.

### IN-09: HowItWorks.md identity-window and stack-size claims are wrong

**File:** `HowItWorks.md` ("Early initialisation" items 4-5)
**Issue:** (a) "raspi maps PA 0–2MiB (kernel at `0x80000`)" — the raspi identity window is a single 1 GiB L1 block descriptor (`physicalpt3.entries[0]`, vminit.rs:254-262), not 0–2 MiB; the very next clause ("virt adds a second 1GiB block") implies the first is also 1 GiB, contradicting the 2 MiB figure. (b) "the initial stack (size set by STACKSZ - 4KiB)" — `STACKSZ = 4096*4` (16 KiB, l.S:3).
**Fix:** "raspi maps PA 0–1GiB (kernel at 0x80000)" and "size set by STACKSZ — 16KiB".

### IN-10: flake.nix objcopy hook can export an empty `OBJCOPY`

**File:** `flake.nix:28-33`
**Issue:** If the `llvm-objcopy` glob matches nothing (e.g. `llvm-tools` dropped from rust-toolchain.toml), `$(ls … | head -1)` evaluates to the empty string and the hook exports `OBJCOPY=""`. xtask's `env_or("OBJCOPY", …)` (main.rs:310) treats a set-but-empty var as authoritative, so `dist` then fails with `"": No such file or directory` — a confusing error far from the cause. Latent today: rust-toolchain.toml includes `llvm-tools`, so the glob always matches.
**Fix:** In `objcopyHook`, only export when the resolved path is non-empty (`[ -n "$found" ] && export OBJCOPY="$found"`), letting xtask fall back to its bare-name default otherwise.

---

_Reviewed: 2026-11-01_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
