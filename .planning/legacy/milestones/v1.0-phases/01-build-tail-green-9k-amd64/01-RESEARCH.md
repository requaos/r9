# Phase 1: Build-Tail Green (9k amd64) - Research

**Researched:** 2026-10-30
**Domain:** Plan 9 mk-based build-system repair (namespace build on Linux hosts), kernel root-embedding machinery, build reproducibility, minimega VM smoke harness
**Confidence:** HIGH (nearly every claim verified by direct in-repo reads this session; minimega behavior verified against pinned-tag upstream source)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Fix depth + loop resilience**
- **D-01:** Fix-depth target is **fix everything** — every component of the default cmd build must compile: `$PROGS` (single-file cmds) + `cc` + all `DIRS` subdirs in `sys/src/cmd/mkfile` (minus its NOMK exclusions: `sml|dup|.+\..+|unix|old`). This includes ghostscript, games/mahjongg, cifs, aquarela. — **Reversibility:** costly — the failure tail is unknown length; changing course later means re-scoping already-landed fix work.
- **D-02:** Vendored snapshots (ghostscript, antiword, apeg) get **patched in place** like any other code. No upstream-compat preservation — this fork owns them. **BUT — toolchain-first diagnostic rule:** this vendored code built fine for decades, so a fresh build failure is *probably our toolchain* (cpp/6c/pcc) having drifted or being buggy — cf. mahjongg, which breaks because Harvey's own cpp is buggy (Dockerfile TODO). When vendored code fails, first check whether our compiler/preprocessor is at fault (e.g. preprocess the same file with a reference cpp, compare against a known-good toolchain run); fix toolchain bugs in the toolchain. Patch the vendored source only when the vendored code itself is genuinely at fault.
- **D-03:** mk DIRS loops keep **strict abort-on-first-failure** semantics. Add a separate report-all diagnostic target (e.g. `mk checkall`) that runs every dir and prints the full failure list — visibility without weakening the default gate.
- **D-04:** Sequencing is **build-order whack-a-mole**: run the build, fix the current blocker, rerun. No upfront failure inventory; unknown tail length accepted. The decoded chain from research (`jpg/mkfile` → cmd loop abort → missing `ipconfig`/`venti`) is the starting point, not a pre-planned work order.

