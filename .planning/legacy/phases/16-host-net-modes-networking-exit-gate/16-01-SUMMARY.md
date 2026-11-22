---
phase: 16-host-net-modes-networking-exit-gate
plan: 01
subsystem: netdev-infra
tags: [netdev, qemu, toml, mode-tables, hostfwd, slirp, tap, vmnet, r9, flake-export, emitter, xtask, x86_64]
duration: 21 min
completed: 2026-11-11
requires:
  - "13-01-complete the [qemu.netdev] flat stanza + apply_netdev emitter (the surface D-05 re-shapes)"
  - "14-07-complete the r9 flake packages/kernel-x64 + iso-builder export shape (the skeleton netdev-args mirrors)"
provides:
  - "r9 xtask lib target (xtask/src/lib.rs): the config/uregcheck modules + the ONE netdev argv builder netdev_args/netdev_args_checked, consumed by BOTH front-ends — the in-process apply_netdev and the exported r9-netdev-args bin"
  - "the D-05 Netdev TOML shape in all five config_q35*.toml: [qemu.netdev] default=\"slirp\" + slirp/tap/vmnet sub-tables, the D-04 hostfwd set (tcp:127.0.0.1:17010-:17010, tcp:127.0.0.1:5670-:567) and the NixOS tap helper path as data"
  - "r9 packages.<system>.netdev-args (host-std bin r9-netdev-args: --mode slirp|tap|vmnet [--pcap <path>] [--config <path>], default config = committed config_q35shell.toml embedded at build time) — PUSHED at r9 d1c4849dfa1d769240630c29b9b03b88d5819558, the 16-04 pin target"
affects:
  - "16-04 (pins exactly the pushed r9 SHA and substitutes the emitter's output in boot-r9-x64.sh)"
  - "16-02/16-03/16-05 (the tap table's br0/helper data + the vmnet table the doctor and docs cite)"
  - "every r9 x86_64 gate boot (the slirp default argv now carries the two hostfwd rules — parallel boots of two q35 gates on one host collide on 127.0.0.1:17010/5670; the TOMLs note it)"
tech-stack:
  added: []
  patterns:
    - "one emission, two front-ends (D-05): a pure checked core (netdev_args_checked -> Result) + a fatal wrapper (netdev_args, the config.rs eprintln/exit idiom); front-ends never assemble argv tokens"
    - "single-source-as-data at the emitter layer: the D-04 hostfwd remap, the NixOS helper path, and the vmnet socket path are TOML data consumed by ONE builder — committed data machine-checked by a lib test parsing all five TOMLs (include_str!)"
    - "pcap gating is two-sided: the table's pcap flag enables, the caller's path places — neither alone emits a filter-dump object"
key-files:
  created:
    - ../r9/xtask/src/lib.rs
    - ../r9/xtask/src/bin/r9-netdev-args.rs
  modified:
    - ../r9/xtask/src/config.rs
    - ../r9/xtask/src/main.rs
    - ../r9/xtask/src/pcap.rs
    - ../r9/xtask/src/uregcheck.rs
    - ../r9/xtask/Cargo.toml
    - ../r9/x86_64/lib/config_q35.toml
    - ../r9/x86_64/lib/config_q35exec.toml
    - ../r9/x86_64/lib/config_q35lowmcfg.toml
    - ../r9/x86_64/lib/config_q35net.toml
    - ../r9/x86_64/lib/config_q35shell.toml
    - ../r9/flake.nix
