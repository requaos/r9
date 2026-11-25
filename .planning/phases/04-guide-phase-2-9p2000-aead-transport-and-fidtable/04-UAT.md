---
status: complete
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
source: [04-01-SUMMARY.md, 04-02-SUMMARY.md, 04-03-SUMMARY.md, 04-04-SUMMARY.md, 04-05-SUMMARY.md, 04-06-SUMMARY.md, 04-07-SUMMARY.md, 04-08-SUMMARY.md, 04-09-SUMMARY.md, 04-10-SUMMARY.md, 04-11-SUMMARY.md, 04-12-SUMMARY.md, 04-13-SUMMARY.md, 04-14-SUMMARY.md, 04-15-SUMMARY.md, 04-16-SUMMARY.md, 04-17-SUMMARY.md, 04-18-SUMMARY.md, 04-19-SUMMARY.md, 04-20-SUMMARY.md, 04-21-SUMMARY.md, 04-22-SUMMARY.md, 04-23-SUMMARY.md, 04-24-SUMMARY.md, 04-25-SUMMARY.md]
started: 2026-11-25T17:11:33.381Z
updated: 2026-11-25T17:28:48.517Z
---

## Current Test

[testing complete]

## Tests

### 1. Kernel byte baseline at the phase base commit
expected: Kernel byte baseline at the phase base commit: four-line before.sha256 and four numeric + four METRIC-COMMAND lines in before.metrics
coverage_id: D1
plan: 04-01
result: pass

### 2. Owner answers on wire proposals P1-P8
expected: Owner answers on the eight wire proposals P1 to P8 recorded as dated DECISION lines
coverage_id: D2
plan: 04-01
result: pass

### 3. IG-D11 lock recorded
expected: Owner answer on the IG-D11 lock recorded as DECISION IG-D11, PROJECT.md left untouched
coverage_id: D3
plan: 04-01
result: pass

### 4. port links crypto/ and ds/ by path, and lint rule R4 permits that edge for…
expected: port links crypto/ and ds/ by path, and lint rule R4 permits that edge for port alone in every dependency table
result: pass
source: automated
coverage_id: D1
plan: 04-02

### 5. crypto::session derives per-direction cipher key, MAC key and nonce prefix…
expected: crypto::session derives per-direction cipher key, MAC key and nonce prefix from secret, transcript and generation, with no key accessor and no Debug
result: pass
source: automated
coverage_id: D2
plan: 04-02

### 6. port::chan seals a packet with the 28-byte header at the pinned offsets and…
expected: port::chan seals a packet with the 28-byte header at the pinned offsets and opens it back to the same body
result: pass
source: automated
coverage_id: D3
plan: 04-02

### 7. open_packet refuses a flipped FidHandle byte, a flipped MAC byte, the wrong…
expected: open_packet refuses a flipped FidHandle byte, a flipped MAC byte, the wrong direction's keys and an unexpected sequence
result: pass
source: automated
coverage_id: D4
plan: 04-02

### 8. the tracer pin has RED, GREEN-1, GREEN-2 and ADVANCED lines in the phase…
expected: the tracer pin has RED, GREEN-1, GREEN-2 and ADVANCED lines in the phase evidence
result: pass
source: automated
coverage_id: D5
plan: 04-02

### 9. The transcript hash is unkeyed BLAKE3 over whole messages in wire order: A…
expected: The transcript hash is unkeyed BLAKE3 over whole messages in wire order: A then B equals BLAKE3(A||B), B then A differs, empty equals BLAKE3(empty), and chain(prev) absorbs prev first
result: pass
source: automated
coverage_id: D1
plan: 04-03

### 10. The six derivation outputs equal an independently computed Python HKDF vector…
expected: The six derivation outputs equal an independently computed Python HKDF vector on the default and scalar-backend legs, and a changed transcript bit or generation changes the keys
result: pass
source: automated
coverage_id: D2
plan: 04-03

### 11. zeroize clears all four keys and both prefixes; is_zeroized answers false for…
expected: zeroize clears all four keys and both prefixes; is_zeroized answers false for any single non-zero byte in the 68 covered positions and true after c2s-only or pair teardown
result: pass
source: automated
coverage_id: D3
plan: 04-03

