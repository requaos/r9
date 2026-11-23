//! `cargo xtask gate` — the golden-invariant gate (plan 01-18, D-11/D-12/
//! D-13/D-14, HARN-01/HARN-02).
//!
//! The quick tier (the per-commit gate, D-12) is x86-64 only and does this
//! in order:
//!
//! 1. resolve the read-only amd64 Harvey tree and record its identity (a
//!    store path, or a content digest for a non-store override);
//! 2. assemble `image:cpu` through the checked image engine (its two-way
//!    drift checks run; nothing is written into the tree);
//! 3. build the kernel with `--config q35cpu` through `BuildStep`, and
//!    compose the ISO with the module and append the profile names (the
//!    same `build_x86_64_iso` path the q35 qemu steps use);
//! 4. run `expect gates/x86_64/drawterm-gate.expect` twice — `GATE_MODE=rio`
//!    then `GATE_MODE=control-noG` — or once with `force-no-rio` for the
//!    failing direction.  The env contract is the moved driver's: the
//!    composed ISO, the q35cpu netdev argv from the ONE shared builder,
//!    `DRAWTERM_BIN` from PATH, the creds user, and a 0600 password file
//!    (T-01-58 — the value never rides argv or the environment);
//! 5. prove the tree is unmodified (`nix store verify --no-trust` for a
//!    store path, a recomputed digest otherwise), failing with
//!    `GATE-FAIL harvey-tree-modified`;
//! 6. print `GATE PASS quick x86_64 run=<id>`.
//!
//! The full tier (D-14, phase boundaries and pin bumps) checks every moved
//! gate app exists (`nix eval .#apps.<system>.<name>.program`), runs each
//! with `nix run .#<name>`, records PASS/FAIL in
//! `target/gate/<run-id>/full.tsv`, and fails if any app failed.
//!
//! Venue discipline (D-13): the quick gate refuses a non-Linux host or a
//! missing grub-mkrescue with `GATE-VENUE` naming the Linux devshell, and a
//! KVM run without a writable `/dev/kvm` with `GATE-VENUE kvm-unavailable`.
//! No path returns TCG unless TCG was asked for.

use std::{
    env, fs,
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    Arch, BuildStep, DynError, Profile, Result, assemble_named_image, build_x86_64_iso,
    harvey, load_named_config, workspace,
};
use xtask::config::netdev_args;

/// The driven apps of the full tier (D-14), per architecture.  The names
/// are the flake app attr names the moved gates carry (Harvey's app
/// spellings, kept so CI and the docs read the same commands).
pub const FULL_APPS_X86_64: &[&str] = &[
    "drawterm-gate-x64",
    "boot-r9-x64",
    "boot-r9-smoke-x64",
    "boot-r9-tier1-x64",
    "boot-r9-net-smoke-x64",
    "boot-r9-services-x64",
    "boot-r9-churn-x64",
    "boot-r9-auth-x64",
];

pub const FULL_APPS_AARCH64: &[&str] =
    &["boot-r9-smoke", "boot-r9-net-smoke", "boot-r9-tier1", "boot-r9-cpu"];

/// The full tier's app list for `arch` (empty for riscv64: no moved gate).
pub fn full_apps(arch: Arch) -> &'static [&'static str] {
    match arch {
        Arch::X86_64 => FULL_APPS_X86_64,
        Arch::Aarch64 => FULL_APPS_AARCH64,
        Arch::Riscv64 => &[],
    }
}

/// The quick-gate venue gate: Linux host, grub-mkrescue in PATH.  A Mac
/// run must refuse with `GATE-VENUE` (it is not the witness, D-13), and a
/// Linux run outside the devshell must name the devshell.
pub fn quick_venue(linux: bool, grub_mkrescue: bool) -> std::result::Result<(), String> {
    if !linux {
        return Err(
            "GATE-VENUE not-linux — the quick gate boots a q35 KVM guest and composes a GRUB \
             ISO; run it on the Linux host inside the Linux devshell (`nix develop`), never on \
             the Mac (D-13: the Mac is not the witness)"
                .into(),
        );
    }
    if !grub_mkrescue {
        return Err(
            "GATE-VENUE grub-mkrescue-missing — grub-mkrescue is not in PATH; run inside the \
             Linux devshell (`nix develop`, which carries grub2 and xorriso)"
                .into(),
        );
    }
    Ok(())
}

