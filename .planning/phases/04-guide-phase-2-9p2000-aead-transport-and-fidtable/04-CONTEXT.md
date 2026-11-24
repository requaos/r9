# Phase 4: Guide Phase 2 - 9P2000.aead Transport and FidTable - Context

**Gathered:** 2026-11-24
**Status:** Ready for planning

<domain>
## Phase Boundary

r9 gains the `9P2000.aead` dialect as host-proven code: the wire header, `ExtMask` and TLVs, the keyed BLAKE3 packet MAC, strict sequence nonces, `Tauth`-derived per-connection per-direction session keys with in-band re-keying, encode and decode of every base and new message type, and a TPHT-backed `FidTable` / `FidShard` with `StatefulHandle` and `StatelessCap`. `cargo xtask test-9p` runs the wire pins on the host. `devmnt` gains the version selection point but still offers plain `9P2000` by default, so a booted r9 behaves exactly as before and still boots to rio.

Requirements: WIRE-01 to WIRE-13. Open items settled here: W1, W2, W4, the nonce-extension half of N1, N5 as far as this phase needs it, and the wire half of N11.

Not in this phase: a live sealed mount in a booted kernel; a real `Tauth` key exchange or any `factotum` work (Noise versus DP9IK stays open); provisioning the `StatelessCap` domain secret to nodes (Phase 6); the legacy gateway bounce buffers and `RFNOLEGACY` behaviour (Phase 6); page sealing, DMA ingress and any page-level MAC check (Phase 5); the semantics of `Tcompound`, `Tstream`, `Tlease_revoke` and `Treclaim` beyond what WIRE-06 and WIRE-09 require; any Harvey edit.

</domain>

<decisions>
## Implementation Decisions

### Carried in (not re-asked)
- **D-01:** Keyed BLAKE3 is the only MAC; the packet MAC field is 32 bytes; "HMAC-BLAKE3" means BLAKE3 keyed mode with truncated output (Phase 3 D-05, D-07).
- **D-02:** ChaCha20 uses the RFC 8439 layout, 32-bit block counter and 96-bit nonce (Phase 3 D-06).
- **D-03:** Every requirement follows IG-V01: committed RED pin, implementation, GREEN twice, pin advance.
- **D-04:** Owner standing rules apply (no push without approval, no AI attribution in commits, Harvey read-only), except the `port/src` diff-first rule, which is changed for this phase by D-24. The compendium's appendix listings are non-normative sketches and are not copied as-is.
- **D-05:** TPHT keeps the 16-slot home block and the measured spill threshold from Phase 3; `FidTable` is built on `ds/src/tpht/` as it stands.

### Wire header and AAD (W1 settled)
- **D-06:** The header is 24 bytes in the RFC section 4.1 order: Size(4) Type(1) Tag(2) Pad(1) FidHandle(8) SeqNonce(8) ExtMask(2) ExtTLVs Length(2), then TLVs, then ciphertext, then the 32-byte MAC. All multi-byte fields are little-endian. — **Reversibility:** one-way — this is the published wire contract of the dialect; any peer built against it breaks if it changes.
- **D-07:** The MAC covers every byte of the packet before the MAC, as one contiguous slice from byte 0 of Size to the last ciphertext byte. `Pad` and `ExtTLVs Length` are therefore authenticated. The receiver verifies before it acts on any field. The RFC section 4.3 AAD formula (and the same formula in guide section 6.3 and compendium section 5.3) is superseded. — **Reversibility:** one-way — the MAC input is part of the wire contract.
- **D-08 (rest of N1 settled):** The 12-byte ChaCha20 nonce is a 4-byte prefix followed by the 8-byte wire sequence nonce. The prefix is not sent on the wire: it is derived per connection and per direction from the `Tauth` key derivation, alongside the keys (the TLS 1.3 shape). — **Reversibility:** one-way — the nonce construction is part of the wire contract.
- **D-09:** In data messages (`Twrite`, `Rread`, `Tstream`) the encrypted body is two parts: a small parameter block, then file data whose length is a whole number of 4096-byte pages. Short data is padded and the true count is in the parameters. The receiver can read header, TLVs and parameters into a control buffer and the pages straight into page frames. There is no rule about the data's offset within the packet. — **Reversibility:** costly — the Phase 5 DMA ingress path is built on this body shape.
- **D-10:** The settled layout is recorded in three places: the RFC text is amended with dated notes (sections 3.2, 4.1 to 4.4, 5 as needed); PROJECT.md gets dated Key Decisions rows naming the superseded wording and W1, W2, W4 leave the Open Questions tables in PROJECT.md and STATE.md; and compendium listing 9.9 in `.planning/r9-architecture-and-code-v4.md` is rewritten in place with a dated note (unlike Phase 3, where the compendium was left unedited).

