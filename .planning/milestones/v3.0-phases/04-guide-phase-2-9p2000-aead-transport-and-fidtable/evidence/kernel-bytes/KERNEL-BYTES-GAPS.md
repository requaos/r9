# Kernel bytes after the Phase 4 gap closure (plan 04-24)

The gap closure fixed the three re-key criticals (CR-01, CR-02, CR-03) and five blocking
warnings under `port/src` after the first owner stop had already gated. Every one of those fixes
lands in code linked into both kernels, so the byte evidence plan 04-14 recorded at `3398df4` no
longer describes the tree the owner reviewed in plan 04-23. This file is the replacement
evidence set for the **gap-closure kernel**: whole-ELF sha256 before and after on one venue,
defined text symbol byte totals against the baseline and against `@04-14`, the FP/SIMD
disassembly scan counts against the baseline, and the x86_64 text symbol total under 2,097,152
bytes. The verdict lines are at the end.

**This supersedes the `@04-14` set in `KERNEL-BYTES.md` for the gap-closure kernel.** That file
and its `after-04-14.sha256` and `after.metrics` are kept as history and are not rewritten; they
describe the kernel as it stood before the gap fixes. The reason is stated in one line: every
fix under `port/src` changes kernel bytes, so a set recorded before those fixes cannot be the
evidence for the kernel that contains them.

- Base commit (before, from `before.sha256`): `3ce5ed8c4e309710e2157ab5b63a03a40797d185`
- `@04-14` after commit (from `after-04-14.sha256`): `3398df49499d1b877fcf0ac92fa54c0872df9c71`
- `@04-24` after commit (`git rev-parse HEAD` when the after builds were made):
  `8df21a37dc15f637337057453aa470a7d407aede` — the commit that carries the owner-approved
  gap-closure `port/src` change (reviewed at `6fd201c`, `VERDICT port-src-review-gaps: APPROVED`)
  plus `.planning`-only commits. No source file differs between the reviewed sha and this commit:
  `git diff --quiet 6fd201c HEAD -- port crypto ds xtask aarch64 x86_64 riscv64 Cargo.lock
  flake.nix` exits 0, and the same check including `rust-toolchain.toml` and `.cargo` exits 0.
- Venue: `Darwin arm64` (all of the before, `@04-14` and `@04-24` builds were made on this one
  machine; `uname -sm` was checked against the `venue` line of `before.sha256` before the builds)
- The four-line record is `after-04-24.sha256`, in the baseline's format with the `after` line
  first; its `venue` line equals the baseline's.

Both kernels were rebuilt **from nothing**: `target/aarch64-unknown-none-elf` and
`target/x86_64-unknown-none-elf` were moved aside to
`target/phase04-kernel-bytes/pre-04-24-targets/` before the after builds, then rebuilt with
`nix develop -c cargo xtask build --arch aarch64 --config virtshell` and
`nix develop -c cargo xtask build --arch x86-64 --config q35churnshell` (both exit 0, logs
`target/phase04-closing2/build-aarch64.log` and `build-x86_64.log`, gitignored).

## Baseline, `@04-14` and `@04-24`

| Config | Baseline (`3ce5ed8`) | `@04-14` (`3398df4`) | `@04-24` (`8df21a3`) | `@04-24` vs `@04-14` |
|--------|----------------------|----------------------|----------------------|----------------------|
| `virtshell` ELF | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | `0ae5d5745632065371f99c0db1cae2bfe2c88d58b043b9e160f991035a43dd6a` | `2ee30a2e6a6ef31f9b784b1290bf64031656897c6d2dc01db2284946c727dcee` | different |
| `q35churnshell` ELF | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | `c850fb6408261299713894af9917c3b55c776ee26130295239bea9cd20748957` | `2066521c55e01eca9914605d120561e67d6cfc3dfcbca3416a6de81e20f24928` | different |

