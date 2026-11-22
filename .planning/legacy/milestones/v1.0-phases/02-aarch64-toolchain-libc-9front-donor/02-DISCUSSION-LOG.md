# Phase 2: aarch64 Toolchain + libc (9front donor) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-31
**Phase:** 2-aarch64 Toolchain + libc (9front donor)
**Areas discussed:** ISA identity (arm64/aarch64 conflation), Alpha letter-7 resolution, Hello-world proof standard, Import mechanics, Donor fidelity, Syscall ABI freeze

---

## ISA identity (arm64 vs aarch64 conflation)

Raised by the user before gray-area selection: concern that arm64 (RPi4+) and aarch64 (Apple M1+) might be different targets, and whether 9front's arm64 imports genuinely cover native execution on an Apple M5 Max.

Resolved with facts: arm64 ≡ aarch64 (same AArch64/ARMv8-A ISA); 9front's 7c emits baseline ARMv8.0-A userspace code that runs natively on Apple silicon; what differs between Pi/Apple/qemu is the *platform layer* (boot protocol, interrupt controller, memory map) — kernel territory, and our kernel target is qemu `-M virt`, which presents the same GICv2/PL011/DTB contract on any host.

| Option | Description | Selected |
|--------|-------------|----------|
| qemu virt via HVF | M5 Max natively executes guest AArch64 inside qemu -M virt (Hypervisor.framework) — matches locked scope | ✓ |
| Bare-metal Apple silicon someday | Asahi-scale problem (m1n1, AIC, no firmware DTB); v2+ milestone at minimum | |

**User's choice:** qemu virt via HVF
**Notes:** Forward-notes for Phase 3 recorded: 16K host pages constrain HVF guest granules; cache maintenance via CTR_EL0 discovery. Bare-metal Apple noted as deferred idea.

---

## Alpha letter-7 resolution

Collision surface mapped before asking: `libmach/alpha/` (7.out.h, ureg.h), alpha.c/alphadb.c/alphaobj.c, setmach.c:121 entry, mkfile wiring, five retired mach.h enums; last touched 2013; vendored 9-cc's mach.h unaffected; mach.h already has MARM64/FARM64/AARM64/ObjArm64.

| Option | Description | Selected |
|--------|-------------|----------|
| Hard-delete Alpha | Follow 9front exactly; git history is the attic | ✓ |
| Quarantine to attic | Move files aside; but Phase 1 deferred attic work to hygiene phase | |
| Reletter Alpha | Keep Alpha buildable on another letter; most work, preserves nothing usable | |

**User's choice:** Hard-delete Alpha
**Follow-up — excision depth:**

| Option | Description | Selected |
|--------|-------------|----------|
| Collision-only excision | Alpha files + setmach + mkfile + five Alpha mach.h enums; other retired arches stay | ✓ |
| Sweep all retired arches | Also remove MIPS2/29000/SPARC64 entries — deferred hygiene scope | |

---

## Hello-world proof standard

Presented with the tension: roadmap SC#3's "qemu user-mode" example is impossible (no Plan 9 user emulation); Phase 1's lesson is that build-green masks silent miscompilation; Phase 4 SC#2 already plans r9 execution later.

| Option | Description | Selected |
|--------|-------------|----------|
| Run on 9front kernel | Execute hello-world on 9front's arm64 kernel under qemu virt/HVF — validates codegen + libc asm + syscall ABI vs reference; Phase 4 ABI bisect point | ✓ |
| Static-only gate | compile+link+nm/acid/disasm; execution deferred to Phase 4 | |
| Hybrid | static gate for scaffolding, exec at phase exit | |

**User's choice:** Run on 9front kernel
**Follow-up — kernel provenance:**

| Option | Description | Selected |
|--------|-------------|----------|
| Prebuilt image, pinned | Download pinned 9front arm64 image as test fixture | |
| Build donor kernel with our 7c | Maximal end-to-end proof: our toolchain compiles the donor's own kernel | ✓ |

**User's choice:** Build donor kernel with our 7c (the stronger option, over the recommendation)
**Follow-up — harness scope:**

| Option | Description | Selected |
|--------|-------------|----------|
| Harness covers TC-05 too | Shell subset (rc, ipconfig, factotum, core cmds) executes on donor kernel at phase exit | ✓ |
| Hello-world only | Subset link-verified only; first execution in Phase 5 | |

