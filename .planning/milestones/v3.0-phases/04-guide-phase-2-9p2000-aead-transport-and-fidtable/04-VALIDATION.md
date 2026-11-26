---
phase: "4"
slug: "guide-phase-2-9p2000-aead-transport-and-fidtable"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-24"
validated_at: "2026-11-25"
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `04-RESEARCH.md` section "Validation Architecture". File and test names are the research proposal; the plans fix the final names.

> **Reconciliation note (2026-11-25 — DEBT-02 / v3.0 audit, Phase 5 D-34):** this record was
> reconciled after the phase completed, from `status: draft` / `nyquist_compliant: false` /
> `wave_0_complete: false` to the validated state above. The per-task verification map below is
> filled in from the completed verification report (`04-VERIFICATION.md`, `passed`, 109/109
> must-haves and 42/42 gap-closure truths, 2026-11-25T16:22:17Z) and the UAT record (`04-UAT.md`,
> 112/112 passed). Editing this archived file is explicitly authorized by Phase 5 D-34 and does
> not alter archived v3.0 requirement rows.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in tests through `cargo xtask`; Miri for `ds/src/tpht/` when it is edited |
| **Config file** | none; workspace `Cargo.toml` and `.planning/config.json` (`build_command`, `test_command`) |
| **Quick run command** | `nix develop -c cargo xtask test-9p` once it exists; until then `nix develop -c cargo test -p crypto --lib session` or `nix develop -c cargo test -p port --test <file>` |
| **Full suite command** | `nix develop -c cargo xtask test && nix develop -c cargo test -p xtask && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy` (plus `nix develop -c cargo xtask miri` when `ds/src/tpht/` changed) |
| **Estimated runtime** | quick: seconds once built; full: a few minutes; Miri 230 to 402 seconds (Phase 3 measurement) |

---

## Sampling Rate

- **After every task commit:** Run the single test file the task touches, then `nix develop -c cargo xtask test`
- **After every plan wave:** Run the full suite command; add `cargo xtask miri` if `ds/src/tpht/` changed in the wave
- **Before `/gsd-verify-work`:** Full suite green twice (IG-V01); `cargo xtask test-9p` green twice; kernel byte evidence recorded; aarch64 quick pass; then the owner stop
- **Max feedback latency:** about 120 seconds once built (Miri excepted, as in Phase 3)

---

## Per-Task Verification Map

