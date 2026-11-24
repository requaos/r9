# Phase 2 end-of-phase owner stop — results (round 1)

Plan 02-06, Task 2. The owner took the branch route: the return arrived as commit
`a0454ff` on `phase-02/design-lock`. Every verdict below is decided only from a line in that
return (the 21 files under `evidence/owner-stop/return/`); no step was run on the Mac.

- branch: `phase-02/design-lock`
- branch-tip: `a0454ffed4f80473866239c4164a1918bf0d216c`
- hash-commit: `4624ea2f0cb45c81870dcc881d544d7d36a2e0bf` — message `boot-iso hash bump`,
  `flake.nix` only, one line changed (the `outputHash` value)
- return-commit: `a0454ffed4f80473866239c4164a1918bf0d216c` — message `witness evidence`,
  files only under `evidence/owner-stop/return/`
- host (return/env.txt): `Linux teeniebox 7.4.0-rc3 ... x86_64`, 16 cores, `/dev/kvm`
  writable, Nix 2.35.6
- returned steps (return/env.txt): `steps=full` — step 7 only. No host-tests, boot-iso,
  kernel-step, gate-kvm or gate-tcg log was returned.
- result: **8 of the 9 verdicts FAIL**. Task 3 does not start and `main` is not pushed.

gated-commit 4624ea2f0cb45c81870dcc881d544d7d36a2e0bf

kernel-x64-elf-sha256 (Linux): not returned — the only `kernel-x64.elf` sha256 in the return
is `2377dc2fb599a838425659ce7fe52f118ee4bfc9ac8f8a877091fab0cb111018` for
`r9-kernel-x64-af46615` (pre-Phase-2 code), not the gated commit's kernel.

## Verdicts

VERDICT stop-branch-push: PASS
EVIDENCE stop-branch-push: `git ls-remote origin refs/heads/phase-02/design-lock` ->
`a0454ffed4f80473866239c4164a1918bf0d216c`; `git merge-base 2b415882aca30e3965ae3fb9612e91d05081879d
origin/phase-02/design-lock` is `2b415882aca30e3965ae3fb9612e91d05081879d`, so the branch base is
the last Mac commit, and the only two commits on top are the hash commit and the return commit.

VERDICT stop-bootiso-hash-commit: FAIL
EVIDENCE stop-bootiso-hash-commit: not returned — the commit has the allowed shape (own
commit `4624ea2`, `flake.nix` only, `outputHash = "sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490="`),
but the return holds no `got:` line (and no `hash mismatch`, no `b6OcAb`, no `x07uJKM8`) from
the boot-iso build, so the committed value cannot be shown to equal a measured value.

VERDICT stop-kernel-bootiso-build: FAIL
EVIDENCE stop-kernel-bootiso-build: not measured at the gated commit — the only
`KERNEL-BOOTISO-BUILD-OK` line is in `WITNESS-RETURN.md`'s last-40-lines excerpt of
`out/kernel/kernel.log`, whose lines name `r9-kernel-x64-af46615` and
`r9-boot-iso-af46615` (pre-Phase-2 code) and whose log carries no `HEAD=` line.

VERDICT stop-kernel-bootiso-rebuild: FAIL
EVIDENCE stop-kernel-bootiso-rebuild: not measured at the gated commit — the only
`KERNEL-BOOTISO-REBUILD-OK` line follows
`checking outputs of '/nix/store/8dpjymiwp76qqb6s0fyp6kkyylvlkv8y-r9-boot-iso-af46615.drv'...`
in the same stale excerpt; no `HEAD=` line.

VERDICT stop-x86-host-tests: FAIL
EVIDENCE stop-x86-host-tests: not returned — no `Executing` line, no
`x86_64-unknown-linux-gnu` line and no `cargo xtask test` line appears in any returned file.

VERDICT stop-x86-rfork-mask-test: FAIL
EVIDENCE stop-x86-rfork-mask-test: not returned — the string
`rfork_reserved_bits_are_unsupported` appears in no returned file.

VERDICT stop-gate-kvm: FAIL
EVIDENCE stop-gate-kvm: not measured at the gated commit — the only `GATE PASS quick` lines
are three excerpts in `WITNESS-RETURN.md` (`run=320886-1795453021`, `run=540452-1795468407`,
`run=307565-1795452813`) from the stale witness directories `out/probe`, `out/gate` and
`out/flipback`, alongside `r9-kernel-x64-af46615`; no direct KVM quick-gate log was returned
and none has a `HEAD=` line.

VERDICT stop-gate-tcg: FAIL
EVIDENCE stop-gate-tcg: not returned — `--accel tcg` appears in no returned file.

VERDICT stop-gate-full: FAIL
EVIDENCE stop-gate-full: not measured at the gated commit — `full/full.log` carries
`GATE PASS full x86_64-linux run=685446-1795491574`, the store labels name `r9-kernel-x64-4624ea2`,
but `grep -c '^HEAD=' full/full.log` is 0, so the log does not carry the plan's commit stamp.

## Notes

- No `HEAD=` line exists anywhere in the return: `grep -c '^HEAD='` over all 21 returned
  files is 0. The checklist's `run_logged` wrapper writes that stamp; the committed `full/`
  tree is the witness driver's own work directory, whose logs it never stamped.
- The full set ran four times in this round: `347497-1795455456` (`boot-r9-tier1-x64 FAIL`),
  then `405781-1795461728`, `623461-1795468943` and `685446-1795491574` (all eight apps PASS).
  `full/full.log` is the last run.
- `WITNESS-RETURN.md` is the pass-2 (Phase 1) witness bundle: its kernel and gate excerpts
  name `r9-kernel-x64-af46615` and `r9-boot-iso-af46615`, so they are not this round's
  evidence whatever their pass lines say.
- `return/secret-scan.txt` reports a clean credential scan; no credential appeared in the
  returned lines and nothing needed redaction.
- The return commit is the allowed shape (files only under `evidence/owner-stop/return/`) and
  is not the gated commit. Nothing on the branch touches code other than the hash commit.

## Stop rule applied

The STOP RULES branch for a missing or differing `HEAD=` line: the affected verdicts are FAIL
and the owner is asked to run sections 3 to 7 again at the branch tip. Steps 3, 5 and 6 were
not returned at all, and the boot-iso build's `got:` value was not returned, so the round
cannot be carried. No code fix is needed on the Mac for this stop; the gate lines that were
returned show no gate failure of their own.
