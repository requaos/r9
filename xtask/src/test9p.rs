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
//! Each listed file pins one part of the dialect:
//!
//! - `crypto/src/session.rs`, `crypto/tests/session.rs` — WIRE-05, the
//!   per-connection per-direction session keys.
//! - `port/tests/aead_wire.rs` — WIRE-01, the header byte layout, and
//!   WIRE-06, the opcode registry.
//! - `port/tests/aead_tlv.rs` — WIRE-02, `ExtMask` and the TLV records.
//! - `port/tests/aead_transport.rs` — WIRE-03, WIRE-05 and WIRE-11, the
//!   sealed transport, the handshake and the anti-splicing pins.
//! - `port/tests/aead_fid.rs` — WIRE-07 to WIRE-10, the handles, the
//!   `FidTable`, the `FidShard` sweep and the qid.
//! - `port/tests/aead_negotiate.rs` — WIRE-12 and WIRE-05, the `devmnt`
//!   version selection and one whole sealed session.
//!
//! Refusing when a listed test file is absent is the point of listing the
//! files here rather than globbing: a renamed or deleted file must fail the
//! step loudly instead of shrinking it into a silent pass (T-04-51).  For
//! the same reason a failing leg returns immediately, so no leg's status is
//! dropped or masked by a later one (T-04-52), and a leg whose result lines
//! report no passed tests is an error, so a filter that matches nothing
//! cannot pass (T-04-39).
//!
//! Each listed `port/tests` file is **its own leg**, on both backends, so a
//! file that lost every test fails by name (WR-08).  A `test result:` line
//! carries no file name, so a leg that ran several files at once could not
//! tie a zero count to the file it came from; one leg per file is what makes
//! the zero-test guard precise.
//!
//! Every default leg has a **scalar-backend twin**: the host build links
//! sha2's hardware SHA path and chacha20's NEON (or AVX2) backend, while the
//! kernels run the scalar code, so the same wire pins must run on both.  The
//! twin carries the crates' own soft-backend `--config` flags in its argument
//! vector, never in `.cargo/config.toml` (lint rule R5).  Seven legs and
//! their seven twins make fourteen commands, and the step is still not part
//! of `cargo xtask test`.

use std::path::{Path, PathBuf};

use crate::Result;

/// The `port/tests/` files the step runs, one `--test NAME` per entry.
pub const PORT_TESTS: &[&str] =
    &["aead_wire", "aead_tlv", "aead_transport", "aead_fid", "aead_negotiate"];

/// The `crypto` test targets the step runs, one `--test NAME` per entry.
pub const CRYPTO_TESTS: &[&str] = &["session"];

/// The soft-backend `--config` value the scalar-backend twins carry.
///
/// The host build links sha2's hardware SHA path and chacha20's NEON (or
/// AVX2) backend; the kernels run the scalar code.  The same wire pins must
/// run on both, so each default leg has a twin that forces the crates' soft
/// backends.  The flags ride in the leg's own argument vector, never in
/// `.cargo/config.toml` (lint rule R5).
const SOFT_BACKEND_CONFIG: &str =
    "build.rustflags=[\"--cfg\",\"chacha20_backend=\\\"soft\\\"\",\"--cfg\",\"sha2_backend=\\\"soft\\\"\"]";

