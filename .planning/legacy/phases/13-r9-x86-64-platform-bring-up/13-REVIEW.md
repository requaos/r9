---
phase: 13-r9-x86-64-platform-bring-up
reviewed: 2026-11-07T17:29:27Z
depth: standard
files_reviewed: 34
files_reviewed_list:
  - ../r9/x86_64/lib/config_q35.toml
  - ../r9/x86_64/lib/grub.cfg
  - ../r9/x86_64/src/l.S
  - ../r9/x86_64/src/main.rs
  - ../r9/Cargo.toml
  - ../r9/xtask/src/main.rs
  - ../r9/xtask/src/config.rs
  - ../r9/flake.nix
  - ../r9/x86_64/src/platform/mod.rs
  - ../r9/x86_64/src/platform/q35/mod.rs
  - ../r9/x86_64/src/platform/q35/mb2.rs
  - ../r9/x86_64/src/platform/q35/devcons.rs
  - ../r9/x86_64/src/pagealloc.rs
  - ../r9/x86_64/src/seg.rs
  - ../r9/x86_64/src/dat.rs
  - ../r9/x86_64/src/cpu.rs
  - ../r9/x86_64/src/pio.rs
  - ../r9/x86_64/Cargo.toml
  - ../r9/x86_64/src/platform/q35/lapic.rs
  - ../r9/x86_64/src/platform/q35/timer.rs
  - ../r9/x86_64/src/trap.rs
  - ../r9/x86_64/src/uart16550.rs
  - ../r9/x86_64/src/platform/q35/acpi.rs
  - ../r9/x86_64/src/platform/q35/pci.rs
  - ../r9/x86_64/lib/config_q35lowmcfg.toml
  - ../r9/x86_64/lib/q35-mcfg-moved.bin
  - ../r9/x86_64/src/platform/q35/ioapic.rs
  - ../r9/x86_64/src/platform/q35/netgate.rs
  - ../r9/x86_64/lib/config_q35net.toml
  - ../r9/port/src/devether.rs
  - ../r9/xtask/src/pcap.rs
  - ../r9/.github/workflows/xtask.yml
  - build/x86_64/README.md
  - .planning/phases/13-r9-x86-64-platform-bring-up/13-EVIDENCE.md
  - .planning/phases/13-r9-x86-64-platform-bring-up/deferred-items.md
findings:
  critical: 1
  warning: 4
  info: 4
  total: 9
  necessity: 2
status: issues_found
---

# Phase 13: Code Review Report

**Reviewed:** 2026-11-07T17:29:27Z
**Depth:** standard
**Files Reviewed:** 34
**Status:** issues_found

## Summary

Reviewed the full Phase-13 scope: the r9 x86_64 q35 platform (l.S early boot + mb2 header/stash, main9, mb2/acpi parsers, LAPIC/timer/IOAPIC, PCI/ECAM discovery + virtio-net bind + netgate, page allocator, trap/dispatch), the xtask build/boot-test/pcap harness changes (change-focused against r9 base `581e3c3`), the three q35 TOML configs, the flake's kernel-x64/boot-iso derivations, the new CI job, and the Harvey-side `build/x86_64/README.md`. The two `.planning` artifacts were consistency-checked only (not source); the `q35-mcfg-moved.bin` fixture was verified by use — it exists (60 bytes), is committed, and is referenced by `config_q35lowmcfg.toml`'s `[qemu] extra` `-acpitable file=` tokens, which `apply_qemu_config` emits verbatim with qemu cwd = workspace.

