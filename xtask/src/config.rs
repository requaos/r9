/// Test
///
use crate::{Command, DynError, Profile};

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File, create_dir_all},
    io::Write,
    path::PathBuf,
    process::exit,
};

/// build section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Build {
    /// The buildflags controls build-time operations and compiler settings.
    pub buildflags: Option<Vec<String>>,

    /// A list of custom flags to pass to all compiler invocations that Cargo performs.
    pub rustflags: Option<Vec<String>>,

    /// Build for the given architecture.
    pub target: String,
}

/// Config section
/// currently available configuration sections are dev, ip, link, nodev, nouart
/// the section name is becomes the prefix for the configuration option
/// example usage for section "dev"
/// ```toml
///  dev = [
///     'arch',
///     'cap',
///     'foo="baz"'
///  ]
/// ```
///  this will create the following configuration options
///  dev_arch, dev_cap and dev_foo="baz"
///
/// usage example:
///  ```rust
/// #[cfg(dev_arch)]
/// pub mod devarch;
/// ```
/// ```rust
/// #[cfg(dev_foo = "baz")]
/// pub mod foobaz;
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub dev: Option<Vec<String>>,
    pub features: Option<Vec<String>>,
    pub ip: Option<Vec<String>>,
    pub link: Option<Vec<String>>,
    pub nodev: Option<Vec<String>>,
    pub nouart: Option<Vec<String>>,

    /// platform/board possible values: empty, raspi3b, vfive2, nezha, virt etc.
    ///
    /// example usage
    /// ´´´rust
    /// #[cfg(platform = "virt")]
    /// pub mod virt;
    /// ```
    pub platform: Option<String>,

    /// Filepath of DTB file relative to crate
    pub dtb: Option<String>,
}

/// Qemu section
/// Affects arguments to be passed to qemu - doesn't affect build artefacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Qemu {
    /// Machine (`-M`) value for qemu: raspi3b, raspi4b, etc.
    pub machine: Option<String>,

    /// Accelerator (`-accel`) value for qemu: tcg, kvm, hvf.
    /// Overridable per-invocation via --accel / R9_ACCEL.
    /// hvf (darwin) requires GICv3 + -cpu host: the virt legs derive that
    /// shape from this TOML via virt_resolved — TOML stays the TCG reference.
    pub accel: Option<String>,

    /// CPU model (`-cpu`) value for qemu: cortex-a72, host, etc.
    /// Overridable per-invocation via --cpu / R9_CPU.
    pub cpu: Option<String>,

    /// CPU count (`-smp`) for qemu.
    pub smp: Option<u32>,

    /// Guest RAM size (`-m`) value for qemu: 1024M, etc.
    pub memory: Option<String>,

    /// Filepath of DTB file relative to crate
    pub dtb: Option<String>,

    /// Initrd image delivery: qemu's `-initrd` flag
    /// never delivers for ELF kernels (hw/arm/boot.c: an ELF `-kernel` is
    /// not-linux, so the image is never loaded into guest RAM and /chosen is
    /// never patched), so the image is placed by qemu's generic loader at a
    /// fixed physical address and xtask writes linux,initrd-start/end into a
    /// patched copy of the DTB (see prepare_initrd_dtb).
    pub initrd: Option<Initrd>,

    /// The idle-then-bound NIC stanza as data (13-01, D-01a): a netdev
    /// backend + PCI device model + optional filter-dump capture, emitted
    /// by apply_netdev (the q35 generalization of the frozen-at-pin
    /// aarch64 apply_virt_netdev emission — that consumer is NOT
    /// migrated this phase).  Emit-only-when-set: an absent table changes
    /// no existing argv shape.
    pub netdev: Option<Netdev>,

    /// Verbatim extra argv tokens appended after the structured keys —
    /// the data path for machine-specific perturbations (13-04's
    /// `-global q35-pcihost.MCFG=0xe0000000` moved-window profile).
    /// Emit-only-when-set.
    pub extra: Option<Vec<String>>,
}

/// The [qemu] netdev table (13-01, D-01a): the NIC staged as data.
/// `mode = "user"` + `device = "virtio-net-pci-non-transitional"` +
/// `pcap = true` expresses `-netdev user,id=n0` +
/// `-device virtio-net-pci-non-transitional,netdev=n0` +
/// `-object filter-dump,id=f0,netdev=n0,file=<path>` — the same stanza
/// shape the aarch64 virt leg emits from Rust, single-sourced here as
/// TOML so Phase 16 (HNET-02) consumes the same table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Netdev {
    /// netdev backend mode ("user" — slirp)
    pub mode: String,

    /// PCI device model bound to the netdev (virtio-net-pci-non-transitional
    /// — the modern-only, non-transitional transport, Phase 7 D-03)
    pub device: String,

    /// attach a per-run filter-dump pcap capture object when true
    pub pcap: bool,
}

