---
last_mapped_commit: bdcf45a51dced761eec09538b5f0786ff81e0399
last_mapped_at: 2026-11-13
---
# Codebase Structure

**Analysis Date:** 2026-11-13

## Directory Layout

```
harvey/                          # repo root — doubles as the Plan 9 namespace root in builds
├── flake.nix / flake.lock       # nix composition: devshell, packages, boot/gate apps; inputs.r9 pin
├── AGENTS.md, BUILD_ON_UNIX     # project instructions; unix bootstrap doc
├── amd64/  aarch64/ 386/ arm/ … # per-objtype install trees (/amd64/bin etc.) + per-arch ABI.md
├── sys/
│   ├── include/                 # 51 public headers (libc.h, 9p.h, bio.h, fcall.h, authsrv.h…)
│   ├── man/                     # manual sections 1-9, 9k (troff); style(6) = the C style guide
│   └── src/
│       ├── 9k/                  # ACTIVE C kernel (amd64): port/ k10/ mk/ ip/ boot/ root/
│       ├── 9/                   # LEGACY C kernel tree (pc, bcm, omap, teg2, …) — dormant
│       ├── cmd/                 # ~274 userspace command/daemon dirs (cat.c, rio/, upas/, auth/…)
│       ├── libc/                # Plan 9 libc: port/ 9sys/ 9syscall/ fmt/ + per-arch dirs
│       ├── lib*/                # 40+ libraries (lib9p, libbio, libdraw, libsec, libthread…)
│       ├── ape/                 # ANSI/POSIX environment (ported Unix software)
│       ├── boot/                # EFI bootloader; nboot/ newer boot tree
│       ├── games/
│       ├── BUILD, BUILDTOOLCHAIN, BUILD_SETUP_NAMESPACE, RUN_BUILD_IN_PRIVATE_NAMESPACE
│       └── mkfile, mkfile.proto # top mk templates (</$objtype/mkfile, mksyslib, mkone, mkmany)
├── build/
│   ├── x86_64/                  # v2.1 venue: mkimage.sh, stage-build.sh, tiers/, root-tree/, *.expect
│   ├── aarch64/                 # frozen v2.0 venue: same shape (mkimage.sh, markers/, *.expect)
│   ├── mm/                      # minimega .mm/sh scripts for 9k10cpu/k10t1/k10draw boots
│   ├── harness/                 # std-only Rust host harness (srv9/drive9/mkfs9/trace-syscalls…)
│   ├── scripts/                 # legacy in-OS build stages (linux-build, iso, repl…)
│   └── Dockerfile, build.bash   # legacy docker path (superseded by nix)
├── nix/                         # flake app scripts (boot-r9-x64.sh, net-doctor.sh, build.sh…)
├── cfg/                         # guest config templates (termrc, cpurc, namespace, pxe/)
├── lib/                         # file-tree content: font/bit, ndb, namespace templates
├── rc/                          # rc shell libraries + rc/bin scripts (rcmain master: rc/lib/)
├── adm/  usr/  cron/  lp/  mail/  dist/  unix/  bbin/  acme/  # root-fs content trees
├── .build/                      # GITIGNORED: host bins (hostbin-<os>-<arch>), logs, staged artifacts
├── .planning/                   # GSD planning artifacts (PROJECT, ROADMAP, phases/, milestones/)
└── .github/workflows/           # harvey.yml, phase6.yml, r9-shell.yml

../r9/  (sibling repo — Rust kernel, flake input)  [NOT part of this git tree]
├── Cargo.toml            # workspace: aarch64, x86_64, port, riscv64, xtask (+ check-cfg lints)
├── port/src/             # arch-neutral kernel: namec.rs, pgrp.rs, dev*.rs, devip/, virtio.rs, paqfs.rs
├── x86_64/src/           # l.S, main.rs, platform/q35/, proc.rs, syscall.rs, exec.rs, usermem.rs, entropy.rs (18-01 RandomHooks: CPUID-gated RDRAND/RDSEED + TSC jitter)
├── x86_64/lib/           # config_q35*.toml boot profiles, grub.cfg, kernel.ld
├── aarch64/ riscv64/     # aarch64 frozen at v2.0 pin; riscv64 upstream vestige
├── xtask/src/            # build/qemu/boot-test/ureg-check harness; bin/r9-netdev-args.rs
└── lib/                  # bare-metal target spec JSONs
```

## Directory Purposes