### StatelessCap on the wire (W2 settled)
- **D-11:** The header `FidHandle` field stays 8 bytes in every packet. A `StatelessCap` travels in its own TLV under a new, sixth `ExtMask` bit (working name `EXT_STATELESS_CAP`). `EXT_CAP_TOKEN` keeps its single meaning, the 32-byte per-user token of RFC section 4.4. WIRE-02's "all five extension bits" and the RFC section 4.4 list are amended to six. — **Reversibility:** one-way — the extension bit and TLV are part of the wire contract.
- **D-12:** The new `ExtMask` bit is the discriminator between the two handle kinds. When it is set, the 8-byte header field must be all zero and the packet is rejected otherwise. A `StatefulHandle` can never be all zero because its `TinyPtr` byte is non-zero.
- **D-13:** The domain secret that signs and verifies `StatelessCap` tags is a caller-supplied `crypto::Key` parameter of the `FidTable` API. This phase proves the property in `test-9p`: two table instances given the same key verify each other's capabilities with zero FID memory. No kernel entropy source and no provisioning in this phase.
- **D-14:** The capability tag stays an 8-byte truncation of keyed BLAKE3 and `StatelessCap` stays 24 bytes, as WIRE-07 and every source state.

### Session keys and Tauth (W4 settled; N5 for this phase)
- **D-15 (W4 settled):** Transient keys are per connection and per direction. One `Tauth` yields, for each direction, a cipher key, a MAC key and the nonce prefix of D-08. The FID is bound by the MAC over the header, not by a key. There is one sequence counter per direction. Keys are zeroized on connection teardown, not on `Tclunk`. The RFC section 3.2 per-FID formula and its `Tclunk` teardown rule are amended. — **Reversibility:** costly — the `crypto/src/session.rs` API, the nonce domain and re-keying are all built on this scope.
- **D-16 (N5 for this phase):** The session secret is injected behind a seam. `Tauth` / `Rauth` carry opaque exchange bytes, a trait hands the transport a 32-byte session secret, and tests supply the secret directly. The real authenticated exchange and the Noise versus DP9IK choice are not decided here. Stated limit: no live authenticated connection between two real machines comes out of this phase.
- **D-17:** `Tversion` / `Rversion` and the `Tauth` exchange travel in plain 9P2000 framing. Everything from `Tattach` onward uses the sealed envelope, with sequence counters starting at 0. A hash of the cleartext handshake bytes is mixed into the key derivation, so an altered version string or msize gives the two ends different keys and the first sealed packet fails. — **Reversibility:** one-way — the handshake transcript binding is part of the wire contract.
- **D-18:** Re-keying (WIRE-04) is a fresh `Tauth` exchange run in band, inside the sealed channel, deriving new keys from a new session secret. Under D-16 this calls the exchange seam again and tests supply the second secret. A hash ratchet was offered and declined.
- **D-19:** Either side starts a re-key when its send counter reaches a fixed threshold constant, well below 2^64 - 1 and settable low in tests. If the counter would reach the limit with no new keys, the sender closes the connection rather than wrap.
- **D-20:** Cutover is per direction, strict, with acknowledgement and timeout:
  1. The sender's last packet under its old send keys is a sealed "keys change now" marker; its next packet uses the new keys with the counter back at 0.
  2. The receiver, on the marker, switches its receive keys for that direction and sends a "key change complete" acknowledgement, which proves it derived the same keys. The receiver zeroizes its old keys for that direction on the marker.
  3. The sender zeroizes its old keys for that direction when the acknowledgement arrives.
  4. If the acknowledgement does not arrive within a fixed timeout, the sender drops the connection and zeroizes everything.
  5. An old-key packet is never accepted after the marker, with or without the acknowledgement. There is no overlap window. Outstanding requests and replies continue across the cutover.
