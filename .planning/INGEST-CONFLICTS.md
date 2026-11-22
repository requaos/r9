## Conflict Detection Report

Mode: new. Sources: four SPEC documents, precedence by owner manifest (lower wins): IG 0, AC 1, RFC 2, PT 3.
- IG = /Users/neilskinner/code/r9/.planning/r9-implementation-guide-amended-v3.md
- AC = /Users/neilskinner/code/r9/.planning/r9-architecture-and-code-v4.md
- RFC = /Users/neilskinner/code/r9/.planning/rfc-9p2000-aead.md
- PT = /Users/neilskinner/code/r9/.planning/r9-permissions-and-test-plan.md

Checks run: UNKNOWN or low-confidence classifications (none, all four are high confidence); cross-reference cycles (none, every `cross_refs` list is empty); LOCKED ADR contradictions (none, no ADRs and no locked entries); existing-context contradictions (not applicable, no existing PROJECT, REQUIREMENTS or ROADMAP); competing PRD acceptance (none, no PRDs); SPEC-versus-SPEC contradictions (listed below).

### BLOCKERS (0)

None.

### WARNINGS (11)

[WARNING] W1: 9P2000.aead wire header layout and AAD contents are inconsistent across sources
  Found: RFC section 4.1 and 4.2 define the header as Size(4) Type(1) Tag(2) Pad(1) FidHandle(8) SeqNonce(8) ExtMask(2) ExtTLVs Length(2), then TLVs. RFC section 4.3 defines AAD as `Size || Type || Tag || FidHandle || SeqNonce || ExtMask || ExtTLVs`, which omits Pad and ExtTLVs Length. IG section 6.3 and AC section 5.3 give the same AAD formula without Pad or ExtTLVs Length. AC listing 9.9 `AeadHeader` is `repr(C, packed)` with size, msg_type, tag, `fid_handle: FidHandle` (an enum, not a fixed 8 bytes), seq_nonce, ext_mask only.
  Impact: The higher-precedence sources (IG, AC) omit the fields rather than contradict them, so precedence does not decide it. Without ExtTLVs Length a receiver cannot find the ciphertext start. If Pad and ExtTLVs Length are not covered by the MAC they can be altered undetected. Phase 2 wire tests and the anti-splicing pins depend on this.
  → Owner to fix one canonical byte layout, state whether Pad and ExtTLVs Length are in AAD, and correct the AC listing 9.9 header.

[WARNING] W2: FidHandle is "exactly 8 bytes" on the wire, but StatelessCap is 24 bytes
  Found: RFC section 5 says "every FidHandle occupies exactly 8 bytes" and the envelope reserves FidHandle(8B). `StatelessCap` is 24 bytes in RFC section 5.1, IG section 6.2, AC section 5.2 and AC listing 9.6. The RFC comment says only "represented via token indirection or wire capability". `FidHandle` is a Rust enum over both variants. All four wire-level statements cannot hold at once.
  Impact: Single-read dereference, zero-server-RAM immutable files and the `Twalk` return value (Phase 2) cannot be implemented until it is clear how a 24-byte capability occupies an 8-byte header field (for example via the 32-byte `EXT_CAP_TOKEN` TLV or a session-table indirection).
  → Owner to specify the on-wire form of `StatelessCap`.

[WARNING] W3: MAC primitive is stated three different ways
  Found: RFC section 3.1 and 4.2 mandate Keyed BLAKE3 only, 32-byte MAC. IG section 6.3 labels the MAC "BLAKE3 / Poly1305" (32B). AC section 1.2 table says "Keyed BLAKE3 / Poly1305 Encrypt-then-MAC"; AC section 3.2 says "BLAKE3/Poly1305" for tag verification; AC section 5.3 says "BLAKE3". RFC normative references include RFC 8439 (ChaCha20-Poly1305) but the RFC never uses Poly1305. Poly1305 produces a 16-byte tag, not 32.
  Impact: Precedence would pick IG, but IG's own wording is "BLAKE3 / Poly1305" and does not choose. Affects `crypto/` in Phase 1 (what to implement and pin) and the wire format in Phase 2.
  → Owner to choose Keyed BLAKE3 only, or define when Poly1305 applies (for example page sealing versus wire) and fix the 32-byte field accordingly.

