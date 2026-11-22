# Phase 20: arm64 catch-up to v2.1 parity - Pattern Map

**Mapped:** 2026-11-17
**Files analyzed:** 22 new/modified files (13 in r9, 9 in Harvey)
**Analogs found:** 20 / 22

Repos: r9 = `/Users/neilskinner/code/r9` at `8e4e730` (clean tree); Harvey = `/Users/neilskinner/code/harvey` at `52a9e0fa0`.
All line numbers below were read at those revisions. Every analog path is git-tracked (checked with `git ls-files` in its own repo).
Scope follows CONTEXT D-03: the must tier, b4-b11, and b12-b16 + c1 (the items research recommended deferring) are all in scope.

Translation rule for every r9 port (x86_64 -> aarch64): `Ureg` -> `TrapFrame`; result slot `frame.ax` -> `frame.x0`; `pml4_*` / `PTE_*` -> `root_table.map_to(...)` / `Entry::rw_user_data()`; `platform::timer::ticks` -> `crate::platform::timer::ticks`; `port::println!` -> `println!`. Argument fetch (`sargs`), `err`, `copy_user_str`, `UserSlice`, `UserSliceMut`, `UserString`, `fd_op_ref`/`fd_op_done`, `fd_take`/`fd_store` exist under the same names on both sides.

## File Classification

### r9 (sibling repo)

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `aarch64/src/main.rs` (hook registration: note_pending x2, user hooks, devcap tick, devssl fd-resolve; bootconf seed merge) | config / provider | event-driven (hook wiring) | `x86_64/src/main.rs:440-567` | exact |
| `aarch64/src/proc.rs` (push_note ready arm, note_pending, Proc.user + fork copy, current_user/set_current_user, empty_* singletons, sweep_husk_payloads, lone-member brk) | service | event-driven / CRUD | `x86_64/src/proc.rs:1086-1091, 1151, 2158-2255, 2349-2373, 2404-2414, 2745, 746-800` | exact |
| `aarch64/src/syscall.rs` (DISPATCH arms, pwrite/pread device arms, sys_rfork fd split, sys_create union + DMDIR, sys_mount DEV_SSL, sys_remove/wstat/fwstat, exit_tail drains, sys_exec shebang entry) | controller | request-response | `x86_64/src/syscall.rs:124-162, 307-404, 762, 860-1025, 1204-1292, 1396-1497, 1529-1597, 1898-2009` | exact |
| `aarch64/src/exec.rs` (`#!` pre-pass, close-on-exec sweep) | service | transform | `x86_64/src/exec.rs:59-214, 540-577` | exact |
| `aarch64/src/platform/virt/bootconf.rs` (NEW - or a section of `platform/virt/devcons.rs`; DTB `/chosen/bootargs` reader + pair parser) | utility | transform | parser: `x86_64/src/platform/q35/mb2.rs:456-555`; DTB read: `aarch64/src/platform/virt/devcons.rs:76-121` | role-match (two analogs composed) |
| `aarch64/src/platform/virt/pci.rs` (b16 cap-extent hardening) | utility | transform | `x86_64/src/platform/q35/pci.rs:945-1010` | exact |
| `aarch64/lib/config_virtshell.toml` (add `"cap"` to `dev`) and a NEW `aarch64/lib/config_virtcpu.toml` (bootconf tracer profile) | config | batch | `x86_64/lib/config_q35cpu.toml` | role-match |
| `aarch64/src/platform/virt/churngate.rs` (NEW, c4) + `aarch64/lib/config_virtchurn.toml` (NEW) | test (kernel-side gate) | batch | `x86_64/src/platform/q35/churngate.rs` + `x86_64/lib/config_q35churn.toml` | role-match |
| inline `#[cfg(test)] mod tests` in `aarch64/src/{syscall,exec,proc}.rs` and the bootconf module (c3) | test | request-response | `x86_64/src/syscall.rs:2093-2160, 2288-2479`; `x86_64/src/exec.rs:906-1034`; `x86_64/src/proc.rs:3070-3110`; `x86_64/src/platform/q35/mb2.rs:896-960` | exact (but see "No Analog Found": host-arch selection) |
| `port/tests/devenv_test.rs:267` | test | CRUD | itself (one literal) | exact |
| `.github/workflows/xtask.yml` (aarch64 `dev_exec` build lane; port integration tests) | config (CI) | batch | same file, `build-aarch64` and `boot-test-x86-64` jobs | exact |
| `xtask/src/main.rs` (`TestStep` at 3438-3446: drop `--lib` restriction or add `--tests`; new aarch64 boot-test marker sets) | utility | batch | same file: 3566-3572 (the `check` step already passes `--lib --tests`), 1025-1034 (virtnotes marker consts), 2983-3001 (bootconf marker assert) | exact |
| `flake.nix` (r9; devShell line 836) | config | batch | same file lines 465-471 (`optionalAttrs ... isLinux`) | exact |

