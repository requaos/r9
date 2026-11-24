---
phase: "4"
slug: "guide-phase-2-9p2000-aead-transport-and-fidtable"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-24"
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `04-RESEARCH.md` section "Validation Architecture". File and test names are the research proposal; the plans fix the final names.

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

Task IDs are assigned by the plans; rows are keyed by requirement until then.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| TBD | TBD | TBD | WIRE-01 | — | Header fields at pinned offsets, 28 bytes total; encode then decode is identity; a hand-written byte vector decodes to the expected fields | unit | `cargo test -p port --test aead_wire header` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-01 | — | Compendium listing 9.9 rewritten with a dated note; RFC amendment notes present | doc check | `grep -c "Amended 2026-" .planning/rfc-9p2000-aead.md` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-02 | TBD | Six extensions round-trip; every mask combination parses; `ExtTLVs Length == 0` iff mask 0; unknown bit, wrong order, duplicate, bad length and non-zero pad each rejected; parser allocates nothing | unit | `cargo test -p port --test aead_tlv` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-03 | TBD | Flipping any byte before the MAC, or of the MAC, closes the connection with zero bytes parsed; later seal and open return closed | integration | `cargo test -p port --test aead_transport mac_failure` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-03 | TBD | After failure every key byte and the nonce prefix are zero | unit | `cargo test -p crypto --lib session` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-04 | TBD | Counter starts at 0 and increments by 1; previous, skipped and repeated values rejected; re-key fires at a low threshold; marker then acknowledgement; no old-key packet accepted after the marker; acknowledgement timeout closes; counter never wraps | integration | `cargo test -p port --test aead_transport rekey` and `cargo test -p port --test aead_transport sequence` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-05 | TBD | Sealed `Tattach` before `Tauth` refused; derivation known-answer vector; the two directions' keys differ; altered `Rversion` gives a failing first packet; teardown zeroizes | unit + integration | `cargo test -p crypto --test session` and `cargo test -p port --test aead_transport handshake` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-06 | — | Every base opcode (including `Rerror`, `Tflush`, `Rflush`) and 130 to 137 round-trips; registry table pinned; data bodies are whole pages with the true count | unit | `cargo test -p port --test aead_wire messages` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-07 | TBD | Sizes 8, 24, 64 and `align_of::<FidEntry>() == 64`; wire forms round-trip; a stateful handle is never all zero; capability bit with a non-zero header handle rejected | unit | `cargo test -p port --test aead_fid layout` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-08 | TBD | Counting probe shows one read for a resolve; wrong client, wrong epoch and clunked handle do not resolve; two tables with one key verify each other's capabilities with zero FID memory; a different key rejects | unit | `cargo test -p port --test aead_fid resolve` and `cargo test -p port --test aead_fid stateless` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-09 | — | Sweep equals a `BTreeSet` model of `active \ reclaimed` on randomized inputs | unit | `cargo test -p port --test aead_fid sweep` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-10 | — | 13-byte `Qid` round trip; three class values in their own byte (D-27); two classes at once rejected; `#G` path formula at boundary values | unit | `cargo test -p port --test aead_fid qid` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-11 | TBD | Replay dropped; edited `FidHandle` fails the MAC; cross-connection and reflected packets fail | integration | `cargo test -p port --test aead_transport splice` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | WIRE-12 | TBD | aead offer reaches the engine; plain offer yields a working `Mnt`; downgrade is an error; existing suite unchanged and green | integration | `cargo test -p port --test aead_negotiate` and `cargo test -p port --test devmnt_test` | ❌ W0 (new file); ✅ `devmnt_test.rs` | ⬜ pending |
| TBD | TBD | TBD | WIRE-13 | — | The subcommand exists, runs the listed files, and refuses when one is missing | unit + smoke | `cargo test -p xtask test9p` and `nix develop -c cargo xtask test-9p` | ❌ W0 | ⬜ pending |
| TBD | TBD | TBD | (lint rule R4) | — | An arch crate naming `crypto` is still rejected; `port` naming it is accepted | unit | `cargo test -p xtask lintcontract` | ✅ (fixtures change) | ⬜ pending |
| TBD | TBD | TBD | (D-25) | — | Kernel byte evidence recorded; FP/SIMD scan equal to baseline | evidence | grep of `VERDICT` lines in `evidence/kernel-bytes/KERNEL-BYTES.md` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crypto/src/session.rs` and `crypto/tests/session.rs` — WIRE-05, WIRE-03 (zeroize)
- [ ] `port/tests/common/aead_link.rs` — in-memory duplex link, client and server ends, fake clock, decode counter
- [ ] `port/tests/aead_wire.rs`, `aead_tlv.rs`, `aead_transport.rs`, `aead_fid.rs`, `aead_negotiate.rs`
- [ ] `xtask/src/test9p.rs` and its subcommand — WIRE-13
- [ ] `xtask/src/lintcontract.rs` rule R4 and fixtures — must land with the `port` → `crypto`/`ds` dependency edge
- [ ] Signature stubs for `chan`, `fid` and `session` so RED pins fail on behaviour, not on a missing module
- [ ] `evidence/kernel-bytes/before.sha256` at the phase's base commit

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

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
