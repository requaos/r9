# Phase 16 Evidence Index — Host Net Modes + Networking Exit Gate

**Phase:** 16-host-net-modes-networking-exit-gate
**Consolidated:** 2026-11-11 (plan 16-07, task 2)
**Status:** complete — every requirement truth (HNET-01..HNET-05, NETIP-08) mapped to the gate runs and probes that prove it, at the pinned r9 SHA, with the retained artifacts under the phase evidence dir (the table below). Nothing below is claimed green that was not run: every cited capture exists at its named path and was re-verified to carry its markers at consolidation time; where a verdict lived on a driver's console rather than a retained wire capture — or a leg is documented-only, or an observation is PENDING FIRST PUSH — this file says so.

## The NETIP-08 verdict (the phase's joint acceptance gate)

**GREEN, twice consecutively, on the tap leg — and the tap captures are the sole accepted evidence.** `evidence/16-05-netsoak-a.log` and `evidence/16-05-netsoak-b.log` (with their `16-05-netsoak-a.pcap.gz` / `-b.pcap.gz` wire captures) are the verdict record: two back-to-back full runs of `R9_NET=tap nix run .#boot-r9-net-soak-x64`, both ALL STAGES PASS / exit 0, both riding **r9 `d1c4849dfa1d769240630c29b9b03b88d5819558`** (the flake.lock pin — asserted unchanged through 16-08). Per-run asserts (re-verified on both files at consolidation): exactly 9 `tcpblast: done` (1 baseline 2048-round + burst 8×512), zero `netip: FAIL`, the `/net/arp` view ok, `netip: L2-TRUTH tap venue` present, `netip: NETIP-DONE` present; the peer's byte accounting exact on every connection both directions both runs (67108864 = 2048×32768 baseline; 8× 16777216 = 512×32768 burst). Slirp evidence is refused by the gate itself — the app's parse-time refusal, verbatim:

> `slirp evidence not accepted — the NETIP-08 soak verdict is the tap leg's (slirp is not L2 truth by contract and its captures say so); want --net=tap / R9_NET=tap; refusing before any bake or boot (D-11's venue-honesty control)`

One kernel finding surfaced during the verdict runs stays OPEN (never a blind fix): the intermittent burst-leg guest triple fault, 1 failure in 5 full runs, observer-healed — recorded in `deferred-items.md` + WINDOWS #66 with the reproduction statistics and the recommended stress-loop shape (rounds=1/burstn=8, uninstrumented pinned kernel, `-d int,guest_errors`). The failed run's captures were deliberately NOT retained (only green runs earn the evidence names); the signature is recorded in prose in the deferred item.

## Pinned commits and the pin-advance lineage

| Repo | SHA | Role |
|------|-----|------|
| r9 (`requaos/r9`, branch `main`) | **`d1c4849dfa1d769240630c29b9b03b88d5819558`** | the phase's kernel pin — 16-01's five pushed commits (the netdev mode tables, the ONE arg-builder, the `r9-netdev-args` export). Fully pushed (`origin/main..HEAD` = 0); Harvey's `flake.lock` pins exactly this rev. |
| harvey | `45d4436f8` | the pin advance `af52fee → d1c4849` as its OWN lock-only commit (16-04 Task 1; the own-commit rule — nothing bundled with the advance) |
| harvey | `e79a587e4` | 16-07's net-modes README (`build/x86_64/README.md`) |
| harvey | this docs commit | 16-EVIDENCE.md + the 16-07 SUMMARY (the consolidation you are reading) |

**The lineage, with the own-commit rule at each hop:** `af52feef7620137c43c5e04d077819fbcf3ddb82` (the Phase-15 close pin) → **16-01** pushes `cb96d56`..`d1c4849` to r9 (push-before-pin: every r9 commit on `origin/main` before any Harvey pin names it) → **16-04** advances Harvey's lock to `d1c4849` in the dedicated lock-only commit `45d4436f8` → **16-08 fix SHAs: NONE.** The soak's first-run failures were root-caused (debugcon instrumented autopsy) as the HARVEY-side gate's own 600s stage budget killing the guest mid-baseline; r9 was exonerated and left byte-identical at `d1c4849` (tree clean, pushed, `git -C ../r9 status` clean; unpushed count 0) — no kernel fix, no FOD re-wire, no pin advance (16-08's push-before-pin and FOD acceptance clauses vacuously satisfied).

**Per-plan commit trail (all ancestors of the pinned HEADs; r9 commits live in the sibling repo):**

