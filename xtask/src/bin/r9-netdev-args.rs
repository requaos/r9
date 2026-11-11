//! r9-netdev-args — the exported netdev emitter front-end (16-01
//! Task 3, D-05): prints the qemu netdev argv tokens for a mode, one
//! space-separated line on stdout (tokens contain no spaces; Harvey
//! consumes via word splitting at the 16-04 pin advance).  Every
//! token comes from the ONE shared builder —
//! xtask::config::netdev_args_checked — never assembled here; this
//! front-end only parses --mode/--pcap/--config and prints.
//!
//! Default config: the committed config_q35shell.toml embedded at
//! build time (xtask::DEFAULT_NETDEV_CONFIG) — ONE committed TOML is
//! the single source, build-time embedded, no copied data.
//! `--config <path>` overrides with an explicit file.  The mode name
//! is validated against the fixed set before any argv exists — no
//! shell passthrough (T-16-01).

use std::process::exit;

use xtask::DEFAULT_NETDEV_CONFIG;
use xtask::config::{Configuration, netdev_args_checked};

/// The committed usage contract — the pipe-separated mode list is
/// part of the interface (Harvey-side consumers and the phase verify
/// grep it verbatim).
const USAGE: &str = "usage: r9-netdev-args --mode slirp|tap|vmnet [--pcap <path>] [--config <path>]";

/// Print the usage line plus the specific grievance to stderr and
/// exit 1 — the fail-loud mode-selection discipline (D-01 semantics
/// at the emitter layer; never a silent empty line).
fn usage_exit(detail: &str) -> ! {
    eprintln!("{USAGE}");
    eprintln!("r9-netdev-args: {detail}");
    exit(1);
}

/// Pull a flag's value: either the inline `--flag=value` form or the
/// next argv word; a missing value is a usage error.
fn value_of(flag: &str, inline: Option<String>, rest: &mut impl Iterator<Item = String>) -> String {
    inline.or_else(|| rest.next()).unwrap_or_else(|| usage_exit(&format!("{flag} needs a value")))
}

fn main() {
    let mut mode: Option<String> = None;
    let mut pcap: Option<String> = None;
    let mut config_path: Option<String> = None;
    let mut rest = std::env::args().skip(1);
    while let Some(arg) = rest.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) => (f.to_string(), Some(v.to_string())),
            None => (arg, None),
        };
        match flag.as_str() {
            "--mode" => mode = Some(value_of(&flag, inline, &mut rest)),
            "--pcap" => pcap = Some(value_of(&flag, inline, &mut rest)),
            "--config" => config_path = Some(value_of(&flag, inline, &mut rest)),
            _ => usage_exit(&format!("unknown argument `{flag}`")),
        }
    }
    let mode = match mode {
        Some(m) => m,
        None => usage_exit("--mode is required"),
    };
    // The fixed selector set, validated before any argv exists
    // (T-16-01: the only runtime input is the mode name, and it never
    // reaches qemu unvalidated).
    match mode.as_str() {
        "slirp" | "tap" | "vmnet" => {}
        _ => usage_exit(&format!("unknown mode `{mode}`")),
    }
    let text = match &config_path {
        Some(p) => std::fs::read_to_string(p)
            .unwrap_or_else(|e| usage_exit(&format!("--config {p}: {e}"))),
        None => DEFAULT_NETDEV_CONFIG.to_string(),
    };
    let config = toml::from_str::<Configuration>(&text)
        .unwrap_or_else(|e| usage_exit(&format!("config parse: {e}")));
    let argv = netdev_args_checked(&config, &mode, pcap.as_deref())
        .unwrap_or_else(|e| usage_exit(&e));
    println!("{}", argv.join(" "));
}
