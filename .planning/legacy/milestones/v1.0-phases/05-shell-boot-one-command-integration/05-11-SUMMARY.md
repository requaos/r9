---
phase: 05-shell-boot-one-command-integration
plan: 11
subsystem: infra
tags: [r9, nix, flake, inputs.r9, flake.lock, boot-r9, compose-gate, fdtput, qemu-virt, INT-01, INT-04, D-10, D-11, D-13]

# Dependency graph
requires:
  - phase: 05-shell-boot-one-command-integration (05-10)
    provides: r9 packages.kernel-virt (kernel-virt.elf + aarch64/lib/virt-gicv2.dtb store-path contract) and the R9_HARVEY_DIR xtask inversion — Harvey consumes store paths only
  - phase: 05-shell-boot-one-command-integration (05-02..05-09)
    provides: subset-build.sh / mkimage.sh (build+bake), run-r9.sh's argv/DTB mechanics, and the drive-r9.expect-proven real boot chain the gate's markers come from
provides:
  - "inputs.r9 (github:requaos/r9) pinned in flake.lock to 9549cee — r9 main's HEAD with ALL phase r9-side work (05-01 + 05-04..05-10), pushed to origin this session"
  - "nix run .#boot-r9 — THE one command (INT-01): userland build (lima on darwin, native unshare on linux) → mkimage.sh bake → fdtput two-cell /chosen DTB patch → qemu -nographic with the flake-input kernel to the interactive % prompt"
  - "nix/boot-r9.sh with the R9_KERNEL_PKG store-path contract (T-05-21) + R9_SKIP_BUILD=1 iteration hatch + the D-13 --override-input dev flow documented in the header"
  - "subset-build.sh's additive native-linux leg (nsrun.sh sudo-unshare) — one driver on both hosts"
  - "05-12's wiring point: the boot-r9-smoke app slot in flake.nix"
affects: [05-12 (the smoke wraps this app), 05-13 (CI inherits the linux leg; darwin clean-clone reproduces this command), INT-03 (this command is the documented reproduction), verify-work]

# Actuals (#2632) — pairs with the plan's estimate (90000 tokens, confidence: low).
actuals:
  tokens: 47500    # MEASURED chars/4 over the realized diff: authored 22374 chars (T1 2819 + T2 14428 + T3 code 5127) + this SUMMARY ~15500 + the compose-gate evidence capture 144664 chars = ~190000/4 ≈ 47500.  Authored-only ≈ 9400 — the estimate overshot ~2x even counting the 145KB boot-log capture (confidence was low; the only surprises were the mktemp/read-only-DTB gate catches and the IFD eval boundary)
  tasks: 3
  commits: 3       # MEASURED: git rev-list --count 49145f3a0..862ea0403 = 3 (harvey; the docs commit rides on top).  r9: 0 new commits — the 80 phase commits were PUSHED to origin/main (33a9273..9549cee), not created
plan_head_before: {harvey: "49145f3a0559a06d9405042867a7e90ea4162cae", r9: "9549cee63d86a0c6d9b4d04ed8860b2efabcd3e6 (unchanged — push-only this plan)"}
plan_head_after: {harvey: "862ea0403 (pre-metadata; the docs commit rides on top)", r9: "9549cee63d86a0c6d9b4d04ed8860b2efabcd3e6"}

# Tech tracking
tech-stack:
  added: []        # zero new dependencies (T-05-SC held): the one new flake input is the project's own fork; the compose script drives existing in-repo machinery
  patterns:
    - "flake-input kernel consumption: the app exports R9_KERNEL_PKG from r9.packages.<system>.kernel-virt; the script takes kernel/DTB ONLY from that read-only store path (find-by-name fallback under the package root), never a sibling-checkout path — INT-04's letter"
    - "cp-from-store needs chmod u+w before in-place patching (nix/setup.sh's install_tool precedent) — store files are r--r--r-- and cp duplicates the mode; fdtput EPERM otherwise"
    - "one host driver, two transports: subset-build.sh gained a native-linux leg (nsrun.sh sudo-unshare, artifacts land in place via the binds, no rsync-back) so the compose script is platform-blind — the flake app's runtimeInputs carry the split"
    - "per-system app gating with optionalAttrs (builtins.hasAttr system r9.packages) — an input that ships no package for a system must not break that system's evaluation"

