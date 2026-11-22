# Phase 20: arm64 catch-up to v2.1 parity - Research

**Researched:** 2026-11-17
**Domain:** r9 aarch64 kernel wiring against the shared `port` crate; Harvey arm64 composed gates; cross-repo pin protocol
**Confidence:** HIGH for everything marked RAN (executed this session on the darwin-arm64 host); MEDIUM for READ (source read, not executed); LOW for INFERRED

**Evidence legend used throughout** (the orchestrator asked for observed vs inferred to be kept apart):

| Mark | Meaning | Provenance tag |
|------|---------|----------------|
| RAN | I executed it this session and read the output | `[VERIFIED: <what ran>]` |
| READ | I opened the file with Read this session; values quoted verbatim | `[VERIFIED: path:lines]` |
| REC | Taken from a planning record I read, not re-checked | `[CITED: path]` |
| INFERRED | Reasoned, not run | `[ASSUMED]` |

No CONTEXT.md exists for this phase (the user skipped discuss-phase), so there is no `<user_constraints>` block. The only locked inputs are the ROADMAP Phase 20 entry and the orchestrator's additional context.

## Summary

The r9 aarch64 drift is far smaller at compile level than expected and larger at behavior level than the two known errors suggest. The two `note_pending` errors are the **only** compile errors: a 9-line patch (two initializer fields plus a 7-line `proc::note_pending`) makes `cargo xtask build --arch aarch64 --config virtshell` succeed, and with that kernel `nix run .#boot-r9-smoke` printed `ALL STAGES PASS` on this host six times out of six under HVF, plus once under TCG gicv3 and once under the gicv2 reference shape `[VERIFIED: nix run .#boot-r9-smoke --override-input r9 git+file://<scratch clone>]`. The capture shows `devenv: seeded cputype=arm64` and `paqfs: arm64`, so the objtype rename is proven end to end through boot, init, and the `% ` prompt.

Three things sit behind that green result and decide the plan. First, a behavior regression is already visible on the wire in every arm64 capture: `setting #c/hostowner to glenda: devcons: user hooks not registered` and `init: can't read #c/user: devcons: user hooks not registered`. The shared `port` crate gained per-proc user hooks in phase 18-02 and aarch64 never registered them; the smoke gate does not assert on it. Second, WINDOWS #49 (`rc: pushword but no argv!`), closed in 17-06 on an audit with a live-fire reopener, fired live four times in thirteen HVF boots this session, always in a freshly forked child, and each fire turned a gate red; zero fires in four TCG boots. Third, two carried-in checks are red for rename reasons: `cargo test -p port` fails one assertion (`port/tests/devenv_test.rs:267` still expects 10 bytes for the 8-byte `arm64-r9`), and the donor venue needs `donor-vm-build.sh` re-run before `subset-exec.sh` works.

The drift went unnoticed because no CI lane compiles the aarch64 crate with `dev_exec`: r9's `xtask.yml` builds aarch64 with the default raspi3b config only, and `cargo xtask check`/`clippy` use no `dev_*` cfg keys.

**Primary recommendation:** Scope Phase 20 as (1) the compile fix with the full donor note semantics, (2) the user-hooks regression, (3) the two rename fixes, (4) CI and devshell guards so this cannot recur, and (5) the push-before-pin advance with the four arm64 gates green at the pin under TCG. Treat the HVF fork-child defect as its own debug plan with an explicit owner decision on whether it gates the phase. Defer the Harvey-side cpu/auth/drawterm port to its own phase, as STATE.md already records.

## Architectural Responsibility Map

| Capability | Primary tier | Secondary tier | Rationale |
|------------|--------------|----------------|-----------|
| Device semantics (devpipe, devmnt, devip, devssl, devcap, namec) | r9 `port/` (arch-neutral kernel) | — | arm64 inherits these once it compiles; no arm64 work |
| Hook registration, syscall DISPATCH, proc/exec/exit paths | r9 `aarch64/` (arch crate) | r9 `port/` | Each arch owns its own `main.rs`, `syscall.rs`, `proc.rs`, `exec.rs`; x86_64's were ported from aarch64 at 14-04 and then diverged |
| Boot configuration delivery (`service=cpu`) | r9 `aarch64/` (DTB `/chosen/bootargs`) | Harvey nix app (DTB patch) | x86_64 uses the mb2 cmdline; the arm64 analogue does not exist yet |
| Userland build, image bake, expect gates | Harvey `build/aarch64/`, `nix/*.sh` | lima guest (namespace build) | Harvey owns composition; r9 arrives only as the flake input `kernel-virt` |
| Pin and reproducibility | Harvey `flake.lock` | r9 `flake.nix` (x86_64 boot-iso FOD) | Push-before-pin, lock-only commit |
| CI | r9 `.github/workflows/xtask.yml`, Harvey `r9-shell.yml` + `phase6.yml` | — | Both need an arm64 `dev_exec` lane |

## Measured Start State

All of this was run on 2026-11-17 at r9 `8e4e730` and Harvey `5392c684e`. Neither repo's tracked or untracked state changed: `git status --porcelain` was byte-identical before and after for Harvey, and empty for r9 `[VERIFIED: git status diff]`. The kernel patch lives only in a scratch clone.

### What the two records got slightly wrong

| Record | Says | Measured |
|--------|------|----------|
| ROADMAP Phase 20, orchestrator context | `PipeHooks.note_pending` added in r9 `56acaea` | Added in `1f14a50` (`git log -S'note_pending: fn' -- port/src`). `56acaea` is the next commit (devpipe blocking write) `[VERIFIED: git log -S]` |
| STATE.md Blockers, quick 261117-bz5 SUMMARY | r9 `8e4e730` is LOCAL ONLY; pin not advanced | r9 `main` equals `origin/main` at `8e4e730`, and Harvey `flake.lock` already pins `8e4e730abc7648e43ee38645e5061b5e03cb322f` (Harvey commit `4a4f463e1`) `[VERIFIED: git branch -vv; flake.lock:45]` |
| quick 261117-bz5 SUMMARY | r9 `cargo test -p port` not run | Run this session: one failure, see below |

Harvey `master` is 6 commits ahead of `origin/master`; the remote-tracking tree still has `aarch64/` and no `arm64/` `[VERIFIED: git ls-tree origin/master]`. The rename has not reached hosted CI.

### The v2.0 pin

The last pin before the 2026-11-06 pivot is r9 `dbea82f` (Harvey commit `246da54f4`, "pin r9 dbea82f — exec MAX_ARGS 16->32") `[VERIFIED: git log -- flake.lock]`. Between `dbea82f` and `8e4e730` r9 has 144 commits; only 3 touch `aarch64/` (`de1fba1` set_private, `fe67c75` identity-keyed mounts, `8e4e730` rename) `[VERIFIED: git log dbea82f..HEAD -- aarch64]`. The first Harvey pin that contains `1f14a50` is `aa55a27` (Harvey `dfb6e5ab1`, 2026-11-15), so the arm64 leg has been uncompilable at the pin since then `[VERIFIED: git merge-base --is-ancestor]`.

### Compile drift: exactly two errors

`[VERIFIED: cargo xtask build --arch aarch64 --config virtshell in the scratch clone, nightly rustc 1.95.0-nightly (842bd5be2 2026-01-29)]`

- Unpatched: 2 errors, both E0063, at `aarch64/src/main.rs:493` and `:523`.
- With the probe patch below: build exits 0. No further errors appear behind the first two.

The probe patch (the complete diff applied in the scratch clone):

```rust
// aarch64/src/main.rs — both initializers gain one field
        wake: proc::pipe_wake,
        note_pending: proc::note_pending,

// aarch64/src/proc.rs — new fn, copied from x86_64/src/proc.rs:2368-2373
pub fn note_pending() -> bool {
    match current() {
        Some(p) => !p.note_queue.is_empty(),
        None => false,
    }
}
```

Source of the field name and shape `[VERIFIED: /Users/neilskinner/code/r9/port/src/devpipe.rs:131-138]`:

```rust
pub struct PipeHooks {
    pub current: fn() -> u64,
    pub block: fn(),
    pub wake: fn(&[u64]),
    /// The blocking loops' note probe (the donor's notepending check
    /// around sleep): true = a queued note interrupts this read.
    pub note_pending: fn() -> bool,
}
```

`DevipHooks` at `port/src/devip/stack.rs:112-119` has the identical four fields `[VERIFIED: Read]`. The aarch64 initializers at `aarch64/src/main.rs:493-497` and `:523-527` carry only `current`, `block`, `wake` `[VERIFIED: Read]`.

The probe patch is **not** the right fix on its own. It wires the probe without the other half of `1f14a50`. The x86_64 `push_note` readies a blocked target `[VERIFIED: /Users/neilskinner/code/r9/x86_64/src/proc.rs:2359-2362]`:

```rust
    if p.state == ProcState::Blocked {
        p.sleep_until = 0;
        p.state = ProcState::Ready;
    }
```

The aarch64 `push_note` (`aarch64/src/proc.rs:2000-2011`) ends at `p.note_queue.push_back(note); Ok(())` with no ready arm `[VERIFIED: grep -A22 display; re-read before editing]`. Port both halves together.

### Gate results with the probe kernel

`[VERIFIED: nix run .#<app> --no-write-lock-file --override-input r9 git+file://<scratch clone>]` — qemu 11.1.2, accelerators `hvf tcg`.

| Gate | Shape | Runs | Pass | Notes |
|------|-------|------|------|-------|
| `boot-r9-smoke` | gicv3, hvf, cpu host | 6 | 6 | First two included the full userland build; consecutive green |
| `boot-r9-smoke` | gicv3, tcg, cortex-a72 | 1 | 1 | |
| `boot-r9-smoke` | gicv2, tcg (`R9_GICV2=1`) | 1 | 1 | The v1.0 reference shape |
| `boot-r9-net-smoke` | gicv3, hvf | 3 | 1 | Two failures: `rc: pushword but no argv!` in pid 5 (the `stacksmoke` child of termrc), then the driver waits for a marker that never comes |
| `boot-r9-net-smoke` | gicv3, tcg | 1 | 1 | |
| `boot-r9-tier1` | gicv3, hvf | 3 | 1 | Two failures: pushword fire ate `T1-OK sort` (run 1) and `T1-SKIP ipconfig` (run 3) |
| `boot-r9-tier1` | gicv3, tcg | 1 | 1 | 36 T1-OK + 23 T1-SKIP + T1-DONE, matches the computed expectation |

HVF total: 4 pushword fires in 13 boots. TCG total: 0 in 4. The smoke gate has roughly ten forks per boot and the tier-1 sweep roughly 130, which is consistent with a per-fork probability rather than a gate-specific fault `[ASSUMED]`.

Other checks run:

| Check | Result |
|-------|--------|
| `cargo test -p port --no-fail-fast` (nightly-2026-01-30) | 164 lib tests pass; 11 integration binaries pass; **1 failure**: `devenv_test::devenv_device_flow_over_dynamic_groups` at `port/tests/devenv_test.rs:267`, `left: 8 right: 10` `[VERIFIED: cargo test]` |
| `cargo xtask ureg-check` (`R9_HARVEY_DIR` = this checkout) | pass `[VERIFIED]` |
| `cargo xtask check`, `cargo xtask clippy` | exit 0, but neither sets any `dev_*` cfg, so neither compiles the `dev_exec` code `[VERIFIED: ran; log shows default-cfg check only]` |
| `cargo xtask boot-test --arch aarch64 --config virt` / `virtlowecam` / `virtpanic` at unpatched HEAD in the real r9 checkout | all exit 0 (12 s, 3 s, 4 s; TCG) `[VERIFIED]` |
| `cargo xtask boot-test` for the `dev_exec` profiles (`virtexec`, `virtexec3`, `virtmulti`, `virtnotes`, `virtnet`, `virtip`, `virtshell`) | **Not observed.** They need the patched source, and my scratch clone's path is 138 bytes to the serial socket, over macOS's 104-byte `sun_path` limit, so the harness never connects. Run them from `/Users/neilskinner/code/r9` during execution |
| `build/aarch64/subset-exec.sh` (donor venue) | First run FAIL (`7c: No such file or directory: u.h`); PASS after `donor-vm-build.sh` (45 s each) `[VERIFIED]` |
| `harness trace-syscalls --boot-chain` (donor venue, `TRACE_OUT` set to scratch) | FAIL at the final assert, `measured set lacks fversion`; the chain itself ran to `__GSD_ALL_DONE__` `[VERIFIED]` — see Pitfall 6 |

Evidence files are in the session scratchpad and will not survive a reboot. The directory is `/private/tmp/claude-501/-Users-neilskinner-code-harvey/9e555d3f-93ee-4b26-bbdd-68d7243514da/scratchpad/`; the files are `p20-smoke-run-1.log`, `p20-smoke-serial-1.log`, `p20-series-results.txt`, `p20-series-*.run.log`, `p20-port-test-nff.log`, `p20-subset-exec-3.log`, `p20-trace-bootchain.log`, and the scratch clone `r9-probe/`. The executor must regenerate evidence, not cite these.

## Drift Inventory: x86_64 vs aarch64 against `port`

Method: `git log dbea82f..HEAD -- port x86_64` as the authority, then for each commit the files it touched per crate, then a diff of what each arch crate actually calls. Classes: **(a)** arch-neutral, arm64 has it once it compiles; **(b)** small aarch64 hook or wiring; **(c)** real aarch64 kernel work; **(d)** x86-only.

Three mechanical comparisons bound the (b) set `[VERIFIED: grep/comm over both crates]`:

- **DISPATCH table.** Both are `[Option<Handler>; 55]`. x86_64 has exactly five slots aarch64 lacks `[VERIFIED: x86_64/src/syscall.rs:139-143 vs aarch64/src/syscall.rs:143-145]`:
  ```rust
      t[REMOVE as usize] = Some(sys_remove);
      t[WSTAT_ as usize] = Some(sys_wstat);
      t[FWSTAT_ as usize] = Some(sys_fwstat);
      t[WSTAT as usize] = Some(sys_wstat);
      t[FWSTAT as usize] = Some(sys_fwstat);
  ```
- **Hook registrations in `main.rs`.** x86_64 registers three that aarch64 does not: `port::devcons::set_user_hooks`, `port::devcap::set_tick_hook`, `port::devssl::set_fd_resolve_hook`. Everything else matches (`set_post_hook`, `set_proc_hooks`, `set_pid_hook`, devpipe/devether/devip `set_hooks`, `set_clock`, `set_contention_hook`, devdup `set_hooks`, devsrv `set_resolve_hook`, `set_cons_hooks`, `set_random_hooks`, `set_group_hook`).
- **`port` APIs called by x86_64 only:** `devmnt::write`, `devcap::{attach,read,write,remove,walk,set_tick_hook}`, `devssl::{read,write,set_fd_resolve_hook}`, `devcons::{set_user_hooks,UserHooks}`, `namec::{walk_one_union,create_chan_union}`, `virtio::{Caps,CapTriple}`, `allocator::{heap_census,heap_stats}`, `devether::driver_ops`.

### Class (a): free once it compiles

All in `port/` only. arm64 gets these with no arch edit `[VERIFIED: per-commit file lists show no x86_64/src change, or only a call-site already present on aarch64]`.