[WARNING] W4: Scope of the transient session key (per FID versus per connection and direction)
  Found: IG section 3 (ledger item for `crypto/src/session.rs`) and AC section 2.1 require "per-connection, per-direction keys derived during `Tauth`". RFC section 3.2 says `K_transient` is "bound specifically to the connection and direction" but defines it as `HKDF-Expand(S_session, "9P-AEAD-FID" || fid, 32)` (per FID, no direction input) and says both endpoints drop it when a channel is closed via `Tclunk` (which closes a FID in 9P).
  Impact: IG wins by default precedence, but the RFC text contradicts itself, so the intended derivation inputs and key lifetime are unclear. This determines the `crypto/src/session.rs` API, rekeying, and the sequence-nonce domain in Phase 2.
  → Owner to confirm per-connection per-direction keys, and amend the RFC derivation info string and teardown rule to match.

[WARNING] W5: Where `unsafe` is allowed in `ds/` and `crypto/` contradicts itself
  Found: IG section 2 and AC section 1.1 confine unsafe operations to `x86_64/`, `aarch64/` and `ds/src/tpht/`. IG section 11 lint contracts require `#![forbid(unsafe_code)]` on `crypto/`, `ds/` and `port/`. A crate-level forbid on `ds/` cannot coexist with unsafe code in `ds/src/tpht/`. AC listing 9.3 declares `pub unsafe fn deref_tiny` and `pub unsafe trait PodSlot` in `ds/src/tpht/flattened.rs`, and listing 9.4 uses `unsafe` in `allocate_slot`.
  Impact: Both statements are in the highest-precedence source, so precedence cannot resolve it. Phase 1 lint configuration and `UNSAFE-AUDIT.md` scope depend on the answer.
  → Owner to decide: `ds/` is not crate-level forbid (forbid per module except `tpht`), or `tpht` unsafe moves to an arch crate.

[WARNING] W6: TPHT home block capacity stated as 4 slots and as 16 slots
  Found: IG section 5.3 says a `FlattenedBlock` "exceeds its 4-slot capacity" before spilling to dereference banks. IG section 5.2 and AC section 4.2, 4.3 and listing 9.3 define 16 slots per bucket (`occupied_mask` bits 0..15, `quotients [u16;16]`, `SLOTS_PER_BLOCK = 16`, home slots 1..=16).
  Impact: Same source contradicts itself, so precedence does not apply. The spill threshold changes the hit-rate target (over 90% home-block hits) and the Phase 1 TPHT single-line hit tests.
  → Owner to confirm 16 slots (this ingest carries 16 in constraints IG-M06) or correct the layout.

[WARNING] W7: DiscoveryBeaconV2 is stated as 192 bytes but its fields sum to 202
  Found: IG section 6.4 and AC section 5.9 state a 192-byte packed beacon. AC field list and listings 9.14, 9.15 (`repr(C, packed)`): magic 6 + version 2 + nonce 8 + hostname 32 + domain 32 + res_class 16 + res_instance 16 + attributes 64 + ip_addr 16 + port 2 + addr_family 1 + _reserved 7 = 202. AC also says "8-byte aligned"; 202 is not a multiple of 8.
  Impact: `cargo xtask test-9p` decode pins (Phase 4) and interop between `cmd/beacon` and `cmd/discover` depend on the exact size.
  → Owner to correct either the total (202, or 208 if padded to 8) or the field sizes (for example reduce `attributes` or `_reserved` to reach 192).

[WARNING] W8: PT persona table and `/adm/users` disagree on UID 0 and on glenda
  Found: PT section 3.1 lists UID 0 as `glenda` (group `sys`, administrator). The `/adm/users` file in section 3.2 defines `0:adm:adm:glenda,sys` and `1:sys:sys:glenda`; there is no line for a user named `glenda`; glenda appears only as a member and as leader of `dev` and `audit`. PT test 1.1 precondition also writes the group as `pair_ab:larry:larry,curly` without a UID, unlike the stated `uid:name:leader:members` format.
  Impact: The registry that Fossil would load does not match the personas the suites rely on. Group-leader checks for `dev` and `audit` name a leader with no user entry.
  → Owner to add an explicit `glenda` entry (and decide which name owns UID 0) and align the test precondition text.

[WARNING] W9: PT test 3.2 requires alice to be absent from `/adm/users`, but she is present
  Found: PT test 3.2 asserts `fossil01` "finds no `alice` in local `/adm/users`" so her channel maps to UID 9999 `none`. PT section 3.2 `/adm/users` contains `2001:alice:alice:`, and PT section 3.1 lists alice as 2001 with group `arch-team` (which has no `/adm/users` line either).
  Impact: As written the test precondition is false, so the attenuation path would not be exercised.
  → Owner to remove alice from the lab.r9 `/adm/users` used for test 3.2 (or use a different unregistered federated user).

