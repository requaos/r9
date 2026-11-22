# Phase 6: Full-Tree Userspace — Wave 0 + Libraries/Core Tiers - Context

**Gathered:** 2026-11-04
**Status:** Ready for planning

<domain>
## Phase Boundary

The mk build factory sweeps the full tree for aarch64 without structural aborts or silent hangs. Wave 0 closes the known landmines with regression proof (gethunk ×2 compile-hangs, jpg/mahjongg/4a structural blockers, the FPdbleword float fix, harness hygiene, and the 9k-amd64 pool-panic re-test verdict); the cross-compiled-generator audit and dir-coverage report make the sweep provably complete; and the tiered append-only allowlist machinery — whose only truth is artifact+execution markers — carries tier-0 (22 libraries) and tier-1 (shell/coreutils + the guest build chain) green on the 9front donor kernel first, then on r9.

Work happens in this repo (Harvey). r9 is touched only as an execution venue — its syscall dispatch grows to the measured tier-1 footprint, but all tier gates, allowlists, markers, and CI wiring live Harvey-side. Addresses USPACE-01, USPACE-02, USPACE-03, USPACE-04, USPACE-05, USPACE-06, USPACE-10.

NOT in this phase: tiers 2–4 (services, GUI, long tail — Phase 10), any networking/auth/desktop work (Phases 7–9, 11, 12), ape (excluded by milestone decision), gs (excluded from tiers), other-arch toolchain letters as aarch64 *targets* (4a–9l etc. are host tools via BUILDTOOLCHAIN only), minimega harness work (deferred until an amd64 dev host exists), fossil/venti runtime (no block device in r9 — Phase 10 compile-gates them).

</domain>

<decisions>
## Implementation Decisions

### Wave-0 scope + verdict standards
- **D-01:** The **FPdbleword fix joins Wave 0** (beyond the locked USPACE-01/02 landmines). `aarch64/include/u.h:54-61` changes `ulong lo/hi` → `uint lo/hi` (matching the working `amd64/include/u.h:49-56`), libc + libfmt rebuild, and a **float-print marker lands in the tier-1 execution gate** so `%e/%f/%g` correctness is execution-asserted, not assumed. Rationale: tier-1 float commands (awk/bc/units class) would otherwise execute "green" while emitting garbage — exactly the compile-green≠run-correct class this phase exists to kill. — **Reversibility:** costly — every freshly built aarch64 binary links libc/fmt; the fix changes float-introspection behavior tree-wide, and the tier-1 gate's float marker keys off the corrected output.
- **D-02:** The **`/usr/bin/grep` absolute-path fix joins Wave 0 as harness hygiene** — resolve a host grep once per script (`command -v` fallback chain) across the six `build/aarch64/` scripts that hardcode it. One commit. Rationale: tier gates extend these same scripts, and the native-linux leg is a supported platform where the absolute path breaks (NixOS-class hosts).
- **D-03:** **minimega is deferred wholesale** until the user has an amd64 development host — it is x86/KVM-bound and the dev fleet is darwin-aarch64. This removes the Phase-1 `build/mm/` venue from Phase 6's toolbox. — **Reversibility:** reversible — a venue decision, not a code contract; revisits when an amd64 host exists.
- **D-04:** **USPACE-10's pool-panic re-test runs in a new x86_64 CI leg** (GitHub `ubuntu-latest` exposes `/dev/kvm`): build 9k10cpu with the pswt-fixed 6c, boot under qemu-system-x86_64 KVM, serial-captured verdict asserting banner + no "bad magic" pool panic — **recorded either way** (even a FAIL verdict closes USPACE-10 with information). Keeps USPACE-10 in Phase 6 without minimega or a local amd64 host.

