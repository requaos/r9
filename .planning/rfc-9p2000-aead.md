# RFC: The 9P2000.aead Protocol Specification

```
Network Working Group                                     Plan r9 Architecture Group
Request for Comments: 9200                                                J. Doe, Ed.
Category: Standards Track                                           Plan r9 Systems
Updates: 9P2000, 9P2000.u, 9P2000.e                                   November 2026
```

## Abstract

This document specifies **9P2000.aead**, an authenticated, zero-copy, cluster-scale evolution of the Plan 9 File Protocol (9P). 9P2000.aead preserves Plan 9's universal "everything is a file" philosophy while addressing the historical scaling bottlenecks of distributed 9P: round-trip latency amplification on metadata-heavy operations, absence of protocol-level cache coherence, throughput bottlenecks imposed by static message-size constraints, server memory exhaustion from stateful file handle tracking, and cascading search latency across multi-machine union directories.

The protocol introduces an authenticated encryption envelope binding 9P control headers as Authenticated Associated Data (AAD) ahead of a 4KB-aligned ChaCha20 ciphertext payload authenticated via Keyed BLAKE3. It establishes a hybrid file handle architecture supporting stateless capability tokens alongside single-memory-access stateful handles (`deref_tiny`), four new protocol message pairs (`Tcompound`/`Rcompound`, `Tstream`/`Rstream`, `Tlease_revoke`/`Rlease_ack`, and `Treclaim`/`Rreclaim`), explicit file volatility classes (`QTIMMUTABLE`, `QTLEASED`, `QTVOLATILE`), small-set and compressed bitmap lease tracking (`ClientSet`), and an in-kernel legacy translation gateway ensuring backward compatibility with unencrypted 9front and Linux clients.

---

## 1. Introduction & Problem Statement

The 9P protocol originated in Bell Labs' Plan 9 operating system to provide a uniform, transparent interface connecting user terminals, shared memory multiprocessing CPU servers, and centralized file engines. In traditional environments, 9P operated over local area networks using specialized transports (such as IL) and later standard TCP. While 9P excels at uniform abstraction, evaluating its performance across high-density computing clusters, multi-tenant cloud fabrics, and wide-area networks exposes six architectural scaling bottlenecks documented across cluster deployments:

1. **Synchronous Remote Procedure Call Amplification:**  
   Every standard 9P transaction requires a synchronous request (`T-message`) matched to a server response (`R-message`). Constructing a basic file manipulation sequence—walking to a directory node, opening the file, performing read or write operations, and closing the handle—demands a minimum of four sequential network round-trips (`Twalk`, `Topen`, `Tread`/`Twrite`, `Tclunk`). On high-diameter or routed networks, network round-trip time (RTT) dominates execution latency, degrading parallel compilation, directory traversals, and multi-tenant job launching.

2. **Absence of Protocol-Level Cache Coherence:**  
   Because Plan 9 unifies persistent storage with dynamic, stateful kernel devices (such as `/proc`, `/net`, and `/dev`), the base 9P protocol deliberately omits client-side data caching contracts and server-initiated invalidation callbacks. Every file access queries the remote server to prevent acting on stale data. When hundreds of diskless compute nodes boot concurrently or reference shared binaries from `/bin`, central file servers face severe I/O serialization storms.

3. **Throughput Ceilings Under Fixed Message Units:**  
   Standard 9P transfers data in chunks bounded by the negotiated maximum message size (`msize`). In historical deployments, `msize` was maintained at modest sizes (such as 8 KB) to prevent bulk transfers from inducing head-of-line starvation on interactive terminal traffic sharing the same transport connection. In the absence of streaming primitives, single-stream bulk throughput is mathematically capped at $\frac{\mathtt{msize}}{\text{RTT}}$, limiting performance on high-bandwidth links.

4. **Transport Multiplexing Bottlenecks:**  
   Multiplexing multiple independent processes across a single ordered TCP stream using 16-bit tag identifiers creates head-of-line blocking: a single lost packet halts every unrelated process on the client awaiting a reply. Furthermore, high-concurrency servers serialize requests through a single kernel mount channel bounded by the 65,535 tag ceiling.

5. **Stateful File Handle Memory Exhaustion:**  
   In 9P, every active directory traversal or open file consumes a 32-bit File Identifier (FID) tracked in stateful server-side tables until explicitly released via `Tclunk`. In large clusters with thousands of clients mounting shared roots, server memory consumption and hash table lookup overhead scale linearly with client count, degrading lookup efficiency.

6. **Sequential Union Directory Traversal Overhead:**  
   Plan 9 relies heavily on per-process union namespaces (`bind -a`, `bind -b`), where multiple physical directories are merged onto a single path. Resolving an absent entry in a union directory forces the client virtual file system to probe every bound layer sequentially. When union layers span multiple remote servers across network proxies (`exportfs`), resolving negative lookups triggers cascading network probes across every server in the chain.

9P2000.aead resolves these limitations by incorporating modern cryptographic primitives, succinct data structures, and streaming abstractions into the wire specification without adding custom system calls or breaking Plan 9's closed call list.

---

## 2. Key Words & Terminology

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this document are to be interpreted as described in BCP 14 (RFC 2119 / RFC 8174) when, and only when, they appear in all capitals, as shown here.

* **Client:** The endpoint initiating 9P requests (`T-messages`) and mounting remote file systems.
* **Server:** The endpoint exporting file trees, validating access permissions, and replying with `R-messages`.
* **FID (File Identifier):** A numeric handle representing an active path, open file, or synthetic device context on a connection.
* **QID:** A 13-byte unique server resource identifier consisting of a 1-byte type (`qtype`), a 4-byte version (`vers`), and an 8-byte unique path integer (`path`).
* **AAD (Authenticated Associated Data):** Cleartext protocol headers authenticated cryptographically alongside the payload to prevent packet tampering and splicing attacks.
* **TinyPtr:** An 8-bit displacement pointer used in succinct hash tables to dereference memory in a single read access.
* **TPHT:** Tiny Pointer Hash Table, an open-addressed hash table utilizing key quotienting and tiny pointers to achieve bounded probe complexity and single-read dereferencing.
* **Roaring Bitmap:** A compressed integer set data structure dynamically choosing between Array, Bitmap, and Run-Length Encoded (RLE) containers.

---

## 3. Protocol Architecture & Cryptographic Foundations

9P2000.aead establishes an end-to-end cryptographic transit channel between communicating nodes. The protocol operates on the principle that the encrypted wire format matches the operating system's internal encrypted memory layout, allowing network payloads to Direct Memory Access (DMA) straight into process backing stores without intermediate kernel decryption or re-encryption passes.

