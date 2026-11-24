# Phase 4: Guide Phase 2 - 9P2000.aead Transport and FidTable - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-24
**Phase:** 4-Guide Phase 2 - 9P2000.aead Transport and FidTable
**Areas discussed:** Header bytes and AAD (W1), StatelessCap on the wire (W2), Session keys and Tauth (W4, N5), Kernel reach and landing sites

---

## Header bytes and AAD (W1)

### What does the wire MAC cover?

| Option | Description | Selected |
|--------|-------------|----------|
| Every byte before the MAC (Recommended) | Whole header (28 bytes; first written as 24 in error) including Pad and ExtTLVs Length, then TLVs, then ciphertext, as one contiguous slice | ✓ |
| RFC formula as written | Only the seven listed fields; Pad and ExtTLVs Length unauthenticated | |
| All fields, Pad must be zero | As the first, plus non-zero Pad rejected | |

**User's choice:** Every byte before the MAC

### How is the 8-byte sequence nonce extended to 12 bytes?

| Option | Description | Selected |
|--------|-------------|----------|
| Direction word + sequence (Recommended) | 4-byte direction constant then the sequence nonce | |
| Four zero bytes + sequence | Uniqueness rests on per-direction keys | |
| Derived per-connection prefix | 4 fixed bytes from the Tauth key derivation, per connection and direction, not on the wire | ✓ |

**User's choice:** Derived per-connection prefix

### How do data messages meet the 4KB-aligned rule?

| Option | Description | Selected |
|--------|-------------|----------|
| Whole pages, params separate (Recommended) | Parameter block then data in whole 4096-byte pages; no packet-offset rule | ✓ |
| Data at a 4096-byte packet offset | Header, TLVs and parameters padded so data starts 4096 bytes in | |
| Leave to research | Researcher proposes against the Phase 5 DMA design | |

**User's choice:** Whole pages, params separate

### Where is the settled layout recorded?

| Option | Description | Selected |
|--------|-------------|----------|
| Amend RFC, record AC as corrected (Recommended) | As Phase 3 D-08; compendium not edited | |
| Also edit the compendium listing | Listing 9.9 rewritten in place with a dated note | ✓ |

**User's choice:** Also edit the compendium listing

---

## StatelessCap on the wire (W2)

### How does a 24-byte capability travel in an 8-byte field?

| Option | Description | Selected |
|--------|-------------|----------|
| EXT_CAP_TOKEN TLV carries it (Recommended) | Capability rides in the existing 32-byte token TLV | |
| New dedicated TLV for the capability | Sixth ExtMask bit; EXT_CAP_TOKEN stays the per-user token | ✓ |
| Per-connection indirection table | 8-byte index, server remembers the capability | |
| Widen the header handle to 24 bytes | 40-byte header | |

**User's choice:** New dedicated TLV for the capability

### What goes in the header field of a stateless packet?

| Option | Description | Selected |
|--------|-------------|----------|
| ExtMask bit decides, field is zero (Recommended) | Field must be all zero; stateful handle is never zero | ✓ |
| Field repeats qid_path | Object visible in clear | |
| Field carries the cap's 8-byte tag | Capability split across header and TLV | |

**User's choice:** ExtMask bit decides, field is zero

### Where does the domain secret come from in this phase?

| Option | Description | Selected |
|--------|-------------|----------|
| Caller-supplied key, tests only (Recommended) | API parameter; provisioning is Phase 6 | ✓ |
| Random per node at boot | Needs kernel entropy now | |
| Derived from the Tauth session secret | Valid only on its own connection | |

**User's choice:** Caller-supplied key, tests only

### Keep the 8-byte capability tag?

| Option | Description | Selected |
|--------|-------------|----------|
| Keep 8 bytes, 24-byte cap (Recommended) | As every source states | ✓ |
| Widen to 16 bytes, 32-byte cap | Changes WIRE-07 and the sources | |

**User's choice:** Keep 8 bytes, 24-byte cap

---

## Session keys and Tauth (W4, N5)

### Scope of the transient keys

| Option | Description | Selected |
|--------|-------------|----------|
| Per connection, per direction (Recommended) | FID bound by the MAC; zeroized on connection teardown | ✓ |
| Per FID under connection keys | Sub-key per FID, dropped on Tclunk | |

**User's choice:** Per connection, per direction

### Source of the session secret

| Option | Description | Selected |
|--------|-------------|----------|
| Injected secret behind a seam (Recommended) | Trait supplies a 32-byte secret; tests inject it | ✓ |
| Harvey factotum's existing secret | Reuse the unmodified factotum conversation | |
| Noise handshake in the kernel now | New primitive, larger phase | |

**User's choice:** Injected secret behind a seam

### Messages before keys exist

| Option | Description | Selected |
|--------|-------------|----------|
| Clear handshake, bound into the keys (Recommended) | Handshake hash mixed into the derivation; sealed from Tattach | ✓ |
| Clear handshake, not bound | Keys depend on the secret only | |
| Only Tversion in clear | Tauth already in the envelope under a fixed public key | |

**User's choice:** Clear handshake, bound into the keys

### Re-key mechanism

| Option | Description | Selected |
|--------|-------------|----------|
| Hash ratchet at a threshold (Recommended) | HKDF of current keys, marker packet | |
| Fresh Tauth exchange in band | New exchange inside the sealed channel | ✓ |
| Refuse at the limit for now | Close at the limit; re-key deferred | |

**User's choice:** Fresh Tauth exchange in band
**Notes:** With the injected seam, the re-key calls the seam again; tests supply the second secret.

### Re-key trigger

