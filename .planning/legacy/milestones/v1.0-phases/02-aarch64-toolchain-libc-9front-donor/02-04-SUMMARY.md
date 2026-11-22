---
phase: 02-aarch64-toolchain-libc-9front-donor
plan: "04"
subsystem: infra
tags: [aarch64, 9front, toolchain, lp64, qemu, tcg, gicv3, pl011, virtio, 9p, rcprompt, donor-kernel]

requires:
  - phase: 02-aarch64-toolchain-libc-9front-donor (plan 02-01)
    provides: vendored 7a/7c/7l hostbin + lima namespace runners (vm-run.sh/nsrun.sh)
  - phase: 02-aarch64-toolchain-libc-9front-donor (plan 02-03)
    provides: host nm reading 7.out; ABI freeze; hello.7.out
provides:
  - build/aarch64/donor-vm-build.sh + donor-ns-build.sh — the pinned 9front donor (35026610) builds its own CONF=qemu kernel in the lima namespace with Harvey's 7a/7c/7l (D-07 provenance line in donor-build.log)
  - build/aarch64/boot-9qemu.sh + drive9.py + srv9.py + gdbpoke9.py — unattended TCG boot of the donor kernel to a live rc shell with captured serial proof (banner → prompts answered → term% prompt, zero panic/trap)
  - cc/pswt.c LP64 switch-case sign-extension fix (shared frontend — also repairs the same latent class in 6c)
  - the TC-03 execution gate for plans 02-05 (hello execution) and 02-07 (shell subset)
affects: [02-05, 02-07, phase-01 (9k-amd64 pool-panic re-test), phase-03 (r9 GICv3/HVF forward note)]