### 3.1 Cryptographic Primitives

The protocol mandates two core cryptographic primitives for data-plane traffic:

1. **Confidentiality:** ChaCha20 stream cipher in the RFC 8439 layout, with a 32-bit block counter and a 96-bit nonce.
2. **Integrity & Authenticity:** Keyed BLAKE3 operating in tree-hashing mode. BLAKE3 computes 1KB tree chunks across parallel vector lanes, achieving line-rate processing exceeding 10 GB/s per core and breaking the serial accumulation bottlenecks of legacy message authentication codes. Keyed BLAKE3 is the only MAC of this protocol; Poly1305 is not used, and [RFC8439] is cited for ChaCha20 only.

Amended 2026-11-24 (r9 Phase 3, decisions D-05 to D-08): item 1 previously read "64-bit block counters and 96-bit nonces", which asks 160 bits of ChaCha20's 128 bits of counter and nonce state; the RFC 8439 layout above (a 32-bit block counter and a 96-bit nonce) is the operative reading. Where this document says "HMAC-BLAKE3" (section 5), it means BLAKE3's keyed mode with the output truncated, and no HMAC is built over BLAKE3. The rule that extends the 8-byte wire sequence nonce to the 12-byte cipher nonce is settled in r9 Phase 4.

### 3.2 Key Hierarchy & Derivation

A process MUST NEVER transmit its master identity seed ($S_{\text{root}}$) or derived private heap keys across the network wire. All network interactions derive ephemeral channel keys through the authentication subsystem (`factotum`) during connection bootstrapping:

1. **Session Secret Handshake:**  
   During transport connection establishment, the client issues a `Tauth` message before `Tattach`. The local `factotum` agent executes an authenticated key exchange (such as a Noise Protocol handshake or mutual DP9IK ticket exchange) with the remote server's `factotum`, establishing a mutual shared secret ($S_{\text{session}}$).

2. **Transient Channel Key Derivation:**  
   Both endpoints derive an ephemeral, forward-secret channel key ($K_{\text{transient}}$) bound specifically to the connection and direction using RFC 5869 HKDF-SHA256:

   $$K_{\text{transient}} = \text{HKDF-Expand}(S_{\text{session}}, \text{"9P-AEAD-FID"} \parallel \mathtt{fid}, 32)$$

3. **Object-Level Key Binding:**  
   When referencing persistent or immutable resources, sub-keys are derived deterministically by binding the user's root session key to the resource's 13-byte QID path and version:

   $$K_{\text{file}} = \text{HKDF-Expand}(K_{\text{user}}, \mathtt{Qid.path} \parallel \mathtt{Qid.vers}, 32)$$

4. **Forward Secrecy & Channel Teardown:**  
   When a channel is closed via `Tclunk`, both endpoints drop $K_{\text{transient}}$ from their memory tables and execute deterministic memory scrubbing via zeroization routines (`Zeroizing<[u8; 32]>`), rendering captured wire packets permanently unrecoverable.

Amended 2026-11-24 (r9 Phase 4, decisions D-15 to D-17): items 2 and 4 above are superseded. The transient keys are per connection and per direction, not per FID: the formula `HKDF-Expand(S_session, "9P-AEAD-FID" || fid, 32)` and its `fid` binding no longer apply. One `Tauth` yields, for each direction, a 32-byte cipher key, a 32-byte MAC key and a 4-byte ChaCha20 nonce prefix, and the FID is bound by the packet MAC over the header rather than by a key. The derivation as implemented is one HKDF-Extract with the handshake transcript as salt and the 32-byte session secret (`S_session`) as input keying material, then one HKDF-Expand of 32 bytes per direction and purpose. Each `info` is the fixed 18-byte form `"9P2000.aead"` (11 bytes), `0x00`, direction (`0x01` client to server, `0x02` server to client), purpose (`0x01` cipher key, `0x02` MAC key, `0x03` nonce prefix), generation as 4 bytes little-endian. The transcript is unkeyed BLAKE3 over every plain-framed handshake message whole, in wire order: `Tversion`/`Rversion`, `Tauth`/`Rauth` and the auth-fid `Twrite`/`Rwrite`/`Tread`/`Rread`. For a re-key the next transcript chains: unkeyed BLAKE3 of the previous generation's 32-byte transcript hash followed by the exchange messages of that re-key only — the in-band `Tauth`, `Rauth` and every afid `Twrite`, `Rwrite`, `Tread` and `Rread` — each re-framed in its standard 9P2000 form `size[4] type[1] tag[2]` then the standard body, whole, in the order they cross the wire (the exchange rides auth fid 7 with tags 1, 2 and 3). The sealed bytes, the 28-byte header, the TLVs, the page padding and the MAC are not hashed, and neither is `Trekey` or `Rrekey` of any kind nor any other traffic sealed during the exchange. Keys are zeroized on connection teardown, not on `Tclunk`, so item 4's teardown rule is superseded too. Pinned by: `session_derive_known_answer` and `aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation`.

Amended 2026-11-25 (r9 Phase 4 gap closure, DECISION P9): "(the exchange rides auth fid 7 with tags 1, 2 and 3)" in the note above is superseded. The in-band exchange rides the reserved afid `0xFFFFFFFE`, and every one of its messages carries the reserved tag `NOTAG` (`0xFFFF`): the in-band `Tauth`, `Rauth`, `Twrite`, `Rwrite`, `Tread` and `Rread`. A receiver treats a sealed message as exchange traffic only when its tag is `NOTAG` and its type is one of those six, with the reserved afid in the `Tauth`, `Twrite` and `Tread`; every other sealed message is ordinary traffic and is never consumed by the re-key state machine. The re-key control pair `Trekey`/`Rrekey` likewise carries `NOTAG`. The data of the standard-form frames the transcript absorbs is unpadded, as the note above states: a 21-byte `Twrite` is absorbed as exactly 44 bytes (`size[4] type[1] tag[2] fid[4] offset[8] count[4] data[21]`), and the page padding P4 puts on the wire is never hashed. A `Tstream` or `Rstream` follows the same rule if the exchange ever carries one: reserved tag (and, for `Tstream`, the reserved afid in its `FidHandle`), the same standard body form, data unpadded. Pinned by: `aead_transport_rekey_transcript_twrite_frame_is_44_bytes_unpadded` and `aead_transport_rekey_exchange_messages_carry_reserved_tag_and_afid`.

---

## 4. Wire Framing & Packet Structure

Every 9P2000.aead packet encapsulates data within an authenticated encryption envelope where all transport control fields are authenticated as Associated Data.

### 4.1 Binary Wire Envelope Format

