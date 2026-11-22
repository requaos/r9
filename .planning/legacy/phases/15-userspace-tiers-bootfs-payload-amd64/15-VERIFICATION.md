---
phase: 15-userspace-tiers-bootfs-payload-amd64
verified: 2026-11-10T21:16:33Z
status: passed
score: 4/4 must-haves verified
covered_files: [".github/workflows/phase6.yml", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-01-PLAN.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-01-SUMMARY.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-02-PLAN.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-02-SUMMARY.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-03-PLAN.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-03-SUMMARY.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-04-PLAN.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-04-SUMMARY.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-05-PLAN.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-05-SUMMARY.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-06-PLAN.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-06-SUMMARY.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-07-PLAN.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-07-SUMMARY.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-08-PLAN.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-08-SUMMARY.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-09-PLAN.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-09-SUMMARY.md", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-01-scoping-proof.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-02-donor-services-serial.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-03-services-runA.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-03-services-runB.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-03-tier1-flip.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-acme-raw.txt", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-acme-table.txt", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-distill-repro.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-rio-raw.txt", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-rio-table.txt", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-sam-raw.txt", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-sam-table.txt", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-trace-venue.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-06-venue-bringup.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-07-gs-smoke.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-08-phase-close-tier1.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-08-services-final.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-08-services-payload-boot.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-review-fix-gs-smoke-negative.log", ".planning/phases/15-userspace-tiers-bootfs-payload-amd64/evidence/15-review-fix-trace-venue.log", "build/mm/boot-9k10t1-sweep.sh", "build/mm/boot-9kdraw-trace.sh", "build/mm/boot-9kgs-smoke.sh", "build/mm/build-9k10t1.sh", "build/mm/build-9kdraw.sh", "build/x86_64/bootfs-payload.txt", "build/x86_64/coverage-sweep.sh", "build/x86_64/dir-coverage.txt", "build/x86_64/drawsmoke.c", "build/x86_64/gs-render.ps", "build/x86_64/mkimage.sh", "build/x86_64/replay-acme.rc", "build/x86_64/replay-rio.rc", "build/x86_64/replay-sam.rc", "build/x86_64/root-tree/lib/ndb/local.template", "build/x86_64/root-tree/usr/glenda/lib/profile", "build/x86_64/services-r9.expect", "build/x86_64/services-sweep.rc", "build/x86_64/services-termrc", "build/x86_64/subset-build.sh", "build/x86_64/svctcpecho.c", "build/x86_64/tier1-sweep.rc", "build/x86_64/tier2.ledger", "build/x86_64/tier3.ledger", "build/x86_64/tiers/tier0-libs.txt", "build/x86_64/tiers/tier2-services.txt", "build/x86_64/tiers/tier3-gui.txt", "build/x86_64/tiers/tier4-tail.txt", "build/x86_64/trace-distill.sh", "flake.lock", "flake.nix", "nix/services-r9-x64.sh", "sys/src/9k/k10/k10draw", "sys/src/9k/k10/k10t1", "sys/src/9k/k10/multiboot.c", "sys/src/9k/k10/root/k10draw.rc", "sys/src/9k/k10/root/k10t1.rc", "sys/src/9k/k10/vgadispi.c", "sys/src/9k/port/devcons.c", "sys/src/9k/port/devroot.c", "sys/src/cmd/auth/factotum/mkfile", "sys/src/cmd/auth/mkfile", "sys/src/cmd/aux/mkfile", "sys/src/cmd/ip/mkfile", "sys/src/cmd/mkfile", "sys/src/libmemdraw/addr.c", "sys/src/libmemdraw/arctest.c", "sys/src/libmemdraw/draw.c", "sys/src/libmemdraw/drawtest.c", "sys/src/libmemdraw/mkfile", "sys/src/liboventi/mkfile", "sys/src/mkfile"]
covered_digest: "v4:sha256:aa5dcc6eeff22673c853154667af8cec8c3f5b7fd06dc1a5cc6007d3999076ea"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 15: Userspace Tiers + Bootfs Payload (amd64) — Verification Report

