---
phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable
plan: "11"
subsystem: kernel-transport-devmnt
tags: [9p2000-aead, devmnt, negotiate-dialect, downgrade-fail-closed, handoff, transcript-binding, fidtable, stateless-cap, rekey, end-to-end, tdd]

# Dependency graph
requires:
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-07, 04-08)
    provides: "port::chan::transport::{Handshake, Session, SendHalf, rekey_step, teardown, keys_zeroized}; the in-band re-key and strict per-direction cutover"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-09, 04-10)
    provides: "port::fid::{FidTable, FidEntry, FidHandle, StatefulHandle, StatelessCap}; issue_cap/verify_cap/resolve/clunk and the single-read resolve"
  - phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable (04-04, 04-05)
    provides: "port::chan::{wire, msg, tlv} — the header, the message codec with the P5 data layout, ExtMask and the capability TLV"
provides:
  - "port::devmnt::{Dialect, Negotiated, Mnt::negotiate_dialect}: the version selection point, fail-closed on a plain answer to an aead offer"
  - "port::chan::transport::Handoff with into_handshake: the agreed session's io halves, msize and exact Tversion/Rversion bytes, absorbed into the transcript"
  - "the plain path pinned byte-for-byte: Mnt::negotiate delegates with Dialect::Plain, fversion is untouched, devmnt_test has no diff against the phase base"
  - "port/tests/aead_negotiate.rs: six negotiation pins and four end-to-end pins over a server end whose fids live in a FidTable"
  - "port/tests/common/aead_link.rs: E2eClient/E2eServer, the canned tree and the negotiation driver over the real devmnt code"
affects: [04-14 (phase-end code review of the whole port/src change), 04-15 (the boot gate: the kernel still offers plain 9P2000), 04-17 test-9p, phase 06 (the legacy gateway is the only downgrade path)]

actuals:
  tokens: 16281
  tasks: 2
  commits: 6
  plan_head_before: 04d6f3c398d6e5c6cd358bd3ec6a2c39d5cbc88c
  plan_head_after: d7b7b2940c89562def91f09b6685c60c27563bb3

tech-stack:
  added: []
  patterns:
    - "the dialect selection point is one function: the version string is the only difference between the branches, and the msize rules are shared"
    - "the version exchange runs directly on the two io halves (one write, one framed reply) so the aead branch never has to extract them from a half-built Mnt; BusyCell has no take() and devmnt is forbid(unsafe_code)"
    - "a downgrade is an error naming the plain answer; an unknown version keeps the existing bad-version error"
    - "Handoff::into_handshake sets the session's msize from the handoff and absorbs Tversion then Rversion into the unkeyed BLAKE3 transcript (D-17)"
    - "the e2e server is driven synchronously from the client's MntIo write half, so negotiate_dialect sees the same one-whole-message-per-write peer the kernel pipe provides"
    - "the server issues a capability in the Ropen reply with an all-zero header handle (D-12); a Tread carrying it resolves through verify_cap and touches no fid memory; a failed verify closes the connection (P8)"
    - "the re-key pin drives one in-band exchange plus both strict cutovers inside the same negotiated session, then reads the same bytes under generation 1"

key-files:
  created:
    - port/tests/aead_negotiate.rs
    - .planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/evidence/PINS-04-11.md
  modified:
    - port/src/devmnt.rs
    - port/src/chan/transport.rs
    - port/tests/common/aead_link.rs

key-decisions:
  - "negotiate_dialect performs the version exchange on the two halves before any Mnt exists, instead of building an Mnt and taking the halves back out: BusyCell has no safe extraction and devmnt.rs is forbid(unsafe_code). The plain path is unchanged in signature, behaviour and bytes."
  - "Handoff::into_handshake overwrites config.msize with the negotiated msize, so the session's packet bound is always the value both ends agreed."
  - "A plain answer to an aead offer returns 'devmnt: server answered plain 9P2000 to a 9P2000.aead offer'; the legacy fallback stays Phase 6's gateway."
  - "The e2e fixture's server owns the FidTable and one client_id per connection; the canned tree is a root directory and one file whose bytes are returned in one padded page with the true count."
  - "The capability is issued on Ropen, which is the point where a server knows the qid and mode; the client presents it on a Tread with a zero header handle."
  - "The cross-connection pin presents connection A's attach handle on connection B, whose table has no entry for that fid: the handle carries no client_id, so the rejection comes from resolve's entry identity check."

