# Phase 4 end-of-phase owner stop — x86_64 Linux host checklist

Run this page top to bottom on the x86_64 Linux host (the KVM witness). It is the single
end-of-phase stop for Phase 4 (the 9P2000.aead transport and the fid table), and it checks the
things the Mac cannot check: the sandboxed x86_64 kernel derivations, the boot-to-rio gates,
and the `boot-iso` fixed-output hash for the new kernel bytes (D-25).

## 0. What this stop is, and why it is here

- **Branch:** `phase-04/aead-wire`. Nothing else is pushed at this stop.
- **Base commit:** `3ce5ed8c4e309710e2157ab5b63a03a40797d185` — the `base` line of
  `evidence/kernel-bytes/before.sha256` in this phase directory. It is the commit the kernel
  baseline was taken at, before the first workspace change of the phase.
- **Pushed tip sha:** this file does not hold it, and no sha is written here for it. Commits are
  made after this file, so a sha written now could not equal the pushed tip. The pushed tip is
  the output of `git rev-parse HEAD` on the Mac at the moment of the push in section 1; the
  executor of plan 04-15 states it before the push and confirms it after, and the owner confirms
  the Linux checkout equals it in section 2. If the owner pushes by hand, run
  `git rev-parse HEAD` first and note the value.
- **`main` is not pushed at this stop.** Only the branch above moves. A push of `main` is a
  separate decision, taken only after these results are green.

**Why the Mac cannot make this check.** `kernel-x64` and `boot-iso` are nix derivations that
vendor every crate from the committed `Cargo.lock` through `importCargoLock`, inside the nix
sandbox, and exist only for `x86_64-linux`. Whether the enlarged lock still builds with `port`
linking `crypto` and `ds`, whether the new kernel boots to rio, and whether the fixed-output
`boot-iso` hash matches the new kernel bytes can only be measured on a Linux host.

**What the Mac has already proven.** Both kernels were rebuilt from nothing at the reviewed
commit and their whole-ELF sha256, defined text symbol totals and FP/SIMD scan counts were
recorded: `evidence/kernel-bytes/KERNEL-BYTES.md` carries the `@04-14` verdict lines and
`evidence/kernel-bytes/after-04-14.sha256` holds the four-line record. Phase 2's "`.text`
identical" rule is **not** used here (D-29): linking `crypto`/`ds` changes `.text` by itself, so
the whole-ELF hashes differ from the baseline and the committed `boot-iso` `outputHash` **will
not** match the new x86_64 kernel. Those Mac hashes are a devshell measurement and are **not**
comparable with a Linux-built kernel: the kernel `.rodata` carries toolchain store-path strings
and the toolchain derivation differs per system. Only the measurements on this one host are the
Linux evidence.

## The one-commit rule (read before starting)

Sections 5 to 8 must all run at **one commit, the gated commit**: the branch tip after
section 5 — the hash commit. The hash build, the rebuilt ISO and every gate then measure the
same bytes.

If any commit reaches the branch after section 5 has run — a fix from the Mac, whatever it
touches; note that even a test-only edit above non-test code moves the panic line numbers in the
kernel — the round is **void**. Start again at section 2 and run sections 3 to 8 again in full,
the hash build included. Results from a void round are not reported as results.

**One exception:** the return commit of section 9, made after section 8, which adds only log
files under `evidence/owner-stop/return/` in the Phase 4 directory and changes no code. It does
not void the round and is not the gated commit.

## Setup (once, in the r9 checkout after section 2)

Every command below that produces a verdict is run through this function, so its output and exit
code land in a log under `~/r9-p04-stop/` and each log begins with the commit it measured:

```bash
mkdir -p "$HOME/r9-p04-stop"
run_logged() {
  name=$1; shift
  log="$HOME/r9-p04-stop/$name.log"
  { printf 'HEAD=%s\n' "$(git rev-parse HEAD)"; "$@"; } 2>&1 | tee "$log"
  rc=${PIPESTATUS[0]}
  printf 'EXIT=%s\n' "$rc" | tee -a "$log"
  return "$rc"
}
```

