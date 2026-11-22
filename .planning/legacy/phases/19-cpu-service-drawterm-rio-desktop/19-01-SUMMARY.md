---
phase: 19-cpu-service-drawterm-rio-desktop
plan: 01
subsystem: kernel
tags: [mb2, grub, bootconf, plan9-ini, cmdline, r9, x86_64, mkpaqfs, iso]

# Dependency graph
requires:
  - phase: 13-x86_64-platform-bringup
    provides: the mb2/GRUB2 -cdrom boot channel and the mb2 info parser
  - phase: 14-harvey-userspace-on-r9
    provides: the exec substrate, the boot chain image contracts (mkimage §1-§14), and the one-image-per-gate discipline
provides:
  - r9's cmdline-to-ROOT-env-seed plumbing (mb2 tag 1 -> bootconf pairs -> the seeded env group; 256-byte loud cap)
  - the append seam — the grub.cfg @R9-APPEND@ token consumed by BOTH ISO emitters (xtask build_x86_64_iso + flake r9-iso-builder --append=) with empty-append byte-stability
  - the [dist] append config schema key and the committed config_q35cpu.toml boot profile
  - the q35cpu boot-test gate (stage machine + whole-capture post-pass: bootconf line present, failure-loop symptom count 0, cpurc marker)
  - mkimage §15 — the cpu-image seed contract baking out/bootfs-cpu.paq (chain tree + the cpurc fixture at /rc/bin/cpurc)
