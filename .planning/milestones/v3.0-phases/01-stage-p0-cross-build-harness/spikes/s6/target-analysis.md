# D-17 replacement-target analysis (S6, Task 1)

Question: which of the shell tools the r9 boot images carry (`ls cat echo
ps rm test`) is the least entangled with the boot path and the gates, so a
`no_std` Rust probe can replace it (D-17), print a marker, and be flipped
back without weakening an existing gate?

This analysis is read-only against Harvey at the pin. Every Harvey citation
was read with `git -C /Users/neilskinner/code/harvey show
bd7a6982a:<path>`; the checkout was not edited or built in.

## Method and scope

- **Boot path (x86_64):** `build/x86_64/root-tree/rc/bin/termrc` (the chain
  image's termrc), `rc/lib/rcmain` (installed at `/rc/lib/rcmain`),
  `build/x86_64/root-tree/lib/namespace`,
  `build/x86_64/root-tree/usr/glenda/lib/profile` (only when rc runs with
  `-l`), and `build/x86_64/root-tree/rc/bin/cpurc` (cpu image only; run by
  `init -c`). The chain boot does `termrc` -> `exec rc -i` -> `rcmain`; it
  does not source the profile (`termrc:12` has no `-l`), so the profile is
  listed for completeness.
- **Gates (x86_64):** `build/x86_64/*.expect` (guest-side `send` commands
  only), `build/x86_64/*-sweep.rc`, `build/x86_64/*-termrc`, and the
  fixture scripts under `build/x86_64/root-tree/lib/*.rc`. Tcl `exec rm -f`
  lines are HOST-side cleanup and are not guest invocations.
- **aarch64:** the same boot files under `build/aarch64/` and its gates,
  because Task 2 proves the technique on that image. The virtnet terminal
  boot runs `termrc` -> `rc -i`; it does not run `cpurc` (cpu boots only).
- **Counts** below are guest-side invocation sites. A bare `echo` is an
  invocation of `/bin/echo` (see the builtin section).

## Staging: what each image actually carries

The x86_64 chain-family bakes copy the four sweep commands explicitly:

- `build/x86_64/mkimage.sh:302-309` (chain image `bootfs-chain.paq`):
  `/amd64/bin/ls`, `cat`, `echo`, `ps` are staged; `rm` and `test` are
  NOT.
- The same four names are asserted in the chain-family manifests at
  `mkimage.sh:320`, `:472`, `:616`, `:1070`, `:1182`, `:1674`, `:2043`.
- `test` is staged only in the tier/net/churn/cpu images (e.g. the churn
  manifest at `mkimage.sh:1183`, the cpu image's tier-2 desktop suite at
  `mkimage.sh:1750-1753` and its manifest at `:2046`). `rm` is harvested
  by `build/x86_64/stage-build.sh:197-201` but no x86_64 bake stages an
  `amd64/bin/rm` (grep of `mkimage.sh` finds only `test`).
- The aarch64 bake stages `echo` from `arm64/bin/echo`
  (`build/aarch64/mkimage.sh:140` `SUBSET="rc grep cat cp date echo ls
  mkdir mv ps rm sed test wc"`, copied by the `for f in $SUBSET` loop).

Consequence: `ps` is the only candidate in this set that (a) is in every
x86_64 chain-family image, (b) is absent from the boot path, and (c) has a
single gate use.

## rc's builtin table: echo is NOT a builtin

Harvey's rc builtin table is `sys/src/cmd/rc/plan9.c:35-48`:

```
builtin Builtin[] = {
    "cd", "whatis", "eval", "exec", "exit", "shift", "wait", ".", "finit", "flag", "rfork", 0
};
```

`echo` is not in it. Every bare `echo` in `cpurc`, `profile`, the sweep
scripts and the typed gate commands execs `/bin/echo` (through the
namespace's `bind /root/$cputype/bin /bin`), i.e. the replacement binary.
The comment at `build/x86_64/stage-build.sh:183` ("echo is an rc builtin
too") is stale; the builtin table is authoritative and the `ls /bin`
presence gates (`build/x86_64/smoke-r9.expect:253`, aarch64
`smoke-r9.expect:184`) prove the binary is a real file.

## ls

- Boot path: **none** (termrc, rcmain, namespace, profile, cpurc).
- x86_64 gates:
  - `build/x86_64/smoke-r9.expect:250` — typed `ls /bin`; the gate checks
    the four command names are present (`:253`).
  - `build/x86_64/tier1-sweep.rc:95` — `lsout=`{$B/hls /}`; `:96-97`
    assert `/boot` is listed.
  - `build/x86_64/services-sweep.rc:125` — `$B/hls -ld /mnt/svcscratch`;
    `:338` — `$B/hls -ld /mnt/keys/probeuser`.
- aarch64 gates: `build/aarch64/smoke-r9.expect:181`, `drive-r9.expect:144`
  (typed `ls /bin`); `tier1-sweep.rc:119` (`$B/hls /`); `trace-sweep.rc:10`
  (diagnostic).
- Staged: chain/chain-family yes.
- Verdict: gate-critical in the smoke and tier sweeps; not a good probe
  target.

## cat

- Boot path: **none**.
- x86_64 gates (guest-side):
  - `smoke-r9.expect:287` — typed `cat /lib/namespace` (asserts the known
    line at `:291`).
  - `smoke-r9-net.expect:431` and `:440` — typed `cat $d/local` /
    `cat <[0=4]` while discovering the tcp/7 conversation.
  - `root-tree/lib/churn-cats.rc:34` — `cat /lib/churn-cats.rc` per churn
    iteration (the churn gate's measured workload).
  - `tier1-sweep.rc:102,107,117,163,177,325` — `$B/hcat` for the
    sentinel/grep/wc/tr/tail/factotum legs.
  - `services-sweep.rc:104,154,300`; `auth-sweep.rc:123,142,218,251,327,332,471`.
  - `root-tree/lib/netip-dhcp.rc:72,89,115`, `netip-loopback.rc:77,102,157,173`,
    `netip-static.rc:64,81,114`, `netip-tap.rc:100,117,151,201,205,209`,
    `netip-tcp.rc:47,73,124,140,171,189,193,194,220,224`,
    `netstress.rc:147,198,202,206` (all bare `cat` or `$B/hcat`).
- aarch64 gates: `smoke-r9.expect:218`, `drive-r9.expect:181` (typed
  `cat /lib/namespace`); `smoke-r9-net.expect:338,347`;
  `tier1-sweep.rc:126,131,141,204,218,437,469,577`;
  `root-tree/lib/net-sweep.rc:49,100,104` and the netip fixtures
  (`netip-aan.rc:92,116,161,192`, `netip-dhcp.rc:67,84,110`,
  `netip-loopback.rc:67,92,148,164`, `netip-soak.rc:118,122,126`,
  `netip-static.rc:60,78,111`, `netip-tcp.rc:37,62,114,130,161,179,183,184,210,214`,
  `netip-v6.rc:58,83`, `writeprobe.rc:55`).
- Staged: yes (chain-family).
- Verdict: deeply load-bearing in every sweep; disqualified.

## echo

- x86_64 boot path:
  - `root-tree/rc/bin/cpurc:138,139` (pairing markers), `:187,188`
    (ether), `:206,207` (keyfs), `:220,221` (authsrv), `:261`
    (`$B/hecho` writing the factotum key), `:270,271` (factotum),
    `:289,290` (listen-17010), `:300,301` (DONE/FAIL sweep). The whole cpu
    bring-up marker vocabulary is echo output.
  - `root-tree/usr/glenda/lib/profile:52` — `echo -n $sysname > /dev/label`
    (guarded by `test -w /dev/label`, cpu case only).
  - termrc/namespace/rcmain: no echo.
- x86_64 gates (guest-side; extensive):
  - `smoke-r9.expect:222,307` (typed echo round-trips; the marker is the
    typed command plus the output, so a probe would change the output and
    the gate's two-occurrence stage).
  - `smoke-r9-net.expect:339,356,378,406,443,446,452,455,470`.
  - `drawterm-gate.expect:382`.
  - `tier1-sweep.rc` (83,84,91,92,... — the T1-OK/T1-FAIL vocabulary and
    every `$B/hecho` pipe), `services-sweep.rc:126,...`,
    `auth-sweep.rc:172,...`, `root-tree/lib/netip-*.rc` and
    `netstress.rc` throughout (the netip fixtures' `echo` is both output
    and, at `netip-tap.rc:191`, a `dc` input).
- aarch64 boot path:
  - `root-tree/rc/bin/cpurc:23,25,27,28,30,31,34,36,38,39,41` —
    `/arm64/bin/echo` carries the entire cpu fixture vocabulary.
  - `/lib/shebangecho` is literally `#!/arm64/bin/echo SHEBANG-OK`
    (`mkimage.sh` step 5c), so the kernel's `#!` pre-pass execs echo.
  - termrc and profile: no echo.
- aarch64 gates: `smoke-r9.expect:158,239`; `drive-r9.expect:120,202`;
  `smoke-r9-net.expect:276,299,315,350,353,359,362,371,378`; tier1 sweep
  and the netip fixtures throughout.
- Staged: yes (chain-family and aarch64).
- Verdict: **disqualified**. It is not an rc builtin, so replacing it
  rewrites cpurc's entire pass/fail ladder and every typed smoke marker.
  It is the example in D-17's prose, not a safe target.

## ps

- Boot path: **none** (termrc, rcmain, namespace, profile, cpurc).
- x86_64 gates: exactly two guest-side sites:
  - `build/x86_64/smoke-r9.expect:265` — typed `ps`; asserts at least three
    process lines and an `rc` line (`:270-283`).
  - `build/x86_64/tier1-sweep.rc:137` — `$B/hps`; asserts `rc` is resident
    (`:138-139`).
- aarch64 gates: `smoke-r9.expect:196`, `drive-r9.expect:159`,
  `tier1-sweep.rc:162` and `:607`, `trace-sweep.rc:11`.
- Staged: `/amd64/bin/ps` in the chain image (`mkimage.sh:309`) and in every
  chain-family manifest; the aarch64 image stages `ps` too
  (`mkimage.sh:140`).
- Runnable by full path: yes. The namespace binds
  `/root/$cputype/bin` onto `/bin` (`root-tree/lib/namespace:27`), so
  `/bin/ps` is the staged binary; the smoke gate's bare `ps` is the same
  exec through `path`.
- Verdict: **best candidate**. Zero boot-path use; two gate sites, both
  pure inspection; present in every x86_64 chain-family image, so no new
  staging is required to exercise it.

## rm

- x86_64 boot path: **none**. `cpurc` has no rm leg.
- x86_64 gates: **zero guest-side invocations.** Every `rm` in the x86_64
  drivers is host-side Tcl cleanup: `churn-r9.expect:104`,
  `cpu-bringup.expect:84`, `drawterm-gate.expect:142`,
  `netsoak-r9.expect:139`, `services-r9.expect:89`,
  `smoke-r9-auth.expect:117`, `smoke-r9-net.expect:176`,
  `smoke-r9.expect:125,372`, `stress-r9.expect:174`,
  `tier1-r9.expect:82`.
- aarch64: `root-tree/rc/bin/cpurc:33` (`/arm64/bin/rm '#¤/caphash'`, the
  cpu fixture's REMOVE leg), `tier1-sweep.rc:365-366` (`$B/hrm`), and
  `root-tree/lib/writeprobe.rc:73` (bare `rm` through the ramfs mount).
- Staged: **not staged in any x86_64 image today** (grep of
  `mkimage.sh` finds no `amd64/bin/rm`; only the harvest at
  `stage-build.sh:197-201`). It is not reachable by full path from the
  current chain shell.
- Verdict: lowest use count, but fails the "can still be run by full path"
  condition on today's chain image; it would require adding staging before
  it could be probed.

## test

- x86_64 boot path: `rc/lib/rcmain:19,24` (`/bin/test -r
  $home/lib/profile`, only under `flag l`; termrc's `exec rc -i` does not
  set it) and `root-tree/usr/glenda/lib/profile:43,45,47,49,51,60` (the
  cpu-case guards; the profile is not sourced at boot).
- x86_64 gates: `tier1-sweep.rc:127` (`check test $B/htest -f
  $B/hcat`); the netip fixtures' `test -e` guards
  (`netip-dhcp.rc:47`, `netip-loopback.rc:37,86,186`,
  `netip-static.rc:40`, `netip-tap.rc:78`,
  `netip-tcp.rc:32,56,100,153`, `netstress.rc:93`), and
  `churn-cats.rc:22,26` (`test -x` guards). `services-sweep.rc:299` is a
  factotum key string (`service=test`), not an invocation.
- aarch64: `cpurc:35`, `tier1-sweep.rc:151` and the netip fixtures
  (`netip-aan-drop.rc:22,50`, `netip-aan.rc:76,101,142,146,169`,
  `netip-dhcp.rc:43`, `netip-loopback.rc:22,76,177`,
  `netip-soak.rc:72`, `netip-static.rc:36`,
  `netip-tcp.rc:22,46,90,143`, `netip-v6.rc:35`, `net-sweep.rc:65`).
- Staged: not in the chain image; present in the tier/net/cpu images.
- Verdict: used by every fixture's guard and absent from the chain image;
  not a candidate.

## Recommendation: ps for D-17

Replace **`ps`** in pass 2's probe image:

1. It is already staged in every x86_64 chain-family image
   (`mkimage.sh:306-309` and the manifests), so the swap needs no staging
   change and `/bin/ps` is reachable from the serial shell.
2. It has **zero** boot-path invocations: neither termrc, rcmain,
   namespace, profile nor cpurc execs ps.
3. It has only two gate invocations (`smoke-r9.expect:265`,
   `tier1-sweep.rc:137`), both read-only process-list inspection. A D-17
   gate can run `/bin/ps` and assert the probe marker, then flip back; the
   existing smoke stage is the only place a probe image must not be
   substituted silently.
4. `echo` is explicitly disqualified: it is not an rc builtin
   (`sys/src/cmd/rc/plan9.c:35-48`), and cpurc plus every smoke marker is
   echo-driven. `cat` and `ls` are sweep-critical. `rm` is unused on x86_64
   but not staged in the chain image; `test` is not staged there either.

For Task 2's arm64 end-to-end leg the plan specifies echo deliberately
(the aarch64 virtnet boot never runs cpurc, so the cpurc echo ladder is
not on that path); the technique is identical for any of the six.

## Evidence commands

```
git -C /Users/neilskinner/code/harvey show bd7a6982a:sys/src/cmd/rc/plan9.c | sed -n '35,48p'
git -C /Users/neilskinner/code/harvey show bd7a6982a:build/x86_64/mkimage.sh | sed -n '302,320p'
git -C /Users/neilskinner/code/harvey show bd7a6982a:build/x86_64/stage-build.sh | sed -n '180,201p'
git -C /Users/neilskinner/code/harvey show bd7a6982a:build/aarch64/mkimage.sh | sed -n '130,145p'
```

The line citations above are the output of `git grep -n` against
`bd7a6982a` over the boot-path and gate file set, with comment-only lines
and host-side Tcl `exec rm` lines separated out by inspection.
