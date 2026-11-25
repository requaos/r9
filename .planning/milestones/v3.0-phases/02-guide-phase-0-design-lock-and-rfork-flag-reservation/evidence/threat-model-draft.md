# r9 Threat Model

This document states what r9's cryptographic memory and its `9P2000.aead` transport are meant to
protect, against whom, and what is deliberately left unprotected.  It gathers the security
statements scattered through the four design documents into one place, so that the phases that
build the protections can be checked against a single statement of intent.

**Status.** This is a living document.  Nothing in it is fixed, the scope boundary included; later
phases revise it as designs firm up.  No protection described in this document is implemented in
r9's code.  At the time of writing the workspace members are `aarch64`, `x86_64`, `port`, `riscv64`
and `xtask`; there is no `crypto/` crate, no `ds/` crate and no AEAD transport code.  Every
protection below is a design intention with a delivering phase, and section 5 gives each one the
status `not implemented`.  Read "is specified to" wherever a mechanism is described.

**Citation rule.** Every statement cites the document that makes it, or is marked as new.  The tags
are `[AC n.n]` for a section of the architecture compendium, `[RFC n.n]` for a section of the
`9P2000.aead` RFC, `[IG n.n]` or `[IG n row k]` for a section or a table row of the implementation
guide, and `[PT §n.n]` or `[PT Test n.n]` for a section or a numbered test of the permissions and
test plan.  The mark **[new]** means the statement is made first in this document and has no
source; those are the statements a reviewer most needs to check.  A row that carries both a tag and
**[new]** has a sourced part and an unsourced part, and says which is which.  The four documents
are listed under Sources.  They disagree with each other in places; section 7 lists the
disagreements and settles none of them.

---

## 1. Scope boundary

The boundary is section 2.2 of the architecture compendium, reproduced word for word:

> **In Scope:** Protection of pages at rest (evicted frames, backing stores, NVMe swap, Venti archives, distributed 9P-AEAD transit); prevention of stale-frame data disclosure across distinct process KeyIDs; protection against stray kernel reads through incorrect page table mappings (since only the shadow frame contains plaintext); prevention of packet splicing, injection, and replay over 9P network connections.

> **Out of Scope:** Physical hardware bus probing of live DRAM, cold-boot physical attacks, and arbitrary code execution within the kernel context. Because cryptographic keys are held within kernel memory, a compromised kernel supervisor inevitably exposes keys. Hardware memory encryption (AMD SEV-SNP, Intel TDX) is required if physical DRAM bus sniffing is in the threat model.

Both sentences are `[AC 2.2]`.  The same boundary appears in extracted form as constraint AC-T01 in
`.planning/intel/constraints.md`.  Sections 2 to 5 analyse what lies inside the boundary; section 6
restates what lies outside it.

---

## 2. Assets

What an attacker would want to read, forge or destroy.  The sources name these things but never
list them as assets; the list as a list is **[new]**.

