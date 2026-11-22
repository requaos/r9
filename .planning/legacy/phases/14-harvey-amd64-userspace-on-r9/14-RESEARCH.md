# Phase 14: Harvey amd64 Userspace on r9 - Research

**Researched:** 2026-11-08
**Domain:** cross-kernel user-mode ABI + exec bring-up (Plan 9 amd64 userspace on a Rust x86_64 kernel), mb2 module delivery, nix flake composition, gate machinery re-keying
**Confidence:** HIGH (every load-bearing ABI fact was read from source in both repos this session; offsets cross-checked against three independent assembly sites)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Design theme (carried from Phase 13 — honor downstream):** modern-only, KVM-native, no legacy compatibility; every locked decision below resolved to the option that keeps ONE implementation of each hard-won discipline.

### ABI contract (XEXEC-01)
- **D-01:** **TrapFrame≡Ureg, 9k-verbatim.** r9's amd64 trap frame IS the Ureg, byte-verbatim from `amd64/include/ureg.h` — no translation anywhere. The same binaries run on BOTH kernels (the k10t1 donor venue executes the identical 6.outs, making XEXEC-04's donor runs a live ABI-conformance witness). The aarch64 Phase-4 D-05/D-07 precedent exactly. — **Reversibility:** one-way — this publishes a cross-repo ABI contract consumed by every amd64 binary; undoing it strands the toolchain's output format.
- **D-02:** **amd64/ABI.md freezes the FULL exec interface**, not just Ureg offsets: Ureg offsets + syscall number mapping (authoritative in `sys/src/libc/9syscall/amd64/sys.h`, restated) + argument/return register convention + entry state + note-delivery convention. One document = the whole kernel/user boundary, one enforcement surface for the planner.
  - *Errata (planning pass 2):* the `sys/src/libc/9syscall/amd64/sys.h` path cited above (and in CONTEXT D-02) is stale — no such file exists; the authoritative number table is the shared `sys/src/libc/9syscall/sys.h` (line 606 below cites it correctly).
- **D-02a (derived from D-01, not separately decided):** the syscall entry mechanism is whatever 9k's existing `9syscall/amd64` stubs + `sys/src/9k/k10/l64syscall.s` already emit — researcher pins the exact register/MSR contract from those stubs; r9 implements it. No fresh entry-mechanism choice exists once the ABI is 9k-verbatim.
- **D-02b:** enforcement carried from aarch64 verbatim: r9-side const asserts + `xtask ureg-check` extended to amd64 + a negative control that fires on drift — live on every gate.

### Bootfs delivery (feeds XEXEC-02/03/04)
- **D-03:** **mb2 module tag off the same GRUB ISO.** GRUB loads the bootfs image alongside the kernel (`module2` line in the committed grub.cfg); the kernel finds it via the mb2 module tags the 13-02 tag walker already parses. One artifact (the reproducible boot-iso), no new qemu devices, loader-mediated lineage per Phase-13 D-02. — **Reversibility:** costly — the module tag threads through grub.cfg, the ISO staging, and the kernel's platform-data parse.
- **D-04:** **bootfs.paq format carried verbatim** from aarch64 (Phase 4/5) — same packed format, same r9 miniz-inflate path. No second archive format; the known inflate heisenbug history is documented and pinned host-side.

### Compose + gates (XEXEC-03/05, INT-05 direction)
- **D-05:** **r9 exports the ISO builder, Harvey composes.** r9's flake exposes the boot-iso derivation parameterized over `{kernel, modules}` — ONE reproducibility implementation (the xorriso-date wrapper, strip staging, FOD hash discipline stay in r9, un-forked). Harvey's flake builds `bootfs.paq` from its own userspace, calls r9's builder with `kernel-x64` + the module, and owns the `boot-r9` app (Phase-5 INT-03 composition shape preserved). — **Reversibility:** costly — the parameterization reshapes r9's flake API surface Harvey consumes.
- **D-06:** **gates split by artifact.** Kernel-level gates (boot-test profiles, ureg-check, the staged-exec legs) stay r9-side `cargo xtask`. Composed-system gates — the `% ` prompt expect smoke (green twice, XEXEC-03) and the in-guest `/net` gates (XEXEC-05) — live Harvey-side, where the composed ISO exists (the `build/aarch64/smoke-r9.expect` discipline re-keyed).
- **D-07:** **XEXEC-05 /net re-gates are in-guest expect-driven** (loopback ping/udp/tcp echo, static + DHCP ipconfig, composed net smoke — the aarch64 Phase-8 gate scripts re-keyed to x86). Kernel-level /net pieces (devether/netif/netgate) stay r9-side where Phase 13 put them.

### Exec bring-up staging (XEXEC-02)
- **D-08:** **staged-init carried** from aarch64 Phase 4/5: first a hello-equivalent static marker exec (ABI + exec + syscall path proven in isolation, its own gate), then boot(8)→init→rc staged next, each leg gated — exec bugs never masquerade as boot-chain bugs.
- **D-09:** **first exec is a purpose-built static marker binary** (the `build/aarch64/smokesleep.c` discipline — minimal libc surface, one syscall path, a serial marker). Real-tree binaries join at the boot(8) stage.

### Claude's Discretion
- The user-mode substrate details on x86_64: ring-3 entry mechanics, per-process address-space shape (CR3 switching vs shared higher-half), FP-state save placement relative to the Ureg — researcher verifies against 9k k10's shapes and PROJECT.md's locked per-process user-memory model; planner sequences.
- The mb2 module tag plumbing details (how the module address/size reach devroot, module relocation if any) and which additional mb2 tags get parsed.
- Bootfs content assembly mechanics (which mkbootfs/paq builder invocation, staging lists) and the CI wiring for the Harvey-side composed smoke.
- The exact tier-0/tier-1 staging lists for XEXEC-04's bootfs (the aarch64 `build/*/tiers/*.txt` allowlists are the template; append-only discipline carried).
- Naming/paths of the new gates and the `boot-r9` x86_64 app wiring inside Harvey's flake.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope (no new capabilities proposed; the bootfs-medium question explicitly routed around virtio-blk/9p as Future Requirements)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| XEXEC-01 | amd64 Ureg/TrapFrame ABI contract published and build-enforced — offsets frozen, const asserts + xtask ureg-check with a negative control | Full byte-offset table re-derived from `amd64/include/ureg.h` and cross-checked against 3 independent asm sites; r9's existing x86_64 Ureg shown NON-conforming (208 vs 184 bytes); uregcheck parser gaps identified (u16int, non-typedef struct); 9k MSR/entry contract pinned from `vsvm.c`/`l64syscall.s` |
| XEXEC-02 | Harvey amd64 binaries exec on r9 x86_64 through the real EL/syscall path (measured-set dispatch carries; staged-init + hello-equivalent proof) | aarch64 exec substrate mapped (exec.rs/proc.rs/syscall.rs ≈ 5,360 lines to port); amd64 a.out facts pinned (S_MAGIC 0x8A97, HEADR 40, INITTEXT 0x200028, INITRND 2 MiB); stack-based syscall arg convention pinned; staged-init + IRQ-masked discipline documented |
| XEXEC-03 | full boot chain boot(8)→init→rc reaches the interactive `% ` prompt on x86_64 with the expect smoke gate green twice consecutively | aarch64 chain (BOOT_ENTRY /boot/boot → boot(8) 2-component walk → /%objtype/init → rcmain/termrc → `% `) mapped; smoke-r9.expect stage list enumerated; mkfile.aarch64 precedent for the amd64 boot(8) build |
| XEXEC-04 | tier machinery re-keyed to `objtype=amd64`: tier-0 libraries + tier-1 core commands proven executing on r9-x86_64 (donor venue per Phase-6 D: Harvey's own 9k kernel, k10t1) | k10t1 venue scripts read (build-9k10t1.sh, boot-9k10t1-sweep.sh — 43-file embed, marker contract, T1-OK/T1-SKIP); allowlist structure + objtype-gated mk filters documented; identical-6.outs donor-witness strategy confirmed feasible |
| XEXEC-05 | `/net` stack re-gated on x86_64: loopback ping/udp/tcp echo, static + DHCP ipconfig, and the composed net smoke all green on the new arch | Phase-8 in-guest fixtures enumerated (lib/netip-*.rc + ping/udpecho/tcpblast/tcpecho + netaddr.rc seam); slirp DHCP shape + venue-honesty rule carried; Phase-13 netgate trigger seam identified as the userspace-swap point |
</phase_requirements>

## Summary

Phase 14 is the x86_64 re-derivation of the aarch64 Phase 4/5 exec bring-up, landing on a platform (r9 x86_64/q35) that Phase 13 deliberately left exec-free: the trap path, GDT/TSS, STAR/FMASK helpers, and netgate exist, but there is no user mode, no syscall entry, and no exec. The single most important technical fact this research established: **r9's existing x86_64 `Ureg` (dat.rs) is NOT the 9k Ureg** — it uses four u64 segment slots (size 208, trapno@152, cs@176) while Harvey's `amd64/include/ureg.h` packs four u16 segment fields into one qword (size 184, type@128, cs@152). D-01's byte-verbatim contract therefore means reworking the frame layout and its assembly producers (`intrcommon` SUBQ/store offsets, UREG_*_OFFSET consts, the CR-01 SWAPPED-witness arithmetic) BEFORE any ring-3 code runs. The 184-byte layout was verified against three independent 9k assembly sites (l64syscall.s syscall path, l64idt.s trap path, syscallreturn's post-ADDQ reads) — all agree.

