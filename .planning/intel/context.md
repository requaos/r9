# Context (topic-keyed notes)

No DOC-type documents were ingested. The topics below are repository facts, source-document framing, and the roadmap tables that the SPECs define. Each carries source attribution.

## Topic: Pinned commits versus current repository state
- source: /Users/neilskinner/code/r9/.planning/r9-implementation-guide-amended-v3.md (header, section 1); /Users/neilskinner/code/r9/.planning/r9-architecture-and-code-v4.md (header, section 1.1)
- note: All four docs are dated November 20, 2026 (RFC: November 2026) and pin `requaos/r9` at commit `c787ce9` and `requaos/harvey` at `a6b7e6a9`. Recorded as context, not as a conflict.
- observed at ingest (git, not from the docs): r9 HEAD is `6ee45b5`; `c787ce9` ("fix(19-10): re-wire the boot-iso FOD hash after the zero-count-Twrite bytes") is 60 commits behind HEAD. Top-level directories at HEAD: `aarch64`, `x86_64`, `port`, `riscv64`, `xtask`, `lib`, `docs`, plus `UNSAFE-AUDIT.md`. There is no `crypto/`, `ds/`, `kern/`, `sys/` or `venti/` directory at HEAD. The docs' landing sites reference those, so they are new crates or directories to create. The architecture doc's own workspace list (section 1.1) names only `aarch64`, `x86_64`, `port`, `riscv64`, `xtask`. Landing-site paths in the docs should be re-verified against HEAD before planning.

## Topic: What the repository does at the pinned commit
- source: IG section 1; AC section 1.1
- note: `requaos/r9` at `c787ce9` is described as the active pure-Rust kernel port that boots Harvey's `amd64` userspace on QEMU `q35` to an interactive shell, CPU server and `rio`. `requaos/harvey` at `a6b7e6a9` is the 64-bit Plan 9 4th Edition derivative in C, the source for `sys/src/9k/` kernel drivers, 35 core libraries and userland binaries. AC states target JSON specs live in `lib/`, and that `RFCKEY` (bit 18), `RFROBASE` (bit 19), `RFRANDMEM` (bit 20) are already reserved in `sys/include/libc.h` and `x86_64/src/proc.rs`. [see I3]

## Topic: Document lineage and version labels
- source: IG section 1; AC header and abstract
- note: IG describes itself as the authoritative implementation specification translating the "Plan r9 Architecture Compendium (v2.1)". The AC file is titled "(v2.2)" while its abstract and body refer to "Compendium v2.1". AC states it incorporates the IG audit findings. AC claims the audit ledger is backed by 55 passing unit tests on a host toolchain. [see I8]

## Topic: Project character
- source: AC abstract; IG section 1
- note: r9 is "a faithful pure-Rust port of Bell Labs' Plan 9 and Harvey operating systems". Stated guiding principles: the network file protocol is the OS fabric; the system call list stays closed; `a.out` is the sole binary format; hardware acceleration is exposed only as synthetic 9P file trees with no `ioctl` extensions.

