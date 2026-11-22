---
phase: 16-host-net-modes-networking-exit-gate
verified: 2026-11-12T13:24:42Z
status: passed
score: 48/48 must-haves verified
covered_files: [".github/workflows/r9-shell.yml", ".planning/phases/16-host-net-modes-networking-exit-gate/16-01-PLAN.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-01-SUMMARY.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-02-PLAN.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-02-SUMMARY.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-03-PLAN.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-03-SUMMARY.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-04-PLAN.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-04-SUMMARY.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-05-PLAN.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-05-SUMMARY.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-06-PLAN.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-06-SUMMARY.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-07-PLAN.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-07-SUMMARY.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-08-PLAN.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-08-SUMMARY.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-EVIDENCE.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-REVIEW-DISPOSITION.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-REVIEW-FIX.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-REVIEW.md", ".planning/phases/16-host-net-modes-networking-exit-gate/16-UAT.md", ".planning/phases/16-host-net-modes-networking-exit-gate/deferred-items.md", ".planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-04-boot-slirp.log", ".planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-05-netsoak-a.log", ".planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-05-netsoak-a.pcap.gz", ".planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-05-netsoak-b.log", ".planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-05-netsoak-b.pcap.gz", ".planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-06-netsmoke-a.log", ".planning/phases/16-host-net-modes-networking-exit-gate/evidence/16-06-netsmoke-b.log", "build/x86_64/README.md", "build/x86_64/mkimage.sh", "build/x86_64/netsoak-r9.expect", "build/x86_64/root-tree/lib/netaddr.rc", "build/x86_64/root-tree/lib/netip-tap.rc", "build/x86_64/smoke-r9-net.expect", "flake.lock", "flake.nix", "nix/boot-r9-x64.sh", "nix/host-echo.c", "nix/net-doctor.sh", "nix/smoke-r9-net-x64.sh", "nix/soak-r9-net-x64.sh"]
covered_digest: "v4:sha256:9919e7477747a37c56539e7c62b50c4881635b74826c129649e7b3610ba5706c"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 48/48
  gaps_closed:
    - "Hosted-CI observation of the pushed phase — SATISFIED by direct gh observation this session: run 43121894436 (r9-shell, created 2026-11-11T23:42:41Z) completed conclusion=success with all three jobs green incl. net-smoke-x64; three further r9-shell runs green since (43118249320, 43133062661, 43137727518); the run's tree carries the phase-16 surface byte-identical to current HEAD"
  gaps_remaining: []
  regressions: []
---

# Phase 16: Host Net Modes + Networking Exit Gate Verification Report

**Phase Goal:** Host networking is an explicit, documented choice — a hermetic slirp+hostfwd default that runs unprivileged and in CI, real-L2 tap/bridge on the linux primary host (opt-in socket_vmnet on darwin), a doctor-checker that prints exact remediation — and the milestone's networking exit gate (sustained bidirectional TCP on the tap leg) runs there. Work spans both repos (r9 platform TOML + Harvey flake/docs).
**Verified:** 2026-11-12T13:24:42Z
**Status:** passed
**Re-verification:** Yes — supersedes the 2026-11-11T17:05:00Z report (stale: covered sources changed after it was written — the WR-02 emitter fix at r9 208ceba, the pin advance 4d19a9e75, the clean-checkout gate-script fix, and the later NixOS-portability venue fixes). All truths re-checked against the current tree; live probes re-executed by this verifier; the prior report's single human-verification item is now closed by direct observation.

## Goal Achievement

### Observable Truths