/// The accel chain (D-13): `--accel` beats `R9_ACCEL` beats kvm.  A kvm
/// request without a writable `/dev/kvm` is an error, never a silent TCG
/// downgrade; TCG is returned only when TCG was asked for.
pub fn resolve_accel(
    flag: Option<&str>,
    env_value: Option<&str>,
    kvm_writable: bool,
) -> Result<String> {
    let value = flag.or(env_value).unwrap_or("kvm");
    match value {
        "kvm" if !kvm_writable => Err(
            "GATE-VENUE kvm-unavailable — /dev/kvm is absent or not writable and kvm was \
             selected; no silent TCG downgrade (D-13).  Pass --accel tcg explicitly for the \
             best-effort leg."
                .into(),
        ),
        "kvm" => Ok("kvm".into()),
        "tcg" => Ok("tcg".into()),
        other => {
            Err(format!("GATE-VENUE accel-unknown — --accel/R9_ACCEL={other}: want kvm|tcg").into())
        }
    }
}

/// Parse one `--serial-check <cmd>=<marker>` pair (the driver types `cmd`
/// on the serial console and requires `marker`).  A value with no `=`, an
/// empty command or an empty marker is an error.
pub fn parse_serial_check(value: &str) -> Result<(String, String)> {
    let Some((cmd, marker)) = value.split_once('=') else {
        return Err(format!(
            "GATE-VENUE serial-check — {value:?} has no `=`: want <cmd>=<marker>"
        )
        .into());
    };
    if cmd.trim().is_empty() {
        return Err("GATE-VENUE serial-check — the command before `=` is empty".into());
    }
    if marker.trim().is_empty() {
        return Err("GATE-VENUE serial-check — the marker after `=` is empty".into());
    }
    Ok((cmd.to_string(), marker.to_string()))
}

/// A stable content digest of a directory tree: an FNV-1a 64 hash over the
/// sorted (relative path, kind, bytes) of every entry.  Stable across runs
/// of one binary, and changes when any file's bytes change (the non-store
/// override's HARN-02 witness).
pub fn tree_digest(dir: &Path) -> Result<String> {
    let mut entries = Vec::new();
    collect_entries(dir, dir, &mut entries)?;
    entries.sort();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (rel, path) in entries {
        fnv_bytes(&mut hash, rel.as_os_str().as_encoded_bytes());
        if path.is_file() {
            fnv_bytes(&mut hash, b"f");
            let mut file = fs::File::open(&path)
                .map_err(|e| -> DynError { format!("tree digest: read {}: {e}", path.display()).into() })?;
            let mut buf = [0u8; 8192];
            loop {
                let n = file.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                fnv_bytes(&mut hash, &buf[..n]);
            }
        } else {
            fnv_bytes(&mut hash, b"d");
        }
    }
    Ok(format!("{hash:016x}"))
}

fn collect_entries(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, PathBuf)>) -> Result<()> {
    let mut children: Vec<_> = fs::read_dir(dir)
        .map_err(|e| -> DynError { format!("tree digest: read dir {}: {e}", dir.display()).into() })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let path = child.path();
        let rel = path
            .strip_prefix(root)
            .map_err(|e| -> DynError { format!("tree digest: {}: {e}", path.display()).into() })?
            .to_path_buf();
        let meta = fs::symlink_metadata(&path)?;
        if meta.is_dir() {
            out.push((rel, path.clone()));
            collect_entries(root, &path, out)?;
        } else {
            out.push((rel, path));
        }
    }
    Ok(())
}

fn fnv_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

/// The full tier's app list for a flake system name.
fn flake_system(arch: Arch) -> Result<&'static str> {
    match arch {
        Arch::X86_64 => Ok("x86_64-linux"),
        Arch::Aarch64 => Ok("aarch64-linux"),
        Arch::Riscv64 => {
            Err("GATE-VENUE riscv64 — no moved gate app exists for riscv64 (D-14)".into())
        }
    }
}

pub struct GateStep {
    arch: Arch,
    full: bool,
    force_no_rio: bool,
    serial_checks: Vec<String>,
    accel: Option<String>,
    verbose: bool,
}

