# Rio de-risk: the DEMO credential path and drawterm's X11 password path (D-28)

Status: static findings complete on the Mac from pinned Harvey `bd7a6982a3e4f465f8ed3da5e0c7290521514ff0`.
The KVM legs that exercise the fix are **not** run here; plan 01-12 runs them on the
owner's x86_64 Linux host (D-13). Nothing in this file is gate evidence for rio
serving. All Harvey citations are `git show bd7a6982a3e4f465f8ed3da5e0c7290521514ff0:<path>`
readings; the Harvey checkout itself was not modified (HARN-02).

Scope: answer, before the r9 gate is built on the rio marker, where the generated
DEMO credential comes from (CREDENTIAL-PATH), where drawterm's password prompt reads
input under X11 versus `-G` (PROMPT-PATH), why pass 1 timed out (ROOT-CAUSE), and what
the fixed harness does (FIX), with the login identity (LOGIN-USER) and the teardown
contract (TEARDOWN). D-28 requires this ordering: credential path and headless auth
first, then re-run the rio leg with the unchanged `control-noG` control.

## The DEMO credential path and `out/auth-creds.txt`

CREDENTIAL-PATH: `authdomain-gen.c:312-333` generates the ephemeral DEMO p9sk1 domain (glenda/bootes/norbert, one shared user password, authid bootes); `authdomain-gen.c:635-647` writes `auth-creds` as `user=<last user>` + `password=<secret>`, which on the DEMO path is norbert; `mkimage.sh:1941-1943` makes that the surviving host-side `out/auth-creds.txt` after the §15 cpu bake (`mkimage.sh:1848-1857`).

The credential is generated at bake time; it is never committed and never printed by
the generator. The chain is:

- `build/x86_64/authdomain-gen.c:312-333` (`democonfig`) is the ephemeral DEMO domain:
  label `DEMO`, authdom `DEMO-harvey`, authid `bootes`, and users `glenda`, `bootes`,
  `norbert`. One password is generated into `d->users[0].pw` and copied to the other
  two (`:326-331`); the machine password and the secstore password are generated
  separately (`:324-325`). So the S3 assumption "the DEMO domain shares one password"
  is true for the three *users* — and false for the machine/secstore passwords.
- `build/x86_64/authdomain-gen.c:521-526` derives every user's DES key with
  `passtokey(u->pw)`. Because all three DEMO users share one password string, all
  three derive the same key; glenda's key therefore does derive from the same
  password the host-side creds artifact carries — from the generator source, not from
  the spike text.
