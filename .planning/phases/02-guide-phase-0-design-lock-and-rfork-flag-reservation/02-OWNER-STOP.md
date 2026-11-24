# Phase 2 end-of-phase owner stop — x86_64 Linux host checklist

Run this page top to bottom on the x86_64 Linux KVM host. It is the single end-of-phase stop
for Phase 2 (the rfork flag reservation); the boot-to-rio gate and the kernel byte-change
procedure run here, once, and nowhere earlier (D-25).

- **Branch:** `phase-02/design-lock`
- **Base commit:** `c002feb2379949ea50bf7e02248c0d21f4edf85e` (the `base` line of
  `evidence/kernel-bytes/before.sha256` in this phase directory; use it when a failure must
  be told from one that was already there).
- **Pushed tip sha:** this file does not hold it and no sha is written here for it. Commits
  are made after this file and before the push, so a sha written now could not equal the
  pushed tip. The pushed tip sha is the output of `git rev-parse HEAD` on the Mac at the
  moment of the push in section 1; the executor of plan 02-06 states it before the push and
  confirms it after the push. An owner who pushes by hand runs `git rev-parse HEAD` first and
  notes the value.
- **`main` is never pushed at this stop (D-26).** Only the branch above moves.

## The one-commit rule (read before starting)

Sections 4 to 7 must all run at **one commit, the gated commit**: the branch tip after
section 4 — the hash commit, or the pulled tip when the old hash still builds. The hash build,
the witness `kernel` step and every gate then measure the same bytes.

If any commit reaches the branch after section 4 has run — a fix from the Mac, whatever it
touches; note that even a test-only edit above non-test code moves the panic line numbers in
the kernel — the round is **void**. Start again at section 2 and run sections 3 to 7 again in
full, the hash build and the witness `kernel` step included. Results from a void round are
not reported as results.

**One exception:** the return commit of section 8, made after section 7, which adds only log
files under `evidence/owner-stop/return/` in the Phase 2 directory and changes no code. It
does not void the round and is not the gated commit.

## Setup (once, in the r9 checkout after section 2)

Every command below that produces a verdict is run through this function, so its output and
exit code land in a log under `~/r9-p02-stop/` and each log begins with the commit it measured:

```bash
mkdir -p "$HOME/r9-p02-stop"
run_logged() {
  name=$1; shift
  log="$HOME/r9-p02-stop/$name.log"
  { printf 'HEAD=%s\n' "$(git rev-parse HEAD)"; "$@"; } 2>&1 | tee "$log"
  rc=${PIPESTATUS[0]}
  printf 'EXIT=%s\n' "$rc" | tee -a "$log"
  return "$rc"
}
```

Run these snippets in bash, with the current directory at the r9 checkout root.

## 1. On the Mac: approve the branch push (D-26)

Review what would be published:

```bash
git log --oneline origin/main..HEAD
```

If it is approved, push the branch (the executor may run exactly this on your word):

```bash
git push origin HEAD:refs/heads/phase-02/design-lock
```

The pushed tip sha is what `git rev-parse HEAD` prints on the Mac at that moment. Nothing else
is pushed; `main` stays where it is.

## 2. On the Linux host: get the branch

```bash
git fetch origin
git switch -C phase-02/design-lock origin/phase-02/design-lock
git rev-parse HEAD
```

Confirm that `git rev-parse HEAD` equals the pushed tip sha from section 1. In a repeated
round, compare against the new tip the executor named at that round's push. Then run the
Setup block above.

## 3. The x86_64 host tests (D-24), before the hash step (D-29)

This is the first compile of this phase's x86_64 test code.

```bash
run_logged host-tests nix develop -c cargo xtask test --verbose
```

Pass needs all three:

- exit 0;
- an `Executing` line for package `x86_64` with target `x86_64-unknown-linux-gnu`;
- the line `test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok` from
  that leg.

When it does not go green:

- **No `x86_64` leg runs** (the fallback did not select the host triple): run

  ```bash
  run_logged host-tests-fallback nix develop -c cargo test --package x86_64 --bins --target x86_64-unknown-linux-gnu rfork
  ```

  Its `... ok` line is then the witness, and the xtask fallback is reported back as still
  needing a fix.
- **A compile error:** report the compiler output in full and stop this checklist here,
  before section 4. The fix is made on the Mac in `x86_64/` or `xtask/` (no approval needed
  for those directories), committed, and offered for a second approved push to the same
  branch; the checklist then restarts at section 2.
- **A failing test other than the `rfork` ones:** run the same command at the base commit in
  a second worktree to see whether it failed before this phase, and report both results:

  ```bash
  git worktree add ~/r9-p02-stop/base c002feb2379949ea50bf7e02248c0d21f4edf85e
  ```

  The owner rules on a failure that was already there; it is not waived by the executor.