**`sys/src/9k/` (active C kernel):**
- `port/` — 62 portable `.c` files: `proc.c`, `chan.c`, `dev*.c` (one file per device), `sysfile.c`/`sysproc.c`, `segment.c`, `qio.c`, `devmnt.c` (9P client), `devroot.c` (embedded root), `portdat.h:198` (Dev vtable), `portfns.h`
- `k10/` — amd64 machine layer: `main.c` (entry at :145), `mmu.c`, `trap.c`, `l64*.s` asm, `apic.c`/`ioapic.c`, `mp.c`/`sipi.c`, `vsvm.c`; kernel configs `k10cpu`, `k10cpuf`, `k10f`, `k10aoe`, `k10root`, `k10t1` (tier-1 CI venue), `k10draw`; staged embed trees `.tier1-stage/`, `.draw-stage/`; `root/` protos (`k10root.proto`, `k10t1.rc`, `tcp17010`)
- `mk/` — config toolchain: `parse` (awk: config → `$CONF.c` devtab at lines 296–300), `portmkfile`, `bootmkfile`, `mkroot`, `mkrootall`, `mkenum`, `mkrr`
- `ip/` — in-kernel TCP/IP stack (`devip.c`, `tcp.c`, `udp.c`, `ipifc.c`, `iproute.c`)
- `boot/` — boot(8) user bootstrap: `boot.c`, `bootr9.c` (r9 bootconf), `local.c`, `bootip.c`, `bootauth.c`, `mkfile.amd64`/`mkfile.aarch64`
- `root/` — embedded-root content (`rcmain`, `cpu.c`, `nvram`, `common` ndb master)

**`sys/src/cmd/` (userspace, ~274 entries):**
- Single-file commands (`cat.c`, `cp.c`, `ls.c`); multi-file dirs (`rio/`, `acme/`, `sam/`, `upas/`, `auth/`, `ip/`, `fossil/`, `venti/`, `aux/`, `ndb/`, `exportfs/`)
- Toolchain dirs: `6a/ 6c/ 6l` (amd64 asm/compiler/linker), `8a/ 8c/ 8l` (x86), `7a/ 7c/ 7l` (aarch64, vendored 9front donor), `4a-ql` (other arches), `mk/` (build tool), `rc/` (shell)
- `unix/9cc/` — vendored 9-cc `iar` (host archiver for Plan 9 archives)
- `go/` — image-time Go program fetch scripts (amd64 only)

**`sys/src/libc/`:**
- `port/` (112 portable files), `9sys/` (syscall wrappers: `open.c`, `mount.c`, `rfork.c`, `convM2S.c`), `9syscall/sys.h` (THE syscall number table — shared kernel↔user), `fmt/`, per-arch dirs (`amd64/`, `386/`, `aarch64/`, `arm/`, `riscv/`, …) with `main9.s`, `memcpy.s`, `tas.s`

**`build/x86_64/` (v2.1 delivery venue):**
- `mkimage.sh` — the bake: 12+ sections producing `out/bootfs{,-chain,-tier,-net,-services,-substrate}.paq` (§13 substrate: randread as /amd64/init, the 18-01 q35auth gate image) from `root-tree/` + built binaries; every contract documented in the file header; manifest asserts fail the bake
- `stage-build.sh` — namespace userland build driver (objtype=amd64) → `out/` (chain) + `smoke-out/` (marker)
- `tiers/` — `tier0-libs.txt`, `tier1-cmds.txt`, `tier2-services.txt`, `tier3-gui.txt`, `tier4-tail.txt` (append-only DATA)
- `root-tree/` — committed bake skeleton (`rc/bin/termrc`, `lib/namespace`, `lib/netaddr.rc`, `lib/netip-*.rc`, `lib/ndb/local.template`, `usr/glenda/lib/profile`)
- Gates: `smoke-r9.expect`, `smoke-r9-net.expect`, `tier1-r9.expect` (aarch64 template), `services-r9.expect`, `churn-r9.expect`, `netsoak-r9.expect`, `stress-r9.expect` (+ retained `*.log` captures)
- Fixtures/helpers: `churncats.c`, `churnmark.c`, `churnsem.c`, `randread.c` (18-01 substrate driver), `netblast.c`, `drawsmoke.c`, `smokesleep.c`, `replay-{rio,acme,sam}.rc` (donor syscall replays), `services-sweep.rc`, `services-termrc`, `bootfs-payload.txt` (DESK-03 manifest), `coverage-sweep.sh` + `dir-coverage.txt`
- `README.md` — the venue's how-to

**`nix/` (flake app bodies):**
- x64: `boot-r9-x64.sh`, `smoke-r9-x64.sh`, `tier1-r9-x64.sh`, `smoke-r9-net-x64.sh`, `services-r9-x64.sh`, `soak-r9-net-x64.sh`, `churn-r9-x64.sh`, `stress-r9-net-x64.sh`, `net-doctor.sh`, `host-echo.c`
- Frozen aarch64: `boot-r9.sh`, `smoke-r9.sh`, `smoke-r9-net.sh`, `tier1-r9.sh`
- Build env: `setup.sh`, `toolchain.sh`, `build.sh`, `vm-build.sh`, `iso.sh`, `lima-harvey.yaml`, `minimega.nix`

