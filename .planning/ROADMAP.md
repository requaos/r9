# Roadmap: r9

## Overview

r9 is already a Rust kernel that boots Harvey's C userland to a shell, CPU server and rio on QEMU q35. This roadmap takes it from there to a self-hosting system with Harvey archived. It starts with a cross-build harness (Stage P0) so that every later change keeps the golden invariant: the OS boots to rio on QEMU q35 at every commit. It then builds the guide's foundations (design lock, `crypto/` and `ds/`, the `9P2000.aead` wire, cryptographic memory, distributed extensions and discovery, process lifecycle), moves to multi-CPU SMP, then the native `std` (`libstd_r9`) so that the Harvey userland port (Track P, Stages P1 to P3) is written once, in idiomatic Rust. Storage, graphics and the self-hosting toolchain follow, and the last phase archives Harvey against the owner's deprecation checklist.

Phase order is the owner's migration plan and overrides any order inferred from the guide. GSD phase numbers are sequential integers; each phase title keeps the source document's own label ("Stage P0", "Guide Phase N", "Milestone S", "Milestone STD", "Stage P1" to "Stage P3") so it traces back. In this file "Milestone S" and "Milestone STD" are source labels for Phases 8 and 9, not roadmap milestones; roadmap milestones are the versioned groups below.

| GSD phase | Source label | | GSD phase | Source label |
|-----------|--------------|-|-----------|--------------|
| 1 | Stage P0 | | 9 | Milestone STD |
| 2 | Guide Phase 0 | | 10 | Stage P1 |
| 3 | Guide Phase 1 | | 11 | Stage P2 |
| 4 | Guide Phase 2 | | 12 | Stage P3 |
| 5 | Guide Phase 3 | | 13 | Guide Phase 6 |
| 6 | Guide Phase 4 | | 14 | Guide Phase 7 |
| 7 | Guide Phase 5 | | 15 | Guide Phase 8 |
| 8 | Milestone S | | 16 | Final (Harvey retirement) |

Structure note: the standard granularity band (4-6 phases) would compress this, but the owner's plan fixes the 16-phase structure. Phases 10 to 12 (Track P, roughly 1.2 million lines of C) are expected to split into many plans, and Stage P3 may need to be subdivided at planning time.

## Milestones

- **v3.0 Cross-Build Harness and Wire Foundations** - Phases 1-4 (in progress)
- **v3.1 Secure Memory and Cluster Fabric** - Phases 5-7 (planned)
- **v3.2 SMP and Native std** - Phases 8-9 (planned)
- **v3.3 Harvey Userland Port (Track P)** - Phases 10-12 (planned)
- **v3.4 Storage and Graphics** - Phases 13-14 (planned)
- **v4.0 Self-Hosting and Harvey Retirement** - Phases 15-16 (planned)

Milestone numbering continues from the closed Harvey-rooted milestone v2.1.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): planned milestone work
- Decimal phases (2.1, 2.2): urgent insertions (marked INSERTED)

### v3.0 Cross-Build Harness and Wire Foundations (In Progress)

**Milestone Goal:** The golden invariant is an r9-owned gate, Harvey is a read-only supply of C artifacts that Rust replaces one at a time, the design decisions are locked, and the `crypto/`, `ds/` and `9P2000.aead` wire foundations exist and are pinned by host tests.

- [x] **Phase 1: Stage P0 - Cross-Build Harness** - `cargo xtask` draws C artifacts from a read-only Harvey tree through a fallback manifest; the boot-to-rio gate runs per commit (completed 2026-11-23)
- [x] **Phase 2: Guide Phase 0 - Design Lock and rfork Flag Reservation** - threat model, corrected C prototypes, four `rfork` flag bits reserved in r9, maintainer confirms decisions (completed 2026-11-24)
- [ ] **Phase 3: Guide Phase 1 - crypto/ and ds/ Foundations** - HKDF, ChaCha20, BLAKE3, TPHT, Roaring, `ClientSet`, with lint contracts
- [ ] **Phase 4: Guide Phase 2 - 9P2000.aead Transport and FidTable** - wire envelope, monotonic nonces, hybrid `FidHandle`, `FidShard`, Factotum `Tauth`

#### Phase 1: Stage P0 - Cross-Build Harness
**Goal**: The owner can build and boot r9 to rio on QEMU q35 from the r9 repo with `cargo xtask`, drawing every C library object and userland binary from a pre-built, read-only Harvey tree, and can replace those C artifacts with Rust ones one at a time without ever breaking the boot.
**Depends on**: Nothing (first phase). The planning-artifact migration is already done and is not a phase.
**Requirements**: HARN-01, HARN-02, HARN-03, HARN-04, HARN-05, HARN-06, HARN-07, HARN-08
**Success Criteria** (what must be TRUE):
  1. One documented `cargo xtask` command boots r9 on QEMU q35 x86_64 to a rio session using only the r9 kernel and the pre-built Harvey tree, and exits non-zero when rio is not reached.
  2. Harvey is located through one resolver that honours the existing `R9_HARVEY_DIR` and the approved alternatives (`HARVEY_SRC`, `vendor/harvey/`), a missing tree fails with a message naming the variable, and after a full gate run the Harvey tree is byte-for-byte unmodified.
  3. The xtask Harvey fallback manifest lists every C library object and userland binary in the image per architecture, and the harness fails when the image and the manifest disagree in either direction.
  4. Flipping one manifest entry to a Rust replacement swaps that single artifact into the image, the gate still passes, and flipping it back restores the Harvey artifact.
  5. The existing x86_64 q35 boot-test profiles and the aarch64 virt quick pass still pass through the reconciled mechanism.
  6. Inside `nix develop`, with nothing set by hand, `cargo xtask` commands find the Harvey tree: the r9 flake pins Harvey and the devshell exports its read-only location, and that tree contains the pre-built artifacts the harness needs.
  7. Flake ownership is flipped: r9's flake imports Harvey and exposes the composed boot and gate outputs that Harvey's flake used to own, Harvey's flake has no `r9` input, and each moved gate passes when run from r9.
