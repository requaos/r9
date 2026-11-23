# S3 — the headless rio proof and the mkimage determinism check

Question: can "reached rio" be proven without a display — a non `-G`
drawterm under Xvfb whose session makes rio post `/srv/rio.<user>.<pid>`
and `/srv/riowctl.<user>.<pid>`, observed from the r9 serial console —
and are two `mkimage.sh` bakes of one tree with a pinned `KERNDATE`
byte-identical, as pass 2's assembler differential tests (D-16) assume?
(01-RESEARCH.md, "Wave 0 Gaps", S3; D-11; Pitfall 6; Assumptions A5, A6,
A12.)

Everything below is authored and statically checked on the Mac. The
runtime legs need x86_64 Linux KVM, Harvey's native-Linux userland
build, grub/xorriso and the Linux-only drawterm package, so they are
Plan 06's owner run (D-13). Nothing in Harvey's checkout or r9's
`port/src` was edited.

## Static answers (this Mac)

VERDICT s3-static-srv-global-mac: PASS
EVIDENCE s3-static-srv-global-mac: s3/evidence/static-analysis.md (A5)

r9's srv table is a kernel-wide `static SRV` (`port/src/devsrv.rs:59`),
not a namespace field: `create` inserts into it (`:121-145`) and
`walk`/`read`/`stat` read the same table. Only the mount is namespaced
(`port/src/pgrp.rs:56-59`). The cpu image's `/lib/namespace:25` binds
`#c #s`-style `#s` at `/srv`; init's console rc reaches its namespace
through `newns(user, 0)` (`sys/src/cmd/init.c:63` → `libauth/newns.c:60`
reads `/lib/namespace`), and the cpu session does the same through
`auth_chuid` (`cpu.c:620` → `auth_chuid.c:37`). Rio posts both names
into that table (`rio/fsys.c:152-153`, `:169-170`). The source supports
seeing them from the serial console's `ls /srv`.

VERDICT s3-static-drawterm-x11-mac: PASS
EVIDENCE s3-static-drawterm-x11-mac: s3/evidence/static-analysis.md (A6)

Harvey's `nix/drawterm.nix` links the X11 backend: `buildInputs` are
libX11/libXt/xorgproto (`:29`), the merged X11 prefix is assembled at
`:37-48`, and the build runs `make CONF=unix X11=$PWD/x11` (`:51`); the
vendored `Make.unix:12-13` sets `GUI=x11` and links `-lX11`. The
`gui-none` backend exits loudly by design (`gui-none/screen.c:28`), so
there is no headless draw backend to substitute — Xvfb plus the X11
backend is the supported route.

## Runtime legs (owner's x86_64 Linux host, Plan 06)

VERDICT s3-rio-kvm-linux: FAIL
EVIDENCE s3-rio-kvm-linux: s3/evidence/rio-kvm-linux.log

VERDICT s3-norio-control-kvm-linux: PASS
EVIDENCE s3-norio-control-kvm-linux: s3/evidence/norio-control-kvm-linux.log

VERDICT s3-determinism-kvm-linux: FAIL
EVIDENCE s3-determinism-kvm-linux: s3/evidence/determinism-kvm-linux.txt

The witness ran both gates on the x86_64 host (Linux x86_64 NixOS
7.4.0-rc3, 16 CPU, 30 GiB, `/dev/kvm` writable; env:
`linux-witness/evidence/env.txt`). The cpurc ladder and the non `-G`
drawterm under Xvfb are real: the gate reached `cpurc: DONE` and spawned its
headless drawterm. **The headless session never authenticated**: the owner did
not know the generated DEMO password for glenda, and believes the headless
drawterm login attempt did not work at all. The `tr-ok glenda@glenda(unknown)
-> glenda@bootes` line in the capture came from the owner's own manual
**graphical** drawterm login attempt while the run appeared hung, NOT from the
gate's headless drawterm (owner clarification, 2026-11-23;
`s3/evidence/rio-kvm-linux.ATTRIBUTION-NOTE.md`). The driver timed out feeding
the password stage (`rio-gate: TIMEOUT at feed stage: dt-password`), so
`S3-RIO-SERVING` never appears (grep count 0) and the rio leg is a FAIL with
evidence — not a venue failure. The `-G` control passed: the serial console's `ls /srv`
shows only `/srv/boot` and `/srv/factotum`, zero cpurc: FAIL and zero
kernel-panic markers, then `S3-CONTROL-NO-RIO`. The determinism check did
not produce a comparison: bake 1 completed, but bake 2 aborted at the auth
bake with `error: auth bake: sweep subject hsecstored missing from the
computed auth-suite set` (`S3-ERROR: bake2-failed`), so none of the nine
images reached an IDENTICAL/DIFFERENT verdict.

