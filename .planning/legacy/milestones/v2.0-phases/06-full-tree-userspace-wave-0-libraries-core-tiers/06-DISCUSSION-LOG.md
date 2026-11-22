# Phase 6: Full-Tree Userspace — Wave 0 + Libraries/Core Tiers - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-04
**Phase:** 6-Full-Tree Userspace — Wave 0 + Libraries/Core Tiers
**Areas discussed:** Wave-0 scope + verdict standards, Tier-0 lib set + execution markers, Tier-1 core-cmds membership + gate shape, Allowlist machinery + marker/CI shape

---

## Wave-0 scope + verdict standards

### Q1: Does the aarch64 FPdbleword bug join Wave 0?

| Option | Description | Selected |
|--------|-------------|----------|
| Fix in Wave 0 | One-line header fix (uint lo/hi, matching amd64/include/u.h:49-56) + rebuild libc/fmt + float-print marker in the tier-1 execution gate | ✓ |
| Defer — gates avoid floats | Leave it broken; tier gates avoid float-printing assertions | |
| Fix, but sequenced last in Wave 0 | Own atomic commit after the gethunk pair, donor-kernel float repro gate | |

**User's choice:** Fix in Wave 0
**Notes:** Bug source: `.planning/codebase/CONCERNS.md` (2026-11-03 audit, HIGH) — `aarch64/include/u.h:54-61` 64-bit halves make frexp/NaN/Inf read/write outside the 8-byte double; all `%e/%f/%g` output silently garbage in every aarch64 binary.

### Q2: Fold the /usr/bin/grep absolute-path fix into Wave 0 harness hygiene?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — Wave-0 harness hygiene | Resolve grep once per script (command -v fallback); one commit | ✓ |
| No — separate quick task | Keep Wave 0 laser-focused on named landmines | |
| No — new tier machinery avoids those scripts | Leave v1.0 scripts untouched | |

**User's choice:** Yes — Wave-0 harness hygiene
**Notes:** Six build/aarch64 scripts hardcode /usr/bin/grep; absent on NixOS-class linux hosts; the native-linux leg is a supported platform.

### Q3: What does the USPACE-10 pool-panic re-test verdict assert?

| Option | Description | Selected |
|--------|-------------|----------|
| Banner + no pool panic | Rebuild 9k10cpu with pswt-fixed 6c, boot Phase-1 minimega harness, assert banner + no 'bad magic' | |
| Banner + embedded-root reach | Additionally assert ip/ipconfig present in root image | |
| Record-only, no assertions | Minimal: rebuild + boot attempt, record whatever happens | |

**User's choice:** (free-text redirect) "I wanted to use minimega before I realized that it was architecture bound. Since I will be developing on darwin-aarch64, let's defer the minimega stuff for another time when I have an amd64 development host available."
**Notes:** This redirect removed the Phase-1 build/mm venue from Phase 6's toolbox entirely and forced the venue follow-up (Q4). Recorded as D-03 (minimega deferred wholesale) in CONTEXT.md.

### Q4: How does the USPACE-10 re-test get its recorded verdict without minimega?

| Option | Description | Selected |
|--------|-------------|----------|
| Re-test in x86_64 CI leg | GitHub ubuntu-latest exposes /dev/kvm; build 9k10cpu + boot qemu-system-x86_64 KVM, serial verdict | ✓ |
| Local plain-qemu TCG re-test | qemu-system-x86_64 TCG on darwin — slow, new mini-harness | |
| Defer USPACE-10 out of Phase 6 | Back to milestone carry-over; amend SC#1 | |

**User's choice:** Re-test in x86_64 CI leg
**Notes:** Verdict asserts banner + no "bad magic" pool panic and is recorded either way — even FAIL closes USPACE-10 with information. Recorded as D-04.

---

## Tier-0 lib set + execution markers

### Q1: How should tier-0's 22 libraries be sequenced internally?