Run these snippets in bash, with the current directory at the r9 checkout root. Each log's
`HEAD=` line is the commit that step measured; a `HEAD=` line that does not match the commit the
step is supposed to measure makes that step's result void.

## 1. On the Mac: approve the branch push

Review exactly what would be published:

```bash
git log --oneline origin/main..HEAD
```

If it is approved, push the branch — the executor may run exactly this command on your word:

```bash
git push origin HEAD:refs/heads/phase-04/aead-wire
```

Nothing else is pushed: `main` stays where it is, and no tag is created. The pushed tip sha is
what `git rev-parse HEAD` prints on the Mac at that moment; the executor states it, and section 2
checks the Linux checkout against it.

## 2. On the Linux host: get the branch

```bash
git fetch origin
git switch -C phase-04/aead-wire origin/phase-04/aead-wire
git rev-parse HEAD
```

Confirm that `git rev-parse HEAD` equals the pushed tip sha the executor stated. If it does not,
stop and report both values; nothing below is meaningful at the wrong commit. Then run the Setup
block above.

## 3. The x86_64 host tests first, before the hash step (D-29)

This is the first compile of this phase's x86_64 test code. It runs **before** section 5 so a
compile fix cannot void a hash commit.

```bash
run_logged host-tests nix develop -c cargo xtask test
run_logged host-test-9p nix develop -c cargo xtask test-9p
```

**Pass:** exit 0 for both, and both `EXIT=` lines are `0`. `test-9p` is the crypto/`port` leg
that exercises the new wire code on this host's x86_64 backends.

**If it fails:** report the full output and stop this checklist here, before section 4. A fix is
made on the Mac, re-verified with the plan 04-14 Task 3 suite runs and the kernel comparison,
committed and offered for a second approved push to the same branch; the checklist then restarts
at section 2.

## 4. The sandboxed kernel builds at the tip

These are the derivations that vendor the enlarged `Cargo.lock` inside the nix sandbox. This is
the first time they are built with `port` linking `crypto` and `ds`.

```bash
run_logged tip-kernel-x64 nix build .#kernel-x64 -o "$HOME/r9-p04-stop/tip-kernel-x64"
run_logged tip-kernel-virt nix build .#kernel-virt -o "$HOME/r9-p04-stop/tip-kernel-virt"
```

**Pass:** exit 0 for both, and both `EXIT=` lines are `0`.

**If it fails:** a failure that names a missing vendored crate, a `Cargo.lock` problem or a cargo
fetch error is the sandbox risk eating the new edge. Report the output in full and stop. Do not
edit the derivation, the lock or `flake.nix` on this host: the fix is new planned work on the
Mac, and this checklist is then run again in full from section 2.

## 5. The `boot-iso` `outputHash` commit, made by the owner on this host (D-28)

Only after section 3 is green (D-29). Run the build at the current tip:

```bash
run_logged boot-iso-first nix build .#boot-iso
```

The expected result is a failure with `hash mismatch in fixed-output derivation` and a `got:`
value. That is the whole point: linking `crypto`/`ds` changed the x86_64 kernel bytes, so the
committed hash from before Phase 4 no longer matches (D-29).

- The one line to change is `flake.nix` line 983, which reads today (twelve leading spaces):

  ```nix
              outputHash = "sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=";
  ```

  Replace the value between the quotes with the `got:` value and change nothing else on the line.
- Recommended, directly above that line, is a new dated comment paragraph in the form of the
  paragraph already there: the date, "end-of-phase stop / Phase 4 (D-29)", the commit the ISO was
  built at, the measured value, that it replaces the `x07uJKM...` value, and that the x86_64
  kernel text changed because `port` links `crypto` and `ds` (the Mac evidence set is in
  `evidence/kernel-bytes/KERNEL-BYTES.md`).