/// The [qemu] initrd table: the image file plus the guest physical address
/// the generic loader places it at (initrd PA is TOML data, per the Phase-3
/// config-as-data convention).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Initrd {
    /// Filepath of initrd image relative to crate
    pub file: String,

    /// Guest physical address the image loads at (emitted as the generic
    /// loader's `addr=`).  Must sit inside guest RAM, clear of the kernel
    /// (0x40080000) and DTB (0x40000000) placements.
    pub addr: u64,
}

impl Initrd {
    /// Resolve the initrd image path.  Ordinary values pass through
    /// unchanged (workspace-relative, as today — the qemu steps run with
    /// the workspace as cwd); a `harvey:`-prefixed value resolves the
    /// remainder under the Harvey checkout named by R9_HARVEY_DIR via
    /// uregcheck::harvey_dir — the established cross-repo channel (the
    /// 05-10 D-10 inversion: r9 never hardcodes a sibling layout).  This
    /// is the channel 07-05's virtnet profile uses to boot the
    /// Harvey-baked image.
    pub fn resolved_file(&self) -> Result<PathBuf, String> {
        match self.file.strip_prefix("harvey:") {
            Some(rest) if !rest.is_empty() => {
                let dir = crate::uregcheck::harvey_dir().map_err(|e| e.to_string())?;
                Ok(dir.join(rest))
            }
            Some(_) => {
                Err("qemu initrd: bare `harvey:` prefix names no file within the Harvey checkout"
                    .to_string())
            }
            None => Ok(PathBuf::from(&self.file)),
        }
    }

    /// The `-device` argument placing the image in guest RAM: qemu's generic
    /// loader is machine-agnostic and works for ELF kernels, unlike `-initrd`.
    /// A `harvey:`-prefixed file emits its resolved path; ordinary values
    /// emit byte-identically to the pre-resolution form.
    pub fn loader_device_arg(&self) -> String {
        let file = match self.resolved_file() {
            Ok(path) => path.display().to_string(),
            Err(e) => {
                // Both qemu steps run prepare_initrd_dtb (a Result path)
                // before argv assembly, so a resolution failure surfaces
                // there first; a config fatal here follows the file's
                // eprintln+exit idiom (Configuration::load).
                eprintln!("qemu initrd: {e}");
                exit(1);
            }
        };
        format!("loader,file={},addr={:#x}", file, self.addr)
    }

    /// Path (relative to the workspace) of the DTB copy prepare_initrd_dtb
    /// patches with /chosen linux,initrd-start/end — a target/ build
    /// artifact, never committed; the base DTB blob stays untouched.
    pub fn patched_dtb_path(dtb: &str) -> String {
        let stem = std::path::Path::new(dtb)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "dtb".to_string());
        format!("target/initrd-{stem}.dtb")
    }
}

/// Dist section
/// Selects the boot-image format produced by `cargo xtask dist`:
/// absent or "flat-gzip" = objcopy + gzip flat image (raspi);
/// "elf" = the cargo-built ELF is the boot image, dist is a no-op.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dist {
    /// Boot image format: "flat-gzip" (default when absent) or "elf".
    pub format: Option<String>,
}

/// the TOML document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub build: Option<Build>,
    pub config: Option<Config>,
    pub link: Option<HashMap<String, String>>,
    pub qemu: Option<Qemu>,
    pub dist: Option<Dist>,
}

impl Configuration {
    pub fn load(filename: String) -> Self {
        let contents = match fs::read_to_string(filename.clone()) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("Could not read file `{filename}`");
                exit(1);
            }
        };
        let config: Configuration = match toml::from_str(&contents) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("TOML: Unable to load data from `{filename}`");
                eprintln!("{e}");
                exit(1);
            }
        };
        config
    }
}

fn apply_build(cmd: &mut Command, rustflags: &mut Vec<String>, config: &Configuration) {
    if let Some(config) = &config.build {
        let target = &config.target;
        cmd.arg("--target").arg(target);

        if let Some(flags) = &config.buildflags {
            // add the buildflags to the command
            for f in flags {
                cmd.arg(f);
            }
        }

        if let Some(flags) = &config.rustflags {
            // store the passed rustflags temporarily
            for f in flags {
                rustflags.push(f.to_string());
            }
        }
    }
}