[WARNING] W10: PT network topology does not allow the mounts that Suite 3 performs
  Found: PT section 2.2 puts `venti01`-`venti03` on VLAN 10 only and `cpu01`-`cpu06` on VLAN 20 only; `fossil01` is the only node on both. PT test 3.1 has `cpu01` mount `/net/tcp!venti03!564` at `/n/archive`, and the topology diagram labels the Fossil-to-Venti link as VLAN 10 traffic. Also, the same test treats the Venti node as a 9P2000.aead server on port 564 and as the `archive.r9` domain.
  Impact: Test 3.1 cannot reach `venti03` from a compute node unless routing or a gateway exists that the document does not define. The federation suite cannot pass as written.
  → Owner to either put an archive-domain 9P server on VLAN 20 or add routing, and name which node serves `archive.r9`.

[WARNING] W11: "3 rfork flag bits" versus a fourth flag, `RFNOLEGACY`, with no bit assigned
  Found: IG section 2 says crypto extensions "add 3 flag bits to `rfork`"; IG Phase 0 and AC section 1.1 reserve bits 18-20 (`RFCKEY`, `RFROBASE`, `RFRANDMEM`). RFC section 10.3, IG section 6.3 and AC section 5.8 introduce `rfork(RFNOLEGACY)` as an additional flag. No source assigns its bit.
  Impact: Phase 0 exit gate is "flag bits reserved"; Phase 4 delivers `RFNOLEGACY`. Without a reserved bit the gate is incomplete and the closed-syscall statement is inaccurate.
  → Owner to assign a bit for `RFNOLEGACY` in Phase 0 (for example 21) or state that it is not an `rfork` flag.
  Resolved 2026-11-22 (owner): `RFNOLEGACY` is a fourth `rfork` flag, `1 << 21`, reserved in guide Phase 0 (GSD Phase 2) with bits 18-20, in r9 only; Harvey is not edited. See ROADMAP.md Phase 2 and REQUIREMENTS.md DSGN-03.

### INFO (13)

[INFO] I1: Cost of full XSAVE/XRSTOR differs
  Found: IG section 4.3 says about 300 CPU cycles; AC section 3.3 says over 400 cycles.
  Note: Auto-resolved, IG (precedence 0) wins: about 300 cycles. No design effect; both agree the restricted `ymm0`-`ymm5` path is about 30 cycles.

[INFO] I2: IPv6 site-local discovery group appears only in AC prose
  Found: AC section 5.9 lists `[ff05::9:9]!9999` (site-local, routed fabric) as a third multicast group. IG section 6.4 lists IPv4 `239.9.9.9!9999` and IPv6 link-local `[ff02::9:9]!9999` only. RFC section 12 (IANA) lists the same two. AC listings 9.14 and 9.15 use only the two shared groups.
  Note: Auto-resolved in favour of the IG group set (IG wins, RFC agrees, AC code agrees). The site-local group is kept in `constraints.md` AC-P03 as an AC-only extension, not adopted as a requirement. The prose line in AC section 5.9 is the only place it appears.

[INFO] I3: Phase 0 dependency and the state of the rfork bit reservation
  Found: IG Phase 0 depends on "Milestone v2.1 closed" and lists "reserve `rfork` flag bits 18-20 in `libc.h`" as a deliverable. AC Phase 0 depends on "None". AC section 1.1 also says the bits are already reserved in Harvey `sys/include/libc.h` and `x86_64/src/proc.rs`.
  Note: Auto-resolved, IG wins (dependency stays "Milestone v2.1 closed", reservation stays a Phase 0 deliverable). Whether the bits are reserved in the r9 repo at HEAD was not verified in this ingest.

[INFO] I4: The two audit ledgers differ in composition and file names though both say 17
  Found: IG section 3 and AC section 2.1 each list 17 corrections. AC splits the funnel defect into two (tombstone reuse; correlated tier hashes) and omits the `port/src/access.rs` `UserSlice` item that IG includes. AC names the XOR-fold defect under `key.rs`, the GPU files `virtio_gpu.rs` and `devgpu.rs`; IG names `crypto/src/kdf.rs`, `kern/src/gpu.rs`, `port/src/devgpu.rs`.
  Note: Auto-resolved, IG wins. `constraints.md` IG-A01 carries the IG list of 17. The AC-only detail (salted independent per-tier hashing; last tier probes 8 consecutive blocks) is Funnel-specific and Funnel is now scratchpad-only.