**Plans**: 34/34 plans complete — 7/7 executed in planning pass 1 (research spikes S1-S6 and the owner checkpoint, D-22); 24 pass-2 plans (01-08 to 01-31) implement HARN-01 to HARN-08, with four owner stops (A 01-12, B 01-22, C 01-27, E 01-28); plan 01-32 is the owner-directed rio de-risk iteration after the stop-A FAIL (D-30); plan 01-33 is the stop-B fix iteration; plan 01-34 is the stop-C fix iteration after the tap/capture/fpprobe failures

Plans:
**Wave 1**
- [x] 01-01-PLAN.md — S1 sandboxed build venue (tracer) on an owner-approved Mac builder
- [x] 01-03-PLAN.md — S5 Harvey's own kernel under q35 (Mac TCG legs, build/prompt recipe)
- [x] 01-04-PLAN.md — S6 Rust-to-a.out probe (amd64 static, arm64 HVF swap, x86_64 leg scripted)
- [x] 01-05-PLAN.md — S3 headless rio gate and mkimage determinism check, authored for the KVM run

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 01-02-PLAN.md — S2 case collisions through the builder, A9 lazy devshell, S4 amd64 tiers on aarch64-linux
- [x] 01-06-PLAN.md — Owner's Linux KVM witness session for S1, S3, S5, S6

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 01-07-PLAN.md — Pass-1 closing: consolidated results and the owner's decisions (D-04 after S2)

**Cross-cutting constraints:** <!-- gsd:generated -->
- Each moved driver differs from Harvey's only in paths and env wiring (recorded diff), and its run's stage list and pass counts match the latest committed Harvey-hosted evidence where one exists (HARN-08 equivalence)

*Planning pass 2 (implementation; waves continue after pass 1)*

**Wave 4**
- [x] 01-08-PLAN.md — Tracer: per-arch resolver -> smoke manifest -> image engine -> virtexec boot (HVF); full HARN-02 resolver
- [x] 01-09-PLAN.md — Rio de-risk prep: DEMO credential path, drawterm X11 password path, fixed feed, reusable witness driver (D-28)
- [x] 01-10-PLAN.md — Harvey as a normal flake input at bd7a698, A13 checks, Mac builder and pin-bump docs

**Wave 5**
- [x] 01-11-PLAN.md — Image engine hardening: two-way drift, Rust provenance, `cargo xtask image`, reference compare, `image:` scheme
- [x] 01-12-PLAN.md — OWNER STOP A: lima remote builder, rio leg re-run on KVM, x86_64 drawterm through the follow

**Wave 6**
- [x] 01-32-PLAN.md — Rio de-risk iteration 2: separate the X11 login from rio startup (OWNER STOP A2)
- [x] 01-13-PLAN.md — Flake supply for arm64: proot venue, hostbin, mkpaqfs, harvey-tree-arm64 (smoke slice), lazy devshell exports

**Wave 7**
- [x] 01-14-PLAN.md — harvey-tree-amd64 (all tiers, stage programs, fixtures), host authdomain-gen, tier-0 library table

**Wave 8**
- [x] 01-15-PLAN.md — amd64 reference bake (member lists) and the marker, chain, substrate manifests verified against it

**Wave 9**
- [x] 01-16-PLAN.md — cpu and auth images with the bake-time auth domain in r9's assembler; q35cpu on `image:cpu`
- [x] 01-17-PLAN.md — Full arm64 tree, arm64 reference and the aarch64 bootfs image; virtnet/virtip on HVF

**Wave 10**
- [x] 01-18-PLAN.md — Quick gate: `cargo xtask gate` with the rio leg and control; drawterm gate moved into r9
- [x] 01-19-PLAN.md — net, services, tier and churn images
- [x] 01-29-PLAN.md — Five x86_64 profiles (q35exec, q35churn, q35shell, q35churnshell, q35auth) on `image:`; no x86_64 profile reads a Harvey-baked image (split from 01-15)

**Wave 11**
- [x] 01-20-PLAN.md — Swap probe: ps -> Rust a.out, flip, flip back, probe removed (D-17/D-18/D-27)
- [x] 01-21-PLAN.md — aarch64 boot, smoke and tier-1 gates moved into r9; tier-1 (HVF, TCG) and smoke on the Mac

**Wave 12**
- [x] 01-22-PLAN.md — OWNER STOP B: quick gate x2 + failing direction, swap shas, nine x86_64 profiles, boot-iso FOD on KVM
- [x] 01-30-PLAN.md — aarch64 net-smoke and cpu gates moved into r9; whole aarch64 quick pass on the Mac (split from 01-21)

**Wave 13**
- [x] 01-23-PLAN.md — x86_64 smoke, tier-1, churn, services gates moved into r9

**Wave 14**
- [x] 01-24-PLAN.md — x86_64 net smoke, net soak and stress gates, net-doctor and host-echo moved into r9
- [x] 01-25-PLAN.md — CI on the flake supply; relocated Harvey-hosted r9 legs; TCG quick gate (best effort)

**Wave 15**
- [x] 01-26-PLAN.md — Harvey flip change prepared in a scratch clone (q35 own-kernel app, deletions); stop-C witness steps
- [x] 01-31-PLAN.md — x86_64 plain boot, auth gate, cpu-bringup driver and drawterm developer app moved into r9; every composed output has an r9 home (split from 01-24)