fn apply_platform_config(cmd: &mut Command, rustflags: &mut Vec<String>, config: &Configuration) {
    if let Some(config) = &config.config {
        // if the target will use features make them available
        if let Some(features) = &config.features {
            let mut joined = features.join(",");
            if !features.is_empty() && joined.is_empty() {
                joined = features.first().unwrap().into();
            }
            cmd.arg(format!("--features={joined}"));
        }

        if let Some(platform) = &config.platform {
            rustflags.push("--cfg".into());
            rustflags.push(format!("platform=\"{platform}\""));
        }

        if let Some(devices) = &config.dev {
            // get all [config] 'dev' settings
            for dev in devices {
                rustflags.push("--cfg".into());

                // prefix the setting
                rustflags.push(format!("dev_{dev}"));
            }
        }

        if let Some(ips) = &config.ip {
            // get all [config] 'ip' settings
            for ip in ips {
                rustflags.push("--cfg".into());

                // prefix the setting
                rustflags.push(format!("ip_{ip}"));
            }
        }
        if let Some(links) = &config.link {
            // get all [config] 'link' settings
            for link in links {
                rustflags.push("--cfg".into());

                // prefix the setting
                rustflags.push(format!("link_{link}"));
            }
        }

        if let Some(nodevs) = &config.nodev {
            // get all [config] 'nodev' settings
            for nodev in nodevs {
                rustflags.push("--cfg".into());

                // prefix the setting
                rustflags.push(format!("nodev_{nodev}"));
            }
        }

        if let Some(nouarts) = &config.nouart {
            // get all [config] 'nodev' settings
            for nouart in nouarts {
                rustflags.push("--cfg".into());

                // prefix the setting
                rustflags.push(format!("nouart_{nouart}"));
            }
        }
    }
}

fn apply_link(
    rustflags: &mut Vec<String>,
    config: &Configuration,
    target: &str,
    profile: &Profile,
    workspace_path: &str,
) {
    // we don't need to handle the linker script for clippy
    if let Some(link) = &config.link {
        let filename = link["script"].clone();

        // do we have a linker script ?
        if !filename.is_empty() {
            let mut contents = match fs::read_to_string(format!("{workspace_path}/{filename}")) {
                Ok(c) => c,
                Err(_) => {
                    eprintln!("Could not read file `{filename}`");
                    exit(1);
                }
            };

            // replace the placeholders with the values from the TOML
            if let Some(link) = &config.link {
                for l in link.iter() {
                    match l.0.as_str() {
                        "arch" => contents = contents.replace("${ARCH}", l.1),
                        "load-address" => contents = contents.replace("${LOAD-ADDRESS}", l.1),
                        "script" => {} // do nothing for the script option
                        _ => eprintln!("ignoring unknown option '{} = {}'", l.0, l.1),
                    }
                }
            }

            // construct the path to the target directory
            let path = format!(
                "{}/target/{}/{}",
                workspace_path,
                target,
                profile.to_string().to_lowercase()
            );

            // make sure the target directory exists
            if !std::path::Path::new(&path).exists() {
                // if not, create it
                let _ = create_dir_all(&path);
            }

            // everything is setup, now create the linker script
            // in the target directory
            let mut file = File::create(format!("{path}/kernel.ld")).unwrap();
            let _ = file.write_all(contents.as_bytes());

            // pass the script path to the rustflags
            rustflags.push(format!("-Clink-args=-T{path}/kernel.ld"));
        }
    }
}

/// Per-invocation qemu overrides: values that beat the TOML [qemu]
/// keys at emission time. The full resolution chain (CLI flag > R9_ACCEL /
/// R9_CPU env > TOML > built-in default) is assembled by the caller in main.
#[derive(Debug, Default)]
pub struct QemuOverrides {
    pub accel: Option<String>,
    pub cpu: Option<String>,
}

fn apply_qemu_config(cmd: &mut Command, config: &Configuration, overrides: &QemuOverrides) {
    if let Some(config) = &config.qemu {
        if let Some(machine) = &config.machine {
            cmd.arg("-M");
            cmd.arg(machine);
        }
        // A per-invocation override beats the TOML value.
        if let Some(accel) = overrides.accel.as_ref().or(config.accel.as_ref()) {
            cmd.arg("-accel");
            cmd.arg(accel);
        }
        if let Some(cpu) = overrides.cpu.as_ref().or(config.cpu.as_ref()) {
            cmd.arg("-cpu");
            cmd.arg(cpu);
        }
        if let Some(smp) = &config.smp {
            cmd.arg("-smp");
            cmd.arg(smp.to_string());
        }
        if let Some(memory) = &config.memory {
            cmd.arg("-m");
            cmd.arg(memory);
        }
        if let Some(dtb) = &config.dtb {
            cmd.arg("-dtb");
            // With an initrd configured the kernel needs the /chosen
            // linux,initrd-start/end properties, so qemu gets the patched
            // copy prepare_initrd_dtb produced, never the bare base blob.
            match &config.initrd {
                Some(_) => cmd.arg(Initrd::patched_dtb_path(dtb)),
                None => cmd.arg(dtb),
            };
        }
        if let Some(initrd) = &config.initrd {
            cmd.arg("-device");
            cmd.arg(initrd.loader_device_arg());
        }
        // Verbatim extra tokens ride AFTER the structured keys — never
        // before them (an override of the same flag stays possible).
        if let Some(extra) = &config.extra {
            for arg in extra {
                cmd.arg(arg);
            }
        }
    }
}