### 12. cap_tag is the first 8 bytes of keyed BLAKE3 over r9 StatelessCap1 || body…
expected: cap_tag is the first 8 bytes of keyed BLAKE3 over r9 StatelessCap1 || body under the domain Key; cap_verify accepts it and rejects body, tag or domain-key changes
result: pass
source: automated
coverage_id: D4
plan: 04-03

### 13. rekey_confirm is keyed BLAKE3 under the direction's MAC key over r9 rekey ack…
expected: rekey_confirm is keyed BLAKE3 under the direction's MAC key over r9 rekey ack || generation LE; verify accepts the match and rejects other keys, generations and flipped bytes
result: pass
source: automated
coverage_id: D5
plan: 04-03

### 14. The packet MAC is keyed BLAKE3 under the Python-derived MAC key: split…
expected: The packet MAC is keyed BLAKE3 under the Python-derived MAC key: split updates equal one shot and each flipped tag byte is rejected
result: pass
source: automated
coverage_id: D6
plan: 04-03

### 15. Session zeroize documentation scope
expected: The module documentation states exactly what zeroize does and does not cover (P6 off): the four keys and two prefixes are scrubbed, the per-packet hasher and cipher state are not
coverage_id: D7
plan: 04-03
result: pass

### 16. The canonical 28-byte header: eight field offsets, little-endian…
expected: The canonical 28-byte header: eight field offsets, little-endian encode/decode, round trip and the framing bounds (59 rejected, 60 and msize accepted, msize + 1 rejected)
result: pass
source: automated
coverage_id: D1
plan: 04-04

### 17. Opcode registry pinned as a table: 100-105, 107-127, 130-137, 138/139 known…
expected: Opcode registry pinned as a table: 100-105, 107-127, 130-137, 138/139 known; 106, 128, 129 and 140+ rejected; is_t_message true exactly for even known opcodes
result: pass
source: automated
coverage_id: D2
plan: 04-04

### 18. A Verified packet only comes from the transport's MAC check: private fields…
expected: A Verified packet only comes from the transport's MAC check: private fields, crate-private new_verified called from transport.rs only
result: pass
source: automated
coverage_id: D3
plan: 04-04

### 19. Post-verification header rules: zero Pad, known opcode, ExtMask within…
expected: Post-verification header rules: zero Pad, known opcode, ExtMask within EXT_KNOWN, TLV length consistent with the mask, multiple of 8, inside the packet; capability bit requires an all-zero handle (D-12)
result: pass
source: automated
coverage_id: D4
plan: 04-04

### 20. Both pins carry RED, GREEN-1, GREEN-2 and ADVANCED lines in the phase evidence
expected: Both pins carry RED, GREEN-1, GREEN-2 and ADVANCED lines in the phase evidence
result: pass
source: automated
coverage_id: D5
plan: 04-04

### 21. The 27 base 9P2000 bodies 100-127 (no 106) encode to hand-built bytes and…
expected: The 27 base 9P2000 bodies 100-127 (no 106) encode to hand-built bytes and decode to an equal Msg; strings are raw bytes with a 16-bit length; Twalk/Rwalk preserve order and reject 17 elements
result: pass
source: automated
coverage_id: D1
plan: 04-05

### 22. The new pairs 130-137 and Trekey/Rrekey 138/139 round-trip in the RFC 6.2…
expected: The new pairs 130-137 and Trekey/Rrekey 138/139 round-trip in the RFC 6.2 field order; the re-key body is 37 bytes and kind 3 is rejected
result: pass
source: automated
coverage_id: D2
plan: 04-05

### 23. Twrite/Rread/Rstream carry a parameter block then zero-padded 4096-byte pages…
expected: Twrite/Rread/Rstream carry a parameter block then zero-padded 4096-byte pages with the true count; counts 0/1/4095/4096/4097 map to 0/1/1/1/2 pages and decode enforces the page and count bounds
result: pass
source: automated
coverage_id: D3
plan: 04-05

### 24. The pin ledger records the Harvey oracle digests and both pins'…
expected: The pin ledger records the Harvey oracle digests and both pins' RED/GREEN-1/GREEN-2/ADVANCED lines
result: pass
source: automated
coverage_id: D4
plan: 04-05

