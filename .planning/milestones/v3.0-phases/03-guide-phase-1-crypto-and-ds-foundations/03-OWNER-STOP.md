# Phase 3 end-of-phase owner stop — x86_64 Linux host checklist

Run this page top to bottom on the x86_64 Linux host (the KVM witness). It is the single
end-of-phase stop for Phase 3 (`crypto/` and `ds/` foundations), and it checks the one thing
the Mac cannot check.

## 0. What this stop is, and why it is here

- **Branch:** `phase-03/crypto-ds-foundations`. Nothing else is pushed at this stop.
- **Base commit:** `4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1` — the `base` line of
  `evidence/kernel-bytes/before.sha256` in this phase directory. It is the commit the kernel
  baseline was taken at, before the first workspace change of the phase; section 4 builds it
  again for the byte comparison.
- **Pushed tip sha:** this file does not hold it, and no sha is written here for it. Commits
  are made after this file, so a sha written now could not equal the pushed tip. The pushed
  tip is the output of `git rev-parse HEAD` on the Mac at the moment of the push in section 1;
  the executor of plan 03-11 states it before the push and confirms it after, and the owner
  confirms the Linux checkout equals it in section 2. If the owner pushes by hand, run
  `git rev-parse HEAD` first and note the value.
- **`main` is not pushed at this stop.** Only the branch above moves. A push of `main` is a
  separate decision, taken only after these results are green.

**Why the Mac cannot make this check.** The `kernel-x64` and `kernel-virt` packages are nix
derivations that vendor every crate from the committed `Cargo.lock` through
`importCargoLock`, inside the nix sandbox. Phase 3's `Cargo.lock` grew by the six `crypto/`
pins and their transitive crates (plan 03-01) and by `ds/` with its `roaring` test oracle
(plan 03-12). Whether those vendored derivations still build with the enlarged lock, and
whether they produce the same kernel bytes as the base commit, can only be measured by
building them on a Linux host: `kernel-x64` and `boot-iso` exist only for `x86_64-linux`, and
the derivations cannot be built on `aarch64-darwin` (03-RESEARCH.md assumption A8). This is
the risk this stop retires.

**What the Mac has already proven.** Both kernels were rebuilt from nothing at the end of the
phase, in the devshell, and are byte-identical to the baseline taken before the first
workspace change, on one venue: `evidence/kernel-bytes/KERNEL-BYTES.md` carries
`VERDICT elf-virtshell@03-10: IDENTICAL` and `VERDICT elf-q35churnshell@03-10: IDENTICAL`, and
`evidence/kernel-bytes/after-03-10.sha256` holds the four-line record. Those hashes are a Mac
devshell measurement and are **not** comparable with a Linux-built kernel: the kernel
`.rodata` carries toolchain store-path strings and the toolchain derivation differs per
system. Only section 4's tip-versus-base comparison, made on this one host, is the Linux
evidence.

## Setup (once, in the r9 checkout, after section 2)

Every command below that produces a verdict runs through this function, so its output and exit
code land in a log under `~/r9-p03-stop/` and each log begins with the commit it measured:

```bash
mkdir -p "$HOME/r9-p03-stop"
run_logged() {
  name=$1; shift
  log="$HOME/r9-p03-stop/$name.log"
  { printf 'HEAD=%s\n' "$(git rev-parse HEAD)"; "$@"; } 2>&1 | tee "$log"
  rc=${PIPESTATUS[0]}
  printf 'EXIT=%s\n' "$rc" | tee -a "$log"
  return "$rc"
}
```

Run these snippets in bash with the current directory at the r9 checkout root, except where a
step says to change directory. Each log's `HEAD=` line is the commit that step measured; a
`HEAD=` line that does not match the commit the step is supposed to measure makes that step's
result void.

## 1. On the Mac: approve the branch push

Review exactly what would be published:

```bash
git log --oneline origin/main..HEAD
```

