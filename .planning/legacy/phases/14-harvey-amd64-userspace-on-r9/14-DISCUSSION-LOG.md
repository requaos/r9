# Phase 14: Harvey amd64 Userspace on r9 - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-08
**Phase:** 14-Harvey amd64 Userspace on r9
**Areas discussed:** ABI source-of-truth, Bootfs medium, Compose shape, Exec staging + entry

---

## ABI source-of-truth (XEXEC-01)

### Q1: Contract shape between r9's trap path and Harvey's existing amd64 Ureg

| Option | Description | Selected |
|--------|-------------|----------|
| TrapFrame≡Ureg, 9k-verbatim | r9's TrapFrame IS the Ureg — byte-verbatim from amd64/include/ureg.h, no translation; same binaries run on both kernels (k10t1 executes the identical 6.outs); the aarch64 Phase-4 precedent | ✓ |
| Internal frame + translate | r9 keeps its natural trap layout; a shim fills a Ureg on note delivery only — freedom, but translation on every note and dual testing | |
| You decide | Researcher compares layouts; planner picks under a no-divergence constraint | |

**User's choice:** TrapFrame≡Ureg, 9k-verbatim
**Notes:** Noted during discussion: this DERIVES the syscall entry mechanism too — r9 implements whatever 9k's existing `9syscall/amd64` stubs emit; no fresh entry-mechanism choice exists (recorded as D-02a, and the SYSCALL-vs-int-gate question was withdrawn as already decided).

### Q2: How much of the boundary does amd64/ABI.md freeze?

| Option | Description | Selected |
|--------|-------------|----------|
| Full exec interface | Ureg offsets + syscall number mapping + arg/return registers + entry state + note-delivery convention — one document, one enforcement surface | ✓ |
| Ureg offsets only | Minimal doc; syscall table stays in sys.h, entry convention lives in r9 code — boundary spread across three places | |

**User's choice:** Full exec interface

---

## Bootfs medium

### Q1: How does the bootfs image reach the guest?

| Option | Description | Selected |
|--------|-------------|----------|
| mb2 module tag | GRUB loads bootfs alongside the kernel from the SAME reproducible ISO (module2 in grub.cfg; mb2 tag walker from 13-02) — one artifact, no new devices, loader-mediated lineage | ✓ |
| Separate disk image | Second qemu -drive — requires virtio-blk, explicitly a Future Requirement; would pull a block driver into scope | |
| qemu -initrd | Direct loader delivery — declined in Phase-13 D-02 (qemu's -kernel loader speaks mb1; GRUB is the loader on this chain) | |

**User's choice:** mb2 module tag

### Q2: Bootfs image format?

| Option | Description | Selected |
|--------|-------------|----------|
| bootfs.paq carried | Reuse the aarch64 packed format + r9 miniz-inflate verbatim — proven, same arch-neutral path, small ISO, reproducible | ✓ |
| Uncompressed flat image | No inflate on the exec-critical path, but a NEW format to build/parse and a much bigger ISO | |
| You decide | Researcher weighs inflate-on-exec risk vs format proliferation | |

**User's choice:** bootfs.paq carried

---

## Compose shape

### Q1: Who builds the composed (kernel + bootfs) ISO?

| Option | Description | Selected |
|--------|-------------|----------|
| r9 ISO builder, Harvey composes | r9 exposes boot-iso parameterized {kernel, modules} — ONE reproducibility implementation; Harvey builds bootfs.paq, calls the builder, owns boot-r9 (INT-03 shape) | ✓ |
| Harvey builds the ISO | Full ownership but forks the reproducible-ISO machinery — two implementations to keep disciplined | |
| xtask profile wrap | boot-r9 wraps cargo xtask with a Harvey config — dev-loop friendly, less nix-pure than INT-05's pin | |

**User's choice:** r9 ISO builder, Harvey composes

### Q2: Where do phase 14's gates live?

| Option | Description | Selected |
|--------|-------------|----------|
| Split by artifact | Kernel-level gates r9-side (xtask); composed-system gates (expect smoke, in-guest /net) Harvey-side where the composed ISO exists — mirrors aarch64 | ✓ |
| All gates in r9 xtask | xtask gains a bootfs-boot profile — one runner, but a Harvey artifact dependency crosses the repo boundary phase 13 kept clean | |

**User's choice:** Split by artifact

---

## Exec staging + entry

### Q1: Exec bring-up staging on x86_64?

| Option | Description | Selected |
|--------|-------------|----------|
| Staged-init carried | Marker exec first (ABI+exec+syscall proven isolated), then boot(8)→init→rc, each leg gated — aarch64 Phase-4/5 discipline | ✓ |
| Straight to full chain | Fewer intermediate gates; first failure conflates ABI/exec/boot-chain bugs | |

**User's choice:** Staged-init carried

### Q2: Where do XEXEC-05's /net re-gates run?

| Option | Description | Selected |
|--------|-------------|----------|
| In-guest expect gates | With the % prompt live: loopback ping/udp/tcp echo + ipconfig + composed smoke as expect-driven in-guest gates Harvey-side; kernel /net stays r9-side | ✓ |
| All kernel-side netgates | Extend the phase-13 netgate shape — proves kernel paths, not the ipconfig/echo user experience the requirement names | |

**User's choice:** In-guest expect gates

### Q3: The hello-equivalent first exec for XEXEC-02?

| Option | Description | Selected |
|--------|-------------|----------|
| Purpose-built marker | Tiny static marker binary (smokesleep.c discipline) — minimal libc surface, fastest bug isolation; real binaries join at boot(8) | ✓ |
| Real tree binary first | /bin/echo from the tier build — proves toolchain output immediately but drags libc init into the first exec | |
| Both, sequenced | Planner sequences marker then an early real binary | |

**User's choice:** Purpose-built marker

---

## Claude's Discretion

(No "you decide" selections — every question resolved to a concrete option. Discretion areas recorded in CONTEXT.md: user-mode substrate details, mb2 module plumbing, bootfs assembly mechanics + CI wiring, tier staging lists, gate/app naming.)

## Deferred Ideas

None — discussion stayed within phase scope.
