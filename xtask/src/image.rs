//! The manifest-driven stage-and-bake engine (D-15, D-16, D-29).
//!
//! One generic engine plus declarative per-image manifests (research F1
//! mitigation): a manifest lists every member of one image, each entry
//! naming its source and image path, and the engine stages exactly those
//! entries into a `target/` staging tree, bakes the `.paq` with the
//! devshell's `mkpaqfs`, and writes the member list the D-29 comparison
//! reads.
//!
//! The tree is READ-ONLY here: sources are copied out of it, never opened
//! for writing (HARN-02's prohibition), and the staging tree, the baked
//! image and the member list all live under `target/`.
//!
//! Manifest shape (schema 1):
//!
//! ```toml
//! schema = 1
//! arch = "arm64"          # or "amd64"
//! image = "smoke"
//!
//! [[file]]
//! path = "/init"          # absolute path inside the image
//! source = "harvey"       # harvey | rust | generated | tree
//! from = "build/aarch64/smoke-out/init"   # tree-relative source
//! mode = "0775"           # octal string
//!
//! [[dir]]
//! path = "/proc"
//! ```
//!
//! `source`'s `rust` / `generated` / `tree` kinds are declared so plans
//! 01-11 / 01-16 / 01-20 add them without a schema break; this plan's
//! engine stages `harvey` entries and returns a named error for any other
//! kind rather than silently skipping it.

use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::Result;

/// One image's declarative manifest.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Manifest schema version; only 1 is understood.
    pub schema: u32,
    /// Guest architecture: `arm64` or `amd64`.
    pub arch: String,
    /// The image name (used in error text and the member-list record).
    pub image: String,
    /// The image's files.
    #[serde(default)]
    pub file: Vec<FileEntry>,
    /// The image's bare directories.
    #[serde(default)]
    pub dir: Vec<DirEntry>,
}

/// One file entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    /// Absolute path inside the image (must start with `/`).
    pub path: String,
    /// Where the bytes come from.
    pub source: Source,
    /// Tree-relative source path (never absolute, never `..`).
    pub from: String,
    /// Octal mode string, e.g. `"0755"`.
    pub mode: String,
}

/// The supported provenance kinds (D-15's Harvey/Rust marking; the
/// extra kinds are the later plans' declarative slots).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// A file in the resolved Harvey tree.
    Harvey,
    /// An r9 Rust build artifact (plans 01-11/01-16/01-20).
    Rust,
    /// A generated file (plans 01-11/01-16/01-20).
    Generated,
    /// A file in an r9 tree fixture (plans 01-11/01-16/01-20).
    Tree,
}

/// One bare directory entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirEntry {
    /// Absolute path inside the image (must start with `/`).
    pub path: String,
}

/// One member of a staged image, as the D-29 member list records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Path relative to the image root (no leading `/`).
    pub path: String,
    /// `true` for a directory.
    pub is_dir: bool,
    /// Permission bits (octal value, e.g. `0o775`).
    pub mode: u32,
    /// File size in bytes (`None` for a directory).
    pub size: Option<u64>,
    /// Lowercase hex sha256 (`None` for a directory).
    pub sha256: Option<String>,
}

impl Member {
    /// The one-line member-list record:
    /// `<path> file <mode> <size> <sha256>` or `<path> dir <mode> - -`.
    /// Mode prints as octal without zero padding (`stat %a`'s shape), so a
    /// shell-produced reference list is byte-comparable.
    pub fn line(&self) -> String {
        let mode = format!("{:o}", self.mode);
        match (self.is_dir, self.size, &self.sha256) {
            (true, _, _) => format!("{} dir {mode} - -", self.path),
            (false, size, sha) => format!(
                "{} file {mode} {} {}",
                self.path,
                size.unwrap_or(0),
                sha.as_deref().unwrap_or("-")
            ),
        }
    }
}

/// Parse and validate a manifest (schema 1).
///
/// Rejects, with a named error: unknown keys (every table is
/// `deny_unknown_fields`), a schema other than 1, an unknown arch, an
/// image path that does not start with `/` or names a `..`/`.` component,
/// a `from` that is absolute or names a `..`/`.` component, a malformed
/// octal mode, and duplicate image paths.
pub fn parse_manifest(text: &str) -> Result<Manifest> {
    let manifest: Manifest =
        toml::from_str(text).map_err(|e| -> crate::DynError {
            format!("image manifest: {e}").into()
        })?;
    validate(&manifest)?;
    Ok(manifest)
}