## Topic: Roadmap as specified
- source: IG section 8 (precedence 0, used as the base); AC section 8.1 (same table, differences noted)
- note: Order of delivery: Phase 0, 1, 2, 3, 4, 5, Milestone S, Milestone STD, Track P, Phase 6, 7, 8. Columns: scope; dependencies; verification exit gate.
  - Phase 0: threat model doc, corrected C prototypes, reserve `rfork` flag bits 18-20 in `libc.h`. Depends on: Milestone v2.1 closed (AC says None). Gate: maintainer confirms decisions; flag bits reserved.
  - Phase 1: `crypto/` and `ds/` foundations (RFC HKDF, ChaCha20, BLAKE3, `FlattenedTphtShard`, `RoaringBitmap`, `ClientSet`). Depends on: Phase 0. Gate: RFC vector pins pass; TPHT single-line hit tests pass on host toolchain.
  - Phase 2: core 9P2000.aead transport and `FidTable` (`FidHandle`, `FidShard`, `deref_tiny`, AAD envelope, monotonic nonces, Factotum `Tauth`). Depends on: Phase 1. Gate: wire encode/decode tests pass; single-read dereference verified; anti-splicing AAD pins green.
  - Phase 3: cryptographic memory and zero-copy DMA paging (`UninitializedFrame`, dual-path stores, interleaved SFENCE hiding, `ShadowFrame`, `StrideTracker`). Depends on: Phase 2. Gate: 9P packets DMA directly into `BackingStore`; demand `#PF` decrypts correctly; zero memory amplification.
  - Phase 4: distributed 9P extensions and cluster service discovery (`Tcompound`, `Tstream`, `LeaseRecord`, `UnionMountTable`, `healthy_replicas` bitmap, `DiscoveryBeaconV2`, `cmd/beacon`, `cmd/discover` with 30s/5m lifecycle and `#M` integration, legacy bounce gateway, `RFNOLEGACY`). Depends on: Phase 3. Gate: atomic compound scripts pass; multi-client lease revocation tests green; discovery beacons populate `/srv` across IPv4 and IPv6; 9front interoperability verified.
  - Phase 5: process lifecycle and notes (`rfork` flags `RFCKEY`, `RFROBASE`, `RFRANDMEM` live; stack boundary note delivery). Depends on: Phase 4. Gate: sibling isolation pins; stack spoofing traps pass.
  - Milestone S: multi-CPU SMP; retire single-CPU spinlock invariants. Depends on: Phase 3, Phase 5. Gate: multi-CPU boot on QEMU `q35`; zero contention deadlocks.
  - Milestone STD: `sys/src/libstd_r9/` mapping `std::fs`, `net`, `thread`, `process`, `sync` to Plan 9 syscalls. Depends on: Milestone S. Gate: `std` test suite passes; idiomatic Rust binaries run without `#![no_std]`.
  - Track P: complete Rust port of Harvey kernel devices, 35 libraries and userland commands in idiomatic Rust via `libstd_r9`. Depends on: Milestone S, Milestone STD. Gate: system boots without C code; all Harvey tools compiled natively using `std`.
  - Phase 6: storage engine (BLAKE3 Venti CAS, Chained-TPHT score index, Fossil GFS snapshotting, NVMe TRIM GC). Depends on: Phases 2, 4, Track P. Gate: Venti dedup green; snapshot mark-and-sweep GC verified.
  - Phase 7: graphics (2D framebuffer, kernel `devdraw`, `#G` Venus, `draw(3)` zero-copy host blit bridge). Depends on: Track P (`draw`/`rio`). Gate: hardware triangle rendered in a `rio` window on QEMU. (AC lists "`devdraw`" without "kernel".)
  - Phase 8: self-hosting Rust toolchain (`rustc`, `rust-lld`, APEX C runtime, ELF-to-`a.out`). Depends on: Track P, Milestone STD, Phase 6. Gate: r9 compiles its own kernel and userland on r9.
  - Strategic note (IG section 9.1, AC section 8.3): `std` precedes Track P to avoid rewriting about 1.2 million lines of userland twice (first `#![no_std]`, then idiomatic). Track P Stage P0 cross-build path enables one-at-a-time binary replacement.
  - The permissions and test plan has no phase or dependency mapping of its own. [see I12]

## Topic: Superseded earlier baselines
- source: AC section 1.2
- note: AC table records v1.0 baseline and v2.0 audit findings against the v2.1 target, for example 9P was deferred to Phase 6 in v1.0 and is now Phase 1 and 2; vector budget was `ymm0`-`ymm3` and is now `ymm0`-`ymm5`; page cache was a 3-tier Funnel table (11MB) and is now Flattened-TPHT; Venti index was an 8-byte folded Funnel table and is now Chained-TPHT with a 32-byte BLAKE3 check (AC claims 105.4% space efficiency); Venus was wrongly `1<<1` and is now bits 0, 3, 4 plus capset 4. These are history behind the ledger corrections, not current requirements.

## Topic: Externally cited work
- source: RFC section 13; IG section 5
- note: TPHT is cited to Tang, Mai, Kuszmaul and Conway, "Succinct and Fast Tiny Pointer Hash Tables", arXiv:2607.28892v1 (2026). Roaring to Lemire, Ssi-Yan-Kai and Kaser, arXiv:1603.06549. Open addressing to Farach-Colton et al., arXiv:2501.02305v2. The RFC header names itself RFC 9200, Standards Track, "Plan r9 Architecture Group", editor J. Doe, "Updates: 9P2000, 9P2000.u, 9P2000.e". This is an internal document, not an IETF publication.

## Topic: Out-of-scope and boundary statements
- source: AC section 2.2
- note: See `constraints.md` AC-T01. Physical DRAM bus attacks, cold boot and kernel-level code execution are explicitly out of scope; SEV-SNP or TDX is named as the remedy if bus sniffing enters the threat model.
