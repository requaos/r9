# Phase 4: r9 Userspace-Exec + Harvey↔r9 ABI - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-02
**Phase:** 4-r9-userspace-exec-harvey-r9-abi
**Areas discussed:** Init/root-fs delivery, Trap-frame + static ABI check, First-process + user VM, Process model scope

---

## Init/root-fs delivery

| Option | Description | Selected |
|--------|-------------|----------|
| qemu -initrd + DTB | xtask assembles image from Harvey binaries dir; kernel reads linux,initrd-start/end from /chosen; no kernel rebuild on userland change | ✓ |
| Embedded include_bytes! blob | devroot-style, compiled into ELF; every userland change relinks kernel | |
| xtask-stitched post-build blob | blob appended to ELF post-build; nonstandard ELF surgery | |

**User's choice:** qemu -initrd + DTB
**Notes:** Iteration story matters for Phase 5; fdt.rs already parses DTB.

| Option | Description | Selected |
|--------|-------------|----------|
| Custom minimal table | ~50-line Rust parser; we own both ends | |
| tar archive | host tooling everywhere; no_std reader overhead | |
| paqfs/bootfs image | Plan 9-native; r9 needs a paqfs reader; mkpaqfs produces | ✓ |

**User's choice:** paqfs/bootfs image — overriding the custom-table recommendation (Plan 9-native fidelity)
**Notes:** Researcher must pin compression requirements and confirm mkpaqfs output compatibility; 9front bootfs device is the reference reader.

| Option | Description | Selected |
|--------|-------------|----------|
| SC-scoped set | hello + staged init + smoke binaries; TOML/env path glue to Harvey output dir | ✓ |
| Full shell subset now | 16 binaries + rc before r9 can exec one; bisect-hostile | |
| Commit binaries into r9 | self-contained but stale-copy risk; contradicts flake-input direction | |

**User's choice:** SC-scoped set
**Notes:** Cross-repo glue is acknowledged temporary; INT-04 flake input replaces in Phase 5.

| Option | Description | Selected |
|--------|-------------|----------|
| Staged minimal init | small 7c binary named init through the real a.out path; scoped job | ✓ |
| Real Harvey /bin/init now | needs bind/mount/env/cons machinery — scope explosion | |
| initcode asm stub first | extra asm layer; direct a.out load must exist anyway | |

**User's choice:** Staged minimal init

---

## Trap-frame + static ABI check

| Option | Description | Selected |
|--------|-------------|----------|
| Frame = Ureg exactly | tail becomes {sp, pc, psr, type}; ESR/FAR live-read at entry, passed as args; frame stays 288B | ✓ |
| Ureg prefix + extras tail | 35 pinned + {esr, far} appended; 304B; two conventions | |
| Two frames, convert at SVC | copy per syscall; IRQ-from-EL0 still needs full state | |

**User's choice:** Frame = Ureg exactly
**Notes:** trap.S saves SP_EL0/SPSR_EL1, drops in-frame ESR/FAR; pc≡ELR and type≡interrupt_type are renames.

| Option | Description | Selected |
|--------|-------------|----------|
| Const asserts + xtask ureg.h parse | in-crate always-on + cross-repo gate via path glue | ✓ |
| Const asserts only | no cross-repo visibility; weakens R9-04 | |
| Vendored ureg.h + hash pin | two copies of a frozen file — the drift shape this phase prevents | |

**User's choice:** Const asserts + xtask ureg.h parse

| Option | Description | Selected |
|--------|-------------|----------|
| Extend aarch64/ABI.md | kernel-side appendix in the frozen canonical doc; one canonical place per SC#1 | ✓ |
| New r9-side ABI doc | two documents to keep in agreement | |
| Machine-readable shared table | third artifact + parsers for 35 u64 fields | |

**User's choice:** Extend aarch64/ABI.md
**Notes:** Appendix documents enforcement; frozen wire text untouched.

| Option | Description | Selected |
|--------|-------------|----------|
| One unified frame | all exceptions use the Ureg frame; EL1 traps get kernel-context values; virtpanic gate unaffected | ✓ |
| Split EL0/EL1 frames | two asm paths, two structs; no payoff this phase | |

**User's choice:** One unified frame

---

## First-process + user VM

| Option | Description | Selected |
|--------|-------------|----------|
| Demand-fault everything | nothing mapped at exec; fault handler pages from initrd paqfs; stack grows on fault; SC#3's literal wording | ✓ |
| Eager text, fault-grown stack | simpler first bring-up; still owes the demand path | |
| Fully eager, fixed stack | violates SC#3 'growing the user stack' wording | |

