---
phase: 08-r9-tcp-ip-path-devip-compatible-net
plan: 03
subsystem: infra
tags: [aarch64, plan9, bootfs, payloads, tcp, ipconfig, ping, udpecho, aan, tcpblast, tcpecho, netaddr, mk, bake]

# Dependency graph
requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net (plan 08-01)
    provides: the contract doc + supply chain record this plan's payloads serve (the gates assert the contract doc)
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: the payload precedent (07-04 arp.req/netblast.c — compile/harvest shape, magic contract, insertion-only block discipline), the /net mount-point bake (07-07), the od -j trailer extraction fix
  - phase: 02-aarch64-toolchain-libc-9front-donor
    provides: the 7c/7l aarch64 toolchain + libthread/libauth tier-0 closure aan links via #pragma autolib
provides:
  - build/aarch64/out/{ipconfig,ping,udpecho,aan,tcpblast,tcpecho} — all six Phase-8 gate binaries built for aarch64, harvested to out/, magic-asserted (00008c47), and baked into bootfs.paq with staged-manifest asserts (T-08-05)
  - build/aarch64/tcpblast.c — the NETIP-08 bidirectional TCP soak machinery (argv-driven host/port, 64-round TCG-sane default, 32 KiB patterned write/read/byte-compare, round/done markers)
  - build/aarch64/tcpecho.c — the NETIP-03 in-guest tcp/7 echo server (libc announce/listen/accept, announced/conn markers)
  - build/aarch64/netaddr.rc — the D-11 single addressing seam: leg-prefixed keys per leg (lo_/slirp_ v4 triples + slirp v6 pair + blastport), the file every later gate derives addresses from
