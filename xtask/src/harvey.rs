//! The ONE per-architecture Harvey-tree resolver (D-01, D-21, HARN-02).
//!
//! Every xtask path that touches a Harvey tree — the ureg.h drift gate,
//! the boot-test precondition, the `harvey:` initrd scheme, the `harvey:`
//! dist-module scheme, the initrd assembler and its staleness check, and
//! the three pcap netaddr readers — resolves through [`harvey_dir`].  No
//! other xtask source file reads a Harvey-tree variable directly; the
//! `no_other_source_reads_a_harvey_tree_variable` test below enforces that
//! source invariant with `include_str!` over every sibling source file.
//!
//! There is **no built-in default path** (D-01): xtask never probes a
//! sibling `../harvey` checkout and never falls back to an implicit
//! `vendor/harvey/`; an unset variable is a named error that lists every
//! variable tried.
//!
//! Precedence (D-21):
//!
//! 1. `R9_HARVEY_DIR` — manual override, wins when set (needs no stamp).
//! 2. `R9_HARVEY_DIR_ARM64` / `R9_HARVEY_DIR_AMD64` — the flake-built
//!    per-architecture tree.  Must carry `<tree>/HARVEY-TREE` with
//!    `arch=arm64` or `arch=amd64` (plan 01-13 writes it), and a
//!    `/nix/store/` value that does not exist yet is realised on first use
//!    with a printed `nix build` of `packages.<system>.harvey-tree-<arch>`
//!    (D-05).
//! 3. `HARVEY_SRC` — explicit alias, lowest precedence (needs no stamp).
//!
//! Relative values resolve against the workspace root, so
//! `R9_HARVEY_DIR=vendor/harvey` selects `<workspace>/vendor/harvey`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fmt};

use crate::{Arch, DynError, Result};

/// `R9_HARVEY_DIR`: the manual override (wins when set, D-21).
const OVERRIDE_VAR: &str = "R9_HARVEY_DIR";
/// `HARVEY_SRC`: the explicit alias, lowest precedence.
const ALIAS_VAR: &str = "HARVEY_SRC";
/// `R9_HARVEY_TREE_SYSTEM`: the nix system a realise-on-first-use builds for.
const TREE_SYSTEM_VAR: &str = "R9_HARVEY_TREE_SYSTEM";

/// How a resolved tree was selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    /// `R9_HARVEY_DIR` — the manual override.
    Override,
    /// `R9_HARVEY_DIR_ARM64` / `R9_HARVEY_DIR_AMD64` — the flake tree.
    PerArch,
    /// `HARVEY_SRC` — the explicit alias.
    HarveySrc,
}

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Via::Override => write!(f, "R9_HARVEY_DIR override"),
            Via::PerArch => write!(f, "per-architecture flake tree"),
            Via::HarveySrc => write!(f, "HARVEY_SRC"),
        }
    }
}

/// A resolved Harvey tree and how it was selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub path: PathBuf,
    pub via: Via,
}

/// The per-architecture variable name for `arch` (a named error for an
/// architecture the Harvey trees do not cover).
fn per_arch_var(arch: Arch) -> Result<&'static str> {
    match arch {
        Arch::Aarch64 => Ok("R9_HARVEY_DIR_ARM64"),
        Arch::X86_64 => Ok("R9_HARVEY_DIR_AMD64"),
        Arch::Riscv64 => Err(no_riscv_tree()),
    }
}

/// The tree-stamp expectation for `arch` (`<tree>/HARVEY-TREE` carries
/// `arch=arm64` or `arch=amd64`; the flake trees write it).
fn tree_arch_name(arch: Arch) -> Result<&'static str> {
    match arch {
        Arch::Aarch64 => Ok("arm64"),
        Arch::X86_64 => Ok("amd64"),
        Arch::Riscv64 => Err(no_riscv_tree()),
    }
}

