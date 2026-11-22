# Phase 5: Shell Boot + One-Command Integration - Context

**Gathered:** 2026-11-02
**Status:** Ready for planning

<domain>
## Phase Boundary

The Core Value delivered: **one command** (`nix run` app in Harvey's flake), from clean checkouts of both repos, on darwin-aarch64 or linux, boots **r9 + the real Harvey aarch64 boot chain** — `boot(8)` → `init` → `rc -i` — to an interactive `/bin/rc` shell (`% ` prompt) over the PL011 console in qemu. Includes the flake-input composition (INT-04), an expect-style smoke test driving a scripted command sweep (INT-02 + SC#2), GH Actions CI on the linux leg, and clean-checkout reproducibility documentation (INT-03).

This phase makes r9 a real Plan 9 kernel environment: it grows **bind + mount + devmnt (kernel 9P client) + per-process namespaces (pgrp/union walk)**, full devcons (cooked + consctl/raw + DEL note), devenv with real per-pgrp semantics, devdup, devsrv, devpipe, and devproc beyond the note slice — the device/namespace surface the real boot chain and an interactive rc require.

**Work spans both repos** plus the flake glue. Kernel side in `../r9` (fork `requaos/r9`, branch `main`); userland/boot-chain binaries + image assembly + composition + smoke + CI in this repo; planning artifacts and acceptance evidence live here.

NOT in this phase: virtio-9p host mount (v1.x — but devmnt landing here is its prerequisite), SMP/PSCI, virtio-net/blk, RTC, PCIe ECAM, real auth-server/factotum handshake (auth paths are stubbed), boot(8)'s disk/network methods (local fossil/kfs, tcp — the embedded method only), rio/acme/devdraw, amd64-on-r9, hardware bring-up, the 9k-amd64 pool-panic re-test, phase-04 deferreds #3/#4 and the remaining Info findings.

</domain>

<decisions>
## Implementation Decisions

### Boot chain shape (user decisions)
- **D-01:** **Port the real Harvey boot chain** — `boot(8)` runs as pid 1, execs the real `/bin/init`, which fork/exec's `/bin/rc` in its `for(;;)` restart loop. No minimal staged-init shortcut: the staged init's job ends here. — **Reversibility:** costly — the phase's kernel surface (namespaces, devices, devmnt) is sized by this choice; reverting to a staged-exec-rc path would strand most of the machinery built.
- **D-02:** **Full `boot(8)` with auth stubs** — boot.c's real flow runs: `opencons`/`bindenvsrv`, method pick, `doauth` (stubbed), `rfork(RFNAMEG)`, `connectroot` (`fversion`), `nsinit` (bind/mount), `execinit`. Auth never fatals in the embedded path: `doauthenticate` already no-ops when the root server requires no auth (`trbuf[0]==0`) and `nsinit` falls through to unauthenticated mount when `fauth` fails — stubs hold that line. — **Reversibility:** reversible — stub granularity (compile-time file swap vs runtime degrade) is planner's call; the contract is "auth paths never fatal, hostowner gets a sane default".
- **D-03:** **r9 grows full namespace machinery: bind + mount + devmnt.** Per-process namespaces (pgrp equivalent, mount table, union walk in name resolution, MREPL/MAFTER/MBEFORE/MCREATE) plus the kernel 9P client so `mount(fd…)` works — boot.c runs unmodified. `RFNAMEG`/`RFFDG` rfork flags come with it. devmnt also becomes the virtio-9p prerequisite for v1.x. — **Reversibility:** one-way — this defines r9's kernel architecture permanently (namespace model + 9P client); every later device/server assumption builds on it.
- **D-04:** **Embedded root follows the `connectpaq` precedent** (`sys/src/9k/boot/paq.c`): boot's method pipes + forks + execs a **userspace paqfs 9P server** (Harvey's own `sys/src/cmd/paqfs/`, built for aarch64) over the initrd image, returns the pipe fd; `connectroot`/`nsinit` then `fversion` + `mount` it at `/root` via devmnt and bind onto `/`. Zero boot.c divergence in the shared flow — the embedded method is a small additive `Method` entry. — **Reversibility:** costly — the image layout, the devroot raw-image exposure, and the boot method all key off this shape.
- **D-05:** **Interactive entry contract: `init -t` + scoped termrc → `rc -i`.** The paqfs image ships a scoped `/rc/bin/termrc` that skips the example's network/window machinery (no ipconfig/ndb/rio) and ends in `exec rc -i` → rc bootstraps `/rc/lib/rcmain` (compiled-in, `sys/src/cmd/rc/exec.c:216`) → `prompt=('% ' ' ')` → interactive `. -i '#d/0'` (devdup mandatory). The smoke asserts the literal `% ` prompt.

