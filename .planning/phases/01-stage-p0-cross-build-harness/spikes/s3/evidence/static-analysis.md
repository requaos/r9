# S3 static analysis — A5, A6, the cpu profile guard, and A12's premise

Read-only: r9's sources at HEAD and Harvey `bd7a6982a` through
`git -C /Users/neilskinner/code/harvey show`. Nothing in either checkout
was edited, built or run for this file.

## A5 — is r9's srv table kernel-wide, and is it visible from the serial console?

**The table is kernel-wide.** `port/src/devsrv.rs:59` declares

```rust
static SRV: Lock<(Vec<SrvEntry>, u64)> = Lock::new("devsrv", (Vec::new(), 1));
```

— one static for the whole kernel, not a field of `Pgrp` and not reached
through a `Chan`. `create` locks that static and pushes the entry
(`port/src/devsrv.rs:121-145`, the push at `:138`), and `walk`/`read`/
`stat` all consult the same static (`:98-119`, `:257-283`, `:285-299`).
By contrast the *mount* table is per namespace group:
`Pgrp { mounts: Vec<MountPoint>, ... }` at `port/src/pgrp.rs:56-59`
(and `MountEntry` at `port/src/dat.rs:88-99`). A namespace therefore
decides where `#s` is mounted, not which srv entries exist.

**The cpu image binds `#s` at `/srv`.**
`harvey@bd7a698 build/x86_64/root-tree/lib/namespace:25` carries
`bind -c #s /srv`, after the re-mount pair at lines 17-18 and the other
device binds at 21-24.

**Init's console rc uses that namespace.** `sys/src/cmd/init.c:63`
calls `newns(user, 0)` before either `cpustart` or the interactive rc
handoff; `sys/src/libauth/newns.c:60` reads `/lib/namespace`, and
`:68` performs `rfork(RFENVG|RFCNAMEG)` before applying it. The console
path then reaches `rcexec` (`init.c:171-181`) — for the cpu service it
is `execl("/bin/rc", "rc", nil)`, an interactive rc over the serial
console. Its `/srv` is therefore the `#s` of the image's
`/lib/namespace`.

**The cpu session uses the same namespace file.** `cpu -R` execs
`rc -li` (`sys/src/cmd/cpu.c:370`); the post-auth server path calls
`auth_chuid(ai, nil)` (`cpu.c:620`), whose `newns(ai->cuid, ns)` call
is `sys/src/libauth/auth_chuid.c:37` (with `ns == nil`, `newns.c:60`
again selects `/lib/namespace`). So the session in which rio runs has
`#s` at `/srv` too.

**Rio posts into that table.** `sys/src/cmd/rio/fsys.c:152-153`
creates `/srv/riowctl.<user>.<pid>` and posts `wctl`; `:169-170` creates
`/srv/rio.<user>.<pid>` and posts `wsys`. Both go through the `#s`
device in rio's namespace, i.e. the one kernel-wide table above.

**A5 verdict: PASS** (static). The source supports seeing both entries
from the serial console's `ls /srv`. The runtime confirmation is the
Plan 06 KVM leg `s3-rio-kvm-linux` (`PENDING-OWNER`).

## A6 — does Harvey's drawterm package build the X11 backend?

`harvey@bd7a698 nix/drawterm.nix:9-11` states that the gui-x11 backend
is linked, and the derivation does exactly that: `buildInputs` at
`:29` are `libX11`, `libXt` and `xorgproto`; `:37-48` assembles a
merged X11 prefix; `:51` runs `make CONF=unix X11=$PWD/x11`. The
vendored `sys/src/cmd/unix/drawterm/Make.unix:12` sets `GUI=x11` and
`:13` links `-lX11`. So the package binary can serve `/dev/draw` under
an X server.

The alternative is not a headless draw backend: `gui-none/screen.c:28`
(`noscreen`) prints `gui-none: ... called but this build has no screen;
use -G` and exits 1, and the file's header (`:1-8`) says every screen
entry point fails loudly by design. A `-G` session has no `/dev/draw`
for the cpu profile's guard to see.

**A6 verdict: PASS** (static). The runtime leg is the same Plan 06
`s3-rio-kvm-linux`; the `-G` control is `s3-norio-control-kvm-linux`.

## The cpu profile's rio guard, exactly

`build/x86_64/root-tree/usr/glenda/lib/profile`:

- `:30` `switch($service)`, `:34` `case cpu`.
- `:42` starts `wsysup=()`; `:43-46` set it when either
  `/mnt/term/mnt/wsys` (the stock terminal-side rio idiom) or
  `/mnt/wsys` (a remote rio already mounted) exists.
- `:56` sets the `cpu% ` prompt; `:58` enters the start block only when
  `~ $#wsysup 0`; `:60` tests `test -e /mnt/term/dev/draw`; `:61-62`
  run `plumber` then `exec rio`.

So `exec rio` happens only when **all** hold:

1. the shell is `rc -li` as a user whose `$home/lib/profile` is readable
   (`rc/lib/rcmain:19` and `:23-24` source it only under `flag l`;
   `cpu.c:370` is the `-li` exec), with `$service` equal to `cpu`;
2. no desktop is already present (`$wsysup` unset);
3. the client serves a draw device at `/mnt/term/dev/draw`.

It is skipped when any of these is false: a non-cpu service; an
existing `/mnt/term/mnt/wsys` or `/mnt/wsys`; a `-G` (or otherwise
draw-less) session; a login shell without `-l`; or a user with no
readable `$home/lib/profile`.

That last condition decides S3's login: the cpu image stages only
`/usr/glenda/lib/profile` (`mkimage.sh`'s chain skeleton copy). The
bake's `out/auth-creds.txt` (`<redacted>`: filename only) names the DEMO probe user (norbert), who
has no staged home, so `rc -li` as norbert exits the guard cleanly and
never starts rio. The DEMO domain gives glenda, bootes and norbert one
shared password (`<redacted>`: prose only; `build/x86_64/authdomain-gen.c:313-329`, `:473-475`),
so the parsed password (`<redacted>`: prose only) authenticates glenda. `rio-gate.sh` therefore
dials `user=glenda` (`S3_DT_USER`) and keeps the creds user only for
the password (`<redacted>`: prose only). The `-G` control is meaningful precisely because it
removes condition 3 while keeping 1 and 2.

## A12 — what the determinism claim rests on

`build/x86_64/mkimage.sh:105-110` states the determinism contract: the
staged tree's mtimes are pinned to `KERNDATE` and mkpaqfs's header time
rides the same value, so two bakes of one tree are byte-identical; the
staging order is fixed by construction. The implementation is at
`:243-250` for the marker image and repeats at `:338-344`, `:497-500`,
`:639-642`, `:1106-1119` and the other sections.

The exception is explicit: `mkimage.sh:1290-1305` says the auth image
is **not** byte-reproducible across bakes (a fresh p9sk1 domain is
generated at bake time), and `:1848-1856` does the same for the cpu
image with its own separate domain ("one image per gate"). A12 is
therefore testable as stated for the six non-auth images, and the
auth/cpu pair must be compared by member list and size, not bytes.
`s3/determinism.sh` implements exactly that split.