Task IDs were assigned by the plans; the rows below are reconciled (DEBT-02, 2026-11-25) with the
delivering plans, their waves, and the results recorded in `04-VERIFICATION.md` and `04-UAT.md`
(112/112). Every command listed ran green in the closing suite (`evidence/closing-suite-gaps.md`,
two consecutive runs) and again in the verification pass.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 04-04 T1 | 04-02, 04-04 | 2, 3 | WIRE-01 | — | Header fields at pinned offsets, 28 bytes total; encode then decode is identity; a hand-written byte vector decodes to the expected fields | unit | `cargo test -p port --test aead_wire header` | ✅ | ✅ green (19×2 legs) |
| 04-13 T1 | 04-13 | 9 | WIRE-01 | — | Compendium listing 9.9 rewritten with a dated note; RFC amendment notes present | doc check | `grep -c "Amended 2026-" .planning/rfc-9p2000-aead.md` | ✅ | ✅ green |
| 04-06 T1 | 04-06 | 4 | WIRE-02 | T-04-17, T-04-18, T-04-19 | Six extensions round-trip; every mask combination parses; `ExtTLVs Length == 0` iff mask 0; unknown bit, wrong order, duplicate, bad length and non-zero pad each rejected; parser allocates nothing | unit | `cargo test -p port --test aead_tlv` | ✅ | ✅ green (11×2 legs) |
| 04-07 T2 | 04-07, 04-16 | 5, 6 | WIRE-03 | T-04-23 | Flipping any byte before the MAC, or of the MAC, closes the connection with zero bytes parsed; later seal and open return closed | integration | `cargo test -p port --test aead_transport mac_failure` | ✅ | ✅ green (55×2 legs) |
| 04-03 T1 | 04-03, 04-18 | 3, 1 | WIRE-03 | T-04-09 | After failure every key byte and the nonce prefix are zero | unit | `cargo test -p crypto --lib session` | ✅ | ✅ green (CR-01 closed; mid-exchange pins pass) |
| 04-08 T1 | 04-08, 04-16, 04-18, 04-21 | 7, 6, 1, 2 | WIRE-04 | T-04-21, T-04-25, T-04-26, T-04-27, T-04-28 | Counter starts at 0 and increments by 1; previous, skipped and repeated values rejected; re-key fires at a low threshold; marker then acknowledgement; no old-key packet accepted after the marker; acknowledgement timeout closes; counter never wraps | integration | `cargo test -p port --test aead_transport rekey` and `cargo test -p port --test aead_transport sequence` | ✅ | ✅ green (WR-01/02/07, CR-02/03 closed) |
| 04-07 T1 | 04-02, 04-03, 04-07 | 2, 3, 5 | WIRE-05 | T-04-07, T-04-08, T-04-24 | Sealed `Tattach` before `Tauth` refused; derivation known-answer vector; the two directions' keys differ; altered `Rversion` gives a failing first packet; teardown zeroizes | unit + integration | `cargo test -p crypto --test session` and `cargo test -p port --test aead_transport handshake` | ✅ | ✅ green |
| 04-05 T1 | 04-04, 04-05 | 3, 4 | WIRE-06 | — | Every base opcode (including `Rerror`, `Tflush`, `Rflush`) and 130 to 137 round-trips; registry table pinned; data bodies are whole pages with the true count | unit | `cargo test -p port --test aead_wire messages` | ✅ | ✅ green (19×2 legs) |
| 04-09 T1 | 04-09, 04-10 | 3, 4 | WIRE-07 | T-04-13, T-04-30 | Sizes 8, 24, 64 and `align_of::<FidEntry>() == 64`; wire forms round-trip; a stateful handle is never all zero; capability bit with a non-zero header handle rejected | unit | `cargo test -p port --test aead_fid layout` | ✅ | ✅ green (36×2 legs) |
| 04-10 T1 | 04-09, 04-10, 04-19, 04-22 | 3, 4, 1, 2 | WIRE-08 | T-04-29 | Counting probe shows one read for a resolve; wrong client, wrong epoch and clunked handle do not resolve; two tables with one key verify each other's capabilities with zero FID memory; a different key rejects | unit | `cargo test -p port --test aead_fid resolve` and `cargo test -p port --test aead_fid stateless` | ✅ | ✅ green (WR-04/06 closed; (1,0) read pinned) |
| 04-10 T2 | 04-10, 04-22 | 4, 2 | WIRE-09 | — | Sweep equals a `BTreeSet` model of `active \ reclaimed` on randomized inputs | unit | `cargo test -p port --test aead_fid sweep` | ✅ | ✅ green (WR-05 closed; 64 multi-block seeds) |
| 04-09 T2 | 04-09 | 3 | WIRE-10 | — | 13-byte `Qid` round trip; three class values in their own byte (D-27); two classes at once rejected; `#G` path formula at boundary values | unit | `cargo test -p port --test aead_fid qid` | ✅ | ✅ green |
| 04-16 T1 | 04-07, 04-16 | 5, 6 | WIRE-11 | T-04-20, T-04-22 | Replay dropped; edited `FidHandle` fails the MAC; cross-connection and reflected packets fail | integration | `cargo test -p port --test aead_transport splice` | ✅ | ✅ green |
| 04-11 T1 | 04-11 | 8 | WIRE-12 | T-04-24 | aead offer reaches the engine; plain offer yields a working `Mnt`; downgrade is an error; existing suite unchanged and green | integration | `cargo test -p port --test aead_negotiate` and `cargo test -p port --test devmnt_test` | ✅ (both files) | ✅ green (11×2 and 18/18) |
| 04-12 T1 | 04-12, 04-17, 04-20 | 9, 3, 1 | WIRE-13 | — | The subcommand exists, runs the listed files, and refuses when one is missing | unit + smoke | `cargo test -p xtask test9p` and `nix develop -c cargo xtask test-9p` | ✅ | ✅ green (10/10 units; 14 legs, WR-08 closed) |
| 04-02 T2 | 04-02 | 2 | (lint rule R4) | T-04-06 | An arch crate naming `crypto` is still rejected; `port` naming it is accepted | unit | `cargo test -p xtask lintcontract` | ✅ | ✅ green |
| 04-14 T1 | 04-14, 04-24 | 10, 4 | (D-25) | T-04-02 | Kernel byte evidence recorded; FP/SIMD scan equal to baseline | evidence | grep of `VERDICT` lines in `evidence/kernel-bytes/KERNEL-BYTES.md` | ✅ | ✅ green (04-24 gap set; see DEBT-03 correction in `KERNEL-BYTES-GAPS.md`) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky — all rows final at 2026-11-25 (DEBT-02)*

---

## Wave 0 Requirements

- [x] `crypto/src/session.rs` and `crypto/tests/session.rs` — WIRE-05, WIRE-03 (zeroize)
- [x] `port/tests/common/aead_link.rs` — in-memory duplex link, client and server ends, fake clock, decode counter
- [x] `port/tests/aead_wire.rs`, `aead_tlv.rs`, `aead_transport.rs`, `aead_fid.rs`, `aead_negotiate.rs`
- [x] `xtask/src/test9p.rs` and its subcommand — WIRE-13
- [x] `xtask/src/lintcontract.rs` rule R4 and fixtures — must land with the `port` → `crypto`/`ds` dependency edge
- [x] Signature stubs for `chan`, `fid` and `session` so RED pins fail on behaviour, not on a missing module
- [x] `evidence/kernel-bytes/before.sha256` at the phase's base commit

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The system still boots to rio | WIRE-12 | Needs the owner's x86_64 Linux KVM host | `nix develop -c cargo xtask gate --arch x86-64 --accel kvm` at the end-of-phase stop |
| `boot-iso` `outputHash` re-derived and committed; TCG gates; full gate set | D-25 | Owner's x86_64 Linux host is the approved venue; the owner makes the hash commit | Per the Phase 2 end-of-phase stop procedure, transcribed in `04-RESEARCH.md` |
| Branch push and `main` push | standing rule | Owner approval each time | Show unpushed commits, wait for approval |
| Phase-end code review of the whole `port/src` change | D-24 | Owner review replaces per-plan diff stops this phase | Present the cumulative `port/src` diff before any push |
| IG-D11 lock | DSGN-05 follow-up | Owner-only decision | Owner checkpoint in the plan |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-11-25 — reconciled under DEBT-02 (Phase 5 D-34) from
`04-VERIFICATION.md` (`passed`, 109/109 must-haves and 42/42 gap-closure truths) and `04-UAT.md`
(112/112). The sign-off records the completed phase; the manual-only rows above were executed at
the two owner stops (`evidence/owner-stop/`, `evidence/owner-stop-2/`).
