# Phase 17: r9 Kernel Flush-Out - Research

**Researched:** 2026-11-12
**Domain:** r9 kernel defect-closure (Rust no_std kernel, sibling repo `../r9`) + Harvey rc userspace corners + gate/ledger plumbing
**Confidence:** HIGH (every fix site, ledger row, and pin venue read this session; host batteries executed live)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Fix-vs-disposition posture (FLUSH-01/02/03)**

- **D-01:** **Pin-first, fix-on-failure for the 12 already-fixed shared-code findings** (CR-01..06, WR-01/02/04/05/06). Their fixes are live in `port/src/devip/*`; the x86_64 work is building the pins (D-05 venue rules). A finding reopens as a fix-item ONLY if its pin fails — no speculative code re-audit. The pins ARE the audit. Note: CR-03 and WR-03 carried `requires-human-verification` flags at Phase 8 (race/leak logic in host-unreachable paths) — this phase's deterministic sema test (D-07) and leak-repro gates (D-06) are precisely that verification, re-keyed to x86_64.
- **D-02:** **Audit + port + prove for the RFMEM OOM-path leak family (FLUSH-02).** The researcher audits `x86_64/src/proc.rs` against the aarch64 fix family (brk-unwind free-verdict honor, rfork bss-sweep reorder — map-parent-then-child with record-or-unwind on child-map failure — and every OOM-path release), ports whatever is missing, THEN the leak-repro gates run (D-06). Prove-only was rejected: a slow leak the gate's sample misses stays missed.
- **D-03:** **CR-07 (multi-waiter RX-IRQ wake wedge) gets root-caused, fixed, and pinned — the phase's centerpiece.** Never fixed anywhere; skipped at Phase 8 with root cause unpinned; WINDOWS #66 reproduced the class live on x86_64 under the phase-16 soak burst. The fix must land with a race-stress pin (D-07), not just a logic argument.
- **D-04:** **Phase-7 WRs: fix the named classes where the code is LIVE on x86_64; disposition the frozen/superseded.** Live fixes: WaitSet-16-vs-MAX_PROCS-64 sizing (shared wait machinery), RX used-id slot-liveness (virtio port driver code), cap-walk loud-abort holes (audit the x86_64 ECAM/cap walk from 13-04 for the same admission holes WR-01/02/03/04 name — zero-length/undersized extents, short config reads, unvalidated BAR types, unaligned cap pointers — each becoming a loud bind/walk fail). Dispositions: WR-08 (DTB/ICFGR trigger flags — aarch64-only, frozen) and WR-07 (superseded aarch64 fixture quirk). — **Reversibility:** costly — the cap-walk audit touches the q35 discovery path every net gate boots through; changes there re-run the whole x86_64 gate battery.

**Pin architecture (the FLUSH-04 "every fix lands with a pin" discipline)**

- **D-05:** **Host-first, boot-when-needed.** Every finding gets the cheapest pin that can actually fail: a host `cargo test` in r9 where the logic is host-compilable (the precedent already exists — `port/src/devip/{ipifc,arp,netlog,phyadapt,stack}.rs`, `port/src/{rendez,mem,allocator}.rs` all carry `#[cfg(test)]` today), and an xtask boot-test profile only where kernel context is essential (real IRQ paths, real sockets, real page tables). "Always both" was rejected as double gate work with no extra failure signal.
- **D-06:** **Tiered leak gates.** (a) r9-side: an xtask boot profile looping exec of a baked binary with a memory-bound assertion (kernel-level proof, 14-D-06 split); (b) Harvey-side: ONE composed churn leg driving real binaries — the #44 repro shape (`/bin/cat` ×~1400 fork+exec iterations) with a bounded-memory assertion. Kernel-level proof + real-load proof; #44's suspect (exec staging teardown of `ByteBacking::Mnt`) may only drain under real mounted-binary exec.
- **D-07:** **Race pins = deterministic + stress.** Host-deterministic tests for the race LOGIC (sema/waker/rendez are port modules — injectable interleavings on host) PLUS in-kernel stress legs (green-twice-plus on KVM, presence-based markers) proving the fix under real IRQ timing. The stress leg's ancestor is the phase-16 soak burst shape (8-way parallel tcpblast) that made #66 fire 1-in-5.
- **D-08:** **Venue rules are locked here; the concrete per-finding pin mapping is the researcher's; the planner sequences.** No full per-finding pin table was locked at discuss time — nobody had read the fix sites yet; that mapping is research output.

**Adjacent windows scope (FLUSH-04 + folds)**

