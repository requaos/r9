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

VERDICT s3-rio-kvm-linux: PENDING-OWNER
VERDICT s3-norio-control-kvm-linux: PENDING-OWNER
VERDICT s3-determinism-kvm-linux: PENDING-OWNER

Prerequisite: a writable scratch copy of Harvey at the phase pin with no
`.git` (the gate refuses a real checkout and any `/nix/store` path). On
the owner's host:

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
  prompt on it.
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

## Assumptions

ASSUMPTION A5: CONFIRMED STATICALLY — the srv table is kernel-wide and both the console rc and the cpu session bind `#s` at `/srv`; the KVM run is the remaining confirmation (Plan 06).
ASSUMPTION A6: CONFIRMED STATICALLY — Harvey's drawterm package builds the X11 backend (`Make.unix` `GUI=x11`), so Xvfb is a valid display for a non `-G` session; the KVM run is the remaining confirmation (Plan 06).
ASSUMPTION A12: UNTESTED - runs in Plan 06 (the six deterministic images must be byte-identical across two pinned-KERNDATE bakes; auth/cpu must differ only in the bake-time domain).

## What this means for planning pass 2

- **D-11's marker design is settled pending one KVM run.** The quick
  gate's rio assertion can be: non `-G` drawterm under Xvfb, then
  `ls /srv` on the serial console for `rio.<user>.<pid>` **and**
  `riowctl.<user>.<pid>`, with a `-G` control that must see neither
  rio name. The marker is a text assertion, so the pass-2 gate needs no
  pixel check and no second drawterm session.
- **If A5 fails at runtime** (entries not visible despite the source
  reading), the fallback ladder from the research is: (1) type
  `ls /srv` in the *cpu session's* own rc before rio starts, from the
  drawterm pty; (2) `echo RIO-SERVING > '#c/cons'` after a
  `test -e /srv/rio.*` in the session; (3) as a last resort, an
  `xwd -root` dominant-colour check on Xvfb. The first two keep the
  marker textual.
- **If A6 fails at runtime** (the package cannot serve draw under
  Xvfb), there is no automated rio proof in this phase; the checkpoint
  must record the weaker evidence and pass 2 must decide between a
  packaged display server and accepting the human-accepted desktop
  screenshot as the rio proof. That would be a locked-decision change
  (D-11), not a plan detail.
- **If A12 fails** for a non-auth image, pass 2's differential test
  must compare member lists, sizes and per-member hashes instead of
  whole-image bytes. The auth/cpu split already assumes that fallback;
  a failure on a supposedly deterministic image would also mean the
  `boot-iso` FOD cannot inherit the bake, so it must be resolved before
  the assembler's byte-comparison plan is written.
- **The gate's home after the flake flip:** the rewiring here is
  deliberately an `apps.x86_64-linux` app over Harvey's locked inputs,
  matching the shape the moved `drawterm-gate-x64` app will take. Pass 2
  can lift `S3_MODE=rio` into r9's gated app and keep `control-noG` as
  its negative control.