/// The q35 netdev stanza, emitted from data (13-01, D-01a).  A [qemu]
/// netdev table (mode/device/pcap) emits `-netdev <mode>,id=n0` +
/// `-device <device>,netdev=n0` + (pcap) `-object
/// filter-dump,id=f0,netdev=n0,file=<path>`, gated on the config's
/// platform == "q35" — the emission is per-platform DATA consumed by a
/// generalized emitter, never a Rust-formatted string keyed on the
/// platform name alone.  Both x86_64 call sites (the interactive QemuStep
/// arm AND BootTestStep) share this one emission, each passing its own
/// per-step pcap path (target/qemu-<config>.pcap /
/// target/boot-test-<config>.pcap — a gate never asserts against a
/// previous run's capture).  The aarch64 `apply_virt_netdev` stays
/// byte-untouched as the frozen-at-pin legacy consumer (OQ-2: no
/// migration this phase).  The device sits idle and unbound (dev = []
/// still): an untouched virtio device never asserts an interrupt — the
/// 07-01 D-10 idle-NIC staging shape.
pub fn apply_netdev(cmd: &mut Command, config: &Configuration, pcap_path: &str) {
    let q35 = config.config.as_ref().and_then(|c| c.platform.as_deref()) == Some("q35");
    if !q35 {
        return;
    }
    let Some(netdev) = config.qemu.as_ref().and_then(|q| q.netdev.as_ref()) else {
        return;
    };
    cmd.arg("-netdev").arg(format!("{},id=n0", netdev.mode));
    cmd.arg("-device").arg(format!("{},netdev=n0", netdev.device));
    if netdev.pcap {
        cmd.arg("-object").arg(format!("filter-dump,id=f0,netdev=n0,file={pcap_path}"));
    }
}

/// virt qemu resolution shared by QemuStep and BootTestStep: the accel
/// override chain completes here (CLI flag / R9_ACCEL > TOML > tcg), and
/// under hvf the effective config swaps the GICv2 machine/dtb pair for the
/// committed gicv3 shapes — qemu 11.1.2's hvf accel rejects GICv2 and
/// accepts only -cpu host, and the kernel's GICv3 driver now covers it.
/// The returned Configuration is the one prepare_initrd_dtb and the apply
/// path must see (the patched-DTB copy derives from the swapped blob).
pub fn virt_resolved(
    config: &Configuration,
    accel_override: &Option<String>,
    cpu_override: &Option<String>,
) -> (Configuration, QemuOverrides) {
    let toml = config.qemu.as_ref();
    let accel = accel_override
        .clone()
        .or_else(|| toml.and_then(|q| q.accel.clone()))
        .unwrap_or_else(|| "tcg".to_string());
    let config = hvf_effective_config(config, &accel);
    // -cpu: the per-invocation override wins; under hvf the only model qemu
    // accepts is host (the TOML's cortex-a72 is the TCG reference, so it
    // must NOT win here); otherwise TOML > built-in cortex-a72.
    let cpu = cpu_override
        .clone()
        .or_else(|| {
            if accel == "hvf" { None } else { config.qemu.as_ref().and_then(|q| q.cpu.clone()) }
        })
        .unwrap_or_else(|| if accel == "hvf" { "host".to_string() } else { "cortex-a72".to_string() });
    (config, QemuOverrides { accel: Some(accel), cpu: Some(cpu) })
}

/// The hvf effective config: swap the GICv2 machine/dtb pair for the gicv3
/// shapes.  Non-default TOML values pass through untouched (a gicv3 profile
/// is already hvf-shaped), and non-hvf accels pass through wholesale.
fn hvf_effective_config(config: &Configuration, accel: &str) -> Configuration {
    let mut config = config.clone();
    if accel != "hvf" {
        return config;
    }
    if let Some(q) = config.qemu.as_mut() {
        if q.machine.as_deref() == Some("virt,gic-version=2") {
            q.machine = Some("virt,gic-version=3".to_string());
        }
        if q.dtb.as_deref() == Some("aarch64/lib/virt-gicv2.dtb") {
            q.dtb = Some("aarch64/lib/virt-gicv3.dtb".to_string());
        }
    }
    config
}

fn apply_rustflags(cmd: &mut Command, rustflags: &[String]) {
    // pass the collected rustflags
    // !! this overrides the build.rustflags from the target Cargo.toml !!
    if !rustflags.is_empty() {
        let flat = rustflags.join(" ");
        cmd.arg("--config");
        cmd.arg(format!("build.rustflags='{flat}'"));
    }
}