| Option | Description | Selected |
|--------|-------------|----------|
| Counter threshold, either side (Recommended) | Fixed threshold constant; close rather than wrap | ✓ |
| Client only, at a threshold | Tauth stays a T-message | |
| Threshold or elapsed time | Adds a clock and a second constant | |

**User's choice:** Counter threshold, either side

### Cutover for in-flight requests

| Option | Description | Selected |
|--------|-------------|----------|
| Per-direction switch at a marker (Recommended) | Sealed marker, then new keys | ✓ (as amended) |
| Drain, then switch | Stall until replies drain | |
| You decide | Leave to research | |

**User's choice:** Free text: liked the per-direction marker, asked how an old-key packet arriving after the marker is handled, and proposed a very limited grace period with both key sets valid until a "key change complete" acknowledgement, with a timeout refusing old-key packets even if the acknowledgement never comes.
**Notes:** Claude answered that the strict next-in-sequence rule over an ordered stream means an old-key packet after the marker can only be a replay or injection, that the directions switch independently, and that sender-side ordering is kept by a per-direction lock. Two forms were offered: (1) strict cutover with acknowledgement and timeout, no overlap; (2) the overlap window as proposed. The owner chose 1.

---

## Kernel reach and landing sites

### How far into the running kernel?

| Option | Description | Selected |
|--------|-------------|----------|
| Host-proven engine, version hook only (Recommended) | Engine on the host; devmnt gains the selection point | |
| Host only, no kernel change | Everything outside the kernels | ✓ (later superseded) |
| Live sealed mount in the kernel | Real sealed session in a booted r9 with a test key | |

**User's choice:** Host only, no kernel change

### Where does the code live?

| Option | Description | Selected |
|--------|-------------|----------|
| New workspace crate plus crypto/ (Recommended) | New no_std crate; port/src only for the hook | |
| port/src/chan.rs and fid.rs as the guide says | Inside the port crate | ✓ |
| Split: FidTable in ds/, wire in port/src | | |

**User's choice:** port/src/chan.rs and fid.rs as the guide says

### Reconciling the two answers

| Option | Description | Selected |
|--------|-------------|----------|
| port/src, measure bytes, stop if changed (Recommended) | Stop for the owner if loaded bytes change | |
| port/src behind a cargo feature | Kernel builds exclude the modules | |
| port/src and accept the byte change | Run the full x86_64 byte-change procedure | ✓ |
| Separate crate after all | | |

**User's choice:** port/src and accept the byte change

### devmnt version hook

| Option | Description | Selected |
|--------|-------------|----------|
| Hook in devmnt, plain offer by default (Recommended) | Branch in negotiate; plain path pinned unchanged | ✓ |
| devmnt untouched | Selection tested against a model | |

**User's choice:** Hook in devmnt, plain offer by default

### Owner diff review of port/src

| Option | Description | Selected |
|--------|-------------|----------|
| One review per plan (Recommended) | | |
| One review per commit | | |
| New files free, existing files reviewed | | |

**User's choice:** Free text: "the tests serve as inter-plan validation, so reserve code-review for phase end"
**Notes:** Claude's reading, stated to the owner: the diff-first stop is waived for this phase only, plans commit when tests are green, and the whole `port/src` change is reviewed at the phase-end code review before any push.

### Where is the wire MAC verified (N11)?

| Option | Description | Selected |
|--------|-------------|----------|
| At receipt, before parsing; rest to Phase 5 (Recommended) | N11 half-settled | ✓ |
| Settle N11 fully now | Two checks under two keys | |

**User's choice:** At receipt, before parsing; rest to Phase 5

---

## After research (2026-11-24)

The researcher reported contradictions with CONTEXT.md. The owner first asked whether the header is the part that must fit a 12-byte nonce plus an 8-byte truncated per-connection MAC; Claude explained that the header carries only the 8-byte sequence counter, the 12-byte ChaCha20 nonce is built locally from a derived 4-byte prefix plus that counter, and the 8-byte truncated tag belongs to `StatelessCap` in its own TLV. The owner then ruled:

| Question | Options | Owner's choice |
|----------|---------|----------------|
| Header size (CONTEXT said 24, fields sum to 28) | Field list stands, 28 bytes (Recommended) / Trim to 24 | Keep the field list, 28 bytes |
| Qid volatility class collides with QTEXCL, QTAPPEND, QTDIR | Separate class byte (Recommended) / In qtype for aead only / Renumber | Its own byte |
| Tauth exchange bytes | Standard afid reads and writes (Recommended) / Bytes inside Tauth and Rauth | Standard reads and writes on the auth fid |
| TCG gate venue | Linux host as in Phase 2 (Recommended) / amd64 container | The x86_64 Linux host |

Applied without asking, as plain corrections: the codec covers `Rerror` and `Tflush` / `Rflush`; the stream data field is on `Rstream`; Phase 2's ".text identical" stop rule is not reused.

---

## Claude's Discretion

- Sixth `ExtMask` bit value and name; TLV record format
- Session derivation `info` layout and the exchange seam trait
- Re-key threshold, acknowledgement timeout, clock source, marker and acknowledgement encodings
- `StatelessCap` tag coverage and `epoch` meaning
- New message body encodings, `FidShard` shape, `test-9p` layout, module layout, order of work

## Not decided

- Locking IG-D11 now that W1, W2 and W4 are settled: to be put to the owner as a plan checkpoint.

## Deferred Ideas

- Overlap window during re-key, for any future unordered or multi-path transport
- Real `Tauth` key exchange; Noise versus DP9IK
- Provisioning the `StatelessCap` domain secret (Phase 6)
- A live sealed mount in a booted kernel
- The page-level half of N11 (Phase 5)