/// The validation half of [`parse_manifest`] (also useful for a manifest
/// built in code by a later plan).
pub fn validate(manifest: &Manifest) -> Result<()> {
    if manifest.schema != 1 {
        return Err(format!(
            "image manifest for \"{}\": schema = {} is not understood — only schema = 1 exists",
            manifest.image, manifest.schema
        )
        .into());
    }
    match manifest.arch.as_str() {
        "arm64" | "amd64" => {}
        other => {
            return Err(format!(
                "image manifest for \"{}\": arch = {other:?} is not one of \"arm64\", \"amd64\"",
                manifest.image
            )
            .into())
        }
    }
    let mut seen: HashSet<&str> = HashSet::new();
    for file in &manifest.file {
        validate_image_path(&file.path, &manifest.image, "[[file]] path")?;
        if !seen.insert(&file.path) {
            return Err(format!(
                "image manifest for \"{}\": duplicate image path {} (one entry per member)",
                manifest.image, file.path
            )
            .into());
        }
        validate_relative_source(&file.from, &manifest.image, &file.path)?;
        parse_mode(&file.mode, &manifest.image, &file.path)?;
    }
    for dir in &manifest.dir {
        validate_image_path(&dir.path, &manifest.image, "[[dir]] path")?;
        if !seen.insert(&dir.path) {
            return Err(format!(
                "image manifest for \"{}\": duplicate image path {} (one entry per member)",
                manifest.image, dir.path
            )
            .into());
        }
    }
    Ok(())
}

/// An image path is absolute inside the image and carries no `.` or `..`
/// component (T-01-27: a manifest cannot escape the staging root).
fn validate_image_path(path: &str, image: &str, what: &str) -> Result<()> {
    let mut components = Path::new(path).components();
    if !matches!(components.next(), Some(Component::RootDir)) {
        return Err(format!(
            "image manifest for \"{image}\": {what} {path:?} must start with `/` (an absolute \
             path inside the image)"
        )
        .into());
    }
    for component in components {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(format!(
                    "image manifest for \"{image}\": {what} {path:?} carries a `.` or `..` \
                     component — image paths are normal components only"
                )
                .into())
            }
        }
    }
    Ok(())
}

/// A `from` value is tree-relative: no root, no `.`/`..`, no empty path.
fn validate_relative_source(from: &str, image: &str, path: &str) -> Result<()> {
    let components: Vec<Component> = Path::new(from).components().collect();
    if components.is_empty() {
        return Err(format!(
            "image manifest for \"{image}\": entry {path} has an empty `from` — name the \
             tree-relative source file"
        )
        .into());
    }
    for component in components {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(format!(
                    "image manifest for \"{image}\": entry {path} `from` = {from:?} is not a \
                     plain tree-relative path (absolute or `.`/`..` components are rejected)"
                )
                .into())
            }
        }
    }
    Ok(())
}

/// Parse an octal mode string.
fn parse_mode(mode: &str, image: &str, path: &str) -> Result<u32> {
    match u32::from_str_radix(mode, 8) {
        Ok(bits) if bits <= 0o7777 => Ok(bits),
        _ => Err(format!(
            "image manifest for \"{image}\": entry {path} mode = {mode:?} is not an octal mode \
             (e.g. \"0755\")"
        )
        .into()),
    }
}