| Asset | Where it lives | Source |
|-------|----------------|--------|
| A process's master identity seed `S_root` and the private heap keys derived from it | Process key state held by the kernel; never sent across the network | [RFC 3.2] |
| The user's root session key `K_user` and the per-object keys `K_file = HKDF-Expand(K_user, Qid.path ‖ Qid.vers, 32)` | Derived on demand for a persistent or immutable resource | [AC 5.1] [RFC 3.2] [IG 6.1] |
| The session secret `S_session` produced by the authenticated key exchange | The `factotum` agent at each end of a connection | [RFC 3.2] |
| The transient channel key `K_transient` | Each endpoint's memory tables, until the channel is closed | [RFC 3.2] |
| The cluster (domain) secret that signs `StatelessCap` tokens | Every server replica in the cluster | [AC 5.2] [RFC 5.2] |
| Segment keys `k_dec` and `k_mac`, held in `KeyCapability` typestates inside self-zeroizing containers | Kernel memory | [AC 3.1] [IG 4.1] |
| Page plaintext | The shadow frame only | [AC 2.2] [IG 4.4] |
| Page ciphertext at rest | Evicted frames, backing stores, NVMe swap, Venti archives | [AC 2.2] |
| The integrity tags and nonces that authenticate sealed pages | On disk, in out-of-band tag slabs of `TagEntry` records (a 12-byte nonce and a 32-byte BLAKE3 tag each).  Where the tags of evicted memory pages are kept is not stated **[new]** | [AC 6.2] [AC 3.1] |
| 9P message payloads in transit | Ciphertext on the network between two nodes | [RFC 3] [AC 5.3] |
| 9P control headers (size, type, tag, `FidHandle`, sequence nonce, `ExtMask`, extension TLVs) | Cleartext on the wire; authenticated, not hidden | [RFC 4.3] [AC 5.3] |
| Capability tokens carried as `EXT_CAP_TOKEN` (32 bytes) and the revocation list they are checked against | The wire, and the server's cache-resident revocation list | [RFC 4.4] [AC 5.3] |
| `StatelessCap` tokens | Held by the client as its file handle | [AC 5.2] [RFC 5.2] |
| The legacy session key `K_legacy_session` and the plaintext that passes through the legacy gateway | Pre-pinned, non-pageable kernel bounce frames | [AC 5.8] [RFC 10.2] |
| Vector register contents during kernel page cryptography (cipher state in `ymm0` to `ymm5`) | CPU registers, for the length of a fault | [AC 3.3] [IG 4.3] [RFC 11] |
| Discovered service records `/srv/{hostname}_{class}_{instance}`, their `.status` files and the `healthy_replicas` set | `/srv` and the `#M` mount driver | [AC 5.9] [IG 6.4] |
| User and group records (`/adm/users`) | The file server `fossil01` | [PT §3.2] |
| The server's ability to complete a write while clients hold read leases | The lease state on the server | [RFC 11] |

---

## 3. Attacker classes and capabilities

The sources name only "network adversaries" `[RFC 4.3]`, "malicious processes" `[IG 3 row 16]`,
"malicious or unresponsive clients" `[RFC 11]` and the test persona `eve` `[PT §3.1]`.  They do not
define attacker classes or state capabilities.  The classification below, and every capability not
quoted from a source, is **[new]**.

| Class | Capabilities | In or out of scope | Source |
|-------|--------------|--------------------|--------|
| On-path network adversary | Reads every packet on a fabric between nodes; replays a captured packet; moves a valid payload to another channel by editing the cleartext header; injects packets.  Also drops, delays and reorders packets **[new]** | In scope: splicing, injection and replay | [RFC 4.3] [PT Test 2.3] [AC 2.2] |
| Malicious local process, including a sibling running under the same user | Runs arbitrary user-mode code; tries to read another process's private heap; aims its stack pointer at a read-only segment before a note is delivered; writes crafted bytes to a device file | In scope: disclosure across KeyIDs.  The note and device cases are corrected defects, not boundary items **[new]** | [PT §3.1] [PT Test 4.1] [IG 3 row 16] [IG 3 row 15] [AC 2.2] |
| Authenticated user without permission for a file | Mounts the file server with valid credentials and tries to read or change files that belong to others | Covered by the permission suites; not named in the boundary **[new]** | [PT §3.1] [PT Test 1.1] |
| User from another authentication domain with no local record | Presents a valid ticket for the other domain and tries to read private files | Covered by the permission suites; not named in the boundary **[new]** | [PT Test 3.2] |
| Malicious or unresponsive client holding leases | Keeps read leases and withholds `Rlease_ack`, to stall a conflicting write | In scope of the RFC's security considerations; not named in the boundary **[new]** | [RFC 11] [RFC 6.2.3] |
| Holder of stolen or copied storage media | Reads and alters evicted frames, backing stores, swap and archives while r9 is not running on them **[new]** | In scope: pages at rest | [AC 2.2] |
| Cleartext legacy peer, and anyone on the path to it | Speaks `9P2000`, `9P2000.u`, `9P2000.e` or `9P2000.L` with no encryption; sees and can alter everything on that link (the path part is **[new]**) | Data on a legacy link is unprotected by design; the gateway and `RFNOLEGACY` limit what reaches it | [AC 5.8] [RFC 10.1] [RFC 10.3] |
| Sender of forged discovery beacons | Any host that can send to the discovery multicast groups emits a beacon with a hostname, class, address and port of its choosing, or replays one.  The beacon fields are listed in the source and none of them is an authenticator | No source treats it **[new]** | [AC 5.9] |
| Compromised replica, or any other holder of the cluster secret | Mints a `StatelessCap` for any immutable file and has it accepted by every replica | No source treats it **[new]** | [AC 5.2] [RFC 5.2] |
| Compromised `factotum` or authentication server | Learns or chooses the session secret from which channel keys are derived | No source treats it **[new]** | [RFC 3.2] |
| Local attacker using microarchitectural side channels | Observes vector registers or reassigned frames through speculative execution; other channels (timing, cache occupancy, power) are **[new]** and untreated | The register and frame cases are treated; the rest is not | [RFC 11] |
| Physical attacker with bus or cold-boot access | Probes live DRAM on the memory bus; images memory after a cold boot | Out of scope | [AC 2.2] |
| Attacker executing code in kernel context | Reads keys directly, because keys are held in kernel memory | Out of scope | [AC 2.2] |

