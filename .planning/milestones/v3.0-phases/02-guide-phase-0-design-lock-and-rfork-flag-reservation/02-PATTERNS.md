# Phase 2: Guide Phase 0 - Design Lock and rfork Flag Reservation - Pattern Map

**Mapped:** 2026-11-23
**Files analyzed:** 17 (5 new, 12 modified)
**Analogs found:** 16 / 17
**r9 HEAD at mapping:** `5134211` (docs-only commits since the research HEAD `575d093`; every source line number below was re-read at this HEAD)

This file replaces an earlier pattern map built from the superseded research pass. Nothing was carried forward from it; every excerpt below was read from the tracked source in this pass. All analog paths were checked with `git ls-files` (all tracked).

## File Classification

| New/Modified File | New/Mod | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|---------|------|-----------|----------------|---------------|
| `port/src/rfork.rs` | new | model (frozen constants) | compile-time constants | `port/src/dat.rs:55-86` (O* and M* flag blocks) | role-match (no macro-built table exists in `port`) |
| `port/src/lib.rs` | mod | config (module registry) | — | itself, lines 97-102 | exact |
| `port/tests/rfork_test.rs` | new | test (integration) | transform (pure asserts) | `port/tests/fdgroup_test.rs` | role-match |
| `x86_64/src/syscall.rs` | mod | controller (syscall dispatch) | request-response | itself: lines 33, 91-92, 1659-1672, 2246-2251 | exact |
| `x86_64/src/proc.rs` | mod | service (process machinery) | request-response | itself: lines 44-55, 167-181, 2936-2938, 3408-3415 | exact |
| `aarch64/src/syscall.rs` | mod | controller (syscall dispatch) | request-response | itself: lines 32, 87-108, 1967-1980, 2680-2687 | exact |
| `aarch64/src/proc.rs` | mod | service (process machinery) | request-response | itself: line 2536 (and 11 sibling sites) | exact |
| `xtask/src/rforkh.rs` | new | utility (generator + drift check) | file-I/O | `xtask/src/uregcheck.rs` | role-match (uregcheck parses a header; rforkh renders one) |
| `xtask/src/lib.rs` | mod | config (module registry) | — | itself, lines 20-23 | exact |
| `xtask/src/main.rs` (subcommand) | mod | route (CLI wiring) | request-response | itself: `ureg-check` (198-205, 303, 4732-4742) and `gate` flag style (250-273) | exact |
| `xtask/src/main.rs` (`TestStep`) | mod | route (CLI wiring) | batch | itself: lines 3534-3583 | exact |
| `sys/include/rfork.h` | new | generated artefact (C header) | file-I/O (output) | none in r9 | **no analog** |
| `docs/threat-model.md` | new | documentation | — | `docs/devip-contract.md` (authority rule, bracketed marks), `docs/harvey-supply.md` (layout, Sources) | role-match |
| `.planning/PROJECT.md` | mod | planning record | — | itself, lines 112-128 | exact |
| `.planning/ROADMAP.md`, `REQUIREMENTS.md`, `STATE.md` | mod | planning record | — | themselves (wording sites) | exact |
| `flake.nix` (line 983, owner commit) | mod | config (fixed-output hash) | — | itself, lines 975-983; commit `2bc5478` | exact |

`UNSAFE-AUDIT.md` needs no edit (research Q3: line 84 already covers "every other port module").

## Pattern Assignments

### `port/src/rfork.rs` (model, compile-time constants) — NEW, owner approves the diff first

**Analog:** `port/src/dat.rs` (the existing home of Harvey-frozen flag values shared by both arch crates).

**Provenance comment + `pub const` pattern** (`port/src/dat.rs:76-86`):
```rust
/// Mount/bind flag bits (frozen: Harvey sys/include/libc.h:558-564).  The
/// low two bits are the ORDER field (MREPL replaces; MBEFORE/MAFTER union
/// before/after); MCREATE permits creation in a mounted directory; MMASK
/// bounds the valid set (sysfile.c bindmount's (flag&~MMASK) check).
pub const MREPL: u32 = 0x0000;
pub const MBEFORE: u32 = 0x0001;
pub const MAFTER: u32 = 0x0002;
pub const MCREATE: u32 = 0x0004;
pub const MCACHE: u32 = 0x0010;
pub const MORDER: u32 = 0x0003;
pub const MMASK: u32 = 0x0017;
```
Copy: the "(frozen: Harvey sys/include/libc.h:LINES)" citation form, `pub` visibility (consumers are other crates), one constant per line. The same form is at `port/src/dat.rs:55-68` (open modes).