**Phase Goal:** The services/GUI/long-tail tiers ride the re-keyed machinery — session daemons, the IP suite, and the auth suite compile and donor-execute on k10t1 then r9-x86_64; the GUI stack compiles with measure-first donor syscall traces captured; the desktop bootfs payload is staged so Phases 18-19 never block on payload.
**Verified:** 2026-11-10T21:16:33Z
**Status:** passed
**Re-verification:** No — initial verification (no prior `*-VERIFICATION.md` existed)

**Evidence model note.** This phase's runtime proofs are multi-hour qemu gates (donor boots, r9 composed gates, the draw trace venue). Per the verification contract for this phase, their committed captures under `evidence/` are the evidence; this verification sample-verified markers inside those captures, cross-checked their internal consistency (boot output, probe observation lines, timestamps, git-pinned provenance), and independently re-derived the distilled trace tables from the committed raw logs (see Behavioral Spot-Checks). The r9 kernel fixes live in the sibling repo `/home/req/code/r9` and were verified against its git log and Harvey's `flake.lock` pin.

## Goal Achievement

### Observable Truths (roadmap success criteria, merged with plan must-haves)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | **SC1/USPACE-07** — the services tier executes on k10t1 first, then r9-x86_64, with committed execution markers | ✓ VERIFIED | **Compile:** `build/x86_64/tiers/tier2-services.txt` (68 data lines, 65 entries per SUMMARY — ip suite incl. dhcpd/ftpfs/httpd/ppp/imap4d/snoopy, ndb, auth suite 32 singles + secstore, exportfs, dial, mntgen, srv, aux:listen) through the six filter sites (all six read `$TIER1A $TIER2A $TIER3A $TIER4A` in ONE multi-file awk — `sys/src/mkfile:94`, `sys/src/cmd/mkfile:47`, `cmd/ip/mkfile:50`, `cmd/auth/mkfile:65`, `cmd/auth/factotum/mkfile:27` T1KEEPTA, `cmd/aux/mkfile:72-73` T2KEEPTA/T2KEEPDA leaf filter); `subset-build.sh` carries per-entry `mk clean` (lines 371/381/451/461), T2-ART sha256 + `00008a97` magic checks, `TIER-BUILD-OK`. The aux leaf filter and the first `cmd/aux:` entry landed in the SAME commit `c88d05b8e` (git -S verified both). Scoping proof: `evidence/15-01-scoping-proof.log` EXP==MK sets (8/8). **Donor execution:** `evidence/15-02-donor-services-serial.log` — real k10t1 boot, 9 SVC-OK subjects with interaction observations (SVC-CSOBS ip=127.0.0.1, SVC-DNSOBS probehost.test→127.0.0.1, SVC-LISTENOBS tcp9901 round-trip, SVC-FACTOBS masked-key readback, SVC-AUTHOBS connected, keyfs readnvram zero-key "1 keys read"), SVC-DONE, zero SVC-FAIL. **r9 execution:** `evidence/15-03-services-runA.log` + `runB.log` — GRUB→r9 boots, same 9 SVC-OK + SVC-DONE, twice-green; `flake.nix:272` `boot-r9-services-x64` app wired to `nix/services-r9-x64.sh` whose SVC expectations are COMPUTED from tier2-services.txt (lines 156-190). **Markers committed:** `build/x86_64/tier2.ledger` — 13 rows, every one `exec=SVC-OK:k10t1+SVC-OK:r9x64`, zero SKIP rows. **D-01 fixes:** r9 commits `e045dde`(rfork RFFDG), `06f8aa1`(devmnt Twrite), `21d2350`(Tcreate+DMDIR), `de1fba1`(devproc private/noswap), `242f228`(heap 4→32MiB), `2a31fcc`(test-only), FOD re-wire `3362cf7` — all in `/home/req/code/r9` git log; `flake.lock` pins `af52feef…`. **SKIP flips:** commit `7a9ad4b70` carries tier1-sweep.rc flips + expect + drivers together (Pitfall 4); `evidence/15-03-tier1-flip.log`: T1-OK ipconfig + T1-OK factotum + T1-DONE (38 T1-OK), re-proven at phase close (`15-08-phase-close-tier1.log`). |
| 2 | **SC2/USPACE-08** — GUI stack compiles for amd64; donor syscall traces for rio/acme/sam captured and committed | ✓ VERIFIED | `tiers/tier3-gui.txt` carries cmd/rio, cmd/acme, cmd/sam, cmd/samterm, cmd/plumb (+ utility class, 11 entries); libdraw family appended to tier0-libs.txt; T3-ART markers in subset-build.sh (lines 410-422) with the D-07 compile-only boundary in the marker vocabulary. **Venue:** k10draw config (`sys/src/9k/k10/k10draw`, DISPI vga section), `vgadispi.c` (VGAdev at :162, ports 0x1CE/0x1CF, vgalinearpci), `-initrd` channel (`multiboot.c:83-112` Fmods→addbootfile), eve-gated kbin (`devcons.c:783-787` open, `:1123-1129` write — both `if(!iseve())`), `devroot.c:14` Nbootfiles=64. **Traces:** raw + distilled for all three — `evidence/15-06-{rio,acme,sam}-raw.txt` (1922/3779/2193 lines) + `-table.txt` (real syscall counts, e.g. rio Pwrite 140 with lucm font path arg); all four venue markers (DRAW-SMOKE-OK, FONT-CHANNEL-OK, KBIN-OK, MOUSEIN-OK) + RIO/ACME/SAM-SCRIPT-DONE + DRAWTEST-OK + ARCTEST-OK present in `15-06-venue-bringup.log`, `15-06-trace-venue.log`, and the post-review re-runs. **15-09 re-join:** root-cause fixes verified as real class commits — `506ca9605` (drawtest round-to-nearest compositing, diff shows the +127 rounding on both compare sides), `e8df41c7d` (arctest byteaddr sign-extension), `9ecac570b` (stale vttest/test targets removed; real $O.drawtest/$O.arctest retained). **Verifier independently reproduced all three distilled tables byte-identical from the committed raws** (see spot-checks). `tier3.ledger`: TRACE pointers on rio/acme/sam(+samterm), utilities honestly `exec=STAGED:draw-venue-payload`. |
| 3 | **SC3/USPACE-09** — long tail compiles full-set; fossil+venti compile-gated + documented; gs included | ✓ VERIFIED | `dir-coverage.txt`: 702/702 rows `build=ok`, zero not-ok; driver `coverage-sweep.sh` (650 lines) with COVERAGE_SWEEP=1 passthrough, VISIT/EXIT markers, `timeout -k` watchdog. Exclusions: exactly 7 named `# exclude:` records in `tier4-tail.txt` (picpack/postbgi/postdaisy/postdmd/postmd upstream-disabled TARGETS class with measured type errors; tcs/font dead mkfile; games/blabs dead toolchain) — re-derived for amd64, not carried from aarch64; the 'x86-hardware-only' class flipped to MEMBERS (aux/vga, realemu, na, gps, mnihongo, flashfs). fossil+venti runtime-unverifiable note committed in three places: tier4 entries (lines 110/169), dir-coverage trailing comments (lines 499/621/622), SUMMARY. gs: tier4 entry `cmd/gs`, render smoke `evidence/15-07-gs-smoke.log` — GS-BIN-PRESENT, GS-BITS 43269, GS-RENDER-OK, GS-SMOKE-DONE on a real 9k donor boot; harness `boot-9kgs-smoke.sh` gates on GS-EXIT-OK + GS-RENDER-OK + GS-BITS>0 + zero panic (lines 343-359) and stages only `.ps` init files from `sys/src/cmd/gs/lib` (no font binaries anywhere). Negative control `15-review-fix-gs-smoke-negative.log` shows the gate FAILing (GS-GATE-FAIL) on a doctored scenario — the asserts bite. The ppmraw-vs-plan-example-'bit' device deviation is documented in 15-07-SUMMARY with the measured ld.tr reason and stays inside the plan's own named FILE-device class. |
| 4 | **SC4/DESK-03** — bootfs payload staged with manifest assertions | ✓ VERIFIED | `build/x86_64/bootfs-payload.txt`: [paths]/[fonts]/[floors] blocks; four app-named fonts (lucm/unicode.9, lucidasans/euro.8, pelm/latin1.8, pelm/euro.9) all present in `lib/font/bit` (ls-verified); floors `font-files 4303` / `font-bytes 27000000` against the measured tree (4303 files, 27676353 bytes — verifier re-measured, exact match). `mkimage.sh` `stage_payload()` (lines 631-750): whole-tree bake-time copy + staged==source count equality + token substitution from the netaddr seam with both-direction asserts + per-path/per-font presence + floors + the D-17 profile shape asserts (terminal rio-free, cpu case exec rio + latin1.8 verbatim). WIRED: called at line 887 by the services contract (`stage_payload "$STAGE5" services`). `root-tree/lib/ndb/local.template` carries @GUESTV4@/@MASKV4@/@GWV4@ tokens only (zero literals — verifier read the file); `root-tree/usr/glenda/lib/profile` terminal case has no rio line, cpu case intact. **EARLY headroom check ran and bit:** `15-08-services-payload-boot.log`/`-gate-driver.log` show the font-carrying services image green after the r9 module-stash raise (`236d5a7` 8→64MiB + FOD re-wire `af52fee`, pinned in flake.lock); payload manifest lines on the driver capture show `/lib/font/bit 4303 files 27676353 bytes` + all four fonts staged. |