### Harvey

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `build/aarch64/smoke-r9.expect` (new whole-capture post-passes: `not registered`, `unknown device`, `unsupported this era`; optional bootconf leg) | test | event-driven | same file 240-270; `build/x86_64/smoke-r9.expect:419-458` | exact |
| `build/aarch64/smoke-r9-net.expect`, `build/aarch64/tier1-r9.expect` (same post-passes; pushword fast-fail) | test | event-driven | `build/aarch64/smoke-r9.expect:128-135` (`expect_before`) | exact |
| `nix/smoke-r9.sh`, `nix/smoke-r9-net.sh`, `nix/tier1-r9.sh` (repeat-count harness for D-02; bootargs fdtput seam) | utility | batch | `nix/smoke-r9.sh:163-205` (fdtput recipe); `nix/smoke-r9-x64.sh:214-215` (append seam) | role-match |
| `nix/boot-r9.sh` (bootargs seam for the interactive app) | utility | batch | `nix/boot-r9.sh:182-187`; `nix/boot-r9-x64.sh:335-341` | role-match |
| `flake.nix` (Harvey; stale "frozen at the v2.0 pin" comments at 168-170, 242-244; any new arm64 app) | config | batch | same file 114-152 (`mkScriptApp`), 469-473 (hasAttr gate) | exact |
| `flake.lock` | config | batch | commit `4a4f463e1` (lock-only, 6 lines) | exact |
| `.github/workflows/r9-shell.yml`, `.github/workflows/phase6.yml` | config (CI) | batch | `r9-shell.yml` jobs `boot-smoke` / `net-smoke`; `phase6.yml:244-259` (`tier-r9`) | exact |
| `build/harness/src/trace.rs` (D-06 `fversion` distiller; stale `TRACE_OUT` defaults at 511-515) | utility | transform | same file, `REQUIRED_BC` at 159 and the default block at 511-519 | exact |
| `build/aarch64/donor-vm-build.sh`, `build/aarch64/README.md` (document donor re-provision as a prerequisite) | config / docs | batch | `build/aarch64/donor-vm-build.sh` header | exact |
| `.planning/WINDOWS.md` row 49 (reopen) | docs | - | row 49 itself (line 66; JSON mirror at 719-721) | exact |

## Pattern Assignments

### `aarch64/src/main.rs` (provider, hook wiring)

**Analog:** `/Users/neilskinner/code/r9/x86_64/src/main.rs`
**Target:** `exec_bootstrap` at `aarch64/src/main.rs:458`; existing registrations at 473-568.

**b1 - note_pending field** (x86_64 469-474 and 487-492). The aarch64 initializers at 493-497 and 523-527 lack the fourth field:
```rust
    port::devpipe::set_hooks(port::devpipe::PipeHooks {
        current: proc::pipe_current_pid,
        block: proc::pipe_block_current,
        wake: proc::pipe_wake,
        note_pending: proc::note_pending,
    });
```
`DevipHooks` takes the identical line inside the `#[cfg(dev_ip)]` block. `EtherHooks` has no such field on either arch; leave it.

**b3 - user hooks** (x86_64 453-460). Insert after `set_pid_hook` (aarch64 486), not dev-gated and not `platform = "virt"`-gated:
```rust
    port::devcons::set_user_hooks(port::devcons::UserHooks {
        current: proc::current_user,
        set: proc::set_current_user,
    });
```

**b14 - devcap tick hook** (x86_64 461-465):
```rust
    #[cfg(dev_cap)]
    port::devcap::set_tick_hook(platform::timer::ticks);
```
On aarch64 the tick source is spelled `crate::platform::timer::ticks` (see aarch64 528). `cfg(dev_cap)` is already predeclared in the workspace `Cargo.toml:46`; the port registry arm is `port/src/namec.rs:98`.

**b15 - devssl fd resolver** (x86_64 503-509). Insert after `set_resolve_hook` (aarch64 537); unconditional:
```rust
    port::devssl::set_fd_resolve_hook(proc::current_fd_resolve);
```
`proc::current_fd_resolve` already exists at `aarch64/src/proc.rs:2164`.

**c1 - bootconf merge into the ROOT env seed** (x86_64 541-567). Replace the fixed three-pair array at aarch64 582-587:
```rust
    let bootconf = mb2::parse_bootconf_pairs(facts.cmdline())
        .unwrap_or_else(|err| panic!("bootconf: mb2 cmdline parse failed: {err:?}"));
    port::devenv::set_group_hook(proc::current_env_group);
    let mut seed: alloc::vec::Vec<(&[u8], &[u8])> = alloc::vec![
        (b"cputype", b"amd64"),
        (b"nobootprompt", b"embed"),
        (b"readparts", b"0"),
    ];
    seed.extend(bootconf.pairs());
    let root_env = port::devenv::seed_root_group(&seed);
    println!("devenv: seeded cputype=amd64 nobootprompt=embed readparts=0");
    if !bootconf.is_empty() {
        fn show(b: &[u8]) -> &str {
            core::str::from_utf8(b).unwrap_or("<?>")
        }
        port::print!("devenv: bootconf");
        for (k, v) in bootconf.pairs() {
            port::print!(" {}={}", show(k), show(v));
        }
        port::println!();
    }
```
Keep these invariants from the x86_64 comment block (529-540, 552-557): compiled pairs first and loader pairs appended (merge, never replace); an absent or empty string leaves the existing `devenv: seeded cputype=arm64 ...` line byte-identical; a malformed string panics at first boot; the cmdline-sourced keys print on their own `devenv: bootconf` line so the existing gates' bytes do not move.