/// The exact command the step runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// The cargo program the step was handed.
    pub program: PathBuf,
    /// `test --package ...` and one `--test` pair or filter.
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
///
/// Seven legs in order — the crypto library `session` pins, the crypto
/// `session` integration pins, then one leg per `PORT_TESTS` entry — each
/// immediately followed by its scalar-backend twin, so the run is fourteen
/// commands.
pub fn command_specs(cargo: &Path) -> Vec<CommandSpec> {
    let mut bases: Vec<Vec<String>> = Vec::new();

    // Leg 1: the crypto library `session` pins.
    let mut crypto_lib =
        vec!["test".to_string(), "--package".to_string(), "crypto".to_string(), "--lib".to_string()];
    for name in CRYPTO_TESTS {
        crypto_lib.push((*name).to_string());
    }
    bases.push(crypto_lib);

    // Leg 2: the crypto `session` integration pins.
    let mut crypto_tests =
        vec!["test".to_string(), "--package".to_string(), "crypto".to_string()];
    for name in CRYPTO_TESTS {
        crypto_tests.push("--test".to_string());
        crypto_tests.push((*name).to_string());
    }
    bases.push(crypto_tests);

    // Legs 3 to 7: one leg per `port/tests` wire pin file.  A result line
    // carries no file name, so only a leg per file can tie a zero count to
    // the file that lost its tests (T-04-39, WR-08).
    for name in PORT_TESTS {
        bases.push(vec![
            "test".to_string(),
            "--package".to_string(),
            "port".to_string(),
            "--test".to_string(),
            (*name).to_string(),
        ]);
    }

    let mut specs = Vec::with_capacity(bases.len() * 2);
    for args in bases {
        let default = CommandSpec { program: cargo.to_path_buf(), args: args.clone() };
        let mut twin_args = args;
        twin_args.push("--config".to_string());
        twin_args.push(SOFT_BACKEND_CONFIG.to_string());
        let twin = CommandSpec { program: cargo.to_path_buf(), args: twin_args };
        specs.push(default);
        specs.push(twin);
    }
    specs
}

/// Refuse before anything runs when a listed test file is missing.
///
/// A listed-but-absent file would otherwise shrink the step into a silent
/// pass, so this names the first missing path and returns before any leg
/// runs (T-04-51).
pub fn preflight(workspace: &Path) -> Result<()> {
    let mut listed: Vec<PathBuf> = Vec::new();
    for name in CRYPTO_TESTS {
        listed.push(workspace.join("crypto/src").join(format!("{name}.rs")));
        listed.push(workspace.join("crypto/tests").join(format!("{name}.rs")));
    }
    for name in PORT_TESTS {
        listed.push(workspace.join("port/tests").join(format!("{name}.rs")));
    }

    for path in listed {
        if !path.exists() {
            return Err(format!("test-9p: listed test file {} is missing", path.display()).into());
        }
    }
    Ok(())
}

/// Run every leg through `runner`, stopping at the first spawn error,
/// non-zero status or zero-test leg so no leg's status is dropped or masked
/// by a later one (T-04-52, T-04-39).
pub fn run_specs(
    specs: &[CommandSpec],
    runner: &mut dyn FnMut(&CommandSpec) -> Result<LegOutcome>,
) -> Result<()> {
    for spec in specs {
        let outcome = runner(spec)?;
        if !outcome.status_ok {
            return Err(format!(
                "test-9p: leg failed: {} {}",
                spec.program.display(),
                spec.args.join(" ")
            )
            .into());
        }
        check_leg_output(spec, &outcome.stdout)?;
    }
    Ok(())
}

/// Reject a leg whose result lines report no passed tests, which would be a
/// silent pass (T-04-39).
///
/// A leg runs exactly one test binary, but cargo may print more than one
/// `test result: ok.` line for it.  The leg is `Ok` only when there is at
/// least one such line and **every** one reports a passed count above zero:
/// one zero-passed line means a listed file lost every test, and it must not
/// hide behind a sibling's positive line (WR-08).
pub fn check_leg_output(spec: &CommandSpec, stdout: &str) -> Result<()> {
    let mut result_lines = 0usize;
    let mut positive_lines = 0usize;
    for line in stdout.lines() {
        let Some(rest) = line.split_once("test result: ok.").map(|(_, rest)| rest) else {
            continue;
        };
        result_lines += 1;
        let positive = rest
            .split_whitespace()
            .next()
            .and_then(|count| count.parse::<u64>().ok())
            .is_some_and(|count| count > 0);
        if positive {
            positive_lines += 1;
        }
    }

    if result_lines > 0 && positive_lines == result_lines {
        return Ok(());
    }

    Err(format!(
        "test-9p: leg ran no tests: {} {}",
        spec.program.display(),
        spec.args.join(" ")
    )
    .into())
}