### 25. Transparency and safety prohibitions
expected: The transparency and safety prohibitions: 130-139 carry no behaviour beyond encode/decode, and data-page padding is written as zeros
coverage_id: D5
plan: 04-05
result: pass

### 26. The six extension records round-trip in every one of the 64 mask…
expected: The six extension records round-trip in every one of the 64 mask combinations, with padded lengths 16, 40, 16, 8, 8 and 32
result: pass
source: automated
coverage_id: D1
plan: 04-06

### 27. The parser borrows the area and allocates nothing: a counting global…
expected: The parser borrows the area and allocates nothing: a counting global allocator sees no allocation across a parse of the six-record area
result: pass
source: automated
coverage_id: D2
plan: 04-06

### 28. Every malformed area is rejected with a distinct error: unknown bits, wrong…
expected: Every malformed area is rejected with a distinct error: unknown bits, wrong order or duplicates, bad len/reserved/padding, a mask/area mismatch and a bad volatility class
result: pass
source: automated
coverage_id: D3
plan: 04-06

### 29. The stream-priority value splits into its 2-bit tier and 30-bit credit
expected: The stream-priority value splits into its 2-bit tier and 30-bit credit
result: pass
source: automated
coverage_id: D4
plan: 04-06

### 30. The plain-framed handshake with a transcript-bound derivation and the…
expected: The plain-framed handshake with a transcript-bound derivation and the SessionExchange seam: matching keys, an empty exchange, refusal before auth, an altered Rversion failing the first packet, and per-connection keys
result: pass
source: automated
coverage_id: D1
plan: 04-07

### 31. The sealed wire bytes: parameters at block counter 0, data page i at 1 + 64 *…
expected: The sealed wire bytes: parameters at block counter 0, data page i at 1 + 64 * i, and the tag as the MAC over every byte before it, all computed in the test without the transport
result: pass
source: automated
coverage_id: D2
plan: 04-07

### 32. Failing closed: any altered authenticated byte, a framing rejection, a short…
expected: Failing closed: any altered authenticated byte, a framing rejection, a short slice or a malformed-after-MAC packet closes the session and zeroizes both directions' keys with nothing decoded; teardown zeroizes and Tclunk alone does not
result: pass
source: automated
coverage_id: D3
plan: 04-07

### 33. The in-memory link fixture (port/tests/common/mod.rs, AeadLink, TestExchange…
expected: The in-memory link fixture (port/tests/common/mod.rs, AeadLink, TestExchange, connect/connect_with, ClientEnd, ServerEnd, Stage, decode counter) that every transport pin above runs on
result: pass
source: automated
coverage_id: D4
plan: 04-07

### 34. A re-key is due when the send counter reaches the configured threshold; the…
expected: A re-key is due when the send counter reaches the configured threshold; the in-band exchange travels sealed, and a server that reaches the threshold first asks the client to start it
result: pass
source: automated
coverage_id: D1
plan: 04-08

### 35. Per-direction strict cutover: the marker is the last old-key packet, the next…
expected: Per-direction strict cutover: the marker is the last old-key packet, the next packet is sequence 0 under the new keys, the receiver zeroizes its old keys at the marker and the sender holds them until the ACK; the directions cut over independently and a same-secret seam still changes the keys
result: pass
source: automated
coverage_id: D2
plan: 04-08

### 36. The acknowledgement carries the new generation and a confirm under the new…
expected: The acknowledgement carries the new generation and a confirm under the new receive-direction MAC key; the sender verifies it and zeroizes its old keys, and a wrong confirm or wrong generation closes the connection
result: pass
source: automated
coverage_id: D3
plan: 04-08

### 37. The caller-driven deadline closes the waiting sender with no clock read; an…
expected: The caller-driven deadline closes the waiting sender with no clock read; an old-key packet is rejected after the marker; the send counter never wraps and u64::MAX is never sent
result: pass
source: automated
coverage_id: D4
plan: 04-08

### 38. The re-key inputs are independent of the transport: the test derives…
expected: The re-key inputs are independent of the transport: the test derives generation 1 from the recorded frames and finds the first post-marker packet's MAC and each ACK's confirm match its own derivation and not the other direction, generation 0 or generation 2
result: pass
source: automated
coverage_id: D5
plan: 04-08