impl GateStep {
    pub fn new(matches: &clap::ArgMatches) -> Self {
        let arch = Arch::from(matches);
        let full = matches.get_flag("full");
        let force_no_rio = matches.get_flag("force-no-rio");
        let serial_checks = matches
            .get_many::<String>("serial-check")
            .map(|values| values.cloned().collect())
            .unwrap_or_default();
        let accel = matches
            .try_get_one::<String>("accel")
            .ok()
            .flatten()
            .cloned()
            .or_else(|| env::var("R9_ACCEL").ok())
            .filter(|value| !value.trim().is_empty());
        let verbose = matches.get_flag("verbose");
        Self { arch, full, force_no_rio, serial_checks, accel, verbose }
    }

    pub fn run(self) -> Result<()> {
        for check in &self.serial_checks {
            parse_serial_check(check)?;
        }
        if self.full {
            return self.run_full();
        }
        quick_venue(cfg!(target_os = "linux"), command_in_path("grub-mkrescue").is_some())
            .map_err(|e| -> DynError { e.into() })?;
        if self.arch != Arch::X86_64 {
            return Err(
                "GATE-VENUE arch — the quick gate is x86-64 only (D-14); the aarch64 apps \
                 belong to the full tier (--full)"
                    .into(),
            );
        }
        let accel = resolve_accel(
            self.accel.as_deref(),
            env_value("R9_ACCEL").as_deref(),
            kvm_writable(),
        )?;
        self.run_quick(&accel)
    }

