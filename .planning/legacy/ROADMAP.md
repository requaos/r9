# Roadmap: Harvey OS — Build Revival + r9 Adoption

## Overview

v2.1 pivots the Harvey+r9 pair from aarch64 (frozen at its green v2.0 pin) to x86_64 under qemu/KVM on the new linux-x86_64 primary host, and turns Harvey's full amd64 userspace loose on the r9 kernel as the flush-out instrument. The port itself is two phases: the r9 x86_64 platform layer (recorded DTB-free boot protocol, APIC/HPET interrupts, data-driven PCI/ECAM, virtio-net over the arch-neutral `port/` stack — Phase 13) and the userspace re-derivation (amd64 ABI contract, exec, boot→init→rc to `% `, tiers + `/net` gates re-keyed to `objtype=amd64` — Phase 14). From there the inherited v2.0 payoff re-lands on the new arch in dependency order — userspace tiers + bootfs payload (15) in parallel with host net modes + the tap-leg exit gate (16) — feeding both the kernel flush-out phase (17: the open Phase-7/8 review backlog closed under real load, every fix pinned) and the in-guest p9sk1 auth domain (18). Phase 19 is the payoff: cpu service + drawterm + rio desktop on x86_64, with `nix run .#boot-r9` still the ONE command and the full chain proven from clean checkouts.

## Milestones

- ✅ **v1.0 Boot Revival** — Phases 1-5 (shipped 2026-11-04)
- ⏸ **v2.0 Full System & Drawterm** — Phases 6-12 (subsumed into v2.1 on 2026-11-06 at the x86_64 pivot: Phases 6-8 delivered complete; 9-12 inherited, re-keyed to x86_64)
- 🚧 **v2.1 x86_64 Pivot + r9 Kernel Flush-Out** — Phases 13-19 (in progress)

## Phases

