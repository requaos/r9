---
phase: 14-harvey-amd64-userspace-on-r9
plan: 03
subsystem: exec-substrate-loader
tags: [exec, a.out, usermem, x86_64, r9, tdd, xexec-02]
duration: 12 min
completed: 2026-11-08
requires:
  - "14-02-complete r9 HEAD 2b16f61 (the frozen 184-byte Ureg, the armed SYSCALL entry trio, touser/SYSRETQ)"
provides:
  - "x86_64 usermem.rs — the validated user-access surface (UserPtr/UserSlice/UserSliceMut fused constructors, validated_only variants, UserString validate-each-byte), the single choke point every user VA crosses through (T-14-05 mitigation)"
  - "x86_64 exec.rs — the amd64 a.out loader: parse_aout validation battery (S_MAGIC 0x8A97, checked be readers, loud named Errs before ANY mapping), 6l-derived segment constants, segment_layout, seed_exec_frame on the 184-byte Ureg"
  - "dat.rs Ureg::zeroed/seed_exec_entry (the 9k sysexecregs + touser entry state: bp=Tos via RARG, ip, If flags, flat user selectors) — the frame seed 14-04's proc machinery and 14-05's bootstrap consume"
affects:
  - "../r9/x86_64/src/{usermem,exec,dat,trap,main}.rs"
  - "14-04 (Proc implements UserAddressSpace; install_image/exec_into/exec_replace mapping bodies), 14-05 (exec_bootstrap consumes seed_exec_frame + touser)"
tech-stack:
  added: []
  patterns:
    - "trait-predicate module port: the aarch64 Proc-coupled usermem compiled ahead of its proc via UserAddressSpace (validate_user_range signature carried verbatim)"
    - "RED-stub TDD with exact named-Err string assertions — every test fails for its planned reason under the stub, zero accidental greens"
key-files:
  created:
    - ../r9/x86_64/src/usermem.rs
    - ../r9/x86_64/src/exec.rs
  modified:
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/src/dat.rs
    - ../r9/x86_64/src/trap.rs
key-decisions:
  - "usermem's validation predicate is the UserAddressSpace trait — the exact aarch64 Proc::validate_user_range(va, len, writable) shape as a trait so the module compiles ahead of 14-04's proc port; 14-04's Proc implements it with zero usermem interface churn"
  - "the loader constants are the 6l HEADTYPE-2 set (HDR_SIZE 40, INITTEXT 0x200028, TEXT_MAP_BASE 0x200000, DATA_ALIGN 2 MiB per INITRND, USTACKTOP 0x00007ffffffff000/16 MiB per ABI.md section 7) — synthetic-header host tests pin them now; A1's real-binary proof rides 14-05's stage-build magic assert"
  - "USTOP is unreachable by u32 a.out fields on the 128 TiB user map (~8 GiB max vs ~140 TiB): the donor check carried as factored defense-in-depth (check_data_bss_below_ustop) and its boundary arithmetic host-pinned at the function parse_aout calls"
  - "the Ureg seed lives on dat.rs (seed_exec_entry) — the frozen frame's own module — planting 9k sysexecregs' bp=Tos/sp/ip/type=64 plus touser's If-flags and flat selectors via the pub(crate)'d witnessed USER_TEXT/USER_DATA_SEL consts"
requirements-completed: []   # XEXEC-02 is declared by 14-02/03/04/05 (shared-ID gate): completes with 14-05
commits: 3
plan_head_before: 2b16f61744cdc61d781d5700f3632e20d745aa85   # r9 repo (all code r9-side; harvey ledger 98775ca1e for the docs commit)
plan_head_after: 822b3ec                                    # r9 repo
actuals:
  tokens: 10124   # chars/4 over the realized r9 diff (2b16f61..822b3ec: 851+/4-)
  tasks: 2
  commits: 3      # r9 repo (393f4f0, 8c4285a, 822b3ec); the harvey-side docs commit follows this file
