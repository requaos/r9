# Phase 14: Harvey amd64 Userspace on r9 - Context

**Gathered:** 2026-11-08
**Status:** Ready for planning

<domain>
## Phase Boundary

The core value lands on x86_64: Harvey amd64 binaries exec on r9 through the re-derived ABI contract, the full boot chain boot(8)→init→rc reaches the interactive `% ` prompt via one command (`nix run .#boot-r9`), and the tier + `/net` gate machinery runs keyed to `objtype=amd64` — no new toolchain work (6c/6a/6l already green). **Work spans both repos plus the flake glue**: the exec/ABI/user-mode substrate lands in `../r9` (fork `requaos/r9`, branch `main`); Harvey-side work is the bootfs build, the compose (`boot-r9`), the re-keyed tier + expect gates, and evidence consolidation.

Addresses XEXEC-01, XEXEC-02, XEXEC-03, XEXEC-04, XEXEC-05.

NOT in this phase: the fuller userspace tiers + bootfs payload + DESK-03 (Phase 15 — this phase's tier scope is tier-0 libs + tier-1 core commands as execution proof, XEXEC-04), host net modes / tap leg / NETIP-08 (Phase 16), kernel flush-out fixes (Phase 17), auth (Phase 18), cpu/drawterm/rio (Phase 19). virtio-blk, virtio-9p, SMP stay Future Requirements.

</domain>

<decisions>
## Implementation Decisions

**Design theme (carried from Phase 13 — honor downstream):** modern-only, KVM-native, no legacy compatibility; every locked decision below resolved to the option that keeps ONE implementation of each hard-won discipline.

### ABI contract (XEXEC-01)
- **D-01:** **TrapFrame≡Ureg, 9k-verbatim.** r9's amd64 trap frame IS the Ureg, byte-verbatim from `amd64/include/ureg.h` — no translation anywhere. The same binaries run on BOTH kernels (the k10t1 donor venue executes the identical 6.outs, making XEXEC-04's donor runs a live ABI-conformance witness). The aarch64 Phase-4 D-05/D-07 precedent exactly. — **Reversibility:** one-way — this publishes a cross-repo ABI contract consumed by every amd64 binary; undoing it strands the toolchain's output format.
- **D-02:** **amd64/ABI.md freezes the FULL exec interface**, not just Ureg offsets: Ureg offsets + syscall number mapping (authoritative in `sys/src/libc/9syscall/amd64/sys.h`, restated) + argument/return register convention + entry state + note-delivery convention. One document = the whole kernel/user boundary, one enforcement surface for the planner.
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 14 — goal, success criteria 1–5, requirement mapping
- `.planning/REQUIREMENTS.md` — XEXEC-01..05 full text; Out of Scope table; Future Requirements (virtio-blk/9p/SMP excluded)
- `.planning/PROJECT.md` — Key Decisions (per-process user-memory model locked; push-before-pin; tier allowlists as data), Constraints
- `.planning/phases/13-r9-x86-64-platform-bring-up/13-CONTEXT.md` — the platform decisions this phase builds on (D-02 mb2/GRUB2, the netgate trigger seam, kernel-x64/boot-iso packages)

