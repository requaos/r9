//! `cargo xtask test-9p`: run the 9P2000.aead wire pins on the host
//! (WIRE-13, D-23).
//!
//! The step is a **named subset of what `cargo xtask test` already runs**:
//! the `session` pins in the `crypto` library and every file listed in
//! `PORT_TESTS` under `port/tests/`.  The guide's verification matrix names
//! one command that runs the wire pins; this is it, and it is not added to
//! `cargo xtask test` (the ordinary run already covers the same files, and
//! stays as it is).
//!
//! Refusing when a listed test file is absent is the point of listing the
//! files here rather than globbing: a renamed or deleted file must fail the
//! step loudly instead of shrinking it into a silent pass (T-04-51).  For
//! the same reason a failing leg returns immediately, so no leg's status is
//! dropped or masked by a later one (T-04-52).
//!
//! Plan 04-12 completes the file list and adds the scalar-backend legs and
//! the guard against a leg that ran zero tests.  None of that is here yet:
//! this module lists what exists today (`aead_transport`) and runs it.

use std::path::{Path, PathBuf};

use crate::Result;

/// The `port/tests/` files the step runs, one `--test NAME` per entry.
pub const PORT_TESTS: &[&str] = &["aead_transport"];

/// The `crypto` library test filter the step runs.
pub const CRYPTO_LIB_FILTER: &str = "session";

/// The exact command the step runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// The cargo program the step was handed.
    pub program: PathBuf,
    /// `test --package ...` and the filter or `--test` pairs.
    pub args: Vec<String>,
}

/// The commands the step runs (a pure function: no environment, no spawn).
pub fn command_specs(cargo: &Path) -> Vec<CommandSpec> {
    let crypto = CommandSpec {
        program: cargo.to_path_buf(),
        args: vec![
            "test".to_string(),
            "--package".to_string(),
            "crypto".to_string(),
            "--lib".to_string(),
            CRYPTO_LIB_FILTER.to_string(),
        ],
    };

    let mut port_args = vec!["test".to_string(), "--package".to_string(), "port".to_string()];
    for name in PORT_TESTS {
        port_args.push("--test".to_string());
        port_args.push((*name).to_string());
    }
    let port = CommandSpec { program: cargo.to_path_buf(), args: port_args };

    vec![crypto, port]
}

/// Refuse before anything runs when a listed test file is missing.
///
/// A listed-but-absent file would otherwise shrink the step into a silent
/// pass, so this names the first missing path and returns before any leg
/// runs (T-04-51).
pub fn preflight(workspace: &Path) -> Result<()> {
    let mut listed: Vec<PathBuf> = PORT_TESTS
        .iter()
        .map(|name| workspace.join("port/tests").join(format!("{name}.rs")))
        .collect();
    listed.push(workspace.join("crypto/src/session.rs"));

    for path in listed {
        if !path.exists() {
            return Err(format!("test-9p: listed test file {} is missing", path.display()).into());
        }
    }
    Ok(())
}

/// Run every leg in `workspace`, returning at the first spawn error or
/// non-zero status so no leg's status is dropped or masked by a later one
/// (T-04-52).
pub fn run(workspace: &Path, cargo: &Path) -> Result<()> {
    preflight(workspace)?;
    for spec in command_specs(cargo) {
        let status = std::process::Command::new(&spec.program)
            .args(&spec.args)
            .current_dir(workspace)
            .status()
            .map_err(|e| format!("test-9p: run {}: {e}", spec.program.display()))?;
        if !status.success() {
            return Err(format!(
                "test-9p: leg failed: {} {}",
                spec.program.display(),
                spec.args.join(" ")
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("r9-test9p-{name}-{}", process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, text).unwrap();
    }

    #[test]
    fn test9p_command_specs_name_every_listed_file() {
        let cargo = Path::new("/usr/bin/cargo");
        let specs = command_specs(cargo);
        let crypto = specs
            .iter()
            .find(|spec| spec.args == ["test", "--package", "crypto", "--lib", "session"])
            .unwrap_or_else(|| {
                panic!("a `cargo test --package crypto --lib session` leg exists: {specs:?}")
            });
        assert_eq!(
            crypto.program,
            PathBuf::from(cargo),
            "the crypto leg runs the cargo it was given"
        );

        let port = specs
            .iter()
            .find(|spec| spec.args.iter().any(|arg| arg == "port"))
            .unwrap_or_else(|| panic!("a `cargo test --package port` leg exists: {specs:?}"));
        for name in PORT_TESTS {
            assert!(
                port.args.windows(2).any(|pair| pair[0] == "--test" && pair[1] == *name),
                "the port leg carries `--test {name}`: {:?}",
                port.args
            );
        }
    }

    #[test]
    fn test9p_refuses_when_a_listed_file_is_missing() {
        let dir = scratch("missing");
        for name in PORT_TESTS {
            write(&dir, &format!("port/tests/{name}.rs"), "");
        }
        write(&dir, "crypto/src/session.rs", "");
        preflight(&dir).expect("every listed file present must pass");

        let missing = PORT_TESTS[0];
        let missing_path = dir.join("port/tests").join(format!("{missing}.rs"));
        fs::remove_file(&missing_path).unwrap();
        let err = preflight(&dir).expect_err("a missing listed file must be refused").to_string();
        assert!(err.starts_with("test-9p:"), "carries the test-9p prefix: {err}");
        assert!(err.contains(&missing_path.display().to_string()), "names the missing path: {err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test9p_uses_the_program_it_is_given() {
        let cargo = PathBuf::from("/nix/store/abc-cargo/bin/cargo");
        let specs = command_specs(&cargo);
        assert!(!specs.is_empty(), "the step has legs to run");
        for spec in specs {
            assert_eq!(spec.program, cargo, "every spec runs the cargo it is given");
        }
    }
}
