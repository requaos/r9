---
phase: 13-r9-x86-64-platform-bring-up
fixed_at: 2026-11-08T14:53:30Z
review_path: .planning/phases/13-r9-x86-64-platform-bring-up/13-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 13: Code Review Fix Report

**Fixed at:** 2026-11-08T14:53:30Z
**Source review:** .planning/phases/13-r9-x86-64-platform-bring-up/13-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope (critical_warning): 4 (CR-01, WR-01, WR-02, WR-03)
- Fixed: 4
- Skipped: 0 (in scope)
- Out of scope, not applied: 5 (WR-04, IN-01..IN-04 — see "Skipped Issues (Out of Scope)")

All fixes land in the sibling r9 repo (`/home/req/code/r9`, branch `main`), one
atomic commit per finding. `workflow.use_worktrees` is `false`, so edits and
commits were made directly in the main checkout (no isolated worktree; see
"Verification Environment").

## Fixed Issues

### CR-01: `intrspecial` marks the SWAPPED witness bit in the saved RIP word, not the saved CS word

**Status:** fixed: requires human verification (logic fix in dormant entry-path code)
**Files modified:** `../r9/x86_64/src/trap.rs`
**Commit (r9):** `b31a039`
**Applied fix:** Changed the `intrspecial` witness-bit slot constant from
`SWAPPED_CS_OFFSET + 16` to `SWAPPED_CS_OFFSET + 24`, matching `intrregular`'s
arithmetic. At the `orb`, the scratch pushes (%rax/%rbx) are already popped, so
`%rsp` is at its entry value and the frame is `[ret][err][RIP][CS][RFLAGS]` at
+0/+8/+16/+24/+32 — byte 24+7=31 is the saved CS word's high byte (free bit 63
of the zero-extended CS push); +16 addressed the saved RIP's high byte, which
both hid the witness from the return-path `btr` and would make a canonical user
RIP non-canonical. Added a six-line comment at the `orb` pinning the corrected
arithmetic so the offset cannot silently regress. Human-verification flag: the
proof is structural (offset now equals `intrregular`'s; the compensating `btr`
at CS bit 63 in `intrcommon` now sees the bit) plus gates — the buggy branch is
dormant until user mode exists (Phase 14), so no runtime exercise of the
swapped path is possible today.

### WR-01: q35net discovery→bind stage anchor is a satisfaction-time `capture.len()` — same-burst hazard

**Status:** fixed: requires human verification (anchor logic change)
**Files modified:** `../r9/xtask/src/main.rs`
**Commit (r9):** `209fb78`
**Applied fix:** The `Q35PciDiscovery`→`X64Bind` transition now anchors
positionally — `bind_after = last_marker_line_end(&capture[pci_after..],
PCI_NET_DEVICE_MARKER).map(|p| pci_after + p).unwrap_or(capture.len())` —
mirroring the facts→discovery anchor one stage earlier, instead of
`bind_after = capture.len()`. The bind lines (`virtio-net: features offered`,
`#l0: virtio:`) ride the same pre-`sti` serial burst as the discovery lines, so
a satisfaction-time anchor excluded them forever whenever they arrived in the
same socket read, reding the gate only at budget expiry. The later
`capture.len()` resets (X64PingPong/X64TxBulk) are untouched — those markers
arrive seconds later, after 256 slirp ARP round trips. Verified end-to-end by
the q35net boot gate (PASS, see verification log below). Human-verification
flag: the race window itself cannot be forced deterministically from the host;
the fix mirrors the already-green facts-stage pattern and the full q35net gate
passed with the new anchor.

### WR-02: `gen_trap_stub!(8)` dead second arm — double fault routed through `intrregular`, contradicting the documented special set