Prerequisite: a writable scratch copy of Harvey at the phase pin with no
`.git` (the gate refuses a real checkout and any `/nix/store` path). The
commands that produced the witness results, for a pass-2 rerun, are:

```
HARVEY_TREE="$HOME/s3/harvey"
git clone https://github.com/requaos/harvey "$HARVEY_TREE"
git -C "$HARVEY_TREE" checkout bd7a6982a3e4f465f8ed3da5e0c7290521514ff0
rm -rf "$HARVEY_TREE/.git"
```

The gate's step 1 bootstraps the scratch tree's host toolchain with
Harvey's own `nix run .#setup && nix run .#toolchain` when needed, then
runs `stage-build.sh`, `subset-build.sh` and `mkimage.sh`; its flake app
resolves the r9 packages from Harvey's locked r9 pin (116bcc2) at run
time, so nothing is hand-set but `HARVEY` and `S3_OUT`.

```
cd <r9 checkout>
S3F=.planning/phases/01-stage-p0-cross-build-harness/spikes/s3

HARVEY="$HARVEY_TREE" S3_OUT="$PWD/s3-work" R9_ACCEL=kvm \
  KERNDATE=1795382101 \
  nix run "path:$PWD/$S3F#s3-rio-gate"
# expects, before the final PASS line:  S3-RIO-SERVING
# capture: s3-work/rio-gate-rio.log (password redacted by the driver)

HARVEY="$HARVEY_TREE" S3_OUT="$PWD/s3-work" R9_ACCEL=kvm \
  KERNDATE=1795382101 S3_MODE=control-noG \
  nix run "path:$PWD/$S3F#s3-rio-gate"
# expects: S3-CONTROL-NO-RIO (no rio srv entry in the -G session)

HARVEY="$HARVEY_TREE" S3_OUT="$PWD/s3-work" KERNDATE=1795382101 \
  nix develop -c bash "$S3F/determinism.sh"
# expects: six DETERMINISM <img> IDENTICAL lines, auth/cpu DIFFERENT,
# two DETERMINISM-NOTE lines, then S3-DETERMINISM-DONE
```

The determinism run consumes the `out/` left by one full gate run; run
it after the rio leg. `KERNDATE=1795382101` is the bd7a698 commit time
and is passed to BOTH gate runs so the first bake and the second bake
share the pinned timestamp; `determinism.sh` reads the value back from
the first bake's log and refuses with `bake1-kerndate-mismatch` if it
differs, so an all-DIFFERENT result cannot be blamed on the clock.

Evidence to copy back into `s3/evidence/` (all already redacted or
credential-free): `rio-gate-rio.log`, `rio-gate-control-noG.log`,
`bake.log`, `determinism-bake2.log`, `bake1.sha256`, `bake2.sha256`.
Do not copy `build/x86_64/out/auth-creds.txt` or `authfeed.txt`; the
scratch tree may be deleted after the run.

## Gate mechanics worth knowing before Plan 06

- **Login is glenda, not the creds-file user.** The creds file names the
  DEMO probe user (norbert), who has no staged home, so `rc -li` would
  never source a profile and rio could not start. Only
  `/usr/glenda/lib/profile` is staged, and DEMO gives all users one
  shared password, so the parsed password authenticates glenda.
  `S3_DT_USER` overrides the name; `rio-gate.expect` anchors the password
  prompt on it. The witness did not exercise this end to end: the owner did
  not know the generated DEMO password, so the headless feed never succeeded.
  Pass 2 must determine the generated credential path before relying on it.
- **Mac-side static checking:** the flake app evaluates on
  aarch64-darwin (`nix eval ...#apps.x86_64-linux.s3-rio-gate.program`
  prints a store path) because the r9 package paths are resolved at run
  time. r9 116bcc2's kernel derivation reads rust-src's `Cargo.lock`
  through import-from-derivation, which cannot build on the Mac; keeping
  that reference out of the app's evaluation is what lets the Mac check
  pass. The packages are still exactly Harvey's locked r9 pin, built by
  `nix build path:$S3_FLAKE#kernel-x64` etc.
- **The control is the discriminator.** `control-noG` runs the identical
  pipeline with `-G`; if it ever reports `S3-FAIL control-saw-rio`, the
  rio-mode assertion is not specific and must not be trusted.
