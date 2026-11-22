---
phase: 04-r9-userspace-exec-harvey-r9-abi
plan: 02
subsystem: infra
tags: [r9, aarch64, paqfs, no_std, initrd, qemu, tdd]

requires:
  - phase: 04-r9-userspace-exec-harvey-r9-abi (plan 01)
    provides: committed probe-tree.paq fixture (sha256 27b7482e…), loader+patched-DTB initrd delivery, kernel /chosen discovery + validation + pagealloc reservation, virtexec profile
provides:
  - port::paqfs no_std reader — open_image / read_block / root_dir / read_dir / walk / read_at, stateless-per-call, trap-context-safe (no statics, no locks, typed Errs on hostile input)
  - host byte-level verification of the D-02 format against real mkpaqfs output (synthetic NoEnc image + committed fixture with DeflateEnc blocks + corrupt-input negatives + PaqDir bounds)
  - boot-time root-walk serial proof: banner → initrd marker → `paqfs: dir` / `paqfs: hello.txt`
  - the reader 04-04's fault handler (read_at page-in) and 04-05's devroot (walk/read_dir) consume
affects: [04-03, 04-04, 04-05]

actuals:
  tokens: 14500
  tasks: 2
  commits: 5
  plan_head_before_r9: 04e6dcb9cef22eac68a357bfbc3cec2ace005bb0
  plan_head_after_r9: 387bc975a1d423e748a13a8157cef7b50a42212f
  plan_head_before_harvey: cb1784a7f9e1b0b0b1100fe1d4580fe3d6db54f8

tech-stack:
  added:
    - miniz_oxide 0.9 (default-features=false, features=[with-alloc]) — raw RFC 1951 inflate for DeflateEnc blocks, no_std
    - adler2 2 (default-features=false) — block payload checksums, seeded via Adler32::from_checksum(0)
  patterns:
    - "Harvey libflate adler32 is the SEED-0 variant (s1=s2=0), not RFC 1950 (s1=1): reproduce with adler2::Adler32::from_checksum(0); fixture block headers pin this byte-for-byte"
    - "Runtime kernel access to a physical range: kernel_pagetable().map_phys_range(…, VaMapping::Offset(KZERO), rw_kernel_data, Page4K, Kernel) — the vminit DTB convention, applied post-boot; r9 has NO full-RAM kernel map"
    - "TDD RED on a new Rust module: tests + compiling API skeleton with Err bodies, so tests fail on assertions for planned behavior (never a compile error — that is INVALID_RED)"

key-files:
  created:
    - ../r9/port/src/paqfs.rs (no_std paqfs reader, 304 lines)
    - ../r9/port/tests/paqfs_test.rs (5 host tests + mkpaqfs-compatible synthetic image builder)
    - .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-02-red-evidence.json (TDD RED gate record)
  modified:
    - ../r9/port/src/lib.rs (pub mod paqfs)
    - ../r9/port/Cargo.toml (+miniz_oxide, +adler2), ../r9/Cargo.lock
    - ../r9/aarch64/src/main.rs (initrd KZERO+PA map + paqfs root walk, banner-first order)
    - ../r9/aarch64/lib/config_virtexec.toml (initrd = the paqfs fixture)
    - ../r9/HowItWorks.md (paqfs section + main9 initrd step — file-header mandate)

key-decisions:
  - "Harvey adler32 seed-0 vs RFC-1950 seed-1: proven against fixture bytes (root block header 0x479d0ee0 = seed-0; seed-1 yields 0x579d0ee1). Resolved inside the plan-mandated adler2 crate via Adler32::from_checksum(0) — no hand-rolled checksum, no plan deviation"
  - "The plan's 'initrd PA range maps at KZERO+start (existing physical-map convention)' assumed a full-RAM kernel map r9 does not have (vminit maps only DTB+kernel sections). main9 now maps the initrd range explicitly via kernel_pagetable().map_phys_range with VaMapping::Offset(KZERO) — same convention as vminit's DTB map, table pages from pagealloc, data pages already reserved (04-01)"
  - "Reader API grew root_dir()/read_dir() beyond the plan's 4-method sketch: Task 2's root walk and 04-05's devroot readdir are inexpressible otherwise; stateless-per-call invariant preserved (Vec per call)"
  - "Serial evidence emits banner FIRST, then initrd marker, then paqfs lines (acceptance pins the order): the 04-01 marker block moved below the 'r9 from the Internet' banner"