### Shell environment (user decisions)
- **D-06:** **Full devcons** — cooked line discipline (line buffering, erase, kill-line), DEL posts the "interrupt" note to the process group, **plus `/dev/consctl` raw-mode switching**. The real devcons, not a minimal subset. The userland read path from EL0 is new this phase (Phase 4 binaries only wrote).
- **D-07:** **devenv with real per-pgrp env semantics** — `#e` keyed by process group; `rfork(RFENVG)` shares, `RFCENVG` copies; init's `setenv("#e/objtype"…)` and rc's var export to children behave exactly like Plan 9.
- **D-08:** **Ship a scoped `/lib/namespace`** (binds for `#e`→/env, `#p`→/proc, `#d`/dev entries, `/bin` union entries as appropriate) so init's `newns(user, 0)` really executes — the per-login namespace rebuild is exercised end-to-end, proving /lib/namespace parsing and bind semantics against a real file.
- **D-09:** **The smoke drives a scripted command sweep**, not just the prompt: assert `% `, then type and assert markers — `echo` exact string, `ls /bin` (subset binaries present), `ps` (rcmain's fn reads `/proc/*/status` — devproc must expose status/args), `cat` of a known file. SC#2 becomes asserted evidence on every run, not a human-eyeballs item.

### Flake composition + CI (user decisions)
- **D-10:** **Harvey's flake owns composition** — a new app (e.g. `nix run .#boot-r9`) builds the aarch64 userland (lima namespace build on darwin, native on linux), pulls the r9 kernel from the flake input, bakes the initrd, launches qemu to the interactive prompt. The Phase 4 r9→Harvey env glue (`R9_HARVEY_DIR`, workspace-relative initrd path, ureg-check `../harvey` default) **inverts** to Harvey→r9. — **Reversibility:** one-way — this publishes the INT-04 integration shape; r9's xtask stops knowing Harvey paths.
- **D-11:** **r9's flake exposes a kernel package only** (`packages.kernel-virt`: built aarch64 ELF + committed DTB — the Phase-3-deferred packages.* shape, now defined by this consumer). Harvey's app bakes the initrd with its own `mkpaqfs` and launches qemu itself (the `-device loader` + fdtput-patched-DTB flags are documented from r9's config TOML). Harvey never runs r9's xtask.
- **D-12:** **The shell smoke is an expect script + flake app in Harvey** (INT-02's literal "expect-style"; expect is already in Harvey's devshell; Phase 1's expect+qemu lineage). It drives the composed boot per D-09. r9's `cargo xtask boot-test` keeps its kernel-focused profiles (virt/virtexec/virtmulti/virtnotes/virtpanic) unchanged — r9-side gates stay r9-side.
- **D-13:** **CI = GH Actions linux leg, full pipeline only.** Clean checkout → toolchain → aarch64 subset → initrd bake → boot smoke, on ubuntu-latest (native `unshare` namespace build, qemu TCG). **No darwin leg in CI** — darwin is verified by the documented manual command (Phase 3 D-10 pattern). This amends ROADMAP SC#3's literal "CI green on both": CI-green means linux; darwin is documented-and-run-by-hand. r9 pinned via `flake.lock`; dev flow uses `--override-input` for local r9 checkouts.

### Known-flake policy (user decisions)
- **D-14:** **The note-timeout race gets a debug-first Wave 0** (`/gsd-debug` discipline) before the shell path depends on notes. Pre-existing (~1/3 virtmulti runs, reproduces on pre-fix base b4f858a per 04-REVIEW-FIX.md); the shell leans on notes harder (DEL interrupts at every prompt), and a flaky smoke would erode the milestone's headline claim.
- **D-15:** **Explicit stack-growth smoke binary** — a small 7c deep-recursion binary ships in the image, forces multiple stack page-ins, prints a marker. Stack growth graduates from proven-by-shared-path to execution-evidenced before rc's parser depends on it.
- **D-16:** **Phase-04 deferred #2 (harness qemu cleanup) enters scope** — CI spawns qemu per run; leaked processes/sockets become CI reliability issues. Deferreds #3 (cold-pointer syscall aborts) and #4 (contained heisenbug) + the 8 open Info findings ride the milestone backlog.

### Carried forward (locked in roadmap/prior phases — do not re-litigate)
- qemu shape: `-M virt,gic-version=2 -cpu cortex-a72 -smp 1 -nographic`, ELF `-kernel`, committed virt-gicv2 DTB at `0x40000000`; TCG on darwin (HVF rejects GICv2 on qemu 11.1.2); `-m ≤ 1G` (pagealloc bitmap ceiling).
- initrd delivery = qemu generic `-device loader` + fdtput-patched DTB (Phase 4 D-01 amended); `mkpaqfs` produces, r9's no_std paqfs reader consumes (kernel-side exec source), adler32 seed-0 quirk documented.
- **Measure-first syscall discipline (Phase 4 D-14) extends to the real chain**: trace the real boot+init+rc+subset path on the 9front donor kernel (Phase 2 harness), implement exactly the measured set — expect pipe/dup/wstat/RFNAMEG/RFFDG-class entries to appear; loud-unimplemented default stays.
- Proof standard: serial/expect evidence + marker assertions + human checkpoint at the gate; canonical evidence consolidated in Harvey `.planning/phases/05-*/` with the r9 commit SHA recorded (Phase 3 D-13).
- r9 commits land on `main` of `requaos/r9`; HowItWorks.md updated with material changes (its header mandates it); r9 follows its rustfmt.toml, Harvey follows Plan 9 C style — style rules don't cross the repo boundary.
- `aarch64` naming everywhere.
- The 16-binary shell subset (rc, grep, ipconfig, factotum, cat/cp/date/echo/ls/mkdir/mv/ps/rm/sed/test/wc) is built and **executed on the donor kernel** (Phase 2 TC-05) — the image's binary pool; `boot`, `init`, `paqfs` join the aarch64 build set this phase.
- **amd64-boot safety constraint**: `boot.c`/`init.c`/`rcmain` are shared with the Phase-1-green 9k amd64 build — all changes additive and objtype-gated; amd64 boot behavior must not change (embedded-method entry, no edits to existing flows).
- KERNDATE/reproducibility policy (Phase 1, `build/REPRODUCIBILITY.md`) applies to INT-03: documented commands + pinned toolchains (r9 via flake.lock pin; qemu/plan9port via nixpkgs pin) — process-level reproducibility, per the requirement wording.
- virtio-9p stays v1.x; devmnt (D-03) is its prerequisite, not its implementation.
- Apple host 16K pages / guest granule + `CTR_EL0` architectural discovery (Phase 2 D-03) apply to any new cache/page maintenance r9 grows.

### the agent's Discretion
- Embedded boot-method naming, argv contract (how the kernel's exec of `/boot/boot` selects it), and method-file placement — additive, modeled on `paq.c`.
- Auth stub granularity (objtype-gated file swap vs runtime degrade-through) — the contract is D-02's "never fatal in the embedded path".
- paqfs image root-tree skeleton beyond the mandated files (`/bin/*`, `/boot/{boot,paqfs}`, `/rc/lib/rcmain`, `/rc/bin/termrc`, `/lib/namespace`, `/adm/timezone/local`, `/usr/…/lib/profile`, `/tmp`).
- devproc file surface beyond the D-09-forced status/args (+ existing note, + ctl for init's `pri 10` write which degrades gracefully) — trace-driven.
- Expect-script mechanics, marker strings, flake app naming, and wave ordering within the phase (subject to D-14's debug-first ordering).
- How qemu cleanup (#2) is implemented (process-group kill, socket cleanup, timeout policy).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 5 — goal, success criteria 1–5 (note D-13's amendment of SC#3)
- `.planning/REQUIREMENTS.md` — INT-01..INT-04 full text; v2 deferral list (virtio-9p et al.)
- `.planning/PROJECT.md` — Key Decisions (r9 adoption, flake composition decision "Harvey consumes r9 via flake input", proof standards)
- `.planning/STATE.md` — Accumulated Context (Phase 4 decisions), Blockers/Concerns (note-race, stack-growth gap, deferred inventory)

### The real boot chain being ported (Harvey repo — read before any boot-path planning)
- `sys/src/9k/boot/boot.c` — the pid-1 flow: opencons/bindenvsrv → pickmethod → doauth → rfork(RFNAMEG) → connectroot → nsinit → execinit
- `sys/src/9k/boot/paq.c` — **the connectpaq precedent** D-04 models: pipe + fork + exec userspace paqfs server, return fd
- `sys/src/9k/boot/local.c` — `connectlocal`'s bind sequence (`#c`→/dev, `#p`→/proc, …) — the bind surface the embedded method needs
- `sys/src/9k/boot/doauthenticate.c` — auth degradation behavior (fsession no-auth no-op, fromauth fall-through)
- `sys/src/9k/boot/bootauth.c` + `nopsession.c` — the `authentication()` paths being stubbed
- `sys/src/cmd/init.c` — real init: degradation-tolerant env/newns handling, `fexec` fork/wait loop, `-t`/`-m`/`-c` service routing, the `for(;;)` rc restart loop
- `sys/src/cmd/rc/plan9.c` (`Rcmain = "/rc/lib/rcmain"`), `sys/src/cmd/rc/exec.c:130-218` (loadboot — rc's compiled-in rcmain bootstrap; `-m` override)
- `rc/lib/rcmain` — the shipped rcmain: `% ` prompt default, path default, `. -i '#d/0'` interactive path (devdup mandatory)
- `sys/src/9k/root/rcmain` + `sys/src/9k/k10/root/k10root.rc`/`k10root.proto` — the 9k embedded-root precedent (rcmain at /boot, shebang-exec'd rc)
- `cfg/example/termrc` — the heavy terminal script D-05's scoped termrc strips down
- `sys/src/cmd/paqfs/` — the userspace paqfs 9P server (D-04's mount target) + `mkpaqfs` host tool
- `lib/namespace` — the namespace file shape D-08 scopes

### The frozen ABI + userland supply chain (Harvey repo)
- `aarch64/ABI.md` — frozen wire contract + Phase 4 kernel-side appendix (TrapFrame≡Ureg); consumed by every new syscall this phase
- `aarch64/include/ureg.h` + `sys/src/libc/9syscall/sys.h` + `sys/src/libc/9syscall/mkfile` — frame layout + syscall table + stub generator
- `build/aarch64/README.md` — donor-kernel harness (the D-14-style trace rig for the real chain), qemu-11.1.2 drift accommodations
- `build/aarch64/init.c`, `stage-build.sh`, `subset-build.sh`, `mkpaqfs-host.sh` — the existing staged-init supply chain being replaced/grown
- `build/REPRODUCIBILITY.md` — the KERNDATE policy INT-03 honors

### Prior-phase decisions and evidence (Harvey repo)
- `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-CONTEXT.md` — D-01..D-16 (initrd delivery, paqfs reader, TrapFrame, exec/VM, scheduler, notes); its Deferred Ideas list names this phase's contents
- `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-EVIDENCE.md` — the phase's canonical evidence bundle + r9 SHA pin (54e8c10-era; planner re-pins at phase start)
- `.planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-REVIEW-FIX.md` + `deferred-items.md` — the note-timeout race evidence (D-14's Wave-0 target) and deferred #2/#3/#4 texts
- `.planning/phases/03-r9-qemu-virt-platform-bring-up/03-CONTEXT.md` — D-08 (packages.* deferred to this phase's consumer), D-10 (manual darwin verification pattern D-13 reuses), D-11/D-13 (boot-test harness + evidence consolidation)
- `.planning/phases/02-aarch64-toolchain-libc-9front-donor/02-CONTEXT.md` — D-06..D-09 (execution-gate standard, donor harness), D-14/D-15/16/17 (ABI freeze), TC-05 subset provenance

### r9 repo (sibling, `../r9` — where kernel work happens)
- `../r9/port/src/dat.rs` — Proc/Chan/Dev scaffolding the namespace + device work builds on
- `../r9/port/src/{devcons.rs,devproc.rs}` — console lock + devproc-lite (note-only) being extended (D-06, D-09)
- `../r9/port/src/fdt.rs` — DTB parser (unchanged this phase)
- `../r9/aarch64/src/{trap.rs,trap.S}` — EL0/SVC dispatch + note-delivery boundary (D-14's Wave-0 debug target)
- `../r9/aarch64/src/{vm.rs,pagealloc.rs}` — user VM + demand-fault handler (D-15's stack-growth path lives here)
- `../r9/aarch64/src/platform/virt/{gic.rs,timer.rs,pl011.rs,devcons.rs}` — IRQ/console plumbing devcons-read builds on
- `../r9/aarch64/lib/config_virt*.toml` — TOML-as-data profiles; the loader+patched-DTB qemu flags D-11 documents for Harvey's launcher
- `../r9/xtask/src/{main.rs,config.rs,uregcheck.rs}` — initrd baking being replaced Harvey-side (D-10/D-11); ureg-check's `../harvey` default inverts
- `../r9/flake.nix` — devshell + apps; grows `packages.kernel-virt` per D-11
- `../r9/HowItWorks.md` — update with material changes (namespace/devmnt/devices are material)

### Flakes / CI (both repos)
- `flake.nix` (Harvey) — mkScriptApp pattern, existing setup/toolchain/build/iso apps; the D-10 composition app + D-12 smoke app land here; gains the r9 input
- `.github/workflows/harvey.yml` — the legacy CI (untouched); the new INT-02 workflow lands alongside
- `../r9/.github/workflows/xtask.yml` — r9's CI (untouched)

### qemu virt machine facts (carryover — re-verify only if qemu version changes)
- qemu 11.1.2: ELF `-kernel` gets no `-initrd` delivery (loader+patched-DTB stands); HVF rejects GICv2 (TCG on darwin); GIC wake poke / ECAM / DTB loader notes in `build/aarch64/README.md`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `sys/src/cmd/paqfs/` — the userspace paqfs 9P server: D-04's mount target, already the initrd producer's sibling; portable C, needs an aarch64 build + a raw-image file to serve
- `sys/src/9k/boot/paq.c` — the exact connect-method shape to model (pipe/fork/exec server → return fd)
- `sys/src/cmd/init.c` — degradation-tolerant by construction (readenv → "*unknown*", setenv prints-and-continues, `newns` return ignored, `for(;;)` rc restart) — runs on partial machinery with loud warnings
- `build/aarch64/{subset-build.sh,stage-build.sh}` — the lima namespace-build + rsync-back flows the image build extends
- r9's `boot-test` harness (zero-dep unix-socket, marker assertions) — stays r9-side; the Harvey expect script follows Phase 1's `build/mm/` capture/assert pattern instead
- r9's paqfs no_std reader + devroot walk/open/read/stat — the kernel's exec source; devroot additionally exposes the raw initrd image for the userspace paqfs server (D-04)
- Harvey's devshell already ships expect, qemu, lima — the D-12/D-13 tooling needs no new host deps
- Phase 2 donor harness (`build/aarch64/`) — the trace rig for the measure-first syscall set of the real chain

### Established Patterns
- Execution-gated ordering (riscv64 vaporware rule): each kernel capability lands behind a running binary that exercises it — this phase's gate is the boot-to-prompt path itself
- Tracer/falsification-first (Phase 4 04-01 pattern): D-14's Wave-0 debug and the donor trace precede machinery
- TOML-as-data for qemu/config variation (r9); mkScriptApp writeShellApplication for Harvey flake apps
- Additive, objtype-gated edits to shared sources (boot.c/init.c/rcmain) — amd64 boot behavior unchanged (Phase 1 green is sacred)
- Evidence standard: captured log + asserted markers + human checkpoint; canonical evidence in Harvey `.planning/phases/05-*/` with r9 SHA pinned
- One `config_*.toml` per boot profile; markers not byte-order under preemption

### Integration Points
- `flake.nix` (Harvey): new `inputs.r9` (git URL, flake.lock-pinned), new composition app (D-10) + smoke app (D-12); `nix/` scripts for image bake + qemu launch
- `../r9/flake.nix`: new `packages.kernel-virt` output (D-11)
- r9 kernel: name-resolution/namespace core (Chan/Mount/pgrp + union walk), devmnt (9P client), devsrv/devdup/devpipe/devenv devices, devproc extension, devcons read+cooked path — all keyed to the measured trace
- r9 xtask: initrd baking and `../harvey` path glue removed/inverted per D-10 (the kernel package build is what remains)
- `build/aarch64/`: image root-tree skeleton + bake script (mkpaqfs over the staged root) replacing the staged-init `out/` flow
- `.github/workflows/`: new INT-02 workflow (linux leg) alongside the legacy harvey.yml

</code_context>

<specifics>
## Specific Ideas

- The acceptance story, end-to-end: `nix run .#boot-r9` → userland builds (lima on darwin) → initrd bakes → qemu launches → serial shows boot's prints → `init: starting /bin/rc` → `% ` prompt. The user types `ls /bin`, `ps`, `cat /lib/namespace`, `echo hello` — each answers correctly. DEL at the prompt posts an interrupt note without killing rc. `exit` → init restarts rc after 1s (the `for(;;)` loop — the free end-of-demo behavior).
- Smoke markers (D-09) must be preemption- and TCG-timing-tolerant: assert marker presence, never interleaving order (Phase 4 D-13 discipline); expect timeouts sized for TCG on GH ubuntu runners.
- Watch-out: the userspace paqfs server needs the raw initrd image exposed as a file (devroot raw-image node, e.g. `/dev/rootfs` or a well-known path in the root tree) — while the kernel also execs `/boot/boot` from the *walked* paqfs tree. Two views of the same image; planner defines both.
- Watch-out: boot's `fsession`/`fversion` against the userspace paqfs server through devmnt — the 9P message set the server must answer is part of the donor trace; paqfs(4) is a real server and should cover it, but the trace settles it.
- Watch-out: rc's `sigexit` fn and DEL-note interplay — a DEL at the wrong boundary (mid-command-read) must not kill the shell; the D-14 Wave-0 race fix lands before this is exercised.
- Watch-out: `pagealloc`'s 2GiB ceiling — keep qemu `-m` ≤ 1G.
- Watch-out: `boot.c`'s `bind("#æ"/"#S", "/dev", MAFTER)` calls fail-soft already (no fatal check) — nonexistent devices in r9 are tolerated there, but `connectlocal`'s `bind("#c"/"#p", …)` calls **fatal** — the embedded method's bind set must match devices r9 actually has.
- User overrides to honor downstream (deliberate, don't second-guess): full real chain over minimal staged path (fidelity is the point — r9 becomes a real Plan 9 environment); full mount/devmnt over bind-only (boot.c unmodified + virtio-9p prerequisite); full devcons over cooked-only; linux-only CI over dual-leg (darwin stays documented-manual).
- The staged `init.c`/`smokefork`/`smokenote` binaries and the virtexec/virtmulti/virtnotes profiles remain as r9-side regression gates — they're not the phase's deliverable but must not rot (the r9 boot-test suite keeps running them).

</specifics>

<deferred>
## Deferred Ideas

- virtio-9p host mount (collapses userland iteration time) — v1.x top candidate; this phase's devmnt is its prerequisite
- boot(8)'s disk/network methods (local fossil/kfs, tcp boot, DHCP/pxe paths) — real hardware/network era
- Real auth: factotum handshake, auth server, secstore — auth stubs this phase; the security milestone owns it
- factotum/ipconfig in the boot image — subset binaries exist but the scoped termrc doesn't start them; network-era concern
- SMP via PSCI, virtio-net/blk, RTC, PCIe ECAM — v2
- rio/acme/devdraw/devmouse — graphical era (devcons raw mode landed here is their prerequisite)
- amd64-on-r9 (replace 9k on the primary arch) — v2
- 9k-amd64 pool-panic re-test with the LP64-fixed 6c (phase-02 deferred #8) — amd64-side debt
- Phase-04 deferreds #3 (cold-pointer syscall aborts) and #4 (contained heisenbug root cause) + 8 open Info findings — milestone backlog
- raspi3b-on-qemu-11.1.2 upstream defect — pre-existing, human-accepted 2026-11-01
- HVF acceleration for virt boots — needs a GICv3 driver
- Upstream contribution flow to original r9 — fork `requaos/r9` remains the working repo

</deferred>

---

*Phase: 5-Shell Boot + One-Command Integration*
*Context gathered: 2026-11-02*
