# Phase 2: Guide Phase 0 - Design Lock and rfork Flag Reservation - Research

**Researched:** 2026-11-23
**Domain:** Rust `no_std` kernel crate layout (`port/`), xtask build tooling, C header generation, reproducible-kernel evidence, design-document authoring
**Confidence:** HIGH (every code claim below was read from the file or measured in this session; the provenance tag on each claim says which)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Carried in (owner, 2026-11-22; not re-asked)**
- **D-01:** Four flags, not three: `RFCKEY` 1<<18, `RFROBASE` 1<<19, `RFRANDMEM` 1<<20, `RFNOLEGACY` 1<<21. Reserved in r9 only; Harvey is never edited. The guide's "reserve bits 18-20 in `libc.h`" is a recorded deviation.
- **D-02:** Warnings W1 to W10 stay open and are settled in the phase each affects. This phase does not pull any of them forward.

**Locking the guide's thirteen decisions (DSGN-05)**
- **D-03:** Confirmation is in bulk with exceptions only. The phase presents all thirteen (IG-D01 to IG-D13) in one table with a proposed status each; the owner confirms the table at one checkpoint and rules individually only on the exceptions below. The result is written to the PROJECT.md Key Decisions table.
- **D-04:** Proposed for lock as written: IG-D01, IG-D02, IG-D04, IG-D05, IG-D06, IG-D08, IG-D10, IG-D12.
- **D-05:** IG-D03 (closed syscall list) is locked with its wording corrected from "3 flag bits" to four, per D-01.
- **D-06:** IG-D07 (kernel safety boundary) is locked amended to what is true at HEAD: `port/` uses per-module `forbid(unsafe_code)` with three named floor modules that hold `unsafe` (`globalalloc`, `mcslock`, `devip_waker`); no new floor module without owner approval. The guide's "crate-level forbid" wording is not locked. How lint and audit scope grow when new `unsafe` lands (note N2) stays for Phases 3 and 5, and the `ds/src/tpht/` part of the wording stays open under W5.
- **D-07:** IG-D09 (TPHT as primary hashing; open at W5, W6), IG-D11 (native `9P2000.aead`; open at W1 to W4) and IG-D13 (cluster discovery; open at W7) are **not locked** in this phase. They remain carried constraints, and the lock is deferred until their W items are settled in Phases 3, 4 and 6. PROJECT.md must record them as "not locked, deferred to Phase N" so the phase gate is explicit about which are locked and which are not.
- **D-08:** "Locked" means owner-only change. Planning and execution treat a locked decision as fixed. If a later phase finds one wrong, work stops and the owner is asked. A change is recorded as a new dated row that supersedes the old one, never a silent edit.

**C prototypes (DSGN-02)**
- **D-09:** No source document defines the "corrected C prototypes". The owner's reading: the C statement of the whole `rfork` interface, that is every `RF*` flag (Harvey's frozen values, `RFNOMNT` included, plus the four new ones) and the `rfork()` declaration.
- **D-10:** The landing location is a new top-level `sys/include/` directory, matching the guide's own landing sites (`sys/include/libc.h`, `sys/src/...`), so the file does not move when Phases 9 to 12 fill in `sys/`.
- **D-11:** The file is a small r9-owned `sys/include/rfork.h` holding only the `RF*` enum and the `rfork()` declaration, with a comment naming the Harvey `libc.h` lines it mirrors (`sys/include/libc.h:613-624` for the enum, `:684` for `extern int rfork(int);`). It is not a copy of Harvey's `libc.h` and is not named `libc.h`.
- **D-12:** `rfork.h` is generated, not hand-maintained: an xtask step writes it from the Rust flag constants, and a check fails when the committed file is stale. The Rust constants are the single source of truth.

**Threat model document (DSGN-01)**
- **D-13:** The document is `docs/threat-model.md`, beside `docs/devip-contract.md` and `docs/harvey-supply.md`.
- **D-14:** Depth is a full analysis: assets, attacker classes and their capabilities, trust boundaries, and a protection table. It is more than a restatement of AC section 2.2.
- **D-15:** Every statement either cites its source (compendium section, RFC section, guide row) or is visibly marked as new in this document. The in-scope and out-of-scope boundaries are stated exactly as AC section 2.2 / AC-T01 (pages at rest, stale-frame disclosure across KeyIDs, stray kernel reads, packet splicing, injection and replay in scope; DRAM bus probing, cold boot and kernel-context code execution out of scope; SEV-SNP or TDX as the remedy). The owner reviews the draft at a checkpoint before it is committed as final.
- **D-16:** The document is a living document and nothing in it is locked, the boundary included. Later phases may revise it as designs firm up. It is therefore not part of the D-08 lock set. None of its protections exists in code yet, and it must say so.

**Reservation and test shape (DSGN-03, DSGN-04)**
- **D-17:** Owner amendment to the roadmap: the `rfork` flags get **one definition in the `port/` crate**, not two arch copies. ROADMAP Phase 2 success criterion 2 and DSGN-03 both name `x86_64/src/proc.rs` and `aarch64/src/syscall.rs` as the home; the plan must amend both texts (and the matching lines in PROJECT.md and STATE.md) to say the constants live in one `port/` module that both arch crates import. This is a `port/src` edit, so the diff is shown to the owner before it is committed.
- **D-18:** All `rfork` flags move: the eleven existing flags (bits 0-6, 10-13) and the four new ones live in the one `port/` module; the duplicate constants are deleted from `x86_64/src/proc.rs:171-181` and `aarch64/src/syscall.rs:92-108`, which import them. No re-export shims.
- **D-19:** `RFNOMNT` = 1<<14 is added to the same module as a named constant that stays outside both `SUPPORTED` masks, exactly like the four reserved flags. `rfork(RFNOMNT)` keeps failing as it does today. The module therefore defines sixteen flags.
- **D-20:** "Behaviour unchanged" is pinned by a host test: none of the four reserved bits (nor `RFNOMNT`) is in either `sys_rfork` `SUPPORTED` mask, so `rfork` with any of them still returns "rfork: unsupported flags". Phases 6 and 7 must change that test on purpose when they make a flag live. No live boot check is added for this.
- **D-21:** The host test of DSGN-04 also pins the four values and proves they overlap no existing flag, with bit 14 covered through the real `RFNOMNT` constant.
- **D-22:** Kernel bytes follow the standing rules: compare both kernels' sha256 before and after the move. Moving constants is expected to change nothing; if x86_64 bytes do change, the `boot-iso` `outputHash`, amd64-container TCG gates and owner linux check apply, and an aarch64 change gets the quick pass.

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
| DSGN-01 | Threat model document committed: in-scope protections, out-of-scope attacks, SEV-SNP/TDX named as remedy (AC-T01) | Source-passage inventory with file and line references (section "Threat-Model Source Inventory"); doc layout recommendation; D-15/D-16 rules |
| DSGN-02 | Corrected C prototypes committed inside r9, landing location decided | `sys/include/rfork.h` generated by `cargo xtask rfork-header`; Harvey enum and declaration quoted verbatim; generation route verified (`#[path]` include); staleness check wiring |
| DSGN-03 | Four reserved `rfork` bits as named constants in r9 (amended by D-17: one `port/` module) | Every definition and import site enumerated; module layout and visibility; wording amendments listed |
| DSGN-04 | Host test pins four values, proves no overlap, `rfork` behaviour unchanged | Port integration test for values/overlap; hoisted `SUPPORTED` masks for the D-20 test; where each test can run (Mac vs x86_64 Linux vs hosted CI) |
| DSGN-05 | Maintainer confirms IG-D01..IG-D13; PROJECT.md Key Decisions records which are locked | Exact current rows, source rows, proposed-status table, and every wording site that must change |
</phase_requirements>

## Summary

The phase is small in code and large in documents. The code work is: one new dependency-free module `port/src/rfork.rs` holding sixteen `u64` flag constants plus an `ALL` name table, both arch crates importing from it, the two function-local `SUPPORTED` masks hoisted to module level so a host test can see them, one xtask step that generates `sys/include/rfork.h` and checks it for staleness, and tests. The document work is a threat model, a thirteen-row decision table for the owner, and wording amendments in four planning files.

Three findings change how the plan must be shaped. **(1) CONTEXT D-22's expectation that moving the constants "is expected to change nothing" is false for kernel bytes.** I measured it in a scratch copy of HEAD: inserting one comment line before the rfork constants in `x86_64/src/proc.rs` changed the x86_64 kernel ELF sha256, and still changed it after `objcopy --strip-all` (66 bytes in `.rodata`: panic `Location` line numbers), while `.text` stayed byte-identical. On aarch64 a one-line insertion changed the ELF sha256 (debug line tables) with `.text` and `.rodata` identical. Deleting 11 and 17 lines from `proc.rs`/`syscall.rs` will therefore change both kernels' bytes unless line counts are balanced. The plan must pick a path: balance lines so bytes stay identical, or run the owner's x86_64 byte-change procedure. **(2) `cargo xtask test` does not run the `xtask` crate's own tests and does not run the x86_64 crate's tests inside the nix devshell** (no rustup there), so the x86_64 half of D-20 cannot be proved on the Mac, and the staleness check belongs in `cargo xtask check` plus an xtask unit test, not in `cargo xtask test`. **(3) `xtask` has no `port` dependency and should not get one**; the verified route is `#[path = "../../port/src/rfork.rs"]` including the same source file into xtask, which needs the module to be core-only with no `use` lines.

