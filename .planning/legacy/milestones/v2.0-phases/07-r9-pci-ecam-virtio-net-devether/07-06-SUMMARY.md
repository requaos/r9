---
phase: 07-r9-pci-ecam-virtio-net-devether
plan: 06
subsystem: infra
tags: [r9, virtio, virtio-net, pci, ecam, bar-assignment, capability-walk, feature-negotiation, virtqueue, isr-read-to-ack, gic-icfgr, devether, aarch64, gicv2, gicv3]

# Dependency graph
requires:
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: 07-02 (DTB-driven PCI/ECAM discovery + the VirtioNetDiscovery facts this bind consumes), 07-03 (the CommonCfg/PciCfg/RingMem seams + the negotiation machine + the ring index model), 07-05 (the virtnet gate whose red path must stay honest)
provides:
  - io.rs volatile u8/u16/u64 read/write twins of the u32 floor, SAFETY-commented (the u8 reader names the virtio ISR read-to-ack)
  - pci.rs device bring-up: BAR4 sizing (all-ones mask/restore) + assignment from the DTB ranges 64-bit MMIO window, PCI_COMMAND Memory-Space+Bus-Master, bounds-checked capability walk (T-07-11), the arch-side CommonCfg/PciCfg/RingMem seam impls, the publish-once device state + INTID cells, the ISR-read-first bounded-drain lock-free IRQ handler
  - port::virtio's virtio_net consumer surface: queue indices, ISR bits, the RX frame-length rule, nintr/nnote counters; rx_replenish learns the slot being posted and reports the notify truth (Replenish)
  - trap.rs's match-guard dispatch arm keyed on the published INTID (zero hardcoded interrupt numbers)
  - GIC ICFGR level-trigger programming BEFORE ISENABLER in both gicv2.rs and gicv3.rs enable paths (Pitfall N4)
  - dev = ["ether"] on all nine aarch64 virt configs (D-10's xtask end state); the NIC binds on both GIC generations
  - vm.rs's next_free_device_va_span + deviceutil's use of it — the device-VA arena now reserves a map's whole span (the aliasing fix)
affects: [07-07, 07-08, phase-8, phase-9, phase-11]

# Actuals (#2632) — pairs with the plan's estimate (tokens 85000, confidence: low).
actuals:
  tokens: 14431     # chars/4 over the realized r9 diff (993be67..92e9b26, 57726 chars)
  tasks: 2
  commits: 2        # MEASURED: git rev-list --count 993be67..92e9b26 (r9); the harvey metadata docs commit rides on top

plan_head_before: 993be67456a9543e93f45b4d4a2ca5c1cae8f547   # r9 repo (code commits)
plan_head_after: 92e9b2671fee91777a1db74fa72efda330738f10    # r9 repo

# Tech tracking
tech-stack:
  added: []   # zero new dependencies (T-07-SC vacuous — the Cargo diff is empty)
  patterns:
    - "Publish-once device state + published INTID (pl011 discipline): written once at bind before gic::enable/unmask, read lock-free in IRQ context; the trap dispatch keys on the published value, never a constant"
    - "Device-VA arena reservations are span-sized (next_free_device_va_span) — a map must reserve its whole range or the next map aliases its tail"
    - "BAR bring-up from data: size via all-ones mask/restore, assign from the DTB ranges 64-bit window, Memory-Space+Bus-Master explicit, readback sanity print (Pitfall N3)"
    - "Replenish outcome carries the notify truth (Replenish { offered, notified }) so nnote accounting is exact under suppression"
    - "IRQ handler discipline: ISR byte read FIRST (read-to-ack), DRAIN_CAP bounded retire with the §2.7.14 re-check shape, no locks/no console on the hot path, one-time prints only"

key-files:
  created: []
  modified:
    - r9:aarch64/src/platform/virt/pci.rs
    - r9:aarch64/src/io.rs
    - r9:port/src/virtio.rs
    - r9:port/src/virtio_tests.rs
    - r9:aarch64/src/trap.rs
    - r9:aarch64/src/platform/virt/gic/gicv2.rs
    - r9:aarch64/src/platform/virt/gic/gicv3.rs
    - r9:aarch64/src/main.rs
    - r9:aarch64/src/vm.rs
    - r9:aarch64/src/deviceutil.rs
    - r9:aarch64/lib/config_virt.toml
    - r9:aarch64/lib/config_virtexec.toml
    - r9:aarch64/lib/config_virtexec3.toml
    - r9:aarch64/lib/config_virtmulti.toml
    - r9:aarch64/lib/config_virtnotes.toml
    - r9:aarch64/lib/config_virtpanic.toml
    - r9:aarch64/lib/config_virtshell.toml
    - r9:aarch64/lib/config_virtlowecam.toml

key-decisions:
  - "07-06: the device-VA arena now reserves a map's WHOLE span (vm::next_free_device_va_span; deviceutil uses it) — the one-page bump aliased every multi-page map's tail (the 64 KiB GICD map clobbered the ECAM window's dev-1 page; the bind was the first ECAM consumer after gic::init, latent since 07-02)"
  - "07-06: the pci node's ranges 64-bit MMIO window is parsed pci.rs-local (stride from the declared cell counts; the 0b11 space-bits tuple) — fdt::property_range_iter caps child addresses at 2 cells and PCI declares 3; the BAR base comes from the DTB either way (NETDRV-01's rule holds)"
  - "07-06: rx_replenish's alloc closure learns the slot being posted (per-slot buffer-page binding) and the call returns Replenish { offered, notified } (exact nnote under suppression) — the 07-03 seam's sanctioned evolution; 22 port tests green"
  - "07-06: ring bring-up order is Ring::new chain assembly → queue_size accepted as-offered → queue addresses → queue_enable LAST → DRIVER_OK → rx_replenish pre-posts all 128 chains with ONE notify — the 07-03 boot-order contract (never notify before DRIVER_OK) governs over the truth bullet's looser 'pre-posts before DRIVER_OK' phrasing"
  - "07-06: the config-change (ISR bit 1) link-status line is a one-time latched print from IRQ context — the plan's explicit one-time-print carve-out; slirp's link never changes, so it never fires on the gates"

patterns-established:
  - "Arch-side volatile seams as tiny private structs over VirtRange + io.rs widths (Regs/CommonCfg, VolatileMem/RingMem, EcamCfg/PciCfg) — the RegBlock idiom honored without touching regblock.rs"
  - "Bind-in-the-arming-window shape: probe consume → BAR/caps → negotiate → queues → pre-post → publish → gic::enable → bind print, all IRQ-masked (D-10's IRQ source armed safely)"

requirements-completed: [NETDRV-02]   # copied verbatim from plan frontmatter; shared with 07-03 — the ready-ids gate releases it now (07-03's SUMMARY exists)

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "io.rs volatile u8/u16/u64 read/write twins with per-block SAFETY comments (the u8 reader names the ISR read-to-ack); exercised by the live bind on every ether boot"
    requirement: NETDRV-02
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch aarch64 --config virt PASS + all ether profiles bind (the widths carry every common-cfg/ISR/BAR access); workspace check/clippy exit 0; bare-build dead-code clean via the in-file allow precedent"
        status: pass
    human_judgment: false
  - id: D2
    description: "Device bring-up from data on the real device: BAR4 sized 0x4000 (mask/restore), assigned 0x8000000000 from the DTB ranges window with matching readback, PCI_COMMAND Memory+Master, cap walk (common@0x0+0x1000 notify@0x3000 isr@0x1000 device@0x2000 mult 4) bounds-checked against the BAR size, negotiation printing offered 0x10130bf8024 negotiated 0x100010020, queues at qsize 256 with 128 RX chains pre-posted, the '#l0: virtio: 1000Mbps: 52:54:00:12:34:56' bind line"
    requirement: NETDRV-02
    verification:
      - kind: e2e
        ref: "boot-test virt PASS + greps (bind line, negotiated 0x100010020, discovery line, bar4/caps/pre-post prints) run twice (task gate + tracer feedback re-run); same bind line present in all seven other ether profiles' logs incl. virtnet"
        status: pass
    human_judgment: false
  - id: D3
    description: "GIC ICFGR level-trigger programming before ISENABLER in BOTH gicv2.rs and gicv3.rs enable paths (Pitfall N4); the NIC binds on both GIC generations (virtexec3 = first NIC-on-GICv3 boot)"
    requirement: NETDRV-02
    verification:
      - kind: e2e
        ref: "seven-profile regression loop PASS twice (all with the ICFGR write live: timer/PL011/NIC); bind-line greps on virtexec + virtexec3; acceptance: ICFGR write precedes ISENABLER in both diffs"
        status: pass
    human_judgment: false
  - id: D4
    description: "trap.rs dispatch arm keyed on the pci-published INTID (0 = unbound); zero hardcoded interrupt numbers; zero unhandled-IRQ lines on all eight boots (the 07-05 stray-IRQ scan stays silent)"
    requirement: NETDRV-02
    verification:
      - kind: e2e
        ref: "grep -rn 'intid 36|== 36' trap.rs/pci.rs → none; grep -ci 'unhandled irq' on all eight fresh logs → 0; seven-profile loop + virtnet all clean"
        status: pass
    human_judgment: false
  - id: D5
    description: "D-10 end state: dev = ['ether'] on all nine aarch64 virt configs; raspi4b/default untouched; virtpanic dump byte-shape stable (start/halt markers, 31 register rows); virtnet still red-paths naming netsweep: FAIL (awaiting 07-07's #l); workspace test/check/clippy green"
    requirement: NETDRV-02
    verification:
      - kind: e2e
        ref: "ALL-CONFIGS-BIND-OK + RED-PATH-STILL-CORRECT + CI-ALL-GREEN; grep -l ether config_virt*.toml covers all nine; raspi4b/default zero matches"
        status: pass
    human_judgment: false
  - id: D6
    description: "The IRQ handler's retire/replenish/counter behavior UNDER LIVE TRAFFIC (RX frames demuxed, TX retired, storm bounds) — this plan's gates keep the armed handler inert (idle slirp emits nothing until spoken to, RESEARCH Q9), so the plumbing is proven armed-but-idle, not traffic-proven"
    requirement: NETDRV-02
    verification: []
    human_judgment: true
    rationale: "No traffic source exists until 07-07's /net/ether0 file contract; the traffic-level proof is 07-08's NETDRV-04 ARP ping-pong + TX-blast reconciliation gate (the assertion suite is already unit-proven by 07-05). The handler's discipline (ISR-first, bounded drain, lock-free) is code-review-visible and its inert safety is gate-proven (zero unhandled IRQs, nothing wedged), but no automated gate in this plan moves a frame."

# Metrics
duration: 25 min
completed: 2026-11-05
status: complete
---

# Phase 07 Plan 06: Virtio-Net Driver Bind — BAR/Caps, Negotiation, Rings, ISR Handler Summary

**r9 owns the NIC: the virtio-net driver binds from DTB data on every aarch64 virt boot — BAR4 sized and assigned from the ranges 64-bit window (0x8000000000), PCI_COMMAND Memory-Space+Bus-Master explicit, the capability walk bounds-checked over the mapped BAR, exactly MAC|STATUS|VERSION_1 negotiated (0x100010020, printed next to the offered mask), both queues live at qsize 256 with all 128 RX chains pre-posted, the ISR read-to-ack handler armed on the DTB-routed SPI with ICFGR level programming in both GIC drivers, and `#l0: virtio: 1000Mbps: 52:54:00:12:34:56` on every boot — with the latent device-VA arena aliasing bug (multi-page maps clobbering each other) found and fixed en route.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-11-05T00:41:40Z
- **Completed:** 2026-11-05T01:07:11Z
- **Tasks:** 2
- **Files modified:** 18 (all r9-side; zero Harvey repo source changes)

## Accomplishments

- Task 1 (tracer): the full bind chain on the real device — io.rs u8/u16/u64 volatile twins (SAFETY-commented, the u8 reader names read-to-ack); pci.rs bring-up (BAR4 sizing with all-ones mask/restore, assignment from the DTB `ranges` 64-bit window, PCI_COMMAND bits 1+2, cap walk via the 07-03 `Caps` model with per-cap bounds checks against the BAR size, sanity prints per Pitfall N3); the three arch-side seam impls (`Regs`/`CommonCfg`, `VolatileMem`/`RingMem`, `EcamCfg`/`PciCfg` — the RegBlock idiom kept local, regblock.rs untouched); negotiation through the 07-03 machine printing BOTH masks (offered 0x10130bf8024, negotiated 0x100010020); per-queue bring-up (queue_size 256 accepted as-offered, one zeroed 4 KiB page per ring part mapped Normal at KZERO+PA via the initrd idiom — never rw_device, queue_enable LAST, DRIVER_OK before the first notify); all 128 RX chains pre-posted with zeroed pages; publish-once state + INTID cells; the lock-free bounded-drain ISR-read-first handler; the trap.rs match-guard arm keyed on the published INTID; ICFGR level programming before ISENABLER in both GIC drivers; the cfg(dev_ether) bind call in the IRQ-arming window; config_virt.toml gains dev = ["ether"]
- Task 1's port half: the virtio_net consumer section in port::virtio (queue indices, ISR bits, the 12-byte strip rule, the nintr/nnote counters) + the sanctioned seam evolution — `rx_replenish` learns the slot being posted and returns `Replenish { offered, notified }` (exact nnote under suppression); 22 port tests green including the new slot/suppression pin
- Task 2: dev = ["ether"] on the seven remaining aarch64 virt configs (virtpanic = ["panic", "ether"]; virtshell = ["exec", "bootchain", "ether"] — the packaged kernel binds the NIC for the Harvey compose boot, 07-08's dependency); the handler-discipline doc comment completed (Pitfall N4/N10 signposts); the eight-boot regression green with the NIC binding on BOTH GIC generations (virtexec3 is the first NIC-on-GICv3 boot), virtpanic's dump byte-shape stable (31 register rows), virtnet still red-paths naming `netsweep: FAIL`, zero unhandled-IRQ lines anywhere, workspace CI green
- Tracer feedback gate: interactive + end-of-phase + automated-only verify → all three verify legs re-run end-to-end post-commit (virt bind gate, six-profile regression, workspace test/check/clippy) — TRACER-GATE-ALL-GREEN; expanded per the row-3 auto-continue

## Task Commits

Each task was committed atomically (r9 repo, branch main — the dual-repo contract):

1. **Task 1 (tracer): virtio-net driver bind — BAR/caps, negotiation, rings, ISR handler** — `b12f04c` (feat) — 11 files: io.rs, pci.rs, virtio.rs, virtio_tests.rs, trap.rs, gicv2.rs, gicv3.rs, main.rs, config_virt.toml + the two deviation files (vm.rs, deviceutil.rs)
2. **Task 2: dev = ["ether"] across every aarch64 virt config (D-10 end state)** — `92e9b26` (feat) — 7 TOMLs + pci.rs (doc comment)

**Plan metadata:** docs commit rides on top in the Harvey repo (SUMMARY.md + STATE.md + ROADMAP.md + REQUIREMENTS.md), per the dual-repo contract.

## Files Created/Modified

- `r9:aarch64/src/platform/virt/pci.rs` (+~700) — the bring-up half: publish-once discovery handoff, BAR sizing/assignment, `mmio64_window` (pci-local ranges parse), `alloc_ring_page` (zeroed Normal pages at KZERO+PA), `bring_up_queue`, `bind_virtio_net`, `virtio_irq_intid`, `virtio_net_irq`, the seam impls, the state struct
- `r9:aarch64/src/io.rs` (+66) — the six width twins, each SAFETY-commented, `allow(dead_code)` per the file's write_or_reg precedent (bare builds carry no dev cfgs)
- `r9:port/src/virtio.rs` (+~75) — the virtio_net consumer section + the rx_replenish signature evolution
- `r9:port/src/virtio_tests.rs` — call-site updates + `rx_replenish_reports_the_slot_and_suppression` (22/22 green)
- `r9:aarch64/src/trap.rs` (+9) — the cfg(dev_ether)-gated match-guard arm
- `r9:aarch64/src/platform/virt/gic/gicv2.rs` / `gicv3.rs` — ICFGR constants + the level-trigger write before ISENABLER in both branches (v3: SGI frame for intid < 32, GICD otherwise)
- `r9:aarch64/src/main.rs` (+11) — the bind call in the arming window, ahead of the dev_panic brk site
- `r9:aarch64/src/vm.rs` (+13) — `next_free_device_va_span` (the deviation fix)
- `r9:aarch64/src/deviceutil.rs` — both maps reserve their whole span
- `r9:aarch64/lib/config_virt{,exec,exec3,multi,notes,panic,shell,lowecam}.toml` — dev gains "ether" with a D-10 comment

## Decisions Made

1. **The device-VA arena fix (Rule 1/3 — see Deviations).** `vm::next_free_device_va_span(len)` reserves the whole page-rounded span; `deviceutil` (both map paths) uses it. Single-page callers (`next_free_device_page4k`, e.g. proc kstacks) are untouched and stay correct.
2. **The PCI `ranges` parse lives in pci.rs** (not fdt.rs): the existing `property_range_iter` bails on child address cells > 2, and PCI addressing needs 3. The local walk derives the stride from the declared cell counts and selects the tuple whose phys.hi space bits are 0b11 (64-bit memory). fdt.rs stays untouched (outside this plan's scope).
3. **rx_replenish's sanctioned evolution:** slot-parameterized alloc (per-slot buffer-page binding — the arch side records `rx_buf_pa[slot]` in the closure) and the `Replenish { offered, notified }` return (nnote stays exact when the device suppresses notifications). The 07-03 header's boot call order is unchanged.
4. **Ring bring-up order per the 07-03 contract:** Ring::new assembles the chains before DRIVER_OK; the avail posting + ONE notify happen after (never notify before DRIVER_OK, §3.1.1). The must_have truth's "pre-posts … before DRIVER_OK" reads as the chain assembly; the task's own action text ("one notify after DRIVER_OK") and the 07-03 contract govern.
5. **One-time latched link-status print from IRQ context** (ISR bit 1): the plan's explicit one-time-print carve-out; slirp's link never changes so it never fires on the gates; the Console-lock deadlock class is bounded by the latch and documented in the handler comment.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug + Rule 3 - Blocking] Device-VA arena aliasing: multi-page maps clobber each other**
- **Found during:** Task 1 (first bind boot: synchronous external abort at FAR = ECAM VA + 0x8020 — dev 1's BAR4 read)
- **Issue:** `vm::next_free_device_page4k()` bumps the device-VA arena by ONE 4 KiB page, but `deviceutil::map_device_register` maps the whole range from that VA. The 07-02 ECAM map (1 MiB) advanced the arena 4 KiB, so gic::init's 64 KiB GICD map landed INSIDE the ECAM span and overwrote the dev-1 config page's translation. Latent since 07-02 — harmless only because nothing re-read the ECAM window after gic::init until this plan's bind.
- **Fix:** `vm::next_free_device_va_span(len)` (page-rounded span reservation) + both `deviceutil` map paths use it. All arena consumers' VAs shift (gicd/gicc now map beyond the ECAM span); no VA constants exist anywhere, so the shift is invisible except that the aliasing is gone.
- **Files modified:** r9:aarch64/src/vm.rs, r9:aarch64/src/deviceutil.rs
- **Verification:** the bind boots clean (BAR readback matches, negotiation succeeds); all seven other profiles green TWICE with the shifted VAs (timer/PL011 IRQs live on every boot); workspace check/clippy green (raspi/x86_64 unaffected)
- **Committed in:** `b12f04c` (part of the Task 1 commit)

**2. [Rule 3 - Blocking] `fdt::property_range_iter` cannot read the PCI `ranges` property**
- **Found during:** Task 1 pre-implementation ground-truthing (fdt.rs line 270: `address_cells > 2` terminates iteration; the pci node declares `#address-cells = <0x03>`)
- **Issue:** the plan's action text reads the 64-bit MMIO window "via the existing property_range_iter" — that helper supports ≤ 2-cell child addresses; PCI needs 3.
- **Fix:** `mmio64_window()` in pci.rs walks the property's u32 cells directly, stride derived from the declared cell counts (3 child + 2 parent + 2 size), selecting the phys.hi space-bits-0b11 tuple. The BAR base still comes from the DTB, never a constant. fdt.rs untouched (outside plan scope).
- **Files modified:** r9:aarch64/src/platform/virt/pci.rs
- **Verification:** BAR4 = 0x8000000000 on both committed DTBs (the lowecam ranges are byte-identical); the bind boots on both windows
- **Committed in:** `b12f04c`

**3. [Rule 1 - Bug] BAR size operator-precedence bug (caught by the gates before commit)**
- **Found during:** Task 1 verification (boot log: `bar4 size 0x3ffe` + the cap bounds check aborting the bind)
- **Issue:** `!(mask & !0xF).wrapping_add(1)` computes `!(x + 1)` — method call binds tighter than `!` — yielding 0x3FFE for qemu's 16 KiB BAR; the (correct) T-07-11 cap bounds check then refused the notify cap at 0x3000+0x1000.
- **Fix:** `(!(mask & !0xF)).wrapping_add(1)` → size 0x4000; the caps walk then validates and the bind completes.
- **Files modified:** r9:aarch64/src/platform/virt/pci.rs
- **Verification:** fresh log carries `pci: bar4 pa 0x8000000000 size 0x4000 readback 0x8000000000` + the cap-set print + the negotiated-mask line + the bind line
- **Committed in:** `b12f04c`

**4. [Rule 3 - Blocking] Plan's verify commands omit `--arch aarch64`**
- **Found during:** Task 1 verification (the 07-01/07-02/07-05 class — xtask `Arch::from` defaults to x86_64)
- **Issue:** `cargo xtask boot-test --config virt` as written looks for `x86_64/lib/config_virt.toml` (absent); every historical invocation carries the flag.
- **Fix:** ran all boot-test invocations as `cargo xtask boot-test --arch aarch64 --config <p>`.
- **Files modified:** none
- **Verification:** all boots run the intended aarch64 profiles
- **Committed in:** n/a

---

**Total deviations:** 4 auto-fixed (2 bug, 2 blocking — one shared-class plan-command correction). **Impact on plan:** the arena fix is a genuine latent-bug repair that every future multi-page device map depends on (Rule 1 at its best); the ranges parse is the plan's intent via a working mechanism; the precedence fix is the gates doing their job. No scope creep — every plan truth holds as measured.

## Issues Encountered

- The Task-1 bind's first boot faulted (external abort) — root-caused to the arena aliasing (deviation 1) via FAR arithmetic + the DTB reg sizes, not by trial: the probe print showed the FIRST re-read of dev-1 config space aborting after gic::init while the same read worked during discovery, and the GICD frame (0x10000 = 16 pages) maps exactly over the ECAM dev-1 page under the one-page bump.
- Pre-existing warnings observed and deliberately untouched (scope boundary): raspi3b uartpl011 unused import, riscv64 unused import, port test unused import, the aarch64 dead-code set (22 on config builds). My additions added zero net warnings (the io.rs widths ride the file's `allow(dead_code)` precedent; the state's 07-07-facing fields carry targeted allows naming their consumer).
- NETDRV-02's REQUIREMENTS.md checkbox is shared with 07-03 — the ready-ids gate releases it with this SUMMARY (07-03's exists).

## User Setup Required

None - no external service configuration required. (R9_HARVEY_DIR is the established cross-repo channel; the verify commands set it explicitly.)

## Next Phase Readiness

- **07-07 (devether `#l` + /net/ether0):** the device is live (DRIVER_OK, rings posted, IRQ armed); the published state carries the MAC, both rings, the notify offsets, the RX buffer pool, and the nintr/nnote counters — everything the file contract's data path and ifstats file read. The TX post path (`Ring::tx_post` + `tx_notify_off`) is in place awaiting its syscall-context caller.
- **07-08 (Harvey compose + evidence):** the packaged virtshell kernel binds the NIC (dev = ["exec", "bootchain", "ether"]); the virtnet gate turns green the moment the #l device exists (every assertion is already unit-proven).
- **07-08 note:** the arena fix changes device-VA placements globally — no consumer depends on VAs (all discovered), so nothing downstream adjusts.
- **Coverage watch-item (D6):** the handler is armed-and-inert-proven; traffic-level proof lands with 07-08's NETDRV-04 gates.
- No blockers.

## Self-Check: PASSED

- Commits (r9): `b12f04c` (Task 1), `92e9b26` (Task 2) FOUND in `git log`; measured count `git rev-list --count 993be67..92e9b26` = 2 ✓
- Acceptance greps (re-run post-commit): io.rs `read_reg8|write_reg8` = 2; ICFGR gicv2 = 5 / gicv3 = 7 with the write preceding ISENABLER in both diffs; no hardcoded-36 matches in trap.rs/pci.rs; `dev_ether` in main.rs = 2; `rw_kernel_data` in pci.rs = 2 (the only `rw_device` match is the anti-pattern comment); `virtio_net`/`PCI_COMMAND`/`CommonCfg` present; the handler's first device read is `io::read_reg8(&state.isr, 0)`; config_virt.toml carries `dev = ["ether"]`; all nine virt configs grep for ether, raspi4b/default zero
- Gates re-run at summary time: boot-test virt PASS + DRIVER-BIND-OK greps (bind line, `negotiated 0x100010020`, discovery line) ×2 (task gate + tracer re-run); seven-profile regression loop PASS ×2; ALL-CONFIGS-BIND-OK (virtexec + virtexec3 bind lines); RED-PATH-STILL-CORRECT (virtnet exit≠0, `netsweep: FAIL`); virtpanic markers + 31 register rows; unhandled-IRQ count 0 on all eight logs; `cargo xtask test` 45/45, check, clippy all exit 0 ×2
- r9 worktree clean of plan files post-commit (`git status --porcelain` empty); pre-existing harvey-repo dirty files untouched

---
*Phase: 07-r9-pci-ecam-virtio-net-devether*
*Completed: 2026-11-05*
