# Linux KVM witness — S1, S5, S3 and S6 (Plan 06)

This is the owner's one-session run of every spike leg that needs an x86_64
Linux host with KVM (D-13). The Mac cannot reach that host, and nothing is
copied between them — no `scp`, no shared filesystem, no Mac-hosted server.
Everything the host needs is committed to r9 and arrives with `git pull`;
results come back through GitHub (return path A) or as pasted text (return
path B).

Nothing here needs root and nothing calls `sudo`. `run-from-repo.sh` refuses
to run as root. `run-all.sh` also refuses root and verifies
`MANIFEST.sha256` before its first step. The scripts write only under
`${R9_WITNESS_STAGE:-$HOME/r9-spike-work/witness-bundle}` and
`R9_SPIKE_WORK` (default `$HOME/r9-spike-work`).

## Needs on the Linux host

- nix with flakes and the sandbox enabled;
- a user with write access to `/dev/kvm`;
- network access to GitHub (the r9 clone or pull, and the s3 and s6 steps
  fetch the pinned Harvey and r9 revisions);
- at least **30 GB free** under `~/r9-spike-work` (the s3 step builds the
  full Harvey x86_64 userland and bakes nine images);
- no `sudo`, no root, and no access to your own r9 or Harvey checkouts.

The steps that need `qemu`, `git`, `expect`, `python3` or `procps` get them
from nix through the committed spike locks and through r9's devshell. The
host itself needs only nix: `run-from-repo.sh` rebuilds the probe inside
r9's devshell before it stages the bundle.

## Expected duration

The **s3 step includes a full Harvey x86_64 tier build and nine image
bakes**, so expect **tens of minutes or more** for that step alone; the two
gate runs and the determinism rebake repeat parts of it. s1 is seconds to a
few minutes once nix has fetched its inputs, s5 is a short KVM boot, and s6
adds three chain-image bakes plus three q35shell boots. Budget a session of
an hour or more, network depending.

## The command sequence (pull-only)

On the Linux host:

1. Get the repository:

   ```
   git clone git@github.com:requaos/r9.git
   ```

   or, in an existing r9 clone:

   ```
   git -C <r9> pull --ff-only
   ```

2. Record the exact revision you are about to run:

   ```
   git -C <r9> rev-parse HEAD
   ```

3. Run the witness entry point from the r9 checkout root:

   ```
   cd <r9> && bash .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/run-from-repo.sh
   ```

Optional environment variables:

- `R9_SPIKES_ONLY="s3 s6"` — run a subset of the steps;
- `R9_WITNESS_STAGE=/some/path` — change the staging root (default
  `$HOME/r9-spike-work/witness-bundle`);
- `R9_SPIKE_WORK=/some/path` — change the scratch root (default
  `$HOME/r9-spike-work`);
- `S5_KERNEL_EXTRA=/path/to/your/own/9k10cpu.elf` — add a second s5 leg with
  a kernel you built yourself at `bd7a698` (the committed kernel is used
  first).

`run-from-repo.sh` rebuilds the probe, checks the committed artifacts, stages
the bundle, and verifies its manifest before any leg runs. Add `--check` to
stop after staging and manifest verification; that mode runs no Linux-only
leg and is the one exercised on the Mac.

## The committed kernel and the rebuilt probe

The exact S5 kernel bytes are committed at
`spikes/s5/9k10cpu.elf`:

- size: **2,030,226 bytes**
- sha256: `ecdcdaddea8d3759d74303b869ddfae8efeba1d4d160382577cfc52b9a32c45b`

This is a deliberate, documented deviation from the spikes' text-only
convention. The kernel is a binary, and with no Mac→Linux transfer allowed,
committing the exact bytes is the only way `git pull` can deliver the kernel
the KVM leg must boot. `run-from-repo.sh` checks the file against
`spikes/s5/evidence/kernel-provenance.txt` before staging and aborts with
`R9-WITNESS-ERROR kernel-sha-mismatch: ...` if it does not match. Nothing
else binary is committed: `spikes/s5/work/` stays gitignored.

