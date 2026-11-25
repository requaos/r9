---
phase: 04
review: 04-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: open
    title: "The re-key exchange secret survives `fail()` — `keys_zeroized()` reports a lie"
  - id: CR-02
    severity: critical
    disposition: open
    title: "In-band exchange messages are indistinguishable from ordinary 9P traffic"
  - id: CR-03
    severity: critical
    disposition: open
    title: "The re-key transcript absorbs the page-padded body, contradicting DECISION P2"
  - id: WR-01
    severity: warning
    disposition: open
    title: "The re-key state never returns to idle — at most one re-key per connection"
  - id: WR-02
    severity: warning
    disposition: open
    title: "`SendHalf::send` ignores the re-key acknowledgement deadline"
  - id: WR-03
    severity: warning
    disposition: open
    title: "`seal_packet`/`open_packet` use a different keystream layout from `Session`"
  - id: WR-04
    severity: warning
    disposition: open
    title: "`FidTable::resolve` can accept a torn read of a clunked slot for fid 0 / client 0"
  - id: WR-05
    severity: warning
    disposition: open
    title: "`sweep` removes whatever the slot now holds — a reused slot's live entry is destroyed"
  - id: WR-06
    severity: warning
    disposition: open
    title: "An out-of-window `ptr` in a handle aliases another slot; the bitmaps then name the wrong id"
  - id: WR-07
    severity: warning
    disposition: open
    title: "Re-key control messages carry tag 0, not the pinned `NOTAG`"
  - id: WR-08
    severity: warning
    disposition: open
    title: "`cargo xtask test-9p` cannot detect a listed test file that lost all its tests"
  - id: WR-09
    severity: warning
    disposition: open
    title: "`insert` can create two live entries for one `(client_id, fid_key)` after a fallover"
  - id: IN-01
    severity: info
    disposition: open
    title: "Module docs describe the pre-implementation state"
  - id: IN-02
    severity: info
    disposition: open
    title: "`DataLayout::pages` is computed and never used; `crypt_body` recomputes it with a literal"
  - id: IN-03
    severity: info
    disposition: open
    title: "`wire::is_t_message` has no production caller"
  - id: IN-04
    severity: info
    disposition: open
    title: "`Msg`'s hand-written `PartialEq` ends in `_ => false`, which hides a new variant"
  - id: IN-05
    severity: info
    disposition: open
    title: "The threat-model overclaim guard only fires on rows carrying the Phase 4 status"
  - id: IN-06
    severity: info
    disposition: open
    title: "The end-to-end capability test claims \"zero FID memory\" but does not measure it"
  - id: IN-07
    severity: info
    disposition: open
    title: "The `ds` removal test's \"the freed slot takes another key\" is not asserted"
open: 19
total: 19
recorded: 2026-11-25T03:16:25.575Z
---

# Phase 04: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | open | - |
| CR-02 | critical | open | - |
| CR-03 | critical | open | - |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| WR-06 | warning | open | - |
| WR-07 | warning | open | - |
| WR-08 | warning | open | - |
| WR-09 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