status: complete
coverage:
  - deliverable: "usermem.rs — the validated user-access surface with host tests (Task 1, tracer)"
    verification:
      - kind: command
        ref: "cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu usermem — 8 passed (fused-vs-validated_only residency split, bounds Options, validate-each-byte boundary)"
        status: pass
      - kind: command
        ref: "cargo xtask check + clippy exit 0 (A/B vs HEAD: warning counts equal, none in new code); q35 boot-test PASS (module compile-present, call-site-free)"
        status: pass
      - kind: command
        ref: "tracer gate end-to-end re-run: usermem tests + check + clippy + q35 boot-test all green"
        status: pass
      - kind: command
        ref: "acceptance greps: validated_only=14 (>=2), mod usermem=1 (baseline 0), CR3/PML4=11 (>=1), aarch64+port diff empty"
        status: pass
    human_judgment: false
  - deliverable: "exec.rs — the amd64 a.out loader validation battery + segment math + Ureg seed (Task 2, TDD)"
    verification:
      - kind: command
        ref: "RED r9@8c4285a: 0 passed / 10 failed, every failure on its planned assertion (named-Err mismatch, layout zeros, seed zeros) — zero accidental greens"
        status: pass
      - kind: command
        ref: "GREEN r9@822b3ec: 10 passed / 0 failed incl. empty-input, empty-data (text-only) and 2 MiB-boundary adjacency edges; seed read back at ABI.md byte offsets"
        status: pass
      - kind: command
        ref: "cargo xtask check + clippy exit 0; q35 boot-test PASS; usermem 8/8 still green with the loader linked in"
        status: pass
      - kind: command
        ref: "acceptance greps: 0x8A97=2 (>=1), 0x200000=8 (>=2), 0x0000_7fff_ffff_f000=1 (>=1), mod exec=2 (baseline 0), exec filter 10 passed"
        status: pass
    human_judgment: false
---

# Phase 14 Plan 3: Exec Substrate — usermem + the amd64 a.out Loader Summary

Ported the validated user-memory surface (usermem.rs, CR3/live-memory SAFETY re-key, UserAddressSpace trait predicate) and the amd64 a.out loader (exec.rs: parse_aout validation battery, 6l-derived segment constants with the 2 MiB data alignment, segment_layout, the 184-byte-Ureg seed) to the r9 x86_64 crate — host-verified via a RED/GREEN test battery covering the empty and adjacent-segment edges.

**Duration:** 12 min (2026-11-08T19:49:42Z → 2026-11-08T20:02:08Z)
**Tasks:** 2/2 complete
**Files:** 5 (2 created, 3 modified — all r9-side; nothing Harvey-side beyond this summary)

## Commits

| Repo | Hash | Message |
|------|------|---------|
| r9 | 393f4f0 | feat(14-03): port usermem — the validated user-access surface (CR3/live-memory re-key) |
| r9 | 8c4285a | test(14-03): the exec validation battery — RED against the loader stubs |
| r9 | 822b3ec | feat(14-03): the amd64 a.out loader — parse/validate, segment math, Ureg seed |

## Accomplishments

### Task 1: usermem.rs port — validated user access on the per-process live-memory model (tracer)

