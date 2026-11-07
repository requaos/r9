// Platform dispatch, mirroring aarch64/src/platform/mod.rs.
//
// q35 is the x86_64 machine: the kernel build emits platform="q35" from
// config_q35.toml's [config] platform key (xtask apply_platform_config).
//
// x86_64 differs from aarch64's fallback shape: there is no second board
// to fill the raspi3b slot, so BOTH arms compile the same q35 module.
// The not(q35) arm exists so the host test build (cargo test --bins
// --target x86_64-unknown-linux-gnu — the cargo-xtask test leg's shape,
// which carries no platform cfg) runs q35's host-testable parser surface
// (mb2): its console piece is test-inert by construction (pio's asm is
// cfg(not(test)) no-ops on the host), so nothing inside q35 needs a cfg.

#[cfg(platform = "q35")]
pub mod q35;
#[cfg(platform = "q35")]
pub use crate::platform::q35::*;

#[cfg(not(platform = "q35"))]
pub mod q35;
#[cfg(not(platform = "q35"))]
pub use crate::platform::q35::*;
