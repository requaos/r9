---
phase: 13-r9-x86-64-platform-bring-up
plan: 06
subsystem: kernel
tags: [x86_64, q35, nix, flake, kernel-x64, boot-iso, reproducible-builds, fixed-output-derivation, grub-mkrescue, xorriso, kvm, tcg, ci, github-actions, evidence]

# Dependency graph
requires:
  - phase: 13-r9-x86-64-platform-bring-up (13-01..13-05)
    provides: the three green q35 gates (q35/q35lowmcfg/q35net) whose binary the package ships, the xtask ISO dev-copy mechanics the flake derivation mirrors scrubbed, and the five SUMMARYs this plan's evidence rolls up
provides:
  - r9.packages.<system>.kernel-x64 — the Phase-14 compose contract (the kernel-virt derivation shape re-keyed, built by the SAME `cargo xtask build --arch x86-64 --config q35` the gates prove)
  - r9.packages.x86_64-linux.boot-iso — the byte-reproducible BIOS-only grub-mkrescue fixed-output derivation (double-build-proven ISO sha256 87a96c23a421…, --rebuild re-verifies)
  - The KVM-native default proven across ALL THREE q35 profiles in one loop at the pinned HEAD, with the TCG fallback run for q35 + q35net (plus q35lowmcfg in the CI-sequence validation) — X64-05 closed
  - The boot-test-x86-64 CI leg (r9 xtask.yml): self-provisioned, explicitly TCG, locally validated + parse-checked; hosted observation PENDING FIRST PUSH
  - build/x86_64/README.md — the x86_64 companion (boot chain + the two X64-01 recorded decisions, accel contract + the no-silent-downgrade rule, timer chain, qemu-11.1.2 drift notes incl. the -global MCFG crash routed from deferred items)
  - 13-EVIDENCE.md + evidence/ — the consolidated, r9-SHA-pinned (cc29c7a) phase evidence bundle (X64-01..05 mapped)
affects: ["Phase 14 XEXEC (composes kernel-x64; swaps the exec dd path in at the netgate seam)", "Phase 16 HNET (the no-silent-downgrade doctrine)", "the first r9 push to origin (the hosted CI leg observation)"]

# Actuals (#2632) — pairs with the plan's `estimate` to calibrate future estimates.
# Same estimateTokens scale (chars/4 over the realized diff), never a harness token count.
actuals:
  tokens: 9747     # chars/4 over the realized diff: r9 0cf9c3a..cc29c7a (9102 chars, 2 files) + harvey authored text (29886 chars); the 663-KiB pcap is a copied gate artifact, excluded from authoring tokens. Plan estimated 52000, confidence low — the overestimate direction again
  tasks: 3
  commits: 2      # MEASURED: git rev-list --count a2cec42d..HEAD at SUMMARY write (README 6ab8f91b5 + evidence 283db3390); this SUMMARY/metadata commit makes 3. Production code: 2 commits in the sibling r9 repo (edd3d97, cc29c7a)
plan_head_before: a2cec42d90d1c4ac0a2903ad0fcb4071691c9aed
plan_head_after: 283db3390   # harvey-side pre-docs HEAD; the kernel/CI production commits live in ../r9 (0cf9c3a..cc29c7a)

# Tech tracking
tech-stack:
  added: []        # zero new crates — grub2/xorriso were already devshell inputs from 13-01; the derivation consumes the same nixpkgs packages
  patterns:
    - "Reproducible grub-mkrescue under nix: a --xorriso wrapper that rewrites grub-mkrescue's injected current-time --modification-date and injects --set_all_file_dates + pinned -volid into the mkisofs option region — SOURCE_DATE_EPOCH alone is insufficient on grub 2.14"
    - "strip-all + store-hash zero-zap for FOD-consumed kernels: rustc's per-invocation-random .llvm.<u64> anon symbols force symtab-free staging, and build-std's toolchain-path panic strings need a length-preserving hash scrub (nix's FOD scanner keys on the store hash, not the prefix)"
    - "The FOD hash-wiring protocol: prove byte-identity across two INDEPENDENT derivations (version-override clones force fresh kernel builds; a cache-hit 'rebuild' proves nothing), THEN wire outputHash — exactly the plan's prohibition, earned the hard way"