### Tier-0 libraries (22 outstanding)
- **D-05:** **Single wave, planner sorts** — all 22 libs are one wave; internal order is the planner's dependency sort with no milestone-priority weighting.
- **D-06:** **Library execution marker = artifact + consumer-carried execution.** Per-lib artifact marker (fresh `.a` built from portable C via the mksyslib fallback) + execution proven by tier-1 consumer binaries that link them (the consumer's donor-kernel gate is the execution evidence) + in-tree lib test binaries run on the donor where they exist (libsec `*test.c` class). No new per-lib smoke-binary machinery.
- **D-07:** **Fresh-archive per lib in the sweep** — the tier sweep's lib build deletes the target `.a` before repopulating. Deterministic kill of the 02-10 staleness class (stale `alphaobj.6` duplicate-symbol'd Phase 2); no discipline required of future edits.
- **D-08:** **All 22 attempted; exclusion only documented** — a lib with no aarch64 consumer (researcher verifies at plan time — libdynld/libaml/libscribble are the edge cases) gets excluded by name with a committed comment, never silently dropped (the notorious-dirs handling pattern).

### Tier-1 core commands + guest build chain
- **D-09:** **Membership = v1.0 16-subset + boot chain + guest build chain + researcher-proposed coreutils classics.** The guest build chain is 7a/7c/7l/cc/cpp/mk/yacc/lex as aarch64 binaries (USPACE-06 literal); the coreutils extension (awk/sort/tr/uniq/tail/dd/tar/chmod/tee/xargs/basename/dirname/kill/sleep class) is proposed by the researcher from the 9front donor's own `/bin` baseline and locked in the PLAN. **At least one float-printing canary (awk or bc) is mandatory** — it gates D-01's FPdbleword fix.
- **D-10:** **Gate mechanics = in-guest scripted rc sweep + serial markers.** A committed in-guest rc script runs each tier-1 binary with fixed inputs and emits per-binary OK/FAIL markers over serial; the host asserts the marker set. This is the 05-03 sourced-sweep pattern (sourced, not typed) — scales to dozens of binaries, preemption/TCG-timing tolerant. expect-typed per-binary interaction does not scale; per-binary one-shot boots are too slow.
- **D-11:** **The guest self-host canary is in tier-1** — inside the donor guest, our aarch64 7a/7c/7l compile a known source (hello-class) and the produced binary runs and prints. The strongest reading of "self-hosted build chain executes": proves codegen end-to-end in-guest, not merely that compiler binaries start.
- **D-12:** **The r9-side tier-1 gate is Harvey's composed smoke extended** — the tier-1 rc sweep bakes into the boot-r9 image; `boot-r9-smoke` (or a tier-specific sibling flake app) drives it and asserts markers. r9's `cargo xtask boot-test` profiles (virt/virtexec/virtmulti/virtnotes/virtpanic) stay unchanged kernel-focused regression gates — v1.0 D-12's "r9-side gates stay r9-side" holds.

### Allowlist machinery + marker/CI shape
- **D-13:** **The append-only allowlist is per-tier committed data files** (`build/aarch64/tiers/tier0-libs.txt`, `tier1-cmds.txt`) — plain data, one dir per line with comments. The mk layer filters the **aarch64 leg's** DIRS through them; the amd64 leg sweeps full-tree as today. Append-only = append a line; diffs are reviewable; matches the project's single-source-as-data convention (r9 TOML precedent). — **Reversibility:** one-way — the default dual-arch build's behavior keys off these files; every later tier (Phase 10) appends to them, and the coverage report + CI diff assert against them.
- **D-14:** **Markers are per-tier append-only ledgers** (`build/aarch64/markers/tier0.ledger`, `tier1.ledger`) — machine-readable lines (dir, artifact sha256, exec marker, evidence path, commit, date) so scripts/CI can diff them. The phase EVIDENCE.md stays the human summary. Scales to the 250-dir long tail without hundreds of tiny files.
- **D-15:** **The USPACE-02 dir-coverage report is committed AND CI-regenerated-and-diffed** (red on drift) — "DIRS reaches every dir" rots as dirs are added/moved; the CI diff keeps it honest. Same enforcement shape as USPACE-03's locked generator-audit red.
- **D-16:** **Full pipeline in CI** (linux legs, per v1.0 D-13 precedent): Wave-0 synthetic repro gates, generator audit, coverage diff, tier-0/1 donor+r9 execution gates, and the new x86_64 9k pool-panic leg (D-04) all run in CI; darwin stays documented-manual. Plan-time may split fast host-side checks from slow boot legs into separate jobs, but everything runs in CI unless measured over-budget.

