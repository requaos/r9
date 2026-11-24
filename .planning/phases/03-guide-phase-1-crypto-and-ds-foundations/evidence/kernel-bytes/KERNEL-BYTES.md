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

## End of the phase (plan 03-10)

Plan 03-10 adds no code and no dependency: it is the audit, the closing evidence and the owner
checklist. The comparison below is the last one the Mac can make before the owner stop, and it
covers the whole phase — every commit from the baseline to the phase's last code commit.

- Base commit (before, from `before.sha256`): `4beb5c8eea27f2f7c586a9cedfbc3bc675c6bae1`
- After commit (`git rev-parse HEAD` when the after builds were made):
  `1be121c11fc712648fe5876fb70e591eefde1a00` — the plan 03-10 Task 1 commit, which is the
  phase's last commit that precedes this measurement (Task 2 and Task 3 change only evidence
  and planning documents)
- Venue: `Darwin arm64` (both the before and after builds were made on this one machine;
  `uname -sm` was checked against the `venue` line of `before.sha256` before the builds)
- The four-line record is `after-03-10.sha256`, in the baseline's format with the `after` line
  first; its `venue` line equals the baseline's.

Both kernels were rebuilt from nothing: `target/aarch64-unknown-none-elf` and
`target/x86_64-unknown-none-elf` were moved aside to
`target/phase03-kernel-bytes/pre-03-10-{aarch64,x86_64}-target/` before the after builds, then
rebuilt with `nix develop -c cargo xtask build --arch aarch64 --config virtshell` and
`nix develop -c cargo xtask build --arch x86-64 --config q35churnshell` (both exit 0).

| Config | Before (`4beb5c8`) | After (`1be121c`) | Same or different |
|--------|--------------------|-------------------|-------------------|
| `virtshell` | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | `9218b998327c49b6ea06e79761d421328ad3451702719a0a0af8b79c59fa083e` | same |
| `q35churnshell` | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | `c9dcef0b32b77e13f30f8a6ba7b448948e684720dc794de003d06a1e4988b78b` | same |

### Verdicts (plan 03-10)

VERDICT elf-virtshell@03-10: IDENTICAL
VERDICT elf-q35churnshell@03-10: IDENTICAL

A supplementary byte-for-byte `cmp` against the copies plan 03-12 kept under
`target/phase03-kernel-bytes/before/` (`virtshell.elf`, `q35churnshell.elf`) also reports no
difference (both exit 0).

### Protected paths (plan 03-10)

`git diff --stat 4beb5c8 HEAD -- rust-toolchain.toml port/src .cargo port/Cargo.toml
aarch64/Cargo.toml x86_64/Cargo.toml riscv64/Cargo.toml` printed nothing: no protected path
changed between the baseline and this measurement.

One file outside that list did change in the phase and is recorded here so the owner stop can
account for it: `flake.nix` changed in plan 03-02 (`bdb4f4b`), devshell only — a second
toolchain derivation (`toolchainMiri`, `rust-toolchain.toml`'s four components plus `miri`)
and two `shellHook` lines exporting `R9_MIRI_TOOLCHAIN` and appending its `bin` to `PATH`.
The kernel package derivations still take `toolchainFor`, `rust-toolchain.toml` was not
edited and `.cargo/` was not edited, so the kernels' inputs are unchanged; the devshell change
cannot reach a `nix build` of a kernel package.

### What this does not prove

These hashes are one venue's devshell build of the working tree. They do not prove that the
nix-sandboxed kernel derivations (`packages.x86_64-linux.kernel-x64` and
`packages.<system>.kernel-virt`, which vendor from `Cargo.lock` through `importCargoLock`)
build with the enlarged lock, and they are not comparable with a Linux-built kernel: the
kernel `.rodata` carries toolchain store-path strings and the toolchain derivation differs per
system. That check is the end-of-phase owner stop's, in `../03-OWNER-STOP.md` sections 3 to 5
(RESEARCH A8).