patterns-established:
  - "a RED stub keeps the dispatcher's real signature and answers Rerror to every request, so every end-to-end pin fails on an assertion (Pitfall 11)"
  - "the end-to-end pins start from Mnt::negotiate_dialect and never build a Session by hand, which is what makes them a pin on the real devmnt code"

requirements-completed: [WIRE-05, WIRE-12]

coverage:
  - id: D1
    description: "devmnt has the dialect selection point: an agreed 9P2000.aead hands the io halves, the negotiated msize and the exact Tversion/Rversion bytes to the engine; a plain answer to an aead offer is an error; an unknown version is the existing bad-version error; the msize rules are shared"
    requirement: WIRE-12
    verification:
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_aead_offer_reaches_the_engine"
        status: pass
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_plain_answer_to_aead_offer_is_an_error"
        status: pass
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_unknown_version_is_an_error"
        status: pass
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_msize_rules_hold_on_the_aead_branch"
        status: pass
    human_judgment: false
  - id: D2
    description: "The plain path is unchanged: Mnt::negotiate keeps its signature and bytes, fversion still calls it, devmnt_test is unedited and green, and a plain offer still yields a working Mnt that walks and reads (including against a dual-stack fake)"
    requirement: WIRE-12
    verification:
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_plain_offer_yields_a_working_mnt"
        status: pass
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_plain_offer_sends_the_plain_version_string"
        status: pass
      - kind: integration
        ref: "port/tests/devmnt_test.rs (18 passed; git diff against 3ce5ed8 is empty)"
        status: pass
      - kind: other
        ref: "git grep 'Mnt::negotiate(Box::new(io_r), Box::new(io_w), offer)' -- port/src/devmnt.rs == 1 (the fversion call); no negotiate_dialect(.. Dialect::Aead call in devmnt.rs"
        status: pass
    human_judgment: false
  - id: D3
    description: "A whole sealed session runs from the real devmnt negotiation: Tauth exchange plain, sealed attach (zero handle) returning a new handle, walk/open/read/clunk resolving through FidTable, Rread in one padded page with the true count, and teardown zeroizing both ends"
    requirement: WIRE-05
    verification:
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_end_to_end_session"
        status: pass
    human_judgment: false
  - id: D4
    description: "A capability issued in EXT_STATELESS_CAP verifies with zero FID memory and a flipped byte closes the connection; the same session survives an in-band re-key in both directions and reads the same bytes; a handle from another connection is refused with Rerror"
    requirement: WIRE-05
    verification:
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_end_to_end_stateless_capability"
        status: pass
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_end_to_end_survives_a_rekey"
        status: pass
      - kind: integration
        ref: "port/tests/aead_negotiate.rs#aead_negotiate_end_to_end_handle_from_another_connection_is_refused"
        status: pass
    human_judgment: false
  - id: D5
    description: "Nothing in a booted kernel asks for the aead dialect, and the D-16 limit is stated: no live authenticated connection between two real machines exists in this phase"
    requirement: WIRE-12
    verification:
      - kind: other
        ref: "git grep -l 'Dialect::Aead' -- port/src aarch64 x86_64 riscv64 names only port/src/devmnt.rs (0 other files)"
        status: pass
    human_judgment: true
    rationale: "The grep proves no call site names the selector today; whether that is the full reach of a booted kernel, and whether the D-16 prose limit is stated adequately, is the owner's judgment at the phase-end review (D-23, D-24)."

# Metrics
duration: "~45min"
completed: 2026-11-25
status: complete
---

# Phase 4 Plan 11: the `devmnt` selection point and one whole sealed session Summary

**`Mnt::negotiate_dialect` hands an agreed `9P2000.aead` session to the engine as a `Handoff` (io halves, msize, exact Tversion/Rversion), a plain answer fails closed, the plain mount path is byte-for-byte unchanged, and one whole sealed session runs from that negotiation through attach, walk, open, read, clunk, a stateless capability, a two-direction re-key and teardown with fids in a `FidTable`.**

## Performance

- **Duration:** ~45 min (approximate; several large context reads before the first commit)
- **Started:** 2026-11-24 (before the first RED commit at 20:12 local)
- **Completed:** 2026-11-25T01:16Z
- **Tasks:** 2 (both TDD: RED, GREEN, ledger advance)
- **Files modified:** 5 (2 production/test modified, 1 test created, 1 fixture extended, 1 ledger created)
- **Commits:** 6 measured (`git rev-list --count 04d6f3c..HEAD` before the SUMMARY commit)

