// Platform dispatch, mirroring riscv64/src/platform/mod.rs.
//
// raspi3b is the default: with no platform cfg emitted (bare cargo build/test)
// the not(any(...)) fallback arm selects it, so existing invocations keep
// working.  The raspi3b module also compiles under platform = "raspi4b"
// because raspi4b is a re-export shim over raspi3b — their only divergence
// today is DTB + qemu machine, both data in the config TOMLs.

#[cfg(platform = "virt")]
pub mod virt;
#[cfg(platform = "virt")]
pub use crate::platform::virt::*;

#[cfg(platform = "raspi4b")]
pub mod raspi4b;
#[cfg(platform = "raspi4b")]
pub use crate::platform::raspi4b::*;

#[cfg(any(test, not(platform = "virt")))]
pub mod raspi3b;
// The test arm is narrowed to non-virt builds: under the virtshell cfg
// (`cargo xtask test`'s aarch64 leg, HOST-TEST-ROUTE A) a bare `test`
// arm glob-imported raspi3b beside virt and made `devcons` ambiguous
// (E0659).  The raspi3b module itself still compiles under test.
#[cfg(any(
    all(test, not(platform = "virt")),
    platform = "raspi3b",
    not(any(platform = "virt", platform = "raspi4b"))
))]
pub use crate::platform::raspi3b::*;
