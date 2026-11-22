# Phase 2: aarch64 Toolchain + libc (9front donor) - Context

**Gathered:** 2026-10-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Harvey can compile and link its own aarch64 Plan 9 binaries on darwin-aarch64 and linux-aarch64 hosts — vendor `7a`/`7c`/`7l` + libmach arm64 + `libc/aarch64` from 9front, wire `objtype=aarch64`, and prove it with binaries that *execute* (not just link) on a 9front reference kernel under qemu `-M virt`. The shell-boot userland subset (rc, ipconfig, factotum, core cmds) builds and runs for aarch64, freezing the user side of the Harvey↔r9 ABI.

NOT in this phase: any Harvey aarch64 *kernel* (r9 is the kernel future — 9k C aarch64 port is out of scope), r9 work (Phases 3–4), root-fs delivery to r9 (Phase 4), ISO/dist stage, broad skeletal-port cleanup (deferred hygiene phase).

</domain>

<decisions>
## Implementation Decisions

### ISA identity (clarified up front — the arm64/aarch64 conflation)
- **D-01:** `arm64` ≡ `aarch64` — two names for the same AArch64/ARMv8-A ISA. 9front's arm64 imports **are** the aarch64 toolchain; no custom "aarch64 target" exists or is needed (rolling our own = Pitfall 9's multi-month silent-miscompilation detour for zero ISA benefit). The project's `aarch64` label is a tree-naming choice only (D-06 rename-on-import, carried forward). A Cortex-A72 (RPi4) and an Apple M5 Max both execute the same AArch64 userspace code 7c emits. — **Reversibility:** one-way — every import, stub, and the Phase 4 ABI freeze builds on this identity; undoing it means re-deriving a toolchain.
- **D-02:** Execution loci, pinned: (a) host tools (`7a`/`7c`/`7l` themselves) compile via plan9port `9c` into native macOS arm64 / linux aarch64 binaries — the Phase 1-proven pattern; (b) guest code runs on qemu `-M virt` via **HVF on the M5 Max** = native execution of guest AArch64 on Apple silicon (virtualization, not emulation); TCG is the fallback and the only path on x86_64-linux. Bare-metal Apple silicon confirmed out of scope.
- **D-03:** Forward-notes to Phase 3 (kernel territory, not Phase 2 blockers): Apple host 16K pages constrain HVF guest granule choices (9front arm64's 64K-granule preference will not transfer); r9's cache maintenance must use architectural discovery (`CTR_EL0`), not hardcoded Apple geometry.

### Alpha letter-7 resolution (TC-01)
- **D-04:** Hard-delete Alpha from libmach, following 9front exactly: remove `sys/src/libmach/alpha/` (`7.out.h`, `ureg.h`), `alpha.c`, `alphadb.c`, `alphaobj.c` (+ committed stale `.6`/`.8` artifacts), the `setmach.c:121` `"alpha"` entry, and `libmach/mkfile` wiring (lines 22/33/42/69). Git history is the attic — the code is 13 years dead (last touched 2013), already marked `/* retired */`, and no Alpha compiler/libc/kernel exists in Harvey to use it. The vendored 9-cc's own `mach.h` copy (`sys/src/cmd/unix/9cc/include/mach.h`) is independent and unaffected.
- **D-05:** Collision-only excision: also remove the five Alpha enums in `sys/include/mach.h` (`MALPHA`, `FALPHA`, `FALPHAB`, `AALPHA`, `ObjAlpha`). Other retired arches (MIPS2, 29000, SPARC64) stay untouched — sweeping them is the hygiene-phase work Phase 1 deferred. `mach.h` already carries `MARM64`/`FARM64`/`FARM64B`/`AARM64`/`ObjArm64` — the enums are waiting; letter 7 was the blocker.

### Hello-world proof standard (TC-03 gate)
- **D-06:** The proof is **execution**: our 7c/7l hello-world runs on 9front's own arm64 kernel under qemu `-M virt` via HVF on the M5 Max — validating codegen, libc asm, and the syscall ABI against the reference kernel. This realizes Phase 1's "boot-test, not build-test" lesson (build-green masked LP64 miscompilation; only execution exposed it) and gives Phase 4 a known-good ABI bisect point: runs on 9front kernel but not r9 ⇒ bug is in r9. (Roadmap SC#3's "qemu user-mode" example is impossible — qemu user-mode has no Plan 9 emulation; this decision supersedes that wording.)
- **D-07:** The donor kernel is **built with our imported 7a/7c/7l** from the pinned 9front checkout (`CONF=qemu` reference kernel), built **in-donor-tree** — the 9front kernel never enters Harvey's tree; the pinned checkout is harness scaffolding. Fallback to a pinned prebuilt 9front image, if the donor-kernel build proves intractable, is a **checkpoint decision with the user** — never a silent downgrade of the proof standard.
- **D-08:** The harness doubles as the **TC-05 acceptance gate**: at phase exit the shell subset (rc, ipconfig, factotum, core cmds — all built by our 7c) executes on the donor kernel, not merely links. TC-05 gets execution evidence a full phase before Phase 5 needs it.
- **D-09:** Harness constraint with a verification: `NMOUNT=54` is Harvey-only — binaries calling `nmount()` die on the 9front kernel. Verified this session that none of rc/ipconfig/factotum call it (Harvey's `boot` does, but the donor kernel runs 9front's own boot). Phase exit check: `nm` the shell-subset binaries, assert zero `NMOUNT` references.

### Import mechanics (bridge-aware shrunk scope)
- **D-10:** One-shot import + per-dir provenance READMEs (the `sys/src/cmd/unix/9cc/` precedent); the 9front pin SHA is recorded in the harness docs. User framing, quoted: *"our goal is using the r9 kernel, so this 9front stuff is a temporary bridge."* No repeatable import script — the bridge doesn't warrant machinery.
- **D-11:** Permanent import set only: `7a`/`7c`/`7l` → `sys/src/cmd/`, libmach `7.c`/`7db.c`/`7obj.c` + setmach entry, `libc/arm64` → `sys/src/libc/aarch64`, `arm64/include/{u.h,ureg.h}` → `aarch64/include/`, objtype wiring (`aarch64/mkfile` CC=7c LD=7l AS=7a O=7, `pcc.c` entry, `mkfile.proto` CPUS — CPUS lands **last**, after the gate), and `case aarch64` in the 9syscall stub generator. All per D-06 rename-on-import: `arm64` paths normalize to `aarch64`, tool letters stay `7a/7c/7l`, donor file contents otherwise untouched (path rewrites inside files are part of normalization — e.g. 9front `7.c`'s `#include "/arm64/include/ureg.h"` becomes `/aarch64/include/ureg.h`).
- **D-12:** `aout2uimage` is **not imported** into Harvey. It wraps a Plan 9 a.out *kernel* as a uImage; r9 is Rust/ELF and Harvey builds no aarch64 C kernel — TC-05's "if the boot path needs it" resolves to: it doesn't. The donor tree's own copy serves the D-07 harness build.

### Donor fidelity
- **D-13:** Adapt 7c/7l to **Harvey's shared cc frontend** with minimal local shims (the `compat.h`/`mkfile.port` expectations). Do NOT sync 9front's cc pieces: Harvey's `cc.h` carries Phase 1's `NVAR=BITS*32` LP64 fix that 6c/8c/5c all depend on — touching the shared frontend risks the amd64 toolchain green Phase 1 just proved boot-clean. Drift reconciled file-by-file, toolchain-first diagnostic rule (Phase 1 D-02) applies when vendored code fails.
- **D-14:** Full 9front `libc/arm64` asm set imported verbatim (memmove/memset/atom/atomic/atomic64/cycles/fabs/floor/sqrt/setjmp/tas/main9/main9p/notejmp/getcallerpc/getfcr/argv0/_seek). Verified-working ABI + codegen-tested asm; with D-06's execution proof, more verbatim asm = stronger proof. 9front-only API drift reconciled against Harvey's `sys/include/libc.h` as it surfaces (research rates this MEDIUM — not yet diffed file-by-file).

### Syscall ABI freeze (TC-03/TC-05 freeze; R9-04 lead-in)
- **D-15:** Freeze the **full Harvey `sys.h`** (0–54, NMOUNT included) as the aarch64 user-side table, stubs generated with the **9front arm64 convention**: syscall number in `R0`, args in 8-byte slots at user `SP+0`, `SVC`, result in `R0` — matching the roadmap Phase 4 wording ("args at SP_EL0+0, number in R0, result in R0"). The `case aarch64` generator replicates the per-name quirks from existing cases (`seek`/`nsec` vlong buffer stores on `-1`, `_exits` naming). — **Reversibility:** one-way — this is the published contract Phase 4 builds r9's `svc` dispatch and static trap-frame checks against; changing it after Phase 4 means re-freezing both repos.
- **D-16:** Table diff vs 9front, verified this session against 9front `front` branch: **0–52 identical** (names and numbers, incl. `SYSR1=0` and the gap at 48–49); **53 same wire, renamed** (Harvey `NSEC` vs 9front `_NSEC` — researcher confirms buffer-arg semantics match); **54 `NMOUNT` Harvey-only** (`nmount(fd, afd, old, flag, aname, dev)` — 6th arg mounts a specific device channel; Harvey's 5-arg `mount()` = `MOUNT=46` matches 9front exactly). r9 implements the table lazily in Phase 4 — NMOUNT may stay unimplemented until something calls it.
- **D-17:** Kernel-side dispatch style does **not** cross the wire: Harvey 9k's generated `systab[]` handlers are `void sysxxx(Ar0*, va_list)`; 9front's are `void sysxxx(Ar0 ar0)` — internal C style only. r9 mimics neither; it matches the wire contract (R0/stack slots/result) plus the frozen `aarch64/include/ureg.h` trap frame, verified by Phase 4's build-time static checks.

### Carried forward from Phase 1 (locked there, restated so downstream doesn't re-litigate)
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` §Phase 2 — goal, success criteria 1–5, requirement mapping
- `.planning/REQUIREMENTS.md` — TC-01..05 full text + Out of Scope list
- `.planning/PROJECT.md` — Key Decisions (aarch64 naming, vendor-and-own, boot-test lesson, minimega boundary, r9 adoption)

### Prior-phase decisions and debts
- `.planning/phases/01-build-tail-green-9k-amd64/01-CONTEXT.md` — D-02 (toolchain-first diagnostic rule), D-05/D-06 (rename-on-import), reproducibility/minimega discretion items
- `.planning/phases/01-build-tail-green-9k-amd64/01-REVIEW-DISPOSITION.md` — open review debt assigned to early Phase 2 (CR-01 boot script shebang; WR-01..04 KERNDATE fallback / preflight hatch / socat race / checkall cd)
- `.planning/phases/01-build-tail-green-9k-amd64/deferred-items.md` — incl. the 9k embedded-root `bad magic` pool panic note (amd64-harness caveat; Phase 2's harness is the donor kernel, but read before touching boot paths)

### Research (donor map + risk burn-down)
- `.planning/research/STACK.md` — the 9front donor map (file sets, drift surfaces, version-compat table), license/NOTICE action, Plan B (0intro/9-cc)
- `.planning/research/PITFALLS.md` — Pitfall 3 (scaffolding-first death), 4 (naming collision), 9 (stale-5c trap), 10/11 (libc + ureg ABI contract), 14 (mk hygiene)

### Donor (external — pinned at import; SHA recorded per D-10)
- 9front branch `front` — `sys/src/cmd/{7a,7c,7l}`, `sys/src/libc/arm64/`, `sys/src/libmach/{7.c,7db.c,7obj.c,setmach.c}`, `arm64/include/{u.h,ureg.h}`, `sys/src/9/arm64/` (`CONF=qemu` — the D-07 harness kernel)
- 9front `sys/src/libc/9syscall/sys.h` — table-diff reference for D-16 (verified 2026-10-31: identical 0–52, `_NSEC` rename at 53, no NMOUNT)

### In-tree code being changed
- `sys/src/libc/9syscall/sys.h` + `sys/src/libc/9syscall/mkfile` — the syscall table and per-arch stub generator (`case aarch64` lands here)
- `sys/include/mach.h` — existing `MARM64`/`FARM64`/`AARM64`/`ObjArm64` enums; the five retired Alpha enums D-05 removes
- `sys/src/libmach/setmach.c`, `sys/src/libmach/mkfile` — the Alpha collision sites
- `sys/src/cmd/6c/BUILD`, `sys/src/cmd/6a/BUILD` — the host-tool BUILD pattern 7a/7c/7l mirror
- `sys/src/cmd/unix/9cc/README` — vendoring/provenance precedent
- `sys/src/mkfile.proto` — CPUS (aarch64 lands last, post-gate)
- `build/scripts/funcs`, `build/scripts/linux-build` — the dormant `build_aarch64` wiring (Phase 1 rename) awaiting real dirs
- `sys/src/9k/mk/parse` (`mksystab`), `sys/src/9k/port/portdat.h` (`Ar0`) — kernel-side dispatch context (wire-invisible per D-17; read to understand what r9 does NOT need to mimic)

### Harness precedents
- `build/mm/` — Phase 1's launch/start `.mm` split + serial capture/assert pattern (banner grep is source of truth); port the *pattern* to plain qemu for aarch64
- Phase 1 `ufs` lineage (Go 9P file server serving the tree to a VM) — candidate mechanism for binary delivery to the donor kernel

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `.build/hostbin-<os>-aarch64/` — the working host Plan 9 toolchain (6a/6c/6l, cpp, pcc, vendored `iar`); 7a/7c/7l BUILD scripts plug into this same machinery
- `sys/src/cmd/unix/9cc/` — the vendor-in-tree + BUILD-script precedent (README provenance shape)
- lima VM `harvey` — the linux-aarch64 namespace-build environment on this mac (for aarch64 userland cross-builds once wiring lands)
- Phase 1 serial-capture/assert wrappers (`build/mm/`) — the expect-style assertion pattern, minus minimega (x86-only)

### Established Patterns
- `</$objtype/mkfile` + `sys/src/mkfile.proto` wiring; `mksyslib`/`mkone`/`mkmany` includes; standard targets (`all/install/clean/nuke/update/installall`)
- 9syscall stub generation: mkfile sed-extracts `sys.h` names/numbers and emits per-arch asm — the `case aarch64` block is the entire stub deliverable's shape
- mk DIRS strict abort-on-first-failure; `checkall` report-all diagnostic target now exists (Phase 1) for full-tree failure visibility
- Host-tool BUILD scripts compile Plan 9 sources with plan9port `9c`/`9l` and install into per-OS/arch hostbin dirs

### Integration Points
- `sys/src/cmd/mkfile` — 7a/7c/7l join the cmd tree here (and `cc/mkfile` CURCC only after 7c builds)
- `sys/src/libc/mkfile` (`$PORTDIRS $objtype`) — `libc/aarch64` lands here post-gate
- `sys/src/libmach/{mkfile,setmach.c}` + `sys/include/mach.h` — libmach arm64 wiring after Alpha excision
- `pcc.c` objtype entry, `aarch64/mkfile`, config `<objtype>` section literally named `aarch64` (TC-04)
- `build/scripts/linux-build:63` `build_aarch64` — dormant correct-but-inert wiring from Phase 1, activated by this phase's real dirs

</code_context>

<specifics>
## Specific Ideas

- The user's bridge framing (drives D-10/D-11/D-12): *"remember that our goal is using the r9 kernel, so this 9front stuff is a temporary bridge"* — import the minimum permanent set; the donor kernel and its build stay out of Harvey's tree.
- qemu invocation shape for the harness (matching roadmap Phase 3): `qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -smp 1 -nographic` (+ `-accel hvf` on the M5 Max).
- The hello-world gate order is strict: 7a/7c/7l build natively → libc/aarch64 compiles → hello-world links → **executes on donor kernel** → only then arch dir/mkfile/CPUS scaffolding.
- `nsec` naming nuance for the researcher: Harvey's stub is `nsec` (53), 9front's kernel implements 53 as `_nsec` — confirm buffer-arg semantics match on the wire.
- Researcher's open file-level questions: diff 9front `7c/7a/7l` + `cc/` expectations vs Harvey's `cc/` (the D-13 shim surface); 9front libc/arm64 symbols with no Harvey `libc.h` counterpart (the D-14 reconcile list); 9front `ObjArm64` object-char expectations vs Harvey `mach.h`'s curious `ObjArm64, /* .4? */` comment.

</specifics>

<deferred>
## Deferred Ideas

- Broad skeletal-port attic/quarantine (riscv64 vaporware tree, arm/mips/power fragments, other retired `mach.h` arches MIPS2/29000/SPARC64) — later hygiene phase (Phase 1 deferral, reconfirmed by D-05)
- `aout2uimage` import into Harvey — revisit only if a future boot path needs uImage wrapping of a Plan 9 a.out kernel (r9 endgame is ELF/Rust) — D-12
- Bare-metal Apple silicon bring-up (m1n1 boot chain, Apple AIC instead of GIC, no firmware DTB — Asahi-scale problem) — user considered and set aside; v2+ at earliest, qemu virt via HVF is the target (D-02)
- Go programs in the aarch64 image — Go unsupported on plan9/arm64 (already Out of Scope in REQUIREMENTS.md)

</deferred>

---

*Phase: 2-aarch64 Toolchain + libc (9front donor)*
*Context gathered: 2026-10-31*
