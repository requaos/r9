---
phase: 14-harvey-amd64-userspace-on-r9
fixed_at: 2026-11-09T17:38:16Z
review_path: .planning/phases/14-harvey-amd64-userspace-on-r9/14-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 2
skipped: 1
status: partial
---

# Phase 14: Code Review Fix Report

**Fixed at:** 2026-11-09T17:38:16Z
**Source review:** .planning/phases/14-harvey-amd64-userspace-on-r9/14-REVIEW.md
**Iteration:** 1 (Info-remainder run)

**Summary:**
- Findings in scope: 3 (fix_scope `all`, remainder after the prior
  critical_warning run): IN-01, IN-02, IN-03
- Fixed: 2
- Skipped: 1
- Already handled by the prior run and recorded in the disposition
  ledger (14-REVIEW-DISPOSITION.md): CR-01, WR-01, WR-02, WR-03 — not
  re-attempted here; their commits are r9 `f13ffda`/`aa72df2`/`86a8677`,
  harvey `ea0604aeb`, FOD re-wire r9 `5a563dd`, pin advance
  `f6783ec89`.

Dual-repo dispositions for THIS run, each commit atomic in the repo
that owns the file (`workflow.use_worktrees=false` — all edits and
commits landed directly in the main checkouts; no worktree was
created):

| Finding | File | Repo | Commit |
|---------|------|------|--------|
| IN-01 | `../r9/x86_64/src/proc.rs` | r9 | `37eb1eb` |
| IN-02 | `../r9/x86_64/src/trap.rs` | r9 | `2679bb7` |
| IN-03 | `../r9/x86_64/src/exec.rs` | r9 | — (skipped, no edit) |
| (close) | `../r9/flake.nix` FOD re-wire | r9 | `e560405` |
| (close) | `flake.lock` pin advance | harvey | `c811bb83f` |

Close discipline (IN-01/IN-02 are kernel-byte commits, so the standing
drift-class rule applied — re-wire-then-pin, all of it executed):

- FOD re-wire: r9 `e560405` (two-derivation proof, below)
- r9 pushed push-before-pin: `5a563dd..e560405 main -> main`, 0
  unpushed after
- Harvey pin advance: `c811bb83f` — lock-only (flake.lock 3+/3-), pins
  `github:requaos/r9` at `e560405240626d4f9e54b14cecf5c03862d2e28f`;
  sanity `nix eval .#apps.x86_64-linux.boot-r9-x64.type` = `"app"`

## Fixed Issues

### IN-01: `sched: park` println on every preemption — debug artifact in the scheduler hot path

**Status:** fixed
**File:** `../r9/x86_64/src/proc.rs:1686`
**Files modified:** `../r9/x86_64/src/proc.rs`
**Commit:** r9 `37eb1eb`
**Applied fix:** Deleted the `println!("sched: park {cur_pid}")` from
`reschedule()` rather than cfg-gating it. The scope note preferred the
tick-print cfg gate IF the print had diagnostic value under a debug
gate — it has none: both `preempt()` call sites (timer.rs:224,
syscall.rs:1418) are `#[cfg(dev_exec)]`, so `reschedule()` is
unreachable outside dev_exec and a tick-print-shaped
`#[cfg(not(dev_exec))]` gate would be compile-time-live but
runtime-dead in every profile — the exact "witness that certifies
nothing" anti-pattern WR-01 just fixed. The 14-06 park-and-swtch
design is proven (both boot gates green), the print's bring-up role is
preserved in the reschedule() doc block and git history, and a pre-edit
grep confirmed nothing (expect scripts, gates, docs) keys on the
`sched: park` string. Post-fix proof: 0 occurrences in both boot-gate
serial captures. `cur_pid` remains used by the `position()` closure —
no unused-variable fallout.

### IN-02: Dead code: `touser` and `UTZERO` are never called, and their "unused until 14-05" comments are now false

