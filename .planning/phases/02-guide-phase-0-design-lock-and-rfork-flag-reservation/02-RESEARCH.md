# Phase 2: Guide Phase 0 - Design Lock and rfork Flag Reservation - Research

**Researched:** 2026-11-23 (forced re-research, independent of the earlier pass)
**Domain:** Rust kernel constants shared across crates, xtask code generation and drift checks, reproducible-kernel evidence, design-lock documentation
**Confidence:** HIGH for everything measured on this Mac; the x86_64 host-test leg and every Linux-host step are unverified here and are marked

r9 HEAD at research time: `575d093`. Harvey checkout HEAD: `c697666ba315f137ca2fd11768f835d04bd9f018`. All experiments ran in scratch copies made with `git archive HEAD`; the repository was not modified and nothing was committed.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried in (owner, 2026-11-22; not re-asked)
- **D-01:** Four flags, not three: `RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21. Reserved in r9 only; Harvey is never edited. The guide's "reserve bits 18-20 in `libc.h`" is a recorded deviation.
- **D-02:** Warnings W1 to W10 stay open and are settled in the phase each affects. This phase does not pull any of them forward.

#### Locking the guide's thirteen decisions (DSGN-05)
- **D-03:** Confirmation is in bulk with exceptions only. The phase presents all thirteen (IG-D01 to IG-D13) in one table with a proposed status each; the owner confirms the table at one checkpoint and rules individually only on the exceptions below. The result is written to the PROJECT.md Key Decisions table.
- **D-04:** Proposed for lock as written: IG-D01, IG-D02, IG-D04, IG-D05, IG-D06, IG-D08, IG-D10, IG-D12.
- **D-05:** IG-D03 (closed syscall list) is locked with its wording corrected from "3 flag bits" to four, per D-01.
- **D-06:** IG-D07 (kernel safety boundary) is locked amended to what is true at HEAD: `port/` uses per-module `forbid(unsafe_code)` with three named floor modules that hold `unsafe` (`globalalloc`, `mcslock`, `devip_waker`); no new floor module without owner approval. The guide's "crate-level forbid" wording is not locked. How lint and audit scope grow when new `unsafe` lands (note N2) stays for Phases 3 and 5, and the `ds/src/tpht/` part of the wording stays open under W5.
- **D-07:** IG-D09 (TPHT as primary hashing; open at W5, W6), IG-D11 (native `9P2000.aead`; open at W1 to W4) and IG-D13 (cluster discovery; open at W7) are **not locked** in this phase. They remain carried constraints, and the lock is deferred until their W items are settled in Phases 3, 4 and 6. PROJECT.md must record them as "not locked, deferred to Phase N" so the phase gate is explicit about which are locked and which are not.
- **D-08:** "Locked" means owner-only change. Planning and execution treat a locked decision as fixed. If a later phase finds one wrong, work stops and the owner is asked. A change is recorded as a new dated row that supersedes the old one, never a silent edit.

#### C prototypes (DSGN-02)
- **D-09:** No source document defines the "corrected C prototypes". The owner's reading: the C statement of the whole `rfork` interface, that is every `RF*` flag (Harvey's frozen values, `RFNOMNT` included, plus the four new ones) and the `rfork()` declaration.
- **D-10:** The landing location is a new top-level `sys/include/` directory, matching the guide's own landing sites (`sys/include/libc.h`, `sys/src/...`), so the file does not move when Phases 9 to 12 fill in `sys/`.
- **D-11:** The file is a small r9-owned `sys/include/rfork.h` holding only the `RF*` enum and the `rfork()` declaration, with a comment naming the Harvey `libc.h` lines it mirrors (`sys/include/libc.h:613-624` for the enum, `:684` for `extern int rfork(int);`). It is not a copy of Harvey's `libc.h` and is not named `libc.h`.
- **D-12:** `rfork.h` is generated, not hand-maintained: an xtask step writes it from the Rust flag constants, and a check fails when the committed file is stale. The Rust constants are the single source of truth.

#### Threat model document (DSGN-01)
- **D-13:** The document is `docs/threat-model.md`, beside `docs/devip-contract.md` and `docs/harvey-supply.md`.
- **D-14:** Depth is a full analysis: assets, attacker classes and their capabilities, trust boundaries, and a protection table. It is more than a restatement of AC section 2.2.
- **D-15:** Every statement either cites its source (compendium section, RFC section, guide row) or is visibly marked as new in this document. The in-scope and out-of-scope boundaries are stated exactly as AC section 2.2 / AC-T01 (pages at rest, stale-frame disclosure across KeyIDs, stray kernel reads, packet splicing, injection and replay in scope; DRAM bus probing, cold boot and kernel-context code execution out of scope; SEV-SNP or TDX as the remedy). The owner reviews the draft at a checkpoint before it is committed as final.
- **D-16:** The document is a living document and nothing in it is locked, the boundary included. Later phases may revise it as designs firm up. It is therefore not part of the D-08 lock set. None of its protections exists in code yet, and it must say so.

#### Reservation and test shape (DSGN-03, DSGN-04)
- **D-17:** Owner amendment to the roadmap: the `rfork` flags get **one definition in the `port/` crate**, not two arch copies. ROADMAP Phase 2 success criterion 2 and DSGN-03 both name `x86_64/src/proc.rs` and `aarch64/src/syscall.rs` as the home; the plan must amend both texts (and the matching lines in PROJECT.md and STATE.md) to say the constants live in one `port/` module that both arch crates import. This is a `port/src` edit, so the diff is shown to the owner before it is committed.
- **D-18:** All `rfork` flags move: the eleven existing flags (bits 0-6, 10-13) and the four new ones live in the one `port/` module; the duplicate constants are deleted from `x86_64/src/proc.rs:171-181` and `aarch64/src/syscall.rs:92-108`, which import them. No re-export shims.
- **D-19:** `RFNOMNT` = 1<<14 is added to the same module as a named constant that stays outside both `SUPPORTED` masks, exactly like the four reserved flags. `rfork(RFNOMNT)` keeps failing as it does today. The module therefore defines sixteen flags.
- **D-20:** "Behaviour unchanged" is pinned by a host test: none of the four reserved bits (nor `RFNOMNT`) is in either `sys_rfork` `SUPPORTED` mask, so `rfork` with any of them still returns "rfork: unsupported flags". Phases 6 and 7 must change that test on purpose when they make a flag live. No live boot check is added for this.
- **D-21:** The host test of DSGN-04 also pins the four values and proves they overlap no existing flag, with bit 14 covered through the real `RFNOMNT` constant.
- **D-22:** Kernel bytes follow the standing rules: compare both kernels' sha256 before and after the move. Moving constants is expected to change nothing; if x86_64 bytes do change, the `boot-iso` `outputHash`, amd64-container TCG gates and owner linux check apply, and an aarch64 change gets the quick pass.

#### Decisions after research (owner, 2026-11-23; source: `02-RESEARCH.md`, "Kernel Bytes" and "Open Questions")
- **D-23:** Supersedes the expectation in D-22. Research measured that shifting source lines changes x86_64 kernel bytes (panic line numbers in `.rodata`; `.text` identical in the experiment) and the aarch64 ELF (debug line tables). The owner chose to accept the change: edit the kernel sources naturally, with no line-count balancing, and run the full x86_64 byte-change procedure as planned work in this phase (re-derive and commit the `boot-iso` `outputHash` on the owner's x86_64 Linux host, the x64 TCG gates, the owner's linux boot-iso check). aarch64 gets the quick pass. The before/after sha256 of both kernels and of their `.text` sections is still recorded as evidence; a `.text` difference is a stop, because the phase must change no executable code.
- **D-24:** The x86_64 host test that the reserved bits are outside x86_64's supported mask is witnessed on the owner's x86_64 Linux host, at the same stop as the KVM gate. If the devshell skips the x86_64 host tests there too, the plan includes the small xtask fix that makes them run. Hosted CI is not the witness and no push is required for verification.
- **D-25:** The boot-to-rio gate (`cargo xtask gate`, owner's x86_64 Linux KVM host) runs once, at the end of the phase, after all work is committed. No per-commit or mid-phase gate run in this phase.

### Claude's Discretion
- Name and file of the `port/` module that holds the flags.
- How the generator reads the Rust constants (xtask depending on `port`, or another route) and the name of the xtask step and its staleness check; whether the check runs inside `cargo xtask check` or `test`.
- Where the host tests sit (the `port` crate for values and overlap; the arch crates for the `SUPPORTED` masks, which stay where they are).
- Layout and section order of `docs/threat-model.md`, and the form of the "new in this document" mark.
- The layout of the thirteen-decision table presented at the owner checkpoint, and the order of work within the phase.

### Deferred Ideas (OUT OF SCOPE)
- **Implement `RFNOMNT` (bit 14) in r9's kernel.** The owner asked for it during the discussion. It is a kernel behaviour change, which this phase excludes, so it needs its own phase or a place in an existing one. D-19 leaves the constant defined so the implementing phase only adds it to `SUPPORTED` and writes the behaviour.
- Locking IG-D09, IG-D11 and IG-D13: deferred by D-07 to Phases 3, 4 and 6 respectively, once W5/W6, W1-W4 and W7 are settled.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DSGN-01 | Threat model document committed, stating in-scope protections and out-of-scope attacks | Q6: source inventory with line references, contradictions between sources, a list of what is absent from every source and so must be marked new |
| DSGN-02 | Corrected C prototypes committed inside r9 | Q1 and Q8: `sys/include/rfork.h` generated by an xtask step that compiles `port/src/rfork.rs` by `#[path]`; route compiled and run in scratch, with a missing-file and a stale-file negative control |
| DSGN-03 | Four `rfork` bits reserved as named constants (home amended to one `port/` module by D-17) | Q3: every definition and use site; a `port/src/rfork.rs` module that built into all four kernel configs; the wording sites to amend (Q7) |
| DSGN-04 | Host test pins the four values, proves no overlap, behaviour unchanged | Q2: `port/tests/rfork_test.rs` and a mask test in each arch `syscall.rs`, run green on the Mac for `port` and `aarch64`; the x86_64 leg needs an xtask fix and the owner's host (D-24). Kernel Bytes: `.text` identical on all four kernel configs for the real edit shape |
| DSGN-05 | Owner confirms IG-D01..IG-D13; PROJECT.md records which are locked | Q7: verbatim current rows, the guide rows, the status table from D-03..D-07, every line that must change |
</phase_requirements>

## Summary

The phase is four small pieces of code and three documents. The code is: one new `port` module holding sixteen `rfork` flag constants, the deletion of the two arch copies, the hoist of each `sys_rfork` `SUPPORTED` mask to module level so a test can name it, and an xtask step that writes `sys/include/rfork.h` and refuses a stale copy. All of it was built and run in a scratch copy at HEAD `575d093`: the generator, both negative controls, the `port` and `aarch64` host tests, `cargo xtask check`, and all four kernel configurations the flake and the gate use.

The measurement the plan depends on holds for the real edit shape, not only for a comment shift. With the constant blocks deleted and both masks hoisted, `.text` is byte-identical on `q35churnshell`, `q35cpu`, `virtshell` and `virtcpu`. Every differing byte in a loaded section is a panic `Location` line number: 118 bytes in x86_64 `.rodata` and 44 bytes in aarch64 `.data`. The aarch64 change is therefore in a loaded section too, not only in debug tables as D-23's text says. That does not alter D-23 (aarch64 gets the quick pass either way) but the plan and evidence should describe it correctly.

Two things cannot be closed on this Mac and belong to the single end-of-phase owner stop: the x86_64 host tests (no Rosetta, no x86_64 std in the nix toolchain, so the x86_64 test module was not even compiled here) and everything that needs x86_64 Linux (`boot-iso`, KVM, the TCG gate). `cargo xtask test` skips the x86_64 crate in any devshell without `rustup`, so the fix D-24 anticipates is needed.

**Primary recommendation:** put the flags in `port/src/rfork.rs` behind one `macro_rules!` list, have xtask compile that same file with `#[path]` to render the header, run the staleness check at the top of `cargo xtask test`, extend the `TestStep` host-triple fallback to x86_64 hosts, and schedule one owner stop on the x86_64 Linux host that does hash re-derivation, `--rebuild`, host tests, KVM gate and TCG gate in that order.

## Differences from the previous research pass

The earlier file was read only after this pass's own measurements, as a list of leads.

