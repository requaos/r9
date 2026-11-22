# Phase 03 — Deferred Items

Items discovered during phase 03 execution that are acknowledged and deferred, most recent first.

| # | Item | Discovered In | Deferred At | Notes |
|---|------|---------------|-------------|-------|
- **raspi3b qemu boot broken upstream on qemu 11.1.2** — guest data-abort at FAR `0xfe215004` during early boot: the rpi4 AUX_ENABLES MMIO base is touched while running on the raspi3b machine. A/B-proven pre-existing (identical abort at pre-plan xtask `6a309ae` + pre-03-02 kernel `ae4cd08` vs post-plan HEAD; raspi qemu argv byte-identical). (03-03 Task 2 (raspi boot-smoke gate) — 2026-11-01 (03-03) — Fix deferred. Candidate suspect: `init_early_uart_rpi4` hardcoded rpi4 constants. Until fixed, raspi banner-smoke gates on this host are unmeetable — later plans (03-05/03-06) should reuse the A/B + argv-identity pattern or fix this first. The virt platform (this phase's target) is unaffected.)
  status: acknowledged
