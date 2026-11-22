# Phase 17: r9 Kernel Flush-Out - Context

**Gathered:** 2026-11-12
**Status:** Ready for planning

<domain>
## Phase Boundary

The r9 kernel is functionally hardened under real userspace load: the open Phase-7/8 review backlog (the CR classes, the RFMEM OOM leaks, the WR hardening gaps) plus the known kernel windows are closed **with tests or explicitly dispositioned with rationale**, every fix landing with a pin (host test or boot-test gate). **Kernel work in sibling repo `../r9`** (branch `main`, push-before-pin); Harvey-side work is the composed churn gate, the three rc userspace rows (D-11), and gate/pin plumbing. Runs parallel with Phase 18 per the roadmap.

Addresses FLUSH-01, FLUSH-02, FLUSH-03, FLUSH-04.

NOT in this phase: the auth substrate (Phase 18), cpu/drawterm/rio (Phase 19), aarch64 anything (frozen at the v2.0 pin — disposition-only there), the ~45 historical WINDOWS rows this phase doesn't touch (close-what-you-touch, D-12), a full WINDOWS.md re-triage, new capabilities of any kind.

Key scouting fact that shapes everything: the Phase-8 fixes for CR-01..06 + WR-01/02/04/05/06 landed in **shared `port/src/devip/*`** — live on x86_64 — but their pins were aarch64 boot-tests, now frozen; nothing re-proves them on x86_64. The WR-03 RFMEM leak fixes landed in **per-arch `aarch64/src/proc.rs`** — `x86_64/src/proc.rs` is a separate file written fresh in Phases 13/14 whose carry-over of that fix family is unaudited (the brk-unwind free-verdict pattern appears present around line 799; the rest unknown). CR-07 was never fixed anywhere.

</domain>

<decisions>
## Implementation Decisions

### Fix-vs-disposition posture (FLUSH-01/02/03)

- **D-01:** **Pin-first, fix-on-failure for the 12 already-fixed shared-code findings** (CR-01..06, WR-01/02/04/05/06). Their fixes are live in `port/src/devip/*`; the x86_64 work is building the pins (D-05 venue rules). A finding reopens as a fix-item ONLY if its pin fails — no speculative code re-audit. The pins ARE the audit. Note: CR-03 and WR-03 carried `requires-human-verification` flags at Phase 8 (race/leak logic in host-unreachable paths) — this phase's deterministic sema test (D-07) and leak-repro gates (D-06) are precisely that verification, re-keyed to x86_64.
- **D-02:** **Audit + port + prove for the RFMEM OOM-path leak family (FLUSH-02).** The researcher audits `x86_64/src/proc.rs` against the aarch64 fix family (brk-unwind free-verdict honor, rfork bss-sweep reorder — map-parent-then-child with record-or-unwind on child-map failure — and every OOM-path release), ports whatever is missing, THEN the leak-repro gates run (D-06). Prove-only was rejected: a slow leak the gate's sample misses stays missed.
- **D-03:** **CR-07 (multi-waiter RX-IRQ wake wedge) gets root-caused, fixed, and pinned — the phase's centerpiece.** Never fixed anywhere; skipped at Phase 8 with root cause unpinned; WINDOWS #66 reproduced the class live on x86_64 under the phase-16 soak burst. The fix must land with a race-stress pin (D-07), not just a logic argument.
- **D-04:** **Phase-7 WRs: fix the named classes where the code is LIVE on x86_64; disposition the frozen/superseded.** Live fixes: WaitSet-16-vs-MAX_PROCS-64 sizing (shared wait machinery), RX used-id slot-liveness (virtio port driver code), cap-walk loud-abort holes (audit the x86_64 ECAM/cap walk from 13-04 for the same admission holes WR-01/02/03/04 name — zero-length/undersized extents, short config reads, unvalidated BAR types, unaligned cap pointers — each becoming a loud bind/walk fail). Dispositions: WR-08 (DTB/ICFGR trigger flags — aarch64-only, frozen) and WR-07 (superseded aarch64 fixture quirk). — **Reversibility:** costly — the cap-walk audit touches the q35 discovery path every net gate boots through; changes there re-run the whole x86_64 gate battery.

### Pin architecture (the FLUSH-04 "every fix lands with a pin" discipline)

