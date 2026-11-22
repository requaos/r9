# Phase 15 Evidence Index — Userspace Tiers + Bootfs Payload (amd64)

**Phase:** 15-userspace-tiers-bootfs-payload-amd64
**Consolidated:** 2026-11-10 (plan 15-08, task 3)
**Status:** complete — every requirement truth (USPACE-07/08/09, DESK-03) mapped to the gate runs that prove it, at the pinned r9 SHA, with the retained artifacts under the phase evidence dir (see the table at the end). Nothing below is claimed green that was not run: every cited capture exists at its named path and was re-verified to carry its markers at consolidation time; where a record is stale or a verdict lived on a driver's console rather than the wire, this file says so.

## Pinned commits

| Repo | SHA | Role |
|------|-----|------|
| r9 (`requaos/r9`, branch `main`) | **`af52feef7620137c43c5e04d077819fbcf3ddb82`** | the phase-complete HEAD — includes 15-03's seven services fixes + FOD re-wire (pin `3362cf7`) and 15-08's two commits (the module-stash raise + its FOD re-wire). Fully pushed (`origin/main..HEAD` = 0; push-before-pin held at every advance); Harvey's `flake.lock` pins exactly this rev (lock-only commit `eba6b07e2`). |
| harvey | `07649114f` | 15-08's last code commit (the ndb template + D-17 profile); the payload manifest + font bake landed at `bdc1fcf07`. |
| harvey | this docs commit | 15-EVIDENCE.md + the new retained captures + the 15-08 summary (the consolidation you are reading). |

**Per-plan commit trail (all ancestors of the pinned HEADs):**

| Plan | Commits | Closed |
|------|---------|--------|
| 15-01 (tier-2 compile) | `c88d05b8e`..`01e6e0e7f` (5) | USPACE-07 compile half |
| 15-02 (donor services) | `64506863b`..`754e99e3b` (28) | USPACE-07 donor-execution half |
| 15-03 (r9 services + D-01) | `f5a9c0278`..`b8f9ea33e` (6) + r9 `e045dde`..`2a31fcc` (7) | USPACE-07 r9 half |
| 15-04 (GUI compile) | `e42069fbb`..`062bd06a9` (3) | USPACE-08 compile half |
| 15-05 (draw donor) | `ed1a0af37`..`018731130` (4) | USPACE-08 venue half |
| 15-09 (libmemdraw audit) | `506ca9605`..`f1382b8d3` (4) | USPACE-08 pixel-path half |
| 15-06 (traces + re-join) | `3680ad88f`..`1d74692b2` (4) | **USPACE-08 complete** |
| 15-07 (coverage + gs) | `36e985996`..`b7334da3d` (8) | **USPACE-09 complete** |
| 15-08 (payload + this bundle) | `eba6b07e2`..`07649114f` (3) + r9 `236d5a7`..`af52fee` (2) | **DESK-03 complete** |

## Requirement → gate-run mapping

### USPACE-07 — the services tier compiles AND executes on both venues

- **Compile (15-01):** 65-entry `build/x86_64/tiers/tier2-services.txt` through the six filter sites (tier1+tier2 union awk, the NEW aux leaf filter) and the subset driver's per-entry fresh builds — 91 T2-ART markers, `TIER-BUILD-OK`; the executable scoping proof (committed at evidence/15-01-scoping-proof.sh ) pins the default amd64 build surface to EXACTLY tier1∪tier2 (8/8 checks). The retained log (evidence/15-01-scoping-proof.log ) carries the EXP/MK evaluation lines; the PROOF-PASS verdict lines print to the console by the script's design (recorded in the 15-01 and 15-07 SUMMARYs) — the log itself is the evaluation record, not the verdict carrier.
- **Donor execution (15-02):** the k10t1 venue runs the committed services-sweep from the boot flow; all nine subjects earn interaction-backed SVC-OK markers (mntgen walk-create, cs `ip=127.0.0.1` ipinfo answer, dns baked A record, exportfs sentinel-through-mount, listen loopback round-trip, factotum masked-key readback, keyfs write-back-on-create, authsrv connect-accept), green 3× consecutively — capture retained at evidence/15-02-donor-services-serial.log (SVC-DONE, zero SVC-FAIL, nine SVC-OK subjects — re-verified at consolidation).
- **r9 execution (15-03):** the fifth bake contract (out/bootfs-services.paq) + the composed `boot-r9-services-x64` gate, green TWICE consecutively (captures evidence/15-03-services-runA.log and evidence/15-03-services-runB.log , both SVC-DONE + zero SVC-FAIL + nine SVC-OK subjects) after five root-caused r9 kernel fixes (the D-01 table below); the tier-1 SKIP flips re-greened the tier-1 gate in the same commit (evidence/15-03-tier1-flip.log : 38 T1-OK, zero SKIP/FAIL).
- **Both-venues record:** `build/x86_64/tier2.ledger` — 13 rows, every one `exec=SVC-OK:k10t1+SVC-OK:r9x64`, zero SKIP rows.
- **Re-proven at the phase close (15-08, this plan):** the services gate green on the font-carrying payload image FOUR times (the EARLY check capture evidence/15-08-services-payload-boot.log , the Task-2 full-payload image, and the final re-green evidence/15-08-services-final.log — each SVC-DONE, zero SVC-FAIL, nine SVC-OK subjects, zero panic) at the advanced pin `af52fee`.

