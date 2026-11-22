---
phase: 03
review: 03-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "TTBR0 identity map keeps GIC/PL011 MMIO Normal-cacheable (and EL1-executable) forever on virt"
  - id: WR-02
    severity: warning
    disposition: open
    title: "`boot-test` accepts non-virt configs into a guaranteed 300 s failure"
  - id: WR-03
    severity: warning
    disposition: open
    title: "raspi PL011 `gpiosetpull` writes GPIO registers through the PL011 MMIO window"
  - id: WR-04
    severity: warning
    disposition: open
    title: "`map_phys_range` swallows mid-range `map_to` failures and returns `Ok` for a partial map"
  - id: WR-05
    severity: warning
    disposition: open
    title: "DTB-rediscovered timer INTID is advisory-only; enable and dispatch stay hardcoded at 30"
  - id: IN-01
    severity: info
    disposition: open
    title: "Dead `mrs x0, elr_el1` before the higher-half jump"
  - id: IN-02
    severity: info
    disposition: open
    title: "x28 \"entrypoint\" cached from x4 but never used"
  - id: IN-03
    severity: info
    disposition: open
    title: "earlyvm page tables zeroed four times"
  - id: IN-04
    severity: info
    disposition: open
    title: "`apply_link` panics on a `[link]` table missing `script`; redundant nested `if let`"
  - id: IN-05
    severity: info
    disposition: open
    title: "`-Z build-std=core,alloc` passed twice on every aarch64 build"
  - id: IN-06
    severity: info
    disposition: open
    title: "Reference-to-temporary in `dump_dtb` default"
  - id: IN-07
    severity: info
    disposition: open
    title: "`--config` name interpolated into a filesystem path unsanitized"
  - id: IN-08
    severity: info
    disposition: open
    title: "\"logged once\" comment vs. log-every-IRQ implementation"
  - id: IN-09
    severity: info
    disposition: open
    title: "HowItWorks.md identity-window and stack-size claims are wrong"
  - id: IN-10
    severity: info
    disposition: open
    title: "flake.nix objcopy hook can export an empty `OBJCOPY`"
open: 15
total: 15
recorded: 2026-11-02T00:41:19.708Z
---

# Phase 03: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |
| IN-08 | info | open | - |
| IN-09 | info | open | - |
| IN-10 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