### The ABI contract (XEXEC-01)
- `amd64/include/ureg.h` — THE source of truth D-01 freezes byte-verbatim
- `aarch64/ABI.md` — the publication template (structure, what's frozen, enforcement notes)
- `sys/src/libc/9syscall/amd64/sys.h` + the amd64 syscall stubs — the authoritative syscall number mapping + entry convention D-02a pins
- `sys/src/9k/k10/l64syscall.s` — 9k's entry-state precedent the researcher verifies against

### Prior-phase precedents (Harvey repo, milestones)
- `.planning/milestones/v1.0-phases/04-r9-userspace-exec-harvey-r9-abi/04-CONTEXT.md` — THE exec/ABI template: D-15 donor-verbatim ABI, TrapFrame≡Ureg publication gate, measured-set dispatch (D-14), staged-init
- `.planning/milestones/v1.0-phases/05-shell-boot-one-command-integration/05-CONTEXT.md` — the boot(8)→init→rc chain + one-command compose + expect smoke precedents
- `.planning/milestones/v2.0-phases/08-*/08-CONTEXT.md` (net phase) — the /net gate scripts D-07 re-keys
- `build/aarch64/smoke-r9.expect`, `build/aarch64/smokesleep.c` — the expect-smoke + purpose-built-marker disciplines
- `build/aarch64/tiers/tier0-libs.txt`, `tier1-cmds.txt` — the tier allowlists XEXEC-04 re-keys to `objtype=amd64`
- `build/mm/build-9k10t1.sh`, `boot-9k10t1-sweep.sh` — the k10t1 donor venue

### r9 repo (sibling, `../r9`)
- `../r9/x86_64/src/platform/q35/mb2.rs` — the tag walker D-03's module tag extends
- `../r9/x86_64/lib/grub.cfg` + `../r9/flake.nix` — the ISO staging + boot-iso FOD derivation D-05 parameterizes
- `../r9/port/src/{devroot,devproc,devcons}.rs`, `../r9/port/src/exec*` — the arch-neutral exec/devroot substrate
- `../r9/xtask/src/main.rs` — boot-test profiles + ureg-check D-02b extends
- `sys/src/9k/boot/boot.c`, `rc/bin/termrc` — boot(8)/rc chain semantics references

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Phase-13's whole platform: kernel-x64 package, reproducible boot-iso FOD, mb2 tag walker, netgate trigger seam (recorded for exactly this phase's exec swap), CR-01-fixed trap path
- r9 `port/` devroot/devproc/devcons/pgrp/fdgrp — the exec + process substrate, arch-neutral by construction
- aarch64's exec/ABI/boot-chain machinery (v1.0 Phases 4–5): ABI.md publication flow, ureg-check, staged-init gates, smoke expects — re-key, don't reinvent
- Tier machinery: allowlists as committed data, artifact+execution markers, k10t1 venue scripts — re-key to amd64

### Established Patterns
- Cross-repo one-way ABI contract: offsets frozen in ABI.md + const asserts + ureg-check + negative control, live on every gate (aarch64 Phase-4 shape, now on amd64)
- Gates split by artifact (D-06): kernel-level r9-side, composed-system Harvey-side
- push-before-pin: r9 pushes before any flake.lock advance; pin advance is its OWN commit + fresh CI
- Staged, execution-gated ordering: marker exec → boot chain, every commit bootable

### Integration Points
- `../r9/x86_64/src/` — user-mode substrate, syscall entry, module-tag platform data
- `../r9/flake.nix` — boot-iso parameterization `{kernel, modules}`
- Harvey `flake.nix` — `bootfs-x64` build + `boot-r9` app consuming r9's builder
- `build/x86_64/` (new sibling to `build/aarch64/`) — expect smokes + re-keyed tier/net gates

</code_context>

<specifics>
## Specific Ideas

- The acceptance story, end-to-end: ureg-check amd64 + const asserts + negative control green from the first commit (XEXEC-01) → purpose-built static marker 6.out execs on r9 x86_64 printing its serial marker through the real syscall path (XEXEC-02 staged leg 1) → boot(8)→init→rc reaches `% `, expect smoke green twice (XEXEC-03) → tier-0/1 binaries from the same allowlists execute on r9-x86_64 AND on k10t1 — identical 6.outs, the donor venue doubling as the ABI witness (XEXEC-04) → loopback ping/udp/tcp echo + static/DHCP ipconfig + composed net smoke green as committed in-guest gates (XEXEC-05) → `nix run .#boot-r9` is the one command.
- Watch-out: the exec-critical path now includes miniz-inflate (bootfs.paq) — the aarch64 heisenbug history (contained, root cause once deferred) says keep the host pins tight and exec_bootstrap IRQ-masked discipline carried.
- Watch-out: 9k-verbatim means r9 inherits 9k's historical slot arithmetic (incl. the error-code slot variance class CR-01 just fixed in r9's own path) — the researcher diffs r9's push order against `amd64/include/ureg.h` BEFORE any ring-3 code runs, not after.
- Watch-out: DHCP ipconfig under qemu slirp (XEXEC-05) — slirp's DHCP behavior is the known-working aarch64 shape; carry the venue honesty rule (slirp forgives L2 bugs; it is not a correctness target).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope (no new capabilities proposed; the bootfs-medium question explicitly routed around virtio-blk/9p as Future Requirements)

</deferred>

---

*Phase: 14-Harvey amd64 Userspace on r9*
*Context gathered: 2026-11-08*