    fn run_quick(self, accel: &str) -> Result<()> {
        if command_in_path("expect").is_none() {
            return Err(
                "GATE-VENUE expect-missing — the drawterm driver is an expect script; run \
                 inside the Linux devshell (`nix develop`)"
                    .into(),
            );
        }
        let drawterm_bin = drawterm_bin()?;
        let run_id = run_id();
        let gate_dir = target_gate_dir(&run_id);
        fs::create_dir_all(&gate_dir)?;
        let q35cpu = load_named_config(Arch::X86_64, "q35cpu");
        let cpu = env_value("R9_CPU").unwrap_or_else(|| cpu_default(accel).to_string());
        println!("gate: venue accel={accel} cpu={cpu} run={run_id}");

        // 1. the tree identity (store path, or a digest for an override).
        let tree = harvey::harvey_dir(Arch::X86_64)?;
        let store_tree = tree.starts_with("/nix/store");
        let tree_before = if store_tree { None } else { Some(tree_digest(&tree)?) };
        println!(
            "gate: tree {} ({})",
            tree.display(),
            if store_tree {
                "store path — nix store verify after the run".to_string()
            } else {
                format!("override tree — digest {}", tree_before.clone().unwrap_or_default())
            }
        );

        // 2. the cpu image through the checked engine (drift + provenance).
        let assembled = assemble_named_image(Arch::X86_64, "cpu", self.verbose)?;
        println!(
            "gate: image cpu {} sha256 {} members {}",
            assembled.paq.display(),
            assembled.sha256,
            assembled.members.len()
        );

        // 3. the kernel through BuildStep, the ISO through the q35 path.
        BuildStep::for_gate(Arch::X86_64, "q35cpu", self.verbose).run()?;
        let kernel_rel =
            format!("target/{}/{}/x86_64", Arch::X86_64.target(), Profile::Debug.dir());
        let iso_rel = build_x86_64_iso(&kernel_rel, "q35cpu", &q35cpu, self.verbose)?;
        let iso = workspace().join(&iso_rel);
        println!("gate: iso {} ", iso.display());

        // 4. the creds: 0600 file path only (T-01-58).
        let creds_path = workspace().join("target/images/x86_64/cpu.creds");
        let (user, password) = read_creds(&creds_path)?;
        let pwfile = gate_dir.join("dt-password");
        write_private(&pwfile, &password)?;
        drop(password);

        let netdev = netdev_args(&q35cpu, "slirp", None).join(" ");
        if netdev.trim().is_empty() {
            let _ = fs::remove_file(&pwfile);
            return Err(
                "GATE-VENUE netdev — config_q35cpu carries no slirp argv (the D-04 hostfwd \
                 pair is the driver's dial contract)"
                    .into(),
            );
        }

        let legs: Vec<&str> =
            if self.force_no_rio { vec!["force-no-rio"] } else { vec!["rio", "control-noG"] };
        let mut leg_failure: Option<DynError> = None;
        for leg in legs {
            let log = gate_dir.join(format!("{leg}.log"));
            println!("gate: leg {leg} (capture {})", log.display());
            let (ok, marker) = self.run_driver(
                leg,
                accel,
                &cpu,
                &iso,
                &log,
                &user,
                &pwfile,
                &creds_path,
                &drawterm_bin,
                &netdev,
                &run_id,
            )?;
            if !ok {
                let marker = marker.unwrap_or_else(|| format!("GATE-FAIL {leg}-leg-failed"));
                leg_failure = Some(format!("{marker} (leg {leg}; capture {})", log.display()).into());
                break;
            }
        }
        let _ = fs::remove_file(&pwfile);

        // 5. the tree-unmodified proof (HARN-02), on pass AND fail.
        let tree_failure: Option<String> = if store_tree {
            store_verify(&tree).err().map(|e| e.to_string())
        } else {
            let after = tree_digest(&tree)?;
            if Some(&after) == tree_before.as_ref() {
                None
            } else {
                Some(format!(
                    "GATE-FAIL harvey-tree-modified — override tree {} changed during the gate \
                     (digest {} -> {after})",
                    tree.display(),
                    tree_before.clone().unwrap_or_default()
                ))
            }
        };
        if let Some(failure) = tree_failure {
            return Err(failure.into());
        }
        if let Some(failure) = leg_failure {
            return Err(failure);
        }

        println!("GATE PASS quick x86_64 run={run_id}");
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn run_driver(
        &self,
        leg: &str,
        accel: &str,
        cpu: &str,
        iso: &Path,
        log: &Path,
        user: &str,
        pwfile: &Path,
        creds_path: &Path,
        drawterm_bin: &Path,
        netdev: &str,
        run_id: &str,
    ) -> Result<(bool, Option<String>)> {
        let driver = workspace().join("gates/x86_64/drawterm-gate.expect");
        if !driver.is_file() {
            return Err(format!(
                "GATE-VENUE driver-missing — {} is not in this checkout",
                driver.display()
            )
            .into());
        }
        let mut command = Command::new("expect");
        command
            .arg(&driver)
            .current_dir(workspace())
            .env("R9_ISO", iso)
            .env("R9_SMOKE_LOG", log)
            .env("R9_NETDEV_ARGV", netdev)
            .env("R9_DEMO_USER", user)
            .env("R9_DEMO_PWFILE", pwfile)
            .env("R9_CREDS_FILE", creds_path)
            .env("DRAWTERM_BIN", drawterm_bin)
            .env("R9_QEMU", Arch::X86_64.qemu_system())
            .env("R9_MACHINE", "q35")
            .env("R9_CPU", cpu)
            .env("R9_ACCEL", accel)
            .env("R9_RUN_TAG", format!("r9x64dtgate-{run_id}"))
            .env("R9_DT_TAG", format!("r9x64dtgate-dt-{run_id}"))
            .env("GATE_MODE", leg)
            .env("GATE_DT_USER", "glenda")
            .env("GATE_SERIAL_CHECKS", self.serial_checks.join("\n"));
        if self.verbose {
            println!("Executing {command:?}");
        }
        let output = command.output().map_err(|e| -> DynError {
            format!(
                "GATE-VENUE expect-spawn — {e}: the driver needs expect on PATH (the Linux \
                 devshell carries it)"
            )
            .into()
        })?;
        print!("{}", String::from_utf8_lossy(&output.stdout));
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        if output.status.success() {
            return Ok((true, None));
        }
        Ok((false, driver_marker(&output)))
    }

    fn run_full(self) -> Result<()> {
        let system = flake_system(self.arch)?;
        let apps = full_apps(self.arch);
        if apps.is_empty() {
            return Err("GATE-VENUE full — no moved gate app list exists for this architecture".into());
        }
        let run_id = run_id();
        let gate_dir = target_gate_dir(&run_id);
        fs::create_dir_all(&gate_dir)?;
        if command_in_path("nix").is_none() {
            return Err(
                "GATE-VENUE nix-missing — the full tier drives the flake apps with nix; run \
                 inside the Linux devshell (`nix develop`)"
                    .into(),
            );
        }

        let mut missing = Vec::new();
        for app in apps {
            let reference = format!(".#apps.{system}.{app}.program");
            let ok = Command::new("nix")
                .args(["eval", "--raw", &reference])
                .current_dir(workspace())
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false);
            if !ok {
                missing.push(*app);
            }
        }
        if !missing.is_empty() {
            return Err(format!(
                "GATE-FAIL full-apps-missing — apps.{system} does not provide: {} (plans 01-21, \
                 01-23, 01-24, 01-30 and 01-31 move them)",
                missing.join(", ")
            )
            .into());
        }

        let mut failed = Vec::new();
        let mut tsv = String::from("app\tresult\n");
        for app in apps {
            println!("gate: full app {app}");
            let ok = Command::new("nix")
                .args(["run", &format!(".#{app}")])
                .current_dir(workspace())
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            tsv.push_str(&format!("{app}\t{}\n", if ok { "PASS" } else { "FAIL" }));
            if !ok {
                failed.push(*app);
            }
        }
        fs::write(gate_dir.join("full.tsv"), tsv)?;
        if !failed.is_empty() {
            return Err(format!(
                "GATE-FAIL full-run — apps failed: {} (results: {})",
                failed.join(", "),
                gate_dir.join("full.tsv").display()
            )
            .into());
        }
        println!("GATE PASS full {system} run={run_id}");
        Ok(())
    }
}

