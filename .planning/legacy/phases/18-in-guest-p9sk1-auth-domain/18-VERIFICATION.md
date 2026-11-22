---
phase: 18-in-guest-p9sk1-auth-domain
verified: 2026-11-14T19:05:00Z
status: passed
score: 9/9 must-haves verified
covered_files: [".github/workflows/r9-shell.yml", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-01-PLAN.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-01-SUMMARY.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-02-PLAN.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-02-SUMMARY.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-03-PLAN.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-03-SUMMARY.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-04-PLAN.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-04-SUMMARY.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-05-PLAN.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-05-SUMMARY.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-06-PLAN.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-06-SUMMARY.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-07-PLAN.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-07-SUMMARY.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-08-PLAN.md", ".planning/phases/18-in-guest-p9sk1-auth-domain/18-08-SUMMARY.md", "build/x86_64/README.md", "build/x86_64/auth-sweep.rc", "build/x86_64/randread.c", "build/x86_64/secstore-obj.txt", "build/x86_64/smoke-r9-auth.expect", "flake.lock", "nix/smoke-r9-auth-x64.sh"]
covered_digest: "v4:sha256:01c3fd1382314d6d5db6b7ed78afefd30babe5bb8e5ac3b2bfe22c04572b7f0e"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 8/9
  gaps_closed:
    - "AUTH-08 secstore round-trip (kernel half 18-07 r9 ab4cb66 + client half 18-08 sweep 9d restore + gate flip; green twice at pin 9103522)"
    - "AUTH-03 / CR-01 eve-gate bypass (18-06 r9 652f025: iseve gate + donor 128-byte bound + refusal pins + randread HOSTOWNER-REFUSED leg)"
    - "AUTH-02 / CR-02 entropy probe mis-wiring (18-06 r9 f594b4c: cpuid(1,0) rebind + scripted-cpuid seam + probe pins)"
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "CR-03 (review, critical, disposition open): the 18-07 donor-faithful socket config landed only on the connect path — AcceptPool::spawn_listener (stack.rs:814-823) still builds announced-side pool sockets with smoltcp stock defaults (Nagle ON, 10ms delayed ACK), and x86_64's q35 tick never calls stack::tick_check; a client write >2048B (POOL_SOCK_BUF) into an announced connection can park both procs forever — the 18-05 stall class on the un-fixed half. Structurally CONFIRMED in code by this verification (tcp_socket's only caller is connect_proto at proto.rs:565; the pin writes only client-side 2+31B; DEMO object is 401B). NOT reproduced (no failing test, no reproducible artifact) — new-scope relative to the prior gaps, so advisory per the #3304 evidence gate, not a blocker. NOTE: the 18-07/18-08 summaries' and STATE.md's claim that 'the bulk-write (>2048-byte) stall class is closed by ack_delay(None)' is overstated — it holds for connect-path sockets only. Natural reopener: Phase 19's cpu/exportfs bulk traffic over announced connections (the review's suggested fix is two set_* calls at spawn_listener + pin legs per WR-04)."
    category: architectural
    reason: "Behavioral consequence (>2048B announced-connection write hangs) is well-reasoned but unexercised by any gate or test; defect site (stack.rs pool sockets) predates the round unmodified; the round's fix strictly improved the connect half. Developer decision requested: accept as tracked open item (already recorded open in 18-REVIEW-DISPOSITION.md) or plan an interstitial r9 fix (small, well-specified)."
    evidence_status: "structural code confirmation only (spawn_listener lacks the set_* calls; tick_check aarch64-only at aarch64/src/platform/virt/timer.rs:130); no failing test or reproduced hang provided"
  - finding: "WR-03 (review, warning, open): the hostowner write drops the donor's rename — eve writing a foreign name (the `echo name > /dev/hostowner` idiom stores the trailing newline) permanently ends eve for the boot (write-once-per-boot lockout). Code-confirmed: the eve path stores raw bytes, no rename, no newline strip — this is 18-06's own documented fail-closed divergence, pinned as intended behavior in hostowner_write_stores_for_eve; reopener (first real hostowner-write consumer brings donor rename semantics) recorded in the SUMMARY."
    category: other
    reason: "Deliberate prohibition-backed design decision with recorded reopener; residual risk is a documented divergence, not an incomplete implementation of any must-have."
    evidence_status: "none needed — matches the plan's own prohibition"
  - finding: "WR-04 (review, warning, open): the RED->GREEN pin covers only the client/connect side at 2+31B — no server-conv write pattern and no >2048B bulk-drain leg (CR-03's blind spot). Confirmed by reading the pin."
    category: other
    reason: "Test-coverage extension suggestion; the must-have truth (the CLIENT pattern) is exactly what the pin tests and its name says so."
    evidence_status: "pin source read; no missing assertion claimed by any must-have"
  - finding: "IN-06 (review, info, open): sweep 9d's venue-copy redirect ($B/hcat ... > /adm/secstore/secstore-obj.txt, auth-sweep.rc:471) is fatal-on-failure with a mislocalizing 600s timeout; IN-07 (review, info, open): README:359-365 still names r9 pin d1c4849 in present tense after the 9103522 advance. Both confirmed present."
    category: other
    reason: "Info-level nits; neither affects a gate verdict or a must-have."
    evidence_status: "read in source this pass"
---

# Phase 18: In-Guest p9sk1 Auth Domain Verification Report

**Phase Goal:** The x86_64 guest is a self-contained p9sk1 auth domain — one bake-time generator produces the machine key, `/adm/keys`, nvram-substitute, and ndb entries as a hash-paired unit so all four p9sk1 configs agree by construction; keyfs + authsrv serve it over loopback `/net`; the in-guest probe proves the handshake before any drawterm is ever attempted. Work spans both repos (r9 `/dev/random` + devcap; Harvey bake machinery + image).
**Verified:** 2026-11-14T19:05:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure (previous: gaps_found 8/9, three code-confirmed gaps: AUTH-08 secstore round-trip, CR-01 eve-gate bypass, CR-02 entropy mis-wiring)

## Goal Achievement

All three previous gaps are closed and INDEPENDENTLY REPRODUCED by this verification, not just read from SUMMARYs: the refusal pins, the two-write pin, the probe pins, both full batteries, the FOD build, the net-smoke gate at the pin, the q35auth substrate gate (five cap markers + HOSTOWNER-REFUSED + venue label), and the generator self-test were all run live in this pass; the twice-green auth-gate captures at the advanced pin were verified marker-by-marker. The phase goal — self-contained p9sk1 domain, all four configs paired by construction, keyfs+authsrv over loopback /net, in-guest handshake proven pre-drawterm — now includes the secstore round-trip on delivered bytes.

One critical REVIEW finding (CR-03) remains open in the project's own disposition record and is carried as an advisory below with a developer decision requested — it is outside every must-have this phase contracted (the 401-byte DEMO round-trip and the client two-write pattern), is not reproduced by any failing test, and its defect site (accept-pool socket defaults) predates the gap-closure round unmodified.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | AUTH-01: bake-time generator is the only way domains are made — machine key + /adm/keys + nvram-substitute + ndb entries as ONE hash-paired unit; DEMO loudly labeled; pairing fails loud | ✓ VERIFIED (regression held) | `authdomain-gen.c` (901 lines) unchanged since initial pass; **self-test RE-RUN LIVE this pass: exit 0, GEN-SELFTEST-OK, MISSING-F-REFUSED, all negative controls**; `authdom: PAIRED DEMO` present in both 18-08 captures at the advanced pin |
| 2 | AUTH-02: r9 /dev/random CSPRNG seeded from x86_64 entropy, live on both accel legs — now including CR-02 closed (cpuid1_ecx from a FRESH cpuid(1,0)) | ✓ VERIFIED (gap 3 closed) | `armed_source()` rebind read in source (entropy.rs:132-141: `cpuid(1,0)` supplies cpuid1_ecx; leaf-7 call supplies ONLY its EBX); scripted-cpuid seam with loud unscripted-leaf panic (entropy.rs:106-127); **entropy filter RE-RUN LIVE: 9/9** (incl. entropy_probe_arms_rdrand_from_leaf1_ecx, entropy_probe_ignores_leaf7_ecx_bit30, entropy_probe_venue_shapes_stay_correct); RED evidence JSON verified genuine (both target pins failed pre-fix on exactly the planned assertions, 7 siblings green); cpu.rs diff over the round EMPTY (contracts untouched); `random: source=rdrand` + READS-OK in MY OWN q35auth boot this pass |
| 3 | AUTH-03: devcap #¤ (caphash/capuse) — now including CR-01 closed (the #c/hostowner write is eve-gated with the donor bound) | ✓ VERIFIED (gap 2 closed) | Gate read in source (devcons.rs:583-603): iseve() FIRST -> `devcons: hostowner write requires the hostowner`; empty/\>=128 -> `devcons: bad arg to hostowner write`; gate fires BEFORE the store lock; **devcons filter RE-RUN LIVE: 7/7** (3 new refusal pins + user_eve_gate_default_and_hostowner_write unchanged); randread.c leg 5 committed (prints `cap: HOSTOWNER-REFUSED err=` only on refusal, fail-fast on success, randread.c:150); xtask CAP_HOSTOWNER_REFUSED_MARKER in the finish() demand list (main.rs:929,2792); **q35auth substrate gate RE-RUN LIVE: exit 0, `cap: HOSTOWNER-REFUSED` present, all four pre-existing cap legs green, zero substrate: FAIL** |
| 4 | AUTH-04: keyfs starts non-interactively over baked /adm/keys; authsrv listens on tcp/567 | ✓ VERIFIED (regression held) | both 18-08 captures: `authdom: PAIRED DEMO`, `keys read`, `AUTH-OK keyfs`, `AUTH-OK authsrv-listen`; wiring unchanged (sweep nvram= before keyfs; listen1 -t tcp!*!567 authsrv -d) |
| 5 | AUTH-05: guest domain ndb content staged in the image | ✓ VERIFIED (regression held) | both 18-08 captures: `AUTH-CSOBS ... authdom=... auth=127.0.0.1` -> `AUTH-OK cs` — cs answers from the baked /lib/ndb/local live |
| 6 | AUTH-06: in-guest loopback p9sk1 probe green before any drawterm; non-hostowner speaksfor gate + fail-closed control | ✓ VERIFIED (regression held) | both 18-08 captures: `ticket request using norbert@...` + `authprobe: SPEAKSFOR-OK bootes for norbert`; SPEAKSFOR-DENIED exactly 1 (count post-pass still armed, expect:431-437); tr-ok x4 |
| 7 | AUTH-07: serial-visible auth logging from day one | ✓ VERIFIED (regression held) | both 18-08 captures: 4 tr-ok tokens, PAIRED banner, ANNOUNCE line; #c/cons fallback posture unchanged |
| 8 | AUTH-08: secstore available for persistent key storage beyond baked keys — the client PUT/GET round-trip DELIVERED at the advanced pin | ✓ VERIFIED (gap 1 closed) | Kernel half: fix at proto.rs::tcp_socket (2124-2133, Nagle off + ack_delay None) read in source; **two_write pin RE-RUN LIVE: 1/1**; RED evidence JSON genuine (14 discovered, only the target failed pre-fix on the park assertion); before/after captures genuine (repro: qout 33 held at t+3s; fix: `mode-2 REPLY count read` + `mode-2 child done`). Client half: sweep 9d restored (auth-sweep.rc:469-483 — venue copy, PUT `-s 127.0.0.1 -i -u norbert -p`, `-G` GET, bytes-back grep -> AUTH-SECOBS + rtstat, verdict ONLY on rtstat ok, rtstat in the terminal ladder); expect gate flipped to six presence stages (:360-365) + named-red post-pass (:463-484); zero AUTH-SKIP echo paths; **both 18-08-auth-{a,b}.log (550 lines each) carry the full round-trip set** — `secstore: saved`, `AUTH norbert` x2, `[norbert] PUT`, `[norbert] GET`, `AUTH-SECOBS AUTH-ROUNDTRIP-PAYLOAD-18-05`, `AUTH-OK secstore-roundtrip` — plus every pre-existing leg, zero AUTH-FAIL, zero AUTH-SKIP, nvram-prompt absent, redaction present |
| 9 | AUTH-09: writable-/adm + nvram-substitute scoping decided and documented | ✓ VERIFIED (regression held) | README AUTH section now documents the CLOSED round-trip (marker rows :475-476, `### The closed blocker ... DELIVERED` :485) and keeps host-9P /adm documented-not-wired; STATE.md 18-05/AUTH-08 blocker RESOLVED (:266) with evidence cites |

**Score:** 9/9 truths verified (0 present, behavior-unverified)

The prior coincidental-reliance item (AUTH-02's KVM rdrand label held by the host's RDSEED) is RESOLVED: the venue precondition is now declared and pinned as a scripted venue shape (entropy_probe_venue_shapes_stay_correct), and the RDRAND-only arm is tested rather than load-bearing-by-luck.

### Advisory (New Scope, Unevidenced)

New-scope findings from Step 7 with no deterministic evidence — reported, not blocking, do not revert a completed must-have.

| # | Finding | Category | Why Advisory |
|---|---------|----------|--------------|
| 1 | CR-03 (critical, open in 18-REVIEW-DISPOSITION.md): accept-pool sockets keep smoltcp stock Nagle + delayed ACK — >2048B writes into announced connections can hang (x86_64 has no tick_check). Structurally confirmed in code this pass; behavioral consequence NOT reproduced (no failing test, no reproducible artifact); defect site predates the round; no gate exercises it (DEMO object 401B) | architectural | new-scope, no deterministic evidence (#3304 gate) — developer decision requested: accept as tracked open item or plan an interstitial r9 fix (two set_* calls at spawn_listener + the WR-04 pin legs) |
| 2 | WR-03 (warning, open): hostowner write-once-per-boot eve lockout (no donor rename, raw newline stored) | other | deliberate, prohibition-backed divergence with recorded reopener; pinned as intended behavior |
| 3 | WR-04 (warning, open): the two-write pin covers only the client/connect half at 2+31B | other | coverage-extension suggestion; the contracted truth (client pattern) is exactly what the pin asserts |
| 4 | IN-06 / IN-07 (info, open): fatal-on-failure copy redirect in sweep 9d; README present-tense note still names r9 pin d1c4849 (confirmed at README:359-365) | other | doc/robustness nits; no gate verdict affected |

Note on CR-03's blast radius: the claim "the bulk-write (>2048-byte) stall class is closed" (18-07-SUMMARY, 18-08-SUMMARY, STATE.md:266) is TRUE for connect-path sockets only. Phase 19's cpu/exportfs path (announced connections, bulk file traffic) is the natural venue that will either exercise or refute it — the review's fix is small and well-specified.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `../r9/port/src/devcons.rs` (652f025) | iseve gate + 128-byte bound + 3 refusal pins | ✓ VERIFIED | read in source; pins run live 7/7; pre-existing XXX comments at :42/:63 are r9-base code (blame 153aff4/f4df4a8 — pre-phase, outside this round's hunks) |
| `../r9/x86_64/src/entropy.rs` (f594b4c) | cpuid(1,0) rebind + scripted-cpuid seam + 3 probe pins | ✓ VERIFIED | read in source; 9/9 live; TEST_SOURCE retired (comment-only reference remains) |
| `../r9/port/src/devip/proto.rs` + `devip_tests.rs` (ab4cb66) | donor-faithful tcp_socket + two_write pin | ✓ VERIFIED | read in source; pin live 1/1; commit touches exactly these two files (verified) |
| `../r9/flake.nix` (9103522) | FOD re-wire sha256-8uL7mR... | ✓ VERIFIED | wired at :614; **`nix build .#boot-iso` RUN LIVE this pass: exit 0** — committed r9 tree matches the wired NAR hash |
| `build/x86_64/randread.c` (31b9b5c2d) | HOSTOWNER-REFUSED leg 5 | ✓ VERIFIED | :150 refusal marker, fail-fast on success; live-proven in my q35auth run |
| `build/x86_64/auth-sweep.rc` (946da6635) | leg 9d restored, SKIP retired, rtstat laddered | ✓ VERIFIED | :469-483 verified; zero AUTH-SKIP echo paths (comment mentions only) |
| `build/x86_64/smoke-r9-auth.expect` (946da6635) | six presence stages + flipped post-pass | ✓ VERIFIED | :360-365, :463-491; every pre-existing tooth still armed (nvram-absent :386, zero-FAIL :392, DEMO :398, ANNOUNCE :404, DENIED-once :431, tr-ok :438); redaction LAST (:493-504) |
| `evidence/18-08-auth-{a,b}.log` | green-twice, marker-complete, redacted | ✓ VERIFIED | all markers counted present in both; 550 lines each; `<demo-pw-redacted>` mechanism in the gate |
| `evidence/18-06-t2-red-*.json`, `18-07-t2-red-*.json`, `18-07-devip-twowrite-{repro,fix}.console.log` | RED evidence + before/after pair | ✓ VERIFIED | parsed/read this pass — genuine (exact target-test failures pre-fix; qout-33 stall vs mode-2 delivery) |
| `flake.lock` (eaf4879fe) | r9 rev 9103522, lock-only | ✓ VERIFIED | rev == r9 origin/main; **commit-to-commit diff is flake.lock ONLY** |
| `.github/workflows/r9-shell.yml` | CI comment = restored contract | ✓ VERIFIED | :96-105; PENDING FIRST PUSH convention line retained |
| Prior artifacts (authdomain-gen.c, donors x17, authprobe.c, mkimage, flake apps) | regression | ✓ VERIFIED | all present; self-test live-green; flake app wiring intact (:237,:385,:426,:443) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| donor hostownerwrite gate | randread HOSTOWNER-REFUSED -> xtask assert | devcons write gate -> iseve coherence -> devcap #¤ boundary | ✓ WIRED | every hop read in source; the full chain booted green in MY q35auth run (refusal marker present, 0 FAIL) |
| cpuid(1,0) ECX | armed-source print -> finish() venue assert | classify -> has_rdrand leaf-1 bit 30 | ✓ WIRED | source=rdrand printed in my live KVM boot; venue-shape pin guards both labels |
| SConn two-write pattern | delivered pair + framed reply | tcp_write_data -> smoltcp (Nagle off) -> loopback ingress -> parked reader | ✓ WIRED | pin live 1/1; captures show mode-2 round-trip; secstore round-trip green twice on real client bytes |
| 18-06+18-07 kernel bytes | gates green at the pin | two-derivation proof -> FOD re-wire -> push -> lock-only advance | ✓ WIRED | FOD build green live; flake.lock == 9103522 == r9 origin/main; **net-smoke RUN LIVE at the pin: exit 0, ALL STAGES PASS** |
| /boot/secstore-obj.txt | AUTH-OK secstore-roundtrip | venue copy -> PUT -> -G GET -> bytes-back grep -> expect presence demands | ✓ WIRED | sweep :471-483 + expect :360-365/:463-484; verdict emittable ONLY from the delivered payload line |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| sweep 9d verdict | gout (GET stdout) | the real secstored listener over loopback TCP | Yes — AUTH-SECOBS carries the exact committed payload line back | ✓ FLOWING |
| HOSTOWNER-REFUSED | errstr of the refused write | the kernel gate | Yes — refusal string observed live in my boot | ✓ FLOWING |
| armed-source label | CPUID leaves | real (or scripted-pinned) hardware probe | Yes — venue-correct on the live boot; both venue shapes pinned | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| CR-01 refusal pins | `cargo test -p port --lib -- devcons` | 7 passed, 0 failed | ✓ PASS (live) |
| devip two-write pin | `cargo test -p port --lib -- two_write` | 1 passed (named pin in output) | ✓ PASS (live) |
| CR-02 probe pins | `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu -- entropy` | 9 passed, 0 failed | ✓ PASS (live) |
| Full batteries | `cargo test -p port --lib` / `... -p x86_64 --bins ...` | 147/147 and 86/86 — exact claimed counts | ✓ PASS (live) |
| Substrate gate (CR-01 live + venue label) | `cargo xtask boot-test --config q35auth` | exit 0; HOSTOWNER-REFUSED + 4 cap legs + source=rdrand + READS-OK + DONE, 0 FAIL | ✓ PASS (live) |
| Net gate at the pin | `nix run .#boot-r9-net-smoke-x64` | exit 0; ALL STAGES PASS (loopback ping, udp echo, tcp/7 echo) | ✓ PASS (live) |
| FOD integrity | `nix build .#boot-iso` (r9) | exit 0 at wired sha256-8uL7mR... | ✓ PASS (live) |
| Generator self-test (AUTH-01) | `sh build/x86_64/authdomain-selftest.sh` | exit 0; GEN-SELFTEST-OK + controls | ✓ PASS (live) |
| Auth gate green-twice at pin | not re-run this pass (minutes-long double VM boot; fast-check constraint) | retained 18-08-auth-{a,b}.log verified marker-by-marker incl. the full round-trip set; gate machinery committed and stubbed-control-proven | ✓ PASS (capture-based) |

### Probe Execution

No `scripts/*/tests/probe-*.sh` convention in this repo; the phase's probe machinery is the xtask boot-tests + the nix flake-app gates — two of which (q35auth, net-smoke) were executed live by this verification (above).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|------------------|----------|---------|
| AUTH-01 | 18-03, 18-04, 18-08 | hash-paired domain generator | ✓ SATISFIED | self-test live-green; PAIRED DEMO in both pin captures |
| AUTH-02 | 18-01, 18-06, 18-08 | r9 /dev/random from x86_64 entropy | ✓ SATISFIED | CR-02 closed at source; 9/9 pins; live venue label; bytes pinned at 9103522 |
| AUTH-03 | 18-02, 18-06, 18-08 | devcap #¤ auth_chuid path | ✓ SATISFIED | CR-01 closed at source; 7/7 pins; live refusal; bytes pinned |
| AUTH-04 | 18-04, 18-08 | keyfs non-interactive; authsrv tcp/567 | ✓ SATISFIED | captures + wiring |
| AUTH-05 | 18-04, 18-08 | ndb content staged | ✓ SATISFIED | cs answers live in both captures |
| AUTH-06 | 18-05, 18-08 | loopback p9sk1 probe green pre-drawterm; speaksfor | ✓ SATISFIED | both captures; DENIED exactly-once armed |
| AUTH-07 | 18-04, 18-05, 18-08 | serial-visible auth logging | ✓ SATISFIED | tr-ok x4, banner, ANNOUNCE |
| AUTH-08 | 18-05, 18-07, 18-08 | secstore round-trip delivered | ✓ SATISFIED | kernel fix pinned + client round-trip green twice on delivered bytes; SKIP arm retired |
| AUTH-09 | 18-05, 18-08 | scoping decided + documented | ✓ SATISFIED | README closed-blocker section; documented-not-wired posture kept |

No orphaned requirements: all nine AUTH IDs appear across the eight plans' frontmatter; REQUIREMENTS.md shows all nine [x] with closure cites on AUTH-02/03/08 and the traceability rows Complete — statuses match the evidence.

### Decision Coverage

All trackable CONTEXT.md decisions are honored by shipped artifacts (15/15, none missing) — gate output recorded per #2492.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `../r9/port/src/devcons.rs` | 42, 63 | pre-existing `XXX: Just for testing` comments | ℹ️ Info | r9 base code (blame 153aff4/f4df4a8, pre-phase); outside this round's hunks; non-blocking per #3304 (predates round, never flagged) |
| `../r9/xtask/src/main.rs` | 630-631 | pre-existing `FIXME` (disk.bin note) | ℹ️ Info | pre-existing (blame c1568ee); untouched by this round |
| `build/x86_64/README.md` | 359-365 | IN-07: present-tense note names stale r9 pin d1c4849 | ℹ️ Info | doc nit; the closed-blocker section correctly names 9103522 |
| `build/x86_64/auth-sweep.rc` | 471 | IN-06: fatal-on-failure redirect in leg 9d | ℹ️ Info | bounded by the leg-3b live venue probe; failure mode is a mislocalized timeout, not a false green |

No TBD/FIXME/XXX introduced by this round; no stubs, no placeholder returns, no AUTH-SKIP emission paths.

### Prohibition Verdicts (ADR-550, non-authoritative LLM-judge — human review recommended)

All 8 judgment-tier prohibitions across the three gap plans were judged against the code: **all KEPT**. Highlights: the 18-06 rename-deferral prohibition holds (no renameuser ported — the store keeps raw bytes; WR-03 advisory records the residual); the classifier-contract prohibition holds (cpu.rs round diff empty, verified); the no-FOD-in-18-06/07 prohibition holds (per-commit file lists verified: 652f025/f594b4c/ab4cb66 touch no flake; 9103522 is the FOD commit); the no-client-workaround prohibition holds (the `-s 127.0.0.1` flag pins the dial ADDRESS for r9's cs `$attr` ENOENT — it does not dodge the TCP delivery defect, which is fixed in-kernel and pinned); the register/block discipline is untouched (the fix is two socket-config calls; ab4cb66 touches only proto.rs::tcp_socket + the pin); the lock-only prohibition is commit-verified (eaf4879fe diff = flake.lock alone); the verdict-only-on-bytes-back prohibition is wired (sweep :474-483); the add-never-substitute flip prohibition is verified (every pre-existing post-pass still present and armed). Per the autonomous-verify rule these verdicts are NON-AUTHORITATIVE — flagged `unverified-prohibition` for the end-of-phase human checkpoint; none reverses a truth.

### Human Verification

N/A — Infrastructure/foundation phase (kernel, build machinery, CI gates) with no user-facing elements. All acceptance criteria are verified programmatically; every behavior-dependent truth has a passing behavioral test (most re-run live by this verification).

### Gaps Summary

None. All three previously code-confirmed gaps are closed at the source, pinned, pushed, carried into the advanced pin (r9 9103522 == origin/main; Harvey flake.lock lock-only at eaf4879fe), and re-proven: CR-01's refusal and CR-02's venue label booted green in this verification's own q35auth run; the two-write pin and full batteries ran green locally; the FOD, net-smoke, and generator gates ran green locally; the twice-green auth captures carry the full round-trip marker set with every pre-existing tooth armed.

**Escalation note for the developer (advisory, non-blocking):** CR-03 (accept-pool sockets keep Nagle + delayed ACK; >2048B writes into announced connections can hang on x86_64) is open in the project's own disposition record, is structurally confirmed, and is the one finding that Phase 19's cpu/exportfs path is likely to exercise. Options: (a) accept as a tracked open item (already recorded — no override needed, it binds no must-have), or (b) fold the two `set_*` calls at `spawn_listener` + the WR-04 pin legs into the next r9 kernel-byte pass (Phase 19's own FOD cycle would carry it). Also note the STATE.md/SUMMARY "bulk-write class closed" phrasing is connect-path-only true.

---

_Verified: 2026-11-14T19:05:00Z_
_Verifier: Claude (gsd-verifier)_
