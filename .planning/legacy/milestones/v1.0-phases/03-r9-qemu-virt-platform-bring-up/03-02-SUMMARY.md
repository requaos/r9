---
phase: 03-r9-qemu-virt-platform-bring-up
plan: "02"
subsystem: infra
tags: [rust, aarch64, r9, platform-tree, cfg-dispatch, check-cfg, trap, raspi3b, raspi4b]

requires:
  - phase: 03-r9-qemu-virt-platform-bring-up (plan 01)
    provides: "../r9 flake devshell — every build/clippy in this plan ran through `nix develop -c cargo xtask …`"
provides:
  - "../r9/aarch64/src/platform/ tree per D-01: cfg-gated mod.rs dispatch (virt arm compiles out until 03-04; raspi4b arm; raspi3b default via not(any(virt, raspi4b)) re-export fallback) with wildcard re-exports keeping main.rs platform-agnostic"
  - "raspi drivers moved to platform/raspi3b/{devcons,mailbox,uartmini,uartpl011}.rs (uartpl011.rs preserved as the raspi PL011 option); raspi4b/mod.rs is a glob re-export shim over raspi3b"
  - "[config] platform = \"raspi3b\" / \"raspi4b\" keys in the two existing config TOMLs (riscv64 ordering: between [link] and [qemu])"
  - "aarch64/Cargo.toml [lints.rust] unexpected_cfgs block (P10): platform values + predeclared dev_panic"
  - "D-03 gating: mailbox::init, print_board_info/print_pi_name, test_sysexit, and the user-page-table block compile only under cfg(not(platform = \"virt\"))"
  - "trap::unmask_irq() — VBAR setup (init) and IRQ unmask are now separately orderable (P5); raspi unmasks at the historical point in main9"
affects: [03-03, 03-04, 03-05, 03-06, phase-04-userspace-exec]

# Actuals (#2632) — same scale as the plan's estimateTokens (chars/4 over realized diff).
# Cross-repo plan: commits = 2 r9 code commits (65c737d, 6a309ae) + 1 harvey docs commit.
# r9 leg: ae4cd08 -> 6a309ae (diff 11,214 chars → 2,803 tokens). Harvey leg measured from ledger.
actuals:
  tokens: 2803
  tasks: 2
  commits: 3

# Commit ledger (#3968) — cross-repo plan: harvey base from the persisted ledger;
# the r9 leg is ae4cd08..6a309ae (2 commits). commits: 3 = 2 r9 + this harvey docs commit.
# plan_head_after names the pre-amend docs hash; the amend only added this ledger field.
plan_head_before: 68d20e843945e2492da98346fdb58eb54264d1fe
plan_head_after: 90f0c29f9cf6cbd9d9fedb50f3ca2f496604306e

tech-stack:
  added: []
  patterns:
    - "aarch64 platform dispatch mirrors riscv64 exactly: cfg-gated `pub mod` + wildcard re-export pairs, default platform via not(any(…)) fallback on the re-export arm"
    - "near-identical platforms share code via a glob re-export shim (raspi4b over raspi3b) instead of file duplication — divergence so far is data in config TOMLs"
    - "trap init split contract: VBAR setup in trap::init(), IRQ unmask in trap::unmask_irq() called only when the platform's handlers are armed (P5)"

key-files:
  created:
    - "../r9/aarch64/src/platform/mod.rs"
    - "../r9/aarch64/src/platform/raspi3b/mod.rs"
    - "../r9/aarch64/src/platform/raspi4b/mod.rs"
  modified:
    - "../r9/aarch64/src/main.rs"
    - "../r9/aarch64/src/trap.rs"
    - "../r9/aarch64/Cargo.toml"
    - "../r9/aarch64/lib/config_default.toml"
    - "../r9/aarch64/lib/config_raspi4b.toml"
    - "../r9/aarch64/src/platform/raspi3b/devcons.rs (moved + import fix)"
    - "../r9/aarch64/src/platform/raspi3b/mailbox.rs (moved + unused type params dropped)"
    - "../r9/aarch64/src/platform/raspi3b/uartmini.rs (moved, content 100% preserved)"
    - "../r9/aarch64/src/platform/raspi3b/uartpl011.rs (moved + import fix)"