Overall assessment: the kernel-side code is unusually disciplined (bounds-checked firmware parsing, publish-once cells, fail-closed reconciliation, honest provenance prints), and the diff-focused checks of the arming-window ordering, the x2APIC/0x6E0 gating, the MADT/MCFG gating, and the netdev/extra emission all held up under tracing. However, one genuine frame-arithmetic bug exists in the trap entry path (`intrspecial`'s SWAPGS witness bit is written into the saved-RIP slot instead of the saved-CS slot), the new q35net boot-test stage anchor reintroduces exactly the same-burst satisfaction-anchor hazard the file documents and fixed one stage earlier, the CI job has no timeout, and there is a dead ISA-override fold chain whose `allow(dead_code)` justification comments are contradicted by the landed bind code.

## Critical Issues

### CR-01: `intrspecial` marks the SWAPPED witness bit in the saved RIP word, not the saved CS word — wrong slot arithmetic (+16 vs +24)

**File:** `../r9/x86_64/src/trap.rs:239-246` (cross-ref: the correct constant in `intrregular` at `trap.rs:274`, and the return-path `btr` at `trap.rs:348,383`)
**Issue:** Both the special and regular stubs are `pushq $0; callq <handler>`, so at each handler's entry the stack is identical: `[ret][fake-error][RIP][CS][RFLAGS]` at `%rsp+0/+8/+16/+24/+32`. `intrregular` correctly uses `CS_OFFSET = 24` and `SWAPPED_CS_OFFSET + 24 = 31` — byte 31 is the top byte of the 64-bit CS word (hardware pushes CS zero-extended, so bit 63 is free as the SWAPGS witness). `intrspecial` pops its scratch pushes before the `orb`, leaving `%rsp` at the same entry value, but its constant is `SWAPPED_CS_OFFSET + 16 = 23` — byte 23 is the **top byte of the saved RIP**, not CS. Consequences when the `swapgs` branch is taken (entry with `%GSBASE != Mach`, i.e. an NMI/debug/breakpoint delivered while in user mode):
1. The SWAPPED witness bit is never recorded in CS, so `intrcommon`'s return-path `btr ..., 183(%rsp)` (CS bit 63) never sees it — the compensating `swapgs` on return never executes, leaving the kernel running on the user GS base: precisely the corruption the ~90-line comment above `intrspecial` exists to prevent.
2. `orb $0x80, 23(%rsp)` ORs 0x80 into byte 7 of the saved RIP. A canonical user-mode RIP (top byte 0x00) becomes non-canonical → `iretq` raises #GP instead of returning.

Dormancy caveat: user processes do not exist until Phase 14 (XEXEC), and today `%GSBASE` is always the Mach base at entry, so the buggy branch (`je 1f` not taken) is unreachable on every currently-bootable configuration — this is why all Phase-13 gates are green. The bug is in shipped entry-path code that Phase 14 activates, and the same file's `intrregular` proves the intended constant. Note: introduced in pre-phase commit `0710f69` (present at the phase base `581e3c3`), but `trap.rs` is a listed in-scope file and the phase's IRQ-dispatch work lands directly on top of it.
**Fix:**
```asm
// trap.rs, intrspecial:
-        SWAPPED_CS_OFFSET = const SWAPPED_CS_OFFSET + 16,
+        SWAPPED_CS_OFFSET = const SWAPPED_CS_OFFSET + 24,
```
(Match `intrregular`'s arithmetic: the CS word sits at `%rsp+24` at the `orb`, after the scratch pushes/pops cancel; byte 24+7=31 is its high byte.)

## Warnings

### WR-01: q35net discovery→bind stage anchor is a satisfaction-time `capture.len()` — the same-burst hazard the file documents and fixed one stage earlier

**File:** `../r9/xtask/src/main.rs:1883-1890` (hazard documented and fixed for facts→discovery at `main.rs:1860-1875`; consumer at `main.rs:1900-1908`)
**Issue:** The `Q35Facts`→`Q35PciDiscovery` transition deliberately uses the positional anchor `last_marker_line_end(...)` because, per its own comment, "the kernel prints the whole facts+discovery block in one serial burst, so anchoring at capture length would skip past the discovery lines that arrived in the same socket read". The discovery markers (`q35: mcfg`, `pciexbar: `, `pci: virtio-net 1af4:1041`) and the bind markers (`virtio-net: features offered`, `#l0: virtio:`) are emitted by the same straight-line pre-`sti` boot sequence with no wall-clock gap between them (KVM/TscDeadline path skips both calibration spins; the intervening prints are microseconds apart), so the bind lines routinely land in the same socket read as the discovery lines. When that happens, the `Q35PciDiscovery` satisfaction sets `bind_after = capture.len()` past the already-arrived bind markers — they are excluded forever, `X64Bind` never satisfies, and the gate reds only at overall-budget expiry (300 s KVM / 1800 s TCG). This is a race-dependent flake of the q35net gate (the retained greens show runs that won the read-chunking race), not a deterministic failure — which is the worst kind of gate defect: intermittent reds that look like kernel bugs. (The later `bind_after = capture.len()` resets for X64PingPong/X64TxBulk are safe in practice: those markers arrive seconds later, after 256 slirp ARP round trips; the `ticks_after` anchor is safe because tick lines keep re-arriving at 1 Hz.)
**Fix:** Anchor positionally past the last discovery line, mirroring the facts stage:
```rust
if q35net_profile {
    // Positional anchor: just past the last device-found line — the
    // bind prints ride the same serial burst as the discovery prints.
    bind_after = last_marker_line_end(&capture[pci_after..], PCI_NET_DEVICE_MARKER)
        .map(|p| pci_after + p)
        .unwrap_or(capture.len());
    stage = BootStage::X64Bind;
}
```

### WR-02: `gen_trap_stub!(8)` has a dead second arm — double fault routes through `intrregular`, contradicting the documented special-handling set

**File:** `../r9/x86_64/src/trap.rs:48-82` (documented intent at `trap.rs:115-121`)
**Issue:** Rust macro arms match top-down: `(8) => gen_stub!(err)` (line 48) shadows the later `(8) => gen_stub!(spec)` arm (line 80), which is unreachable dead code. The module's own comment states "NMIs, double faults, and the various debug exceptions fall into this category" of traps requiring the GSBASE-check path (`intrspecial`), because the CS-selector proxy is unreliable for them. With the shadowing, vector 8 (double fault) enters via `intrregular` — exactly the racy CS-selector path the comment says is insufficient for double faults. Today this is benign (no user mode → CS always kernel → the `je` always skips `swapgs`), but it is a dead arm masking a documented-intent divergence that will silently mis-handle DF-from-user in Phase 14.
**Fix:** Give DF both properties — error-word consumption AND the special path — e.g. add a `(8) => { gen_stub!(err_spec) }` shape (a `callq {intrspecial}` without the `pushq $0`, since hardware pushes the error word) and delete the shadowed dead arm; or, minimally, delete the unreachable `(8) => gen_stub!(spec)` arm and correct the comment to drop "double faults" from the special set.

### WR-03: New CI job `boot-test-x86-64` has no `timeout-minutes`

**File:** `../r9/.github/workflows/xtask.yml:62-73`
**Issue:** The new job runs three sequential TCG boot tests, each with an effective overall budget of 300 s × TCG_OVERALL_MULT(6) = 1800 s plus a 20 s × 45 first-marker budget, after apt provisioning and three kernel+ISO builds. A hang anywhere below the harness's own budgets (e.g. qemu wedged on the socket chardev, grub-mkrescue stall) burns ~90+ minutes of hosted-runner time before GitHub's 360-minute default job cap kills it. None of the jobs in this workflow carry `timeout-minutes`, but this job is the long-pole addition and the one whose worst case is measured in hours. (CI/CD check: missing job timeout.)
**Fix:**
```yaml
  boot-test-x86-64:
    runs-on: ubuntu-latest
    timeout-minutes: 75   # 3 × (TCG budget 30 min) + provisioning + builds, with margin
```

### WR-04: Dead ISA-override fold chain whose staging justification is contradicted by the landed bind

**File:** `../r9/x86_64/src/platform/q35/ioapic.rs:86-104,182-191`
**Issue:** `Ioapic::override_applies` and `overrides_fold` (plus the `overrides`/`noverrides` fields) carry `#[allow(dead_code)]` with comments asserting "kernel callers arrive with the bind (the 13-05 dev_ether bind)" and "the bind's ISA-source routing helper (the 13-05 dev_ether bind)". The bind landed in this phase (`pci.rs:1115`) and calls only `ioapic::enable(nic.gsi, ...)` with the PCI-derived GSI — repo-wide grep confirms `override_applies`/`overrides_fold` have **zero** non-test callers (the only references are the definitions, the `allow` annotations, and the unit tests). The machinery is exercised exclusively by its own tests: generality without instantiation, and — worse — the justification comments state a consumer that provably does not exist, so the dead code reads as live wiring. When an ISA-source IRQ consumer actually arrives (the PIT/keyboard era), the fold is a one-line call; until then the comments actively mislead.
**Category:** necessity
**Evidence the consumer cannot occur in this tree:** `grep -rn "override_applies\|overrides_fold" --include="*.rs"` matches only `ioapic.rs` (definition + tests); `pci.rs`'s `bind_inner` routes exclusively via `super::ioapic::enable(nic.gsi, VIRTIO_NET_VECTOR, true)`; no other crate imports `q35::ioapic`.
**Fix:** Either delete `override_applies` + the `overrides`/`noverrides` fields until an ISA-source caller exists (keep `overrides_fold`/`rte_index_for`/`rte_lo` — `enable` consumes those), or keep the chain but fix the comments to state the truth ("no caller yet; the first ISA-source IRQ consumer wires this — pinned by tests only") so the dead code cannot be mistaken for live routing.

## Info

### IN-01: devcons probe's "printed loudly" failure path is a silent no-op

**File:** `../r9/x86_64/src/platform/q35/devcons.rs:38`
**Issue:** On a scratch-readback mismatch the closure does `println!("can't initialise uart: ...")` and returns `Err` — but `port::devcons::Console::set_uart` stores `None` on `Err`, and `port::devcons::print`'s `putstr` drops all output when the console is unset. So the promised loud print goes nowhere; a missing 16550 manifests as a completely silent serial wire (the harness's first-marker budget error is the only diagnostic, and it says nothing about the UART). The comment in `main.rs:113-114` ("the tolerant probe prints loudly if the uart is missing") repeats the false claim.
**Fix:** Fix the comments to match reality (the harness-side budget expiry is the diagnostic), or emit the failure some other way (e.g. a port-0x80 POST-code byte) if loud kernel-side diagnosis is wanted.

### IN-02: Boot-test budget env overrides silently ignore unparseable values

**File:** `../r9/xtask/src/main.rs:1676-1696`
**Issue:** `R9_BOOT_TEST_TIMEOUT` and `R9_BOOT_TEST_FIRST_MARKER_TIMEOUT` are read via `.and_then(|v| v.parse().ok())` — a typo'd value silently falls back to the (possibly TCG-multiplied) default. This contradicts the harness's own stated discipline for sibling env vars (`R9_NET_SWEEP_MODE` / `R9_NETIP_SWEEP` / `R9_NETIP_SOAK_ROUNDS`: "a typo'd value is a harness bug, not a gate failure — validated before qemu spawns, never after a wasted boot").
**Fix:** Validate pre-spawn and error loudly, e.g. `env::var(...).map(|v| v.parse::<u64>().map_err(|e| format!("{name} must be seconds: {v:?}: {e}"))?)?` in `BootTestStep::run` before qemu spawns.

### IN-03: `platform/mod.rs` cfg arms are token-identical — the attributes cannot change what compiles

**File:** `../r9/x86_64/src/platform/mod.rs:14-22`
**Issue:** The four `#[cfg(platform = "q35")]` / `#[cfg(not(platform = "q35"))]` attributes guard byte-identical `pub mod q35;` / `pub use crate::platform::q35::*;` pairs. Whatever the platform cfg state, exactly the same tokens compile — the conditional is a no-op (the documented host-test rationale is satisfied equally by the unconditional form, since `q35`'s console piece is already `cfg(not(test))`-inert). Four attribute lines of pure ceremony that suggest a dispatch which does not exist.
**Category:** necessity
**Evidence the branch difference cannot occur:** both cfg arms' bodies are lexically identical; any evaluation of `platform = "q35"` selects between identical token streams.
**Fix:**
```rust
pub mod q35;
pub use crate::platform::q35::*;
```
with the host-test rationale kept as the comment.

### IN-04: New CI job copies `actions/checkout@v3` — stale floating major, not hash-pinned

**File:** `../r9/.github/workflows/xtask.yml:66` (matching pre-existing jobs at lines 16/26/34/42/50)
**Issue:** The new `boot-test-x86-64` job's `actions/checkout@v3` follows the workflow's house style but perpetuates a mutable-tag pin (v3 is two majors stale; current is v4) with no SHA pinning — a known supply-chain weakness for CI that builds and boots kernel images. Not introduced by this phase's conventions, but the new job is an opportunity to pin.
**Fix:** `uses: actions/checkout@<full-sha> # v4.x.y` for the new job (and opportunistically the rest).

---

_Reviewed: 2026-11-07T17:29:27Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
