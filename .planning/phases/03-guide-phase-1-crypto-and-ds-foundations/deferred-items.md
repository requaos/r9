# Deferred items — Phase 3 (plan 03-12)

Out-of-scope discoveries, not fixed by plan 03-12:

- `nix develop -c cargo fmt --all -- --check` is red repo-wide at HEAD (pre-existing:
  `aarch64/`, `port/`, `x86_64/` and `xtask/` files predate the current rustfmt style).
  Plan 03-12's new `xtask` code was formatted to rustfmt's preferred shape, but
  `ds/src/lib.rs` keeps the plan-mandated module order rather than rustfmt's
  alphabetical reordering. No repo-wide formatting change is in this plan's scope.
