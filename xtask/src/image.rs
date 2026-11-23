//! The manifest-driven stage-and-bake engine, hardened (D-15, D-16, D-29, HARN-05).
//!
//! One generic engine plus declarative per-image manifests (research F1
//! mitigation): a manifest lists every member of one image, each entry
//! naming its source and image path, and the engine stages exactly those
//! entries into a `target/` staging tree, verifies that manifest and image
//! agree in BOTH directions, bakes the `.paq` with the devshell's `mkpaqfs`,
//! and writes the member list the D-29 comparison reads.
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
//! [[file]]
//! path = "/amd64/bin/ps"
//! source = "rust"
//! from = "probe/ps.aout"  # workspace-relative r9 artifact
//! replaces = "amd64/bin/ps"               # the Harvey artifact it stands in for
//! mode = "0775"                           # optional, defaults to 0755
//!
//! [[file]]
//! path = "/etc/generated"
//! source = "generated"
//! generator = "UNRESOLVED"                # rejected until plan 01-16 registers generators
//!
//! [[tree]]
//! path = "/lib/ndb"                        # image directory
//! from = "amd64/lib/ndb"                   # tree-relative source directory
//! members = "smoke-tree.members"           # beside the manifest: `f <rel> <size> <sha256> <mode>`
//!
//! [[library]]
//! name = "libc"                            # tier-0 library audit row (HARN-03)
//! from = "amd64/lib/libc.a"
//! in_image = false                         # never staged; libraries are not image members
//!
//! [[dir]]
//! path = "/proc"
//! ```
//!
//! HARN-05's enforcement is structural, not documentation: a staged file no
//! entry produces is `IMAGE-DRIFT unlisted`; a declared entry that is not
//! staged is `IMAGE-DRIFT missing`; a `[[tree]]` member whose bytes disagree
//! with its members record is `IMAGE-DRIFT changed`; a `rust` source under a
//! Harvey tree root, or byte-identical to the artifact it replaces, is
//! `IMAGE-PROVENANCE`; a source escaping its allowed root is `IMAGE-PATH`;
//! and a generated entry naming no registered generator (including the
//! reserved `UNRESOLVED` the manifest emitter writes) is `IMAGE-GENERATOR`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::Result;

/// The generator registry.  Plan 01-16 registers the real generators; this
/// plan ships the registry and the named unknown-name error, so a manifest
/// can never stage a generated file from a typo'd or unresolved name.
const GENERATORS: &[&str] = &[];

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
    /// Source directories expanded into the image (D-15: the whole file set).
    #[serde(default)]
    pub tree: Vec<TreeEntry>,
    /// Tier-0 library audit rows (HARN-03: `in_image = false`).
    #[serde(default)]
    pub library: Vec<LibraryEntry>,
}

/// One file entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    /// Absolute path inside the image (must start with `/`).
    pub path: String,
    /// Where the bytes come from.
    pub source: Source,
    /// Source path: tree-relative for `harvey`, workspace-relative for
    /// `rust`; absent for `generated` (the generator produces the bytes).
    #[serde(default)]
    pub from: Option<String>,
    /// Octal mode string, e.g. `"0755"`.  Required for `harvey` (the
    /// staged binaries' own modes), optional elsewhere (default 0755).
    #[serde(default)]
    pub mode: Option<String>,
    /// For `source = "rust"`: the tree-relative Harvey artifact the r9
    /// build stands in for (HARN-05's provenance comparison).
    #[serde(default)]
    pub replaces: Option<String>,
    /// For `source = "generated"`: the registered generator name.
    #[serde(default)]
    pub generator: Option<String>,
}

/// One `[[tree]]` entry: a source directory expanded to every file under it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeEntry {
    /// Absolute path inside the image (the directory files land under).
    pub path: String,
    /// Tree-relative source directory.
    pub from: String,
    /// Members file beside the manifest: `f <relpath> <size> <sha256> <mode>`.
    pub members: String,
}

/// One `[[library]]` entry (HARN-03's tier-0 library audit table).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryEntry {
    /// The bare library name (`libc`, `libbio`, …).
    pub name: String,
    /// Tree-relative `.a` path.
    pub from: String,
    /// Always `false` in this phase: libraries are not image members.
    pub in_image: bool,
}

/// The supported provenance kinds (D-15's Harvey/Rust marking; `generated`
/// and `tree` are the later plans' declarative slots).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// A file in the resolved Harvey tree.
    Harvey,
    /// An r9 Rust build artifact (workspace-relative `from`, HARN-05).
    Rust,
    /// A generated file (plan 01-16 registers the generators).
    Generated,
    /// Not a file source: a `[[tree]]` table expands a directory.
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
    /// Lowercase hex sha256 (`None` for a directory; the literal `opaque`
    /// for a reference-bake entry compared by path, size and mode only).
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

/// What one assembled image is: its staging tree, baked `.paq`, member
/// list and digest.
#[derive(Debug, Clone)]
pub struct Assembled {
    /// The image's arch (`arm64` / `amd64`).
    pub arch: String,
    /// The image name.
    pub image: String,
    /// The staging tree (mkpaqfs's exact input).
    pub stage_dir: PathBuf,
    /// The baked `.paq`.
    pub paq: PathBuf,
    /// The member list written beside the `.paq`.
    pub members_path: PathBuf,
    /// The sorted member list (D-29).
    pub members: Vec<Member>,
    /// The baked `.paq`'s sha256 (per-bake: mkpaqfs embeds a time field).
    pub sha256: String,
}

/// A manifest emitted from a reference member list.
#[derive(Debug, Clone)]
pub struct Emitted {
    /// The TOML text (`schema = 1`, arch, image, entries).
    pub text: String,
    /// The reference paths no tree file could satisfy (each emitted as a
    /// `generator = "UNRESOLVED"` entry, which `parse_manifest` rejects).
    pub unresolved: Vec<String>,
}

/// Parse and validate a manifest (schema 1).
///
/// Rejects, with a named error: unknown keys (every table is
/// `deny_unknown_fields`), a schema other than 1, an unknown arch, an
/// image path that does not start with `/` or names a `..`/`.` component,
/// a source path that is absolute or names a `..`/`.` component, a
/// malformed octal mode, duplicate image paths, a source kind missing its
/// required fields, a library row claiming `in_image = true`, and an
/// unknown or `UNRESOLVED` generator (`IMAGE-GENERATOR`).
pub fn parse_manifest(text: &str) -> Result<Manifest> {
    let manifest: Manifest = toml::from_str(text).map_err(|e| -> crate::DynError {
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
        validate_file_entry(file, &manifest.image)?;
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
    for tree in &manifest.tree {
        validate_image_path(&tree.path, &manifest.image, "[[tree]] path")?;
        if !seen.insert(&tree.path) {
            return Err(format!(
                "image manifest for \"{}\": duplicate image path {} ([[tree]] and another entry \
                 claim it)",
                manifest.image, tree.path
            )
            .into());
        }
        validate_relative(
            &tree.from,
            &manifest.image,
            &format!("[[tree]] {} `from`", tree.path),
        )?;
        validate_relative(
            &tree.members,
            &manifest.image,
            &format!("[[tree]] {} `members`", tree.path),
        )?;
    }
    for library in &manifest.library {
        if library.name.trim().is_empty() {
            return Err(format!(
                "image manifest for \"{}\": a [[library]] entry has an empty `name`",
                manifest.image
            )
            .into());
        }
        validate_relative(
            &library.from,
            &manifest.image,
            &format!("[[library]] {} `from`", library.name),
        )?;
        if library.in_image {
            return Err(format!(
                "image manifest for \"{}\": [[library]] {} sets in_image = true but carries no \
                 image path — libraries are an audit table (HARN-03), and image members are \
                 [[file]] entries",
                manifest.image, library.name
            )
            .into());
        }
    }
    Ok(())
}

/// Per-source validation: each kind's required fields, absent fields it
/// must not carry, and the generator registry (`IMAGE-GENERATOR`).
fn validate_file_entry(file: &FileEntry, image: &str) -> Result<()> {
    match file.source {
        Source::Harvey => {
            let from = require_field(file, image, "from")?;
            validate_relative(from, image, &format!("entry {} `from`", file.path))?;
            if file.mode.is_none() {
                return Err(format!(
                    "image manifest for \"{image}\": entry {} source = \"harvey\" requires `mode` \
                     (the staged binary's own octal mode, e.g. \"0775\")",
                    file.path
                )
                .into());
            }
            reject_field(file, image, file.generator.as_deref(), "generator", "harvey")?;
            reject_field(file, image, file.replaces.as_deref(), "replaces", "harvey")?;
        }
        Source::Rust => {
            let from = require_field(file, image, "from")?;
            validate_relative(from, image, &format!("entry {} `from`", file.path))?;
            let replaces = require_field(file, image, "replaces")?;
            validate_relative(replaces, image, &format!("entry {} `replaces`", file.path))?;
            reject_field(file, image, file.generator.as_deref(), "generator", "rust")?;
        }
        Source::Generated => {
            let generator = require_field(file, image, "generator")?;
            known_generator(generator, image, &file.path)?;
            reject_field(file, image, file.from.as_deref(), "from", "generated")?;
            reject_field(file, image, file.replaces.as_deref(), "replaces", "generated")?;
        }
        Source::Tree => {
            return Err(format!(
                "image manifest for \"{image}\": entry {} names source = \"tree\"; a [[tree]] \
                 table expands a directory, use that instead",
                file.path
            )
            .into())
        }
    }
    if let Some(mode) = &file.mode {
        parse_mode(mode, image, &file.path)?;
    }
    Ok(())
}

/// A required per-source field.
fn require_field<'a>(file: &'a FileEntry, image: &str, field: &str) -> Result<&'a str> {
    file_from(file, field).ok_or_else(|| -> crate::DynError {
        format!(
            "image manifest for \"{image}\": entry {} source = {:?} requires `{field}`",
            file.path, file.source
        )
        .into()
    })
}