| Topic | Previous pass | This pass |
|-------|---------------|-----------|
| aarch64 byte change | "debug line tables"; `.rodata` identical | Loaded bytes change too: 41 `u32` words (44 bytes) in `.data`, all panic line numbers. The stripped aarch64 kernels differ by exactly those 44 bytes. `.text` and `.text.boot` identical |
| Edit shape measured | One inserted comment line per arch | The real shape: both constant blocks deleted, imports changed, both masks hoisted, tests added, on four configs (`q35churnshell`, `q35cpu`, `virtshell`, `virtcpu`) |
| x86_64 `.rodata` delta | 66 bytes (comment shift) | 118 bytes (116 words) for the real edit; deltas are exactly -15, +13 and +2 lines, matching the edit |
| Generator route | `#[path]` include recommended, mechanics checked, sketch "not compiled" (its A4) | Same route, compiled and run end to end, including missing-file and stale-file failures and a C syntax check of the output |
| Where the staleness check runs | `CheckStep` (`cargo xtask check`) | `TestStep` (`cargo xtask test`), the command in `.planning/config.json` `test_command`; this placement was run. Either is defensible; only this one was exercised |
| x86_64 host-test witness | Offered hosted CI as witness | Superseded by D-24. Found the working Linux command in the legacy record and the exact xtask condition to change |
| Build reproducibility | Not tested across paths | Rebuilt HEAD at a second, deeper path: all four ELF sha256 values identical |
| Include guard in `rfork.h` | Recommended | Not recommended: Harvey's `libc.h` has none (0 matches for `#ifndef`/`#pragma once`) and the file is not meant to be included next to `libc.h` |
| Mac kernel versus Linux kernel bytes | "not tested" | Still not tested, but `flake.nix:228-237` states the kernel `.rodata` carries `/nix/store/<hash>-rust-default/...` strings, so a Mac-built ELF cannot equal the Linux one. Compare before and after on one venue only |
| `cargo xtask clippy --arch aarch64` | Listed in the full suite | Fails at HEAD with 4 `undocumented_unsafe_blocks` errors before any edit. Do not use it as a pass criterion |
| Transport to the owner's host | Not raised | `run-witness.sh` states the x86_64 host gets code "from GitHub with `git pull`". See Open Question 1 |

Agreements (independently reproduced): baseline ELF sha256 values for `q35churnshell` (`8f2d485e...`) and `virtshell` (`76e6e153...`), the stripped `q35churnshell` hash (`316934ca...`), the definition and use sites, Harvey's enum lines, and the PROJECT.md row inventory.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `rfork` flag values | `port` crate (portable kernel logic) | — | D-17: one definition both arch crates import |
| Which flags a kernel accepts (`RFORK_SUPPORTED`) | Each arch crate's `syscall.rs` | — | CONTEXT: the masks "stay where they are"; the two kernels may diverge later |
| C statement of the interface (`sys/include/rfork.h`) | Generated artefact in the repo | `xtask` (generator and drift check) | D-12: Rust constants are the single source |
| Value and overlap pins | `port` host tests | — | Pure constants, no arch dependency |
| Mask pins ("behaviour unchanged") | Arch crate host tests | Owner's x86_64 host for the x86_64 leg | D-20, D-24 |
| Kernel byte evidence | Local `cargo xtask build` on the Mac | Owner's Linux host for `boot-iso` | Standing owner rule, D-23 |
| Boot to rio | `cargo xtask gate` on x86_64 Linux KVM | — | D-25, Phase 1 D-13 |
| Design lock record | `.planning/PROJECT.md` Key Decisions | ROADMAP, REQUIREMENTS, STATE wording | D-03, D-17 |
| Threat model | `docs/threat-model.md` | — | D-13 |

## Standard Stack

No new dependency. Nothing is installed in this phase.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Rust toolchain | `nightly-2026-01-30` | Everything | Pinned in `rust-toolchain.toml` [VERIFIED: `rust-toolchain.toml`: `channel = "nightly-2026-01-30"`] |
| `clap` | 4.5 (already in `xtask/Cargo.toml`) | The new `rfork-header` subcommand | The existing subcommand mechanism [VERIFIED: `xtask/src/main.rs:90-276`] |
| `std::fs` | — | Write and compare the header | `uregcheck.rs` precedent: "Plain text parsing — no crate" [VERIFIED: `xtask/src/uregcheck.rs:19`] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| xtask `#[path]`-including `port/src/rfork.rs` | `xtask` depending on `port` | Pulls `smoltcp`, `miniz_oxide`, `bitflags` and three nightly features into every xtask build, including the `kernel-x64` and `netdev-args` nix derivations that build xtask first. Not tried; not needed |
| same | A `port` integration test that renders and compares the header | Runs automatically under `cargo xtask test`, but D-12 asks for an xtask step that writes the file, so the renderer would exist twice or live in the kernel crate |
| same | `include_str!` plus parsing the Rust source | A parser to maintain; the compiler already parses the file |

## Package Legitimacy Audit

Not applicable: this phase installs no external package. `Cargo.lock` does not change (verified in the scratch build: no new crate was fetched or compiled).

## Architecture Patterns

### System Architecture Diagram

```
                    port/src/rfork.rs   (rfork_flags! list: 16 names, one bit each)
                       |            \
        compiled into  |             \  compiled again, by #[path], into
        the kernels    |              \ the host tool
                       v               v
   x86_64/src/{proc,syscall}.rs     xtask/src/rforkh.rs
   aarch64/src/{proc,syscall}.rs       | render()
        |  use port::rfork::*          |
        v                              +--> `cargo xtask rfork-header`        --> writes sys/include/rfork.h
   RFORK_SUPPORTED (per arch,          +--> `cargo xtask rfork-header --check` --> exit 1 if file missing or stale
   module level, 11 flags)             +--> top of `cargo xtask test`          --> same check, then the cargo test legs
        |
        v
   sys_rfork: flag & !RFORK_SUPPORTED != 0  -->  "rfork: unsupported flags"   (unchanged behaviour)

   Host tests:
     port/tests/rfork_test.rs            values, single-bit, no overlap, Harvey prefix      (any host)
     aarch64 syscall.rs mask test        5 named bits outside mask, mask == 0x3c7f          (aarch64 host: the Mac)
     x86_64 syscall.rs mask test         same                                               (x86_64 host: owner, D-24)

   Evidence:
     Mac:   build 4 kernel configs before and after  -->  whole-file sha256 differs, .text sha256 equal (else STOP)
     Linux: boot-iso hash mismatch --> commit new outputHash --> build + --rebuild --> host tests --> gate kvm --> gate tcg
```

### Recommended Project Structure
```
port/src/rfork.rs            # NEW  the sixteen flags (owner approves the diff first)
port/src/lib.rs              # +2 lines: #[forbid(unsafe_code)] pub mod rfork;
port/tests/rfork_test.rs     # NEW  value and overlap pins (not under port/src)
x86_64/src/proc.rs           # delete lines 167-181, import from port::rfork
x86_64/src/syscall.rs        # import from port::rfork, hoist RFORK_SUPPORTED, mask test
aarch64/src/syscall.rs       # delete lines 87-108, import, hoist, mask test
aarch64/src/proc.rs          # 12 sites: crate::syscall::RF* -> port::rfork::RF*
xtask/src/rforkh.rs          # NEW  render / write / check
xtask/src/lib.rs             # + pub mod rforkh;
xtask/src/main.rs            # + rfork-header subcommand; check call in TestStep; x86_64 fallback
sys/include/rfork.h          # NEW  generated
docs/threat-model.md         # NEW
```
`sys/` and `docs/threat-model.md` are not ignored: `.gitignore` holds only `/target`, `/result`, `/graphify-out/` [VERIFIED: `.gitignore`; `git check-ignore` returned no match for either path].

### Pattern 1: one list, constants and table together
**What:** a `macro_rules!` in `port/src/rfork.rs` expands one list into the `pub const`s and an `ALL: &[(&str, u64)]` table, so the name in the header is `stringify!` of the identifier and cannot drift from it.
**When to use:** here. The table costs the kernels nothing: the measurement shows no kernel byte other than line numbers changed, so `ALL` is not emitted into either kernel.
**Example:** see Code Examples (compiled and run).

### Pattern 2: drift check with both failure directions shown
**What:** `check()` compares the committed file with `render()`. Precedent: `uregcheck::run_check` runs a negative control after the real check ("prove the gate can catch drift, not just parse the pristine header") [VERIFIED: `xtask/src/uregcheck.rs:134-136`]. The Phase 1 manifest engine is the other precedent: two-way drift between manifest and image [VERIFIED: `xtask/src/main.rs:3761-3765`, doc comment "two-way drift, Rust provenance, path validation"].
**Measured:** file absent gives exit 1 naming the regenerate command; a one-character mutation (`(1<<21)` to `(1<<22)`) gives exit 1 from both `cargo xtask rfork-header --check` and `cargo xtask test`.

### Pattern 3: hoist the mask, keep the check
**What:** replace the function-local `const SUPPORTED` with a module-level `pub(crate) const RFORK_SUPPORTED` holding the same eleven names in the same order; the function keeps both checks and both error strings. A `#[cfg(test)]` module in the same file asserts the five unsupported names are outside it and that it equals `0x3c7f`.
**Why required:** a `const` declared inside a function body is not nameable from any other item, including a test module [VERIFIED: `x86_64/src/syscall.rs:1659`, `aarch64/src/syscall.rs:1967`, both inside `fn sys_rfork`].

### Anti-Patterns to Avoid
- **Re-export shims** (`pub(crate) use port::rfork::*` in the old homes): D-18 forbids them. `aarch64/src/proc.rs` must change its twelve `crate::syscall::RF*` paths.
- **Line-count balancing:** D-23 rules it out.
- **Putting the renderer in `port`:** it would add host-only string code to a kernel crate and widen the `port/src` approval.
- **Including `rfork.h` beside Harvey's `libc.h`:** both declare an anonymous enum with the same enumerators; a C translation unit including both fails. The banner should say so if the owner wants it stated.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Reading Rust constants from a host tool | A source parser | `#[path = "../../port/src/rfork.rs"] mod flags;` | The compiler is the parser; compiled and run |
| Section hashing | A custom ELF tool in the repo | `"$OBJCOPY" -O binary --only-section=.text <elf> <out>` then `shasum -a 256` | `OBJCOPY` is exported by the devshell hook (`flake.nix:58-63`); ran on both arches |
| The x86_64 Linux witness steps | New scripts | `run-witness.sh` steps `kernel` and `gate` | They already do build, `--rebuild` and the double gate with named verdict lines |
| C header syntax check | A C parser | `cc -fsyntax-only -x c sys/include/rfork.h` | One command; passed on the generated file with Apple clang |

**Key insight:** the phase's risk is in evidence and ordering, not in code. Reuse the Phase 1 witness driver and the existing gate rather than writing new harness code.

## Runtime State Inventory

Constants move between files, so each category was checked.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None. The flags are compile-time constants; verified by grep of `aarch64 x86_64 riscv64 port xtask lib docs gates nix` for all sixteen names | None |
| Live service config | None. No gate script or config names a flag as data (`nix/gates/tier1-r9.sh` and two config TOMLs mention `RFMEM` in comments only) | None |
| OS-registered state | None | None |
| Secrets / env vars | None | None |
| Build artefacts | The fixed-output `boot-iso` hash: `outputHash = "sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=";` [VERIFIED: `flake.nix:983`]. It covers an ISO holding the stripped x86_64 kernel, so it goes stale with this phase's kernel edit. Cargo `target/` is ignored and rebuilt | Re-derive and commit on the owner's x86_64 Linux host (D-23) |

## Kernel Bytes: measured on the real edit shape

**Method.** Three scratch trees from `git archive HEAD`: `base`, `b2/deeper/path` (same source, different path) and `edit` (the full code change in Code Examples). Each built with `cargo xtask build` inside the repo's devshell for four configs. Section hashes came from a small ELF reader and were cross-checked with `"$OBJCOPY" -O binary --only-section=.text`.

**Why these four configs.** `q35churnshell` is what the flake's `kernel-x64` builds and `boot-iso` stages [VERIFIED: `flake.nix:415`, `cargo xtask build --arch x86-64 --config q35churnshell`]. `q35cpu` is what the gate builds [VERIFIED: `xtask/src/gate.rs:11`, "build the kernel with `--config q35cpu`"]. `virtshell` is `kernel-virt` and `virtcpu` is `kernel-virt-cpu` [VERIFIED: `flake.nix:544`, `flake.nix:578`].

**Reproducibility across paths (same source, two paths):** all four identical.