## 4. The `boot-iso` `outputHash` commit, made by the owner on this host (D-23, D-28)

Only after section 3 is green (D-29). Run the build at the current tip:

```bash
run_logged boot-iso-first nix build .#boot-iso
```

In the first round the expected result is a failure with `hash mismatch in fixed-output
derivation` and a `got:` value.

- The one line to change is `flake.nix` line 983, which reads (twelve leading spaces):

  ```nix
              outputHash = "sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=";
  ```

  Replace the value between the quotes with the `got:` value and change nothing else on the
  line.
- Recommended, directly above that line, is a new dated comment paragraph in the form of the
  paragraph already there: the date, "end-of-phase stop / Phase 2 (D-28)", the commit the ISO
  was built at, the measured value, that it replaces the `b6OcAb...` value set by commit
  `2bc5478`, and that the kernel changed by panic line numbers only with `.text` identical.
- Commit `flake.nix` alone, as its own commit, with this message and no attribution line:

  ```text
  flake: re-derive boot-iso outputHash after the rfork flag move

  Phase 2 moved the rfork flag constants into port::rfork. The x86_64 kernel changed by
  panic line numbers only; .text is identical (evidence/kernel-bytes/KERNEL-BYTES.md in
  the Phase 2 directory). Built on the x86_64 Linux host at COMMIT; measured GOT;
  replaces sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=.
  ```

- Prove it at the new tip with the witness driver's `kernel` step, which records the
  `kernel-x64.elf` sha256, does the plain build and then `--rebuild` (the plain build must
  come first):

  ```bash
  R9_WITNESS_WORK=$HOME/r9-p02-stop/kernel R9_WITNESS_STEPS="kernel" run_logged kernel-step bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
  ```

  Both the `KERNEL-BOOTISO-BUILD-OK` and the `KERNEL-BOOTISO-REBUILD-OK` verdict lines must
  appear, and the log's `HEAD=` line is the gated commit.
- Push the hash commit to the same branch:

  ```bash
  git push origin phase-02/design-lock
  ```

- If the build succeeds with the hash already in `flake.nix`, the x86_64 ISO bytes match it:
  make no hash commit, say so in the report, and still run the witness `kernel` step at this
  tip.
- In a repeated round this whole section is run again at the new tip, whatever the fix
  touched. The build decides by measurement: a mismatch means a further hash commit in the
  same form (its message names the value it replaces); success means the committed hash
  already matches the kernel bytes of the new tip. The `KERNEL-BOOTISO-BUILD-OK` and
  `KERNEL-BOOTISO-REBUILD-OK` lines reported must come from this round.

## 5. The boot-to-rio quick gate under KVM, once (D-25, D-27)

Call it directly, not through the witness driver's `gate` step, which runs it twice:

```bash
run_logged gate-kvm nix develop -c cargo xtask gate --arch x86-64 --accel kvm
```

Pass needs exit 0 and the quick-tier pass line: the format string
`GATE PASS quick x86_64 run=` followed by the run id (printed by `xtask/src/gate.rs`).

"Once" means once per valid round; a void round's run does not count.

## 6. The TCG gate the kernel byte-change rule requires

On this native amd64 host, which is the approved venue (no Docker container is needed):

```bash
run_logged gate-tcg nix develop -c cargo xtask gate --arch x86-64 --accel tcg
```

Same pass line as section 5.

## 7. The full gate set (D-27)

```bash
R9_WITNESS_WORK=$HOME/r9-p02-stop/full R9_WITNESS_STEPS="full" run_logged gate-full bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
```

This runs `cargo xtask gate --arch x86-64 --full --accel kvm` and keeps each app's captures.
Pass needs the full-tier pass line — the format string `GATE PASS full ` followed by the flake
system `x86_64-linux`, ` run=` and the run id (`xtask/src/gate.rs`) — and a step exit of 0.
The driver closes with a line beginning `R9-WITNESS-RUN-OK: every selected step passed`.

## 8. Report back

Report one line per step — gated commit sha, hash commit sha or "no change",
`kernel-x64.elf` sha256, `KERNEL-BOOTISO-BUILD-OK`, `KERNEL-BOOTISO-REBUILD-OK`, host tests,
the mask test line, KVM gate, TCG gate, full set — and the `HEAD=` line of each log, by
either route:

- paste the lines and the `EXIT=` lines, or
- commit the logs under `evidence/owner-stop/return/` in this Phase 2 directory on the branch
  and push it (this is the section 8 return commit named in the header: logs only, no code).

The `HEAD=` lines of sections 4 to 7 must all be the gated commit. Never return a credentials
file. A gate FAIL is a complete answer: report it and stop; `main` is not pushed.
