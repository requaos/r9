# Phase 08 Evidence Index — r9 TCP/IP Path: devip-Compatible /net

**Phase:** 08-r9-tcp-ip-path-devip-compatible-net
**Consolidated:** 2026-11-06 (plan 08-09)
**Status:** complete — every gate mapped to the r9 SHA it ran at. The phase delivers a devip-exact `/net` tree (`#I`) in the r9 kernel over smoltcp 0.14.0: the D-05 event-driven seam (RX-IRQ wake + `poll_at` one-shot deadline + file-op drive; socket wakers), loopback + null mediums first, the ethermedium bound to Phase 7's devether, static + DHCP IPv4 via the **unmodified** Harvey `ip/ipconfig`, full-fidelity ether-leg IPv6, `/net/arp` as a snoop-fed view, `/net/log` + flag-guarded prints, the D-17 minimal CSPRNG, aan drop-and-resume on loopback, and the tcpblast bidirectional soak machinery proven on slirp — every slirp-green gate labeled **NOT-L2-TRUTH**; the tap-leg truth verdict is Phase 9's joint acceptance gate (D-15).

## Pinned commits (evidence ordering: r9 pushed BEFORE the pin; the pin records the gated HEAD)

| Repo | SHA | Role |
|------|-----|------|
| r9 (`requaos/r9`, branch `main`) | **`19f61ed133d96fcdbfdfc5049ab2168beae512b3`** | the phase-complete HEAD — pushed to origin 2026-11-06 (`a5b1b42..19f61ed`, 30 commits: 08-01..08-08's 28 + 08-09's `14b2f23` + `19f61ed`) BEFORE locking (push-before-pin, T-08-18). Verified: `git -C ../r9 ls-remote origin main` names this rev post-push. |
| harvey flake.lock | pins r9 rev `19f61ed133d96fcdbfdfc5049ab2168beae512b3` (narHash `sha256-psrO31a2NWiEWGyKKdydXe0R/73ULGK/u3furd6oetQ=`, owner `requaos`) | **the pin** — locked rev == live r9 HEAD == origin/main; resolves from a clean clone. |
| harvey | `d8080f115` | **the pin commit** (08-09 Task 3) — flake.lock ALONE (the `git show --stat` gate: 1 file, 3 insertions, 3 deletions). |
| harvey | `174f7bc30` | 08-09 Task 1: the composed net smoke (`build/aarch64/smoke-r9-net.expect` + `nix/smoke-r9-net.sh` + the `boot-r9-net-smoke` flake app) + retained green captures. |
| harvey | `ef9b89c5c` | 08-09 Task 2: the `net-smoke` job in `.github/workflows/r9-shell.yml` (D-16 short gates; checkout SHA-pinned per the 06-08 rule). |
| r9 pre-existing state | — | `aarch64/src/exec.rs` carries a PRE-EXISTING uncommitted modification and one stash entry — never touched/staged/committed all phase; the file was present in the working tree for every gate run below (the phase's recorded discipline). |

## Gate → commit mapping

### 08-01 — the devip contract doc + the smoltcp supply chain — r9 `669c561` (HEAD at plan close)

- `docs/devip-contract.md` (747 lines): the FULL documented devip surface enumerated from `sys/man/3/ip` × `sys/src/9k/ip/` with every divergence flagged [man-vs-src] and the r9 mapping decisions recorded with their math (D-03/D-08). Lands FIRST — the phase's cost center reviewed before gate-writing (r9 `8330700`).
- smoltcp 0.14.0 enters as port/'s first external crate: crates.io pin + committed `Cargo.lock` (D-02); `cfg(dev_ip)` predeclared in the workspace check-cfg; the joint NETIP-01/virtio provenance record (D-01/D-04) — r9 `669c561`.
- Verify legs green: `cargo xtask check`/`clippy` exit 0, the virt boot-test green, the nix vendored build green.

### 08-02 — the D-05 event-driven seam + host pins — r9 `63a3ab7` (HEAD at plan close)

- `port/src/devip/stack.rs` (the BusyCell single-ownership Interface+SocketSet+adapter, the three drive legs), `waker.rs` (the RawWaker bridge — the phase's ONE new unsafe site, #[path]-mounted outside the forbidden devip subtree), `phyadapt.rs` (the fixed-pool LoopbackDev), `devip/` module shell — r9 `ae4d82a`.
- Host pins land FIRST (RED `ee65bfc` → GREEN `c28e6f4` + the AcceptPool): waker bridge fires on RX/close/buffer-space; register→wake→re-register lost-wakeup shape; deadline arming from `poll_at`; accept-pool demux; the stack-debt drain; and the **OQ-1 DHCP-shaped exchange pin** — OQ-1 verdict (recorded in contract §12.6): **PASS-path via explicit `UdpMetadata.local_address`** — the udp socket sources 0.0.0.0 on the wire ONLY when the send metadata carries it; the naive shape is silently dropped (`get_source_address` → None on an address-less interface); no wire-layer shim; the D-09 smoltcp-dhcpv4 fallback stays unwired (r9 `63a3ab7`).

### 08-03 — the Harvey gate payloads + the D-11 addressing seam — harvey `84fb37ab0` + `499b4f00e` (r9 unchanged at `63a3ab7`)

- Six gate binaries build for aarch64 via phase-tagged insertion-only build blocks (D-12 — NOT tier-allowlist appends): `ipconfig`, `ping`, `udpecho`, `aan` from the full tree + new payloads `tcpblast`/`tcpecho`; all harvested to `out/`, magic-asserted (`00008c47`), baked with manifest asserts.
- `build/aarch64/netaddr.rc`: the single committed addressing seam — leg-prefixed keys (`lo_`/`slirp_`), sourced by every gate fixture and read by the r9 xtask profiles through `R9_HARVEY_DIR`; no scattered `10.0.2.x` (Pitfall 9). The v6 pair recorded provisional until 08-07's probe finalized it.

### 08-04 — the TRACER: #I tree + loopback + ping/udp echo — r9 `9de69c7` (HEAD at plan close)

- `port/src/devip.rs` + `conv.rs`/`medium.rs`/`ipifc.rs`/`proto.rs` + the icmp/udp protos, wired through namec/syscall/main/timer (the D-05 seam's kernel legs) + the `virtip` profile — r9 `fdff261`; udp module — r9 `9cc6482` (RED) + `9de69c7` (GREEN).
- **The blocking-human RFMEM decision (option A, user 2026-11-05):** RFMEM landed with the donor dupseg semantics EXTENDED to growth (the checkpoint's "ping does not brk_-grow post-fork" premise falsified at the gate) — RFMEM families carry the shared break; SEMACQUIRE(37)/SEMRELEASE(38) implemented (no network syscalls; NETIP-02 intact) — r9 `4298fc0`.
- Gate (NETIP-03 legs 1-2): `R9_NETIP_SWEEP=loopback cargo xtask boot-test --arch aarch64 --config virtip` PASS — `ping 127.0.0.1` replies + the udp echo round-trip over the real `/net` file tree (the fd-4-held conv discipline), NIC-free.
- **Venue: loopback, NIC-free — no L2 claim.**

### 08-05 — the tcp proto module + observability + tcp/7 echo — r9 `560efd1` (HEAD at plan close)

- tcp module: the accept pool (128 listeners = devip-exact Maxincall, contract §12.3), the blocking connect bridge (probe → drive → re-probe — the gate-caught wakeup-loss fix `6528d5b`), listen/err files, close/hangup/hangupxmit mapping (FIN after the TX buffer's tail), the 28-tag bridge-maintained stats — r9 `2b647d4` (RED pins) + `2df471b` (GREEN).
- Observability (D-07): `/net/log` with the DONOR's Log* bit vocabulary (the 08-04 placeholder bits replaced) + flag-guarded prints; the D-13 netlog-activity marker is the ctl-acceptance print `netlog: flags set <mask>`. The remaining top-level surface: `/net/iproute` (longest-match + the smoltcp default-route mirror), live `/net/ipselftab` + `/net/ndb` — r9 `560efd1`.
- Gate (NETIP-03 leg 3): `R9_NETIP_SWEEP=tcp … --config virtip` PASS — tcpecho (unmodified libc announce/listen/accept) answers the rc client's connect/write/read through /net/tcp; the stats-honesty leg proves the counters move; the file-surface legs green.
- **Venue: loopback, NIC-free — no L2 claim.**

### 08-06 — the D-17 CSPRNG + the ethermedium + the static gate — r9 `d170850` (HEAD at plan close)

- `/dev/random` + `/dev/urandom`: the minimal DTB-seeded ChaCha20 DRBG in devcons (pure safe Rust; the forbid holds), seeded lazily at first read from `/chosen/rng-seed`, jitter-mixed per read — ipconfig's unconditional `truerand()` resolves — r9 `945e5da`.
- The ethermedium: `bind ether <path>` brings up the EtherDev adapter over devether's netif convs 0x800/0x806/0x86dd (arrival-order merge via the new global ingress seq stamps), smoltcp `Medium::Ethernet` (MTU 1514 header-inclusive, software checksums), the two-core seam (per-conv CoreSel + the address-class demux), the RX-IRQ WAKE leg (pci.rs → note_rx_pending — never a poll), the snoop-fed `/net/arp` view (90-byte lines, NCACHE=256 LRU, flush/add/del), the gratuitous ARP on every ether address add — r9 `8f753ca` + the boundary backstop pin `d170850`.
- Gate (NETIP-04): `R9_NETIP_SWEEP=static … --config virtip` PASS — the REAL unmodified `ip/ipconfig -g <gw> ether /net/ether0 add <guest> <mask>` (the seam's values) configures the guest; ping the gateway replies; `/net/arp` carries the gateway entry; the pcap carries the gratuitous ARP + the ICMP exchange — **log labeled `netip: NOT-L2-TRUTH slirp venue` (D-10; the tap verdict is Phase 9's).**

### 08-07 — the DHCP + IPv6 gates — r9 `89ab75f` (HEAD at plan close)

- The OQ-1 bridge rule lands at the file surface (udp_write_data sources 0.0.0.0 exactly in the absorbed-DHCP window — keyed on the self-table's accept-all flag, the `add 0.0.0.0` absorption marker; outside the window smoltcp's own source selection governs) + host pins — r9 `37e77fb`; the icmpv6 write's short-drop bound fix — r9 `89ab75f`.
- Gate (NETIP-05): `R9_NETIP_SWEEP=dhcp … --config virtip` PASS — the unmodified ipconfig dhcpclient's DORA over `/net/udp` broadcast fidelity: the pcap carries broadcast src-0.0.0.0:68 → 255.255.255.255:67 Discover/Request (guest MAC as L2 src) and Offer/AK from the seam gateway; the lease (10.0.2.15) ties to the seam; the guest configures (selftab/route/ndb) and pings the gateway. The smoltcp dhcpv4 socket NEVER wired (D-09). **NOT-L2-TRUTH labeled.**
- Gate (NETIP-06): `R9_NETIP_SWEEP=v6 … --config virtip` PASS — the devip ip6 surface (the icmpv6 full-packet bridge — parse-on-write/synthesize-on-read with the write-side ICMP_USEAD6 prefix; ipifc add6/ra6/try with the tentative DAD lifecycle, never pushed to smoltcp until confirmed) configures the guest via unmodified ipconfig; the NS/NA exchange + 3/3 ICMPv6 echo replies with the gateway are pcap-proven — **NOT-L2-TRUTH labeled.**
- **The slirp v6 probe verdict (2026-11-06, the probe-then-pin discipline):** slirp answers ND for `fec0::2` (the guest's NS got an NA) AND serves ICMPv6 echo there (3/3 replies); NO unsolicited RAs observed in the gate window; the guest's one extra v6 frame is its own MLDv2 solicited-node group-join report. `netaddr.rc`'s `slirp_guestv6`/`slirp_gwv6` finalized to `fec0::15`/`fec0::2` from this probe; the gate pins to the probed reality (`NETIP_V6_SLIRP_NA = true` with a loud drift arm).

### 08-08 — the aan drop-and-resume gate + the tcpblast soak machinery — r9 `58b3ae4` (HEAD at plan close)

- Gate (NETIP-07, D-14): `R9_NETIP_SWEEP=aan … --config virtip` PASS — the aan client+server pair over tcp LOOPBACK establishes, the scripted in-band drop (`rc /lib/netip-aan-drop.rc <port>` transmitted as ONE plain command line; the SERVER-side rc walks /net/tcp and writes `hangup` to the underlying conv) kills the conn (both ends see `network closed`), the client redials, and a second marker round-trip completes the resume — NIC-free, with the unmodified 08-03-baked aan binary. Two pre-existing kernel bugs fixed en route (each its own commit): the fd op-ref pattern (pread/pwrite hold a transient clone — the take/store emptied the slot mid-op; r9 `a81143e`) and sys_rendezvous's post-wake re-verification (the stale-waiter spurious-ready class; r9 `688ed87`); plus the fault autopsy observability (unresolvable() names pid+name + dumps the TrapFrame + the live proc table with parent/argv topology; r9 `a01089d`).
- Gate (NETIP-08 machinery, D-15): `R9_NETIP_SWEEP=soak … --config virtip` PASS — tcpblast exchanges 64 TCG-scaled bidirectional rounds (exactly 2 MiB each way) with the host echo endpoint (xtask's 127.0.0.1-bound listener spawned pre-qemu / joined post-run — T-08-14); the pcap proves both directions above the rounds-scaled threshold with ZERO RSTs; the guest `/net/tcp/stats` counters reconcile (InSegs 1408 / OutSegs 1216). The 8-round override PASS at its own scale (262144 bytes each way) proves the threshold honest at every scale. The pcap TCP/IPv4 classifier + assert_tcp_bidirectional carry 5 host pins — r9 `d86ed9f` + `58b3ae4`.
- **Venue: slirp — `netip: NOT-L2-TRUTH slirp venue` printed INTO the gate log. The tap-leg truth verdict is Phase 9's joint Phase-8/9 acceptance gate (D-15) — see the handoff below.**

### 08-09 — the composed smoke + CI + the pin (this plan) — r9 `19f61ed133d96fcdbfdfc5049ab2168beae512b3`

- r9 `14b2f23`: `config_virtshell.toml` gains `dev = [..., "ip"]` — the PACKAGED kernel (`r9.packages.<system>.kernel-virt`) carries devip `#I` (inert until a `bind #I` lands); the compose pipeline's clean-checkout consumer gets the phase's work. (Rule 3 — the packaged kernel shipped without devip before this.)
- r9 `19f61ed`: **EtherDev drops its netif convs** — `impl Drop for EtherDev` closes the three lane convs (`conv_close`'s last-close reset) so a re-bind can allocate the ether types; ipifc's unbind takes the core OUT of the STACK cell and drops it after the guard (the seam never holds STACK across netif's locks — the bind path's discipline mirrored). First exercised live by the composed smoke's static→dhcp unbind+rebind.
- **The composed net smoke (D-13):** `build/aarch64/smoke-r9-net.expect` + `nix/smoke-r9-net.sh` + the flake app `boot-r9-net-smoke` — smoke-r9.expect's shape with the five D-13 flow stages composed into ONE boot, each presence-asserted (never interleaving order): ipconfig static → unbind ether → ipconfig dhcp → unbind ether → ping 127.0.0.1 + udp echo (the baked fixtures) → tcp/7 echo typed inline (continuation-line paced) → `NET-SMOKE: ALL STAGES PASS` (typed-echo two-occurrence) + the pid-warning-absent/nic-bind post-passes; both slirp legs' **NOT-L2-TRUTH** labels stage-asserted.
- **Smoke verdict (the plumbing proof, 08-09 Task 1):** exit 0 on BOTH host shapes — `virt,gic-version=2` + tcg cortex-a72 (the CI shape) and `virt,gic-version=3` + hvf -cpu host (the darwin default) — against the live sibling-checkout kernel at `19f61ed` (direct-env per the smoke's R9_KERNEL/R9_DTB/R9_INITRD contract; the pre-pin run could not use the stale pin's kernel, which predates devip). Retained captures: `08-09-net-smoke-gicv2-tcg.log`, `08-09-net-smoke-gicv3-hvf.log` (this directory).
- **Smoke verdict at the pin (08-09 Task 3):** `nix run .#boot-r9-net-smoke` — the full packaged pipeline (nix builds the pinned `19f61ed` kernel package from the pushed github rev, the userland build, the bake, the smoke) — **PASS, exit 0** on this darwin host with `R9_GICV2=1` (the TCG/gicv2 shape — the CI venue's timing class). The default darwin leg (gicv3+hvf) passed the same tree direct-env (above); its one packaged-run failure was the pre-existing rc argv-stack flake (see the defects section — HVF-exposed, never seen on TCG, and the plain 05-12 smoke hits it too).
- **Hosted CI (D-16):** the `net-smoke` job in `r9-shell.yml` beside `boot-smoke` (byte-untouched), both running `nix run .#boot-r9-*-smoke` on ubuntu-latest with the pinned installers. **Run 42171968459** — <https://github.com/requaos/harvey/actions/runs/42171968459> — **BOTH JOBS success** (boot-smoke AND net-smoke) at headSha `bfc7fd8de4609ff097aff8d69453d36257f682b1` (the pushed pin+compose+CI-job+evidence HEAD), 2026-11-06T06:02Z — the composed net smoke green in hosted TCG CI on its first run (and the pre-existing rc flake does NOT fire on the TCG venue, as recorded). **Run 42173127408** — also **both jobs success** at headSha `daac7e6a1be4c6bf587771164b4995fbd2ff1316` (the subsequent tip carrying the plan's metadata commit), 2026-11-06 — the 07-09/07-10 both-runs convention.

## The D-16 sustained soak (documented-manual — the exact dev-host command)

The sustained bidirectional soak is a dev-host discipline, NOT a CI leg (TCG on every automated venue; CI wall-clock honesty over a flaky mega-gate — D-16's deliberate Phase-6 override). The exact invocation, committed verbatim in `build/aarch64/root-tree/lib/netip-soak.rc`'s header:

```
cd ../r9 && R9_HARVEY_DIR=<the harvey checkout> \
    R9_NETIP_SOAK_ROUNDS=4096 R9_NETIP_SWEEP=soak \
    cargo xtask boot-test --arch aarch64 --config virtip
```

4096 rounds = 256 MiB total, 128 MiB each way (the named sustained count; the gate default 64 is the TCG-sane CI scale; the rounds override plumbs through the sweep's typed line, and the pcap byte threshold scales with it).

## Regression state at the pinned HEAD (r9 `19f61ed`)

| Leg | Command | Result |
|-----|---------|--------|
| host suites | `cargo test -p port` | 152/152 PASS (incl. the 35-pin devip suite) |
| 8 boot profiles | `cargo xtask boot-test --arch aarch64 --config {virt,virtlowecam,virtexec,virtexec3,virtmulti,virtnotes,virtpanic,virtnet}` | PASS ×8 |
| 7 virtip modes | `R9_NETIP_SWEEP={loopback,tcp,static,dhcp,v6,aan,soak} cargo xtask boot-test --arch aarch64 --config virtip` | PASS ×7 |
| composed net smoke (darwin, gicv3+hvf) | direct-env `expect build/aarch64/smoke-r9-net.expect` at the live `19f61ed` kernel | PASS (08-09) |
| composed net smoke (darwin, gicv2+tcg) | the same, gicv2 leg | PASS (08-09) |
| composed net smoke at the pin (the packaged pipeline) | `nix run .#boot-r9-net-smoke` (R9_GICV2=1 — the CI shape) | PASS, exit 0 (08-09, darwin host) |
| composed shell smoke (the 05-12 gate) on the devip kernel | direct-env `expect build/aarch64/smoke-r9.expect` at the live `19f61ed` kernel, gicv3+hvf | PASS ×2 (08-09; the virtshell+`ip` no-regression leg — plus the `virt` profile in the battery and the net smoke's own boot stages on both shapes) |

## OQ-1 verdict + the slirp v6 probe (the phase's two research closures)

- **OQ-1 (08-02, empirical, PASS-path):** the udp socket sources 0.0.0.0 on the wire ONLY when the send metadata carries `local_address: Some(0.0.0.0)` — the naive shape is silently dropped on an address-less interface. The bridge sets it from the conv's recorded laddr in exactly the absorbed-DHCP window (08-07's file-surface rule, keyed on the self-table's accept-all flag); no wire-layer shim; the D-09 dhcpv4 fallback stays unwired (contract §12.6).
- **slirp v6 probe (08-07):** recorded above under 08-07 — echo served + ND answered, no RAs; `fec0::15`/`fec0::2` finalized.

## Phase-9 handoff (the tap leg — NETIP-08's truth verdict, D-15's joint acceptance gate)

Phase 9 lands the tap/bridge venue and re-runs with THIS phase's machinery, unchanged:

1. **DHCP on tap** — `R9_NETIP_SWEEP=dhcp` shape against the tap leg (real L2 broadcast, not slirp's forgiving DHCP).
2. **Static on tap** — `R9_NETIP_SWEEP=static` shape (real ARP on the wire).
3. **The sustained soak on tap** — the documented-manual command above; the pcap + guest-counter reconciliation as the exit-gate evidence.
4. **The HNET-02 TOML lift** of `build/aarch64/netaddr.rc` (the D-11 seam becomes platform TOML single-sourcing — the Phase 7 D-09 flag-shape pattern's data form).
5. **The multi-waiter RX-wake defect is a BLOCKER-CLASS for the tap venue** (aan/cpu ride real net with many blocked net procs) — recorded in `deferred-items.md` + WINDOWS; must be root-caused before Phase 12's cpu service depends on it.

## Defects/hazards recorded this phase (open unless noted)

- **The RX-IRQ wake with MULTIPLE registered devip waiters wedges/panics** (08-09, measured both GIC gens; static-alone passes) — the phase gates all ran ≤1 waiter. Sidestepped in the composed smoke by stage order (ether legs first, single-waiter per exchange; loopback last — loopback delivery is synchronous in the sender's drive).
- **EtherDev leaked its netif convs on unbind** ("ether-type already in use" on re-bind) — **FIXED in 08-09 (r9 `19f61ed`)** and proven by the composed smoke's double unbind+rebind.
- **devcons input burst truncation** (08-09): a typed line longer than the PL011-side 256-byte RX ring truncates mid-line with no reader-visible signal (measured: wedgedbg `line=255`); the smoke's typed-input discipline (short lines / paced continuation lines) is encoded in its header.
- **rc loop-nested backquote flake residual** (08-09): `pushword but no argv!` fired once (HVF) inside netip-loopback.rc's udp spin — the 06-07 fix covered the top-level case; the loop-nested/pipeline case remains (the aborted child's exit status printed `<bad utf8>` — worth its own look).
- Carried open from earlier plans (their deferred-items entries govern): the arena drain under fork+exec spin loops (08-05), the namec Creat/no-union-consult corner (08-05), the rc redirect-stack/fd-reuse corner (08-08), the aan Nbuf flow-control rule for Phase 12 (08-08).

## Notes

- The legacy `harvey.yml` workflow's red streak on master (since 2026-11-04T12:01Z, pre-existing and unrelated — the amd64 9k ISO donor-VM path) was REWRITTEN onto the nix pipeline by quick task 261105-slx on 2026-11-06; its hosted state at this writing is tracked there, not here.
- The smoke captures under `build/aarch64/` are gitignored transients by design (the harness's fresh-capture hygiene rule); the retained passing copies live beside this file.
- Every slirp-green gate above carries the **NOT-L2-TRUTH** label in its log AND its row here — NETIP-08's transparency prohibition (the plan's prohibition row) holds: nothing in this index presents slirp evidence as L2 truth.

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Consolidated: 2026-11-06*