### 39. A removed entry's arena slot is zeroed, so deref_tiny of the removed (key…
expected: A removed entry's arena slot is zeroed, so deref_tiny of the removed (key, ptr) pair returns all-zero words until the slot is reused
result: pass
source: automated
coverage_id: D1
plan: 04-09

### 40. A table can name an arena slot by its dense id and read it with a…
expected: A table can name an arena slot by its dense id and read it with a bounds-checked call that returns None outside the arena
result: pass
source: automated
coverage_id: D2
plan: 04-09

### 41. StatefulHandle is 8 bytes, StatelessCap is 24 bytes, both round-trip their…
expected: StatefulHandle is 8 bytes, StatelessCap is 24 bytes, both round-trip their wire forms, and a stateful handle is never all zero
result: pass
source: automated
coverage_id: D3
plan: 04-09

### 42. FidHandle::from_wire is Stateful without a capability, Stateless with one and…
expected: FidHandle::from_wire is Stateful without a capability, Stateless with one and a zero header field, and an error when both are present
result: pass
source: automated
coverage_id: D4
plan: 04-09

### 43. FidEntry is 64 bytes with 64-byte alignment and round-trips its eight-word…
expected: FidEntry is 64 bytes with 64-byte alignment and round-trips its eight-word form, with fid_key and client_id in word 0
result: pass
source: automated
coverage_id: D5
plan: 04-09

### 44. The qid is 13 bytes as type[1] vers[4] path[8]; the volatility class has…
expected: The qid is 13 bytes as type[1] vers[4] path[8]; the volatility class has exactly three bytes that collide with the Plan 9 type bits; the #G path formula holds at its boundaries
result: pass
source: automated
coverage_id: D6
plan: 04-09

### 45. Both pins are recorded RED -> GREEN-1 -> GREEN-2 -> ADVANCED in the phase…
expected: Both pins are recorded RED -> GREEN-1 -> GREEN-2 -> ADVANCED in the phase evidence
result: pass
source: automated
coverage_id: D7
plan: 04-09

### 46. A handle resolves to its FidEntry in exactly one arena read and zero header…
expected: A handle resolves to its FidEntry in exactly one arena read and zero header reads
result: pass
source: automated
coverage_id: D1
plan: 04-10

### 47. A handle from another client or epoch, a clunked handle and an out-of-range…
expected: A handle from another client or epoch, a clunked handle and an out-of-range shard id do not resolve; the out-of-range case reads nothing
result: pass
source: automated
coverage_id: D2
plan: 04-10

### 48. A capability is issued and verified with zero FID memory; two tables with one…
expected: A capability is issued and verified with zero FID memory; two tables with one domain key verify each other and a different key rejects
result: pass
source: automated
coverage_id: D3
plan: 04-10

### 49. The 8-byte tag is the truncation of keyed BLAKE3 over the literal label and…
expected: The 8-byte tag is the truncation of keyed BLAKE3 over the literal label and the 16-byte wire body; any flipped byte and a wrong epoch fail verification
result: pass
source: automated
coverage_id: D4
plan: 04-10

### 50. The sweep returns exactly active minus reclaimed on randomized inputs…
expected: The sweep returns exactly active minus reclaimed on randomized inputs; reclaimed fids survive and swept fids are rejected; the reclaim window resets
result: pass
source: automated
coverage_id: D5
plan: 04-10

### 51. All three pins are recorded RED -> GREEN-1 -> GREEN-2 -> ADVANCED in the…
expected: All three pins are recorded RED -> GREEN-1 -> GREEN-2 -> ADVANCED in the phase evidence
result: pass
source: automated
coverage_id: D6
plan: 04-10

### 52. FidTable module-doc guarantees
expected: The module doc states a resolve is not a consistent snapshot and the capability is a bearer token with a 2^-64 forgery bound; client_id has no authenticated source yet
coverage_id: D7
plan: 04-10
result: pass

### 53. devmnt has the dialect selection point: an agreed 9P2000.aead hands the io…
expected: devmnt has the dialect selection point: an agreed 9P2000.aead hands the io halves, the negotiated msize and the exact Tversion/Rversion bytes to the engine; a plain answer to an aead offer is an error; an unknown version is the existing bad-version error; the msize rules are shared
result: pass
source: automated
coverage_id: D1
plan: 04-11

