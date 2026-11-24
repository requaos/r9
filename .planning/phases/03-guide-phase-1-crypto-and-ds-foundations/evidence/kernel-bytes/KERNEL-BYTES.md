# Kernel bytes after `crypto/` joins the workspace (plan 03-01)

Plan 03-01 adds the `crypto` workspace member and its six pinned crates. No kernel crate
depends on `crypto/`, no file under `port/src` changed, `rust-toolchain.toml` is unchanged
and `.cargo/` is unchanged. This file records the measurement (D-21, D-22).

- Base commit (before, from `before.sha256`): `4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1`
- After commit (`git rev-parse HEAD` when the after builds were made):
  `ba6efec01be56c6d91da87617e20a8ce7ad1c8d0`
- Venue: `Darwin arm64` (both the before and after builds were made on this one machine)

A Mac devshell build is not comparable with the Linux `kernel-x64.elf` built at the owner
stop (`nix build .#kernel-x64`): the kernel `.rodata` carries toolchain store-path strings,
and the toolchain derivation differs per system. Only the before/after comparison below,
made on one venue, is evidence. The same comparison is repeated after `ds/` joins the
workspace (plan 03-12, verdicts `@03-12`) and at the end of the phase (plan 03-10).

Both kernels were rebuilt from nothing: `target/aarch64-unknown-none-elf` and
`target/x86_64-unknown-none-elf` were moved aside to
`target/phase03-kernel-bytes/pre-03-01-targets/` before the after builds, then rebuilt with
`nix develop -c cargo xtask build --arch aarch64 --config virtshell` and
`nix develop -c cargo xtask build --arch x86-64 --config q35churnshell`.

## Before and after

| Config | Before (`4beb5c8`) | After (`ba6efec`) | Same or different |
|--------|--------------------|-------------------|-------------------|
| `virtshell` | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | same |
| `q35churnshell` | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | same |

## Verdicts

VERDICT elf-virtshell@03-01: IDENTICAL
VERDICT elf-q35churnshell@03-01: IDENTICAL

## Protected paths and lockfile

`git diff --stat 4beb5c8 HEAD -- rust-toolchain.toml port/src .cargo port/Cargo.toml
aarch64/Cargo.toml x86_64/Cargo.toml riscv64/Cargo.toml` printed nothing: no protected path
changed since the baseline.

`git diff 4beb5c8 HEAD -- Cargo.lock` has 0 removed lines (176 added): the six pins and their
transitive crates are new entries only.

## After `ds/` joins (plan 03-12)

Plan 03-12 adds the `ds` workspace member and its `roaring` dev-dependency. No kernel crate
depends on `crypto/` or `ds/`, no file under `port/src` changed, `rust-toolchain.toml` is
unchanged and `.cargo/` is unchanged.

- Base commit (before, from `before.sha256`): `4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1`
- After commit (`git rev-parse HEAD` when the after builds were made):
  `25af196f72ca18f93769f0fa2cd8d2ac02540632`
- Venue: `Darwin arm64` (both the before and after builds were made on this one machine)

Both kernels were rebuilt from nothing: `target/aarch64-unknown-none-elf` and
`target/x86_64-unknown-none-elf` were moved aside to
`target/phase03-kernel-bytes/pre-03-12-targets/` before the after builds, then rebuilt with
`nix develop -c cargo xtask build --arch aarch64 --config virtshell` and
`nix develop -c cargo xtask build --arch x86-64 --config q35churnshell`. A byte-for-byte
`cmp` against the copies under `target/phase03-kernel-bytes/before/` also reports no
difference.

| Config | Before (`4beb5c8`) | After (`25af196`) | Same or different |
|--------|--------------------|-------------------|-------------------|
| `virtshell` | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | same |
| `q35churnshell` | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | same |

### Verdicts (plan 03-12)

VERDICT elf-virtshell@03-12: IDENTICAL
VERDICT elf-q35churnshell@03-12: IDENTICAL

### Protected paths and lockfile (plan 03-12)

`git diff --stat 4beb5c8 HEAD -- rust-toolchain.toml port/src .cargo port/Cargo.toml
aarch64/Cargo.toml x86_64/Cargo.toml riscv64/Cargo.toml` printed nothing: no protected path
changed since the baseline.

`git diff 4beb5c8 HEAD -- Cargo.lock` has 0 removed lines (212 added): the `ds` package and
its `roaring =0.11.5` dev-dependency are new entries only, and the six `crypto` pins from
plan 03-01 are unchanged. `Cargo.lock` now has its final Phase 3 shape (no later plan adds
a crate), so this is the state the optional early Linux build of RESEARCH A8 and the
end-of-phase owner stop both vendor from.