**Notes:** In-donor-tree vs imported build was left open initially; the user's later "temporary bridge" framing resolved it to in-donor-tree (donor kernel never enters Harvey's tree). Fallback to prebuilt image requires an explicit user checkpoint.

---

## Import mechanics

| Option | Description | Selected |
|--------|-------------|----------|
| Repeatable scripted import | Committed script: pinned checkout → copy → arm64→aarch64 rename → NOTICE | |
| One-shot + provenance READMEs | unix/9cc precedent; pin SHA recorded | ✓ |

**User's choice:** One-shot + provenance READMEs
**Notes (user, quoted):** *"One-shot is fine here, remember that our goal is using the r9 kernel, so this 9front stuff is a temporary bridge (maybe we should focus on changes needed for r9 to be the plan9 arm64 kernel and skip this 9front import step?)"*

The bridge question was answered with analysis: r9 replaces the kernel, not the compiler — the toolchain import is unavoidable and permanent (something must compile Plan 9 C userland on any kernel). But the bridge framing correctly *shrinks* the import: donor kernel stays in the pinned checkout as scaffolding; aout2uimage is not imported into Harvey.

| Option | Description | Selected |
|--------|-------------|----------|
| Confirm shrunk scope | Permanent set only (7a/7c/7l, libmach 7-files, libc→aarch64, includes, objtype wiring, 9syscall stubs); bridge stays in donor checkout; no aout2uimage | ✓ |
| Import more than the bridge needs | aout2uimage and/or donor kernel into Harvey's tree | |

---

## Donor fidelity

| Option | Description | Selected |
|--------|-------------|----------|
| Adapt 7c/7l to Harvey cc | Minimal local shims; NVAR=BITS*32 LP64 fix (shared by 6c/8c/5c) untouchable | ✓ |
| Sync 9front's cc pieces | Closer to verbatim but risks amd64 green | |

| Option | Description | Selected |
|--------|-------------|----------|
| Full 9front asm set | Verbatim libc/arm64 asm; reconcile 9front-only API drift as it surfaces | ✓ |
| Minimal asm + portable C | riscv64 conservative pattern; weaker execution proof | |

---

## Syscall ABI freeze (user-requested area)

User asked: "I want to understand the difference in syscalls between the harvey kernel and 9front 9syscall stubs we will be importing."

Investigated live: Harvey sys.h vs 9front's current table (fetched and diffed — 0–52 identical; 53 NSEC vs _NSEC rename, same wire; 54 NMOUNT Harvey-only, 6-arg nmount with dev channel; Harvey's 5-arg mount() = 9front's). Stub conventions per arch (number in register, args on stack, trap). Kernel-side dispatch difference (9k's Ar0*/va_list vs 9front's by-value Ar0) shown not to cross the wire. Verified rc/ipconfig/factotum never call nmount (safe on the donor kernel); phase-exit nm check recorded.

| Option | Description | Selected |
|--------|-------------|----------|
| Freeze full sys.h | Stubs for 0–54 incl. NMOUNT, 9front arm64 convention; r9 implements lazily | ✓ |
| Trim NMOUNT for aarch64 | Diverges libc per-arch; re-added the moment something calls nmount | |

---

## the agent's Discretion

- 9P delivery mechanism into the running donor kernel (ufs lineage vs image baking) — planner/researcher
- Per-dir NOTICE/README wording — research license action + unix/9cc/README shape
- Timing of 7c joining cc/mkfile CURCC — only after it builds (Pitfall 9)
- libc mem*/str* C-vs-asm for anything 9front's set doesn't cover — portable C first

## Deferred Ideas

- Broad skeletal-port attic/quarantine (riscv64 vaporware tree, arm/mips/power fragments, retired mach.h arches MIPS2/29000/SPARC64) — later hygiene phase
- aout2uimage import into Harvey — revisit only if a future boot path needs uImage wrapping (r9 is ELF/Rust)
- Bare-metal Apple silicon bring-up (m1n1/AIC — Asahi-scale) — v2+ at earliest; qemu virt via HVF is the target
- Go programs in the aarch64 image — Go unsupported on plan9/arm64 (already Out of Scope)
