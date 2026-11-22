# Phase 16: Host Net Modes + Networking Exit Gate - Context

**Gathered:** 2026-11-10
**Status:** Ready for planning — **all decisions below were auto-selected (recommended) without interactive confirmation**; see the provenance note before `### Implementation decisions provenance`. Review and edit before `/gsd:plan-phase 16`, or re-run `/gsd-discuss-phase 16` to re-open any of them.

<domain>
## Phase Boundary

Host networking becomes an explicit, documented choice: the hermetic slirp+hostfwd default (zero privilege, CI-able), the `--net=slirp|tap|vmnet` explicit selector with no silent auto-downgrade, the netdev stanza single-sourced as data in r9's x86_64 platform TOML and consumed by Harvey's flake (healing the hardcoded `-netdev user` line `nix/boot-r9-x64.sh` carries today), the linux tap/bridge L2-truth leg with a doctor checker printing exact remediation, and the milestone's networking exit gate — NETIP-08's sustained bidirectional TCP soak executing ON the tap leg (slirp evidence explicitly refused). **Work spans both repos**: r9 carries the TOML mode tables + the exported netdev emitter (plus any kernel fix the soak surfaces, push-before-pin); Harvey carries the `--net` surface, the flake apps (soak gate + doctor), the tap venue fixtures in the netaddr seam, and the per-distro docs.

Addresses HNET-01, HNET-02, HNET-03, HNET-04, HNET-05, NETIP-08 (the tap-leg truth gate joining this phase per the v2.0 Phase-8 D-15 sequencing decision — the aarch64 machinery was built for exactly this).