| Option | Description | Selected |
|--------|-------------|----------|
| libdraw family first | Unblocks Phase 10/12 GUI critical path earliest | |
| Tier-1 consumer closure first | Sort by what tier-1 links; GUI libs last | |
| Single wave, planner sorts | One wave; planner's dependency sort, no milestone weighting | ✓ |

**User's choice:** Single wave, planner sorts (overrode the recommendation)

### Q2: What discharges a library's execution marker?

| Option | Description | Selected |
|--------|-------------|----------|
| Artifact + consumer-carried execution | Fresh .a per lib + tier-1 consumers carry execution + in-tree lib tests on donor where present | ✓ |
| Per-lib smoke binaries | 22 purpose-built 7c smoke programs | |
| Compile-only for libs | USPACE-05 literal; execution markers apply to executable tiers only | |

**User's choice:** Artifact + consumer-carried execution

### Q3: How is ar archive staleness (02-10) handled during the 22-archive sweep?

| Option | Description | Selected |
|--------|-------------|----------|
| Fresh-archive per lib in the sweep | Delete target .a before repopulating; deterministic | ✓ |
| Document nuke discipline only | Executor discipline (02-10's documented suggestion) | |
| Fix ar staleness generally | Membership-change detection — its own hygiene project | |

**User's choice:** Fresh-archive per lib in the sweep

### Q4: Scope of the tier-0 sweep for edge-case libs (libdynld/libaml/libscribble)?

| Option | Description | Selected |
|--------|-------------|----------|
| All 22 attempt; exclude only documented | No-consumer libs excluded by name with committed comment | ✓ |
| Pre-exclude libdynld/libaml/libscribble | Sweep 19; pre-judges without evidence | |
| Researcher proposes at plan time | No exclusions decided in discussion | |

**User's choice:** All 22 attempt; exclude only documented

---

## Tier-1 core-cmds membership + gate shape

### Q1: What defines tier-1 'coreutils' membership?

| Option | Description | Selected |
|--------|-------------|----------|
| Subset + chain + researcher-proposed classics | v1.0 16-subset + boot chain + guest build chain + coreutils extension from donor /bin baseline, locked in PLAN; float canary mandatory | ✓ |
| Minimal: subset + chain only | Tightest USPACE-06 reading; rest waits for Phase 10 | |
| Goal-backward from later phases | Membership = what Phase-10/11/12 paths execute | |

**User's choice:** Subset + chain + researcher-proposed classics
**Notes:** Float-printing canary (awk or bc) is mandatory — it gates the Wave-0 FPdbleword fix (D-01).

### Q2: How does the tier-1 donor-execution gate drive dozens of binaries?

| Option | Description | Selected |
|--------|-------------|----------|
| In-guest scripted rc sweep + serial markers | Committed rc script, per-binary OK/FAIL markers, host asserts set (05-03 sourced-sweep pattern) | ✓ |
| expect-typed per binary | v1.0 smoke style; timing-fragile, doesn't scale | |
| Per-binary one-shot boots | Strongest isolation, slowest | |

**User's choice:** In-guest scripted rc sweep + serial markers

### Q3: Does tier-1 include a guest self-host canary?

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — guest compiles + runs hello | aarch64 7a/7c/7l inside donor guest compile known source; output runs and prints | ✓ |
| Invocation smoke only | 7c -V / mk -n / yacc on small grammar | |
| Later phase | Self-host proof deferred | |

**User's choice:** Yes — guest compiles + runs hello

### Q4: Where does the r9-side tier-1 execution gate live?

| Option | Description | Selected |
|--------|-------------|----------|
| Harvey-side composed smoke extends | Tier-1 sweep bakes into boot-r9 image; boot-r9-smoke/sibling app asserts; r9 boot-test profiles unchanged | ✓ |
| New r9 boot-test profile | cargo xtask profile; splits userland evidence across repos | |
| Both venues | Redundant evidence, double maintenance | |

**User's choice:** Harvey-side composed smoke extends
**Notes:** Upholds v1.0 D-12 ("r9-side gates stay r9-side").

---

## Allowlist machinery + marker/CI shape

### Q1: How is the append-only allowlist physically represented?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-tier data files | build/aarch64/tiers/*.txt; mk filters aarch64 leg only; amd64 full-tree unchanged | ✓ |
| mk includes per tier | mkfile.aarch64.tierN DIRS fragments | |
| Per-dir mkfile sentinels | AARCH64_TIER=1 per dir's mkfile | |

**User's choice:** Per-tier data files
**Notes:** Rated one-way (D-13) — the default dual-arch build keys off these files; Phase 10 appends; coverage report + CI diff assert against them.

### Q2: How are artifact+execution markers recorded?

| Option | Description | Selected |
|--------|-------------|----------|
| Per-tier append-only ledgers | build/aarch64/markers/*.ledger machine-readable lines; EVIDENCE.md stays human summary | ✓ |
| Per-dir marker files | Maximal per-dir blame; hundreds of tiny files by Phase 10 | |
| Single REPORT.md | Fragile machine-diffing, append conflicts | |

**User's choice:** Per-tier append-only ledgers

### Q3: USPACE-02 dir-coverage report form and cadence?

| Option | Description | Selected |
|--------|-------------|----------|
| Committed + CI-regenerated diff | Red on drift; same enforcement shape as USPACE-03 | ✓ |
| Committed once, no CI | Proves reach at phase exit only | |
| CI-only artifact | Proof not in tree for clean-checkout verification | |

**User's choice:** Committed + CI-regenerated diff

### Q4: Which Phase-6 gates run in CI vs committed-evidence-only?

| Option | Description | Selected |
|--------|-------------|----------|
| Full pipeline in CI | v1.0 D-13 precedent extends; everything in CI unless measured over-budget; darwin documented-manual | ✓ |
| Fast gates in CI, boot gates local | Faster CI; clean-checkout claim rests on local runs | |
| Planner proposes by measured runtime | Defer the split to execution | |

**User's choice:** Full pipeline in CI
**Notes:** Includes the new x86_64 9k pool-panic leg (D-04). Plan-time may split fast/slow jobs but everything runs in CI unless measured over-budget.

---

## the agent's Discretion

- Exact tier-0 lib ordering within the single wave (dependency sort); coreutils extension list proposed from donor /bin (locked in PLAN)
- Physical layout/naming inside build/aarch64/tiers/ and markers/; the mk-layer filter mechanism
- Ledger line format details (must stay machine-readable, diff-stable)
- CI workflow topology (extend r9-shell.yml vs sibling; fast/slow job split) — everything-in-CI constraint holds
- Sweep script marker vocabulary + per-binary fixed inputs; self-host canary source program
- x86_64 pool-panic CI leg mechanics — assertion content locked by D-04
- Wave-0 task ordering among fixes — each its own atomic commit

## Deferred Ideas

- minimega harness work — deferred until the user has an amd64 development host (D-03)
- Tiers 2–4 (services, GUI, long tail) + bootfs payload — Phase 10
- fossil/venti runtime verification — no block device in r9 this milestone
- gs (ghostscript) tier tension (REQUIREMENTS includes for Phase 10; PITFALLS recommends exclude-and-document) — resolves at Phase 10 planning
- hswal stride bugs beyond commonllp64 — logged, fix when a host tool needs non-_MAGIC formats
- General ar/mk staleness mechanism — D-07 covers swept dirs; general fix stays hygiene-phase (02-10)
- `nix run .#build` exit-status plumbing (02-12) — markers remain the truth
- virtio-9p, SMP/PSCI, RTC, virtio-blk — future milestones
- raspi3b-on-qemu-11.1.2 upstream defect — pre-existing, human-accepted 2026-11-01
