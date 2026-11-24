//! The static lint contract for `crypto/` and `ds/`, checked by `cargo
//! xtask test` before any cargo leg compiles (Phase 3 CONTEXT D-09, D-10,
//! D-11, D-16, D-19, D-21; LINT-01, DS-07).
//!
//! The rules are plain text scans of the source tree — no crate, no
//! compiler, no `syn`:
//!
//! - R1 (D-16, DS-07): no file or module declaration under `ds/src` names
//!   Funnel Hashing, and `ds/src/lib.rs` carries the crate-documentation
//!   restriction note (`Funnel Hashing`, `scratchpad`).
//! - R2 (D-10): `crypto/src` holds no `unsafe` token (line comments
//!   stripped first) and `crypto/src/lib.rs` carries
//!   `#![forbid(unsafe_code)]`.
//! - R3 (D-09): the same token rule for every `.rs` file under `ds/src`
//!   that is not under `ds/src/tpht/`; every non-`tpht` module declared in
//!   `ds/src/lib.rs` carries `#[forbid(unsafe_code)]`, and the `tpht` line
//!   carries none.
//! - R4 (D-21): no kernel crate (`port`, `aarch64`, `x86_64`, `riscv64`)
//!   depends on `crypto` or `ds`.
//! - R5 (RESEARCH C3, F3): `rust-toolchain.toml` lists no `miri`
//!   component, and `.cargo/config.toml` names neither soft-backend cfg.
//! - R6 (D-11, D-19): both crates opt into the workspace lints; `roaring`
//!   lives only under `ds`'s `[dev-dependencies]` and never in `crypto`.
//!
//! One failure lists every problem with its file and line, prefixed
//! `lint-contract:`.  The tests build any forbidden token they write into
//! a fixture from pieces with `concat()`, so this module's own source never
//! matches a scan.

use std::fs;
use std::path::Path;

use crate::Result;

