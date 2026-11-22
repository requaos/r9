---
phase: 18
review: 18-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "Ungated `/dev/hostowner` write defeats the devcap eve gate (authorization bypass)"
  - id: CR-02
    severity: critical
    disposition: fixed
    title: "RDRAND CPUID bit read from leaf 7 ECX instead of leaf 1 ECX (entropy source misclassification)"
  - id: WR-01
    severity: warning
    disposition: open
    title: "One-byte stack overflow in `authdomain-gen -V`/`-i` on a manifest of exactly `Filecap` bytes"
  - id: WR-02
    severity: warning
    disposition: open
    title: "Generator accepts passwords the donor `passtokey` silently truncates at 27 chars"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "Workflow comment overstates the CI contract — the secstore round-trip is NAMED-SKIPPED, not exercised"
  - id: IN-02
    severity: info
    disposition: open
    title: "Sweep's secondary probe address lives outside the netaddr seam"
  - id: IN-03
    severity: info
    disposition: open
    title: "Modulo bias in `genpassword`"
  - id: IN-04
    severity: info
    disposition: open
    title: "Necessity — tick-hook infrastructure feeds a value that provably nothing reads"
  - id: IN-05
    severity: info
    disposition: open
    title: "Whitespace-bearing local-file passwords break the sweep's factotum leg non-obviously"
  - id: CR-03
    severity: critical
    disposition: open
    title: "18-07 TCP fix applied only to the connect path — accept-pool sockets keep Nagle + delayed ACK, so bulk writes into announced connections hang (the 18-05 stall class, still live)"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Hostowner write drops the donor's rename — a single write is an unrecoverable per-boot eve lockout"
  - id: WR-04
    severity: warning
    disposition: open
    title: "The two-write pin covers only the fixed half — no server-side pattern and no bulk-drain leg (CR-03's blind spot)"
  - id: IN-06
    severity: info
    disposition: open
    title: "Sweep 9d's copy redirect is fatal-on-failure with mislocalizing timeout evidence"
  - id: IN-07
    severity: info
    disposition: open
    title: "README probe note still names the previous r9 pin as current"
open: 11
total: 14
recorded: 2026-11-14T17:40:00.000Z
---

# Phase 18: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | 18-06 r9 652f025 — iseve gate + donor 1..=127 bound + 3 refusal pins + randread HOSTOWNER-REFUSED boot leg; re-verified sound by the gap-closure review |
| CR-02 | critical | fixed | 18-06 r9 f594b4c — cpuid(1,0) ECX rebind + scripted-cpuid seam + probe-wiring pins; re-verified sound by the gap-closure review |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| IN-01 | info | fixed | 18-08 harvey 946da6635 — CI contract comment corrected to the flipped gate; confirmed resolved by the gap-closure review |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| CR-03 | critical | open | gap-closure round — the accept-pool hole in the 18-07 fix (stack.rs spawn_listener keeps smoltcp stock Nagle + 10ms delayed ACK; no tick_check on x86_64); secstore PUTs > 2048 B into announced connections park forever; gate green only because the DEMO object is 401 B |
| WR-03 | warning | open | gap-closure round — 18-06's deliberate rename deferral leaves a write-once-per-boot eve lockout (echo-name idiom stores an unmintable base) |
| WR-04 | warning | open | gap-closure round — the RED->GREEN pin exercises only the client/connect side at 2+31 B, CR-03's blind spot |
| IN-06 | info | open | gap-closure round — fatal copy redirect in sweep 9d mislocalizes a failure as a 600 s secstored-auth timeout |
| IN-07 | info | open | gap-closure round — README present-tense pin note still names d1c4849 after the 9103522 advance |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