**Score:** 4/4 truths verified (0 present, behavior-unverified)

**IP-suite scope note (ℹ️ Info, recorded not hidden).** SC1 names "the ip/ suite" among the executors. The ip suite's *compile* is marker-proven (all ip container members in tier2 with T2-ART); its *runtime* evidence is the net substrate every probe rides — the loopback ipifc bring-up (SVC-LOOPBACK-UP on BOTH venue captures), the TCP announce/connect round-trip (listen probe via the compiled svctcpecho client), and ipconfig's live T1-OK on r9. No individual ip-suite daemon (httpd, ftpd, dhcpd, …) carries its own per-daemon SVC marker. This matches the phase's planning contract exactly: D-02 (15-CONTEXT.md:26) defines the probe semantics and subject set, and the 15-02 PLAN must-haves enumerate cs/dns/exportfs/listen/mntgen + the D-08 auth subset — no ip-daemon probes were ever planned (the sweep's own comments record the measured reason: loopback-only venue, raw ipifc ctl writes instead of ipconfig). The interpretation is judged consistent with the SC as operationalized by the approved plans; broader ip-daemon exercise belongs to the networked eras (Phase 18+).

### Required Artifacts

`gsd_run query verify.artifacts` over all 9 plans: **25/26 pass**. The single tool miss is a pattern-check false positive: 15-09's artifact `sys/src/libmemdraw/drawtest.c` declares `contains: drawtest`, but the file never self-references its own name. The file exists (24,184 bytes, substantive), carries the round-to-nearest fix from commit `506ca9605`, and its `$O.drawtest` target is reachable from `sys/src/libmemdraw/mkfile:35` — VERIFIED on manual inspection.