patterns-established:
  - "Boot-proof marker chain: banner → initrd: start=… end=… size=… → paqfs: <name> per root entry; zero 'paqfs: open failed', zero panic/Unhandled lines; asserted by grep chain + an order assertion on line numbers"
  - "Fixture DeflateEnc coverage guard: the fixture test scans block headers linearly (re-bake-robust, no hardcoded offsets) and asserts ≥1 DeflateEnc DataBlock, so a future re-bake can't silently deflate the inflate-path coverage"

requirements-completed: [R9-03]

coverage:
  - id: D1
    description: "port::paqfs parses real mkpaqfs output on the host with both encodings verified (D-02 byte-level verification complete): 44-byte header, 28-byte trailer, 12-byte block headers, PaqDir entries, pointer-block chases with 0-offset termination; NoEnc + DeflateEnc decode; corrupt magic / unknown encoding / unknown type / out-of-image offset / PaqDir overrun all typed Errs, zero panics"
    requirement: "R9-03"
    verification:
      - kind: integration
        ref: "cargo test -p port → 26/26 ok (12 port unit + 9 fdt + 5 paqfs: synthetic_noenc_walk_and_read, committed_fixture_walk_and_read [byte-exact vs probe-tree sources, window reads, deflate-block guard], committed_fixture_root_listing, corrupt_inputs_error_no_panic, paqdir_size_overrun_terminates_scan); TDD RED gate record 04-02-red-evidence.json → RED_EVIDENCE_OK before GREEN"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-01/D-02 delivery chain proven end-to-end with real bytes: host mkpaqfs fixture → generic-loader + fdtput-patched DTB → /chosen discovery → validation → reservation → KZERO+PA map → paqfs parse → root entries on serial; emission order banner → initrd marker → paqfs lines; no-initrd path ('initrd: none') untouched"
    requirement: "R9-03"
    verification:
      - kind: integration
        ref: "cargo xtask qemu --arch aarch64 --config virtexec, serial captured to target/boot-logs/virtexec-paq.log (gitignored): 'initrd: start=0x48000000 end=0x480003d6 size=982' then 'paqfs: dir' + 'paqfs: hello.txt'; grep chain (banner, initrd marker, both paqfs markers, zero open-failed, zero panic|Unhandled) all PASS; line-number order assertion banner(197) < initrd(200) < paqfs(201,202) PASS"
        status: pass
    human_judgment: false
  - id: D3
    description: "Platform no-regression: Phase-3 virt boot-test profile PASS (banner/ticks/echo), raspi3b (default) + raspi4b builds green; port crate still compiles no_std for the kernel target; clippy clean of paqfs warnings"
    requirement: "R9-03"
    verification:
      - kind: integration
        ref: "cargo xtask boot-test --arch aarch64 --config virt → PASS (target/boot-test-virt.log); cargo xtask build --arch aarch64 (raspi3b default) exit 0; cargo xtask build --arch aarch64 --config raspi4b exit 0; cargo xtask build --config virt exit 0; cargo xtask clippy --config default exit 0 with zero paqfs diagnostics"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-11-02
status: complete
---

# Phase 04 Plan 02: paqfs Reader + Boot-Time Root Walk Summary

**r9's port crate gained a no_std paqfs reader (D-02) — host-verified byte-level against real mkpaqfs output through TDD (RED_EVIDENCE_OK → GREEN), both encodings decoded with Harvey's seed-0 adler32 reproduced via adler2::Adler32::from_checksum(0) — and the kernel proved the full delivery chain at boot: initrd fixture mapped at KZERO+PA, root walked, `paqfs: dir` / `paqfs: hello.txt` on serial with zero failures and the Phase-3 profiles all green.**

## Performance

- **Duration:** 15min
- **Started:** 2026-11-02T12:39:25Z
- **Completed:** 2026-11-02
- **Tasks:** 2
- **Files modified:** 9 (2 created + 7 modified in r9; 1 evidence record created in Harvey)

## Accomplishments

