# Phase 4 second end-of-phase owner stop (gap closure) — results

Plan 04-25, Task 2. The owner ran the corrected runner on the x86_64 Linux host and returned
the logs through the branch under `evidence/owner-stop-2/return/round2/` (commit `194c71d`).
Every verdict below is decided only from a line in those returned logs or from the git checks
this task names; no step was run on the Mac. All eleven verdicts are PASS and none is FAIL, so
the stop rules do not apply and Task 3's `main`-push decision follows.

- branch: `phase-04/aead-wire-gaps`
- round: 2 of 2 — this round supersedes round 1's return (see Disclosures)
- `origin/main` is `5d1549af07de7220d6f48aac0d0db221aa7c541c`, unchanged by this task
- result: **11 of 11 verdicts PASS**

pushed-tip 0dd1037dfe2067778fe4f731f5b30ea3d355d348

gated-commit 0dd1037dfe2067778fe4f731f5b30ea3d355d348

hash-commit none

old-outputHash sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=

new-outputHash unchanged

## Verdicts

VERDICT stop2-branch-push: PASS
EVIDENCE stop2-branch-push: `REPORT.txt` line `pushed-tip:
0dd1037dfe2067778fe4f731f5b30ea3d355d348` with `host-tip:
0dd1037dfe2067778fe4f731f5b30ea3d355d348`, and every round-2 log begins
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`; `git rev-parse
origin/phase-04/aead-wire-gaps` is `194c71d3cdf865a98132e3edb53775ce9f1d6eee`, the logs-only
return commit of section 9, whose parent (`origin/phase-04/aead-wire-gaps^`) is
`0dd1037dfe2067778fe4f731f5b30ea3d355d348` — the branch the checks ran on, with no hash commit
because the committed hash reproduced.

VERDICT stop2-host-test: PASS
EVIDENCE stop2-host-test: `host-tests.log` `HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`,
every `test result:` line `0 failed`, the final one
`test result: ok. 140 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in
0.00s`, then `EXIT=0`.

VERDICT stop2-host-test9p: PASS
EVIDENCE stop2-host-test9p: `host-test-9p.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`, the `aead_*` legs run on x86_64 (the last
being `Running tests/aead_negotiate.rs (target/debug/deps/aead_negotiate-f57b1c9a8f304d85)`),
then `EXIT=0`.

VERDICT stop2-kernel-x64-build: PASS
EVIDENCE stop2-kernel-x64-build: `tip-kernel-x64.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`, then
`building '/nix/store/8rwc8mgpnmcdnnl567p9z7lpdfhb19qs-r9-kernel-x64-0dd1037.drv'...`, then
`EXIT=0` — the sandboxed `kernel-x64` derivation builds with the gap-closure `port` code.

VERDICT stop2-kernel-virt-build: PASS
EVIDENCE stop2-kernel-virt-build: `tip-kernel-virt.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`, then
`building '/nix/store/dypvcs2cl4rsq865hbiwi3gxl7czbb0s-r9-kernel-virt-0dd1037.drv'...`, then
`EXIT=0` — the sandboxed `kernel-virt` derivation builds with the gap-closure `port` code.

VERDICT stop2-bootiso-hash-commit: PASS
EVIDENCE stop2-bootiso-hash-commit: no change required — the round-2 runner forced the rebuild
that a plain build cannot make: `boot-iso-first.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`, then
`building '/nix/store/p4ivjdb0yph97bm0vwm5bsv85ynwbah5-r9-boot-iso-0dd1037.drv'...`, then
`EXIT=0`, and `boot-iso-rebuild.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`, then
`checking outputs of '/nix/store/p4ivjdb0yph97bm0vwm5bsv85ynwbah5-r9-boot-iso-0dd1037.drv'...`,
then `EXIT=0`. The committed `sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=` reproduces
from the gap-closure kernel, so no commit was made (`REPORT.txt`: `hash-commit: none`,
`new-outputHash: unchanged`); plan 04-25 Task 1 allows "no change". No hash commit exists.

VERDICT stop2-bootiso-build: PASS
EVIDENCE stop2-bootiso-build: `boot-iso-first.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`, then
`this derivation will be built:` / `building
'/nix/store/p4ivjdb0yph97bm0vwm5bsv85ynwbah5-r9-boot-iso-0dd1037.drv'...`, then `EXIT=0` — the
ISO builds green with the committed hash, its derivation named for the pushed tip.

VERDICT stop2-bootiso-rebuild: PASS
EVIDENCE stop2-bootiso-rebuild: `boot-iso-rebuild.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`, then
`checking outputs of '/nix/store/p4ivjdb0yph97bm0vwm5bsv85ynwbah5-r9-boot-iso-0dd1037.drv'...`,
then `EXIT=0`, with no `hash mismatch` and no `got:` line — the committed value is the built
value.

VERDICT stop2-gate-kvm: PASS
EVIDENCE stop2-gate-kvm: `gate-kvm.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`:
`GATE PASS quick x86_64 run=88132-1795622365`, then `EXIT=0` — boot to rio under KVM.

VERDICT stop2-gate-tcg: PASS
EVIDENCE stop2-gate-tcg: `gate-tcg.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`:
`GATE PASS quick x86_64 run=98267-1795622561`, then `EXIT=0` — run natively on the owner's
x86_64 host, as the kernel byte-change rule requires.

VERDICT stop2-gate-full: PASS
EVIDENCE stop2-gate-full: `gate-full.log`
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348`:
`GATE PASS full x86_64-linux run=108390-1795622765`, then `EXIT=0` — the composed auth gate
passed and the capture was retained.