| Artifact (representative) | Exists | Substantive | Wired | Status |
|---------------------------|--------|-------------|-------|--------|
| `build/x86_64/tiers/tier2-services.txt` | ✓ | ✓ 68 data lines, append-only grammar + provenance comments | read by 6 mk filters + driver + gate | ✓ VERIFIED |
| `sys/src/cmd/aux/mkfile` leaf filter | ✓ | ✓ T2KEEPTA/T2KEEPDA anchored ^cmd/aux: | gates TARG/DIRS under amd64 | ✓ VERIFIED |
| `build/x86_64/subset-build.sh` | ✓ | ✓ T2-ART/T3-ART, magic, per-entry clean, TIER-BUILD-OK | the phase build driver | ✓ VERIFIED |
| `build/x86_64/services-sweep.rc` | ✓ | ✓ 16KB, bounded spins, SVC-FAIL/SVC-SKIP arms | sourced from `k10t1.rc:54` boot flow; baked at /lib by mkimage | ✓ VERIFIED |
| `build/x86_64/services-r9.expect` + `nix/services-r9-x64.sh` | ✓ | ✓ computed expectations from tier data | `flake.nix:272` boot-r9-services-x64 app | ✓ VERIFIED |
| `build/x86_64/tier2.ledger` | ✓ | ✓ 13 rows, both venues | consumed by EVIDENCE/audit | ✓ VERIFIED |
| `build/x86_64/coverage-sweep.sh` + `dir-coverage.txt` | ✓ | ✓ 650-line driver; 702 rows all ok | the D-09 proof pair | ✓ VERIFIED |
| `build/x86_64/bootfs-payload.txt` + `root-tree/` skeleton | ✓ | ✓ manifest blocks + template + profile | `mkimage.sh:887` stage_payload call | ✓ VERIFIED |
| `build/mm/boot-9kdraw-trace.sh` + replay scripts + `trace-distill.sh` | ✓ | ✓ | venue harness; distiller re-run by verifier | ✓ VERIFIED |
| `sys/src/9k/k10/k10draw` + `vgadispi.c` + `multiboot.c` + `devcons.c` kbin | ✓ | ✓ | built into the draw venue kernel; iseve()-gated | ✓ VERIFIED |