### 54. The plain path is unchanged: Mnt::negotiate keeps its signature and bytes…
expected: The plain path is unchanged: Mnt::negotiate keeps its signature and bytes, fversion still calls it, devmnt_test is unedited and green, and a plain offer still yields a working Mnt that walks and reads (including against a dual-stack fake)
result: pass
source: automated
coverage_id: D2
plan: 04-11

### 55. A whole sealed session runs from the real devmnt negotiation: Tauth exchange…
expected: A whole sealed session runs from the real devmnt negotiation: Tauth exchange plain, sealed attach (zero handle) returning a new handle, walk/open/read/clunk resolving through FidTable, Rread in one padded page with the true count, and teardown zeroizing both ends
result: pass
source: automated
coverage_id: D3
plan: 04-11

### 56. A capability issued in EXT_STATELESS_CAP verifies with zero FID memory and a…
expected: A capability issued in EXT_STATELESS_CAP verifies with zero FID memory and a flipped byte closes the connection; the same session survives an in-band re-key in both directions and reads the same bytes; a handle from another connection is refused with Rerror
result: pass
source: automated
coverage_id: D4
plan: 04-11

### 57. No booted-kernel aead caller; D-16 limit stated
expected: Nothing in a booted kernel asks for the aead dialect, and the D-16 limit is stated: no live authenticated connection between two real machines exists in this phase
coverage_id: D5
plan: 04-11
result: pass

### 58. `cargo xtask test-9p` runs the crypto `session` pins and the five `port`…
expected: `cargo xtask test-9p` runs the crypto `session` pins and the five `port` wire-pin files on the default and scalar backends, refuses any missing listed file before running, and errors when a leg reports no passed tests
result: pass
source: automated
coverage_id: D1
plan: 04-12

### 59. UNSAFE-AUDIT Phase 4 delta
expected: `UNSAFE-AUDIT.md` carries the Phase 4 delta: `port` links `crypto/` and `ds/`, `port::chan`/`port::fid` are forbid modules, the measured per-crate unsafe counts, and the re-measured `ds` unsafe line
coverage_id: D2
plan: 04-12
result: pass

### 60. Eight dated RFC amendment notes
expected: Eight dated Phase 4 amendment notes in the RFC text (sections 3.2, 4.1, 4.3, 4.4, 5, 6.1, 6.2, 11), each quoting superseded wording; five name their independent pin
coverage_id: D1
plan: 04-13
result: pass

### 61. Compendium 9.9 rewritten in place
expected: Compendium listing 9.9 rewritten in place with a dated note; section 5.3 points at amended RFC 4.3 for the MAC input
coverage_id: D2
plan: 04-13
result: pass

### 62. Decision records show the settled wire
expected: PROJECT.md, REQUIREMENTS.md and the threat model record the settled wire: W1/W2/W4 out of Open Questions, eleven Key Decisions rows, the IG-D11 lock, the WIRE-02/06/10 amendments, and the Phase 4 wire-protection statuses
coverage_id: D3
plan: 04-13
result: pass

### 63. COVERAGE.md no-external-API declaration
expected: COVERAGE.md holds the single no-external-API declaration and no table rows
coverage_id: D4
plan: 04-13
result: pass

### 64. Owner approval of the cumulative port/src diff (D-24)
expected: The owner reviewed the whole cumulative port/src change of Phase 4 and approved it before any push (D-24).
coverage_id: D1
plan: 04-14
result: pass

### 65. The four suite commands and test-9p passed twice each and Miri passed once at…
expected: The four suite commands and test-9p passed twice each and Miri passed once at the reviewed commit.
result: pass
source: automated
coverage_id: D2
plan: 04-14

### 66. The D-29 kernel byte evidence set for virtshell and q35churnshell, measured…
expected: The D-29 kernel byte evidence set for virtshell and q35churnshell, measured from a from-nothing build whose sha256 equals the recorded after ELF.
result: pass
source: automated
coverage_id: D3
plan: 04-14

