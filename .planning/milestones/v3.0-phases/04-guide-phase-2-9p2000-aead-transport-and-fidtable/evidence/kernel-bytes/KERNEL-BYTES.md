# Kernel bytes after Phase 4 links `crypto` and `ds` (plan 04-14)

Phase 4 makes `port` depend on `crypto/` and `ds/` and links the new wire code into both
kernels. The dependency edge alone changes the whole-ELF bytes (Phase 2's "`.text` identical"
rule is **not** used here — D-29), so this file records the replacement evidence set: whole-ELF
sha256 before and after on one venue, defined text symbol byte totals before and after, the
FP/SIMD disassembly scan counts against the baseline, and the x86_64 text symbol total under
2,097,152 bytes. The verdict lines are at the end.

- Base commit (before, from `before.sha256`): `3ce5ed8c4e309710e2157ab5b63a03a40797d185`
- After commit (`git rev-parse HEAD` when the after builds were made):
  `3398df49499d1b877fcf0ac92fa54c0872df9c71` — the Task 2 approval commit. It is the commit the
  owner reviewed (`f72cc2a`) plus the `.planning`-only approval commit; no source file differs
  between the two (Task 3 precondition checked `git diff --quiet f72cc2a HEAD -- port crypto ds
  xtask aarch64 x86_64 Cargo.lock`, clean).
- Venue: `Darwin arm64` (both the before and after builds were made on this one machine;
  `uname -sm` was checked against the `venue` line of `before.sha256` before the builds)
- The four-line record is `after-04-14.sha256`, in the baseline's format with the `after` line
  first; its `venue` line equals the baseline's.

Both kernels were rebuilt from nothing: `target/aarch64-unknown-none-elf` and
`target/x86_64-unknown-none-elf` were moved aside to
`target/phase04-kernel-bytes/after-targets/` before the after builds, then rebuilt with
`nix develop -c cargo xtask build --arch aarch64 --config virtshell` and
`nix develop -c cargo xtask build --arch x86-64 --config q35churnshell` (both exit 0, logs
under `target/phase04-closing/kernel/`, gitignored).

## Before and after

| Config | Before (`3ce5ed8`) | After (`3398df4`) | Same or different |
|--------|--------------------|-------------------|-------------------|
| `virtshell` | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | `0ae5d5745632065371f99c0db1cae2bfe2c88d58b043b9e160f991035a43dd6a` | different |
| `q35churnshell` | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | `c850fb6408261299713894af9917c3b55c776ee26130295239bea9cd20748957` | different |

| Config | Text symbol bytes before | Text symbol bytes after | FP/SIMD scan lines before | FP/SIMD scan lines after |
|--------|-------------------------|-------------------------|---------------------------|--------------------------|
| `virtshell` | 754232 | 755220 | 309 | 309 |
| `q35churnshell` | 759072 | 760137 | 6 | 6 |

The whole-ELF bytes differ, as expected (D-25): the dependency edge changes `port`'s crate hash
and link order, so `.text` contents move even where the code is the same. `IDENTICAL` would
mean `port` did not link the new code and is reported; here it is `DIFFERENT`.

The text symbol total moved by +988 bytes on aarch64 and +1,065 on x86_64. The x86_64 total of
760,137 bytes leaves 1,337,015 bytes of headroom under the 2,097,152-byte (2 MiB) unit, so
`.text` stays one 2 MiB section and no later section moves (Pitfall 9).

## Measurement integrity (D-29)

The four `METRIC-COMMAND` lines are copied unchanged from `before.metrics`; each reads the bare
repo-relative build output — `target/aarch64-unknown-none-elf/debug/aarch64` for `virtshell`,
`target/x86_64-unknown-none-elf/debug/x86_64` for `q35churnshell` — and none names an absolute
path, a worktree path, or a copy under `target/phase04-kernel-bytes/`. `shasum -a 256` of each
build output was taken immediately before and immediately after the four commands ran; both
hashes of a path are equal to each other and to that config's `elf` line in
`after-04-14.sha256`. `after.metrics` records the measured hash and path per config on its
`MEASURED` lines. The baseline copies under `target/phase04-kernel-bytes/before/` were not
read.

A Mac devshell hash is **not** comparable with the Linux `kernel-x64.elf` built at the owner
stop (`nix build .#kernel-x64`): the kernel `.rodata` carries toolchain store-path strings, and
the toolchain derivation differs per system. Only this before/after comparison, made on one
venue, is evidence; the Linux byte comparison and the `boot-iso` `outputHash` re-derivation are
the owner stop's (`../04-OWNER-STOP.md`).

## Verdicts

VERDICT elf-virtshell@04-14: DIFFERENT
VERDICT elf-q35churnshell@04-14: DIFFERENT
VERDICT text-symbols-virtshell@04-14: RECORDED before=754232 after=755220
VERDICT text-symbols-q35churnshell@04-14: RECORDED before=759072 after=760137
VERDICT fpsimd-virtshell@04-14: EQUAL
VERDICT fpsimd-q35churnshell@04-14: EQUAL
VERDICT text-under-2mib-q35churnshell@04-14: PASS