**Primary recommendation:** Put all sixteen flags in `port/src/rfork.rs` defined once by a `macro_rules!` that also emits an `ALL: &[(&str, u64)]` table; include that file into xtask with `#[path]`; generate `sys/include/rfork.h` from `ALL`; hoist each arch's `SUPPORTED` to a `pub(crate) const RFORK_SUPPORTED` in its `syscall.rs`; treat kernel bytes as changing and measure before and after at `.text` level, with the owner's decision on whether `.text`-identical satisfies the standing procedure.

## Architectural Responsibility Map

This is a bare-metal kernel project, so the tiers are crates and tools, not web tiers.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Flag constant definitions (16 flags, `ALL` table) | `port/` crate (`port/src/rfork.rs`) | — | D-17: one definition both arch crates import |
| `SUPPORTED` mask (which flags are live) | Per-arch `syscall.rs` (`x86_64`, `aarch64`) | — | Liveness is arch-specific (RFROBASE/PKEY is x86-only per IG-D08); CONTEXT discretion keeps masks where they are |
| Header generation and staleness check | `xtask` (host std tool) | `port/src/rfork.rs` as data source | The Rust constants are the single source of truth (D-12) |
| Value/overlap pin tests | `port/` host tests (`port/tests/`) | — | `port` builds and tests on every host via `cargo xtask test` |
| `SUPPORTED`-mask pin tests | Arch crates' existing `#[cfg(test)]` modules in `syscall.rs` | — | Mask stays arch-local; aarch64 leg runs on the Mac, x86_64 leg only where rustup + an x86_64 host exist |
| Threat model, decision table | `docs/threat-model.md`, `.planning/PROJECT.md` | Owner checkpoints | Documents; owner confirmation is the gate |
| Kernel-byte evidence | Build tooling (`cargo xtask build`, flake `kernel-x64`/`kernel-virt`, `boot-iso`) | Owner's x86_64 Linux host | Reproducible builds make section hashes comparable |

## Standard Stack

No new external packages. Everything uses what is already in the workspace.

### Core
| Library / Tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| Rust nightly | `nightly-2026-01-30` (`rust-toolchain.toml`; devshell reports `rustc 1.95.0-nightly (842bd5be2 2026-01-29)`, `cargo 1.95.0-nightly (efcd9f586 2026-01-23)`) | All crates | Pinned by repo [VERIFIED: `/Users/neilskinner/code/r9/rust-toolchain.toml`, `nix develop -c cargo --version`] |
| `port` crate | 0.1.0, edition 2024 | Home of the flag module | Already builds `no_std` for the kernel and `std` under `cfg(test)` [VERIFIED: `port/Cargo.toml`, `port/src/lib.rs:3`] |
| `xtask` crate | 0.1.0, deps `clap`, `serde`, `target-lexicon`, `toml` only | Header generator | Existing home of drift checks (`uregcheck.rs`) [VERIFIED: `xtask/Cargo.toml`] |
| `clang` (clang-wrapper 21.1.8) | in devshell | Optional C syntax check of `rfork.h` | `clang -fsyntax-only -Wall -Wno-unused-command-line-argument -x c` on a sample of the planned header printed `C-SYNTAX-OK` [VERIFIED: run this session] |

### Supporting
| Tool | Purpose | When to Use |
|------|---------|-------------|
| `objcopy -O binary --only-section=.text` (from clang-wrapper in devshell) | Section-level kernel comparison | D-22 before/after evidence; `.text` sha256 is the "no behaviour change" proxy [VERIFIED: run this session] |
| `nix develop -c cargo xtask build --arch x86-64 --config q35churnshell` / `--arch aarch64 --config virtshell` | Build the same kernels the flake packages build, locally on the Mac, in about 10 s after the first build | Before/after comparison on the dev machine [VERIFIED: ran both this session] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `#[path]` include of `port/src/rfork.rs` into xtask | `xtask` depends on `port` | Pulling `port` into xtask compiles `smoltcp`, `miniz_oxide`, `bitflags` into every `cargo xtask` invocation, including the flake's `kernel-x64` build which runs `cargo xtask build`; the include costs nothing. `port` has no `#[global_allocator]` or `#[panic_handler]` (grep of `port/src` found none), so the dependency would link, but it buys nothing [VERIFIED: grep + `xtask/Cargo.toml`] |
| `#[path]` include | Text-parse `port/src/rfork.rs` from xtask like `uregcheck` parses `ureg.h` | Parser is brittle and re-derives what the compiler already knows; the include reads the real constants |
| Per-arch `RFORK_SUPPORTED` hoist | One shared `port::rfork::SUPPORTED` | Shared mask would run its test everywhere, but liveness is arch-specific (RFROBASE is a PKEY feature, IG-D08) and CONTEXT keeps masks in the arch crates. Not recommended; mention to owner only if x86_64 test coverage is judged too thin |

**Installation:** none. **Version verification:** no packages added; `cargo`/`rustc` versions confirmed above.

## Package Legitimacy Audit