Roadmap success criteria (the contract) plus the eight plans' frontmatter truths (43). Every truth was re-checked against the current codebase; behavioral truths were re-exercised live this session by the verifier.

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | HNET-01 — default boot hermetic and CI-able: slirp, hostfwd on 127.0.0.1 for BOTH tcp/17010 and tcp/567, zero privilege, green in hosted CI | ✓ VERIFIED | TOML data in all five q35 configs carries both loopback hostfwd tokens (grep ×5, one each per file); verifier ran BOTH the freshly-built emitter (`cargo build` at r9 208ceba) AND the nix-built store bin the flake wires (`r9-netdev-args-208ceba`): `--mode slirp` → `-netdev user,id=n0,hostfwd=tcp:127.0.0.1:17010-:17010,hostfwd=tcp:127.0.0.1:5670-:567 …`, exit 0; `evidence/16-04-boot-slirp.log` carries the banner, `%` prompt, both hostfwd tokens, zero `0.0.0.0`; net gate green twice (`evidence/16-06-netsmoke-a/b.log`, distinct md5s, NOT-L2-TRUTH ×2 + NETIP-DONE each, zero FAIL); **green in hosted CI: DIRECTLY OBSERVED via gh — run 43121894436 (r9-shell) conclusion=success with the `net-smoke-x64` job success on ubuntu-latest under `R9_ACCEL=tcg` (created 2026-11-11T23:42:41Z, completed 00:01:08Z), plus three further green r9-shell runs since (43118249320 @6f90078ee, 43133062661 @865960ddd, 43137727518 @048f43925); the run's commit 79cd4ab1d carries every phase-16 surface file byte-identical to current HEAD (only non-phase `nix/build.sh` differs); a run at current HEAD 6c80bfa64 was in_progress at verification time. All 16 phase commits pushed (origin/master..HEAD = 0)** |
| SC2 | HNET-02 — `--net=slirp\|tap\|vmnet` explicit selection, no silent downgrade; netdev stanza single-sourced in r9 TOML, consumed by Harvey's flake | ✓ VERIFIED | Verifier re-ran `boot-r9-x64.sh` live this session (env contract via realized store paths): `--net=bogus` → exit 1 `--net/R9_NET=bogus: want slirp\|tap\|vmnet`; `--net=vmnet` on linux → exit 1 cross-venue naming `linux's valid net modes: slirp tap`; `--net tap` (no `=`) → exit 1 naming the accepted form; `--net=tap` → the doctor runs inline FIRST (full 8-check report embedded: 7 PASS + WARN masquerade + DOCTOR-OK), then the pipeline proceeds (shape echo `net=tap`, emitter argv; verifier interrupted at the bake — tree left clean). `grep -c 'netdev user'` = 0 across boot/smoke/soak scripts AND both expect drivers; flake.nix exports `R9_NETDEV_EMITTER="${r9.packages.${system}.netdev-args}/bin/r9-netdev-args"` in all three apps (flake.nix:173/240/320); ONE builder: `netdev_args`/`netdev_args_checked` in r9 `xtask/src/config.rs:555/570`, bin routes through `netdev_args_checked` (bin:19,98) |
| SC3 | HNET-03/HNET-04 — linux tap/bridge leg documented per-distro and tested on the primary host (L2-truth venue); darwin socket_vmnet opt-in documented-manual | ✓ VERIFIED | README (430 lines): "### The NixOS page — TESTED on the primary host (D-10)" (:194) leading with the nosuid-helper (:205) and `allow virbr0`-vs-`br0` trap (:216) divergences; "### The generic-Linux page — DOCUMENTED, NOT TESTED" (:276); "### The darwin page — DOCUMENTED-MANUAL (HNET-04)" (:295); real-LAN "NOT TESTED" (:186). Tested-on-host proof: the green-twice tap captures (SC5) + verifier re-ran the doctor live on THIS host today: br0 UP at 192.168.122.1/24, ip_forward=1, helper mode 4511, exact `allow br0` line present, DOCTOR-OK — the venue is real and remediated |
| SC4 | HNET-05 — doctor diagnoses privileged host setup, prints exact sudo/bridge/firewall remediation | ✓ VERIFIED | Verifier ran `net-doctor.sh` live: `--mode slirp` → 3 PASS + DOCTOR-OK exit 0; `--mode tap` → 7 PASS + WARN masquerade (exact `sudo iptables-save`/`sudo nft list ruleset` commands) + DOCTOR-OK exit 0 (WARN-never-gates proven live); `--mode slirq` → exit 1 usage. `FAILS` counter + exit semantics at :82-92/:437-442; `remedy_nixos`/`remedy_generic` dispatched on `/run/current-system` (:119-127); "allow all" explicitly forbidden (:175,:228); read-only — mutation-shaped lines only inside `cat <<EOF` print payloads |
| SC5 | NETIP-08 — exit gate green on x86_64: sustained bidirectional TCP + load soak on the tap leg; slirp evidence explicitly not accepted | ✓ VERIFIED | Verifier re-asserted markers on BOTH retained captures (text-mode grep): `16-05-netsoak-a.log` (md5 5f262258…) / `-b.log` (md5 84ebd584…) each carry exactly 9 `tcpblast: done`, zero `netip: FAIL`, `arp view ok` ×1, `L2-TRUTH tap venue` ×1, `NETIP-DONE` ×1; both pcaps genuine (gunzip → pcap magic `d4c3b2a1`); the soak app refuses slirp at parse time before any contract/bake/boot — verifier re-ran it live BOTH ways (`--net=slirp` and `R9_NET=slirp`) → exact refusal, exit 1; both green runs rode pinned r9 d1c4849 and the current pin's delta is provably xtask-bin-only (`git diff --stat d1c4849..208ceba` = 1 file, `xtask/src/bin/r9-netdev-args.rs` +41, the WR-02 fix — no kernel bytes), so the retained evidence still describes the kernel bytes the pin builds. The open 1-in-5 observer-healed burst triple fault stays recorded (deferred-items, third entry) and does not falsify the consecutive-green-twice wording |
| 1.1 | 16-01 — all five q35 TOMLs carry `[qemu.netdev]` default + three sub-tables; slirp argv contains exactly the two hostfwd tokens | ✓ VERIFIED | `default = "slirp"` + slirp/tap/vmnet sub-tables extracted from all five (q35shell shown in full); token counts 1/1 (17010, 5670) per file ×5; behavioral emitter run confirms the emitted argv |
| 1.2 | 16-01 — tap argv contains `br=br0` and `helper=/run/wrappers/bin/qemu-bridge-helper` verbatim | ✓ VERIFIED | TOML data ×5 + verifier ran both the fresh-built and nix-built 208ceba emitter: `--mode tap` → `-netdev tap,id=n0,br=br0,helper=/run/wrappers/bin/qemu-bridge-helper …` |
| 1.3 | 16-01 — ONE arg-builder serves both front-ends byte-identically | ✓ VERIFIED | `netdev_args_checked` pure core consumed by both `apply_netdev` (config.rs:651) and the bin (bin:19 import, argv from `netdev_args_checked` at bin:98); 22 tests in config.rs + 2 lib tests + 1 bin test (counts re-verified); no disabled tests |
| 1.4 | 16-01 — flake exports `packages.<system>.netdev-args`; bin rejects unknown mode with usage listing the modes | ✓ VERIFIED | flake export wired (Harvey flake.nix references it in 3 apps; realized store bin `r9-netdev-args-208ceba` present); verifier ran the bin: `--mode bogus` → exit 1, usage `--mode slirp\|tap\|vmnet […]` |
| 1.5 | 16-01 — `cargo xtask check`/`clippy` pass at the final r9 commit; commits pushed before pin | ✓ VERIFIED | Documented at d1c4849 (16-01 SUMMARY) and re-run at 208ceba by the review-fix pass (16-REVIEW-FIX: check exit 0, 112 tests green); push-before-pin re-verified: d1c4849 and 208ceba both on r9 origin/main (verifier-checked this session) and both precede their harvey pin commits (45d4436f8, 4d19a9e75) |
| 1.6 | 16-01 — no new r9-side tap boot-test profile (D-07) | ✓ VERIFIED | `git diff --stat 5f856c9..208ceba` = 13 files: flake.nix, the five config TOMLs, xtask/* only (main.rs −80 lines = the lib split; pcap.rs/uregcheck.rs minor split fallout) — no profile/gate additions, no kernel bytes |
| 2.1 | 16-02 — netaddr.rc gains additive tap_ keys; slirp_ keys and blastport byte-unchanged | ✓ VERIFIED | Read the seam: `tap_guestv4=192.168.122.15`, `tap_maskv4=255.255.255.0`, `tap_gwv4=192.168.122.1` (:55-57) added; `slirp_guestv4`/`slirp_gwv4` (:40-42) + `blastport=7777` (:62) in their original shape |
| 2.2 | 16-02 — /lib/netip-tap.rc drives real ipconfig, pings gw, asserts /net/arp post-flow | ✓ VERIFIED | Read the 234-line fixture (grew with 16-08's burst-budget fix): real `ip/ipconfig -g $tap_gwv4 ether /net/ether0 add …` (:93), `ip/ping -n 3 $tap_gwv4` (:142), the /net/arp post-flow assert at :147-151 with the L2-TRUTH label |
| 2.3 | 16-02 — venue label + NETIP-DONE printed | ✓ VERIFIED | In-fixture markers (`netip: L2-TRUTH tap venue` :169, `netip: NETIP-DONE` :233) verified in code AND in both retained green captures |
| 2.4 | 16-02 — bake seam assert extends to the tap prefix, teeth-proven | ✓ VERIFIED | mkimage.sh:526-529: one `_stray` grep per venue prefix (10.0.2. and 192.168.122.) over root-tree/lib excluding netaddr.rc, `err` on hit; verifier re-ran the tap-prefix grep standalone over root-tree (empty = clean); the doctored-input negative control is recorded in the 16-02 SUMMARY |
| 2.5 | 16-02 — netip-tap.rc staged (manifest line); slirp gates' contract unchanged | ✓ VERIFIED | mkimage.sh:513-514 presence + manifest include netip-tap.rc; additive-only edits |
| 3.1 | 16-03 — doctor prints PASS/FAIL/WARN per check, exits 0 iff zero FAILs, WARN never gates | ✓ VERIFIED | Code (`fail()`/`warn()` vocabulary, `FAILS` counter :82, exit semantics :437-442) + verifier's live runs this session: WARN masquerade present with DOCTOR-OK exit 0; bogus mode exit 1 |
| 3.2 | 16-03 — every FAIL prints exact remediation, NixOS-first | ✓ VERIFIED | `remedy_nixos` (:132, declarative: environment.etc, sysctl, trustedInterfaces) vs `remedy_generic` (:181, imperative ip/nft/sysctl/chmod) dispatched on `/run/current-system` |
| 3.3 | 16-03 — tap checks cover the live-probed reality | ✓ VERIFIED | All 8 checks live-ran green on this host today (bridge-exists/UP, seam-derived bridge-addr, ip_forward, masquerade WARN, helper-wrapper setuid 4511, exact `allow br0` line, blastport-free); the wrong-`allow` FAIL direction is recorded verbatim in the 16-03 SUMMARY |
| 3.4 | 16-03 — slirp checks qemu-present + ports free; vmnet INFO n/a on linux | ✓ VERIFIED | Live run: `PASS qemu-present`, `PASS hostfwd-17010-free`, `PASS hostfwd-5670-free` |
| 3.5 | 16-03 — the pre-remediation host reported FAIL bridge-conf-allow with environment.etc remediation | ✓ VERIFIED | Point-in-time truth — retained Live Evidence block in the 16-03 SUMMARY records the exact output; today the same check PASSes because the host was remediated for 16-08 (consistent story, corroborated by the green soaks) |
| 3.6 | 16-03 — strictly read-only | ✓ VERIFIED | Every mutation-shaped line lives inside `cat <<EOF` print blocks; probes are `ss`/`sysctl -n`/`test`/`grep` reads |
| 4.1 | 16-04 — pin advance is its OWN lock-only commit | ✓ VERIFIED | `git show 4d19a9e75` → flake.lock 3+/3-, nothing else (same for the earlier 45d4436f8); current lock rev parsed = 208ceba775577cbae194f5a47dab61930abee743 |
| 4.2 | 16-04 — --net flag > R9_NET > default; unknown/cross-venue are usage errors | ✓ VERIFIED | Code :145-172 case ladder + four live failure-shape runs re-executed by the verifier this session (bogus / vmnet-on-linux / no-`=` / tap-with-doctor) |
| 4.3 | 16-04 — netdev argv from emitter substitution; hostfwd tokens in the echo | ✓ VERIFIED | `NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode "$NET_MODE")` (:199); `grep -c 'netdev user'` = 0 on the script; the 16-04 capture carries both hostfwd tokens |
| 4.4 | 16-04 — tap-mode failure embeds doctor remediation + names `nix run .#net-doctor`, never falls through | ✓ VERIFIED | Code :185-193 (contract → doctor run → abort on FAIL with the `full report: nix run .#net-doctor` pointer); live-run by verifier today shows the inline doctor report then the pipeline proceeds on the remediated host |
| 4.5 | 16-04 — hermetic unprivileged default boot to the rc prompt | ✓ VERIFIED | `evidence/16-04-boot-slirp.log`: banner ×1, `%` prompt, both hostfwd tokens ×1 each, zero `0.0.0.0` — re-verified by direct grep this session |
| 4.6 | 16-04 — composed gates' bakes/venues untouched (D-02) | ✓ VERIFIED | The net gate's venue remains FIXED slirp (script comments + NOT-L2-TRUTH labels in captures); the post-phase venue fixes (stage-build staging) added a build STEP, changing no bake contract or venue |
| 5.1 | 16-05 — soak HARD-REFUSES slirp before any bake or boot | ✓ VERIFIED | soak-r9-net-x64.sh step 0a (~:155) parse-time `err "slirp evidence not accepted …"` ahead of the contract/doctor/everything else; verifier re-ran live: both `--net=slirp` and `R9_NET=slirp` → exact refusal, exit 1, before any contract spend |
| 5.2 | 16-05 — host echo peer is a committed C binary, poll()-concurrent, gate-child lifecycle | ✓ VERIFIED | host-echo.c: poll() loop, MAXCONN 64, binds the literal address, SIGTERM stop-flag; **verifier recompiled and re-ran it live this session**: 8 simultaneous held-open dials (all 8 connected before any closed — interleaved closes prove concurrency) → 8 distinct byte-exact replies; kill → re-dial REFUSED (listener dies with the process) |
| 5.3 | 16-05 — gate runs doctor-preflight → bake → compose → peer → tap boot → expect-driven fixture | ✓ VERIFIED | Pipeline stages in code (0a parse/0b contract/0c doctor/0d seam/0e emitter/0f peer, then bake/compose/expect); driver types `rc /lib/netip-tap.rc` over the split `R9_NETDEV_ARGV` (netsoak-r9.expect:145) |
| 5.4 | 16-05 — post-pass COUNTS `tcpblast: done` (1+8=9), never interleaving order | ✓ VERIFIED | `count_occurrences` + `want_done` assert (driver:299-315) with the NEVER-order discipline (:49,:284); the 8-count negative control recorded (16-05 SUMMARY); both green captures carry exactly 9 (verifier grep) |
| 5.5 | 16-05 — capture post-pass asserts ARP marker + tap venue label | ✓ VERIFIED | Driver bail-if-absent arms; both captures carry both markers (verifier grep: arp=1, venue=1 each) |
| 6.1 | 16-06 — net gate consumes the emitter's slirp argv (R9_NETDEV_ARGV, split in driver) | ✓ VERIFIED | smoke-r9-net-x64.sh:159 `export R9_NETDEV_ARGV="$NETDEV_ARGV"` (captured once at :153); driver `foreach tok [split $env(R9_NETDEV_ARGV)]`; `grep -c 'netdev user'` = 0 on both |
| 6.2 | 16-06 — driver-side check observes BOTH loopback hostfwd listeners mid-boot, stage-logged | ✓ VERIFIED | `hostfwd_stage` proc (driver:254, anchored `127.0.0.1:${port}( \|$)` regexp over `exec ss -tln`) invoked for 17010 and 5670 (:301-302); the iproute2 dep is in the app's runtimeInputs (flake.nix:241); the twice-green runs' stage-ok lines printed on the driver console (pty-vs-console split disclosed in 16-EVIDENCE.md) |
| 6.3 | 16-06 — r9-shell.yml net-smoke-x64 job (ubuntu-latest, TCG) — tap/soak never in CI | ✓ VERIFIED | Job present (:84-90) `env R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64`; no tap/soak job in the file; **hosted green OBSERVED (see SC1)** |
| 6.4 | 16-06 — gate green twice locally with captures retained | ✓ VERIFIED | Both captures exist, distinct md5s (e0e7d26e… / 4e36394f…), NOT-L2-TRUTH ×2 + NETIP-DONE ×3, zero FAIL each (verifier grep) |
| 6.5 | 16-06 — actions pinned to full SHAs | ✓ VERIFIED | All six `uses:` lines carry 40-hex SHAs (3× actions/checkout@11d5960a…, 3× nix-installer-action@3138316d…); zero non-SHA uses in the file |
| 7.1 | 16-07 — README documents the three modes end-to-end (D-04 remap + 1:1 variant, D-08 topology, vmnet argv) | ✓ VERIFIED | 430-line README: mode surface (`--net=` ×9 mentions), hostfwd table + remap rationale, tap topology, contracts table, probe-evidence notes |
| 7.2 | 16-07 — NixOS page is the tested page, leads with the two live-probed divergences | ✓ VERIFIED | "### The NixOS page — TESTED on the primary host (D-10)" (:194) with nosuid-helper (:205-208) + virbr0-trap (:216-217) leading |
| 7.3 | 16-07 — generic-Linux and darwin pages carry explicit not-tested labels | ✓ VERIFIED | "DOCUMENTED, NOT TESTED" (:276) and "DOCUMENTED-MANUAL (HNET-04)" (:295) section headers |
| 7.4 | 16-07 — no documented-only leg claims tested status | ✓ VERIFIED | real-LAN "NOT TESTED" (:186); no evidence row in 16-EVIDENCE.md cites the documented-only legs |
| 7.5 | 16-07 — 16-EVIDENCE.md consolidates: captures + SHAs, green-twice, proofs, PENDING FIRST PUSH | ✓ VERIFIED | Read: verdict statement, pin lineage, per-gate green-twice, 7-file index (all on disk), requirement mapping, both PENDING FIRST PUSH entries (:89-94). Note: the PENDING entries record the pre-push state; the hosted greens have since been observed (this report, SC1) — a docs refresh of those two lines is cosmetic, not a truth failure (the truth requires the file to carry the PENDING items, which it does) |
| 8.1 | 16-08 — GREEN TWICE consecutively, both captures retained, r9 SHA recorded | ✓ VERIFIED | Both captures: markers 9/0/1/1/1 verified by verifier grep; distinct md5s; both at pinned d1c4849 per the lineage table; current-pin delta proven xtask-bin-only (no kernel bytes) |
| 8.2 | 16-08 — soak-surfaced kernel defects fixed push-before-pin in-phase, or exonerated | ✓ VERIFIED | The defect that loaded was root-caused HARVEY-side (stage budget); r9 exonerated by instrumented autopsy and left byte-identical (d1c4849..208ceba touches no kernel bytes — re-verified); the genuine kernel finding (triple fault) is recorded OPEN in deferred-items, not blind-fixed |
| 8.3 | 16-08 — both captures carry exactly 9 done markers + ARP + venue + NETIP-DONE, zero FAIL | ✓ VERIFIED | Verifier's own text-mode grep on both files: done=9, fail=0, arp=1, venue=1, netipdone=1 |
| 8.4 | 16-08 — every pin advance its OWN commit, ordered after pushes | ✓ VERIFIED | Both pin commits lock-only (git show 4d19a9e75, 45d4436f8); push-before-pin verified (r9 SHAs on origin/main precede the pin commits — re-checked this session: origin/main contains 208ceba) |
| 8.5 | 16-08 — no r9-side tap boot-test profile added | ✓ VERIFIED | r9 window diff scope (xtask + flake + the five TOMLs only, re-verified) |