affects: [08-04 (loopback gates consume tcpecho + the seam's lo_ leg), 08-05 (tcp proto gates), 08-06/08-07 (slirp static + DHCP gates source the slirp_ leg), 08-08 (aan + tcpblast soak rides slirp_gwv4:blastport), Phase 9 (the seam lifts into HNET-02 TOML; tcpblast runs the tap truth verdict)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as estimateTokens.
actuals:
  tokens: 6000     # (14546-char build/aarch64 diff across both task commits + ~9500-char this SUMMARY + ~400-char STATE/ROADMAP edits) / 4
  tasks: 2
  commits: 4       # MEASURED: 2 Harvey task commits (84fb37ab0, 499b4f00e) + 1 concurrent foreign commit in the window (30e51b215, docs(06) — not this plan's work) + 1 Harvey metadata commit (this file + STATE/ROADMAP)
plan_head_before: 8ced5ca833d49736dc852a33e0a04a796ce14ab5   # Harvey repo ledger base (#3968)
plan_head_after: 499b4f00ea4e802ca55323ee9fd6cc8b4fe5420e    # Harvey HEAD at SUMMARY write — the metadata commit follows this write per the atomic close-out order

# Tech tracking
tech-stack:
  added: [tcpblast.c + tcpecho.c payloads (u.h/libc.h only, 7c -FTVw + 7l compile), netaddr.rc (rc key=value seam)]
  patterns: [phase-owned bootfs payloads (D-12: phase-tagged insertion-only build blocks, never tier-allowlist appends), out/-harvest as the magic-contract surface beside bin/ install as the bake source, single-source-as-data addressing seam with leg-prefixed keys (D-11), magic+manifest double assert (subset-build host-side od check + mkimage staged-path assert)]

key-files:
  created: [build/aarch64/tcpblast.c, build/aarch64/tcpecho.c, build/aarch64/netaddr.rc]
  modified: [build/aarch64/subset-build.sh, build/aarch64/mkimage.sh]

key-decisions:
  - "netaddr.rc keys are leg-prefixed (lo_guestv4/slirp_guestv4/…): bare duplicate guestv4 keys would be last-wins-on-source, silently binding the slirp address into loopback gates — the prefixed shape keeps wholesale sourcing unambiguous and every acceptance grep passes on the substring (the file header documents the consumption contract)"
  - "out/ harvest is the payload magic-contract surface while /aarch64/bin stays the bake source: subset-build.sh asserts od magic on out/ post-rsync, mkimage.sh stages from bin/ and asserts staged paths pre-bake — a missing/stale payload fails loudly at both layers (T-08-05)"
  - "slirp v6 pair recorded provisional (fec0::15/fec0::2, qemu's default v6 shape) with the file's own comment naming 08-07's live probe as the finalizer — the plan's sanctioned sequencing: the FILE and its leg sections land now, values finalize at the gate that consumes them"

patterns-established:
  - "Phase-tagged payload block shape: `# 08-03:` comment + echo banner + (cd dir && objtype=aarch64 mk <target>) || fail + cp-to-out/ harvest — copyable by later phases"
  - "Six-name payload loop idiom in the assert tails (one `for f in …` loop per contract layer instead of per-payload line pairs)"

requirements-completed: []  # NETIP-03..08 are each shared with pending plans (08-04..08-09 per the phase's requirements rows) — the shared-ID gate (#2388) blocks completion marking until the last declaring plan finishes; no REQUIREMENTS.md change this plan

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The four full-tree gate binaries (ipconfig dir install; ping/udpecho per-target from cmd/ip; aan per-target from cmd linking libthread/libauth via autolib) built via # 08-03:-tagged insertion-only payload blocks, harvested to out/, magic-asserted, and baked with mkimage manifest asserts — Phase 6 tier data files untouched, consumer sources byte-unmodified (D-12/D-09)"
    requirement: NETIP-03
    verification:
      - kind: other
        ref: "sh build/aarch64/subset-build.sh && sh build/aarch64/mkimage.sh → BAKE-OK (in the nix devshell, lima leg)"
        status: pass
      - kind: other
        ref: "for f in ipconfig ping udpecho aan; do test -s out/$f + od magic == 00008c47; done → PAYLOADS-MAGIC-OK"
        status: pass
      - kind: other
        ref: "git diff --stat: subset-build.sh +38/-0 and mkimage.sh +19/-0 on Task 1 (insertion-only); build/aarch64/tiers/ clean; no sys/src/cmd edits in either task commit"
        status: pass
    human_judgment: false
  - id: D2
    description: "tcpblast.c (argv host/port, 64×32 KiB patterned write/read/compare rounds, round/done markers) + tcpecho.c (announce tcp!*!7, listen/accept echo loop, announced/conn markers) + netaddr.rc (D-11 seam: loopback + slirp legs incl. provisional v6 pair + blastport) — all compiled warning-free, baked, and magic-asserted"
    requirement: NETIP-08
    verification:
      - kind: other
        ref: "full bake re-run after Task 2 → BAKE-OK; for f in tcpblast tcpecho magic loop + netaddr.rc grep chain → PAYLOAD-AND-SEAM-OK"
        status: pass
      - kind: other
        ref: "host-side 7c -FTVw syntax probe of both payloads (devshell hostbin) → rc=0, zero warnings (USED(argv) idiom applied)"
        status: pass
      - kind: other
        ref: "grep -c '10\\.0\\.2\\|127\\.0\\.0\\.1' tcpblast.c → 0 (no compiled-in address); marker greps: 3× tcpblast round/done, 4× tcpecho announced/conn"
        status: pass
    human_judgment: false

# Metrics
duration: 8 min
completed: 2026-11-05
status: complete
---

# Phase 8 Plan 03: Gate Payloads + Addressing Seam Summary

**All six Phase-8 gate binaries (ipconfig, ping, udpecho, aan from the full tree; tcpblast + tcpecho as new payloads) built for aarch64 via phase-tagged insertion-only build blocks, harvested to out/, magic-asserted (00008c47), and baked into bootfs.paq with manifest asserts — plus the D-11 addressing seam (build/aarch64/netaddr.rc) landed with leg-prefixed keys so every later gate derives its addresses from one committed file.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-11-05T19:42:13Z
- **Completed:** 2026-11-05T19:50:59Z
- **Tasks:** 2
- **Files modified:** 5 (3 created + 2 modified, all under build/aarch64/)

## Accomplishments

- **Task 1 — the four full-tree gate binaries:** `ip/ipconfig` (dir install — idempotent beside its v1.0 tier-1 entry; the harvest to out/ is the new payload surface), `ip/ping` + `ip/udpecho` (per-target installs from cmd/ip via mkmany's `%.install` — suffix rules independent of the tier filter, so D-12's no-tier-append stance holds), and `aan` (per-target from cmd; links libthread/libauth via #pragma autolib off the fresh tier-0 archives). All four harvested to `build/aarch64/out/`, added to the darwin rsync-back list, magic-asserted in the subset-build tail, and asserted in mkimage.sh at three layers (out/ input, bin/ install, staged manifest). Bake green on the first run.
- **Task 2 — the new payloads + the seam:** `tcpblast.c` (the NETIP-08 machinery: argv-driven host/port, 64 TCG-sane rounds of 32 KiB round-patterned write/read/byte-compare, `tcpblast: round <i> ok` / `tcpblast: done <n> rounds` stdout markers, loud `exits("errors")` on any short/failed/mismatched round, no sleeps/no pacing) and `tcpecho.c` (NETIP-03's third leg: libc `announce("tcp!*!7")` → listen/accept echo loop, `tcpecho: announced` / `tcpecho: conn` markers, sysfatal %r on failures). Both compile warning-free (host-side 7c probe), bake beside netblast, and carry the same magic contract. `netaddr.rc` lands the D-11 seam: one commented section per leg with leg-prefixed keys (`lo_guestv4=127.0.0.1`; `slirp_guestv4=10.0.2.15`/`slirp_maskv4`/`slirp_gwv4=10.0.2.2`; provisional `slirp_guestv6=fec0::15`/`slirp_gwv6=fec0::2` pending 08-07's live probe; `blastport=7777`), the D-11 + Phase-9-TOML-lift header comment, and the consumption contract documented in-file.
- **Both bakes green end-to-end:** `BAKE-OK` + `PAYLOADS-MAGIC-OK` (Task 1) and `BAKE-OK` + `PAYLOAD-AND-SEAM-OK` (Task 2); all six `out/` payloads carry `00008c47` arm64 a.out magic; the final image bakes at 4,172,943 bytes with header/trailer magics intact.

## Task Commits

Each task was committed atomically (Harvey repo — this plan is Harvey-side only):

1. **Task 1: the four full-tree gate binaries + bake asserts** — `84fb37ab0` (feat)
2. **Task 2: tcpblast + tcpecho + netaddr.rc + bake** — `499b4f00e` (feat)

**Plan metadata:** (this SUMMARY + STATE/ROADMAP — committed after this write per the atomic close-out order)

_Note: a concurrent session's `30e51b215` (docs(06)) landed in the ledger window between the two task commits — not this plan's work; recorded here so the measured `commits:` count reconciles._

## Files Created/Modified

- `build/aarch64/tcpblast.c` — NEW: the NETIP-08 bidirectional TCP soak client (77 lines)
- `build/aarch64/tcpecho.c` — NEW: the NETIP-03 tcp/7 echo server (53 lines)
- `build/aarch64/netaddr.rc` — NEW: the D-11 single addressing seam (42 lines)
- `build/aarch64/subset-build.sh` — `# 08-03:` blocks (4 mk-based installs + 2 compile/harvest blocks), rsync-back list extended by six, magic-assert tail now loops the six payloads (+57/-0 across both commits)
- `build/aarch64/mkimage.sh` — input asserts for the six out/ payloads + the three new bin/ install locations, staging cps for ping/udpecho/aan/tcpblast/tcpecho, five new staged-manifest asserts (+29/-2 across both commits)

## Decisions Made

- **Leg-prefixed seam keys** (`lo_`/`slirp_`): the plan text's bare duplicate `guestv4` assignments would be last-wins on source — wholesale sourcing (the stated consumption mode) would bind 10.0.2.15 into loopback gates. Prefixed keys keep every leg unambiguous, satisfy every acceptance grep (substring match), and the in-file header documents the contract for 08-04..08-08's sweeps. (Deviation 1)
- **out/ harvest = magic-contract surface; bin/ install = bake source** — the two-layer assert (host od check post-rsync + staged-path manifest pre-bake) makes a missing/stale payload fail loudly at whichever layer it disappears (T-08-05).
- **Slirp v6 values provisional** (`fec0::15`/`fec0::2` — qemu slirp's default v6 shape) with the finalizer named in-file (08-07's live probe), per the plan's sanctioned sequencing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] netaddr.rc's bare per-leg `guestv4` duplicates would be last-wins-on-source**
- **Found during:** Task 2 (writing the seam file)
- **Issue:** the plan's action text assigns bare `guestv4` in both the loopback and slirp sections; rc sourcing binds the last assignment, so any sweep sourcing the whole file for the loopback leg would silently get the slirp address — recreating Pitfall 9's failure mode inside the very file meant to kill it
- **Fix:** leg-prefixed keys (`lo_guestv4`, `slirp_guestv4`, `slirp_maskv4`, `slirp_gwv4`, `slirp_guestv6`, `slirp_gwv6`) with the consumption contract documented in the file header; all plan acceptance greps pass unchanged (substring matches), and the must_haves `contains: guestv4` holds
- **Files modified:** build/aarch64/netaddr.rc (content decision)
- **Verification:** the full acceptance grep chain (`guestv4=127.0.0.1`, `10.0.2.15`, `blastport=`, v6 keys, D-11 header) green
- **Committed in:** `499b4f00e` (part of Task 2)

**2. [Rule 1 - Bug] tcpecho.c compile warning (unused argv)**
- **Found during:** Task 2 host-side 7c syntax probe
- **Issue:** `main(int argc, char *argv[])` with only argc used tripped `-w` ("param declared and not used")
- **Fix:** `USED(argv);` after the argc check — the tree's compiler-builtin idiom
- **Files modified:** build/aarch64/tcpecho.c
- **Verification:** re-probe warning-free, rc=0
- **Committed in:** `499b4f00e` (part of Task 2)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — one plan-text defect corrected in the direction D-11 exists to enforce, one warning fix in my own new code)
**Impact on plan:** No scope creep; no locked decision changed; the seam file is strictly more truthful to its own purpose.

## Issues Encountered

- **gsd_run/gsd-tools shim unavailable** in this environment (same as 08-02) — STATE.md and ROADMAP.md were hand-edited to the same fields the verbs maintain (position, metric row, decisions, progress count; roadmap plan-count flip 1/9 → 3/9). `requirements.mark-complete` was correctly a no-op anyway: all six declared IDs are shared with pending plans (shared-ID gate #2388).
- **A concurrent session committed `30e51b215` (docs(06)) between this plan's two task commits** — no conflict (disjoint files); noted in Task Commits for ledger reconciliation.
- None otherwise: both bakes ran green on their first attempt; no consumer source needed a compile fix (the P3/NETIP-02 prohibitions stayed vacuous).

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **08-04 unblocked Harvey-side:** the baked image now carries `/aarch64/bin/ip/ping`, `/aarch64/bin/ip/udpecho`, `/aarch64/bin/tcpecho`, and `/aarch64/bin/ip/ipconfig`; the loopback sweeps source `netaddr.rc`'s `lo_` leg (or reference 127.0.0.1 through it) — no gate needs a hardcoded address.
- **08-06/07/08 unblocked Harvey-side:** `/aarch64/bin/aan` and `/aarch64/bin/tcpblast` baked; the slirp leg's v4 triple + `blastport=7777` are the single source for the static/DHCP/soak fixtures; the provisional v6 pair finalizes at 08-07's live probe as recorded in-file.
- **r9 untouched this plan** (Harvey-side only as designed); r9's pre-existing uncommitted `aarch64/src/exec.rs` modification was never touched.
- No blockers.

## Self-Check: PASSED

- `build/aarch64/tcpblast.c`, `tcpecho.c`, `netaddr.rc` exist and non-empty — FOUND
- `build/aarch64/subset-build.sh` carries 10× `# 08-03:` tags; `mkimage.sh` carries 4 tagged sites — VERIFIED
- All six `build/aarch64/out/{ipconfig,ping,udpecho,aan,tcpblast,tcpecho}` exist with `00008c47` magic post-bake — VERIFIED (od loop)
- Commits `84fb37ab0` and `499b4f00e` exist on master (`git log --oneline`) — FOUND
- Insertion-only discipline: Task 1 diff +57/-0; Task 2's four deletions are this plan's own Task-1 lines extended to the six-name loops (no 07-era or tier-machinery lines edited/deleted) — VERIFIED (`git diff` line inspection)
- Phase 6 tier data files (`build/aarch64/tiers/`) untouched — VERIFIED (git status clean)
- Consumer sources (`sys/src/cmd/ip/ipconfig/`, `ping.c`, `udpecho.c`, `aan.c`, libc dial/announce) byte-unmodified — VERIFIED (no sys/src edits in either commit)
- Both full bakes exited 0 with BAKE-OK and the payload grep chains green — VERIFIED at execution time

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Completed: 2026-11-05*
