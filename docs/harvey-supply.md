# Harvey supply: the pinned input, the Mac builders, pin bumps and local overrides

This document is the owner's and the executor's reference for how r9 obtains
Harvey: the pinned flake input, the Mac Linux-builder registration that lets
the tree derivations build on this aarch64-darwin host, the pin-bump practice,
and the session-only local override. Every owner-applied step has an exact
revert.

## 1. What the supply is

The r9 flake builds read-only Harvey trees per guest architecture from the
pinned Harvey source. The trees are nix store paths, so they are read-only by
construction; nothing in the harness ever writes into them.

- Harvey arrives as a normal flake input (`inputs.harvey.url =
  "github:requaos/harvey"`, locked by `flake.lock` to a pushed commit — the
  current pin is `bd7a6982a3e4f465f8ed3da5e0c7290521514ff0`, the pushed
  `legacy-final` commit, per D-23).
- The devshell (`nix develop`) exports `R9_HARVEY_DIR_ARM64` and
  `R9_HARVEY_DIR_AMD64`, each naming the complete tree for its architecture
  (D-21). They resolve to the flake attributes `harvey-tree-arm64` and
  `harvey-tree-amd64` (plan 01-13 ships `harvey-tree-arm64`; plan 01-14
  ships `harvey-tree-amd64`). Each tree is built lazily, only when a command
  for that architecture first needs it (D-05). `R9_HARVEY_TREE_SYSTEM` names
  the nix system the realise-on-first-use builds for — `aarch64-linux` on
  both aarch64-darwin and aarch64-linux hosts, `x86_64-linux` on
  x86_64-linux — and `R9_MKPAQFS` names the host `mkpaqfs` (attribute
  `harvey-mkpaqfs`, built on every system).
- `R9_HARVEY_DIR` remains a manual override that wins over both per-arch
  variables when set (D-01, D-21).
- xtask keeps no built-in default path (D-01): outside the devshell, with no
  variable set, the resolver fails with a named error that lists the variables
  it tried.

### Attribute names and build commands per host

The trees are `packages.<system>.harvey-tree-<arch>` on the Linux systems;
the host `mkpaqfs` is `packages.<system>.harvey-mkpaqfs` everywhere. The
devshell's `R9_HARVEY_TREE_SYSTEM` is the `<system>` in the tree attributes
it names.

| Host | Build command | Notes |
|---|---|---|
| aarch64-darwin (this Mac) | `nix build --no-link --print-out-paths .#packages.aarch64-linux.harvey-tree-arm64` | Dispatched to the registered `ssh-ng://r9-lima-builder`; the Mac cannot realise an aarch64-linux derivation locally. The amd64 tree is the same command with `harvey-tree-amd64` (the builder builds amd64 content on aarch64-linux, S4). |
| aarch64-linux | `nix build --no-link --print-out-paths .#packages.aarch64-linux.harvey-tree-arm64` | Local build. |
| x86_64-linux | `nix build --no-link --print-out-paths .#packages.x86_64-linux.harvey-tree-arm64` | Local build; the amd64 tree there is `.#packages.x86_64-linux.harvey-tree-amd64`. |
| all hosts (host tool) | `nix build .#harvey-mkpaqfs` | plan9port `9c`/`9l`; builds locally on every system. The devshell's `R9_MKPAQFS` points at it. |

Verify a tree after the build:

```sh
p=$(nix eval --raw .#packages.aarch64-linux.harvey-tree-arm64.outPath)
nix store verify --no-trust "$p"
grep -q '^arch=arm64$' "$p/HARVEY-TREE" && echo TREE-OK
```

The devshell's export is lazy: `nix develop` sends no tree build, and the
path it prints equals the one the later `nix build` produces (S2's A9 proof;
the plan 01-13 gate checks this equality).

## 2. Mac builder, primary: the lima VM `harvey` as an ssh-ng remote builder (D-24)

