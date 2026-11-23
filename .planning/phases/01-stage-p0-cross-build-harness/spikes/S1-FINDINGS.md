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

## Commands

```
S=.planning/phases/01-stage-p0-cross-build-harness/spikes
export NIX_SSHOPTS="-F /Users/neilskinner/.lima/harvey/ssh.config -oControlPath=none"

# the venue, first try (fails: nested userns refused)
nix build "path:$S/s1#packages.aarch64-linux.s1-nsprobe" \
  --store ssh-ng://lima-harvey --eval-store auto -L --no-link --print-out-paths

# the same guest script through proot (passes)
nix build "path:$S/s1#packages.aarch64-linux.s1-nsprobe-proot" \
  --store ssh-ng://lima-harvey --eval-store auto -L --no-link --print-out-paths

# x86_64-linux attribute evaluates on the Mac (Plan 06's witness run builds it)
nix eval --raw "path:$S/s1#packages.x86_64-linux.s1-nsprobe.drvPath"
# -> /nix/store/ny7zir5kilsg1sb88fx655fcp1dqs77a-s1-nsprobe-0.drv
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
4. `PATH` carries plan9port's `plan9/bin` first: nix's `plan9port` package
   puts only its `9` wrapper on `PATH`, and Harvey's mkfiles call `rc`, `mk`,
   `sed`, `ls`, `rm` by name.

The unshare variant of the same file is kept (and is what the FAIL leg ran);
switching venues is the single `mode` argument, so the guest script does not
change.

## Assumptions

ASSUMPTION A1: REFUTED - `unshare -Ur -m` cannot start inside this nix build
sandbox (nested `uid_map` write is EPERM), and not even outside it on the
Ubuntu 24.04 guest. The proot venue runs the same guest work with no
capabilities, so D-03 is not blocked — but an unshare-based derivation is.
ASSUMPTION A16: UNTESTED - hosted CI itself was not run. The evidence is the
same class of host: the lima Ubuntu 24.04 guest with
`kernel.apparmor_restrict_unprivileged_userns=1` refuses unprivileged userns
creation outright, so a userns tree derivation needs the proot venue there
too.

## Witness legs (owner's x86_64 Linux host, Plan 06)

VERDICT s1-witness-rung1-x86_64-linux: PENDING-OWNER
VERDICT s1-witness-rung2-x86_64-linux: PENDING-OWNER

The same flake evaluates for `x86_64-linux` on the Mac, so Plan 06's witness
run builds the identical derivations on the owner's KVM host.

## What this means for planning pass 2

- **D-03's venue is viable, but not through `unshare`.** Pass 2's tree
  derivation must either (a) use the proot venue measured here, or (b) get a
  builder where the derivation itself runs with CAP_SETUID in a namespace it
  owns. Option (a) costs proot's ptrace overhead (measured in rung 2); option
  (b) is a D-04 machine decision (the native Linux builder's VM may have the
  same AppArmor default) and is not assumed.
- **The sandbox-evidence guard belongs in the real derivation.** The
  `/usr/bin/env`-must-be-absent assertion plus the `uid_map`/`ns/user` lines
  are how a future run proves it was sandboxed; without them a proot venue
  silently proves nothing about the sandbox.