- **A12's split.** The six non-auth images must be byte-identical; the
  auth and cpu images embed a fresh bake-time p9sk1 domain every run
  (`mkimage.sh:1290-1305`, `:1848-1856`), so `determinism.sh` compares
  their bake-log member lists and sizes instead.

## Return hygiene and gate teardown

- **Credential scan is clean.** The returned `out/secret-scan.txt` records
  no `auth-creds` artifact and no DEMO password value in `out/`. The
  committed excerpts were rescanned before commit; every password-named
  line carries a `<redacted>` marker and no value.
- **A drawterm survived the rio-gate timeout.** `out/survivors.txt` lists
  PID 88800 (`.../drawterm -s '' -O cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda`)
  alive after the gate returned FAIL. `ps` shows the store path as argv[0],
  not the `r9s3riogate-dt-*` tag the gate sets with `exec -a`, so a
  tag-only teardown sweep misses it. Pass 2's gate must reap by the
  scratch/dial identity or the pty child pid, not by the argv0 tag alone.
  This is a gate-teardown finding for pass 2.

## Assumptions

ASSUMPTION A5: CONFIRMED - the serial console in the `-G` control lists `/srv` from its own namespace (`/srv/boot`, `/srv/factotum`), confirming the console reads the kernel-wide srv table. The positive rio posting was not reached because the gate's headless drawterm never authenticated (the generated DEMO password was not known), so visibility is confirmed while a live rio session remains unproven. Evidence: s3/evidence/norio-control-kvm-linux.log.
ASSUMPTION A6: UNTESTED - Harvey's drawterm is statically confirmed to link the X11 backend (static PASS) and the gate's headless drawterm starts under Xvfb, but no headless authentication completed because the generated DEMO password for glenda was not known; the `tr-ok glenda@glenda(unknown) -> glenda@bootes` line was the owner's manual graphical login attempt, not the gate's headless session. Evidence: s3/evidence/rio-kvm-linux.log and s3/evidence/rio-kvm-linux.ATTRIBUTION-NOTE.md (owner clarification 2026-11-23).
ASSUMPTION A12: REFUTED - the two-bake check cannot pass on this tree: bake 1 completes, bake 2 aborts at the auth bake (`hsecstored missing from the computed auth-suite set`), so no image is compared. Byte-identity of the six non-auth images is therefore unproven and the second bake is not re-runnable as-is. Evidence: s3/evidence/determinism-kvm-linux.txt.

## What this means for planning pass 2

- **The `-G` control is green; the rio leg needs its headless credential
  path fixed before D-11's marker can be trusted.** The control proves the
  discriminator (serial `ls /srv` shows no rio entry in a `-G` session,
  with zero cpurc: FAIL and zero panic markers). The gate's headless
  drawterm never authenticated because the generated DEMO password for
  glenda was not known; the `tr-ok` line was the owner's manual graphical
  attempt and is not gate evidence. Pass 2 should determine the generated
  DEMO password/credential path, fix the headless drawterm auth and the
  password feed, re-run rio mode, and keep `control-noG` unchanged as the
  negative control; the marker design itself is not invalidated.
- **A12 must be resolved before the assembler differential test is
  written.** The second bake aborts at the auth bake with
  `hsecstored missing from the computed auth-suite set`, so the six-image
  byte comparison never ran. Pass 2 cannot plan a whole-image byte
  differential until `mkimage.sh` can re-bake the tree; investigate the
  auth-suite resolver first. The auth/cpu member-list fallback stays the
  design for those two images, but it is moot until bake 2 completes.
- **Gate teardown must not rely on the argv0 tag.** PID 88800 drawterm
  survived the timeout and `ps` showed the store path rather than the
  `r9s3riogate-dt-*` tag; reap by the scratch/dial identity or the pty
  child pid instead (see "Return hygiene and gate teardown").
- **The `-G` control's no-sudo venue is worth keeping.** The control's
  stage build ran in an unprivileged user namespace (`unshare -Ur -m`
  chroot scaffold) outside the nix sandbox, which is the path pass 2 should
  keep for the Linux bake when the host has no passwordless sudo.
- **The gate's home after the flake flip:** the rewiring here is
  deliberately an `apps.x86_64-linux` app over Harvey's locked inputs,
  matching the shape the moved `drawterm-gate-x64` app will take. Pass 2
  can lift `S3_MODE=rio` into r9's gated app and keep `control-noG` as
  its negative control.