- **Near-verbatim aarch64 sibling port** (185 lines → 404 with tests): `UserSlice`/`UserSliceMut` fused constructors (validate + force-resident before anything dereferenceable exists), the documented `validated_only` variants (the arg block at SP+8 and NOTED's saved-ureg window — residency by construction), per-struct SAFETY comments on every raw materialization, `UserPtr`, and `UserString::copy_from` with its validate-each-byte loop.
- **SAFETY narrative re-keyed** per the locked model: the aarch64 "TTBR0 is live" justification becomes "the calling proc's CR3 is loaded — its user half is visible in kernel context through the per-process PML4" (PROJECT.md: no Linux-style copy layer).
- **The predicate is the `UserAddressSpace` trait** — signature-for-signature aarch64's `Proc::validate_user_range(va, len, writable)` + `fault::ensure_user_resident(proc, start, end)` pair — so the module compiles ahead of 14-04's proc port and plugs in without interface churn. Constructors are generic over `impl UserAddressSpace`.
- **8 host tests over a mock user space backed by real memory** (the mock's segment IS the buffer's address range, so the raw materializations genuinely dereference): fused-vs-validated_only residency split asserted via recorded demands, out-of-segment rejection before any residency demand, `read_u64`/`write_u64` bounds Options, and the validate-each-byte boundary (a string walking past the segment end errs, never reads wild).
- Wired `#[allow(dead_code)] mod usermem` in main.rs behind the aarch64 `mod exec`/`mod proc` precedent; `extern crate alloc` added at the crate root.
- **Tracer feedback gate** (end-of-phase mode, automated-only verify): end-to-end re-run green — `⚡ Tracer verified end-to-end — expanding`.

### Task 2: exec.rs — the amd64 a.out loader with its validation battery (TDD)

- **RED (r9@8c4285a):** the module skeleton with degenerate stubs (parse errs, layout zeroes, seed plants zeroed only) + 10 tests asserting EXACT named-Err strings, segment ranges, and ABI-offset frame reads — 0 passed / 10 failed, every failure on its planned assertion, zero accidental greens.
- **GREEN (r9@822b3ec):** `parse_aout` with checked be32/be64 readers — S_MAGIC 0x8A97 (`_MAGIC(HDR_MAGIC, 26)`), empty-text, entry-in-text, and USTOP checks, every violation a loud named Err before ANY mapping (T-14-04). Entry is the BE u64 at bytes 32..39 (HDR_MAGIC layout).
- **Constants per 6l HEADTYPE 2 + ABI.md section 7:** `HDR_SIZE 40` (HEADR=32+8), `INITTEXT 0x200028` (0x200000+40), `TEXT_MAP_BASE 0x200000` (the header rides the text mapping), `DATA_ALIGN 2 MiB` (INITRND 0x200000 — the A1 rule, NOT aarch64's 64 KiB; real-binary proof at 14-05), `USTACKTOP 0x00007ffffffff000` + `USTKSIZE 16 MiB` (the 9k mem.h donor values, 14-01's recorded policy).
- **`segment_layout`:** install_image's ranges host-testable ahead of 14-04 — text from TEXT_MAP_BASE page-rounded, data/bss byte-exact. The two edges pinned: empty data (data+bss==0 → text-only, empty data/bss ranges, no overlap) and adjacency (text ending exactly at 0x400000 → data_va at that same boundary, zero gap, zero overlap, no stray page).
- **USTOP factored** (`check_data_bss_below_ustop`): u32 a.out fields (~8 GiB max) cannot reach the 128 TiB ceiling, so the donor-carried defense-in-depth is pinned at the exact-boundary arithmetic of the function parse_aout calls, plus a maximal-u32-header-stays-legal proof.
- **Ureg seeding:** `dat.rs::Ureg::zeroed()` + `seed_exec_entry` (9k sysexecregs: bp=Tos via RARG=D_BP — the aarch64 x0 analog, sp, ip, type=64 "fiction for acid"; plus touser's entry state: flags=If, cs/ss/segment quartet = the flat user selectors) and `exec.rs::seed_exec_frame` placing the frame at kstack_top−184. The test reads every planted field back at the ABI.md byte offsets (bp@48, type@128, ip@144, cs@152, flags@160, sp@168, ss@176, quartet@120..126) — the fields 14-02's touser/syscallreturn consume. `trap.rs`'s witnessed `USER_TEXT_SEL`/`USER_DATA_SEL` consts became `pub(crate)` (values unchanged, const asserts intact).
- **Module doc** carries the eager-vs-demand policy (data+bss eager — the cold-syscall-argument fix; text/stack/tos demand) and the exec_into/exec_replace split in the aarch64 terms, with the mapping-carrying bodies explicitly assigned to 14-04's proc port (this plan is the loader half, per the objective).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Verify-command re-key] `cargo test -p x86_64 usermem` / bare `cargo xtask check` cannot run as written (same class as 14-01/14-02's re-keys)**
- **Found during:** Task 1 verify
- **Issue:** the x86_64 crate's default target is `x86_64-unknown-none` (host-target test compile fails on no_std mismatches); this host has no bare cargo outside the nix devshell; boot-test needs `R9_HARVEY_DIR`.
- **Fix:** ran `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` inside `nix develop --command sh -c '...'`, check/clippy the same way, `R9_HARVEY_DIR=/home/req/code/harvey` exported for every boot-test. Same commands, same exit semantics.
- **Files modified:** none (execution-only re-key)
- **Commit:** n/a

**2. [Rule 1 - Bug] the USTOP-overflow behavior test is unprovable through parse_aout as written**
- **Found during:** Task 2 GREEN
- **Issue:** the plan's test assumed a synthetic header can push data_va+data+bss past USTOP — but u32 a.out fields cap segments at ~8 GiB while USTOP (USTACKTOP−16 MiB) sits at ~140 TiB: a 10,000x gap. The plan-as-written test (data=bss=0x7f00_0000) passed validation instead of erroring.
- **Fix:** factored the check into `check_data_bss_below_ustop(data_va, data, bss)` (parse_aout calls it unchanged — the donor check carried verbatim inside) and the test drives that function at the exact boundary (sum==USTOP rejected, one byte below accepted, u64-scale overflow rejected) plus proves a maximal u32-field header stays legal everywhere on the map.
- **Files modified:** ../r9/x86_64/src/exec.rs
- **Commit:** r9@822b3ec

**3. [Rule 2 - Missing-critical] the Ureg seed needed access the module boundaries did not allow**
- **Found during:** Task 2 implementation
- **Issue:** the seeding plants cs/ss/flags/sp/ds/es/fs/gs/bp/type_ — all private to dat.rs's Ureg — and the selector values live in trap.rs's private consts.
- **Fix:** `Ureg::zeroed()` + `seed_exec_entry` added to dat.rs (the frozen frame's own module — the aarch64 TrapFrame::seed_el0_entry precedent), selectors made `pub(crate)` in trap.rs with their const asserts unchanged (computed from GdtData, never hand-pinned).
- **Files modified:** ../r9/x86_64/src/dat.rs, ../r9/x86_64/src/trap.rs
- **Commit:** r9@822b3ec

**Total deviations:** 3 auto-fixed (1 verify-environment re-key, 1 test-arithmetic correction, 1 access-shape completion). **Impact:** none on scope or guarantees — the USTOP mitigation is stronger (boundary pinned at the function itself), and the seed is witnessed against the ABI table rather than field names.

## Authentication Gates

None.

## Known Stubs

None — no stubs, placeholders, or unwired data paths were introduced. The exec_into/exec_replace mapping bodies (install_image/map_data_bss_eager/seed_user_pages, read_aout's backing check) are plan-declared 14-04 scope ("the first half of the exec substrate" per this plan's objective), documented as such in exec.rs's module doc — not stubs left behind.

## Issues Encountered

None beyond the documented deviations.

## TDD Gate Compliance

Task 2 carried `tdd="true"` and was executed as a full RED→GREEN cycle (plan `type: execute`, `workflow.tdd_mode` false, so the classifier gate was not armed — evidence recorded here and in the commit messages):
- **RED** r9@8c4285a: 10 tests against degenerate stubs — `test result: FAILED. 0 passed; 10 failed`, every failure on its planned assertion (exact named-Err string mismatches, zero layouts, zero seed fields); the earlier compile error in the harness (AoutHeader lacking PartialEq) was fixed before the recorded RED run, which compiled clean.
- **GREEN** r9@822b3ec: `10 passed; 0 failed` — same tests, implementation landed.
- **REFACTOR:** assessed; no cleanup warranted (no duplication introduced; the IF const duplication between dat.rs and exec.rs is two locally-documented one-liners) — no refactor commit.

## Next Plan Readiness

Ready for 14-04 — the proc/syscall port has its validated-access choke point (UserAddressSpace to implement), its loader (parse_aout/segment_layout/seed_exec_frame to consume from install_image/exec_into/exec_replace), and the 184-byte frame seed ready for the scheduler's kstack placement.

## Self-Check: PASSED

- r9 files exist: x86_64/src/{usermem.rs, exec.rs} created; {main.rs, dat.rs, trap.rs} modified in r9@822b3ec ✓
- commits are ancestors of r9 HEAD: 393f4f0 ✓, 8c4285a ✓, 822b3ec ✓
- all task acceptance-criteria greps re-run PASS (logged in coverage)
- plan-level verification green: 44 x86_64 host-bin tests (26 prior + 8 usermem + 10 exec), port lib 101, check/clippy exit 0 (A/B-equal warnings), q35 boot-test PASS