---

## 4. Trust boundaries

Where data or control passes between parties that do not trust each other equally.  No source has
a trust-boundary list; the list as a list is **[new]**.

| Boundary | What crosses it | Source |
|----------|-----------------|--------|
| User process and kernel | System calls from a closed list; the user stack pointer at note delivery; bytes written to device files; user buffers the kernel reads and writes | [IG 2 row 3] [IG 3 row 16] [IG 3 row 15] [IG 3 row 17] |
| Process and process, across KeyIDs | Nothing is meant to cross: siblings have separate page-table roots and private segments have separate keys | [AC 2.2] [IG 2 row 8] [PT Test 4.1] |
| Kernel and backing store | Ciphertext and tags leave memory on eviction and return on a fault | [AC 2.2] [AC 3.1] |
| Node and network | Authenticated ciphertext with cleartext headers; incoming ciphertext is placed by DMA into an uninitialised frame and decrypted only on a later fault | [RFC 4.3] [AC 5.7] [RFC 9.1] |
| Application and network adapter through the user-space ring | Ring nodes under `#M` mapped into user space with `segattach`, bypassing the kernel file layer | [AC 5.7] |
| Authentication domain and authentication domain | Tickets from one domain presented to a server in another; an unregistered user is attenuated to `none` | [PT Test 3.1] [PT Test 3.2] |
| Native transport and the legacy gateway | Plaintext exists in kernel bounce frames while it is translated to or from an unencrypted peer | [AC 5.8] [RFC 10.2] |
| The cluster and the discovery multicast groups | Unauthenticated beacons arrive from the local subnet, link or site scope and lead to dialling and to `/srv` entries | [AC 5.9] [IG 6.4] |
| Kernel and `factotum` | `factotum` runs the authenticated key exchange that yields the session secret; every channel key is derived from that secret | [RFC 3.2] |
| Processes sharing one pooled connection | Several processes and users on one node send over a single connection, told apart by capability tokens | [AC 5.3] [RFC 4.4] |
| Server replica and server replica | Replicas share the domain secret, so each accepts tokens the others issued | [AC 5.2] [RFC 5.2] |
| Portable kernel code and architecture code | Hardware operations (`WRPKRU`, `MOVNTDQ`, `CLFLUSHOPT`, inline assembly, raw atomics) are confined to the architecture crates behind audited comments and reached through safe interfaces | [IG 2 row 7] [AC 1.1] |
| Network fabrics | Storage traffic, grid traffic and terminal traffic are carried on separate VLANs in the test topology | [PT §2.2] |

---

## 5. Protections

Each row is a protection a source specifies, the attacker class from section 3 it addresses, and
the phase of `.planning/ROADMAP.md` that is planned to deliver it.  The phase column and the pairing
of protection with attacker are **[new]**; the sources give neither.  The rows are worded in the
present tense for brevity; each describes what a source specifies, not something r9 does.  Every
status is `not implemented`.  Where sources disagree about a mechanism, the row points to section 7.

