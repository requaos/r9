# Phase 5: Guide Phase 3 - Cryptographic Memory and Zero-Copy DMA Paging - Context

**Gathered:** 2026-11-25
**Status:** Ready for planning

<domain>
## Phase Boundary

r9 gains sealed demand paging: user pages are stored as ChaCha20 ciphertext with a keyed BLAKE3 tag in an in-RAM backing store, decrypt on fault into a shadow frame, are re-sealed on eviction, and a tampered page kills the process. A Roaring-pool page allocator with a High-Water Pager, the stride tracker, the superpage residency record, in-flight fault de-duplication and user-frame pinning come with it. `9P2000.aead` data pages are read from the transport straight into destination frames with no decrypt on receive. Both architectures get the pager, each with a scalar and a vector decrypt path. Sealed paging runs only in new boot profiles; default boots do not use it.

Requirements: MEM-01 to MEM-13. Open items settled here: N7, N9, N10 (for this phase), the second half of N11, and N2 for `x86_64`. Several requirements are amended by the decisions below (MEM-01, MEM-02, MEM-06, MEM-09, MEM-11; success criteria 2 and 3 wording).

Not in this phase: an out-of-RAM backing store (swap file, Venti); sealed paging on by default; a live sealed mount over the network, a real `Tauth` exchange or any factotum work; true NIC placement of data pages and the `#M` ring bypass (Phase 6); the legacy gateway and its bounce frames (Phase 6); `RFCKEY`, `RFROBASE`, `RFRANDMEM` behaviour and sibling isolation (Phase 7); SMP locking of the new structures (Phase 8); any Harvey edit.

</domain>

<decisions>
## Implementation Decisions

### Carried in (not re-asked)
- **D-01:** Keyed BLAKE3 is the only MAC, including page tags; ChaCha20 uses the RFC 8439 layout, 32-bit block counter and 96-bit nonce (Phase 3 D-05, D-06). One key and nonce cover at most 2^32 - 1 blocks (PROJECT.md Key Decisions).
- **D-02:** `KeyCapability` and the key hierarchy live in `crypto/`; `port/src/segcrypt.rs` imports them (Phase 3 D-22). The `ReadOnly` capability is a type-level guarantee, not a cryptographic one.
- **D-03:** Data messages carry a parameter block, then file data padded to whole 4096-byte pages with the true count in the parameters; data page `i` encrypts from block counter `1 + 64 * i` (Phase 4 D-09). The packet MAC is verified at receipt under connection keys before any field is acted on (Phase 4 D-26).
- **D-04:** Every requirement follows IG-V01: committed RED pin, implementation, GREEN twice, pin advance. The golden invariant (boots to rio on q35) holds at every commit.
- **D-05:** Owner standing rules apply (no push without approval, no AI attribution, Harvey read-only), except the `port/src` diff-first rule, which D-28 changes for this phase. The compendium's appendix listings (9.2, 9.5) are non-normative sketches and are not copied as-is.
- **D-06:** `ds/` may hold `unsafe` in `ds/src/tpht/` and one reserved Roaring SIMD module; a third place needs owner approval (Phase 3 D-09). A new unsafe-holding module in `port/` needs owner approval (IG-D07).