key-files:
  created:
    - build/x86_64/README.md
    - .planning/phases/13-r9-x86-64-platform-bring-up/13-EVIDENCE.md
    - .planning/phases/13-r9-x86-64-platform-bring-up/evidence/boot-test-q35.log
    - .planning/phases/13-r9-x86-64-platform-bring-up/evidence/boot-test-q35lowmcfg.log
    - .planning/phases/13-r9-x86-64-platform-bring-up/evidence/boot-test-q35net.log
    - .planning/phases/13-r9-x86-64-platform-bring-up/evidence/boot-test-q35net.pcap
  modified:
    - ../r9/flake.nix
    - ../r9/.github/workflows/xtask.yml
    - .planning/phases/13-r9-x86-64-platform-bring-up/deferred-items.md

key-decisions:
  - "The reproducibility mechanics, measured: grub-mkrescue 2.14 has NO SOURCE_DATE_EPOCH support and injects a current-time --modification-date into its xorriso argv; xorriso-native args appended after `--` execute AFTER the image is written (silently ineffective — only -volid appeared to work). The --xorriso wrapper is the pinning vector: rewrite the injected date + inject --set_all_file_dates and -volid into the mkisofs option region"
  - "The staged kernel is strip-ALL, not the dev copy's strip-debug: two fresh sandbox builds from identical source differ ONLY in ~200 per-invocation-random `.llvm.<u64>` anonymous-symbol suffixes (symtab-level nondeterminism); GRUB mb2 loads only PT_LOADs, so the symtab is dead weight the distributable sheds anyway"
  - "A fixed-output derivation must not reference store paths, and nix's scanner keys on the store HASH: build-std's panic-location strings ride the kernel .rodata with the toolchain path, and a prefix remap (/nix/store→/nix-store) still matched the reference — the length-preserving zero-zap sed (hash→32 zeros) breaks both the full-path and hash matches while keeping the strings readable; grub's modinfo.sh shebang gets the same treatment at graft time"
  - "OQ-3 boundary recorded in the flake: the flake owns the DISTRIBUTABLE reproducible ISO; the xtask gate keeps building its own dev copy per invocation so gate evidence stays fresh against the tree it just built"
  - "kernel-x64 on every linux host, boot-iso on x86_64-linux only (grub2's i386-pc BIOS modules + the channel itself are x86-native)"
  - "The CI leg selects TCG EXPLICITLY (--accel tcg --cpu qemu64) — hosted runners are KVM-less (A7), and explicit selection honors the no-silent-downgrade rule; the KVM-native default remains the dev-host proof"

patterns-established:
  - "The double-build proof must span independent derivations: a `--rebuild` that cache-hits its kernel input proves only the ISO layer; version-override clones (overrideAttrs nulling the FOD attrs) force genuinely fresh builds for comparison"
  - "The evidence-artifact freshness hazard: later gate runs sharing a config name overwrite target/ logs — copy artifacts out IMMEDIATELY after the run that must be retained"

requirements-completed: [X64-05]