/// The nix attribute name of `arch`'s flake tree.
fn tree_attr_name(arch: Arch) -> Result<&'static str> {
    match arch {
        Arch::Aarch64 => Ok("harvey-tree-arm64"),
        Arch::X86_64 => Ok("harvey-tree-amd64"),
        Arch::Riscv64 => Err(no_riscv_tree()),
    }
}

fn no_riscv_tree() -> DynError {
    "a Harvey tree for riscv64 does not exist — the Harvey trees are per guest architecture \
     (arm64, amd64); name the riscv64 tools explicitly instead of a Harvey tree"
        .into()
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

/// A non-empty lookup value (`""` and whitespace-only count as unset).
fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.trim().is_empty())
}

/// Absolute form of a configured value; a relative one joins `workspace`.
fn absolute(value: &str, workspace: &Path) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() { path.to_path_buf() } else { workspace.join(path) }
}

/// `packages.<R9_HARVEY_TREE_SYSTEM>.harvey-tree-<arch>` — the one
/// attribute a realise-on-first-use builds (T-01-29: nothing else).
fn realise_attribute(arch: Arch, lookup: &dyn Fn(&str) -> Option<String>) -> Result<String> {
    let system = non_empty(lookup(TREE_SYSTEM_VAR)).ok_or_else(|| -> DynError {
        format!(
            "{TREE_SYSTEM_VAR} is not set — realising a per-architecture Harvey tree runs \
             `nix build <workspace>#packages.<system>.{}` and needs the system (e.g. \
             aarch64-linux); export it, or point the per-architecture variable at an existing \
             tree (D-05)",
            tree_attr_name(arch).unwrap_or("harvey-tree-<arch>")
        )
        .into()
    })?;
    Ok(format!("packages.{system}.{}", tree_attr_name(arch)?))
}

/// Resolve the Harvey tree for `arch` against an injected environment
/// lookup, workspace root and realiser (the testable core).
///
/// Precedence: `R9_HARVEY_DIR` > the per-architecture variable >
/// `HARVEY_SRC`.  Empty counts as unset.  No built-in default (D-01).
pub fn resolve(
    arch: Arch,
    lookup: &dyn Fn(&str) -> Option<String>,
    workspace: &Path,
    realise: &dyn Fn(&str) -> Result<PathBuf>,
) -> Result<Resolved> {
    // A riscv64 Harvey tree does not exist; the per-arch and override
    // branches below cannot conjure one.
    tree_arch_name(arch)?;

    if let Some(value) = non_empty(lookup(OVERRIDE_VAR)) {
        return Ok(Resolved { path: absolute(&value, workspace), via: Via::Override });
    }

    let per_arch = per_arch_var(arch)?;
    if let Some(value) = non_empty(lookup(per_arch)) {
        let exported = absolute(&value, workspace);
        let path = if is_store_path(&exported) && !exported.exists() {
            // D-05: a store path that was never built is realised on first
            // use, and the printed out path must equal the exported one.
            let attribute = realise_attribute(arch, lookup)?;
            let realised = realise(&attribute)?;
            if realised != exported {
                return Err(format!(
                    "{per_arch} names {} but the realiser printed {} — the `nix build` out \
                     path must equal the exported one (D-05)",
                    exported.display(),
                    realised.display()
                )
                .into());
            }
            realised
        } else {
            exported
        };
        check_tree_stamp(&path, arch)?;
        return Ok(Resolved { path, via: Via::PerArch });
    }

    if let Some(value) = non_empty(lookup(ALIAS_VAR)) {
        return Ok(Resolved { path: absolute(&value, workspace), via: Via::HarveySrc });
    }

    Err(no_tree_error(arch).into())
}

/// Is `path` under the nix store (where a missing value is a pending build,
/// not a typo)?
fn is_store_path(path: &Path) -> bool {
    path.starts_with(Path::new("/nix/store"))
}