- **Task 1 (TDD): port::paqfs reader.** RED: 5 tests + compiling API skeleton committed first (`2fbd373`); the gate record (04-02-red-evidence.json, exit 101, 5/5 named failures incl. target `synthetic_noenc_walk_and_read`) classified **RED_EVIDENCE_OK** before any implementation. GREEN (`eaf5a4a`): full parser — header/trailer/block parse with big-form rejection, NoEnc full-blocksize reads, DeflateEnc raw inflate hard-capped at blocksize (T-04-04), per-block adler32 of the decoded payload (T-04-03), PaqDir scan with checkDirSize bounds discipline, pointer-block chase with zero-termination, byte-granular `read_at` clamped to PaqDir.length (the Pitfall-5 contract for 04-04). Two discoveries pinned by fixture bytes: **Harvey's adler32 is seed-0** (reproduced via `Adler32::from_checksum(0)` — the plan-mandated crate stays), and the fixture's blocks are all DeflateEnc (NoEnc coverage comes from the synthetic image, exactly as the plan structured it; a re-bake-robust scan asserts the fixture keeps ≥1 DeflateEnc DataBlock).
- **Task 2: boot-time root walk.** `config_virtexec.toml` now delivers the committed paqfs fixture as the initrd; main9 maps the reserved initrd range into the kernel table at KZERO+PA (explicit `map_phys_range` — r9 has no full-RAM map, see deviation 2) and prints one `paqfs: <name>` line per root entry after the banner. Serial evidence: `initrd: start=0x48000000 end=0x480003d6 size=982` → `paqfs: dir` / `paqfs: hello.txt`, zero `paqfs: open failed`, zero panic/Unhandled lines. `initrd: none` backward-compat intact (virt profile untouched).
- **No-regression:** virt boot-test PASS, raspi3b+raspi4b builds green, clippy clean, 26/26 host tests.

## Task Commits

**r9 repo** (HEAD after plan: `387bc975a1d423e748a13a8157cef7b50a42212f`):

1. `2fbd373ba85b2d15ccc202b764f7f32362052c44` — **test(04-02)** RED: failing tests + API skeleton (TDD)
2. `eaf5a4aaf6402af52989ca26c980dbe458877f50` — **feat(04-02)** GREEN: paqfs reader + deps
3. `1552c1613adeb193e5334f290415644414a613b8` — **feat(04-02)** Task 2: boot-time root walk + config
4. `387bc975a1d423e748a13a8157cef7b50a42212f` — **docs(04-02)** HowItWorks.md (file-header mandate)

**Harvey repo:** plan metadata + RED evidence record committed with this SUMMARY (see final commit).

**TDD gate validation:** r9 log carries `test(04-02):` (RED) before `feat(04-02):` (GREEN) — compliant; no refactor commit (no post-GREEN changes were needed).

## Files Created/Modified

- `../r9/port/src/paqfs.rs` — the reader (new)
- `../r9/port/tests/paqfs_test.rs` — 5 tests + synthetic-image builder (new)
- `../r9/port/src/lib.rs`, `../r9/port/Cargo.toml`, `../r9/Cargo.lock` — module + deps
- `../r9/aarch64/src/main.rs` — initrd map + paqfs proof block (banner-first)
- `../r9/aarch64/lib/config_virtexec.toml` — initrd = `port/tests/fixtures/probe-tree.paq`
- `../r9/HowItWorks.md` — paqfs + initrd documentation
- `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-02-red-evidence.json` — RED gate record

Fixture (unchanged this plan, 04-01 artifact): `../r9/port/tests/fixtures/probe-tree.paq`, sha256 `27b7482ed68562c891a87ee6a3f4ca31633570b9f61de6d1a902eb61ccd21300`, 982 bytes, 9 blocks (2 Dir + 2 Data + 3 Ptr + root Dir + root Ptr), all DeflateEnc.

## Decisions Made

See key-decisions: seed-0 adler32 via `from_checksum(0)`; the explicit initrd KZERO+PA runtime map; `root_dir`/`read_dir` API addition; banner-first emission order.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — missing critical] `root_dir()` / `read_dir()` added to the reader API**
- **Found during:** Task 2 (consuming the Task-1 API)
- **Issue:** the plan's 4-method sketch (open_image/read_block/walk/read_at) cannot express "walk the root DirBlock and print one line per root entry" — no listing operation exists; 04-05's devroot readdir needs the same
- **Fix:** added `root_dir()` (first entry of the trailer-root DirBlock, paqfs.c:914-938 semantics) and `read_dir()` (pointer-chase + per-block scan returning Vec<PaqDir>), both stateless-per-call; covered by the `committed_fixture_root_listing` test (exactly {hello.txt, dir})
- **Files modified:** ../r9/port/src/paqfs.rs, ../r9/port/tests/paqfs_test.rs
- **Commit:** eaf5a4a (r9)