### Backing store, sealed set and scrubbing (N9 settled)
- **D-07:** An evicted page's sealed ciphertext lives in kernel-owned RAM frames behind a `BackingStore` seam. Eviction frees the plaintext shadow frame. A dirty anonymous page therefore returns no net memory in this phase; that is accepted. An out-of-RAM store plugs into the seam later. — **Reversibility:** reversible — the seam exists so that a swap-file or Venti store can replace the RAM store.
- **D-08:** Sealed demand paging is off in every default boot for the whole phase. It runs only in the new profiles (`q35lowmem`, `q35crypt`, and their aarch64 counterparts), selected by a boot switch. Turning it on by default is not part of this phase.
- **D-09:** In the sealed-paging profiles every user page is sealed. Text and data are sealed when the image is installed, text once per image; bss, stack and heap are sealed at eviction. The plaintext boot image (paqfs in RAM) is outside the claim.
- **D-10:** A key domain (KeyID) is one process. Private pages (data, bss, stack, heap) are sealed under a per-process key derived from a boot secret. Shared text is sealed once under a per-image key that every process running the image holds as `ReadOnly`. Phase 7 adds only the `RFCKEY` sibling split. — **Reversibility:** costly — the page-record key references, the recycle rule and Phase 7's `RFCKEY` are built on this domain shape.
- **D-11 (N9 settled):** A frame reused by a different key domain is zeroed and then flushed (`CLFLUSHOPT`, gated on CPUID leaf 7 EBX bit 23). Only same-KeyID reuse (`recycle_within`) skips both. This is the RFC section 11 rule. The guide section 4.4 "no scrubbing" wording and the compendium section 3.4 "scrubbed with `CLFLUSHOPT`" wording are recorded as corrected, and MEM-01's "popped without scrubbing" and MEM-06 are amended to match. The owner chose this over the guide's rule, which was the recommended option. — **Reversibility:** reversible — relaxing to the guide rule later is a local allocator change.

### Vector path and the FP rule (N7 settled)
- **D-12 (N7 settled):** The fault decrypt path gets its registers from a second, narrower section kind inside the `fpu` module. It spills, scrubs and restores only the six vector registers and hands out a guard token, like `KernelSimd`. The existing rules hold: interrupts disabled, no nesting, no sleeping inside. The owner's rule that kernel SIMD goes through `fpu` is kept and extended, not bypassed; `cryptpage.rs` does not spill registers itself. The cost target is the guide's: about 30 cycles for spill, scrub and restore. No `XSAVE`/`XRSTOR` runs on the fault path. The pager's seal path on eviction uses the same restricted section and routine, not a full `with_simd()` section; the full section is used only by the Roaring SIMD kernels (owner, 2026-11-25, during milestone requirements).
- **D-13:** AVX is enabled on x86_64: `CR4.OSXSAVE` set and `XCR0 = x87 | SSE | AVX`, with AVX-512 state left off (MEM-04). Per-process FP state moves from the 512-byte FXSAVE image to XSAVE, so user `ymm` state survives a switch, `rfork`, exec reset and note delivery. This applies to every x86_64 boot, not only the sealed profiles, and user programs gain AVX. XSAVE replaces FXSAVE only where FXSAVE runs today (process switch, `rfork`, exec, notes); it is never on the fault path (D-12). `XSAVEOPT` is used where the CPU has it, so a process that never touches AVX pays close to today's FXSAVE cost; research measures this under KVM and TCG (owner, 2026-11-25). — **Reversibility:** costly — every FP save site (switch, fork, exec, notes) and the user-visible CPU feature set change.
- **D-14:** CPUID chooses the scalar or the vector path at boot. TCG's default `qemu64` runs scalar. One extra TCG run with an AVX2-capable CPU model covers the vector path and the register-preservation probe without KVM. Scalar and vector outputs are pinned identical (MEM-04).
- **D-15 (MEM-02 and MEM-03 reconciled):** The fault path decrypts into the still-unmapped frame, issues `SFENCE`, computes the MAC over the ciphertext while the write-combining buffers drain, and publishes the frame only if the MAC passes. On failure the frame is zeroed and the process is killed. MEM-02's "verified in cache before decrypting" (corrections ledger row 5) is amended to "verified before the frame is published"; the rest of that row stands: Encrypt-then-MAC, the MAC is over ciphertext, dedicated MAC keys, no plaintext read back from DRAM. The owner chose this over verify-first, which was the recommended option. — **Reversibility:** costly — the vector routine's structure, the benchmark baseline and the amended ledger row all rest on this order.
- **D-16:** The Roaring SIMD kernels are built in this phase for both architectures, AVX2 and NEON, behind the `ds/src/setops.rs` seam, and are reached through `with_simd()`. A probe first tries `fearless_simd` on the soft-float kernel targets (builds `no_std`, a token can be minted from the `fpu` guard, supply-chain cost); if it fails, the kernels are written as inline asm in the reserved module, as `fpu.rs` already does for SSE. The owner chose "build now" over "measure first", which was the recommended option.