| Protection, as specified | Attacker addressed | Source | Delivering phase | Status |
|------------|--------------------|--------|------------------|--------|
| Pages at rest are ciphertext: evicted frames, backing stores, NVMe swap, Venti archives | Holder of storage media | [AC 2.2] | Phase 5 (memory, swap); Phase 13 (Venti archives) | not implemented |
| Keys are derived with HKDF-SHA256 (RFC 5869), so a child key is not a reversible function of its parent | Any attacker who obtains one derived key | [IG 3 row 1] [AC 2.1] | Phase 3 | not implemented |
| Pages are sealed encrypt-then-MAC with a MAC key separate from the decryption key; ciphertext is verified before it is decrypted into a shadow frame | Holder of storage media who alters ciphertext | [IG 3 row 5] [AC 2.1] | Phase 5 | not implemented |
| A `ReadOnly` capability holds no `k_mac`, so its holder cannot produce a valid integrity tag | Process with read-only access to a segment | [AC 3.1] [IG 4.1] | Phase 5 | not implemented |
| Stale-frame disclosure across KeyIDs is prevented.  The sources give three different mechanisms (section 7, item 3) | Malicious local process | [AC 2.2] [IG 4.4] [AC 3.4] [RFC 11] | Phase 5 | not implemented |
| A stray kernel read through a wrong mapping reaches ciphertext, because only the shadow frame holds plaintext.  This guards against a kernel defect, not an attacker (section 7, item 4 for the exception) | None; a kernel defect | [AC 2.2] | Phase 5 | not implemented |
| Frames pinned for DMA or for cryptography are never chosen for eviction, so they are not re-encrypted or reused mid-flight | None; a kernel defect | [IG 3 row 9] [IG 11] | Phase 5 | not implemented |
| Vector registers used in the fault path are zeroed with `VPXOR` before return to user mode, and AVX-512 state is disabled in XCR0 | Local attacker using side channels | [IG 4.3] [AC 3.3] [RFC 11] | Phase 5 | not implemented |
| 9P payloads in transit are encrypted with ChaCha20 | On-path network adversary | [RFC 3.1] [AC 5.3] | Phase 4 | not implemented |
| Splicing, injection and replay are rejected: size, type, tag, `FidHandle`, sequence nonce, `ExtMask` and TLVs are authenticated as associated data, and the receiver accepts only the next sequence number | On-path network adversary | [RFC 4.2] [RFC 4.3] [AC 5.3] [PT Test 2.3] | Phase 4 | not implemented |
| On a MAC failure the receiver ends the connection and zeroes the channel state without processing any enclosed byte | On-path network adversary | [RFC 4.3] [RFC 11] | Phase 4 | not implemented |
| Channel keys are dropped and zeroized when a channel is closed, so captured packets stay unreadable | Attacker who records traffic and later obtains endpoint state | [RFC 3.2] | Phase 4 | not implemented |
| A channel re-keys before its sequence counter reaches 2^64 - 1, so a nonce is never reused under one key | On-path network adversary | [RFC 11] | Phase 4 | not implemented |
| A `StatelessCap` is accepted only if its MAC verifies under the cluster secret | Client that forges or alters a file handle | [AC 5.2] [RFC 5.2] | Phase 4 | not implemented |
| A capability token on a pooled connection is validated against a revocation list | Process on a shared connection using a revoked or forged token | [RFC 4.4] [AC 5.3] | Phase 4 | not implemented |
| After failover, a reclaimed handle is restored only when the client's capability token validates; unreclaimed handles are freed and their keys zeroized | Client replaying a stale handle | [RFC 6.2.4] [AC 5.2] | Phase 4 and Phase 6 | not implemented |
| Legacy traffic exists as plaintext only in pre-pinned, non-pageable bounce frames that are scrubbed after each use | Cleartext legacy peer; local process hoping to find leftovers | [AC 5.8] [RFC 10.2] [IG 6.3] | Phase 6 | not implemented |
| A process that calls `rfork(RFNOLEGACY)` cannot mount, bind or walk a channel that goes through the legacy gateway.  The flag's bit is reserved in Phase 2 and `rfork` rejects it until Phase 6 **[new]** | Cleartext legacy peer (accidental leakage to it) | [RFC 10.3] [AC 5.8] | Phase 6 | not implemented |
| A client that does not acknowledge a lease revocation within the timeout (default 5 seconds) loses the lease and its channel | Malicious or unresponsive client holding leases | [RFC 11] [RFC 6.2.3] | Phase 6 | not implemented |
| A discovered resource is authenticated before its channel is registered in `/srv` | Sender of forged discovery beacons | [AC 5.9] [AC 9.14] | Phase 6 | not implemented |
| Siblings created with `rfork(RFCKEY)` have independent page-table roots and separate private keys; reading a sibling's frame fails the integrity check | Malicious local process (a sibling) | [PT Test 4.1] [IG 2 row 8] | Phase 7 | not implemented |
| After `rfork(RFROBASE)` a write to the read-only base segment is trapped by a hardware protection key | Malicious local process altering shared code | [PT Test 4.2] [IG 2 row 8] | Phase 7 | not implemented |
| A note is delivered only onto a real stack segment, with checked arithmetic inside the stack bounds | Malicious local process aiming its stack pointer | [IG 3 row 16] [AC 2.1] | Phase 7 | not implemented |
| The kernel builds every VirtIO header for the `#G` device itself; user bytes never become a kernel header | Malicious local process writing crafted bytes | [IG 3 row 15] [AC 2.1] | Phase 14 | not implemented |
| File access follows owner and group permissions, only a group leader can change a group's membership, and a user with no local record is attenuated to `none` | Authenticated user without permission; user from another domain | [PT Test 1.1] [PT Test 1.3] [PT Test 3.2] | Phase 13 | not implemented |