| Plan | Commits | Closed |
|------|---------|--------|
| 16-01 (r9 emitter + mode tables) | r9 `cb96d56`..`d1c4849` (5) | HNET-02/HNET-04 machinery halves |
| 16-02 (tap_ seam + netip-tap fixture) | `0ddaa88a2`, `d28134991` | NETIP-08 guest half; HNET-03 seam |
| 16-03 (net-doctor) | `912bb8ee8`, `c4cf19d27` | HNET-05 |
| 16-04 (pin advance + --net surface) | `45d4436f8`, `209d29cdd`, `116fd7f2f` | HNET-01 local half; HNET-02 surface |
| 16-05 (soak machinery) | `9e83e6220`, `c787fafea` | NETIP-08 machinery |
| 16-06 (net gate on emitter argv + CI job) | `f01450794`, `4bac09784` | HNET-01 local+authored-CI halves |
| 16-08 (the verdict runs) | `0a29da7d9`, `c2b78b7bf` (+ trailing `2a7c3a9b0` gitignore chore) | **NETIP-08 verdict; HNET-03 tested-on-host** |
| 16-07 (this consolidation) | `e79a587e4` + this commit | HNET-03/HNET-04 documented halves |

## Green-twice declarations per gate

| Gate | Declaration | Evidence |
|------|-------------|----------|
| `boot-r9-net-smoke-x64` (slirp venue, FIXED per D-02) | green TWICE locally on KVM after the emitter re-key (16-06): both runs exit 0, 47 stage-ok lines each, both hostfwd-listening observation stages ok; the re-verified retained pair carries `netip: NOT-L2-TRUTH slirp venue` ×2 (static + dhcp legs) and `netip: NETIP-DONE` each | `evidence/16-06-netsmoke-a.log`, `evidence/16-06-netsmoke-b.log` |
| `R9_NET=tap boot-r9-net-soak-x64` (the NETIP-08 verdict leg) | green TWICE consecutively (16-08): ALL STAGES PASS, exit 0, byte-exact accounting both directions both runs, zero `netip: FAIL`, ARP L2-truth + venue label + NETIP-DONE in both | `evidence/16-05-netsoak-a.log`, `evidence/16-05-netsoak-b.log` + the pcaps |
| default slirp boot (HNET-01's hermetic proof) | one captured full-pipeline unprivileged boot to the rc `%` prompt (16-04) — a single-run boot proof by design (the twice-green discipline belongs to the gates, not the interactive boot); both hostfwd tokens in the echoed argv, zero `0.0.0.0` literals | `evidence/16-04-boot-slirp.log` |

**Precision note (what the retained captures carry):** the netsmoke/netsoak captures are the spawned pty's guest-console output — they carry the guest-side markers (venue labels, `tcpblast: done`, NETIP-DONE, the banner/prompt), NOT the host-side driver lines (`stage ok — hostfwd-…-listening`, the spawn argv echo); those printed on the driver consoles at run time and are recorded in the 16-06/16-08 SUMMARYs. The 16-04 boot capture is the exception by construction (the boot script's own stdout was captured), so it alone shows the host-side argv echo — including the emitter store path `r9-netdev-args-d1c4849`, the pin's own name in the evidence.

## Retained artifacts (evidence/ — every indexed file exists on disk; 7 files)

| File | What it is | r9 SHA ridden |
|------|------------|---------------|
| `evidence/16-04-boot-slirp.log` | the HNET-01 hermetic-default boot proof: full pipeline unprivileged to the `%` prompt, argv echo carrying `hostfwd=tcp:127.0.0.1:17010-:17010` + `hostfwd=tcp:127.0.0.1:5670-:567` and the `r9-netdev-args-d1c4849` emitter path (25584 bytes) | `d1c4849` (via the flake.lock pin; the emitter path names it) |
| `evidence/16-06-netsmoke-a.log` | net-smoke run A on KVM (the emitter-argv re-key): NOT-L2-TRUTH ×2, NETIP-DONE | `d1c4849` |
| `evidence/16-06-netsmoke-b.log` | net-smoke run B on KVM (the twice-green pair) | `d1c4849` |
| `evidence/16-05-netsoak-a.log` | NETIP-08 verdict run A (green): 9× `tcpblast: done`, zero FAIL, arp view ok, L2-TRUTH tap venue, NETIP-DONE | `d1c4849` |
| `evidence/16-05-netsoak-b.log` | NETIP-08 verdict run B (green, consecutive with A) | `d1c4849` |
| `evidence/16-05-netsoak-a.pcap.gz` | run A's wire capture (filter-dump retained by the gate; gzip, 450MB→16MB, md5 round-trip proven against the raw pcap) | `d1c4849` |
| `evidence/16-05-netsoak-b.pcap.gz` | run B's wire capture (same discipline) | `d1c4849` |