key-decisions:
  - "raspi3b's pub-mod gate is any(test, not(platform = \"virt\")) — wider than the plan's literal text so the raspi4b shim's `pub use crate::platform::raspi3b::*` resolves under platform=\"raspi4b\"; the re-export arm keeps the plan's exact any(test, raspi3b, not(any(virt, raspi4b))) default fallback"
  - "raspi4b shim uses a glob re-export, not the plan's explicit 4-name list — the explicit list introduced a new unused-imports warning (uartmini/uartpl011), violating the plan's zero-new-warnings requirement"
  - "check-cfg declares cfg(platform, values(virt, raspi3b, raspi4b)) + cfg(dev_panic); the dynamic dev/ip/link/nodev/nouart family names are declare-on-first-use, documented in a Cargo.toml comment (check-cfg has no name wildcards; grep confirmed no code uses any family member today)"
  - "Mailbox::request<T,U> unused type parameters dropped (plan directs fixing pre-existing warnings in moved files; clippy 12 → 11, all remaining warnings pre-existing in files this plan never touched)"

requirements-completed: [R9-01]

coverage:
  - id: D1
    description: "D-01 platform tree: raspi drivers moved under platform/raspi3b/, cfg dispatch mod.rs with raspi3b default fallback, raspi4b re-export shim, explicit [config] platform keys in both TOMLs, check-cfg block — both raspi configs build green through the 03-01 devshell"
    requirement: R9-01
    verification:
      - kind: other
        ref: "static gate: new-path/old-path file existence, platform-key greps on both TOMLs, unexpected_cfgs + dev_panic greps on Cargo.toml — PASS"
        status: pass
      - kind: integration
        ref: "cd ../r9 && nix develop -c cargo xtask build --arch aarch64 → Finished, 0 warnings; --config raspi4b → Finished, 0 warnings"
        status: pass
    human_judgment: false
  - id: D2
    description: "D-03 raspi-only couplings (mailbox init, board info, test_sysexit, user-page-table block) gated cfg(not(platform = \"virt\")); trap::init VBAR-only with unmask_irq() split (P5), raspi unmasking at the historical point in main9; builds + clippy green"
    requirement: R9-01
    verification:
      - kind: other
        ref: "region-scoped awk: trap.rs init() body contains 0 DAIFClr; greps: pub fn unmask_irq present, main9 calls trap::unmask_irq() immediately after trap::init(), 9 not(virt) gate sites — PASS"
        status: pass
      - kind: integration
        ref: "cd ../r9 && nix develop -c cargo xtask build --arch aarch64 && … --config raspi4b && … clippy --arch aarch64 --config default → all exit 0; clippy 11 warnings, all pre-existing in untouched files"
        status: pass
    human_judgment: false

duration: 5 min
completed: 2026-11-01
status: complete
---

# Phase 03 Plan 02: aarch64 platform tree + trap init/unmask split Summary

**The aarch64 crate now has the riscv64-style platform tree (D-01): the four raspi drivers moved under `platform/raspi3b/`, a cfg-gated `platform/mod.rs` dispatch with raspi3b as the no-cfg default, a glob re-export shim for raspi4b, explicit `[config] platform` keys in both TOMLs, and check-cfg hygiene (P10) — plus D-03's `not(virt)` gating of every raspi-only main9 coupling and the P5 split of trap init (VBAR) from IRQ unmask (`unmask_irq()`), with both raspi configs building clean and zero newly-introduced warnings.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-11-01T22:37:46Z
- **Completed:** 2026-11-01T22:42:29Z
- **Tasks:** 2
- **Files modified:** 12 (all in sibling repo `../r9`; harvey side carries only this SUMMARY + state files per D-13)

## Accomplishments