---

## 6. Out of scope and the remedy

- Probing live DRAM on the physical memory bus is out of scope. [AC 2.2]
- Cold-boot attacks on physical memory are out of scope. [AC 2.2]
- Arbitrary code execution in kernel context is out of scope. [AC 2.2]
- The reason given is that cryptographic keys are held in kernel memory, so a compromised kernel supervisor exposes them. [AC 2.2]
- The remedy named is hardware memory encryption, AMD SEV-SNP or Intel TDX, which is required if bus sniffing is brought into the threat model. [AC 2.2]
- It follows that none of the protections in section 5 is a defence against an attacker in any of these three positions, and none should be described as one. **[new]**

---

## 7. Open points and source contradictions

The first eight items are places where the sources contradict each other or the repository.  None
is settled here; where the project already tracks the question, its warning or note is named.

- **MAC primitive.** The RFC names keyed BLAKE3 for integrity `[RFC 3.1]`; the guide and the compendium label the wire MAC "BLAKE3 / Poly1305" `[IG 6.3]` `[AC 1.2]`.  Open as warning W3. Not settled here.
- **Session key scope.** The RFC's prose binds the transient key to the connection and direction, while its formula binds it to the `fid` `[RFC 3.2]`; the guide says per connection and per direction `[IG 3 row 13]`.  Open as warning W4.  Not settled here.
- **Stale-frame mechanism.** The guide says frames are taken from free pools with no scrubbing and relies on the unmapped page-table entry and a full overwrite `[IG 4.4]`; the compendium says a frame reassigned across domains is "scrubbed with `CLFLUSHOPT`" `[AC 3.4]`; the RFC says such frames must undergo zeroization and cache-line flushing `[RFC 11]`.  A cache-line flush does not zero memory, so these are three different mechanisms for one in-scope item.  Tracked as note N9.  Not settled here.
- **"Only the shadow frame contains plaintext."** The boundary says so `[AC 2.2]`, but in the legacy gateway plaintext sits in kernel bounce frames `[AC 5.8]` `[RFC 10.2]`.  Tracked as note N10.  Not settled here.
- **`StatelessCap` contents and tag length.** The compendium lists `qid_path`, `qid_vers`, `mode`, `epoch` and an 8-byte MAC `[AC 5.2]`; the RFC lists the file's hash, its permissions and an 8-byte tag `[RFC 5.2]`.  Both give a 64-bit tag, while the packet MAC and the capability token are 32 bytes `[RFC 4.2]` `[RFC 4.4]`.  The token's on-wire form is open as warning W2.  Not settled here.
- **Where the wire MAC is verified.** The RFC verifies on packet receipt, at the network adapter path `[RFC 9.1]`; the guide verifies ciphertext "in cache before decrypting into shadow frames" `[IG 3 row 5]`, and the compendium defers decryption to the page fault `[AC 5.7]`.  The sources do not say whether these are two checks under two keys.  Tracked as note N11.  Not settled here.
- **Statements about the repository that are not true of it.** The compendium says `port` forbids `unsafe` in every module and that `rfork` bits 18 to 20 are already reserved `[AC 1.1]`.  Tracked as notes N2 and N3.  Not settled here.
- **ChaCha20 parameters.** The RFC gives a 64-bit block counter and a 96-bit nonce `[RFC 3.1]`, which together exceed the 128 bits the cipher has for counter and nonce.  Tracked as note N1. Not settled here.