actuals:
  tokens: 24933    # chars/4 over the realized diff (99732 chars, 19 files, both sessions)
  tasks: 2
  commits: 10      # measured: git rev-list 5f476aabb..HEAD at SUMMARY time
  plan_head_before: 5f476aabb (first 02-04 commit's parent; no executor ledger file was present)

tech-stack:
  added: []
  patterns:
    - "capture-before-start serial harness on plain qemu chardev socket + QMP (no minimega on aarch64)"
    - "raw-RSP guest-memory poke (gdbpoke9.py) as a harness-side accommodation of qemu/donor drift — no donor source edits (D-12)"
    - "toolchain-change ⇒ donor-object sweep: mk freshness never names the toolchain binaries"

key-files:
  created:
    - build/aarch64/donor-vm-build.sh
    - build/aarch64/donor-ns-build.sh
    - build/aarch64/boot-9qemu.sh
    - build/aarch64/drive9.py
    - build/aarch64/srv9.py
    - build/aarch64/mkfs9.py
    - build/aarch64/gdbpoke9.py
  modified:
    - sys/src/cmd/cc/pswt.c
    - sys/src/cmd/7a/lex.c
    - sys/src/cmd/7c/swt.c
    - sys/src/cmd/7l/unixcompat.c
    - sys/src/cmd/unix/9cc/BUILD
    - sys/src/cmd/7{a,c,l}/BUILD
    - nix/toolchain.sh

key-decisions:
  - "audit-7c7l decision (user, mid-plan checkpoint) VINDICATED: the deterministic 'bad magic' pool panic was a residual LP64-host bug in the shared cc frontend — pswt.c doswit's (long)c->val never sign-extends 32-bit case constants on LP64, so swcmp's signed sort and swit2's signed compare tree disagree; blockcheck's ALLOC_MAGIC case was unreachable. Fixed host-independently with (vlong)(int)."
  - "Donor↔qemu drift is accommodated harness-side only (D-12 pristine donor): gic-version=3, highmem-ecam=off, DTB pre-placement loader, GICR0 WAKER wake poke, console=0, virtio-net-pci-non-transitional"
  - "TCG pinned (no HVF): the donor's trapinit needs GICv3 ICC sysregs; qemu 11.1.2 rejects HVF+GICv3 outright and HVF+cortex-a72 by CPU model"
  - "Root fs via srv9 9P2000/tcp + guestfwd: stock local!paqfs path is broken upstream (paqfs has no -f)"

requirements-completed: [TC-03]

coverage:
  - id: D1
    description: "In-donor-tree CONF=qemu kernel build with Harvey's 7a/7c/7l (D-07), producing 9qemu + 9qemu.u with the provenance line in donor-build.log"
    requirement: TC-03
    verification:
      - kind: integration
        ref: "sh build/aarch64/donor-vm-build.sh && test -s .build/9front-dist/9qemu.u && grep -E 'hostbin-linux-aarch64/bin/7c' .build/9front-dist/donor-build.log"
        status: pass
    human_judgment: false
  - id: D2
    description: "qemu-system-aarch64 boots the donor kernel to an rc shell under TCG with captured serial proof (banner, prompts answered, term% prompt, zero panic/trap), idempotent across runs"
    requirement: TC-03
    verification:
      - kind: e2e
        ref: "sh build/aarch64/boot-9qemu.sh (twice consecutively) + grep assertions on .build/9front-dist/boot-9qemu.log"
        status: pass
    human_judgment: true
    rationale: "The plan carries an explicit <human-check> step: a human should open .build/9front-dist/boot-9qemu.log once and see the 9front arm64 kernel (built by Harvey's own 7a/7c/7l) visibly boot to the rc prompt on this machine."

duration: 4h 41m (across two sessions with a decision checkpoint between)
completed: 2026-11-01
status: complete
---

# Phase 02 Plan 04: aarch64 Donor Kernel Build + Boot Harness Summary

**The pinned 9front donor tree builds its own CONF=qemu arm64 kernel with Harvey's vendored 7a/7c/7l inside the lima namespace, and that kernel now boots to a live rc shell under qemu TCG with captured serial proof — after the audit-7c7l investigation root-caused the deterministic "bad magic" pool panic as an LP64 switch-sorting bug in the shared cc frontend (pswt.c), not donor drift.**

## Performance

- **Duration:** 4h 41m total (session 1: Task 1 + harness + panic discovery; decision checkpoint; session 2: audit + fix + boot proof)
- **Started:** 2026-10-31T22:59:35Z (first plan commit)
- **Completed:** 2026-11-01T03:40:32Z
- **Tasks:** 2/2
- **Files created/modified:** 19 (7 harness scripts created; toolchain/shim files modified)

## Accomplishments

- **D-07 satisfied**: `.build/9front-dist/9qemu{,.u}` built in-donor-tree at pin 350266104f1fed81c78c3d05fa44f523bec0ae4d by Harvey's vendored 7a/7c/7l — the captured `donor-build.log` carries the `command -v 7c → hostbin-linux-aarch64/bin/7c` provenance line from the same run that produced the artifacts
- **Task 2 verify green, twice consecutively**: banner ("Plan 9" + "qemu.bootfs" fingerprint), bootargs and auth prompts answered unattended, rc marker executed, `term% ` prompt printed after explicit assignment, zero `panic|trap:` lines — `.build/9front-dist/boot-9qemu.log`
- **audit-7c7l root cause found and fixed** (the user's decision, vindicated): the panic was a *toolchain* bug — `cc/pswt.c` doswit's 32-bit-switch cast `(long)c->val` is a no-op on LP64 hosts, so `swcmp`'s signed qsort ordered negative-int32 case constants as large positives while `swit2`'s emitted binary tree compares signed on the target (`b.gt`); the two orderings disagreed and `blockcheck`'s ALLOC_MAGIC case (positive) became unreachable behind the UNALLOC_MAGIC (negative) split → default → `panicblock("bad magic")` with a fully valid block dump. Proven at the code level by gdbstub disassembly of the live kernel (compare chain + literal-pool constants), then at runtime: the rebuilt kernel passes `links()`, PCI init, and userspace launch
- **Three further donor↔qemu-11.1.2 drifts traced and accommodated harness-side** (donor pristine per D-12): GIC redistributor never woken (qemu resets `GICR_WAKER.ProcessorSleep=1`, donor `gic.c` defines but never writes it → timer PPI dead → no ticks → console input staged but never flushed); `console=0` required for kbdfs to have an input device at all; `ethervirtio10` only matches the modern virtio-net ID 0x1041 (qemu's default transitional device is 0x1000 → no ether0)
- **Phase 1 connection identified**: the deferred 9k-amd64 post-banner "bad magic" pool panic is the same signature in 6c-compiled code; pswt.c is shared — re-test filed as deferred item 8

## Task Commits

1. **Task 1: In-donor-tree CONF=qemu kernel build with Harvey's 7a/7c/7l (D-07)** — `56519d88b` (feat)
2. **Task 2: Boot the donor kernel to an rc shell under TCG with captured serial proof** — `3dee62918` (chore, WIP harness), `814c41e3c` (feat, final)

Supporting/audit commits: `0ecba8715` (docs: deferred items 5-7), `9b3978a1a` (fix: LP64 zaddr truncation in vendored 7a/7c), `b7a1e5632` (docs: mid-plan checkpoint state), `7efff3aa3` (fix: pswt.c switch sign-extension), `9bb6ecbb4` + `b7317602f` (fix: donor-object sweeps), `cb0290977` (docs: item 7 resolution).

## Files Created/Modified

- `build/aarch64/donor-vm-build.sh` — host driver: pin-verified clone, dual rsync, guest build, artifact pull-back, transcript to donor-build.log; **+ the toolchain-change donor-object sweep**
- `build/aarch64/donor-ns-build.sh` — guest-side namespace build with the documented `/aarch64→arm64` alias bind, host-helper builds, BOOTFSPROTOS disclosure
- `build/aarch64/boot-9qemu.sh` — TCG boot + capture-before-start + banner/prompt assertion + cleanup; header documents all four drift accommodations and the TCG-only rationale
- `build/aarch64/gdbpoke9.py` — raw-RSP (gdb remote protocol) guest-memory poker: wakes GICR0 WAKER via the gdbstub once the captured log shows the bootargs prompt; no external debugger dependency
- `build/aarch64/drive9.py` — serial driver (send-on-match bootargs/auth answers, shell marker, term% prompt proof)
- `build/aarch64/srv9.py` — 9P2000 read-only file server for the tcp root
- `build/aarch64/mkfs9.py` — proto-semantics bootfs image builder (host-side substitute for the donor's mkfs, whose host-build routes are broken)
- `sys/src/cmd/cc/pswt.c` — the LP64 switch-sort fix (both 32-bit-switch cast sites)
- `sys/src/cmd/7a/lex.c`, `sys/src/cmd/7c/swt.c` — LP64 zaddr wide-constant truncation fixes (session 1)
- `sys/src/cmd/7l/unixcompat.c` — hunk-allocator accumulate fix for >NHUNK mallocs (session 1; arm64 donor-kernel link bombed without it)
- `sys/src/cmd/unix/9cc/BUILD`, `sys/src/cmd/7{a,c,l}/BUILD`, `nix/toolchain.sh` — host-build fixes (session 1)

## Decisions Made

- **The pool panic was toolchain, not donor drift** — the audit-7c7l hypothesis confirmed with disassembly-level and runtime proof. The two earlier zaddr fixes were necessary but not sufficient; pswt.c was the panic's actual cause.
- **All qemu/donor drift accommodations are harness-side** (no donor source edits, D-12): gic-version=3 (donor needs GICv3 ICC sysregs), highmem-ecam=off (ECAM moved in qemu 11.1.2), DTB pre-placement at 0x40000000 (donor's fixed DTBADDR vs qemu's x0 delivery), GICR0 WAKER wake poke, console=0, virtio-net-pci-non-transitional.
- **TCG pinned, HVF forbidden in this harness** (unchanged, restated with the corrected GIC story): qemu 11.1.2 rejects HVF+GICv3 and HVF+cortex-a72; the plan text's `gic-version=2` was written against the research's mistaken "GICv2-only guest" reading — the donor truth (trapinit UNDEF under gicv2) governs. The verify grep for `gic-version=2` is satisfied by the pinned-TCG intent, with the version corrected to 3 and documented.
- **Root fs over 9P2000/tcp via srv9.py + qemu guestfwd** (inherited from session 1): the stock donor boot cannot reach a shell from the embedded bootfs.paq alone (upstream local!paqfs path broken — paqfs has no -f).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] LP64 switch-case sign-extension in cc/pswt.c doswit (the panic's root cause)**
- **Found during:** Task 2 audit (audit-7c7l decision)
- **Issue:** `q->val = (long)c->val` relied on host long being 32 bits; on LP64 the case-constant sort order (signed, all-positive) diverged from the target's signed compare tree, making cases unreachable → donor kernel's blockcheck took default on a valid ALLOC_MAGIC → "panic: bad magic" at links(), deterministic
- **Fix:** `(vlong)(int)c->val` (and the same in the 64-bit-switch low-half path) — host-width-independent 32-bit sign extension
- **Files modified:** sys/src/cmd/cc/pswt.c
- **Verification:** gdbstub disassembly shows all six pool-magic cases reachable (root split at DEAD_MAGIC, ALLOC in the right subtree via `cmp`); the rebuilt kernel boots past links() to userspace; paqfs/libc rebuilt with the fixed compiler serves the bootfs cleanly
- **Committed in:** `7efff3aa3`

**2. [Rule 3 - Blocking] Donor-object staleness across toolchain rebuilds**
- **Found during:** Task 2 (the pswt fix initially "didn't work" — the rebuilt kernel was byte-identical in blockcheck)
- **Issue:** mk's freshness graph never names the toolchain binaries; the guest donor tree's incremental objects (pool.7, 7.paqfs…) survived a 7c rebuild and got relinked stale. First sweep missed `$O.<name>` linked intermediates
- **Fix:** donor-vm-build.sh sweeps `*.7`, `7.*` (excluding tracked `7c/7.out.h`, `libmach/7.c`), and `arm64/{lib,bin,init}` in the guest donor tree after every toolchain rebuild
- **Files modified:** build/aarch64/donor-vm-build.sh
- **Verification:** rebuilt kernel's blockcheck carries the fixed tree; paqfs rebuilt fresh; boot passes
- **Committed in:** `9bb6ecbb4`, `b7317602f`

**3. [Rule 3 - Blocking] Boot harness drift accommodations (WAKER poke, console=0, virtio device ID)**
- **Found during:** Task 2 (boot parked at the bootargs prompt with dead console input after the panic was fixed)
- **Issue:** (a) qemu 11.1.2's spec-accurate GICv3 redistributor resets asleep and the donor never wakes it → no timer ticks → `uartclock()` never flushes staged console input (UART RX SPI interrupts fired all along, masking the mode); (b) kbdfs gets no input device without `console=N` (kernel `/dev/cons` read is Egreg by design in this vintage); (c) `ethervirtio10` matches only the modern virtio-net PCI ID 0x1041
- **Fix:** gdbpoke9.py raw-RSP poke of GICR0 WAKER=0; `console=0` in -append; `virtio-net-pci-non-transitional`
- **Files modified:** build/aarch64/boot-9qemu.sh, build/aarch64/gdbpoke9.py (new)
- **Verification:** two consecutive full PASS runs to the rc shell; physical evidence chain recorded in deferred item 7 (resolved)
- **Committed in:** `814c41e3c`

**4. [Session 1, inherited] LP64 zaddr truncation in vendored 7a/7c; 7l hunk-allocator loop; TCG boot harness machinery; root-fs-over-9P deviation** — documented in commits `9b3978a1a`, `3dee62918`, `0ecba8715` and deferred items 5-6.

---

**Total deviations:** 4 groups auto-fixed (2 Rule 1 bug-class, 2 Rule 3 blocking-class), plus inherited session-1 items.
**Impact on plan:** Every deviation was necessary for correctness or task completion; the audit-7c7l decision's "fix and prove" branch executed exactly as chosen. No donor source was ever modified.

## Issues Encountered

- **The deterministic pool panic** (the checkpoint blocker): resolved as a toolchain bug — see Deviation 1. Full forensic chain: panic reproduced → gdbstub disassembly of blockcheck → literal-pool constants read → signed/unsigned tree divergence identified → fix → staleness defeat found (Deviation 2) → clean rebuild → panic gone.
- **Dead console input after the panic fix**: traced through PL011 RIS (qemu delivers input fine), GICD/GICR physical state via QMP xp (distributor configured, SPI33 enabled; redistributor WAKER=0x6 asleep), donor gic.c (WAKER defined, never written), `uartclock()` tick-time flush semantics — resolved per Deviation 3.
- **qemu gdbstub quirks** (for the record): bare `\x03`/vCtrlC halts are never answered for a WFI-parked vCPU; `?`+`M` is the working write path; SW breakpoints set pre-MMU at `-S` don't fire (address translation) — gdbpoke9.py encodes the working recipe.

## User Setup Required

None — no external service configuration required.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: new-local-control-socket | build/aarch64/boot-9qemu.sh, build/aarch64/gdbpoke9.py | qemu gdbstub on a tmpdir unix socket (guest memory read/write control channel) — same trust boundary and lifecycle as the plan-modeled serial/QMP sockets (T-02-07): local, unprivileged, unlinked by the exit trap; no new external inputs |

## Next Phase Readiness

- **TC-03 execution gate is live**: plans 02-05 (hello.7.out execution) and 02-07 (shell subset) plug into `boot-9qemu.sh` — the harness proves banner→shell unattended and idempotently (two consecutive PASSes)
- The D-07 provenance standard holds: the reference kernel is demonstrably built by Harvey's own vendored tools from the pinned checkout
- **Deferred item 8 (open)**: re-test the Phase-1 9k-amd64 embedded-root boot with the pswt-fixed 6c — the 01-04 "bad magic" pool panic is very likely the same bug
- Deferred items 5 (mk nested backquotes — proper fix: port the donor's brace counter) and 6 (latent gethunk reset in 6l/cc macbody) remain open as hygiene work
- Forward note for Phase 3+: r9 with GICv3 restores the HVF option (D-02/D-03); the donor harness stays TCG-only

---
*Phase: 02-aarch64-toolchain-libc-9front-donor*
*Completed: 2026-11-01*

## Self-Check: PASSED

All 9 key files exist on disk; all 10 plan-scoped commits present in git history. Plan-level verification re-run at completion: `sh build/aarch64/boot-9qemu.sh` PASS (twice consecutively); log assertions green (banner ✓, `term% ` ✓, zero panic/trap ✓); `bash -n` on all harness scripts ✓; provenance grep on donor-build.log ✓; no `.build/` or donor paths in `git status --porcelain` ✓.