If it is approved, push the branch — the executor may run exactly this command on your word:

```bash
git push origin HEAD:refs/heads/phase-03/crypto-ds-foundations
```

Nothing else is pushed: `main` stays where it is, and no tag is created. The pushed tip sha is
what `git rev-parse HEAD` prints on the Mac at that moment; the executor states it, and section
2 checks the Linux checkout against it.

## 2. On the Linux host: get the branch

```bash
git fetch origin
git switch -C phase-03/crypto-ds-foundations origin/phase-03/crypto-ds-foundations
git rev-parse HEAD
```

Confirm that `git rev-parse HEAD` equals the pushed tip sha the executor stated. If it does
not, stop and report both values; nothing below is meaningful at the wrong commit. Then run
the Setup block above.

## 3. The sandboxed kernel builds at the tip (the A8 risk)

These are the derivations that vendor the enlarged `Cargo.lock` inside the nix sandbox. This
is the first time they are built with Phase 3's lock.

```bash
run_logged tip-kernel-x64 nix build .#kernel-x64 -o "$HOME/r9-p03-stop/tip-kernel-x64"
run_logged tip-kernel-virt nix build .#kernel-virt -o "$HOME/r9-p03-stop/tip-kernel-virt"
```

**Pass:** exit 0 for both, and both `EXIT=` lines are `0`.

**If it fails:** a failure that names a missing vendored crate, a `Cargo.lock` problem or a
cargo fetch error is the A8 risk. Report the output in full and stop. Do not edit the
derivation, the lock or `flake.nix` on this host: the fix is new planned work on the Mac, and
this checklist is then run again in full from section 2.

## 4. The same two builds at the base commit, and the byte comparison

A second worktree holds the base commit, so both sets of kernel bytes come from one host and
one nix store.

```bash
git worktree add "$HOME/r9-p03-stop/base" 4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1
cd "$HOME/r9-p03-stop/base"
run_logged base-kernel-x64 nix build .#kernel-x64 -o "$HOME/r9-p03-stop/base-kernel-x64"
run_logged base-kernel-virt nix build .#kernel-virt -o "$HOME/r9-p03-stop/base-kernel-virt"
cd -
```

Each of those two logs must show the base commit in its `HEAD=` line (the `cd` is what makes
that so). Then hash the four kernel ELF files from the tip checkout:

```bash
run_logged kernel-hashes sha256sum \
  "$HOME/r9-p03-stop/tip-kernel-x64/kernel-x64.elf" \
  "$HOME/r9-p03-stop/tip-kernel-virt/kernel-virt.elf" \
  "$HOME/r9-p03-stop/base-kernel-x64/kernel-x64.elf" \
  "$HOME/r9-p03-stop/base-kernel-virt/kernel-virt.elf"
```

**Pass:** exit 0 for both builds, and the hashes pair up — `tip-kernel-x64` and
`base-kernel-x64` print one and the same hash, and `tip-kernel-virt` and `base-kernel-virt`
print one and the same hash. That is "no kernel byte changed on Linux", which is what keeps
the `boot-iso` `outputHash` procedure from applying (CONTEXT D-21).

**If a pair differs:** kernel bytes changed on Linux although the Mac builds were identical.
Report both hashes and the output in full and stop. Do not edit the `boot-iso` `outputHash`
and do not change any source file on this host: the owner decides what follows (the x86_64
kernel byte-change procedure, or new planned work).

## 5. The fixed-output ISO at the tip — the second witness for the x86_64 kernel

With the committed `outputHash`, this derivation stages the x86_64 kernel ELF and will fail if
those bytes moved. The plain build comes first, then the rebuild.

```bash
run_logged boot-iso-first nix build .#boot-iso
run_logged boot-iso-rebuild nix build .#boot-iso --rebuild
```

**Pass:** exit 0 for both, with the `outputHash` that is committed in `flake.nix` (line 983,
`sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=`).