fn env_value(var: &str) -> Option<String> {
    env::var(var).ok().filter(|value| !value.trim().is_empty())
}

fn cpu_default(accel: &str) -> &'static str {
    if accel == "kvm" { "host" } else { "qemu64" }
}

fn kvm_writable() -> bool {
    fs::OpenOptions::new().write(true).open("/dev/kvm").is_ok()
}

fn command_in_path(name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    env::split_paths(&paths).map(|dir| dir.join(name)).find(|candidate| {
        candidate.is_file()
            && fs::metadata(candidate)
                .map(|meta| meta.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    })
}

fn drawterm_bin() -> Result<PathBuf> {
    if let Some(value) = env_value("DRAWTERM_BIN") {
        return Ok(PathBuf::from(value));
    }
    command_in_path("drawterm").ok_or_else(|| -> DynError {
        "GATE-VENUE drawterm-missing — drawterm is not in PATH and DRAWTERM_BIN is unset; run \
         inside the Linux devshell (`nix develop`, which carries Harvey's drawterm) or pass \
         DRAWTERM_BIN"
            .into()
    })
}

fn run_id() -> String {
    let seconds =
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
    format!("{}-{seconds}", std::process::id())
}

fn target_gate_dir(run_id: &str) -> PathBuf {
    workspace().join("target/gate").join(run_id)
}

fn read_creds(path: &Path) -> Result<(String, String)> {
    let text = fs::read_to_string(path).map_err(|e| -> DynError {
        format!(
            "GATE-VENUE creds — {}: {e}: the image assembler writes this 0600 file \
             (target/images/x86_64/cpu.creds); assemble image:cpu first",
            path.display()
        )
        .into()
    })?;
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key))
            .map(|value| value.trim_end_matches('\r').to_string())
            .filter(|value| !value.is_empty())
    };
    let user = field("user=").ok_or_else(|| -> DynError {
        format!("GATE-VENUE creds — {}: no non-empty user= line", path.display()).into()
    })?;
    let password = field("password=").ok_or_else(|| -> DynError {
        format!("GATE-VENUE creds — {}: no non-empty password= line", path.display()).into()
    })?;
    Ok((user, password))
}

fn write_private(path: &Path, secret: &str) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| -> DynError { format!("GATE-VENUE pwfile — {}: {e}", path.display()).into() })?;
    file.write_all(secret.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn driver_marker(output: &Output) -> Option<String> {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text.lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with("GATE-FAIL"))
        .map(str::to_string)
}