| r9 commit | What |
|-----------|------|
| `091614a`, `1cbc1db`, `501a7c5`, `d5445dc` | devmnt tag-mux, mountmux drop arm, zero-count Twrite, Ropen iounit |
| `3c67e2a` | Session-teardown Drop (`Mnt::drop` closes its connection chans) |
| `56acaea` | devpipe blocking write on a full queue |
| `22d40fb`, `270fdbf`, `b1e7e3e` | devssl device, direction split, rc4_256 (the device; the syscall arms are class b) |
| `ab4cb66`, `f610473`, `9704b2e` | TCP two-write (Nagle off, no delayed ACK), AcceptPool sockets, listen-park ingress drive |
| `2bc52a7`, `abe72e7`, `d4a5d38` | WaitSet 512, used-ring liveness, NET_WAITERS deregister |
| `652f025` | `#c/hostowner` eve gate (but see the user-hooks regression: without hooks it now errors) |
| `242f228` | 32 MiB heap. Seen live: the arm64 boot log maps `Kernel Data 0x40600000..0x42800000`, 34 MiB `[VERIFIED: smoke capture]` |
| `fe67c75` | Identity-keyed mount consults — the aarch64 half landed in the same commit |
| `de1fba1` | devproc private/noswap — aarch64 half landed in the same commit |
| `df9ada8` | `tick_check` under `dev_ip` — aarch64 was the precedent x86_64 copied |

### Class (b): small aarch64 wiring

Sizes are the x86_64-side line counts from `git show --numstat`, which include inline tests where noted; treat them as upper bounds for the aarch64 port.

