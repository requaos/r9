# Phase 5: Shell Boot + One-Command Integration - Research

**Researched:** 2026-11-02
**Domain:** Plan 9 boot chain porting (boot(8)→init→rc) onto the r9 Rust kernel; kernel namespace machinery (bind/mount/9P client); Nix flake composition; expect-based boot smoke testing
**Confidence:** HIGH (all load-bearing claims verified by reading both repos' sources this session)

## Summary

Phase 5 turns r9 into a real Plan 9 kernel environment and delivers the project Core Value: one `nix run` command boots r9 + the real Harvey aarch64 boot chain to an interactive `% ` rc prompt. The work splits cleanly into four streams:

1. **r9 kernel (Rust, `../r9`)**: the big one. r9 today has *no* name resolution layer — `sys_open` routes paths by hardcoded prefix matching (`"cons"`, `"/proc/…"`, else devroot) [VERIFIED: ../r9/aarch64/src/syscall.rs:385-433]. This phase builds the real thing: per-process namespaces (pgrp equivalent + mount table + union walk), the kernel 9P client (devmnt) so `mount(fd,…)` works, and the device set the boot chain needs (devcons read+cooked+consctl, devenv per-pgrp, devdup, devsrv, devpipe, devproc beyond-note). Harvey's own `sys/src/9k/port/` C sources are the in-repo reference implementation for every one of these.
2. **Harvey userland (Plan 9 C)**: build `boot`, `init`, `paqfs` for aarch64. `init` and `paqfs` are standard mk targets; `boot` has **no standalone mkfile** — it's currently built only as part of 9k kernel builds via `bootmkfile` + an awk-generated method table [VERIFIED: sys/src/9k/mk/bootmkfile:30-34, sys/src/9k/mk/parse:142]. An additive aarch64 boot build is new wiring. The already-existing `embed.c` boot method (pipe+fork+exec `/boot/paqfs -iv <image>`, return pipe fd) is the D-04 precedent — and needs no modification, only a method table that names it.
3. **Image + composition glue (Harvey)**: paqfs root-tree skeleton (scoped termrc/rcmain/namespace + mount-point dirs + subset binaries), baked by the existing host `mkpaqfs`; new Harvey flake app composing userland build (lima on darwin, native `unshare` on linux) + r9 kernel from the flake input + qemu launch.
4. **Gates**: D-14 note-race debug first (Wave 0), donor-kernel syscall trace of the real chain (measure-first), expect smoke app, GH Actions linux CI leg, reproducibility docs.

**Primary recommendation:** Sequence kernel work behind execution gates exactly as Phase 4 did — donor trace first (the measure-first syscall discipline is carried-forward locked policy), then namespace core → devpipe/devdup → devmnt (gated by boot's `fversion`+`mount` to the userspace paqfs server) → devcons read/cooked (gated by the interactive prompt) → devenv/devproc completion (gated by the D-09 command sweep). The whole boot chain fails soft by design everywhere except `connectlocal`'s binds (which the embed method never runs) — exploit that: get boot's prints flowing before any namespace machinery is correct.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Boot chain shape (user decisions)**
- **D-01:** Port the real Harvey boot chain — `boot(8)` runs as pid 1, execs the real `/bin/init`, which fork/exec's `/bin/rc` in its `for(;;)` restart loop. No minimal staged-init shortcut. (Reversibility: costly)
- **D-02:** Full `boot(8)` with auth stubs — real flow runs: `opencons`/`bindenvsrv`, method pick, `doauth` (stubbed), `rfork(RFNAMEG)`, `connectroot` (`fversion`), `nsinit` (bind/mount), `execinit`. Auth never fatals in the embedded path. Contract: "auth paths never fatal, hostowner gets a sane default". (Reversibility: reversible)
- **D-03:** r9 grows full namespace machinery: bind + mount + devmnt. Per-process namespaces (pgrp equivalent, mount table, union walk, MREPL/MAFTER/MBEFORE/MCREATE) plus the kernel 9P client. `RFNAMEG`/`RFFDG` rfork flags come with it. boot.c runs unmodified. (Reversibility: one-way)
- **D-04:** Embedded root follows the `connectpaq` precedent: boot's method pipes + forks + execs a **userspace paqfs 9P server** (`sys/src/cmd/paqfs/`, built for aarch64) over the initrd image, returns the pipe fd; `connectroot`/`nsinit` then `fversion` + `mount` it at `/root` via devmnt and bind onto `/`. Zero boot.c divergence; small additive `Method` entry. (Reversibility: costly)
- **D-05:** Interactive entry contract: `init -t` + scoped termrc → `rc -i`. Scoped `/rc/bin/termrc` skips network/window machinery, ends in `exec rc -i` → rc bootstraps `/rc/lib/rcmain` → `prompt=('% ' ' ')` → `. -i '#d/0'` (devdup mandatory). Smoke asserts the literal `% ` prompt.

**Shell environment (user decisions)**
- **D-06:** Full devcons — cooked line discipline (line buffering, erase, kill-line), DEL posts "interrupt" note to the process group, plus `/dev/consctl` raw-mode switching. The userland read path from EL0 is new this phase.
- **D-07:** devenv with real per-pgrp env semantics — `#e` keyed by process group; `rfork(RFENVG)` shares, `RFCENVG` copies.
- **D-08:** Ship a scoped `/lib/namespace` (binds for `#e`→/env, `#p`→/proc, `#d`/dev entries, `/bin` union entries) so init's `newns(user, 0)` really executes.
- **D-09:** Smoke drives a scripted command sweep: assert `% `, then `echo` exact string, `ls /bin` subset presence, `ps` (devproc must expose status/args), `cat` of a known file.

**Flake composition + CI (user decisions)**
- **D-10:** Harvey's flake owns composition — new app (e.g. `nix run .#boot-r9`) builds aarch64 userland (lima on darwin, native on linux), pulls r9 kernel from flake input, bakes initrd, launches qemu. Phase 4 r9→Harvey env glue (`R9_HARVEY_DIR`, workspace-relative initrd, ureg-check `../harvey` default) **inverts** to Harvey→r9. (Reversibility: one-way)
- **D-11:** r9's flake exposes a kernel package only (`packages.kernel-virt`: built aarch64 ELF + committed DTB). Harvey's app bakes initrd with its own `mkpaqfs` and launches qemu itself. Harvey never runs r9's xtask.
- **D-12:** Shell smoke = expect script + flake app in Harvey. r9's `cargo xtask boot-test` profiles (virt/virtexec/virtmulti/virtnotes/virtpanic) stay unchanged, r9-side.
- **D-13:** CI = GH Actions linux leg, full pipeline only, on ubuntu-latest (native `unshare` namespace build, qemu TCG). **No darwin leg in CI** — darwin verified by documented manual command. r9 pinned via `flake.lock`; dev flow uses `--override-input`. This amends ROADMAP SC#3's "CI green on both": CI-green means linux; darwin is documented-and-run-by-hand.

**Known-flake policy (user decisions)**
- **D-14:** Note-timeout race gets a debug-first Wave 0 (`/gsd-debug` discipline) before the shell path depends on notes. Pre-existing (~1/3 virtmulti runs, reproduces on pre-fix base b4f858a per 04-REVIEW-FIX.md).
- **D-15:** Explicit stack-growth smoke binary — small 7c deep-recursion binary in the image, forces multiple stack page-ins, prints a marker.
- **D-16:** Phase-04 deferred #2 (harness qemu cleanup) enters scope. Deferreds #3/#4 + 8 open Info findings ride the milestone backlog.

**Carried forward (locked in roadmap/prior phases — do not re-litigate)**
- qemu shape: `-M virt,gic-version=2 -cpu cortex-a72 -smp 1 -nographic`, ELF `-kernel`, committed virt-gicv2 DTB at `0x40000000`; TCG on darwin (HVF rejects GICv2 on qemu 11.1.2); `-m ≤ 1G` (pagealloc bitmap ceiling).
- initrd delivery = qemu generic `-device loader` + fdtput-patched DTB; `mkpaqfs` produces, r9's no_std paqfs reader consumes; adler32 seed-0 quirk documented.
- Measure-first syscall discipline (Phase 4 D-14): trace the real boot+init+rc+subset path on the 9front donor kernel, implement exactly the measured set; loud-unimplemented default stays.
- Proof standard: serial/expect evidence + marker assertions + human checkpoint at the gate; canonical evidence in Harvey `.planning/phases/05-*/` with r9 commit SHA recorded.
- r9 commits land on `main` of `requaos/r9`; HowItWorks.md updated with material changes; r9 follows its rustfmt.toml, Harvey follows Plan 9 C style.
- `aarch64` naming everywhere.
- The 16-binary shell subset (rc, grep, ipconfig, factotum, cat/cp/date/echo/ls/mkdir/mv/ps/rm/sed/test/wc) is built and executed on the donor kernel (Phase 2 TC-05) — the image's binary pool; `boot`, `init`, `paqfs` join the aarch64 build set this phase.
- **amd64-boot safety constraint:** `boot.c`/`init.c`/`rcmain` are shared with the Phase-1-green 9k amd64 build — all changes additive and objtype-gated; amd64 boot behavior must not change.
- KERNDATE/reproducibility policy (Phase 1, `build/REPRODUCIBILITY.md`) applies to INT-03: documented commands + pinned toolchains (r9 via flake.lock pin; qemu/plan9port via nixpkgs pin) — process-level reproducibility.
- virtio-9p stays v1.x; devmnt (D-03) is its prerequisite, not its implementation.
- Apple host 16K pages / guest granule + `CTR_EL0` architectural discovery (Phase 2 D-03) apply to any new cache/page maintenance r9 grows.

### the agent's Discretion
- Embedded boot-method naming, argv contract (how the kernel's exec of `/boot/boot` selects it), and method-file placement — additive, modeled on `paq.c`.
- Auth stub granularity (objtype-gated file swap vs runtime degrade-through) — the contract is D-02's "never fatal in the embedded path".
- paqfs image root-tree skeleton beyond the mandated files (`/bin/*`, `/boot/{boot,paqfs}`, `/rc/lib/rcmain`, `/rc/bin/termrc`, `/lib/namespace`, `/adm/timezone/local`, `/usr/…/lib/profile`, `/tmp`).
- devproc file surface beyond the D-09-forced status/args (+ existing note, + ctl for init's `pri 10` write which degrades gracefully) — trace-driven.
- Expect-script mechanics, marker strings, flake app naming, and wave ordering within the phase (subject to D-14's debug-first ordering).
- How qemu cleanup (#2) is implemented (process-group kill, socket cleanup, timeout policy).

### Deferred Ideas (OUT OF SCOPE)
- virtio-9p host mount; boot(8)'s disk/network methods; real auth (factotum handshake, auth server, secstore); factotum/ipconfig in the boot image; SMP via PSCI, virtio-net/blk, RTC, PCIe ECAM; rio/acme/devdraw/devmouse; amd64-on-r9; 9k-amd64 pool-panic re-test; Phase-04 deferreds #3/#4 + 8 open Info findings; raspi3b-on-qemu-11.1.2 upstream defect; HVF acceleration; upstream contribution flow to original r9.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| INT-01 | One command (`nix run` app or `cargo xtask`) boots r9 + Harvey aarch64 userland to interactive `/bin/rc` over PL011 in qemu | D-10 composition app on Harvey's existing `mkScriptApp` pattern [VERIFIED: flake.nix:12-25]; qemu launch flags documented from r9's config TOML [VERIFIED: ../r9/aarch64/lib/config_virtexec.toml:17-43]; boot chain sources fully mapped (see Architecture Patterns) |
| INT-02 | Expect-style smoke test asserts the shell prompt, runnable on darwin-aarch64 and linux | expect already in Harvey's devshell [VERIFIED: flake.nix:82]; legacy heredoc-expect precedent [VERIFIED: build/build.bash:27-79]; D-09 command sweep markers defined in CONTEXT |
| INT-03 | Reproducible-from-clean-checkout verified for both repos (documented commands, pinned toolchains) | KERNDATE policy + host-side derivation [VERIFIED: build/REPRODUCIBILITY.md:1-40]; r9 pin via flake.lock; qemu/plan9port/rust via nixpkgs/rust-overlay pins [VERIFIED: flake.nix:4, ../r9/flake.nix:4-6,20-21] |
| INT-04 | Harvey's flake consumes r9 as a proper flake input (git URL, pinned rev) — no `../r9` paths or submodules | Harvey flake currently has a single input (nixpkgs) [VERIFIED: flake.nix:4]; r9 flake has no `packages.*` outputs yet — D-11's `packages.kernel-virt` is new [VERIFIED: ../r9/flake.nix:44-93]; r9 remote is `git@github.com:requaos/r9.git`, HEAD `54e8c1079933` [VERIFIED: git remote/log this session] |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Name resolution (namec/walk + union dirs + mount table) | r9 kernel `port` crate | aarch64 syscall layer | D-03; replaces sys_open's hardcoded prefix routing [VERIFIED: ../r9/aarch64/src/syscall.rs:385-433] |
| Per-process namespace state (pgrp equivalent) + RFNAMEG/RFCNAMEG | r9 kernel (`Proc` grows pgrp/env fields) | — | Proc today has no pgrp/env fields [VERIFIED: ../r9/aarch64/src/proc.rs:167-237] |
| Kernel 9P client (devmnt: Tversion/Tattach/Twalk/Topen/Tread/Tstat/Tclunk over an fd) | r9 kernel | — | mount(fd) against userspace paqfs; reference sys/src/9k/port/devmnt.c (1206 lines) |
| Devices: devcons read+cooked+consctl+DEL-note, devenv per-pgrp, devdup, devsrv, devpipe, devproc ext | r9 kernel `port` crate | platform/virt PL011 RX ring | D-06/D-07; free-function device pattern is the established r9 idiom [VERIFIED: ../r9/port/src/devproc.rs:13-16] |
| Shared-pipe semantics across fork (devpipe queue) | r9 kernel | — | Chan::dup() value-copy diverges offsets today [VERIFIED: ../r9/port/src/dat.rs:63-77] — pipes need a shared Arc-backed queue |
| boot(8)/init/paqfs aarch64 binaries | Harvey userland build (lima guest namespace on darwin, native unshare on linux) | — | additive objtype-gated wiring; boot needs a new standalone aarch64 build (see Pitfall 2) |
| Root-tree skeleton + paqfs bake | Harvey `build/aarch64/` | host `mkpaqfs` (`.build/bin/mkpaqfs`) | mkpaqfs host build exists [VERIFIED: build/aarch64/mkpaqfs-host.sh:33-44] |
| Composition app (`nix run .#boot-r9`) + smoke app | Harvey flake.nix | r9 flake `packages.kernel-virt` (new) | D-10/D-11/D-12; mkScriptApp pattern |
| CI linux leg | Harvey `.github/workflows/` (new file, alongside legacy harvey.yml) | — | D-13 |
| Donor trace of the real chain | Harvey `build/aarch64/trace-syscalls.py` rig | 9front donor kernel | measure-first carried-forward policy; rig exists and documents its own pipeline [VERIFIED: build/aarch64/trace-syscalls.py:1-46] |

## Standard Stack

**This phase installs no new external packages.** Every dependency is either in-repo source (Plan 9 C in Harvey; Rust no_std in r9) or a host binary supplied by the two flakes' existing pins. No crates, no npm, no pypi additions are required by the research below; r9's existing `adler2` dependency (Phase 4) is unchanged.

### Core (in-repo / already-pinned)

| Component | Version/Pin | Purpose | Why Standard |
|-----------|-------------|---------|--------------|
| Harvey `sys/src/9k/boot/` (boot.c, embed.c, local.c, bootauth.c, aux.c, settime.c…) | repo HEAD | the pid-1 boot chain being ported | D-01/D-02: real chain, unmodified |
| Harvey `sys/src/cmd/init.c` | repo HEAD | real init (`-t` → terminal service → termrc → rc) | D-05 |
| Harvey `sys/src/cmd/paqfs/` (paqfs.c + mkpaqfs.c) | repo HEAD | userspace 9P server over the initrd + host-side image baker | D-04 |
| Harvey vendored 7a/7c/7l | repo (Phase 2) | aarch64 Plan 9 toolchain | TC-02/TC-03 |
| r9 `port` + `aarch64` crates | `requaos/r9` main @ `54e8c1079933` (re-pin at phase start) | kernel | Phase 4 output |
| r9 rust toolchain | `../r9/rust-toolchain.toml` via rust-overlay | kernel builds | R9-05; single source of truth, never restated in the flake [VERIFIED: ../r9/flake.nix:17-21] |

### Supporting (host tools — supplied by the flakes, nothing new to install)

| Tool | Supplied By | Purpose | When Used |
|------|-------------|---------|-----------|
| qemu-system-aarch64 | both flakes' nixpkgs pins | virt machine (TCG) | boot + smoke |
| expect | Harvey devShell [VERIFIED: flake.nix:82] | smoke driver | INT-02 |
| dtc (fdtput) | r9 devShell [VERIFIED: ../r9/flake.nix:52] | DTB /chosen patching | initrd delivery (Harvey's launcher reimplements/documents the same step per D-11) |
| lima + openssh + rsync | Harvey devShell (darwin) [VERIFIED: flake.nix:84] | namespace-build VM on darwin | userland builds |
| util-linux (unshare) | Harvey build app runtimeInputs [VERIFIED: flake.nix:62] | native namespace build on linux | linux userland builds + CI |
| python3 | devshell (already used by Phase 4 rig) | trace-syscalls.py | measure-first trace |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Harvey expect smoke (D-12) | Extend r9's Rust boot-test harness to the shell | Rejected by D-12: INT-02 says "expect-style" literally; r9's harness stays kernel-focused; Phase 1 expect+qemu lineage is Harvey-side |
| `packages.kernel-virt` from r9's flake (D-11) | Harvey builds the kernel from the r9 flake source with its own cargo invocation | Rejected by D-11: r9 owns its toolchain pin; Harvey consumes a built artifact |
| Real factotum handshake | glenda no-op path | Deferred (real auth is a later milestone); the no-op is already the code's natural degradation [VERIFIED: sys/src/9k/boot/bootauth.c:17-20] |

**Installation:** none (no new packages). The one new flake *input* is `inputs.r9` (git URL, flake.lock-pinned) — see INT-04.

**Version verification:** no registry packages to verify. The pins that matter are flake.lock entries (nixpkgs, rust-overlay, r9) — the planner's clean-checkout tasks will refresh/record them. qemu 11.1.2 facts are carried-forward locked decisions from Phases 3/4 (CONTEXT.md canonical refs); re-verify only if the nixpkgs pin moves qemu.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| *(none — this phase adds no external packages)* | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

The only new external reference is the `inputs.r9` flake input pointing at `github.com:requaos/r9` (the project's own working fork, locked in PROJECT.md) — not a registry package. If the planner finds a need for any new crate/nixpkgs derivation during planning, it must run the legitimacy gate before landing in a plan.

## Architecture Patterns

### System Architecture Diagram

```
HOST (darwin-aarch64 or linux)
│
│  nix run .#boot-r9                       nix run .#boot-r9-smoke
│  └──────────────────────────────┐        └──────────────┐
│  Harvey flake app               │        Harvey flake app (expect)
▼                                 ▼                       ▼
┌─────────────────┐   ┌────────────────────────┐   ┌────────────────┐
│ aarch64 userland│   │ r9 flake input (pinned) │   │ expect script  │
│ namespace build │   │ packages.kernel-virt    │   │ spawn qemu,    │
│ (lima / unshare)│   │ → ELF + virt-gicv2.dtb  │   │ assert markers │
└───────┬─────────┘   └───────────┬────────────┘   └───────┬────────┘
        │ binaries                │ kernel ELF             │ stdio
        ▼                         ▼                        │
┌──────────────────────────────────────────────┐          │
│ mkpaqfs root tree → bootfs.paq               │          │
│ (boot, init, paqfs, subset, rc files,        │          │
│  /lib/namespace, mount-point dirs)           │          │
└───────────────────────┬──────────────────────┘          │
                        ▼                                 ▼
        qemu -M virt,gic-version=2 -cpu cortex-a72 -smp 1 -nographic
          -kernel <r9 ELF> -dtb <fdtput-patched DTB>
          -device loader,file=bootfs.paq,addr=0x48000000
                        │
                        ▼  PL011 serial (stdio)
┌─────────────────────────────────────────────────────────┐
│ r9 kernel (aarch64, qemu virt)                          │
│  boot → mount_initrd → devroot                          │
│  exec /boot/boot (pid 1, EL0)                           │
│     │                                                   │
│     ▼  svc syscalls over the frozen ABI                 │
│  namespace core: namec/walk + pgrp mount table + union  │
│  devmnt (9P client) ── devpipe ◄── userspace paqfs      │
│  devcons devproc devenv devdup devsrv devroot           │
└─────────────────────────────────────────────────────────┘
                        │
        boot: opencons → bindenvsrv → pickmethod(embed)
          → authentication (glenda no-op) → rfork(RFNAMEG)
          → connectembed: pipe+fork+exec /boot/paqfs -iv /dev/rootfs
          → fversion → mount(fd,-1,/root,MREPL|MCREATE)
          → bind(/root,/,MAFTER|MCREATE) → execinit /aarch64/init -t
        init: newns(user,0) over /lib/namespace
          → rc -c ". /rc/bin/termrc; …" → termrc ends exec rc -i
          → rcmain → prompt '% ' → . -i '#d/0'  ← interactive shell
```

### Recommended Project Structure (new/changed files)

```
harvey/                              # this repo
├── flake.nix                        # + inputs.r9, + boot-r9 app, + smoke app
├── build/aarch64/
│   ├── root-tree/ or mkimage.sh     # paqfs root skeleton + bake (replaces staged-init out/ flow)
│   ├── boot-r9.sh / smoke-r9.expect # qemu launcher + expect driver
│   └── (trace real chain via existing trace-syscalls.py)
├── sys/src/9k/boot/                 # additive aarch64 boot build wiring (mkfile or mk fragment)
│   └── (embed.c reused as-is; new method table only)
├── lib/namespace → scoped variant   # D-08 (image ships its own scoped copy)
├── rc/bin/termrc (image-scoped)     # D-05 (ships in the image, not the repo's cfg/)
└── .github/workflows/r9-shell.yml   # D-13 linux CI leg (new file)

r9/                                  # sibling repo (kernel work)
├── port/src/
│   ├── namec.rs / namespace.rs      # NEW: name resolution + union walk + mount table
│   ├── pgrp.rs                      # NEW: per-process namespace/env groups
│   ├── devmnt.rs                    # NEW: 9P client
│   ├── devpipe.rs devdup.rs devsrv.rs devenv.rs  # NEW devices
│   ├── devcons.rs                   # EXTEND: read path, cooked mode, consctl, DEL note
│   └── devproc.rs                   # EXTEND: status/args/ctl + dir reads
├── aarch64/src/
│   ├── syscall.rs                   # dispatch grows to the measured set
│   └── proc.rs                      # pgrp/env/fd-group wiring; MAX_PROCS/MAX_FDS sizing
├── flake.nix                        # + packages.kernel-virt (D-11)
├── xtask/                           # initrd baking + ../harvey glue removed/inverted (D-10)
└── HowItWorks.md                    # material-change updates (header mandates it)
```

### Pattern 1: The connectembed precedent (D-04's model, already in-tree)

**What:** A boot `Method` whose `connect` pipes, forks, and execs a userspace 9P server over the root image, returning the pipe fd to `connectroot`.
**When to use:** The embedded root method for r9. `embed.c` exists and is compiled into 9k's bootlib already — it is *closer* to D-04 than paq.c (paq.c serves from `/dev/flash/ramdisk`; embed.c takes the image path from the method arg/boot arg).
**Verified source** [VERIFIED: sys/src/9k/boot/embed.c:24-74] — the full connect flow:

```c
int
connectembed(void)
{
	int i, p[2];
	Dir *dir;
	char **arg, **argp;

	dir = dirstat("/boot/paqfs");
	if(dir == nil)
		return -1;
	free(dir);

	dir = dirstat(paqfile);
	if(dir == nil || dir->mode & DMDIR)
		return -1;
	free(dir);

	print("paqfs...");
	if(bind("#c", "/dev", MREPL) < 0)
		fatal("bind #c");
	if(bind("#p", "/proc", MREPL) < 0)
		fatal("bind #p");
	if(pipe(p)<0)
		fatal("pipe");
	switch(fork()){
	case -1:
		fatal("fork");
	case 0:
		arg = malloc((bargc+5)*sizeof(char*));
		argp = arg;
		*argp++ = "/boot/paqfs";
		*argp++ = "-iv";
		*argp++ = paqfile;
		for(i=1; i<bargc; i++)
			*argp++ = bargv[i];
		*argp = 0;

		dup(p[0], 0);
		dup(p[1], 1);
		close(p[0]);
		close(p[1]);
		exec("/boot/paqfs", arg);
		fatal("can't exec paqfs");
	default:
		break;
	}
	waitpid();

	close(p[1]);
	return p[0];
}
```

Note the **fatal** binds of `#c` and `#p` — r9 must have devcons and devproc attachable by their `#`-names before boot reaches this point (contrast: boot.c's own `bind("#æ"/"#S", "/dev", MAFTER)` calls at lines 272-274 are unchecked/fail-soft).

**The two-views watch-out (CONTEXT specifics, confirmed):** the kernel execs `/boot/boot` by *walking* the paqfs tree (devroot), while the userspace paqfs server needs the *raw image bytes* as a file (embed.c's `dirstat(paqfile)` + paqfs `init(argv[0], …)` reads them). devroot today holds the image slice (`DevRoot::image()` returns it [VERIFIED: ../r9/port/src/devroot.rs:180-184]) but exposes no raw-image *file*. The phase must add one (e.g. a `/dev/rootfs` node) — planner defines both views.

### Pattern 2: The boot binary build (the missing mkfile)

**What:** How `boot$CONF.out` is built for 9k amd64 today, and what aarch64/r9 needs instead.
**Verified source** [VERIFIED: sys/src/9k/mk/bootmkfile:1-34]:

```
BOOTDIR=../boot
BOOTLIB=$BOOTDIR/libboot.a$O

BOOTFILES=\
	bootauth.$O\
	aux.$O\
	boot.$O\
	bootcache.$O\
	bootip.$O\
	local.$O\
	embed.$O\
	settime.$O\
	parts.$O\
	paq.$O\
	printstub.$O\
	usb.$O\
…
boot$CONF.out: ../mk/parse $CONF print.$O $BOOTDIR/boot.c $BOOTLIB
	awk -f ../mk/parse -- -mkbootconf $CONF > boot$CONF.c
	$CC $CFLAGS boot$CONF.c
	$CC $CFLAGS ../boot/printstub.c
	$LD -o boot$CONF.out boot$CONF.$O $BOOTLIB printstub.$O
```

`method[]`, `cpuflag`, `rootdir`, `bootdisk` are **generated** from the kernel config's `boot` section by `awk -f ../mk/parse -- -mkbootconf` [VERIFIED: sys/src/9k/mk/parse:142-160 emits `Method method[] = {` … `int cpuflag = …`]. `doauthenticate.c`, `getpasswd.c`, `nopsession.c` are **not** in BOOTFILES — 9k's boot authenticates via `bootauth.c`'s `authentication()` (see Pitfall 6).

For aarch64/r9 there is no kernel config; the phase adds an **additive, objtype-gated** aarch64 boot build (a small mkfile or mk fragment under `sys/src/9k/boot/` or a build/aarch64 script) that supplies the generated-equivalent bootconf by hand: a `method[]` naming the embed method first (so `rootserver` defaults to it), `cpuflag = 0` (terminal → `init -t` per D-05), `rootdir = "/root"`, `bootdisk = ""`. Nothing in the existing amd64 path changes (carried-forward amd64-boot safety constraint).

### Pattern 3: Method selection contract (argv vs seeded env)

**What:** How boot picks the embedded method without prompting.
**Verified source** [VERIFIED: sys/src/9k/boot/boot.c:321-375 (`rootserver`)]: it reads `#e/nobootprompt` first (if nonempty, its *content* is looked up as a method name → `HaveMethod`, no prompt), then `#e/bootargs`, then `argv[1]` (`if(reply[0] == 0 && arg != 0) strcpy(reply, arg);`), and only prompts via `outin()` when all are empty.

Recommendation (the agent's-discretion argv contract): **seed the initial environment** — r9's devenv root group carries at minimum `cputype=aarch64` (mandatory regardless: `execinit` builds `"/%s/init -t"` from `#e/cputype` [VERIFIED: sys/src/9k/boot/boot.c:208-213], and init's `readenv("#e/cputype")` → `setenv("#e/objtype", cpu)` [VERIFIED: sys/src/cmd/init.c:56-58]) plus `nobootprompt=embed` (or the chosen method name). One mechanism solves both the method selection and the cputype problem; argv stays a fallback. Alternative: kernel passes `embed` as argv[1] to `/boot/boot` — works, but leaves cputype unsolved. Don't do both inconsistently; pick env-seeding.

### Pattern 4: r9's device idiom — free functions, not the Dev trait

**What:** r9's `port` devices (devroot, devproc-lite) deliberately do **not** implement dat.rs's `Dev` trait — its skeleton is non-constructible (`Walkqid` has an unsized `_qids: [Qid]` field; `attach` is a static) [VERIFIED: ../r9/port/src/devroot.rs:10-17, ../r9/port/src/dat.rs:126-129]. Device operations are inherent methods/free functions, dispatched by hardcoded prefix routing in `sys_open`.
**When to use:** This phase's namespace core replaces the prefix routing with real name resolution. The planner must decide the dispatch shape (a hand-built devtab mapping `#c`→devcons etc. vs. growing the trait into a constructible form). The free-function idiom is proven; the *routing* is what's new. dat.rs documents the interim convention: `DEV_ROOT = 0`, `DEV_CONS = 1`, `DEV_PROC = 2` [VERIFIED: ../r9/port/src/dat.rs:14-20].

### Pattern 5: The boot flow end-to-end (what r9's syscalls must serve)

Verified against the sources; each step names its syscall/device demands:

1. `boot()` starts [VERIFIED: sys/src/9k/boot/boot.c:230-292]: `fmtinstall`, `opencons()` — close 0/1/2, **`bind("#c", "/dev", MBEFORE)`** (first bind syscall of the chain), open `/dev/cons` ×3.
2. `bindenvsrv()` — `bind("#ec", "/env", MREPL)` (unchecked; r9 has no `#ec` — fails soft), `bind("#e", "/env", MBEFORE|MCREATE)`, `bind("#s", "/srv/", MREPL|MCREATE)` — both unchecked [VERIFIED: boot.c:51-57].
3. `readfile("#e/cputype", …)` [boot.c:254] — devenv read; seeds execinit's path.
4. `usbinit(Dontpost)` — no-op when `#u/usb/ctl` absent [VERIFIED: sys/src/9k/boot/usb.c:130-137 `if(access("#u/usb/ctl", AEXIST) < 0 …) return`].
5. `pickmethod` → `rootserver` (Pattern 3) → `configembed`.
6. `bind("#æ",…)`/`bind("#S",…)` unchecked fail-soft [boot.c:272-274]; `partinit()` runs `readparts()` unless `readparts=0` — set `readparts=0` in the seeded env or tolerate its failure (it probes `#S` disks that don't exist; check parts.c behavior during execution).
7. `doauth` → `authentication()` → `/boot/factotum` absent → `glenda()` → write `#c/hostowner` (guarded `if(fd >= 0)`; failure prints, never fatals) [VERIFIED: sys/src/9k/boot/bootauth.c:17-20,56-72]. **The auth stub is: don't ship /boot/factotum in the image.** Zero C changes.
8. `rfork(RFNAMEG)` [boot.c:279] — namespace group split.
9. `connectroot` → `connectembed()` (Pattern 1) → **`fversion(fd, 0, buf, sizeof buf)`** [boot.c:141] — FVERSION(40) → devmnt Tversion to userspace paqfs (paqfs implements `[Tversion] rversion` [VERIFIED: sys/src/cmd/paqfs/paqfs.c:111-113]); `srvcreate("boot", fd)` → `create("#s/boot", 1, 0666)` + write fd number — **fatal** if devsrv create fails [VERIFIED: sys/src/9k/boot/aux.c srvcreate].
10. `nsinit` [VERIFIED: boot.c:151-199]: `bind("/", "/", MREPL)`; `fauth(fd, rp)` — FAUTH(10) may fail → `afd = -1` → fall through; **`mount(fd, afd, "/root", MREPL|MCREATE, rp)`** — fatal on failure: devmnt mount against the pipe fd; then `bind(rp, "/", MAFTER|MCREATE)` with `/root` fallback logic.
11. `settime(islocal=0, …)` — opens `#s/boot` (devsrv), `mount(f, afd, "/tmp", MREPL, rp)` (a second devmnt mount, onto `/tmp` — the image ships `/tmp`), stats it, `open("#c/time", OWRITE)` unchecked-ish (write failure → warning) [VERIFIED: sys/src/9k/boot/settime.c:36-63].
12. `swapproc` — `#c/swap` absent → warning, returns [VERIFIED: boot.c:377-390].
13. `execinit` [VERIFIED: boot.c:201-228]: `chmod("/srv/partfs.sdXX", 0600)` unchecked; `exec("/aarch64/init -t")`.
14. `init` [VERIFIED: sys/src/cmd/init.c:22-76]: `closefds()` closes 3..29 (harmless over a smaller table); `#p/%d/ctl` write `"pri 10"` — **warning-only** on failure; `readenv("#e/cputype")`; `setenv("#e/objtype", …)` — **create in devenv**; `cpenv("/adm/timezone/local", "#e/timezone")`; `readenv("#c/user")` — devcons user file; `newns(user, 0)`; terminal path: `execl("/bin/rc", "rc", "-c", ". /rc/bin/termrc; home=/usr/$user; cd; . lib/profile", nil)` [init.c:177-178]; `for(;;)` restart loop with `rfork(RFNOTEG)` children + `notify`/`noted(NCONT)` + `wait()`.
15. `newns(user, nil)` is in **libauth**, not libc [VERIFIED: sys/src/libauth/newns.c:113-117 → buildns:34-88]: `rfork(RFENVG|RFCNAMEG)` — **RFCNAMEG = fresh namespace copy**; `setenv("user"…)/setenv("home"…)`; parses `/lib/namespace`; ops = `bind`/`unmount`/`mount`/`cd`/`.`-include; default flags MREPL when no `-a`/`-b` [newns.c:176-177]; `$var` expansion reads `#e/<var>` [newns.c:320-362]. init links libauth already (`#include <auth.h>`), so no new link deps.
16. termrc (scoped, D-05) → `exec rc -i` → rc bootstraps `/rc/lib/rcmain` — compiled-in via `loadboot`, `Rcmain = "/rc/lib/rcmain"` [VERIFIED: sys/src/cmd/rc/plan9.c:27, sys/src/cmd/rc/exec.c:132-155,216] → `prompt=('% ' '\t')` default [VERIFIED: rc/lib/rcmain:5-10] → interactive path `. -i '#d/0'` [rcmain:23-28] — **devdup mandatory**.

### Pattern 6: Expect smoke mechanics

Legacy precedent [VERIFIED: build/build.bash:27-79] — heredoc expect spawning qemu directly, `expect -exact`/`send` pairs over stdio:

```expect
spawn qemu-system-i386 $FLAG_KVM -nographic …
expect -exact "Selection:"
send "2\n"
expect -exact "Plan 9"
```

Phase 1's discipline (carried): capture attaches before start; banner/marker grep is the source of truth; never trust the supervisor's exit status. D-09's sweep: assert `% `, then `echo`-marker, `ls /bin` subset names, `ps` output, `cat` of a known file — presence assertions, never interleaving order (Phase 4 D-13), timeouts sized for TCG on GH runners. qemu cleanup per D-16: fresh log per run (`rm -f` before launch — the stale-log rerun poison is documented in deferred-items #2), process-group kill, and on darwin the bracket-fixed pkill pattern (`[q]emu-system-aarch64…`) so pkill can't self-match [carried: STATE.md Phase 03 entry].

### Pattern 7: The lima/namespace build flow (unchanged, extended)

The aarch64 userland build runs in the guest's Plan 9 namespace: host driver → rsync checkout → ssh → `nsrun.sh --wrap` → `sudo unshare -m` → bind `/sys` → `su` back to the repo user with captured exports [VERIFIED: build/aarch64/vm-run.sh:1-80, nsrun.sh:1-60]. `subset-build.sh` builds libs in `#pragma lib` closure order then the 16 binaries and rsyncs `aarch64/bin` back, asserting each binary exists [VERIFIED: subset-build.sh:118-184]. This phase extends the same flow to `boot`/`init`/`paqfs` and the image bake. On linux CI the identical guest script runs under native `unshare` (no lima) — the guest-script discipline (single committed file, regenerated per run) already abstracts the transport.

### Anti-Patterns to Avoid

- **Hardcoded path routing in syscalls** — the Phase 4 sys_open prefix-match was an explicit scaffold; do not extend it for the new devices. The namespace core owns path resolution (D-03's one-way door).
- **Chan value-copy for shared objects** — `Chan::dup()` documents that "the streaming offset diverges after the fork" [VERIFIED: ../r9/port/src/dat.rs:63-77]. Fine for the Phase-4 smoke; **wrong** for pipes (parent/child share a queue) and for fd 0/1 dup across fork. devpipe chans must reference a shared object (Arc), or the connectembed flow deadlocks silently.
- **Prompt-driven boot** — if `nobootprompt`/`bootargs`/argv all miss, `rootserver` loops on `outin()` forever and the smoke hangs until timeout. Seed the env (Pattern 3).
- **Shipping /boot/factotum** — its mere presence flips `authentication()` into the real factotum-exec path (access(…AEXEC) check [VERIFIED: bootauth.c:17]). The scoped image omits it.
- **Byte-order/interleaving assertions in the smoke** — markers under preemption+TCG arrive unordered; assert presence only.
- **New `#ifndef` guards / `//` comments / `exit()` in Harvey C** — Plan 9 style rules (AGENTS.md conventions); r9 side follows its rustfmt.toml; the styles never cross the repo boundary.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Userspace 9P server over the initrd | a new server | `sys/src/cmd/paqfs/paqfs.c` — full T-message set incl. Tauth (returns Enoauth) + `-i` stdio mode [VERIFIED: paqfs.c:105-124,162-213] | D-04's mount target; proven against 9k |
| paqfs image baking | a new image tool | host `mkpaqfs` via `build/aarch64/mkpaqfs-host.sh` [VERIFIED: lines 33-44] | byte-format owner; r9's reader already consumes it (adler32 seed-0 quirk settled Phase 4) |
| DTB initrd patching | a new DTB tool | the fdtput two-32-bit-cell recipe [VERIFIED: ../r9/xtask/src/config.rs:385-434] | qemu 11.1.2 never delivers `-initrd` for ELF kernels (Phase 4 D-01 amendment); Harvey's launcher documents/reuses this shape per D-11 |
| Syscall set selection | guessing from source reading | the donor trace rig `build/aarch64/trace-syscalls.py` [VERIFIED: header pipeline lines 1-46] | carried-forward measure-first policy; the real chain's set (expect pipe/dup/wstat/RFNAMEG-class entries) comes from the trace |
| Namespace semantics (union walk, MCREATE, mount table) | inventing from first principles | Harvey's own `sys/src/9k/port/{chan.c,pgrp.c,devmnt.c,dev.c,sysfile.c}` (1693/322/1206/461/1472 lines) + `lib/namespace` + `libauth/newns.c` | in-repo reference implementation with the exact flag semantics; port don't redesign |
| Cooked console line discipline | a new line editor | `sys/src/9k/port/devcons.c` (1405 lines: cooked/raw, consctl, DEL→interrupt note) | D-06 demands the real semantics |
| Smoke driver | a Rust socket harness Harvey-side | expect (devshell already ships it) + build.bash heredoc pattern | D-12; INT-02's literal "expect-style" |

**Key insight:** this phase's novelty is *placement* (r9 Rust kernel + flake composition), not *design* — every protocol, file surface, and boot behavior has an in-repo C reference. When in doubt, port the 9k semantics literally.

## Common Pitfalls

### Pitfall 1: `#e/cputype` must be seeded or boot builds `//init`
**What goes wrong:** `readfile("#e/cputype", cputype, …)` leaves `cputype=""` when devenv is empty (readfile fails soft, `buf[0]=0` [VERIFIED: sys/src/9k/boot/aux.c:80-93]); `execinit` then sprints `"/%s/init -%s%s"` → `"//init -t"` and the exec fails → `fatal(cmd)`.
**Why it happens:** on 9k the kernel's config-env device (`#ec`) seeds cputype from the kernel build; r9 has no `#ec`.
**How to avoid:** kernel-seeded initial devenv root group: at least `cputype=aarch64`, plus `nobootprompt=<embed-method-name>` (Pattern 3). init's own `readenv("#e/cputype")` consumes the same seed.
**Warning signs:** boot prints reach `version…`/mount succeeds then `fatal //init`.

### Pitfall 2: boot has no standalone mkfile — method table is awk-generated
**What goes wrong:** assuming `cd sys/src/9k/boot && objtype=aarch64 mk` works — there is no mkfile there; `method[]`, `cpuflag`, `rootdir`, `bootdisk` come from `awk -f ../mk/parse -- -mkbootconf $CONF` against a *kernel* config.
**How to avoid:** Pattern 2 — additive aarch64 bootconf (hand-written equivalent of the generated file) + a build step that compiles `boot.c` + libboot members with 7c and links with 7l. Keep it objtype-gated; the amd64 bootmkfile path is untouched.
**Warning signs:** link errors for `method`, `cpuflag`, `rootdir`; or worse, an amd64-visible behavior change (forbidden).

### Pitfall 3: MAX_PROCS=8 / MAX_FDS=16 sizing
**What goes wrong:** the steady-state process set is pid1(boot→init), the **persistent** paqfs server child, rc, and a transient command child; pipelines and rc's own forks push higher. r9 today: `MAX_PROCS: usize = 8`, `MAX_FDS: usize = 16` [VERIFIED: ../r9/aarch64/src/proc.rs:52-54]. init's `closefds()` closes 3..29 [VERIFIED: sys/src/cmd/init.c:263-269] — harmless (closes fail), but no fd ≥16 can ever exist, and pipe+dup flows in connectembed transiently hold several.
**How to avoid:** size both tables for the traced chain with headroom (the trace counts; rc pipelines like the embedded rcmain's `fn ps` use several fds and procs at once). Make exhaustion loud, never silent.
**Warning signs:** `open: no free fd` errstr at the shell; `proc table full` during `ps` or pipelines.

### Pitfall 4: pipes must share state across fork — Chan::dup() diverges
**What goes wrong:** connectembed forks; child dups pipe ends onto 0/1 and execs paqfs; parent fversion/mounts its end. If the two ends don't reference one shared queue, the 9P exchange never crosses.
**Why it happens:** `Chan::dup()` is a value copy — "Plan 9's shared-Chan semantics (one offset, refcounted) are approximated by a value copy" [VERIFIED: ../r9/port/src/dat.rs:63-77].
**How to avoid:** devpipe chans carry an Arc-backed queue (9k reference: `sys/src/9k/port/devpipe.c` + `qio.c` Queue). The rfork fd-copy path clones the Arc, not the bytes.
**Warning signs:** boot hangs at `paqfs…` or `version…` with no traffic; fversion times out.

### Pitfall 5: devproc must grow directory reads + status/args/ctl
**What goes wrong:** D-09's `ps` assertion. `/bin/ps` does `chdir("/proc")`, opens `"."` (a **directory read** of the devproc root), then reads `<pid>/status` (tokenize ≥ 12 fields) and `<pid>/args` [VERIFIED: sys/src/cmd/ps.c:33-40,63-75,121-138]. devproc-lite today serves only `<pid>/note`, write-only, walk-errors everything else [VERIFIED: ../r9/port/src/devproc.rs:1-11].
**How to avoid:** extend devproc: root dir listing of live pids, per-pid dir listing, `status` (≥12 whitespace fields matching ps's tokenizer), `args`, keep `note`, add `ctl` accepting `pri 10` (init degrades to a warning without it — acceptable but noisy).
**Warning signs:** `ps` prints nothing / errors `/proc`; init prints the pri warning.

### Pitfall 6: auth stub = absence, not new code
**What goes wrong:** writing stub C for auth. The chain already degrades: `/boot/factotum` absent → `glenda()` sets hostowner from `$user` or `"glenda"`, guarded `#c/hostowner` write [VERIFIED: sys/src/9k/boot/bootauth.c:17-20,56-72]; `fauth` failing → nsinit mounts unauthenticated (`afd=-1`) [VERIFIED: sys/src/9k/boot/boot.c:165-172]; `doauthenticate.c` isn't even linked into 9k boot [VERIFIED: bootmkfile BOOTFILES]. FAUTH(10) can stay loud-unimplemented (-1) — the fall-through is the designed path (the trace settles whether paqfs's Tauth/Enoauth is even exercised).
**How to avoid:** don't ship `/boot/factotum`; don't write auth C. Hostowner sane-default = `glenda`.
**Warning signs:** any auth-related `fatal` — means something shipped factotum or fauth succeeded unexpectedly.

### Pitfall 7: DEL/note delivery at the prompt depends on the Wave-0 race fix
**What goes wrong:** the shell leans on notes (rc's `sigexit`, DEL→interrupt at every prompt, init's `notify(pinhead)`). The pre-existing note-timeout race (~1/3 virtmulti runs) would make the smoke flaky and erode the milestone claim.
**How to avoid:** D-14 ordering — the `/gsd-debug`-discipline Wave 0 lands before the shell path is wired to notes; smoke markers assert presence so a flake is a loud fail, not a hang.
**Warning signs:** `note-timeout` in captures; DEL killing rc instead of interrupting.

### Pitfall 8: mount points must exist in the image tree
**What goes wrong:** `bind`/`mount` targets must pre-exist as directories: `/root` (nsinit's mount), `/env` `/proc` `/dev` `/srv` `/fd` `/bin` `/tmp` `/mnt` (namespace file + settime's `/tmp` mount). The paqfs image is read-only — nothing can be created at runtime.
**How to avoid:** the root-tree skeleton ships empty dirs (k10root.proto is the precedent — empty dirs from `/tmp/empty` [VERIFIED: sys/src/9k/k10/root/k10root.proto:1-30]). MCREATE unions still need the *target* dir to exist.
**Warning signs:** `bind`/`mount` "does not exist" failures in boot prints / newns (silent without newnsdebug — init ignores newns's return).

### Pitfall 9: devdup for interactive rc
**What goes wrong:** rcmain's interactive path is `. -i '#d/0'` — rc *opens the device path* `#d/0` (fd 0 as a file) and sources it with the interactive flag. No devdup → no prompt, no error a human would recognize.
**How to avoid:** devdup exposing per-fd files by number; bound at `#d` (opencons binds `#c` at /dev, so `#d/0` resolves via the device name directly — no bind needed for `#d/0` literal use, but D-08's namespace binds `#d`→/fd).
**Warning signs:** rcmain runs (profile echo works) but no `% ` prompt; rc exits immediately.

### Pitfall 10: qemu process/log hygiene (deferred #2, now in scope)
**What goes wrong:** leaked qemu from a prior run + stale serial log short-circuits the next run's marker wait (documented: the rerun matched the old log and killed the fresh qemu).
**How to avoid:** `rm -f` logs before launch; kill by process group (setsid absent on macOS — backgrounded subshell + bracket-fixed `pkill -f '[q]emu-system-aarch64…'` [carried: STATE.md Phase 03]); expect-side timeout + explicit kill in an EXIT trap.
**Warning signs:** smoke passes instantly against a stale log; orphaned qemu processes accumulate on CI runners.

### Pitfall 11: expect timing under TCG
**What goes wrong:** GH ubuntu runners emulate aarch64 via TCG — boot+rc parse is minutes-scale, not seconds; default expect timeouts (10s) flake.
**How to avoid:** `set timeout` sized generously (legacy used 600-2400s [VERIFIED: build/build.bash:33,46]); assert ordered *stages* (banner → boot prints → `init: starting /bin/rc` → `% `) so failures localize; keep markers free of serial-cooking artifacts (CRLF — the kernel console maps `\n`→`\r\n` [VERIFIED: ../r9/port/src/devcons.rs:105-113]).

### Pitfall 12: init's `closefds` + fd preopen convention
**What goes wrong:** r9 preopens fd 0/1/2 as devcons chans at proc construction [VERIFIED: ../r9/aarch64/src/proc.rs:179-181 comment]; boot's `opencons` *closes* 0/1/2 then reopens via `/dev/cons` (post-bind). If devcons attach-by-name (`#c`) and `/dev/cons` resolution differ, fds end up pointing at different chans than later dups expect.
**How to avoid:** the namespace era makes `/dev/cons` resolve through the mount table to devcons — one device, one attach path. Verify fd 0/1/2 after `opencons` equal fresh `#c/cons` opens.

## Code Examples

### Syscall numbers (frozen wire table — the dispatch set the trace selects from)

Verbatim [VERIFIED: sys/src/libc/9syscall/sys.h:1-53] (subset relevant to this phase):

```c
#define	SYSR1		0
#define	_ERRSTR		1
#define	BIND		2
#define	CHDIR		3
#define	CLOSE		4
#define	DUP		5
#define	ALARM		6
#define	EXEC		7
#define	EXITS		8
#define	_FSESSION	9
#define	FAUTH		10
#define	_FSTAT		11
#define	SEGBRK		12
#define	_MOUNT		13
#define	OPEN		14
#define	_READ		15
#define	OSEEK		16
#define	SLEEP		17
#define	_STAT		18
#define	RFORK		19
#define	_WRITE		20
#define	PIPE		21
#define	CREATE		22
#define	FD2PATH		23
#define	BRK_		24
#define	REMOVE		25
#define	_WSTAT		26
#define	_FWSTAT		27
#define	NOTIFY		28
#define	NOTED		29
…
#define	UNMOUNT		35
#define	_WAIT		36
#define	SEMACQUIRE	37
#define	SEMRELEASE	38
#define	SEEK		39
#define	FVERSION	40
#define	ERRSTR		41
#define	STAT		42
#define	FSTAT		43
#define	WSTAT		44
#define	FWSTAT		45
#define	MOUNT		46
#define	AWAIT		47
#define	PREAD		50
#define	PWRITE		51
…
#define NSEC		53
#define NMOUNT		54
```

r9's dispatch table is a 55-entry `Option<Handler>` array with `None` → loud unimplemented [VERIFIED: ../r9/aarch64/src/syscall.rs:74-95]; Phase 4 implemented exactly {CLOSE 4, EXEC 7, EXITS 8, OPEN 14, RFORK 19, NOTIFY 28, NOTED 29, ERRSTR 41, AWAIT 47, PREAD 50, PWRITE 51}. This phase's trace adds the boot-chain set — expect BIND(2), CHDIR(3), DUP(5), ALARM(6), FAUTH(10), SLEEP(17), PIPE(21), CREATE(22), FD2PATH(23), REMOVE(25), SEEK(39), FVERSION(40), STAT(42), FSTAT(43), WSTAT(44), MOUNT(46) at minimum [ASSUMED — the donor trace settles it; loud-unimplemented makes any miss a loud, named failure].

### Mount flags and rfork flags (frozen values)

Verbatim [VERIFIED: sys/include/libc.h:558-564]:

```c
#define	MORDER	0x0003	/* mask for bits defining order of mounting */
#define	MREPL	0x0000	/* mount replaces object */
#define	MBEFORE	0x0001	/* mount goes before others in union directory */
#define	MAFTER	0x0002	/* mount goes after others in union directory */
#define	MCREATE	0x0004	/* permit creation in mounted directory */
#define	MCACHE	0x0010	/* cache some data */
#define	MMASK	0x0017	/* all bits on */
```

Verbatim [VERIFIED: sys/include/libc.h:613-623]:

```c
	RFNAMEG		= (1<<0),
	RFENVG		= (1<<1),
	RFFDG		= (1<<2),
	RFNOTEG		= (1<<3),
	RFPROC		= (1<<4),
	RFMEM		= (1<<5),
	RFNOWAIT	= (1<<6),
	RFCNAMEG	= (1<<10),
	RFCENVG		= (1<<11),
	RFCFDG		= (1<<12),
	RFREND		= (1<<13),
```

r9 already mirrors these constants [VERIFIED: ../r9/aarch64/src/syscall.rs:58-68] — Phase 5 gives RFNAMEG/RFCNAMEG/RFENVG/RFCENVG/RFNOTEG their real semantics.

### rcmain (the interactive entry's bootstrap — devdup is mandatory)

Verbatim [VERIFIED: rc/lib/rcmain:1-34] (the interactive arm):

```rc
if not if(flag i){
	if(flag l && /bin/test -r $home/lib/profile) . $home/lib/profile
	status=''
	if(! ~ $#* 0) . $*
	. -i '#d/0'
}
```

with the prompt default:

```rc
switch($#prompt){
case 0
	prompt=('% ' '	')
```

### Scoped namespace precedent (9k embedded root)

The k10root namespace shows the exact shape D-08's scoped file follows [VERIFIED: sys/src/9k/k10/root/k10root.namespace]:

```
# kernel devices
bind #c /dev
bind #d /fd
bind -c #e /env
bind #p /proc
bind -c #s /srv
…
# standard bin
bind /root/$cputype/bin /bin
bind -a /root/rc/bin /bin
```

Note the `/root/` prefix: at `newns` time `/` is still devroot and the served tree is mounted at `/root` — the scoped namespace file binds through `/root/…` (or runs after boot's `bind(/root, /, MAFTER)`; either way the paths must match the actual mount state).

### paqfs argv contract (the userspace server)

`paqfs -i` serves 9P over fd 0/1 (stdio mode: `pfd[0] = 0; pfd[1] = 1;`); the image path is the sole positional arg [VERIFIED: sys/src/cmd/paqfs/paqfs.c:162-213]. It implements the full message set: Tflush/Tversion/Tattach/Tauth/Twalk/Topen/Tcreate/Tread/Twrite/Tclunk/Tremove/Tstat/Twstat [VERIFIED: paqfs.c:111-124] — devmnt's client side must cover at least version/attach/walk/open/read/stat/clunk for the boot path; fauth→Tauth gets paqfs's `Enoauth` error, which nsinit's fall-through already tolerates.

### qemu launch flags (Harvey's launcher documents these from r9's TOML per D-11)

Verbatim from the virt exec profile [VERIFIED: ../r9/aarch64/lib/config_virtexec.toml:21-43]:

```toml
[qemu]
machine = "virt,gic-version=2"
cpu = "cortex-a72"
smp = 1
memory = "1024M"
accel = "tcg"
dtb = "aarch64/lib/virt-gicv2.dtb"
initrd = { file = "target/bootfs.paq", addr = 0x48000000 }
```

plus the loader-device emission and the fdtput /chosen patch (two 32-bit BE cells, high word first — dtc has no 8-byte type) [VERIFIED: ../r9/xtask/src/config.rs:368-371,395-434]. Harvey's compose app assembles the same argv itself: `-M virt,gic-version=2 -cpu cortex-a72 -smp 1 -m 1024M -accel tcg -kernel <r9-elf> -dtb <patched-copy> -device loader,file=<paq>,addr=0x48000000 -nographic`.

### Flake composition pattern (where the new pieces slot in)

Harvey's apps use `mkScriptApp` (writeShellApplication with PLAN9/PATH header + script file + runtimeInputs) [VERIFIED: flake.nix:12-25]; existing apps: setup/toolchain/build/iso [VERIFIED: flake.nix:37-72]. The new pieces:
- `inputs.r9.url = "github:requaos/r9"` (+ flake.lock pin; dev flow `--override-input r9 …` per D-13) [ASSUMED — exact URL form is planner's choice; the repo remote is `git@github.com:requaos/r9.git`, VERIFIED this session]
- `packages` in r9's flake: currently **none exist** — `packages.kernel-virt` (built ELF + committed DTB) is new output per D-11 [VERIFIED: ../r9/flake.nix:44-93 shows only devShells + apps]
- the compose app (`boot-r9`) and smoke app land in Harvey's `apps` next to `build`/`iso`

## State of the Art

This is a brownfield OS project, not an ecosystem-library domain — the "state of the art" is the project's own carried-forward decisions:

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| r9 xtask bakes initrd from `../harvey` paths (R9_HARVEY_DIR) | Harvey's flake bakes the initrd; r9 exposes only `packages.kernel-virt` | this phase (D-10/D-11) | the r9→Harvey env glue inverts to Harvey→r9; xtask loses initrd/harvey-path knowledge |
| staged `init.c` (build/aarch64/init.c) as pid 1 | real `boot(8)` → real `init` → real rc | this phase (D-01) | staged init/smokefork/smokenote remain as r9-side regression gates (CONTEXT specifics: they must not rot) |
| sys_open hardcoded prefix routing | real name resolution + mount table + union walk | this phase (D-03) | one-way kernel architecture decision |
| qemu `-initrd` | generic `-device loader` + fdtput-patched DTB | Phase 4 (04-01, D-01 amended) | unchanged here; Harvey's launcher adopts it |

**Deprecated/outdated:**
- Phase 4's `cargo xtask initrd` subcommand and `ensure_initrd_image` [VERIFIED: ../r9/xtask/src/main.rs:1613-1735] — superseded Harvey-side this phase (D-10); removal/inversion is part of the phase, but r9's boot-test profiles (virtexec/virtmulti/virtnotes) still consume `target/bootfs.paq` — the r9-side regression gate supply chain must keep working (either r9 keeps a minimal bake for its own gates, or the staged binaries' bake moves with the inversion; planner resolves with D-12's "r9-side gates stay r9-side" constraint).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The trace-settled syscall set includes BIND/CHDIR/DUP/ALARM/SLEEP/PIPE/CREATE/FD2PATH/REMOVE/SEEK/FVERSION/STAT/FSTAT/WSTAT/MOUNT (plus possibly UNMOUNT/FWSTAT/OSEEK) | Code Examples | Low risk — measure-first discipline makes the trace authoritative; loud-unimplemented surfaces any miss by name. Listed set is informed by reading boot.c/init.c/newns.c call sites, not traced. |
| A2 | `inputs.r9.url = "github:requaos/r9"` is the right flake-input URL form | Code Examples | Trivial — flake input URL syntax variants (git+ssh vs github:) are a planner choice; the lock pin is what INT-04 mandates. |
| A3 | GH Actions ubuntu-latest has `unshare -m` usable for the namespace build (util-linux present) | CI leg | Low — Phase 1's build app uses the same mechanism on Linux; runner sudo+unshare availability is standard but unverified this session. If blocked, CI falls back to a lima-style VM on the runner or the build runs via the repo's existing linux path. |
| A4 | A nix-install action (DeterminateSystems or cachix) will be used in the new workflow; harvey.yml predates nix CI | CI leg | Low — workflow scaffolding detail; both actions are standard. Not verified this session. |
| A5 | `readparts()` (partinit) fails soft when no `#S` disks exist, or is neutralized by seeded `readparts=0` | Pattern 5 step 6 | Medium-low — parts.c not fully read this session; boot.c gates it on `getenv("readparts")` being nil-or-nonzero [VERIFIED: boot.c:84-93], so the seeded env controls it. Executor verifies behavior on first boot. |
| A6 | qemu `-nographic` stdio serial works with expect's spawn on both host OSes (legacy build.bash proves it for i386; aarch64 virt PL011 over stdio is the same mechanism) | Pattern 6 | Low — Phase 3/4 r9 boot-test uses a unix socket instead; stdio is the expect-natural path but the planner may choose the socket shape if stdio interleaves with qemu monitor. |

## Open Questions (RESOLVED)

> **Resolution (planning revision 1, 2026-11-02):** all four questions are settled in the phase plans.
>
> 1. **What execs as pid 1 / argv** → 05-04 Task 1 (transitional argv hint + `cfg(dev_bootchain)` entry selection; kernel execs `/boot/boot` via the two-component devroot walk) and 05-05 (the locked answer: kernel seeds the root devenv group — `cputype=aarch64`, `nobootprompt=embed` — before the first exec; argv reduced to `[b"boot"]`, the transitional hint removed).
> 2. **r9 boot-test initrd after the D-10 inversion** → 05-10 Task 2, which cites this question by name and takes the lowest-risk recommendation verbatim: r9 keeps its own bake against the unchanged staged-init supply chain (`build/aarch64/stage-build.sh`), with the Harvey dir explicit via required `R9_HARVEY_DIR`.
> 3. **devtab dispatch shape** → 05-04, which cites this question by name in its must_haves: a hand-built `#`-char → attach-fn registry (no generated devtab, no trait conformance this phase), retiring the sys_open prefix routing.
> 4. **Scoped termrc/namespace source-of-truth location** → 05-02 Task 2: the `build/aarch64/root-tree/` committed skeleton, staged by `mkimage.sh` at bake time; `rc/lib/rcmain` ships verbatim (copied from the stock source at bake time, never edited).

1. **What exactly execs as pid 1 and with what argv?** (the agent's discretion)
   - What we know: Phase 4's `exec_bootstrap` walks devroot for `"init"` and execs with argv `[b"init"]` [VERIFIED: ../r9/aarch64/src/main.rs:420-436]; Phase 5 execs `/boot/boot` (a subdirectory walk — devroot walks single components fine). Method selection via seeded env (Pattern 3 recommendation) or argv.
   - What's unclear: whether the kernel seeds devenv before the first exec (new mechanism) or carries a bootargs-style string from the DTB/TOML.
   - Recommendation: seed devenv `{cputype=aarch64, nobootprompt=<method>, service=terminal}` at kernel init; keep argv minimal.

2. **How does r9's own boot-test suite keep its initrd after the Harvey-side inversion (D-10)?**
   - What we know: virtexec/virtmulti/virtnotes/virtpanic consume `target/bootfs.paq` produced from Harvey `build/aarch64/out/` staged binaries via `cargo xtask initrd` [VERIFIED: ../r9/xtask/src/main.rs:1626-1735]; D-12 says r9-side gates stay r9-side; CONTEXT says the staged binaries "remain as r9-side regression gates… must not rot".
   - What's unclear: whether r9 keeps a slimmed bake (its own mkpaqfs call against Harvey-staged binaries) or the profiles migrate to the Harvey-baked image.
   - Recommendation: planner decides; keeping r9's bake working against the *existing* staged-init supply chain (build/aarch64/stage-build.sh unchanged) is the lowest-risk answer, with the new real-chain image bake a separate Harvey-side script.

3. **devtab dispatch shape for the namespace era** (Pattern 4)
   - What we know: dat.rs's `Dev` trait is non-constructible; devices are free functions with a DEV_ROOT/DEV_CONS/DEV_PROC index convention; sys_open routes by prefix.
   - What's unclear: whether the phase grows a generated devtab (9k shape) or a hand-built registry mapping `#c`/`#e`/`#p`/`#d`/`#s`/`#|`/mnt-attachment to device ops.
   - Recommendation: hand-built registry first (small fixed device set), trait conformance deferred — matches the established r9 idiom and keeps the diff reviewable.

4. **Where does the scoped termrc/namespace live in-repo?**
   - What we know: the image ships `/rc/bin/termrc` (scoped), `/rc/lib/rcmain` (stock), `/lib/namespace` (scoped) — mandated files (D-05/D-08 + the agent's-discretion skeleton).
   - What's unclear: the source-of-truth location (e.g. `build/aarch64/root-tree/…` vs `cfg/r9/…`) and whether the stock `rc/lib/rcmain` ships verbatim (it reads `$home/lib/profile` — the image ships `/usr/glenda/lib/profile`; rcmain itself is shared but not modified — amd64 safety holds since only the *image contents* differ).
   - Recommendation: a dedicated `build/aarch64/root-tree/` skeleton dir staged by the bake script; rcmain copied verbatim from `rc/lib/rcmain`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| nix | everything (flakes) | ✓ | /nix/var/nix/profiles/default/bin/nix | — |
| qemu-system-aarch64 | boot + smoke | ✓ via nix devshells (bare host: MISSING — expected) | nixpkgs pin (carried-forward facts reference 11.1.2) | — |
| expect | INT-02 smoke | ✓ bare host `/usr/bin/expect` AND Harvey devShell [VERIFIED: flake.nix:82] | — | — |
| limactl | darwin userland build | ✓ via Harvey devShell (bare host: MISSING — expected); VM `harvey` existence not probed this session | — | linux host builds natively |
| dtc (fdtput) | DTB patch | ✓ via r9 devShell [VERIFIED: ../r9/flake.nix:52] | — | — |
| cargo/rustc | r9 kernel build | ✓ via r9 devShell (rust-overlay pin from rust-toolchain.toml) | — | — |
| unshare (util-linux) | linux namespace build | CI: ubuntu-latest [ASSUMED A3]; locally via build app runtimeInputs [VERIFIED: flake.nix:62] | — | lima VM |
| 9front donor harness | measure-first trace | ✓ in-repo rig [VERIFIED: build/aarch64/trace-syscalls.py] | — | fallback: patched dosyscall print (documented in the rig header) |

**Missing dependencies with no fallback:** none — every tool is devshell-supplied on both hosts.

**Missing dependencies with fallback:** lima on darwin (linux builds natively); the primary trace method (documented fallback in the rig).

## Security Domain

This phase's security surface is the syscall trust boundary and the new namespace/9P-client machinery — not web-style authn/authz. `security_enforcement` is not disabled in config, so the applicable mapping:

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | stubbed by design (D-02) | auth never fatals; hostowner=glenda; real auth is a later milestone (documented deferral) |
| V3 Session Management | no | — |
| V4 Access Control | yes (file perm bits) | paqfs perm checks (Pexec/Pwrite/Pread bits) [VERIFIED: sys/src/cmd/paqfs/paqfs.c:50-58]; devproc write validates target pid [VERIFIED: ../r9/port/src/devproc.rs:126-143] |
| V5 Input Validation | **yes — the core one** | T-04-11 per-access user-VA validation for every new syscall arg [VERIFIED: ../r9/aarch64/src/syscall.rs:17-25]; cold-pointer aborts are known-deferred (#3) — new code must not widen the window |
| V6 Cryptography | no | no hand-rolled crypto this phase |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Kernel dereference of unchecked user pointer | Tampering/EoP | `Proc::validate_user_range` before any user-VA read/write (existing discipline — extends to BIND/MOUNT/PIPE/DUP args); nested demand-paging from syscall context stays out of scope (deferred #3) |
| Namespace escape via MCREATE/union misuse | EoP | port 9k chan.c/pgrp.c semantics literally; mount-table entries refcounted; RFNAMEG/RFCNAMEG group semantics from the donor |
| fd confusion across fork/dup (pipe endpoints) | Tampering | shared-Arc pipe queue; fd-table copy semantics per rfork flags (RFFDG copy / RFCFDG fresh) |
| Note spoofing via /proc writes | Repudiation/Tampering | devproc write resolves the addressed pid only (existing T-04-17: no broadcast) — DEL-note posting goes through the same hook with note-group scoping |
| 9P message malformation from userspace server (paqfs over pipe) | Tampering | devmnt bounds-checks message sizes against the negotiated msize (9k devmnt.c mntchk reference) |

## Sources

### Primary (HIGH confidence) — all read this session

- Harvey boot chain: `sys/src/9k/boot/boot.c` (full flow, Method selection, nsinit, execinit), `embed.c` (connectembed verbatim), `paq.c`, `local.c` (fatal binds), `bootauth.c` (glenda no-op), `aux.c` (readfile/srvcreate), `settime.c`, `usb.c` (usbinit no-op guard), `boot.h` (Method struct), `doauthenticate.c` (not linked — BOOTFILES evidence)
- Build machinery: `sys/src/9k/mk/bootmkfile` (boot$CONF.out rule), `sys/src/9k/mk/parse` (mkbootconf generation), `sys/src/9k/k10/k10cpu` (config shape), `sys/src/9k/k10/mkfile`
- Userland: `sys/src/cmd/init.c` (full), `rc/lib/rcmain` (full), `sys/src/cmd/rc/plan9.c:27` + `exec.c:120-230` (loadboot), `sys/src/cmd/paqfs/paqfs.c` (argv + message table + stdio mode), `mkpaqfs.c` (usage), `sys/src/cmd/ps.c` (status/args reads), `cfg/example/termrc` (near-empty), `lib/namespace` (full), `sys/src/libauth/newns.c` (full — rfork flags, ops, env expansion), `sys/src/libc/9syscall/sys.h` (numbers), `sys/include/libc.h` (M*/RF* flags)
- r9: `port/src/dat.rs` (Chan/Dev/Qid + DEV_* convention + dup semantics), `devcons.rs` (write-only + CRLF cooking), `devproc.rs` (note-only scope), `devroot.rs` (paqfs-backed walk/open/read/stat + raw image accessor), `aarch64/src/syscall.rs` (dispatch table + prefix routing), `proc.rs` (Proc fields, MAX_PROCS=8, MAX_FDS=16), `exec.rs` (USTACKTOP/USTKSIZE/USTOP), `main.rs:391-458` (exec_bootstrap), `xtask/src/main.rs` + `config.rs` (initrd bake, fdtput recipe, boot-test profiles/markers), `flake.nix` (no packages.*), `aarch64/lib/config_virt{,exec}.toml`, `HowItWorks.md` header mandate
- Glue: `flake.nix` (Harvey), `build/aarch64/{subset-build.sh,stage-build.sh,mkpaqfs-host.sh,nsrun.sh,vm-run.sh,trace-syscalls.py}`, `.github/workflows/harvey.yml`, `build/build.bash` (expect pattern), `build/mm/boot-9k10cpu.mm`, `build/REPRODUCIBILITY.md`
- Planning: `.planning/phases/05-*/05-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/ROADMAP.md` §Phase 5, `.planning/phases/04-*/deferred-items.md`, `.planning/config.json`
- Git state: r9 remote `git@github.com:requaos/r9.git`, HEAD `54e8c1079933` (2026-11-02); Harvey HEAD `5882af63039a`
- Graph context: graphify query "namespace bind mount devmnt" over graphify-out/graph.json — devmnt function inventory confirmed (mntversion/mntattach/mntwalk/mntopen/mntread/mntstat/mntclunk/mntrpc/mountmux in sys/src/9/port/devmnt.c)

### Secondary (MEDIUM confidence)

- None — no web sources were needed; both repos are the authoritative sources for their own mechanics.

### Tertiary (LOW confidence)

- Assumptions A1–A6 above (trace-settled syscall set, flake URL form, CI runner capabilities) — all confirmation-gated by the plan's own measure-first and clean-checkout tasks.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; all components verified in-repo
- Architecture: HIGH — every flow traced to source with line numbers; the one genuinely new design (r9 name resolution shape) is flagged as planner's-choice with a recommendation
- Pitfalls: HIGH — each pitfall cites the code that causes it; A5 (readparts) is the only unverified failure-mode assumption and is explicitly marked

**Research date:** 2026-11-02
**Valid until:** 2026-12-02 (stable — all sources are the two pinned repos; re-verify only if r9 main moves beyond the phase-start re-pin)


