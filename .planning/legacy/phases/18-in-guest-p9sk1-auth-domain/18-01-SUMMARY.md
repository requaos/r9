---
phase: 18-in-guest-p9sk1-auth-domain
plan: 01
subsystem: r9 x86_64 kernel entropy substrate + q35auth boot gate
tags: [rdrand, rdseed, entropy, randomhooks, chacha20-drbg, devcons, q35auth, boot-test, kvm, tcg, jitter, tsc, xtask, mkimage]

requires:
  - phase: 17-r9-kernel-flush-out
    provides: r9 main @ 8f3ff11 (== origin/main, the Phase-17 terminus), the devcons RandomHooks seam, the xtask boot-test chassis + marker discipline
provides:
  - "/dev/random re-keyed from x86_64 entropy on every r9 x86_64 boot: CPUID-gated RDSEED -> RDRAND -> two-sample TSC fold, registered through port's untouched RandomHooks seam"
  - "The once-per-boot armed-source fact line `random: source=rdrand` / `random: source=jitter` (the boot-facts family, gate-presence-asserted — T-18-02's no-silent-downgrade tripwire)"
  - "The q35auth substrate boot profile (r9 config + xtask arm + venue assert) green on KVM and TCG, green twice each, four evidence logs retained"
  - "The staged-driver chassis 18-02 extends: randread.c (smoke-out marker lineage) + mkimage's bootfs-substrate.paq bake (randread as the image's /amd64/init, the D-08 leg-1 rule per-image)"
