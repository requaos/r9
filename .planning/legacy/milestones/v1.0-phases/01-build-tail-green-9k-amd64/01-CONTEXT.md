# Phase 1: Build-Tail Green (9k amd64) - Context

**Gathered:** 2026-10-30
**Status:** Ready for planning

<domain>
## Phase Boundary

Repair Harvey's broken upstream build chain so `nix run .#build` from a clean checkout completes the full namespace build and produces all three 9k amd64 kernels (`k10cpu`, `k10cpuf`, `k10f`); ensure userland installs into `/amd64/bin` (ipconfig, factotum, venti); quarantine `arm64` vestiges in favor of project `aarch64` naming; pin a reproducibility (KERNDATE) policy; and stand up the minimega smoke harness that boots a built kernel on a Linux KVM host.

NOT in this phase: 386 legacy kernels, ISO/dist stage, aarch64 toolchain imports (Phase 2), r9 work (Phases 3–4).

</domain>

<decisions>
## Implementation Decisions

### Fix depth + loop resilience
- **D-01:** Fix-depth target is **fix everything** — every component of the default cmd build must compile: `$PROGS` (single-file cmds) + `cc` + all `DIRS` subdirs in `sys/src/cmd/mkfile` (minus its NOMK exclusions: `sml|dup|.+\..+|unix|old`). This includes ghostscript, games/mahjongg, cifs, aquarela. — **Reversibility:** costly — the failure tail is unknown length; changing course later means re-scoping already-landed fix work.
- **D-02:** Vendored snapshots (ghostscript, antiword, apeg) get **patched in place** like any other code. No upstream-compat preservation — this fork owns them. **BUT — toolchain-first diagnostic rule:** this vendored code built fine for decades, so a fresh build failure is *probably our toolchain* (cpp/6c/pcc) having drifted or being buggy — cf. mahjongg, which breaks because Harvey's own cpp is buggy (Dockerfile TODO). When vendored code fails, first check whether our compiler/preprocessor is at fault (e.g. preprocess the same file with a reference cpp, compare against a known-good toolchain run); fix toolchain bugs in the toolchain. Patch the vendored source only when the vendored code itself is genuinely at fault.
- **D-03:** mk DIRS loops keep **strict abort-on-first-failure** semantics. Add a separate report-all diagnostic target (e.g. `mk checkall`) that runs every dir and prints the full failure list — visibility without weakening the default gate.
- **D-04:** Sequencing is **build-order whack-a-mole**: run the build, fix the current blocker, rerun. No upfront failure inventory; unknown tail length accepted. The decoded chain from research (`jpg/mkfile` → cmd loop abort → missing `ipconfig`/`venti`) is the starting point, not a pre-planned work order.

