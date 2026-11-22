# Requirements: Harvey OS — v2.0 Full System & Drawterm

**Defined:** 2026-11-04
**Core Value:** Harvey userspace + r9 kernel boot to a shell on aarch64 under qemu virt — reproducible from clean checkouts on macOS or Linux.
**Milestone goal:** One command — `nix run .#boot-r9` — brings up a complete Harvey system (full aarch64 userspace, in-guest p9sk1 auth, tap/bridge networking) that a host drawterm connects to for an interactive rio desktop session.

**Research basis:** `.planning/research/SUMMARY.md` (HIGH confidence; in-tree verified). Decisive finding: Plan 9 networking is a file-interface contract — libc `dial()`/`announce()`/`listen()` are pure I/O on `/net` (zero net syscalls in `sys.h`), so any TCP provider must present devip's exact file tree.

## v2.0 Requirements

### Full aarch64 Userspace (USPACE)

- [x] **USPACE-01**: Wave-0 landmine fixes land before any tier: `cc/macbody` + `6l/obj.c` gethunk reset-not-accumulate bugs fixed (they fire as silent build HANGS at full-tree scale), with regression proof
- [x] **USPACE-02**: Structural mk blockers for a full-tree aarch64 sweep fixed (jpg/mkfile-class `oc` derivation, mahjongg cpp, `4a` lex.c) so the DIRS recursion reaches every cmd dir
- [x] **USPACE-03**: Cross-compiled-generator audit: every build-then-run generator in swept mkfiles builds with the host toolchain (7c output never runs on the host); CI reds on `.7`-magic generator artifacts
- [x] **USPACE-04**: Tiered allowlist machinery: per-dir append-only allowlist growth (never a flag-flip); per-tier EXECUTION gates on the 9front donor kernel FIRST, then r9 via measure-first traces; artifact+execution markers are the only gates (namespace-build mk exit status is always 0)
- [x] **USPACE-05**: All remaining libraries compile for aarch64 (22 of 35 outstanding: libdraw, libmemdraw, libmemlayer, libframe, libcontrol, libhtml, libhttpd, libdisk, libflate, libgeometry, liblex, libplumb, libventi/liboventi, …) via the `mksyslib` portable-C fallback (stale committed `.6`/`.8` objects NOT cargo-culted)
- [x] **USPACE-06**: Core commands + self-hosted build chain compile and donor-execute: shell/coreutils + 7a/7c/7l/cc/cpp/mk/yacc/lex as aarch64 binaries
- [ ] **USPACE-07**: Session daemons + services compile and donor-execute: aux/listen, mntgen, ndb/cs+dns, exportfs, ip/ suite (ipconfig, ping), auth/ suite (~80 files)
- [ ] **USPACE-08**: GUI stack compiles: libdraw family + rio + acme + sam + plumb; donor-kernel syscall traces for rio/acme/sam captured (measure-first — libthread programs exercise unmeasured kernel corners)
- [ ] **USPACE-09**: Long tail compiles: upas + lp + remaining cmd dirs; fossil + venti compile-gated and documented as unverifiable this milestone (no block device in r9); gs included (753 files)
- [x] **USPACE-10**: v1.0 carry-over: 9k-amd64 embedded-root `bad magic` pool panic re-tested against the pswt.c-fixed 6c (v1.0 deferred-items #8)

### r9 Network Driver (NETDRV)

- [x] **NETDRV-01**: r9 PCI/ECAM platform glue: DTB-driven ECAM scan + `interrupt-map`→GICv2 SPI routing — no hardcoded IRQs/addresses (qemu moved the ECAM window once already)
- [x] **NETDRV-02**: virtio-net driver with split virtqueues, zero-offload feature mask (MAC+STATUS only) from Day 1, barrier-first aarch64 ring code; provenance decision (virtio-drivers crate vs from-scratch `port/src/virtio.rs`) recorded
- [x] **NETDRV-03**: devether `#l` presenting `/net/ether0` per netif.h; RX/TX packet counters; guest ARP request observable on the host
- [x] **NETDRV-04**: Interrupt accounting + INTx level-triggered plumbing (ISR read-to-ack) proven under load — no silent virtqueue wedges

### r9 TCP/IP Path (NETIP)

- [x] **NETIP-01**: TCP implementation decision FINAL and recorded before driver-dependent plans (research recommends smoltcp 0.14 behind the devip ABI vs a staged Rust port of `sys/src/9k/ip`; same-axis virtio provenance decision)
- [x] **NETIP-02**: `/net` file tree with devip-exact semantics: tcp/udp/icmp clone/ctl/data/status + ipifc; loopback + null mediums land FIRST
- [x] **NETIP-03**: TCP debuggable without the NIC: `ping 127.0.0.1`, udp echo, in-guest tcp/7 echo over loopback
- [x] **NETIP-04**: arp + ethermedium bound to devether; static ipconfig assigns an IPv4 address; guest reachable from host
- [x] **NETIP-05**: DHCP guest configuration via ipconfig (`/net/udp` broadcast fidelity) or smoltcp dhcpv4 fallback
- [x] **NETIP-06**: IPv6: devip ip6 tree + ipconfig IPv6 configuration
- [x] **NETIP-07**: aan session-reliability layer available for cpu connections
- [ ] **NETIP-08**: Networking exit gate: sustained bidirectional TCP flow + load soak on the tap leg (slirp forgives L2 bugs — ping on slirp is not a gate)

### Host Networking (HNET)

- [ ] **HNET-01**: slirp+hostfwd hermetic default: hostfwd bound to 127.0.0.1 for BOTH tcp/17010 and tcp/567 (drawterm dials the auth server directly); zero privilege; CI-able
- [ ] **HNET-02**: `--net=slirp|tap|vmnet` explicit mode selection on the boot app (no silent auto-downgrade); netdev stanza single-sourced as data in r9 platform TOML, consumed by Harvey's flake
- [ ] **HNET-03**: linux tap/bridge leg (qemu-bridge-helper + br0) documented per-distro and tested — the L2-truth leg
- [ ] **HNET-04**: darwin socket_vmnet leg (root daemon + fd handoff) documented as opt-in (builtin vmnet is entitlement-walled on nixpkgs qemu — ad-hoc signature)
- [ ] **HNET-05**: doctor-style checker diagnosing privileged host setup, printing exact sudo/bridge/firewall remediation

### In-Guest Auth Domain (AUTH)

- [ ] **AUTH-01**: Bake-time domain generator: machine key + `/adm/keys` + nvram-substitute + ndb entries produced as ONE hash-paired unit (all four p9sk1 configs agree by construction); demo domain loudly labeled DEMO; pairing fails loud
- [ ] **AUTH-02**: r9 `/dev/random` CSPRNG (DTB-seeded) for p9sk1 challenges
- [ ] **AUTH-03**: r9 devcap `#¤` (caphash/capuse) — the `auth_chuid` path the cpu server depends on
- [ ] **AUTH-04**: keyfs starts non-interactively over the baked `/adm/keys`; authsrv listens on tcp/567
- [ ] **AUTH-05**: ndb content for the guest domain (authdom, hostid, sys, ether) staged in the image
- [ ] **AUTH-06**: In-guest loopback p9sk1 probe (auth/debug) green BEFORE any drawterm attempt; non-hostowner speaksfor gate
- [ ] **AUTH-07**: Serial-visible auth logging from day one (p9sk1 failures are opaque otherwise)
- [ ] **AUTH-08**: secstore available for persistent key storage beyond baked keys
- [ ] **AUTH-09**: writable-`/adm` + nvram-substitute scoping decided and documented (baked+paired for CI; host-9P `/adm` mount for real use)

### CPU Service (CPU)

- [ ] **CPU-01**: r9 `#!` exec (tcp17010 is literally `#!/bin/cpu -R`, execl'd by listen — small but load-bearing)
- [ ] **CPU-02**: `service=cpu` bootconf path: plan9.ini delivered via DTB `/chosen/bootargs` → cpurc runs instead of termrc (bootr9.c currently hardcodes terminal)
- [ ] **CPU-03**: aux/listen + tcp17010 live at boot; exportfs served; r9 implements WSTAT/FWSTAT/REMOVE (exportfs Twstat path)
- [ ] **CPU-04**: Full `cpu -R` remoteside flow verified end-to-end: p9any→p9sk1 → auth_chuid → client exportfs mounted at /mnt/term → stdio from /mnt/term/dev/cons → rc -li

### Drawterm + rio Desktop (DESK)

- [ ] **DESK-01**: Flake drawterm app: provenance decision recorded (research recommends nixpkgs 9front drawterm + wrapper); wrapper pins `-O`, scrubs `$cpu`/`$auth`/`$PASS`, prints the exact connect line
- [ ] **DESK-02**: profile `case cpu` binds /mnt/term/dev over /dev and execs rio (draw device exists only in-session; serial marker proves rio did NOT start at boot)
- [ ] **DESK-03**: `/lib/font/bit/*` + glenda-style lib/profile + `/lib/ndb/local` template staged in bootfs with manifest assertions
- [ ] **DESK-04**: r9 devssl `#D` (rc4_256/sha1) so drawterm sessions can run encrypted via pushssl; `-e clear` remains the documented fallback. Recorded tension: stronger crypto on this path is blocked by PROTOCOL compatibility, not primitives — 4e cpu negotiates only `rc4_256 sha1`, and drawterm's TLS mode is rcpu+dp9ik (a different protocol stack Harvey doesn't serve; that's the hardening milestone). In the hermetic default (hostfwd→127.0.0.1) session bytes never leave the host.
- [ ] **DESK-05**: plumb/plumber runs in the drawterm session
- [ ] **DESK-06**: `-G` text-mode drawterm interop gate in CI via hostfwd (scripted login → shell marker)
- [ ] **DESK-07**: Human-accepted graphical rio leg on darwin AND linux (window opens, mouse works, typed input lands)

### Integration (INT — continuing v1.0 numbering)

- [ ] **INT-05**: `nix run .#boot-r9` remains the ONE command — now brings up the full system (net + auth + cpu listener); boot-r9-smoke extended to the new chain
- [ ] **INT-06**: r9 flake.lock pin advanced past the networking commits as its OWN commit + fresh hosted CI run (push-before-pin; never bundled with the feature needing it)
- [ ] **INT-07**: Clean-checkout proof of the full v2.0 chain on darwin-aarch64 and linux

## Future Requirements

Deferred to future milestones. Tracked but not in the v2.0 roadmap.

- virtio-9p host mount in r9 qemu-virt (collapses userland iteration time)
- SMP via PSCI; PL031 RTC; virtio-blk
- virtio-gpu / framebuffer console in r9 (native graphics without drawterm)
- fossil/venti RUNTIME (needs a block device — compile-only in v2.0)
- APE port to aarch64 (POSIX environment)
- Go userland if plan9/arm64 support ever lands upstream
- dp9ik + rcpu + TLS 1.2/1.3 server-side (the TLS/security hardening milestone — replaces DESK-04's era crypto)
- Harvey-on-r9 for amd64 (replace 9k on the primary arch)
- Hardware aarch64 bring-up (r9 already has raspi3b/4b base)

## Out of Scope

Explicitly excluded from v2.0. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| APE (`sys/src/ape`) | No arm64 in `ape/lib/ap` (verified); a separate port, not a cmd dir; nothing in the drawterm session path needs it |
| Go userland on aarch64 | Go unsupported on plan9/arm64 |
| Harvey 9k (C kernel) aarch64 port | Superseded by r9 adoption (2026-10-30); 9k stays amd64 reference |
| e1000 / other NIC models | virtio-net only; one driver, one truth |
| virtio-gpu / framebuffer in r9 | The draw device arrives over 9P from drawterm — the entire point of the drawterm architecture |
| dp9ik / rcpu / TLS server-side | Different auth+session protocol stack (9front); Harvey is 4e p9sk1 — hardening milestone scope |
| MSI / v2m virtio interrupts | GICv2 has no MSI on this machine; INTx level-triggered SPIs from the DTB interrupt-map only |
| slirp as a correctness target | Hermetic default + CI leg only; it forgives L2 bugs — tap is the truth leg (NETIP-08) |
| DHCPv6 / SLAAC tooling | IPv6 scoped to devip ip6 + static/ipconfig configuration (NETIP-06) |

## Traceability

Each requirement maps to exactly one phase. Populated 2026-11-04 at roadmap creation (v2.0 = Phases 6–12, continuing v1.0 numbering).

| Requirement | Phase | Status |
|-------------|-------|--------|
| USPACE-01 | Phase 6 | Complete |
| USPACE-02 | Phase 6 | Complete |
| USPACE-03 | Phase 6 | Complete |
| USPACE-04 | Phase 6 | Complete |
| USPACE-05 | Phase 6 | Complete |
| USPACE-06 | Phase 6 | Complete |
| USPACE-07 | Phase 10 | Pending |
| USPACE-08 | Phase 10 | Pending |
| USPACE-09 | Phase 10 | Pending |
| USPACE-10 | Phase 6 | Complete |
| NETDRV-01 | Phase 7 | Complete |
| NETDRV-02 | Phase 7 | Complete |
| NETDRV-03 | Phase 7 | Complete |
| NETDRV-04 | Phase 7 | Complete |
| NETIP-01 | Phase 8 | Complete |
| NETIP-02 | Phase 8 | Complete |
| NETIP-03 | Phase 8 | Complete |
| NETIP-04 | Phase 8 | Complete |
| NETIP-05 | Phase 8 | Complete |
| NETIP-06 | Phase 8 | Complete |
| NETIP-07 | Phase 8 | Complete |
| NETIP-08 | Phase 8 | Pending — machinery proven on slirp (08-08/08-09); the tap-leg truth verdict is Phase 9's joint acceptance gate (D-15) |
| HNET-01 | Phase 9 | Pending |
| HNET-02 | Phase 9 | Pending |
| HNET-03 | Phase 9 | Pending |
| HNET-04 | Phase 9 | Pending |
| HNET-05 | Phase 9 | Pending |
| AUTH-01 | Phase 11 | Pending |
| AUTH-02 | Phase 11 | Pending |
| AUTH-03 | Phase 11 | Pending |
| AUTH-04 | Phase 11 | Pending |
| AUTH-05 | Phase 11 | Pending |
| AUTH-06 | Phase 11 | Pending |
| AUTH-07 | Phase 11 | Pending |
| AUTH-08 | Phase 11 | Pending |
| AUTH-09 | Phase 11 | Pending |
| CPU-01 | Phase 12 | Pending |
| CPU-02 | Phase 12 | Pending |
| CPU-03 | Phase 12 | Pending |
| CPU-04 | Phase 12 | Pending |
| DESK-01 | Phase 12 | Pending |
| DESK-02 | Phase 12 | Pending |
| DESK-03 | Phase 10 | Pending |
| DESK-04 | Phase 12 | Pending |
| DESK-05 | Phase 12 | Pending |
| DESK-06 | Phase 12 | Pending |
| DESK-07 | Phase 12 | Pending |
| INT-05 | Phase 12 | Pending |
| INT-06 | Phase 12 | Pending |
| INT-07 | Phase 12 | Pending |

**Coverage:**
- v2.0 requirements: 50 total
- Mapped to phases: 50 ✓
- Unmapped: 0

**Mapping notes:**
- DESK-03 (bootfs fonts/profile/ndb payload) maps to Phase 10, not Phase 12 — research stages the payload with the GUI tier so Phases 11–12 never block on it; Phase 12 consumes it.
- USPACE-10 (v1.0 carry-over pool-panic re-test) rides Phase 6's Wave 0 — cheapest known-signature verification, alongside the gethunk landmine fixes.

---
*Requirements defined: 2026-11-04*
*Last updated: 2026-11-04 — traceability populated at v2.0 roadmap creation*