**Wave 15b — stop-B fixes**
- [x] 01-33-PLAN.md — Fix the moved driver's spawn_id registration, the kernel/reference step preconditions and the q35churnshell syscall-10 gap; OWNER STOP B2 rerun

**Wave 16**
- [x] 01-27-PLAN.md — OWNER STOP C: full moved set from r9 on KVM, Harvey diff approval, the single Harvey commit, owner push

**Wave 17b — stop-C fixes**
- [x] 01-34-PLAN.md — Fix the tap seam and full/equiv capture paths; investigate the fpprobe note-delivery failure; OWNER STOP C2 rerun

**Wave 17**
- [x] 01-28-PLAN.md — Lock-only pin bump; OWNER STOP E at the bump; phase-boundary audit (R1-R7)
**Open questions**: Note N4. At HEAD r9's xtask has no rio assertion (Harvey's flake proves rio through its drawterm gate), so the "reached rio" check is new work. Owner direction 2026-11-22: the nix flake supplies the read-only Harvey tree through the devshell (HARN-07), which settles the default-path question: the devshell exports the location and xtask keeps no built-in default. Findings at HEAD that the plan must solve: r9's `flake.nix` has no Harvey input and its devshell sets no `R9_HARVEY_DIR`; Harvey's flake takes r9 as an input (`inputs.r9`) and owns the composed outputs (about twenty apps: `boot-r9`, `boot-r9-smoke`, `boot-r9-net-smoke`, `boot-r9-tier1`, `boot-r9-cpu`, the `-x64` variants including services, net-soak, churn, stress and auth, `drawterm-gate-x64`, `net-doctor`, `drawterm`, driven by `nix/*.sh` and `build/*/*.expect` in Harvey); owner direction 2026-11-22 (HARN-08) flips this in Phase 1 so r9 imports Harvey and produces those outputs, and Harvey drops its `r9` input. The plan must order the flip so no commit leaves a lock cycle or a gate with no home, decide which scripts and expect files move to r9 and which stay in the Harvey tree and are read from it, and account for Harvey's `flake.lock` currently pinning r9 `116bcc2` (the `legacy-final` checkpoint keeps the old arrangement reproducible). Editing Harvey's `flake.nix` is the one approved Harvey change in this phase; the Harvey commit and push need owner approval; Harvey's flake exposes no package of the built tree (only `harness`, `minimega`, `drawterm`), and the built binaries (for example `amd64/bin`) are gitignored, so a plain source input is read-only but not pre-built. The plan must decide how the pre-built artifacts get into the read-only tree (a derivation in r9's flake that builds the Harvey userland from the pinned source, or another approved route), per system including aarch64-darwin, where the Harvey namespace build currently runs in the lima VM. The swap probe cannot be a port (no Rust userland exists yet); choose a throwaway stand-in. The expected edits are in `xtask/` (and possibly `flake.nix`), which need no owner approval and should change no kernel bytes; any `port/src` edit would need owner approval with the diff shown first.