### 67. The aarch64 quick pass: tier-1 on HVF and TCG, smoke, net-smoke and cpu, each…
expected: The aarch64 quick pass: tier-1 on HVF and TCG, smoke, net-smoke and cpu, each ending ALL STAGES PASS.
result: pass
source: automated
coverage_id: D4
plan: 04-14

### 68. Owner-stop page: x86_64 Linux checklist
expected: 04-OWNER-STOP.md: the ordered x86_64 Linux checklist with run_logged verdicts, host tests before boot-iso, the owner's hash commit, and the native TCG gate.
coverage_id: D5
plan: 04-14
result: pass

### 69. Branch push only on explicit owner approval
expected: The branch push happened only on the owner's explicit approval, and the pushed tip was stated before and confirmed after.
coverage_id: D1
plan: 04-15
result: pass

### 70. Eleven owner-stop verdicts recorded, each from a returned line with its HEAD=…
expected: Eleven owner-stop verdicts recorded, each from a returned line with its HEAD= value, none FAIL.
result: pass
source: automated
coverage_id: D2
plan: 04-15

### 71. x86_64 host tests: cargo xtask test (36 suites, final line 140 passed / 0…
expected: x86_64 host tests: cargo xtask test (36 suites, final line 140 passed / 0 failed) and cargo xtask test-9p (14 result lines, all aead legs), both EXIT=0.
result: pass
source: automated
coverage_id: D3
plan: 04-15

### 72. The sandboxed kernel-x64 and kernel-virt derivations build on the x86_64 host…
expected: The sandboxed kernel-x64 and kernel-virt derivations build on the x86_64 host with port linking crypto and ds.
result: pass
source: automated
coverage_id: D4
plan: 04-15

### 73. The owner re-derived the boot-iso outputHash in one flake.nix-only commit…
expected: The owner re-derived the boot-iso outputHash in one flake.nix-only commit (ca2b5ab) with no attribution line.
result: pass
source: automated
coverage_id: D5
plan: 04-15

### 74. boot-iso builds at the hash commit and rebuilds reproducibly with the…
expected: boot-iso builds at the hash commit and rebuilds reproducibly with the committed hash (no hash mismatch, no got:).
result: pass
source: automated
coverage_id: D6
plan: 04-15

### 75. Boot to rio: the KVM quick gate, the native x86_64 TCG gate and the full gate…
expected: Boot to rio: the KVM quick gate, the native x86_64 TCG gate and the full gate set each passed with their GATE PASS line.
result: pass
source: automated
coverage_id: D7
plan: 04-15

### 76. origin/main pushed once on second approval
expected: origin/main was pushed once, on the owner's second approval, to exactly the sha shown.
coverage_id: D8
plan: 04-15
result: pass

### 77. Strict per-direction sequence numbers: 0,1,2 rise by one, and a repeat, a…
expected: Strict per-direction sequence numbers: 0,1,2 rise by one, and a repeat, a skip or an earlier value is rejected after the MAC verifies with the P7 outcome (closed and zeroized)
result: pass
source: automated
coverage_id: D1
plan: 04-16

### 78. A replayed captured Twrite is not handed to the handler, and an edited…
expected: A replayed captured Twrite is not handed to the handler, and an edited cleartext FidHandle, a cross-connection packet and a reflected packet each fail the MAC
result: pass
source: automated
coverage_id: D2
plan: 04-16

### 79. Two threads sending through one shared SendHalf put sequence numbers 0 to 399…
expected: Two threads sending through one shared SendHalf put sequence numbers 0 to 399 on the wire with no gap or repeat; Session::seal and SendHalf::send share the counter; a closed session writes nothing and stays zeroized
result: pass
source: automated
coverage_id: D3
plan: 04-16

### 80. xtask::test9p lists the wire-pin files, builds the commands purely, and…
expected: xtask::test9p lists the wire-pin files, builds the commands purely, and refuses when a listed file is missing
result: pass
source: automated
coverage_id: D1
plan: 04-17

### 81. cargo xtask test-9p exists and runs the tracer pin on the host, exiting…
expected: cargo xtask test-9p exists and runs the tracer pin on the host, exiting non-zero when a leg fails
result: pass
source: automated
coverage_id: D2
plan: 04-17

### 82. The test9p-step pin records RED, GREEN-1, GREEN-2 and ADVANCED
expected: The test9p-step pin records RED, GREEN-1, GREEN-2 and ADVANCED
result: pass
source: automated
coverage_id: D3
plan: 04-17