| Config | Whole-ELF sha256 at HEAD `575d093` (Mac build) |
|--------|-----------------------------------------------|
| `q35churnshell` | `8f2d485e375937a764006273bbd6caf1f9bdb178dd307ad973abe981ac616cce` |
| `q35cpu` | `c53ad37fde7712891d09ff08d7d0ebec660a05f82dcc98c400e255472fe2c61c` |
| `virtshell` | `76e6e153f578b7022da74570802729e51d3105b9e3ffab0c31ef12917fec56d2` |
| `virtcpu` | `10fd33201b83e8b22cb0ba780a9a8ccb6a416195ea6e6a2c214723a06c1b012a` |

**Before versus after the real edit:**

| Config | Whole ELF | `.text` | Loaded sections that differ | Stripped ELF (`objcopy --strip-all`) |
|--------|-----------|---------|-----------------------------|--------------------------------------|
| `q35churnshell` | differs (`d955f169...`) | identical `38eed81462c8bf248f48310a08c0ebd9e4e62b6a8244a05a8e5df0d180820330` | `.rodata`, 118 bytes | 118 bytes differ (`316934ca...` to `ffe5f3b5...`) |
| `q35cpu` | differs (`32de083b...`) | identical `3618c3304bf0c8d8...` | `.rodata`, 118 bytes | 118 bytes differ |
| `virtshell` | differs (`6f752134...`) | identical `b904ec4e5ae8dc76e4b2edce5f20e5fd5adc331bbddd203cc3f060ec9ff540fb`; `.text.boot` identical | `.data`, 44 bytes | 44 bytes differ |
| `virtcpu` | differs (`d60f59ad...`) | identical `ea9b4bcd901f07c9...`; `.text.boot` identical | `.data`, 44 bytes | 44 bytes differ |

`.data` (x86_64), `.rodata` (aarch64), `.bss`, `.earlyvm_pagetables` and `.eh_frame_hdr` are identical. The other differing sections are not loaded: `.debug_abbrev`, `.debug_aranges`, `.debug_info`, `.debug_line`, `.symtab`, `.strtab`.

**What the loaded differences are.** Read as little-endian `u32` words, every differing word changes by one of a few constants, and those constants are the line shifts of the edit:

| Arch | Differing words | Delta histogram (new minus old) | Explanation |
|------|-----------------|---------------------------------|-------------|
| x86_64 | 116 | -15 (65), +13 (28), +2 (23) | `proc.rs` loses 15 lines after line 167; `syscall.rs` gains 13 lines above `sys_rfork` (hoisted mask) and nets +2 below it (11 lines removed inside the function) |
| aarch64 | 41 | -7 (28), -18 (13) | `syscall.rs`: the 22-line block at 87-108 becomes 15 lines, then 11 more lines leave `sys_rfork` |

Example: the word at x86_64 `0xffffffff80405060` goes 1217 to 1230; the word at aarch64 `0xffff800040778d20` goes 1365 to 1358. These are panic `Location` line fields. No other loaded byte changed. [VERIFIED: measured this session; scripts and ELFs under the session scratchpad]

**Consequences for the plan.**
1. The `.text` stop condition in D-23 is met by the real shape on both arches. The executor must still re-measure on the committed result, because the final edit may differ in detail from the scratch one.
2. Record evidence as: whole-ELF sha256 and `.text` sha256, before and after, for all four configs (the flake kernel and the gate kernel on x86_64 are different configs; record both). Add `.text.boot` for aarch64.
3. Take "before" from the commit preceding the first kernel-affecting commit, and keep the ELF copies outside `target/`.
4. Before and after must be built on the same venue. The stripped kernel carries toolchain store-path strings: "the store-hash runs inside build-std's panic-location strings (/nix/store/<hash>-rust-default/... rides the kernel's .rodata)" [VERIFIED: `flake.nix:230-232`]. A Mac ELF hash will not equal the Linux `kernel-x64.elf` hash and must not be compared with it.
5. The aarch64 delta is in `.data`, a loaded section. D-23's parenthetical "debug line tables" understates it; the action (quick pass) is unchanged.
6. Adding `port/src/rfork.rs` alone (before any arch edit) was not measured as a separate step. It changes the `port` crate hash and may change non-loaded symbol tables. D-23 needs base versus final only.

**Commands (run on the Mac in this session's scratch copy; substitute the repo root):**
```bash
nix develop -c cargo xtask build --arch x86-64 --config q35churnshell
shasum -a 256 target/x86_64-unknown-none-elf/debug/x86_64
nix develop -c bash -c '"$OBJCOPY" -O binary --only-section=.text target/x86_64-unknown-none-elf/debug/x86_64 "$OUT/q35churnshell.text"'
# repeat with --config q35cpu; then --arch aarch64 --config virtshell and virtcpu
# (ELF at target/aarch64-unknown-none-elf/debug/aarch64; also hash --only-section=.text.boot)
```
Each config overwrites the same `target/.../debug/<arch>` file, so copy the ELF out after each build. All four builds together took 47 s cold in scratch.

## Per-Question Findings

### Q1. How an xtask step reads the Rust constants; staleness precedent; what `check` and `test` run

- `xtask` does not depend on `port` [VERIFIED: `xtask/Cargo.toml` `[dependencies]`: `clap`, `serde`, `target-lexicon`, `toml`]. `port` is `#![cfg_attr(not(any(test)), no_std)]` with `#![feature(allocator_api)]` and `#![feature(step_trait)]` and pulls `bitflags`, `miniz_oxide`, `adler2`, `smoltcp` [VERIFIED: `port/src/lib.rs:3-5`, `port/Cargo.toml`]. Making xtask depend on it is possible in principle on the pinned nightly but was not tried and is not recommended (see Alternatives).
- **Recommended route, compiled and run:** `xtask/src/rforkh.rs` declares `#[path = "../../port/src/rfork.rs"] #[allow(dead_code)] mod flags;`. The path is relative to `xtask/src/`. `rfork.rs` uses only `core` syntax, so it compiles unchanged under `std`. Cross-crate file inclusion already exists in xtask: `include_str!("../../x86_64/lib/config_q35shell.toml")` [VERIFIED: `xtask/src/lib.rs:109`]. The nix derivations use `src = ./.` [VERIFIED: `flake.nix:409`], so the path resolves in the sandbox.
- **Results in scratch:**
  - `cargo xtask rfork-header --check` with no file: `rfork-header: read .../sys/include/rfork.h: No such file or directory (os error 2) (run `cargo xtask rfork-header`)`, exit 1.
  - `cargo xtask rfork-header`: writes the file; `--check` then prints "matches port/src/rfork.rs", exit 0.
  - After `sed 's/(1<<21)/(1<<22)/'`: `--check` exit 1 ("is stale against port/src/rfork.rs"); `cargo xtask test` exit 1 with the same line before any cargo leg runs.
  - `cc -fsyntax-only -Wall -std=c99 -x c sys/include/rfork.h`: clean.
- **Staleness precedent:** `xtask/src/uregcheck.rs` compares Rust-side pinned tables with a C header and ends with a negative control; it lives in the xtask lib (`pub mod uregcheck;`) with its step wired in `main.rs` (`UregCheckStep`, `xtask/src/main.rs:4732-4742`). The new module follows the same split.
- **What `cargo xtask check` runs** [VERIFIED: `xtask/src/main.rs:3652-3740`]: `cargo check` for `aarch64 --bins`, `riscv64 --bins`, `x86_64 --bins`, `port --lib --tests --benches`, then `--tests --benches` legs for each arch only where `rustup` reports an installed std target. It runs no test.
- **What `cargo xtask test` runs** [VERIFIED: `xtask/src/main.rs:3521-3607`]: `cargo test --package port --lib --tests`, then one host-arch crate leg (see Q2). **It does not run the xtask crate's own tests.** Those run only through `cargo test -p xtask` (Phase 1's validation file records this as its warning W-1).
- **Recommendation:** subcommand `rfork-header` with `--check`; call `xtask::rforkh::check(&workspace())?` as the first statement of `TestStep::run`. `cargo xtask test` is `.planning/config.json`'s `test_command` and is run by CI (`.github/workflows/xtask.yml`, jobs `tests` and `test-aarch64-host`), so a stale header fails locally and in CI with no extra command. A `rforkh` unit test under `cargo test -p xtask` covers the renderer's shape.
- **Scratch caveat:** `cargo test -p xtask` needs `gates/` and `.planning/phases/01-.../witness/run-witness.sh` present (two `gate::tests` read them). In the real repo they exist. In scratch the lib tests passed (97, including the new one) and those two bin tests failed for the missing file only, identically in the unedited tree.

### Q2. Where host tests run; the mask; what the owner must run (D-24)

**Crates that build on the host:** `port` (std under `cfg(test)`), `xtask`, and each arch crate's bin under `cfg(test)` (`#![cfg_attr(not(any(test)), no_std)]`, `#![cfg_attr(not(test), no_main)]`) [VERIFIED: `x86_64/src/main.rs:4-5`, `aarch64/src/main.rs:3-4`]. Both `mod proc` and `mod syscall` are compiled in every config, with no `cfg` gate [VERIFIED: `x86_64/src/main.rs:35,41`; `aarch64/src/main.rs:34,38`], so a mask test in `syscall.rs` runs under the default cfg leg.

**How `TestStep` selects the arch leg** [VERIFIED: `xtask/src/main.rs:3550-3583`]:
```rust
        let arch = std::env::consts::ARCH;
        let mut target = rustup_state.std_supported_target(arch).map(|t| t.to_string());
        ...
        if target.is_none() && arch == "aarch64" {
            target = host_triple();
        }
```
`RustupState::new()` tolerates a missing `rustup` and is then empty [VERIFIED: `xtask/src/main.rs:45-55`].

| Host | `rustup` on PATH | Legs that run | Skipped |
|------|------------------|---------------|---------|
| aarch64-darwin devshell (this Mac) | no | `port`; `aarch64 --bins --target aarch64-apple-darwin`; the same under the `virtshell` cfg set | x86_64 crate tests |
| x86_64 Linux devshell | no | `port` only | **x86_64 crate tests** (and there is no aarch64 leg) |
| x86_64 Linux with `rustup` and an installed `x86_64-unknown-linux-gnu` | yes | `port`; `x86_64 --bins --target x86_64-unknown-linux-gnu` | — |
| Hosted CI `ubuntu-latest` (`xtask.yml` `tests`) | yes | as the row above | — |

Run on the Mac with the scratch edit: `port` lib 168 passed, `rfork_test` 3 passed, `aarch64` 72 passed (default cfg) and 88 passed (`virtshell` cfg), each including `syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported`.

**The mask must be hoisted for D-20** (Pattern 3). Existing test modules to sit beside: `x86_64/src/syscall.rs:2246` (`mod tests`), `aarch64/src/syscall.rs:2535, 2604, 2680`.

**D-24: what the owner runs, and the fix.**
- In a Linux devshell without `rustup`, bare `cargo xtask test` does not run the x86_64 crate. The legacy record shows the same and the command that works: "ran `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` inside `nix develop --command sh -c '...'`" and "the exact command xtask would issue with rustup present" [CITED: `.planning/legacy/phases/14-harvey-amd64-userspace-on-r9/14-03-SUMMARY.md:121`, `14-01-SUMMARY.md:118`].
- **Fix (xtask, no owner approval needed):** widen the fallback condition to `if target.is_none() && (arch == "aarch64" || arch == "x86_64")`. On an x86_64 Linux devshell `host_triple()` then yields the leg `cargo test --package x86_64 --bins --target x86_64-unknown-linux-gnu`. CI behaviour does not change (rustup already supplies the same triple). Update the comment above it, which says the fallback is aarch64-only.
- **Owner command after the fix:** `nix develop -c cargo xtask test --verbose`, expecting an `Executing` line for package `x86_64` and `test syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok`. Fallback if the fix misbehaves: `nix develop -c cargo test --package x86_64 --bins --target x86_64-unknown-linux-gnu rfork`.
- **Not verified here [ASSUMED]:** the x86_64 test module, the `use port::rfork::RFPROC;` added to `x86_64/src/proc.rs`'s `mod tests`, and the fallback on x86_64 were not compiled or run. This Mac has no Rosetta (`arch: posix_spawnp: /usr/bin/true: Bad CPU type in executable`), the nix toolchain ships std only for `aarch64-apple-darwin`, and `-Z build-std` for `x86_64-apple-darwin` failed with `error[E0152]: duplicate lang item in crate core`. The x86_64 **kernel** (non-test) build with the edit succeeded. The x86_64 mask test is the same text as the aarch64 one that passed.
- The x86_64 crate has 139 `#[test]` attributes across 16 files (grep count). All of them will run on the owner's host for the first time through `cargo xtask test`. If one fails, run the same command at the phase's base commit to tell a pre-existing failure from a new one.