The remaining items are absent from every source.  Each is a gap a later phase has to close or
accept explicitly.

- **Beacon spoofing.** The discovery beacon has no authentication field `[AC 5.9]`, and no source says what a forged or replayed beacon can cause before the authentication step that precedes `/srv` registration: dialling an attacker's address, exhausting connections, or holding a dead node `Healthy`. **[new]**
- **How discovery authenticates.** The compendium says a discovered resource is authenticated `[AC 5.9]` and its appendix says through `factotum` `[AC 9.14]`, but no source says which identity a beacon's `hostname` and `domain` fields are checked against. **[new]**
- **Denial of service other than lease retention.** Lease retention is the only availability attack any source treats `[RFC 11]`.  Connection floods, forced teardown by a single forged packet (the response to a MAC failure is to end the connection `[RFC 4.3]`), and resource exhaustion are untreated. **[new]**
- **Trust placed in `factotum`.** Every channel key descends from a secret `factotum` produces `[RFC 3.2]`.  No source states what is assumed about `factotum`, the authentication server, or the key-exchange protocol, which the RFC gives only by example (a Noise handshake or a DP9IK ticket exchange). **[new]**
- **Trust placed in holders of the cluster secret.** Any replica holding the domain secret can issue a token every other replica accepts `[AC 5.2]`.  No source covers the compromise of one replica, rotation of the secret, or revocation of a `StatelessCap`. **[new]**
- **Side channels beyond the two the RFC names.** The RFC covers vector registers and reassigned frames `[RFC 11]`.  Timing of MAC verification, cache occupancy, and the access pattern visible in cleartext headers are untreated. **[new]**
- **Traffic analysis.** Headers are authenticated but sent in the clear `[RFC 4.3]`, so message types, sizes, handles and timing are visible to an on-path observer.  No source says whether that is accepted. **[new]**
- **`RFRANDMEM`.** The flag is named among the flags that go live with process lifecycle `[IG 8]`, but no source defines what it does, so no protection can be claimed for it.  Tracked as note N8. **[new]**
- **The 64-bit tag on `StatelessCap`.** No source gives the forgery bound that an 8-byte tag is meant to provide or a limit on verification attempts `[AC 5.2]`. **[new]**

---

## Sources

- AC: `.planning/r9-architecture-and-code-v4.md`, the architecture compendium.  Its section 9 is an appendix of source listings that the project treats as non-normative sketches.
- RFC: `.planning/rfc-9p2000-aead.md`, the `9P2000.aead` protocol specification.
- IG: `.planning/r9-implementation-guide-amended-v3.md`, the implementation guide.  `[IG 2 row k]` counts the rows of its reconciled-decisions table; `[IG 3 row k]` counts the rows of its corrections ledger.
- PT: `.planning/r9-permissions-and-test-plan.md`, the permissions and test plan.
- `.planning/intel/constraints.md`, entry AC-T01: the scope boundary in extracted form.
- `.planning/PROJECT.md`: the open warnings W1 to W10 and the notes N1 to N11 named in section 7.
- `.planning/ROADMAP.md`: the phase numbers in section 5.