### USPACE-08 — the GUI stack compiles, and the donor-kernel syscall traces are captured

- **Compile (15-04):** `build/x86_64/tiers/tier3-gui.txt` (11 entries, the gping promise kept) + the libdraw family closed into tier-0 (24 T0-ART) — 12 T3-ART markers; the compile-vs-execution boundary (D-07) lives in the marker vocabulary itself (T3-ART is artifact evidence only).
- **The venue (15-05):** k10draw boots under qemu -vga std with the DISPI VGAdev; `/dev/draw` proven alloc/paint/readback; the 36MB font payload reaches the guest through -initrd→addbootfile→paqfs (bootfs-draw.paq 20240113 bytes — the A3 measurement); the eve-gated /dev/kbin + /dev/mousein replay surface. The four venue markers (DRAW-SMOKE-OK, FONT-CHANNEL-OK, KBIN-OK, MOUSEIN-OK) are on the committed capture evidence/15-06-venue-bringup.log (re-verified, one occurrence each).
- **The traces (15-06):** born-traced rio/acme/sam trees driven by the committed replay scripts — RIO-SCRIPT-DONE, ACME-SCRIPT-DONE, SAM-SCRIPT-DONE on the full-set capture evidence/15-06-trace-venue.log (re-verified); the distilled tables are committed beside their raw logs (evidence/15-06-rio-raw.txt and evidence/15-06-rio-table.txt , evidence/15-06-acme-raw.txt and evidence/15-06-acme-table.txt , evidence/15-06-sam-raw.txt and evidence/15-06-sam-table.txt ) with the byte-identical distill reproducibility proof at evidence/15-06-distill-repro.log (all three REPRODUCED byte-identical). TRACE-GATE-PASS is the host harness's verdict (recorded in the 15-06 SUMMARY) — it is a driver-console line, NOT a wire marker; the wire evidence is the five session markers above.
- **The pixel-path audit + re-join (15-09 + 15-06):** drawtest's two-sided compositing fix and arctest's byteaddr zero-extension fix (class commits `506ca9605`, `e8df41c7d`); the runtime re-join green on the venue — DRAWTEST-OK and ARCTEST-OK both on evidence/15-06-trace-venue.log (re-verified, one occurrence each).
- **The ledger:** `build/x86_64/tier3.ledger` — 12 rows: TRACE pointers on rio/acme/sam(+samterm), the utility class honestly `exec=STAGED:draw-venue-payload` (an artifact pointer is earned by driving it, never by inheritance).

### USPACE-09 — the long tail compiles, full-set, machine-verified