fn file_from<'a>(file: &'a FileEntry, field: &str) -> Option<&'a str> {
    let value = match field {
        "from" => file.from.as_deref(),
        "replaces" => file.replaces.as_deref(),
        "generator" => file.generator.as_deref(),
        _ => None,
    };
    value.filter(|v| !v.is_empty())
}

/// A field a source kind must not carry (a stale key is a manifest defect,
/// never silently ignored).
fn reject_field(
    file: &FileEntry,
    image: &str,
    value: Option<&str>,
    field: &str,
    source: &str,
) -> Result<()> {
    match value {
        Some(value) => Err(format!(
            "image manifest for \"{image}\": entry {} source = \"{source}\" must not carry \
             `{field}` = {value:?}",
            file.path
        )
        .into()),
        None => Ok(()),
    }
}

/// The generator registry check (`IMAGE-GENERATOR`): `UNRESOLVED` is the
/// emitter's reserved "no tree file matched" marker and is always rejected;
/// every other name must be registered (none are, in this plan).
fn known_generator(name: &str, image: &str, path: &str) -> Result<()> {
    if name == "UNRESOLVED" {
        return Err(format!(
            "IMAGE-GENERATOR: image \"{image}\" entry {path} names generator \"UNRESOLVED\" — the \
             manifest emitter writes that marker for a reference file it could not match to the \
             tree; resolve it before the image can be assembled (plans 01-16/01-17)"
        )
        .into());
    }
    if !GENERATORS.contains(&name) {
        return Err(format!(
            "IMAGE-GENERATOR: image \"{image}\" entry {path} names unknown generator {name:?} — \
             the registry is empty in plan 01-11; plan 01-16 registers the real generators"
        )
        .into());
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

/// A source path is relative: no root, no `.`/`..`, no empty path.
fn validate_relative(from: &str, image: &str, what: &str) -> Result<()> {
    let components: Vec<Component> = Path::new(from).components().collect();
    if components.is_empty() {
        return Err(format!(
            "image manifest for \"{image}\": {what} is an empty path — name the source file"
        )
        .into());
    }
    for component in components {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(format!(
                    "image manifest for \"{image}\": {what} = {from:?} is not a plain \
                     tree-relative path (absolute or `.`/`..` components are rejected)"
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

/// One parsed `[[tree]]` members-file line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeMember {
    rel: String,
    size: u64,
    sha256: String,
    mode: u32,
}

/// Parse a `[[tree]]` members file: one `f <relpath> <size> <sha256> <mode>`
/// line per file, blank lines and `#` comments allowed.  A malformed line is
/// a named error — a members file that half-parses is never trusted.
fn parse_tree_members(text: &str, image: &str, path: &Path) -> Result<Vec<TreeMember>> {
    let mut members = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 5 || fields[0] != "f" {
            return Err(format!(
                "image manifest for \"{image}\": members file {} line {} is not \
                 `f <relpath> <size> <sha256> <mode>`: {line:?}",
                path.display(),
                index + 1
            )
            .into());
        }
        let rel = fields[1].to_string();
        validate_relative(&rel, image, &format!("members file {} entry", path.display()))?;
        let size = fields[2].parse::<u64>().map_err(|_| -> crate::DynError {
            format!(
                "image manifest for \"{image}\": members file {} line {} size {:?} is not a \
                 byte count",
                path.display(),
                index + 1,
                fields[2]
            )
            .into()
        })?;
        let mode = u32::from_str_radix(fields[4], 8).map_err(|_| -> crate::DynError {
            format!(
                "image manifest for \"{image}\": members file {} line {} mode {:?} is not octal",
                path.display(),
                index + 1,
                fields[4]
            )
            .into()
        })?;
        members.push(TreeMember { rel, size, sha256: fields[3].to_string(), mode });
    }
    Ok(members)
}

/// Stage `manifest` from `tree` (and, for `rust` entries, `workspace`) into
/// `stage_dir`.
///
/// `stage_dir` is recreated from scratch on every call.  Each `harvey`
/// entry is copied from `<tree>/<from>`, each `rust` entry from
/// `<workspace>/<from>`, each `dir` entry created, and each `[[tree]]`
/// source directory expanded to every file under it (the members file
/// supplies the mode a listed file must carry).  A missing source is
/// `IMAGE-DRIFT missing` naming the image, the image path and the source;
/// a symlink resolving outside its allowed root is `IMAGE-PATH`
/// (T-01-27/T-01-38); an unregistered generator is `IMAGE-GENERATOR`.
/// Nothing under `tree` is ever opened for writing.
pub fn stage_full(
    manifest: &Manifest,
    manifest_dir: &Path,
    tree: &Path,
    workspace: &Path,
    stage_dir: &Path,
) -> Result<()> {
    let canonical_tree = tree.canonicalize().map_err(|e| -> crate::DynError {
        format!("image manifest for \"{}\": Harvey tree {}: {e}", manifest.image, tree.display())
            .into()
    })?;
    let canonical_workspace = workspace.canonicalize().map_err(|e| -> crate::DynError {
        format!(
            "image manifest for \"{}\": workspace {}: {e}",
            manifest.image,
            workspace.display()
        )
        .into()
    })?;
    if stage_dir.exists() {
        fs::remove_dir_all(stage_dir)?;
    }
    fs::create_dir_all(stage_dir)?;
    for dir in &manifest.dir {
        fs::create_dir_all(stage_dir.join(image_rel(&dir.path)))?;
    }
    for file in &manifest.file {
        let dest = stage_dir.join(image_rel(&file.path));
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        match file.source {
            Source::Harvey => {
                let from = file.from.as_deref().unwrap_or_default();
                let src = tree.join(from);
                if !src.is_file() {
                    return Err(missing_error(manifest, file, from, &src));
                }
                let canonical_src = src.canonicalize().map_err(|e| -> crate::DynError {
                    format!(
                        "image manifest for \"{}\": source {}: {e}",
                        manifest.image,
                        src.display()
                    )
                    .into()
                })?;
                if !canonical_src.starts_with(&canonical_tree) {
                    return Err(format!(
                        "IMAGE-PATH: image \"{}\" entry {} source {} is a symlink resolving to {} \
                         outside the Harvey tree {} — a manifest source cannot escape the tree",
                        manifest.image,
                        file.path,
                        from,
                        canonical_src.display(),
                        canonical_tree.display()
                    )
                    .into());
                }
                copy_with_mode(
                    &canonical_src,
                    &dest,
                    parse_mode(file.mode.as_deref().unwrap_or("0755"), &manifest.image, &file.path)?,
                )?;
            }
            Source::Rust => {
                let from = file.from.as_deref().unwrap_or_default();
                let src = workspace.join(from);
                if !src.is_file() {
                    return Err(missing_error(manifest, file, from, &src));
                }
                let canonical_src = src.canonicalize().map_err(|e| -> crate::DynError {
                    format!(
                        "image manifest for \"{}\": source {}: {e}",
                        manifest.image,
                        src.display()
                    )
                    .into()
                })?;
                if !canonical_src.starts_with(&canonical_workspace) {
                    return Err(format!(
                        "IMAGE-PATH: image \"{}\" entry {} source {} is a symlink resolving to {} \
                         outside the workspace {} — a rust artifact must live in r9's workspace",
                        manifest.image,
                        file.path,
                        from,
                        canonical_src.display(),
                        canonical_workspace.display()
                    )
                    .into());
                }
                copy_with_mode(
                    &canonical_src,
                    &dest,
                    parse_mode(file.mode.as_deref().unwrap_or("0755"), &manifest.image, &file.path)?,
                )?;
            }
            Source::Generated => {
                return Err(format!(
                    "IMAGE-GENERATOR: image \"{}\" entry {} names generator {:?}, but this plan \
                     registers none (plan 01-16 registers the real generators)",
                    manifest.image,
                    file.path,
                    file.generator.as_deref().unwrap_or_default()
                )
                .into())
            }
            Source::Tree => {
                return Err(format!(
                    "image manifest for \"{}\": entry {} names source = \"tree\"; use a [[tree]] \
                     table",
                    manifest.image, file.path
                )
                .into())
            }
        }
    }
    for tree_entry in &manifest.tree {
        let members_path = manifest_dir.join(&tree_entry.members);
        let text = fs::read_to_string(&members_path).map_err(|e| -> crate::DynError {
            format!(
                "image manifest for \"{}\": [[tree]] {} members file {}: {e}",
                manifest.image,
                tree_entry.path,
                members_path.display()
            )
            .into()
        })?;
        let members = parse_tree_members(&text, &manifest.image, &members_path)?;
        let by_rel: HashMap<&str, &TreeMember> =
            members.iter().map(|member| (member.rel.as_str(), member)).collect();
        let src_root = tree.join(&tree_entry.from);
        if !src_root.is_dir() {
            return Err(format!(
                "IMAGE-DRIFT missing: image \"{}\" [[tree]] {} source {} is not a directory in \
                 the resolved Harvey tree ({})",
                manifest.image,
                tree_entry.path,
                tree_entry.from,
                src_root.display()
            )
            .into());
        }
        let canonical_root = src_root.canonicalize().map_err(|e| -> crate::DynError {
            format!(
                "image manifest for \"{}\": [[tree]] {} source {}: {e}",
                manifest.image,
                tree_entry.path,
                tree_entry.from
            )
            .into()
        })?;
        if !canonical_root.starts_with(&canonical_tree) {
            return Err(format!(
                "IMAGE-PATH: image \"{}\" [[tree]] {} source {} resolves to {} outside the Harvey \
                 tree {}",
                manifest.image,
                tree_entry.path,
                tree_entry.from,
                canonical_root.display(),
                canonical_tree.display()
            )
            .into());
        }
        let image_root = stage_dir.join(image_rel(&tree_entry.path));
        fs::create_dir_all(&image_root)?;
        let mut files = Vec::new();
        walk_files(&canonical_root, &mut files)?;
        for src in files {
            let rel = src
                .strip_prefix(&canonical_root)
                .map_err(|e| -> crate::DynError { format!("tree member {}: {e}", src.display()).into() })?
                .to_string_lossy()
                .into_owned();
            let canonical_src = src.canonicalize().map_err(|e| -> crate::DynError {
                format!("tree member {}: {e}", src.display()).into()
            })?;
            if !canonical_src.starts_with(&canonical_tree) {
                return Err(format!(
                    "IMAGE-PATH: image \"{}\" [[tree]] {} member {} is a symlink resolving to {} \
                     outside the Harvey tree {}",
                    manifest.image,
                    tree_entry.path,
                    rel,
                    canonical_src.display(),
                    canonical_tree.display()
                )
                .into());
            }
            let dest = image_root.join(&rel);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            let mode = match by_rel.get(rel.as_str()) {
                Some(member) => member.mode,
                None => fs::metadata(&canonical_src)?.permissions().mode() & 0o7777,
            };
            copy_with_mode(&canonical_src, &dest, mode)?;
        }
    }
    Ok(())
}

/// The 01-08 entry point (harvey-only manifests): the tree is also the
/// manifest directory and the workspace.  Kept so no caller breaks; the
/// rust/generated/tree kinds stage through [`stage_full`] or
/// [`assemble_checked`].
pub fn stage(manifest: &Manifest, tree: &Path, stage_dir: &Path) -> Result<()> {
    stage_full(manifest, tree, tree, tree, stage_dir)
}

/// The `IMAGE-DRIFT missing` error for a declared source that is absent.
fn missing_error(manifest: &Manifest, file: &FileEntry, from: &str, src: &Path) -> crate::DynError {
    format!(
        "IMAGE-DRIFT missing: image \"{}\" entry {} source {} is absent at {} — the manifest \
         declares a member the image cannot stage",
        manifest.image,
        file.path,
        from,
        src.display()
    )
    .into()
}

/// Copy `src` to `dest` and set `dest`'s permission bits.
fn copy_with_mode(src: &Path, dest: &Path, mode: u32) -> Result<()> {
    fs::copy(src, dest)?;
    fs::set_permissions(dest, fs::Permissions::from_mode(mode))?;
    Ok(())
}

/// The image path without its leading `/` (the staging-tree form).
fn image_rel(path: &str) -> String {
    path.trim_start_matches('/').to_string()
}

/// Join a `[[tree]]` image root and a member's relative path.
fn join_image(base: &str, rel: &str) -> String {
    let base = base.trim_end_matches('/');
    if base.is_empty() { rel.to_string() } else { format!("{base}/{rel}") }
}

/// Every file under `root`, recursively (sorted for deterministic walks).
fn walk_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(root)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let meta = entry.metadata()?;
        if meta.is_dir() {
            walk_files(&path, out)?;
        } else {
            out.push(path);
        }
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

/// What the manifest says one image path must be.
#[derive(Debug, Clone)]
enum Expected {
    Dir,
    File {
        /// The source, for the error text.
        source: String,
        /// For a `[[tree]]` member: the members-file `(size, sha256, mode)`.
        checked: Option<(u64, String, u32)>,
    },
}

impl Expected {
    fn describe(&self) -> String {
        match self {
            Expected::Dir => "directory".to_string(),
            Expected::File { source, .. } => format!("file from {source}"),
        }
    }
}

/// The manifest's declared image paths (files, dirs, tree members and the
/// directories those imply), keyed by staging-tree path.
fn expected_members(
    manifest: &Manifest,
    manifest_dir: &Path,
) -> Result<BTreeMap<String, Expected>> {
    let mut expected: BTreeMap<String, Expected> = BTreeMap::new();
    for file in &manifest.file {
        let source = match file.source {
            Source::Harvey => format!("tree {}", file.from.as_deref().unwrap_or_default()),
            Source::Rust => format!("workspace {}", file.from.as_deref().unwrap_or_default()),
            Source::Generated => {
                format!("generator {}", file.generator.as_deref().unwrap_or_default())
            }
            Source::Tree => "tree".to_string(),
        };
        expected.insert(image_rel(&file.path), Expected::File { source, checked: None });
    }
    for dir in &manifest.dir {
        expected.insert(image_rel(&dir.path), Expected::Dir);
    }
    for tree_entry in &manifest.tree {
        let members_path = manifest_dir.join(&tree_entry.members);
        let text = fs::read_to_string(&members_path).map_err(|e| -> crate::DynError {
            format!(
                "image manifest for \"{}\": [[tree]] {} members file {}: {e}",
                manifest.image,
                tree_entry.path,
                members_path.display()
            )
            .into()
        })?;
        for member in parse_tree_members(&text, &manifest.image, &members_path)? {
            expected.insert(
                join_image(&image_rel(&tree_entry.path), &member.rel),
                Expected::File {
                    source: format!("tree {}/{}", tree_entry.from, member.rel),
                    checked: Some((member.size, member.sha256, member.mode)),
                },
            );
        }
        expected.insert(image_rel(&tree_entry.path), Expected::Dir);
    }
    // Every ancestor of a declared path is a directory in the image (the
    // engine creates it implicitly); the drift check must expect it.
    let paths: Vec<String> = expected.keys().cloned().collect();
    for path in paths {
        let mut parts: Vec<&str> = path.split('/').collect();
        parts.pop();
        let mut prefix = String::new();
        for part in parts {
            if part.is_empty() {
                continue;
            }
            if prefix.is_empty() {
                prefix = part.to_string();
            } else {
                prefix = format!("{prefix}/{part}");
            }
            expected.entry(prefix.clone()).or_insert(Expected::Dir);
        }
    }
    Ok(expected)
}

/// HARN-05's two-way drift check on the staging tree (the exact mkpaqfs
/// input): a staged path no entry declares is `IMAGE-DRIFT unlisted`, a
/// declared path that is not staged is `IMAGE-DRIFT missing`, and a
/// `[[tree]]` member whose size, sha256 or mode disagrees with its members
/// record is `IMAGE-DRIFT changed`.
fn check_drift(
    manifest: &Manifest,
    expected: &BTreeMap<String, Expected>,
    stage_dir: &Path,
) -> Result<()> {
    let actual = member_list(stage_dir)?;
    let actual_by_path: BTreeMap<&str, &Member> =
        actual.iter().map(|member| (member.path.as_str(), member)).collect();
    let mut problems: Vec<String> = Vec::new();
    for member in &actual {
        if !expected.contains_key(member.path.as_str()) {
            problems.push(format!(
                "IMAGE-DRIFT unlisted: image \"{}\" has {} in the staged image but no manifest \
                 entry declares it",
                manifest.image,
                member.path
            ));
        }
    }
    for (path, want) in expected {
        match actual_by_path.get(path.as_str()) {
            None => problems.push(format!(
                "IMAGE-DRIFT missing: image \"{}\" path {} ({}) is declared but not staged",
                manifest.image,
                path,
                want.describe()
            )),
            Some(got) => match want {
                Expected::Dir => {
                    if !got.is_dir {
                        problems.push(format!(
                            "IMAGE-DRIFT changed: image \"{}\" path {} is declared a directory \
                             but staged as a file",
                            manifest.image, path
                        ));
                    }
                }
                Expected::File { source, checked: None } => {
                    if got.is_dir {
                        problems.push(format!(
                            "IMAGE-DRIFT changed: image \"{}\" path {} ({source}) is declared a \
                             file but staged as a directory",
                            manifest.image, path
                        ));
                    }
                }
                Expected::File { source, checked: Some((size, sha256, mode)) } => {
                    if got.is_dir {
                        problems.push(format!(
                            "IMAGE-DRIFT changed: image \"{}\" path {} ({source}) is declared a \
                             file but staged as a directory",
                            manifest.image, path
                        ));
                        continue;
                    }
                    let got_size = got.size.unwrap_or(0);
                    let got_sha = got.sha256.as_deref().unwrap_or("-");
                    if got_size != *size || got_sha != sha256 || got.mode != *mode {
                        problems.push(format!(
                            "IMAGE-DRIFT changed: image \"{}\" path {} ({source}) — the members \
                             file records size {} sha256 {} mode {:o}; the staged file has size \
                             {} sha256 {} mode {:o}",
                            manifest.image,
                            path,
                            size,
                            sha256,
                            mode,
                            got_size,
                            got_sha,
                            got.mode
                        ));
                    }
                }
            },
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        problems.push(
            "manifest and image cannot drift in either direction (HARN-05, D-15); fix the \
             manifest or the tree, never the check"
                .to_string(),
        );
        Err(problems.join("\n").into())
    }
}

/// HARN-05's Rust provenance rule: a `rust` entry's artifact must resolve
/// inside the workspace, never under a Harvey tree root (after symlink
/// resolution), and its bytes must differ from the Harvey artifact it
/// replaces.  A `replaces` file absent from the tree is itself `IMAGE-PATH`.
fn check_provenance(
    manifest: &Manifest,
    tree: &Path,
    harvey_roots: &[PathBuf],
    workspace: &Path,
) -> Result<()> {
    let canonical_workspace = workspace.canonicalize().map_err(|e| -> crate::DynError {
        format!(
            "image manifest for \"{}\": workspace {}: {e}",
            manifest.image,
            workspace.display()
        )
        .into()
    })?;
    let roots: Vec<PathBuf> =
        harvey_roots.iter().filter_map(|root| root.canonicalize().ok()).collect();
    for file in manifest.file.iter().filter(|file| file.source == Source::Rust) {
        let from = file.from.as_deref().unwrap_or_default();
        let replaces = file.replaces.as_deref().unwrap_or_default();
        let src = workspace.join(from);
        let canonical_src = src.canonicalize().map_err(|e| -> crate::DynError {
            format!(
                "IMAGE-PATH: image \"{}\" entry {} source {}: {e}",
                manifest.image,
                file.path,
                src.display()
            )
            .into()
        })?;
        if !canonical_src.starts_with(&canonical_workspace) {
            return Err(format!(
                "IMAGE-PATH: image \"{}\" entry {} source {} resolves to {} outside the workspace \
                 {} — a rust artifact must come from r9's own build",
                manifest.image,
                file.path,
                from,
                canonical_src.display(),
                canonical_workspace.display()
            )
            .into());
        }
        for root in &roots {
            if canonical_src.starts_with(root) {
                return Err(format!(
                    "IMAGE-PROVENANCE: image \"{}\" entry {} `from` = {} resolves into the Harvey \
                     tree {} — an entry marked rust must come from r9's Rust build, never a \
                     Harvey tree (HARN-05)",
                    manifest.image,
                    file.path,
                    from,
                    root.display()
                )
                .into());
            }
        }
        let replaced = tree.join(replaces);
        if !replaced.is_file() {
            return Err(format!(
                "IMAGE-PATH: image \"{}\" entry {} `replaces` = {} is not a file in the Harvey \
                 tree ({}) — the rust entry must name the artifact it stands in for (HARN-05)",
                manifest.image,
                file.path,
                replaces,
                replaced.display()
            )
            .into());
        }
        let rust_sha = sha256_file(&canonical_src)?;
        let replaced_sha = sha256_file(&replaced)?;
        if rust_sha == replaced_sha {
            return Err(format!(
                "IMAGE-PROVENANCE: image \"{}\" entry {} from {} is identical to the Harvey \
                 artifact {} it replaces — the swap is never a relabelled Harvey binary (HARN-05)",
                manifest.image,
                file.path,
                from,
                replaces
            )
            .into());
        }
    }
    Ok(())
}

/// Assemble one image through the full enforcement chain: parse → stage →
/// generators (registry; none in this plan) → two-way drift → Rust
/// provenance → bake → member list.
///
/// `harvey_roots` names every Harvey tree root in play for the provenance
/// prefix check; `out_dir` receives `<image>.stage/`, `<image>.paq` and
/// `<image>.members`.  The tree is never written to (HARN-02).
pub fn assemble_checked(
    manifest_path: &Path,
    tree: &Path,
    harvey_roots: &[PathBuf],
    workspace: &Path,
    out_dir: &Path,
    mkpaqfs: &Path,
) -> Result<Assembled> {
    let text = fs::read_to_string(manifest_path).map_err(|e| -> crate::DynError {
        format!("image manifest {}: {e}", manifest_path.display()).into()
    })?;
    let manifest = parse_manifest(&text)?;
    let manifest_dir = manifest_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let stage_dir = out_dir.join(format!("{}.stage", manifest.image));
    stage_full(&manifest, manifest_dir, tree, workspace, &stage_dir)?;
    // Generators: the registry is empty in this plan; `validate` already
    // refused every generated entry, so this step is the registry's seat.
    let expected = expected_members(&manifest, manifest_dir)?;
    check_drift(&manifest, &expected, &stage_dir)?;
    check_provenance(&manifest, tree, harvey_roots, workspace)?;
    fs::create_dir_all(out_dir)?;
    let paq = out_dir.join(format!("{}.paq", manifest.image));
    bake(mkpaqfs, &stage_dir, &paq)?;
    let members = member_list(&stage_dir)?;
    let members_path = out_dir.join(format!("{}.members", manifest.image));
    write_members(&members, &members_path)?;
    let sha256 = sha256_file(&paq)?;
    Ok(Assembled {
        arch: manifest.arch,
        image: manifest.image,
        stage_dir,
        paq,
        members_path,
        members,
        sha256,
    })
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

/// The paths a manifest's bake depends on (the staleness check reads their
/// mtimes): every `harvey` source, every `rust` artifact, every `[[tree]]`
/// source file, and the `[[tree]]` members files.
pub fn input_paths(
    manifest: &Manifest,
    manifest_dir: &Path,
    tree: &Path,
    workspace: &Path,
) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    for file in &manifest.file {
        match file.source {
            Source::Harvey => {
                if let Some(from) = &file.from {
                    inputs.push(tree.join(from));
                }
            }
            Source::Rust => {
                if let Some(from) = &file.from {
                    inputs.push(workspace.join(from));
                }
            }
            Source::Generated | Source::Tree => {}
        }
    }
    for tree_entry in &manifest.tree {
        inputs.push(manifest_dir.join(&tree_entry.members));
        let mut files = Vec::new();
        if walk_files(&tree.join(&tree_entry.from), &mut files).is_ok() {
            inputs.extend(files);
        }
    }
    inputs
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

/// Parse a reference bake's member list (the D-29 comparison input).  The
/// same `<path> file <mode> <size> <sha256>` / `<path> dir <mode> - -`
/// shape the engine writes; a leading `/` is normalised away, and a
/// `sha256` field of `opaque` (auth material) is kept verbatim so
/// [`compare_members`] compares it by path, size and mode only.
pub fn parse_reference_members(text: &str) -> Result<Vec<Member>> {
    let mut members = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 5 {
            return Err(format!(
                "reference member list line {} is not `<path> file|dir <mode> <size> <sha256>`: \
                 {line:?}",
                index + 1
            )
            .into());
        }
        let path = fields[0].trim_start_matches('/').trim_end_matches('/').to_string();
        let mode = u32::from_str_radix(fields[2], 8).map_err(|_| -> crate::DynError {
            format!("reference member list line {}: mode {:?} is not octal", index + 1, fields[2])
                .into()
        })?;
        match fields[1] {
            "dir" => members.push(Member { path, is_dir: true, mode, size: None, sha256: None }),
            "file" => {
                let size = fields[3].parse::<u64>().map_err(|_| -> crate::DynError {
                    format!(
                        "reference member list line {}: size {:?} is not a byte count",
                        index + 1,
                        fields[3]
                    )
                    .into()
                })?;
                members.push(Member {
                    path,
                    is_dir: false,
                    mode,
                    size: Some(size),
                    sha256: Some(fields[4].to_string()),
                })
            }
            other => {
                return Err(format!(
                    "reference member list line {}: kind {other:?} is not one of \"file\", \
                     \"dir\"",
                    index + 1
                )
                .into())
            }
        }
    }
    Ok(members)
}

/// D-29's member-list comparison: assembled versus the reference bake.
///
/// Passes on identical lists; fails naming each added, removed or changed
/// path (member lists, never byte differentials — assumption A12 was
/// refuted).  A reference entry whose sha256 field is `opaque` is compared
/// by path, size and mode only.
pub fn compare_members(assembled: &Assembled, reference: &[Member]) -> Result<()> {
    let reference_by_path: BTreeMap<&str, &Member> =
        reference.iter().map(|member| (member.path.as_str(), member)).collect();
    let assembled_by_path: BTreeMap<&str, &Member> =
        assembled.members.iter().map(|member| (member.path.as_str(), member)).collect();
    let mut differences: Vec<String> = Vec::new();
    for member in &assembled.members {
        match reference_by_path.get(member.path.as_str()) {
            None => differences.push(format!("  added: {}", member.line())),
            Some(want) => {
                if member.is_dir != want.is_dir {
                    differences.push(format!(
                        "  changed: {} (assembled {}, reference {})",
                        member.path,
                        if member.is_dir { "directory" } else { "file" },
                        if want.is_dir { "directory" } else { "file" }
                    ));
                } else if member.mode != want.mode {
                    differences.push(format!(
                        "  changed: {} (mode {:o} != reference {:o})",
                        member.path, member.mode, want.mode
                    ));
                } else if !member.is_dir {
                    if member.size != want.size {
                        differences.push(format!(
                            "  changed: {} (size {:?} != reference {:?})",
                            member.path, member.size, want.size
                        ));
                    }
                    let reference_sha = want.sha256.as_deref().unwrap_or("-");
                    if reference_sha != "opaque" && member.sha256.as_deref() != Some(reference_sha)
                    {
                        differences.push(format!(
                            "  changed: {} (sha256 {:?} != reference {reference_sha})",
                            member.path, member.sha256
                        ));
                    }
                }
            }
        }
    }
    for want in reference {
        if !assembled_by_path.contains_key(want.path.as_str()) {
            differences.push(format!("  removed: {}", want.line()));
        }
    }
    if differences.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "image {}/{} differs from the reference member list (D-29):\n{}",
            assembled.arch,
            assembled.image,
            differences.join("\n")
        )
        .into())
    }
}

/// Emit a manifest from a reference bake's member list (D-29 / HARN-03's
/// tooling): map each reference file to the tree file with the same
/// sha256, preferring the identical relative path, then an identical
/// basename, then the lexically first; map reference dirs to `[[dir]]`;
/// write a `generator = "UNRESOLVED"` entry, with a comment saying why, for
/// every file no tree file matches.  `parse_manifest` rejects UNRESOLVED,
/// so an emitted manifest can never silently omit a file.
pub fn emit_manifest(
    reference: &[Member],
    tree: &Path,
    arch: &str,
    image: &str,
) -> Result<Emitted> {
    let index = tree_sha_index(tree)?;
    let mut text = String::new();
    text.push_str(
        "# Derived from a reference bake's member list by `cargo xtask image --emit-manifest`.\n\
         # Every file maps to the tree file with the same sha256 (identical relative path\n\
         # first, then identical basename, then the lexically first); UNRESOLVED entries fail\n\
         # parse_manifest, so a manifest can never silently omit a file.\n\n",
    );
    text.push_str(&format!("schema = 1\narch = \"{arch}\"\nimage = \"{image}\"\n"));
    let mut unresolved = Vec::new();
    for member in reference {
        if member.is_dir {
            text.push_str(&format!("\n[[dir]]\npath = \"/{}\"\n", member.path));
            continue;
        }
        let sha = member.sha256.as_deref().unwrap_or("");
        if sha.is_empty() || sha == "opaque" {
            unresolved.push(member.path.clone());
            text.push_str(&format!(
                "\n# UNRESOLVED: reference {} has no content sha256 ({sha:?}) to match against the \
                 tree\n[[file]]\npath = \"/{}\"\nsource = \"generated\"\ngenerator = \
                 \"UNRESOLVED\"\nmode = \"0{:o}\"\n",
                member.path, member.path, member.mode
            ));
            continue;
        }
        match pick_tree_candidate(index.get(sha), &member.path) {
            Some(candidate) => text.push_str(&format!(
                "\n[[file]]\npath = \"/{}\"\nsource = \"harvey\"\nfrom = \"{}\"\nmode = \"0{:o}\"\n",
                member.path, candidate, member.mode
            )),
            None => {
                unresolved.push(member.path.clone());
                text.push_str(&format!(
                    "\n# UNRESOLVED: no tree file matches sha256 {sha} (reference {}, size {})\n\
                     [[file]]\npath = \"/{}\"\nsource = \"generated\"\ngenerator = \
                     \"UNRESOLVED\"\nmode = \"0{:o}\"\n",
                    member.path,
                    member.size.unwrap_or(0),
                    member.path,
                    member.mode
                ));
            }
        }
    }
    Ok(Emitted { text, unresolved })
}

/// Prefer the identical relative path, then an identical basename, then the
/// lexically first candidate (the index's vectors are sorted).
fn pick_tree_candidate(candidates: Option<&Vec<String>>, reference_path: &str) -> Option<String> {
    let candidates = candidates?;
    if let Some(candidate) = candidates.iter().find(|candidate| *candidate == reference_path) {
        return Some(candidate.clone());
    }
    let basename = reference_path.rsplit('/').next().unwrap_or(reference_path);
    if let Some(candidate) =
        candidates.iter().find(|candidate| candidate.rsplit('/').next() == Some(basename))
    {
        return Some(candidate.clone());
    }
    candidates.first().cloned()
}

/// Every tree file indexed by sha256, tree-relative paths sorted.
fn tree_sha_index(tree: &Path) -> Result<HashMap<String, Vec<String>>> {
    let mut files = Vec::new();
    walk_files(tree, &mut files)?;
    let mut index: HashMap<String, Vec<String>> = HashMap::new();
    for file in files {
        let rel = file
            .strip_prefix(tree)
            .map_err(|e| -> crate::DynError { format!("tree file {}: {e}", file.display()).into() })?
            .to_string_lossy()
            .into_owned();
        let sha = sha256_file(&file)?;
        index.entry(sha).or_default().push(rel);
    }
    for candidates in index.values_mut() {
        candidates.sort();
    }
    Ok(index)
}

/// The tier-0 library list parser (the pinned data recipe: skip `#` lines
/// and blanks, take the first whitespace field), as spikes/s4/tier.nix does.
pub fn parse_tier_list(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            trimmed.split_whitespace().next().map(str::to_string)
        })
        .collect()
}