key-files:
  created:
    - nix/boot-r9.sh
    - .planning/phases/05-shell-boot-one-command-integration/05-11-compose-gate.log
  modified:
    - flake.nix
    - flake.lock
    - .gitignore
    - build/aarch64/subset-build.sh
    - .planning/phases/05-shell-boot-one-command-integration/deferred-items.md

key-decisions:
  - "r9 main pushed to origin BEFORE locking (33a9273..9549cee, 80 commits — all of 05-01 + 05-04..05-10) so the pin resolves from a clean clone (INT-03's leg); the lock gate then proved locked rev == live HEAD and owner=requaos"
  - "subset-build.sh owns BOTH transports (the plan's 'linux → the same script natively (unshare)' was impossible as shipped — it hard-required limactl): additive NATIVE_LINUX leg, darwin/lima path byte-identical, host toolchain auto-bootstrap mirrors vm-run.sh's guest-side ensure"
  - "qemu runs as a FOREGROUND CHILD, not exec (the plan's parenthetical): identical console semantics (stdio inherited, same process group) but the EXIT trap reclaims the mktemp DTB dir — D-16 hygiene over the literal word"
  - "pkgs.git added to the app's runtimeInputs (beyond the plan's mirror-the-build-app list): the mandated KERNDATE derivation silently no-ops to build-time without git (Rule 2 — the provenance header is load-bearing for INT-03)"
  - "boot-r9 exists only where r9 ships kernel-virt (hasAttr gate): r9 has no x86_64-darwin system and that host tuple is out of project scope (PROJECT.md) — an unguarded reference broke nix flake show for all systems"
  - "x86_64-linux full program eval deferred to 05-13's native CI host: r9's kernel-virt embeds genuine IFD (importCargoLock reads a Cargo.lock INSIDE the built rust toolchain), so evaluating a foreign-system package builds a foreign-system toolchain — platform mismatch on every host here; aarch64-linux was proven natively in the lima guest instead"

patterns-established:
  - "compose-app contract: store-path env var + presence validation + self-describing argv echo (captured gate logs read themselves) + foreground qemu with trap cleanup"

requirements-completed: [INT-01, INT-04]  # shared-ID gate (#2388): INT-03 is also declared by 05-13 (no SUMMARY yet) — requirements.ready-ids returns INT-01/INT-04 ready, INT-03 blocked until 05-13 lands

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "inputs.r9 declared (github:requaos/r9) and flake.lock pins the exact rev 9549cee63d86a0c6d9b4d04ed8860b2efabcd3e6 — equal to the live r9 main HEAD containing all phase r9-side work (pushed to origin this session); nix flake metadata resolves owner=requaos repo=r9"
    requirement: INT-04
    verification:
      - kind: integration
        ref: "python3 assert lock.nodes.r9.locked.rev == git -C ../r9 log -1 --format=%H (both 9549cee…); nix flake metadata --json → {'owner':'requaos','repo':'r9','rev':'9549cee…','narHash':'sha256-g9OBd1…'}"
        status: pass
    human_judgment: false
  - id: D2
    description: "nix run .#boot-r9 is THE one command on darwin: the compose gate log (05-11-compose-gate.log) shows the full boot — 'r9 from the Internet' banner → paqfs... → version… → init: starting /bin/rc → 'stacksmoke: depth 64 ok, checksum 2088960' → the %  prompt — with the kernel from the flake-input store path (the self-describing argv line names it) and the image from Harvey's own bake (1016221 bytes, paqfs magics asserted)"
    requirement: INT-01
    verification:
      - kind: e2e
        ref: "timeout 1200 sh -c 'nix run .#boot-r9 </dev/null 2>&1 | tee build/aarch64/compose-gate.log' + marker greps — banner/paqfs/stacksmoke/%  ALL PASS; pgrep qemu orphan check PASS; evidence committed as .planning/phases/05-shell-boot-one-command-integration/05-11-compose-gate.log"
        status: pass
    human_judgment: false
  - id: D3
    description: "the compose script contract: sh -n clean; carries R9_KERNEL_PKG / fdtput / loader,file= / 0x48000000; ZERO parent-relative sibling-checkout references in code lines; the D-13 --override-input dev flow and R9_SKIP_BUILD=1 hatch documented in the header; shellcheck clean under writeShellApplication's checkPhase"
    requirement: INT-04
    verification:
      - kind: unit
        ref: "sh -n nix/boot-r9.sh + the four contract greps + 'grep -vE '^\\s*#' | grep -c ../r9' == 0; nix eval .#apps.aarch64-darwin.boot-r9.program realised (shellcheck checkPhase ran)"
        status: pass
    human_judgment: false
  - id: D4
    description: "the app evaluates on darwin AND linux so 05-13's CI inherits it: aarch64-linux full program eval PASS (run NATIVELY in the lima guest — the plan's literal command); existing apps (setup/toolchain/build/iso/default) + devshell unregressed on aarch64-darwin; x86_64-linux appset evaluates (.type = app)"
    requirement: INT-01
    verification:
      - kind: integration
        ref: "lima guest: nix eval .#apps.aarch64-linux.boot-r9.program → /nix/store/9irm726…-harvey-boot-r9/bin/harvey-boot-r9; host: all five pre-existing apps + devShells.default evaluate; nix eval .#apps.x86_64-linux.boot-r9.type → \"app\""
        status: pass
    human_judgment: false
  - id: D5
    description: "x86_64-linux FULL program eval (the plan's second nix eval leg): blocked host-side by r9's rust-toolchain IFD platform boundary — evaluating a foreign-system kernel-virt builds a foreign-system toolchain (platform mismatch on aarch64 hosts); the identical expression evaluates on aarch64-linux, and 05-13's ubuntu-latest CI runs it natively"
    verification:
      - kind: other
        ref: "nix eval .#apps.x86_64-linux.boot-r9.program → 'Cannot build … platform mismatch' (rust-overlay FOD fetches tagged x86_64-linux); mechanism root-caused to importCargoLock reading ${toolchain}/…/Cargo.lock (IFD)"
        status: unknown
    human_judgment: true
    rationale: "host-capability boundary, not an app defect — but the letter of the plan's verify leg has not run green anywhere; 05-13's CI (native x86_64-linux) is the first environment that can execute it, and this entry keeps that visible instead of silently claiming it"

