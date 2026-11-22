# Phase 4: r9 Userspace-Exec + Harvey↔r9 ABI - Research

**Researched:** 2026-11-01/02
**Domain:** OS kernel greenfield — aarch64 EL0 trap/syscall path, demand-paged user VM, paqfs initrd, cross-repo ABI enforcement
**Confidence:** HIGH (all load-bearing claims read from source this session: Harvey frozen ABI files, r9 trap/vm/xtask code, 9front donor sources at pin 350266104f, crate sources, ARM register docs)

## Summary

Phase 4 turns r9 from a kernel-with-IRQs into a kernel that runs Harvey aarch64 userland: EL0 `svc` dispatch against the frozen wire ABI, a demand-paged first-process path fed by a paqfs initrd, and build-time frame-layout enforcement. Every load-bearing piece has a verified reference: the frozen user-side contract (`aarch64/ABI.md` + `aarch64/include/ureg.h` + `sys.h`) is byte-stable; the 9front donor kernel at pin `350266104f` is the working reference implementation for the syscall loop (`trap.c`), exec (`sysproc.c`), note delivery (`trap.c` `notify`/`noted`), and fault decode (`faultarm64`); r9 already owns the trap-frame plumbing, recursive page tables with user entries, a DTB parser, and the boot-test harness this phase extends.

The three investigations CONTEXT.md assigned are settled: **(D-02)** the paqfs format is fully pinned below at byte level from `mkpaqfs.c`/`paqfs.c` — uncompressed blocks are format-legal two ways (`-u` flag and per-block incompressible fallback), compression is raw RFC 1951 deflate with the adler32 carried out-of-band, and the reader's obligations reduce to header/trailer parse + typed block decode + pointer-block chase; **(D-09)** the icache sequence after text page-in is `DC CVAU` per `CTR_EL0.DminLine`-derived line, `DSB ISH`, `IC IVAU` per `IminLine` line, `DSB ISH`, `ISB`, executed on the **user VA** while the faulting process's TTBR0 is active (no aliasing, no PAN on cortex-a72); **(D-14)** the donor syscall trace needs **no kernel rebuild**: 9front ships `syscallfmt.c` in every kernel and devproc exposes `startsyscall` ctl + `/proc/N/syscall` — an rc loop over those (driven by the Phase-2 `drive9.py` harness) yields the measured syscall set.

