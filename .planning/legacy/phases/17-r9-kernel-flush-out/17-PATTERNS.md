# Phase 17: r9 Kernel Flush-Out - Pattern Map

**Mapped:** 2026-11-12
**Files analyzed:** 27 (new/modified across both repos)
**Analogs found:** 27 / 27 (every file has an in-repo analog; this phase's novelty is the fixes, not the machinery — confirmed by research and verified reads)

**Path convention:** `../r9/...` = the sibling r9 repo at `/home/req/code/r9` (independent git repo, branch `main`, all named paths verified `git ls-files`-tracked there). Unprefixed paths = Harvey repo at `/home/req/code/harvey` (verified tracked). aarch64 files are REFERENCE ONLY (frozen at v2.0 pin) — copy their fix shapes into x86_64, never edit them.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `../r9/port/src/netif.rs` | kernel lib (shared) | event-driven | itself: WaitSet at :330-364 (one of the three copies to resize) | exact |
| `../r9/port/src/devip/stack.rs` | kernel lib (shared) | event-driven | itself: `note_rx_pending` :294-304 + `tick_check` :265-279 (CR-07 terrain) | exact |
| `../r9/port/src/devip/conv.rs` | kernel lib (shared) | event-driven | `../r9/port/src/netif.rs:330-364` (identical WaitSet copy) | exact |
| `../r9/port/src/devip/proto.rs` (+ `waker.rs`) | kernel lib (shared) | event-driven | itself: block loop :739-755; `../r9/port/src/devip_tests.rs` waker pins | exact |
| `../r9/port/src/virtio.rs` | device transport lib | transform | itself: `next_used` V5 validation :730-757 and `Caps::parse` :128-182 | exact |
| `../r9/port/src/namec.rs` | name-resolution service | request-response | itself: `chain_snapshot` locking :130-212, `walk_one` :312-318, `create_chan` :390 | exact |
| `../r9/port/src/devpipe.rs` | kernel device driver | streaming | `../r9/port/src/devip/proto.rs:739-755` (register→drop→block idiom) | role-match |
| `../r9/x86_64/src/platform/q35/pci.rs` | platform driver (per-arch) | transform | itself: extent check :976-986, `bar_assignment` :803-814, `size_bar64` :819-833 | exact |
| `../r9/x86_64/src/proc.rs` (+ its `mod tests`) | per-arch kernel substrate | request-response + event-driven | `../r9/aarch64/src/proc.rs` fix family (reference) + own `mod tests` :2506-2680 | exact |
| `../r9/x86_64/src/exec.rs` | syscall implementation | request-response | itself: `exec_replace` parse→free→seed ordering :384-427 | exact |
| `../r9/x86_64/src/syscall.rs` | syscall dispatch | request-response | itself: semacquire loop :900-911; `../r9/port/src/namec.rs:130-212` snapshot pattern | exact |
| `../r9/port/src/virtio_tests.rs` (extend) | test (host pin) | transform | itself: `used_drain_rejects_out_of_range_and_odd_ids` :605-616 | exact |
| `../r9/port/src/devip_tests.rs` (extend) | test (host pin) | event-driven | itself: header pin-map + SEAM_TEST_LOCK discipline :1-42 | exact |
| `../r9/x86_64/lib/config_q35churn.toml` (new; name planner's) | config (boot profile) | batch | `../r9/x86_64/lib/config_q35net.toml` (whole file) | exact |
| `../r9/xtask/src/main.rs` (extend) | build/gate tooling | batch | itself: X64Net profile wiring :1812-1833 + assertions :2364-2390 | exact |
| `../r9/x86_64/src/platform/q35/` churn/stress kernel trigger (new, cfg-gated; if boot-context driver needed) | gate-driver (kernel-side) | batch | `../r9/x86_64/src/platform/q35/netgate.rs` (whole file — the 13-05 precedent) | exact |
| `../r9/.github/workflows/xtask.yml` (likely NO edit) | CI config | batch | itself: `tests` job already runs `cargo xtask test` which runs both host batteries (main.rs:2834-2853) | exact |
| `sys/src/cmd/rc/simple.c` | command (userspace C) | file-I/O | itself: `doredir` :77-97; `sys/src/cmd/rc/havefork.c:134-141` (06-07 fix precedent) | exact |
| `sys/src/cmd/rc/plan9.c` | command (userspace C) | file-I/O | itself: `Execute` :357-400, `Dup` :582-586 | exact |
| `sys/src/cmd/rc/exec.c` | command (userspace C) | transform | itself: `pushword`/`popword` :42-62; havefork.c:134-141 invariant-restore precedent | exact |
| `build/x86_64/churn-r9.expect` (new; name planner's) | gate-driver (expect) | batch | `build/x86_64/netsoak-r9.expect` (whole file — the 16-05 soak driver) | exact |
| `nix/churn-r9-x64.sh` (new) + `flake.nix` app (extend) | config (flake app) | batch | `nix/soak-r9-net-x64.sh` + `netSoakR9X64Pkg` flake.nix:317-330 | exact |
| `flake.lock` (repeated FOD re-wire) | config (pin) | batch | prior pin advances (16-04: r9 rev `208ceba…` == origin/main today) | exact |
| `.planning/WINDOWS.md` | ledger | N/A (record) | itself: row #44 open-shape :645-657 and the resolved row above it :640-644 | exact |
| `.planning/milestones/v2.0-phases/07-…/07-REVIEW-DISPOSITION.md` | ledger | N/A (record) | itself: table :62-84 + 08 ledger's `fixed` row shape | exact |
| `.planning/milestones/v2.0-phases/08-…/08-REVIEW-DISPOSITION.md` | ledger | N/A (record) | itself: CR-01..06/WR rows :25+ | exact |
| `.planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md` (new) | ledger index (closure table) | N/A (record) | `08-REVIEW-FIX.md` per-finding table (Files modified / Commits / Applied fix) :25-101 | role-match |

## Pattern Assignments

### Wave 1 — carried-fix audits + host pins (FLUSH-01/02)

#### `../r9/x86_64/src/proc.rs` (per-arch kernel substrate; RFMEM/OOM audit D-02 + CR-03 pin home)

**Analog:** `../r9/aarch64/src/proc.rs` (the frozen fix family — read-only diff baseline) + the file's own `mod tests`.

**Audit anchors** — the four sites research verified as already carrying the family (do NOT "fix" these; audit every OTHER OOM release against them):
- brk-unwind free-verdict honor, `x86_64/src/proc.rs:787-804` — `if release_user_page(done_pa) { free_phys_page(done_pa) }` per member
- bss-sweep record-or-unwind, :2384-2403 — on child-map failure, unmap the parent's UNTRACKED mapping and free the page
- rfork failure pivot, :2274-2286 — Moribund + release-pivot walk + `free_root_table_and_kstack`
- exit release pivot, :1325-1327/:1317-1339 — `free_user_mappings` honoring each record's release verdict

**Existing per-arch test pattern** (`x86_64/src/proc.rs:2506-2662`, `#[cfg(test)] mod tests` with `use super::*;`):
```rust
    /// RFMEM shared-page refcount decisions (the aarch64 pins,
    /// verbatim).
    #[test]
    fn rfmem_shared_pages_lifecycle() {
        let mut t = SharedPages::new();
        assert!(t.release(0x1000)); // never shared: frees
        t.share(0x2000); // parent + child = 2
        assert!(!t.release(0x2000)); // first sharer exits — sibling holds
        assert!(t.release(0x2000)); // last sharer frees
        ...
    }
```
New CR-03 deterministic interleaving pins go HERE (D-07 host half). The richer 4-pin reference is `../r9/aarch64/src/proc.rs:2689-2758` (`mod sem_tests`: `register_is_idempotent`, `wake_consumes_picked_registrations_in_order`, `reregistered_waiter_is_found_by_the_next_release`, `wake_never_touches_other_words`) — x86_64 compressed these into 1 (`sem_registry_consumes_and_reregisters`, :2662); extend toward the aarch64 shape using its `waiters_for(pa)` helper idiom (:2695-2698).

**Loop-under-pin** (`x86_64/src/syscall.rs:900-911` — the fix shape CR-03's pins guard):
```rust
    loop {
        // Register BEFORE every block: sem_wake CONSUMES registrations
        // when it picks its wakers, so a woken waiter that loses the
        // unit race must RE-register before re-blocking.
        proc::sem_register_waiter(pa, proc.pid);
        if sem_try_acquire(&mut word) {
            break;
        }
        proc::sched();
    }
    proc::sem_deregister_waiter(pa, proc.pid);
```

#### `../r9/x86_64/src/exec.rs` (the un-audited OOM remainder + #44 suspect)

**Analog:** itself — `exec_replace` :384-427. The ordering doctrine every new OOM-release must preserve: PARSE first → every VALIDATION before the free → FREE second (`proc.free_user_mappings()` drops `text_source`, releasing the `ByteBacking::Mnt` Arc) → SEED third, where post-free failure kills via `exit_tail` (:415-421). #44's gate assertion targets the staging around `ByteBacking::Mnt { file: Arc<Vec<u8>> }` (declared `x86_64/src/proc.rs:260-265`); the audit reads `install_image`'s every allocation for the same record-or-unwind hygiene as the proc.rs anchors above.

#### `../r9/port/src/devip_tests.rs` (host pin home for the CR-01..06 re-cites + WaitSet pins)

**Analog:** itself — the crate-root test-module discipline (header :1-42): stub hooks registered once per process via `std::sync::Once`, an ADVANCEABLE stub clock, stateful pins serialized on a `SEAM_TEST_LOCK`, wire packets crafted with smoltcp's own `wire` codecs, sequence numbers never hardcoded. The 12 already-fixed findings' pins already live here and pass (`cargo test -p port --lib` = 101) — D-01 is re-cite + boot-gate re-green, NOT new pin construction. New WaitSet (WR-05) pins target the struct directly as pure logic (Pitfall 8: never the published-device path — `devether_tests` consumes the single publish).

---

### Wave 2 — Phase-7 hardening class-sweep (FLUSH-03)

#### `../r9/port/src/netif.rs` + `devip/stack.rs` + `devip/conv.rs` (WaitSet resize, WR-05)

**Analog:** the existing WaitSet (`netif.rs:330-364`; identical shape at `stack.rs:172-194`, `conv.rs:120-136`):
```rust
struct WaitSet {
    pids: [u64; 16],
}
...
    fn register(&mut self, pid: u64) {
        if pid == 0 || self.pids.contains(&pid) {
            return;
        }
        if let Some(slot) = self.pids.iter_mut().find(|p| **p == 0) {
            *slot = pid;
        }
        // The full case is impossible (one waiter per proc, ≤ 16 procs).
    }
```
Fix shape: ONE port-level const sized to the real table — `MAX_PROCS: usize = 512` on BOTH arches (`x86_64/src/proc.rs:111`, `aarch64/src/proc.rs:73`) — all three arrays, all `take(&mut [u64; N])` callers (`stack.rs:271, :296`), and the full arm made loud (count+report). The port crate CANNOT import the arch const; stale `[u64; 16]` comment text ("MAX_PROCS=16") must be corrected too. Pins: pure-struct tests at all three homes.

#### `../r9/port/src/virtio.rs` (WR-06 used-id liveness + WR-04 cap parse)

**Analog:** itself — `next_used` :730-757, whose existing V5 block IS the pattern for the new check:
```rust
        if id >= u32::from(self.qsize) || id & 1 != 0 {
            return Some(Err("virtio: used-ring id out of range (device fault)"));
        }
        ...
        // Slot recovery (ethervirtio.c): the used id is the chain head.
        let slot = ((id & u32::from(self.qmask())) >> 1) as usize;
        self.posted[slot] = false;
```
Add before the clear: `if !self.posted[slot] { return Some(Err("virtio: used-ring id for an unposted slot (device fault)")); }` — same string vocabulary ("(device fault)", loud Err, never a panic). `Caps::parse` :128-182 gains the `p & 3` alignment reject and `cap_len >= CAP_NOTIFY_LEN` in the notify arm, matching its existing early-return rejects (:141-152).

**Pin home** (`port/src/virtio_tests.rs`, crate-root beside devip_tests — header :1-8 documents the placement rule). Copy the negative-test shape verbatim from :605-616:
```rust
#[test]
fn used_drain_rejects_out_of_range_and_odd_ids() {
    let mut r = rig(8, Dir::Tx);
    r.ring.tx_post(0x2000_0000, 60, &r.notify, 0, 1).unwrap();
    device_complete(&r, 8, 72); // id == qsize — out of range
    assert!(drain_one(&mut r).is_err(), "id >= qsize is a device fault");
    ...
}
```
The synthetic lying-cap PciCfg for WR-01/02 pins builds on the module's `caps_fixture()` (:244) and `Rig` (:284).

#### `../r9/x86_64/src/platform/q35/pci.rs` (WR-01/02/03 loud-abort holes)

**Analog:** itself — the existing T-07-11 extent check :976-986:
```rust
    // Each cap's (bar, offset, length) bounds-checked against the BAR
    // size BEFORE the region is trusted (T-07-11 — a lying cap aborts
    // the bind loudly).
    for t in [caps.common, caps.notify, caps.isr, device] {
        if u64::from(t.offset) + u64::from(t.length) > bar_size {
            return Err("virtio: capability extent beyond the bar (device fault)");
        }
    }
    if device.length < 6 {
        return Err("virtio: device config capability too small for a MAC");
    }
```
The four holes to close as ONE class commit (Pitfall 5 — the whole x64 battery re-runs after, by design): `offset == bar_size || length == 0` passes (needs `>=`-style edge + minimum extents: common ≥ 0x40, isr ≥ 1, notify_off×multiplier ≤ notify length); `device.length < 6` but the IRQ handler reads le16 at devcfg+6 (`:1343` — needs 8); `size_bar64` :819-833 writes all-ones with no type-nibble validation; `bar_assignment` :803-814 checks window END but not natural alignment. `bar_assignment` is already `#[cfg(any(test, dev_ether))]` — host-testable today. Every reject is a bind-time `Err`, never a panic (V5 device-input posture).

---

### Wave 3 — CR-07 root-cause + pins (the centerpiece; folds #47/#50/#66)

#### `../r9/port/src/devip/stack.rs` + `devip/proto.rs` + `waker.rs` (the waiter registry + wake bridge)

**Analog:** the block/wake discipline already in the tree — proto.rs's per-syscall loop :739-755:
```rust
                    t.table_mut(Proto::Icmp)[id].waiters.register(pid);
                }
                stack::register_net_waiter(pid);
                ...
                stack::drive_stack();
                (hks.block)();
```
and the two drain-all wake entries — `note_rx_pending` (stack.rs:294-304, IRQ-side) and `tick_check` (:265-279, timer twin):
```rust
pub fn note_rx_pending() {
    let mut pids = [0u64; 16];
    let n = NET_WAITERS.enter().expect("devip: waiters contended (IRQ invariant violation)").take(&mut pids);
    if n == 0 {
        STACK_DEBT.store(true, Ordering::Release);
        return;
    }
    if let Ok(h) = hooks() {
        (h.wake)(&pids[..n]);
    }
}
```
The hook it drives (`x86_64/src/proc.rs:1973-1981`, `pipe_wake`) takes `PROC_TABLE` (512 slots) in IRQ context — the thundering-herd terrain of the three recorded candidates. Constraints on ANY fix: IRQ-context discipline documented at :285-293 (NET_WAITERS BusyCell provably uncontended; no port mcslock in IRQ; wake hook runs after the guard drops); poll NEVER runs in IRQ context; WAKE-only from IRQ. Register sites: 8 call sites in proto.rs (:744, :833, :1170, :1388, :1777, :1859, :2184, :2502).

**Deterministic pin home:** `../r9/port/src/devip_tests.rs` — its pin (b) `register_wake_reregister_never_loses_a_wake` is the exact bridge-contract precedent; new multi-waiter interleaving pins extend the stub-wake-log harness. **Stress leg:** re-run the 16 soak burst shape — see the composed-gate patterns below.

#### `../r9/port/src/devpipe.rs` (#50, folded D-09)

**Analog:** `proto.rs:739-755` — the register → drop guard → block shape (devpipe's read loop already follows it; verify against the CR-07 root-cause outcome, then close or split out with its own 3-part disposition).

---

### Wave 4 — namec + rc + gates (FLUSH-04, D-10/D-11)

#### `../r9/port/src/namec.rs` (#45 union-create fix; #18 verify)

**Analog:** itself — the per-consult snapshot locking (:130-212, LOCKING comment at :130-138; implementation `let guard = pg.lock(&node); guard.chain_snapshot(&c.path)` then lock-free walks at :206-212) is #18's fix shape, ALREADY in shared code. #18's remaining work: audit the four pgrp-lock sites in `x86_64/src/syscall.rs` (:527, :599, :716 are `pg.lock`; :1159 is `proc.pgrp.lock` — two spellings, so a site-count grep needs `-E '(pg|pgrp)\.lock'` (4 at HEAD; bare `pg\.lock` finds 3); each site already preceded by a "NO pgrp lock is held across…" comment at :471, :699, :962; verify each drops the lock before any walk) + a contention boot gate (rc pipelines). #45's fix site: `walk_one` :312-318 is union-blind (`dev_walk(c, name)` with no mount-point consultation) and `create_chan` :390 errors `"namec: device has no create"` for union heads — the fix makes sys_create's final-component probe consult the union chain.

**Pin:** host test if the namec machinery is reachable (extend devip_tests-style crate-root module; the research cites devroot_rawimg/devenv test precedent), else the boot-gate rc `>` write (the original #45 repro shape).

#### `sys/src/cmd/rc/simple.c` + `plan9.c` + `exec.c` (D-11, one fix-class commit)

**Analog:** the in-tree 06-07 rc fix at `havefork.c:134-141` — the exact shape of an rc corner fix: a block comment naming the measured flake + restoring the invariant, style(6)-conformant:
```c
		/* 06-07: at a TOP-LEVEL backquote assignment the popped "stop" list
		 * was the stack's ONLY list — the stack then reads argv==0 and the
		 * next op's pushword/deref faults ("pushword but no argv", a measured
		 * flake on the r9 tier-1 sweep's `out=\`{...}\`` checks).  Restore
		 * the invariant every reader assumes: Xbackq leaves a list on the
		 * stack for the result (empty or not). */
		if(runq->argv==0)
			pushlist();
```
Fix sites (all verified this session):
- #46: `simple.c` `doredir` :77-97 — `Dup(rp->from, rp->to)` unchecked in both ROPEN (:85) and RDUP (:90) arms; `Dup` is the thin wrapper `plan9.c:582-586` (`return dup(a, b);`). Fix checks the return, reports via the file's error idiom.
- #65: `plan9.c` `Execute` :357-400 — only the LAST path candidate's errstr survives (:390 `rerrstr` overwrites inside the loop; :396-398 prints). Fix captures the FIRST meaningful errstr per the comment's own contract (:383-389).
- #49: `exec.c` `pushword`/`popword` :42-62 — `panic("pushword but no argv!", 0)` / `panic("popword but no word!", 0)` are the crash surfaces; fix restores the argv-stack invariant at the caller (the havefork.c 06-07 precedent — do not "fix" the panic sites).
Style: tabs, `/* */` only, `nil`, `-1`+errstr, `exits`, no auto-init at declaration, match the surrounding file. Proof = existing rc-driven sweeps re-green + a cheap targeted fixture ONLY if the corner needs one (Pitfall 6: no host unit tests, no r9 pin mechanics).

---

### Pin/gate machinery (D-05/D-06/D-07 venues)

#### `../r9/x86_64/lib/config_q35churn.toml` (new boot profile) + `../r9/xtask/src/main.rs` (profile wiring)

**Analog:** `../r9/x86_64/lib/config_q35net.toml` — profile = `[build]`/`[link]`/`[config]` (platform + `dev = [...]` keys gating cfg attributes) / `[qemu]` (+ optional netdev tables) / `[dist]`. A churn profile is `config_q35.toml`-shaped with the exec/module staging of `config_q35exec.toml`; a stress profile is q35net's shape minus new netdev semantics (Pitfall 7: existing profiles' semantics untouched — new TOMLs only).

**xtask wiring points** (main.rs): a new `BootProfile` variant beside `X64Net`/`X64Exec`/`X64Shell` (:1410 enum, doc-comment per profile); config-name gate at :1812-1833 (`let q35net_profile = self.arch == Arch::X86_64 && self.config_name == "q35net";` — copy the shape); marker constants as `const …_MARKER: &str` beside the q35net family (:855-866); ordered stage assertions + finish-time post-pass in the `match profile` arm (:2344-2430 — the `X64Net` pcap post-pass :2364-2390 is the precedent for a heap_stats-bound assertion parsed from the kernel's own marker, never a second hardcoded constant). Markers: presence-based, never timing/interleaving (the v1.0 discipline, :2308-2313).

#### `../r9/x86_64/src/platform/q35/` kernel-side churn trigger (if boot-context, cfg-gated)

**Analog:** `netgate.rs` (whole file) — the complete shape: file-top comment naming the phase/task and the donor machinery re-keyed; `cfg(dev_*)` gating; `masked_println!` for any post-sti boot-context print (:61-68); bounded waits with `timer::ticks()` + hlt (:269-303); fail-closed `netgate: FAIL <stage>` prints with the terminal marker withheld (:156-263); and its OWN `#[cfg(test)] mod tests` pinning the reconcile logic as pure functions (:317-404). The churn driver's memory-bound assertion consumes `heap_stats()` (`x86_64/src/allocator.rs:23,38`, kernel-only) — assert the cursor SLOPE, not absolute free bytes (the doctrine comment at `port/src/allocator.rs:574-583`).

#### `build/x86_64/churn-r9.expect` + `nix/churn-r9-x64.sh` + `flake.nix` app (Harvey composed churn leg, D-06b)

**Analog:** `build/x86_64/netsoak-r9.expect` (the 16-05 driver) — copy its whole skeleton: header documenting stage order + the multi-waiter warning slot; required-env pre-checks exiting 2 (:123-129); fresh-capture rm (:139); `stage` proc with per-stage + overall budgets where a stall IS the finding (:217-240); `expect_before` fast-fail on panic markers (:248-255); ordered boot stages then typed fixture launch with argv-riding gate data (:257-273); whole-capture COUNT post-passes — `count_occurrences` + exact-count assertion (:299-318) — which is exactly the `/bin/cat` ×~1400 count shape; distinct marker prefixes (`netsoak:` vs `netip:` vs `netgate:` — carry a `churn:`-family prefix); every exit path kills qemu's group (:167-177). CR-07's stress leg re-runs the burst shape: `baseline_rounds 2048 / burst_count 8 / burst_rounds 512` (:119-121) — #66 fired 1-in-5 under exactly this load; green-twice-plus with planner-stated N.

**Flake app analog:** `netSoakR9X64Pkg` (flake.nix:317-330) — `mkScriptApp pkgs "harvey-boot-r9-…" ./nix/<script>.sh '' export R9_KERNEL_X64_PKG=… export R9_ISO_BUILDER=… ''` with `pkgs.expect` in runtimeInputs; the script analog is `nix/soak-r9-net-x64.sh` (env contract, bake, iso compose, driver spawn, EXIT-trap survivor sweep keyed on the run tag).

#### `flake.lock` (FOD re-wire) + `../r9/.github/workflows/xtask.yml` (CI)

The pin-advance loop is locked: every r9 kernel-byte class-commit → push → Harvey `flake.lock` r9 FOD re-wire → pin advance as its OWN commit → gates re-green (current base: r9 `208ceba` == origin/main). CI placement of host tests likely needs NO workflow edit: the `tests` job already runs `cargo xtask test --verbose`, and `TestStep` (xtask/src/main.rs:2823-2865) already runs `cargo test -p port --lib` AND `cargo test -p <arch> --bins --target <host-triple>` — the `--target` override Pitfall 1 is already encoded there. Only a new boot-profile CI leg would touch the yml (copy the `boot-test-x86-64` job's explicit-TCG shape).

---

### Ledger closure (D-13/14/15/16)

#### `.planning/WINDOWS.md` + the two REVIEW-DISPOSITION ledgers + `17-FLUSHOUT.md`

**WINDOWS row closure** — edit the JSON block in place; row #44's open shape (:645-657) becomes the closed shape of the row above it (:639-644): `"status": "fixed"` (the ledger's native closure token at HEAD — 7 fixed / 55 open / 3 waived / 0 `resolved`; D-14's literal `resolved` maps onto it), `"reason": "r9 <sha> + <pin name>"` (machine-shaped, greppable), `"resolved_at": "<stamp>"`. Apply to #18, #44–#47, #49, #50, #65, #66 ONLY (D-12).

**Disposition ledger closure** — hand-edit rows in place (D-13). 08 ledger row shape (08-REVIEW-DISPOSITION.md, the CR-01 row): `| CR-01 | critical | fixed | 08-REVIEW-FIX.md — socket lingers… (r9 bd4db36 + pin 7300113) |` → Source cell re-cites the x86_64 pin (host test name or boot profile). CR-07's row flips `skipped` → `fixed`. 07 ledger (07-REVIEW-DISPOSITION.md:62-84, 13 open WR/IN rows): WR-01..06 → `fixed` with pin cites; WR-07/08 → dispositioned with the 3-part D-15 rationale (WHERE the class lives / WHY not fixed / WHAT reopens it); IN rows STAY `open` (Pitfall 10).

**17-FLUSHOUT.md** — one row per finding: finding ID → action (fixed/dispositioned/folded) → pin → commit (r9 or Harvey SHA) → ledger refs. Analog for the table shape: `08-REVIEW-FIX.md`'s per-finding entries (Files modified / Commits / Applied-fix prose, :25-101) — D-16's five columns are that shape minus the prose.

## Shared Patterns

### Block/wake discipline (register → drop guard → block; re-register before every re-block)
**Source:** `../r9/x86_64/src/syscall.rs:900-911` (sem), `../r9/port/src/devip/proto.rs:739-755` (devip loops), documented in `netif.rs:380-389` (the devpipe idiom hooks)
**Apply to:** every CR-07-era waiter change (stack.rs, conv.rs, devpipe.rs, waker.rs) and every WaitSet consumer.

### Loud device-input rejection (V5 posture)
**Source:** `../r9/port/src/virtio.rs:744-752` — `return Some(Err("virtio: … (device fault)"))`; `../r9/x86_64/src/platform/q35/pci.rs:976-986` — bind-time `Err` before the region is trusted
**Apply to:** all WR-01/02/03/04/06 fixes — a loud bind/walk fail, never a panic, never silent acceptance; keep the `(device fault)` string family.

### Leak-gate assertion = memory-side probe, slope not absolute
**Source:** `../r9/port/src/allocator.rs:574-583` (the cursor doctrine, verbatim in RESEARCH) surfaced via `heap_stats()` (`x86_64/src/allocator.rs:19-38`); oom observability `x86_64/src/runtime.rs:24-31`
**Apply to:** the r9-side churn profile's bound and the composed leg's assertion; Moribund-husk accumulation is a known era shape — do not chase it as a leak.

### Expect-driver discipline (presence markers, count post-passes, budgets that fail loud)
**Source:** `build/x86_64/netsoak-r9.expect` (:49-61 marker discipline, :217-240 stage proc, :283-332 post-passes)
**Apply to:** the churn expect driver and the CR-07 stress leg; typed launch lines ≤ ~120 chars; distinct marker prefixes per producer.

### Venue honesty + class-commit + re-wire-then-pin
**Source:** flake.nix app-per-gate family (:165-330); the locked carried-forward rules (CONTEXT); `../r9/xtask/src/main.rs:2308-2313` (presence, never order)
**Apply to:** every fix pass: host-pin where host-compilable, boot profile only where kernel context is essential; one commit per bug class; FOD re-wire + pin advance after every pass; no edits to existing gate/profile semantics.

## No Analog Found

None. Every file this phase touches has a tracked in-repo analog (the research's key insight: all pin machinery already exists in one of the two repos). The two role-match (not exact) rows:
- `17-FLUSHOUT.md` — no prior phase shipped this exact closure-table shape; `08-REVIEW-FIX.md`'s per-finding table is the closest (same columns in spirit, D-16 adds pin/ledger-ref columns).
- `../r9/port/src/devpipe.rs` — same subsystem but the analog is the devip block-loop idiom, not a devpipe-specific fix precedent.

## Metadata

**Analog search scope:** `../r9/{port/src, port/src/devip, x86_64/src, x86_64/src/platform/q35, x86_64/lib, aarch64/src (reference), xtask/src, .github/workflows}`; Harvey `{sys/src/cmd/rc, build/x86_64, nix/, flake.nix, .planning/}` (both phase dirs and milestones/v2.0-phases/07+08)
**Files scanned:** ~35 read or grep-verified this session; all named analog paths verified git-tracked in their repo (Harvey `git ls-files` + r9 `git ls-files`)
**Tracked-source gate:** clean — no gitignored/mirror paths named; r9 is an independent sibling repo, not a submodule
**Pattern extraction date:** 2026-11-12
