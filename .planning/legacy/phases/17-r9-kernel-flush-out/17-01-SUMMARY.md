---
phase: 17-r9-kernel-flush-out
plan: 01
subsystem: kernel
tags: [r9, x86_64, rfmem, oom-release, semacquire, devip, closure-ledger, flake-pin]

requires:
  - phase: 08-r9-tcp-ip-path-devip-compatible-net
    provides: the 12 shared-code devip/RFMEM fixes + their disposition ledger (the carry-over subject)
  - phase: 16-host-net-modes-networking-exit-gate
    provides: the composed x64 gate family (net-smoke-x64) and the pinned r9 base 208ceba
provides:
  - 17-FLUSHOUT.md seeded — the phase's single 30-row closure index (D-16)
  - the 08 ledger re-cited: 13 x86_64-keyed Source cells (12 already-fixed rows + CR-03's transferred pins)
  - the D-02 RFMEM/OOM carry-over audit — every alloc-then-early-return path in x86_64/src/{proc,exec}.rs enumerated, 4 gaps ported (r9 3e8bcec)
  - CR-03's four deterministic x86_64 sem pins (r9 f9088d0) — the Phase-8 requires-human-verification host half discharged
  - the r9 pin lineage advanced 208ceba -> 476605d (push-before-pin, FOD re-wired with the two-derivation proof)
affects: [17-02 (leak gates run against the audited+ported tree), 17-07 (CR-07 folds #47/#50/#66), 17-04 (07-WR class sweep), VERIFICATION (walks 17-FLUSHOUT.md)]

actuals:
  tokens: 6937
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns: ["the closure pipeline shape: host battery -> composed x64 gate -> ledger Source re-cite -> FLUSHOUT row (D-01/D-13/D-16)", "the OOM-release record-or-unwind doctrine applied to x86_64-local map machinery (walk-failure self-unwind inside pml4_map_user)"]

key-files:
  created: [.planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md]
  modified: [.planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW-DISPOSITION.md, ../r9/x86_64/src/proc.rs, ../r9/flake.nix, flake.lock]

key-decisions:
  - "The D-02 audit verdict: the four research anchors ARE carried verbatim, but four x86_64-LOCAL OOM paths leaked against the family's own hygiene — ported as one class commit (r9 3e8bcec), not swept under 'carried'"
  - "The brk per-page alloc-failure `?` arm is family-verbatim on BOTH arches (early-return without the done-unwind; the done pages stay recorded+refcounted so member exits free them) — marked carried, not 'fixed' on x86_64 alone (nothing may diverge from or flow back into frozen aarch64)"
  - "CR-03's Phase-8 requires-human-verification obligation: the host half is discharged by the four deterministic sem pins; the in-kernel contention leg rides 17-02's churn profile — the transfer recorded in both the 08 ledger and FLUSHOUT"

patterns-established:
  - "Walk-failure self-unwind: a page-table walk that allocates intermediates frees (and unlinks from the parent entry) everything it created on the failing path — the map path's own doc contract, now enforceable"

requirements-completed: [FLUSH-01, FLUSH-02]

coverage:
  - id: D1
    description: "CR-01 exemplar closed end-to-end through the closure pipeline (battery -> gate -> ledger re-cite -> FLUSHOUT row)"
    requirement: FLUSH-01
    verification:
      - kind: unit
        ref: "cargo test -p port --lib (101 passed / 0 failed, run twice + tracer re-run)"
        status: pass
      - kind: integration
        ref: "nix run .#boot-r9-net-smoke-x64 (exit 0, ALL STAGES PASS, run at 208ceba pre-pin and 476605d post-pin)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The D-02 OOM-release audit: every alloc-then-early-return path in x86_64/src/{proc,exec}.rs enumerated (24 rows below), 4 gaps ported as one r9 class commit"
    requirement: FLUSH-02
    verification:
      - kind: unit
        ref: "cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu (64 passed / 0 failed at r9 476605d)"
        status: pass
      - kind: integration
        ref: "cargo xtask check + clippy exit 0; nix build .#boot-iso + --rebuild green at the wired hash"
        status: pass
    human_judgment: false
  - id: D3
    description: "CR-03's four deterministic sem pins (the Phase-8 verification transfer, D-07 host half)"
    requirement: FLUSH-01
    verification:
      - kind: unit
        ref: "cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu -- sem_ (5 passed / 0 failed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The 12 already-fixed 08-ledger rows re-cited with x86_64-keyed pins + the 30-row FLUSHOUT seed"
    requirement: FLUSH-01
    verification:
      - kind: manual_procedural
        ref: "grep -c x86_64 08-REVIEW-DISPOSITION.md = 13 (>= 12, baseline 0); grep -c '^| ' 17-FLUSHOUT.md = 31 (header + 30 rows); all six probed rows present"
        status: pass
    human_judgment: false
  - id: D5
    description: "WR-03's leak-gate verification (17-02's tiered gates) — deliberately NOT closed by this plan; the ledger row and FLUSHOUT row both name the still-open gates"
    verification: []
    human_judgment: true
    rationale: "The gates have not run (17-02 owns them); no automation in this plan can prove bounded memory under churn — recording the pending obligation honestly rather than claiming closure"

# Metrics
duration: 38 min
completed: 2026-11-12
status: complete
---

# Phase 17 Plan 01: r9 Kernel Flush-Out — Verification Wave Summary

**CR-01 wired end-to-end through the closure pipeline, the D-02 OOM-release audit completed with four x86_64-local gaps ported (push-before-pin, FOD re-wired at 476605d), CR-03's four deterministic sem pins landed, and all 12 already-fixed 08-ledger rows re-cited into the seeded 30-row FLUSHOUT index.**

## Performance

- **Duration:** 38 min
- **Started:** 2026-11-12T16:52:57Z
- **Completed:** 2026-11-12T17:31:26Z
- **Tasks:** 3/3
- **Files modified:** 5 (Harvey 3 + r9 2)

## Accomplishments

- The CR-01 exemplar proven through every closure layer: port battery 101/101 -> composed net gate exit 0 -> 08-ledger Source re-cite -> the FLUSHOUT row (the shape every later closure row reuses)
- The D-02 audit remainder: EVERY allocation-then-early-return path in x86_64/src/{proc,exec}.rs enumerated (24 rows, zero sampled); four genuine leaks ported as one r9 class commit; push-before-pin held; the boot-iso FOD re-wired with the two-derivation byte-identity proof
- CR-03's Phase-8 requires-human-verification transfer discharged on the host half: `sem_register_is_idempotent`, `sem_wake_consumes_picked_registrations_in_order`, `sem_reregistered_waiter_is_found_by_the_next_release`, `sem_wake_never_touches_other_words` (the aarch64 4-pin shape re-keyed; `sem_` filter = 5 passing)
- All 12 already-fixed 08-ledger rows carry x86_64-keyed Source citations (13 `x86_64` mentions, baseline 0); CR-07 stays skipped; WR-03's row names 17-02's gates as still-open — no row claims its gates ran
- 17-FLUSHOUT.md seeded with exactly 30 rows + header (08-CR-01..07, 08-WR-01..06, 07-WR-01..08, WINDOWS #18/#44/#45/#46/#47/#49/#50/#65/#66), pending cells literal, 07-WR-07/08 noting dispositioned-in-17-04

## The D-02 Audit Table (every alloc-then-early-return path, zero sampled rows)

Sites in `x86_64/src/proc.rs` (18) and `x86_64/src/exec.rs` (6). "carried" = the aarch64 WR-03 fix-family hygiene (or the module's own established prefix idiom) verbatim at the site, quote included; "ported" = a gap closed by r9 commit `3e8bcec`.

### x86_64/src/proc.rs

| # | Site (post-fix lines) | The alloc-then-early-return path | Verdict |
|---|------------------------|----------------------------------|---------|
| 1 | `pml4_map_user` walk | intermediate alloc Err dropped the created-so-far pages: linked into the table, never returned, never recorded — the doc's "a walk failure frees them itself, leaving no leaks" was never true | **ported** — r9 3e8bcec: the failing arm unlinks each created page from the entry this call wrote into its parent and frees it, innermost first |
| 2 | `map_user_page` | `alloc_phys_page()` then `pml4_map_user(...)?` — on walk Err the fresh leaf page leaked (unrecorded) | **ported** — r9 3e8bcec: leaf freed beside the walk's own unwind |
| 3 | `new_user_proc` PML4 alloc | `alloc_phys_page().map_err(...)?` with nothing else yet allocated | carried — `let pml4_pa = alloc_phys_page().map_err(\|_\| "proc: PML4 page alloc failed")?;` |
| 4 | `new_user_proc` kstack-table tuple | `(Ok,Ok,Err)` evaluated all three allocs — the successful prefix leaked | **ported** — r9 3e8bcec: per-page loop with the prefix freed on failure (the kstack pages' own idiom) |
| 5 | `new_user_proc` kstack pages | per-page prefix unwind | carried — `unwind_new_proc(pml4_pa, &kstack_pas[..i], &kstack_table_pas, "kstack-pages");` |
| 6 | `new_user_proc` table-full | guard dropped, full-prefix unwind | carried — `drop(table); unwind_new_proc(pml4_pa, &kstack_pas, &kstack_table_pas, "table-full");` |
| 7 | `rfmem_brk_set` per-page alloc | early `?` returns BEFORE the completed-pages unwind — verbatim the aarch64 family shape (aarch64 :387 identical `map_err(\|_\| "brk: out of memory")?`); the done pages stay recorded in members' mapping_logs + refcounted, so member exits free them (no permanent leak; the mapped-beyond-break wart is identical on both arches) | carried (family-verbatim) — `let pa = alloc_phys_page().map_err(\|_\| "brk: out of memory")?;` |
| 8 | `rfmem_brk_set` member-map failure | never-published page: unmap from takers, drop records, free | carried (anchor) — `for &m in &mapped { ... pml4_unmap_user(p.pml4_pa, va); ... } free_phys_page(pa); break;` |
| 9 | `rfmem_brk_set` completed-pages unwind | release verdict honored per member | carried (anchor, WR-03 arm 1) — `if release_user_page(done_pa) { free_phys_page(done_pa); }` |
| 10 | `rfork_clone` -> `new_user_proc` Err | new_user_proc unwinds itself (rows 3-6) | carried (delegation) |
| 11 | `rfork_clone` failure pivot | Moribund + release-verdict walk + tables + root/kstack | carried (anchor, WR-03) — `child.state = ProcState::Moribund; for rec in core::mem::take(&mut child.mapping_log) { if release_user_page(rec.phys as u64) { free_phys_page(rec.phys as u64); } }` |
| 12 | `rfork_copy` share-arm map Err | no fresh page (sharing the parent's); propagates to the pivot; the failed call's walk intermediates were row 1's leak — now self-unwound | carried (pivot; row-1 port closes the corner) — `.map_err(\|_\| "rfork: child shared map failed")?` |
| 13 | `rfork_copy` copy-arm page alloc | early `?` with nothing yet allocated in-iteration; prior pages all recorded -> pivot releases | carried — `alloc_phys_page().map_err(\|_\| "rfork: page alloc failed")?` |
| 14 | `rfork_copy` copy-arm child-map Err | the copied page was never recorded — it leaked | **ported** — r9 3e8bcec: `free_phys_page(pa)` before the `return Err("rfork: child user map failed")` (the sweep arms' record-or-unwind hygiene) |
| 15 | `rfork_copy` sweep-page alloc | early `?`; prior sweep pages recorded in BOTH logs + share_new(2) -> pivot releases one count, parent keeps the other | carried — `alloc_phys_page().map_err(\|_\| "rfork: sweep page alloc failed")?` |
| 16 | `rfork_copy` sweep parent-map Err | never-published page freed | carried (anchor, WR-03 arm 2a) — `free_phys_page(pa); return Err("rfork: sweep parent map failed");` |
| 17 | `rfork_copy` sweep child-map Err | parent's UNTRACKED mapping unmounted + page freed | carried (anchor, WR-03 arm 2b) — `pml4_unmap_user(parent.pml4_pa, va); free_phys_page(pa); return Err("rfork: sweep child map failed");` |
| 18 | `page_in_zeroed` | alloc/map failure PANICS (loud death) — not an early-return path; out of the audit's class, noted for completeness | N/A-panic (noted) — `alloc_phys_page().unwrap_or_else(\|_\| panic!("proc: phys page alloc failed"))` |

### x86_64/src/exec.rs

| # | Site | The path | Verdict |
|---|------|----------|---------|
| 19 | `read_aout` / `parse_aout` / `check_seed_fit` | pure validation into stack buffers — no allocation before any early return | N/A (no allocation) |
| 20 | `map_text_eager` per-page `map_user_page ?` | earlier pages are RECORDED (mapping_log) — exec_replace's post-free kill and exits free them through the record walk; the per-call leaf/intermediates were rows 1-2's leak | carried via the record walk (+ rows 1/2 ports close the per-call corner) |
| 21 | `map_data_bss_eager` per-page | same shape as row 20 | carried via the record walk (+ rows 1/2) |
| 22 | `seed_user_pages` stack-seed maps + the two arg-overflow Errs | arg-overflow pre-checked by `check_seed_fit`'s identical arithmetic (the 14-CR-01 fix — post-free arg-shape failure unreachable); post-free OOM kills via exit_tail (recorded pages freed at exit) | carried — `check_seed_fit` mirrors `seed_user_pages`' arithmetic so exec_replace never fails for arg-shape reasons after the free |
| 23 | `exec_replace` ordering | parse -> EVERY validation -> free -> seed; post-free failure kills loudly | carried (the ordering-doctrine anchor) — `Err(msg) => { port::println!("exec: post-free seed failed ({msg}) — killing pid {}", proc.pid); crate::syscall::exit_tail(proc, msg.as_bytes()); }` |
| 24 | `exec_into` install Err | propagates to the bootstrap caller — boot-fatal context (kernel cannot start) | carried-by-caller (bootstrap-fatal era shape) |

**Totals: 24 paths enumerated, 0 sampled; 4 ported (rows 1, 2, 4, 14 — the one class commit r9 3e8bcec), 17 carried with verbatim quotes/anchors, 3 N/A/noted (rows 18, 19).**

## Battery and Gate Counts (recorded)

- `cargo test -p port --lib`: **101 passed / 0 failed** (run at 208ceba twice + the tracer re-run; unchanged at 476605d)
- `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu`: **64 passed / 0 failed** (floor 60 at HEAD + the 4 new sem pins); `-- sem_` filter: **5 passed / 0 failed**; no E0463 (the --target override honored, Pitfall 1)
- `cargo xtask check` + `cargo xtask clippy`: exit 0 (pre-existing warnings unchanged; the one proc.rs clippy note is the pre-existing dangling doc comment at the `round_up_page` boundary)
- `nix run .#boot-r9-net-smoke-x64`: exit 0, ALL STAGES PASS — at 208ceba (pre-pin, twice) AND at 476605d (post-pin)
- `nix run .#boot-r9-smoke-x64`: exit 0, ALL STAGES PASS at 476605d (the composed x64 smoke re-green at the new pin)

## The Pin-Advance Lineage (208ceba -> 476605d)

- r9 `3e8bcec` — fix(17-WR-03): the four OOM-release ports (kernel bytes)
- r9 `f9088d0` — test(17-01): CR-03's four deterministic sem pins (cfg(test) only)
- r9 `476605d` — fix(17-01): the boot-iso FOD re-wire, **two-derivation proof**: wr03fix-proof-a/-b (never-before-used version overrides -> genuinely fresh kernel builds, both logs show fresh `.drv`s) through the non-FOD bootIo realization — byte-identical ISOs (`cmp` clean; file sha256 `1cd4ef4036cfba865d54b9117529b1d3848a90c7852f58b0b9ca53d005d9405b` both); NAR hash `sha256-K60O4HjE0s0esuCNq0wR3+QQB8BrFpUW/Gv3yVAj2hU=` (base16 `2bad0ee078c4d2cd1eb2e08dab4c11dfe41007c06b169516fc6bf7c95023da15`) three-way-agreed with the FOD's own got-value; `nix build .#boot-iso` + `--rebuild` green at the wired value
- r9 pushed `208ceba..476605d` (origin/main..HEAD = 0) BEFORE the Harvey advance — push-before-pin held
- Harvey `940d08497` — the pin advance as its OWN commit, diff = **flake.lock only** (asserted: `PIN-COMMIT-CLEAN`)

## Task Commits

Each task was committed atomically (Harvey repo; the r9 class/test/FOD commits are Task 2's, listed above):

1. **Task 1: CR-01 exemplar end-to-end (tracer)** - `4da57c296` (docs) — the tracer feedback gate re-ran all three verify legs green post-commit (interactive run, end-of-phase mode, `<automated>`-only verify -> row 3: no checkpoint, continue to expansion)
2. **Task 2: D-02 audit + CR-03 sem pins** - r9 `3e8bcec` (fix) + r9 `f9088d0` (test) + r9 `476605d` (FOD re-wire) + Harvey `940d08497` (pin advance, lock-only)
3. **Task 3: 11 re-cites + 30-row FLUSHOUT seed** - `aa7a4b195` (docs)

## Files Created/Modified

- `.planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md` — created: the D-16 closure table, 30 rows + header, the phase's index
- `.planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW-DISPOSITION.md` — 13 Source cells re-cited with x86_64-keyed pins; the CR-03/WR-03 verification-transfer note in the header
- `../r9/x86_64/src/proc.rs` — the four OOM-release ports + the four sem pins (one file, two commits)
- `../r9/flake.nix` — the boot-iso FOD hash re-wire with the provenance comment
- `flake.lock` — the r9 pin advance (208ceba -> 476605d, lock-only commit)

## Decisions Made

- The audit's bar: a path "carried" only where the aarch64 family hygiene (or the module's own established prefix idiom) is verbatim at the site; x86_64-local machinery promising more than aarch64 (the tracked `mapping_tables` model, the "no leaks" walk doc) had to make good on its own contract — hence the four ports
- The brk `?` arm (row 7) deliberately NOT "improved" on x86_64 alone: the shape is family-verbatim, the done pages are refcounted+recorded (no permanent leak), and diverging from frozen aarch64 is out of bounds for a carry-over audit
- The new sem pins use word PAs distinct from the existing pin's (0x96..-0x9a.. vs 0x91../0x95..) — SEM_WAITERS is one shared static and cargo runs the bin's tests in parallel; the isolation note is in the test comment

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Root-owned stale build artifacts blocked the stage build**
- **Found during:** Task 1 (the first `nix run .#boot-r9-net-smoke-x64`)
- **Issue:** `ar: cannot create /amd64/lib/libc.a: Permission denied` — 3606 files in the checkout (amd64/lib, amd64/bin, aarch64/bin, sys/src .6 droppings) were owned by real root from an earlier root-run/restored build. The stage build's `unshare -Ur -m` venue maps the invoking uid only; root-owned files read as `nobody` and every write to them fails
- **Fix:** Removed the 3603 untracked root-owned artifacts (the idempotent stage/tier build regenerates them req-owned; verified: the gate then built and booted green). Left in place: the 3 root-owned git-TRACKED sources (`sys/src/cmd/eqn/eqn.c` — also pre-existing-modified in the working tree, `sys/src/cmd/jpg/{ycbcr,rgbv}.h` — reads don't block builds) and 25 files under the 3 root-owned dirs (`amd64/bin/git`, `acme/{bin/aarch64,lib}` — not on any x64 build path)
- **Files modified:** none (untracked build artifacts only; `git status` unchanged)
- **Verification:** the stage build + both composed gates green afterward
- **Committed in:** n/a (environment hygiene, no repo change)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Environment-only; no scope creep. The eqn.c working-tree modification is pre-existing (not mine, untouched).

## Issues Encountered

- The first net-smoke run failed at the libc stage build (the root-owned artifacts above) — resolved as the Rule 3 fix; the gate then passed and was re-run green at both pins
- `nix run .#boot-r9-x64` was mistakenly run once as the "gate" — it is the INTERACTIVE boot app (boots to `% ` and stays; no expect driver). Recognized from the process tree (no expect, qemu parented to the script), killed, and replaced with the correct `boot-r9-smoke-x64` gate. Side observation recorded: the interactive boot DID reach the `% ` prompt on the new kernel — the WR-03-fix kernel boots the full chain
- None of these touched repo state; all gates named in the plan ran green

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 17-02 can build its tiered leak gates directly on the audited+ported tree at pin 476605d: the D-02 audit is complete (its precondition, per D-02), the four ports are in, and the WR-03/CR-03 rows already name its gates
- 17-07 owns CR-07 (+#47/#50/#66 fold); the FLUSHOUT row and the 08-ledger row both read pending/skipped awaiting it
- The tracer chain (battery -> gate -> re-cite -> row) is proven; later plans replicate it per row
- No blockers. Residual environment note: 3 root-owned tracked sources + 3 root-owned dirs remain in the checkout (harmless to builds; a `sudo chown` hygiene pass is optional and out of scope)

## Self-Check: PASSED

- Created/modified files exist on disk: 17-FLUSHOUT.md, 08-REVIEW-DISPOSITION.md (Harvey); ../r9/x86_64/src/proc.rs, ../r9/flake.nix (r9); flake.lock
- Commits verified ancestors of HEAD: 4da57c296, 940d08497, aa7a4b195 (Harvey, measured count 3 = frontmatter `commits`); r9 3e8bcec/f9088d0/476605d on origin/main
- Plan-level verification re-run post-commit: battery 101/101, sem_ 5/5, full x86_64 64/64, net-smoke exit 0 at the pin, 13 re-cites, 31 FLUSHOUT lines, all probed rows present

---
*Phase: 17-r9-kernel-flush-out*
*Completed: 2026-11-12*
