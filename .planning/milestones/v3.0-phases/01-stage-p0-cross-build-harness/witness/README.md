# Pass-2 witness driver (01-09, 01-32): rio, control, a13 and rio-diag

This is the owner's one-command handoff for the rio de-risk at stop A (plan
01-12, D-13/D-28) and the diagnostic stop A2 (plan 01-32). It runs on the
x86_64 Linux host with KVM. The Mac cannot reach that host and nothing is
copied between them — no `scp`, no shared filesystem, no Mac-hosted server.
Everything the host needs is committed to r9 and arrives with `git pull`;
results come back through GitHub (return path A) or as pasted text (return
path B).

The driver refuses root and needs no privileges. It writes only under
`R9_WITNESS_WORK` (default `$HOME/r9-witness-work`) and never escalates
privileges.

## What each step proves

| Step | Command it runs | Expected marker |
|---|---|---|
| `rio` | `HARVEY=<scratch> RIO_MODE=rio R9_ACCEL=kvm KERNDATE=1795382101 nix run path:<r9>/.planning/phases/01-stage-p0-cross-build-harness/rio-derisk#rio-gate` | `RIO-SERVING` (the serial `ls /srv` shows **both** `rio.<user>.<pid>` and `riowctl.<user>.<pid>`) |
| `control` | the same with `RIO_MODE=control-noG` (the unchanged `-G` discriminator) | `RIO-CONTROL-NO-RIO` (no `rio.` entry) |
| `a13` | `nix build --no-link --print-out-paths .#checks.x86_64-linux.harvey-drawterm` from the r9 root | a `/nix/store/...` path (plan 01-10 adds this check) |
| `rio-diag` | the same as `rio` with `RIO_MODE=rio-diag` (the diagnostic, plan 01-32) | `RIO-DIAG login-ok` **or** `RIO-FAIL login-not-reached`, then the unchanged pair verdict (`RIO-SERVING` / `RIO-FAIL rio-not-serving`); `out/rio-diag/` also carries `x11.txt` and two PNGs |

The `rio` and `control` steps prefetch Harvey
`bd7a6982a3e4f465f8ed3da5e0c7290521514ff0`, copy it writable without `.git`
to `$R9_WITNESS_WORK/harvey`, and run the plan-01-09 gate. Both steps reuse
that scratch tree when it already exists. The gate builds the amd64 userland
and bakes the cpu image on the first step, so budget a long session.

The positive leg is the pair marker only. A shell prompt, a `tr-ok` line, one
entry alone, or a partial match does not count (D-28). If the pair never
appears, the step fails with `RIO-FAIL rio-not-serving`; if the X11 password
feed cannot find or type into the drawterm window, it fails with
`RIO-FAIL dt-feed-timeout`. Those are real answers — return the log.

### The `rio-diag` diagnostic step (plan 01-32, stop A2)

Stop A's rio log could not say whether the X11 password feed failed or the
login worked and rio never served. `rio-diag` answers that question without
weakening the D-28 marker:

- The gate patches the scratch Harvey tree's staged cpu profile
  (`build/x86_64/root-tree/usr/glenda/lib/profile`) to create two `#s`
  service entries immediately before `exec rio`:
  `/srv/r9-login-ok` and `/srv/r9-login-pid.<pid>` (the profile shell's
  pid rides the name). The original profile is restored on every exit
  path the gate controls. The entries are not regular files: `#s` create
  is write-only and its write expects an fd number, so an unposted entry
  is listed by `ls /srv` but cannot be `cat`ed — the entry **name** is
  the payload.
- After the session window the driver parses the serial console's
  `ls /srv`. `RIO-DIAG login-ok` means the authenticated profile reached
  its rio handoff; `RIO-FAIL login-not-reached` means the X11 feed never
  reached the profile. The unchanged pair poll then prints
  `RIO-SERVING` or `RIO-FAIL rio-not-serving` in the same run, so the
  result separates **LOGIN-FAIL** (marker absent) from
  **RIO-START-FAIL** (marker present, pair absent).
- Before typing and after Return the driver records the drawterm window
  id/geometry/focus and `xprop`/`xwininfo` into `out/rio-diag/x11.txt`,
  plus one PNG each: `drawterm-before-typing.png` and
  `drawterm-after-return.png`. A missing capture tool is a named
  `RIO-FAIL diag-capture` failure — the diagnostic never runs without
  evidence. No screenshot is taken while input is on screen, and
  raw-mode `readcons` never echoes the password.

**It is not a gate.** The step's exit code is the gate's: 0 only on
`RIO-SERVING`. The marker alone records no PASS, the control leg is
unchanged, and `rio-diag` is not part of the gate PASS criteria.

Stop A2 runs from the plan's pushed commit (the executor cannot push):
`git fetch origin witness-src/01-32 && git checkout FETCH_HEAD`, then
`R9_WITNESS_STEPS="rio-diag rio control a13" bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh`.

## Needs on the Linux host