### Q3. Every definition and import site; `port` layout

**Definitions (the only two):**

`x86_64/src/proc.rs:167-181` [VERIFIED: read]:
```rust
// rfork flags (Harvey sys/include/libc.h — frozen values).  Homed here
// (the aarch64 port's home is syscall.rs) so the process machinery —
// rfork_clone/rfork_copy — compiles standalone; syscall.rs consumes
// them from this module.
pub(crate) const RFNAMEG: u64 = 1 << 0;
pub(crate) const RFENVG: u64 = 1 << 1;
pub(crate) const RFFDG: u64 = 1 << 2;
pub(crate) const RFNOTEG: u64 = 1 << 3;
pub(crate) const RFPROC: u64 = 1 << 4;
pub(crate) const RFMEM: u64 = 1 << 5;
pub(crate) const RFNOWAIT: u64 = 1 << 6;
pub(crate) const RFCNAMEG: u64 = 1 << 10;
pub(crate) const RFCENVG: u64 = 1 << 11;
pub(crate) const RFCFDG: u64 = 1 << 12;
pub(crate) const RFREND: u64 = 1 << 13;
```
`aarch64/src/syscall.rs:87-108` [VERIFIED: read]: a five-line comment (87-91), then the same eleven constants with interleaved doc comments; `RFPROC` is private there (`const RFPROC: u64 = 1 << 4;` at line 98), the other ten are `pub(crate)`. CONTEXT cites 171-181 and 92-108 (the constants); the comment blocks above them (167-170 and 87-91) describe the old homes and go too.

**Use sites that must change:**
| File | Lines | Today | Change |
|------|-------|-------|--------|
| `x86_64/src/syscall.rs` | 91-92 | `// rfork flags live in proc.rs (the process machinery owns them).` and `use proc::{RFCFDG, RFCENVG, RFCNAMEG, RFENVG, RFMEM, RFNAMEG, RFNOTEG, RFNOWAIT, RFFDG, RFPROC, RFREND};` | `use port::rfork::{...}`; drop the stale comment |
| `x86_64/src/syscall.rs` | 1659-1670 | function-local `SUPPORTED` | module-level `RFORK_SUPPORTED` |
| `x86_64/src/proc.rs` | unqualified uses at 2675, 2702, 2724, 2725, 2851, 2853, 2862, 2864, 2871, 2873, 2920, 2925 | resolved by the local consts | add `use port::rfork::{RFCENVG, RFCFDG, RFCNAMEG, RFENVG, RFFDG, RFMEM, RFNAMEG, RFNOTEG, RFNOWAIT, RFREND};` (ten names; `RFPROC` is not used in non-test code there) |
| `x86_64/src/proc.rs` | 3253 (inside `mod tests`, which has `use super::*;` at 2938) | `rfork_clone(&mut parent, &frame, RFPROC)` | add `use port::rfork::RFPROC;` inside `mod tests` [ASSUMED to compile; not built here] |
| `aarch64/src/syscall.rs` | 1961-2015 | unqualified names | one `use port::rfork::{...}` (eleven names) |
| `aarch64/src/syscall.rs` | 1967-1978 | function-local `SUPPORTED` | module-level `RFORK_SUPPORTED` |
| `aarch64/src/proc.rs` | 2536, 2572, 2598, 2599, 2827, 2829, 2840, 2842, 2853, 2855, 2888, 2896 | `crate::syscall::RF*` | `port::rfork::RF*` (a private `use` in `syscall.rs` is not visible from `proc.rs`) |

No other code site exists. `riscv64/` has none. `port/src/*.rs` and `port/tests/*.rs` mention flag names in comments and assertion strings only. `xtask/src/main.rs:2930,2945` mention `RFMEM` inside message strings. `HowItWorks.md:88` describes the flag validation in prose and names no file line. [VERIFIED: grep for all sixteen names across `aarch64 x86_64 riscv64 port xtask lib docs gates nix` and the top-level docs, with the code hits then read]

**Both kernels reject the same way** [VERIFIED: `x86_64/src/syscall.rs:1652-1672`, `aarch64/src/syscall.rs:1960-1980`]: `let flag = args[0] & 0xffff_ffff;`, then the three mutually-exclusive pairs give `"rfork: bad flags"`, then `if flag & !SUPPORTED != 0 { return err(proc, "rfork: unsupported flags"); }`. Both masks are the same eleven names in the same order: `RFPROC | RFMEM | RFFDG | RFNOWAIT | RFREND | RFNOTEG | RFNAMEG | RFCNAMEG | RFENVG | RFCENVG | RFCFDG`, value `0x3c7f`.

**`port` layout and conventions** [VERIFIED: `port/src/lib.rs:1-60` read, remainder viewed]: each module is declared `#[forbid(unsafe_code)] pub mod name;`, roughly alphabetical; three modules lack the attribute (`devip_waker`, `globalalloc`, `mcslock`). Add
```rust
#[forbid(unsafe_code)]
pub mod rfork;
```
after `pub mod rendez;`. Constants must be `pub` (the consumers are other crates) and `u64` (both `sys_rfork` copies work in `u64`). `UNSAFE-AUDIT.md:84` already covers "every other port module" as unsafe-free, so the audit file needs no edit. `port/Cargo.toml` opts into the workspace lints; the new module triggered no new warning. Tests go in `port/tests/rfork_test.rs`: the integration-test directory already holds ten files and is picked up by `--tests` with no wiring. It is outside `port/src`, so the owner approval covers `port/src/rfork.rs` and the two-line `lib.rs` change (show the test file with it anyway).

### Q4. Reproducibility, the `outputHash`, and the x86_64 byte-change procedure as last performed

**Reproducibility mechanism:** `apply_path_remap` adds `--remap-path-prefix=<NIX_BUILD_TOP>=/build` and `--remap-path-prefix=<workspace>=/r9` to every kernel build [VERIFIED: `xtask/src/config.rs:847-856`]. Confirmed in practice above (two paths, identical hashes).

**Flake outputs** [VERIFIED: `flake.nix`]:
- `kernel-x64` (Linux systems only, `flake.nix:603-604`): `cargo xtask build --arch x86-64 --config q35churnshell`, output `kernel-x64.elf` (`:406-424`).
- `kernel-virt` / `kernel-virt-cpu`: `--arch aarch64 --config virtshell` / `virtcpu` (`:535-556`, `:569-584`).
- `boot-iso` exists only for `system == "x86_64-linux"` (`:624-626`), is fixed-output, and stages the kernel with `objcopy --strip-all "$KERNEL" "$stage/boot/kernel"` followed by a `sed` that zeroes store hashes (`:236-237`).
- The hash line: `outputHash = "sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=";` (`flake.nix:983`).
- The comment directly above it (`:975-982`) describes the earlier `sha256-xFt7...` value "at 9af05f8"; the current value was set by owner commit `2bc5478` ("witness(01-34): stop C2 returns; boot-iso hash update") after kernel fix `2a5cbed`, without a new comment paragraph. The re-derivation commit in this phase should add its own paragraph.
- STATE.md's "Owed checks" still names `sha256-TIzwBaHF...` at `116bcc2`. That value was replaced twice in Phase 1; do not use it.

**The procedure as last performed (Phase 1, plans 01-33 and 01-34)** [CITED: `.planning/phases/01-stage-p0-cross-build-harness/witness/WITNESS-LOG.md:355-359, 383-385, 457-469, 816-832`; `01-33-SUMMARY.md`; `witness/run-witness.sh:437-492`]:
1. On the owner's x86_64 Linux host, `nix build .#boot-iso` at the commit with new kernel bytes fails with
   `error: hash mismatch in fixed-output derivation '...-r9-boot-iso-<rev>.drv': specified: sha256-<old> got: sha256-<new>`.
2. The `got:` value is written into `flake.nix:983` and committed as its own commit. Recorded rule: "A boot-iso fixed-output hash re-derivation is its own commit and must be proven by the plain build AND --rebuild" [CITED: `01-33-SUMMARY.md:36`].
3. The witness `kernel` step proves it: `nix build --no-link --print-out-paths .#kernel-x64`, `sha256sum .../kernel-x64.elf`, `nix build --no-link --print-out-paths .#boot-iso` (verdict `KERNEL-BOOTISO-BUILD-OK`), then `nix build --no-link .#boot-iso --rebuild` (verdict `KERNEL-BOOTISO-REBUILD-OK`). The plain build must precede `--rebuild`, which refuses a never-built derivation. Invocation: `R9_WITNESS_STEPS="kernel" bash .planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh`.
4. The TCG leg: `nix develop -c cargo xtask gate --arch x86-64 --accel tcg` on the owner's host. The flake comment records the owner-approved venue: "The native amd64 host is the TCG venue (the Docker container is not required when the owner's host is already amd64)" [VERIFIED: `flake.nix:980-982`].
5. The "owner's linux boot-iso check" is step 3's `--rebuild`.

**What runs where:**
| Step | Mac (aarch64-darwin) | Owner's x86_64 Linux host |
|------|----------------------|---------------------------|
| Before/after kernel builds, whole-ELF and `.text` sha256 | yes | optional |
| `port` and `aarch64` host tests, header check | yes | yes |
| x86_64 host tests | no | yes |
| aarch64 quick pass | yes (HVF and TCG) | no |
| `nix build .#boot-iso`, hash re-derivation, `--rebuild` | no (`boot-iso` is x86_64-linux only) | yes |
| `cargo xtask gate` KVM and TCG | no (`GATE-VENUE not-linux`) | yes |

**The aarch64 quick pass as commands** (last run green in plan 01-30) [CITED: `01-30-SUMMARY.md:171`]:
```bash
nix run .#boot-r9-tier1 && R9_ACCEL=tcg nix run .#boot-r9-tier1 && nix run .#boot-r9-smoke && nix run .#boot-r9-net-smoke && nix run .#boot-r9-cpu && echo AARCH64-QUICK-PASS
```
Tier-1 on HVF then TCG, then smoke, net-smoke and cpu once on HVF. Each prints `... ALL STAGES PASS`. The HVF and TCG tier-1 runs share one capture path, so the capture on disk is the TCG run. These apps build `kernel-virt` / `kernel-virt-cpu` through nix, which on the Mac goes to the registered Linux builder; whether the builder is up was not probed in this session [ASSUMED available, as in Phase 1].

### Q5. The boot-to-rio gate, specified for the single end-of-phase run (D-25)

- Command on the witness: `nix develop -c cargo xtask gate --arch x86-64 --accel kvm`. KVM is the default accelerator and "never silently downgrades" [VERIFIED: `xtask/src/main.rs:268-270`].
- What it does [VERIFIED: `xtask/src/gate.rs:1-23`]: resolves the read-only amd64 Harvey tree, assembles `image:cpu`, builds the `q35cpu` kernel, composes the ISO, runs `gates/x86_64/drawterm-gate.expect` twice (`GATE_MODE=rio`, then `GATE_MODE=control-noG`), proves the tree unmodified, prints `GATE PASS quick x86_64 run=<id>`.
- Venue restrictions [VERIFIED: `xtask/src/gate.rs:126-143`, and the header lines 30-33]: refuses a non-Linux host (`GATE-VENUE not-linux`), a missing `grub-mkrescue` (`GATE-VENUE grub-mkrescue-missing`), and KVM without a writable `/dev/kvm` (`GATE-VENUE kvm-unavailable`). The quick tier is x86-64 only.
- Witness rule: "The required green witness is a local x86_64 Linux run under KVM. Hosted CI runs the gate best-effort. On the Mac the gate may run under TCG as a convenience but is not the witness." [VERIFIED: `01-CONTEXT.md:37`, D-13]. In practice the Mac refuses outright.
- Tier rule: "Two tiers. The quick gate ... runs for every commit. The full set of moved gates (smoke, net, auth, services and the rest) runs at phase boundaries and at Harvey pin bumps." [VERIFIED: `01-CONTEXT.md:38`, D-14]. Full tier: `cargo xtask gate --arch x86-64 --full --accel kvm`, eight apps listed in `FULL_APPS_X86_64` (`xtask/src/gate.rs:52-61`).
- D-25 fixes one run of `cargo xtask gate` at the end of this phase and overrides the per-commit practice for Phase 2. It does not say whether the phase-boundary `--full` run of Phase 1 D-14 also applies. See Open Question 2.
- The witness driver's `gate` step runs the quick gate twice and requires both [VERIFIED: `run-witness.sh:494-514`]. D-25 says "once"; calling the gate directly gives one run, the `gate` step gives two. The plan should name which it wants.
- `docs/harvey-supply.md` section 6 restates the venue rule and covers the Harvey supply (devshell exports, lima builder, pin bumps); nothing in it needs to change for this phase.