Why this is needed: the pass-1 S2 spike proved that `nix copy --from
ssh-ng://lima-harvey` into the Mac's system store as the owner's user is
refused (`require-sigs = true`, `trusted-users = root`; "lacks a signature by
a trusted key"). When the lima VM is registered as a remote **builder**, the
Mac's own nix daemon (running as root, a trusted user) imports the realised
build result itself, so no user-side signed copy is ever attempted. That is
the fix for S2's copy-back FAIL, and it is the only builder venue measured end
to end for Harvey's 23 case-colliding source paths (S2) and for amd64 tier
builds (S4).

References: the Nix manual's "Remote Builds" chapter
(<https://nix.dev/manual/nix/latest/advanced-topics/distributed-builds.html>)
gives the requirements (SSH access, the remote user in the guest's
`trusted-users`, a passphrase-less key because the daemon cannot prompt), and
the `builders` setting reference
(<https://nix.dev/manual/nix/latest/command-ref/conf-file.html#conf-builders>)
defines the machine-specification line format verified below:
`URI systems identity-file max-jobs speed-factor supported-features …` with
`-` for a defaulted field.

These are owner steps (stop A). The executor never applies them: the Mac's
`/etc/nix`, the nix-darwin configuration, the lima configuration and any
FlakeHub login are all owner-only surfaces (HARN-07 prohibition).

### (a) Pin the VM's ssh port

Lima assigns the VM's forwarded ssh port dynamically (it was 51721 during pass
1). A remote-builder registration needs a stable port, so pin it. Edit
`~/.lima/harvey/lima.yaml` so its `ssh:` section reads:

```yaml
ssh:
  localPort: 60022
  loadDotSSHPubKeys: true
```

Then restart the VM (`limactl` is not on `PATH`; lima 2.2.0 lives in the nix
store):

```sh
/nix/store/8szjcdjs279pd9h4d0s5dmyzjvi0lamh-lima-2.2.0/bin/limactl stop harvey
/nix/store/8szjcdjs279pd9h4d0s5dmyzjvi0lamh-lima-2.2.0/bin/limactl start harvey
```

Revert (a): remove the `localPort: 60022` line from
`~/.lima/harvey/lima.yaml` and run the same stop/start pair.

### (b) Create a root-owned key and authorise it in the guest

The Mac's nix daemon runs as root and cannot prompt for a passphrase, so the
builder key is passphrase-less and root-owned:

```sh
sudo ssh-keygen -t ed25519 -N '' -f /etc/nix/r9-lima-builder
sudo cat /etc/nix/r9-lima-builder.pub \
  | ssh -F ~/.lima/harvey/ssh.config lima-harvey \
      'mkdir -p ~/.ssh && chmod 700 ~/.ssh && cat >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys'
```

(`~/.lima/harvey/ssh.config` is lima's own generated config; after step (a)
it points at port 60022.)

Revert (b): `sudo rm -f /etc/nix/r9-lima-builder /etc/nix/r9-lima-builder.pub`
and remove the appended line from the guest's `~/.ssh/authorized_keys`:

```sh
ssh -F ~/.lima/harvey/ssh.config lima-harvey \
  'sed -i "/r9-lima-builder$/d" ~/.ssh/authorized_keys'   # the key comment defaults to the key path
```

### (c) Give root an ssh Host alias and prove the connection

The daemon connects as root, so the alias lives in root's ssh config
(`/var/root/.ssh/config` on macOS), not the owner's. Do **not** reuse lima's
`ssh.config` for the daemon: it sets `ControlMaster auto` with a
`ControlPath`, and nix's ssh-ng probe then finds lima's master socket and the
store handshake dies with `protocol mismatch, got 'started …'` (measured in
pass 1, `spikes/s1/evidence/builder-setup.txt`). The dedicated alias has no
control-socket settings, so the failure mode does not apply.

```sh
sudo tee -a /var/root/.ssh/config <<'EOF'

Host r9-lima-builder
  HostName 127.0.0.1
  Port 60022
  User neilskinner
  IdentityFile /etc/nix/r9-lima-builder
  StrictHostKeyChecking accept-new
EOF
sudo ssh r9-lima-builder true
```

`User` is the guest login user (`neilskinner` on this Mac, uid 501 in the
guest, per the pass-1 builder-setup evidence). The `sudo ssh … true` connects
once, accepts the loopback host key, and proves the alias, the key and the
port before the daemon ever tries them. The alias only ever resolves to the
lima loopback forward (127.0.0.1), never a LAN address.

Revert (c): remove the `Host r9-lima-builder` block from
`/var/root/.ssh/config`, and drop the cached loopback host key with
`sudo ssh-keygen -R '[127.0.0.1]:60022'`.

### (d) Register the builder through the nix-darwin Determinate module

`/etc/nix/nix.custom.conf` on this Mac is generated — its header says "This
custom configuration file for Determinate Nix is generated by the determinate
module for nix-darwin", so the file itself is never edited. The Determinate
nix-darwin guide (<https://docs.determinate.systems/guides/nix-darwin/>,
"Configure the Determinate Nix CLI") names the option exactly: custom settings
written to `/etc/nix/nix.custom.conf` are set with the
`determinateNix.customSettings` attribute (module 3.15.2 and later; this Mac
runs Determinate 3.21.0). Add to the owner's nix-darwin configuration:

```nix
determinateNix.customSettings = {
  builders = "ssh-ng://r9-lima-builder aarch64-linux - 4 1 big-parallel,benchmark";
  builders-use-substitutes = true;
};
```

The machine line follows the manual's format verified against the `builders`
reference linked above: URI `ssh-ng://r9-lima-builder` (the Host alias from
step (c)); system `aarch64-linux`; identity file `-` (defaulted — the alias's
`IdentityFile` supplies it); max-jobs `4` (the VM has 4 CPUs); speed-factor
`1`; supported features `big-parallel,benchmark`. `builders-use-substitutes =
true` lets the guest fetch dependencies from its own substituters instead of
waiting for the Mac to upload them.

Apply with:

```sh
darwin-rebuild switch
```

and confirm `/etc/nix/nix.custom.conf` regenerated with the two lines. The Nix
manual notes the daemon must restart for `builders` changes to take effect;
nix-darwin's activation does this, and if the probe below does not route to
the builder, restart it by hand (`sudo launchctl kickstart -k
system/org.nixos.nix-daemon` — confirm the label with `sudo launchctl list |
grep -i nix`).

Revert (d): remove the two `determinateNix.customSettings` attributes from the
nix-darwin configuration and run `darwin-rebuild switch` again; the generated
`/etc/nix/nix.custom.conf` returns to its previous contents.

### (e) Keep the guest's trusted-users line from pass 1

The pass-1 builder bring-up appended one line to the **guest's**
`/etc/nix/nix.custom.conf` and restarted the guest daemon:

```
trusted-users = root neilskinner
```

Keep it (SPIKE-RESULTS.md "Machine changes made by the spikes", change 1): the
daemon-to-daemon ssh-ng session authenticates as that guest user, and the
guest store only accepts builds from a trusted user. This is a guest-only
line; no Mac configuration carries it.

Revert (e) — only when retiring the builder venue entirely, never as part of
routine work:

```sh
ssh -F ~/.lima/harvey/ssh.config lima-harvey \
  'sudo sed -i "/^trusted-users = root neilskinner$/d" /etc/nix/nix.custom.conf && sudo systemctl restart nix-daemon.service'
```

### Verifying the registration

Build a never-substitutable derivation (the timestamp in its name guarantees
no substituter can ever serve it), as the owner, after step (d):

```sh
nix build --no-link --print-out-paths --impure --expr 'derivation { name = "r9-builder-probe-'"$(date +%s)"'"; system = "aarch64-linux"; builder = "/bin/sh"; args = [ "-c" "echo ok > $out" ]; }'
```

The Mac cannot build `aarch64-linux` locally, so the daemon forwards the build
to the registered builder and imports the result itself — the exact path S2
showed is refused for a user-side `nix copy`. Success looks like: the build
log names `ssh-ng://r9-lima-builder` as the build host, the command prints a
`/nix/store/…-r9-builder-probe-…` path, `nix path-info` on that path succeeds
on the Mac, and the file it contains reads `ok`:

```sh
out=<printed path>
nix path-info "$out"
cat "$out"   # prints: ok
```

If the probe instead fails with no available builder, the daemon has not
picked up the `builders` line — recheck that `/etc/nix/nix.custom.conf`
regenerated and restart the daemon as in step (d).

## 3. Mac builder, alternative: the Determinate native Linux builder

The alternative is Determinate's own native Linux builder
(<https://docs.determinate.systems/determinate-nix/linux-builder/>), enabled
by logging in to FlakeHub:

```sh
determinate-nixd auth login
```

No Nix configuration edits are needed: the Mac's `/etc/nix/nix.conf` already
carries the `external-builders` entry that routes Linux builds through
`determinate-nixd`, and the local daemon imports build results itself, so
S2's copy-back failure does not apply here either.

Two caveats, both recorded in pass 1:

- Its handling of Harvey's 23 case-colliding source paths is **unmeasured**,
  and Determinate's own troubleshooting page
  (<https://docs.determinate.systems/troubleshooting/native-linux-builder/>)
  lists files differing only by case as a known failure of the native builder.
  The lima venue is the only one measured end to end for them (23/23, S2).
- The login is a third-party (FlakeHub) credential on the owner's Mac; the
  pass-1 prohibition set forbids the executor from taking it, so this option
  is owner-only too.

Whichever builder is used, the tree derivations are insulated from the case
question by construction (D-24): the Harvey source reaches the builder through
a fixed-output fetch (rev plus SRI hash, the shape S2 proved with
`s2-casecheck-fod`), so the source never round-trips through the Mac's
case-insensitive APFS store, and tree outputs never contain the colliding
names.

To revert the alternative: `determinate-nixd auth logout` (the owner's own
action; pass 1 never took a login, so there is nothing to log out of today).

## 4. Pin bumps (D-06, D-23)

A Harvey pin bump is one r9 commit and lock-only. The url is branchless, so
the bump never edits `flake.nix`:

1. Prove the target commit is pushed **before** locking to it — r9 must never
   lock an unpushed Harvey commit (D-23), because nobody else can fetch it and
   CI breaks:

   ```sh
   git ls-remote https://github.com/requaos/harvey | grep <sha>
   ```

   The bump proceeds only if the commit prints (a fetch of that rev succeeding
   is equally good proof).

2. Lock to it:

   ```sh
   nix flake lock --override-input harvey github:requaos/harvey/<sha>
   ```

3. Commit `flake.lock` alone as the bump commit. Per D-14 it must pass the
   quick gate, and a pin bump also runs the full moved-gate set; both before
   the commit lands on main.

## 5. Local Harvey work (D-06)

For work against a local Harvey checkout, override the input for the session
only:

```sh
nix develop --override-input harvey path:/path/to/harvey --no-write-lock-file
```

`--no-write-lock-file` keeps the override out of `flake.lock`; such a lock is
never committed. For a tree that is already built, `R9_HARVEY_DIR` (or the
per-arch variables) can name it directly for the session — the resolver's
precedence makes the explicit value win (D-01).

## 6. Venues

- The quick gate's required witness is a local **x86_64 Linux KVM** run
  (D-13); hosted CI is a best-effort second witness.
- The Mac runs the aarch64 HVF/TCG legs, the xtask tests, and the derivation
  builds — the Linux ones through the builder registered above.

## Sources

- Nix manual, "Remote Builds":
  <https://nix.dev/manual/nix/latest/advanced-topics/distributed-builds.html>
- Nix manual, `builders` setting and machine-specification format, and
  `builders-use-substitutes`:
  <https://nix.dev/manual/nix/latest/command-ref/conf-file.html#conf-builders>
- Determinate nix-darwin guide (`determinateNix.customSettings` →
  `/etc/nix/nix.custom.conf`):
  <https://docs.determinate.systems/guides/nix-darwin/>
- Determinate native Linux builder:
  <https://docs.determinate.systems/determinate-nix/linux-builder/>
- Determinate native-builder troubleshooting (case-collision caveat):
  <https://docs.determinate.systems/troubleshooting/native-linux-builder/>