The S6 amd64 a.out probe is **not** committed. `run-from-repo.sh` rebuilds it
from the committed sources:

```
nix develop -c sh spikes/s6/build-probe.sh amd64
nix develop -c python3 -I spikes/s6/aout.py check --arch amd64 spikes/s6/work/probe-amd64.aout
```

It records the rebuilt probe's sha256 and compares it with the value recorded
in `spikes/s6/evidence/amd64-static.txt` (`db94057f04c751c59a648de5c8505f45
2bb225b6a276714e7ee5544396a5dcd5`). A mismatch is **not** fatal: the probe
was built with this host's locked rustc, and the structural `aout.py check`
is the gate. The S6 KVM leg validates the actual load and run.

## What run-from-repo.sh does

1. Refuses root and resolves the r9 checkout root with git (any cwd inside
   the checkout works).
2. Verifies the committed kernel's sha256 and size against
   `spikes/s5/evidence/kernel-provenance.txt`.
3. Rebuilds the amd64 probe from the committed sources with nix, runs
   `aout.py check`, and records the sha256 comparison.
4. Stages a complete bundle under
   `${R9_WITNESS_STAGE:-$HOME/r9-spike-work/witness-bundle}` with the same
   layout the Mac-side bundle used: `s1/`, `s3/`, `s5/`, `s6/`,
   `linux-witness/`, and `MANIFEST.sha256`. No `work/`, `evidence/` or
   findings file is staged.
5. Writes `MANIFEST.sha256` over every staged file and verifies it from a
   fresh extraction.
6. Unless `--check` was given, runs `linux-witness/run-all.sh` from the
   staging root (passing `R9_SPIKES_ONLY` through), tees the console to
   `run-all.log`, and writes `WITNESS-RETURN.md` beside `out/`.
7. Prints what to report back and the two return paths below.

## What run-all.sh does

- **Preflight** writes `out/env.txt`: `uname -a`, `nproc`, memory, free disk
  under `R9_SPIKE_WORK`, `nix --version`, the `sandbox` and
  `experimental-features` settings, whether `/dev/kvm` is writable, and the
  sysctls `user.max_user_namespaces`, `kernel.unprivileged_userns_clone` and
  `kernel.apparmor_restrict_unprivileged_userns`. The last three are what
  decide whether Harvey's unshare-based x86_64 venue can start.