The second core fact: the **amd64 syscall convention is stack-based, not register-based** — the libc stub spills the first argument (register BP, the 6c `RARG`) into the stack slot at user `SP+8`, the syscall number rides in BP at the `SYSCALL` instruction, and the kernel copies arguments from user `SP+8` upward (`9k k10/syscall.c:251`), returning the result in AX. Wire-shape-wise this is the SAME contract as aarch64 (number in one register, args at SP+8, result in one register) — only the register names, the entry instruction (SYSCALL vs SVC), and the MSR contract (STAR/LSTAR/FMASK, no hardware frame push) differ. The 9k MSR contract is fully pinned from `9k k10/vsvm.c:174-185`: Efer.Sce, Star = user-code-sel<<48 | kernel-code-sel<<32 (the SYSRET +16/+8 selector trick r9's dead `Gdt::star()` already encodes), Lstar = entry, Sfmask = If. r9's existing GDT order (ktext, kdata, udata, utext) is already SYSRET-correct by construction.

The delivery and gate machinery is all re-key, not reinvent: bootfs.paq rides as a GRUB `module2` (mb2 tag 3: u32 mod_start/mod_end physical, 8-aligned tags — spec-confirmed), consumed by the existing arch-neutral `port/src/devroot.rs mount_initrd`/`paqfs.rs`; the tier allowlists and k10t1 donor venue already prove amd64 compilation (6c/6a/6l green since Phase 6's mm venue); the expect smokes, net fixtures, and one-command compose all have direct aarch64 templates.

**Primary recommendation:** sequence the phase as (1) ABI freeze — rework r9's x86_64 Ureg to the 184-byte 9k layout, publish `amd64/ABI.md`, extend const-asserts + ureg-check (with its u16int/non-typedef parser gaps fixed) + negative control; (2) ring-3 + staged exec — SYSCALL entry per the 9k MSR contract, port the aarch64 exec/proc/syscall substrate (measured-set dispatch, validate_user_range, IRQ-masked bootstrap), marker-binary gate; (3) boot chain + compose — mb2 module tag, bootfs bake, parameterized r9 boot-iso, `boot-r9` app, `% `-prompt expect smoke green twice; (4) tiers re-keyed to objtype=amd64 with the k10t1 donor witness; (5) /net gates re-keyed in-guest.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Ureg layout + const asserts + ureg-check | r9 kernel (`x86_64/src/dat.rs`, `xtask/src/uregcheck.rs`) | Harvey (`amd64/include/ureg.h` — frozen source of truth; `amd64/ABI.md` publication) | The frame is kernel-owned; the contract document + header are Harvey-owned; enforcement spans both (D-01/D-02b) |
| SYSCALL/SYSRET entry + MSR arming | r9 kernel (`x86_64/src/trap.rs` naked asm, `seg.rs` star()/cpu.rs fmask() revival) | — | Entry mechanics are 100% kernel-side; contract frozen by ABI.md (D-02a) |
| Exec / proc / syscall dispatch (measured set) | r9 kernel (aarch64 `exec.rs`/`proc.rs`/`syscall.rs` ported to x86_64 arch crate) | `port/` (devroot, paqfs, namec, devcons, devproc — arch-neutral, reuse as-is) | aarch64's arch crates hold the exec substrate; `port/` is shared by construction |
| Ring-3 entry + per-process address space | r9 kernel (`x86_64` crate: CR3/PML4 policy, TSS rsp0) | — | PROJECT.md locks the per-process live-user-memory model (TTBR0-analog = per-process page tables, no copy layer) |
| a.out binary production (6c/6a/6l) | Harvey tree (in-tree toolchain, `nix run .#toolchain`) | — | Already green; amd64 binaries are plain `mk` output, no donor needed |
| bootfs.paq bake | Harvey (`build/x86_64/mkimage.sh` re-key; in-tree mkpaqfs) | — | The byte-format owner is Harvey's mkpaqfs (aarch64 D-discipline) |
| ISO composition (kernel + module) | r9 flake (parameterized boot-iso builder) | Harvey flake (calls the builder, owns `boot-r9` app) | D-05: ONE reproducibility implementation, un-forked, r9-side |
| Kernel-level gates (staged-exec legs, ureg-check) | r9 `cargo xtask` boot-test profiles | — | D-06 artifact split |
| Composed gates (`% ` smoke ×2, /net in-guest) | Harvey (`build/x86_64/*.expect` + flake apps) | — | D-06: composed ISO exists Harvey-side |
| Tier-0/1 execution proof | Harvey (allowlists, k10t1 venue, r9 venue expect) | r9 (the exec substrate the binaries run on) | XEXEC-04; identical 6.outs on both kernels = ABI witness |
| /net kernel pieces (devether/netif/netgate) | r9 (Phase 13 placement, unchanged) | Harvey in-guest gates | D-07 split |

## Standard Stack

### Core

No new packages. The phase is built entirely on existing, in-tree/pinned components:

| Component | Version/State | Purpose | Why Standard |
|-----------|---------------|---------|--------------|
| Harvey 6c/6a/6l | in-tree `sys/src/cmd/6{a,c,l}`, built by `nix run .#toolchain` (BUILDTOOLCHAIN) | amd64 userspace compilation | Already green — the Phase-6 k10t1 venue built the 43-file amd64 tier embed with them [VERIFIED: build/mm/build-9k10t1.sh:56-59 preconditions] |
| Harvey mkpaqfs | in-tree, host-built via `build/aarch64/mkpaqfs-host.sh` | bootfs.paq byte-format owner | D-04 carried verbatim; header magic `529ab12b`, trailer `6b46e688` [VERIFIED: build/aarch64/mkimage.sh:281-289] |
| r9 kernel (x86_64 crate) | HEAD `029e32f` (Phase-13 complete, review fixes in) | target kernel | q35 platform live: mb2 walker, LAPIC/IOAPIC, TSC-deadline, PCI/ECAM, virtio-net, devether |
| `port/` crate (r9) | same | devroot/paqfs/namec/devcons/devproc/devpipe/devmnt/devip — arch-neutral | Phase 13 kept it shared; exec reuses it unchanged |
| miniz_oxide (r9 dep) | pinned in r9 Cargo.lock | paqfs deflate inflate | existing; the heisenbug history is containment-not-registry [VERIFIED: port/src/paqfs.rs:205] |
| r9 x86_64 target spec | `lib/x86_64-unknown-none-elf.json`, `"features": "-mmx,-sse,+soft-float"` | kernel builds SSE-free | user FP state survives syscalls untouched (no kernel XMM clobber) [VERIFIED: lib/x86_64-unknown-none-elf.json] |
| nix: grub2 + xorriso | nixpkgs pinned via flake inputs | reproducible boot-iso | Phase-13's xorriso-date wrapper discipline stays r9-side (D-05) |
| qemu-system-x86_64 | 11.1.2 on this host | gates | KVM-native default, TCG fallback documented |

### Supporting

| Component | Purpose | When to Use |
|-----------|---------|-------------|
| expect | the composed smoke/net gate driver | via flake app runtimeInputs (bare host lacks it — see Environment Availability) |
| plan9port (`$PLAN9/bin` LAST) | rc/mk on the host for the namespace builds | subset/tier builds + boot-9k10t1-sweep |
| Linux mount namespaces (`unshare -m`, sudo) | Harvey amd64 userland build + k10t1 venue | linux-only; the k10t1 script errors loudly elsewhere [VERIFIED: build/mm/build-9k10t1.sh:40-45] |

### Alternatives Considered

None — locked decisions close the choices (D-01..D-09). The only sub-choice space is Claude's-discretion mechanics noted in Open Questions.

**Installation:** nothing to install. Toolchain provisioning on a fresh host is the existing sequence `nix run .#setup && nix run .#toolchain` (+ `nix run .#build` for amd64/bin) — the same preconditions build-9k10t1.sh asserts.

## Package Legitimacy Audit

This phase installs **no new external packages** — no npm/pypi/crates additions, no new nixpkgs packages beyond those Phase 13 already uses (grub2, xorriso, qemu). All Rust dependencies (miniz_oxide, zerocopy, bitstruct, seq_macro) are existing pinned r9 deps. Registry verification: not applicable (no new installs). Disposition: Approved (nothing to audit).

## Architecture Patterns

### System Architecture Diagram

```text
                       ONE COMMAND: nix run .#boot-r9 (x86_64)
                                    |
        +---------------------------+---------------------------+
        | Harvey flake (compose)                                | r9 flake (builder export)
        |  1. amd64 userland build (mk, objtype=amd64,          |  boot-iso derivation parameterized
        |     6c/6a/6l via namespace build)                     |  over {kernel, modules}: xorriso-date
        |  2. bake bootfs.paq (mkimage re-key -> mkpaqfs)       |  wrapper + strip staging + FOD hash
        |  3. call r9's ISO builder(kernel-x64, bootfs.paq)  ---+->  grub.cfg: multiboot2 /boot/kernel
        |  4. qemu-system-x86_64 -M q35 -accel kvm -cdrom ISO       + module2 /boot/bootfs.paq
        v                                                          
   SeaBIOS -> GRUB2 (serial console) -> r9 kernel (mb2 ELF64, higher-half)
        |
        v
   r9 x86_64 boot: mb2 tags (mmap/ACPI/+NEW module tag 3: bootfs phys range)
        |-> map module -> port::devroot::mount_initrd(&'static [u8]) -> paqfs tree
        |-> arming window (LAPIC->timer->IOAPIC->NIC bind, IF=0)     [Phase 13, unchanged]
        |-> [NEW swap point, replaces/extends netgate trigger after sti]
        |      exec_bootstrap (IRQ-masked): first Proc -> exec /boot/boot
        |      -> seed forkret-shaped 184-byte Ureg -> ring 3 via SYSRETQ
        v
   USER (ring 3, cs=utext|3, text@0x200028, stack top r9-owned)
        |  SYSCALL instruction: number in BP, args at user SP+8, result AX
        v
   r9 syscallentry (LSTAR): SWAPGS -> GS:Mach -> TSS rsp0 kernel stack
        |-> software-push hardware frame (ss/sp from R13, flags from R11,
        |    cs from constructed selector, ip from RCX)  [SYSCALL pushes nothing]
        |-> 15 GP + packed u16 ds/es/fs/gs + type(=0 class syscall) + error
        |-> dispatch (measured set): args copied from user SP+8 after
        |    validate_user_range; port/ devices serve 9P-shaped files
        v
   boot(8) -> starts /boot/paqfs (user 9P server) -> mounts root
        -> execs /amd64/init -> init for(;;) fexecs /bin/rc (rcmain/termrc)
        -> INTERACTIVE % PROMPT  <-- expect smoke asserts, green twice
        -> in-guest /net gates: loopback ping/udp/tcp echo, static+DHCP ipconfig
        -> tier-0/1 binaries (same 6.outs) also run on k10t1 donor = ABI witness
```

### Recommended Project Structure

```text
../r9/                                        (fork requaos/r9, branch main)
├── x86_64/src/
│   ├── dat.rs            # Ureg REWORKED to 184-byte 9k layout + offset consts + const asserts
│   ├── trap.rs           # intrcommon re-key (SUBQ 128, packed dseg); NEW syscallentry/return naked asm
│   ├── seg.rs            # star()/utextsel revived (already SYSRET-correct GDT order)
│   ├── cpu.rs            # fmask() revived; wrmsr helpers
│   ├── exec.rs           # NEW: a.out amd64 loader (S_MAGIC 0x8A97, INITTEXT 0x200028, INITRND 2MiB)
│   ├── proc.rs           # NEW: Proc/segments/sched port (aarch64/src/proc.rs keyed to x86_64 frame)
│   ├── syscall.rs        # NEW: SYSCALL dispatch (BP number, SP+8 args, AX result)
│   ├── usermem.rs        # NEW: UserPtr/UserSlice validate (per-process live-user-memory model)
│   └── platform/q35/mb2.rs   # + TAG_MODULE = 3 parse
├── x86_64/lib/
│   ├── config_q35exec.toml    # dev = ["exec"] (staged leg 1)
│   ├── config_q35shell.toml   # dev = ["exec", "bootchain", ...]
│   └── grub.cfg               # + module2 /boot/bootfs.paq line
├── xtask/src/uregcheck.rs     # amd64 arm: u16int fields + non-typedef struct parsing + negative control
└── flake.nix                  # boot-iso parameterized over {kernel, modules}; builder exported

harvey/
├── amd64/ABI.md               # NEW: the frozen full exec interface (publication, XEXEC-01)
├── build/x86_64/              # NEW sibling to build/aarch64/
│   ├── smokesleep.c           # marker binary (re-keyed, 6c-compiled)
│   ├── stage-build.sh / subset-build.sh   # objtype=amd64 drivers (re-keyed)
│   ├── mkimage.sh             # bootfs.paq bake (paths /amd64/init, /amd64/bin/...)
│   ├── smoke-r9.expect        # % prompt smoke (re-keyed: -cdrom ISO, no DTB)
│   └── net gates              # netip-*.rc fixtures re-keyed (XEXEC-05)
├── sys/src/9k/boot/mkfile.amd64   # boot(8) amd64 build (mkfile.aarch64 template)
├── build/mm/                  # k10t1 venue scripts (extend: r9-x64 execution leg)
└── flake.nix                  # bootfs-x64 build + boot-r9 x86_64 app + smoke apps
```

### Pattern 1: The two-layer ABI enforcement (aarch64 Phase-4 shape, re-keyed)

**What:** the Ureg contract is enforced at build time twice — in-crate `const _: () = assert!(offset_of!(...) == N)` per pinned offset + `size_of` check (always on, no Harvey checkout needed), and a cross-repo `cargo xtask ureg-check` parse of the frozen header (fails every boot-test profile on drift), plus a negative control (a mutated header copy must produce a named-field diff + NEGATIVE CONTROL FAILED).

**When to use:** exactly as aarch64 did — the aarch64 asserts read: `const _: () = assert!(core::mem::offset_of!(TrapFrame, x0) == 0);` … `const _: () = assert!(core::mem::size_of::<TrapFrame>() == 288);` [VERIFIED: ../r9/aarch64/src/trap.rs:148-155]. The amd64 arm mirrors this against the 184-byte table below.

**Critical parser gap (discovered this session):** uregcheck.rs "extracts `u64int <name>;` declarations inside `typedef struct Ureg { ... }`" [VERIFIED: ../r9/xtask/src/uregcheck.rs:26-29] — but `amd64/include/ureg.h` is a bare `struct Ureg {` (no typedef) and declares `u16int ds; u16int es; u16int fs; u16int gs;`. The amd64 arm needs (a) `u16int` support with the four-fields-one-qword packing rule, and (b) struct-without-typedef parsing. The pinned field table becomes 22 slots with the 4 u16 segment fields occupying slots 15-18 of a packed qword.

### Pattern 2: Software-built hardware frame on the SYSCALL path

**What:** the `SYSCALL` instruction pushes NOTHING (no SS/RSP/FLAGS/CS/RIP). The entry routine builds the hardware-frame half of the Ureg in software from the architectural registers: RCX=user rip, R11=user flags; user SP saved before switching stacks; SS/CS constructed from user selectors.

**When to use:** the r9 syscallentry port. 9k's precedent (read this session, `sys/src/9k/k10/l64syscall.s:26-48`):

```text
TEXT syscallentry(SB), 1, $-4
	SWAPGS
	BYTE $0x65; MOVQ 0, RMACH     /* m-> (MOVQ GS:0x0, R15) */
	MOVQ	16(RMACH), RUSER       /* m->proc */
	MOVQ	SP, R13                /* save user sp */
	MOVQ	16(RUSER), SP          /* m->proc->kstack */
	ADDQ	$KSTACK, SP
	PUSHQ	$SSEL(SiUDS, SsRPL3)   /* old stack segment */
	PUSHQ	R13                    /* old sp */
	PUSHQ	R11                    /* old flags */
	PUSHQ	$SSEL(SiUCS, SsRPL3)   /* old code segment */
	PUSHQ	CX                     /* old ip */
	SUBQ	$(18*8), SP            /* unsaved registers */
	MOVW	$SSEL(SiUDS, SsRPL3), (15*8+0)(SP)
	MOVW	ES, (15*8+2)(SP)
	MOVW	FS, (15*8+4)(SP)
	MOVW	GS, (15*8+6)(SP)
	PUSHQ	SP                     /* Ureg* */
	PUSHQ	RARG                   /* system call number */
	CALL	syscall(SB)
```

Note the segment stores land at `(15*8+0..6)` — the packed u16 qword at offset 120, exactly ureg.h's `ds,es,fs,gs`. The kernel stack switch goes through the TSS/m->proc->kstack (r9's TSS rsp0 mechanism). SWAPGS on the syscall path is UNCONDITIONAL (SYSCALL only fires from ring 3 — no CS-check/witness logic needed here, unlike the IDT path).

### Pattern 3: Measured-set syscall dispatch with user-pointer validation

**What:** implement exactly the syscall numbers the boot chain measures, loud-fail everything else; validate every user-VA argument against the calling proc's segment map before reading (the per-process live-user-memory model — user pages are mapped in the kernel's view during syscall context; NO Linux-style copy layer, per PROJECT.md's locked decision).

**When to use:** porting aarch64/src/syscall.rs. Its header documents the invariants: "the implemented set is exactly the measured boot-chain set — every other number is loud-unimplemented, never wedged. EVERY user-VA argument … validates against the calling proc's segment map via `Proc::validate_user_range` BEFORE the kernel reads or writes it" [VERIFIED: ../r9/aarch64/src/syscall.rs:8-15]. The amd64 re-key changes only the frame-field accessors (number at `frame.bp`, args at `frame.sp + 8`, result into `frame.ax`).

### Pattern 4: Staged, execution-gated ordering + IRQ-masked exec_bootstrap

**What:** leg 1 = purpose-built static marker exec (own gate); leg 2 = boot(8)→init→rc; every commit bootable. exec_bootstrap runs with interrupts masked and unmasks only at the first user entry.

**When to use:** D-08/D-09. The aarch64 heisenbug record is the reason: "an intermittent (~5–30%, layout-sensitive) corruption of miniz inflate's in-flight state appeared when a tick landed inside exec_bootstrap's `exec_into` read… Exposure eliminated: exec_bootstrap now runs with IRQs masked and unmasks only at the first EL0 entry" [VERIFIED: .planning/milestones/v1.0-phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md:12]. The x86_64 invariant: IF=0 through the whole bootstrap, `sti` only at the ring-3 handoff (or immediately before the SYSRETQ). The same config gates the timer's tick println off under `dev_exec` (aarch64 precedent) — Phase 13's console-MCS-lock vs tick-print hazard applies verbatim.

### Pattern 5: Gates split by artifact + expect-marker discipline

**What:** kernel-level gates stay `cargo xtask boot-test` profiles (serial markers, negative panic scan, presence-not-timing); composed gates are expect scripts spawning qemu directly, staged assertions each naming their failure, fresh-capture hygiene, two-occurrence echo markers (typed echo then output), whole-capture post-passes.

**When to use:** all XEXEC-03/05 gates. The aarch64 smoke enumerates the exact stage vocabulary to re-key: banner → paqfs → stacksmoke → `% ` → echo round-trip → ls/ps/cat sweep → DEL safety → init restart → pid-warning-absent → nic-bind [VERIFIED: build/aarch64/smoke-r9.expect:138-269]. "green twice consecutively" = run the gate twice, both captures retained.

### Anti-Patterns to Avoid

- **Reusing r9's current 208-byte Ureg as-is** — it violates D-01; every offset from 120 up differs. Rework first, ring-3 later. (The CONTEXT watch-out says diff BEFORE any ring-3 code runs.)
- **Copying the IDT stub shape for the SYSCALL entry** — the IDT stubs rely on a CPU-pushed hardware frame + call-pushed vector slot; SYSCALL pushes nothing. Software-build the frame (Pattern 2).
- **Register-args syscall dispatch** — amd64 args come from the user STACK (SP+8), not from registers. Only the number (BP) and the return (AX) are register-borne.
- **Reading SP+0 as an argument** — SP+0 is the stub's return-address slot; args start at SP+8 (the stub's spilled arg0 lands exactly there).
- **Treating nsec/seek like ordinary syscalls** — the stub generator emits NO arg0 spill for `nsec` (`if(! ~ $i nsec) echo MOVQ RARG, 'a0+0(FP)'` [VERIFIED: sys/src/libc/9syscall/mkfile:91-92]) — a generic SP+8 read for nsec would consume the return address. `seek` is emitted as `TEXT _seek`. The dispatch must special-case both (aarch64 ABI.md already notes nsec as "zero-arg register-return").
- **Unmasked kernel computation windows** — see the heisenbug (Pattern 4); also Phase 13's "post-sti boot-context code must mask around driver ops AND console prints".

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| a.out parsing/validation | custom header reader | port aarch64/src/exec.rs's validated `AoutHeader` (be32/be64 readers, checked `.get()`) | 499 lines of proven validation; only magic/base/alignment constants change |
| paqfs consumption | new archive reader | `port/src/paqfs.rs` + `devroot::mount_initrd` (arch-neutral) | D-04: same format, same inflate path, collision-refusal discipline included |
| Frame layout enforcement | ad-hoc tests | const asserts + ureg-check + negative control (aarch64 two-layer shape) | drift must be a build failure with a named-field diff, not a runtime mystery |
| Reproducible ISO | a second ISO pipeline in Harvey | r9's parameterized boot-iso derivation (xorriso-date wrapper, strip staging) | D-05: ONE implementation; grub's non-SOURCE_DATE_EPOCH behavior already solved there |
| Tier allowlists | new hardcoded lists | `build/*/tiers/*.txt` append-only data + the objtype-gated mk filters | D-13 append-only discipline; coverage reports assert against the files |
| Expect gate structure | new harness | re-key `smoke-r9.expect`/net fixtures stage-for-stage | the marker discipline (two-occurrence echo, strip-echoed-line, fresh capture) is load-bearing and measured |
| In-guest net fixtures | new scripts | `lib/netip-{loopback,tcp,static,dhcp}.rc` shapes + netaddr.rc seam | Phase-8 gates re-key, not reinvent (D-07) |

