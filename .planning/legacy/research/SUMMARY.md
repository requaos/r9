# Project Research Summary

**Project:** Harvey OS v2.0 "Full System & Drawterm"
**Domain:** Plan 9-derivative OS milestone — full aarch64 userspace + r9 kernel networking + p9sk1 auth + cpu/drawterm desktop (two-repo: Harvey + sibling `../r9`)
**Researched:** 2026-11-04
**Confidence:** HIGH

## Executive Summary

This milestone turns the v1.0 16-binary proof into a full Plan 9 system: every buildable `sys/src/cmd` target and library on aarch64, a real TCP/IP path in the r9 kernel behind the devip `/net` file ABI, an in-guest p9sk1 auth domain, and the canonical drawterm→cpu→rio desktop session over tcp/17010. The decisive research finding (verified across all four files): **Plan 9 networking is a file-interface contract, not a syscall API** — libc `dial()`/`announce()`/`listen()` are pure I/O on `/net/tcp/clone` et al. (zero net syscalls exist in `sys.h`), so whoever provides TCP must present devip's exact file tree or ipconfig/cs/dns/listen/cpu all break. Harvey ships a kernel C stack (`sys/src/9k/ip/`, 15.4k lines) with userspace *clients only* — there is no Harvey userspace TCP/IP stack to reuse.