[INFO] I5: AC appendix listings contradict IG ledger items and the port unsafe rule
  Found: (a) AC listing 9.1 `crypto/src/key.rs` ("Tested, forbid(unsafe_code)") contains a local HMAC whose `finalize` is `key[i] ^ buffer[i % 64]`, the placeholder XOR fold that IG ledger item 1 says must be replaced by RFC 5869 HKDF in `crypto/src/kdf.rs`. (b) AC listing 9.10 `VentiTphtIndex::resolve_offset` returns an offset from the 8-byte folded key without the full 32-byte score check that IG ledger item 12 requires. (c) AC listings 9.6 and 9.8 (`port/src/fid.rs`, `port/src/union.rs`) contain `unsafe impl PodSlot` and `unsafe` transmute blocks inside `port/`, which IG section 2 forbids. (d) The listings reference types not defined in the appendix (`FlattenedBlock`, `FeistelQuotienter`, `PackedEntry`).
  Note: Auto-resolved, IG wins. Appendix listings are treated as non-normative reference sketches (constraints AC-R01); they must not be copied as-is. The listings are titled "Complete, Corrected Source Listings", which is inaccurate for these items.

[INFO] I6: AC discovery and beacon listings deviate from the stated behaviour
  Found: AC listing 9.14 advances `now_ms` by 1000 per loop pass while also calling `sleep(100)` and blocking in `read`, so its clock is not wall time and the 30s and 330s thresholds would not hold. It joins the IPv6 group and falls back to IPv4 only on error, rather than listening on both. Listing 9.15 sets IPv6 TTL 4 while prose gives only the IPv4 TTL of 2.
  Note: Auto-resolved, the prose spec (IG section 6.4, AC section 5.9: dual-stack, 30s then 330s total) governs. Listings are non-normative.

[INFO] I7: Docs pin older commits than the repository HEAD
  Found: Docs pin r9 `c787ce9` and Harvey `a6b7e6a9`. r9 HEAD is `6ee45b5`, 60 commits after `c787ce9`.
  Note: Recorded as context, not a conflict, per instruction. See `context.md`. HEAD has no `crypto/`, `ds/`, `kern/`, `sys/`, `venti/` directories.

[INFO] I8: Compendium version label differs
  Found: AC file title says "(v2.2)"; its abstract and body say "v2.1"; IG says it translates "Compendium (v2.1)". The file is named `v4`.
  Note: No design effect. Recorded in `context.md`.

[INFO] I9: Stream credit width in EXT_STREAM_PRIO
  Found: AC section 5.3 says priority tier plus "32-bit stream credit (WINDOW_UPDATE)". RFC section 4.4 encodes `priority_tier` 2 bits plus `flow_credit` 30 bits.
  Note: Read as compatible (a single 32-bit field split 2 + 30). AC gives no bit layout, so the RFC layout is the only precise definition and is carried in RFC-P04. If AC intends 32 bits of credit plus a separate tier field, the RFC layout would need to change.

[INFO] I10: Message opcode set in AC listing is partial
  Found: AC listing 9.9 `MsgType` omits `Tcreate`, `Tremove`, `Tstat`, `Twstat` (opcodes 114/115, 122-127), which RFC section 6.1 defines. Opcode numbers present agree with the RFC.
  Note: Additive, not a contradiction. RFC registry is carried in RFC-P06.

[INFO] I11: Minor naming and wording differences
  Found: IG ledger says the `#G` context ID is packed into the "upper 24 bits of QID path", but the formula `((ctx_id & 0xFFFFFF) << 8) | (node_type & 0xFF)` places it in bits 8-31. Prose names the GPU node `blob`; AC listing 9.12 constant is `QCTX_DATA = 4`. IG/AC landing sites reference `kern/src/gpu.rs`, `kern/src/notes.rs`, `venti/src/index.rs` and `sys/src/...`, none of which exist at HEAD or in the AC workspace list.
  Note: Formula governs over the "upper 24 bits" wording. Node naming and directory placement to be settled when those crates are created.

[INFO] I12: PT has no phase mapping and incomplete executable coverage
  Found: PT defines five test suites but no phase or dependency placement. `perm_test.rc` covers Suites 1 and 2 only. `run_cluster_tests.sh` calls `/sys/src/tests/federation_test.rc`, which the document never defines. Suites 4 and 5 and Tests 2.1-2.3, 3.1-3.2 are described only as prose.
  Note: Gap, not contradiction. Suite dependencies (Fossil, Venti, `rio`, `upas`, `chat-rs`, `#G`) fall in Phase 4 through Phase 7 and Track P. `chat-rs` is not defined in any ingested document.

[INFO] I13: Nothing locked, no ADRs or PRDs in this ingest set
  Found: All four documents are SPEC with `locked: false`.
  Note: Per precedence rules there are no LOCKED decisions to protect. IG section 2 "non-negotiable" statements are carried as constraints (IG-D01 to IG-D13), not as locked decisions.
