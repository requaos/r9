//! `cargo xtask miri`: run Miri over `ds/src/tpht/` with the second
//! toolchain derivation (Phase 3 CONTEXT D-13; RESEARCH C3, F6, F7).
//!
//! Miri is deliberately **not** part of `cargo xtask test`: the ordinary
//! test run stays fast, and the concurrent TPHT code gets its own step with
//! many seeds.  The toolchain comes from `R9_MIRI_TOOLCHAIN`, which the
//! devshell exports from the `toolchainMiri` derivation in `flake.nix`.
//! `rust-toolchain.toml` is never edited: adding `miri` there moves the
//! pinned toolchain store path embedded in every kernel (RESEARCH C3,
//! Pitfall 2).
//!
//! The step must never use the devshell `cargo` (`cargo()` or `$CARGO`):
//! that toolchain has no Miri.  It runs `<toolchain>/bin/cargo` explicitly
//! with that `bin` directory first on `PATH`.
//!
//! The resolver takes an injected environment lookup, so the named unset
//! error and the command shape are unit-testable without Miri installed.

use std::path::{Path, PathBuf};

use crate::Result;

/// The environment variable the devshell exports.
pub const TOOLCHAIN_VAR: &str = "R9_MIRI_TOOLCHAIN";

/// The exact command the step runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// `<toolchain>/bin/cargo`.
    pub program: PathBuf,
    /// `miri test --package ds --lib tpht`.
    pub args: Vec<String>,
    /// `<toolchain>/bin`, prepended to `PATH`.
    pub path_prefix: PathBuf,
    /// `-Zmiri-many-seeds=0..<seeds>`.
    pub miriflags: String,
}

/// Resolve `R9_MIRI_TOOLCHAIN` from an injected lookup.
pub fn resolve_toolchain(lookup: &dyn Fn(&str) -> Option<String>) -> Result<PathBuf> {
    let value = lookup(TOOLCHAIN_VAR).filter(|value| !value.trim().is_empty());
    match value {
        Some(value) => Ok(PathBuf::from(value.trim())),
        None => Err(format!(
            "miri: {TOOLCHAIN_VAR} is not set (empty and whitespace count as unset) — r9's xtask \
             has no built-in default toolchain; `nix develop` exports it from the second toolchain \
             derivation `toolchainMiri` (D-13, RESEARCH C3)"
        )
        .into()),
    }
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect_rs_files(&path, out),
            Ok(kind) if kind.is_file() => {
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    out.push(path);
                }
            }
            _ => {}
        }
    }
}

/// Count the `#[test]` attributes under `ds/src/tpht/`.
pub fn tpht_test_count(workspace: &Path) -> Result<usize> {
    let mut files = Vec::new();
    collect_rs_files(&workspace.join("ds/src/tpht"), &mut files);
    files.sort();
    let mut count = 0;
    for path in files {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("miri: read {}: {e}", path.display()))?;
        for line in text.lines() {
            let code = match line.find("//") {
                Some(i) => &line[..i],
                None => line,
            };
            count += code.matches("#[test]").count();
        }
    }
    Ok(count)
}

/// The command for one Miri run (a pure function: no environment, no spawn).
pub fn command_spec(toolchain: &Path, seeds: u32) -> CommandSpec {
    let bin = toolchain.join("bin");
    CommandSpec {
        program: bin.join("cargo"),
        args: vec![
            "miri".to_string(),
            "test".to_string(),
            "--package".to_string(),
            "ds".to_string(),
            "--lib".to_string(),
            "tpht".to_string(),
        ],
        path_prefix: bin,
        miriflags: format!("-Zmiri-many-seeds=0..{seeds}"),
    }
}

/// Run Miri over the TPHT tests, refusing when the filter would match
/// nothing.
pub fn run(workspace: &Path, seeds: u32) -> Result<()> {
    run_with_lookup(workspace, seeds, &|name| std::env::var(name).ok())
}