**aarch64 quarantine mechanics**
- **D-05:** Vestige handling is **rename + remove**: rename `build_arm64` → `build_aarch64` in `build/scripts/funcs` and `build/scripts/linux-build` (fixing their `objtype=aarch64` + `cmd/7?` references so they're correct-but-dormant until Phase 2), drop the `arm64/bbin` entry from `build/mkdirs`, remove the empty root `arm64/` dir if present.
- **D-06:** Donor import naming (Phase 2 rule, locked now): **rename on import** — 9front's `arm64` paths normalize to `aarch64` (`libc/arm64` → `libc/aarch64`, `arm64/mkfile` → `aarch64/mkfile`, `objtype=aarch64`) via a repeatable normalization step; donor file contents otherwise untouched. — **Reversibility:** one-way.

**Correction to D-05 found during research:** `build_arm64` does **not** exist in `build/scripts/funcs` (verified by reading the file — it contains only `isodate`, `buildinit`, `quitonfailure`). The two live sites are `build/scripts/build:76-80` and `build/scripts/linux-build:63-67`. Two additional `arm64` sites CONTEXT/PITFALLS missed are documented below (BUILD_SETUP_NAMESPACE bind-mount, dist/replica/plan9.proto entry).

### the agent's Discretion
- **Reproducibility policy (BUILD-04)** — pinned timestamps (SOURCE_DATE_EPOCH-style, e.g. derived from the git commit) over build-time `date -n`; scope = kernels (kernel embeds KERNDATE). Document the policy in-repo; the planner may extend the policy to the full tree if cheap.
- **minimega smoke harness (BUILD-05)** — boot `9k10cpu` via `vm config kernel`+`initrd` on a Linux KVM host; `.mm` scripts live in-repo (suggest `build/mm/` or `nix/`); assert boot via serial-log capture (no miniccc in-guest agent — Harvey isn't a Linux guest); CI wiring optional this phase.

### Deferred Ideas (OUT OF SCOPE)
- Attic/quarantine for skeletal ports (riscv64 vaporware tree, arm/mips/power fragments) beyond the `arm64` vestiges — broader cleanup, belongs to a later hygiene phase (per CONCERNS.md)
- TLS 1.3 / cipher pruning — separate hardening milestone
- 386 kernel + ISO/dist stage + expect/qemu ISO build — deferred per project scope
- minimega cluster/CI deployment — local KVM-host harness first this phase
- NOT in this phase: 386 legacy kernels, ISO/dist stage, aarch64 toolchain imports (Phase 2), r9 work (Phases 3–4)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BUILD-01 | `nix run .#build` from clean checkout produces k10cpu + k10cpuf + k10f — fix jpg/mkfile `oc` derivation, 4a lex.c failure, remaining tail | Full generator inventory decoded (§Architecture Patterns, §Common Pitfalls): exact broken idiom (`$c^c` tool-prefix derivation) vs working idiom (`objtype=$cputype mk` recursion), the five affected mkfiles, and the fix pattern. Build pipeline mapped end-to-end (nix app → BUILD → linux-build → kernels). |
| BUILD-02 | `/amd64/bin` carries `ip/ipconfig`, `auth/factotum`, `venti/venti` so embedded-root kernels link | Rootdir embedding chain fully decoded (parse awk `mkrootrules` → `$CONF.root.s` → `mkrootall` → `addbootfile`); exact per-kernel file lists quoted below; correction: **all three** kernels (incl. k10cpu) embed factotum+ipconfig — k10cpu only "stays green" when `/amd64/bin` is already populated. |
| BUILD-03 | Vestigial `arm64` artifacts renamed/quarantined to `aarch64` | Complete grep-verified inventory below (§Common Pitfalls 3), including two sites prior research missed (`sys/src/BUILD_SETUP_NAMESPACE` bind-mount — **coupled to root `arm64/` dir existence**, and `dist/replica/plan9.proto:38-39`). |
| BUILD-04 | KERNDATE/reproducibility policy decided + documented | Both `-DKERNDATE` sites located (k10/mkfile:20, 9k/mk/portmkfile `$CONF.$O` rule); a second non-determinism source found (`conffile` embeds absolute build path); SOURCE_DATE_EPOCH convention cited from reproducible-builds.org spec; vm-build.sh `.git` exclusion constrains where the value can be computed. |
| BUILD-05 | minimega packaged by flake + `.mm` boots 9k10cpu on Linux KVM host, boot log captured | `nix/minimega.nix` already committed (tag 3.3.0 = latest upstream, hash-pinned). qemu CLI construction verified in upstream source at the pinned tag: `vm config kernel` → `-kernel` (multiboot-loads `9k10cpu.elf`; multiboot header verified in `l32p.s:25-28`), serial capture via `serial-ports` unix sockets, `-S` start-paused (must `vm start`), no `-enable-kvm` in generated args (TCG fallback possible). **Environment gap: no KVM host in current dev fleet** (see Environment Availability + Open Question 1). |
</phase_requirements>

## Summary

This phase is almost entirely **in-tree mk/rc/C repair against a fully decoded failure model** — no new libraries or frameworks are introduced. The build pipeline is: `nix run .#build` → (darwin: lima rsync to linux guest) → `nix/build.sh` → `RUN_BUILD_IN_PRIVATE_NAMESPACE` → `BUILD_SETUP_NAMESPACE` (bind-mounts the checkout over `/sys`, `/amd64`, `/386`, `/rc`, …) → `sys/src/BUILD` (`mk clean; mk libs; mk all`, objtype=amd64) → `build/scripts/linux-build` (arch loop → 9k kernel loop → dist/man stages). Two structural breakages are already decoded: (1) the jpg/jtagfs mkfiles derive a tool prefix from `$cputype` (`c=o` under `cputype=unix` → nonexistent `oc`/`ol`), which aborts mk's strict DIRS loop and starves the userland install; (2) kernel configs embed userland binaries (`/amd64/bin/auth/factotum`, `/amd64/bin/ip/ipconfig`, ± fossil/venti) via `sys/src/9k/mk/parse` + `mkrootall`, so any cmd-chain abort detonates the kernel link later in a different directory with a misleading error.

The other three requirements are small, well-bounded edits: the `arm64` quarantine is a six-site mechanical change (two newly discovered sites have a **coupling trap** — removing root `arm64/` without editing `BUILD_SETUP_NAMESPACE` breaks the namespace build because `mount --bind` fails on a missing source); KERNDATE pinning is a two-site mkfile change plus a policy doc (with a second embedded-path non-determinism source, `conffile`, to write down); and the minimega smoke harness's packaging is already committed — what remains is the `.mm` script, serial capture, and resolving **where the Linux KVM host comes from** (the dev fleet has none: this mac is darwin-aarch64 and the lima guest is linux-aarch64 with no `/dev/kvm`).

**Primary recommendation:** Plan as three parallel workstreams ordered by dependency — (A) cmd-chain whack-a-mole repairs (D-04) gated by a new `checkall` report-all target (D-03) and a post-cmd preflight asserting the three kernel-critical binaries exist; (B) the mechanical quarantine + KERNDATE pin + policy doc (independent of A); (C) the minimega `.mm` harness, verified first in degraded TCG mode, gated on a user checkpoint for the KVM-host demonstration that BUILD-05's letter requires.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Host toolchain bootstrap (pcc, cpp, 6a/6c/6l, iar, awk/yacc/lex) | Host build scripts (`nix/setup.sh`, `nix/toolchain.sh`, `*/BUILD`) | — | These produce the Linux binaries the namespace build execs; failures here are toolchain-tier, not mkfile-tier (D-02's diagnostic rule keys on this distinction) |
| Compile-and-run generators (rgbycc, genrtab, maketab, pcode, genarch) | Host tier (must produce **host-runnable** binaries) | cmd mkfiles | Generators execute during the build; under the namespace build the target (Plan 9 amd64 a.out) is unrunnable on Linux — the unix/mkfile `9c/9l` indirection is the in-tree host-tool escape |
| cmd/library/userland compile + install | mk + in-tree mkfiles (namespace build) | `sys/src/cmd/mkfile` DIRS loop | Strict abort semantics make one broken dir mask the rest — the reason for D-03's `checkall` |
| Kernel config → devtab/systab/root embedding | 9k mk machinery (`mk/parse`, `mkrootall`, `bootmkfile`, `portmkfile`) | userland install (supplies rootdir inputs) | Cross-tree dependency is invisible to mk; only a preflight assertion makes it explicit |
| Reproducibility (KERNDATE) | k10 mkfile + portmkfile (`-DKERNDATE`) | `nix/build.sh` (value derivation/export) | mkfile applies, build script derives — value must be computed before/above the namespace (git absent in lima guest checkout) |
| `arm64`→`aarch64` quarantine | Build scripts + tree layout | namespace setup script | BUILD_SETUP_NAMESPACE bind-mounts the root dir — rename sites are coupled |
| Smoke harness (boot 9k10cpu, capture log) | Host orchestration (minimega + qemu on Linux) | flake packaging (already committed) | minimega owns VM lifecycle; the repo owns the `.mm` script + capture/assert wrapper |

## Project Constraints (from AGENTS.md)

- **Tech stack**: Plan 9 C dialect + mk; host tools must build with plan9port; `$PLAN9/bin` **LAST** in PATH (plan9port's 9p `mount` must not shadow util-linux mount — already encoded in flake.nix wrappers; do not regress).
- **Host platforms**: darwin-aarch64 (this machine, via lima VM) and linux aarch64/x86_64; the namespace build requires Linux mount namespaces.
- **Compatibility/conventions**: `sys/man/6/style` — tabs, lowercase, `nil`, `exits`/`sysfatal`, block comments only, no `#ifndef` header guards, no `//` comments. Match the file being edited.
- **Naming**: `aarch64`, never `arm64`, in tooling and docs.
- **Upstream posture**: dormant; this fork owns fixes — in-tree fixes preferred over external patch stacks (reinforces D-02).
- **GSD workflow**: edits land through the phase workflow with atomic commits.

## Standard Stack

This phase introduces **no new language libraries**. The "stack" is the existing build machinery plus two host-side tools for the smoke harness.

### Core (all already in-tree or already-packaged — verified this session)
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| mk (plan9port) | plan9port unstable-2025-11-10 (nixpkgs pin) [VERIFIED: nix store path `plan9port-0-unstable-2025-11-10` on this machine] | Build driver | The entire tree is mkfiles; plan9port supplies it on hosts |
| plan9port host tools (9c/9l/awk/yacc/lex/sed/date/xd/file/rc) | same pin | Host compilers + POSIX-ish tools mkfiles expect | `nix/setup.sh` copies them into `.build/hostbin-<os>-<arch>/bin` (first in PATH) [VERIFIED: nix/setup.sh:33-35 loop over `sed yacc lex awk date dd rm basename ls`] |
| Vendored 9-cc `iar` (as host `ar`) | in-tree `sys/src/cmd/unix/9cc` | Plan 9-format archives from host | Established vendor-and-own pattern; `sys/src/cmd/unix/9cc/BUILD` builds with host cc [VERIFIED: sys/src/cmd/unix/9cc/BUILD:1-30] |
| qemu (x86_64 system emulation) | nixpkgs pin (flake devshell/build app) | VM runner under minimega | minimega invokes it; no direct dependency management needed |
| minimega | 3.3.0 (pinned in `nix/minimega.nix`; **verified latest upstream tag** via GitHub API, commit `7157b8e60b68e92a1101683dc16dbcf33c3705f0`) [VERIFIED: api.github.com/repos/sandia-minimega/minimega/tags] | VM orchestration for the smoke harness | BUILD-05's mandated tool; Sandia official project; already packaged in-tree |

### Supporting
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| socat (or `nc -U`) | any | Connect to minimega's per-VM serial unix socket → boot log file | The serial-capture half of the smoke harness [CITED: minimega 3.3.0 `cmd/minimega/kvm.go` — serial chardev socket at `$minimega_runtime/<vm_id>/serial<N>`] |
| git | host | Derive pinned KERNDATE (`git log -1 --format=%ct`) | BUILD-04 policy — must run **host-side** (vm-build.sh excludes `.git` from the guest rsync) [VERIFIED: nix/vm-build.sh:36-39] |
| expect | nixpkgs pin | NOT needed for BUILD-05 (that was the old ISO bootstrap's tool) | Only if the planner wants expect-style assertions outside minimega — prefer serial-log grep |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| minimega `.mm` harness | Raw qemu + expect script (upstream CI's old pattern) | minimega is the locked decision (BUILD-05 text); it gives VM lifecycle/QMP/serial sockets for free; raw qemu is the documented fallback for the TCG-degraded path and for Phase 3+ aarch64 boots |
| `objtype=$cputype mk` generator fix (awk pattern) | New `hostcmd` mk template / more hostbin entries | The awk/spell pattern is already in-tree and proven; a third pattern adds machinery where two suffice. Prefer normalizing jpg/jtagfs onto the existing idiom |

**Installation:** nothing to install — `nix run .#setup` / `.#toolchain` / `.#build` / devshell already wrap everything. `nix build .#minimega` (linux) builds the smoke-harness binary from the committed derivation.

## Package Legitimacy Audit

The only external artifact this phase consumes is **minimega**, and it was already committed (`817ac1d66 build: package minimega via nix (linux)`) as a `fetchFromGitHub` + `buildGoModule` derivation with a pinned tag and output hash — not a registry package. Registry checks are category-inapplicable and were run anyway for the record:

| Package | Source | Registry check | Verdict | Disposition |
|---------|--------|----------------|---------|-------------|
| minimega 3.3.0 | `github.com/sandia-minimega/minimega`, tag `3.3.0`, `hash = "sha256-jcQH+F/YSluNrt+fhnB81iFPpUgFlpxuNc4Zse6rD1o="` [VERIFIED: nix/minimega.nix:9-14] | npm: not present (expected); PyPI: unrelated same-name package (expected) | **OK** (direct-from-upstream: official Sandia National Laboratories project, repo active since 2012, tag verified via GitHub API; upstream ships `vendor/` so the nix build performs **zero** registry fetches — `vendorHash = null` [VERIFIED: nix/minimega.nix:31-32]) | Approved (already committed) |
| libpcap (buildInput) | nixpkgs | n/a (nixpkgs channel pin) | OK | Approved |

**Packages removed due to SLOP verdict:** none (the npm "minimega" absence is expected — wrong registry for a Go project).
**Packages flagged SUS:** none.

*No packages discovered via WebSearch/training are recommended in this research. All other tooling comes from the nixpkgs pin already in `flake.lock`.*

## Architecture Patterns

### System Architecture Diagram (the build pipeline this phase repairs)

```
nix run .#build
│
├─ darwin: nix/vm-build.sh ──rsync(excludes .git .build .planning)──> lima guest (linux-aarch64)
│                                          └─ guest runs: setup → toolchain → build
└─ linux:  nix/build.sh
           ├─ preflight: pcc cpp ar present in .build/hostbin-<os>-<arch>/bin
           ├─ idempotent sed fixups (aux/mklatinkbd→mklatinkbd, aux/data2s→data2s)
           ├─ bash build/mkdirs            (creates per-arch dirs incl. ./arm64/bbin ← BUILD-03)
           ├─ sudo cp cpp bind rc 6a 6c 6l 8a 8c 8l → /bin   (pcc execs /bin/cpp, /bin/6c…)
           └─ sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE
              └─ sudo unshare -m bash BUILD_SETUP_NAMESPACE
                 ├─ mount --bind $HARVEY/{arm,arm64,amd64,386,…} → /arm, /arm64, /amd64, /386…
                 │     ▲ BUILD-03 coupling: bind FAILS if $HARVEY/arm64 removed first
                 └─ su -c ./BUILD
                    ├─ mk clean; mk libs; mk all        (objtype=amd64; libs → cmd → games)
                    │    └─ sys/src/cmd/mkfile: $PROGS, then `for(i in cc $DIRS)`  ← STRICT abort
                    │         ├─ 4a … cc … awk … gs … ip … jpg jtagfs … venti …   (ls order)
                    │         └─ generator mkfiles: jpg/jtagfs BROKEN ($c^c → oc), awk/spell/gs OK-ish
                    └─ rc $HARVEY/build/scripts/linux-build
                         ├─ for arch in (386 amd64): mkdir /$i/bin/{auth,aux,ip,venti,…}; build_$i
                         │    ├─ build_386: full 386 userland + 9/pc kernels + pcboot   ← 386 scope Q
                         │    └─ build_amd64: libc, 6?, objtype=amd64 mk install, then
                         │         for i in k10cpu k10cpuf k10f: mk CONF=$i install     ← THE DELIVERABLE
                         │              └─ parse $CONF → $CONF.c + $CONF.root.s rule
                         │              └─ mkrootall cp/file/strip/data2s /amd64/bin/… → addbootfile
                         └─ dist + man stages (/sys/lib/dist, lookman, sys/man)          ← scope Q
```

### Pattern 1: Host-runnable generators via the `unix` pseudo-arch (the fix for BUILD-01's head)

**What:** Generator programs (compile-then-execute during build) must be built with **host** tools. The in-tree blessed idiom is awk's: recurse with `objtype=$cputype`, which resolves `CC=9c LD=9l O=o` via `/unix/mkfile` — plan9port's host compiler/linker producing a host-runnable binary.

**Verified working idiom** [VERIFIED: sys/src/cmd/awk/mkfile:42-50]:
```mk
proctab.c:	$cputype.maketab
	./$cputype.maketab >proctab.c

$cputype.maketab:	y.tab.h maketab.c
	objtype=$cputype mk maketab.$cputype

maketab.$objtype:V:
	$CC $CFLAGS maketab.c
	$LD $LDFLAGS -o $objtype.maketab maketab.$O
```
With `cputype=unix` [VERIFIED: nix/build.sh:12] and `unix/mkfile` = `CC=9c LD=9l O=o AS=9a` [VERIFIED: unix/mkfile:3-6], this builds `unix.maketab` with plan9port's `9c`/`9l` — a Linux binary — and runs it.

**Broken idiom (the current blockers)** [VERIFIED: sys/src/cmd/jpg/mkfile:45,61-69]:
```mk
c=`{sed -n 's/^O=//p' /$cputype/mkfile}		# → o  (because unix/mkfile has O=o)

ycbcr.h:	rgbycc.c
	$c^c rgbycc.c					# → oc  (does not exist)
	$c^l -o $c.rgbycc rgbycc.$c			# → ol  (does not exist)
	$c.rgbycc > ycbcr.h
```
Same shape at `sys/src/cmd/jtagfs/mkfile:5,47-49` (`Oo=`{…}` → `oc -FVw genrtab.c && ol -o o.genrtab …`).

**When to use:** jpg (`rgbycc`, `rgbrgbv`) and jtagfs (`genrtab`) should be normalized onto the awk idiom (or an explicit `9c/9l` host rule). spell (`pcode`, `sys/src/cmd/spell/mkfile:11-29`) and awk itself already use the good idiom; gs's `genarch` falls back to committed `default.$objtype.h` headers when `objtype != cputype` and `default.amd64.h` exists [VERIFIED: sys/src/cmd/gs/mkfile:146-170 + `default.amd64.h` present in dir listing] — gs's generator is likely **not** a blocker for amd64.

**Toolchain-first diagnostic (D-02):** before patching any vendored C, preprocess with a reference cpp and compare — mahjongg's failure is a *known Harvey cpp bug* [VERIFIED: build/Dockerfile:13: `# TODO mahjonng doesn't build becuase of problems with cpp - fix cpp!`].

### Pattern 2: Embedded-root kernel inputs are a *postcondition*, not a kernel bug

The kernel configs' `rootdir` sections [VERIFIED: sys/src/9k/k10/k10cpu:71-75, k10cpuf:79-85, k10f:89-95]:
```
# k10cpu (all three kernels embed these two):
	bootk10cpu.out boot
	/amd64/bin/auth/factotum factotum
	/amd64/bin/ip/ipconfig ipconfig
	../root/nvram nvram
# k10cpuf/k10f additionally:
	/amd64/bin/fossil/fossil fossil
	/amd64/bin/venti/venti venti
```
`sys/src/9k/mk/parse` (`mkrootrules`, lines 466-478) turns each into a `$CONF.root.s` dependency produced by `../mk/mkrootall` (cp → `file` → best-effort `strip` → `data2s`). **Correction to prior research:** k10cpu is *not* "non-embedded" — it embeds factotum+ipconfig too; it merely built green in the prior session because `/amd64/bin` was already populated. From a truly clean checkout (`amd64/*bin` is gitignored [VERIFIED: .gitignore]), **all three** kernels fail without the userland install.

**Preflight assertion (Pitfall-2 fix, belongs in `nix/build.sh` or the amd64 build path before the kernel loop):**
```bash
for f in /amd64/bin/ip/ipconfig /amd64/bin/auth/factotum /amd64/bin/fossil/fossil /amd64/bin/venti/venti; do
  [ -x "$HARVEY$f" ] || { echo "preflight: $f missing — cmd chain incomplete" >&2; exit 1; }
done
```
(Paths inside the namespace are the bind-mounted checkout subdirs, so asserting against `$HARVEY/amd64/...` before entering the namespace is equivalent.)

### Pattern 3: Strict loops + a report-all diagnostic (D-03)

mk aborts the whole recipe when any iteration of `for(i in cc $DIRS) @{ cd $i; mk $MKFLAGS all }` fails [VERIFIED: sys/src/cmd/mkfile:33-38]. Add a sibling target that never aborts mid-loop and prints the failure list at the end. Sketch for `sys/src/cmd/mkfile`:
```mk
checkall:V:
	fail=()
	for(i in cc $DIRS) @{
		cd $i
		if(! mk $MKFLAGS all)
			fail=($fail $i)
		cd ..
	}
	if(! ~ $#fail 0) {
		echo 'checkall failures: '^$"fail
		exit checkall
	}
```
(rc idiom — capture per-dir status without exiting; exact syntax to be tuned at execution. Keep it *separate* from `dirs`/`all` per D-03: default semantics unchanged.)

### Pattern 4: KERNDATE pinning (BUILD-04)

Current: `-DKERNDATE=`{date -n}` at **two** sites [VERIFIED: sys/src/9k/k10/mkfile:20 and the `$CONF.$O` rule in sys/src/9k/mk/portmkfile]; `parse` emits `ulong kerndate = KERNDATE;` into every `$CONF.c` [VERIFIED: sys/src/9k/mk/parse:461].

Policy (locked by discretion default): pin from the git commit, SOURCE_DATE_EPOCH-style [CITED: reproducible-builds.org/specs/source-date-epoch — "A UNIX timestamp… seconds, excluding leap seconds, since 01 Jan 1970 00:00:00 UTC"]. Implementation shape:
- `nix/build.sh` (host side, **before** namespace): `KERNDATE="${KERNDATE:-$(git log -1 --format=%ct 2>/dev/null || date +%s)}"`; export it. Must also be computed host-side in the darwin path because `nix/vm-build.sh` rsyncs with `--exclude .git` [VERIFIED: nix/vm-build.sh:37] — the guest checkout has no git metadata.
- mkfiles honor the env var, falling back to `date -n`. rc substitution idiom at both sites:
  ```mk
  '-DKERNDATE='`{if(~ $#KERNDATE 0) KERNDATE=`{date -n}; echo $KERNDATE}
  ```
- **Second non-determinism source the policy doc must name:** `parse` emits `char* conffile = "<absolute build pwd>/$CONF"` [VERIFIED: sys/src/9k/mk/parse:455-460] — kernels are bit-reproducible only for identical checkout paths unless this is normalized. Cheapest honest policy: "bit-identical for same-path clean builds; conffile path embedding documented as the cross-path exception" (the success criterion's "or the timestamp exception written down" clause explicitly allows this).
- nvram is created deterministically (512 zero bytes) by linux-build when absent [VERIFIED: build/scripts/linux-build:35-36]; gitignored.

### Pattern 5: minimega smoke harness (BUILD-05)

All mechanisms verified against minimega 3.3.0 source (`cmd/minimega/kvm.go` at tag `3.3.0`) and official module docs:
- `vm config kernel <path>` → qemu `-kernel`; `vm config initrd` → `-initrd`; `vm config append` → `-append` (multiboot cmdline). [VERIFIED: kvm.go `qemuArgs`]
- `vm launch kvm <name>` builds the VM; qemu is started **paused** (`-S` is always in the generated args); `vm start <name>` resumes via QMP. [VERIFIED: kvm.go `qemuArgs` + `Start()`]
- `vm config serial-ports 1` → qemu chardev socket, server=on,wait=off, at `$minimega_runtime/<vm_id>/serial0` (default runtime dir `/tmp/minimega`). Capture: `socat -u UNIX-CONNECT:/tmp/minimega/0/serial0 CREATE:boot.log &`. [VERIFIED: kvm.go `SerialPorts` doc comment + `qemuArgs` chardev construction]
- `vm config baremetal true` suppresses PC display/VNC/USB/CD-ROM extras and routes qemu stdout/stderr to `<vm_id>/qemu.log`; requires kernel image + `backchannel false`. Optional here (k10cpu probes PC hardware; non-baremetal's default device set is also fine). [VERIFIED: kvm.go `Baremetal` doc comment + `NewKVM` validation]
- **No `-enable-kvm`/`-accel` appears in the generated qemu args** — acceleration comes from the qemu binary (`QemuPath` default `"kvm"`). `vm config qemu /nix/store/.../qemu-system-x86_64` therefore yields TCG — the degraded-mode path for hosts without `/dev/kvm`. [VERIFIED: kvm.go `qemuArgs` — read in full]
- `.mm` files = one minimega command per line, run via `minimega -e read script.mm` against a daemon started as `minimega -nostdin &` (root, for KVM). [CITED: sandia.gov/minimega Module 18]

Harvey side (verified in-tree): `9k10cpu.elf` is ELF via `6l -H5 -E0x00110000 -P0x00110000` [VERIFIED: sys/src/9k/k10/mkfile:22]; the multiboot header sits at the head of `l32p.s` (`LONG $0x1badb002`, flags 3, checksum) [VERIFIED: sys/src/9k/k10/l32p.s:25-28] and `l32p` is first in the config's `amd64 +dev` object list [VERIFIED: k10cpu:107-131], so qemu's `-kernel` multiboot loader can find it. k10 enables the COM1 serial console **unconditionally** early in `main()` — `i8250console("0");` [VERIFIED: sys/src/9k/k10/main.c:182] — and prints the `\nPlan 9` banner at line ~184, so serial capture needs no boot-line surgery. `vm config initrd` is likely **unnecessary** (root fs is embedded); CONTEXT's "kernel+initrd" phrasing should not force one — decide at harness bring-up.

### Anti-Patterns to Avoid
- **Deriving tool names from `$cputype`/`O=` in generator recipes** — the root cause of BUILD-01's head. Any new generator goes through the awk idiom or an explicit `9c/9l` host rule. Grep-watchlist: `\$c\^`, `Oo=`, `cpuobjtype` in mkfiles.
- **"mk exited 0 ⇒ everything installed"** — strict-abort loops skip the tail silently. Verify *files exist* (preflight), not exit status (PITFALLS "Looks Done But Isn't").
- **Editing `build_arm64` without `BUILD_SETUP_NAMESPACE`** — removing root `arm64/` while the namespace script still bind-mounts it breaks every subsequent build.
- **Computing KERNDATE inside the guest/namespace** — no `.git` there (rsync excludes it); derive host-side, export through.
- **Adding `-enable-kvm` assumptions to the harness** — minimega doesn't emit it; don't write docs/scripts that assume KVM presence without a probe.
- **Letting the smoke harness need an in-guest agent** (miniccc) — Harvey isn't a Linux guest; serial-log assertion only (locked discretion).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| VM orchestration for smoke test | custom bash+expect qemu wrapper | minimega (BUILD-05) + `.mm` script | lifecycle, QMP, serial sockets, error states already solved; upstream CI's expect+prompt-matching ISO bootstrap is the documented cautionary tale (CONCERNS.md "Fragile bootstrap build") |
| Serial boot-log capture | pexpect on qemu stdio | `vm config serial-ports 1` + `socat UNIX-CONNECT:` on `/tmp/minimega/<id>/serial0` | minimega already wires the chardev socket; stdio is muxed/captured internally and unreliable |
| Host-runnable build generators | new mk include template / new hostbin machinery | awk idiom: `objtype=$cputype mk <tool>.$cputype` → `9c/9l` via `unix/mkfile` | already in-tree at awk/spell; proven; zero new machinery |
| Reproducible timestamps | custom epoch format or a committed `VERSION` file | SOURCE_DATE_EPOCH convention: pin from `git log -1 --format=%ct`, honor env override | ecosystem-standard [CITED: reproducible-builds.org spec]; tools and reviewers recognize it |
| Plan 9 object stripping in mkrootall | fixing GNU strip to read 6l a.out | accept soft-fail (mkrootall has no `-e`; `strip` failure is non-fatal) or host-build `sys/src/cmd/strip.c` later | GNU binutils strip (in the build app's runtimeInputs) rejects Plan 9 binaries; embedding proceeds unstripped — deterministic, harmless, just larger. Not on the green path. |
| Failure-tail visibility | editing `dirs`/`all` to continue-on-error | separate `checkall` target (D-03) | locked decision: default gate stays strict |

**Key insight:** every problem in this phase already has an in-tree or mandated solution; the work is *aligning* broken call sites to existing idioms, not inventing mechanisms.

## Runtime State Inventory

This phase contains a rename (`build_arm64`→`build_aarch64`, `arm64/` removal), so the five categories are answered explicitly:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no databases/datastores hold `arm64` or build state. `amd64/bin` etc. are regenerable build outputs (gitignored). | none |
| Live service config | None — no external services configured from this tree. | none |
| OS-registered state | `nix/build.sh` sudo-copies `cpp`, `bind`, `rc`, `6a/6c/6l/8a/8c/8l` into the **host/guest `/bin`** each run [VERIFIED: nix/build.sh:39-46] — pre-existing behavior, unrelated to the rename, idempotent. No launchd/systemd/Task Scheduler state. | none |
| Secrets/env vars | None reference `arm64`. Env contract that matters: `HARVEY`, `HARVEY_LINUX_BIN`, `objtype=amd64`, `cputype=unix`, `FORCERCFORMK=1` (build.sh:4-13), and (new) `KERNDATE`. | add `KERNDATE` export (code edit, not a secret) |
| Build artifacts | (a) untracked empty root `arm64/bbin/` — local leftover of `build/mkdirs` runs (untracked: `git ls-files arm64` is empty); (b) untracked generated `sys/src/9k/k10/k10f.c`, `sys/src/9/pc/pccpuf.c` from a prior build; (c) `.build/hostbin-{darwin,linux}-aarch64/` toolchains; (d) gitignored `/sys/src/9k/root/nvram`. | (a) remove dir + drop mkdirs entry + edit BUILD_SETUP_NAMESPACE **in one commit**; (b) covered by `$ARCH.clean` (`rm -f $ARCH^*.c`) — consider gitignoring generated `$CONF.c` (Pitfall 14 hygiene); (c)(d) regenerate — no action |

**Nothing-found categories stated explicitly above.** The dirty-tree files (`rc/bin/Kill`, troff fonts, postscript C1-C3 modifications) are pre-existing unrelated local edits — flag to the user before any `git clean`-based "clean checkout" verification.

## Common Pitfalls

### Pitfall 1: `$cputype`-derived generator tool names (`oc` class of failure)
**What goes wrong:** `jpg/mkfile:45` and `jtagfs/mkfile:5` compute a tool prefix from `/$cputype/mkfile`'s `O=`; under `cputype=unix` (`O=o`) recipes invoke nonexistent `oc`/`ol`. mk aborts the DIRS loop; everything alphabetically after the failing dir silently never builds.
**Root cause:** mkfiles written for self-hosting Plan 9 (host==target==`$cputype`); the namespace build exports `cputype=unix` [VERIFIED: nix/build.sh:12].
**How to avoid:** normalize onto the awk idiom (Pattern 1). Grep sweep for the whole class: `grep -ln 'cputype' sys/src/cmd/*/mkfile` → currently `awk gs jpg jtagfs spell` [VERIFIED this session]; awk/spell/gs use survivable idioms, jpg/jtagfs are broken.
**Warning signs:** `o^c`-style concatenated names in logs; "not found" for `oc`/`ol`; zero-length `ycbcr.h`/`rgbv.h`/`rtab.c`.

### Pitfall 2: Embedded-root kernels fail late, far from the cause
**What goes wrong:** kernel link dies in `mkrootall`/`$CONF.root.s` naming a missing `/amd64/bin/...` file; looks like a kernel bug, is actually a userland-ordering symptom. **All three** kernels embed factotum+ipconfig (see Pattern 2 correction).
**How to avoid:** preflight assertion after the cmd phase (Pattern 2 snippet); verify `k10cpuf` **and** `k10f` link, not just k10cpu ("Looks Done But Isn't" checklist).

### Pitfall 3: `arm64` quarantine has a bind-mount coupling (NEW finding)
**What goes wrong:** D-05 says remove root `arm64/`; but `sys/src/BUILD_SETUP_NAMESPACE` does `mkdir -p /arm64; mount --bind $HARVEY/arm64 /arm64` — `mount --bind` **fails when the source doesn't exist**, breaking the whole namespace build.
**Complete `arm64` inventory** (grep, excluding `.git`/`.planning`; verified this session):
| Site | Disposition |
|------|-------------|
| `build/scripts/build:76-80` `fn build_arm64` (`objtype=arm64`, `cmd/7?` glob) | rename→`build_aarch64`, `objtype=aarch64` (D-05; dormant until Phase 2) |
| `build/scripts/linux-build:63-67` same fn | same rename |
| `build/mkdirs:264` `./arm64/bbin` | drop entry |
| `sys/src/BUILD_SETUP_NAMESPACE` (`mkdir -p /arm64; mount --bind $HARVEY/arm64 /arm64`) | **same commit as dir removal**: either drop the stanza (recommended — Phase 2 adds `/aarch64` wiring when the arch is real) or rename to `/aarch64` + add `./aarch64/bbin` to mkdirs |
| `dist/replica/plan9.proto:38-39` (`arm64 d775 sys sys` + `+`) | NEW finding — rename to `aarch64` (cheap, completes "tree speaks aarch64 only") or document-quarantine; planner's call |
| root `arm64/` (untracked, holds empty `bbin/`) | remove |
| Legitimate keepers: `nix/{setup,toolchain,build}.sh` + `flake.nix` (`uname -m` arm64→aarch64 host mapping), `nix/lima-harvey.yaml` (upstream Ubuntu image URL), `sys/include/{a.out.h,mach.h}` + `sys/src/cmd/unix/9cc/**` (vendored arm64 object definitions — Phase 2 libmach territory), `sys/src/libdynld/dynld-arm64.c` (pre-existing source), `AGENTS.md` | document as quarantined/explained in the plan |

### Pitfall 4: Two clean builds differ even with KERNDATE pinned
**What goes wrong:** (a) `conffile` absolute path embedded per Pattern 4; (b) the darwin path's rsync pushes host `amd64/bin` artifacts into the "clean" guest (vm-build.sh excludes `.git`/`.build` but **not** built output) [VERIFIED: nix/vm-build.sh:36-39].
**How to avoid:** policy doc states same-path bit-reproducibility + the conffile exception; the BUILD-04 demonstration defines "clean" precisely (fresh `git clone` or `git clean -fdx` + setup/toolchain/build) — note `git clean -fdx` also wipes `.build/hostbin-*`, so toolchain rebuild is part of the timed path.

### Pitfall 5: "Clean checkout" is ambiguous in the darwin path
Covered above — the rsync excludes make the guest tree *nearly* clean but carry host binaries. Planner: define the verification procedure explicitly.

### Pitfall 6: minimega mechanics that bite first-time users
- qemu starts **paused** (`-S`); forgetting `vm start` yields a silent zero-log VM.
- Daemon needs root (KVM + runtime dir); serial socket only exists after launch.
- `vm config kernel` on a non-ELF kernel → qemu multiboot magic failure; use the `.elf` variant.
- Booting amd64 under TCG on the aarch64 lima guest is 10–50× slower — minutes, not seconds; don't misread as a hang (CONCERNS.md performance trap, re-rated for this harness).

### Pitfall 7: Vendored-code failures are probably our toolchain (D-02 enforcement)
mahjongg is the documented case (Harvey cpp bug, Dockerfile:13). For every vendored failure: reproduce with reference cpp first; fix toolchain bugs in `sys/src/cmd/cpp`, not in the vendored source.

## Code Examples

### Fix jpg generators (Pattern 1 application)
```mk
# sys/src/cmd/jpg/mkfile — replace the $c^c idiom (lines 45, 61-69) with:
ycbcr.h:	rgbycc.c
	objtype=$cputype mk rgbycc.host
	./rgbycc.host > ycbcr.h

rgbycc.host:VQ:	rgbycc.c
	$CC $CFLAGS rgbycc.c
	$LD $LDFLAGS -o $target rgbycc.$O

# (and identically for rgbv.h/rgbrgbv.c; extend nuke-headers to rm *.host)
```
Shape only — executor matches mk's stem rules; the invariant is: **never derive a tool name from `/$cputype/mkfile`**.

### KERNDATE pin (BUILD-04)
```bash
# nix/build.sh (and vm-build.sh ssh line), before entering the namespace:
export KERNDATE="${KERNDATE:-$(git log -1 --format=%ct 2>/dev/null || date +%s)}"
```
```mk
# sys/src/9k/k10/mkfile:20 and sys/src/9k/mk/portmkfile ($CONF.$O rule):
$CC $CFLAGS '-DKERNDATE='`{if(~ $#KERNDATE 0) KERNDATE=`{date -n}; echo $KERNDATE} $CONF.c
```
Policy doc (suggest `build/REPRODUCIBILITY.md`): scope = 9k kernels; derivation = git commit timestamp; override = `KERNDATE=<epoch>` env; exceptions = `conffile` embeds the checkout path (same-path reproducibility only); verification = two clean builds, `sha256sum amd64/9k10cpu*` identical.

### minimega smoke harness (BUILD-05)
```
# build/mm/boot-9k10cpu.mm — run with: minimega -e read build/mm/boot-9k10cpu.mm
vm config memory 512
vm config kernel /path/to/amd64/9k10cpu.elf
vm config serial-ports 1
vm config backchannel false
# uncomment on hosts without KVM (TCG — slow):
# vm config qemu /nix/store/.../bin/qemu-system-x86_64
vm launch kvm k10cpu
vm start k10cpu
```
```bash
# capture + assert (wrapper script; daemon already running as root):
socat -u UNIX-CONNECT:/tmp/minimega/0/serial0 CREATE:boot-9k10cpu.log &
sleep <budget>; grep -q 'Plan 9' boot-9k10cpu.log
```
Banner assertion grounded in-tree: `print("\nPlan 9\n");` [VERIFIED: sys/src/9k/k10/main.c:184 region] and COM1 enabled unconditionally [VERIFIED: main.c:182 `i8250console("0");`].

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| expect+qemu ISO bootstrap (`build/build.bash`, 9legacy ISO download, prompt-matching) | Nix namespace build (`nix run .#build`, lima on darwin) | this fork, 2026-10 | ISO path stays dead (CONCERNS.md: supply-chain + EOL-Ubuntu rot); do not resurrect |
| Build-time `date -n` KERNDATE | SOURCE_DATE_EPOCH-style pin from git commit | this phase (BUILD-04) | Two clean builds bit-identical (same path); policy documented in-repo |
| Custom expect/qemu VM smoke | minimega `.mm` + serial-socket capture | this phase (BUILD-05) | Reusable harness for every later phase; no in-guest agent |

**Deprecated/outdated:**
- `build_arm64` functions / `arm64/` dir / mkdirs entry: vestigial scaffolding referencing never-committed `cmd/7?` dirs — quarantined by this phase (BUILD-03), real wiring lands in Phase 2 under `aarch64`.
- `build/Dockerfile` (ubuntu:21.10, EOL): not the supported path; nix flow supersedes. Its TODO comments remain valuable diagnostics (mahjongg/cpp).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | qemu `-kernel` multiboots `9k10cpu.elf` correctly (ELF entry lands on `_protected`; header within first 8KiB of image). Strong in-tree evidence (l32p.s:25-28 header; l32p first in object list; `-H5 -E0x00110000` ELF link) but not demonstrated this session. | Pattern 5 | Smoke harness fails at first boot; fallback = ISO boot or debugging the ELF entry — costs execution time, not design |
| A2 | minimega TCG fallback (`vm config qemu` → plain qemu-system-x86_64) boots the amd64 kernel acceptably on the aarch64 lima guest. Mechanism verified in source (no `-enable-kvm` emitted); performance untested. | Pattern 5, Env | If TCG is pathologically slow or minimega errors without KVM elsewhere, BUILD-05's demonstration needs a real KVM host (Open Question 1) |
| A3 | awk/spell/gs generators survive `cputype=unix` (awk/spell route through `objtype=$cputype mk` → 9c/9l; gs falls back to committed `default.amd64.h`). Pattern analysis + absence from the prior session's failure list; the tail past 4a/jpg is unexplored per D-04. | Pattern 1 | They appear as fresh whack-a-mole blockers — expected by design (D-04), low marginal cost |
| A4 | Beyond KERNDATE and `conffile`, no other non-determinism sources affect the three kernel binaries (prior-session k10cpu build exists, but no bit-identical pair has ever been produced). | Pattern 4 | BUILD-04's "two clean builds identical" demo uncovers more variance; policy doc's exception clause absorbs it if documented honestly |
| A5 | The current dirty tree (rc/bin/Kill, troff fonts, postscript edits, untracked generated `k10f.c`/`pccpuf.c`) is unrelated to Phase 1 failures. | Runtime State Inventory | If wrong, some "breakage" is local edits; the clean-checkout verification exposes it immediately |
| A6 | `vm config initrd` is unnecessary for k10cpu (root fs embedded via rootdir). CONTEXT's "kernel+initrd" phrasing is advisory. | Pattern 5 | None — executor adds an initrd line only if bring-up shows a need |

## Open Questions (RESOLVED)

1. **Where is the Linux KVM host for BUILD-05's letter-of-requirement demonstration?**
   - What we know: dev fleet = darwin-aarch64 host + linux-aarch64 lima guest with **no `/dev/kvm`** [verified by ssh probe 2026-10-30]; minimega is x86/KVM-centric and manages amd64 VMs only; TCG fallback exists (A2).
   - What's unclear: whether the user has (or will accept) an x86_64-linux KVM host / CI runner / nested-virt environment for the final demonstration — or accepts the TCG-degraded boot-log capture as this phase's proof with full-KVM verification deferred.
   - Recommendation: plan the harness with TCG as the development path + a `checkpoint:human-verify` (or user-provided host) for the KVM demonstration. The `.mm` script and capture mechanics are identical either way.
   - **Disposition (RESOLVED):** 01-04-PLAN.md — the wrapper develops against the TCG path (`MM_TCG`/`MM_QEMU`) and the blocking `checkpoint:human-verify` task ("Boot demonstration") puts the KVM-vs-TCG acceptance decision to the user.
2. **Is the 386 pass inside the green gate?** `linux-build` runs `arch=(386 amd64)` — build_386 compiles the whole userland with 8c *and* builds legacy `9/pc` kernels + pcboot **before** amd64 starts [VERIFIED: build/scripts/linux-build:131-155, 6-24]. Phase boundary says 386 legacy kernels are NOT in this phase.
   - Recommendation: narrow to `arch=(amd64)` this phase (documented, trivially reversible); otherwise the whack-a-mole tail roughly doubles and drags in the deferred 386/ISO milestone.
   - **Disposition (RESOLVED):** 01-02-PLAN.md Task 2 — `arch=(amd64)` with a revert comment; 386 pass deferred, trivially reversible.
3. **Do the dist/man stages belong to "the full namespace build"?** linux-build continues past the kernels into `/sys/lib/dist` (`mk cddisk` — ISO-adjacent) and man indexing [VERIFIED: build/scripts/linux-build:157-179]. Kernels are already produced by that point.
   - Recommendation: define green = through the kernel loop; gate dist/man behind a flag or split `linux-build`, leaving ISO/dist to its deferred milestone. Confirm against success criterion 1's wording with the user if the checker disagrees.
   - **Disposition (RESOLVED):** 01-02-PLAN.md Task 2 — dist/man stages gated behind `WITH_DIST=1`; the green gate ends at the kernel loop.
4. **Reproducibility doc location** — `build/REPRODUCIBILITY.md` suggested; planner may choose `docs/` or README. Cosmetic.
   - **Disposition (RESOLVED):** 01-03-PLAN.md Task 2 — policy doc lands at `build/REPRODUCIBILITY.md` as suggested.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| nix (flakes) | all apps | ✓ (host + guest) | Determinate Nix 3.21.0 / 2.34.6 host; 2.35.3 guest [verified by probe] | — |
| lima + `harvey` VM | darwin namespace build | ✓ (`~/.lima/harvey` present; CONTEXT says running) | ubuntu 24.04 aarch64 guest | native linux host |
| plan9port | host tools | ✓ via nixpkgs pin | 0-unstable-2025-11-10 | — |
| qemu (x86_64) | smoke harness | ✓ via flake devshell/apps (not on bare PATH — by design) | nixpkgs pin | — |
| minimega build (`nix build .#minimega`) | BUILD-05 | ✓ derivation committed; linux-only output | 3.3.0 pinned | — |
| **Linux KVM host (x86_64)** | BUILD-05 demonstration | **✗** — dev fleet has none (darwin-aarch64 host; lima guest linux-aarch64, `/dev/kvm` absent [ssh probe]); vmx/svm n/a | — | TCG via `vm config qemu` (slow); or user-provided host; or CI runner |
| git | KERNDATE derivation | ✓ host; ✗ in lima guest checkout (rsync `--exclude .git`) | — | compute host-side, export through ssh (Pattern 4) |
| KVM modules in guest | n/a | ✗ (no nesting on aarch64 lima) | — | — |

**Missing dependencies with no fallback:** none that block the amd64 build tail (BUILD-01..04).
**Missing dependencies with fallback:** Linux KVM host → TCG degraded mode (BUILD-05 development) with a checkpoint for the requirement's KVM wording.

## Security Domain

Build-tooling phase; ASVS categories mostly non-applicable, mapped honestly:

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | weakly | mkfile recipes quote/handle paths; no new parsers this phase |
| V6 Cryptography | no | — (milestone rule: **zero libsec/devtls changes** — TLS 1.2 cap, SSL3 floor, NULL/RC4 suites are documented debt for a later hardening milestone, not this phase) [VERIFIED: .planning/codebase/CONCERNS.md] |
| V14 Configuration / build pipeline (closest fit) | yes | build.sh's `sudo cp … /bin/{cpp,bind,rc,6a…}` mutates the host — pre-existing, idempotent, documented; vendored-code patches follow D-02 (toolchain-first); no new network fetches (minimega hash-pinned; go dir self-skips without `/bin/go` [VERIFIED: sys/src/cmd/go/mkfile:18-28]); the un-checksummed 9legacy ISO bootstrap stays dead |

| Threat Pattern | STRIDE | Standard Mitigation |
|----------------|--------|---------------------|
| Supply-chain via fetched artifacts | Tampering | minimega `fetchFromGitHub` pinned tag+hash, upstream `vendor/` (no registry fetches); nixpkgs pin via flake.lock |
| Build-time code execution (generators) | Elevation of Privilege | generators build via `9c/9l` into host binaries with no privilege; the only sudo surface remains build.sh's `/bin` copies (unchanged) |
| Vendored CVE-era code (gs, antiword) entering kernels | — | none of it is embedded in kernels (rootdir = factotum/ipconfig/fossil/venti/boot/nvram only); patching policy is D-02 |

## Sources

### Primary (HIGH confidence — direct reads this session)
- In-tree files (all read in full or at cited line ranges): `sys/src/cmd/mkfile`, `sys/src/cmd/{jpg,awk,4a,cc,spell,jtagfs,gs,go}/mkfile`, `sys/src/games/mkfile`, `unix/mkfile`, `sys/src/mkfile.proto`, `nix/{build.sh,vm-build.sh,setup.sh,toolchain.sh,minimega.nix,lima-harvey.yaml}`, `flake.nix`, `build/scripts/{linux-build,build,funcs}`, `build/mkdirs`, `sys/src/{BUILD,BUILDTOOLCHAIN,RUN_BUILD_IN_PRIVATE_NAMESPACE,BUILD_SETUP_NAMESPACE}`, `sys/src/9k/k10/{mkfile,k10cpu,k10cpuf,k10f,l32p.s,main.c,crap.c}`, `sys/src/9k/mk/{parse,mkrootall,mkroot,bootmkfile,portmkfile,mkenum}`, `sys/src/cmd/{BUILD,aux/BUILD,cpp/BUILD,6c/BUILD,unix/9cc/BUILD}`, `.gitignore`, `dist/replica/plan9.proto`, `build/Dockerfile`
- minimega upstream source at the pinned tag: `cmd/minimega/kvm.go` @ `3.3.0` (raw.githubusercontent.com) — qemuArgs construction, serial socket paths, baremetal mode, `-S` semantics, no `-enable-kvm`
- GitHub API: minimega tags (3.3.0 = latest, commit 7157b8e6…)
- Environment probes: ssh into lima guest (uname, /dev/kvm, cpuinfo), host nix/limactl/qemu presence, git status/ls-files
- Prior decoded research: `.planning/research/PITFALLS.md` (Pitfalls 1, 2, 14 + grep lists), `.planning/research/SUMMARY.md`, `.planning/codebase/CONCERNS.md`

### Secondary (MEDIUM confidence)
- minimega official docs (sandia.gov/minimega Module 8: `vm config kernel/initrd` fields, KVM-module failure mode; Module 18: `.mm` files, `minimega -e`, `read`, daemon `-nostdin`)
- reproducible-builds.org SOURCE_DATE_EPOCH spec (definition, git-commit derivation convention)

### Tertiary (LOW confidence)
- none load-bearing; assumptions A1-A6 logged above

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; minimega pin verified against GitHub API; all build machinery read directly
- Architecture: HIGH — the pipeline diagram and every pattern were assembled from full-file reads of the actual mkfiles/scripts this session
- Pitfalls: HIGH for in-tree mechanics (generator idioms, rootdir chain, arm64 coupling); MEDIUM for harness runtime behavior (A1/A2 untested end-to-end)

**Research date:** 2026-10-30
**Valid until:** 2026-12-30 (stable domain; the only fast-moving input is the nixpkgs pin, which is locked in flake.lock)

**Nyquist validation:** skipped — `.planning/config.json` sets `workflow.nyquist_validation: false`.

---
*Phase 1 research — Harvey OS build revival*