### Q6. Threat-model source inventory (`docs/threat-model.md`)

Line numbers are from `grep -n` and direct reads this session. AC = `.planning/r9-architecture-and-code-v4.md`; RFC = `.planning/rfc-9p2000-aead.md`; IG = `.planning/r9-implementation-guide-amended-v3.md`; PT = `.planning/r9-permissions-and-test-plan.md`.

**The boundary to reproduce exactly (D-15):** AC 2.2, lines 84-88. Line 87 is the in-scope sentence, line 88 the out-of-scope sentence including "Because cryptographic keys are held within kernel memory, a compromised kernel supervisor inevitably exposes keys" and the SEV-SNP / TDX remedy. The extracted form is AC-T01, `.planning/intel/constraints.md:211-214`.

**AC**
| Section | Lines | Kind | Content |
|---------|-------|------|---------|
| 1.1 | 33 | boundary claim | `port` "enforces `#![forbid(unsafe_code)]` across all modules" (not true at HEAD; D-06) |
| 1.1 | 35 | claim | bits 18-20 "are reserved in `sys/include/libc.h` and `x86_64/src/proc.rs`" (false at HEAD; note N3) |
| 1.2 | 52 | protection | MAC primitive row: "Keyed BLAKE3 / Poly1305 Encrypt-then-MAC" |
| 2.1 | 66-70, 79, 81-82 | defects as threats | key derivation, read-only child keys, decrypt-then-MAC, session key discarded, `#G` user bytes as kernel headers, unchecked note stack pointer |
| 2.2 | 84-88 | boundary | the threat model boundary |
| 3.1 | 96-98 | asset, boundary | keys in `Zeroizing<[u8; 32]>`; `ReadOnly` has no `k_mac` |
| 3.3 | 110-112 | protection | six vector registers; AVX-512 disabled in XCR0 |
| 3.4 | 117-119 | protection | `UninitializedFrame`; same-KeyID reuse skips zeroing; cross-domain reassignment "scrubbed with `CLFLUSHOPT`" |
| 5.1 | 230 | asset | `K_file = HKDF-Expand(K_user, Qid.path ‖ Qid.vers, 32)` |
| 5.2 | 241 | asset, boundary | `StatelessCap` with an 8-byte BLAKE3 MAC; "Any cluster replica sharing the domain key" validates |
| 5.2 | 253 | protection | failover sweep of unreclaimed handles |
| 5.3 | 258-259 | protection | AAD envelope and 32-byte keyed BLAKE3 MAC |
| 5.3 | 263 | boundary | `EXT_CAP_TOKEN`: several processes share one pooled connection, checked against a revocation list |
| 5.5 | 278 | protection | client seals dirty blocks with "spatial nonces (CoreID ‖ PageOffset ‖ Epoch)" |
| 5.7 | 288-289 | boundary | ciphertext DMA into `UninitializedFrame`, decryption deferred; user-space ring bypass through `segattach` with `#s` capabilities |
| 5.8 | 293-297 | boundary, protection | legacy peers; plaintext only in pre-pinned bounce frames; `rfork(RFNOLEGACY)` forbids `CHAN_UNTRUSTED_WIRE` channels |
| 5.9 | 307, 319-325, 340-342 | boundary | `cmd/discover` authenticates a discovered resource before registering it; the beacon format has no authentication field; staged health states |

**RFC**
| Section | Lines | Kind | Content |
|---------|-------|------|---------|
| Abstract | 14 | protection | AEAD envelope summary |
| 2 | 52 | attacker | AAD authenticated "to prevent packet tampering and splicing attacks" |
| 3.1 | 63-68 | protection | ChaCha20 for confidentiality (67), keyed BLAKE3 for integrity and authenticity (68) |
| 3.2 | 72 | asset | root seed and private heap keys never cross the wire |
| 3.2 | 74-76 | boundary | `Tauth` before `Tattach`; factotum runs the key exchange ("Noise Protocol handshake or mutual DP9IK ticket exchange") |
| 3.2 | 77-80 | asset | `K_transient`, described as bound to connection and direction; formula at 80 binds to `fid` |
| 3.2 | 82-85 | asset | per-object key binding |
| 3.2 | 87-88 | protection | forward secrecy; keys dropped and zeroized on `Tclunk` |
| 4.2 | 146-147 | protection | monotonic nonce; receivers "MUST reject any packet whose sequence counter does not strictly match the expected next", "preventing packet replay and out-of-order injection" |
| 4.3 | 159-165 | attacker, protection | AAD definition; "network adversaries cannot execute splicing attacks ... or replay attacks"; on MAC failure terminate and zero state "without processing any enclosed bytes" |
| 4.4 | 184-185 | boundary | `EXT_CAP_TOKEN`, 32 bytes, HMAC-BLAKE3 |
| 5.2 | 239-243 | asset | `StatelessCap`: 8-byte HMAC-BLAKE3 tag under "a server cluster secret" |
| 6.2.3 | 356 | availability | the server blocks a conflicting write until acks arrive or the lease times out |
| 6.2.4 | 368, 374 | protection | reclaim validates the client's capability token; unreclaimed handles freed and keys zeroized |
| 9.1 | 485 | boundary | on receipt the NIC path "validates the BLAKE3 MAC using K_transient" |
| 10.2 | 527-533 | protection | legacy gateway bounce frames, scrubbed after use |
| 10.3 | 535-537 | attacker, protection | `RFNOLEGACY`, "to prevent accidental data leakage from cryptographically protected processes to cleartext networks" |
| 11 | 541-550 | all four items | keystream reuse and re-key before 2^64 - 1 (543); speculative-execution side channels, `VPXOR` scrub, zeroize and `CLFLUSHOPT` on cross-KeyID reassignment (545); splicing and replay (547); denial of service through lease retention, 5-second timeout (549) |

**IG**
| Section | Lines | Kind | Content |
|---------|-------|------|---------|
| 2 | 29 | boundary | closed syscall list ("3 flag bits", corrected by D-05) |
| 2 | 33 | boundary | kernel safety boundary (amended by D-06) |
| 2 | 34 | protection | independent CR3 per sibling plus intra-process PKEYs |
| 2 | 37 | protection | native wire security; "Cleartext fallbacks use pinned bounce buffers" |
| 3 | 49-53 | defects | KDF, child key, no encryption, unsound asm, decrypt-then-MAC |
| 3 | 61 | defect | session key discarded, "zero wire authentication" |
| 3 | 63 | attacker | "raw user bytes became kernel VirtIO headers" |
| 3 | 64 | attacker | "Malicious processes could aim SP into read-only Block A" |
| 3 | 65 | protection | `UserSlice` residency enforced on demand |
| 4.1 | 75-93 | asset | `KeyCapability` typestates |
| 4.3 | 110-111 | protection | registers zeroed with `VPXOR`; AVX-512 disabled |
| 4.4 | 115-121 | protection | frames "popped directly from free pools without scrubbing"; PTE unmapped until overwrite and MAC verify |
| 6.2 | 187, 190 | asset | `StatelessCap`; failover sweep |
| 6.3 | 195, 203 | protection | envelope with MAC labelled "BLAKE3 / Poly1305"; legacy gateway "Sandboxed via `rfork(RFNOLEGACY)`" |
| 8 | 258 | scope | Phase 0 row: "Threat model doc, corrected C prototypes, reserve `rfork` flag bits 18–20 in `libc.h`" |
| 11 | 343-345, 352 | verification | lint contracts, unsafe audit, `q35crypt` "probing tamper kills, sibling isolation, note spoofing"; pinned user frames must not be re-encrypted mid-flight |

**PT**
| Section | Lines | Kind | Content |
|---------|-------|------|---------|
| 1 | 24-34 | boundary | topology; `cpu05` hosts `eve` "(adversary)" on the grid fabric |
| 2.2 | 77-81 | boundary | VLAN 10 storage, VLAN 20 grid, VLAN 30 terminal |
| 3.1 | 170 | attacker | `eve` (UID 1005): "Adversary / Probe: Probes unauthorized reads, attempts note spoofing and memory snooping" |
| 3.1 | 173 | boundary | `none` (UID 9999), "Unauthenticated fallback for public reads" |
| Test 1.1-1.3 | 220, 242, 262 | protection | discretionary file sharing and group leadership |
| Test 2.3 | 296-304 | attacker | in-line packet tap on VLAN 20; replay with a stale nonce; FID edited in the cleartext header |
| Test 3.1 | 310-321 | boundary | two auth domains, distinct transient keys |
| Test 3.2 | 324-332 | boundary | unregistered cross-domain user attenuated to UID 9999 |
| Test 4.1 | 338-347 | attacker, protection | sibling reads another sibling's heap; MMU trap, MAC failure, note `sys: crypto integrity violation` |
| Test 4.2 | 349-354 | protection | read-only base through PKEY 1 / `WRPKRU` |