### 83. CR-01: a failure or teardown mid-exchange leaves no live exchange secret or…
expected: CR-01: a failure or teardown mid-exchange leaves no live exchange secret or seam output; both queries answer true
result: pass
source: automated
coverage_id: D1
plan: 04-18

### 84. WR-01: the re-key returns to idle and a second threshold crossing starts a…
expected: WR-01: the re-key returns to idle and a second threshold crossing starts a second exchange; a server asks again
result: pass
source: automated
coverage_id: D2
plan: 04-18

### 85. WR-02: the shared SendHalf closes on a withheld acknowledgement at the…
expected: WR-02: the shared SendHalf closes on a withheld acknowledgement at the deadline and the session completes zeroization
result: pass
source: automated
coverage_id: D3
plan: 04-18

### 86. WR-03: seal_packet and open_packet use the P5 placement; a data page decrypts…
expected: WR-03: seal_packet and open_packet use the P5 placement; a data page decrypts alone at block counter 1
result: pass
source: automated
coverage_id: D4
plan: 04-18

### 87. IN-01 corrected only inside transport.rs: the module doc and Session::fail…
expected: IN-01 corrected only inside transport.rs: the module doc and Session::fail describe the shipped re-key state
result: pass
source: automated
coverage_id: D5
plan: 04-18

### 88. Client id 0 is reserved: insert and resolve refuse it, and a cleared or torn…
expected: Client id 0 is reserved: insert and resolve refuse it, and a cleared or torn identity word matches no client or handle
result: pass
source: automated
coverage_id: D1
plan: 04-19

### 89. A handle whose ptr is above ds::tpht::HOME_SLOTS is refused by resolve, clunk…
expected: A handle whose ptr is above ds::tpht::HOME_SLOTS is refused by resolve, clunk and mark_reclaimed with zero arena reads and unchanged bitmaps
result: pass
source: automated
coverage_id: D2
plan: 04-19

### 90. One live entry per (client_id, fid_key) across every shard, including after a…
expected: One live entry per (client_id, fid_key) across every shard, including after a fallover
result: pass
source: automated
coverage_id: D3
plan: 04-19

### 91. resolve stays exactly one arena read and zero header reads with no lock
expected: resolve stays exactly one arena read and zero header reads with no lock
result: pass
source: automated
coverage_id: D4
plan: 04-19

### 92. One leg per listed `port/tests` file: fourteen specs (seven legs, each with…
expected: One leg per listed `port/tests` file: fourteen specs (seven legs, each with its scalar-backend twin), in `PORT_TESTS` order
result: pass
source: automated
coverage_id: D1
plan: 04-20

### 93. A leg fails when any `test result: ok.` line reports zero passed tests, and…
expected: A leg fails when any `test result: ok.` line reports zero passed tests, and when it has no result line
result: pass
source: automated
coverage_id: D2
plan: 04-20

### 94. CR-02: an ordinary request or reply crossing an in-band exchange is delivered…
expected: CR-02: an ordinary request or reply crossing an in-band exchange is delivered unchanged and the exchange continues to generation 1
result: pass
source: automated
coverage_id: D1
plan: 04-21

### 95. WR-07 and P9 routing: the control and exchange messages carry NOTAG, the…
expected: WR-07 and P9 routing: the control and exchange messages carry NOTAG, the afid, and a reserved-tag violation closes the receiver
result: pass
source: automated
coverage_id: D2
plan: 04-21

### 96. CR-03: the re-key transcript absorbs the unpadded standard frames; a 21-byte…
expected: CR-03: the re-key transcript absorbs the unpadded standard frames; a 21-byte Twrite is 44 bytes and an empty one is 23
result: pass
source: automated
coverage_id: D3
plan: 04-21

### 97. RFC, PROJECT and ledger match the code's values
expected: The RFC, PROJECT.md and the wire-decision ledger state the values the code sends (P2, P3 and P9)
coverage_id: D4
plan: 04-21
result: pass

### 98. Owner told the P4 limit and wave-3 worktree setting
expected: The owner is told the P4 limit the exchange still carries and the wave-3 worktree setting
coverage_id: D5
plan: 04-21
result: pass