/// A per-architecture tree must carry `<tree>/HARVEY-TREE` with the
/// matching `arch=` line (T-01-28: a tree of the wrong architecture is a
/// named error, never a confusing downstream failure).
fn check_tree_stamp(path: &Path, arch: Arch) -> Result<()> {
    let want = tree_arch_name(arch)?;
    let stamp = path.join("HARVEY-TREE");
    let text = fs::read_to_string(&stamp).map_err(|_| -> DynError {
        format!(
            "{} is not a flake-built Harvey tree: {} is missing — the per-architecture trees \
             written by r9's flake carry it (plan 01-13); a checkout selected with \
             R9_HARVEY_DIR or HARVEY_SRC needs no stamp",
            path.display(),
            stamp.display()
        )
        .into()
    })?;
    let found = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("arch=").map(str::trim))
        .find(|value| !value.is_empty());
    match found {
        Some(found) if found == want => Ok(()),
        Some(found) => Err(format!(
            "{} is stamped arch={found} but this command needs arch={want} — the \
             per-architecture variables each name that architecture's tree (D-21)",
            path.display()
        )
        .into()),
        None => Err(format!(
            "{} is not a flake-built Harvey tree: {} carries no `arch=` line (expected \
             arch={want})",
            path.display(),
            stamp.display()
        )
        .into()),
    }
}

/// Resolve the Harvey tree for `arch` from the process environment and the
/// workspace root, realising a per-architecture store path when needed.
pub fn harvey_dir(arch: Arch) -> Result<PathBuf> {
    let workspace = workspace_root();
    let lookup = |var: &str| env::var(var).ok();
    let realise = |attribute: &str| realise_tree(attribute);
    Ok(resolve(arch, &lookup, &workspace, &realise)?.path)
}

/// The realiser: `nix build --no-link --print-out-paths
/// <workspace>#packages.<system>.harvey-tree-<arch>`, printed before it
/// runs (D-05's observable first use).
fn realise_tree(attribute: &str) -> Result<PathBuf> {
    let reference = format!("{}#{attribute}", workspace_root().display());
    println!("harvey: realising {reference} (nix build --no-link --print-out-paths)");
    let out = Command::new("nix")
        .args(["build", "--no-link", "--print-out-paths"])
        .arg(&reference)
        .output()
        .map_err(|e| -> DynError {
            format!("spawn nix to realise {reference}: {e}").into()
        })?;
    if !out.status.success() {
        return Err(format!("nix build {reference} failed: {}", out.status).into());
    }
    let text = String::from_utf8(out.stdout)?;
    let printed = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| -> DynError {
            format!("nix build {reference} printed no out path").into()
        })?;
    Ok(PathBuf::from(printed))
}