#### Phase 2: Guide Phase 0 - Design Lock and rfork Flag Reservation
**Goal**: The design decisions that Phases 3 to 7 build on are confirmed by the maintainer, and the four `rfork` flag bits the crypto work needs are reserved in r9, with no change to runtime behaviour.
**Depends on**: Phase 1 (the harness gates every later commit). The guide's own dependency, "Milestone v2.1 closed", is satisfied by the `legacy-final` checkpoint.
**Requirements**: DSGN-01, DSGN-02, DSGN-03, DSGN-04, DSGN-05
**Success Criteria** (what must be TRUE):
  1. A threat model document is committed and states the in-scope protections and the out-of-scope attacks (DRAM bus probing, cold boot, kernel-context code execution).
  2. The four `rfork` flag bits are reserved as named constants in one `port/` module, `port/src/rfork.rs` (`port::rfork`), which defines every `rfork` flag once and is imported by both arch crates (owner amendment 2026-11-23, Phase 2 CONTEXT D-17): `RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20 and `RFNOLEGACY` 1<<21. The exit gate covers four flags, not three.
  3. A host test pins the four values and proves they overlap no existing `rfork` flag (the highest bit in use today is 14), and `rfork` behaviour is unchanged.
  4. The corrected C prototypes are committed inside r9 with their landing location decided against the repository layout at HEAD.
  5. The maintainer has confirmed the guide's reconciled decisions and PROJECT.md Key Decisions records which are locked.
**Plans**: 6/6 plans complete in 4 waves; four owner stops (the `port/src` diff in 02-01, the threat-model draft in 02-02, the thirteen-decision table in 02-03, and the single end-of-phase stop on the x86_64 Linux host in 02-06). Required dispatch mode: run this phase with `workflow.use_worktrees` = `false` in `.planning/config.json` (`node "$HOME/.claude/gsd-core/bin/gsd-tools.cjs" query config-set workflow.use_worktrees false`, or set `"use_worktrees": false` under `"workflow"` in `.planning/config.json` by hand), so every plan runs in sequence on the main working tree. The key is unset and defaults to true; with worktrees on, plan 02-06 stops without merging or pushing, and the continuations of 02-01 and 02-02 can stop after their owner stops.

Plans:
**Wave 1**
- [x] 02-01-PLAN.md — Tracer: one rfork flag definition in `port`, generated `sys/include/rfork.h`, value pins; OWNER STOP on the `port/src` diff
- [x] 02-02-PLAN.md — Threat model document: RED document test, draft, OWNER STOP on the draft, commit
- [x] 02-03-PLAN.md — Design lock: D-17 wording at six sites, thirteen-decision table, OWNER STOP to confirm, Key Decisions rows

**Wave 2** *(blocked on 02-01)*
- [x] 02-04-PLAN.md — Both arch crates import the flags from `port`; masks hoisted and pinned; header check and x86_64 host-triple fallback in `cargo xtask test`

**Wave 3** *(blocked on 02-04)*
- [x] 02-05-PLAN.md — Mac-side closing evidence: kernel bytes before/after (`.text` identical or stop), aarch64 quick pass, the owner's checklist

**Wave 4** *(blocked on 02-02, 02-03, 02-05)*
- [x] 02-06-PLAN.md — OWNER STOP (end of phase): branch push, x86_64 host tests, `boot-iso` `outputHash` commit, KVM quick gate once, TCG gate, full gate set; then `main` after a second approval
**Open questions**: Warning W11 is decided, not open: owner decision 2026-11-22, `RFNOLEGACY` is a fourth `rfork` flag assigned `1 << 21` in this phase; its behaviour is implemented in Phase 6 against the reserved bit. Verified at roadmap time: bits 18-20 are not defined in Harvey `libc.h` or in r9 at HEAD, despite the architecture compendium saying they are already reserved. The reservation is r9-only; Harvey is not edited. This is a recorded deviation from the guide, which says "reserve bits 18-20 in `libc.h`" (owner decision 2026-11-22). Measured in Phase 2 research: moving the flag constants into `port/src/rfork.rs` changes kernel bytes on both architectures (panic line numbers only; `.text` is identical), so the x86_64 kernel byte-change procedure is planned work in this phase and aarch64 gets the quick pass (CONTEXT D-23).

#### Phase 3: Guide Phase 1 - crypto/ and ds/ Foundations
**Goal**: The cryptographic primitives and the concurrent data structures that the wire, memory and storage layers depend on exist as pinned, host-tested crates.
**Depends on**: Phase 2
**Requirements**: CRYP-01, CRYP-02, CRYP-03, CRYP-04, DS-01, DS-02, DS-03, DS-04, DS-05, DS-06, DS-07, LINT-01
**Success Criteria** (what must be TRUE):
  1. `cargo xtask test` runs the `crypto/` pins and they pass: HKDF-SHA256 against RFC 5869 and RFC 4231 vectors, ChaCha20 against published vectors, and BLAKE3 keyed and tree modes against the official vectors.
  2. TPHT single-line hit tests pass on the host: over 90% of lookups resolve in the home block, `deref_tiny` is exactly one memory access, the Feistel key reconstruction round-trips, and the chained overflow reaches over 95% fill with no allocation failure.
  3. Roaring union, intersection and difference match a reference model on randomized inputs, and `ClientSet` uses no heap for up to four clients and upgrades beyond that.
  4. `cargo xtask check` and clippy pass with the lint contract settled for `crypto/` and `ds/`, every unsafe site carries a `// SAFETY:` comment, and `UNSAFE-AUDIT.md` lists the new sites.
