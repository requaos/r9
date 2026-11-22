# Retrospective

Cross-milestone learnings. Newest milestone section above the trends.

## Milestone: v1.0 — Boot Revival

**Shipped:** 2026-11-04
**Phases:** 5 | **Plans:** 38 | **Tasks:** 91
**Timeline:** 2026-10-30 → 2026-11-04 (5 days) | **Commits:** 257 harvey + 107 r9

### What Was Built

- Dormant Harvey build revived and made reproducible: one `nix run .#build` from a clean checkout links all three 9k amd64 kernels with a populated `/amd64/bin`; KERNDATE-pinned same-path bit-reproducibility demonstrated with a negative control.
- aarch64 toolchain + userspace from a pinned 9front donor: vendored 7a/7c/7l, libc/aarch64, frozen syscall ABI (`aarch64/ABI.md`), 16-binary shell subset executing on the donor kernel under TCG.
- r9 (Rust kernel) brought up on qemu `-M virt`: platform tree, GICv2 + timer + PL011 IRQ, boot-test harness, paqfs initrd delivery, demand-paged exec, SVC dispatch of the measured set, preemption, RFORK/AWAIT/EXITS, notes.
- **The core value:** `nix run .#boot-r9` boots r9 + Harvey aarch64 userland to an interactive `/bin/rc` shell from clean checkouts on darwin + linux; `nix run .#boot-r9-smoke` expect-gates it; GH Actions green on x86_64-linux (run 41825728636).
- Milestone-close hardening: 10 review fixes (stat muid class, cold-buffer residency, lock discipline, keyed qemu sweeps), 5 Python harness tools → one std-only Rust binary, maximal unsafe elimination in r9 (static mut → 0, per-process usermem model locked, 160-site audit), 8 deferred items resolved.

### What Worked

- **Tracer-first / measure-first ordering** — the Wave-0 spike falsified qemu's `-initrd` delivery before machinery depended on it (D-01 amendment); the 05-03 measured syscall set meant zero guessed handlers; D-14's debug-first wave buried the note race before rc depended on notes.
- **Boot-test gates as the regression fabric** — every plan re-ran the profile fan; the gates caught real latent bugs repeatedly (x0 clobber, QuickFit tails, kstack overflow, pick_next deadlock, stdin starvation).
- **Donor fidelity as the specification** — "port 9k semantics literally" + host-pinned tests caught three plan-text flag inversions before they became runtime drift.
- **Evidence discipline** — committed logs, SHA-pinned actions, push-before-pin, and the D-13 ordering convention made the clean-checkout/CI claims auditable (and made the G-05-1 CI failure diagnosable from the raw log).

### What Was Inefficient

- **The lima VM's undocumented manual `/bin/rc`** masked a namespace-env gap on every local leg; the first fresh-host run (hosted CI) paid for it. Convention: environment conveniences must be codified in the scripts that need them (Dockerfile:42 did; nsrun.sh didn't — now it does).
- **Streaming CI log interleave** cost a misread symptom ("died at libString") — raw logs via API are the source of truth.
- **Deferred-items format drift** (table vs list shapes across phases) fought the audit tooling's writer; the list form with `status:` fields is the durable shape.

### Patterns Established

- Wave-0 debug-first plans before load-bearing subsystems; measure-first traces before growth.
- Fused constructors for cross-boundary access (usermem: validate+reside at construction).
- Two-repo commits: r9 production commits land first, harvey records the SHA (D-13); flake input over relative paths.
- The honesty floor: audits distinguish eliminated / encapsulated / floor; "attempted, reverted, reason" descopes never land broken.

### Key Lessons

- A gate that runs on every plan is worth more than a deep review at the end — but the end review still found two Criticals the gates couldn't see (wire-format records, residency classes).
- "It passes on my machine" was literally an undocumented file on the machine. Fresh-host execution (CI) is a distinct verification axis, not a redundant one.
- Unexplained defuses are forbidden fruit: every flake gets a root cause or a watchdog.

## Cross-Milestone Trends

| Milestone | Phases | Plans | Tasks | Days | Commits (harvey+r9) |
|-----------|--------|-------|-------|------|---------------------|
| v1.0 Boot Revival | 5 | 38 | 91 | 5 | 257+107 |