# Coverage metadata (#1602) — one entry per shipped deliverable.
coverage:
  - id: D1
    description: "r9.packages.<system>.kernel-x64 — the kernel-virt derivation shape re-keyed (cargo xtask build --arch x86-64 --config q35, ELF at kernel-x64.elf); the Phase-14 compose contract"
    requirement: X64-05
    verification:
      - kind: other
        ref: "nix build .#kernel-x64 --no-link --print-out-paths green; output carries kernel-x64.elf (ELF 64-bit x86-64); nix eval .#packages.x86_64-linux.kernel-x64.pname → \"r9-kernel-x64\""
        status: pass
    human_judgment: false
  - id: D2
    description: "r9.packages.x86_64-linux.boot-iso — the byte-reproducible BIOS-only grub-mkrescue fixed-output derivation, hash wired only after the double-build byte-identity check"
    requirement: X64-05
    verification:
      - kind: other
        ref: "two independent derivations (fresh kernel builds) both produced ISO sha256 87a96c23a421ce886411ef103723b36c025fbb46f7e94ced726685369ca82003 (cmp byte-identical); post-wiring: nix build .#boot-iso green + nix build .#boot-iso --rebuild green (hash re-verified against fresh bytes) + a post-commit fresh build green; boot smoke at the derivation output: banner + x2apic live + TscDeadline armed"
        status: pass
    human_judgment: false
  - id: D3
    description: "The KVM-native default acceptance loop across all three q35 profiles (the X64-05 acceptance-gates-by-default clause, proven on the whole set)"
    requirement: X64-05
    verification:
      - kind: e2e
        ref: "for c in q35 q35lowmcfg q35net: cargo xtask boot-test --arch x86-64 --config $c — PASS x3 in one loop, no overrides, at the pinned HEAD (evidence/boot-test-*.log retained; q35net pcap 4352/4352 ARP)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The documented, runnable TCG fallback legs + build/x86_64/README.md (boot chain with the two X64-01 recorded decisions, accel contract with the no-silent-downgrade rule, timer chain, qemu-11.1.2 drift notes)"
    requirement: X64-05
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch x86-64 --config q35 --accel tcg --cpu qemu64 AND --config q35net --accel tcg --cpu qemu64 — PASS (run twice: plan verify + tracer-gate re-run); grep -ci tcg build/x86_64/README.md = 15 (≥2)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The boot-test-x86-64 CI leg (r9 .github/workflows/xtask.yml): self-provisioned (grub-pc-bin/xorriso/qemu-system-x86), explicitly TCG, locally validated per the no-push CI discipline — the hosted run itself is PENDING FIRST PUSH (not claimed as green)"
    requirement: X64-05
    verification:
      - kind: other
        ref: "grep boot-test-x86-64 xtask.yml = 1 with the explicit tcg/qemu64 overrides + apt provisioning; python3 yaml.safe_load → YAML-OK; the exact encoded command sequence run locally: q35/q35lowmcfg/q35net TCG --verbose all PASS; matrix + PENDING FIRST PUSH recorded in 13-EVIDENCE.md"
        status: pass
    human_judgment: false
  - id: D6
    description: "13-EVIDENCE.md + evidence/ — the consolidated phase evidence: every requirement (X64-01..05) mapped to gate runs, the per-plan commit trail, the CI matrix, the recorded decisions (q35-vs-pc + mb2-against-mb1, the PCIEXBAR carve-out, the driver_ops/netgate seam, the flake-owns-ISO boundary), the r9 SHA pinned"
    verification:
      - kind: other
        ref: "test -s 13-EVIDENCE.md && grep -c 'X64-0' = 13 (≥5); grep -c cc29c7a = 4 (≥1); evidence/ carries the three serial logs + the q35net pcap from the final KVM loop; the decisions section names PCIEXBAR carve-out / driver_ops / flake-owns-ISO"
        status: pass
    human_judgment: false
  - id: D7
    description: "No regression on the existing flake outputs (nix flake check) and the deferred qemu -global MCFG drift item closed into the README"
    verification:
      - kind: other
        ref: "nix flake check . → all checks passed (exit 0; pre-existing warnings only); deferred-items.md entry status: resolved with the README pointer"
        status: pass
    human_judgment: false

# Metrics
duration: 59 min
completed: 2026-11-07
status: complete
---

# Phase 13 Plan 06: Ship + Seal — kernel-x64, the Reproducible boot-iso, the KVM-default Gate Loop, the CI Leg, the Evidence Summary