No external packages are installed by this phase. The `package-legitimacy` check was not run because there is nothing to check.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
 port/src/rfork.rs  (core-only, no `use`; one macro defines 16 consts + ALL table)
   |                    |                         |
   | pub mod rfork      | #[path] include         | pub mod rfork (same crate graph)
   v                    v                         v
 x86_64/src/*         xtask/src/rforkhdr.rs    port/tests/rfork_test.rs
 aarch64/src/*        (reads ALL; renders C)    (pins 16 values, bit 14 = RFNOMNT,
   |                    |                        pairwise disjoint, single bit each)
   | imports flags      | `cargo xtask rfork-header`        -> writes sys/include/rfork.h
   | defines            | `cargo xtask rfork-header --check`-> fails if committed file stale
   | RFORK_SUPPORTED    |                          (also called from CheckStep; unit test
   |  (module-level)    |                           include_str!s the committed header)
   v                    v
 syscall.rs tests: reserved bits and RFNOMNT not in RFORK_SUPPORTED
   (aarch64 leg: `cargo xtask test` on Mac; x86_64 leg: hosted CI / x86_64 rustup host)

 Owner checkpoints:  (a) port/src diff before commit   (b) threat-model draft
                     (c) 13-row decision table          (d) kernel-byte evidence / procedure
```

### Recommended Project Structure
```
port/src/rfork.rs            # NEW: the one definition (16 flags + ALL)
port/src/lib.rs              # +  #[forbid(unsafe_code)] pub mod rfork;   (owner diff approval)
port/tests/rfork_test.rs     # NEW: value + overlap pins (picked up by `--lib --tests`)
xtask/src/rforkhdr.rs        # NEW: render + write + check; `#[path]` includes port's rfork.rs
xtask/src/lib.rs             # + pub mod rforkhdr;
xtask/src/main.rs            # + `rfork-header` subcommand; CheckStep calls the check
sys/include/rfork.h          # NEW (generated, committed)
docs/threat-model.md         # NEW
x86_64/src/proc.rs           # delete consts at 167-181, import from port
x86_64/src/syscall.rs        # import at :92, hoist SUPPORTED, add test
aarch64/src/syscall.rs       # delete consts 87-108, import, hoist SUPPORTED, add test
aarch64/src/proc.rs          # 12 qualified uses `crate::syscall::RF*` -> `port::rfork::RF*`
```

### Pattern 1: One macro, consts and table cannot drift
**What:** a `macro_rules!` emits each `pub const NAME: u64 = 1 << BIT;` and a single `ALL` table from the same token list, so the generator and the tests iterate the exact set the kernel imports.
**When to use:** the flag module. It must use only `core` and contain no `use`/`crate::` so the same file compiles inside `port` (no_std) and inside `xtask` (std) via `#[path]`.
**Example (compiled and run in this session with `rustc --edition 2024`, including `--test`):**
```rust
macro_rules! rfork_flags {
    ($($(#[$m:meta])* $name:ident = $bit:literal;)+) => {
        $($(#[$m])* pub const $name: u64 = 1 << $bit;)+
        /// Every flag, in declaration order: the generator's and the tests' one list.
        pub const ALL: &[(&str, u64)] = &[$((stringify!($name), $name)),+];
    };
}
rfork_flags! {
    RFNAMEG = 0;
    RFPROC = 4;
    RFNOMNT = 14;
    RFCKEY = 18;
}
```
and in the consumer: `#[path = "../../port/src/rfork.rs"] mod rfork;` [VERIFIED: compiled and ran in scratch]. The real module lists all sixteen in the order of Harvey's enum, then the four new ones, keeping the provenance comment "Harvey `sys/include/libc.h` — frozen values" that both arch files carry today (CONTEXT code_context).

### Pattern 2: Staleness check as a drift gate with a negative control
**What:** `rforkhdr::check()` renders the header from `ALL` and compares it byte for byte with the committed `sys/include/rfork.h`; a unit test also `include_str!`s the committed file and asserts equality, and a negative control mutates the rendered text in memory and requires the comparison to fail. The existing precedent is `xtask/src/uregcheck.rs`, whose `run_amd64_negative_control` mutates the header in memory and fails the run if the mutation passes [VERIFIED: `xtask/src/uregcheck.rs:177-200`]. `xtask` unit tests already `include_str!` repo files with `../../` paths [VERIFIED: `xtask/src/lib.rs:114-169` includes `../../x86_64/lib/config_q35.toml`].
**Optional strengthening:** when the Harvey tree resolves (`harvey::harvey_dir(Arch::X86_64)`), also compare the twelve frozen values with `sys/include/libc.h:613-624` of that tree, as `uregcheck` does for `ureg.h`. The realised amd64 tree in the nix store contains `sys/include/libc.h` and its lines 613-624 and 684 match the quote below [VERIFIED: read from `/nix/store/24n9809l3fbciw0pv2iy72214bxkwzh3-harvey-tree-amd64-0/sys/include/libc.h`].

### Pattern 3: Hoist the mask so a test module can see it
The test modules already use `use super::*` (x86_64 `mod tests` at `x86_64/src/syscall.rs:2247`) or name items explicitly (aarch64 `mod noted_plan_tests`, `use super::{NotedPlan, plan_noted};` at `aarch64/src/syscall.rs:2536-2537`). A `pub(crate) const RFORK_SUPPORTED: u64 = RFPROC | ...;` at module level is reachable from both. No existing test calls a `sys_*` handler directly, and `sargs` reads the arguments through `UserSlice::validated_only(proc, frame.sp, ...)` so a behavioural test of `sys_rfork` needs user-memory fixtures; the hoist is the cheap route [VERIFIED: grep of both test regions found no `sys_*(` call; `x86_64/src/syscall.rs:256-264`].

### Anti-Patterns to Avoid
- **Re-export shims** (`pub(crate) use port::rfork::*` in `proc.rs`): forbidden by D-18.
- **Hand-editing `sys/include/rfork.h`:** the file is generated; hand edits fail the staleness check.
- **Including `sys/include/rfork.h` and Harvey's `libc.h` in the same C translation unit:** both declare the `RF*` enumerators, which is a C redeclaration error. The header is a standalone statement of the interface, not a build input today (r9 has no `sys/` consumer). Say so in the header comment.
- **Putting the staleness check only in `cargo xtask test`:** that command never runs the xtask crate's tests (see Pitfall 2).
- **Adding a `port` dependency to xtask** (see Alternatives).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Getting Rust constants into a host tool | A text parser of `rfork.rs` | `#[path]` include of the same source | The compiler reads the real values; verified to compile in both a std binary and `--test` |
| Const/table consistency | A hand-written name list next to the consts | The `rfork_flags!` macro | Prevents a flag existing without being in the generated header or the overlap test |
| Header staleness detection | A new diff mechanism | Render-and-compare in `xtask` with a negative control, same shape as `uregcheck` | Precedent exists; `include_str!` unit tests already used |
| C-syntax validation | A custom parser | `clang -fsyntax-only` in the devshell (optional leg) | Already on PATH in the devshell |

**Key insight:** every piece needed already exists in the repo as a pattern; the risk is in the wording and the kernel-byte consequences, not in new machinery.

## Runtime State Inventory

Not a rename/migration phase in the data sense, but constants move between files, so the five categories were answered:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None. `rfork` flag values are compile-time constants; no database or datastore stores them. | None |
| Live service config | None. The flags are not named in any service configuration; the only outside reference is Harvey's `libc.h`, which is read-only and unchanged. | None |
| OS-registered state | None. | None |
| Secrets / env vars | None. | None |
| Build artifacts / installed packages | **The flake's FOD `boot-iso` `outputHash`** at `flake.nix:983`: `outputHash = "sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=";` is the NAR hash of an ISO that embeds the strip-all x86_64 kernel. Any x86_64 kernel byte change (including a line shift) invalidates it. Also Cargo `target/` directories (gitignored; rebuilt). | See "Kernel Bytes" below: re-derive the hash on the owner's Linux host if bytes change |

## Kernel Bytes (D-22): measured, not assumed

**Experiment (scratch copy of `git archive HEAD` under the session scratchpad; the repo itself was not modified):**

| Build | Edit | Result |
|-------|------|--------|
| x86_64, `cargo xtask build --arch x86-64 --config q35churnshell` (the exact config the flake's `kernel-x64` builds, `flake.nix:406-424`) | none | ELF sha256 `8f2d485e375937a764006273bbd6caf1f9bdb178dd307ad973abe981ac616cce` |
| same | one comment line inserted before line 166 of `x86_64/src/proc.rs` | ELF sha256 `d7bacbab19b92904a5f2458878b4fbf72d327e17ca06f1333cdd6c77cefe9bf5` |
| both ELFs after `objcopy --strip-all` (the form `boot-iso` stages, `flake.nix:236`) | — | `316934ca...` vs `b02afea0...`, 66 bytes differ |
| `.text` of both | — | identical, `38eed81462c8bf248f48310a08c0ebd9e4e62b6a8244a05a8e5df0d180820330` |
| `.rodata` of both | — | differ in 66 bytes (`d42737e4...` vs `7018239b...`): panic `Location` line numbers |
| aarch64, `--arch aarch64 --config virtshell` (flake `kernel-virt`) | one comment line inserted before line 90 of `aarch64/src/syscall.rs` | ELF sha256 `76e6e153...` before, `b796f96a...` after; `.text` identical (`b904ec4e...`), `.rodata` identical (`9bd217e2...`) |

[VERIFIED: measured this session; commands and hashes above]. The nix build already normalises paths with `--remap-path-prefix` (build dir to `/build`, workspace to `/r9`) so absolute paths are not the cause [VERIFIED: `xtask/src/config.rs:840-856`].

**Consequences for the plan:**
1. Deleting `x86_64/src/proc.rs:171-181` and `aarch64/src/syscall.rs:92-108` and hoisting the masks WILL change the x86_64 ELF (rodata line numbers) and the aarch64 ELF (debug line tables) unless line counts are balanced. D-22's "expected to change nothing" should be restated by the plan as "expected to change no executable code (`.text` identical)".
2. **Option L (line-balancing):** keep every edit line-count-neutral before the next panic site (replace the 15-line x86_64 constant block with a 15-line import-plus-comment block; replace the `use proc::{...}` line at `x86_64/src/syscall.rs:92` with one line; place the hoisted mask so lines added above equal lines removed below). `rustfmt` is not enforced in CI (`.github/workflows/xtask.yml` has no fmt step; `rustfmt.toml` sets only edition, `use_small_heuristics = "Max"`, `newline_style`). Verify by building both kernels before and after on the Mac and comparing whole-ELF sha256; if identical, the standing x86_64 procedure is not triggered. Cheap: about 10 s per build after the first.
3. **Option P (accept the change):** run the owner's standing procedure: re-derive and commit the `boot-iso` `outputHash` on the owner's x86_64 Linux host (`nix build .#boot-iso`; the FOD reports the got-hash), run the x64 TCG gates (STATE: amd64 Docker container, cleaned up afterwards; the `flake.nix` stop-B2 comment records that a native amd64 host is the TCG venue and the container is then not required), and get the owner's linux boot-iso check. Docker is installed on this Mac (`/usr/local/bin/docker`); whether its daemon is running was not checked. For aarch64, the quick pass (below).
4. **Do not skip the procedure on a `.text`-identical argument alone.** STATE's standing rule says any x86_64 kernel byte change triggers it. Whether `.text`-identical with a rodata-line-number-only delta satisfies it is an owner decision; the plan should put that question at the owner checkpoint, offering Option L first.
5. The Mac-built before/after comparison is valid on its own (same machine, toolchain, flags). Whether a Mac-built ELF equals the nix-sandbox ELF was not tested [ASSUMED not needed]; do not claim it.

**Commands (verified runnable on the Mac):**
```bash
nix develop -c cargo xtask build --arch x86-64 --config q35churnshell
shasum -a 256 target/x86_64-unknown-none-elf/debug/x86_64
nix develop -c cargo xtask build --arch aarch64 --config virtshell
shasum -a 256 target/aarch64-unknown-none-elf/debug/aarch64
# section proxy (objcopy comes from the devshell's clang-wrapper)
nix develop -c bash -c 'objcopy -O binary --only-section=.text target/x86_64-unknown-none-elf/debug/x86_64 /tmp/x.text && shasum -a 256 /tmp/x.text'
```
Use a scratchpad path, not `/tmp`, in real runs. Build the BEFORE kernels from the commit before the first kernel-touching edit and keep the ELF copies.

**What needs the owner's Linux host:** `nix build .#boot-iso` (`boot-iso` exists only for `system == "x86_64-linux"`, `flake.nix:624-626`), the `--rebuild` check, the KVM quick gate. **What runs here:** local kernel builds and hashing, `cargo xtask test`, `cargo test -p xtask`, the aarch64 gates under HVF/TCG.

## Per-Question Findings

### Q1. Generating `sys/include/rfork.h` from the Rust constants
- `xtask` does not depend on `port` [VERIFIED: `xtask/Cargo.toml` `[dependencies]`: clap, serde, target-lexicon, toml]. Do not add it (see Alternatives). Use the `#[path]` include; there is no existing `#[path]` in xtask but `port/src/lib.rs` already uses the mechanism for `devip_waker` [VERIFIED: `port/src/lib.rs`, `#[path = "devip/waker.rs"] pub mod devip_waker;`].
- The flake derivations all use `src = ./.` so the include path resolves inside the nix build too [VERIFIED: `flake.nix:409, 440, 538, 572`].
- The xtask crate is a lib plus a bin; drift checks live in the lib (`pub mod uregcheck;` in `xtask/src/lib.rs`), the subcommand wiring in `main.rs`. New module `rforkhdr` goes in the lib so `cargo test -p xtask` covers it.
- Where things run: `cargo xtask check` (`CheckStep`, `xtask/src/main.rs:3652-3739`) loops `cargo check` for `aarch64`, `riscv64`, `x86_64` bins and `port` lib/tests; it is the natural place to add an in-process `rforkhdr::check()` call after the loop. `cargo xtask test` (`TestStep`, `:3521-3607`) runs only `port --lib --tests` plus the host-arch crate leg; it never runs the xtask crate's tests. The xtask crate's own tests run via `cargo test -p xtask` (Phase 1 validation records 185 tests, and states this explicitly) [VERIFIED: `01-VALIDATION.md` lines 39-50]. CI's `xtask.yml` lint job runs `cargo xtask check` and `cargo xtask clippy`.
- **Recommendation:** subcommand `cargo xtask rfork-header` (writes) and `cargo xtask rfork-header --check` (fails on drift); `CheckStep::run` calls the check; a `rforkhdr` unit test `include_str!`s the committed file and also runs a negative control; the quick feedback command for the phase is `nix develop -c cargo test -p xtask rfork`.

### Q2. Where host tests run, and the `SUPPORTED` mask
- `port`: `cargo xtask test` runs `cargo test --package port --lib --tests` always. Baseline today: 168 lib tests pass plus the `port/tests/*_test.rs` integration files (devcons 3, devdup_devsrv 2, devenv 6, ...) [VERIFIED: ran `nix develop -c cargo xtask test` this session]. A new `port/tests/rfork_test.rs` is picked up with no wiring; precedent for integration tests exists (`port/tests/namespace_test.rs` etc.).
- `aarch64`: the leg runs when the host arch is aarch64, using the host triple even with no rustup (`xtask/src/main.rs:3561-3563`), plus a second leg under the `virtshell` cfg set (`:3580-3582`). So an aarch64 `syscall.rs` test runs on the Mac.
- `x86_64`: the leg runs only if `rustup` reports an installed x86_64 std target. In the nix devshell there is no rustup, `RustupState` is empty, and the leg is skipped; the host-triple fallback is aarch64-only [VERIFIED: `xtask/src/main.rs:31-60, 3550-3565`; `which cargo rustc` found nothing outside the devshell, devshell `cargo` works]. Consequence: the x86_64 `SUPPORTED` pin runs only in hosted CI (`xtask.yml` `tests` job runs `cargo xtask test` on ubuntu-latest with rustup; owner reports `xtask.yml` all green at the pushed commit, `ci.txt`) or on an x86_64 host with rustup. [CITED: `.planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/ci.txt`]
- **Decision point for the plan:** either (a) accept CI as the x86_64 witness, or (b) make a small xtask edit extending the host-triple fallback to x86_64 hosts (mirrors the aarch64 fallback; xtask edits need no owner approval). (b) was not tried and the x86_64 crate's host tests were not run here; mark as unverified until tried on the Linux host.
- Test modules exist in both syscall files: x86_64 `#[cfg(test)] mod tests` at `x86_64/src/syscall.rs:2246-2247`; aarch64 `#[cfg(test)]` modules at `aarch64/src/syscall.rs:2535, 2604, 2680`. The `SUPPORTED` masks are function-local today:

x86_64 `sys_rfork`, `x86_64/src/syscall.rs:1659-1672`:
```rust
    const SUPPORTED: u64 = RFPROC
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
    if flag & !SUPPORTED != 0 {
        return err(proc, "rfork: unsupported flags");
    }
```
aarch64, `aarch64/src/syscall.rs:1967-1980`: the same eleven names in the same order, same error string [VERIFIED: read both]. Both mask the flag first: `let flag = args[0] & 0xffff_ffff;`. Neither mask contains `RFNOMNT` (no r9 file defines it at HEAD; grep of `x86_64 aarch64 riscv64 port xtask` found no `RFNOMNT`).
- **Hoisting is required for D-20**; a function-local `const` is not nameable from a test module. It changes line numbers (see Kernel Bytes) but not `.text` (shown by the experiment; the real move should be re-measured).
- Suggested hoist: `pub(crate) const RFORK_SUPPORTED: u64 = RFPROC | RFMEM | ...;` at module level in each `syscall.rs`, with the function body using it. Test: `for f in [RFCKEY, RFROBASE, RFRANDMEM, RFNOLEGACY, RFNOMNT] { assert_eq!(RFORK_SUPPORTED & f, 0) }` and `assert_eq!(RFORK_SUPPORTED & !0xffff_ffff, 0)`. Name is a recommendation (Claude's discretion).

### Q3. Every definition and import site (D-18)
Definitions (verbatim):

`x86_64/src/proc.rs:167-181`:
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
`aarch64/src/syscall.rs:87-108`: the same eleven constants with the visibility `pub(crate)` except `const RFPROC: u64 = 1 << 4;` which is private, and interleaved doc comments naming which `proc.rs` function reads each. `riscv64/`: no definition or use [VERIFIED: grep of `riscv64` found no `RF*` flag name].

Import / use sites:
- `x86_64/src/syscall.rs:92`: `use proc::{RFCFDG, RFCENVG, RFCNAMEG, RFENVG, RFMEM, RFNAMEG, RFNOTEG, RFNOWAIT, RFFDG, RFPROC, RFREND};` (replace with `use port::rfork::{...}`; same names; one line, line-neutral).
- `x86_64/src/proc.rs` uses the constants unqualified at lines 2675, 2702, 2724-2725, 2851-2925 and, in tests, 3253 (`RFPROC`): it needs an import of `RFNOWAIT, RFMEM, RFCFDG, RFFDG, RFCNAMEG, RFNAMEG, RFCENVG, RFENVG, RFNOTEG, RFREND` plus `RFPROC` for the test (import `RFPROC` under `#[cfg(test)]` or inside the test module to avoid an unused-import warning in the kernel build).
- `aarch64/src/proc.rs` uses fully qualified `crate::syscall::RF*` at lines 2536, 2572, 2598, 2599, 2827, 2829, 2840, 2842, 2853, 2855, 2888, 2896: replace each with `port::rfork::RF*` in place (line-neutral).
- `aarch64/src/syscall.rs` uses unqualified names in `sys_rfork` (lines 1961-2015): add one import.
- No other `.rs` file in `x86_64/`, `aarch64/`, `riscv64/`, `port/`, `xtask/` mentions an `RF*` flag as code [VERIFIED: grep]. `port/tests/namespace_test.rs:408` mentions `RFNAMEG` only inside a string.

`port/` layout: modules are declared in `port/src/lib.rs` as `#[forbid(unsafe_code)] pub mod name;`, alphabetically, 31 `pub mod` lines; exactly three lack the forbid: `devip_waker`, `globalalloc`, `mcslock` [VERIFIED: awk over `port/src/lib.rs`]. Add `#[forbid(unsafe_code)] pub mod rfork;` between `rendez` and `virtio`. Visibility: `pub const` (the arch crates are separate crates, so `pub(crate)` cannot work). `UNSAFE-AUDIT.md:84` ("every other port module unsafe-free, `#[forbid(unsafe_code)]`") already covers a new unsafe-free module. Tests for it belong in `port/tests/rfork_test.rs` (public API) rather than a `_tests.rs` file mounted at the crate root, since the module needs no unsafe fixtures; if the executor prefers the crate-root convention (`mod rfork_tests;` under `#[cfg(test)]`), that is also consistent with `lib.rs`.

### Q4. Kernel reproducibility and the sha256 procedure
See "Kernel Bytes" above for the commands and measurements. Additional verified facts:
- Flake outputs: `packages.<system>.kernel-x64` builds `cargo xtask build --arch x86-64 --config q35churnshell` and copies `target/x86_64-unknown-none-elf/debug/x86_64` to `kernel-x64.elf` (`flake.nix:406-424`); `kernel-virt` builds `--arch aarch64 --config virtshell` and copies `target/aarch64-unknown-none-elf/debug/aarch64` (`flake.nix:535-556`; `kernel-virt-cpu` and the other aarch64 profiles exist). `boot-iso` is defined only under `nixpkgs.lib.optionalAttrs (system == "x86_64-linux")` and takes `kernel-elf = "${kernel-x64}/kernel-x64.elf"` (`flake.nix:624-627`). The stager runs `objcopy --strip-all "$KERNEL" "$stage/boot/kernel"` (`flake.nix:236`).
- The `outputHash` location, verbatim: `flake.nix:983`: `outputHash = "sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=";` (re-derived at stop B2 by the owner's Linux host; the comment block at `flake.nix:972-982` records it).
- STATE's owed check refers to an older value (`sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=` at `116bcc2`), which the stop-B2 re-derivation replaced; do not reuse it.
- The kernel derivations cannot be realised on aarch64-darwin through nix (STATE records an import-from-derivation on the rust-src `Cargo.lock`); the local `cargo xtask build` route works on the Mac and is the one used above.

### Q5. The per-commit gate
- `nix develop -c cargo xtask gate --arch x86-64` is the quick gate (golden invariant: assemble `image:cpu`, build `q35cpu` kernel, boot on q35, rio asserted twice, Harvey tree unmodified, prints `GATE PASS quick x86_64 run=<id>`) [VERIFIED: `xtask/src/gate.rs` header]. `--full` runs the moved gate apps. KVM is the default; `--accel tcg` is allowed.
- **It refuses on the Mac:** `quick_venue` returns `GATE-VENUE not-linux` for a non-Linux host (`xtask/src/gate.rs:129-137`), by design (D-13: the Mac is not the witness). The required witness is the owner's x86_64 Linux KVM run (`docs/harvey-supply.md` section 6). Hosted CI runs `nix develop -c cargo xtask gate --arch x86-64 --accel tcg` best-effort (`gates.yml`, job `quick-gate-tcg`, `continue-on-error: true`).
- Tier rule (Phase 1 CONTEXT D-12 to D-14): quick gate "for every commit" by practice (not a hook), full set "at phase boundaries and at Harvey pin bumps" [VERIFIED: `01-CONTEXT.md:36-38`].
- **Plan implication:** only the commit(s) that touch kernel source (the arch-crate import and hoist commit) can alter boot behaviour; docs-only, xtask-only and `port` test commits cannot. Name the quick gate (owner's Linux KVM) once after the kernel-touching commit group and again at the phase boundary; the full set at the phase boundary is the existing rule. The aarch64 quick pass runs on the Mac: `nix run .#boot-r9-tier1` (HVF) and `R9_ACCEL=tcg nix run .#boot-r9-tier1`, plus `nix run .#boot-r9-smoke`, `.#boot-r9-net-smoke`, `.#boot-r9-cpu` once on HVF [VERIFIED: `01-VALIDATION.md:156-158`; HANDOFF.md:46].

### Q6. Threat-model source inventory (for `docs/threat-model.md`)
Line numbers are from `grep -n`/`sed -n` this session. The passages below state an asset, attacker, trust boundary or protection; everything else in the document is "new in this document" (D-15).

**AC = `.planning/r9-architecture-and-code-v4.md`**
| Ref | Lines | Content |
|-----|-------|---------|
| AC 2.2 | 84-88 | The boundaries: in scope (pages at rest: evicted frames, backing stores, NVMe swap, Venti archives, 9P-AEAD transit; stale-frame disclosure across distinct process KeyIDs; stray kernel reads through incorrect page-table mappings, since only the shadow frame contains plaintext; packet splicing, injection, replay); out of scope (physical DRAM bus probing, cold-boot, arbitrary code execution in the kernel context; keys live in kernel memory so a compromised supervisor exposes them; SEV-SNP or TDX required for bus sniffing). Copy exactly (D-15). Same text as AC-T01, `.planning/intel/constraints.md:211` |
| AC 1.1 | 33, 35 | Line 33 states `port` forbids unsafe "across all modules" (differs from HEAD, note N2); line 35 states bits 18-20 are already reserved in `libc.h` and `x86_64/src/proc.rs` (false at HEAD, note N3) |
| AC 3.1 | 96-100 | Asset: keys in `Zeroizing<[u8; 32]>`; `ReadOnly` capability has `k_dec` and BLAKE3 verify only, no `k_mac`, "mathematically preventing" valid tags (trust boundary between sibling domains) |
| AC 3.2-3.3 | 104-112 | Cache-line and vector-register handling during decrypt; `ymm0`-`ymm5` only; AVX-512 disabled in XCR0 (line 112) |
| AC 3.4 | 116-119 | `UninitializedFrame`; same-KeyID recycling skips zeroing (118); cross-domain reassignment scrubbed with `CLFLUSHOPT` (119) (stale-frame disclosure protection) |
| AC 5 | 253 | Failover sweep: un-reclaimed handles freed, keys zeroized |
| AC 5.8 | 292-297 | Legacy gateway: plaintext only in pre-pinned non-pageable bounce frames, scrubbed with `zeroize` + `CLFLUSHOPT` (296); `rfork(RFNOLEGACY)` forbids binding, mounting or walking `CHAN_UNTRUSTED_WIRE` channels (297) (a trust boundary) |

**RFC = `.planning/rfc-9p2000-aead.md`**
| Ref | Lines | Content |
|-----|-------|---------|
| 2 (AAD def) | 52 | AAD authenticated "to prevent packet tampering and splicing attacks" |
| 3.1 | 66-68 | Primitives: ChaCha20 (confidentiality), keyed BLAKE3 (integrity and authenticity) |
| 3.2 | 70-88 | Key hierarchy: root seed never on the wire; `Tauth` before `Tattach`, authenticated key exchange via factotum (Noise or DP9IK) (74-76); transient key per connection and direction (78-80); per-object key binding (84-86); forward secrecy and zeroization on clunk (88) |
| 4.2 | 143-157 | Monotonic 64-bit sequence nonce; receivers MUST reject any non-next value, "preventing packet replay and out-of-order injection" (147) |
| 4.3 | 159-165 | AAD contents; splicing and replay attacks excluded for "network adversaries" (165); MAC failure terminates the connection and zeroes state without parsing |
| 5.2 | 243 | `StatelessCap`: HMAC tag under a server cluster secret, verified by any replica holding the domain secret |
| 8 (recovery) | 368-374 | Reclaim grace period, orphan sweep, key zeroization |
| 10.2-10.3 | 524-537 | Legacy gateway bounce frames; `RFNOLEGACY` "to prevent accidental data leakage from cryptographically protected processes to cleartext networks" (537) |
| 11 | 540-550 | Security considerations: keystream reuse and nonce management (re-key before 2^64 - 1) (544); microarchitectural side channels, vector scrub, `CLFLUSHOPT` on cross-KeyID reassignment (546); splicing and replay (547-548); lease-retention denial of service with a 5-second revoke timeout (549-550) |

**IG = `.planning/r9-implementation-guide-amended-v3.md`**
| Ref | Lines | Content |
|-----|-------|---------|
| 3 ledger | 64 | Note delivery: "Malicious processes could aim SP into read-only Block A" (attacker class: malicious process); fix requires `SegKind::Stack` |
| 4 | 69-120 | Isolation subsystem: capability tokens (75-90), vector scrub with `VPXOR` (110), AVX-512 disabled (111), allocator zeroing discussion (115) |
| 5/7 | 203 | Legacy gateway via non-pageable bounce buffers, `rfork(RFNOLEGACY)` |
| 8 | 258 | Phase 0 row: "Threat model doc, corrected C prototypes, reserve `rfork` flag bits 18-20 in `libc.h`" |

**PT = `.planning/r9-permissions-and-test-plan.md`**
| Ref | Lines | Content |
|-----|-------|---------|
| Topology | 24-34 | `cpu05` hosts `eve`, labelled adversary, on the grid fabric |
| 3.1 | 170 | Persona `eve` (UID 1005): "Adversary / Probe: Probes unauthorized reads, attempts note spoofing and memory snooping" (the only named attacker persona); `none` UID 9999 anonymous world at line 173 |
| 1.1 | 220-237 | Discretionary access: user C and adversary denied |
| 2.3 | 296-304 | Anti-splicing and replay: stale nonce dropped; FID edit fails the AAD MAC |
| 3.2 | 324-334 | Unregistered cross-domain user attenuated to UID 9999 |
| Suite 4 | 336-358 | Sibling memory partitioning (`RFCKEY`): cross-sibling dereference traps, direct frame read fails the MAC, kernel kills with `sys: crypto integrity violation` (4.1); read-only base via PKEY 1 / `WRPKRU` trap (4.2) |

Passages that do NOT exist in any source (so they are "new in this document" and need the visible mark): a list of assets as such (keys, plaintext frames, backing stores, FID and lease state), a list of attacker classes with capabilities (network attacker, malicious sibling or process, malicious or unresponsive client, malicious legacy peer, physical attacker, compromised kernel), an explicit trust-boundary list, and the protection-by-attacker table. The sources give only fragments; the full analysis (D-14) is new work.

**Recommended document layout (Claude's discretion):** (1) Status banner stating the doc is living and that no protection listed exists in code yet (D-16); (2) Scope boundary copied exactly from AC 2.2 (D-15); (3) Assets; (4) Attacker classes and capabilities; (5) Trust boundaries; (6) Protection table with columns protection, mitigates, source, planned phase, **status: not implemented**; (7) Out-of-scope attacks and the remedy (SEV-SNP/TDX); (8) Open points. Mark new statements with a fixed marker such as `[NEW]`, and cite others as `[AC 2.2]`, `[RFC 4.3]`, `[IG row 4]`, `[PT 2.3]`. The owner reviews the draft before it is committed as final (D-15).

### Q7. DSGN-05: exact current text and every wording site

**Current PROJECT.md Key Decisions block** (`PROJECT.md:112-127`), intro line 114 verbatim: "No ADR exists and nothing is locked in this ingest. The guide's "non-negotiable" decisions (IG-D01 to IG-D13) are carried as constraints, not locked; whether to lock them is confirmed in Phase 2 (DSGN-05)."

Rows that mention IG decisions today (only five of thirteen have a row):
| Line | Decision text | Current outcome |
|------|---------------|-----------------|
| 122 | `libstd_r9` precedes Track P (IG-D04) | Carried from IG-D04; confirm in Phase 2 |
| 124 | Succinct TPHT is primary concurrent hashing; Roaring is the set algebra; Funnel is scratchpad only (IG-D09, IG-D10) | Carried constraint; confirm in Phase 2 |
| 125 | Native `9P2000.aead` is the distributed wire security (IG-D11; details open at W1-W4) | Carried constraint; confirm in Phase 2 |
| 126 | Dual C runtime (IG-D06) | Carried constraint; confirm in Phase 2 |
| 127 | Build priority (IG-D01) | Carried constraint; confirm in Phase 2 |
| 123 | The `RFNOLEGACY` row (names `x86_64/src/proc.rs`, `aarch64/src/syscall.rs`) | Decided 2026-11-22 |

There are no rows for IG-D02, IG-D03, IG-D05, IG-D07, IG-D08, IG-D12, IG-D13. Row 124 combines D09 (not locked) and D10 (locked) and must be split. The Constraints section lines 44-48 and 76-77 reference IG-D03, D02, D09, D06, D07 as constraints.

**Source rows.** Guide section 2 table: `r9-implementation-guide-amended-v3.md:25` header, `:26` separator, rows `:27` (Build Priority) through `:39` (Cluster Service Discovery) in IG-D01..IG-D13 order. The guide table carries no IDs; the IDs are assigned in `.planning/intel/constraints.md` headings: IG-D01 `:17`, D02 `:22`, D03 `:27`, D04 `:32`, D05 `:37`, D06 `:42`, D07 `:47`, D08 `:52`, D09 `:57`, D10 `:62`, D11 `:67`, D12 `:72`, D13 `:77`. Texts for IG-D03 and IG-D07 (the two that change), verbatim:
- IG-D03 (`constraints.md:27-30`): "No POSIX emulation calls and no `mmap` extensions. Software adapts to Plan 9 primitives (`rfork`, `bind`, `mount`, 9P). Crypto extensions add 3 flag bits to `rfork` and zero syscalls. Landing sites `sys/include/libc.h`, `sys/src/syscall.rs`. [see W11 for the RFNOLEGACY flag]"
- IG-D07 (`constraints.md:47-50`): "`port/` is `#![forbid(unsafe_code)]`. Low-level operations (`WRPKRU`, `MOVNTDQ`, `CLFLUSHOPT`, inline assembly, raw atomics) are restricted to `x86_64/`, `aarch64/`, and `ds/src/tpht/` behind audited `// SAFETY:` blocks. Section 11 lint contracts: `undocumented_unsafe_blocks = deny` on arch crates; `#![forbid(unsafe_code)]` on `crypto/`, `ds/`, and `port/`. [see W5 for the ds/ inconsistency]"

**IG-D07 amended wording must match HEAD.** Verified: `port/src/lib.rs` has per-module `#[forbid(unsafe_code)]` with exactly three modules lacking it (`devip_waker`, `globalalloc`, `mcslock`). Note the `lib.rs` header comment (lines 10-17) calls `globalalloc` and `mcslock` "the two named floor modules" and `devip_waker` "the ONE named phase-08 bridge module", while `UNSAFE-AUDIT.md:83` says the bridge is "not floor". D-06 says "three named floor modules"; the lock row should say what D-06 says and may add "(`devip_waker` is a named bridge in `lib.rs`)" so the row and the code comment agree. Also verified: `Cargo.toml` opts only `aarch64` and `port` into `undocumented_unsafe_blocks = deny`; `x86_64` and `riscv64` are not opted in (`Cargo.toml` comment; `x86_64/Cargo.toml` has `[lints.rust]` only). That matches note N2.

**Proposed status table for the owner checkpoint (D-03 to D-07):**
| ID | Decision | Proposed status |
|----|----------|-----------------|
| IG-D01 | Build priority: `ds/` and 9P upfront | Locked as written |
| IG-D02 | `a.out` exclusively | Locked as written |
| IG-D03 | Closed syscall list | Locked, wording corrected to four `rfork` flag bits; suggest also noting the landing site is now `sys/include/rfork.h` plus `port/src/rfork.rs`, not `sys/include/libc.h` (a recommendation for the owner to accept or drop) |
| IG-D04 | Native Rust runtime (`libstd_r9`) | Locked as written |
| IG-D05 | Ported C compilers | Locked as written |
| IG-D06 | Dual C runtime | Locked as written |
| IG-D07 | Kernel safety boundary | Locked amended (D-06 wording); N2 scope growth stays for Phases 3 and 5, `ds/src/tpht/` part open under W5 |
| IG-D08 | CR3 plus PKEY isolation | Locked as written |
| IG-D09 | TPHT primary hashing | Not locked, deferred to Phase 3 (W5, W6) |
| IG-D10 | Roaring bitmaps | Locked as written |
| IG-D11 | Native `9P2000.aead` | Not locked, deferred to Phase 4 (W1-W4; W3 also read in Phase 3) |
| IG-D12 | `#G` synthetic device | Locked as written |
| IG-D13 | Cluster discovery | Not locked, deferred to Phase 6 (W7) |

D-08 rule to record in the table's lead-in: a change is a new dated row superseding the old one.

**Every wording site that must change** (line numbers at HEAD):
| File | Line(s) | Why |
|------|---------|-----|
| `.planning/PROJECT.md` | 114 | "nothing is locked in this ingest" lead-in; replace with the Phase 2 result |
| `.planning/PROJECT.md` | 122, 124-127 | "Carried constraint; confirm in Phase 2" rows become Locked or "Not locked, deferred to Phase N"; split row 124 |
| `.planning/PROJECT.md` | new rows | Add rows for IG-D02, D03, D05, D07, D08, D12, D13 |
| `.planning/PROJECT.md` | 123 | RFNOLEGACY row names `x86_64/src/proc.rs` and `aarch64/src/syscall.rs` and says "Phase 4 (GSD Phase 6) implements": amend home to the one `port/` module (D-17) |
| `.planning/PROJECT.md` | 76 | Compatibility already says four flags; keep, optionally name the home |
| `.planning/PROJECT.md` | 77 | Safety constraint repeats the IG-D07 wording ("`port/` and `crypto/` forbid `unsafe`"); align with the amended IG-D07 |
| `.planning/PROJECT.md` | 105 (N3) | Says Phase 2 reserves the bits "in `x86_64/src/proc.rs` and `aarch64/src/syscall.rs`": amend (D-17); lines 171-181 and 92-108 references become "formerly" |
| `.planning/ROADMAP.md` | 147 | Success criterion 2 names the two arch files (D-17 amends it) |
| `.planning/ROADMAP.md` | 149 | Criterion 4: optionally name `sys/include/rfork.h` |
| `.planning/ROADMAP.md` | 152 | Open-questions paragraph: "Touching `x86_64/src/proc.rs` may or may not change kernel bytes" is now answered (it does change `.rodata` line numbers; `.text` identical) |
| `.planning/REQUIREMENTS.md` | 27 | DSGN-03 names the two arch files (D-17) |
| `.planning/REQUIREMENTS.md` | 26 | DSGN-02: record the landing decision (`sys/include/rfork.h`) |
| `.planning/STATE.md` | 71 | Decision log line says "Reserved in r9 only (`x86_64/src/proc.rs`, `aarch64/src/syscall.rs`)" (D-17); line 77 says "Nothing is locked ... pending confirmation in Phase 2" (superseded after the owner checkpoint) |

### Q8. Harvey's exact `rfork` enum and declaration (read-only reference)
From `/Users/neilskinner/code/harvey/sys/include/libc.h` (Harvey checkout HEAD `c697666ba315f137ca2fd11768f835d04bd9f018`; the file's last commit is `80deb8e34`) and identical in the realised amd64 tree: line 610 `/* rfork */`, 611 `enum`, 612 `{`, enumerators 613-624, 625 `};`, and line 684 the declaration. Verbatim, tabs preserved as in the file:
```c
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
and line 684: `extern	int	rfork(int);` (tab-separated). [VERIFIED: read lines 605-630 and 680-688 this session]. The enumerator range 613-624 and line 684 match D-11's citation exactly.