---

### `aarch64/src/proc.rs` (service)

**Analog:** `/Users/neilskinner/code/r9/x86_64/src/proc.rs`

**b2 - push_note readies a Blocked target** (x86_64 2349-2364; target `aarch64/src/proc.rs:2000-2011`). Add the four lines between `push_back` and `Ok(())`, and carry the doc-comment paragraph from x86_64 2342-2348:
```rust
    p.note_queue.push_back(note);
    if p.state == ProcState::Blocked {
        p.sleep_until = 0;
        p.state = ProcState::Ready;
    }
    Ok(())
```
`ProcState::{Ready, Running, Blocked, Moribund}` is the x86_64 variant set (`state_name`, 2270-2277); research assumption A8 (same names on aarch64) was not opened here, confirm before editing.

**b1 - note_pending** (x86_64 2366-2373), place directly after `push_note`:
```rust
pub fn note_pending() -> bool {
    match current() {
        Some(p) => !p.note_queue.is_empty(),
        None => false,
    }
}
```

**b3 - per-proc user.** Four sites:
- Field (x86_64 1086-1091): `pub user: Vec<u8>,` with the doc comment. aarch64's last Proc field today is `privatemem` (783, initializer 869).
- Constructor (x86_64 1151): `user: b"glenda".to_vec(),`
- Fork copy (x86_64 2742-2745), in `rfork_copy` beside the slash/dot/env copies (aarch64 2523-2571):
```rust
    // The user copies at fork (donor proc.c newproc:
    // `kstrdup(p->user, up->user)`) — no rfork flag governs it; a cap
    // the parent presents does NOT carry to the child's user.
    child.user = parent.user.clone();
```
- Hooks (x86_64 2404-2414):
```rust
pub fn current_user() -> Vec<u8> {
    current().expect("user read with no current process").user.clone()
}

pub fn set_current_user(user: Vec<u8>) {
    current().expect("user set with no current process").user = user;
}
```

**b8 / b10 - exit sweep singletons and husk payload sweep** (x86_64 2174-2255). aarch64 has `empty_fdgroup` already (used at `aarch64/src/syscall.rs:2099`) but no `empty_pgrp`, `empty_env_group`, `empty_rendez_group` or `sweep_husk_payloads` (grep returned nothing). Copy the three lazily-built singletons (each is the `empty_fdgroup` shape, x86_64 2162-2172) and:
```rust
pub fn sweep_husk_payloads(proc: &mut Proc) {
    if Arc::strong_count(&proc.pgrp) == 1 {
        let chans = {
            let node = LockNode::new();
            proc.pgrp.lock(&node).take_mount_chans()
        };
        for c in chans {
            port::namec::close_chan(&c);
        }
    }
    proc.pgrp = empty_pgrp();
    proc.env_group = empty_env_group();
    proc.rgrp = empty_rendez_group();
    let _ = proc.slash.take();
    let _ = proc.dot.take();
}
```
`Pgrp::take_mount_chans` is `port/src/pgrp.rs:83` (already in `port`, no `port/src` edit needed).

**b9 - lone-member RFMEM brk** (x86_64 746-800; target `aarch64/src/proc.rs:368-381`). The aarch64 arm today only moves `segs.bss.0.end`. Before porting, note the arch difference stated in x86_64's own comment (`x86_64/src/syscall.rs:1054-1062`): x86_64 has no trap-context pager, so it maps growth eagerly; aarch64 demand-faults Bss pages (`aarch64/src/fault.rs:103-104`, `page_in_zeroed`). The aarch64 doc comment at 363 calls the lazy end move the intended shape for a one-member family. The planner should treat b9 as "verify whether the defect exists on aarch64" rather than a mechanical copy; if a port is needed, the multi-member loop in the same function (aarch64 382 onward: `allocate_physpage` -> `map_kernel_alias` -> zero -> `map_to` -> `mapping_log.push`) is the in-crate template for mapping one page into one member.

**c2 lead (HVF fork-child defect, D-01).** The code under suspicion is `rfork_copy` (`aarch64/src/proc.rs:2275` onward) and `map_kernel_alias` (2577-2590). The x86_64 counterpart for side-by-side reading is `x86_64/src/proc.rs:2584` onward. No analog for the fix exists; this is a debug plan.

---

### `aarch64/src/syscall.rs` (controller, request-response)

**Analog:** `/Users/neilskinner/code/r9/x86_64/src/syscall.rs`

**b13 - DISPATCH arms** (x86_64 139-143; target table at aarch64 129-162, insert after `FD2PATH`):
```rust
    t[REMOVE as usize] = Some(sys_remove);
    t[WSTAT_ as usize] = Some(sys_wstat);
    t[FWSTAT_ as usize] = Some(sys_fwstat);
    t[WSTAT as usize] = Some(sys_wstat);
    t[FWSTAT as usize] = Some(sys_fwstat);
```
Update the table's doc comment the way x86_64 118-123 does (names the five numbers 25, 26, 27, 44, 45 and what stays loud-None).