**Key insight:** every piece of this phase exists twice already — once as aarch64 precedent (exec/ABI/boot chain/net gates) and once as amd64 precedent (toolchain, tier builds, k10t1 venue). The phase is two re-keyings joined by one genuinely new artifact (the x86_64 user-mode substrate in r9), and even that has a complete aarch64 sibling to port against.

## Common Pitfalls

### Pitfall 1: r9's existing Ureg silently disagrees with the 9k contract
**What goes wrong:** r9 `x86_64/src/dat.rs` currently declares `ds: u64, es: u64, fs: u64, gs: u64` ("we waste a few bytes to keep the stack aligned") with `UREG_TRAPNO_OFFSET = 19*8` (152) and `UREG_CS_OFFSET = 22*8` (176), total 208 bytes [VERIFIED: ../r9/x86_64/src/dat.rs:11-12,38-46]. Harvey's ureg.h packs the four segments as u16 into ONE qword: type@128, cs@152, total 184.
**Why it happens:** the struct predates the project's ABI discipline (upstream r9 heritage).
**How to avoid:** rework the struct + `intrcommon` producers FIRST (the SUBQ becomes `(1+15)*8 = 128` — 15 GP qwords + 1 packed-dseg qword below the vector slot, so vector=type@128, placeholder/error@136, ip@144, cs@152); update both consts (TRAPNO→16*8, CS→19*8); re-run every Phase-13 gate.
**Warning signs:** any ring-3 crash whose fault RIP decodes to a sane user instruction but a garbage frame; ureg-check diff naming ds/es/fs/gs.

