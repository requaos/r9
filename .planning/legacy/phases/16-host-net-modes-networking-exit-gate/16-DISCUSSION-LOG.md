# Phase 16: Host Net Modes + Networking Exit Gate - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-10
**Phase:** 16-Host Net Modes + Networking Exit Gate
**Areas discussed:** --net surface & failure UX, Netdev single-sourcing, Tap bridge topology, Soak gate + doctor checker

**Provenance:** the interactive AskUserQuestion pass was dismissed without answers; the harness instructed continuing with best judgment without recording invented user preferences. All selections below are therefore **agent recommendations (auto-selected, the workflow's `--auto` discipline)** — flagged [auto] in CONTEXT.md and legitimately re-litigable. Only the "Carried forward" items are genuine locked user decisions from prior phases.

---

## --net surface & failure UX

**Q1 — Where does the net-mode selector live?**

| Option | Description | Selected |
|--------|-------------|----------|
| Flag + env default | `--net=tap` parsed by boot-r9-x64.sh (roadmap's literal spelling); R9_NET env default (R9_ACCEL machine contract). Flag wins. | ✓ [auto] |
| Env var only (R9_NET) | Exactly the R9_ACCEL mechanism — but documented interface diverges from the roadmap's `--net=` spelling | |
| CLI flag only | Simplest surface; CI/gates must always pass the flag explicitly | |

**Q2 — Which apps gain mode selection?**

| Option | Description | Selected |
|--------|-------------|----------|
| Interactive boot app only | boot-r9-x64 gains --net; composed gates keep fixed slirp venues (deterministic bakes); tap soak gets its own app | ✓ [auto] |
| All composed net apps | net-smoke/services would need tap-substituted fixtures no requirement names | |
| Everything incl. r9 xtask profiles | Rejected as the D-07 shape (gates-split doctrine) | |

**Q3 — What happens when tap prereqs are missing?**

| Option | Description | Selected |
|--------|-------------|----------|
| Fail loud + embed the doctor's remediation | One doctor implementation invoked by the boot script's tap leg; names `nix run .#net-doctor` for the full report | ✓ [auto] |
| Fail loud + print the doctor command only | Least nesting, one extra manual step | |
| Auto-run doctor and exit | Functionally identical to the selected shape; selected form keeps the boot script's own step framing | |

**Q4 — hostfwd tcp/567 vs zero-privilege (surfaced during analysis: Linux refuses unprivileged binds <1024)**

| Option | Description | Selected |
|--------|-------------|----------|
| Host-side remap 5670→567 | Guest port stays 567 (service contract); host entry 127.0.0.1:5670; zero privilege + CI-ability hold | ✓ [auto] |
| Direct host:567 + sysctl | Requires ip_unprivileged_port_start system-wide change — weakens HNET-01's zero-privilege default | |
| Direct host:567 always | Requires CAP_NET_BIND_SERVICE — violates HNET-01 outright | |

## Netdev single-sourcing

**Q1 — Consumption mechanics (HNET-02)**

| Option | Description | Selected |
|--------|-------------|----------|
| Mode sub-tables in one TOML + r9-exported emitter bin | [qemu.netdev.{slirp,tap,vmnet}] + default in the booted config; emitter reuses xtask's structs/apply_netdev arg-building (ONE implementation, two front-ends); boot script substitutes its output | ✓ [auto] |
| Per-mode TOML config files | Mode-per-config doesn't reach Harvey's script without an emitter anyway (Harvey builds its own qemu cmdline) | |
| Harvey reads the TOML directly | Second TOML parser in shell — exactly the fork the requirement exists to kill | |

**Q2 — Where hostfwd/bridge params live**

| Option | Description | Selected |
|--------|-------------|----------|
| TOML owns qemu params; netaddr seam owns guest addressing | Hostfwd list + bridge names as TOML data (incl. the 5670 remap); tap_ leg keys join the seam; literal-grep assert extends | ✓ [auto] |
| All net params in one side or the other | Splits the single-source discipline either way | |

**Q3 — r9-side tap legs?**

| Option | Description | Selected |
|--------|-------------|----------|
| Composed-system only | NETIP-08 is a composed gate (14-D-06); r9 gets TOML + emitter + soak-surfaced fixes only | ✓ [auto] |
| xtask tap boot-test profiles too | Can join Phase 17 if a flush-out finding wants a kernel-level tap pin | |

## Tap bridge topology

**Q1 — Bridge shape**

| Option | Description | Selected |
|--------|-------------|----------|
| NAT'd private bridge (libvirt-style) | Fixed private subnet, host .1, masquerade out, no LAN inbound; self-contained on any host network; guest never externally exposed | ✓ [auto] |
| Bridge onto the host's real LAN | True external L2 reachability nothing in v2.1 needs; exposes the guest; per-network remediation variance | |

**Q2 — Guest addressing**

| Option | Description | Selected |
|--------|-------------|----------|
| Static via the seam | netip-static re-venues with tap_ values; /net/arp host-entry assert = the L2 proof | ✓ [auto] |
| DHCP from host dnsmasq | Privileged host moving part for zero gate value; DHCP already gated on slirp | |

**Q3 — Per-distro docs scope**

| Option | Description | Selected |
|--------|-------------|----------|
| NixOS (tested) + one generic-Linux page + darwin manual | HNET-03's minimal honest reading; demand names more pages | ✓ [auto] |
| Every major distro | Untested documentation rots; doctor covers the actual failure modes | |

## Soak gate + doctor checker

**Q1 — Soak shape (NETIP-08)**

| Option | Description | Selected |
|--------|-------------|----------|
| Tiered: baseline + burst + wedge/ARP scan | tcpblast at KVM-scaled rounds + 8 parallel instances + /net/arp truth assert + timeout wedge scan; numbers are committed gate data; green-twice; slirp refused outright | ✓ [auto] |
| Minimal single-connection soak | Meets the letter; under-exercises the multi-waiter/wedge classes the flush-out depends on surfacing | |
| Long single soak (10min) | Wall-clock-heavy without adding concurrency coverage | |

**Q2 — Host-side echo peer**

| Option | Description | Selected |
|--------|-------------|----------|
| Flake-built committed binary (tcpecho lineage) | Deterministic markers/byte-accounting owned by the gate; gate's own child process | ✓ [auto] |
| nixpkgs socat/ncat | External-tool drift; assertion machinery becomes tool-shaped | |

**Q3 — Where the tap gate lives**

| Option | Description | Selected |
|--------|-------------|----------|
| New dedicated flake app, primary host only | App-per-gate shape; doctor-preflight; hosted CI keeps slirp legs (bridges impossible unprivileged) | ✓ [auto] |
| A mode of net-smoke-x64 | Conflates the hermetic CI gate with the privileged truth gate — opposite venues | |

**Q4 — Doctor shape (HNET-05)**

| Option | Description | Selected |
|--------|-------------|----------|
| Read-only diagnose+print, one implementation two front-ends | Per-check PASS/FAIL + exact remediation (NixOS snippet first on NixOS); exit 0/1; shared with the boot tap leg | ✓ [auto] |
| Diagnose + --fix applying changes | Auto-sudo contradicts the fail-loud/print-remediation requirement text | |

---

## Claude's Discretion

Recorded in CONTEXT.md: emitter package name/shape, bridge subnet choice, doctor implementation language + check ordering, docs file layout, v6-seam inclusion (trivially-free only), soak app name + evidence layout + rc concurrency mechanics, whether any slirp machinery-proof leg re-runs.

## Deferred Ideas

Recorded in CONTEXT.md: v6 legs on tap (unless trivially free), real-LAN variant testing, DHCP-on-tap, 1:1 host:567 sysctl variant, r9-side tap boot-test profiles (→ Phase 17 if needed), distro pages beyond NixOS+generic, tap-mode bakes for non-soak gates.
