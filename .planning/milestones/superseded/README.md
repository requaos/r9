# Superseded planning

Planning artifacts that were written and then withdrawn before any plan was executed. They are a record only; no planning tool should read them as live state.

## 05-guide-phase-3-cryptographic-memory-and-zero-copy-dma-paging

Superseded on 2026-11-25, the day it was written. It holds the discussion (`05-CONTEXT.md`, decisions D-01 to D-38), research, pattern map and validation strategy for the old Phase 5, "Cryptographic Memory and Zero-Copy DMA Paging". The owner's v3.0 documents dropped software cryptographic memory:

- `docs/plan_r9_architecture_code_compendium_v3_0.md`
- `docs/r9_implementation_guide_for_agentic_developers_v3_0.md`

No plan was written and no code was changed under it. Findings from its research that are still true of the tree and are carried into the new Phase 5 entry in `ROADMAP.md`:

- `x86_64/` has 32 undocumented `unsafe` blocks in 9 files; the 7 aarch64 sites named in `STATE.md` are confirmed.
- The x86_64 "out of scope" table in `UNSAFE-AUDIT.md` names files that no longer exist (`node0.rs`, `vsvm.rs`).
- `xtask/src/test9p.rs` pins the test-leg count in three assertions and a literal list; adding a leg or folding `test-9p` into `cargo xtask test` must update them.
- TCG's default `qemu64` CPU has no XSAVE or AVX.
- Neither architecture tracks dirty pages; both map user text writable; the page-in paths panic on allocation failure; `UserSlice` holds no reference that could release a pin.