### Carried forward (locked in roadmap/prior phases — do not re-litigate)
- Tier structure: tier-0 = libs, tier-1 = core cmds + build chain; tiers 2–4 (services/GUI/long tail) = Phase 10. ape excluded by decision; gs excluded from tiers; other-arch toolchain letters excluded from the aarch64 target sweep.
- Donor-kernel execution FIRST, then r9 via measure-first traces (USPACE-04; v1.0 D-14/05-03 discipline — implement exactly the measured syscall set, loud-unimplemented default stays).
- mk exit status is never trusted (namespace build always exits 0 — 02-12); artifact+execution markers are the only gates.
- Allowlist growth is append-only, never a flag-flip; a dir enters the default build only after passing its per-dir gate.
- gethunk ×2 fixes use the proven `7l/unixcompat.c` accumulate-when-contiguous pattern; each fix is its own commit with a synthetic large-macro/large-input repro as the gate (Pitfall 4). The hswal item stays logged, not speculatively fixed.
- Generator discipline: generators always build with the host toolchain (pcc/9c into the per-OS host bin dir), never `$CC`; CI reds on `.7`-magic generator artifacts (USPACE-03).
- Bake-into-bootfs for gates (deterministic), ufs-over-tcp for iteration (Phase 2 bake-vs-ufs resolution).
- Evidence standard: marker assertions over serial, timing-tolerant (assert presence, never interleaving order); canonical evidence consolidated in `.planning/phases/06-*/` with the r9 commit SHA recorded where r9 is involved.
- amd64 green is sacred: shared-frontend fixes (cc/macbody, 6l/obj.c) must keep the 6c/6l amd64 toolchain and 9k kernel build green; additive/objtype-gated edits to shared sources.
- NMOUNT=54 divergence ledger lives and grows — divergent syscalls wall loudly on the donor (Phase 2 D-09).
- `aarch64` naming everywhere; Plan 9 C style (`sys/man/6/style`) in Harvey; r9 follows its rustfmt.toml — style rules don't cross the repo boundary.
- qemu shape for r9/donor boots: `-M virt,gic-version=2` (r9) / donor-kernel drift accommodations in `build/aarch64/README.md` (gic-version=3, highmem-ecam=off, DTB pre-placement, GICR wake poke, console=0, virtio-net-pci-non-transitional); TCG on darwin; `-m ≤ 1G`.
- r9 commits land on `main` of `requaos/r9`; r9-side changes (measured-set dispatch growth) follow the push-before-pin discipline; Harvey `flake.lock` r9 pin advances as its OWN commit with a fresh hosted CI run, never bundled.

### the agent's Discretion
- Exact tier-0 lib ordering within the single wave (dependency sort), and the exact coreutils extension list proposed from the donor `/bin` baseline (D-09) — locked in PLAN, user reviews there.
- Physical layout/naming inside `build/aarch64/tiers/` and `build/aarch64/markers/` and the mk-layer filtering mechanism that consumes the allowlist (mkfile.proto vs cmd/mkfile hook).
- Ledger line format details (field order, hash algorithm) — must stay machine-readable and diff-stable.
- CI workflow topology (extend `r9-shell.yml` vs sibling workflow; job split fast-vs-slow per D-16) — subject to the everything-in-CI constraint.
- The in-guest sweep script's marker vocabulary and per-binary fixed inputs (D-10), and the self-host canary's source program (D-11).
- The x86_64 pool-panic CI leg's mechanics (job placement, serial capture/assert recipe) — the assertion content (banner + no bad-magic panic, verdict recorded either way) is locked by D-04.
- Wave-0 task ordering among the fixes (gethunk pair, FPdbleword, grep hygiene, structural blockers) — each remains its own atomic commit.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 6 — goal, success criteria 1–5, requirement mapping
- `.planning/REQUIREMENTS.md` — USPACE-01..06 + USPACE-10 full text; Out of Scope table (ape, gs-class exclusions); traceability
- `.planning/PROJECT.md` — Key Decisions, Known upstream breakage list (jpg DIRS abort, 4a lex.c, mahjongg cpp), Constraints
- `.planning/STATE.md` — Accumulated Context (all v1.0 phase decisions), Deferred Items inventory (02-6 gethunk ×2, 02-8 pool-panic re-test, 02-10 ar staleness, 02-12 mk exit status)