key-decisions:
  - "ONE builder with a testable core: netdev_args(config, mode, pcap_path) -> Vec<String> is the plan-named fatal front-end; netdev_args_checked is the pure core whose Err carries the slirp|tap|vmnet usage vocabulary — the unit tests and the exported bin consume the checked core so the bin can print the EXACT pinned usage line, and no front-end ever assembles a token"
  - "The five committed q35 TOMLs are machine-checked at test time: a lib test embeds all five via include_str! and asserts default=slirp + the three sub-tables each — the must-haves truth is enforced at the data layer, never left to grep"
  - "xtask gained default-run = \"xtask\" (Rule 3, found live by the Task-3 gate battery): the second bin made cargo's xtask custom-subcommand protocol ambiguous — cargo xtask check/clippy AND kernel-x64's flake buildPhase ride it"
  - "The emitter's default config is the committed config_q35shell.toml embedded at build time (include_str!) — ONE committed TOML, no copied data; nix's git-source filtering requires new files to be staged before a flake build sees them (found live: the first nix build .#netdev-args failed with the bin absent from the store source)"
patterns-established:
  - "Mode selection at the emitter layer: selector keys (slirp/tap/vmnet) are distinct from backend strings (user/tap/vmnet-socket) — the sub-table KEY selects, the mode FIELD is what qemu sees"
requirements-completed: [HNET-02, HNET-04]
actuals:
  tokens: 14668   # chars/4 over git diff 5f856c9..d1c4849 (the realized r9 production diff; estimate was 55000)
  tasks: 3
  commits: 5      # r9 production commits cb96d56..d1c4849 (this plan's work lives in the sibling r9 repo per the plan's own dual-repo instruction; the Harvey repo carries only this docs commit)
r9_pushed_sha: d1c4849dfa1d769240630c29b9b03b88d5819558
r9_base_sha: 5f856c9
plan_head_before: fa10f57f3b11dbe0362764863fb5cdce17885e1c
plan_head_after: 44dd41258ab8d9cd9756c42aac354912d8308a97
status: complete
coverage:
  - id: D1
    description: "The D-05 netdev mode tables + the ONE shared arg-builder: all five config_q35*.toml carry default=\"slirp\" + the three sub-tables with the D-04 hostfwd set verbatim; netdev_args/apply_netdev emit the default path byte-identically"
    requirement: HNET-02
    verification:
      - kind: unit
        ref: "r9 xtask config::tests (cargo test -p xtask netdev — 11 tests: stanza argv byte-exact, pcap gating, byte-identity, missing-key/default-key rejection, all-five-TOMLs parse)"
        status: pass
      - kind: other
        ref: "cd ../r9 && cargo test -p xtask (35 lib + 73 bin + 3 doc tests, 0 failed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The tap and vmnet emission arms: tap,id=n0,br=br0,helper=/run/wrappers/bin/qemu-bridge-helper and vmnet-socket,id=n0,path=/var/run/socket_vmnet as data in all five TOMLs, unknown mode a loud slirp|tap|vmnet error"
    requirement: HNET-04
    verification:
      - kind: unit
        ref: "r9 xtask config::tests::netdev_args_{tap_emits_br_and_helper_verbatim, vmnet_emits_socket_path, unknown_mode_is_a_loud_error, tap_table_requires_br_and_helper}"
        status: pass
    human_judgment: false
  - id: D3
    description: "The exported emitter: r9 packages.<system>.netdev-args builds a host-std r9-netdev-args bin whose stdout is the builder's argv; pushed at r9 d1c4849 (the 16-04 pin target)"
    requirement: HNET-02
    verification:
      - kind: other
        ref: "cd ../r9 && nix build .#netdev-args && ./result/bin/r9-netdev-args --mode tap --pcap /tmp/probe.pcap (filter-dump present) && --mode slirp (absent); --mode bogus -> exit 1 + the exact usage line; nix eval .#packages.x86_64-linux.netdev-args.outPath prints a store path; nix build .#kernel-x64 green"
        status: pass
      - kind: other
        ref: "cd ../r9 && cargo xtask check && cargo xtask clippy at the pushed HEAD (CLEAN-FOR-PUSH)"
        status: pass
    human_judgment: false
---

# Phase 16 Plan 01: r9 netdev mode tables + the exported emitter Summary

