---
phase: "04"
slug: "guide-phase-2-9p2000-aead-transport-and-fidtable"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-25"
---

# Phase 04 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register source: the `<threat_model>` blocks of plans 04-01 to 04-25 (authored at plan time).
No `## Threat Flags` entry in any summary names a threat outside that register.
Verification depth: ASVS level 1 (the named control or pin exists in the tree, and where it is a
test, the test is defined and reports `ok`). Block threshold: `high`.

Unless a row says otherwise, "ok" means the test is defined in `port/tests/`, `crypto/` or `ds/`
and has an `ok` line in the owner's logs under `evidence/owner-stop-2/return/`. Paths starting
`evidence/` and `04-` are relative to this phase directory.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| network -> `Session::frame_len`, `receive`, `open` | Every byte is untrusted until the MAC verifies; `Size` alone is read before that | Sealed 9P packets |
| cleartext handshake -> key derivation | Plain-framed `Tversion`/`Rversion` can be altered on the path | Version string, msize |
| exchange seam -> `derive` | The 32-byte secret comes from a caller; no real exchange this phase (D-16) | Session secret |
| `port` -> `crypto` | `port` holds key values but never key bytes | Session and domain keys |
| verified body / TLV area -> decoders | Authenticated bytes from a peer that may still be malformed | 9P message bodies, extension records |
| peer -> re-key state machine | A peer chooses when to send markers, acknowledgements and control messages | Re-key control traffic |
| client -> `FidTable` | Handle bytes and capabilities are attacker-chosen within an authenticated connection | 8-byte handles, bearer capabilities |
| concurrent callers -> shard / send direction | Inserts, clunks, sweeps and senders share state | Fid entries, sequence counter |
| session memory -> later reader | A closed session's storage may be read after the close | Keys, exchange secret |
| kernel `fversion` -> `devmnt` | The only kernel entry to negotiation; it stays plain | Dialect choice |
| workspace -> kernel image | `port` links `crypto/` and `ds/` into both kernels | Code |
| test step / documents -> verdict and later phases | A green `test-9p`, the RFC and the ledgers are relied on as evidence | Test results, wire documentation |
| executor -> owner review -> remote | `port/src` changes and pushes pass owner checkpoints | Commits |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-04-01 | Tampering | A one-way wire value fixed without the owner seeing it | high | mitigate | `04-WIRE-DECISIONS.md` lines 20-29: `DECISION P1`-`P9` and `DECISION IG-D11`, all dated 2026-11-25 | closed |
| T-04-02 | Repudiation | A baseline taken after a source change, or on another venue | medium | mitigate | `evidence/kernel-bytes/before.sha256`: `base 3ce5ed8`, `venue Darwin arm64`; after-records carry the same venue | closed |
| T-04-03 | Information disclosure | Session key bytes reachable from `port` | high | mitigate | `crypto/src/session.rs:106-111` `DirectionKeys` fields private, no accessor, no `Debug` impl; `Key::bytes` is `pub(crate)` (`crypto/src/key.rs:78`) | closed |
| T-04-04 | Tampering | A header field acted on before the MAC is verified | high | mitigate | `PacketMac::verify` in `crypto/src/session.rs:278`; `aead_transport_tracer_rejects_tampering` defined and ok in owner logs `evidence/owner-stop-2/return/` | closed |
| T-04-05 | Information disclosure | Timing leak in the tag comparison | medium | mitigate | accumulator fold `acc |= computed_byte ^ tag_byte` at `crypto/src/session.rs:285` and `crypto/src/mac.rs:106`; no `==` on tag slices in `crypto/src`, `port/src/chan`, `port/src/fid.rs` | closed |
| T-04-06 | Tampering | An arch crate linking `crypto/` or `ds/` directly, outside the reviewed edge | medium | mitigate | rule R4 in `xtask/src/lintcontract.rs:336` and `:740` rejects the three arch crates | closed |
| T-04-07 | Information disclosure | Keystream reuse after a re-key that returns the same secret | high | mitigate | `derive(secret, transcript, generation)` at `crypto/src/session.rs:222`; `session_generation_changes_every_key` ok | closed |
| T-04-08 | Tampering | Version or msize altered in the cleartext handshake | high | mitigate | `session_transcript_bit_changes_keys` ok; end-to-end pin under T-04-24 | closed |
| T-04-09 | Information disclosure | Keys left in memory after teardown | medium | mitigate | `SessionKeys::zeroize` (`crypto/src/session.rs:125`), `is_zeroized`, section `# What zeroize covers` in the same file | closed |
| T-04-10 | Spoofing | A compromised or trivial exchange seam choosing the secret | medium | accept | accepted — see Accepted Risks Log; `crypto/src/session.rs:152` states that `crypto/` holds no default domain secret | closed |
| T-04-11 | Denial of service | An oversized or runt `Size` causing an over-read or a large allocation | high | mitigate | `frame_size` in `port/src/chan/wire.rs`; `aead_wire_header_size_bounds` ok | closed |
| T-04-12 | Tampering | Decoding fields of an unauthenticated packet | high | mitigate | `Verified` in `port/src/chan/transport.rs`, built after `PacketMac::verify` | closed |
| T-04-13 | Elevation of privilege | A packet carrying both a stateful handle and a capability | medium | mitigate | `check_header` in `port/src/chan/wire.rs`; `aead_wire_header_rules_after_verification` ok | closed |
| T-04-14 | Denial of service | A length field larger than the remaining bytes causing an over-read or a large allocation | high | mitigate | `aead_wire_messages_reject_truncated_and_trailing` ok | closed |
| T-04-15 | Information disclosure | Stale buffer bytes sent as data-page padding | medium | mitigate | zero padding written at `port/src/chan/msg.rs:778-780`; `aead_wire_messages_data_bounds` ok | closed |
| T-04-16 | Tampering | A count that disagrees with the data area making the receiver deliver padding as file data | medium | mitigate | `count <= data_len < count + PAGE_LEN` enforced at `port/src/chan/msg.rs:884-897` | closed |
| T-04-17 | Denial of service | A record length running past the TLV area | high | mitigate | fixed length per type at `port/src/chan/tlv.rs:211-215`; `aead_tlv_rejects_mask_area_mismatch` ok | closed |
| T-04-18 | Tampering | An unknown or duplicated extension smuggling a second meaning past the parser | medium | mitigate | `port/src/chan/tlv.rs:178` (mask outside `EXT_KNOWN`), `:201-206` (duplicate, out of order) | closed |
| T-04-19 | Elevation of privilege | A volatility class with two bits set read as a more permissive cache class | medium | mitigate | `aead_tlv_rejects_bad_volatility_class` ok | closed |
| T-04-20 | Tampering | Splicing a payload onto another fid by editing the cleartext handle | high | mitigate | `aead_transport_splice_edited_fidhandle_fails_mac` ok | closed |
| T-04-21 | Tampering | Replay or reordering of sealed packets | high | mitigate | `aead_transport_sequence_rejects_repeat_skip_and_earlier`, `aead_transport_splice_replayed_twrite_is_dropped`, `aead_transport_send_order_under_contention` ok | closed |
| T-04-22 | Spoofing | A packet moved across connections or reflected to its sender | high | mitigate | `aead_transport_splice_cross_connection_and_reflection_fail` ok | closed |
| T-04-23 | Tampering | Acting on fields of a packet whose MAC failed | high | mitigate | `aead_transport_mac_failure_every_byte` ok | closed |
| T-04-24 | Tampering | Version or msize altered in the cleartext handshake | high | mitigate | `aead_transport_handshake_altered_rversion_fails_first_packet` ok | closed |
| T-04-25 | Information disclosure | Keystream reuse when the counter returns to 0 or wraps | high | mitigate | `SEQ_HARD_LIMIT` in `port/src/chan/transport.rs`; `aead_transport_rekey_same_secret_still_changes_keys`, `aead_transport_rekey_counter_never_wraps` ok | closed |
| T-04-26 | Tampering | An old-key packet accepted after a cutover | high | mitigate | `aead_transport_rekey_no_old_key_packet_after_marker` ok | closed |
| T-04-27 | Denial of service | A peer that never acknowledges a re-key | medium | mitigate | `ack_deadline` in `port/src/chan/transport.rs`; `aead_transport_rekey_ack_timeout_closes` ok | closed |
| T-04-28 | Spoofing | An acknowledgement from a peer that did not derive the new keys | medium | mitigate | `verify_rekey_confirm` at `crypto/src/session.rs:350`, constant time | closed |
| T-04-29 | Elevation of privilege | A clunked handle still resolving to its old entry | high | mitigate | `FID_LIVE` in `port/src/fid.rs`; `tpht_remove_clears_the_arena_slot` ok | closed |
| T-04-30 | Tampering | A volatility class read as a Plan 9 type bit, turning a volatile file into a directory or the reverse | high | mitigate | `aead_fid_qid_class_values_collide_with_plan9_type_bits` ok | closed |
| T-04-31 | Denial of service | An out-of-range slot id read by the sweep | low | mitigate | `tpht_read_slot_is_bounds_checked` ok | closed |
| T-04-32 | Elevation of privilege | One client presenting another client's handle | high | mitigate | `aead_fid_resolve_rejects_wrong_client_epoch_and_shard` ok | closed |
| T-04-33 | Elevation of privilege | A stale handle used after clunk or failover | high | mitigate | `aead_fid_resolve_clunked_handle_does_not_resolve` ok | closed |
| T-04-34 | Elevation of privilege | A forged capability | medium | mitigate | constant-time `StatelessCap` verify at `crypto/src/session.rs:163`; `aead_fid_stateless_every_flipped_byte_rejects` ok | closed |
| T-04-35 | Denial of service | An out-of-range `shard_id` indexing past the shard array | medium | mitigate | `self.shards.get(handle.shard_id as usize)` with early return at `port/src/fid.rs:770` | closed |
| T-04-36 | Tampering | Downgrade to plain 9P2000 by altering `Rversion` | high | mitigate | `aead_negotiate_plain_answer_to_aead_offer_is_an_error` ok | closed |
| T-04-37 | Denial of service | A regression of the plain mount path breaking the boot to rio | high | mitigate | `Mnt::negotiate` and `fversion` kept; `devmnt_test.rs` ok in owner logs; boot gates PASS at both owner stops | closed |
| T-04-38 | Elevation of privilege | The aead engine reachable from a booted kernel before a real exchange exists | medium | mitigate | `negotiate_dialect` and `Dialect::` have no call site in `port/src` outside `devmnt.rs`, nor in `aarch64`, `x86_64`, `riscv64` | closed |
| T-04-39 | Repudiation | `test-9p` passing while running nothing (renamed file, filter matching nothing) | medium | mitigate | `preflight` (`xtask/src/test9p.rs:143`), `check_leg_output` (`:191`); 10 `test9p_*` unit tests ok (run 2026-11-25) | closed |
| T-04-40 | Tampering | Wire pins proven only on the hardware backends while the kernel runs the scalar ones | medium | mitigate | `test9p_has_a_scalar_backend_twin_for_every_leg` (`xtask/src/test9p.rs:366`), 14 legs asserted | closed |
| T-04-41 | Repudiation | A decision recorded as locked without the owner's answer | medium | mitigate | `.planning/PROJECT.md:129` old IG-D11 row kept and marked superseded; `:168` new row cites `DECISION IG-D11: lock-amended` | closed |
| T-04-42 | Tampering | A documented wire value that differs from the pinned code, misleading a second implementation | medium | mitigate | `.planning/rfc-9p2000-aead.md`; the independent wire pins of T-04-50, T-04-53 and T-04-65 ok | closed |
| T-04-43 | Elevation of privilege | `port/src` code reaching a push without the owner's review | high | mitigate | `evidence/closing-suite.md:58` `VERDICT port-src-review: APPROVED HEAD=f72cc2a` | closed |
| T-04-44 | Denial of service | FP or SIMD instructions introduced into a kernel built without FP/SIMD | high | mitigate | `evidence/kernel-bytes/KERNEL-BYTES.md` FP/SIMD verdicts `EQUAL` | closed |
| T-04-45 | Repudiation | Evidence taken at a commit other than the one pushed | medium | mitigate | `evidence/kernel-bytes/after-04-14.sha256`: `after 3398df4`; reviewed sha in `closing-suite.md` | closed |
| T-04-46 | Elevation of privilege | A push made without the owner's approval, or of more than was approved | high | mitigate | `evidence/owner-stop/RESULTS.md`: `stop-branch-push` and `stop-main-push` PASS, `origin/main` recorded unchanged before the push | closed |
| T-04-47 | Repudiation | A result measured at a commit other than the pushed tip or the hash commit | medium | mitigate | `HEAD=` line in every owner-stop log, checked in `evidence/owner-stop/RESULTS.md` | closed |
| T-04-48 | Tampering | A branch commit other than the `flake.nix` hash commit and the return files reaching `main` | high | mitigate | `stop-bootiso-hash-commit: PASS` in `evidence/owner-stop/RESULTS.md` | closed |
| T-04-49 | Denial of service | A kernel that no longer boots to rio reaching `main` | high | mitigate | `stop-gate-kvm`, `stop-gate-tcg`, `stop-gate-full` PASS in `evidence/owner-stop/RESULTS.md` | closed |
| T-04-50 | Tampering | A keystream placement or MAC input that both ends share but that differs from the documented wire, found only when Phase 5 or a second implementation reads pages | medium | mitigate | `aead_transport_wire_data_pages_decrypt_alone_at_their_block_counter`, `aead_transport_wire_tag_is_packet_mac_of_all_bytes_before_it` ok | closed |
| T-04-51 | Repudiation | `test-9p` reporting green after a listed test file was renamed or deleted | medium | mitigate | `test9p_refuses_when_a_listed_file_is_missing` ok | closed |
| T-04-52 | Repudiation | A failing leg's status dropped or masked by a later leg | medium | mitigate | `run` returns on the first error (`xtask/src/test9p.rs:169-178`, `:222-231`) | closed |
| T-04-53 | Tampering | Re-key inputs (transcript bytes, their order, the generation, the direction of the confirm key) that both ends share but that differ from the documented wire, found only by a second implementation | medium | mitigate | `aead_transport_rekey_wire_keys_and_confirm_equal_test_side_derivation` ok | closed |
| T-04-54 | Information disclosure | The 32-byte exchange secret and the seam output left live in a failed or torn-down session | high | mitigate | `keys_zeroized()` in `port/src/chan/transport.rs`; `aead_transport_rekey_failure_mid_exchange_zeroizes_exchange_secret`, `aead_transport_rekey_teardown_mid_exchange_zeroizes_exchange_secret` ok | closed |
| T-04-55 | Denial of service | A connection that can re-key only once and then runs to the hard counter limit and closes | medium | mitigate | `handle_ack` in `port/src/chan/transport.rs`; `aead_transport_rekey_second_threshold_starts_a_second_exchange` ok | closed |
| T-04-56 | Denial of service | A peer that withholds the acknowledgement from a connection sending only through `SendHalf` | medium | mitigate | `aead_transport_send_order_ack_deadline_closes` ok | closed |
| T-04-57 | Tampering | Two keystream layouts behind one public API, so a helper returns mis-decrypted plaintext after a good MAC | medium | mitigate | `crypt_body` in `port/src/chan/transport.rs`; `aead_transport_tracer_data_body_uses_session_keystream_placement` ok | closed |
| T-04-58 | Elevation of privilege | A clunked fid resolving through a torn read of its slot (use after clunk) | medium | mitigate | `aead_fid_resolve_torn_clunk_image_never_matches`, `aead_fid_insert_refuses_client_id_zero` ok | closed |
| T-04-59 | Tampering | A client editing its handle's pointer byte so the table drops or marks another slot's bitmap id | medium | mitigate | home-window check at `port/src/fid.rs:773`; `aead_fid_resolve_out_of_window_ptr_is_rejected` ok | closed |
| T-04-60 | Denial of service | A leaked duplicate entry for one key that survives until a sweep | low | mitigate | `aead_fid_insert_duplicate_after_fallover_is_refused` ok | closed |
| T-04-61 | Repudiation | A listed wire-pin file emptied of tests while `test-9p` still reports green | medium | mitigate | `test9p_runs_one_leg_per_listed_port_file`, `test9p_rejects_a_leg_with_one_zero_test_result_line` ok (run 2026-11-25) | closed |
| T-04-62 | Tampering | An ordinary reply absorbed into the re-key transcript, so the next generation's keys depend on user data and the user's reply is lost | high | mitigate | `aead_transport_rekey_ordinary_read_during_exchange_is_delivered`, `aead_negotiate_end_to_end_read_during_rekey_is_served` ok | closed |
| T-04-63 | Denial of service | Ordinary traffic that crosses an exchange closing the connection | high | mitigate | `aead_transport_rekey_ordinary_write_at_server_is_not_consumed` ok | closed |
| T-04-64 | Spoofing | A message with the reserved tag but another fid, or a control message with an ordinary tag, steering the re-key state machine | medium | mitigate | `NOTAG` checks at `port/src/chan/transport.rs:1200` and `:1221`; `aead_transport_rekey_exchange_tag_with_wrong_fid_closes`, `aead_transport_rekey_control_with_wrong_tag_closes` ok | closed |
| T-04-65 | Tampering | Re-key inputs that both ends of this implementation share but that differ from the documented wire (padded data, undocumented tags) | medium | mitigate | `exchange_frame` in `port/src/chan/transport.rs`; RFC notes in `.planning/rfc-9p2000-aead.md` | closed |
| T-04-66 | Denial of service | A sweep removing a live entry that reused a swept slot, or the entry of key (0, 0) when a slot reads as zeros | medium | mitigate | removal by recorded key at `port/src/fid.rs:880` and `:925`; `aead_fid_sweep_concurrent_clunk_and_reinsert_keeps_the_newcomer` ok | closed |
| T-04-67 | Tampering | Bitmap bookkeeping that drifts from the live entries (a late clunk dropping a newcomer's id), so a later sweep misses or mis-collects | medium | mitigate | `aead_fid_sweep_bookkeeping_matches_live_entries_under_churn` ok | closed |
| T-04-68 | Elevation of privilege | Gap-closure `port/src` code reaching a push without the owner's review | high | mitigate | `evidence/closing-suite-gaps.md:98` `VERDICT port-src-review-gaps: APPROVED HEAD=6fd201c` | closed |
| T-04-69 | Repudiation | A finding recorded `fixed` with no pin behind it | medium | mitigate | `ADVANCED` commits in `evidence/PINS-04-16.md` to `PINS-04-22.md`; `04-REVIEW-DISPOSITION.md` | closed |
| T-04-70 | Denial of service | FP or SIMD instructions introduced into a kernel built without FP/SIMD by the gap fixes | high | mitigate | `evidence/kernel-bytes/KERNEL-BYTES-GAPS.md:99-100` `fpsimd-virtshell@04-24: EQUAL`, `fpsimd-q35churnshell@04-24: EQUAL` | closed |
| T-04-71 | Repudiation | Evidence taken at a commit other than the reviewed one, or stale `@04-14` evidence presented for the gap-closure kernel | medium | mitigate | `evidence/kernel-bytes/after-04-24.sha256`: `after 8df21a3`, same venue; `@04-14` set kept as history | closed |
| T-04-72 | Tampering | A runner that edits more than the one `outputHash` line on the owner's host | medium | mitigate | `04-OWNER-STOP-2-RUNNER.sh:90` (exactly one `outputHash` line), `:93` (+2/-1 diff), `:97` (hash commit touches `flake.nix` alone); `bash -n` passes | closed |
| T-04-73 | Elevation of privilege | A push made without the owner's approval, or of more than was approved | high | mitigate | `evidence/owner-stop-2/RESULTS.md`: `stop2-branch-push` and `stop2-main-push` PASS, `origin/main` recorded unchanged before the push | closed |
| T-04-74 | Repudiation | A result measured at a commit other than the pushed tip or the hash commit | medium | mitigate | `HEAD=0dd1037` on every verdict in `evidence/owner-stop-2/RESULTS.md` | closed |
| T-04-75 | Tampering | A branch commit other than the `flake.nix` hash commit and the return files reaching `main` | high | mitigate | `stop2-bootiso-hash-commit: PASS` (no hash commit was needed) in `evidence/owner-stop-2/RESULTS.md` | closed |
| T-04-76 | Denial of service | A gap-closure kernel that no longer boots to rio reaching `main` | high | mitigate | `stop2-gate-kvm`, `stop2-gate-tcg`, `stop2-gate-full` PASS in `evidence/owner-stop-2/RESULTS.md` | closed |
| T-04-SC | Tampering | npm/pip/cargo installs (recorded in all 25 plans) | low | accept | accepted — see Accepted Risks Log; the only `Cargo.lock` change in the phase is commit `f78f702` (two path-dependency lines, no new crate) | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-04-01 | T-04-10 | The exchange seam's secret is not authenticated this phase; a real key exchange is out of scope (D-16) and stays outside the IG-D11 lock (N5). No default secret exists in `crypto/`, and no booted kernel reaches the aead engine (T-04-38). | Plan-time disposition, plan 04-03 (decision D-16) | 2026-11-25 |
| AR-04-02 | T-04-SC | No package was added in any plan of the phase; `Cargo.lock` gained two path-dependency lines for `crypto` and `ds` and no crate. | Plan-time disposition, plans 04-01 to 04-25 | 2026-11-25 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-25 | 77 | 77 | 0 | /gsd-secure-phase (orchestrator, L1 evidence check at `e684c09`) |

Notes on this audit:

- 75 threats are `mitigate`, 2 are `accept`. All 49 tests the register names are defined in
  source; 46 have `ok` lines in the owner-stop-2 logs, and the `test9p_*` runner tests were run
  in this audit (`cargo test -p xtask test9p_`: 10 passed, 0 failed).
- The security auditor subagent was not spawned: with no open threat, a plan-time register and
  ASVS level 1, the workflow's short-circuit rule applies. This audit confirms that each control
  exists; it does not re-review the code for threats outside the register.
- Process threats (T-04-01, -02, -41, -43 to -49, -68 to -76) are closed on committed evidence
  files and verdict lines, which this audit read and did not re-run.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-25