**Primary recommendation:** Plan in three waves — (W0) the measurement + falsification spikes (D-14 donor trace of `hello`; qemu `-initrd`/`/chosen` probe; mkpaqfs host-build recipe), (W1) the trap-frame reshape + static checks + SVC dispatch with the measured set (R9-04, R9-02), (W2) initrd/paqfs/devroot + demand-fault exec + notes + scheduler (R9-03, SC#4 smoke) — with execution-gated ordering throughout (no capability lands without a running binary that exercises it).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Init/root-fs delivery
- **D-01:** Root-fs delivery is **qemu `-initrd` + DTB**: r9's xtask assembles a root-fs image from a directory of Harvey aarch64 binaries and passes it via `-initrd`; the kernel reads `linux,initrd-start`/`linux,initrd-end` from `/chosen` in the DTB (`port/fdt.rs` already parses the DTB; `/chosen` parsing is new). Userland changes never rebuild/relink the kernel — the Phase 5 iteration story. Image bytes are pinned artifacts, so gates stay deterministic. — **Reversibility:** reversible — contained to the xtask qemu step and the kernel's initrd discovery; the devroot-facing interface (find file by name, read bytes) is unchanged by any future mechanism swap.
- **D-02:** The initrd image format is **paqfs/bootfs** (Plan 9-native). Harvey's host tool `mkpaqfs` (`sys/src/cmd/paqfs/`) produces the image — the Phase 2 harness already baked `bootfs.paq` with it for the donor kernel; 9front's `bootfs` device is the reference reader shape. r9 grows a Rust `no_std` paqfs reader. **Researcher must pin:** paqfs's per-file compression requirements (libflate/glossary machinery — what must the reader implement), whether uncompressed entries are format-legal, and byte-level format verification against `mkpaqfs` output. — **Reversibility:** costly — reader and producer are both keyed to the format; a format swap touches both repos' build steps.
- **D-03:** The initrd carries the **SC-scoped set only**: Phase 2's `hello` (SC#2), the staged `init` (SC#3), and the smoke-suite binaries (SC#4). Harvey's build drops aarch64 binaries in a known output dir; r9's xtask reaches it via a **TOML/env path** — acknowledged temporary cross-repo glue, replaced by the INT-04 flake input in Phase 5. Do NOT commit Harvey binaries into r9.
- **D-04:** r9's first user process is a **staged minimal init** — a small 7c-compiled Harvey binary named `init` that goes through the real a.out load/exec path but has a scoped job (print via `write`, exercise exec/fork smoke, exit). The full Harvey `/bin/init` (namespace, auth, boot rc) lands in Phase 5 when devices + namespace syscalls exist. No initcode asm stub layer — the direct a.out load exists anyway.

#### Trap-frame + static ABI check (R9-04, SC#1)
- **D-05:** **The trap frame IS Harvey's Ureg, exactly.** r9's `TrapFrame` tail becomes `{sp_el0→sp, elr_el1→pc, spsr_el1→psr, interrupt_type→type}` — 35 × u64, 288 bytes with align(16), unchanged size. `trap.S` grows saves of `SP_EL0` and `SPSR_EL1`, drops the in-frame `ESR_EL1`/`FAR_EL1` stores. ESR/FAR are read live at trap entry and passed to the dispatcher as separate arguments (fault decode only; syscall return never needs them). — **Reversibility:** one-way — this layout is published into `aarch64/ABI.md` (D-07) and pinned by the static checks (D-06); changing it after this phase re-freezes both repos.
- **D-06:** **Two-layer static check.** (1) In-crate Rust const asserts (`mem::offset_of` per field vs pinned byte offsets) — always on, fails `cargo build` even without a Harvey checkout. (2) An xtask check that parses `../harvey/aarch64/include/ureg.h` (field names/order/count) and compares against the crate's offset table, failing build/boot-test on drift — same TOML/env path glue as D-03, cleaned up by INT-04 in Phase 5. A mismatch is a build failure, not a runtime mystery (SC#1 wording).
- **D-07:** The kernel-side contract is written as an **appendix to `aarch64/ABI.md`** (the existing frozen canonical doc — one canonical place per SC#1): the pinned frame-offset table (`r0`@0 … `type`@272), the arg-read formula (`args at ureg->sp+8`, `SP+0` reserved), and how the two-layer check works. Frozen wire-contract text is untouched — the appendix documents enforcement, not new ABI. r9's docs (README/HowItWorks) get a pointer line. — **Reversibility:** one-way — once published, downstream code and checks key off the document.
- **D-08:** **One unified frame for all exceptions.** `trap.S`'s single `handle_interrupt` path saves SP_EL0/SPSR_EL1 always; on EL1 traps those slots hold kernel-context values. The panic dump prints ESR/FAR from the new trap() args — outside the struct — so the virtpanic x0–x30 row-count gate (Phase 3, boot-test asserted) keeps working unchanged. — **Reversibility:** costly — touches every trap path, but contained in `trap.S`/`trap.rs`.

#### First-process + user VM (R9-03, SC#3)
- **D-09:** **Demand-fault everything.** Nothing is mapped at exec: the first instruction fetch faults, and the user-fault handler pages text/data in from the initrd paqfs on demand; stack faults grow the stack segment within a bound. This is the Plan 9 segment model and SC#3's literal wording ("faulting in user text and growing the user stack"). The user-fault handler is this phase's core VM deliverable. **icache maintenance after text load is required** (SC#3 wording — researcher specifies the `dc cvau`/`ic ivau` + barrier sequence for the fault-in path, per Phase 2 D-03's architectural-discovery rule: use `CTR_EL0`, not hardcoded geometry). — **Reversibility:** costly — the fault handler and loader intertwine; an eager variant would be a rewrite of the exec path.
- **D-10:** **TTBR0 identity map torn down at first exec** (closes Phase 3 review item WR-01). The first user process's root table replaces the identity map in TTBR0 (`vm::switch(_, RootPageTableType::User)` exists but is raspi-gated — ungate on virt); every later process switch swaps TTBR0. Verified during discussion: all runtime MMIO (GIC, PL011) already maps via `map_device_register` into the TTBR1 kernel table (`KZERO + 0x100000000000` upward) — the identity map has no runtime consumer. The early-boot identity window remains for the pre-MMU→MMU transition only.
- **D-11:** **r9-specific user address-space policy** (user decision, overriding the 9front-convention recommendation): r9 owns stack-top placement, segment/growth policy, and the initial stack frame (argc/argv construction for libc's `main9.s` crt0). **Hard constraint for the planner:** text/data VAs are fixed inside the a.out headers by 7l (no relocation exists) — the loader maps text/data where the headers say; "r9-specific" governs stack top and segment policy only. — **Reversibility:** costly — baked into the loader, seeded frames, and any user-visible assumptions; revisit before Phase 5 if libc surprises surface.
- **D-12:** **Remove `test_sysexit` and the raspi user-table experiments from `main9`** when the real exec path lands (git history preserves them). Phase 3 CONTEXT flagged them "not a deliverable"; the experiment never `eret`s to EL0 and must not be mistaken for the real path.

#### Process model scope (R9-02, SC#4)
- **D-13:** **Tick-preemptive round-robin** over a small fixed process table — the existing generic-timer IRQ reschedules, so IRQ-from-EL0 + return-to-a-different-process is exercised this phase rather than discovered in Phase 5. Boot-test asserts markers, not byte-order (interleaving is nondeterministic). **Derived requirement:** preemption forces **per-process kernel stacks** — a process blocked mid-syscall cannot share the single interrupt stack. — **Reversibility:** costly — scheduler shape touches trap return, timer handler, and the process table.
- **D-14:** **Measure-first syscall set.** The researcher traces the exact syscall footprint of `hello` + the staged `init` + the smoke binaries on the 9front donor kernel (qemu gdb / kernel instrumentation — the Phase 2 harness makes this cheap), then r9 implements exactly that set **plus the SC#4-named rfork/notes/wait core** (RFORK=19, NOTIFY=28, NOTED=29, AWAIT=47, EXITS=8, EXEC=7 + the fd-ops set). Zero guessed syscalls; each has a measured caller. Watch: fmt/pool allocator may pull `BRK_`(24)/`SEGBRK`(12) into hello's footprint — the trace settles it. The ABI table stays 0–54 lazy: unimplemented numbers return a loud error, never a silent wedge.
- **D-15:** Device set for EL0 fds: **devroot + devcons + devproc-lite**. devroot serves the initrd paqfs as the root file tree (walk/open/read/stat) — exec sources binaries through the same devroot path, one mechanism. devcons provides console read/write with **fd 0/1/2 preopened at exec** per Plan 9 convention. devproc-lite (below) exists only for the note channel.
- **D-16:** **Parent-posts-child notes** (user decision, overriding the alarm round-trip recommendation). Verified against the frozen `sys.h` (0–54): **no postnote syscall exists** — `postnote(2)` is a libc function writing `/proc/N/note`, so the device set extends to **devproc-lite exposing ONLY `/proc/N/note`** (walk/open/write) — libc `postnote(PNPROC, pid, msg)` works unmodified. Smoke shape: parent rforks child → child `notify()`+spin → parent `postnote` → delivery at the child's return-to-EL0 boundary (pc redirected to the handler, note string on the user stack) → `noted()` resumes → child exits → parent `await`s. Exercises RFORK/NOTIFY/write/delivery/NOTED/EXITS/AWAIT end-to-end. — **Reversibility:** costly — note delivery semantics are wired into trap return and devproc-lite's shape.

#### Carried forward (locked in roadmap/prior phases — do not re-litigate)
- Wire ABI frozen (Phase 2 D-15/16/17): number in R0, args at `SP+8` (8-byte slots, `SP+0` reserved LR slot), `SVC`, result in R0; table = Harvey `sys.h` 0–54 verbatim, NMOUNT lazy. Kernel-side dispatch style does not cross the wire (D-17).
- `hello.7.out` (committed, `build/aarch64/hello.7.out`, sha256-verified on the donor kernel) is the ABI bisect point: runs on 9front but not r9 ⇒ bug is in r9.
- Proof standard: execution evidence captured over serial, asserted by `cargo xtask boot-test` (zero-dep unix-socket harness from Phase 3), human checkpoint at the gate; canonical evidence consolidated in Harvey `.planning/phases/04-*/` with the r9 commit SHA recorded (Phase 3 D-13).
- qemu shape: `-M virt,gic-version=2 -cpu cortex-a72 -smp 1 -nographic`, ELF `-kernel`, DTB at `0x40000000`; TCG on darwin (HVF rejects GICv2 on qemu 11.1.2); `-m ≤ 1G` (pagealloc bitmap ceiling).
- r9 commits land on `main` of `requaos/r9`; HowItWorks.md updated with material changes (its own header mandates it); platform-tree + TOML-as-data conventions from Phase 3 hold.
- `aarch64` naming everywhere; r9 code follows r9's `rustfmt.toml`; Harvey's Plan 9 C style rules do not cross the repo boundary.
- virtio-9p is v1.x — it is NOT this phase's root-fs answer (REQUIREMENTS.md v2 list).
- Apple host 16K pages / guest granule + `CTR_EL0` architectural discovery (Phase 2 D-03) apply to any new cache/page maintenance r9 grows.

### the agent's Discretion
- Process table size and Proc struct field layout (fixed small array; scheduler-visible state only).
- First-entry mechanics (e.g. seeding a Ureg on the new process's kernel stack so the normal trap-return path `eret`s into EL0 — the Plan 9 forkret shape), subject to D-05's frame layout.
- Where the user-fault handler, a.out loader, paqfs reader, and devroot/devcons/devproc-lite live in the r9 crate tree (port vs aarch64) — follow the Phase 3 platform-tree conventions.
- `await` wait-message format — match Plan 9's `"pid ..."` convention closely enough for the smoke; full fidelity is Phase 5's concern.
- Timer tick rate / time-slice length for the round-robin.
- How the xtask check surfaces drift (error text, exit code) — must fail the build/boot-test, exact UX is free.
- Whether the smoke suite is one binary or several (7c-compiled, committed via the D-03 initrd path).

### Deferred Ideas (OUT OF SCOPE)
- virtio-9p host mount (collapses userland iteration time) — v1.x, top candidate per REQUIREMENTS.md
- Full devproc (`/proc/N/{ctl,status,…}`), devenv, namespace syscalls for the real `/bin/init` — Phase 5
- SMP via PSCI, virtio-net/blk, RTC, PCIe ECAM — v2
- raspi user-table/experiment revival — superseded by the real path (D-12); git history preserves
- HVF acceleration for virt boots — needs a GICv3 driver (qemu 11.1.2 rejects GICv2 under HVF); future work
- Upstream contribution flow to original r9 — fork `requaos/r9` remains the working repo
- 9k-amd64 pool-panic re-test with the LP64-fixed 6c (phase-02 deferred-items.md #8) — amd64-side debt, not this phase
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| R9-02 | r9 process model + `svc` syscall dispatch sufficient for userland (EL0 entry/return, trap frame matching Plan 9 ureg conventions) | "Trap-frame reshape" and "SVC dispatch" patterns (D-05/D-08 frame, SP_EL0/EL1h subtleties, result writeback via frame `x0`); donor `syscall()`/`dosyscall` reference quoted; process model pattern (per-process kernel stacks, forkret-shaped first entry, tick round-robin); D-14 measured syscall set method |
| R9-03 | r9 first-process path loads and runs a Harvey aarch64 `init` (embedded root-fs or equivalent delivery), faulting in user pages | "First-process exec" pattern (a.out parse numbers verified against `hello.7.out` bytes, execregs stack seeding, forkret entry); "Demand-fault handler" pattern (fault-status decode table, paqfs-backed page-in, D-09 icache sequence); initrd delivery mechanics (D-01/D-02 pinned: `-initrd`→`/chosen`, paqfs byte format) |
| R9-04 | Harvey↔r9 syscall ABI contract documented and enforced by static checks (cross-repo layout match verified at build time) | D-06 two-layer check design (const-assert offsets table + xtask ureg.h parse), D-07 appendix contents (pinned offset table r0@0…type@272, arg formula `ureg->sp+8`), verbatim ureg.h/sys.h quotes below as the ground truth to check against |
</phase_requirements>

## Architectural Responsibility Map

This phase spans two repos and a host harness; "tier" here names the owning layer. Getting these assignments wrong is the classic multi-tier failure mode for this phase (e.g. putting syscall semantics in userland, or paqfs production inside r9's kernel build).

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Syscall wire ABI (numbers, arg slots, result) | Frozen contract (Harvey `aarch64/ABI.md`) | — | Already frozen Phase 2; this phase does NOT change it (D-07 appendix is enforcement docs only) |
| Trap-frame layout + EL0 entry/return asm | r9 kernel (`aarch64/src/trap.S`, `trap.rs`) | — | D-05/D-08; must match frozen `ureg.h` field-for-field |
| Syscall dispatch + handlers (measured set) | r9 kernel (`port` or `aarch64` crate) | donor 9front (semantics reference) | D-14 measured set; r9 mimics neither Harvey 9k nor 9front dispatch style (D-17) |
| Process model (table, scheduler, per-proc kstacks) | r9 kernel | — | D-13; swtch.rs reused for kernel-context switches |
| User VM policy (stack top, growth bounds, initial frame) | r9 kernel (D-11) | 7l fixes text/data VAs (hard constraint) | r9 owns stack/segment policy only; text/data VAs come from a.out headers |
| Demand-fault page-in + icache maintenance | r9 kernel (`vm.rs` + fault path) | — | D-09; runs in the faulting process's TTBR0 context |
| a.out loading (parse, segment map) | r9 kernel | Harvey `sys/include/a.out.h` (format truth) | 8-word BE header, R_MAGIC=0x8C47; verified against `hello.7.out` bytes |
| paqfs image production | Host tooling (Harvey `mkpaqfs` via plan9port 9c) | r9 xtask (invocation + path glue) | D-02/D-03; producer is Plan 9 C, built like Phase 2 host tools |
| paqfs reading (walk/open/read) | r9 kernel (Rust no_std reader) | — | D-02; reference reader = Harvey `paqfs.c` `blockRead`/`paqWalk` (see format spec) |
| initrd discovery (`/chosen` parse) | r9 kernel (`port/fdt.rs` extension) | qemu (writes `linux,initrd-start/end`) | D-01 |
| devroot / devcons / devproc-lite | r9 kernel (`port/dat.rs` scaffolding grows real impls) | — | D-15/D-16 |
| ABI static checks | r9 (const asserts) + r9 xtask (cross-repo parse) | Harvey `ureg.h` (parsed, read-only) | D-06 |
| Syscall footprint measurement (D-14) | Donor harness (Harvey `build/aarch64/` + lima guest) | — | runs BEFORE implementation; produces the measured syscall list artifact |
| Smoke/init binary sources | Harvey repo (7c-compiled) | r9 xtask (initrd assembly via TOML/env path) | D-03/D-04; agent's discretion: exact dir |

## Standard Stack

### Core (r9 kernel — Rust no_std, pinned nightly-2026-01-30 per `rust-toolchain.toml`)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `aarch64-cpu` | 11.2.0 (already in `Cargo.lock`) | Sysreg wrappers: `SP_EL0`, `SPSR_EL1`, `ELR_EL1`, `FAR_EL1`, `ESR_EL1`, `CSSELR_EL1`, `CCSIDR_EL1`, `CLIDR_EL1` all present | Already r9's register crate; verified this session by downloading and grepping the crate source [VERIFIED: crates.io source of aarch64-cpu 11.2.0] |
| inline `asm!` | — | `CTR_EL0` read (NOT in aarch64-cpu 11.2.0 — verified absent from crate source), `DC CVAU`/`IC IVAU`, barrier ops | r9 already uses inline asm for `ttbr0/1` reads in `vm.rs`; same pattern [VERIFIED: crates.io source probe this session] |
| `bitstruct` | 0.1 (already) | `EsrEl1` bitfield decode (existing); extend with data-abort ISS decode | Existing `registers.rs` pattern [VERIFIED: r9 `registers.rs:84-213`] |
| `num_enum` | 0.7 (already) | `ExceptionClass`, fault-status enums (existing) | Same |
| `miniz_oxide` | 0.9.1, `default-features = false, features = ["with-alloc"]` | Raw RFC 1951 inflate for paqfs `DeflateEnc` blocks (`inflate::decompress_to_vec_with_limit(cdat, blocksize)`) | The no_std-capable, pure-safe-Rust inflate; docs.rs states "The library is fully no_std" [CITED: docs.rs/crate/miniz_oxide/0.9.1] |
| `adler2` | 2.x | paqfs block adler32 verification (checksum of the DECODED block vs block header) | miniz_oxide already depends on it (zero added tree weight); no_std, no unsafe [CITED: docs.rs miniz_oxide deps list] |

### Supporting (host side)
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| Harvey `mkpaqfs` (host build) | in-tree `sys/src/cmd/paqfs/` | Produce the initrd paqfs image from a directory of aarch64 binaries | Every boot-test/qemu invocation that carries userland (D-01/D-03). Build recipe proven in Phase 2: plan9port `9c -O0 -I<paqfs dir> -c mkpaqfs.c` + `9l` against flate/mp/sec (`build/aarch64/donor-ns-build.sh:240-243` built the donor's copy this way) [VERIFIED: donor-ns-build.sh lines read this session] |
| plan9port (`9c`/`9l`, libflate/libmp/libsec) | host | Compile mkpaqfs | Same as Phase 2; `$PLAN9/bin` LAST in PATH per project rules |
| qemu-system-aarch64 | 11.1.2 (flake-pinned) | `-initrd` delivery + boot-test | Carried forward; unchanged version means CONTEXT.md's qemu facts stand |
| `cargo xtask` | in-repo | initrd assembly, `-initrd` qemu arg, ureg.h cross-check, smoke boot-test profiles | D-01/D-03/D-06 integration points (see Patterns) |
| Donor kernel trace rig | 9front pin `350266104f` in lima guest | D-14 syscall measurement via `startsyscall` + `/proc/N/syscall` | Before ANY syscall is implemented (Wave 0) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `miniz_oxide` inflate | hand-rolled inflate | Hand-rolling RFC 1951 is ~500 lines with nasty edge cases (dynamic Huffman, window wrap); no win |
| `miniz_oxide` | `flate2` | flate2 is a std-oriented wrapper over miniz_oxide; use the backend directly |
| `adler2` | 12-line local adler32 | Adler32 is spec-trivial (RFC 1950); local impl acceptable, but adler2 is already in the dep tree via miniz_oxide — using it directly is zero-cost and tested |
| qemu `-initrd` (D-01) | kernel-embedded blob (`include_bytes!`) | D-01 locked: initrd keeps userland iteration from relinking the kernel. Blob is the fallback if the Wave-0 `/chosen` probe fails |
| donor `startsyscall` trace (D-14) | donor kernel rebuild with a `print` in `dosyscall` | The print patch is the documented fallback (needs a donor rebuild via `donor-ns-build.sh`); startsyscall needs no rebuild and is tried first |
| paqfs (D-02, locked) | custom table-of-files format | User override: paqfs is Plan 9-native fidelity; CONTEXT specifics say don't second-guess |

**Installation:**
```bash
# r9 repo, aarch64 crate Cargo.toml additions:
miniz_oxide = { version = "0.9", default-features = false, features = ["with-alloc"] }
adler2 = "2"
# Harvey repo: mkpaqfs host build (new small BUILD recipe, modeled on donor-ns-build.sh:240-243)
```

**Version verification:** crate versions verified against crates.io/docs.rs this session (miniz_oxide 0.9.1 published 2026-03-13; adler2 current; aarch64-cpu 11.2.0 already locked). mkpaqfs is in-tree Harvey source — no registry involved.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| miniz_oxide | crates.io | 8+ yrs (first publish 2017-10-10) | ~19.8M/wk | github.com/Frommi/miniz_oxide | OK (seam) | Approved — docs.rs API confirmation + seam OK |
| adler2 | crates.io | maintained fork (2024-08 publish of current) | ~15.2M/wk | github.com/oyvindln/adler2 | OK (seam) | Approved — already a transitive dep via miniz_oxide |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

*Both packages were named from training knowledge and then confirmed: seam verdict OK + docs.rs official documentation of the exact API surface used (`miniz_oxide::inflate::decompress_to_vec_with_limit`, no_std feature flags). Per protocol this earns the approval; no `checkpoint:human-verify` needed for these two. No other external packages are introduced by this phase's research.*

## Frozen Contract Ground Truth (verbatim — the values the static checks pin)

These are the in-repo discrete values the plan's code must match. Quoted verbatim from files read this session.

### The trap frame: `aarch64/include/ureg.h` (Harvey, frozen) — 35 × u64int, sizeof=280

[VERIFIED: harvey `aarch64/include/ureg.h:1-40` — read this session, quoted verbatim]

```c
typedef struct Ureg {
	u64int	r0;
	…
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
(Field order: `r0`…`r28`, `r29`, `r30`/`link` union, `sp`, `pc`, `psr`, `type` — the union occupies one u64 slot. The 9front `arm64/include/ureg.h` at pin 350266104f is byte-identical — fetched and compared this session [VERIFIED: raw.githubusercontent.com/9front/9front/350266104f…/arm64/include/ureg.h].)

**Pinned byte offsets (index×8):** `r0`@0 … `r28`@224, `r29`@232, `r30`/`link`@240, `sp`@248, `pc`@256, `psr`@264, `type`@272; size 280, `align(16)` → 288. This matches D-07's "r0@0 … type@272" and D-05's "35 × u64, 288 bytes".

r9's CURRENT `TrapFrame` (r9 `trap.rs:37-74`, read this session): `x0`…`x28`, `frame_pointer` (x29), `link_register` (x30), then `esr_el1`@248, `elr_el1`@256, `far_el1`@264, `interrupt_type`@272. The D-05 reshape keeps every offset: x29/x30 keep their slots, and the tail swaps *sources* not *offsets* — sp←SP_EL0 @248, pc←ELR_EL1 @256, psr←SPSR_EL1 @264, type stays @272. Size unchanged (288). The existing `sub sp, sp, #288` in `trap.S:57` and the `#16*15..17` store slots already land on these offsets [VERIFIED: r9 `trap.S:51-91` read this session].

### The syscall number table: `sys/src/libc/9syscall/sys.h` (frozen, 0–54)

[VERIFIED: harvey `sys/src/libc/9syscall/sys.h:1-53` — read this session; key lines verbatim]

```c
#define	ALARM		6
#define	EXEC		7
#define	EXITS		8
#define	SEGBRK		12
#define	OPEN		14
#define	RFORK		19
#define	_WRITE		20
#define	BRK_		24
#define	NOTIFY		28
#define	NOTED		29
#define	MOUNT		46
#define	AWAIT		47
#define	PREAD		50
#define	PWRITE		51
#define	TSEMACQUIRE	52
#define NSEC		53
#define NMOUNT		54
```
(48–49 are a gap; 0–52 wire-identical to 9front, `NMOUNT` Harvey-only and lazy. There is **no POSTNOTE** — postnote is libc writing `/proc/N/note` [VERIFIED: harvey `sys/src/libc/9sys/postnote.c` — full file read this session: PNPROC→`/proc/%d/note`, PNGROUP→`/proc/%d/notepg`, open OWRITE + write + close].)

### The user stub shape: `sys/src/libc/9syscall/mkfile` `case aarch64`

[VERIFIED: harvey `sys/src/libc/9syscall/mkfile:96-103` — read this session, verbatim]

```
case aarch64
	j=$i
	if(~ $i seek) j=_seek
	echo TEXT $j'(SB)', 1, '$0'
	echo MOV R0, '0(FP)'
	echo MOV '$'$n, R0
	echo SVC
	echo RETURN
```
plus the mkfile-level quirk `if(~ $i exits) i=_exits` (line 9). No stub-side buffer store. The stub's `MOV R0, 0(FP)` is why the first kernel-visible argument sits at user `SP+8` (SP+0 = the materialized arg0 slot the kernel never reads as an argument — ABI.md documents the kernel formula `(Sargs*)(ureg->sp+8)`).

### crt0 expectations: `sys/src/libc/aarch64/main9.s` + `9sys/callmain.c`

[VERIFIED: harvey `sys/src/libc/aarch64/main9.s:1-9` and `sys/src/libc/9sys/callmain.c:13-23` — read this session, verbatim]

```
TEXT	_main(SB), 1, $0
	SUB	$16, SP
	MOV	$setSB(SB), R28
	MOV	R0, _tos(SB)
	MOV	$main(SB), R0
	MOV	$0, R30
	B	_callmain(SB)
```
```c
void
_callmain(void (*main)(int, char**), int argc, char *arg0)
{
	…
	main(argc, &arg0);
	exits("main");
}
```
So the exec-time user frame r9 must seed (D-11) is exactly 9front's `execregs` contract [CITED: 9front `sysproc.c` `execregs` at pin, fetched this session]:
```c
uintptr
execregs(uintptr entry, int argc, char *argv[], Tos *tos)
{
	uintptr *sp = (void*)argv;
	Ureg *ureg;
	*--sp = argc;
	ureg = up->dbgreg;
	ureg->sp = (uintptr)sp;
	ureg->pc = entry;
	ureg->link = 0;
	return (uintptr)tos;
}
```
i.e. EL0 entry state: **R0 = tos pointer** (stored into `_tos(SB)` by crt0; see Tos note below), **SP → [argc][argv0][argv1]…[nil]**, **PC = entry** (from the a.out header), **LR = 0**, **PSR = EL0t** (0 — 9front `touser` writes SPSR=0 [CITED: 9front `l.s` `touser`, fetched this session]).

**Tos note:** Harvey's `Tos` (`sys/include/tos.h`, read this session) is only dereferenced by nsec/cycles/profiling paths; `getpid()` on Harvey reads `#c/pid` — a devcons file, NOT Tos [VERIFIED: harvey `sys/src/libc/9sys/getpid.c` — read this session]. Safe minimum: map one zeroed user page for Tos at r9's chosen stack-top-minus-`sizeof(Tos)` and pass its VA in R0 (cheap insurance; policy is r9's per D-11). Whether the smoke needs `#c/pid` in devcons is settled by the D-14 trace (Open Question OQ-2).

### Note-path constants (D-16)

[VERIFIED: harvey `sys/include/libc.h:556,585-588` — read this session, verbatim]
```c
#define	ERRMAX	128	/* max length of error string */
#define	NCONT	0	/* continue after note */
#define	NDFLT	1	/* terminate after note */
#define	NSAVE	2	/* clear note but hold state */
#define	NRSTR	3	/* restore saved state */
```

### The fd syscalls are PREAD/PWRITE, not _READ/_WRITE

Harvey libc `write(fd,buf,n)` = `pwrite(fd, buf, n, -1LL)` and `read` = `pread(…, -1LL)` [VERIFIED: harvey `sys/src/libc/9sys/write.c`,`read.c` — read this session]. Offset `-1` means "use the channel's offset" (normal streaming read/write). r9's PWRITE/PREAD handlers must special-case offset `-1` (u64 all-ones). The `_WRITE`(20)/`_READ`(15) entries exist in the table but libc's `write`/`read` do not call them.

### a.out header (the exec format) — verified against the committed binary

[VERIFIED: harvey `sys/include/a.out.h:2-12,31-33` — read this session]
```c
struct	Exec
{
	long	magic;		/* magic number */
	long	text;	 	/* size of text segment */
	long	data;	 	/* size of initialized data */
	long	bss;	  	/* size of uninitialized data */
	long	syms;	 	/* size of symbol table */
	long	entry;	 	/* entry point */
	long	spsz;		/* size of pc/sp offset table */
	long	pcsz;		/* size of pc/line number table */
};
…
#define	R_MAGIC		_MAGIC(HDR_MAGIC, 28)	/* arm64 */
```
All fields big-endian on disk (donor `sysexec` byte-swaps with `beswal`/`beswav` [CITED: 9front sysproc.c at pin]). `R_MAGIC` = 0x8000 | (4·28·28+7) = **0x8C47**; because HDR_MAGIC is set, the header is 40 bytes: 8×u32 then a **u64 entry point at bytes 32..39** (the u32 `entry` field is vestigial on HDR_MAGIC arches; text follows at file offset 40).

Concrete proof from the committed bisect binary [VERIFIED: `xxd` of harvey `build/aarch64/hello.7.out` this session; sha256 `1c89e0990e386ae503d9f513d3a6c1985222f772d1edb40016c19aa3881abb04` matches the Phase-2 gate evidence]:
```
00000000: 0000 8c47 0000 4b68 0000 0aa0 0000 0620   magic=R_MAGIC, text=0x4b68, data=0xaa0, bss=0x620
00000010: 0000 2fc5 0001 0044 0000 0000 0000 0b38   syms=0x2fc5, entry32=0x10044, spsz=0, pcsz=0xb38
00000020: 0000 0000 0001 0044 ...                  hdr64: entry=0x10044
```
## D-02 Deliverable: paqfs Format Specification (pinned)

Everything below is verified against Harvey's `sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c,paqfs.c}` (all three read in full this session) and `sys/man/{4/paqfs,8/mkpaqfs}` (read this session). All multi-byte integers on disk are **big-endian**.

### Image layout
```
[44-byte header][block][block]…[block][28-byte trailer]
```
- **Header** (44 bytes) [VERIFIED: paqfs.h:6-19,34-41 + mkpaqfs.c:405-420]:
  - small form (blocksize < 65536): `magic u32 = 0x529AB12B` @0, `version u16 = 1` @4, `blocksize u16` @6, `time u32` @8, `label char[32]` @12.
  - big form: u16 `0x25A9` @0 (BigHeaderMagic), `version u16` @2, `blocksize u32` @4 — then same time/label. (mkpaqfs default blocksize = 4096 → small form. Reader may assert small-form + magic and fail loudly otherwise.)
- **Trailer** (28 bytes, last 28 bytes of the image): `magic u32 = 0x6B46E688` @0, `root u32` @4 = **byte offset of the root DirBlock**, `sha1[20]` @8 over the entire preceding stream (running digest; verification optional — paqfs only checks it under `-v`).
- **Block** = 12-byte header + payload [VERIFIED: paqfs.h:43-50 + mkpaqfs.c:431-444 + paqfs.c:942-987]:
  - small form (payload size < 65536): `magic u32 = 0x198A1CBF` @0, `size u16` @4; big form: u16 `0x91A8` @0 + `size u32` @2. Then `type u8` @6, `encoding u8` @7, `adler32 u32` @8 = adler32 of the **decoded** payload.
  - `type`: `DirBlock=0`, `DataBlock=1`, `PointerBlock=2`. `encoding`: `NoEnc=0`, `DeflateEnc=1`.
  - Payload length on disk: `size` bytes for DeflateEnc; **`blocksize` bytes always for NoEnc** (mkpaqfs.c:365 sets `bh.size = blocksize` and only overwrites it when compression wins; the reader `paqfs.c:966-968` reads a full blocksize for NoEnc).

### Compression requirements — the D-02 question, answered
- **Uncompressed entries are format-legal, twice over**: (a) `mkpaqfs -u` writes every block `NoEnc` (mkpaqfs.c:34,49-51,371-379: `if(!uflag)` guards the deflate attempt); (b) even without `-u`, any block that doesn't compress smaller is written `NoEnc` (`if(n > 0 && n < blocksize)` gate at mkpaqfs.c:374). The reader MUST therefore support NoEnc regardless of producer flags.
- **The compression is raw RFC 1951 deflate, NOT zlib/gzip**: mkpaqfs calls libflate's `deflateblock` (mkpaqfs.c:373); Harvey's `sys/include/flate.h` (grepped this session) declares `deflateblock` and `deflatezlibblock` as distinct entry points, and `deflatezlibblock.c:15` builds the zlib variant by wrapping `deflateblock` output with the 2-byte zlib header + 4-byte adler trailer — paqfs does neither; the adler32 lives out-of-band in the block header. The reader decodes with raw inflate (`inflateblock` on the C side, paqfs.c:976) → **`miniz_oxide::inflate` (the raw-deflate module), never the zlib/gzip paths.**
- mkpaqfs compresses at level 6 (mkpaqfs.c:373) — irrelevant to the reader.
- **Format-fidelity choice for the planner:** either (a) bake initrds with `mkpaqfs -u` and the Rust reader hard-fails on DeflateEnc (smallest reader, deterministic, slightly bigger image), or (b) reader supports both encodings via miniz_oxide. Recommend (b) — miniz_oxide is cheap, and baked images from other Plan 9 tooling (the donor's own bootfs.paq) are deflated; but (a) is a legitimate Wave-0 descope if inflate bring-up stalls. Either way, `-u` vs default must be a deliberate mkpaqfs invocation flag in the xtask step, and the reader must reject unknown encodings loudly (paqfs.c:963-965 does `default: return 0`).

### Directory and file structure
- **PaqDir entry** (inside DirBlock payloads) [VERIFIED: paqfs.h:59-69 + mkpaqfs.c:400-403,446-461 + paqfs.c:1175-1194]:
  `size u16` @0 (total entry size incl. strings), `qid u32` @2, `mode u32` @6, `mtime u32` @10, `length u32` @14, `offset u32` @18 (byte offset of this entry's **PointerBlock**), then three length-prefixed strings: name, uid, gid — each `u16 n = strlen+2` then `n-2` bytes. MinDirSize = 28 + string bytes. Entries pack back-to-back within a block; a zero `size` field / block-fill terminates the block's entries (paqfs.c `checkDirSize` walks until the size prefix fails bounds; mkpaqfs zero-pads).
- **PointerBlock**: array of `u32` BE block byte-offsets, one per data/dir block, **zero-padded** (mkpaqfs `emallocz`) — readers stop at the first 0 offset (paqfs.c:677 `if(addr == 0) return nil`, 753-754 break). Max blocks per file = blocksize/4 (one pointer block — no indirect blocks).
- **Directory walk** (reader algorithm, from paqfs.c `paqWalk` 730-779): read dir's PointerBlock → for each DirBlock offset → scan PaqDir entries matching `name` → recurse. Root: trailer.root → DirBlock → first PaqDir is the root dir entry (hand-crafted into the fs root by paqfs.c:914-938).
- **File read** (paqfs.c:531-540 shape): file's PaqDir.offset → PointerBlock → DataBlock[i] holds file bytes [i·blocksize, (i+1)·blocksize) of the DECODED stream; truncate the final read to PaqDir.length.
- **Limits**: file length and all offsets are u32 → ≤ 4 GiB images; directory/file block counts bounded by blocksize/4 (≥1024 blocks at 4K blocksize).
- **sha1 trailer verification** is optional for r9 (paqfs -v semantics); recommend skipping in the kernel reader but DOING a one-shot byte-level verification in xtask/host tests (below).

### What the Rust no_std reader must implement (minimal walk/open/read)
1. `open_image(bytes: &[u8])`: parse header (magic/version/blocksize), read trailer at `len-28` (magic), keep `root` offset. The initrd is memory-resident (qemu loads it into RAM), so "block read" = slice indexing at a byte offset — no I/O layer needed.
2. `read_block(offset, expect_type) -> [u8; blocksize]`: parse 12-byte header, check magic/type/size ≤ blocksize; NoEnc → copy `blocksize` bytes; DeflateEnc → `miniz_oxide::inflate::decompress_to_vec_with_limit(&img[off+12..off+12+size], blocksize)` and assert `out.len() == blocksize`; verify adler32(decoded) == header field (`adler2::Adler32`).
3. `walk(path)`: pointer-block chase + PaqDir scan per above (names are the 8.3-free Plan 9 names from mkpaqfs's input tree).
4. `read_at(dir_entry, off, buf)`: pointer-block → DataBlocks → memcpy window; clamp to `length`.
5. Host-side verification task (Wave 0, runs on the dev host under `cargo test` with std): feed `mkpaqfs`-produced bytes (a fixture image baked by the Phase-2-proven host tool) through the reader and assert walk/read of known files — this is D-02's "byte-level format verification against mkpaqfs output". The reader crate must be `no_std` but testable on host (r9's `port` crate already has this pattern: `#[cfg(test)]` std tests).

### Reference-reader correction (context fidelity note)
CONTEXT.md D-02 names "9front's `bootfs` device" as the reference reader shape. Verified this session against the pinned donor tree listing (GitHub tree API at 350266104f): **9front has no in-kernel paqfs/bootfs reader at this pin** — `sys/src/9/boot/bootfs.proto` is an mkfs proto (a different artifact), and the donor kernel mounts the paq via the *userspace* paqfs (that's the upstream-broken `local!paqfs` path the Phase-2 harness routed around). The actual reference reader is the userspace `paqfs.c` (`blockRead`/`blockLoad`/`paqWalk`) — fully analyzed above and equivalent for r9's purposes. No plan impact; noted so the planner doesn't hunt for a nonexistent kernel device.

## D-14 Deliverable: Donor-Kernel Syscall Trace Method (pinned)

**Primary method — zero donor rebuild:** 9front ships a built-in per-syscall tracer, compiled into every kernel (verified this session: `sys/src/9/port/syscallfmt.c` exists at the pin and `sys/src/9/port/portmkfile` builds `PORTFILES` = `mkfilelist` glob of ALL `port/*.c`; `syscallfmt.$O` is explicitly dependency-listed). The pieces, all verified in donor sources fetched this session:

- `dosyscall` (`sys/src/9/port/sysproc.c`, at pin): when `up->procctl == Proc_tracesyscall`, it calls `syscallfmt(scallnr, userpc(), args)` (formats `"pid text syscallname pc args…"` into `up->syscalltrace`), then sets `Proc_stopme` and stops the process. On resume it executes the syscall, and — because the debugger re-armed `Proc_tracesyscall` while the process sat stopped — the tail block calls `sysretfmt` (formats `" = ret errstr start stop"` into `syscalltrace`) and stops again.
- devproc (`sys/src/9/port/devproc.c`, at pin) exposes: ctl command **`startsyscall`** (`CMstartsyscall`: requires the target Stopped, sets `procctl=Proc_tracesyscall`, readies it, then `procstopwait`s — the write blocks until the target's next stop), and file **`/proc/N/syscall`** (`Qsyscall`, mode 0400): reads return the current `syscalltrace` string.

**The trace recipe** (guest rc, driven from the host by the existing Phase-2 `drive9.py` send-on-match serial harness):
```rc
# target binary available in the guest (baked into CONF=qemu bootdir à la run-gate.sh,
# or served by srv9.py): launch it, stop it, then single-step syscalls:
/boot/hello &
pid = $apid          # or read from /proc — harness-side detail
echo stop > /proc/$pid/ctl
while (true) {
    echo startsyscall > /proc/$pid/ctl   # blocks until the target's next stop
    cat /proc/$pid/syscall               # exactly one fragment per stop
}
# loop breaks with Eprocdied after the target's EXITS — clean termination signal
```
Each loop iteration captures one fragment; fragments **alternate entry/return** per syscall (entry fragment after the pre-exec stop, return fragment after the post-exec stop). The serial-captured log is parsed host-side into the ordered, deduplicated syscall-name set — that IS the "measured footprint" artifact (store it in the phase evidence dir with the r9/SHA cross-reference).

Candidates to expect (NOT a substitute for the trace): `hello` = `PWRITE`(51) via `print`→`write`→`pwrite(fd,…,-1)`, `_exits`→`EXITS`(8); possibly `BRK_`(24) if fmt/pool allocates (`sbrk`→`brk_` verified: harvey `sys/src/libc/9sys/sbrk.c` read this session), plus crt0 has no syscalls (`main9.s` is pure asm). The staged `init` and smoke binaries are Phase-4-produced Harvey binaries — they run on the donor as-is (that's the point of the ABI bisect point), so trace them the same way as they're written, BEFORE writing the matching r9 handler.

**Fallback (if `/proc/N/syscall` surprises):** patch donor `dosyscall` with an unconditional `print("%ld %s\n", scallnr, sysctab[scallnr])`, rebuild via `build/aarch64/donor-ns-build.sh` (Phase-2 harness makes this cheap), boot, run targets, read the console log. Also note `syscallfmt.c`'s own header comment "WE ARE OVERRUNNING SOMEHOW" — a known-upstream cosmetic caveat; the name/number stream is still sound.

**Not recommended:** raw-RSP gdb single-stepping (gdbpoke9.py exists but symbol-less memory poking is the wrong granularity for a per-syscall census) and qemu `-d` tracing (no syscall-level semantics for SVC args).

## D-09 Deliverable: Icache/Cache-Maintenance Sequence for Text Page-In (pinned)

**When:** after EVERY text page-in (initial exec and every demand fault that populates a text page — CONTEXT watch-out), and after any write to memory that will subsequently be fetched as instructions at EL0.

**Where (the VA question, resolved):** the user-fault handler runs in the faulting process's address space (its TTBR0 is live — that's what faulted). Write the page contents and do all maintenance on the **user VA** directly. This sidesteps kernel↔user VA aliasing entirely. cortex-a72 is ARMv8.0-A — no PAN (Privilege Access Never is an ARMv8.1 feature), so EL1 can freely read/write user-mapped pages [ASSUMED — standard architecture knowledge; risk-if-wrong: a permission fault in the handler, immediately visible; nothing in qemu's cortex-a72 model implements PAN].

**The sequence** (ARM ARM self-modifying-code discipline; field layouts [CITED: Arm Developer CTR_EL0 register docs, support.arm.com/documentation/102530/0002 — `DminLine` [19:16] and `IminLine` [3:0], each log2 of the line size in WORDS, bytes = `4 << field`]):

```rust
// After copying text bytes into the user VA range [start, end):
// Source: ARM ARM self-modifying-code sequence + 9front cache.v8.s ordering
// (cacheduwbse then cacheiinvse), geometry from CTR_EL0 per Phase-2 D-03.
unsafe fn flush_icache_user_range(start: usize, end: usize) {
    let ctr: u64;
    core::arch::asm!("mrs {}, ctr_el0", out(reg) ctr);   // aarch64-cpu 11.2.0 has NO CTR_EL0 (source-verified)
    let dline = 4usize << ((ctr >> 16) & 0xf);           // DminLine
    let iline = 4usize << (ctr & 0xf);                   // IminLine
    let mut p = start & !(dline - 1);
    while p < end {                                      // clean dcache to Point of Unification
        core::arch::asm!("dc cvau, {}", in(reg) p);
        p += dline;
    }
    core::arch::asm!("dsb ish");                         // drains the cleans (Inner-Shareable)
    let mut p = start & !(iline - 1);
    while p < end {                                      // invalidate icache to Point of Unification
        core::arch::asm!("ic ivau, {}", in(reg) p);
        p += iline;
    }
    core::arch::asm!("dsb ish");
    core::arch::asm!("isb");                             // discard any stale prefetched instructions
}
```

Notes:
- **Never hardcode 64** (cortex-a72's actual line): Apple-silicon HVF guests and other future targets differ — Phase-2 D-03 mandates CTR_EL0 discovery. Cheap insurance: print CTR_EL0 once at boot (debug line) so the evidence log carries the probed geometry.
- CTR_EL0 `DIC`[29]/`IDC`[28] would let a machine skip the ic/dc legs entirely; expect both 0 on qemu TCG cortex-a72 — always execute the full sequence; the bits are an optimization only. [ASSUMED for qemu's exact reset values — the runtime boot print settles it; the full sequence is correct regardless]
- 9front's equivalent (`sys/src/9/arm64/cache.v8.s` `cacheduwbse`+`cacheiinvse`, fetched this session) discovers geometry via CSSELR_EL1/CCSIDR_EL1 and masks interrupts (`DAIFSet #2`) around the op — the CTR_EL0 read is the project-mandated simpler path; 9front's ORDER (clean D to PoU → DSB → invalidate I to PoU → DSB → ISB) is the ordering reference and matches the sequence above. 9front also runs the sequence in `trapinit` before publishing its vector table — same "written instructions must be visible" problem.
- The fault handler itself should run with IRQs off (they arrive masked on exception entry; don't unmask in the handler) — short, non-blocking, no lock interplay with the timer tick (see Pitfall 3).
- Barrier scope: `ish` (inner-shareable) suffices at `-smp 1`; 9front uses `$SY`/`$NSH` DSB variants in the same roles. Keep `ish` — harmless now and survives a future SMP move.

## Architecture Patterns

### System Architecture Diagram

Data flow for the phase's primary use case (boot to staged `init` via real syscall path):

```
HOST (xtask)                          GUEST (r9 on qemu -M virt)
────────────                          ──────────────────────────
Harvey output dir                     qemu loads:  kernel ELF (-kernel)
 (hello, init, smoke*)                     DTB @0x40000000 (-dtb, /chosen patched)
      │                                    initrd image in RAM (-initrd)
      ▼                                         │
mkpaqfs → bootfs.paq  ──(-initrd argv)──────────┤
                                                ▼
                          main9: fdt.rs finds /chosen {linux,initrd-start,end}
                                                │
                                                ▼
                          paqfs reader (no_std): walk/open/read on the image
                                                │  served as root file tree
                                                ▼
                          devroot ── exec path: open "/init" via devroot
                                                │
                                                ▼
                          a.out parse (BE header, R_MAGIC 0x8C47)
                          → new Proc: fresh TTBR0 root (identity map torn down, D-10)
                          → seed Ureg on proc's kernel stack (forkret shape)
                          → seed user stack [argc][argv…][nil], R0=tos, psr=EL0t
                                                │
                                                ▼  eret into EL0
                          EL0 init runs ──first instruction fetch ABORTS (nothing mapped, D-09)
                                                │
                          trap: EC=InstructionAbortLowerEl, IFSC=translation fault
                          → fault handler: paqfs read file-offset window → map page
                            (write via user VA, then DC CVAU/DSB/IC IVAU/DSB/ISB)
                          → eret back into EL0
                                                │
                          init: print() → SVC #51 (PWRITE fd=1, off=-1)
                                                │
                          trap: EC=0x15 (SVC) → dispatch[51] → devcons write
                          (Console MCS lock; PL011 polled TX) → result in frame.x0
                          → return-to-EL0 boundary: deliver pending notes (D-16)
                                                │
                          timer tick (INTID 30) from EL0 → round-robin switch (D-13):
                          save frame on proc kstack → swtch → restore other proc → eret
```

Decision points worth tracing in the plan: (1) sync-exception EC split — 0x15=SVC → dispatch; 0x20/0x24 (instruction/data abort from lower EL) → user-fault handler; everything else from EL0 → fault note to the process (donor `faultnote` shape) or loud panic from EL1; (2) return-to-EL0 boundary checks notes before eret; (3) tick handler decides reschedule-or-return.

### Recommended Project Structure (r9 repo — following Phase-3 platform-tree conventions)

```
r9/
├── aarch64/src/
│   ├── trap.rs / trap.S          # D-05/D-08 frame reshape, SVC dispatch entry, note delivery at return
│   ├── vm.rs                     # user-fault page-in hook, per-process root tables, TTBR0 switch ungated on virt
│   ├── main.rs                   # main9: initrd discovery → paqfs mount → staged init exec; D-12 experiment removal
│   ├── proc.rs (NEW)             # Proc table, scheduler, per-process kernel stacks (D-13) [placement: discretion]
│   ├── syscall.rs (NEW)          # dispatch table 0–54, measured handlers, loud-unimplemented default (D-14)
│   ├── exec.rs (NEW)             # a.out parse + segment setup + execregs-shaped frame seeding
│   ├── fault.rs (NEW)            # user-fault handler: decode, paqfs page-in, stack growth, icache (D-09)
│   └── platform/virt/            # devcons device shim (shares PL011 + Console lock), timer→scheduler tick
├── port/src/
│   ├── fdt.rs                    # + /chosen property reads (linux,initrd-start/end are u64 BE cells)
│   ├── paqfs.rs (NEW)            # no_std paqfs reader (host-testable) — D-02 spec above
│   ├── devroot.rs (NEW)          # root file tree over the paqfs reader
│   ├── devproc.rs (NEW)          # lite: ONLY /proc/N/note walk/open/write (D-16)
│   └── dat.rs                    # Proc/Chan/Dev scaffolding grows real fields
└── xtask/src/
    ├── main.rs                   # initrd assembly step, boot-test smoke profiles, marker assertions
    └── config.rs                 # + [qemu] initrd key (emission follows existing D-06/D-07 patterns),
                                  #   + ureg.h cross-repo check (D-06 layer 2), harvey-path TOML/env glue (D-03)
```
Harvey repo touches: `aarch64/ABI.md` gains the D-07 appendix ONLY; new staged-`init` + smoke C sources (discretion: e.g. `build/aarch64/` area); `sys/src/cmd/paqfs` gains a host-build recipe if Phase 2 didn't leave one reusable; evidence under `.planning/phases/04-*/`.

### Pattern 1: Trap-frame reshape (D-05/D-08) — with the SP_EL0/EL1h subtlety

**What:** `TrapFrame` becomes exactly Harvey's Ureg (offsets pinned above); `trap.S` saves SP_EL0/SPSR_EL1 into it; ESR_EL1/FAR_EL1 leave the frame and become separate args to `trap()`.

**Verified current state** (r9 `l.S` read this session): r9 drops EL2→EL1 with `SPSR_EL2_M_EL1H` — **r9 runs EL1h (SPSel=1), the 9front convention**. Consequences the plan must honor:
- On EL0-origin traps: hardware switched SP to SP_EL1 (kernel stack); `MRS SP_EL0` yields the user SP at any point in the handler — safe to read after the stack switch. ✓ D-08's "saves SP_EL0 always" works here.
- On EL1h-origin traps (IRQ while in kernel, the virtpanic brk, etc.): `MRS SP_EL0` yields a **stale user SP or garbage — NOT a kernel-context value** (D-08's wording is wrong on this detail). The interrupted kernel SP is only available as the *live* `sp` at handler entry, BEFORE `handle_interrupt` switches to the interrupt stack (`trap.S:53-55`). So: capture entry-`sp` into the frame's `sp` slot on EL1-origin vectors (the 16 vector stubs know their origin class, just like they know `type`), `MRS SP_EL0` on EL0-origin vectors. Panic dumps then show a truthful `sp` for both classes.
- **Restore side must be conditional:** on returns to EL0, write frame.sp→SP_EL0, frame.pc→ELR_EL1, frame.psr→SPSR_EL1 before the register-pop + `eret`. On returns to EL1h, write ELR/SPSR but **never SP_EL0** — otherwise an IRQ that interrupted EL1 kernel code would scribble a kernel address into the user SP, and the next eret-to-EL0 would fault instantly. Donor reference: `vtrap1`'s return writes only ELR/SPSR from the frame; `noteret`/`forkret` (EL0-bound) write SP_EL0/ELR/SPSR [CITED: 9front `l.s` at pin, fetched this session]. Test on `frame.psr & 0xF == 0` (EL0t) at the restore point.
- SVC result: write to frame `x0` before restore — the restore path's `ldp x0, …` propagates it to user R0. r9's existing restore already does this mechanically [VERIFIED: r9 trap.S:94].

**Const-assert layer (D-06 layer 1)** — always-on, no Harvey checkout needed:
```rust
// aarch64/src/trap.rs — pinned offsets from aarch64/ABI.md appendix (D-07)
const _: () = assert!(core::mem::offset_of!(TrapFrame, x0) == 0);
const _: () = assert!(core::mem::offset_of!(TrapFrame, x28) == 224);
const _: () = assert!(core::mem::offset_of!(TrapFrame, link_register) == 240);
const _: () = assert!(core::mem::offset_of!(TrapFrame, sp_el0) == 248);
const _: () = assert!(core::mem::offset_of!(TrapFrame, pc) == 256);
const _: () = assert!(core::mem::offset_of!(TrapFrame, psr) == 264);
const _: () = assert!(core::mem::offset_of!(TrapFrame, interrupt_type) == 272);
const _: () = assert!(core::mem::size_of::<TrapFrame>() == 288);
```
(`core::mem::offset_of!` is stable since Rust 1.77 — r9 pins nightly-2026-01-30, far past [ASSUMED — std stability knowledge; risk zero, compile itself proves it].)

**xtask layer (D-06 layer 2):** parse `../harvey/aarch64/include/ureg.h` (path via the D-03 TOML/env glue), extract field names/order/count, compare to the crate's table; fail build/boot-test on drift. Plain text parsing — no crate.

### Pattern 2: SVC dispatch + measured table (D-14)

Donor's loop (9front `trap.c`, at pin — fetched this session):
```c
void
syscall(Ureg *ureg)
{
	…
	scallnr = ureg->r0;
	if(dosyscall(scallnr, (Sargs*)(ureg->sp+BY2WD), &ureg->r0))
		returnto(noteret);      /* noted() rewrote the ureg — different return path */
	if(up->procctl || up->nnote)
		donotify(ureg);          /* note delivery at the return-to-EL0 boundary */
	if(up->delaysched)
		sched();
	…
}
```
r9's equivalent shape: `trap()` on EC 0x15 → `syscall::dispatch(frame, esr)` → number = `frame.x0`, args read from user memory at `frame.sp + 8` (+8 per slot) after a user-range validity check, result → `frame.x0`; `NOTED` gets the donor's special-case (its handler rewrote the frame — return without writing the result slot); then the notes check; then maybe-reschedule. Dispatch table 0–54: `Option<fn>` per number; `None` → loud error (print number, set errstr-equivalent, return -1) — never a silent wedge (D-14). Arg-read formula to publish in the D-07 appendix: `args at ureg->sp+8; SP+0 reserved`.

### Pattern 3: Demand-fault handler (D-09 core VM deliverable)

Fault decode (donor `faultarm64`, at pin — fetched this session): route on `ESR_EL1.ISS[5:0]` (IFSC/DFSC): **translation fault (4–7), access-flag fault (8–11), permission fault (12–15), TLB conflict (48)** → demand-page path; everything else → note/panic. Read/write: **WnR = ISS bit 6** (0=read/fetch). Fault address from FAR_EL1 (read live per D-05). r9's `registers.rs` already defines `InstructionFaultStatusCode` with exactly these values (lines 172-213, read this session) — extend with a data-abort ISS decode (WnR bit) and LowerEl constructors (the existing `from_esr_el1` filters `InstructionAbortSameEl` only).

Handler algorithm:
1. `addr = FAR_EL1`; classify segment by r9's per-process VA map (text / data / bss / stack / tos — D-11 policy).
2. Text/data: allocate phys page (`pagealloc::allocate_physpage` — MCS-locked, fine with IRQs off), map into the process root table at the faulting page VA (`vm.rs` `map_to` — its per-write full-TLB-invalidate is heavy but correct; optimization is Phase 5+), paqfs-read the file window (text: file offset = pageVA − 0x10000; data: file offset = (pageVA − adata) + (40 + textsize) — byte-granular, see the hello map), write via the user VA, **icache sequence after text page-in (D-09)**.
3. Bss/stack/tos: allocate + map zero page. Stack growth bounded (D-11 bound; donor's USTKSIZE is 16 MiB — reference only).
4. Unresolvable → fault note to the process (donor's `faultnote` → postnote "sys: trap: fault read/write addr=…"), which kills a handlerless process — the smoke never triggers this, but hello-with-a-bug should die loudly, not hang.

### Pattern 4: First-process exec (R9-03/SC#3)

- a.out parse: 8×BE u32 + BE u64 entry @32 (HDR_MAGIC); reject non-`0x8C47` loudly. All the numbers are verified against `hello.7.out` above.
- Text VA fixed by 7l: `INITTEXT = 0x10000+HEADR` when no `-T` flag [VERIFIED: harvey `sys/src/cmd/7l/obj.c:145-155` grepped this session: `INITTEXT = 0x10000+HEADR`], i.e. segments land at 0x10000 with the 40-byte header included in the mapping; data at the 64K-aligned boundary above text (donor align=0xffff for R_MAGIC). The loader maps where headers say (D-11 hard constraint) — r9's policy freedom is stack top/growth/initial frame only.
- First entry (discretion, forkret shape): seed a Ureg on the new process's kernel stack (sp→user stack seeded `[argc][argv0][nil]`, pc=entry, psr=0 (EL0t), link=0, r0=tos VA), then the normal trap-return path erets into EL0 — donor's `forkchild`+`forkret` is the exact shape [CITED: 9front trap.c/l.s at pin]. The first instruction fetch then faults — D-09 path takes over.
- TTBR0 teardown (D-10): the new process's freshly built root table replaces the identity map via the existing `vm::switch(table, RootPageTableType::User)` (ungate on virt — it's currently raspi-gated at the main.rs call site, not in vm.rs); every later process switch swaps TTBR0. MMIO safety proof stands (all runtime MMIO is TTBR1-side via `map_device_register`).
- Per-process root tables: `vm.rs`'s `USER_PAGETABLE` static singleton + `init_user_page_tables` give the construction pattern (recursive entry at slot 511); Phase 4 allocates a root table page per process instead of sharing the singleton. `map_to`'s temp-recursive-entry dance already handles editing the *active* user table; editing a not-yet-active table works by direct KZERO-offset physical writes at setup time.

### Pattern 5: Notes (D-16)

Delivery at the return-to-EL0 boundary, donor `notify()` verbatim semantics [CITED: 9front trap.c at pin]:
```c
sp = ureg->sp;
sp -= 256;	/* debugging: preserve context causing problem */
sp -= sizeof(Ureg);
sp = STACKALIGN(sp);              /* & ~7 */
/* validate user writable range, handler alignment, sp alignment */
nureg = (Ureg*)sp;
memmove(nureg, ureg, sizeof(Ureg));
sp -= BY2WD+ERRMAX;               /* 8 + 128 */
memmove((char*)sp, msg, ERRMAX);
sp -= 3*BY2WD;
*(uintptr*)(sp+2*BY2WD) = sp+3*BY2WD;   /* SP+16 = &msg  → handler arg1 home */
*(uintptr*)(sp+1*BY2WD) = (uintptr)nureg;/* SP+8 = nureg  → handler arg0 home */
ureg->r0 = (uintptr)nureg;              /* handler arg0 in R0 */
ureg->sp = sp;
ureg->pc = (uintptr)up->notify;         /* the notify()-installed handler VA */
ureg->link = 0;
```
This matches libc's handler signature `void f(void *ureg, char *msg)` (R0 + stack slot per the arm64 calling convention; `atnotify.c`'s `notifier` reads both — read this session). `noted(arg0)`: `NCONT`/`NRSTR` restore the saved Ureg over the live one; `NSAVE` re-pushes state; `NDFLT` exits with the note message (donor `noted()`/`sysnoted` quoted in research notes; Harvey libc values NCONT=0/NDFLT=1/NSAVE=2/NRSTR=3 verified above). r9 scope for the smoke: NCONT/NRSTR restore + NDFLT exit; NSAVE can return loud-unimplemented (the smoke's `notejmp` uses NCONT [VERIFIED: harvey `sys/src/libc/aarch64/notejmp.c` read this session: sets r0/pc/sp on the saved ureg then `noted(NCONT)`]).

Posting path: libc `postnote(PNPROC, pid, msg)` → open+write+close `/proc/N/note` → devproc-lite's write handler queues the note on the target Proc (donor `postnote`→`pushnote`→`procinterrupt` semantics; for r9-lite: append to target's note queue + mark pending; the delivery happens at the target's next return-to-EL0). devproc-lite directory: `/proc` root listing minimal (just pid dirs), `/proc/N/` contains only `note`.

### Pattern 6: Scheduler + rfork (D-13)

- Process model: small fixed table (discretion: size), Proc = { state, kernel stack, root table, fd table (devroot/devcons chans), note queue, wait queue, name/pid, saved Ureg pointer }.
- EL0 entry/exit rides the trap path; kernel-context switches (a process blocked mid-syscall) ride the EXISTING `swtch.rs`/`swtch.S` (callee-saved + sp + spsr, eret-based) [VERIFIED: r9 swtch.S:1-46 read this session]. CONTEXT notes the experiment's limit: swtch alone never enters EL0 — EL0 entry is always the trap-return path.
- `rfork(flags)`: minimal viable subset for the smoke: RFPROC|RFMEM semantics = child gets a fresh address space with copied (or copy-on-write-free: eagerly copied) user pages, fresh kernel stack, dup'd fd table, and a copied Ureg with `r0=0` (donor `forkchild` does exactly `cureg->r0 = 0`; parent returns child pid). Flag validation per donor `sysrfork` (reject (RFFDG|RFCFDG) together etc.). Smoke needs only the RFPROC-ish combination the traced binaries actually use — the D-14 trace pins it.
- Tick-preemption: the timer tick (INTID 30, already armed) becomes the scheduler tick (rate = discretion; current 1 Hz print tick is replaced — see Pitfall 3). IRQ-from-EL0: save frame on the current process's kernel stack, mark current Ready, pick next, `vm::switch` TTBR0, restore/eret. Marker-based boot-test assertions, never ordering (D-13).
- `await(buf,n)`: block parent until a child's exit record exists; format `"pid utime stime realtime 'msg'"` approximately (donor: `"%d %lud %lud %lud %q"` — match closely enough for the smoke; discretion).
- `exits(msg)`: free user pages/table, post wait record to parent, wake it, mark Moribund, reschedule. Donor `pexit` reference fetched this session.
- **Per-process kernel stacks are mandatory** (D-13 derived): on EL0 entry, switch SP to the *current* process's kernel stack top (from a `current` static) instead of the global `interruptstackbase`; the frame lives on that stack so a blocked syscall's state survives. 9front keeps SP_EL1 = current process kernel stack (Proc-at-top-of-KSTACK allocation, KSTACK=8 KiB); r9 equivalent: update SP_EL1 (or the asm's target stack pointer) on every process switch.

### Pattern 7: xtask integration (D-01/D-03/D-06)

- `[qemu]` TOML gains `initrd = "path/to/bootfs.paq"` → emits `-initrd <path>` (both `QemuStep` and `BootTestStep` share `apply_qemu_config` — one edit covers both) [VERIFIED: r9 `xtask/src/config.rs:294-322` read this session; the existing test `qemu_full_keys_emit_all_flags` is the exact pattern for a new emission test].
- initrd assembly step: run host-built `mkpaqfs -o <paq> <dir>` over the Harvey output dir (path from TOML/env per D-03). Pin the image as a build artifact under `target/`, never committed; sha256 printed for evidence.
- `boot-test` smoke profile(s): new config(s) (e.g. `config_virtexec.toml`, `config_virtnotes.toml`) with per-profile marker lists — the existing `BootStage` state machine + `count_marker_lines`/`contains_marker` helpers generalize [VERIFIED: r9 `xtask/src/main.rs:713-1063` read this session]. Panic profile stays untouched (TrapFrame Debug still prints x0..x30 rows — ESR/FAR move to separate args, printed separately per D-08).
- D-06 layer-2 check as a `BuildStep`-adjacent check or a `boot-test` precondition — must fail the run on drift; exact UX is discretion.

### Anti-Patterns to Avoid
- **Eager text/data loading at exec:** contradicts D-09 (demand-fault everything) and forgoes the fault-handler proof. Don't pre-map "just to get hello running" — the first-fetch fault IS the test.
- **Guessing syscalls:** any handler without a traced caller is scope creep (D-14). The trace artifact precedes implementation.
- **Reading ESR/FAR from the frame:** D-05 moved them out — read live at entry, pass as args. Stale-frame ESR reads are a classic silent-corruption source.
- **A second, EL0-only frame path:** D-08 is one unified frame. Forking the save/restore asm doubles the audit surface and breaks the virtpanic row-count gate.
- **Kernel-side big-endian mistakes:** a.out AND paqfs are both BE-on-disk; aarch64 is LE. Every header field read needs `from_be` (the paqfs reader and a.out parser should share a tiny `be32/be16` helper; write host tests against real fixture bytes — D-02 Wave-0 task).
- **Sharing the paqfs reader between devroot and the fault handler without a locking story:** the fault handler runs in trap context (IRQs off). Keep the reader stateless-per-call (the image is a memory slice — no I/O state) and it is reentrant by construction.
- **`static mut USER_PAGETABLE` reuse across processes:** one singleton user table = all processes share an address space. Per-process root tables from the start (Pattern 4).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Deflate decode for paqfs blocks | Custom inflate | `miniz_oxide::inflate` (no_std, `with-alloc`) | RFC 1951 dynamic-Huffman edge cases; miniz_oxide is the flate2 backend, battle-tested [CITED: docs.rs] |
| adler32 block verification | Custom checksum | `adler2` crate | Already transitive via miniz_oxide; spec-exact, no unsafe |
| DTB `/chosen` parsing | New DTB parser | Extend `port/fdt.rs` (`find_by_path("/chosen")` + property u64 read — `bytes_to_u64` exists privately; expose a `property_value_as_u64`) | Parser is proven on real qemu DTBs [VERIFIED: r9 fdt.rs read this session] |
| Page-table mechanics | New mapping code | `vm.rs` (`map_to`, `Entry::rw_user_text()/rw_user_data()`, `switch`, recursive slot 511) | Exists, tested, user-entry builders already defined [VERIFIED: r9 vm.rs:168-188] |
| Kernel context switch | New switch asm | `swtch.rs`/`swtch.S` as-is | Callee-saved + sp + spsr + eret; already correct [VERIFIED: r9 swtch.S] |
| paqfs image production | Rust paqfs writer | Host-built Harvey `mkpaqfs` (plan9port 9c recipe, Phase-2-proven) | Producer/reader keyed to the same format; one producer keeps D-02's "costly to change" contained |
| a.out parsing | "ELF because Rust" | 30-line BE header read | Harvey binaries are Plan 9 a.out (R_MAGIC); 8 words + u64 entry. Hand-rolling THIS is fine — it's a header, not a format |
| Syscall tracer | gdb scripting on raw RSP | donor `startsyscall` + `/proc/N/syscall` rc loop (D-14) | Zero rebuild, syscall-semantic output, driven by the existing serial harness |
| Serial capture/assertion | expect/rexpect | existing `boot-test` unix-socket harness | Phase-3 D-11 decision; marker model already proven |

**Key insight:** this phase's hard problems (cache maintenance, trap asm, demand paging) are all *sequencing* problems, not *algorithm* problems — the donor provides the sequencing, r9 provides the machinery. Where a real algorithm exists (inflate), take the crate.

## Common Pitfalls

### Pitfall 1: SP_EL0 vs entry-SP capture on the unified frame
**What goes wrong:** On EL1h-origin traps the frame's `sp` slot gets a stale/garbage SP_EL0 (or the post-switch interrupt-stack pointer), and on EL0-origin traps a naive pre-switch capture saves the kernel SP_EL1 instead of the user SP.
**Why it happens:** r9 runs EL1h (SPSel=1 from `l.S`'s EL2→EL1 drop — verified this session); with SPSel=1 the two origins disagree about where the interrupted SP lives (EL0: SP_EL0 register; EL1h: the live `sp` at handler entry, destroyed by the interrupt-stack switch at `trap.S:53-55`).
**How to avoid:** per-vector-origin capture: EL0 vectors `MRS SP_EL0` after the switch; EL1 vectors save live `sp` BEFORE the switch (the stubs already carry per-origin `type` constants — same dispatch point).
**Warning signs:** `ureg->sp` garbage in panic dumps; user stack corruption after the first IRQ-from-EL0.

### Pitfall 2: Unconditional SP_EL0 restore corrupts the user stack pointer
**What goes wrong:** An IRQ preempts EL1 kernel code; the unified restore writes frame.sp (a kernel address) into SP_EL0; the next eret-to-EL0 runs with a kernel SP → instant fault, mystifying because the syscall path "worked" in isolation.
**Why it happens:** D-08's "one unified frame" read naively on the restore side.
**How to avoid:** restore SP_EL0 only when returning to EL0 (`frame.psr & 0xF == 0`); ELR/SPSR always. Donor precedent: `vtrap1` (EL1) skips SP_EL0 on return; `noteret`/`forkret` (EL0) write it [CITED: 9front l.s at pin].
**Warning signs:** first timer-tick-during-syscall kills the process with a stack fault at a kernel-looking address.

### Pitfall 3: Console MCS-lock self-deadlock once syscalls print
**What goes wrong:** devcons `write` from syscall context holds `port/devcons.rs`'s Console MCS lock while TX-polling; the timer tick fires and its current `println!("tick {n}")` (`platform/virt/timer.rs:81-85`) tries the same lock from IRQ context → deadlock. `timer.rs`'s own comment (lines 77-80) documents the invariant: IRQ printing is safe ONLY because post-boot nothing else holds the lock — devcons syscalls break that invariant.
**How to avoid:** the tick's println goes away when the tick becomes the scheduler tick (D-13) — reschedule-only, no printing (or a trylock-with-drop policy). boot-test profiles that asserted `>=3 ticks` need their markers replaced by smoke markers (see OQ-4). The PL011 RX echo path is already lock-free (`pl011.rs:93-107`) and stays that way.
**Warning signs:** boot wedges exactly when the first user write coincides with a tick.

### Pitfall 4: Writing text through a kernel VA, executing through the user VA (alias desync)
**What goes wrong:** page contents written via a kernel (KZERO-offset) mapping are not visible to instruction fetches at the user VA — stale/zero icache lines, SIGILL-class mystery.
**How to avoid:** the fault handler runs with the process's TTBR0 live — write via the user VA and run the D-09 sequence on that same user VA. (If a design ever writes via a kernel alias: maintenance by VA is still required on the user mapping; cortex-a72's PIPT L1 makes PA-tag aliasing benign for dcache, but the IC IVAU leg must name the user VA.)
**Warning signs:** first-fetched page executes as zeros or `brk` — check the maintenance happened AFTER the write, on the right VA, with the trailing ISB.

### Pitfall 5: Data-segment file offset is not page-aligned
**What goes wrong:** treating the a.out file as if segment file offsets match segment page offsets — data read from `PGROUND(40+textsize)` instead of exactly `40+textsize` returns shifted bytes.
**Why:** 7l packs the file tightly: header(40) + text + data + syms…; the DATA VA is 64K-aligned but its FILE offset is `40+textsize` (hello: file 0x4B90 → VA 0x20000).
**How to avoid:** the fault-in path reads paqfs at byte granularity (`read_at(dir_entry, absolute_file_offset, …)`) — page-window file offset = `(pageVA - adata) + (40 + textsize)`.
**Warning signs:** `_tos`/setSB writes land in the wrong place; rodata strings garbled on first print.

### Pitfall 6: 9front's TRAPFRAMESIZE (304) ≠ sizeof(Ureg) (280) ≠ r9's 288
**What goes wrong:** copying donor asm constants blindly. 9front reserves 304 bytes/frame and puts the Ureg at `RSP+16` within it (`ADD $16, RSP, R0` throughout `l.s`); Harvey's frozen Ureg is 280 bytes; r9's aligned frame is 288 starting at `sp+0`.
**How to avoid:** r9 keeps its own frame discipline (D-05 pins size 288 unchanged); only FIELD OFFSETS cross repos, and they're pinned by D-06's checks.
**Warning signs:** any asm diff review that pattern-matches `288` vs `304` without comment.

### Pitfall 7: Implementing `_WRITE`(20)/`_READ`(15) and missing the real path
**What goes wrong:** hello prints nothing although `_write` is implemented — Harvey libc `write()` is `pwrite(fd,…,-1)` (PWRITE=51) [VERIFIED: 9sys/write.c this session].
**How to avoid:** D-14 trace-first; PREAD/PWRITE handle offset -1 as "use channel offset".
**Warning signs:** dispatch table "complete" but the trace names don't match the implemented set.

### Pitfall 8: `-initrd`+ELF+`-dtb` interaction assumed, not probed
**What goes wrong:** r9 expects `/chosen` `linux,initrd-start/end` but the committed trim-down `virt-gicv2.dtb` + ELF-boot path doesn't receive them (qemu version drift class of bug — Phase 3 already hit "no DTB for ELF kernels" once).
**How to avoid:** Wave-0 falsification probe: boot current r9 with `-initrd /bin/echo` (any bytes), print all `/chosen` properties from the kernel over serial, assert start/end present and sane (within RAM 0x40000000.., start < end, size matches the file). One boot-test run settles it before the paqfs reader exists. (CONTEXT carries the behavior as a verified carryover for qemu 11.1.2 — the probe converts carryover to this-repo evidence.)
**Warning signs:** initrd-start reads 0 or the property is absent.

### Pitfall 9: pagealloc ceiling vs initrd size
**What goes wrong:** the initrd lands in RAM pages that the bitmap allocator hands out later → userland bytes silently overwritten. Also `-m` beyond the bitmap ceiling (CONTEXT: keep ≤ 1G).
**How to avoid:** reserve the initrd PhysRange in `main9`'s `used_ranges` alongside `dtb_physrange` (the existing pattern, `main.rs:132-141` read this session); map it kernel-side (KZERO+PA) for the paqfs reader.
**Warning signs:** paqfs parse succeeds at boot, corrupts later under allocation pressure.

### Pitfall 10: Single global interrupt stack vs blocked syscalls
**What goes wrong:** a process blocks mid-syscall (await, console read); its trap frame lives on the shared interrupt stack; the next process's trap overwrites it.
**How to avoid:** D-13's derived requirement — per-process kernel stacks before the first blocking syscall lands; EL0 entry switches to `current_proc.kstack_top`.
**Warning signs:** works until rfork+await smoke; then frame corruption under interleaving.

### Pitfall 11: TTBR0 teardown ordering (WR-01 closure)
**What goes wrong:** tearing down the identity map before the first user table is fully built, or while some boot code still executes through it → prefetch abort in kernel.
**How to avoid:** D-10 sequence: build the process root table (with recursive slot 511 self-entry) → `vm::switch(table, User)` → verify MMIO liveness (console print after the switch — the proof that all runtime MMIO is TTBR1-side). The identity map exists only for the pre-MMU→MMU transition; nothing post-main9-entry should need it.
**Warning signs:** kernel dies silently at first-exec; serial shows nothing after the exec marker.

### Pitfall 12: devproc-lite overreach
**What goes wrong:** growing `/proc/N/{ctl,status,…}` "because devproc has them" — Phase-5 scope creep that drags in procctl/stop/start semantics.
**How to avoid:** D-16 hard scope: ONLY `/proc/N/note` (walk/open/write). `postnote(2)` needs nothing else [VERIFIED: postnote.c this session]. PNGROUP (`notepg`) is out — smoke uses PNPROC.
**Warning signs:** plan tasks mentioning ctl/status files.

## Code Examples

### Syscall handler shape (PWRITE with offset -1)
```rust
// Source: semantics from 9front sysproc.c/devcons patterns at pin; r9-side shape is new.
// args live at user SP+8.. (8-byte slots): PWRITE(fd, buf, n, off)
fn sys_pwrite(frame: &mut TrapFrame, proc: &mut Proc) -> u64 {
    let sargs = (frame.sp_el0 + 8) as *const u64;      // D-07 formula
    let (fd, buf, n, off) = unsafe { (*sargs, *sargs.add(1), *sargs.add(2), *sargs.add(3)) };
    // validate user range [buf, buf+n) against proc's segments BEFORE reading
    let chan = match proc.fd(fd) { Some(c) => c, None => return proc.err("bad fd") };
    let written = if off as i64 == -1 { chan.write_user(buf, n) } else { chan.pwrite_user(buf, n, off) };
    written as u64   // → frame.x0 by the dispatcher
}
```

### Note delivery at the return boundary (r9, D-16)
```rust
// Source: 9front trap.c notify()/donotify() at pin (quoted in Pattern 5); ERRMAX=128 verified.
fn deliver_notes(frame: &mut TrapFrame, proc: &mut Proc) {
    let Some(msg) = proc.pop_note() else { return };
    let Some(handler) = proc.notify_handler else { return proc.exit_with(&msg) }; // NDFLT-ish
    let mut sp = frame.sp_el0 - 256;            // donor's debug preserve area
    sp -= 280;                                  // sizeof(Ureg) — frozen size, NOT r9's 288 frame stride
    sp &= !7;
    let nureg = sp;                             // Ureg copy lands here
    sp -= 8 + 128;                              // msg slot (ERRMAX) + slack
    sp -= 3 * 8;                                // final user sp for the handler
    // write: nureg ← copy of the live frame as a Ureg; msg bytes at nureg-136…;
    // [sp+8] = nureg (arg0 home), [sp+16] = &msg (arg1 home) — donor layout verbatim
    unsafe { seed_note_frame(nureg, sp, frame, &msg) };
    frame.x0 = nureg;                           // R0 = handler arg0 (void *ureg)
    frame.sp_el0 = sp;
    frame.pc = handler;                         // notify()-installed handler VA
    frame.link_register = 0;
}
```

### First-exec frame seeding (forkret shape)
```rust
// Source: 9front execregs + forkchild/forkret at pin; r9 TrapFrame per D-05.
fn seed_exec_frame(kstack_top: usize, entry: u64, user_sp: u64, tos_va: u64) -> *mut TrapFrame {
    let frame = unsafe { &mut *((kstack_top - 288) as *mut TrapFrame) };  // 288 = r9 aligned frame
    *frame = TrapFrame::zeroed();
    frame.x0 = tos_va;                    // crt0 stores into _tos(SB)
    frame.sp_el0 = user_sp;               // user stack: [argc][argv0][nil]…
    frame.pc = entry;                     // from a.out u64 entry field
    frame.psr = 0;                        // EL0t, interrupts unmasked
    // link_register/x30 = 0 already
    frame
}
```

### `/chosen` initrd discovery
```rust
// Source: r9 port/fdt.rs API (read this session) + qemu arm boot protocol (carryover, probe Pitfall 8)
let chosen = dt.find_by_path("/chosen").expect("no /chosen in DTB");
let start = dt.property(&chosen, "linux,initrd-start").and_then(|p| dt.property_value_as_u64(&p));
let end   = dt.property(&chosen, "linux,initrd-end")  .and_then(|p| dt.property_value_as_u64(&p));
// reserve PhysRange(start..end) in pagealloc used_ranges; reader maps KZERO+start..KZERO+end
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| r9 EL0 vectors route to dump-and-wedge; `SVC` EC 0x15 prints-and-halts | Real dispatch with result writeback + note boundary | This phase | `trap.rs:170-174`'s "Syscall {syscallid}" + halt loop is replaced wholesale |
| Single global interrupt stack (`trap.S` `interruptstackbase`, 64 KiB) | Per-process kernel stacks + tick-driven round-robin | This phase (D-13) | Blocking syscalls become possible; `handle_interrupt`'s stack switch targets `current_proc` |
| raspi user-table experiment (`test_sysexit`, VA 0x1000 text, never erets) | Real a.out exec via forkret-shaped first entry | This phase (D-12) | Experiment deleted; git history preserves |
| ESR/FAR stored in-frame | Read live at entry, passed as dispatcher args | This phase (D-05) | Frame = frozen Ureg exactly; fault decode args explicit |
| 9front kernel boots paq via userspace paqfs (broken upstream `local!paqfs`) | r9 reads paqfs in-kernel (Rust no_std) | This phase (D-02) | No userspace fs bootstrap problem; exec sources through devroot |

**Deprecated/outdated:**
- `test_sysexit` + raspi user-table experiments (D-12 removal — "not a deliverable", must not be mistaken for the real path).
- 9front's CCSIDR/CSSELR cache-geometry discovery (works, but project-mandated CTR_EL0 per Phase-2 D-03 — simpler, one register, no cache-selection dance).
- The 1 Hz `tick {n}` print (superseded by the scheduler tick; see Pitfall 3).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | cortex-a72 (ARMv8.0-A) has no PAN; EL1 can read/write user-mapped pages without SCTLR bit management | D-09 | Fault handler's user-VA writes fault → visible immediately at first page-in; fix is mechanical (PAN bit management) — but PAN literally doesn't exist on v8.0, so the real risk is a wrong mental model elsewhere |
| A2 | qemu TCG cortex-a72's CTR_EL0 has DIC=0/IDC=0 (full cache sequence required) | D-09 | None — the sequence runs unconditionally; the boot-time CTR_EL0 print converts this to evidence |
| A3 | qemu writes `linux,initrd-start/end` into the `-dtb`-supplied DTB's `/chosen` for ELF `-kernel` boots | Pitfall 8 | D-01 delivery mechanism broken → fallback is an embedded blob. Wave-0 probe settles it before anything depends on it |
| A4 | The `startsyscall`+`/proc/N/syscall` rc loop drives cleanly under drive9.py (fragment alternation, Eprocdied termination) | D-14 | Trace method falls back to a donor-kernel `print` patch + rebuild — costs hours, not correctness. Recommend a Wave-0 dry run on stock `hello` |
| A5 | The smoke suite needs no `#c/pid` (getpid reads it on Harvey) | Tos note / OQ-2 | Smoke binary that calls getpid fails open loudly; add a `pid` file to devcons-lite (trivial) |
| A6 | `core::mem::offset_of!` available (stable 1.77) on pinned nightly-2026-01-30 | Pattern 1 | Zero — the first `cargo build` proves it |
| A7 | qemu virt places the initrd inside the RAM range reported by the DTB memory node (0x40000000..) | Pitfall 9 | pagealloc reservation math wrong → probe output (Pitfall 8) reveals the actual addresses |
| A8 | 9front's `Sargs`/bulk-arg-copy details are NOT needed — r9 reads arg slots individually with per-handler arity | Pattern 2 | None for the ABI; donor's `validaddr(sizeof(Sargs))` bulk check is replaced by per-access validation |

## Open Questions (RESOLVED)

Resolved at plan time (checker revision 1, 2026-11-02) — per-question citations inline below; the plans named are the resolution of record.

1. **Does the `-dtb`-supplied DTB receive `/chosen` initrd properties on the ELF boot path?**
   - What we know: CONTEXT carries "qemu places the image in RAM and writes linux,initrd-start/end into /chosen" as a verified carryover for the pinned qemu (11.1.2), marked for re-verification only if the version changes. It hasn't changed.
   - What's unclear: the *combination* ELF-kernel + explicit `-dtb` + `-initrd` hasn't been run in this repo (Phase 3's DTB work was about qemu loading NO dtb for ELF boots; the committed `virt-gicv2.dtb` is a workaround whose `/chosen` contents under `-initrd` are unprobed).
   - Recommendation: Wave-0 spike — one boot-test-style run printing `/chosen` properties; converts carryover to evidence (Pitfall 8).
   - **Resolution:** 04-01 (Wave-0 tracer) runs exactly this probe — qemu `-kernel` ELF + `-dtb` + `-initrd`, printing `/chosen` properties as the phase's first gate before any consumer depends on the mechanism.
2. **Does the smoke/staged-init set need `#c/pid` (getpid) or `fd2path`?**
   - What we know: Harvey `getpid()` opens `#c/pid` (devcons file); `postnote` needs only `/proc/N/note`.
   - What's unclear: the final smoke binaries' libc closure.
   - Recommendation: D-14 trace settles; if `#c/pid` appears, devcons-lite gains one static file.
   - **Resolution:** carried as an explicit watch item in 04-08's objective — the planned smoke set never calls getpid (rfork returns the pid to the parent directly); if any smoke binary's runtime behavior demands `#c/pid`, the loud open-failure names it and devcons gains one static `pid` file.
3. **What does the D-07 appendix say `ureg->type` CONTAINS on r9?**
   - What we know: the field is pinned (offset 272); 9front stores `(exception_class << 32) | ESR_EL1` there; r9's current frame stores its small vector index (0–15).
   - What's unclear: whether the appendix should pin r9's value semantics or leave them kernel-internal. The smoke/libc never reads `type` (notejmp reads r0/pc/sp only [VERIFIED: notejmp.c this session]).
   - Recommendation: appendix documents r9's encoding as kernel-side informational (wire contract only pins the field slot). Flag for the user at the plan checkpoint if they want donor-compatible `type` values instead.
   - **Resolution:** 04-03 implements the recommendation — the ABI.md appendix pins the `type@272` slot and documents r9's encoding as kernel-side informational; the two-layer static gate enforces the layout.
4. **boot-test marker coupling when the tick stops printing.**
   - What we know: current virt boot-test asserts ≥3 `tick` lines; Pitfall 3 requires the tick to stop printing once syscalls can print.
   - What's unclear: whether the Phase-3 virt profile (ticks+echo) must stay green IN ADDITION to new smoke profiles, or whether the exec profile supersedes it.
   - Recommendation: planner adds new per-smoke profiles (D-13 marker model) and keeps the Phase-3 profile pinned to a no-processes boot (tick printing intact there) OR migrates assertions; decide with the phase-03 evidence requirements in view.
   - **Resolution:** 04-06 Task 1 step 5 — the tick println becomes `cfg(not(dev_exec))`-gated; the Phase-3 virt profile (no `dev` list, no processes) keeps its tick+echo assertions intact, and the exec-era profiles (virtexec/virtmulti/virtnotes) assert markers only, never tick lines.
5. **Where staged-`init`/smoke sources live in Harvey's tree.**
   - What we know: D-03 pins an output dir consumed by r9's xtask; Phase 2's wiring (`aarch64/mkfile` CC=7c) exists.
   - Recommendation: planner picks (discretion) — e.g. a `build/aarch64/` subdir or `sys/src/cmd/` targets; keep them OUT of the donor-bootdir staging area to avoid confusion with gate binaries.
   - **Resolution:** sources live in `build/aarch64/` (`init.c` in 04-05, `smokefork.c` in 04-07, `smokenote.c` in 04-08) with `stage-build.sh` driving the lima-guest compile into the gitignored `build/aarch64/out/`; the donor-bootdir staging area is untouched.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo + rustc (r9 pinned toolchain) | all r9 work | ✓ (via r9 flake/devshell, R9-05 complete) | nightly-2026-01-30 | — |
| qemu-system-aarch64 | boot-test, spikes | ✓ (flake-pinned; Phase 3 proven) | 11.1.2 | — |
| plan9port (`9c`/`9l`, libflate/mp/sec) | mkpaqfs host build | ✓ (Phase 2 proven on darwin+linux aarch64) | in-tree recipe | donor-ns-build.sh's recipe as reference |
| Harvey 7a/7c/7l host tools | building hello/init/smoke | ✓ (Phase 2 complete, TC-03 gate passed) | in-tree | — |
| lima guest + donor tree (pin 350266104f) | D-14 trace | ✓ (Phase 2 harness; `vm-run.sh`/`donor-vm-build.sh` drivers; limactl via nix devshell) | — | none — the trace has no host-side substitute |
| mkpaqfs host binary | initrd production | ⚠ recipe proven (donor-ns-build.sh:240-243 built the donor's), Harvey-side host BUILD recipe is new work | — | run the donor's mkpaqfs (identical format) |
| aarch64-cpu 11.2.0 CTR_EL0 wrapper | D-09 | ✗ (verified absent from crate source) | — | one-line inline `asm!("mrs …, ctr_el0")` — r9 already does this for ttbr0/1 |

**Missing dependencies with no fallback:** none blocking.
**Missing dependencies with fallback:** mkpaqfs Harvey-side host build (fallback: donor's mkpaqfs — same in-tree source family); CTR_EL0 crate wrapper (fallback: inline asm).

## Security Domain

This phase is a kernel-in-qemu test environment (no network, no multi-user, single privileged boot user), but it establishes the kernel's user-boundary discipline. ASVS mapping:

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (no auth this phase; devproc-lite exposes only `/proc/N/note` write) |
| V3 Session Management | no | — |
| V4 Access Control | partial | devproc-lite write path must target only the addressed pid's note queue; exec requires the OEXEC-opened path (donor convention) |
| V5 Input Validation | yes | a.out header validation (magic, entry ≥ text start, sizes bounded — donor `sysexec` checks quoted); paqfs parse validation (magic/type/size ≤ blocksize, adler32, bounds-checked offsets); **user-pointer validation on every syscall arg** (`validaddr` analog against the process's segment map before any kernel read/write of user memory — the classic kernel boundary) |
| V6 Cryptography | no | — (paqfs sha1 trailer verification explicitly optional/skipped in-kernel; host-side fixture verification only) |

### Known Threat Patterns for {stack}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| User pointer into kernel memory (syscall args pointing at kernel VAs) | Elevation of Privilege | Segment-range validation per access; never trust `SP+8` contents without checks |
| Malformed a.out (entry below text, sizes over USTOP) | Tampering / DoS | Donor's sysexec check set (entry ≥ text, text/data/bss < USTKTOP−USTKSIZE) ported to the loader |
| Malformed paqfs image (bad magic, oversize block, offset past image end) | DoS / info leak | All offsets bounds-checked against the image slice; adler32 per block; fail loudly |
| Note stack overwrite (delivery onto an unwritable user stack) | DoS | Donor's `okaddr` checks in `notify()` (validated writable range + alignment) — port them |
| Fault-handler recursion (fault while handling a fault on kernel state) | DoS | Handler touches only pre-mapped kernel memory + the user VA being installed; keep IRQs off; panic loudly on kernel-context faults |

## Sources

### Primary (HIGH confidence)
- Harvey `aarch64/include/ureg.h`, `aarch64/ABI.md`, `sys/src/libc/9syscall/{sys.h,mkfile}`, `sys/src/libc/aarch64/{main9.s,notejmp.c,setjmp.s}`, `sys/src/libc/9sys/{callmain.c,postnote.c,write.c,read.c,sbrk.c,getpid.c}`, `sys/include/{a.out.h,libc.h,tos.h}`, `sys/src/cmd/paqfs/{paqfs.h,mkpaqfs.c,paqfs.c}`, `sys/man/{4/paqfs,8/mkpaqfs}`, `sys/src/cmd/7l/obj.c` — all READ this session (Harvey repo)
- r9 `aarch64/src/{trap.rs,trap.S,vm.rs,main.rs,swtch.rs,swtch.S,pagealloc.rs,param.rs,registers.rs,l.S,pre_mmu/vminit.rs}`, `aarch64/src/platform/virt/{mod.rs,timer.rs,pl011.rs,devcons.rs}`, `port/src/{dat.rs,fdt.rs,devcons.rs,bitmapalloc.rs}`, `xtask/src/{main.rs,config.rs}`, `Cargo.toml`, `aarch64/Cargo.toml`, `port/Cargo.toml`, `rust-toolchain.toml`, `aarch64/lib/config_{default,virt}.toml` — all READ this session (r9 repo)
- 9front at pin 350266104f (fetched raw this session): `sys/src/9/arm64/{trap.c,mem.h,l.s,cache.v8.s,dat.h,qemu(CONF)}`, `arm64/include/ureg.h`, `sys/src/9/port/{sysproc.c,proc.c,devproc.c,syscallfmt.c,portmkfile,mkfilelist}`, repo tree listing (GitHub API)
- crates.io source of aarch64-cpu 11.2.0 (downloaded + grepped: no CTR_EL0; SP_EL0/SPSR_EL1/ELR_EL1/FAR_EL1/CSSELR/CCSIDR present)
- docs.rs miniz_oxide 0.9.1 crate page + `inflate::decompress_to_vec` docs
- `xxd`/sha256 of committed `build/aarch64/hello.7.out`
- gsd package-legitimacy seam verdicts (miniz_oxide OK, adler2 OK)

### Secondary (MEDIUM confidence)
- Arm Developer CTR_EL0 register documentation (support.arm.com/documentation/102530/0002 and Cortex-A53 TRM mirror) — DminLine[19:16]/IminLine[3:0], log2-words encoding, DIC/IDC semantics
- qemu `virt` machine documentation (qemu.org/docs/master/system/arm/virt.html) — RAM at 0x40000000, DTB conventions; `-initrd`→`/chosen` behavior is a Phase-3 carryover fact (version unchanged), Wave-0 probe scheduled

### Tertiary (LOW confidence)
- cortex-a72 = ARMv8.0-A / no PAN (standard knowledge, unverified this session — A1)
- qemu TCG cortex-a72 CTR_EL0 reset values (A2)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — crate APIs verified against docs.rs/crates.io source; host tools proven in Phase 2
- Architecture: HIGH — donor and r9 sources both read directly; SP_EL0/EL1h subtleties derived from r9's actual l.S/trap.S, not assumed
- Pitfalls: HIGH — each cites the code it derives from
- D-02 paqfs: HIGH — byte-level spec from producer+reader source, both read in full
- D-09 icache: HIGH on sequence/ordering (ARM docs + 9front reference), MEDIUM on qemu reset values (runtime print settles)
- D-14 trace: HIGH on mechanism (donor source), MEDIUM on harness-drive smoothness (Wave-0 dry run recommended)

**Research date:** 2026-11-01/02
**Valid until:** 2026-12-02 (stable: pinned donor, frozen ABI, pinned qemu; fast-moving: none)