**b13 - handlers** (x86_64 860-1025, copy as a block): `valid_stat_wire` (869-891), `remove_chan` (898-903), `wstat_chan` (911-913), `sys_remove` (922-943), `sys_wstat` (951-973), `sys_fwstat` (978-997), `wstat_stage_args` (1005-1025). Core shape:
```rust
fn remove_chan(c: &Chan) -> Result<u64, &'static str> {
    match c.dev {
        DEV_CAP => port::devcap::remove(c).map(|_| 0),
        _ => Err("remove: device does not implement remove"),
    }
}
```
Validation order is fixed: `wstat_stage_args` copies and runs `valid_stat_wire` before any namec or device routing; the `n > 2 + u16::MAX` reject precedes the copy.

**b5 / b14 / b15 - pwrite arms** (x86_64 332-352; target aarch64 323-328). Replace the read-only `DEV_MNT` arm and add two:
```rust
        DEV_MNT => port::devmnt::write(&mut chan, src.as_slice(), off),
        DEV_CAP => {
            let r = port::devcap::write(&mut chan, src.as_slice());
            proc.fd_op_done(fd, chan);
            return match r {
                Ok(wrote) => wrote,
                Err(port::devcap::CapErr::Static(msg)) => err(proc, msg),
                Err(port::devcap::CapErr::InvalidCapability(msg)) => {
                    err_dyn(proc, &String::from_utf8_lossy(&msg))
                }
            };
        }
        DEV_SSL => port::devssl::write(&mut chan, src.as_slice()),
```
`err_dyn` is `x86_64/src/syscall.rs:257`; aarch64 has only `err` (239), so `err_dyn` must be ported with it. The import line needs `DEV_CAP` and `DEV_SSL` (x86_64 28-29 vs aarch64 27).

**b14 / b15 - pread arms** (x86_64 394-396; target aarch64 427-433):
```rust
        DEV_MNT => port::devmnt::read(&mut chan, dst.as_slice_mut(), off),
        DEV_CAP => port::devcap::read(&mut chan, dst.as_slice_mut(), off),
        DEV_SSL => port::devssl::read(&mut chan, dst.as_slice_mut(), off),
```

**b15 - mount accepts a DEV_SSL chan** (x86_64 759-767; target aarch64 859):
```rust
    if (chan.dev != port::dat::DEV_PIPE && chan.dev != port::dat::DEV_SSL)
        || chan.mode & port::dat::OMASK != port::dat::ORDWR
```

**b4 - non-RFPROC fd-group split** (x86_64 1554-1575; target aarch64 1622-1630). Narrow the reject to `RFMEM | RFNOWAIT` and add:
```rust
        if flag & (RFMEM | RFNOWAIT) != 0 {
            return err(proc, "rfork: bad flags for non-RFPROC");
        }
        if flag & RFFDG != 0 {
            let node = LockNode::new();
            let copy = proc.fd_group.lock(&node).deep_copy();
            proc.fd_group = Arc::new(Lock::new("fdgrp", copy));
        } else if flag & RFCFDG != 0 {
            proc.fd_group = proc::empty_fdgroup();
        }
```
`FdGroup::deep_copy` is `port/src/fdgrp.rs:148`. Also correct the stale doc line at aarch64 1594 ("has no measured caller").

**b6 / b7 - sys_create** (x86_64 1396-1497; target aarch64 1409-1510). Three changes:
1. Delete the DMDIR reject at aarch64 1414-1417 (the perm word rides to the device).
2. Add `let nomount = path[0] == b'#';` (x86_64 1427).
3. Swap the probe and create for the union twins (x86_64 1461, 1477-1484) in place of `namec::walk_one` (aarch64 ~1484) and `namec::create_chan` (aarch64 1503):
```rust
    let opened = match namec::walk_one_union(&parent, final_name, &proc.pgrp, nomount) {
        ...
        Err(_) => match namec::create_chan_union(
            &parent,
            final_name,
            mode & !(port::dat::OEXCL | port::dat::OCEXEC),
            perm,
            &proc.pgrp,
            nomount,
        ) {
```

**b8 / b10 - exit_tail** (x86_64 1898-1975; target aarch64 2048-2121). Four changes:
1. `sys_exits` copies the message into a stack buffer and drops the Vec before the diverging call (x86_64 1910-1919):
```rust
    let mut sbuf = [0u8; ERRMAX];
    let n = msg_vec.len().min(ERRMAX);
    sbuf[..n].copy_from_slice(&msg_vec[..n]);
    drop(msg_vec);
    exit_tail(proc, &sbuf[..n]);
```
2. Owning drain, one chan at a time (x86_64 1937-1959), replacing the borrowed loop at aarch64 2101-2107:
```rust
        let mut drained = { let node = LockNode::new(); detached.lock(&node).drain_all() };
        for c in drained.drain(..) {
            port::namec::close_chan(&c);
            drop(c);
        }
        drop(drained);
```
3. `drop(detached);` before the tail (x86_64 1963).
4. `proc::sweep_husk_payloads(proc);` after the three queue resets and before `proc.state = ProcState::Moribund` (x86_64 1971-1975).

