# r9 Threat Model

This document states what r9's `9P2000.aead` transport and its process isolation are meant to
protect, against whom, and what is deliberately left unprotected.  It gathers the security
statements scattered through the design documents into one place, so that the phases that
build the protections can be checked against a single statement of intent.

**Status.** This is a living document.  Nothing in it is fixed, the scope boundary included; later
phases revise it as designs firm up.  Except where a row says otherwise, no protection described in
this document is implemented in r9's code.  At the time of writing the workspace members are
`aarch64`, `crypto`, `ds`, `port`, `riscv64`, `x86_64` and `xtask`: the six Phase 4 wire protections
of section 5 are implemented and host-tested but used by no booted kernel (Phase 4 D-23), and every
other protection below is a design intention with a delivering phase, given the status
`not implemented` in section 5.  Read "is specified to" wherever a mechanism is described.

**Citation rule.** Every statement cites the document that makes it, or is marked as new.  The tags
are `[AC n]` for a section of the v3.0 architecture compendium, `[RFC n.n]` for a section of the
`9P2000.aead` RFC, `[IG n]` for a section of the v3.0 implementation guide, and `[PT §n.n]` or
`[PT Test n.n]` for a section or a numbered test of the permissions and test plan.  The mark
**[new]** means the statement is made first in this document and has no source; those are the
statements a reviewer most needs to check.  A row that carries both a tag and **[new]** has a
sourced part and an unsourced part, and says which is which.  The documents are listed under
Sources; where the superseded v2.1/v4 documents disagree with the v3.0 documents, the v3.0
documents win.

---

## 1. Scope boundary

The memory protection boundary is hardware MMU isolation, quoted word for word from the v3.0
compendium's rescope summary [AC 1]:

> Uniform hardware MMU isolation (`CR3`/`TTBR0`/`satp` + PTE `RO`/`RW`) across `x86_64`, `aarch64`, and `riscv64`.

Per-process page tables, read-only user text and kernel/user separation are the whole of the memory
boundary; there is no software key separation between processes **[new]**.

**In scope:** prevention of packet splicing, injection, and replay over 9P network connections
(`9P2000.aead` transit security) [AC 1] [AC 5]; isolation between processes by the hardware MMU
[AC 1]; confinement of cleartext legacy links at the translation gateway, with
`rfork(RFNOLEGACY)` as the process opt-out [AC 5] [RFC 10.3].

**Removed from scope by the 2026-11-25 v3.0 rescope** [AC 1]: software protection of pages at rest
(evicted frames, backing stores, NVMe swap, Venti archives); stale-frame and shadow-frame
encryption; `ymm0`-`ymm5` register sealing; `PKEY`/`WRPKRU` page-fault hooks; and sibling-key
isolation.  The compendium's mandate table marks the v2.1 software RAM encryption baseline
"**Dropped.**" in favour of the hardware MMU boundary quoted above [AC 1].

