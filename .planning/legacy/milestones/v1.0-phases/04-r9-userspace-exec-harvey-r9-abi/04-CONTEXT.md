# Phase 4: r9 Userspace-Exec + Harvey↔r9 ABI - Context

**Gathered:** 2026-11-02
**Status:** Ready for planning

<domain>
## Phase Boundary

r9 loads and runs Harvey aarch64 userland binaries: EL0 entry/return, `svc` syscall dispatch against the frozen user-side wire ABI, first-process loading from an initrd root-fs, demand-paged user memory, and build-time static ABI enforcement. **Kernel-side work happens in the sibling repo `../r9`** (fork `requaos/r9`, branch `main`); Harvey-side changes are limited to the `aarch64/ABI.md` kernel-side appendix and any harness/binary outputs (the frozen wire contract itself does not change). Planning artifacts and acceptance evidence live in this repo.

NOT in this phase: full `/bin/rc` shell boot (Phase 5), real Harvey `/bin/init` with namespace/auth (Phase 5), flake-input consumption of r9 by Harvey (Phase 5 INT-04), virtio-anything / SMP-PSCI / RTC / PCIe (v2), devproc beyond the `/proc/N/note` slice, namespace syscalls beyond what the measured footprint requires, upstream-r9 PR flow.

</domain>

<decisions>
## Implementation Decisions

### Init/root-fs delivery
- **D-01:** Root-fs delivery is **qemu `-initrd` + DTB**: r9's xtask assembles a root-fs image from a directory of Harvey aarch64 binaries and passes it via `-initrd`; the kernel reads `linux,initrd-start`/`linux,initrd-end` from `/chosen` in the DTB (`port/fdt.rs` already parses the DTB; `/chosen` parsing is new). Userland changes never rebuild/relink the kernel — the Phase 5 iteration story. Image bytes are pinned artifacts, so gates stay deterministic. — **Reversibility:** reversible — contained to the xtask qemu step and the kernel's initrd discovery; the devroot-facing interface (find file by name, read bytes) is unchanged by any future mechanism swap.
- **D-02:** The initrd image format is **paqfs/bootfs** (Plan 9-native). Harvey's host tool `mkpaqfs` (`sys/src/cmd/paqfs/`) produces the image — the Phase 2 harness already baked `bootfs.paq` with it for the donor kernel; 9front's `bootfs` device is the reference reader shape. r9 grows a Rust `no_std` paqfs reader. **Researcher must pin:** paqfs's per-file compression requirements (libflate/glossary machinery — what must the reader implement), whether uncompressed entries are format-legal, and byte-level format verification against `mkpaqfs` output. — **Reversibility:** costly — reader and producer are both keyed to the format; a format swap touches both repos' build steps.
- **D-03:** The initrd carries the **SC-scoped set only**: Phase 2's `hello` (SC#2), the staged `init` (SC#3), and the smoke-suite binaries (SC#4). Harvey's build drops aarch64 binaries in a known output dir; r9's xtask reaches it via a **TOML/env path** — acknowledged temporary cross-repo glue, replaced by the INT-04 flake input in Phase 5. Do NOT commit Harvey binaries into r9.
- **D-04:** r9's first user process is a **staged minimal init** — a small 7c-compiled Harvey binary named `init` that goes through the real a.out load/exec path but has a scoped job (print via `write`, exercise exec/fork smoke, exit). The full Harvey `/bin/init` (namespace, auth, boot rc) lands in Phase 5 when devices + namespace syscalls exist. No initcode asm stub layer — the direct a.out load exists anyway.