### 99. The failover sweep removes only the entries its snapshot named, by the…
expected: The failover sweep removes only the entries its snapshot named, by the recorded key, so a live newcomer that reused a swept slot survives (WR-05)
result: pass
source: automated
coverage_id: D1
plan: 04-22

### 100. A shard's TPHT mutation and bitmap bookkeeping are one critical section, so…
expected: A shard's TPHT mutation and bitmap bookkeeping are one critical section, so the active bitmap names exactly its live entries under churn
result: pass
source: automated
coverage_id: D2
plan: 04-22

### 101. Block indices above 0 are exercised end to end through the fid table (the…
expected: Block indices above 0 are exercised end to end through the fid table (the coincidental-reliance item): the sweep equals its BTreeSet model, resolve is one arena read, clunk clears one slot, and survivors resolve
result: pass
source: automated
coverage_id: D3
plan: 04-22

### 102. 04-REVIEW-DISPOSITION.md records the twelve gap findings CR-01..CR-03…
expected: 04-REVIEW-DISPOSITION.md records the twelve gap findings CR-01..CR-03, WR-01..WR-09 as fixed with plan+commit and leaves the seven info findings open
result: pass
source: automated
coverage_id: D1
plan: 04-23

### 103. evidence/closing-suite-gaps.md holds five pre-review PASS verdicts at the…
expected: evidence/closing-suite-gaps.md holds five pre-review PASS verdicts at the amendment HEAD 6fd201c
result: pass
source: automated
coverage_id: D2
plan: 04-23

### 104. Single review page for the gap-closure diff
expected: evidence/PORT-SRC-REVIEW-GAPS.md is the single review page for the whole gap-closure port/src diff, range GAPBASE..6fd201c
coverage_id: D3
plan: 04-23
result: pass

### 105. Owner approval at 6fd201c
expected: The owner approved the gap-closure port/src change at 6fd201c with the diff shown
coverage_id: D4
plan: 04-23
result: pass

### 106. Both suite runs, both test-9p runs and Miri pass at the reviewed commit, with…
expected: Both suite runs, both test-9p runs and Miri pass at the reviewed commit, with five closing PASS verdicts in closing-suite-gaps.md.
result: pass
source: automated
coverage_id: D1
plan: 04-24

### 107. The @04-24 kernel byte set: both kernels rebuilt from nothing, measured file…
expected: The @04-24 kernel byte set: both kernels rebuilt from nothing, measured file proven by sha256 before and after, four baseline METRIC-COMMAND lines unchanged, elf DIFFERENT, fpsimd EQUAL, x86_64 text under 2 MiB.
result: pass
source: automated
coverage_id: D2
plan: 04-24

### 108. The aarch64 quick pass: tier-1 on HVF, tier-1 on TCG, smoke, net-smoke and…
expected: The aarch64 quick pass: tier-1 on HVF, tier-1 on TCG, smoke, net-smoke and cpu each exit 0 with an ALL STAGES PASS line.
result: pass
source: automated
coverage_id: D3
plan: 04-24

### 109. Second owner-stop page and runner
expected: The second owner-stop page and runner: ordered checklist, hole-in-one runner, bash syntax clean, native TCG venue stated, hash commit restricted to flake.nix, returned-round guard.
coverage_id: D4
plan: 04-24
result: pass

### 110. Eleven second-owner-stop verdicts recorded from the owner's returned round-2…
expected: Eleven second-owner-stop verdicts recorded from the owner's returned round-2 logs, each with its decisive line and HEAD= value; none FAIL.
result: pass
source: automated
coverage_id: D1
plan: 04-25

### 111. The golden invariant for the gap-closure kernel: boot to rio under KVM, the…
expected: The golden invariant for the gap-closure kernel: boot to rio under KVM, the native TCG gate, and the full gate set, all at 0dd1037 with GATE PASS lines.
result: pass
source: automated
coverage_id: D2
plan: 04-25

### 112. `main` published only on the owner's explicit second approval, at exactly the…
expected: `main` published only on the owner's explicit second approval, at exactly the sha shown, with the verdict commit left local.
result: pass
source: automated
coverage_id: D3
plan: 04-25

## Summary

total: 112
passed: 112
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
