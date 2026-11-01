// Raspi4b diverges from raspi3b only in DTB and qemu machine today — both are
// data in the config TOMLs — so this is a thin re-export shim over raspi3b.
// If real code divergence appears, duplicate the drivers into this directory
// per the riscv64 per-platform-dir convention.
pub use crate::platform::raspi3b::*;
