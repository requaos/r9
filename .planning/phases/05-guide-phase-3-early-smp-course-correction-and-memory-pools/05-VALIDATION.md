---
phase: "05"
slug: "guide-phase-3-early-smp-course-correction-and-memory-pools"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-25"
---

# Phase 05 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in unit/integration test harness via `cargo xtask` + Miri (`cargo xtask miri`) + QEMU expect gates (`gates/aarch64/`, `gates/x86_64/`) |
| **Config file** | Workspace `Cargo.toml`, `.planning/config.json`, `xtask/src/test9p.rs`, `xtask/src/lintcontract.rs` |
| **Quick run command** | `nix develop -c cargo test -p <crate> <filter>` (or `nix develop -c cargo xtask test-9p` for 9P wire/slab pins) |
| **Full suite command** | `nix develop -c cargo xtask check && nix develop -c cargo xtask clippy --arch aarch64 --config virtshell && nix develop -c cargo xtask clippy --arch x86-64 --config q35cpu && nix develop -c cargo xtask test` |
| **Estimated runtime** | ~30 seconds (host unit/check/clippy); ~3–5 minutes (QEMU TCG boot gates) |

---

## Sampling Rate

- **After every task commit:** Run targeted crate/module test (`nix develop -c cargo test -p <crate> <mod>`) + `nix develop -c cargo xtask check`
- **After every plan wave:** Run full host suite (`nix develop -c cargo xtask test`, both arch clippy legs) + `nix develop -c cargo xtask miri` when `ds/src/tpht/` or `ds/src/slab.rs` changes
- **At each owner stop (4 stops, D-32):** Cumulative `port/src` diff review; aarch64 quick pass on Mac (HVF+TCG); x86_64 CI (stops 1–4) + owner's Linux KVM host (stops 2, 3, 4)
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds (host unit/lint checks)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | TBD | 1 | CORR-01 | — | N/A | CI / expect gate | `nix run nixpkgs#gh -- run view` + `nix develop -c cargo xtask gate --arch x86-64 --accel tcg` | ✅ | ⬜ pending |
| 05-01-02 | TBD | 1 | CORR-02 | — | N/A | nix eval / CI build | `grep -c outputHash flake.nix` + CI `nix build --no-link .#boot-iso` | ✅ | ⬜ pending |
| 05-01-03 | TBD | 1 | CORR-03 | T-05-01 / T-05-02 | Non-zero gen, owning `client_id` + `epoch` check, payload scrub on clunk, race-free Boehm seqlock | unit + integration + Miri | `cargo test -p ds slab && cargo xtask test-9p && cargo xtask miri` | ❌ W0 | ⬜ pending |
| 05-01-04 | TBD | 1 | CORR-04 | — | `RFNOLEGACY = 1<<18`, legacy crypto bits removed | unit | `cargo xtask rfork-header --check && cargo test -p xtask rforkh` | ✅ | ⬜ pending |
| 05-01-05 | TBD | 1 | CORR-05 | — | Zero undocumented `unsafe` on `x86_64` & `aarch64`; policy & `UNSAFE-AUDIT.md` synced | lint + unit | `cargo xtask clippy --arch aarch64 --config virtshell && cargo xtask clippy --arch x86-64 --config q35cpu && cargo test -p xtask lintcontract` | ✅ | ⬜ pending |
| 05-01-06 | TBD | 1 | DEBT-01 | — | N/A | unit + CI | `cargo xtask test` | ✅ | ⬜ pending |
| 05-01-07 | TBD | 1 | DEBT-02 | — | N/A | doc check | `grep -E "^(status: validated\|nyquist_compliant: true)" .planning/milestones/v3.0-phases/04-*/04-VALIDATION.md` | ✅ | ⬜ pending |
| 05-01-08 | TBD | 1 | DEBT-03 | — | N/A | doc check | `grep` check on `KERNEL-BYTES-GAPS.md` & `04-25-SUMMARY.md` | ✅ | ⬜ pending |
| 05-01-09 | TBD | 1 | DEBT-04 | — | N/A | unit | `cargo test -p ds tpht::flattened::tests::default_banks` | ❌ W0 | ⬜ pending |
| 05-01-10 | TBD | 1 | DEBT-05 | — | `docs/threat-model.md` matches v3.0 rescope & `rfork` bit 18 | unit | `cargo test -p xtask --test threat_model` | ✅ | ⬜ pending |
| 05-02-01 | TBD | 2 | SMP-01 | — | Per-CPU scheduling on x86_64 q35 under `-smp 2` | boot gate | CI TCG + KVM `cargo xtask gate --arch x86-64` (`-smp 2`) | ✅ | ⬜ pending |
| 05-02-02 | TBD | 2 | SMP-02 | — | Per-CPU state (`TPIDR_EL1` / `GS_BASE` `Mach.me`) + `CpuFp` | unit + boot gate | `cargo test -p port smp` + `-smp 2` boot gates | ❌ W0 | ⬜ pending |
| 05-02-03 | TBD | 2 | SMP-03 | — | Fine-grained locks, lost-wakeup-free `wake_pending`, no BKL | unit + stress gate | `cargo test -p port mcslock` + 2-CPU stress gate | ✅ | ⬜ pending |
| 05-02-04 | TBD | 2 | SMP-04 | — | aarch64 virt boots `-smp 2` under HVF (GICv3) and TCG (GICv2) | boot gate | `nix run .#boot-r9-smoke` (HVF & TCG, `-smp 2`) | ✅ | ⬜ pending |
| 05-02-05 | TBD | 2 | SMP-07 | T-05-04 | `IrqSpinLock` + cross-CPU TLB shootdown completion barrier before frame reuse | unit + boot gate | `cargo test -p port irq_spinlock` + boot gate shootdown marker | ❌ W0 | ⬜ pending |
| 05-02-06 | TBD | 2 | SMP-08 | — | 2-CPU user 9P (`FidTable`) + exec churn (`palloc.hash`) stress gate, 0 deadlocks | boot gate (both arches) | `nix run .#boot-r9-smp-stress` (aarch64) + x86_64 CI/KVM stress gate | ❌ W0 | ⬜ pending |
| 05-02-07 | TBD | 2 | SMP-09 | — | All existing gates at `-smp 2` + 1 `-smp 1` smoke leg per arch | boot gate | `grep -rn "\-smp " gates/` + CI green | ✅ | ⬜ pending |
| 05-02-08 | TBD | 2 | SMP-11 | — | Per-CPU run queues + work stealing in `port/` | unit | `cargo test -p port sched` | ❌ W0 | ⬜ pending |
| 05-02-09 | TBD | 2 | SMP-12 | — | Lock rank assertions (panic on inversion) + watchdog (names stuck CPU & lock); absent in release | unit | `cargo test -p port lock_rank && cargo test -p port watchdog` | ❌ W0 | ⬜ pending |
| 05-02-10 | TBD | 2 | SMP-13 | T-05-07 | Boot-switch `aeadloop=1` loopback `9P2000.aead` server; default boot exposes nothing | unit + boot gate | `cargo test -p port aead_loop` + default boot negative check | ❌ W0 | ⬜ pending |
| 05-03-01 | TBD | 3 | MEM-07 | T-05-03 | Roaring clean/dirty/pinned pools, `Evictable = Dirty \ (Pinned \| Active)`, High-Water Pager ~15% clean | unit | `cargo test -p port palloc` | ❌ W0 | ⬜ pending |
| 05-03-02 | TBD | 3 | MEM-08 | — | `SuperpageResidency` == 64 bytes, bit clear, atomic reset | unit | `cargo test -p port superpage` | ❌ W0 | ⬜ pending |
| 05-03-03 | TBD | 3 | MEM-10 | T-05-03 | `UserSlice`/`UserSliceMut` RAII `PinGuard` refcount; pinned frame never evicted | unit + pressure gate | `cargo test -p port pin_guard` + small-mem pressure gate | ❌ W0 | ⬜ pending |
| 05-03-04 | TBD | 3 | MEM-14 | T-05-06 | x86_64 demand loading (`#PF` pager); rio green | boot gate | CI TCG + KVM `cargo xtask gate --arch x86-64` | ❌ W0 | ⬜ pending |
| 05-03-05 | TBD | 3 | MEM-15 | T-05-04 / T-05-06 | RO user text, dirty tracking both arches, live evictor, small-mem `-smp 2` pressure gate | boot gate (both arches) | `nix run .#boot-r9-mem-pressure` (aarch64) + x86_64 CI/KVM pressure gate | ❌ W0 | ⬜ pending |
| 05-03-06 | TBD | 3 | MEM-16 | T-05-04 | Flattened-TPHT `palloc.hash` image page cache + shared text reverse-map unmap | unit + boot gate | `cargo test -p port pagecache` + pressure/stress gates | ❌ W0 | ⬜ pending |
| 05-03-07 | TBD | 3 | MEM-17 | T-05-05 | Roaring AVX2 & NEON kernels identical to scalar behind `setops.rs` + `fpu::with_simd()` | unit + boot selftest | `cargo test -p ds setops` + aarch64 HVF/TCG + x86_64 AVX2 TCG boot selftest | ❌ W0 | ⬜ pending |
| 05-03-08 | TBD | 3 | MEM-18 | T-05-05 | x86_64 AVX/XSAVE where CPUID reports it, FXSAVE elsewhere, boot line names format | boot gate (both CPUs) | CI TCG `qemu64` (`format=fxsave`) + CI TCG `qemu64,+avx,+avx2,+xsave,+xsaveopt` (`format=xsaveopt`) + KVM `fpprobe` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `xtask/src/gate.rs:885,918` — fix archived Phase 1 witness path so `cargo test -p xtask` starts green
- [ ] `ds/src/slab.rs` — generic `#![forbid(unsafe_code)]` seqlock slab + unit/concurrency tests
- [ ] `ds/src/setops_simd.rs` — reserved audited unsafe AVX2/NEON Roaring kernels + `lintcontract.rs` R3 update
- [ ] `port/src/smp.rs`, `port/src/sched.rs` — `PerCpu` trait, `IrqSpinLock` rank checker, watchdog, shootdown coordinator, work-stealing run queues + host tests
- [ ] `port/src/palloc.rs`, `port/src/superpage.rs`, `port/src/pagecache.rs`, `port/src/aead_loop.rs` — pools, `SuperpageResidency`, TPHT page cache, loopback 9P2000.aead server + host tests
- [ ] `gates/aarch64/` & `gates/x86_64/` — new `-smp 2` stress gate (`SMP-08`) and small-memory pressure gate (`MEM-15`) expect scripts + CI AVX2 TCG job

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Owner x86_64 Linux KVM multi-core validation at Stops 2–4 | SMP-01, SMP-08, MEM-14, MEM-15, MEM-18 | Owner's bare-metal Linux KVM host is external to local macOS host and GitHub-hosted TCG CI runners | Run x86_64 `-smp 2` gates, stress gate, memory-pressure gate, and KVM `fpprobe` on the owner's Linux KVM host at Owner Stops 2, 3, and 4 (D-32) |
| Owner cumulative `port/src` diff review at Stops 1–4 | Phase 5 D-32 | Owner review gate replacing per-commit diff-first rule | Present cumulative `port/src` diff and gate evidence at Stops 1, 2, 3, and 4 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