### Pitfall 2: The SWAPPED-witness arithmetic after the layout shift (CR-01 class)
**What goes wrong:** the IDT path's SWAPGS witness rides the HIGH BYTE of the zero-extended saved CS word (`orb ${SWAPPED}, {SWAPPED_CS_OFFSET}(%rsp)` with `SWAPPED_CS_OFFSET = 7`, i.e. byte 7 of the CS qword). Phase 13's CR-01 was exactly this offset landing on the saved RIP word instead (+16 vs +24) — fixed in commit b31a039, dormant until user mode.
**Why it happens:** the witness offsets are frame-layout-relative; moving CS from 176→152 silently moves the witness.
**How to avoid:** when re-keying UREG_CS_OFFSET, re-derive every `SWAPPED_CS_OFFSET`-bearing expression in trap.rs (intrspecial's +24 comment, intrregular's CS_OFFSET=24 checks, intrcommon's btr/restore); keep the fixed comment shape ("+16 would be the saved RIP's high byte") in sync.
**Warning signs:** NMI/debug-exception corruption only under user loads; a witness bit set inside the saved RIP.

### Pitfall 3: mb2 module placement is 32-bit physical
**What goes wrong:** the module tag's mod_start/mod_end are u32 PHYSICAL addresses [CITED: gnu.org multiboot2 manual, tag type 3]; code that assumes a fixed load address (the aarch64 `-device loader,addr=0x48000000` + DTB /chosen pattern) or 64-bit addresses breaks.
**Why it happens:** the aarch64 delivery pinned an address in config; GRUB chooses module placement itself.
**How to avoid:** parse tag 3 per module (one tag per module; tags may repeat; 8-aligned advance `(size+7)&~7`); map whatever phys range GRUB chose (q35 1024M guest keeps modules below 4 GiB by construction); no config addr key needed.
**Warning signs:** boot with no module tag found while grub.cfg carries module2 — check that `module2` (not mb1 `module`) is used INSIDE the multiboot2 menuentry, else GRUB silently drops it.

### Pitfall 4: amd64 data-segment alignment is 2 MiB, not 64 KiB
**What goes wrong:** aarch64 exec.rs computes data_va as "the 64K-aligned VA above text (7l's R_MAGIC alignment)" [VERIFIED: ../r9/aarch64/src/exec.rs:63-70]. 6l's amd64 default is `INITRND = 0x200000` with `INITTEXT = 0x200000+HEADR` [VERIFIED: sys/src/cmd/6l/obj.c:154-161] — the data segment lands at the next 2 MiB boundary above text.
**Why it happens:** arch linker defaults differ; the aarch64 constants were derived from 7l.
**How to avoid:** the amd64 exec port recomputes text_va=0x200000-mapped-from (header included, TEXT_MAP_BASE analog 0x200000), data_va 2 MiB-aligned; assert against a real 6l binary's header at bring-up (the aarch64 research discipline: "verified against a real 7l binary's bytes").
**Warning signs:** exec maps data over text or bss over the following page; a.out header parse succeeds but data_va < text end.

### Pitfall 5: nsec's missing arg0 spill
**What goes wrong:** for `nsec` the stub generator skips `MOVQ RARG, a0+0(FP)` — a generic kernel arg-copy from SP+8 reads the return-address slot.
**Why it happens:** nsec is zero-arg register-return on 64-bit (ABI.md aarch64 notes the same).
**How to avoid:** dispatch nsec (53) from the frame directly (no stack read); remember `exits` is emitted `_exits` and `seek` as `_seek` (TEXT renames only — numbers unchanged).
**Warning signs:** nsec returning absurd values or consuming the wrong slot; watch also that `SYSCALL` in 6a assembles to the syscall instruction with the REX prefix handled by the assembler (9k's l64syscall.s uses `BYTE $0x48; SYSRET` for SYSRETQ only — plain `SYSCALL` needs no prefix).

### Pitfall 6: composed-ISO reproducibility with a Harvey-built module
**What goes wrong:** r9's boot-iso is a fixed-output derivation whose hash was wired only after a two-derivation byte-identity proof. Parameterizing over a Harvey-built bootfs.paq means the ISO bytes now depend on Harvey's bake — a non-deterministic bake (mtimes, ordering) churns or breaks the FOD.
**Why it happens:** mkpaqfs bakes from a staged tree; staging order/mtimes can leak.
**How to avoid:** carry the KERNDATE/mkpaqfs determinism discipline from the aarch64 bake into the x86_64 mkimage; decide where the FOD boundary sits after parameterization (r9 exports the BUILDER; the composed ISO's hash discipline is part of the D-05 flake-API design — see Open Questions).
**Warning signs:** `nix build --rebuild` hash mismatch on the composed ISO.

### Pitfall 7: FP/SSE state and CR4.OSFXSR for user float canaries
**What goes wrong:** tier-1 carries float canaries (bc/awk `T1-OK float-d01` mandatory on every venue). r9's kernel is SSE-free (`-mmx,-sse,+soft-float`) so kernel code never clobbers XMM — but user SSE instructions executing in ring 3 require CR4.OSFXSR=1 (and OSXMMEXCPT for #XM) or they #UD.
**Why it happens:** GRUB/SeaBIOS likely leaves CR4 friendly, but nothing in r9's boot explicitly owns that state.
**How to avoid:** verify/program CR0 (EM=0, TS per policy) and CR4 (OSFXSR, OSXMMEXCPT) at exec bring-up; document that user XMM persists across syscalls BECAUSE the kernel is SSE-free (the aarch64 "handler uses no FP/SIMD" argument, x86 edition); revisit if the kernel ever gains SSE.
**Warning signs:** tier-1 float canaries faulting with #UD/#NM on r9-x64 while clean on k10t1.

### Pitfall 8: tick-print vs dev_exec console lock
**What goes wrong:** Phase 13's timer tick prints from IRQ context (safe while the idle loop prints nothing); the aarch64 exec configs cfg-gate the tick print OFF under dev_exec because syscall context takes the Console MCS lock.
**Why it happens:** two contexts, one lock.
**How to avoid:** the q35exec/q35shell configs disable the tick println (aarch64 precedent) or adopt the masked-post discipline netgate already uses.
**Warning signs:** wedge at the first syscall that prints while a tick is pending.

### Pitfall 9: ureg-check's parser is deliberately narrow
**What goes wrong:** extending to amd64 by relaxing the parser until it passes (e.g. ignoring the u16 fields) would hollow out the gate.
**Why it happens:** the parser matches only `u64int <name>;` inside `typedef struct Ureg`.
**How to avoid:** extend it to pin the u16 quartet EXPLICITLY (names ds, es, fs, gs at slots 15-18 packed at byte 120) and the bare `struct Ureg {` form; the negative control must still fire on a drifted header copy (04-03 precedent: mutate a copy, expect named-field diff + "NEGATIVE CONTROL FAILED").

### Pitfall 10: slirp venue honesty on the /net gates
**What goes wrong:** green loopback/DHCP gates on slirp being read as L2 correctness.
**Why it happens:** "slirp forgives L2 bugs; it is not a correctness target" [VERIFIED: 14-CONTEXT.md specifics].
**How to avoid:** label the XEXEC-05 gates as the aarch64 Phase-8 re-key (loopback/ipconfig/DHCP under slirp); the tap-leg truth gate is Phase 16 (NETIP-08), out of scope here.

## Code Examples

### The frozen Ureg — source of truth, verbatim

Source: `amd64/include/ureg.h:1-30` (read this session; this is the whole file):

```c
struct Ureg {
	u64int	ax;
	u64int	bx;
	u64int	cx;
	u64int	dx;
	u64int	si;
	u64int	di;
	u64int	bp;
	u64int	r8;
	u64int	r9;
	u64int	r10;
	u64int	r11;
	u64int	r12;
	u64int	r13;
	u64int	r14;
	u64int	r15;

	u16int	ds;
	u16int	es;
	u16int	fs;
	u16int	gs;

	u64int	type;
	u64int	error;				/* error code (or zero) */
	u64int	ip;				/* pc */
	u64int	cs;				/* old context */
	u64int	flags;				/* old flags */
	u64int	sp;				/* sp */
	u64int	ss;				/* old stack segment */
};
```

### The pinned byte-offset table for ABI.md (computed from the struct, cross-checked)

Computed with C layout rules (8-byte alignment; the u16 quartet at 120..126 needs no padding before `type`):

| Field | Offset | Width | Cross-check site |
|-------|--------|-------|------------------|
| ax | 0 | 8 | l64syscall.s `MOVQ 16(SP), AX /* Ureg.ax */` at SP=ureg-16 |
| bx | 8 | 8 | — |
| cx | 16 | 8 | touser `MOVQ $(UTZERO+0x28), CX /* ip */` (CX carries ip at entry) |
| dx | 24 | 8 | — |
| si | 32 | 8 | — |
| di | 40 | 8 | — |
| bp | 48 | 8 | syscallreturn `MOVQ (16+6*8)(SP), BP /* Ureg.bp */` (16+48=64=SP+16+48) |
| r8..r15 | 56-112 | 8 ea | intrcommon push order (13 pushes + 2-slot hole) |
| ds | 120 | 2 | syscallentry `MOVW $SSEL(...), (15*8+0)(SP)` = 120 |
| es | 122 | 2 | syscallentry `(15*8+2)(SP)` |
| fs | 124 | 2 | syscallentry `(15*8+4)(SP)` |
| gs | 126 | 2 | syscallentry `(15*8+6)(SP)` |
| type | 128 | 8 | 9k IDT: vector XCHG lands here; r9: `shrw $3` vector slot |
| error | 136 | 8 | CPU error code (7 classes) or stub placeholder |
| ip | 144 | 8 | syscallreturn post-ADDQ `MOVQ 24(SP), CX /* ip */` (SP'=ureg+120; +24=144) |
| cs | 152 | 8 | post-ADDQ `CMPW 48(SP)` family: SP'+48=ureg+168=sp… cs reads: 9k `userureg`: `ureg->cs == SSEL(SiUCS, SsRPL3)` |
| flags | 160 | 8 | post-ADDQ `MOVQ 40(SP), R11 /* flags */` (+40=160) |
| sp | 168 | 8 | post-ADDQ `MOVQ 48(SP), SP /* sp */` (+48=168) |
| ss | 176 | 8 | highest; constructed on SYSCALL path, CPU-pushed on IDT path |
| **sizeof** | **184** | | 23 qwords |

Independent confirmation: 9k's IDT path (`sys/src/9k/k10/l64idt.s`) builds the identical 23-qword frame via `SUBQ $24` + 13 pushes + the R14/R15/segment hole; both entries agree with ureg.h field-for-field. r9's current frame agrees ONLY through r15@112.

### The user-side stub (what every 6.out emits)

Source: `sys/src/libc/9syscall/mkfile:81-95` (case amd64, verbatim):

```make
case amd64
	if(~ $i seek)
		echo TEXT _seek'(SB)', 1, '$0'
	if not
		echo TEXT $i'(SB)', 1, '$0'
	#
	# For architectures which pass the first argument
	# in a register, if the system call takes no arguments
	# there will be no 'a0+0(FP)' reserved on the stack.
	#
	if(! ~ $i nsec)
		echo MOVQ RARG, 'a0+0(FP)'
	echo MOVQ '$'$n, RARG
	echo SYSCALL
	echo RET
```

With `RARG` = `REGARG` = `D_BP` in 6a/6c [VERIFIED: sys/src/cmd/6a/lex.c:210; sys/src/cmd/6c/6.out.h:916 `REGARG = D_BP, /* MIGHT CHANGE */`]. Wire contract: **number in BP; args at user SP+8 upward (arg0 = the stub's spilled BP); result in AX.**

### The kernel-side MSR arming (9k precedent, verbatim)

Source: `sys/src/9k/k10/vsvm.c:174-185`:

```c
	wrmsr(FSbase, 0ull);
	wrmsr(GSbase, PTR2UINT(&sys->machptr[m->machno]));
	wrmsr(KernelGSbase, 0ull);

	r = rdmsr(Efer);
	r |= Sce;
	wrmsr(Efer, r);
	r = ((u64int)SSEL(SiU32CS, SsRPL3))<<48;
	r |= ((u64int)SSEL(SiCS, SsRPL0))<<32;
	wrmsr(Star, r);
	wrmsr(Lstar, PTR2UINT(syscallentry));
	wrmsr(Sfmask, If);
```

Notes for r9: its dead `Gdt::star()` computes `((UTEXTSEL - 16) | RPL_USER) << 48 | KTEXTSEL << 32` with the +16/+8 SYSRET selector math explained in-comment (SYSRET CS=STAR[48..]+16, SS=STAR[48..]+8; AMD-vs-Intel RPL portability handled by the explicit OR) [VERIFIED: ../r9/x86_64/src/seg.rs:238-273]. r9's GDT order (ktext@8, kdata@0x10, udata@0x18, utext@0x20) is already SYSRET-correct. 9k masks only IF (`Sfmask = If`); r9's dead `fmask()` has IF|TF|DF — follow the 9k donor (D-02a: whatever 9k emits). 9k sets user GS base to 0 (KernelGSbase=0) — user code does not use GS.

### The first-user-entry contract (9k touser, verbatim excerpt)

Source: `sys/src/9k/k10/l64syscall.s:8-22`:

```text
TEXT touser(SB), 1, $-4
	CLI
	SWAPGS
	MOVQ	$SSEL(SiUDS, SsRPL3), AX
	MOVW	AX, DS
	MOVW	AX, ES
	MOVW	AX, FS
	MOVW	AX, GS

	MOVQ	$(UTZERO+0x28), CX		/* ip */
	MOVQ	$If, R11			/* flags */

	MOVQ	RARG, SP			/* sp */

	BYTE $0x48; SYSRET			/* SYSRETQ */
```

CX=user ip, R11=flags, BP(RARG)=user sp; DS/ES/FS/GS loaded with the flat user data selector; `UTZERO = (0+2*MiB)` [VERIFIED: sys/src/9k/k10/mem.h:57] and +0x28 (40) = past the a.out header = the linked entry (matches 6l `INITTEXT = 0x200000+HEADR`, HEADR=40). For exec'd processes the entry comes from the a.out header's `entry` field; for the seeded first process r9 plants the equivalent in the frame.

### The kernel-side argument read (9k precedent, verbatim)

Source: `sys/src/9k/k10/syscall.c:248-254`:

```c
		if(sp < (USTKTOP-PGSZ) || sp > (USTKTOP-sizeof(up->arg)-BY2SE))
			validaddr(UINT2PTR(sp), sizeof(up->arg)+BY2SE, 0);

		memmove(up->arg, UINT2PTR(sp+BY2SE), sizeof(up->arg));
		up->psstate = systab[scallnr].n;

		systab[scallnr].f(&ar0, (va_list)up->arg);
```

Args copied from user `sp+BY2SE` (sp+8) after validating the stack range; handlers receive `(Ar0*, va_list)`; return: `ureg->ax = ar0.p;` [VERIFIED: syscall.c:280]. 9k's `USTKTOP 0x00007ffffffff000ull`, `USTKSIZE (16*1024*1024)` [VERIFIED: mem.h:59-60] are the donor reference; r9 owns its own stack-top policy (aarch64 used 0x0000_8000_0000_0000 — the x86_64 choice is a discretion item).

### The note-delivery convention (what ABI.md must also freeze)

Source: `sys/src/9k/k10/syscall.c:182-192` — the notify path copies the Ureg into a user-allocated NFrame below the user sp, passes the handler its ureg pointer as arg0 through **ureg->bp** (the RARG register — consistent with the 6c first-arg convention), and resumes at `up->notify`:

```c
	memmove(&nf->ureg, ureg, sizeof(Ureg));
	...
	nf->arg1 = nf->msg;
	nf->arg0 = &nf->ureg;
	ureg->bp = PTR2UINT(nf->arg0);
	nf->ip = 0;
	ureg->sp = sp;
	ureg->ip = PTR2UINT(up->notify);
```

smokesleep.c (the marker-binary template) exercises exactly this: `notify(handler)`, `alarm(300)`, `sleep(30000)`, handler checks `ureg == nil`, prints, `noted(NCONT)` [VERIFIED: build/aarch64/smokesleep.c:34-63].

### mb2 module tag (external, spec-confirmed)

Tag type 3 (`MULTIBOOT_TAG_TYPE_MODULE`): `{ u32 type; u32 size; u32 mod_start; u32 mod_end; u8 cmdline[] }` — mod_start/mod_end are 32-bit physical addresses; size excludes padding; tags advance `(size+7)&~7`; one tag per module, may repeat [CITED: gnu.org/software/grub/manual/multiboot2/multiboot.html]. GRUB side: `module2 /boot/bootfs.paq "bootfs"` inside the menuentry after `multiboot2 /boot/kernel` — plain `module` is the mb1 directive and mixing them silently drops modules [CITED: phip1611.de minimal-grub-configuration; ACRN docs]. r9's walker already validates "tag sizes are legitimately non-8-multiples … only tag POSITIONS are aligned" [VERIFIED: ../r9/x86_64/src/platform/q35/mb2.rs:14-17] — the module tag obeys the same rule.

## State of the Art

Internal lineage (this project's own art):

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| r9 x86_64 Ureg: 4×u64 segments, 208 bytes (upstream heritage) | 9k-verbatim 184-byte Ureg (D-01) | This phase | Frame producers + all offset consts rework; ABI publish |
| aarch64 initrd: fixed addr + DTB /chosen patch | mb2 module tag (D-03) | This phase | grub.cfg `module2` + tag-3 parse; no fixed address |
| r9 boot-iso: self-contained FOD, kernel only | parameterized `{kernel, modules}` builder (D-05) | This phase | Harvey composes; reproducibility stays r9-side |
| tier gates: aarch64-keyed allowlists + k10t1 donor | objtype=amd64 re-key; donor doubles as ABI witness (XEXEC-04) | This phase | identical 6.outs on both kernels |
| 9k k10 syscalls: full systab + generated handlers | r9 measured-set dispatch, loud-unimplemented rest | aarch64 Phase 4 | carried to x86_64 unchanged in spirit |

**Deprecated/outdated (do not use):** r9's old-generation x86_64 locals that Phase 13 superseded; 9k's `sys/src/9/` legacy kernel tree as any reference for this work (9k = `sys/src/9k/`); mb1 (`module` directive, `-kernel` loads).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | 6l amd64 binaries land at data_va = next 2 MiB above text when linked with default mk flags (derived from obj.c INITRND=0x200000; NOT yet verified against a real amd64 6.out's header bytes) | Pitfall 4 / exec port | exec maps data wrong; the bring-up must assert against a real binary's header (the aarch64 "verified against a real binary's bytes" discipline) |
| A2 | GRUB/SeaBIOS leaves CR4.OSFXSR/CR0 in a state where ring-3 SSE executes, and r9 should own that state explicitly | Pitfall 7 | float canaries #UD on r9-x64 |
| A3 | The parameterized boot-iso keeps a workable FOD discipline (builder exported; composed ISO's hash handled Harvey-side or per-input) — exact mechanism is a planner design decision | D-05 / Open Questions | reproducibility gate churns; `--rebuild` proofs fail |
| A4 | boot(8)'s embed-only method table (bootr9.c hand-written bootconf) works on amd64 via an `mkfile.amd64` with the same trimmed BOOTFILES | XEXEC-03 | boot chain build breaks on arch-specific bootconf assumptions |
| A5 | The aarch64 syscall.rs measured set (BIND..PREAD etc., ~30 numbers of 0-54) carries over with the same membership for the amd64 boot chain — amd64 binaries use the same libc syscall surface | Pattern 3 | dispatch gaps surface as loud-unimplemented during the boot chain (fail-loud, not silent — low risk) |
| A6 | mb2 module stays below 4 GiB and within mappable RAM on the q35 1024M guest (GRUB loads modules in low memory) | Pitfall 3 | module unfindable/unmappable; would need a bigger guest or relocation |

All other claims in this document carry [VERIFIED: path] or [CITED: url] tags from files read or fetched this session.

## Open Questions

Resolution state (planning pass, 2026-11-08): all four questions below are RESOLVED in executable plan content — per-question markers cite the resolving plan task.

1. **The parameterized boot-iso flake API shape (D-05)** — RESOLVED (plan 14-07 Task 1: BOTH shapes from one source — an argv-driven `iso-builder` script derivation plus a flake-lib function deriving from the same code; the composed ISO is reproducible-by-construction with the kernel-only FOD hash preserved and proven by `nix build .#boot-iso --rebuild`)
   - What we know: current r9 boot-iso is a self-contained FOD with a hash wired after a byte-identity proof; Harvey must call a builder with kernel-x64 + a module it builds itself.
   - What's unclear: whether the export is a flake lib function (`r9.lib.bootIso {kernel, modules}` returning a derivation Harvey FOD-wraps) or a package parameterized via an input overlay; where the composed ISO's outputHash discipline lives.
   - Recommendation: planner decides with a small design note in the plan; either shape keeps the xorriso-wrapper/strip staging un-forked in r9. Costly to reverse (D-05's own rating) — decide once, early.
2. **Per-process address-space shape on x86_64 (discretion item)** — RESOLVED (plan 14-04 Task 1 implements the recommendation verbatim — per-process CR3/PML4 with the kernel higher half shared, TSS rsp0 = kernel stack top; the entry-state and stack-bound donor references are restated in 14-01 Task 2's ABI.md)
   - What we know: PROJECT.md locks the per-process live-user-memory model (the TTBR0-live analog — user pages visible in kernel context, no copy layer); 9k uses per-process CR3 with the higher-half kernel mapped; kernel text/data live at 0xffffffff80000000+ (KZERO); user text at 0x200028; r9's own aarch64 chose USTACKTOP 0x0000_8000_0000_0000.
   - What's unclear: the concrete x86_64 choice (per-process PML4 + shared kernel half is the obvious conforming shape) and the TSS-rsp0 vs kstack-field mechanics r9's port.rs will use.
   - Recommendation: mirror 9k's shape (per-process CR3, kernel half shared, TSS rsp0 = kernel stack top); record in the plan as the locked discretion resolution.
3. **Where the x86_64 exec substrate lives (arch crate vs port)** — RESOLVED (plan 14-04 Task 1: straight port into the x86_64 arch crate, cfg-gated per Phase 13's port-sharing rule; no port/ refactor until a third arch needs it — the recommendation adopted as written)
   - What we know: aarch64 keeps exec.rs/proc.rs/syscall.rs/usermem.rs in the arch crate (~5,360 lines total); port/ holds the arch-neutral devices. The x86_64 crate already has its own dat/trap/seg.
   - What's unclear: how much factors into port/ during the port (e.g. a shared frame-trait) vs straight arch-crate duplication.
   - Recommendation: straight port first (cfg-gated per Phase 13's port-sharing rule); refactor toward port/ only if a third arch needs it. Avoid premature abstraction.
4. **grub.cfg staging for the module (committed file vs template)** — RESOLVED (plan 14-05 Task 1, carried by 14-07 Task 1: TEMPLATED — module2 lines emitted IN when a module is staged, OUT for kernel-only ISOs; GRUB fails a menuentry whose module file is absent, and kernel-only ISO bytes plus the wired boot-iso FOD hash must stay unchanged — this diverges from the unconditional-line recommendation below, which predated the FOD-preservation constraint)
   - What we know: grub.cfg is committed verbatim in r9 (`x86_64/lib/grub.cfg`), staged by xtask AND the flake; the module line references a path that only exists in the composed ISO.
   - What's unclear: whether the committed grub.cfg gains the module2 line unconditionally (kernel-only dev boots tolerate a missing module with a loud parse note) or the builder templates it.
   - Recommendation: unconditional line + loud "no module tag" behavior in dev boots is simplest and matches the loader-mediated lineage; planner's call.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qemu-system-x86_64 | all gates | ✓ | 11.1.2 | — |
| KVM (/dev/kvm) | KVM-native gate default | ✓ | — | TCG + qemu64 (documented, Phase-13 shape) |
| nix | flake builds/apps | ✓ | 2.34.9 | — |
| expect | composed smoke/net gates | ✗ (bare host) | — | provided by flake-app runtimeInputs (the aarch64 apps already do this — not blocking) |
| Harvey amd64 toolchain (6c/6a/6l in `.build/hostbin-linux-x86_64/bin`) | userland + k10t1 builds | ✗ not provisioned on this host | — | `nix run .#setup && nix run .#toolchain` (existing apps; first plan task) |
| amd64/bin + amd64/lib populated | tier embeds, bootfs bake | ✗ not provisioned | — | `nix run .#build` (namespace build; linux-only, works here) |
| sudo + unshare -m | namespace builds, k10t1 venue | ✓ (linux host) | — | darwin: lima guest (this host is linux — fine) |
| r9 x86_64 kernel build | kernel gates | ✓ (target/x86_64-unknown-none-elf/debug/x86_64 exists) | HEAD 029e32f | — |
| git remotes | push-before-pin (r9 → requaos/r9; harvey) | ✓ both repos are git checkouts | — | hosted CI pending first push for Phase-13 commits (recorded in 13-VERIFICATION) |

**Missing dependencies with no fallback:** none — the two unprovisioned items (amd64 toolchain + amd64/bin) are produced by existing flake apps, which is plan work, not blockers.

**Missing dependencies with fallback:** expect (via nix runtimeInputs).

## Security Domain

`security_enforcement` is absent from `.planning/config.json` → treated as enabled. This phase's threat surface is narrow (no auth, no network exposure beyond qemu slirp, no user data).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | auth is Phase 18; bootfs ships NO /boot/factotum (auth-stub-by-absence carried) |
| V3 Session Management | no | no sessions this phase |
| V4 Access Control | no | single-user guest, no multi-tenant surface |
| V5 Input Validation | yes | (a) every user-VA syscall argument validated via the proc's segment map (`validate_user_range`) BEFORE kernel read/write — the aarch64 invariant carried; (b) the mb2 module tag is untrusted firmware input — checked-slice gets, named ParseError, never a silent skip (13-02 discipline); (c) the a.out header fully validated before mapping (exec.rs's checked readers); (d) devroot refuses an image shipping its own /rootfs.img entry (collision discipline) |
| V6 Cryptography | no | none introduced |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| User pointer dereference in syscall handlers (kernel takeover via crafted SP/args) | Escalation/Elevation | validate_user_range before ANY user-VA access; args come from the user stack — validate the stack range first (9k syscall.c:248-251 precedent) |
| Malformed a.out header (text/data/bss overflow, entry outside text) | Tampering | exec.rs's validated AoutHeader + byte-fit checks + entry-in-text assertions (port the aarch64 checks; A1's data_va assert is part of this) |
| Firmware-provided mb2 structures (module bounds) | Tampering | the walker's two-pass total_size reconciliation + per-tag bounds/alignment validation; module range checked against the mb2 memory map before mapping |
| Bootfs image collision/shadowing | Tampering | devroot's rootfs.img collision refusal (existing) |
| FOD hash laundering via the composed ISO | Repudiation | the two-derivation byte-identity proof discipline before wiring any outputHash (Phase-13 precedent; Pitfall 6) |

## Sources

### Primary (HIGH confidence — read this session)
- `amd64/include/ureg.h` (whole file) — the D-01 source of truth
- `sys/src/9k/k10/l64syscall.s` (whole file) — syscallentry/syscallreturn/touser entry contract
- `sys/src/9k/k10/l64idt.s` (whole file) — IDT frame construction, error-code slot variance
- `sys/src/9k/k10/vsvm.c:130-192` — STAR/LSTAR/FMASK/GSbase MSR arming, TSS/GDT init, userureg
- `sys/src/9k/k10/syscall.c:160-305` — note delivery, arg copy at sp+8, return in ax
- `sys/src/9k/k10/mem.h:34-61` — KSTACK/UTZERO/USTKTOP/USTKSIZE
- `sys/src/libc/9syscall/sys.h` (whole file) — the frozen number table 0-54
- `sys/src/libc/9syscall/mkfile:81-95` — the amd64 stub generator (RARG spill + SYSCALL)
- `sys/src/cmd/6a/lex.c:210`, `sys/src/cmd/6a/a.h`, `sys/src/cmd/6c/6.out.h:838-916` — RARG/REGARG = D_BP
- `sys/src/cmd/6l/obj.c:144-189` + `sys/include/a.out.h:14-33` — HEADTYPE 2 defaults, S_MAGIC(26), _MAGIC formula
- `../r9/x86_64/src/dat.rs:1-110` — current (non-conforming) Ureg + offset consts
- `../r9/x86_64/src/trap.rs:1-419` — stubs/intrspecial/intrregular/intrcommon (frame construction, SWAPPED witness)
- `../r9/x86_64/src/seg.rs:189-274` — Gdt layout, dead star()/utextsel with SYSRET selector math
- `../r9/x86_64/src/cpu.rs:20-40` — dead fmask()
- `../r9/x86_64/src/main.rs:130-200` — arming order, netgate trigger (the Phase-14 swap point)
- `../r9/x86_64/src/platform/q35/mb2.rs` — tag walker, validation discipline
- `../r9/lib/x86_64-unknown-none-elf.json` — SSE-free kernel target
- `../r9/aarch64/ABI.md` (whole file) — the publication template + enforcement appendix
- `../r9/aarch64/src/exec.rs:1-80` — a.out loader facts (R_MAGIC 0x8C47, HDR_SIZE 40, INITTEXT, USTACKTOP policy)
- `../r9/aarch64/src/syscall.rs:1-70` — measured-set dispatch invariants
- `../r9/aarch64/src/main.rs:440-560` — exec_bootstrap orchestration + hook registrations
- `../r9/aarch64/src/trap.rs:148-155` — const-assert pattern
- `../r9/xtask/src/uregcheck.rs:1-60` — parser shape + the u16int/non-typedef gap
- `../r9/xtask/src/main.rs:1471-1485` — boot-test arch gating of ureg-check ("x86_64 skips it: … Phase 14's XEXEC-01")
- `../r9/flake.nix:100-205` — kernel-x64 + boot-iso FOD mechanics
- `../r9/x86_64/lib/grub.cfg` — current loader config
- `harvey/flake.nix` + `nix/boot-r9.sh` — compose app pipeline
- `build/aarch64/mkimage.sh` — bake pipeline + magic contract
- `build/aarch64/smoke-r9.expect` — the stage vocabulary
- `build/aarch64/smokesleep.c` — marker binary discipline
- `build/aarch64/tiers/tier0-libs.txt` + `tier1-cmds.txt` — allowlist structure
- `build/mm/build-9k10t1.sh` + `boot-9k10t1-sweep.sh` — donor venue mechanics
- `.planning/phases/13-r9-x86-64-platform-bring-up/13-CONTEXT.md`, `13-VERIFICATION.md` — platform decisions, CR-01 fix state, open findings
- `.planning/milestones/v1.0-phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md` — the inflate heisenbug containment record

### Secondary (MEDIUM confidence)
- [Multiboot 2 specification (GNU)](https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html) — module tag type 3 layout
- [Minimal GRUB configuration for a Multiboot2 kernel with boot modules (phip1611.de)](https://phip1611.de/blog/minimal-grub-configuration-to-boot-a-multiboot2-kernel-with-boot-modules) — module2 directive usage

### Tertiary (LOW confidence)
- None — no claim in this document rests on an unverified web source; external claims are limited to the two Secondary entries.

## Metadata

**Confidence breakdown:**
- ABI/Ureg facts: HIGH — read from source in both repos; offsets computed and cross-checked against three independent assembly sites
- Syscall entry contract: HIGH — stub generator, kernel asm, MSR arming, and arg-copy all read verbatim this session
- r9 current-state facts: HIGH — files read at HEAD 029e32f; note the knowledge graph was 40h/24-commits stale and was NOT trusted where it disagreed with the tree (the vsvm.rs node no longer exists; its Gdt/Tss content now lives in seg.rs)
- Bootfs/compose/gate machinery: HIGH — scripts and flakes read this session
- amd64 data-segment layout (A1): MEDIUM — derived from linker source, not yet checked against a real binary's bytes
- aarch64 substrate port size/shape: HIGH for what exists, MEDIUM for effort estimates (not attempted here)

**Research date:** 2026-11-08
**Valid until:** 2026-12-08 (stable internal lineage; re-check r9 HEAD if commits land past 029e32f, and re-verify A1 the moment a real amd64 6.out exists on the built host)