/// HARN-03's library table check: every name in the pinned tier-0 list must
/// have a `[[library]]` entry (`in_image = false`) whose `from` file exists
/// in the tree.  Reports every gap, not just the first.
pub fn check_libraries(manifest: &Manifest, tree: &Path, tier0_list_text: &str) -> Result<()> {
    let mut problems: Vec<String> = Vec::new();
    for name in parse_tier_list(tier0_list_text) {
        match manifest.library.iter().find(|library| library.name == name) {
            None => problems.push(format!("{name}: no [[library]] entry in image \"{}\"", manifest.image)),
            Some(library) => {
                if library.in_image {
                    problems.push(format!("{name}: in_image = true (libraries are not image members)"));
                }
                let path = tree.join(&library.from);
                if !path.is_file() {
                    problems.push(format!("{name}: {} is missing from the tree", path.display()));
                }
            }
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "libraries: image \"{}\" does not satisfy the tier-0 list:\n  {}",
            manifest.image,
            problems.join("\n  ")
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

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

    /// Build a tree of files under `scratch/tree`.
    fn tree_with(scratch: &Scratch, files: &[(&str, &[u8])]) -> PathBuf {
        let tree = scratch.0.join("tree");
        for (rel, bytes) in files {
            let path = tree.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        tree
    }

    fn sample_tree(scratch: &Scratch) -> PathBuf {
        tree_with(
            scratch,
            &[
                ("build/aarch64/smoke-out/init", b"init-bytes"),
                ("build/aarch64/smoke-out/hello", b"hello"),
            ],
        )
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

    /// A rust manifest: the artifact is workspace-relative and names the
    /// Harvey artifact it replaces.
    fn rust_manifest() -> String {
        "\
schema = 1
arch = \"amd64\"
image = \"swap\"

[[file]]
path = \"/amd64/bin/ps\"
source = \"rust\"
from = \"probe/ps.aout\"
replaces = \"amd64/bin/ps\"
mode = \"0775\"
"
        .to_string()
    }

    #[test]
    fn rust_manifest_requires_from_replaces_and_a_valid_mode() {
        let manifest = parse_manifest(&rust_manifest()).unwrap();
        assert_eq!(manifest.file[0].source, Source::Rust);
        assert_eq!(manifest.file[0].replaces.as_deref(), Some("amd64/bin/ps"));

        let text = rust_manifest().replace("replaces = \"amd64/bin/ps\"\n", "");
        let err = parse_manifest(&text).unwrap_err().to_string();
        assert!(err.contains("requires `replaces`"), "{err}");

        let text = rust_manifest().replace("from = \"probe/ps.aout\"\n", "");
        let err = parse_manifest(&text).unwrap_err().to_string();
        assert!(err.contains("requires `from`"), "{err}");
    }

    #[test]
    fn stage_copies_entries_with_manifest_mode_and_creates_dirs() {
        let scratch = Scratch::new("stage");
        let tree = sample_tree(&scratch);

        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage_full(&manifest, &tree, &tree, &tree, &stage_dir).unwrap();

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
        let tree = sample_tree(&scratch);
        let stage_dir = scratch.0.join("stage");
        fs::create_dir_all(stage_dir.join("leftover")).unwrap();
        fs::write(stage_dir.join("leftover/stale"), b"stale").unwrap();

        let manifest = parse_manifest(&sample_manifest()).unwrap();
        stage_full(&manifest, &tree, &tree, &tree, &stage_dir).unwrap();
        assert!(!stage_dir.join("leftover").exists());
    }

    #[test]
    fn stage_names_the_missing_source_and_the_manifest() {
        let scratch = Scratch::new("missing");
        let tree = tree_with(&scratch, &[("build/aarch64/smoke-out/init", b"init")]);
        // hello is deliberately absent.

        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let err = stage_full(&manifest, &tree, &tree, &tree, &scratch.0.join("stage"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("IMAGE-DRIFT missing"), "{err}");
        assert!(err.contains("smoke"), "{err}");
        assert!(err.contains("build/aarch64/smoke-out/hello"), "{err}");
        assert!(err.contains("/hello"), "{err}");
    }

    #[test]
    fn member_list_is_sorted_and_records_size_sha_mode_and_dirs() {
        let scratch = Scratch::new("members");
        let tree = sample_tree(&scratch);

        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage_full(&manifest, &tree, &tree, &tree, &stage_dir).unwrap();
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
        let inputs = input_paths(&manifest, Path::new("/ws/xtask/manifests/arm64"), tree, Path::new("/ws"));
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0], Path::new("/tree/build/aarch64/smoke-out/init"));
    }

    /// Stage the sample manifest, then add a file no entry declares: the
    /// drift check catches the image-side half of HARN-05.
    #[test]
    fn drift_reports_an_unlisted_staged_file() {
        let scratch = Scratch::new("drift-unlisted");
        let tree = sample_tree(&scratch);
        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage_full(&manifest, &tree, &tree, &tree, &stage_dir).unwrap();
        fs::write(stage_dir.join("stray"), b"stray").unwrap();
        let expected = expected_members(&manifest, &tree).unwrap();
        let err = check_drift(&manifest, &expected, &stage_dir).unwrap_err().to_string();
        assert!(err.contains("IMAGE-DRIFT unlisted"), "{err}");
        assert!(err.contains("stray"), "{err}");
        assert!(err.contains("smoke"), "{err}");
    }

    /// A declared entry that is not staged: the manifest-side half.
    #[test]
    fn drift_reports_a_missing_entry() {
        let scratch = Scratch::new("drift-missing");
        let tree = sample_tree(&scratch);
        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage_full(&manifest, &tree, &tree, &tree, &stage_dir).unwrap();
        fs::remove_file(stage_dir.join("hello")).unwrap();
        let expected = expected_members(&manifest, &tree).unwrap();
        let err = check_drift(&manifest, &expected, &stage_dir).unwrap_err().to_string();
        assert!(err.contains("IMAGE-DRIFT missing"), "{err}");
        assert!(err.contains("hello"), "{err}");
        assert!(err.contains("smoke"), "{err}");
    }

    /// A `[[tree]]` entry with a members file: every file under the source
    /// dir lands in the image, and a source file absent from the members
    /// list is unlisted.
    #[test]
    fn tree_entry_expands_every_file_and_flags_an_unlisted_member() {
        let scratch = Scratch::new("tree-unlisted");
        let tree = tree_with(
            &scratch,
            &[("amd64/lib/ndb/local", b"local"), ("amd64/lib/ndb/extra", b"extra")],
        );
        let manifest_text = "\
schema = 1
arch = \"amd64\"
image = \"t\"

[[tree]]
path = \"/lib/ndb\"
from = \"amd64/lib/ndb\"
members = \"t.members\"
";
        let manifest = parse_manifest(manifest_text).unwrap();
        let sha = sha256_file(&tree.join("amd64/lib/ndb/local")).unwrap();
        fs::write(scratch.0.join("t.members"), format!("f local 5 {sha} 644\n")).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage_full(&manifest, &scratch.0, &tree, &tree, &stage_dir).unwrap();
        assert_eq!(fs::read(stage_dir.join("lib/ndb/local")).unwrap(), b"local");
        assert_eq!(fs::read(stage_dir.join("lib/ndb/extra")).unwrap(), b"extra");
        assert_eq!(
            fs::metadata(stage_dir.join("lib/ndb/local")).unwrap().permissions().mode() & 0o777,
            0o644
        );
        let expected = expected_members(&manifest, &scratch.0).unwrap();
        let err = check_drift(&manifest, &expected, &stage_dir).unwrap_err().to_string();
        assert!(err.contains("IMAGE-DRIFT unlisted"), "{err}");
        assert!(err.contains("lib/ndb/extra"), "{err}");
    }

    /// A members line whose size disagrees with the source file is
    /// `IMAGE-DRIFT changed`.
    #[test]
    fn tree_entry_flags_a_changed_member() {
        let scratch = Scratch::new("tree-changed");
        let tree = tree_with(&scratch, &[("amd64/lib/ndb/local", b"local")]);
        let manifest_text = "\
schema = 1
arch = \"amd64\"
image = \"t\"

[[tree]]
path = \"/lib/ndb\"
from = \"amd64/lib/ndb\"
members = \"t.members\"
";
        let manifest = parse_manifest(manifest_text).unwrap();
        let sha = sha256_file(&tree.join("amd64/lib/ndb/local")).unwrap();
        fs::write(scratch.0.join("t.members"), format!("f local 99 {sha} 644\n")).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage_full(&manifest, &scratch.0, &tree, &tree, &stage_dir).unwrap();
        let expected = expected_members(&manifest, &scratch.0).unwrap();
        let err = check_drift(&manifest, &expected, &stage_dir).unwrap_err().to_string();
        assert!(err.contains("IMAGE-DRIFT changed"), "{err}");
        assert!(err.contains("lib/ndb/local"), "{err}");
        assert!(err.contains("size 99"), "{err}");
    }

    /// A members line naming a file the source dir does not carry is
    /// `IMAGE-DRIFT missing`.
    #[test]
    fn tree_entry_flags_a_listed_file_absent_from_the_source() {
        let scratch = Scratch::new("tree-listed-missing");
        let tree = tree_with(&scratch, &[("amd64/lib/ndb/local", b"local")]);
        let manifest_text = "\
schema = 1
arch = \"amd64\"
image = \"t\"

[[tree]]
path = \"/lib/ndb\"
from = \"amd64/lib/ndb\"
members = \"t.members\"
";
        let manifest = parse_manifest(manifest_text).unwrap();
        let sha = sha256_file(&tree.join("amd64/lib/ndb/local")).unwrap();
        fs::write(
            scratch.0.join("t.members"),
            format!("f local 5 {sha} 644\nf gone 1 {sha} 644\n"),
        )
        .unwrap();
        let stage_dir = scratch.0.join("stage");
        stage_full(&manifest, &scratch.0, &tree, &tree, &stage_dir).unwrap();
        let expected = expected_members(&manifest, &scratch.0).unwrap();
        let err = check_drift(&manifest, &expected, &stage_dir).unwrap_err().to_string();
        assert!(err.contains("IMAGE-DRIFT missing"), "{err}");
        assert!(err.contains("lib/ndb/gone"), "{err}");
    }

    /// HARN-05's provenance rule, first half: a rust artifact that resolves
    /// under a Harvey tree root is refused even when the path looks local.
    #[test]
    fn rust_provenance_rejects_a_source_under_a_harvey_root() {
        let scratch = Scratch::new("provenance-tree");
        let workspace = scratch.0.join("ws");
        let tree = scratch.0.join("tree");
        fs::create_dir_all(workspace.join("probe")).unwrap();
        fs::write(workspace.join("probe/ps.aout"), b"rust-bytes").unwrap();
        fs::create_dir_all(tree.join("amd64/bin")).unwrap();
        fs::write(tree.join("amd64/bin/ps"), b"harvey-bytes").unwrap();
        let manifest = parse_manifest(&rust_manifest()).unwrap();
        // The artifact's own directory is a Harvey tree root: canonicalized,
        // it resolves into the tree and must be refused.
        let roots = vec![workspace.join("probe")];
        let err = check_provenance(&manifest, &tree, &roots, &workspace).unwrap_err().to_string();
        assert!(err.contains("IMAGE-PROVENANCE"), "{err}");
        assert!(err.contains("resolves into the Harvey tree"), "{err}");
        assert!(err.contains("/amd64/bin/ps"), "{err}");
    }

    /// HARN-05's provenance rule, second half: byte-identical to the
    /// Harvey artifact it replaces is refused.
    #[test]
    fn rust_provenance_rejects_bytes_identical_to_the_replaced_artifact() {
        let scratch = Scratch::new("provenance-identical");
        let workspace = scratch.0.join("ws");
        let tree = scratch.0.join("tree");
        fs::create_dir_all(workspace.join("probe")).unwrap();
        fs::write(workspace.join("probe/ps.aout"), b"same-bytes").unwrap();
        fs::create_dir_all(tree.join("amd64/bin")).unwrap();
        fs::write(tree.join("amd64/bin/ps"), b"same-bytes").unwrap();
        let manifest = parse_manifest(&rust_manifest()).unwrap();
        let err = check_provenance(&manifest, &tree, &[], &workspace).unwrap_err().to_string();
        assert!(err.contains("IMAGE-PROVENANCE"), "{err}");
        assert!(err.contains("identical to the Harvey artifact"), "{err}");
    }

    /// A genuine Rust artifact (different bytes, outside the Harvey roots)
    /// passes the provenance check.
    #[test]
    fn rust_provenance_accepts_a_distinct_workspace_artifact() {
        let scratch = Scratch::new("provenance-ok");
        let workspace = scratch.0.join("ws");
        let tree = scratch.0.join("tree");
        fs::create_dir_all(workspace.join("probe")).unwrap();
        fs::write(workspace.join("probe/ps.aout"), b"r9-bytes").unwrap();
        fs::create_dir_all(tree.join("amd64/bin")).unwrap();
        fs::write(tree.join("amd64/bin/ps"), b"harvey-bytes").unwrap();
        let manifest = parse_manifest(&rust_manifest()).unwrap();
        check_provenance(&manifest, &tree, std::slice::from_ref(&tree), &workspace).unwrap();
    }

    /// T-01-38: a Harvey source symlink resolving outside the tree is
    /// `IMAGE-PATH`, not a copy of a host file.
    #[test]
    fn stage_rejects_a_harvey_source_symlink_escaping_the_tree() {
        let scratch = Scratch::new("symlink-escape");
        let tree = scratch.0.join("tree");
        fs::create_dir_all(tree.join("build/aarch64/smoke-out")).unwrap();
        fs::write(scratch.0.join("outside"), b"secret").unwrap();
        symlink(scratch.0.join("outside"), tree.join("build/aarch64/smoke-out/init")).unwrap();
        fs::write(tree.join("build/aarch64/smoke-out/hello"), b"hello").unwrap();
        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let err = stage_full(&manifest, &tree, &tree, &tree, &scratch.0.join("stage"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("IMAGE-PATH"), "{err}");
        assert!(err.contains("outside the Harvey tree"), "{err}");
        assert!(err.contains("/init"), "{err}");
    }

    /// An unknown generator and the reserved UNRESOLVED marker are both
    /// `IMAGE-GENERATOR`; the emitter's marker can never reach assembly.
    #[test]
    fn generated_entries_require_a_registered_generator() {
        let template = "\
schema = 1
arch = \"amd64\"
image = \"t\"

[[file]]
path = \"/etc/generated\"
source = \"generated\"
generator = \"BOGUS\"
";
        let err = parse_manifest(template).unwrap_err().to_string();
        assert!(err.contains("IMAGE-GENERATOR"), "{err}");
        assert!(err.contains("BOGUS"), "{err}");

        let err = parse_manifest(&template.replace("BOGUS", "UNRESOLVED")).unwrap_err().to_string();
        assert!(err.contains("IMAGE-GENERATOR"), "{err}");
        assert!(err.contains("UNRESOLVED"), "{err}");
    }

    /// HARN-03: `[[library]]` rows are never staged, and
    /// `check_libraries` names every tier-0 gap.
    #[test]
    fn library_entries_are_never_staged_and_check_libraries_reports_gaps() {
        let scratch = Scratch::new("libraries");
        let tree = tree_with(&scratch, &[("amd64/lib/libc.a", b"archive"), ("amd64/lib/libbio.a", b"bio")]);
        let manifest_text = "\
schema = 1
arch = \"amd64\"
image = \"t\"

[[file]]
path = \"/init\"
source = \"harvey\"
from = \"amd64/lib/libc.a\"
mode = \"0644\"

[[library]]
name = \"libc\"
from = \"amd64/lib/libc.a\"
in_image = false
";
        let manifest = parse_manifest(manifest_text).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage_full(&manifest, &tree, &tree, &tree, &stage_dir).unwrap();
        assert!(stage_dir.join("init").is_file());
        assert!(!stage_dir.join("amd64").exists(), "libraries are not image members");

        let list = "# tier0\n\nlibc\t\t# everything links it\nlibbio\n";
        let err = check_libraries(&manifest, &tree, list).unwrap_err().to_string();
        assert!(err.contains("libbio"), "{err}");
        assert!(err.contains("no [[library]] entry"), "{err}");

        let full = format!(
            "{manifest_text}\n[[library]]\nname = \"libbio\"\nfrom = \"amd64/lib/libbio.a\"\n\
             in_image = false\n"
        );
        let manifest = parse_manifest(&full).unwrap();
        check_libraries(&manifest, &tree, list).unwrap();
    }

    #[test]
    fn library_in_image_true_is_rejected_and_members_paths_are_checked() {
        let text = "\
schema = 1
arch = \"amd64\"
image = \"t\"

[[library]]
name = \"libc\"
from = \"amd64/lib/libc.a\"
in_image = true
";
        let err = parse_manifest(text).unwrap_err().to_string();
        assert!(err.contains("in_image = true"), "{err}");

        let text = "\
schema = 1
arch = \"amd64\"
image = \"t\"

[[tree]]
path = \"/lib\"
from = \"amd64/lib\"
members = \"../t.members\"
";
        let err = parse_manifest(text).unwrap_err().to_string();
        assert!(err.contains("members"), "{err}");
        assert!(err.contains("tree-relative"), "{err}");
    }

    /// The D-29 comparison passes on identical lists and names every
    /// added, removed and changed path.
    #[test]
    fn compare_members_passes_on_identical_lists_and_names_each_difference() {
        let scratch = Scratch::new("compare");
        let tree = sample_tree(&scratch);
        let manifest = parse_manifest(&sample_manifest()).unwrap();
        let stage_dir = scratch.0.join("stage");
        stage_full(&manifest, &tree, &tree, &tree, &stage_dir).unwrap();
        let members = member_list(&stage_dir).unwrap();
        let assembled = Assembled {
            arch: "arm64".to_string(),
            image: "smoke".to_string(),
            stage_dir: stage_dir.clone(),
            paq: scratch.0.join("smoke.paq"),
            members_path: scratch.0.join("smoke.members"),
            members: members.clone(),
            sha256: "digest".to_string(),
        };
        compare_members(&assembled, &members).unwrap();

        let mut reference = members.clone();
        reference[0].size = Some(999); // hello's size
        reference[0].sha256 = Some("opaque".to_string()); // …but opaque ignores sha
        let err = compare_members(&assembled, &reference).unwrap_err().to_string();
        assert!(err.contains("changed: hello"), "{err}");
        assert!(err.contains("size"), "{err}");

        let mut reference = members.clone();
        reference[0].sha256 = Some("different".to_string());
        let err = compare_members(&assembled, &reference).unwrap_err().to_string();
        assert!(err.contains("changed: hello"), "{err}");
        assert!(err.contains("sha256"), "{err}");

        let mut reference = members.clone();
        reference.pop(); // drop proc
        reference.push(Member {
            path: "extra".to_string(),
            is_dir: false,
            mode: 0o644,
            size: Some(1),
            sha256: Some("abc".to_string()),
        });
        let err = compare_members(&assembled, &reference).unwrap_err().to_string();
        assert!(err.contains("added: proc dir"), "{err}");
        assert!(err.contains("removed: extra file"), "{err}");
    }

    /// An opaque reference hash compares path, size and mode only.
    #[test]
    fn compare_members_treats_opaque_reference_hashes_by_path_size_and_mode() {
        let assembled = Assembled {
            arch: "arm64".to_string(),
            image: "auth".to_string(),
            stage_dir: PathBuf::from("/stage"),
            paq: PathBuf::from("/stage/auth.paq"),
            members_path: PathBuf::from("/stage/auth.members"),
            members: vec![Member {
                path: "adm/auth".to_string(),
                is_dir: false,
                mode: 0o600,
                size: Some(32),
                sha256: Some("realhash".to_string()),
            }],
            sha256: "digest".to_string(),
        };
        let reference = parse_reference_members("adm/auth file 600 32 opaque\n").unwrap();
        compare_members(&assembled, &reference).unwrap();
    }

    /// The emitter maps by sha256, preferring the identical relative path,
    /// then an identical basename, then the lexically first.
    #[test]
    fn emit_manifest_prefers_relative_path_then_basename_then_lexical() {
        let scratch = Scratch::new("emit-prefer");
        let tree = tree_with(
            &scratch,
            &[
                ("amd64/bin/ps", b"ps-bytes"),
                ("build/tools/ps", b"ps-bytes"),
                ("amd64/bin/cat", b"cat-bytes"),
                ("build/a/renamed", b"cat-bytes"),
                ("build/b/zzz", b"zzz-bytes"),
            ],
        );
        let sha_ps = sha256_file(&tree.join("amd64/bin/ps")).unwrap();
        let sha_cat = sha256_file(&tree.join("amd64/bin/cat")).unwrap();
        let sha_zzz = sha256_file(&tree.join("build/b/zzz")).unwrap();
        let reference = parse_reference_members(&format!(
            "amd64/bin/ps file 775 8 {sha_ps}\n\
             amd64/bin/cat file 775 9 {sha_cat}\n\
             /amd64/bin/zzz file 775 9 {sha_zzz}\n"
        ))
        .unwrap();
        let emitted = emit_manifest(&reference, &tree, "amd64", "t").unwrap();
        assert!(emitted.unresolved.is_empty(), "{:?}", emitted.unresolved);
        assert!(emitted.text.contains("from = \"amd64/bin/ps\""), "{}", emitted.text);
        // cat's identical path wins over the renamed copy.
        assert!(emitted.text.contains("from = \"amd64/bin/cat\""), "{}", emitted.text);
        // zzz has no identical path; its basename matches build/b/zzz.
        assert!(emitted.text.contains("from = \"build/b/zzz\""), "{}", emitted.text);
        // The emitted manifest parses.
        parse_manifest(&emitted.text).unwrap();
    }

    /// A reference file with no matching tree bytes becomes an UNRESOLVED
    /// generated entry with a why-comment, and parse_manifest rejects it.
    #[test]
    fn emit_manifest_marks_unmatched_files_unresolved_and_rejection_is_named() {
        let scratch = Scratch::new("emit-unresolved");
        let tree = tree_with(&scratch, &[("amd64/bin/ps", b"ps-bytes")]);
        let sha_ps = sha256_file(&tree.join("amd64/bin/ps")).unwrap();
        let reference = parse_reference_members(&format!(
            "amd64/bin/ps file 775 8 {sha_ps}\namd64/bin/gone file 775 3 feedface\n"
        ))
        .unwrap();
        let emitted = emit_manifest(&reference, &tree, "amd64", "t").unwrap();
        assert_eq!(emitted.unresolved, vec!["amd64/bin/gone".to_string()]);
        assert!(emitted.text.contains("# UNRESOLVED: no tree file matches sha256 feedface"), "{}", emitted.text);
        assert!(emitted.text.contains("generator = \"UNRESOLVED\""), "{}", emitted.text);
        let err = parse_manifest(&emitted.text).unwrap_err().to_string();
        assert!(err.contains("IMAGE-GENERATOR"), "{err}");
    }

    /// Reference dirs map to `[[dir]]` entries; an opaque reference hash is
    /// an explicit UNRESOLVED with its reason.
    #[test]
    fn emit_manifest_maps_dirs_and_flags_opaque_hashes() {
        let scratch = Scratch::new("emit-dirs");
        let tree = tree_with(&scratch, &[("amd64/bin/ps", b"ps-bytes")]);
        let reference =
            parse_reference_members("proc dir 755 - -\nadm/auth file 600 32 opaque\n").unwrap();
        let emitted = emit_manifest(&reference, &tree, "arm64", "t").unwrap();
        assert!(emitted.text.contains("[[dir]]\npath = \"/proc\""), "{}", emitted.text);
        assert!(emitted.text.contains("no content sha256"), "{}", emitted.text);
        assert_eq!(emitted.unresolved, vec!["adm/auth".to_string()]);
    }
}