The `#[cfg(dev_churn)]` heap marker at x86_64 1986-1990 belongs to c4 only.

**b12 - sys_exec entry** (x86_64 1204-1292; target aarch64 1172-1292). x86_64 factored the body into `copy_exec_args` (267) and `exec_backing_for` (1204-1254) so the `#!` pre-pass can re-resolve the interpreter; aarch64 still has both inline. Port the factoring, then the call:
```rust
    match exec::exec_with_shebang(
        proc,
        backing,
        &path,
        name,
        &arg_slices,
        frame,
        &mut |proc, path| exec_backing_for(proc, path),
    ) {
        Ok(tos_va) => tos_va,
        Err(msg) => err(proc, msg),
    }
```

---

### `aarch64/src/exec.rs` (service, transform)

**Analog:** `/Users/neilskinner/code/r9/x86_64/src/exec.rs`

**b12 - `#!` pre-pass** (x86_64 59-214): consts `SHEBANG_LINE_BOUND = 32`, `MAX_SHEBANG_DEPTH = 2`; `struct ShebangExec`; the pure `shebang_pass(head, n, file, elem, caller_args)` (101-160); and `exec_with_shebang` (173-214) which loops header-read -> `shebang_pass` -> `resolve` and ends in `exec_replace(proc, backing, name, &final_args, frame)`. Donor argv order is `[elem, #!tokens[1..], file, caller.argv[1..]]`. The frame parameter type becomes `&mut TrapFrame`. Check that aarch64's `MAX_ARGS` matches x86_64's 32 (`exec.rs:59`).

**b11 - close-on-exec sweep** (x86_64 540-577; target `exec_replace` at aarch64 294, between `check_seed_fit` at 309 and `free_user_mappings` at 315):
```rust
fn close_on_exec_fds(proc: &mut Proc) {
    let node = port::mcslock::LockNode::new();
    let taken = proc.fd_group.lock(&node).take_cexec();
    for c in taken {
        port::namec::close_chan(&c);
    }
}
```
`FdGroup::take_cexec` is `port/src/fdgrp.rs:82`; the `cexec` mark is set in `port/src/namec.rs:437-441`, so opens on arm64 already record it and only the sweep is missing.

---

### `aarch64/src/platform/virt/bootconf.rs` (NEW; utility, transform) - c1

Two analogs composed. No `bootargs` reader exists in `aarch64/src` today.

**Parser analog:** `/Users/neilskinner/code/r9/x86_64/src/platform/q35/mb2.rs:456-555`. `Bootconf<'a>` holds borrowed `(key, value)` slices in a fixed array (`MAX_BOOTCONF_PAIRS = 32`, line 61; `MAX_CMDLINE = 256`, line 52) with `pairs()` and `is_empty()`. `parse_bootconf_pairs` rules (488-504): whitespace-separated tokens; a token with `=` opens a pair split at the first `=`; a token without `=` extends the previous value (so `init=/arm64/init -c` is one pair); a leading continuation or empty key is a named error; the empty string is zero pairs. Errors are `ParseError::{InvalidBootconfToken, TooManyBootconfPairs, CmdlineTooLong}` (119-131). The parser is pure `&[u8] -> Result`, so it can be copied into the aarch64 crate unchanged; moving it into `port/src/` to share it would re-stale the x86_64 FOD (CONTEXT discretion note, RESEARCH Pitfall 3), so duplicate rather than hoist.

**DTB read analog:** `/Users/neilskinner/code/r9/aarch64/src/platform/virt/devcons.rs:76-121` (`rng_seed_read`). Same `/chosen` node, same lazy re-parse off the recorded `DTB_VA`:
```rust
    let Some(chosen) = dt.find_by_path("/chosen") else {
        return 0;
    };
    let Some(prop) = dt.property(&chosen, "rng-seed") else {
        return 0;
    };
    let Some(bytes) = dt.property_value_bytes(&prop) else {
        return 0;
    };
```
Copy the `// SAFETY:` comment shape at 94-97 for the raw DTB slice (the crate denies `clippy::undocumented_unsafe_blocks`, RESEARCH Pitfall 7). `bootargs` is a NUL-terminated string property: cut at the first NUL the way `mb2.rs:442-447` (`parse_cmdline`) does, and reject strings over `MAX_CMDLINE`. An absent `/chosen` or absent property is the no-op case. A second in-crate precedent for reading `/chosen` eagerly while `dt` is in scope is the initrd discovery at `aarch64/src/main.rs:179-186`.

**Lifetime difference to plan for:** x86_64's `facts.cmdline()` borrows from `Facts`, which `exec_bootstrap` receives by reference. aarch64's `exec_bootstrap(initrd_img)` receives no DTB handle, so either pass the string in as a new parameter (as `initrd_img` is), or copy into a fixed `[u8; MAX_CMDLINE]` buffer the way `Facts` does (`mb2.rs:224, 448-452`).

---

### `aarch64/src/platform/virt/pci.rs` (utility) - b16

**Analog:** `/Users/neilskinner/code/r9/x86_64/src/platform/q35/pci.rs:945-1010` (`check_cap_extents`).

