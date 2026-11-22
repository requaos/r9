# Phase 6: Full-Tree Userspace — Wave 0 + Libraries/Core Tiers - Research

**Researched:** 2026-11-04
**Domain:** Plan 9 mk build-system engineering + cross-compilation sweep machinery (in-tree C/mk/rc/shell; no external packages)
**Confidence:** HIGH (every load-bearing claim verified against in-tree files read this session; external facts confined to one [CITED] CI claim)

## Summary

Phase 6 is not a library-research phase — the milestone bundle (`.planning/research/PITFALLS.md` et al., HIGH confidence, 2026-11-04) already owns the WHY/WHAT (tiered allowlists, donor-first execution, Wave-0 audits). What the planner needs is the file-level HOW, and that is fully determinable in-tree. This research read every site: the two `gethunk` landmines (`sys/src/cmd/cc/macbody:851-868`, `sys/src/cmd/6l/obj.c:1290-1310`), the proven fix pattern (`sys/src/cmd/7l/unixcompat.c:28-62`), the FPdbleword defect (`aarch64/include/u.h:54-61` vs the correct `amd64/include/u.h:49-56`), the mk sweep machinery (`sys/src/mkfile`, `sys/src/cmd/mkfile`, `mkone`/`mkmany`/`mksyslib`, `build/scripts/linux-build`), the harness scripts the tier machinery extends (`subset-build.sh`, `subset-exec.sh`, `mkimage.sh`, `smoke-r9.sh`/`smoke-r9.expect`), and the Wave-0 blocker states (jpg fixed in Phase 1 commit `7be7e98c8`; mahjongg never broke in the namespace path; 4a's failure was the disabled 386 pass).

Three findings reshape naive planning assumptions. First, the sweep's recursion is not one DIRS list but a tree: `sys/src/cmd/mkfile` computes DIRS from `ls` (line 17), container mkfiles (`ip`, `aux`, `auth`, `games`) carry their own explicit `TARG`+`DIRS` — and `ip/mkfile` overrides `all`/`install` to recurse DIRS only while mkmany's rules still build its 22 TARGs per-target; the allowlist filter must therefore handle both dir-granular and per-target membership, with a uniform keep-rule. Second, the tier data files at `build/aarch64/tiers/` are NOT reachable as `/build/...` inside the build namespace (neither `BUILD_SETUP_NAMESPACE` nor `nsrun.sh` binds `/build`) — but both run under `unshare -m` (not chroot), so host-absolute `$HARVEY/build/aarch64/tiers/...` paths resolve fine in mk recipes (`$HARVEY` is exported in the devshell and proven used in-namespace by subset-build.sh's guest script). Third, the donor execution venue (`subset-exec.sh`, `run-gate.sh`, `donor-vm-build.sh`) is lima-coupled today — D-16's full-pipeline CI requires a native-linux leg; `subset-build.sh`'s `NATIVE_LINUX` split and `donor-ns-build.sh`'s self-re-entrant `--ns` shape are the templates.

**Primary recommendation:** Wave 0 lands the two gethunk accumulate-when-contiguous fixes (verbatim pattern quoted below) + the FPdbleword `ulong→uint` one-liner + the grep hygiene sweep as atomic commits with synthetic timeout-gated repros; then the allowlist machinery lands as (a) the two committed data files, (b) an objtype-gated mk-layer filter keeping an unfiltered `ALLDIRS`/`ALLTARG` alongside filtered `DIRS`/`TARG` so `checkall`-based coverage stays total, and (c) `tier-build.sh`/`tier-exec.sh` extending the subset scripts with the D-07 fresh-archive rule, the sourced rc sweep (D-10), and append-only sha256 ledgers (D-14).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Wave-0 scope + verdict standards**
- **D-01:** The **FPdbleword fix joins Wave 0**. `aarch64/include/u.h:54-61` changes `ulong lo/hi` → `uint lo/hi` (matching the working `amd64/include/u.h:49-56`), libc + libfmt rebuild, and a **float-print marker lands in the tier-1 execution gate** so `%e/%f/%g` correctness is execution-asserted, not assumed.
- **D-02:** The **`/usr/bin/grep` absolute-path fix joins Wave 0 as harness hygiene** — resolve a host grep once per script (`command -v` fallback chain) across the six `build/aarch64/` scripts that hardcode it. One commit.
- **D-03:** **minimega is deferred wholesale** until the user has an amd64 development host. This removes the Phase-1 `build/mm/` venue from Phase 6's toolbox.
- **D-04:** **USPACE-10's pool-panic re-test runs in a new x86_64 CI leg** (GitHub `ubuntu-latest` exposes `/dev/kvm`): build 9k10cpu with the pswt-fixed 6c, boot under qemu-system-x86_64 KVM, serial-captured verdict asserting banner + no "bad magic" pool panic — **recorded either way** (even a FAIL verdict closes USPACE-10 with information).

**Tier-0 libraries (22 outstanding)**
- **D-05:** **Single wave, planner sorts** — all 22 libs are one wave; internal order is the planner's dependency sort with no milestone-priority weighting.
- **D-06:** **Library execution marker = artifact + consumer-carried execution.** Per-lib artifact marker (fresh `.a` built from portable C via the mksyslib fallback) + execution proven by tier-1 consumer binaries that link them + in-tree lib test binaries run on the donor where they exist (libsec `*test.c` class). No new per-lib smoke-binary machinery.
- **D-07:** **Fresh-archive per lib in the sweep** — the tier sweep's lib build deletes the target `.a` before repopulating. Deterministic kill of the 02-10 staleness class.
- **D-08:** **All 22 attempted; exclusion only documented** — a lib with no aarch64 consumer (libdynld/libaml/libscribble are the edge cases) gets excluded by name with a committed comment, never silently dropped.

**Tier-1 core commands + guest build chain**
- **D-09:** **Membership = v1.0 16-subset + boot chain + guest build chain + researcher-proposed coreutils classics.** The guest build chain is 7a/7c/7l/cc/cpp/mk/yacc/lex as aarch64 binaries (USPACE-06 literal); the coreutils extension (awk/sort/tr/uniq/tail/dd/tar/chmod/tee/xargs/basename/dirname/kill/sleep class) is proposed by the researcher from the 9front donor's own `/bin` baseline and locked in the PLAN. **At least one float-printing canary (awk or bc) is mandatory** — it gates D-01's FPdbleword fix.
- **D-10:** **Gate mechanics = in-guest scripted rc sweep + serial markers.** A committed in-guest rc script runs each tier-1 binary with fixed inputs and emits per-binary OK/FAIL markers over serial; the host asserts the marker set. This is the 05-03 sourced-sweep pattern (sourced, not typed) — scales to dozens of binaries, preemption/TCG-timing tolerant.
- **D-11:** **The guest self-host canary is in tier-1** — inside the donor guest, our aarch64 7a/7c/7l compile a known source (hello-class) and the produced binary runs and prints.
- **D-12:** **The r9-side tier-1 gate is Harvey's composed smoke extended** — the tier-1 rc sweep bakes into the boot-r9 image; `boot-r9-smoke` (or a tier-specific sibling flake app) drives it and asserts markers. r9's `cargo xtask boot-test` profiles stay unchanged kernel-focused regression gates.

**Allowlist machinery + marker/CI shape**
- **D-13:** **The append-only allowlist is per-tier committed data files** (`build/aarch64/tiers/tier0-libs.txt`, `tier1-cmds.txt`) — plain data, one dir per line with comments. The mk layer filters the **aarch64 leg's** DIRS through them; the amd64 leg sweeps full-tree as today.
- **D-14:** **Markers are per-tier append-only ledgers** (`build/aarch64/markers/tier0.ledger`, `tier1.ledger`) — machine-readable lines (dir, artifact sha256, exec marker, evidence path, commit, date) so scripts/CI can diff them. The phase EVIDENCE.md stays the human summary.
- **D-15:** **The USPACE-02 dir-coverage report is committed AND CI-regenerated-and-diffed** (red on drift).
- **D-16:** **Full pipeline in CI** (linux legs): Wave-0 synthetic repro gates, generator audit, coverage diff, tier-0/1 donor+r9 execution gates, and the new x86_64 9k pool-panic leg all run in CI; darwin stays documented-manual. Plan-time may split fast host-side checks from slow boot legs into separate jobs.

**Carried forward (locked in roadmap/prior phases)**
- Tier structure: tier-0 = libs, tier-1 = core cmds + build chain; tiers 2–4 = Phase 10. ape excluded by decision; gs excluded from tiers; other-arch toolchain letters excluded from the aarch64 target sweep.
- Donor-kernel execution FIRST, then r9 via measure-first traces (USPACE-04); loud-unimplemented default stays.
- mk exit status is never trusted (namespace build always exits 0 — 02-12); artifact+execution markers are the only gates.
- Allowlist growth is append-only, never a flag-flip; a dir enters the default build only after passing its per-dir gate.
- gethunk ×2 fixes use the proven `7l/unixcompat.c` accumulate-when-contiguous pattern; each fix is its own commit with a synthetic large-macro/large-input repro as the gate (Pitfall 4). The hswal item stays logged, not speculatively fixed.
- Generator discipline: generators always build with the host toolchain (pcc/9c into the per-OS host bin dir), never `$CC`; CI reds on `.7`-magic generator artifacts (USPACE-03).
- Bake-into-bootfs for gates (deterministic), ufs-over-tcp for iteration.
- Evidence standard: marker assertions over serial, timing-tolerant (assert presence, never interleaving order); canonical evidence consolidated in `.planning/phases/06-*/` with the r9 commit SHA recorded where r9 is involved.
- amd64 green is sacred: shared-frontend fixes (cc/macbody, 6l/obj.c) must keep the 6c/6l amd64 toolchain and 9k kernel build green; additive/objtype-gated edits to shared sources.
- NMOUNT=54 divergence ledger lives and grows — divergent syscalls wall loudly on the donor.
- `aarch64` naming everywhere; Plan 9 C style (`sys/man/6/style`) in Harvey; r9 follows its rustfmt.toml.
- qemu shape for r9/donor boots: `-M virt,gic-version=2` (r9) / donor-kernel drift accommodations in `build/aarch64/README.md` (gic-version=3, highmem-ecam=off, DTB pre-placement, GICR wake poke, console=0, virtio-net-pci-non-transitional); TCG on darwin; `-m ≤ 1G`.
- r9 commits land on `main` of `requaos/r9`; r9-side changes follow push-before-pin discipline; Harvey `flake.lock` r9 pin advances as its OWN commit with a fresh hosted CI run, never bundled.

### the agent's Discretion
- Exact tier-0 lib ordering within the single wave (dependency sort), and the exact coreutils extension list proposed from the donor `/bin` baseline (D-09) — locked in PLAN, user reviews there.
- Physical layout/naming inside `build/aarch64/tiers/` and `build/aarch64/markers/` and the mk-layer filtering mechanism that consumes the allowlist (mkfile.proto vs cmd/mkfile hook).
- Ledger line format details (field order, hash algorithm) — must stay machine-readable and diff-stable.
- CI workflow topology (extend `r9-shell.yml` vs sibling workflow; job split fast-vs-slow per D-16) — subject to the everything-in-CI constraint.
- The in-guest sweep script's marker vocabulary and per-binary fixed inputs (D-10), and the self-host canary's source program (D-11).
- The x86_64 pool-panic CI leg's mechanics (job placement, serial capture/assert recipe) — the assertion content (banner + no bad-magic panic, verdict recorded either way) is locked by D-04.
- Wave-0 task ordering among the fixes (gethunk pair, FPdbleword, grep hygiene, structural blockers) — each remains its own atomic commit.

### Deferred Ideas (OUT OF SCOPE)
- minimega harness work (amd64 test VMs) — deferred until the user has an amd64 development host (D-03)
- Tiers 2–4: services (ip/, ndb/, auth/, aux/listen), GUI (rio/acme/sam/plumb compile + donor syscall traces), long tail (upas/lp/games) + bootfs payload — Phase 10
- fossil/venti runtime verification — no block device in r9 this milestone (compile-gated in Phase 10)
- gs (ghostscript) — excluded from tiers; the USPACE-09 tension resolves at Phase 10 planning, not here
- hswal stride bugs beyond commonllp64 (9cc libmach) — stays logged; fix when a host tool needs non-_MAGIC formats
- ar/mk general staleness mechanism (membership-change detection) — D-07's fresh-archive rule covers the swept dirs; the general fix remains hygiene-phase scope (02-10)
- `nix run .#build` exit-status plumbing (02-12) — markers remain the truth; the plumbing fix stays deferred
- virtio-9p host mount, SMP/PSCI, RTC, virtio-blk — future milestones
- raspi3b-on-qemu-11.1.2 upstream defect — pre-existing, human-accepted 2026-11-01
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| USPACE-01 | gethunk ×2 compile-hang fixes with regression proof | §Wave-0 Fix Specifics: verbatim buggy/fixed code, NHUNK ceilings, synthetic repro gate designs |
| USPACE-02 | Structural mk blockers fixed; DIRS reaches every cmd dir | §Wave-0 Blocker State (jpg FIXED Phase 1, mahjongg/4a non-issues in current path) + §Dir-Coverage Report (checkall mechanics + the ALLDIRS-unfiltered requirement) |
| USPACE-03 | Cross-compiled-generator audit; CI reds on `.7`-magic | §Generator Audit: 9 swept sites enumerated with file:line, audit artifact format, CI check recipe |
| USPACE-04 | Tiered append-only allowlist; donor-first then r9; markers only | §Allowlist Hook Mechanics + §Tier Sweep + Ledger Script Shapes |
| USPACE-05 | 22 libraries compile via mksyslib portable-C fallback | §Tier-0 Library Enumeration: exact 22, per-lib evidence, dependency sort, edge-case exclusions |
| USPACE-06 | Core commands + self-hosted build chain compile and donor-execute | §Tier-1 Membership (D-09 baseline verified against donor tree) + §Self-Host Canary |
| USPACE-10 | 9k-amd64 pool-panic re-test against pswt-fixed 6c | §USPACE-10 x86_64 CI Leg: plain-qemu boot recipe, assertion content, workflow placement |
</phase_requirements>

## Project Constraints (from AGENTS.md)

- **Plan 9 C style** (`sys/man/6/style`): tabs only, no `//` comments, `nil` never `NULL`, `-1`+errstr error idiom, `exits()` never `exit()`, no `#ifndef` header guards, automatic variables never initialized at declaration, `ARGBEGIN`/`ARGEND` for flags. Applies to the gethunk/FPdbleword edits (match the surrounding file's tighter historical style).
- **`aarch64` naming** everywhere (never `arm64`) in Harvey tooling/docs; tool letters stay 7a/7c/7l.
- **plan9port `$PLAN9/bin` LAST** in PATH (BUILD_ON_UNIX); host tools from `.build/hostbin-<os>-<arch>/bin` FIRST.
- **Vendor-and-own**: in-tree fixes preferred over external patch stacks; this fork owns the tree (upstream dormant since 2022-03-30).
- **Execution-gated ordering** (riscv64 vaporware rule): no dir lands before a running binary exercises it — tier allowlist entries are execution-gated, never compile-only for executables.
- **GSD workflow enforcement**: edits route through GSD commands; atomic commits per fix class.
- **Host platforms**: darwin-aarch64 primary dev; linux aarch64/x86_64; namespace build requires Linux mount namespaces (lima VM on darwin).
- **amd64 leg unchanged**: additive/objtype-gated edits to shared sources only.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Wave-0 fixes (gethunk ×2, FPdbleword, grep) | In-tree C sources + headers (host toolchain) | — | Shared frontend (cc/macbody) + 6l run on the host via plan9port 9c/pcc; fixes are host-compiled code paths |
| Tier allowlist data | `build/aarch64/tiers/*.txt` (committed data) | mk layer (consumer) | D-13 single-source-as-data; mk reads, never owns |
| Allowlist filtering | mk layer (`sys/src/mkfile`, `cmd/mkfile`, container mkfiles) | rc driver (nested exceptions) | D-13 letter: mk filters the aarch64 leg's DIRS; driver handles per-target/nested members (subset-build precedent) |
| Tier-0/1 build sweep | Guest/namespace mk (objtype=aarch64, 7c/7l) | `build/aarch64/tier-build.sh` driver | The build factory is mk-in-namespace; the driver loops tier files + asserts artifacts |
| Tier execution gates | Donor kernel guest (9front pin 350266104f) first | r9 guest second (D-12) | Donor-first unconfounds 7c-bugs from r9-gaps (v1.0 lesson) |
| In-guest sweep | Committed rc script staged into devroot/bootfs | Host marker assertion | D-10: sourced not typed; serial markers are the only truth |
| Coverage report | `mk checkall` (report-all, unfiltered) | CI regen+diff (D-15) | Reach must be measured unfiltered — see the ALLDIRS finding |
| USPACE-10 verdict | x86_64 CI leg (plain qemu, no minimega per D-03) | — | D-04; verdict recorded either way |
| r9-side dispatch growth | r9 repo (`aarch64/src/syscall.rs`) | Harvey measured traces | Cross-repo; push-before-pin; not Harvey-side gating |

## Standard Stack

This phase installs **zero external packages**. Everything is in-tree or already pinned by the devshell.

### Core (in-tree, verified this session)
| Tool | Location | Purpose | Why Standard |
|------|----------|---------|--------------|
| mk (plan9port) | `$PLAN9/bin/mk` | Build factory | The repo's only build tool [VERIFIED: nix/build.sh:108, sys/src/BUILD:6-10] |
| Vendored 7a/7c/7l | `sys/src/cmd/7{a,c,l}` @ 9front pin `350266104f1fed81c78c3d05fa44f523bec0ae4d` | aarch64 target toolchain + tier-1 build-chain deliverable | Phase 2 vendored; both the sweep's compiler and a tier-1 deliverable [VERIFIED: sys/src/cmd/7l/README:1-9] |
| pcc/9c/9l (plan9port) | `.build/hostbin-<os>-<arch>/bin` | Host toolchain for generators + toolchain bootstrap | Generator discipline (Pitfall 2): host tools, never `$CC` [VERIFIED: unix/mkfile:3-5 `CC=9c LD=9l O=o`] |
| `harness` (Rust, std-only) | `build/harness/` (srv9/drive9/gdbpoke9/mkfs9/trace-syscalls) | 9P root server, serial driver, wake poke, bootfs builder | v1.0-proven gate tooling [VERIFIED: build/harness/src/ listing] |
| expect | devshell | r9-side smoke driver | `smoke-r9.expect` shape; r9 devcons serves reads so typed works r9-side [VERIFIED: build/aarch64/smoke-r9.expect:88-96] |
| mkpaqfs (host) | `.build/bin/mkpaqfs` via `mkpaqfs-host.sh` | bootfs.paq bake | Harvey owns the byte format [VERIFIED: build/aarch64/mkimage.sh:56-61] |
| qemu-system-aarch64 11.1.2 | devshell (nixpkgs pin) | donor + r9 boot venue | TCG on darwin (HVF rejects GICv2/GICv3 donor shapes); drift accommodations in README [VERIFIED: /nix/store qemu-11.1.2 outputs present; `nix eval nixpkgs#qemu.version` → 11.1.2] |
| qemu-system-x86_64 | devshell (same qemu package) | USPACE-10 leg | plain `-kernel` multiboot; no minimega (D-03) |

### Supporting (host shell)
| Tool | Purpose | Note |
|------|---------|------|
| `od`/`tr` | a.out magic assertions (`00008c47` = R_MAGIC .7) | subset-build.sh:241-242 precedent — no `file(1)` dependency |
| `sha256sum` (coreutils, in devshell) | ledger artifact hashes | devshell line `coreutils` provides it on both hosts [VERIFIED: flake.nix:145] |
| GNU grep (resolved, D-02) | host-side marker greps needing `-q` | plan9 grep lacks `-q`; resolve once per script |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| mk-layer objtype-gated filter (recommended) | Driver-side precomputed DIRS env var | Env shape hides state per entry point and diverges between `nix run .#build`, subset-style runs, and future native builds; in-mkfile backquote filter works under ANY `objtype=aarch64 mk` invocation |
| Committed data files (D-13) | mkflags/CLI knobs | A flag is a flag-flip — explicitly forbidden ("never a flag-flip", carried-forward) |
| `od` magic checks | `file(1)` | `file` is not in the devshell closure; od is POSIX and already the in-tree idiom |

**Version verification:** no new packages → no registry verification needed. Toolchain pins are in-tree (`flake.lock` nixpkgs `1d060086d60c`, r9 `e72abfe6f864` — [VERIFIED: flake.lock parsed this session]).

## Package Legitimacy Audit

**No external packages are installed by this phase.** The phase consumes only in-tree sources, the already-vendored 9front toolchain (pinned, with per-dir READMEs), and the existing devshell closure (plan9port, qemu, expect, coreutils — all already pinned in `flake.lock`). The Package Legitimacy Gate has nothing to check.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
HOST (darwin via lima | linux native — same scripts)
│
│  build/aarch64/tiers/*.txt ── committed allowlist data (D-13)
│  build/aarch64/markers/*.ledger ── append-only marker truth (D-14)
▼
tier-build.sh ──(lima ssh | nsrun.sh --wrap native)──► NAMESPACE BUILD (unshare -m)
│                                                        │  /sys ← $HARVEY/sys
│                                                        │  /aarch64 ← $HARVEY/aarch64
│                                                        │  ($HARVEY host path ALSO live —
│                                                        │   unshare -m ≠ chroot)
│                                                        ▼
│                                              mk layer: sys/src/mkfile LIBS filter
│                                              cmd/mkfile TARG/DIRS filter (objtype-gated)
│                                              container mkfiles (ip/auth/games) same idiom
│                                                        │ per-lib: rm -f $LIB (D-07) → mk install
│                                                        │ per-cmd: mk install | mk <t>.install
│                                                        ▼
│                                              artifacts: aarch64/lib/*.a, aarch64/bin/*
│                                              emit T0-ART/T1-ART <name> <sha256>
▼
mkimage.sh ── bake bootfs.paq (root-tree + tier-1 binaries + tier1-sweep.rc + manifest)
▼
┌─ DONOR GATE FIRST (USPACE-04) ──────────────────────────────┐
│ tier-exec.sh: stage tier binaries → guest donor tree        │
│   → CONF=qemu bootdir block (data + h* binaries) → rebake   │
│   → boot 9qemu-subset-class kernel (TCG, GICv3 accommodations)
│   → staged termrc SOURCES /boot/tier1-sweep.rc (D-10)       │
│   → per-binary T1-OK/T1-FAIL markers over serial            │
│   → float canary (D-01) + self-host canary (D-11)           │
│   → host asserts marker set + zero panic lines              │
└──────────────────────────────────────────────────────────────┘
▼ (donor green)
┌─ r9 GATE (D-12) ─────────────────────────────────────────────┐
│ boot-r9-tier1 flake app: same bootfs.paq (sweep baked in)    │
│   → smoke-r9.expect-class driver: at % prompt,               │
│     `rc /lib/tier1-sweep.rc` (r9 devcons serves reads)       │
│   → assert same marker set against r9's measured dispatch    │
└──────────────────────────────────────────────────────────────┘
▼
CI (linux legs, D-16): fast host job (repros, generator audit, coverage
diff, grep hygiene) ∥ slow boot jobs (donor gate, r9 gate, x86_64 pool-panic)
```

### Recommended Project Structure (new files this phase)

```
build/aarch64/
├── tiers/
│   ├── tier0-libs.txt        # D-13: one lib per line, # comments
│   └── tier1-cmds.txt        # D-13: sys/src-relative entries (dir, single, or dir:target)
├── markers/
│   ├── tier0.ledger          # D-14 append-only
│   └── tier1.ledger
├── tier-build.sh             # subset-build.sh generalized (tier-file loops, D-07 fresh-archive)
├── tier-exec.sh              # subset-exec.sh generalized (donor gate, sourced sweep)
├── tier1-sweep.rc            # committed in-guest sweep (D-10); baked as data
├── generator-audit.txt       # USPACE-03 committed artifact
├── dir-coverage.txt          # USPACE-02 committed report (D-15)
└── coverage-sweep.sh         # report generator (checkall parse + find cross-check)
sys/src/cmd/mkfile            # + ALLTARG/ALLDIRS rename + objtype-gated filter
sys/src/mkfile                # + LIBS/SUBSYS/CMDS objtype-gated filter
.github/workflows/phase6.yml  # new sibling workflow (r9-shell.yml untouched)
```

### Pattern 1: The proven accumulate-when-contiguous gethunk fix

**What:** the hunk allocator must MERGE consecutive sbrk hunks when contiguous, or `while(nhunk < n) gethunk()` never terminates for n larger than the largest single hunk.
**When to use:** the two Wave-0 sites (cc/macbody, 6l/obj.c) — verbatim below in §Wave-0.
**Source:** [VERIFIED: sys/src/cmd/7l/unixcompat.c:28-62] — Phase 2's proven fix (the arm64 donor-kernel link incident: malloc(3.7MB) → thunk 88TB → ENOMEM without it):

```c
void
gethunk(void)
{
	char *h;
	long nh;

	nh = NHUNK;
	if(thunk >= 5L*NHUNK) {
		nh = 5L*NHUNK;
		if(thunk >= 25L*NHUNK)
			nh = 25L*NHUNK;
	}
	h = mysbrk(nh);
	if(h == (char*)-1) {
		diag("out of memory");
		errorexit();
	}
	/*
	 * Merge with the current hunk when contiguous (consecutive sbrk
	 * calls return contiguous addresses while nothing else interleaves
	 * the break) — REQUIRED for malloc()'s `while(nhunk < n)` loop to
	 * terminate for n larger than the largest single hunk. ...
	 */
	if(h == hunk + nhunk)
		nhunk += nh;
	else {
		hunk = h;
		nhunk = nh;
	}
	thunk += nh;
}
```

The fix edits ONLY the tail: replace `hunk = h; nhunk = nh; thunk += nh;` with the accumulate-when-contiguous block. Each site's own escalation ladder (10L for cc, 5/25L for 6l) and error reporter (`yyerror` vs `diag`) stay untouched — additive, style-matched edits (carried-forward amd64-safety rule).

### Pattern 2: The objtype-gated mk-layer filter (D-13 consumption)

**What:** filter DIRS/TARG/LIBS through the tier data files only when `objtype=aarch64`; unfiltered otherwise.
**Mechanics proven this session:**
- mk imports the environment into its symbol table and expands `$objtype`/`$NOMK`-style variables in backquote assignments BEFORE handing the command to the shell — evidence: `sys/src/cmd/mkfile:16-17` uses `` `{sed ... /$cputype/mkfile} `` and `` /^('$NOMK')$/d `` inside backquotes [VERIFIED: sys/src/cmd/mkfile:16-17].
- Recipes/backquotes run under rc (`FORCERCFORMK=1` in the namespace build) [VERIFIED: nix/build.sh:36, sys/src/BUILD:5].
- `$HARVEY` is exported in the devshell and already used by in-namespace guest scripts [VERIFIED: flake.nix:156, subset-build.sh guest script lines 103/146-147 addressing `$HARVEY/build/aarch64/out` from inside the namespace].
- **`/build` is NOT bound** in either namespace setup — `BUILD_SETUP_NAMESPACE` binds `/sys` (via `mount --bind .. /sys` from `$HARVEY/sys/src`), per-arch dirs, `/acme`, `/rc`, `/mail`, `/lib` (merged), `/unix`, `/usr/glenda`; `nsrun.sh` mirrors the set [VERIFIED: sys/src/BUILD_SETUP_NAMESPACE:5-86, build/aarch64/nsrun.sh:57-143]. Under `unshare -m` (not chroot) host-absolute paths stay live, so address the tier files as `$HARVEY/build/aarch64/tiers/...`, never `/build/...`.
- **Plan 9 grep dialect inside mk recipes**: plan9port grep lacks `-q` (mkimage.sh:126 comment) and has no `-E`; use rc's `~` list-membership/pattern operator or `grep pattern file >/dev/null` with `if`.

Reference idiom (shape for the planner; discretion area). In `sys/src/cmd/mkfile`:

```mk
TIERFILE=$HARVEY/build/aarch64/tiers/tier1-cmds.txt
ALLTARG=`{ls *.[cy] | sed '/\.tab\.c$/d;s/..$//'}
ALLDIRS=`{ls -l | sed '/^d/!d; s/.* //; /^('$NOMK')$/d'}
# D-13: the aarch64 leg sweeps the append-only allowlist; every other
# objtype sweeps the full tree exactly as before.  Keep-rule: candidate i
# at this mkfile's sys/src-relative path P is kept iff some entry E is
# exactly P/i (member) or starts with P/i/ (container of a member).
TARG=`{if(~ $objtype aarch64){
		keep=`{grep -v '^#' $TIERFILE}
		for(i in $ALLTARG){ if(~ $i @{...exact-match against keep's cmd/ entries' basenames...}) echo $i }
	} if not echo $ALLTARG}
DIRS=<same shape over $ALLDIRS, prefix-aware>
```

(Final rc spelling is planner's discretion; the constraints above — backquote timing, `$HARVEY`-relative paths, Plan 9 grep dialect, ALLDIRS kept unfiltered — are verified mechanics.)

**Why ALLDIRS must stay:** `checkall` must iterate the UNFILTERED set or the USPACE-02 coverage proof collapses to "the allowlist reaches the allowlist" (tautology). cmd/mkfile line 46's `for(i in cc $DIRS)` becomes `for(i in cc $ALLDIRS)`. For amd64, DIRS==ALLDIRS by construction (passthrough branch), so amd64 behavior is byte-identical.

### Pattern 3: Sourced in-guest sweep (D-10)

**What:** a committed rc script, staged into the guest as data, runs each tier binary with fixed inputs and prints one marker line per binary; the host asserts the full set.
**Why sourced, not typed (the 05-03 lesson):** on the donor, `opencons`'s `bind("#c","/dev",MBEFORE)` pins the kernel cons into the chain's fd 0 and the donor's devcons fails /dev/cons reads with Egreg BY DESIGN (userland console input is kbdfs-served); a traced/typed chain wedges [CITED: .planning/milestones/v1.0-phases/05-*/05-03-SUMMARY.md lines 49,133]. The proven accommodation: a staged termrc whose final exec is `exec rc -c '. /boot/oursweep.rc'` [VERIFIED: build/aarch64/trace-termrc:10-11]. NOTE the nuance: at the donor's normal `term% ` prompt, typed input DOES work (subset-exec.sh's 22 typed steps prove it — `console=0` bootargs route kbdfs) — but per-binary typing doesn't scale (D-10). One typed line (`rc /boot/tier1-sweep.rc`) at term% is the proven hybrid; the termrc-override variant needs zero console interaction. Either is D-10-conformant ("the sweep is sourced, not typed per-binary"); the termrc shape is more TCG-timing tolerant.
**Staging mechanics:** CONF bootdir entries carry the script as DATA — "mkrootall strips only file-identified executables; text rides through" [VERIFIED: build/aarch64/trace-boot.rc header lines 1-8]; subset-exec.sh lines 177-196 + 284-302 show the bootdir-block awk patch (marker-guarded, idempotent).

### Pattern 4: Fresh-archive rule (D-07)

```sh
# per lib, inside the namespace guest script (subset-build.sh:113-116 shape):
rm -f "/aarch64/lib/lib${name}.a"   # kill stale members (02-10: ar vu never prunes)
(cd "/sys/src/$name" && objtype=aarch64 mk install) || fail "lib $name"
[ -s "/aarch64/lib/lib${name}.a" ] || fail "lib $name artifact"
```

Origin: `amd64/lib/libmach.a` carried pre-excision `alphaobj.6` (defining `_is7`/`_read7`) long after alphaobj.c was deleted and duplicate-symbol'd the new 7obj.6 [CITED: v1.0 phase-02 deferred-items.md item "Archive member staleness"]. `mksyslib`'s rule is `ar vu $LIB $newprereq` [VERIFIED: sys/src/cmd/mksyslib:5-6] — append-only, never prunes.

### Anti-Patterns to Avoid
- **Trusting mk exit status:** the namespace build always exits 0 (sys/src/BUILD ends with `/bin/bash` break-glass; BUILD_SETUP_NAMESPACE's cleanup tail succeeds) [CITED: v1.0 02 deferred-items, item "nix run .#build exit status"]. Artifact existence + execution markers only.
- **expect-typed per-binary sweeps on the donor:** doesn't scale; TCG-timing fragile (D-10).
- **`/build/...` absolute paths in mk recipes:** not bound in the namespace — use `$HARVEY/...`.
- **Filtering the checkall path:** coverage must stay unfiltered (Pattern 2's ALLDIRS).
- **Editing amd64-leg behavior:** every filter branch must be objtype-gated with a passthrough `if not`.
- **A third writer to `build/aarch64/out/`:** last-writer-wins fragility (CONCERNS.md) — tier builds harvest into the existing owners' flows (subset-build.sh's per-file sync; mkimage.sh's real-init rodata guard stays the tripwire).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Serial drive + marker assertion | New expect/python driver | `harness drive9 --steps` (donor boots) / `smoke-r9.expect` `stage` proc (r9) | v1.0-proven; two-occurrence echo discipline and PANIC_RE fast-fail already encoded [VERIFIED: smoke-r9.expect:112-129] |
| 9P root server for donor guest | Go ufs / new server | `harness srv9 serve --writable-prefix /tmp --writable-prefix /usr/glenda/tmp` | subset-exec.sh:381-383 proven; the writable scratch is what the in-guest compile canary needs |
| bootfs bake | r9's `cargo xtask initrd` | Harvey's own mkpaqfs via `mkimage.sh` | mkimage.sh:147 + magic asserts 153-157; r9's xtask is never invoked (smoke-r9.sh header step 2) |
| Donor kernel rebake | Fresh relink script | subset-exec.sh's CONF bootdir patch + `mk CONF=qemu` targeted rebake (rm qemu.c/qemu.root* first) | subset-exec.sh:225-237 proven, idempotent marker-guarded |
| Diff-stable ledger hashing | harness-side sha256 (std-only Rust has no sha256!) | devshell `sha256sum` (coreutils) | flake.nix:145 [VERIFIED]; harness is deliberately std-only (flake.nix:46-48) |
| Dir-coverage walk | A new recursive finder | `mk checkall` (report-all, accumulates failures, prints full list) + a `find -name mkfile` cross-check | checkall exists at sys/src/cmd/mkfile:44-57, built by Phase 1 exactly for this [VERIFIED] |

**Key insight:** every mechanism this phase needs already has a proven in-tree skeleton from v1.0; the phase is *generalization + gating*, not invention. Novel code is confined to the mk filter idiom, the sweep rc script, and the CI workflow.

## Wave-0 Fix Specifics

### 1. gethunk site 1 — `cc/macbody` (USPACE-01)

Buggy code verbatim [VERIFIED: sys/src/cmd/cc/macbody:851-868]:

```c
void
gethunk(void)
{
	char *h;
	long nh;

	nh = NHUNK;
	if(thunk >= 10L*NHUNK)
		nh = 10L*NHUNK;
	h = (char*)mysbrk(nh);
	if(h == (char*)-1) {
		yyerror("out of memory");
		errorexit();
	}
	hunk = h;
	nhunk = nh;
	thunk += nh;
}
```

- NHUNK ceiling: `#define	NHUNK		50000L` [VERIFIED: sys/src/cmd/cc/cc.h:25]; escalation caps at `10L*NHUNK` = 500000 bytes. Any single macro-related alloc > 500000 bytes loops forever.
- Triggering callers (all `while(nhunk < n) gethunk();` loops): macbody:139 (`dodefine` — macro definition storage), macbody:580, 638, 772 (expansion machinery); plus the shared lexer `lexbody:53,68` (`alloc`/`allocn`, surfaced via lex.c:1513,1528) [VERIFIED by grep this session]. Note: the filename is `macbody` (no `.c` — included fragment; `cc/mkfile:37` has `mac.$O: macbody`).
- `mysbrk` for cc comes from `sys/src/cmd/cc/compat` lines 8-12 (`return sbrk(size);`) [VERIFIED: file read this session].
- **Synthetic repro gate:** generate a C file with a single macro whose replacement text exceeds 600KB (shell loop writing `#define BIG "…"` then one use), compile with the freshly built host cc-family binary under `timeout 60`: pre-fix = hang (timeout kill = RED, the regression signature); post-fix = compiles (GREEN). Gate must run the HOST-built 7c AND 6c (the shared frontend is in both). Commit shape: fix + repro script + timeout gate in one atomic commit (Pitfall 4's discipline; carried-forward: "each fix is its own commit with a synthetic large-macro repro as the gate").

### 2. gethunk site 2 — `6l/obj.c` (USPACE-01)

Buggy code verbatim [VERIFIED: sys/src/cmd/6l/obj.c:1290-1310]:

```c
void
gethunk(void)
{
	char *h;
	long nh;

	nh = NHUNK;
	if(thunk >= 5L*NHUNK) {
		nh = 5L*NHUNK;
		if(thunk >= 25L*NHUNK)
			nh = 25L*NHUNK;
	}
	h = mysbrk(nh);
	if(h == (char*)-1) {
		diag("out of memory");
		errorexit();
	}
	hunk = h;
	nhunk = nh;
	thunk += nh;
}
```

- NHUNK: `NHUNK		= 100000,` [VERIFIED: sys/src/cmd/6l/l.h:121]; ceiling `25L*NHUNK` = 2.5MB.
- The 7l incident prototype: the donor-kernel link allocated 3.7MB for the embedded bootfs data → infinite hunk reset → thunk 88TB → ENOMEM [CITED: 7l/unixcompat.c:44-54 comment]. The 6l trigger class is the same (kernel embedded-root data via mkrootall → root.s → link): a large embedded root image, or a huge single Sym/string alloc.
- **Synthetic repro gate:** generate a C file with one enormous initialized data array (≥4MB — e.g. `char big[4194304] = {…};` via a generator loop), compile+link with 6c/6l under `timeout 120`: pre-fix link hangs (RED on timeout); post-fix links (GREEN). Planner: verify at plan time that the generated input routes through the hunk allocator (the linker's data-image assembly); if the array path doesn't trigger, the fallback repro is a generated source with a >2.5MB string literal (string-table path). Flag: exact repro input shape needs one plan-time spike with a pre-fix binary to confirm the hang signature (watchdog via `timeout`, never a silent retry).
- **amd64-safety (carried-forward):** both fixes are in host-run code paths — regression proof = rebuild host 6c/6l + cc.a via `nix run .#toolchain`, then the amd64 9k kernel build + `mk checkall` green (the Phase-1 gate).

### 3. FPdbleword one-liner (D-01, USPACE-06 gate interaction)

Defect verbatim [VERIFIED: aarch64/include/u.h:54-61]:

```c
union FPdbleword
{
	double	x;
	struct {	/* little endian */
		ulong lo;
		ulong hi;
	};
};
```

`ulong` is 64-bit on aarch64 (`typedef unsigned long ulong;` — aarch64/include/u.h:5), so the union is 16 bytes and `hi` sits outside the 8-byte double. Correct reference [VERIFIED: amd64/include/u.h:49-56]: same union with `uint lo; uint hi;`.

- **Fix:** `ulong lo;` → `uint lo;`, `ulong hi;` → `uint hi;` (two lines).
- **Consumers the fix unbreaks** [CITED: .planning/codebase/CONCERNS.md FPdbleword entry]: `sys/src/libc/port/frexp.c` (`e = (x.hi >> SHIFT) & MASK`, SHIFT=20), `sys/src/libc/port/nan.c` (isNaN/isInf/Inf/NaN read/write `a.hi`/`a.lo`), `sys/src/libc/fmt/fltfmt.c:94` (`frexp(f, &e)` — the `%e/%f/%g` path).
- **Rebuild order:** libc (port + fmt) FIRST, then every aarch64 binary (all link libc). In the sweep: the fix commit lands before any tier build; the tier-0 sweep's fresh libc build picks it up. `riscv64/include/u.h` carries the same latent defect — note in the commit message; do NOT fix (skeletal port, out of scope).
- **Gate (D-01):** float-print marker in the tier-1 execution sweep, e.g. `awk 'BEGIN{printf "%.3f\n", 1.0/3.0}'` must emit `0.333` on the donor AND r9. Prior v1.0 gates never printed floats (the smoke never does), so no re-baseline is needed [CITED: 06-CONTEXT.md specifics].

### 4. `/usr/bin/grep` hygiene (D-02)

Actual call-site inventory (one commit): subset-build.sh (2 hits: lines 60, 201), stage-build.sh (2: 40, 117), mkimage.sh (3: 127, 129, 131), vm-run.sh (1: 45), run-gate.sh (1: 99), pre-gate-build.sh (1: 83), donor-vm-build.sh (1: 85), subset-exec.sh (1: 116) — 9 hits across 8 files [VERIFIED: grep -c this session; CONCERNS.md said "six scripts" — the true count is 8 files]. Fix pattern per CONCERNS: resolve once at script top, e.g. `GREP=$(command -v /usr/bin/grep || command -v grep)` — or rely on the devshell PATH with `$PLAN9/bin` LAST (project rule). The rationale to preserve: plan9 grep lacks `-q` (mkimage.sh:126-128 comment), so the resolved grep must be a host grep; the fallback chain keeps that while dropping the absolute path.

### 5. Structural blockers — current state (USPACE-02)

| Blocker | State | Evidence |
|---------|-------|----------|
| jpg/mkfile `oc` derivation | **FIXED in Phase 1** (commit `7be7e98c8`, 2026-10-30): generators now `objtype=$cputype mk rgbycc.$cputype` (host 9c/9l via unix/mkfile), `./$cputype.rgbycc > ycbcr.h` | [VERIFIED: sys/src/cmd/jpg/mkfile:60-78 read this session; git show 7be7e98c8] |
| mahjongg cpp | **Non-issue in the current build path**: Phase 1 evidence — "the Dockerfile TODO's known cpp bug did NOT manifest in this build path — mahjongg compiled and installed with zero changes" | [CITED: 01-01-SUMMARY.md line 156]; mkfile is clean mkone shape [VERIFIED: sys/src/games/mahjongg/mkfile read] |
| 4a lex.c `rfork bind` | **Was the disabled 386 pass**, not the default sweep; 4a builds as an amd64 target today (Phase 1's 145-dir green checkall included it). As an aarch64 *target* it's excluded by decision (toolchain letters = host tools via BUILDTOOLCHAIN only) | [CITED: 01-01-SUMMARY.md:150 residual-failures note; PITFALLS.md notorious-dirs table] |

Wave-0's USPACE-02 residue: jpg's generator fix is host-correct for ANY target objtype (`objtype=$cputype` recursion is arch-independent) — re-verify once during the coverage sweep; the real work is the coverage report machinery (below) plus whatever NEW structural aborts the unfiltered aarch64 sweep surfaces (expected class: per-dir 7c failures recorded as data, not reach aborts).

## Tier-0 Library Enumeration (USPACE-05, D-05..D-08)

### The exact 22

`sys/src/mkfile:5-40` LIBS lists 35 libs [VERIFIED: file read this session]. The v1.0 green ledger is 13: the subset-build.sh:113 loop `for d in libc libbio libregexp libip libndb libmp libsec libString libthread lib9p libauth libauthsrv libflate` [VERIFIED: subset-build.sh:113] — superset of linux-build's `build_aarch64` lib list (12, no libflate) [VERIFIED: build/scripts/linux-build build_aarch64 fn]. **22 outstanding = 35 − 13:**

| # | Lib | .c files | Per-arch asm | Stale `.6`/`.8` committed | Consumer evidence (execution carrier) |
|---|-----|---------|--------------|---------------------------|----------------------------------------|
| 1 | libaml | 5 | none | 10 | **NONE** — only self-consumers; 9k kernel doesn't include aml.h [VERIFIED: grep this session] → D-08 documented exclusion (execution leg) |
| 2 | libavl | 1 | none | 2 | venti/copy.c includes avl.h (tier-4/Phase 10 consumer; no tier-1 carrier) |
| 3 | libbin | 1 | none | 2 | upas/bayes/dfa.c, venti/copy.c (Phase 10) |
| 4 | libcomplete | 1 | none | 2 | acme/text.c (tier-3), gs (excluded) |
| 5 | libcontrol | 16 | none | 32 | GUI apps (tier-3); pulls libscribble via scribble.c |
| 6 | libdisk | 4 | none | 6 | disk/prep class (Phase 10) |
| 7 | libdraw | 57 | none (all portable C in this tree) | 112 | rio/acme (tier-3); drawterm vendored headers only |
| 8 | libdynld | 16 | **needs `dynld-$objtype.$O` → `dynld-aarch64.c` ABSENT** (only `dynld-arm64.c`, Inferno import d6405e5fb) | 6 | **ZERO** dynld.h includers outside libdynld [VERIFIED: grep] → see edge cases |
| 9 | libframe | 9 | none | 18 | acme/sam (tier-3) |
| 10 | libgeometry | 6 | none | 12 | libdraw-adjacent; no tier-1 carrier |
| 11 | libhtml | 4 | none | 8 | webfs/links class (Phase 10) |
| 12 | libhttpd | 17 | none | 34 | ip/httpd (tier-2/Phase 10) |
| 13 | liblex | 5 | none | 10 | lex itself links it? (verify at plan); no tier-1 carrier |
| 14 | libmach | 44 | none (vendored 7.c/7db.c/7obj.c are portable C) | 74 | **nm** (`sys/src/cmd/nm.c:8` `#include <mach.h>`) — a tier-1-carrier candidate if nm joins tier-1 [VERIFIED: grep] |
| 15 | libmemdraw | 25 | none | 44 | libdraw family consumers (tier-3) |
| 16 | libmemlayer | 12 | none | 24 | tier-3 GUI |
| 17 | liboventi | 16 | none | 32 | venti suite (Phase 10) |
| 18 | libplumb | 3 | none | 6 | plumber (tier-3) |
| 19 | libscribble | 3 | none | 6 | only libcontrol/scribble.c [VERIFIED: grep] — no tier-1 binary carrier → D-08 documented exclusion (execution leg) |
| 20 | libstdio | 49 | none | 98 | 153 cmd files include <stdio.h> (ape-flavored cmds; many Phase-10) |
| 21 | libsunrpc | 15 | none | 30 | 9nfs/nfs (Phase 10) |
| 22 | libventi | 30 | none | 58 | venti (Phase 10) |

File-count/asm/stale columns: [VERIFIED by `find` this session]. The stale `.6`/`.8` objects in nearly every dir are exactly USPACE-05's "do NOT cargo-cult" warning — the sweep builds fresh `.7` objects from portable C via mksyslib's `%.$O: %.c` rule [VERIFIED: sys/src/cmd/mksyslib:13-14] and never touches the committed `.6`/`.8`.

### Dependency sort (D-05 — planner's order; advisory only)

mksyslib libraries only `ar` objects — they never link — so lib-to-lib link deps don't constrain build order; only header/include adjacency matters, and headers resolve via `/sys/include` regardless of build order. Recommended single-wave order (leaf-ish first, matching the existing alphabetical LIBS order is also fine — this ordering is informational for the ledger narrative):

`libavl libbin libgeometry liblex libstdio libcomplete libdisk libplumb libframe* libmemdraw libmemlayer libdraw libscribble libcontrol libhtml libhttpd libsunrpc libmach libfmt…` — no: libfmt doesn't exist (fmt is libc/fmt). Final proposed order: libavl, libbin, libcomplete, libgeometry, liblex, libstdio, libdisk, libplumb, libscribble, libmemdraw, libmemlayer, libdraw, libframe, libcontrol, libhtml, libhttpd, libsunrpc, libventi, liboventi, libmach, libaml, libdynld (the two documented-exclusion candidates LAST so the wave's exclusions are recorded after all artifact evidence exists). [ASSUMED: ordering is cosmetic; no hard constraint found in any mkfile.]

### Edge-case decisions (D-08) — per-lib evidence

- **libdynld:** two defects for aarch64: (a) mkfile:6 requires `dynld-$objtype.$O` = `dynld-aarch64.7`, but only `dynld-arm64.c` exists (single Inferno-import commit `d6405e5fb`) [VERIFIED: ls + git log this session]; (b) zero consumers of dynld.h anywhere [VERIFIED: grep]. Options for the planner: (i) attempt a rename `dynld-arm64.c → dynld-aarch64.c` (D-11 rename-on-import precedent) and compile — the artifact marker then stands with execution excluded (no consumer); (ii) documented exclusion entirely. D-08 says "all 22 attempted" — recommend (i): the rename is one additive commit; if it fails to compile, the exclusion comment records the attempt + failure.
- **libaml:** compile attempted (5 portable C files, mksyslib shape [VERIFIED: libaml/mkfile read]); no consumer → execution-leg documented exclusion.
- **libscribble:** compile attempted; consumer chain is libcontrol-internal (createscribble, control.h:200) with no tier-1 binary → execution-leg documented exclusion.
- **libmach:** vendored aarch64 wiring EXISTS (7.c/7db.c/7obj.c + setmach entry, mkfile FILES list includes 7/7db/7obj; `7obj.$O: /sys/src/cmd/7c/7.out.h` dependency present) [VERIFIED: libmach/mkfile read this session] — but it was NEVER built for aarch64 in v1.0 (not in subset-build.sh's loop; `/aarch64/lib/libmach.a` absent from current outputs). Including `nm` in tier-1 (proposed below) gives libmach consumer-carried execution via a baked `.7` object as nm's fixed input.

## Tier-1 Membership (D-09)

Membership = **v1.0 16-subset** (`rc grep cat cp date echo ls mkdir mv ps rm sed test wc` + `ip/ipconfig` + `auth/factotum` [VERIFIED: subset-build.sh:98-99, 123-135]) + **boot chain** (`boot init paqfs stacksmoke` [VERIFIED: subset-build.sh:144-166]) + **guest build chain** (7a/7c/7l/cc/cpp/mk/yacc/lex as aarch64 binaries) + **coreutils extension** (researcher-proposed).

Guest build chain mechanics: `mk` = sys/src/cmd/mk (mkone-shaped dir [VERIFIED: head of mk/mkfile]); `cpp` = sys/src/cmd/cpp (mkone [VERIFIED]); `yacc` = sys/src/cmd/yacc.c (single-file); `lex` = sys/src/cmd/lex/ (dir); `cc` = the shared frontend — as a binary it doesn't exist standalone; its tier-1 evidence is `cc.a7` (cc/mkfile builds `LIB=cc.a$O` [VERIFIED: cc/mkfile:3,29]) plus the 7c binary that links it. `/aarch64/bin/7a,7c,7l` already exist as target binaries (v1.0 green [VERIFIED: ls aarch64/bin this session]).

**Proposed coreutils extension** (every entry verified to exist in Harvey's tree AND in the 9front donor baseline at .build/9front-donor/sys/src/cmd [VERIFIED: both listings this session]):

| Entry | Form | Fixed-input sweep idea (discretion) |
|-------|------|--------------------------------------|
| awk | dir (generator: maketab) | **MANDATORY float canary**: `awk 'BEGIN{printf "%.3f\n", 1.0/3.0}'` → `0.333` (D-01 gate) |
| bc | bc.y single | second float canary (optional; note Plan 9 bc prints `.333` without leading zero — verify exact output at plan time) |
| sort, tr, uniq, tail, dd, tar, chmod, tee, basename, sleep | singles | round-trip on the baked sentinel file (subset-exec steps.tsv precedent) |
| cmp, comm, fmt, join, look, pr, split, strings, sum, md5sum, sha1sum | singles | known-content assertions |
| ed, dc, units, cal, file, touch, time, seq, read, du, xd | singles (units.y is yacc-single) | usage/output markers; `file` needs a magic db — prefer `xd` if the db is absent from the image |
| nm | single | links libmach → consumer-carries libmach; fixed input = a baked `.7` object |

NOT proposed (D-09's literal class names that don't exist on Plan 9): `xargs` (MISSING in both trees), `dirname` (MISSING; rc/`basename` idioms cover it), `kill` (MISSING as a cmd — process control is `/proc/n/ctl` writes) [VERIFIED: both trees' listings this session]. This deviation from D-09's parenthetical class list is factual, not a scope cut — the class names were illustrative ("class").

Tier-1 count: 16 + 4 (boot chain) + 8 (build chain: 7a 7c 7l cc(artifact) cpp mk yacc lex) + ~30 coreutils ≈ 58 gated entries. The D-10 sourced sweep is mandatory at this scale (typed steps would be ~120 expect stages).

## Allowlist Hook Mechanics (D-13)

The full call chain, verified this session:

```
nix run .#build → nix/build.sh (linux native | lima on darwin)
  → sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE → sudo unshare -m BUILD_SETUP_NAMESPACE
    → su -c ./BUILD → mk clean; mk libs; mk all   [sys/src/BUILD:6-10]
      → sys/src/mkfile: for(i in $LIBS $SUBSYS $CMDS)   [lines 53-58]
        → cmd: sys/src/cmd/mkfile all:V: $PROGS dirs    [line 59]
          → PROGS from TARG=`{ls *.[cy]...}              [line 3]
          → dirs: for(i in cc $DIRS) — DIRS=`{ls -l|sed} [line 17]
            → container mkfiles (ip/aux/auth/games/...) own TARG+DIRS
  → rc build/scripts/linux-build → arch=(amd64 aarch64) loop
    → build_amd64: cd /sys/src && objtype=amd64 {mk install}   [full-tree]
    → build_aarch64: EXPLICIT subset (libc, libbio, 7?, 10 libs, named cmds) [never touches sys/src/mkfile's all]
```

Key structural facts:
1. **The aarch64 leg currently never runs the mk recursion** — `build_aarch64` is a hardcoded subset in `linux-build` [VERIFIED: build/scripts/linux-build, build_aarch64 fn]. The phase grows the aarch64 leg to the filtered full sweep.
2. **Recursion is a tree, not one list.** cmd/mkfile derives DIRS from `ls` (dotdirs/unix/old/sml/dup excluded via NOMK [VERIFIED: line 10-14,17]); container mkfiles carry explicit TARG+DIRS; `ip/mkfile` overrides `all:V: $DIRS`/`install:V: installdirs` AFTER `</sys/src/cmd/mkmany`, yet `/amd64/bin/ip/ping` etc. exist — mk executes BOTH the mkmany `all:V: $PROGS` and the local `all:V: $DIRS` (multiple virtual-target rules both run — empirical evidence: ping AND ipconfig are both in amd64/bin/ip [VERIFIED: ls amd64/bin/ip this session]). **Consequence:** keeping container `ip` in cmd's filtered DIRS still builds ip's 22 TARGs unless ip/mkfile's TARG is also filtered. The filter idiom must be applied per container mkfile that holds tier-listed members.
3. **Uniform keep-rule** (handles every level): at a mkfile whose sys/src-relative path is P, candidate `i` is kept iff some tier entry E satisfies `E == P/i` (member) or `E` starts with `P/i/` (container of a member). One rule serves sys/src/mkfile (P="", candidates cmd/nboot/games/ape//acme via CMDS+SUBSYS), cmd/mkfile (P=cmd, TARG singles + DIRS), ip/mkfile (P=cmd/ip), auth/mkfile, games/mkfile (P=games via sys/src CMDS — note games lives at sys/src/games, not cmd/games [VERIFIED: ls this session]).
4. **Tier entry forms** (recommendation; discretion): `libdraw` (lib), `cmd/sort` (single), `cmd/awk` (dir), `cmd/ip/ipconfig` (nested dir), `cmd/auth/factotum:factotum` (dir:target — the dir's TARG carries fgui which needs the out-of-tier graphics stack; the per-target install rule `mk factotum.install` is the subset-proven shape [VERIFIED: subset-build.sh:118-128]).
5. **Two consumers of the same data:** (a) the mk-layer filter (default-build sweep), (b) the rc/sh drivers (tier-build.sh per-entry targeted gates + markers). The `cmd/auth/factotum:factotum` form is consumed by the driver (per-target install); the mk filter keeps container `auth` + `factotum` via the prefix rule and auth/factotum/mkfile's own TARG filter keeps only `factotum` when it declares `TIERDIR=cmd/auth/factotum`. Alternatively the driver handles all per-target entries and the mk filter handles dirs only — planner's discretion; the constraint is amd64-leg byte-parity.
6. **clean/nuke asymmetry (pitfall):** filtered DIRS means `mk clean`/`nuke` won't sweep non-tier dirs — stale `.7` objects can linger there. Mitigation is D-07's fresh-archive rule for libs + artifact assertions for cmds; never gate on cleanliness.
7. **LIBS filtering:** sys/src/mkfile's LIBS (lines 5-40) gets the same treatment (keep iff in tier0-libs.txt). libc must stay first-class: build_aarch64 builds libc explicitly BEFORE anything (current lines: `for(i in /sys/src/libc)...`) — keep that bootstrap line unchanged.
8. **The amd64 leg is untouched by construction:** every filter is `if(~ $objtype aarch64) … if not <exactly today's value>`.

## Generator Audit (USPACE-03)

### Complete inventory (all swept mkfiles grepped this session)

Patterns used: `<\|`, `./$cputype.`, `./$O.`, `$O.out` execution, `objtype=$cputype`, recipes executing a just-built target [VERIFIED: grep across sys/src this session].

**In the swept path (all/install reachable):**

| # | Site | Generator | Built by | Runs when | Host-safe? |
|---|------|-----------|----------|-----------|------------|
| G1 | sys/src/cmd/awk/mkfile:42-50 | `maketab` → `proctab.c` | `objtype=$cputype mk maketab.$cputype` → 9c/9l (unix/mkfile) | always | ✅ host ELF (`unix.maketab`) |
| G2 | sys/src/cmd/jpg/mkfile:60-64 | `rgbycc` → `ycbcr.h` | same idiom (Phase-1 fix) | always | ✅ (`unix.rgbycc`) |
| G3 | sys/src/cmd/jpg/mkfile:70-74 | `rgbrgbv` → `rgbv.h` | same | always | ✅ |
| G4 | sys/src/cmd/jtagfs/mkfile:44-52 | `genrtab` → `rtab.c` | same (Phase-1 fix f90918777) | always | ✅ |
| G5 | sys/src/cmd/spell/mkfile:18-25 | `pcode` → amspell/brspell | `$cpuobjtype.pcode` via `objtype=$cputype` | always | ✅ (pcode.c got the gcc14 const fix in Phase 1, 5c78cae6e) |
| G6 | sys/src/cmd/gs/mkfile:~147-160 | `$O.genarch` → `$objtype.h` | **`$CC` (target toolchain!)** but guarded `if(~ $objtype $cputype)` | guarded OFF for aarch64 → falls to `default.$objtype.h`; **`default.aarch64.h` does NOT exist** (386/alpha/amd64/arm/mips/power/riscv/riscv64 only) → `exit false` | ⚠️ guarded-not-run, but gs fails the unfiltered sweep; gs is tier-excluded by decision — the audit records the guard as the proof + names gs a documented coverage exclusion |
| G7 | sys/src/cmd/usb/usbd/mkfile:33-36 | `mkdev` → `devtab.c` | **rc script**, not compiled [VERIFIED: `file` → "Plan 9 rc shell script"] | always | ✅ host rc (plan9port) |
| G8 | sys/src/cmd/upas/bayes/mkfile:14-17 | `$O.regen` → `/mail/lib/classify.re` | `$CC` target build, guarded `if(~ $cputype $objtype)` | guarded OFF cross-compile; committed classify.re used | ✅ (execution skipped; the target binary is built but never run) |
| G9 | sys/src/cmd/7l/mkfile:35-36 + 7l/BUILD:18 | `sh mkcname` → `cnam.c` | host sh + ed | every 7l build | ✅ script, host-run |

**Out of the swept path (documented in the audit as not-swept):** `4l/mkfile:34`, `5l/mkfile:34`, `vl/mkfile:32`, `il/mkfile:39` (`./xxx $OFILES` — linker self-checks; toolchain letters excluded from the aarch64 target sweep); compiler `t:V`/`x:V` smoke targets (`9c:37`, `kc:36`, `4c:42`, `5c:36`, `vc:36` — manual targets); `bzip2/mkfile:22-27` (test target); `spell/mkfile:44-45` (`test:V`); `ptrace/mkfile:30` (`T:V`); `ip/httpd/mkfile:52-58` (`trial` targets); `jtagfs/mkfile:55` (`rtest:V`); `cmd/mkfile:144-158` (`compilers:V` — manual); `sys/src/9k/mk/parse` (awk config generator — kernel-side, host awk, not userspace sweep).

### Audit artifact format (committed; USPACE-03 literal)

`build/aarch64/generator-audit.txt` — one line per site, fixed field order (diff-stable):

```
G1 site=sys/src/cmd/awk/mkfile:43 generator=maketab artifact=unix.maketab built-by=objtype=$cputype-mk runs=always proof=host-magic
G6 site=sys/src/cmd/gs/mkfile:150 generator=genarch artifact=7.genarch built-by=$CC-guarded runs=never-for-aarch64 proof=guard-line+excluded-dir
…
```

### CI `.7`-magic check mechanics (USPACE-03's red)

A host-side script (runs post-sweep in the fast CI job):
1. For each `runs=always` row: locate the artifact in-tree; assert first-4-bytes ≠ `00008c47` (R_MAGIC .7) via `od -An -tx1 -N4` (subset-build.sh:241-242 idiom) and, on linux CI, == `7f454c46` (ELF) for compiled artifacts / `#!` for scripts. **A `.7`-magic generator artifact = automatic red.**
2. Negative sweep: `find sys/src -type f -name '*.7'` among generator basenames (`rgbycc rgbrgbv maketab genrtab pcode genarch mkdev regen`) that the build EXECUTED — executed-set is derivable from the build log (the `./unix.X` lines). Simpler total rule that needs no log parse: assert no file matching `7.{rgbycc,rgbrgbv,maketab,genrtab,pcode}` exists anywhere in the tree post-sweep (the `7.` prefix is what a `$CC`-built generator would be named under the current idioms). G6's `7.genarch` is the known exception — whitelisted with its guard line as the recorded proof.
3. Build-log scan for exec-format failures: `cannot execute|exec format error|Exec format error` → red (the historical symptom class, PITFALLS.md Pitfall 2 warning signs).

## Tier Sweep + Ledger Script Shapes (D-10/D-14/D-07)

### tier-build.sh (extends subset-build.sh)

Recommendation: **evolve subset-build.sh in place** — its hardcoded lib/cmd loops become tier-file loops (the v1.0 16-subset ⊂ tier1-cmds.txt by construction), so mkimage.sh/smoke-r9.sh inherit tier-1 without call-site changes. Keep the script name. New internals:

```sh
# tier-0 libs (fresh-archive per D-07):
while read -r line; do
  case "$line" in ''|'#'*) continue;; esac
  d="$line"
  name="${d}"  # tier0-libs.txt lines are lib dir names
  rm -f "/aarch64/lib/lib${name#lib}.a"          # D-07
  (cd "/sys/src/$d" && objtype=aarch64 mk install) || fail "lib $d"
  [ -s "/aarch64/lib/lib${name#lib}.a" ] || fail "lib $d artifact missing"
  echo "T0-ART $d $(sha256sum "/aarch64/lib/lib${name#lib}.a" | cut -d' ' -f1)"
done < "$HARVEY/build/aarch64/tiers/tier0-libs.txt"
# tier-1 cmds: dir → (cd dir && mk install); cmd/<single> → mk <s>.install; dir:t → mk t.install
```

(The guest-script half runs inside nsrun.sh/vm-run.sh exactly as today's lines 83-176; platform split unchanged.)

### tier-exec.sh (extends subset-exec.sh — the donor gate)

Same 4-phase shape (stage → CONF patch + rebake → boot → assert):
1. **Stage:** rsync tier-1 binaries to the guest donor tree `arm64/bin/tier1/` (subset-exec.sh:166-174 shape); stage `tier1-sweep.rc` + `hello.c` (canary source) as data.
2. **CONF bootdir block:** one line per binary (`/$objtype/bin/tier1/X hX`) + data lines for the sweep script and hello.c; awk-patch per subset-exec.sh:284-302 (marker-guarded idempotent).
3. **Boot** the rebaked kernel with boot-9qemu.sh's exact qemu invocation (TCG, gic-version=3, highmem-ecam=off, DTB pre-place, GICR poke, console=0 [VERIFIED: subset-exec.sh:404-428]).
4. **Sweep:** staged termrc override (trace-termrc precedent) whose tail is `exec rc -c '. /boot/tier1-sweep.rc'`; the sweep emits `T1-OK <name>` / `T1-FAIL <name>` per binary and `T1-DONE` last; ends with `exit` so init's restart-loop sleep is captured before teardown (trace-sweep.rc:8-13 precedent). Host asserts: every tier1-cmds.txt entry has a T1-OK, zero `panic|trap:` lines, `T1-DONE` present. Marker assertions are presence-based with generous timeouts (v1.0 smoke discipline; never interleaving order).

### tier1-sweep.rc shape (marker vocabulary is discretion; this is the proven idiom)

```rc
#!/bin/rc
# per binary: fixed input, known output; failure must not abort the sweep
fn check { # name cmd...
  if($2) echo T1-OK $1
  if not echo T1-FAIL $1
}
check echo /boot/hecho __T1_SENTINEL__   # …illustrative; real body per-binary
# float canary (D-01, MANDATORY):
fn floatgate {
  out=`{/boot/hawk 'BEGIN{printf "%.3f\n", 1.0/3.0}'}
  if(~ $out 0.333) echo T1-OK float-d01
  if not echo T1-FAIL float-d01
}
# self-host canary (D-11): see §Self-Host Canary
echo T1-DONE
exit
```

### r9 gate (D-12)

mkimage.sh gains the tier-1 payload: binaries into `/aarch64/bin/`, the sweep script at e.g. `/lib/tier1-sweep.rc`, manifest assertions extended (mkimage.sh:111-144 pattern — one `[ -f ]` per new baked path). New flake app `boot-r9-tier1` (mkScriptApp, smokeR9Pkg shape with `pkgs.expect`), driving a `tier1-r9.expect` that: staged boot → `% ` prompt → `send "rc /lib/tier1-sweep.rc\r"` → staged marker waits (presence-only, TCG-sized 600s/stage timeouts) → `T1-DONE` → zero-panic whole-file scan. r9's devcons serves reads so typed works (05-03 finding for 05-09). smoke-r9.expect stays untouched (v1.0 gate).

### Ledger line format (D-14 — discretion; proposal)

```
tier=0 dir=libdraw artifact=aarch64/lib/libdraw.a sha256=<64hex> exec=EXCLUDED:no-consumer evidence=.planning/phases/06-*/evidence/t0-libdraw.log commit=<40hex> date=2026-11-05
tier=1 dir=cmd/awk bin=awk sha256=<64hex> exec=T1-OK:donor,T1-OK:r9 float-d01=0.333 evidence=<path> commit=<40hex> date=2026-11-05
```

Fixed field order (`tier dir artifact/bin sha256 exec evidence commit date`), space-separated `key=value`, one line per entry, append-only. sha256 = artifact at gate time (evidence, not a reproducibility oracle — artifacts are gitignored build outputs and process-level reproducibility is the documented standard [CITED: CONCERNS.md reproducibility entry]). CI diffs ledgers against regenerated marker output (red on drift). Append discipline: new lines land in their own commits with the gate evidence (the r9 SHA recorded in the evidence path's log when r9 is involved).

## Self-Host Canary (D-11)

**Requirement:** inside the donor guest, OUR aarch64 7a/7c/7l compile a known source; the produced binary runs and prints.

Mechanics, all from proven pieces:
- **Closest existing harness piece:** `run-gate.sh` (devroot embed of hello + exec marker over serial) — the canary generalizes it. The compile happens in the guest's writable `/tmp` (srv9 `--writable-prefix /tmp` proven by subset-exec.sh's mkdir/cp/mv/rm round-trip).
- **Staged into devroot (CONF bootdir):** `h7a h7c h7l` (from `/aarch64/bin/`, v1.0-green target binaries) + `hello.c` (data — `build/aarch64/hello.c` exists, prints `hello aarch64\nharvey` [CITED: build/aarch64/README.md data files]) + sweep script.
- **Headers + libc:** our 7c/7l carry `thestring=aarch64` → include search `/aarch64/include`, library search `/aarch64/lib` [CITED: build/aarch64/README.md donor-ns-build entry]. The donor guest's `/sys/include/libc.h` is the donor's own (same 9front pin lineage as our vendored 7c — compatible for hello-class source). Harvey's `aarch64/include/` + `aarch64/lib/libc.a` are staged into the SERVED root (`$DIST/srvroot/aarch64/…`) host-side — subset-exec.sh already manipulates srvroot (`mkdir -p "$ROOTFS/tmp"` lines 144-146). 7c has the preprocessor INTEGRATED (macbody compiles into cc.a → 7c) — no cpp needed for the canary. 7l autolibs libc.a via libc.h's `#pragma lib`.
- **Sweep lines:** `cd /tmp && /boot/h7c -FTVw /boot/hello.c && /boot/h7l -o /tmp/hello hello.7 && /tmp/hello` → marker `T1-OK selfhost` when output matches the known string.
- **One unproven mechanic to verify at plan time:** executing a binary from the srv9-served WRITABLE prefix. Exec from the srv9-served read-only tree IS proven (the donor's entire userspace runs off it), and devroot-embedded `/boot/h*` exec is proven; the writable-prefix exec is the same server with write caps — very likely fine, but flag it. Fallback if it fails: `harness srv9` gains no new code — instead bind a second writable prefix or (last resort) compile-only marker + host-side R_MAGIC assert on a fetched copy of the produced binary (weaker: proves codegen, not in-guest exec — document if used).

## Dir-Coverage Report (USPACE-02, D-15)

**What the sweep concretely runs:** Phase 1's `checkall` target [VERIFIED: sys/src/cmd/mkfile:44-57] — report-all (brace-group accumulator `fail=()`, prints `checkall failures: …`, `exit 0` on green). Per dir it runs `mk $MKFLAGS all` and echoes `mk $i` per visit — **the echo log IS the reach evidence**.

**The filter conflict and its resolution:** with the D-13 filter engaged, `objtype=aarch64 mk checkall` would visit only allowlisted dirs — the coverage proof must run UNFILTERED. Hence Pattern 2's `ALLDIRS`/`ALLTARG`: checkall iterates `$ALLDIRS` (all arches), `dirs`/`all` use filtered `$DIRS`. The coverage sweep for aarch64 then visits every dir and records per-dir `build=ok|fail` — per-dir 7c failures are DATA (the Phase-10 backlog), not gate failures. **Gate = reach (every on-disk dir visited, no abort, no hang), not green.**

**Coverage scope:** cmd (274 entries at sys/src/cmd — 132 single-file + 142 dirs [VERIFIED: ls this session]) + games (sys/src/games — its mkfile lacks checkall; add an additive `checkall` target there) + nboot (efiboot.fat is explicit-only since Phase 1 commit 76c55c91a, so nboot recursion is clean).

**Committed form (D-15):** `build/aarch64/dir-coverage.txt` — header (commit, date, host triple, objtype) + one line per dir: `sys/src/cmd/ip/ipconfig visited=1 build=ok` … + explicit `excluded=NOMK|decision` lines for dirs intentionally out (ape — subsystem not cmd; gs — tier-excluded but VISITED in coverage; toolchain letters — visited, build=ok|fail recorded, target-sweep-excluded) + a tail cross-check: `find sys/src/cmd sys/src/games -name mkfile | wc -l` vs visited count. CI regenerates and `git diff --exit-code`s (red on drift, D-15).

**Hang watchdog:** checkall builds every dir — a residual gethunk-class hang wedges it. Wave-0's fixes + repro gates land FIRST (dependency order); the driver additionally watches log-tail staleness (kill at N minutes without a new `mk <dir>` line; N TCG-sized generous). The report's committed existence + the repro gates' green is the "no silent hangs" proof.

## USPACE-10 x86_64 CI Leg (D-04)

**Premise verification:** GitHub-hosted `ubuntu-latest` exposes `/dev/kvm` on standard x64 runners — GitHub changelog 2024-04-02 ("hardware accelerated Android virtualization" on 2-vCPU GitHub-hosted Linux runners) + corroborating runner research ("still confirmed working on ubuntu-latest in July 2026") [CITED: github.blog/changelog/2024-04-02…; github.com/randax/talos-box/issues/74]. KVM accelerates same-architecture guests only — the x86_64 leg gets KVM; the aarch64 legs stay TCG (v1.0 accepted). Verify `/dev/kvm` writability at workflow-authoring time and fall back to `-cpu qemu64` TCG with a large budget (boot-9k10cpu.sh's MM_TCG discipline: 600s).

**Plain-qemu boot path (minimega-free, D-03):** the .mm template's essence is `qemu -kernel 9k10cpu.elf` multiboot (multiboot header at `sys/src/9k/k10/l32p.s:25-28`, ELF via `6l -H5` [CITED: build/mm/boot-9k10cpu.mm:8-9]) + serial0 capture attached before start + banner grep. Plain equivalent:

```sh
qemu-system-x86_64 -kernel amd64/9k10cpu.elf -m 512 \
  -accel kvm -cpu host -display none -no-reboot \
  -serial file:"$LOG" &  # TCG fallback: -accel tcg -cpu qemu64
# budget-wait for banner, then classify
```

The kernel artifact: `sys/src/9k/k10/mkfile:22` links `$target.elf` (`-H5 -E0x00110000 …`); install copies `9k10cpu{,.elf}` to `/amd64` (mkfile:25-27); both exist in the current tree [VERIFIED: ls amd64/9k10cpu*]. The CI leg builds it from a clean checkout via `nix run .#build` (linux native — proven by r9-shell.yml's full-pipeline leg), guaranteeing the pswt-fixed 6c (fix `7efff3aa3` in `cc/pswt.c`, shared frontend → linked into 6c; the in-namespace 6c rebuild chains from the hostbin 6c which the toolchain app rebuilt post-fix).

**Assertion content (locked by D-04):** banner `Plan 9` present (k10 prints it unconditionally on COM1 — `main.c:191` [CITED: boot-9k10cpu.sh header]) AND no bad-magic pool-panic signature. Recorded signature from the 01-04 incident: "bad magic" + "panic: pool panic" + `boot 1: suicide: sys: trap: fault read addr=0x0` [CITED: v1.0 01 deferred-items item 2]. Note: the string "pool panic" does NOT exist verbatim in the current 9k tree [VERIFIED: grep this session — no hits in sys/src/9k]; the robust classify set is `bad magic`, `panic`, `suicide: sys: trap`. Healthy continuation markers: boot.c prints `argc=%d` (line 69) and `version...` (line 139) [VERIFIED: sys/src/9k/boot/boot.c read this session]. **Verdict recorded either way** — the job asserts verdict production (banner seen + classification written to the committed evidence), NOT the outcome; a FAIL verdict closes USPACE-10 with information and spawns a new deferred item (D-04's letter).

**Workflow placement (recommendation; discretion):** NEW sibling workflow (e.g. `.github/workflows/phase6.yml`), leaving `r9-shell.yml` untouched (v1.0 gate stays as-is). Jobs:
- `wave0-host` (fast): setup+toolchain → gethunk synthetic repros (timeout-gated) → generator audit + `.7`-magic scan → grep-hygiene grep (no `/usr/bin/grep` literal in build/aarch64/*.sh) → FPdbleword header assertion → coverage sweep + diff.
- `tier-donor` (slow, TCG aarch64): full build → tier-build → tier-exec (needs the native-linux donor venue — see below).
- `tier-r9` (slow, TCG aarch64): boot-r9-tier1 app.
- `x86_64-poolpanic` (KVM): as above.

**Supply-chain + hygiene for the new file (CONCERNS.md IN-06):** pin ALL actions by 40-char SHA — including `actions/checkout` (r9-shell.yml:35 leaves it at `@v4`; the new workflow must not repeat that) — and add `concurrency: { group: phase6-${{ github.ref }}, cancel-in-progress: true` (r9-shell.yml lacks one; 90-min serial runs amplify). Consider fixing r9-shell.yml's checkout pin as a separate hygiene commit (not this phase's gate).

## Common Pitfalls (phase-specific, beyond the milestone bundle)

### Pitfall P6-1: The tautological coverage proof
**What goes wrong:** coverage sweep runs with the allowlist filter engaged → "DIRS reaches every dir" proves only that the allowlist reaches the allowlist.
**Avoid:** `ALLDIRS`/`ALLTARG` stay unfiltered; checkall iterates them (Pattern 2). CI diffs the committed report.
**Warning signs:** the report's visited-count equals the tier-file count.

### Pitfall P6-2: `/build/...` paths in mk recipes
**What goes wrong:** mk filter reads `/build/aarch64/tiers/...` → file not found in the namespace (no bind) → filter returns empty → aarch64 leg builds NOTHING (or errors loudly, depending on branch shape).
**Avoid:** address tier files `$HARVEY/build/...` (host-absolute path stays live under unshare -m). Verified mechanics above.
**Warning signs:** aarch64 leg installs zero libs with no error.

### Pitfall P6-3: plan9 grep dialect inside mk recipes
**What goes wrong:** filter idiom uses `grep -q`/`-E` → plan9port grep (on PATH in the recipe shell) rejects the flags → filter silently empties or errors.
**Avoid:** rc's `~` operator or `grep pattern file >/dev/null`. (Host-side shell scripts get the D-02 resolved GNU grep — different world, don't mix.)

### Pitfall P6-4: lima-coupled donor venue blocks D-16 CI
**What goes wrong:** tier-exec.sh copied from subset-exec.sh hard-requires limactl (subset-exec.sh:115-121) → CI linux leg can't run the donor gate.
**Avoid:** implement the NATIVE_LINUX split in the donor build/exec scripts this phase — templates: subset-build.sh:37-57 (`NATIVE_LINUX=1` + nsrun.sh --wrap) and donor-ns-build.sh's self-re-entrant wrap→--ns→--build shape (it already runs guest-side; on linux CI the "guest" is the host's own namespace). The donor checkout fetch (clone + pin-verify 350266104f) must also run host-side on CI (donor-vm-build.sh:105-106's pin check logic).
**Warning signs:** CI job green only on a runner where someone pre-made a lima VM.

### Pitfall P6-5: The gs genarch artifact reads as a false `.7`-magic red
**What goes wrong:** the unfiltered coverage sweep builds `7.genarch` (7c) even though the guard prevents running it → naive artifact scan reds.
**Avoid:** the audit row G6 records the guard as the proof; the CI scan whitelists exactly the audited guarded rows. Any NEW `.7`-magic generator = red (that's the point of the check).

### Pitfall P6-6: Filtered clean/nuke leaves stale `.7` residue
**What goes wrong:** non-tier dirs never get `mk clean` → stale objects from pre-filter runs linger → future tier append hits phantom freshness.
**Avoid:** D-07 fresh-archive for libs; per-target artifact assertions for cmds; Wave-0 could optionally add a one-time `find sys/src -name '*.7' -delete` hygiene sweep before the first gated run (planner's call).

### Pitfall P6-7: out/ last-writer-wins gains a third writer
**What goes wrong:** tier builds harvest binaries to `build/aarch64/out/` while stage-build.sh owns it wholesale (`--delete`) and subset-build.sh owns per-file sync [CITED: CONCERNS.md fragile area].
**Avoid:** tier-1 binaries live in `aarch64/bin` (the existing harvest target); `out/` keeps exactly its two writers; mkimage.sh's real-init rodata guard (mkimage.sh:127-130) stays the tripwire.

## Code Examples

### Fresh-archive lib sweep (D-07 + artifact marker)
```sh
# inside the namespace guest script (subset-build.sh:113-116 idiom)
for d in $(grep -v '^#' "$HARVEY/build/aarch64/tiers/tier0-libs.txt"); do
	lib="lib${d#lib}.a"
	rm -f "/aarch64/lib/$lib"		# D-07: kill stale members (02-10 class)
	(cd "/sys/src/$d" && objtype=aarch64 mk install) || fail "lib $d"
	[ -s "/aarch64/lib/$lib" ] || fail "lib $d: no artifact"
	echo "T0-ART $d $(sha256sum "/aarch64/lib/$lib" | awk '{print $1}')"
done
```

### The aarch64-only mk filter (skeleton — discretion area; mechanics verified)
```mk
# sys/src/mkfile — LIBS filter (D-13). ALLLIBS keeps the full 35-list verbatim.
ALLLIBS=<the existing 35-entry list, renamed>
LIBS=`{if(~ $objtype aarch64){
		keep=`{cat $HARVEY/build/aarch64/tiers/tier0-libs.txt}
		for(i in $ALLLIBS){
			if(~ $i $keep) echo $i		# rc ~ = list membership, exact
		}
	} if not echo $ALLLIBS}
```
(Comments in the data file: `~` exact-match means `# …` lines can never match a lib name — safe without stripping. The amd64 branch reproduces today's expansion word-for-word.)

### In-guest float canary (D-01 gate)
```rc
# tier1-sweep.rc fragment
out=`{/boot/hawk 'BEGIN{printf "%.3f\n", 1.0/3.0}'}
if(~ $out 0.333) echo T1-OK float-d01
if not echo T1-FAIL float-d01
```

### CI `.7`-magic red (USPACE-03)
```sh
# post-sweep, host-side, fast CI job
bad=0
for f in $(find sys/src -type f \( -name 'unix.*' -o -name '*.rgbycc' -o -name '*.rgbrgbv' -o -name '*.maketab' -o -name '*.genrtab' -o -name '*.pcode' \)); do
	magic=$(od -An -tx1 -N4 "$f" | tr -d ' \n')
	case "$magic" in
	00008c47) echo "RED: .7-magic generator artifact: $f"; bad=1;;     # Plan 9 aarch64 a.out
	7f454c46) : ;;                                                      # ELF — host-correct
	23212f*)  : ;;                                                      # #! script
	*) echo "NOTE: $f magic $magic (record in audit)";;
	esac
done
exit $bad
```

## State of the Art

| Old Approach (v1.0) | Current Approach (Phase 6) | When Changed | Impact |
|---------------------|---------------------------|--------------|--------|
| Hardcoded 16-cmd subset in subset-build.sh + build_aarch64 | Tier data files consumed by mk filter + drivers (D-13) | this phase | Allowlist growth = append a line; dual-arch build stays proportional |
| Typed drive9 step table (22 expect steps) | Sourced rc sweep + serial markers (D-10) | this phase | Scales to ~58 tier-1 binaries; TCG-timing tolerant |
| `ar vu` append-only archives | D-07 fresh-archive per swept lib | this phase | 02-10 staleness class killed deterministically |
| lima-only donor exec venue | + native-linux leg (D-16 CI) | this phase | Donor gates run in GitHub CI |
| minimega amd64 boot harness | plain qemu `-kernel` + `-serial file:` (D-03 deferral) | this phase | USPACE-10 verdict without an amd64 dev host |
| mk exit status trusted-ish | markers only (carried forward) | v1.0 02-12 | Never gate on rc |

**Deprecated/outdated:** expect-typed per-binary gates at tier scale; `/usr/bin/grep` absolute paths (D-02); `build/mm/` as a Phase-6 venue (D-03).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | mk executes BOTH rules when a virtual target is defined twice (mkmany's `all:V: $PROGS` + ip/mkfile's `all:V: $DIRS`) — inferred from /amd64/bin/ip containing both TARG binaries (ping) and DIRS subdir installs (ipconfig) | Allowlist Hook §2 | If mk actually uses last-rule-wins, ip's TARGs wouldn't need filtering (simpler plan); if first-wins, container recursion breaks differently. Planner should confirm with a two-line mk experiment before locking the filter design |
| A2 | The tier-0 lib build order is unconstrained (mksyslib archives never link; headers resolve via /sys/include) | Tier-0 §Dependency sort | If a lib's mkfile secretly references another lib's artifact, ordering matters; none found by inspection, but the sweep will surface it as a per-dir failure (data, not abort) |
| A3 | Executing a guest-compiled binary from srv9's writable prefix works (exec proven from the read-only tree and from devroot; the writable prefix is the same server with caps) | Self-Host Canary | If it fails, fallback = compile-only marker + host-side R_MAGIC assert (weaker proof; documented) |
| A4 | The 6l synthetic repro (>4MB initialized array) routes through the hunk allocator's `while(nhunk<n)` loop | Wave-0 §2 | If the array path doesn't trigger, use the >2.5MB string-literal path; the repro must be confirmed to hang pre-fix (timeout) before it can gate green post-fix |
| A5 | `nm` links libmach for aarch64 without new arch gaps (libmach 7-format wired in Phase 2; nm.c:8 includes mach.h) | Tier-0 §14 | If nm exposes a libmach aarch64 gap, nm drops from tier-1 and libmach takes the documented-exclusion route instead |
| A6 | rc `~` list-membership semantics work identically in the namespace build's rc (plan9port rc vs in-tree rc) | Code Examples | Filter idiom fails loudly (empty DIRS) — visible immediately; alternative is grep-based |
| A7 | Plan 9 bc's exact float output string (leading-zero behavior) | Tier-1 table | bc canary assertion needs one empirical check at plan time; awk (mandatory canary) is unaffected |
| A8 | GitHub `ubuntu-latest` /dev/kvm availability persists through workflow authoring | USPACE-10 leg | TCG fallback (-cpu qemu64, 600s budget) documented in the same job — degradation, not blockage |

## Open Questions (RESOLVED)

All three questions were consumed at plan time; dispositions name the consuming tasks.

1. **mk double-rule semantics for virtual targets (A1)** — RESOLVED at plan time.
   - What we know: mkmany defines `all:V: $PROGS`; ip/mkfile re-defines `all:V: $DIRS`; both outcomes appear in amd64/bin/ip.
   - What's unclear: the exact mk rule-resolution semantics (both-run vs first/last-wins with V-target special-casing).
   - Recommendation: one plan-time spike (tiny mkfile experiment, 5 minutes) before locking the filter design; the uniform keep-rule works under either answer but the container-mkfile edit set changes.
   - **Disposition:** consumed by 06-02 — Task 1 runs the A1/A6 spike (the verdict line is recorded in the D-02 hygiene commit body) and Task 2's `checkpoint:decision` selects the container-mkfile edit set from it (planned-layout assumes both-run; last-wins would be presented, not silently switched).

2. **The guest build chain's execution markers beyond the canary** — RESOLVED at plan time.
   - What we know: D-11 locks 7a/7c/7l in-guest proof; cpp/mk/yacc/lex need execution evidence too (tier-1 membership).
   - What's unclear: fixed inputs per tool (tiny grammar for yacc, tiny mkfile for mk, `-D` smoke for cpp).
   - Recommendation: planner assigns per-tool fixed inputs (discretion area); all run in the guest's /tmp writable scratch.
   - **Disposition:** consumed by 06-06 Task 1 — per-tool fixed inputs assigned and committed: `tiny.y` (yacc → y.tab.c), `tiny.l` (lex → lex.yy.c greps `yylex`), `tiny.mk` (mk -f), sweep-generated `-D` smoke (cpp); all four execute in-guest on the donor with T1-OK execution markers — no chain member lands artifact-only (SC#5's literal chain).

3. **Whether tier-build absorbs subset-build or supersedes it** — RESOLVED at plan time.
   - What we know: mkimage.sh + smoke-r9.sh call subset-build.sh by name; the 16-subset ⊂ tier-1.
   - Recommendation: evolve subset-build.sh in place (name stable, loops become tier-driven) — one less call-site change; record in the plan.
   - **Disposition:** consumed by 06-03 — subset-build.sh/subset-exec.sh evolve in place (names stable; the hardcoded loops became tier-file loops); the promotion is recorded in 06-02's `assumption_delta_decision` block.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| nix (Determinate) | everything | ✓ | 3.21.0 / 2.34.6 [VERIFIED: `nix --version`] | — |
| lima VM `harvey` | darwin builds (build/exec legs) | ✓ exists (`~/.lima/harvey`, sockets present); running state unverified this session | — | `limactl start harvey` (scripts do this) |
| plan9port | mk/rc/9c host tools | ✓ via devshell (not on this shell's PATH — devshell-only) | nixpkgs pin `1d060086d60c` | — |
| qemu-system-aarch64 | donor + r9 boots | ✓ via devshell | 11.1.2 [VERIFIED: nix eval + /nix/store outputs] | — |
| qemu-system-x86_64 | USPACE-10 leg | ✓ (same qemu package) | 11.1.2 | — |
| hostbin toolchains | all builds | ✓ populated: `.build/hostbin-darwin-aarch64/bin`, `.build/hostbin-linux-aarch64/bin` [VERIFIED: ls] | — | `nix run .#setup && nix run .#toolchain` |
| 9front donor checkout | donor gates | ✓ `.build/9front-donor` @ pin 350266104f; `.build/9front-dist` carries 9qemu{,.u} + gate kernels [VERIFIED: ls] | — | donor-vm-build.sh re-fetch |
| r9 sibling checkout | r9-side dispatch growth | ✓ `../r9` present [VERIFIED: ls] | — | flake input only (kernel-virt package) |
| r9 flake pin | composed boots | ✓ `flake.lock` r9 `e72abfe6f864` (advanced past CONCERNS.md's recorded 9549cee) | — | pin-bump discipline per carried-forward rule |
| GitHub `ubuntu-latest` /dev/kvm | USPACE-10 CI leg | ✓ [CITED: github.blog changelog 2024-04-02 + corroboration] | — | TCG `-cpu qemu64` + 600s budget (documented in-job) |
| lima on linux CI | — | ✗ not applicable — native-linux leg replaces it (Pitfall P6-4) | — | subset-build.sh NATIVE_LINUX pattern |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** /dev/kvm (TCG fallback); lima-on-CI (native-linux leg).

## Security Domain

Build-system phase; ASVS categories mostly not applicable, but the phase owns real supply-chain surface:

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | partial | The gethunk fixes close unbounded-loop DoS in host tooling on adversarial-size inputs; the synthetic repros are the regression proof |
| V6 Cryptography | no | — (sha256 ledger hashing is integrity evidence, not crypto agility) |
| V14 Configuration / supply chain (ASVS-adjacent) | **yes** | All GH actions pinned 40-char SHA in the new workflow (r9-shell.yml header rule + CONCERNS IN-06); no new downloads this phase (donor pin already enforced: subset-exec.sh:105-106 `git rev-parse HEAD` == `350266104f…`); `permissions: contents: read` |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Slopsquatted/malicious package | Tampering | N/A — zero new packages (audit section above) |
| CI action tag hijack | Tampering | 40-char SHA pins (including actions/checkout — do not repeat r9-shell.yml:35's `@v4`) |
| Build-time source mutation | Tampering | nix/build.sh:64-68's sed-in-tree edits are a known CONCERNS entry; this phase adds no new build-time mutations (tier files are read-only data) |
| Foreign-process pkill sweeps | DoS (cross-run) | New expect drivers follow smoke-r9.sh's R9_RUN_TAG per-run-keyed pkill discipline (CONCERNS WR-07 fix shape); never kernel-name-class sweeps |
| Stale-artifact false-green | Spoofing (evidence) | D-07 fresh-archive + sha256 ledger + od magic asserts; mk exit status never trusted |

## Sources

### Primary (HIGH confidence) — all read this session
- `sys/src/mkfile`, `sys/src/mkfile.proto`, `sys/src/cmd/mkfile`, `mkone`/`mkmany`/`mksyslib` (read in full)
- `sys/src/cmd/cc/macbody:851-868`, `sys/src/cmd/6l/obj.c:1289-1310`, `sys/src/cmd/7l/unixcompat.c:1-202`, `sys/src/cmd/cc/cc.h:25`, `sys/src/cmd/6l/l.h:121`, `sys/src/cmd/cc/compat` (verbatim quotes)
- `aarch64/include/u.h:1-77`, `amd64/include/u.h:44-59` (verbatim FPdbleword)
- `sys/src/cmd/jpg/mkfile`, `awk/mkfile`, `jtagfs/mkfile`, `spell/mkfile`, `gs/mkfile:135-160`, `usb/usbd/mkfile`, `upas/bayes/mkfile`, `4a/mkfile`, `games/mkfile`, `games/mahjongg/mkfile`, `ip/mkfile` (full reads), plus tree-wide generator greps (`<\|`, `./$cputype`, `$O.` execution)
- `build/scripts/linux-build` (build_aarch64 verbatim), `nix/build.sh`, `sys/src/BUILD`, `RUN_BUILD_IN_PRIVATE_NAMESPACE`, `BUILD_SETUP_NAMESPACE`, `build/aarch64/nsrun.sh` (namespace bind sets)
- `build/aarch64/subset-build.sh`, `subset-exec.sh`, `mkimage.sh`, `smoke-r9.expect`, `nix/smoke-r9.sh`, `flake.nix`, `.github/workflows/r9-shell.yml`, `build/mm/boot-9k10cpu.{sh,mm}`, `build/aarch64/README.md`, `trace-termrc`, `trace-sweep.rc`, `trace-boot.rc`
- `sys/src/9k/boot/boot.c:64-139`, `sys/src/9k/k10/mkfile:22-27`
- lib dirs: 35 mkfiles grepped; libdynld, libmach, libaml, libthread, libdraw read in full; consumer greps (`pragma lib`, header includes)
- `.planning/REQUIREMENTS.md`, `ROADMAP.md`, `06-CONTEXT.md`, `research/PITFALLS.md`, `research/SUMMARY.md`, `codebase/CONCERNS.md`, v1.0 `01-01-SUMMARY.md`, `02 deferred-items.md`, `05-03-SUMMARY.md` (sourced-sweep finding), `05-CONTEXT.md`
- git: `7be7e98c8` (jpg fix), `f90918777` (jtagfs), `5c78cae6e` (spell), `76c55c91a` (efiboot.fat), `7efff3aa3` (pswt), `d6405e5fb` (libdynld import), flake.lock parse, `ls` of `aarch64/{bin,lib}`, `amd64/bin/ip`, `.build/*`

### Secondary (MEDIUM confidence)
- GitHub `/dev/kvm` on ubuntu-latest [CITED: github.blog/changelog/2024-04-02-github-actions-hardware-accelerated-android-virtualization-now-available; github.com/randax/talos-box/issues/74 "still confirmed working on ubuntu-latest in July 2026"]
- 9front donor `/bin` baseline: verified against `.build/9front-donor/sys/src/cmd` listing (primary, in-tree) — the *served* root's arm64/bin is the minimal 20-entry set (`.build/9front-dist/srvroot/arm64/bin` listing this session)

### Tertiary (LOW confidence)
- None load-bearing. The one uncertain mechanic (writable-prefix exec, A3) is flagged as plan-time verification with a documented fallback.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero external packages; every tool verified in-tree
- Architecture: HIGH — every hook point read at line level; the two genuine uncertainties (mk double-rule semantics A1, writable-prefix exec A3) are isolated, flagged, and have plan-time spikes/fallbacks
- Pitfalls: HIGH — milestone bundle pitfalls + 7 new phase-specific mechanics pitfalls, each with in-tree evidence

**Research date:** 2026-11-04
**Valid until:** 2026-12-04 (30 days — all in-tree mechanics, stable; re-verify only the GitHub /dev/kvm claim at workflow-authoring time)