**Contradictions and tensions between sources (each should appear in the document's open-points section):**
1. **MAC primitive.** RFC 3.1 line 68 says keyed BLAKE3; IG 6.3 line 195 and AC 1.2 line 52 say "BLAKE3 / Poly1305". Open as W3; do not resolve here (D-02).
2. **Session key scope.** RFC 3.2 prose (77-79) says connection and direction; the formula on line 80 binds to `fid`; IG ledger line 61 says per-connection, per-direction. Open as W4.
3. **Stale-frame protection mechanism.** IG 4.4 (118-121): no scrubbing at all, rely on unmapped PTE and full overwrite. AC 3.4 (119): cross-domain frames "scrubbed with `CLFLUSHOPT`". RFC 11 (545): such frames "MUST undergo hardware zeroization and cache line flushing". `CLFLUSHOPT` flushes cache lines and does not zero memory, so the three statements describe different mechanisms for the in-scope item "stale-frame data disclosure across distinct process KeyIDs".
4. **"Only the shadow frame contains plaintext"** (AC 2.2 line 87) against the legacy gateway, where plaintext sits in kernel bounce frames (AC 5.8 line 296, RFC 10.2 lines 531-533).
5. **`StatelessCap` contents and tag length.** AC 5.2 line 241 lists `qid_path`, `qid_vers`, `mode`, `epoch` and an 8-byte MAC; RFC 5.2 line 243 lists the file's hash, permissions and an 8-byte tag. Both give a 64-bit tag while the packet MAC is 32 bytes and `EXT_CAP_TOKEN` is 32 bytes. Related open item W2.
6. **Where the wire MAC is verified.** RFC 9.1 line 485 verifies on packet receipt; IG ledger line 53 verifies ciphertext "in cache before decrypting into shadow frames". These may be two checks with two keys; the sources do not say.
7. **False statements about HEAD:** AC 1.1 lines 33 and 35 (note N2, note N3).
8. **ChaCha20 parameters:** RFC 3.1 line 67 gives a 64-bit counter and a 96-bit nonce, which exceed the cipher's 128 bits of counter plus nonce (note N1).

**Absent from every source, so new in the document and needing the visible mark:** an asset list as such; attacker classes with stated capabilities (the sources name only "network adversaries", "malicious processes", "malicious or unresponsive clients" and the persona `eve`); a trust-boundary list; a table mapping protection to attacker; any treatment of beacon spoofing on the discovery multicast groups; any treatment of denial of service other than lease retention; the trust placed in `factotum` and in the holder of the cluster secret; side channels other than the vector-register and cache-line items in RFC 11; and the status of each protection in code (none exists at HEAD: there is no `crypto/`, no `ds/`, no AEAD wire code, verified by the workspace member list `["aarch64", "x86_64", "port", "riscv64", "xtask"]`).

**Layout recommendation (discretion):** (1) status banner: living document, nothing locked, no protection implemented (D-16); (2) scope boundary, quoted from AC 2.2; (3) assets; (4) attacker classes and capabilities; (5) trust boundaries; (6) protection table with columns protection, attacker addressed, source, delivering phase, status; (7) out of scope and the remedy; (8) open points and source contradictions. Use bracketed citations (`[AC 2.2]`, `[RFC 4.3]`, `[IG 3 row 16]`, `[PT 2.3]`) and one fixed mark for new statements, for example `[new]`, defined once at the top.

### Q7. DSGN-05: current text, source rows, and every wording site

**PROJECT.md Key Decisions today** [VERIFIED: `.planning/PROJECT.md:112-128`, read]. Lead-in, line 114: "No ADR exists and nothing is locked in this ingest. The guide's "non-negotiable" decisions (IG-D01 to IG-D13) are carried as constraints, not locked; whether to lock them is confirmed in Phase 2 (DSGN-05)."

Only five rows concern the thirteen decisions, and one of them covers two:
| Line | Decision cell | Rationale cell | Outcome cell |
|------|---------------|----------------|--------------|
| 122 | `libstd_r9` precedes Track P | Avoids rewriting Harvey userland twice (first `no_std`, then idiomatic) (IG section 9.1) | Carried from IG-D04; confirm in Phase 2 |
| 124 | Succinct TPHT is primary concurrent hashing; Roaring is the set algebra; Funnel is scratchpad only | IG-D09, IG-D10 | Carried constraint; confirm in Phase 2 |
| 125 | Native `9P2000.aead` is the distributed wire security | IG-D11; details open at W1-W4 | Carried constraint; confirm in Phase 2 |
| 126 | Dual C runtime: classic APE via ported 6c, APEX via Clang, never mixed | IG-D06 | Carried constraint; confirm in Phase 2 |
| 127 | Build priority: `ds/` and `9P2000.aead` upfront (guide Phases 1 and 2, which are GSD Phases 3 and 4) | IG-D01: Plan 9 is a network file fabric | Carried constraint; confirm in Phase 2 |

Line 123 is the `RFNOLEGACY` row ("... the four bits are reserved in r9 only (`x86_64/src/proc.rs`, `aarch64/src/syscall.rs`) ...", outcome "Decided 2026-11-22"). **There is no row for IG-D02, IG-D03, IG-D05, IG-D07, IG-D08, IG-D12 or IG-D13.** Row 124 mixes IG-D09 (not locked, D-07) with IG-D10 (locked, D-04) and must be split.

**Source rows.** The guide's table carries no IDs; `.planning/intel/constraints.md` assigns them.
| ID | Guide row (`r9-implementation-guide-amended-v3.md`) | `constraints.md` heading line | Proposed status (D-03 to D-07) |
|----|------------------------------------------------------|-------------------------------|--------------------------------|
| IG-D01 | 27 Build Priority | 17 | Locked as written |
| IG-D02 | 28 Executable Binary Format | 22 | Locked as written |
| IG-D03 | 29 System Call Interface | 27 | Locked, wording corrected to four flag bits (D-05) |
| IG-D04 | 30 Native Rust Runtime | 32 | Locked as written |
| IG-D05 | 31 C Toolchain & Compilers | 37 | Locked as written |
| IG-D06 | 32 Dual C Runtime Model | 42 | Locked as written |
| IG-D07 | 33 Kernel Safety Boundary | 47 | Locked amended to HEAD (D-06) |
| IG-D08 | 34 Memory Isolation Model | 52 | Locked as written |
| IG-D09 | 35 Primary Concurrent Hashing | 57 | Not locked, deferred to Phase 3 (W5, W6) |
| IG-D10 | 36 Integer Set Algebra | 62 | Locked as written |
| IG-D11 | 37 Distributed Wire Security | 67 | Not locked, deferred to Phase 4 (W1-W4) |
| IG-D12 | 38 3D Graphics Architecture | 72 | Locked as written |
| IG-D13 | 39 Cluster Service Discovery | 77 | Not locked, deferred to Phase 6 (W7) |

The two texts that change, verbatim from the guide:
- Row 29 (IG-D03): "No POSIX emulation calls or `mmap` extensions are permitted. Software adapts to Plan 9 primitives (`rfork`, `bind`, `mount`, 9P). Crypto extensions add 3 flag bits to `rfork` and zero syscalls." Landing sites: `sys/include/libc.h`, `sys/src/syscall.rs`.
- Row 33 (IG-D07): landing site "`port/` (`#![forbid(unsafe_code)]`)"; "Low-level hardware operations (`WRPKRU`, `MOVNTDQ`, `CLFLUSHOPT`, inline assembly, raw atomics) are restricted to `x86_64/`, `aarch64/`, and `ds/src/tpht/` behind audited `// SAFETY:` blocks."

**HEAD facts the IG-D07 row must match.** `port/src/lib.rs` declares per-module forbids; three modules lack one. Its own header comment words this as "EXCEPT the two named floor modules — `globalalloc` ... and `mcslock` ... plus the ONE named phase-08 bridge module `devip_waker`" [VERIFIED: `port/src/lib.rs:10-19`]. D-06 calls all three "floor modules". Lock D-06's wording and, to keep the row and the code comment from appearing to disagree, add that `lib.rs` labels `devip_waker` a bridge. The workspace lint `undocumented_unsafe_blocks = "deny"` is opted into by `aarch64` and `port` only [VERIFIED: `Cargo.toml:5-12`; `x86_64/Cargo.toml` has `[lints.rust]` only].

**Guide rows carry landing sites that do not exist at HEAD** (for example `port/src/fid.rs`, `xtask/src/elf2aout.rs`, `kern/src/gpu.rs`). "Locked as written" should be understood, and preferably worded, as locking the decision and not the file paths, which STATE.md already says must be re-verified per phase. This is a wording point for the owner checkpoint, not a new decision.

**Every line that must change:**
| File | Line | Decision | Change |
|------|------|----------|--------|
| `.planning/PROJECT.md` | 114 | D-03 | Replace the "nothing is locked" lead-in with the Phase 2 result and the D-08 rule (a change is a new dated row) |
| `.planning/PROJECT.md` | 122, 126, 127 | D-04 | Outcome becomes locked (IG-D04, IG-D06, IG-D01) |
| `.planning/PROJECT.md` | 124 | D-04, D-07 | Split: IG-D10 locked; IG-D09 "not locked, deferred to Phase 3" |
| `.planning/PROJECT.md` | 125 | D-07 | "not locked, deferred to Phase 4" |
| `.planning/PROJECT.md` | new rows | D-04..D-07 | IG-D02, IG-D03 (four flags), IG-D05, IG-D07 (amended), IG-D08, IG-D12, IG-D13 (not locked, Phase 6) |
| `.planning/PROJECT.md` | 123 | D-17 | Names the two arch files; change to the one `port/` module. It also says "Phase 4 (GSD Phase 6) implements"; leave |
| `.planning/PROJECT.md` | 105 (N3) | D-17 | "Phase 2 therefore reserves all four bits ... in `x86_64/src/proc.rs` and `aarch64/src/syscall.rs`" and the line references 171-181 and 92-108 |
| `.planning/PROJECT.md` | 77 | D-06 | "`port/` and `crypto/` forbid `unsafe`" repeats the guide wording; align with the amended IG-D07 |
| `.planning/PROJECT.md` | 76 | D-05 | Already says four bits; optionally name the home |
| `.planning/PROJECT.md` | 28, 131 | — | Active item for Phase 2 and the "Last updated" footer, at phase close |
| `.planning/ROADMAP.md` | 147 | D-17 | Success criterion 2 names the two arch files |
| `.planning/ROADMAP.md` | 149 | D-10 | Criterion 4: "landing location decided" can name `sys/include/rfork.h` |
| `.planning/ROADMAP.md` | 152 | D-17, D-23 | "Touching `x86_64/src/proc.rs` may or may not change kernel bytes" is answered: it does, `.text` unchanged |
| `.planning/REQUIREMENTS.md` | 27 | D-17 | DSGN-03 names the two arch files |
| `.planning/REQUIREMENTS.md` | 26 | D-10 | DSGN-02 landing location can be recorded |
| `.planning/STATE.md` | 71 | D-17 | "Reserved in r9 only (`x86_64/src/proc.rs`, `aarch64/src/syscall.rs`)" |
| `.planning/STATE.md` | 77 | D-03 | "Nothing is locked ... pending confirmation in Phase 2" |
| `.planning/STATE.md` | 142 | — | Stale "owed check" hash `sha256-TIzw...`; optional correction while the file is open |

`grep` for `x86_64/src/proc.rs` or `aarch64/src/syscall.rs` across the four files returns exactly: ROADMAP 147 and 152, REQUIREMENTS 27, PROJECT 105 and 123, STATE 71. `REQUIREMENTS.md:28` ("highest bit in use today is 14") stays true and needs no change. `.planning/intel/constraints.md:30` still says "3 flag bits [see W11]"; it is an extract of a source document and the four files above are the ones CONTEXT names, so leave it unless the owner asks.

### Q8. Harvey's enum and declaration; `rfork.h` content

`/Users/neilskinner/code/harvey/sys/include/libc.h` at Harvey HEAD `c697666b` [VERIFIED: read lines 608-627 and 680-686]:
```c
/* rfork */
enum
{
	RFNAMEG		= (1<<0),
	RFENVG		= (1<<1),
	RFFDG		= (1<<2),
	RFNOTEG		= (1<<3),
	RFPROC		= (1<<4),
	RFMEM		= (1<<5),
	RFNOWAIT	= (1<<6),
	RFCNAMEG	= (1<<10),
	RFCENVG		= (1<<11),
	RFCFDG		= (1<<12),
	RFREND		= (1<<13),
	RFNOMNT		= (1<<14)
};
```
Line 610 is the comment, 611 `enum`, 612 `{`, enumerators 613-624, 625 `};`. Line 684 is `extern	int	rfork(int);` (tab separated). These match D-11's citations. Whitespace rule in Harvey: a leading tab, then two tabs after names shorter than eight characters and one tab otherwise; the last enumerator has no comma. The file has no include guard (0 matches for `#ifndef` or `#pragma once`).

**Generated file, as produced in scratch:**
```c
/*
 * rfork(2): the flag bits and the declaration.
 *
 * GENERATED by `cargo xtask rfork-header` from port/src/rfork.rs.
 * Do not edit; change the Rust constants and regenerate.
 *
 * RFNAMEG to RFNOMNT mirror Harvey sys/include/libc.h:613-624 and
 * the declaration mirrors libc.h:684.  The flags after RFNOMNT are
 * reserved by r9 and are in no Harvey header.
 */
enum
{
	RFNAMEG		= (1<<0),
	RFENVG		= (1<<1),
	RFFDG		= (1<<2),
	RFNOTEG		= (1<<3),
	RFPROC		= (1<<4),
	RFMEM		= (1<<5),
	RFNOWAIT	= (1<<6),
	RFCNAMEG	= (1<<10),
	RFCENVG		= (1<<11),
	RFCFDG		= (1<<12),
	RFREND		= (1<<13),
	RFNOMNT		= (1<<14),
	/* r9 only: reserved, rejected by rfork until implemented */
	RFCKEY		= (1<<18),
	RFROBASE	= (1<<19),
	RFRANDMEM	= (1<<20),
	RFNOLEGACY	= (1<<21)
};

extern	int	rfork(int);
```
The first twelve enumerator lines are byte-identical to Harvey's except that `RFNOMNT` gains a trailing comma. The banner wording is discretion. Note that `RFNOMNT` is also rejected by r9 today; the "r9 only" comment line describes the four below it, so the banner or a second comment may want to say that `RFNOMNT` is defined by Harvey and not implemented by r9.

## Common Pitfalls

### Pitfall 1: Comparing kernels built on different venues
**What goes wrong:** a Mac "before" hash is compared with a Linux `kernel-x64.elf` hash and the mismatch is read as a regression.
**Why:** the kernel `.rodata` carries toolchain store-path strings (`flake.nix:230-232`); the toolchain derivation differs per system.
**Avoid:** build before and after on the same machine. Record the Linux `kernel-x64.elf` sha256 separately as the hash that pairs with the new `outputHash`.

### Pitfall 2: A test that is green because it never ran
**What goes wrong:** the x86_64 mask test is reported as passing from a `cargo xtask test` run that skipped the x86_64 crate.
**Why:** no `rustup` in the devshell and an aarch64-only fallback; also `cargo xtask test` never runs xtask's own tests.
**Avoid:** require the test's own `... ok` line in the evidence, from `--verbose` output showing the `x86_64` leg. Run `cargo test -p xtask` explicitly for the renderer test.
**Warning sign:** test counts unchanged after adding tests.

### Pitfall 3: `RFPROC` in the x86_64 test module
**What goes wrong:** deleting the constants from `x86_64/src/proc.rs` breaks `mod tests` at line 3253, which reaches `RFPROC` through `use super::*`. The kernel build and `cargo xtask check` on the Mac do not compile that module, so the break is first seen on the owner's host at the end of the phase.
**Avoid:** import `RFPROC` inside `mod tests`. Importing it at file level instead produces an unused-import warning in the kernel build. This Mac cannot compile the module; treat the first x86_64 host run as the check and keep a fallback ready.

### Pitfall 4: The header reproduces a wrong constant faithfully
**What goes wrong:** the staleness check only proves the header equals the Rust table.
**Avoid:** `port/tests/rfork_test.rs` pins the twelve Harvey values literally and in order, pins the four new values, and pins the union `0x3c7c7f`.

### Pitfall 5: `port/src/rfork.rs` grows a `crate::` path
**What goes wrong:** xtask stops compiling, because the file is also a module of xtask.
**Avoid:** a header comment in the file says so (it is in the example). `cargo xtask` itself failing to build is the tripwire.

### Pitfall 6: The hash commit and the "all work committed" rule
**What goes wrong:** the gate is run, then the `outputHash` is changed, so the commit that was gated is not the phase's last kernel-relevant commit.
**Avoid:** order the owner stop as hash mismatch, commit the new hash, build and `--rebuild`, host tests, then the gate. The gate builds `q35cpu` from source and does not read `outputHash`, but D-25 says the gate runs after all work is committed.

### Pitfall 7: `--rebuild` before a plain build
**What goes wrong:** `nix build .#boot-iso --rebuild` fails with "some outputs ... are not valid, so checking is not possible".
**Avoid:** plain build first (the witness `kernel` step already does this; `run-witness.sh:455-475`).

### Pitfall 8: Using aarch64 clippy as a criterion
**What goes wrong:** `cargo xtask clippy --arch aarch64` exits 1 at unmodified HEAD (4 "unsafe block missing a safety comment" errors), so it would block the phase for an unrelated reason.
**Avoid:** use `cargo xtask clippy` (default arch x86_64, the form CI runs), which exits 0 with the edit. Report the aarch64 result as pre-existing if it comes up.

### Pitfall 9: Treating the threat model as locked or as implemented
**Avoid:** D-16. The status column says "not implemented" for every protection, and the word "locked" does not appear in the document.

### Pitfall 10: Missing one wording site
**Avoid:** end the documentation task with `grep -n 'x86_64/src/proc.rs\|aarch64/src/syscall.rs' .planning/{ROADMAP,REQUIREMENTS,PROJECT,STATE}.md` and `grep -c 'confirm in Phase 2' .planning/PROJECT.md` (expect 0).

## Code Examples

Everything in this section was compiled and run in the scratch copy unless marked.

### `port/src/rfork.rs`
```rust
//! rfork(2) flag bits: the one definition both arch crates import.
//!
//! Bits 0-6 and 10-14 are Harvey's frozen values (Harvey
//! `sys/include/libc.h:613-624`).  Bits 18-21 are reserved by r9 and
//! exist in no Harvey header.  `RFNOMNT` and the four reserved flags
//! are named here but are in neither arch's `RFORK_SUPPORTED` mask, so
//! rfork rejects them with "rfork: unsupported flags".
//!
//! This file is also compiled into xtask (`#[path]`) to generate
//! `sys/include/rfork.h`, so it must stay free of `crate::` paths and
//! of anything outside `core`.

macro_rules! rfork_flags {
    ($( $(#[$doc:meta])* $name:ident = 1 << $bit:literal; )*) => {
        $( $(#[$doc])* pub const $name: u64 = 1 << $bit; )*
        /// Every flag as (C name, value), in declaration order.
        pub const ALL: &[(&str, u64)] = &[ $( (stringify!($name), $name), )* ];
    };
}

rfork_flags! {
    RFNAMEG = 1 << 0;
    RFENVG = 1 << 1;
    RFFDG = 1 << 2;
    RFNOTEG = 1 << 3;
    RFPROC = 1 << 4;
    RFMEM = 1 << 5;
    RFNOWAIT = 1 << 6;
    RFCNAMEG = 1 << 10;
    RFCENVG = 1 << 11;
    RFCFDG = 1 << 12;
    RFREND = 1 << 13;
    /// Defined by Harvey, not implemented by r9.
    RFNOMNT = 1 << 14;
    /// Reserved (Phase 7).
    RFCKEY = 1 << 18;
    /// Reserved (Phase 7).
    RFROBASE = 1 << 19;
    /// Reserved (Phase 7).
    RFRANDMEM = 1 << 20;
    /// Reserved (Phase 6).
    RFNOLEGACY = 1 << 21;
}

/// Index into `ALL` of the first flag Harvey does not define.
pub const FIRST_R9_ONLY: usize = 12;
```

### Arch side (identical text in both `syscall.rs` files)
```rust
use port::rfork::{RFCENVG, RFCFDG, RFCNAMEG, RFENVG, RFFDG, RFMEM, RFNAMEG, RFNOTEG, RFNOWAIT, RFPROC, RFREND};

/// The rfork flags this kernel implements.  Any other bit is rejected
/// with "rfork: unsupported flags".
pub(crate) const RFORK_SUPPORTED: u64 = RFPROC
    | RFMEM
    | RFFDG
    | RFNOWAIT
    | RFREND
    | RFNOTEG
    | RFNAMEG
    | RFCNAMEG
    | RFENVG
    | RFCENVG
    | RFCFDG;

// inside sys_rfork, replacing the local const and its use:
    if flag & !RFORK_SUPPORTED != 0 {
        return err(proc, "rfork: unsupported flags");
    }

#[cfg(test)]
mod rfork_mask_tests {
    use super::RFORK_SUPPORTED;
    use port::rfork::{RFCKEY, RFNOLEGACY, RFNOMNT, RFRANDMEM, RFROBASE};

    #[test]
    fn rfork_reserved_bits_are_unsupported() {
        for (name, bit) in [
            ("RFNOMNT", RFNOMNT),
            ("RFCKEY", RFCKEY),
            ("RFROBASE", RFROBASE),
            ("RFRANDMEM", RFRANDMEM),
            ("RFNOLEGACY", RFNOLEGACY),
        ] {
            assert_eq!(RFORK_SUPPORTED & bit, 0, "{name} must stay outside RFORK_SUPPORTED");
        }
        assert_eq!(RFORK_SUPPORTED, 0x3c7f, "the implemented set is bits 0-6 and 10-13");
    }
}
```
Ran green in the aarch64 crate (both cfg legs). The x86_64 copy built into the kernel but its test module was not compiled here.

### `port/tests/rfork_test.rs`
```rust
use port::rfork::*;

#[test]
fn rfork_reserved_values_are_pinned() {
    assert_eq!(RFCKEY, 1 << 18);
    assert_eq!(RFROBASE, 1 << 19);
    assert_eq!(RFRANDMEM, 1 << 20);
    assert_eq!(RFNOLEGACY, 1 << 21);
}

#[test]
fn rfork_harvey_values_are_pinned() {
    let harvey: [(&str, u64); 12] = [
        ("RFNAMEG", 1 << 0), ("RFENVG", 1 << 1), ("RFFDG", 1 << 2), ("RFNOTEG", 1 << 3),
        ("RFPROC", 1 << 4), ("RFMEM", 1 << 5), ("RFNOWAIT", 1 << 6), ("RFCNAMEG", 1 << 10),
        ("RFCENVG", 1 << 11), ("RFCFDG", 1 << 12), ("RFREND", 1 << 13), ("RFNOMNT", 1 << 14),
    ];
    assert_eq!(&ALL[..FIRST_R9_ONLY], &harvey[..]);
    assert_eq!(ALL.len(), 16);
}

#[test]
fn rfork_flags_do_not_overlap() {
    let mut seen = 0u64;
    for (name, bit) in ALL {
        assert!(bit.is_power_of_two(), "{name} is not a single bit");
        assert_eq!(seen & bit, 0, "{name} overlaps an earlier flag");
        assert!(*bit <= u32::MAX as u64, "{name} does not fit rfork's int argument");
        seen |= bit;
    }
    assert_eq!(seen, 0x3c7c7f);
}
```

### `xtask/src/rforkh.rs` (core)
```rust
use std::fs;
use std::path::Path;

use crate::Result;

#[path = "../../port/src/rfork.rs"]
#[allow(dead_code)]
mod flags;

pub const HEADER_PATH: &str = "sys/include/rfork.h";

pub fn render() -> String {
    let mut out = String::new();
    // ... banner lines ...
    out.push_str("enum\n{\n");
    let last = flags::ALL.len() - 1;
    for (i, (name, value)) in flags::ALL.iter().enumerate() {
        assert!(value.is_power_of_two(), "{name} is not a single bit");
        if i == flags::FIRST_R9_ONLY {
            out.push_str("\t/* r9 only: reserved, rejected by rfork until implemented */\n");
        }
        let tabs = if name.len() < 8 { "\t\t" } else { "\t" };
        let comma = if i == last { "" } else { "," };
        out.push_str(&format!("\t{name}{tabs}= (1<<{}){comma}\n", value.trailing_zeros()));
    }
    out.push_str("};\n\nextern\tint\trfork(int);\n");
    out
}

pub fn write(workspace: &Path) -> Result<()> { /* create_dir_all, fs::write(render()) */ }

pub fn check(workspace: &Path) -> Result<()> {
    let path = workspace.join(HEADER_PATH);
    let committed = fs::read_to_string(&path)
        .map_err(|e| format!("rfork-header: read {}: {e} (run `cargo xtask rfork-header`)", path.display()))?;
    if committed != render() {
        return Err(format!(
            "rfork-header: {} is stale against port/src/rfork.rs (run `cargo xtask rfork-header`)",
            path.display()
        ).into());
    }
    Ok(())
}
```
Wiring in `xtask/src/main.rs` (all three ran): a `rfork-header` subcommand with `clap::arg!(--check "...")`; a match arm calling `xtask::rforkh::check(&workspace())` or `write`; and `xtask::rforkh::check(&workspace())?;` as the first statement of `TestStep::run`. `workspace()` is the existing helper at `xtask/src/main.rs:4744`.

### The x86_64 fallback (D-24) [ASSUMED: not run on an x86_64 host]
```rust
        if target.is_none() && (arch == "aarch64" || arch == "x86_64") {
            target = host_triple();
        }
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Two arch-local copies of the flags | One `port` module | This phase (D-17) | Later phases add behaviour by editing an arch mask, not by defining a constant |
| Guide: "reserve bits 18-20 in `libc.h`" | Reserve four bits in r9 only | Owner 2026-11-22 | Recorded deviation |
| Docker container for the x64 TCG gates | Owner's native amd64 host | Owner-approved at stop B2 (`flake.nix:980-982`) | The container is not needed when the host is amd64 |
| Per-commit quick gate (Phase 1 D-12) | One gate run at phase end | D-25, this phase only | — |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The x86_64 mask test module and the `RFPROC` import inside `x86_64/src/proc.rs` `mod tests` compile and pass on an x86_64 std host | Q2, Q3, Pitfall 3 | Found only at the end-of-phase owner stop; a small fix and a second owner run |
| A2 | Widening the `TestStep` fallback makes the x86_64 leg run in an x86_64 Linux devshell, and the nix toolchain there ships std for `x86_64-unknown-linux-gnu` (by symmetry with `aarch64-apple-darwin` here, and the legacy record of the explicit command working) | Q2 | Owner uses the explicit `cargo test --package x86_64 --bins --target ...` command instead |
| A3 | All 139 existing x86_64 host tests pass on the owner's host at HEAD | Q2 | A pre-existing failure appears at the stop; discriminate by running at the base commit |
| A4 | The committed edit will have the same `.text` result as the scratch edit | Kernel Bytes | D-23 already makes a `.text` difference a stop; re-measure on the real commits |
| A5 | The Linux builder needed by `nix run .#boot-r9-*` on the Mac is up | Q4 | Quick pass waits on the owner starting the lima VM |
| A6 | The owner reads "locked as written" as locking decisions, not the guide's landing-site paths | Q7 | A wording choice at the checkpoint; no rework |

## Open Questions

1. **How the phase's commits reach the owner's x86_64 host.**
   - Known: `run-witness.sh` states "Nothing is copied from the Mac: the host gets every script from GitHub with `git pull` (or a clone) and returns results through a pushed branch or a pasted file. The Mac cannot reach that host." Pushes need owner approval. D-24 says no push is required "for verification", meaning hosted CI is not needed.
   - Unclear: whether the owner will push a branch, or move the commits another way (a bundle, for example).
   - Recommendation: make transport an explicit first item of the owner stop and let the owner choose. Do not push without approval.

2. **Whether the end-of-phase stop includes the full tier.**
   - Known: D-25 fixes one `cargo xtask gate` run. Phase 1 D-14 says the full set runs "at phase boundaries". The byte-change rule says "x64 TCG gates" and was last satisfied by one `cargo xtask gate --arch x86-64 --accel tcg`.
   - Recommendation: plan the stop as quick gate under KVM plus the quick gate under TCG, and put one yes/no line to the owner in the stop on `--full`. Do not add `--full` silently and do not drop it silently.

3. **One gate run or the witness step's two.**
   - Known: the `gate` witness step runs the quick gate twice; D-25 says once.
   - Recommendation: call `nix develop -c cargo xtask gate --arch x86-64 --accel kvm` directly, once, and use the witness driver only for the `kernel` step.

4. **Who writes the `outputHash` commit.**
   - Known: the new value exists only on the Linux host. Last time the owner committed it (`2bc5478`).
   - Recommendation: the owner pastes the `got:` line back, or commits on the host; either way it is a separate commit with a comment paragraph, made before the gate run.

5. **`rfork.h` banner wording about `RFNOMNT` and about not including the file beside `libc.h`.** Discretion; the owner sees the file in the diff.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `nix` | everything | yes | Determinate Nix 3.24.3 (nix 2.35.6) | — |
| `cargo` / `rustc` | builds, tests | devshell only (`which cargo` outside: not found) | rustc 1.95.0-nightly (842bd5be2 2026-01-29) | — |
| `$OBJCOPY` (llvm-objcopy) | section hashes | yes, exported by the devshell | from the pinned toolchain | the Python ELF reader used here |
| `/usr/bin/cc` | optional header syntax check | yes | Apple clang | skip |
| QEMU | aarch64 quick pass | yes (devshell) | 11.1.2 | — |
| `rustup` | — | not installed | — | not needed after the xtask fallback |
| Rosetta 2 | running x86_64 host tests locally | **no** ("Bad CPU type in executable") | — | owner's x86_64 host (D-24) |
| Harvey checkout | reading `libc.h` | yes, HEAD `c697666b` (11 known modified font files; untouched) | — | the nix-store tree |
| Docker | amd64 container for TCG gates | binary at `/usr/local/bin/docker`; daemon not probed | — | owner's native amd64 host (approved venue) |
| Owner's x86_64 Linux KVM host | `boot-iso`, host tests, KVM and TCG gate | not reachable from here | — | none |
| Linux builder (lima) | `nix run .#boot-r9-*` kernel packages on the Mac | not probed | — | none; owner starts it |

**Missing dependencies with no fallback:** the owner's x86_64 Linux host, for the whole end-of-phase stop.
**Missing dependencies with fallback:** none that block Mac-side work.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness; expect-driven boot gates |
| Config file | none (`.planning/config.json`: `build_command` `nix develop -c cargo xtask check`, `test_command` `nix develop -c cargo xtask test`) |
| Quick run command | `nix develop -c cargo xtask test` (after the wiring: header check, `port` tests, both aarch64 legs) |
| Full suite command | `nix develop -c cargo xtask test && nix develop -c cargo test -p xtask && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy` |
| Estimated runtime | quick: a few seconds once built; four kernel builds for byte evidence: under a minute cold |

`cargo xtask test` does not run xtask's own tests; `cargo test -p xtask` does. `cargo xtask clippy --arch aarch64` fails at HEAD and is excluded.

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DSGN-01 | Document exists, quotes the AC 2.2 boundary, names SEV-SNP and TDX, says no protection is implemented, has no uncited unmarked statement | doc check plus owner review | `test -f docs/threat-model.md && grep -c 'SEV-SNP' docs/threat-model.md` and a grep for the status sentence | ❌ Wave 0 |
| DSGN-02 | `sys/include/rfork.h` equals the rendering of the Rust table; missing and stale files are rejected | CLI plus unit | `nix develop -c cargo xtask rfork-header --check`; `nix develop -c cargo test -p xtask rfork` | ❌ Wave 0 |
| DSGN-03 | Sixteen flags defined once in `port`; no definition left in the arch crates; all crates build | integration test, grep, build | `nix develop -c cargo test -p port --test rfork_test`; `! grep -rn 'const RF[A-Z]*: u64' x86_64/src aarch64/src`; `nix develop -c cargo xtask check` | ❌ Wave 0 |
| DSGN-04 (values) | Four values pinned; sixteen single bits, disjoint; bit 14 is `RFNOMNT` | integration test | `nix develop -c cargo test -p port --test rfork_test` | ❌ Wave 0 |
| DSGN-04 (aarch64 mask) | Five names outside `RFORK_SUPPORTED`; mask is `0x3c7f` | unit | `nix develop -c cargo xtask test` on the Mac, expecting `syscall::rfork_mask_tests::rfork_reserved_bits_are_unsupported ... ok` twice | ❌ Wave 0 |
| DSGN-04 (x86_64 mask) | Same, for x86_64 | unit, owner's host | `nix develop -c cargo xtask test --verbose` on x86_64 Linux after the fallback fix | ❌ Wave 0; manual venue |
| DSGN-04 (no code change) | `.text` identical before and after on four configs | build comparison | the commands in "Kernel Bytes"; a `.text` difference stops the phase | procedure |
| DSGN-04 (boots) | Reaches rio | gate, owner's host | `nix develop -c cargo xtask gate --arch x86-64 --accel kvm`, once, at phase end; aarch64 quick pass on the Mac | exists |
| DSGN-05 | PROJECT.md has a row for each of IG-D01..IG-D13 with locked, locked amended, or not locked and deferred | doc check plus owner checkpoint | `for i in 01 02 03 04 05 06 07 08 09 10 11 12 13; do grep -q "IG-D$i" .planning/PROJECT.md || echo MISSING $i; done`; `grep -c 'confirm in Phase 2' .planning/PROJECT.md` returns 0 | manual confirmation |

### Sampling Rate
- **Per task commit:** `nix develop -c cargo xtask test`; add `nix develop -c cargo test -p xtask rfork` when xtask changes.
- **Per wave merge:** the full suite command.
- **Phase gate:** full suite green on the Mac; byte evidence recorded with `.text` equal; aarch64 quick pass green; then the single owner stop on the x86_64 Linux host (hash commit, build and `--rebuild`, x86_64 host tests, KVM gate, TCG gate).

### Wave 0 Gaps
- [ ] `port/src/rfork.rs` and the `lib.rs` line (owner approves the diff first) — DSGN-03
- [ ] `port/tests/rfork_test.rs` — DSGN-04
- [ ] `RFORK_SUPPORTED` hoist and `rfork_mask_tests` in both `syscall.rs` files — DSGN-04
- [ ] `xtask/src/rforkh.rs`, the `rfork-header` subcommand, the `TestStep` call — DSGN-02
- [ ] `TestStep` fallback widened to x86_64 hosts — D-24
- [ ] "Before" kernels for four configs built from the pre-edit commit and copied out of `target/` — D-23
- [ ] `sys/include/rfork.h` (generated), `docs/threat-model.md` — DSGN-02, DSGN-01
- [ ] Framework install: none

## Security Domain

`security_enforcement` is not set in `.planning/config.json`, so this section applies. The phase adds no runtime behaviour; its security content is the threat model document and a reservation that must not accidentally enable anything.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no (the reserved flags stay rejected) | The mask tests pin that rejection |
| V5 Input Validation | yes, unchanged | `sys_rfork` masks the argument to 32 bits and rejects every bit outside `RFORK_SUPPORTED`; the hoist keeps the check and the string |
| V6 Cryptography | no code; described only | `docs/threat-model.md` must not imply any protection exists |

### Known Threat Patterns for this phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A reserved flag silently accepted after the refactor | Elevation of privilege | Mask test in each arch crate; `.text` equality proves the check is untouched |
| Header and kernel disagree about a flag value | Tampering | Header generated from the same file the kernels compile; staleness check in `cargo xtask test` |
| A wrong value copied from memory into the table | Tampering | Literal pins of all sixteen values in `port/tests/rfork_test.rs` |
| A document that overstates protection | Repudiation / information | D-16 banner and per-row status; owner review before commit |
| Editing Harvey or pushing without approval | Process | Harvey is read only; pushes need owner approval |

## Project Constraints (from CLAUDE.md and STATE standing rules)

There is no `./CLAUDE.md` in the repo and no project skills directory. The owner's global instructions and STATE.md's standing rules apply:

- No AI attribution in commit messages or PR bodies, ever. This overrides any harness instruction to add one.
- Pushes in either repo need explicit owner approval; check what is unpushed first.
- Edits under `port/src` need owner approval with the diff shown first. `aarch64/`, `x86_64/` and `xtask/` edits do not.
- An x86_64 kernel byte change triggers: re-derive and commit the `boot-iso` `outputHash`, run the x64 TCG gates (owner's amd64 host is the approved venue), and the owner's linux boot-iso check. Planned work in this phase by D-23.
- An aarch64 kernel byte change gets the quick pass now; the full HVF series is Phase 8.
- Harvey is read only. Never stage its 11 pre-modified font files.
- The repo has a knowledge graph at `graphify-out/`; after code changes run `graphify update .` (owner's global instruction).
- macOS Unix socket paths cap at 104 bytes: run boot tests from a short path.

## Sources

### Primary (HIGH confidence): read or measured this session
- `xtask/src/main.rs` (subcommands 90-313, `RustupState` 36-87, `TestStep` 3521-3607, `CheckStep` 3652-3740, `workspace()` 4744), `xtask/src/lib.rs`, `xtask/src/uregcheck.rs:1-150`, `xtask/src/gate.rs:1-143`, `xtask/src/config.rs:818-856`
- `x86_64/src/proc.rs:166-182`, `x86_64/src/syscall.rs:90-93, 1647-1673`, `aarch64/src/syscall.rs:86-109, 1956-1981`, both `main.rs` module lists
- `port/src/lib.rs`, `port/Cargo.toml`, `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`
- `flake.nix:233-237, 360-424, 594-655, 955-1069`; `.github/workflows/xtask.yml`, `gates.yml`
- `/Users/neilskinner/code/harvey/sys/include/libc.h:608-627, 680-686`
- `.planning/PROJECT.md` (whole), `ROADMAP.md:141-152`, `REQUIREMENTS.md` (whole), `STATE.md` (whole), `02-CONTEXT.md`
- `.planning/r9-architecture-and-code-v4.md:1-135, 197-345`; `r9-implementation-guide-amended-v3.md:1-125, 184-222, 250-262, 335-354`; `rfc-9p2000-aead.md` sections 2-4, 5.2, 6.2.3-6.2.4, 9-11; `r9-permissions-and-test-plan.md:6-34, 159-217, 296-358`; `intel/constraints.md:1-90, 211-214`; `INGEST-CONFLICTS.md:67-85`
- Scratch builds and tests: four kernel configs at HEAD (two paths) and with the edit; `cargo xtask test`, `check`, `clippy`, `rfork-header`; section and word-level diffs

### Secondary (MEDIUM confidence): project records
- `.planning/phases/01-stage-p0-cross-build-harness/witness/run-witness.sh`, `WITNESS-LOG.md`, `01-33-SUMMARY.md`, `01-34-SUMMARY.md`, `01-30-SUMMARY.md`, `01-CONTEXT.md:35-38`
- `.planning/legacy/phases/14-harvey-amd64-userspace-on-r9/14-01-SUMMARY.md`, `14-03-SUMMARY.md` (the x86_64 host-test command on Linux)
- `docs/harvey-supply.md`, `.planning/HANDOFF.md`

### Tertiary (LOW confidence)
- None. Nothing in this research rests on a web search or on recall alone; the unverified items are in the Assumptions Log.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. No new dependency; existing mechanisms only.
- Architecture (module, generator, tests): HIGH on the Mac side, compiled and run; the x86_64 host-test leg is unverified (A1, A2).
- Kernel bytes: HIGH. Measured on the real edit shape for four configs, with a reproducibility control.
- Owner-host procedure: MEDIUM. Taken from Phase 1 records and the script, not re-run.
- Threat-model inventory: HIGH for the cited lines; the list of what is absent is a judgement from reading the security-relevant sections, not every line of all four documents.

**Research date:** 2026-11-23
**Valid until:** the next commit that touches `x86_64/src/{proc,syscall}.rs`, `aarch64/src/{proc,syscall}.rs`, `xtask/src/main.rs` or `flake.nix` (line references and hashes go stale); otherwise 30 days.