The packet layout is structured as follows:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                          Size (4B)                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   Type (1B)   |           Tag (2B)            |   Pad (1B)    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                      FidHandle (8B)                           +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                   Monotonic Sequence Nonce (8B)               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|        ExtMask (2B)           |       ExtTLVs Length (2B)     |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    ExtTLVs Payload (Variable)                 |
|                               ...                             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                  Ciphertext Payload (ChaCha20)                +
|                    (Strictly 4KB Aligned)                     |
|                               ...                             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
+                     Keyed BLAKE3 MAC (32B)                    +
|                                                               |
+                                                               +
|                                                               |
+                                                               +
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

Amended 2026-11-24 (r9 Phase 4, decision D-06): the header is 28 bytes, and an earlier note of this layout in the phase's context gave 24, an arithmetic slip; the field list above is the decision. The eight fields and their offsets from byte 0 of the packet are `Size` at 0 (4 bytes), `Type` at 4 (1), `Tag` at 5 (2), `Pad` at 7 (1), `FidHandle` at 8 (8), `SeqNonce` at 16 (8), `ExtMask` at 24 (2) and `ExtTLVs Length` at 26 (2), a total of 28 bytes, followed by the TLVs, the ciphertext and the 32-byte MAC. Every multi-byte field is little-endian. `Pad` must be zero; a non-zero `Pad` is a protocol error once the MAC verifies.

### 4.2 Field Definitions

* **Size (32 bits, little-endian):**  
  The total length of the packet in bytes, including the Size field itself, the authenticated header, extension TLVs, ciphertext payload, and the 32-byte BLAKE3 MAC.
* **Type (8 bits):**  
  The 9P message opcode defining the operation (see Section 6 for message opcode assignments).
* **Tag (16 bits, little-endian):**  
  A client-assigned identifier pairing a response (`R-message`) with an outstanding request (`T-message`). Tag `0xFFFF` (`NOTAG`) is reserved for `Tversion`/`Rversion`.
* **FidHandle (64 bits):**  
  The wire handle identifying the target file or capability context (see Section 5).
* **Monotonic Sequence Nonce (64 bits, little-endian):**  
  A strictly incrementing sequence counter per connection and direction. Senders MUST increment this counter by exactly 1 for every transmitted packet. Receivers MUST reject any packet whose sequence counter does not strictly match the expected next sequence integer, preventing packet replay and out-of-order injection.
* **ExtMask (16 bits, little-endian):**  
  A presence bitmask declaring which optional out-of-band extension TLVs follow the base header (see Section 4.3).
* **ExtTLVs Length (16 bits, little-endian):**  
  The total byte length of the extension TLVs payload. If `ExtMask == 0`, this field MUST be set to 0.
* **ExtTLVs Payload (variable):**  
  Contiguous, 64-bit aligned extension attribute records parsed zero-copy from the frame buffer.
* **Ciphertext Payload (variable):**  
  The ChaCha20-encrypted message body. In data-carrying messages (`Twrite`, `Rread`, `Tstream`), the payload buffer MUST be 4KB-aligned to support zero-copy physical DMA directly into host page frames.
* **Keyed BLAKE3 MAC (256 bits / 32 bytes):**  
  The cryptographic authentication tag computed over the entire AAD header block concatenated with the ciphertext payload.

### 4.3 Authenticated Associated Data (AAD) & Anti-Splicing Rules

The AAD input to the Keyed BLAKE3 MAC is defined as:

$$\mathtt{AAD} = \mathtt{Size} \parallel \mathtt{Type} \parallel \mathtt{Tag} \parallel \mathtt{FidHandle} \parallel \mathtt{SeqNonce} \parallel \mathtt{ExtMask} \parallel \mathtt{ExtTLVs}$$

Because the FID, message tag, packet size, and monotonic sequence counter are explicitly committed to the BLAKE3 MAC, network adversaries cannot execute **splicing attacks** (transplanting valid encrypted payloads from one file handle or stream context to another) or **replay attacks** (re-injecting previously captured transactions). If MAC verification fails at the transport boundary, the receiver MUST immediately terminate the connection and zero all associated channel state without processing any enclosed bytes.

Amended 2026-11-24 (r9 Phase 4, decisions D-07, D-08 and D-26): the AAD field list above is superseded. The MAC covers every byte of the packet before the MAC as one contiguous slice, from byte 0 of `Size` to the last ciphertext byte (the half-open range `[0, size - 32)`), so `Pad` and `ExtTLVs Length` are authenticated as well; the field list `Size || Type || Tag || FidHandle || SeqNonce || ExtMask || ExtTLVs` no longer describes the input. The receiver verifies that MAC over the whole packet before it acts on any field but `Size`, which it reads only to bound the packet. The 12-byte ChaCha20 nonce is the 4-byte derived prefix of section 3.2 followed by the 8-byte wire sequence number, little-endian; the prefix is never sent on the wire. The same AAD formula in guide section 6.3 and compendium section 5.3 is superseded by this paragraph. Pinned by: `session_packet_mac_is_keyed_blake3_under_the_mac_key` and `aead_transport_wire_tag_is_packet_mac_of_all_bytes_before_it`.

### 4.4 Extension Bitmask (`ExtMask`) & TLVs

The `ExtMask` bit positions govern optional out-of-band protocol metadata:

```rust
pub const EXT_DEADLINE: u16       = 1 << 0;
pub const EXT_CAP_TOKEN: u16      = 1 << 1;
pub const EXT_QID_VOLATILITY: u16 = 1 << 2;
pub const EXT_STREAM_PRIO: u16    = 1 << 3;
pub const EXT_OBINARY: u16        = 1 << 4;
```

#### Detailed Attribute Definitions:

1. **`EXT_DEADLINE` (`0x0001`):**  
   Contains an 8-byte monotonic cluster deadline formatted as an unsigned 64-bit nanosecond timestamp. When propagated across chained `exportfs` proxies, intermediaries and servers evaluate deadlines locally. If the deadline expires while a request is queued or in-flight, the server MUST cancel processing immediately and drop the transaction without requiring an explicit `Tflush` round-trip.

2. **`EXT_CAP_TOKEN` (`0x0002`):**  
   Encapsulates a 32-byte cryptographic capability token (signed via HMAC-BLAKE3). This token allows multiple client processes and users on a single compute node to multiplex across a single shared, pooled 9P2000.aead transport connection without triggering connection storms on authentication servers. Servers validate tokens in $O(1)$ against a cache-resident Roaring Bitmap Certificate Revocation List (CRL).

