//! The xtask library target (16-01, D-05): the ONE shared netdev
//! arg-emission implementation consumed by BOTH front-ends — the
//! xtask bin's in-process qemu boots (config::apply_netdev) and the
//! exported r9-netdev-args bin (Harvey's flake input, consumed at the
//! 16-04 pin advance).  Every -netdev/-device/filter-dump token for
//! every mode comes from config::netdev_args; nothing re-assembles
//! netdev argv by hand (the phase's standing prohibition).
//!
//! The crate split is the least-churn shape (16-RESEARCH Pattern 1,
//! A5): config + uregcheck and the root types they close over (Arch,
//! Profile, DynError/Result, env_or) move here verbatim from the old
//! single-bin root; src/main.rs and the emitter bin are consumers of
//! this lib.  uregcheck rides along because config's initrd
//! resolution reaches through uregcheck::harvey_dir (the R9_HARVEY_DIR
//! channel) and uregcheck's boot-test entry takes the root Arch —
//! the three files were one dependency island in the old crate, and
//! the island moves whole.

pub mod config;
pub mod uregcheck;

use std::{env, fmt};

use clap::ArgMatches;

pub type DynError = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, DynError>;

/// The cargo profile the build steps target (debug is the gate
/// default; --release flips it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub fn from(matches: &ArgMatches) -> Self {
        if matches.get_flag("release") { Profile::Release } else { Profile::Debug }
    }

    pub fn dir(&self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// The arch selector every step takes on the command line.
#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
pub enum Arch {
    Aarch64,
    Riscv64,
    X86_64,
}

impl Arch {
    pub fn from(matches: &ArgMatches) -> Self {
        *matches.get_one::<Arch>("arch").unwrap_or(&Arch::X86_64)
    }

    pub fn qemu_system(&self) -> String {
        env_or(
            "QEMU",
            match self {
                Arch::Aarch64 => "qemu-system-aarch64",
                Arch::Riscv64 => "qemu-system-riscv64",
                Arch::X86_64 => "qemu-system-x86_64",
            },
        )
    }

    pub fn target(&self) -> String {
        env_or("TARGET", format!("{}-unknown-none-elf", self.to_string().to_lowercase()).as_str())
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Env lookup with a default (the old bin-root helper, shared by both
/// bins through the lib).
pub fn env_or(var: &str, default: &str) -> String {
    let default = default.to_string();
    env::var(var).unwrap_or(default)
}

// The ONE netdev argv builder and its testable core, re-exported at
// the lib root so both front-ends name them exactly as the D-05
// contract does.  (Defined in config beside the Netdev tables they
// read; this re-export is the public seam.)
pub use config::{netdev_args, netdev_args_checked};
