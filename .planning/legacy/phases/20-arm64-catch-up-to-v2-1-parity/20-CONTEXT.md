# Phase 20: arm64 catch-up to v2.1 parity — Context

**Gathered:** 2026-11-17
**Source:** owner answers during /gsd-plan-phase 20 (no discuss-phase was run; these two decisions were put to the owner after research and are locked)

<domain>
Bring the r9 aarch64 kernel (sibling repo `../r9`, crate dir `aarch64/`, Rust name stays aarch64) and Harvey arm64 userspace (objtype `arm64`) to parity with the v2.1 x86_64 work. See 20-RESEARCH.md for the measured start state and drift inventory.
</domain>

<decisions>

### Pass bar

- **D-01: The phase gates on HVF, not TCG.** The arm64 gates (boot-r9-smoke, boot-r9-net-smoke, boot-r9-tier1) must be reliably green under HVF on the darwin-arm64 host. The intermittent `rc: pushword but no argv!` failure measured in research (4 of 13 HVF boots, 0 of 4 TCG boots; fires in a freshly forked child) must be root-caused and fixed in this phase — it is not deferrable and not time-boxed. TCG results are still recorded as a control leg.
- **D-02: "Reliably green" needs a stated repeat count.** Because the defect fired in roughly 30% of HVF boots, a single green run proves nothing. Each HVF gate's acceptance must name an N of consecutive green runs large enough to distinguish a fix from luck, and the plan must justify the N it picks.

### Scope

- **D-03: Full kernel parity now.** In addition to the must tier (the complete note fix including the `push_note` ready-a-Blocked-target half, the per-proc user hook registration, the rename test fix in `port/tests/devenv_test.rs`, donor-venue re-provision, CI coverage that compiles the aarch64 `dev_exec` code), this phase includes the aarch64 halves of rfork, mount-write, exit and exec, AND the cpu-port items research recommended deferring: the `#!` exec pre-pass, WSTAT/FWSTAT/REMOVE dispatch arms, devcap, devssl, and DTB bootconf.
- **D-04 [informational]: Research rated the cpu-port items as phase-sized.** (Acted on during planning and superseded by D-07: the planner returned the split recommendation and the owner chose one phase.) The owner chose to include them anyway. If the planner finds the phase exceeds the budget for full-fidelity plans, it must return a phase-split recommendation rather than thin the plans or silently drop items.

- **D-07: One phase, no split.** The planner returned a phase-split recommendation (about 12 plans / 32 tasks; three sub-phases proposed). The owner chose to proceed as a single Phase 20, accepting the planner's warning about plan quality at this size. Do not return another split recommendation for size alone. Preserve the planner's proposed ordering inside the one phase: the HVF-green-at-the-pin work (incl. the fork-child defect) first, syscall-path parity second, cpu-port arms last — a new arm64 gate must not be built or have its N-green series run before the fork-child defect is fixed.
- **D-08: The three unnamed items are in scope.** virtio capability hardening (`check_cap_extents`), the aarch64 host-test battery, and the arm64 churn gate (which pins the exit-leak fix). The churn gate goes with the syscall-path work.
- **D-09: Unknowns for the cpu-port arms are resolved inside the phase, not assumed.** Research only inventoried these items. The plans that build them must begin with a measuring/investigation task (writable venue: ramfs in arm64 tier-1; an arm64 consumer and fixture gate for devcap/devssl/`#!`/bootconf; whether the aarch64 crate compiles as a host test binary; whether the lone-member brk defect exists on aarch64) and must carry a checkpoint where the finding could change the design.

### Carried in

- **D-05: The objtype is `arm64`** (quick 261117-bz5). The harness dir keeps the name `build/aarch64/`; r9's crate keeps `aarch64/`. Do not rename either.
- **D-06: The boot-chain trace gate's `measured set lacks fversion` failure** (one run, cause unconfirmed) must be diagnosed in this phase, since the `bind /boot/ourinit /arm64/init` deviation it exercises was introduced by the rename and has no green run yet.

### Claude's Discretion

- Plan count, wave structure and ordering, provided the HVF fork-child defect is investigated early enough that its size is known before the parity work is declared done.
- The devcap profile route: a separate arm64 cpu profile (new r9 package + new Harvey app; the three existing gates stay on an unchanged kernel) versus adding `"cap"` to virtshell (changes the kernel under all three gates). The planner leans to the separate profile; the owner has not ruled. Treat it as a `checkpoint:decision` before the task that implements it.
- The D-02 repeat count N: derive it from per-fork failure rate and have the harness re-measure the pre-fix rate before fixing N (planner estimate from 13 boots: about 36 smoke boots or 5 tier-1 sweeps for 95%).
- Requirement IDs: the planner proposed ARM-01..ARM-11; research proposes ARM-01..06; extend as needed for the D-03 scope.
- Whether r9 edits touch `port/src/` (which re-stales the x86_64 boot-iso FOD that cannot be checked on this host) — avoid where an aarch64-side change suffices, and flag every case where it cannot be avoided.

</decisions>

<deferred>
None recorded.
</deferred>
