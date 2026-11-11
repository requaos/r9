/// Test
///
use crate::{DynError, Profile};

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File, create_dir_all},
    io::Write,
    path::PathBuf,
    process::{Command, exit},
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

    /// The host-net mode tables as data (13-01 D-01a, re-shaped 16-01
    /// D-05): a default selector plus one optional sub-table per mode
    /// (slirp/tap/vmnet), emitted by the ONE shared builder
    /// netdev_args through the apply_netdev front-end (the q35
    /// generalization of the frozen-at-pin aarch64 apply_virt_netdev
    /// emission — that consumer is NOT migrated).  Emit-only-when-set:
    /// an absent table changes no existing argv shape.
    pub netdev: Option<Netdev>,

    /// Verbatim extra argv tokens appended after the structured keys —
    /// the data path for machine-specific perturbations (13-04's
    /// `-global q35-pcihost.MCFG=0xe0000000` moved-window profile).
    /// Emit-only-when-set.
    pub extra: Option<Vec<String>>,
}

/// The [qemu.netdev] mode tables (13-01 D-01a, re-shaped 16-01 D-05):
/// a `default` selector plus one OPTIONAL sub-table per host-net mode.
/// Every mode's argv comes from ONE builder — netdev_args below —
/// consumed by the in-process apply_netdev front-end AND the exported
/// r9-netdev-args bin (Harvey's flake input; two front-ends, one
/// emission).  The sub-table KEY is the selector name
/// (slirp/tap/vmnet); each table's `mode` field is the qemu backend
/// string it emits ("user", "tap", "vmnet-socket") — the two are
/// distinct, never conflated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Netdev {
    /// default selector used when a caller passes no explicit mode
    /// ("slirp" — the hermetic, zero-privilege, CI-able venue HNET-01)
    pub default: String,

    /// the slirp/user table — the committed default venue
    pub slirp: Option<SlirpNetdev>,

    /// the linux tap/bridge table (HNET-03; the helper path is
    /// explicit data per 16-RESEARCH Pitfall 1 — the compiled-in
    /// default helper is the nosuid store libexec on nixpkgs qemu)
    pub tap: Option<TapNetdev>,

    /// the darwin socket_vmnet table (HNET-04 — documented-manual;
    /// its data still lands in the ONE config so the emitter prints
    /// the exact argv the darwin docs cite)
    pub vmnet: Option<VmnetNetdev>,
}

/// [qemu.netdev.slirp] — the qemu "user" backend as data.  The hostfwd
/// rules bind loopback host addresses ONLY (16-01 D-04): the host
/// entry point for auth is 127.0.0.1:5670 -> guest 567, because a
/// literal host:567 bind needs privilege on default Linux
/// (ip_unprivileged_port_start 1024) and the hermetic default must
/// stay zero-privilege; the 1:1 sysctl variant is docs-only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlirpNetdev {
    /// qemu backend string ("user" — the slirp selector's backend;
    /// the selector KEY is "slirp", this is the wire name)
    pub mode: String,

    /// PCI device model bound to the netdev (virtio-net-pci-non-transitional
    /// — the modern-only, non-transitional transport, Phase 7 D-03)
    pub device: String,

    /// hostfwd rules, appended to the -netdev token in order —
    /// loopback-bound host addresses only (D-04)
    pub hostfwd: Vec<String>,

    /// attach a per-run filter-dump pcap capture object when true AND
    /// the caller supplies a capture path (the gating is the
    /// builder's: data enables, the caller places)
    pub pcap: bool,
}

/// [qemu.netdev.tap] — the linux bridge-attached tap backend as data.
/// `br` and `helper` are BOTH required keys: the helper MUST be
/// explicit (Pitfall 1 — qemu's compiled-in default is its own nosuid
/// store libexec, where the setuid bit is dead; NixOS's live wrapper
/// is /run/wrappers/bin/qemu-bridge-helper).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TapNetdev {
    /// qemu backend string ("tap")
    pub mode: String,

    /// PCI device model bound to the netdev
    pub device: String,

    /// host bridge the tap attaches to (br0 — the NAT'd private
    /// bridge, 16-CONTEXT D-08)
    pub br: String,

    /// path of the setuid qemu-bridge-helper wrapper
    pub helper: String,

    /// attach a per-run filter-dump pcap capture object when true AND
    /// the caller supplies a capture path
    pub pcap: bool,
}

