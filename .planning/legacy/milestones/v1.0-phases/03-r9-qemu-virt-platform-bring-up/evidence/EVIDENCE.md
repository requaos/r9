# Phase 3 Evidence — r9 qemu-virt Platform Bring-up (03-06, D-13)

Canonical acceptance evidence for ROADMAP Phase 3 success criteria SC#2–SC#5.
Both logs were captured fresh from the exact committed r9 tree named below,
by harness invocations that **build the image they test in the same run**
(evidence can never certify a stale kernel tree — T-03-09).

## Integrity record

| Field | Value |
|-------|-------|
| r9 commit | `55ca7c458952d88245912ad7ad31ca3f55b0e5a6` (`git -C ../r9 rev-parse HEAD`, branch `main`, tree clean at capture) |
| r9 repo | sibling `../r9` (fork `requaos/r9`) |
| qemu | QEMU emulator version **11.1.2** (from the r9 flake devshell's pinned nixpkgs) |
| rust | rustc 1.95.0-nightly (842bd5be2 2026-01-29), from `rust-toolchain.toml` via the devshell |
| Host | **darwin-aarch64** (`Darwin arm64 27.0.0`) |
| Accel | **TCG** (`-cpu cortex-a72`) — HVF rejects GICv2 on qemu 11.1.2 (research P2); TCG is the phase's reference accel |
| Captured | 2026-11-02T00:28Z (both runs, this session) |
| Assertion discipline | marker and count assertions only, **never timings** (P11 — TCG wall-time varies) |

## Exact commands run

From `../r9`, inside the pinned devshell:

```
nix develop -c cargo xtask boot-test --arch aarch64 --config virt
nix develop -c cargo xtask boot-test --arch aarch64 --config virtpanic
```

Each invocation runs `BuildStep` then `BootTestStep`.  The qemu process the
harness spawned (identical for both profiles except the socket/log name,
captured from a `--verbose` run):

```
qemu-system-aarch64 -M virt,gic-version=2 -accel tcg -cpu cortex-a72 -smp 1 -m 1024M \
  -dtb aarch64/lib/virt-gicv2.dtb -display none \
  -chardev socket,id=cons,path=target/boot-test-<config>.sock,server=on,wait=on \
  -serial chardev:cons -kernel target/aarch64-unknown-none-elf/debug/aarch64
```

`server=on,wait=on` holds guest start until the harness connects, so the
capture covers the entire boot from the first byte (capture-before-start).

## Assertion results

### virt-boot.log — standard profile (SC#2 banner, SC#3 ticks, SC#4 IRQ echo)

| # | Assertion | Result | Evidence |
|---|-----------|--------|----------|
| 1 | Boot banner `r9 from the Internet` | **PASS** (1 occurrence) | virt-boot.log |
| 2 | `DTB found at: 0xffff800040000000` (x0=0 ELF-boot fallback to fixed PA 0x40000000 fired) | **PASS** (1) | virt-boot.log |
| 3 | DTB-derived machine info: `Memory: 0x40000000 .. +0x40000000`; `found:` lines for `arm,cortex-a15-gic`, `arm,pl011`, `arm,armv8-timer` | **PASS** (1 each) | virt-boot.log |
| 4 | Periodic timer ticks: ≥ 3 line-initial `tick` rows after the banner (SC#3) | **PASS** (exactly 3 within the harness window; each tick returns through trap.S `eret` to the wfi loop) | virt-boot.log |
| 5 | Typed probe `r9echo7` echoed via the INTID-33 PL011 RX IRQ path (SC#4; qemu socket serial does not self-echo, so the bytes can only reappear through the kernel) | **PASS** (1, in post-write bytes only) | virt-boot.log |
| 6 | Zero wedge markers: no `Unhandled interrupt`, no `panicked` | **PASS** (0 and 0) | virt-boot.log |
| 7 | Harness verdict | **PASS** — `boot-test: PASS -- banner, >= 3 ticks and the echo probe observed in order` (60146 bytes captured) | harness stdout |

### virtpanic-boot.log — panic profile (SC#5: deliberate panic dumps registers and halts cleanly, D-12)

| # | Assertion | Result | Evidence |
|---|-----------|--------|----------|
| 1 | Boot banner present (init completed up to the hook) | **PASS** (1) | virtpanic-boot.log |
| 2 | Start marker `virtpanic: deliberate sync exception via brk #0 (D-12 debug config)` printed at the end of main9, **before** `trap::unmask_irq()` | **PASS** (1) | virtpanic-boot.log |
| 3 | Full TrapFrame register dump: ≥ 30 rows matching `^ +x[0-9]+:` (x0–x30 plus esr_el1/elr_el1/far_el1/interrupt_type fields) | **PASS** (31 rows) | virtpanic-boot.log |
| 4 | Exception routed through the REAL sync trap path: `esr_el1` shows `ec: Ok(Brk)`, `interrupt_type: 4` (SYNC_INVALID_EL1h) | **PASS** | virtpanic-boot.log |
| 5 | Deterministic clean-halt marker `kernel halted` printed after the dump; kernel then parks in `wfi` (no silent wedge) | **PASS** (1) | virtpanic-boot.log |
| 6 | Determinism: zero IRQ interleaving — 0 `tick` lines, 0 echo (IRQs never unmask in this config) | **PASS** (0 ticks) | virtpanic-boot.log |
| 7 | Harness verdict | **PASS** — `boot-test: PASS -- banner, virtpanic start marker, >= 30 register rows and the clean-halt marker observed in order` (61369 bytes captured) | harness stdout |

## Log provenance

- Canonical copies (this directory, committed): `virt-boot.log`, `virtpanic-boot.log` — byte-identical copies of the harness outputs.
- r9-side originals (regenerated on every harness run, gitignored `target/`):
  `../r9/target/boot-test-virt.log`, `../r9/target/boot-test-virtpanic.log`.

## Notes

- The panic dump prints `Unhandled interrupt` between the TrapFrame and the
  halt marker: that is the generic sync-exception label on the shared
  dump-and-halt path (03-06 changed only the marker + wfi parking, per plan).
- The register dump uses bare x0–x30 field names (03-06) so dump rows are
  machine-countable; the x29/x30 role names (frame_pointer / link_register)
  remain documented on the struct fields in `aarch64/src/trap.rs`.
- Host variance: this bundle is the darwin-aarch64 reference run.  A linux
  re-run is D-10's domain (03-01's queued linux leg) and Phase 5's INT-02/03.

## Human review

This bundle is the phase-gate artifact (D-11/D-13 — the Phase 1/2
captured-evidence + human-approval standard).  The review checklist is the
`<human-check>` embedded in 03-06-PLAN.md Task 2; approval is harvested at
the end-of-phase gate per the project's `human_verify_mode = end-of-phase`.
