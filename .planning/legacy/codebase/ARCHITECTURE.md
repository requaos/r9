---
last_mapped_commit: bdcf45a51dced761eec09538b5f0786ff81e0399
last_mapped_at: 2026-11-13
---
<!-- refreshed: 2026-11-13 -->
# Architecture

**Analysis Date:** 2026-11-13

## System Overview

Harvey OS is a **two-repo composed system**. This repo (Harvey) owns the Plan 9
userspace, libc, native toolchain (6a/6c/6l), and all build/bake/gate machinery.
The sibling repo `../r9` (`/home/req/code/r9`) is the adopted kernel — a Rust
no_std reimplementation of the Plan 9 kernel. Harvey's legacy C kernel
(`sys/src/9k/`, amd64) remains in-tree and green (the "Harvey-on-9k" track), but
the active v2.1 milestone is Harvey-on-r9 on x86_64 under qemu q35/KVM. The two
repos are joined **only** through nix flake composition: `flake.nix` pins
`inputs.r9` (git rev + narHash in `flake.lock`) and consumes r9's exported
packages (`kernel-x64`, `iso-builder`, `netdev-args`, and aarch64's
`kernel-virt`). No submodules, no relative-path glue.

```text
┌──────────────────────────────────────────────────────────────────────────┐
│                     Nix flake composition (flake.nix)                     │
│   inputs.r9 pinned rev → kernel-x64 / iso-builder / netdev-args           │
│   apps: boot-r9-x64, boot-r9-smoke-x64, boot-r9-tier1-x64,                │
│         boot-r9-net-smoke-x64, boot-r9-services-x64, boot-r9-net-soak-x64,│
│         boot-r9-churn-x64, boot-r9-stress-x64, boot-r9-auth-x64,         │
│         net-doctor, build, iso,                                           │
│         setup, toolchain (+ frozen aarch64: boot-r9, *-smoke, tier1)      │
├───────────────────────────────┬──────────────────────────────────────────┤
│  Harvey repo (this tree)      │  r9 repo (../r9, Rust no_std kernel)     │
│                               │                                          │
│  userspace build              │  cargo workspace                         │
│   sys/src/cmd/ (274 dirs)     │   port/    arch-neutral kernel crate     │
│   sys/src/lib*/ (40+ libs)    │             (namec, pgrp, devcons/env/   │
│   sys/src/libc/ + 9syscall/   │              dup/pipe/proc/srv/root/mnt/ │
│   toolchain sys/src/cmd/6*/   │              ether, devip/, virtio,      │
│                               │              paqfs, rendez, mcslock)     │
│  bake + gates                 │   x86_64/  l.S + platform/q35/ (mb2,     │
│   build/x86_64/mkimage.sh     │             acpi, lapic, ioapic, pci,    │
│   build/x86_64/tiers/*.txt    │             timer, netgate) + proc,      │
│   build/x86_64/*.expect       │             syscall, exec, usermem,      │
│   build/harness/ (Rust)       │             swtch, trap                  │
│   build/mm/ (minimega, 9k)    │   aarch64/ frozen v2.0 pin; riscv64/     │
│                               │   xtask/   build/qemu/boot-test/         │
│  C kernels (kept green)       │             ureg-check harness           │
│   sys/src/9k/  amd64 (active) │   x86_64/lib/config_q35*.toml profiles   │
│   sys/src/9/   legacy multi-  │   x86_64/lib/grub.cfg, kernel.ld         │
│                arch           │                                          │
├───────────────────────────────┴──────────────────────────────────────────┤
│        qemu-system-x86_64 -M q35 -accel kvm -cdrom <composed ISO>        │
│   SeaBIOS → GRUB2 (multiboot2 kernel + module2 bootfs.paq) → r9 →        │
│   Harvey boot(8) → paqfs root → init → rc → `% ` prompt                  │
└──────────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| Flake composition | Pin r9, export one-command boot/gate apps, devshell | `flake.nix`, `nix/*.sh` |
| Portable 9k kernel (C) | Processes, chans, devices, VM, syscall dispatch | `sys/src/9k/port/` |
| k10 (amd64) machine layer (C) | MMU, traps, FPU, APIC/IOAPIC, SMP, asm entry | `sys/src/9k/k10/` |
| Kernel config → devtab generation | Declarative configs → `$CONF.c` (devtab, bootconf) + embedded root | `sys/src/9k/mk/parse`, `sys/src/9k/k10/k10cpu`, `sys/src/9k/k10/k10t1` |
| Kernel IP stack (C) | In-kernel TCP/IP (`/net` via devip) | `sys/src/9k/ip/` |
| boot(8) user bootstrap | First user process: find root, auth, exec init | `sys/src/9k/boot/boot.c`, `bootr9.c` |
| libc | C library: portable, syscall wrappers, per-arch asm | `sys/src/libc/` |
| Syscall ABI table | Syscall numbers shared kernel↔libc (BIND=2 … NMOUNT) | `sys/src/libc/9syscall/sys.h` |
| Userspace commands | ~274 command/daemon/file-server directories | `sys/src/cmd/` |
| APE | ANSI/POSIX env for ported Unix software | `sys/src/ape/` |
| Legacy kernel | Original multi-arch Plan 9 kernel tree (dormant) | `sys/src/9/` |
| r9 portable kernel (Rust) | Arch-neutral proc/exec/namespace/devices/virtio/devip/paqfs | `../r9/port/src/` |
| r9 x86_64 platform (Rust) | l.S entry, mb2, ACPI/ECAM, LAPIC/IOAPIC/timer, proc/syscall/exec | `../r9/x86_64/src/` |
| r9 boot profiles | TOML data → cfg emission, qemu args, dist shape | `../r9/x86_64/lib/config_q35*.toml`, `../r9/xtask/src/config.rs` |
| r9 acceptance harness | cargo xtask build/qemu/boot-test/ureg-check; netdev argv emitter | `../r9/xtask/src/main.rs`, `../r9/xtask/src/bin/r9-netdev-args.rs` |
| amd64 exec/ABI contract | Frozen 184-byte Ureg + syscall wire contract | `amd64/ABI.md` (aarch64 twin: `aarch64/ABI.md`) |
| Userland staging build | Namespace build → marker + boot-chain binaries | `build/x86_64/stage-build.sh` |
| Image bake | bootfs*.paq images from root-tree skeleton + built binaries | `build/x86_64/mkimage.sh`, `build/x86_64/root-tree/` |
| Tier allowlists | Append-only committed data gating userspace sweeps | `build/x86_64/tiers/tier{0..4}*.txt` |
| Expect gates | qemu serial-driven smoke/tier/net/services/churn/soak gates | `build/x86_64/*.expect`, `build/aarch64/*.expect` |
| Host harness | std-only Rust: srv9/drive9/gdbpoke9/mkfs9/trace-syscalls | `build/harness/` |
| minimega 9k venue | .mm scripts booting 9k10cpu/k10t1/k10draw under KVM | `build/mm/` |
| 9k namespace build | mk build inside private mount namespace | `sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE`, `sys/src/BUILD`, `sys/src/BUILD_SETUP_NAMESPACE` |
| CI | nix-pipeline workflows | `.github/workflows/harvey.yml`, `phase6.yml`, `r9-shell.yml` |

## Pattern Overview

**Overall:** Plan 9 "everything is a file" OS, dual-kernel (C 9k + Rust r9), with a
reproducible nix+bake+expect delivery pipeline wrapped around it.

**Key Characteristics:**
- **File-oriented IPC:** all resources are files over 9P2000 (`sys/include/fcall.h`); per-process namespaces built via bind/mount
- **Device = vtable:** kernel devices implement `struct Dev` (`sys/src/9k/port/portdat.h:198`); `devtab[]` generated from declarative config by awk (`sys/src/9k/mk/parse` lines 296–300); r9 mirrors this with `dev*.rs` modules cfg-gated by TOML (`../r9/port/src/`, `../r9/x86_64/lib/config_q35*.toml`)
- **Portable/machine split, twice:** 9k `port/` + `k10/`; r9 `port/` crate + `x86_64/`/`aarch64/`/`riscv64/` crates
- **Frozen ABI as documents + build gates:** `amd64/ABI.md` is the whole kernel/user boundary; enforced two-layer (const asserts + `cargo xtask ureg-check` with negative control)
- **Committed data, never flags:** tier allowlists (`build/x86_64/tiers/*.txt`), boot profiles (r9 `config_*.toml`), bootfs payload manifest (`build/x86_64/bootfs-payload.txt`) are append-only data files driving machinery
- **Evidence gates:** every claim is an expect/boot-test gate that exits 0/1 with retained serial captures; venue honesty (SKIP lines labeled, never unearned OK)
- **Pin discipline:** r9 arrives only via flake input pin; kernel-byte commits re-stale the boot-iso FOD hash — re-wire-then-pin at phase close

## Layers

**Harvey userspace (C):**
- Location: `sys/src/cmd/`, `sys/src/lib*/`, `sys/src/ape/`
- Built by mk inside a Linux mount namespace emulating the Plan 9 root
  layout (`sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE` → `BUILD_SETUP_NAMESPACE` →
  `sys/src/BUILD`), driven host-side by `nix/build.sh` (native) or
  `nix/vm-build.sh` (lima VM on darwin)
- Keyed by `objtype` (currently `amd64` for v2.1; `aarch64` frozen at v2.0)

**9k kernel (C, amd64 — kept green):**
- Portable: `sys/src/9k/port/` (proc.c, chan.c, dev*.c, sysfile.c, sysproc.c, segment.c, qio.c, qlock.c, devmnt.c 9P client, devroot.c embedded root)
- Machine: `sys/src/9k/k10/` (main.c:145 entry, mmu.c, trap.c, l64*.s, apic/ioapic, mp/sipi)
- Config-generated glue: `sys/src/9k/mk/` (`parse` awk, `portmkfile`, `bootmkfile`, `mkroot`, `mkenum`)
- Configs: `k10cpu`, `k10cpuf`, `k10f`, `k10aoe`, `k10root`, `k10t1` (tier-1 CI venue with embedded sweep), `k10draw` (draw donor venue)

**r9 kernel (Rust, x86_64 active):**
- `../r9/port/src/`: arch-neutral — `namec.rs` (name resolution), `pgrp.rs`, `fdgrp.rs`, `devcons.rs`/`devenv.rs`/`devdup.rs`/`devpipe.rs`/`devproc.rs`/`devsrv.rs`/`devroot.rs`/`devmnt.rs` (9P client), `devether.rs` + `netif.rs`, `devip/` (arp, conv, ipifc, medium, proto, route, stack, waker — smoltcp 0.14 behind the devip ABI), `virtio.rs` (from-scratch unsafe-free transport), `paqfs.rs` (bootfs reader), `rendez.rs`, `mcslock.rs`, allocator family. `#[forbid(unsafe_code)]` on every module except named floors (`globalalloc`, `mcslock`, `devip_waker` bridge)
- `../r9/x86_64/src/`: `l.S` (mb2 header + 32-bit entry + long mode), `main.rs`, `platform/q35/` (`mb2.rs`, `acpi.rs`, `lapic.rs`, `ioapic.rs`, `pci.rs` ECAM, `timer.rs` CPUID-gated x2APIC TSC-deadline/xAPIC periodic, `netgate.rs`, `churngate.rs`), `proc.rs` (per-process PML4/TSS-rsp0, round-robin, rfork/await, notes), `syscall.rs` (SYSCALL/SYSRET, software-built Ureg), `exec.rs` (a.out loader, S_MAGIC 0x8A97), `usermem.rs` (validated user access on live-CR3 model; 4-GiB device alias window at PML4[510]), `swtch.rs` (one resume path via `first_entry` trampoline), `trap.rs`
- `../r9/aarch64/` (frozen at v2.0 pin): platform/{raspi3b,raspi4b,virt}, GIC, DTB boot
- `../r9/xtask/src/`: subcommands `build | expand | kasm | dist | test | clippy | check | qemu | boot-test | ureg-check | initrd | clean`; `config.rs` compiles TOML profiles into `--cfg` flags (`dev_*`, `ip_*`, `link_*`, `nodev_*`, `nouart_*`, `platform`); `bin/r9-netdev-args.rs` emits qemu netdev argv (slirp/tap/vmnet) from the same TOML