/// Stage `manifest` from `tree` into `stage_dir`.
///
/// `stage_dir` is recreated from scratch on every call.  Each `harvey`
/// entry is copied from `<tree>/<from>` to `<stage_dir>/<path>` with the
/// manifest's mode; each `dir` entry is created.  A missing source is a
/// named error naming the source path and the manifest.  Nothing under
/// `tree` is ever opened for writing, and a source symlink that resolves
/// outside the tree is refused (T-01-27).
pub fn stage(manifest: &Manifest, tree: &Path, stage_dir: &Path) -> Result<()> {
    let canonical_tree = tree
        .canonicalize()
        .map_err(|e| -> crate::DynError {
            format!("image manifest for \"{}\": Harvey tree {}: {e}", manifest.image, tree.display())
                .into()
        })?;
    if stage_dir.exists() {
        fs::remove_dir_all(stage_dir)?;
    }
    fs::create_dir_all(stage_dir)?;
    for dir in &manifest.dir {
        fs::create_dir_all(stage_dir.join(dir.path.trim_start_matches('/')))?;
    }
    for file in &manifest.file {
        if file.source != Source::Harvey {
            return Err(format!(
                "image manifest for \"{}\": entry {} names source = {:?}; this engine stages \
                 harvey entries only (the rust/generated/tree kinds land with plans \
                 01-11/01-16/01-20)",
                manifest.image, file.path, file.source
            )
            .into());
        }
        let src = tree.join(&file.from);
        if !src.is_file() {
            return Err(format!(
                "image manifest for \"{}\": source {} for {} missing at {} — the resolved Harvey \
                 tree does not carry it (the manifest names the producer)",
                manifest.image,
                file.from,
                file.path,
                src.display()
            )
            .into());
        }
        let canonical_src = src.canonicalize().map_err(|e| -> crate::DynError {
            format!("image manifest for \"{}\": source {}: {e}", manifest.image, src.display())
                .into()
        })?;
        if !canonical_src.starts_with(&canonical_tree) {
            return Err(format!(
                "image manifest for \"{}\": source {} resolves to {} outside the Harvey tree {}",
                manifest.image,
                file.from,
                canonical_src.display(),
                canonical_tree.display()
            )
            .into());
        }
        let dest = stage_dir.join(file.path.trim_start_matches('/'));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&canonical_src, &dest)?;
        let mode = parse_mode(&file.mode, &manifest.image, &file.path)?;
        fs::set_permissions(&dest, fs::Permissions::from_mode(mode))?;
    }
    Ok(())
}

/// The sorted member list of a staged image tree.
///
/// Records path, size, sha256 and octal mode for files, plus `dir` lines
/// for directories; paths are relative to `stage_dir` (the image root),
/// with no leading `/`, and the list is sorted by path.
pub fn member_list(stage_dir: &Path) -> Result<Vec<Member>> {
    let mut members = Vec::new();
    walk(stage_dir, "", &mut members)?;
    members.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(members)
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<Member>) -> Result<()> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if prefix.is_empty() { name } else { format!("{prefix}/{name}") };
        let meta = entry.metadata()?;
        let mode = meta.permissions().mode() & 0o7777;
        if meta.is_dir() {
            out.push(Member { path: path.clone(), is_dir: true, mode, size: None, sha256: None });
            walk(&entry.path(), &path, out)?;
        } else {
            let file = entry.path();
            out.push(Member {
                path,
                is_dir: false,
                mode,
                size: Some(meta.len()),
                sha256: Some(sha256_file(&file)?),
            });
        }
    }
    Ok(())
}

/// Write a member list, one [`Member::line`] per line, creating the
/// parent directory as needed.
pub fn write_members(list: &[Member], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = String::new();
    for member in list {
        text.push_str(&member.line());
        text.push('\n');
    }
    fs::write(path, text)?;
    Ok(())
}

/// Bake `stage_dir` into the `.paq` at `out` with the host `mkpaqfs`.
pub fn bake(mkpaqfs: &Path, stage_dir: &Path, out: &Path) -> Result<()> {
    let mut cmd = Command::new(mkpaqfs);
    cmd.arg("-o").arg(out).arg(stage_dir);
    let status = cmd.status().map_err(|e| -> crate::DynError {
        format!("spawn mkpaqfs {}: {e}", mkpaqfs.display()).into()
    })?;
    if !status.success() {
        return Err(format!("mkpaqfs {} -o {} {} failed: {status}", mkpaqfs.display(), out.display(), stage_dir.display()).into());
    }
    Ok(())
}

/// The tree paths a manifest's bake depends on (the staleness check reads
/// their mtimes).  Only the kinds this engine stages are listed.
pub fn input_paths(manifest: &Manifest, tree: &Path) -> Vec<PathBuf> {
    manifest
        .file
        .iter()
        .filter(|file| file.source == Source::Harvey)
        .map(|file| tree.join(&file.from))
        .collect()
}