## Why the ISO hash did not move

The gap fixes did move the Mac debug ELFs: `q35churnshell` went
`c850fb6408261299713894af9917c3b55c776ee26130295239bea9cd20748957` at `@04-14` to
`2066521c55e01eca9914605d120561e67d6cfc3dfcbca3416a6de81e20f24928` at `@04-24`
(`evidence/kernel-bytes/after-04-24.sha256`, `KERNEL-BYTES-GAPS.md`). What did not move is the
code itself: the defined text-symbol totals are unchanged (`755220` aarch64, `760137` x86_64)
and the FP/SIMD scan counts are unchanged (`309` and `6`), on both sides of the change. The
whole-ELF delta is therefore debug information — DWARF, line tables and file paths — which the
sandboxed kernel derivation does not carry into the image. That is why the Linux-built
`kernel-x64` produces ISO bytes identical to the ones the first stop hashed, and why the
committed `outputHash` still reproduces. Round 2 does not merely assert this: the v2 runner
always runs `nix build .#boot-iso --rebuild`, so `boot-iso-rebuild.log` at
`HEAD=0dd1037dfe2067778fe4f731f5b30ea3d355d348` actually checks the outputs
(`checking outputs of '/nix/store/p4ivjdb0yph97bm0vwm5bsv85ynwbah5-r9-boot-iso-0dd1037.drv'...`)
and exits `0` — the round-1 conclusion, which rested on a cached fixed-output path, is replaced
by a real build and rebuild that agree.

## Disclosures

1. **`bdee8a8` — round-1 return logs (superseded).** The first round's `boot-iso-first.log`
   ended `EXIT=0` at `HEAD=611dbb4...`, but a plain `nix build .#boot-iso` returns the
   fixed-output path already in the store from the first stop, so that conclusion rested on a
   cached fixed-output path and could report "no hash change" without rebuilding. Round 2's
   runner (`0dd1037`) forces `--rebuild`; **round 2 supersedes round 1's return for every
   verdict in this file.**
2. **`0dd1037` — tooling-only runner fix, before section 5.** This is the pushed tip every
   round-2 log measures. Its diff touches only
   `.planning/phases/04-guide-phase-2-9p2000-aead-transport-and-fidtable/04-OWNER-STOP-2-RUNNER.sh`
   — planning tooling, not source. `git diff --quiet 611dbb4 194c71d -- port crypto ds xtask
   aarch64 x86_64 riscv64 Cargo.lock flake.nix` exits 0, so no kernel byte moved between the
   round-1 tip and this round's tip, and the hash conclusion is measured on unchanged source.
3. **`194c71d` — round-2 return logs.** Adds only the ten files under
   `evidence/owner-stop-2/return/round2/`, as section 9 allows; it is the only commit above the
   pushed tip and changes no source.
4. **No hash commit.** `REPORT.txt` records `hash-commit: none` and `new-outputHash:
   unchanged`; `origin/phase-04/aead-wire-gaps` holds `0dd1037` plus the return commit only.
   Nothing here claims a `flake.nix` commit exists.

## Notes

- Each verdict log begins with its `HEAD=` line and ends with its `EXIT=` line.
- No `hash mismatch` and no `got:` line appears in any round-2 log.
- No credential string appears in the returned logs; nothing needed redaction.
- The gates ran at `0dd1037dfe2067778fe4f731f5b30ea3d355d348`, the same commit the kernel and
  ISO builds measured, so the one-commit rule holds for this round: no commit reached the
  branch between section 5 and section 8.

VERDICT stop2-main-push: PASS bb68d988752ccb616a980b943ea86aecca8f25be
