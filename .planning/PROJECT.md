# r9

## What This Is

r9 is a faithful pure-Rust port of Bell Labs' Plan 9 and Harvey (the 64-bit Plan 9 4th Edition derivative). The Rust kernel already boots Harvey's C userland on QEMU q35 x86_64 (primary) and aarch64 virt (HVF/TCG). The work ahead replaces Harvey's C userland with Rust one binary at a time, adds a native encrypted 9P wire protocol (`9P2000.aead`) and cryptographic memory, then makes r9 self-hosting so Harvey can be archived. The owner is the sole product owner and works with a single implementer.

## Core Value

The OS boots to rio on QEMU q35 at every commit. This golden invariant outranks every feature: a change that breaks it is wrong, however good the feature.

## Requirements

### Validated

<!-- Inferred from the repository at HEAD 6ee45b5 and from HANDOFF.md (legacy-final checkpoint 116bcc2). The roadmapper did not re-run these. -->

- ✓ x86_64 q35 kernel boots Harvey's amd64 userland chain through to a shell and CPU server (`cargo xtask boot-test` profiles q35, q35net, q35exec, q35shell, q35cpu, q35auth, q35churn, q35churnshell, q35lowmcfg) — pre-roadmap (Harvey-rooted phases through 20)
- ✓ aarch64 virt kernel boots Harvey userland under HVF and TCG with smoke, net-smoke, tier-1 and cpu series — pre-roadmap; full HVF series deferred to Milestone S (see STATE.md)
- ✓ rio is exercised end to end by an r9-owned `cargo xtask gate` (HARN-01) as well as Harvey's composed gates — Phase 1: positive and negative rio legs on the Linux KVM witness, plus the nine x86_64 q35 profiles and the aarch64 quick pass through the reconciled mechanism
- ✓ Cross-build harness (HARN-01…08) — Phase 1: one documented `cargo xtask gate` boots r9 to rio from a pre-built read-only Harvey tree; one resolver with named errors and per-arch devshell supply; per-arch fallback manifests with two-way drift and Rust-provenance checks; a one-at-a-time Rust swap proven at flip and flip-back; flake ownership flipped (r9 imports Harvey `c697666`, Harvey has no `r9` input) and every moved gate passes from r9
- ✓ In-kernel devices implemented in Rust under `port/src/`: devcons, devdup, devenv, devether, devip (smoltcp), devmnt, devpipe, devproc, devroot, devsrv, devssl, devcap, plus paqfs and allocators — pre-roadmap
- ✓ Per-CPU FP/SIMD ownership (`CpuFp`, `fpu::kernel_begin()` / `with_simd()`) and x86_64 entry hardening (DF cleared on entry, GS base read via IA32_GS_BASE) at checkpoint 116bcc2 — pre-roadmap

### Active

<!-- Full list with IDs: .planning/REQUIREMENTS.md (127 v1 requirements, each mapped to exactly one phase). -->

- [ ] Design lock and four reserved `rfork` flag bits (DSGN, Phase 2)
- [ ] `crypto/` and `ds/` foundations: HKDF, ChaCha20, BLAKE3, TPHT, Roaring, ClientSet (CRYP, DS, LINT, Phase 3)
- [ ] `9P2000.aead` transport and `FidTable` (WIRE, Phase 4)
- [ ] Cryptographic memory and zero-copy DMA paging (MEM, Phase 5)
- [ ] Distributed 9P extensions, leases, legacy gateway, cluster service discovery with 30s unhealthy / 5m dead lifecycle (DIST, Phase 6)
- [ ] Process lifecycle, `rfork` crypto flags live, secure note delivery (PROC, Phase 7)
- [ ] Multi-CPU SMP scheduler and memory locking (SMP, Phase 8)
- [ ] `libstd_r9` native Rust `std` (STD, Phase 9)
- [ ] Track P: Harvey kernel devices, base CLI, complex userland and compilers in idiomatic Rust (KDEV, CLI, USER, Phases 10-12)
- [ ] Venti/Fossil BLAKE3 storage engine and cluster permission suites (STOR, PERM, Phase 13)
- [ ] `#G` Venus 3D acceleration and `draw(3)` host blit (GFX, Phase 14)
- [ ] Self-hosting toolchain: rustc, rust-lld, APEX, elf2aout (SELF, Phase 15)
- [ ] Harvey retired: archived read-only after the deprecation checklist passes (RETIRE, Phase 16)