**Score:** 48/48 truths verified (0 present, behavior-unverified)

### Post-Prior-Report Changes (re-verification focus)

The four change-sets that made the prior report stale were each re-verified against the current tree:

| Change | Verified |
|--------|----------|
| WR-02 emitter fix (r9 208ceba) | `token_safe` guard in the bin (:47,85-91) rejects spaced/empty `--pcap` before any argv exists; live rejections re-run on BOTH the fresh-built debug bin and the nix-built `r9-netdev-args-208ceba` store bin (exit 1, exact message); the single bin test `pcap_token_safety_rejects_empty_and_whitespace` re-ran PASS |
| Pin advance 4d19a9e75 → 208ceba | Lock-only commit (flake.lock 3+/3-); lock parses to 208ceba; 208ceba = r9 origin/main (pushed); delta over the evidence SHA d1c4849 is the one bin file (+41), no kernel bytes |
| Clean-checkout gate-script fix | `stage-build.sh` staged FIRST in all four tier-driver gate scripts (smoke-r9-net-x64.sh:183-184, soak-r9-net-x64.sh:317-318, tier1-r9-x64.sh:142-143, services-r9-x64.sh:148) — the 16-06 hosted-run discovery remediation |
| NixOS portability fixes (shebangs, /bin/ls, /bin/cp, brace-free LIBS, ape substrate; commits 80ea819a3..6c80bfa64) | Post-phase CI-green remediation, not plan must-haves; confirmed present on master and OUT of the phase-16 truth set; the phase surface between the CI-green commit 79cd4ab1d and HEAD differs only in `nix/build.sh` (a non-phase file) — byte-identical otherwise |