- **D-09:** **Fold #47, #50, #66 into the CR-07 work item.** #47 is the original multi-waiter RX-wedge window record; #50 is an intermittent early-boot devpipe DataAbort in the same family orbit; #66 is the class live on x86_64. One root-cause pass; all rows close or disposition together (a row may legitimately fall out as a different class during investigation — then it gets its own disposition, still inside this phase).
- **D-10:** **WINDOWS #18 (pgrp lock held across namec walks, open since v1.0 phase 5) is FIXED in 17.** It is the last known live namec/pgrp wedge, same subsystem as #45 (one namec pass), and "real userspace load" means rc pipelines and backgrounded commands — exactly the contender shape that wedges it. Phases 18/19 (listen, exportfs, cpu) will contend it hard; fixing it after they land would be re-debugging under auth load. Fix shape per the ledger's own note: snapshot the union chain under a brief lock, walk unlocked.
- **D-11:** **The three Harvey-userspace rc rows (#46 redirect/fd-reuse corner, #49 argv-stack-underflow residual, #65 Execute errstr masking) are fixed IN-PHASE.** Deliberate user override of the defer recommendation — they're small and rc is the load driver; a flush-out phase that leaves the load driver flaky hasn't flushed. They ride as one rc fix-class commit (Harvey repo; no r9 pin mechanics — their regression proof is boot-gate-shaped: the existing rc-driven sweeps re-greening plus a targeted fixture if cheap).
- **D-12:** **Ledger hygiene = close-what-you-touch.** 17 closes/dispositions the rows it actually touches (#18, #44–#47, #50, #66, plus review findings); the other ~45 historical rows stay untouched, statuses unchanged.

**Ledger closure shape (what "closed" looks like)**

- **D-13:** **The 07/08 REVIEW-DISPOSITION ledgers close in-place.** Hand-edit the rows: Disposition → `fixed` (Source cites the x86_64 pin: host test name or boot profile) or `dispositioned` (Source carries the D-15 rationale). No superseding doc — the ledgers are designed to survive re-reviews; one file per backlog stays the truth.
- **D-14:** **WINDOWS rows close via the ledger's native resolve fields.** `status` → `resolved`, `reason` = the r9 commit SHA + pin name (e.g. `r9 <sha> + boot churn gate`), `resolved_at` stamped — machine-shaped and greppable, matching every prior closure.
- **D-15:** **The disposition bar is a 3-part test.** Every "explicitly dispositioned with rationale" entry carries: WHERE the class lives (frozen aarch64 / superseded fixture / era note), WHY it's not fixed (freeze policy / supersession citation), and WHAT would reopen it (the reopener condition — e.g. "aarch64 reactivation" or "the fixture returns"). One-line reasons were rejected as too weak a future decision record.
- **D-16:** **17-FLUSHOUT.md is the phase's single closure table.** One row per finding/window: finding ID → action taken (fixed / dispositioned / folded) → pin (host test / boot profile / composed gate) → commit (r9 SHA or Harvey SHA) → ledger refs (REVIEW-DISPOSITION row + WINDOWS row). VERIFICATION walks it row by row; the ledgers stay the per-backlog truth, this table is the phase's index over them.

**Carried forward (locked in prior phases — do not re-litigate)**

- push-before-pin; pin advance is its OWN commit + fresh hosted CI; FOD hash re-stales per kernel commit — **re-wire-then-pin after every r9 kernel-byte fix pass**, then the Harvey gates re-green
- Gates split by artifact (14-D-06): kernel-level r9-side cargo xtask, composed-system Harvey-side
- Venue honesty: named venues, no unearned OK, green-twice, presence-based/timing-tolerant assertions, TCG-generous timeouts
- KVM-native by default, TCG fallback documented (X64-05)
- aarch64 frozen at the v2.0 pin — no aarch64 code changes; reference reads of aarch64 files are fine
- **Phase-14 doctrine: panics lie under corruption — memory-side probes only** (no flailing at panic sites; the corruption is upstream)
- Atomic class-sweeps: bug-class fixes land one commit per class, never interleaved with bring-up commits
- Modern-only design theme (Phase 13)
- 16-D-07's standing offer: a r9-side tap boot-test profile may join this phase if a flush-out finding wants a kernel-level tap pin
- The 15-D-01 fix-everything boundary note: Phase-18-owned substrate (devcap `#¤`, `/adm/keys`, keyfs) is NOT pulled forward

### Claude's Discretion

- Exact host-test module placement and names (extend the existing `cfg(test)` homes vs a new tests file — follow the module's existing pattern)
- Boot-profile names/shapes in xtask; the baked-binary choice for the r9-side churn loop (the 14-09 static marker lineage is the obvious candidate)
- The composed churn leg's app shape (flake app-per-gate vs extending an existing gate — planner's call within D-06's tiered shape)
- CR-07 investigation methodology within the memory-side-probe doctrine (how to instrument, which of #66's toolkit pieces to re-run)
- The cap-walk audit's depth on the x64 ECAM path (which of the four WR admission holes get explicit rejects vs structural validation)
- Wave decomposition and ordering (CR-07 early vs late; researcher's dependency map decides)
- 17-FLUSHOUT.md's exact columns beyond D-16's five; where it lives in the phase dir
- The three rc fixes' exact shapes (researcher reads exec.c/plan9.c corners; one class commit)
- CI placement of the new host tests (r9's hosted CI carries them — same job or new, planner decides)

### Deferred Ideas (OUT OF SCOPE)

- **Full WINDOWS.md re-triage** (the ~45 untouched historical rows) — declined for this phase (D-12); if the ledger's noise becomes a real retrieval problem, name a dedicated hygiene pass.
- **Console RX-ring truncation class (#48)** — aarch64 PL011-era record; re-derive on the x64 UART only if a live symptom appears under the churn/stress legs.
- **Era notes (#13/#53 eager exec, #54 nsec/RTC, #55 dead PML4 slot)** — recorded shapes of this era, not bugs; untouched unless a flush-out finding collides.
- **r9-side tap boot-test profiles for their own sake** — join only if a flush-out finding wants a kernel-level tap pin (16-D-07 standing offer).
- **A aarch64-side fix of anything** — frozen at the v2.0 pin; every aarch64 finding is disposition-only with the 3-part rationale.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| FLUSH-01 | Phase-8 CR classes closed with tests or dispositioned: conv_close FIN-drop, permanent-send-error wedge, semacquire wakeup loss, ipifc multicast panic, unbind stale-handle aliasing, tcp_gc aliasing, multi-waiter RX-wake | Findings→pin-venue map below: CR-01..06 host pins already run on this host (`cargo test -p port --lib` = 101 pass, live run this session); CR-03's fixed loop shape verified present on x86_64; CR-07 terrain fully mapped (stack.rs:213-304 + pipe_wake) — the only never-fixed item, root-cause candidates enumerated |
| FLUSH-02 | RFMEM OOM-path page leaks (the 6 WR findings) closed with refcount proofs — leak-repro gates show bounded memory under fork/exec churn | D-02 audit result: the aarch64 WR-03 fix family IS carried in x86_64/src/proc.rs (verified at 4 anchor sites with verbatim quotes); registry pins already exist on x86_64; remaining audit surface = exec.rs install_image path + every remaining OOM release; `heap_stats()` cursor is the kernel-side assertion probe (arena now 32 MiB, residue ~16 KiB/exit measured at 15-03) |
| FLUSH-03 | Phase-7 WR hardening gaps: cap-walk loud-abort holes, WaitSet sizing, used-id slot-liveness | All three verified STILL OPEN with exact sites: WaitSet `[u64; 16]` ×3 modules vs `MAX_PROCS: usize = 512` on BOTH arches (bigger gap than the review's "64"); `next_used` clears `posted[slot]` unconditionally (virtio.rs:755); the four cap-walk admission holes carry verbatim onto the q35 x64 path (extent check pci.rs:979-983, device.length<6 vs devcfg+6 read at 1343, size_bar64 no type nibble, Caps::parse no p&3 / no notify cap_len) |
| FLUSH-04 | Known kernel windows closed or gated: #44 fork+exec arena drain, #45 namec union-create — every fix lands with a pin | #44: ByteBacking::Mnt staging + heap_stats cursor slope named as the gate assertion; #45: walk_one confirmed union-blind (namec.rs:312-318); BONUS: #18's fix shape appears ALREADY IMPLEMENTED in shared namec (per-consult chain_snapshot) — phase verifies + closes; rc rows' exact sites pinned (simple.c doredir, plan9.c Execute/Dup, exec.c pushword/popword) |
</phase_requirements>

## Summary

This phase is a defect-closure sweep over two review ledgers (Phase 7: 8 WR open; Phase 8: 12 fixed + CR-07 skipped) and nine WINDOWS rows (#18, #44–#47, #49, #50, #65, #66), with all kernel fixes landing in the sibling r9 repo and every fix/disposition pinned per the established discipline. The research read every finding body, both disposition ledgers, the fix report (the carry-over audit's diff baseline), all nine WINDOWS JSON rows, and the actual fix sites in `../r9` — then executed the two host-test batteries live to verify the pin venues.

Three scouting facts materially reshape the plan. **First**, the D-02 audit's feared carry-over gap is smaller than the CONTEXT estimated: `x86_64/src/proc.rs` carries the complete aarch64 WR-03 fix family (brk-unwind free-verdict honor at 799-801, bss-sweep record-or-unwind at 2384-2403, the rfork failure pivot at 2274-2286, the exit release pivot at 1325-1327) AND CR-03's register-per-iteration semacquire loop (syscall.rs:901-910), AND host pins for both contracts already exist in x86_64's own test module (`rfmem_shared_pages_lifecycle`, `sem_registry_consumes_and_reregisters`). The FLUSH-02 work is: finish the every-OOM-release audit (exec.rs is the un-read remainder), then let the D-06 leak gates prove it. **Second**, the 12 "already-fixed shared-code" findings' host pins are port-crate tests that RUN TODAY on this host — `cargo test -p port --lib` = 101 passed in 0.04s, including the CR-01..06/WR devip pins; D-01's pin-first posture is mostly a re-cite + boot-gate re-green exercise, not new pin construction. **Third**, two facts got stale in the sources: WaitSet is 16 vs **512** now (not 64 — both arches bumped MAX_PROCS for the tier sweeps), and WINDOWS #18's fix shape (snapshot-then-walk) appears already implemented in shared `namec.rs` — the row likely closes as verify+pin rather than fix.

The genuinely new engineering is concentrated in four places: **CR-07** (never fixed; full terrain mapped — NET_WAITERS drain-all + pipe_wake takes PROC_TABLE in IRQ context; three recorded root-cause candidates; #66/#47/#50 fold in), the **Phase-7 hardening trio** (three WaitSet arrays to resize + louden; `posted[]` liveness check; the four cap-walk admission holes on the q35 path), the **D-06 tiered leak gates** (r9-side churn boot profile asserting `heap_stats` cursor slope + Harvey-side /bin/cat composed churn), and the **three rc corners** (exact sites: simple.c `doredir` unchecked `Dup`, plan9.c `Execute` last-candidate errstr, exec.c `pushword`/`popword` argv-stack invariant).

**Primary recommendation:** sequence as (1) audit-completion + pins for the already-carried fixes (FLUSH-01/02 verification wave — cheap, mostly re-cites + the exec.rs OOM audit), (2) the Phase-7 hardening class-sweep (FLUSH-03 — self-contained, host-testable, one commit per class), (3) CR-07 root-cause + deterministic/stress pins (the centerpiece, needs the #66 toolkit), (4) #44/#45/#18 + the rc class commit (FLUSH-04 + D-10/D-11), each r9 fix pass followed by FOD re-wire-then-pin + gate re-green, closing ledgers/FLUSHOUT continuously as rows land.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CR-07 RX-wake wedge fix | r9 `port/src/devip/stack.rs` + `waker.rs` (shared kernel) | r9 `x86_64/src/proc.rs` (pipe_wake), xtask stress profile | The waiter registry + wake bridge are arch-neutral port code; the wake hook and IRQ timing are arch-side; the pin's stress leg is gate machinery |
| RFMEM/OOM leak family (FLUSH-02) | r9 `x86_64/src/proc.rs` (per-arch) | r9 `x86_64/src/exec.rs` (staging), `port/src/allocator.rs` (heap_stats probe) | The mapping/refcount machinery is per-arch by design (aarch64 frozen as reference); the arena probe is shared |
| Phase-7 hardening trio (FLUSH-03) | r9 `port/src/{netif,virtio}.rs` (shared) | r9 `x86_64/src/platform/q35/pci.rs` (bind-time extent checks) | WaitSet ×2 + used-id retire live in port; the third WaitSet copy is devip/conv.rs; cap-walk bind-time rejects are q35-side consumers of port's parsed caps |
| devip CR-01..06 re-pins (FLUSH-01) | r9 `port/src/devip_tests.rs` (host) | Harvey x64 net gates (live in-kernel) | The pins are port-crate host tests (verified passing); the boot gates re-prove under real sockets/IRQs |
| #44 arena drain (FLUSH-04) | r9 exec staging (`ByteBacking::Mnt`) | Harvey composed churn gate (D-06b) | Kernel-level proof r9-side; real-load proof Harvey-side per 14-D-06 split |
| #45 union-create + #18 pgrp lock | r9 `port/src/namec.rs` | r9 `x86_64/src/syscall.rs` (pgrp lock sites) | Both are shared namec/pgrp machinery; x86_64 syscall.rs holds the 4 remaining pgrp-lock sites to audit (three `pg.lock` spellings + one `pgrp.lock` at :1159) |
| rc rows #46/#49/#65 (D-11) | Harvey `sys/src/cmd/rc/` (userspace) | Harvey boot gates (proof) | Pure Harvey-repo C fixes; no r9 pin mechanics; proof is rc-driven sweeps re-greening |
| Ledger closure (D-13/14/16) | Harvey `.planning/` ledgers | 17-FLUSHOUT.md (phase dir) | The ledgers are the per-backlog truth; FLUSHOUT is the phase index |
| FOD re-wire + pin advance | Harvey `flake.lock` + r9 flake FOD | r9 `origin/main` (push-before-pin) | The two-repo composition: every kernel-byte commit re-stales the FOD hash |

## Standard Stack

No external packages are installed by this phase (Rust core/alloc only in r9; Harvey C under the existing Plan 9 toolchain). The "stack" is the repo-internal machinery:

### Core (the machinery this phase rides)

| Asset | Location | Purpose | Why Standard |
|-------|----------|---------|--------------|
| Host test homes (port) | `../r9/port/src/{devip_tests,virtio_tests,devether_tests,rx_chain_tests,allocator_tests}.rs` [VERIFIED: ../r9/port/src listing] | `#[cfg(test)]` pin modules — the D-05 host-pin homes | 5 established test modules; `cargo test -p port --lib` = 101 pass live on this host |
| Host test home (x86_64) | `../r9/x86_64/src/proc.rs:2506` `mod tests` [VERIFIED: ../r9/x86_64/src/proc.rs:2506] | per-arch pins (`rfmem_shared_pages_lifecycle` :2641, `sem_registry_consumes_and_reregisters` :2662) | The aarch64 `rfmem_tests`/`sem_tests` precedent, already partially mirrored |
| Kernel-side gate driver precedent | `../r9/x86_64/src/platform/q35/netgate.rs` [VERIFIED: ../r9/x86_64/src/platform/q35/netgate.rs:156-227] | boot-context in-kernel test stages with masked posts + terminal markers | The 13-05 IRQ-path discipline shape |
| Boot profile configs | `../r9/x86_64/lib/config_{default,q35,q35exec,q35lowmcfg,q35net,q35shell}.toml` [VERIFIED: ../r9/x86_64/lib listing] | xtask `boot-test --config <name>` profiles | New profiles (churn/stress) join this family |
| Leak probe | `../r9/x86_64/src/allocator.rs:23` `heap_stats()` → `port/src/allocator.rs:586` `arena_stats()` [VERIFIED: ../r9/x86_64/src/allocator.rs:19-38] | (bump-cursor bytes, arena size) — the memory-side assertion input | "a cursor that climbs with every proc while live allocations stay flat names a permanent-burn bug's slope" (allocator.rs:577-580, verbatim) |
| Composed gate family | Harvey `flake.nix` apps `boot-r9-{x64,smoke-x64,tier1-x64,net-smoke-x64,services-x64,net-soak-x64}` [VERIFIED: /home/req/code/harvey/flake.nix:165-317] | the gates that must re-green after every fix pass | app-per-gate pattern exists; the churn leg (D-06b) joins it |
| Marker binary lineage | `build/x86_64/smokesleep.c` (the 14-09 static marker) [VERIFIED: build/x86_64/mkimage.sh:206-212] | the r9-side churn loop's exec payload candidate | mkimage already asserts `amd64/init` IS the marker in the leg-1 image |
| Stress shape ancestor | `build/x86_64/tcpblast.c` + `netsoak-r9.expect` + 16's burst leg (8-way) [VERIFIED: build/x86_64 listing] | the CR-07 stress pin re-runs this shape | #66 fired 1-in-5 under exactly this load |

### Supporting

| Asset | Location | When to Use |
|-------|----------|-------------|
| `--target` override battery | `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` | REQUIRED on linux — 60 pass live; plain `--bins` fails with 1330 E0463 errors (per-package-target `default-target = "x86_64-unknown-none"` has no std) [VERIFIED: live run + ../r9/x86_64/Cargo.toml:1-7] |
| oom high-water print | `../r9/x86_64/src/runtime.rs:24-31` — `panic!("oom: {layout:?} arena used {used:#x} of {total:#x}")` | the churn gate's failure observability |
| r9 hosted CI | `../r9/.github/workflows/xtask.yml` — lint job (check :19 + clippy :20), `tests` job running `cargo xtask test --verbose` (:28 — TestStep runs both host batteries per xtask main.rs:2834-2853), three per-arch build jobs, a TCG boot-test job [VERIFIED: live read] | host tests ARE in CI (the tests job rides every push) — the discretion item "CI placement of the new host tests" is already discharged by the existing tests job; new pins ride it with no workflow edit |

## Package Legitimacy Audit

**None.** This phase installs no external packages (no crates.io additions, no npm/pypi). All work is in-repo Rust (core/alloc) and Harvey C. The gate does not apply.

## Architecture Patterns

### System Architecture Diagram (the closure pipeline)

```
  REVIEW-DISPOSITION ledgers (07: 13 open rows; 08: 12 fixed + CR-07 skipped)
  WINDOWS.md JSON rows (#18,#44,#45,#46,#47,#49,#50,#65,#66)
        |
        v
  [Research-audited fix sites] --(D-01 pin-first)--> existing host pins re-run
        |                                        (port --lib: 101 pass = the 12 re-proved)
        |--(D-02 audit+port)--> x86_64/proc.rs OOM releases (family verified carried;
        |                       exec.rs remainder audited) --> D-06 leak gates
        |--(D-03 CR-07)------> root-cause (memory-side probes, #66 toolkit)
        |                       deterministic host pin + KVM stress leg (green-twice+)
        |--(D-04 P7 trio)----> WaitSet 512 + loud full-arm | posted[] liveness |
        |                       q35 cap-walk loud aborts (4 holes)
        |--(D-10/D-11)-------> namec #18 verify+#45 fix | rc class commit (Harvey)
        v
  every r9 fix pass: class-commit -> push -> FOD re-wire -> pin advance (own commit)
        -> Harvey x64 gate family re-greens
        |
        v
  Ledger closure (D-13/14/15): rows hand-edited in-place; WINDOWS JSON
  status=fixed (the native closure token; D-14's 'resolved' maps onto it)
  + reason="r9 <sha> + <pin>"; 3-part dispositions
        |
        v
  17-FLUSHOUT.md (D-16): one row per finding -> action -> pin -> commit -> ledger refs
        |
        v
  VERIFICATION walks the table row by row
```

### Findings → Fix-Site → Pin-Venue Map (the D-08 research output)

| Finding | Fix Site (read this session) | x86_64 Status | Cheapest Pin That Can Fail (D-05) |
|---------|------------------------------|---------------|-----------------------------------|
| CR-01 FIN-drop | `port/src/devip/{conv,stack}.rs` linger list (r9 bd4db36) | live shared code | existing host pin (devip_tests) — runs today; ledger Source re-cites it + the x64 net gates |
| CR-02 send-error wedge | `port/src/devip/proto.rs` (r9 904af8f) | live | existing host pins (udp/icmp/icmpv6 oversize+unaddressable) |
| CR-03 semacquire wakeup loss | `x86_64/src/syscall.rs:901-910` loop; registry `x86_64/src/proc.rs:845-856` | **FIX SHAPE PRESENT** (verified read, quote below) | D-07 deterministic: extend x86_64 proc tests (pin `sem_registry_consumes_and_reregisters` exists at :2662) + in-kernel contention leg under RFMEM churn |
| CR-04 multicast panic | `port/src/devip/ipifc.rs` (r9 ac6370c) | live | existing host pin |
| CR-05 unbind aliasing | `port/src/devip/ipifc.rs` (r9 0f602d8) | live | existing host pin |
| CR-06 tcp_gc aliasing | `port/src/devip/{proto,conv}.rs` (r9 79517b1) | live | existing host pin |
| **CR-07 multi-waiter RX wedge** | `port/src/devip/stack.rs:213-304`; wake hook `x86_64/src/proc.rs:1973-1981` | **NEVER FIXED — centerpiece** | D-07 both: host-deterministic (stack/waker modules are host-compilable — devip_tests already pins the waker bridge) + KVM stress boot leg (#66 shape) |
| P8 WR-01/02/04/05/06 | `port/src/devip/{proto,ipifc,route,devip}.rs` (r9 SHAs in ledger) | live | existing host pins (all RED-proven at Phase 8) |
| P8 WR-03 RFMEM leaks | `x86_64/src/proc.rs:799-801, 2384-2403, 2274-2286, 1325-1327` | **FIX FAMILY CARRIED** (verified, quotes below) | audit remainder (exec.rs) + D-06 tiered leak gates |
| P7 WR-01 extent holes | consumer `q35/pci.rs:979-983` | **HOLE CARRIES** (quote below) | host test: synthetic lying-cap PciCfg in virtio_tests + bind-time reject in q35 (netgate/host-test precedent); x64 net gates re-green |
| P7 WR-02 devcfg length | `q35/pci.rs:984` check vs `:1343` read | **HOLE CARRIES** | same synthetic-cap host test |
| P7 WR-03 BAR type/window | `q35/pci.rs:820-832` size_bar64; `:804-814` bar_assignment | **PARTIAL**: window END checked (better than aarch64); type nibble + natural alignment absent | host test on bar_assignment (already `cfg(any(test, dev_ether))` — testable today); type-nibble reject is bind-time |
| P7 WR-04 cap alignment | `port/src/virtio.rs:139-175` `Caps::parse` | **HOLE CARRIES** (shared) | host test beside existing virtio_tests negatives |
| P7 WR-05 WaitSet sizing | `netif.rs:335`, `stack.rs:179`, `conv.rs:125` — all `pids: [u64; 16]` | **OPEN — gap is 16 vs 512** | host tests at all three homes (netif/devip test modules exist) |
| P7 WR-06 used-id liveness | `port/src/virtio.rs:730-757` | **OPEN** (quote below) | host test in virtio_tests "beside the existing out-of-range/odd-id negatives" (the finding's own suggestion) |
| P7 WR-07 leading zeros | aarch64-era `net-sweep.rc` | superseded | 3-part disposition |
| P7 WR-08 DTB trigger flags | aarch64 gic files | frozen | 3-part disposition |
| #44 arena drain | `x86_64/src/exec.rs` staging (`ByteBacking::Mnt { file: Arc<Vec<u8>> }`, exec.rs/proc.rs quotes below) | open (residue measured ~16 KiB/exit at 15-03) | D-06 both tiers: r9 churn profile (smokesleep + heap_stats bound) + Harvey /bin/cat ×~1400 composed leg |
| #45 union-create | `port/src/namec.rs:312-318` walk_one | open (quote below) | host test if namec machinery reachable (devroot_rawimg/devenv test precedent) else boot-gate rc `>` write |
| #18 pgrp lock across walks | `port/src/namec.rs:130-212` | **APPEARS ALREADY FIXED** (per-consult snapshot, quote below) | audit the 4 remaining pgrp-lock sites (`x86_64/src/syscall.rs:527,599,716` as `pg.lock` + `:1159` as `proc.pgrp.lock`) + contention boot gate (rc pipelines) |
| #46 rc redirect corner | `sys/src/cmd/rc/simple.c:77-92` doredir + `plan9.c` Dup | open | rc-driven sweeps re-green + cheap targeted fixture |
| #49 rc argv underflow | `sys/src/cmd/rc/exec.c:41-60` + `havefork.c:136` (06-07 fix orbit) | open (HVF-timing-exposed; CI venue is TCG/KVM) | boot-gate sweeps; repro needs the fast-fork backquote-in-loop shape |
| #50 devpipe early-boot DataAbort | `port/src/devpipe.rs` read loop (register-under-lock + drop + block — verified shape) | folded with CR-07 (D-09) | rides CR-07's disposition |
| #65 Execute errstr masking | `sys/src/cmd/rc/plan9.c:358-399` | open (quote below) | boot-gate rc-driven; targeted fixture cheap |
| #66 x64 triple fault | 16 deferred-items (toolkit + stress shape) | folded with CR-07 (D-09) | the stress leg IS the #66 shape (rounds=1/burstn=8, `-d int,guest_errors`) |

### Pattern 1: The block/wake discipline (what every CR-class fix must preserve)

**What:** register → drop guard → block, with re-register before every re-block.
**When to use:** any new waiter machinery in devip/devpipe/sem (CR-07's fix will touch this).
**Example** — the x86_64 semacquire loop, verified present [VERIFIED: ../r9/x86_64/src/syscall.rs:901-910]:
```rust
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
The devip block loops use the same shape: `t.table_mut(...)[id].waiters.register(pid); stack::register_net_waiter(pid); ... drive_stack(); (hks.block)();` [VERIFIED: ../r9/port/src/devip/proto.rs:739-755].

### Pattern 2: The leak-gate assertion (memory-side probe, per Phase-14 doctrine)

**What:** assert the bump-cursor slope, not absolute free bytes.
**Example** [VERIFIED: ../r9/port/src/allocator.rs:574-583, verbatim]:
```rust
// Diagnostic surface (15-03, the services-venue oom hunt): read-only
// arena stats off the live QuickFit.  tail.cursor is the bump
// high-water mark (bytes carved from the arena's front, irreversibly
// for blocks still allocated AND for the never-reused prefix holes);
// arena.len() is the fixed GLOBAL_HEAP_SIZE.  The difference is NOT
// "free" in a bump sense (freed blocks recycle through the quick/misc
// lists, and the cursor only ever grows) — the pair still localizes a
// leak: a cursor that climbs with every proc while live allocations
// stay flat names a permanent-burn bug's slope.
```
`heap_stats()` is kernel-only: `#[cfg(not(test))] pub(crate) use global::heap_stats;` [VERIFIED: ../r9/x86_64/src/allocator.rs:38] — the churn boot profile prints it (or a derived marker) on serial; the expect driver asserts the bound. The oom handler already prints it: `panic!("oom: {layout:?} arena used {used:#x} of {total:#x}")` [VERIFIED: ../r9/x86_64/src/runtime.rs:29].

### Pattern 3: Ledger closure shapes (D-13/14)

- 08 ledger row: `| CR-01 | critical | fixed | 08-REVIEW-FIX.md — socket lingers... (r9 bd4db36 + pin 7300113) |` → hand-edit Disposition/Source to cite the x86_64 pin [VERIFIED: 08-REVIEW-DISPOSITION.md:25].
- 07 ledger is YAML-frontmatter titles + table; 13 rows `open` [VERIFIED: 07-REVIEW-DISPOSITION.md:62-84].
- WINDOWS JSON row fields: `"status": "open", "reason": "", "resolved_at": null` → `"status": "fixed"` + `reason` + `resolved_at` — the ledger's native closure token at HEAD is `fixed` (live count: 7 fixed / 55 open / 3 waived / 0 `resolved`; every prior closure reads `fixed`, e.g. WINDOWS.md:184, 262, 639, 743, 808), so D-14's literal token `resolved` maps onto the native `fixed` (the field names `reason`/`resolved_at` are unchanged) [VERIFIED: live grep + .planning/WINDOWS.md:639-644 (a closed row's shape) and :646-662 (row #44 open shape)].

### Anti-Patterns to Avoid

- **Re-auditing fixed code instead of pinning it** (D-01: "The pins ARE the audit") — a failing pin reopens the finding; a passing pin closes it.
- **One big pin at the end** — every r9 kernel-byte commit re-stales the FOD hash; sequence class-commits with re-wire-then-pin after each pass.
- **Instrumenting the panic side of a race** — observer effects healed #66 (debugcon runs were green); instrument the memory side.
- **Timing/interleaving assertions in race pins** — presence-based markers only (the locked venue-honesty rules).
- **Sizing WaitSet to "64"** — the review's number is stale; both arches are 512 today (quotes below).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Leak detection in gates | custom accounting counters | `heap_stats()`/`arena_stats()` (port/allocator.rs:586) | already the 15-03 probe; single-CPU read-only; kernel-only gating understood |
| Host pin harness | new test framework | existing `#[cfg(test)]` homes (5 port modules + x86_64 `mod tests`) | publish-once constraints and RED-proof discipline already documented in them |
| Race stress fixture | new load generator | the 16 soak burst shape (8-way tcpblast lineage) + `/bin/cat` spin | #66 fired under exactly this shape; reusing it makes the regression pin commensurate |
| Marker vocabulary | new marker strings | the `netgate:`/`tcpblast: done`/tier-marker families | expect drivers and post-passes already parse these |
| Churn exec payload | new test binary | the 14-09 smokesleep marker lineage | mkimage already stages/asserts it; deterministic |
| Expect driving | new driver framework | the smoke/netsoak/services expect drivers + fake-marker-stream self-test pattern (16-05) | proven against silent no-match and pty canonical-mode pitfalls |

**Key insight:** every piece of pin machinery this phase needs already exists in one of the two repos; the phase's novelty is in the fixes and the root-cause work, not the gates.

## Common Pitfalls

### Pitfall 1: The x86_64 host battery needs the `--target` override
**What goes wrong:** `cargo test -p x86_64 --bins` fails with 1330 errors (`E0463 can't find crate for std`).
**Why:** `x86_64/Cargo.toml` uses `cargo-features = ["per-package-target"]` with `default-target = "x86_64-unknown-none"` [VERIFIED: ../r9/x86_64/Cargo.toml:1-7] — a bare no-std target with no std/test.
**How to avoid:** `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` — 60 passed live on this host (the aarch64 precedent ran `--target aarch64-apple-darwin` on darwin).
**Warning signs:** E0463/E0433 error showers in CI or verify legs.

### Pitfall 2: WaitSet sizing against a stale constant
**What goes wrong:** fixing WR-05 to `[u64; 64]` per the 07-REVIEW text leaves the hole open.
**Why:** the review predates the tier sweeps: `pub const MAX_PROCS: usize = 512;` on BOTH x86_64 (proc.rs:111) and aarch64 (proc.rs:73) [VERIFIED, quoted in Code Examples]. Also: the port crate CANNOT import the arch const — the capacity must be a port-level constant (three modules each define their own `WaitSet`-shape today: netif.rs:334, stack.rs:172-179 area, conv.rs:120-125).
**How to avoid:** one shared port-side const sized to the real table (512), all three arrays + all `take(&mut [u64; N])` buffers moved to it, and the full arm made loud (count+report, the IN-07 finding's suggestion).
**Warning signs:** any new `[u64; 16]` or `&mut [u64; 16]` signature.

### Pitfall 3: FOD re-stale per kernel commit
**What goes wrong:** bundling several fix classes then pinning once — the FOD hash discipline breaks and the gates ride a mix.
**Why:** locked carried-forward rule (re-wire-then-pin after every r9 kernel-byte fix pass).
**How to avoid:** class-commit → push → FOD re-wire → pin advance (own commit) → Harvey gates re-green, per class. The current pin is r9 `208ceba` == origin/main == local HEAD (verified this session), so the phase starts from a clean pushed base.

### Pitfall 4: Observer effects in the CR-07 hunt
**What goes wrong:** instrumented kernels run green (both debugcon diag runs of #66 were green; the failure is 1-in-5 uninstrumented).
**Why:** timing-sensitive race; instrumentation changes IRQ/sched timing.
**How to avoid:** memory-side probes only (Phase-14 doctrine); the stress pin runs the UNINSTRUMENTED pinned kernel with `-d int,guest_errors` exception-cascade capture; green-twice-plus with the planner stating N.
**Warning signs:** a fix "proven" only under instrumentation.

### Pitfall 5: The cap-walk audit re-runs the whole gate battery
**What goes wrong:** discovery-path edits (q35/pci.rs) red every net gate that boots through it.
**Why:** D-04's own reversibility note — costly.
**How to avoid:** land the loud-abort rejects as ONE class commit after the host tests are RED-proven; expect the full x64 battery re-run as the gate step (that's the design, not a failure).

### Pitfall 6: rc fixes are not r9 pins
**What goes wrong:** inventing host unit tests for rc, or r9 pin mechanics for the D-11 rows.
**Why:** locked D-11 — proof is boot-gate-shaped (rc-driven sweeps re-green + cheap fixture); rc is Plan 9 C (no host test harness).
**How to avoid:** one fix-class commit in `sys/src/cmd/rc/`, style(6)-conformant (tabs, `/* */`, `exits`), verified by the existing tier1/smoke sweeps + a targeted fixture only if the corner needs one.

### Pitfall 7: Existing-gate semantics must not change
**What goes wrong:** "improving" smoke/tier/netsoak expect semantics while adding legs.
**Why:** locked watch-out — Phase 16's gates stay green unperturbed; new profiles and host tests are ADDITIVE.
**How to avoid:** new TOML profiles + new expect drivers/apps; no edits to existing profile semantics.

### Pitfall 8: The publish-once netif test constraint
**What goes wrong:** a second full-path netif drive test trips `publish-once` asserts.
**Why:** `devether_tests` consumes the single publish; documented in its header (07-REVIEW incremental notes).
**How to avoid:** new WaitSet tests target the struct directly (pure logic), not the published-device path.

### Pitfall 9: Parallel-boot hostfwd collisions
**What:** two simultaneous q35 boots collide on `127.0.0.1:17010/5670` [VERIFIED: config_q35net.toml comment "Parallel-boot constraint"] — sequence gate runs; matters when running the new churn/stress legs alongside existing gates.

### Pitfall 10: IN rows are NOT in phase scope
**What:** 07's IN-01..05 and 08's IN-01..07 stay `open` (D-12 close-what-you-touch; FLUSH requirements name only CR/WR classes). Don't sweep them up. (08's IN rows read "out of fix scope (critical_warning); re-run with --all to include" [VERIFIED: 08-REVIEW-DISPOSITION.md:38-44].)

## Code Examples

Verified, load-bearing excerpts (verbatim, this session):

### The WaitSet full-arm hole (P7 WR-05 — three copies)
```rust
// Source: [VERIFIED: ../r9/port/src/netif.rs:330-350] (identical shape at stack.rs:172-194, conv.rs:120-136)
/// A fixed-capacity pid set (one waiter per proc — the proc table is
/// MAX_PROCS=16, so 16 slots can never fill).  pid 0 marks an empty
/// slot (pids start at 1).  No allocation ANYWHERE — the IRQ-side drain
/// moves the pids into a caller's stack buffer.
struct WaitSet {
    pids: [u64; 16],
}
...
        // The full case is impossible (one waiter per proc, ≤ 16 procs).
```
vs the real table [VERIFIED: ../r9/x86_64/src/proc.rs:111 and ../r9/aarch64/src/proc.rs:73]:
```rust
pub const MAX_PROCS: usize = 512;
```

### The used-id liveness hole (P7 WR-06)
```rust
// Source: [VERIFIED: ../r9/port/src/virtio.rs:742-756]
        if id >= u32::from(self.qsize) || id & 1 != 0 {
            return Some(Err("virtio: used-ring id out of range (device fault)"));
        }
        ...
        // Slot recovery (ethervirtio.c): the used id is the chain head.
        let slot = ((id & u32::from(self.qmask())) >> 1) as usize;
        self.posted[slot] = false;
```
The missing check (per the finding): `if !self.posted[slot] { return Some(Err("virtio: used-ring id for an unposted slot (device fault)")); }` before the clear.

### The cap-walk admission holes on the x64 path (P7 WR-01/02/03/04)
```rust
// Source: [VERIFIED: ../r9/x86_64/src/platform/q35/pci.rs:976-986]
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
Holes: `offset == bar_size, length == 0` passes (not `>`); no minimum extents (common ≥ 0x40, isr ≥ 1, notify_off × multiplier ≤ notify length); `device.length < 6` but the IRQ handler reads the link status le16 at offset 6: `let status = mmio_read16(state.devcfg + 6);` [VERIFIED: ../r9/x86_64/src/platform/q35/pci.rs:1343] — needs 8. And `size_bar64` writes all-ones to BAR+0/+4 with no type-nibble validation [VERIFIED: ../r9/x86_64/src/platform/q35/pci.rs:820-832]; `bar_assignment` checks the window END (`base.checked_add(size).is_some_and(|end| end <= win.1)`) but not natural alignment [VERIFIED: ../r9/x86_64/src/platform/q35/pci.rs:804-814]. Shared `Caps::parse` has no `p & 3` alignment check and the notify arm checks `p + CAP_NOTIFY_LEN > cfg.len()` but not `cap_len >= CAP_NOTIFY_LEN` [VERIFIED: ../r9/port/src/virtio.rs:139-165].

### The RFMEM fix family, already carried on x86_64 (D-02 audit result)
```rust
// Source: [VERIFIED: ../r9/x86_64/src/proc.rs:787-804] — brk unwind honors the verdict
        // Unwind the completed pages: unmap from every member and
        // release per member — the last release frees.
        for &(done_va, done_pa) in &done {
            for &m in &members {
                ...
                if release_user_page(done_pa) {
                    free_phys_page(done_pa);
                }
            }
        }
```
```rust
// Source: [VERIFIED: ../r9/x86_64/src/proc.rs:2394-2403] — bss-sweep child-map failure unmaps the parent's untracked mapping
                let created = match pml4_map_user(child.pml4_pa, va, pa, SegKind::Bss.pte()) {
                    Ok(c) => c,
                    Err(_) => {
                        // The parent's mapping is UNTRACKED (the record
                        // push below never ran) — unmap it and free the
                        // page, mirroring the brk arm's failure hygiene.
                        pml4_unmap_user(parent.pml4_pa, va);
                        free_phys_page(pa);
                        return Err("rfork: sweep child map failed");
                    }
                };
```
Also verified: the rfork failure pivot (`x86_64/src/proc.rs:2274-2286`: Moribund + release-pivot walk + `free_root_table_and_kstack`) and the exit pivot (`free_user_mappings`, :1317-1339, `if release_user_page(rec.phys as u64) { free_phys_page(...) }`).

### #18's fix shape, already in shared namec
```rust
// Source: [VERIFIED: ../r9/port/src/namec.rs:130-138]
/// LOCKING: the mount-table consults take the pgrp lock PER
/// CONSULT (snapshot the union chain via chain_snapshot, drop the lock,
/// then walk) — the lock is NEVER held across a device walk.  A walk into
/// a mounted tree is a devmnt Twalk RPC that BLOCKS mid-syscall (the pipe
/// read schedules the server proc); holding the pgrp lock across it
/// deadlocks the moment any 9P server proc sharing this pgrp (a fork()
/// child without RFNAMEG) takes the lock while serving.
```
Implemented at :206-212 (`let guard = pg.lock(&node); guard.chain_snapshot(&c.path)` then lock-free walks). Remaining audit: the four pgrp-lock sites in `x86_64/src/syscall.rs` (:527, :599, :716 are `pg.lock`; :1159 is `proc.pgrp.lock` — `grep -nE '(pg|pgrp)\.lock'` counts 4 at HEAD; a bare `pg\.lock` grep finds only 3) [VERIFIED: grep].

### #45's site — walk_one is union-blind
```rust
// Source: [VERIFIED: ../r9/port/src/namec.rs:312-318]
/// A single-component walk from an already-resolved chan through the
/// device dispatch (sys_create's final-component probe).  No
/// mount-point consultation — the caller's parent already resolved with
/// the union semantics it needed.
pub fn walk_one(c: &Chan, name: &[u8]) -> Result<Chan> {
    dev_walk(c, name)
}
```
With `create_chan`'s `_ => return Err("namec: device has no create")` at :390 for the union-head device [VERIFIED].

### CR-07 terrain — the multi-waiter wake path
```rust
// Source: [VERIFIED: ../r9/port/src/devip/stack.rs:294-304] (IRQ-side; tick_check at :265-279 is the timer twin)
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
The hook on x86_64 [VERIFIED: ../r9/x86_64/src/proc.rs:1973-1981]:
```rust
pub fn pipe_wake(pids: &[u64]) {
    let node = LockNode::new();
    let mut table = PROC_TABLE.lock(&node);
    for p in table.procs.iter_mut().flatten() {
        if p.state == ProcState::Blocked && pids.contains(&p.pid) {
            p.state = ProcState::Ready;
        }
    }
}
```
Note both wake entries drain ALL waiters (`take`) and the hook walks a 512-slot PROC_TABLE per wake — the thundering-herd + IRQ-context-lock terrain the three recorded candidates live in (see Open Questions). Registered per-syscall from the proto.rs block loops (8 call sites, [VERIFIED: grep — proto.rs:744, 833, 1170, 1388, 1777, 1859, 2184, 2502]).

### #44's suspect — the exec staging
```rust
// Source: [VERIFIED: ../r9/x86_64/src/proc.rs:260-265]
    /// devmnt binaries (the /bin union crossing): the WHOLE file, read
    /// eagerly at exec through the 9P client — syscall context can
    /// block on RPCs; a fault-context pager NEVER can, so the bytes are
    /// kernel-owned from exec on.  Arc-shared across rfork (the
    /// source-handle clone).
    Mnt { file: Arc<Vec<u8>> },
```
`exec_replace` orders parse → `free_user_mappings()` (drops `text_source`, releasing the Arc) → seed [VERIFIED: ../r9/x86_64/src/exec.rs:384-436]. The 15-03 measured facts: heap is 32 MiB; the services venue exhausted 4 MiB with "exec staging + resident daemons + a ~16 KiB per-exit residue" [CITED: .planning/STATE.md:158].

### The rc corners (D-11)
```c
/* Source: [VERIFIED: sys/src/cmd/rc/plan9.c:397-399] — #65: only the LAST candidate's errstr survives */
	if(errstr[0] == '\0')		/* pick up any werrstr "too long"s */
		rerrstr(errstr, sizeof errstr);
	pfmt(err, "%s: %s\n", argv[1], errstr);
```
```c
/* Source: [VERIFIED: sys/src/cmd/rc/simple.c:83-91] — #46: Dup return unchecked in both arms */
		case ROPEN:
			if(rp->from!=rp->to){
				Dup(rp->from, rp->to);
				close(rp->from);
			}
			break;
		case RDUP:
			Dup(rp->from, rp->to);
			break;
```
(#49's orbit: `panic("pushword but no argv!", 0)` at exec.c:46 / popword at :56; the 06-07 Xbackq fix's comment at havefork.c:136 [VERIFIED: grep + reads]. `Dup` itself is `plan9.c` `return dup(a, b);`.)

## State of the Art (repo-internal drift table)

| Old Fact (review/ledger era) | Current Fact (verified today) | Impact |
|------------------------------|-------------------------------|--------|
| MAX_PROCS = 64 (07-REVIEW WR-05) | `MAX_PROCS: usize = 512` both arches | WaitSet sizing target is 512, port-level const |
| kernel arena 4 MiB (WINDOWS #44, aarch64-era) | 32 MiB since 15-03, residue ~16 KiB/exit measured | churn gate bounds must scale; cursor-slope assertion, not absolute |
| #18 "pgrp lock held across namec walks" (v1.0 phase 5) | per-consult `chain_snapshot` + lock-free walks in shared namec | row likely closes as verify + contention pin, not a fix |
| Phase-8 pins = aarch64 boot gates + port host tests | port host pins still green on this host (101/101) | D-01 pin-first is mostly re-cite + re-green |
| Harvey pin r9 d1c4849 (16-04) | flake.lock r9 rev = `208ceba775577cbae194f5a47dab61930abee743` == r9 HEAD == origin/main | clean pushed base for fix passes |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The WR-03 fix family carry-over is COMPLETE on x86_64 beyond the four verified anchors — i.e. the remaining `exec.rs` install_image OOM releases and any alloc site I did not enumerate are also clean | FLUSH-02 / D-02 map | Low: D-02's executor audit + D-06 gates catch any miss; the audit is a locked task regardless |
| A2 | The existing port host pins (101) are an acceptable x86_64 re-proof for the 12 shared-code findings, with the x64 net-gate family as the in-kernel leg | FLUSH-01 map | Medium: if VERIFICATION demands arch-keyed boot pins per finding, more boot profiles are needed — planner should confirm the bar with the D-13 Source-cell shape (host test name suffices per the CONTEXT wording "host test name or boot profile") |
| A3 | #18 is genuinely fixed (not just commented) — the 4 syscall.rs pgrp-lock sites also follow snapshot-then-drop | FLUSH-04 map | Low: the audit is cheap and the row disposition shape covers either outcome |
| A4 | smoltcp 0.14 semantics quoted in the 08 review (BufferFull permanence, check_ip_addrs panic, socket_set remove behavior) still hold for the pinned smoltcp version | CR-02/04 context | Low: fixes already landed and pinned; nothing this phase changes rides on re-deriving them [CITED: 08-REVIEW.md, verified against crate source at review time] |
| A5 | namec/Chan machinery is host-test-reachable for a #45 pin (devroot_rawimg_test/devenv_test precedent extends to walk_one's union consult) | Pin map #45 | Low: fallback is the boot-gate rc `>` write (the original repro shape) |
| A6 | #49 (rc argv underflow) is reproducible on the KVM/TCG linux venue | rc rows | Medium: the historical fires were HVF-timing-only (darwin); the fix may have to land on invariant-audit grounds with sweep re-green as the pin — record as a 3-part-adjacent note if unreproducible |
| A7 | `cargo test -p port` (full, non---lib) remains green alongside `--lib` 101 | Environment | Low: the lib battery is the pin home; the full battery adds doc tests |

## Open Questions (RESOLVED)

All four resolved at planning (2026-11-12); each resolution names the plan task that adopted it. OQ-1 is resolved as adopted-by-task only — the investigation mandate is planned; the causal answer itself is execution output per locked D-03, never answered at plan time.

1. **CR-07 root cause — which candidate?**
   - What we know: three recorded candidates (the multi-pid wake hook path; a woken waiter's re-probe driving BOTH cores (`drive_stack` iterates `[lb, eth]` — verified at stack.rs:479-480) and consuming the ARP reply before the ping reader re-probes; IRQ-context BusyCell contention). #66's x86_64 form is a triple fault with NO trap dump (IDT delivery itself failed) preceded by an empty-status exit cascade.
   - What's unclear: which candidate is causal; whether #47 (aarch64 wedge), #50 (devpipe DataAbort), and #66 (x64 triple fault) are one class or two (D-09 allows a row to fall out as a different class).
   - Recommendation: root-cause under the memory-side-probe doctrine with the #66 toolkit (16 deferred-items records the debugcon instrumentation set + the recommended `rounds=1/burstn=8` + `-d int,guest_errors` stress loop); planner sequences it early enough that the stress pin can gate the fix (D-03), with wave ordering per the dependency map (discretion).
   - Resolution (RESOLVED as adopted-by-task): the root-cause mandate is adopted by 17-07 Task 1 — the stress instrument (reduced #66-shape loop with -d int,guest_errors capture) plus memory-side-probe-only investigation, delivering 17-CR07-ROOTCAUSE.md with the mechanism, candidate verdicts, and the #47/#50/#66 fold verdict; the fix + deterministic pin ride 17-07 Task 2, the N=10 stress pin Task 3. Which candidate is causal is execution output per locked D-03 — deliberately not answered at plan time.
2. **What is "bounded memory" for the D-06 assertion?**
   - What we know: cursor only grows (recycled via quick/misc lists); the slope is the signal; residue ~16 KiB/exit was measured at 15-03; arena 32 MiB.
   - What's unclear: the acceptable slope per 1000 execs after a genuine fix (some table-growth is legitimate — Moribund husks are never reaped: proc.rs:103-108 comment "Slots are never REAPED this era").
   - Recommendation: the r9-side churn profile asserts a per-exec delta ceiling derived from a first measured-green baseline; document the number in the profile's comment. Moribund-husk accumulation is a KNOWN era shape — do not chase it as a leak (it's bounded by MAX_PROCS × husk size).
   - Resolution (RESOLVED): the r9-side churn profile asserts a per-exec cursor-delta ceiling derived from a first measured-green baseline, the number and its derivation documented in the driver comment; Moribund-husk accumulation excluded as a known bounded era shape — adopted by 17-02 Task 1 (churngate.rs + its slope-logic pins).
3. **Does the 12-finding re-proof need new arch-keyed boot pins, or do existing host pins + x64 gates satisfy D-13's Source cell?**
   - Recommendation: host test names + the existing x64 gate names (both are "pins that can fail"); only CR-03 wants a NEW deterministic x86_64-keyed interleaving test (D-07) since the aarch64 `sem_tests` module has 4 pins the x86_64 side compresses into 1.
   - Resolution (RESOLVED): existing host pin names + the existing x64 gate family satisfy D-13's Source cell ("host test name or boot profile"); only CR-03 gains NEW deterministic x86_64-keyed pins (the four sem_ tests) — adopted by 17-01 Tasks 1-3 (the 12 re-cites; CR-03's new pins land in Task 2). The flagged assumption stands: if VERIFICATION demands arch-keyed boot pins per finding, that reopens as gap-closure work.
4. **IN rows in the two ledgers stay open — confirm VERIFICATION accepts that** (D-12 says untouched rows stay; the phase's FLUSHOUT table simply omits them).
   - Resolution (RESOLVED): the IN rows stay open per D-12 and the FLUSHOUT table omits them — adopted by 17-01 Task 3 (the 30-row seed omits IN rows by construction) and 17-04 Task 3 (the 07 ledger closes with only the IN rows open, open count <= 5); VERIFICATION walks FLUSHOUT (D-16), which never sees an IN row.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| nix (offline devshell) | all r9 builds/tests | ✓ | 2.34.9 | — |
| cargo (via `nix develop --offline`) | host pins | ✓ | in devshell | — |
| `cargo test -p port --lib` | port host pins | ✓ | 101 pass / 0.04s (live) | — |
| `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` | x86_64 host pins | ✓ (override REQUIRED) | 60 pass (live) | plain `--bins` = 1330 E0463 errors — not usable |
| qemu-system-x86_64 | boot gates | ✓ | 11.1.2 on PATH | — |
| /dev/kvm | KVM-native gates | ✓ present + writable | — | TCG documented fallback (X64-05) |
| r9 repo state | fix landing | ✓ `main` @ 208ceba == origin/main (pushed, clean) | — | — |
| Harvey flake.lock r9 pin | composition | ✓ rev 208ceba == r9 HEAD | — | — |
| tap/br0 venue (192.168.122.0/24) | net-stress legs if wanted | not re-probed this session | — | slirp hostfwd legs (the soak burst shape ran on tap at 16; CR-07 stress can run slirp-side since #66 fired on the composed soak) |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** tap venue unverified this session — the stress leg does not require it (the #66 shape is the burst leg, venue-agnostic in class).

## Security Domain

Brief and honest: this is an OS-kernel bug-fix phase, not a web app. `security_enforcement` is absent from config (treated enabled); the applicable surface:

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2/V3 Authentication/Sessions | no | Phase 18 owns auth substrate |
| V4 Access Control | no | no permission-surface changes this phase |
| V5 Input Validation | **yes (device-input class)** | the P7 WR family IS untrusted-device input validation: every admission hole closed becomes a loud bind/walk fail (never a panic, never silent acceptance) — the module-posture quotes ("the module header says the device is untrusted input (V5)") carry |
| V6 Cryptography | no | none touched |

| Threat Pattern | STRIDE | Standard Mitigation (this phase) |
|-----------------|--------|----------------------------------|
| Lying/malformed virtio device (caps, BARs, used-ring) | Tampering/Elevation | loud bind-fail rejects (WR-01..04/06 fixes) + host-test negative controls |
| Kernel memory exhaustion via fork/exec churn | DoS | D-06 leak gates + refcount proofs |
| Kernel panic as DoS (user-triggerable) | DoS | CR-04-class fixes re-pinned; no new panic paths in fixes (panic-boundary P-F discipline) |

## Sources

### Primary (HIGH confidence — read this session)
- `.planning/WINDOWS.md` — rows #18, #44–#50, #65, #66 (JSON block lines 308, 646-745, 906-932)
- `.planning/milestones/v2.0-phases/07-.../07-REVIEW.md` (WR-01..08 bodies) + `07-REVIEW-DISPOSITION.md` (13 open)
- `.planning/milestones/v2.0-phases/08-.../08-REVIEW.md` (CR-01..07, WR-01..06) + `08-REVIEW-DISPOSITION.md` (12 fixed + CR-07 skipped) + `08-REVIEW-FIX.md` (fix bodies + SHAs + battery) + `deferred-items.md` (#47/#44/#45/#46/#49 detail)
- `.planning/phases/16-.../deferred-items.md` (#66 toolkit + stress shape)
- `../r9`: `x86_64/src/{proc.rs,syscall.rs,exec.rs,allocator.rs,runtime.rs,main.rs}`, `aarch64/src/proc.rs` (reference), `port/src/{namec.rs,netif.rs,virtio.rs,devpipe.rs,allocator.rs,rendez.rs}`, `port/src/devip/{stack.rs,waker.rs,proto.rs,conv.rs}`, `x86_64/src/platform/q35/pci.rs`, `x86_64/lib/config_q35net.toml`, `xtask/src/main.rs`, `.github/workflows/xtask.yml`
- Harvey: `sys/src/cmd/rc/{exec.c,plan9.c,simple.c,havefork.c}`, `flake.nix`, `build/x86_64/` (mkimage.sh, tcpblast.c, smokesleep lineage), `flake.lock`
- Live executions: `cargo test -p port --lib` (101 pass), `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` (60 pass), plain `--bins` failure captured, KVM/qemu/nix probes

### Secondary (MEDIUM)
- smoltcp 0.14 internals as quoted in 08-REVIEW (verified against crate source at review time; not re-derived this session)
- STATE.md 15-03 arena/residue measurements (recorded evidence, not re-measured)

### Tertiary
- None — no external web sources used (external search providers disabled in `.planning/config.json`; every claim is repo-grounded). Knowledge-graph query returned empty on phase terms (graph 134h/234-commits stale — treated as no signal).

## Project Constraints (from CLAUDE.md)

No `CLAUDE.md`/`.claude/CLAUDE.md` exists. Project instructions live in `AGENTS.md` (loaded): for Harvey-repo C changes (the rc fixes) follow `sys/man/6/style` — tabs only, block comments `/* */` only, `nil` never `NULL`, `exits()`/`sysfatal`/`-1`+errstr idioms, `goto usage` accepted, no auto-init at declaration, match the surrounding file. r9 follows its own rustfmt.toml. The 64-bit ARM architecture is `aarch64` (never `arm64`) in all docs/tooling. GSD workflow enforcement: phase work runs under `/gsd-execute-phase`.

## Metadata

**Confidence breakdown:**
- Fix-site map (findings → code): HIGH — every site read, load-bearing values quoted verbatim
- Pin venues: HIGH — both batteries executed live on this host this session
- CR-07 root cause: LOW-by-design — candidates enumerated, cause unpinned (the phase's centerpiece task, per D-03)
- Ledger closure mechanics: HIGH — all three ledger formats read at the row level

**Research date:** 2026-11-12
**Valid until:** 2026-12-12 (repo-internal facts; stable unless r9 main moves — re-verify the 208ceba base before Wave 1 if fixes land from Phase 18's parallel work)
