# Phase 16: Host Net Modes + Networking Exit Gate - Research

**Researched:** 2026-11-10
**Domain:** qemu host networking (slirp/hostfwd, tap+bridge via setuid helper, darwin vmnet) + flake app/gate engineering across the Harvey and r9 repos
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

> Provenance note from CONTEXT.md: **all decisions below were auto-selected (recommended) without interactive confirmation** — every D-NN is flagged [auto] and legitimately re-litigable until planning. Items marked "carried" are locked prior-phase discipline and NOT re-litigable.

### Locked Decisions ([auto] — review before planning)

**`--net` surface & failure UX (HNET-01, HNET-02)**

- **D-01 [auto]: Flag + env default.** `boot-r9-x64.sh` parses `--net=slirp|tap|vmnet` from its own args (`nix run .#boot-r9-x64 -- --net=tap` — the roadmap's literal interface spelling); `R9_NET` sets the default when no flag is passed (the R9_ACCEL machine-contract precedent — CI/scripts get an env, humans get a flag). Flag wins over env; default `R9_NET=slirp`. No silent auto-downgrade, ever: an unknown mode is a usage error listing the valid modes; a cross-venue request (`--net=vmnet` on linux, `--net=tap` on darwin — darwin's tap is not this phase's leg) is rejected at parse naming the venue's valid modes, never left to fail obscurely inside qemu.
- **D-02 [auto]: Mode selection rides the interactive boot app only.** `boot-r9-x64` gains `--net`; the composed gates (net-smoke-x64, services-x64, tier gates) keep fixed slirp venues — their bakes substitute `slirp_` seam values and must stay deterministic; a tap-mode services bake would need tap-substituted fixtures no requirement names. The tap soak gate is its own dedicated app (D-11).
- **D-03 [auto]: A missing-prereq tap boot fails loud WITH the doctor's remediation embedded.** The boot script's tap leg invokes the same check implementation the doctor app runs (ONE implementation — D-12's library consumed two ways), prints the per-issue remediation blocks inline in the failure, and names `nix run .#net-doctor` for the full report. Never auto-fixes.
- **D-04 [auto]: hostfwd tcp/567 is host-side-remapped to 5670.** The slirp default's hostfwd set is `tcp:127.0.0.1:17010-:17010` and `tcp:127.0.0.1:5670-:567` — guest ports stay 17010/567 (the in-guest service contracts Phases 18/19 own), but the HOST entry point for auth is 127.0.0.1:5670. Rationale: HNET-01's "zero privilege; CI-able" is the hard constraint and Linux refuses unprivileged binds below 1024 (`ip_unprivileged_port_start` defaults to 1024) — a literal host:567 mapping would make the hermetic default require a sysctl. The 1:1 alternative is documented in the docs as the opt-in sysctl variant. The remap is data in the TOML slirp stanza (D-05), never script text.

**Netdev single-sourcing (HNET-02)**

- **D-05 [auto]: Mode sub-tables in the ONE platform TOML + r9 exports a netdev-args emitter.** The x86_64 platform TOML's `[qemu.netdev]` grows per-mode tables — `[qemu.netdev.slirp]` (mode/device/hostfwd list incl. the D-04 remap/pcap), `[qemu.netdev.tap]` (mode/device/bridge/pcap), `[qemu.netdev.vmnet]` (darwin-only, documented-manual) — plus a default key, all in the config the flake boots (q35shell lineage). r9's flake exports a small emitter package: a bin reusing the xtask config structs with `apply_netdev`'s arg-building refactored into shared code — ONE emission implementation, two front-ends (in-process xtask, exported CLI). `boot-r9-x64.sh` replaces its hardcoded `-netdev user,id=n0 -device virtio-net-pci-non-transitional,netdev=n0` line with command substitution of the emitter's output for the selected mode; the flake app exports the emitter path env-style (`R9_ISO_BUILDER` precedent).
- **D-06 [auto]: Host-qemu params live in the TOML; guest-side addressing lives in the netaddr seam.** The TOML owns everything qemu sees (netdev/device/hostfwd/bridge names); the seam (`build/x86_64/root-tree/lib/netaddr.rc`) gains the `tap_` leg keys (tap_guestv4/mask/gw from the bridge subnet; `blastport=7777` reused). The bake-time venue-literal grep assert extends to the tap values — one seam, mechanically asserted, no tap literal anywhere else.
- **D-07 [auto]: Tap legs are composed-system (Harvey-side) only.** r9's work this phase = the TOML mode tables + the emitter export (+ soak-surfaced kernel fixes under the 15-D-01 fix-everything doctrine, push-before-pin, FOD re-wire-then-pin after every kernel-byte fix pass — carried). NO new r9-side tap boot-test profiles: NETIP-08 is a composed gate per the 14-D-06 gates-split; kernel-level /net evidence stays on the existing profiles.

**Tap bridge topology (HNET-03, HNET-04)**

- **D-08 [auto]: NAT'd private bridge (libvirt-default style).** br0 with a fixed private subnet (planner picks — must NOT collide with slirp's 10.0.2.0/24), host owns the bridge's .1, masquerade out the default route, NO LAN→br0 inbound. Self-contained on any host network (the NixOS primary host included, wifi/VPN hosts too), deterministic subnet drives the seam, and the guest is never exposed to external networks. Real-LAN bridging ships as ONE documented variant paragraph in the docs, explicitly not tested.
- **D-09 [auto]: Static guest addressing via the seam; DHCP-on-tap is a documented non-goal.** The netip-static fixture re-venues (ipconfig ether bind → add → `-g` default route, all through the real devip tree) with `tap_` seam values; `/net/arp` gains the host-entry assertion post-flow (the L2 proof this venue exists for). DHCP fidelity is already gated on slirp (14-09); tap DHCP would add dnsmasq as a privileged host moving part for zero gate value. The seam's v6 note is honored only as a researcher-verified trivially-free extension — v6 is NOT a NETIP-08 criterion and joins only if it costs nothing.
- **D-10 [auto]: Per-distro docs = NixOS (tested) + one generic-Linux reference (documented) + darwin socket_vmnet manual.** NixOS gets the full declarative path (networking.bridges, ip_forward, masquerade/trustedInterfaces, the qemu bridge-helper wrapper + `/etc/qemu/bridge.conf` `allow br0` via environment.etc) proven on the primary host; generic Linux gets one page of equivalent imperative commands — documented-not-tested; darwin stays HNET-04's opt-in documented-manual (builtin vmnet is entitlement-walled on nixpkgs qemu). Companion README follows the `build/aarch64/README.md` drift-notes discipline.

**Soak gate + doctor checker (HNET-05, NETIP-08)**

- **D-11 [auto]: Tiered soak on the existing tcpblast, refusing slirp outright.** (a) baseline: tcpblast against the host-side echo peer at a raised, KVM-scaled round count (the committed default 64 is TCG-sane; the tap leg's count is gate data, planner-tunable); (b) burst: N parallel tcpblast instances (8) backgrounded from the sweep rc, each asserting its own completion marker; (c) wedge scan: the gate timeout catches stalls loudly + the `/net/arp` host-entry truth assert (D-09). Exact numbers are committed data in the gate harness. Green-twice on the primary host; the app HARD-REFUSES a slirp run.
- **D-12 [auto]: The host-side echo peer is a flake-built committed binary (tcpecho lineage), not socat/ncat.** A tiny host-compiled echo server listening on `blastport` bound to the bridge IP — deterministic markers and byte-accounting owned by the gate, no external-tool behavior drift. It runs as the gate app's own child (no daemon, no systemd, dies with the gate). The doctor checks the port free.
- **D-13 [auto]: The soak gate is a new dedicated flake app in the app-per-gate shape** (name planner discretion, `boot-r9-net-soak-x64`-style): doctor-preflight → bake the net image (mkimage's net section + tcpblast, unchanged 14-09 contract) → compose ISO → start the host echo peer → boot `--net=tap` through the emitter → drive the in-guest sweep via expect → assert completion + ARP + captures retained → green-twice → evidence consolidated in the phase dir with the r9 SHA. Runs on the primary host only (needs br0); hosted CI keeps the slirp legs.
- **D-14 [auto]: The doctor is read-only diagnose+print, never fixes, and is one implementation consumed two ways.** Flake app `nix run .#net-doctor` (+ invoked inline by the boot tap leg per D-03). Checks per mode: slirp (qemu present, hostfwd ports free), tap (br0 exists+up, bridge IP present, ip_forward=1, masquerade rule present, qemu-bridge-helper executable/setuid + bridge.conf `allow br0`, blastport free), vmnet (darwin: socket_vmnet present — informational). Output: per-check PASS/FAIL blocks, every FAIL printing the exact remediation — NixOS declarative snippet FIRST on NixOS, generic commands otherwise. Exit 0/1 = CI-able preflight.

### Carried forward (locked in prior phases — do not re-litigate)

- Netdev-as-data: `[qemu.netdev]` TOML consumed by the generalized `apply_netdev` emitter, q35-gated (Phase 13, OQ-2 resolved); aarch64 stays frozen-at-pin.
- Gates split by artifact (14-D-06): kernel-level r9-side, composed-system Harvey-side.
- Venue honesty: slirp never L2 truth (asserted NOT-L2-TRUTH stage in slirp captures); named venues; no unearned OK.
- R9_ACCEL precedent semantics: explicit modes, no auto, fail-loud naming fallbacks (D-01 applies them to net).
- push-before-pin; pin advance is its own commit + fresh hosted CI; FOD hash re-stales per kernel commit — re-wire-then-pin after every r9-byte fix pass.
- The netaddr single-seam discipline (14-09 D-11): no venue address literal outside `netaddr.rc`, mechanically asserted by the bake gate.
- The 14-09 net payloads ride unchanged: tcpblast, tcpecho, ipconfig; sourced-not-typed in-guest sweeps; serial markers; presence-based timing-tolerant assertions.
- NETIP-08 sequencing (v2.0 08-D-15): machinery + slirp proof were Phase 8's (done, aarch64); the tap-leg truth verdict is THIS phase's joint acceptance gate.

### Claude's Discretion

- The emitter package's exact name/shape in r9's flake (a bin crate or an xtask subcommand wrapped — the constraint is ONE shared arg-building implementation with in-process xtask).
- The private bridge subnet choice (collision-free with 10.0.2.0/24 and common LAN ranges).
- The doctor's implementation language/file home (bash in `nix/` mirroring the script-app shape is the obvious default) and check ordering.
- Per-distro docs file layout (companion README beside `build/x86_64/` per the aarch64 precedent).
- The v6 seam keys' inclusion (researcher-verified trivially-free only — D-09's constraint).
- The soak app's name, capture/evidence file layout, and the concurrency mechanics in the rc sweep (backgrounding + marker collection).
- Whether the slirp machinery-proof leg re-runs anywhere this phase (not required — the XEXEC-05 gates already cover slirp; the soak app's refusal is the venue control).

### Deferred Ideas (OUT OF SCOPE)

- **v6 legs on tap** — joins only as a researcher-verified trivially-free extension; NOT a NETIP-08 criterion. If not free, the phase that first needs v6 on x64 picks it up.
- **Real-LAN bridge variant** — documented paragraph only this phase; testing it is a future demand-named item.
- **DHCP-on-tap** (dnsmasq on br0) — documented non-goal; DHCP fidelity stays gated on slirp.
- **1:1 host:567 via sysctl** — the opt-in variant documented alongside D-04's default; not the gate's shape.
- **r9-side tap boot-test profiles** — can join Phase 17's flush-out if a soak finding wants a kernel-level pin.
- **Per-distro pages beyond NixOS + generic Linux** — demand-named later.
- **A tap-mode services/gates bake** — no requirement names it; revisit when one does.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HNET-01 | slirp+hostfwd hermetic default: hostfwd bound to 127.0.0.1 for BOTH tcp/17010 and tcp/567; zero privilege; CI-able | Live-probed hostfwd syntax + unprivileged bind proof (17010/5670 OK, 567 EPERM); ip_unprivileged_port_start=1024 verified on host; no x64 CI leg exists today — a new slirp CI leg must land (see CI Reality) |
| HNET-02 | `--net=slirp\|tap\|vmnet` explicit mode selection, no silent auto-downgrade; netdev stanza single-sourced as data in r9's x86_64 platform TOML, consumed by Harvey's flake | apply_netdev/Netdev struct read verbatim; 5 flat [qemu.netdev] TOMLs enumerated; emitter package shape via kernel-x64/iso-builder precedents; no arg parsing exists in boot-r9-x64.sh today (R9_ACCEL env-case is the template) |
| HNET-03 | linux tap/bridge leg (qemu-bridge-helper + br0) documented per-distro and tested on the primary host — the L2-truth leg | Live host: br0 exists 192.168.122.1/24, setuid wrapper at /run/wrappers/bin/qemu-bridge-helper, bridge.conf allows only virbr0; qemu 11.1.2 accepts helper=/ + br= on tap netdev (live-probed); compiled-in helper default is the nosuid store path — helper= MUST be explicit on NixOS |
| HNET-04 | darwin socket_vmnet leg documented as opt-in documented-manual | socket_vmnet README facts (root daemon, socket paths, brew/launchd, entitlement Apple-restricted); nixpkgs socket-vmnet 1.2.2 aarch64-darwin exists; vmnet-socket,path= syntax |
| HNET-05 | doctor-style checker diagnosing privileged host setup, printing exact sudo/bridge/firewall remediation | Every check probed live on the primary host; masquerade-rule check needs root (nft unreadable unprivileged) — design note; nft NOT on PATH, iptables present |
| NETIP-08 | Networking exit gate: sustained bidirectional TCP flow + load soak on the tap leg on x86_64 (slirp refused) | tcpblast CLI/marker contract read verbatim; aarch64 netip-soak.rc precedent (4096-round sustained scale, pcap + /net/tcp/stats reconciliation); rc wait builtin semantics verified in-tree; multi-waiter RX-IRQ kernel risk named in the existing gate's header |
</phase_requirements>

## Summary

Phase 16 turns host networking from an accident of one hardcoded qemu line into three explicit, data-sourced modes. The r9 side carries the data: the flat `[qemu.netdev]` table (mode/device/pcap) that five q35 TOMLs share today grows per-mode sub-tables (`slirp`/`tap`/`vmnet` + a default key), and r9's flake exports a small emitter package — `apply_netdev`'s arg-building refactored into one shared function consumed by in-process xtask and a new bin, exactly the kernel-x64/iso-builder export precedent. The Harvey side carries the surface: `boot-r9-x64.sh` heals its hardcoded `-netdev user` line into emitter substitution, gains `--net`/`R9_NET` in the R9_ACCEL fail-loud shape, the netaddr seam grows `tap_` keys, the bake literal-grep assert extends, two new flake apps (soak gate + doctor) join the app-per-gate family, and per-distro docs land.

The host leg is unusually well-prepared on the primary NixOS machine: **br0 already exists at 192.168.122.1/24** (empty, stp off), **ip_forward is already 1**, the **setuid qemu-bridge-helper wrapper already lives at `/run/wrappers/bin/qemu-bridge-helper`**, and `/etc/qemu/bridge.conf` exists containing `allow virbr0` — the remediation is literally one `allow br0` line plus masquerade. The decisive NixOS divergence (flagged in the CONTEXT watch-outs) is now proven with byte-level evidence: **both the system qemu and the flake's nixpkgs qemu 11.1.2 compile the helper's default path into their own read-only store libexec (nosuid, cannot work), and neither build carries a `QEMU_BRIDGE_HELPER` env override** — so the tap stanza MUST carry `helper=/run/wrappers/bin/qemu-bridge-helper` explicitly, which qemu 11.1.2 accepts on both the `tap` and `bridge` backends (live-probed). D-04's 5670 remap is likewise proven necessary: `ip_unprivileged_port_start=1024` on this host, port 567 bind fails EPERM as an unprivileged user while 17010/5670/7777 bind clean.

The soak rides machinery that already exists: tcpblast's exact contract (`host port [rounds]`, default 64, 32 KiB patterned round-trip byte-compare, per-round + done markers, one dialed connection for all rounds, no pacing), the aarch64 netip-soak.rc precedent (sustained scale = 4096 rounds/256 MiB, pcap both-directions + zero-RST reconciliation + `/net/tcp/stats` counters), and rc's `wait` builtin (waits for all outstanding children when given no pid — verified in `sys/man/1/rc:736-743`). The one named kernel risk is written into the existing net gate's header: the multi-waiter RX-IRQ wake defect (aarch64 08-09) — the 8-way burst leg WILL hit the multi-waiter path, and any fix lands in-phase under 15-D-01 push-before-pin.

**Primary recommendation:** shape the r9 emitter as a lib-target refactor inside the xtask crate (one arg-builder fn + config structs moved to `xtask/src/lib.rs`, the new bin a second `src/bin/` consumer) exported through a stdenv.mkDerivation mirroring `kernel-x64`; put `helper=/run/wrappers/bin/qemu-bridge-helper` in the tap TOML stanza verbatim; adopt the host's existing br0/192.168.122.1/24 as the committed seam values (collision-free with 10.0.2.0/24 and the host's 192.168.86.0/24 LAN); and design the burst leg's assertions as marker-COUNT post-passes (never rc status collection — rc cannot collect per-child statuses after bare `wait`).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Netdev argv emission (netdev/device/hostfwd/helper/pcap) | r9 repo (TOML data + emitter package) | Harvey flake (consumes exported bin) | Single-source-as-data: qemu sees one authored stanza; both front-ends (xtask, Harvey's boot script) print the same bytes |
| `--net` mode selection + venue validation | Harvey `nix/boot-r9-x64.sh` | — | The boot app is the roadmap's literal interface; fail-loud parse precedented by R9_ACCEL case |
| Guest-side addressing (tap_ leg) | Harvey `netaddr.rc` seam | mkimage bake assert | 14-09 D-11: guest addresses live ONLY in the seam, mechanically asserted at bake |
| Host privileged setup (br0, masquerade, setuid helper, ACL) | Host OS config (NixOS declarative / colmena) | net-doctor app (diagnoses, never fixes) | OS-owned state; the doctor is read-only diagnose+print with exact remediation |
| Slirp hermetic default (hostfwd 17010 + 5670→567) | r9 TOML slirp stanza | CI leg (r9-shell.yml family) | Zero-privilege constraint is data-driven: the remap rides the stanza, not script text |
| Host-side echo peer (blastport listener) | Harvey flake package (tcpecho-lineage binary) | soak gate app (child process lifecycle) | D-12: no socat/ncat drift; dies with the gate |
| Soak gate orchestration | Harvey flake app + expect driver | host echo peer + in-guest rc sweep | 14-D-06 gates-split: composed-system gate is Harvey-side; primary host only |
| Kernel fixes surfaced by the soak | r9 repo (push-before-pin) | Harvey flake.lock advance (own commit) | 15-D-01 fix-everything; FOD re-wire-then-pin carried |

## Standard Stack

No new registry packages are installed this phase. The stack is the existing cross-repo machinery plus host configuration. Versions verified live on the primary host.

### Core

| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| qemu (nixpkgs, flake runtimeInputs) | 11.1.2 | the emulator all gates boot under | Already the flake's qemu; hostfwd/`tap,helper=`/filter-dump all live-probed on exactly 11.1.2 [VERIFIED: live probe of /nix/store/bp9li432ba8adp47dr9z9y0hx56wn8v2-qemu-11.1.2] |
| qemu (system, qemu-host-cpu-only) | 11.1.2 | the primary host's interactive qemu | Same version; compiled-in helper path divergence verified identical [VERIFIED: live probe] |
| r9 xtask config structs (`toml` 0.9 + `serde`) | as pinned in ../r9/xtask/Cargo.toml | the TOML→argv pipeline D-05 refactors | [VERIFIED: ../r9/xtask/Cargo.toml — `toml = "0.9"`, serde derive] |
| mkScriptApp + writeShellApplication (nixpkgs) | nixos-unstable pin 1d060086 | the flake app shape for soak + doctor | [VERIFIED: flake.nix:26-39] |
| NixOS `security.wrappers` + `environment.etc` | NixOS 26.11pre-git (host) | setuid qemu-bridge-helper + bridge.conf | [VERIFIED: live host — wrapper `-r-s--x--x` at /run/wrappers/bin/qemu-bridge-helper; /etc/qemu/bridge.conf → /etc/static/qemu/bridge.conf] |

### Supporting

| Component | Version | Purpose | When to Use |
|-----------|---------|---------|-------------|
| socket-vmnet (nixpkgs) | 1.2.2, aarch64-darwin only | darwin vmnet daemon (documented-manual leg) | HNET-04 docs; brew route is the README alternative [VERIFIED: `nix eval nixpkgs#socket-vmnet` → 1.2.2, platforms ["aarch64-darwin"]] |
| iproute2 (`ip`, `ss`) | system profile | doctor checks (link/addr/port) | present on host PATH [VERIFIED: live] |
| expect | flake runtimeInput | soak gate in-guest driver | not on bare PATH; rides mkScriptApp runtimeInputs [VERIFIED: live PATH audit] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `-netdev tap,...,helper=/run/wrappers/bin/qemu-bridge-helper,br=br0` | `-netdev bridge,br=br0` | bridge backend also accepts `helper=` (live-probed) but defaults to the nosuid store path on nixpkgs; explicit `tap`+`helper=`+`br=` is the honest, NixOS-correct shape — D-05's TOML should express it |
| Committed C echo peer (D-12) | socat/ncat | rejected by D-12 (behavior drift, no owned markers) — planner must not "simplify" to socat |
| nft-based masquerade check in doctor | iptables read | nft NOT on host PATH; iptables present [VERIFIED: live]; the flake app can add `nftables` to runtimeInputs — planner choice |
| Emitter as new workspace crate | second bin in xtask | xtask is single-bin, no lib target today — a lib-target refactor (src/lib.rs) is less surface than a new crate [VERIFIED: ../r9/xtask/Cargo.toml has no [[bin]]; src/ = config.rs, main.rs, pcap.rs, uregcheck.rs] |

**Installation:** none (no new packages). Host-side privileged setup is documented NixOS config, applied by the human (D-10), checked by the doctor (D-14).

## Package Legitimacy Audit

This phase installs **no new external packages** — no npm/PyPI/crates additions, no new nixpkgs attrs consumed by the flakes beyond what the existing apps already reference (qemu, expect, plan9port, iproute2-class tools, grub2/xorriso in r9). The one new darwin-named dependency (socket-vmnet) is **documented-manual only** (HNET-04) — it is never installed by either flake.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| qemu (nixpkgs attr) | nixpkgs | decades | n/a (nixpkgs) | gitlab.com/qemu-project/qemu | OK | Already consumed (pre-existing) |
| socket-vmnet (nixpkgs attr) | nixpkgs | 1.2.2 | n/a | github.com/lima-vm/socket_vmnet | OK | Docs-only reference (darwin manual leg); never a flake input |
| toml/serde/clap/target-lexicon (r9 xtask deps) | crates.io | pinned in ../r9/Cargo.lock | n/a | canonical | OK | Pre-existing, committed lock; no additions required |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                       ┌─────────────────────────── r9 repo (../r9) ───────────────────────────┐
                       │  x86_64/lib/config_q35*.toml                                      │
                       │    [qemu.netdev] default="slirp"                                  │
                       │      [qemu.netdev.slirp] mode/device/hostfwd[]/pcap                │
                       │      [qemu.netdev.tap]   mode/device/br/helper/pcap                │
                       │      [qemu.netdev.vmnet] mode/device/socket-path (darwin)          │
                       │        │ (read at build)                                          │
                       │        ▼                                                          │
                       │  xtask lib refactor: ONE netdev_args(config, mode, pcap) -> argv   │
                       │   ├── xtask main.rs (in-process QemuStep/BootTestStep)             │
                       │   └── src/bin/<emitter> (prints argv for --mode M)                 │
                       │        ▼                                                          │
                       │  flake.nix packages.<emitter>  (mirrors kernel-x64 derivation)     │
                       └────────────────┬───────────────────────────────────────────────────┘
                                        │ flake input (pin advance = own commit)
                                        ▼
┌────────────────────────── Harvey repo ─────────────────────────────────────────────────────┐
│ flake.nix: export R9_NETDEV_EMITTER (R9_ISO_BUILDER precedent) in boot-r9-x64 + new apps   │
│   boot-r9-x64 (interactive)      net-doctor (new)          net-soak-x64 (new, D-13)        │
│        │ --net/R9_NET parse            │ checks              │ doctor-preflight            │
│        │  flag > env > default slirp   │ (read-only)         │ bake net image (mkimage)    │
│        ▼                               ▼                     │ compose ISO (iso-builder)   │
│  MODE==slirp → emitter argv      PASS/FAIL + remediation    │ host echo peer on br0 IP    │
│  MODE==tap   → doctor-inline ──FAIL──→ abort with blocks    │ boot --net=tap via emitter  │
│  MODE==vmnet on linux → usage error (cross-venue reject)     │ expect drives in-guest rc   │
│        │                              ▲                     │  baseline + 8-way burst     │
│        ▼                              │                     │  + ARP assert + wedge scan   │
│  qemu-system-x86_64                    │                     │  slirp runs REFUSED         │
│   slirp: -netdev user,id=n0,           │                     ▼ evidence + r9 SHA, 2x green│
│          hostfwd=tcp:127.0.0.1:17010-:17010,                                       │
│          hostfwd=tcp:127.0.0.1:5670-:567   (zero privilege, CI)                            │
│   tap:   -netdev tap,id=n0,br=br0,                                                     │
│          helper=/run/wrappers/bin/qemu-bridge-helper (setuid) ◄── NixOS-only reality      │
│   vmnet: -netdev vmnet-socket,id=n0,path=/var/run/socket_vmnet (darwin, manual)           │
│        │                                                                                │
│        ▼                                                                                │
│  guest /net via seam: netaddr.rc  lo_/slirp_/tap_ keys + blastport=7777                  │
│   (bake-time literal-grep assert extends to the tap subnet)                              │
└──────────────────────────────────────────────────────────────────────────────────────────┘
         │ tap leg                                            │ slirp leg
         ▼                                                    ▼
   HOST: br0 192.168.122.1/24                          HOST: 127.0.0.1:17010/:5670
   setuid qemu-bridge-helper + bridge.conf "allow br0"  (hostfwd → guest 17010 / 567)
   ip_forward=1 + masquerade → default route (wifi)     hosted CI (r9-shell.yml family,
   host echo peer binds br0-IP:7777                     R9_ACCEL=tcg, no privilege)
```

### Recommended Project Structure

```
r9 (../r9):
├── xtask/src/lib.rs            # NEW (or equivalent): shared config structs + the ONE
│                               #   netdev-args builder (moved from main.rs reach)
├── xtask/src/bin/<emitter>.rs  # NEW: thin CLI --mode <m> [--pcap-path <p>] → prints argv
├── xtask/src/config.rs         # Netdev struct grows mode sub-tables; apply_netdev
│                               #   delegates to the shared builder
├── x86_64/lib/config_q35*.toml # 5 configs migrate flat [qemu.netdev] → sub-tables
└── flake.nix                   # packages.<emitter> export (kernel-x64-shaped derivation)

harvey:
├── nix/boot-r9-x64.sh          # --net/R9_NET parse + emitter substitution + doctor-inline
├── nix/net-soak-r9-x64.sh      # NEW (D-13 app body; smoke-r9-net-x64.sh shape)
├── nix/net-doctor.sh           # NEW (D-14; one implementation, two front-ends)
├── nix/host-echo.c             # NEW host echo peer source (tcpecho lineage, blastport,
│                               #   concurrent accepts) — or planner-chosen home
├── flake.nix                   # 2 new apps + R9_NETDEV_EMITTER-style export + hasAttr gates
├── build/x86_64/netsoak-r9.expect     # NEW expect driver (smoke-r9-net.expect bones)
├── build/x86_64/root-tree/lib/netaddr.rc      # tap_ leg keys (+optional trivially-free v6)
├── build/x86_64/root-tree/lib/netip-tap.rc    # NEW re-venued fixture (netip-static shape)
├── build/x86_64/mkimage.sh     # literal-grep assert extends to the tap subnet; netsoak bake
└── build/x86_64/README.md      # NEW (or docs home): NixOS tested + generic Linux + darwin
```

### Pattern 1: One emitter, two front-ends (D-05)

**What:** the arg-building currently inline in `apply_netdev` becomes a pure function returning the argv tokens; the in-process xtask callers push them onto `Command`, the exported bin prints them.

**Today's code, verbatim** [VERIFIED: ../r9/xtask/src/config.rs:479-492]:

```rust
pub fn apply_netdev(cmd: &mut Command, config: &Configuration, pcap_path: &str) {
    let q35 = config.config.as_ref().and_then(|c| c.platform.as_deref()) == Some("q35");
    if !q35 {
        return;
    }
    let Some(netdev) = config.qemu.as_ref().and_then(|q| q.netdev.as_ref()) else {
        return;
    };
    cmd.arg("-netdev").arg(format!("{},id=n0", netdev.mode));
    cmd.arg("-device").arg(format!("{},netdev=n0", netdev.device));
    if netdev.pcap {
        cmd.arg("-object").arg(format!("filter-dump,id=f0,netdev=n0,file={pcap_path}"));
    }
}
```

The struct it consumes [VERIFIED: ../r9/xtask/src/config.rs:128-139]:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Netdev {
    /// netdev backend mode ("user" — slirp)
    pub mode: String,

    /// PCI device model bound to the netdev (virtio-net-pci-non-transitional
    /// — the modern-only, non-transitional transport, Phase 7 D-03)
    pub device: String,

    /// attach a per-run filter-dump pcap capture object when true
    pub pcap: bool,
}
```

Call sites today: QemuStep (main.rs:742) and BootTestStep (main.rs:1699), each passing its own pcap path (`target/qemu-<config>.pcap` / `target/boot-test-<config>.pcap`). The refactor keeps both call shapes byte-identical (the unit test `q35_netdev_stanza_emits_from_data` at config.rs:806+ asserts the exact argv and must keep passing for the default path).

**Packaging:** xtask is a single-bin crate with no lib target [VERIFIED: ../r9/xtask/Cargo.toml — no `[[bin]]`, members list in workspace Cargo.toml]. Two minimal shapes: (a) add `xtask/src/lib.rs` holding the shared builder + config module, make `main.rs` and a new `src/bin/<emitter>.rs` consumers; (b) a new workspace member crate depending on nothing but serde/toml. Shape (a) is the least-churn realization of "ONE emission implementation, two front-ends" and keeps the existing tests in place.

**TOML migration scope:** ALL FIVE q35 configs carry the identical flat stanza today [VERIFIED: grep over ../r9/x86_64/lib/config_{q35,q35exec,q35lowmcfg,q35net,q35shell}.toml — each has `mode = "user"` under `[qemu.netdev]`]. Migrating to mode sub-tables is a breaking struct change: all five TOMLs, the `Netdev` struct, `apply_netdev`, and the config.rs unit tests change together. config_q35shell.toml:42-45 today:

```toml
[qemu.netdev]
mode = "user"
device = "virtio-net-pci-non-transitional"
pcap = true
```

**When to use:** always this phase — it IS D-05.

### Pattern 2: Flag-over-env fail-loud mode selection (D-01)

**What:** the R9_ACCEL case in boot-r9-x64.sh, verbatim [VERIFIED: nix/boot-r9-x64.sh:95-107]:

```sh
R9_ACCEL="${R9_ACCEL:-kvm}"
case "$R9_ACCEL" in
	kvm)
		[ -e /dev/kvm ] || err "/dev/kvm absent and R9_ACCEL=$R9_ACCEL — no silent downgrade: set R9_ACCEL=tcg explicitly for the TCG fallback (the Phase-13 discipline)"
		R9_CPU="${R9_CPU:-host}"
		;;
	tcg)
		R9_CPU="${R9_CPU:-qemu64}"
		;;
	*) err "R9_ACCEL=$R9_ACCEL: want kvm|tcg" ;;
esac
```

**Critical fact: the script parses NO arguments today** — `--net` is a NEW arg surface (the R9_ACCEL env-case is the only precedent). `nix run .#boot-r9-x64 -- --net=tap` delivers `--net=tap` as `$1` to the script, which currently ignores `"$@"`. The mode validation lives in a new case/parse block adjacent to the accel block; cross-venue rejection (vmnet on linux, tap on darwin) is a usage error naming the venue's valid modes — same `err` idiom.

**The hardcoded line D-05 heals** [VERIFIED: nix/boot-r9-x64.sh:162-167]:

```sh
set -- qemu-system-x86_64 \
	-M q35 -cpu "$R9_CPU" -smp 1 -m 1024M -accel "$R9_ACCEL" \
	-cdrom "$ISO" \
	-netdev user,id=n0 \
	-device virtio-net-pci-non-transitional,netdev=n0 \
	-nographic
```

### Pattern 3: The seam + bake assert extension (D-06/D-09)

The current assert, verbatim [VERIFIED: build/x86_64/mkimage.sh:512-519]:

```sh
# 10b. the D-11 single-seam discipline, asserted at bake time (T-14-17
#      carried mechanically): NO venue address literal may live in any
#      root-tree fixture EXCEPT netaddr.rc — the grep scopes to the
#      fixtures directory and excludes the seam by name.  A stray
#      10.0.2.x elsewhere re-couples the gates to one venue and reds
#      the bake here, never the eye later.
_stray=$("$GREP" -rn '10\.0\.2\.' "$ROOT_TREE/lib/" 2>/dev/null | "$GREP" -v 'netaddr\.rc' || true)
[ -z "$_stray" ] || err "venue address literal outside netaddr.rc (the D-11 seam violated): $_stray"
```

The extension is mechanical: a second grep for the chosen tap subnet literal (e.g. `192\.168\.122\.`) with the same netaddr.rc exclusion. **Watch-out (15-07 lesson):** the fixtures' own comment prose had to be reworded once before — netaddr.rc's tap-section comments must be the ONLY place the literal appears in prose too.

The seam's existing keys and the phase-naming comment [VERIFIED: build/x86_64/root-tree/lib/netaddr.rc:37-44]:

```rc
slirp_guestv4=10.0.2.15
slirp_maskv4=255.255.255.0
slirp_gwv4=10.0.2.2

# tcpblast host-echo port (the aarch64 soak dialled slirp_gwv4:blastport
# with the sweep owning the host-side echo listener; the x64 gate bakes
# the binary this phase, the soak itself joins with the Phase-16 legs).
blastport=7777
```

The v6 note [VERIFIED: netaddr.rc:19-22]: "The aarch64 seam's v6 keys (slirp_guestv6/gwv6, fec0::) are NOT carried: no x64 gate this phase speaks v6 — they join with Phase 16's NETIP legs (the tap venue), re-keyed from the aarch64 probe findings."

### Pattern 4: The composed gate pipeline (D-13's bones)

The soak app copies nix/smoke-r9-net-x64.sh's pipeline stage-for-stage: env-contract validation (R9_KERNEL_X64_PKG, R9_ISO_BUILDER) → accel case → KERNDATE provenance → subset-build.sh → mkimage.sh (net section → bootfs-net.paq) → per-run tag + survivor-sweep trap (`pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}"`, unique prefix so gates never cross-kill) → iso-builder compose → expect driver with fresh rm -f'd capture → exit 0/1. The expect driver spawns qemu DIRECTLY [VERIFIED: build/x86_64/smoke-r9-net.expect:174-179]:

```tcl
spawn $QEMU -M $R9_MACHINE -cpu $R9_CPU -smp 1 -m 1024M \
    -accel $R9_ACCEL \
    -cdrom $env(R9_ISO) \
    -netdev user,id=n0 \
    -device virtio-net-pci-non-transitional,netdev=n0 \
    -nographic \
    -name $run_tag
```

The soak driver replaces the two netdev lines with the emitter's tap argv — natural mechanism: the caller shells out to the emitter once and exports the tokens (e.g. `R9_NETDEV_ARGS`) for the driver to split; marker discipline (presence-only, CRLF-tolerant, two-occurrence for typed lines, ≤120-char typed input with continuation-line pacing) is documented at expect:96-127 and carries verbatim.

### Pattern 5: rc burst leg (D-11b)

rc's wait builtin, verbatim [VERIFIED: sys/man/1/rc:736-743]:

```
.BI "wait [" pid "]"
.br
Wait for the process with the given
.I pid
to exit.
If no
.I pid
is given, all outstanding processes are waited for.
```

And `$status` is "Set to the wait message of the last-executed program (unless started with &)" [VERIFIED: sys/man/1/rc:881-883] — i.e. **per-child statuses are NOT collectible after a bare `wait`**. The burst pattern must therefore be: launch N tcpblasts with `&` (the services-sweep precedent backgrounds daemons this way at services-sweep.rc:199/221/280), `wait`, then let the gate assert the COUNT of completion markers in the capture (expect post-pass counting `tcpblast: done` occurrences = N) — never rc-side status collection, and never interleaving-order assumptions (the CONTEXT watch-out). Optional hardening: a post-wait rc-side count of the capture is NOT possible in-guest (no file to grep — markers ride the console), so the expect post-pass owns the count assert.

### Anti-Patterns to Avoid

- **Hand-editing the host (`ip link add br0` / manual sysctl) on the NixOS primary host** — D-10 demands the declarative path proven; manual edits drift on next rebuild (the host is colmena-managed: /etc/nixos/configuration.nix literally `throw`s "managed by colmena" [VERIFIED: live read]).
- **Relying on `-netdev bridge,br=br0` without `helper=`** — the compiled-in default helper path is the qemu package's own nosuid store libexec on BOTH system and flake qemu 11.1.2; unprivileged bridge attach then fails obscurely inside qemu [VERIFIED: live binary string extraction — `/nix/store/<hash>-qemu-11.1.2/libexec/qemu-bridge-helper`; 0 occurrences of `QEMU_BRIDGE_HELPER` in the binary].
- **Binding hostfwd to 0.0.0.0** — HNET-01 says 127.0.0.1 explicitly; the hermetic default must not expose the guest's future auth/cpu listeners to the LAN (the host is on wifi, 192.168.86.0/24).
- **Asserting burst marker ORDER** — 8 backgrounded tcpblasts interleave freely on one serial console; presence-per-count only.
- **Adding the new r9 attr to the EXISTING hasAttr gate list** — flake.nix:266-267 gates ALL x64 apps on `[ "kernel-x64" "iso-builder" ]`; extending that list makes every x64 app vanish against r9 pins older than the emitter. New apps should carry their own optionalAttrs gate (or the whole list moves only after the pin advance commit) — the CONTEXT's FOD/re-wire-then-pin watch-out generalizes to the flake API surface.
- **Touching the slirp gates' baked fixtures when adding tap keys** — additive keys only, last-wins discipline (CONTEXT watch-out: "the tap venue must NOT perturb the existing slirp gates' fixtures").

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Tap creation + bridge attach as root | custom setuid tool / sudo scripts | qemu-bridge-helper (the existing setuid wrapper) + bridge.conf ACL | The helper is qemu's own audited mechanism; the NixOS wrapper already exists on the host [VERIFIED: live] |
| Host-side echo server semantics | socat/ncat invocations | a tiny committed C (or nixpkgs-std) binary in tcpecho lineage (D-12) | deterministic markers, byte-accounting owned by the gate, no tool drift across hosts |
| Netdev argv strings in shell | printf/assembling `-netdev` lines in boot-r9-x64.sh | the r9 emitter's output (command substitution) | D-05: single source; the D-04 remap and helper path are data, never script text |
| Guest address constants | literals in fixtures/expect | netaddr.rc seam values | 14-09 D-11 mechanical bake assert |
| Port-availability checking | ad-hoc `netstat` parsing | `ss -tln` (present on host) with exact port match | simple, present; the doctor's port-free checks are one-liners [VERIFIED: ss on PATH] |
| Sustained-soak scale numbers | inline magic counts in expect | committed gate-data file/thresholds (D-11: "exact numbers are committed data in the gate harness") | single-source-as-data; KVM-vs-TCG scales differ (64 default TCG-sane; 4096 = the aarch64 sustained precedent) |

**Key insight:** every hard part of this phase (setuid helper, ACL, masquerade, hostfwd, pcap, echo semantics, marker discipline) already exists as a proven mechanism in qemu, NixOS, or the 14-09 payload set — the phase's engineering is wiring and data placement, not invention.

## Runtime State Inventory

Not a rename/refactor phase — omitted per protocol. (Host-side state relevant to this phase is covered under Environment Availability; the only mutable host state the phase documents is br0 + masquerade + bridge.conf, all NixOS-declarative per D-10.)

## Common Pitfalls

### Pitfall 1: The nosuid store helper (the #1 NixOS divergence)

**What goes wrong:** `-netdev bridge,br=br0` (or tap without `helper=`) execs the helper at the qemu package's compiled-in path — inside /nix/store, which is mounted nosuid — so the setuid bit is dead and unprivileged tap creation fails "obscurely inside qemu" (exactly what D-01 exists to prevent).
**Why it happens:** nixpkgs compiles `CONFIG_QEMU_HELPERDIR` to the package's own libexecdir; NixOS's setuid copy lives only at `/run/wrappers/bin/qemu-bridge-helper`, and neither qemu build carries an env override (0 hits for `QEMU_BRIDGE_HELPER`).
**How to avoid:** the tap TOML stanza carries `helper=/run/wrappers/bin/qemu-bridge-helper` explicitly; the doctor checks the wrapper's presence + setuid + bridge.conf `allow br0`; the docs lead with this on NixOS.
**Warning signs:** qemu error "bridge helper failed" / "failed to create tun device" at boot with br0 and bridge.conf both correct.

### Pitfall 2: bridge.conf allows the WRONG bridge (live on this host)

**What goes wrong:** /etc/qemu/bridge.conf currently contains `allow virbr0` while the bridge that exists is `br0` — the helper denies with `access denied by acl file` [VERIFIED: live probe — running the wrapper against a non-allowed bridge prints exactly that].
**Why it happens:** the file came from the host's libvirt-era setup; the allow-list is bridge-name-exact.
**How to avoid:** doctor greps bridge.conf for the exact `allow br0` line and prints the environment.etc remediation; docs show `environment.etc."qemu/bridge.conf".text = "allow br0"` (append semantics if libvirt's virbr0 must keep working).
**Warning signs:** helper runs but denies despite correct wrapper.

### Pitfall 3: Port 567 cannot be hostfwd'd unprivileged

**What goes wrong:** a literal `hostfwd=tcp:127.0.0.1:567-:567` makes the hermetic default require privilege.
**Why it happens:** `net.ipv4.ip_unprivileged_port_start = 1024` on this host [VERIFIED: live sysctl]; a bind attempt on 567 as an unprivileged user fails with `Permission denied` [VERIFIED: live python bind probe — errno 13], while 17010/5670/7777 bind clean [VERIFIED: same probe].
**How to avoid:** D-04's remap (`tcp:127.0.0.1:5670-:567`) in the TOML stanza; the 1:1 sysctl variant is documented opt-in only. (ASSUMED, not live-probed to failure: qemu binds hostfwd listeners eagerly at netdev init, so a <1024 host port aborts qemu startup — the kernel EPERM is the verified part.)
**Warning signs:** CI leg failing at qemu launch with a bind error on 567.

### Pitfall 4: The multi-waiter RX-IRQ wake defect under burst

**What goes wrong:** 8 parallel tcpblast clients blocked in reads + the echo traffic = multiple registered devip waiters on the ether core — the measured aarch64 08-09 defect class ("with the loopback servers blocked in reads, the static leg's gateway ping wedged the guest fully idle").
**Why it happens:** recorded kernel-side wake-path gap; the existing gate's header explicitly warns "Phase 16's tap legs WILL hit the multi-waiter path — the record names it" [VERIFIED: build/x86_64/smoke-r9-net.expect header, lines ~50-56].
**How to avoid:** cannot be designed around — the soak's POINT is to surface it; the wedge scan (gate timeout catches stalls loudly) is the detection, and any fix lands in-phase push-before-pin (15-D-01) with FOD re-wire-then-pin.
**Warning signs:** burst leg hangs with zero markers after N-1 completions; guest idle with no alarm.

### Pitfall 5: Expect marker discipline on one serial console

**What goes wrong:** ordered stage assertions dead-lock or false-pass when 8 writers interleave; typed lines double-stage markers; >120-char typed lines truncate in the 256-byte RX ring.
**Why it happens:** preemption makes arrival order meaningless; devcons echoes cooked-mode input; the 16550 RX ring drops bursts (all documented at smoke-r9-net.expect:96-127).
**How to avoid:** presence-per-count assertions (count N completion markers, never order); two-occurrence discipline for typed-line-contained markers; short typed lines with continuation-prompt pacing.
**Warning signs:** expect stage timeouts while the capture plainly contains the marker later in the buffer.

### Pitfall 6: hasAttr gate / pin-advance ordering

**What goes wrong:** Harvey-side flake changes referencing `r9.packages.<system>.<emitter>` before the r9 commit lands (or bundling the pin advance with other changes) breaks every x64 app eval or violates INT-06.
**Why it happens:** the flake API surface widens (new exported package); the frozen apps' hasAttr list must not silently gain the new attr.
**How to avoid:** r9 commit + push first (push-before-pin); the emitter export and TOML changes as their own r9 commits; Harvey's flake.lock advance as its OWN commit with fresh hosted CI; new Harvey apps in their own optionalAttrs gate keyed on the attrs they actually consume.
**Warning signs:** `nix flake check`/eval errors on x86_64-linux referencing a missing r9 attr.

### Pitfall 7: Sequential host echo peer serializes the burst

**What goes wrong:** a host peer modeled verbatim on the in-guest tcpecho (single-threaded sequential accept — `listen` then one `accept` loop at a time [VERIFIED: build/x86_64/tcpecho.c:33-48]) services 8 concurrent tcpblast connections one-at-a-time; the "burst" degrades to a queue and the wedge-scan semantics blur.
**Why it happens:** tcpblast holds ONE dialed connection for ALL its rounds [VERIFIED: tcpblast.c:54-83 — dial once, loop rounds]; 8 instances = 8 simultaneously-open connections.
**How to avoid:** the D-12 host peer must handle concurrent connections (fork-per-accept or a poll loop) while preserving the echo semantics (read-what-came, write-it-back, partial-safe; the guest-side does the byte-compare).
**Warning signs:** burst leg wall-clock ≈ 8× baseline instead of ≈ baseline; connections timing out at the host while markers trickle.

### Pitfall 8: Doctor's masquerade check is root-gated

**What goes wrong:** `nft list ruleset` (and iptables-save) need root on this host; an unprivileged doctor cannot positively verify the masquerade rule.
**Why it happens:** netfilter reads are netlink-privileged; nft is not even on the default PATH (iptables is) [VERIFIED: live PATH audit].
**How to avoid:** design the check as: try the read; on EPERM print an explicit "needs root to verify" line with the exact `sudo` command, and treat it as a WARN-class finding (the doctor is diagnose+print per D-14 — degrade loudly, never silently pass). Alternatively the doctor runs under sudo for full fidelity — planner choice; the CONTEXT's "Exit 0/1 = CI-able preflight" must define which findings gate the exit code.
**Warning signs:** doctor green while guest traffic to the outside blackholes.

## Code Examples

### The slirp default stanza (D-04, HNET-01)

```
# Source: qemu.org docs (hostfwd form) + live probe on qemu 11.1.2 (parse + filter-dump coexistence)
-netdev user,id=n0,hostfwd=tcp:127.0.0.1:17010-:17010,hostfwd=tcp:127.0.0.1:5670-:567
-device virtio-net-pci-non-transitional,netdev=n0
-object filter-dump,id=f0,netdev=n0,file=<pcap-path>
```

hostfwd general form (qemu docs): `hostfwd=proto:hostaddr:hostport-guestaddr:guestport` — an empty guest address forwards to the guest's default address (10.0.2.15 under slirp). Multiple hostfwd rules ride one netdev comma-separated. Live-verified on qemu 11.1.2: both hostfwd rules + the filter-dump object parse and run together unprivileged; the pcap file is created (a 24-byte classic-pcap global header with no traffic — filter-dump attaches to the same netdev regardless of hostfwd) [VERIFIED: live qemu probe].

### The tap stanza on NixOS (HNET-03)

```
# Source: live probe on qemu 11.1.2 (-netdev tap with br= + helper= accepted; helper invoked)
-netdev tap,id=n0,br=br0,helper=/run/wrappers/bin/qemu-bridge-helper
-device virtio-net-pci-non-transitional,netdev=n0
```

Live evidence: `-netdev tap,id=x,br=zzz,helper=/nonexistent-helper-probe` on both the system and flake qemu 11.1.2 fails with `bridge helper failed` — the options are accepted and the helper is invoked at the given path (the failure IS the nonexistent path). The `bridge` backend accepts `br=`/`helper=` identically.

### The NixOS declarative remediation (D-10, doctor output target)

```nix
# Source: wiki.nixos.org Internet_Connection_Sharing + Networking (options verified against
# the wiki; NOT yet applied on this host — br0/ip_forward already exist, masquerade unknown)
networking.bridges.br0.interfaces = [ ];            # empty member list = pure NAT bridge
networking.interfaces.br0.ipv4.addresses = [ { address = "192.168.122.1"; prefixLength = 24; } ];
networking.nat.enable = true;
networking.nat.externalInterface = "wlp0s20f3";     # the default-route iface (wifi; probe it)
networking.nat.internalInterfaces = [ "br0" ];
boot.kernel.sysctl."net.ipv4.ip_forward" = 1;       # already 1 on this host [VERIFIED: live]
networking.firewall.trustedInterfaces = [ "br0" ];  # no LAN→br0 inbound stays the rule:
                                                    # trusted = forward br0↔host, external
                                                    # ingress still firewalled
environment.etc."qemu/bridge.conf".text = "allow br0";   # today's file says: allow virbr0
security.wrappers.qemu-bridge-helper = {                 # already present on this host via
  source = "${lib.getBin pkgs.qemu}/libexec/qemu-bridge-helper";   # the libvirt-era setup
  owner = "root"; setuid = true;                        # [VERIFIED: live wrapper]
};
```

Gotchas: NetworkManager can fight declarative bridges (NixOS discourse #15580 — this host runs networkmanager per the user's groups); the host is colmena-managed so the snippet lands in the colmena config, not /etc/nixos.

### Generic-Linux imperative equivalents (the documented-not-tested page)

```sh
ip link add br0 type bridge && ip link set br0 up
ip addr add 192.168.122.1/24 dev br0
sysctl -w net.ipv4.ip_forward=1
nft add table nat 2>/dev/null; nft add chain nat postrouting '{ type nat hook postrouting priority 100 ; }'
nft add rule nat postrouting oifname "<default-iface>" ip saddr 192.168.122.0/24 masquerade
chmod u+s /usr/libexec/qemu-bridge-helper     # distro path varies; the doctor checks
echo "allow br0" >> /etc/qemu/bridge.conf
```

### The darwin socket_vmnet manual page facts (HNET-04)

- Daemon: socket_vmnet runs as root (launchd or sudo), exposing vmnet over a UNIX socket — default `/var/run/socket_vmnet`; Homebrew: `${HOMEBREW_PREFIX}/var/run/socket_vmnet`; MacPorts: `/var/run/socket_vmnet`. [CITED: github.com/lima-vm/socket_vmnet README]
- Install: `brew install socket_vmnet` + `sudo brew services start socket_vmnet`; or `sudo make install.launchd`; or nix: `socket-vmnet` 1.2.2 exists in nixpkgs for aarch64-darwin [VERIFIED: nix eval]. Manual start: `sudo .../socket_vmnet --vmnet-gateway=192.168.105.1 <socket-path>` (the gateway is daemon-chosen — the darwin seam values would key off it if ever exercised).
- qemu side: `-netdev vmnet-socket,id=n0,path=<socket>` (qemu ≥ 9.0; sparse official docs — MEDIUM) or the README's fd-passing form via socket_vmnet_client + `-netdev socket,id=net0,fd=3` [CITED: README]. The flake's darwin qemu is nixpkgs-built and unsigned — builtin `vmnet-host/shared/bridged` backends would require running qemu itself as root (the entitlement `com.apple.vm.networking` is Apple-contract-restricted) [CITED: README]. Shared/NAT is the default mode; bridged is a separate daemon/socket (`socket_vmnet.bridged.en0`).
- Doctor on darwin: informational check that the socket path exists.

### rc burst shape (D-11b, in-guest)

```rc
# N-way burst: markers counted by the expect post-pass, never ordered
i=0
while(! ~ $i $burstn){
	tcpblast $tap_gwv4 $blastport $bustrounds &
	i=`{echo $i 1 + p | dc}
}
wait
echo netsoak: burst wait returned
```

(Exact rc dialect constraints from the 15-02 measured lessons apply: flat status ladders, no braced if-not bodies, `=`-free patterns. The marker count of `tcpblast: done` occurrences == $burstn is asserted host-side by the expect post-pass; `wait` blocks until all children exit [VERIFIED: sys/man/1/rc:736-743].)

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `-net nic,user` / `-netdev user` with no hostfwd | explicit `hostfwd=` rules bound to 127.0.0.1 per service | long-stable qemu | the hermetic default's surface; both D-04 ports live in one netdev |
| `-netdev tap,script=/etc/qemu-ifup` (root qemu) | setuid `qemu-bridge-helper` + ACL file + `tap,helper=` | qemu ~1.3 (HelperNetworking) | unprivileged bridged tap; the NixOS wrapper twist is the live divergence |
| builtin `vmnet-*` needing root qemu | `vmnet-socket,path=` + socket_vmnet daemon (qemu ≥ 9.0) | qemu 9.0 (2024) | darwin's only rootless route; documented-manual here |
| flat `[qemu.netdev]` (mode/device/pcap) | mode sub-tables + default key (this phase) | Phase 16 | the D-05 data shape; breaking TOML/struct migration in r9 |

**Deprecated/outdated:**
- `-net nic` legacy syntax — not used anywhere in this project's surfaces; keep it that way.
- The aarch64 apply_virt_netdev hardcoded emission — frozen-at-pin deliberately (OQ-2 resolved; do NOT migrate it in passing).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | qemu binds slirp hostfwd listeners eagerly at netdev init, so a host port <1024 aborts qemu startup (the kernel EPERM on bind is verified; the eager-bind timing is training knowledge) | Pitfall 3, D-04 | Low — remap is decided regardless; only the failure's exact shape changes |
| A2 | br0's existing 192.168.122.1/24 on the primary host is stable/declarative (colmena-managed; the colmena config itself was not readable) | Environment Availability | Medium — if br0 is a leftover manual artifact, a reboot drops it; the doctor's remediation covers recreation |
| A3 | The masquerade rule for br0's subnet is absent on this host (nft/iptables ruleset unreadable without root — no observation either way, NOT a verified absence) | Environment Availability, Pitfall 8 | Medium — if present, one doctor finding is pre-green; remediation text unchanged |
| A4 | `-netdev vmnet-socket,path=...` exact spelling on qemu 9+ (official docs sparse; corroborated by two secondary sources; not probeable on linux) | Code Examples (darwin) | Low — darwin leg is documented-manual; the fd-passing alternative is verified from the README |
| A5 | A host-std Rust bin builds cleanly under the r9 flake's existing toolchain+importCargoLock pattern (kernel-x64 is no_std; the emitter is the first host-std r9 package — pattern carried from devshell's cargo usage) | Pattern 1 | Low-Medium — may need a plain cargo build line instead of xtask buildflags; planner-visible |
| A6 | NixOS `networking.nat`/`networking.bridges` options render the intended nftables/iptables shape on this host's config style (options verified against wiki, not applied) | Code Examples (NixOS) | Low — the tested-on-primary-host leg of D-10 is exactly where this gets proven |
| A7 | The colmena-managed host accepts the documented environment.etc/security.wrappers additions unchanged (the wrapper + bridge.conf already exist via the libvirt-era setup — the exact colmena source of the existing wrapper was not located) | Pitfall 2, NixOS example | Low-Medium — docs may need a "wrapper already present via virtualisation.libvirtd" note |

## Open Questions (RESOLVED)

All four resolved at planning (2026-11-10); each resolution names the plan tasks that adopted it.

1. **Bridge subnet + seam values: adopt the host's existing br0/192.168.122.1/24 or define a fresh one?**
   - What we know: br0 exists at 192.168.122.1/24, empty, collision-free with 10.0.2.0/24 (slirp) and 192.168.86.0/24 (the host LAN); libvirt's conventional range means it's a well-understood private subnet.
   - What's unclear: whether the colmena config commits it (A2) and whether the planner prefers a project-distinct range.
   - Recommendation: adopt 192.168.122.0/24 (the D-08 "libvirt-default style" decision text names it; the host already carries it) — tap_guestv4=192.168.122.15, tap_maskv4=255.255.255.0, tap_gwv4=192.168.122.1 as seam values, with the doctor + docs pinning .1 as the host.
   - Resolution: RESOLVED — adopted 192.168.122.0/24 with host .1 / guest .15 as the tap_ seam values (16-02 Task 1); consumed by the 16-03 doctor's bridge-addr check and the 16-05 soak peer's bind.
2. **Which app carries the HNET-01 CI leg, and does the slirp-default boot get a smoke variant?**
   - What we know: no x64 composed gate runs in hosted CI today (zero grep hits for the x64 app names across .github/workflows); r9-shell.yml is the slirp-net CI home so far (aarch64 legs, R9_ACCEL=tcg implied by hosted-runner shape, 90m timeout precedent).
   - What's unclear: whether HNET-01's CI proof is a new x64 net-smoke CI job (running the existing boot-r9-net-smoke-x64 once its venue rides the emitter+hostfwd data) or a new minimal slirp-default leg.
   - Recommendation: a new job in r9-shell.yml running the x64 composed net gate with R9_ACCEL=tcg, asserting the hostfwd ports bind (the gate app exports them); tap/soak legs stay primary-host-only (impossible unprivileged).
   - Resolution: RESOLVED — the existing x64 composed net gate rides the emitter argv + hostfwd listener asserts (16-06 Task 1) and a new r9-shell.yml job net-smoke-x64 runs it at R9_ACCEL=tcg (16-06 Task 2); tap/soak legs stay primary-host-only.
3. **Does the doctor gate its exit code on root-gated findings (masquerade)?**
   - What we know: masquerade-rule reads need root; nft is not on PATH (iptables is).
   - What's unclear: D-14's "Exit 0/1 = CI-able preflight" — which findings are fatal vs WARN when unprivileged.
   - Recommendation: slirp checks all unprivileged-fatal; tap checks fatal except masquerade (WARN + exact sudo command) unless the doctor runs under sudo — make it explicit in the plan so the CI preflight semantics are deterministic.
   - Resolution: RESOLVED — exit 0 iff zero FAILs; every slirp check fatal, every tap check fatal EXCEPT the root-gated masquerade read which degrades to a loud WARN with the exact sudo command (16-03 Task 1).
4. **v6 on tap: trivially free or not?**
   - What we know: the seam anticipates v6 keys; no x64 gate speaks v6; NETIP-08 doesn't require it.
   - What's unclear: whether a v6 ping/echo over tap costs anything kernel-side.
   - Recommendation: leave to the plan's discretion clause (researcher-verified-free-only); do not let it grow.
   - Resolution: RESOLVED — evaluated NOT free to verify; v6 keys stay deferred to the phase that first needs them, and the seam's stale v6 note is re-keyed to record that verdict (16-02 Task 1). No v6 leg this phase.

## Environment Availability

Probed live on the NixOS linux-x86_64 primary host (2026-11-10). Read-only probes only — no bridges created, no config edited.

| Dependency | Required By | Available | Version / Value | Fallback |
|------------|------------|-----------|-----------------|----------|
| nix with flakes | everything | ✓ | 2.34.9 | — |
| qemu (flake nixpkgs) | all gates/apps | ✓ | 11.1.2 | — |
| qemu (system profile) | interactive boots | ✓ | 11.1.2 (qemu-host-cpu-only) | — |
| /dev/kvm | KVM venues | ✓ | crw-rw-rw- (world rw) | R9_ACCEL=tcg |
| /dev/net/tun | tap leg | ✓ | crw-rw-rw- | — |
| br0 bridge | tap/soak (HNET-03, NETIP-08) | ✓ EXISTS | 192.168.122.1/24, UP, no members, stp off, vlan_filtering 0 | create via documented NixOS config (D-10) |
| ip_forward sysctl | tap masquerade | ✓ | net.ipv4.ip_forward = 1 (already set) | — |
| ip_unprivileged_port_start | D-04 rationale | ✓ | 1024 (the default; verified) | — |
| setuid qemu-bridge-helper | tap leg | ✓ | /run/wrappers/bin/qemu-bridge-helper, `-r-s--x--x` | security.wrappers remediation |
| /etc/qemu/bridge.conf | tap leg ACL | ⚠ EXISTS, WRONG ALLOW | contains `allow virbr0` only (br0 denied — live-probed: "access denied by acl file") | environment.etc remediation (the doctor prints it) |
| masquerade rule for br0 | tap outbound | UNKNOWN | nft/iptables ruleset unreadable without root (A3 — no observation) | doctor WARN + remediation |
| hostfwd ports 17010/5670 | slirp default | ✓ FREE + bindable | both bind unprivileged (live-probed); no listener | — |
| blastport 7777 | soak echo peer | ✓ FREE + bindable | live-probed | — |
| ip / ss / sysctl / iptables | doctor checks | ✓ | on system PATH | add iproute2/nftables to app runtimeInputs |
| nft | doctor masquerade check | ✗ NOT on PATH | — | iptables (present) or runtimeInputs |
| expect | gate drivers | ✓ via flake runtimeInputs (not bare PATH) | — | — |
| r9 sibling checkout | emitter dev loop | ✓ | HEAD 5f856c9, clean tree; Harvey's flake.lock pins af52fee (2 commits behind HEAD) | — |
| darwin (socket_vmnet) | HNET-04 docs only | n/a (this host is linux) | nixpkgs socket-vmnet 1.2.2 aarch64-darwin verified to exist | documented-manual |

**Missing dependencies with no fallback:** none — every leg is executable on this host today except the darwin manual leg (by design) and the masquerade verification (root-gated, WARN-class).

**Missing dependencies with fallback:** nft (use iptables or add to runtimeInputs); bridge.conf allow-line (remediation is the phase's own docs).

## CI Reality (HNET-01)

- Workflow files: `.github/workflows/harvey.yml` (legacy build), `phase6.yml` (wave0-host, tier-r9 aarch64, tier-2 compile, tier-9k donor, x86_64-poolpanic 9k), `r9-shell.yml` (aarch64 boot-smoke + net-smoke — the slirp-net CI story) [VERIFIED: ls + grep].
- **No x64 composed gate leg exists in hosted CI today** — zero occurrences of boot-r9-x64/boot-r9-smoke-x64/boot-r9-tier1-x64/boot-r9-net-smoke-x64/boot-r9-services-x64 in .github/ [VERIFIED: grep exit 1]. HNET-01's "green in hosted CI" therefore requires a NEW leg; r9-shell.yml is the natural home (its header is literally "the GH Actions linux leg of the one-command boot", supply-chain-pinned actions, 90m TCG timeouts).
- Hosted runners are KVM-less and privilege-less: R9_ACCEL=tcg + cpu qemu64 (the Phase-13 CI discipline), slirp-only — tap/bridge legs are primary-host-only by construction (setuid helper + root config), matching D-13.
- Supply-chain rules already in force: actions pinned to full SHAs (r9-shell.yml header), checkout API-resolved.

## Security Domain

`security_enforcement` absent in .planning/config.json → enabled. This phase's security surface is host-privilege plumbing, not auth (Phase 18 owns the 567 listener itself).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | auth substrate is Phase 18; this phase only binds the hostfwd port remap (5670→567) |
| V3 Session Management | no | — |
| V4 Access Control | yes (host-side) | qemu-bridge-helper ACL file (`/etc/qemu/bridge.conf` allow-list) + setuid wrapper confinement; NAT bridge with NO LAN→br0 inbound (D-08); doctor read-only |
| V5 Input Validation | yes | `--net` mode names validated against the fixed set (usage error, no passthrough into shell); hostfwd/helper paths come from pinned TOML data, never user input |
| V6 Cryptography | no | none this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Unprivileged user pivoting tap/bridge to sniff LAN | Elevation/Information disclosure | setuid helper + ACL allow-list (only named bridges); NAT bridge has no external ingress; never run qemu as root |
| hostfwd exposed on 0.0.0.0 (guest services reachable from LAN) | Information disclosure / Spoofing | hostfwd bound to 127.0.0.1 explicitly in the TOML data (HNET-01 wording); the doctor/CI assert loopback-bound |
| bridge.conf allow-all (`allow all`) | Elevation | docs + doctor require the exact `allow br0` line; the helper's ACL is the control point |
| Retained pcaps carrying future auth traffic | Information disclosure | this era's captures carry soak traffic only; the Phase-18+ docs must revisit capture-retention policy when real keys flow (flag to Phase 18) |
| Store-path injection into qemu argv via env | Tampering | the emitter consumes only the pinned r9 TOML; boot script env vars are names/paths validated by the existing `-f/-x` contract checks (R9_ISO_BUILDER precedent at boot-r9-x64.sh:89-93) |

## Sources

### Primary (HIGH confidence)

- Live probes on the primary host (this session): br0/192.168.122.1/24; /etc/qemu/bridge.conf = `allow virbr0`; setuid wrapper + its ACL denial; ip_unprivileged_port_start=1024; ip_forward=1; unprivileged bind matrix (17010/5670/7777 OK, 567 EPERM); port-free checks; tool PATH audit; /dev/kvm, /dev/net/tun; nix 2.34.9; r9 HEAD/pin delta
- Live probes on qemu 11.1.2 (both system qemu-host-cpu-only and flake nixpkgs builds): `tap,br=,helper=` accepted + helper invoked; `bridge,br=,helper=` accepted; hostfwd×2 + filter-dump coexist unprivileged; compiled-in helper default = store libexec; no QEMU_BRIDGE_HELPER env string in the binary; `-netdev help` backend list
- Repo reads (Read tool, this session): ../r9/xtask/src/config.rs (Netdev struct :128-139, apply_netdev :479-492, tests :795+), ../r9/xtask/src/main.rs (:700-768, :1538-1699 call sites), ../r9/x86_64/lib/config_q35shell.toml (+4 sibling configs), ../r9/flake.nix (kernel-x64 :312-330, iso-builder export :369-441, FOD hash discipline), ../r9/xtask/Cargo.toml + workspace Cargo.toml, nix/boot-r9-x64.sh, flake.nix (mkScriptApp :26-39, x64 app gates :260-273), build/x86_64/root-tree/lib/netaddr.rc + netip-static.rc, build/x86_64/tcpblast.c + tcpecho.c, build/x86_64/mkimage.sh (net section :485-594, assert :512-519), nix/smoke-r9-net-x64.sh, build/x86_64/smoke-r9-net.expect, build/aarch64/root-tree/lib/netip-soak.rc, sys/man/1/rc (:736-743, :881-883), .github/workflows/{r9-shell.yml,phase6.yml,harvey.yml}, .planning/REQUIREMENTS.md, .planning/STATE.md, .planning/HOSTS.md
- nixpkgs registry eval: socket-vmnet 1.2.2, platforms ["aarch64-darwin"]

### Secondary (MEDIUM confidence)

- qemu.org system docs (net.html, invocation.html) — hostfwd semantics, netdev backends (some truncation; core claims cross-checked by live probes)
- qemu wiki Documentation/Networking — slirp/user networking
- wiki.nixos.org Internet_Connection_Sharing + Networking — networking.bridges/nat/sysctl/trustedInterfaces options
- nixpkgs/NixOS community sources (mynixos, discourse #15580, nixpkgs issue #516021) — security.wrappers qemu-bridge-helper shape, NetworkManager-bridge conflict, known wrapper build breakage class
- github.com/lima-vm/socket_vmnet README — daemon model, socket paths, install routes, entitlement restriction, modes
- nftables wiki Performing NAT — masquerade primitive

### Tertiary (LOW confidence)

- vmnet-socket `path=` exact option spelling (qemu 9+; sparse official docs; two secondary corroborations) — see A4
- Blog guides (adonis0147 qemu-socket-vmnet, dev.to macOS networking) — corroborating only

## Metadata

**Confidence breakdown:**
- Standard stack (qemu/nixpkgs surfaces): HIGH — nearly every load-bearing claim live-probed on the exact versions in play
- Architecture (emitter/flake/app shapes): HIGH — all shapes read from the repos verbatim; the lib-target refactor is a recommendation, not a verified build (A5)
- Pitfalls: HIGH — pitfalls 1-3 and 5 carry live or in-repo evidence; pitfall 4 is a recorded in-repo measurement; pitfalls 6-8 are structural consequences of verified facts

**Research date:** 2026-11-10
**Valid until:** 2026-12-10 (stable domain — qemu 11.1.2 and NixOS pin are the moving parts; re-verify if either flake pin advances)