# Metrics
duration: 25 min
completed: 2026-11-03
status: complete
---

# Phase 5 Plan 11: Harvey's flake owns the composition — inputs.r9 + the boot-r9 one command Summary

**Harvey's flake now consumes r9 as a proper flake input — `inputs.r9 = github:requaos/r9` locked to `9549cee` (r9 main's phase-complete HEAD, pushed to origin this session) — and `nix run .#boot-r9` is INT-01's one command: it builds the aarch64 userland (lima namespace build on darwin; a new additive native-unshare leg in subset-build.sh for linux), bakes bootfs.paq with Harvey's own mkpaqfs, fdtput-patches a writable copy of the r9 package's DTB, and launches the flake-input kernel to the interactive `% ` prompt — the compose gate log shows the full marker sequence (banner → paqfs... → init → stacksmoke → `% `), with zero sibling-checkout paths anywhere in the shipped composition.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-11-03T15:28:35Z
- **Completed:** 2026-11-03T15:53Z
- **Tasks:** 3
- **Files modified:** 5 (created: nix/boot-r9.sh + the evidence log; modified: flake.nix, flake.lock, .gitignore, build/aarch64/subset-build.sh, deferred-items.md)

## Accomplishments

- **Task 1 — the pin (INT-04):** pushed r9 main to origin FIRST (80 commits, `33a9273..9549cee` — the phase's full r9-side work, so the pin resolves from a clean clone per INT-03), then declared `inputs.r9.url = "github:requaos/r9"` and `nix flake lock`ed it: locked rev `9549cee63d86a0c6d9b4d04ed8860b2efabcd3e6` == live r9 HEAD (python assert gate), and `nix flake metadata` resolves the node to owner `requaos` (narHash `sha256-g9OBd1…`). The D-13 dev override (`--override-input r9 git+file:///path/to/r9`) is documented in the flake comment — dev-only, never the default.
- **Task 2 — the compose script:** `nix/boot-r9.sh` orchestrates build → bake → DTB patch → launch, each step loud on failure: KERNDATE provenance (BUILD-04 discipline, env-overridable), `R9_SKIP_BUILD=1` iteration hatch (non-default, documented), kernel/DTB resolved ONLY from the `R9_KERNEL_PKG` store path with presence validation (T-05-21) and a find-by-name fallback, the Phase-4 two-cell fdtput `/chosen` patch at 0x48000000 (mirroring r9's config.rs:385-434, cited in the header), and the locked virt argv (`-M virt,gic-version=2 -cpu cortex-a72 -smp 1 -m 1024M -accel tcg -nographic -kernel … -dtb … -device loader,file=…,addr=0x48000000`) with a self-describing argv echo. The compose gate caught two real bugs (mktemp template typo; read-only store-copy EPERM) — both fixed, see Deviations.
- **Task 3 — the app + the COMPOSE GATE PASS:** `apps.boot-r9` (mkScriptApp `harvey-boot-r9`) wired with the platform-split runtimeInputs (qemu+dtc+git always; lima/openssh/rsync darwin; util-linux/gnused/mount/binutils linux), gated by `(builtins.hasAttr system r9.packages)` so x86_64-darwin (no r9 package; out of project scope) can't break evaluation. The scripted darwin gate ran end-to-end: `nix run .#boot-r9 </dev/null | tee compose-gate.log` built the userland via lima, baked the image, and booted the **flake-input kernel** to `% ` — all four staged markers asserted, qemu killed by process sweep, **zero orphans**. Evidence committed: `05-11-compose-gate.log` (the argv line self-describes: `-kernel /nix/store/ylgcqwi…-r9-kernel-virt-9549cee/kernel-virt.elf`).
- **Eval matrix:** aarch64-linux full program eval PASS — the plan's literal `nix eval .#apps.aarch64-linux.boot-r9.program` run **natively in the lima guest** (→ `/nix/store/9irm726i…-harvey-boot-r9/bin/harvey-boot-r9`); x86_64-linux `.type` = "app" (full program eval deferred to 05-13's native CI — the IFD boundary, D5 above); existing apps + devshell evaluate unregressed.

## Task Commits

Each task was committed atomically (single-repo this plan — harvey-side; r9 was push-only):

1. **Task 1: inputs.r9 + flake.lock pin (INT-04)** — `58d1b1077` (feat; flake.nix + flake.lock)
2. **Task 2: nix/boot-r9.sh — the compose script** — `73072d4f7` (feat; nix/boot-r9.sh + build/aarch64/subset-build.sh)
3. **Task 3: the boot-r9 app wired + compose gate PASS on darwin** — `862ea0403` (feat; flake.nix + .gitignore + deferred-items.md + 05-11-compose-gate.log)

**Plan metadata:** (the docs commit follows — SUMMARY + STATE + ROADMAP + REQUIREMENTS)

## Files Created/Modified

- `flake.nix` — inputs.r9 + the bootR9Pkg/boot-r9 app (+ hasAttr gate, 05-12 wiring comment)
- `flake.lock` — the r9 node (rev 9549cee, narHash) + r9's nixpkgs/rust-overlay follows
- `nix/boot-r9.sh` — NEW: the compose pipeline (the R9_KERNEL_PKG contract; 243 lines incl. the header documentation)
- `build/aarch64/subset-build.sh` — additive NATIVE_LINUX leg (nsrun.sh sudo-unshare; toolchain auto-bootstrap; rsync-back skipped natively); darwin leg byte-identical
- `.gitignore` — `/build/aarch64/compose-gate.log` (generated gate capture)
- `.planning/phases/05-shell-boot-one-command-integration/05-11-compose-gate.log` — the committed gate evidence (2652 lines)
- `.planning/phases/05-shell-boot-one-command-integration/deferred-items.md` — the pre-existing x86_64-darwin eval break (proven on the pre-05-11 flake)

## Decisions Made

- **Push-before-pin** — the lock gate's "locked rev == live HEAD" is only half the INT-03 contract; the rev must also resolve from a clean clone, so the 80 phase commits went to origin/main first (recorded here per the plan's mandate). Evidence: `git ls-remote origin main` = `9549cee…` post-push.
- **subset-build.sh owns both transports** — see key-decisions; the alternative (a duplicated guest-script in boot-r9.sh) was rejected as a drift hazard.
- **Foreground child over exec** — see key-decisions.
- **git in runtimeInputs** — see key-decisions.
- **hasAttr gating** — see key-decisions.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] mktemp template carried a stray `}` (my Write typo)**
- **Found during:** Task 3 compose gate, first attempt
- **Issue:** `mktemp -d "${TMPDIR:-/tmp}/boot-r9.XXXXXX}"` — a literal `}` inside the quoted template produced dirs like `/tmp/boot-r9.MBf9RM}`. Syntactically valid (sh -n and shellcheck both passed); functionally harmless alone, but wrong.
- **Fix:** removed the brace.
- **Files modified:** nix/boot-r9.sh
- **Verification:** standalone reproduction creates a clean `boot-r9.XXXXXX` dir; the re-run gate passed
- **Committed in:** `73072d4f7` (amended)