/// Host-side initrd DTB patch.  When [qemu] carries an
/// initrd, copy the profile's DTB into target/ and set /chosen
/// linux,initrd-start/end with fdtput (from dtc, supplied by the nix
/// devshell).  The kernel's property_value_as_u64 reads each property as one
/// 8-byte big-endian cell; dtc has no 8-byte property type (its `l` is a
/// 4-byte size modifier and silently truncates u64s), so each u64 is written
/// as two 32-bit cells, high word first — byte-identical to one BE u64.
/// Returns the patched DTB path relative to the workspace, or None when no
/// initrd is configured.  The qemu steps call this BEFORE assembling argv so
/// a patch failure never reaches a qemu launch.
pub fn prepare_initrd_dtb(
    config: &Configuration,
    workspace_path: &str,
) -> Result<Option<String>, DynError> {
    let Some(qemu) = &config.qemu else { return Ok(None) };
    let Some(initrd) = &qemu.initrd else { return Ok(None) };
    let Some(dtb) = &qemu.dtb else {
        return Err("[qemu] initrd requires a [qemu] dtb to patch the /chosen properties into"
            .into());
    };

    let resolved = initrd.resolved_file()?;
    // Ordinary values are workspace-relative (as today); a harvey:-resolved
    // path is used as-is, matching uregcheck's own R9_HARVEY_DIR semantics
    // (the variable's value resolves against the xtask process cwd).
    let image = if initrd.file.starts_with("harvey:") {
        resolved
    } else {
        std::path::Path::new(workspace_path).join(resolved)
    };
    let size = fs::metadata(&image).map_err(|e| format!("initrd image {}: {e}", initrd.file))?.len();
    let patched = Initrd::patched_dtb_path(dtb);
    if let Some(parent) = std::path::Path::new(&patched).parent() {
        create_dir_all(format!("{workspace_path}/{}", parent.display()))?;
    }
    fs::copy(format!("{workspace_path}/{dtb}"), format!("{workspace_path}/{patched}"))
        .map_err(|e| format!("copy {dtb} -> {patched}: {e}"))?;

    let end = initrd.addr + size;
    for (prop, value) in [("linux,initrd-start", initrd.addr), ("linux,initrd-end", end)] {
        let hi = (value >> 32) as u32;
        let lo = value as u32;
        let status = Command::new("fdtput")
            .arg("-t")
            .arg("x")
            .arg(format!("{workspace_path}/{patched}"))
            .arg("/chosen")
            .arg(prop)
            .arg(format!("{hi:#x}"))
            .arg(format!("{lo:#x}"))
            .status()
            .map_err(|e| format!("spawn fdtput (the nix devshell provides dtc): {e}"))?;
        if !status.success() {
            return Err(format!("fdtput {prop} on {patched} failed: {status}").into());
        }
    }
    Ok(Some(patched))
}

pub fn apply_to_clippy_step(cmd: &mut Command, config: &Configuration) {
    let mut rustflags: Vec<String> = Vec::new();
    apply_platform_config(cmd, &mut rustflags, config);
    apply_rustflags(cmd, &rustflags);
}

pub fn apply_to_build_step(
    cmd: &mut Command,
    config: &Configuration,
    target: &str,
    profile: &Profile,
    workspace_path: &str,
) {
    let mut rustflags: Vec<String> = Vec::new();
    apply_build(cmd, &mut rustflags, config);
    apply_platform_config(cmd, &mut rustflags, config);
    apply_link(&mut rustflags, config, target, profile, workspace_path);
    apply_rustflags(cmd, &rustflags);
}

pub fn apply_to_qemu_step_with_overrides(
    cmd: &mut Command,
    config: &Configuration,
    overrides: &QemuOverrides,
) {
    apply_qemu_config(cmd, config, overrides);
}

/// D-09/D-10: the aarch64-virt netdev stanza.  Every aarch64 virt qemu
/// invocation (the interactive QemuStep AND BootTestStep — one shared
/// emission) carries an idle, unbound virtio-net PCI device plus a per-run
/// pcap capture of its netdev: the NIC is present from the first boot of
/// every profile (D-10 always-on) while no kernel code probes it yet, and
/// an untouched virtio device never asserts an interrupt, so existing
/// profiles run byte-identically — the staging proof that the later driver
/// layers (07-02+) land perturbation-free.  The pcap path is per-step and
/// per-config (target/boot-test-<config>.pcap / target/qemu-<config>.pcap)
/// so a gate never asserts against a previous run's capture.  Phase 9 lifts
/// this flag shape into platform TOML (HNET-02); hardcoded here per D-09's
/// no-speculative-machinery rule.
///
/// Gated on the resolved config's platform == "virt": the riscv64 leg's
/// `-netdev type=user,id=net0` + virtio-net-device MMIO stanza is a
/// separate emission site (NOT a device-model precedent), and raspi/x86_64
/// configs gain nothing.  Both call sites are aarch64-only by construction
/// (the QemuStep Aarch64 arm / BootTestStep's arch check), so the composed
/// gate is arch == aarch64 AND platform == "virt".
pub fn apply_virt_netdev(cmd: &mut Command, config: &Configuration, pcap_path: &str) {
    let virt = config.config.as_ref().and_then(|c| c.platform.as_deref()) == Some("virt");
    if !virt {
        return;
    }
    cmd.arg("-netdev").arg("user,id=n0");
    cmd.arg("-device").arg("virtio-net-pci-non-transitional,netdev=n0");
    cmd.arg("-object").arg(format!("filter-dump,id=f0,netdev=n0,file={pcap_path}"));
}