affects: [19-02 (#! exec replaces the binary fixture), 19-05 (pin advance over these kernel bytes), 19-07 (full seventh contract expands §15), 19-09 (--system rides the append seam)]

# Actuals (#2632) — pairs with the plan's estimate (48000 tokens, low confidence).
# chars/4 over the realized diff (both repos: 73,913 chars).
actuals:
  tokens: 18478
  tasks: 3
  commits: 2

# Tech tracking
tech-stack:
  added: []  # zero external packages — in-tree sources only (T-19-SC honored)
  patterns:
    - "the boot-protocol config channel: GRUB -append -> grub.cfg token -> mb2 tag 1 -> kernel-parsed bootconf pairs MERGED into the seeded ROOT env group (image-scoped service identity, no forked boot binaries)"
    - "template-token emission with byte-stability pins: @R9-APPEND@ joins @R9-MODULE@ — every no-append emission provably byte-identical (unit test + real-ISO extract round-trip)"

key-files:
  created:
    - ../r9/x86_64/lib/config_q35cpu.toml
  modified:
    - ../r9/x86_64/src/platform/q35/mb2.rs
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/lib/grub.cfg
    - ../r9/xtask/src/main.rs
    - ../r9/xtask/src/config.rs
    - ../r9/xtask/src/lib.rs
    - ../r9/flake.nix
    - build/x86_64/mkimage.sh

key-decisions:
  - "The bootconf pair parse is donor plan9.ini's one-line analog: whitespace tokens split at the FIRST '=' (embedded '=' literal in values), and a no-'=' token CONTINUES the previous pair's value — the shape init=/amd64/init -c requires (A4 verified: boot.c execinit tokenizes the multi-word getenv(\"init\") value verbatim; the cpuflag-boot-build fallback was not needed)"
  - "The §15 cpurc fixture is a MARKER BINARY (inline 6c/6l, the 14i2 precedent), not a script: rcmain eval's $cflag as a command word and rc exec's /rc/bin/cpurc directly (searchpath's absolute-path arm), so an rc script cannot run until 19-02 lands kernel #! — the binary prints cpurc: RAN and parks forever (a returning fixture would let init's terminal fallback print one 'init: starting /bin/rc' — normal post-cpurc behavior, but the gate's zero-symptom truth requires the park)"
  - "q35cpu carries dev = [exec, bootchain] WITHOUT ether/ip — the tracer needs no net; the carried netdev stanza keeps the q35 discovery contract honest the q35auth way (NIC present, unbound, pcap off)"
  - "Overlong cmdlines are rejected loudly at BOTH ends: mb2 parse (CmdlineTooLong, first boot) and compose time (xtask validate_append + the flake builder's --append= checks) — a mis-staged ISO never silently truncates"

patterns-established:
  - "Append-token emission: doc lines drop BEFORE the marker scan (the committed template embeds the marker literal in backticks; marker-first order mangles doc comments — fixed here, pinned by the module-bearing byte-stability test)"
  - "Cross-repo gate naming: the q35cpu stage machine names its markers (devenv: bootconf / cpurc: RAN) and its prohibitions (init: starting count 0) — presence-based, never timing"

requirements-completed: [CPU-02]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "r9 kernel consumes the mb2 cmdline tag into the ROOT env seed: tag-1 parse with the 256-byte loud cap, donor-plan9.ini pair parse (multi-word values, embedded '='), merge into seed_root_group, the distinct devenv: bootconf print"
    requirement: CPU-02
    verification:
      - kind: unit
        ref: "r9 x86_64 bin tests (cargo test -p x86_64 --bins -- mb2): cmdline_tag_parses_and_bootconf_pairs_merge + cmdline_and_bootconf_violations_are_loud — 9 passed"
        status: pass
      - kind: integration
        ref: "cargo xtask boot-test --config q35cpu — the devenv: bootconf init=/amd64/init -c line on serial (target/boot-test-q35cpu.log)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The append seam: grub.cfg @R9-APPEND@ token + both ISO emitters (xtask emitter and flake r9-iso-builder --append=) + the config schema key, with empty-append byte-stability"
    requirement: CPU-02
    verification:
      - kind: unit
        ref: "r9 xtask tests: grub_cfg_kernel_only_emission_is_phase13_identical (both no-append shapes), grub_cfg_append_substitutes_on_the_multiboot_line, append_validation_is_loud_at_compose_time — 35 lib + 75 bin passed"
        status: pass
      - kind: integration
        ref: "real r9-iso-builder kernel-only ISO round-trip: extracted /boot/grub/grub.cfg byte-identical (cmp) to Phase 13's config; --append leg carries the bootconf on the multiboot2 line; overlong/newline/whitespace appends refuse exit 1"
        status: pass
    human_judgment: false
  - id: D3
    description: "mkimage §15 cpu-image seed: chain tree + cpurc fixture baked to out/bootfs-cpu.paq (399724 bytes) with the §15-named manifest print, per-file cap assert, KERNDATE determinism, header/trailer magic checks; every earlier contract re-baked green in the same run"
    requirement: CPU-02
    verification:
      - kind: manual_procedural
        ref: "sh build/x86_64/mkimage.sh — '== PASS: baked .../bootfs-cpu.paq (399724 bytes; chain + the §15 cpurc fixture ...)' plus the 14 'cpu manifest (§15):' lines; all 8 prior PASS lines intact"
        status: pass
    human_judgment: false
  - id: D4
    description: "The end-to-end tracer green twice + control leg: q35cpu boots the §15 paq through the append seam (cpurc: RAN, bootconf line, zero init-failure-loop lines) on KVM twice consecutively and once on TCG; q35shell control leg green (terminal path untouched); full battery green; r9 pushed to origin/main"
    requirement: CPU-02
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --config q35cpu (KVM x2: CPU-BOOTCONF-TRACER-GREEN; TCG x1: TCG-CPU-LOG-GREEN) + --config q35shell (CHAIN-CONTROL-GREEN, KVM x2 + TCG x1) + era gates q35exec/q35auth/q35churn KVM x2 + CI channel q35/q35lowmcfg/q35net TCG + cargo xtask test x2 (147 port lib) + PUSHED-CLEAN at r9 e491ed5"
        status: pass
    human_judgment: false

# Metrics
duration: 32 min
completed: 2026-11-14
status: complete
---

# Phase 19 Plan 01: CPU Service Tracer — cmdline bootconf Summary

**The service=cpu boot architecture proven end-to-end on the kernel layer: GRUB -append -> grub.cfg @R9-APPEND@ -> mb2 tag-1 cmdline -> r9's donor-plan9.ini bootconf pairs merged into the seeded ROOT env group -> boot(8) execinit -> init -c -> cpurc runs (cpurc: RAN, zero init-loop lines), image-scoped with byte-stable no-append emissions**

## Performance

- **Duration:** 32 min
- **Started:** 2026-11-14T21:45:01Z
- **Completed:** 2026-11-14T22:17:09Z
- **Tasks:** 3
- **Files modified:** 9 (8 in r9, 1 in harvey)

## Accomplishments

- The mb2 parser consumes the cmdline tag (type 1): NUL-validated string into a 256-byte Facts buffer, overlong rejected loudly at parse; `parse_bootconf_pairs` walks the donor plan9.ini one-line shape (first-'=' split, embedded '=' literal, no-'=' tokens continue the previous value — the `init=/amd64/init -c` multi-word shape boot.c's execinit tokenizes verbatim, A4 verified in-phase)
- main.rs's exec prologue MERGES the bootconf pairs into the compiled `seed_root_group` seed (append, never replace — T-19-01) and prints the cmdline-sourced keys on their own `devenv: bootconf` line; the existing `devenv: seeded` line's bytes never moved; empty/absent cmdline = the pre-19-01 boot exactly
- The append seam with empty-append byte-stability: the grub.cfg template token, the xtask emitter (with compose-time validation: content, the 256-byte kernel cap, printable-only), the flake `r9-iso-builder --append=` parameter (same validation), the `[dist] append` config key, and `config_q35cpu.toml` (dev = [exec, bootchain], module = harvey:build/x86_64/out/bootfs-cpu.paq, append = init=/amd64/init -c)
- The q35cpu boot-test gate: discovery -> module/paqfs -> exec entry -> boot(8)'s prints -> the cpurc marker, with the failure-loop symptom (`init: starting /bin/rc`) a fail-fast and the bootconf line asserted whole-capture at finish
- mkimage §15 (the seventh contract's seed form): the chain tree + the cpurc fixture (a marker binary — rc exec's the file directly pre-#!; inline 6c/6l, prints its marker and parks so init's terminal fallback never runs), the §15-named manifest print, the 14n/14o guard classes; all eight prior contracts re-baked green in the same run

## Task Commits

Each task was committed atomically (cross-repo: kernel work lands in r9 `main`, push-before-pin; the harvey-side file lands here):

1. **Task 1: mb2 cmdline tag to ROOT env seed — kernel consumption** - r9 `8b14ecb` (feat)
2. **Task 2: the append seam — grub.cfg template, both ISO emitters, config schema, q35cpu profile** - r9 `db8c646` (feat)
3. **Task 3: §15 seed contract + end-to-end tracer boot green twice + battery + push** - harvey `5aae21d46` (feat, mkimage §15) + r9 `e491ed5` (fix, the bootconf print's from_utf8 under no_std); r9 pushed to origin/main at `e491ed5` (PUSHED-CLEAN)

**Plan metadata:** (docs: complete plan — below)

## Files Created/Modified

- `../r9/x86_64/src/platform/q35/mb2.rs` - tag-1 parse (NUL-validated, 256-byte cap), the Bootconf pair parser (donor plan9.ini shape), 2 new test functions
- `../r9/x86_64/src/main.rs` - the exec-prologue bootconf merge + the `devenv: bootconf` print
- `../r9/x86_64/lib/grub.cfg` - the @R9-APPEND@ token on the multiboot2 line + @R9-APPEND-DOC@ documentation
- `../r9/xtask/src/main.rs` - emit_grub_cfg's append parameter (Result-typed, loud drift error), validate_append, the q35cpu gate profile (stage machine + finish post-pass), emitter/append unit tests
- `../r9/xtask/src/config.rs` - Dist's `append` key
- `../r9/xtask/src/lib.rs` - config_q35cpu joins the committed-TOML machine check
- `../r9/x86_64/lib/config_q35cpu.toml` - NEW: the tracer profile (module + append + carried netdev venue)
- `../r9/flake.nix` - r9-iso-builder's optional --append=CMDLINE (usage documented, validated, awk/sed emission arms)
- `build/x86_64/mkimage.sh` - §15: the cpu-image seed bake (out/bootfs-cpu.paq)

## Decisions Made

- **The no-'=' continuation rule** (the pair parse's key semantic): a whitespace token without '=' extends the previous pair's value — the one-line analog of plan9.ini's value-runs-to-end-of-line. Required by the must-have's own bootconf line (`init=/amd64/init -c` would otherwise drop `-c` and boot the terminal path); A4 verified against boot.c:208-214 before committing to it (the cpuflag boot-build fallback not needed)
- **The §15 fixture is a marker binary, not a script** (mechanism derivation under Claude's discretion): `rc -c /rc/bin/cpurc` rides rcmain's `eval $cflag`, and rc's searchpath/Execute exec's the absolute path DIRECTLY — a text file cannot run without kernel #! (19-02's CPU-01). The binary parks forever so init's terminal fallback loop never prints (zero-symptom truth). The expansion point is documented in the section comment
- **q35cpu without ether/ip dev keys**: the tracer's cpurc fixture needs no net; the discovery contract stays honest via the carried (unbound) netdev stanza — the q35auth precedent

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Doc-marker lines dropped before the module-marker scan in emit_grub_cfg**
- **Found during:** Task 2 (the new module-bearing byte-stability pin)
- **Issue:** The committed template's documentation embeds the marker literal in backticks (`` `# @R9-MODULE@ ` ``); the marker-find-first order "uncommented" that mid-line occurrence through the module branch, leaving a mangled half-stripped doc comment in every module-bearing ISO emission (latent since 14-05 — never caught because only the kernel-only shape was pinned)
- **Fix:** Doc-marker lines drop WHOLE before the marker scan (the placeholder line never starts with a doc marker); both no-append shapes now pinned byte-exact against the committed template
- **Files modified:** ../r9/xtask/src/main.rs
- **Verification:** grub_cfg_kernel_only_emission_is_phase13_identical (module-bearing arm added) + grub_cfg_module_emission_carries_the_module2_line; all 75 xtask bin tests green
- **Committed in:** r9 db8c646 (part of Task 2's commit)

**2. [Rule 3 - Blocking] Two no_std compile blockers in the bootconf print**
- **Found during:** Task 3 (the first q35cpu build)
- **Issue:** `core::str::from_utf8_lossy` (then `alloc::str::from_utf8_lossy`) is not available in this build's core/alloc split; a closure-based helper tripped a lifetime error
- **Fix:** a local `fn show(&[u8]) -> &str` over `core::str::from_utf8(...).unwrap_or("<?>")` — the compose-time seam already validates printable ASCII, so from_utf8 holds for every sanctioned bootconf
- **Files modified:** ../r9/x86_64/src/main.rs
- **Verification:** full battery green after the fix (KVM x2, TCG, host tests)
- **Committed in:** r9 e491ed5

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both fixes necessary and local; no scope creep. The Task 2 fix strengthens an existing emitter (byte-stability now pinned for BOTH shapes).

## Issues Encountered

- A one-off failure in `cargo test -p port --lib` during Task 2 verification (146 passed / 1 failed, name not captured) that never reproduced across 9 subsequent identical runs (147 passed each); port/ is untouched by this plan — recorded as an observation, not chased (scope boundary). If it recurs in later plans' batteries, capture the test name there.
- The first `git push` attempt failed with an SSH access-rights error and succeeded unchanged on immediate retry (transient); `PUSHED-CLEAN` verified after.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The service=cpu architecture is proven on the kernel layer and pushed (r9 at `e491ed5` == origin/main); 19-02 (#! exec), 19-03, 19-04 build on the flushed-out kernel tree; 19-05 owns the flake.lock pin advance over these kernel bytes (never advanced here, per the cross-repo discipline)
- The kernel-byte changes mean the boot-iso FOD hash re-stales structurally (the standing 13-06/14-07 discipline): 19-05's re-wire-then-pin pass covers it; the grub.cfg-side empty-append byte-stability is already proven (unit + real-ISO extract), so the FOD drift is kernel-bytes-only
- 19-07 expands §15: the real cpurc script (needs 19-02's #!), the cpu suite payload, and the seam asserts — the expansion points are named in the section comment
- The q35cpu gate is live in the boot-test chassis and CI-able (TCG leg green)

## Self-Check: PASSED

- Created file exists: ../r9/x86_64/lib/config_q35cpu.toml FOUND (committed r9 db8c646)
- Bake artifact exists: build/x86_64/out/bootfs-cpu.paq FOUND (399724 bytes, gitignored by design)
- Task commits exist on their repos' histories: r9 8b14ecb, db8c646, e491ed5 (== origin/main) FOUND; harvey 5aae21d46 FOUND
- Acceptance criteria re-verified: all Task 1/2/3 criteria pass (see coverage block)

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-14*
