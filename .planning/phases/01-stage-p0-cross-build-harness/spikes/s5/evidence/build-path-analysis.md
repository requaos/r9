# S5 build-path and prompt analysis — Harvey's own 9k kernel without r9

Read-only analysis at Harvey commit `bd7a6982a` (the Phase 1 pin, legacy-final).
Every citation is `<path>:<line>` at that commit, read with
`git -C /Users/neilskinner/code/harvey show bd7a6982a:<path>`. Nothing was
built, edited or executed inside the Harvey checkout; the only kernel under
test is the copy recorded in `kernel-provenance.txt`.

## 1. Minimal chain that builds amd64/9k10cpu.elf without r9

The flake app surface is `harvey-setup`, `harvey-toolchain`, `harvey-build`
(platform-split) and `harvey-iso` (flake.nix:75-103). Every `boot-*` app in
the flake boots **r9's** kernel through the `r9` input; there is no
own-kernel launch app (flake.nix:114-464, app names `harvey-boot-r9*` and
`harvey-drawterm*`). D-08's own-kernel command is therefore new work.

**On Linux (the owner's x86_64 witness host).**

1. `nix run .#setup` copies plan9port's `9c` (as `pcc`), `sed`, `yacc`,
   `lex`, `awk`, `date`, `dd`, `rm`, `basename`, `ls` and a `size` stub
   into `.build/hostbin-<os>-<arch>/bin` (nix/setup.sh:14-40).
2. `nix run .#toolchain` builds the Plan 9 cross toolchain (`6a/6c/6l`,
   `8a/8c/8l`, `cpp`, `aux`, `pcc`) into the same hostbin (nix/toolchain.sh:1-21).
3. `nix run .#build` runs nix/build.sh. It first edits two mkfiles in the
   checkout with GNU sed (nix/build.sh:62-69) — so the build is *not*
   read-only toward the checkout — then:

   - **sudo, host `/bin` install.** It requires the host's setuid sudo and
     copies `cpp`, `bind`, `rc`, `6a 6c 6l 8a 8c 8l` (and `ls` when the
     host lacks `/bin/ls`) into the host `/bin` (nix/build.sh:92-124). pcc
     executes those by absolute `/bin` path (nix/build.sh:99-115).
   - **sudo, mount namespace.** It runs
     `sudo unshare -m bash BUILD_SETUP_NAMESPACE $USER exports.$$`
     (sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE:5-8). The namespace side
     bind-mounts the checkout over `/sys`, `/amd64`, `/386`, `/rc`, ... ,
     builds a merged `/lib` so the distro loader stays reachable, then
     drops back to the invoking user with `su -c ./BUILD $USER`
     (sys/src/BUILD_SETUP_NAMESPACE:5-89).
   - `sys/src/BUILD` runs `mk clean`, `mk libs`, `mk all`, then
     `rc $HARVEY/build/scripts/linux-build` (sys/src/BUILD:9-15). The amd64
     branch builds libc and `cmd/6?`, installs the whole amd64 tree, then
     loops `k10cpu k10cpuf k10f` with
     `mk nuke && mk CONF=$i install && mk nuke` (build/scripts/linux-build:26-42,
     kernel loop 37-40).
   - The link needs userland first: k10cpu embeds `/boot/boot`, `factotum`,
     `ipconfig`, `nvram` from the amd64 tree (sys/src/9k/k10/k10cpu:71-75);
     k10cpuf and k10f add `fossil` and `venti` (sys/src/9k/k10/k10cpuf,
     `k10f`, rootdir sections). build.sh's preflight therefore requires
     `amd64/bin/{ip/ipconfig,auth/factotum,fossil/fossil,venti/venti}`
     (nix/build.sh:74-90). "Minimal" still means the full cmd chain before
     the kernels; there is no kernel-only shortcut in the supported app.

**On darwin (this Mac).** The flake selects `nix/vm-build.sh`
(flake.nix:94-103). It creates or starts the lima VM (`nix/lima-harvey.yaml`),
rsyncs the checkout to guest-local disk (bind-mounting a virtiofs subdir is
broken, nix/vm-build.sh:1-7, 50-54), runs
`nix run .#setup && nix run .#toolchain && nix run .#build` inside the
guest (58), and rsyncs `amd64/`, `386/`, `arm64/` back (64-67). The Mac host
needs lima, not sudo; root lives inside the VM. This is the only supported
darwin route today.

**Sudo-free on Linux?** Yes, conditionally. `build/mm/build-9k10t1.sh`
documents and carries the venue split: when `sudo -n true` fails, the same
build core runs under `unshare -Ur -m` with a tmpfs chroot scaffold
(build/mm/build-9k10t1.sh:250-356; userns entry 287-356). It is still
Linux-only (57-63) and assumes the host toolchain and the prebuilt
`amd64/bin`/`libc.a` preconditions (65-80), `flock` (88-90) and
`$PLAN9/bin/rc` (80). So a sudo-free build exists only where unprivileged
user namespaces are enabled; darwin still needs lima. Spike S1's result was
not on disk when this analysis ran (no `01-01-SUMMARY.md` in the phase
directory), so this analysis does not lean on it.

## 2. What `root is from` accepts, and what an rc prompt needs

The kernel config's `boot` section is `boot cpu` followed by `tcp`
(sys/src/9k/k10/k10cpu:68-70). `mkbootconf` turns `boot cpu` into
`cpuflag = 1` and every other line into one `Method` entry
(sys/src/9k/mk/parse:138-197; cpuflag 152-156, method entries 167-177,
`cpuflag` emission 180). Because only `tcp` is listed, the generated method
table has exactly one method: `tcp`.

The prompt is assembled from that table: `root is from (<methods>)`
(sys/src/9k/boot/boot.c:340-345), and `outin` appends the default in
brackets and a colon-space (sys/src/9k/boot/aux.c:157-170). That is exactly
the observed `root is from (tcp)[tcp]: ` on all three Mac legs. The prompt
is printed before any method configuration runs (`pickmethod` calls
`rootserver` first, boot.c:98-108), which is why even the no-NIC leg reaches
it.

Accepted replies: `outin` reads one line (aux.c:171-186). `findmethod`
matches the reply's prefix before `!` (boot.c:294-316); the loop re-prompts
until a known method is named (boot.c:360-365); the rest is tokenized and
the text after `!` becomes `$sys` (boot.c:367-373). With only `tcp`
compiled, any reply must start with `tcp`.

The `tcp` path: `configtcp` calls `configip` (sys/src/9k/boot/bootip.c:110-116),
which binds `#I` and any present `#l0..#l3` under `/net` (49-64), forks
`/boot/ipconfig` with the reply arguments (67-77), then resolves the
filesystem and authentication IPs — first `/net/ndb` (163-193), then the
`fs=`/`auth=` environment (142-161), and finally interactive prompts
"filesystem IP address" and "authentication server IP address" (94-97,
195-208). `connecttcp` dials `tcp!<fsip>!564` (118-130). On connect,
`fversion` speaks 9P and `fauth`/`mount` proceed; a server that requires no
authentication simply returns an error to `fauth` and the mount is
attempted anyway (boot.c:135-145, 165-171; libauth/newns.c:125-142).

Candidate root sources with the `tcp`-only method:

- **Host 9P server (smallest, no Harvey source edit).** Harvey carries its
  own Unix `u9fs` under `sys/src/cmd/unix/u9fs`; its makefile builds with
  the host C compiler (`makefile`:17-20, 51-55). `-a none` accepts every
  attach (authnone.c:5-25). It chroots to the directory named as its
  argument (u9fs.c:1759-1761) and serves 9P2000 on stdin/stdout
  (`serve(0,1)`, u9fs.c:1767), so it needs a TCP listener wrapper
  (inetd/socat) on port 564. The chroot needs root or an unprivileged
  user-namespace scaffold; on darwin that means lima.
- **Disk image.** Needs a disk/`local` boot method compiled in; k10cpu has
  only `tcp` (k10cpu:68-70). A Harvey kernel-config edit.
- **paq root.** Needs the `paq` method and a `/boot/paqfs` boot file; the
  config has neither, and the rootdir list embeds no `paqfs`
  (k10cpu:68-75). Also a Harvey source edit.

So "build and boot to the root is from prompt" needs no root source at all;
"boot to an rc prompt" is smallest via u9fs (option B below).

**What an rc prompt needs from the mounted tree.** The boot program execs
`/<cputype>/init -c` when `cpuflag=1` (boot.c:201-213), and the cmd mkfile
installs init at `/$objtype/init` (sys/src/cmd/mkfile:181-182), i.e.
`/amd64/init`. Before that, `init` calls `newns` (sys/src/cmd/init.c:63),
which reads `/lib/namespace` (sys/src/libauth/newns.c:113-117). That file
binds `#c /dev`, `#d /fd`, `#e /env`, `#p /proc`, `#s /srv`
(lib/namespace:7-11) and binds `/$cputype/bin` over `/bin`, then `/rc/bin`
after it (lib/namespace:26-27). `init -c` first execs
`/bin/rc -c /rc/bin/cpurc` (init.c:66-67, 183-187); if that exits, the
loop starts `/bin/rc` (init.c:69-71, 170-181). `rc` reads
`/rc/lib/rcmain` on every start (sys/src/cmd/rc/plan9.c:27).

Minimum tree for a prompt: `amd64/init`, `amd64/bin/rc`,
`rc/lib/rcmain`, `rc/bin/cpurc` (optional; init falls back), `lib/namespace`,
plus the built `amd64/bin` tree the namespace binds. A built Harvey
checkout already holds all of those; `/adm/timezone/local` is optional
(init.c:59 only warns). The root tree also needs the kernel devices, which
`lib/namespace` binds — no userland device files needed.

## 3. q35 specifics: NIC drivers, PCI config access, and the Task 1 legs

k10cpu compiles the ether and ip sections (k10cpu:18-19) and these link
drivers: `ether8139`, `ether8169`, `ether82557`, `ether82563`, `ether82598`,
`etherigbe`, `etherm10g`, `ethervirtio`, each with `pci`
(k10cpu:34-45). `cpuserver = 1` (k10cpu:78).

- **virtio:** `ethervirtio` matches vendor `0x1AF4` with device
  `0x1000`/`0x1041`, revision 0, and a legacy I/O BAR
  (sys/src/9k/386/ethervirtio.c:556-571). The Task 1 q35+virtio leg binds
  it: `#l0: virtio: 1Gbps port 0xc040 irq 11: 525400123456`.
- **q35's default NIC is not unsupported.** QEMU's q35 default is the
  e1000e (i82574, PCI `8086:10d3`). `ether82563` maps `0x10d3` to `i82574`
  (sys/src/9k/386/ether82563.c:1932-1934) and registers the `i82574` probe
  (2262-2267). It did not need to be exercised: the planned three legs use
  virtio, none and the pc default.
- **pc default NIC:** the Task 1 pc leg matches `etherigbe`, whose table
  covers the e1000 family including `i82540em` (`0x100E`)
  (sys/src/9k/386/etherigbe.c:33-49); the log shows
  `#l0: i82543: ... irq 11: 525400123456`.
- **PCI config access** is the legacy mechanism #1 through I/O ports
  `0xCF8`/`0xCFC` (sys/src/9k/386/pci.c:39-43). There is no MCFG/ECAM
  path in this kernel (no MCFG or mmconfig reference in `sys/src/9k`), and
  q35 still exposes the legacy ports, so enumeration works — all three
  legs enumerated their devices.
- **Why pc and q35 logs differ:** both machines' BIOS publish a `$PIR`
  table at `0xF0000`, which the kernel scans and applies only when the
  named interrupt router is a known south bridge (pci.c:719-757). On pc
  the router is the PIIX3 (`8086:7000`, in `southbridges[]`
  pci.c:642-667), so the BIOS workaround line prints
  (`pcirouting: BIOS workaround: PCI.0.1.3 ... 10 -> 9`). On q35 the
  table's router devfn resolves to `00:01.0`, which is QEMU's Bochs VGA
  (`1234:1111`), not in the table, so the kernel prints
  `pcirouting: ignoring south bridge PCI.0.1.0 1234/1111` and skips the
  fixup (pci.c:743-756). The interrupt line programmed by SeaBIOS is still
  used, so the NIC binds on both machines.
- **The q35+virtio warning is a driver quirk, not a failure:**
  `intrenable: nil handler for 11, tbdf 0xc001000 for ether0` comes from the
  generic devether attach passing `ether->interrupt` to `intrenable`
  (sys/src/9k/386/devether.c:426-427; k10/trap.c:40-44), but `ethervirtio`
  never sets that field — it installs its own handler in `reset`
  (ethervirtio.c:666-676). `etherigbe` does set it (etherigbe.c:2034), which
  is why the pc leg has no such line. The virtio NIC still prints and works.
- **q35 + no NIC** reaches the prompt because the prompt precedes method
  configuration; a `tcp` root would then be unreachable (no `#l0`).

## 4. D-20 scope options for a one-command Harvey app

These are options for the closing checkpoint, not decisions; nothing here
edits Harvey. Both reuse `s5/boot-q35.sh` as the launch-and-classify core
(it already supports TCG and KVM, q35 and pc, virtio/none/default).

### Option A — build and boot to the `root is from` boot prompt

- **Inputs:** Harvey source at the pin; nix; a Linux builder (native, or
  lima on darwin); qemu. No root image, no 9P server, no network.
- **Command shape:** `setup → toolchain → build`, then
  `qemu -M q35 -accel kvm|tcg -cpu host|qemu64 -m 512 -kernel
  amd64/9k10cpu.elf -display none -no-reboot -serial file:<log>`.
- **New Harvey-side work:** one flake app (build + launch + classify) and
  its script; docs. No kernel, userland or config source change. Reuses
  the S5 script nearly verbatim.
- **Sudo/venue:** the build's sudo/namespace needs (section 1) remain;
  darwin keeps the lima path.
- **Evidence:** Mac TCG PASS on all three legs; KVM witness pending
  (Plan 06). This is the smallest honest D-20 delivery.

### Option B — build and boot to an rc prompt via a host-served 9P root

- **Inputs:** everything in A, plus a root tree and a 9P server. Smallest
  Harvey-only server: `sys/src/cmd/unix/u9fs` built with the host cc
  (makefile:51-55), `-a none` (authnone.c:5-25), chrooted at the assembled
  root (u9fs.c:1759-1761), behind a TCP listener on 564 (u9fs speaks on
  stdin/stdout, u9fs.c:1767). The root tree is the built checkout's
  `amd64/init`, `amd64/bin`, `rc/`, `lib/namespace` (section 2).
- **Guest side:** qemu slirp (the S5 `S5_NET=virtio` mode, no hostfwd
  needed); at the prompt take the `tcp` default, then answer
  `filesystem IP address` = `10.0.2.2` and any valid auth IP (u9fs
  auth-none rejects Tauth, and the mount proceeds).
- **New Harvey-side work:** an app script that stages the minimal root,
  starts the listener (privileged chroot or an `unshare -Ur` scaffold) and
  drives the two prompt answers (expect-style), plus a decision on whether
  to ship a minimal `cpurc` or rely on init's `/bin/rc` fallback
  (init.c:69-71).
- **Why not disk or paq:** both need a Harvey kernel-config edit to compile
  another boot method (section 2) and, for paq, a new `/boot/paqfs` embed.

Option A answers the D-20 question with evidence today; Option B is the
next increment and needs no Harvey source edit if u9fs is accepted as the
root server.

## 5. Citation summary (all at bd7a6982a)

| Claim | Citation |
|---|---|
| Apps `harvey-setup`/`harvey-toolchain`/`harvey-build`, platform split | flake.nix:75-103 |
| No own-kernel launch app | flake.nix:114-464 (all `boot-*` are `harvey-boot-r9*`) |
| sudo `/bin` installs and namespace entry | nix/build.sh:92-130; sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE:5-8 |
| Namespace binds and su drop | sys/src/BUILD_SETUP_NAMESPACE:5-89 |
| Kernel loop k10cpu/k10cpuf/k10f | build/scripts/linux-build:37-40 |
| darwin lima build | nix/vm-build.sh:1-67; flake.nix:94-103 |
| sudo-free userns venue | build/mm/build-9k10t1.sh:250-356 |
| `boot cpu` + `tcp` method table | sys/src/9k/k10/k10cpu:68-70; sys/src/9k/mk/parse:138-197 |
| Prompt text and default | sys/src/9k/boot/boot.c:340-345; aux.c:157-170 |
| tcp config, fs/auth prompts, port 564 | sys/src/9k/boot/bootip.c:94-130, 163-208 |
| init exec and rc fallback | sys/src/9k/boot/boot.c:201-213; sys/src/cmd/init.c:63-71, 170-187 |
| init install path, rcmain | sys/src/cmd/mkfile:181-182; sys/src/cmd/rc/plan9.c:27 |
| namespace binds | lib/namespace:7-11, 26-27 |
| u9fs build/auth/chroot/serve | sys/src/cmd/unix/u9fs/makefile:17-20, 51-55; authnone.c:5-25; u9fs.c:1759-1767 |
| NIC drivers and IDs | sys/src/9k/k10/k10cpu:34-45; ethervirtio.c:556-571; ether82563.c:1932-1934, 2262-2267; etherigbe.c:33-49 |
| PCI config mechanism and routing | sys/src/9k/386/pci.c:39-43, 642-667, 719-757 |
| virtio intrenable warning | devether.c:426-427; ethervirtio.c:666-676; k10/trap.c:40-44 |