affects: [p9sk1 challenges (keyfs/authsrv /dev/random consumers), 18-02 devcap legs, 18-04/18-05 composed auth gates, every future q35 profile's random]

actuals:
  tokens: 14293   # chars/4 over the realized diff: r9 37463 + harvey chassis 8749 + STRUCTURE.md map delta + the four evidence logs (estimate was 36000)
  tasks: 3
  commits: 4      # r9: f7079e1, 63471ec (pushed, origin/main == HEAD); harvey: 5159a8406 (chassis) + this docs commit

tech-stack:
  added: []        # no new crates/dependencies anywhere (T-18-SC kept)
  patterns:
    - "CPUID-gated dual-mode precedent (13-03 x2APIC/xAPIC) applied to entropy: pure has_rdrand/has_rdseed decision helpers + asm wrappers, host-pinnable without a CPU"
    - "cfg(test) TSC-source seam: a module-level thread_local scripted queue shared by the seam and the test helper, so mixing pins never read the real counter"
    - "Per-image leg-1 marker rule (D-08): each gate image's /amd64/init IS its marker — bootfs.paq keeps smokesleep, bootfs-substrate.paq carries randread"
    - "Device-direct #c/random open from the first exec (boot(8)'s readenv idiom) — no namespace exists before the chain boots"

key-files:
  created:
    - ../r9/x86_64/src/entropy.rs
    - ../r9/x86_64/lib/config_q35auth.toml
    - build/x86_64/randread.c
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-01-q35auth-kvm-a.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-01-q35auth-kvm-b.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-01-q35auth-tcg-a.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-01-q35auth-tcg-b.log
  modified:
    - ../r9/x86_64/src/cpu.rs
    - ../r9/x86_64/src/main.rs
    - ../r9/xtask/src/main.rs
    - ../r9/xtask/src/lib.rs
    - build/x86_64/stage-build.sh
    - build/x86_64/mkimage.sh
    - .planning/codebase/STRUCTURE.md

key-decisions:
  - "The staged marker binary is randread AS the substrate image's /amd64/init (a NEW bootfs-substrate.paq bake), not a binary beside smokesleep's: with no kernel exec-by-name machinery in this plan's file set, the D-08 leg-1 rule per-image is the chassis that runs the driver with zero kernel surface — q35exec's bootfs.paq and its identity guards stay byte-identical"
  - "The driver opens #c/random (device-direct), not /dev/random: the first exec has no namespace (the paqfs /dev is an empty stub), and the boot(8) readenv('#c/user') idiom is the donor-blessed pre-namespace form — same qid, same DRBG"
  - "TCG leg runs R9_ACCEL=tcg R9_CPU=qemu64 (not R9_ACCEL alone): the xtask emitter passes TOML cpu=host through to -cpu, which TCG rejects — the composed gate driver's own documented default (smoke-r9-net-x64.sh step 0b) is the venue truth"
  - "The finish()-time venue assert derives the expected armed label from the RESOLVED accel (flag > R9_ACCEL > TOML) — jitter expected on tcg, rdrand otherwise; a missing line, wrong label, or both labels fails the run (T-18-02)"
  - "The xtask config machine-check family renamed five->all and completed: q35auth joins, plus 17-02's q35churnshell which already carried the full tables (q35churn stays the documented slirp-only exception)"

patterns-established:
  - "Pattern: per-image leg-1 markers — a gate image's /amd64/init is its own marker binary; 18-02's devcap legs and any future substrate leg extend the same driver"
  - "Pattern: armed-source boot facts — security-relevant mode selections print once per boot in the boot-facts window and the gate venue-matches them against the resolved config (extensible to devcap's own facts)"
  - "Pattern: the cfg(test) source seam (module-level thread_local queue + #[cfg]-switched accessor) for pinning timing-dependent mixing logic deterministically"

requirements-completed: [AUTH-02]

coverage:
  - id: D1
    description: "x86_64 entropy module: rng_seed (RDSEED -> RDRAND -> TSC fold, Intel bounded-retry, always-fill) + jitter (two-sample fold) with deterministic host pins"
    requirement: AUTH-02
    verification:
      - kind: unit
        ref: "r9: cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu -- entropy => 6 passed (leaf-bit vectors, max-leaf-first reject, classification order, same-input-reproduces/differing-differs mixing pin, DRNG bounded-retry + fallback)"
        status: pass
    human_judgment: false
  - id: D2
    description: "cpu.rs pure gating helpers (has_rdrand leaf-1 ECX bit 30; has_rdseed leaf-7 EBX bit 18 with max-leaf check first) + rdrand/rdseed asm wrappers (carry-only CF via setc)"
    requirement: AUTH-02
    verification:
      - kind: unit
        ref: "r9: entropy_has_rdrand_keys_off_leaf1_ecx_bit30 + entropy_has_rdseed_keys_off_leaf7_ebx_bit18_maxleaf_first (the qemu64 TCG leaf-1 shape pinned verbatim)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Unconditional set_random_hooks registration in main's boot-facts window (before every consumer) + the once-per-boot armed-source print"
    requirement: AUTH-02
    verification:
      - kind: e2e
        ref: "evidence/18-01-q35auth-kvm-{a,b}.log carry `random: source=rdrand`; 18-01-q35auth-tcg-{a,b}.log carry `random: source=jitter`; /tmp/q35auth-tcg.log (verify copy) re-proven"
        status: pass
    human_judgment: false
  - id: D4
    description: "q35auth boot profile: config + xtask name-selected arm, substrate: FAIL fail-fast, finish()-time accel-venue assert on the armed label"
    requirement: AUTH-02
    verification:
      - kind: e2e
        ref: "KVM leg green twice (Q35AUTH-KVM-GREEN) + TCG leg green three times (TCG-JITTER-GREEN) — READS-OK + substrate: DONE present, zero substrate: FAIL, on all runs; port/devcons.rs and aarch64/ untouched (git diff --stat clean)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Live ring-3 reads: the staged driver reads 2x64 bytes, asserts full-length + pairwise distinct, prints its OWN markers (D-09 discipline)"
    requirement: AUTH-02
    verification:
      - kind: e2e
        ref: "randread's `random: READS-OK` + `substrate: DONE` in all four evidence logs, past the exec-entry anchor, via WRITE syscalls"
        status: pass
    human_judgment: false
  - id: D6
    description: "Substrate image chassis: randread.c + stage-build entry + mkimage §13 bake with identity guards, factotum-absence, KERNDATE determinism, magic checks"
    verification:
      - kind: integration
        ref: "mkimage.sh bake PASS (bootfs-substrate.paq 41025 bytes, header 529ab12b trailer 6b46e688); the bake's rodata identity guards grep the marker strings; boot proved by D4"
        status: pass
    human_judgment: false
  - id: D7
    description: "Push-before-pin posture: r9 main pushed, NO FOD re-wire, NO Harvey flake.lock advance (18-02 owns the one kernel-byte pass)"
    verification:
      - kind: unit
        ref: "git -C ../r9 log origin/main..HEAD empty (8f3ff11..63471ec pushed); git -C harvey status flake.lock clean"
        status: pass
    human_judgment: false
  - id: D8
    description: "Regression batteries stay green after the /dev/random behavior change"
    verification:
      - kind: unit
        ref: "cargo test -p port --lib => 124 passed; cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu => 82 passed; cargo xtask check + clippy clean"
        status: pass
    human_judgment: false

duration: 50min
completed: 2026-11-14
status: complete
---

# Phase 18 Plan 01: x86_64 Entropy Re-Key Summary

**r9's /dev/random re-keyed from CPUID-gated RDSEED/RDRAND with a two-sample TSC-jitter fallback through port's untouched RandomHooks seam, the armed source published per boot, and proven live from ring 3 on KVM and TCG legs green twice each via the new q35auth profile + bootfs-substrate bake.**

## Performance
- **Duration:** 50 min - **Started:** 2026-11-14T01:52:41Z - **Completed:** 2026-11-14T02:42:22Z - **Tasks:** 3/3 - **Files modified:** 13 (7 r9, 6 harvey)

## Accomplishments
- AUTH-02's substrate gap closed: x86_64 registered random hooks for the first time (the deterministic-seed era ended) — every r9 x86_64 boot now seeds the devcons ChaCha20 DRBG from real entropy and prints which source armed
- The armed-source venue tripwire (T-18-02) is mechanical: the xtask gate matches `random: source=rdrand`/`random: source=jitter` against the resolved accel leg and fails on absence, mismatch, or both — a silent downgrade can never pass a gate
- The jitter fold proved non-degenerate under TCG on the first try (three consecutive green TCG runs — no widening needed)
- r9 pushed (8f3ff11..63471ec) with the FOD re-wire and Harvey pin advance deliberately NOT done (18-02's single kernel-byte pass owns them); port/devcons.rs and aarch64/ byte-identical

## Task Commits
1. **Task 1: Entropy gather + hooks registration + armed-source print** - `f7079e1` (feat, r9) — tracer verify re-run green after commit
2. **Task 2: q35auth substrate profile, KVM leg green** - `63471ec` (feat, r9) + `5159a8406` (feat, harvey chassis)
3. **Task 3: TCG leg green twice, green-twice both legs, regression, push** - no new commits (verification-only: the fold held under TCG; files list was conditional)

**Plan metadata:** this SUMMARY commit (docs, harvey)

## Files Created/Modified
- `../r9/x86_64/src/entropy.rs` - the RandomHooks implementations + 6 deterministic host pins (TSC seam, source seam)
- `../r9/x86_64/src/cpu.rs` - has_rdrand/has_rdseed pure helpers + rdrand/rdseed asm wrappers
- `../r9/x86_64/src/main.rs` - `mod entropy;`, unconditional registration, the armed-source print pair
- `../r9/x86_64/lib/config_q35auth.toml` - the substrate profile (dev=["exec"], [dist] module = bootfs-substrate.paq)
- `../r9/xtask/src/main.rs` - the five pinned marker consts, the q35auth arm, Q35SubstratePrints stage, fail-fast, venue assert
- `../r9/xtask/src/lib.rs` - config machine-check family gains q35auth (+q35churnshell)
- `build/x86_64/randread.c` - the substrate driver (churnmark lineage; #c/random device-direct)
- `build/x86_64/stage-build.sh` - randread joins STAGE_SRCS
- `build/x86_64/mkimage.sh` - §13 the SUBSTRATE bake (bootfs-substrate.paq)
- `evidence/18-01-q35auth-{kvm,tcg}-{a,b}.log` - the four green-twice boot captures

## Decisions Made
- Driver-as-init on a NEW image (not beside smokesleep, not a kernel exec-by-name gate, not a typed chain line): the only chassis that runs a compiled marker with zero kernel surface and zero shared-image mutation — see key-decisions
- #c/random device-direct open at the first exec (no namespace exists; boot(8)'s own idiom)
- TCG leg = R9_ACCEL=tcg + R9_CPU=qemu64 (TOML cpu=host is KVM-only; the composed driver's documented default)
- mkimage section numbered §13 (SUBSTRATE); the 18-04/18-05 AUTH contract that follows will be §14 — their plans' "§13k" references were authored expecting to be next; executors navigate by section name

## Deviations from Plan

**1. [Rule 3 - Blocking] Harvey-side marker chassis files beyond files_modified**
- **Found during:** Task 2
- **Issue:** The plan's action text requires a staged compiled in-guest driver ("follow whatever staging chassis q35churn used... the in-guest marker driver is staged exactly that way"), but no plan in the phase stages it, and the compiled-marker chassis (NAME.c + stage-build.sh STAGE_SRCS + mkimage staging) lives Harvey-side while this plan's files_modified lists only r9 paths. Task 2's done criteria (live READS-OK from ring 3) are unachievable without it.
- **Fix:** Minimal additive harvey changes disjoint from sibling 18-03's files: `build/x86_64/randread.c` (new), one STAGE_SRCS word in `stage-build.sh`, mkimage §13 (a new image bake; zero mutation of existing contracts). Committed as `5159a8406`.
- **Files modified:** build/x86_64/{randread.c,stage-build.sh,mkimage.sh}
- **Verification:** bake PASS + KVM/TCG boots green (below)
- **Commit:** 5159a8406

**2. [Rule 1 - Bug] /dev/random open ENOENTed at the first exec**
- **Found during:** Task 2 (first KVM run printed `substrate: FAIL random...`)
- **Issue:** The marker venue has no namespace: /dev in the paqfs root is an empty stub dir, so the plain-path open walks to nothing. q35exec's smokesleep never opens a file, so this never surfaced before.
- **Fix:** randread opens `#c/random` — the device-direct form (boot(8)'s pre-namespace readenv idiom); same qid, same DRBG.
- **Files modified:** build/x86_64/randread.c
- **Verification:** all subsequent runs green
- **Commit:** 5159a8406

**3. [Rule 3 - Blocking] `mod entropy;` declared in main.rs, not lib.rs**
- **Found during:** Task 1
- **Issue:** The plan says "Add `mod entropy;` to x86_64/src/lib.rs" — the x86_64 crate is binary-only (no lib.rs exists); the crate root is main.rs.
- **Fix:** `mod entropy;` in main.rs (beside the other module declarations).
- **Files modified:** ../r9/x86_64/src/main.rs
- **Commit:** f7079e1

**4. [Rule 3 - Documentation] TCG leg invocation carries R9_CPU=qemu64**
- **Found during:** Task 3
- **Issue:** The plan's TCG verify names only `R9_ACCEL=tcg`, but the xtask emitter passes TOML cpu=host through, and `-cpu host` under `-accel tcg` is rejected by qemu (the 13-06 no-silent-downgrade chain has no cpu auto-swap for q35 configs).
- **Fix:** `R9_ACCEL=tcg R9_CPU=qemu64` — exactly the composed gate driver's own default (smoke-r9-net-x64.sh step 0b).
- **Verification:** three consecutive TCG greens
- **Commit:** none (invocation-level)

**5. [Documentation] xtask config test family completed beyond the letter**
- The acceptance criterion said "join it if the machine-check test enumerates configs"; the family renamed five->all, gained q35auth AND 17-02's already-conforming q35churnshell (q35churn stays out — slirp-only by design). Commit 63471ec.

**Total deviations:** 5 (3 auto-fixed Rule 1/3, 2 documentation-class). **Impact:** the Harvey-side chassis is the load-bearing addition — it creates the exact seam 18-02's devcap legs extend; everything else is invocation/naming accuracy.

## Issues Encountered
None beyond the deviations above. The R9_HARVEY_DIR env var must be set for boot-test (pre-existing harness contract, not an issue).

## Authentication Gates
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Ready for 18-02 (wave 2): the q35auth arm, the randread driver chassis, and the bootfs-substrate bake are the extension points for the devcap dev key + cap legs; entropy.rs's decision/fill split keeps any fold-watching pinnable; the FOD re-wire + Harvey flake.lock pin advance remain 18-02's single kernel-byte pass (this plan changed r9 kernel bytes and pushed, per push-before-pin)
- Note for 18-04/18-05: mkimage's AUTH contract lands as §14 (this plan took §13 for the substrate bake — navigate by section name, not number)

Coverage contract: all eight deliverables carry automated verification with pass status; none require human judgment.

## Self-Check: PASSED

All key files exist on disk; all commit hashes verified as ancestors of HEAD in their repos (r9: f7079e1, 63471ec == origin/main; harvey: 5159a8406); evidence logs label-verified (kvm-a/b: rdrand; tcg-a/b: jitter; all four: READS-OK + substrate: DONE, zero substrate: FAIL).
