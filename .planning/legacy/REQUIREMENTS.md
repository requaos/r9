# Requirements: Harvey OS — v2.1 x86_64 Pivot + r9 Kernel Flush-Out

**Defined:** 2026-11-06
**Core Value:** Harvey userspace + r9 kernel boot to a shell on x86_64 under qemu/KVM — reproducible from clean checkouts on Linux (primary host since 2026-11-06; darwin stays a documented-manual venue).
**Milestone goal:** Re-target Harvey+r9 from aarch64 to x86_64 under qemu/KVM and use Harvey's full amd64 userspace to functionally harden ("flush out") the r9 kernel — delivering the inherited v2.0 payoff (auth, cpu/drawterm, rio) on x86_64 — as the proving ground en route to r9 becoming a complete Rust port of Harvey.

**Inheritance:** v2.0 (subsumed 2026-11-06) delivered Phases 6–8 (USPACE-01..06, NETDRV-01..04, NETIP-01..07, USPACE-10 — see `.planning/milestones/v2.0-REQUIREMENTS.md`). The 26 pending v2.0 requirements are inherited below, re-keyed to x86_64 where arch-bound.

## v2.1 Requirements

### r9 x86_64 Platform Bring-Up (X64)

- [x] **X64-01**: r9 boots on qemu x86_64 (q35-vs-pc machine decision recorded with rationale) via a recorded DTB-free boot protocol (decided against 9k's existing multiboot path or equivalent) — the Phase-3-style platform gate: banner, timer ticks, serial input echo
- [x] **X64-02**: APIC interrupt plumbing (LAPIC + IOAPIC) + HPET/PIT timers proven under load — the GICv2/GICv3 equivalent; INTx level-triggered virtio ISR read-to-ack re-proven
- [x] **X64-03**: PCI/ECAM discovery without DTB (ACPI MCFG or the fixed q35 window) with a moved-window perturbation proof — NETDRV-01's no-hardcoded-constants discipline carried to x86_64
- [x] **X64-04**: virtio-net binds on x86_64 (arch-neutral `port/` carry-over) and devether presents `/net/ether0` with the guest ARP observable on the host — the NETDRV gates re-proven
- [x] **X64-05**: r9's flake ships an x86_64 kernel package; acceptance gates run KVM-native by default with the TCG fallback documented

### Harvey amd64 Userspace on r9 (XEXEC)

- [x] **XEXEC-01**: amd64 Ureg/TrapFrame ABI contract published and build-enforced — the `aarch64/ABI.md` equivalent: offsets frozen, const asserts + xtask ureg-check with a negative control
- [x] **XEXEC-02**: Harvey amd64 binaries exec on r9 x86_64 through the real EL/syscall path (measured-set dispatch carries; staged-init + hello-equivalent proof)
- [x] **XEXEC-03**: full boot chain boot(8)→init→rc reaches the interactive `% ` prompt on x86_64 with the expect smoke gate green twice consecutively (boot-r9-smoke equivalent)
- [x] **XEXEC-04**: tier machinery re-keyed to `objtype=amd64`: tier-0 libraries + tier-1 core commands proven executing on r9-x86_64 (donor venue per Phase-6 D: Harvey's own 9k kernel, k10t1)
- [x] **XEXEC-05**: `/net` stack re-gated on x86_64: loopback ping/udp/tcp echo, static + DHCP ipconfig, and the composed net smoke all green on the new arch

### r9 Kernel Flush-Out (FLUSH)

- [x] **FLUSH-01**: Phase-8 review CR classes closed with tests (or explicitly dispositioned with rationale): conv_close FIN-drop, permanent-send-error wedge, semacquire wakeup loss, ipifc multicast panic, unbind stale-handle aliasing, tcp_gc aliasing, multi-waiter RX-wake
- [x] **FLUSH-02**: RFMEM OOM-path page leaks (the 6 WR findings) closed with refcount proofs
- [x] **FLUSH-03**: Phase-7 WR hardening gaps closed: cap-walk loud-abort holes, WaitSet-16-vs-MAX_PROCS-64 sizing, used-id slot-liveness
- [x] **FLUSH-04**: known kernel windows closed or gated: fork+exec arena drain (WINDOWS #44), namec union-create miss (WINDOWS #45) — every flush-out fix lands with a pin (host test or boot-test gate) per the established discipline

### Userspace Tiers on amd64 (USPACE — continuing v2.0 numbering, re-keyed)

- [x] **USPACE-07**: Session daemons + services compile and execute: aux/listen, mntgen, ndb/cs+dns, exportfs, ip/ suite, auth/ suite (~80 files) — on the k10t1 (9k amd64) venue, then r9-x86_64
- [x] **USPACE-08**: GUI stack compiles for amd64 (libdraw family + rio + acme + sam + plumb) and donor-kernel syscall traces for rio/acme/sam are captured (measure-first input for r9 kernel growth)
- [x] **USPACE-09**: Long tail compiles for amd64: upas + lp + remaining cmd dirs; fossil + venti compile-gated (runtime-unverifiable — no block device in r9); gs included

### r9 TCP/IP Path — exit gate (NETIP — continuing)

- [x] **NETIP-08**: Networking exit gate: sustained bidirectional TCP flow + load soak on the **tap leg** on x86_64 (slirp forgives L2 bugs — ping on slirp is not a gate)

### Host Networking (HNET — continuing)

- [x] **HNET-01**: slirp+hostfwd hermetic default: hostfwd bound to 127.0.0.1 for BOTH tcp/17010 and tcp/567; zero privilege; CI-able
- [x] **HNET-02**: `--net=slirp|tap|vmnet` explicit mode selection on the boot app (no silent auto-downgrade); netdev stanza single-sourced as data in r9's x86_64 platform TOML, consumed by Harvey's flake
- [x] **HNET-03**: linux tap/bridge leg (qemu-bridge-helper + br0) documented per-distro and tested on the primary host — the L2-truth leg
- [x] **HNET-04**: darwin socket_vmnet leg documented as opt-in documented-manual (builtin vmnet is entitlement-walled on nixpkgs qemu)
- [x] **HNET-05**: doctor-style checker diagnosing privileged host setup, printing exact sudo/bridge/firewall remediation

### In-Guest Auth Domain (AUTH — continuing)

- [x] **AUTH-01**: Bake-time domain generator: machine key + `/adm/keys` + nvram-substitute + ndb entries produced as ONE hash-paired unit; demo domain loudly labeled DEMO; pairing fails loud
- [x] **AUTH-02**: r9 `/dev/random` CSPRNG for p9sk1 challenges, seeded from an x86_64-appropriate entropy source (no DTB — e.g. RDRAND/jitter mix), re-keyed from v2.0's DTB-seeded design <!-- closed 18-06/18-08: CR-02 (the armed_source RDRAND-probe rebind to cpuid(1,0) over the scripted-cpuid seam, r9 f594b4c) + the 18-08 FOD pass pinning the bytes (r9 9103522, flake.lock advanced lock-only) -->
- [x] **AUTH-03**: r9 devcap `#¤` (caphash/capuse) — the `auth_chuid` path the cpu server depends on <!-- closed 18-06/18-08: CR-01 (the #c/hostowner write iseve gate with the donor's 128-byte bound, refused live from ring 3, r9 652f025) + the 18-08 FOD pass pinning the bytes (r9 9103522) -->
- [x] **AUTH-04**: keyfs starts non-interactively over the baked `/adm/keys`; authsrv listens on tcp/567
- [x] **AUTH-05**: ndb content for the guest domain (authdom, hostid, sys, ether) staged in the image
- [x] **AUTH-06**: In-guest loopback p9sk1 probe (auth/debug) green BEFORE any drawterm attempt; non-hostowner speaksfor gate
- [x] **AUTH-07**: Serial-visible auth logging from day one
- [x] **AUTH-08**: secstore available for persistent key storage beyond baked keys <!-- 18-05: PARTIAL-BLOCKED — the substrate green (the ramfs venue + secuser account, live-proven per gate run); the client PUT/GET round-trip is venue-blocked by r9's devip two-write TCP defect (kernel pinned b2a6151; STATE blocker + 18-05-SUMMARY §THE BLOCKER; reopener: the next r9 kernel-byte pass) --><!-- closed 18-06/18-07/18-08: the devip two-write fix (r9 ab4cb66, donor-faithful tcp sockets — Nagle off, ACK immediate) delivered SConn's pattern; 18-08 restored the sweep's 9d legs + flipped the gate to presence demands — the round-trip green twice at the advanced pin (r9 9103522), evidence evidence/18-08-auth-{a,b}.log; the AUTH-SKIP arm retired -->
- [x] **AUTH-09**: writable-`/adm` + nvram-substitute scoping decided and documented (baked+paired for CI; host-9P `/adm` mount for real use)

### CPU Service (CPU — continuing)

- [x] **CPU-01**: r9 `#!` exec (tcp17010 is literally `#!/bin/cpu -R`, execl'd by listen)
- [x] **CPU-02**: `service=cpu` bootconf path: plan9.ini delivered via the x86_64 boot-protocol equivalent of DTB `/chosen/bootargs` → cpurc runs instead of termrc (re-keyed)
- [x] **CPU-03**: aux/listen + tcp17010 live at boot; exportfs served; r9 implements WSTAT/FWSTAT/REMOVE
- [x] **CPU-04**: Full `cpu -R` remoteside flow verified end-to-end: p9any→p9sk1 → auth_chuid → client exportfs at /mnt/term → stdio from /mnt/term/dev/cons → rc -li

### Drawterm + rio Desktop (DESK — continuing)

- [x] **DESK-01**: Flake drawterm app: provenance decision recorded; wrapper pins `-O`, scrubs `$cpu`/`$auth`/`$PASS`, prints the exact connect line
- [x] **DESK-02**: profile `case cpu` binds /mnt/term/dev over /dev and execs rio (serial marker proves rio did NOT start at boot)
- [x] **DESK-03**: `/lib/font/bit/*` + glenda-style lib/profile + `/lib/ndb/local` template staged in bootfs with manifest assertions
- [x] **DESK-04**: r9 devssl `#D` (rc4_256/sha1) for encrypted sessions via pushssl; `-e clear` documented fallback (stronger crypto is PROTOCOL-blocked — the hardening milestone)
- [x] **DESK-05**: plumb/plumber runs in the drawterm session
- [x] **DESK-06**: `-G` text-mode drawterm interop gate in CI via hostfwd (scripted login → shell marker)
- [x] **DESK-07**: Human-accepted graphical rio leg on linux AND darwin (drawterm is host-side; the darwin leg connects to the linux-hosted guest) <!-- 19-10 scope per the amended D-03 (owner decision 2026-11-14): the linux GRAPHICAL row + the darwin -G TEXT row are this phase's legs (signed rows + screenshots in 19-EVIDENCE.md, D-13); the darwin GRAPHICAL row is deferred 2026-11-14 by owner decision — reopener: a future milestone porting a 64-bit Cocoa/AppKit backend (9front gui-cocoa class) into sys/src/cmd/unix/drawterm; XQuartz considered and rejected. Not silently marked complete: the checkbox flips only when both human rows are signed (pending as of 2026-11-15) --><!-- closed 19-10, accepted WITH RECORDED DEFERRAL per the owner resolutions — the gate rule above was superseded by the owner's 2026-11-17 decision on the second row: the linux GRAPHICAL row is SIGNED (human-accepted 2026-11-16, Neil Skinner, r9 0921a1bc, screenshot evidence/19-10-linux-rio.png — Row 1 in 19-EVIDENCE.md, commit d4f0eeae5); the darwin -G row is DEFERRED BY OWNER DECISION 2026-11-17 — the owner declined the recorded SSH-tunnel venue and a mac-local x86_64 TCG substitute; verbatim intent: "aarch64 cpu port, but just note it so that I can move back to the mac and perform the work natively on the aarch64 machine"; that port is phase-scale (DTB bootconf seam, auth-suite tiers, §15 mkimage contract, devssl #D registration, drawterm gate — all x86_64-only today) and explicitly the owner's own future work; reopener: the aarch64 cpu port — the DESK-07 darwin -G row reopens against it; the darwin GRAPHICAL row stays deferred 2026-11-14 (gui-cocoa reopener, XQuartz rejected). Standing evidence: the signed linux Row 1 + INT-07's automated clean-checkout proof (GREEN both gates); both darwin legs carry named reopeners in STATE.md's Deferred Items — recorded deferrals, never silent completions -->

### Integration (INT — continuing)

- [x] **INT-05**: `nix run .#boot-r9` remains the ONE command on x86_64 — brings up the full system (net + auth + cpu listener); boot-r9-smoke extended to the new chain
- [x] **INT-06**: r9 flake.lock pin advances via push-before-pin as its OWN commit + fresh hosted CI run (never bundled)
- [x] **INT-07**: Clean-checkout proof of the full v2.1 chain on linux-x86_64 (darwin documented-manual) <!-- 19-10 GREEN 2026-11-15: two independent fresh clones of harvey at e4001b383 (r9 solely via the flake.lock github pin df4ad78 — no sibling clone): nix run .#boot-r9-smoke-x64 exit 0 ALL STAGES PASS (chain + cpu legs) and nix run .#drawterm-gate-x64 exit 0 ALL STAGES PASS (the -G session end-to-end, capture redacted); record in 19-EVIDENCE.md (clone SHA, locked rev, verdicts, retained consoles) -->

### arm64 Catch-Up to v2.1 Parity (ARM — Phase 20, added 2026-11-17)

Reactivates the arm64 leg (objtype `arm64`; r9 crate `aarch64/`). Supersedes the "aarch64 re-activation" Future item and the "aarch64 as an active development target" Out-of-Scope row for this phase only. Scope per 20-CONTEXT.md D-01..D-09.

- [x] **ARM-01**: r9 `cargo xtask build --arch aarch64 --config virtshell` builds at HEAD; an r9 CI lane compiles the aarch64 `dev_exec` code; `cargo xtask test` runs the `port` integration tests; the r9 devShell evaluates on darwin
- [x] **ARM-02**: Note-interruptible blocking IO (the `note_pending` probe and the `push_note` ready-a-Blocked-target half) and per-proc `#c/user` / `#c/hostowner` behave on arm64 as on x86_64; no `hooks not registered` line on any arm64 gate wire
- [x] **ARM-03**: `boot-r9-smoke`, `boot-r9-net-smoke` and `boot-r9-tier1` are reliably green under HVF on darwin-arm64 at the pin — the `pushword but no argv` fork-child defect root-caused and fixed, each gate green N consecutive runs with N derived from the measured per-fork rate, TCG recorded as the control leg
- [x] **ARM-04**: The objtype rename is proven on every venue: `cargo test -p port`, the donor `subset-exec.sh` gate after re-provision, and the boot-chain trace gate with its `fversion` failure diagnosed
- [x] **ARM-05**: Each group of r9 changes reaches Harvey push-before-pin as its own lock-only `flake.lock` commit, with the gates re-greened at the pin without `--override-input`
- [x] **ARM-06**: The aarch64 halves of the v2.1 syscall-path fixes are ported: non-RFPROC rfork fd-group split, devmnt write, DMDIR and union creates, exit-path drains and husk sweep, fd-drain cascade, close-on-exec sweep, and the lone-member brk arm where the defect is measured to exist
- [x] **ARM-07**: Every ARM-06 port has a live arm64 pin: a writable venue in the tier-1 sweep, and an arm64 churn gate that pins the exit-leak fix
- [x] **ARM-08**: The `#!` exec pre-pass and the REMOVE / WSTAT / FWSTAT dispatch arms with stat validation are ported to aarch64
- [x] **ARM-09**: devcap and devssl are served on arm64 (device arms, tick hook, fd-resolve hook, `mount` over a `#D` chan), and the virtio capability-extent hardening (`check_cap_extents`) is ported
- [x] **ARM-10**: Boot configuration reaches the ROOT env seed on arm64 through DTB `/chosen/bootargs` (merge, never replace; absent string leaves the boot byte-identical)
- [x] **ARM-11**: An arm64 fixture gate exercises the cpu-port arms live (bootconf, `#!`, REMOVE over devcap, devssl) and is green on HVF at the pin
- [x] **ARM-12**: An aarch64 host-test battery pins the ported pure logic (shebang pass, stat validation, bootconf parser, cap extents), by the route the host-compilability measurement selects

## Future Requirements

Deferred to future milestones. Tracked but not in the v2.1 roadmap.

- Rust port of Harvey userspace into r9 — the north star's final track; starts only after r9 is proven/flushed under the C userspace
- aarch64 re-activation (frozen at the v2.0 pin; TCG-bootable history preserved)
- virtio-9p host mount in r9 (collapses userland iteration time)
- SMP bring-up (x86_64: INIT-SIPI-SIPI; aarch64: PSCI); RTC; virtio-blk
- virtio-gpu / framebuffer console in r9 (native graphics without drawterm)
- fossil/venti RUNTIME (needs a block device — compile-only in v2.1)
- dp9ik + rcpu + TLS 1.2/1.3 server-side (the TLS/security hardening milestone)
- Hardware targets beyond qemu

## Out of Scope

Explicitly excluded from v2.1. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| aarch64 as an active development target | Frozen at the v2.0 pin (2026-11-06 pivot): TCG-bootable history preserved, v2.1 phases key to x86_64 only |
| Rust port of Harvey userspace | The north star's final track, deliberately a FUTURE milestone — v2.1 exists to flush out r9 under the C userspace first |
| Harvey 9k (C kernel) aarch64 port | Superseded by r9 adoption (2026-10-30); 9k stays the amd64 standalone kernel + donor venue |
| virtio MSI-X | INTx remains the one interrupt truth this milestone; MSI-X is future hardening |
| e1000 / other NIC models | virtio-net only; one driver, one truth |
| virtio-gpu / framebuffer in r9 | The draw device arrives over 9P from drawterm — the entire point of the drawterm architecture |
| dp9ik / rcpu / TLS server-side | Different auth+session protocol stack (9front); Harvey is 4e p9sk1 — hardening milestone scope |
| slirp as a correctness target | Hermetic default + CI leg only; it forgives L2 bugs — tap is the truth leg (NETIP-08) |
| DHCPv6 / SLAAC tooling | IPv6 scoped to devip ip6 + static/ipconfig configuration |
| APE | Nothing in the v2.1 session path needs it (amd64 APE exists in-tree if ever wanted) |

## Traceability

Each requirement maps to exactly one phase. Populated at roadmap creation (v2.1 phases start at 13 — v1.0 = 1–5, v2.0 = 6–12, archived).

| Requirement | Phase | Status |
|-------------|-------|--------|
| X64-01 | Phase 13 | Complete |
| X64-02 | Phase 13 | Complete |
| X64-03 | Phase 13 | Complete |
| X64-04 | Phase 13 | Complete |
| X64-05 | Phase 13 | Complete |
| XEXEC-01 | Phase 14 | Complete |
| XEXEC-02 | Phase 14 | Complete |
| XEXEC-03 | Phase 14 | Complete |
| XEXEC-04 | Phase 14 | Complete |
| XEXEC-05 | Phase 14 | Complete |
| USPACE-07 | Phase 15 | Complete |
| USPACE-08 | Phase 15 | Complete |
| USPACE-09 | Phase 15 | Complete |
| DESK-03 | Phase 15 | Complete |
| HNET-01 | Phase 16 | Complete |
| HNET-02 | Phase 16 | Complete |
| HNET-03 | Phase 16 | Complete |
| HNET-04 | Phase 16 | Complete |
| HNET-05 | Phase 16 | Complete |
| NETIP-08 | Phase 16 | Complete |
| FLUSH-01 | Phase 17 | Complete |
| FLUSH-02 | Phase 17 | Complete |
| FLUSH-03 | Phase 17 | Complete |
| FLUSH-04 | Phase 17 | Complete |
| AUTH-01 | Phase 18 | Complete |
| AUTH-02 | Phase 18 | Complete |
| AUTH-03 | Phase 18 | Complete |
| AUTH-04 | Phase 18 | Complete |
| AUTH-05 | Phase 18 | Complete |
| AUTH-06 | Phase 18 | Complete |
| AUTH-07 | Phase 18 | Complete |
| AUTH-08 | Phase 18 | Complete |
| AUTH-09 | Phase 18 | Complete |
| CPU-01 | Phase 19 | Complete |
| CPU-02 | Phase 19 | Complete |
| CPU-03 | Phase 19 | Complete |
| CPU-04 | Phase 19 | Complete |
| DESK-01 | Phase 19 | Complete |
| DESK-02 | Phase 19 | Complete |
| DESK-04 | Phase 19 | Complete |
| DESK-05 | Phase 19 | Complete |
| DESK-06 | Phase 19 | Complete |
| DESK-07 | Phase 19 | Complete |
| INT-05 | Phase 19 | Complete |
| INT-06 | Phase 19 | Complete |
| INT-07 | Phase 19 | Complete |
| ARM-01 | Phase 20 | Complete |
| ARM-02 | Phase 20 | Complete |
| ARM-03 | Phase 20 | Complete |
| ARM-04 | Phase 20 | Complete |
| ARM-05 | Phase 20 | Complete |
| ARM-06 | Phase 20 | Complete |
| ARM-07 | Phase 20 | Complete |
| ARM-08 | Phase 20 | Complete |
| ARM-09 | Phase 20 | Complete |
| ARM-10 | Phase 20 | Complete |
| ARM-11 | Phase 20 | Complete |
| ARM-12 | Phase 20 | Complete |

**Coverage:**
- v2.1 requirements: 58 total (46 from the v2.1 roadmap + 12 ARM added with Phase 20)
- Mapped to phases: 58/58
- Unmapped: 0

---
*Requirements defined: 2026-11-06*
*Last updated: 2026-11-06 — v2.1 roadmap created (Phases 13-19; traceability populated)*
