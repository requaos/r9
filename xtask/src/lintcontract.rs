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
//! - R4 (D-22, D-25): only `port` may depend on `crypto` or `ds`, in any
//!   dependency table; the three arch crates (`aarch64`, `x86_64`, `riscv64`)
//!   may not name either key and reach them through `port`.
//! - R5 (RESEARCH C3, F3): `rust-toolchain.toml` lists no `miri`
//!   component, and `.cargo/config.toml` names neither soft-backend cfg.
//! - R6 (D-11, D-19; Phase 5 CORR-05): `crypto/`, `ds/` and both compiled
//!   arch crates (`aarch64`, `x86_64`) opt into the workspace lints; `roaring`
//!   lives only under `ds`'s `[dev-dependencies]` and never in `crypto`.
//!
//! One failure lists every problem with its file and line, prefixed
//! `lint-contract:`.  The tests build any forbidden token they write into
//! a fixture from pieces with `concat()`, so this module's own source never
//! matches a scan.

use std::fs;
use std::path::{Path, PathBuf};

use crate::Result;

/// Check the whole static contract against `workspace`.
pub fn check(workspace: &Path) -> Result<()> {
    let mut problems: Vec<String> = Vec::new();

    check_funnel(workspace, &mut problems);
    check_crypto_token_rule(workspace, &mut problems);
    check_ds_token_and_forbid_rules(workspace, &mut problems);
    check_kernel_dependencies(workspace, &mut problems);
    check_toolchain_and_cargo_config(workspace, &mut problems);
    check_lints_and_roaring(workspace, &mut problems);

    if problems.is_empty() {
        return Ok(());
    }
    Err(format!(
        "lint-contract: {} problem(s) — the rules are Phase 3 CONTEXT D-09, D-10, D-11, D-16, D-19 \
         and D-21, and Phase 4 CONTEXT D-22 and D-25:\n{}",
        problems.len(),
        problems.join("\n")
    )
    .into())
}

/// A path relative to the workspace, in `file:line` message form.
fn rel(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect_files(&path, out),
            Ok(kind) if kind.is_file() => out.push(path),
            _ => {}
        }
    }
}

/// Every file below `dir`, in a deterministic order.
fn sorted_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(dir, &mut files);
    files.sort();
    files
}

fn read_file(path: &Path, display: &str, problems: &mut Vec<String>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) => {
            problems.push(format!("{display}: read: {e}"));
            None
        }
    }
}

/// Everything from `//` to the end of the line.  A bare word in a comment
/// must not fail the token rules; `//!` and `///` are line comments too.
fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// Everything from `#` to the end of the line (TOML comments).
fn strip_hash_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

/// Find `word` as a whole word: the characters on either side are not
/// letters, digits or `_`, so `unsafe_code` and `unsafe_op_in_unsafe_fn`
/// never match `unsafe`.
fn find_whole_word(text: &str, word: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(offset) = text[start..].find(word) {
        let i = start + offset;
        let before = i.checked_sub(1).map(|p| bytes[p] as char);
        let after = bytes.get(i + word.len()).map(|&b| b as char);
        let is_word_char =
            |c: Option<char>| c.map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
        if !is_word_char(before) && !is_word_char(after) {
            return Some(i);
        }
        start = i + word.len();
    }
    None
}