- `platform/{mod.rs, raspi3b/, raspi4b/}` exists per D-01: `mod.rs` mirrors `riscv64/src/platform/mod.rs` with aarch64 platforms — virt arm gated on `platform = "virt"` (compiles out until 03-04 creates the module; no config sets it yet), raspi4b arm, and raspi3b as the default via `not(any(platform = "virt", platform = "raspi4b"))` on the re-export arm so bare `cargo xtask build --arch aarch64` keeps working.
- `git mv` moved `devcons.rs`, `mailbox.rs`, `uartmini.rs`, `uartpl011.rs` into `platform/raspi3b/` — mailbox/uartmini content 100% preserved (their `crate::` imports stay valid), devcons/uartpl011 got one-line sibling-import fixes (`super::uartmini`, `super::mailbox`). uartpl011.rs preserved per prohibition, compiling as the raspi PL011 option.
- Both config TOMLs declare their platform explicitly (`platform = "raspi3b"` / `"raspi4b"` between `[link]` and `[qemu]`, riscv64 ordering); `aarch64/Cargo.toml` gained the `[lints.rust] unexpected_cfgs` block declaring the platform values and predeclaring `dev_panic` for 03-06.
- D-03 gating: `platform::mailbox::init`, `print_board_info` (+ its `print_pi_name` helper), `test_sysexit` (definition and call, with a Phase-4 comment per research OQ-3), and the user-page-table block all compile only under `cfg(not(platform = "virt"))` — which preserves bare-build behavior via the raspi3b fallback. The kernel-alloc test, `print_stacks`/`print_binary_sections`/`print_memory_info`, vmdebug prints, and the `Box::new` line stay platform-agnostic per the plan.
- P5 split: `trap::init()` retains only the `adr`/`msr vbar_el1` asm (region-scoped awk proves zero `DAIFClr` in its body); new `trap::unmask_irq()` carries `msr DAIFClr, #2`. main9's raspi path unmasks immediately after `trap::init()` — the exact historical point — while virt deliberately does not unmask until 03-05 arms GIC + timer + UART RX handlers.
- Regression sweep through the 03-01 devshell: `cargo xtask build --arch aarch64` (default/raspi3b) and `--config raspi4b` both green with zero warnings; `cargo xtask clippy --arch aarch64 --config default` green with 11 warnings, all pre-existing in files this plan never touched (`pre_mmu/util.rs` ×5, `pre_mmu/vminit.rs` ×3, `pagealloc.rs` ×1, `vm.rs` ×2).

## Task Commits

Each task was committed atomically (repo: `../r9`, branch `main`):

1. **Task 1: Platform tree — move raspi drivers, cfg dispatch mod.rs, config platform keys, check-cfg** — `65c737d` (feat) — 11 files, +59/-13
2. **Task 2: main9 raspi gating (D-03) + trap init/unmask split (P5) + regression sweep** — `6a309ae` (feat) — 3 files, +38/-12

**Plan metadata:** harvey docs commit (see below; `docs(03-02): complete aarch64 platform tree plan`)

## Files Created/Modified

- `../r9/aarch64/src/platform/mod.rs` (created) — cfg-gated platform dispatch with raspi3b default fallback + wildcard re-exports
- `../r9/aarch64/src/platform/raspi3b/mod.rs` (created) — declares the four moved driver modules
- `../r9/aarch64/src/platform/raspi4b/mod.rs` (created) — glob re-export shim over raspi3b (divergence today is DTB + qemu machine, both data in TOMLs)
- `../r9/aarch64/src/platform/raspi3b/{devcons,mailbox,uartmini,uartpl011}.rs` (moved via git mv; devcons/uartpl011 one-line import fixes; mailbox also dropped unused `request<T,U>` type params)
- `../r9/aarch64/src/main.rs` (modified) — four moved mod declarations collapsed to `mod platform;`; call sites route through `platform::`; not(virt) gates on raspi couplings; unmask call after trap init
- `../r9/aarch64/src/trap.rs` (modified) — init() VBAR-only; new `unmask_irq()`
- `../r9/aarch64/Cargo.toml` (modified) — `[lints.rust] unexpected_cfgs` check-cfg block + family-policy comment
- `../r9/aarch64/lib/config_default.toml`, `../r9/aarch64/lib/config_raspi4b.toml` (modified) — `[config] platform` keys