**Differences the new file must carry (not in the analog):**
- Type is `u64`, not `u32`: both `sys_rfork` copies work in `u64` (`x86_64/src/syscall.rs:1652`, `aarch64/src/syscall.rs:1960`), and the constants being replaced are `u64` (`x86_64/src/proc.rs:171-181`).
- The constants come from one `macro_rules! rfork_flags` list that also emits `ALL: &[(&str, u64)]`. No macro-generated constant table exists in `port` today (the only `macro_rules!` there are `println!`/`print!` at `port/src/devcons.rs:95,101`, `netlog_dbg!` at `port/src/devip/netlog.rs:112` and a local `emit!` at `port/src/devip.rs:1079`). Use the compiled text in `02-RESEARCH.md` "Code Examples / `port/src/rfork.rs`" for the macro.
- The file is also compiled into xtask by `#[path]`, so it must use nothing outside `core` and no `crate::` path. The module header must say so (research Pitfall 5).

**Module doc style** — `//!` header naming the donor and the role, as in `port/src/pgrp.rs:1-8`:
```rust
//! pgrp — the per-process namespace group: the mount table.
//!
//! Semantics ported literally from 9k's pgrp.c/chan.c (cmount's list
//! ...
//! MAFTER binds (bind order); MREPL replaces the whole chain.  The frozen
//! M* flag values are re-exported from dat.rs.
```

**Do not copy** `port/src/pgrp.rs:35` (`pub use crate::dat::{MAFTER, ...}`): that is a re-export, which D-18 forbids for the rfork flags.

---

### `port/src/lib.rs` (module registry) — MODIFIED, part of the `port/src` approval

**Analog:** itself. Insert after `pub mod rendez;` (`port/src/lib.rs:97-102`):
```rust
#[forbid(unsafe_code)]
pub mod pgrp;
#[forbid(unsafe_code)]
pub mod rendez;
#[forbid(unsafe_code)]
pub mod virtio;
```
Add exactly:
```rust
#[forbid(unsafe_code)]
pub mod rfork;
```
The crate comment at `port/src/lib.rs:10-19` states the rule ("Every port module carries `#[forbid(unsafe_code)]` EXCEPT the two named floor modules ... plus the ONE named phase-08 bridge module `devip_waker`"). The new module takes the attribute. Modules that got a new-module comment (`devcap` 54-60, `hmacsha1` 61-65) put a short `//` block above the attribute; a one- or two-line comment in that form is optional.

**`#[path]` precedent in this file** (`port/src/lib.rs:44-45`), relevant to the xtask include:
```rust
#[path = "devip/waker.rs"]
pub mod devip_waker;
```

---

### `port/tests/rfork_test.rs` (integration test) — NEW

**Analog:** `port/tests/fdgroup_test.rs` (integration test under `port/tests/`, picked up by `--tests` with no wiring; already discusses rfork flag semantics).

**Header + imports pattern** (`port/tests/fdgroup_test.rs:1-3, 20-28`):
```rust
//! Host-side tests for the 05-06 fd-group semantics (D-03's
//! RFFDG/RFCFDG arms): the per-process fd table as a shareable/copyable
//! group object, and its interplay with devpipe's Arc-shared queue.
...
//! The flag→branch wiring lives in aarch64/src/proc.rs (the arch crate —
//! not host-testable); these tests pin the PORT-crate primitives the
//! wiring mirrors (the 05-05 devenv pattern), and the pipe-queue
//! interplay that Pitfall 4 makes load-bearing.

use port::dat::{Chan, Qid, DEV_CONS, ORDWR};
use port::devpipe;
use port::fdgrp::FdGroup;
use port::namec;
```
Copy: a `//!` header saying what is pinned and where the arch-side half lives (here: the `RFORK_SUPPORTED` mask tests in each arch `syscall.rs`); imports through the public `port::` path (the test is an external crate, so `port::rfork::*`, never `crate::`). Test bodies: use the compiled text in `02-RESEARCH.md` "Code Examples / `port/tests/rfork_test.rs`" (three tests, union pin `0x3c7c7f`).

**Why not a `port/src/*_tests.rs` file:** those exist only where tests need `unsafe` or crate-private items while the module keeps its forbid (`port/src/lib.rs:104-147`). The flag tests need neither, and `port/tests/` is outside the `port/src` approval scope.

---

### `x86_64/src/syscall.rs` (controller, request-response) — MODIFIED

**Analog:** itself.