#[cfg(test)]
mod tests {
    use super::{Configuration, QemuOverrides, apply_to_qemu_step_with_overrides};

    /// Parse a TOML document and return the qemu argv it would emit.
    fn qemu_argv(toml_doc: &str) -> Vec<String> {
        let config: Configuration = toml::from_str(toml_doc).expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-aarch64");
        apply_to_qemu_step_with_overrides(&mut cmd, &config, &QemuOverrides::default());
        cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect()
    }

    /// A [qemu] section carrying all six keys (machine/accel/cpu/smp/
    /// memory/dtb) emits each as its qemu flag, in this order, after -M.
    #[test]
    fn qemu_full_keys_emit_all_flags() {
        let args = qemu_argv(
            r#"
                [qemu]
                machine = "virt,gic-version=2"
                accel = "tcg"
                cpu = "cortex-a72"
                smp = 1
                memory = "1024M"
                dtb = "aarch64/lib/virt-gicv2.dtb"
            "#,
        );
        assert_eq!(
            args,
            vec![
                "-M",
                "virt,gic-version=2",
                "-accel",
                "tcg",
                "-cpu",
                "cortex-a72",
                "-smp",
                "1",
                "-m",
                "1024M",
                "-dtb",
                "aarch64/lib/virt-gicv2.dtb",
            ]
        );
    }

    /// A [qemu] initrd table emits the generic-loader
    /// `-device` (qemu's `-initrd` never delivers for ELF kernels) and hands
    /// qemu the patched DTB copy in target/ in place of the base blob — one
    /// edit covers QemuStep and BootTestStep (both share the apply path).
    #[test]
    fn qemu_initrd_emits_loader_and_patched_dtb() {
        let args = qemu_argv(
            r#"
                [qemu]
                machine = "virt,gic-version=2"
                accel = "tcg"
                cpu = "cortex-a72"
                smp = 1
                memory = "1024M"
                dtb = "aarch64/lib/virt-gicv2.dtb"
                initrd = { file = "aarch64/lib/probe-initrd.txt", addr = 0x48000000 }
            "#,
        );
        assert_eq!(
            args,
            vec![
                "-M",
                "virt,gic-version=2",
                "-accel",
                "tcg",
                "-cpu",
                "cortex-a72",
                "-smp",
                "1",
                "-m",
                "1024M",
                "-dtb",
                "target/initrd-virt-gicv2.dtb",
                "-device",
                "loader,file=aarch64/lib/probe-initrd.txt,addr=0x48000000",
            ]
        );
    }

    /// Backward compatibility: a [qemu] section with only today's machine/dtb
    /// keys emits exactly -M and -dtb and nothing else — the regression guard
    /// for every existing config TOML.
    #[test]
    fn qemu_minimal_keys_emit_only_machine_and_dtb() {
        let args =
            qemu_argv("[qemu]\nmachine = \"raspi3b\"\ndtb = \"aarch64/lib/bcm2710-rpi-3-b.dtb\"\n");
        assert_eq!(args, vec!["-M", "raspi3b", "-dtb", "aarch64/lib/bcm2710-rpi-3-b.dtb"]);
    }

    /// D-09/D-10: the aarch64-virt netdev stanza — an idle, unbound
    /// virtio-net-pci-non-transitional plus a per-run filter-dump pcap —
    /// appends to virt configs only.  A non-virt platform (raspi shown; the
    /// riscv64 leg emits its own MMIO stanza elsewhere) gains nothing, so
    /// every non-virt argv shape stays byte-untouched.
    #[test]
    fn virt_netdev_stanza_is_virt_only() {
        let virt: Configuration = toml::from_str(
            "[config]\nplatform = \"virt\"\n[qemu]\nmachine = \"virt,gic-version=2\"\n",
        )
        .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-aarch64");
        super::apply_virt_netdev(&mut cmd, &virt, "target/boot-test-virt.pcap");
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(
            args,
            vec![
                "-netdev",
                "user,id=n0",
                "-device",
                "virtio-net-pci-non-transitional,netdev=n0",
                "-object",
                "filter-dump,id=f0,netdev=n0,file=target/boot-test-virt.pcap",
            ]
        );

        let raspi: Configuration =
            toml::from_str("[config]\nplatform = \"raspi3b\"\n[qemu]\nmachine = \"raspi3b\"\n")
                .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-aarch64");
        super::apply_virt_netdev(&mut cmd, &raspi, "target/qemu-default.pcap");
        assert_eq!(cmd.get_args().count(), 0);
    }