/// Check the whole static contract against `workspace`.
pub fn check(workspace: &Path) -> Result<()> {
    let _ = workspace;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;

    use super::*;

    /// The real workspace root: `xtask` is one directory below it.
    fn real_workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(1).unwrap().to_path_buf()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("r9-lintcontract-{name}-{}", process::id()));
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

    /// A minimal tree that satisfies every rule.  A test overwrites the one
    /// file its rule reads.
    fn fixture(name: &str) -> PathBuf {
        let dir = scratch(name);
        write(
            &dir,
            "crypto/Cargo.toml",
            "[package]\nname = \"crypto\"\n\n[lints]\nworkspace = true\n",
        );
        write(&dir, "crypto/src/lib.rs", "#![forbid(unsafe_code)]\n");
        write(
            &dir,
            "ds/Cargo.toml",
            "[package]\nname = \"ds\"\n\n[dev-dependencies]\nroaring = \"=0.11.5\"\n\n[lints]\nworkspace = true\n",
        );
        write(
            &dir,
            "ds/src/lib.rs",
            "//! Funnel Hashing is not built in this crate; scratchpad only.\n\n\
             pub mod tpht;\n\
             #[forbid(unsafe_code)]\n\
             pub mod roaring;\n",
        );
        write(&dir, "ds/src/tpht/mod.rs", "//! tpht\n");
        for kernel in ["port", "aarch64", "x86_64", "riscv64"] {
            write(
                &dir,
                &format!("{kernel}/Cargo.toml"),
                "[package]\nname = \"x\"\n\n[dependencies]\nport = { path = \"../port\" }\n",
            );
        }
        write(
            &dir,
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"nightly-2026-01-30\"\ncomponents = [\"rustfmt\", \"rust-src\", \"clippy\", \"llvm-tools\"]\n",
        );
        write(&dir, ".cargo/config.toml", "[alias]\nxtask = \"run --package xtask --\"\n");
        dir
    }

    /// A forbidden token built from pieces, as the module doc requires.
    fn unsafe_token() -> String {
        ["uns", "afe"].concat()
    }

    fn unsafe_block() -> String {
        format!("fn f() {{\n    {} {{ }}\n}}\n", unsafe_token())
    }

    #[test]
    fn lintcontract_workspace_passes() {
        let workspace = real_workspace();
        check(&workspace).expect("the real workspace must satisfy the lint contract");
    }

    #[test]
    fn lintcontract_ds_crate_doc_carries_the_funnel_note() {
        let workspace = real_workspace();
        let text = fs::read_to_string(workspace.join("ds/src/lib.rs"))
            .expect("ds/src/lib.rs must exist");
        assert!(
            text.contains("Funnel Hashing"),
            "ds/src/lib.rs must name `Funnel Hashing` in its crate documentation (D-16, DS-07)"
        );
        assert!(
            text.contains("scratchpad"),
            "the note must restrict Funnel Hashing to scratchpads (D-16, DS-07)"
        );
    }

    #[test]
    fn lintcontract_rejects_unsafe_in_crypto() {
        let dir = fixture("unsafe-crypto");
        write(&dir, "crypto/src/x.rs", &unsafe_block());
        let err = check(&dir).expect_err("an unsafe token in crypto/src must be rejected");
        let text = err.to_string();
        assert!(
            text.contains("crypto/src/x.rs:2"),
            "the error names the file and line: {text}"
        );
        assert!(text.contains("lint-contract:"), "the error carries the prefix: {text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_rejects_unsafe_outside_tpht() {
        let dir = fixture("unsafe-roaring");
        write(&dir, "ds/src/roaring.rs", &unsafe_block());
        let text = check(&dir)
            .expect_err("an unsafe token outside ds/src/tpht/ must be rejected")
            .to_string();
        assert!(text.contains("ds/src/roaring.rs:2"), "names the file and line: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("unsafe-tpht-extra");
        write(&dir, "ds/src/tpht_extra.rs", &unsafe_block());
        let text = check(&dir)
            .expect_err("ds/src/tpht_extra.rs is outside the allowed directory")
            .to_string();
        assert!(text.contains("ds/src/tpht_extra.rs:2"), "names the file and line: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("unsafe-inside-tpht");
        write(&dir, "ds/src/tpht/flattened.rs", &unsafe_block());
        check(&dir).expect("ds/src/tpht/ may hold unsafe (D-09)");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_ignores_token_in_line_comment_and_in_unsafe_code() {
        let dir = fixture("token-comment");
        write(
            &dir,
            "crypto/src/x.rs",
            &format!("// {}\nfn f() {{}}\n", unsafe_token()),
        );
        write(
            &dir,
            "ds/src/roaring.rs",
            &format!("// {}\n#![forbid({}_code)]\n", unsafe_token(), unsafe_token()),
        );
        write(
            &dir,
            "ds/src/setops_extra.rs",
            &format!("// {}\nfn g() {{}}\n", unsafe_token()),
        );
        check(&dir).expect("a token in a line comment or in `unsafe_code` must not match");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_rejects_missing_forbid() {
        let dir = fixture("missing-forbid-ds");
        write(
            &dir,
            "ds/src/lib.rs",
            "//! Funnel Hashing is scratchpad-only.\n\npub mod tpht;\npub mod roaring;\n",
        );
        let text = check(&dir)
            .expect_err("a ds module without a forbid must be rejected")
            .to_string();
        assert!(text.contains("ds/src/lib.rs:4"), "names the module line: {text}");
        assert!(text.contains("forbid"), "names the missing attribute: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("missing-forbid-crypto");
        write(&dir, "crypto/src/lib.rs", "//! crypto\n");
        let text = check(&dir)
            .expect_err("crypto/src/lib.rs without the crate attribute must be rejected")
            .to_string();
        assert!(text.contains("crypto/src/lib.rs"), "names the file: {text}");
        assert!(text.contains("forbid"), "names the missing attribute: {text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_rejects_funnel_file_and_mod() {
        let dir = fixture("funnel-file");
        write(&dir, "ds/src/funnel.rs", "//! no\n");
        let text = check(&dir)
            .expect_err("a file named funnel under ds/src must be rejected")
            .to_string();
        assert!(text.contains("ds/src/funnel.rs"), "names the file: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("funnel-nested");
        write(&dir, "ds/src/tpht/funnel_hash.rs", "//! no\n");
        let text = check(&dir)
            .expect_err("a nested Funnel module file must be rejected")
            .to_string();
        assert!(text.contains("funnel_hash"), "names the file: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("funnel-case");
        write(&dir, "ds/src/Funnel.rs", "//! no\n");
        let text = check(&dir)
            .expect_err("the file-name match is case-insensitive")
            .to_string();
        assert!(text.contains("Funnel.rs"), "names the file: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("funnel-mod");
        write(&dir, "ds/src/roaring.rs", "mod funnel;\n");
        let text = check(&dir)
            .expect_err("a `mod funnel;` declaration must be rejected")
            .to_string();
        assert!(
            text.contains("ds/src/roaring.rs:1"),
            "names the declaration line: {text}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_rejects_missing_funnel_note() {
        let dir = fixture("missing-note");
        write(
            &dir,
            "ds/src/lib.rs",
            "//! ds crate.\n\npub mod tpht;\n#[forbid(unsafe_code)]\npub mod roaring;\n",
        );
        let text = check(&dir)
            .expect_err("ds/src/lib.rs without the restriction note must be rejected")
            .to_string();
        assert!(text.contains("Funnel Hashing"), "names the missing note: {text}");
        assert!(text.contains("note"), "says the note is missing: {text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_rejects_kernel_dependency() {
        for kernel in ["port", "aarch64", "x86_64", "riscv64"] {
            for dep in ["crypto", "ds"] {
                let dir = fixture(&format!("kernel-dep-{kernel}-{dep}"));
                write(
                    &dir,
                    &format!("{kernel}/Cargo.toml"),
                    &format!(
                        "[package]\nname = \"x\"\n\n[dependencies]\n{dep} = {{ path = \"../{dep}\" }}\n"
                    ),
                );
                let text = check(&dir)
                    .expect_err("a kernel crate depending on crypto or ds must be rejected")
                    .to_string();
                assert!(
                    text.contains(&format!("{kernel}/Cargo.toml")),
                    "{kernel} dependency named: {text}"
                );
                let _ = fs::remove_dir_all(&dir);
            }
        }

        let dir = fixture("kernel-dep-dev");
        write(
            &dir,
            "port/Cargo.toml",
            "[package]\nname = \"port\"\n\n[dev-dependencies]\nds = { path = \"../ds\" }\n",
        );
        let text = check(&dir)
            .expect_err("a dev-dependency table is a dependency table")
            .to_string();
        assert!(text.contains("port/Cargo.toml"), "names the manifest: {text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_rejects_toolchain_and_cargo_config_drift() {
        let dir = fixture("toolchain-miri");
        write(
            &dir,
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"nightly-2026-01-30\"\ncomponents = [\"rustfmt\", \"miri\"]\n",
        );
        let text = check(&dir)
            .expect_err("a miri component in rust-toolchain.toml must be rejected")
            .to_string();
        assert!(text.contains("rust-toolchain.toml"), "names the file: {text}");
        assert!(text.contains("miri"), "names the component: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("cargo-sha2-drift");
        write(
            &dir,
            ".cargo/config.toml",
            "[build]\nrustflags = [\"--cfg\", \"sha2_backend=\\\"soft\\\"\"]\n",
        );
        let text = check(&dir)
            .expect_err("a backend cfg in .cargo/config.toml must be rejected")
            .to_string();
        assert!(text.contains(".cargo/config.toml"), "names the file: {text}");
        assert!(text.contains("sha2_backend"), "names the cfg: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("cargo-chacha-drift");
        write(
            &dir,
            ".cargo/config.toml",
            "[build]\nrustflags = [\"--cfg\", \"chacha20_backend=\\\"soft\\\"\"]\n",
        );
        let text = check(&dir)
            .expect_err("the chacha backend cfg must be rejected too")
            .to_string();
        assert!(text.contains("chacha20_backend"), "names the cfg: {text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_rejects_roaring_outside_dev_dependencies() {
        let dir = fixture("roaring-dep");
        write(
            &dir,
            "ds/Cargo.toml",
            "[package]\nname = \"ds\"\n\n[dependencies]\nroaring = \"=0.11.5\"\n\n[lints]\nworkspace = true\n",
        );
        let text = check(&dir)
            .expect_err("roaring outside [dev-dependencies] must be rejected")
            .to_string();
        assert!(text.contains("ds/Cargo.toml"), "names the manifest: {text}");
        assert!(text.contains("roaring"), "names the crate: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("roaring-crypto");
        write(
            &dir,
            "crypto/Cargo.toml",
            "[package]\nname = \"crypto\"\n\n[dev-dependencies]\nroaring = \"=0.11.5\"\n\n[lints]\nworkspace = true\n",
        );
        let text = check(&dir)
            .expect_err("crypto must not name roaring")
            .to_string();
        assert!(text.contains("crypto/Cargo.toml"), "names the manifest: {text}");
        assert!(text.contains("roaring"), "names the crate: {text}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_rejects_missing_lint_opt_in() {
        let dir = fixture("lints-crypto");
        write(&dir, "crypto/Cargo.toml", "[package]\nname = \"crypto\"\n");
        let text = check(&dir)
            .expect_err("crypto without [lints] workspace = true must be rejected")
            .to_string();
        assert!(text.contains("crypto/Cargo.toml"), "names the manifest: {text}");
        assert!(text.contains("lints"), "names the missing table: {text}");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("lints-ds");
        write(
            &dir,
            "ds/Cargo.toml",
            "[package]\nname = \"ds\"\n\n[dev-dependencies]\nroaring = \"=0.11.5\"\n",
        );
        let text = check(&dir)
            .expect_err("ds without [lints] workspace = true must be rejected")
            .to_string();
        assert!(text.contains("ds/Cargo.toml"), "names the manifest: {text}");
        assert!(text.contains("lints"), "names the missing table: {text}");
        let _ = fs::remove_dir_all(&dir);
    }
}