- **D-21:** Sealing and sending happen under one per-direction lock, so packets leave in counter order. D-20's strictness relies on this and on the ordered, reliable byte stream 9P runs over.

### Kernel reach and landing sites
- **D-22:** The code lands where the guide puts it: `port/src/chan.rs` (wire codec, `ExtMask` / TLVs, transport state), `port/src/fid.rs` (`FidHandle`, `FidEntry`, `FidTable`, `FidShard`), and `crypto/src/session.rs` (session key derivation). `port` gains its dependency on `crypto/` and `ds/` in this phase. No new workspace crate.
- **D-23:** Nothing in a booted kernel uses the new engine. `devmnt`'s `negotiate` gains the branch where an agreed `"9P2000.aead"` hands the session to the new engine; `devmnt` still offers plain `"9P2000"` unless asked. Host tests pin both branches against the real `devmnt` code, and the plain path is pinned unchanged (WIRE-12). The engine, session keys and `FidTable` are proven on the host by `test-9p`, with a client end and a server end over an in-memory link.
- **D-24:** The `port/src` diff-first stop is waived for this phase (owner: "the tests serve as inter-plan validation, so reserve code-review for phase end"). Plans commit once their tests are green; the owner reviews the whole `port/src` change at the phase-end code review, before any push. The standing rule is unchanged for other phases.
- **D-25:** The kernel byte change is accepted. The full x86_64 byte-change procedure runs as planned work in this phase: `boot-iso` `outputHash` re-derived and committed on the owner's x86_64 Linux host, the x64 TCG gates in an amd64 container, and the owner's linux boot-iso check. aarch64 gets the quick pass. "No kernel change" in this phase means no behaviour change; boot to rio is the invariant. Phase 2 D-23 to D-29 (end-of-phase stop, pushed branch as transport, owner makes the hash commit, order of steps) are the model; they were not re-asked.
- **D-26 (N11, wire half):** The packet MAC is verified under the connection's keys when the packet is received, before any field is acted on (WIRE-03). Whether Phase 5 adds a second, page-level check under page keys at fault time stays open for Phase 5; N11 is recorded as half-settled.

### Not decided in discussion
- **IG-D11 lock:** W1, W2 and W4 are now settled, which was the stated condition for locking IG-D11 (Phase 2 D-07). The lock itself was not discussed. The plan must put it to the owner as a checkpoint; it must not be locked by assumption.