fn store_verify(path: &Path) -> Result<()> {
    let output = Command::new("nix")
        .args(["store", "verify", "--no-trust"])
        .arg(path)
        .output()
        .map_err(|e| -> DynError {
            format!(
                "GATE-VENUE nix-missing — `nix store verify` could not run ({e}); run inside \
                 the Linux devshell (`nix develop`)"
            )
            .into()
        })?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "GATE-FAIL harvey-tree-modified — `nix store verify --no-trust {}` failed: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("r9-gate-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn accel_prefers_the_flag_then_the_env_then_kvm() {
        assert_eq!(resolve_accel(Some("tcg"), Some("kvm"), true).unwrap(), "tcg");
        assert_eq!(resolve_accel(None, Some("tcg"), true).unwrap(), "tcg");
        assert_eq!(resolve_accel(None, None, true).unwrap(), "kvm");
        assert_eq!(resolve_accel(Some("kvm"), None, true).unwrap(), "kvm");
    }

    #[test]
    fn accel_never_downgrades_kvm_to_tcg() {
        let err = resolve_accel(None, None, false).unwrap_err().to_string();
        assert!(err.contains("GATE-VENUE kvm-unavailable"), "{err}");
        assert!(!err.contains("tcg —"), "a kvm venue error must not return tcg: {err}");
        let err = resolve_accel(None, Some("kvm"), false).unwrap_err().to_string();
        assert!(err.contains("GATE-VENUE kvm-unavailable"), "{err}");
        // TCG is returned only when it was asked for.
        assert_eq!(resolve_accel(Some("tcg"), None, false).unwrap(), "tcg");
        let err = resolve_accel(Some("hvf"), None, true).unwrap_err().to_string();
        assert!(err.contains("GATE-VENUE accel-unknown"), "{err}");
    }

    #[test]
    fn quick_venue_refuses_the_mac_and_names_the_linux_devshell() {
        let err = quick_venue(false, true).unwrap_err();
        assert!(err.contains("GATE-VENUE"), "{err}");
        assert!(err.contains("Linux devshell"), "{err}");

        let err = quick_venue(true, false).unwrap_err();
        assert!(err.contains("GATE-VENUE"), "{err}");
        assert!(err.contains("grub-mkrescue"), "{err}");
        assert!(err.contains("Linux devshell"), "{err}");

        assert!(quick_venue(true, true).is_ok());
    }

    #[test]
    fn parse_serial_check_splits_on_the_first_equals() {
        let (cmd, marker) = parse_serial_check("/bin/ps=R9-RUST-AOUT-PROBE-OK").unwrap();
        assert_eq!(cmd, "/bin/ps");
        assert_eq!(marker, "R9-RUST-AOUT-PROBE-OK");
        let (cmd, marker) = parse_serial_check("echo a=b").unwrap();
        assert_eq!(cmd, "echo a");
        assert_eq!(marker, "b");
        assert!(parse_serial_check("no-equals").is_err());
        assert!(parse_serial_check("=marker").is_err());
        assert!(parse_serial_check("cmd=").is_err());
    }

    #[test]
    fn tree_digest_is_stable_and_changes_with_file_bytes() {
        let dir = scratch("digest");
        fs::write(dir.join("a.txt"), "one").unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/b.txt"), "two").unwrap();

        let first = tree_digest(&dir).unwrap();
        let second = tree_digest(&dir).unwrap();
        assert_eq!(first, second, "the digest must be stable across runs");

        fs::write(dir.join("a.txt"), "ONE").unwrap();
        let changed = tree_digest(&dir).unwrap();
        assert_ne!(first, changed, "a file byte change must change the digest");

        fs::write(dir.join("sub/c.txt"), "three").unwrap();
        let added = tree_digest(&dir).unwrap();
        assert_ne!(changed, added, "an added file must change the digest");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn full_tier_lists_are_complete() {
        let x86_64 = full_apps(Arch::X86_64);
        for app in [
            "drawterm-gate-x64",
            "boot-r9-x64",
            "boot-r9-smoke-x64",
            "boot-r9-tier1-x64",
            "boot-r9-net-smoke-x64",
            "boot-r9-services-x64",
            "boot-r9-churn-x64",
            "boot-r9-auth-x64",
        ] {
            assert!(x86_64.contains(&app), "x86-64 full tier lacks {app}: {x86_64:?}");
        }
        assert_eq!(x86_64.len(), 8);

        let aarch64 = full_apps(Arch::Aarch64);
        for app in ["boot-r9-smoke", "boot-r9-net-smoke", "boot-r9-tier1", "boot-r9-cpu"] {
            assert!(aarch64.contains(&app), "aarch64 full tier lacks {app}: {aarch64:?}");
        }
        assert_eq!(aarch64.len(), 4);

        assert!(full_apps(Arch::Riscv64).is_empty());
    }
}
