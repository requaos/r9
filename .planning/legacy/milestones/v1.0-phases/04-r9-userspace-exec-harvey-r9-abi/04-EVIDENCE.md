# Phase 04 Evidence Bundle — r9 Userspace-Exec + Harvey↔r9 ABI

**Phase:** 04-r9-userspace-exec-harvey-r9-abi
**Consolidated:** 2026-11-02 (plan 04-08, task 3)
**Status:** complete — all four phase success criteria have execution evidence

## Pinned commits (D-13 ordering: r9 first, Harvey evidence second)

| Repo | SHA | Role |
|------|-----|------|
| r9 (`requaos/r9`, branch `main`) | **`54e8c1079933aa8be7d3b1571702a408169e1950`** | the phase's final r9 HEAD — WR-08 fix (the last of 9 code-review fix commits; see 04-REVIEW-FIX.md) |
| r9 | `b4f858abe015a66f403290580970c94a5e57a835` | pre-fix phase HEAD (04-08 close-out); superseded by the 04-REVIEW fix pass above |
| r9 | `dac3b33923fba47107bfea10101d224283ee018b` | 04-08 Task 3: HowItWorks notes tour + README profile rows (docs-only) |
| r9 | `c464ea2ebc08fee3ba65e95bbcb74c862da50946` | 04-08 Task 1: devproc-lite + NOTIFY/NOTED + boundary delivery |
| r9 | `fae19fc2c5c1ccedec810dbf8b0bbb86eabf0cd2` | 04-08 Task 2: virtnotes profile (the SC#4 gate ran on this exact tree) |
| harvey | `c72d0631419b80a30b4a990245ab2e60f2192d82` | 04-08 Task 2: smokenote + init's final job + the SC#4 evidence log |

The SC#4 gate log (`04-08-sc4-boot.log`) was produced by the r9 tree at
`c464ea2` + `fae19fc` (the docs commit `dac3b33` and the whitespace-only
style fix `b4f858a` change no compiled code — same kernel image, matching
04-06's docs-on-top precedent; the gate was re-run green at the final
HEAD). After consolidation, the `/gsd-code-review 4 --fix` pass (9/9
findings fixed; 04-REVIEW-FIX.md) moved r9 HEAD to `54e8c10`; every gate
was re-run green at the fix HEAD (port 26/26 + xtask 19/19 host tests,
ureg-check 35/35, and all five boot profiles — virtexec, virtmulti,
virtnotes, virt, virtpanic). Evidence-run
bootfs.paq sha256: `c04421d2ae4f924c6671f8736f7a696e430a712ba7ba2581653c827b65cd286c`
(per-bake digest — mkpaqfs embeds a time field).

## Success-criterion → evidence mapping

### SC#1 — Static ABI enforcement (plan 04-03)

The trap frame IS Harvey's `Ureg` field-for-field; drift is a build failure,
not a runtime mystery:

- The published contract: `aarch64/ABI.md` "Kernel-side enforcement (Phase 4
  appendix)" — pinned offsets (r0@0 … type@272), `sizeof(Ureg)`=280 vs r9's
  288 stride, the arg-read rule (`(Sargs*)(ureg->sp + 8)`).
- Layer 1: in-crate const asserts in r9 `aarch64/src/trap.rs`
  (`core::mem::offset_of!` per pinned offset + `size_of::<TrapFrame>() == 288`)
  — always on, no Harvey checkout needed.
- Layer 2: `cargo xtask ureg-check` parses the sibling Harvey checkout's
  `aarch64/include/ureg.h` (field names/order/count) and fails every boot-test
  profile on drift.  Negative control proven in 04-03: a drifted header
  produces a named-field expected-vs-found diff and a non-zero exit.

### SC#2 — hello runs on r9 (plan 04-06)

The committed Phase-2 ABI-bisect binary `build/aarch64/hello.7.out`
(sha256 `1c89e0990e386ae503d9f513d3a6c1985222f772d1edb40016c19aa3881abb04`)
was exec'd by the staged init on r9 under qemu virt and printed via the real
EL0 svc path.  Evidence: `04-06-sc2-boot.log` (committed), quoted verbatim:

```
455:exec: entering EL0 pc=0x10068 sp=0x7fffffffffd8
461:init: r9 EL0 alive
466:hello aarch64 harvey
469:all user processes exited
```

Zero `panic`/`Unhandled interrupt` lines; the marker string is asserted
present in the staged binary's own rodata (evidence honesty — the marker
reached serial only through the binary's own pwrite syscall path).

### SC#3 — Demand-paged first process (plan 04-05)