NOT in this phase: kernel flush-out as such (Phase 17 — though soak-surfaced fixes feed it in-phase per the 15-D-01 fix-everything doctrine), auth substrate (Phase 18 — the tcp/567 hostfwd forwards to a listener that doesn't exist until then; binding the rule now is fine), cpu/drawterm (Phase 19 — the 17010 consumer), v6 tooling beyond trivially-free extensions (DHCPv6/SLAAC explicitly out), real-LAN bridge testing (documented variant only).

</domain>

<decisions>
## Implementation Decisions

### Implementation decisions provenance

The interactive question pass was dismissed without answers; per the harness contract every decision below was selected by the agent as its recommendation (the workflow's `--auto` discipline: recommended option, logged selection, no AskUserQuestion). **None of these is a recorded user preference.** Every D-NN is flagged [auto] and is legitimately re-litigable — edit this file directly or re-run `/gsd-discuss-phase 16` before planning. Where a decision merely applies an already-locked prior-phase discipline, it says "carried" and is NOT re-litigable.

### `--net` surface & failure UX (HNET-01, HNET-02)
- **D-01 [auto]: Flag + env default.** `boot-r9-x64.sh` parses `--net=slirp|tap|vmnet` from its own args (`nix run .#boot-r9-x64 -- --net=tap` — the roadmap's literal interface spelling); `R9_NET` sets the default when no flag is passed (the R9_ACCEL machine-contract precedent — CI/scripts get an env, humans get a flag). Flag wins over env; default `R9_NET=slirp`. No silent auto-downgrade, ever: an unknown mode is a usage error listing the valid modes; a cross-venue request (`--net=vmnet` on linux, `--net=tap` on darwin — darwin's tap is not this phase's leg) is rejected at parse naming the venue's valid modes, never left to fail obscurely inside qemu. — **Reversibility:** reversible — a script-local surface; widening or renaming touches only the one boot script and its docs.
- **D-02 [auto]: Mode selection rides the interactive boot app only.** `boot-r9-x64` gains `--net`; the composed gates (net-smoke-x64, services-x64, tier gates) keep fixed slirp venues — their bakes substitute `slirp_` seam values and must stay deterministic; a tap-mode services bake would need tap-substituted fixtures no requirement names. The tap soak gate is its own dedicated app (D-11). — **Reversibility:** reversible.
- **D-03 [auto]: A missing-prereq tap boot fails loud WITH the doctor's remediation embedded.** The boot script's tap leg invokes the same check implementation the doctor app runs (ONE implementation — D-12's library consumed two ways), prints the per-issue remediation blocks inline in the failure, and names `nix run .#net-doctor` for the full report. Never auto-fixes. — **Reversibility:** reversible.
- **D-04 [auto]: hostfwd tcp/567 is host-side-remapped to 5670.** The slirp default's hostfwd set is `tcp:127.0.0.1:17010-:17010` and `tcp:127.0.0.1:5670-:567` — guest ports stay 17010/567 (the in-guest service contracts Phases 18/19 own), but the HOST entry point for auth is 127.0.0.1:5670. Rationale: HNET-01's "zero privilege; CI-able" is the hard constraint and Linux refuses unprivileged binds below 1024 (`ip_unprivileged_port_start` defaults to 1024) — a literal host:567 mapping would make the hermetic default require a sysctl. The 1:1 alternative is documented in the docs as the opt-in sysctl variant. The remap is data in the TOML slirp stanza (D-05), never script text. — **Reversibility:** costly — Phase 18/19 documentation and any drawterm-dial examples will cite the host port; changing it later re-edits every consumer reference.

### Netdev single-sourcing (HNET-02)
- **D-05 [auto]: Mode sub-tables in the ONE platform TOML + r9 exports a netdev-args emitter.** The x86_64 platform TOML's `[qemu.netdev]` grows per-mode tables — `[qemu.netdev.slirp]` (mode/device/hostfwd list incl. the D-04 remap/pcap), `[qemu.netdev.tap]` (mode/device/bridge/pcap), `[qemu.netdev.vmnet]` (darwin-only, documented-manual) — plus a default key, all in the config the flake boots (q35shell lineage). r9's flake exports a small emitter package: a bin reusing the xtask config structs with `apply_netdev`'s arg-building refactored into shared code — ONE emission implementation, two front-ends (in-process xtask, exported CLI). `boot-r9-x64.sh` replaces its hardcoded `-netdev user,id=n0 -device virtio-net-pci-non-transitional,netdev=n0` line with command substitution of the emitter's output for the selected mode; the flake app exports the emitter path env-style (`R9_ISO_BUILDER` precedent). — **Reversibility:** costly — the emitter is a cross-repo flake API surface (r9 package consumed by Harvey's flake); reshaping it later re-touches both lockfiles and both call sites.
- **D-06 [auto]: Host-qemu params live in the TOML; guest-side addressing lives in the netaddr seam.** The TOML owns everything qemu sees (netdev/device/hostfwd/bridge names); the seam (`build/x86_64/root-tree/lib/netaddr.rc` — which already anticipates this phase by name) gains the `tap_` leg keys (tap_guestv4/mask/gw from the bridge subnet; `blastport=7777` reused). The bake-time venue-literal grep assert extends to the tap values — one seam, mechanically asserted, no tap literal anywhere else. — **Reversibility:** reversible — additive keys in two data files.
- **D-07 [auto]: Tap legs are composed-system (Harvey-side) only.** r9's work this phase = the TOML mode tables + the emitter export (+ soak-surfaced kernel fixes under the 15-D-01 fix-everything doctrine, push-before-pin, FOD re-wire-then-pin after every kernel-byte fix pass — carried). NO new r9-side tap boot-test profiles: NETIP-08 is a composed gate per the 14-D-06 gates-split; kernel-level /net evidence stays on the existing profiles. — **Reversibility:** reversible — a r9-side tap profile can always join Phase 17 if a flush-out finding wants a kernel-level tap pin.

### Tap bridge topology (HNET-03, HNET-04)
- **D-08 [auto]: NAT'd private bridge (libvirt-default style).** br0 with a fixed private subnet (planner picks — must NOT collide with slirp's 10.0.2.0/24), host owns the bridge's .1, masquerade out the default route, NO LAN→br0 inbound. Self-contained on any host network (the NixOS primary host included, wifi/VPN hosts too), deterministic subnet drives the seam, and the guest is never exposed to external networks — drawterm connects from the host itself in Phase 19; nothing in v2.1 needs external reachability. Real-LAN bridging ships as ONE documented variant paragraph in the docs, explicitly not tested. — **Reversibility:** reversible — host-side config + seam values; switching topology re-edits the doctor's remediation text and the seam, nothing structural.
- **D-09 [auto]: Static guest addressing via the seam; DHCP-on-tap is a documented non-goal.** The netip-static fixture re-venues (ipconfig ether bind → add → `-g` default route, all through the real devip tree) with `tap_` seam values; `/net/arp` gains the host-entry assertion post-flow (the L2 proof this venue exists for). DHCP fidelity is already gated on slirp (14-09); tap DHCP would add dnsmasq as a privileged host moving part for zero gate value. The seam's v6 note (v6 keys "join with Phase 16's NETIP legs") is honored only as a researcher-verified trivially-free extension (e.g. a v6 ping/echo on tap) — v6 is NOT a NETIP-08 criterion and joins only if it costs nothing. — **Reversibility:** reversible.
- **D-10 [auto]: Per-distro docs = NixOS (tested) + one generic-Linux reference (documented) + darwin socket_vmnet manual.** NixOS gets the full declarative path (networking.bridges, ip_forward, masquerade/trustedInterfaces, the qemu bridge-helper wrapper + `/etc/qemu/bridge.conf` `allow br0` via environment.etc) proven on the primary host; generic Linux gets one page of equivalent imperative commands (ip link/addr, sysctl, nftables/iptables, setuid helper + bridge.conf) — documented-not-tested; darwin stays HNET-04's opt-in documented-manual (builtin vmnet is entitlement-walled on nixpkgs qemu). Not per-every-distro — demand names more pages later. Companion README follows the `build/aarch64/README.md` drift-notes discipline. — **Reversibility:** reversible.

### Soak gate + doctor checker (HNET-05, NETIP-08)
- **D-11 [auto]: Tiered soak on the existing tcpblast, refusing slirp outright.** (a) baseline: tcpblast against the host-side echo peer at a raised, KVM-scaled round count (the committed default 64 is TCG-sane; the tap leg's count is gate data, planner-tunable); (b) burst: N parallel tcpblast instances (8) backgrounded from the sweep rc, each asserting its own completion marker; (c) wedge scan: the gate timeout catches stalls loudly (the netblast/tcpblast no-pacing posture) + the `/net/arp` host-entry truth assert (D-09). Exact numbers are committed data in the gate harness (single-source-as-data). Green-twice on the primary host; the app HARD-REFUSES a slirp run ("slirp evidence not accepted" — the refusal is the venue-honesty control, mechanically). — **Reversibility:** reversible — thresholds are data; the tier shape is the gate's own script.
- **D-12 [auto]: The host-side echo peer is a flake-built committed binary (tcpecho lineage), not socat/ncat.** A tiny host-compiled echo server listening on `blastport` bound to the bridge IP — deterministic markers and byte-accounting owned by the gate, no external-tool behavior drift, same echo semantics the in-guest tcpecho proves. It runs as the gate app's own child (no daemon, no systemd, dies with the gate). The doctor checks the port free. — **Reversibility:** reversible.
- **D-13 [auto]: The soak gate is a new dedicated flake app in the app-per-gate shape** (name planner discretion, `boot-r9-net-soak-x64`-style): doctor-preflight → bake the net image (mkimage's net section + tcpblast, unchanged 14-09 contract) → compose ISO → start the host echo peer → boot `--net=tap` through the emitter → drive the in-guest sweep via expect → assert completion + ARP + captures retained → green-twice → evidence consolidated in the phase dir with the r9 SHA. Runs on the primary host only (needs br0); hosted CI keeps the slirp legs (HNET-01 covers CI — bridges are impossible unprivileged). — **Reversibility:** reversible — an additive flake app.
- **D-14 [auto]: The doctor is read-only diagnose+print, never fixes, and is one implementation consumed two ways.** Flake app `nix run .#net-doctor` (+ invoked inline by the boot tap leg per D-03 — shared code, two front-ends). Checks per mode: slirp (qemu present, hostfwd ports free — trivial), tap (br0 exists+up, bridge IP present, ip_forward=1, masquerade rule present, qemu-bridge-helper executable/setuid + bridge.conf `allow br0`, blastport free), vmnet (darwin: socket_vmnet present — informational). Output: per-check PASS/FAIL blocks, every FAIL printing the exact remediation — NixOS declarative snippet FIRST on NixOS, generic commands otherwise. Exit 0/1 = CI-able preflight. — **Reversibility:** reversible.

### Carried forward (locked in prior phases — do not re-litigate)
- Netdev-as-data: `[qemu.netdev]` TOML consumed by the generalized `apply_netdev` emitter, q35-gated (Phase 13, OQ-2 resolved); aarch64 stays frozen-at-pin.
- Gates split by artifact (14-D-06): kernel-level r9-side, composed-system Harvey-side.
- Venue honesty: slirp never L2 truth (asserted NOT-L2-TRUTH stage in slirp captures); named venues; no unearned OK.
- R9_ACCEL precedent semantics: explicit modes, no auto, fail-loud naming fallbacks (D-01 applies them to net).
- push-before-pin; pin advance is its own commit + fresh hosted CI; FOD hash re-stales per kernel commit — re-wire-then-pin after every r9-byte fix pass.
- The netaddr single-seam discipline (14-09 D-11): no venue address literal outside `netaddr.rc`, mechanically asserted by the bake gate.
- The 14-09 net payloads ride unchanged: tcpblast (the soak client — round-count argv[3], per-round + completion markers, wedge-loud posture), tcpecho, ipconfig; sourced-not-typed in-guest sweeps; serial markers; presence-based timing-tolerant assertions.
- NETIP-08 sequencing (v2.0 08-D-15): machinery + slirp proof were Phase 8's (done, aarch64); the tap-leg truth verdict is THIS phase's joint acceptance gate using that machinery.

### Claude's Discretion
- The emitter package's exact name/shape in r9's flake (a bin crate or an xtask subcommand wrapped — the constraint is ONE shared arg-building implementation with in-process xtask).
- The private bridge subnet choice (collision-free with 10.0.2.0/24 and common LAN ranges).
- The doctor's implementation language/file home (bash in `nix/` mirroring the script-app shape is the obvious default) and check ordering.
- Per-distro docs file layout (companion README beside `build/x86_64/` per the aarch64 precedent).
- The v6 seam keys' inclusion (researcher-verified trivially-free only — D-09's constraint).
- The soak app's name, capture/evidence file layout, and the concurrency mechanics in the rc sweep (backgrounding + marker collection).
- Whether the slirp machinery-proof leg re-runs anywhere this phase (not required — the XEXEC-05 gates already cover slirp; the soak app's refusal is the venue control).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 16 — goal, success criteria 1–5, requirement mapping; §Phase 17 (the parallel flush-out consumer of soak findings), §18/§19 (the 567/17010 hostfwd consumers)
- `.planning/REQUIREMENTS.md` — HNET-01..HNET-05 + NETIP-08 full text; Out of Scope table (slirp-not-a-correctness-target, DHCPv6/SLAAC rows)
- `.planning/PROJECT.md` — Key Decisions (netdev-as-data, gates-split, venue honesty, push-before-pin), Constraints
- `.planning/STATE.md` — Accumulated Context (13's netdev-table decision, 14-09's net gates)
- `.planning/HOSTS.md` — the NixOS linux-x86_64 primary host specifics D-10's docs target

### Prior-phase context (the machinery this phase rides)
- `.planning/phases/13-r9-x86-64-platform-bring-up/13-CONTEXT.md` — D-01a (netdev stanza in TOML), the platforms-as-data doctrine
- `.planning/phases/14-harvey-amd64-userspace-on-r9/14-CONTEXT.md` — D-06 (gates split), D-07 (in-guest expect-driven net gates)
- `.planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-CONTEXT.md` — D-01 (fix-everything boundary), D-16 (ndb template/netaddr seam interplay)
- `.planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-CONTEXT.md` — D-12 (gate binaries as harness payloads), D-15 (NETIP-08 tap-verdict sequencing INTO this phase) — the machinery origin

### r9 repo (sibling, `../r9`)
- `../r9/xtask/src/config.rs` — `apply_netdev` (the arg-building D-05 refactors into shared code) + the netdev config structs
- `../r9/x86_64/lib/config_q35shell.toml` (and the config_q35* family) — the `[qemu.netdev]` shape D-05 extends with mode sub-tables
- `../r9/flake.nix` — the `packages.iso-builder` export shape the emitter package mirrors

### Harvey repo — the surfaces this phase changes
- `nix/boot-r9-x64.sh` — the hardcoded `-netdev user` line D-05 heals; the R9_* env contract D-01's R9_NET joins; the R9_ACCEL fail-loud precedent
- `flake.nix` — the app-per-gate shape (mkScriptApp + R9_* exports) D-13's soak app and D-14's doctor join
- `build/x86_64/root-tree/lib/netaddr.rc` — the single addressing seam (names THIS phase in its comments); D-06's tap_ keys land here
- `build/x86_64/root-tree/lib/netip-static.rc` — the fixture D-09 re-venues
- `build/x86_64/tcpblast.c` — THE soak client (round-count argv, marker vocabulary, wedge-loud posture) D-11 re-runs on tap
- `build/x86_64/tcpecho.c` — the echo semantics D-12's host peer mirrors host-side
- `nix/smoke-r9-net-x64.sh` + `build/x86_64/smoke-r9-net.expect` — the composed net gate shape the soak app extends (its own app, same pipeline bones)
- `build/x86_64/mkimage.sh` — the net bake section (unchanged contract, consumed by the soak bake)
- `build/aarch64/README.md` — the drift-notes/docs discipline D-10's companion follows

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- The whole 14-09 net payload set — tcpblast/tcpecho/svctcpecho + the netip-* fixtures + netaddr.rc — rides unchanged; the soak is a re-run at scale on a new venue, not new machinery
- r9's netdev TOML→argv pipeline (`apply_netdev`) already emits per-TOML netdev args for xtask boots — D-05 refactors it for a second front-end rather than inventing a format
- The flake's mkScriptApp + R9_* env-contract pattern — the soak and doctor apps are mechanical additions
- The R9_ACCEL explicit-mode plumbing in boot-r9-x64.sh — the template R9_NET/--net copies

### Established Patterns
- Single-source-as-data everywhere: TOML for qemu params, seam for guest addresses, gate data files for thresholds
- One implementation, two front-ends (the emitter shared by xtask and the exported CLI; the doctor shared by its app and the boot failure path)
- Fail-loud + no silent downgrade (R9_ACCEL → R9_NET; the soak app's slirp refusal)
- Venue honesty with asserted labels; evidence = serial markers + retained captures, green-twice, r9 SHA recorded

### Integration Points
- `../r9/x86_64/lib/config_q35shell.toml` `[qemu.netdev]` — grows mode sub-tables (r9 commit; push-before-pin)
- `../r9/flake.nix` — the exported emitter package (r9 commit)
- `nix/boot-r9-x64.sh` — flag parsing + emitter substitution + doctor-on-tap-failure
- `flake.nix` — two new apps (soak, doctor)
- `build/x86_64/root-tree/lib/netaddr.rc` — tap_ leg keys; the bake literal-grep assert extends
- Host: br0 + masquerade + qemu-bridge-helper/bridge.conf — documented NixOS config, doctor-verified

</code_context>

<specifics>
## Specific Ideas

- **The acceptance story, end-to-end:** the TOML grows `[qemu.netdev.{slirp,tap,vmnet}]` + default, and r9 exports the emitter (push-before-pin; pin advance own commit) → `boot-r9-x64.sh` heals its hardcoded line into `$(r9-netdev-args --mode "$MODE")`, gains `--net`/R9_NET with usage-error listing, and the D-04 hostfwd set (17010 + 5670→567) lands as slirp-stanza data → the slirp default re-proves hermetic + zero-privilege + hosted-CI green (HNET-01) → the primary host gets br0 (NixOS declarative, D-08) and `nix run .#net-doctor` walks every check green printing remediations for each failure class (HNET-05) → a `--net=tap` boot with prereqs missing fails loud WITH the doctor's remediation blocks embedded (D-03) → netaddr.rc grows the tap_ leg; netip-static re-venues; the literal-grep assert covers the tap values (D-06/D-09) → the soak app runs: doctor-preflight → bake → compose → host echo peer up on the bridge IP → `--net=tap` boot → baseline tcpblast at KVM-scaled rounds + 8-way burst + ARP-truth assert + wedge scan, green twice with captures retained (NETIP-08; slirp runs refused outright) → the docs land (NixOS tested + generic + darwin manual, HNET-03/04) → evidence + r9 SHA consolidated; any soak-surfaced kernel fix lands push-before-pin under the 15-D-01 doctrine.
- **Watch-out:** the emitter export changes r9's flake API surface — the FOD/hash discipline means the Harvey-side re-wire-then-pin happens after the r9 commit lands, not interleaved (the phase-14 lesson).
- **Watch-out:** host port 567 cannot be bound unprivileged on default Linux — D-04 exists precisely so HNET-01's "zero privilege" survives contact with `ip_unprivileged_port_start`; do not "fix" the remap back to 1:1 without revisiting CI-ability.
- **Watch-out:** slirp's DHCP + static values are baked via the seam — the tap venue must NOT perturb the existing slirp gates' fixtures (additive keys only, last-wins discipline respected).
- **Watch-out:** qemu-bridge-helper on NixOS is wrapper-dependent (setuid path + `/etc/qemu/bridge.conf`) — the doctor must check the NixOS-shaped reality, not the generic-Linux setuid assumption; this is the #1 per-distro divergence the docs must get right.
- **Watch-out:** the burst leg's 8 backgrounded tcpblasts all marker-print on one serial console — the expect assertions must be presence-based per-marker-count, never interleaving-order (the carried gate discipline).

</specifics>

<deferred>
## Deferred Ideas

- **v6 legs on tap** (the seam's anticipated v6 keys, re-keyed from the aarch64 probe findings) — joins only as a researcher-verified trivially-free extension; NOT a NETIP-08 criterion (D-09's constraint). If not free, the phase that first needs v6 on x64 picks it up.
- **Real-LAN bridge variant** — documented paragraph only this phase (D-08); testing it is a future demand-named item.
- **DHCP-on-tap** (dnsmasq on br0) — documented non-goal (D-09); DHCP fidelity stays gated on slirp.
- **1:1 host:567 via sysctl** — the opt-in variant documented alongside D-04's default; not the gate's shape.
- **r9-side tap boot-test profiles** (kernel-level tap evidence) — can join Phase 17's flush-out if a soak finding wants a kernel-level pin (D-07).
- **Per-distro pages beyond NixOS + generic Linux** — demand-named later (D-10).
- **A tap-mode services/gates bake** (tap-substituted fixtures for non-soak gates) — no requirement names it (D-02); revisit when one does.

</deferred>

---

*Phase: 16-Host Net Modes + Networking Exit Gate*
*Context gathered: 2026-11-10*
