# Phase 2: aarch64 Toolchain + libc (9front donor) - Research

**Researched:** 2026-10-31
**Domain:** Plan 9 toolchain porting — 9front `7a`/`7c`/`7l` + libmach arm64 + `libc/aarch64` vendoring, host-built on darwin-aarch64/linux-aarch64, proven by execution on a 9front donor kernel under qemu `-M virt`
**Confidence:** HIGH — the central risk (does 9front's 7c/7a/7l compile against Harvey's frontend and plan9port host build?) was **burned down empirically this session**: all three tools were trial-built and run on darwin-aarch64, and the complete shim set is enumerated below with file:line precision.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### ISA identity (clarified up front — the arm64/aarch64 conflation)
- **D-01:** `arm64` ≡ `aarch64` — two names for the same AArch64/ARMv8-A ISA. 9front's arm64 imports **are** the aarch64 toolchain; no custom "aarch64 target" exists or is needed (rolling our own = Pitfall 9's multi-month silent-miscompilation detour for zero ISA benefit). The project's `aarch64` label is a tree-naming choice only (D-06 rename-on-import, carried forward). A Cortex-A72 (RPi4) and an Apple M5 Max both execute the same AArch64 userspace code 7c emits. — **Reversibility:** one-way — every import, stub, and the Phase 4 ABI freeze builds on this identity; undoing it means re-deriving a toolchain.
- **D-02:** Execution loci, pinned: (a) host tools (`7a`/`7c`/`7l` themselves) compile via plan9port `9c` into native macOS arm64 / linux aarch64 binaries — the Phase 1-proven pattern; (b) guest code runs on qemu `-M virt` via **HVF on the M5 Max** = native execution of guest AArch64 on Apple silicon (virtualization, not emulation); TCG is the fallback and the only path on x86_64-linux. Bare-metal Apple silicon confirmed out of scope.
- **D-03:** Forward-notes to Phase 3 (kernel territory, not Phase 2 blockers): Apple host 16K pages constrain HVF guest granule choices (9front arm64's 64K-granule preference will not transfer); r9's cache maintenance must use architectural discovery (`CTR_EL0`), not hardcoded Apple geometry.

#### Alpha letter-7 resolution (TC-01)
- **D-04:** Hard-delete Alpha from libmach, following 9front exactly: remove `sys/src/libmach/alpha/` (`7.out.h`, `ureg.h`), `alpha.c`, `alphadb.c`, `alphaobj.c` (+ committed stale `.6`/`.8` artifacts), the `setmach.c:121` `"alpha"` entry, and `libmach/mkfile` wiring (lines 22/33/42/69). Git history is the attic — the code is 13 years dead (last touched 2013), already marked `/* retired */`, and no Alpha compiler/libc/kernel exists in Harvey to use it. The vendored 9-cc's own `mach.h` copy (`sys/src/cmd/unix/9cc/include/mach.h`) is independent and unaffected.
- **D-05:** Collision-only excision: also remove the five Alpha enums in `sys/include/mach.h` (`MALPHA`, `FALPHA`, `FALPHAB`, `AALPHA`, `ObjAlpha`). Other retired arches (MIPS2, 29000, SPARC64) stay untouched — sweeping them is the hygiene-phase work Phase 1 deferred. `mach.h` already carries `MARM64`/`FARM64`/`FARM64B`/`AARM64`/`ObjArm64` — the enums are waiting; letter 7 was the blocker.

#### Hello-world proof standard (TC-03 gate)
- **D-06:** The proof is **execution**: our 7c/7l hello-world runs on 9front's own arm64 kernel under qemu `-M virt` via HVF on the M5 Max — validating codegen, libc asm, and the syscall ABI against the reference kernel. This realizes Phase 1's "boot-test, not build-test" lesson (build-green masked LP64 miscompilation; only execution exposed it) and gives Phase 4 a known-good ABI bisect point: runs on 9front kernel but not r9 ⇒ bug is in r9. (Roadmap SC#3's "qemu user-mode" example is impossible — qemu user-mode has no Plan 9 emulation; this decision supersedes that wording.)
- **D-07:** The donor kernel is **built with our imported 7a/7c/7l** from the pinned 9front checkout (`CONF=qemu` reference kernel), built **in-donor-tree** — the 9front kernel never enters Harvey's tree; the pinned checkout is harness scaffolding. Fallback to a pinned prebuilt 9front image, if the donor-kernel build proves intractable, is a **checkpoint decision with the user** — never a silent downgrade of the proof standard.
- **D-08:** The harness doubles as the **TC-05 acceptance gate**: at phase exit the shell subset (rc, ipconfig, factotum, core cmds — all built by our 7c) executes on the donor kernel, not merely links. TC-05 gets execution evidence a full phase before Phase 5 needs it.
- **D-09:** Harness constraint with a verification: `NMOUNT=54` is Harvey-only — binaries calling `nmount()` die on the 9front kernel. Verified this session that none of rc/ipconfig/factotum call it (Harvey's `boot` does, but the donor kernel runs 9front's own boot). Phase exit check: `nm` the shell-subset binaries, assert zero `NMOUNT` references.

#### Import mechanics (bridge-aware shrunk scope)
- **D-10:** One-shot import + per-dir provenance READMEs (the `sys/src/cmd/unix/9cc/` precedent); the 9front pin SHA is recorded in the harness docs. User framing, quoted: *"our goal is using the r9 kernel, so this 9front stuff is a temporary bridge."* No repeatable import script — the bridge doesn't warrant machinery.
- **D-11:** Permanent import set only: `7a`/`7c`/`7l` → `sys/src/cmd/`, libmach `7.c`/`7db.c`/`7obj.c` + setmach entry, `libc/arm64` → `sys/src/libc/aarch64`, `arm64/include/{u.h,ureg.h}` → `aarch64/include/`, objtype wiring (`aarch64/mkfile` CC=7c LD=7l AS=7a O=7, `pcc.c` entry, `mkfile.proto` CPUS — CPUS lands **last**, after the gate), and `case aarch64` in the 9syscall stub generator. All per D-06 rename-on-import: `arm64` paths normalize to `aarch64`, tool letters stay `7a/7c/7l`, donor file contents otherwise untouched (path rewrites inside files are part of normalization — e.g. 9front `7.c`'s `#include "/arm64/include/ureg.h"` becomes `/aarch64/include/ureg.h`).
- **D-12:** `aout2uimage` is **not imported** into Harvey. It wraps a Plan 9 a.out *kernel* as a uImage; r9 is Rust/ELF and Harvey builds no aarch64 C kernel — TC-05's "if the boot path needs it" resolves to: it doesn't. The donor tree's own copy serves the D-07 harness build.

#### Donor fidelity
- **D-13:** Adapt 7c/7l to **Harvey's shared cc frontend** with minimal local shims (the `compat.h`/`mkfile.port` expectations). Do NOT sync 9front's cc pieces: Harvey's `cc.h` carries Phase 1's `NVAR=BITS*32` LP64 fix that 6c/8c/5c all depend on — touching the shared frontend risks the amd64 toolchain green Phase 1 just proved boot-clean. Drift reconciled file-by-file, toolchain-first diagnostic rule (Phase 1 D-02) applies when vendored code fails.
- **D-14:** Full 9front `libc/arm64` asm set imported verbatim (memmove/memset/atom/atomic/atomic64/cycles/fabs/floor/sqrt/setjmp/tas/main9/main9p/notejmp/getcallerpc/getfcr/argv0/_seek). Verified-working ABI + codegen-tested asm; with D-06's execution proof, more verbatim asm = stronger proof. 9front-only API drift reconciled against Harvey's `sys/include/libc.h` as it surfaces (research rates this MEDIUM — not yet diffed file-by-file).

#### Syscall ABI freeze (TC-03/TC-05 freeze; R9-04 lead-in)
- **D-15:** Freeze the **full Harvey `sys.h`** (0–54, NMOUNT included) as the aarch64 user-side table, stubs generated with the **9front arm64 convention**: syscall number in `R0`, args in 8-byte slots at user `SP+0`, `SVC`, result in `R0` — matching the roadmap Phase 4 wording ("args at SP_EL0+0, number in R0, result in R0"). The `case aarch64` generator replicates the per-name quirks from existing cases (`seek`/`nsec` vlong buffer stores on `-1`, `_exits` naming). — **Reversibility:** one-way — this is the published contract Phase 4 builds r9's `svc` dispatch and static trap-frame checks against; changing it after Phase 4 means re-freezing both repos.
- **D-16:** Table diff vs 9front, verified this session against 9front `front` branch: **0–52 identical** (names and numbers, incl. `SYSR1=0` and the gap at 48–49); **53 same wire, renamed** (Harvey `NSEC` vs 9front `_NSEC` — researcher confirms buffer-arg semantics match); **54 `NMOUNT` Harvey-only** (`nmount(fd, afd, old, flag, aname, dev)` — 6th arg mounts a specific device channel; Harvey's 5-arg `mount()` = `MOUNT=46` matches 9front exactly). r9 implements the table lazily in Phase 4 — NMOUNT may stay unimplemented until something calls it.
- **D-17:** Kernel-side dispatch style does **not** cross the wire: Harvey 9k's generated `systab[]` handlers are `void sysxxx(Ar0*, va_list)`; 9front's are `void sysxxx(Ar0 ar0)` — internal C style only. r9 mimics neither; it matches the wire contract (R0/stack slots/result) plus the frozen `aarch64/include/ureg.h` trap frame, verified by Phase 4's build-time static checks.

#### Carried forward from Phase 1 (locked there, restated so downstream doesn't re-litigate)
- D-06 rename-on-import and the riscv64 vaporware gate: hello-world **executes** (per D-06 above) before any `aarch64/` arch dir, libc dir, or CPUS entry lands.
- Vendor-and-own: in-tree patches allowed; toolchain-first diagnostic rule (vendored code that built for decades and now fails → suspect our cpp/compiler first).
- plan9port `$PLAN9/bin` LAST in PATH; per-OS/arch hostbin dirs (`.build/hostbin-<os>-aarch64`); host-tool BUILD-script pattern (model on `sys/src/cmd/6c/BUILD`, `6a/BUILD`).
- Harvey links statically — 7l's `dyn.c` is droppable per research.
- minimega is x86/KVM-only upstream — aarch64 boots (incl. the donor-kernel harness) use plain qemu.

### the agent's Discretion
- 9P delivery mechanism for getting our binaries into the running donor kernel (ufs-style host 9P server vs image baking) — planner/researcher picks; the Phase 1 `ufs` lineage is the obvious starting point.
- Per-dir NOTICE/README wording for provenance — follow research STACK.md's license action (credit Bell Labs/Lucent, Vita Nuova, C H Forsyth, 9front) and the `unix/9cc/README` shape.
- Whether `7c` joins `sys/src/cmd/cc/mkfile`'s `CURCC` list — only after it builds (Pitfall 9); timing is planner's call.
- libc mem*/str* asm-vs-C for any file 9front's set doesn't cover — portable C first (riscv64 pattern), asm later.

### Deferred Ideas (OUT OF SCOPE)
- Broad skeletal-port attic/quarantine (riscv64 vaporware tree, arm/mips/power fragments, other retired `mach.h` arches MIPS2/29000/SPARC64) — later hygiene phase (Phase 1 deferral, reconfirmed by D-05)
- `aout2uimage` import into Harvey — revisit only if a future boot path needs uImage wrapping of a Plan 9 a.out kernel (r9 endgame is ELF/Rust) — D-12
- Bare-metal Apple silicon bring-up (m1n1 boot chain, Apple AIC instead of GIC, no firmware DTB — Asahi-scale problem) — user considered and set aside; v2+ at earliest, qemu virt via HVF is the target (D-02)
- Go programs in the aarch64 image — Go unsupported on plan9/arm64 (already Out of Scope in REQUIREMENTS.md)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TC-01 | libmach learns arm64 (`7.c`, `7db.c`, `7obj.c`, setmach entry); retired-Alpha-under-letter-7 collision resolved | Full import/excision map verified this session: donor files sized (`7.c` 152 lines, `7db.c` 800, `7obj.c` 153), Alpha sites enumerated verbatim (§Architecture Patterns → libmach arm64), plus **two additional required sites beyond D-11's list**: `executable.c` (crackhdr exectab entry — nothing can identify a 7l binary without it) and `obj.c` (`[ObjArm64]` table line). mach.h compatibility proven (donor files don't use the `Map.segment.read` member Harvey lacks). |
| TC-02 | `7a`/`7c`/`7l` vendored with BUILD scripts; build natively on darwin-aarch64 and linux-aarch64; `compat.h` frontend drift reconciled | **Proven by trial build this session** (darwin-aarch64, plan9port 9c/9l): all three tools compile, link, and function (7c codegen verified on fib/shift/FP smoke test; 7a assembled 7c's `-S` output; 7l parsed objects and emitted correct diagnostics). The complete drift/shim set is 9 items, all enumerated in §Code Examples. The `compat.h` answer is a NEW additive header `sys/src/cmd/cc/compat.h` (9front has it; Harvey doesn't) — zero edits to existing shared files. |
| TC-03 | `sys/src/libc/aarch64/` imported + `9syscall` `case aarch64`; hello-world executes before any scaffolding | Donor libc file set verified verbatim (20 files). One non-obvious dependency found: 9front `main9.s` branches to `_callmain`, which lives in 9front's `libc/9sys/callmain.c` — **Harvey lacks it**; import it too (additive, inert on amd64). The `case aarch64` generator shape is verified verbatim from 9front; wire semantics re-verified from 9front kernel trap.c (one wording correction to D-15: kernel-visible args begin at user **SP+8**, not SP+0 — SP+0 is the reserved LR slot). Gate order per CONTEXT: tools → libc compiles → hello links → **executes on donor kernel** → only then arch dirs/CPUS. |
| TC-04 | `aarch64/mkfile` (CC=7c O=7), `pcc.c` objtype entry, `mkfile.proto` CPUS, config section literally named `aarch64` | All sites verified in-tree this session: `aarch64/mkfile` content is byte-pattern known (9front's 5-line file, rename applied); pcc.c needs BOTH the table entry AND `'7'` added to `allos`; CPUS is `sys/src/mkfile.proto:6`; `build/mkdirs` needs `./aarch64/bbin` re-added. "Config section literally named aarch64" is ambiguous — most plausible reading is the `arch=(...)` lists + `build_$i` dispatch in `build/scripts/{funcs,linux-build,build}` (the dormant `build_aarch64` is already correctly named); see Open Question 1. |
| TC-05 | Shell-boot userland subset (rc, ipconfig, factotum, core cmds) builds for aarch64 | Dependency check done: rc needs only libc; ipconfig only libc+libip (portable C); factotum needs libauth+libauthsrv (portable C). All portable — no per-arch asm beyond libc. The D-08 execution gate runs them on the donor kernel. D-09's NMOUNT check needs a **host `nm`** against the new libmach (no host nm exists today — new BUILD task, §Architecture Patterns). |
</phase_requirements>

## Summary

This phase vendors 9front's arm64 toolchain (`7a`/`7c`/`7l`), libmach arm64 support, and `libc/arm64` (renamed to `aarch64` on import) into Harvey, wires `objtype=aarch64`, and proves the result with binaries that **execute** on a 9front donor kernel under qemu `-M virt`. The donor was cloned and pinned this session: `github.com/9front/9front` branch `front` @ **`350266104f1fed81c78c3d05fa44f523bec0ae4d`** (2026-10-31).

The headline finding: **the central risk is already retired**. Research trial-built all three tools on this darwin-aarch64 host using Harvey's existing Phase 1 machinery (plan9port `9c`/`9l`, Harvey's shared `cc` frontend with its LP64 fixes). `7c` compiles C to arm64 assembly correctly (recursion, 64-bit shifts, FP constants verified in `-S` output); `7a` assembles `7c`'s own `-S` output (dialect agreement); `7l` parses the objects and emits correct ken-linker diagnostics. The entire 9front→Harvey drift surface is **9 small shims**, all in vendored files or one new additive header — Harvey's shared `cc.h`/`sub.c`/etc. stay untouched per D-13. Two drift items deserve planner attention: 9front's rotate-fold optimization (`OROL` node op + `rolor()` folder) has no counterpart in Harvey's frontend and must be neutralized in the vendored `7c` (3 edit sites, zero functional loss), and 9front's `libc/arm64/main9.s` depends on `_callmain` from `libc/9sys/callmain.c`, which Harvey lacks (additive import, inert for amd64).

The syscall ABI freeze (D-15) is verified against both trees and one wording correction is needed: the 9front arm64 kernel reads syscall args at `ureg->sp+BY2WD` (BY2WD=8), i.e. user-visible args begin at **SP+8** — SP+0 is the reserved link-register slot; the stub materializes arg0 there from R0 before loading the syscall number into R0. The Harvey table (SYSR1=0 … NMOUNT=54) is wire-identical to 9front 0–52; NSEC=53 is zero-arg register-return on 64-bit in both trees (9front's buffer path is 32-bit compat only); NMOUNT=54 is Harvey-only and must stay uncalled on the donor kernel (D-09 nm check).

The harness has one **verified hardware constraint that overrides D-02/CONTEXT's suggested qemu line**: qemu 11.1.2 on this Mac reports `HVF does not support GICv2 emulation` and rejects `-cpu cortex-a72` under HVF (valid HVF CPUs: cortex-a53, cortex-a57, host, max). The 9front `CONF=qemu` arm64 kernel is a **GICv2-only** guest (its `arm64/gic.c` uses the v2-only `GICD_TARGETSR0` register bank; 9front's GICv3 drivers live in `imx8/`/`lx2k/` and are not wired into the qemu CONF). Therefore the donor-kernel harness on the M5 Max runs under **TCG** (`-accel tcg -cpu cortex-a72 -M virt,gic-version=2`), not HVF. TCG boot-to-shell on an M5 Max is slow but adequate for a smoke gate, and Phase 1 already accepted TCG-degraded serial capture as a phase demonstration (2026-10-31 decision). HVF returns when the guest is r9 and speaks GICv3 — a forward-note for Phase 3.

**Primary recommendation:** Execute in strict gate order — (1) Alpha excision + libmach arm64 + host `nm` (TC-01); (2) vendor 7a/7c/7l with the enumerated shim set + BUILD scripts into BUILDTOOLCHAIN (TC-02); (3) import `libc/aarch64` + `callmain.c` + `case aarch64` generator, compile libc, link hello-world; (4) build the donor kernel in-donor-tree with our tools (lima VM namespace), boot under TCG, execute hello-world (TC-03 gate); (5) only then land `aarch64/mkfile`, `aarch64/include/`, pcc.c, mkdirs, and CPUS **last** (TC-04); (6) build + execute the shell subset on the donor kernel (TC-05), asserting zero NMOUNT references with the host nm.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Host toolchain binaries (`7a`/`7c`/`7l` as macOS/Linux executables) | Host build env (nix devshell, plan9port 9c/9l) | — | Phase 1 pattern: BUILD scripts → `.build/hostbin-<os>-aarch64/bin` |
| Target codegen (`.7` objects, `7.out` executables) | Cross: host tools emitting aarch64 Plan 9 a.out | — | ken-suite cross-compilers are host-native binaries emitting target code |
| Object/executable introspection (`nm`, `acid`, `crackhdr`) | Host (libmach + nm/acid host-built) | Target (same sources compile for aarch64 later) | D-09's gate needs nm on the host; libmach is shared source |
| Syscall stub generation | Build-time codegen (9syscall mkfile `case aarch64`) | — | Runs wherever mk runs; output is target asm |
| libc aarch64 (asm + C) | Target | Host (nothing — libc is target-only) | Compiled by 7c, assembled by 7a, archived by iar into `libc.a` |
| Reference execution (hello-world, shell subset) | qemu `-M virt` guest running 9front donor kernel | Host delivers binaries (bootfs bake or 9P/ufs) | D-06/D-07/D-08 — execution is the proof standard |
| Donor kernel build | lima VM Linux namespace build over the pinned 9front checkout | — | 9front mkfiles need Plan 9 namespace semantics (`/$objtype/mkfile`); Harvey's namespace-build pattern applies to the donor tree |
| Syscall ABI contract (frozen table + ureg.h) | This phase (user side, in-tree) | r9 repo Phase 4 (kernel side) | D-15/D-17: freeze here; r9 matches it there |

## Project Constraints (from AGENTS.md)

Actionable directives the plan must honor:

- **Naming:** the architecture is `aarch64` (not `arm64`) in this project's tooling and docs — rename-on-import applies to every donor path (`libc/arm64` → `libc/aarch64`, `arm64/include` → `aarch64/include`, `/arm64/include/ureg.h` → `/aarch64/include/ureg.h` inside files). Tool letters stay `7a`/`7c`/`7l`.
- **Style:** Plan 9 C dialect per `sys/man/6/style` — tabs only, block comments only (no new `//`), lowercase names, `nil` not `NULL`, `exits`/`sysfatal` not `exit`, `-1`+errstr error idiom, no `#ifndef` header guards in Plan 9 headers, no auto-init of automatic variables. Vendored 9front files keep their content per vendor-and-own (in-tree patches allowed and expected for the shims below).
- **Build:** `mk` everywhere for target builds (`mkfile` + `</$objtype/mkfile` + `mkone`/`mkmany`/`mksyslib` includes); host tools via BUILD scripts modeled on `sys/src/cmd/6c/BUILD`, `6a/BUILD`, `6l/BUILD`; plan9port `$PLAN9/bin` LAST in PATH; per-OS/arch hostbin dirs (`.build/hostbin-<os>-<arch>/bin` via `HARVEY_LINUX_BIN`).
- **Host platforms:** darwin-aarch64 (primary) and linux-aarch64 (lima VM) — both must build the tools; the namespace build is Linux-only.
- **Upstream posture:** this repo is the maintained fork; in-tree fixes preferred over external patch stacks.
- **Licenses:** Harvey MIT; 9front imports MIT-compatible — retain provenance NOTICE/README per imported dir (Bell Labs/Lucent, Vita Nuova, C H Forsyth, 9front) per the `sys/src/cmd/unix/9cc/README` precedent.
- **Toolchain diagnostic rule (Phase 1 D-02):** vendored code that built for decades and now fails → suspect our cpp/compiler/host shims first.
- **GSD workflow:** edits go through GSD commands; no direct repo edits outside a workflow.

## Standard Stack

### Core (all donor-verified this session)

| Component | Version/Pin | Purpose | Why Standard |
|-----------|-------------|---------|--------------|
| 9front `sys/src/cmd/7c` | branch `front` @ `35026610` | aarch64 C compiler | The only maintained Plan 9 arm64 compiler [VERIFIED: donor checkout this session] |
| 9front `sys/src/cmd/7a` | same pin | aarch64 assembler | Companion; `a.y`+`lex.c`+`a.h`, same shape as Harvey's 6a [VERIFIED] |
| 9front `sys/src/cmd/7l` | same pin | aarch64 linker → Plan 9 a.out (`R_MAGIC`) | Emits a.out only (no ELF path in sources); `R_MAGIC = _MAGIC(HDR_MAGIC, 28)` already present in Harvey's `sys/include/a.out.h:33` [VERIFIED: both files read this session] |
| 9front `sys/src/libc/arm64/` | same pin | per-arch libc (asm ABI set) | 20 files, verified verbatim [VERIFIED: donor dir listing + file reads] |
| 9front `arm64/include/{u.h,ureg.h}` | same pin | per-arch types + trap frame | ureg.h quoted verbatim in §Code Examples [VERIFIED] |
| 9front `sys/src/libmach/{7.c,7db.c,7obj.c}` + setmach entry | same pin | nm/acid/obj support for 7-format | Sizes: 152/800/153 lines [VERIFIED] |
| 9front `sys/src/libc/9sys/callmain.c` | same pin | `_callmain` startup helper required by arm64 `main9.s` | Harvey lacks it; additive import, inert on amd64 [VERIFIED: both trees read] |
| 9front `sys/src/9/arm64/` `CONF=qemu` | same pin | donor kernel (harness scaffolding, **never enters Harvey's tree** — D-07) | GICv2-only guest ⇒ TCG (see Summary) [VERIFIED: gic.c read] |

### Supporting (already in Harvey's stack — no new packages)

| Tool | Role | Notes |
|------|------|-------|
| plan9port (nixpkgs `plan9port-0-unstable-2025-11-10`) | host `mk`, `rc`, `9c`/`9l`, `yacc`, `ed` consumers | [VERIFIED: devshell probe this session] |
| qemu 11.1.2 (nix flake `pkgs.qemu`) | `qemu-system-aarch64` system emulation | hvf + tcg both present; HVF constraints probed (Summary) [VERIFIED: probe] |
| lima VM `harvey` (vz, aarch64, 4 CPU, 8 GiB) | Linux namespace builds (Harvey + donor trees) | Status: **Running** this session [VERIFIED: `limactl list`] |
| vendored 9-cc `iar` (`.build/hostbin-*/bin/ar`) | Plan 9 archiver for `libc.a` | Phase 1 asset; 7-format archives are fine (ar doesn't parse objects) |
| host `ed` (`/bin/ed`) | `mkenam`/`mkcname` generators | Both ran correctly under BSD ed this session [VERIFIED: trials] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| 9front 7a/7c/7l | 0intro/9-cc `src/cmd/{7a,7c,7l}` (pre-ported to unix) | Plan B if host-build shims ballooned — they didn't (9 small shims, proven). 9-cc's libmach lacks 7-object support, so libmach must come from 9front regardless. [CITED: .planning/research/STACK.md] |
| Neutralizing OROL in vendored 7c | Port `rolor`+`OROL` into Harvey's shared cc.h/sub.c | Rejected by D-13 (frontend protection). Restoring rotate-fusion is a hygiene-phase optimization; correctness unaffected. |
| Donor kernel built by our tools (D-07) | Pinned prebuilt 9front arm64 image | Fallback only, as a **user checkpoint** — D-07 forbids silent downgrade. [ASSUMED] availability (Phase 1 research rated MEDIUM from 9front community sources; not re-verified this session). |
| Bootfs baking for binary delivery | ufs 9P server over virtio-net + guest ipconfig | Both viable on the donor CONF (it has `ethervirtio10` + ip stack). Baking needs zero guest config and is deterministic for the gate; ufs iterates faster. Research recommendation: **bake for the D-06/D-08 gates**, document ufs as the iteration path. See Open Question 2. |

**Installation:** no package installation — donor import is `git` source vendoring at the recorded pin; host tools come from the existing nix flake.

**Version verification:** donor pin verified by `git clone --depth 1 --branch front https://github.com/9front/9front.git` → `350266104f1fed81c78c3d05fa44f523bec0ae4d` (2026-10-31). qemu/plan9port versions verified by running them in the devshell this session (qemu 11.1.2; plan9port 0-unstable-2025-11-10).

## Package Legitimacy Audit

This phase installs **no registry packages** (npm/PyPI/crates). All external inputs are source vendoring or existing nixpkgs pins:

| Input | Source | Pin | Verification | Verdict |
|-------|--------|-----|--------------|---------|
| 9front donor tree | `github.com/9front/9front` (mirror of canonical `git.9front.org/plan9front/plan9front`) | `350266104f1fed81c78c3d05fa44f523bec0ae4d` (branch `front`, 2026-10-31) | Cloned and read this session; the decades-old canonical Plan 9 fork | OK |
| plan9port | nixpkgs via `flake.lock` | `0-unstable-2025-11-10` | Ran in devshell this session | OK (existing Phase 1 dependency) |
| qemu | nixpkgs via `flake.lock` | 11.1.2 | Ran in devshell this session | OK (existing devshell dependency) |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram (data flow for the phase's deliverables)

```
Host (darwin-aarch64 / lima linux-aarch64, nix devshell)
│
│  BUILDTOOLCHAIN + BUILD scripts (plan9port 9c/9l, yacc, ed)
▼
sys/src/cmd/{7a,7c,7l}  ──►  .build/hostbin-<os>-aarch64/bin/{7a,7c,7l}
│                                   │
│  (namespace build, lima VM)       │ host tools on PATH
▼                                   ▼
sys/src/libc/{9sys,9syscall,fmt,port,aarch64}  ──7c/7a/iar──►  /aarch64/lib/libc.a
│                                                              │
│  cmd tree (objtype=aarch64)                                  ▼
└────────────────────────────────────────────────────────►  7.out binaries (R_MAGIC)
                                                              │
                                    delivery (bake into donor bootfs.paq | ufs over tcp)
                                                              ▼
Donor tree (pinned 9front @ 35026610, NEVER in Harvey's repo)
  CONF=qemu kernel built in-donor-tree BY OUR 7a/7c/7l  ──►  9qemu + aout2uimage ──► 9qemu.u
                                                              │
                                                              ▼
                                          qemu-system-aarch64 -M virt,gic-version=2
                                            -cpu cortex-a72 -accel tcg  (HVF impossible:
                                             donor is GICv2-only — verified)
                                                              │
                                                              ▼
                                          hello-world executes; shell subset executes (D-06/D-08)
                                          nm asserts zero NMOUNT refs (D-09)
```

### Recommended Project Structure (import targets)

```
sys/src/cmd/7a/          # vendored 9front 7a + shims + BUILD + mkfile (Harvey idiom) + README (provenance)
sys/src/cmd/7c/          # vendored 9front 7c + shims (math.h shadow, gc.h swcmp, OROL excision) + BUILD + mkfile + README
sys/src/cmd/7l/          # vendored 9front 7l + shims (l.h MAXIO, unixcompat.c) + BUILD + mkfile + README
sys/src/cmd/cc/compat.h  # NEW additive header (see Code Examples) — the only shared-frontend-side change
sys/src/libmach/{7.c,7db.c,7obj.c}   # vendored; setmach.c/mach.h/mkfile/obj.c/executable.c edited
sys/src/libc/aarch64/    # vendored 9front libc/arm64 verbatim except mkfile objtype line; + README
sys/src/libc/9sys/callmain.c         # vendored from 9front 9sys (additive)
sys/src/libc/9syscall/mkfile         # gains `case aarch64`
aarch64/mkfile                       # 5-line objtype wiring (post-gate)
aarch64/include/{u.h,ureg.h}         # vendored (post-gate; ureg.h is the ABI freeze)
```

### Pattern 1: Host-tool BUILD script (Phase 1 pattern, proven for 7a/7c/7l this session)

**What:** Compile Plan 9 tool sources with plan9port `9c`, link with `9l`, install to `$HARVEY_LINUX_BIN`.
**When:** 7a/7c/7l host builds (TC-02), and the new host `nm` (and optionally `acid`).
**Verified skeleton** (the exact flag set that built 7c clean this session — modeled on `sys/src/cmd/6c/BUILD`):

```bash
# Source: trial build this session, mirroring sys/src/cmd/6c/BUILD [VERIFIED: build transcript]
rm -f *.o
(cd ../cc; $PLAN9/bin/yacc -D1 -d cc.y)
9c -g -I . -I ../cc -O0 \
   -Wno-parentheses -Wno-missing-braces -Wno-switch -Wno-comment \
   -Wno-sign-compare -Wno-unknown-pragmas -Wno-misleading-indentation \
   -Wno-stringop-truncation -Wno-stringop-overflow -Wno-format-truncation \
   -fno-omit-frame-pointer -fsigned-char -fcommon -Wno-char-subscripts \
   -Wno-builtin-declaration-mismatch -Wno-implicit-function-declaration \
   -Wno-int-conversion -Wno-maybe-uninitialized -Wno-incompatible-pointer-types \
   -DRunemask=0x1FFFFF \
   ../cc/acid.c ../cc/bits.c ../cc/com64.c ../cc/com.c ../6l/unixcompat.c \
   ../cc/dcl.c ../cc/dpchk.c ../cc/funct.c ../cc/lex.c ../cc/mac.c ../cc/pgen.c \
   ../cc/pickle.c ../cc/pswt.c ../cc/scon.c ../cc/sub.c ../cc/y.tab.c \
   cgen.c enam.c list.c machcap.c mul.c peep.c reg.c sgen.c swt.c txt.c \
  && 9l -o 7c -g *.o
cp 7c $HARVEY_LINUX_BIN
```

Notes: do **not** pass `-static` (6c/BUILD's `-static` is already neutralized by the `9l` shim in `nix/toolchain.sh`); `../6l/unixcompat.c` supplies the hunk-allocator malloc family for the compiler build. Wire the new BUILDs into `sys/src/BUILDTOOLCHAIN` (which today runs 6a/6c/6l/8a/8c/8l/cpp/aux — verified this session at lines 1–9).

### Pattern 2: The nine verified shims (the complete D-13 drift surface)

Every shim below was discovered by trial compilation this session and each is quoted with its evidence. All edits land in **vendored files** or **one new additive header** — Harvey's existing shared frontend files (`cc.h`, `sub.c`, `com.c`, …) are untouched per D-13.

1. **NEW `sys/src/cmd/cc/compat.h`** — 9front has this header (7a/a.h and 7l/l.h include `../cc/compat.h`); Harvey doesn't. Add it declaring only what Harvey's `cc/compat` file + `lexbody`/`macbody` already define. Verified content (matches Harvey's `compat` definitions — note `mysbrk(ulong)`, getting this wrong is a hard error):

```c
/* Source: synthesized this session from 9front cc/compat.h + Harvey cc/compat defs
   [VERIFIED: trial build — 7a/7l compile with exactly this] */
#ifndef	EXTERN
#define EXTERN	extern
#endif

enum
{
	Plan9	= 1<<0,
	Unix	= 1<<1,
	Windows	= 1<<2
};
EXTERN	int	systemtype(int);
EXTERN	int	pathchar(void);

EXTERN	int	myaccess(char*);
EXTERN	void*	mysbrk(ulong);
EXTERN	int	mywait(int*);
EXTERN	int	mycreat(char*, int);
EXTERN	char*	mygetwd(char*, int);
EXTERN	int	myexec(char*, char*[]);
EXTERN	int	mydup(int, int);
EXTERN	int	myfork(void);
EXTERN	int	mypipe(int*);

EXTERN	void*	alloc(long n);
EXTERN	void*	allocn(void *p, long on, long n);
```

2. **`7c/math.h`** — two-line SDK shadow, copied from Harvey's `6c/math.h` (verbatim: `// This is ONLY here for building on unix.` + `double modf(double, double *);`). Without it, plan9port's `u.h:57` (`#include <math.h>`) pulls the macOS SDK math.h, whose `round(double)`/`log2(double)` hard-conflict with `cc.h:625` `long round(long, int);` and `cc.h:702` `int log2(uvlong);`. [VERIFIED: sys/src/cmd/6c/math.h read this session; conflict reproduced and resolved in trial]
3. **`7c/gc.h` swcmp decl** — change `int swcmp(void*, void*);` (9front gc.h:268, stale even vs 9front's own pswt.c) to `int swcmp(const void*, const void*);` matching Harvey's `cc/pswt.c:4` definition and `6c/gc.h:273`. [VERIFIED: all three files read]
4. **`7c` OROL excision** — 9front's frontend has a rotate-fold optimization Harvey's lacks: enum `OROL` (9front cc.h:277), producer `rolor()` (9front cc/sub.c:918–952), consumers `case OROL:` in 7c/cgen.c:202 and 7c/txt.c:1054–1062, and the caller `rolor(n);` in 7c/sgen.c:191. Neutralize **in the vendored 7c only**: delete/comment the `rolor(n);` call (sgen.c:191), delete the `case OROL:` label (cgen.c:202 — it shares a body with `case OAND:`/`case OOR:`, so only the label line goes), and delete the OROL case block (txt.c:1054–1062). After that, no OROL node can ever be produced. [VERIFIED: 9front sub.c/cgen.c/txt.c/sgen.c read; deletion compiled and linked in trial]
5. **`7a/a.h` hunk-allocator decls** — Harvey's shared `lexbody` defines `alloc()`/`allocn()` using `hunk`/`nhunk`/`thunk`/`gethunk` (lexbody:40–76), declared in Harvey's `cc.h:446/461/481/676` and re-declared per-tool in `6a/a.h:112–172`; 9front's 7a/a.h doesn't (9front keeps the allocator `static` inside `cc/compat:81–99`). Append to vendored 7a/a.h (model: 6a/a.h): `EXTERN char *hunk; EXTERN long nhunk; EXTERN long thunk; void gethunk(void); void* allocn(void*, long, long); void* mysbrk(ulong);`. Storage comes from `7a/lex.c:1` `#define EXTERN` (empty) — same pattern as 6a/lex.c:1. [VERIFIED: both trees read; trial built]
6. **`7a/a.h` BUFSIZ** — `#define BUFSIZ IOUNIT` → `#define BUFSIZ (16*1024)` (IOUNIT exists in neither plan9port's libc.h nor Harvey's `sys/include/libc.h`; Harvey's 6a/a.h:21 uses `(16*1024)`). [VERIFIED]
7. **`7a/a.h` macexpand decl** — `int macexpand(Sym*, char*, int);` (9front's 3-arg form) → `void macexpand(Sym*, char*);` matching Harvey's macbody:378 definition and 6a/a.h:163. 9front's 7a sources never call macexpand directly, so only the decl changes. [VERIFIED]
8. **`7l/l.h` MAXIO** — `MAXIO = IOUNIT,` → `MAXIO = (16*1024),` (Harvey's 6l/l.h:125 pattern). [VERIFIED]
9. **NEW `7l/unixcompat.c`** — 9front's 7l gets `my*`/alloc from `../cc/compat` on Plan 9; Harvey's 6l solves host builds with `6l/unixcompat.c`. Vendored 7l gets the analogous self-contained file: hunk-backed `malloc`/`calloc`/`free`/`realloc` + `mysbrk` + `setmalloctag` no-op. **Critical:** `gethunk` must call `mysbrk` (→ `sbrk`), NEVER `malloc` — research's first trial hit the infinite-recursion segfault (`malloc`→`gethunk`→`malloc`, verified via macOS crash report stack-guard hit). Model: `6l/obj.c:1291` `gethunk` uses `mysbrk(nh)`. Note 9front's 7l sources use only `malloc`/`calloc` (no bare `alloc(`) — verified by grep. [VERIFIED: trial + crash diagnosis]

**Generator note (enam.c / cnam.c):** 9front does not commit `7c/enam.c` (generated from `7.out.h` by `cc/mkenam`, an `ed` script) nor `7l/cnam.c` (generated by `7l/mkcname` from `l.h`'s `C_*` enum). Both scripts ran correctly this session with host `ed` (+ plan9port `rc` for mkenam's `$T^c` concat syntax). Recommendation: **commit the generated `enam.c`** (Harvey's 6c precedent — 6c/mkfile has no enam.c rule, the file is committed) and **generate `cnam.c` in 7l's BUILD/mkfile** (it must regenerate if l.h's enum changes; the BUILD runs `sh mkcname`).

### Pattern 3: libmach arm64 wiring (TC-01 full site map)

Six sites, in dependency order. Alpha excision (D-04/D-05) is sites 1–3; arm64 addition is sites 3–6.

1. **Delete Alpha files** [VERIFIED: `ls sys/src/libmach/` this session]: `alpha/` (contains `7.out.h`, `ureg.h`), `alpha.c`, `alphadb.c`, `alphaobj.c`, plus committed stale artifacts `alpha.6 alpha.8 alphadb.6 alphadb.8 alphaobj.6 alphaobj.8`. (The tree also commits dozens of other stale `.6`/`.8` objects across libmach — those stay; hygiene phase.)
2. **`sys/include/mach.h`** — remove the five Alpha enums (verified verbatim this session): `MALPHA,` (line 35), `FALPHA,` (64), `FALPHAB,` (65), `AALPHA,` (91), `ObjAlpha,` (110). Keep `MARM64` (40), `FARM64`/`FARM64B` (72–73), `AARM64` (95), `ObjArm64` (115). **Divergence note:** 9front *keeps* these enums (`FALPHA /* 7.out */` still present in 9front's mach.h) — Harvey's D-05 excision is deliberate and safe because the enums are in-memory selectors only (object files carry magic numbers, not enum values). Also verified: `ObjArm64, /* .4? */` — the `.4?` comment is **stale in 9front too** (9front mach.h:107 identical); the true suffix is `.7` per 9front obj.c's table (`[ObjArm64] "arm64 .7", _is7, _read7`). The comment may optionally be corrected to `/* .7 */` while here.
3. **`sys/src/libmach/mkfile`** — remove `alpha\` (line 22), `alphadb\` (33), `alphaobj\` (42), and the dep rule `alphaobj.$O: alpha/7.out.h` (69); add `7\`, `7db\`, `7obj\` to FILES and `7obj.$O: 7c/7.out.h` (following the `4obj.$O: 4c/4.out.h` pattern — `CFLAGS=$CFLAGS -I/sys/src/cmd` at line 63 already makes 7obj.c's `#include "7c/7.out.h"` resolve). [VERIFIED: mkfile read]
4. **`sys/src/libmach/setmach.c`** — delete the `"alpha"` table entry (lines 121–126) and `malpha`/`alphamach` from the externs (lines 19–22); add `marm64`/`arm64mach` externs and 9front's entry verbatim: `{ "arm64", /*ARM64*/ FARM64, FARM64B, AARM64, &marm64, &arm64mach, },` (9front setmach.c:109–114). [VERIFIED: both files read]
5. **Vendored `7.c`/`7db.c`/`7obj.c`** land in `sys/src/libmach/`. `7.c:9` carries the one absolute include to rewrite per D-11: `#include "/arm64/include/ureg.h"` → `"/aarch64/include/ureg.h"` — the **only** `/arm64/` path in the whole import set (verified by grep across the donor set). Compatibility proof: the three files do **not** use `Map.segment.read` (a member 9front's mach.h has and Harvey's lacks) — verified by grep this session.
6. **`sys/src/libmach/executable.c`** — **required, and missing from D-11's list.** `crackhdr` (used by nm/acid) needs the exectab entry; 9front's (executable.c:239–247) is verbatim:

```c
/* Source: 9front sys/src/libmach/executable.c:239-247 [VERIFIED: donor read this session] */
	{ R_MAGIC,			/* Arm64 7.out and boot image */
		"arm64 plan 9 executable",
		"arm64 plan 9 dlm",
		FARM64,
		1,
		&marm64,
		sizeof(Exec)+8,
		nil,
		commonllp64 },
```

plus the `FARM64` case in the bootimage switch (9front executable.c:372–377: `fp->type = FARM64B; fp->txtaddr = fp->entry; fp->name = "arm64 plan 9 boot image"; fp->dataddr = _round(...)`). Harvey's executable.c already has `commonllp64` (line 473) and `armdotout` (892) — the entry is mechanical. [VERIFIED: both files read]

7. **`sys/src/libmach/obj.c`** — add `[ObjArm64] "arm64 .7", _is7, _read7,` to the object-parser table (9front obj.c:59). Also **required beyond D-11's list.**

### Pattern 4: `case aarch64` syscall stub generator (the ABI freeze, D-15)

9front's `sys/src/libc/9syscall/mkfile` arm64 case, verified verbatim this session (lines 110–117):

```
case arm64
	j=$i
	if(~ $i seek) j=_seek
	echo TEXT $j'(SB)', 1, '$0'
	echo MOV R0, '0(FP)'
	echo MOV '$'$n, R0
	echo SVC
	echo RETURN
```

Harvey's `case aarch64` is this block with the `arm64`→`aarch64` case-label rename. Harvey's existing line-9 rule (`if(~ $i exits) i=_exits`) supplies `_exits` naming unchanged. Notes for the freeze:
- The `_seek` rename pairs with `libc/aarch64/_seek.c` (the C wrapper calling the 4-arg `_seek(vlong*, int, vlong, int)` stub) — both sides are in the import set.
- Harvey's `NSEC=53` generates a stub named `nsec` (Harvey's sys.h lacks 9front's underscore rename). Verified this session: on amd64, the generated 137-byte stub **wins** over `libc/9sys/nsec.c`'s bintime-reader in `libc.a` (member extracted and sized). aarch64 inherits this; it is wire-correct (zero-arg, result in R0) — the dead C fast path is a pre-existing amd64 condition, not a Phase 2 regression. (Optional future cleanup: rename to `_NSEC` like 9front — hygiene phase, not this one; wire unchanged.)
- **Wording correction to D-15 for the freeze document:** the kernel reads args at user `SP+8` (`(Sargs*)(ureg->sp+BY2WD)`, BY2WD=8 — 9front `sys/src/9/arm64/trap.c:196` and `arm64/mem.h:92`, both read this session). `SP+0` is the reserved link-register slot; the stub's `MOV R0, 0(FP)` materializes arg0 into the first slot the kernel reads. Roadmap Phase 4's "args at SP_EL0+0" should be read as "the stub frame's arg slots, starting 8 bytes below the kernel-visible `ureg->sp`-relative base…" — the freeze doc must state: **number in R0, args in 8-byte slots at user SP+8 (SP+0 reserved), SVC #0, result in R0**.

### Pattern 5: libc/aarch64 import (TC-03)

Verified donor `sys/src/libc/arm64/` contents (20 files): `_seek.c argv0.s atom.s atomic.s atomic64.s cycles.s fabs.s floor.s getcallerpc.s getfcr.s lock.c main9.s main9p.s memmove.s memset.s notejmp.c setjmp.s sqrt.s tas.s mkfile`. [VERIFIED: donor listing]

Import verbatim except the mkfile's first line `objtype=arm64` → `objtype=aarch64` (rename-on-import). The donor mkfile (verbatim):

```
objtype=arm64          # → aarch64 on import
</$objtype/mkfile
OS=7

LIB=/$objtype/lib/libc.a
SFILES=\
	argv0.s\ atom.s\ atomic.s\ atomic64.s\ cycles.s\ fabs.s\ floor.s\
	getcallerpc.s\ getfcr.s\ main9.s\ main9p.s\ memmove.s\ memset.s\
	setjmp.s\ sqrt.s\ tas.s\
CFILES=\ lock.c\ notejmp.c\ _seek.c\
HFILES=/sys/include/libc.h
OFILES=${CFILES:%.c=%.$O} ${SFILES:%.s=%.$O}
</sys/src/cmd/mksyslib
```

Reconcile list (D-14, upgraded from MEDIUM to verified specifics):
- **`_callmain` dependency:** 9front's arm64 `main9.s` ends `B _callmain(SB)`; `_callmain` is defined in 9front's `libc/9sys/callmain.c` which Harvey lacks (Harvey's libc has no `_callmain` anywhere — verified by grep). Import `callmain.c` additively into `sys/src/libc/9sys/` (+`callmain.$O` in that mkfile's OFILES). It defines `_privates`/`_nprivates` which Harvey's `9sys/privalloc.c:8–9` consumes — coherent. Inert on amd64 (no references → archive member never pulled). Verified compile-compatible: needs only `memset`/`exits`, both in Harvey's libc.h (lines 387, 668).
- **Atomics API drift:** donor `atomic.s`/`atomic64.s` implement 9front's typed atomics (`agetl/agetp/aswapl/aswapp/aincl/acasl/acasp/agetv/aswapv/aincv/acasv/coherence`) which Harvey's `sys/include/libc.h` does **not** declare (Harvey has `ainc/adec/cas32/casp/casl/_tas` at lines 438–453). The asm imports verbatim regardless (no header dependency); nothing in Harvey calls the new API today (verified by grep over libthread/rc/ipconfig/factotum). Planner choice: optionally add the decls + `Along`/`Aptr`/`Avlong` typedefs to libc.h (additive, zero callers). Default: add decls so future code can use them; do not convert callers this phase.
- `lock.c` calls `_tas` and `_barrier` — both exist: Harvey's libc.h:453 declares `_tas`; `_barrier` is in the donor `tas.s` (verified verbatim: LDXRW/STXRW loop + `DMB $0xB /* ISH */`).
- `notejmp.c` consumes the frozen `ureg.h` — lands with the headers, one logical unit (Pitfall 11 discipline).

### Pattern 6: Donor-kernel harness (D-06/D-07/D-08 gate)

- Donor build locus: the pinned checkout, bind-mounted as `/` in a **lima VM Linux mount namespace** (9front mkfiles require Plan 9 absolute paths; Harvey's `RUN_BUILD_IN_PRIVATE_NAMESPACE` pattern), with `HARVEY_LINUX_BIN` (containing our 7a/7c/7l) on PATH and `objtype=arm64` *inside the donor tree* (the donor's own naming; no rename there).
- Donor build needs, in-donor-tree, all built by our 7c/7a/7l: arm64 libc + the kernel's LIB list (`libmemlayer libmemdraw libdraw libip libsec libmp libc` — 9front arm64/mkfile:61–69), the bootdir binaries (`boot`, `paqfs`, `factotum` — from the CONF's bootdir section), `mkpaqfs`-built `$CONF.bootfs.paq`, and `aux/aout2uimage` (D-12: the donor's own copy) producing `9qemu.u`. Link rule: `$LD -o $target -T$loadaddr -l $prereq` with `kzero=0xffffffff80000000`, `loadaddr=0xffffffffc0100000` (9front arm64/mkfile:5–6, 71–75). [VERIFIED: donor mkfile read]
- qemu invocation (verified against qemu 11.1.2 on the M5 Max): `qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -accel tcg -smp 1 -nographic -m 1G -kernel 9qemu.u -serial mon:stdio`. **Not HVF** (GICv2-only donor; empirically rejected). Serial capture/assert pattern: port Phase 1's `build/mm/` banner-grep discipline to plain qemu (minimega is x86-only — carried-forward decision).
- Binary delivery (discretion): bake the hello-world/shell binaries into the donor's bootfs paq during the in-donor-tree build (deterministic; no guest network config) vs. serve via host `ufs` over virtio-net (Phase 1 lineage; faster iteration). Recommendation: bake for the gate.
- Phase-exit checks: guest executes hello-world + shell subset; host `nm` on the shell binaries asserts zero `NMOUNT` references (D-09).

### Anti-Patterns to Avoid

- **Scaffolding before execution (the riscv64 death):** Harvey's `sys/src/libc/riscv64/` (11 files, mem*.s commented out) + `riscv64/mkfile` naming a never-committed `jc/jl/ja` + `riscv64` in CPUS is the in-tree fossil of the ordering failure. Gate order is strict: tools build → libc compiles → hello links → **hello executes on donor** → only then `aarch64/` dirs and CPUS. [VERIFIED: `ls sys/src/libc/riscv64/` this session]
- **Touching the shared frontend:** any fix tempting you into `cc.h`/`sub.c`/`com.c`/`dcl.c` is forbidden by D-13 — shim the vendored file instead (the OROL/rolor excision is the model).
- **Syncing 9front's cc/:** quantified drift — all 21 shared files differ (dcl.c 232 diff lines, sub.c 178, cc.h 138, com.c 132, com64.c 90, pgen.c 67). Harvey's carries the Phase 1 LP64 fixes (`cc.h:43` `#define NVAR (BITS*32)`). Do not sync.
- **Assuming HVF:** the donor kernel is GICv2-only; HVF cannot host it (verified). TCG is the harness.
- **BSD sed with `\t`:** macOS BSD sed treats `\t` as literal `t` — research's first shim attempts silently no-opped. Use `perl -pi -e` or literal tabs in BUILD scripts. [VERIFIED: this session]
- **Plan 9 hostbin tools shadowing GNU:** the devshell prepends `.build/hostbin-<os>-aarch64/bin`, so `ls`/`sed`/`date` are Plan 9 variants (no `-la`, different flags). BUILD scripts must use POSIX-safe invocations or absolute `/bin/...` paths. [VERIFIED: this session]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| A64 assembler/compiler/linker | widened 5a/5c/5l or new tools | 9front 7a/7c/7l | A decade of shipped correctness (RPi3/4, i.MX8, LX2K, qemu); re-deriving A64 codegen is Pitfall 9's multi-month silent-miscompilation detour. Proven buildable this session. |
| libc arch asm (tas/setjmp/memmove/atomics) | hand-written from ARM docs | 9front libc/arm64 verbatim (D-14) | `tas` under SMP weak-memory-order and `notejmp`/`Ureg` layout are the worst bug classes (Pitfalls 10/11); the donor set is execution-proven. |
| arm64 disassembler | own disassembler for acid/nm | 9front 7db.c (800 lines) | Complete, maintained, matches 7c's encoding choices. |
| 7-object parser | own parser | 9front 7obj.c (`_is7`/`_read7`) | Format details (ANAME/D_FILE framing) are subtle; verified against donor. |
| Startup glue (`_main`→`main`) | adapting Harvey's amd64 main9.s pattern | 9front main9.s + 9sys/callmain.c | The arm64 startup convention (main as fn ptr, argv via `&arg0` stack slot) differs from Harvey amd64's `inargc/inargv` externs — import the matched pair, don't hybridize. |
| uImage wrapping for donor kernel | importing aout2uimage | donor tree's own copy (D-12) | Harness scaffolding stays out of Harvey's tree. |
| DTB/device discovery (future kernel) | hardcoded addresses from one qemu's dumpdtb | DTB parse (Phase 3 concern) | QEMU virt docs: only flash@0x0 and RAM@0x40000000 are stable [CITED: qemu virt doc 11.1.x]. |

**Key insight:** every "deceptively small" piece (stub generator quirks, startup asm, atomics) encodes ABI decisions that only execution validates — the phase's D-06 execution gate exists precisely because these files look trivial and are not.

## Runtime State Inventory

This phase imports and deletes source; it also deletes committed build artifacts and adds new host binaries.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no databases/datastores key on `arm64`/`alpha`/tool letters | n/a |
| Live service config | None — no external services configured from these strings | n/a |
| OS-registered state | None — no launchd/systemd/Task-Scheduler registrations | n/a |
| Secrets/env vars | None — `NMOUNT`/`NSEC` are source constants; no env vars renamed (`HARVEY_LINUX_BIN` semantics unchanged) | n/a |
| Build artifacts | (a) Committed stale objects in `sys/src/libmach/`: `alpha.6 alpha.8 alphadb.6 alphadb.8 alphaobj.6 alphaobj.8` (Alpha excision deletes these per D-04; dozens of other stale `.6`/`.8` files in libmach/9sys/port stay — hygiene phase). (b) `amd64/lib/libc.a` and installed amd64 binaries are regenerated by the normal build — no manual migration. (c) `.build/hostbin-<os>-aarch64/bin/` gains `7a 7c 7l nm`(+) via BUILD scripts — auto-generated, no git state. (d) `sys/src/cmd/cc/` gains `compat.h` (new source file, not an artifact). | Code edits + `git rm` on the six Alpha artifacts; rebuilds are automatic |

## Common Pitfalls

### Pitfall A: hunk-allocator recursion (host shims)
**What goes wrong:** a vendored `unixcompat.c` whose `gethunk` calls `malloc` (which calls `gethunk`) — infinite recursion, stack-guard segfault after object load.
**Why:** the ken linker malloc is hunk-backed; Harvey's 6l breaks the cycle by having `gethunk` call `mysbrk` (`6l/obj.c:1291`).
**How to avoid:** gethunk → mysbrk → sbrk; never malloc. (Research hit this for real; diagnosed via macOS crash report.) [VERIFIED: this session]
**Warning signs:** exit 139/SIGSEGV with alternating-frame stack in the crash log.

### Pitfall B: SDK `math.h` collision (darwin host)
**What goes wrong:** `error: conflicting types for 'round'` / `'log2'` from Apple SDK math.h vs `cc.h:625/702`.
**Why:** plan9port's `u.h:57` includes `<math.h>`; without a shadow, the SDK's one-arg `round`/`log2` collide with the compiler's two-arg ones.
**How to avoid:** per-compiler-dir `math.h` shadow (6c/math.h pattern — "ONLY here for building on unix"). 9front's 7c ships none because 9front builds natively. [VERIFIED: reproduced + fixed this session]
**Warning signs:** errors cite `.../apple-sdk-.../usr/include/math.h`.

### Pitfall C: enum-position assumptions after Alpha excision
**What goes wrong:** removing `MALPHA`/`FALPHA`/`FALPHAB`/`AALPHA`/`ObjAlpha` shifts subsequent enum values.
**Why it happens:** the enums are positional.
**How to avoid:** safe here — they are in-memory selectors (crackhdr maps on-disk magic → enum; object files never store enum values). Rule: never persist these values to disk; all in-tree users recompile together. 9front keeps the enums (D-05 deliberately goes further) — either is fine, don't mix. [VERIFIED: both mach.h files read]
**Warning signs:** none at build time; a stale precompiled consumer would mis-detect file types — full rebuild after excision.

### Pitfall D: hello-world gate ordering
**What goes wrong:** `aarch64/` dirs, libc dir, or CPUS land before a binary executes → riscv64-style vaporware.
**How to avoid:** the strict order in the Summary; CPUS is edited **last** (`sys/src/mkfile.proto:6` `CPUS=amd64 arm 386 riscv riscv64 power mips` — verified). [VERIFIED]
**Warning signs:** an arch dir whose mkfile names tools absent from `sys/src/cmd/`.

### Pitfall E: NMOUNT leakage onto the donor kernel (D-09)
**What goes wrong:** a shell-subset binary calling `nmount()` dies on the 9front kernel (no syscall 54 there).
**How to avoid:** phase-exit `nm` over the subset binaries asserting zero NMOUNT refs; requires the host nm built in TC-01. CONTEXT verified rc/ipconfig/factotum clean; Harvey's `boot` calls it but the donor runs 9front's boot.

### Pitfall F: qemu invocation drift
**What goes wrong:** `-cpu cortex-a72` under `-accel hvf` → `Invalid CPU model` (valid HVF: a53/a57/host/max); `gic-version=2` under HVF → `HVF does not support GICv2 emulation`. [VERIFIED: qemu 11.1.2 probes this session]
**How to avoid:** harness pins `-accel tcg -cpu cortex-a72 -M virt,gic-version=2` for the donor kernel; document that HVF needs a GICv3 guest (r9 Phase 3 forward-note, alongside D-03's granule note).
**Warning signs:** "works on my qemu" divergence — the flake pins qemu (11.1.2 today); keep probes after flake bumps.

### Pitfall G: `seek`/`nsec` stub quirks
**What goes wrong:** generating plain stubs for SEEK/NSEC breaks the vlong conventions.
**How to avoid:** replicate the verified 9front arm64 case exactly (`_seek` rename; uniform R0 spill; no stub-side buffer store — the kernel always writes the buffer: 9front `sysseek` does `*v = sseek(...); return 0` and Harvey 9k's `sysseek` at `sys/src/9k/port/sysfile.c:904` does `*rv = sseek(fd, offset, whence); ar0->i = 0` [VERIFIED: both read this session]). NSEC is zero-arg register-return on 64-bit in both trees [VERIFIED: Harvey `sysproc.c:1216` `ar0->vl = todget(nil)`; 9front `sysproc.c:1391` `if(sizeof(uintptr)==sizeof(vlong)) return (uintptr)nsec();`].

### Pitfall H: generator scripts and host `ed`/`sed` dialects
**What goes wrong:** `mkenam` (rc + ed), `mkcname` (sh + ed), and mkfile `sed` behave differently on BSD/macOS vs Plan 9.
**How to avoid:** both ed scripts ran correctly this session [VERIFIED]; commit `enam.c` (6c precedent) so only `cnam.c` regenerates per build; keep BUILD scripts POSIX-sh (bash is present in the devshell wrapper).

## Code Examples

### The frozen wire contract (D-15, corrected)

```c
/* User side (generated stub shape, per syscall):
 *   TEXT name(SB), 1, $0
 *       MOV  R0, 0(FP)      // materialize arg0 into its stack slot
 *       MOV  $NN, R0        // syscall number
 *       SVC                 // trap
 *       RETURN
 * Kernel side (9front donor, sys/src/9/arm64/trap.c:195-196) [VERIFIED: donor read]:
 *   scallnr = ureg->r0;
 *   dosyscall(scallnr, (Sargs*)(ureg->sp+BY2WD), &ureg->r0);   // BY2WD=8 (arm64/mem.h:92)
 * Freeze wording: number in R0; args in 8-byte slots at user SP+8 (SP+0 = reserved LR slot);
 * SVC #0; result in R0. Trap frame = frozen aarch64/include/ureg.h below.
 */
```

### `aarch64/include/ureg.h` (the trap frame both r9 and Harvey's libc decode — D-15/D-17 freeze target)

```c
/* Source: 9front arm64/include/ureg.h [VERIFIED: donor file read this session, verbatim] */
typedef struct Ureg {
	u64int	r0;  u64int	r1;  u64int	r2;  u64int	r3;
	u64int	r4;  u64int	r5;  u64int	r6;  u64int	r7;
	u64int	r8;  u64int	r9;  u64int	r10; u64int	r11;
	u64int	r12; u64int	r13; u64int	r14; u64int	r15;
	u64int	r16; u64int	r17; u64int	r18; u64int	r19;
	u64int	r20; u64int	r21; u64int	r22; u64int	r23;
	u64int	r24; u64int	r25; u64int	r26; u64int	r27;
	u64int	r28;	/* sb */
	u64int	r29;
	union {
		u64int	r30;
		u64int	link;
	};
	u64int	sp;
	u64int	pc;	/* interrupted addr */
	u64int	psr;
	u64int	type;	/* of exception */
} Ureg;
```

(The donor file uses one field per line; formatting compressed here for space — the import is verbatim. `notejmp.c` sets `r0`, `pc`, `sp` from a `jmp_buf` built by `setjmp.s` — which stores LR at `8(R0)`, SP at `0(R0)` per the donor setjmp.s [VERIFIED].)

### `aarch64/mkfile` (TC-04; 9front `arm64/mkfile` verbatim with the rename)

```
</sys/src/mkfile.proto

CC=7c
LD=7l
AS=7a
O=7
```

[VERIFIED: donor file read this session; identical shape to Harvey's `amd64/mkfile` (CC=6c LD=6l O=6 AS=6a)]

### `pcc.c` objtype entry (TC-04)

Current table (`sys/src/cmd/pcc.c:14-28`, verified this session): entries like `{"amd64", "6c", "6l", "6", "6.out"},` … `{"riscv64", "jc", "jl", "j", "j.out"},`; then `char *allos = "05689kqvij";` (line 28). Add:

```c
	{"aarch64",	"7c", "7l", "7", "7.out"},
```

and add `7` to `allos` (else pcc rejects `.7` files as "wrong architecture", line 152 check). Comment at line 13 says "sync with /sys/src/ape/cmd/cc.c" — APE is out of scope for aarch64 this phase; note it, don't do it.

### Host `nm` for the D-09 gate

`sys/src/cmd/nm.c` exists but no host BUILD script produces it. New BUILD: compile nm.c + Harvey's libmach sources (now including 7.c/7db.c/7obj.c) under plan9port 9c, link with 9l against plan9port bio/9. Risk: LOW-MEDIUM — libmach is portable Plan 9 C and the vendored `sys/src/cmd/unix/9cc/` subset proves libmach-under-plan9port compiles (its libmach is the same lineage, minus 7-object support). Not trial-built this session [ASSUMED: low risk].

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Scaffolding-first ports (headers+mkfile, no tools) | Execution-gated import (riscv64 vaporware rule) | Phase 1–2 policy | No dead dirs; CPUS lands last |
| Assume qemu user-mode can run Plan 9 binaries | System-emulation donor-kernel execution proof (D-06) | CONTEXT 2026-10-31 | QEMU user emulation targets Linux/BSD syscall ABIs only — no Plan 9 target exists [CITED: QEMU docs user-mode section; Phase 1 research] |
| Harvey 5c lineage as arm64 starting point | 9front 7c vendor-and-own | This phase | Pitfall 9 retired by construction |
| HVF-everywhere assumption on Apple silicon | Accelerator matched to guest GIC: TCG for GICv2 donor, HVF once r9 speaks GICv3 | Verified this session | Correct expectations + no debugging a qemu hard error |

**Deprecated/outdated:**
- Harvey `sys/src/libmach/alpha/*` + five Alpha enums — retired this phase (D-04/D-05).
- The `ObjArm64 /* .4? */` comment — stale in both trees (true suffix `.7`); cosmetic fix allowed while editing.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | "Config `<objtype>` section literally named `aarch64`" (TC-04) = the `arch=(...)` lists + `build_$i` dispatch in `build/scripts/{funcs,linux-build,build}` (funcs:37 `arch=(386 amd64)`, linux-build:133 `arch=(amd64)`; `build_aarch64` already exists and is correctly named) | TC-04 row, Open Q1 | Low — if another "config" was meant (e.g. nix-side), the wiring sites are all enumerated anyway |
| A2 | A pinned prebuilt 9front arm64 image is obtainable as the D-07 fallback | Pattern 6 | Medium — fallback blocked if unavailable; but it's only a fallback, and the primary (in-donor-tree build) is fully specified |
| A3 | Host libmach + nm (+acid) compile under plan9port 9c with minor/no shims | Code Examples (host nm) | Low-Medium — if libmach drags in Plan 9-isms, the fallback is target-built nm run on the donor kernel |
| A4 | The donor-tree build works inside a lima Linux namespace with our hostbin on PATH (9front mkfiles + plan9port mk) | Pattern 6 | Medium — not prototyped this session; this is the harness's main engineering task; D-07's checkpoint bounds the risk |
| A5 | 9front's arm64 CONF=qemu kernel boots under qemu `-M virt` TCG (cortex-a72, gic-version=2) as shipped | Pattern 6 | Low — it is 9front's own tested CI/qemu target [CITED: .planning/research/STACK.md + PITFALLS.md, MEDIUM tier]; if it drifts, the failure is in donor territory, not our toolchain |
| A6 | `-cpu host`/`max` under TCG is not needed; cortex-a72 under TCG is well-supported | Pattern 6 | Low — TCG emulates any listed CPU; HVF is what restricts choice |

## Open Questions (all RESOLVED at planning, 2026-10-31)

1. **TC-04 "config section literally named `aarch64`" — which file?**
   - What we know: the dormant `build_aarch64` functions (`build/scripts/build:76`, `linux-build:63`) are already correctly named; the arch lists are `funcs:37` (`arch=(386 amd64)`) and `linux-build:133` (`arch=(amd64)`); dispatch is `build_$i`.
   - What's unclear: whether the roadmap meant these lists or another config surface.
   - Recommendation: treat the funcs/linux-build arch lists as the site (adding `aarch64` there activates `build_aarch64`); planner notes the interpretation in PLAN.md.
   - **RESOLVED** (adopted by 02-06 + 02-07): the funcs/linux-build arch-list interpretation is taken (Assumption A1); 02-06 records it as a flagged assumption and 02-07 Task 3 performs the activation (`arch=(amd64 aarch64)` + subset-scoped `build_aarch64`). If another config surface was meant, the wiring sites are all enumerated and the correction is one line.
2. **Binary delivery into the donor guest: bake vs ufs?** (the agent's Discretion)
   - What we know: the CONF's bootdir embeds `$CONF.bootfs.paq` (baking = rebuild paq in-donor-tree with mkpaqfs — deterministic, zero guest config); the CONF has `ethervirtio10` + full ip stack (ufs over tcp = Phase 1 lineage, faster iteration).
   - Recommendation: bake for the D-06/D-08 gates; document ufs as the iteration path.
   - **RESOLVED** (adopted by 02-04/02-05/02-07): bake-for-gates is implemented in run-gate.sh (02-05) and reused by subset-exec.sh (02-07); ufs-over-tcp is documented as the iteration path in build/aarch64/README.md (02-05 Task 2).
3. **Host vs target nm/acid for SC#2/D-09?**
   - What we know: host nm needs host libmach (A3); target nm/acid build for aarch64 needs only libc + libmach and run on the donor kernel.
   - Recommendation: host nm for the gate (scriptable); target-build nm/acid as TC-05 stretch (they ride the same libmach).
   - **RESOLVED** (adopted by 02-03 + 02-07): host nm is built and functionally proven on 7l objects in 02-03 Task 3, and the D-09 gate uses it in 02-07 Task 3. acid is not host-built this phase — ROADMAP SC#2's "nm/acid" is delivered **nm-only** (acid was the stretch half of this question; its 7db.c disassembler tables are landed in libmach either way, and target-built nm/acid ride the same libmach on the donor kernel when a future phase needs them). The nm-only interpretation is recorded in 02-07's objective and success criteria so the phase SUMMARY closes SC#2 against it.
4. **Commit `enam.c` or generate per-build?**
   - Recommendation: commit it (Harvey's 6c precedent — no enam.c rule in 6c/mkfile); regenerate only when 7.out.h's opcode enum changes (document in 7c/README).
   - **RESOLVED** (adopted by 02-01): enam.c is generated via the donor mkenam ed script and committed (6c precedent) in 02-01 Task 1; cnam.c is generated per-build via `sh mkcname` in 7l/BUILD (02-01 Task 2).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qemu-system-aarch64 | D-06/D-08 harness | ✓ (nix devshell) | 11.1.2, hvf+tcg compiled [VERIFIED: probe] | — |
| TCG accel (donor kernel path) | harness | ✓ | — | HVF is NOT a valid fallback for a GICv2 guest (verified) |
| plan9port (mk, rc, 9c, 9l, yacc) | all host builds | ✓ (nix devshell) | 0-unstable-2025-11-10 [VERIFIED] | — |
| host `ed` | mkenam/mkcname | ✓ `/bin/ed` (BSD) — both scripts proven | — | commit generated files |
| lima VM `harvey` (vz aarch64, 4 CPU, 8 GiB) | namespace builds (Harvey + donor) | ✓ **Running** this session [VERIFIED: `limactl list` via nix wrapper] | — | any linux-aarch64 host |
| nix | devshell/apps | ✓ | 2.34.6 (Determinate) | — |
| perl | shim application in BUILD scripts (BSD-sed `\t` trap) | ✓ (macOS system perl used this session) | — | literal tabs |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** host nm/acid (not yet built — new BUILD task, A3; fallback: target-built nm on the donor kernel).

## Security Domain

A C toolchain/libc port has little ASVS surface; stating applicability honestly:

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication / V3 Session / V4 Access Control | no | — (no services, no authn/authz in this phase) |
| V5 Input Validation | marginal | The vendored compiler/linker parse untrusted-ish input only in the harness sense; ken-suite robustness is inherited, not extended |
| V6 Cryptography | no | — (libsec untouched this phase — the Pitfall-13 boundary: zero libsec/devtls changes, no TLS claims) |
| V14 (Config/Build) analog | **yes** | Supply-chain discipline is the real security content of this phase: donor pinned by SHA (`35026610…`, recorded per D-10); per-dir provenance READMEs (license: MIT-compatible, credit Bell Labs/Lucent, Vita Nuova, C H Forsyth, 9front); no network fetches at build time beyond the one pinned clone; qemu/plan9port pinned by `flake.lock`; the 9l `-static` shim and hostbin PATH ordering documented in `nix/toolchain.sh` |

Known threat patterns for this stack: none new. The one carry-forward rule (PITFALLS "Security Mistakes"): new code uses `kstrcpy`/`snprint`, never propagates the kernel `strcpy`/`sprintf` idiom — vacuously satisfied this phase (no new kernel code).

## Sources

### Primary (HIGH confidence — read/trial-verified this session)
- Harvey tree (direct reads, quoted inline): `sys/include/mach.h`, `sys/include/a.out.h:14,16,31-33`, `sys/include/libc.h` (438-453), `sys/src/libmach/{setmach.c,mkfile,executable.c}`, `sys/src/libc/9syscall/{mkfile,sys.h}`, `sys/src/libc/{mkfile,amd64/main9.s,amd64/tas.s,amd64/atom.s,9sys/nsec.c,9sys/privalloc.c,9sys/mkfile,riscv64/}`, `sys/src/mkfile.proto`, `amd64/mkfile`, `sys/src/cmd/{mkfile,pcc.c,BUILD}`, `sys/src/cmd/cc/{mkfile,cc.h,compat,compat.c,lexbody,macbody,pswt.c,dcl.c}`, `sys/src/cmd/6{c,a,l}/{BUILD,mkfile}`, `sys/src/cmd/6c/{math.h,gc.h}`, `sys/src/cmd/6a/a.h`, `sys/src/cmd/6l/{unixcompat.c,l.h,obj.c:1291}`, `sys/src/cmd/unix/9cc/README`, `sys/src/BUILDTOOLCHAIN`, `sys/src/9k/port/{sysfile.c:904,sysproc.c:1216}`, `build/scripts/{funcs,build,linux-build}`, `build/mkdirs`, `nix/{toolchain.sh,build.sh,setup.sh,vm-build.sh}`, `flake.nix`
- 9front donor (`350266104f1fed81c78c3d05fa44f523bec0ae4d`, cloned 2026-10-31): `sys/src/cmd/{7a,7c,7l}/` (all sources + mkfiles), `sys/src/cmd/cc/` (mkfile.port, compat.h, mkenam, sub.c:918-952), `sys/src/libc/arm64/` (all 20 files), `sys/src/libc/9sys/callmain.c`, `sys/src/libc/9syscall/mkfile:110-117`, `sys/src/libc/9syscall/sys.h`, `sys/src/libmach/{7.c,7db.c,7obj.c,setmach.c,obj.c,executable.c}`, `sys/include/mach.h`, `arm64/{mkfile,include/u.h,include/ureg.h}`, `sys/src/9/arm64/` (mkfile, qemu CONF, gic.c, trap.c:186-207, mem.h:92), `sys/src/9/port/sysproc.c:1391`, `sys/src/9/{imx8,lx2k}/gic.c` (GICv3 existence check)
- **Trial builds this session (darwin-aarch64, nix devshell):** 7c full compile+link+codegen smoke (`-S` output verified for recursion/shift/FP); 7a full compile+link+assemble of 7c's `-S` output; 7l full compile+link+object-parse with correct ken-linker diagnostics (`entry not text: _main` / `_main: not defined`); the gethunk→malloc recursion fault reproduced and fixed via mysbrk; macOS crash-report stack-guard diagnosis
- qemu probes this session: `-accel help` (hvf+tcg present); `-accel hvf -cpu cortex-a72` → `Invalid CPU model` (valid: cortex-a53, cortex-a57, host, max); `-accel hvf -M virt,gic-version=2` → `HVF does not support GICv2 emulation`; `sysctl kern.hv_support=1`
- r9 repo (`/Users/neilskinner/code/r9`, read this session): workspace members `aarch64/x86_64/port/riscv64/xtask`; aarch64 is raspi3b/4b-only; `aarch64/src/trap.rs:110-113` reads the syscall id from `esr_el1.iss()` (the SVC immediate) and spins — **Phase 4 must switch it to `frame.x0`** per the freeze; `TrapFrame` is `#[repr(C, align(16))]` with x0..x28, frame_pointer(x29), link_register(x30), esr_el1, elr_el1, far_el1 — the Phase 4 static-check target vs the frozen ureg.h; xtask already has `-machine virt[,dumpdtb]` plumbing used today only for riscv64

### Secondary (MEDIUM)
- QEMU `'virt' generic virtual platform` doc, version 11.1.x (qemu-project.gitlab.io/qemu/system/arm/virt.html, fetched 2026-10-31): default CPU cortex-a15 (32-bit; aarch64 guests MUST pass -cpu); gic-version values 2/3/4/x-5/host/max; GICv2 caps at 8 CPUs; flash@0x0, RAM@0x40000000 stable, all else via DTB; bare-metal/ELF boot ⇒ DTB at RAM start; Linux-protocol (non-ELF) boot ⇒ DTB in x0. [CITED]
- `.planning/research/STACK.md` + `PITFALLS.md` (2026-10-30): donor map, license/NOTICE action, Plan B (0intro/9-cc), Pitfalls 3/4/9/10/11/14. Where this session's direct reads disagreed with nothing, they were re-verified anyway.

### Tertiary (LOW)
- Prebuilt 9front arm64 image availability (9fans/community, via Phase 1 research) — fallback only (A2).
- Research-store seam note: provider tiers rated LOW (no curated docs DB configured); the load-bearing claims above are nonetheless primary-source (file reads, build transcripts, binary probes) and are tagged accordingly.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — donor pinned and read; toolchain trial-built and run on the actual host
- Architecture: HIGH — all integration points opened and quoted; two D-11 list gaps found (executable.c, obj.c)
- Pitfalls: HIGH — several reproduced live this session (math.h, sed `\t`, gethunk recursion, HVF rejections)
- Harness (donor build in lima namespace): MEDIUM — fully specified but not prototyped (A4)

**Research date:** 2026-10-31
**Valid until:** 2026-11-30 (donor pin immutable; qemu/plan9port re-probe after any flake bump)
