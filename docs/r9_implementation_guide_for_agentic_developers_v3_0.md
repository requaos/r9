# r9 Implementation Guide for Agentic Developers (v3.0)

**Baseline Commit:** `requaos/r9` @ `900f58b` | `requaos/harvey` @ `c697666`  
**Supersedes:** r9 Implementation Guide (Amended Specification v2.1)  
**Date:** November 25, 2026  
**Audience:** Autonomous / agentic engineering workflows executing against `main`

---

## 1. Immediate Course-Correction Tasks (Before Starting Phase 3)

Execute these five surgical tasks first to align HEAD (`900f58b`) with v3.0 and turn `main` green:

1. **Fix Red CI on `900f58b`:** Inspect and repair the failing workflow from `chore: complete the v3.0 audit move into milestones/` so `main` is green before any refactor lands.
2. **Replace `boot-iso` Fixed-Output Derivation Hash:** Remove the brittle manual `outputHash` maintenance on `boot-iso` in `flake.nix` (seen in commit `ca2b5ab`) by converting it to a standard pure derivation built from workspace inputs rather than a FOD.
3. **Refactor `port/src/fid.rs` to Sharded Generational Slab:**
   * Implement `ds/src/slab.rs` (`#![forbid(unsafe_code)]`) or embed the `FidShard` slab directly in `port/src/fid.rs` per *Compendium v3.0 §4*.
   * Remove `FlattenedTphtShard` usage from `FidTable` and revert the serialized-insert lock workaround from `6fd201c`.
   * Keep `ds/src/tpht/` intact for Phase 4 (`UnionMountTable` dcache), Page Cache (`palloc.hash`), and Phase 6 (Venti).
   * Verify all `cargo xtask test-9p` legs remain green.
4. **Retire Cryptographic Memory Artifacts & `rfork` Bits:**
   * In `port/src/rfork.rs` and `xtask` (`rfork-header`), mark bits 18–20 (`RFCKEY`, `RFROBASE`, `RFRANDMEM`) as `RESERVED_UNUSED` (or remove them if no external binary depends on them yet) and reserve a single bit for `RFNOLEGACY`. Regenerate `sys/include/rfork.h`.
   * Remove any planned or stubbed `cryptpage.rs`, `segcrypt.rs`, `shadow.rs`, or `ymm` assembly tasks from `.planning/`.
5. **Sync Unsafe Policy Docs & Lint Contracts:**
   * Update `docs/` and `UNSAFE-AUDIT.md` to explicitly codify the per-module `#![forbid(unsafe_code)]` rule in `ds/` (matching `25af196`) and crate-wide `#![forbid(unsafe_code)]` in `crypto/`.
   * *Corrected 2026-11-25 (owner, Phase 5 D-02):* this task's original wording also named `port/` as crate-wide; `port/` is and stays per-module forbid with the three named floor modules `globalalloc`, `mcslock` and `devip_waker` (no fourth without owner approval). The identical policy sentence now lives in this guide's section 3, the compendium section 2, the workspace `Cargo.toml` header comment and `UNSAFE-AUDIT.md`.

---

## 2. Realigned Multi-Track Engineering Roadmap (v3.0)