- `build/x86_64/authdomain-gen.c:625-647` writes two convenience artifacts from that
  same run: `feed` (the **last** user's password plus one newline, `:625-633`) and
  `auth-creds` (`user=<last user>` then `password=<secret>`, two newline-terminated
  lines, `:635-647`). On the DEMO path the last user is `norbert` (add order at
  `:472-475`); the creds file therefore names **norbert**, not glenda.
- `build/x86_64/authdomain-gen.c:649-671` writes the separate `authid-creds`
  artifact (`user=bootes` plus bootes's own password) for the guest's factotum
  server key. It is not the client creds file.
- `build/x86_64/mkimage.sh:1533-1541` (§14 auth bake) validates and copies
  `auth-creds` to the host-side `out/auth-creds.txt`. The auth feed is staged at
  `/boot/authfeed.txt` (`mkimage.sh:1497-1507`) and copied to `out/authfeed.txt`
  (`:1520-1521`).
- `build/x86_64/mkimage.sh:1848-1857` (§15 cpu bake) re-runs the generator at bake
  time (its own per-bake domain) and `:1941-1943` overwrites `out/auth-creds.txt`.
  §15 bakes after §14, so after a full `mkimage.sh` run the surviving
  `out/auth-creds.txt` is the **cpu image's** pair — the image the gate dials. The
  two images deliberately carry different domains (`mkimage.sh:1849-1852`).
- The value itself is fresh from the host CSPRNG (`authdomain-gen.c:191-199`
  `randbytes`, fed by `/dev/urandom`), 12 alphanumeric characters
  (`authdomain-gen.c:104-105` `Demopwlen=12`, `:215-230` `genpassword`). The
  generator prints only label/authdom/authid/users, never a password
  (`authdomain-gen.c:674-676`).

Consequence for the harness: the only sanctioned source of the password is the
scratch bake's `build/x86_64/out/auth-creds.txt` (or the identical `feed` copied to
`out/authfeed.txt`). The gate parses it after the bake, before any spawn, exactly as
the pass-1 driver did (`spikes/s3/rio-gate.sh:234-246`), and must never copy that file
into a committed artifact or a log.

## `readcons` -> drawterm's own `/dev/cons` -> `kbdq`; X11 keys in, window text out

PROMPT-PATH: `readcons.c:44-59` opens drawterm's own `/dev/cons` and prints the prompt there; `main.c:74-79` maps that console to drawterm's `#c` device; `devcons.c:657-721` reads `kbdq`, fed by `echo()` in `kbdputc` (`devcons.c:291-371`, `:399-446`); X11 `KeyPress` reaches `kbdputc(kbdq, k)` at `x11.c:1111-1250`, `:1249`; the host-stdin feeder (`main.c:180-249`) starts only when `-G` is set (`main.c:50-53`). Under X11 no pty input can reach the prompt and the prompt is drawn in the window, not written to the pty (`devcons.c:364-370`, `x11.c:548-561`).

`readcons("password", ...)` does **not** read the host stdin. It opens drawterm's own
emulated console:

- `sys/src/cmd/unix/drawterm/readcons.c:44-59`: `readcons` opens `/dev/cons` for read
  and write and prints the prompt to that same descriptor. With `raw=1` it writes
  `rawon` to `/dev/consctl` and then reads one byte at a time (`readcons.c:60-107`).
- `sys/src/cmd/unix/drawterm/main.c:85-92` (`getkey`) builds the prompt
  `<user>@<authdom> password` and calls `readcons(..., raw=1)`. The authdom comes from
  the ticket request (`cpu.c:631-640`), i.e. the nvram domain `DEMO-harvey`
  (`authdomain-gen.c:322`, `:537`). The prompt on the wire is therefore
  `glenda@DEMO-harvey password: ` — not `glenda@glenda`.
- drawterm's `/dev/cons` is its internal `#c` console: `main.c:59` binds `#c` at
  `/dev`, and `main.c:74-79` opens it as fds 0/1/2.
- The console read side is `kern/devcons.c:657-721` (`consread`), which drains `kbdq`
  (raw mode) or its line discipline over `kbdq`. The produce side is `echo()`
  (`kern/devcons.c:291-371`; `qproduce(kbdq, ...)` at `:364`), called by `_kbdputc`
  (`:397-412`), called by `kbdputc` (`:414-446`).
- Under the **X11** backend the only feeder of `kbdq` is the X keyboard handler:
  `gui-x11/x11.c:1111-1250` (`xkeyboard`) handles `KeyPress`, maps `XK_Return` to
  `'\n'` at `:1193-1195`, and calls `kbdputc(kbdq, k)` at `:1249`. The event loop is
  `gui-x11/x11.c:650-682` (`xproc`, `XNextEvent` -> `xkeyboard`).
- The stdin reader exists only for `-G`: `gui-x11` is bypassed when `Gflag` is set and
  `gconsinit()` runs instead (`main.c:50-53`); `gconsinit` starts the `gcons` kproc
  (`main.c:225-249`) whose `gconsread` reads fd 0 and calls `kbdputc(kbdq, r)`
  (`main.c:180-223`). Without `-G` no host-stdin reader is started at all.
- Console **output** follows the same fork: `screeninit()` installs the window-backed
  screen output (`gui-x11/x11.c:548-561`, `terminit`), while `-G` installs
  `gconsputs` -> `write(1, ...)` (`main.c:162-167`). `consread`'s echo and `readcons`'s
  prompt reach `screenputs` (`devcons.c:364-370`), so under X11 the prompt is painted
  into the drawterm window and never appears on the pty.

Answer to the static lead: **pty input can never reach the password prompt in X11
mode.** The prompt is output to the X window, and `kbdq` is fed only by X key events;
the host-stdin reader is not started without `-G`. Under `-G` the same `readcons`
path works because `gconsread` feeds `kbdq` from the pty — which is why the pass-1
`control-noG` leg's pty feed behaved differently.

## Why the pass-1 rio leg timed out at `dt-password`

ROOT-CAUSE: the pass-1 rio mode fed the pty and waited there for a prompt that X11 never writes to the pty (`spikes/s3/rio-gate.expect:220-222`; `main.c:50-53`, `devcons.c:364-370`), and it had not parsed the generated password from the bake (`mkimage.sh:1520-1541`, `:1941-1943`); the timeout at `spikes/s3/evidence/rio-kvm-linux.log:53` is the direct consequence.

The pass-1 rio mode spawned drawterm **without** `-G` under Xvfb and then waited for
the password prompt on the spawned pty and fed the password there
(`spikes/s3/rio-gate.expect:220-222`, `feedstage dt-password`). Under X11:

- the prompt is drawn in the X window, not written to the pty (`devcons.c:364-370`,
  `x11.c:548-561`), and
- the pty is not read at all: `gconsinit`/`gconsread` run only with `-G`
  (`main.c:50-53`, `:180-249`).

So the prompt regex could never match the pty and the pty feed could never satisfy
`readcons`; the driver timed out at the feed stage. The captured log records exactly
that (`spikes/s3/evidence/rio-kvm-linux.log:53`: `TIMEOUT at feed stage: dt-password`)
and the session never produced `S3-RIO-SERVING`. The unknown generated DEMO password
was a second, independent blocker: pass 1 did not parse the bake's
`out/auth-creds.txt` / `out/authfeed.txt` (`mkimage.sh:1520-1541`, `:1941-1943`), so
even a working pty feed had no correct value to send. Both causes are fixed by FIX
and CREDENTIAL-PATH below; the marker itself is not implicated.

## The fix: XTEST keystrokes into the private Xvfb display (candidate a), password from a 0600 file

FIX: candidate (a) — on the private Xvfb display, `xdotool search --sync --classname drawterm` finds the window (drawterm sets `WM_NAME`/`WM_CLASS` at `x11.c:829-855` but never `_NET_WM_PID`), `xdotool windowfocus --sync` sets input focus (no window manager runs), and `xdotool type --file <0600 file>` types the password into the exact path the owner's manual login used (`x11.c:1111-1250`, `:1249`); candidates (b) no drawterm option/env supplies the password (`cpu.c:142-157`, `:200-211`, `:631-640`) and (c) a preloaded factotum key (`posix-factotum.c:38-104`, `cpu.c:574-577`, `:592-593`) are rejected for this plan.

Chosen mechanism: in rio mode only, the harness starts Xvfb on a private display with
`-displayfd` and `-nolisten tcp`, runs drawterm **without** `-G`, waits with a bounded
timeout for the drawterm window, focuses it, and types the password from a 0600 file
with `xdotool type --file`, then a Return. This reproduces the exact input path the
owner's manual graphical login used: X key event -> `xkeyboard` ->
`kbdputc(kbdq, k)` -> `readcons` (`x11.c:1111-1250`, `:1249`).

- Window lookup is by WM class/name, not by pid. drawterm sets `WM_NAME` `"drawterm"`
  and `WM_CLASS` res_name `"drawterm"` / res_class `"Drawterm"` (`x11.c:829-855`,
  `XSetWMProperties` at `:847`, `XMapWindow` at `:860`), but it never sets
  `_NET_WM_PID`: `x11.c` has no pid-property call. `xdotool search --pid` therefore
  cannot find this window; the harness uses `xdotool search --sync --classname
  drawterm` (equivalently `--name '^drawterm$'`) on the run's private display, where
  it is unique by construction.
- Focus is set explicitly because no window manager runs on the Xvfb display:
  `xdotool windowfocus --sync <wid>` issues `XSetInputFocus`; `windowactivate` (an
  EWMH window-manager request) is not available.
- The prompt text is in the window, so the driver cannot read it. That is safe:
  keystrokes typed after the window exists sit in `kbdq` until `readcons` consumes
  them (`devcons.c:657-721`). The driver waits for the window, a short settle, then
  types password + Return. The bounded wait and type are a named failure
  `RIO-FAIL dt-feed-timeout`.
- The password never enters an argv: the shell parses it from
  `out/auth-creds.txt`, writes it to a 0600 file inside the run's scratch dir, exports
  only that file's path to the expect driver, and the driver passes the path to
  `xdotool type --file`. The EXIT trap removes the scratch dir (and with it the
  file). The control leg reads the same file for its pty feed, so there is no
  password environment variable either.

Rejected candidates:

- **(b) a drawterm option or environment that supplies the password non-interactively
  — rejected.** `-O` sets only the dial tuple (`cpu.c:145-148`, `parseospec` at
  `:752-...`); `-u` sets the user name (`cpu.c:155-157`); `-s`/`-u` drive
  `secstorefetch` (`cpu.c:200-211`) into `secstorebuf`, whose `findkey` consumer
  (`main.c:94-138`) needs a reachable secstore server — the bootstrap cpu image runs
  none. `-k` only prefixes the `auth_proxy` key spec used with a **mounted factotum**
  (`cpu.c:142-144`, `:574-576`). With no key source, `p9any` falls through to
  `getkey`/`readcons` (`cpu.c:631-640`, `main.c:85-92`).
- **(c) a key preloaded through drawterm's own factotum/secstore path — rejected for
  this plan, recorded as the fallback.** It is technically open: `dialfactotum`
  connects to `$NAMESPACE/factotum` as a unix socket (`posix-factotum.c:38-104`), and
  `p9any` takes the factotum proxy path whenever `/mnt/factotum/ctl` opens
  (`cpu.c:592-593`, `:574-577`). But it requires a host-side plan9port `factotum`
  daemon, a NAMESPACE-derived socket, and a ctl key load — a second long-lived
  process and an unmeasured protocol path — whereas XTEST drives the same path the
  owner's manual login exercised. If the XTEST feed fails on the KVM witness, this is
  the next candidate to measure, not to assume.
- **Patching Harvey's `readcons`/GUI to read stdin, or running a second drawterm —
  rejected.** Harvey is read-only in this phase (HARN-02; the only Harvey commit is
  the later owner-approved D-10 commit), and a second drawterm would not be the gate's
  session.

## The login user

LOGIN-USER: glenda — the only staged home profile is `build/x86_64/root-tree/usr/glenda/lib/profile` (`mkimage.sh:53`; profile `:16-22`, `:58-64`), and the DEMO users share one password (`authdomain-gen.c:317-333`), so the creds file's norbert password authenticates glenda; the dial tuple is `-O cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda` and the prompt is `glenda@DEMO-harvey password: ` (`main.c:85-92`).

The headless session logs in as **glenda**, not the creds file's norbert.
`build/x86_64/root-tree/usr/glenda/lib/profile` is the only staged home profile
(`mkimage.sh:53`, staged by the chain/cpu skeleton and section P), and rcmain sources
a profile only when `$home/lib/profile` is readable; a norbert login would never
source it and the cpu case could never reach `exec rio`
(`build/x86_64/root-tree/usr/glenda/lib/profile:16-22`, `:58-64`). The DEMO users
share one password (`authdomain-gen.c:317-333`), so the norbert creds value
authenticates glenda. The dial tuple is
`cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda`; the prompt that follows is
`glenda@DEMO-harvey password: ` (PROMPT-PATH), so a regex anchored on
`glenda@glenda` would never match even on a readable wire.

## Teardown: recorded pids plus a run-identity sweep, never the argv0 tag alone

TEARDOWN: record the qemu `exp_pid`, the drawterm pty child `exp_pid` and the Xvfb `$!`, kill and `wait` each on every exit path, then sweep the user's qemu/Xvfb/drawterm processes and kill any whose `/proc/<pid>/cmdline` or `/proc/<pid>/environ` carries the run tag or the run scratch path; the argv0 tag is an extra key only, because the pass-1 survivor showed the store path instead of the tag (`spikes/S3-FINDINGS.md:154-166`).

The pass-1 gate leaked drawterm PID 88800 after its timeout, and `ps` showed the
store path rather than the `r9s3riogate-dt-*` tag, so an argv0-tag-only sweep misses
it (`spikes/S3-FINDINGS.md:154-166`). The pass-2 harness therefore records and reaps:

- the qemu pid (`exp_pid` after the spawn, plus its process group),
- the drawterm pty child pid (`exp_pid` after the drawterm spawn),
- the Xvfb pid (`$!` in the shell),
- on **every** exit path (normal, named failure, expect timeout, shell error): kill
  and `wait` each recorded pid, then sweep the user's remaining
  `qemu-system*`/`Xvfb`/`drawterm` processes and kill any whose
  `/proc/<pid>/cmdline` **or** `/proc/<pid>/environ` carries the run's unique
  identity. Each process is given one: qemu's `-name $R9_RUN_TAG` and its ISO path
  under the run scratch dir; Xvfb's `R9_RUN_ID` environment entry and its
  `-displayfd` path under the run scratch dir; drawterm's `exec -a` argv0 tag plus
  `R9_RUN_ID` in its environment. The argv0 tag remains an extra key, never the only
  one.
- the shell's EXIT trap removes the run scratch dir, which also removes the 0600
  password file.

## Not gate evidence

The pass-1 `tr-ok glenda@glenda(unknown) -> glenda@bootes` line was the owner's own
manual **graphical** drawterm login attempt made while the run appeared hung. It is
not produced by the gate's headless drawterm and must not be cited as an
authentication, an A6 confirmation, or evidence that rio is up
(`spikes/s3/evidence/rio-kvm-linux.ATTRIBUTION-NOTE.md`, `spikes/S3-FINDINGS.md:59-68`,
D-28). The s3 rio verdict remains FAIL until plan 01-12's KVM run prints
`RIO-SERVING` from the pair check below.

The gate marker is unchanged and is the **pair**: the positive leg passes only when
the serial console's `ls /srv` shows both a `rio.<user>.<pid>` entry and a
`riowctl.<user>.<pid>` entry. A shell prompt, a `tr-ok` line, one entry alone, or a
partial match never counts (D-28). On success the driver prints `RIO-SERVING`;
otherwise `RIO-FAIL rio-not-serving`. The `control-noG` leg keeps its `-G` session
and its `RIO-CONTROL-NO-RIO` / `RIO-FAIL control-saw-rio` outcomes unchanged; it is
the discriminator that proves the positive assertion is not vacuous.