### v2.0 milestone research (the phase's bug-class + tier-design source)
- `.planning/research/SUMMARY.md` — research flags; "Phase 1, 5 (USERSPACE): what they need is Wave-0 audits scheduled as real tasks, not research"
- `.planning/research/PITFALLS.md` — Pitfall 1 (whack-a-mole + notorious-directories table + tier definitions), Pitfall 2 (generator host-toolchain rule), Pitfall 3 (aarch64 bug-class catalog: unsigned char, varargs width, LP64 fossils, struct punning), Pitfall 4 (gethunk ×2 preemptive fixes), Pitfall 5 (ape exclusion); the trade-off table rows on donor-first execution and measure-first
- `.planning/research/FEATURES.md` — full-userspace table stakes; verification checklist ("mk exit status is ALWAYS 0 — the gate is artifact existence + per-tier donor-kernel execution markers")
- `.planning/research/STACK.md` + `.planning/research/ARCHITECTURE.md` — component inventory; no new userspace imports decision

### Codebase audit (Wave-0 additions came from here)
- `.planning/codebase/CONCERNS.md` — the FPdbleword bug entry (D-01), `/usr/bin/grep` hardcoding entry (D-02), ar/mk hygiene items; the fragility notes on `build/aarch64/out/` last-writer-wins ownership