**Plans**: 2/13 plans executed; 13 plans in 13 sequential waves, in this order: 03-01, 03-12, 03-02, 03-03, 03-04, 03-05, 03-06, 03-13, 03-07, 03-08, 03-09, 03-10, 03-11. One planned owner stop, the end-of-phase stop on the x86_64 Linux host in 03-11 (branch push approval, sandboxed kernel builds and byte comparison, then a second approval for `main`). Run with `workflow.use_worktrees` = `false`, so every plan runs in sequence on the main working tree.
- [x] 03-01-PLAN.md
- [ ] 03-02-PLAN.md
- [ ] 03-03-PLAN.md
- [ ] 03-04-PLAN.md
- [ ] 03-05-PLAN.md
- [ ] 03-06-PLAN.md
- [ ] 03-07-PLAN.md
- [ ] 03-08-PLAN.md
- [ ] 03-09-PLAN.md
- [ ] 03-10-PLAN.md
- [ ] 03-11-PLAN.md
- [x] 03-12-PLAN.md
- [ ] 03-13-PLAN.md
**Open questions**: W3 (MAC primitive; also read by Phase 4), W5 (`unsafe` in `ds/src/tpht/` versus `forbid(unsafe_code)` on `ds/`), W6 (TPHT home block 4 versus 16 slots). Notes N1 (ChaCha20 64-bit counter plus 96-bit nonce exceeds the 128-bit state) and N2 (HEAD's lint and unsafe layout differs from the guide). Landing sites `crypto/` and `ds/` do not exist at HEAD; add them to the workspace members.

#### Phase 4: Guide Phase 2 - 9P2000.aead Transport and FidTable
**Goal**: r9 speaks the authenticated, encrypted `9P2000.aead` dialect with replay and splicing protection and a single-read fid table, while plain 9P2000 peers keep working.
**Depends on**: Phase 3
**Requirements**: WIRE-01, WIRE-02, WIRE-03, WIRE-04, WIRE-05, WIRE-06, WIRE-07, WIRE-08, WIRE-09, WIRE-10, WIRE-11, WIRE-12, WIRE-13
**Success Criteria** (what must be TRUE):
  1. `cargo xtask test-9p` passes wire round trips for every base and new message type, `ExtMask` and TLV parsing, and the opcode registry.
  2. Altering any authenticated byte of a packet makes the receiver drop the connection and zero its channel state without parsing the packet; a replayed packet with a stale nonce is dropped; a payload spliced onto another channel fails MAC verification.
  3. `FidTable` dereference resolves a `FidEntry` in exactly one memory read, any replica holding the domain secret verifies a `StatelessCap` with zero server FID memory, and the failover sweep returns exactly `active \ reclaimed`.
  4. A `Tauth` then `Tattach` exchange derives per-connection, per-direction session keys that are zeroized on teardown, and a plain-9P2000 peer still works, so the system still boots to rio.
**Plans**: TBD
**Open questions**: W1 (header layout and AAD contents), W2 (24-byte `StatelessCap` in an 8-byte handle), W4 (session key scope); W3 also bears here. Notes N1 and N5 (`factotum` key exchange: Noise versus DP9IK, and `factotum` is read-only Harvey C until Phase 12). Landing sites `port/src/chan.rs` and `port/src/fid.rs` do not exist at HEAD (devmnt lives in `port/src/devmnt.rs`); edits under `port/src` need owner approval with the diff shown first. `cargo xtask test-9p` is new harness work.

### v3.1 Secure Memory and Cluster Fabric (Planned)

**Milestone Goal:** Memory is cryptographically protected and demand-decrypted, the cluster can share leased state and discover its own services, and the `rfork` crypto flags are live.

- [ ] **Phase 5: Guide Phase 3 - Cryptographic Memory and Zero-Copy DMA Paging** - shadow frames, dual-path stores, restricted vector ABI, DMA straight into the backing store
- [ ] **Phase 6: Guide Phase 4 - Distributed 9P Extensions and Service Discovery** - compound, streams, leases, union mounts, legacy gateway, beacon and discover with 30s/5m lifecycle
- [ ] **Phase 7: Guide Phase 5 - Process Lifecycle and Notes** - `RFCKEY`, `RFROBASE`, `RFRANDMEM` live, secure note delivery

#### Phase 5: Guide Phase 3 - Cryptographic Memory and Zero-Copy DMA Paging
**Goal**: Evicted pages are stored encrypted and authenticated and come back correctly on demand, and network 9P packets land in page frames without a kernel copy.
**Depends on**: Phase 4
**Requirements**: MEM-01, MEM-02, MEM-03, MEM-04, MEM-05, MEM-06, MEM-07, MEM-08, MEM-09, MEM-10, MEM-11, MEM-12, MEM-13
**Success Criteria** (what must be TRUE):
  1. Under the `q35lowmem` profile, which forces eviction, a demand page fault on an evicted page decrypts into a shadow frame with correct contents, and a corrupted page in the backing store is detected and the process killed under `q35crypt`.
  2. A `9P2000.aead` data packet is DMA'd straight into an uninitialized frame in the destination segment's backing store, with no kernel copy or decrypt on receive and zero memory amplification.
  3. `ymm0`-`ymm5` are restored for the interrupted context and scrubbed before return to user mode, and the scalar and vector paths produce identical output.
  4. A KVM benchmark records cycles per cold fault (scalar versus vector) and eviction cost, and the boot-to-rio gate stays green.
**Plans**: TBD
**Open questions**: Notes N2 (x86_64 is not lint-opted-in at HEAD; extend the lint and `UNSAFE-AUDIT.md` to `shadow.rs` and `cryptpage.rs`) and N7 (reconcile the restricted vector ABI with the owner's `fpu::kernel_begin()` / `with_simd()` rule). Landing sites `x86_64/src/mmu.rs`, `shadow.rs`, `cryptpage.rs` and `port/src/palloc.rs`, `segcrypt.rs` do not exist at HEAD. `q35lowmem` and `q35crypt` are new profiles (HEAD's `q35lowmcfg` is a moved-MCFG machine, unrelated). An x86_64 kernel byte change triggers the `boot-iso` hash, amd64 Docker TCG, and owner linux check procedure.

#### Phase 6: Guide Phase 4 - Distributed 9P Extensions and Service Discovery
**Goal**: Cluster nodes can run atomic compound operations, stream data with flow control, cache shared files under leases, mount unions in parallel, interoperate with legacy 9P peers, and find each other's services dynamically.
**Depends on**: Phase 5
**Requirements**: DIST-01, DIST-02, DIST-03, DIST-04, DIST-05, DIST-06, DIST-07, DIST-08, DIST-09, DIST-10, DIST-11, DIST-12
**Success Criteria** (what must be TRUE):
  1. A `Tcompound` script (walk, open, read) runs atomically and stops at the first error with the correct completed count; a `Tstream` sender never exceeds its granted credit.
  2. Three readers cache a leased file with no heap-allocated client set and generate zero network packets on repeat reads; a writer causes `Tlease_revoke` to exactly the other readers, the server blocks until acknowledgements or the 5-second timeout, and the next read sees the write.
  3. A 9front client mounts through the legacy bounce gateway, and a process created with `rfork(RFNOLEGACY)` (bit 21, reserved in Phase 2) cannot bind, mount or walk legacy-gateway channels.
  4. A beacon on one node populates `/srv/{hostname}_{class}_{instance}` on another over both IPv4 and IPv6.
  5. After 30 seconds of silence the peer's `.status` reads `unhealthy`, its `healthy_replicas` bit clears and its socket stays open; a heartbeat restores it; beyond 330 seconds its `/srv` entries are removed.
**Plans**: TBD
**Open questions**: W7 (`DiscoveryBeaconV2` stated 192 bytes, fields sum to 202); W11 is decided (the `RFNOLEGACY` bit exists, this phase implements its behaviour). Note N5 (`cmd/discover` authenticates via Factotum). Ring bypass (DIST-08) is placed here by the roadmapper because the guide's Phase 4 row does not name it. Landing sites `sys/src/cmd/beacon/`, `sys/src/cmd/discover/`, `port/src/union.rs`, `port/src/legacy_9p.rs` do not exist at HEAD. Edits under `port/src` need owner approval.

#### Phase 7: Guide Phase 5 - Process Lifecycle and Notes
**Goal**: Processes can isolate their private memory cryptographically, lock their shared code read-only in hardware, and receive notes only on a genuine stack.
**Depends on**: Phase 6
**Requirements**: PROC-01, PROC-02, PROC-03, PROC-04, PROC-05
**Success Criteria** (what must be TRUE):
  1. Two `rfork(RFPROC | RFMEM | RFCKEY)` siblings are isolated: the second process touching the first's private heap takes an MMU trap, a direct read of the frame fails the MAC check, and the kernel kills it with the note `sys: crypto integrity violation`.
  2. After `rfork(RFROBASE)`, a write into the shared base code (`SG_TEXT`) faults through the hardware protection key in about 20 cycles without touching DRAM.
  3. A note aimed at a frame that is not on a stack is refused, and stack-spoofing probes trap while frames stay within stack bounds.
  4. `RFRANDMEM` behaves according to a specification written during planning.
**Plans**: TBD
**Open questions**: Note N8 (`RFRANDMEM` semantics are defined nowhere in the sources). The PT Suite 4 assertions (4.1, 4.2) are this phase's acceptance tests. Landing site `kern/src/notes.rs` does not exist at HEAD. Edits under `port/src` need owner approval.

### v3.2 SMP and Native std (Planned)

**Milestone Goal:** The kernel runs on multiple CPUs on both architectures, and Rust programs written with ordinary `std` run on r9.

- [ ] **Phase 8: Milestone S - Multi-CPU SMP and Memory Locking** - multi-CPU scheduler, single-CPU invariants retired, deferred arm64 series and WINDOWS items closed
- [ ] **Phase 9: Milestone STD - libstd_r9 Native Standard Library** - `std` fs, net, thread, sync, process, alloc, time, env over Plan 9 syscalls

#### Phase 8: Milestone S - Multi-CPU SMP and Memory Locking
**Goal**: r9 schedules across multiple CPUs on x86_64 q35 and aarch64 virt with all kernel memory structures safe under concurrency, and the work the owner deferred to this milestone is closed.
**Depends on**: Phase 5, Phase 7 (the guide lists Phase 3 and Phase 5)
**Requirements**: SMP-01, SMP-02, SMP-03, SMP-04, SMP-05, SMP-06
**Success Criteria** (what must be TRUE):
  1. r9 boots on QEMU q35 x86_64 with multiple CPUs, all online and scheduling, reaches rio repeatedly, and an 8-way soak shows no deadlock and no triple fault.
  2. aarch64 virt boots multi-CPU under HVF and TCG, and the full arm64 HVF series passes (smoke 1753, net-smoke 36, tier-1 10, cpu 36, plus 3 TCG each).
  3. Every single-CPU spinlock invariant in the kernel has been audited and replaced with SMP-safe locking, including the allocator, page tables and segment and page-cache structures.
  4. WINDOWS #70 and #66 are fixed and #49 is closed, with the open-entry ledger kept in a live file.
**Plans**: TBD
**Open questions**: The WINDOWS ledger is frozen under `.planning/legacy/WINDOWS.md` (51 open, 4 waived, 17 fixed, 72 total); re-home open entries to a live file before updating. SMP prerequisites: x86_64 per-CPU `Mach` mapping; aarch64 per-CPU current frame, kernel stack and proc pointers; FP ownership is already per-CPU. The ~4.4 hour full HVF series is owner-run venue time. Edits under `port/src` need owner approval.

#### Phase 9: Milestone STD - libstd_r9 Native Standard Library
**Goal**: A Rust program using ordinary `std` builds for r9, runs as an `a.out` without `#![no_std]`, and uses files, networking, threads, processes, time and environment through Plan 9 primitives.
**Depends on**: Phase 8
**Requirements**: STD-01, STD-02, STD-03, STD-04, STD-05, STD-06, STD-07
**Success Criteria** (what must be TRUE):
  1. A Rust binary written with `std` builds for an r9 userland target, is converted to `a.out` by `xtask/src/elf2aout.rs`, and runs on r9 under q35.
  2. `std::fs` opens, reads, writes, stats, lists and removes files over 9P, and `std::net` connects and accepts over `/net/tcp` using both IPv4 and IPv6 dial strings.
  3. `std::thread` spawns and joins threads across CPUs, and `Mutex` contention sleeps and wakes through `rendezvous`.
  4. The `std` test suite passes on r9 under multi-CPU, with `std::process`, `env`, `time` and `alloc` covered.
**Plans**: TBD
**Open questions**: How `std` is built for the r9 target (custom target JSON with `-Zbuild-std` against the pinned nightly versus another mechanism) is a planning decision. `elf2aout` lands here, not in Phase 15, because the Phase 9 gate cannot be met without it; Phase 15 runs the same tool natively. Landing site `sys/src/libstd_r9/` does not exist at HEAD.

### v3.3 Harvey Userland Port - Track P (Planned)

**Milestone Goal:** The Harvey kernel devices, base CLI, complex userland and compilers run as idiomatic Rust, and the system boots to rio with zero Harvey C binaries.

- [ ] **Phase 10: Stage P1 - Track P Kernel Devices** - `#c #d #e #s #| #I #p` at Harvey parity, safe Rust
- [ ] **Phase 11: Stage P2 - Track P Base CLI** - `rc`, `bind`, `mount`, `cat`, `echo`, `ls` in Rust, removed from the fallback manifest
- [ ] **Phase 12: Stage P3 - Track P Complex Userland and Compilers** - `sam`, `rio`, `upas`, `factotum`, `6c`/`6a`/`6l` and the remaining userland and libraries

#### Phase 10: Stage P1 - Track P Kernel Devices
**Goal**: The kernel devices `#c`, `#d`, `#e`, `#s`, `#|`, `#I` and `#p` are complete, safe Rust at parity with Harvey, so Harvey userland needs nothing from a C device.
**Depends on**: Phase 8, Phase 9 (also the Phase 1 harness)
**Requirements**: KDEV-01, KDEV-02, KDEV-03, KDEV-04, KDEV-05
**Success Criteria** (what must be TRUE):
  1. A committed parity inventory compares Harvey's devtab with r9's devices for the seven devices, with no open gaps.
  2. Harvey's tier sweeps and the boot-to-rio gate pass with these devices, with no regression at any commit.
  3. `#I` serves the `/net` conversation interface (clone, ctl, data, listen, UDP headers) and `#p` serves the `/proc/<pid>` files that process tools read.
  4. Device logic in these seven devices contains no `unsafe`, with any floor exceptions documented.
**Plans**: TBD
**Open questions**: Several of these devices already exist in Rust under `port/src/` at HEAD (devcons, devdup, devenv, devsrv, devpipe, devip, devproc), so this phase is the parity gap, measured at planning time. Edits under `port/src` need owner approval.

#### Phase 11: Stage P2 - Track P Base CLI
**Goal**: The everyday command line (`rc`, `bind`, `mount`, `cat`, `echo`, `ls`) runs as Rust binaries and the Harvey versions leave the image.
**Depends on**: Phase 10
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-04, CLI-05, CLI-06, CLI-07
**Success Criteria** (what must be TRUE):
  1. The system boots to rio with `rc`, `bind`, `mount`, `cat`, `echo` and `ls` running as Rust binaries from the image and no Harvey copies of them present.
  2. Harvey's boot and rc scripts run unmodified under the Rust `rc`, and the Harvey tier sweeps that exercise these tools pass.
  3. The fallback manifest no longer lists the six tools, and each swap was its own commit with the golden gate green.
**Plans**: TBD
**Open questions**: None recorded.

#### Phase 12: Stage P3 - Track P Complex Userland and Compilers
**Goal**: The remaining Harvey userland, including `sam`, `rio`, `upas`, `factotum` and the `6c`, `6a`, `6l` toolchain, runs as Rust and the system boots to rio with no Harvey C binary in the image.
**Depends on**: Phase 11
**Requirements**: USER-01, USER-02, USER-03, USER-04, USER-05, USER-06
**Success Criteria** (what must be TRUE):
  1. `sam`, `rio`, `upas` and `factotum` run as Rust binaries and the golden gate reaches a rio session on q35 with the Rust `rio`.
  2. The Rust `6c`, `6a` and `6l` compile and link a defined reference C corpus into `a.out` binaries that run on r9 and match Harvey's compilers on that corpus.
  3. The fallback manifest holds no C userland binary and no library object, and the root image boots to rio with zero Harvey C binaries.
  4. Every Harvey command or library that was not ported has a recorded retirement reason.
**Plans**: TBD
**Open questions**: Stage P3 is large and is likely to need subdividing when planned. The method for proving compiler parity is set at planning time. If `factotum` is pulled forward for Phase 4 (note N5), this phase inherits its completion.

### v3.4 Storage and Graphics (Planned)

**Milestone Goal:** Content-addressed storage with snapshots and garbage collection runs on the cluster and passes the permission suites, and rio renders hardware-accelerated 3D.

- [ ] **Phase 13: Guide Phase 6 - Storage Engine and Cluster Permission Suites** - BLAKE3 Venti, Chained-TPHT score index, Fossil snapshots, GC with TRIM, permission suites on minimega
- [ ] **Phase 14: Guide Phase 7 - Graphics (#G Venus and draw(3) Host Blit)** - native `devdraw`, VirtIO-GPU Venus, host blit, hardware triangle in rio

#### Phase 13: Guide Phase 6 - Storage Engine and Cluster Permission Suites
**Goal**: Storage is content-addressed by BLAKE3 with deduplication, snapshots and garbage collection, and the cluster's permission, lease and federation behaviour is verified on a 12-node testbed.
**Depends on**: Phase 4, Phase 6, Phase 12
**Requirements**: STOR-01, STOR-02, STOR-03, STOR-04, STOR-05, STOR-06, STOR-07, PERM-01, PERM-02, PERM-03, PERM-04, PERM-05
**Success Criteria** (what must be TRUE):
  1. Writing identical content twice stores one block and reads back by score, the score is the same before and after compression, and round trips across compression tiers return identical bytes.
  2. A score lookup verifies the full 32-byte score, so a collision on the folded 64-bit hint never returns the wrong data.
  3. A Fossil snapshot sends its surviving roots to Venti, mark-and-sweep reclaims unreferenced blocks, and TRIM is issued for contiguous purged runs.
  4. PT Suites 1, 2 and 3 pass on the minimega testbed: pair and team file sharing with group leadership, lease coherence with anti-splicing, and two-domain federation with unregistered users mapped to UID 9999.
  5. PT Suite 5 (mail and chat visibility) passes, or has been explicitly deferred.
**Plans**: TBD
**Open questions**: W8 (UID 0 `adm` versus `glenda`), W9 (`alice` present but test needs her absent), W10 (Suite 3 mounts `venti03` across VLANs with no route; `archive.r9` server unnamed). Note N6 (`chat-rs` is defined in no document, so Suite 5 needs a definition or deferral). PT Suite 4 is covered in Phase 7. The `TagEntry` nonce field (AC superset) is decided at planning. Landing site `venti/` does not exist at HEAD.

#### Phase 14: Guide Phase 7 - Graphics (#G Venus and draw(3) Host Blit)
**Goal**: rio draws natively on q35 and a Vulkan program renders on the host GPU into a rio window without CPU copies.
**Depends on**: Phase 12
**Requirements**: GFX-01, GFX-02, GFX-03, GFX-04, GFX-05
**Success Criteria** (what must be TRUE):
  1. rio draws natively on q35 through the kernel `devdraw` and 2D framebuffer, with no drawterm client needed to see a window.
  2. VirtIO-GPU negotiates features 0, 3 and 4 and selects Venus capset 4, and `/dev/gpu/clone` yields a context whose `cmd`, `blob` and `fence` nodes work.
  3. Compositing two images that both have host resource ids stays in host VRAM through a host blit, and falls back to CPU memdraw otherwise.
  4. A hardware-rendered triangle appears in a rio window on QEMU.
**Plans**: TBD
**Open questions**: The `#G` QID path wording ("upper 24 bits") is governed by the formula `((ctx_id & 0xFFFFFF) << 8) | (node_type & 0xFF)`. Landing sites `kern/src/gpu.rs` and `port/src/devgpu.rs` do not exist at HEAD. Edits under `port/src` need owner approval.

### v4.0 Self-Hosting and Harvey Retirement (Planned)

**Milestone Goal:** r9 compiles itself and Harvey is archived.

- [ ] **Phase 15: Guide Phase 8 - Self-Hosting Toolchain** - rustc, rust-lld, APEX, Clang, classic APE, r9 builds r9
- [ ] **Phase 16: Final - Harvey Retirement** - deprecation checklist passes with Harvey absent, then Harvey is archived

#### Phase 15: Guide Phase 8 - Self-Hosting Toolchain
**Goal**: r9 builds its own kernel and userland on r9, with Rust, C and C++ toolchains running natively.
**Depends on**: Phase 9, Phase 12, Phase 13
**Requirements**: SELF-01, SELF-02, SELF-03, SELF-04, SELF-05, SELF-06
**Success Criteria** (what must be TRUE):
  1. `rustc` compiles a Rust program on r9, `rust-lld` links it to a static ELF, the in-tree tool wraps it into `a.out`, and it runs.
  2. Modern C++/LLVM tooling builds with Clang against the vendored `vendor/apex/` runtime and runs on r9.
  3. Native Plan 9 C software (Ghostscript as the demonstration) builds with the ported `6c`/`6a`/`6l` over classic APE, and the two C runtimes are never mixed in one binary.
  4. r9 compiles its own kernel and userland on r9, and the resulting system boots to rio.
**Plans**: TBD
**Open questions**: `vendor/apex/` does not exist at HEAD. The APEX syscall stubs are generated from `sys/src/libc/9syscall/sys.h`, a Harvey path, so the source of that header is settled at planning time.

#### Phase 16: Final - Harvey Retirement
**Goal**: Harvey is no longer needed and is archived read-only.
**Depends on**: Phase 14, Phase 15 (and transitively every earlier phase)
**Requirements**: RETIRE-01, RETIRE-02, RETIRE-03, RETIRE-04, RETIRE-05
**Success Criteria** (what must be TRUE):
  1. Every device in the devtab is safe Rust, confirmed by an inventory scan that covers more than the seven Stage P1 devices.
  2. The root image contains zero Harvey C binaries, confirmed by an empty fallback manifest and an image scan.
  3. With the Harvey tree absent, native Plan 9 C software builds with ported 6c/6l over classic APE, modern C++/LLVM builds via Clang against `vendor/apex/`, and `rustc` and `rust-lld` run natively.
  4. Every composed gate that Harvey's flake hosted (boot-r9-x64 smokes, net smoke, auth, drawterm, rio) runs from the r9 repo without Harvey, and the golden-invariant gate is green.
  5. `vendor/harvey/` is removed, the Harvey repository is archived read-only, and the r9 README carries the deprecation banner.
**Plans**: TBD
**Open questions**: Note N4 (the composed gates are the main hidden dependency). Archiving the Harvey repository and any push need explicit owner approval.

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → … → 16 (decimal insertions, if any, run between their surrounding integers)

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Stage P0 - Cross-Build Harness | v3.0 | 34/34 | Complete | 2026-11-23 |
| 2. Guide Phase 0 - Design Lock and rfork Flag Reservation | v3.0 | 6/6 | Complete | 2026-11-24 |
| 3. Guide Phase 1 - crypto/ and ds/ Foundations | v3.0 | 2/13 | In Progress | - |
| 4. Guide Phase 2 - 9P2000.aead Transport and FidTable | v3.0 | 0/TBD | Not started | - |
| 5. Guide Phase 3 - Cryptographic Memory and Zero-Copy DMA Paging | v3.1 | 0/TBD | Not started | - |
| 6. Guide Phase 4 - Distributed 9P Extensions and Service Discovery | v3.1 | 0/TBD | Not started | - |
| 7. Guide Phase 5 - Process Lifecycle and Notes | v3.1 | 0/TBD | Not started | - |
| 8. Milestone S - Multi-CPU SMP and Memory Locking | v3.2 | 0/TBD | Not started | - |
| 9. Milestone STD - libstd_r9 Native Standard Library | v3.2 | 0/TBD | Not started | - |
| 10. Stage P1 - Track P Kernel Devices | v3.3 | 0/TBD | Not started | - |
| 11. Stage P2 - Track P Base CLI | v3.3 | 0/TBD | Not started | - |
| 12. Stage P3 - Track P Complex Userland and Compilers | v3.3 | 0/TBD | Not started | - |
| 13. Guide Phase 6 - Storage Engine and Cluster Permission Suites | v3.4 | 0/TBD | Not started | - |
| 14. Guide Phase 7 - Graphics (#G Venus and draw(3) Host Blit) | v3.4 | 0/TBD | Not started | - |
| 15. Guide Phase 8 - Self-Hosting Toolchain | v4.0 | 0/TBD | Not started | - |
| 16. Final - Harvey Retirement | v4.0 | 0/TBD | Not started | - |