(Harvey repo: only this SUMMARY + state files, per D-13.)

## Decisions Made

- **raspi3b module gate widened relative to the plan's literal text** — the plan's gate (`any(test, platform = "raspi3b", not(any(virt, raspi4b)))`) on `pub mod raspi3b` would exclude the raspi3b module under `platform = "raspi4b"`, breaking the raspi4b shim's import. The mod declaration uses `any(test, not(platform = "virt"))`; the re-export arm keeps the plan's exact fallback shape, which is where the default-fallback semantics live. (Deviation 1.)
- **raspi4b shim is a glob re-export** (`pub use crate::platform::raspi3b::*;`), not the plan's explicit four-name list — the explicit list compiled fine but introduced a NEW `unused_imports` warning (nothing references uartmini/uartpl011 through `platform::`), violating the plan's zero-new-warnings requirement. Identical semantics, zero warnings. (Deviation 2.)
- **check-cfg family policy:** `cfg(platform, values("virt", "raspi3b", "raspi4b"))` and `cfg(dev_panic)` are declared; the `dev_*`/`ip_*`/`link_*`/`nodev_*`/`nouart_*` families derive cfg names dynamically from TOML content and check-cfg has no name-wildcard syntax, so each family member is declared when code first uses it — documented in a Cargo.toml comment. Grep confirmed no code in the aarch64 or port crates uses any family member today, so no warnings were possible either way.
- **`Mailbox::request<T, U>` lost its unused type parameters** — the method body never used them (only the turbofish at the call site referenced them); the free `request<T, U>` function retains its real generics. The plan directs fixing pre-existing warnings in moved files rather than noting them.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in plan's literal shape] raspi3b `pub mod` gate widened so the raspi4b shim resolves**
- **Found during:** Task 1 (design-time check before the raspi4b build)
- **Issue:** the plan prescribes `cfg(any(test, platform = "raspi3b", not(any(platform = "virt", platform = "raspi4b"))))` for the raspi3b arm, but `platform/raspi4b/mod.rs` re-exports from `crate::platform::raspi3b` — under `platform = "raspi4b"` the plan's gate excludes the raspi3b module entirely, so the shim could not compile
- **Fix:** `pub mod raspi3b` uses `any(test, not(platform = "virt"))` (the shim needs raspi3b present under raspi4b); the `pub use …::*` re-export arm keeps the plan's exact gate unchanged, preserving the raspi3b-default-fallback semantics and preventing glob ambiguity when raspi4b's re-export is active
- **Files modified:** ../r9/aarch64/src/platform/mod.rs
- **Verification:** raspi4b build green; bare default build green; the `not(any(virt, raspi4b))` fallback string present in mod.rs per acceptance criterion 1
- **Committed in:** `65c737d`

**2. [Rule 1 - Bug] raspi4b shim explicit import list introduced a new warning**
- **Found during:** Task 1, verify gate 2 (`--config raspi4b` build)
- **Issue:** the plan's shim text (`pub use crate::platform::raspi3b::{devcons, mailbox, uartmini, uartpl011};`) compiled but emitted a NEW `unused_imports` warning for uartmini/uartpl011 (nothing names them through `platform::`); the plan requires zero newly-introduced warnings
- **Fix:** glob re-export (`pub use crate::platform::raspi3b::*;`) — identical re-export semantics (mirrors the glob idiom in platform/mod.rs itself), zero warnings; the four-name coverage is preserved through the glob
- **Files modified:** ../r9/aarch64/src/platform/raspi4b/mod.rs
- **Verification:** both raspi builds green with zero warnings
- **Committed in:** `65c737d`

