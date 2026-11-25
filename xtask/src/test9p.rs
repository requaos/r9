//! `cargo xtask test-9p`: run the 9P2000.aead wire pins on the host
//! (WIRE-13, D-23).
//!
//! The step is a **named subset of what `cargo xtask test` already runs**:
//! the `session` pins in the `crypto` crate and every file listed in
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

/// The `crypto` test targets the step runs, one `--test NAME` per entry.
pub const CRYPTO_TESTS: &[&str] = &[];

/// The `crypto` library test filter the step runs.
pub const CRYPTO_LIB_FILTER: &str = "session";

/// The soft-backend `--config` value the scalar-backend twins carry.
///
/// The host build links sha2's hardware SHA path and chacha20's NEON (or
/// AVX2) backend; the kernels run the scalar code.  The same wire pins must
/// run on both, so each default leg has a twin that forces the crates' soft
/// backends.  The flags ride in the leg's own argument vector, never in
/// `.cargo/config.toml` (lint rule R5).
#[allow(dead_code)]
const SOFT_BACKEND_CONFIG: &str =
    "build.rustflags=[\"--cfg\",\"chacha20_backend=\\\"soft\\\"\",\"--cfg\",\"sha2_backend=\\\"soft\\\"\"]";

/// The exact command the step runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// The cargo program the step was handed.
    pub program: PathBuf,
    /// `test --package ...` and the filter or `--test` pairs.
    pub args: Vec<String>,
}

/// The outcome of one leg: its status and the output used to count tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegOutcome {
    /// Whether the leg's process exited successfully.
    pub status_ok: bool,
    /// The leg's captured standard output.
    pub stdout: String,
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

/// Run every leg, stopping at the first spawn error, non-zero status or
/// zero-test leg so no leg's status is dropped or masked by a later one
/// (T-04-52, T-04-39).
pub fn run_specs(
    _specs: &[CommandSpec],
    _runner: &mut dyn FnMut(&CommandSpec) -> Result<LegOutcome>,
) -> Result<()> {
    Ok(())
}

/// Reject a leg that reports no passed tests, which would be a silent pass
/// (T-04-39).
pub fn check_leg_output(_spec: &CommandSpec, _stdout: &str) -> Result<()> {
    Ok(())
}