**The phase is sealed: the flake ships kernel-x64 (the Phase-14 compose contract, built by the same gated xtask command) and a byte-reproducible boot-iso fixed-output derivation (double-build-proven at ISO 87a96c23…, after root-causing grub-mkrescue's current-time xorriso date, rustc's random anon-symbol suffixes, and nix's hash-keyed FOD reference scanner), all three q35 gates green KVM-native in one loop with the TCG fallback run and CI-able, and every requirement consolidated SHA-pinned Harvey-side — X64-05 closed, Phase 13 complete.**

## Performance

- **Duration:** 59 min
- **Started:** 2026-11-07T16:09:25Z
- **Completed:** 2026-11-07T17:08:51Z
- **Tasks:** 3 (Task 1 auto, Task 2 tracer, Task 3 auto)
- **Files modified:** 7 harvey-side (1 README + 5 evidence + deferred-items) + 2 r9-side (flake.nix, xtask.yml)

## Accomplishments
- **kernel-x64** exists as `r9.packages.<system>.kernel-x64` (linux hosts) — the kernel-virt derivation shape verbatim (same two-lockfile cargoDeps symlinkJoin, same toolchain/cargoSetupHook inputs), built by `cargo xtask build --arch x86-64 --config q35`, ELF at `kernel-x64.elf`. The Phase-14 compose contract is in place.
- **boot-iso** is a BIOS-only grub-mkrescue fixed-output derivation whose outputHash was wired ONLY after two independent derivations (fresh kernel builds under both) produced the byte-identical ISO `87a96c23a421ce886411ef103723b36c025fbb46f7e94ced726685369ca82003`; `nix build .#boot-iso --rebuild` and a post-commit fresh build both re-verify the hash, and the derivation output boots the full chain (banner → x2apic → TSC-deadline timer).
- **The acceptance gates run KVM-native by default across the whole set**: the three-profile loop green in one shell at the pinned HEAD, logs + pcap retained under `evidence/`; the TCG fallback (`--accel tcg --cpu qemu64`) is documented in the companion README and proven runnable for q35 + q35net (and q35lowmcfg via the CI-sequence validation).
- **The CI leg** (`boot-test-x86-64`) is authored, parse-checked, and locally validated per the no-push discipline (all three encoded commands green under the same TCG overrides); the hosted observation is honestly recorded as PENDING FIRST PUSH in the evidence file.
- **13-EVIDENCE.md** consolidates the phase: X64-01..05 each mapped to its gate runs, the per-plan commit trail, the CI matrix, and the phase's recorded decisions (q35-vs-pc; mb2 recorded against 9k's mb1; the CF8/CFC PCIEXBAR carve-out; the driver_ops widen + netgate seam; the flake-owns-ISO boundary) — pinned at r9 `cc29c7a`.

## Task Commits

Each task was committed atomically (production in the r9 repo, docs in harvey):

1. **Task 1: flake kernel-x64 package + the reproducible boot-iso fixed-output derivation** - `edd3d97` (feat, r9)
2. **Task 2: the boot-test-x86-64 CI leg (TCG, self-provisioned)** - `cc29c7a` (feat, r9) + `6ab8f91b5` (docs, harvey — build/x86_64/README.md)
3. **Task 3: 13-EVIDENCE.md + evidence/ + the deferred-item close** - `283db3390` (docs, harvey)

**Plan metadata:** (this docs commit — see below)

## Tracer Feedback Gate (Task 2)

Task 2 is the plan's tracer. Precedence chain: no `gate` attribute (row 1 N/A) → auto mode inactive (AUTO_CHAIN/AUTO_CFG false, row 2 N/A) → interactive + `human_verify_mode=end-of-phase` + the verify carries only `<automated>` (row 3): the verify was RE-RUN end-to-end after the task's commits — three-profile KVM loop PASS ×3, TCG legs PASS ×2, YAML parse YAML-OK — logged `⚡ Tracer verified end-to-end — expanding`, and execution continued to Task 3 without a checkpoint. The re-run's KVM loop produced the final evidence artifacts (the TCG re-run legs then overwrote the shared target/ log names — see Issues Encountered).