| Phase | Deliverable Scope | Dependencies | Verification Exit Gate |
| :--- | :--- | :--- | :--- |
| **Phase 0–2 (Done + Delta)** | `crypto/`, `ds/` (TPHT + Roaring + Slab), Core `9P2000.aead` wire/session, **Slab `FidTable` refactor**, CI & Nix cleanup | HEAD `900f58b` | `main` CI green; `cargo xtask test` & `test-9p` green; zero serialized TPHT workarounds in `FidTable`. |
| **Phase 3 (NEW: Early SMP)** | **Minimal 2-CPU SMP Bring-Up:** `x86_64` SIPI/LAPIC (`q35 -smp 2`), `aarch64` PSCI/GIC (`virt -smp 2`), per-CPU `Mach`, `IrqSpinLock`, TLB shootdown IPI, retire single-CPU spinlock assumptions | Phase 2 Delta | Both CPUs reach scheduler loop on `q35` & `virt`; 2-CPU concurrent `FidTable` + `FlattenedTphtShard` + timer-IRQ stress test passes with zero deadlocks. |
| **Phase 4** | **Distributed 9P Extensions:** `Tcompound`, `Tstream` + Roaring P0–P2 deadline wheel, `Qid` volatility + `LeaseRecord`/`ClientSet`, `UnionMountTable` (Roaring + TPHT dcache), legacy 9front/Linux bounce gateway, `RFNOLEGACY` | Phase 3 (SMP) | Atomic compound scripts pass; multi-client lease revocation green under 2-CPU SMP; 9front interop verified. |
| **Phase 5** | **Process Lifecycle, Stack-Safe Notes & `riscv64` Sync:** Checked `SegKind::Stack` note delivery (`notes.rs`), `RFNOLEGACY` enforcement, bring `riscv64` up to parity with `port::rfork` & SMP traits | Phase 4 | Stack spoofing traps pass; `riscv64` builds and passes boot smoke. |
| **Track P (Scoped)** | **Pure-Rust Harvey Port (No C Compilers):** Kernel devices, core libraries, and essential userland commands. **Explicitly excludes porting `6c`/`6a`/`6l` and Classic APE.** | Concurrent (3–5) | System boots Rust kernel + ported userland with Harvey C tools used strictly as bootstrap artifacts. |
| **Phase 6** | **Storage Engine:** BLAKE3 Venti CAS (Chained-TPHT score index, full 32B verify), Fossil GFS snapshotting (Roaring mark-and-sweep GC), NVMe TRIM | Phases 3, 4, Track P | Venti dedup & collision-resistance pins green; Fossil snapshot GC verified under SMP. |
| **Phase 7** | **Graphics:** 2D framebuffer, `devdraw`, `#G` VirtIO-GPU Venus (bits 0, 3, 4 + Capset 4), `draw(3)` host-blit bridge | Track P (`draw`/`rio`) | Hardware triangle rendered in `rio` window on QEMU. |
| **Phase 8** | **Self-Hosting Rust Toolchain:** `rustc` + `rust-lld` + vendored APEX + `elf2aout` (zero `mmap`) | Phase 6, Track P | `r9` compiles its own kernel and core userland natively on `r9`. |

---

## 3. Agentic Developer Operating Rules & Verification Protocol

Every PR or milestone step executed by an agentic developer **must** adhere to the following repository discipline:

1. **RED $\to$ Implement $\to$ 2$\times$ GREEN:** Commit a failing test pin first, implement the change, verify green twice consecutively, then advance the milestone pin.
2. **Mandatory Local Verification Commands:**
   * `cargo xtask check` — enforces workspace lint contracts: crate-wide `#![forbid(unsafe_code)]` on `crypto/` (over r9's own code, not the pinned crates it wraps); per-module `#![forbid(unsafe_code)]` on `ds/` (exactly `ds/src/tpht/` and one reserved Roaring SIMD module may hold `unsafe`; a third needs owner approval) and on `port/` (the three named floor modules `globalalloc`, `mcslock`, `devip_waker`; no fourth without owner approval); `clippy::undocumented_unsafe_blocks = "deny"` on `ds/src/tpht/`, `aarch64` and `x86_64`. *(Corrected 2026-11-25 per owner D-02: `port/` is per-module, not crate-wide.)*
   * `cargo xtask test` — host unit pins for `crypto/`, `ds/` (TPHT, Roaring, Slab), and `port/`.
   * `cargo xtask test-9p` — per-file `9P2000.aead` wire, `ExtMask`, `StatelessCap`, slab `FidShard`, and failover sweep legs.
   * `cargo xtask boot-test` & `cargo xtask ureg-check` — requires `R9_HARVEY_DIR` pointing to a valid Harvey checkout (no implicit `../harvey` fallback).
   * **Phase 3+ SMP Gate:** All new kernel boot gates must run with `-smp 2` on QEMU `q35` (`x86_64`) and `virt` (`aarch64`).
3. **`UNSAFE-AUDIT.md` Lockstep:** Any commit adding, modifying, or removing an `unsafe` block in `ds/src/tpht/`, `x86_64/`, `aarch64/`, or `riscv64/` must update `UNSAFE-AUDIT.md` in the same commit with a `// SAFETY:` invariant justification.
4. **No Silent Scope Creep:** Agents must not reintroduce software RAM encryption, `PKEY`/`WRPKRU` page-fault hooks, or Rust rewrites of `6c`/`6a`/`6l`. Any proposed deviation requires an explicit `.planning/` ADR update.