### Out of Scope

- POSIX emulation calls, `mmap` extensions, or any new syscall — the system call list is closed (IG-D03); software adapts to `rfork`, `bind`, `mount`, 9P
- Kernel loading of ELF — `a.out` is the sole executable format (IG-D02); in-tree tooling converts linker ELF
- Physical DRAM bus probing, cold-boot attacks, and arbitrary code execution in kernel context — outside the threat model (AC-T01); AMD SEV-SNP or Intel TDX is the remedy if bus sniffing is ever added
- Funnel Hashing on primary paths (page cache, FidTable, Venti index) — restricted to write-once ephemeral scratchpads (IG-D09)
- Mixing classic APE and APEX C runtimes in one binary (IG-D06)
- Rewriting Harvey userland first in `#![no_std]` — that is the double-rewrite `libstd_r9` before Track P exists to avoid (IG section 9.1)
- Copying the architecture compendium's appendix source listings as-is — non-normative sketches that contradict the guide (INGEST-CONFLICTS I5)
- riscv64 and Raspberry Pi hardware — the crates remain in the tree but no phase targets them
- Modifying the Harvey tree — Harvey is a read-only reference and is never edited, including for the Phase 2 flag reservation (owner decision 2026-11-22); Harvey planning is frozen
- Enterprise process artifacts (stakeholders, sprints, estimates)

## Context

**Pinned versus current.** The four source documents (dated 2026-11-20) pin `requaos/r9` at `c787ce9` and `requaos/harvey` at `a6b7e6a9`. r9 HEAD is `6ee45b5`, 60 commits later. The last traditional-Plan-9 state is the annotated `legacy-final` tag (r9 `116bcc2`, Harvey `bd7a6982a`).

**Repository layout at HEAD (verified at roadmap time).** Cargo workspace members `aarch64`, `x86_64`, `port`, `riscv64`, `xtask`; other top-level items `lib/` (target JSON specs), `docs/`, `flake.nix`, `UNSAFE-AUDIT.md`, `HowItWorks.md`, toolchain `nightly-2026-01-30`. There is no `crypto/`, `ds/`, `kern/`, `sys/`, or `venti/` directory, and no `vendor/`. Landing-site paths in the source documents (`port/src/chan.rs`, `port/src/fid.rs`, `x86_64/src/mmu.rs`, `kern/src/gpu.rs`, `kern/src/notes.rs`, `sys/src/...`, `venti/src/index.rs`) are therefore new files or crates and must be re-verified when each phase is planned. Kernel code lives in `x86_64/`, `aarch64/` and `port/`.

**xtask at HEAD.** `xtask/src/{main,config,lib,pcap,uregcheck}.rs` plus `bin/r9-netdev-args.rs`; subcommands `build`, `expand`, `kasm`, `dist`, `test`, `clippy`, `check`, `qemu`, `boot-test`, `ureg-check`, `initrd`, `clean`. No `test-9p`, no `elf2aout`, and no `q35lowmem` or `q35crypt` profile (the existing `q35lowmcfg` is a moved-MCFG machine, unrelated). Nothing in r9's xtask or x86_64 configs asserts `rio`.

**The Harvey seam today.** `R9_HARVEY_DIR` is required by `boot-test`, `ureg-check`, `initrd` and the pcap netaddr seam; unset or empty is a named hard error, with no `../harvey` default (a deliberate earlier decision, "Phase 5 D-10"). The q35shell config pulls its root image from `module = "harvey:build/x86_64/out/bootfs-chain.paq"`, which Harvey's bake produces. Harvey's own flake owns the composed gates (`boot-r9-x64` smokes, net, auth, drawterm gate, rio replay) and consumes r9's `kernel-x64`, `iso-builder` and `netdev-args` packages. The sibling checkout is `/Users/neilskinner/code/harvey`.