Correction to RESEARCH b16: aarch64 already calls the shared parser (`aarch64/src/platform/virt/pci.rs:572`, `virtio::Caps::parse(&ecam_cfg)?`) and has an inline bar4 + extent check at 573-579. What it lacks is the rest of x86_64's hardening: the zero-length reject, the device cap inside the checked set, `common.length < 0x40`, and `device.length < 8`. Port `check_cap_extents` as a pure fn (x86_64 gates it `#[cfg(any(test, dev_ether))]`) and call it after `Caps::parse`. Its host pins are `x86_64/src/platform/q35/pci.rs:1535-1615` (`live_caps()` fixture).

---

### `aarch64/lib/config_virtshell.toml` and NEW `config_virtcpu.toml` (config)

**Analog:** `/Users/neilskinner/code/r9/x86_64/lib/config_q35cpu.toml`

Today: `dev = ["exec", "bootchain", "ether", "ip"]` (virtshell). x86_64's cpu profile: `dev = ["exec", "bootchain", "cap", "ether", "ip"]`. Without `"cap"` the `#[cfg(dev_cap)]` registry arm (`port/src/namec.rs:98`) and the tick hook are not compiled, so b14 would be dead code on arm64. Changing virtshell changes the packaged `kernel-virt`; a separate profile keeps the three existing gates on an unchanged kernel. The planner must pick one and say which.