### Required Artifacts

All plan-declared artifacts exist, are substantive, and are wired (verified manually at all levels).

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `../r9/xtask/src/lib.rs` | lib target + all-five-TOML parse tests | ✓ VERIFIED | exists; 2 include_str! tests present |
| `../r9/xtask/src/bin/r9-netdev-args.rs` | exported CLI front-end | ✓ VERIFIED | routes through the ONE builder; WR-02 token_safe guard + test re-ran green |
| `../r9/flake.nix` | `packages.<system>.netdev-args` | ✓ VERIFIED | consumed by Harvey flake in 3 apps; realized store bin present |
| `build/x86_64/root-tree/lib/netip-tap.rc` | the tap-leg truth fixture | ✓ VERIFIED | 234 lines, all legs present |
| `nix/net-doctor.sh` | read-only doctor | ✓ VERIFIED | 442 lines; live-run green all three mode shapes this session |
| `nix/boot-r9-x64.sh` | --net surface + emitter + doctor gate | ✓ VERIFIED | 276 lines; live failure-shape runs re-executed |
| `nix/host-echo.c` | poll()-concurrent peer | ✓ VERIFIED | recompiled + 8-way concurrency + lifecycle re-tested by verifier |
| `nix/soak-r9-net-x64.sh` | the soak pipeline | ✓ VERIFIED | 363 lines; slirp refusal re-run live; refusal → contract → doctor → seam → emitter → peer → bake → expect |
| `build/x86_64/netsoak-r9.expect` | soak driver | ✓ VERIFIED | 336 lines; counting post-pass; split-argv spawn |
| `nix/smoke-r9-net-x64.sh` + `build/x86_64/smoke-r9-net.expect` | net gate | ✓ VERIFIED | R9_NETDEV_ARGV export + driver split; hostfwd_stage proc for both ports; stage-build first |
| `.github/workflows/r9-shell.yml` | net-smoke-x64 job | ✓ VERIFIED | job present, all actions full-SHA-pinned, HOSTED GREEN observed |
| `build/x86_64/README.md` | net-modes companion | ✓ VERIFIED | 430 lines, all venue labels |
| `16-EVIDENCE.md` | phase evidence record | ✓ VERIFIED | index matches disk; PENDING entries now satisfied by observation |
| `evidence/16-05-netsoak-a.log` / `-b.log` | the green-twice pair | ✓ VERIFIED | markers 9/0/1/1/1 verified; distinct md5s |
| `evidence/*.pcap.gz` ×2 | wire captures | ✓ VERIFIED | genuine gzip pcaps (magic d4c3b2a1) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| q35 TOML mode tables | qemu argv | Netdev struct → netdev_args_checked → bin stdout AND apply_netdev | ✓ WIRED | code-read + live emitter runs (fresh-built AND nix-built 208ceba) for all three modes |
| r9 `packages.netdev-args` | Harvey flake | `r9.packages.${system}.netdev-args` in flake.nix (3 app exports) | ✓ WIRED | boot-r9-x64, net-smoke, net-soak apps all export R9_NETDEV_EMITTER |
| netaddr.rc tap_ keys | netip-tap.rc | sourced in-guest | ✓ WIRED | fixture uses `$tap_*` throughout; zero venue literal elsewhere (stray grep empty) |
| netaddr.rc tap_ keys | doctor bridge check | seam_init grep-eval of tap_gwv4/blastport | ✓ WIRED | live run printed the seam value (192.168.122.1/24) |
| net-doctor.sh | flake app + boot tap leg | mkScriptApp + R9_NET_DOCTOR invocation | ✓ WIRED | flake.nix:177/281/321; boot:185-193; live-run through the tap leg |
| flake.lock pin | r9 pushed SHA | lock-only pin commits after pushes | ✓ WIRED | 45d4436f8 (→d1c4849), 4d19a9e75 (→208ceba); 208ceba on origin/main |
| soak app | fixture over emitter tap argv | typed `rc /lib/netip-tap.rc` + R9_NETDEV_ARGV split | ✓ WIRED | driver:145 |
| smoke gate | emitter argv | R9_NETDEV_EMITTER → R9_NETDEV_ARGV → split spawn | ✓ WIRED | script:153/159, driver:164; zero netdev literals |
| r9-shell.yml net-smoke-x64 | the app | `env R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64` | ✓ WIRED | workflow:90; hosted run observed green |
| 16-EVIDENCE.md | retained captures + SHAs | index table | ✓ WIRED | all indexed files on disk |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| emitter argv | netdev tokens | r9 TOML data via the ONE builder | live runs (both bins) printed the TOML's exact values | ✓ FLOWING |
| boot script argv | NETDEV_ARGV | emitter command substitution | live tap probe's shape echo + the 16-04 capture's store-path emitter output | ✓ FLOWING |
| doctor tap address | tap_gwv4/TAP_SUBNET | netaddr.rc seam (grep-eval) | live run printed 192.168.122.1/24 — seam-derived, no literal in the script | ✓ FLOWING |
| soak peer bind | tap_gwv4 + blastport | seam (0d step) | peer launched with seam values; green captures prove the guest dialed them | ✓ FLOWING |
| gate captures | guest markers | real serial pty of booted guest | pty control bytes present; distinct per run (md5s) | ✓ FLOWING |
| hosted CI | workflow jobs | GitHub Actions on pushed master | gh-observed conclusions incl. net-smoke-x64 success ×4 runs | ✓ FLOWING |