- **s1** runs `nix build` for `s1-nsprobe` (rung 1) and `s1-libc-amd64`
  (rung 2: Harvey's host toolchain plus the amd64 libc in the proot venue)
  from the staged flake. Logs: `out/s1/`.
- **s5** runs Harvey's own `9k10cpu.elf` under q35 KVM with virtio-net, via
  the staged `s5/boot-q35.sh`; the serial capture is `out/s5/q35-kvm.log`.
  The kernel is the committed `s5/9k10cpu.elf` verified above. If
  `S5_KERNEL_EXTRA` is set, the same leg runs again with that kernel and its
  own capture.
- **s3** prefetches the pinned Harvey source
  (`bd7a6982a3e4f465f8ed3da5e0c7290521514ff0`), copies it to
  `$R9_SPIKE_WORK/harvey` (writable, no `.git`), runs the rio gate, the
  `control-noG` gate and the two-bake determinism check, all with
  `KERNDATE=1795382101`. Logs: `out/s3/rio.log`,
  `out/s3/control-noG.log`, `out/s3/determinism.log`.
- **s6** clones `requaos/r9` bloblessly and checks out
  `116bcc23c8052a1728992cdbc7b625143260d683`, then runs the q35shell
  control/probe/flip-back swap against the s3-built scratch Harvey tree.
  `S6_PROBE` is the probe this host rebuilt and staged. Log:
  `out/s6/linux-x86_64.log`; the terminal line is
  `S6-X86 control=PASS probe=PASS flipback=PASS sha-equal=yes` on success.
  If the s3 step left no built `build/x86_64/out`, s6 prints
  `S6-SKIP no-harvey-build` and records exit 77 instead of pretending to
  run.
- **Finish** lists surviving `qemu-system`, `Xvfb` or `drawterm` processes
  in `out/survivors.txt`, deletes any `auth-creds` artifact found under
  `out/` and any file containing the scratch tree's DEMO password value,
  and records both in `out/secret-scan.txt` (the value itself is never
  printed).

## Exit codes and the summary

`out/summary.tsv` has one row per selected step: `step<TAB>exit<TAB>seconds`.
`run-all.sh` exits 0 only when every selected step exited 0; a skipped s6 is
77. A non-zero step is still a complete answer — the evidence that names the
failure is in `out/<step>/`. `run-from-repo.sh` records that exit code in
`WITNESS-RETURN.md` and exits with the same status.

## Re-running a subset

Rerun the entry point with the subset you want:

```
cd <r9> && R9_SPIKES_ONLY="s3 s6" bash .planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/run-from-repo.sh
```

The s6 step needs the scratch tree and `build/x86_64/out` that the s3 step
leaves in `$R9_SPIKE_WORK/harvey`; rerun s3 first if that tree is gone.
Re-staging does not delete the scratch tree.

## Returning the results

`WITNESS-RETURN.md` is written beside `out/` in the staging root (default
`$HOME/r9-spike-work/witness-bundle/WITNESS-RETURN.md`). It contains the git
HEAD sha, the kernel sha256, the probe sha256 match/mismatch, the contents of
`out/env.txt`, `out/summary.tsv`, `out/survivors.txt`,
`out/secret-scan.txt`, and the last 40 lines of every `out/<step>/*.log`.

### Return path A (preferred): push a witness-returns branch from the Linux host

From the Linux host, in the r9 clone:

```
cd <r9>
git switch -C witness-returns/01-06
ret=.planning/phases/01-stage-p0-cross-build-harness/spikes/linux-witness/return
mkdir -p "$ret"
cp -a "$HOME/r9-spike-work/witness-bundle/out" "$ret/"
cp -a "$HOME/r9-spike-work/witness-bundle/run-all.log" "$ret/"
cp -a "$HOME/r9-spike-work/witness-bundle/WITNESS-RETURN.md" "$ret/"
git add "$ret"
git commit -m "witness(01-06): Linux KVM witness returns"
git push -u origin witness-returns/01-06
```

Then report: `pushed witness-returns/01-06 at <commit sha>`. The executor
fetches that branch from GitHub on the Mac. Nothing is pushed from the Mac.

If you changed `R9_WITNESS_STAGE`, substitute that path for
`$HOME/r9-spike-work/witness-bundle` in the `cp` commands.

### Return path B: push is not possible

Paste the full `WITNESS-RETURN.md` into the session:

```
cat "$HOME/r9-spike-work/witness-bundle/WITNESS-RETURN.md"
```

The full `out/` tree and `run-all.log` stay at the staging root; path B only
sends the compact summary, which is enough to record the verdicts if a step
failed. If the executor needs a specific full log afterward, it will ask for
that one file.

Ignore the `tar` command that `run-all.sh` prints at the end of its output:
it belongs to the retired `scp` flow. Use return path A or B above.

## What not to send back

Do not add `auth-creds.txt` or `authfeed.txt` from the scratch Harvey tree to
the return artifacts. The driver never copies them into `out/`, and it
deletes any that appear there; only `out/`, `run-all.log` and
`WITNESS-RETURN.md` belong in a return.

## Cleanup on the Linux host

```
rm -rf ~/r9-spike-work
```

## Retired Mac-side bundle

`make-bundle.sh` is kept for reference: it built and checksummed the first
bundle on the Mac. `scp` is no longer the path; do not use it for this
session. The pull-only flow above is the handoff.