### aarch64 quarantine mechanics
- **D-05:** Vestige handling is **rename + remove**: rename `build_arm64` → `build_aarch64` in `build/scripts/funcs` and `build/scripts/linux-build` (fixing their `objtype=aarch64` + `cmd/7?` references so they're correct-but-dormant until Phase 2), drop the `arm64/bbin` entry from `build/mkdirs`, remove the empty root `arm64/` dir if present.
- **D-06:** Donor import naming (Phase 2 rule, locked now): **rename on import** — 9front's `arm64` paths normalize to `aarch64` (`libc/arm64` → `libc/aarch64`, `arm64/mkfile` → `aarch64/mkfile`, `objtype=aarch64`) via a repeatable normalization step; donor file contents otherwise untouched. — **Reversibility:** one-way — undoing it means re-normalizing every imported donor path and re-teaching the mk/objtype machinery which name it keys on (`parse` keys MACH off `$objtype`).

### the agent's Discretion
- **Reproducibility policy (BUILD-04)** — user took the recommended default: pinned timestamps (SOURCE_DATE_EPOCH-style, e.g. derived from the git commit) over build-time `date -n`; scope = kernels (kernel embeds KERNDATE). Document the policy in-repo; the planner may extend the policy to the full tree if cheap.
- **minimega smoke harness (BUILD-05)** — recommended defaults: boot `9k10cpu` via `vm config kernel`+`initrd` on a Linux KVM host; `.mm` scripts live in-repo (suggest `build/mm/` or `nix/`); assert boot via serial-log capture (no miniccc in-guest agent — Harvey isn't a Linux guest); CI wiring optional this phase.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` §Phase 1 — goal, success criteria, requirement mapping
- `.planning/REQUIREMENTS.md` — BUILD-01..05 full text + Out of Scope list
- `.planning/PROJECT.md` — Key Decisions (in-tree fixes, aarch64 naming, nix, minimega boundary)

### Research (decoded failure chain + donor map)
- `.planning/research/PITFALLS.md` — the decoded build-tail failure chain (jpg/mkfile `oc` derivation at `sys/src/cmd/jpg/mkfile:56-68`, 4a `rfork bind lex.c`), riscv64 vaporware rule, KERNDATE open question
- `.planning/research/SUMMARY.md` — phase shape and risk ordering
- `.planning/codebase/CONCERNS.md` — full upstream-breakage inventory (vendored gs/antiword, fragile areas, test gaps)

### Build machinery (the code being fixed)
- `sys/src/cmd/mkfile` — the cmd build driver (`$PROGS`, `DIRS`, NOMK exclusions, `cc $DIRS` loop)
- `build/scripts/linux-build` + `build/scripts/funcs` — arch build functions incl. the `build_arm64` vestige
- `build/mkdirs` — per-arch output dir creation (has the `arm64/bbin` entry)
- `nix/build.sh`, `nix/vm-build.sh`, `flake.nix` — the nix build path (`nix run .#build`) that must end green

### Harness
- `nix/minimega.nix` — the nix-packaged minimega derivation (BUILD-05 substrate)
- minimega docs: https://sandia-minimega.github.io/minimega/ (quickstart + `vm config`/`vm launch` API)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `sys/src/cmd/unix/9cc/` — the vendored-host-tool pattern (BUILD script + in-tree sources) established this session; new host-tool needs in this phase should follow it
- `nix/*.sh` + flake apps — `nix run .#build` is the verification harness; `.build/hostbin-<os>-aarch64/` holds the working Plan 9 toolchain (6a/6c/6l, cpp, pcc, iar)
- lima VM `harvey` (running) — the linux-aarch64 namespace-build environment on this mac

### Established Patterns
- mk strictness: `for(i in $DIRS)` loops abort on first failure (`mk: for(i in ...): exit status=exit(1)`) — the reason one broken cmd masks the rest
- Kernel root embedding: `sys/src/9k/mk/parse` + `mkrootall` read kernel configs (k10cpu/k10cpuf/k10f) and require userland binaries present at `/amd64/bin/...` — missing binaries = "don't know how to make" at kernel link time
- Host-tool BUILD scripts: `sys/src/cmd/*/BUILD` compile with plan9port `9c`/`9l` and `cp` into `$HARVEY_LINUX_BIN`

### Integration Points
- `sys/src/cmd/mkfile` — where a report-all target (`checkall`) lands
- `build/scripts/funcs` + `linux-build` — the `build_arm64` → `build_aarch64` rename sites
- `build/mkdirs` — the `arm64/bbin` removal site
- `flake.nix` — where a minimega boot app would register if added (BUILD-05)

</code_context>

<specifics>
## Specific Ideas

- The known starting blockers (from the decoded chain): `sys/src/cmd/jpg/mkfile:56-68` (derives nonexistent `oc` from `cputype=unix` — `unix/mkfile` sets `O=o`), `4a`'s `rfork bind lex.c` recipe failure, `cwfs`'s host-`size` invocation (already stubbed in nix setup; verify in-tree expectation)
- Kernels must come up in this order of unblocking: fix cmd chain → userland installs → k10cpu (no embedded root) stays green, k10cpuf/k10f (embedded root) go green once `/amd64/bin` is populated

</specifics>

<deferred>
## Deferred Ideas

- Attic/quarantine for skeletal ports (riscv64 vaporware tree, arm/mips/power fragments) beyond the `arm64` vestiges — broader cleanup, belongs to a later hygiene phase (per CONCERNS.md)
- TLS 1.3 / cipher pruning — separate hardening milestone
- 386 kernel + ISO/dist stage + expect/qemu ISO build — deferred per project scope
- minimega cluster/CI deployment — local KVM-host harness first this phase

</deferred>

---

*Phase: 1-Build-Tail Green (9k amd64)*
*Context gathered: 2026-10-30*