### Behavioral Spot-Checks

Re-executed live by the verifier this session (all fast; the one interrupted boot probe left the tracked tree clean — git status verified):

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| emitter slirp argv carries both loopback hostfwd rules | fresh-built bin `--mode slirp` | exact argv, exit 0 | ✓ PASS |
| emitter tap argv (br0 + helper verbatim) | fresh-built bin `--mode tap` | exact argv | ✓ PASS |
| emitter vmnet argv (socket path) | fresh-built bin `--mode vmnet` | exact argv | ✓ PASS |
| emitter unknown-mode rejection | `--mode bogus` | exit 1 + usage listing slirp\|tap\|vmnet | ✓ PASS |
| **WR-02** spaced --pcap rejection | `--pcap "/tmp/has space.pcap"` (fresh + nix-built 208ceba bins) | exit 1, exact token-split message | ✓ PASS |
| **WR-02** empty --pcap rejection | `--pcap ""` | exit 1 | ✓ PASS |
| WR-02 bin test | `cargo test --bin r9-netdev-args` (single named test) | 1 passed, 0 failed | ✓ PASS |
| doctor slirp green | `sh nix/net-doctor.sh --mode slirp` | 3 PASS + DOCTOR-OK, exit 0 | ✓ PASS |
| doctor tap on live host (WARN never gates) | `sh nix/net-doctor.sh --mode tap` | 7 PASS + WARN masquerade + DOCTOR-OK, exit 0 | ✓ PASS |
| doctor unknown-mode usage error | `--mode slirq` | exit 1 | ✓ PASS |
| boot script unknown mode | `--net=bogus` | exit 1, names slirp\|tap\|vmnet | ✓ PASS |
| boot script cross-venue rejection | `--net=vmnet` on linux | exit 1, names linux's valid modes | ✓ PASS |
| boot script no-`=` form rejected | `--net tap` | usage error naming `--net=<mode>` | ✓ PASS |
| boot script tap leg doctors first | `--net=tap` (interrupted at bake) | doctor report inline (7 PASS + WARN + DOCTOR-OK), then pipeline proceeds | ✓ PASS |
| soak slirp refusal (flag) | `sh nix/soak-r9-net-x64.sh --net=slirp` | exact refusal, exit 1, before any contract | ✓ PASS |
| soak slirp refusal (env default) | `R9_NET=slirp sh nix/soak-r9-net-x64.sh` | same refusal | ✓ PASS |
| host-echo 8-way concurrency | compiled + 8 held-open dials | 8 distinct byte-exact replies, interleaved closes | ✓ PASS |
| host-echo dies with parent | pkill → re-dial | ConnectionRefused | ✓ PASS |
| NETIP-08 capture markers ×2 files | text-mode grep asserts | done=9/9, fail=0/0, arp/venue/NETIP-DONE present | ✓ PASS |
| pcap authenticity ×2 | gunzip → magic bytes | `d4 c3 b2 a1` both | ✓ PASS |
| hosted CI observation | `gh run view 43121894436` (+ run list) | conclusion=success; net-smoke-x64 job success; 3 further green runs | ✓ PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes exist in this project; phase verification rides the flake gate apps, live tool probes (above), and retained evidence.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| HNET-01 | 16-04, 16-06 | slirp+hostfwd hermetic default, loopback-bound pair, zero privilege, CI-able | ✓ SATISFIED | SC1 row, hosted green now OBSERVED |
| HNET-02 | 16-01, 16-04 | explicit --net selection, no silent downgrade, netdev single-sourced in TOML | ✓ SATISFIED | SC2 row |
| HNET-03 | 16-02, 16-05, 16-07, 16-08 | linux tap/bridge leg documented per-distro, tested on primary host | ✓ SATISFIED | SC3 row |
| HNET-04 | 16-01, 16-07 | darwin socket_vmnet documented-manual | ✓ SATISFIED | vmnet TOML table + emitter arm live-run; README DOCUMENTED-MANUAL page; doctor INFO n/a |
| HNET-05 | 16-03 | doctor checker with exact remediation | ✓ SATISFIED | SC4 row |
| NETIP-08 | 16-02, 16-05, 16-08 | exit gate: sustained bidirectional TCP + soak on the tap leg, slirp refused | ✓ SATISFIED | SC5 row |