**Out of scope:** physical hardware bus probing of live DRAM, cold-boot physical attacks, and
arbitrary code execution within the kernel context.  Hardware memory encryption (AMD SEV-SNP,
Intel TDX) is the remedy if physical DRAM bus sniffing is ever brought into the threat model.
(Carried from the superseded v4 compendium's section 2.2; the v3.0 rescope changed none of it.)
**[new]**

---

## 2. Assets

What an attacker would want to read, forge or destroy.  The sources name these things but never
list them as assets; the list as a list is **[new]**.

| Asset | Where it lives | Source |
|-------|----------------|--------|
| A process's private memory (heap, stack, data) | The process's own address space, isolated by the hardware MMU | [AC 1] |
| The session secret produced by the authenticated key exchange | The `factotum` agent at each end of a connection | [RFC 3.2] |
| The per-connection, per-direction channel keys (cipher key, MAC key, nonce prefix) | Each endpoint's memory tables, until the channel is closed and they are zeroized | [RFC 3.2] |
| The cluster (domain) secret that signs `StatelessCap` tokens | Every server replica in the cluster | [AC 1] [RFC 5.2] |
| 9P message payloads in transit | Ciphertext on the network between two nodes | [RFC 3] [AC 5] |
| 9P control headers (size, type, tag, `FidHandle`, sequence nonce, `ExtMask`, extension TLVs) | Cleartext on the wire; authenticated by the packet MAC, not hidden | [RFC 4.2] [RFC 4.3] [AC 5] |
| Capability tokens carried as `EXT_CAP_TOKEN` (32 bytes) and the revocation list they are checked against | The wire, and the server's cache-resident revocation list | [RFC 4.4] |
| `StatelessCap` tokens | Held by the client as its file handle | [AC 1] [RFC 5.2] |
| The legacy session key `K_legacy_session` and the plaintext that passes through the legacy gateway | Pre-pinned, non-pageable kernel bounce frames | [AC 5] [RFC 10.2] |
| Discovered service records `/srv/{hostname}_{class}_{instance}`, their `.status` files and the `healthy_replicas` set | `/srv` and the `#M` mount driver | [new] (was [AC 5.9] of the superseded v4 compendium) |
| User and group records (`/adm/users`) | The file server `fossil01` | [PT §3.2] |
| The server's ability to complete a write while clients hold read leases | The lease state on the server | [RFC 11] |

---

## 3. Attacker classes and capabilities

The sources name only "network adversaries" `[RFC 4.3]`, "malicious processes" and "malicious or
unresponsive clients" `[RFC 11]`, and the test persona `eve` `[PT §3.1]`.  They do not define
attacker classes or state capabilities.  The classification below, and every capability not quoted
from a source, is **[new]**.

| Class | Capabilities | In or out of scope | Source |
|-------|--------------|--------------------|--------|
| On-path network adversary | Reads every packet on a fabric between nodes; replays a captured packet; moves a valid payload to another channel by editing the cleartext header; injects packets.  Also drops, delays and reorders packets **[new]** | In scope: splicing, injection and replay | [RFC 4.3] [PT Test 2.3] [AC 5] |
| Malicious local process, including a sibling running under the same user | Runs arbitrary user-mode code; tries to read another process's private memory; aims its stack pointer outside its stack before a note is delivered; writes crafted bytes to a device file | In scope: the hardware MMU is the memory boundary.  The note and device cases are corrected defects, not boundary items **[new]** | [PT §3.1] [AC 1] [IG 2] |
| Authenticated user without permission for a file | Mounts the file server with valid credentials and tries to read or change files that belong to others | Covered by the permission suites; not named in the boundary **[new]** | [PT §3.1] [PT Test 1.1] |
| User from another authentication domain with no local record | Presents a valid ticket for the other domain and tries to read private files | Covered by the permission suites; not named in the boundary **[new]** | [PT Test 3.2] |
| Malicious or unresponsive client holding leases | Keeps read leases and withholds `Rlease_ack`, to stall a conflicting write | In scope of the RFC's security considerations; not named in the boundary **[new]** | [RFC 11] [RFC 6.2.3] |
| Cleartext legacy peer, and anyone on the path to it | Speaks `9P2000`, `9P2000.u`, `9P2000.e` or `9P2000.L` with no encryption; sees and can alter everything on that link (the path part is **[new]**) | Data on a legacy link is unprotected by design; the gateway and `RFNOLEGACY` limit what reaches it | [AC 5] [RFC 10.1] [RFC 10.3] |
| Sender of forged discovery beacons | Any host that can send to the discovery multicast groups emits a beacon with a hostname, class, address and port of its choosing, or replays one | No source treats it **[new]** | [new] (the beacon format was [AC 5.9] of the superseded v4 compendium) |
| Compromised replica, or any other holder of the cluster secret | Mints a `StatelessCap` for any immutable file and has it accepted by every replica | No source treats it **[new]** | [RFC 5.2] |
| Compromised `factotum` or authentication server | Learns or chooses the session secret from which channel keys are derived | No source treats it **[new]** | [RFC 3.2] |
| Physical attacker with bus or cold-boot access | Probes live DRAM on the memory bus; images memory after a cold boot | Out of scope | [new] (carried from [AC 2.2] of the superseded v4 compendium) |
| Attacker executing code in kernel context | Reads keys directly, because keys are held in kernel memory | Out of scope | [new] (carried from [AC 2.2] of the superseded v4 compendium) |

---

## 4. Trust boundaries

Where data or control passes between parties that do not trust each other equally.  No source has
a trust-boundary list; the list as a list is **[new]**.

| Boundary | What crosses it | Source |
|----------|-----------------|--------|
| User process and kernel | System calls from a closed list; the user stack pointer at note delivery; bytes written to device files; user buffers the kernel reads and writes | [AC 1] [IG 2] |
| Process and process | Nothing is meant to cross: each process has its own page tables, and read-only user text is shared only as read-only mappings | [AC 1] |
| Node and network | Authenticated ciphertext with cleartext headers; incoming frames are verified and decrypted on receipt into standard kernel frames, with no fault-time decryption | [RFC 4.3] [AC 5] |
| Application and network adapter through the user-space ring | Ring nodes under `#M` mapped into user space with `segattach`, bypassing the kernel file layer | [AC 5] |
| Authentication domain and authentication domain | Tickets from one domain presented to a server in another; an unregistered user is attenuated to `none` | [PT Test 3.1] [PT Test 3.2] |
| Native transport and the legacy gateway | Plaintext exists in kernel bounce frames while it is translated to or from an unencrypted peer | [AC 5] [RFC 10.2] |
| The cluster and the discovery multicast groups | Unauthenticated beacons arrive from the local subnet, link or site scope and lead to dialling and to `/srv` entries | [new] (was [AC 5.9] [IG 6.4] of the superseded v2.1/v4 documents) |
| Kernel and `factotum` | `factotum` runs the authenticated key exchange that yields the session secret; every channel key is derived from that secret | [RFC 3.2] |
| Processes sharing one pooled connection | Several processes and users on one node send over a single connection, told apart by capability tokens | [RFC 4.4] |
| Server replica and server replica | Replicas share the domain secret, so each accepts tokens the others issued | [RFC 5.2] |
| Portable kernel code and architecture code | Hardware operations (inline assembly, raw atomics, MMIO) are confined to the architecture crates behind audited comments and reached through safe interfaces | [AC 2] |
| Network fabrics | Storage traffic, grid traffic and terminal traffic are carried on separate VLANs in the test topology | [PT §2.2] |

---

## 5. Protections

Each row is a protection a source specifies, the attacker class from section 3 it addresses, and
the phase of `.planning/ROADMAP.md` that is planned to deliver it.  The phase column and the pairing
of protection with attacker are **[new]**; the sources give neither.  The rows are worded in the
present tense for brevity; each describes what a source specifies, not something r9 does.  Every
status is `not implemented` except the six Phase 4 wire protections, which are implemented and
host-tested but not yet used by a booted kernel.

| Protection, as specified | Attacker addressed | Source | Delivering phase | Status |
|------------|--------------------|--------|------------------|--------|
| Processes are isolated by hardware MMU per-process page tables (`CR3`/`TTBR0`/`satp`), and user text is mapped read-only so the evictor can drop unmodified file-backed pages | Malicious local process | [AC 1] | Phase 5 (read-only user text and dirty tracking; per-process page tables pre-exist) | not implemented |
| Frames pinned while the kernel holds user memory across a sleep are never chosen by the evictor, so they are not reused mid-operation | None; a kernel defect | **[new]** (Phase 5 D-17, D-21) | Phase 5 | not implemented |
| 9P payloads in transit are encrypted with ChaCha20 | On-path network adversary | [RFC 3.1] [AC 5] | Phase 4 | implemented and host-tested in Phase 4; not used by a booted kernel (D-23) |
| Splicing, injection and replay are rejected: size, type, tag, `FidHandle`, sequence nonce, `ExtMask` and TLVs are authenticated, and the receiver accepts only the next sequence number | On-path network adversary | [RFC 4.2] [RFC 4.3] [AC 5] [PT Test 2.3] | Phase 4 | implemented and host-tested in Phase 4; not used by a booted kernel (D-23) |
| On a MAC failure the receiver ends the connection and zeroes the channel state without processing any enclosed byte | On-path network adversary | [RFC 4.3] [RFC 11] | Phase 4 | implemented and host-tested in Phase 4; not used by a booted kernel (D-23) |
| Channel keys are dropped and zeroized when a channel is closed, so captured packets stay unreadable | Attacker who records traffic and later obtains endpoint state | [RFC 3.2] | Phase 4 | implemented and host-tested in Phase 4; not used by a booted kernel (D-23) |
| A channel re-keys before its sequence counter reaches 2^64 - 1, so a nonce is never reused under one key | On-path network adversary | [RFC 11] | Phase 4 | implemented and host-tested in Phase 4; not used by a booted kernel (D-23) |
| A `StatelessCap` is accepted only if its MAC verifies under the cluster secret | Client that forges or alters a file handle | [AC 1] [RFC 5.2] | Phase 4 | implemented and host-tested in Phase 4; not used by a booted kernel (D-23) |
| A capability token on a pooled connection is validated against a revocation list | Process on a shared connection using a revoked or forged token | [RFC 4.4] | Phase 4 | not implemented |
| After failover, a reclaimed handle is restored only when the client's capability token validates; unreclaimed handles are freed and their keys zeroized | Client replaying a stale handle | [RFC 6.2.4] [RFC 5.2] | Phase 4 and Phase 6 | not implemented |
| Legacy traffic exists as plaintext only in pre-pinned, non-pageable bounce frames that are scrubbed after each use | Cleartext legacy peer; local process hoping to find leftovers | [AC 5] [RFC 10.2] | Phase 6 | not implemented |
| A process that calls `rfork(RFNOLEGACY)` cannot mount, bind or walk a channel that goes through the legacy gateway.  The flag is bit 18 and `rfork` rejects it until Phase 6 **[new]** | Cleartext legacy peer (accidental leakage to it) | [RFC 10.3] [AC 5] | Phase 6 | not implemented |
| A client that does not acknowledge a lease revocation within the timeout (default 5 seconds) loses the lease and its channel | Malicious or unresponsive client holding leases | [RFC 11] [RFC 6.2.3] | Phase 6 | not implemented |
| A discovered resource is authenticated before its channel is registered in `/srv` | Sender of forged discovery beacons | [new] (was [AC 5.9] of the superseded v4 compendium) | Phase 6 | not implemented |
| A note is delivered only onto a real stack segment, with checked arithmetic inside the stack bounds | Malicious local process aiming its stack pointer | [AC 1] [IG 2] | Phase 7 | not implemented |
| The kernel builds every VirtIO header for the `#G` device itself; user bytes never become a kernel header (the device is [AC 7]; the header rule is **[new]**) | Malicious local process writing crafted bytes | [AC 7] [new] | Phase 14 | not implemented |
| File access follows owner and group permissions, only a group leader can change a group's membership, and a user with no local record is attenuated to `none` | Authenticated user without permission; user from another domain | [PT Test 1.1] [PT Test 1.3] [PT Test 3.2] | Phase 13 | not implemented |

The six Phase 4 wire protections above are host-tested by `cargo xtask test-9p` over an in-memory
link and are not used by a booted kernel: `devmnt` still offers plain `9P2000` by default (D-23).
Peer authentication is not provided in Phase 4 (D-16): the session secret is supplied behind a test
seam, the real authenticated exchange is undecided, and no live authenticated connection between
two real machines comes out of this phase.  The capability-revocation and failover-reclaim rows
remain unimplemented.

---

## 6. Out of scope and the remedy

- Probing live DRAM on the physical memory bus is out of scope.  **[new]** (carried from [AC 2.2] of the superseded v4 compendium)
- Cold-boot attacks on physical memory are out of scope.  **[new]** (carried from [AC 2.2] of the superseded v4 compendium)
- Arbitrary code execution in kernel context is out of scope: cryptographic keys are held in kernel memory, so a compromised kernel supervisor exposes them.  **[new]** (carried from [AC 2.2] of the superseded v4 compendium)
- Software protection of pages at rest, stale-frame and shadow-frame encryption, `ymm0`-`ymm5` register sealing, `PKEY`/`WRPKRU` page-fault hooks and sibling-key isolation are removed from scope by the 2026-11-25 v3.0 rescope, which drops software cryptographic memory in favour of hardware MMU isolation.  [AC 1]
- The remedy for physical DRAM threats is hardware memory encryption, AMD SEV-SNP or Intel TDX, required if bus sniffing is ever brought into the threat model.  **[new]** (carried from [AC 2.2] of the superseded v4 compendium)
- It follows that none of the protections in section 5 is a defence against an attacker in any of these positions, and none should be described as one.  **[new]**

---

## 7. Open points and source contradictions

The rescope settled or voided most of the points this section used to track.  The dispositions,
none of which this document settles by itself:

- **Settled or void since 2026-11-25.**  The MAC primitive is keyed BLAKE3 only (W3, Phase 3); session keys are per connection and per direction (W4, Phase 4 D-15); ChaCha20 uses the RFC 8439 layout (N1, Phase 3 D-06); the `StatelessCap` travels in its own TLV under a sixth `ExtMask` bit (W2, Phase 4 D-11 to D-14); the wire MAC is verified on packet receipt before any field is acted on, and the fault-time check is void because frames are verified and decrypted on receive (N11, Phase 4 D-26); the stale-frame mechanism, the shadow-frame boundary sentence and `RFRANDMEM`'s semantics are void with software cryptographic memory (N8 to N10).  [RFC 3.1] [RFC 3.2] [RFC 4.4]
- **The retired `rfork` cryptographic bits.**  rfork bits 18-20 were retired with software cryptographic memory by the 2026-11-25 v3.0 rescope (CORR-04, D-02): `RFNOLEGACY` is bit 18 (`1 << 18`), and the adjacent bits 19-21 are free.  The compendium's eliminated list names the retired flags.  [AC 1] **[new]**
- **`StatelessCap` contents and tag length.**  The wire form is settled (W2, Phase 4); the remaining gap is that no source gives the forgery bound the 8-byte capability tag is meant to provide or a limit on verification attempts.  [RFC 5.2] **[new]**
- **Beacon spoofing.**  The discovery beacon has no authentication field, and no source says what a forged or replayed beacon can cause before the authentication step that precedes `/srv` registration: dialling an attacker's address, exhausting connections, or holding a dead node `Healthy`.  **[new]**
- **How discovery authenticates.**  No source says which identity a beacon's `hostname` and `domain` fields are checked against.  **[new]**
- **Denial of service other than lease retention.**  Lease retention is the only availability attack any source treats `[RFC 11]`.  Connection floods, forced teardown by a single forged packet (the response to a MAC failure is to end the connection `[RFC 4.3]`), and resource exhaustion are untreated.  **[new]**
- **Trust placed in `factotum`.**  Every channel key descends from a secret `factotum` produces `[RFC 3.2]`.  No source states what is assumed about `factotum`, the authentication server, or the key-exchange protocol, which the RFC gives only by example (a Noise handshake or a DP9IK ticket exchange); Phase 4 injects the session secret behind a test seam (D-16).  **[new]**
- **Trust placed in holders of the cluster secret.**  Any replica holding the domain secret can issue a token every other replica accepts `[RFC 5.2]`.  No source covers the compromise of one replica, rotation of the secret, or revocation of a `StatelessCap`.  **[new]**
- **Side channels.**  Timing of MAC verification, cache occupancy, speculative-execution channels and the access pattern visible in cleartext headers are untreated by every source.  **[new]**
- **Traffic analysis.**  Headers are authenticated but sent in the clear `[RFC 4.3]`, so message types, sizes, handles and timing are visible to an on-path observer.  No source says whether that is accepted.  **[new]**

---

## Sources

- AC: `docs/plan_r9_architecture_code_compendium_v3_0.md`, the v3.0 architecture compendium
  (2026-11-25).  `[AC 1]` is its section 1 (Architectural Mandate & v3.0 Direction Changes),
  `[AC 2]` the workspace crate and safety boundaries, `[AC 5]` the `9P2000.aead` wire fabric and
  cluster extensions, `[AC 7]` storage, graphics and toolchain targets.
- IG: `docs/r9_implementation_guide_for_agentic_developers_v3_0.md`, the v3.0 implementation guide
  (2026-11-25).  `[IG 2]` is its realigned roadmap table.
- RFC: `.planning/rfc-9p2000-aead.md`, the `9P2000.aead` protocol specification, as amended through
  Phase 4.  Where its zero-copy paging and cryptographic-memory sections differ from the v3.0
  documents, the v3.0 documents win.
- PT: `.planning/r9-permissions-and-test-plan.md`, the permissions and test plan.  Its
  cryptographic-memory tests (4.1, 4.2) are dropped with the rescope and are not cited here.
- `.planning/PROJECT.md`: the Key Decisions table, including the 2026-11-25 rescope rows, and the
  notes N1 to N11 named in section 7.
- `.planning/ROADMAP.md`: the phase numbers in section 5.
- The superseded documents (`.planning/r9-architecture-and-code-v4.md`,
  `.planning/r9-implementation-guide-amended-v3.md`) are kept for history only; where they disagree
  with the v3.0 documents, the v3.0 documents win.