/// [qemu.netdev.vmnet] — the darwin socket_vmnet backend as data
/// (HNET-04, documented-manual this era: socket_vmnet runs as root
/// exposing vmnet over a UNIX socket; qemu's builtin vmnet-* backends
/// are entitlement-walled on nixpkgs-built qemu).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmnetNetdev {
    /// qemu backend string ("vmnet-socket" — qemu >= 9.0)
    pub mode: String,

    /// PCI device model bound to the netdev
    pub device: String,

    /// the socket_vmnet UNIX socket path (16-RESEARCH darwin facts;
    /// /var/run/socket_vmnet is the conventional default)
    #[serde(rename = "socket-path")]
    pub socket_path: String,

    /// attach a per-run filter-dump pcap capture object when true AND
    /// the caller supplies a capture path
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

    /// Boot module staged into the x86_64 ISO at /boot/bootfs.paq with
    /// the grub.cfg `module2` placeholder line emitted IN (14-05, D-03
    /// — GRUB delivers bootfs as an mb2 tag-3 module; the kernel finds
    /// it through the tag walker, never a fixed address).  A `harvey:`
    /// prefix resolves the remainder under the Harvey checkout named by
    /// R9_HARVEY_DIR (the 07-05 initrd channel's discipline — absence is
    /// a loud error naming the Harvey-side producers).  Absent key =
    /// kernel-only ISO: no module staged, no module2 line emitted (the
    /// staged loader config stays byte-identical to Phase 13's, so the
    /// wired boot-iso FOD hash holds).
    pub module: Option<String>,
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

/// The ONE netdev argv builder (16-01, D-05): every -netdev /
/// -device / filter-dump token for every mode, keyed on the selector
/// name.  The fatal front-end: an unknown mode (or a mode whose
/// sub-table this config lacks) prints the error and exits 1 — the
/// eprintln/exit idiom (Initrd::loader_device_arg) — never a
/// silently empty argv.  Both consumers (the in-process apply_netdev
/// and the exported r9-netdev-args bin) route through this one
/// implementation; nothing re-assembles netdev argv by hand.
pub fn netdev_args(config: &Configuration, mode: &str, pcap_path: Option<&str>) -> Vec<String> {
    match netdev_args_checked(config, mode, pcap_path) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("qemu netdev: {e}");
            exit(1);
        }
    }
}

/// The pure, testable core of netdev_args (the emitter bin prints its
/// own usage line from the Err).  pcap gating: the filter-dump object
/// exists only when the mode table's pcap is true AND the caller
/// supplies a capture path — a path alone (pcap = false) emits
/// nothing, and the table's pcap = true alone emits nothing.
pub fn netdev_args_checked(
    config: &Configuration,
    mode: &str,
    pcap_path: Option<&str>,
) -> Result<Vec<String>, String> {
    let Some(netdev) = config.qemu.as_ref().and_then(|q| q.netdev.as_ref()) else {
        return Err("no [qemu.netdev] table in this config".to_string());
    };
    let (netdev_token, device, pcap) = match mode {
        "slirp" => {
            let Some(t) = netdev.slirp.as_ref() else {
                return Err(
                    "netdev mode `slirp` has no [qemu.netdev.slirp] table in this config"
                        .to_string(),
                );
            };
            let mut token = format!("{},id=n0", t.mode);
            for fwd in &t.hostfwd {
                token.push_str(",hostfwd=");
                token.push_str(fwd);
            }
            (token, &t.device, t.pcap)
        }
        _ => {
            return Err(format!("unknown netdev mode `{mode}`: want slirp|tap|vmnet"));
        }
    };
    let mut args = vec!["-netdev".to_string(), netdev_token];
    args.push("-device".to_string());
    args.push(format!("{device},netdev=n0"));
    if pcap {
        if let Some(path) = pcap_path {
            args.push("-object".to_string());
            args.push(format!("filter-dump,id=f0,netdev=n0,file={path}"));
        }
    }
    Ok(args)
}