3. **`EXT_QID_VOLATILITY` (`0x0004`):**  
   Emitted in `Rwalk`, `Ropen`, and `Rcreate` responses to declare the caching contract and lease parameters of the resolved QID. The record carries:
   * `volatility_class`: 1 byte (`0x20` = `QTIMMUTABLE`, `0x40` = `QTLEASED`, `0x80` = `QTVOLATILE`)
   * `lease_generation`: 8 bytes (`u64` lease epoch)
   * `granted_mode`: 1 byte (`0` = None, `1` = SharedRead, `2` = ExclusiveWrite)

4. **`EXT_STREAM_PRIO` (`0x0008`):**  
   Provides transport multiplexing guidance across shared channels. Encodes:
   * `priority_tier`: 2 bits (`P0_CONTROL = 0`, `P1_METADATA = 1`, `P2_BULK = 2`)
   * `flow_credit`: 30 bits representing incremental window flow-control credits in bytes

5. **`EXT_OBINARY` (`0x0010`):**  
   Negotiates structured binary serialization over synthetic kernel file devices (`/proc`, `#G`, `/net`, `telescope`). When asserted, the server returns aligned, zero-copy binary data structures directly from memory rather than serializing data into ASCII text strings.

Amended 2026-11-24 (r9 Phase 4, decisions D-11, D-12, D-27 and P1): there are six extension bits, not five. The sixth is `EXT_STATELESS_CAP = 1 << 5` (`0x0020`), and the mask's known bits are `0x003F`. Each record is `type[1] reserved[1] len[2] value[len] padding`, all multi-byte fields little-endian: `type` is the bit index 0 to 5, `reserved` is zero, `len` is the value length unpadded, and zero padding rounds the record up to a multiple of 8 bytes measured from the start of the TLV area. Records appear in ascending type order, each type at most once, the set of types equals the set bits of `ExtMask` exactly, and `len` is the fixed value length for its type: `EXT_DEADLINE` 8, `EXT_CAP_TOKEN` 32, `EXT_QID_VOLATILITY` 10, `EXT_STREAM_PRIO` 4, `EXT_OBINARY` 0 and `EXT_STATELESS_CAP` 24. `EXT_CAP_TOKEN` keeps its single meaning, the 32-byte per-user token above; the capability of section 5 travels under `EXT_STATELESS_CAP`, not here. Any bit outside the six, or any rule above broken, is a protocol error found after the MAC verifies and closes the connection; there is no skip path for an unknown bit. The `EXT_QID_VOLATILITY` class byte (`0x20`, `0x40`, `0x80`) is its own byte and is never written into a qid type byte, where those three values are Plan 9's `QTEXCL`, `QTAPPEND` and `QTDIR` (D-27).

---

## 5. Hybrid File Handle Architecture (`FidHandle`)

Traditional 9P servers maintain a stateful descriptor for every active FID, pinning server heap memory and requiring lock-synchronized hash lookups. 9P2000.aead replaces this with a **Hybrid File Handle (`FidHandle`)** architecture that distinguishes immutable, content-addressed files from mutable, stateful streams.

On the wire, every `FidHandle` occupies exactly 8 bytes (64 bits).

### 5.1 The `FidHandle` Wire Layout

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StatefulHandle {
    pub fid_key: u32,       // 4 bytes: Client-scoped FID key
    pub ptr: TinyPtr,       // 1 byte: NonZeroU8 displacement into TPHT arena
    pub shard_id: u8,       // 1 byte: Server cache-aligned shard ID (0..=255)
    pub epoch: u16,         // 2 bytes: Server generation epoch
} // 8 bytes exact

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct StatelessCap {
    pub qid_path: u64,      // 8 bytes: Content-addressed unique path
    pub qid_vers: u32,      // 4 bytes: Version identifier
    pub qid_type: u8,       // 1 byte: Includes QTIMMUTABLE bit
    pub mode: u8,           // 1 byte: Open access mode
    pub epoch: u16,         // 2 bytes: Epoch counter
    pub mac_tag: [u8; 8],   // 8 bytes: Truncated Keyed BLAKE3 MAC
} // 24 bytes in memory (represented via token indirection or wire capability)

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FidHandle {
    Stateless(StatelessCap),
    Stateful(StatefulHandle),
}
```

### 5.2 Stateless Capabilities (`StatelessCap`)

For files marked `QTIMMUTABLE` (such as Venti Content-Addressed Storage blocks, read-only system binaries, and static snapshots), the server **allocates zero memory in its internal FID table**.

When a client walks to an immutable file, the server returns a `StatelessCap` token embedded in the handle. The token contains the file's cryptographic hash, permissions, and an 8-byte HMAC-BLAKE3 verification tag computed using a server cluster secret. On subsequent `Tread` transactions, any server node in the cluster possessing the domain secret verifies the MAC in $O(1)$ and serves data directly from storage without tracking the client's session state.

### 5.3 Stateful Handles & Single-Memory-Access Resolution (`deref_tiny`)

For mutable files, device streams (`#G`), inter-process communication pipes (`#|`), and active leases, state is required. The server allocates a slot in its **Flattened Tiny Pointer Hash Table (TPHT)** (Tang et al., 2026).

When the handle is established, the server embeds an 8-bit `TinyPtr` directly into the `StatefulHandle`. On every subsequent `Tread`, `Twrite`, or `Tstream` request:

$$\mathtt{slot} = ((b \ll 4) + (\mathtt{ptr.get()} - 1)) \ \& \ \mathtt{slot\_mask}$$

The server resolves the 64-byte `FidEntry` in **strictly one memory access** directly from the flat value arena, completely bypassing:
1. Full 64-bit key comparisons
2. Bucket header scans
3. Seqlock spin loops
4. Secondary hash probe chains

```rust
#[repr(C, align(64))]
#[derive(Copy, Clone)]
pub struct FidEntry {
    pub fid_key: u32,       // 4 bytes: Owning client FID
    pub client_id: u32,     // 4 bytes: Authenticated client node ID
    pub qid_path: u64,      // 8 bytes: Authoritative resource path
    pub offset: u64,        // 8 bytes: Current file cursor
    pub qid_vers: u32,      // 4 bytes: Resource version
    pub iounit: u32,        // 4 bytes: Preferred I/O chunk size
    pub lease_gen: u32,     // 4 bytes: Associated distributed lease generation
    pub ring_slot: u32,     // 4 bytes: Zero-copy DMA bypass ring slot
    pub dev_handler: u16,   // 2 bytes: Kernel synthetic device index
    pub epoch: u16,         // 2 bytes: Allocation epoch
    pub qid_type: u8,       // 1 byte: Volatility class and mode
    pub open_mode: u8,      // 1 byte: Read/Write/Exec flags
    pub flags: u16,         // 2 bytes: Status flags
    pub _reserved: [u8; 16],// 16 bytes: Hardware cache line padding
} // Total: 64 bytes exact (1 CPU cache line)
```