**3. [Rule 1 - Bug] Pre-existing clippy warning in a moved file (plan-directed fix)**
- **Found during:** Task 2, regression sweep (clippy, 12 warnings)
- **Issue:** `clippy::extra_unused_type_parameters` on `Mailbox::request<T, U>` (mailbox.rs:75) — pre-existing (file content was 100% preserved by the move), but the plan says pre-existing warnings in MOVED files get fixed, not just noted
- **Fix:** dropped the unused type parameters from the method and the turbofish at its single call site; the free function `request<T, U>` (which has real generics) is untouched
- **Files modified:** ../r9/aarch64/src/platform/raspi3b/mailbox.rs
- **Verification:** clippy 12 → 11 warnings, mailbox.rs warning gone; both builds green
- **Committed in:** `6a309ae`

**4. [Rule 3 - Blocking] check-cfg cannot literally declare "every --cfg name xtask emits"**
- **Found during:** Task 1, step (6) (Cargo.toml block authoring)
- **Issue:** xtask's dev/ip/link/nodev/nouart families derive cfg NAMES dynamically from TOML content (`--cfg dev_<name>` etc.); check-cfg has no name-wildcard syntax, so an unbounded family cannot be literally enumerated
- **Fix:** enumerated the families from `xtask/src/config.rs` as directed, grep-confirmed zero family members are used in code today (so zero warnings are possible), declared the two cfgs that ARE used/planned (`platform` values, `dev_panic` for 03-06), and documented the declare-on-first-use family policy in a Cargo.toml comment
- **Files modified:** ../r9/aarch64/Cargo.toml (comment only)
- **Verification:** `unexpected_cfgs` + `dev_panic` greps pass; builds emit no cfg warnings
- **Committed in:** `65c737d`

---

**Total deviations:** 4 auto-fixed (3 Rule 1, 1 Rule 3)
**Impact on plan:** All four are minimal, intent-preserving adjustments discovered by the plan's own verification gates; no scope creep, no behavioral change on raspi, and the must-have truths/prohibitions all hold.

## Issues Encountered

None beyond the deviations above. Note for later plans: 11 pre-existing clippy warnings remain in files this plan did not touch (`aarch64/src/pre_mmu/util.rs` ×5 — unused lifetime, no-op operation, u8→u8 casts ×2, char-literal truncation ×2; `pre_mmu/vminit.rs` ×3 + `pagealloc.rs` ×1 — unneeded returns; `vm.rs` ×2 — too-many-arguments + unneeded return). Recorded here per the plan's "note rather than fix" instruction for non-moved files.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **03-04 can create `platform/virt/*` and flip `config_virt.toml`'s platform key with zero further restructuring** — the cfg'd virt arm compiles out until then (no config sets `platform = "virt"` yet, which remains correct and intended).
- **03-05 owns the virt unmask point:** call `trap::unmask_irq()` only after GIC + timer + UART RX handlers are armed (the P5 ordering contract the split created).
- **03-06's panic config:** `dev_panic` is already check-cfg-declared; `#[cfg(dev_panic)]` code will not warn.
- R9-01 remains open (shared by plans 03-02…03-06; the shared-ID gate marks it when the last declaring plan completes).

---
*Phase: 03-r9-qemu-virt-platform-bring-up*
*Completed: 2026-11-01*

## Self-Check: PASSED

- FOUND: ../r9/aarch64/src/platform/mod.rs
- FOUND: ../r9/aarch64/src/platform/raspi3b/mod.rs
- FOUND: ../r9/aarch64/src/platform/raspi3b/mailbox.rs (moved)
- FOUND: ../r9/aarch64/src/platform/raspi3b/uartpl011.rs (moved, not deleted)
- FOUND: ../r9/aarch64/src/platform/raspi4b/mod.rs
- FOUND: r9 commit 65c737d (`feat(03-02): restructure aarch64 crate into cfg-gated platform tree`, branch main)
- FOUND: r9 commit 6a309ae (`feat(03-02): gate raspi couplings not(virt); split trap init/unmask`, branch main)
- FOUND: .planning/phases/03-r9-qemu-virt-platform-bring-up/03-02-SUMMARY.md