**Records that live outside evidence/ (cited, not on-disk captures):**

- The doctor's live outputs — including the PRE-remediation `FAIL bridge-conf-allow` (`allow virbr0`-only state) and the WARN masquerade line — are quoted verbatim in the 16-03 SUMMARY's "Live Evidence" section (the run-time logs were session artifacts; the SUMMARY block is the durable record).
- The 16-02 negative-control pair (doctored-literal RED bake → exit 1 naming file+line; clean GREEN bake → 5 images + the netip-tap manifest line) is recorded in the 16-02 SUMMARY's "Negative Control Record".
- The 16-08 failed-run signature (the burst triple fault) is in `deferred-items.md` + WINDOWS #66 and the 16-08 SUMMARY — intentionally NOT an evidence capture.
- The peer's per-connection accounting lines (`hostecho: closed …`) printed to the peer log in the run's retained scratch dir, consolidated into the 16-08 SUMMARY's byte-exact table (67108864 + 8×16777216, both directions, both runs).

## Requirement → proof mapping

- **HNET-01 (slirp+hostfwd hermetic default, CI-able):** the captured unprivileged boot (`evidence/16-04-boot-slirp.log`, loopback-bound pair in the argv echo); the twice-green net-smoke with its hostfwd-listening observation stages (`evidence/16-06-netsmoke-a/b.log` + the 16-06 SUMMARY's driver-console record); the hosted-CI leg AUTHORED (r9-shell.yml `net-smoke-x64`, actionlint-clean, pinned actions) — the hosted observation is PENDING FIRST PUSH (below).
- **HNET-02 (`--net` selection, netdev single-sourced as data):** 16-01's machine-checked TOML mode tables + ONE builder (35 lib tests + the all-five-TOMLs include_str! parse guard, r9 repo); 16-04's live failure-shape proofs (unknown mode, cross-venue, no-`=` form — all exit 1 with the named vocabulary) and `grep -c 'netdev user' nix/boot-r9-x64.sh` == 0 (no netdev literal anywhere Harvey-side; the gates' drivers consume `R9_NETDEV_ARGV` the same way).
- **HNET-03 (linux tap/bridge leg, per-distro documented, tested on the primary host):** the seam + fixture (16-02, negative-control-proven bake assert); the doctor (16-03); the remediation APPLIED on the primary host (bridge.conf `allow br0` + `trustedInterfaces` — the colmena config); the green-twice verdict captures are the tested-on-primary-host proof. The documented halves: `build/x86_64/README.md` (16-07) — NixOS page TESTED, generic-Linux page DOCUMENTED NOT TESTED, real-LAN paragraph NOT TESTED.
- **HNET-04 (darwin socket_vmnet, documented-manual):** the vmnet TOML table + emitter arm (16-01, r9 unit-tested); the README darwin page (16-07) with the A4-flagged `vmnet-socket,path=` argv (corroborated-not-probed) and the fd-passing alternative; the doctor's INFO n/a line on linux. Never executed — by construction (no darwin runner this milestone).
- **HNET-05 (the doctor):** 16-03's live-host verdicts (slirp 3 PASS + DOCTOR-OK exit 0; tap FAIL bridge-conf-allow + WARN masquerade exit 1; unknown-mode usage error), the read-only audit (every mutation-shaped line is print payload), and the byte-identity of the two front-ends (bare script vs flake app, slirp AND tap paths).
- **NETIP-08 (the exit gate):** the verdict statement above.

## The flagged-assumption outcomes (A1–A7 from 16-RESEARCH.md)

| Assumption | Outcome | Evidence |
|------------|---------|----------|
| A1: qemu binds slirp hostfwd listeners eagerly at netdev init (a <1024 host port aborts startup) | **CONFIRMED** — both listeners provably exist by banner time (the 16-06 hostfwd observation stages ran mid-boot), and the 5670-remapped default boots unprivileged; the eager-bind backstop never had to fire | 16-06 runs + SUMMARY; `evidence/16-04-boot-slirp.log` |
| A2: br0's existing 192.168.122.1/24 is stable/declarative (colmena-managed) | **CONFIRMED** — br0 at the committed subnet held through every run of the phase; the remediation delta (allow line + firewall) was applied through the colmena config and survived | the 16-08 runs' SYNs answered at host .1; 16-08 SUMMARY |
| A3: the masquerade rule for br0's subnet is absent on this host (no observation either way) | **STILL UNOBSERVED** — the netfilter read stayed WARN-class all phase (privileged); no committed gate dials past the default route (the soak terminates at the host peer on the bridge IP), so nothing exercises outbound NAT | the doctor WARN; the honest-boundary note in the README's NixOS page |
| A4: `-netdev vmnet-socket,path=` exact spelling (qemu ≥ 9.0, docs sparse) | **NOT PROVEN — stays flagged** — no darwin runner exists; the README darwin page marks the emitted form corroborated-not-probed and names the fd-passing alternative | `build/x86_64/README.md` (16-07) |
| A5: a host-std Rust bin builds under the r9 flake's toolchain pattern (kernel-x64 is no_std; the emitter is the first host-std package) | **CONFIRMED** — `packages.netdev-args` builds green (`nix build .#netdev-args`), the cargoSetupHook+plain-`cargo build --release` shape worked first try | 16-01 SUMMARY |
| A6: NixOS `networking.nat`/`networking.bridges` render the intended shape on the real host | **SPLIT** — the bridges/interfaces/sysctl/trustedInterfaces lines are proven live (trustedInterfaces was load-bearing: its absence was the 16-08 wedge root-cause); the `networking.nat` MASQUERADE rendering is NOT exercised by any committed gate and stays unproven (WARN class) | 16-08 wedge diagnosis + deferred-items doctor-gap entry; the README's honest-boundary note |
| A7: the colmena-managed host accepts environment.etc/security.wrappers additions | **CONFIRMED for the applied delta** — the `allow br0` bridge.conf and `trustedInterfaces` lines went through colmena and took effect (the soak's traffic flowed); the setuid wrapper itself pre-existed via the libvirt-era setup and its exact colmena source remains unlocated (the README documents it as such) | the green-twice runs; README NixOS page |

## PENDING FIRST PUSH (the carried honesty convention)

Hosted observations that cannot execute from this checkout pre-push — both land with the phase's Harvey commit push (the standing convention since 16-04; the orchestrator/user owns the push):

1. **The pin-advance hosted-CI observation (16-04):** Harvey's production commits of the pin advance (`45d4436f8`) are local to `master` until the push; the local eval battery at the new pin (all six x64 apps green) is the standing evidence.
2. **The net-smoke-x64 hosted-CI observation (16-06):** the r9-shell.yml job is authored, actionlint-clean, and runs the app that is green twice on this host; its first hosted green run is pending.

## Venue-honesty recap

- Every slirp capture keeps its NOT-L2-TRUTH label: `evidence/16-06-netsmoke-a.log` and `-b.log` each carry `netip: NOT-L2-TRUTH slirp venue` twice (static + dhcp legs) — re-verified at consolidation. The 16-04 boot capture is a boot proof (banner → `%` prompt), not a netip sweep — it carries no venue label and claims no networking truth beyond the argv echo.
- The soak's venue labels are the asserted inverse: `netip: L2-TRUTH tap venue` — one per run, present in both netsoak captures.
- The soak's slirp refusal is quoted at the top of this file; it fires at parse time BEFORE the doctor preflight (the ordering that keeps it reachable on wrong-allow hosts).
- Documented-only legs (generic Linux, darwin socket_vmnet, the real-LAN bridge variant) claim nothing: each carries its explicit not-tested label in `build/x86_64/README.md`, and no evidence row above cites them.

## Open items carried past the phase (none block the verdict)

- The intermittent burst-leg guest triple fault (1-in-5, observer-healed) — `deferred-items.md` + WINDOWS #66; the recommended stress-loop shape and the proven debugcon toolkit are recorded there.
- The doctor-gap firewall under-report (the WARN-class masquerade read cannot surface `trustedInterfaces` on privileged-read hosts) — `deferred-items.md`; the README (16-07) already documents the firewall opening as mandatory checklist material pending any doctor fix.
- r9 xtask pcap.rs dead-code warnings (4, pre-existing) — `deferred-items.md` (16-01).
- The Phase-18 policy revisit for real-key capture retention (T-16-18's accept disposition): this era's pcaps carry soak traffic only; the flag rides to Phase 18 with the auth work.

---
*Phase: 16-host-net-modes-networking-exit-gate*
*Consolidated: 2026-11-11*