/// The q35 netdev emission front-end (13-01 D-01a; a thin wrapper
/// since 16-01 D-05): resolve the effective mode — these in-process
/// call sites boot the TOML default (no explicit selector exists at
/// either qemu step) — and push the ONE builder's argv onto the
/// Command.  Gated on the config's platform == "q35": a non-q35
/// config gains nothing.  Both x86_64 call sites (the interactive
/// QemuStep arm AND BootTestStep) share this one emission, each
/// passing its own per-step pcap path (target/qemu-<config>.pcap /
/// target/boot-test-<config>.pcap — a gate never asserts against a
/// previous run's capture).  The aarch64 `apply_virt_netdev` stays
/// byte-untouched as the frozen-at-pin legacy consumer (OQ-2: no
/// migration).  The device sits idle and unbound until a driver binds
/// it — the 07-01 D-10 idle-NIC staging shape.
pub fn apply_netdev(cmd: &mut Command, config: &Configuration, pcap_path: &str) {
    let q35 = config.config.as_ref().and_then(|c| c.platform.as_deref()) == Some("q35");
    if !q35 {
        return;
    }
    let Some(netdev) = config.qemu.as_ref().and_then(|q| q.netdev.as_ref()) else {
        return;
    };
    for arg in netdev_args(config, &netdev.default, Some(pcap_path)) {
        cmd.arg(arg);
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

    /// 13-01 (D-01a) / 16-01 (D-05): the [qemu.netdev] mode tables
    /// emit the slirp default stanza from data on q35 — the sub-table
    /// shape (default selector + [qemu.netdev.slirp]) carrying the
    /// D-04 hostfwd set verbatim, loopback-bound host addresses only
    /// (host 17010 -> guest 17010, host 5670 -> guest 567).  The
    /// per-step pcap paths ride the caller's argument:
    /// target/boot-test-q35shell.pcap (BootTestStep) /
    /// target/qemu-q35shell.pcap (QemuStep).  pcap = false drops only
    /// the filter-dump object; a non-q35 platform (virt shown) gains
    /// nothing — its emission stays apply_virt_netdev's.
    #[test]
    fn q35_netdev_stanza_emits_from_data() {
        let q35: Configuration = toml::from_str(
            "[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n[qemu.netdev]\ndefault = \
             \"slirp\"\n[qemu.netdev.slirp]\nmode = \"user\"\ndevice = \
             \"virtio-net-pci-non-transitional\"\nhostfwd = [\"tcp:127.0.0.1:17010-:17010\", \
             \"tcp:127.0.0.1:5670-:567\"]\npcap = true\n",
        )
        .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &q35, "target/boot-test-q35shell.pcap");
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(
            args,
            vec![
                "-netdev",
                "user,id=n0,hostfwd=tcp:127.0.0.1:17010-:17010,hostfwd=tcp:127.0.0.1:5670-:567",
                "-device",
                "virtio-net-pci-non-transitional,netdev=n0",
                "-object",
                "filter-dump,id=f0,netdev=n0,file=target/boot-test-q35shell.pcap",
            ]
        );
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &q35, "target/qemu-q35shell.pcap");
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert!(args.last().is_some_and(|a| a.ends_with("file=target/qemu-q35shell.pcap")));

        let nocap: Configuration = toml::from_str(
            "[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n[qemu.netdev]\ndefault = \
             \"slirp\"\n[qemu.netdev.slirp]\nmode = \"user\"\ndevice = \
             \"virtio-net-pci-non-transitional\"\nhostfwd = [\"tcp:127.0.0.1:17010-:17010\", \
             \"tcp:127.0.0.1:5670-:567\"]\npcap = false\n",
        )
        .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &nocap, "target/qemu-q35shell.pcap");
        assert_eq!(cmd.get_args().count(), 4);

        let virt: Configuration = toml::from_str(
            "[config]\nplatform = \"virt\"\n[qemu]\nmachine = \"virt,gic-version=2\"\n[qemu.netdev]\ndefault = \
             \"slirp\"\n[qemu.netdev.slirp]\nmode = \"user\"\ndevice = \
             \"virtio-net-pci-non-transitional\"\nhostfwd = []\npcap = true\n",
        )
        .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &virt, "target/qemu-virt.pcap");
        assert_eq!(cmd.get_args().count(), 0);
    }

    /// A [qemu.netdev.slirp] table missing any required key is a
    /// config error, not a silently partial stanza — serde names the
    /// missing sub-table key before any qemu argv exists (16-01,
    /// re-keyed from the flat-stanza trio guard).
    #[test]
    fn q35_netdev_table_requires_all_three_keys() {
        let bad = toml::from_str::<Configuration>(
            "[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n[qemu.netdev]\ndefault = \
             \"slirp\"\n[qemu.netdev.slirp]\nmode = \"user\"\ndevice = \
             \"virtio-net-pci-non-transitional\"\npcap = true\n",
        );
        let err = bad.expect_err("a slirp table without hostfwd must not deserialize");
        assert!(
            err.to_string().contains("hostfwd"),
            "serde must name the missing sub-table key: {err}"
        );
    }

    /// A [qemu.netdev] table without a default selector is a config
    /// error — the in-process front-ends resolve their effective mode
    /// from it (16-01, D-05).
    #[test]
    fn netdev_table_requires_default_key() {
        let bad = toml::from_str::<Configuration>(
            "[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n[qemu.netdev.slirp]\nmode \
             = \"user\"\ndevice = \"virtio-net-pci-non-transitional\"\nhostfwd = []\npcap = \
             true\n",
        );
        let err = bad.expect_err("a netdev table without default must not deserialize");
        assert!(
            err.to_string().contains("default"),
            "serde must name the missing default key: {err}"
        );
    }

    /// 16-01 (D-05) pcap gating, caller-side: the slirp table's
    /// pcap = true alone emits NO filter-dump — the capture object
    /// exists only when the builder is also handed a path.  The
    /// default-path argv stays the four netdev/device tokens.
    #[test]
    fn netdev_args_without_pcap_path_emits_no_filter_dump() {
        let q35: Configuration = toml::from_str(
            "[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n[qemu.netdev]\ndefault = \
             \"slirp\"\n[qemu.netdev.slirp]\nmode = \"user\"\ndevice = \
             \"virtio-net-pci-non-transitional\"\nhostfwd = [\"tcp:127.0.0.1:17010-:17010\", \
             \"tcp:127.0.0.1:5670-:567\"]\npcap = true\n",
        )
        .expect("test TOML must parse");
        let args = super::netdev_args_checked(&q35, "slirp", None).expect("slirp must emit");
        assert_eq!(
            args,
            vec![
                "-netdev",
                "user,id=n0,hostfwd=tcp:127.0.0.1:17010-:17010,hostfwd=tcp:127.0.0.1:5670-:567",
                "-device",
                "virtio-net-pci-non-transitional,netdev=n0",
            ]
        );
    }

    /// 16-01 (D-05), the two-front-ends rule pinned in-process:
    /// apply_netdev (the qemu steps' front-end, resolving the TOML
    /// default mode) and the shared builder emit byte-identical argv
    /// for the same config and pcap path — the exported bin rides the
    /// same builder, so its stdout is this same sequence.
    #[test]
    fn netdev_args_matches_apply_netdev_default_path() {
        let q35: Configuration = toml::from_str(
            "[config]\nplatform = \"q35\"\n[qemu]\nmachine = \"q35\"\n[qemu.netdev]\ndefault = \
             \"slirp\"\n[qemu.netdev.slirp]\nmode = \"user\"\ndevice = \
             \"virtio-net-pci-non-transitional\"\nhostfwd = [\"tcp:127.0.0.1:17010-:17010\", \
             \"tcp:127.0.0.1:5670-:567\"]\npcap = true\n",
        )
        .expect("test TOML must parse");
        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        super::apply_netdev(&mut cmd, &q35, "target/qemu-q35shell.pcap");
        let via_apply: Vec<String> =
            cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        let via_builder =
            super::netdev_args(&q35, "slirp", Some("target/qemu-q35shell.pcap"));
        assert_eq!(via_apply, via_builder);
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
