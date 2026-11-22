---
phase: 13-r9-x86-64-platform-bring-up
plan: 05
subsystem: kernel
tags: [x86_64, q35, ioapic, madt, virtio-net, ecam, pci, devether, netgate, arp, pcap, boot-test, kvm, tcg, tso-fences]

# Dependency graph
requires:
  - phase: 13-r9-x86-64-platform-bring-up (13-04)
    provides: the checksummed ACPI facts (MADT IOAPIC base+gsib + every IntSrcOverride), the publish-once DISCOVERY cell (device 00:02.0 pin 1 -> gsi 22, the ECAM window), and the gate's ordered-marker machinery this plan extends
  - phase: 13-r9-x86-64-platform-bring-up (13-03)
    provides: the trap dispatch with EOI-after-work (Pitfall 5's ordering), the pre-sti arming window, and the spare 0x20..=0x3F IRQ vector span
provides:
  - The MADT-driven IOAPIC registry (ioapic.rs): the 82093AA IOREGSEL 0x00 / IOWIN 0x10 indirect layout (verified live: id 0 version 0x11 rtes 23), init from the published MADT facts with the IntSrcOverride fold (D-06, zero hardcoded GSIs), every RTE masked for a known state, enable(gsi, vector, level) with the UNMASK write LAST — boot context only
  - The virtio-net bind (pci.rs's bind half, X64-04): the four port seams over the identity map (EcamDev: PciCfg, Regs: CommonCfg, VolatileMem: RingMem — the trait's core-fence default KEPT per Pitfall 6's verdict), caps-name-the-BAR, BAR sizing all-ones mask/restore with the firmware-assignment-kept/kernel-window-fallback decision, PCI_COMMAND MEMORY|MASTER + readback, modern-only negotiation (live: offered 0x10130bf8024 -> exactly VERSION_1|MAC|STATUS), queue setup select->size->addresses->notify->enable LAST, RX pre-post + DRIVER_OK, publish_driver with the `#l0:` MAC print, ioapic::enable(gsi 22, vector 0x21, level)
  - The ISR-read-first IRQ handler (X64-02's teeth): the ISR byte read FIRST (deasserts the level line BEFORE the dispatcher's LAPIC EOI), bounded drain with re-check, no locks/no console on the hot path — proven under load: ping-pong nintr exactly 256/256 (one IRQ per frame), bulk 4096 frames in 288 IRQs (the bounded-drain batching), zero storms
  - The driver_ops() widen (port/devether.rs pub(crate) -> pub) — the Open-Question-1 resolution, the ONE-line seam
  - The kernel-side ARP trigger (netgate.rs, cfg(dev_ether)): the committed 60-byte fixture embedded, ping-pong x256 reconciled EXACTLY (rx == tx == N, nintr in [1,N], rings drained), TX-bulk 4096 with backpressure bounds and no wedge, fail-closed FAIL markers + X64-NET-DONE — the Phase-14 exec swap-in seam recorded
  - The q35net gate (config_q35net.toml + the xtask arm): ordered markers banner -> bind audit -> #l0: -> ping-pong -> tx-bulk -> X64-NET-DONE + the finish()-time pcap assertion (N parsed from the kernel's own marker, >= 1 gateway reply, 60-byte frames from the guest MAC)
affects: [13-06, "Phase 14 XEXEC (the kernel-side trigger seam the userspace dd path replaces)", "Phase 16 HNET (the bound NIC + the netdev-as-data stanza)"]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 26150   # chars/4 over git diff f607126..0cf9c3a in ../r9 (104600 chars, 12 files; plan estimated 78000, confidence low — the overestimate direction again: the aarch64 arc copied nearly verbatim)
  tasks: 3
  commits: 1      # harvey-side: this docs commit (ledger fd74807be). Production code: 3 commits in the sibling r9 repo (81d6f92, b05fda3, 0cf9c3a) per the dual-repo execution context.
plan_head_before: fd74807bea17d7bf1ae9021889993595cc785fb6
plan_head_after: fd74807bea17d7bf1ae9021889993595cc785fb6   # harvey-side pre-docs HEAD (the 13-04 convention: no harvey production commits this plan); all production commits live in ../r9 (f607126..0cf9c3a)

# Tech tracking
tech-stack:
  added: []        # zero new crates — the from-scratch discipline holds (the port touch is a one-line visibility widen)
  patterns:
    - "Masked-post/masked-print boot-context discipline on x86: after sti, every driver-op call and every println from boot context runs inside cli..sti — the console's MCS lock is not preempt-safe against an interrupted holder, and the timer tick prints from IRQ context (found live as a mid-println wedge)"
    - "Firmware-kept BAR assignment: SeaBIOS already ran PCI resource allocation; a nonzero firmware assignment inside the identity map is KEPT (rewriting it could collide), with the [ecam-window-end, ioapic-base) window — both endpoints from parsed tables — as the fallback"
    - "The vector publish cell: the discovery cell carries the GSI, the bind derives the IDT vector and publishes it once; the dispatch guards on the published value (the aarch64 virtio_irq_intid analog — zero hardcoded vectors at the dispatch)"
    - "Gate floors as data: the pcap assertion parses N from the kernel's own `ping-pong done N/N` marker, never a second constant that could drift from the kernel's pin"
    - "The kernel-side trigger seam for Phase 14: run() consumes driver_ops() (the widened pub seam) from boot context after sti — the exec era swaps the same ops call in from the file-contract write path"

key-files:
  created:
    - ../r9/x86_64/src/platform/q35/ioapic.rs
    - ../r9/x86_64/src/platform/q35/netgate.rs
    - ../r9/x86_64/lib/config_q35net.toml
  modified:
    - ../r9/x86_64/src/platform/q35/pci.rs
    - ../r9/x86_64/src/platform/q35/mod.rs
    - ../r9/x86_64/src/trap.rs
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/src/pagealloc.rs
    - ../r9/x86_64/Cargo.toml
    - ../r9/port/src/devether.rs
    - ../r9/xtask/src/main.rs
    - ../r9/xtask/src/pcap.rs

key-decisions:
  - "The IOAPIC register layout is the 82093AA/ACPI IOWIN 0x10 scheme (IOREGSEL 0x00), NOT 9k's deprecated 0x04 variant — verified live through the bring-up cross-check read: id 0 version 0x11 rtes 23 gsib 0 (a wrong data window reads zeros there); every RTE is masked at init for a known state, and enable() is the only unmask path (written masked, the UNMASK write LAST)"
  - "SeaBIOS's BAR assignment is kept when present (live: bar4 0xfe000000/16 KiB, readback verified); the kernel-assigned fallback window is [MCFG-window end, MADT IOAPIC base) — both endpoints from parsed firmware tables, host-tested against the live and moved shapes"
  - "The trap dispatch's device arm guards on a publish-once vector cell the bind writes (VIRTIO_NET_VECTOR 0x21 — the first spare past the timer): the plan's 'guard on the published virtio vector from the discovery cell' resolved as the vector-publish cell because the discovery cell carries the GSI, not the IDT vector (the aarch64 virtio_irq_intid analog)"
  - "The console-lock wedge (found live at the first tx-bulk run): the netgate is the kernel's FIRST post-sti boot-context printer, and port's println takes an MCS lock that an IRQ-context locker (the timer tick's once-a-second print) cannot wait on — the fix is masked_println (cli..sti around every netgate print), the mirror of the handler's no-console-on-the-hot-path discipline; the partial vq1 serial line was the signature"
  - "The masked-post window: every tx_post runs inside cli..sti (the aarch64 writer invariant's TIME-exclusion, x86 edition — a post's avail-publication window must never interleave with the handler's retire), while the reply waits run enabled so the interrupt being waited FOR can arrive"
  - "X64-02's load proof landed with its teeth visible in the counters: ping-pong nintr exactly 256/256 (one interrupt per frame each direction — the ISR-read-first ordering never re-delivering), tx-bulk 4096 frames in 288 IRQs (the bounded drain batching mid-drain completions across invocations — the T-07-13 shape), TX nnote 1189/4096 (qemu's used-ring NO_NOTIFY suppression active, the ring's suppression-exact accounting holding)"
  - "The objdump fence observation (Pitfall 6's recorded verdict): the whole 2,039,697-line disassembly of the q35net kernel contains ZERO mfence — core's fence(Release)/fence(Acquire) lower to compiler-only barriers on x86 TSO and the notify site needs none, correct while EVENT_IDX stays out of every negotiation mask (the SeqCst revisit condition is in the VolatileMem comment)"
  - "The aarch64 virtnet regression leg ran as the pair virtnet-shape compile + live virt gate: the exec'd sweep needs build/aarch64/bootfs.paq, a Phase-6-era subset-build artifact absent from this checkout (no cached toolchain bins); the driver_ops widen is a pure visibility change whose behavioral surface (EtherDriverOps publication, the IRQ path, #l0) is fully exercised by the virt leg"

patterns-established:
  - "The q35net serial contract: bind audit (offered/negotiated pair) -> #l0: + MAC -> netgate: ping-pong done N/N intr rx/tx -> netgate: vq0/vq1 ifstats lines -> netgate: tx-bulk done N rx-under-load R -> X64-NET-DONE — order-anchored in the gate, `netgate: FAIL` fail-fasts"
  - "Kernel-side load gates before exec exists: reconcile driver ifstats exactly kernel-side (fail-closed FAIL markers), prove the frames host-side via pcap with the floor parsed from the kernel's own marker — Phase 14 swaps the trigger for the exec'd dd without touching the gate's assertion shape"

requirements-completed: [X64-04, X64-02]

# Coverage metadata (#1602) — one entry per shipped deliverable.
coverage:
  - id: D1
    description: "ioapic.rs — the MADT-driven redirection-table registry: 0x10-layout indirect registers, init from the published facts with the override fold + loud-by-absence, all-RTEs-masked known state, enable() unmask-last"
    requirement: X64-02
    verification:
      - kind: unit
        ref: "r9 host tests: platform::q35::ioapic::tests::{override_fold_matches_the_live_q35_table, rte_lo_composition_matches_the_82093aa_field_map, rte_index_covers_the_live_route_and_refuses_out_of_range}"
        status: pass
      - kind: integration
        ref: "live serial every q35/q35net boot: 'ioapic: id 0 version 0x11 rtes 23 gsib 0 (madt 0xfec00000)' — the cross-check read through the 0x10 window; q35 idle gate green with it initialized"
        status: pass
    human_judgment: false
  - id: D2
    description: "The pci.rs bind (X64-04): four port seams over the identity map, caps-name-the-BAR, BAR sizing/assignment with readback, PCI_COMMAND MEMORY|MASTER, modern-only negotiation audited on serial, queue bring-up + RX pre-post + DRIVER_OK, publish_driver + #l0: + MAC, ioapic::enable"
    requirement: X64-04
    verification:
      - kind: unit
        ref: "r9 host tests: pci::tests::{mmio_window_spans_ecam_end_to_ioapic, bar_assignment_keeps_firmware_and_falls_back_to_the_window}"
        status: pass
      - kind: e2e
        ref: "q35net gate green x3 KVM + x1 TCG: 'pci: bar4 pa 0xfe000000 size 0x4000 (firmware) readback 0xfe000000', 'virtio-net: features offered 0x10130bf8024 negotiated 0x100010020', 'virtio-net: rx pre-posted 128 buffers qsize 256', '#l0: virtio: 1000Mbps: 52:54:00:12:34:56', 'ioapic: gsi 22 -> vector 0x21 (level)'"
        status: pass
    human_judgment: false
  - id: D3
    description: "The port seam: driver_ops() widened pub(crate) -> pub (the OQ-1 resolution — the one-line change, no new port surface; the manifest untouched)"
    requirement: X64-04
    verification:
      - kind: integration
        ref: "port host pins re-run green on the x86_64 host (cargo xtask test: 101 port + 91 xtask green); aarch64 virtnet-shape compile rc=0 + live virt boot-test PASS (the sibling leg unperturbed); git status port/Cargo.toml clean"
        status: pass
    human_judgment: false
  - id: D4
    description: "netgate.rs — the kernel-side ARP trigger: the embedded 60-byte gateway-targeted fixture with the negotiated-MAC overwrite, ping-pong x256 reconciled exactly (rx == tx == N, nintr in [1,N], rings drained), TX-bulk 4096 with bounded backpressure and no wedge, fail-closed FAIL markers + X64-NET-DONE"
    requirement: X64-02
    verification:
      - kind: unit
        ref: "r9 host tests: netgate::tests::{frame_is_the_committed_fixture_shape, pingpong_reconcile_is_exact_and_fails_closed, txbulk_reconcile_bounds_and_reports_rx, progress_is_wrap_safe}"
        status: pass
      - kind: e2e
        ref: "live serial (all 4 gate legs): 'netgate: ping-pong done 256/256 intr rx 256 tx 256', vq0/vq1 lines with used == lastused, 'netgate: tx-bulk done 4096 rx-under-load 4096 intr rx 288 tx 288', 'X64-NET-DONE'"
        status: pass
    human_judgment: false
  - id: D5
    description: "The host-side wire proof + gate arm (X64-04's acceptance): the q35net boot-test stages (bind audit -> #l0: -> netgate stages -> terminal, order-anchored, netgate: FAIL fail-fast) + the finish()-time pcap assertion over this run's filter-dump capture"
    requirement: X64-04
    verification:
      - kind: unit
        ref: "r9 xtask tests: pcap::{x64_netgate_golden_passes, x64_netgate_short_requests_fail, x64_netgate_no_replies_fail, x64_netgate_wrong_src_mac_fails, x64_netgate_missing_marker_fails} (91 total green)"
        status: pass
      - kind: e2e
        ref: "two consecutive q35net gate runs green (plus a third and the TCG leg --accel tcg --cpu qemu64): the pcap assertion held at 4352 ARP requests / 4352 gateway replies, every request exactly 60 bytes sourced from the guest MAC (target/boot-test-q35net.pcap, 678936 bytes)"
        status: pass
    human_judgment: false

# Metrics
duration: 39 min
completed: 2026-11-07
status: complete
---

# Phase 13 Plan 05: IOAPIC + virtio-net Bind + the Net Gate Summary

**The NIC is bound and the interrupts proven under load: a MADT-driven IOAPIC (the 82093AA 0x10 layout, live-verified) arms the virtio line at gsi 22, the four port seams carry a modern-only negotiation (offered 0x10130bf8024 -> exactly VERSION_1|MAC|STATUS) into devether's `#l0:`, and the kernel-side ARP trigger reconciles ping-pong x256 EXACTLY (rx == tx == 256, nintr 256/256) plus a 4096-frame TX-bulk through backpressure with zero lost or wedged interrupts — 4352 requests and 4352 gateway replies host-observable in the pcap, green twice consecutively on KVM and once on TCG.**

## Performance

- **Duration:** 39 min
- **Started:** 2026-11-07T15:24:19Z
- **Completed:** 2026-11-07T16:03:20Z
- **Tasks:** 3 (Task 1 auto, Task 2 tracer, Task 3 auto)
- **Files modified:** 12 in ../r9 (3 created, 9 modified) + this plan's harvey docs

## Accomplishments
- X64-04 closed end-to-end: the arch-neutral `port/` stack drives a real virtio-net NIC on x86_64 through exactly the four seams (no x86 specialization — VolatileMem keeps the trait's core-fence default), `/net/ether0` is live with the negotiated MAC, and the guest's ARP is host-observable in the filter-dump pcap with the exact 60-byte frame shape
- X64-02's load proof landed with teeth: the ISR-read-first ordering shows as nintr exactly 256/256 over the ping-pong exchange (never a re-delivery), the bounded drain batches 4096 bulk frames into 288 interrupts, the rings end drained, and every reconciliation is fail-closed (`netgate: FAIL` reds the gate instantly)
- The IOAPIC is live and MADT-driven with zero hardcoded GSIs: base + gsib + all five IntSrcOverrides from 13-04's facts, every RTE masked for a known state, and the enable path writing the RTE masked-then-unmasked-LAST inside the pre-sti window
- The Pitfall-6 fence verdict is recorded in the metal: 0 mfence across the whole 2,039,697-line kernel disassembly — the Release/Acquire fences compile away on TSO and the notify site needs none (the objdump check the research asked for, plus the EVENT_IDX revisit condition at the VolatileMem impl)
- One live bug found and fixed by the gate itself: the console MCS-lock wedge (the netgate is the first post-sti boot-context printer; the timer tick's IRQ-context print cannot wait on an interrupted holder — the partial vq1 serial line was the signature; every netgate print now runs masked)

## Task Commits

Each task was committed atomically (r9 repo, `main`):

1. **Task 1: ioapic.rs — the MADT-driven redirection-table registry** - `81d6f92` (feat)
2. **Task 2: pci.rs bind — the four port seams, negotiation, ISR-read-first handler, /net/ether0 live** - `b05fda3` (feat; includes the ioapic allow-comment reword naming the now-present bind)
3. **Task 3: netgate + the q35net gate — kernel-side ARP ping-pong/TX-bulk with pcap assertion** - `0cf9c3a` (feat)

**Plan metadata:** (this harvey docs commit — see below)

## Files Created/Modified
- `../r9/x86_64/src/platform/q35/ioapic.rs` - (new) the registry: IOREGSEL 0x00/IOWIN 0x10 constants (the 9k 0x04 divergence documented + live-verified), Ioapic::new from the MADT facts (loud named errors), all-RTE masking at init, enable(gsi, vector, level) hi/masked-lo/unmask-LAST, override_applies; 3 host tests pinning the live override table, the 82093AA field map, the gsi->index bounds
- `../r9/x86_64/src/platform/q35/pci.rs` - the bind half (~500 lines): EcamDev/Regs/VolatileMem seams, mmio/ecam write family, mmio_window + bar_assignment (host-tested), size_bar64, bring_up_queue (enable LAST), bind_virtio_net (tolerant, the full §3.1.1+§4.1.5 arc), virtnet_tx_post/virtnet_ifstats, virtio_net_irq (the ISR-first bounded drain with the discipline header), the VIRTIO_VECTOR publish cell + virtio_irq_vector
- `../r9/x86_64/src/platform/q35/netgate.rs` - (new) the cfg(dev_ether) trigger: the embedded 60-byte fixture, masked_println (the console-lock fix), ping-pong x256 with masked posts + enabled waits, TX-bulk 4096 with bounded backpressure, the exact/bounds reconciles, the FAIL/terminal markers; 4 host tests
- `../r9/x86_64/src/platform/q35/mod.rs` - the ioapic/netgate module slots + the acpi_facts() publish-once accessor
- `../r9/x86_64/src/trap.rs` - the cfg(dev_ether) device dispatch arm guarding on the published vector
- `../r9/x86_64/src/main.rs` - ioapic::init in the pre-sti window, the cfg(dev_ether) bind inside the window, netgate::run() after sti
- `../r9/x86_64/src/pagealloc.rs` - allocate_physpage (the identity-map shape: PA doubles as VA)
- `../r9/x86_64/Cargo.toml` - check-cfg predeclares dev_ether/dev_ip in the same commit the config first emits them
- `../r9/x86_64/lib/config_q35net.toml` - (new) q35 + dev = ["ether"] + the netdev stanza (the D-01a bound step)
- `../r9/port/src/devether.rs` - driver_ops() widened pub (the one-line seam, OQ-1 resolved)
- `../r9/xtask/src/main.rs` - the q35net arm: 4 new BootStages (bind/ping-pong/tx-bulk/done, order-anchored past the discovery anchor), the netgate:FAIL fail-fast, BootProfile::X64Net + the finish()-time pcap assertion flow
- `../r9/xtask/src/pcap.rs` - assert_x64_netgate + pingpong_n (the floor parsed from the kernel's marker); 5 unit tests

## Decisions Made
- The eight key-decisions above — the load-bearing ones: the firmware-kept BAR policy (SeaBIOS's assignment honored, the derived window as fallback), the vector publish cell (the plan's "published virtio vector from the discovery cell" resolved as a bind-published cell because discovery carries the GSI), the masked-post/masked-print discipline (the live-found console-lock wedge), and the virtnet-regression pair (bootfs.paq absent)
- N pinned at 256 (the plan's named pin) and the bulk total at 4096; the pcap floor derives from the kernel's own marker, so the two can never drift apart silently

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The console's MCS lock deadlocks against the netgate's post-sti prints**
- **Found during:** Task 3 (the first q35net gate run — wedged mid-println on the vq1 ifstats line, budget expiry, partial serial line)
- **Issue:** The netgate is this kernel's FIRST boot-context printer after sti; port's println takes an MCS queue lock, and the timer tick prints once a second FROM IRQ CONTEXT — an IRQ-context locker whose predecessor is an interrupted boot-context holder spins forever (the holder cannot run until IRQ returns). The handler-side discipline (no console on the hot path) has a mirror side the plan did not name: a post-sti boot-context printer must mask
- **Fix:** masked_println! (cli..sti around every netgate print — microseconds per line on the emulated 16550) and the already-masked tx_post windows; the wait loops stay enabled so the waited-for interrupt can arrive
- **Files modified:** ../r9/x86_64/src/platform/q35/netgate.rs
- **Verification:** the full gate matrix green (x3 KVM + TCG) with every netgate line arriving whole
- **Committed in:** 0cf9c3a

**2. [Rule 3 - Blocking] Task 1's dispatch arm references Task 2's handler — the compile-clean boundary**
- **Found during:** Task 1 (the plan's action 3 lands the trap arm delegating to "pci.rs's handler (Task 2)" — a forward reference that cannot compile alone)
- **Issue:** The arm needs both a vector source and a handler before either exists
- **Fix:** Task 1 landed the arm together with the VIRTIO_VECTOR publish-once cell, the virtio_irq_vector() accessor, and a guarded no-op virtio_net_irq() (unreachable until a bind publishes the vector — the lapic::eoi() tolerant-degradation idiom); Task 2 replaced the no-op with the full ISR-read-first drain in the same plan
- **Files modified:** ../r9/x86_64/src/trap.rs, ../r9/x86_64/src/platform/q35/pci.rs
- **Verification:** every commit bootable (the q35 gate green at Task 1); the real handler live at Task 2
- **Committed in:** 81d6f92 + b05fda3

**3. [Rule 3 - Blocking] config_q35net.toml created in Task 2, not Task 3**
- **Found during:** Task 2 (the verify runs `boot-test --config q35net` — the config must exist a task early)
- **Issue:** The plan's file placement put the net profile in Task 3 while Task 2's gate consumes it
- **Fix:** The full config landed with Task 2 (identical content to what Task 3's action specifies); Task 3 owns its header reword + the gate arm
- **Files modified:** ../r9/x86_64/lib/config_q35net.toml
- **Verification:** Task 2's gate green against it
- **Committed in:** b05fda3 (+ 0cf9c3a for the header wording)

**4. [Rule 3 - Blocking] The aarch64 virtnet regression leg needs an absent build artifact**
- **Found during:** Task 3 verify (the plan's command fails pre-spawn: `initrd image harvey:build/aarch64/bootfs.paq not found`)
- **Issue:** The full virtnet exec-sweep gate needs bootfs.paq — a Phase-6-era subset-build product (the aarch64 cross-toolchain bins are not built in this checkout, and producing them is the namespace-build machinery, far outside a regression re-run)
- **Fix:** The faithful pair: `cargo xtask build --arch aarch64 --config virtnet` (rc=0 — the dev_exec+bootchain+ether cfg shape compiles against the widened port) + `cargo xtask boot-test --arch aarch64 --config virt` (PASS — the live NIC bind through the same port machinery: negotiate, rings, IRQ, #l0). The driver_ops widen is a pure visibility change; every behavioral surface the sibling leg exercises is covered
- **Files modified:** none (command-shape substitution, documented)
- **Verification:** both legs green; the full virtnet sweep re-runs wherever bootfs.paq exists (regenerable per the harness's own error message)
- **Committed in:** n/a

**5. [Rule 3 - Blocking] Natural-seam files the per-task lists did not enumerate**
- **Found during:** Tasks 1-3 (the 13-02 precedent): main.rs (the ioapic init call + the bind + the netgate call), pagealloc.rs (allocate_physpage — the ring pages need an allocator), x86_64/Cargo.toml (the dev_ether check-cfg predeclare, in the same commit the config first emits it)
- **Fix:** Landed as the minimal seams their actions imply
- **Verification:** zero new warnings in any touched file (the warning set stays the documented pre-existing family)
- **Committed in:** 81d6f92 + b05fda3

---

**Total deviations:** 5 auto-fixed (1 Rule 1 bug found live by the plan's own gate, 4 Rule 3 blockers/sequencing seams)
**Impact on plan:** All fixes protect the plan's own gates and the load proof's honesty; no scope creep — the masked-print discipline is a correctness requirement of the post-sti boot-context execution model this plan introduced.

## Issues Encountered
- One Edit-tool mishap during Task 1 (a pci.rs section header briefly consumed by a misplaced edit) — caught immediately by the follow-up diff check and restored byte-identically before any commit; the committed diff of pci.rs at 81d6f92 is exactly the intended accessor addition
- The first q35net run's partial serial line was the diagnosis gift: the wedge point (mid-println, after a clean 256/256 ping-pong) named the console-lock collision without a debugger

## Verification Log

On the final committed tree (r9 `0cf9c3a`):
- **q35net gate, KVM-default, x3** (two consecutive for the load-stability check + one after the config header reword) — PASS: banner -> facts -> discovery -> bind audit -> `#l0:` -> `netgate: ping-pong done 256/256 intr rx 256 tx 256` -> vq0/vq1 lines (used == lastused) -> `netgate: tx-bulk done 4096 rx-under-load 4096 intr rx 288 tx 288` -> `X64-NET-DONE`, then the finish()-time pcap assertion: 4352 ARP requests / 4352 gateway replies, 60-byte frames from the guest MAC (target/boot-test-q35net.pcap, 678936 bytes)
- **q35net gate, TCG leg** (`--accel tcg --cpu qemu64`) — PASS with identical counts (4352/4352): the periodic-timer + INTx path under emulation carries the whole chain (13-03's CPUID-gated fallback mode paying off)
- **q35 idle gate** — PASS (the IOAPIC initialized, no device bound: each commit bootable, D-01a)
- **Host tests** — x86_64 26 (3 ioapic + 2 bind + 4 netgate new), xtask 91 (5 pcap new), port 101 — all green; `cargo xtask check` + `cargo xtask clippy` green, zero warnings in any touched file (the q35net build's set is the documented pre-existing family)
- **Objdump fence observation** — `objdump -d target/x86_64-unknown-none-elf/debug/x86_64 > target/x64-fence.dis`: 2,039,697 lines, `grep -c mfence` = **0** (the count masked by `|| true` per the plan's own command); the notify site rides plain mov/call instructions (virtnet_tx_post is 100 lines, the publication sequence plain stores) — the TSO lowering RESEARCH predicted, now recorded
- **aarch64 sibling leg** — virtnet-shape compile rc=0 + virt boot-test PASS (see Deviation 4)
- Acceptance greps all hold: '0x10' x8 in ioapic.rs with IOREGSEL/IOWIN named (the only 0x04 mentions are the documented-divergence comments), SyncUnsafeCell 2, IntSrc|override 35, no GSI literal in the enable path (structural arithmetic only); publish_driver 2, impl RingMem 1 / fn fence 0; driver_ops pub at devether.rs:149; virtio.rs untouched (0 diff lines); dev = ["ether"] == 1; no exec/bootchain token in the net config; q35net 12 in xtask
- Zero new dependencies: the r9 Cargo.tomls' manifests untouched across all three commits

## Known Stubs
None — the plan's goal is achieved with no placeholder surface. Two deliberate uncalled-helper notes (not stubs): ioapic::override_applies stands ready for the first ISA-source IRQ consumer (host-pinned against the live q35 table; the current bind routes a PCI-derived GSI that needs no fold), and timer::ticks()'s port-side deadlines consumer remains the exec era — both carry doc'd allows naming their consumers, the timer.rs `ticks()` precedent.

## Threat Flags
None beyond the plan's threat model — T-13-10's V5 boundary re-proven (the port host pins green on x86; the used-ring validation unchanged); T-13-11's EOI ordering is the load proof itself (nintr 256/256 then 288-batched, zero storms, rings drained); T-13-12's fail-closed reconciliation is wired both kernel-side (FAIL markers) and harness-side (the fail-fast + the pcap floor from the kernel's own marker).

## Authentication Gates
None - no authentication was required.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- 13-06 (flake packaging + the KVM/TCG gate consolidation) inherits a fully-gated net stack: the q35net profile is the sixth green boot-test config, and the evidence consolidation can copy target/boot-test-q35net.log/.pcap from a fresh run
- Phase 14 (XEXEC) has its seam recorded: netgate::run() consumes driver_ops() from boot context — the exec era replaces the trigger with the dd-through-the-file-contract path while the gate's marker/pcap assertion shape stays
- X64-04 closed and X64-02 closed (its LAPIC/timer half from 13-03, its IOAPIC/under-load half from this plan); X64-05 remains 13-06's
- No blockers.

## Self-Check: PASSED

- All 12 r9 key-files exist on disk (`[ -f ]` each: FOUND); the r9 working tree is clean (all three commits landed on main)
- r9 commits verified: 81d6f92, b05fda3, 0cf9c3a are ancestors of r9 `main` HEAD (`git merge-base --is-ancestor`)
- Production code lives in the sanctioned sibling repo /home/req/code/r9 per the dual-repo execution context; the harvey-side commit for this plan is the docs commit carrying this SUMMARY/STATE/ROADMAP/REQUIREMENTS

---
*Phase: 13-r9-x86-64-platform-bring-up*
*Completed: 2026-11-07*