### Key Link Verification

`gsd_run query verify.key-links` over all 9 plans: **18/20 verified**. The 2 misses are tool limitations in 15-09 (both use a directory as the `from:` node, which the verb requires to be a file): "sys/src/libmemdraw → boot-9kdraw-trace.sh (DRAWTEST-OK re-join)" and "sys/src/libmemdraw → subset-build.sh (tier-0 rebuild)". Both are manually verified: DRAWTEST-OK and ARCTEST-OK appear exactly once each on `15-06-trace-venue.log` and the post-review re-runs; libmemdraw is a tier0-libs member whose rebuild gate (`TIER-BUILD-OK`) the fix commits rode. Status: WIRED (manual).

Single-source data-flow (Level 4) — the phase's core anti-fraud property holds:

| Data file | Consumers | Computed, not duplicated | Status |
|-----------|-----------|--------------------------|--------|
| `tiers/tier2-services.txt` | 6 mk filters, subset-build.sh, services-r9-x64.sh expectations, mkimage services payload | ✓ (gate expectations grepped/computed from the file; mkimage payload set computed from it) | ✓ FLOWING |
| `bootfs-payload.txt` | mkimage stage_payload asserts | ✓ (payload_block() parses the file's own blocks) | ✓ FLOWING |
| `root-tree/lib/netaddr.rc` seam | local.template substitution | ✓ (sourced; both-direction asserts) | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Distilled trace tables derive from committed raws | `sh build/x86_64/trace-distill.sh {rio,acme,sam} <raw> /tmp/x "$NOTE"` + `cmp` | all three REPRODUCED byte-identical (verifier re-run, independent of the committed repro log) | ✓ PASS |
| Build scripts syntactically valid | `bash -n` on 8 harness/driver scripts | 8/8 clean | ✓ PASS |
| Donor + r9 SVC markers in committed captures | grep marker sets in donor-serial, runA, runB | 9 SVC-OK + SVC-DONE + OBS lines each; zero SVC-FAIL; zero final-run SKIPs | ✓ PASS |
| Trace venue bring-up + completion markers | grep 4 bring-up + 3 SCRIPT-DONE + DRAWTEST/ARCTEST-OK in 4 venue logs | present in all venue captures incl. post-review re-runs | ✓ PASS |
| Payload floors match reality | `find lib/font/bit -type f \| wc -l` / byte sum | 4303 files / 27676353 bytes vs floors 4303/27000000 | ✓ PASS |
| Negative control bites | read `15-review-fix-gs-smoke-negative.log` | GS-GATE-FAIL emitted on doctored scenario | ✓ PASS |
| Font equality copy-check wired | grep stage_payload P2 block | `err` on staged≠source count | ✓ PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` convention exists in this repo. The phase's probes are the qemu venue gates (donor boots, r9 composed gates, draw trace venue, gs smoke) — multi-hour runs whose green captures are the committed evidence per this phase's verification contract; NOT re-run here, sample-verified above instead (markers, internal consistency, reproducible derivation, negative controls).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|---------------------|----------|
| USPACE-07 | 15-01, 15-02, 15-03, 15-08 | services compile + execute both venues | ✓ SATISFIED | Truth 1; REQUIREMENTS.md line 36 `[x]`, traceability `Complete` |
| USPACE-08 | 15-04, 15-05, 15-06, 15-09, 15-08 | GUI compile + donor traces | ✓ SATISFIED | Truth 2; line 37 `[x]` |
| USPACE-09 | 15-07, 15-08 | long tail full-set + fossil/venti ceiling + gs | ✓ SATISFIED | Truth 3; line 38 `[x]` |
| DESK-03 | 15-08 | bootfs payload + manifest asserts | ✓ SATISFIED | Truth 4; line 75 `[x]` |

No orphaned requirements: the traceability table maps exactly USPACE-07/08/09 + DESK-03 to Phase 15, all `[x] Complete` (REQUIREMENTS.md:133-136). Held marks were released (the `[x]` states are live, not held — no `HELD:` qualifiers remain).

### Decision Coverage

Non-blocking gate (workflow.context_coverage_gate enabled): CONTEXT `<decisions>` D-01..D-17 translation was spot-checked during truth verification (D-02 probe semantics → sweep; D-03 fifth contract → mkimage; D-05/D-06/D-07 → replay+raw+tables+completion gates; D-09 full-set proof; D-13/D-14/D-15/D-16/D-17 → payload; D-01 fix-everything → r9 commits). No decision found translated-then-vanished.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none real) | — | all `XXX` grep hits are `mktemp …XXXXX` template strings in 8 harness scripts | ℹ️ Info | false positives, no debt markers (`TBD`/`FIXME`/`XXX`/`TODO`/`HACK`) found in any phase-modified file |

No stubs, no empty implementations, no hardcoded-empty data flows: every marker vocabulary has a FAIL arm, every gate has a negative classification, every ledger row carries sha256 + evidence + commit.

### Prohibition Audit (judgment-tier, verifier-evaluated — non-authoritative)

All 20 plan prohibitions were substantively checked; none found violated. Highlights: no tier-membership trim (data grew to 91 markers over 65 entries); exclusions re-derived with named measured reasons (7 records); per-daemon markers only through client interactions (OBS lines); no real key material (proto=pass test values, absent-nvram zero-key path); zero gap→SKIP conversions in final runs with every surfaced gap fixed as its own r9 commit (pin discipline verified: lock-only pin + two FOD re-wires); kbin iseve()-gated at open AND write; tracesup.c untouched since Phase 6 (git log); no GPL font staging (only `.ps` init files); whole font tree ships with equality assert; terminal profile provably rio-free (bake assert + committed file); template token-only. These verdicts are LLM-judged on codebase evidence — the end-of-phase human checkpoint may glance at them, per the judgment-tier soft-gate.

### Human Verification Required

N/A — infrastructure/foundation phase (build machinery, kernel venues, compile tiers, payload staging; no end-user-facing UI to test manually). All acceptance criteria verified programmatically against committed captures; the one behavior class that could warrant eyes (draw-venue pixels) is machine-verified by drawsmoke's pixel readback with byte-order canonicalization (WR-03 fix), and the trace venue runs headless by design.

### Gaps Summary

None. All four roadmap success criteria verified with committed, internally consistent evidence; the r9 kernel-fix trail, pin discipline, and FOD re-wires check out against the sibling repo and flake.lock; the independently re-derived trace tables prove the distillation pipeline.

**Documented caveats (checked recorded, not hidden — none is a phase must-have):** fossil+venti compile-gated only (the SC's own letter — runtime-unverifiable this milestone, noted in 3 places); 7 named coverage exclusions with measured reasons; gs smoke uses ppmraw (measured ld.tr deviation, documented, same FILE-device class); payload declared by the services contract only (DECLARE-by-invocation design; other contracts can stage it); tier1.ledger's two 14-08-era stale SKIP rows + the unexplained one-off tier-witness drift on rc/grep/awk — both honestly recorded in `deferred-items.md` with owning-work descriptions, and both are records-vs-gate claims, not failed truths (the live gates show 38 T1-OK / zero SKIP); the review's 7 info-tier findings (IN-01..07) recorded open in `15-REVIEW-DISPOSITION.md` after 8 substantive fixes (CR-01 + WR-01..07) landed with re-run evidence (`15-review-fix-*.log`).

---

_Verified: 2026-11-10T21:16:33Z_
_Verifier: Claude (gsd-verifier)_