Almost everything server-side already exists in-tree (cpu.c, aux/listen, auth/*, rio, GUI libs, service scripts, cpurc/ndb templates — all verified). The genuinely new code is confined to r9 (PCI/ECAM + virtio-net + devether + the `/net` stack + small device/syscall additions) and host-side plumbing (qemu netdev tiers + flake drawterm app). The full-userspace sweep is a build-scope change, not new imports — but at ~3,357 files it is the milestone's largest workstream and its whack-a-mole failure mode is the top schedule risk.

Key risks and mitigations: (1) compile-green≠run-correct at 250-command scale → tiered allowlist waves, per-tier execution gates on the 9front donor kernel *before* r9, never a flag-flip; (2) virtio offload/feature landmines ("ping works, TCP hangs") → zero-offload feature mask on Day 1, exit gate is a sustained bidirectional TCP flow on the **tap leg**, not ping on slirp; (3) auth first-boot chicken-and-egg without NVRAM → one bake-time domain generator producing hash-paired keys+ndb+nvram-substitute so all four p9sk1 configs agree by construction; (4) two open research disagreements (TCP implementation choice, drawterm provenance) → resolved below as recommendations with decision-spike flags.

## Key Findings

### Recommended Stack

Core finding: the only real stack *decisions* live in r9 and in the drawterm package; everything else is vendored/in-tree already. Stack additions (see STACK.md for versions/pins):

- **virtio-drivers 0.13 (rcore-os crate)**: virtio-net driver + PCI/ECAM transport for r9 — the only maintained no_std Rust virtio stack; its PCI transport is memory-mapped CAM/ECAM only, an exact match for qemu `-M virt` aarch64 (ECAM window at `0x4010000000` in the committed DTB). Caveat: `VIRTIO_F_VERSION_1` negotiation is TODO upstream — gate with a boot-test and keep the `virtio-non-transitional` qemu habit (transitional 0x1000 vs modern 0x1041 was the v1.0 donor trap).
- **smoltcp 0.14**: TCP/UDP/ICMP/IPv4 behind a devip-compatible `/net` — recommended over porting the 15.4k-line C stack (see open decision below). no_std, MSRV 1.91 cleared by r9's nightly-2026-01-30, `medium-ethernet`+`proto-ipv4`+`socket-tcp/udp/icmp`+`alloc` feature set.
- **nixpkgs `pkgs.drawterm` (9front fork, rev 3aa7ce0c, `0-unstable-2026-10-12`)**: protocol-compatible with Harvey's `cpu -R` (p9any→p9sk1 + rc4_256/sha1, verified against 9front `cpu.c`), builds cocoa on darwin / X11 on linux / wayland variant exists. **Requires the `-O` flag + env scrub wrapper** (see Pitfall 13).
- **socket_vmnet (nixpkgs `socket-vmnet`)**: unprivileged darwin real-L2 path (root daemon + fd handoff), because qemu's builtin `vmnet-*` netdevs need root or a restricted Apple entitlement nix builds can't carry.
- **No new userspace imports**: full aarch64 sweep compiles from what's vendored (9front 7a/7c/7l pin, libc/aarch64, portable-C GUI libs with the `mksyslib` `%.$O: %.c` fallback — do NOT cargo-cult the committed stale `.6`/`.8` binary objects).
- **Excluded by decision**: APE (no arm64 in `ape/lib/ap`, nothing in the session path needs it), Go userland, 9k C-kernel aarch64 port, virtio-gpu/framebuffer in r9 (draw arrives over 9P — the whole point of drawterm), dp9ik/TLS/rcpu server-side, e1000/other NICs.

### Expected Features

The full drawterm→cpu session lifecycle was verified end-to-end against in-tree Harvey sources and drawterm source — it is the behavior contract (FEATURES.md §0). Table stakes per area:

- **Full aarch64 userspace**: shell/coreutils, guest build chain (7a/7c/7l + cc/cpp/mk/yacc/lex), session daemons (aux/listen, mntgen, ndb/cs+dns, exportfs), auth suite (80 files in cmd/auth), GUI stack (libdraw/libmemdraw/libmemlayer/libframe + rio + fonts), IP userspace (ipconfig, ping). Known structural blockers first: `cmd/jpg/mkfile` recursion abort (kills the DIRS sweep before `ip/`+`venti/`), mahjongg cpp, `4a` lex.c.
- **Guest networking**: virtio-net in r9 + devip-compatible `/net` + devether `/net/ether0` + loopback + static ipconfig (DHCP is a differentiator, needs `/net/udp` broadcast fidelity — flag for phase research, smoltcp dhcpv4 is the fallback).
- **Auth domain**: keyfs over `/adm/keys` + authsrv on tcp/567 + baked machine-key/nvram-substitute + ndb content + **r9 devcap `#¤`** (auth_chuid is how cpu becomes the user). DES/libsec already green from v1.0.
- **CPU service**: `cpu -R` at tcp/17010 via aux/listen + cpurc + `service=cpu` boot path; devmnt mount of drawterm's exportfs at `/mnt/term`; **`#!` exec in r9** (tcp17010 is literally `#!/bin/cpu -R` and listen execl()s it — small but load-bearing).
- **rio desktop**: GUI libs + rio + `/lib/font/bit/*` in the image + profile `case cpu` doing the `/mnt/term/dev` binds (NOT cpurc — draw device only exists inside the session).
- **Host side**: flake drawterm app + slirp/hostfwd unprivileged default (CI-able) + documented tap/bridge (linux) and socket_vmnet/vmnet (darwin) real-L2 tiers.

**Defer (scope pressure-valves, in order):** devssl rc4_256/sha1 (ship `-e clear` first — the natural descope valve), gs (753 files, GPL island), upas/lp/fax, fossil/venti (no block device — compile-only, unverifiable), secstore, APE, IPv6, aan, dhcpd, acme/sam (post-rio differentiators).

### Architecture Approach

Everything above the kernel line is **existing Harvey userspace, ported not invented**; everything on the kernel line is **new Rust in r9**; the host's only novelties are qemu netdev wiring and the drawterm package. Major components (ARCHITECTURE.md has the full boundary table):

1. **r9 PCI/ECAM platform glue** (`aarch64/src/platform/virt/pci.rs`) — DTB-driven ECAM scan + `interrupt-map`→GICv2 SPI routing; never hardcoded IRQs/addresses (qemu moved ECAM once already).
2. **r9 virtio transport + devether `#l`** (`port/src/virtio.rs` or the virtio-drivers crate + `devether.rs`) — split virtqueues, zero-offload feature mask, barrier-first aarch64 ring code, `/net/ether0` per `netif.h`.
3. **r9 devip-compatible `/net`** — the milestone's pivotal component: clone/ctl/data/status files, ipifc, arp, loopback+ether mediums, over smoltcp (recommended) or a staged Rust port of `9k/ip` (alternative). Loopback-first staging decouples TCP debugging from driver heisenbugs.
4. **r9 kernel service additions** — `#!` exec, devcap `#¤`, `/dev/random` (DTB-seeded CSPRNG), WSTAT/FWSTAT/REMOVE, tracer-gated syscall extensions (measure-first rule).
5. **Boot/bootconf path** — plan9.ini `service=cpu` via DTB `/chosen/bootargs` → cpurc instead of termrc (bootr9.c currently hardcodes terminal).
6. **Auth bake tooling** — one host-side generator: machine key + `/adm/keys` + nvram-substitute + ndb entries, hash-paired; baked for CI determinism, host-9P `/adm` mount for real use.
7. **Flake wiring** — netdev stanza single-sourced as data in r9's platform TOML (consumed by Harvey's boot-r9 app), `--net=slirp|tap|vmnet` explicit, doctor-checker for privileged setup, drawterm wrapper app.

### Critical Pitfalls

Top items from PITFALLS.md (16 critical + 6 moderate total):

1. **Compile-green ≠ run-correct at 250-command scale** — tiered allowlist waves (append-only-via-gate), per-tier execution on the 9front donor kernel FIRST (decouples 7c-bugs from r9-gaps), then r9 via measure-first traces; mk exit status is always 0 in the namespace build — artifact+execution markers are the only gates.
2. **Cross-compiled generators can't run on the host** (`jpg`'s ycbcr.h, `awk`'s proctab.c class) — Wave-0 audit of every tier mkfile for build-then-run recipes; generators always build with the host toolchain, never `$CC`; CI reds on `.7`-magic generator artifacts.
3. **Preemptive `gethunk` ×2 fixes** — Phase 2 left two known cc/6l landmines ("fix when touched"); the full tree IS the trigger (compile-HANG mid-wave). Known bug, known fix pattern (7l/unixcompat.c), cheapest critical pitfall to close — Wave 0.
4. **virtio feature-negotiation + aarch64 ordering + GICv2 plumbing** — zero-offload mask (MAC+STATUS only), barrier-first virtqueues (descriptor→store-store barrier→avail idx), INTx-first level-triggered SPIs from the DTB interrupt-map + ISR-read-to-ack, MSI/v2m a documented non-goal. Exit gate: long-lived bidirectional TCP + load soak + interrupt accounting, **on the tap leg** (slirp masks L2 bugs — it forgives ARP/broadcast/marginal frames).
5. **Auth bootstrap without NVRAM + p9sk1 opacity** — bake-time paired generator (nothing else in AUTH is testable without it); serial auth logging from Day 1; in-guest loopback p9sk1 probe (`auth/debug`) green BEFORE any drawterm attempt; clock skew is a red herring on p9sk1 (no timestamps in tickets) — don't burn cycles on it.
6. **9front drawterm defaults to rcpu/TLS/dp9ik** — bare invocation = TLS ClientHello into the p9sk1 listener = hang with no error. Wrapper owns `-O -e clear` (or negotiated crypt) + scrubs `$cpu`/`$auth`/`$PASS`; proven by a scripted `-G` text-mode interop gate in CI, graphical leg human-accepted per host OS.

## Implications for Roadmap

Suggested phase structure (7 phases; two intentional parallel tracks):

### Phase 1: USERSPACE Wave 0 + Libraries/Core Tiers
**Rationale:** Cheapest critical fixes are known bugs with known fixes (gethunk ×2, jpg recursion abort); Wave-0 audits (generator host-toolchain sweep, aarch64 bug-class grep) must precede any tier. Genuinely parallel with all r9 work — different repo, no shared code.
**Delivers:** mk recursion fixed; tier-0 libs (auth-adjacent first: libsec/libmp/libauth/libauthsrv/libndb, then libdraw family) + tier-1 core cmds green on donor AND r9 with per-tier markers; mkimage manifest assertion extended to the full bake set.
**Addresses:** Full-userspace table stakes (shell/core, build chain); unblocks everything.
**Avoids:** Pitfalls 1, 2, 3, 4, 5 (ape excluded by name in the phase goal).

### Phase 2: r9 PCI/ECAM + virtio-net + devether
**Rationale:** First DMA-capable device in r9; the DTB interrupt-map parser + level-trigger programming spike goes first (v1.0 tracer-first shape) because it's the first nontrivial interrupt plumbing. Zero-offload mask from Day 1.
**Delivers:** `/net/ether0` file tree; RX/TX packet counters; ARP request visible on host; barrier-audited virtqueues; interrupt accounting gate.
**Uses:** virtio-drivers 0.13 (recommended) or `port/src/virtio.rs`; committed DTB as the contract.
**Avoids:** Pitfalls 6 (partially), 7, 8; cautionary tale 4 (pin `virtio-net-pci-non-transitional` in TOML before writing driver code).

### Phase 3: r9 TCP/IP Path — devip-Compatible /net
**Rationale:** The milestone's pivotal component. Loopback-first staging (devip core + null/loopback mediums → icmp/udp → tcp → arp/ethermedium bound to devether) means the TCP state machine is debuggable before/without the NIC — sequencing gold, explicitly parallel with Phase 2.
**Delivers:** `/net/{tcp,udp,icmp,ipifc,arp,ndb}` with clone/ctl/data semantics; `ping 127.0.0.1` → udpecho → in-guest tcp/7 echo over loopback → ipconfig static IP on ether; exit gate = sustained bidirectional TCP **on the tap leg**.
**Uses:** smoltcp 0.14 (recommended — see open decision); libc's announce/dial contract as the spec; `sys/src/9k/ip/` as semantics reference (read-only).
**Avoids:** Pitfalls 6, M1 (any plan growing the syscall table for networking is a red flag — announce/listen are file ops), M4 (msize ≥64KiB), 15 (serial + filter-dump capture wired from day one).

### Phase 4: Host Networking + boot-r9 Net Modes
**Rationale:** Slirp+hostfwd leg is pure qemu-args work (can start with Phase 2); acceptance needs Phase 3's ether binding. Tap leg is the L2-truth exit gate for the whole networking story; darwin real-L2 is documented opt-in only (entitlement wall is certain on stock macs).
**Delivers:** `--net=slirp|tap|vmnet` on boot-r9 (no silent auto-downgrade); netdev stanza single-sourced in r9 TOML and consumed by Harvey's flake; doctor-checker printing exact sudo/bridge/firewall remediation; hostfwd bound to 127.0.0.1.
**Avoids:** Pitfalls 9, 10, M2, M3; security mistake (0.0.0.0 hostfwd).
**Implements:** Host-side tier matrix (slirp default / socket_vmnet darwin / tap+bridge linux).

### Phase 5: USERSPACE Services + GUI Tiers + Bootfs Payload
**Rationale:** Services (ip/, ndb/, auth/, aux/listen) and GUI (rio, acme, sam, plumb) ride the Phase 1 machinery; GUI binaries stay compile-gated until Phase 7 gives them somewhere to run, but their donor-kernel syscall traces must land here (measure-first for rio/acme — libthread programs exercise unmeasured kernel corners). Fonts/profiles/ndb templates staged into the image here so Phases 6–7 never block on payload.
**Delivers:** Tier-2/3 allowlist green; rio+acme+sam binaries; `/lib/font/bit/*`, glenda-style `lib/profile`, `/lib/ndb/local` template in bootfs with manifest assertions.
**Avoids:** Pitfalls 1 (tail), M5 (GUI syscall trace before DESKTOP commits), M6 (allowlist append-only), 14's fonts half.

### Phase 6: In-Guest p9sk1 Auth Domain
**Rationale:** Depends on Phase 3 loopback (AS dial) and Phase 5 binaries; must complete — with its loopback probe green — before DESKTOP starts (Pitfall 12 makes the probe an entry criterion, not a nicety). Wave 0 is the bake-time domain generator: nothing else in AUTH is testable without it.
**Delivers:** Bake tool (keys+nvram-substitute+ndb hash-paired); keyfs non-interactive start with pairing assertion; authsrv on tcp/567; r9 kernel additions (`#!` exec, devcap `#¤`, DTB-seeded `/dev/random`, WSTAT/FWSTAT/REMOVE); `auth/debug` loopback gate + non-hostowner speaksfor gate; serial auth logging.
**Avoids:** Pitfalls 11, 12; security mistakes (demo domain labeled DEMO; pairing fail-loud).
**Note:** the "instant cpu server" shortcut (factotum key + listen1, no AS) is a legitimate intermediate stepping stone, not the shipped gate.

### Phase 7: CPU Service + Drawterm + rio Desktop
**Rationale:** The milestone's user-visible payoff; strictly last — it consumes every other phase (this strictness is the finding, not a choice: FEATURES.md's dependency graph is a chain).
**Delivers:** `service=cpu` bootconf path (plan9.ini via DTB bootargs); real cpurc; aux/listen + tcp17010; drawterm flake app + wrapper (`-O`, env scrub, printed connect line); `-G` text-mode CI gate via hostfwd + human-accepted graphical rio leg on darwin and linux; rio started from profile (serial marker proves it did NOT start at boot).
**Avoids:** Pitfalls 13, 14, 15 (runbook), 16 (pin-bump discipline).
**Ships with:** `-e clear` documented (devssl deferred to the hardening milestone — weak rc4/sha1 crypto is an anti-feature in the new kernel anyway).

### Phase Ordering Rationale

- **Tracks:** {1, 5} (Harvey userspace) run fully parallel with {2, 3, 4} (r9/host networking); {6} needs {3-loopback, 5}; {7} needs all. Only two hard cross-track joins: Phase 6 and Phase 7.
- **Loopback-before-NIC** (Phase 3 ∥ Phase 2) is the highest-leverage decoupling in the research — it keeps the hardest software (TCP) out of the heisenbug zone (driver + weakly-ordered rings + level-triggered IRQs).
- **Donor-before-r9 execution gates** in every userspace tier keep 7c-miscompiles and r9-gaps unconfounded — the v1.0 lesson generalized.
- **Auth-before-desktop with a loopback gate** converts the four-way p9sk1 config agreement from a guessing game into a construction (one generator) plus an assertion (the probe).
- **`-e clear` first** makes devssl the clean descope valve without touching the success chain.

### Research Flags

Phases likely needing deeper research during planning (`--research-phase` or an in-phase decision spike):

- **Phase 3 (TCP/IP path):** **OPEN DECISION between research files** — STACK.md recommends smoltcp 0.14 behind a devip-compatible `/net`; ARCHITECTURE.md recommends a staged Rust port of the `9k/ip` IPv4 subset (est. 6–8k lines Rust) and its option table never considered smoltcp-in-kernel. This synthesizer's recommendation: **smoltcp** — both files agree the hard contract is the `/net` file ABI (which must be hand-built either way), and smoltcp replaces the TCP/UDP/ICMP/ARP machinery (the port's real cost: Reno, timers, reassembly, Block buffers) with a maintained, upstream-tested stack. The port's only advantage — semantic fidelity for free — is mitigated by STACK.md's verified enumeration of the exact file interface. Must be FINAL before the driver plan (PITFALLS: it determines whether M1's ABI work lands kernel-side or user-side). Related sub-decision: virtio-drivers crate (STACK) vs from-scratch `port/src/virtio.rs` (ARCHITECTURE) — same recommendation, crate. Also: DHCP-vs-static (static recommended for the milestone; smoltcp dhcpv4 as fallback).
- **Phase 6 (AUTH):** small spikes — nvram-substitute form (baked devroot file vs qemu nvram backend) and writable-`/adm` scoping (image-baked vs ramfs overlay vs host-9P mount). Research recommends: baked + hash-paired for CI, host-9P for real use, both documented.
- **Phase 4 (HOSTNET):** small decision spike — darwin real-L2 opt-in (socket_vmnet vs manually-signed qemu vmnet). Not a research phase.
- **Phase 7 (DESKTOP):** drawterm provenance is formally open — FEATURES.md prefers 9fans (4e protocol fidelity, no `-O` needed) but nixpkgs doesn't package it; STACK.md verified the nixpkgs 9front fork's p9sk1 compatibility directly from source. Recommendation: **nixpkgs 9front drawterm + wrapper** (`-O`, env scrub) — "nix run .#drawterm from a clean checkout" with a custom 9fans derivation buys nothing the wrapper doesn't already provide. Decide in the phase plan, not at the invocation site.

Phases with standard patterns (skip research-phase):

- **Phase 1, 5 (USERSPACE):** bug classes, landmines, and tier handling are fully enumerated in PITFALLS.md; what they need is Wave-0 audits scheduled as real tasks, not research.
- **Phase 2 (NETDRV):** crate docs + DTB contract + 9front `ethervirtio10.c` reference are sufficient; the interrupt-parser spike is a task, not research.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All in-tree facts verified by direct reads; crate/nixpkgs facts MEDIUM (docs.rs + raw nixpkgs, cross-verified) — no LOW-tier claim is load-bearing |
| Features | HIGH | The drawterm→cpu session lifecycle is verified end-to-end against Harvey's own sources (the tree IS the spec, being a 4e derivative) + drawterm source |
| Architecture | HIGH | All structural claims read directly from both trees; effort estimates (Rust port sizes) and qemu host-networking claims are MEDIUM |
| Pitfalls | HIGH | Grounded in this repo's v1.0 artifacts (deferred-items, retrospective) + primary protocol/spec docs; symptom-pattern corroboration from vendor trackers is MEDIUM |

**Overall confidence:** HIGH

### Gaps to Address

- **TCP/IP implementation choice (smoltcp vs 9k/ip port) and virtio driver provenance (crate vs from-scratch):** research files disagree; recommendation above; resolve at Phase 3 plan time — this is the milestone's one genuinely open architecture decision.
- **Drawterm provenance (9front-nixpkgs vs 9fans-custom):** resolve in Phase 7 plan; wrapper spec is agreed by all three files that address it.
- **nvram substitute + writable /adm scoping:** open design detail for Phase 6 (baked+paired vs host-9P vs ramfs overlay).
- **r9 effort estimates** for devip/devether/devcap/#! exec are inference from C line counts (MEDIUM) — planning inputs, not commitments.
- **libdraw remote-`segattach` fallback** on a 9P-served draw device: expected to "just work" (libdraw falls back to draw messages), MEDIUM confidence — verify in Phase 5 or 7.
- **qemu virtio-net-on-virt drift:** 11.1.2 already moved ECAM once; pin device variants in TOML and treat the committed DTB as the contract.
- **Hosted CI tap permissions:** assumed unavailable (hostfwd covers CI); confirm early in Phase 4.
- **nixpkgs pin age:** must be new enough to carry `drawterm 0-unstable-2026-*` + `socket-vmnet` — check at Phase 4/7.
- **r9 pin:** Harvey `flake.lock` r9 input (9549cee) must advance past the networking commits — pin-bump = own commit + fresh CI leg, never bundled with the feature that needs it (carried-over v1.0 rule).

## Sources

### Primary (HIGH confidence)
- Harvey in-tree: `sys/src/cmd/cpu.c` (full read), `rc/bin/service/tcp17010`, `cpurc`, `lib/namespace`, `usr/glenda/lib/profile`, `lib/ndb/{common,local,auth}`, `cmd/auth/mkfile` + `keyfs.c`, `cmd/aux/listen.c:541`, `sys/src/9k/ip/` (15.4k lines), `9k/386/ethervirtio.c`, `9k/port/{devcap.c,sysproc.c:282-301}`, `libc/9sys/{announce,dial,pushssl}.c`, `libc/9syscall/sys.h` (no net syscalls), `libdraw/init.c`, `libauth/auth_chuid.c`, `ape/lib/ap` (no arm64), `9k/root/profile`, `boot/bootr9.c`, in-tree man pages (cpu(1), authsrv(6), keyfs(4), auth(8), ndb(6), listen(8), rio(1), drawterm(8)); measured tree counts (274 cmd entries, 3357 .c/.y, 35 lib dirs).
- r9 in-tree: `port/src/` device inventory, `aarch64/src/{syscall.rs,exec.rs}`, `platform/virt/`, `rust-toolchain.toml`, `xtask` netdev pattern, committed DTB, `HowItWorks.md`.
- v1.0 planning artifacts: phase deferred-items (gethunk ×2, hswal, ECAM move, transitional virtio ID), RETROSPECTIVE.md (boot-test rule, measure-first rule, fresh-host lesson), PROJECT.md known-breakage list.
- 9fans/drawterm `main.c`/`cpu.c` and 9front drawterm `cpu.c` + man page (protocol verification both forks).
- ARM BSA DEN0094C (GICv2 MSI/INTx facts); qemu docs (slirp/hostfwd, vmnet ≥7.1).

### Secondary (MEDIUM confidence)
- docs.rs: virtio-drivers 0.13.0 (PCI transport, VirtIONet, VERSION_1 TODO), smoltcp 0.14.0 (features, MSRV 1.91).
- Raw nixpkgs: `pkgs.drawterm` package.nix (rev 3aa7ce0c, platforms), `socket-vmnet` existence; brew formula 1.2.2.
- qemu-devel/Apple vmnet threads, qemu GitLab #1364 (entitlement wall); socket_vmnet/lima docs; Guix/OpenWrt qemu-bridge-helper docs.
- 9p.io/plan9.io wikis (standalone/instant cpu server recipes) — corroborated by in-tree files.
- virtio offload symptom-pattern corroboration (libvirt-users, RH bz 814898, virtio-win threads).

### Tertiary (LOW confidence)
- None load-bearing. (Research-store seam rated web digests LOW; per v1.0 convention the man-page/spec/tracker claims above were treated as primary/secondary.)

---
*Research completed: 2026-11-04*
*Ready for roadmap: yes*