| # | Item | r9 commit | x86_64 side | Observable on an arm64 gate today? |
|---|------|-----------|-------------|------------------------------------|
| b1 | `note_pending` fields + fn | `1f14a50` | 2 fields + 7 lines | Yes — the compile error |
| b2 | `push_note` readies a Blocked target | `1f14a50` | 4 lines | No gate asserts it; `virtnotes` profile is the nearest |
| b3 | Per-proc user + `set_user_hooks` (`current_user`, `set_current_user`, fork copy) | `0358cbe` | main.rs +8, proc.rs +61 | **Yes — two error lines in every capture** |
| b4 | Non-RFPROC `rfork(RFFDG/RFCFDG)` in-place fd-group split | `e045dde` | +19 −5 | No. aarch64 still rejects it: `if flag & (RFMEM \| RFNOWAIT \| RFFDG \| RFCFDG) != 0` at `aarch64/src/syscall.rs:1628` `[VERIFIED: grep]`. lib9p `postmountsrv` needs it |
| b5 | `DEV_MNT => port::devmnt::write(...)` | `06f8aa1` | 1 line | No. aarch64 returns `"devmnt: the mounted tree is read-only this era"` (`aarch64/src/syscall.rs:323-327`) |
| b6 | DMDIR creates through a mount | `21d2350` | +5 −6 | No. aarch64 returns `"create: directories unsupported this era"` (`:1416`) |
| b7 | Union consult in `sys_create` + `#`-rooted nomount bit | `cfe7af0`, `421d5e1` | +20 −7 | No (WINDOWS #45) |
| b8 | Exit-path drains: transients dropped before the diverging tail, husk payload sweep, slash/dot plain drop | `c4f2f2c`, `72edbba` | +149 −19 | No. This is WINDOWS #44, about 16 KiB leaked per exec before the fix `[CITED: STATE.md 17-02]`; arm64 still has it `[ASSUMED: aarch64 has no sweep_husk_payloads]` |
| b9 | Lone-member RFMEM brk arm maps growth | `c4f2f2c` | part of the above | No |
| b10 | fd-drain cascade + `take_mount_chans` close-dispatch at pgrp death | `ab0a977` | +28 −3 | No |
| b11 | Close-on-exec sweep (`close_on_exec_fds`, `take_cexec`) | `99714a6` | exec.rs +75 | No |
| b12 | `#!` exec pre-pass | `d188171` (pins `0a39318`) | exec.rs +109 −28, tests +378 | No |
| b13 | `REMOVE`/`WSTAT`/`FWSTAT` handlers + statcheck | `20c704c` (pins `69a1a5b`) | +49 −35, tests +406 | No |
| b14 | devcap arms + `dev_cap` cfg + tick hook | `82b5cdb` | main +5, syscall +32 | No. Workspace `check-cfg` already declares `cfg(dev_cap)` |
| b15 | devssl read/write arms + fd-resolve hook; `fversion`/`mount` accept `DEV_SSL` chans | `07889e2`, `917685c` | main +7, syscall +9 | No. `#D` attaches unconditionally from `port` (`port/src/namec.rs:107`), then aarch64 read/write falls to `"pread: unknown device"` |
| b16 | virtio cap-walk hardening via shared `virtio::Caps::parse` in the arch PCI bind | `cc22223` | `platform/q35/pci.rs` | No. aarch64 `platform/virt/pci.rs` does not call `virtio::Caps` `[VERIFIED: API-use diff]` |

### Class (c): real aarch64 kernel work

| # | Item | Why it is not small |
|---|------|---------------------|
| c1 | Bootconf delivery (`service=cpu`, `init=`) via DTB `/chosen/bootargs` into the ROOT env seed | x86_64 did this with 308 lines in `mb2.rs` plus the grub append seam (`8b14ecb`). aarch64 seeds three fixed pairs only (`aarch64/src/main.rs:582-587`) and has no `bootargs` reader `[VERIFIED: grep -c bootargs aarch64/src = 0]`. Needs a parser, the Harvey-side `fdtput` seam in `nix/boot-r9.sh`, and a gate |
| c2 | The HVF fork-child defect (WINDOWS #49) | Unknown root cause; see Pitfall 1. Could be one line or a week |
| c3 | An aarch64 host-test battery | The x86_64 crate carries its pins inline (`cfg(test)` in `syscall.rs`, `exec.rs`, `proc.rs`); aarch64 has none `[VERIFIED: grep cfg(test) aarch64/src = 0]`. `cargo xtask test` only tests the host-arch crate when rustup reports a std target. Every (b) port lands unpinned unless a boot-test profile or composed gate covers it |
| c4 | A churn leak gate for arm64 | The x86_64 gate is `platform/q35/churngate.rs` plus `config_q35churn*.toml`; b8 has no arm64 pin without an analogue |

### Class (d): x86-only

Everything under `x86_64/src/platform/q35/` (mb2, ACPI, LAPIC, IOAPIC, PCI/ECAM discovery, netgate, churngate), `entropy.rs` (aarch64 seeds `/dev/random` from the DTB `rng-seed` plus timer jitter via `RandomHooks`, already registered), user SSE and the eager stack (`e417660`), the 64 MiB module stash (`236d5a7`; aarch64 maps the initrd from DTB `linux,initrd-start/end`), the x86-local OOM-release paths (`3e8bcec`), the 14-06 scheduler fixes (`8aa9ba2`, `c166164`), the grub ISO builder and its FOD.

## Scope Recommendation

The stated bar is `boot-r9-smoke` green on arm64 at the pin. Measured, that bar costs 9 lines plus a pin advance. "Parity" needs a cut-line; this is the one I recommend, with the reasoning for each tier.

### In scope (must)

1. **b1 + b2 together** — the compile fix with the complete `1f14a50` semantics.
2. **b3** — the user-hooks regression. It is a v2.0-to-now behavior loss on the arm64 boot path (`#c/user` unreadable, `#c/hostowner` write refused), visible in every capture, and x86_64's own comment says the hook is "NOT dev-gated — every dev_exec platform serves #c/user" `[VERIFIED: sed display of x86_64/src/main.rs:453-460]`. Add a post-pass to `build/aarch64/smoke-r9.expect` that fails on `hooks not registered` so it cannot regress silently.
3. **`port/tests/devenv_test.rs:267`** — `assert_eq!(n, 10)` becomes `8` `[VERIFIED: Read port/tests/devenv_test.rs:266-267; "arm64-r9" is 8 bytes; test output left: 8]`.
4. **Guards:** an r9 CI lane that builds `--arch aarch64 --config virtshell`; `cargo xtask test` (or CI) running the `port` integration tests, not only `--lib`; the r9 devShell made usable on darwin.
5. **Donor venue:** re-provision, rerun `subset-exec.sh`, rerun the boot-chain trace, and fix or record the trace assert.
6. **Push-before-pin**, lock-only pin advance as its own commit, the four arm64 gates green at the pin, records corrected (STATE's stale LOCAL ONLY note, WINDOWS #49 reopened).

### In scope if the owner wants kernel parity now (recommended: yes for b4–b11, as one plan)

b4 through b11 are donor-correctness fixes to paths the arm64 boot chain and tier-1 sweep already execute: rfork, write and create through mounts, exit, exec. They are mechanical because x86_64's `syscall.rs` and `proc.rs` were ported from aarch64's. The risk is that none has an arm64 pin; the regression pin is the existing four gates staying green, and b8 has no positive pin without c4. If the owner prefers the project's "every fix lands with a pin" rule strictly, defer b8/b9 with c4.

### Defer to the arm64 cpu-port phase

b12–b16, c1, c3, c4, and every Harvey-side item in "Harvey-side drift" below. These are new capability with no arm64 consumer. STATE.md already records the lesson that applies: "Never-exercised assertion code is unverified code" `[CITED: STATE.md 19-08 pass 3]`. Porting `#!`, WSTAT, devcap and devssl arms now means carrying roughly 800 lines of unexercised kernel code until tier-2, the §15 image and the drawterm gate exist on arm64 to exercise them. STATE.md's Deferred Items names this port as "phase-scale" and "owner-performed" `[CITED: STATE.md Deferred Items, 2026-11-17]`.

### Wants its own plan or phase

c2. It predates this phase, it is not caused by the drift, and its root cause is unknown. It does decide whether "green at the pin on arm64" is reliable on the primary darwin host.

### Capability-by-capability view of phases 13–19

| v2.1 phase | arm64 analogue | Cost on arm64 |
|------------|----------------|---------------|
| 13 platform bring-up | Done in v1.0/v2.0 (virt, GIC, PCI/ECAM, virtio-net) | None |
| 14 userspace on r9, tiers 0–1, `/net` gates | Done in v2.0; green again with the fix | The must-scope above |
| 15 tiers 2–4, bootfs payload | Nothing exists: no `tier2-services.txt`, no arm64 arm past `T1KEEP` in the mkfiles, no services image | Its own phase |
| 16 host net modes, tap soak | arm64 has slirp only; `apply_virt_netdev` frozen | Its own phase; darwin has no tap |
| 17 flush-out | Class (a) mostly free; b4–b11 are the arch halves | One plan (optional tier above) |
| 18 auth domain | Needs tier-2 auth suite, mkimage §14, b3, b14 | Part of the cpu-port phase |
| 19 cpu/drawterm/rio | Needs c1, b12–b15, mkimage §15, a gate | Part of the cpu-port phase |

## Existing arm64 Gates

### Harvey nix apps

Defined at `flake.nix:469-474`, gated on `builtins.hasAttr system r9.packages` `[VERIFIED: Read flake.nix]`. All four consume only `r9.packages.SYSTEM.kernel-virt` (`kernel-virt.elf` plus `aarch64/lib/virt-gicv{2,3}.dtb`).

| App | Script | Driver | Asserts |
|-----|--------|--------|---------|
| `boot-r9` | `nix/boot-r9.sh` | none (interactive) | — |
| `boot-r9-smoke` | `nix/smoke-r9.sh` | `build/aarch64/smoke-r9.expect` | banner `r9 from the Internet`, `paqfs...`, `stacksmoke: depth 64 ok`, `% `, typed echo `r9shell7` twice, `ls /bin` subset, `ps`, `cat /lib/namespace`, DEL safety (`r9delok`), `init: starting /bin/rc` restart, no `#p/0/ctl`, `#l0: virtio: 1000Mbps:` bind line; panic fast-fail |
| `boot-r9-net-smoke` | `nix/smoke-r9-net.sh` | `build/aarch64/smoke-r9-net.expect` | boot stages, then `netip:` fixture markers for static, dhcp, loopback ping, udp echo, tcp/7 echo, both `NOT-L2-TRUTH slirp venue` labels, completion marker |
| `boot-r9-tier1` | `nix/tier1-r9.sh` | `build/aarch64/tier1-r9.expect` | boot stages, one typed line `rc /lib/tier1-sweep.rc r9`, then `T1-OK float-d01`, `T1-OK sort`, `T1-OK sum`, `T1-SKIP tar`, `T1-DONE` in that order `[VERIFIED: build/aarch64/tier1-r9.expect:136-153]`, then a marker-parity post-pass against the computed set |

All three expect drivers use `set timeout 600` per stage. A pushword fire that eats a marker costs up to ten minutes before the gate fails.

Shape selection (`nix/smoke-r9.sh` step 0b, same in the others): gicv3 everywhere; `R9_ACCEL=auto` picks hvf on darwin-arm64 when qemu offers it, else tcg; `R9_GICV2=1` forces gicv2 + cortex-a72 + tcg. `R9_SKIP_BUILD=1` skips the userland build (iteration only).

### Green at the v2.0 pin, expected now

| Gate | At v2.0 pin | With the fix |
|------|-------------|--------------|
| `boot-r9-smoke` | Green (05-12, INT-01..04) `[CITED: PROJECT.md:40]` | Green, RAN |
| `boot-r9-net-smoke` | Green (08-09) `[CITED: flake.nix:129-140]` | Green on TCG, RAN; flaky on HVF |
| `boot-r9-tier1` | Green (06-07) `[CITED: flake.nix:146-150]` | Green on TCG, RAN; flaky on HVF |
| r9 `virt`, `virtlowecam`, `virtpanic` | Green | Green at unpatched HEAD, RAN |
| r9 `virtexec`, `virtexec3`, `virtmulti`, `virtnotes`, `virtnet`, `virtip`, `virtshell` | Green `[CITED: PROJECT.md:36-39]` | Not observed (see Measured Start State). `virtnotes` is the one most likely to move, because b2 changes note delivery `[ASSUMED]` |

### Assumptions the rename or v2.1 changed

- The marker string `hello aarch64 harvey` is deliberately kept everywhere it appears (`build/aarch64/hello.c:7`, `tracehello.c:26`, `run-gate.sh`, `tier1-sweep.rc:582`, `build/harness/src/{srv9,drive9,trace}.rs`, r9 `xtask/src/main.rs:1010` `BOOT_HELLO_MARKER`) `[VERIFIED: grep]`. It is consistent; do not rename one side.
- `git grep` for `objtype=aarch64`, `/aarch64/{bin,init,lib,include}`, `cputype=aarch64` outside `.planning` and ledgers returns only r9 package paths (`$R9_KERNEL_PKG/aarch64/lib/...`), which are correct `[VERIFIED: git grep]`.
- The tier-1 r9 SKIP set (23 entries) encodes "r9's root is read-only this era" (`build/aarch64/tier1-sweep.rc:356-358`). b5/b6/b7 would make some of those runnable, but no writable venue is staged in the arm64 image, and the amd64 tier-1 list does not carry the write-flow entries at all. Leave the SKIP set alone this phase.

### CI

| Workflow | Job | Runs | State |
|----------|-----|------|-------|
| Harvey `r9-shell.yml` | `nix run .#boot-r9-smoke`, `nix run .#boot-r9-net-smoke` on `ubuntu-latest` (TCG) | every push/PR | Cannot have passed at any pin from `aa55a27` on `[ASSUMED: same compile error on x86_64-linux; hosted results not observable — no `gh` on this host]` |
| Harvey `phase6.yml` | `nix run .#boot-r9-tier1`; `generator-scan.sh`; coverage drift | every push/PR | Same |
| r9 `xtask.yml` | `cargo xtask check`, `clippy`, `test` (`port --lib` only), `build --arch aarch64` (default = raspi3b, no `dev_exec`), x86_64 boot-tests | push/PR to `main` | Green while aarch64 `dev_exec` was broken `[VERIFIED: Read .github/workflows/xtask.yml:38-44; aarch64/lib/config_default.toml has platform = "raspi3b" and no dev list]` |

## Harvey-side arm64 Drift

**Nothing that built before fails now.** `build/aarch64/subset-build.sh` printed `TIER-BUILD-OK` with 74 entries in `arm64/bin`, compiling the current tree (including the 17-06 rc fix and the 15-09 libmemdraw audit) with `objtype=arm64` `[VERIFIED: gate run log]`. `mkimage.sh` baked a 4,198,438-byte `bootfs.paq`. The full `COVERAGE_SWEEP=1` proof (`build/aarch64/coverage-sweep.sh`) was not run.

What phases 13–19 added only on the amd64 side `[VERIFIED: ls, wc, diff]`:

| Area | arm64 (`build/aarch64/`) | amd64 (`build/x86_64/`) |
|------|--------------------------|-------------------------|
| Tier lists | `tier0-libs.txt` (35 libs), `tier1-cmds.txt` (62 entries) | tier0 (27), tier1 (40), `tier2-services.txt`, `tier3-gui.txt`, `tier4-tail.txt` |
| mkfile filter arms | `sys/src/{mkfile,cmd/mkfile,cmd/ip/mkfile,cmd/auth/mkfile,cmd/auth/factotum/mkfile}` filter arm64 through `T1KEEP` only; `cmd/aux/mkfile` has no arm64 arm | T1–T4 keeps |
| `mkimage.sh` | 291 lines, one image | 2091 lines, bake contracts through §15 (chain, tier, net, services, churn, substrate, auth, cpu) |
| `root-tree` | `lib/namespace`, netip fixtures, `rc/bin/termrc`, profile | adds `bin/service/tcp17010`, `rc/bin/cpurc`, `lib/ndb/local.template`, `lib/netaddr.rc`, churn/stress/tap fixtures |
| Gates | 3 expect drivers | 11 apps: services, churn, auth, cpu bring-up, drawterm gate, soak, stress |
| Auth | none | `authdomain-gen.c`, donors, selftest |

arm64 tier-0 is a superset of amd64's in library count because the 06-05 wave built the whole library set; that work carries over to a later tier-2/3 phase.

## Venues and How to Run Things on This Host

Host: darwin 27.0.0, arm64 (Apple silicon), Determinate Nix 2.34.6. Nothing else is on PATH: no `cargo`, `rustup`, `qemu-*`, `limactl`, or `gh` `[VERIFIED: command -v]`. Everything comes through nix.

| Need | How | Notes |
|------|-----|-------|
| r9's pinned toolchain | `nix develop /Users/neilskinner/code/r9#kernel-virt -c <cmd>` | `rustc 1.95.0-nightly (842bd5be2 2026-01-29)`. Export `OBJCOPY` to the toolchain's `lib/rustlib/*/bin/llvm-objcopy` first; the default devShell does this in its hook |
| r9's default devShell | **Fails on darwin**: `Refusing to evaluate package 'grub-2.14' ... not available on the requested hostPlatform` `[VERIFIED: nix develop]` | Cause `[VERIFIED: /Users/neilskinner/code/r9/flake.nix:836]`: `packages = [ toolchain pkgs.qemu pkgs.dtc pkgs.grub2 pkgs.xorriso ];` — unconditional since 13-01 |
| qemu + dtc beside the toolchain | `nix develop .#kernel-virt -c nix shell --inputs-from . nixpkgs#qemu nixpkgs#dtc -c <cmd>` | qemu 11.1.2 |
| r9 boot-tests | `R9_HARVEY_DIR=/Users/neilskinner/code/harvey cargo xtask boot-test --arch aarch64 --config <name>` | TCG per the TOMLs. Run from the real checkout: the serial socket path must stay under 104 bytes |
| Harvey devshell | `nix develop` in Harvey | plan9port, qemu, expect 5.45.4, lima 2.2.0, stable cargo 1.99.0 (harness only — it cannot build r9) |
| lima guest | VM `harvey`, vz, aarch64, 4 CPU, 8 GiB, currently Running `[VERIFIED: limactl list]` | Entered by the scripts over ssh; `nix develop -c limactl shell harvey -- <cmd>` for ad hoc use |
| Namespace build | `build/aarch64/vm-run.sh '<cmd>'` rsyncs the checkout to guest `~/harvey-build` and runs under `nsrun.sh` | `subset-build.sh` is the gate's driver |
| Composed gate against a local r9 | `nix run .#boot-r9-smoke --no-write-lock-file --override-input r9 git+file:///Users/neilskinner/code/r9` | Dirty tracked files are included; new files must be `git add`ed first `[CITED: STATE.md 16-01]`. Run under `bash`: zsh does not word-split an unquoted `$O` variable holding the flags |
| Donor venue | `nix develop -c sh build/aarch64/donor-vm-build.sh`, then `subset-exec.sh`, then `TRACE_OUT=<path> harness trace-syscalls --boot-chain` | Uses guest `~/harvey-build-donor`, a separate checkout |

Timings observed, warm guest and warm nix store: smoke with full tier build about 60 s; with `R9_SKIP_BUILD=1` 8–9 s under HVF or TCG; net smoke 16–38 s; tier-1 under TCG 28 s including the build; donor provision 45 s; donor tier-1 gate 45 s; boot-chain trace 91 s; incremental kernel rebuild about 2 s. A cold kernel build was not timed.

**Cannot be run on this host:**

- `nix build r9#boot-iso` and any FOD re-wire or two-derivation proof — the attr exists only on `x86_64-linux`.
- All eleven `*-x64` Harvey apps — gated on `kernel-x64` + `iso-builder` + `netdev-args`, linux only; KVM absent.
- The x86_64 crate's host test battery — `cargo xtask test` picks the host arch.
- Hosted CI observation — no `gh`; needs the owner or a browser.
- Tap/bridge networking.

## Pin-Advance Protocol

The shape used in phases 14–19 `[CITED: .planning/phases/19-cpu-service-drawterm-rio-desktop/19-05-SUMMARY.md, 19-PATTERNS.md:428-430]`:

1. Land the r9 commits on `main`; run the local battery.
2. If kernel bytes of the **x86_64** kernel changed: on linux-x86_64, build two independent kernels (plain, plus a marker commit in a discarded clone), realize the non-FOD `lib.bootIso` for both, confirm byte-identical ISOs, read the FOD's own got-value, re-wire `outputHash` in a commit that touches `flake.nix` only.
3. Push r9. Observe hosted CI green at the pushed HEAD.
4. In Harvey, advance the pin as its own lock-only commit: `chore(NN-PP): advance the r9 pin <old> -> <new> lock-only (<why>)`, touching `flake.lock` only (6 lines changed in `4a4f463e1`).
5. Re-green the composed gates at the pin, without `--override-input`.

**arm64 has no FOD.** `kernel-virt` is an ordinary derivation; the single `outputHash` in r9's `flake.nix` belongs to `boot-iso`, which stages `kernel-x64` and exists only on `x86_64-linux` `[VERIFIED: Read flake.nix; one outputHash]`. Harvey's arm64 apps never touch it.

What this means for planning:

- Changes confined to `aarch64/**`, `port/tests/**`, `xtask/**`, `.github/**`, docs, and the `devShells` block of `flake.nix` should not change the x86_64 kernel ELF, so step 2 should be vacuous `[ASSUMED: the two-derivation proofs show the rev string does not reach the kernel bytes; not verifiable here]`. The pin for `8e4e730` was advanced lock-only with no re-wire on exactly this reasoning.
- **Any edit under `port/src/` changes x86_64 kernel bytes** (code, or just panic-location line numbers) and re-stales the FOD. That forces a linux-x86_64 pass this host cannot do. Keep Phase 20's kernel edits out of `port/src/`. Everything in the must and optional tiers above fits in `aarch64/`.
- Plan a checkpoint for the owner to confirm `nix build .#boot-iso` still passes on the linux host after the r9 push, since it cannot be checked from darwin.
- Push Harvey `master` before adding any r9 CI lane that checks out `requaos/harvey` for the aarch64 `ureg-check`: r9 reads `arm64/include/ureg.h` (`xtask/src/uregcheck.rs:151`) and `origin/master` still has `aarch64/`.

## Standard Stack

No new libraries. The phase uses what both repos already pin.

| Tool | Version | Source |
|------|---------|--------|
| Rust | `nightly-2026-01-30`, targets `aarch64-unknown-none`, `x86_64-unknown-none`, `riscv64gc-unknown-none-elf` | `[VERIFIED: /Users/neilskinner/code/r9/rust-toolchain.toml]` |
| qemu | 11.1.2 (`hvf`, `tcg`) | `[VERIFIED: qemu-system-aarch64 --version]` |
| expect | 5.45.4 | `[VERIFIED]` |
| lima | 2.2.0 | `[VERIFIED]` |
| nix | Determinate 2.34.6 | `[VERIFIED]` |

## Package Legitimacy Audit

Not applicable: the phase installs no external packages. Any plan that proposes a new crate or nixpkgs input must run the legitimacy gate first.

## Architecture Patterns

### Data flow of the composed gate

```
r9 checkout ──push──▶ github:requaos/r9 ──flake.lock rev──▶ r9.packages.SYSTEM.kernel-virt
                                                              │  kernel-virt.elf + virt-gicv{2,3}.dtb
Harvey checkout                                               ▼
  │ subset-build.sh ──rsync──▶ lima guest (unshare -m, objtype=arm64)
  │        ◀── arm64/bin, arm64/lib, build/aarch64/out ──┘
  │ mkimage.sh ──▶ build/aarch64/bootfs.paq
  │ fdtput ──▶ patched DTB (/chosen linux,initrd-start/end)
  ▼
qemu-system-aarch64 -M virt,gic-version=3 [-accel hvf|tcg] -kernel … -dtb … (initrd at 0x48000000)
  ▼ serial
expect driver: staged markers ─▶ post-passes ─▶ exit 0 / 1
```

### Pattern: mirror the x86_64 arch side

x86_64's `syscall.rs`, `proc.rs` and `exec.rs` were ported from aarch64's at 14-04 with the frame type and argument convention changed (`TrapFrame`/`x0` versus `Ureg`/`bp`, args at `sp + 8`). For each class (b) item, `git show <commit> -- x86_64/src` is the template and the aarch64 function of the same name is the target. Function-name sets differ only by the items in the inventory `[VERIFIED: comm over fn names]`.

### Pattern: hook registration

Registered once in `exec_bootstrap` before the first exec; a second registration asserts. b3's target shape `[VERIFIED: sed display of x86_64/src/main.rs:457-460; re-read before editing]`:

```rust
    port::devcons::set_user_hooks(port::devcons::UserHooks {
        current: proc::current_user,
        set: proc::set_current_user,
    });
```

`UserHooks` has exactly `current: fn() -> alloc::vec::Vec<u8>` and `set: fn(alloc::vec::Vec<u8>)` `[VERIFIED: sed display of port/src/devcons.rs:244-247]`.

### Anti-patterns

- **Editing `port/src/` to make aarch64 easier.** It re-stales the x86_64 FOD and needs a venue this host lacks.
- **Shipping the probe patch as the fix.** It omits `push_note`'s ready arm.
- **Declaring the gates green from HVF runs alone, or from TCG runs alone.** Record both; they disagree today.
- **Bundling the pin advance with anything else.** It is its own lock-only commit.

## Don't Hand-Roll

| Problem | Don't build | Use instead |
|---------|-------------|-------------|
| Note-interrupt semantics | A new design | `1f14a50`'s x86_64 half, verbatim |
| Per-proc user storage | A global | `0358cbe`'s `Proc.user` + fork copy, as x86_64 does |
| Toolchain on darwin | rustup | r9's flake (`#kernel-virt` shell today; the fixed default devShell after this phase) |
| Running a gate against local r9 | Editing `flake.lock` | `--override-input r9 git+file://…` with `--no-write-lock-file` |
| arm64 build of an xtask-owned image | A new script | `build/aarch64/subset-build.sh` + `mkimage.sh` |

## Runtime State Inventory

This phase is the first live proof of a rename, so the inventory applies.

| Category | Items found | Action |
|----------|-------------|--------|
| Stored data | Append-only ledgers `build/aarch64/markers/tier{0,1}.ledger` and `gate-evidence.log` carry `aarch64/...` artifact paths; nothing resolves old rows against the tree `[CITED: 261117-bz5 SUMMARY]` | None; new rows will say `arm64/` |
| Live service config | Hosted CI on both repos has not run the renamed Harvey tree (master is 6 ahead of origin) | Push Harvey before relying on hosted results |
| OS-registered state | lima VM `harvey` — no name dependency on the objtype | None |
| Secrets/env vars | None — verified by grep for `cputype=aarch64`/`objtype=aarch64` outside `.planning` | None |
| Build artifacts | (1) Guest `~/harvey-build-donor` held a pre-rename Harvey tree and a 2026-10-31 `7c` whose strings contained `aarch64` and no `arm64`; `subset-exec.sh` does not re-push that checkout. Re-provisioned this session by `donor-vm-build.sh`. (2) `.build/9front-dist/srvroot/aarch64/` stale staging still on disk. (3) `build/aarch64/smoke-out/` binaries dated Nov 2, pre-rename, consumed by r9's `virtexec`-family initrd | (1) Make `donor-vm-build.sh` a documented prerequisite of the donor gates; (2) harmless, optionally delete; (3) rerun `sh build/aarch64/stage-build.sh` before the r9 exec profiles |

## Common Pitfalls

### Pitfall 1: WINDOWS #49 is live on HVF, and it is probably not an rc bug

**What goes wrong:** `rc: pushword but no argv!` then `aborting`, the child exits with a garbage status (`"<bad utf8>"`, `"X&"`), a marker never prints, and the expect driver sits for up to 600 s before failing.
**Observed:** 4 fires in 13 HVF boots, 0 in 4 TCG boots, with the rc that already carries the 17-06 guards. Fire sites: the `stacksmoke` child of termrc (pid 5, twice, identical line), a backquote child (`sortout=`), and a plain `echo T1-SKIP ipconfig` child.
**Why the rc attribution looks wrong:** the child's first action after `fork()` is `pushword("exec")` `[VERIFIED: /Users/neilskinner/code/harvey/sys/src/cmd/rc/havefork.c:225-230]`, and `pushword` panics only when `runq->argv` is nil (`sys/src/cmd/rc/exec.c:43-46`). The parent had a non-empty argv (it was running a simple command). For the child to see nil there, the child's memory image differs from the parent's at fork. That points at the aarch64 rfork copy path under real-CPU semantics, not at rc's op sequencing `[ASSUMED — this is a lead, not a diagnosis]`. Places to look first: `rfork_copy` in `aarch64/src/proc.rs:2275` onward, `map_kernel_alias` re-mapping the KZERO alias before the copy (break-before-make and TLB invalidation), and whether a parent page can be absent from `mapping_log` at copy time.
**How to avoid being bitten:** make TCG the truth venue for the phase gate, as WINDOWS #49's own record does ("the CI/truth venue is TCG"), and record HVF results beside it. Reopen #49 in WINDOWS.md with this session's fire count.
**Warning signs:** a gate that hangs right after `paqfs...` or mid-sweep; `<bad utf8>` in a `process N exited:` line.

### Pitfall 2: Compiles is not parity

The user-hooks regression (b3) passed every gate. Any (a)-class `port` change that added a hook struct, or made an existing path depend on one, can do the same. After the fix, grep each gate capture for `not registered`, `unknown device`, and `unsupported this era`.

### Pitfall 3: `port/src/` edits re-stale a FOD that cannot be checked here

See the pin protocol. If a fix seems to need a `port/src/` change, stop and plan a linux-x86_64 FOD pass.

### Pitfall 4: The r9 devShell and the macOS socket limit

`nix develop` in r9 fails on darwin (grub2). And `cargo xtask boot-test` connects to `target/boot-test-<config>.sock` by absolute path; a checkout under a long path fails silently by retrying until its deadline. `/Users/neilskinner/code/r9` is short enough.

### Pitfall 5: The donor venue must be re-provisioned after the rename

`subset-exec.sh` stages binaries into the guest donor tree but does not refresh guest `~/harvey-build-donor`. Its `7c` wrapper (`~/9front-hostbin/7c`) execs that checkout's hostbin `7c`, which was built with the old name and searched `/aarch64/include`; the alias is gone. `donor-vm-build.sh` re-pushes the checkout, rebuilds `7a/7c/7l` unconditionally, and sweeps stale donor objects. After it, `subset-exec.sh` passed: "the tier-1 set executed on the 9front donor kernel (T1-DONE; float canary 0.333 green)" with 56 members + float-d01 + libtest ×2 + selfhost ×2, which also proves the merged `/arm64` staging (Harvey `include` + `lib/libc.a` beside donor `bin` + `init`).

### Pitfall 6: The boot-chain trace gate

Run with `TRACE_OUT` set. The defaults at `build/harness/src/trace.rs:511-515` point into `.planning/phases/04-…`, `05-…` and `06-…`, directories that were archived to `.planning/milestones/` `[VERIFIED: Read; ls .planning/phases]`; a default run would write a stray file under `.planning/phases/`.

One run this session: the chain executed (`ourboot Exec "/arm64/init" "init" "-t"`, then init, rc, termrc, the sweep, `__GSD_CHAIN_DONE__`, `__GSD_ALL_DONE__`), which confirms the `bind /boot/ourinit /arm64/init` deviation from quick 261117-bz5 works live. The gate then failed its last assert, `measured set lacks fversion`. The raw log contains exactly one `Fversion` record, and it is glued to the tail of another pid's return fragment on the same line (line 406 of `.build/9front-dist/bootchain-trace-run.log`). Whether the distiller missed it for that reason, and whether this recurs, is not established from one run `[ASSUMED]`. Plan a rerun; if it repeats, the fix is in the distiller's line splitting, not in the rename.

### Pitfall 7: aarch64 is a lint-gated crate

`aarch64/Cargo.toml` has `[lints] workspace = true`, which includes `clippy::undocumented_unsafe_blocks = deny` `[VERIFIED: sed display of aarch64/Cargo.toml:14-18]`. But `cargo xtask clippy` does not set `dev_exec`, so SAFETY-comment violations in the code this phase edits are not caught by CI. Match the existing `// SAFETY:` convention by hand.

### Pitfall 8: Carried forward from STATE.md

- raspi3b-on-qemu-11.1.2 boot defect is accepted and pre-existing. r9's default aarch64 config is raspi3b; do not use `cargo xtask qemu` without `--config virt*`, and do not try to fix it here.
- CR-03 (accept-pool sockets) was closed at `f610473` in `port`; arm64 inherits it. The older STATE blocker text describing it as open is superseded.
- "aarch64 is FROZEN" is superseded for this phase by the Roadmap Evolution note; flake comments that say "frozen at the v2.0 pin" (`flake.nix:168-170`, `:242-244`) become stale and should be updated when the apps change.
- `mkimage.sh`'s TARG resolver breaks inside Harvey's nix devshell because hostbin's Plan 9 awk shadows GNU awk `[CITED: STATE.md 18-06]`. That is the x86_64 script, but the same PATH order applies to any new arm64 bake logic: run bakes through the flake apps.
- In-guest rc dialect: no braced `if not`, no bare `=` in patterns, group-free sed `[CITED: STATE.md 15-02, 19-07]`. Applies to any new arm64 fixture.

## Code Examples

### The complete note fix (b1 + b2), aarch64 side

Target for `aarch64/src/proc.rs::push_note`, taken from x86_64 `[VERIFIED: /Users/neilskinner/code/r9/x86_64/src/proc.rs:2349-2373]`:

```rust
    p.note_queue.push_back(note);
    if p.state == ProcState::Blocked {
        p.sleep_until = 0;
        p.state = ProcState::Ready;
    }
    Ok(())
```

Confirm `ProcState::Blocked`, `ProcState::Ready` and `sleep_until` exist with those names on aarch64 before editing; `sleep_until` appears 11 times in `aarch64/src` `[VERIFIED: grep count]`, the enum variants were not opened this session `[ASSUMED]`.

### The rename test fix

```rust
// port/tests/devenv_test.rs:266-267 today
    let n = devenv::write(&mut c, b"arm64-r9", u64::MAX).expect("replace write");
    assert_eq!(n, 10);
// "arm64-r9" is 8 bytes; the old literal "aarch64-r9" was 10
```

### r9 devShell on darwin

`flake.nix:836` lists `pkgs.grub2 pkgs.xorriso` unconditionally. Gate them the way the packages block already gates x86_64 deliverables (`nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux`, `flake.nix` packages section) — for a list, `pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.grub2 pkgs.xorriso ]` `[ASSUMED: standard nixpkgs idiom; whether xorriso evaluates on darwin was not tested]`.

## State of the Art

| Old | Current | Changed | Impact |
|-----|---------|---------|--------|
| objtype `aarch64` | `arm64` | 2026-11-17 | Proven live this session through smoke, net, tier-1 and the donor gate |
| "aarch64 frozen at the v2.0 pin" | Phase 20 reactivates it | 2026-11-17 | Records and flake comments need updating |
| WINDOWS #49 fixed (17-06) | Reopener met | this session | Reopen |

## Assumptions Log

| # | Claim | Section | Risk if wrong |
|---|-------|---------|---------------|
| A1 | aarch64-only r9 changes leave the x86_64 kernel ELF, and so the boot-iso FOD, unchanged | Pin protocol | A stale FOD on r9 main; caught only on the linux host |
| A2 | Harvey's three arm64 CI jobs have been red since pin `aa55a27` and will pass on hosted TCG after the fix | CI | Hosted runners may differ from local TCG (speed, budgets) |
| A3 | The pushword fires come from the aarch64 rfork image under HVF rather than rc | Pitfall 1 | Debug effort aimed at the wrong layer |
| A4 | The pushword rate is per-fork, so smoke is flaky too at a lower rate | Measured start | Smoke may be reliably green on HVF, or worse than 0/6 suggests |
| A5 | aarch64 still has the WINDOWS #44 exit leak | Inventory b8 | If absent, b8 is not needed |
| A6 | The r9 `dev_exec` boot-test profiles pass with the fix | Gates | A profile (most likely `virtnotes`) may need its expectations re-keyed |
| A7 | The trace gate's `fversion` miss is a line-interleave in the distiller | Pitfall 6 | It may be a real change in the traced flow |
| A8 | `ProcState::{Blocked,Ready}` exist under those names on aarch64 | Code examples | Trivial; a compile error |
| A9 | The devShell fix idiom works and xorriso evaluates on darwin | Code examples | Gate both packages, or neither |

## Open Questions (RESOLVED)

All five questions below were answered after this research was written. Each carries a RESOLVED line naming where; the original text is kept as the record of what was asked.

1. **Does the HVF fork defect gate this phase?**
   - RESOLVED: yes. 20-CONTEXT.md D-01 gates the phase on HVF and makes the root cause and fix non-deferrable and not time-boxed (the recommendation below was not taken); D-02 requires a stated repeat count. Planned in 20-02 (rate, N from the lower 95% bound, root cause, owner checkpoint), 20-03 (fix, pin, series), 20-06/20-11/20-17 (series at each pin). Requirement ARM-03.
   - Known: TCG is clean (4/4); HVF fires in about a third of boots; smoke was 6/6 on HVF.
   - Unclear: root cause; true smoke rate.
   - Recommendation: gate on TCG, record HVF, give the defect one time-boxed debug plan, and let the owner decide whether a miss blocks phase close. A larger N (say 20 smoke boots on HVF, about 3 minutes) would settle A4 cheaply.

2. **How much kernel parity belongs here?**
   - RESOLVED: all of it. 20-CONTEXT.md D-03 puts the must tier, b4–b11 and the cpu-port items b12–b16 and c1 in this phase; D-07 keeps it one phase; D-08 adds the capability hardening, the host-test battery and the churn gate; D-09 requires positive pins and leading measurements, which answers the "land without positive pins" concern (20-07 through 20-17).
   - Recommendation: must tier plus b4–b11 as one optional plan; b12–b16 and c1 go with the cpu port. This needs the owner's yes, because b4–b11 land without positive pins.

3. **Should a requirement set be minted?** The roadmap says "Requirements: TBD". Candidates that match the recommended scope:
   - RESOLVED: yes. ARM-01..ARM-12 are in .planning/REQUIREMENTS.md and mapped to Phase 20 in ROADMAP.md. They supersede the six candidates below: ARM-03 is gated on HVF with a derived N (not TCG twice), ARM-06 is not optional, and ARM-07..ARM-12 cover the D-03/D-08 scope.
   - ARM-01: r9 aarch64 `virtshell` builds at HEAD and an r9 CI lane keeps it building.
   - ARM-02: note-interruptible blocking IO and `#c/user`/`#c/hostowner` behave on arm64 as on x86_64; no `hooks not registered` on any gate wire.
   - ARM-03: `nix run .#boot-r9-smoke`, `.#boot-r9-net-smoke`, `.#boot-r9-tier1` green at the pin on TCG, twice consecutively, with HVF results recorded.
   - ARM-04: the rename is proven on every venue: composed gates, `cargo test -p port`, donor `subset-exec.sh`, boot-chain trace.
   - ARM-05: pin advanced push-before-pin as a lock-only commit.
   - ARM-06 (if the optional tier is taken): arch halves of the 15-03/17-02/17-05/19-10 fixes ported.

4. **Who observes hosted CI?** No `gh` here. A human checkpoint, or install `gh` via nix for the session.
   - RESOLVED: the owner, at a blocking human checkpoint after each group's push: 20-06 Task 3, 20-11 Task 3 and 20-17 Task 4, each of which also carries the linux-host `boot-iso` check. `gh` is not installed.

5. **Is the smoke gate allowed to grow a post-pass?** Adding a `not registered` check changes a gate that has been byte-stable since v2.0. Recommended, but it is a gate change.
   - RESOLVED: yes. 20-01 Task 3 adds the `hooks not registered` absence post-pass and the `pushword but no argv` fast-fail arm to all three arm64 expect drivers, after measuring each gate's baseline counts; the `unknown device` and `unsupported this era` checks are added only where the measured baseline is zero. D-01 (the gates must name the defect) and ARM-02 (no `hooks not registered` line on any gate wire) require it.

## Environment Availability

| Dependency | Needed for | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| nix | everything | yes | 2.34.6 | — |
| r9 nightly toolchain | kernel build, port tests | yes, via `#kernel-virt` shell | 1.95.0-nightly 2026-01-29 | — |
| r9 default devShell | convenience | no (grub2) | — | `#kernel-virt` shell + `nix shell` for qemu |
| qemu-system-aarch64 | gates | yes, via nix | 11.1.2, hvf + tcg | — |
| lima guest `harvey` | namespace build | yes, Running | lima 2.2.0 | — |
| 9front donor tree + dist | donor gates | yes (`.build/9front-donor`, `.build/9front-dist`) | pin `350266104f` `[CITED: donor-vm-build.sh header]` | — |
| x86_64-linux + KVM | FOD proof, x64 gates | no | — | owner's linux host |
| `gh` | hosted CI observation | no | — | browser, or `nix shell nixpkgs#gh` |

**Missing with no fallback on this host:** the FOD verification. Plan around it by keeping edits out of `port/src/`.

## Validation Architecture

Skipped: `workflow.nyquist_validation` is `false` in `.planning/config.json`. The gate commands the planner needs are in "Existing arm64 Gates" and "Venues".

## Security Domain

`security_enforcement` is absent from the config, so this section applies. The phase adds no network surface, no new parsing of untrusted input, and no cryptography.

| ASVS category | Applies | Control |
|---------------|---------|---------|
| V2 Authentication | no | The auth domain is not ported this phase |
| V3 Session management | no | — |
| V4 Access control | yes | b3 restores the per-proc user that `#c/hostowner`'s eve gate (`port/src/devcons.rs:288-295`) compares against; fork must copy the user, never share a mutable one |
| V5 Input validation | yes, if b4–b11 are taken | Every new syscall arm validates user memory through the existing `UserSlice` path, as the x86_64 originals do; do not add raw pointer reads |
| V6 Cryptography | no | devssl/devcap arms are deferred |

| Threat | STRIDE | Mitigation |
|--------|--------|------------|
| Hostowner write by a non-eve proc once hooks are registered | Elevation | The gate is already in `port`; confirm on the wire that boot's write succeeds and nothing else can |
| Unpinned kernel arms reaching a release | Tampering | Keep unexercised arms (b12–b16) out until a gate exists |
| A stale FOD shipped on r9 main | Tampering | Owner checkpoint on the linux host |

## Sources

### Primary (executed or read this session)
- Scratch clone of r9 at `8e4e730` with the 9-line probe patch: build, `cargo test -p port`, `xtask check/clippy/ureg-check`
- Harvey apps `boot-r9-smoke`, `boot-r9-net-smoke`, `boot-r9-tier1` via `--override-input`, 17 boots across HVF and TCG
- `build/aarch64/donor-vm-build.sh`, `subset-exec.sh`, `harness trace-syscalls --boot-chain`
- r9: `git log dbea82f..HEAD`, per-commit `--stat`/`--numstat`; `port/src/{devpipe,devip/stack,namec,devcons}.rs`; `aarch64/src/{main,syscall,proc}.rs`; `x86_64/src/{main,syscall,proc}.rs`; `flake.nix`; `.github/workflows/xtask.yml`; `aarch64/lib/config_*.toml`
- Harvey: `flake.nix`, `flake.lock`, `nix/{smoke-r9,vm-build}.sh`, `build/aarch64/{vm-run,subset-build,subset-exec,donor-vm-build}.sh`, expect drivers, tier lists, `.github/workflows/*.yml`, `sys/src/cmd/rc/havefork.c`, `build/harness/src/trace.rs`

### Secondary (planning records, not re-verified)
- `.planning/STATE.md`, `ROADMAP.md`, `REQUIREMENTS.md`, `PROJECT.md`, `WINDOWS.md` (#49)
- `.planning/quick/261117-bz5-…/261117-bz5-SUMMARY.md`
- `.planning/phases/19-…/19-05-SUMMARY.md`, `19-PATTERNS.md`, `17-…/17-06-SUMMARY.md`

### Not consulted
- No web or Context7 lookups were needed; the domain is entirely in-repo. The `research-plan`/`research-store` seam was not used for that reason.

## Project Constraints (from AGENTS.md)

- Objtype, cputype and arch dir are `arm64`; host-side names stay `aarch64` (`build/aarch64/`, qemu binary, Rust triple, nix systems, r9's crate dir).
- Harvey C: tabs, lowercase, `nil`, `exits`/`sysfatal`, `/* */` comments, no header guards, explicit comparisons. r9 follows its own `rustfmt.toml`.
- `$PLAN9/bin` last on PATH; host tools build with plan9port.
- Namespace builds need Linux mount namespaces: the lima guest on macOS.
- In-tree fixes are preferred over external patch stacks.
- Start file-changing work through a GSD command; do not edit the repo outside a GSD workflow.
- User global rule: never add AI attribution to commit messages, in either repo.

## Metadata

**Confidence breakdown:**
- Compile drift and gate results: HIGH — executed.
- Drift inventory classes (a) and (b): HIGH for presence/absence (mechanical diffs), MEDIUM for port effort (sizes are x86_64-side counts).
- Scope recommendation: MEDIUM — depends on two owner decisions.
- HVF defect cause: LOW — a lead only.
- FOD neutrality of aarch64-only changes: MEDIUM — reasoned, not verifiable on this host.

**Research date:** 2026-11-17
**Valid until:** the next r9 commit touching `port/` or `aarch64/`. Re-run the build and the smoke before trusting the inventory past that.