Amended 2026-11-24 (r9 Phase 4, decisions D-11 to D-14 and P8): the 8-byte `FidHandle` field above is right, but a `StatelessCap` is not carried in it. The header's `FidHandle` field stays 8 bytes in every packet; a `StatelessCap` travels in its own `EXT_STATELESS_CAP` TLV of section 4.4 (24 bytes), and when that TLV is present the header field must be all zero, otherwise the packet is rejected. That is the discriminator between the two handle kinds, and it is safe because a `StatefulHandle` can never be all zero. The capability stays 24 bytes: the first 16 bytes are `qid_path(8) qid_vers(4) qid_type(1) mode(1) epoch(2)` and the last 8 are the tag. The tag is the first 8 bytes of keyed BLAKE3 under the domain secret over the label `r9 StatelessCap1` followed by those first 16 bytes; a failed verification closes the connection. `epoch` is the issuing table's current epoch, and a table at a later epoch rejects a capability issued at an earlier one. The capability is a bearer token within the domain: it is not bound to a client or a connection, and any replica holding the domain secret verifies it in O(1) with zero server FID memory. Pinned by: `session_cap_tag_is_truncated_keyed_blake3_of_label_and_body` and `aead_fid_stateless_tag_is_keyed_blake3_of_label_and_wire_body`.

---

## 6. Protocol Message Set & Operations

9P2000.aead supports the foundational message set of 9P2000 (`Tversion`, `Tauth`, `Tattach`, `Twalk`, `Topen`, `Tcreate`, `Tread`, `Twrite`, `Tclunk`, `Tremove`, `Tstat`, `Twstat`) and introduces four specialized message pairs designed for distributed cluster operation.

### 6.1 Opcode Registry

| Opcode (`u8`) | Request | Opcode (`u8`) | Response | Architectural Role |
| :--- | :--- | :--- | :--- | :--- |
| `100` | `Tversion` | `101` | `Rversion` | Negotiates protocol dialect and message unit |
| `102` | `Tauth` | `103` | `Rauth` | Establishes authenticated capability sessions |
| `104` | `Tattach` | `105` | `Rattach` | Attaches connection to root file namespace |
| `110` | `Twalk` | `111` | `Rwalk` | Traverses path hierarchy; returns `FidHandle` |
| `112` | `Topen` | `113` | `Ropen` | Prepares handle for data I/O; grants leases |
| `114` | `Tcreate` | `115` | `Rcreate` | Creates and opens a new regular or synthetic file |
| `116` | `Tread` | `117` | `Rread` | Reads data chunk from file handle |
| `118` | `Twrite` | `119` | `Rwrite` | Writes data chunk to file handle |
| `120` | `Tclunk` | `121` | `Rclunk` | Closes and releases file handle and keys |
| `122` | `Tremove` | `123` | `Rremove` | Unlinks and deletes file from server namespace |
| `124` | `Tstat` | `125` | `Rstat` | Queries file metadata attributes |
| `126` | `Twstat` | `127` | `Rwstat` | Mutates file metadata attributes |
| **`130`** | **`Tcompound`** | **`131`** | **`Rcompound`** | **Atomic multi-operation execution pipeline** |
| **`132`** | **`Tstream`** | **`133`** | **`Rstream`** | **Bidirectional streaming and server push** |
| **`134`** | **`Tlease_revoke`** | **`135`** | **`Rlease_ack`** | **Asynchronous cache lease revocation** |
| **`136`** | **`Treclaim`** | **`137`** | **`Rreclaim`** | **Post-failover handle recovery** |

Amended 2026-11-24 (r9 Phase 4, decisions D-28 and P3): the registry above omits three base 9P2000 messages that the codec carries, and gains one more pair. `Rerror` is 107, `Tflush` is 108 and `Rflush` is 109; opcode 106 is not a 9P2000 message and is illegal. The in-band re-key pair of section 11 is `Trekey` 138 (client to server) and `Rrekey` 139 (server to client), with tag `NOTAG` (0), a zero `FidHandle` and the body `kind[1] generation[4] confirm[32]`, where `kind` is 0 `REQUEST`, 1 `MARKER` or 2 `ACK`. All ten message pairs 130 to 139 are encoded and decoded in Phase 4; their distributed semantics beyond the re-key pair are later phases' work.

Amended 2026-11-25 (r9 Phase 4 gap closure, DECISION P3 and P9): the note above gives the re-key pair's tag as `NOTAG` (0); the value is `NOTAG` (`0xFFFF`), and every in-band exchange message carries it too. The Tag sentence of section 4.2, "Tag `0xFFFF` (`NOTAG`) is reserved for `Tversion`/`Rversion`", is extended: `NOTAG` is also the tag of every `Trekey` and `Rrekey` (P3) and of the in-band exchange messages `DECISION P9` gives it to — the in-band `Tauth`, `Rauth`, `Twrite`, `Rwrite`, `Tread` and `Rread`, and, under the owner's P9 amendment, a `Tstream` or `Rstream` — and no ordinary request carries it (`devmnt` issues tags 1 to `0xFFFE`). Pinned by: `aead_transport_rekey_control_messages_carry_notag`.

---

### 6.2 Detailed Specifications of New Message Types

#### 6.2.1 `Tcompound` / `Rcompound` (Opcodes 130 / 131)

`Tcompound` collapses multiple dependent operations into a single atomic transaction executed within a single network round-trip.

* **Format:**
  ```text
  size[4] Tcompound tag[2] num_ops[2] op_stream[...]
  size[4] Rcompound tag[2] num_completed[2] status[2] result_stream[...]
  ```
* **Execution Semantics:**  
  The server maintains an ephemeral `CURRENT_FID` register during the evaluation of the sub-operation stream. If a sub-operation specifies FID `0xFFFFFFFE` (`NOFID_CURRENT`), the server automatically substitutes the active `CURRENT_FID` resulting from the immediately preceding operation.
* **Failure Atomicity:**  
  Operations are evaluated sequentially. If any sub-operation returns an error, execution stops immediately, intermediate state changes are rolled back where applicable, and `Rcompound` returns the error status along with the count of successfully executed operations (`num_completed`).
* **Canonical Use Case:**  
  Allocating an interactive virtual network conversation or graphics context:
  $$\mathtt{Walk}(\text{"net/tcp/clone"}) \longrightarrow \mathtt{Open}(\mathtt{ORDWR}) \longrightarrow \mathtt{Read}(\mathtt{offset}=0, \mathtt{count}=16) \longrightarrow \mathtt{Clunk}$$
  This entire control-plane sequence executes in exactly one round-trip.