- **The proof (15-07):** `build/x86_64/dir-coverage.txt` — 702 rows, EVERY row build=ok, zero not-ok, 1353 VISIT / 1353 EXIT (re-verified at consolidation: 702/702/0). The driver `build/x86_64/coverage-sweep.sh` visits the full universe minus the re-derived tier4 exclusions; a killed run leaves a classifiable partial that can never pass.
- **The fix discipline (D-10):** every first-run failure root-caused — three in-tree class commits (`09ea2f525` 4c lex-object override, `c8d635e59` 5c enam.c regeneration, `379677dc6` libdynld dead check, plus the `a7c430352` ambiguous-recipes repair); the 7 remaining failures are re-derived upstream-disabled/dead-tool exclusions recorded as named `# exclude:` comments in `build/x86_64/tiers/tier4-tail.txt` with measured digests — never visited, never rows.
- **The honest ceiling:** fossil + venti compile-gated with the runtime-unverifiable note committed in THREE places (tier4 entries, trailing comments on their dir-coverage rows, the 15-07 SUMMARY) — no block device in r9; virtio-blk is a future milestone.
- **gs beyond compile (D-11):** the font-free fixture rendered IN GUEST to a 43269-byte PPM bitmap through the FILE device — GS-RENDER-OK + GS-BITS 43269 on the retained capture evidence/15-07-gs-smoke.log (re-verified), zero font payloads staged anywhere (D-14's fonts come from lib/font/bit only).

### DESK-03 — the desktop payload staged in bootfs with manifest assertions (15-08)

- **The manifest is data (D-15):** `build/x86_64/bootfs-payload.txt` — [paths] / [fonts] / [floors] blocks consumed by name by mkimage's payload section; the single list (no second hardcoded one anywhere). Negative controls prove the asserts bite: each of the six doctored-manifest/tree classes (file floor, byte floor, absent named font, absent path, empty file, trimmed 3-file font source) fails `stage_payload` at its named assert; the positive control passes.
- **The whole tree ships (D-14):** the bake-time copy stages lib/font/bit verbatim (4303 files, 27676353 apparent bytes) with a staged==source file-count check — a partial copy cannot pass silently even after the tree grows past the committed floors. The four app-named fonts (lucm/unicode.9 for rio, lucidasans/euro.8 for acme, pelm/latin1.8 + pelm/euro.9 for the profile) each asserted present and printed on every bake.
- **The headroom check ran EARLY and bit (the plan's own sequencing rule):** the services paq measured 7644254 → 26484599 bytes at the first font-carrying bake (fonts deflate to ~18.8MB, NOT the assumed ≤15MB); consecutive bakes byte-identical at each KERNDATE (final pair: sha256 `83035697…`, 26487039 bytes at KERNDATE 1794338236). The FIRST boot panicked — r9's 8 MiB module-stash clamped the 26.5MB mb2 module (the D-01 table's last row); at the raised pin the image boots green: `q35: module @0x6800000 +0x19428ff (mb2 tag 3, stashed)`, `paqfs root mounted from module (26487039 bytes)`, kernel Used 0x6800000 (104 MiB) of ~1 GiB — evidence/15-08-services-final.log .
- **The ndb template (D-16):** `build/x86_64/root-tree/lib/ndb/local.template` carries @GUESTV4@/@MASKV4@/@GWV4@ tokens ONLY (the 15-02 baked-entry content model); mkimage sources the netaddr seam and substitutes; both directions asserted (seam values present, zero leftover tokens — each class proven by negative control), and the pre-existing 10b seam grep stays clean (a literal planted in the template is caught — control-verified).
- **The profile (D-17):** the terminal case is provably rio-free AT BAKE TIME (the assert greps the staged file; a planted `exec rio` fails the bake — control-verified) while the cpu case keeps `exec rio` + `font=/lib/font/bit/pelm/latin1.8.font` verbatim for Phase 19 (each loss class control-verified); the whole-file grep for the two dropped services stays zero (the header comment names neither, the 14-09 comment-prose lesson applied).
- **Which image carries it:** the payload is a DECLARE-by-invocation section (mkimage's `stage_payload`); the services contract — the image Phases 18/19 consume — declares it this era. The profile and template additionally ride the committed skeleton tar into every contract that stages it (inert there: no gate execs the profile, and only the services bake substitutes ndb/local).

## The D-01 fix table — every r9 kernel gap surfaced by services execution, with its record

| r9 SHA | Fix | Surfaced by | Regression battery at the fix |
|--------|-----|-------------|-------------------------------|
| `e045dde3fa4ce463d3acab55b813237d07de7f19` | rfork non-RFPROC RFFDG/RFCFDG — the in-place fd-group split (lib9p postmountsrv's enabler) | 15-03 (every srv-mounted daemon died `mount: bad fd`) | q35exec/q35shell/q35/q35net + aarch64 virt PASS, port --lib 101/101, clippy clean (15-03 SUMMARY) |
| `06f8aa1a5173d47bcf598cd92efcadbdac681281` | devmnt Twrite — writes through a mount (the policy lives in the server, paqfs still refuses) | 15-03 (exportfs content-through-mount) | same battery green (15-03 SUMMARY) |
| `21d2350ca751a33b4b2110066dec8d70f8f3a698` | devmnt Tcreate + sys_create DMDIR — directory creates through a mount (Tcreate rides a cloned fid) | 15-03 (keyfs DMDIR create) | same battery green (15-03 SUMMARY) |
| `de1fba1ab5212669368ea010f31f4ef16cd9fa6c` | devproc private/noswap ctl verbs — factotum's private() writes | 15-03 (factotum startup) | same battery green (15-03 SUMMARY) |
| `242f228dd101553159c19eb27a437986d5a002ef` | kernel heap arena 4 → 32 MiB + the oom arena-stats print (capacity, deliberately not the leak fix — the ~16 KiB per-exit residue stays Phase 17's) | 15-03 (mid-sweep exhaustion at proc ~73) | same battery green (15-03 SUMMARY) |
| `3362cf74eeeec35f83f8fc0c8e428b7ae605a04c` | the boot-iso FOD re-wire after the fixes (two-derivation byte-identity proof) | 15-03 close | `nix build .#boot-iso` green at the wired hash (15-03 SUMMARY) |
| `2a31fccea0a4283b6c107598a79ce1f6ad785016` | devproc_test ProcHooks initializer (test-only bytes, post-pin, no kernel change) | 15-03 Task 3 | cargo xtask check + devproc_test 1/1 (15-03 SUMMARY) |
| `236d5a787891d593e17859049f3788e72cc47ae4` | the boot-module stash 8 → 64 MiB — the DESK-03 font payload's 26.5MB services bake clamped at the 8 MiB MODSTASHMAX and panicked the stash-length reconciliation | **15-08 (the EARLY headroom check — this plan's first boot)** | q35exec/q35shell/q35/q35net + aarch64 virt PASS, port --lib 101/101, xtask check + clippy clean — run by 15-08 at the fix commit |
| `af52feef7620137c43c5e04d077819fbcf3ddb82` | the boot-iso FOD re-wire after the stash raise (desk03-proof-a/-b byte-identical, NAR `sha256-q54Zp2…` three-way-agreed with the FOD's own got-value) | 15-08 | `nix build .#boot-iso` + `--rebuild` green, re-verified at the committed clean rev — run by 15-08 |

All nine pushed BEFORE the pin advances that name them (push-before-pin held at `3362cf7` and `af52fee`).

## The flagged-assumption outcomes (A1–A7 + this plan's own)

| Assumption | Outcome | Evidence |
|------------|---------|----------|
| A1: qemu's VBE DISPI ports set the video mode under KVM (route 1) | CONFIRMED — route 1 stands, the realmode fallback never needed | 15-05 SUMMARY (key-decisions); the venue boots every 15-06 session |
| A2: `-initrd` registers as an mb1 module under `-kernel` | CONFIRMED — the `mod …` boot print on the capture | 15-05 SUMMARY; FONT-CHANNEL-OK on evidence/15-06-venue-bringup.log |
| A3: paqfs deflates the 36MB font tree to ~≤15MB | FALSIFIED in its letter — measured ~18.8MB into the services paq (the draw bake's whole paq: 20240113 bytes); the headroom held anyway, but the binding limit was r9's 8 MiB module stash, not venue RAM (fixed per D-01) | 15-05 + 15-08 measurements; the module line on evidence/15-08-services-final.log |
| A4: the pcc/APE class (gs 753 files et al.) compiles on amd64 | HELD — the full-tree proof is green | 15-07: 702 rows, zero not-ok |
| A5: tracesup.c builds under 6c unmodified | HELD — built by 6c unmodified at HEAD | 15-06 SUMMARY (A5 record) |
| A6: the donor services embed fits Nbootfiles=64 | HELD — 58 embedded files, no ceiling raise | 15-02 SUMMARY; the 58-count assert in build/mm/build-9k10t1.sh |
| A7: r9's devmnt/sys_mount grows for srv-mounted services | LANDED as fix commits (the read-only policy was transport-layer) | the D-01 table rows 2–3 |
| 15-08: bake interruption idempotence (a killed mkimage leaves a partial STAGE) | HELD — every invocation rebuilds its STAGE fresh from committed inputs, and the KERNDATE determinism pair is the interruption detector (consecutive bakes byte-identical at each pin; the 38-byte cross-KERNDATE delta was isolated to the timestamp value, reproduced exactly by re-baking at the old KERNDATE) | this plan's bakes |
| 15-08: the profile's mousectl/plumber lines — guard or drop | DROPPED — no devmouse/plumber substrate on a serial-console image, and a failed redirect RAISES in rc (the 15-02 measured dialect); a guard would additionally require /bin/test on every image that might read the profile. A future draw-terminal venue re-derives its own bring-up | the profile header; the bake-time shape asserts |
| 15-08: the EARLY headroom fallback is "bump venue memory, never trim fonts" | The fallback's premise was wrong — venue memory was never the limit; the kernel stash was. Fonts were never trimmed; the kernel capacity was raised (D-01) | the D-01 table's 15-08 rows |

## The discretion / OQ resolutions

- **OQ-1 (the VGA route):** route 1 (DISPI + PCI-LFB) adopted as primary — resolved by 15-05 Task 1; route 2 (realmode port) remains the documented fallback, never needed.
- **OQ-2 (keyboard replay determinism):** the in-guest eve-gated kbin-class file — resolved by 15-05 Task 3; the committed 15-06 replay scripts are in-guest data driven through kbin/mousein; serial injection stayed the bring-up bootstrap only.
- **OQ-3 (r9 gap sizing):** budgeted as named per-gap fix tasks under push-before-pin — resolved by 15-03 Task 3 (seven commits) and extended by 15-08 (two more); sizing stayed flush-out by design.
- **The CI topology (15-01 Task 3):** host-only tier-compile gates with computed marker-parity checks in phase6.yml; no donor/r9 execution legs in CI per the Phase-14 venue precedent.
- **The profile guard choice (15-08):** drop, not guard — the flagged-assumption table above.
- **The payload-declaring contract (15-08):** `stage_payload` is a shared declare-by-invocation section; the services image declares it (the 18/19 consumer). Growing the marker/chain/tier/net images too would have changed four proven gate images with no re-gate demanded by any plan — the reading is recorded in the 15-08 SUMMARY.

## The venue-honesty record

- **Conditional SVC-SKIPs:** the services sweep's skips are CONDITIONAL arms (venue-degradation only); on every green run above the emitted set is EMPTY and every subject earned its OK — the gates assert the converse on any emitted name.
- **The tier-1 SKIP history:** the 14-08-era `T1-SKIP ipconfig` (stale no-devrandom reason) and `T1-SKIP factotum` (no-auth-era) were flipped LIVE by 15-03 in the same commit as every gate expectation — the r9 tier-1 gate expects ZERO skips since (38 T1-OK on every 15-08 re-run). NOTE the committed 14-08 execution ledger `build/x86_64/tier1.ledger` still carries its two historical `exec=SKIP:r9x64-…` rows — a stale RECORD (not a gate claim), flagged to the phase deferred items at consolidation.
- **SKIP→18 entries: NONE.** No Phase-18-owned substrate entry fired anywhere in the phase (the D-08 boundary held: keyfs served its empty-DB/write-back path over an absent test keyfile, authsrv proved listening, no key material in any capture).
- **NOT-L2-TRUTH (carried from 14-09):** the x64 net fixtures run on qemu slirp, which forgives L2 bugs by design; the label is printed INTO the captures by the fixtures and asserted as stages (live on the 15-08 phase-close net re-run). The tap-leg truth verdict is Phase 16's NETIP-08.
- **fossil + venti:** compile-gated only, runtime-unverifiable this milestone (no block device in r9) — the note is committed in three places (15-07).
- **The gs venue's FONT-CHANNEL-FAIL line is BY DESIGN** — D-11 stages zero font payloads in the gs paq (licensing boundary); DESK-03's fonts come from lib/font/bit only.
- **The donor-side green records stand from their own plans' runs** (15-02's captures at its commits); 15-08 rebuilt the donor embed only to re-pin the tier witness (see the deviations) and did not re-run the donor sweep — no fresh donor claim is made here.

## Regression state at the phase close (all at the advanced pin `af52fee`, run by 15-08)

| Leg | Result |
|-----|--------|
| `nix run .#boot-r9-services-x64` (the payload-carrying services image) | PASS — the EARLY check, the Task-2 image, and the final re-green (evidence/15-08-services-payload-boot.log and evidence/15-08-services-final.log ) |
| `nix run .#boot-r9-tier1-x64` | ALL STAGES PASS — 38 T1-OK, zero SKIP/FAIL (evidence/15-08-phase-close-tier1.log ) |
| `nix run .#boot-r9-net-smoke-x64` | ALL STAGES PASS — the NOT-L2-TRUTH labels live, both degenerate-input stages (evidence/15-08-phase-close-net.log ) |
| `nix run .#boot-r9-smoke-x64` | ALL STAGES PASS — the echo/empty-line/ls-ps-cat/DEL/restart/nic-bind stages (evidence/15-08-phase-close-smoke.log ) |
| r9 battery at the stash fix | q35exec / q35shell / q35 / q35net / aarch64 virt PASS; port --lib 101/101; xtask check + clippy clean |
| `nix build .#boot-iso` (r9, at `af52fee`, clean committed tree) | exit 0, + `--rebuild` exit 0 (the re-wired FOD) |
| mkimage determinism | consecutive bakes byte-identical at each KERNDATE (final: sha256 `83035697…`, 26487039 bytes) |

These re-runs exist because 15-08 advanced the r9 pin (new kernel bytes under every r9 gate) and the skeleton change touched every mkimage contract's bytes — no gate is claimed at the close on bytes it did not boot.

## Retained artifacts (evidence/)

| File | What it is |
|------|------------|
| evidence/15-01-scoping-proof.log | the scoping proof's EXP/MK evaluation log (verdict lines print to console by design — see above) |
| evidence/15-02-donor-services-serial.log | the donor services capture — nine SVC-OK with OBS lines, SVC-DONE |
| evidence/15-03-services-runA.log and evidence/15-03-services-runB.log | the r9 services gate, the twice-green pair |
| evidence/15-03-tier1-flip.log | the tier-1 SKIP-flip re-green (38 T1-OK, zero SKIP) |
| evidence/15-06-rio-raw.txt and evidence/15-06-rio-table.txt | rio: raw per-pid fragments + the distilled table |
| evidence/15-06-acme-raw.txt and evidence/15-06-acme-table.txt | acme: raw + table |
| evidence/15-06-sam-raw.txt and evidence/15-06-sam-table.txt | sam + samterm: raw + table |
| evidence/15-06-trace-venue.log | the full-set trace capture — the three SCRIPT-DONE markers, DRAWTEST-OK, ARCTEST-OK |
| evidence/15-06-venue-bringup.log | the draw-venue bring-up capture — the four 15-05 markers |
| evidence/15-06-distill-repro.log | the byte-identical distill reproducibility proof (rio/acme/sam) |
| evidence/15-07-gs-smoke.log | the gs render smoke — GS-RENDER-OK, GS-BITS 43269 |
| evidence/15-08-services-payload-boot.log | the EARLY headroom check — the first font-carrying boot green at the raised pin |
| evidence/15-08-services-final.log | the final re-green at 15-08's last code commit (the module/paqfs lines with the full 26.5MB payload stashed) |
| evidence/15-08-phase-close-tier1.log , evidence/15-08-phase-close-net.log , evidence/15-08-phase-close-smoke.log | the phase-close regression re-runs at `af52fee` |
| evidence/15-08-services-payload-gate-driver.log | the services gate driver's own console for the EARLY-check run |

---
*Phase: 15-userspace-tiers-bootfs-payload-amd64*
*Consolidated: 2026-11-10*