/// The workspace root (`xtask/`'s parent), the base for relative values.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .expect("xtask/ always has a parent workspace root")
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    /// A scratch tree carrying a `HARVEY-TREE` stamp of `arch`.
    fn stamped_tree(name: &str, arch: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("r9-harvey-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("HARVEY-TREE"), format!("arch={arch}\n")).unwrap();
        dir
    }

    /// A lookup backed by a list of `(var, value)` pairs.
    fn lookup_of(pairs: Vec<(&'static str, String)>) -> impl Fn(&str) -> Option<String> {
        move |var| {
            pairs
                .iter()
                .find(|(name, _)| *name == var)
                .map(|(_, value)| value.clone())
        }
    }

    /// A realiser that fails the test if called.
    fn never_realise(_: &str) -> Result<PathBuf> {
        panic!("the realiser must not run for this resolution");
    }

    #[test]
    fn riscv64_has_no_harvey_tree() {
        let err = harvey_dir(Arch::Riscv64).unwrap_err().to_string();
        assert!(err.contains("riscv64"), "{err}");
        assert!(err.contains("per guest architecture"), "{err}");
    }

    #[test]
    fn override_wins_over_per_arch_and_alias() {
        let lookup = lookup_of(vec![
            ("R9_HARVEY_DIR", "/a".to_string()),
            ("R9_HARVEY_DIR_ARM64", "/b".to_string()),
            ("HARVEY_SRC", "/c".to_string()),
        ]);
        let resolved = resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &never_realise).unwrap();
        assert_eq!(resolved.path, PathBuf::from("/a"));
        assert_eq!(resolved.via, Via::Override);
    }

    #[test]
    fn per_arch_wins_over_the_alias() {
        let tree = stamped_tree("perarch", "arm64");
        let lookup = lookup_of(vec![
            ("R9_HARVEY_DIR_ARM64", tree.to_str().unwrap().to_string()),
            ("HARVEY_SRC", "/c".to_string()),
        ]);
        let resolved = resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &never_realise).unwrap();
        assert_eq!(resolved.path, tree);
        assert_eq!(resolved.via, Via::PerArch);
        let _ = fs::remove_dir_all(&resolved.path);
    }

    #[test]
    fn harvey_src_is_the_lowest_precedence_alias() {
        let lookup = lookup_of(vec![("HARVEY_SRC", "/c".to_string())]);
        let resolved = resolve(Arch::X86_64, &lookup, Path::new("/ws"), &never_realise).unwrap();
        assert_eq!(resolved.path, PathBuf::from("/c"));
        assert_eq!(resolved.via, Via::HarveySrc);
    }

    #[test]
    fn empty_counts_as_unset() {
        let lookup = lookup_of(vec![
            ("R9_HARVEY_DIR", String::new()),
            ("R9_HARVEY_DIR_ARM64", "  ".to_string()),
            ("HARVEY_SRC", String::new()),
        ]);
        let err = resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &never_realise)
            .unwrap_err()
            .to_string();
        assert!(err.contains("R9_HARVEY_DIR_ARM64"), "{err}");
    }

    #[test]
    fn a_relative_value_resolves_against_the_workspace() {
        let lookup = lookup_of(vec![("R9_HARVEY_DIR", "vendor/harvey".to_string())]);
        let resolved = resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &never_realise).unwrap();
        assert_eq!(resolved.path, PathBuf::from("/ws/vendor/harvey"));
        assert_eq!(resolved.via, Via::Override);
    }

    #[test]
    fn the_named_error_lists_every_variable_and_says_there_is_no_default() {
        for (arch, per_arch) in
            [(Arch::Aarch64, "R9_HARVEY_DIR_ARM64"), (Arch::X86_64, "R9_HARVEY_DIR_AMD64")]
        {
            let lookup = lookup_of(vec![]);
            let err = resolve(arch, &lookup, Path::new("/ws"), &never_realise)
                .unwrap_err()
                .to_string();
            assert!(err.contains(per_arch), "{err}");
            assert!(err.contains("R9_HARVEY_DIR"), "{err}");
            assert!(err.contains("HARVEY_SRC"), "{err}");
            assert!(err.contains("no built-in default"), "{err}");
            assert!(err.contains("vendor/harvey"), "{err}");
        }
    }

    #[test]
    fn a_wrong_arch_stamp_is_a_named_error() {
        let tree = stamped_tree("wrongarch", "amd64");
        let lookup =
            lookup_of(vec![("R9_HARVEY_DIR_ARM64", tree.to_str().unwrap().to_string())]);
        let err = resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &never_realise)
            .unwrap_err()
            .to_string();
        assert!(err.contains("arch=amd64"), "{err}");
        assert!(err.contains("arch=arm64"), "{err}");
        let _ = fs::remove_dir_all(&tree);
    }

    #[test]
    fn a_per_arch_tree_without_a_stamp_is_rejected() {
        let dir = std::env::temp_dir().join(format!("r9-harvey-nostamp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let lookup = lookup_of(vec![("R9_HARVEY_DIR_ARM64", dir.to_str().unwrap().to_string())]);
        let err = resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &never_realise)
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a flake-built Harvey tree"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn override_and_alias_paths_need_no_stamp() {
        let dir = std::env::temp_dir().join(format!("r9-harvey-plain-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.to_str().unwrap().to_string();
        for (var, via) in [("R9_HARVEY_DIR", Via::Override), ("HARVEY_SRC", Via::HarveySrc)] {
            let lookup = lookup_of(vec![(var, path.clone())]);
            let resolved =
                resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &never_realise).unwrap();
            assert_eq!(resolved.via, via);
            assert_eq!(resolved.path, dir);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_existing_tree_never_calls_the_realiser() {
        let tree = stamped_tree("existing", "arm64");
        let calls = Cell::new(0usize);
        let realise = |_: &str| -> Result<PathBuf> {
            calls.set(calls.get() + 1);
            panic!("an existing tree is never realised");
        };
        let lookup =
            lookup_of(vec![("R9_HARVEY_DIR_ARM64", tree.to_str().unwrap().to_string())]);
        let resolved = resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &realise).unwrap();
        assert_eq!(resolved.path, tree);
        assert_eq!(calls.get(), 0);
        let _ = fs::remove_dir_all(&resolved.path);
    }

    #[test]
    fn a_missing_store_path_calls_the_realiser_once_and_rejects_a_mismatch() {
        let exported = "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-harvey-tree-arm64";
        let lookup = lookup_of(vec![
            ("R9_HARVEY_DIR_ARM64", exported.to_string()),
            ("R9_HARVEY_TREE_SYSTEM", "aarch64-linux".to_string()),
        ]);
        let calls = Cell::new(0usize);
        let seen = RefCell::new(String::new());
        let realise = |attribute: &str| -> Result<PathBuf> {
            calls.set(calls.get() + 1);
            *seen.borrow_mut() = attribute.to_string();
            Ok(PathBuf::from(
                "/nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-harvey-tree-arm64",
            ))
        };
        let err = resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &realise)
            .unwrap_err()
            .to_string();
        assert_eq!(calls.get(), 1);
        assert_eq!(*seen.borrow(), "packages.aarch64-linux.harvey-tree-arm64");
        assert!(err.contains(exported), "{err}");
        assert!(err.contains("must equal the exported one"), "{err}");
    }

    #[test]
    fn a_realise_without_the_tree_system_is_a_named_error() {
        let lookup = lookup_of(vec![(
            "R9_HARVEY_DIR_ARM64",
            "/nix/store/cccccccccccccccccccccccccccccccc-harvey-tree-arm64".to_string(),
        )]);
        let err = resolve(Arch::Aarch64, &lookup, Path::new("/ws"), &never_realise)
            .unwrap_err()
            .to_string();
        assert!(err.contains("R9_HARVEY_TREE_SYSTEM"), "{err}");
    }

    /// The assumption-delta enforcement (plan 01-08's
    /// `<assumption_delta_decision>`): only harvey.rs reads a Harvey-tree
    /// variable, and uregcheck.rs keeps no zero-argument resolver.
    #[test]
    fn no_other_source_reads_a_harvey_tree_variable() {
        const SOURCES: [(&str, &str); 6] = [
            ("config.rs", include_str!("config.rs")),
            ("uregcheck.rs", include_str!("uregcheck.rs")),
            ("pcap.rs", include_str!("pcap.rs")),
            ("main.rs", include_str!("main.rs")),
            ("lib.rs", include_str!("lib.rs")),
            ("bin/r9-netdev-args.rs", include_str!("bin/r9-netdev-args.rs")),
        ];
        let needles = [
            "env::var(\"R9_HARVEY_DIR",
            "env::var(\"HARVEY_SRC",
            ".var(\"R9_HARVEY_DIR",
            ".var(\"HARVEY_SRC",
            // Split so this file itself never contains the literal the
            // grep gate (and this test) looks for.
            concat!("uregcheck::", "harvey_dir"),
        ];
        for (name, text) in SOURCES {
            for needle in needles {
                assert!(
                    !text.contains(needle),
                    "{name} reads a Harvey tree directly: {needle}"
                );
            }
        }
        let uregcheck = SOURCES.iter().find(|(name, _)| *name == "uregcheck.rs").unwrap().1;
        assert!(
            !uregcheck.contains("pub fn harvey_dir()"),
            "uregcheck.rs must not keep a zero-argument resolver"
        );
    }
}
