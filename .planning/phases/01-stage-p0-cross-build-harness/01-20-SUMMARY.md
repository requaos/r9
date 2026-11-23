---
phase: 01-stage-p0-cross-build-harness
plan: 20
subsystem: cross-build-harness
tags: [xtask, image-assembler, swap-probe, ps, aout, rust-lld, parse_aout, d-17, d-18, d-27, harn-04, harn-05]

requires:
  - phase: 01-stage-p0-cross-build-harness
    provides: "01-16's cpu manifest (4442 members, authdomain + ndb-template generators) and the provenance rule in image.rs; 01-18's generic --serial-check gate option; 01-19's whole nine-image x86_64 contract set; S6's proven probe recipe and recorded x86_64 probe sha256"
provides:
  - "PROBE-FLIP-SHA 064747f42c7f3eff6cd677a8820e6b4b1fff95e3: cpu.toml marks /amd64/bin/ps source = \"rust\" (from target/probe/x86_64/ps.aout, replaces amd64/bin/ps); the assembled member list differs from the control at exactly that path"
  - "PROBE-FLIPBACK-SHA af7df344b9c3702100d4408e85044a722e861a67: harvey ps restored, probe module/sources/subcommand deleted, member list byte-identical to the control"
  - "The two shas are the stop-B KVM witness inputs for plan 01-22 (step_probe / step_flipback)"
affects: [01-22, HARN-04, HARN-05, D-17, D-18, D-27]

actuals:
  tokens: 8009
  tasks: 3
  commits: 3

plan_head_before: ed91b673fac8f44720e5bf96ddcd857068b74002
plan_head_after: af7df344b9c3702100d4408e85044a722e861a67

tech-stack:
  added: []
  patterns:
    - "The S6 swap recipe runs in-tree: plain rustc (never a workspace member) -> a flat rust-lld payload at INITTEXT -> the 40-byte big-endian Plan 9 a.out header, validated by a Rust port of aout.py's parse_aout rules"
    - "The swap proof is three commits: add the builder, flip one manifest entry, flip back and delete the builder; the member list (path/mode/size/sha) is the comparison, never the .paq bytes (mkpaqfs embeds a per-bake time)"
    - "A same-host control/probe comparison reuses one cached DEMO auth domain so the six generated cpu members are identical for both bakes, isolating the swapped path; the committed assembler still generates a fresh domain per bake"

key-files:
  created:
    - xtask/src/probe.rs (temporary; deleted at PROBE-FLIPBACK-SHA)
    - xtask/probe/probe-amd64.rs (S6 source, byte-verbatim; deleted at PROBE-FLIPBACK-SHA)
    - xtask/probe/probe-amd64.ld (S6 linker script, byte-verbatim; deleted at PROBE-FLIPBACK-SHA)
  modified:
    - xtask/src/main.rs (the `probe` subcommand added, then reverted byte-for-byte)
    - xtask/manifests/amd64/cpu.toml (the /amd64/bin/ps entry flipped, then restored byte-for-byte)

key-decisions:
  - "The three commits are the plan's fixed proof shape: the TDD RED/GREEN cycle ran inside Task 1 (4 tests failed, then 4 passed) but the RED tests and GREEN implementation land together in 519fdef, because the plan names exactly three commits and stop B checks out commits 2 and 3 (#tdd commit-shape note)."
  - "The Mac's cpu bakes used a session-only, gitignored authdomain-gen shim (target/r9-lima-authdomain-gen) forwarding -o/-i/-V into lima-harvey; its -o result is cached so the control, probe and flip-back bakes share the six generated members. No committed code changes."
  - "The Task 2 verify command's `awk '{print $3}'` reads the member kind (`file`); the path is field 2. The corrected command prints exactly `amd64/bin/ps` and the diff is one changed pair."
  - "The Task 3 remnant gate's pattern set also matches the generic rust-source fixtures in image.rs and the generic --serial-check test in gate.rs (8 matches, all present at the base commit and explicitly kept by the same task). The corrected probe-only gate is 0."

requirements-completed: [HARN-04, HARN-05]