**The r9 half of D-05 landed and is pushed: [qemu.netdev] is now default + slirp/tap/vmnet sub-tables in all five q35 TOMLs (D-04 hostfwd remap as data), one shared argv builder in a new xtask lib serves the in-process boots and the exported r9-netdev-args flake bin — pushed at r9 `d1c4849dfa1d769240630c29b9b03b88d5819558` (the 16-04 pin target).**

## Performance

- **Duration:** 21 min
- **Started:** 2026-11-11T11:55:55Z
- **Completed:** 2026-11-11T12:17:02Z
- **Tasks:** 3/3
- **Files modified:** 13 (all in the sibling r9 repo)

## The pushed r9 SHA (the 16-04 pin target)

`d1c4849dfa1d769240630c29b9b03b88d5819558` on `origin/main` (base was 5f856c9; push-before-pin held — the pin advance is 16-04's own commit, nothing bundled).

## Accomplishments

- The breaking D-05 struct migration landed atomically: `Netdev` is a default selector + `Option` sub-tables (`SlirpNetdev`/`TapNetdev`/`VmnetNetdev`), all five q35 TOMLs migrated, `apply_netdev` reduced to a thin front-end over the ONE builder — both existing qemu call sites keep their call shapes and per-step pcap paths
- The D-04 hostfwd remap is data: `tcp:127.0.0.1:17010-:17010` and `tcp:127.0.0.1:5670-:567` (loopback-bound host addresses only) in the slirp table of every q35 config; the tap table carries `br=br0` + `helper=/run/wrappers/bin/qemu-bridge-helper` verbatim (the live-probed NixOS divergence); the vmnet table carries `socket-path=/var/run/socket_vmnet` (darwin-only, documented-manual)
- ONE emission implementation, two front-ends: `netdev_args` (fatal, the config.rs eprintln/exit idiom) + `netdev_args_checked` (pure, testable) in the new `xtask/src/lib.rs` target; the in-process `apply_netdev` and the exported `r9-netdev-args` bin both route through it — nothing re-assembles netdev argv (the phase prohibition)
- r9's flake exports `packages.<system>.netdev-args` (host-std `stdenv.mkDerivation` mirroring kernel-x64's cargoSetupHook+cargoDeps skeleton, plain `cargo build --release --bin r9-netdev-args`), beside kernel-x64 in the existing isLinux optionalAttrs; the bin's default config is the committed q35shell TOML embedded at build time
- Full gate battery green at the pushed HEAD: `cargo xtask check`, `cargo xtask clippy`, 111 xtask tests, `nix build .#netdev-args .#kernel-x64`, and the three emitter modes + pcap gating + bogus-mode usage error proven live on both the dev build and the nix-built bin

## Task Commits (all in the sibling r9 repo)

1. **Task 1 RED: re-key the netdev stanza tests to the D-05 mode-tables shape** — `cb96d56` (test)
2. **Task 1 GREEN: netdev mode tables + the ONE shared arg-builder (slirp default path)** — `e5ef95f` (feat)
3. **Task 2 RED: tap/vmnet emission + loud-error tests** — `34b1b9e` (test)
4. **Task 2 GREEN: tap + vmnet emission arms and their tables in all five q35 TOMLs** — `d7180c4` (feat)
5. **Task 3: the r9-netdev-args emitter bin + the flake export (push-before-pin)** — `d1c4849` (feat)

