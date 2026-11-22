# Deferred Items — Phase 01

## From plan 01-04 (minimega smoke harness / BUILD-05)

### 1. Dormant-arch toolchain siblings share the LP64 wide-constant bug

The `long`-truncation-of-`vlong` bug class fixed for amd64 in 01-04
(6c/swt.c zaddr, 6l/span.c oclass, 6c/cgen.c immconst) also exists in
dormant-arch tools, not fixed this phase (out of scope: amd64-only green
gate; these tools are not built into the Phase 1 toolchain path):

- `sys/src/cmd/9c/swt.c:486` — `(vlong)l != a->offset` (power64 compiler)
- `sys/src/cmd/4c/swt.c:462` — same (mips32 compiler)
- `sys/src/cmd/9a/lex.c:644` — same (power64 assembler)
- `sys/src/cmd/9l/list.c:207` — same (power64 linker object reader/writer)
- `sys/src/cmd/4l/span.c:188` / `sys/src/cmd/9l/span.c:150` —
  `(vlong)l == v` oclass fit test (mips32/power64 linkers)
- `sys/src/cmd/6a/lex.c:1467` has the same TEXT but is safe: its `l` is
  `int` (always 32-bit), so the truncation detector works on all hosts.
  8c/8a do not carry the pattern (386 object format has no T_64 path).

If a dormant arch is revived (or Phase 2 imports 7c/7l for aarch64), audit
for the same idiom before trusting output. Detection signature to grep:
`long l;` adjacent to `(vlong)l` comparisons or `= a->offset`.
  status: acknowledged

### 2. Post-banner boot-process pool panic (observed, not chased)

The fixed 9k10cpu kernel boots under TCG to the `Plan 9` banner (BUILD-05's
assertion), initializes MMU/APIC/PCI correctly ("508M memory: 1936K+230M
kernel, 276M user"), then the embedded-root `boot` process reports
"bad magic" and dies in a pool panic (`panic: pool panic`,
`boot 1: suicide: sys: trap: fault read addr=0x0 pc=0x20ac98`).

Out of BUILD-05's scope (banner criterion met), but it means the
boot-to-userspace handoff is not yet healthy. Possibly further LP64-era
toolchain miscompiles in code paths not exercised by early boot, or an
embedded-root/boot(8) issue. Investigate before Phase 2 relies on
boot-to-shell as a verification signal. Evidence: boot-9k10cpu.log
(gitignored runtime artifact; regenerate via build/mm/boot-9k10cpu.sh).

### 3. 01-03 latent regression note (fixed in 01-04)

01-03's KERNDATE line in nix/build.sh + nix/vm-build.sh tripped
writeShellApplication's shellcheck (SC2015), so any fresh app build of
`.#build` failed; and the `date +%s` fallback resolves to plan9port's date
(hostbin first on PATH) which prints garbage for GNU-style format args —
the guest's no-git fallback produced a date STRING, not an epoch. Both
fixed in 01-04 (if-block + numeric-validated `date -n`/`+%s` fallback).
01-03's verification used direct namespace runs, so the app-level failure
was invisible until 01-04 exercised `nix run .#build` fresh.