#### 6.2.2 `Tstream` / `Rstream` (Opcodes 132 / 133)

`Tstream` establishes continuous data streaming over an active `FidHandle`, bypassing the stop-and-wait request-response loop and eliminating the $\frac{\mathtt{msize}}{\text{RTT}}$ throughput limit.

* **Format:**
  ```text
  size[4] Tstream tag[2] fid_handle[8] stream_id[4] flags[2] offset[8]
  size[4] Rstream tag[2] stream_id[4] chunk_seq[8] count[4] data[...]
  ```
* **Flow Control:**  
  Streaming channels maintain sliding-window flow control using `EXT_STREAM_PRIO` headers. Senders MUST NOT emit data chunks exceeding the granted stream window credit. Receivers issue window updates by emitting empty `Rstream` frames carrying updated byte credits.

#### 6.2.3 `Tlease_revoke` / `Rlease_ack` (Opcodes 134 / 135)

A server-initiated, asynchronous notification revoking or downgrading a client's local cache lease on a `QTLEASED` file.

* **Format:**
  ```text
  size[4] Tlease_revoke tag[2] qid_path[8] lease_gen[8] target_mode[1] num_blocks[4] [block_ranges]
  size[4] Rlease_ack tag[2] qid_path[8] lease_gen[8] client_status[1]
  ```
* **Coherence Semantics:**  
  When a write operation conflicts with active read leases, the server dispatches `Tlease_revoke` to all holding clients. The message carries a compressed Roaring Bitmap or range list indicating which 4KB block indices are affected. 
* Clients receiving `Tlease_revoke` MUST:
  1. Purge or flush the specified 4KB blocks from their local cache.
  2. Suspend local speculative execution against the revoked ranges.
  3. Emit `Rlease_ack` confirming lease invalidation.
* The server blocks the conflicting write until all outstanding `Rlease_ack` messages are received or a lease timeout expires.

#### 6.2.4 `Treclaim` / `Rreclaim` (Opcodes 136 / 137)

Provides idempotent state recovery following the restart or failover of a server node.

* **Format:**
  ```text
  size[4] Treclaim tag[2] old_handle[8] target_epoch[2] client_token[32]
  size[4] Rreclaim tag[2] new_handle[8] recovery_status[2]
  ```
* **Recovery Protocol:**  
  Following a server reboot, the server advertises an incremented generation epoch in `Rversion` and opens a configurable grace period (default: 30 seconds). Clients possessing active `StatefulHandle` descriptors replay them via `Treclaim`. The server validates the client's capability token, restores the channel state into the active TPHT shard, and records the restored FID in a shard-local `reclaimed_fids` Roaring Bitmap.
* **Orphan Sweeping:**  
  When the grace window closes, the server executes a single SIMD set difference against the pre-failover table:

  $$\mathtt{OrphanedHandles} = \mathtt{active\_fids} \setminus \mathtt{reclaimed\_fids}$$

  All un-reclaimed descriptors are immediately freed, and their cryptographic keys are zeroized without memory leaks.

Amended 2026-11-24 (r9 Phase 4, decisions D-09, P4 and P5): the data messages above name the wrong message for the data field. The messages whose ciphertext carries whole 4096-byte pages are `Twrite`, `Rread` and `Rstream`; `Tstream` has no data field (its format above carries only `stream_id`, `flags` and `offset`). In those three, the encrypted body is two parts: a small parameter block, then file data padded with zeros to a whole number of 4096-byte pages, with the true byte count in the parameters. `Twrite`'s parameters are `fid[4] offset[8] count[4]` (16 bytes) and `Rread`'s are `count[4]` (4 bytes). Parameters encrypt from ChaCha20 block counter 0 and every parameter block fits one 64-byte block; data page `i` (0-based) encrypts from block counter `1 + 64 * i`, so each page decrypts on its own at its counter. The base message bodies keep their 9P2000 bodies byte for byte, and the header's `FidHandle` carries the primary fid's server-issued handle. The four new pairs and the re-key pair are encoded and decoded only in Phase 4; nothing here says their distributed behaviour works. Pinned by: `aead_transport_wire_data_pages_decrypt_alone_at_their_block_counter`.

---

## 7. Distributed Cache Coherence & Volatility Contracts

9P2000.aead eliminates unnecessary network round-trips by codifying cacheability directly into the resource identifier. The high-order bits of `Qid.qtype` establish three mutually exclusive volatility classes:

```rust
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VolatilityClass {
    Immutable = 0x20, // QTIMMUTABLE
    Leased    = 0x40, // QTLEASED
    Volatile  = 0x80, // QTVOLATILE
}
```

### 7.1 Volatility Classes

1. **`QTIMMUTABLE` (`0x20`):**  
   Applied to content-addressed storage (Venti blocks), immutable file snapshots, and read-only system binaries (`SG_TEXT` / Block A). Clients MAY cache data indefinitely in local DRAM or NVMe storage. Clients MUST NOT issue `Tstat` or `Twalk` revalidation queries. Because contiguous immutable 4KB blocks cannot mutate, they collapse into Run Containers within the client's `SuperpageResidency` tracker, triggering $O(1)$ transparent 2MB hardware superpage promotions in the CPU memory management unit.

2. **`QTLEASED` (`0x40`):**  
   Applied to standard mutable file systems (such as Fossil storage). Clients receive read or write leases from the server. Clients holding a valid read lease MAY serve reads directly from their local cache. Clients holding an exclusive write lease MAY buffer writes locally in uncommitted shadow frames. Leases are revoked asynchronously via `Tlease_revoke` upon conflict.

3. **`QTVOLATILE` (`0x80`):**  
   Applied to dynamic kernel synthetic devices (`/proc`, `/dev`, `/net`, `#G`). Clients MUST NOT cache data, read-ahead buffers, or directory lookups. All read and write transactions route directly to the authoritative server device handler.

### 7.2 The `ClientSet` Small-Set Optimization

On server nodes, tracking which clients hold read leases on a given inode presents a significant scalability bottleneck: while most files have 1 to 4 readers, shared datasets and common system libraries have thousands of concurrent reader nodes.

To eliminate heap allocation overhead for the common case while scaling seamlessly to cluster workloads, the server represents reader cohorts using the **`ClientSet`** algebraic data type:

```rust
pub enum ClientSet {
    Empty,
    Single(u32),
    Inline { count: u8, ids: [u32; 4] },
    Roaring(Box<RoaringBitmap>),
}
```

