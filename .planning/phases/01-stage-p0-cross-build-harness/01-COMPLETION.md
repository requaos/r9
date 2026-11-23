# Phase 01 — Completion Record

**Completed:** 2026-11-23
**Plans:** 34/34
**Verification:** `passed` — 7/7 must-haves, HARN-01…08 all satisfied
**Fingerprint:** `v4:sha256:7cbd5c0778637899f477de9cd14183a4e341961fa3a6d0cbfced8ebcd8d2a8db` (179 covered files)
**Security:** `threats_open: 0` (82/82 closed)
**Nyquist:** `validated`, `nyquist_compliant: true`, 0 requirement-level gaps
**Code review:** 1 Critical (CR-01) fixed in `b36b633` + regression test, 12 Warnings / 13 Info recorded `open` by owner

## phase.complete file-reference warnings

`gsd_run query phase.complete 01` returned 30 SUMMARY warnings of the form
"references N file(s) not on disk". They are scanner false positives, not missing
deliverables:

1. **Phase-relative paths.** Paths such as `witness/WITNESS-LOG.md`,
   `harvey-flip/HARVEY-FLIP.md`, `witness/evidence/stop-e/...` and
   `rio-derisk/...` exist under
   `.planning/phases/01-stage-p0-cross-build-harness/`, not at the repo root the
   scanner resolves against.
2. **Templates, globs and commands.** Values such as
   `target/images/<arch>/<name>.paq`, `xtask/manifests/<objtype>/<image>.toml`,
   `build/x86_64/{svctcpecho,authprobe}.c` and `grep -c '#\[test\]' …` are prose
   patterns or shell commands, not file claims.
3. **Intentionally removed/superseded spike paths.** `spikes/...`,
   `xtask/src/probe.rs` and `nix/setup.sh` were removed by the Harvey flip or the
   probe teardown (T-01-66); their evidence remains in git history and in the
   committed witness returns.

No warning corresponds to a gap in a phase deliverable; the verifier inspected the
actual resolver, image engine, gate, flake supply and owner evidence and returned
`passed`.

## Residual risks (accepted)

- Hosted `quick-gate-tcg` is red; it is the best-effort second witness (D-13). The
  owner's native amd64 TCG gate passed at stop C3.
- HARN-04's KVM swap/flip-back proof is stop B3 and predates the additive 01-34
  `gate.rs` change (`run_quick`/`serial-check` untouched).
- No bump-time aarch64 quick-pass log; 01-30's Mac log plus the Nyquist live re-run
  cover it.
- The 12 owner-open review warnings (notably WR-01 `ureg-check` partial-tree exit 0,
  WR-03 early-path tree proof, WR-05 `target/` authdomain cleanup, WR-09 resolver
  divergence under manual override) are follow-ups, not verification blockers.
