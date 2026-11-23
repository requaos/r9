# Linux KVM witness bundle — S1, S5, S3 and S6 (Plan 06)

This bundle is the owner's one-session run of every spike leg that needs an
x86_64 Linux host with KVM. The Mac cannot reach that host (D-13), so the
executor built and checksummed the bundle on the Mac; the owner runs it and
returns `out/` plus `run-all.log`, and the executor records the verdicts.

Nothing here needs root and nothing calls `sudo`. `run-all.sh` refuses to
start as root and verifies `MANIFEST.sha256` before the first step. It writes
only under `R9_SPIKE_WORK` (default `$HOME/r9-spike-work`) and `./out` inside
this bundle.

## Needs on the Linux host

- nix with flakes and the sandbox enabled;
- a user with write access to `/dev/kvm`;
- network access to GitHub (the s3 and s6 steps fetch the pinned Harvey and
  r9 revisions);
- at least **30 GB free** under `~/r9-spike-work` (the s3 step builds the
  full Harvey x86_64 userland and bakes nine images);
- no `sudo`, no root, and no access to your own r9 or Harvey checkouts.

The steps that need `qemu`, `git`, `expect`, `python3` or `procps` get them
from nix through the committed spike locks, so the host itself needs only
nix.

## Expected duration

The **s3 step includes a full Harvey x86_64 tier build and nine image
bakes**, so expect **tens of minutes or more** for that step alone; the two
gate runs and the determinism rebake repeat parts of it. s1 is seconds to a
few minutes once nix has fetched its inputs, s5 is a short KVM boot, and s6
adds three chain-image bakes plus three q35shell boots. Budget a session of
an hour or more, network depending.

## The command sequence

Replace `<linux-host>` with your machine's ssh name.

On the Mac:

1. `scp "$HOME/r9-spike-bundle/r9-spikes-linux-witness.tar.gz" <linux-host>:`

On the Linux host:

2. `mkdir -p ~/r9-spikes && tar -C ~/r9-spikes -xzf ~/r9-spikes-linux-witness.tar.gz`
3. `less ~/r9-spikes/linux-witness/run-all.sh`   (review it first)
4. `cd ~/r9-spikes && bash linux-witness/run-all.sh 2>&1 | tee run-all.log`
   (optional: `export S5_KERNEL_EXTRA=/path/to/your/own/9k10cpu.elf` first if
   you have one built at `bd7a698`; to rerun only some steps later:
   `R9_SPIKES_ONLY="s3 s6" bash linux-witness/run-all.sh`)
5. `tar -C ~/r9-spikes -czf ~/r9-spikes-out.tar.gz out run-all.log`

On the Mac:

6. `mkdir -p "$HOME/r9-spike-returns/linux-witness" && scp <linux-host>:r9-spikes-out.tar.gz "$HOME/r9-spike-returns/linux-witness/" && tar -C "$HOME/r9-spike-returns/linux-witness" -xzf "$HOME/r9-spike-returns/linux-witness/r9-spikes-out.tar.gz"`

Bring back (all inside the tarball): `out/summary.tsv`, `out/env.txt`,
`out/s1/`, `out/s3/` (rio, control-noG, determinism), `out/s5/`, `out/s6/`,
`out/survivors.txt`, `out/secret-scan.txt`, `run-all.log`.

Optional cleanup on the Linux host after step 6:

```
rm -rf ~/r9-spike-work ~/r9-spikes ~/r9-spikes-linux-witness.tar.gz ~/r9-spikes-out.tar.gz
```

## What the driver does

- **Preflight** writes `out/env.txt`: `uname -a`, `nproc`, memory, free disk
  under `R9_SPIKE_WORK`, `nix --version`, the `sandbox` and
  `experimental-features` settings, whether `/dev/kvm` is writable, and the
  sysctls `user.max_user_namespaces`, `kernel.unprivileged_userns_clone` and
  `kernel.apparmor_restrict_unprivileged_userns`. The last three are what
  decide whether Harvey's unshare-based x86_64 venue can start.
- **s1** runs `nix build` for `s1-nsprobe` (rung 1) and `s1-libc-amd64`
  (rung 2: Harvey's host toolchain plus the amd64 libc in the proot venue)
  from the bundled flake. Logs: `out/s1/`.
- **s5** runs Harvey's own `9k10cpu.elf` under q35 KVM with virtio-net, via
  the bundled `s5/boot-q35.sh`; the serial capture is
  `out/s5/q35-kvm.log`. If `S5_KERNEL_EXTRA` is set, the same leg runs again
  with that kernel and its own capture. The recorded Mac copy's sha256 is
  `ecdcdaddea8d3759d74303b869ddfae8efeba1d4d160382577cfc52b9a32c45b`;
  the bundle's copy is checked against it when the bundle is built.
- **s3** prefetches the pinned Harvey source
  (`bd7a6982a3e4f465f8ed3da5e0c7290521514ff0`), copies it to
  `$R9_SPIKE_WORK/harvey` (writable, no `.git`), runs the rio gate, the
  `control-noG` gate and the two-bake determinism check, all with
  `KERNDATE=1795382101`. Logs: `out/s3/rio.log`,
  `out/s3/control-noG.log`, `out/s3/determinism.log`.
- **s6** clones `requaos/r9` bloblessly and checks out
  `116bcc23c8052a1728992cdbc7b625143260d683`, then runs the q35shell
  control/probe/flip-back swap against the s3-built scratch Harvey tree.
  Log: `out/s6/linux-x86_64.log`; the terminal line is
  `S6-X86 control=PASS probe=PASS flipback=PASS sha-equal=yes` on success.
  If the s3 step left no built `build/x86_64/out`, s6 prints
  `S6-SKIP no-harvey-build` and records exit 77 instead of pretending to
  run.
- **Finish** lists surviving `qemu-system`, `Xvfb` or `drawterm` processes
  in `out/survivors.txt`, deletes any `auth-creds` artifact found under
  `out/` and any file containing the scratch tree's DEMO password value,
  and records both in `out/secret-scan.txt` (the value itself is never
  printed). It then prints the exact `tar` command for returning the
  outputs.

## Exit codes and the summary

`out/summary.tsv` has one row per selected step: `step<TAB>exit<TAB>seconds`.
`run-all.sh` exits 0 only when every selected step exited 0; a skipped s6 is
77. A non-zero step is still a complete answer — the evidence that names the
failure is in `out/<step>/`.

## Re-running a subset

```
R9_SPIKES_ONLY="s3 s6" bash linux-witness/run-all.sh 2>&1 | tee -a run-all.log
```

The s6 step needs the scratch tree and `build/x86_64/out` that the s3 step
leaves in `$R9_SPIKE_WORK/harvey`; rerun s3 first if that tree is gone.

## What not to send back

Do not add `auth-creds.txt` or `authfeed.txt` from the scratch Harvey tree
to the return tarball. The driver never copies them into `out/`, and it
deletes any that appear there; only the `out/` tree and `run-all.log` belong
in the return tarball.
