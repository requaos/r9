# USPACE-10 — 9k-amd64 pool-panic re-test verdict record (06-08, D-04)

**The v1.0 carry-over is closed WITH INFORMATION: the pswt-fixed 6c boots
9k10cpu PAST the 01-04 pool-panic point.**  See "The verdict" below.

## Origin

v1.0 phase-01 deferred-items #2: the fixed 9k10cpu kernel booted under TCG
to the `Plan 9` banner, initialized MMU/APIC/PCI, printed "508M memory:
1936K+230M kernel, 276M user", then the embedded-root `boot` process
reported **"bad magic"** and died in a pool panic (`panic: pool panic`,
`boot 1: suicide: sys: trap: fault read addr=0x0 pc=0x20ac98`).  Phase 2
root-caused the same signature on the 9front donor kernel to cc/pswt.c's
LP64 switch-sort miscompile and fixed it (`7efff3aa3`); the 9k-amd64
re-test with the fixed 6c was deferred (02-8) and landed here as USPACE-10
(D-04: verdict recorded either way — a FAIL verdict closes the requirement
with information; the harness asserts verdict PRODUCTION, never the
outcome).

## The harness

`build/mm/boot-9k10cpu-qemu.sh` — minimega-free (D-03) plain
`qemu-system-x86_64 -kernel amd64/9k10cpu.elf -m 512 -display none
-no-reboot -serial file:<log>`; capture truncated before launch;
accel probed exactly once per run (`/dev/kvm` writable → kvm/-cpu host,
else tcg/-cpu qemu64, 600s budgets, `PP_BOOT_BUDGET`/
`PP_CONTINUE_BUDGET` overrides); deadline-wait for the `Plan 9` banner;
then the WHOLE log is classified (never a post-banner tail, so an
interleaved panic is still caught).

- **Signature set:** `bad magic`, `panic`, `suicide: sys: trap` (the
  recorded 01-04 incident; "pool panic" does not exist verbatim in the
  9k tree — the robust set is used).
- **Healthy continuation markers (what the production boot binary
  actually prints past the panic point):** `root is from` (boot.c:342's
  method prompt — the panicked 01-04 boot died before it),
  `version...` (boot.c:139, post-rootfs-connection), `argc=`
  (boot.c:69, DEBUG-gated builds only).  Any one suffices.  (The plan
  named argc=/version...; both are unreachable in the production build
  — see the 06-08 SUMMARY's Rule-1 deviation.)
- **Verdict contract:** `VERDICT=PASS|FAIL <classification>` +
  banner/signatures/healthy_markers/accel/kernel/log/commit/date fields
  in `$VERDICT_OUT` (default `.build/poolpanic-verdict.txt`); exit 0 iff
  a verdict was produced (banner seen + classification written) — NEVER
  gated on PASS.  An empty capture records `FAIL no-banner` and exits
  non-zero (the CI leg never greens on an empty log).

## Teeth runs (classification paths proven, 2026-11-05, darwin-aarch64)

| Teeth | Input | Result | Exit |
|---|---|---|---|
| A | incident-shaped canned log (banner + all three signatures) | `VERDICT=FAIL pool-panic-signature` (sigs="bad magic; panic; suicide: sys: trap") | 0 — production-not-outcome proven |
| B | empty log | `VERDICT=FAIL no-banner` | 1 — never greens on an empty capture |
| C | healthy canned log (argc=/version...) | `VERDICT=PASS no-panic-signature boot-continues` | 0 |
| D | deliberately wrong kernel image through REAL qemu (`invalid kernel header`, exited at 5s, empty serial) | `VERDICT=FAIL no-banner` | 1 — end-to-end no-banner discipline via the real launch path |
| E | production-build healthy shape (banner + `root is from` prompt loop) | `VERDICT=PASS no-panic-signature boot-continues` (healthy_markers=yes) | 0 |

## The verdict

**LOCAL (darwin-aarch64, qemu 11.1.2 TCG, kernel artifact
`amd64/9k10cpu.elf` — ELF built locally from this checkout, which carries
the pswt fix `7efff3aa3` since Phase 2):**

```
VERDICT=PASS no-panic-signature boot-continues
banner=yes
signatures=none
healthy_markers=yes
accel=tcg
kernel=/Users/neilskinner/code/harvey/amd64/9k10cpu.elf
log=/Users/neilskinner/code/harvey/.build/poolpanic-serial.log
commit=c8f542246371076d65e4d11415b5ac14d5cac0ef
date=2026-11-05T12:55:38Z
```

Retained serial capture: [06-08-poolpanic-serial.log](06-08-poolpanic-serial.log)
(run 2 — the committed evidence; zero `bad magic`/`panic`/`suicide` lines,
one `root is from` marker line + the configip reprompt loop).

Serial log highlights (run 2 capture): banner `Plan 9` at ~5s; MMU/APIC/PCI
init; "508M memory: 1936K+230M kernel, 276M user" (the 01-04 incident's
last healthy line); then — **no "bad magic", no panic, no suicide** — the
boot process continued into interactive rootfs selection:
`root is from (tcp)[tcp]: ` (boot.c:342), looping `configip: can't parse
fs ip` reprompts (no console input is attached on this capture-only serial
— expected; the window closes on the marker, qemu is torn down).

**Reading:** the boot sailed past the exact point where the 01-04 incident
died.  The pswt.c LP64 switch-sort fix (Phase 2, `7efff3aa3`) cured the
9k-amd64 bad-magic pool panic — the same root cause as the donor kernel's,
as Phase 2's audit hypothesized.  The remaining prompt loop is boot(8)'s
normal no-input behavior on a capture-only serial, not a defect.

**Run 1 (superseded, retained for honesty):** the same boot classified by
the pre-fix marker set recorded `VERDICT=PASS no-panic-signature
banner-only` (healthy_markers=no) — the plan-named markers
(argc=/version...) are unreachable in the production build (DEBUG-gated /
post-connection).  The Rule-1 marker correction (`root is from`) reclassified
the SAME boot behavior as boot-continues; both verdict files are retained
(`.build/poolpanic-verdict-run1.txt` — gitignored scratch; this file is the
committed record).

## Hosted run — PENDING FIRST PUSH (05-13 precedent)

The `x86_64-poolpanic` job in `.github/workflows/phase6.yml` runs this
exact harness on ubuntu-latest: `nix run .#build` builds 9k10cpu from the
CLEAN CHECKOUT (guaranteeing the pswt-fixed 6c independent of local
hostbin state), the accel probe selects KVM (`/dev/kvm` is exposed on
GitHub's x86_64 runners — the D-04 premise cited in 06-RESEARCH) with the
TCG fallback documented in-job, and the serial log + verdict file upload
as artifacts.  harvey master is never pushed from this session, so no
hosted run exists yet: **the KVM-classified hosted verdict lands at first
push** — watch `gh run list --workflow phase6-userspace` / the repo's
Actions tab.  Nothing here fabricates that run.