x86_64 delivers bootconf with a `[dist] append = "init=/amd64/init -c"` key (xtask validates and substitutes it, `xtask/src/main.rs:3991-4002, 4034-4085`). aarch64 has no `[dist] append` consumer; its delivery point is the `[qemu] dtb` / initrd patch path that already writes `/chosen linux,initrd-start/end` into a DTB copy (see `config_virtnotes.toml`'s `initrd = { file = ..., addr = 0x48000000 }` comment). The header comment of `config_q35cpu.toml` is the template for documenting the seam, the control leg (the no-append profile stays byte-untouched) and what the gate asserts.

---

### `port/tests/devenv_test.rs` (test)

Line 267: `assert_eq!(n, 10);` -> `8` (`b"arm64-r9"` on line 266 is 8 bytes). This is under `port/tests/`, not `port/src/`, so it does not touch kernel bytes.

---

### `.github/workflows/xtask.yml` (r9 CI)

**Analog:** the same file. The `build-aarch64` job runs `cargo xtask build --arch aarch64 --verbose` (default config = raspi3b, no `dev_exec`). Add a lane in the same shape:
```yaml
  build-aarch64:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3
    - name: Build
      run: cargo xtask build --arch aarch64 --verbose
```
with `--config virtshell` (and any new profile). If an aarch64 boot-test lane is added, copy the `boot-test-x86-64` job's Harvey checkout step (`repository: requaos/harvey`, `path: harvey`, `R9_HARVEY_DIR: ${{ github.workspace }}/harvey`) and its `timeout-minutes` rationale. Ordering constraint from RESEARCH: Harvey `master` must be pushed first, because `ureg-check` reads `arm64/include/ureg.h` and `origin/master` still has `aarch64/`.

**Port integration tests:** `xtask/src/main.rs:3438-3446` builds `["test", "--package", "port", "--lib"]`. The `check` step at 3566-3572 already uses `--lib --tests --benches`; mirror that so `port/tests/*.rs` runs under `cargo xtask test`.

---

### `flake.nix` (r9 devShell)

Line 836: `packages = [ toolchain pkgs.qemu pkgs.dtc pkgs.grub2 pkgs.xorriso ];`. In-file precedent for linux-gating is lines 465-471 (`nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux { ... }`). List form: `++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.grub2 pkgs.xorriso ]` (Harvey's `flake.nix:116-118` uses exactly this `pkgs.lib.optionals isLinux [...]` idiom). The `devShells` block is outside the kernel derivations.

---

### `build/aarch64/smoke-r9.expect` and siblings (Harvey; test)

**Analog for the mechanism:** the same file, lines 240-270. Whole-capture post-passes run after `log_file` is closed:
```tcl
log_file                           ;# stop + flush before the whole-file scan
set fh [open $env(R9_SMOKE_LOG) r]
set whole [read $fh]
close $fh
if {[string match "*#p/0/ctl*" $whole]} {
    puts stderr "smoke-r9: pid-0 ctl path present in the capture — getpid() regressed"
    cleanup
    exit 1
}
puts stderr "smoke-r9: stage ok — pid-warning-absent (no #p/0/ctl in the capture)"
```
Add absence checks in this shape for `hooks not registered`, and per RESEARCH Pitfall 2 for `unknown device` and `unsupported this era`. Extend the final `ALL STAGES PASS` line (270) with each new stage name.

**Analog for a presence + absence pair on a bootconf leg:** `/Users/neilskinner/code/harvey/build/x86_64/smoke-r9.expect:419-458`, which uses `string first` and asserts both `devenv: bootconf init=/amd64/init -c` present and `panic` / `cpurc: FAIL` absent. The staged form is line 389: `stage cpu-bootconf {devenv: bootconf init=/amd64/init -c}`.

**Fast-fail for the pushword defect (D-01):** the `expect_before` block at `build/aarch64/smoke-r9.expect:128-135` is where a marker turns a 600 s stage timeout into an immediate named failure:
```tcl
expect_before {
    -re {panic|Unhandled interrupt} {
        puts stderr "smoke-r9: kernel panic marker on the wire — aborting"
        cleanup
        exit 1
    }
}
```
A `pushword but no argv` arm here makes N-run HVF series cheap. The same block exists in `smoke-r9-net.expect` and `tier1-r9.expect`.

---

### `nix/smoke-r9.sh`, `nix/boot-r9.sh` (Harvey; DTB bootargs seam)

**Analog:** `/Users/neilskinner/code/harvey/nix/smoke-r9.sh:188-204` (identical recipe at `nix/boot-r9.sh:182-187`):
```sh
cp "$R9_DTB_SRC" "$DTB"
chmod u+w "$DTB"
...
fdtput -t x "$DTB" /chosen linux,initrd-start "$HI" "$LO" \
	|| err "fdtput linux,initrd-start patch failed"
```
The bootargs patch is one more `fdtput` on the same writable copy (`-t s` for a string property), with the same `|| err "..."` tail. The x86_64 counterpart of the seam is `nix/smoke-r9-x64.sh:214-215` and `nix/boot-r9-x64.sh:335-341` (`--append="init=/amd64/init -c"`); copy its discipline that the no-append path stays the default and is byte-unchanged (`boot-r9-x64.sh:55`).

**Repeat-count harness (D-02):** no existing script loops a gate N times. The env contract to build on is `R9_ACCEL` (auto|hvf|kvm|tcg), `R9_GICV2=1`, `R9_SKIP_BUILD=1`, and the per-run `R9_RUN_TAG` + `R9_SMOKE_LOG` (`nix/smoke-r9.sh:172-173, 207-213`); the log is `rm -f`'d per run, so a loop must copy each capture aside before the next iteration.

---

### `flake.nix` / `flake.lock` (Harvey)

New arm64 apps follow `mkScriptApp pkgs "<name>" ./nix/<script>.sh '' export R9_KERNEL_PKG="${r9.packages.${system}.kernel-virt}" '' ([...])` (114-152) and are listed inside the `optionalAttrs (builtins.hasAttr system r9.packages)` block at 469-473. Stale comments to update: 168-170 and 242-244 ("frozen at the v2.0 pin").

Pin advance: its own commit touching `flake.lock` only. Subject shape from `git log -- flake.lock`:
`chore(NN-PP): advance the r9 pin <old> -> <new> lock-only (<why>)` (e.g. `4a4f463e1`). No attribution trailer (user global rule).

---

### `.github/workflows/r9-shell.yml`, `phase6.yml` (Harvey CI)

Existing arm64 jobs: `boot-smoke`, `net-smoke` (`r9-shell.yml`), `tier-r9` (`phase6.yml:244-259`). Any new job copies the pinned-SHA steps verbatim:
```yaml
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4 — API-resolved 2026-11-06 (the 06-08 rule)
      - name: install nix
        uses: DeterminateSystems/nix-installer-action@3138316df39ed29be04236d7ffc686fa525866aa
```
`timeout-minutes: 90`, `permissions: contents: read`. `tier-r9` shows the log-upload step (`actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02`, `if: always()`), worth copying to the smoke jobs so a hosted pushword fire leaves a capture. Hosted runners are TCG; HVF evidence (D-01) can only come from the darwin host.

---

### `build/harness/src/trace.rs` (Harvey; D-06)

`REQUIRED_BC` at line 159 is `&["bind", "pipe", "fversion", "mount", "dup"]`; the failing assert (`measured set lacks fversion`) checks the distilled set against it. The stale artifact defaults are at 511-515 (paths under `.planning/phases/04-...`, `05-...`, `06-...`, since archived); `TRACE_OUT` at 519 is the override every run in this phase must set. RESEARCH's single observation is that the one `Fversion` record was glued to the tail of another pid's fragment on the same line; the fix site, if the rerun repeats, is the distiller's line splitting in this file. A shell-side distiller exists for the amd64 venue at `build/x86_64/trace-distill.sh` and is worth reading for how it splits records.

## Shared Patterns

### Hook registration (dependency inversion)
**Source:** `/Users/neilskinner/code/r9/x86_64/src/main.rs:440-527`; aarch64 twin at `aarch64/src/main.rs:469-568`
**Apply to:** b1, b3, b14, b15
All hooks register once in `exec_bootstrap` before the first exec; a second registration asserts. Each registration carries a comment naming the plan/requirement and why it is or is not cfg-gated. Device-wide hooks are ungated; `devcap` is `#[cfg(dev_cap)]`; `devether`/`devip` are `#[cfg(dev_ether)]`/`#[cfg(dev_ip)]`.

### Device arm in a syscall match
**Source:** `/Users/neilskinner/code/r9/x86_64/src/syscall.rs:319-358`
**Apply to:** every new `DEV_*` arm (b5, b14, b15) and `remove_chan`/`wstat_chan`
The fd is held by `fd_op_ref` and returned by `fd_op_done` on every path, including early returns inside an arm. Unknown devices end in a loud named `Err("...: unknown device")`, never a silent 0.

### User memory access
**Source:** `/Users/neilskinner/code/r9/x86_64/src/syscall.rs:1005-1025` (`wstat_stage_args`)
**Apply to:** every new syscall handler
`UserSlice::new(proc, va, n)` / `UserString::copy_from(proc, va, ERRMAX)` only; copy to a kernel `Vec` and validate before routing. Int-width args are masked `& 0xffff_ffff`. No raw pointer reads (RESEARCH Security Domain, V5).

### SAFETY comments
**Source:** `/Users/neilskinner/code/r9/aarch64/src/platform/virt/devcons.rs:94-97`; `aarch64/src/proc.rs:372-374`
**Apply to:** every `unsafe` block touched in `aarch64/`
The crate denies `clippy::undocumented_unsafe_blocks`, and `cargo xtask clippy` does not set `dev_exec`, so CI will not catch a miss in this code today.

### Stay out of `port/src/`
**Source:** CONTEXT "Claude's Discretion"; RESEARCH Pin-Advance Protocol
**Apply to:** all r9 plans
Every `port` API the ports need already exists: `devmnt::write`, `devcap::{read,write,remove,set_tick_hook,CapErr}`, `devssl::{read,write,set_fd_resolve_hook}`, `devcons::{set_user_hooks,UserHooks}`, `namec::{walk_one_union,create_chan_union}`, `FdGroup::{deep_copy,drain_all,take_cexec}`, `Pgrp::take_mount_chans`, `virtio::Caps`. Nothing mapped above requires a `port/src/` edit. The one tempting case is sharing the bootconf parser; duplicate it instead. `port/tests/` is safe.

### Pin protocol
**Source:** `.planning/phases/19-cpu-service-drawterm-rio-desktop/19-PATTERNS.md:428-430`; Harvey commit `4a4f463e1`
**Apply to:** the closing plan
Push r9, observe CI, then a lock-only Harvey commit, then re-green the gates without `--override-input`. During development: `nix run .#<app> --no-write-lock-file --override-input r9 git+file:///Users/neilskinner/code/r9`, under `bash`, with new r9 files `git add`ed first.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| HVF fork-child fix (location unknown; suspects `aarch64/src/proc.rs:2275-2590`) | service | - | Root cause not established; x86_64 has no equivalent defect to copy a fix from. Debug plan, not a port |
| N-consecutive-run gate wrapper (D-02) | utility | batch | No script in either repo repeats a gate and tallies results; build on the env contract in `nix/smoke-r9.sh` |

Partial gaps the planner should size rather than assume:

| Item | Gap |
|------|-----|
| c3 aarch64 host-test battery | The test bodies have exact x86_64 analogs, but `aarch64/src` has zero `cfg(test)` and `cargo xtask test` only tests the host-arch crate when rustup reports a std target. The x86_64 `MockStack` fixture (`syscall.rs:2110-2152`) depends on `new_user_proc` working on a host without a live page table; whether aarch64's `new_user_proc` (`proc.rs:1226`) is host-runnable was not established. Pure fns (`shebang_pass`, `valid_stat_wire`, `parse_bootconf_pairs`, `check_cap_extents`) are the low-risk subset |
| c4 arm64 churn gate | `churngate.rs` is q35-specific in its bootstrap (`bootstrap_and_run(facts)` selected at `x86_64/src/main.rs:292-296`, timer at `platform::q35::timer`), needs `allocator::{heap_census,heap_stats}` call sites, staged `churnmark`/`churnsem` binaries (`build/x86_64/churn*.c`), xtask marker consts (`xtask/src/main.rs:866` onward) and a config in xtask's embedded list (`xtask/src/lib.rs:145`). This is a multi-file port, not one file |
| b9 lone-member brk | May not be a defect on aarch64 (demand pager present); see the note under `aarch64/src/proc.rs` |
| Harvey-side consumer for bootconf, `#!`, devcap, devssl | arm64 has no `rc/bin/cpurc`, no `bin/service/tcp17010`, no tier-2 list (`build/aarch64/root-tree/rc/bin` holds `termrc` only). The x86_64 fixtures are `build/x86_64/root-tree/rc/bin/cpurc` and the 19-03 marker fixture described in `config_q35cpu.toml`'s header. Without a small arm64 fixture these kernel arms land with no live gate, which is the situation D-04 asks the planner to flag |

## Metadata

**Analog search scope:** r9 `aarch64/src`, `x86_64/src`, `port/src` (API presence only), `port/tests`, `xtask/src`, `aarch64/lib`, `x86_64/lib`, `.github/workflows`, `flake.nix`; Harvey `build/aarch64`, `build/x86_64`, `build/harness/src`, `nix/`, `flake.nix`, `.github/workflows`, `.planning/WINDOWS.md`, `.planning/phases/19-*/19-PATTERNS.md`
**Files scanned:** 31 (ranges read, none in full except the two required-reading files)
**Pattern extraction date:** 2026-11-17
**Not verified:** nothing was compiled or run; `ProcState` variant names and `MAX_ARGS` on aarch64 were not opened; the knowledge graph was reported stale for the phase files, so Harvey-side findings come from direct reads after the required `graphify query`.