**2. [Rule 3 — blocking plan defect] initrd bytes are NOT mapped at KZERO+PA by any existing convention**
- **Found during:** Task 2 implementation (pre-write code read)
- **Issue:** the plan asserts "the initrd PA range maps at KZERO+start (the kernel's existing physical-map convention)" — but r9 has no full-RAM kernel map: vminit maps only the DTB + kernel text/rodata/data sections; dereferencing KZERO+0x48000000 would fault
- **Fix:** main9 maps the (page-rounded, pagealloc-reserved) initrd range explicitly via `kernel_pagetable().map_phys_range(…, VaMapping::Offset(KZERO), rw_kernel_data, Page4K, Kernel)` — the same Offset(KZERO) convention vminit uses for the DTB; only table pages are allocated. The slice is then constructed at KZERO+start exactly as the plan intends
- **Files modified:** ../r9/aarch64/src/main.rs
- **Commit:** 1552c16 (r9)

**3. [Rule 3 — blocking plan defect] Task-2 verify command targets the falsified qemu -initrd mechanism**
- **Found during:** Task 2 verification
- **Issue:** the plan's verify leg runs raw qemu with `-initrd` — the mechanism 04-01 falsified (A3: qemu 11.1.2 never delivers -initrd for ELF kernels); it also omits `--arch aarch64` (xtask defaults to x86_64)
- **Fix:** ran the shipped amended-D-01 path: `cargo xtask qemu --arch aarch64 --config virtexec` (generic loader + fdtput-patched DTB from TOML data) with serial captured to `target/boot-logs/virtexec-paq.log`; the plan's grep marker chain + zero-failure assertions applied unchanged and all PASS
- **Files modified:** none (verification invocation only)

**4. [Rule 1 — acceptance mismatch] banner-first emission order**
- **Found during:** Task 2 acceptance (order assertion failed: initrd 196 < paqfs 197-198 < banner 200)
- **Issue:** 04-01 placed the initrd marker print immediately after devcons::init, BEFORE the `r9 from the Internet` banner; the 04-02 acceptance criterion pins "banner → initrd → paqfs (banner first)"
- **Fix:** moved the marker+proof block below the banner print; re-ran — banner(197) < initrd(200) < paqfs(201,202); 04-01's order-insensitive greps unaffected
- **Files modified:** ../r9/aarch64/src/main.rs
- **Commit:** 1552c16 (r9)

**Total deviations:** 4 auto-fixed (2 plan defects, 1 API gap, 1 acceptance mismatch). No Rule 4 (architectural) items.
**Impact on plan:** all plan truths, acceptance criteria, and success criteria met; deviations are recorded as key decisions where they shape downstream plans (04-04/04-05 consume the map convention and the read_dir API).

## Issues Encountered

- One harness-side retry: `cargo xtask qemu --config virtexec` without `--arch aarch64` targets x86_64 ("Could not read config_virtexec.toml" for x86_64/lib) — invocation fix only, no code change.
- No GREEN-phase debugging needed: the reader passed all 5 tests first-try, aided by pre-implementation fixture ground-truthing (a Python decode of probe-tree.paq established the block map, string-prefix semantics, and the seed-0 adler32 discovery before any Rust was written).

## User Setup Required

None.

## Next Phase Readiness

- **04-04** (fault handler): `paqfs::read_at` byte-granular window reads are the text/data page-in source; the reader is reentrant-by-construction (no statics/locks/panics) for trap-context use. The KZERO+PA map convention for the initrd is established — the fault handler reads the same mapping this plan creates.
- **04-05** (devroot): `walk` + `root_dir`/`read_dir` are the find/list primitives; the fixture tree (hello.txt + dir/nested.txt) is the natural devroot test content.
- **04-03**: the virtexec profile now boots the paqfs fixture; exec-path boot profiles inherit the TOML-data initrd shape.
- r9 HEAD for downstream pinning (D-13): `387bc975a1d423e748a13a8157cef7b50a42212f`.

## Self-Check: PASSED

- Key files exist: `port/src/paqfs.rs` ✓, `port/tests/paqfs_test.rs` ✓, `04-02-red-evidence.json` ✓, main.rs/config_virtexec.toml/HowItWorks.md changes committed ✓.
- Commits: `git -C ../r9 log --grep="04-02"` → 2fbd373 (test), eaf5a4a (feat), 1552c16 (feat), 387bc97 (docs); TDD order RED-before-GREEN ✓. Harvey metadata commit below.
- Acceptance criteria re-run at final state: Task-1 greps (mod line, deps, magics, decompress fn, no-panic/no-static-mut) PASS; `cargo test -p port` 26/26 ok; virt kernel build + raspi3b/raspi4b builds + clippy exit 0; serial marker chain + emission-order assertion PASS on virtexec-paq.log; virt boot-test PASS at the final kernel state.
- Fixture untouched (sha256 27b7482e… verified against 04-01 record).

---
*Phase: 04-r9-userspace-exec-harvey-r9-abi*
*Completed: 2026-11-02*
