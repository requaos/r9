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

/// The committed q35shell config embedded at build time — the
/// r9-netdev-args bin's default --config source (16-01 Task 3): ONE
/// committed TOML is the single source, build-time embedded, never a
/// copied data blob.  The q35shell lineage is the config the Harvey
/// flake boots (kernel-x64's own build config).
pub const DEFAULT_NETDEV_CONFIG: &str = include_str!("../../x86_64/lib/config_q35shell.toml");

#[cfg(test)]
mod tests {
    use super::DEFAULT_NETDEV_CONFIG;
    use crate::config::Configuration;

    /// The bin's embedded default parses and its netdev default
    /// selector is slirp — the no-`--config` contract (16-01 Task 3).
    #[test]
    fn netdev_embedded_default_config_is_slirp() {
        let config: Configuration =
            toml::from_str(DEFAULT_NETDEV_CONFIG).expect("embedded q35shell TOML must parse");
        let netdev = config
            .qemu
            .as_ref()
            .and_then(|q| q.netdev.as_ref())
            .expect("embedded q35shell TOML carries a netdev table");
        assert_eq!(netdev.default, "slirp");
    }

    /// Every committed q35 TOML that carries a netdev stanza parses and
    /// carries the FULL D-05 mode-table set — default slirp + the three
    /// sub-tables.  (q35churn is the one deliberate exception: its
    /// carried stanza is slirp-only by design — the net-free churn
    /// gate.)  This is the data-layer machine check of the must-haves
    /// truth: a broken or half-migrated stanza surfaces here at test
    /// time, never at boot time.  18-01 joins q35auth; 17-02's
    /// q35churnshell joins the family it already carried.
    #[test]
    fn netdev_all_q35_tomls_carry_the_mode_tables() {
        let configs = [
            ("config_q35", include_str!("../../x86_64/lib/config_q35.toml")),
            ("config_q35auth", include_str!("../../x86_64/lib/config_q35auth.toml")),
            ("config_q35churnshell", include_str!("../../x86_64/lib/config_q35churnshell.toml")),
            ("config_q35exec", include_str!("../../x86_64/lib/config_q35exec.toml")),
            ("config_q35lowmcfg", include_str!("../../x86_64/lib/config_q35lowmcfg.toml")),
            ("config_q35net", include_str!("../../x86_64/lib/config_q35net.toml")),
            ("config_q35shell", include_str!("../../x86_64/lib/config_q35shell.toml")),
        ];
        for (name, text) in configs {
            let config: Configuration = toml::from_str(text)
                .unwrap_or_else(|e| panic!("{name}: committed TOML must parse: {e}"));
            let netdev = config
                .qemu
                .as_ref()
                .and_then(|q| q.netdev.as_ref())
                .unwrap_or_else(|| panic!("{name}: committed TOML carries a netdev table"));
            assert_eq!(netdev.default, "slirp", "{name}: default selector");
            assert!(netdev.slirp.is_some(), "{name}: slirp sub-table");
            assert!(netdev.tap.is_some(), "{name}: tap sub-table");
            assert!(netdev.vmnet.is_some(), "{name}: vmnet sub-table");
        }
    }
}