### Claude's Discretion
- The bit value and final name of the sixth `ExtMask` bit, and the TLV record format (type and length encoding, ordering, 64-bit alignment, treatment of unknown bits) within WIRE-02.
- The exact `info` and label layout of the session derivation (direction labels, how the handshake hash and nonce prefix are produced), provided it is pinned by a known-answer vector; the shape of the exchange seam trait.
- The re-key threshold constant, the acknowledgement timeout value and where the transport gets its clock; the opcodes or encodings of the marker and acknowledgement.
- What the `StatelessCap` tag covers beyond its own fields, and the meaning of its `epoch` field in this phase.
- Body encodings of the four new message pairs to the extent WIRE-06 needs round trips; the `FidShard` shape and bitmap representation for the sweep (WIRE-09).
- Layout of `cargo xtask test-9p`, where the host tests sit, module layout inside `chan.rs` / `fid.rs` (including split into submodules), and the order of work and commits.
- How `port`'s per-module `forbid(unsafe_code)` and `UNSAFE-AUDIT.md` are kept true for the new modules. A new unsafe-holding module in `port/` still needs owner approval (IG-D07).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` (Phase 4 section, near line 196) — goal, four success criteria, open questions W1, W2, W4, notes N1, N5, landing-site note
- `.planning/REQUIREMENTS.md` (lines 52-66) — WIRE-01 to WIRE-13; WIRE-02 is amended by D-11
- `.planning/PROJECT.md` — Open Questions table (line 86 on), notes N1, N5, N11 (lines 100-110), Key Decisions rows for IG-D11 (line 126), W3 and the ChaCha20 layout (lines 138-139), `K_file` info (line 149)
- `.planning/STATE.md` — standing owner rules, open-question table (line 206 on)
- `.planning/INGEST-CONFLICTS.md` (lines 17-35) — full text of W1, W2, W4

### Source documents
- `.planning/rfc-9p2000-aead.md` — §3.1 (line 63), §3.2 key derivation (line 72; amended by D-15), §4.1 to §4.4 envelope, AAD and `ExtMask` (lines 94-201; amended by D-07, D-11), §5 `FidHandle`, `StatelessCap`, `FidEntry` (lines 205-280), §6 opcode registry and new messages (line 284 on), §10.1 negotiation rules (line 522), §11 security considerations (line 543)
- `.planning/r9-implementation-guide-amended-v3.md` — §2 decisions table (lines 27, 37), §3 corrections ledger row for `crypto/src/session.rs` (line 61), §6.1 Qid (line 171), §6.2 `FidTable` (line 184), §6.3 envelope (line 192), §11 verification matrix row for `cargo xtask test-9p` (line 342)
- `.planning/r9-architecture-and-code-v4.md` — §5.2 `FidTable` (line 237), §5.3 envelope (line 256), listing §9.6 (line 1057, non-normative), listing §9.9 (line 1293; rewritten in place by D-10)
- `.planning/intel/constraints.md` — IG-D11 (line 67), IG-A01 (82), IG-P01 to IG-P03 (145-158), RFC-P02 to RFC-P10 (265-305), PT-T02 (329)
- `docs/threat-model.md` — the splicing, injection and replay protections this phase is the first to implement

### Prior phase decisions
- `.planning/phases/03-guide-phase-1-crypto-and-ds-foundations/03-CONTEXT.md` — D-05 to D-08 (MAC, ChaCha20, RFC amendment pattern), D-09 to D-13 (unsafe and lint contract), D-18, D-21, D-22 (crate sourcing, kernel linking deferred to this phase)
- `.planning/phases/02-guide-phase-0-design-lock-and-rfork-flag-reservation/02-CONTEXT.md` — D-07, D-08 (IG-D11 lock deferred, lock rule), D-23 to D-29 (the x86_64 byte-change procedure as run before)

### Code at HEAD
- `port/src/devmnt.rs` — `VERSION9P` (line 124), `Mnt` (342), `frame_one` (499), `negotiate` (670); the plain path that must stay unchanged and where the D-23 branch lands
- `port/tests/devmnt_test.rs` — existing session tests over the `MntIo` seam
- `crypto/src/key.rs` — `Key`, `k_file`, `derive_labelled`, `KeyCapability`
- `crypto/src/kdf.rs`, `crypto/src/mac.rs`, `crypto/src/page.rs` — HKDF, keyed BLAKE3 and `verify_keyed`, ChaCha20
- `ds/src/tpht/mod.rs`, `ds/src/tpht/flattened.rs` — `TinyPtr`, `FlattenedTphtShard`, `deref_tiny`; `ds/src/roaring.rs` for the sweep bitmap
- `xtask/src/main.rs` — test and check legs, the `miri` step; where `test-9p` is added
- `port/Cargo.toml`, `UNSAFE-AUDIT.md`, `flake.nix` (`boot-iso` `outputHash`)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `devmnt` already separates transport from session through the `MntIo` trait, with host tests driving a session over an in-memory link; the same seam serves the D-23 pins and an in-memory client/server pair.
- `crypto/` already provides everything the envelope needs except session derivation: keyed BLAKE3 with constant-time verify, ChaCha20 with explicit nonce and counter, HKDF, labelled derivation and a zeroizing `Key`.
- `ds/` provides `FlattenedTphtShard` with `deref_tiny`, `TinyPtr` as `NonZeroU8` (which D-12 relies on), and a Roaring bitmap with and-not for the failover sweep.
- `xtask` has per-package test legs and a separate `miri` step as precedent for a dedicated `test-9p` step.