## Accomplishments

- `port/src/chan/transport.rs` gains `Handoff { io_r, io_w, msize, tversion, rversion }` and `Handoff::into_handshake`, which binds the session's msize to the negotiated value and absorbs the exact `Tversion` then `Rversion` bytes into the transcript (D-17).
- `port/src/devmnt.rs` gains `Dialect`, `Negotiated` and `Mnt::negotiate_dialect`. It is the existing `negotiate` body generalised: `Plain` offers `VERSION9P` and yields a `Mnt`; `Aead` offers `9P2000.aead` and yields the `Handoff`, while a plain answer is an error naming the downgrade and an unknown version keeps the bad-version error. The three msize rules are shared. `Mnt::negotiate` now delegates with `Dialect::Plain`; `fversion` is untouched.
- `port/tests/aead_negotiate.rs` starts with six negotiation pins: the aead offer reaching the engine with byte-exact handshake messages, the downgrade error, the plain offer working through both entry points against both a plain and a dual-stack fake, the pre-plan `Tversion` bytes, the unknown-version error, and the msize rules on the aead branch.
- The same file adds four end-to-end pins over `E2eClient`/`E2eServer` in `port/tests/common/aead_link.rs`: a whole session from `Mnt::negotiate_dialect` (plain `Tauth`/`Rauth`/afid exchange, then sealed attach/walk/open/read/clunk), a stateless capability with zero FID memory and the P8 close on a flipped byte, a mid-session re-key in both directions, and a handle from another connection refused with `Rerror`.
- The plain path is pinned unchanged three ways: `devmnt_test` has no diff against the phase base and passes 18 tests, the fversion call site still reads `Mnt::negotiate(Box::new(io_r), Box::new(io_w), offer)` exactly once, and no `Dialect::Aead` name exists outside `port/src/devmnt.rs`.

## Task Commits

Each task was committed atomically (RED then GREEN, plus a ledger-advance docs commit):

1. **Task 1 RED: devmnt dialect selection point** — `6703f7f` (test)
2. **Task 1 GREEN: `negotiate_dialect` and the handoff** — `b1c7989` (feat)
3. **Task 1 ledger: advance `devmnt-dialect`** — `0efdfa5` (docs)
4. **Task 2 RED: one whole sealed session** — `1a27f5c` (test)
5. **Task 2 GREEN: the sealed session end to end** — `1ee2bba` (test)
6. **Task 2 ledger: advance `session-end-to-end`** — `d7b7b29` (docs)

**Plan metadata:** the SUMMARY commit follows (docs).

## Files Created/Modified

- `port/src/devmnt.rs` — `Dialect`, `Negotiated`, `plain_session` and `Mnt::negotiate_dialect`; `Mnt::negotiate` delegates. No change to `fversion`, `attach`, `walk`, `open`, `read`, `clunk` or the allocators.
- `port/src/chan/transport.rs` — `Handoff` and `into_handshake`.
- `port/tests/aead_negotiate.rs` — 6 negotiation pins + 4 end-to-end pins, the test-side fake server and the D-16 header.
- `port/tests/common/aead_link.rs` — `E2eClient`, `E2eServer` (a two-shard `FidTable`, a canned tree, one `client_id` per connection, the plain handshake, the sealed dispatcher), `connect_aead`/`connect_aead_with`, `E2eOpened`, and `#![allow(dead_code)]` for the shared fixture.
- `evidence/PINS-04-11.md` — `devmnt-dialect` and `session-end-to-end`, 8 `PIN` lines.

## Decisions Made

- **The version exchange runs before an `Mnt` exists.** `negotiate_dialect` writes the `Tversion` and frames the `Rversion` directly on the two halves, then constructs the `Mnt` only for `Plain`. The plan's wording proposed taking the halves back out of a half-built `Mnt`, but `BusyCell` has no extraction API and `devmnt.rs` is `forbid(unsafe_code)`; this form keeps the plain path byte-identical and needs no new primitive.
- **The handoff owns the msize decision.** `into_handshake` sets `config.msize = handoff.msize`, so a caller cannot accidentally run the session with a different bound from the one negotiated.
- **The capability is issued on `Ropen`.** That is the first point where the server knows the qid and the mode; the client then presents it on a stateless `Tread`.
- **The cross-connection pin uses a fid absent from B's table.** The handle carries no `client_id`, so the rejection is `resolve`'s entry-identity check against B's own table; using a fid B never inserted makes the refusal deterministic.