/// sha256 of a file via a host tool — no new crates.  coreutils
/// `sha256sum` (nix devshell, linux) first, macOS `shasum` as fallback.
pub fn sha256_file(path: &Path) -> Result<String> {
    let out = Command::new("sha256sum")
        .arg(path)
        .output()
        .or_else(|_| Command::new("shasum").arg("-a").arg("256").arg(path).output())
        .map_err(|e| -> crate::DynError {
            format!("no sha256 tool on PATH (tried sha256sum, shasum): {e}").into()
        })?;
    if !out.status.success() {
        return Err(format!("sha256 of {} failed: {}", path.display(), out.status).into());
    }
    let text = String::from_utf8(out.stdout)?;
    Ok(text.split_whitespace().next().unwrap_or("").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> String {
        "\
schema = 1
arch = \"arm64\"
image = \"smoke\"

[[file]]
path = \"/init\"
source = \"harvey\"
from = \"build/aarch64/smoke-out/init\"
mode = \"0775\"

[[file]]
path = \"/hello\"
source = \"harvey\"
from = \"build/aarch64/smoke-out/hello\"
mode = \"0755\"

[[dir]]
path = \"/proc\"
"
        .to_string()
    }

    /// A per-test scratch root under the system temp dir, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("r9-image-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn manifest_accepts_schema_one_with_files_and_dirs() {
        let manifest = parse_manifest(&sample_manifest()).unwrap();
        assert_eq!(manifest.schema, 1);
        assert_eq!(manifest.arch, "arm64");
        assert_eq!(manifest.image, "smoke");
        assert_eq!(manifest.file.len(), 2);
        assert_eq!(manifest.dir.len(), 1);
        assert_eq!(manifest.file[0].path, "/init");
        assert_eq!(manifest.file[0].source, Source::Harvey);
    }

    #[test]
    fn manifest_rejects_unknown_keys() {
        let text = sample_manifest().replace("schema = 1", "schema = 1\nbogus = true");
        let err = parse_manifest(&text).unwrap_err().to_string();
        assert!(err.contains("bogus"), "{err}");
    }

    #[test]
    fn manifest_rejects_absolute_from() {
        let text = sample_manifest().replace(
            "from = \"build/aarch64/smoke-out/init\"",
            "from = \"/etc/passwd\"",
        );
        let err = parse_manifest(&text).unwrap_err().to_string();
        assert!(err.contains("/init"), "{err}");
        assert!(err.contains("tree-relative"), "{err}");
    }

    #[test]
    fn manifest_rejects_dotdot_from() {
        let text = sample_manifest().replace(
            "from = \"build/aarch64/smoke-out/init\"",
            "from = \"build/../../outside\"",
        );
        let err = parse_manifest(&text).unwrap_err().to_string();
        assert!(err.contains(".."), "{err}");
    }

    #[test]
    fn manifest_rejects_non_absolute_image_path() {
        let text = sample_manifest().replace("path = \"/init\"", "path = \"init\"");
        let err = parse_manifest(&text).unwrap_err().to_string();
        assert!(err.contains("must start with `/`"), "{err}");
    }

    #[test]
    fn manifest_rejects_duplicate_image_paths() {
        let text = sample_manifest().replace("path = \"/hello\"", "path = \"/init\"");
        let err = parse_manifest(&text).unwrap_err().to_string();
        assert!(err.contains("duplicate image path"), "{err}");
    }

    #[test]
    fn manifest_rejects_a_bad_octal_mode() {
        let text = sample_manifest().replace("mode = \"0755\"", "mode = \"u+rwx\"");
        let err = parse_manifest(&text).unwrap_err().to_string();
        assert!(err.contains("octal mode"), "{err}");
    }

    #[test]
    fn stage_copies_entries_with_manifest_mode_and_creates_dirs() {
        let scratch = Scratch::new("stage");
        let tree = scratch.0.join("tree");
        fs::create_dir_all(tree.join("build/aarch64/smoke-out")).unwrap();
        fs::write(tree.join("build/aarch64/smoke-out/init"), b"init-bytes").unwrap();
        fs::write(tree.join("build/aarch64/smoke-out/hello"), b"hello").unwrap();

        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage(&manifest, &tree, &stage_dir).unwrap();

        assert_eq!(fs::read(stage_dir.join("init")).unwrap(), b"init-bytes");
        assert_eq!(
            fs::metadata(stage_dir.join("init")).unwrap().permissions().mode() & 0o777,
            0o775
        );
        assert_eq!(
            fs::metadata(stage_dir.join("hello")).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert!(stage_dir.join("proc").is_dir());
        // The tree source is untouched (read-only discipline).
        assert_eq!(fs::read(tree.join("build/aarch64/smoke-out/init")).unwrap(), b"init-bytes");
    }

    #[test]
    fn stage_recreates_a_stale_stage_dir() {
        let scratch = Scratch::new("restage");
        let tree = scratch.0.join("tree");
        fs::create_dir_all(tree.join("build/aarch64/smoke-out")).unwrap();
        fs::write(tree.join("build/aarch64/smoke-out/init"), b"init").unwrap();
        fs::write(tree.join("build/aarch64/smoke-out/hello"), b"hello").unwrap();
        let stage_dir = scratch.0.join("stage");
        fs::create_dir_all(stage_dir.join("leftover")).unwrap();
        fs::write(stage_dir.join("leftover/stale"), b"stale").unwrap();

        let manifest = parse_manifest(&sample_manifest()).unwrap();
        stage(&manifest, &tree, &stage_dir).unwrap();
        assert!(!stage_dir.join("leftover").exists());
    }

    #[test]
    fn stage_names_the_missing_source_and_the_manifest() {
        let scratch = Scratch::new("missing");
        let tree = scratch.0.join("tree");
        fs::create_dir_all(tree.join("build/aarch64/smoke-out")).unwrap();
        fs::write(tree.join("build/aarch64/smoke-out/init"), b"init").unwrap();
        // hello is deliberately absent.

        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let err = stage(&manifest, &tree, &scratch.0.join("stage")).unwrap_err().to_string();
        assert!(err.contains("smoke"), "{err}");
        assert!(err.contains("build/aarch64/smoke-out/hello"), "{err}");
        assert!(err.contains("/hello"), "{err}");
    }

    #[test]
    fn member_list_is_sorted_and_records_size_sha_mode_and_dirs() {
        let scratch = Scratch::new("members");
        let tree = scratch.0.join("tree");
        fs::create_dir_all(tree.join("build/aarch64/smoke-out")).unwrap();
        fs::write(tree.join("build/aarch64/smoke-out/init"), b"init-bytes").unwrap();
        fs::write(tree.join("build/aarch64/smoke-out/hello"), b"hello").unwrap();

        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage(&manifest, &tree, &stage_dir).unwrap();
        let members = member_list(&stage_dir).unwrap();

        let paths: Vec<&str> = members.iter().map(|m| m.path.as_str()).collect();
        assert_eq!(paths, ["hello", "init", "proc"]);

        let hello = &members[0];
        assert!(!hello.is_dir);
        assert_eq!(hello.mode, 0o755);
        assert_eq!(hello.size, Some(5));
        assert_eq!(hello.sha256.as_deref(), Some(sha256_file(&stage_dir.join("hello")).unwrap().as_str()));

        let init = &members[1];
        assert_eq!(init.mode, 0o775);
        assert_eq!(init.size, Some(10));

        let proc = &members[2];
        assert!(proc.is_dir);
        assert_eq!(proc.size, None);
        assert_eq!(proc.sha256, None);
        assert_eq!(proc.line(), format!("proc dir {:o} - -", 0o755));
        assert_eq!(hello.line().split(' ').count(), 5);

        // Writing the list round-trips the exact lines.
        let list_path = scratch.0.join("out/smoke.members");
        write_members(&members, &list_path).unwrap();
        let text = fs::read_to_string(&list_path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("hello file 755 5 "), "{}", lines[0]);
    }

    #[test]
    fn input_paths_lists_only_harvey_sources() {
        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let tree = Path::new("/tree");
        let inputs = input_paths(&manifest, tree);
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0], Path::new("/tree/build/aarch64/smoke-out/init"));
    }
}