**Status:** fixed: requires human verification (entry-path routing change, dormant)
**Files modified:** `../r9/x86_64/src/trap.rs`
**Commit (r9):** `abb5826`
**Applied fix:** Took the review's primary option: vector 8 (double fault) now
gets BOTH properties — hardware error-word consumption AND the special GSBASE
path. Added a `gen_stub!(err_spec)` arm (`.balign 8; callq {intrspecial};` — no
fake `pushq $0`, since hardware pushes the error word for #DF), switched the
first `(8)` arm of `gen_trap_stub!` to it, and deleted the shadowed dead
`(8) => gen_stub!(spec)` arm. The module's doc comment ("NMIs, double faults,
and the various debug exceptions fall into this category") is now true of the
generated stubs. Routing DF through `intrspecial` is behavior-equivalent today
(GSBASE is always the Mach base at entry, so the `je` skips `swapgs`, same as
the CS-selector test in `intrregular`) and correct for Phase 14's DF-from-user.
Human-verification flag: the #DF path itself never fires on currently-bootable
configs; equivalence argued structurally and covered by the green q35 boot gate.

### WR-03: New CI job `boot-test-x86-64` has no `timeout-minutes`

**Status:** fixed
**Files modified:** `../r9/.github/workflows/xtask.yml`
**Commit (r9):** `029e32f`
**Applied fix:** Added `timeout-minutes: 120` to the `boot-test-x86-64` job with
a comment stating the budget arithmetic. **Deviation from the review's literal
suggestion:** the review proposed `timeout-minutes: 75` with the comment
"3 × (TCG budget 30 min) + provisioning + builds, with margin", but that sum
(90 min of legitimate TCG budgets alone, before apt provisioning and three
kernel+ISO builds) exceeds 75 — the literal value would SIGTERM legitimate
worst-case runs whose own harness budgets should be the thing that fails them.
120 min preserves the finding's intent (a hard cap far below GitHub's 360-min
default, so a wedged qemu chardev or grub-mkrescue stall cannot burn hours)
without converting legitimate slow-TCG behavior into kills.

## Skipped Issues (Out of Scope)

Not applied — outside `fix_scope: critical_warning` for this run. They remain
open in 13-REVIEW.md for the human / a `--fix --all` run.

### WR-04: Dead ISA-override fold chain whose staging justification is contradicted by the landed bind

**File:** `../r9/x86_64/src/platform/q35/ioapic.rs:86-104,182-191`
**Reason:** out of scope — `**Category:** necessity` finding; removing code is a
scope decision for the human (#4100), excluded from critical_warning runs.

### IN-01: devcons probe's "printed loudly" failure path is a silent no-op

**File:** `../r9/x86_64/src/platform/q35/devcons.rs:38`
**Reason:** out of scope — Info tier (needs `--fix --all`).

### IN-02: Boot-test budget env overrides silently ignore unparseable values

**File:** `../r9/xtask/src/main.rs:1676-1696`
**Reason:** out of scope — Info tier (needs `--fix --all`).

### IN-03: `platform/mod.rs` cfg arms are token-identical — the attributes cannot change what compiles

**File:** `../r9/x86_64/src/platform/mod.rs:14-22`
**Reason:** out of scope — Info tier and `**Category:** necessity` (needs
`--fix --all`).

### IN-04: New CI job copies `actions/checkout@v3` — stale floating major, not hash-pinned

**File:** `../r9/.github/workflows/xtask.yml:66`
**Reason:** out of scope — Info tier (needs `--fix --all`).

## Verification Environment

- **Where gates ran:** the r9 main checkout (`/home/req/code/r9`) via
  `nix develop --command sh -c '...'` — `workflow.use_worktrees` is `false`, so
  no isolated worktree was created and the numbers are reproducible directly
  from the r9 tree at the commit hashes above (r9 has no separate worktree
  teardown; `git -C /home/req/code/r9 status --porcelain` is clean).
- **CR-01 + WR-02 (trap.rs):** `cargo xtask check` exit 0; `cargo xtask clippy`
  exit 0 (only pre-existing dead-code warnings in `port`/`xtask`/trap.rs's
  staged `IntrStatus`/`spl*` items — present before the edits); `cargo xtask
  test` exit 0 (101 passed, 0 failed); one `cargo xtask boot-test --arch
  x86-64 --config q35` — **PASS** (banner, facts, discovery markers, >=3 ticks,
  echo probe; run once for the trap-path pair per the orchestrator's grouping —
  the changed branches are dormant without user mode).
- **WR-01 (xtask anchor):** `cargo test -p xtask` exit 0 (91 passed, 0 failed);
  one `cargo xtask boot-test --arch x86-64 --config q35net` — **PASS**
  (discovery + bind markers + netgate ping-pong/tx-bulk + X64-NET-DONE in
  order, pcap assertion held).
- **WR-03 (CI yaml):** parsed with pyyaml in the devshell (`yaml.safe_load`) —
  OK, `jobs.boot-test-x86-64.timeout-minutes == 120`; `actionlint` is not
  available in the devshell and no hosted runner was available, so this fix is
  verified by parse + review only, as sanctioned for this finding.
- Full test suite / verifier phase: not run here (handled by the verifier
  phase).

---

_Fixed: 2026-11-08T14:53:30Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