- **D-05:** **Host-first, boot-when-needed.** Every finding gets the cheapest pin that can actually fail: a host `cargo test` in r9 where the logic is host-compilable (the precedent already exists — `port/src/devip/{ipifc,arp,netlog,phyadapt,stack}.rs`, `port/src/{rendez,mem,allocator}.rs` all carry `#[cfg(test)]` today), and an xtask boot-test profile only where kernel context is essential (real IRQ paths, real sockets, real page tables). "Always both" was rejected as double gate work with no extra failure signal.
- **D-06:** **Tiered leak gates.** (a) r9-side: an xtask boot profile looping exec of a baked binary with a memory-bound assertion (kernel-level proof, 14-D-06 split); (b) Harvey-side: ONE composed churn leg driving real binaries — the #44 repro shape (`/bin/cat` ×~1400 fork+exec iterations) with a bounded-memory assertion. Kernel-level proof + real-load proof; #44's suspect (exec staging teardown of `ByteBacking::Mnt`) may only drain under real mounted-binary exec.
- **D-07:** **Race pins = deterministic + stress.** Host-deterministic tests for the race LOGIC (sema/waker/rendez are port modules — injectable interleavings on host) PLUS in-kernel stress legs (green-twice-plus on KVM, presence-based markers) proving the fix under real IRQ timing. The stress leg's ancestor is the phase-16 soak burst shape (8-way parallel tcpblast) that made #66 fire 1-in-5.
- **D-08:** **Venue rules are locked here; the concrete per-finding pin mapping is the researcher's; the planner sequences.** No full per-finding pin table was locked at discuss time — nobody has read the fix sites yet; that mapping is research output.

### Adjacent windows scope (FLUSH-04 + folds)