| Config | Text symbol bytes, baseline | `@04-14` | `@04-24` | FP/SIMD scan lines, baseline | `@04-14` | `@04-24` |
|--------|-----------------------------|----------|----------|------------------------------|----------|----------|
| `virtshell` | 754232 | 755220 | 755220 | 309 | 309 | 309 |
| `q35churnshell` | 759072 | 760137 | 760137 | 6 | 6 | 6 |

The whole-ELF hashes differ from `@04-14` on both configs, as expected: the gap fixes are linked
into both kernels, so at least one byte of each image moved. `IDENTICAL` would mean the fixes
are not in the kernel and is reported; here it is `DIFFERENT` on both.

The defined text symbol totals are unchanged from `@04-14` (`755220` aarch64, `760137` x86_64)
while the ELF hashes differ. That is recorded as measured, not interpreted: the metric sums the
sizes of defined `T`/`t` symbols, and the gap-closure edits do not move that sum on either
config, whereas the whole-ELF bytes — which include everything outside those symbol sizes — do
move. The evidence that the fixes are in the kernel is the `DIFFERENT` ELF verdict at a commit
whose source diff against the reviewed sha is empty, not the text total.

The x86_64 text symbol total of 760,137 bytes leaves **1,337,015 bytes of headroom** under the
2,097,152-byte (2 MiB) unit, the same headroom `@04-14` reported, so `.text` stays one 2 MiB
section and no later section moves (Pitfall 9).

## Measurement integrity (D-29)

The four `METRIC-COMMAND` lines in `after-04-24.metrics` are copied unchanged from
`before.metrics`; each reads the bare repo-relative build output —
`target/aarch64-unknown-none-elf/debug/aarch64` for `virtshell`,
`target/x86_64-unknown-none-elf/debug/x86_64` for `q35churnshell` — and none names an absolute
path, a worktree path, or a copy under `target/phase04-kernel-bytes/`. Before any command ran,
all four were checked against that requirement (4 of 4 name the build output). `shasum -a 256`
of each build output was taken immediately before and immediately after the four commands ran;
both hashes of a path are equal to each other (`STABLE=yes`) and to that config's `elf` line in
`after-04-24.sha256`. `after-04-24.metrics` records the measured hash and path per config on its
`MEASURED` lines. The baseline and `@04-14` copies under `target/phase04-kernel-bytes/` were
never read by a metric command.

Phase 2's "`.text` identical" stop rule is **not** used (D-29): linking `crypto`/`ds` changes
`.text` by itself, so whole-ELF comparison plus the FP/SIMD and 2 MiB checks is the evidence set.

A Mac devshell hash is **not** comparable with the Linux `kernel-x64.elf` built at the owner
stop (`nix build .#kernel-x64`): the kernel `.rodata` carries toolchain store-path strings, and
the toolchain derivation differs per system. Only this before/after comparison, made on one
venue, is evidence; the Linux byte comparison and the `boot-iso` `outputHash` re-derivation are
the owner stop's (`../04-OWNER-STOP-2.md`).

The committed `boot-iso` `outputHash` (`sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=`,
re-derived at the first stop for the pre-gap kernel) is **expected not to match** the new x86_64
kernel. The re-key and fid-table fixes changed the x86_64 kernel bytes, so the owner re-derives
the value on the x86_64 Linux host in the second stop's section 5.

## Verdicts

VERDICT elf-virtshell@04-24: DIFFERENT
VERDICT elf-q35churnshell@04-24: DIFFERENT
VERDICT text-symbols-virtshell@04-24: RECORDED before=754232 at-04-14=755220 after=755220
VERDICT text-symbols-q35churnshell@04-24: RECORDED before=759072 at-04-14=760137 after=760137
VERDICT fpsimd-virtshell@04-24: EQUAL
VERDICT fpsimd-q35churnshell@04-24: EQUAL
VERDICT text-under-2mib-q35churnshell@04-24: PASS