REQUIREMENTS.md maps exactly these six IDs to Phase 16 (all six rows `| <ID> | Phase 16 | Complete |`) — no orphans, none unaccounted.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `../r9/xtask/src/main.rs` | 630-631 | `FIXME:` ×2 (comment) | ℹ️ Info | Pre-existing upstream r9 debt — the phase window diff (5f856c9..208ceba) adds no FIXME/TODO lines (verifier grep on the diff: 0) |
| `../r9/xtask/src/main.rs` | 590 | `// TODO Choose UART at cmdline` | ℹ️ Info | Pre-existing (Phase 3); untouched by this phase's diff |
| gate scripts / mkimage.sh | various | `mktemp -d …XXXXXX` matched the XXX scan | ℹ️ Info | False positive — mktemp template characters, not debt markers |

Debt-marker gate: no TBD/FIXME/XXX introduced by this phase in either repo (re-scanned across the full phase surface this session). The later venue/CI commits likewise add no debt markers on the phase surface.

### Human Verification Required

None. The prior report's single item — hosted-CI observation of the pushed phase (net-smoke-x64 green + the 208ceba pin-advance observation) — is closed by direct evidence this session: gh API shows run 43121894436 (r9-shell, pushed master) completed `success` with the `net-smoke-x64` job `success`, three further r9-shell runs green since, and the run's tree carries the phase-16 surface byte-identical to current HEAD. The persisted 16-UAT.md item (created from the prior verdict) is satisfied by the same observation; its `result: [pending]` field may be closed by the owner as a bookkeeping step.