Nothing is mapped at exec; the first instruction fetch faults and the user
pager pulls text/data from the initrd paqfs on demand.  CTR_EL0 probed
geometry quoted from the boot log (never hardcoded): `ctr_el0: 0x8444c004`.
Evidence lines (the same chain is carried by the SC#2/SC#4 logs):

```
464:ctr_el0: 0x8444c004
466:exec: entering EL0 pc=0x101f4 sp=0x7fffffffffd8
467:exec: ttbr0 switched, console alive        ← D-10 teardown: TTBR0 identity map replaced, MMIO provably TTBR1-side
457:fault: text page-in va=0x10000             ← 04-06-sc2-boot.log — demand page-in markers
```

### SC#4 — Syscall smoke suite from EL0, panic-free (plans 04-07 + 04-08)

The smoke suite (rfork, notes, basic fd ops) runs from EL0 under visible
preemption with zero kernel panics.  Evidence: `04-08-sc4-boot.log`
(committed; `cargo xtask boot-test --arch aarch64 --config virtnotes`),
quoted verbatim with line numbers:

```
472:init: r9 EL0 alive
491:smokefork: fd ok                    ← fd-ops smoke (open/pread/close /hello's R_MAGIC header)
493:smokefork: child running          ← rfork'd grandchild
496:hello aarch64 harvey
502:smokenote: got note smoke         ← the note, posted by the parent's unmodified libc
                                       postnote(PNPROC, pid, "smoke") → /proc/4/note write
                                       through devproc-lite, delivered at the child's
                                       return-to-EL0 boundary (tick preemption) with pc
                                       redirected into the notify()-installed handler
503:smokenote: resumed                ← noted(NCONT) restored the saved 280-byte Ureg
                                       over the live frame; the spin resumed and observed
                                       the handler's flag
506:init: reaped 2 0 0 0 ""
507:init: reaped 4 0 0 0 "note-ok"
508:smokefork: await: 5 0 0 0 "kid"
511:init: reaped 3 0 0 0 ""
512:init: smoke complete
515:all user processes exited
```

Preemption evidence: **7 `sched: switch old->new` lines** (tick-driven
round-robin; IRQ-from-EL0 returned to a DIFFERENT process — the note was
delivered on exactly such a boundary).  Negative scans on the log: zero
`panic`/`Unhandled interrupt` lines, zero `note-timeout` (delivery never
failed), zero `syscall N unimplemented` lines (the measured set has no gap).

## The D-14 measured syscall set and its coverage

Measured on the 9front donor kernel at pin
`350266104f1fed81c78c3d05fa44f523bec0ae4d` (04-01-d14-trace.txt, verbatim):

```
51 pwrite
14 open
50 pread
4 close
8 exits
```

The implemented dispatch table (r9 `aarch64/src/syscall.rs`,
`static DISPATCH: [Option<fn>; 55]` over the frozen sys.h 0–54):

| Number | Name | Provenance |
|--------|------|-----------|
| 51 | PWRITE | measured trace |
| 14 | OPEN | measured trace |
| 50 | PREAD | measured trace |
| 4 | CLOSE | measured trace |
| 8 | EXITS | measured trace |
| 7 | EXEC | D-14 named core |
| 19 | RFORK | D-14 named core (SC#4) |
| 47 | AWAIT | D-14 named core (SC#4) |
| 28 | NOTIFY | D-14 named core (SC#4) |
| 29 | NOTED | D-14 named core (SC#4) |
| 41 | ERRSTR | measured caller (libc `%r` — observed in the first virtmulti run, 04-07) |

The trace lists NO brk_(24)/segbrk(12) (the fmt/pool watch item settled by
evidence), so they stay None.  Every other number prints
`syscall N unimplemented` and returns -1 — loud, never wedged.

## Wave-0 falsification results (plan 04-01)

- **Initrd delivery (D-01 amended):** qemu 11.1.2's `-initrd` never delivers
  for ELF kernels (assumption falsified); delivery is qemu's generic loader
  at a TOML-data PA + an xtask-produced fdtput-patched DTB copy carrying
  `/chosen` `linux,initrd-start/end`.  Proven on serial:
  `initrd: start=0x48000000 end=0x48000192 size=402` parsed from /chosen,
  validated against the DTB memory node, reserved in pagealloc.
- **paqfs producer + fixture:** host mkpaqfs built in-tree
  (`build/aarch64/mkpaqfs-host.sh`); committed byte-verified fixture
  `r9/port/tests/fixtures/probe-tree.paq`, sha256
  `27b7482ed68562c891a87ee6a3f4ca31633570b9f61de6d1a902eb61ccd21300`
  (header magic 0x529AB12B @0, trailer 0x6B46E688 @ EOF-28; one fixture
  exercises NoEnc + DeflateEnc).

## Regression matrix (all green at the evidence commits)

| Leg | Command | Result |
|-----|---------|--------|
| virt profile | `cargo xtask boot-test --arch aarch64 --config virt` | PASS (banner, ≥3 ticks, echo) |
| virtpanic profile | `cargo xtask boot-test --arch aarch64 --config virtpanic` | PASS (banner, start marker, ≥30 register rows, clean halt) |
| virtexec profile (SC#2) | `cargo xtask boot-test --arch aarch64 --config virtexec` | PASS (ordered chain to all-exited) |
| virtmulti profile | `cargo xtask boot-test --arch aarch64 --config virtmulti` | PASS (smoke markers + ≥1 sched switch) |
| virtnotes profile (SC#4) | `cargo xtask boot-test --arch aarch64 --config virtnotes` | PASS ×3 consecutive runs |
| raspi3b build | `cargo xtask build --arch aarch64` | green (no EL0 regression — raspi has no userland this phase) |
| raspi4b build | `cargo xtask build --arch aarch64 --config raspi4b` | green |
| clippy | `cargo xtask clippy --arch aarch64 --config virtexec` | green (no new warnings) |
| ureg-check | runs as a boot-test precondition in every profile | green (SC#1 live on every run) |

## Phase 5 inheritance

- A staged-init pattern to grow into real `/bin/init` (namespace/auth/boot rc).
- A proven harness: five boot-test profiles over one zero-dep unix-socket
  serial capture, marker-presence (never order) assertions, and an always-on
  negative scan.
- An ABI contract neither repo can drift from silently (const asserts +
  ureg-check gate).
- The notes machinery (devproc-lite + NOTIFY/NOTED + boundary delivery) and
  the swtch-based blocking-syscall path (AWAIT) that real init stands on.
