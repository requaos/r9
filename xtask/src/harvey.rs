//! The ONE per-architecture Harvey-tree resolver (D-01, D-21, HARN-02).
//!
//! Every xtask path that touches a Harvey tree — the ureg.h drift gate,
//! the boot-test precondition, the `harvey:` initrd scheme, the `harvey:`
//! dist-module scheme, the initrd assembler and its staleness check, and
//! the three pcap netaddr readers — resolves through [`harvey_dir`].  No
//! other xtask source file reads a Harvey-tree variable directly (the
//! source invariant in this module's tests enforces it).
//!
//! There is **no built-in default path** (D-01): xtask never probes a
//! sibling `../harvey` checkout and never falls back to an implicit
//! `vendor/harvey/`; an unset variable is a named error that lists every
//! variable tried.
//!
//! Precedence (D-21): `R9_HARVEY_DIR` is a manual override that wins when
//! set; otherwise the per-architecture variable for the requested arch
//! (`R9_HARVEY_DIR_ARM64` / `R9_HARVEY_DIR_AMD64`).  Task 2 of plan 01-08
//! adds `HARVEY_SRC`, relative-value resolution, the tree stamp check and
//! realise-on-first-use behind this same signature.

use std::path::PathBuf;
use std::env;

use crate::{Arch, Result};

/// The per-architecture variable name for `arch` (a named error for an
/// architecture the Harvey trees do not cover).
fn per_arch_var(arch: Arch) -> Result<&'static str> {
    match arch {
        Arch::Aarch64 => Ok("R9_HARVEY_DIR_ARM64"),
        Arch::X86_64 => Ok("R9_HARVEY_DIR_AMD64"),
        Arch::Riscv64 => Err("a Harvey tree for riscv64 does not exist — the Harvey trees are \
                              per guest architecture (arm64, amd64); name the riscv64 tools \
                              explicitly instead of a Harvey tree"
            .into()),
    }
}

/// `ureg-check`'s own header path for `arch`, for error text.
fn arch_header(arch: Arch) -> &'static str {
    match arch {
        Arch::Aarch64 => "arm64/include/ureg.h",
        Arch::X86_64 => "amd64/include/ureg.h",
        Arch::Riscv64 => "riscv64/include/ureg.h",
    }
}

/// The one named error for "no Harvey tree configured" (D-01).  It names
/// every variable tried, what the tree must contain, states there is no
/// built-in default, says `nix develop` exports the per-architecture
/// variables, and says how to select `vendor/harvey/`.
fn no_tree_error(arch: Arch) -> String {
    let per_arch = per_arch_var(arch).unwrap_or("R9_HARVEY_DIR_RISCV64");
    format!(
        "no Harvey tree: {per_arch} (the per-architecture tree for {arch}), R9_HARVEY_DIR \
         (manual override, wins when set) and HARVEY_SRC (explicit alias) are all unset — \
         r9's xtask has no built-in default path (D-01: it never probes ../harvey and never \
         falls back to an implicit vendor/harvey/); the tree must contain {header} for the \
         ureg-check drift gate, build/aarch64/smoke-out/ for the smoke initrd, and the \
         build inputs the images are staged from (D-16); `nix develop` exports the \
         per-architecture variables (D-21), and a vendor/harvey/ checkout is selected \
         explicitly with R9_HARVEY_DIR=vendor/harvey",
        arch = arch,
        header = arch_header(arch),
    )
}

/// Resolve the Harvey tree for `arch`.
///
/// Precedence: `R9_HARVEY_DIR` (override) > the per-architecture variable.
/// Empty counts as unset.  No built-in default (D-01).
pub fn harvey_dir(arch: Arch) -> Result<PathBuf> {
    if let Ok(dir) = lookup("R9_HARVEY_DIR") {
        return Ok(dir);
    }
    if let Ok(dir) = lookup(per_arch_var(arch)?) {
        return Ok(dir);
    }
    Err(no_tree_error(arch).into())
}

/// Read `var` as a non-empty path (empty means unset).
fn lookup(var: &str) -> std::result::Result<PathBuf, ()> {
    match env::var(var) {
        Ok(value) if !value.trim().is_empty() => Ok(PathBuf::from(value)),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riscv64_has_no_harvey_tree() {
        let err = harvey_dir(Arch::Riscv64).unwrap_err().to_string();
        assert!(err.contains("riscv64"), "{err}");
        assert!(err.contains("per guest architecture"), "{err}");
    }
}