/// Run every leg in `workspace`, capturing and echoing each leg's output.
pub fn run(workspace: &Path, cargo: &Path) -> Result<()> {
    preflight(workspace)?;
    let specs = command_specs(cargo);

    run_specs(&specs, &mut |spec| {
        let output = std::process::Command::new(&spec.program)
            .args(&spec.args)
            .current_dir(workspace)
            .output()
            .map_err(|e| format!("test-9p: run {}: {e}", spec.program.display()))?;

        use std::io::Write as _;
        let mut out = std::io::stdout();
        let _ = out.write_all(&output.stdout);
        let _ = out.flush();
        let _ = std::io::stderr().write_all(&output.stderr);

        Ok(LegOutcome {
            status_ok: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        })
    })
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
        assert_eq!(specs.len(), 14, "seven legs, each with a scalar twin: {specs:?}");

        // Order: `crypto --lib session`, its twin, `crypto --test session`,
        // its twin, then one leg per `PORT_TESTS` entry, each with its twin.
        assert_eq!(specs[0].args, ["test", "--package", "crypto", "--lib", "session"]);
        assert_eq!(specs[2].args, ["test", "--package", "crypto", "--test", "session"]);

        for name in PORT_TESTS {
            let default_legs: Vec<&CommandSpec> = specs
                .iter()
                .step_by(2)
                .filter(|spec| {
                    spec.args.windows(2).any(|pair| pair[0] == "--test" && pair[1] == *name)
                })
                .collect();
            assert_eq!(
                default_legs.len(),
                1,
                "exactly one default leg carries `--test {name}`: {specs:?}"
            );
        }
        for spec in &specs {
            assert_eq!(spec.program, PathBuf::from(cargo), "every spec runs the cargo it is given");
        }
    }

    #[test]
    fn test9p_runs_one_leg_per_listed_port_file() {
        let cargo = Path::new("/usr/bin/cargo");
        let specs = command_specs(cargo);
        assert_eq!(specs.len(), 14, "seven legs, each with a scalar twin: {specs:?}");

        // The default legs are every other spec, starting at the first.
        let defaults: Vec<&CommandSpec> = specs.iter().step_by(2).collect();
        assert_eq!(defaults.len(), 7, "one leg per listed file, plus the two crypto pins");

        assert_eq!(defaults[0].args, ["test", "--package", "crypto", "--lib", "session"]);
        assert_eq!(defaults[1].args, ["test", "--package", "crypto", "--test", "session"]);

        for (offset, name) in PORT_TESTS.iter().enumerate() {
            let spec = defaults[2 + offset];
            assert_eq!(
                spec.args,
                ["test", "--package", "port", "--test", *name],
                "the leg for `{name}` names exactly that file"
            );
        }

        for spec in &specs {
            let test_flags = spec.args.iter().filter(|arg| *arg == "--test").count();
            assert!(
                test_flags <= 1,
                "no spec carries more than one `--test` pair: {:?}",
                spec.args
            );
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
        assert_eq!(specs.len() / 2, 7, "seven default/twin pairs: {specs:?}");

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
    fn test9p_rejects_a_leg_with_one_zero_test_result_line() {
        let cargo = Path::new("/usr/bin/cargo");
        let specs = command_specs(cargo);
        let spec = specs.last().expect("the step has at least one leg");
        let leg = spec.args.join(" ");

        // Two positive result lines and one zero-passed line: the leg fails,
        // because one listed file lost every test even though the rest ran.
        let mixed = "running 5 tests\n\
                     \n\
                     test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n\
                     \n\
                     running 2 tests\n\
                     \n\
                     test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n\
                     \n\
                     running 0 tests\n\
                     \n\
                     test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out\n";
        let err = check_leg_output(spec, mixed)
            .expect_err("one zero-passed result line fails the whole leg")
            .to_string();
        assert!(err.starts_with("test-9p:"), "carries the test-9p prefix: {err}");
        assert!(err.contains(&leg), "names the leg `{leg}`: {err}");

        // Every result line reports passed tests: the leg is fine.
        let positive = "running 5 tests\n\
                        \n\
                        test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n\
                        \n\
                        running 2 tests\n\
                        \n\
                        test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n";
        check_leg_output(spec, positive).expect("every result line reporting tests is fine");

        // No result line at all is still an error.
        let empty = "running 0 tests\n\nnothing ran here\n";
        let err = check_leg_output(spec, empty)
            .expect_err("a leg with no result line must be an error")
            .to_string();
        assert!(err.starts_with("test-9p:"), "carries the test-9p prefix: {err}");
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
        assert_eq!(specs.len(), 14, "checks all fourteen specs: {specs:?}");
        for spec in specs {
            assert_eq!(spec.program, cargo, "every spec runs the cargo it is given");
        }
    }
}