**Specified `rfork.h` content** (generated; the renderer emits this; the four new entries use the same tab alignment): a banner comment (generated by `cargo xtask rfork-header` from `port/src/rfork.rs`, do not edit; r9-owned; mirrors Harvey `sys/include/libc.h:613-624` for the first twelve values and `:684` for the declaration; the four new flags are r9-only and not in Harvey; do not include together with Harvey's `libc.h`), an include guard, then the sixteen-entry enum in bit order of Harvey's list followed by `RFCKEY = (1<<18)`, `RFROBASE = (1<<19)`, `RFRANDMEM = (1<<20)`, `RFNOLEGACY = (1<<21)` (last entry without a trailing comma, as Harvey's last entry), then `extern	int	rfork(int);`. A comment per new flag stating "reserved, not accepted by rfork yet" is useful because the header must not imply liveness (D-16 sentiment; D-20). Sixteen = twelve Harvey (bits 0-6, 10-14) plus four; D-19 counts RFNOMNT in the module's sixteen.

## Common Pitfalls

### Pitfall 1: Assuming the const move is byte-neutral
**What goes wrong:** the plan states "compare sha256, expect identical" and the executor finds both kernels differ, then either panics or silently waves it through.
**Why it happens:** panic `Location` structs embed `line!()` (x86_64 `.rodata`) and DWARF line tables embed lines (both ELFs); deleting 11 or 17 lines shifts every later line. Measured above.
**How to avoid:** measure `.text` separately; pre-decide with the owner whether to line-balance (Option L) or run the procedure (Option P); build the BEFORE kernels first and keep them.
**Warning signs:** whole-ELF sha256 differs while `.text` sha256 matches.

### Pitfall 2: `cargo xtask test` does not run the xtask crate's tests or the x86_64 tests in the devshell
**What goes wrong:** the new `rforkhdr` staleness test or the x86_64 mask test is "green" because it never ran.
**Why it happens:** `TestStep` runs only `port --lib --tests` and the host-arch crate; `cargo test -p xtask` is separate (Phase 1 validation warning W-1); the x86_64 leg needs rustup.
**How to avoid:** the phase's quick command is `nix develop -c cargo test -p xtask && nix develop -c cargo xtask test`; wire the staleness check into `cargo xtask check` too; name CI/Linux as the x86_64 mask witness or extend the fallback.
**Warning signs:** test counts unchanged after adding tests.

### Pitfall 3: The generated header is checked only against itself
**What goes wrong:** the Rust constants and Harvey's values drift (or a constant is mistyped) and the header faithfully reproduces the mistake.
**How to avoid:** the port test pins the sixteen values literally; optionally the check also compares the first twelve with Harvey's `libc.h` when the tree resolves; the C syntax check catches malformed output.

### Pitfall 4: `#[path]` include breaks when the module grows a `use`
**What goes wrong:** `port/src/rfork.rs` later gains `use crate::...` and xtask stops compiling.
**How to avoid:** keep the file core-only; put a header comment in it saying it is included by xtask via `#[path]`; the xtask unit test is the tripwire.

### Pitfall 5: Hoisting changes more than the mask
**What goes wrong:** the hoist accidentally changes the `flag & !SUPPORTED` logic or ordering of the "bad flags" check.
**How to avoid:** keep the two checks and the error strings exactly; the test asserts the mask equals the current eleven names (pin the numeric value, `RFORK_SUPPORTED == 0x3c7f` computed from bits 0-6, 10-13, i.e. `0x7f | 0x3c00 = 0x3c7f`); the `.text` equality check proves no code changed.

### Pitfall 6: Wording amendments miss a site
**What goes wrong:** ROADMAP, REQUIREMENTS, PROJECT and STATE disagree about where the flags live, and later phases cite the old file.
**How to avoid:** use the site table in Q7; end the phase with a grep for `x86_64/src/proc.rs` / `aarch64/src/syscall.rs` near "rfork" in the four files, and for "confirm in Phase 2".

### Pitfall 7: Treating the threat model as locked or as implemented
**How to avoid:** D-16: banner and per-protection status column; not part of the D-08 lock set; `Locked` language must not appear in the document.

## Code Examples

### Rendering the header (shape; the executor writes the real one)
```rust
// xtask/src/rforkhdr.rs  (sketch)
#[path = "../../port/src/rfork.rs"]
mod rfork;

pub fn render() -> String {
    let mut s = String::new();
    s.push_str("/* GENERATED by `cargo xtask rfork-header` from port/src/rfork.rs; do not edit. ... */\n");
    s.push_str("enum\n{\n");
    let last = rfork::ALL.len() - 1;
    for (i, (name, value)) in rfork::ALL.iter().enumerate() {
        let bit = value.trailing_zeros();
        let comma = if i == last { "" } else { "," };
        s.push_str(&format!("\t{name}\t\t= (1<<{bit}){comma}\n"));
    }
    s.push_str("};\n\nextern\tint\trfork(int);\n");
    s
}
```
Alignment (tab counts) should be computed so short and long names align like Harvey's; `ALL` order is the generator's order. [ASSUMED sketch; the macro and `#[path]` mechanics it relies on were compiled and run, see Pattern 1.]

### Port pin test (shape)
```rust
// port/tests/rfork_test.rs
use port::rfork::*;
#[test]
fn values_are_pinned() {
    assert_eq!((RFCKEY, RFROBASE, RFRANDMEM, RFNOLEGACY), (1 << 18, 1 << 19, 1 << 20, 1 << 21));
    assert_eq!(RFNOMNT, 1 << 14);
}
#[test]
fn flags_are_single_disjoint_bits() {
    let mut seen = 0u64;
    for (name, v) in ALL {
        assert_eq!(v.count_ones(), 1, "{name}");
        assert_eq!(seen & v, 0, "{name} overlaps");
        seen |= v;
    }
    assert_eq!(ALL.len(), 16);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Flag constants copied per arch (`proc.rs`, `syscall.rs`) | One `port/` module both import | D-17, this phase | Removes the drift risk the copies carry |
| Guide: "3 flag bits", "reserve in `libc.h`" | Four flags, r9-only | Owner 2026-11-22 (W11) | Wording amendments listed in Q7 |
| `ROADMAP`/`DSGN-03` name the two arch files | One `port/` module | D-17 | Amend both texts |

**Deprecated/outdated:** the AC 1.1 claim that bits 18-20 are already reserved (note N3, false at HEAD); the guide's "crate-level forbid" description of `port/` (note N2).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A Mac-built kernel ELF compared before/after is a sufficient proxy for the nix-sandbox build comparison (Mac and sandbox ELFs not compared to each other) | Kernel Bytes | If the owner requires the nix-derived hash, the Linux step is needed anyway; no wrong claim is made, only a stated limit |
| A2 | The real const move changes `.text` zero bytes (shown for one-line comment shifts only; not run on the real change) | Kernel Bytes | If `.text` differs, behaviour might differ; the plan must re-measure on the real edit and treat a `.text` diff as a stop |
| A3 | Extending the host-triple fallback to x86_64 hosts in xtask would make the x86_64 tests run in the devshell | Q2 | The x86_64 crate's host tests were not run here; they may fail or not build on the owner's host |
| A4 | The `rforkhdr` sketch compiles as written | Code Examples | Minor; mechanics were verified, the sketch was not compiled |
| A5 | Docker daemon on the Mac is usable for the amd64 TCG container leg | Kernel Bytes | If not, use the owner's native amd64 host (the stop-B2 note already allows it) |
| A6 | The recommended note that IG-D03's landing site becomes `sys/include/rfork.h` plus `port/src/rfork.rs` is acceptable to the owner | Q7 | Owner may prefer the guide's original landing sites; offer, do not assume |

## Open Questions

1. **Option L or Option P for kernel bytes (owner decision).**
   - What we know: line shifts change both kernels' bytes; `.text` stayed identical in the experiment; line-balancing is feasible because rustfmt is not enforced.
   - What's unclear: whether the owner accepts a `.text`-identical, rodata-line-number-only delta as satisfying the standing x86_64 procedure.
   - Recommendation: attempt Option L first (measure on the Mac); if whole-ELF hashes differ, present `.text` evidence and ask the owner to choose between waiving and running the procedure.
2. **x86_64 mask test witness.**
   - What we know: it cannot run in the devshell on the Mac or (probably) on the Linux devshell; it runs in hosted CI.
   - Recommendation: accept hosted CI as witness for that one assertion, and optionally extend the xtask fallback; record the choice in the plan.
3. **Which commits run the quick gate.**
   - Recommendation: once after the kernel-touching commit group (owner KVM), again at the phase boundary with the full set; docs, xtask and port-test-only commits do not change boot behaviour.
4. **Owner checkpoint ordering.** The `port/src` diff, the threat-model draft and the thirteen-row table are three separate owner stops; the plan should schedule them so work is not blocked (draft the threat model while waiting for the port diff approval).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `nix` | everything (`nix develop`) | yes | on PATH `/nix/var/nix/profiles/default/bin/nix` | none needed |
| `cargo` / `rustc` (devshell) | builds and tests | yes (devshell only) | `cargo 1.95.0-nightly (efcd9f586 2026-01-23)`, `rustc 1.95.0-nightly (842bd5be2 2026-01-29)` | none |
| `clang` / `objcopy` (devshell clang-wrapper) | optional C syntax check; section hashing | yes | clang wrapper 21.1.8 | `gcc` at `/usr/bin/gcc` for syntax check |
| QEMU (devshell) | aarch64 gates | yes | 11.1.2 | none |
| Harvey tree (amd64) in nix store | optional Harvey-drift comparison | yes, realised | contains `sys/include/libc.h` | skip the optional comparison |
| Harvey checkout `/Users/neilskinner/code/harvey` | reading `libc.h` only | yes | HEAD `c697666` | the nix-store tree has the same lines |
| Docker | amd64 TCG container leg (Option P) | binary present at `/usr/local/bin/docker`; daemon not checked | — | owner's native amd64 host |
| Owner's x86_64 Linux KVM host | quick gate, `boot-iso`, byte-change procedure | not available in this environment | — | none; owner-only (STATE standing rules) |
| `limactl`, `qemu-system-x86_64` outside devshell | — | not on PATH | — | devshell supplies QEMU; lima builder is registered by the owner |

**Missing dependencies with no fallback:** the owner's Linux host for the quick gate and for any `boot-iso` re-derivation.
**Missing dependencies with fallback:** none blocking.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in tests: `port` (via `cargo xtask test`), `xtask` (via `cargo test -p xtask`), `aarch64` host leg (via `cargo xtask test`), `x86_64` host leg (hosted CI only); shell/expect gates for boot |
| Config file | none (workspace `Cargo.toml`; `.planning/config.json` sets `nix develop -c cargo xtask check` / `test`) |
| Quick run command | `nix develop -c cargo test -p xtask rfork && nix develop -c cargo test -p port --test rfork_test` |
| Full suite command | `nix develop -c cargo test -p xtask && nix develop -c cargo xtask test && nix develop -c cargo xtask check && nix develop -c cargo xtask clippy`, then the aarch64 quick pass and the owner's KVM quick gate |

Measured baselines today: `cargo xtask test` port lib 168 passed plus the `port/tests` integration files; `cargo test -p xtask` 185 tests per Phase 1 validation.

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DSGN-01 | Threat-model doc exists, states in-scope and out-of-scope exactly, names SEV-SNP/TDX, states no protection is implemented, every statement cited or marked | doc check (xtask unit test `include_str!("../../docs/threat-model.md")` asserting required headings/phrases and the status banner) plus manual owner review | `nix develop -c cargo test -p xtask threat_model` | No, Wave 0 |
| DSGN-02 | `sys/include/rfork.h` equals the rendering of `ALL`; mutated copy rejected; C syntax valid; optional Harvey-value match | unit + CLI + optional clang | `nix develop -c cargo xtask rfork-header --check`; `nix develop -c cargo test -p xtask rfork`; `clang -fsyntax-only -x c sys/include/rfork.h` | No, Wave 0 |
| DSGN-03 | Sixteen flags defined once in `port::rfork`; no definition left in arch files; both arch crates compile importing them | port test + source-invariant `include_str!` test (no `const RF...: u64` in the two arch files) + build | `nix develop -c cargo test -p port --test rfork_test`; `nix develop -c cargo xtask check` | No, Wave 0 |
| DSGN-04 | Four values pinned; pairwise disjoint single bits; bit 14 is `RFNOMNT`; none of the five in either `RFORK_SUPPORTED`; mask value unchanged | unit (port) + unit (aarch64 on Mac; x86_64 in CI/Linux) | `nix develop -c cargo test -p port --test rfork_test`; `nix develop -c cargo xtask test` (aarch64 leg); CI `cargo xtask test` (x86_64 leg) | No, Wave 0 |
| DSGN-04 (behaviour unchanged) | Kernel `.text` unchanged for both arches around the move; boot still reaches rio | build comparison + gate | `.text` sha256 before/after (commands in Kernel Bytes); owner's `nix develop -c cargo xtask gate --arch x86-64`; aarch64 quick pass | Procedure, not a file |
| DSGN-05 | PROJECT.md records each of IG-D01..D13 as locked, locked-amended, or not-locked-deferred; no "confirm in Phase 2" left; roadmap/requirement/state wording amended | doc check (grep) plus owner checkpoint | `grep -c "confirm in Phase 2" .planning/PROJECT.md` returns 0; thirteen IG rows present | Manual owner confirmation required |

Follow the repository's verification discipline (REQUIREMENTS.md standing constraint, IG-V01): RED pin, implementation, GREEN twice, pin advance. For the port test the RED state is the test failing to compile before `port::rfork` exists; record it.

### Sampling Rate
- **Per task commit:** `nix develop -c cargo test -p xtask rfork && nix develop -c cargo test -p port --test rfork_test`
- **Per wave merge:** `nix develop -c cargo test -p xtask && nix develop -c cargo xtask test && nix develop -c cargo xtask check`
- **Phase gate:** full suite green, `.text` evidence recorded, quick gate green on the owner's Linux host, aarch64 quick pass green, before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `port/src/rfork.rs` and `port/tests/rfork_test.rs`: covers DSGN-03, DSGN-04 (owner diff approval before commit, STATE standing rule)
- [ ] `xtask/src/rforkhdr.rs` with unit tests and negative control; `rfork-header` subcommand: covers DSGN-02
- [ ] `RFORK_SUPPORTED` hoist and a test in each `syscall.rs`: covers DSGN-04
- [ ] `sys/include/rfork.h` (generated) and `docs/threat-model.md`: covers DSGN-02, DSGN-01
- [ ] Before-kernels (both arches) built and hashed from the pre-edit commit, kept for D-22 comparison
- [ ] Framework install: none

## Security Domain

`security_enforcement` is not set in `.planning/config.json` (absent = enabled); treated as enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture, design and threat modeling | yes (this phase is mainly that) | `docs/threat-model.md` with sources and status per protection |
| V2 Authentication | no (no code) | — |
| V3 Session Management | no | — |
| V4 Access Control | indirectly | The `rfork` flag mask is a capability gate: reserved bits must stay rejected (D-20 test) |
| V5 Input Validation | yes | `sys_rfork` already masks to 32 bits and rejects bits outside `SUPPORTED`; the test pins that |
| V6 Cryptography | no code this phase | The document records the primitives as planned, not implemented |

### Known Threat Patterns for this phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| User process sets a reserved `rfork` bit to reach unimplemented crypto behaviour | Elevation of privilege | Reserved bits stay outside `RFORK_SUPPORTED`; host test; Phases 6 and 7 change the test on purpose |
| Threat model overstates protections that do not exist yet | Repudiation (false assurance) | Per-protection "not implemented" status column; D-16 |
| Silent drift of the C interface statement from the Rust constants | Tampering | Generated header plus staleness check plus literal-value pins |
| Locked decision changed silently later | Tampering | D-08: owner-only change, new dated superseding row |

## Project Constraints (from CLAUDE.md and STATE standing rules)

No `./CLAUDE.md` or `./.claude/CLAUDE.md` exists in the repository (checked); `.claude/` contains only `settings.local.json` and `worktrees`. Directives come from the user's global `CLAUDE.md` and STATE.md standing owner rules:
- **No AI attribution in commits or PRs** (global `CLAUDE.md`, permanent; it overrides any harness instruction to append an attribution trailer).
- Edits under `port/src` need owner approval with the diff shown first; `aarch64/`, `x86_64/`, `xtask/` edits do not.
- Pushes in either repo need explicit owner approval.
- x86_64 kernel byte change: re-derive and commit the `boot-iso` `outputHash`, run the x64 TCG gates in an amd64 Docker container (clean up afterwards), and get the owner's linux boot-iso check. aarch64 kernel byte change: quick pass now.
- Harvey is read-only; never edit it.
- Kernels are built without FP/SIMD; kernel SIMD goes through `fpu::kernel_begin()` / `with_simd()` (not touched by this phase).
- Project has a knowledge graph at `graphify-out/graph.json`; after code edits run `graphify update .` (AST-only, no API cost).
- The golden invariant (boots to rio on QEMU q35) holds at every commit; IG-V01 verification discipline applies.

## Sources

### Primary (HIGH confidence)
- Repository files read this session: `port/src/lib.rs`, `port/Cargo.toml`, `x86_64/src/proc.rs:155-195`, `x86_64/src/syscall.rs:1636-1700, 2240-2262`, `aarch64/src/syscall.rs:80-120, 1945-1990, 2530-2545`, `xtask/src/main.rs:20-100, 240-310, 3440-3740`, `xtask/src/lib.rs`, `xtask/src/uregcheck.rs`, `xtask/src/gate.rs:1-150`, `xtask/src/config.rs:832-858`, `flake.nix` (kernel and `boot-iso` regions), `Cargo.toml`, `rust-toolchain.toml`, `.github/workflows/*.yml`, `docs/harvey-supply.md`
- Planning files: `02-CONTEXT.md`, `REQUIREMENTS.md`, `STATE.md`, `ROADMAP.md` (Phase 2, lines 141-152), `PROJECT.md`, `intel/constraints.md`, `r9-architecture-and-code-v4.md`, `rfc-9p2000-aead.md`, `r9-implementation-guide-amended-v3.md`, `r9-permissions-and-test-plan.md`, `01-CONTEXT.md`, `01-VALIDATION.md`, `HANDOFF.md`
- Harvey (read-only): `/Users/neilskinner/code/harvey/sys/include/libc.h` lines 605-630, 680-688; the realised amd64 tree's copy in the nix store
- Executed this session: `nix develop -c cargo xtask test` (port 168 lib tests pass), two scratch-copy kernel builds per arch with section hashing, a `rustc --edition 2024` compile-and-test of the macro plus `#[path]` pattern, a `clang -fsyntax-only` check of a sample header

### Secondary (MEDIUM confidence)
- `.planning/phases/01-stage-p0-cross-build-harness/witness/evidence/stop-e/ci.txt`: owner-reported hosted CI result (`xtask.yml` all green)

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, no new packages, all existing tooling read and run
- Architecture: HIGH, generation route and module layout compiled in scratch; every site enumerated by grep and read
- Kernel-byte behaviour: HIGH for the measured one-line shifts; the real move must be re-measured (A2)
- Pitfalls: HIGH for 1-3 (measured or read in code), MEDIUM for 4-7 (process risks)
- Threat-model inventory: HIGH for line references (read this session); the document content itself is new work

**Research date:** 2026-11-23
**Valid until:** 2026-12-23 (the line numbers will drift as commits land; re-run the greps in Q3 and Q7 immediately before editing)