**Plan metadata:** this SUMMARY + STATE/ROADMAP updates (the only Harvey-repo commit of this plan — its production work lives in r9 by the plan's own instruction).

## TDD record (tasks 1 and 2 carry tdd="true")

- **Task 1 RED** (`cb96d56`): the re-keyed `q35_netdev_stanza_emits_from_data` failed at the parse assertion (the sub-table TOML hit the flat struct's `missing field mode`) and the sharpened `q35_netdev_table_requires_all_three_keys` failed on the key-naming assertion (old error named `mode`, not the sub-table key). Both failures were the planned behavior gap, run and captured before any implementation.
- **Task 1 GREEN** (`e5ef95f`): structs + builder + wrapper + five TOMLs + new guards (pcap gating, byte-identity, missing-default-key) — the plan's "breaking struct change migrated together" was honored by putting structs+TOMLs+apply_netdev+tests in the GREEN commit while the RED commit carried only the re-keyed tests (r9's own `test(...)` RED precedent: 13-02/13-03/13-04/14-03/14-04).
- **Task 2 RED** (`34b1b9e`): the three new emission tests failed with `unknown netdev mode tap/vmnet` (arms absent). One guard test (`netdev_tap_table_requires_br_and_helper`) was green on arrival by design — it pins the Task-1 struct shape (serde naming the missing `helper` key), structurally expected, not an unexpected green.
- **Task 2 GREEN** (`d7180c4`): the tap/vmnet arms + the tables in all five TOMLs.
- Task 3 (not TDD): the embedded-config parse tests rode with the implementation; the CLI's behavior contract was verified live (three modes, pcap gating, bogus-mode usage line).

## Files Created/Modified (r9 repo, `../r9`)

- `xtask/src/lib.rs` — NEW: the lib target (config/uregcheck modules, Arch/Profile/DynError/Result/env_or root types, netdev re-exports, `DEFAULT_NETDEV_CONFIG` embedded q35shell TOML + the all-five parse tests)
- `xtask/src/bin/r9-netdev-args.rs` — NEW: the exported CLI front-end (usage line is a pinned contract)
- `xtask/src/config.rs` — Netdev re-shape + sub-table structs, `netdev_args`/`netdev_args_checked`, thin `apply_netdev`, 11 netdev tests
- `xtask/src/main.rs`, `xtask/src/pcap.rs`, `xtask/src/uregcheck.rs` — the lib-split rewiring (bin consumes `xtask::`; `harvey_dir` went `pub`)
- `xtask/Cargo.toml` — `default-run = "xtask"` (the second-bin ambiguity fix)
- `x86_64/lib/config_q35*.toml` ×5 — the mode tables; provenance headers extended with 16/D-04/D-05 (q35.toml gained the header it never had); the parallel-boot hostfwd-port constraint noted in each
- `flake.nix` — `packages.<system>.netdev-args` derivation + export beside kernel-x64

## Decisions Made

See key-decisions in the frontmatter; the load-bearing ones:

1. **The checked-core split** — the plan names `netdev_args -> Vec<String>` with eprintln+exit for unknown modes, but a unit test cannot survive `exit(1)`; the pure `netdev_args_checked` core carries the Err (with the `slirp|tap|vmnet` vocabulary) so the behavior is testable AND the bin can print its own EXACT usage line. One implementation, two skins over it.
2. **All five TOMLs machine-checked** — beyond the plan's q35shell-embedded test, the same lib test parses all five committed TOMLs and asserts the full mode-table set: the must-haves truth is enforced at test time, catching a half-migrated stanza before any boot gate.
3. **`default-run = "xtask"`** — Rule 3 fix; without it every `cargo xtask` invocation (including kernel-x64's flake buildPhase) dies with "could not determine which binary to run".

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] xtask's second bin broke the `cargo xtask` command surface**
- **Found during:** Task 3 (the gate battery)
- **Issue:** with `src/bin/r9-netdev-args.rs` beside the main bin, cargo's custom-subcommand protocol (`cargo xtask check` = `cargo run -p xtask -- check`) fails with "could not determine which binary to run" — breaking every xtask gate AND `kernel-x64`'s flake buildPhase (`cargo xtask build --arch x86-64 --config q35shell`)
- **Fix:** `default-run = "xtask"` in `xtask/Cargo.toml`; both `nix build .#kernel-x64` and the full gate battery re-proven green
- **Files modified:** `../r9/xtask/Cargo.toml`
- **Verification:** `cargo xtask check` + `cargo xtask clippy` exit 0; `nix build .#kernel-x64` green
- **Committed in:** `d1c4849`

**2. [Rule 3 - Blocking] nix's git-source filtering hid unstaged new files from the flake build**
- **Found during:** Task 3 (`nix build .#netdev-args`)
- **Issue:** the first nix build failed with "no bin target named r9-netdev-args" — the flake's `src = ./.` includes only git-TRACKED files, and the new bin/lib were still unstaged
- **Fix:** stage the new files before the flake build (workflow note, no code change); documented here because every future flake-verified r9 task hits it
- **Verification:** `nix build .#netdev-args` green after staging

**3. [Rule 2 - Missing critical functionality] the committed-TOML parse guard**
- **Found during:** Task 3 (reviewing what the embedded test covered)
- **Issue:** nothing machine-checked that the five COMMITTED TOMLs deserialize into the new struct shape — a half-migrated stanza would surface only at the next boot gate
- **Fix:** `netdev_all_five_q35_tomls_carry_the_mode_tables` embeds all five via `include_str!` and asserts default=slirp + the three sub-tables each (the must-haves truth at the data layer)
- **Files modified:** `../r9/xtask/src/lib.rs`
- **Committed in:** `d1c4849`

**4. [Plan-literal TDD shape] RED commits split from the atomic GREEN commits**
- **Found during:** Task 1
- **Issue:** the task text says both "tests red first" and "structs + five TOMLs + apply_netdev + tests in one commit"
- **Resolution:** the RED commit carries only the re-keyed tests (compiles against the old struct, fails on the planned assertions — r9's own `test(...)` precedent); the breaking migration landed atomically in the GREEN commit. Both plan clauses honored.
- **Committed in:** `cb96d56` / `e5ef95f` (and `34b1b9e` / `d7180c4` for Task 2)

---

**Total deviations:** 4 auto-fixed (1× Rule 3 toolchain, 1× Rule 3 workflow, 1× Rule 2 test guard, 1× documented TDD-shape resolution)
**Impact on plan:** No scope creep — every fix was load-bearing for the task's own gates; the emitter contract, TOML data, and prohibition (one emission implementation) are exactly as planned.

## Issues Encountered

- The first `nix build .#netdev-args` failed (see deviation 2); resolved by staging.
- 4 pre-existing dead-code warnings in `../r9/xtask/src/pcap.rs` (frame_len/src_mac/dst_mac on the icmp/bootp/tcp frame structs) — proven pre-existing by building the pristine base 5f856c9 in a throwaway worktree; out of scope, logged to `deferred-items.md`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 16-04 can pin r9 at `d1c4849dfa1d769240630c29b9b03b88d5819558` and substitute `$(r9-netdev-args --mode "$MODE")` in `boot-r9-x64.sh` with zero further r9 change (the success criterion)
- The parallel-boot constraint is now live for every Harvey-side gate driver that boots two q35 venues concurrently: both hostfwd ports bind at netdev init — run x86_64 gates sequentially
- No Harvey-repo production files were touched by this plan (its `files_modified` is entirely ../r9, as planned)

## Self-Check: PASSED

- Artifacts exist: ../r9/xtask/src/lib.rs, ../r9/xtask/src/bin/r9-netdev-args.rs, ../r9/flake.nix (plus all 10 modified files, per the commit file lists)
- All 5 r9 commits (cb96d56, e5ef95f, 34b1b9e, d7180c4, d1c4849) are ancestors of origin/main; r9 HEAD = d1c4849dfa1d769240630c29b9b03b88d5819558 (pushed)
- Gate evidence: cargo xtask check + clippy exit 0 at HEAD; cargo test -p xtask 111 tests 0 failed; nix build .#netdev-args + .#kernel-x64 green

---
*Phase: 16-host-net-modes-networking-exit-gate*
*Completed: 2026-11-11*
