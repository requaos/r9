---
phase: 18-in-guest-p9sk1-auth-domain
plan: 02
subsystem: r9 kernel auth substrate (devcap #¤, hmac-sha1, per-proc user, eve) + the phase FOD/pin close
tags: [devcap, capability-device, hmac-sha1, sha1, rfc2202, per-proc-user, eve-gate, hostowner, registry-rune, dev_cap, q35auth, boot-markers, kvm, tcg, fod-rewire, flake-lock-pin, net-smoke]

requires:
  - phase: 18-in-guest-p9sk1-auth-domain
    provides: 18-01's q35auth substrate profile (randread driver chassis, bootfs-substrate bake, the xtask marker/venue machinery) and the entropy kernel bytes awaiting this plan's single FOD pass
provides:
  - "devcap `#¤` live and donor-verbatim under the cap dev key: caphash (hostowner-only, Eshort, FIFO-256 oldest-eviction) + capuse (LAST-@ key split, HMAC-SHA1 match, consume-on-match one-shot, FIRST-@ from/to split, user-match, the user mutation) + the eve remove hide"
  - "the AUTH-03 prerequisites: hand-rolled HMAC-SHA1 (RFC 3174/2104, five known-answer pins), the per-proc user (seeded glenda, fork-copied, served by #c/user byte-identically for default boots), and the eve gate (current user vs the HOSTOWNER static, glenda-defaulted)"
  - "the phase's ONE kernel-byte FOD pass closed: boot-iso hash re-wired on a two-derivation byte-identity proof, r9 pushed (origin/main == b2a6151), Harvey flake.lock advanced lock-only, every reachable composed gate green at the new pin"
affects: [Phase 19 cpu server (auth_chuid rides these semantics), 18-04/18-05 composed auth gates (ride the new pin), every future cap-format consumer (as/factotum mkcap contract)]

actuals:
  tokens: 24800   # chars/4 over the realized diff: r9 73784 + harvey chassis/pin 7055 + evidence logs 8018 + this SUMMARY (estimate was 48000)
  tasks: 3
  commits: 7      # harvey-measured 3 from the ledger (77174a23b chassis, d55278f9a pin lock-only, the docs commit) + r9 4 (0358cbe, 82b5cdb, 9ef9390, b2a6151 — pushed, origin/main == b2a6151)
plan_head_before: 154020d1c46710b750f2ca9788b5cf09b31b50c6
plan_head_after: d55278f9a789e2ecc3ab63c5d4132e82425135f2   # the last production commit (the self-referencing docs-commit hash is amended-unstable; the docs commit sits directly atop this)

tech-stack:
  added: []        # no new crates anywhere (T-18-SC kept: SHA-1/HMAC hand-rolled per the ChaCha20 precedent)
  patterns:
    - "The formatted-error channel: a device-local error enum (CapErr::Static | InvalidCapability(Vec<u8>)) when a donor error string is FORMATTED — the port Result's &'static str cannot carry bytes, so the syscall arm translates and lands them in the errstr verbatim (err_dyn)"
    - "The eve base defaults to the seeded glenda while the HOSTOWNER static is empty — the donor's writehostowner-precedes-devcap-use ordering made portable to no-boot(8) venues without changing /dev/hostowner read behavior"
    - "The stub-hook seam shared across pin homes: devcons::tests owns the ONE user-hook registration (Once) and exposes pub(crate) installers, so devcons's own pins and devcap_tests share one test-binary registration"
    - "The boot-fixture cross-pin: the C driver's precomputed cap pair duplicated in a host pin (devcap_boot_fixture_hash_matches_the_module) — python-computed once, Rust-verified forever, the boot leg proves the plumbing"

key-files:
  created:
    - ../r9/port/src/devcap.rs
    - ../r9/port/src/hmacsha1.rs
    - ../r9/port/src/devcap_tests.rs
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-02-q35auth-kvm-a.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-02-q35auth-kvm-b.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-02-q35auth-tcg-a.log
    - .planning/phases/18-in-guest-p9sk1-auth-domain/evidence/18-02-q35auth-tcg-b.log
  modified:
    - ../r9/port/src/lib.rs
    - ../r9/port/src/dat.rs
    - ../r9/port/src/devcons.rs
    - ../r9/port/src/namec.rs
    - ../r9/x86_64/src/proc.rs
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/src/syscall.rs
    - ../r9/xtask/src/main.rs
    - ../r9/Cargo.toml
    - ../r9/x86_64/Cargo.toml
    - ../r9/x86_64/lib/config_q35auth.toml
    - ../r9/flake.nix
    - build/x86_64/randread.c
    - build/x86_64/mkimage.sh
    - flake.lock

key-decisions:
  - "The eve comparison base is the HOSTOWNER static DEFAULTED to the seeded glenda until the first #c/hostowner write: the donor seeds eve empty and relies on boot(8)'s writehostowner before any devcap use — r9's substrate venue has no boot(8) (the marker IS the first exec), so the sane-default glenda carries the gate; /dev/hostowner reads stay byte-identical (EOF until written)"
  - "The donor's formatted `invalid capability %s@%s` error rides a device-local CapErr enum to the x86_64 write arm (err_dyn lands the bytes in the errstr verbatim) — no leak, no Cow refactor of the port Result, the static classes stay &'static str"
  - "The ticks field is recorded from a dev_cap-gated arch tick hook (the devip set_clock precedent) and NEVER read — the donor records m->ticks and checks it nowhere; no time-based expiry invented (D-06's prohibition honored with a #[allow(dead_code)] on the ported dead field)"
  - "devcap::remove is ported and host-pinned but unwired from ring 3: REMOVE sits on r9's loud-unimplemented syscall list this era — the ncapdir hide semantics (caphash LAST, permanent shrink) are proven at the port layer until sys_remove lands"
  - "The namec '#¤' registry arm is the cfg(dev_cap) gate (the module compiles everywhere incl. aarch64; the ATTACH is the device's presence) — dev = [\"exec\", \"cap\"] in config_q35auth.toml + the unexpected_cfgs predeclare in BOTH root and x86_64 Cargo.tomls in the same commits as the first use"

patterns-established:
  - "Pattern: device-local formatted-error enums — any future port device whose donor formats error strings (keyfs/authsrv-class messages) reuses the CapErr/err_dyn channel instead of leaking or refactoring the crate Result"
  - "Pattern: the shared stub-hook seam (one Once registration exposed pub(crate) from the owning module's cfg(test) mod) for hook-based devices with multiple pin homes"
  - "Pattern: precomputed fixture pairs cross-pinned host-side — the driver embeds constants, the pin re-derives them from the implementation, the boot gate proves the wiring end-to-end"

requirements-completed: [AUTH-03]

coverage:
  - id: D1
    description: "hmacsha1: hand-rolled SHA-1 + HMAC (RFC 3174/2104), pure safe Rust, no new crate, known-answer pins beside the code"
    requirement: AUTH-03
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- hmac => 5 passed (FIPS abc a9993e36…, RFC 2202 cases 1/2/3/6 incl. the long-key hashed-first arm)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Per-proc user: x86_64 Proc.user seeded glenda, fork-copied in rfork_copy, served by #c/user through the devcons UserHooks (no trailing newline, byte-identical for default boots)"
    requirement: AUTH-03
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- user => 4 passed (serve-verbatim/no-newline, offsets, eve true/false/mutated, dir-listing length); x86_64 --bins => 83 passed incl. user_seeded_glenda (in the address-space pin) + rfork_copies_the_parent_user"
        status: pass
      - kind: e2e
        ref: "the USE-OK marker's #c/user readback reads `norbert` live on all four boot legs (the first r9 kernel uid mutation observed from ring 3)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The eve gate: current user == the HOSTOWNER static (glenda-defaulted while unwritten) — devcap's caphash open/write gate"
    requirement: AUTH-03
    verification:
      - kind: unit
        ref: "user_eve_gate_default_and_hostowner_write (devcons) + devcap_caphash_is_hostowner_only_on_open_and_write (devcap) — both filters green in the 143"
        status: pass
      - kind: e2e
        ref: "cap: HOSTOWNER-GATE on all four evidence logs (the mutated user's caphash open Eperms, err=devcap: caphash requires the hostowner visible on the marker line)"
        status: pass
    human_judgment: false
  - id: D4
    description: "devcap #¤ donor-verbatim: the 2-file qid tree (caphash LAST, 0222/0200), Eshort classes, FIFO-256 oldest-eviction, consume-on-match one-shot, from/to splits, user-match, mutation, the verbatim error strings, the '¤' registry arm under the cap dev key"
    requirement: AUTH-03
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- devcap => 10 passed (round-trip+mutation, one-shot with the formatted payload verbatim, FIFO-256 eviction, hostowner Eperm open+write, user-match with the burn-before-match order, wrong-HMAC, single-user form, eve-remove hide, Eshort, fixture cross-check); port battery 143/143; x86_64 bins 83/83; cargo xtask check + clippy clean; q35auth kernel builds warning-free under dev_cap"
        status: pass
    human_judgment: false
  - id: D5
    description: "The q35auth devcap marker legs green twice on both accel legs: POST-OK / USE-OK / ONESHOT-REJECT / HOSTOWNER-GATE + substrate: DONE, zero FAIL, alongside the random markers"
    requirement: AUTH-03
    verification:
      - kind: e2e
        ref: "SUBSTRATE-DEVCAP-GREEN (the plan's formal chain); evidence/18-02-q35auth-{kvm,tcg}-{a,b}.log all carry the four cap markers + READS-OK + DONE, zero substrate: FAIL, with venue-correct armed-source lines (rdrand x2, jitter x2)"
        status: pass
    human_judgment: false
  - id: D6
    description: "The phase's ONE kernel-byte FOD pass: boot-iso hash re-wired on a two-derivation byte-identity proof"
    requirement: AUTH-03
    verification:
      - kind: other
        ref: "derivations 37v6cap1p8k2jml048vzf9i6gbg0g49i (kernel A plain @ 9ef9390) and zdhwwg8s2km6nsijzpb0ln3xnkgpafbc (kernel B @ 42e0a83, kernel-neutral marker clone) through the non-FOD lib.bootIso: cmp-clean ISOs, file sha256 3a5b4d81645dab237166254ed9d0bca1f724a3969ab6fb1ccc7244a3851774f6; NAR sha256-im+zCIAVCyYvKy36dZ0d4FQUHSHFd9YyXFgV8Iap8pU= (base16 8a6fb308…9f295) three-way-agreed with the FOD's own got-value; nix build .#boot-iso + --rebuild green (r9 b2a6151)"
        status: pass
    human_judgment: false
  - id: D7
    description: "Push-before-pin + lock-only pin advance: r9 main pushed, Harvey flake.lock r9 input advanced with zero other changes"
    requirement: AUTH-03
    verification:
      - kind: unit
        ref: "PUSHED-CLEAN (origin/main == b2a6151, unpushed list empty) + PIN-LOCK-ONLY (git diff HEAD~1 -- flake.nix empty; the pin commit touched flake.lock alone)"
        status: pass
    human_judgment: false
  - id: D8
    description: "Every reachable existing composed gate re-greened at the advanced pin (8f3ff11 -> b2a6151)"
    requirement: AUTH-03
    verification:
      - kind: integration
        ref: "boot-r9-net-smoke-x64 ALL STAGES PASS (exit 0); boot-r9-smoke-x64 ALL STAGES PASS; boot-r9-tier1-x64 ALL STAGES PASS (zero T1-FAIL); boot-r9-churn-x64 ALL STAGES PASS (heap 84 B/exit <= 1772 — the new per-proc user Vec leaves no residue); boot-r9-x64 (interactive compose) reached its designed rc `% ` terminal prompt; no mkimage witness trip (this plan adds no Harvey /amd64/bin bytes)"
        status: pass
    human_judgment: false

duration: 41min
completed: 2026-11-14
status: complete
---

# Phase 18 Plan 02: devcap #¤ + the AUTH-03 Substrate Summary

**devcap `#¤` ported donor-verbatim onto hand-rolled HMAC-SHA1, a per-proc user, and an eve gate — proven live from ring 3 on KVM and TCG legs green twice each, and the phase's one kernel-byte FOD pass closed: hash re-wired on a two-derivation proof, r9 pushed, Harvey pinned lock-only, every reachable gate green at the new pin.**

## Performance
- **Duration:** 41 min - **Started:** 2026-11-14T03:11:03Z - **Completed:** 2026-11-14T03:52:27Z - **Tasks:** 3/3 - **Files modified:** 19 (13 r9, 6 harvey incl. evidence)

## Accomplishments
- AUTH-03's substrate is live: a ring-3 process posts a capability hash as the hostowner, presents the cap, mutates its own kernel user to `norbert`, reads it back from #c/user, gets the donor's formatted one-shot rejection on re-present, and loses caphash access as the mutated user — all four markers on all four boot legs (KVM rdrand x2, TCG jitter x2)
- The first r9 kernel uid mutation landed with its full prerequisite stack: the per-proc user (seed/fork/serve, byte-identical for glenda boots), the eve gate (HOSTOWNER-based, glenda-defaulted), and HMAC-SHA1 (five known-answer pins, zero new crates)
- The donor-verbatim semantics carry 10 deterministic host pins — including the two subtle orders donors hide: the FIFO-256 oldest-eviction (not one-shot-per-post) and remcap-before-user-match (a mismatched cap BURNS)
- The phase's single kernel-byte FOD pass closed exactly per the 13-06/17-XX discipline: two independent derivations, byte-identical ISOs, three-way NAR agreement, push-before-pin, lock-only advance, gates re-greened

## Task Commits
1. **Task 1: HMAC-SHA1 + per-proc user + eve gate** — r9 `0358cbe` (feat)
2. **Task 2: devcap.rs donor-verbatim port + '¤' registry arm + dev key** — r9 `82b5cdb` (feat; includes the late-added clear_stub_hostowner helper via autosquash)
3. **Task 3: devcap markers green-twice both legs, FOD re-wire, push, pin advance** — r9 `9ef9390` (feat, markers) + r9 `b2a6151` (fix, FOD re-wire) + harvey `77174a23b` (feat, chassis + evidence) + harvey `d55278f9a` (chore, pin lock-only)

**Plan metadata:** this SUMMARY + STATE/ROADMAP/REQUIREMENTS (docs commit follows)

## Files Created/Modified
- r9 `port/src/devcap.rs` — the capability device (dirtab with caphash LAST, addcap/remcap FIFO-256, capuse parse/match/mutate, stat/read/remove, CapErr)
- r9 `port/src/hmacsha1.rs` — SHA-1 + HMAC, five KAT pins
- r9 `port/src/devcap_tests.rs` — the 10-class pin home over the shared stub seam
- r9 `port/src/devcons.rs` — UserHooks + current_user/set_current_user + iseve + the per-proc /dev/user serve (USER_CONTENT retired) + the cfg(test) seam module
- r9 `port/src/namec.rs` — the cfg(dev_cap) '#¤' registry arm + DEV_CAP walk/open/stat dispatch
- r9 `port/src/{lib.rs,dat.rs}` — module registrations + DEV_CAP=10
- r9 `x86_64/src/proc.rs` — Proc.user (seed + rfork copy) + the hook fns + 2 pins
- r9 `x86_64/src/{main.rs,syscall.rs}` — hook registrations (user unconditional, tick under dev_cap) + the DEV_CAP read/write arms + err_dyn
- r9 `{Cargo.toml,x86_64/Cargo.toml,xtask/src/main.rs,x86_64/lib/config_q35auth.toml,flake.nix}` — the dev key predeclares, dev=["exec","cap"], the four marker consts + finish asserts, the FOD re-wire
- harvey `build/x86_64/randread.c` — capleg() over the precomputed fixture
- harvey `build/x86_64/mkimage.sh` — §13 identity guards for the cap markers
- harvey `flake.lock` — r9 8f3ff11 → b2a6151
- `evidence/18-02-q35auth-{kvm,tcg}-{a,b}.log` — the green-twice captures

## Decisions Made
- The eve default is glenda while HOSTOWNER is unwritten (the donor's empty-eve + boot-writehostowner ordering cannot survive a venue whose first exec IS the marker — see key-decisions)
- The formatted error rides CapErr to an err_dyn landing (no leak, no crate Result refactor)
- devcap::remove ported+pinned but unwired (REMOVE is loud-unimplemented in r9's syscall table — the port layer carries the semantics until sys_remove)
- The boot fixture is a fixed constant pair cross-pinned host-side (the flagged fixture assumption executed as planned: pins prove the implementation, boots prove the plumbing)

## Deviations from Plan

**1. [Rule 3 - Blocking] Harvey-side chassis edits beyond files_modified**
- **Found during:** Task 3
- **Issue:** The plan's action requires extending the staged in-guest driver with the devcap legs, but the driver (randread.c) and its bake guards (mkimage §13) live Harvey-side while files_modified lists only r9 paths + flake.lock — 18-01's deviation #1, repeated verbatim in shape.
- **Fix:** Minimal additive harvey changes: capleg() in randread.c (the fixture legs) + four marker greps in mkimage §13a + one in §13d. Committed as 77174a23b with the evidence logs.
- **Files modified:** build/x86_64/{randread.c,mkimage.sh}
- **Verification:** bake PASS + the four green boot legs
- **Commit:** 77174a23b

**2. [Rule 3 - Blocking] The formatted error needed a channel the port Result lacks**
- **Found during:** Task 2
- **Issue:** The donor's `invalid capability %s@%s` is FORMATTED with user bytes; port's `Result<T, &'static str>` cannot carry them (a leaked String per failed present would be an unbounded-kernel-heap DoS on a world-writable surface).
- **Fix:** A device-local `CapErr { Static(&'static str), InvalidCapability(Vec<u8>) }` on devcap::write; the x86_64 sys_pwrite DEV_CAP arm translates — Static through err, the formatted bytes through a new err_dyn that lands them in the errstr verbatim (lossy UTF-8, a diagnostic surface). The verbatim pin asserts the exact bytes.
- **Files modified:** ../r9/port/src/devcap.rs, ../r9/x86_64/src/syscall.rs
- **Verification:** the one-shot pin's payload assert + the ONESHOT-REJECT marker's errstr text on all four legs
- **Commit:** 82b5cdb

**3. [Rule 2 - Correctness] The eve base defaults to glenda while HOSTOWNER is unwritten**
- **Found during:** Task 1 (design) — confirmed load-bearing at Task 3's first KVM green
- **Issue:** The donor seeds eve EMPTY and relies on boot(8)'s writehostowner before any devcap use; r9's substrate venue has no boot(8) (the marker IS the first exec), so a literal empty-static comparison would Eperm the very first caphash post — the marker leg unachievable.
- **Fix:** iseve() compares against HOSTOWNER-or-the-seeded-glenda; /dev/hostowner READ behavior unchanged (EOF until written). Pinned true/false/mutated.
- **Files modified:** ../r9/port/src/devcons.rs
- **Verification:** the eve pin + the POST-OK/HOSTOWNER-GATE markers on all four legs
- **Commit:** 0358cbe

**4. [Rule 3 - Blocking] x86_64/Cargo.toml also needed the dev_cap predeclare**
- **Found during:** Task 2
- **Issue:** The plan names only the ROOT Cargo.toml's unexpected_cfgs, but main.rs's `#[cfg(dev_cap)]` tick-hook registration consumes the cfg from the x86_64 crate (which carries its own list).
- **Fix:** Added to both lists in the same commit as the TOML/code (the declared rule).
- **Files modified:** ../r9/x86_64/Cargo.toml
- **Verification:** the q35auth kernel build is unexpected_cfgs-warning-free
- **Commit:** 82b5cdb

**5. [Rule 1 - Bug] RFC 2202 case-6 pin constant corrected**
- **Found during:** Task 1's first test run
- **Issue:** My from-memory case-6 digest (aa4ae5e1…) was wrong; python's reference (matching the RFC on cases 1-4) and the Rust module agree on 90d0dace….
- **Fix:** Corrected the pin constant; the implementation was already correct.
- **Files modified:** ../r9/port/src/hmacsha1.rs
- **Verification:** 5/5 hmac pins green
- **Commit:** 0358cbe

**Total deviations:** 5 (3 auto-fixed Rule 1/3, 1 Rule-2 correctness, 1 pin-constant correction). **Impact:** deviations 2 and 3 are the load-bearing design calls — the formatted-error channel and the eve default are what make the donor's observable contract survive the port's type system and venue; everything else is chassis/tooling accuracy.

## Issues Encountered
- boot-r9-services-x64 was NOT re-run: it stays blocked by the pre-existing #68 dns-stage kernel trap (STATE 17-06/17-07 — A/B-identical pre-existing, waived with the CR-07 record; not a regression of this plan, which adds no services-image bytes)
- boot-r9-stress-x64 is the waived #66 reproduction instrument, not a gate to green; boot-r9-net-soak-x64 (the 45-min soak) is outside the note's enumerated re-green set (smoke/tier1/services/net) and was not run
- boot-r9-x64 is the interactive compose gate: verified to its designed terminal state (the rc `% ` prompt reached at the new pin), then stopped — it parks for typed input by design

## Authentication Gates
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Ready for 18-04 (wave 3): the AUTH image contract rides the new pin b2a6151; devcap's semantics are pinned for any future /adm-side consumer; mkimage's AUTH contract lands as §14 (18-01's numbering note stands); the netaddr/ndb and listen1 tier work is 18-04/18-05's own
- Phase 19 note: auth_chuid's kernel substrate (per-proc user mutation via capuse) is proven live end-to-end — the cpu server consumes exactly these semantics

Coverage contract: all eight deliverables carry automated verification with pass status; none require human judgment.

## Self-Check: PASSED

All key files exist on disk (the three new r9 modules, the four evidence logs, the extended driver); all commit hashes verified as ancestors of HEAD in their repos (r9: 0358cbe, 82b5cdb, 9ef9390, b2a6151 — the last also on origin/main; harvey: 77174a23b, d55278f9a); evidence logs label-verified (kvm-a/b: source=rdrand; tcg-a/b: source=jitter; all four: POST-OK + USE-OK + ONESHOT-REJECT + HOSTOWNER-GATE + substrate: DONE, zero substrate: FAIL).