**Repo-root per-objtype dirs (`amd64/`, `aarch64/`, `386/`, `arm/`, `mips/`, `mips64/`, `power/`, `power64/`, `riscv/`, `riscv64/`, `sparc/`, `spim/`, `spim64/`, `misp/`):**
- Install targets (`/amd64/bin`, `/amd64/lib`, `/amd64/include`) inside the build namespace
- `amd64/ABI.md` and `aarch64/ABI.md` — the frozen Harvey↔r9 syscall/Ureg contracts (canonical docs, committed at the root of the arch tree they govern)

## Key File Locations

**Entry Points:**
- `sys/src/9k/k10/main.c`: 9k kernel entry (`main` at line 145)
- `../r9/x86_64/src/l.S` + `../r9/x86_64/src/main.rs`: r9 kernel entry (mb2 via GRUB)
- `sys/src/9k/boot/boot.c`: boot(8) — first user process both kernels exec
- `rc/lib/rcmain`: rc startup master (copied verbatim at bake; cmp-guarded)
- `build/x86_64/stage-build.sh` → `build/x86_64/mkimage.sh` → `nix/boot-r9-x64.sh`: the one-command pipeline

**Configuration:**
- `flake.nix` + `flake.lock`: composition, apps, r9 pin
- `sys/src/9k/k10/k10cpu` (and siblings): declarative kernel configs (dev/port/dir/boot/rootdir sections)
- `../r9/x86_64/lib/config_q35*.toml`: boot profiles as data (dev set, qemu machine/cpu/smp/memory/accel, netdev modes, dist shape)
- `sys/src/mkfile.proto`, `sys/src/mkfile`, per-arch `mkfile` includes; devshell exports `objtype=amd64`, `FORCERCFORMK=1`

**Core Logic:**
- `sys/src/9k/port/{chan,proc,dev,sysfile,sysproc,segment,qio}.c`: namespace/scheduler/syscalls
- `../r9/port/src/{namec,pgrp,fdgrp,devmnt,paqfs,virtio}.rs` + `devip/`: the Rust kernel's portable core
- `../r9/x86_64/src/{proc,syscall,exec,usermem,swtch,trap}.rs`: x86_64 process/exec substrate
- `sys/src/libc/9syscall/sys.h`: syscall numbers (BIND=2, MOUNT=46, NMOUNT=54 …)
- `amd64/ABI.md`: the whole kernel/user boundary for amd64

**Testing/Gates:**
- `build/x86_64/*.expect` / `build/aarch64/*.expect`: qemu serial gates
- `../r9/xtask/src/main.rs` (`boot-test`, `ureg-check` subcommands): kernel-side acceptance harness
- `../r9/port/tests/` + `../r9/port/src/*_tests.rs`: host-run kernel unit pins
- `build/aarch64/markers/`: artifact+execution marker ledgers (tier evidence)
- `.github/workflows/`: `harvey.yml` (nix pipeline), `phase6.yml` (tier CI, four legs), `r9-shell.yml` (x64 shell/net CI)

**Docs:**
- `.planning/PROJECT.md`, `.planning/ROADMAP.md`: milestone/phase state
- `build/REPRODUCIBILITY.md`: bit-reproducible kernel discipline
- `sys/man/6/style`: the C style guide (tabs, lowercase, nil, exits)
- `../r9/HowItWorks.md`, `../r9/UNSAFE-AUDIT.md`, `../r9/docs/`

## Naming Conventions

**Files (C/userspace):**
- All lowercase, no underscores: `bgetrune.c`, `devcons.c`; one function per file in libraries
- Commands: `sys/src/cmd/<name>.c` single-file; `sys/src/cmd/<name>/` multi-file
- Kernel devices: `sys/src/9k/port/dev<name>.c`; kernel asm: `l64<what>.s` (e.g. `l64syscall.s`)
- Kernel configs: `k10<variant>` (no extension); r9 profiles: `config_<platform><variant>.toml` (`config_q35shell.toml`)
- Objects are toolchain-suffixed (`bread.6`), never `.o`

**Files (Rust/r9):**
- snake_case modules mirroring 9k names: `namec.rs`, `devmnt.rs`, `devip.rs`; devices cfg-gated `dev_<name>`

