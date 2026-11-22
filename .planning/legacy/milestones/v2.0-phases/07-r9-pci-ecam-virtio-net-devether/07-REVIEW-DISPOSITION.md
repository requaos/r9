---
phase: 07
review: 07-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "`readnum_content(-1)` panics the kernel — `connect -1` is an admitted ctl verb"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "Capability extent validation admits zero-length/undersized regions — first register touch panics instead of a loud bind-fail"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "device-config cap admits `length >= 6`; the link-status read at offset 6 needs 8"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "BAR4 type never validated; window size ignored at assignment"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "Capability-walk pointers never alignment-checked — a misaligned cap pointer is an unaligned volatile u32 read"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "`WaitSet` capacity is 16 on a stale \"MAX_PROCS=16\" invariant — the table is now 64, and the full case silently drops a waiter"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "RX retire never checks the slot was actually posted — a duplicated used-ring id aliases one buffer page into two chains"
  - id: WR-07
    severity: warning
    disposition: deferred
    title: "net-sweep.rc's count validation admits leading zeros that the arp loop can never match — infinite loop"
  - id: WR-08
    severity: warning
    disposition: deferred
    title: "Decoded DTB interrupt trigger flags are printed but never honored — ICFGR is always programmed level"
  - id: IN-01
    severity: info
    disposition: open
    title: "Dead `dd` trailer extraction in mkimage.sh"
  - id: IN-02
    severity: info
    disposition: open
    title: "`VirtRange::offset_addr` bounds-checks only the access start, not start+width"
  - id: IN-03
    severity: info
    disposition: open
    title: "Documentation drift — proc-table/fd-table sizes and a stale virtnet header comment"
  - id: IN-04
    severity: info
    disposition: open
    title: "`DescPool::free` has no bounds or duplicate-free checks"
  - id: IN-05
    severity: info
    disposition: open
    title: "netblast's `atoi` silently accepts trailing garbage"
open: 5
total: 14
recorded: 2026-11-05T02:57:20.913Z
---

# Phase 07: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | 07-09 (u32 truncation + host pins) |
| WR-01 | warning | fixed | 17-04 — the bind-time extent admission closed at the T-07-11 site as the pure host-testable check_cap_extents: zero-length extents rejected (the exact `offset == bar_size, length == 0` hole), common >= 0x40 (the register block through CFG_QUEUE_DEVICE+8), isr >= 1 (subsumed by the zero-length reject), and queue_notify_off x multiplier must address a u16 inside the notify cap (bring_up_queue). End edge re-derived against the live caps: qemu's notify cap (length = mult x VIRTIO_QUEUE_MAX = 0x1000 at 0x3000) ends EXACTLY at the 0x4000 bar boundary, so the > edge stays and the zero-length reject closes the hole (a >=-style edge would reject the real device) (r9 cc22223, RED evidence 17-04-t2; cap_extents_* pins + notify_offsets_must_reach_inside_the_notify_cap, battery 76/76; smoke/net-smoke/tier1/churn green at Harvey pin 02218e88c — services fails at a pre-existing dns-stage trap, A/B-proven identical at pin b02408cad) |
| WR-02 | warning | fixed | 17-04 — the device-config minimum rises 6 -> 8: the IRQ handler reads the link-status le16 at devcfg+6, so lengths 6/7 admitted a config the handler over-reads on the first config-change interrupt (r9 cc22223; cap_extents_reject_undersized_common_and_device_configs pins 6/7 rejected + 8/0x40 admitted; battery 76/76; net gate green at pin 02218e88c) |
| WR-03 | warning | fixed | 17-04 — size_bar64 validates the BAR type nibble (memory, bit 0 clear + 64-bit, bits 2:1 == 0b10 — is_64bit_mem_bar_nibble) BEFORE the all-ones probe is trusted (Option return, the bind maps None to a '(device fault)' Err), and bar_assignment adds the natural-alignment check beside the unchanged window-END check (checked_rem — size 0 is a loud None, never a division) (r9 cc22223; only_64bit_memory_nibbles_are_sizing_trusted + bar_assignment_rejects_naturally_misaligned_window_bases; battery 76/76; gates green at pin 02218e88c) |
| WR-04 | warning | fixed | 17-04 — shared Caps::parse rejects misaligned cap pointers (p & 3) BEFORE any read at them (an unaligned volatile u32 on the real ECAM seam is UB/a Device-memory fault) and a notify cap whose cap_len < CAP_NOTIFY_LEN (the multiplier was read out of the NEXT cap's bytes) (r9 cc22223, RED evidence 17-04-t1; caps_parse_rejects_misaligned_capability_pointers + caps_parse_rejects_short_notify_capabilities, caps filter 5/5 incl. the unchanged well-formed fixture pin, battery 114/114) |
| WR-05 | warning | fixed | 17-03 — one port-level MAX_WAITERS=512 const (the live table on both arches) resized the three WaitSet copies + twelve take buffers; the full arm counts + reports (r9 2bc52a7, RED pins aea604c; the nine waitset capacity pins, 9/9, battery 112/112; gates re-green at Harvey pin b02408cad) |
| WR-06 | warning | fixed | 17-03 — next_used rejects a used-ring id for an unposted slot before the clear, the '(device fault)' family (r9 abe72e7, RED pin 3bd8625; used_drain_rejects_unposted_slot_ids + the existing negatives 4/4; net gate re-green at Harvey pin b02408cad) |
| WR-07 | warning | deferred | 17-04 D-15 3-part disposition — WHERE: the class lived in the aarch64-era gate fixture `build/aarch64/root-tree/lib/net-sweep.rc` (the count validation at :35-42 admits leading zeros — `04` — that the arp loop's `while(! ~ $#i $n)` string match at :74-81 can never equal, an infinite loop; netblast mode unaffected, its atoi canonicalizes); WHY: the fixture is superseded — the committed tier-sweep machinery (the x64 venue's tier drivers, `netip-*.rc` canonicalizing lineage) replaced it as the load driver and no live venue boots net-sweep.rc, so there is nothing executing to fix, and the aarch64 venue it served is frozen at the v2.0 pin; Reopener: the net-sweep.rc fixture returns to a live venue's boot image (or aarch64 reactivation un-freezes the tree) — the leading-zero validation must land with the fixture's readoption |
| WR-08 | warning | deferred | 17-04 D-15 3-part disposition — WHERE: the aarch64 DTB interrupt trigger-flag path (aarch64/src/platform/virt/pci.rs:154-162 decodes and PRINTS `level-high`/`edge-rising`, then gicv2.rs:99-103 / gicv3.rs:231-239 unconditionally program ICFGR level — frozen at the v2.0 pin, reference-only in v2.1); WHY: the aarch64 freeze policy — v2.1 keys everything to x86_64 (whose IOAPIC path has no ICFGR trigger-flag equivalent to diverge), and the committed DTBs carry flags 0x04 (level-high) so the mismatch is latent on every boot the frozen tree can make; Reopener: aarch64 reactivation — the decoded flags must then be plumbed into gic::enable(intid, prio, trigger) or non-level routes rejected loudly at discovery (`pci: irq flags {:#x} not level-high — unsupported`) |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