### Prior-phase decisions and debts (Harvey repo — archived v1.0)
- `.planning/milestones/v1.0-phases/02-aarch64-toolchain-libc-9front-donor/deferred-items.md` — the gethunk ×2 item text (known bug, known fix pattern), the pool-panic re-test item (USPACE-10's origin), 02-10 ar staleness, 02-12 exit status, the resolved nested-backquote mk fix (261104-b4p)
- `.planning/milestones/v1.0-phases/02-aarch64-toolchain-libc-9front-donor/02-CONTEXT.md` — D-06..D-09 (execution-gate standard, donor harness, NMOUNT ledger), D-13 (shared-frontend fidelity), D-15/16/17 (ABI freeze)
- `.planning/milestones/v1.0-phases/04-r9-userspace-exec-harvey-r9-abi/04-CONTEXT.md` — D-14 (measure-first syscall discipline), proof standard
- `.planning/milestones/v1.0-phases/05-shell-boot-one-command-integration/05-CONTEXT.md` — D-09 (scripted sweep), D-10/D-11/D-12 (flake composition, r9 kernel package, r9-side gates stay r9-side), D-13 (CI = linux leg, full pipeline; darwin documented-manual), the sourced-not-typed 05-03 trace-sweep finding

### The landmine sites and fix patterns (in-tree)
- `sys/src/cmd/cc/macbody.c` — gethunk site 1 (reset-not-accumulate; fix pattern below)
- `sys/src/cmd/6l/obj.c` — gethunk site 2
- `sys/src/cmd/7l/unixcompat.c` — the PROVEN accumulate-when-contiguous fix pattern (Phase 2, donor-kernel link incident)
- `sys/src/cmd/cc/pswt.c` — the pswt.c LP64 switch-sort fix (7efff3aa3) whose 6c re-test is USPACE-10
- `sys/src/cmd/jpg/mkfile` — generator shape (`$cputype.rgbycc` built via `objtype=$cputype mk`, host-run); the DIRS-abort lineage; re-verify host-toolchain correctness for aarch64 (Pitfall 2)
- `aarch64/include/u.h:54-61` + `amd64/include/u.h:49-56` — the FPdbleword defect and its correct reference (D-01)
- `sys/src/libc/port/frexp.c`, `sys/src/libc/port/nan.c`, `sys/src/libc/fmt/fltfmt.c` — the float-introspection consumers the fix unbreaks

### Harness + supply chain (in-tree)
- `build/aarch64/README.md` — donor-kernel harness (the tier-0/1 execution venue + measure-first trace rig), qemu-11.1.2 drift accommodations, bake-vs-ufs decision
- `build/aarch64/{subset-build.sh,stage-build.sh,mkimage.sh,subset-exec.sh}` — the existing subset build/bake/exec flows the tier machinery extends (and the D-02 grep fix targets)
- `build/aarch64/smoke-r9.expect` + `nix/{boot-r9.sh,smoke-r9.sh}` — the composed smoke D-12 extends for tier-1's r9 gate
- `flake.nix` — mkScriptApp pattern; the r9 input contract (`r9.packages.<system>.kernel-virt`)
- `.github/workflows/r9-shell.yml` — the v1.0 CI shape Phase 6 extends (D-16); supply-chain pin rule (40-char SHAs)
- `build/REPRODUCIBILITY.md` — the KERNDATE/process-level reproducibility policy the CI legs honor

### qemu / CI venue facts (carryover — re-verify only if versions change)
- GitHub `ubuntu-latest` exposes `/dev/kvm` (D-04's x86_64 leg premise — verify at workflow-authoring time)
- qemu 11.1.2 drift facts in `build/aarch64/README.md`; HVF rejects GICv2 (darwin stays TCG for r9 boots)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `build/aarch64/` harness — donor-kernel build (`donor-vm-build.sh`), bake (`mkimage.sh` with the real-init rodata guard), exec (`subset-exec.sh`), and the 05-03 measure-first trace rig (`trace-syscalls.py`) — the tier gates extend all of these
- The v1.0 SUBSET-OK marker pattern (per-command markers asserted host-side) — D-10's per-tier sweep generalizes it
- `build/mm/` Phase-1 serial capture/assert pattern — superseded as a venue (D-03 minimega deferral) but its banner-grep assertion shape transfers to the D-04 CI leg
- Phase-1 `checkall` report-all mk target — the USPACE-02 coverage sweep's natural foundation
- r9's frozen ABI enforcement (const asserts + `cargo xtask ureg-check`) — untouched; r9's dispatch growth this phase follows the measured-set rule
- Vendored 9front toolchain (7a/7c/7l pin `350266104f`) — both the tier-1 build-chain deliverable AND the sweep's compiler
- plan9port host toolchain + `.build/hostbin-<os>-aarch64/` — where Wave-0's generator-correct host tools come from

### Established Patterns
- Execution-gated ordering (riscv64 vaporware rule): no dir/scaffolding lands before a running binary exercises it — tier allowlist entries are execution-gated, never compile-only for executables
- Measure-first syscall discipline: donor trace precedes r9 implementation; loud-unimplemented default stays
- Atomic class-sweeps: bug-class fixes land one commit per class (v1.0 convention), synthetic repro gates for the gethunk pair
- Single-source-as-data (r9 TOML precedent) — D-13's allowlist data files follow it Harvey-side
- Evidence consolidation in `.planning/phases/06-*/` with pinned SHAs; markers timing-tolerant (presence, not order)
- Additive, objtype-gated edits to shared sources; amd64 leg behavior unchanged (D-13: the allowlist filters the aarch64 leg only)
- Dual-arch default build (`arch=(amd64 aarch64)`) stays — the allowlist keeps the aarch64 leg proportional while amd64 sweeps full-tree

### Integration Points
- `sys/src/mkfile.proto` + `sys/src/cmd/mkfile` (DIRS) — where the allowlist filter hooks the aarch64 sweep; the coverage report proves complete reach here
- `build/aarch64/tiers/` + `build/aarch64/markers/` (new) — D-13/D-14 data homes
- `flake.nix` + `.github/workflows/` — the D-16 CI extension and the D-04 x86_64 leg
- `nix/build.sh` / lima choreography — the darwin tier-build path (documented-manual)
- r9 repo: `aarch64/src/syscall.rs` dispatch grows to the measured tier-1 set (r9-side commits, push-before-pin)

</code_context>

<specifics>
## Specific Ideas

- The acceptance story, end-to-end: Wave 0 lands (gethunk repros red→green, FPdbleword fixed, grep resolved, structural blockers cleared) → the coverage sweep commits a report proving DIRS reaches all ~274 cmd entries → the generator audit lists every build-then-run generator with host-toolchain proof → tier0-libs.txt's 22 libs each get a fresh `.a` artifact marker → tier1-cmds.txt binaries run the in-guest rc sweep on the donor: per-binary OK markers stream over serial, awk/bc prints a correct float (D-01's gate), and the guest's own 7a/7c/7l compile + run hello (D-11) → the same sweep bakes into the boot-r9 image and passes against r9's measured-set dispatch (D-12) → ledgers append, CI regenerates the coverage report and runs the full pipeline on linux, including the x86_64 9k pool-panic verdict (D-04).
- Watch-out: the dual-arch default build means a broken aarch64 dir breaks everyone — the D-13 allowlist is the pressure valve; never "just fix it inline" outside a tier (Pitfall 1's warning).
- Watch-out: `build/aarch64/out/` last-writer-wins ownership (CONCERNS.md fragile area) — tier builds and the r9 boot-test bake share it; the mkimage.sh real-init rodata guard is the existing tripwire, and tier machinery must not add a third writer.
- Watch-out: the in-guest sweep runs under TCG in CI — marker assertions must be presence-based with generous timeouts (v1.0 smoke discipline), and the sweep must be *sourced*, never expect-typed (05-03's lesson: the donor's devcons refuses typed console reads, Egreg by design).
- Watch-out: Pitfall 3's bug classes (unsigned char on ARM, varargs width, LP64 fossils, struct punning) mostly survive compilation — the execution gates are the net; Wave-0 grep-audits file findings as per-dir checklists, not drive-by fixes.
- Watch-out: FPdbleword's fix changes float output tree-wide — if any v1.0 gate asserted float-formatted output (none did — the smoke never prints a float), it would need re-baseline; the tier-1 float canary is the first such assertion.
- User overrides to honor downstream (deliberate, don't second-guess): minimega deferred despite the Phase-1 harness being the known venue (darwin-aarch64 dev fleet); full CI pipeline over faster local-only gates (clean-checkout honesty is the milestone's core value); single-wave tier-0 over milestone-priority sequencing (planner's dependency sort is trusted).

</specifics>

<deferred>
## Deferred Ideas

- minimega harness work (amd64 test VMs) — deferred until the user has an amd64 development host (D-03)
- Tiers 2–4: services (ip/, ndb/, auth/, aux/listen), GUI (rio/acme/sam/plumb compile + donor syscall traces), long tail (upas/lp/games) + bootfs payload — Phase 10
- fossil/venti runtime verification — no block device in r9 this milestone (compile-gated in Phase 10)
- gs (ghostscript) — excluded from tiers; REQUIREMENTS USPACE-09 lists it for Phase 10's long tail; PITFALLS recommends exclude-and-document — the tension resolves at Phase 10 planning, not here
- hswal stride bugs beyond commonllp64 (9cc libmach) — stays logged; fix when a host tool needs non-_MAGIC formats
- ar/mk general staleness mechanism (membership-change detection) — D-07's fresh-archive rule covers the swept dirs; the general fix remains hygiene-phase scope (02-10)
- `nix run .#build` exit-status plumbing (02-12) — markers remain the truth; the plumbing fix stays deferred
- virtio-9p host mount, SMP/PSCI, RTC, virtio-blk — future milestones
- raspi3b-on-qemu-11.1.2 upstream defect — pre-existing, human-accepted 2026-11-01

</deferred>

---

*Phase: 6-Full-Tree Userspace — Wave 0 + Libraries/Core Tiers*
*Context gathered: 2026-11-04*