### Gaps Summary

No gaps. All five roadmap success criteria and all 43 plan truths re-verified against the current tree (not SUMMARY claims) with live behavioral evidence re-executed this session: the emitter probed on both the freshly-built binary at r9 208ceba and the nix-built store bin the flake wires (all three modes, unknown-mode rejection, and the WR-02 spaced/empty --pcap rejections, plus the WR-02 unit test green), the doctor on all three mode shapes on the live host, the boot script's four failure/inline-doctor shapes, the soak's slirp refusal both flag and env paths, host-echo recompiled with 8-way concurrency and dies-with-parent re-proven, and full marker asserts on both NETIP-08 verdict captures plus pcap magic-byte authenticity. The dual-repo contract held under re-check: both pin advances are lock-only own commits behind pushed r9 SHAs, and the current 208ceba pin's delta over the evidence SHA d1c4849 is the single WR-02 bin file — the retained evidence still describes the kernel bytes the pin builds. The milestone's networking exit gate (NETIP-08) stands proven on the tap leg. The open 1-in-5 observer-healed burst triple fault (deferred-items + WINDOWS #66) and the four Info-tier review findings remain recorded for Phase-17 triage per the review disposition; the PENDING FIRST PUSH lines in 16-EVIDENCE.md are now observationally satisfied and may be refreshed as a docs chore.

---

_Verified: 2026-11-12T13:24:42Z_
_Verifier: Claude (gsd-verifier)_
