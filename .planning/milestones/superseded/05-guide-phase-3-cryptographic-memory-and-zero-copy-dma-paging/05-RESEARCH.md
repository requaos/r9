# Phase 5: Guide Phase 3 - Cryptographic Memory and Zero-Copy DMA Paging - Research

**Researched:** 2026-11-25
**Domain:** kernel memory management (sealed demand paging, page allocator), x86_64 and aarch64 FP/SIMD state, ChaCha20 and keyed BLAKE3 in kernel context, `9P2000.aead` ingress, lint and CI follow-ups
**Confidence:** MEDIUM overall. HIGH for what the tree holds today and for the assembler, `fearless_simd`, CPU-model and XSAVE probes (all run this session). LOW for anything that needs KVM or an x86_64 boot, neither of which this host can run.

**How claims were checked.** `[VERIFIED: path:lines]` means the lines were displayed this session (Read tool or a line-range print) and the value is quoted beside the tag. `[VERIFIED: probe]` means a throwaway program was built and run this session; the probes lived in the session scratch directory and were deleted afterwards. `[ASSUMED]` means training knowledge or reasoning, not checked here. No web source was used.

**Host limits that shaped this research.** The research host is an arm64 Mac (Apple M5 Max, macOS 27.0). It has no KVM. x86_64 boot tests do not run on it at all: `cargo xtask boot-test --arch x86-64` builds the kernel and then stops with `spawn grub-mkrescue: No such file or directory`, and `nix shell nixpkgs#grub2` is refused as an unsupported system on darwin `[VERIFIED: probe]`. The `x86_64` crate's host tests are also skipped here, because `cargo xtask test` picks the arch-crate leg by host architecture. Every x86_64 boot gate and every x86_64 host test in this phase therefore needs a Linux x86_64 venue (CI, an amd64 container, or the owner's host).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried in (not re-asked)
- **D-01:** Keyed BLAKE3 is the only MAC, including page tags; ChaCha20 uses the RFC 8439 layout, 32-bit block counter and 96-bit nonce (Phase 3 D-05, D-06). One key and nonce cover at most 2^32 - 1 blocks (PROJECT.md Key Decisions).
- **D-02:** `KeyCapability` and the key hierarchy live in `crypto/`; `port/src/segcrypt.rs` imports them (Phase 3 D-22). The `ReadOnly` capability is a type-level guarantee, not a cryptographic one.
- **D-03:** Data messages carry a parameter block, then file data padded to whole 4096-byte pages with the true count in the parameters; data page `i` encrypts from block counter `1 + 64 * i` (Phase 4 D-09). The packet MAC is verified at receipt under connection keys before any field is acted on (Phase 4 D-26).
- **D-04:** Every requirement follows IG-V01: committed RED pin, implementation, GREEN twice, pin advance. The golden invariant (boots to rio on q35) holds at every commit.
- **D-05:** Owner standing rules apply (no push without approval, no AI attribution, Harvey read-only), except the `port/src` diff-first rule, which D-28 changes for this phase. The compendium's appendix listings (9.2, 9.5) are non-normative sketches and are not copied as-is.
- **D-06:** `ds/` may hold `unsafe` in `ds/src/tpht/` and one reserved Roaring SIMD module; a third place needs owner approval (Phase 3 D-09). A new unsafe-holding module in `port/` needs owner approval (IG-D07).

#### Backing store, sealed set and scrubbing (N9 settled)
- **D-07:** An evicted page's sealed ciphertext lives in kernel-owned RAM frames behind a `BackingStore` seam. Eviction frees the plaintext shadow frame. A dirty anonymous page therefore returns no net memory in this phase; that is accepted. An out-of-RAM store plugs into the seam later. — **Reversibility:** reversible — the seam exists so that a swap-file or Venti store can replace the RAM store.
- **D-08:** Sealed demand paging is off in every default boot for the whole phase. It runs only in the new profiles (`q35lowmem`, `q35crypt`, and their aarch64 counterparts), selected by a boot switch. Turning it on by default is not part of this phase.
- **D-09:** In the sealed-paging profiles every user page is sealed. Text and data are sealed when the image is installed, text once per image; bss, stack and heap are sealed at eviction. The plaintext boot image (paqfs in RAM) is outside the claim.
- **D-10:** A key domain (KeyID) is one process. Private pages (data, bss, stack, heap) are sealed under a per-process key derived from a boot secret. Shared text is sealed once under a per-image key that every process running the image holds as `ReadOnly`. Phase 7 adds only the `RFCKEY` sibling split. — **Reversibility:** costly — the page-record key references, the recycle rule and Phase 7's `RFCKEY` are built on this domain shape.
- **D-11 (N9 settled):** A frame reused by a different key domain is zeroed and then flushed (`CLFLUSHOPT`, gated on CPUID leaf 7 EBX bit 23). Only same-KeyID reuse (`recycle_within`) skips both. This is the RFC section 11 rule. The guide section 4.4 "no scrubbing" wording and the compendium section 3.4 "scrubbed with `CLFLUSHOPT`" wording are recorded as corrected, and MEM-01's "popped without scrubbing" and MEM-06 are amended to match. The owner chose this over the guide's rule, which was the recommended option. — **Reversibility:** reversible — relaxing to the guide rule later is a local allocator change.

#### Vector path and the FP rule (N7 settled)
- **D-12 (N7 settled):** The fault decrypt path gets its registers from a second, narrower section kind inside the `fpu` module. It spills, scrubs and restores only the six vector registers and hands out a guard token, like `KernelSimd`. The existing rules hold: interrupts disabled, no nesting, no sleeping inside. The owner's rule that kernel SIMD goes through `fpu` is kept and extended, not bypassed; `cryptpage.rs` does not spill registers itself. The cost target is the guide's: about 30 cycles for spill, scrub and restore. No `XSAVE`/`XRSTOR` runs on the fault path. The pager's seal path on eviction uses the same restricted section and routine, not a full `with_simd()` section; the full section is used only by the Roaring SIMD kernels (owner, 2026-11-25, during milestone requirements).
- **D-13:** AVX is enabled on x86_64: `CR4.OSXSAVE` set and `XCR0 = x87 | SSE | AVX`, with AVX-512 state left off (MEM-04). Per-process FP state moves from the 512-byte FXSAVE image to XSAVE, so user `ymm` state survives a switch, `rfork`, exec reset and note delivery. This applies to every x86_64 boot, not only the sealed profiles, and user programs gain AVX. XSAVE replaces FXSAVE only where FXSAVE runs today (process switch, `rfork`, exec, notes); it is never on the fault path (D-12). `XSAVEOPT` is used where the CPU has it, so a process that never touches AVX pays close to today's FXSAVE cost; research measures this under KVM and TCG (owner, 2026-11-25). — **Reversibility:** costly — every FP save site (switch, fork, exec, notes) and the user-visible CPU feature set change.
- **D-14:** CPUID chooses the scalar or the vector path at boot. TCG's default `qemu64` runs scalar. One extra TCG run with an AVX2-capable CPU model covers the vector path and the register-preservation probe without KVM. Scalar and vector outputs are pinned identical (MEM-04).
- **D-15 (MEM-02 and MEM-03 reconciled):** The fault path decrypts into the still-unmapped frame, issues `SFENCE`, computes the MAC over the ciphertext while the write-combining buffers drain, and publishes the frame only if the MAC passes. On failure the frame is zeroed and the process is killed. MEM-02's "verified in cache before decrypting" (corrections ledger row 5) is amended to "verified before the frame is published"; the rest of that row stands: Encrypt-then-MAC, the MAC is over ciphertext, dedicated MAC keys, no plaintext read back from DRAM. The owner chose this over verify-first, which was the recommended option. — **Reversibility:** costly — the vector routine's structure, the benchmark baseline and the amended ledger row all rest on this order.
- **D-16:** The Roaring SIMD kernels are built in this phase for both architectures, AVX2 and NEON, behind the `ds/src/setops.rs` seam, and are reached through `with_simd()`. A probe first tries `fearless_simd` on the soft-float kernel targets (builds `no_std`, a token can be minted from the `fpu` guard, supply-chain cost); if it fails, the kernels are written as inline asm in the reserved module, as `fpu.rs` already does for SSE. The owner chose "build now" over "measure first", which was the recommended option.

