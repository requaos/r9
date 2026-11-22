# Phase 07 Evidence Index — r9 PCI/ECAM + virtio-net + devether

**Phase:** 07-r9-pci-ecam-virtio-net-devether
**Consolidated:** 2026-11-05 (plan 07-08, task 2)
**Status:** complete — every gate mapped to the commit SHA it ran at. The phase delivers r9's first DMA-capable device: DTB-driven PCI/ECAM discovery, a from-scratch virtio transport + modern virtio-net driver, and devether `#l` presenting `/net/ether0` per `netif.h` — with the standard Harvey composed boot speaking Ethernet by default (D-10 end state).

## Pinned commits (evidence ordering: r9 pushed BEFORE the pin; the pin records the gated HEAD)

| Repo | SHA | Role |
|------|-----|------|
| r9 (`requaos/r9`, branch `main`) | **`2afa3d6ebfde3a6a5fcc123d2cfaf89fd6dc0466`** | the phase-complete HEAD — pushed to origin 2026-11-05 (`e72abfe..2afa3d6`, 19 commits) BEFORE locking (push-before-pin, T-07-17). Verified: `git -C ../r9 ls-remote origin main` names this rev post-push. |
| harvey flake.lock | pins r9 rev `2afa3d6ebfde3a6a5fcc123d2cfaf89fd6dc0466` (narHash `sha256-wPwqnkduhpj05FfMtYSzRfEyXcUup6t1f2L/MNs3c24=`, owner `requaos`) | **the pin** — locked rev == live r9 HEAD == origin/main; resolves from a clean clone. |
| harvey | `06380f2bcee21aeae3a053eee4e5b007da53e67b` | **the pin commit** (07-08 Task 1) — flake.lock ALONE (the `git show --stat` gate: 1 file, 3 insertions, 3 deletions). |
| harvey | `15739d108595bde0f17b88081dfcc25f2c271848` | 07-08 Task 2: the compose-path NIC stanza (`nix/boot-r9.sh`, `build/aarch64/run-r9.sh`, `build/aarch64/smoke-r9.expect`) + the smoke's nic-bind post-pass. The pushed HEAD the hosted CI ran at. |
| r9 (`requaos/r9`, branch `main`) | **`1c161931000e77edb546a972f33b1adfa89b0b30`** | 07-09: the G-07-2 fixed HEAD — pushed to origin 2026-11-05 (`46a2ed2..1c16193`, 2 commits: the RED pin `1e77266` + the GREEN u32 truncation `1c16193`) BEFORE locking (push-before-pin, T-07-20). Verified: `git -C ../r9 ls-remote origin main` names this rev post-push; both virtnet gates re-proved green at this exact SHA. |
| harvey | `d5df2df38699dd5af627d00932933318875fb310` | 07-09: **the pin commit** — flake.lock ALONE (the `git diff-tree` gate: 1 file, 3 insertions, 3 deletions), advancing the r9 pin `46a2ed2` → `1c16193` (narHash `sha256-Kjz9iKvVII2/YAI6LvPb5vPqPNw6eiGZSEe7ERxd4EY=`, owner `requaos`). |
| r9 (`requaos/r9`, branch `main`) | **`a5b1b42ec92d5f6be4e13c1fe370da1768316131`** | 07-10: the G-07-1 fixed HEAD — pushed to origin 2026-11-05 (`1c16193..a5b1b42`, 2 commits: the RED pin `3a49da1` + the GREEN offset-0 fix `a5b1b42`) BEFORE locking (push-before-pin, T-07-22). Verified: `git -C ../r9 ls-remote origin main` names this rev post-push; both virtnet gates re-proved green at this exact SHA. |
| harvey | `91f6ab9aa7528f0b44d69d944b29450cdd311c7f` | 07-10: **the pin commit** — flake.lock ALONE (the `git diff-tree` gate: 1 file, 3 insertions, 3 deletions), advancing the r9 pin `1c16193` → `a5b1b42` (narHash `sha256-V5cZPghFjujQA5Mzlzc+CrUQ6ksiTPX3swXdOXei1Gc=`, owner `requaos`). |

## Gate → commit mapping

### 07-01 — netdev stanza + host pcap/ARP parser — r9 `aaedaa751359b92a28734c83844b1eb70d49a52a`

- `-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0` + `-object filter-dump,…` emission in `config.rs::apply_virt_netdev`, called from both aarch64-virt spawn sites with per-step pcap paths; riscv64 MMIO leg + raspi/x86_64 argv byte-untouched (unit-test-pinned).
- xtask pcap parser accepts BOTH classic-pcap byte orders (qemu 11.1.2 writes the header host-endian); ARP frame field assertions.
- `harvey:`-prefix initrd resolution via `R9_HARVEY_DIR` (the cross-repo channel the virtnet gate later rides).

