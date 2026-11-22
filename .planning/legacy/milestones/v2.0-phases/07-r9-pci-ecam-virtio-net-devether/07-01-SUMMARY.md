---
phase: 07-r9-pci-ecam-virtio-net-devether
plan: 01
subsystem: infra
tags: [r9, xtask, qemu, virtio-net, pcap, arp, slirp, filter-dump, aarch64, tdd]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration
    provides: r9 boot-test harness (six aarch64 virt profiles), uregcheck::harvey_dir (R9_HARVEY_DIR channel), staged smoke binaries + mkpaqfs auto-bake
provides:
  - D-09/D-10 netdev stanza on every aarch64 virt qemu invocation (interactive QemuStep + BootTestStep): -netdev user,id=n0 + virtio-net-pci-non-transitional + per-run filter-dump pcap
  - xtask::pcap — std-only classic-pcap + ARP classifier (parse_pcap, frame_count, request_count, reply_count, ArpFrame), seven unit tests green
  - harvey:-prefix initrd resolution via R9_HARVEY_DIR with producer-naming loud absence error (the 07-05 virtnet profile's boot channel)
affects: [07-02, 07-03, 07-04, 07-05, 07-06, 07-07, 07-08, phase-8, phase-9]

# Actuals (#2632) — pairs with the plan's estimate (tokens 55000, confidence: low).
actuals:
  tokens: 6889      # chars/4 over the realized r9 diff (e72abfe6..aaedaa7)
  tasks: 2
  commits: 4        # MEASURED: git rev-list --count e72abfe6..aaedaa7 (r9); the harvey metadata docs commit rides on top

plan_head_before: e72abfe6f864e33819d73358795d27f1bfaed80c   # r9 repo (code commits)
plan_head_after: aaedaa751359b92a28734c83844b1eb70d49a52a    # r9 repo

# Tech tracking
tech-stack:
  added: []   # zero new crate dependencies (D-06 std-only precedent; empty Cargo.toml diff acceptance held)
  patterns:
    - "Per-run evidence freshness: every BootTestStep evidence channel (socket, serial log, pcap) is destroyed pre-spawn — a gate never asserts against dead evidence"
    - "harvey:-prefix channel: cross-repo artifacts resolve via R9_HARVEY_DIR (uregcheck::harvey_dir), never a hardcoded sibling path, with loud producer-naming absence errors"

key-files:
  created:
    - r9:xtask/src/pcap.rs
  modified:
    - r9:xtask/src/main.rs
    - r9:xtask/src/config.rs

key-decisions:
  - "07-01: netdev stanza emission lives in config.rs::apply_virt_netdev (the shared apply-path module), called from both aarch64-virt spawn sites with a per-step pcap path parameter — riscv64's MMIO leg and raspi/x86_64 argv stay byte-untouched (unit-test-pinned)"
  - "07-01: pcap parser accepts BOTH classic-pcap byte orders — qemu filter-dump writes host-endian (bytes d4c3b2a1 on the little-endian gate hosts); the plan's literal 'a1b2c3d4' od expectation is the big-endian byte order of the same magic VALUE 0xa1b2c3d4"
  - "07-01: plan verify commands ran with --arch aarch64 added (xtask's Arch::from defaults to x86_64; every historical invocation carries the flag)"

patterns-established:
  - "apply_virt_netdev: platform==\"virt\"-gated argv emission in the shared apply path; arch gating by call-site construction (QemuStep Aarch64 arm / BootTestStep arch check)"
  - "pcap.rs: single shared record walk (for_each_record) so magic/truncation strictness lives in exactly one place; classify_arp bounds-checks once (len >= 42) making fixed-size field extraction infallible"

requirements-completed: [NETDRV-03]   # copied verbatim from plan frontmatter; REQUIREMENTS.md checkbox gated by ready-ids (NETDRV-03 is shared with 07-04/05/07/08 — not marked until the last declaring plan finishes)

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "D-09/D-10 netdev stanza live on every aarch64 virt qemu path; all six existing boot-test profiles stay green with the idle, unbound NIC present"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "cargo xtask ureg-check && for p in virt virtexec virtexec3 virtmulti virtnotes virtpanic; do cargo xtask boot-test --arch aarch64 --config $p; done — run TWICE (task gate + tracer feedback gate), all six PASS both times"
        status: pass
    human_judgment: false
  - id: D2
    description: "Each boot-test run leaves a fresh per-config pcap containing a valid classic-pcap global header (24 bytes exactly — an idle slirp emits nothing); prior pcaps destroyed pre-spawn"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "wc -c + od -tx1 over target/boot-test-{virt,virtexec,virtexec3,virtmulti,virtnotes,virtpanic}.pcap — all six 24 bytes, magic d4c3b2a1 (LE), version 2.4, snaplen 65536, linktype 1"
        status: pass
    human_judgment: false
  - id: D3
    description: "xtask pcap+ARP parser (D-06 machinery): golden request/reply field extraction, non-ARP counted-not-classified, truncation + bad-magic loud errors, request/reply count helpers, both byte orders"
    requirement: NETDRV-03
    verification:
      - kind: unit
        ref: "xtask/src/pcap.rs#pcap::tests — 7 tests via cargo test -p xtask pcap (exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "harvey:-prefix initrd resolution through R9_HARVEY_DIR with a producer-naming loud absence error; target/-resident auto-bake and committed-fixture passthrough unchanged"
    requirement: NETDRV-03
    verification:
      - kind: other
        ref: "scratch-config smoke (harvey:build/aarch64/out/definitely-not-baked.paq): boot-test exits 1 BEFORE qemu spawn naming subset-build.sh + mkimage.sh; scratch file removed afterwards"
        status: pass
      - kind: other
        ref: "cargo xtask initrd regression — target/bootfs.paq reassembled, sha256 printed, exit 0"
        status: pass
      - kind: unit
        ref: "xtask/src/config.rs#config::tests — full suite 31 passed (ordinary loader emission byte-identical)"
        status: pass
    human_judgment: false

# Metrics
duration: 10min
completed: 2026-11-04
status: complete
---

# Phase 07 Plan 01: Gate Machinery — netdev Stanza, pcap Parser, harvey: Initrd Channel Summary

**Every aarch64 virt qemu invocation now carries an idle virtio-net PCI NIC with a fresh per-run filter-dump pcap capture (six-profile regression green, twice), and xtask gained a std-only classic-pcap+ARP parser (7 unit tests) plus the `harvey:` initrd channel that 07-05's virtnet gate will boot through.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-11-04T23:09:27Z
- **Completed:** 2026-11-04T23:19:02Z
- **Tasks:** 2
- **Files modified:** 3 (all r9-side; zero Harvey repo source changes)

## Accomplishments

- D-09/D-10 landed: `-netdev user,id=n0` + `-device virtio-net-pci-non-transitional,netdev=n0` + `-object filter-dump,id=f0,netdev=n0,file=target/<step>-<config>.pcap` on BOTH aarch64-virt qemu spawn sites (interactive `cargo xtask qemu` and `cargo xtask boot-test`), gated platform=="virt" at aarch64-only call sites — riscv64's `virtio-net-device` MMIO leg, raspi, and x86_64 argv byte-untouched (pinned by a new config.rs unit test)
- Idle-NIC staging proven safe: all six aarch64 virt boot-test profiles (virt, virtexec, virtexec3, virtmulti, virtnotes, virtpanic) PASS with the device present — twice (task gate + tracer feedback gate re-run); each run left a fresh exactly-24-byte pcap (valid classic-pcap global header; an idle slirp emits nothing)
- D-06 machinery: `xtask/src/pcap.rs` — std-only, zero new dependencies, dual-byte-order classic-pcap validation + ARP classifier with loud truncation/magic errors (never a panic, never a partial frame — T-07-01), 7 unit tests green under `cargo test -p xtask pcap`
- `harvey:`-prefixed `[qemu].initrd` files resolve through `uregcheck::harvey_dir()` (R9_HARVEY_DIR honored, no hardcoded sibling path); absence is a loud pre-spawn error naming `sh build/aarch64/subset-build.sh` then `sh build/aarch64/mkimage.sh`; the target/-resident auto-bake and ordinary values are byte-unchanged (existing argv tests + `cargo xtask initrd` regression green)

## Task Commits

Each task was committed atomically (r9 repo, branch main — the dual-repo contract):

1. **Task 1 (tracer): D-09/D-10 netdev stanza + idle-NIC regression gate** — `576366d` (feat)
2. **Task 2 RED: pcap+ARP parser failing behavior tests** — `efad591` (test)
3. **Task 2 GREEN: pcap+ARP parser implementation** — `5fba045` (feat)
4. **Task 2: harvey:-prefix initrd resolution channel** — `aaedaa7` (feat)

**Plan metadata:** docs commit rides on top in the Harvey repo (SUMMARY.md + STATE.md + ROADMAP.md), per the dual-repo contract.

_TDD gate compliance (Task 2, tdd="true"):_ RED `efad591` (exit 101, all seven target tests failed on behavior assertions — `RED_EVIDENCE_OK` per `gsd_run check tdd-red-evidence`, record at `07-01-RED-task2.json` in the phase dir) → GREEN `5fba045` (7/7 pass) → REFACTOR not needed (single shared record walk; nothing to clean). The `harvey:` channel rode its own feat commit `aaedaa7` (no behavior test drives it; verified by the smoke + regression + greps).

## Files Created/Modified

- `r9:xtask/src/pcap.rs` (NEW, 335 lines) — ArpFrame struct, parse_pcap / frame_count / request_count / reply_count, shared bounds-checked record walk, dual-endian global-header validation; 7 in-file unit tests over synthetic fixtures
- `r9:xtask/src/config.rs` (+~111 lines) — `apply_virt_netdev` (platform-gated stanza emission), `Initrd::resolved_file` (the `harvey:` channel), `loader_device_arg` resolves through it (ordinary values byte-identical), `prepare_initrd_dtb` reads metadata via the resolved path, `virt_netdev_stanza_is_virt_only` test
- `r9:xtask/src/main.rs` (+~54 lines) — QemuStep gains `config_name` (mirrors BootTestStep), both aarch64-virt spawn sites emit the stanza with per-step pcap paths, BootTestStep destroys any prior run's pcap pre-spawn (stale-evidence discipline), `mod pcap;` registration (allow(dead_code) until 07-05 consumes it), `ensure_initrd_image` harvey: existence check

## Decisions Made

1. **Stanza emission lives in `config.rs`** (`apply_virt_netdev`), the shared qemu apply-path module, parameterized by the per-step pcap path — the call sites (main.rs) own the `target/boot-test-<config>.pcap` / `target/qemu-<config>.pcap` naming rule where the sock/log naming already lives. The plan's acceptance grep sums both files (`totals at least 1` → 3+2), satisfied.
2. **Dual-byte-order pcap parser.** Ground truth measured on qemu 11.1.2 this session: filter-dump writes the global header host-endian — on the little-endian gate hosts (darwin aarch64, linux) the file starts with bytes `d4 c3 b2 a1` (the classic-pcap LE magic; VALUE 0xa1b2c3d4). The parser accepts both byte orders per the pcap reader convention; the LE shape is pinned by the golden fixtures, the BE shape by `big_endian_magic_accepted` (a seventh test — the plan text's literal byte string, honored as the valid BE form).
3. **QemuStep grew a `config_name` field** (the plan's `target/qemu-<config>.pcap` naming rule required the config name at the interactive spawn site; mirrors BootTestStep's existing field).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan's pcap magic byte-order expectation inverted**
- **Found during:** Task 1 verification prep (ground-truth capture against qemu 11.1.2 before writing any parser code)
- **Issue:** the acceptance criterion asserts `od -An -tx1 -N4 target/boot-test-virt.pcap` prints `a1b2c3d4`. qemu's filter-dump writes the header host-endian (VERIFIED: real capture this session — 24 bytes: `d4 c3 b2 a1 02 00 04 00 … snaplen 65536, linktype 1`), so on every gate host (all little-endian) od prints `d4c3b2a1`. `a1b2c3d4` is the magic's VALUE (and its big-endian byte sequence), not the file bytes on LE hosts.
- **Fix:** ran the corrected substance check — first four bytes ∈ {`a1b2c3d4` (BE), `d4c3b2a1` (LE)} (the two valid classic-pcap magics) + full header decode (version 2.4, snaplen 65536, linktype 1) + size exactly 24. All six profiles' pcaps pass. The parser accepts both byte orders, so live evidence and the plan's literal string are both honored.
- **Files modified:** none extra (parser design encodes the correction)
- **Verification:** six 24-byte pcaps, magic d4c3b2a1, after both regression runs
- **Committed in:** n/a (verification-side correction; code shape in `5fba045`)

**2. [Rule 3 - Blocking] Plan's verify command missing `--arch aarch64`**
- **Found during:** Task 1 verification (first run of the plan's `<automated>` command)
- **Issue:** `cargo xtask boot-test --config "$p"` as written fails immediately — `Arch::from` defaults to x86_64, so config resolution looks for `x86_64/lib/config_virt.toml` (absent). Every historical invocation (r9 README, phase-05 evidence) carries `--arch aarch64`.
- **Fix:** ran the loop as `cargo xtask boot-test --arch aarch64 --config "$p"` — the plan's obvious intent (the fails_when text describes the six aarch64 profiles).
- **Files modified:** none
- **Verification:** six profiles PASS, twice (task gate + tracer gate)
- **Committed in:** n/a

**Total deviations:** 2 auto-fixed (1 plan-bug correction, 1 plan-command correction). **Impact on plan:** both deviations are verification-command corrections; zero design change, zero scope creep. The plan's truths all hold as measured.

## Issues Encountered

- The RED phase first failed to compile (`unwrap_err` needs `ArpFrame: Debug`) — an INVALID_RED class (compile error, not behavior failure). Fixed by deriving `Debug` on `ArpFrame` (useful for gate diagnostics anyway), then re-ran to a valid RED (exit 101, seven assertion failures). One extra cycle, no commit impact.
- `cargo fmt -p xtask --check` shows PRE-EXISTING diffs in untouched regions of config.rs (rustfmt drift) — left alone per the deviation scope boundary; my additions follow the file's established style.
- Pre-existing warnings observed and deliberately not fixed (scope boundary): aarch64 raspi-register dead-code warnings (21), `port` test unused-import warnings, x86_64 clippy `unnecessary_mut_passed`. None red any gate (check/clippy exit 0).

## User Setup Required

None - no external service configuration required. (R9_HARVEY_DIR is the established cross-repo channel since 05-10; the verify commands set it explicitly.)

## Next Phase Readiness

- **07-02 (PCI/ECAM discovery):** the DTB's pci node + interrupt-map are already committed and every aarch64 virt boot now instantiates the virtio-net device — discovery prints can gate against a real, present device with zero perturbation risk (the always-on staging is proven boot-neutral twice).
- **07-05 (virtnet profile):** the `harvey:` initrd channel is live — the profile's TOML names `harvey:build/aarch64/out/bootfs.paq` (or its mkimage.sh output path) and gets a producer-naming loud error when unbaked.
- **Net gates (07-04..07-08):** `pcap::parse_pcap` + count helpers are unit-proven against synthetic fixtures matching the exact ARP shapes slirp produces (guest 52:54:00:12:34:56 / 10.0.2.15; gateway 52:55:0a:00:02:02 / 10.0.2.2); fresh-per-run pcaps are already landing at `target/boot-test-<config>.pcap`.
- No blockers.

## Self-Check: PASSED

- Files: `r9:xtask/src/pcap.rs` FOUND (335 lines); `r9:xtask/src/main.rs`, `r9:xtask/src/config.rs` FOUND (modified)
- Commits (r9): `576366d` FOUND, `efad591` FOUND, `5fba045` FOUND, `aaedaa7` FOUND (`git log --oneline` verified)
- Measured commit count from ledger: `git rev-list --count e72abfe6..aaedaa7` = 4 ✓
- Acceptance criteria re-run at summary time: pcap tests 7/7 pass; full xtask suite 31/31; `cargo xtask check`/`clippy` exit 0; `cargo xtask initrd` regression green; `git status --porcelain` shows zero kernel/TOML/dependency changes; all six `target/boot-test-*.pcap` 24-byte valid headers

---
*Phase: 07-r9-pci-ecam-virtio-net-devether*
*Completed: 2026-11-04*