- nix with flakes and the sandbox enabled;
- a user with write access to `/dev/kvm` (`DEV_KVM=writable` in `out/env.txt`);
- network access to GitHub (the r9 pull/clone, the Harvey prefetch, nix
  inputs);
- at least **30 GB free** under `$R9_WITNESS_WORK` (the Harvey userland build
  plus the image bakes);
- no root and no elevated privileges;
- the host itself needs only nix plus the usual core utilities (`bash`, git,
  `tee`, `df`, `awk`, `sed`, `find`); the gate's tools (`expect`, Xvfb,
  xdotool, qemu, drawterm, plan9port) come from the committed flake.

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

3. Run the driver from the r9 checkout root:

   ```
   cd <r9> && bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
   ```

To run a subset (for example, re-run only the control after a rio pass):

```
cd <r9> && R9_WITNESS_STEPS="control" bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh
```

Optional environment variables:

- `R9_WITNESS_STEPS="rio control a13"` — the default; space-separated
  selection from `rio-diag`, `rio`, `control`, `a13` (stop A2 uses
  `R9_WITNESS_STEPS="rio-diag rio control a13"`);
- `R9_WITNESS_WORK=/some/path` — change the work root (absolute path);
- `KERNDATE=1795382101` — the bake timestamp; keep the default so a rerun is
  comparable.

The driver writes `out/env.txt`, one directory per step (`out/rio/`,
`out/control/`, `out/a13/`, and `out/rio-diag/` when selected),
`out/summary.tsv` (`step<TAB>exit<TAB>seconds`), then `out/survivors.txt`,
`out/secret-scan.txt` and `WITNESS-RETURN.md` beside `out/`.

## Returning the results

`WITNESS-RETURN.md` is written at
`$R9_WITNESS_WORK/WITNESS-RETURN.md` (default
`$HOME/r9-witness-work/WITNESS-RETURN.md`). It contains the git HEAD, the
preflight environment, the per-step summary, the survivor list, the
credential scan and the last 40 lines of every step log.

### Return path A (preferred): push a witness-returns branch from the Linux host

From the Linux host, in the r9 clone:

```
cd <r9>
git switch -C witness-returns/01-12
ret=.planning/phases/01-stage-p0-cross-build-harness/witness/returns/01-12
mkdir -p "$ret"
cp -a "$HOME/r9-witness-work/out" "$ret/"
cp -a "$HOME/r9-witness-work/WITNESS-RETURN.md" "$ret/"
git add "$ret"
git commit -m "witness(01-12): pass-2 rio/control/a13 returns"
git push -u origin witness-returns/01-12
```

Then report: `pushed witness-returns/01-12 at <commit sha>`. The Mac only
fetches that branch; nothing is pushed from the Mac.

For **stop A2 (plan 01-32)** the same shape uses the 01-32 names:

```
cd <r9>
git switch -C witness-returns/01-32
ret=.planning/phases/01-stage-p0-cross-build-harness/witness/return-01-32
mkdir -p "$ret"
cp -a "$HOME/r9-witness-work/out" "$HOME/r9-witness-work/WITNESS-RETURN.md" "$ret/"
git add "$ret"
git commit -m "witness(01-32): stop A2 returns"
git push -u origin witness-returns/01-32
```

`out/rio-diag/` is inside the copied `out/` tree: `rio-diag.log`, the serial
capture `rio-gate-rio-diag.log`, `x11.txt`, `drawterm-before-typing.png` and
`drawterm-after-return.png` all travel with it.

If you changed `R9_WITNESS_WORK`, substitute that path in the `cp` commands.

### Return path B: push is not possible

Paste the full `WITNESS-RETURN.md` into the session:

```
cat "$HOME/r9-witness-work/WITNESS-RETURN.md"
```

The full `out/` tree stays at the work root; path B sends the compact
summary, which is enough to record the verdicts. If a specific full log is
needed afterwards, the executor will ask for that one file.

## What never to send back

Do not add any `auth-creds.txt` (or `auth-creds`, `*.pwfile`, `dt-password`)
to the return artifacts. The driver never copies them into `out/`, and its
secret scan deletes any that appear there and any returned file containing
the per-bake DEMO password value. Only `out/` and `WITNESS-RETURN.md` belong
in a return. The password value itself is never printed by the gate, the
driver or the scan. The `rio-diag` PNGs are captured before any input and
after Return only, so they are safe to return; the secret scan deletes any
`out/` file that contains the per-bake DEMO value regardless of type.

## Cleanup on the Linux host

```
rm -rf ~/r9-witness-work
```

That removes the scratch Harvey tree, the image bakes, the 0600 password file
(if a gate ever leaves one behind) and all witness outputs. Keep the work
root until the returns are pushed or pasted.

## Notes for the next plans

Later plans add steps to `run-witness.sh`; each step is a self-contained
`step_<name>` function and adding one never edits another. Keep the step
names in `R9_WITNESS_STEPS` stable, because the return paths and the
phase summary cite them by name.