## Files Created/Modified
- `../r9/flake.nix` - kernel-x64 (the kernel-virt shape re-keyed, linux-gated exposure) + boot-iso (the --xorriso wrapper, strip-all + zero-zap staging, the scrubbed grub module dir, the FOD hash wired post-proof; the OQ-3 boundary comment)
- `../r9/.github/workflows/xtask.yml` - the boot-test-x86-64 job: apt grub-pc-bin/xorriso/qemu-system-x86 + the three q35 profiles with explicit --accel tcg --cpu qemu64
- `build/x86_64/README.md` - (new) the x86_64 companion: pins; the boot chain contract with BOTH X64-01 recorded decisions (q35-vs-pc; mb2-against-9k's-mb1); the accel contract (KVM default keys, TCG invocation, the no-silent-downgrade rule, the CI-venue rationale); the timer chain (CPUID 0x15→0x16→PIT, TSC-deadline 0x6E0 vs periodic, the 0x832 x2APIC-range finding); the qemu-11.1.2 drift table (the -global MCFG crash + -acpitable accommodation + re-test condition, the SeaBIOS PCIEXBAR hardcode, TCG's TSC-deadline absence, qemu64's FSGSBASE absence)
- `.planning/phases/13-r9-x86-64-platform-bring-up/13-EVIDENCE.md` - (new) the consolidated, SHA-pinned phase evidence index
- `.planning/phases/13-r9-x86-64-platform-bring-up/evidence/` - (new) the final KVM-loop serial logs (q35 1559 B, q35lowmcfg 1559 B, q35net 2277 B) + the q35net pcap (678936 B, 4352+4352 ARP frames)
- `.planning/phases/13-r9-x86-64-platform-bring-up/deferred-items.md` - the qemu -global MCFG drift entry marked resolved (recorded in the README per its own action item)

## Decisions Made
- The six key-decisions above — the load-bearing trio being the --xorriso wrapper (the only working pinning vector on grub 2.14), strip-all + zero-zap (the only way to satisfy both byte-identity and the FOD no-references rule), and the independent-derivation double-build protocol (a cache-hit --rebuild proves only the ISO layer)
- The remap-path-prefix experiment (config_q35.toml rustflags) was tried and REVERTED: remapping /nix/store→/nix-store still matched nix's hash-keyed reference scanner, so the scrub lives entirely in the ISO staging where it belongs — the kernel package keeps its truthful debug paths
- The final KVM evidence loop was run AFTER the commits (on the pinned tree cc29c7a), and artifacts copied out immediately — the tracer-gate TCG re-runs had overwritten the shared log names

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] grub-mkrescue 2.14 is not SOURCE_DATE_EPOCH-aware and injects a current-time --modification-date into its xorriso argv**
- **Found during:** Task 1 (the plan's own double-build check red on the first attempt)
- **Issue:** The plan's Pitfall-4 premise ("SOURCE_DATE_EPOCH honored by modern grub-mkrescue/xorriso") is false for this grub build: the binary has no SOURCE_DATE_EPOCH support, and it computes `--modification-date=<now>`. Additionally, xorriso args appended after `--` on the grub-mkrescue command line execute AFTER the mkisofs emulation writes the image — silently ineffective (only `-volid` happened to take visible effect; `-volume_date` was swallowed)
- **Fix:** A `--xorriso` wrapper (written into the derivation's buildPhase): rewrites the injected `--modification-date` to the pinned constant and injects `--set_all_file_dates <pinned>` + `-volid R9BOOT` into the mkisofs option region, where they actually apply (also pins the file mtimes riding Rock Ridge TF records — the second nondeterminism found at the same time)
- **Files modified:** ../r9/flake.nix
- **Verification:** three dev-side builds from two stage dirs byte-identical; then the full derivation-level double-build green
- **Committed in:** edd3d97

**2. [Rule 1 - Bug] rustc emits per-invocation-random `.llvm.<u64>` suffixes on ~200 anonymous-namespace symbols — kernel byte-identity is impossible at symtab level**
- **Found during:** Task 1 (the first FOD rebuild: hash mismatch with the strip-debug staging; two fresh sandbox builds from IDENTICAL source produced kernels differing only in those suffixes)
- **Issue:** The plan's staging assumed strip-debug parity with the xtask dev copy; the dev double-build had silently proven only the ISO layer (the kernel was a nix cache hit)
- **Fix:** The ISO stages a strip-ALL kernel (GRUB mb2 reads only PT_LOADs — all intact; the mb2 header rides .text.boot). Two independent fresh builds then produced identical staged kernels
- **Files modified:** ../r9/flake.nix
- **Verification:** identical sha256 across independent derivations; boot smoke at the derivation output
- **Committed in:** edd3d97

**3. [Rule 1 - Bug] FOD outputs must not reference store paths, and nix's scanner keys on the store HASH — a prefix remap is insufficient**
- **Found during:** Task 1 (the FOD registration refused: "references 1 distinct paths: <toolchain>" while the ISO's bytes contained zero `/nix/store/` strings)
- **Issue:** build-std's core/alloc panic-location strings carry the toolchain store path in the kernel .rodata; remapping `/nix/store`→`/nix-store` (tried via the config TOML rustflags, since env RUSTFLAGS would clobber xtask's platform cfgs) still matched the scanner's hash key. grub's modinfo.sh also grafts a store-path bash shebang onto the ISO
- **Fix:** A length-preserving sed zero-zap of the 32-char store-hash runs in the STAGED kernel (`/nix/store/<hash>-` → `<32 zeros>-`; exactly 864 bytes touched = 27 strings, NUL-count and size preserved, ELF valid — measured), plus a writable copy of the grub module dir with modinfo.sh's shebang scrubbed. The TOML remap was reverted
- **Files modified:** ../r9/flake.nix (config_q35.toml experiment reverted to byte-identical)
- **Verification:** zero live store hashes in the ISO; the FOD registers and the hash verifies
- **Committed in:** edd3d97

**4. [Rule 3 - Blocking] kernel-x64's installPhase lacked `mkdir -p "$out"`; the grub module-dir copy was read-only for sed**
- **Found during:** Task 1 (two build failures)
- **Issue:** kernel-virt's subdir mkdir had implicitly created $out; store-copied files are mode 0444 (sed could not write its temp file)
- **Fix:** `mkdir -p "$out"` + `chmod -R u+w "$moddir"`
- **Files modified:** ../r9/flake.nix
- **Verification:** builds green
- **Committed in:** edd3d97

---

**Total deviations:** 4 auto-fixed (3 Rule 1 bugs — all found by the plan's own double-build/FOD gates, exactly the teeth they were designed with — and 1 Rule 3 blocker pair)
**Impact on plan:** All fixes were required to satisfy the plan's own prohibitions (byte-identity before hash-wiring; no store-path references). No scope creep: the remap experiment was reverted, the dev/gate channel is byte-untouched, and zero new dependencies landed.

## Issues Encountered
- The evidence-artifact freshness hazard, found live: the tracer-gate's TCG re-run legs OVERWROTE `target/boot-test-q35.log` / `boot-test-q35net.log` written minutes earlier by the KVM loop (per-config log names are shared across accel legs) — the final KVM loop was re-run and the artifacts copied out immediately; recorded as a pattern so future consolidation copies at once
- `nix build --rebuild` semantics: for already-valid input-addressed outputs it "checks" without necessarily re-executing — the trustworthy double-build uses version-override clones forcing independent derivations (recorded in the flake comment's evidence note)
- One YAML parse iteration: a `:` in the CI step name ("explicit TCG: hosted runners…") broke `yaml.safe_load`; reworded to an em dash pre-commit (the parse gate caught it — its job)
- nix 2.34 CLI surface friction (`nix store delete` refusing live paths, `--expr` needing `--impure` for the dirty-tree flake, `path-info --references` spelling) — worked around with the overrideAttrs clone pattern; no impact on the deliverables

## Verification Log

On the final committed trees (r9 `cc29c7a`, harvey pre-docs `283db3390`):
- `nix build .#kernel-x64 --no-link --print-out-paths` — green; the output carries `kernel-x64.elf` (x86-64 ELF, debug build per the kernel-virt shape)
- `nix build .#boot-iso --no-link --print-out-paths && nix build .#boot-iso --rebuild --no-link --print-out-paths` — both green (the FOD hash re-verified against freshly produced bytes; also green on a post-commit fresh build at version edd3d97, proving the hash survives genuinely-independent rebuilds)
- Double-build evidence (pre-wiring): ISO sha256 `87a96c23a421ce886411ef103723b36c025fbb46f7e94ced726685369ca82003` from two independent derivations, `cmp` byte-identical; zero live store-hash references in the image (grep)
- Boot smoke at the FOD output: `r9 from the Internet` + `lapic: x2apic live` + `timer: 2496… Hz (pit … reference), TscDeadline mode, armed` under KVM
- Three-profile KVM-native loop (final runs, pinned HEAD): PASS ×3 — logs retained (`evidence/boot-test-{q35,q35lowmcfg,q35net}.log`; q35net pcap 4352 requests / 4352 replies asserted)
- TCG legs: q35 + q35net PASS (plan verify AND tracer-gate re-run); q35lowmcfg TCG PASS (CI-sequence validation); all with `--accel tcg --cpu qemu64`
- CI leg: `grep -c boot-test-x86-64 xtask.yml` = 1; the job carries the apt provisioning + explicit tcg/qemu64 overrides; `python3 yaml.safe_load` → YAML-OK; the exact encoded sequence (three `--verbose` TCG runs) green locally; hosted leg PENDING FIRST PUSH recorded in 13-EVIDENCE.md
- `nix flake check .` — all checks passed (exit 0; pre-existing warnings only: apps lack `meta`, foreign systems omitted — identical to the pre-change baseline captured before editing)
- 13-EVIDENCE.md: non-empty; `grep -c 'X64-0'` = 13 (≥5); r9 short SHA `cc29c7a` ×4; evidence/ complete (3 logs + pcap); the decisions section names the PCIEXBAR carve-out, the driver_ops/netgate seam, and the flake-owns-ISO boundary
- Zero new dependencies: r9 Cargo.tomls untouched across both commits

## Known Stubs
None — the plan's goal is achieved with no placeholder surface. The hosted CI observation is not a stub: it is recorded as PENDING FIRST PUSH per the no-push CI validation discipline (the phase's 21-commit r9 push is Phase 14's push-before-pin step), never claimed as an unobserved green.

## Threat Flags
None beyond the plan's threat model — T-13-13's mitigations are the deliverable itself (byte-reproducibility proven BEFORE the hash was wired; SOURCE_DATE_EPOCH + the pinned date/volid/file-dates scrubbing build-time state; the wired hash reds on any non-reproducible rebuild by construction — and did, twice, during development); T-13-14's freshness discipline held (the gates build-what-they-test each invocation; the evidence bundle pins the r9 SHA and copies THIS run's artifacts; the CI leg records local validation rather than claiming a hosted green); T-13-SC's supply chain is unchanged (locked nixpkgs inputs; zero crate additions).

## Authentication Gates
None - no authentication was required.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 14 (XEXEC) composes `r9.packages.<system>.kernel-x64` — the package exists, builds by the gated command, and installs the ELF; the flake-lock pin advance follows the push-before-pin discipline (r9 main is 21 commits ahead of origin, carrying the whole phase; the push also triggers the first hosted run of boot-test-x86-64)
- The netgate seam is recorded for the exec-era swap (driver_ops pub + the boot-context consumer)
- Phase 16's HNET-02 inherits the no-silent-downgrade doctrine as written in the companion README
- Phase 13 is COMPLETE: all five requirements (X64-01..05) evidenced and closed; the phase's canonical evidence lives at 13-EVIDENCE.md
- No blockers.

## Self-Check: PASSED

- All 7 harvey key-files exist on disk (`[ -f ]` each: FOUND, incl. the pcap); the 2 r9 files modified and committed
- r9 commits verified: edd3d97 (Task 1) and cc29c7a (Task 2) are ancestors of r9 `main` HEAD; harvey commits 6ab8f91b5 (Task 2 docs) and 283db3390 (Task 3) are ancestors of harvey HEAD
- Tracer gate: re-run green end-to-end (KVM loop ×3 + TCG ×2 + parse), expanded without checkpoint per the row-3 carve-out
- Production code lives in the sanctioned sibling repo /home/req/code/r9 per the dual-repo execution context; harvey-side commits rode the gsd-tools commit path

---
*Phase: 13-r9-x86-64-platform-bring-up*
*Completed: 2026-11-07*