### 07-02 — DTB-driven PCI/ECAM discovery + the moved-window perturbation — r9 `230127fe5d32a9b5f2f5360094c9b0e07143afeb`

- `aarch64::platform::virt::pci`: the ECAM window read from the DTB pci node's translated `reg` (never a constant), the bus-0 walk finding virtio-net `1af4:1041` rev≥1, the `interrupt-map` → GIC SPI route decode — the three-contract-line serial print on every virt boot (NETDRV-01).
- The moved-window proof: committed `virt-gicv2-lowecam.{dts,dtb}` + `config_virtlowecam.toml` — boot green under ECAM `0x3f000000` (qemu `highmem-ecam=off`) with the identical route; xtask discovery assertions keyed by config name (`virt` → `0x4010000000`, `virtlowecam` → `0x3f000000`).

### 07-03 — virtio transport + host-side unit tests — r9 `bf11fe478fe81002283ad794e198e5b3913a0de6`

- `port/src/virtio.rs` (from-scratch, D-01; `#![forbid(unsafe_code)]`): PCI capability walk, split-virtqueue descriptor/avail/used ring ops, `RingMem::fence(order)` barrier discipline (Release → `dmb ish`, Acquire → `dmb ishld`), ISR read-to-ack.
- Host-side `cargo test -p port` ring state-machine tests (descriptor alloc, avail/used advance, wrap, barrier ORDER recorded via the mock's fence override); avail pre-linking literal per §2.7.13; MAX_QSIZE=256 loud ceiling; used-ring validation incl. odd-id device-fault rejection.
- D-01 provenance recorded r9-side (`bf11fe4` docs commit).

### 07-04 — guest payload bake (Harvey-side gates) — harvey `a288badb620fd17e634185b6c621f7c13bed6679` + `9fb015a573d434b8d029e8399191ec6ea43319a4`

- `build/aarch64/root-tree/lib/test/arp.req` — the committed 60-byte ARP-request fixture (D-07 + ETHERMINTU errata: dst broadcast, src `52:54:00:12:34:56`, spa `10.0.2.15`, tpa `10.0.2.2`, 18-byte zero pad).
- `build/aarch64/root-tree/lib/net-sweep.rc` — the sourced two-mode sweep (`arp` | `blast`) with `netsweep: FAIL <stage>` / `NET-DONE` markers.
- `build/aarch64/netblast.c` — the D-08 TX-bulk generator, baked at `/aarch64/bin/netblast`; `dd` joined the image (`dd.install` + SUBSET staging).
- Gate: green bake with all four payload paths manifest-asserted (insertion-only build-script diffs). No r9-side gate this plan; r9 HEAD at the time was `bf11fe4…` (07-03 close).

### 07-05 — the virtnet gate machinery, RED proven — r9 `993be67456a9543e93f45b4d4a2ca5c1cae8f547`

- `BootProfile::Net` stage machine: boots the REAL Harvey boot chain under xtask (the `harvey:` initrd channel), drives the typed sweep at the `%` prompt, fast-fails naming the sweep stage.
- The pcap + serial-counter assertion suite (`pcap::assert_arp_pingpong` / `assert_blast`) landed RED-path-proven first: exit 1 with `netsweep: FAIL bind` + `stacksmoke: depth 64 ok` in the retained log, three consecutive times (task gate, tracer re-run, post-GREEN re-run RED-path honesty).
- The negative scan exempts exactly the D-02 designed `syscall 10 unimplemented` fauth line; any other unimplemented-syscall line fails.

### 07-06 — the virtio-net driver bind, both GIC generations — r9 `92e9b2671fee91777a1db74fa72efda330738f10`

- BAR assignment + capability walk + modern-only negotiation (VERSION_1+MAC+STATUS, zero-offload), ring bring-up (`Ring::new` chain assembly → queue_enable LAST → DRIVER_OK → rx pre-post with ONE notify), the ISR handler (first device read is the ISR byte).
- GIC ICFGR level-trigger programming BEFORE ISENABLER in BOTH `gicv2.rs` and `gicv3.rs` enable paths (Pitfall N4); the NIC binds on both generations — `virtexec3` was the first NIC-on-GICv3 boot (bind-line greps on virtexec + virtexec3).
- `dev = ["ether"]` across every aarch64 virt config — incl. `virtshell = ["exec", "bootchain", "ether"]`, the packaged kernel the Harvey compose boot consumes (this plan's dependency).
- Gate-exposed latent fix: the device-VA arena now reserves a map's WHOLE span (`vm::next_free_device_va_span`) — the one-page bump had aliased the 64 KiB GICD map over the ECAM window's dev-1 page (latent since 07-02).

### 07-07 — devether `#l` + the payoff gates — r9 `2afa3d6ebfde3a6a5fcc123d2cfaf89fd6dc0466`

- `port::netif` + `port::devether` (both `#![forbid(unsafe_code)]`): the three-level `netif.h` file tree at `/net/ether0`, clone/open/close semantics, four-string stat records, live counters, the blocking whole-frame Qmsg read, the netif-faithful TX write path (60..=1514 bounds, src-MAC overwrite, block-on-full).
- **arp-mode ping-pong (NETDRV-03):** `cargo xtask boot-test --arch aarch64 --config virtnet` PASS — 4 guest ARP requests field-exact (sha = the negotiated MAC parsed from the bind line, spa `10.0.2.15`, tpa `10.0.2.2`, len 60) + 4 slirp replies; pcap == 2N == 8 frames; stats `in:4/out:4`; ifstats `used==lastused` per queue with `nintr ∈ [1,4]`; zero `unhandled irq` lines.
  Evidence (committed): `evidence/07-07-virtnet-arp.log`, `evidence/07-07-virtnet-arp.pcap` (8 frames, byte-exact vs the gated capture).
- **blast-mode load (NETDRV-04):** `R9_NET_SWEEP_MODE=blast cargo xtask boot-test --arch aarch64 --config virtnet` PASS on the same HEAD — netblast's 256 pure-TX frames reconcile exactly (pcap frames == sent == stats out == 256; stats in == 0; `nintr_tx=86 ∈ [1,256]`; zero soverflows; `NET-DONE` inside the budget; negative scan clean) — no silent virtqueue wedge.
  Evidence (committed): `evidence/07-07-virtnet-blast.log`, `evidence/07-07-virtnet-blast.pcap` (256 frames, all `oper=1`).
- Gate-exposed pre-existing kernel fixes riding the same HEAD: `sys_await` %q status quoting (rc's `||`/`&&` had inverted for EVERY successful child); `MAX_PROCS` 16→64 (the never-reaped proc table is a boot-session fork budget — the sweep forks ~20).

### 07-08 — the pin + the standard boot speaks Ethernet — harvey `06380f2bcee21aeae3a053eee4e5b007da53e67b` (pin) + `15739d108595bde0f17b88081dfcc25f2c271848` (compose)

- **Push-before-pin:** r9 `e72abfe..2afa3d6` (19 commits) pushed to `requaos/r9` main BEFORE the lock advanced; `git ls-remote origin main` == the live HEAD; the pin commit names only `flake.lock`; owner==requaos (T-07-17's three gates).
- **D-10 end state:** `-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0` on all three compose paths (`nix/boot-r9.sh`, `build/aarch64/run-r9.sh`, `build/aarch64/smoke-r9.expect` spawn argv) — byte-identical tokens, the flag shape Phase 9's HNET-02 single-sources into platform TOML; no filter-dump object on the compose paths (capture stays a gate feature). The interactive dev harness `build/aarch64/drive-r9.expect` is noted but not gated.
- **Smoke verdict (the composed-boot NIC proof):** `nix run .#boot-r9-smoke` exit 0 on darwin-aarch64, 2026-11-05, default leg **`virt,gic-version=3 accel=hvf cpu=host`** — the first live NIC-on-GICv3 composed boot (R9_GICV2=1 remains the bisection hatch, never the gate default). Kernel under test: the pin's store path `/nix/store/d1vf44ivgdwkyg9w6zrzaxf7nlib6zj7-r9-kernel-virt-2afa3d6/kernel-virt.elf`. Capture: `build/aarch64/smoke-r9.log` (transient, gitignored; 1442 lines). Every pre-existing stage assertion passed unchanged (banner → paqfs → stacksmoke → prompt → echo round-trip → ls/ps/cat sweep → DEL safety → restart → pid-warning-absent) PLUS the new whole-capture post-pass:
  - `990:#l0: virtio: 1000Mbps: 52:54:00:12:34:56` — the virtio-net bind line PRESENT (nic-bind stage ok).
  - The spawn line self-describes the stanza: `-netdev user,id=n0 -device virtio-net-pci-non-transitional,netdev=n0`.
- **Hosted CI (r9-shell.yml):** run **41955568449** — <https://github.com/requaos/harvey/actions/runs/41955568449> — **conclusion: success**, run #7, headSha `15739d108595bde0f17b88081dfcc25f2c271848` (the pushed compose commit carrying the pin), 2026-11-05T02:29:42Z → 02:37:05Z on ubuntu-latest (`nix run .#boot-r9-smoke`, TCG). A FRESH hosted run covering the new pin — nothing pending, nothing substituted.

### 07-09 — readnum_content donor-truncation fix + conv_close pin — virtnet gate re-proof at r9 `1c161931000e77edb546a972f33b1adfa89b0b30`

- **arp mode (NETDRV-03 re-proof):** `R9_BOOT_TEST_TIMEOUT=600 cargo xtask boot-test --arch aarch64 --config virtnet` PASS at r9 `1c161931000e77edb546a972f33b1adfa89b0b30` (the G-07-2 fixed HEAD) — banner, real Harvey boot chain, typed arp sweep N=4, netsweep markers + pcap assertion observed.
- **blast mode (NETDRV-04 re-proof):** `R9_BOOT_TEST_TIMEOUT=600 R9_NET_SWEEP_MODE=blast cargo xtask boot-test --arch aarch64 --config virtnet` PASS on the SAME HEAD (no intermediate r9 commits between the two runs) — typed blast sweep N=256, NET-DONE + pcap assertion observed. Devether behavior changed (readnum_content), so both gates re-prove the data path.
- Captures/counters are unchanged by this fix (the type-file read path is not exercised by the sweeps): no new logs/pcaps committed — this is an index re-proof row, not an evidence refresh.
- Host-side pins land at the same HEAD: `port/src/devether_tests.rs` (crate-root) — readnum_content(-1)==` 4294967295 ` byte-exact + the 0/65535 neighbors + the conv_close last-close reset pin (G-07-1's donor-intentional record); port suite 51/51 (+4), `cargo xtask test` green.

### 07-10 — RX payload-offset fix (G-07-1 round 2) — virtnet gate re-proof at r9 a5b1b42ec92d5f6be4e13c1fe370da1768316131

- **arp mode (NETDRV-03 re-proof):** `R9_BOOT_TEST_TIMEOUT=600 cargo xtask boot-test --arch aarch64 --config virtnet` PASS at r9 `a5b1b42ec92d5f6be4e13c1fe370da1768316131` (the G-07-1 fixed HEAD) — banner, real Harvey boot chain, typed arp sweep N=4, netsweep markers + pcap assertion observed; counters reconcile: stats `in:4/out:4` (`in:` counts at rx_ingress entry, BEFORE the demux — unchanged now that delivery works), pcap == 2N == 8 frames, ifstats `used==lastused` per queue, zero `unhandled irq` lines.
- **blast mode (NETDRV-04 re-proof):** `R9_BOOT_TEST_TIMEOUT=600 R9_NET_SWEEP_MODE=blast cargo xtask boot-test --arch aarch64 --config virtnet` PASS on the SAME HEAD (no intermediate r9 commits between the two runs) — typed blast sweep N=256, NET-DONE + pcap assertion observed; soverflows stays 0 (256 × 64 B ≈ 16 KiB against the 64 KiB per-conversation store cap, with delivery now working).
- Captures/counters are unchanged by this fix (the sweeps assert counters and pcaps, not conversation-queue delivery — that end-to-end confirmation is the guest UAT re-run, verify-work's), so no new logs/pcaps were committed — this is an index re-proof row, not an evidence refresh.
- Host-side pins land at the same HEAD: `port/src/rx_chain_tests.rs` (crate-root) — the chain-layout premise pin + the byte-exact PCAP_REPLY delivery-input pin + the max-size within-page pin through the REAL Ring and the REAL drain arithmetic (the previously-uncovered Ring↔pci.rs seam); port suite lib 54/54 (+3), `cargo xtask test --verbose` green.

## Regression state at the pinned HEAD (r9 `2afa3d6`, from 07-07's self-check)

| Leg | Command | Result |
|-----|---------|--------|
| seven non-net aarch64 virt profiles | `for p in virt virtlowecam virtexec virtexec3 virtmulti virtnotes virtpanic: cargo xtask boot-test --arch aarch64 --config $p` | PASS ×7 |
| host suites | `cargo xtask test` (port 45/45, xtask 47/47) + `cargo xtask check` + `cargo xtask clippy` | green |
| raspi/x86_64/riscv64 builds | unaffected (port additions compile everywhere; their argv byte-untouched) | green |
| composed smoke (darwin, gicv3+hvf) | `nix run .#boot-r9-smoke` at the 2afa3d6 pin | PASS (this plan) |
| composed smoke (linux CI, TCG) | r9-shell.yml run 41955568449 | success (this plan) |

## Notes

- The legacy `harvey.yml` workflow's run on the same push (41955568464) concluded **failure** — PRE-EXISTING and unrelated: it has been red on every master push since at least 2026-11-04T12:01Z (runs 4–9, incl. pushes before this phase's commits existed). It exercises the amd64 9k ISO donor-VM path, which neither the pin nor the aarch64 compose scripts touch. Recorded for transparency; not this phase's gate.
- The smoke capture (`build/aarch64/smoke-r9.log`) is a gitignored transient by design (the harness's fresh-capture hygiene rule); this index's quoted line numbers + the hosted run are the durable record.

---
*Phase: 07-r9-pci-ecam-virtio-net-devether*
*Consolidated: 2026-11-05*