**Status:** fixed
**File:** `../r9/x86_64/src/trap.rs:43-45, 480`
**Files modified:** `../r9/x86_64/src/trap.rs`
**Commit:** r9 `2679bb7`
**Applied fix:** Deleted `UTZERO` (const + doc comment +
`#[allow(dead_code)]`) and `touser` (the naked SYSRETQ fn, its doc
comment, and the now-false "Unused until 14-05's exec_bootstrap"
comment). Reference check BEFORE deleting (per the scope note):
crate-wide grep found zero call sites — only comments; the `UTZERO+0x28`
VALUE lives on in `exec::INITTEXT` (`0x200000 + HDR_SIZE`) /
`TEXT_MAP_BASE` (`0x200000`); `USER_DATA_SEL` stays used by
`seed_exec_entry` and the selector const-asserts; kernel.ld merely
collects `*(.trap*)` with no size/count assertion on the section, so
shrinking it is inert. Donor-history mentions of "touser" elsewhere
(dat.rs, exec.rs shape comments, ABI.md:232 "Return to user / first
entry") reference the 9k/donor concept documented in ABI.md, not the
deleted r9 fn — left untouched. Verified: `grep touser|UTZERO
trap.rs` → zero hits; `cargo build -p x86_64` green (full codegen +
link, since the `.trap` section contents changed).

## Skipped Issues

### IN-03: Impossible-state guard — check_data_bss_below_ustop cannot fire for any parseable header

**File:** `../r9/x86_64/src/exec.rs:525-536`
**Reason:** necessity finding — honestly-documented defense-in-depth,
recorded per the necessity rubric, not auto-removable. The review
itself judged it "removal is a scope decision, not a defect fix". The
impossibility is real but conditional on today's u32-derived a.out
size fields (parse_aout reads be32 → u64, exec.rs:517-519): the guard
exists against a future 64-bit a.out format or a moved stack policy
(its own comment, exec.rs:540-545), so it cannot be proven to guard
nothing — the type-width premise could change under it. Its boundary
arithmetic is host-pinned independently of reachability
(check_data_bss_below_ustop's exact USTOP boundary is unit-tested at
exec.rs:669-677), and the same ceiling is re-proven by sys_brk_'s
independent `addr > exec::USTOP` check (syscall.rs:801-803). Removing
the parse-path call would leave the host-pinned test testing a
function nothing calls — worse than the documented status quo. Left
as is; the comment already discloses the impossibility honestly.
**Original issue:** The USTOP ceiling check compares
`data_va + data + bss >= USTOP` where data/bss are u32-derived u64s
and USTOP sits ~128 TiB up the user map, so the check can never fire
for any header that passed parse_aout.

## Verification

All gates ran in the MAIN r9 checkout (/home/req/code/r9) at the fixed
tree (source state 2679bb7; identical to the pinned e560405 for
everything the battery consumes — e560405 adds only the flake.nix FOD
comment/hash, whose green build is itself verified below). Reproducible
from /home/req/code/r9@e560405 (= the Harvey-pinned rev) and
/home/req/code/harvey@c811bb83f. No worktree was created
(workflow.use_worktrees=false), so nothing here depends on a torn-down
tree.

Regression battery (all green, exit 0):

- `nix develop . -c cargo xtask test` — 101 passed; 0 failed
- `R9_HARVEY_DIR=/home/req/code/harvey nix develop . -c cargo xtask
  ureg-check` — 35 fields match aarch64/include/ureg.h; 26 fields
  match amd64/include/ureg.h; negative control fired (cs -> cseg
  rejected with a named-field diff)
- `R9_HARVEY_DIR=… nix develop . -c cargo xtask boot-test --config
  q35exec` — PASS: banner, memory facts, discovery, module-tag/paqfs
  mount, exec-entry note, marker binary's ring-3 prints (IN-02's
  trap.rs deletions sit on the entry/return paths this gate exercises)
- `R9_HARVEY_DIR=… nix develop . -c cargo xtask boot-test --config
  q35shell` — PASS: the real boot(8) -> init -> rc chain through to
  the interactive `% ` prompt (IN-01's hot path — the park noise this
  finding cites is the composed chain's; post-fix captures show 0
  `sched: park` lines in BOTH gate logs)

boot-iso FOD re-wire (the drift-class rule; procedure per
evidence/14-gap-fod-proof.md and the prior run r9@5a563dd):

1. Mismatch captured at the fixed tree (r9@2679bb7): specified
   `sha256-PLfyY3v48QruonYK+TJ7J+Ci6oVaCF88wkzhFF2SVzU=` (stale at the
   5a563dd pin), got
   `sha256-rac4MQxHuSNvSPHGkRKeTS+qZglazBQLot740djuet4=`
2. Two independent non-FOD realizations via `f.lib.bootIso`, each
   forcing a FRESH kernel build through never-before-used version
   overrides (`reviewfix2-proof-a` / `reviewfix2-proof-b`; both logs
   show `building …-r9-kernel-x64-reviewfix2-proof-{a,b}.drv` — no
   cache hits)
3. Byte-identity at both layers: `cmp` clean; file sha256 both
   `b0435dd294c6face846032c3535ea389a2296146fdbefed095278d9b62044a73`;
   `nix hash path` both outputs =
   `sha256-rac4MQxHuSNvSPHGkRKeTS+qZglazBQLot740djuet4=` (base16
   `ada738310c47b9236f48f1c691129e4d2faa66095acc140ba2def8d1d8ee7ade`,
   round-trip-verified via `nix hash convert --to sri`) — three-way
   agreement with the FOD's own measured got-value
4. Wired in r9 flake.nix with the full-provenance comment (14-07
   style); `nix build .#boot-iso` green and `--rebuild` green at the
   wired value

Per-finding verification tiers: Tier 1 (re-read) for every fix; Tier 2
`nix develop . -c cargo check -p x86_64` for IN-01 and
`nix develop . -c cargo build -p x86_64` for IN-02 (`cargo check`
skips `global_asm!`/naked-fn codegen, so the assembler and linker ran).
All remaining warnings are the pre-existing set (port/, cpu.rs,
dat.rs, trap.rs spl*/IntrStatus dead code — trap.rs's warning count
varies with the xtask cfg profile, not with these fixes); the fixes
introduced none.

Both repos end clean (tracked): r9 clean and pushed
(`5a563dd..e560405`, 0 unpushed); harvey clean after the pin commit —
this REVIEW-FIX.md is deliberately left uncommitted for the
orchestrator, and the pre-existing untracked
`.planning/research/.cache/*.json` files were not touched.

---

_Fixed: 2026-11-09T17:38:16Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
