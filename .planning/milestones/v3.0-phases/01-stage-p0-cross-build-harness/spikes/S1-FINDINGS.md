# S1 — the sandboxed build venue

Question: can Harvey's namespace build run inside a nix build sandbox with
no sudo — nested `unshare -Ur -m`, a private tmpfs scaffold, leaf binds,
a chroot, plan9port `rc`/`mk` — and does real work (Harvey's amd64 `libc`)
build in it, so that D-03's r9-owned tree derivation is feasible?
(01-RESEARCH.md "Wave 0 Gaps" S1; Assumption A1; Pitfall 2; D-03; D-23.)

BUILDER-VENUE: lima (owner, 2026-11-22)

The owner chose the existing lima VM `harvey` over the Determinate native
builder, so no FlakeHub login was taken and no Mac configuration changed.
The VM is aarch64-only, so every leg here is `aarch64-linux` building Plan 9
`amd64`/`arm64` targets through Harvey's cross toolchain. Bring-up transcript
(commands and outputs, no credential material): `s1/evidence/builder-setup.txt`.

## Rung 1 — the venue itself

VERDICT s1-rung1-lima-aarch64: FAIL
EVIDENCE s1-rung1-lima-aarch64: s1/evidence/rung1-builder.log

The `unshare -Ur -m` venue cannot start in this builder:

```
unshare: write failed /proc/self/uid_map: Operation not permitted
```

It fails twice over, and both are measured:

1. **Inside the nix build sandbox** the build already runs in a user
   namespace as an unprivileged uid (`id -u` = 1000,
   `/proc/self/uid_map` = `1000 30001 1`), so it holds no CAP_SETUID over
   its own namespace and cannot write a nested `uid_map`. The sandbox itself
   is real: `/usr/bin/env` is absent and `/proc/self/ns/user` is a
   `user:[…]` namespace, exactly as the venue's guard requires.
2. **Outside the sandbox, on the guest at all**, as the ssh user:
   `unshare -Ur -m true` returns the same EPERM. The guest is Ubuntu 24.04
   with `kernel.apparmor_restrict_unprivileged_userns=1` (kernel 6.8.0-147,
   `user.max_user_namespaces=31405`, `kernel.unprivileged_userns_clone=1`).
   This is Assumption A16's AppArmor restriction, measured live.

VERDICT s1-rung1-proot-lima-aarch64: PASS
EVIDENCE s1-rung1-proot-lima-aarch64: s1/evidence/rung1-builder.log

The Pitfall 2 fallback (a), `proot`, does the same work with no capabilities
and no `uid_map`: `proot -0 -r <root> -b <work>/sys:/sys …` fakes the chroot
with ptrace, the guest sees uid 0, and all three probes pass:

```
S1-RUNG1-SYS-OK      write a file under /sys inside the root and read it back
S1-RUNG1-RC-OK       plan9port /bin/rc -c 'echo …'
S1-RUNG1-MK-OK       plan9port mk on a two-line mkfile created on the tmpfs
```

Sandbox evidence for the passing leg (printed before the venue is entered):

```
SANDBOX-EVIDENCE id-u=1000
SANDBOX-EVIDENCE uid-map=      1000      30001          1;
SANDBOX-EVIDENCE user-ns=user:[4026532421]
SANDBOX-EVIDENCE uname=Linux localhost 6.8.0-147-generic … aarch64 GNU/Linux
SANDBOX-EVIDENCE usr-bin-env=absent
```

## Rung 2 — Harvey's host toolchain and the amd64 libc

VERDICT s1-hostbin-lima-aarch64: PASS
EVIDENCE s1-hostbin-lima-aarch64: s1/evidence/rung2-builder.log

Harvey's `nix/setup.sh` + `nix/toolchain.sh` run unmodified in a nix build
(no namespace needed). The tool set matches the Mac's hostbin listing:

```
6a 6c 6l 7a 7c 7l 8a 8c 8l cpp nm ar pcc
(+ awk basename data2s date dd lex ls mklatinkbd pcc rm sed size yacc)
```

Four sandbox ports were needed, each measured:

- `9l` shim script: shebang `/usr/bin/env bash` does not exist in the nix
  sandbox; use the store's bash path.
- `ed` is not in the sandbox (Harvey's devshell inherited the host's
  `/usr/bin/ed`); `7l/mkcname` needs it, so `pkgs.ed` is an input.
- plan9port goes **last** on PATH, as Harvey's own flake wrapper does
  ("plan9 bin LAST per BUILD_ON_UNIX — plan9port ships a 9p `mount` which
  must not shadow util-linux mount"); hostbin stays first.
- `7l/BUILD`'s `_SYSRM` line is malformed at the pin (its sed script ends
  `s/::*$` with no replacement), so the pinned plan9port sed answers
  `Too much text`; GNU sed shadowing the bootstrap turns that into the real
  syntax error the line tolerates via `|| echo rm`. The following `_HOSTNM`
  line is well formed and needs GNU sed to work.

VERDICT s1-rung2-lima-aarch64: PASS
EVIDENCE s1-rung2-lima-aarch64: s1/evidence/rung2-builder.log

`cd /sys/src/libc && objtype=amd64 mk install` inside the proot venue, with
the hostbin first on PATH, produces:

```
S1-RUNG2-LIBC-SECONDS=2
--rw-r--r-- M 0 0 0 601170 /amd64/lib/libc.a
S1-RUNG2-LIBC-MEMBERS=259
S1-RUNG2-LIBC-OK
```

and `nix store verify --no-trust` over the output (run on the builder) is
clean — the HARN-02 "tree unmodified" proof method works on a tree-like
output.

## Commands

```
S=.planning/phases/01-stage-p0-cross-build-harness/spikes
export NIX_SSHOPTS="-F /Users/neilskinner/.lima/harvey/ssh.config -oControlPath=none"

# rung 1, the venue (fails: nested userns refused)
nix build "path:$S/s1#packages.aarch64-linux.s1-nsprobe" \
  --store ssh-ng://lima-harvey --eval-store auto -L --no-link --print-out-paths

# rung 1, the same guest script through proot (passes)
nix build "path:$S/s1#packages.aarch64-linux.s1-nsprobe-proot" \
  --store ssh-ng://lima-harvey --eval-store auto -L --no-link --print-out-paths

# rung 2
nix build "path:$S/s1#packages.aarch64-linux.s1-hostbin" \
  --store ssh-ng://lima-harvey --eval-store auto -L --no-link --print-out-paths
nix build "path:$S/s1#packages.aarch64-linux.s1-libc-amd64" \
  --store ssh-ng://lima-harvey --eval-store auto -L --no-link --print-out-paths

# store integrity of the libc output (HARN-02 method)
nix store verify --no-trust --store ssh-ng://lima-harvey \
  /nix/store/lkix2l2k7fcm9gv5cdwm47arg4l49fqw-s1-libc-amd64-0

# x86_64-linux attributes evaluate on the Mac (Plan 06 builds them)
nix eval --raw "path:$S/s1#packages.x86_64-linux.s1-libc-amd64.drvPath"
```

`-oControlPath=none` is required with lima's `ssh.config`: it sets
`ControlMaster auto`, so nix's `ssh -O check lima-harvey` probe finds lima's
master socket, skips its own master start, and the daemon handshake dies with
`protocol mismatch, got 'started … oixd'`. Details in
`s1/evidence/builder-setup.txt`.

## The recipe that worked

1. A nix derivation (`spikes/s1/venue.nix`, `stdenvNoCC`) copies the Harvey
   subdirectories the build reads (`sys amd64 arm64 rc lib build`) into a
   writable `$TMPDIR/work`, `chmod -R u+w`, then runs Harvey's own
   `build/mkdirs` on the copy. The store path is never bound writable.
2. Before any venue it prints the `SANDBOX-EVIDENCE` block and **fails if
   `/usr/bin/env` exists**, so a non-sandboxed build can never be recorded as
   a pass.
3. The venue is `proot -0 -r <root> -w /` with leaf `-b` mappings:
   `$work/{sys,amd64,arm64,rc,lib,out}`, `/nix/store`, a private tmp dir as
   `/tmp`, and the four `/dev` nodes one at a time. `<root>/bin` is a real
   directory of symlinks (`sh`/`bash` to the store's bash, `rc`/`mk` to
   plan9port). The guest script runs as `/bin/sh /guest.sh`.
4. `PATH` carries the hostbin first and plan9port's `plan9/bin` last, as
   Harvey's flake wrapper does; nix's `plan9port` package otherwise puts only
   its `9` wrapper on `PATH`.

The unshare variant of the same file is kept (and is what the FAIL leg ran);
switching venues is the single `mode` argument, so the guest script does not
change.

## Assumptions

ASSUMPTION A1: REFUTED - the unshare venue is refused on both builders:
nested `uid_map` write is EPERM inside the lima nix sandbox and on its
Ubuntu 24.04 guest, and on the x86_64 witness the private tmpfs comes up but
the `/nix/store` bind fails (`wrong fs type, bad option, bad superblock`;
guest exit 32). The proot venue runs the same guest work on both (lima
aarch64 and the witness x86_64: 601170 bytes, 259 members, `S1-VENUE-OK`),
so D-03 is not blocked — but an unshare-based derivation is. Evidence:
s1/evidence/witness-x86_64-linux.log.
ASSUMPTION A8: REFUTED - the amd64 libc alone builds in 2 s on the lima
guest (259 members, 601170 bytes). "Tens of minutes per arch" belongs to the
full tier build (hundreds of command links), not to the library tier.
ASSUMPTION A16: UNTESTED - hosted CI itself was not run. The evidence is the
same class of host: the lima Ubuntu 24.04 guest with
`kernel.apparmor_restrict_unprivileged_userns=1` refuses unprivileged userns
creation outright, so a userns tree derivation needs the proot venue there
too.

## Witness legs (owner's x86_64 Linux host, Plan 06)

VERDICT s1-witness-rung1-x86_64-linux: FAIL
EVIDENCE s1-witness-rung1-x86_64-linux: s1/evidence/witness-x86_64-linux.log

VERDICT s1-witness-rung2-x86_64-linux: PASS
EVIDENCE s1-witness-rung2-x86_64-linux: s1/evidence/witness-x86_64-linux.log

The same flake evaluated for `x86_64-linux` and ran on the owner's witness
(Linux x86_64 NixOS 7.4.0-rc3, 16 CPU, 30 GiB, `/dev/kvm` writable, nix
2.35.6 with `sandbox = true`; `linux-witness/evidence/env.txt`). Rung 1
failed at a different point than on lima: the private tmpfs
(`S1-VENUE-NSX=/build/nsroot.nHa578`) was created, but the bind of
`/nix/store` into it was refused (`wrong fs type, bad option, bad
superblock`), so the guest exited 32 (`S1-VENUE-RC=32`) without running.
Rung 2, through proot, built the same amd64 libc on x86_64: 601170 bytes,
259 members, `S1-RUNG2-LIBC-OK`, `S1-VENUE-OK` (4 s). The sandbox guard
held (`usr-bin-env=absent`).

## What this means for planning pass 2

- **D-03's venue is proot on both measured hosts, not `unshare`.** Rung 1
  failed on lima (nested `uid_map` EPERM) and again on the x86_64 witness
  (the private tmpfs starts but `/nix/store` will not bind into it). Rung 2
  built the amd64 libc through proot on both (601170 bytes, 259 members), so
  pass 2's tree derivation should use the proot venue and must not depend on
  unshare or on a privileged builder. The witness run also rules out
  "x86_64 native is different": the D-03 venue question is answered the same
  way on both architectures. If pass 2 wants the unshare option, it needs a
  builder where the derivation owns a user namespace with CAP_SETUID, which
  neither measured venue provides.
- **The measured number to plan against:** the amd64 library tier is seconds,
  not minutes; A8's "tens of minutes" is the whole tiered userland build.
  Pass 2 should keep `lib.fileset` filtering so pin bumps that do not touch
  the build do not re-pay it.
- **The host-toolchain port needs the four sandbox adjustments above** (`ed`,
  the `9l` shebang, plan9port last on PATH, GNU sed for the bootstrap). They
  belong in the real derivation, not just the spike; none of them is a sudo
  path.
- **Tree outputs must exclude the 23 case-colliding Harvey paths**
  (Pitfall 3). The venue copies whole subtrees, so a pass-2 derivation that
  installs `rc/`, `sys/lib/troff/` or `sys/lib/postscript/` wholesale would
  ship the mangled `~nix~case~hack~1` names back to the Mac store. Copy only
  what the images and ureg checks consume, and assert the installed set has
  no `~nix~case~hack~` name.