**Phase Numbering:**
- Integer phases (13, 14, 15…): Planned milestone work — numbering CONTINUES from v2.0 (Phases 6-8 complete; 9-12 subsumed into v2.1's phases, re-keyed); v2.1's first phase is Phase 13
- Decimal phases (14.1, 14.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

<details>
<summary>✅ v1.0 Boot Revival (Phases 1-5) — SHIPPED 2026-11-04</summary>

- [x] Phase 1: Build Tail Green (9k amd64) (4/4 plans) — completed 2026-10-31
- [x] Phase 2: aarch64 Toolchain + libc + 9front Donor (7/7 plans) — completed 2026-11-01
- [x] Phase 3: r9 qemu-virt Platform Bring-up (6/6 plans) — completed 2026-11-01
- [x] Phase 4: r9 Userspace Exec + Harvey↔r9 ABI (8/8 plans) — completed 2026-11-02
- [x] Phase 5: Shell Boot + One-Command Integration (13/13 plans) — completed 2026-11-04

Full detail: `.planning/milestones/v1.0-ROADMAP.md`

</details>

<details>
<summary>⏸ v2.0 Full System & Drawterm (Phases 6-12) — SUBSUMED into v2.1 2026-11-06</summary>

- [x] Phase 6: Full-Tree Userspace — Wave 0 + Libraries/Core Tiers (10/10 plans) — completed 2026-11-05
- [x] Phase 7: r9 PCI/ECAM + virtio-net + devether (10/10 plans) — completed 2026-11-05
- [x] Phase 8: r9 TCP/IP Path — devip-Compatible /net (9/9 plans) — completed 2026-11-06
- Phases 9-12 were never started; their pending requirements are inherited into v2.1 (Phases 15, 16, 18, 19) re-keyed to x86_64

Full detail: `.planning/milestones/v2.0-ROADMAP.md`

</details>

**v2.1 x86_64 Pivot + r9 Kernel Flush-Out (Phases 13-19):**

- [x] **Phase 13: r9 x86_64 Platform Bring-Up** - *(cross-repo: `../r9`)* q35/pc platform layer — DTB-free boot protocol, APIC/HPET interrupts, PCI/ECAM discovery, virtio-net + devether — KVM-native acceptance gates by default (completed 2026-11-07)
- [x] **Phase 14: Harvey amd64 Userspace on r9** - amd64 Ureg/TrapFrame ABI contract, exec through the real syscall path, boot→init→rc to the `% ` prompt, tier machinery + `/net` gates re-keyed to `objtype=amd64` (completed 2026-11-09)
- [x] **Phase 15: Userspace Tiers + Bootfs Payload (amd64)** - services/GUI/long-tail tiers on the k10t1 venue then r9-x86_64; rio/acme/sam donor syscall traces; desktop payload staged in bootfs (completed 2026-11-10)
- [x] **Phase 16: Host Net Modes + Networking Exit Gate** - hermetic slirp+hostfwd default, explicit `--net=slirp|tap|vmnet`, linux tap/bridge L2-truth leg + doctor checker; the NETIP-08 tap-leg soak gate (completed 2026-11-12)
- [x] **Phase 17: r9 Kernel Flush-Out** - *(cross-repo: `../r9`)* the open Phase-7/8 CR/WR backlog + known kernel windows closed under real userspace load, every fix landing with a pin (completed 2026-11-13)
- [x] **Phase 18: In-Guest p9sk1 Auth Domain** - hash-paired bake-time domain generator, `/dev/random` from x86_64 entropy + devcap `#¤`, keyfs/authsrv on tcp/567, loopback probe green before any drawterm (completed 2026-11-14)
- [x] **Phase 19: CPU Service + Drawterm + rio Desktop** - `cpu -R` end-to-end, flake drawterm, rio in-session only, full-chain smoke + clean-checkout proof on linux-x86_64 (completed 2026-11-17)
- [x] **Phase 20: arm64 catch-up to v2.1 parity** (completed 2026-11-22)

## Phase Details

### Phase 13: r9 x86_64 Platform Bring-Up
**Goal**: r9 boots and speaks Ethernet on x86_64 — a q35/pc platform layer with a recorded DTB-free boot protocol, live APIC/HPET interrupts, data-driven PCI/ECAM discovery, and virtio-net bound through the arch-neutral `port/` stack — with KVM-native acceptance gates as the default. **Kernel work in sibling repo `../r9`.**
**Depends on**: Nothing in v2.1 (v2.0 complete — `port/` is arch-neutral and carries whole; the lean `x86_64/` crate — uart16550/pio/trap/vsvm — is the entry point)
**Requirements**: X64-01, X64-02, X64-03, X64-04, X64-05
**Success Criteria** (what must be TRUE):
  1. r9 boots on qemu x86_64 via a recorded DTB-free boot protocol — the q35-vs-pc machine decision recorded with rationale, the protocol decided against (or explicitly reusing) 9k's existing multiboot path — and a committed boot-test gate asserts banner → timer ticks → serial input echo in order, green on repeated runs (X64-01)
  2. APIC interrupt plumbing (LAPIC + IOAPIC) and HPET/PIT timers are proven under load, and INTx level-triggered virtio ISR read-to-ack is re-proven — a sustained IRQ-load run shows no lost or wedged interrupts (X64-02)
  3. PCI/ECAM discovery works without DTB (ACPI MCFG or the fixed q35 window) with zero hardcoded addresses — the moved-window perturbation boot-test still finds and binds the device (X64-03)
  4. virtio-net binds on x86_64 through the arch-neutral `port/` transport and devether presents `/net/ether0` with live RX/TX counters — the guest's ARP request is observable in a host-side capture (X64-04)
  5. r9's flake ships an x86_64 kernel package and the acceptance gates run KVM-native by default, with the TCG fallback documented (X64-05)
**Plans**: 6/6 plans complete
Plans:
**Wave 1**
- [x] 13-01-PLAN.md — q35 boot channel: config_q35.toml, TOML-driven -cdrom xtask arm, grub-mkrescue ISO, mb2 header in l.S — boot-test x86_64 banner gate green twice (X64-01)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 13-02-PLAN.md — main9-x64 rewrite + platform/q35 tree + mb2 parser + old-gen retirement — memory map on serial, gated (X64-01)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 13-03-PLAN.md — LAPIC + CPUID-gated TSC-deadline/periodic timer + IDT dispatch + serial echo — the Phase-3-style gate green twice, KVM + TCG (X64-01, X64-02)

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 13-04-PLAN.md — ACPI mini-parser (RSDP→XSDT→MCFG/MADT) + PCIEXBAR-programming ECAM discovery + recursive walk — the moved-window perturbation gate (X64-03)

**Wave 5** *(blocked on Wave 4 completion)*
- [x] 13-05-PLAN.md — MADT-driven IOAPIC + four-seam virtio-net bind + devether /net/ether0 + kernel-side ARP netgate — ping-pong/TX-bulk reconcile + pcap gate (X64-04, X64-02)

**Wave 6** *(blocked on Wave 5 completion)*
- [x] 13-06-PLAN.md — flake kernel-x64 package + reproducible boot-iso + KVM-native gate loop + TCG fallback docs + CI leg + 13-EVIDENCE.md consolidation (X64-05)

### Phase 14: Harvey amd64 Userspace on r9
**Goal**: The core value lands on x86_64 — Harvey amd64 binaries exec on r9 through the re-derived ABI contract, the boot chain reaches the interactive `% ` prompt via one command, and the tier + `/net` gate machinery runs keyed to `objtype=amd64` (no new toolchain work — 6c/6a/6l already green). **Work spans both repos plus the flake glue.**
**Depends on**: Phase 13
**Requirements**: XEXEC-01, XEXEC-02, XEXEC-03, XEXEC-04, XEXEC-05
**Success Criteria** (what must be TRUE):
  1. The amd64 Ureg/TrapFrame ABI contract is published and build-enforced — the `aarch64/ABI.md` equivalent with offsets frozen, const asserts + xtask ureg-check live on every gate, and a negative control firing on drift (XEXEC-01)
  2. Harvey amd64 binaries exec on r9 x86_64 through the real EL/syscall path — the aarch64 measured-set dispatch carries over, proven by staged-init + hello-equivalent execution evidence on serial (XEXEC-02)
  3. The full boot chain boot(8)→init→rc reaches the interactive `% ` prompt on x86_64 — the expect smoke gate (boot-r9-smoke equivalent) is green twice consecutively (XEXEC-03)
  4. The tier machinery runs keyed to `objtype=amd64`: tier-0 libraries + tier-1 core commands prove executing on r9-x86_64 with committed artifact+execution markers, donor venue = Harvey's own 9k kernel (k10t1) (XEXEC-04)
  5. `/net` is re-gated on x86_64: loopback ping/udp/tcp echo, static + DHCP ipconfig, and the composed net smoke are all green as committed gates on the new arch (XEXEC-05)
**Plans**: 9/9 plans complete
Plans:
**Wave 1**
- [x] 14-01-PLAN.md — ABI contract freeze: the 184-byte 9k-verbatim Ureg rework + witness re-derivation, amd64/ABI.md publication, ureg-check amd64 arm + NEGATIVE CONTROL + the x86_64 boot-test gate — every Phase-13 gate re-green (XEXEC-01, D-01/D-02/D-02b)

**Wave 2** *(blocked on Wave 1 — the entry asm implements the frozen frame)*
- [x] 14-02-PLAN.md — SYSCALL/SYSRET entry contract: star/utextsel/fmask revival per the 9k donor, the syscallentry/syscallreturn/touser software-built-frame asm, init-time MSR arming — gates green with arming live (XEXEC-02 entry half, D-02a)

**Wave 3** *(blocked on Wave 2)*
- [x] 14-03-PLAN.md — exec loader + usermem port: validated user access on the CR3/live-memory model, the amd64 a.out loader (S_MAGIC 0x8A97, 2 MiB alignment) with the empty/adjacent edge tests — host-verified (XEXEC-02)

**Wave 4** *(blocked on Wave 3)*
- [x] 14-04-PLAN.md — proc + syscall dispatch port: per-process PML4/TSS-rsp0 Proc machinery, the measured-set dispatch (BP/SP+8/AX, nsec/seek/exits quirks, loud-unimplemented), the tick-print cfg gate (XEXEC-02)

**Wave 5** *(blocked on Wave 4 — the gate consumes the compile-verified substrate)*
- [x] 14-05-PLAN.md — marker exec end-to-end: mb2 TAG_MODULE delivery + templated module2 staging, Harvey's build/x86_64 marker+bake pipeline (paq format verbatim), IRQ-masked exec_bootstrap — the q35exec gate green twice (XEXEC-02 proof, D-03/D-04/D-08/D-09)

**Wave 6** *(blocked on Wave 5)*
- [x] 14-06-PLAN.md — boot chain boot(8)→init→rc to the `% ` prompt: mkfile.amd64 boot(8) build, the chain bake (rcmain/termrc/identity guards), config_q35shell + BOOT_ENTRY — the q35shell gate green twice (XEXEC-03 r9-side leg)

**Wave 7** *(blocked on Wave 6)*
- [x] 14-07-PLAN.md — compose: r9's parameterized ISO builder (kernel-only FOD preserved), Harvey's boot-r9-x64 app chain, the smoke-r9.expect re-key — the composed smoke green twice consecutively + push-before-pin (XEXEC-03, D-05/D-06)

**Wave 8** *(blocked on Wave 7)*
- [x] 14-08-PLAN.md — tiers re-keyed to objtype=amd64: allowlists + mk filter arms, subset-build with artifact markers, the k10t1 donor venue r9 leg — identical 6.outs on BOTH kernels, the r9 tier gate + ledger (XEXEC-04)

**Wave 9** *(blocked on Wave 8)*
- [x] 14-09-PLAN.md — /net re-gates in-guest: netaddr seam + netip fixtures + net-binary bake, the composed net smoke under the NOT-L2-TRUTH slirp label green + re-green, 14-EVIDENCE.md consolidation (XEXEC-05, D-07)

### Phase 15: Userspace Tiers + Bootfs Payload (amd64)
**Goal**: The services/GUI/long-tail tiers ride the re-keyed machinery — session daemons, the IP suite, and the auth suite compile and donor-execute on k10t1 then r9-x86_64; the GUI stack compiles with measure-first donor syscall traces captured; the desktop bootfs payload is staged so Phases 18-19 never block on payload. **Work in the harvey repo.**
**Depends on**: Phase 14 (XEXEC-04's re-keyed tier machinery + the r9-x86_64 venue); runs parallel with Phase 16
**Requirements**: USPACE-07, USPACE-08, USPACE-09, DESK-03 (bootfs payload staged here per the v2.0 precedent — Phase 19 consumes it)
**Success Criteria** (what must be TRUE):
  1. The services tier executes: aux/listen, mntgen, ndb/cs+dns, exportfs, the ip/ suite, and the auth/ suite (~80 files) run on the k10t1 (9k amd64) venue first, then on r9-x86_64, with committed execution markers (USPACE-07)
  2. The GUI stack compiles for amd64 (libdraw family + rio + acme + sam + plumb) and donor-kernel syscall traces for rio/acme/sam are captured and committed — the measure-first input for any r9 kernel growth the desktop needs (USPACE-08)
  3. The long tail compiles for amd64: upas + lp + the remaining cmd dirs; fossil + venti are compile-gated and documented as runtime-unverifiable this milestone (no block device in r9); gs is included (USPACE-09)
  4. The bootfs payload is staged with manifest assertions: `/lib/font/bit/*`, a glenda-style lib/profile, and a `/lib/ndb/local` template are present in the baked image (DESK-03)
**Plans**: 9/9 plans complete
Plans:
**Wave 1**
- [x] 15-01-PLAN.md — tier-2 services data + filters + driver (the anti-staleness decision lands) + aux leaf filter — T2-ART compile green (USPACE-07)

**Wave 2** *(blocked on Wave 1)*
- [x] 15-02-PLAN.md — services functional probes on the k10t1 donor: services-sweep.rc (D-02/D-08), the grown embed, computed SVC expectations (USPACE-07)
- [x] 15-04-PLAN.md — GUI tier compile (tier3 data + libdraw-family tier0 closure) + $T3 filter wiring — T3-ART compile green (USPACE-08)

**Wave 3** *(blocked on Wave 2)*
- [x] 15-03-PLAN.md — the services image (mkimage fifth contract) + the r9-x64 gate green twice + D-01 fix-everything + the SKIP flips + tier2.ledger (USPACE-07)
- [x] 15-05-PLAN.md — the draw donor venue: k10draw config + vgadispi (DISPI) + the -initrd font channel + the eve-gated kbin file (USPACE-08, D-04)
- [x] 15-09-PLAN.md — the libmemdraw LP64 pixel-path audit (carved out of 15-04): drawtest/arctest root-cause class fixes + stale test targets cleaned — BEFORE the trace capture (USPACE-08)

**Wave 4** *(blocked on Wave 3)*
- [x] 15-06-PLAN.md — the trace rig: tracesup + committed replays for rio/acme/sam, raw logs + distilled syscall tables, the drawtest/arctest re-join (USPACE-08, D-05/D-06/D-07)
- [x] 15-07-PLAN.md — long tail: the full DIRS-coverage proof re-keyed + fix classes + fossil/venti compile-gated + gs compile + the render smoke (USPACE-09, D-09/D-10/D-11)

**Wave 5** *(blocked on Wave 4)*
- [x] 15-08-PLAN.md — DESK-03 payload in the root-tree skeleton (manifest asserts, ndb template, D-17 profile, EARLY headroom check) + 15-EVIDENCE.md (DESK-03 + consolidation)

**UI hint**: yes

### Phase 16: Host Net Modes + Networking Exit Gate
**Goal**: Host networking is an explicit, documented choice — a hermetic slirp+hostfwd default that runs unprivileged and in CI, real-L2 tap/bridge on the linux primary host (opt-in socket_vmnet on darwin), a doctor-checker that prints exact remediation — and the milestone's networking exit gate (sustained bidirectional TCP on the tap leg) runs there. **Work spans both repos (r9 platform TOML + Harvey flake/docs).**
**Depends on**: Phases 13, 14 (virtio-net on x86_64; amd64 guest binaries executing — the `/net` payloads); runs parallel with Phase 15
**Requirements**: HNET-01, HNET-02, HNET-03, HNET-04, HNET-05, NETIP-08 (the tap-leg truth gate joins this phase per the v2.0 dependency shape)
**Success Criteria** (what must be TRUE):
  1. The default boot is hermetic and CI-able: slirp with hostfwd bound to 127.0.0.1 for BOTH tcp/17010 and tcp/567, zero privilege required, green in hosted CI (HNET-01)
  2. `--net=slirp|tap|vmnet` selects the mode explicitly with no silent auto-downgrade, and the netdev stanza is single-sourced as data in r9's x86_64 platform TOML, consumed by Harvey's flake (HNET-02)
  3. The linux tap/bridge leg (qemu-bridge-helper + br0) is documented per-distro and tested on the primary host — the guest is reachable at L2 from the host, providing the L2-truth venue (HNET-03); the darwin socket_vmnet leg is documented as opt-in documented-manual (HNET-04)
  4. A doctor-style checker diagnoses privileged host setup and prints the exact sudo/bridge/firewall remediation commands (HNET-05)
  5. The networking exit gate is green on x86_64: sustained bidirectional TCP flow + load soak on the tap leg — slirp evidence explicitly not accepted (it forgives L2 bugs) (NETIP-08)
**Plans**: 8/8 plans complete
Plans:
**Wave 1**
- [x] 16-01-PLAN.md — r9: [qemu.netdev] mode tables (slirp/tap/vmnet + default, D-04 hostfwd data) + the ONE netdev_args builder + the exported r9-netdev-args emitter package
- [x] 16-02-PLAN.md — Harvey: netaddr tap_ seam keys + the netip-tap.rc L2-truth fixture (ARP assert, burst leg) + the bake seam-assert extension, teeth-proven
- [x] 16-03-PLAN.md — Harvey: the net-doctor app (read-only per-mode checks, exact NixOS-first remediation, WARN class for root-gated reads)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 16-04-PLAN.md — Harvey: r9 pin advance (own commit) + the --net/R9_NET surface on boot-r9-x64 + emitter substitution + doctor-inline tap failure + the hermetic slirp boot proof

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 16-05-PLAN.md — NETIP-08 machinery: the tap soak gate app (doctor preflight, slirp refusal, host echo peer, baseline+8-way burst+ARP counting post-passes)

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 16-06-PLAN.md — HNET-01 CI leg: the net gate rides the emitter argv + hostfwd listener asserts + the r9-shell.yml net-smoke-x64 job
- [x] 16-08-PLAN.md — NETIP-08 runs: the tap soak green twice + the in-phase kernel-fix loop (push-before-pin, FOD re-wire) + evidence/r9-SHA consolidation

**Wave 5** *(blocked on Wave 4 completion)*
- [x] 16-07-PLAN.md — per-distro docs (NixOS tested, generic-Linux + darwin manual) + 16-EVIDENCE.md consolidation

### Phase 17: r9 Kernel Flush-Out
**Goal**: The r9 kernel is functionally hardened under real userspace load — the open Phase-7/8 review backlog (the CR classes, the RFMEM OOM leaks, the WR hardening gaps) plus the known kernel windows are closed with tests or explicitly dispositioned with rationale, every fix landing with a pin (host test or boot-test gate) per the established discipline. **Kernel work in sibling repo `../r9`.**
**Depends on**: Phases 14, 16 (real exec load on r9-x86_64 + the tap-soak load venue for the net findings); runs parallel with Phase 18
**Requirements**: FLUSH-01, FLUSH-02, FLUSH-03, FLUSH-04
**Success Criteria** (what must be TRUE):
  1. The Phase-8 review CR classes are closed with tests or explicitly dispositioned with rationale: conv_close FIN-drop, permanent-send-error wedge, semacquire wakeup loss, ipifc multicast panic, unbind stale-handle aliasing, tcp_gc aliasing, multi-waiter RX-wake — each fix pinned by a host test or boot-test gate (FLUSH-01)
  2. The RFMEM OOM-path page leaks (the 6 WR findings) are closed with refcount proofs — leak-repro gates demonstrate bounded memory under fork/exec churn (FLUSH-02)
  3. The Phase-7 WR hardening gaps are closed: cap-walk loud-abort holes, WaitSet-16-vs-MAX_PROCS-64 sizing, used-id slot-liveness (FLUSH-03)
  4. The known kernel windows are closed or gated — fork+exec arena drain (WINDOWS #44), namec union-create miss (WINDOWS #45) — every flush-out fix lands with a pin (FLUSH-04)
**Plans**: 8/8 plans complete (7/7 original executed + 1 gap-closure pending)
Plans:
**Wave 1**
- [x] 17-01-PLAN.md — the verification wave: the closure pipeline tracer (CR-01 exemplar), the D-02 OOM-audit completion (exec.rs remainder), CR-03's deterministic sem pins, the 12 already-fixed 08-ledger re-cites, 17-FLUSHOUT.md seeded (FLUSH-01, FLUSH-02; D-01/D-02/D-13/D-16)

**Wave 2** *(blocked on Wave 1 — the gates prove the completed audit, D-02)*
- [x] 17-02-PLAN.md — the D-06 tiered leak gates: the r9-side q35churn boot profile (heap_stats slope + CR-03's contention leg) + the Harvey composed /bin/cat churn leg, both green twice, teeth-proven — #44 closed or fixed in-phase (FLUSH-02, FLUSH-04)

**Wave 3** *(blocked on Wave 2 — one pin-advancing plan per wave, flake.lock serializes kernel-byte passes)*
- [x] 17-03-PLAN.md — WR-05 WaitSet resize to the real 512 table + the loud full arm, WR-06 used-ring unposted-slot liveness — pure-struct pins, two class commits, own-commit pin advance (FLUSH-03)

**Wave 4** *(blocked on Wave 3)*
- [x] 17-04-PLAN.md — the cap-walk loud-abort quartet (WR-01..04: shared Caps::parse + the q35 extent/minimum/BAR holes) as ONE class commit + the full x64 battery re-run, WR-07/08 3-part dispositions — the 07 ledger's WR sweep completes (FLUSH-03)

**Wave 5** *(blocked on Wave 4)*
- [x] 17-05-PLAN.md — namec pass: #45 union-create fix (walk_one's union consult + pin) + #18 verify-and-close (the four pg.lock sites + contention evidence) (FLUSH-04, D-10)

**Wave 6** *(blocked on Wave 5)*
- [x] 17-06-PLAN.md — the three rc rows (#46/#49/#65) as one Harvey fix-class commit, sweep-proven — no pin mechanics (FLUSH-04, D-11)

**Wave 7** *(blocked on Wave 6 — the stress pin rides the final kernel + userspace state)*
- [x] 17-07-PLAN.md — CR-07 root-caused (memory-side probes, #66 toolkit) + fixed + deterministic/stress pins (N=10), #47/#50/#66 fold-closed, 17-FLUSHOUT.md completed row-by-row (FLUSH-01 CR-07, FLUSH-04; D-03/D-09/D-16)

**Wave 8** *(gap closure from 17-VERIFICATION.md — blocked on Wave 7; has a blocking-human checkpoint)*
- [x] 17-08-PLAN.md — the CR-07/#66 terminus: the last named host-side capture avenue (TCG x slirp series on the committed stress instrument), the maintainer decision (ratify the 3-part disposition / order a cure pass / keep open, plus the FLUSH-04 checkbox), the ratified outcome recorded across WINDOWS, the 08 ledger, FLUSHOUT, the VERIFICATION overrides block and REQUIREMENTS, plus the stale WR-03 re-cite — records-only, no kernel bytes (FLUSH-01, FLUSH-04; D-03/D-12/D-13/D-14/D-15/D-16)

### Phase 18: In-Guest p9sk1 Auth Domain
**Goal**: The x86_64 guest is a self-contained p9sk1 auth domain — one bake-time generator produces the machine key, `/adm/keys`, nvram-substitute, and ndb entries as a hash-paired unit so all four p9sk1 configs agree by construction; keyfs + authsrv serve it over loopback `/net`; the in-guest probe proves the handshake before any drawterm is ever attempted. **Work spans both repos (r9 `/dev/random` + devcap; Harvey bake machinery + image).**
**Depends on**: Phases 14, 15 (loopback `/net` on x86_64; auth suite binaries from the services tier) — the first hard cross-track join; runs parallel with Phase 17; this phase's loopback probe green is Phase 19's entry criterion
**Requirements**: AUTH-01, AUTH-02, AUTH-03, AUTH-04, AUTH-05, AUTH-06, AUTH-07, AUTH-08, AUTH-09
**Success Criteria** (what must be TRUE):
  1. The bake-time domain generator exists and is the only way domains are made: machine key + `/adm/keys` + nvram-substitute + ndb entries produced as ONE hash-paired unit; the demo domain is loudly labeled DEMO and pairing verification fails loud (AUTH-01)
  2. r9's auth substrate is live on x86_64 with boot-test evidence: a `/dev/random` CSPRNG for p9sk1 challenges seeded from an x86_64-appropriate entropy source (no DTB — e.g. RDRAND/jitter mix, re-keyed from v2.0's DTB-seeded design) and devcap `#¤` (caphash/capuse) — the `auth_chuid` path the cpu server depends on (AUTH-02, AUTH-03)
  3. keyfs starts non-interactively over the baked `/adm/keys`, authsrv listens on tcp/567, and the guest domain's ndb content (authdom, hostid, sys, ether) is staged in the image (AUTH-04, AUTH-05)
  4. The in-guest loopback p9sk1 probe is green BEFORE any drawterm attempt: auth/debug completes a full handshake over loopback, the non-hostowner speaksfor gate passes, and serial-visible auth logging has been present since day one (AUTH-06, AUTH-07)
  5. secstore is available for persistent key storage beyond the baked keys, and the writable-`/adm` + nvram-substitute scoping is decided and documented (in-phase decision: baked+paired for CI; host-9P `/adm` mount for real use) (AUTH-08, AUTH-09)
**Plans**: 5 executed + 3 gap-closure planned

Plans:
**Wave 1**
- [x] 18-01-PLAN.md — r9 `/dev/random` re-keyed to CPUID-gated RDRAND/RDSEED + TSC/jitter fallback (AUTH-02; armed-source print, q35auth substrate profile, KVM+TCG green twice, push-before-pin posture)
- [x] 18-03-PLAN.md — Harvey bake-time domain generator: committed interface + gitignored local file + hash-paired unit (nvram/keys/ndb-auth/manifest) + teeth-proven verify (AUTH-01)

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 18-02-PLAN.md — r9 devcap `#¤` donor-verbatim + hand-rolled HMAC-SHA1 + per-proc user + eve gate (AUTH-03; the phase's ONE FOD re-wire pass + lock-only Harvey pin advance)

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 18-04-PLAN.md — sixth mkimage contract + auth bring-up (pairing boot check → nvram= → keyfs → listen1/authsrv -d on tcp!*!567 → secstored -v) + AUTH-05 ndb staging + the app-per-gate expect gate (AUTH-01, AUTH-04, AUTH-05, AUTH-07)

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 18-05-PLAN.md — probe legs (auth/debug full p9sk1 + speaksfor C probe + negative control) + secstore round-trip + CI leg + AUTH-09 docs (AUTH-06, AUTH-07, AUTH-08, AUTH-09)

**Gap closure** *(waves 5-7 — from 18-VERIFICATION.md gaps_found: CR-01 eve-gate bypass, CR-02 entropy mis-wiring, AUTH-08 devip two-write; ONE FOD cycle across all gap kernel bytes)*
- [x] 18-06-PLAN.md — CR-01: iseve gate + donor bound on the #c/hostowner write + refusal pins + randread HOSTOWNER-REFUSED leg; CR-02: cpuid(1,0) rebind + scripted-cpuid seam + probe-wiring pins; pushed, no FOD (AUTH-02, AUTH-03)
- [x] 18-07-PLAN.md — devip two-write TCP delivery: mechanism record + RED pin + fix + live working-tree re-proof + q35net regression, pushed, no FOD (AUTH-08 kernel half)
- [x] 18-08-PLAN.md — the ONE FOD pass (two-derivation proof → re-wire → push → lock-only pin advance → net-smoke re-green) + secstore round-trip legs restored + gate flipped to presence demands + green twice + records closed (AUTH-01..09)

### Phase 19: CPU Service + Drawterm + rio Desktop
**Goal**: The milestone's payoff — `nix run .#boot-r9` remains the ONE command on x86_64 and now brings up the full system (net + auth + cpu listener); `nix run .#drawterm` connects from the host, authenticates, and lands the user in an interactive rio desktop session — on the flush-out-hardened kernel. **Work spans both repos plus the flake glue.**
**Depends on**: Phases 15, 16, 17, 18 — strictly last (the dependency graph is a chain: desktop consumes cpu, cpu consumes auth, auth consumes net + userspace)
**Requirements**: CPU-01, CPU-02, CPU-03, CPU-04, DESK-01, DESK-02, DESK-04, DESK-05, DESK-06, DESK-07, INT-05, INT-06, INT-07 (DESK-03 staged in Phase 15)
**Success Criteria** (what must be TRUE):
  1. The cpu boot path is real on x86_64: r9 execs `#!` scripts (tcp17010 is literally `#!/bin/cpu -R`, execl'd by listen), `service=cpu` arrives via the x86_64 boot-protocol equivalent of DTB `/chosen/bootargs` (plan9.ini) so cpurc runs instead of termrc, and aux/listen + tcp17010 are live at boot with exportfs served (r9 implements WSTAT/FWSTAT/REMOVE) (CPU-01, CPU-02, CPU-03)
  2. The full `cpu -R` remoteside flow is verified end-to-end with serial evidence: p9any→p9sk1 → auth_chuid → client exportfs mounted at /mnt/term → stdio from /mnt/term/dev/cons → `rc -li` (CPU-04)
  3. The flake drawterm app exists (`nix run .#drawterm`) with its provenance decision recorded; the wrapper pins `-O`, scrubs `$cpu`/`$auth`/`$PASS`, prints the exact connect line — and the `-G` text-mode interop gate is green in CI via hostfwd (scripted login → shell marker) (DESK-01, DESK-06)
  4. rio starts only in-session: profile `case cpu` binds /mnt/term/dev over /dev and execs rio (a serial marker proves rio did NOT start at boot); plumb runs in the drawterm session; and r9 devssl `#D` (rc4_256/sha1) is available for encrypted sessions via pushssl, with `-e clear` as the documented fallback (DESK-02, DESK-04, DESK-05)
  5. The graphical leg is human-accepted on linux AND darwin (drawterm is host-side; the darwin leg connects to the linux-hosted guest); boot-r9-smoke is extended to the full chain; the r9 flake.lock pin advances via push-before-pin as its OWN commit with a fresh hosted CI run (never bundled); and the full v2.1 chain reproduces from clean checkouts on linux-x86_64 (darwin documented-manual) (DESK-07, INT-05, INT-06, INT-07)
**Plans**: 10/10 plans complete
Plans:
**Wave 1**
- [x] 19-01-PLAN.md — service=cpu bootconf tracer: mb2 cmdline to ROOT env seed + the append seam (grub.cfg/iso-builder/xtask) + §15 seed + q35cpu boot profile green twice (CPU-02)
- [x] 19-06-PLAN.md — drawterm vendor -G/-O + gui-none backend + nix package + scrubbing wrapper + flake app + README recipes (DESK-01)

**Wave 2** *(r9 serialization: after 19-01)*
- [x] 19-02-PLAN.md — CR-03 AcceptPool TCP fix (the two set_* calls + WR-04 pin legs, RED-proven) + proactive tick_check wiring (D-08, D-10)

**Wave 3**
- [x] 19-03-PLAN.md — kernel hash-bang exec (donor argv order) + WSTAT/FWSTAT/REMOVE DISPATCH arms + ring-3 profile legs (CPU-01, CPU-03)

**Wave 4**
- [x] 19-04-PLAN.md — devssl #D: rc4_256/sha1 donor port + the device with pushssl-protocol round-trip pins + registry wiring (DESK-04)

**Wave 5** *(all kernel bytes in)*
- [x] 19-05-PLAN.md — the one FOD re-wire pass (two-derivation proof) + pin advance as its OWN lock-only commit + fresh hosted CI + composed gates re-green (INT-06)

**Wave 6** *(at the advanced pin)*
- [x] 19-07-PLAN.md — §15 seventh contract full: re-keyed cpurc (pairing→keyfs→authsrv→factotum→trusted aux/listen) + tcp17010 + cmd/cpu tier + D-12 creds emission + in-session-only profile + bring-up probe green twice (CPU-02, CPU-03, DESK-02, DESK-05)

**Wave 7**
- [x] 19-08-PLAN.md — the -G interop gate: pty-driven scripted drawterm session over hostfwd, encrypted default, green twice on local KVM (DESK-06, CPU-04, DESK-04 live)

**Wave 8** *(serialized after 19-08: both gates bake out/bootfs-cpu.paq and bind the 127.0.0.1:17010/5670 hostfwd pair)*
- [x] 19-09-PLAN.md — boot-r9-x64 --system=cpu full-system mode + full-chain smoke extension + README (INT-05)

**Wave 9**
- [x] 19-10-PLAN.md — human legs (linux graphical row + darwin -G row, blocking-human checkpoints) + darwin graphical deferral records + clean-checkout proof of v2.1 (DESK-07 as owner-resolved, INT-07)
**UI hint**: yes

## Progress

**Execution Order:**
13 → 14 → {15 ∥ 16} → {17 ∥ 18} → 19. The port is a chain (13 platform → 14 userspace); then tiers (15) and host net (16) run in parallel — both feeding the two hard-join phases (17 flush-out under load ∥ 18 auth domain) — and 19 is strictly last. Decimal insertions slot numerically.

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Build-Tail Green (9k amd64) | v1.0 | 4/4 | Complete | 2026-10-31 |
| 2. aarch64 Toolchain + libc | v1.0 | 7/7 | Complete | 2026-11-01 |
| 3. r9 qemu-virt Platform Bring-up | v1.0 | 6/6 | Complete | 2026-11-01 |
| 4. r9 Userspace-Exec + Harvey↔r9 ABI | v1.0 | 8/8 | Complete | 2026-11-02 |
| 5. Shell Boot + One-Command Integration | v1.0 | 13/13 | Complete | 2026-11-04 |
| 6. Full-Tree Userspace — Wave 0 + Libraries/Core | v2.0 | 10/10 | Complete | 2026-11-05 |
| 7. r9 PCI/ECAM + virtio-net + devether | v2.0 | 10/10 | Complete | 2026-11-05 |
| 8. r9 TCP/IP Path — devip-Compatible /net | v2.0 | 9/9 | Complete | 2026-11-06 |
| 9. Host Networking + boot-r9 Net Modes | v2.0 | - | Subsumed → Phase 16 | - |
| 10. Userspace Services + GUI Tiers + Bootfs | v2.0 | - | Subsumed → Phase 15 | - |
| 11. In-Guest p9sk1 Auth Domain | v2.0 | - | Subsumed → Phase 18 | - |
| 12. CPU Service + Drawterm + rio Desktop | v2.0 | - | Subsumed → Phase 19 | - |
| 13. r9 x86_64 Platform Bring-Up | v2.1 | 6/6 | Complete    | 2026-11-07 |
| 14. Harvey amd64 Userspace on r9 | v2.1 | 9/9 | Complete    | 2026-11-09 |
| 15. Userspace Tiers + Bootfs Payload (amd64) | v2.1 | 9/9 | Complete    | 2026-11-10 |
| 16. Host Net Modes + Networking Exit Gate | v2.1 | 8/8 | Complete    | 2026-11-12 |
| 17. r9 Kernel Flush-Out | v2.1 | 8/8 | Complete    | 2026-11-13 |
| 18. In-Guest p9sk1 Auth Domain | v2.1 | 8/8 | Complete    | 2026-11-14 |
| 19. CPU Service + Drawterm + rio Desktop | v2.1 | 10/10 | Complete    | 2026-11-17 |

## Backlog

### Phase 999.1: Follow-up — harvey.yml nix rewrite first-push verification (BACKLOG)

**Goal:** Confirm the rewritten harvey.yml workflow (nix pipeline, commit 261105-slx) goes green on the hosted runner after the next push — the red streak on master predates this work (since 2026-11-04T12:01Z)
**Source phase:** 06 (deferred at verification 2026-11-05)
**Follow-ups:**
- [ ] Hosted green run of the nix-based harvey.yml after first push

### Phase 20: arm64 catch-up to v2.1 parity

**Goal:** Bring the r9 aarch64 kernel and Harvey arm64 userspace back to parity with the v2.1 x86_64 work so `nix run .#boot-r9-smoke` is green at the pin — the first end-to-end proof of the objtype rename aarch64 -> arm64 (quick 261117-bz5: r9 8e4e730 seeds cputype=arm64, the Harvey image stages arm64/). **Kernel work in sibling repo `../r9`.**
**Known start (measured 2026-11-17 at r9 8e4e730):** `r9-kernel-virt` fails to compile — `aarch64/src/main.rs` omits `note_pending` in the `PipeHooks` (:493) and `DevipHooks` (:523) initializers, a field the shared `port` crate gained in phase 19-08 (r9 56acaea). The ARM leg was frozen at the v2.0 pin through phases 13-19, so more drift is expected behind those two errors.
**Carried in from quick 261117-bz5 (unrun there):** the donor-venue gates (`build/aarch64/subset-exec.sh` merged /arm64 staging; `build/aarch64/trace-boot.rc` `bind /boot/ourinit /arm64/init`) and r9 `cargo test -p port` on nightly-2026-01-30.
**Requirements**: ARM-01, ARM-02, ARM-03, ARM-04, ARM-05, ARM-06, ARM-07, ARM-08, ARM-09, ARM-10, ARM-11, ARM-12
**Depends on:** Phase 19
**Plans:** 17/17 plans complete

Plans (strictly sequential, one wave each — the plans share the one r9 checkout and the HVF timing series must not run under other load):

*Group A — arm64 green on HVF at the pin*
- [x] 20-01-PLAN.md — tracer: note fix (both halves) + per-proc user hooks + rename test literal + gate post-passes and pushword fast-fail (ARM-01, ARM-02, ARM-04)
- [x] 20-02-PLAN.md — repeat harness, pre-fix per-fork rate and N derivation from its lower 95% bound, forkstress reproducer committed and baked into the image, root-cause the HVF fork-child defect; owner checkpoint on the fix route (ARM-03)
- [x] 20-03-PLAN.md — fix the fork-child defect, pin it with forkstress, post-fix N series on HVF + TCG control (ARM-03)
- [x] 20-04-PLAN.md — r9 guards: aarch64 virtshell CI lane, port integration tests under xtask test, darwin devShell, dev_exec boot-test profiles re-greened (ARM-01)
- [x] 20-05-PLAN.md — donor re-provision + donor tier-1 gate; boot-chain trace fversion diagnosis (ARM-04)
- [x] 20-06-PLAN.md — group A close: push r9, lock-only pin, HVF series at the pin, records, push Harvey; owner CI + boot-iso checkpoint (ARM-03, ARM-05)

*Group B — syscall-path parity*
- [x] 20-07-PLAN.md — measure the ramfs writable venue (ramfs + /n/wp baked into the tier-1 image) and the lone-member brk defect; owner checkpoint (ARM-07)
- [x] 20-08-PLAN.md — rfork fd-group split, devmnt write, DMDIR + union creates; writeprobe pin in the tier-1 sweep (ARM-06, ARM-07)
- [x] 20-09-PLAN.md — arm64 churn gate (virtchurn) built and run against the unfixed exit path (ARM-07)
- [x] 20-10-PLAN.md — exit-path drains + husk sweep, close-on-exec sweep, brk per measurement; churn gate green; cexecprobe + brkprobe pins (ARM-06, ARM-07)
- [x] 20-11-PLAN.md — group B close: push r9, lock-only pin, HVF series at the pin, push Harvey; owner checkpoint (ARM-03, ARM-05, ARM-07)

*Group C — cpu-port arms*
- [x] 20-12-PLAN.md — measure aarch64 host-test compilability and an arm64 consumer + baseline for each cpu-port arm (sslprobe); owner checkpoint on the devcap profile route (ARM-11, ARM-12)
- [x] 20-13-PLAN.md — DTB bootconf: launcher seam, delivery measurement, owner checkpoint on the carrier, reader + pair parser + ROOT env seed merge; virtio check_cap_extents (ARM-10, ARM-09)
- [x] 20-14-PLAN.md — exec and remove measurements; owner checkpoint on the remove route; `#!` exec pre-pass (sys_exec factored first); REMOVE/WSTAT/FWSTAT dispatch with stat validation (ARM-08)
- [x] 20-15-PLAN.md — devcap/devssl baseline on a cap kernel; owner checkpoint; devcap + devssl arms and hooks; the devcap-carrying kernel by the chosen route (virtcpu profile + kernel-virt-cpu) (ARM-09)
- [x] 20-16-PLAN.md — cpu boot-path measurement; owner checkpoint; arm64 cpu fixture (cpurc, shebangecho, cpuprobe), the boot-r9-cpu gate (ARM-11)
- [x] 20-17-PLAN.md — the aarch64 host-test battery; group C close: CI lanes, push r9, lock-only pin, HVF series at the pin for four gates, records, push Harvey; owner checkpoint (ARM-03, ARM-05, ARM-11, ARM-12)