### Trap-frame + static ABI check (R9-04, SC#1)
- **D-05:** **The trap frame IS Harvey's Ureg, exactly.** r9's `TrapFrame` tail becomes `{sp_el0→sp, elr_el1→pc, spsr_el1→psr, interrupt_type→type}` — 35 × u64, 288 bytes with align(16), unchanged size. `trap.S` grows saves of `SP_EL0` and `SPSR_EL1`, drops the in-frame `ESR_EL1`/`FAR_EL1` stores. ESR/FAR are read live at trap entry and passed to the dispatcher as separate arguments (fault decode only; syscall return never needs them). — **Reversibility:** one-way — this layout is published into `aarch64/ABI.md` (D-07) and pinned by the static checks (D-06); changing it after this phase re-freezes both repos.
- **D-06:** **Two-layer static check.** (1) In-crate Rust const asserts (`mem::offset_of` per field vs pinned byte offsets) — always on, fails `cargo build` even without a Harvey checkout. (2) An xtask check that parses `../harvey/aarch64/include/ureg.h` (field names/order/count) and compares against the crate's offset table, failing build/boot-test on drift — same TOML/env path glue as D-03, cleaned up by INT-04 in Phase 5. A mismatch is a build failure, not a runtime mystery (SC#1 wording).
- **D-07:** The kernel-side contract is written as an **appendix to `aarch64/ABI.md`** (the existing frozen canonical doc — one canonical place per SC#1): the pinned frame-offset table (`r0`@0 … `type`@272), the arg-read formula (`args at ureg->sp+8`, `SP+0` reserved), and how the two-layer check works. Frozen wire-contract text is untouched — the appendix documents enforcement, not new ABI. r9's docs (README/HowItWorks) get a pointer line. — **Reversibility:** one-way — once published, downstream code and checks key off the document.
- **D-08:** **One unified frame for all exceptions.** `trap.S`'s single `handle_interrupt` path saves SP_EL0/SPSR_EL1 always; on EL1 traps those slots hold kernel-context values. The panic dump prints ESR/FAR from the new trap() args — outside the struct — so the virtpanic x0–x30 row-count gate (Phase 3, boot-test asserted) keeps working unchanged. — **Reversibility:** costly — touches every trap path, but contained in `trap.S`/`trap.rs`.

### First-process + user VM (R9-03, SC#3)
- **D-09:** **Demand-fault everything.** Nothing is mapped at exec: the first instruction fetch faults, and the user-fault handler pages text/data in from the initrd paqfs on demand; stack faults grow the stack segment within a bound. This is the Plan 9 segment model and SC#3's literal wording ("faulting in user text and growing the user stack"). The user-fault handler is this phase's core VM deliverable. **icache maintenance after text load is required** (SC#3 wording — researcher specifies the `dc cvau`/`ic ivau` + barrier sequence for the fault-in path, per Phase 2 D-03's architectural-discovery rule: use `CTR_EL0`, not hardcoded geometry). — **Reversibility:** costly — the fault handler and loader intertwine; an eager variant would be a rewrite of the exec path.
- **D-10:** **TTBR0 identity map torn down at first exec** (closes Phase 3 review item WR-01). The first user process's root table replaces the identity map in TTBR0 (`vm::switch(_, RootPageTableType::User)` exists but is raspi-gated — ungate on virt); every later process switch swaps TTBR0. Verified during discussion: all runtime MMIO (GIC, PL011) already maps via `map_device_register` into the TTBR1 kernel table (`KZERO + 0x100000000000` upward) — the identity map has no runtime consumer. The early-boot identity window remains for the pre-MMU→MMU transition only.
- **D-11:** **r9-specific user address-space policy** (user decision, overriding the 9front-convention recommendation): r9 owns stack-top placement, segment/growth policy, and the initial stack frame (argc/argv construction for libc's `main9.s` crt0). **Hard constraint for the planner:** text/data VAs are fixed inside the a.out headers by 7l (no relocation exists) — the loader maps text/data where the headers say; "r9-specific" governs stack top and segment policy only. — **Reversibility:** costly — baked into the loader, seeded frames, and any user-visible assumptions; revisit before Phase 5 if libc surprises surface.
- **D-12:** **Remove `test_sysexit` and the raspi user-table experiments from `main9`** when the real exec path lands (git history preserves them). Phase 3 CONTEXT flagged them "not a deliverable"; the experiment never `eret`s to EL0 and must not be mistaken for the real path.

### Process model scope (R9-02, SC#4)
- **D-13:** **Tick-preemptive round-robin** over a small fixed process table — the existing generic-timer IRQ reschedules, so IRQ-from-EL0 + return-to-a-different-process is exercised this phase rather than discovered in Phase 5. Boot-test asserts markers, not byte-order (interleaving is nondeterministic). **Derived requirement:** preemption forces **per-process kernel stacks** — a process blocked mid-syscall cannot share the single interrupt stack. — **Reversibility:** costly — scheduler shape touches trap return, timer handler, and the process table.
- **D-14:** **Measure-first syscall set.** The researcher traces the exact syscall footprint of `hello` + the staged `init` + the smoke binaries on the 9front donor kernel (qemu gdb / kernel instrumentation — the Phase 2 harness makes this cheap), then r9 implements exactly that set **plus the SC#4-named rfork/notes/wait core** (RFORK=19, NOTIFY=28, NOTED=29, AWAIT=47, EXITS=8, EXEC=7 + the fd-ops set). Zero guessed syscalls; each has a measured caller. Watch: fmt/pool allocator may pull `BRK_`(24)/`SEGBRK`(12) into hello's footprint — the trace settles it. The ABI table stays 0–54 lazy: unimplemented numbers return a loud error, never a silent wedge.
- **D-15:** Device set for EL0 fds: **devroot + devcons + devproc-lite**. devroot serves the initrd paqfs as the root file tree (walk/open/read/stat) — exec sources binaries through the same devroot path, one mechanism. devcons provides console read/write with **fd 0/1/2 preopened at exec** per Plan 9 convention. devproc-lite (below) exists only for the note channel.
- **D-16:** **Parent-posts-child notes** (user decision, overriding the alarm round-trip recommendation). Verified against the frozen `sys.h` (0–54): **no postnote syscall exists** — `postnote(2)` is a libc function writing `/proc/N/note`, so the device set extends to **devproc-lite exposing ONLY `/proc/N/note`** (walk/open/write) — libc `postnote(PNPROC, pid, msg)` works unmodified. Smoke shape: parent rforks child → child `notify()`+spin → parent `postnote` → delivery at the child's return-to-EL0 boundary (pc redirected to the handler, note string on the user stack) → `noted()` resumes → child exits → parent `await`s. Exercises RFORK/NOTIFY/write/delivery/NOTED/EXITS/AWAIT end-to-end. — **Reversibility:** costly — note delivery semantics are wired into trap return and devproc-lite's shape.

### Carried forward (locked in roadmap/prior phases — do not re-litigate)
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 4 — goal, success criteria 1–4, pre-planning flag (highest-risk phase; spike consideration)
- `.planning/REQUIREMENTS.md` — R9-02, R9-03, R9-04 full text; v2 deferral list
- `.planning/PROJECT.md` — Key Decisions (r9 adoption, proof standards, flake composition)

### The frozen user-side ABI (Harvey repo — canonical contract)
- `aarch64/ABI.md` — the frozen wire contract (R0/SP+8/SVC/R0, table 0–54, NMOUNT lazy); gains the D-07 kernel-side appendix this phase
- `aarch64/include/ureg.h` — the trap-frame layout r9's TrapFrame must match field-for-field (all fields u64int; offsets = index×8)
- `sys/src/libc/9syscall/sys.h` — the frozen syscall number table (verified: no POSTNOTE; ALARM=6, EXEC=7, EXITS=8, RFORK=19, NOTIFY=28, NOTED=29, AWAIT=47, PREAD=50, PWRITE=51)
- `sys/src/libc/9syscall/mkfile` — `case aarch64` stub generator (the user-side stub shape, per-name quirks)
- `sys/src/libc/aarch64/` — vendored libc asm incl. `main9.s` crt0 (initial stack-frame expectations) and `setjmp.s`/`notejmp` (note-path interplay)
- `build/aarch64/hello.c` + `build/aarch64/hello.7.out` — the SC#2 test binary and its provenance
- `build/aarch64/README.md` — Phase 2 harness: donor-kernel build, bake-vs-ufs decision, qemu-11.1.2 drift accommodations (GIC wake poke, ECAM/DTB loader) — read before assuming qemu behavior

### paqfs (D-02 — researcher pins the format)
- `sys/src/cmd/paqfs/` — Harvey's mkpaqfs host tool + paqfs implementation (producer side; compression machinery to inventory)
- 9front `sys/man/4/paqfs` (or local equivalent under `sys/man/`) + 9front kernel `bootfs` device — the reference reader shape

### Prior-phase decisions and debts (Harvey repo)
- `.planning/phases/03-r9-qemu-virt-platform-bring-up/03-CONTEXT.md` — D-01..D-13 (platform tree, TOML-as-data, ELF/-kernel, boot-test harness, evidence consolidation); carried-forward list naming Phase 4 scope
- `.planning/phases/03-r9-qemu-virt-platform-bring-up/03-REVIEW-DISPOSITION.md` — WR-01 (TTBR0 identity map never torn down — D-10 closes it), WR-02..WR-05 hygiene items
- `.planning/phases/03-r9-qemu-virt-platform-bring-up/03-PATTERNS.md` — r9 file-tree map at r9 HEAD (structural precedents)
- `.planning/phases/02-aarch64-toolchain-libc-9front-donor/02-CONTEXT.md` — D-15/16/17 (ABI freeze), D-06..D-09 (execution-gate proof standard, donor harness), D-03 (Apple 16K/CTR_EL0 forward-note)
- `.planning/phases/02-aarch64-toolchain-libc-9front-donor/` Phase-2 summaries — `02-05-SUMMARY.md` (TC-03 gate: hello execution proof, guest path resolution), `02-07-SUMMARY.md` (shell subset + NMOUNT nm audit)
- `.planning/phases/01-build-tail-green-9k-amd64/01-CONTEXT.md` — boot-test-not-build-test lesson; serial capture/assert pattern

### r9 repo (sibling, `../r9` — where kernel work happens)
- `../r9/aarch64/src/trap.rs` + `trap.S` — TrapFrame reshape target (D-05/D-08); current EL0 vectors route to dump-and-wedge; `SVC` EC 0x15 handling is print-and-halt today
- `../r9/aarch64/src/main.rs` — `main9` orchestration; `test_sysexit` + raspi user-table experiments (D-12 removes); virt IRQ arming order (GIC → timer → PL011 RX → unmask) is the Phase 4 ordering precedent
- `../r9/aarch64/src/vm.rs` — root tables (`ttbr0/ttbr1`), `switch()`, `Entry::rw_user_text/data`, recursive-table addressing, `next_free_device_page4k` (device VAs are TTBR1-side — D-10's safety proof)
- `../r9/aarch64/src/pre_mmu/vminit.rs` — the identity-map window being torn down (D-10); comments document the boot-transition role
- `../r9/aarch64/src/swtch.rs` + `swtch.S` — existing kernel context switch (callee-saved + sp + spsr); EL0 entry is eret-based, NOT swtch-based — planner notes the experiment's limit
- `../r9/aarch64/src/pagealloc.rs` — bitmap allocator; 16KiB early pages ⇒ 2GiB RAM ceiling (keep `-m ≤ 1G`)
- `../r9/aarch64/src/platform/virt/{gic.rs,timer.rs,pl011.rs,devcons.rs}` — armed IRQ sources + console; console write path must become syscall-context-safe for devcons
- `../r9/port/src/fdt.rs` — DTB parser; `/chosen` initrd parsing (D-01) extends here
- `../r9/port/src/dat.rs` — `Proc` is an empty struct; `Chan`/`Dev`/`Qid` skeletons exist — the process/fd/dev scaffolding starts here
- `../r9/port/src/devcons.rs` — Console lock + println! macro (kernel side); devcons device shares the PL011 with it
- `../r9/xtask/src/{main.rs,config.rs}` — initrd assembly (D-01/D-03), the ureg.h cross-repo check (D-06), boot-test assertions (marker-based per D-13)
- `../r9/HowItWorks.md` — update with material changes (header mandates it)
- 9front arm64 kernel (donor tree at pin `350266104f`, see `build/aarch64/`) — `sys/src/9/arm64/trap.c:196` (arg-read formula), `mem.h` (user layout reference), `proc.c`/`sysproc.c` (rfork/notes semantics reference); the D-14 trace runs on this kernel

### qemu virt machine facts (carryover — researcher re-verifies only if qemu version changes)
- `-initrd` on aarch64 virt: qemu places the image in RAM and writes `linux,initrd-start`/`linux,initrd-end` into `/chosen` of the DTB; r9 already substitutes the fixed-PA DTB fallback (`0x40000000`) for ELF boots
- HVF rejects GICv2 on qemu 11.1.2 — darwin stays TCG this phase (Phase 3 decision)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `port/fdt.rs` — full DTB parser; D-01 needs only `/chosen` property reads added
- `vm.rs` user-table machinery — `init_user_page_tables`, `switch(User)` (raspi-gated today), `Entry::rw_user_text()/rw_user_data()`, recursive-table addressing — the demand-fault handler and TTBR0 swap build directly on these
- `swtch.rs/S` — kernel context switch (callee-saved x19–x30 + sp + spsr) — reused as-is for process switches; EL0 entry/exit rides the trap path instead
- `TrapFrame` restore path in `trap.S` — writes to frame `x0` propagate to user R0 on `eret`: the syscall-result mechanism already exists mechanically
- `platform/virt/{gic,timer,pl011}` — armed, proven IRQ sources; the timer tick doubles as the scheduler tick (D-13)
- `port/devcons.rs` Console lock — the kernel print path devcons shares; boot-test marker assertions already tolerate its output shape
- Phase 2 donor-kernel harness (`build/aarch64/`) — the D-14 measurement rig: gdb attach + donor kernel instrumentation on an execution-proven target
- `mkpaqfs` host tool (`sys/src/cmd/paqfs/`) — the initrd producer (D-02), already exercised in Phase 2

### Established Patterns
- Platform-tree + cfg dispatch (`platform/virt/…`, TOML-as-data) — new Phase 4 machinery (initrd, loader, devices) follows the same placement conventions
- Evidence shape: captured serial log + marker assertions via `cargo xtask boot-test` + human checkpoint; canonical evidence consolidated in Harvey `.planning/phases/04-*/` with the r9 HEAD SHA recorded
- Execution-gated ordering (the riscv64 vaporware rule): each kernel capability lands only behind a running user binary that exercises it
- r9 commits on `main`; HowItWorks.md updated with material changes
- One `config_{name}.toml` per variant; new behavior lands as TOML data, not Rust edits, where feasible

### Integration Points
- `trap.S`/`trap.rs` — frame reshape (D-05/D-08), EL0 SVC dispatch, syscall result in frame `x0`, note delivery at return-to-EL0 (D-16), user-fault dispatch (D-09)
- `vm.rs` — user-fault page-in, segment growth, TTBR0 swap per process, WR-01 teardown (D-10)
- `main.rs`/`main9` — initrd discovery → paqfs mount → staged init exec; experiments removed (D-12)
- `port/src/dat.rs` + new files — Proc table, fd table, devroot/devcons/devproc-lite devices
- `xtask/src/{main.rs,config.rs}` — initrd assembly from the Harvey output dir (D-01/D-03), ureg.h cross-repo check (D-06), smoke-suite boot-test profiles
- `aarch64/ABI.md` (Harvey) — the D-07 kernel-side appendix (only Harvey-repo doc change)

</code_context>

<specifics>
## Specific Ideas

- The D-16 smoke, narrated end-to-end (the acceptance story for SC#4): parent `rfork`s child → child `notify()` installs a handler and spins → parent `postnote(PNPROC, childpid, "smoke")` (libc, unmodified — writes `/proc/N/note` on devproc-lite) → at the child's next return-to-EL0 the kernel redirects pc to the handler with the note string on the user stack → handler verifies, `noted()` resumes → child `exits` → parent `await`s and prints the result. boot-test asserts the markers, not their order (D-13 preemption).
- Arg-read formula to publish in the ABI appendix (D-07): kernel computes `(Sargs*)(ureg->sp + 8)` — `SP+0` is the reserved LR slot the stub's `MOV R0, 0(FP)` materializes; the first kernel-visible argument sits at `SP+8`.
- Watch-out: hello's `print()` goes through fmt/pool machinery that may pull `BRK_`(24)/`SEGBRK`(12) into the footprint — the D-14 donor trace settles the set before any syscall is implemented.
- Watch-out: the console write path from syscall context must be safe against the kernel Console lock (Phase 3's timer.rs note: "what makes tick()'s println from IRQ context safe") — devcons writes from EL0 syscalls share that lock discipline.
- Watch-out: demand page-in of text writes to user pages through the kernel table while the user table owns the mapping — the fault handler's VA access pattern (recursive table or temp kernel mapping) is planner territory, but the icache sequence (D-09) applies after every text page-in, not just initial load.
- User overrides to honor downstream (deliberate, don't second-guess): paqfs over the simpler custom table (Plan 9-native fidelity); r9-specific address-space policy over 9front constants (r9 owns its VM policy); parent-posts-child notes over the alarm round-trip (exercises the real posting path); staged init over real `/bin/init` (phase scoping).

</specifics>

<deferred>
## Deferred Ideas

- virtio-9p host mount (collapses userland iteration time) — v1.x, top candidate per REQUIREMENTS.md
- Full devproc (`/proc/N/{ctl,status,…}`), devenv, namespace syscalls for the real `/bin/init` — Phase 5
- SMP via PSCI, virtio-net/blk, RTC, PCIe ECAM — v2
- raspi user-table/experiment revival — superseded by the real path (D-12); git history preserves
- HVF acceleration for virt boots — needs a GICv3 driver (qemu 11.1.2 rejects GICv2 under HVF); future work
- Upstream contribution flow to original r9 — fork `requaos/r9` remains the working repo
- 9k-amd64 pool-panic re-test with the LP64-fixed 6c (phase-02 deferred-items.md #8) — amd64-side debt, not this phase

</deferred>

---

*Phase: 4-r9 Userspace-Exec + Harvey↔r9 ABI*
*Context gathered: 2026-11-02*