fn run_with_lookup(
    workspace: &Path,
    seeds: u32,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<()> {
    let toolchain = resolve_toolchain(lookup)?;
    let tests = tpht_test_count(workspace)?;
    if tests == 0 {
        return Err("miri: no tests under ds/src/tpht (the filter would match nothing)".into());
    }

    let spec = command_spec(&toolchain, seeds);
    let mut path = std::ffi::OsString::from(spec.path_prefix.as_os_str());
    if let Some(existing) = std::env::var_os("PATH") {
        path.push(":");
        path.push(existing);
    }

    let status = std::process::Command::new(&spec.program)
        .args(&spec.args)
        .current_dir(workspace)
        .env("PATH", path)
        .env("MIRIFLAGS", &spec.miriflags)
        .status()
        .map_err(|e| format!("miri: run {}: {e}", spec.program.display()))?;
    if !status.success() {
        return Err("miri: failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process;

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("r9-miri-{name}-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(root: &std::path::Path, rel: &str, text: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, text).unwrap();
    }

    #[test]
    fn miri_unset_variable_is_a_named_error() {
        let err = resolve_toolchain(&|_| None)
            .expect_err("an unset toolchain variable must be a named error")
            .to_string();
        assert!(err.contains("R9_MIRI_TOOLCHAIN"), "names the variable: {err}");
        assert!(err.contains("no built-in default"), "says there is no default: {err}");
        assert!(err.contains("nix develop"), "says where the value comes from: {err}");
        assert!(err.starts_with("miri:"), "carries the miri: prefix: {err}");
    }

    #[test]
    fn miri_empty_value_counts_as_unset() {
        let unset = resolve_toolchain(&|_| None).expect_err("unset").to_string();
        for value in ["", "   ", "\t\n"] {
            let err = resolve_toolchain(&|_| Some(value.to_string()))
                .expect_err("an empty value must count as unset")
                .to_string();
            assert_eq!(err, unset, "empty value {value:?} must give the same named error");
        }
    }

    #[test]
    fn miri_command_uses_the_toolchain_cargo() {
        let toolchain = std::path::Path::new("/nix/store/abc-miri-toolchain");
        let spec = command_spec(toolchain, 64);
        assert_eq!(
            spec.program,
            PathBuf::from("/nix/store/abc-miri-toolchain/bin/cargo"),
            "the program is the toolchain's own cargo, never the devshell Cargo"
        );
        assert_eq!(
            spec.args,
            ["miri", "test", "--package", "ds", "--lib", "tpht"],
            "the filter is the tpht module"
        );
        assert_eq!(
            spec.path_prefix,
            PathBuf::from("/nix/store/abc-miri-toolchain/bin"),
            "PATH begins with the toolchain bin"
        );
        assert_eq!(
            spec.miriflags, "-Zmiri-many-seeds=0..64",
            "the default seed range is 0..64"
        );
        assert_eq!(
            command_spec(toolchain, 7).miriflags,
            "-Zmiri-many-seeds=0..7",
            "--seeds N narrows the range to 0..N"
        );
    }

    #[test]
    fn miri_refuses_when_tpht_has_no_tests() {
        let dir = scratch("no-tests");
        write(&dir, "ds/src/tpht/mod.rs", "//! no tests here yet\n");
        let err = run_with_lookup(&dir, 64, &|_| Some("/nonexistent/toolchain".to_string()))
            .expect_err("a filter that matches nothing must not pass")
            .to_string();
        assert!(
            err.contains("no tests under ds/src/tpht"),
            "the refusal names the empty module: {err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn miri_counts_tests_under_tpht_only() {
        let dir = scratch("count");
        write(
            &dir,
            "ds/src/tpht/mod.rs",
            "#[cfg(test)]\nmod tests {\n    #[test]\n    fn one() {}\n}\n",
        );
        write(
            &dir,
            "ds/src/roaring.rs",
            "#[cfg(test)]\nmod tests {\n    #[test]\n    fn outside() {}\n}\n",
        );
        assert_eq!(
            tpht_test_count(&dir).expect("the scratch tree is readable"),
            1,
            "only #[test] attributes under ds/src/tpht/ count"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