**Delivery machinery (host):**
- Build: `build/x86_64/stage-build.sh` (namespace driver; outputs gitignored `build/x86_64/out/` chain binaries + `smoke-out/` marker tree)
- Bake: `build/x86_64/mkimage.sh` — bakes `bootfs.paq` (marker), `bootfs-chain.paq` (boot chain), `bootfs-tier.paq`, `bootfs-net.paq`, `bootfs-services.paq` from `build/x86_64/root-tree/` skeleton + built binaries; paqfs byte format (magic 529ab12b) is arch-shared with aarch64; deterministic (KERNDATE-pinned mtimes)
- Compose: r9's `iso-builder` (`../r9/flake.nix` export; templated `grub.cfg` with `multiboot2 /boot/kernel` + `module2 /boot/bootfs.paq`) → GRUB ISO
- Boot/launch: `nix/boot-r9-x64.sh` (qemu q35, KVM default, net mode from `--net=slirp|tap|vmnet` via the emitter, no silent downgrades)
- Diagnose: `nix/net-doctor.sh` (read-only host-state checker with exact remediation prints)

## Data Flow

### Primary Boot Path (x86_64, Harvey-on-r9 — the current core value)

1. `nix run .#boot-r9-x64` → `nix/boot-r9-x64.sh` (kernel arrives ONLY as `R9_KERNEL_X64_PKG` store path from `inputs.r9.packages.kernel-x64` — the q35shell-profile kernel ELF)
2. Userland build: `build/x86_64/stage-build.sh` — native Linux namespace build (`objtype=amd64`, vendored 6c/6l); harvests boot(8)/init/rc/paqfs + marker set to `out/` + `smoke-out/`
3. Bake: `build/x86_64/mkimage.sh` → `out/bootfs.paq` + `out/bootfs-chain.paq`
4. Compose: `R9_ISO_BUILDER` (r9 flake export) builds the GRUB ISO (fixed-output derivation; byte-reproducible)
5. Launch: `qemu-system-x86_64 -M q35 -accel kvm -cpu host -cdrom <ISO> -nographic` (+ emitter-produced netdev argv)
6. In-guest: SeaBIOS → GRUB2 (`../r9/x86_64/lib/grub.cfg`) → mb2 → `../r9/x86_64/src/l.S` → `main.rs` (mb2 stash/tag-3 module `bootfs.paq`) → `platform/q35` init → `port/` devroot+paqfs serve `/` → exec `/boot/boot` (Harvey `sys/src/9k/boot/boot.c` + `bootr9.c` bootconf, `embed` method → `/rootfs.img`) → paqfs 9P server mounts root → exec `/amd64/init` → `rc` (`/rc/lib/rcmain`, `/rc/bin/termrc`) → interactive `% ` prompt

