---
phase: 16-host-net-modes-networking-exit-gate
fixed_at: 2026-11-11T21:34:59Z
review_path: .planning/phases/16-host-net-modes-networking-exit-gate/16-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 16: Code Review Fix Report

**Fixed at:** 2026-11-11T21:34:59Z
**Source review:** .planning/phases/16-host-net-modes-networking-exit-gate/16-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (fix_scope: critical_warning — the 3 WR- findings; 0 CR-)
- Fixed: 3
- Skipped: 0

**Out of scope:** the 4 IN- findings (IN-01..IN-04) stay open in the triage
ledger per project convention (left for phase-17 triage); no IN- fix was
applied.

**Verification venue (#2825 note):** `workflow.use_worktrees` is `false` —
all edits, gates, and commits ran in the MAIN checkouts of both repos
(/home/req/code/harvey on master; /home/req/code/r9 on main). The numbers
below are reproducible directly from the trees as committed.

## Fixed Issues

### WR-01: Unpinned `actions/checkout@v4` in the boot-smoke job

**Files modified:** `.github/workflows/r9-shell.yml`
**Commit:** 67828a9ad (harvey)
**Applied fix:** Pinned the boot-smoke job's checkout to
`actions/checkout@11d5960a326750d5838078e36cf38b85af677262` — the exact
API-resolved SHA the net-smoke and net-smoke-x64 jobs already pin (reused,
not a fresh resolution). Also updated the now-stale comment sentence that
documented the boot-smoke `@v4` deviation as "stays as-is"; all three jobs
now ride the same resolved SHA.
**Verification:** `nix run nixpkgs#actionlint -- .github/workflows/r9-shell.yml`
— clean pass.

### WR-02: `r9-netdev-args` does not enforce its own no-spaces token contract for caller-supplied `--pcap`/`--config` values

**Files modified:** `../r9/xtask/src/bin/r9-netdev-args.rs` (r9 repo);
`flake.lock` (harvey, pin advance)
**Commits:** 208ceba (r9, pushed to origin/main — push-before-pin);
4d19a9e75 (harvey, lock-only: `chore(16): advance the r9 pin to
208ceba775577cbae194f5a47dab61930abee743 — the WR-02 emitter fix`)
**Applied fix:** Added a `token_safe()` predicate (non-empty and
whitespace-free) and a fail-loud CLI-boundary check in `main()` that
rejects spaced/empty `--pcap` values via the existing `usage_exit` path
before any argv exists, with a comment explaining why `--config` needs no
such check (read from disk, never interpolated into a token). Pinned the
contract with a bin-local test
(`pcap_token_safety_rejects_empty_and_whitespace`: clean paths pass;
empty/space/tab/newline values rejected).
**Verification (all in /home/req/code/r9):**
- Live behavior probe: `--pcap "/tmp/my dir/x.pcap"` and `--pcap ""` →
  exit 1 with the usage line + specific grievance; `--pcap /tmp/clean.pcap`
  → exit 0 with the normal slirp argv incl. the filter-dump token.
- `nix develop -c cargo test -p xtask` — all pass (35 lib + 1 new bin +
  73 main + 3), exit 0.
- `nix develop -c cargo xtask check` — exit 0 (pre-existing warnings in
  port tests only).
- `cargo fmt --check`: my added lines are fmt-clean; remaining diffs in
  the bin and elsewhere are pre-existing nightly-rustfmt drift on
  untouched lines (the xtask check gate runs no fmt step).
- Full-workspace `cargo test` fails compiling the `riscv64` crate's TEST
  target (19 errors, e.g. `TryInto` not in scope in `src/sbi.rs`) —
  **pre-existing**: reproduced identically on the unmodified tree via
  `git stash` (exit 101 before my change). Not caused by this fix.
- Pin advance: `nix flake lock --update-input r9` moved d1c4849… →
  208ceba775577cbae194f5a47dab61930abee743 (narHash updated); both
  affected harvey apps still eval: `nix eval .#apps.x86_64-linux.boot-r9-x64.type`
  and `.#apps.x86_64-linux.boot-r9-net-smoke-x64.type` → `"app"`.

### WR-03: `boot-r9-net-smoke-x64` app lacks `procps` for its `pkill` survivor sweep

**Files modified:** `flake.nix`
**Commit:** 21b499683 (harvey)
**Applied fix:** Added `pkgs.procps` to `netSmokeR9X64Pkg`'s runtimeInputs
(after `iproute2`), exactly the way `netSoakR9X64Pkg` declares it, and
extended the app's comment to record the rationale (the EXIT trap's pkill
survivor sweep; `2>/dev/null || true' must mask a failed sweep, never a
missing binary).
**Verification:** `nix eval .#apps.x86_64-linux.boot-r9-net-smoke-x64.type`
→ `"app"` (evaluates clean with the new dep; the wrapper is exposed only
as an app, and writeShellApplication's runtimeInputs→PATH wiring is the
same mechanism the soak app already proves). Re-ran after the WR-03
commit alongside the WR-02 evals — both apps eval.

## Skipped Issues

None — all in-scope findings were fixed.

---

_Fixed: 2026-11-11T21:34:59Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