    /// 13-01 (D-01a): the [qemu] netdev table emits the idle-NIC stanza
    /// from data on q35 — the generalization of the frozen-at-pin
    /// aarch64 emission.  The per-step pcap paths ride the caller's
    /// argument: target/boot-test-q35.pcap (BootTestStep) /
    /// target/qemu-q35.pcap (QemuStep).  pcap = false drops only the
    /// filter-dump object; a non-q35 platform (virt shown) gains
    /// nothing — its emission stays apply_virt_netdev's.
    #[test]
    fn q35_netdev_stanza_emits_from_data() {
        let q35: Configuration = toml::from_str(
            "[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n[qemu.netdev]\nmode = \
             \"user\"\ndevice = \"virtio-net-pci-non-transitional\"\npcap = true\n",
        )
        .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &q35, "target/boot-test-q35.pcap");
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(
            args,
            vec![
                "-netdev",
                "user,id=n0",
                "-device",
                "virtio-net-pci-non-transitional,netdev=n0",
                "-object",
                "filter-dump,id=f0,netdev=n0,file=target/boot-test-q35.pcap",
            ]
        );
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &q35, "target/qemu-q35.pcap");
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert!(args.last().is_some_and(|a| a.ends_with("file=target/qemu-q35.pcap")));

        let nocap: Configuration = toml::from_str(
            "[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n[qemu.netdev]\nmode = \
             \"user\"\ndevice = \"virtio-net-pci-non-transitional\"\npcap = false\n",
        )
        .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &nocap, "target/qemu-q35.pcap");
        assert_eq!(cmd.get_args().count(), 4);

        let virt: Configuration = toml::from_str(
            "[config]\nplatform = \"virt\"\n[qemu]\nmachine = \"virt,gic-version=2\"\n[qemu.netdev]\nmode = \
             \"user\"\ndevice = \"virtio-net-pci-non-transitional\"\npcap = true\n",
        )
        .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &virt, "target/qemu-virt.pcap");
        assert_eq!(cmd.get_args().count(), 0);
    }

    /// A [qemu] netdev table with no mode/device/pcap trio is a config
    /// error, not a silently partial stanza — serde names the missing
    /// key before any qemu argv exists.
    #[test]
    fn q35_netdev_table_requires_all_three_keys() {
        let bad = toml::from_str::<Configuration>(
            "[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n[qemu.netdev]\nmode = \
             \"user\"\n",
        );
        assert!(bad.is_err());
    }

    /// 13-01: the [qemu] extra list emits its tokens verbatim, in order,
    /// after the structured keys — the data path 13-04's
    /// `-global q35-pcihost.MCFG=0xe0000000` moved-window profile rides.
    #[test]
    fn q35_extra_args_emit_verbatim_after_structured_keys() {
        let args = qemu_argv(
            r#"
                [qemu]
                machine = "q35"
                accel = "kvm"
                cpu = "host"
                smp = 1
                memory = "1024M"
                extra = ["-global", "q35-pcihost.MCFG=0xe0000000"]
            "#,
        );
        assert_eq!(
            args,
            vec![
                "-M",
                "q35",
                "-accel",
                "kvm",
                "-cpu",
                "host",
                "-smp",
                "1",
                "-m",
                "1024M",
                "-global",
                "q35-pcihost.MCFG=0xe0000000",
            ]
        );
    }

    /// The emit-only-when-set regression guard: a [qemu] section without
    /// the netdev table and the extra list emits NEITHER — every
    /// pre-13-01 argv shape stays byte-identical.
    #[test]
    fn minimal_qemu_section_emits_neither_netdev_nor_extra() {
        let args =
            qemu_argv("[qemu]\nmachine = \"raspi3b\"\ndtb = \"aarch64/lib/bcm2710-rpi-3-b.dtb\"\n");
        assert_eq!(args, vec!["-M", "raspi3b", "-dtb", "aarch64/lib/bcm2710-rpi-3-b.dtb"]);
        let config: Configuration =
            toml::from_str("[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n")
                .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &config, "target/qemu-q35.pcap");
        assert_eq!(cmd.get_args().count(), 0);
    }

    /// The [dist] section's format key parses to Some("elf") ("elf"
    /// selects the ELF-as-boot-image dist path; the flat-gzip default for an
    /// absent section is decided by the consumer).
    #[test]
    fn dist_format_elf_parses() {
        let config: Configuration =
            toml::from_str("[dist]\nformat = \"elf\"\n").expect("test TOML must parse");
        assert_eq!(config.dist.as_ref().and_then(|d| d.format.as_deref()), Some("elf"));
    }

    /// An absent [dist] section parses to None.
    #[test]
    fn dist_absent_parses_to_none() {
        let config: Configuration =
            toml::from_str("[qemu]\nmachine = \"raspi3b\"\n").expect("test TOML must parse");
        assert!(config.dist.is_none());
    }

    /// Emission-layer precedence: an explicit override beats the TOML
    /// value; a None override leaves the TOML value in place.
    #[test]
    fn qemu_overrides_beat_toml() {
        let config: Configuration =
            toml::from_str("[qemu]\naccel = \"tcg\"\ncpu = \"cortex-a72\"\n")
                .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-aarch64");
        let overrides = QemuOverrides { accel: Some("kvm".to_string()), cpu: None };
        apply_to_qemu_step_with_overrides(&mut cmd, &config, &overrides);
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args, vec!["-accel", "kvm", "-cpu", "cortex-a72"]);
    }

    const VIRT_TOML: &str = r#"
        [qemu]
        machine = "virt,gic-version=2"
        accel = "tcg"
        cpu = "cortex-a72"
        smp = 1
        memory = "1024M"
        dtb = "aarch64/lib/virt-gicv2.dtb"
    "#;

    /// hvf resolution swaps the GICv2 machine/dtb pair for the gicv3 shapes
    /// and forces -cpu host over the TOML's TCG reference (qemu's hvf accel
    /// rejects both GICv2 and non-host CPU models).
    #[test]
    fn virt_resolved_hvf_swaps_machine_dtb_and_cpu() {
        let config: Configuration = toml::from_str(VIRT_TOML).expect("test TOML must parse");
        let (effective, overrides) =
            super::virt_resolved(&config, &Some("hvf".to_string()), &None);
        let qemu = effective.qemu.expect("qemu section survives");
        assert_eq!(qemu.machine.as_deref(), Some("virt,gic-version=3"));
        assert_eq!(qemu.dtb.as_deref(), Some("aarch64/lib/virt-gicv3.dtb"));
        assert_eq!(overrides.accel.as_deref(), Some("hvf"));
        assert_eq!(overrides.cpu.as_deref(), Some("host"));
        // The TOML source of truth is untouched (the clone is the swap).
        assert_eq!(
            config.qemu.as_ref().and_then(|q| q.machine.as_deref()),
            Some("virt,gic-version=2")
        );
    }

    /// tcg resolution is the identity: TOML machine/dtb/cpu pass through and
    /// the defaults only fill gaps.
    #[test]
    fn virt_resolved_tcg_passes_through() {
        let config: Configuration = toml::from_str(VIRT_TOML).expect("test TOML must parse");
        let (effective, overrides) = super::virt_resolved(&config, &None, &None);
        let qemu = effective.qemu.expect("qemu section survives");
        assert_eq!(qemu.machine.as_deref(), Some("virt,gic-version=2"));
        assert_eq!(qemu.dtb.as_deref(), Some("aarch64/lib/virt-gicv2.dtb"));
        assert_eq!(overrides.accel.as_deref(), Some("tcg"));
        assert_eq!(overrides.cpu.as_deref(), Some("cortex-a72"));
    }

    /// A per-invocation --cpu beats even the hvf host default (qemu itself
    /// then reports an invalid model — the override is the user's intent).
    #[test]
    fn virt_resolved_cpu_override_beats_hvf_default() {
        let config: Configuration = toml::from_str(VIRT_TOML).expect("test TOML must parse");
        let (_effective, overrides) = super::virt_resolved(
            &config,
            &Some("hvf".to_string()),
            &Some("cortex-a72".to_string()),
        );
        assert_eq!(overrides.cpu.as_deref(), Some("cortex-a72"));
    }

    /// An already-gicv3 TOML is hvf-shaped: no swap, only -cpu host applies.
    #[test]
    fn virt_resolved_hvf_leaves_gicv3_toml_untouched() {
        let config: Configuration = toml::from_str(
            "[qemu]\nmachine = \"virt,gic-version=3\"\ndtb = \"aarch64/lib/virt-gicv3.dtb\"\n",
        )
        .expect("test TOML must parse");
        let (effective, overrides) =
            super::virt_resolved(&config, &Some("hvf".to_string()), &None);
        let qemu = effective.qemu.expect("qemu section survives");
        assert_eq!(qemu.machine.as_deref(), Some("virt,gic-version=3"));
        assert_eq!(qemu.dtb.as_deref(), Some("aarch64/lib/virt-gicv3.dtb"));
        assert_eq!(overrides.cpu.as_deref(), Some("host"));
    }
}