coverage:
  - id: D1
    description: "At PROBE-FLIP-SHA the cpu image carries the Rust a.out at /amd64/bin/ps; the assembled member list differs from the control at exactly that one path, and HARN-04's provenance rule passes (workspace artifact, bytes differ from Harvey's ps)"
    requirement: HARN-04
    verification:
      - kind: integration
        ref: "cargo xtask image --arch x86-64 --image cpu (control 4442 members) then again after the flip; diff | grep -E '^[<>]' | awk '{print $2}' | sort -u -> amd64/bin/ps (one changed pair)"
        status: pass
      - kind: other
        ref: "sha256 target/probe/x86_64/ps.aout = db94057f04c751c59a648de5c8505f452bb225b6a276714e7ee5544396a5dcd5 (S6's recorded x86_64 probe sha); Harvey ps = f029a723212b13c86fa66d25b5eb358c0ebf4f9523832cc936769b4f9a2651ea"
        status: pass
    human_judgment: false
  - id: D2
    description: "At PROBE-FLIPBACK-SHA the harvey ps is restored, the probe module/sources/subcommand are deleted, and the assembled member list is byte-identical to the control"
    requirement: HARN-04
    verification:
      - kind: integration
        ref: "cargo xtask image --arch x86-64 --image cpu && cmp $TMPDIR/r9-0120-control.members target/images/x86_64/cpu.members -> FLIPBACK-IDENTICAL (both sha256 815b79108916f555995ecc24789b7da7729944b8080ae68d91bc0f7392b1bb63)"
        status: pass
    human_judgment: false
  - id: D3
    description: "No throwaway code or rust manifest entry remains: the probe files and subcommand are gone, every manifest is all-Harvey, and the generic rust-source/--serial-check mechanism is kept"
    requirement: HARN-05
    verification:
      - kind: other
        ref: "git grep -c -e probe-amd64 -e ps.aout -- xtask/src/probe.rs xtask/probe -> 0; grep -c 'mod probe' xtask/src/main.rs -> 0; cargo xtask probe --arch x86-64 -> unknown subcommand; cat xtask/manifests/*/*.toml | grep -c 'source = \\\"rust\\\"' -> 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "The KVM witness runs the flip and flip-back gates on the owner's x86_64 host (stop B, plan 01-22)"
    requirement: HARN-04
    verification: []
    human_judgment: true
    rationale: "The gate needs /dev/kvm and the x86_64 ISO path (Linux-host-only); the Mac can only prove the member-list swap and the shas. The two runs are queued for stop B with PROBE-FLIP-SHA and PROBE-FLIPBACK-SHA and must be judged from that log."

duration: 15min
completed: 2026-11-23
status: complete
---

# Phase 01 Plan 20: The swap probe — ps to a Rust a.out, flip back, remove the probe Summary

**The cpu image's single `/amd64/bin/ps` entry was flipped to the S6 Rust a.out (member list differs at exactly that path; the probe reproduces S6's recorded sha256 `db94057f…`), then restored byte-identically (`FLIPBACK-IDENTICAL`, both member lists sha256 `815b7910…`) and every probe source, module and subcommand deleted.**

## Performance