**Safety boundary at HEAD versus the guide.** `port/` is not crate-level `forbid(unsafe_code)`: it uses per-module forbids with three named floor modules (`globalalloc`, `mcslock`, `devip_waker`). The workspace lint `undocumented_unsafe_blocks = deny` is opted into by `aarch64` and `port` only; `x86_64` is not opted in. `UNSAFE-AUDIT.md` covers aarch64 and port. See open note N2.

**Project character** (IG section 1, AC abstract). The network file protocol is the OS fabric; the syscall list stays closed; `a.out` is the sole binary format; hardware acceleration appears only as synthetic 9P file trees with no `ioctl` extensions.

**Source documents** (all four in `.planning/`, classified SPEC, none locked). Precedence approved by the owner, lower wins: 0 implementation guide (IG) > 1 architecture compendium (AC) > 2 RFC > 3 permissions and test plan (PT). Per-type intel is in `.planning/intel/`; the conflict report is `.planning/INGEST-CONFLICTS.md`. `.planning/legacy/` is frozen Harvey planning history and is not live state.

## Constraints

- **Tech stack**: Pure Rust, `no_std` kernel, pinned `nightly-2026-01-30`, targets `x86_64-unknown-none` (primary), `aarch64-unknown-none`; Harvey C is the userland bootstrap only until Track P retires it
- **Runtime**: QEMU q35 x86_64 primary; aarch64 virt under HVF and TCG; the golden invariant is checked on q35
- **Golden invariant**: boots to rio on QEMU q35 at every commit; Track P replacement is one binary per commit (IG-V02 risk 4)
- **Compatibility**: `a.out` only; closed syscall list; crypto extensions add four `rfork` flag bits (`RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21) and zero syscalls — the guide's "3 flag bits" wording is superseded by the owner's 2026-11-22 decision
- **Safety**: `port/` and `crypto/` forbid `unsafe`; low-level operations (`WRPKRU`, `MOVNTDQ`, `CLFLUSHOPT`, inline asm, raw atomics) stay in `x86_64/`, `aarch64/`, and `ds/src/tpht/` behind audited `// SAFETY:` blocks (IG-D07; the `ds/` wording is open question W5)
- **Kernel FP/SIMD**: kernels are built without FP/SIMD; kernel SIMD goes through `fpu::kernel_begin()` / `with_simd()`; user FP state is per process; arm64 is the reference for x86_64 (owner rule)
- **Verification discipline**: every phase follows committed RED pin, implementation, GREEN verification twice, pin advance (IG-V01)
- **Audit ledger**: the guide's 17 corrections must stay permanently corrected in all later PRs (IG-A01)
- **Process (owner standing rules)**: pushes in either repo need explicit owner approval; edits under `port/src` need owner approval (show the diff first); `aarch64/`, `x86_64/`, `xtask/` edits do not; an x86_64 kernel byte change needs the `boot-iso` `outputHash` re-derived and committed, the x64 TCG gates in an amd64 Docker container (cleaned up afterwards), and the owner's linux boot-iso check; an aarch64 kernel byte change gets a quick pass now and the full HVF series at Milestone S
- **Commits**: no AI attribution in commit messages or PR bodies, anywhere

## Open Questions

Ten of the eleven ingest warnings remain open, approved by the owner to be carried and settled in the phase they affect when that phase is planned. They are not resolved here. W11 was decided on 2026-11-22 (Key Decisions below). Full detail of each is in `.planning/INGEST-CONFLICTS.md`.

| ID | Question | Settle in |
|----|----------|-----------|
| W3 | MAC primitive: Keyed BLAKE3 only, or define where Poly1305 applies (RFC says BLAKE3; IG says "BLAKE3 / Poly1305"; Poly1305 tag is 16 bytes, field is 32) | Phase 3 (guide Phase 1, crypto); also read by Phase 4 wire |
| W5 | `unsafe` in `ds/src/tpht/` versus `forbid(unsafe_code)` on `ds/` | Phase 3 (guide Phase 1, ds) |
| W6 | TPHT home block capacity: 4 slots (IG 5.3) versus 16 slots (IG 5.2, AC) | Phase 3 (guide Phase 1, ds) |
| W1 | Wire header layout and AAD contents: are `Pad` and `ExtTLVs Length` authenticated; AC listing 9.9 header | Phase 4 (guide Phase 2, wire) |
| W2 | `FidHandle` is "exactly 8 bytes" on the wire but `StatelessCap` is 24 bytes: on-wire form | Phase 4 (guide Phase 2, wire) |
| W4 | Session key scope: per FID (RFC derivation) versus per connection and direction (IG, AC) | Phase 4 (guide Phase 2, wire) |
| W7 | `DiscoveryBeaconV2` stated 192 bytes, fields sum to 202 | Phase 6 (guide Phase 4, discovery) |
| W8 | `/adm/users` UID 0 is `adm`, persona table says `glenda`; `glenda` has no entry | Phase 13 (permission suites, PERM) |
| W9 | Test 3.2 needs `alice` absent from `/adm/users` but she is present | Phase 13 (permission suites, PERM) |
| W10 | Suite 3 mounts `venti03` from `cpu01` across VLANs 20 and 10 with no route; `archive.r9` server unnamed | Phase 13 (permission suites, PERM) |

**Roadmapper notes (new; not part of the owner-approved eleven; raise at the named phase).**

- **N1** (Phase 3, Phase 4): the RFC specifies ChaCha20 "with 64-bit block counters and 96-bit nonces" (RFC section 3.1). ChaCha20's state has 128 bits for counter plus nonce; 64 plus 96 is 160. RFC 8439, which the RFC cites, uses a 32-bit counter with a 96-bit nonce, and the wire carries an 8-byte sequence nonce while AC `TagEntry` carries a 12-byte nonce. The exact counter/nonce construction needs fixing before ChaCha20 vectors are pinned.
- **N2** (Phase 3, Phase 5): IG-D07 and IG-V01 describe `port/` as crate-level forbid, `undocumented_unsafe_blocks = deny` on arch crates, and an audit covering `x86_64/src/shadow.rs` and `cryptpage.rs`. HEAD differs: per-module forbids with three floor modules, `x86_64` not lint-opted-in, audit scoped to aarch64 and port. Decide how to extend lint and audit scope when new unsafe lands.
- **N3** (Phase 2, informational): AC section 1.1 says bits 18-20 are already reserved in Harvey `sys/include/libc.h` and `x86_64/src/proc.rs`. That is not true at HEAD. The highest `rfork` bit in use is 14 (`RFNOMNT`, Harvey `libc.h:624`); r9 defines bits 0-6 and 10-13 (`x86_64/src/proc.rs:171-181`, `aarch64/src/syscall.rs:92-108`); nothing in either repo uses bits 15-21. Phase 2 therefore reserves all four bits (18, 19, 20, 21) in r9 only, in `x86_64/src/proc.rs` and `aarch64/src/syscall.rs`. Harvey is not edited: the owner's 2026-11-22 decision overrides the guide's "reserve bits 18-20 in `libc.h`" and is recorded as a deviation from the guide. Nothing in Harvey's C userland uses the new bits, so no Harvey-side definition is needed.
- **N4** (Phase 1, Phase 16): the "boots to rio" proof and the other composed gates live in Harvey's flake, not in r9's xtask. Owner direction 2026-11-22 (HARN-08): Phase 1 flips flake ownership, so r9's flake imports Harvey and produces the composed boot and gate outputs, and Harvey's flake drops its `r9` input. Phase 16 then only has to run the gates with no Harvey tree at all. The default-path question is settled by owner direction 2026-11-22 (HARN-07): the r9 flake pins Harvey and the devshell exports the read-only tree location; xtask keeps no built-in default. **Resolved in Phase 1:** the pre-built artifacts enter the read-only tree through the `harvey-tree-{arm64,amd64}` fixed-output derivations that bake Harvey's own build tree; the flip is pushed at Harvey `c697666` and every moved gate passes from r9.
- **N5** (Phase 4, Phase 6): `Tauth` needs an authenticated key exchange in `factotum` (RFC allows "Noise handshake or mutual DP9IK", unresolved). `factotum` is Harvey C, read-only; its Rust port is Track P (Phase 12). Either use session secrets from the unmodified Harvey factotum or use the Phase 1 harness to swap an r9-owned factotum in earlier.
- **N6** (Phase 13): `chat-rs`, which PT Suite 5 depends on, is defined in no ingested document. Define it or move Suite 5 to deferred.
- **N7** (Phase 5): the guide's restricted vector ABI (own `ymm0`-`ymm5` spill, zero, restore) must be reconciled with the owner's rule that kernel SIMD goes through `fpu::kernel_begin()` / `with_simd()`.
- **N8** (Phase 7): `RFRANDMEM` is named but its semantics are not defined in any source document.

## Key Decisions

No ADR exists and nothing is locked in this ingest. The guide's "non-negotiable" decisions (IG-D01 to IG-D13) are carried as constraints, not locked; whether to lock them is confirmed in Phase 2 (DSGN-05).

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Planning lives in `r9/.planning/`; Harvey planning frozen, copy in `.planning/legacy/` | Harvey becomes a read-only reference; r9 is the product | Done (owner, pre-roadmap) |
| Source precedence IG > AC > RFC > PT | Owner-approved manifest | Decided |
| Golden invariant: boots to rio on QEMU q35 at every commit | Lets Track P replace ~1.2M lines of C one binary at a time without a flag day | Decided |
| Phase order from the owner's migration plan: Stage P0 first, then guide Phases 0-5, Milestone S, Milestone STD, Track P (Stages P1-P3), guide Phases 6-8, Harvey retirement | Owner ordering overrides inferred ordering | Decided |
| `libstd_r9` precedes Track P | Avoids rewriting Harvey userland twice (first `no_std`, then idiomatic) (IG section 9.1) | Carried from IG-D04; confirm in Phase 2 |
| `RFNOLEGACY` is a fourth `rfork` flag, `1 << 21`; its bit is assigned in Phase 0 (GSD Phase 2) together with bits 18-20 (`RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20) | Owner decision 2026-11-22, resolving warning W11; the exit gate "flag bits reserved" covers four flags; no conflict (highest bit in use is 14); the four bits are reserved in r9 only (`x86_64/src/proc.rs`, `aarch64/src/syscall.rs`) and Harvey is not changed, a recorded deviation from the guide's "reserve bits 18-20 in `libc.h`"; Phase 4 (GSD Phase 6) implements the behaviour against the reserved bit | Decided 2026-11-22 |
| Succinct TPHT is primary concurrent hashing; Roaring is the set algebra; Funnel is scratchpad only | IG-D09, IG-D10 | Carried constraint; confirm in Phase 2 |
| Native `9P2000.aead` is the distributed wire security | IG-D11; details open at W1-W4 | Carried constraint; confirm in Phase 2 |
| Dual C runtime: classic APE via ported 6c, APEX via Clang, never mixed | IG-D06 | Carried constraint; confirm in Phase 2 |
| Build priority: `ds/` and `9P2000.aead` upfront (guide Phases 1 and 2, which are GSD Phases 3 and 4) | IG-D01: Plan 9 is a network file fabric | Carried constraint; confirm in Phase 2 |
| Eleven ingest warnings: ten carried as open questions, W11 decided | Owner approved carrying ten; decided W11 on 2026-11-22 | Ten pending |

---
*Last updated: 2026-11-23 after Phase 1 completion (cross-build harness; HARN-01…08 validated)*