## Deviations from Plan

### Auto-fixed Issues

None — no production bug surfaced. Task 2 needed no production change beyond Task 1: the session composed plans 04-03 to 04-10 as intended.

### Plan-structure notes

1. **Three commits per task, not two.** Each pin's `ADVANCED` line must name the GREEN commit, which cannot ride inside it, so a small `docs` commit follows each GREEN (the plan 04-10 precedent; 6 commits, no extra code).
2. **`negotiate_dialect` builds no throwaway `Mnt`.** The plan said "takes the io halves back out of the half-built `Mnt`"; it does the exchange on the halves instead, for the `BusyCell`/`forbid(unsafe_code)` reason above. Behaviour and bytes are identical; the plain-path pins prove it.
3. **`#![allow(dead_code)]` added to the shared test fixture.** `aead_link.rs` is compiled into two test binaries that use different subsets, so the warning is lint noise, not a real dead path.

---

**Total deviations:** 0 auto-fixed; 3 plan-structure notes.
**Impact on plan:** None on behaviour. The commits and the file set are the plan's.

## Issues Encountered

- **The handle carries no `client_id` (recorded in plan 04-10, observed here).** Rejection across connections comes from `resolve` comparing the entry it reads against the server's `client_id` and the handle's `fid_key`/epoch, not from anything inside the handle. The pin uses a fid connection B never inserted so the refusal does not depend on a coincidental slot. This remains a Phase 6 design question (the authenticated source of `client_id`).
- **The first draft of the e2e fixture was appended twice to `aead_link.rs` by a botched in-place edit.** It was cleaned before any commit; no committed state contains the duplicate.

## Authentication Gates

None. No external service, no network, no credential.

## User Setup Required

None — no external service configuration required.

## Known Stubs

None. Both RED stubs (`negotiate_dialect` returning an error; the server answering `Rerror` to everything) were replaced in their GREEN commits, and every pin they exposed now passes.

## Threat Flags

None beyond the plan's `<threat_model>`. The new surface is the cleartext `Rversion` dialect choice, already registered as T-04-36 and now covered by `aead_negotiate_plain_answer_to_aead_offer_is_an_error`, and the kernel reach of `fversion`, T-04-37/T-04-38, covered by the unchanged call site and the grep gate.

## Next Phase Readiness

- **WIRE-12 holds on the host.** The selection point exists, the plain path is unchanged and pinned, and a downgrade fails closed.
- **ROADMAP success criterion 4 holds on the host** apart from the boot gate: one whole `9P2000.aead` session runs from the real `devmnt` negotiation to teardown, with per-connection and per-direction keys and a zeroizing teardown. The boot gate is plan 04-15's.
- **Plan 04-14** reviews the whole `port/src` change at phase end (D-24); this plan's production diff is `devmnt.rs` and `transport.rs`.
- **Plan 04-17's `test-9p`** already runs `aead_negotiate.rs` (the file lives under `port/tests`), and `cargo xtask test-9p` exits 0 with 10 pins here.
- **Phase 6** owns the only legitimate downgrade path (the legacy gateway) and the authenticated source of `client_id`; nothing in this phase provisions either.

---
*Phase: 04-guide-phase-2-9p2000-aead-transport-and-fidtable*
*Completed: 2026-11-25*

## Self-Check: PASSED

- All three files exist on disk (`port/tests/aead_negotiate.rs`, `evidence/PINS-04-11.md`, `04-11-SUMMARY.md`).
- All six plan commits are ancestors of HEAD (`6703f7f`, `b1c7989`, `0efdfa5`, `1a27f5c`, `1ee2bba`, `d7b7b29`).
- `cargo test -p port --test aead_negotiate` reports 10 passed, 0 failed; `devmnt_test` reports 18 passed and has no diff against the phase base 3ce5ed8.
- `cargo xtask test` x2, `cargo xtask test-9p`, `cargo xtask check` and `cargo xtask clippy` all exit 0.
- The pin ledger carries 8 `PIN (devmnt-dialect|session-end-to-end) (RED|GREEN-1|GREEN-2|ADVANCED)` lines.