**User's choice:** Demand-fault everything
**Notes:** User-fault handler is the phase's core VM deliverable; icache maintenance after text load required (CTR_EL0 discovery per Phase 2 D-03).

| Option | Description | Selected |
|--------|-------------|----------|
| Teardown at first exec | user root table replaces identity map in TTBR0; closes WR-01 | ✓ |
| Teardown right after init_vm | no runtime benefit; risks early-debug print paths | |
| Keep identity map | EL0 could touch MMIO — not viable | |

**User's choice:** Teardown at first exec
**Notes:** Verified during discussion: all runtime MMIO already maps via map_device_register into the TTBR1 kernel table (KZERO+0x100000000000 up) — identity map has no runtime consumer.

| Option | Description | Selected |
|--------|-------------|----------|
| Follow 9front arm64 layout | UTZERO/USTKTOP conventions; donor-proven | |
| r9-specific layout | r9 owns stack top/segment policy/initial frame | ✓ |

**User's choice:** r9-specific layout — overriding the 9front-convention recommendation (r9 owns its VM policy)
**Notes:** Hard constraint recorded: text/data VAs are fixed in the a.out headers by 7l — the loader maps where headers say; freedom is stack top + segment policy only.

| Option | Description | Selected |
|--------|-------------|----------|
| Remove, superseded | delete test_sysexit + raspi user-table experiments when real path lands; git history preserves | ✓ |
| Keep cfg-gated on raspi | dead-on-virt code rots in place | |

**User's choice:** Remove, superseded

---

## Process model scope

| Option | Description | Selected |
|--------|-------------|----------|
| Tick-preemptive round-robin | timer IRQ reschedules; IRQ-from-EL0 exercised now; asserts markers not byte-order | ✓ |
| Cooperative until block/exit | deterministic but preemption untested until Phase 5 | |
| Serialized (no concurrency) | doesn't prove multi-process machinery | |

**User's choice:** Tick-preemptive round-robin
**Notes:** Derived requirement: preemption forces per-process kernel stacks.

| Option | Description | Selected |
|--------|-------------|----------|
| Measure-first on donor | trace hello/init/smoke syscall footprint on 9front donor kernel; implement exactly that + rfork/notes/wait core | ✓ |
| Fixed named list now | predictable but guesses; misses wedge at runtime | |
| Broad 9front subset | untested code lands green — violates execution-gated ordering | |

**User's choice:** Measure-first on donor
**Notes:** Watch: fmt/pool may pull BRK_/SEGBRK into hello's footprint; the trace settles it.

| Option | Description | Selected |
|--------|-------------|----------|
| devroot + devcons | paqfs as root tree + console; fd 0/1/2 preopened at exec | ✓ |
| devcons only | open() unimplemented; weakens SC#4 fd-ops claim | |
| Add env + proc now | serves nothing in staged init/smoke | |

**User's choice:** devroot + devcons — later extended by the notes decision (devproc-lite)

| Option | Description | Selected |
|--------|-------------|----------|
| Alarm note round-trip | self-contained; needs only NOTIFY/NOTED/ALARM | |
| Parent-posts-child note | needs an inter-process posting channel | ✓ |
| Trap-derived note | survivable deliberate fault is fiddly | |

**User's choice:** Parent-posts-child note — overriding the alarm recommendation (exercises the real posting path)
**Notes:** Follow-up resolution: frozen sys.h has NO postnote syscall (postnote(2) writes /proc/N/note), so device set extends to devproc-lite exposing ONLY /proc/N/note. Smoke narrated end-to-end in CONTEXT.md specifics.

| Option | Description | Selected |
|--------|-------------|----------|
| devproc-lite: /proc/N/note only | libc postnote works unmodified; third device scoped to the note path | ✓ |
| Switch to alarm round-trip | no devproc at all; loses parent→child posting | |

**User's choice:** devproc-lite

---

## the agent's Discretion

- Process table size and Proc struct field layout
- First-entry mechanics (seeded Ureg on kernel stack, forkret shape) subject to D-05
- File placement of fault handler / a.out loader / paqfs reader / devices in the r9 crate tree
- await wait-message format (close to Plan 9's convention; full fidelity is Phase 5)
- Timer tick rate / time-slice length
- xtask check failure UX (must fail build/boot-test; exact text free)
- Smoke suite: one binary or several

## Deferred Ideas

- virtio-9p host mount — v1.x top candidate
- Full devproc / devenv / namespace syscalls for real /bin/init — Phase 5
- SMP via PSCI, virtio-net/blk, RTC, PCIe ECAM — v2
- HVF acceleration for virt — needs GICv3 driver
- 9k-amd64 pool-panic re-test with fixed 6c — amd64-side debt (phase-02 deferred-items.md #8)