**If it fails with `hash mismatch in fixed-output derivation`:** report the whole message
including its `got:` value and stop. That is a kernel byte difference seen through the ISO; it
is not fixed on this host, and the hash is not edited at this stop.

## 6. The suite on the x86_64 host

This is the first time the default `crypto` test leg links the x86_64 hardware backends
(AVX2/SHA-NI) instead of the aarch64 ones; the soft-backend leg pins the scalar code the
kernels run, so both paths matter.

```bash
run_logged host-test nix develop -c cargo xtask test
run_logged host-check nix develop -c cargo xtask check
run_logged host-clippy nix develop -c cargo xtask clippy
run_logged host-miri nix develop -c cargo xtask miri
```

**Pass:** exit 0 for each of the four. `cargo xtask miri` takes several minutes on this suite
(64 seeds over 29 `tpht` tests); let it finish.

**If a command fails:** report its output in full and stop. Nothing is fixed on this host. A
fix under `crypto/`, `ds/` or `xtask/` is made on the Mac, re-verified with the plan 03-10
Task 2 suite runs and kernel comparison, committed and offered for a second approved push to
the same branch; the checklist then restarts at section 2.

## 7. Report back

Report one line per step, each with its log's `HEAD=` and `EXIT=` lines:

- the pushed tip sha;
- `tip-kernel-x64` build: exit code;
- `tip-kernel-virt` build: exit code;
- the four `kernel-hashes` lines and whether the two pairs match;
- `boot-iso-first`: exit code;
- `boot-iso-rebuild`: exit code;
- `host-test`, `host-check`, `host-clippy`, `host-miri`: exit code each.

By either route:

- paste the lines and the `EXIT=` lines, or
- commit the logs under `evidence/owner-stop/return/` in this Phase 3 directory on the branch
  and push the branch (files only, no code change).

Every `HEAD=` line of sections 3 to 6 must be the pushed tip, except the two base-build logs of
section 4, whose `HEAD=` line is the base commit. Never return a credentials file. A failure is
a complete answer: report it and stop; `main` is not pushed.

## Open choices for the owner (nothing here blocks the stop)

Nothing here changes any verdict above or is a reason to delay this stop. Answer them whenever
you like; "keep as planned" is a complete answer to all of them.

- [ ] **`zeroize` is pinned at `=1.9.2`; the older alternative is `=1.9.1`.** `=1.9.2` was
      published 2026-11-17, seven days before the research; research built and probed that
      version. The older alternative `=1.9.1` (2026-10-06, same feature list, not built in
      research) is a drop-in for this phase's tests. Keep `=1.9.2`, or say "use 1.9.1": that is
      one manifest line and one lock entry on the Mac, then the plan 03-10 Task 2 suite runs
      and kernel comparison again, and this checklist again from section 2. Nothing in Phase
      3's tests depends on which of the two is pinned.
- [ ] **ChaCha20 through `crypto::page` covers 2^32 - 1 blocks (274,877,906,880 bytes) per key
      and nonce**, one block fewer than RFC 8439, because the pinned `chacha20 =0.10.2`
      refuses the last counter value. The PROJECT.md row gives the detail and the module doc
      states it. Accept the limit, or ask for the last block as new work; the D-06 layout (a
      32-bit counter and a 96-bit nonce) and the RFC 8439 vectors are unaffected either way.
- [ ] The other planning-level rows in PROJECT.md marked "for owner review at Phase 3
      verification" are reviewed at verification, not here.

## Not part of this stop

The boot-to-rio gate is not run here. No kernel byte changed — both the Mac comparison and
sections 3 to 5 of this checklist are the evidence — so the golden invariant holds by identity
with the Phase 2 gated state (`4624ea2`), which was gated by the full gate set on this host.
If section 4 shows a differing kernel hash, or section 5 shows an ISO hash mismatch, that
reasoning no longer holds and the owner rules on what runs next.