### Ingress (N10, N11 settled)
- **D-17:** "DMA straight into the backing store with no kernel copy" means no copy above the transport. The 9P client reads each data page from the transport straight into its destination `UninitializedFrame`: no intermediate kernel buffer, no decrypt on receive, one frame per page. 9P runs over TCP through smoltcp, so copies inside the TCP stack remain; this is stated as the limit. MEM-09 and success criterion 2 are amended to this wording. True NIC placement waits for a transport that can do it.
- **D-18:** A received page stays in its frame as wire ciphertext. Its page record stores what decryption needs (key reference, sequence nonce, block counter). A retired receive cipher key stays alive, reference-counted, only while undecrypted pages point at it, and is never accepted for packets again; it is zeroized when the last such page is decrypted or freed. This amends Phase 4 D-20, which zeroized old receive keys on the re-key marker, and the teardown rule of Phase 4 D-15. The owner chose this over converting pages before the key dies, which was the recommended option. — **Reversibility:** costly — it changes the key-lifetime rule of the locked wire decision set (IG-D11 row); the change must be recorded as a new dated PROJECT.md row and RFC note.
- **D-19 (N11, second half settled):** At receipt, after the packet MAC passes, the kernel computes a keyed BLAKE3 tag for each data page under the owning domain's page MAC key (a hash pass over ciphertext; no decrypt, no copy). The fault path checks that tag exactly as it does for an evicted page. Two checks under two keys: the packet MAC at receipt, the page tag at fault.
- **D-20:** Success criterion 2 is proven by `cargo xtask test-9p` on the host, driving ingress over the in-memory link with the real frame and page-record code, plus a probe under `q35crypt` in which the booted kernel pairs the client engine with a test server end over a loopback link with an injected secret and a user process faults the page through the real pager. There is no network leg and no live sealed mount.
- **D-21:** Two kinds of read take the sealed ingress path: an image read at exec from a `9P2000.aead` mount (text and data pages land as sealed frames in the image's backing store and decrypt on first touch; nothing blocks at fault time), and a `read(2)` of whole pages into a page-aligned user buffer, which swaps sealed frames into the caller's address space instead of copying. Every other `read(2)` keeps the verify, decrypt and copy path. The owner chose to include page-aligned `read(2)`; exec-only was the recommended option.
- **D-22 (N10 settled for this phase):** `docs/threat-model.md` is amended in this phase: the plaintext of a sealed page exists only in its shadow frame; kernel I/O buffers (plain 9P reads, pipes) and the plaintext boot image are named as outside the claim. The legacy gateway's bounce frames get their own line when Phase 6 builds them.

### Scope and review process (N2 settled)
- **D-23:** Both architectures get the sealed pager. The portable core (allocator pools, page records, stride tracker, sealing logic, in-flight table) goes in `port/` and is host-tested. x86_64 wires it to a new trap-context pager; aarch64 wires it to its existing fault path. aarch64 gets a scalar path and a NEON vector decrypt path under its own restricted section (D-12's shape), plus aarch64 counterparts of the two profiles. The owner chose this over x86_64-only, which was the recommended option. aarch64 kernel byte changes get the standing quick pass; the full HVF series stays at Phase 8.
- **D-24:** x86_64 has no trap-context pager at HEAD (text, data and bss are mapped eagerly at exec). Building one is part of this phase, not a precondition.
- **D-25 (N2 settled):** `x86_64` takes `[lints] workspace = true` as an early plan: every existing `unsafe` block gets a `// SAFETY:` comment and an `UNSAFE-AUDIT.md` entry, and the new modules then land under the lint. The 7 undocumented aarch64 blocks that fail `cargo xtask clippy --arch aarch64 --config virtshell` today are fixed in the same pass.
- **D-26:** The x86_64 byte-change procedure runs at two owner stops. The first follows the AVX/XSAVE change (D-13), which lands as its own small group of plans because it alters default-boot behaviour. The second is at phase end and covers everything else. Each stop is the full procedure: `boot-iso` `outputHash` re-derived and committed on the owner's x86_64 Linux host, the x64 TCG gates, and the owner's boot-iso check. Phase 4 D-25 and Phase 2 D-23 to D-29 are the model.
- **D-27:** The byte-change evidence follows Phase 4 D-29: `.text` identity is not the test, since the changes are intended.
- **D-28:** The `port/src` diff-first stop is waived for this phase, as in Phase 4 D-24. Plans commit once their tests are green; the owner reviews the cumulative `port/src` change at the phase-end stop, before any push. The standing rule is unchanged for other phases.

### Not decided in discussion
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` (Phase 5 section, lines 63-73) — goal, four success criteria, open questions N2 and N7, landing-site and profile notes; success criteria 2 and 3 are amended by D-17 and D-23
- `.planning/REQUIREMENTS.md` (lines 19-33) — MEM-01 to MEM-13; MEM-01, MEM-02, MEM-06, MEM-09 and MEM-11 are amended by D-11, D-15, D-17 and D-23
- `.planning/PROJECT.md` — Constraints (safety, kernel FP/SIMD, process rules; lines 88-99), notes N2, N7, N9, N10, N11 (lines 121-130), Key Decisions rows for IG-D07 (line 152), the MAC and ChaCha20 choices, the ChaCha20 per-key limit, the wire body shape and re-keying rows, and the IG-D11 lock (line 185)
- `.planning/STATE.md` — standing owner rules, the v3.1 backlog, the pre-existing lint and fmt failures
- `.planning/INGEST-CONFLICTS.md` — I5 (appendix listings are non-normative)

### Source documents
- `.planning/r9-implementation-guide-amended-v3.md` — §3 corrections ledger (lines 43-65; rows for `simd.rs`, `seg.rs`, `shadow.rs`, `page.rs`, `superpage.rs`, `access.rs`), §4.1 to §4.4 (lines 69-121; §4.4 amended by D-11, §4.2 order fixed by D-15), §5.4 Roaring (line 159), §8 Phase 3 row (line 261), §11 verification matrix and risks (lines 335-353)
- `.planning/r9-architecture-and-code-v4.md` — §2.2 threat boundaries (line 84; amended by D-22), §3.1 to §3.4 (lines 92-119), §5.7 zero-copy ingress (line 288; amended by D-17), listings §9.2 `segcrypt.rs` (line 687) and §9.5 `palloc.rs` (line 995), both non-normative
- `.planning/rfc-9p2000-aead.md` — §3.2 key hierarchy (line 72), §9 and §9.1 zero-copy ingress (lines 484-520; amended by D-17, D-19), §11 security considerations (line 561; the stale-frame rule D-11 adopts)
- `.planning/intel/constraints.md` — IG-D07 (line 47), IG-A01 (82), IG-M01 to IG-M04 (105-120), IG-M08 (140), IG-V01 and IG-V02 (195, 200), AC-T01 (211), AC-P01 (216), RFC-P10 (305)
- `docs/threat-model.md` — line 31 (in-scope list), lines 177-178 (N9 and N10 as left open by Phase 2); amended by D-11 and D-22

### Prior phase decisions
- `.planning/milestones/v3.0-phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-CONTEXT.md` — D-09 (body shape), D-15 and D-20 (key scope and re-key cutover; amended by D-18), D-16 and D-23 (injected secret, no live sealed mount), D-24 (the `port/src` waiver D-28 repeats), D-25 and D-29 (byte-change procedure and evidence), D-26 (wire half of N11)
- `.planning/milestones/v3.0-phases/03-guide-phase-1-crypto-and-ds-foundations/03-CONTEXT.md` — D-05, D-06 (MAC, ChaCha20), D-09 to D-12 (unsafe and lint contract), D-20 and the deferred `fearless_simd` note (Roaring SIMD), D-22 (`KeyCapability` location)
- `.planning/milestones/v3.0-phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-CONTEXT.md` — D-07, D-08 (lock rule), D-23 to D-29 (the x86_64 byte-change procedure as first run)

### Code at HEAD
- `x86_64/src/fpu.rs` — module doc (lines 1-54: FXSAVE state, `OSXSAVE` never set, kernel SIMD rules), `FpState` (line 67), `install` / `leave` / `snapshot` / `replace` (244-304), `KernelSimd` (327), `kernel_begin` (338), `with_simd` (383), `xor_fold16` (392); where D-12 and D-13 land
- `aarch64/src/fpu.rs` — the reference design with the same API (`kernel_begin` line 376, `with_simd` 420)
- `x86_64/src/exec.rs` (lines 1-30) — eager mapping policy and the statement that no trap-context pager exists
- `x86_64/src/proc.rs` — `MappingRecord` (line 195), `ByteBacking` and `ByteSource` (248-300), `ensure_user_resident_inherent` (1368), `page_in_zeroed` (1403)
- `x86_64/src/trap.rs` (near line 834) — where `#PF` reads `CR2` today
- `x86_64/src/usermem.rs` — `UserSlice` / `UserSliceMut` behind `UserAddressSpace`; where MEM-10 lands
- `x86_64/src/pagealloc.rs`, `port/src/pagealloc.rs`, `port/src/bitmapalloc.rs` — the current physical page allocator
- `aarch64/src/fault.rs` — `user_fault` (line 36), `page_in_zeroed` (246): the existing demand path D-23 extends
- `port/src/devmnt.rs` — `MntIo` (line 245), `frame_one` (517), `read_loop` (597), `negotiate` (688), `read` (920, 1206); where ingress attaches
- `port/src/devip.rs`, `port/src/virtio.rs` (`rx_replenish`, line 693) — the TCP and NIC receive path whose copies D-17 leaves in place
- `port/src/chan.rs`, `port/src/chan/`, `port/src/fid.rs`, `crypto/src/session.rs` — the Phase 4 engine and session keys D-18 amends
- `port/tests/common/aead_link.rs`, `port/tests/devmnt_test.rs` — the in-memory client/server link D-20 reuses
- `crypto/src/page.rs` (`chacha20_block` line 49, `chacha20_xor` 65), `crypto/src/key.rs`, `crypto/src/mac.rs` — scalar reference cipher, key hierarchy and `KeyCapability`, keyed BLAKE3
- `ds/src/setops.rs` — the four scalar hot loops D-16 replaces; `ds/src/roaring.rs`
- `xtask/src/main.rs` — `BootProfile` (line 1709), the q35 markers, `q35lowmcfg` (line 1231, unrelated to `q35lowmem`), test and check legs
- `x86_64/lib/config_q35*.toml`, `aarch64/lib/config_virt*.toml` — profile configuration files the new profiles join
- `x86_64/Cargo.toml` (line 29, `[lints.rust]`), `Cargo.toml` (lines 5-13, lint scope comment), `UNSAFE-AUDIT.md`, `flake.nix` (`boot-iso` `outputHash`)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `fpu::kernel_begin()` / `with_simd()` exist on both architectures with the same shape, with host tests; the restricted section (D-12) is a sibling of `KernelSimd`.
- `crypto/` provides the scalar ChaCha20 reference, keyed BLAKE3 with constant-time verify, HKDF, labelled derivation, a zeroizing `Key` and `KeyCapability`.
- `ds/` provides the Roaring bitmap with and-not for `Evictable = Dirty \ (Pinned | Active)`, the `setops` seam built for D-16, and TPHT.
- `devmnt` separates transport from session through `MntIo`, and `port/tests/common/aead_link.rs` already runs a client and a server end over an in-memory link.
- `usermem.rs` is the single validated entry point for user memory, so residency and pinning (MEM-10) have one place to land.
- aarch64 already has a fault-context demand path (`aarch64/src/fault.rs`).

### Established Patterns
- x86_64 maps text, data and bss eagerly at exec and has no pager; a binary on a 9P mount is read whole at exec into kernel memory because a fault handler must never block on an RPC. There is no eviction, swap or block device anywhere.
- The kernel is built soft-float; `CR4.OSXSAVE` is never set, so AVX is unusable at any privilege level at HEAD and user FP state is FXSAVE. Kernel SIMD lives in inline asm that takes the guard as proof; `#[target_feature]` is ABI-unsound on this target.
- All kernel code runs with interrupts disabled on a single CPU; Phase 8 retires that invariant, so new shared structures should not make it harder.
- `port/` uses per-module `forbid(unsafe_code)` with three named floor modules; `x86_64` is not under the deny lint and holds roughly 200 `unsafe` sites.
- The landing sites the documents name (`x86_64/src/mmu.rs`, `shadow.rs`, `cryptpage.rs`, `port/src/palloc.rs`, `segcrypt.rs`, `inflight.rs`, `superpage.rs`) do not exist at HEAD.
- Kernel builds are reproducible; source line shifts alone change loaded bytes. Before and after must be compared on one venue.
- Build and test commands: `nix develop -c cargo xtask check`, `nix develop -c cargo xtask test`.

### Integration Points
- `x86_64/src/trap.rs` (`#PF`), `x86_64/src/proc.rs` and `exec.rs` (image install, segment map), `x86_64/src/fpu.rs`, `x86_64/src/pagealloc.rs`; the aarch64 siblings.
- `port/src/lib.rs` module list, `port/src/devmnt.rs` read path, `port/src/chan/` session key lifetime, `crypto/src/session.rs`.
- `ds/src/setops.rs` and the reserved Roaring SIMD module.
- `xtask/src/main.rs` boot profiles and gates; `x86_64/lib` and `aarch64/lib` profile configs.
- Documents: REQUIREMENTS.md, ROADMAP.md, PROJECT.md, STATE.md, the RFC, `docs/threat-model.md`, `UNSAFE-AUDIT.md`, `Cargo.toml` lint scope comment.

### Research must confirm
- That the soft-float kernel target's assembler accepts AVX2 and NEON mnemonics in `global_asm!` / inline asm, and what XSAVE needs on QEMU TCG and KVM (CPUID leaf 0xD sizes, `XSAVEOPT` availability).
- Which TCG CPU model provides AVX2 in the pinned QEMU, and its cost in gate time.
- Whether `fearless_simd` builds `no_std` on r9's kernel target specs and how a capability token is minted without `std` feature detection.
- How smoltcp can deliver TCP payload into a caller-chosen buffer per page (D-17), and how many copies remain.
- What Harvey's userland assumes about FP state size in note frames and `/proc` (D-13).
- That a retained receive key (D-18) cannot be reached by the packet path after cutover, and what bounds its lifetime.
- The measured size of the x86_64 lint opt-in (D-25).

</code_context>

<specifics>
## Specific Ideas

- The owner took the stricter stale-frame rule (zero and flush across domains) over the guide's no-scrub rule.
- The owner kept the guide's latency hiding: decrypt, then verify before publish, rather than verify first.
- The owner wants AVX on for the whole system with XSAVE user state, not a profile-only switch, while sealed paging itself stays profile-only.
- The owner wants the Roaring SIMD kernels built now on both architectures, with the `fearless_simd` probe deciding the form.
- The owner wants aarch64 to get the full sealed pager with a NEON decrypt path in this phase.
- The owner prefers keeping a retired receive key alive for pending pages over sweeping pages at a re-key.
- The owner wants page-aligned `read(2)` to use the sealed ingress path as well as exec.
- Review model as in Phase 4: tests validate between plans, `port/src` review at phase end; plus an early byte-change stop after the XSAVE change.

</specifics>

<deferred>
## Deferred Ideas

- **Plan 9 swap file over 9P** as an out-of-RAM backing store: needs a pager that can block on RPCs; a later phase, or Venti in Phase 13, behind the D-07 seam.
- **Sealed paging on by default in every boot:** not before the profiles have run for a while; a candidate for Phase 7, when `RFCKEY` makes per-process keys user-visible.
- **True NIC placement of 9P data pages** (mergeable receive buffers, a TCP path without socket-buffer copies) and the `#M` ring bypass: Phase 6 (DIST-08).
- **A live sealed mount over virtio-net with a host-side `9P2000.aead` test server:** the phase that has a real key exchange, or Phase 6.
- **The legacy gateway's bounce frames in the threat model** (rest of N10): Phase 6.
- **TPHT bank-sizing precondition and the other v3.0 audit follow-ups** (CI wiring of `test-9p`, `04-VALIDATION.md`, the boot-iso evidence claim): v3.1 backlog, not folded into this phase.

</deferred>

---

*Phase: 5-Guide Phase 3 - Cryptographic Memory and Zero-Copy DMA Paging*
*Context gathered: 2026-11-25*