/// The module name a line declares, if any: `[pub[(...)]] mod name;` or
/// `mod name {`, after any leading attributes.  Line comments are ignored,
/// so the DS-07 note's prose never parses as a declaration.
fn declared_module_name(line: &str) -> Option<String> {
    let mut code = strip_line_comment(line).trim();
    loop {
        let Some(rest) = code.strip_prefix("#[") else { break };
        match rest.find(']') {
            Some(end) => code = rest[end + 1..].trim_start(),
            None => return None,
        }
    }
    if let Some(after) = code.strip_prefix("pub") {
        let rest = after.trim_start();
        if let Some(paren) = rest.strip_prefix('(') {
            match paren.find(')') {
                Some(end) => code = paren[end + 1..].trim_start(),
                None => return None,
            }
        } else if after.is_empty() || after.starts_with(char::is_whitespace) {
            code = rest;
        }
    }
    let rest = code.strip_prefix("mod")?;
    let rest = rest.strip_prefix(char::is_whitespace)?;
    let end = rest
        .find(|c: char| c == ';' || c == '{' || c.is_whitespace())
        .unwrap_or(rest.len());
    let name = rest[..end].trim().trim_start_matches("r#");
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// R1 (D-16, DS-07): no Funnel file or module under `ds/src`, and the crate
/// documentation carries the restriction note.
fn check_funnel(workspace: &Path, problems: &mut Vec<String>) {
    let ds_src = workspace.join("ds/src");
    for path in sorted_files(&ds_src) {
        let display = rel(workspace, &path);
        let under = path.strip_prefix(&ds_src).unwrap_or(&path);
        let names_funnel = under
            .components()
            .any(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase().contains("funnel"));
        if names_funnel {
            problems.push(format!(
                "{display}: the path names Funnel Hashing; ds/ does not build it (D-16, DS-07): \
                 write-once ephemeral scratchpads only, never the page cache, FidTable or a Venti \
                 index"
            ));
        }
        if display.ends_with(".rs") {
            if let Some(text) = read_file(&path, &display, problems) {
                for (n, line) in text.lines().enumerate() {
                    if let Some(name) = declared_module_name(line) {
                        if name.to_ascii_lowercase().contains("funnel") {
                            problems.push(format!(
                                "{display}:{}: declares module `{name}`; Funnel Hashing is not \
                                 built in ds/ (D-16, DS-07)",
                                n + 1
                            ));
                        }
                    }
                }
            }
        }
    }

    let lib = "ds/src/lib.rs";
    if let Some(text) = read_file(&workspace.join(lib), lib, problems) {
        if !text.contains("Funnel Hashing") {
            problems.push(format!(
                "{lib}: missing the `Funnel Hashing` restriction note (D-16, DS-07): the crate \
                 documentation must say Funnel Hashing is not built, and that a future \
                 implementation serves write-once ephemeral scratchpads only — never the page \
                 cache, `FidTable` or a Venti index"
            ));
        }
        if !text.contains("scratchpad") {
            problems.push(format!(
                "{lib}: the Funnel Hashing note must say `scratchpad` (D-16, DS-07): write-once \
                 ephemeral scratchpads only"
            ));
        }
    }
}

fn scan_token(display: &str, text: &str, problems: &mut Vec<String>, rule: &str) {
    for (n, line) in text.lines().enumerate() {
        let code = strip_line_comment(line);
        if find_whole_word(code, "unsafe").is_some() {
            problems.push(format!(
                "{display}:{}: `unsafe` token outside the approved places — {rule}",
                n + 1
            ));
        }
    }
}

/// R2 (D-10): `crypto/src` holds no `unsafe` token and the crate root
/// carries `#![forbid(unsafe_code)]`.
fn check_crypto_token_rule(workspace: &Path, problems: &mut Vec<String>) {
    let src = workspace.join("crypto/src");
    for path in sorted_files(&src) {
        let display = rel(workspace, &path);
        if let Some(text) = read_file(&path, &display, problems) {
            scan_token(
                &display,
                &text,
                problems,
                "crypto/ is crate-level forbid(unsafe_code) and any unsafe there needs owner approval (D-10)",
            );
        }
    }

    let lib = "crypto/src/lib.rs";
    if let Some(text) = read_file(&workspace.join(lib), lib, problems) {
        if !text.contains("#![forbid(unsafe_code)]") {
            problems.push(format!(
                "{lib}: missing `#![forbid(unsafe_code)]` (D-10): crypto/ is crate-level forbid"
            ));
        }
    }
}

/// R3 (D-09): the token rule for every `.rs` file under `ds/src` outside
/// `ds/src/tpht/`, and the per-module forbid layout in `ds/src/lib.rs`.
fn check_ds_token_and_forbid_rules(workspace: &Path, problems: &mut Vec<String>) {
    let src = workspace.join("ds/src");
    let tpht = src.join("tpht");
    for path in sorted_files(&src) {
        if path.starts_with(&tpht) {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let display = rel(workspace, &path);
        if let Some(text) = read_file(&path, &display, problems) {
            scan_token(
                &display,
                &text,
                problems,
                "only ds/src/tpht/ may hold unsafe in ds/ (D-09)",
            );
        }
    }

    let lib = "ds/src/lib.rs";
    if let Some(text) = read_file(&workspace.join(lib), lib, problems) {
        let lines: Vec<&str> = text.lines().collect();
        for (n, line) in lines.iter().enumerate() {
            let Some(name) = declared_module_name(line) else { continue };
            let same_line = strip_line_comment(line).contains("#[forbid(unsafe_code)]");
            let previous_line =
                n > 0 && strip_line_comment(lines[n - 1]).contains("#[forbid(unsafe_code)]");
            if name == "tpht" {
                if same_line || previous_line {
                    problems.push(format!(
                        "{lib}:{}: `tpht` carries `#[forbid(unsafe_code)]`; tpht is the one ds \
                         module that may hold unsafe (D-09)",
                        n + 1
                    ));
                }
            } else if !same_line && !previous_line {
                problems.push(format!(
                    "{lib}:{}: module `{name}` lacks `#[forbid(unsafe_code)]` on the line before it \
                     or the same line (D-09)",
                    n + 1
                ));
            }
        }
    }
}

/// The dependency key a table header introduces, e.g. `crypto` for
/// `[dependencies.crypto]` or `[target.'cfg(unix)'.dev-dependencies.crypto]`.
fn dependency_key_in_table(table: &str) -> Option<&str> {
    let segments: Vec<&str> = table.split('.').collect();
    for (i, segment) in segments.iter().enumerate() {
        if matches!(*segment, "dependencies" | "dev-dependencies" | "build-dependencies") {
            return segments.get(i + 1).copied();
        }
    }
    None
}

/// Is this header a dependency table (as opposed to a table that names one
/// dependency, such as `[dependencies.crypto]`)?
fn is_dependency_table(table: &str) -> bool {
    table
        .split('.')
        .any(|segment| matches!(segment, "dependencies" | "dev-dependencies" | "build-dependencies"))
        && dependency_key_in_table(table).is_none()
}

fn is_dev_dependency_table(table: &str) -> bool {
    table.split('.').any(|segment| segment == "dev-dependencies")
}

/// R4 (D-22, D-25): only `port` may depend on `crypto` or `ds`, in any
/// dependency table; the three arch crates may not name either key and reach
/// them through `port`.
fn check_kernel_dependencies(workspace: &Path, problems: &mut Vec<String>) {
    for kernel in ["port", "aarch64", "x86_64", "riscv64"] {
        // `port` is the one kernel crate the edge belongs to (D-22); every
        // arch crate reaches crypto/ and ds/ through it.
        if kernel == "port" {
            continue;
        }
        let display = format!("{kernel}/Cargo.toml");
        let Some(text) = read_file(&workspace.join(&display), &display, problems) else {
            continue;
        };
        let mut table = String::new();
        for (n, line) in text.lines().enumerate() {
            let code = strip_hash_comment(line).trim();
            if code.is_empty() {
                continue;
            }
            if code.starts_with('[') && code.ends_with(']') {
                table = code[1..code.len() - 1].trim().to_string();
                if let Some(dep) = dependency_key_in_table(&table) {
                    if dep == "crypto" || dep == "ds" {
                        problems.push(format!(
                            "{display}:{}: arch crate dependency `{dep}`; only `port` may depend \
                             on crypto/ or ds/ (Phase 4 D-22); arch crates reach them through \
                             `port`",
                            n + 1
                        ));
                    }
                }
                continue;
            }
            if !is_dependency_table(&table) {
                continue;
            }
            let key = code.split('=').next().unwrap_or("").trim().trim_matches('"');
            if key == "crypto" || key == "ds" {
                problems.push(format!(
                    "{display}:{}: arch crate dependency `{key}`; only `port` may depend on \
                     crypto/ or ds/ (Phase 4 D-22); arch crates reach them through `port`",
                    n + 1
                ));
            }
        }
    }
}

/// R5 (RESEARCH C3, F3): the pinned toolchain lists no `miri` component and
/// `.cargo/config.toml` names neither soft-backend cfg.
fn check_toolchain_and_cargo_config(workspace: &Path, problems: &mut Vec<String>) {
    let toolchain = "rust-toolchain.toml";
    if let Some(text) = read_file(&workspace.join(toolchain), toolchain, problems) {
        for (n, line) in text.lines().enumerate() {
            let code = strip_hash_comment(line);
            if code.contains("\"miri\"") || code.contains("'miri'") {
                problems.push(format!(
                    "{toolchain}:{}: lists the `miri` component; the kernel toolchain must not \
                     change — Miri comes from the second derivation `toolchainMiri`, exported as \
                     R9_MIRI_TOOLCHAIN (D-13, RESEARCH C3 and Pitfall 2)",
                    n + 1
                ));
            }
        }
    }

    let cargo_config = ".cargo/config.toml";
    if let Some(text) = read_file(&workspace.join(cargo_config), cargo_config, problems) {
        for (n, line) in text.lines().enumerate() {
            let code = strip_hash_comment(line);
            for cfg in ["sha2_backend", "chacha20_backend"] {
                if code.contains(cfg) {
                    problems.push(format!(
                        "{cargo_config}:{}: names `{cfg}`; backend cfgs belong in the per-leg \
                         `--config build.rustflags` argument, never .cargo/config.toml (a global \
                         entry would reach the kernel builds, RESEARCH F3)",
                        n + 1
                    ));
                }
            }
        }
    }
}

fn has_lints_opt_in(text: &str) -> bool {
    let mut in_lints = false;
    for line in text.lines() {
        let code = strip_hash_comment(line).trim();
        if code.is_empty() {
            continue;
        }
        if code.starts_with('[') && code.ends_with(']') {
            in_lints = code == "[lints]";
            continue;
        }
        if in_lints && code.replace(' ', "") == "workspace=true" {
            return true;
        }
    }
    false
}

/// R6 (D-11, D-19; Phase 5 CORR-05): `crypto/`, `ds/` and both compiled arch
/// crates (`aarch64`, `x86_64`) opt into the workspace lints; `roaring`
/// lives only under `ds`'s `[dev-dependencies]` and never in `crypto`.
fn check_lints_and_roaring(workspace: &Path, problems: &mut Vec<String>) {
    for display in [
        "crypto/Cargo.toml",
        "ds/Cargo.toml",
        "aarch64/Cargo.toml",
        "x86_64/Cargo.toml",
    ] {
        let Some(text) = read_file(&workspace.join(display), display, problems) else {
            continue;
        };
        if !has_lints_opt_in(&text) {
            problems.push(format!(
                "{display}: missing `[lints]` with `workspace = true` (D-11, CORR-05): the gated \
                 crates opt into the workspace lints (undocumented_unsafe_blocks = deny)"
            ));
        }
    }

    let ds = "ds/Cargo.toml";
    if let Some(text) = read_file(&workspace.join(ds), ds, problems) {
        let mut table = String::new();
        for (n, line) in text.lines().enumerate() {
            let code = strip_hash_comment(line).trim();
            if code.is_empty() {
                continue;
            }
            if code.starts_with('[') && code.ends_with(']') {
                table = code[1..code.len() - 1].trim().to_string();
                if dependency_key_in_table(&table) == Some("roaring")
                    && !is_dev_dependency_table(&table)
                {
                    problems.push(format!(
                        "{ds}:{}: `roaring` in `[{table}]`; it is a host-only test oracle and lives \
                         only under [dev-dependencies] (D-19)",
                        n + 1
                    ));
                }
                continue;
            }
            let key = code.split('=').next().unwrap_or("").trim().trim_matches('"');
            if key == "roaring" && !is_dev_dependency_table(&table) {
                problems.push(format!(
                    "{ds}:{}: `roaring` outside [dev-dependencies]; it is a host-only test oracle \
                     and never links into a kernel (D-19)",
                    n + 1
                ));
            }
        }
    }

    let crypto = "crypto/Cargo.toml";
    if let Some(text) = read_file(&workspace.join(crypto), crypto, problems) {
        for (n, line) in text.lines().enumerate() {
            if strip_hash_comment(line).contains("roaring") {
                problems.push(format!(
                    "{crypto}:{}: names `roaring`; the oracle belongs only to ds's \
                     [dev-dependencies] (D-19)",
                    n + 1
                ));
            }
        }
    }
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
            // The two compiled arch crates carry the R6 opt-in (CORR-05);
            // port and riscv64 are outside R6's scan.
            let lints = if matches!(kernel, "aarch64" | "x86_64") {
                "\n[lints]\nworkspace = true\n"
            } else {
                ""
            };
            write(
                &dir,
                &format!("{kernel}/Cargo.toml"),
                &format!(
                    "[package]\nname = \"x\"\n\n[dependencies]\nport = {{ path = \"../port\" }}\n{lints}"
                ),
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
        // Only the three arch crates are rejected under R4 (D-22); `port` is
        // the crate the edge belongs to and is covered by the accepting test.
        for kernel in ["aarch64", "x86_64", "riscv64"] {
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
                    .expect_err("an arch crate depending on crypto or ds must be rejected")
                    .to_string();
                assert!(
                    text.contains(&format!("{kernel}/Cargo.toml")),
                    "{kernel} dependency named: {text}"
                );
                let _ = fs::remove_dir_all(&dir);
            }
        }

        // The dev-dependency table is a dependency table too, for an arch
        // crate just as for `port` under Phase 3's rule (D-22 keeps it so).
        let dir = fixture("kernel-dep-dev");
        write(
            &dir,
            "aarch64/Cargo.toml",
            "[package]\nname = \"aarch64\"\n\n[dev-dependencies]\nds = { path = \"../ds\" }\n",
        );
        let text = check(&dir)
            .expect_err("an arch crate's dev-dependency table is a dependency table")
            .to_string();
        assert!(text.contains("aarch64/Cargo.toml"), "names the manifest: {text}");
        let _ = fs::remove_dir_all(&dir);

        // The same crate naming `crypto` under `[dependencies]` and `ds` under
        // `[dev-dependencies]` is rejected on both lines.
        let dir = fixture("kernel-dep-mixed-tables");
        write(
            &dir,
            "aarch64/Cargo.toml",
            "[package]\nname = \"aarch64\"\n\n[dependencies]\ncrypto = { path = \"../crypto\" }\n\n\
             [dev-dependencies]\nds = { path = \"../ds\" }\n",
        );
        let text = check(&dir)
            .expect_err("an arch crate naming crypto and ds in two tables is rejected")
            .to_string();
        assert!(
            text.contains("aarch64/Cargo.toml:5:") && text.contains("aarch64/Cargo.toml:8:"),
            "both dependency lines are named: {text}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn lintcontract_accepts_port_depending_on_crypto_and_ds() {
        // `port` gains the edge in Phase 4 (D-22, D-25), in every dependency
        // table: plain, dev and the `[dependencies.<key>]` table form.
        for dep in ["crypto", "ds"] {
            let dir = fixture(&format!("port-dep-{dep}"));
            write(
                &dir,
                "port/Cargo.toml",
                &format!(
                    "[package]\nname = \"port\"\n\n[dependencies]\n{dep} = {{ path = \"../{dep}\" }}\n"
                ),
            );
            check(&dir).expect("port may depend on crypto and ds (Phase 4 D-22)");
            let _ = fs::remove_dir_all(&dir);
        }

        let dir = fixture("port-dep-dev");
        write(
            &dir,
            "port/Cargo.toml",
            "[package]\nname = \"port\"\n\n[dev-dependencies]\nds = { path = \"../ds\" }\n",
        );
        check(&dir).expect("port may carry crypto or ds in a dev-dependency table (Phase 4 D-22)");
        let _ = fs::remove_dir_all(&dir);

        let dir = fixture("port-dep-table");
        write(
            &dir,
            "port/Cargo.toml",
            "[package]\nname = \"port\"\n\n[dependencies.crypto]\npath = \"../crypto\"\n",
        );
        check(&dir).expect("port may carry crypto in the named-table form (Phase 4 D-22)");
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

        // CORR-05: R6 covers both compiled arch crates — dropping the opt-in
        // from either arch manifest must fail the contract.
        for arch in ["aarch64", "x86_64"] {
            let dir = fixture(&format!("lints-{arch}"));
            write(
                &dir,
                &format!("{arch}/Cargo.toml"),
                "[package]\nname = \"x\"\n\n[dependencies]\nport = { path = \"../port\" }\n",
            );
            let text = check(&dir)
                .expect_err("an arch crate without [lints] workspace = true must be rejected")
                .to_string();
            assert!(
                text.contains(&format!("{arch}/Cargo.toml")),
                "names the arch manifest: {text}"
            );
            assert!(text.contains("lints"), "names the missing table: {text}");
            let _ = fs::remove_dir_all(&dir);
        }
    }
}