**Import pattern: port constants arrive with the other `port::` imports** (`x86_64/src/syscall.rs:27-34`):
```rust
use port::dat::{
    Chan, DEV_CAP, DEV_CONS, DEV_DUP, DEV_ENV, DEV_ETHER, DEV_IP, DEV_MNT, DEV_PIPE, DEV_PROC,
    DEV_ROOT, DEV_SRV, DEV_SSL, OEXEC, OTRUNC,
};
use port::mcslock::LockNode;
use port::namec::{self, NamecMode};
use port::pgrp::{MAFTER, MBEFORE, MMASK, MORDER};
use port::println;
```
`MAFTER`/`MBEFORE`/`MMASK`/`MORDER` are the direct precedent: Harvey-frozen flag bits defined once in `port` and imported by name in both arch `syscall.rs` files (`aarch64/src/syscall.rs:32` is the identical line).

**Lines to replace** (`x86_64/src/syscall.rs:91-92`):
```rust
// rfork flags live in proc.rs (the process machinery owns them).
use proc::{RFCFDG, RFCENVG, RFCNAMEG, RFENVG, RFMEM, RFNAMEG, RFNOTEG, RFNOWAIT, RFFDG, RFPROC, RFREND};
```
The stale comment goes; the `use` becomes `use port::rfork::{...}` (eleven names). It may stay at this position or join the `port::` group at 27-34; either keeps the build identical, but the line count differs, which is allowed (D-23: no line balancing).

**Function-local mask to hoist** (`x86_64/src/syscall.rs:1659-1672`):
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
Becomes a module-level `pub(crate) const RFORK_SUPPORTED: u64 = ...` with the same eleven names in the same order, and `if flag & !RFORK_SUPPORTED != 0 { return err(proc, "rfork: unsupported flags"); }` in the function. The preceding check and its string (`"rfork: bad flags"`, lines 1653-1658) and the 32-bit mask at line 1652 are untouched. Module-level constants in this file are plain `const NAME: u64 = ...;` with `///` docs where a note is needed (`x86_64/src/syscall.rs:46-89`).