- **D-09:** **Fold #47, #50, #66 into the CR-07 work item.** #47 is the original multi-waiter RX-wedge window record; #50 is an intermittent early-boot devpipe DataAbort in the same family orbit; #66 is the class live on x86_64. One root-cause pass; all rows close or disposition together (a row may legitimately fall out as a different class during investigation — then it gets its own disposition, still inside this phase).
- **D-10:** **WINDOWS #18 (pgrp lock held across namec walks, open since v1.0 phase 5) is FIXED in 17.** It is the last known live namec/pgrp wedge, same subsystem as #45 (one namec pass), and "real userspace load" means rc pipelines and backgrounded commands — exactly the contender shape that wedges it. Phases 18/19 (listen, exportfs, cpu) will contend it hard; fixing it after they land would be re-debugging under auth load. Fix shape per the ledger's own note: snapshot the union chain under a brief lock, walk unlocked.
- **D-11:** **The three Harvey-userspace rc rows (#46 redirect/fd-reuse corner, #49 argv-stack-underflow residual, #65 Execute errstr masking) are fixed IN-PHASE.** Deliberate user override of the defer recommendation — they're small and rc is the load driver; a flush-out phase that leaves the load driver flaky hasn't flushed. They ride as one rc fix-class commit (Harvey repo; no r9 pin mechanics — their regression proof is boot-gate-shaped: the existing rc-driven sweeps re-greening plus a targeted fixture if cheap).
- **D-12:** **Ledger hygiene = close-what-you-touch.** 17 closes/dispositions the rows it actually touches (#18, #44–#47, #50, #66, plus review findings); the other ~45 historical rows stay untouched, statuses unchanged.

### Ledger closure shape (what "closed" looks like)

- **D-13:** **The 07/08 REVIEW-DISPOSITION ledgers close in-place.** Hand-edit the rows: Disposition → `fixed` (Source cites the x86_64 pin: host test name or boot profile) or `dispositioned` (Source carries the D-15 rationale). No superseding doc — the ledgers are designed to survive re-reviews; one file per backlog stays the truth.
- **D-14:** **WINDOWS rows close via the ledger's native resolve fields.** `status` → `resolved`, `reason` = the r9 commit SHA + pin name (e.g. `r9 <sha> + boot churn gate`), `resolved_at` stamped — machine-shaped and greppable, matching every prior closure.
- **D-15:** **The disposition bar is a 3-part test.** Every "explicitly dispositioned with rationale" entry carries: WHERE the class lives (frozen aarch64 / superseded fixture / era note), WHY it's not fixed (freeze policy / supersession citation), and WHAT would reopen it (the reopener condition — e.g. "aarch64 reactivation" or "the fixture returns"). One-line reasons were rejected as too weak a future decision record.
- **D-16:** **17-FLUSHOUT.md is the phase's single closure table.** One row per finding/window: finding ID → action taken (fixed / dispositioned / folded) → pin (host test / boot profile / composed gate) → commit (r9 SHA or Harvey SHA) → ledger refs (REVIEW-DISPOSITION row + WINDOWS row). VERIFICATION walks it row by row; the ledgers stay the per-backlog truth, this table is the phase's index over them.

### Carried forward (locked in prior phases — do not re-litigate)

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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 17 — goal, success criteria 1–4, requirement mapping; §Phase 18 (the parallel phase — no shared substrate conflicts)
- `.planning/REQUIREMENTS.md` — FLUSH-01..04 full text; Out of Scope table
- `.planning/PROJECT.md` — Key Decisions (push-before-pin, gates-split, venue honesty rows), Constraints
- `.planning/STATE.md` — Accumulated Context (Phases 13–16 kernel decisions)

### The backlog ledgers (THE subject matter)
- `.planning/WINDOWS.md` — rows #18, #44, #45, #46, #47, #49, #50, #65, #66 (open, touched by this phase); the JSON block is the machine-readable truth
- `.planning/milestones/v2.0-phases/07-r9-pci-ecam-virtio-net-devether/07-REVIEW.md` — WR-01..08 full finding bodies (the cap-walk/WaitSet/used-id classes)
- `.planning/milestones/v2.0-phases/07-r9-pci-ecam-virtio-net-devether/07-REVIEW-DISPOSITION.md` — all-13-open ledger this phase closes
- `.planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW.md` — CR-01..07 + WR-01..06 full finding bodies (file/line anchors, some aarch64-pathed)
- `.planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW-DISPOSITION.md` — the 12-fixed + CR-07-skipped ledger with the r9 SHAs of the original fixes
- `.planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-REVIEW-FIX.md` — what each Phase-8 fix actually changed (the carry-over audit's diff baseline)
- `.planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/deferred-items.md` — the CR-07 original record + #46/#49-era notes
- `.planning/phases/16-host-net-modes-networking-exit-gate/deferred-items.md` — #66's diagnostic toolkit + the burst-leg stress-loop shape (the CR-07 stress pin's ancestor)

### Prior-phase context (the machinery this phase rides)
- `.planning/phases/14-harvey-amd64-userspace-on-r9/14-CONTEXT.md` — D-06 gates-split (r9-side vs Harvey-side pin venues)
- `.planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-CONTEXT.md` — D-01 fix-everything boundary (18-substrate stays out)
- `.planning/phases/16-host-net-modes-networking-exit-gate/16-CONTEXT.md` — D-07 (r9-side tap profile may join 17), the soak-burst shape, slirp-refusal venue honesty

### r9 repo (sibling, `../r9` — where the fixes land)
- `../r9/port/src/devip/` — conv.rs, ipifc.rs, stack.rs, proto.rs, waker.rs, arp.rs, route.rs — the CR/WR fix sites (shared, live on x86_64); existing `cfg(test)` homes in ipifc/arp/netlog/phyadapt/stack
- `../r9/x86_64/src/proc.rs` — the per-arch RFMEM/OOM substrate D-02 audits (rfmem_brk_set ~716, release_user_page ~661, the exit walks ~1325/~2277)
- `../r9/aarch64/src/proc.rs` — REFERENCE ONLY (frozen): the aarch64 fix family D-02 audits against (the Phase-8 fix commits)
- `../r9/port/src/namec.rs` + the pgrp machinery — #45 union-create + #18 lock-across-walk fix sites
- `../r9/port/src/devpipe.rs` — #50's read path (early-boot DataAbort record)
- `../r9/x86_64/src/platform/q35/` — the x64 ECAM/cap walk D-04 audits (13-04's discovery path); netgate.rs host-test precedent
- `../r9/xtask/` — boot-test profile machinery (where the churn/race/stress profiles land); `r9-netdev-args` bin precedent for shared-code front-ends
- `../r9/port/src/{rendez,mem,allocator,maths}.rs` — host-test precedent for the race/leak deterministic pins

### Harvey repo — the surfaces this phase changes
- `sys/src/cmd/rc/exec.c` + `sys/src/cmd/rc/plan9.c` — the three rc rows (#46, #49, #65)
- `build/x86_64/tcpblast.c` + the 14-09 net payload set — the composed churn leg's client lineage (the #44 repro used /bin/cat; tcpblast's marker vocabulary is the gate-shape reference)
- `flake.nix` + `build/x86_64/mkimage.sh` — if the composed churn leg becomes its own flake app/bake (app-per-gate shape)
- `../r9` r9 SHA discipline: every kernel fix pass ends with the Harvey-side FOD re-wire + pin advance + gates re-green

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- r9's host-test infrastructure: `#[cfg(test)]` already in 10+ port/x86_64 modules — the D-05 host pins extend an established pattern, not start one
- r9's xtask boot-test profiles (the 13/14 gate lineage) — the chassis for churn/race/stress profiles; netgate.rs's masked-post discipline is the IRQ-path precedent
- The phase-16 soak burst shape (8-way parallel tcpblast + expect marker collection) — the CR-07 stress leg re-runs this shape as a regression pin
- The 14-09 static marker binary lineage — the r9-side churn loop's exec payload candidate
- `08-REVIEW-FIX.md` + the per-fix r9 SHAs in the disposition ledger — the diff baseline the carry-over audit reads

### Established Patterns
- Cheapest-pin-that-can-fail: every `<automated>` verify names a failure signal (`<fails_when>`) — a pin that cannot fail is not a pin
- Kernel-level evidence r9-side, composed evidence Harvey-side (14-D-06); never duplicate a gate across both
- push-before-pin + FOD re-wire-then-pin per kernel-byte commit; pin advance its own commit
- Memory-side probes only under corruption; green-twice(-plus) for race/regression gates; presence-based assertions
- Ledger rows carry machine-shaped reasons (SHA + pin name) — greppable closure

### Integration Points
- `../r9` `main` branch — all kernel fixes + host tests + boot profiles (push-before-pin)
- `../r9/flake.nix` / Harvey `flake.nix` — only if a new composed churn app joins (D-06b)
- Harvey `sys/src/cmd/rc/` — the three-row rc fix-class commit (D-11)
- `.planning/WINDOWS.md` + the two REVIEW-DISPOSITION ledgers + `17-FLUSHOUT.md` — the closure records (D-13/14/16)

</code_context>

<specifics>
## Specific Ideas

- **The acceptance story, end-to-end:** the researcher audits `x86_64/src/proc.rs` against the aarch64 RFMEM fix family and maps every finding to its pin (D-02/D-08) → the 12 shared-code pins land (host tests where reachable, boot profiles where essential) — any pin that fails reopens its finding as a fix-item (D-01) → the leak-repro tier goes green twice: r9-side marker-binary churn with bounded memory + Harvey composed /bin/cat churn (D-06) → CR-07 root-caused under #66's toolkit (memory-side probes), fixed, pinned deterministic + stress; #47/#50/#66 close or disposition with it (D-03/D-09) → the Phase-7 live classes fixed (WaitSet sizing, used-id retire, x64 cap-walk loud-aborts) with WR-07/08 dispositioned (D-04) → #44 arena-drain and #45 union-create fixed + pinned; #18 joins the namec pass (D-10) → the three rc rows land as one Harvey fix-class commit (D-11) → ledgers close in-place with 3-part dispositions (D-13/14/15) → 17-FLUSHOUT.md indexes every row; VERIFICATION walks it (D-16) → every r9 fix pass re-wired-then-pinned, all Harvey gates re-green.
- **Watch-out:** the WR-03/CR-03 fixes were `requires-human-verification` at Phase 8 — that obligation transfers here; don't mark them closed on the aarch64 fix's word.
- **Watch-out:** `x86_64/src/proc.rs` was written fresh in Phases 13/14 — presence of one fix pattern (the ~line-799 brk-unwind) proves nothing about the rest of the family; audit all OOM-path releases, don't sample.
- **Watch-out:** CR-07/#66 is a race — 1-in-5 observed; a stress pin needs enough runs to be meaningful (green-twice-plus; the planner states N) and presence-based failure markers, never interleaving order. Observer effects are real: instrument the memory side, not the panic side.
- **Watch-out:** every r9 kernel-byte commit re-stales the FOD hash — sequence fix passes as class-commits with re-wire-then-pin after each pass, never one big pin at the end.
- **Watch-out:** don't perturb Phase 16's green gates — new boot profiles and host tests are additive; no edits to existing profile semantics.
- **Watch-out:** rc fixes are Harvey-repo commits with no r9 pin mechanics — their proof is boot-gate-shaped (existing rc-driven sweeps re-green + a cheap targeted fixture if the corner needs one), not a host unit test.
- **User overrides to honor downstream (deliberate, don't second-guess):** the three rc rows fixed IN-phase (D-11) over the defer recommendation; #18 fixed in 17 (D-10) over disposition.

</specifics>

<deferred>
## Deferred Ideas

- **Full WINDOWS.md re-triage** (the ~45 untouched historical rows) — declined for this phase (D-12); if the ledger's noise becomes a real retrieval problem, name a dedicated hygiene pass.
- **Console RX-ring truncation class (#48)** — aarch64 PL011-era record; re-derive on the x64 UART only if a live symptom appears under the churn/stress legs.
- **Era notes (#13/#53 eager exec, #54 nsec/RTC, #55 dead PML4 slot)** — recorded shapes of this era, not bugs; untouched unless a flush-out finding collides.
- **r9-side tap boot-test profiles for their own sake** — join only if a flush-out finding wants a kernel-level tap pin (16-D-07 standing offer).
- **A aarch64-side fix of anything** — frozen at the v2.0 pin; every aarch64 finding is disposition-only with the 3-part rationale.

</deferred>

---

*Phase: 17-r9 Kernel Flush-Out*
*Context gathered: 2026-11-12*
