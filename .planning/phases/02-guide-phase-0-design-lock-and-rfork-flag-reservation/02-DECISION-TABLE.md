# Phase 2: Thirteen-Decision Lock Table — Proposal for Owner Confirmation

This is the proposal for the owner's single confirmation (CONTEXT D-03): all thirteen reconciled decisions of the implementation guide (IG-D01 to IG-D13) with a proposed status each, for bulk confirmation with individual rulings only on exceptions.

| ID | Guide row | Decision | Proposed status | Basis |
|----|-----------|----------|-----------------|-------|
| IG-D01 | row 27, Build Priority | Core `ds/` data structures and `9P2000.aead` wire handling are built upfront (guide Phases 1 and 2), because Plan 9 is fundamentally a network file fabric. | Locked as written | D-04 |
| IG-D02 | row 28, Executable Binary Format | Plan 9 `a.out` is the sole binary format the kernel loads; linker-emitted ELF is converted by in-tree tooling, and contrib and 9front utilities run unmodified. | Locked as written | D-04 |
| IG-D03 | row 29, System Call Interface | The system call list is strictly closed: no POSIX emulation calls or `mmap` extensions, software adapts to `rfork`, `bind`, `mount` and 9P, and the crypto extensions add four `rfork` flag bits (`RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21) and zero syscalls, not the guide's three. | Locked, wording corrected: four `rfork` flag bits (`RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21), not the guide's three | D-05, D-01 |
| IG-D04 | row 30, Native Rust Runtime | A full Rust `std` (`sys/src/libstd_r9/`) wraps Plan 9 syscalls and is sequenced as Milestone STD, after Milestone S and before Track P, removing the `#![no_std]` requirement for Harvey userland and native applications. | Locked as written | D-04 |
| IG-D05 | row 31, C Toolchain and Compilers | Plan 9 C compilers (`6c`, `6a`, `6l`) are ported to Rust in Track P Stage P3, with Harvey's C compilers serving as bootstrap tools until parity is proven. | Locked as written | D-04 |
| IG-D06 | row 32, Dual C Runtime Model | Classic APE (via `6c`) and vendored APEX (via Clang) are segregated for Plan 9 contrib software and LLVM/C++ tooling respectively, and the two runtimes never mix in one binary. | Locked as written | D-04 |
| IG-D07 | row 33, Kernel Safety Boundary | Low-level hardware operations (`WRPKRU`, `MOVNTDQ`, `CLFLUSHOPT`, inline assembly, raw atomics) are restricted to `x86_64/`, `aarch64/` and `ds/src/tpht/` behind audited `// SAFETY:` blocks, and `port/` is the safe crate. | Locked amended to HEAD: `port/` uses per-module `forbid(unsafe_code)`; three named modules hold `unsafe` (`globalalloc`, `mcslock`, `devip_waker`); no new such module without owner approval. Not part of the lock: the guide's crate-level forbid wording; how lint and audit scope grow (note N2, Phases 3 and 5); the `ds/src/tpht/` wording (W5). HEAD detail: `port/src/lib.rs` calls `globalalloc` and `mcslock` floor modules and `devip_waker` a named bridge. | D-06 |
| IG-D08 | row 34, Memory Isolation Model | Sibling processes run under independent CR3 page-table roots plus intra-process PKEYs, avoiding PKEY capacity exhaustion while enforcing read-only base permissions in about 20 cycles. | Locked as written | D-04 |
| IG-D09 | row 35, Primary Concurrent Hashing | Succinct TPHT (flattened and chained, `ds/src/tpht/`) replaces Funnel Hashing in the page cache, 9P `FidTable` and the Venti index, with Funnel restricted to write-once ephemeral scratchpads. | Not locked; deferred to Phase 3 (W5, W6) | D-07 |
| IG-D10 | row 36, Integer Set Algebra | Hybrid Roaring bitmaps (`ds/src/roaring.rs`) provide the integer set algebra for frame tracking, superpage residency, 9P priority scheduling, deadline wheels, union layer filters and an inline `ClientSet`. | Locked as written | D-04 |
| IG-D11 | row 37, Distributed Wire Security | Native `9P2000.aead` is the distributed wire security: an AAD envelope with `ExtMask`, monotonic sequence nonces, hybrid `FidHandle`, four new message pairs (`Tcompound`, `Tstream`, `Tlease_revoke`, `Treclaim`) and pinned bounce buffers for cleartext fallbacks. | Not locked; deferred to Phase 4 (W1-W4) | D-07 |
| IG-D12 | row 38, 3D Graphics Architecture | A Plan 9 `#G` synthetic device carries the VirtIO-GPU Venus transport over the `/dev/gpu/` nodes `clone`, `ctl`, `cmd`, `blob` and `fence`, with zero-copy window presentation via `draw(3)` host texture blits. | Locked as written | D-04 |
| IG-D13 | row 39, Cluster Service Discovery | Multicast beacons plus staged `/srv` registration discover services from `class:instance[+attrs]` descriptors over dual-stack IPv4/IPv6, with a two-tier 30s `unhealthy` and 5m `dead` lifecycle. | Not locked; deferred to Phase 6 (W7) | D-07 |

## What "locked" means (D-08)

A lock is owner-only change. Planning and execution treat a locked decision as fixed. If a later phase finds one wrong, work stops and the owner is asked. A change is recorded as a new dated row that supersedes the old one, never a silent edit.

## Question for the owner

Does "locked as written" lock the decision and not the guide's landing-site file paths, which are re-verified when each phase is planned? (yes/no)

## Owner ruling

PENDING
