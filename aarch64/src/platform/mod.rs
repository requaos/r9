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
#[cfg(any(test, platform = "raspi3b", not(any(platform = "virt", platform = "raspi4b"))))]
pub use crate::platform::raspi3b::*;