### 9k Standalone Boot (Harvey-on-9k track)

1. EFI/multiboot → `sys/src/9k/k10/l64v.s`/`l32p.s` → `sys/src/9k/k10/main.c:145` `main(u32int ax, u32int bx)`
2. Machine init → portable init in `sys/src/9k/port/` → pid 1 = `boot` (`sys/src/9k/boot/boot.c`) from embedded root (`devroot.c`)
3. Venue boots driven by `build/mm/boot-9k10cpu.mm` (minimega/KVM) and `build/mm/boot-9k10t1-sweep.sh` (tier-1 sweep from embedded root — an rc script IS `/boot/boot` per `initcode.c`'s contract)

### Userspace Namespace Build Flow

1. `nix develop` (devshell: plan9port, `$PLAN9/bin` LAST in PATH, `objtype=amd64`, `HARVEY_LINUX_BIN=.build/hostbin-<os>-<arch>/bin`)
2. `nix run .#build` → `nix/build.sh` → `sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE` (`sudo unshare -m`) → `sys/src/BUILD_SETUP_NAMESPACE` → `sys/src/BUILD` (`mk clean; mk libs; mk all`) → `build/scripts/linux-build`
3. Artifacts land in the repo-root `amd64/` install tree (`/amd64/bin/...` inside the namespace); host tools in `.build/`

### Gate Flow (acceptance evidence)

1. Flake app (e.g. `boot-r9-smoke-x64` → `nix/smoke-r9-x64.sh`) runs build → bake → compose, then spawns qemu under expect
2. `build/x86_64/smoke-r9.expect` drives serial stages (banner → `% `, echo round-trip, ls/ps/cat, DEL safety, init restart) with whole-capture post-passes; exits 0/1 naming the failed stage; capture retained in `build/x86_64/*.log`
3. r9-side equivalents: `cargo xtask boot-test` with `BootProfile`/`BootStage` marker assertions (`../r9/xtask/src/main.rs`); `cargo xtask ureg-check` cross-checks Ureg layouts against `amd64/include/ureg.h`

### Tier Sweep Flow (both-kernels witness)

1. Tier data: `build/x86_64/tiers/tier0-libs.txt` … `tier4-tail.txt` (append-only)
2. Subset build (mk filter arms) → artifact markers; k10t1 embeds the set (`sys/src/9k/k10/k10t1` rootdir) and boots the sweep donor-side
3. Same `/amd64/bin` set bakes into `bootfs-tier.paq` and executes on r9 (`nix run .#boot-r9-tier1-x64` → `build/x86_64/tier1-r9.expect`) — identical marker sets on BOTH kernels is the ABI witness

**State Management:**
- Kernel singletons are idiomatic: 9k `Sys* sys` (`sys/src/9k/k10/main.c`), `conf`, `devtab[]` (generated), per-CPU `Mach *m`; r9 uses per-proc `Proc` + spinlock-free MCS/BusyCell primitives (`../r9/port/src/mcslock.rs`)
- User state: env vars are files in `/env`; process control via `/proc`; per-pgrp mount tables (`pgrp.c` / `../r9/port/src/pgrp.rs`)

## Key Abstractions

**Chan (9k) / namec (r9):**
- The open-file handle produced by name resolution; every file op flows through it
- Examples: `sys/src/9k/port/chan.c`, `../r9/port/src/namec.rs` (+ `namec_tests.rs`)

**Dev vtable + generated devtab (9k):**
- `struct Dev` (`sys/src/9k/port/portdat.h:198`); one `dev<name>.c` per device; `devtab[]` emitted by `sys/src/9k/mk/parse` from the config's `dev` section; r9 analog: `#[cfg(dev_<name>)] pub mod dev<name>;` gated by TOML (`../r9/port/src/lib.rs`, `../r9/xtask/src/config.rs`)

**9P2000:**
- Wire format `sys/include/fcall.h`; codecs `sys/src/libc/9sys/convM2S.c`/`convS2M.c`; kernel client `sys/src/9k/port/devmnt.c` (+ `../r9/port/src/devmnt.rs`); user-space server framework `sys/src/lib9p/` (`Srv`, `File`, `Req`, `Fid`; canonical ramfs in `sys/src/lib9p/ramfs.c`)

**rfork process model:**
- `rfork(flags)` with fine-grained sharing (RFPROC, RFMEM, RFNAMEG…); r9 implements donor dupseg semantics (`../r9/x86_64/src/proc.rs`, RFMEM growth families + refcount-gated frees — Phase 8/17)
- Notes (not signals): textual notes via `atnotify`/`noted`; r9 delivers through devproc

**paqfs boot image:**
- Read-only compressed root filesystem (magic 529ab12b, trailer 6b46e688): host mkpaqfs (`build/x86_64/mkpaqfs-host.sh`) bakes; r9 `../r9/port/src/paqfs.rs` reads (miniz inflate); delivered as mb2 tag-3 module on x86_64, DTB/loader device on aarch64

**Exec/ABI contract:**
- `amd64/ABI.md` / `aarch64/ABI.md`: Ureg layout byte-verbatim (184-byte amd64 frame), syscall number in BP, args at SP+8, result in AX, note convention; r9's TrapFrame IS Ureg — no translation; build-enforced via const asserts + `cargo xtask ureg-check` (negative control)

**Tier allowlists + marker ledgers:**
- Committed append-only data (`build/x86_64/tiers/*.txt`) is the only growth path; artifact+execution marker ledgers (`build/aarch64/markers/`) are the only gates — mk exit status is never trusted

**Expect serial gates:**
- Stage-asserted qemu serial drivers (`build/x86_64/smoke-r9.expect` et al.): fresh capture per run, process-group teardown, venue-labeled evidence, exit 0/1

**Flake composition with hasAttr gating:**
- `flake.nix` exports x64 apps only where r9 ships the consumed attrs (`kernel-x64`, `iso-builder`, `netdev-args`); aarch64 apps frozen at the v2.0 pin; never widen the gate list ahead of a pin advance

## Entry Points

**r9 kernel entry (x86_64):**
- Location: `../r9/x86_64/src/l.S` (mb2 header, `ENTRY(start)` per `../r9/x86_64/lib/kernel.ld`, KZERO=0xffffffff80000000) → `../r9/x86_64/src/main.rs`
- Triggers: GRUB2 multiboot2 off the composed ISO (never `-kernel`)
- Responsibilities: mb2 stash, long mode, platform/q35 init, first user exec (boot(8))

**9k kernel entry (amd64):**
- Location: `sys/src/9k/k10/main.c:145` `main(u32int ax, u32int bx)` (multiboot)
- Triggers: EFI loader (`sys/src/boot/efi/`) / nboot (`sys/src/nboot/`) / minimega qemu venue (`build/mm/`)

**boot(8):**
- Location: `sys/src/9k/boot/boot.c`; r9 bootconf `sys/src/9k/boot/bootr9.c` (hand-written; the amd64 C kernel awk-generates its own); built by `sys/src/9k/boot/mkfile.amd64`
- Responsibilities: root selection (embed/local/tcp), auth handshake, exec init

**rc startup:**
- `/rc/lib/rcmain` + `/rc/bin/termrc` (or cpurc for `service=cpu` — Phase 19), staged at bake time from `rc/lib/rcmain` (single source, cmp-guarded) and `build/x86_64/root-tree/`

**User commands:**
- `void main(int argc, char *argv[])` + `ARGBEGIN{...}ARGEND` + `exits("msg")` — see `sys/src/cmd/cat.c`, `cp.c`

**Host build/bootstrap:**
- `BUILD_ON_UNIX` (repo root) → `nix run .#setup` → `.#toolchain` → `.#build`/`.#iso`; kernel mk entry `sys/src/9k/k10/mkfile` (`mk 9k` builds `9k10cpu`); CI: `.github/workflows/harvey.yml`, `phase6.yml` (four-leg tier CI), `r9-shell.yml` (x64 shell + net-smoke jobs)

## Architectural Constraints

- **Two kernels must keep sharing one userspace:** identical `/amd64/bin` sets run on k10t1 and r9 — any ABI-affecting change must re-freeze `amd64/ABI.md` and re-pin both repos; never change `ureg.h` or `sys.h` casually
- **Threading:** 9k = per-CPU `Mach`, tas/qlocks, `Rendez` sleep/wakeup, kernel coroutines; r9 = no_std Rust, MCS locks/BusyCell (`../r9/port/src/mcslock.rs`), per-proc CR3 with live TTBR0-equivalent user memory model (no Linux-style copy layer may be introduced)
- **Global state:** kernel singletons idiomatic (see above); r9 `static mut` count is 0 and gated
- **Toolchain:** Plan 9 compilers (6c/6a/6l for amd64) only for native code — never gcc/clang; `mk` not make; `$PLAN9/bin` LAST in PATH (plan9port `mount` shadowing)
- **aarch64 frozen:** v2.0-pin apps/scripts in `nix/` + `build/aarch64/` stay byte-untouched; all v2.1 work keys to x86_64
- **Pin/FOD discipline:** r9 kernel bytes arrive only via `flake.lock` pin; a kernel-byte commit re-stales the `boot-iso` fixed-output derivation — re-wire-then-pin with two-derivation byte-identity proof; pin advances are their own commits, never bundled
- **Circular imports:** 9k `port/` ↔ machine dirs interdepend by design via the `dat.h`/`fns.h`/`mem.h` triad; do not add new reverse deps
- **Check-cfg predeclare rule (r9):** every new TOML-emitted cfg (`dev_*`, `ip_*`, …) must be declared in the root `Cargo.toml` `[workspace.lints.rust] unexpected_cfgs` check-cfg list BEFORE first code use

## Anti-Patterns

### Deriving tool names from `$cputype`

**What happens:** `sys/src/cmd/jpg/mkfile:56-68` derives a nonexistent `oc` compiler from `cputype=unix`, aborting the cmd DIRS loop before `ip/`+`venti/`.
**Why it's wrong:** root-embedding kernels then can't find `/amd64/bin/ip/ipconfig`.
**Do this instead:** host-built generators normalize on the awk idiom — `objtype=$cputype mk <tool>.$cputype` (Phase 1 decision).

### Trusting mk exit status as gate evidence

**What happens:** plan9port mk aborts at the first failing recipe and `sys/src/BUILD` swallows statuses.
**Why it's wrong:** a red build can look green.
**Do this instead:** truth lives in committed artifact+execution marker ledgers; allowlist growth is a reviewable diff (`build/x86_64/tiers/*.txt`, `build/aarch64/markers/`).

### Sibling-checkout or submodule kernel glue

**What happens:** relative paths/submodules for the r9 kernel were considered.
**Why it's wrong:** fragile, non-reproducible composition; the pin must name a fetchable commit.
**Do this instead:** `inputs.r9` git pin in `flake.nix`; local checkouts only via `--override-input` (dev-only).

### Silent venue/mode downgrades

**What happens:** auto-falling back TCG→KVM or tap→slirp would mask venue-specific defects.
**Why it's wrong:** slirp forgives L2 bugs; evidence must name its venue.
**Do this instead:** explicit `R9_ACCEL` / `--net=<mode>` resolution failing loudly (`nix/boot-r9-x64.sh`); tap leg refuses to launch unless `nix/net-doctor.sh` passes; SKIP lines in ledgers, never unearned OK.

### Adding a second hardcoded list

**What happens:** duplicating a payload/marker/expectation list in a second place drifts.
**Why it's wrong:** gates assert stale sets.
**Do this instead:** one committed data file everywhere — `build/x86_64/bootfs-payload.txt`, `tiers/*.txt`, computed SVC expectations from the tier data.

## Error Handling

**9k kernel (C):**
- `error(Efoo)` longjmp-style unwind (`sys/src/9k/port/error.h`); `waserror()`/`poperror()`/`nexterror()` guard cleanup; `panic()` for unrecoverable state
- Syscalls return `-1` + errstr; user prints via `%r` (`sys/src/cmd/cp.c` pattern: `fprint(2, "cp: ...%r\n")`, set `failed`, end `exits("errors")`); fatal via `sysfatal`
- Exit is `exits(0)` / `exits("msg")` — never `exit()`

**r9 (Rust):**
- `Result`/`DynError` through xtask; kernel-side loud-unimplemented syscall default (measured-set dispatch, no silent stubs); `virtpanic`-class debug configs; marker-based gates make failures name the stage
- Host tests: `../r9/port/tests/` + inline `*_tests.rs` (152-pin port suite, virtio state-machine pins)

**Build/bake:** every step loud — `err()` helpers exit 1 with context (`build/x86_64/mkimage.sh`); expect gates never hang-to-timeout silently

## Cross-Cutting Concerns

**Logging:** serial console is the wire — `print`/`iprint` (kernel), expect captures retained as evidence (`build/x86_64/*.log`); no syslog concept
**Validation:** ABI enforced at build (const asserts + ureg-check); bake manifests assert every staged path present/non-empty (`build/x86_64/mkimage.sh`); seam asserts (netaddr venue tokens) run at bake time
**Authentication:** Phase 18 (planned) in-guest p9sk1 domain — bake-time hash-paired generator; today the chain images carry NO `/boot/factotum` (auth stub = deliberate absence in `mkimage.sh` contracts)
**Reproducibility:** KERNDATE from git commit (env-overridable), pinned mtimes, fixed-output ISO derivation, bit-identical bakes (`build/REPRODUCIBILITY.md`)
**Observability:** r9 heap_stats/churngate markers (Phase 17 leak gates); `cargo xtask boot-test` per-run fresh serial logs; qemu `-d int,guest_errors` capture instrument (`nix/stress-r9-net-x64.sh`)

---

*Architecture analysis: 2026-11-13*