- **Duration:** 15 min (2026-11-23T11:57:00Z → 2026-11-23T12:12:00Z)
- **Started:** 2026-11-23T11:57:00Z
- **Completed:** 2026-11-23T12:12:00Z
- **Tasks:** 3 of 3
- **Files modified:** 5 (3 temporary files created and deleted; 2 files flipped and reverted; net tree diff against the plan base is zero)
- **Plan estimate:** 70,000 tokens (low confidence); **actual:** 8,009 tokens (chars/4 over the sum of the three commits' patch bytes — the net diff is zero because the probe is added then deleted; a net-diff measure would report 0 and corrupt calibration).

## Accomplishments

- **The probe builder is real and reproducible.** `cargo xtask probe --arch x86-64` runs plain rustc (`x86_64-unknown-none`, `-C panic=abort -C opt-level=s -C relocation-model=static`, flat `--oformat=binary`), links at `INITTEXT` 0x200028, wraps the 40-byte big-endian header and checks every `parse_aout` rule in Rust. The 96-byte flat payload became a 136-byte a.out whose sha256 `db94057f04c751c59a648de5c8505f452bb225b6a276714e7ee5544396a5dcd5` **exactly matches S6's recorded x86_64 probe build** — the recipe is deterministic on this devshell.
- **The flip commit is the witness input.** In `064747f` (`PROBE-FLIP-SHA`), `/amd64/bin/ps` is the only `source = "rust"` entry in any manifest (1 of 1), with `from = "target/probe/x86_64/ps.aout"`, `replaces = "amd64/bin/ps"` and its 0755 mode kept. The assembly (4442 members) passes the provenance rule: the artifact resolves inside the workspace and its bytes (`db94057f…`) differ from Harvey's ps (`f029a723…`). The control-vs-flip member diff is **exactly one changed pair at `amd64/bin/ps`**.
- **The flip-back is byte-identical.** `cpu.toml` and `main.rs` were restored from `PROBE-FLIP-SHA~1`/the base commit (zero diff against the base), and `xtask/src/probe.rs` + `xtask/probe/` deleted. The re-assembled 4442-member list `cmp`s equal to the control (`FLIPBACK-IDENTICAL`; both sha256 `815b79108916f555995ecc24789b7da7729944b8080ae68d91bc0f7392b1bb63`).
- **D-18 holds.** No `probe-amd64`, `ps.aout` or `R9-RUST-AOUT-PROBE-OK` reference remains in the probe's own files, `mod probe` is gone, `cargo xtask probe` is an unknown subcommand, and every manifest is all-Harvey (`source = "rust"` count 0). The generic rust-source support in `image.rs` and the generic `--serial-check` option in `gate.rs` are kept, as the plan requires.
- **D-12 and the kernel baseline held at each commit.** `cargo xtask test` 87 passed / `clippy` 0 / `check` 0 before each of the three commits; the x86_64 q35churnshell ELF sha256 `50b01c9a…` and aarch64 virtshell `76e6e153…` equal the 01-08 baseline.
- **The two gate runs are queued for stop B.** Plan 01-22's `step_probe` checks out `PROBE-FLIP-SHA` (`064747f…`), runs `cargo xtask probe --arch x86-64`, then the gate with `--serial-check '/bin/ps=R9-RUST-AOUT-PROBE-OK'`; `step_flipback` checks out `PROBE-FLIPBACK-SHA` (`af7df34…`) and runs the plain gate.

## TDD Gate Compliance (Task 1)

| Phase | Evidence | Notes |
|---|---|---|
| RED | `nix develop -c cargo test -p xtask probe::` → **0 passed; 4 failed; 85 filtered out** (exit non-zero). Target tests `probe::tests::wrap_prepends_the_40_byte_plan9_header` (left 0, right 43), `check_accepts_the_wrapped_payload` (`Err("check: not implemented")`), `check_rejects_each_parse_aout_violation`, `build_probe_names_the_arch_for_non_x86_64` — all failed on the planned assertions, not a build error. | Semantic assessment: the target tests executed and failed for the intended reason (stub `wrap` returns empty, stub `check`/`build_probe` return not-implemented). |
| GREEN | Same command → **4 passed; 0 failed** in the bin target (the lib and emitter targets filter to 0 by design); `cargo xtask probe --arch x86-64` → 96 flat → 136 a.out with the marker. | |
| REFACTOR | — | none needed |

**Commit-shape note (tdd.md Fail-Fast rule 3).** The plan fixes exactly three commits (`feat` builder, `feat` flip, `revert` flip-back) and stop B checks out commits 2 and 3, so the RED tests and GREEN implementation land together in `519fdef` rather than as separate `test(...)`/`feat(...)` commits. The RED output is recorded above; no plan-level `type: tdd` gate applies (frontmatter is `type: execute`).

## Task Commits

1. **Task 1: Probe builder — rustc flat payload, 40-byte header, parse_aout-mirroring check** — `519fdef` (feat)
2. **Task 2: The flip commit — cpu manifest's /amd64/bin/ps becomes rust** — `064747f` (feat) — **PROBE-FLIP-SHA**
3. **Task 3: The flip-back commit — restore harvey ps and delete the probe** — `af7df34` (revert) — **PROBE-FLIPBACK-SHA**

**Plan metadata:** the `docs(01-20): complete …` commit that carries this SUMMARY.

## Files Created/Modified

- `xtask/src/probe.rs` (created at `519fdef`, deleted at `af7df34`) — `build_probe(Arch)`, `wrap`, `check`, four behaviour tests
- `xtask/probe/probe-amd64.rs` (created then deleted) — S6's `no_std` probe source, byte-verbatim
- `xtask/probe/probe-amd64.ld` (created then deleted) — S6's flat linker script at 0x200028, byte-verbatim
- `xtask/src/main.rs` (modified then restored byte-for-byte) — `mod probe;`, the `probe --arch` clap subcommand and its dispatch arm
- `xtask/manifests/amd64/cpu.toml` (flipped then restored byte-for-byte) — the `/amd64/bin/ps` entry only

## Decisions Made

- **Three commits, TDD inside Task 1.** The plan names exactly three shas as the stop-B inputs; adding RED/GREEN commits would change that proof shape. The tests were written first and observed failing (recorded above), then implemented, and land with the builder.
- **The Mac bake uses a session-only authdomain shim.** `R9_AUTHDOMAIN_GEN` is unset on the Darwin devshell by design (01-14/01-16), so `target/r9-lima-authdomain-gen` forwards `-o/-i/-V` into the running `lima-harvey` VM. Its `-o` output is cached and reused for the control, flip and flip-back bakes, so the six generated members are identical and the comparison isolates the ps swap. The committed assembler still runs a fresh domain per bake; stop B runs the real generator natively.
- **The member list, not the `.paq`, is the comparison.** The three bakes produced `cpu.paq` sha256 `0427c549…`, `7a3f1e93…` and `a5af6eda…` (mkpaqfs embeds a per-bake time; 01-29's note), while the members file was identical at the flip-back and differed only at `amd64/bin/ps` at the flip.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Task 2's member-diff command reads the wrong field**

- **Found during:** Task 2 (the flip commit)
- **Issue:** the plan's `diff … | grep -E '^[<>]' | awk '{print $3}'` assumes the tree-members format (`f <path> …`). The image member list is `<path> file <mode> <size> <sha256>`, so field 3 is the literal `file`; the command prints `file` no matter which path changed and can never satisfy the plan's `fails_when` ("anything other than /amd64/bin/ps").
- **Fix:** ran the command as written (output: `file`), then the corrected field 2, which prints exactly `amd64/bin/ps`; the diff is one changed pair (2 `^[<>]` lines). The plan's intent — a single changed path — is proven.
- **Files modified:** none (command only)
- **Verification:** `diff "$TMPDIR/r9-0120-control.members" target/images/x86_64/cpu.members | grep -E '^[<>]' | awk '{print $2}' | sort -u` → `amd64/bin/ps`; changed-line count 2.
- **Committed in:** `064747f` (the flip commit's evidence)

**2. [Rule 1 - Bug] Task 3's remnant gate matches the generic support the same task says to keep**

- **Found during:** Task 3 (the flip-back commit)
- **Issue:** the plan's `git grep -e 'probe-amd64' -e 'R9-RUST-AOUT-PROBE-OK' -e 'ps.aout' -- xtask` reports 8 matches — `xtask/src/gate.rs` (2) and `xtask/src/image.rs` (6) — all present at the plan base `ed91b67` and explicitly kept by Task 3's action ("Keep the generic rust-source support in image.rs and the generic `--serial-check` gate option"). The gate can never print 0 without deleting the mechanism the plan protects.
- **Fix:** kept `gate.rs`/`image.rs` untouched; ran the corrected gate over the probe's own paths (`xtask/src/probe.rs`, `xtask/probe`), which prints 0, plus `grep -c 'mod probe' main.rs` = 0, `grep -c '"probe"' main.rs` = 0, and `cargo xtask probe` rejected as an unknown subcommand. D-18's property (no throwaway code remains) holds.
- **Files modified:** none
- **Verification:** `git grep -c -e probe-amd64 -e ps.aout -- xtask/src/probe.rs xtask/probe` → 0; `git grep … ed91b67` shows the same 8 generic matches, proving they predate the plan.
- **Committed in:** `af7df34` (the flip-back commit's evidence)

**3. [Rule 3 - Blocking/environment] The Mac devshell cannot run the cpu bake's authdomain generator**

- **Found during:** Task 2 (the control assembly)
- **Issue:** `cargo xtask image --arch x86-64 --image cpu` with `env -u R9_HARVEY_DIR` fails: `R9_AUTHDOMAIN_GEN is not set — … the Mac devshell leaves it unset`. The generator is Linux-only (01-14/01-16), so the plan's Mac verify command cannot run as written.
- **Fix:** a session-only, gitignored shim at `target/r9-lima-authdomain-gen` forwards `-o/-i/-V` into the running `lima-harvey` VM (the worktree path is mounted unchanged). The shim caches the `-o` domain and reuses it for all three bakes so the generated members are identical; the committed assembler is unchanged and still generates fresh material per bake. This is 01-16's documented convention, adapted to the current VM.
- **Files modified:** `target/` only (gitignored; not committed)
- **Verification:** the control bake printed 4442 members; both later bakes reused the cache; the stop-B KVM witness runs the real generator natively.
- **Committed in:** n/a (environment-only)

### Verification-scope notes

1. **Task 1's functional verify was run at `519fdef`** (the probe is deleted at HEAD by design): `cargo test -p xtask probe::` → 4 passed in the bin target (the lib/emitter targets run 0), `cargo xtask probe --arch x86-64` → 96 flat → 136 a.out, `strings … | grep -c 'R9-RUST-AOUT-PROBE-OK'` → 1, `od -An -tx1 -N4` → `00008a97`, `grep -c probe Cargo.toml` → 0.
2. **The kernel ELF check is a build-once fact.** No plan commit touches kernel sources; the two ELFs built before Task 1 and re-hashed at each commit stayed at the 01-08 baselines (`50b01c9a…`, `76e6e153…`).

---

**Total deviations:** 2 auto-fixed (2 plan-command defects), 1 environment fix, plus two verification-scope notes. **Impact:** no production code is affected; the corrected commands prove the same intent, and the shim is the only way the Mac can run the bake at all. No scope creep.

## Issues Encountered

- **The flip and flip-back `.paq` shas differ from the control's** while the member lists prove the swap and the equality (`0427c549…` / `7a3f1e93…` / `a5af6eda…`). `mkpaqfs` embeds a time field; D-29's comparison is the member list, which behaved exactly as the plan requires.
- **The lima VM cannot see the Nix store path of `harvey-tree-amd64`** (that tree is realised on the Determinate external builder, not imported into the VM). The generator does not need the tree's cwd — with no `authdomain.local` it prints "generating ephemeral DEMO material" — so the shim runs it from the VM's home directory.

## Known Stubs

None. The probe is deleted at the flip-back commit; no placeholder file or manifest entry remains, and every manifest is all-Harvey.

## Threat Flags

None beyond the plan's threat model. T-01-65 (spoofing) is implemented by the provenance rule, whose pass is evidenced by the probe sha `db94057f…` differing from Harvey's ps `f029a723…`; T-01-66 (leftover code) by the deletion plus the corrected probe-only gate; T-01-SC by no package installs (the probe uses the devshell's rustc).

## Next Phase Readiness

- **Plan 01-22 (`step_probe` / `step_flipback`)** has both inputs: `PROBE-FLIP-SHA = 064747f42c7f3eff6cd677a8820e6b4b1fff95e3` and `PROBE-FLIPBACK-SHA = af7df344b9c3702100d4408e85044a722e861a67`. At the flip sha the `probe` subcommand and sources exist, so `cargo xtask probe --arch x86-64` rebuilds the a.out; at the flip-back sha the plain gate runs.
- **No default-image change.** HEAD's cpu manifest is byte-identical to the plan base's; the default image stays all-Harvey (D-18), and the generic rust-source/`--serial-check` mechanism remains for the gate work.
- **The stop-B KVM runs are the only unproven claim** (D4 above); the Mac has proven the member-list swap and equality.

---

*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-23*

## Self-Check: PASSED

- Final state: `xtask/src/probe.rs`, `xtask/probe/` absent; `cpu.toml` and `main.rs` byte-identical to the plan base; `cat xtask/manifests/*/*.toml | grep -c 'source = "rust"'` = 0.
- Task commits `519fdef`, `064747f` (PROBE-FLIP-SHA) and `af7df34` (PROBE-FLIPBACK-SHA) are ancestors of HEAD; `git rev-list --count ed91b67..HEAD` = 3 (measured against the plan ledger).
- Task 1 verify (at `519fdef`): probe tests 4 passed; probe build 96→136 bytes, marker count 1, magic `00008a97`, `Cargo.toml` probe count 0.
- Task 2 verify: assembly 4442 members; corrected member diff prints only `amd64/bin/ps`; probe sha `db94057f…` ≠ Harvey ps `f029a723…`; rust entries 1 (cpu only).
- Task 3 verify: `FLIPBACK-IDENTICAL`; member-list sha `815b7910…` on both sides; probe-only remnants 0; test 87 passed, clippy 0, check 0.
- Plan verification: kernel ELFs `50b01c9a…` and `76e6e153…` = the 01-08 baseline; the two gate runs are queued for stop B with the recorded shas.
- `STATE.md`/`ROADMAP.md`/`REQUIREMENTS.md` were not modified (worktree mode; the orchestrator owns them).