### Established Patterns
- `port/src/chan.rs`, `port/src/fid.rs` and `crypto/src/session.rs` do not exist at HEAD. `devmnt` is a 9P client only; r9 has no in-kernel 9P server, so the server side of this phase exists only under host tests.
- No kernel crate depends on `crypto/` or `ds/` yet; this phase adds the dependency through `port`.
- `port/` uses per-module `forbid(unsafe_code)` with three named floor modules; the workspace denies undocumented unsafe blocks.
- Kernel builds are reproducible and source line shifts alone change loaded bytes (panic line numbers), measured in Phase 2. Before and after must be compared on one venue.
- Build and test commands: `nix develop -c cargo xtask check`, `nix develop -c cargo xtask test`.

### Integration Points
- `port/src/lib.rs` module list; `port/Cargo.toml` dependencies; `devmnt::Mnt::negotiate`.
- `xtask/src/main.rs` for `test-9p`; `UNSAFE-AUDIT.md`; `flake.nix` `outputHash`.
- Documents edited by D-10: the RFC, PROJECT.md, STATE.md, REQUIREMENTS.md (WIRE-02), compendium listing 9.9.
- Later consumers, not touched now: Phase 5 DMA ingress (D-09 body shape, N11), Phase 6 legacy gateway, discovery, domain-secret provisioning and a live sealed mount.

### Research must confirm
- That `port` depending on `crypto/` and `ds/` builds for both kernel targets in portable form, and what the measured kernel byte change is.
- How BLAKE3 keyed mode is fed a header-plus-TLVs-plus-ciphertext slice without copying (the incremental `Blake3` API exists).
- A sound source for the D-20 timeout in host-tested, `no_std` transport code.

</code_context>

<specifics>
## Specific Ideas

- The owner chose the derived nonce prefix over the simpler fixed forms, and a fresh in-band exchange for re-keying over a hash ratchet.
- The owner chose a dedicated TLV for the capability, to keep `EXT_CAP_TOKEN` to one meaning.
- On re-key cutover the owner first proposed a short grace period in which old and new keys are both valid, ended by a "key change complete" acknowledgement, with a timeout so old-key packets are refused even if the acknowledgement never comes. After discussion the owner chose the strict form (D-20), which keeps the acknowledgement and the timeout but no overlap.
- The owner wants the code in the guide's landing sites, the byte change accepted, and code review held at phase end with tests as the validation between plans.
- The owner wants compendium listing 9.9 corrected in the document itself.

</specifics>

<deferred>
## Deferred Ideas

- **Overlap window during re-key** (old and new keys both accepted for receipt): only if the dialect ever runs over an unordered or multi-path transport. The strict next-in-sequence rule would have to change first.
- **Real `Tauth` key exchange and the Noise versus DP9IK choice:** when a factotum exists to run it (Phase 12, or earlier if factotum is pulled forward).
- **Provisioning the `StatelessCap` domain secret to nodes:** Phase 6, with discovery and replicas.
- **A live sealed mount in a booted kernel:** the phase that has a real exchange and a server to talk to.
- **The second half of N11** (a page-level check at fault time): Phase 5.

</deferred>

---

*Phase: 4-Guide Phase 2 - 9P2000.aead Transport and FidTable*
*Context gathered: 2026-11-24*