**Test module pattern** (`x86_64/src/syscall.rs:2246-2251`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::proc::{MappingRecord, ProcGuard, SegKind, Segments};
    use alloc::vec::Vec;
    use port::mem::{PAGE_SIZE_4K, VirtRange};
```
The new `mod rfork_mask_tests` is a separate `#[cfg(test)]` module with named imports (`use super::RFORK_SUPPORTED;`, `use port::rfork::{...}`), the shape of `x86_64/src/proc.rs:3408-3415` and `aarch64/src/syscall.rs:2680-2687`. Body: `02-RESEARCH.md` "Code Examples / Arch side". This module is not compiled on the Mac (research A1); its first compile is on the owner's x86_64 host.

---

### `x86_64/src/proc.rs` (service) — MODIFIED

**Analog:** itself.

**Block to delete, comment included** (`x86_64/src/proc.rs:167-181`):
```rust
// rfork flags (Harvey sys/include/libc.h — frozen values).  Homed here
// (the aarch64 port's home is syscall.rs) so the process machinery —
// rfork_clone/rfork_copy — compiles standalone; syscall.rs consumes
// them from this module.
pub(crate) const RFNAMEG: u64 = 1 << 0;
...
pub(crate) const RFREND: u64 = 1 << 13;
```

**Import block the new `use` joins** (`x86_64/src/proc.rs:44-55`, one `use` per `port` module, alphabetical by module):
```rust
use port::Result;
use port::dat::Chan;
use port::devenv::EnvGroup;
use port::fdgrp::FdGroup;
use port::mcslock::{Lock, LockNode};
use port::mem::{PAGE_SIZE_4K, VirtRange};
...
use port::pgrp::Pgrp;
use port::println;
use port::rendez::RendezGroup;
```
Add `use port::rfork::{RFCENVG, RFCFDG, RFCNAMEG, RFENVG, RFFDG, RFMEM, RFNAMEG, RFNOTEG, RFNOWAIT, RFREND};` after `port::rendez` (ten names; `RFPROC` is not used in non-test code here, and a file-level import of it would warn in the kernel build).

**Test-only import pattern** — `RFPROC` is used at `x86_64/src/proc.rs:3253` inside `mod tests`, which reaches it today through `use super::*;` (`x86_64/src/proc.rs:2936-2938`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
```
Add `use port::rfork::RFPROC;` inside that module. Precedent for a test module importing `port::` items directly (`x86_64/src/proc.rs:3408-3415`):
```rust
#[cfg(test)]
mod fdgrp_release_tests {
    use super::{MAX_FDS, empty_fdgroup, release_fd_group, split_fd_group_in_place};
    use alloc::sync::Arc;
    use port::dat::{Chan, ORDWR};
    use port::fdgrp::FdGroup;
```
Not compiled on the Mac (research Pitfall 3).

---

### `aarch64/src/syscall.rs` (controller, request-response) — MODIFIED

**Analog:** itself, and `x86_64/src/syscall.rs` for the identical shape.

**Block to delete, comments included** (`aarch64/src/syscall.rs:87-108`):
```rust
// rfork flags (Harvey sys/include/libc.h — frozen values).
// pub(crate): proc.rs's rfork_clone reads RFNAMEG/RFCNAMEG
// (the namespace group semantics) — one mirror, one provenance.
// RFENVG/RFCENVG are likewise pub(crate) (the env groups);
// RFFDG/RFCFDG (the fd-group arms).
pub(crate) const RFNAMEG: u64 = 1 << 0;
...
const RFPROC: u64 = 1 << 4;
...
pub(crate) const RFREND: u64 = 1 << 13;
```
Replace with one `use port::rfork::{...}` (eleven names), in the form of `aarch64/src/syscall.rs:32` (`use port::pgrp::{MAFTER, MBEFORE, MMASK, MORDER};`).

**Mask to hoist** (`aarch64/src/syscall.rs:1967-1980`): byte-for-byte the same text as the x86_64 excerpt above, inside `fn sys_rfork(frame: &mut TrapFrame, proc: &mut Proc)` at line 1956. Same hoist, same name `RFORK_SUPPORTED`, same error string.

**Test module pattern** (`aarch64/src/syscall.rs:2680-2687`) — separate named `#[cfg(test)]` modules with explicit `use super::{...}`:
```rust
#[cfg(test)]
mod stat_wire_tests {
    use super::{
        DISPATCH, FWSTAT, FWSTAT_, REMOVE, WSTAT, WSTAT_, remove_chan, valid_stat_wire, wstat_chan,
    };
    use alloc::vec;
    use alloc::vec::Vec;
    use port::dat::OWRITE;
```
Sibling modules: `noted_plan_tests` (2535-2536), `create_cexec_tests` (2604-2605). `mod rfork_mask_tests` follows this naming (`<topic>_tests`) and import style. This leg runs on the Mac under both cfg sets (default and `virtshell`).

---

### `aarch64/src/proc.rs` (service) — MODIFIED

**Analog:** itself. Twelve sites of the form (`aarch64/src/proc.rs:2536`):
```rust
    let nowait = flag & crate::syscall::RFNOWAIT != 0;
```
at lines 2536, 2572, 2598, 2599, 2827, 2829, 2840, 2842, 2853, 2855, 2888, 2896. Each `crate::syscall::RF*` becomes `port::rfork::RF*` (a private `use` in `syscall.rs` is not visible here; a `pub(crate) use` there would be the re-export shim D-18 forbids). Fully-qualified paths keep the line count unchanged in this file; a single `use port::rfork::{...}` at the top is the other valid form.

---

### `xtask/src/rforkh.rs` (utility, file-I/O) — NEW

**Analog:** `xtask/src/uregcheck.rs` (the only existing Rust-versus-C-header check; lives in the xtask lib, step wired in `main.rs`).

**Module header + imports pattern** (`xtask/src/uregcheck.rs:1-10, 19-21, 32-34`):
```rust
//! The cross-repo trap-frame drift gate, dual-arch (D-02b).  Parses
//! the frozen `arm64/include/ureg.h` AND `amd64/include/ureg.h` in
//! ...
//! Plain text parsing — no crate.  The headers are frozen,
//! hand-maintained files, so the parser is deliberately narrow, per
//! arch:

use std::fs;

use crate::{harvey, Arch, DynError, Result};
```
Copy: a `//!` header stating what is compared with what and which decision it serves; `std::fs` only, no new crate; errors through the lib's `Result` (`xtask/src/lib.rs:29-30`: `pub type DynError = Box<dyn std::error::Error>; pub type Result<T> = std::result::Result<T, DynError>;`). `rforkh` needs only `crate::Result`.

**Error-message pattern: tool-name prefix, path, cause** (`xtask/src/uregcheck.rs:99-104`):
```rust
                let text = fs::read_to_string(&header)
                    .map_err(|e| format!("ureg-check: read {}: {e}", header.display()))?;
                compare_to_pinned(&text).map_err(|diff| -> DynError {
                    format!("ureg-check: ureg.h drift detected in {}:\n{diff}", header.display())
                        .into()
                })?;
                println!("ureg-check: 35 fields match arm64/include/ureg.h");
```
Copy: prefix every line with the subcommand name (`rfork-header: ...`), name the file, and print one success line on a match. The research text ("read ...: {e} (run `cargo xtask rfork-header`)", "is stale against port/src/rfork.rs", "matches port/src/rfork.rs") already follows this.

**Negative-control pattern** (`xtask/src/uregcheck.rs:184-214`, called at 134-136):
```rust
                // D-02b: prove the gate can catch drift, not just parse the
                // pristine header — the mutated copy MUST be rejected.
                run_amd64_negative_control(&text)?;
```
```rust
pub(crate) fn run_amd64_negative_control(header_text: &str) -> Result<()> {
    let mutated = header_text.replacen("u64int\tcs;", "u64int\tcseg;", 1);
    if mutated == header_text {
        return Err("NEGATIVE CONTROL FAILED: the mutation did not apply — ...".into());
    }
    match compare_to_pinned_amd64(&mutated) {
        Err(diff) if diff.contains("expected \"cs\"") && diff.contains("found \"cseg\"") => { ... Ok(()) }
        ...
        Ok(()) => Err("NEGATIVE CONTROL FAILED: the mutated amd64 header copy (cs -> cseg) \
             PASSED the check — the gate cannot catch drift; do not relax the parser, fix it".into()),
    }
}
```
For `rforkh` the comparison is whole-string equality, so the equivalent is a unit test that mutates `render()` output in memory (for example `(1<<21)` to `(1<<22)`) and asserts inequality, plus the two CLI failures research ran (missing file, stale file). A runtime control inside `check()` is optional.

**Unit-test pattern: build the expected header text in the test, tabs explicit** (`xtask/src/uregcheck.rs:398-400, 425-438, 462-465`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    ...
    fn frozen_header() -> String {
        let mut s = String::from("typedef struct Ureg {\n");
        for i in 0..=28 {
            ...
                s.push_str(&format!("\tu64int\tr{i};\n"));
        }
        ...
        s
    }

    #[test]
    fn frozen_header_matches_the_pinned_table() {
        assert_eq!(compare_to_pinned(&frozen_header()), Ok(()));
    }
```
Copy: `push_str` + `format!` with literal `\t`, the same string-building style `render()` uses. The renderer test should pin the twelve Harvey enumerator lines literally (tab rule: two tabs after names shorter than eight characters, one otherwise; no comma on the last enumerator) and the declaration line `extern\tint\trfork(int);`. These tests run only under `cargo test -p xtask`, never under `cargo xtask test`.

**Scratch-directory helper for a write/check round-trip test** (`xtask/src/gate.rs:681-686`):
```rust
    fn scratch(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("r9-gate-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
```
`write(workspace)` and `check(workspace)` take the workspace root as a parameter, so a test can pass a scratch directory.

**Cross-crate file inclusion precedent** (`xtask/src/lib.rs:112`), same `../../` base as the `#[path]` include:
```rust
pub const DEFAULT_NETDEV_CONFIG: &str = include_str!("../../x86_64/lib/config_q35shell.toml");
```
The `#[path = "../../port/src/rfork.rs"] #[allow(dead_code)] mod flags;` form itself has no xtask precedent; the in-repo `#[path]` precedent is `port/src/lib.rs:44-45`. Use the compiled text in `02-RESEARCH.md` "Code Examples / `xtask/src/rforkh.rs` (core)".

---

### `xtask/src/lib.rs` (module registry) — MODIFIED

**Analog:** itself (`xtask/src/lib.rs:20-23`):
```rust
pub mod config;
pub mod harvey;
pub mod image;
pub mod uregcheck;
```
Add `pub mod rforkh;` in alphabetical position (after `image`). The module belongs in the lib, not as a bin-local `mod` (as `pcap` and `gate` are at `xtask/src/main.rs:29,34`), because research calls it as `xtask::rforkh::check(...)` and the nix derivations build the lib first.

---

### `xtask/src/main.rs` — subcommand `rfork-header` (route) — MODIFIED

**Analog:** itself. The rule recorded at `xtask/src/main.rs:31-33`: "only the clap block, the dispatch arm and the step constructor touch this file".

**Import line** (`xtask/src/main.rs:6`):
```rust
use xtask::{Arch, DynError, Profile, Result, env_or, harvey, image, uregcheck};
```
Either add `rforkh` here or call `xtask::rforkh::...` by full path (research did the latter).

**Clap block, no-argument form** (`xtask/src/main.rs:197-205`):
```rust
        .subcommand(
            clap::Command::new("ureg-check").about(
                "Verify the Harvey trees' arm64/include/ureg.h and \
                 amd64/include/ureg.h match the pinned layouts (...)",
            ),
        )
```
**Boolean flag form** for `--check` (`xtask/src/main.rs:155-158` and `259`):
```rust
        .subcommand(clap::Command::new("check").about("Runs check").args(&[
            clap::arg!(--json "Output messages as json"),
            clap::arg!(--verbose "Print commands"),
        ]))
```
```rust
                    clap::arg!(--full "Run the moved gate apps in sequence (the full tier, D-14)"),
```
Read with `matches.get_flag("check")` (`xtask/src/main.rs:3528`: `let json_output = matches.get_flag("json");`).

**Dispatch arm** (`xtask/src/main.rs:303-311`):
```rust
        Some(("ureg-check", _)) => UregCheckStep::new().run(),
        ...
        _ => Err("bad subcommand".into()),
    } {
        eprintln!("{e}");
        process::exit(1);
    }
```
An `Err` from the arm prints the message and exits 1; that is the whole failure path for a stale header.

**Step struct** (`xtask/src/main.rs:4732-4742`) and the workspace helper it needs (`4744-4746`):
```rust
struct UregCheckStep {}

impl UregCheckStep {
    fn new() -> Self {
        Self {}
    }

    fn run(self) -> Result<()> {
        uregcheck::run_check().map(|_| ())
    }
}

fn workspace() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR")).ancestors().nth(1).unwrap().to_path_buf()
}
```
Copy for a `RforkHeaderStep { check: bool }` with `new(matches)` reading the flag and `run` calling `xtask::rforkh::check(&workspace())` or `write(&workspace())`. A bare match arm without a struct also works (research ran that form); the struct matches every other subcommand.

---

### `xtask/src/main.rs` — `TestStep` (route, batch) — MODIFIED

**Analog:** itself.

**Where the staleness check goes** (`xtask/src/main.rs:3534-3548`): first statement of `run`, before the leg list is built:
```rust
    fn run(self) -> Result<()> {
        // Each leg: the cargo args plus an optional profile whose cfg set
        // the leg compiles under (None = the crate's bare default cfg).
        let mut all_cmd_args: Vec<(Vec<String>, Option<Configuration>)> = Vec::new();

        all_cmd_args.push((
            vec![
                "test".to_string(),
                "--package".to_string(),
                "port".to_string(),
                "--lib".to_string(),
                "--tests".to_string(),
            ],
            None,
        ));
```
Insert `xtask::rforkh::check(&workspace())?;` above the comment. Precedent for a step calling a lib check as a precondition before its own work (`xtask/src/main.rs:1807`): `uregcheck::check_for_boot_test(self.arch)?;`.

**Host-triple fallback to widen (D-24)** (`xtask/src/main.rs:3552-3563`):
```rust
        let arch = std::env::consts::ARCH;
        let mut target = rustup_state.std_supported_target(arch).map(|t| t.to_string());
        // The aarch64 crate's host tests (HOST-TEST-ROUTE A, Harvey
        // 20-12/20-17): the nix devshell has no rustup, so the rustup
        // probe above finds nothing and the leg used to be skipped
        // silently.  On an aarch64 host the host triple itself carries
        // std (the nix toolchain ships it), so fall back to it.  The
        // fallback is aarch64-only: the x86_64 and riscv64 legs keep
        // their rustup-only selection unchanged.
        if target.is_none() && arch == "aarch64" {
            target = host_triple();
        }
```
Change the condition to `(arch == "aarch64" || arch == "x86_64")` and rewrite the last sentence of the comment, which states the fallback is aarch64-only. Leave the second condition at `xtask/src/main.rs:3580` (`if arch == "aarch64" {` adding the `virtshell` cfg leg) unchanged: it is a different branch with the same test. `host_triple()` is at `xtask/src/main.rs:367-379`. Not runnable on the Mac (research A2).

---

### `docs/threat-model.md` (documentation) — NEW

**Analogs:** `docs/devip-contract.md` (authority rule and bracketed marks) and `docs/harvey-supply.md` (purpose paragraph, numbered `##` sections, Sources list).

**Title + purpose paragraph + authority rule + status statement** (`docs/devip-contract.md:1-18`):
```markdown
# The devip Contract — r9's `/net` File-Semantics Specification

This is the D-03 proof instrument and the D-08 cost-center deliverable: the complete enumerated
devip surface that r9's `#I` device implements over smoltcp 0.14, and the r9-side design decisions
the phase gates assert.  ...

**Authority rule.** The enumeration derives from `sys/man/3/ip`, `sys/man/2/ip`,
`sys/man/8/ipconfig` (the documented surface) CROSSED WITH the Harvey donor source
`sys/src/9k/ip/` (the behavioral truth).  Source beats man wherever they disagree; every such
disagreement is flagged **[man-vs-src]** rather than smoothed over.  ...

**No implementation code here** — format strings, constants, error strings, and decision records
only.

---
```
Copy:
- The bold lead-in paragraphs under the title. For this document: a **Status** paragraph (living document, nothing in it fixed, no protection implemented in code; D-16) and a **Citation rule** paragraph defining the source tags (`[AC 2.2]`, `[RFC 4.3]`, `[IG 3 row 16]`, `[PT 2.3]`) and the one mark for statements with no source.
- The fixed bold bracketed tag used inline wherever it applies: `**[man-vs-src]**` appears in headings and body text (`docs/devip-contract.md:159, 330, 559`). The "new in this document" mark takes the same form, for example `**[new]**`, defined once at the top.
- Do not use the word "locked" anywhere in the file (research Pitfall 9).

**Section numbering and closing Sources list** (`docs/harvey-supply.md:9, 61, 293, 319, 333, 340`):
```markdown
## 1. What the supply is
...
## 6. Venues
...
## Sources
```
Copy: numbered `## N. Title` sections and an unnumbered `## Sources` at the end. Decision IDs are cited in parentheses in running text (`docs/harvey-supply.md:16-18`: "... per D-23)"). Section order: research Q6 "Layout recommendation" (status, scope boundary quoted from AC 2.2 lines 87-88, assets, attacker classes, trust boundaries, protection table, out of scope and remedy, open points and source contradictions).

---

### `.planning/PROJECT.md` Key Decisions (planning record) — MODIFIED

**Analog:** itself (`.planning/PROJECT.md:114-127`).

**Table shape and outcome-cell vocabulary:**
```markdown
| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Source precedence IG > AC > RFC > PT | Owner-approved manifest | Decided |
| `libstd_r9` precedes Track P | Avoids rewriting Harvey userland twice (first `no_std`, then idiomatic) (IG section 9.1) | Carried from IG-D04; confirm in Phase 2 |
| `RFNOLEGACY` is a fourth `rfork` flag, `1 << 21`; ... | Owner decision 2026-11-22, resolving warning W11; ... | Decided 2026-11-22 |
| Succinct TPHT is primary concurrent hashing; Roaring is the set algebra; Funnel is scratchpad only | IG-D09, IG-D10 | Carried constraint; confirm in Phase 2 |
```
Copy: three columns; the IG id sits in the Rationale cell; dated outcomes use the form "Decided 2026-11-22". New outcome wording: "Locked 2026-11-23 (owner)", "Locked amended 2026-11-23 (owner)", "Not locked; deferred to Phase N (W..)". Per D-08 a later change is a new dated row, so rows are added or re-statused, never silently reworded. Row 124 splits into an IG-D09 row and an IG-D10 row. The lead-in at line 114 ("No ADR exists and nothing is locked in this ingest ...") is replaced. Footer form (`.planning/PROJECT.md:131`): `*Last updated: 2026-11-23 after Phase 1 completion (...)*`.

**Wording sites for D-17** (grep at this HEAD, exactly these six): `.planning/ROADMAP.md:147`, `.planning/ROADMAP.md:152`, `.planning/REQUIREMENTS.md:27`, `.planning/PROJECT.md:105`, `.planning/PROJECT.md:123`, `.planning/STATE.md:71`. Full list with the other decisions' sites: `02-RESEARCH.md` Q7 "Every line that must change". Closing check: research Pitfall 10.

---

### `flake.nix` line 983 (config, owner's commit on the Linux host, D-28)

**Analog:** itself (`flake.nix:975-983`):
```nix
            # 2026-11-23 (stop B2 / plan 01-33, owner-approved): the
            # owner's x86_64-linux host built boot-iso at 9af05f8 and
            # measured NAR sha256-xFt7GUshPv+CQNry48fBOAiCsCIuqGZfeC0eX11/8mc=
            # (the long-owed linux check). Re-derived through the
            # owner's standing kernel-byte procedure; replaces the
            # 06b94f8-era TIzw value. The native amd64 host is the TCG
            # venue (the Docker container is not required when the
            # owner's host is already amd64).
            outputHash = "sha256-b6OcAb2pOufpr5AqJj2SqhXnrTYsIQRTgkh9FcgtNT0=";
```
Copy: a dated comment paragraph directly above the hash, naming the stop, the commit the ISO was built at, the measured value and what it replaces. Note the existing paragraph describes the earlier `xFt7...` value; the current `b6OcAb...` value was set by `2bc5478` with no paragraph of its own, so the new paragraph should name `b6OcAb...` as the value it replaces. The plan supplies the single line to change (line 983) and a commit message with no AI attribution. Prior commit subject for form: `2bc5478 witness(01-34): stop C2 returns; boot-iso hash update` (that commit bundled witness returns; this phase's hash commit is its own commit, per `01-33-SUMMARY.md:36` as cited in research Q4).

## Shared Patterns

### Harvey-frozen values: defined once in `port`, imported by name
**Source:** `port/src/dat.rs:76-86`, consumed at `x86_64/src/syscall.rs:33` and `aarch64/src/syscall.rs:32`
**Apply to:** `port/src/rfork.rs`, all four arch files
```rust
use port::pgrp::{MAFTER, MBEFORE, MMASK, MORDER};
```
Named imports, no glob in kernel code, provenance comment at the definition with the Harvey file and line range. (A glob `use port::rfork::*;` is acceptable in `port/tests/rfork_test.rs` only.)

### Per-module `forbid(unsafe_code)` in `port`
**Source:** `port/src/lib.rs:10-21`
**Apply to:** `port/src/lib.rs` (the new `rfork` module declaration)
```rust
#[forbid(unsafe_code)]
pub mod allocator;
```
This is also the HEAD fact the amended IG-D07 row in PROJECT.md must describe (D-06): the crate comment calls `globalalloc` and `mcslock` "floor modules" and `devip_waker` a "named bridge, not floor" (`port/src/lib.rs:42-43`).

### xtask check: lib module does the work, `main.rs` holds only clap block, dispatch arm, step
**Source:** `xtask/src/main.rs:31-33`, `198-205`, `303`, `4732-4742`; `xtask/src/lib.rs:20-23`
**Apply to:** `xtask/src/rforkh.rs`, `xtask/src/lib.rs`, `xtask/src/main.rs`

### xtask error reporting
**Source:** `xtask/src/lib.rs:29-30`, `xtask/src/uregcheck.rs:99-104`, `xtask/src/main.rs:308-312`
**Apply to:** `xtask/src/rforkh.rs`
`Result<T>` with `Box<dyn Error>`; errors built with `format!(...).into()`; message prefixed with the subcommand name and naming the file; `main` prints with `eprintln!` and exits 1. No `panic!`/`unwrap` on the I/O path.

### Arch host tests: named `#[cfg(test)] mod <topic>_tests` with explicit imports
**Source:** `aarch64/src/syscall.rs:2680-2687`, `x86_64/src/proc.rs:3408-3415`
**Apply to:** `rfork_mask_tests` in both `syscall.rs` files; the `RFPROC` import in `x86_64/src/proc.rs` `mod tests`

### Documents: defined bracketed marks, decision IDs in parentheses
**Source:** `docs/devip-contract.md:8-13`, `docs/harvey-supply.md:16-18`
**Apply to:** `docs/threat-model.md`, the `rfork.h` banner wording

### Commit and approval rules (process, applies to every plan)
**Source:** owner's global instructions; `.planning/STATE.md` standing rules; CONTEXT D-17, D-26, D-28
- No AI attribution line in any commit message or PR body.
- `port/src/rfork.rs` and the `port/src/lib.rs` change: show the diff to the owner before committing.
- Nothing is pushed before the end-of-phase stop; then a branch, with owner approval.
- Harvey (`/Users/neilskinner/code/harvey`) is read-only.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `sys/include/rfork.h` | generated C header | file-I/O (output) | r9 has no `sys/` directory, no C header and no committed generated artefact. Its content is produced by `xtask/src/rforkh.rs::render()`. The format reference is Harvey's `sys/include/libc.h:610-625` and `:684` (outside this repository, read-only, not an r9 analog); the exact expected file is in `02-RESEARCH.md` Q8. No include guard (Harvey's `libc.h` has none). The file is written by `cargo xtask rfork-header`, never by hand. |

Partial gaps inside files that do have an analog (planner should take these from `02-RESEARCH.md` "Code Examples"):
- The `macro_rules! rfork_flags` list-to-constants-and-table macro in `port/src/rfork.rs`.
- The `#[path]` include of a `port` source file into xtask.

## Metadata

**Analog search scope:** `port/src`, `port/tests`, `xtask/src`, `x86_64/src`, `aarch64/src`, `docs/`, `flake.nix`, `.planning/{PROJECT,ROADMAP,REQUIREMENTS,STATE}.md`
**Orientation:** two `graphify query` runs (the graph is marked stale for some files; all line numbers above come from direct reads at HEAD `5134211`, not from the graph)
**Files read (targeted ranges):** 14
**Project instructions:** no `./CLAUDE.md` and no project skills directory in the repo
**Pattern extraction date:** 2026-11-23