**Files (build/gates):**
- Gates: `<subject>-r9[-<variant>].expect`; app scripts in `nix/` match flake app names (`smoke-r9-x64.sh`)
- Images: `bootfs[-<contract>].paaq`→`bootfs-chain.paq`, `bootfs-tier.paq`…
- Tier data: `tier<N>-<class>.txt`

**Directories:**
- Lowercase concatenated: `libmemdraw`, `libthread`, `root-tree`, `hostbin-<os>-<arch>`
- The 64-bit ARM arch is ALWAYS `aarch64` (project decision; in-tree `arm64` vestiges are quarantined)

**Types/consts:** `typedef struct Foo Foo;` Capitalized; constants Uppercase (`DMDIR`, `OREAD`); `nil` never `NULL`; see `.planning/codebase/CONVENTIONS.md` (embedded in `AGENTS.md`) and `sys/man/6/style`

## Where to Add New Code

**New userspace command:**
- Code: `sys/src/cmd/<name>/<name>.c` (+ `mkfile` with `TARG=<name>`; `</sys/src/cmd/mkone` single, `mkmany` multi; add dir to `sys/src/cmd/mkfile` SUBDIRS if recursive build must reach it)
- Tier gating: append to the right `build/x86_64/tiers/tier*.txt` (append-only; growth is a reviewable diff)
- Bake into an image: add a copy line/section in `build/x86_64/mkimage.sh` + manifest entry

**New library:**
- `sys/src/lib<name>/` (OFILES with `$O`, `LIB=/$objtype/lib/lib<name>.a`, include `</sys/src/cmd/mksyslib`), public header `sys/include/<name>.h` with `#pragma src/lib`

**New 9k kernel device:**
- `sys/src/9k/port/dev<name>.c` implementing the `Dev` vtable (`portdat.h:198`)
- Register in the config's `dev` section (`sys/src/9k/k10/k10cpu`, and `k10t1` if the tier venue needs it) — `sys/src/9k/mk/parse` regenerates devtab

**New r9 kernel device (in `../r9`):**
- `../r9/port/src/dev<name>.rs` mounted `#[cfg(dev_<name>)] pub mod dev<name>;` in `../r9/port/src/lib.rs`
- Predeclare the cfg in root `Cargo.toml` `[workspace.lints.rust] unexpected_cfgs` BEFORE first code use
- Add the device to the relevant `../r9/x86_64/lib/config_q35*.toml` `dev` list (data, not Rust edits)

**New baked root-fs file:**
- Committed skeleton: `build/x86_64/root-tree/<path>` + a `stage_*` section + manifest assert in `build/x86_64/mkimage.sh`
- Repo-content copies (fonts, timezone) copy at bake time from `lib/font/bit`, `adm/timezone` — never duplicate under root-tree/

**New boot gate:**
- Expect driver: `build/x86_64/<name>.expect` (fresh capture, stage asserts, process-group teardown, exit 0/1)
- App body: `nix/<name>.sh`; register the app in `flake.nix` (respect hasAttr gating — never widen the r9 attr list ahead of a pin advance)

**New r9 boot profile:**
- `../r9/x86_64/lib/config_<name>.toml` (machine/cpu/accel/dev sections — new platforms are data, not Rust edits)

**Per-arch userspace asm:**
- `sys/src/libc/<objtype>/` (and `sys/src/9k/<mach>/` kernel-side)

**Man page:**
- `sys/man/<1-9|9k>/<name>` (troff)

## Special Directories

**`.build/`:**
- Host-side binaries (`.build/hostbin-<os>-<arch>/bin` — host `ar`, `pcc`, `mkpaqfs` at `.build/bin/`), build logs, staged manifests
- Generated: yes; Committed: no (gitignored)

**`build/x86_64/out/` and `build/x86_64/smoke-out/`:**
- Baked-image staging inputs (chain binaries vs marker tree — deliberately separate so the real init never overwrites the marker)
- Generated: yes; Committed: no

**`sys/src/9k/k10/.tier1-stage/`, `.draw-stage/`:**
- Staged embed copies for double-named rootdir entries (a source embedded twice must ride a copy — parse keys C symbols off source paths)
- Generated: yes (by venue builds)

**`graphify-out/`, `.planning/graphs/`:**
- Code-knowledge-graph artifacts for navigation
- Generated: yes; partially committed

**`../r9/target/`, `../r9/result/`:**
- Rust build outputs / nix result symlinks — never committed

**Repo-root `n/`, `fd/`, `env/`, `mnt/`, `tmp/`, `mail/`, `cron/`, `lp/`, `dist/`, `unix/`, `bbin/`, `acme/`, `usr/`:**
- Plan 9 namespace root-tree content the repo doubles as (mounted as `/` inside the build namespace); mostly static content, committed

---

*Structure analysis: 2026-11-13*