/// Run every leg in `workspace`.
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

    /// Every file `preflight` must see present.
    fn listed_files() -> Vec<String> {
        let mut listed: Vec<String> =
            PORT_TESTS.iter().map(|name| format!("port/tests/{name}.rs")).collect();
        listed.push("crypto/src/session.rs".to_string());
        listed.push("crypto/tests/session.rs".to_string());
        listed
    }

    #[test]
    fn test9p_command_specs_name_every_listed_file() {
        let cargo = Path::new("/usr/bin/cargo");
        let specs = command_specs(cargo);
        assert_eq!(specs.len(), 6, "three legs, each with a scalar twin: {specs:?}");

        // Order: `crypto --lib session`, its twin, `crypto --test session`,
        // its twin, the `port` leg, its twin.
        assert_eq!(specs[0].args, ["test", "--package", "crypto", "--lib", "session"]);
        assert_eq!(specs[2].args, ["test", "--package", "crypto", "--test", "session"]);

        let port = &specs[4];
        assert_eq!(port.args[0], "test");
        assert_eq!(port.args[1], "--package");
        assert_eq!(port.args[2], "port");
        for name in PORT_TESTS {
            assert!(
                port.args.windows(2).any(|pair| pair[0] == "--test" && pair[1] == *name),
                "the port leg carries `--test {name}`: {:?}",
                port.args
            );
        }
        for spec in &specs {
            assert_eq!(spec.program, PathBuf::from(cargo), "every spec runs the cargo it is given");
        }
    }

    #[test]
    fn test9p_lists_the_five_port_files_and_the_crypto_session_pins() {
        assert_eq!(
            PORT_TESTS,
            &["aead_wire", "aead_tlv", "aead_transport", "aead_fid", "aead_negotiate"][..],
            "the step runs exactly the five wire pin files"
        );

        let cargo = Path::new("/usr/bin/cargo");
        let specs = command_specs(cargo);
        assert!(
            specs
                .iter()
                .any(|spec| spec.args == ["test", "--package", "crypto", "--lib", "session"]),
            "a `cargo test --package crypto --lib session` leg exists: {specs:?}"
        );
        assert!(
            specs
                .iter()
                .any(|spec| spec.args == ["test", "--package", "crypto", "--test", "session"]),
            "a `cargo test --package crypto --test session` leg exists: {specs:?}"
        );
    }

    #[test]
    fn test9p_has_a_scalar_backend_twin_for_every_leg() {
        let cargo = Path::new("/usr/bin/cargo");
        let specs = command_specs(cargo);
        assert!(!specs.is_empty(), "the step has legs to run");
        assert_eq!(specs.len() % 2, 0, "legs come in default/twin pairs: {specs:?}");

        for pair in specs.chunks(2) {
            let default = &pair[0];
            let twin = &pair[1];
            assert_eq!(twin.program, default.program, "the twin runs the same cargo");

            let mut expected = default.args.clone();
            expected.push("--config".to_string());
            expected.push(SOFT_BACKEND_CONFIG.to_string());
            assert_eq!(
                twin.args, expected,
                "the twin is the default leg plus one `--config` for the scalar backend"
            );
        }

        assert!(SOFT_BACKEND_CONFIG.contains("chacha20_backend"), "names the chacha20 cfg");
        assert!(SOFT_BACKEND_CONFIG.contains("sha2_backend"), "names the sha2 cfg");
        assert!(SOFT_BACKEND_CONFIG.contains("soft"), "selects the soft backend");
    }

    #[test]
    fn test9p_refuses_when_a_listed_file_is_missing() {
        let listed = listed_files();
        let dir = scratch("missing");
        for rel in &listed {
            write(&dir, rel, "");
        }
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
    fn test9p_refuses_when_exactly_one_listed_file_is_missing() {
        let listed = listed_files();
        for missing in &listed {
            let dir = scratch(&format!("missing-one-{}", missing.replace('/', "-")));
            for rel in &listed {
                write(&dir, rel, "");
            }
            preflight(&dir).expect("every listed file present must pass");

            fs::remove_file(dir.join(missing)).unwrap();
            let err =
                preflight(&dir).expect_err("a missing listed file must be refused").to_string();
            assert!(err.starts_with("test-9p:"), "carries the test-9p prefix: {err}");
            assert!(err.contains(missing), "names the missing path {missing}: {err}");
            let _ = fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn test9p_treats_zero_tests_as_an_error() {
        let cargo = Path::new("/usr/bin/cargo");
        let specs = command_specs(cargo);
        let spec = specs.first().expect("the step has at least one leg");

        let zero = "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out\n";
        let err =
            check_leg_output(spec, zero).expect_err("a zero-test leg must be an error").to_string();
        assert!(err.starts_with("test-9p:"), "carries the test-9p prefix: {err}");

        let passing =
            "running 3 tests\n\ntest result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n";
        check_leg_output(spec, passing).expect("a leg that passed tests is fine");
    }

    #[test]
    fn test9p_reports_the_first_failing_leg() {
        let cargo = Path::new("/usr/bin/cargo");
        let specs = command_specs(cargo);
        assert!(specs.len() >= 3, "need at least three legs to test early stop: {specs:?}");

        let third = specs[2].clone();
        let third_args = third.args.join(" ");
        let mut calls = 0usize;
        let mut runner = |spec: &CommandSpec| {
            calls += 1;
            if *spec == third {
                Ok(LegOutcome { status_ok: false, stdout: String::new() })
            } else {
                Ok(LegOutcome {
                    status_ok: true,
                    stdout: "test result: ok. 1 passed; 0 failed".to_string(),
                })
            }
        };

        let err = run_specs(&specs, &mut runner).expect_err("a failing leg must error").to_string();
        assert!(err.contains(&third_args), "names the failing leg `{third_args}`: {err}");
        assert_eq!(calls, 3, "no leg after the failing one runs");
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