* **Inline Phase ($\le 4$ clients):** Occupies 24 bytes inline within the 64-byte `LeaseRecord`. Allocates zero heap memory.
* **Roaring Bitmap Phase ($> 4$ clients):** Automatically converts to a compressed Roaring Bitmap. Contiguous cluster compute node IDs (e.g., nodes `100` through `4096`) collapse into a single 4-byte Run-Length Encoded (RLE) container.
* **Targeted Revocation:** When a write lease upgrade occurs, the server computes revocation targets by removing the writing client (`sharers.remove(writer_id)`), instantly extracting the exact recipient list for targeted `Tlease_revoke` RPCs.

---

## 8. Accelerated Namespace Resolution & Union Mounts

Plan 9 per-process union namespaces allow multiple directories to be bound over a single mount point. In traditional implementations, evaluating a file path in a union directory forces the mount driver (`#M`) to query each bound layer sequentially, creating severe latency bottlenecks when traversing remote network mounts.

9P2000.aead eliminates sequential probing through **Roaring Layer Filters** and **Speculative Parallel Walks**:

```
Client VFS Walk: Twalk("bin", "mk")
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│ UnionMountTable (port/src/union.rs)                         │
│                                                             │
│ 1. Compute Salted Hash: h = parent_qid ^ name_hash          │
│ 2. Evaluate Exact RoaringBitmap Filters:                   │
│    • Layer 0 (Local /bin):      entry_hashes.contains(h)?   │
│    • Layer 1 (Cluster /n/fossil): entry_hashes.contains(h)?   │
│    • Layer 2 (Dynamic /proc):   dynamic_layer_mask assert   │
└────────────────────────────┬────────────────────────────────┘
                             │
            Filter Out Non-Matching Layers (0 Probes)
                             ▼
┌─────────────────────────────────────────────────────────────┐
│ Speculative Parallel Execution                              │
│                                                             │
│ Dispatches Twalk ONLY to candidate layers simultaneously:   │
│ • Candidate Layer 0: [Pending...]                           │
│ • Candidate Layer 1: [Pending...]                           │
│ Winning response commits to UnionRouteEntry dcache in L1.   │
└─────────────────────────────────────────────────────────────┘
```

1. **Exact Roaring Bitmap Layer Presence Filters:**  
   Every static or leased directory layer publishes a compressed Roaring Bitmap (`entry_hashes`) containing salted 32-bit hashes of all resident file names. The client mount driver checks candidate membership in local RAM in $O(1)$ time with **zero false positives**, skipping non-matching remote servers without transmitting a single network packet.

2. **Speculative Parallel Walks:**  
   Layers flagged in `dynamic_layer_mask` (such as live synthetic devices) and matching static layers are probed concurrently using speculative parallel `Twalk` futures. The earliest positive response from the highest-priority layer commits to the mount driver's 32-byte `UnionRouteEntry` cache (stored in a quotiented TPHT with 4-bit Clock-Nibble aging), while remaining in-flight probes are aborted.

---

## 9. Zero-Copy DMA Paging & Kernel-Memory Interplay

9P2000.aead is engineered to integrate directly with operating system demand-paging mechanisms, eliminating memory copying across the user-kernel boundary.

```
Machine 1 (Client Node)                                       Machine 2 (Server Node)
┌──────────────────────────────────────┐                      ┌──────────────────────────────────────┐
│ User Process Virtual Memory Space    │                      │ Storage / Device Engine (Fossil/#G)  │
│ [Virtual Page Fault Address (#PF)]   │                      └──────────────────┬───────────────────┘
└──────────────────▲───────────────────┘                                         │
                   │                                                             │ ChaCha20 + BLAKE3
                   │ Lazy Demand Decrypt                                         ▼
┌──────────────────┴───────────────────┐                      ┌──────────────────────────────────────┐
│ Physical Memory: UninitializedFrame  │◄─────────────────────┤ Network Adapter (RoCE / VirtIO-net)  │
│ (Sealed Ciphertext in BackingStore)  │  Zero-Copy DMA Off   │ 9P2000.aead Wire Packet Ingress      │
└──────────────────────────────────────┘  Network Wire        └──────────────────────────────────────┘
```

### 9.1 Zero-Copy Ingress Pipeline

1. **Direct Memory Placement:**  
   When the network interface controller (NIC) receives an incoming 9P2000.aead packet, it parses the cleartext AAD header and validates the BLAKE3 MAC using $K_{\text{transient}}$. 
2. **Allocator Bypass (`UninitializedFrame`):**  
   The kernel allocator provides raw, un-zeroed physical frames (`UninitializedFrame`). The network adapter DMAs incoming ciphertext directly off the wire into physical frames assigned to the destination process's `BackingStore`.
3. **Lazy Decryption:**  
   The kernel **DOES NOT** decrypt data upon packet arrival. Decryption is deferred until the application thread actively touches the virtual memory address, triggering a page fault (`#PF`).
4. **Dual-Path Store Routing:**  
   When `#PF` executes, the kernel's restricted vector routine (using registers `ymm0` through `ymm5`) decrypts the 4KB frame:
   * The 64-byte line matching the fault address is written using cached stores (`VMOVAPS`) so it sits hot in L1/L2 cache when user execution resumes.
   * The remaining 4,032 bytes are written using non-temporal streaming stores (`MOVNTDQ`), avoiding CPU cache pollution.
   * Write-combining buffer drainage (`SFENCE`) is scheduled concurrently with scalar tag verification, absorbing memory pipeline latency.

---

## 10. Legacy Interoperability Gateway

To ensure seamless operation within heterogeneous computing environments, r9 implements an **In-Kernel Legacy Translation Gateway** supporting standard, unencrypted 9P clients and servers (such as native 9front machines, Linux systems using `v9fs`, and userland `plan9port` daemons).

```
[Legacy Node: 9front / Linux v9fs]
                │
                │ Cleartext 9P2000 / 9P2000.u / 9P2000.L
                ▼
┌─────────────────────────────────────────────────────────────┐
│ r9 Translation Gateway (port/src/legacy_9p.rs)              │
│                                                             │
│ • Dedicated, non-pageable, pre-pinned Bounce Buffer Pool    │
│ • Ingress: DMA cleartext -> Encrypt -> Commit BackingStore  │
│ • Egress:  Decrypt from BackingStore -> Cleartext Wire      │
│ • Deterministic Scrub: Immediate zeroize + CLFLUSHOPT       │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
        [r9 Cryptographically Isolated BackingStore]
```

### 10.1 Protocol Negotiation Rules

During transport negotiation, the server inspects the version string presented in `Tversion`:

* If `version == "9P2000.aead"`, the connection operates under the native end-to-end cryptographic transit engine.
* If `version` specifies `"9P2000"`, `"9P2000.u"`, `"9P2000.e"`, or `"9P2000.L"`, the connection is routed to the legacy translation gateway.

### 10.2 Ephemeral Bounce Buffering & Sanitization

Because legacy clients cannot parse ChaCha20-BLAKE3 envelopes or manage cryptographic capabilities:

1. **Memory Isolation:** The gateway mediates cleartext traffic through pre-pinned, non-pageable kernel bounce frames.
2. **Ingress Path:** Data received from a legacy client is read into a bounce frame, encrypted on-the-fly under a synthetic session key derived via `factotum`, and placed into the target process's `BackingStore`. The bounce frame is immediately sanitized using `Zeroize` and `CLFLUSHOPT`.
3. **Egress Path:** Data requested by a legacy client is resolved from `BackingStore`, decrypted into the bounce frame, and transmitted across the wire. The bounce frame is scrubbed immediately following transmission.

### 10.3 Security Policy Enforcement (`RFNOLEGACY`)

To prevent accidental data leakage from cryptographically protected processes to cleartext networks, r9 introduces the **`RFNOLEGACY`** process flag (`rfork(RFNOLEGACY)`). Processes executed with this flag are forbidden from mounting, binding, or walking channels mediated by the legacy translation gateway, ensuring sensitive data remains confined to authenticated 9P2000.aead links.

---

## 11. Security Considerations

1. **Keystream Reuse & Nonce Management:**  
   Reusing a ChaCha20 nonce with the same key breaks confidentiality. Senders MUST maintain a strictly monotonic 64-bit sequence counter. If the sequence counter approaches $2^{64} - 1$, the channel MUST execute an in-band re-keying handshake before transmitting further data.
2. **Microarchitectural Side-Channel Resistance:**  
   Plaintext memory frames and vector registers represent potential targets for speculative execution attacks (e.g., Spectre). All cryptographic operations in the kernel fault path MUST restore and scrub vector registers (`ymm0`..`ymm5`) via `VPXOR` before returning execution to user mode. Physical frames reassigned across distinct `KeyID` security domains MUST undergo hardware zeroization and cache line flushing (`CLFLUSHOPT`).
3. **Splicing & Replay Defenses:**  
   Transport frames MUST authenticate all header fields, FIDs, tags, and sequence numbers as AAD. Implementations MUST reject packets failing BLAKE3 MAC verification without attempting partial parsing.
4. **Denial of Service via Lease Retention:**  
   Malicious or unresponsive clients holding read leases could attempt to stall server write operations. Servers MUST enforce strict lease revocation timeouts (default: 5 seconds). If a client fails to emit `Rlease_ack` within the timeout window, the server revokes the lease unilaterally and drops the offending client channel.

Amended 2026-11-24 (r9 Phase 4, decisions D-18 to D-21 and P7): item 1's re-keying rule is settled as follows. Re-keying is a fresh `Tauth` exchange run in band inside the sealed channel, deriving new keys from a new session secret; there is no hash ratchet. Either side starts it when its send counter reaches `REKEY_AFTER_PACKETS = 1 << 48`, a fixed threshold well below `2^64 - 1` and settable low in tests. Cutover is per direction, strict, with acknowledgement and timeout: the sender's last packet under its old send keys is a sealed `Trekey` MARKER, and its next packet uses the new keys with the counter back at 0; the receiver, on the marker, switches its receive keys that way and answers with an `Rrekey` ACK carrying a confirm value; the receiver zeroizes the old receive keys on the marker and the sender zeroizes its old send keys when the ACK arrives. The confirm is keyed BLAKE3 under the new MAC key of the direction cut over, over the label `r9 rekey ack` followed by the new generation as 4 bytes little-endian (generation `g + 1` when the cutover is from `g`), and it proves the receiver derived the same keys. If the ACK does not arrive within `REKEY_ACK_TIMEOUT_MS = 5000` milliseconds the sender drops the connection and zeroizes everything. An old-key packet is never accepted after the marker, with or without the ACK: there is no overlap window, and outstanding requests and replies continue across the cutover. The sequence number `u64::MAX` is never sent; a sender that would reach it with no new keys closes the connection rather than wrap, and a packet whose sequence number is not the expected next value is dropped and the connection closed and zeroized, not merely dropped. Item 3's anti-splicing rule stands. The stated limit of this phase (D-16): the exchange is a seam that is handed a 32-byte session secret, the real authenticated key exchange and the Noise versus DP9IK choice are undecided, and no live authenticated connection between two real machines comes out of this phase. Pinned by: `session_rekey_confirm_is_keyed_blake3_of_label_and_generation` and `aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation`.

---

## 12. IANA Considerations

This document specifies a formal protocol dialect name for use in 9P protocol version negotiation:

* **Protocol Dialect String:** `"9P2000.aead"`
* **Default TCP/UDP Port:** `564` (Standard 9P service port)
* **Default Multicast Discovery Endpoints:**
  * IPv4: `239.9.9.9` (Port `9999`)
  * IPv6: `ff02::9:9` (Port `9999`, Link-Local scope)

---

## 13. References

### 13.1 Normative References

* **[RFC2119]** Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997.
* **[RFC8174]** Leiba, B., "Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words", BCP 14, RFC 8174, May 2017.
* **[RFC5869]** Krawczyk, H. and P. Eronen, "HMAC-based Extract-and-Expand Key Derivation Function (HKDF)", RFC 5869, May 2010.
* **[RFC8439]** Nir, Y. and A. Langley, "ChaCha20 and Poly1305 for IETF Protocols", RFC 8439, June 2018 (cited for ChaCha20 only).

### 13.2 Informative References

* **[9P2000]** Pike, R., et al., "The Plan 9 Resource Sharing Protocol", Bell Laboratories, 4th Edition Plan 9 Manual, 2002.
* **[BLAKE3]** O'Connor, J., et al., "BLAKE3: One function, fast everywhere", 2020.
* **[TPHT-2026]** Tang, S., Mai, Z., Kuszmaul, B. C., and Conway, A., "Succinct and Fast Tiny Pointer Hash Tables", arXiv:2607.28892v1, 2026.
* **[Roaring-2016]** Lemire, D., Ssi-Yan-Kai, G., and Kaser, O., "Consistently faster and smaller compressed bitmaps with Roaring", arXiv:1603.06549, 2016.
* **[OpenAddressing-2025]** Farach-Colton, M., et al., "Optimal Bounds for Open Addressing Without Reordering", arXiv:2501.02305v2, 2025.