**2. [Rule 1 - Bug] DTB copy was read-only — fdtput EPERM**
- **Found during:** Task 3 compose gate, first attempt (`Couldn't write blob to '…/virt-patched.dtb': Permission denied`)
- **Issue:** `cp` from the nix store duplicates the source's `r--r--r--` mode (POSIX cp duplicates permission bits when creating); fdtput opens the copy O_RDWR → EPERM. run-r9.sh never hits this because its R9_DTB comes from a checkout (0644), not the store — the flake-input path introduces the read-only source.
- **Fix:** `chmod u+w "$DTB"` after the copy (nix/setup.sh's `install_tool` precedent for store-derived files), with a comment.
- **Files modified:** nix/boot-r9.sh
- **Verification:** standalone DTB patch + fdtget round-trip: initrd-start = 0x48000000, initrd-end = 0x480f819d (= +1016221 = the baked image size); the re-run gate passed
- **Committed in:** `73072d4f7` (amended)

**3. [Rule 3 - Blocking] subset-build.sh had no native-linux leg**
- **Found during:** Task 2 authoring — the plan mandates "linux → the same script natively (unshare)", but subset-build.sh hard-required limactl (darwin-only).
- **Issue:** the compose script's linux half was unimplementable as the shipped drivers stood.
- **Fix:** additive NATIVE_LINUX leg in subset-build.sh — host toolchain auto-bootstrap (vm-run.sh's guest-side ensure, host-side), HARVEY/HARVEY_LINUX_BIN + the devshell env vars exported for nsrun.sh's capture, the SAME generated guest script run under `nsrun.sh --wrap` (sudo unshare), rsync-back skipped (the namespace binds the real checkout — artifacts land in place), host-side asserts shared. The darwin/lima leg is byte-identical (proven: the first gate attempt ran it green end-to-end).
- **Files modified:** build/aarch64/subset-build.sh
- **Verification:** sh -n clean; the darwin leg ran green in both gate attempts (SUBSET-BUILD-OK, magics asserted); the linux leg is correct-by-construction and runs in 05-13's CI
- **Committed in:** `73072d4f7`

**4. [Rule 2 - Missing Critical] pkgs.git in the app's runtimeInputs**
- **Found during:** Task 3 authoring
- **Issue:** the plan's mandated KERNDATE provenance step derives from `git log`; the build app's runtimeInputs (the plan's mirror list) carry no git, so the derivation would silently degrade to build-time (non-reproducible) inside the app.
- **Fix:** added pkgs.git (always) alongside qemu/dtc.
- **Files modified:** flake.nix
- **Verification:** the gate log's build ran with the git-derived KERNDATE path available (no fallback warnings)
- **Committed in:** `862ea0403`

**5. [Rule 3 - Blocking] verify-command adjustments for THIS host (two flag/venue spellings; assertions unchanged)**
- **Found during:** Task 1 + Task 3 verify legs
- **Issue:** (a) `nix flake metadata --format json` — this nix (Determinate Nix 2.34.6) rejects `--format`; the accepted spelling is `--json`. (b) `nix eval .#apps.{aarch64,x86_64}-linux.boot-r9.program` from darwin realises the program string's store-path context, and r9's kernel-virt embeds genuine IFD (importCargoLock reads a Cargo.lock INSIDE the built rust toolchain) — evaluating a foreign-system package therefore builds a foreign-system toolchain: platform mismatch on every host here.
- **Fix:** (a) ran the metadata gate with `--json` — same node, same assertion (owner=requaos PASS). (b) ran the aarch64-linux leg NATIVELY in the lima guest (the plan's literal command — PASS, program path printed) and the x86_64-linux leg as `.type` = "app" + full-eval deferral to 05-13's native x86_64-linux CI (coverage D5 keeps it visible, human_judgment: true).
- **Files modified:** none (verify-venue only)
- **Verification:** as above
- **Committed in:** n/a

**6. [Rule 1 - Tooling] shellcheck 0.11 fails the app build on info diagnostics**
- **Found during:** pre-flight shellcheck of nix/boot-r9.sh (SC2329 info on the trap-invoked `cleanup` function — writeShellApplication's checkPhase runs shellcheck and this version exits nonzero on info)
- **Fix:** `# shellcheck disable=SC2329` with a reason comment; the nix checkPhase then passed (the app built: `/nix/store/mm16nvq4…-harvey-boot-r9`)
- **Files modified:** nix/boot-r9.sh
- **Committed in:** `73072d4f7` (amended)

---

**Total deviations:** 6 auto-fixed (3 Rule 1, 1 Rule 2, 2 Rule 3)
**Impact on plan:** Every deviation is inside the plan's intent — two are gate-caught bugs in my own new script (exactly what the compose gate exists to catch), one closes a driver gap the plan's language assumed, two are verify-command venue adjustments with the assertions preserved. No scope creep; no silent fallbacks added anywhere (the D-01-fidelity prohibition held — a broken chain fails loudly at the named step).

## Issues Encountered

- The first compose-gate attempt failed at the DTB step (deviations 1+2) after a fully green build+bake — fixed and re-run to a full marker pass. One background gate runner was SIGTERMed mid-poll after the known-failed first attempt (it would otherwise have slept out its 1100s window); no state entanglement.
- Pre-existing, logged to deferred-items.md (NOT caused by this plan): the nixpkgs-pin-era x86_64-darwin eval throw — proven by evaluating the pre-05-11 flake (`49145f3a0`), whose setup app fails identically.

## Authentication Gates

None. (The r9 push used the existing GitHub SSH credentials — no gate triggered.)

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. The composition is fully wired: flake input → store paths → compose script → qemu. (R9_SKIP_BUILD=1 is a documented, non-default iteration hatch — not a stub; the default path always builds.)

## Threat Flags

None beyond the plan's register. T-05-20 held (exact-rev + narHash pin; the dev override is documented dev-only); T-05-21 held (kernel/DTB only from the nix-controlled read-only store path, presence validated before launch, no caller-supplied path exec'd); T-05-SC held (zero package installs — the one new flake input is the project's own fork).

## Next Phase Readiness

- **05-12 (the smoke app):** inherits the wired `boot-r9` app and its wiring point comment in flake.nix; the compose gate's marker sequence (banner → paqfs... → init → stacksmoke → `% `) is the proven baseline the expect sweep extends. The kernel arrives via R9_KERNEL_PKG there too.
- **05-13 (CI + clean checkouts):** inherits (a) the native-linux subset-build leg (unexecuted here — CI is its first run venue), (b) the aarch64-linux program-eval proof (native, this session), (c) the x86_64-linux full-eval deferral (coverage D5 — CI ubuntu-latest runs it natively), and (d) a pin whose rev is on origin (clean-clone resolvable).
- r9 HEAD: `9549cee` (main, pushed). Harvey HEAD: `862ea0403` + the metadata commit.

---
*Phase: 05-shell-boot-one-command-integration*
*Completed: 2026-11-03*

## Self-Check: PASSED

- Files: `nix/boot-r9.sh`, `flake.nix`, `flake.lock`, `05-11-compose-gate.log`, this SUMMARY — all FOUND on disk.
- Commits: `58d1b1077` (Task 1), `73072d4f7` (Task 2), `862ea0403` (Task 3) — all FOUND in `git log --oneline --all`. r9: no new commits this plan; the push `33a9273..9549cee` verified via `git ls-remote origin main` = `9549cee…` before locking.
- Plan-level <verification> re-run at final state: (1) pin gate — locked rev == live r9 HEAD (9549cee…), resolves to owner requaos — PASS; (2) script syntax + contract greps + zero `../r9` in code lines — PASS; (3) compose gate — the committed log carries all four staged markers (banner 1738 → paqfs 2471 → init 2543 → stacksmoke 2609 → `% `), no qemu orphans — PASS; (4) eval matrix — aarch64-linux program eval native PASS (lima guest), x86_64-linux .type=app (full eval deferred to 05-13 CI, coverage D5), existing apps + devshell unregressed — PASS.
- Acceptance criteria re-run at final state: Task 1 (input declared; lock == pushed live HEAD; resolves to the fork) — PASS. Task 2 (pipeline loud per step; KERNDATE header; store-path-only kernel/DTB; dev override documented) — PASS. Task 3 (one command to `% ` on darwin from the flake-input kernel + Harvey-baked image; evaluates on darwin + both linux systems; no orphans; fresh log per run) — PASS.