- Commit `flake.nix` alone, as its own commit, with this message and no attribution line:

  ```text
  flake: re-derive boot-iso outputHash for the Phase 4 kernel (port links crypto and ds)
  ```

- Prove it at the new tip: the plain build first, then `--rebuild` (the rebuild must come
  second):

  ```bash
  run_logged boot-iso-after nix build .#boot-iso
  run_logged boot-iso-rebuild nix build .#boot-iso --rebuild
  ```

  Both must exit 0 with the new committed hash.
- Push the hash commit to the same branch:

  ```bash
  git push origin phase-04/aead-wire
  ```

- If the first build succeeds with the hash already in `flake.nix`, the x86_64 ISO bytes already
  match it: make no hash commit, say so in the report, and still run the plain build and
  `--rebuild` at this tip.
- In a repeated round this whole section is run again at the new tip, whatever the fix touched.
  The build decides by measurement: a mismatch means a further hash commit in the same form (its
  message names the value it replaces); success means the committed hash already matches the
  kernel bytes of the new tip.

## 6. The boot-to-rio quick gate under KVM, once

Call it directly, not through the witness driver's `gate` step, which runs it twice:

```bash
run_logged gate-kvm nix develop -c cargo xtask gate --arch x86-64 --accel kvm
```

**Pass:** exit 0 and the quick-tier pass line — the format string `GATE PASS quick x86_64 run=`
followed by the run id (printed by `xtask/src/gate.rs`).

"Once" means once per valid round; a void round's run does not count.

## 7. The TCG gate the kernel byte-change rule requires, natively on this host

On this native amd64 host, which is the approved venue (D-25; no Docker container is needed —
the venue is the owner's x86_64 Linux host, as in Phase 2):

```bash
run_logged gate-tcg nix develop -c cargo xtask gate --arch x86-64 --accel tcg
```

Same pass line as section 6.

## 8. The full gate set

```bash
run_logged gate-full nix develop -c cargo xtask gate --arch x86-64 --full --accel kvm
```

**Pass:** exit 0 and the full-tier pass line — the format string `GATE PASS full ` followed by
the flake system `x86_64-linux`, ` run=` and the run id (`xtask/src/gate.rs`).

## 9. Report back

Report one line per step — pushed tip sha, gated (hash) commit sha or "no change",
`boot-iso-first`, `boot-iso-after`, `boot-iso-rebuild`, `host-tests`, `host-test-9p`,
`tip-kernel-x64`, `tip-kernel-virt`, KVM gate, TCG gate, full set — and the `HEAD=` line of each
log, by either route:

- paste the lines and the `EXIT=` lines, or
- commit the logs under `evidence/owner-stop/return/` in this Phase 4 directory on the branch and
  push it (this is the section 9 return commit named in the header: logs only, no code).

The `HEAD=` lines of sections 3 to 8 must all be the pushed tip, except `boot-iso-after` and
`boot-iso-rebuild`, whose `HEAD=` line is the gated commit (the hash commit) made in section 5.
Never return a credentials file. A gate FAIL is a complete answer: report it and stop; `main` is
not pushed.

## What each failure means, and what happens next

- **Section 3 red:** the new code does not compile or pass on x86_64. Nothing is fixed on this
  host; a fix on the Mac re-runs the plan 04-14 Task 3 evidence (the suite runs and the kernel
  comparison) and this checklist restarts at section 2.
- **Section 4 red:** the sandboxed derivations do not build with the new edge. Same handling.
- **Section 5 mismatch is expected once.** A second mismatch after the hash commit means a
  further kernel byte change landed — the one-commit rule was broken or a commit reached the
  branch — and the round restarts.
- **Sections 6 to 8 red:** the new kernel does not boot to rio. Report the gate log in full and
  stop; the fix is new planned work on the Mac.

**Nothing is fixed on this host.** No source file, no `Cargo.lock`, no derivation and no
`flake.nix` line other than the `outputHash` value of section 5 is edited here. `main` is not
pushed at this stop.