#### Ingress (N10, N11 settled)
- **D-17:** "DMA straight into the backing store with no kernel copy" means no copy above the transport. The 9P client reads each data page from the transport straight into its destination `UninitializedFrame`: no intermediate kernel buffer, no decrypt on receive, one frame per page. 9P runs over TCP through smoltcp, so copies inside the TCP stack remain; this is stated as the limit. MEM-09 and success criterion 2 are amended to this wording. True NIC placement waits for a transport that can do it.
- **D-18:** A received page stays in its frame as wire ciphertext. Its page record stores what decryption needs (key reference, sequence nonce, block counter). A retired receive cipher key stays alive, reference-counted, only while undecrypted pages point at it, and is never accepted for packets again; it is zeroized when the last such page is decrypted or freed. This amends Phase 4 D-20, which zeroized old receive keys on the re-key marker, and the teardown rule of Phase 4 D-15. The owner chose this over converting pages before the key dies, which was the recommended option. — **Reversibility:** costly — it changes the key-lifetime rule of the locked wire decision set (IG-D11 row); the change must be recorded as a new dated PROJECT.md row and RFC note.
- **D-19 (N11, second half settled):** At receipt, after the packet MAC passes, the kernel computes a keyed BLAKE3 tag for each data page under the owning domain's page MAC key (a hash pass over ciphertext; no decrypt, no copy). The fault path checks that tag exactly as it does for an evicted page. Two checks under two keys: the packet MAC at receipt, the page tag at fault.
- **D-20:** Success criterion 2 is proven by `cargo xtask test-9p` on the host, driving ingress over the in-memory link with the real frame and page-record code, plus a probe under `q35crypt` in which the booted kernel pairs the client engine with a test server end over a loopback link with an injected secret and a user process faults the page through the real pager. There is no network leg and no live sealed mount.
- **D-21:** Two kinds of read take the sealed ingress path: an image read at exec from a `9P2000.aead` mount (text and data pages land as sealed frames in the image's backing store and decrypt on first touch; nothing blocks at fault time), and a `read(2)` of whole pages into a page-aligned user buffer, which swaps sealed frames into the caller's address space instead of copying. Every other `read(2)` keeps the verify, decrypt and copy path. The owner chose to include page-aligned `read(2)`; exec-only was the recommended option.
- **D-22 (N10 settled for this phase):** `docs/threat-model.md` is amended in this phase: the plaintext of a sealed page exists only in its shadow frame; kernel I/O buffers (plain 9P reads, pipes) and the plaintext boot image are named as outside the claim. The legacy gateway's bounce frames get their own line when Phase 6 builds them.

#### Scope and review process (N2 settled)
- **D-23:** Both architectures get the sealed pager. The portable core (allocator pools, page records, stride tracker, sealing logic, in-flight table) goes in `port/` and is host-tested. x86_64 wires it to a new trap-context pager; aarch64 wires it to its existing fault path. aarch64 gets a scalar path and a NEON vector decrypt path under its own restricted section (D-12's shape), plus aarch64 counterparts of the two profiles. The owner chose this over x86_64-only, which was the recommended option. aarch64 kernel byte changes get the standing quick pass; the full HVF series stays at Phase 8.
- **D-24:** x86_64 has no trap-context pager at HEAD (text, data and bss are mapped eagerly at exec). Building one is part of this phase, not a precondition.
- **D-25 (N2 settled):** `x86_64` takes `[lints] workspace = true` as an early plan: every existing `unsafe` block gets a `// SAFETY:` comment and an `UNSAFE-AUDIT.md` entry, and the new modules then land under the lint. The 7 undocumented aarch64 blocks that fail `cargo xtask clippy --arch aarch64 --config virtshell` today are fixed in the same pass.
- **D-26:** The x86_64 byte-change procedure runs at two owner stops. The first follows the AVX/XSAVE change (D-13), which lands as its own small group of plans because it alters default-boot behaviour. The second is at phase end and covers everything else. Each stop is the full procedure: `boot-iso` `outputHash` re-derived and committed on the owner's x86_64 Linux host, the x64 TCG gates, and the owner's boot-iso check. Phase 4 D-25 and Phase 2 D-23 to D-29 are the model.
- **D-27:** The byte-change evidence follows Phase 4 D-29: `.text` identity is not the test, since the changes are intended.
- **D-28:** The `port/src` diff-first stop is waived for this phase, as in Phase 4 D-24. Plans commit once their tests are green; the owner reviews the cumulative `port/src` change at the phase-end stop, before any push. The standing rule is unchanged for other phases.
- **D-29 (owner, 2026-11-25, at the v3.1 roadmap):** The v3.0 audit follow-ups, DEBT-01 to DEBT-05, are part of this phase and are its initial steps: they are planned and executed before any pager, FP or allocator work. The owner chose this over a separate inserted phase, knowing the phase is already large. DEBT-02 and DEBT-03 edit files archived under `.planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/`.

#### Not decided in discussion
- **Phase size.** The decisions above make this phase much larger than its roadmap entry (two pagers, XSAVE, Roaring SIMD on two architectures, page-flipping `read(2)`). Whether to split it in the roadmap was not discussed. The planner should raise it with the owner if the plan count or wave structure calls for it; it must not split or drop scope by assumption.
- **Requirement and document amendments.** The exact amended wording of MEM-01, MEM-02, MEM-06, MEM-09, MEM-11, success criteria 2 and 3, the RFC (sections 9.1 and 11, and the re-key text touched by D-18), the corrections-ledger reading of row 5, and the PROJECT.md rows that close N2, N7, N9, N10 and N11 are to be written by the plan and shown to the owner. D-18 touches a locked decision set, so its PROJECT.md row is an owner checkpoint.
- **A new unsafe-holding module in `port/`.** If the portable core cannot be written without one, that needs owner approval (D-06); the default is that raw frame access stays in the arch crates.

### Claude's Discretion
- Names and exact settings of the aarch64 profiles, the low-memory size and trigger for `q35lowmem`, the tamper mechanism for `q35crypt`, and which CPU model the TCG vector run uses.
- Where the boot secret for D-10 comes from (the existing entropy source is the expected input) and the derivation labels, provided they are pinned by known-answer vectors.
- The page record layout, the tag construction beyond "keyed BLAKE3 over the page ciphertext" (what else it binds, such as the page's address or version, to prevent relocation and replay of a sealed page), and how nonces for re-sealed pages are kept unique.
- What is evictable beyond MEM-07's set expression, the High-Water Pager's scheduling, and the `StrideTracker` policy use.
- The XSAVE fallback where `XSAVEOPT` is absent, the save-area size handling, and how note delivery carries the larger state.
- Whether the vector routine is `global_asm!` in `cryptpage.rs` or a standalone `l.S` section if the soft-float assembler rejects AVX mnemonics (guide section 11 risk).
- The in-flight de-duplication structure for MEM-12: the write-once Funnel scratchpad of the guide, sized for a single CPU now; Funnel Hashing stays off every primary path (IG-D09).
- How pinning is expressed for MEM-10 and which existing call sites take pins.
- The page-aligned `read(2)` rules (alignment of offset and length, short reads, what happens to the frames previously mapped there) within D-21.
- The KVM benchmark's harness and report format for MEM-13; the figures are recorded, and "about 4.4 microseconds" is an expectation, not a gate.
- Module layout, the order of work and commits, and how `port`'s per-module `forbid(unsafe_code)` and `UNSAFE-AUDIT.md` are kept true.

### Deferred Ideas (OUT OF SCOPE)
- **Plan 9 swap file over 9P** as an out-of-RAM backing store: needs a pager that can block on RPCs; a later phase, or Venti in Phase 13, behind the D-07 seam.
- **Sealed paging on by default in every boot:** not before the profiles have run for a while; a candidate for Phase 7, when `RFCKEY` makes per-process keys user-visible.
- **True NIC placement of 9P data pages** (mergeable receive buffers, a TCP path without socket-buffer copies) and the `#M` ring bypass: Phase 6 (DIST-08).
- **A live sealed mount over virtio-net with a host-side `9P2000.aead` test server:** the phase that has a real key exchange, or Phase 6.
- **The legacy gateway's bounce frames in the threat model** (rest of N10): Phase 6.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description (short) | Research support |
|----|---------------------|------------------|
| MEM-01 | `UninitializedFrame` unmapped until verified; same-KeyID reuse not scrubbed | Patterns 2 and 3; Pitfalls 7, 8; x86_64 uses the identity alias and aarch64 the KZERO alias, so a frame can be filled without a user mapping |
| MEM-02 | Encrypt-then-MAC, dedicated MAC key, publish only after verify, kill on failure | Pattern 3; Open Question 2 (in-place decrypt conflicts with D-15 for ingress pages) |
| MEM-03 | Cached store for the faulting line, non-temporal for the rest, `SFENCE` overlapped with the MAC | Research question 1 (mnemonics assemble); Pattern 4; aarch64 has no `SFENCE` equivalent (Open Question 12) |
| MEM-04 | `ymm0`-`ymm5` routine under a restricted `fpu` section; AVX-512 off; XSAVE at switch only; NEON path | Research questions 1 to 3; Pattern 1; Pitfalls 1 to 4 |
| MEM-05 | `StrideTracker` chronological ring | Portable, host-tested; no blockers found |
| MEM-06 | `recycle_within`; cross-domain zero then `CLFLUSHOPT` gated on CPUID | TCG `qemu64` has no `CLFLUSHOPT` (probe); fallback needed (Open Question 11) |
| MEM-07 | Roaring pools, address expression, High-Water Pager, AVX2 and NEON set kernels | Research question 4; Pattern 5; lint rule R3 and R4 collisions (Pitfalls 5, 6) |
| MEM-08 | `SuperpageResidency` exactly 64 bytes with counter reset | Open Question 7 (512 bits already fill 64 bytes) |
| MEM-09 | Data page read from the transport into its frame, tagged at receipt, verified at fault | Research questions 5 and 7; Pattern 6; Open Questions 2 and 6 |
| MEM-10 | `UserSlice` residency plus pinning across sleeps | `usermem.rs` has 23 call sites in `x86_64/src/syscall.rs` and holds only a raw pointer today (Pitfall 9) |
| MEM-11 | Profiles `q35lowmem`, `q35crypt` and aarch64 counterparts; x86_64 lint opt-in; audit reconciled | Research question 8; Pattern 7; Pitfalls 10, 11, 12 |
| MEM-12 | In-flight fault de-duplication | Untriggerable in the booted kernel today (Open Question 9); host-tested only |
| MEM-13 | KVM benchmark, scalar versus vector, eviction cost | Needs the owner's x86_64 Linux host; nothing measurable here (Open Question 14) |
| DEBT-01 | `test-9p` and `cargo test -p xtask` in `cargo xtask test` and CI | "What DEBT-01 to DEBT-05 touch" below |
| DEBT-02 | `04-VALIDATION.md` from `draft` to a completed record | Same section |
| DEBT-03 | Boot-iso evidence text corrected | Same section |
| DEBT-04 | TPHT home-hit at production bank sizes; sizing rule written | Same section |
| DEBT-05 | Threat model states `rfork` bits 18-21 reserved | Same section; note the document has an `xtask` test |
</phase_requirements>

## Summary

The tree is ready for the portable half of this phase and not ready for several things the decisions assume. `crypto/` already has the scalar ChaCha20, keyed BLAKE3, labelled derivation and the `KeyCapability` typestate; `ds/` has the Roaring bitmap with `difference` and the four-function `setops` seam; `port/src/chan/transport.rs` has the session engine; both `fpu` modules have the same `kernel_begin` / `with_simd` shape. The landing sites (`palloc.rs`, `segcrypt.rs`, `inflight.rs`, `superpage.rs`, `cryptpage.rs`, `shadow.rs`) do not exist, x86_64 has no pager, and neither architecture has eviction, dirty tracking or a shared page object: each process owns a `mapping_log: Vec<MappingRecord>` that is scanned linearly, and x86_64 text is copied per process on `rfork`, not shared.

Eight findings change how the phase must be planned. (1) AVX2, XSAVE-family and `CLFLUSHOPT` mnemonics assemble in `global_asm!` and inline `asm!` on the soft-float x86_64 kernel target, and NEON assembles on the aarch64 kernel target, so no `l.S` fallback is needed (probe). (2) `fearless_simd` 1.3.0 builds `no_std` on both kernel targets only with its `libm` feature, emits no vector instruction at all on the x86_64 soft-float target, and on aarch64 trips the future-incompatibility lint "enabling the `neon` target feature on the current target is unsound due to ABI issues". It fails the D-16 probe; the Roaring kernels are inline asm. (3) TCG's `qemu64` has no XSAVE, no AVX and no `CLFLUSHOPT` (probe). Setting `CR4.OSXSAVE` there would fault, so the FXSAVE path must stay and the XSAVE path is chosen by CPUID at boot; D-13's "every x86_64 boot" cannot be literally true on the default TCG gates. (4) `-cpu qemu64,+avx,+avx2,+xsave,+xsaveopt` and `-cpu max` both work under TCG in the pinned QEMU 11.1.2; TCG has no `XSAVEC`, `XSAVES` or AVX-512. (5) The lint rules in `xtask/src/lintcontract.rs` forbid two things the decisions need: an arch crate naming `crypto` or `ds` (R4), and `unsafe` anywhere under `ds/src` outside `tpht/` (R3). The vector cipher needs raw key bytes that `crypto` does not expose, and the Roaring SIMD module needs a token that `ds` cannot import from an arch crate. (6) Every ring-3 test binary in the boot images comes from the read-only Harvey tree, so new user-mode probes need an r9-side source. (7) The x86_64 lint opt-in is small: 32 undocumented `unsafe` blocks in 9 files, not 200. (8) The Phase 4 kernel-bytes evidence has an "FP/SIMD scan equal to baseline" verdict that this phase breaks on purpose.

On size: a group-by-group count gives roughly 55 to 60 plans, about twice Phase 1 (34) and more than twice Phase 4 (25). The owner decided today to keep one phase. My reading is that it can be executed as one phase but should not be planned at full fidelity in one pass, because the later groups depend on measurements and owner answers from the earlier ones. See "Phase size" at the end.

**Primary recommendation:** Plan in the order DEBT, lint opt-in, AVX/XSAVE (owner stop 1), then the portable core with host tests, then the x86_64 pager and profiles, then aarch64, ingress and the Roaring kernels; keep both FP save formats and choose by CPUID; write all vector code as inline or global asm that takes an `fpu` guard; and put the eleven owner questions in "Open Questions" to the owner before the pager plans are written.

## Project Constraints (from CLAUDE.md)

No `CLAUDE.md` or `.claude/CLAUDE.md` exists in the repository, and there is no `.claude/skills/` or `.agents/skills/` directory `[VERIFIED: directory listing]`. The owner's global instructions and STATE.md "Standing Owner Rules" apply:

- Never add AI attribution to commit messages or PRs.
- No push without explicit owner approval.
- Edits under `port/src` normally need the diff shown first; waived to a phase-end review for this phase (D-28).
- An x86_64 kernel byte change triggers the `boot-iso` `outputHash` procedure on the owner's Linux host, the x64 TCG gates and the owner's boot-iso check; two stops this phase (D-26).
- An aarch64 kernel byte change gets the quick pass (tier-1 on HVF and TCG; smoke, net-smoke, cpu once on HVF).
- Kernels are built without FP/SIMD; kernel SIMD goes only through the `fpu` module.
- Harvey is read-only. Never stage the 11 pre-modified font files there.
- Builds and tests run through `nix develop -c cargo xtask ...`; `gh` is not installed.
- A knowledge graph exists (`graphify query` from the repo root); it is 588 commits behind HEAD, so treat its relationships as approximate.

## Answers to the "Research must confirm" list

### 1. Do the soft-float kernel assemblers accept AVX2 and NEON?

**Yes on both targets. No `l.S` fallback is needed.** `[VERIFIED: probe]`

A throwaway `#![no_std]` staticlib was built with `cargo build -Z build-std=core --target lib/x86_64-unknown-none-elf.json` and the aarch64 spec, on the pinned `nightly-2026-01-30` (rustc 1.95.0-nightly). The x86_64 probe held one `global_asm!` routine and one inline `asm!` block; `llvm-objdump` showed the encodings:

```
c5 fe 6f 06        vmovdqu  (%rsi), %ymm0
c5 fd fe c1        vpaddd   %ymm1, %ymm0, %ymm0
c5 e5 72 f0 07     vpslld   $0x7, %ymm0, %ymm3
c4 e2 7d 00 c5     vpshufb  %ymm5, %ymm0, %ymm0
c4 e3 7d 46 d1 20  vperm2i128 $0x20, %ymm1, %ymm0, %ymm2
c5 fd e7 07        vmovntdq %ymm0, (%rdi)
0f ae f8           sfence
48 0f ae 27        xsave64  (%rdi)
48 0f ae 37        xsaveopt64 (%rdi)
48 0f ae 2f        xrstor64 (%rdi)
0f 01 d0 / d1      xgetbv / xsetbv
66 0f ae 3f        clflushopt (%rdi)
```

The aarch64 probe assembled `ld1`, `st1`, `add v.4s`, `eor`, `shl`, `sri`, `rev32`, `tbl`, `ext`, `stnp q0, q1` and `movi` in both `global_asm!` and inline `asm!`, with `.arch_extension simd` at the top. The tree already does this in production: `aarch64/src/fpu.rs` `xor_fold16` opens with `".arch_extension simd"` `[VERIFIED: aarch64/src/fpu.rs:441-443]`.

Three details matter to the plan:

- `l.S` is not a different assembler. It is included through `global_asm!` already: `core::arch::global_asm!(include_str!("l.S"), options(att_syntax));` `[VERIFIED: x86_64/src/main.rs:46]`. The guide's fallback would have used the same LLVM integrated assembler.
- A vector register may be named as a clobber on the soft-float target: `out("ymm0") _` compiled `[VERIFIED: probe]`. Naming one as an input or output operand was not tried; the existing code avoids it and uses `clobber_abi("C")`.
- `#[target_feature(enable = "avx2")]` compiles silently on the x86_64 soft-float target and produces **scalar** code: a function that adds two `[u32; 8]` arrays compiled to eight `addl` instructions, and `_mm256_setzero_si256()` became an out-of-line call `[VERIFIED: probe]`. Any library that relies on `target_feature` gets no vector code there. Kernel vector code must be asm.

### 2. What does enabling AVX need, and what must each FXSAVE site change?

**Enable sequence.** `[VERIFIED: probe]` for the CPUID and XCR0 values, measured in a bare-metal multiboot probe under QEMU 11.1.2 TCG; `[ASSUMED]` for the instruction rules that come from the Intel SDM.

| Step | Value observed |
|------|----------------|
| CPUID.1:ECX bit 26 (XSAVE) present | `qemu64`: no (`ECX=0x80002001`). `qemu64,+avx,+avx2,+xsave,+xsaveopt`: yes (`0x94002001`). `max`: yes |
| Set `CR4` bit 18 (OSXSAVE) | CPUID.1:ECX bit 27 then reads 1 (`0x9c002001`) |
| `XGETBV(0)` at reset | `1` (x87 only) |
| `XSETBV(0) = 7` (x87, SSE, AVX) | accepted |
| CPUID.(0xD,0) EAX (XCR0 bits the CPU supports) | `7` on the AVX2 model; `0x21f` on `max` (includes PKRU and MPX bits, no AVX-512) |
| CPUID.(0xD,0) EBX (size for the enabled set) | `0x240` = 576 before `XSETBV`, `0x340` = **832** after |
| CPUID.(0xD,0) ECX (size if everything were enabled) | `0x340` on the AVX2 model; `0xa88` on `max` |
| CPUID.(0xD,2) (AVX component) | size `0x100` = 256, offset `0x240` = 576 |
| CPUID.(0xD,1) EAX | `1` on the AVX2 model (`XSAVEOPT` only); `5` on `max` (`XSAVEOPT`, `XGETBV` with ECX=1); no `XSAVEC`, no `XSAVES` |

So the save area is 832 bytes (512 legacy, 64 header, 256 for the upper halves of `ymm0`-`ymm15`) and must be 64-byte aligned `[ASSUMED: SDM]`. Size it from CPUID.(0xD,0) EBX after `XSETBV` and assert it equals 832; never use ECX, which on `max` reports 2696 bytes for features this kernel leaves off.

**The kernel cannot set OSXSAVE unconditionally.** `qemu64` has no XSAVE. Writing `CR4.OSXSAVE` there raises `#GP` `[ASSUMED: SDM]`. The tree already carries one scar of this kind: `l.S` gates `CR4.FSGSBASE` because "an ungated CR4.FSGSBASE write wedged early boot" under `qemu64` `[VERIFIED: x86_64/src/l.S:247-250]`. The CR4 write lives in two places in `l.S` (lines 262-265 for the first CPU and 492-498 for the application-processor path), both setting `Cr4OSFXSR|Cr4OSXMMEXCPT` `[VERIFIED: x86_64/src/l.S:264, 497]`. Do the XSAVE enable in Rust after CPUID, before `fpu::selftest()` at `x86_64/src/main.rs:299`, and leave `l.S` alone for the first CPU.

**What each FXSAVE site must change.** `[VERIFIED: grep of x86_64/src for fpu::, read of fpu.rs]`

| Site | Today | Change |
|------|-------|--------|
| `fpu.rs` `FpState` (line 67) | `pub fx: [u8; 512]`, `align(16)`, two `const` asserts on 512 and 16 | 832 bytes, `align(64)`; the FXSAVE path uses the first 512 bytes of the same struct |
| `fpu.rs` `save_raw` (180), `restore_raw` (198) | `fxsave64` / `fxrstor64` | One boot-time flag chooses `fxsave64`/`fxrstor64`, `xsave64`/`xrstor64`, or `xsaveopt64`/`xrstor64`; XSAVE needs `edx:eax = 7` as the mask |
| `fpu.rs` `FpState::DEFAULT` (82) | FCW `0x037F`, MXCSR `0x1F80`, rest zero | Add a valid header: `XSTATE_BV` such that the AVX component restores to its initial state, `XCOMP_BV = 0`, reserved header bytes zero (a non-zero reserved byte makes `XRSTOR` fault) `[ASSUMED: SDM]` |
| `fpu.rs` `same_user_state` (111) | compares fixed legacy ranges | Add the 256 AVX bytes, honouring `XSTATE_BV`: a component whose bit is clear is in its initial state whatever the bytes say |
| `fpu.rs` `restore` (222) | asserts MXCSR has no reserved bits | Keep; `XRSTOR` loads MXCSR from the legacy area whenever SSE or AVX is in the mask `[ASSUMED: SDM]` |
| `fpu.rs` `kernel_begin_on` (342), `Drop for KernelSimd` (365) | `save_raw(owner)`, `restore_raw(parked)` | Use the chosen instruction; this is the full section and may use XSAVE (D-12 excludes only the restricted section) |
| `fpu.rs` `image()` (441), `selftest()` (476) and the host tests | build and compare 512-byte images | Add `ymm` content and an AVX leg that runs only when CPUID reports AVX |
| `proc.rs:1048, 1053, 1158-1159` | `fpsave`, `note_fpsave` fields | Type change only; both grow from 512 to 832 bytes per process |
| `proc.rs:1789, 2121` | `fpu::install` | No code change |
| `proc.rs:1904, 1985` | `fpu::leave` | No code change |
| `proc.rs:2144` | `fpu::discard` | No code change |
| `proc.rs:2694` | `child.fpsave = fpu::snapshot(&parent.fpsave)` | No code change; the value copy grows |
| `exec.rs:288` | `fpu::replace(&mut proc.fpsave, &FpState::DEFAULT)` | No code change |
| `syscall.rs:593-594`, `2245` | notes: `note_fpsave` snapshot and replace | No code change; the state stays in the kernel |
| `syscall.rs:2351-2384` | tests compare `FpState` with `assert_eq!` | Still correct for byte copies; see Pitfall 3 |
| `trap.rs:415-423` | comment: the trap stub aligns `%rsp` to 16 for "FXSAVE of a stack FpState" | A stack `FpState` now needs 64-byte alignment; `align(64)` on the type makes the compiler realign the frame. State this in the comment and keep `snapshot`'s by-value temporary |
| `dat.rs:390, 425` | `CpuFp` in `Mach` | Unchanged |

`XSAVE` is never on the fault or eviction path: those use the restricted section (Pattern 1), which touches six registers with `vmovdqu` and no save area.

**`XSAVEOPT` availability and cost.**

- TCG: present only if the CPU model asks for it; `qemu64` does not, `max` does `[VERIFIED: probe]`.
- KVM `-cpu host`: not measurable here. `[ASSUMED]` it is present on any host newer than Sandy Bridge. The settling probe on the owner's Linux host is `grep -o -w -e xsave -e xsaveopt -e avx2 -e clflushopt /proc/cpuinfo | sort -u`.
- Cost under TCG, 200,000 iterations per instruction in a bare-metal loop, two runs each, in host-clock ticks per iteration including about 35 ticks of loop overhead `[VERIFIED: probe]`:

| Instruction | `qemu64,+avx,+avx2,+xsave,+xsaveopt` | `max` |
|-------------|--------------------------------------|-------|
| `fxsave` | 37 | 37 |
| `fxrstor` | 37 | 38 |
| `xsave` (state just restored from an all-zero header) | 57 | 55 |
| `xsaveopt` (same state) | 56 | 55 |
| `xsave` with `ymm0` and `ymm7` loaded | 56 | 56 |
| `xsaveopt`, same area as the last `xrstor` | 56 | 55 |
| `xrstor` | 63 to 65 | 88 to 95 |
| spill, `vpxor`, reload of `ymm0`-`ymm5` (18 instructions) | 37 | 37 |

Reading: under TCG `XSAVEOPT` saves nothing over `XSAVE`, and the header always came back with `XSTATE_BV = 7`, so TCG does not model the init or modified optimizations. A process switch costs about 45 more ticks than today (one save plus one restore). That is noise for gate time. The owner's expectation that a process which never touches AVX pays close to the FXSAVE cost can only be tested on real hardware under KVM; this research could not test it.

A first run of the probe put the save area on the same page as the probe's code and measured `xsave` at 445 ticks. That was QEMU's self-modifying-code path, not XSAVE. It is worth remembering when the kernel benchmark is written: keep benchmark buffers off pages that hold code.

### 3. Which TCG CPU model provides AVX2, and what does it cost?

`[VERIFIED: probe]` Started with `-machine q35 -accel tcg -cpu <model>,enforce`, which refuses to start if TCG lacks a requested feature:

| Model | Result |
|-------|--------|
| `qemu64,+avx,+avx2,+xsave,+xsaveopt` | starts clean |
| `qemu64,+avx,+avx2,+xsave,+xsaveopt,+clflushopt,+fsgsbase,+bmi1,+bmi2` | starts clean |
| `max` | starts clean; has `xsave`, `xsaveopt`, `avx`, `avx2`, `fma`, `bmi2`, `clflushopt`, `fsgsbase`, `rdrand`, `pku`, `x2apic`; no `avx512f`, no `xsavec` |
| `Haswell`, `Haswell-v4`, `Skylake-Client`, `EPYC` | refused: TCG lacks `pcid`, `tsc-deadline`, `invpcid`, `hle`/`rtm` or `xsavec` |
| `qemu64,+xsavec`, `qemu64,+xsaves`, `qemu64,+avx512f` | refused |

Plain `qemu64` reports `xsave`, `xsaveopt`, `avx`, `avx2`, `clflushopt`, `fsgsbase`, `rdrand`, `ssse3`, `sse4.2`, `popcnt` all false and `clflush` true.

**Use `qemu64,+avx,+avx2,+xsave,+xsaveopt,+clflushopt` for the vector run.** It changes only what the phase tests. `max` also turns on `fsgsbase`, `x2apic`, `rdrand` and `pku`, each of which switches a kernel path that the TCG gates otherwise do not exercise (the `CR4.FSGSBASE` gate in `l.S`, `has_x2apic` in `cpu.rs:235`, the entropy source). A named-feature model keeps a red result attributable. xtask already accepts `--cpu` and `R9_CPU`, so no xtask change is needed to pass it `[VERIFIED: xtask/src/main.rs:173-175]`.

**Gate-time cost: not measured.** An x86_64 boot cannot run on this host. What is known: the per-switch cost rises by about 45 ticks (table above), and the run is one extra `boot-test` under the existing TCG budget of 300 s times `TCG_OVERALL_MULT = 6` `[VERIFIED: xtask/src/main.rs:976]`. The settling probe, on a Linux x86_64 venue: `time nix develop -c cargo xtask boot-test --arch x86-64 --config q35exec --accel tcg --cpu qemu64` against the same command with the AVX2 model.

### 4. `fearless_simd` on the kernel targets, versus inline asm

**`fearless_simd` fails the probe on both targets. Write the Roaring kernels as inline asm.** `[VERIFIED: probe]`

`fearless_simd` 1.3.0 (published 2026-11-24, MIT or Apache-2.0, repository `github.com/linebender/fearless_simd`, MSRV 1.89; the legitimacy seam returned `OK`):

| Question | Result |
|----------|--------|
| Builds `no_std` with `default-features = false`? | No: `compile_error!("fearless_simd requires either the `std` or `libm` feature")` |
| Builds with `features = ["libm"]`? | Yes on both specs; it adds the `libm` crate to the lock file |
| Can a token be minted without `std` detection? | Yes: `unsafe { Avx2::assume_supported() }` and `unsafe { Neon::assume_supported() }` are `const fn` behind `#[target_feature]` |
| x86_64 kernel target: does a `u64x4` AND kernel emit vector code? | **No.** The linked archive holds zero instructions that name an `xmm` or `ymm` register. The crate works through `#[target_feature]`, which the soft-float ABI neutralizes (question 1) |
| aarch64 kernel target | Emits NEON, but every build reports the future-incompatibility lint `enabling the `neon` target feature on the current target is unsound due to ABI issues`, which rustc says "will become a hard error in a future release" (rust-lang/rust issue 134375) |
| Supply chain | Two new crates in a `forbid`-heavy tree, about 15,000 lines of generated code per backend, and a float API surface (`libm`) the kernel has no use for |

Inline asm in the reserved module needs four small routines per architecture (bitmap AND, OR, AND-NOT with population count, and array intersection) behind functions that already have fixed signatures:

```
pub(crate) fn bitmap_and(a: &[u64; BITMAP_WORDS], b: &[u64; BITMAP_WORDS], out: &mut [u64; BITMAP_WORDS]) -> u32
pub(crate) fn bitmap_or(...) -> u32
pub(crate) fn bitmap_andnot(...) -> u32
pub(crate) fn array_intersect(a: &[u16], b: &[u16], out: &mut Vec<u16>)
```
`[VERIFIED: ds/src/setops.rs:17-81]` with `BITMAP_WORDS: usize = 1024`.

Two things about the kernels themselves, both `[ASSUMED]`: AVX2 has no vector population count (`VPOPCNTQ` is AVX-512), so the cardinality comes from scalar `popcnt` on the result words or the `vpshufb` nibble-table method, and `qemu64,+avx2` does not imply `popcnt` unless it is also requested; and the array intersection is the only one of the four with a non-trivial vector form, so it can stay scalar behind the seam without breaking MEM-07's wording ("the Roaring set kernels have AVX2 and NEON implementations"), if the owner accepts three of four.

The token problem is separate from the library choice and is real; see Pitfall 5.

### 5. How can smoltcp deliver TCP payload into a caller-chosen buffer per page, and how many copies remain?

`[VERIFIED: port/src/devip/proto.rs:2475-2504, port/src/netif.rs:226-270, port/src/devmnt.rs:245-284]`

**The caller-chosen buffer already exists at the seam that matters.** `MntIo::read_some(&mut self, buf: &mut [u8]) -> Result<u64>` takes the destination slice. `frame_one` (`devmnt.rs:517-551`) today reads a 4-byte size, then allocates `alloc::vec![0u8; size - 4]` and reads the whole body into it. Ingress replaces that one `Vec` with a sequence of `read_some` calls: first into a small head buffer, then one call loop per page straight into the destination frame's bytes. No smoltcp change is needed.

**Copies between NIC and frame on the TCP path today: four.**

1. `netif.rs:249` `FrameStore::push`: `self.bytes[base..base + n].copy_from_slice(&frame[..n])`, the received frame into the conversation's frame store.
2. `netif.rs:267` `FrameStore::pop`: `buf[..n].copy_from_slice(&self.bytes[base..base + n])`, out to the smoltcp adapter.
3. Inside smoltcp 0.14.0: the TCP payload into the socket's receive ring, which is `tcp::SocketBuffer::new(alloc::vec![0; POOL_SOCK_BUF])` with `POOL_SOCK_BUF: usize = 2048` (`port/src/devip/stack.rs:706`).
4. `proto.rs:2504` `s.recv_slice(buf)`: ring to the caller's buffer. With ingress, this is the copy that lands in the destination frame.

`tcp::Socket::recv(|buf| ...)` lends the ring slice and would remove nothing here, since the destination is a different buffer `[ASSUMED: smoltcp API]`. With a 2048-byte ring, one page arrives in at least two `recv_slice` calls.

**The larger fact: the kernel's mount transport does not accept a TCP conversation at all.** `ChanIo::read_some` matches `DEV_PIPE` and `DEV_SSL` and otherwise returns `Err("devmnt: fd is not a 9P-capable connection")` `[VERIFIED: port/src/devmnt.rs:268-284]`. A 9P mount reaches TCP today only through a pipe or `devssl`. D-17's sentence about copies inside the TCP stack is therefore a statement about a path the phase does not build, and D-20 already says there is no network leg. The threat-model and RFC amendments should state the limit as "the transport below `MntIo`" and give the four-copy count for TCP as the present figure. For the pipe transport there is one copy (pipe queue to the caller's buffer) `[ASSUMED]`.

### 6. What does Harvey's userland assume about FP state size?

`[VERIFIED: read-only inspection of /Users/neilskinner/code/harvey and r9]`

- **Note frames: nothing.** r9 keeps the interrupted FP state in the kernel: `proc.note_fpsave = crate::fpu::snapshot(&proc.fpsave);` `[VERIFIED: x86_64/src/syscall.rs:2245]`. The frame written to the user stack holds the `Ureg`, the note text and two argument homes. Harvey's own amd64 kernel does the same (`sys/src/9k/k10/fpu.c` keeps `up->fpusave` as an `Fxsave` in the proc). No user-visible layout changes when the state grows.
- **`/proc`: one consumer, not reachable.** `libmach` opens `/proc/<pid>/fpregs` when `mach->fpregsize` is non-zero (`sys/src/libmach/map.c:106-116`), and the amd64 table sets `#define FPREGSIZE 512 /* TO DO? currently only 0x1A0 used */` (`sys/src/libmach/6.c:18`); arm64 uses 520 (`7.c:12`). r9's `port/src/devproc.rs` has no `fpregs` or `regs` file (grep found none), so `acid` and `db` cannot attach on r9 today and nothing reads FP state through `/proc`. When Phase 10 adds `fpregs`, it should serve the first 512 bytes (the legacy region) so that `libmach` keeps working.
- **User programs gaining AVX.** Harvey's `6l` knows VEX prefixes (`Vex2 = 0xc5`, `Vex3 = 0xc4` in `sys/src/cmd/6l/l.h:228-229`), so its assembler can emit AVX, but no Harvey program was found that probes CPUID for AVX and uses it. Nothing in the image starts using `ymm` registers because of D-13. `[ASSUMED]` for the "no program uses it" half: the search covered `libc`, `libsec`, `libmp` and `libthread` only.
- **Kernel memory:** two `FpState` values per process grow from 1,024 to 1,664 bytes.

### 7. Can a retained receive cipher key be reached by the packet path after a cutover, and what bounds its lifetime?

`[VERIFIED: port/src/chan/transport.rs:851-935, 1296-1319; crypto/src/session.rs:100-112, 363-368]`

**Today the packet path reads exactly one key slot.** `Session::open` builds the MAC from `self.recv_keys.mac()` (line 879) and decrypts with `crypt_body(&self.recv_keys, ...)` (line 927). At the marker, `handle_marker` does:

```rust
let old = core::mem::replace(&mut self.recv_keys, new_keys);
self.rekey.recv_superseded = Some(old);
if let Some(keys) = &mut self.rekey.recv_superseded {
    keys.zeroize();
}
self.recv_seq = 0;
```

`recv_superseded` is read only by `keys_zeroized()` and the `Superseded` status accessor; `open` never touches it. So a key that is not in `self.recv_keys` cannot verify or decrypt a packet.

**Recommended shape for D-18, which keeps that property by construction.**

- Add a type in `crypto/src/session.rs` that holds the cipher half only: the encryption key and the 4-byte nonce prefix, no MAC key, with one method `keystream(seq, counter, buf)`. `DirectionKeys` already separates the three fields (`k_enc`, `k_mac`, `prefix`) and already has `keystream`. The new type has no `mac()` method, so nothing built from it can authenticate a packet.
- At receipt, each page record takes an `Arc` of the current direction's cipher half. The `Session` keeps its own `DirectionKeys` and zeroizes its copy at the marker exactly as today, so the existing D-20 pins on `Superseded::Zeroized` stay green.
- The retained copy is `Zeroizing` and dies when the last `Arc` drops: when the last page is decrypted, or freed at exit or exec.

That gives three independent reasons the packet path cannot reach it: `open` reads only `self.recv_keys`; the retained object has no MAC key; and it is owned by page records, not by the session.

**What bounds its lifetime: only the pages.** A text page that is never touched lives as long as its image does, so a retired cipher key can outlive its connection by the life of a long-running process. There is no time bound in D-18. Two facts make this less alarming than it sounds: the key decrypts only data the kernel already holds, and each page is also covered by the D-19 page tag under a domain key. If the owner wants a bound, the High-Water Pager can convert pages whose key is retired (decrypt, re-seal under the domain key) during idle time. The owner rejected conversion as the primary mechanism; whether it is acceptable as a background bound is Open Question 5.

One more consequence of D-18 that the amendment text must cover: nonce uniqueness. The page ciphertext is under the wire nonce (`prefix || seq`) and counter `1 + 64 * i`. Decryption reuses that pair, which is fine. Re-sealing after eviction must use the domain key and a fresh nonce, never the wire key.

### 8. Size of the x86_64 lint opt-in

`[VERIFIED: probe]` `cargo clippy -p x86_64 -- -W clippy::undocumented_unsafe_blocks`, run under four cfg sets (the q35shell set, `dev_exec dev_churn`, `dev_exec dev_cap`, and bare `platform="q35"`), reported the same figure each time:

**32 undocumented `unsafe` blocks, in 9 files.**

| File | Blocks |
|------|--------|
| `x86_64/src/cpu.rs` | 13 |
| `x86_64/src/pio.rs` | 6 |
| `x86_64/src/seg.rs` | 3 |
| `x86_64/src/main.rs` | 3 |
| `x86_64/src/trap.rs` | 2 |
| `x86_64/src/proc.rs` | 2 |
| `x86_64/src/uart16550.rs` | 1 |
| `x86_64/src/dat.rs` | 1 |
| `x86_64/src/allocator.rs` | 1 |

The crate already carries 137 `SAFETY` comment lines. CONTEXT's "roughly 200 unsafe sites" is the size of the `UNSAFE-AUDIT.md` inventory to write, not of the comment work.

The 7 aarch64 sites are confirmed at exactly the lines STATE.md names: `platform/virt/rtc.rs:75`, `proc.rs:439`, `470`, `1823`, `2753`, `textcopy.rs:158`, `172`.

Mechanics the plan must include:

- Cargo forbids a crate-level `[lints.rust]` table beside `[lints] workspace = true`. `x86_64/Cargo.toml` lines 29-38 hold a `[lints.rust]` `unexpected_cfgs` table; it must be deleted. Every cfg it declares (`platform` with `"q35"`, `dev_ether`, `dev_ip`, `dev_exec`, `dev_bootchain`, `dev_churn`, `dev_cap`) is already declared in the workspace table `[VERIFIED: Cargo.toml:45-55, x86_64/Cargo.toml:29-38]`.
- The workspace `Cargo.toml` comment (lines 5-13) and `UNSAFE-AUDIT.md`'s scope line both say x86_64 is "never compiled by the gates" and not opted in; both need rewriting.
- The lint does not see `unsafe fn`, `unsafe extern` or test-only blocks; the audit documents those per file, as it does for aarch64.
- New cfg names for the sealed profiles must be predeclared in the workspace check-cfg list before first use (the rule written in that file).

## What DEBT-01 to DEBT-05 touch

| Item | Files | Notes |
|------|-------|-------|
| DEBT-01 | `xtask/src/main.rs` `TestStep::run` (3577-3690); `xtask/src/test9p.rs` module doc (lines 4-8 and 43-47 say the step "is not added to `cargo xtask test`"); `.github/workflows/xtask.yml` | CI's `tests` job runs `cargo xtask test --verbose`, so wiring both into `TestStep` covers CI with no workflow edit. Check `xtask/src/test9p.rs` tests for a pin on "fourteen commands" before changing the count. `cargo test -p xtask` spawned from inside `cargo xtask` is safe on Linux and macOS (the build lock is released before xtask runs) `[ASSUMED]`. The CI `tests` job runs without nix; `test-9p` uses `cargo()` so it works there, but confirm the Miri path is not pulled in |
| DEBT-02 | `.planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-VALIDATION.md` | Front matter is `status: draft`, `nyquist_compliant: false`, `wave_0_complete: false`; all 17 rows of the per-task map read `TBD` and `pending`; the sign-off list is unchecked and "Approval: pending". `/gsd-validate-phase 4` may not resolve the archived path, so plan it as a manual reconcile against the 25 SUMMARY files and `04-VERIFICATION.md` |
| DEBT-03 | `04-OWNER-STOP-2.md` line 55 ("the committed `boot-iso` `outputHash` **will not** match the new x86_64 kernel") and `evidence/owner-stop-2/RESULTS.md`, `evidence/kernel-bytes/KERNEL-BYTES-GAPS.md` in the same archive | The returned logs record `new-outputHash: unchanged`. The correction is a dated note, not a rewrite of the record |
| DEBT-04 | `ds/src/tpht/flattened.rs:157-159`, `ds/src/tpht/measure.rs:126-128, 247`, `port/src/fid.rs:618-623` | `default_banks(num_blocks)` is `num_blocks.div_ceil(64).max(2)`; the home-hit pin uses `banks_for(entries)` = `(2 * entries.div_ceil(BANK_NODES)).max(2)` with `BANK_NODES: usize = 255`; `FidTable` builds shards with `with_banks_and_probe(blocks_per_shard, 0, ...)`. A measurement-only change touches `measure.rs` tests and no kernel bytes. Changing `default_banks` changes `ds` code that `port` links, and may move kernel bytes; if so it belongs before owner stop 1. `ds/src/tpht/` edits also call for `cargo xtask miri` |
| DEBT-05 | `docs/threat-model.md` line 181 | The sentence reads "The compendium says `port` forbids `unsafe` in every module and that `rfork` bits 18 to 20 are already reserved `[AC 1.1]`. Tracked as notes N2 and N3. Not settled here." `port/src/rfork.rs:37-43` reserves `RFCKEY = 1 << 18`, `RFROBASE = 1 << 19`, `RFRANDMEM = 1 << 20`, `RFNOLEGACY = 1 << 21`. The document is checked by `xtask/tests/threat_model.rs` (eight tests, including a citation-or-`[new]` rule for every row and bullet), which DEBT-01 puts into the default test run. Lines 177-180 of the same list are the N9, N10 and N11 items that D-11, D-19 and D-22 also amend later in the phase |

## Architectural Responsibility Map

The web tiers of the template do not apply to a kernel. The tiers here are the crates.

| Capability | Primary owner | Secondary | Rationale |
|------------|---------------|-----------|-----------|
| Key hierarchy, page key derivation, cipher-half type for D-18, known-answer vectors | `crypto/` | none | D-02; crate-level `forbid(unsafe_code)` |
| Roaring pools' set kernels (scalar and vector) | `ds/` | arch `fpu` supplies the proof | D-16; the reserved module is the only new unsafe place in `ds` |
| Frame pools, page records, `BackingStore` seam, sealing logic, `StrideTracker`, `SuperpageResidency`, in-flight table, pager policy | `port/` | none | D-23; host-tested; no `unsafe` (D-06 default) |
| Raw frame access, PTE install and removal, TLB and cache flush, `CLFLUSHOPT` | arch crates | `port` calls through a trait | "raw frame access stays in the arch crates" |
| Vector cipher routine, restricted register section | arch crates (`cryptpage.rs`, `fpu.rs`) | `port` supplies the scalar reference through a re-export | MEM-04; lint rule R4 forbids an arch crate naming `crypto` |
| `#PF` and data-abort entry, kill on tamper | arch crates (`trap.rs`, `fault.rs`) | `port` pager decides | Trap context is arch-specific |
| Scatter receive of data pages, page tag at receipt | `port/src/chan/transport.rs`, `port/src/devmnt.rs` | `port` pager owns the frames | D-17, D-19 |
| Page-aligned `read(2)` frame swap | arch `syscall.rs` | `port` pager | Needs the caller's page table |
| Boot profiles, markers, gates | `xtask/`, `x86_64/lib`, `aarch64/lib` | arch probe drivers | Existing pattern |

## Standard Stack

No new external crate is recommended. Everything the phase needs is already pinned.

### Core (already in the tree)
| Library | Version | Purpose | Status |
|---------|---------|---------|--------|
| `blake3` | pinned in `Cargo.lock` (dependency of `crypto`) | keyed page tags | In use; `crypto/src/mac.rs` wraps it (`blake3_keyed`, `verify_keyed`, incremental `Blake3`) |
| `chacha20` | pinned in `Cargo.lock` | scalar reference cipher | In use through `crypto/src/page.rs` (`chacha20_block`, `chacha20_xor`) |
| `zeroize` | pinned | key scrubbing | In use (`Key`, `DirectionKeys`) |
| `smoltcp` | 0.14.0 | TCP below the mount transport | Unchanged this phase |
| `zerocopy` | 0.8.27 | `CpuFp` zero construction | Unchanged |

`[VERIFIED: Cargo.lock names blake3, chacha20, smoltcp 0.14.0; x86_64/Cargo.toml:13]` Exact `blake3` and `chacha20` versions were not read; the planner should not change them.

### Alternatives considered
| Instead of | Could use | Why not |
|------------|-----------|---------|
| Inline asm Roaring kernels | `fearless_simd` 1.3.0 | Emits no vector code on the x86_64 kernel target; unsound-ABI lint on aarch64; needs `libm` (question 4) |
| Inline asm ChaCha20 | the `chacha20` crate's AVX2 or NEON backend | Selected by `target_feature`, which the soft-float targets neutralize; the kernels are pinned to the soft backend by `--cfg chacha20_backend="soft"` in the cross legs `[VERIFIED: xtask/src/main.rs:3815-3832]` |
| A new unsafe module in `port` for frame bytes | arch-side trait implementation | D-06 default; needs owner approval otherwise |

**Installation:** none.

## Package Legitimacy Audit

This phase installs no external package.

| Package | Registry | Age | Downloads | Source repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `fearless_simd` | crates.io | first published 2018-10; 1.3.0 on 2026-11-24 | about 1.26M per week (seam) | github.com/linebender/fearless_simd | OK | Evaluated and **not adopted** (technical failure, question 4) |
| `libm` | crates.io | not checked | not checked | not checked | not run | Would have come with `fearless_simd`; not adopted |

**Packages removed due to SLOP verdict:** none.
**Packages flagged as suspicious:** none.

## Architecture Patterns

### System shape

```
                        user access to VA
                              |
                  +-----------v------------+
   x86_64 #PF --> | arch trap entry        | <-- aarch64 data/instruction abort
   (trap.rs)      | classify VA by segment |     (fault.rs: user_fault)
                  +-----------+------------+
                              |  page record lookup (per image / per process)
              +---------------+----------------+
              | Resident?                      |
       yes: protection fault -> kill     no: sealed or zero-fill
                                               |
                 +-----------------------------v--------------------------+
                 | port pager (no unsafe)                                 |
                 |  in-flight table (MEM-12) -> pop UninitializedFrame    |
                 |  from the clean pool (MEM-01/06/07)                    |
                 +-----------------------------+--------------------------+
                                               |
                 +-----------------------------v--------------------------+
                 | arch frame ops, under fpu restricted section (D-12)    |
                 |  ciphertext frame --ChaCha20--> shadow frame (unmapped)|
                 |  cached store for the fault line, NT stores for rest   |
                 |  SFENCE; keyed BLAKE3 over the ciphertext (scalar)     |
                 +-----------+--------------------------+-----------------+
                             | tag ok                   | tag bad
                   publish PTE, mark Active      zero frame, kill process
                             |                    "sys: crypto integrity violation"
                             v
                      return to user

  ciphertext frames come from three producers:
    image install (text once per image, data per process)      [D-09]
    eviction: High-Water Pager, Evictable = Dirty \ (Pinned | Active),
              seal under the domain key, free the shadow frame  [D-07, MEM-07]
    9P ingress: transport -> frame, packet MAC, page tag        [D-17..D-19]

  Roaring pool algebra runs outside the fault path, inside fpu::with_simd()
  when the CPU has the vector feature, scalar otherwise          [D-16]
```

### Recommended module layout

```
crypto/src/
  key.rs            page-key derivation labels + KATs; keep KeyCapability here
  session.rs        cipher-half type for D-18
  page.rs           page seal/open scalar reference + tag construction
ds/src/
  setops.rs         unchanged signatures; dispatch on an optional proof token
  simd/ (reserved)  x86_64.rs, aarch64.rs: inline-asm kernels (only new unsafe in ds)
port/src/
  palloc.rs         clean/dirty/pinned Roaring pools, UninitializedFrame, KeyID recycle rule
  segcrypt.rs       page records, BackingStore trait + RAM store, seal/open, StrideTracker
  superpage.rs      SuperpageResidency
  inflight.rs       write-once scratchpad
  pager.rs          High-Water Pager policy, fault decision, pin counts
  chan/transport.rs scatter open without data-page decrypt
  devmnt.rs         page-wise read into frames
x86_64/src/
  fpu.rs            XSAVE path + restricted section
  cryptpage.rs      AVX2 routine (global_asm!) + scalar fallback glue
  shadow.rs         frame bytes, PTE publish/unpublish, zero + CLFLUSHOPT
  mmu.rs            the pager's page-table operations (unmap, dirty bit, invlpg)
aarch64/src/
  fpu.rs            restricted section (NEON)
  cryptpage.rs, shadow.rs  counterparts
```

### Pattern 1: the restricted section is a sibling of `KernelSimd`, not a mode of it

**What:** a second guard type in `fpu.rs` whose constructor asserts interrupts disabled and no section open, spills the six registers to a 192-byte stack array with `vmovdqu`, and whose `Drop` scrubs with `vpxor` and reloads. It does not park the owner's state and runs no `XSAVE`.
**Why separate:** the full section's cost is one save and one restore of the whole user state; the restricted one must leave `ymm6`-`ymm15`, the x87 stack and MXCSR untouched and live.
**Rules that follow from the code as it is:**
- The spill and the reload must be in the **same** asm routine as the cipher, or the guard must be the only thing between two asm blocks. The compiler emits no vector code on this target, so two separate asm blocks are sound here in a way they are not on a hard-float target; still, one `global_asm!` routine (the guide's choice, MEM-04's wording) removes the question.
- Never emit `vzeroupper` or `vzeroall`: they clear the upper halves of registers the routine does not own.
- On a CPU without AVX (TCG `qemu64`) the scalar path runs and no section is opened at all.
- On aarch64 the section spills the `q` registers the NEON routine uses (16 bytes each) and scrubs with `movi`/`eor`.

### Pattern 2: typestate frames, with the unsafe confined to the arch crate

`UninitializedFrame` (a frame number, not mapped), `SealedFrame` (ciphertext, kernel-owned), `ShadowFrame` (plaintext, published) are `port` types that hold a frame number and a KeyID. `port` never forms a pointer. An arch trait does the byte work:

```rust
// port side: illustrative shape only, names are the planner's.
pub trait FrameOps {
    /// Decrypt `src` into `dst`; both are frame numbers. `fault_line` is the
    /// cache line to write with cached stores. Returns after SFENCE.
    fn open_into(&self, src: SealedFrameRef, dst: &UninitializedFrame,
                 key: &PageCipher, nonce: &[u8; 12], counter: u32, fault_line: usize);
    fn tag_of(&self, src: SealedFrameRef, mac: &PageMac, bind: &TagBinding) -> [u8; 32];
    fn zero_and_flush(&self, f: &UninitializedFrame);
}
```

Both architectures can fill a frame with no user mapping: x86_64's identity map of the first 4 GiB means "the returned PA doubles as its kernel VA" `[VERIFIED: x86_64/src/pagealloc.rs:104-107]`, and aarch64 has `physaddr_as_ptr_mut_offset_from_kzero` `[VERIFIED: aarch64/src/kmem.rs:132]`. This matters because the existing aarch64 demand path maps the page first and then writes through the user VA (`fault.rs:132-149`); the sealed path must not.

### Pattern 3: ciphertext frames are immutable; the shadow frame is the only plaintext

Decrypt always goes from a ciphertext frame to a different frame. That one rule satisfies D-15's order (the MAC is computed over ciphertext that still exists after the decrypt), makes clean eviction free (drop the shadow frame; the ciphertext is still in the store), and lets text be sealed once per image and decrypted into a private shadow frame per process. Its cost is two frames per resident sealed page. Whether that is acceptable against "one frame per page" for ingress is Open Question 2.

### Pattern 4: the cipher routine reads a boot-time dispatch, never CPUID on the path

One static, written once at boot after CPUID, selects scalar or vector for decrypt and for seal, and separately `CLFLUSHOPT`, `CLFLUSH` or none. The existing `cpu.rs` has the pure-decision-helper style to copy: `pub fn has_tsc_deadline(cpuid1_ecx: u32) -> bool` `[VERIFIED: x86_64/src/cpu.rs:226]`. AVX2 is CPUID.7:EBX bit 5; AVX usability also requires that the kernel itself enabled XCR0 bits 1 and 2.

### Pattern 5: the SIMD proof crosses three crates through one re-exported unsafe trait

`ds` cannot name an arch crate, and an arch crate cannot name `ds` (rule R4). The route that respects both:

- `ds` defines `pub unsafe trait VectorSection {}` (a promise that vector registers are usable and owned until the borrow ends) and takes `Option<&dyn VectorSection>` or a generic at the `setops` boundary.
- `port` re-exports it.
- Each arch crate writes `unsafe impl port::VectorSection for fpu::KernelSimd {}`.
- Host tests in `ds` use a test-only implementor, with a runtime AVX2 check on x86_64 hosts.

### Pattern 6: scatter receive with the placement derived from Size alone

D-03 says the MAC is verified "before any field is acted on", and `open` documents that "`Size` is the only field read before the MAC verifies" `[VERIFIED: port/src/chan/transport.rs:847-849]`. Placement of pages into frames has to happen before the MAC can be checked, because the MAC covers the pages. The body shape allows the page count to be computed from Size and nothing else: the header is 28 bytes, the MAC 32, "every parameter block fits one 64-byte block" `[VERIFIED: port/src/chan/msg.rs:15-19]`, and data is whole 4096-byte pages. If the extension TLVs plus parameters are under 4096 bytes, then `pages = (size - 28 - 32) / 4096` and the remainder is the head. After the MAC passes, the decoded layout must agree with that count or the connection closes. The MAC is fed in parts through the existing incremental `PacketMac::update(part)` `[VERIFIED: crypto/src/session.rs:268]`. Frames that received bytes from a packet whose MAC then fails hold attacker-chosen data and go back through the cross-domain scrub.

### Pattern 7: a profile is a config file, a `dev` key and an xtask profile arm

A boot profile at HEAD is three things: `x86_64/lib/config_<name>.toml` with `[config] dev = [...]`, which xtask turns into `--cfg dev_<key>` `[VERIFIED: x86_64/lib/config_q35churn.toml:36-38, xtask/src/config.rs:406-416]`; a `BootProfile` variant and marker stages in `xtask/src/main.rs` selected by `self.config_name == "<name>"` `[VERIFIED: xtask/src/main.rs:2170-2224]`; and usually a kernel-context driver module such as `x86_64/src/platform/q35/churngate.rs`. The "boot switch" of D-08 is, in this codebase, a compile-time cfg: a sealed-profile kernel is a different binary. That has a useful consequence and a cost. The default kernel does not contain the pager's hot code. But the default kernel's bytes still move whenever a shared file changes, because source line numbers are in the image.

### Anti-patterns to avoid
- **`#[target_feature]` on a kernel function:** compiles, emits scalar code on x86_64, and is the ABI-unsound lint on aarch64.
- **Mapping the page, then filling it:** what `aarch64/src/fault.rs` does today; forbidden for sealed pages (MEM-01).
- **Comparing two `FpState` values with `==` after separate saves:** see Pitfall 3.
- **Copying appendix listings 9.2 and 9.5:** non-normative (D-05). The names `fallback_difference` and the enum form of `KeyCapability` come from those sketches; the tree has `RoaringBitmap::difference` and a typestate `KeyCapability<S>`.

## Don't Hand-Roll

| Problem | Don't build | Use instead | Why |
|---------|-------------|-------------|-----|
| Scalar ChaCha20 | a second scalar cipher in `port` or the arch crates | `crypto::page::chacha20_xor`, `KeyCapability::apply_keystream` | It is the pinned reference the vector path must equal |
| Keyed BLAKE3, constant-time compare | a tree hash or a compare loop | `crypto::mac::blake3_keyed`, `verify_keyed`, `Blake3` (incremental) | Already pinned by vectors; D-01 |
| Key derivation | ad hoc XOR or hashing of the boot secret | `crypto::key::derive_labelled(parent, label, context)` | Ledger row 1 is exactly this mistake |
| Set difference for eviction | a hand-written bitmap | `ds::roaring::RoaringBitmap::difference` / `andnot` | MEM-07 names it; the SIMD seam sits under it |
| Random bytes for the boot secret | reading the TSC | `x86_64/src/entropy.rs` `rng_seed(buf)`, `port::devcons::random_u64()` | Existing source with a jitter fallback under TCG |
| In-memory client and server pair | a new fake transport | `port/tests/common/aead_link.rs` | D-20 names it |
| Kernel-context boot probe | a new harness shape | the `churngate.rs` pattern | Profile, markers and post-pass already exist |
| Packet MAC over scattered parts | concatenating into a buffer | `PacketMac::update` per part | The buffer is the copy D-17 removes |

**Key insight:** every cryptographic primitive this phase needs is in `crypto/` already. The new code is plumbing, typestate and two asm routines per architecture.

## Common Pitfalls

### Pitfall 1: setting `CR4.OSXSAVE` on a CPU without XSAVE
**What goes wrong:** early `#GP`, a wedged boot under the default TCG gates.
**Why:** `qemu64` has no XSAVE (probe). CI's three q35 boot tests and the quick gate all pass `--cpu qemu64`.
**Avoid:** CPUID.1:ECX bit 26 first; keep the FXSAVE path; print one boot line naming the chosen format so a gate can assert it.
**Warning sign:** a boot that stops before the banner under TCG only.

### Pitfall 2: sizing or restoring the XSAVE area wrongly
**What goes wrong:** `XRSTOR` faults on a non-zero reserved header byte or a set `XCOMP_BV`; or the area is sized from CPUID.(0xD,0) ECX and is 2696 bytes on `max`.
**Avoid:** build `DEFAULT` with a zeroed header; size from EBX after `XSETBV` and assert 832; `align(64)`.

### Pitfall 3: `XSAVEOPT` and byte comparison
**What goes wrong:** on real hardware `XSAVEOPT` may skip writing a component that is in its initial state or unmodified since the last `XRSTOR` from the same address, leaving old bytes in the buffer `[ASSUMED: SDM]`. Two saves of the same state then differ byte for byte. `FpState` derives `PartialEq` and the notes tests use `assert_eq!` on it `[VERIFIED: x86_64/src/syscall.rs:2374, 2384]`.
**Avoid:** compare through `same_user_state` with `XSTATE_BV` honoured; always initialize a buffer before saving into it (the code does: `let mut st = FpState::DEFAULT; save(&mut st);`); never rely on TCG to show this, because TCG always writes everything (probe).

### Pitfall 4: the Phase 4 "FP/SIMD scan equal to baseline" verdict
**What goes wrong:** the kernel-bytes evidence format has `VERDICT fpsimd-q35churnshell@04-14: EQUAL` `[VERIFIED: evidence/kernel-bytes/KERNEL-BYTES.md:72-73]`. This phase adds vector instructions to both kernels on purpose.
**Avoid:** redefine the check before owner stop 1 as an allow-list by symbol: vector and XSAVE-family instructions may appear only inside named symbols (`fpu`, `cryptpage`, the `ds` SIMD module). Anything else is still a failure. The same file has `text-under-2mib-q35churnshell: PASS` with text at about 760 KB; keep that check.

### Pitfall 5: lint rule R4 and the vector cipher's key
**What goes wrong:** `x86_64/src/cryptpage.rs` needs 32 raw key bytes and a 12-byte nonce in registers. `Key::bytes` is `pub(crate)` and `DirectionKeys` has "no accessor and no `Debug` impl" by design (T-04-03) `[VERIFIED: crypto/src/key.rs:78, crypto/src/session.rs:102-106]`, and an arch crate may not name `crypto` at all.
**Avoid:** decide the seam explicitly (Open Question 3). The narrowest form is a closure-scoped accessor on the page-cipher type in `crypto`, called by `port`, which passes the borrowed bytes to the arch `FrameOps` implementation.

### Pitfall 6: lint rule R3 and the reserved Roaring module
**What goes wrong:** R3 scans "every `.rs` file under `ds/src` that is not under `ds/src/tpht/`" for an `unsafe` token `[VERIFIED: xtask/src/lintcontract.rs:14-17]`. The reserved module will fail `cargo xtask test` the moment it holds asm.
**Avoid:** amend R3 and its fixtures in the same plan that creates the module, naming one path. `ds/src/lib.rs` already says the module "will be mounted at the crate root with #[path]" because a `forbid` on `roaring` reaches submodules. Miri cannot run inline asm; gate the module's tests with `cfg(not(miri))`.

### Pitfall 7: text is mapped writable on both architectures
**What goes wrong:** `PTE_USER_TEXT: u64 = PTE_P | PTE_RW | PTE_US` `[VERIFIED: x86_64/src/proc.rs:163]`, and aarch64 uses `Entry::rw_user_text()` `[VERIFIED: aarch64/src/fault.rs:128]`. A process can write its text. With text sealed once per image under a read-only key, a modified text page is silently lost at eviction, and a "clean" classification is wrong.
**Avoid:** in sealed profiles publish text shadow frames read-only and executable. On x86_64 the fill already goes through the kernel alias, so nothing needs the write bit.

### Pitfall 8: allocation failure panics
**What goes wrong:** `page_in_zeroed` does `alloc_phys_page().unwrap_or_else(|_| panic!("proc: phys page alloc failed"))` `[VERIFIED: x86_64/src/proc.rs:1404]`, and the aarch64 fault path does the same. A low-memory profile that relies on running out of frames halts the kernel.
**Avoid:** make `q35lowmem` force eviction by a cap on resident shadow frames (a profile constant the pager enforces), not by `-m` alone. Under D-07 a dirty anonymous page returns no memory when evicted, so shrinking RAM cannot be what triggers eviction anyway. Every profile today uses `memory = "1024M"`.

### Pitfall 9: `UserSlice` cannot unpin
**What goes wrong:** `UserSlice` is `{ ptr: *const u8, len: usize }` with no reference to the process `[VERIFIED: x86_64/src/usermem.rs:53-56]`. A pin taken at construction has nothing to release it at drop. Two slices can cover the same frame (the semaphore paths construct `UserSliceMut` on a 4-byte word repeatedly), so a pin is a count, not a bit.
**Avoid:** per-frame pin counts in the pager, with the Roaring `pinned` pool tracking count greater than zero; a pin guard that records the frame range and is `Drop`. There are 23 `UserSlice`/`UserSliceMut` sites in `x86_64/src/syscall.rs`; only those that live across a sleep need a pin (read and write through `devmnt`, `devpipe`, `devip`, `devssl`, and `await`).

### Pitfall 10: dirty tracking is not there
**What goes wrong:** MEM-07's dirty pool needs to know a page was written. x86_64 defines only `PTE_P`, `PTE_RW`, `PTE_US`, `PTE_NX` `[VERIFIED: x86_64/src/proc.rs:156-159]`; nothing reads the hardware dirty bit. The aarch64 gates run on `cortex-a72`, which has no hardware dirty state `[ASSUMED]`, and `fault.rs` treats a permission fault on a mapped page as a kill.
**Avoid:** x86_64 reads PTE bit 6 at eviction. aarch64 either treats every resident non-text page as dirty (always re-seal) or maps pages read-only first and upgrades on a permission fault. The first is far smaller; it is within "what is evictable" discretion.

### Pitfall 11: new user-mode probes cannot come from Harvey
**What goes wrong:** every binary in the boot images is `source = "harvey"` `[VERIFIED: xtask/manifests/amd64/marker.toml:21-50]`, built from C under Harvey's `build/x86_64/` (for example `churnmark.c`, `fpprobe.c`, `fpprobeasm.s`). Harvey is read-only. The phase needs a program that loads `ymm` registers and checks them across a switch, a note and an `rfork`; a program that touches pages after eviction; and one that does a page-aligned `read(2)`.
**Avoid:** the manifest schema already has `source = "rust"` (a workspace-relative r9 artifact) and `source = "generated"` (a registered generator) `[VERIFIED: xtask/src/image.rs:23, 30, 36]`. A probe of a few dozen instructions can be a generated `a.out` (40-byte header plus hand-encoded machine code, pinned by an xtask test). Where an existing Harvey binary is enough (forcing eviction by running `churnmark` many times, as `churngate` does), use a kernel-context driver instead. Open Question 4.

### Pitfall 12: the venue for x86_64 evidence
**What goes wrong:** RED and GREEN-twice evidence for anything in the `x86_64` crate cannot be produced on the research host: no x86_64 boot test, and the crate's host tests are skipped on an aarch64 host. AVX2 Roaring kernels in `ds` likewise run only on an x86_64 host.
**Avoid:** name the venue in each plan. Docker 29.6.1 (aarch64 server) is installed; whether its amd64 emulation exposes AVX2 was not tested. The CI `tests` job (ubuntu-latest, x86_64) runs `cargo xtask test`.

### Pitfall 13: `rfork` and exec assume eager, private pages
**What goes wrong:** `rfork_copy` clones `parent.mapping_log` and copies every page, text included ("Stack/Tos/Text stay fresh copies") `[VERIFIED: x86_64/src/proc.rs:2653, 2716]`. With eviction, a page may be absent from the page table and present only as ciphertext; `free_user_mappings`, `brk` shrink and the RFMEM `SHARED_PAGES` refcounts all walk `mapping_log` and assume a physical frame per record.
**Avoid:** every walker of `mapping_log` is a call site for the sealed profile. Under D-10 a child is a new key domain: its private pages must be re-sealed under the child's key or copied as plaintext into fresh frames, never shared as the parent's ciphertext. RFMEM sharing (same frame, two processes, two KeyIDs) contradicts "a key domain is one process" until Phase 7; decide what RFMEM does in sealed profiles.

### Pitfall 14: `test-9p` documentation and counts
**What goes wrong:** DEBT-01 reverses a deliberate, documented choice in `test9p.rs`; a unit test may pin the leg count.
**Avoid:** read the `test9p` tests before editing; update the module doc in the same commit.

## Code Examples

These are shapes verified to assemble on the kernel targets, not routines to copy.

### Restricted spill, scrub and reload (x86_64)
```rust
// Verified to assemble with -Z build-std=core --target lib/x86_64-unknown-none-elf.json.
// AT&T syntax, as the rest of x86_64/src uses.
core::arch::global_asm!(
    ".globl r9_probe",
    "r9_probe:",
    "vmovdqu %ymm0, 0(%rsp)",      // spill (caller reserved 192 bytes)
    "vmovdqu (%rsi), %ymm0",
    "vpaddd  %ymm1, %ymm0, %ymm0",
    "vpxor   %ymm2, %ymm0, %ymm0",
    "vpslld  $7, %ymm0, %ymm3",
    "vpsrld  $25, %ymm0, %ymm4",
    "vmovntdq %ymm0, (%rdi)",      // non-temporal store
    "sfence",
    "vpxor   %ymm0, %ymm0, %ymm0", // scrub
    "vmovdqu 0(%rsp), %ymm0",      // reload
    "ret",
    options(att_syntax)
);
```

### NEON with the extension directive (aarch64)
```rust
// Source: aarch64/src/fpu.rs:441-449 (existing, compiled by the gates)
core::arch::asm!(
    ".arch_extension simd",
    "movi v0.2d, #0",
    "ld1 {{v1.16b}}, [x0], #16",
    "eor v0.16b, v0.16b, v1.16b",
    /* ... */
);
```

### Boot-time XSAVE enable, in order
```
CPUID.1:ECX bit 26 set?            no  -> keep FXSAVE, scalar cipher, done
CR4 |= 1 << 18                     (OSXSAVE)
CPUID.(0xD,0):EAX has bits 0,1,2?  no  -> XCR0 = 3, no AVX, scalar cipher
XSETBV(0) = 7
CPUID.(0xD,0):EBX == 832           assert
CPUID.(0xD,1):EAX bit 0            -> XSAVEOPT available
CPUID.7:EBX bit 5                  -> AVX2: vector cipher and vector Roaring
CPUID.7:EBX bit 23                 -> CLFLUSHOPT
```
The values in this ladder are the ones the bare-metal probe read back under TCG.

## State of the Art

| Old approach | Current approach | Impact |
|--------------|------------------|--------|
| FXSAVE-only kernels (Harvey's `9k/k10`, r9 at HEAD) | XSAVE with a CPUID-sized area | User AVX; two formats to keep |
| `#[target_feature]` plus intrinsics for kernel SIMD | asm that takes a section guard | The only sound form on soft-float targets with this toolchain |
| Zero every frame on allocation | typestate frames, scrub only across key domains | MEM-01, MEM-06 |

**Deprecated or absent:**
- `XSAVEC` and `XSAVES`: not available under TCG; do not design for the compacted format.
- AVX-512: left off in XCR0 by decision; not available under TCG either.

## Runtime State Inventory

This is not a rename phase, but D-13 and DEBT-04 change state that lives outside the source tree.

| Category | Items found | Action |
|----------|-------------|--------|
| Stored data | None. r9 has no persistent store; the RAM backing store is rebuilt each boot | None |
| Live service config | None | None |
| OS-registered state | None | None |
| Secrets and env vars | The D-10 boot secret is new and lives only in kernel memory. `R9_CPU` and `R9_ACCEL` are read by xtask and are how the AVX2 TCG run is selected | Document the CPU string once, in xtask or the profile TOML |
| Build artifacts | `flake.nix` `boot-iso` `outputHash = "sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I="` (line 984) is stale after any x86_64 byte change; `target/images/x86_64/*.paq` are rebuilt on demand | Owner re-derives the hash at each of the two stops (D-26) |

## Assumptions Log

| # | Claim | Section | Risk if wrong |
|---|-------|---------|---------------|
| A1 | Writing `CR4.OSXSAVE` without CPUID XSAVE raises `#GP` | Q2, Pitfall 1 | Low; the CPUID gate is right either way |
| A2 | `XRSTOR` faults on non-zero reserved header bytes or a bad `XCOMP_BV`; MXCSR is loaded from the legacy area when SSE or AVX is in the mask | Q2, Pitfall 2 | A wrong `DEFAULT` image faults at first `install`; caught by the boot self-test |
| A3 | On real hardware `XSAVEOPT` skips unmodified or initial components | Q2, Pitfall 3 | If it never skips, the pitfall is moot; if it does and comparisons ignore it, tests fail only on real CPUs |
| A4 | KVM `-cpu host` on the owner's host has XSAVE, XSAVEOPT, AVX2, CLFLUSHOPT | Q2 | If AVX2 is absent the vector path and MEM-13's vector column have no KVM venue |
| A5 | A process that never uses AVX pays near-FXSAVE cost with `XSAVEOPT` | Q2 | The owner's stated expectation; unmeasured |
| A6 | The AVX2 TCG run costs about one ordinary boot-test | Q3 | Gate time estimate off |
| A7 | AVX2 has no vector popcount; cardinality needs scalar `popcnt` or a nibble table | Q4 | Kernel design detail |
| A8 | The pipe transport has one copy between queue and caller buffer | Q5 | Wording of the D-17 limit |
| A9 | No Harvey program in the image uses AVX when CPUID offers it | Q6 | A program could start using `ymm`; harmless if XSAVE is right |
| A10 | `cortex-a72` has no hardware dirty state | Pitfall 10 | If it has, aarch64 could track dirt in hardware |
| A11 | Scalar BLAKE3 over 4096 bytes costs more than the vector ChaCha20 pass, so the MAC dominates a cold fault | Open Question 13 | Benchmark interpretation only |
| A12 | `cargo test -p xtask` can be spawned from `cargo xtask test` without a lock conflict | DEBT-01 | The leg would hang; run it once to confirm |
| A13 | Docker's amd64 emulation on this Mac exposes AVX2 | Pitfall 12 | The local venue for x86_64 host tests disappears |
| A14 | About 55 to 60 plans | Phase size | The split question changes weight |

## Open Questions

Questions 1 to 6 need the owner; the rest can be settled by the planner with a recorded reason.

1. **D-13 says AVX is enabled "in every x86_64 boot"; TCG `qemu64` cannot.**
   - Known: `qemu64` has no XSAVE; the CI q35 boot tests and the quick gate use it.
   - Unclear: whether the owner wants the default TCG CPU model changed (which re-baselines every TCG gate) or the reading "every boot whose CPU has XSAVE and AVX; FXSAVE otherwise".
   - Recommendation: the second reading, with a boot line naming the format, and success criterion 3 reworded to match. The first owner stop's TCG gates then prove only that the FXSAVE path still works; the XSAVE path is proven under KVM and by the AVX2 TCG run.

2. **In-place decrypt, D-15's order and "one frame per page".**
   - Known: D-15 decrypts first and then MACs the ciphertext. A page that is decrypted in place no longer has ciphertext to MAC.
   - Options: (a) always decrypt from a ciphertext frame into a second frame, so a resident sealed page costs two frames and "one frame per page" means "no intermediate buffer"; (b) verify first for in-place pages only, which splits the fault path in two; (c) release the ciphertext frame of a private page once its shadow is published, keeping only text and image data in the store.
   - Recommendation: (a) with (c) for private pages, stated in the MEM-09 amendment.

3. **Raw key bytes for the vector routine.**
   - Known: the arch crate needs 32 key bytes; `crypto` exposes none and R4 bars arch crates from naming it.
   - Recommendation: a closure-scoped accessor on a page-cipher type in `crypto`, usable from `port` only, recorded as an amendment to T-04-03. Needs the owner because it loosens a Phase 4 guarantee.

4. **Where user-mode probe binaries come from.**
   - Recommendation: generated `a.out` files from xtask for the `ymm` probe; kernel-context drivers plus existing Harvey binaries for eviction and tamper; decide the page-aligned `read(2)` probe when its rules are fixed.

5. **A bound on the retained receive key (D-18).**
   - Known: nothing bounds it but page lifetime.
   - Recommendation: accept that for this phase and say so in the PROJECT.md row; offer idle-time conversion by the High-Water Pager as a later tightening.

6. **Placement before the packet MAC.**
   - Known: pages must be placed before the MAC is checked. Pattern 6 derives the page count from Size alone and re-checks after.
   - Unclear: whether the owner reads that as consistent with "before any field is acted on".
   - Recommendation: record it as the D-03 reading in the RFC note.

7. **`SuperpageResidency` in 64 bytes with a counter.** 512 residency bits are 64 bytes; the ledger's defect was a 72-byte struct with a separate counter. Recommendation: eight atomic words, the count derived by population count, and "atomic counter reset" implemented as clearing the words on demotion. No superpages are mapped for user memory at HEAD, so this is a host-tested record with no kernel consumer.

8. **What "boot switch" means (D-08).** Recommendation: a `dev` key in the profile TOML, compiled as a cfg, as every existing profile does.

9. **MEM-12 cannot fire in the booted kernel.** All kernel code runs with interrupts off on one CPU and the pager never blocks, so two faults on one page cannot overlap; x86_64 text is not even shared between processes. Recommendation: build and host-test the table, call it on the fault path, and state in the plan that its concurrent behaviour is first exercised in Phase 8.

10. **RFMEM in sealed profiles.** Shared data and bss frames are one frame in two key domains. Recommendation: in sealed profiles an RFMEM family is one key domain for its shared pages, written into the D-10 amendment; or RFMEM children are refused in those profiles until Phase 7. Needs a line from the owner since D-10 is marked costly.

11. **Flush fallback.** `qemu64` has `CLFLUSH` and not `CLFLUSHOPT`. D-11 gates on leaf 7 bit 23 and is silent on the fallback. Recommendation: `CLFLUSHOPT` if present, else `CLFLUSH` if CPUID.1:EDX bit 19, else zero only; and on aarch64 zero plus `dc civac` per line, or zero only. State which in MEM-06.

12. **MEM-03 on aarch64.** There is no `SFENCE`; `stnp` is a hint. Recommendation: the NEON path uses `stnp` for the 63 lines and ordinary stores for the fault line, and MEM-03's `SFENCE` wording is stated as x86_64-only.

13. **What the benchmark will show.** With a scalar MAC over 4096 bytes on every fault, the vector cipher may move the total less than the guide expects `[ASSUMED]`. The report should separate cipher time from MAC time.

14. **KVM measurements.** None could be taken. MEM-13 and the `XSAVEOPT` expectation need the owner's host.

15. **Does `DEBT-04` change `default_banks`?** If yes, it moves kernel bytes and should land before owner stop 1.

## Environment Availability

| Dependency | Required by | Available here | Version | Fallback |
|------------|-------------|----------------|---------|----------|
| Pinned Rust nightly | everything | yes | rustc 1.95.0-nightly (2026-01-29), `nightly-2026-01-30` | none |
| QEMU | boot tests, probes | yes | 11.1.2 | none |
| `llvm-objdump`, `llvm-objcopy`, `rust-lld` | evidence scans | yes | in the toolchain's `rustlib/<host>/bin` | none |
| `grub-mkrescue`, `xorriso` | every x86_64 boot test | **no** (darwin devshell; `nixpkgs#grub2` unsupported on darwin) | none | Linux venue: CI, amd64 container, owner's host |
| KVM | MEM-13, D-13 cost, KVM gates | **no** | none | owner's x86_64 Linux host only |
| x86_64 host for `x86_64` crate tests and AVX2 `ds` tests | MEM-04, MEM-07 | **no** | none | CI `tests` job; amd64 container (AVX2 support untested) |
| HVF | aarch64 quick pass | yes `[ASSUMED]` (not exercised this session) | none | TCG |
| Docker | amd64 TCG gates | yes | 29.6.1, aarch64 server | none |
| `gh` | none needed | no | none | `nix run nixpkgs#gh` |

**Missing with no local fallback:** KVM. Everything that needs it is an owner-run step.
**Missing with a fallback:** x86_64 boot tests and x86_64 host tests (Linux venue).

## Validation Architecture

`workflow.nyquist_validation` is absent from `.planning/config.json`, so validation is enabled.

### Test framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in tests driven by `cargo xtask`; Miri for `ds/src/tpht/`; QEMU boot profiles through `cargo xtask boot-test`; composed gates through `cargo xtask gate` |
| Config | workspace `Cargo.toml`; `.planning/config.json` (`build_command`, `test_command`); profile TOMLs under `x86_64/lib` and `aarch64/lib` |
| Quick run | `nix develop -c cargo test -p port --lib <module>` or `-p crypto`, `-p ds`; seconds once built |
| Full suite | `nix develop -c cargo xtask test && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy --arch x86-64 --config q35shell && nix develop -c cargo xtask clippy --arch aarch64 --config virtshell` (after DEBT-01, `cargo xtask test` includes `test-9p` and `cargo test -p xtask`) |
| Conditional | `nix develop -c cargo xtask miri` when `ds/src/tpht/` changes (DEBT-04) |

### Test layers
| Layer | Venue | What it can prove |
|-------|-------|-------------------|
| L1 host unit and integration (`crypto`, `ds`, `port`) | any host | derivation vectors, seal and open round trips, tag binding, pools, pager policy, stride, superpage, in-flight, scatter receive, retained key lifetime |
| L2 arch-crate host tests | host of the same architecture | `fpu` format selection, restricted section register preservation, vector equals scalar. aarch64 on the Mac; **x86_64 only on an x86_64 host** |
| L3 lint and static | any host | lint contract R1-R6 as amended, clippy `undocumented_unsafe_blocks` on all five gated crates, kernel-spec cross legs |
| L4 aarch64 boot profiles | this host (TCG, HVF) | sealed fault, eviction, tamper kill, NEON preservation on a booted kernel |
| L5 x86_64 boot profiles, TCG | Linux x86_64 or amd64 container | default boots unchanged on `qemu64` (FXSAVE path); `q35lowmem`, `q35crypt` on `qemu64` (scalar) and on the AVX2 model (vector, XSAVE) |
| L6 x86_64 KVM | owner's host | XSAVE on real hardware, MEM-13, boot-to-rio gate, `boot-iso` |
| L7 evidence scans | any host with the built ELFs | kernel bytes before and after, vector instructions only in allow-listed symbols, `.text` under 2 MiB |

### Phase requirements to tests
"Exists" means the check exists at HEAD. Almost none do; file names are proposals.

| Req | Behaviour | Type | Command | Exists? |
|-----|-----------|------|---------|---------|
| DEBT-01 | `cargo xtask test` runs `test-9p` and `cargo test -p xtask` and fails if either fails | unit + smoke | `cargo test -p xtask test_step` (new) and `nix develop -c cargo xtask test` | no |
| DEBT-02 | `04-VALIDATION.md` front matter is not `draft`; no `TBD` or pending rows | doc check | `grep -c "status: draft\|⬜ pending\|TBD" <archived path>` equals 0 | no |
| DEBT-03 | evidence text carries the dated correction; no unsupported claim | doc check | `grep` for the correction note in `04-OWNER-STOP-2.md` | no |
| DEBT-04 | home-hit above 0.90 at the production bank count | unit | `cargo test -p ds tpht::measure` (new case) and Miri | partial (fixture-sized only) |
| DEBT-05 | threat model names bits 18-21 as reserved | doc test | `cargo test -p xtask threat_model` (new assertion) | partial (file and eight tests exist) |
| MEM-01 | a frame is never mapped before its tag verifies; same-KeyID reuse skips scrub, cross-domain does not | unit | `cargo test -p port --lib palloc` and `segcrypt` with a recording `FrameOps` fake | no |
| MEM-01 | same, on a booted kernel | boot | `q35crypt` and aarch64 counterpart markers | no |
| MEM-02 | flipped ciphertext byte or tag: frame zeroed, fault returns kill; MAC key differs from cipher key | unit | `cargo test -p port --lib segcrypt tamper` | no |
| MEM-02 | tamper kills the process, kernel and siblings live | boot | `cargo xtask boot-test --arch x86-64 --config q35crypt` | no |
| MEM-03 | store routing: one cached line, 63 non-temporal, `SFENCE` after the loop | static scan + unit | disassembly check of the `cryptpage` symbol; output equality for every fault-line index 0..63 | no |
| MEM-04 | vector output equals scalar for random keys, nonces, counters | arch host test | `cargo test -p x86_64 cryptpage` (x86_64 host); `cargo test -p aarch64 cryptpage` | no |
| MEM-04 | six registers preserved for the interrupted context and scrubbed inside | arch host test + boot self-test | `fpu` restricted-section tests; boot line in `q35crypt` under the AVX2 model | no |
| MEM-04 | user `ymm` survives switch, note, `rfork`; reset on exec | boot (ring 3 probe) | `q35crypt` leg under the AVX2 TCG model and under KVM | no |
| MEM-04 | no `XSAVE`/`XRSTOR` reachable from the fault or seal path; AVX-512 off | static scan | symbol allow-list scan; boot line printing `XCR0` | no |
| MEM-05 | oldest, middle, newest comparison detects a sequential stream on every fault | unit | `cargo test -p port --lib segcrypt stride` | no |
| MEM-06 | recycle rule and the flush gate | unit | `cargo test -p port --lib palloc recycle`; pure CPUID decision helpers in `x86_64/src/cpu.rs` | no |
| MEM-07 | address expression at chunk boundaries; pinned never evictable; `Evictable = Dirty \ (Pinned | Active)` against a `BTreeSet` model; about 15% clean | unit | `cargo test -p port --lib palloc` | no |
| MEM-07 | vector set kernels equal scalar on seeded inputs | unit | `cargo test -p ds setops` on aarch64 host (NEON) and x86_64 host (AVX2) | scalar only |
| MEM-08 | `size_of == 64`, `align_of == 64`; clear and reset | unit | `cargo test -p port --lib superpage` | no |
| MEM-09 | scatter receive leaves data pages as ciphertext in frames; no allocation of body size; tag at receipt; fault decrypts to the sent bytes | integration | `cargo xtask test-9p` with a new `port/tests/aead_ingress.rs` added to `PORT_TESTS` | no |
| MEM-09 | retained key: unreachable by `open` after cutover; zeroized when the last page goes | integration | `cargo test -p port --test aead_ingress retained_key` | no |
| MEM-09 | loopback probe: a user process faults an ingress page | boot | `q35crypt` probe leg | no |
| MEM-10 | a pinned frame is never in the eviction set; drop releases; overlapping pins count | unit | `cargo test -p port --lib pager pin`; `usermem` mock tests in both arch crates | partial (`usermem` tests exist for residency) |
| MEM-11 | four profiles pass; default boots print no sealed-paging marker | boot | `cargo xtask boot-test` for `q35lowmem`, `q35crypt` and the aarch64 pair; the existing profiles | no |
| MEM-11 | x86_64 under the lint; audit counts reconcile | lint + doc check | `cargo xtask clippy --arch x86-64 --config q35shell`; a count script for `UNSAFE-AUDIT.md` | no (32 failures today) |
| MEM-12 | one decrypt per page for duplicate requests; write-once | unit | `cargo test -p port --lib inflight` | no |
| MEM-13 | figures recorded | manual, owner's KVM host | benchmark profile; report file under the phase evidence directory | no; manual by nature |

### Sampling rate
- **Per task commit:** the one test file the task touches, then `nix develop -c cargo xtask test`.
- **Per wave:** the full suite; the aarch64 sealed profiles on this host; Miri if `tpht/` changed.
- **Before each owner stop:** full suite green twice; kernel-bytes evidence with the symbol allow-list scan; aarch64 quick pass; the x86_64 TCG gates on a Linux venue.
- **Phase gate:** all of the above plus the owner's KVM runs before `/gsd-verify-work`.

### Wave 0 gaps
- [ ] DEBT-01 itself, so that later RED pins in `xtask` tests and `test-9p` are part of the default run.
- [ ] `x86_64` lint opt-in (32 comments) and the 7 aarch64 comments, so new modules land under the lint.
- [ ] `port/tests/aead_ingress.rs` and its entry in `xtask/src/test9p.rs` `PORT_TESTS`.
- [ ] A recording fake `FrameOps` and a fake `BackingStore` under `port/tests/common/`.
- [ ] Profile TOMLs, `BootProfile` variants and marker constants for four profiles; check-cfg predeclarations.
- [ ] A generated ring-3 `ymm` probe and its manifest entries (amd64); the aarch64 equivalent for NEON.
- [ ] The symbol allow-list scan replacing the `fpsimd ... EQUAL` verdict.
- [ ] A Linux x86_64 venue procedure written down once (container or host) for L2 and L5.

## Security Domain

`security_enforcement` is not set to false, so this section applies.

### Applicable ASVS categories
| Category | Applies | Control |
|----------|---------|---------|
| V2 Authentication | no (no `Tauth` work; D-20 injects the secret) | none |
| V3 Session management | yes, narrowly | D-18 key lifetime; the session's own copy still zeroizes at the marker |
| V4 Access control | yes | key domain per process (D-10); `ReadOnly` capability for text; text mapped non-writable in sealed profiles |
| V5 Input validation | yes | packet Size bounds before placement; page count re-checked after the MAC; page-aligned `read(2)` argument checks |
| V6 Cryptography | yes | `crypto/` only: ChaCha20 (RFC 8439 layout), keyed BLAKE3, labelled derivation with known-answer vectors; never a hand-written primitive outside the pinned vector routine, which is pinned equal to the scalar reference |

### Threat patterns for this phase
| Pattern | STRIDE | Mitigation |
|---------|--------|------------|
| Relocation of a sealed page (ciphertext moved to another address or process) | Tampering | Bind the tag to KeyID, virtual page number and a version; discretion item |
| Replay of an old sealed version of a page | Tampering | Per-page version in the record and in the tag |
| Nonce reuse on re-seal | Information disclosure | Nonce from page identity plus a per-page seal counter; never the wire nonce under the domain key |
| Stale frame across key domains | Information disclosure | Zero then flush (D-11); frames that held unauthenticated packet bytes take the same path |
| Kernel vector state leaking to user | Information disclosure | Scrub before reload in the restricted section; default state loaded after a full section with no owner |
| Retired wire key reused for packets | Spoofing | Cipher half only, no MAC key, not in `self.recv_keys` |
| Unpinned user buffer evicted during a blocked read | Tampering | Pin counts (MEM-10) |
| Tag oracle through timing | Information disclosure | `verify_keyed` is constant-time; kill on first failure |
| User process modifies shared text | Tampering | Text PTE read-only in sealed profiles |

## Phase size

The owner decided on 2026-11-25 to keep this as one phase, and the planner raises a split only if the plan count calls for it. A count by work group, from what the code needs:

| Group | Plans (estimate) |
|-------|------------------|
| DEBT-01 to DEBT-05 | 5 |
| Lint opt-in: x86_64 comments, audit inventory, aarch64 sites, contract and Cargo edits | 3 |
| AVX and XSAVE: detection, dual-format `FpState`, callers and tests, AVX2 TCG leg, evidence, owner stop 1 | 5 to 6 |
| Crypto additions and restricted sections and cipher routines, both architectures | 6 |
| Portable core: pools, records and store, sealing, stride, superpage, in-flight, pager, pins | 8 |
| Roaring SIMD: token and contract, AVX2, NEON, wiring | 4 |
| x86_64 pager: profile plumbing, install sealing, `#PF`, eviction and page-table ops, `rfork`/exec/exit walkers, usermem | 6 to 7 |
| aarch64 pager: the same | 5 |
| Ingress: transport, retained key, `devmnt`, exec path, page-aligned `read(2)` on two architectures, probe | 6 to 7 |
| Profiles, benchmark, document amendments, audit reconcile, evidence, owner stop 2, `port/src` review | 6 |
| **Total** | **about 55 to 60** `[ASSUMED]` |

That is about twice the largest phase so far. It also has three internal dependency walls: nothing vector can be tested on x86_64 before XSAVE lands; neither pager can be written before the portable core's types are fixed; and ingress needs both a pager and answers to Open Questions 2, 3 and 6. In my judgement the phase cannot be planned at full fidelity in one pass, for a specific reason: plans for the pagers and ingress written today would encode guesses about six questions only the owner can answer and about measurements only the owner's host can take. If the owner keeps one phase, the workable form is to plan it in stages inside the phase: DEBT, lint and XSAVE now, up to owner stop 1; the rest after the owner has answered the open questions at that stop. If a split is raised, the natural lines are after owner stop 1, and between the x86_64 pager and the group of aarch64, ingress and Roaring SIMD.

## Sources

### Primary (HIGH confidence): the tree, read this session
- `x86_64/src/fpu.rs` (whole file), `x86_64/src/exec.rs:1-70`, `x86_64/src/trap.rs:395-430, 795-870`, `x86_64/src/proc.rs:156-310, 1355-1440, 2630-2720`, `x86_64/src/pagealloc.rs`, `x86_64/src/usermem.rs:1-80`, `x86_64/src/syscall.rs:2225-2250`, `x86_64/src/l.S` (CR0/CR4 lines), `x86_64/Cargo.toml`
- `aarch64/src/fault.rs` (whole file), `aarch64/src/fpu.rs:205-250, 425-470`, `aarch64/src/kmem.rs` (function list)
- `port/src/lib.rs` (module attributes), `port/src/devmnt.rs:235-300, 505-640`, `port/src/chan/transport.rs:845-935, 1295-1330, 1736-1758`, `port/src/chan/msg.rs` (layout comments), `port/src/devip/proto.rs:2470-2520`, `port/src/netif.rs:218-270`, `port/src/fid.rs:605-630`, `port/src/rfork.rs:23-43`, `port/src/bitmapalloc.rs:1-60`
- `crypto/src/key.rs:126-236`, `crypto/src/session.rs:100-112, 355-368`
- `ds/src/setops.rs:1-120`, `ds/src/lib.rs:8-32`, `ds/src/roaring.rs` (public API), `ds/src/tpht/flattened.rs:148-166`, `ds/src/tpht/measure.rs:112-135, 236-262`
- `xtask/src/main.rs` (TestStep 3565-3900, BootProfile 1700-1780, profile dispatch 2154-2224), `xtask/src/test9p.rs:1-60`, `xtask/src/lintcontract.rs:1-75`, `xtask/tests/threat_model.rs:1-50`, `xtask/manifests/amd64/marker.toml`
- `.github/workflows/xtask.yml`, `.github/workflows/gates.yml`, `flake.nix:955-990`, `Cargo.toml`, `lib/x86_64-unknown-none-elf.json`, `lib/aarch64-unknown-none-elf.json`, `rust-toolchain.toml`, `UNSAFE-AUDIT.md:1-60`
- Planning: `05-CONTEXT.md`, `REQUIREMENTS.md`, `STATE.md`, `ROADMAP.md`, guide sections 3, 4, 11; compendium 3.1-3.4; RFC 9; `intel/constraints.md` IG-M01 to IG-M08, IG-V01, IG-V02, AC-T01, AC-P01; archived `04-VALIDATION.md`, `v3.0-MILESTONE-AUDIT.md`, `KERNEL-BYTES.md` verdict lines; `docs/threat-model.md:172-186`
- Harvey (read-only): `sys/src/libmach/6.c:18`, `7.c:12`, `map.c:100-120`, `sys/src/9k/k10/fpu.c` (grep), `sys/src/cmd/6l/l.h:228-234`

### Primary (HIGH confidence): probes run this session, then deleted
- Assembler acceptance on both kernel specs (build plus `llvm-objdump`)
- `#[target_feature]` and intrinsic code generation on the x86_64 soft-float spec
- `fearless_simd` 1.3.0: build with and without `libm`, disassembly of a `u64x4` kernel on both specs, future-incompatibility report
- QEMU 11.1.2 TCG CPU models with `enforce`, and CPU property queries through the monitor
- A bare-metal multiboot program reading CPUID leaves 1, 7, 0xD, enabling OSXSAVE and XCR0, and timing FXSAVE, XSAVE, XSAVEOPT, XRSTOR and the six-register spill under three CPU models, two runs each
- `cargo clippy -p x86_64 -- -W clippy::undocumented_unsafe_blocks` under four cfg sets; `cargo clippy -p aarch64` under the virtshell cfg set
- `cargo xtask boot-test --arch x86-64 --config q35exec --accel tcg` (fails at `grub-mkrescue` on this host)
- crates.io API and the GSD legitimacy seam for `fearless_simd`

### Tertiary (LOW confidence): training knowledge
- Intel SDM behaviour of XSAVE, XRSTOR, XSAVEOPT and CR4 (items A1 to A3)
- smoltcp API details beyond the calls visible in the tree
- Relative cost of scalar BLAKE3 and vector ChaCha20

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. No new dependency; the one candidate was probed and rejected.
- Architecture: MEDIUM. The seams are derived from the tree and the lint rules, but six owner questions shape them.
- Pitfalls: HIGH for those tied to quoted code; MEDIUM for the XSAVEOPT one, which rests on the SDM.
- Performance and gate time: LOW. No KVM, no x86_64 boot on this host.

**Research date:** 2026-11-25
**Valid until:** 2026-12-09 for the code facts (the tree moves quickly); the toolchain and QEMU findings hold while `rust-toolchain.toml` and the flake's QEMU pin are unchanged.
