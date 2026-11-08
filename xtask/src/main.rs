use crate::config::Configuration;
use config::{
    QemuOverrides, apply_netdev, apply_to_build_step, apply_to_clippy_step,
    apply_to_qemu_step_with_overrides, apply_virt_netdev, prepare_initrd_dtb, virt_resolved,
};
use std::{
    env, fmt,
    io::{Read, Write},
    os::unix::{net::UnixStream, process::CommandExt},
    path::{Path, PathBuf},
    process::{self, Command},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use target_lexicon::Triple;

mod config;
// D-06 gate machinery: consumed by the Net profile's finish() pcap
// assertion (07-05) and exercised by its in-file unit suite.
mod pcap;
mod uregcheck;

type DynError = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, DynError>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    fn from(matches: &clap::ArgMatches) -> Self {
        if matches.get_flag("release") { Profile::Release } else { Profile::Debug }
    }

    fn dir(&self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, clap::ValueEnum)]
enum Arch {
    Aarch64,
    Riscv64,
    X86_64,
}

impl Arch {
    fn from(matches: &clap::ArgMatches) -> Self {
        *matches.get_one::<Arch>("arch").unwrap_or(&Arch::X86_64)
    }

    fn qemu_system(&self) -> String {
        env_or(
            "QEMU",
            match self {
                Arch::Aarch64 => "qemu-system-aarch64",
                Arch::Riscv64 => "qemu-system-riscv64",
                Arch::X86_64 => "qemu-system-x86_64",
            },
        )
    }

    fn target(&self) -> String {
        env_or("TARGET", format!("{}-unknown-none-elf", self.to_string().to_lowercase()).as_str())
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

struct RustupState {
    installed_targets: Vec<Triple>,
    curr_toolchain: String,
}

impl RustupState {
    /// Runs rustup command to get a list of all installed toolchains.
    /// Also caches the current toolchain.
    ///
    /// Tolerates rustup being entirely absent (e.g. the nix devshell, which
    /// supplies rustc directly via the overlay with no rustup shims): the
    /// state is then empty, `std_supported_target` returns None, and
    /// CheckStep/TestStep skip their OS-toolchain legs through their existing
    /// Option/let-else handling while the bins/lib checks still run.
    fn new() -> Self {
        Self {
            installed_targets: Self::installed_rustup_targets().unwrap_or_default(),
            curr_toolchain: env::var("RUSTUP_TOOLCHAIN").unwrap_or_default(),
        }
    }

    /// Call `rustup target list --installed` to get all installed target triples
    fn installed_rustup_targets() -> Result<Vec<Triple>> {
        let output =
            Command::new("rustup").arg("target").arg("list").arg("--installed").output()?;
        if !output.status.success() {
            return Err(String::from_utf8(output.stdout.clone())?.into());
        }

        Ok(String::from_utf8(output.stdout.clone())?.lines().flat_map(Triple::from_str).collect())
    }

    /// For the given arch, return a compatible toolchain triple that is
    /// installed and can be used by cargo check.  It will prefer the default
    /// toolchain if it's a match, otherwise it will look for the
    /// <arch-unknown-linux-gnu> toolchain.
    fn std_supported_target(&self, arch: &str) -> Option<&Triple> {
        let arch = Self::target_arch(arch);
        self.installed_targets.iter().filter(|&t| t.architecture.to_string() == arch).find(|&t| {
            self.curr_toolchain.ends_with(&t.to_string())
                || t.to_string() == arch.to_owned() + "-unknown-linux-gnu"
        })
    }

    /// Return the arch in a form compatible with the supported targets and toolchains
    fn target_arch(arch: &str) -> &str {
        match arch {
            "riscv64" => "riscv64gc",
            _ => arch,
        }
    }
}

fn main() {
    let matches = clap::Command::new("xtask")
        .version("0.1.0")
        .author("The r9 Authors")
        .about("Build support for the r9 operating system")
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("build").about("Builds r9").args(&[
                clap::arg!(--release "Build release version").conflicts_with("debug"),
                clap::arg!(--debug "Build debug version (default)").conflicts_with("release"),
                clap::arg!(--arch <arch> "Target architecture")
                    .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                clap::arg!(--config <name> "Configuration")
                    .value_parser(clap::builder::NonEmptyStringValueParser::new())
                    .default_value("default"),
                clap::arg!(--verbose "Print commands"),
            ]),
        )
        .subcommand(
            clap::Command::new("expand").about("Expands r9 macros").args(&[
                clap::arg!(--release "Build release version").conflicts_with("debug"),
                clap::arg!(--debug "Build debug version (default)").conflicts_with("release"),
                clap::arg!(--arch <arch> "Target architecture")
                    .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                clap::arg!(--verbose "Print commands"),
            ]),
        )
        .subcommand(
            clap::Command::new("kasm").about("Emits r9 assembler").args(&[
                clap::arg!(--release "Build release version").conflicts_with("debug"),
                clap::arg!(--debug "Build debug version (default)").conflicts_with("release"),
                clap::arg!(--arch <arch> "Target architecture")
                    .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                clap::arg!(--verbose "Print commands"),
            ]),
        )
        .subcommand(
            clap::Command::new("dist").about("Builds a multibootable r9 image").args(&[
                clap::arg!(--release "Build a release version").conflicts_with("debug"),
                clap::arg!(--debug "Build a debug version").conflicts_with("release"),
                clap::arg!(--arch <arch> "Target architecture")
                    .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                clap::arg!(--config <name> "Configuration")
                    .value_parser(clap::builder::NonEmptyStringValueParser::new())
                    .default_value("default"),
                clap::arg!(--verbose "Print commands"),
            ]),
        )
        .subcommand(clap::Command::new("test").about("Runs unit tests").args(&[
            clap::arg!(--release "Build a release version").conflicts_with("debug"),
            clap::arg!(--debug "Build a debug version").conflicts_with("release"),
            clap::arg!(--json "Output messages as json"),
            clap::arg!(--verbose "Print commands"),
        ]))
        .subcommand(
            clap::Command::new("clippy").about("Runs clippy").args(&[
                clap::arg!(--release "Build a release version").conflicts_with("debug"),
                clap::arg!(--debug "Build a debug version").conflicts_with("release"),
                clap::arg!(--arch <arch> "Target architecture")
                    .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                clap::arg!(--config <name> "Configuration")
                    .value_parser(clap::builder::NonEmptyStringValueParser::new())
                    .default_value("default"),
                clap::arg!(--verbose "Print commands"),
            ]),
        )
        .subcommand(clap::Command::new("check").about("Runs check").args(&[
            clap::arg!(--json "Output messages as json"),
            clap::arg!(--verbose "Print commands"),
        ]))
        .subcommand(
            clap::Command::new("qemu").about("Run r9 under QEMU").args(&[
                clap::arg!(--release "Build a release version").conflicts_with("debug"),
                clap::arg!(--debug "Build a debug version").conflicts_with("release"),
                clap::arg!(--arch <arch> "Target architecture")
                    .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                clap::arg!(--gdb "Wait for gdb connection on start"),
                clap::arg!(--kvm "Run with KVM"),
                clap::arg!(--config <name> "Configuration")
                    .value_parser(clap::builder::NonEmptyStringValueParser::new())
                    .default_value("default"),
                clap::arg!(--verbose "Print commands"),
                clap::arg!(--dump_dtb <file> "Dump the DTB from QEMU to a file")
                    .value_parser(clap::value_parser!(String)),
                clap::arg!(--accel <accel> "Accelerator override (tcg/kvm/hvf); beats R9_ACCEL env and [qemu] accel (D-06)")
                    .value_parser(clap::builder::NonEmptyStringValueParser::new()),
                clap::arg!(--cpu <cpu> "CPU model override; beats R9_CPU env and [qemu] cpu (D-06)")
                    .value_parser(clap::builder::NonEmptyStringValueParser::new()),
            ]),
        )
        .subcommand(
            clap::Command::new("boot-test")
                .about("Boot r9 under QEMU on a serial socket and assert the ordered boot markers (D-11)")
                .args(&[
                    clap::arg!(--release "Build a release version").conflicts_with("debug"),
                    clap::arg!(--debug "Build a debug version").conflicts_with("release"),
                    clap::arg!(--arch <arch> "Target architecture")
                        .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                    clap::arg!(--config <name> "Configuration")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .default_value("default"),
                    clap::arg!(--verbose "Print commands"),
                    clap::arg!(--accel <accel> "Accelerator override (tcg/kvm/hvf); beats R9_ACCEL env and [qemu] accel (D-06)")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new()),
                    clap::arg!(--cpu <cpu> "CPU model override; beats R9_CPU env and [qemu] cpu (D-06)")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new()),
                ]),
        )
        .subcommand(
            clap::Command::new("ureg-check").about(
                "Verify the Harvey checkout's aarch64/include/ureg.h and \
                 amd64/include/ureg.h match the pinned layouts (D-06/D-02b layer 2, \
                 amd64 arm + negative control; R9_HARVEY_DIR names the checkout — \
                 required since 05-10, 05-CONTEXT D-10)",
            ),
        )
        .subcommand(
            clap::Command::new("initrd")
                .about("Assemble target/bootfs.paq from the staged Harvey aarch64 binaries \
                        (04-05, D-01/D-02/D-03; R9_HARVEY_DIR names the checkout — required \
                        since 05-10, 05-CONTEXT D-10)")
                .args(&[
                    clap::arg!(--arch <arch> "Target architecture")
                        .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                    clap::arg!(--config <name> "Configuration")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .default_value("default"),
                    clap::arg!(--verbose "Print commands"),
                ]),
        )
        .subcommand(clap::Command::new("clean").about("Cargo clean"))
        .get_matches();

    if let Err(e) = match matches.subcommand() {
        Some(("build", m)) => BuildStep::new(m).run(),
        Some(("expand", m)) => ExpandStep::new(m).run(),
        Some(("kasm", m)) => KasmStep::new(m).run(),
        Some(("dist", m)) => {
            let s1 = BuildStep::new(m);
            let s2 = DistStep::new(m);
            s1.run().and_then(|_| s2.run())
        }
        Some(("test", m)) => TestStep::new(m).run(),
        Some(("clippy", m)) => ClippyStep::new(m).run(),
        Some(("check", m)) => CheckStep::new(m).run(),
        Some(("qemu", m)) => {
            let s1 = BuildStep::new(m);
            let s2 = DistStep::new(m);
            let s3 = QemuStep::new(m);
            s1.run().and_then(|_| s2.run()).and_then(|_| s3.run())
        }
        Some(("boot-test", m)) => {
            // Evidence freshness: the harness always builds the image it
            // tests in the same invocation (never certifies a stale tree).
            let s1 = BuildStep::new(m);
            let s2 = BootTestStep::new(m);
            s1.run().and_then(|_| s2.run())
        }
        Some(("ureg-check", _)) => UregCheckStep::new().run(),
        Some(("initrd", m)) => InitrdStep::new(m).run(),
        Some(("clean", _)) => CleanStep::new().run(),
        _ => Err("bad subcommand".into()),
    } {
        eprintln!("{e}");
        process::exit(1);
    }
}

fn env_or(var: &str, default: &str) -> String {
    let default = default.to_string();
    env::var(var).unwrap_or(default)
}

fn cargo() -> String {
    env_or("CARGO", "cargo")
}

fn objcopy() -> String {
    let llvm_objcopy = {
        let toolchain = env_or("RUSTUP_TOOLCHAIN", "nightly-x86_64-unknown-none");

        // find host architecture by taking last 3 segments from toolchain
        let mut arch_segments: Box<[_]> = toolchain.split('-').rev().take(3).collect();
        arch_segments.reverse();
        let host = arch_segments.join("-");

        let home = env_or("RUSTUP_HOME", "");
        let mut path = PathBuf::from(home);
        path.push("toolchains");
        path.push(toolchain);
        path.push("lib");
        path.push("rustlib");
        path.push(host);
        path.push("bin");
        path.push("llvm-objcopy");
        if path.exists() {
            path.into_os_string().into_string().unwrap()
        } else {
            "llvm-objcopy".into()
        }
    };
    env_or("OBJCOPY", &llvm_objcopy)
}

fn load_config(arch: Arch, matches: &clap::ArgMatches) -> Configuration {
    let default = "default".to_string();
    let config_file = matches.try_get_one("config").ok().flatten().unwrap_or(&default);
    Configuration::load(format!(
        "{}/{}/lib/config_{}.toml",
        workspace().display(),
        arch.to_string().to_lowercase(),
        config_file
    ))
}

fn verbose(matches: &clap::ArgMatches) -> bool {
    matches.get_flag("verbose")
}

struct BuildStep {
    arch: Arch,
    config: Configuration,
    profile: Profile,
    verbose: bool,
}

impl BuildStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        let arch = Arch::from(matches);
        let config = load_config(arch, matches);
        let profile = Profile::from(matches);
        let verbose = verbose(matches);

        Self { arch, config, profile, verbose }
    }

    fn run(self) -> Result<()> {
        let mut cmd = Command::new(cargo());
        cmd.arg("build");

        apply_to_build_step(
            &mut cmd,
            &self.config,
            &self.arch.target(),
            &self.profile,
            workspace().to_str().unwrap(),
        );

        cmd.current_dir(workspace());
        cmd.arg("--workspace");
        cmd.arg("--exclude").arg("xtask");
        exclude_other_arches(self.arch, &mut cmd);
        if self.profile == Profile::Release {
            cmd.arg("--release");
        }
        cmd.arg("-Z").arg("build-std=core,alloc");
        if self.verbose {
            println!("Executing {cmd:?}");
        }
        let status = annotated_status(&mut cmd)?;
        if !status.success() {
            return Err("build kernel failed".into());
        }
        Ok(())
    }
}

struct DistStep {
    arch: Arch,
    config: Configuration,
    profile: Profile,
    verbose: bool,
}

impl DistStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        let arch = Arch::from(matches);
        let config = load_config(arch, matches);
        let profile = Profile::from(matches);
        let verbose = verbose(matches);
        Self { arch, config, profile, verbose }
    }

    fn run(self) -> Result<()> {
        match self.arch {
            Arch::Aarch64 => {
                // [dist] format = "elf" — the cargo-built ELF is the
                // boot image; skip the objcopy+gzip flat-image pipeline
                // entirely. Absent section / "flat-gzip" keeps today's shape.
                if self.config.dist.as_ref().and_then(|d| d.format.as_deref()) == Some("elf") {
                    println!(
                        "dist format \"elf\": the kernel ELF is the boot image; skipping objcopy/gzip"
                    );
                } else {
                    // Qemu needs a flat binary in order to handle device tree files correctly
                    let mut cmd = Command::new(objcopy());
                    cmd.arg("-O");
                    cmd.arg("binary");
                    cmd.arg(format!(
                        "target/{}/{}/aarch64",
                        self.arch.target(),
                        self.profile.dir()
                    ));
                    cmd.arg(format!(
                        "target/{}/{}/aarch64-qemu",
                        self.arch.target(),
                        self.profile.dir()
                    ));
                    cmd.current_dir(workspace());
                    if self.verbose {
                        println!("Executing {cmd:?}");
                    }
                    let status = annotated_status(&mut cmd)?;
                    if !status.success() {
                        return Err("objcopy failed".into());
                    }

                    // Compress the binary.  We do this because they're much faster when used
                    // for netbooting and qemu also accepts them.
                    let mut cmd = Command::new("gzip");
                    cmd.arg("-k");
                    cmd.arg("-f");
                    cmd.arg(format!(
                        "target/{}/{}/aarch64-qemu",
                        self.arch.target(),
                        self.profile.dir()
                    ));
                    cmd.current_dir(workspace());
                    if self.verbose {
                        println!("Executing {cmd:?}");
                    }
                    let status = annotated_status(&mut cmd)?;
                    if !status.success() {
                        return Err("gzip failed".into());
                    }
                }
            }
            Arch::X86_64 => {
                // D-02 retired the elf32 objcopy conversion (the -kernel
                // load path): the q35 configs ship the ELF64 itself,
                // staged into the grub-mkrescue ISO by the qemu steps
                // (see build_x86_64_iso).  Any other format value is a
                // config error — there is no x86_64 flat-image pipeline
                // anymore.
                if self.config.dist.as_ref().and_then(|d| d.format.as_deref()) == Some("elf") {
                    println!(
                        "dist format \"elf\": the kernel ELF feeds the grub-mkrescue ISO; no objcopy"
                    );
                } else {
                    return Err(
                        "x86_64 dist supports only format = \"elf\" (the mb2/GRUB2 channel; the \
                         32-bit objcopy conversion path is retired)"
                            .into(),
                    );
                }
            }
            Arch::Riscv64 => {
                // Qemu needs a flat binary in order to handle device tree files correctly
                let mut cmd = Command::new(objcopy());
                cmd.arg("-O");
                cmd.arg("binary");
                cmd.arg(format!("target/{}/{}/riscv64", self.arch.target(), self.profile.dir()));
                cmd.arg(format!(
                    "target/{}/{}/riscv64-qemu",
                    self.arch.target(),
                    self.profile.dir()
                ));
                cmd.current_dir(workspace());
                if self.verbose {
                    println!("Executing {cmd:?}");
                }
                let status = annotated_status(&mut cmd)?;
                if !status.success() {
                    return Err("objcopy failed".into());
                }
            }
        };

        Ok(())
    }
}

struct QemuStep {
    arch: Arch,
    config: Configuration,
    config_name: String,
    profile: Profile,
    wait_for_gdb: bool,
    kvm: bool,
    dump_dtb: String,
    accel: Option<String>,
    cpu: Option<String>,
    verbose: bool,
}

impl QemuStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        let arch = Arch::from(matches);
        let config = load_config(arch, matches);
        let config_name = matches
            .try_get_one::<String>("config")
            .ok()
            .flatten()
            .cloned()
            .unwrap_or_else(|| "default".to_string());
        let profile = Profile::from(matches);
        let wait_for_gdb = matches.get_flag("gdb");
        let kvm = matches.get_flag("kvm");
        let dump_dtb: String = matches
            .try_get_one::<String>("dump_dtb")
            .ok()
            .flatten()
            .unwrap_or(&"".to_string())
            .clone();
        // The override chain, per-invocation layer: the CLI flag beats the
        // R9_ACCEL / R9_CPU environment value. The TOML value and the built-in
        // default are layered on at emission time in run().
        let accel = matches
            .try_get_one::<String>("accel")
            .ok()
            .flatten()
            .cloned()
            .or_else(|| env::var("R9_ACCEL").ok());
        let cpu = matches
            .try_get_one::<String>("cpu")
            .ok()
            .flatten()
            .cloned()
            .or_else(|| env::var("R9_CPU").ok());
        let verbose = verbose(matches);

        Self { arch, config, config_name, profile, wait_for_gdb, kvm, dump_dtb, accel, cpu, verbose }
    }

    fn run(self) -> Result<()> {
        let target = self.arch.target();
        let dir = self.profile.dir();
        let qemu_system = self.arch.qemu_system();

        if self.kvm && self.arch != Arch::X86_64 {
            return Err("KVM only supported under x86-64".into());
        }

        match self.arch {
            Arch::Aarch64 => {
                let platform = self.config.config.as_ref().and_then(|c| c.platform.as_deref());
                // virt resolves the accel/cpu override chain and the hvf
                // effective config up front (the gicv3 machine/dtb/host-cpu
                // swap) so the initrd DTB patching below sees the swapped
                // blob; raspi keeps the historical emit-only-when-set shape.
                let (config, overrides) = if platform == Some("virt") {
                    virt_resolved(&self.config, &self.accel, &self.cpu)
                } else {
                    (
                        self.config.clone(),
                        QemuOverrides { accel: self.accel.clone(), cpu: self.cpu.clone() },
                    )
                };

                // A config whose initrd image lives under target/
                // (the assembled bootfs.paq) builds it when missing or stale
                // — a missing producer is a loud error, never a silent boot
                // without userland.
                ensure_initrd_image(&config, self.verbose)?;
                // If the config carries an initrd, patch a target/ copy
                // of the DTB with /chosen linux,initrd-start/end BEFORE qemu's
                // argv is assembled (the apply path emits the patched path).
                prepare_initrd_dtb(&config, workspace().to_str().unwrap())?;

                let mut cmd = Command::new(qemu_system);

                if platform == Some("virt") {
                    // virt: --accel/--cpu flag > R9_ACCEL/R9_CPU env > TOML
                    // > built-in default (tcg + cortex-a72); --accel hvf
                    // selects the gicv3 machine/DTB and -cpu host (see
                    // virt_resolved).
                    apply_to_qemu_step_with_overrides(&mut cmd, &config, &overrides);

                    // D-09/D-10: the idle virtio-net NIC + fresh per-run
                    // pcap capture ride every aarch64 virt boot (the same
                    // emission BootTestStep carries, with the interactive
                    // step's own pcap name).
                    apply_virt_netdev(
                        &mut cmd,
                        &config,
                        &format!("target/qemu-{}.pcap", self.config_name),
                    );

                    cmd.arg("-nographic");

                    // On virt the PL011 is serial0 and must carry stdio
                    // (research P7): a single -serial mon:stdio. The raspi
                    // null + second-serial pair is wrong here.
                    cmd.arg("-serial");
                    cmd.arg("mon:stdio");

                    if self.wait_for_gdb {
                        cmd.arg("-s").arg("-S");
                    }
                    // Show exception level change events in stdout
                    cmd.arg("-d");
                    cmd.arg("int");
                    // The cargo-built ELF is the -kernel directly
                    // (DistStep skips the objcopy/gzip flat-image step).
                    cmd.arg("-kernel");
                    cmd.arg(format!("target/{target}/{dir}/aarch64"));
                } else {
                    // raspi: flag/env overrides still beat the TOML value if
                    // supplied, but with none set nothing new is emitted — the
                    // existing invocation shape stays byte-identical.
                    apply_to_qemu_step_with_overrides(&mut cmd, &config, &overrides);

                    // TODO Choose UART at cmdline
                    // If using UART0 (PL011), this enables serial
                    cmd.arg("-nographic");

                    // If using UART1 (MiniUART), this enables serial
                    cmd.arg("-serial");
                    cmd.arg("null");
                    cmd.arg("-serial");
                    cmd.arg("mon:stdio");

                    if self.wait_for_gdb {
                        cmd.arg("-s").arg("-S");
                    }
                    // Show exception level change events in stdout
                    cmd.arg("-d");
                    cmd.arg("int");
                    cmd.arg("-kernel");
                    cmd.arg(format!("target/{target}/{dir}/aarch64-qemu.gz"));
                }
                cmd.current_dir(workspace());
                if self.verbose {
                    println!("Executing {cmd:?}");
                }
                let status = annotated_status(&mut cmd)?;
                if !status.success() {
                    return Err("qemu failed".into());
                }
            }
            Arch::Riscv64 => {
                let mut cmd = Command::new(qemu_system);
                cmd.arg("-nographic");
                //cmd.arg("-curses");
                // cmd.arg("-bios").arg("none");
                let dump_dtb = &self.dump_dtb;
                if !dump_dtb.is_empty() {
                    cmd.arg("-machine").arg(format!("virt,dumpdtb={dump_dtb}"));
                } else {
                    cmd.arg("-machine").arg("virt");
                }
                cmd.arg("-cpu").arg("rv64");
                // FIXME: This is not needed as of now, and will only work once the
                // FIXME: disk.bin is also taken care of. Doesn't exist by default.
                if false {
                    cmd.arg("-drive").arg("file=disk.bin,format=raw,id=hd0");
                    cmd.arg("-device").arg("virtio-blk-device,drive=hd0");
                }
                cmd.arg("-netdev").arg("type=user,id=net0");
                cmd.arg("-device").arg("virtio-net-device,netdev=net0");
                cmd.arg("-smp").arg("4");
                cmd.arg("-m").arg("1024M");
                cmd.arg("-serial").arg("mon:stdio");
                if self.wait_for_gdb {
                    cmd.arg("-s").arg("-S");
                }
                cmd.arg("-d").arg("guest_errors,unimp");
                cmd.arg("-kernel");
                cmd.arg(format!("target/{target}/{dir}/riscv64"));
                cmd.current_dir(workspace());
                if self.verbose {
                    println!("Executing {cmd:?}");
                }
                let status = annotated_status(&mut cmd)?;
                if !status.success() {
                    return Err("qemu failed".into());
                }
            }
            Arch::X86_64 => {
                // D-01/D-02: the q35 channel is TOML-driven through the
                // same apply path aarch64 uses (machine/accel/cpu/smp/
                // memory from [qemu], --accel/--cpu flag > R9_ACCEL/
                // R9_CPU env > TOML at emission) — the retired hardcoded
                // -M q35/-smp 8/-m 8192/-kernel arm is gone.  The kernel
                // ships inside the grub-mkrescue ISO and boots via
                // -cdrom (SeaBIOS -> GRUB2 -> mb2 -> kernel), never as a
                // -kernel ELF.
                let overrides = QemuOverrides {
                    accel: self.accel.clone(),
                    cpu: self.cpu.clone(),
                };
                let kernel = format!("target/{target}/{dir}/x86_64");
                let iso = build_x86_64_iso(&kernel, &self.config_name, &self.config, self.verbose)?;

                let mut cmd = Command::new(qemu_system);
                apply_to_qemu_step_with_overrides(&mut cmd, &self.config, &overrides);
                // D-01a: the idle NIC + per-run pcap ride every q35
                // invocation from the [qemu] netdev table (data; the
                // aarch64 apply_virt_netdev stays the frozen-at-pin
                // legacy consumer).
                apply_netdev(
                    &mut cmd,
                    &self.config,
                    &format!("target/qemu-{}.pcap", self.config_name),
                );
                cmd.arg("-nographic");
                // COM1 (the kernel console) on stdio, monitor muxed.
                cmd.arg("-serial");
                cmd.arg("mon:stdio");
                if self.wait_for_gdb {
                    cmd.arg("-s").arg("-S");
                }
                cmd.arg("-cdrom");
                cmd.arg(&iso);
                cmd.current_dir(workspace());
                if self.verbose {
                    println!("Executing {cmd:?}");
                }
                let status = annotated_status(&mut cmd)?;
                if !status.success() {
                    return Err("qemu failed".into());
                }
            }
        };

        Ok(())
    }
}

/// `cargo xtask boot-test`: build the kernel, boot it under qemu with
/// the serial console on a unix socket, assert the ordered boot markers
/// (banner -> >=3 periodic ticks -> typed-char echo), and write the captured
/// serial log.  `server=on,wait=on` on the socket chardev holds guest start
/// until we connect, giving capture-before-start for free (the socket
/// chardev drops pre-connect data without it).
///
/// Zero new crate dependencies: std::os::unix::net::UnixStream + std::process
/// only (rexpect/expectrl remain the fallback ONLY if this
/// socket design proves infeasible).  Marker assertions only, never timings
/// (TCG speed varies).
struct BootTestStep {
    arch: Arch,
    config: Configuration,
    config_name: String,
    profile: Profile,
    accel: Option<String>,
    cpu: Option<String>,
    verbose: bool,
}

/// The NETIP-08 soak's host echo endpoint (08-08, Task 2): a
/// std::net::TcpListener bound to 127.0.0.1:<the seam's blastport>
/// (loopback-only bind — T-08-14's mitigation: the guest reaches it via
/// slirp's gateway forwarding, so NO external interface ever sees it).
/// Spawned BEFORE qemu starts (the guest never dials a closed port) and
/// joined after the run; a startup line names the bound port so the
/// harness log carries it.  std-only (no new crates — T-08-SC).
struct HostEcho {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl HostEcho {
    /// Bind 127.0.0.1:port and spawn the accept+echo thread.  The
    /// listener runs nonblocking with a short poll cadence so the stop
    /// flag is observed promptly; each accepted stream echoes until
    /// EOF/error/stop.
    fn spawn(port: u16) -> Result<HostEcho> {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))
            .map_err(|e| format!("netip soak gate: bind 127.0.0.1:{port}: {e}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("netip soak gate: nonblocking listener: {e}"))?;
        println!("netip: host echo endpoint listening on 127.0.0.1:{port} (the NETIP-08 soak's far end; loopback-only per T-08-14)");
        let stop = Arc::new(AtomicBool::new(false));
        let stop_thread = stop.clone();
        let thread = std::thread::spawn(move || {
            loop {
                if stop_thread.load(Ordering::Relaxed) {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _peer)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(100)));
                        let mut buf = [0u8; 65536];
                        loop {
                            if stop_thread.load(Ordering::Relaxed) {
                                break;
                            }
                            match stream.read(&mut buf) {
                                Ok(0) => break,      // the peer closed
                                Ok(n) => {
                                    if stream.write_all(&buf[..n]).is_err() {
                                        break;
                                    }
                                }
                                // A read timeout just polls the stop flag.
                                Err(e)
                                    if e.kind() == std::io::ErrorKind::WouldBlock
                                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                                Err(_) => break,
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(HostEcho { stop, thread: Some(thread) })
    }

    /// Signal the endpoint to stop and join its thread (the post-run
    /// teardown — one site covers pass/fail/timeout).
    fn stop_and_join(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// Assertion markers, named so every profile reuses them.
const BOOT_BANNER_MARKER: &str = "r9 from the Internet";
const BOOT_TICK_MARKER: &str = "tick";
const BOOT_TICK_MIN: usize = 3;
/// The probe string typed into the serial socket.  qemu's socket serial does
/// not self-echo, so these bytes can only reappear via the kernel's INTID-33
/// RX echo path.
const BOOT_ECHO_PROBE: &[u8] = b"r9echo7";
/// Overall wall-clock budget for the whole marker sequence (overridable via
/// R9_BOOT_TEST_TIMEOUT); one total budget, never per-marker timings.
const BOOT_TEST_TIMEOUT_SECS: u64 = 300;
/// The x86_64 -cdrom channel's FIRST-marker budget (overridable via
/// R9_BOOT_TEST_FIRST_MARKER_TIMEOUT): SeaBIOS + GRUB2 add seconds of
/// firmware wall-clock before the kernel banner (Pitfall 8), so the banner
/// wait is budgeted generously on its own, INSIDE the overall budget above
/// — a wedged firmware/bootloader fails fast with a channel-aware message
/// instead of consuming the whole total.  Order, never timings: this is a
/// wall-clock allowance, not a timing assertion.
const BOOT_FIRST_MARKER_TIMEOUT_SECS: u64 = 20;

/// The same budgets under TCG (13-03): SeaBIOS+GRUB run tens of times
/// slower there, and GRUB's BIOS-disk load of the debug kernel ELF (18 MB
/// class) is the dominant cost — measured at 13+ MINUTES on this host
/// (KVM boots the same ISO in seconds).  The multipliers scale both
/// budgets when the EFFECTIVE accel resolves to tcg (flag > TOML, same
/// chain as the qemu emission); the env overrides still beat the scaled
/// defaults.  Still order-never-timings: these are wall-clock
/// allowances for firmware+loader, not assertions on the guest.
const TCG_FIRST_MARKER_MULT: u64 = 45;
const TCG_OVERALL_MULT: u64 = 6;

/// The q35 machine-info fact line prefix (13-02, X64-01's kernel half):
/// the main9-x64 payload prints one `q35: memory ...` line per mb2
/// memory-map range it parsed.  The gate asserts >= 1 such line strictly
/// after the banner's byte offset — the ordered-presence shape the
/// virtlowecam profile uses for its ecam marker.  Presence, never
/// timings; the negative panic scan above stays on.
const BOOT_Q35_MEMORY_MARKER: &str = "q35: memory";

/// The q35 discovery markers (13-04, X64-03): after the memory facts,
/// print_machine_info walks the ACPI tables and pci.rs programs the MCH
/// PCIEXBAR from the checksummed MCFG before the recursive bridge walk.
/// The gate asserts the MCFG parse line, the pciexbar old->new audit
/// line, and the virtio-net device-found line, strictly after the facts
/// stage — presence, never timings; the negative panic scan stays on.
/// The exact expected BASES are keyed off the config NAME (Task 3's
/// moved-window proof), exactly like the aarch64 ecam markers.
const Q35_MCFG_MARKER: &str = "q35: mcfg";
const Q35_PCIEXBAR_MARKER: &str = "pciexbar: ";

/// The q35net bind + trigger markers (13-05, X64-04): the negotiation
/// audit pair and the devether `#l0:` bind line (the driver half), then
/// the kernel-side netgate's stage markers and the terminal — asserted
/// IN ORDER past the discovery stage's anchor; the pcap assertion at
/// finish() proves the frames host-side.  `netgate: FAIL` (the kernel's
/// fail-closed reconcile) fails the stage loop fast from any stage.
const Q35_BIND_FEATURES_MARKER: &str = "virtio-net: features offered";
const Q35_BIND_L0_MARKER: &str = "#l0: virtio:";
const Q35_NETGATE_PINGPONG_MARKER: &str = "netgate: ping-pong done";
const Q35_NETGATE_TXBULK_MARKER: &str = "netgate: tx-bulk done";
const Q35_NET_DONE_MARKER: &str = "X64-NET-DONE";
const Q35_NETGATE_FAIL_MARKER: &str = "netgate: FAIL";

/// The q35exec staged-exec markers (14-05, XEXEC-02's leg-1 gate):
/// after the Phase-13 discovery stages, the kernel's module/paqfs notes
/// (the mb2 tag-3 module found + the paqfs root mounted through
/// port::devroot::mount_initrd), the exec-entry note (the handoff
/// print, before the first ring-3 instruction), then the MARKER
/// BINARY'S OWN WRITE-syscall output — the notify handler's got-note
/// line and the interrupted marker, both produced from ring 3 through
/// the real EL/SYSCALL path (D-09: the gate's proof is the binary's
/// own prints, never a kernel print on its behalf) — and the all-exited
/// park marker.  The binary is smokesleep staged at /amd64/init: it
/// exercises the ABI's note-delivery convention (notify handler
/// receiving its Ureg* through the bp path, alarm/sleep interplay,
/// noted(NCONT)) — D-08's staged leg 1.
const Q35_MODULE_MARKER: &str = "q35: module";
const Q35_PAQFS_MARKER: &str = "q35: paqfs";
const Q35_EXEC_ENTRY_MARKER: &str = "exec: entering ring 3 pc=0x";
const Q35_SMOKESLEEP_GOT_MARKER: &str = "smokesleep: got note";
// BOOT_SMOKESLEEP_MARKER ("smokesleep: interrupted") carries over as
// the marker's terminal print; BOOT_ALL_EXITED_MARKER as the park.

/// Panic-profile markers: a config with dev = ["panic"] boots
/// virt to a deliberate brk #0 sync exception BEFORE trap::unmask_irq(), so
/// the log is deterministic (zero IRQ interleaving) -- banner, start marker,
/// full TrapFrame register dump, clean-halt marker.  No ticks, no echo:
/// IRQs never unmask in this profile.
const BOOT_PANIC_START_MARKER: &str = "virtpanic: deliberate sync exception";
const BOOT_PANIC_HALT_MARKER: &str = "kernel halted";
/// The TrapFrame {:#?} Debug prints one `    x<N>:` row per register x0..x30.
const BOOT_PANIC_REGISTER_MIN: usize = 30;

/// Exec-profile markers: a config with dev = ["exec"]
/// boots virt into main9's exec orchestration — devroot mounts the initrd
/// bootfs.paq, /init is exec'd, enters EL0, prints via the real pwrite
/// syscall path, execs /hello (the committed ABI-bisect binary),
/// hello prints and exits, and the kernel prints the all-exited marker and
/// parks.  The markers arrive in exactly this order.
const BOOT_EXEC_ENTRY_MARKER: &str = "exec: entering EL0 pc=0x";
const BOOT_INIT_MARKER: &str = "init: r9 EL0 alive";
const BOOT_HELLO_MARKER: &str = "hello aarch64 harvey";
const BOOT_ALL_EXITED_MARKER: &str = "all user processes exited";

/// Multi-profile markers: the virtmulti config boots the SAME kernel+initrd
/// as virtexec, but the staged init rforks two children (hello +
/// smokefork) and awaits both, so the smoke markers arrive in
/// nondeterministic order — asserted for PRESENCE, never interleaving.
/// `sched: switch` is the tick-preemption marker: at least one
/// switch proves IRQ-from-EL0 returned to a DIFFERENT process.
const BOOT_SMOKE_FD_MARKER: &str = "smokefork: fd ok";
const BOOT_SMOKE_CHILD_MARKER: &str = "smokefork: child running";
const BOOT_SMOKE_AWAIT_MARKER: &str = "smokefork: await:";
const BOOT_SMOKE_DONE_MARKER: &str = "init: smoke complete";
const BOOT_SCHED_SWITCH_MARKER: &str = "sched: switch";

/// Notes-profile additions: the virtnotes
/// config boots the SAME kernel+initrd as virtexec/virtmulti, but the
/// staged init also rforks a third child (smokenote), posts it a note via
/// unmodified libc postnote (a /proc/N/note write on devproc-lite), and
/// the kernel delivers at the child's return-to-EL0 boundary — the
/// handler prints the note and resumes via noted(NCONT).  Presence, never
/// interleaving order.
const BOOT_NOTE_GOT_MARKER: &str = "smokenote: got note";
const BOOT_NOTE_RESUMED_MARKER: &str = "smokenote: resumed";

/// Stack-smoke marker: the staged init's
/// fourth child (stacksmoke) recurses depth 64 with ~256-byte frames,
/// forcing multiple demand-paged stack page-ins beyond the two eager seed
/// pages, and prints this on unwind through the real pwrite path.  The
/// virtexec profile's marker set gains it — presence, never order;
/// the multi/notes assertion sets are unchanged (the marker rides their
/// initrd too, but they don't assert it).
const BOOT_STACKSMOKE_MARKER: &str = "stacksmoke: depth 64 ok";

/// Sleep-smoke marker: the staged
/// init's fifth child (smokesleep) arms alarm(300), sleeps 30 s, and the
/// alarm note interrupts the sleep — sys_sleep wakes early, fails -1
/// "interrupted" (the donor tsleep shape), the handler runs at the
/// return-to-EL0 boundary, and main prints this.  Asserted at the
/// AllExited stage for ALL THREE exec-family profiles (Exec/Multi/Notes
/// share the initrd; all-exited arrives only after init reaped every
/// child, so absence is definitive — the stacksmoke check's placement
/// shape, presence-only).
const BOOT_SMOKESLEEP_MARKER: &str = "smokesleep: interrupted";

/// PCI discovery contract (07-02, NETDRV-01): on every aarch64 virt boot
/// the kernel prints the DTB-decoded ECAM window, the found virtio-net
/// device, and the decoded INTx route.  finish() asserts presence, with
/// the expected ECAM base keyed off the config NAME — virtlowecam boots
/// the moved-window machine (`highmem-ecam=off`), so its DTB decodes to a
/// DIFFERENT base; that divergence is the perturbation proof.  The hex
/// constants live here, in test expectations — never in kernel code.
const PCI_NET_DEVICE_MARKER: &str = "pci: virtio-net 1af4:1041";
const PCI_NET_ROUTE_MARKER: &str = "pci: inta -> spi 4 intid 36";
const PCI_ECAM_BASE_STANDARD: &str = "pci: ecam base 0x4010000000";
const PCI_ECAM_BASE_LOWECAM: &str = "pci: ecam base 0x3f000000";

/// The q35 discovery contract (13-04, X64-03): every q35 boot prints the
/// MCFG-parsed window, the pciexbar old->new audit line, the found
/// device, and the derived route.  The expected BASES are keyed off the
/// config NAME — q35lowmcfg boots the moved-MCFG machine
/// (`-global q35-pcihost.MCFG=0xe0000000`), where the kernel must
/// follow the table: the MCFG parse print shows the moved base and the
/// audit line shows SeaBIOS's hardcoded old value (0xb0000001) moving
/// to it.  A single constant serving both configs would prove nothing —
/// the virtlowecam ecm pattern, re-keyed.  The device and route markers
/// are shared: discovery must not degrade under the move.
const Q35_MCFG_BASE_DEFAULT: &str = "q35: mcfg 0xb0000000";
const Q35_MCFG_BASE_MOVED: &str = "q35: mcfg 0xe0000000";
const Q35_PCIEXBAR_NEW_DEFAULT: &str = "-> 0xb0000001";
const Q35_PCIEXBAR_NEW_MOVED: &str = "-> 0xe0000001";
const Q35_NET_ROUTE_MARKER: &str = "pci: inta -> gsi 22";

/// Net profile (virtnet, 07-05): the config boots the REAL Harvey boot
/// chain (dev = ["exec", "bootchain", "ether"], the Harvey-baked
/// bootfs.paq via the 07-01 harvey: initrd channel) to the rc `% `
/// prompt; the harness then types the one-line sweep invocation and the
/// sweep's markers come back over the socket serial.  The boot-chain
/// staging matches the packaged smoke's (build/aarch64/smoke-r9.expect):
/// banner -> paqfs -> stacksmoke -> the literal "% ".  The sweep's
/// markers are printed by /lib/net-sweep.rc (07-04): a FAIL line names
/// its stage and aborts the run immediately (never a 300s hang);
/// NET-DONE gates completion.
const BOOT_PAQFS_MARKER: &str = "paqfs...";
const NET_PROMPT_MARKER: &str = "% ";
const NET_SWEEP_DONE_MARKER: &str = "netsweep: NET-DONE";
const NET_SWEEP_FAIL_MARKER: &str = "netsweep: FAIL";

/// NetIP profile (virtip, 08-04): the config boots the REAL Harvey boot
/// chain (dev = ["exec", "bootchain", "ether", "ip"] — the devip `#I`
/// device is live) to the rc `% ` prompt; the harness then types the
/// loopback sweep invocation and the sweep's `netip:` markers come back
/// over the socket serial.  The devip kernel markers assert the wiring:
/// the `#I0` attach print and the medium-bound print.  The sweep's
/// markers are printed by /lib/netip-loopback.rc: a FAIL line names its
/// stage and aborts the run immediately (never a budget hang); the
/// ping/udp legs' ok lines and NETIP-DONE gate completion.
const DEVIP_ATTACH_MARKER: &str = "#I0: devip attached";
const DEVIP_MEDIUM_MARKER: &str = "devip: medium loopback bound";
const NETIP_SWEEP_DONE_MARKER: &str = "netip: NETIP-DONE";
const NETIP_SWEEP_FAIL_MARKER: &str = "netip: FAIL";
const NETIP_PING_OK_MARKER: &str = "netip: ping ok";
const NETIP_UDP_OK_MARKER: &str = "netip: udp echo ok";
/// The 08-05 tcp mode's leg markers (the tcp/7 echo + the P-M
/// stats-honesty assertion), printed by /lib/netip-tcp.rc.
const NETIP_TCP_OK_MARKER: &str = "netip: tcp echo ok";
const NETIP_TCP_STATS_OK_MARKER: &str = "netip: tcp stats ok";
/// The D-13 netlog-activity marker: the ctl acceptance print for the
/// fixture's `set tcp udp icmp` write (clean gate traffic legitimately
/// produces no netlog packet lines — the marker proves the path live).
const NETIP_NETLOG_MARKER: &str = "netlog: flags set";
/// The 08-06 static mode's markers: the ether medium's bind (the device
/// is #l0's published NIC), the fixture's leg markers, and the D-10 venue
/// label (slirp-green is never L2-truth — the label rides the gate log).
const DEVIP_MEDIUM_ETHER_MARKER: &str = "devip: medium ether bound #l0";
const NETIP_STATIC_OK_MARKER: &str = "netip: static ok";
const NETIP_ARP_OK_MARKER: &str = "netip: arp view ok";
const NETIP_NOT_L2_MARKER: &str = "netip: NOT-L2-TRUTH slirp venue";
/// The 08-07 dhcp mode's marker (NETIP-05), printed by
/// /lib/netip-dhcp.rc after the real ipconfig's DORA configures the guest.
const NETIP_DHCP_OK_MARKER: &str = "netip: dhcp ok";
/// The 08-07 v6 mode's marker (NETIP-06), printed by /lib/netip-v6.rc
/// after the unmodified ipconfig's static v6 configuration + ping -6 leg.
const NETIP_V6_OK_MARKER: &str = "netip: v6 ok";
/// The 08-08 aan mode's markers (NETIP-07, D-14), printed by
/// /lib/netip-aan.rc: the two round-trip markers (the remote rc's
/// responses echoed to the console — BOTH are asserted because the
/// resume-after-drop is the whole point), the ok marker after the
/// scripted drop + resume completes, and the forbidden client exit
/// string (aan's "connection refused" → threadexitsall — the listener
/// surviving the drop is the gate's design).
const NETIP_AAN_RT1_MARKER: &str = "netip: aan roundtrip 1";
const NETIP_AAN_RT2_MARKER: &str = "netip: aan roundtrip 2";
const NETIP_AAN_OK_MARKER: &str = "netip: aan ok";
/// The server-side drop block's confirmation (it writes `hangup` to the
/// underlying conv's ctl and prints this to the console) — asserted so
/// a silent no-op drop can't masquerade as a resume.
const NETIP_AAN_DROP_MARKER: &str = "netip: aan drop ok";
const NETIP_AAN_DIED_LINE: &str = "server died";
/// The 08-08 soak mode's markers (NETIP-08 machinery, D-15), printed by
/// /lib/netip-soak.rc: the tcpblast payload's own completion line plus
/// the fixture's ok marker and the D-10/D-15 venue label.
const NETIP_SOAK_OK_MARKER: &str = "netip: soak ok";
const TCPBLAST_DONE_MARKER: &str = "tcpblast: done";
/// The probe-pinned slirp ND verdict (08-07's Task-2 probe — the gate
/// asserts the NA reply iff the probe observed slirp answering NS for
/// its v6 gateway address; the SUMMARY records the probe).  Finalized
/// from the recorded probe, never assumed.
const NETIP_V6_SLIRP_NA: bool = true;

/// Always-on negative scan: any panic/Unhandled-interrupt line in
/// the capture fails the run — every profile EXCEPT the deliberate-panic
/// one (whose expected flow prints exactly that dump).  Case-insensitive,
/// matching `grep -ciE 'panic|Unhandled interrupt'`.  The
/// notes-era failures join: `note-timeout` (a note was never
/// delivered) and `unimplemented` (a syscall outside the implemented set
/// was called — a traced-set miss surfaces here; trace
/// and implement the gap, never silence it).  The stray-IRQ line's
/// lowercase shape joins as a hit pattern (Pitfall N10): once 07-06 arms
/// the NIC's IRQ source, any `unhandled IRQ <n>` line is a bind-ordering
/// bug, not a cosmetic — none of the pre-net profiles emit it today.
///
/// Profile-aware: the Net profile rides the REAL Harvey boot chain, whose
/// boot(8) auth handshake calls fauth — syscall 10 is loud-None by design
/// (D-02), printing `syscall 10 unimplemented` on every real-chain boot.
/// For the net profile exactly that ONE line is exempt; any OTHER
/// unimplemented-syscall line (a traced-set miss) still fails the run,
/// and every other pattern applies unchanged.
fn negative_scan_hit_for(capture: &[u8], net_profile: bool) -> Option<&'static str> {
    let lower: Vec<u8> = capture.iter().map(|b| b.to_ascii_lowercase()).collect();
    if contains_marker(&lower, b"panic") {
        Some("panic")
    } else if contains_marker(&lower, b"unhandled interrupt") {
        Some("Unhandled interrupt")
    } else if contains_marker(&lower, b"unhandled irq") {
        Some("unhandled IRQ")
    } else if contains_marker(&lower, b"note-timeout") {
        Some("note-timeout")
    } else if unimplemented_hit(&lower, net_profile) {
        Some("unimplemented syscall")
    } else {
        None
    }
}

/// The `unimplemented` arm, line-wise so the Net profile can exempt the
/// D-02 designed fauth line while still failing any other
/// unimplemented-syscall hit.
fn unimplemented_hit(lower: &[u8], net_profile: bool) -> bool {
    if !contains_marker(lower, b"unimplemented") {
        return false;
    }
    if !net_profile {
        return true;
    }
    lower.split(|b| *b == b'\n').any(|line| {
        contains_marker(line, b"unimplemented") && !contains_marker(line, b"syscall 10 unimplemented")
    })
}

/// The first capture line containing `needle`, CRLF-trimmed — the sweep's
/// fast-fail names the sweep's own stage (`netsweep: FAIL bind`, etc.),
/// never a bare marker name.
fn line_containing(capture: &[u8], needle: &[u8]) -> Option<String> {
    capture
        .split(|b| *b == b'\n')
        .find(|line| line.windows(needle.len()).any(|w| w == needle))
        .map(|line| String::from_utf8_lossy(line).trim_end_matches('\r').to_string())
}

/// The net sweep's mode: R9_NET_SWEEP_MODE selects `arp` (the ping-pong
/// gate, default) or `blast` (the TX-bulk gate).  A typo'd value is a
/// harness bug, not a gate failure — validated before qemu spawns, never
/// after a wasted boot.
fn net_sweep_mode() -> Result<String> {
    let mode = env::var("R9_NET_SWEEP_MODE").unwrap_or_else(|_| "arp".to_string());
    match mode.as_str() {
        "arp" | "blast" => Ok(mode),
        _ => Err(format!("R9_NET_SWEEP_MODE must be \"arp\" or \"blast\", got {mode:?}").into()),
    }
}

/// The netip sweep's mode: R9_NETIP_SWEEP selects the leg set —
/// `loopback` (the 08-04 ping + udp echo legs), `tcp` (the 08-05 tcp/7
/// echo + stats-honesty legs), `static` (the 08-06 NETIP-04 static
/// ipconfig gate on the ether leg, on slirp — labeled not-L2-truth),
/// `dhcp` (the 08-07 NETIP-05 gate: the REAL ipconfig dhcpclient's DORA
/// over /net/udp broadcast fidelity, same venue label), `v6` (the
/// 08-07 NETIP-06 gate: the devip ip6 surface — icmpv6 full-packet bridge
/// + ipifc add6/ra6/try — static v6 config via unmodified ipconfig, the
/// NS/NA exchange + the guest's ICMPv6 echo leg pcap-proven), or `aan`
/// (the 08-08 NETIP-07 gate: the aan drop-and-resume over tcp loopback —
/// D-14, NIC-free), or `soak` (the 08-08 NETIP-08 machinery gate: the
/// tcpblast bidirectional soak against the host echo endpoint on slirp,
/// pcap + guest-counter reconciled, labeled not-L2-truth).
/// A typo'd value is a harness bug, not a gate failure — validated before
/// qemu spawns, never after a wasted boot.
fn netip_sweep_mode() -> Result<String> {
    let mode = env::var("R9_NETIP_SWEEP").unwrap_or_else(|_| "loopback".to_string());
    match mode.as_str() {
        "loopback" | "tcp" | "static" | "dhcp" | "v6" | "aan" | "soak" => Ok(mode),
        _ => Err(
            format!("R9_NETIP_SWEEP must be \"loopback\", \"tcp\", \"static\", \"dhcp\", \"v6\", \"aan\", or \"soak\", got {mode:?}")
                .into(),
        ),
    }
}

/// The soak gate's round count: R9_NETIP_SOAK_ROUNDS overrides the
/// TCG-scaled gate default (64 — tcpblast's own default; the sustained
/// dev-host soak per D-16 uses a named larger count, documented verbatim
/// in the fixture's header).  Digits only, validated pre-spawn (a typo
/// is a harness bug, never a wasted boot).
fn soak_rounds() -> Result<String> {
    let raw = env::var("R9_NETIP_SOAK_ROUNDS").unwrap_or_else(|_| "64".to_string());
    if raw.is_empty() || raw.len() > 6 || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "R9_NETIP_SOAK_ROUNDS must be 1-6 digits, got {raw:?}"
        )
        .into());
    }
    Ok(raw)
}

/// Count lines whose text starts with `marker`.  Serial lines arrive CRLF
/// (the console's putb maps \n to \r\n), so splitting on \n leaves the \r
/// trailing each line, where starts_with never sees it.
fn count_marker_lines(capture: &[u8], marker: &str) -> usize {
    capture.split(|b| *b == b'\n').filter(|line| line.starts_with(marker.as_bytes())).count()
}

fn contains_marker(capture: &[u8], marker: &[u8]) -> bool {
    capture.windows(marker.len()).any(|w| w == marker)
}

/// The first byte offset at which `marker` occurs (the ordered-presence
/// anchor: a later stage counts its own markers only past this point,
/// so a marker that rode the wire BEFORE the anchor never satisfies it).
fn marker_position(capture: &[u8], marker: &[u8]) -> Option<usize> {
    capture.windows(marker.len()).position(|w| w == marker)
}

/// The byte offset just past the LAST line starting with `marker`
/// (including its terminator) — a POSITIONAL anchor.  The X64 discovery
/// stage (13-04) anchors here, not at capture length: the kernel prints
/// the whole facts+discovery block in one serial burst, so a
/// satisfaction-time anchor would skip past the discovery lines that
/// arrived in the same socket read.
fn last_marker_line_end(capture: &[u8], marker: &str) -> Option<usize> {
    let mut off = 0usize;
    let mut found = None;
    for line in capture.split_inclusive(|b| *b == b'\n') {
        off += line.len();
        if line.starts_with(marker.as_bytes()) {
            found = Some(off);
        }
    }
    found
}

/// Count TrapFrame register-dump rows: lines shaped `<spaces>x<N>:` (the
/// {:#?} Debug prints x0..x30 one per line, space-indented, bare register
/// names).  Serial lines arrive CRLF; the digit/colon scan never reaches
/// the trailing \r.
fn count_register_lines(capture: &[u8]) -> usize {
    capture
        .split(|b| *b == b'\n')
        .filter(|line| {
            let mut i = 0;
            while i < line.len() && line[i] == b' ' {
                i += 1;
            }
            // Require the Debug indent (>= 1 space), then `x`, then >= 1
            // digit, then `:` immediately -- the annotated name shape
            // ("x29 (frame_pointer):") deliberately does not match.
            if i == 0 || i >= line.len() || line[i] != b'x' {
                return false;
            }
            i += 1;
            let digits = i;
            while i < line.len() && line[i].is_ascii_digit() {
                i += 1;
            }
            i > digits && i < line.len() && line[i] == b':'
        })
        .count()
}

/// Which ordered marker the harness is currently waiting for.
#[derive(Debug, PartialEq)]
enum BootStage {
    Banner,
    Ticks,
    Echo,
    PanicStart,
    PanicDump,
    PanicHalt,
    ExecEntry,
    InitPrint,
    HelloPrint,
    /// The multi profile: collect the (unordered) smoke markers +
    /// >= 1 sched-switch preemption line before awaiting all-exited.
    MultiMarkers,
    AllExited,
    /// The net profile: after the banner, wait for the real Harvey boot
    /// chain's markers (paqfs + stacksmoke), then the rc prompt.
    NetBoot,
    NetPrompt,
    /// The sweep line has been typed; await the sweep's markers (fast-fail
    /// on a `netsweep: FAIL` line, complete on `netsweep: NET-DONE`).
    NetSweep,
    /// The x86_64 facts stage (13-02): banner observed, now await >= 1
    /// `q35: memory` fact line strictly after the banner's byte offset —
    /// the mb2 parse's on-serial evidence.
    Q35Facts,
    /// The x86_64 discovery stage (13-04): facts observed, now await the
    /// ACPI MCFG parse line, the pciexbar audit line, and the virtio-net
    /// device-found line, all strictly after the facts stage's anchor —
    /// X64-03's on-serial evidence.
    Q35PciDiscovery,
    /// The q35net bind stage (13-05): discovery observed, now await the
    /// virtio-net negotiation audit pair and the devether `#l0:` bind
    /// line — X64-04's driver-half evidence, strictly after the
    /// discovery stage's anchor.
    X64Bind,
    /// The q35net trigger stages (13-05): the kernel-side netgate's
    /// `ping-pong done` then `tx-bulk done` then the terminal
    /// X64-NET-DONE marker, in order (a `netgate: FAIL` line fails fast
    /// from any of them).
    X64PingPong,
    X64TxBulk,
    X64NetDone,
    /// The q35exec module stage (14-05): discovery observed, now await
    /// the kernel's module-found and paqfs-mount notes (the mb2 tag-3
    /// delivery + the devroot mount), strictly after the discovery
    /// stage's anchor.
    Q35ModulePaqfs,
    /// The q35exec entry stage: paqfs mounted, now await the exec-entry
    /// note — the handoff print naming the seeded pc/sp, before the
    /// first ring-3 instruction runs.
    Q35ExecEntry,
    /// The q35exec marker stage: the staged binary's OWN prints from
    /// ring 3 — the notify handler's got-note line, then the
    /// interrupted marker (WRITE syscalls through the real EL/SYSCALL
    /// path, D-09's point).
    Q35MarkerPrints,
}

impl fmt::Display for BootStage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BootStage::Banner => write!(f, "banner marker {BOOT_BANNER_MARKER:?}"),
            BootStage::Ticks => {
                write!(f, ">= {BOOT_TICK_MIN} lines beginning {BOOT_TICK_MARKER:?}")
            }
            BootStage::Echo => {
                write!(f, "echo of probe {:?}", String::from_utf8_lossy(BOOT_ECHO_PROBE))
            }
            BootStage::PanicStart => {
                write!(f, "panic start marker {BOOT_PANIC_START_MARKER:?}")
            }
            BootStage::PanicDump => {
                write!(f, ">= {BOOT_PANIC_REGISTER_MIN} TrapFrame register rows")
            }
            BootStage::PanicHalt => {
                write!(f, "clean-halt marker {BOOT_PANIC_HALT_MARKER:?}")
            }
            BootStage::ExecEntry => {
                write!(f, "exec-entry marker {BOOT_EXEC_ENTRY_MARKER:?}")
            }
            BootStage::InitPrint => write!(f, "init print marker {BOOT_INIT_MARKER:?}"),
            BootStage::HelloPrint => write!(f, "hello print marker {BOOT_HELLO_MARKER:?}"),
            BootStage::MultiMarkers => write!(
                f,
                "smoke markers {{{BOOT_SMOKE_FD_MARKER:?}, {BOOT_SMOKE_CHILD_MARKER:?}, \
                 {BOOT_SMOKE_AWAIT_MARKER:?}, {BOOT_HELLO_MARKER:?}, {BOOT_SMOKE_DONE_MARKER:?}}} \
                 (+ {{{BOOT_NOTE_GOT_MARKER:?}, {BOOT_NOTE_RESUMED_MARKER:?}}} on the notes \
                 profile) + >= 1 {BOOT_SCHED_SWITCH_MARKER:?} line"
            ),
            BootStage::AllExited => {
                write!(f, "all-exited marker {BOOT_ALL_EXITED_MARKER:?}")
            }
            BootStage::NetBoot => write!(
                f,
                "boot-chain markers {{{BOOT_PAQFS_MARKER:?}, {BOOT_STACKSMOKE_MARKER:?}}}"
            ),
            BootStage::NetPrompt => write!(f, "rc prompt {NET_PROMPT_MARKER:?}"),
            BootStage::NetSweep => write!(
                f,
                "sweep markers (fast-fail {NET_SWEEP_FAIL_MARKER:?}, completion {NET_SWEEP_DONE_MARKER:?})"
            ),
            BootStage::Q35Facts => {
                write!(f, ">= 1 line beginning {BOOT_Q35_MEMORY_MARKER:?} after the banner")
            }
            BootStage::Q35PciDiscovery => write!(
                f,
                "discovery markers {{{Q35_MCFG_MARKER:?}, {Q35_PCIEXBAR_MARKER:?}, {PCI_NET_DEVICE_MARKER:?}}} after the facts"
            ),
            BootStage::X64Bind => write!(
                f,
                "bind markers {{{Q35_BIND_FEATURES_MARKER:?}, {Q35_BIND_L0_MARKER:?}}} after the discovery"
            ),
            BootStage::X64PingPong => {
                write!(f, "netgate marker {Q35_NETGATE_PINGPONG_MARKER:?} after the bind")
            }
            BootStage::X64TxBulk => write!(f, "netgate marker {Q35_NETGATE_TXBULK_MARKER:?}"),
            BootStage::X64NetDone => write!(f, "terminal marker {Q35_NET_DONE_MARKER:?}"),
            BootStage::Q35ModulePaqfs => write!(
                f,
                "module/paqfs notes {{{Q35_MODULE_MARKER:?}, {Q35_PAQFS_MARKER:?}}} after the discovery"
            ),
            BootStage::Q35ExecEntry => {
                write!(f, "exec-entry note {Q35_EXEC_ENTRY_MARKER:?} after the paqfs mount")
            }
            BootStage::Q35MarkerPrints => write!(
                f,
                "the marker binary's own prints {{{Q35_SMOKESLEEP_GOT_MARKER:?}, \
                 {BOOT_SMOKESLEEP_MARKER:?}}} from ring 3"
            ),
        }
    }
}

/// The boot-test assertion profile being run.
#[derive(Clone, Copy, PartialEq)]
enum BootProfile {
    Ticks,
    Panic,
    Exec,
    Multi,
    Notes,
    /// The virtnet net gate: the real Harvey boot chain to the rc prompt,
    /// the typed sweep invocation, and the finish()-time pcap assertion.
    Net,
    /// The virtip netip gate (08-04): the real Harvey boot chain with the
    /// devip `#I` device live, the typed loopback sweep, and the
    /// finish()-time marker assertions (devip attach/bound markers, the
    /// sweep's ping/udp ok lines, NETIP-DONE).  No pcap assertion —
    /// loopback traffic never touches the wire (that is the point).
    Netip,
    /// The x86_64 channel gate (13-01 opened it; 13-02 grew it to the
    /// kernel half; 13-03 completed it to the timer half): SeaBIOS ->
    /// GRUB2 -> mb2 kernel, banner marker, >= 1 `q35: memory` mb2-fact
    /// line strictly after the banner, >= 3 tick lines strictly after
    /// the facts, then the typed echo probe reappearing — X64-01's full
    /// Phase-3-style platform-gate shape, order-anchored at every stage.
    X64,
    /// The q35net net gate (13-05, X64-04 + X64-02's load proof): the
    /// X64 discovery stages, then the bind markers (negotiation audit,
    /// `#l0:`), the kernel-side netgate's ping-pong/tx-bulk stage
    /// markers, and the terminal X64-NET-DONE — with the finish()-time
    /// pcap assertion over this run's filter-dump capture (>= N
    /// requests parsed-from-the-marker, >= 1 gateway reply, 60-byte
    /// frames sourced from the guest MAC).
    X64Net,
    /// The q35exec staged-exec gate (14-05, XEXEC-02's leg 1): the X64
    /// discovery stages, then the kernel's module/paqfs notes, the
    /// exec-entry note, the marker binary's OWN WRITE-syscall prints
    /// from ring 3 (D-09), and the all-exited park.
    X64Exec,
}

/// Teardown: SIGKILL the spawned qemu's process GROUP (qemu spawned
/// group-leader via `process_group(0)`, so pgid == pid), then reap the
/// leader.  std has no killpg, so the group signal rides kill(1) — POSIX
/// base, not a new host dependency; `-KILL -- -<pgid>` with the `--`
/// guard parses on BSD (darwin) and GNU (linux) alike.  A group-kill
/// failure falls back to the plain child kill so the leader never
/// survives; `wait` reaps the zombie either way.
fn kill_process_group(child: &mut process::Child) {
    let pgid = child.id();
    let group_killed = Command::new("kill")
        .arg("-KILL")
        .arg("--")
        .arg(format!("-{pgid}"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !group_killed {
        let _ = child.kill();
    }
    let _ = child.wait();
}

impl BootTestStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        let arch = Arch::from(matches);
        let config = load_config(arch, matches);
        let config_name = matches
            .try_get_one::<String>("config")
            .ok()
            .flatten()
            .cloned()
            .unwrap_or_else(|| "default".to_string());
        let profile = Profile::from(matches);
        // The override chain, per-invocation layer: the CLI flag beats the
        // R9_ACCEL / R9_CPU environment value (mirrors QemuStep::new); the
        // TOML value and the built-in default layer on at emission in run().
        let accel = matches
            .try_get_one::<String>("accel")
            .ok()
            .flatten()
            .cloned()
            .or_else(|| env::var("R9_ACCEL").ok());
        let cpu = matches
            .try_get_one::<String>("cpu")
            .ok()
            .flatten()
            .cloned()
            .or_else(|| env::var("R9_CPU").ok());
        let verbose = verbose(matches);

        Self { arch, config, config_name, profile, accel, cpu, verbose }
    }

    fn run(self) -> Result<()> {
        if self.arch == Arch::Riscv64 {
            return Err("boot-test currently supports aarch64 and x86_64 only".into());
        }

        // The cross-repo ureg.h drift gate runs before qemu
        // spawns — a drifted (or absent) Harvey checkout fails every
        // boot-test profile on BOTH arches (aarch64 pins
        // aarch64/include/ureg.h, x86_64 pins amd64/include/ureg.h —
        // an ABI mismatch is a build failure, not a runtime mystery).
        // Bare `cargo xtask build` instead tolerates a missing
        // checkout — the const asserts still guard there.
        uregcheck::check_for_boot_test(self.arch)?;

        // The virtnet profile's sweep mode is validated BEFORE qemu
        // spawns — a typo'd R9_NET_SWEEP_MODE is a harness bug, not a gate
        // failure, and a wasted TCG boot proves nothing.
        if self.config_name == "virtnet" {
            net_sweep_mode()?;
        }
        // Same discipline for the virtip profile (R9_NETIP_SWEEP, 08-04).
        if self.config_name == "virtip" {
            let mode = netip_sweep_mode()?;
            // The slirp-leg modes' pcap assertions derive their addresses
            // from the Harvey checkout's netaddr.rc seam (D-11) — resolve
            // it BEFORE qemu spawns, so an absent/unreadable seam is a
            // loud pre-spawn harness error (the 07-01 harvey:-channel
            // precedent), never a wasted boot.
            if mode == "static" || mode == "dhcp" {
                pcap::netaddr_slirp().map_err(|e| format!("netip {mode} gate: {e}"))?;
            }
            if mode == "v6" {
                pcap::netaddr_slirp_v6().map_err(|e| format!("netip v6 gate: {e}"))?;
            }
            if mode == "soak" {
                // 08-08 (NETIP-08 machinery): the soak needs the v4 seam
                // (the dial target) + the blastport (the host endpoint's
                // bind) + the round count — all validated pre-spawn.
                pcap::netaddr_slirp().map_err(|e| format!("netip soak gate: {e}"))?;
                pcap::netaddr_blastport().map_err(|e| format!("netip soak gate: {e}"))?;
                soak_rounds()?;
            }
        }

        let target = self.arch.target();
        let dir = self.profile.dir();
        // On virt the cargo-built ELF is the -kernel directly; on q35 the
        // freshly built ELF feeds the grub-mkrescue ISO (D-02's -cdrom
        // channel — never a -kernel load).
        let kernel = match self.arch {
            Arch::Aarch64 => format!("target/{target}/{dir}/aarch64"),
            Arch::X86_64 => format!("target/{target}/{dir}/x86_64"),
            Arch::Riscv64 => unreachable!("riscv64 rejected at the arch gate"),
        };
        let kernel_path = workspace().join(&kernel);
        if !kernel_path.exists() {
            return Err(format!("kernel image {kernel} missing (build step ran first?)").into());
        }
        // The ISO is built from THIS invocation's kernel build (evidence
        // freshness — the same discipline as the aarch64 -kernel ELF)
        // and carries the mb2 header check inside: the channel refuses
        // to boot an ISO around a kernel GRUB would silently reject.
        let iso_rel = if self.arch == Arch::X86_64 {
            Some(build_x86_64_iso(&kernel, &self.config_name, &self.config, self.verbose)?)
        } else {
            None
        };

        // Resolve accel/cpu + the effective config up front: virt's chain is
        // --accel/--cpu flag > R9_ACCEL/R9_CPU env > TOML > tcg/cortex-a72,
        // with hvf swapping in the gicv3 machine/DTB and -cpu host
        // (virt_resolved); raspi keeps the emit-only-when-set shape.
        let platform = self.config.config.as_ref().and_then(|c| c.platform.as_deref());
        let (config, overrides) = if platform == Some("virt") {
            virt_resolved(&self.config, &self.accel, &self.cpu)
        } else {
            (self.config.clone(), QemuOverrides { accel: self.accel.clone(), cpu: self.cpu.clone() })
        };

        // Build-if-missing/stale for target/-resident initrd images
        // (same hook as QemuStep).
        ensure_initrd_image(&config, self.verbose)?;
        // If the config carries an initrd, patch a target/ copy of the
        // DTB with /chosen linux,initrd-start/end (same apply path as QemuStep).
        prepare_initrd_dtb(&config, workspace().to_str().unwrap())?;

        let sock_rel = format!("target/boot-test-{}.sock", self.config_name);
        let log_rel = format!("target/boot-test-{}.log", self.config_name);
        let pcap_rel = format!("target/boot-test-{}.pcap", self.config_name);
        let sock_abs = workspace().join(&sock_rel);
        let log_abs = workspace().join(&log_rel);
        let pcap_abs = workspace().join(&pcap_rel);
        // A stale socket file from a crashed run would break qemu's bind.
        let _ = std::fs::remove_file(&sock_abs);
        // Destroy
        // any PRIOR run's serial log BEFORE qemu spawns, so this run's
        // marker waits can only ever match fresh output (a rerun that
        // matched a pre-seeded log once killed the fresh qemu).  The
        // capture re-creates the log at File::create below; a spawn/connect
        // failure now leaves NO log rather than a stale one.
        let _ = std::fs::remove_file(&log_abs);
        // The same stale-evidence discipline for the pcap capture: a later
        // gate asserts against THIS run's frames, never a previous run's
        // (the NETDRV-03 transparency prohibition — qemu's filter-dump
        // truncates at open, but the removal keeps the no-dead-evidence
        // invariant independent of qemu internals).
        let _ = std::fs::remove_file(&pcap_abs);

        // 08-08 (NETIP-08 machinery): the soak mode's host echo endpoint
        // spawns BEFORE qemu — the guest never dials a closed port; it
        // joins after the run (one teardown site covers pass/fail/timeout).
        let host_echo = if self.config_name == "virtip" && netip_sweep_mode()? == "soak" {
            let port = pcap::netaddr_blastport().map_err(|e| format!("netip soak gate: {e}"))?;
            Some(HostEcho::spawn(port)?)
        } else {
            None
        };

        // Spawn qemu with the serial console on the socket chardev.  The
        // machine/cpu/smp/memory/accel/dtb plumbing is the resolved config's,
        // via the same apply path QemuStep uses.
        let mut cmd = Command::new(self.arch.qemu_system());
        apply_to_qemu_step_with_overrides(&mut cmd, &config, &overrides);
        // D-09/D-10: every aarch64 virt boot carries the idle virtio-net NIC
        // + a fresh per-run pcap capture of its netdev (the gate evidence
        // channel; no kernel code probes the device this plan).
        apply_virt_netdev(&mut cmd, &config, &pcap_rel);
        // D-01a: the q35 leg's idle NIC + pcap ride the same emission
        // point, from the [qemu] netdev table (data — the generalized
        // q35 emitter; a no-op for every aarch64 config).
        apply_netdev(&mut cmd, &config, &pcap_rel);
        cmd.arg("-display");
        cmd.arg("none");
        if self.arch == Arch::X86_64 {
            // A wedged channel (bad entry, triple fault) must not ride a
            // SeaBIOS reboot loop through the whole budget — exit loudly
            // on the first reset instead (Pitfall 8).
            cmd.arg("-no-reboot");
        }
        cmd.arg("-chardev");
        cmd.arg(format!("socket,id=cons,path={sock_rel},server=on,wait=on"));
        cmd.arg("-serial");
        cmd.arg("chardev:cons");
        if let Some(iso) = &iso_rel {
            // D-02: SeaBIOS -> GRUB2 -> mb2 -> kernel from the ISO.
            cmd.arg("-cdrom");
            cmd.arg(iso);
        } else {
            cmd.arg("-kernel");
            cmd.arg(&kernel);
        }
        cmd.current_dir(workspace());
        if self.verbose {
            println!("Executing {cmd:?}");
        }
        // Qemu spawns as its own process-group leader (pgid == its
        // pid) so teardown kills the whole GROUP — no orphaned qemu (or a
        // helper it spawned) survives the pass/fail/timeout exit paths.
        // std's process_group(0) is the setpgid(0,0) pre-exec (darwin has
        // no setsid(1) CLI, but this is the libc call via std) — zero new
        // dependencies.
        let mut child = cmd
            .process_group(0)
            .spawn()
            .map_err(|e| format!("{}: {}", cmd.get_program().to_string_lossy(), e))?;

        // The accel-aware budget axis (13-03): TCG's firmware+loader are
        // tens of times slower than KVM's — the resolved accel (the same
        // chain the qemu emission uses: flag > TOML) scales the gate's
        // wall-clock allowances.  A missing accel key leaves the defaults.
        let effective_accel = overrides
            .accel
            .clone()
            .or_else(|| config.qemu.as_ref().and_then(|q| q.accel.clone()))
            .unwrap_or_default();
        let tcg_budgets = effective_accel == "tcg";

        let result = self.assert_markers(&mut child, &sock_abs, &log_abs, tcg_budgets);

        // Always reap the guest (by process group) and drop the
        // socket file.  One teardown site covers the pass, fail, and
        // timeout paths: assert_markers always returns here.  The soak
        // mode's host echo endpoint stops+joins here too (08-08).
        kill_process_group(&mut child);
        let _ = std::fs::remove_file(&sock_abs);
        if let Some(echo) = host_echo {
            echo.stop_and_join();
        }

        result
    }

    /// Connect, capture, and assert the ordered markers.  Writes every
    /// captured byte to the evidence log at target/boot-test-<config>.log.
    /// `tcg_budgets` scales the wall-clock allowances for TCG's slower
    /// firmware+loader (13-03; the env overrides still win).
    fn assert_markers(
        &self,
        child: &mut process::Child,
        sock_abs: &Path,
        log_abs: &Path,
        tcg_budgets: bool,
    ) -> Result<()> {
        let timeout_secs: u64 = env::var("R9_BOOT_TEST_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or({
                let base = BOOT_TEST_TIMEOUT_SECS;
                if tcg_budgets { base * TCG_OVERALL_MULT } else { base }
            });
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        // The x86_64 channel's dedicated first-marker budget (Pitfall 8):
        // firmware time precedes the banner on the -cdrom path, and this
        // arm's gate is the banner itself.
        let first_marker_deadline = Instant::now()
            + Duration::from_secs(
                env::var("R9_BOOT_TEST_FIRST_MARKER_TIMEOUT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or({
                        let base = BOOT_FIRST_MARKER_TIMEOUT_SECS;
                        if tcg_budgets { base * TCG_FIRST_MARKER_MULT } else { base }
                    }),
            );

        // wait=on means qemu holds guest start until this connect, so the
        // capture covers the entire boot.  Retry while qemu creates the
        // socket, but bail if qemu itself died (bad args etc.).
        let mut stream = loop {
            match UnixStream::connect(sock_abs) {
                Ok(stream) => break stream,
                Err(e) => {
                    if let Some(status) = child.try_wait()? {
                        return Err(format!("qemu exited before serial connect: {status}").into());
                    }
                    if Instant::now() >= deadline {
                        return Err(format!("serial socket never appeared: {e}").into());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        };
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;

        // The FRESH evidence log — the prior run's log was destroyed
        // pre-spawn, so every byte written
        // here is this run's.
        let mut log_file = std::fs::File::create(log_abs)?;
        let mut capture: Vec<u8> = Vec::new();
        let mut stage = BootStage::Banner;
        let mut probe_at = 0usize;
        // The X64 facts stage's ordered-presence anchor: the byte offset
        // just past the banner, set when the banner is observed.
        let mut facts_after = 0usize;
        // The X64 discovery stage's ordered-presence anchor (13-04): the
        // byte offset at the facts stage's satisfaction, so a discovery
        // line that rode the wire earlier never counts.
        let mut pci_after = 0usize;
        // The X64 ticks stage's ordered-presence anchor (13-03): the
        // byte offset just past the discovery stage's satisfaction, so a
        // tick line that rode the wire earlier never counts.  0 on the
        // aarch64 profiles (the whole capture counts, as before).
        let mut ticks_after = 0usize;

        // Assertion profile selection: a [config] dev list containing
        // "panic" selects the panic profile -- banner, the virtpanic start
        // marker, a full TrapFrame register dump and the clean-halt marker,
        // in that order.  IRQs never unmask in that profile, so the
        // ticks/echo stages are never entered.
        let panic_profile = self
            .config
            .config
            .as_ref()
            .and_then(|c| c.dev.as_ref())
            .is_some_and(|devs| devs.iter().any(|d| d == "panic"));
        // A dev list containing "exec" selects the exec
        // profile -- banner, exec entry, the staged init's print, hello's
        // print (the committed bisect binary via the real EL0 svc
        // path), and the all-exited marker, in that order.  No probe is
        // typed: under dev_exec the PL011 RX bytes feed the devcons read
        // buffer instead of echoing.
        let exec_profile = self
            .config
            .config
            .as_ref()
            .and_then(|c| c.dev.as_ref())
            .is_some_and(|devs| devs.iter().any(|d| d == "exec"));
        // The MULTI
        // profile is selected by CONFIG NAME ("virtmulti") — the kernel
        // config is byte-identical to virtexec's (same dev = ["exec"] cfg,
        // same initrd); only the assertion set differs.  Selection by name
        // (rather than a dev-list addition) keeps the kernel cfg surface
        // unchanged: there is no second kernel behavior to gate, only a
        // second set of boot-test expectations over the same boot.
        // The NOTES profile ("virtnotes") is
        // the same shape — same kernel config, the marker set gains the
        // smokenote pair.
        let multi_profile = self.config_name == "virtmulti";
        let notes_profile = self.config_name == "virtnotes";
        // The NET profile ("virtnet", the NETDRV-03/04 gate) is also
        // selected by config name — and its dev list CONTAINS "exec", so
        // the Banner-stage dispatch must check it FIRST (before the
        // exec-profile arm) or the exec flow would mis-select.
        // The NETIP profile ("virtip", 08-04's NETIP-03 loopback gate)
        // is the same shape — selected by name, checked first.
        let net_profile = self.config_name == "virtnet";
        let netip_profile = self.config_name == "virtip";
        // The Q35NET profile ("q35net", 13-05's X64-04 gate) is the same
        // shape — selected by config name; its dev list carries "ether"
        // but no exec (the kernel-side trigger prohibition), so the
        // Banner-stage dispatch must route it at the X64 facts stages
        // and the Q35PciDiscovery satisfaction branches into the bind +
        // netgate stages below.
        let q35net_profile = self.arch == Arch::X86_64 && self.config_name == "q35net";
        // The Q35EXEC profile ("q35exec", 14-05's XEXEC-02 leg-1 gate)
        // is the same name-selected shape: its dev list is ["exec"] and
        // its [dist] module key stages bootfs.paq into the ISO, so the
        // discovery satisfaction branches into the module/paqfs →
        // exec-entry → marker-prints stages below.
        let q35exec_profile = self.arch == Arch::X86_64 && self.config_name == "q35exec";
        // The X64 bind stage's ordered-presence anchor (13-05): the byte
        // offset at the discovery stage's satisfaction — a bind line
        // that rode the wire earlier never counts.
        let mut bind_after = 0usize;

        while Instant::now() < deadline {
            // Always-on negative scan: a panic/Unhandled-interrupt line
            // fails any profile except the deliberate-panic one.  The net
            // profile exempts exactly the D-02 designed fauth line (the
            // real Harvey chain prints `syscall 10 unimplemented` twice a
            // boot); every other pattern — and any other unimplemented
            // syscall — still fails the run.  The netip profile rides the
            // same real boot chain — the same single-line exemption.
            if !panic_profile && let Some(hit) = negative_scan_hit_for(&capture, net_profile || netip_profile) {
                return Err(self.fail(stage, &capture, &format!("negative scan hit {hit:?}")));
            }
            // The q35net fail-fast (13-05): the kernel netgate's own
            // reconciliation is fail-closed — a `netgate: FAIL` line
            // means a stage mismatch the missing terminal marker would
            // otherwise only surface as a slow budget expiry.  Fail the
            // gate the moment the kernel names its stage.
            if q35net_profile && contains_marker(&capture, Q35_NETGATE_FAIL_MARKER.as_bytes()) {
                let tail = String::from_utf8_lossy(&capture[capture.len().saturating_sub(200)..]);
                return Err(self.fail(
                    stage,
                    &capture,
                    &format!("kernel netgate failed ({Q35_NETGATE_FAIL_MARKER}): {tail}"),
                ));
            }
            // The x86_64 channel's dedicated first-marker budget: the
            // banner is the gate's first marker, and SeaBIOS + GRUB
            // precede it on the wire.
            if self.arch == Arch::X86_64
                && stage == BootStage::Banner
                && Instant::now() >= first_marker_deadline
            {
                return Err(self.fail(
                    stage,
                    &capture,
                    "first-marker budget expired (SeaBIOS+GRUB+kernel: check the ISO boot for \
                     loader errors on the captured serial tail)",
                ));
            }
            match stage {
                BootStage::Banner => {
                    if contains_marker(&capture, BOOT_BANNER_MARKER.as_bytes()) {
                        if self.arch == Arch::X86_64 {
                            // The X64-01 channel gate (13-01) grew its
                            // kernel half in 13-02: the banner alone no
                            // longer satisfies the gate — the main9-x64
                            // payload must print the mb2 facts on the same
                            // wire.  Advance to the facts stage with the
                            // anchor set just past the banner, so the
                            // memory lines are asserted strictly after it
                            // (a fact line that rode the wire pre-banner
                            // can never satisfy the stage).
                            facts_after = marker_position(&capture, BOOT_BANNER_MARKER.as_bytes())
                                .map(|p| p + BOOT_BANNER_MARKER.len())
                                .unwrap_or(capture.len());
                            stage = BootStage::Q35Facts;
                        } else {
                            stage = if net_profile || netip_profile {
                                BootStage::NetBoot
                            } else if panic_profile {
                                BootStage::PanicStart
                            } else if exec_profile || multi_profile || notes_profile {
                                BootStage::ExecEntry
                            } else {
                                BootStage::Ticks
                            };
                        }
                    }
                }
                BootStage::Q35Facts => {
                    if count_marker_lines(&capture[facts_after..], BOOT_Q35_MEMORY_MARKER) >= 1 {
                        // 13-04: the gate grew to the discovery half —
                        // X64-03's evidence.  The anchor is POSITIONAL
                        // (just past the last memory fact line): the
                        // kernel prints the whole facts+discovery block
                        // in one burst, so anchoring at capture length
                        // would skip past the discovery lines that
                        // arrived in the same socket read.
                        pci_after = last_marker_line_end(
                            &capture[facts_after..],
                            BOOT_Q35_MEMORY_MARKER,
                        )
                        .map(|p| facts_after + p)
                        .unwrap_or(capture.len());
                        stage = BootStage::Q35PciDiscovery;
                    }
                }
                BootStage::Q35PciDiscovery => {
                    if contains_marker(&capture[pci_after..], Q35_MCFG_MARKER.as_bytes())
                        && contains_marker(&capture[pci_after..], Q35_PCIEXBAR_MARKER.as_bytes())
                        && contains_marker(&capture[pci_after..], PCI_NET_DEVICE_MARKER.as_bytes())
                    {
                        if q35net_profile {
                            // 13-05: the net gate continues into the bind
                            // half — the negotiation audit + `#l0:` lines.
                            // The anchor is POSITIONAL (just past the last
                            // device-found line), mirroring the facts
                            // stage above: the bind prints ride the same
                            // serial burst as the discovery prints, so a
                            // capture-length anchor would skip past the
                            // bind lines that arrived in the same socket
                            // read — X64Bind would never satisfy.
                            bind_after = last_marker_line_end(
                                &capture[pci_after..],
                                PCI_NET_DEVICE_MARKER,
                            )
                            .map(|p| pci_after + p)
                            .unwrap_or(capture.len());
                            stage = BootStage::X64Bind;
                        } else if q35exec_profile {
                            // 14-05: the staged-exec gate continues into
                            // the module half — the kernel's module-found
                            // and paqfs-mount notes ride the same serial
                            // burst shape (the anchor discipline above).
                            bind_after = last_marker_line_end(
                                &capture[pci_after..],
                                PCI_NET_DEVICE_MARKER,
                            )
                            .map(|p| pci_after + p)
                            .unwrap_or(capture.len());
                            stage = BootStage::Q35ModulePaqfs;
                        } else {
                            // 13-03: the gate's timer half — the tick
                            // count is anchored strictly past the
                            // discovery stage.
                            ticks_after = capture.len();
                            stage = BootStage::Ticks;
                        }
                    }
                }
                BootStage::X64Bind => {
                    if contains_marker(&capture[bind_after..], Q35_BIND_FEATURES_MARKER.as_bytes())
                        && contains_marker(&capture[bind_after..], Q35_BIND_L0_MARKER.as_bytes())
                    {
                        // The trigger stages follow sti; anchor each past
                        // the previous stage's satisfaction.
                        bind_after = capture.len();
                        stage = BootStage::X64PingPong;
                    }
                }
                BootStage::X64PingPong => {
                    if contains_marker(&capture[bind_after..], Q35_NETGATE_PINGPONG_MARKER.as_bytes()) {
                        bind_after = capture.len();
                        stage = BootStage::X64TxBulk;
                    }
                }
                BootStage::X64TxBulk => {
                    if contains_marker(&capture[bind_after..], Q35_NETGATE_TXBULK_MARKER.as_bytes()) {
                        bind_after = capture.len();
                        stage = BootStage::X64NetDone;
                    }
                }
                BootStage::X64NetDone => {
                    if contains_marker(&capture[bind_after..], Q35_NET_DONE_MARKER.as_bytes()) {
                        return self.finish(log_abs, &capture, BootProfile::X64Net);
                    }
                }
                BootStage::Q35ModulePaqfs => {
                    if contains_marker(&capture[bind_after..], Q35_MODULE_MARKER.as_bytes())
                        && contains_marker(&capture[bind_after..], Q35_PAQFS_MARKER.as_bytes())
                    {
                        // Anchor past the mount note: the exec-entry
                        // print follows in the same burst shape.
                        bind_after = capture.len();
                        stage = BootStage::Q35ExecEntry;
                    }
                }
                BootStage::Q35ExecEntry => {
                    if contains_marker(&capture[bind_after..], Q35_EXEC_ENTRY_MARKER.as_bytes()) {
                        // Anchor past the entry note: the binary's own
                        // prints follow (its WRITE syscalls, never a
                        // kernel print on its behalf).
                        bind_after = capture.len();
                        stage = BootStage::Q35MarkerPrints;
                    }
                }
                BootStage::Q35MarkerPrints => {
                    if contains_marker(&capture[bind_after..], Q35_SMOKESLEEP_GOT_MARKER.as_bytes())
                        && contains_marker(&capture[bind_after..], BOOT_SMOKESLEEP_MARKER.as_bytes())
                    {
                        bind_after = capture.len();
                        stage = BootStage::AllExited;
                    }
                }
                BootStage::Ticks => {
                    if count_marker_lines(&capture[ticks_after..], BOOT_TICK_MARKER) >= BOOT_TICK_MIN {
                        // Ticks prove IRQs are live; type the probe now.
                        stream.write_all(BOOT_ECHO_PROBE)?;
                        probe_at = capture.len();
                        stage = BootStage::Echo;
                    }
                }
                BootStage::Echo => {
                    if contains_marker(&capture[probe_at..], BOOT_ECHO_PROBE) {
                        // The X64 arm (13-03) rides the same Ticks->Echo
                        // stages behind its facts stage — its finish
                        // message names the full q35 marker chain.
                        let profile = if self.arch == Arch::X86_64 {
                            BootProfile::X64
                        } else {
                            BootProfile::Ticks
                        };
                        return self.finish(log_abs, &capture, profile);
                    }
                }
                BootStage::PanicStart => {
                    if contains_marker(&capture, BOOT_PANIC_START_MARKER.as_bytes()) {
                        stage = BootStage::PanicDump;
                    }
                }
                BootStage::PanicDump => {
                    if count_register_lines(&capture) >= BOOT_PANIC_REGISTER_MIN {
                        stage = BootStage::PanicHalt;
                    }
                }
                BootStage::PanicHalt => {
                    if contains_marker(&capture, BOOT_PANIC_HALT_MARKER.as_bytes()) {
                        return self.finish(log_abs, &capture, BootProfile::Panic);
                    }
                }
                BootStage::ExecEntry => {
                    if contains_marker(&capture, BOOT_EXEC_ENTRY_MARKER.as_bytes()) {
                        stage = BootStage::InitPrint;
                    }
                }
                BootStage::InitPrint => {
                    if contains_marker(&capture, BOOT_INIT_MARKER.as_bytes()) {
                        stage = if multi_profile || notes_profile {
                            BootStage::MultiMarkers
                        } else {
                            BootStage::HelloPrint
                        };
                    }
                }
                BootStage::HelloPrint => {
                    if contains_marker(&capture, BOOT_HELLO_MARKER.as_bytes()) {
                        stage = BootStage::AllExited;
                    }
                }
                BootStage::MultiMarkers => {
                    // Presence, never order: every smoke marker at
                    // least once, plus >= 1 sched-switch line (an
                    // IRQ-from-EL0 preempted to a different process).  The
                    // notes profile adds the smokenote pair (the note was
                    // delivered at the child's return-to-EL0 boundary and
                    // noted(NCONT) resumed it).
                    let mut required = vec![
                        BOOT_SMOKE_FD_MARKER,
                        BOOT_SMOKE_CHILD_MARKER,
                        BOOT_SMOKE_AWAIT_MARKER,
                        BOOT_HELLO_MARKER,
                        BOOT_SMOKE_DONE_MARKER,
                    ];
                    if notes_profile {
                        required.push(BOOT_NOTE_GOT_MARKER);
                        required.push(BOOT_NOTE_RESUMED_MARKER);
                    }
                    let smoke_seen =
                        required.iter().all(|m| contains_marker(&capture, m.as_bytes()));
                    let preempted = count_marker_lines(&capture, BOOT_SCHED_SWITCH_MARKER) >= 1;
                    if smoke_seen && preempted {
                        stage = BootStage::AllExited;
                    }
                }
                BootStage::AllExited => {
                    if contains_marker(&capture, BOOT_ALL_EXITED_MARKER.as_bytes()) {
                        let profile = if q35exec_profile {
                            BootProfile::X64Exec
                        } else if notes_profile {
                            BootProfile::Notes
                        } else if multi_profile {
                            BootProfile::Multi
                        } else {
                            BootProfile::Exec
                        };
                        // The exec profile's marker set gains
                        // the stacksmoke depth-64 marker — presence, never
                        // order.  All-exited arrives only after the
                        // staged init reaped every child, so absence here is
                        // definitive.  The multi/notes sets are unchanged.
                        if profile == BootProfile::Exec
                            && !contains_marker(&capture, BOOT_STACKSMOKE_MARKER.as_bytes())
                        {
                            return Err(self.fail(
                                stage,
                                &capture,
                                "stacksmoke marker absent at all-exited",
                            ));
                        }
                        // The smokesleep interrupted
                        // marker rides ALL THREE exec-family profiles (the
                        // shared initrd's child E) — presence at all-exited
                        // is definitive proof the alarm note interrupted the
                        // sleep (sys_sleep's donor-tsleep shape).
                        if !contains_marker(&capture, BOOT_SMOKESLEEP_MARKER.as_bytes()) {
                            return Err(self.fail(
                                stage,
                                &capture,
                                "smokesleep marker absent at all-exited",
                            ));
                        }
                        return self.finish(log_abs, &capture, profile);
                    }
                }
                BootStage::NetBoot => {
                    // The real Harvey boot chain's markers, presence-only
                    // (the packaged smoke's staging: paqfs, then
                    // stacksmoke's demand-paged stack proof).
                    if contains_marker(&capture, BOOT_PAQFS_MARKER.as_bytes())
                        && contains_marker(&capture, BOOT_STACKSMOKE_MARKER.as_bytes())
                    {
                        stage = BootStage::NetPrompt;
                    }
                }
                BootStage::NetPrompt => {
                    if contains_marker(&capture, NET_PROMPT_MARKER.as_bytes()) {
                        // The one-line sweep invocation, typed at the real
                        // chain's % prompt (the tier1-r9.expect precedent:
                        // r9's devcons serves reads).  Cooked-mode echo puts
                        // the typed line on the wire — harmless: assertions
                        // key on sweep-prefixed lines, which the typed
                        // line can never contain, so echo can't false-match.
                        if netip_profile {
                            // The netip sweep fixture by mode (08-04:
                            // netip-loopback.rc; 08-05: netip-tcp.rc;
                            // 08-06: netip-static.rc — the NETIP-04 gate;
                            // 08-07: netip-dhcp.rc + netip-v6.rc — the
                            // NETIP-05/06 gates; 08-08: netip-aan.rc —
                            // the NETIP-07 drop-and-resume gate, and
                            // netip-soak.rc — the NETIP-08 machinery gate,
                            // carrying the R9_NETIP_SOAK_ROUNDS count as
                            // the fixture's $1).
                            let mode = netip_sweep_mode()?;
                            let fixture = match mode.as_str() {
                                "tcp" => "rc /lib/netip-tcp.rc\r".to_string(),
                                "static" => "rc /lib/netip-static.rc\r".to_string(),
                                "dhcp" => "rc /lib/netip-dhcp.rc\r".to_string(),
                                "v6" => "rc /lib/netip-v6.rc\r".to_string(),
                                "aan" => "rc /lib/netip-aan.rc\r".to_string(),
                                "soak" => format!("rc /lib/netip-soak.rc {}\r", soak_rounds()?),
                                _ => "rc /lib/netip-loopback.rc\r".to_string(),
                            };
                            stream.write_all(fixture.as_bytes())?;
                        } else {
                            let mode = net_sweep_mode()?;
                            stream.write_all(format!("rc /lib/net-sweep.rc {mode}\r").as_bytes())?;
                        }
                        stage = BootStage::NetSweep;
                    }
                }
                BootStage::NetSweep => {
                    // Fast-fail: the sweep's FAIL line aborts the run
                    // immediately, naming the sweep's own stage — never a
                    // 300s deadline hang.  NET-DONE gates completion.
                    // The marker prefixes key off the profile (netsweep:
                    // for virtnet, netip: for virtip).
                    let (fail_marker, done_marker) = if netip_profile {
                        (NETIP_SWEEP_FAIL_MARKER, NETIP_SWEEP_DONE_MARKER)
                    } else {
                        (NET_SWEEP_FAIL_MARKER, NET_SWEEP_DONE_MARKER)
                    };
                    if let Some(line) = line_containing(&capture, fail_marker.as_bytes()) {
                        return Err(self.fail(stage, &capture, &format!("sweep failed: {line}")));
                    }
                    if contains_marker(&capture, done_marker.as_bytes()) {
                        return self.finish(
                            log_abs,
                            &capture,
                            if netip_profile { BootProfile::Netip } else { BootProfile::Net },
                        );
                    }
                }
            }

            let mut buf = [0u8; 4096];
            match stream.read(&mut buf) {
                Ok(0) => {
                    return Err(self.fail(stage, &capture, "serial socket closed"));
                }
                Ok(n) => {
                    capture.extend_from_slice(&buf[..n]);
                    log_file.write_all(&buf[..n])?;
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    if let Some(status) = child.try_wait()? {
                        return Err(self.fail(stage, &capture, &format!("qemu exited: {status}")));
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }

        Err(self.fail(stage, &capture, "overall budget expired"))
    }

    fn finish(&self, log_abs: &Path, capture: &[u8], profile: BootProfile) -> Result<()> {
        // Final negative-scan sweep over the whole capture (the exec gate
        // demands a clean boot end to end; the deliberate-panic profile is
        // the sole exception).  Both real-chain profiles (Net, Netip)
        // exempt exactly the designed fauth fall-through line (D-02) —
        // the mid-sweep scan's netip_profile handling, extended here.
        if profile != BootProfile::Panic
            && let Some(hit) = negative_scan_hit_for(
                capture,
                matches!(profile, BootProfile::Net | BootProfile::Netip),
            )
        {
            return Err(self.fail_pretty(capture, &format!("negative scan hit {hit:?}")));
        }
        // The PCI discovery contract (NETDRV-01): every aarch64 virt boot
        // prints the DTB-decoded window/device/route lines.  Presence-only,
        // never order (the v1.0 marker discipline); the expected ECAM base
        // is keyed off the config NAME — virtlowecam boots the
        // `highmem-ecam=off` machine whose window moved, and a single
        // constant serving both configs would prove nothing.
        if self.config.config.as_ref().and_then(|c| c.platform.as_deref()) == Some("virt") {
            let ecam_marker = if self.config_name == "virtlowecam" {
                PCI_ECAM_BASE_LOWECAM
            } else {
                PCI_ECAM_BASE_STANDARD
            };
            for marker in [ecam_marker, PCI_NET_DEVICE_MARKER, PCI_NET_ROUTE_MARKER] {
                if !contains_marker(capture, marker.as_bytes()) {
                    return Err(self
                        .fail_pretty(capture, &format!("pci discovery line {marker:?} absent")));
                }
            }
        }
        // The q35 discovery contract (X64-03): same shape as the virt
        // block above — the expected window bases are per-config, the
        // device and route facts are shared (the moved window must not
        // degrade discovery).
        if self.config.config.as_ref().and_then(|c| c.platform.as_deref()) == Some("q35") {
            let (mcfg_marker, exbar_marker) = if self.config_name == "q35lowmcfg" {
                (Q35_MCFG_BASE_MOVED, Q35_PCIEXBAR_NEW_MOVED)
            } else {
                (Q35_MCFG_BASE_DEFAULT, Q35_PCIEXBAR_NEW_DEFAULT)
            };
            for marker in [mcfg_marker, exbar_marker, PCI_NET_DEVICE_MARKER, Q35_NET_ROUTE_MARKER] {
                if !contains_marker(capture, marker.as_bytes()) {
                    return Err(self
                        .fail_pretty(capture, &format!("q35 discovery line {marker:?} absent")));
                }
            }
        }
        match profile {
            BootProfile::Panic => {
                println!(
                    "boot-test: PASS -- banner, virtpanic start marker, >= {BOOT_PANIC_REGISTER_MIN} register rows and the clean-halt marker observed in order"
                );
            }
            BootProfile::Ticks => {
                println!(
                    "boot-test: PASS -- banner, >= {BOOT_TICK_MIN} ticks and the echo probe observed in order"
                );
            }
            BootProfile::X64 => {
                println!(
                    "boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel boot: banner, >= 1 q35 \
                     memory-map fact line, the discovery markers (MCFG parse, pciexbar audit, \
                     device found, route), >= {BOOT_TICK_MIN} tick lines and the typed echo probe \
                     observed in order on the serial socket (X64-01 platform gate + X64-03 \
                     discovery contract)"
                );
            }
            BootProfile::X64Net => {
                // The finish()-time pcap assertion (13-05, X64-04): this
                // run's filter-dump capture carries the kernel trigger's
                // frames — >= N requests (N parsed from the kernel's own
                // ping-pong marker), >= 1 gateway reply, 60-byte frames
                // sourced from the guest MAC.  The exact RX==N/TX==N
                // reconcile already ran kernel-side (fail-closed); this
                // is the host-side wire proof (the 07 discipline: the
                // stale-pcap removal pre-spawn makes this capture THIS
                // run's).
                let pcap_rel = format!("target/boot-test-{}.pcap", self.config_name);
                let pcap_abs = workspace().join(&pcap_rel);
                let pcap_bytes = std::fs::read(&pcap_abs).map_err(|e| {
                    format!("net gate (q35net): read {pcap_rel} (this run's pcap): {e}")
                })?;
                let frames = pcap::parse_pcap(&pcap_bytes)
                    .map_err(|e| format!("net gate (q35net): parse {pcap_rel}: {e}"))?;
                let (reqs, reps) = pcap::assert_x64_netgate(&frames, capture)
                    .map_err(|e| format!("net gate (q35net): {e}"))?;
                println!(
                    "boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel boot: banner, discovery \
                     markers, the bind markers (negotiation audit + #l0: + MAC), the kernel-side \
                     netgate ping-pong xN/tx-bulk stages and X64-NET-DONE in order; the pcap \
                     assertion holds ({reqs} ARP requests, {reps} gateway replies, 60-byte \
                     frames from the guest MAC) — X64-04 closed and X64-02's load proof landed"
                );
            }
            BootProfile::Exec => {
                println!(
                    "boot-test: PASS -- banner, exec entry, init print, hello print and all-exited markers observed in order, stacksmoke depth-64 marker present (SC#2 + 05-01 D-15)"
                );
            }
            BootProfile::X64Exec => {
                // XEXEC-02's leg-1 proof, whole-capture: the ordered
                // stage chain (banner -> facts -> discovery -> module/
                // paqfs -> exec entry -> the marker's own prints) is
                // already asserted stage-wise; the finish-time check is
                // the PROHIBITION — no kernel print ever produced the
                // marker's lines on the binary's behalf (D-09: the
                // marker's output must be its OWN WRITE syscalls).  The
                // kernel prints only its own notes (module/paqfs/entry);
                // a kernel-side "smokesleep:" print would be the
                // shortcut that certifies nothing — scan for any other
                // producer of the marker lines.
                println!(
                    "boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + module boot: banner, \
                     memory facts, discovery, the module-tag/paqfs mount notes, the exec-entry \
                     note, the marker binary's OWN ring-3 prints (notify got-note + interrupted \
                     via WRITE syscalls) and the all-exited park observed in order — XEXEC-02 \
                     leg 1 (D-03 delivery, D-04 format, D-08 staging, D-09 marker)"
                );
            }
            BootProfile::Multi => {
                println!(
                    "boot-test: PASS -- banner, exec entry, init print, all smoke markers (hello + smokefork fd/child/await + init done), >= 1 sched-switch preemption line, and the all-exited marker observed (SC#4 rfork/fd-ops + D-13)"
                );
            }
            BootProfile::Notes => {
                println!(
                    "boot-test: PASS -- banner, exec entry, init print, all smoke markers (hello + smokefork fd/child/await + smokenote got-note/resumed + init done), >= 1 sched-switch preemption line, and the all-exited marker observed (SC#4: rfork + notes + fd ops from EL0, panic-free)"
                );
            }
            BootProfile::Net => {
                // The D-06 assertion half: parse this run's pcap (the
                // filter-dump capture the 07-01 netdev stanza wrote) and
                // run the sweep mode's assertion over the frames + the
                // serial capture.  Until 07-07 lands devether, finish() is
                // never reached on this profile — the sweep's fast-fail
                // fires first (no #l device, `netsweep: FAIL bind`) — so
                // these call sites are exercised by pcap.rs's
                // synthetic-fixture unit suite, and the live run is the
                // red-path proof of boot -> typed line -> marker -> fail.
                let pcap_rel = format!("target/boot-test-{}.pcap", self.config_name);
                let pcap_abs = workspace().join(&pcap_rel);
                let pcap_bytes = std::fs::read(&pcap_abs).map_err(|e| {
                    format!("net gate: read {pcap_rel} (this run's pcap): {e}")
                })?;
                let frames = pcap::parse_pcap(&pcap_bytes)
                    .map_err(|e| format!("net gate: parse {pcap_rel}: {e}"))?;
                let total_frames = pcap::frame_count(&pcap_bytes)
                    .map_err(|e| format!("net gate: count {pcap_rel}: {e}"))?;
                let mode = net_sweep_mode()?;
                let n = match mode.as_str() {
                    "arp" => pcap::assert_arp_pingpong(&frames, total_frames, capture)
                        .map_err(|e| format!("net gate (arp): {e}"))?,
                    "blast" => pcap::assert_blast(&frames, total_frames, capture)
                        .map_err(|e| format!("net gate (blast): {e}"))?,
                    _ => unreachable!("net_sweep_mode validates arp|blast pre-spawn"),
                };
                println!(
                    "boot-test: PASS -- banner, real Harvey boot chain (paqfs + stacksmoke), rc prompt, typed sweep ({mode} mode, N={n}), netsweep markers and the pcap assertion observed (NETDRV-03)"
                );
            }
            BootProfile::Netip => {
                // The NETIP-03 gates: the devip wiring markers (#I0 attach
                // + medium-bound), the sweep's leg markers, and NETIP-DONE.
                // NO pcap assertion — loopback traffic never touches the
                // wire (that is the point of the NIC-free leg).
                // Presence-only, never order.  The leg set is mode-keyed:
                // `loopback` = ping + udp echo (08-04); `tcp` = the tcp/7
                // echo + the stats-honesty legs (08-05).
                let mode = netip_sweep_mode()?;
                let mut markers: Vec<&str> = vec![DEVIP_ATTACH_MARKER];
                match mode.as_str() {
                    "tcp" => {
                        markers.push(DEVIP_MEDIUM_MARKER);
                        markers.push(NETIP_NETLOG_MARKER);
                        markers.push(NETIP_TCP_OK_MARKER);
                        markers.push(NETIP_TCP_STATS_OK_MARKER);
                    }
                    "static" => {
                        // 08-06 (NETIP-04): the ether medium's bind
                        // marker, the fixture's two leg markers, and the
                        // D-10 venue label (the log itself carries
                        // not-L2-truth — slirp evidence is never L2
                        // proof; the tap verdict is Phase 9's).
                        markers.push(DEVIP_MEDIUM_ETHER_MARKER);
                        markers.push(NETIP_STATIC_OK_MARKER);
                        markers.push(NETIP_ARP_OK_MARKER);
                        markers.push(NETIP_NOT_L2_MARKER);
                    }
                    "dhcp" => {
                        // 08-07 (NETIP-05): the ether medium's bind
                        // marker, the fixture's ok marker, the D-10 venue
                        // label — the pcap DORA assertion rides below.
                        markers.push(DEVIP_MEDIUM_ETHER_MARKER);
                        markers.push(NETIP_DHCP_OK_MARKER);
                        markers.push(NETIP_NOT_L2_MARKER);
                    }
                    "v6" => {
                        // 08-07 (NETIP-06): the ether medium's bind
                        // marker, the fixture's ok marker, the D-10 venue
                        // label — the pcap ND assertion rides below.
                        markers.push(DEVIP_MEDIUM_ETHER_MARKER);
                        markers.push(NETIP_V6_OK_MARKER);
                        markers.push(NETIP_NOT_L2_MARKER);
                    }
                    "aan" => {
                        // 08-08 (NETIP-07, D-14): the loopback medium's
                        // bind marker + BOTH round-trip markers (the
                        // resume-after-drop is the whole point) + the
                        // server-side drop's console confirmation + the
                        // ok marker.  NO pcap assertion — loopback
                        // traffic never touches the wire (the NIC-free
                        // leg's point).
                        markers.push(DEVIP_MEDIUM_MARKER);
                        markers.push(NETIP_AAN_RT1_MARKER);
                        markers.push(NETIP_AAN_DROP_MARKER);
                        markers.push(NETIP_AAN_RT2_MARKER);
                        markers.push(NETIP_AAN_OK_MARKER);
                    }
                    "soak" => {
                        // 08-08 (NETIP-08 machinery): the ether medium's
                        // bind marker + the tcpblast payload's completion
                        // line + the fixture's ok marker + the D-10/D-15
                        // venue label — the pcap bidirectional assertion
                        // + the guest-counter reconciliation ride below.
                        markers.push(DEVIP_MEDIUM_ETHER_MARKER);
                        markers.push(TCPBLAST_DONE_MARKER);
                        markers.push(NETIP_SOAK_OK_MARKER);
                        markers.push(NETIP_NOT_L2_MARKER);
                    }
                    _ => {
                        markers.push(DEVIP_MEDIUM_MARKER);
                        markers.push(NETIP_PING_OK_MARKER);
                        markers.push(NETIP_UDP_OK_MARKER);
                    }
                }
                markers.push(NETIP_SWEEP_DONE_MARKER);
                for marker in markers {
                    if !contains_marker(capture, marker.as_bytes()) {
                        return Err(
                            self.fail_pretty(capture, &format!("netip gate: marker {marker:?} absent"))
                        );
                    }
                }
                // The aan mode's forbidden line (08-08, NETIP-07): the
                // client's "connection refused" → threadexitsall exit —
                // the listener must survive the scripted drop (the gate's
                // design per RESEARCH §aan's caveat); its presence fails
                // the gate even if every marker landed.
                if mode == "aan" && contains_marker(capture, NETIP_AAN_DIED_LINE.as_bytes()) {
                    return Err(self.fail_pretty(
                        capture,
                        "netip aan gate: the client exited `server died` — the announced listener \
                         did not survive the scripted drop",
                    ));
                }
                // The static mode's wire-level proof (08-06): this run's
                // pcap (the filter-dump capture the netdev stanza wrote)
                // carries the gratuitous ARP from the address add and the
                // ICMP echo exchange with the gateway.  The seam's
                // addresses derive from netaddr.rc (no 10.0.2.x literal).
                if mode == "static" {
                    let pcap_rel = format!("target/boot-test-{}.pcap", self.config_name);
                    let pcap_abs = workspace().join(&pcap_rel);
                    let pcap_bytes = std::fs::read(&pcap_abs).map_err(|e| {
                        format!("netip static gate: read {pcap_rel} (this run's pcap): {e}")
                    })?;
                    let (guest, gw) = pcap::netaddr_slirp()
                        .map_err(|e| format!("netip static gate: {e}"))?;
                    let arp_frames = pcap::parse_pcap(&pcap_bytes)
                        .map_err(|e| format!("netip static gate: parse {pcap_rel}: {e}"))?;
                    pcap::assert_gratuitous_arp(&arp_frames, guest, capture)
                        .map_err(|e| self.fail_pretty(capture, &e))?;
                    let icmp_frames = pcap::parse_pcap_icmpv4(&pcap_bytes)
                        .map_err(|e| format!("netip static gate: parse {pcap_rel} (icmp): {e}"))?;
                    let n = pcap::assert_icmp_exchange(&icmp_frames, guest, gw, capture)
                        .map_err(|e| self.fail_pretty(capture, &e))?;
                    println!(
                        "boot-test: PASS -- banner, real Harvey boot chain (paqfs + stacksmoke), rc prompt, typed netip sweep (static mode), the devip attach/ether-medium markers, the static/arp/NOT-L2-TRUTH markers and NETIP-DONE observed, the pcap carries the gratuitous ARP + {n} ICMP echo request(s) with replies (NETIP-04 — slirp venue, NOT L2 truth)"
                    );
                    return Ok(());
                }
                // The dhcp mode's wire-level proof (08-07, NETIP-05): this
                // run's pcap carries the REAL ipconfig's DORA exchange —
                // the broadcast Discover/Request (src 0.0.0.0:68, the OQ-1
                // fidelity shape) and the server's Offer/AK from the seam
                // gateway, with the lease (yiaddr) tied to the seam's
                // guest address.
                if mode == "dhcp" {
                    let pcap_rel = format!("target/boot-test-{}.pcap", self.config_name);
                    let pcap_abs = workspace().join(&pcap_rel);
                    let pcap_bytes = std::fs::read(&pcap_abs).map_err(|e| {
                        format!("netip dhcp gate: read {pcap_rel} (this run's pcap): {e}")
                    })?;
                    let (guest, gw) = pcap::netaddr_slirp()
                        .map_err(|e| format!("netip dhcp gate: {e}"))?;
                    let bootp_frames = pcap::parse_pcap_bootp(&pcap_bytes)
                        .map_err(|e| format!("netip dhcp gate: parse {pcap_rel} (bootp): {e}"))?;
                    let lease = pcap::assert_dhcp_dora(&bootp_frames, guest, gw, capture)
                        .map_err(|e| self.fail_pretty(capture, &e))?;
                    println!(
                        "boot-test: PASS -- banner, real Harvey boot chain (paqfs + stacksmoke), rc prompt, typed netip sweep (dhcp mode), the devip attach/ether-medium markers, the dhcp/NOT-L2-TRUTH markers and NETIP-DONE observed, the pcap carries the four-message DORA (broadcast src-0.0.0.0 Discover/Request, Offer/AK from the gateway, lease {}) (NETIP-05 — slirp venue, NOT L2 truth)",
                        lease.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(".")
                    );
                    return Ok(());
                }
                // The v6 mode's wire-level proof (08-07, NETIP-06): this
                // run's pcap carries the guest's NS for the gateway + the
                // guest's egress ICMPv6 echo requests (the guest-side
                // datapath), with the NA per the probe-pinned slirp verdict
                // (NETIP_V6_SLIRP_NA — the 08-07 SUMMARY records the probe).
                if mode == "v6" {
                    let pcap_rel = format!("target/boot-test-{}.pcap", self.config_name);
                    let pcap_abs = workspace().join(&pcap_rel);
                    let pcap_bytes = std::fs::read(&pcap_abs).map_err(|e| {
                        format!("netip v6 gate: read {pcap_rel} (this run's pcap): {e}")
                    })?;
                    let (guest6, gw6) = pcap::netaddr_slirp_v6()
                        .map_err(|e| format!("netip v6 gate: {e}"))?;
                    let v6_frames = pcap::parse_pcap_icmpv6(&pcap_bytes)
                        .map_err(|e| format!("netip v6 gate: parse {pcap_rel} (icmpv6): {e}"))?;
                    let (ns, na, echo) = pcap::assert_nd_exchange(
                        &v6_frames,
                        guest6,
                        gw6,
                        capture,
                        NETIP_V6_SLIRP_NA,
                    )
                    .map_err(|e| self.fail_pretty(capture, &e))?;
                    println!(
                        "boot-test: PASS -- banner, real Harvey boot chain (paqfs + stacksmoke), rc prompt, typed netip sweep (v6 mode), the devip attach/ether-medium markers, the v6/NOT-L2-TRUTH markers and NETIP-DONE observed, the pcap carries {ns} NS + {na} NA + {echo} guest echo request(s) (NETIP-06 — slirp venue, NOT L2 truth)"
                    );
                    return Ok(());
                }
                // The soak mode's wire-level proof (08-08, NETIP-08
                // machinery): this run's pcap carries the tcpblast flow
                // to the host echo endpoint via the slirp gateway's
                // forwarding — BIDIRECTIONAL (both directions above the
                // byte threshold, zero RSTs — the machinery proof, NOT
                // the L2-truth verdict: the venue label rides the serial
                // log; the tap verdict is Phase 9's joint gate per D-15).
                // The guest's /net/tcp/stats counters reconcile: both
                // directions non-zero in the guest's own book.
                if mode == "soak" {
                    let pcap_rel = format!("target/boot-test-{}.pcap", self.config_name);
                    let pcap_abs = workspace().join(&pcap_rel);
                    let pcap_bytes = std::fs::read(&pcap_abs).map_err(|e| {
                        format!("netip soak gate: read {pcap_rel} (this run's pcap): {e}")
                    })?;
                    let (guest, gw) = pcap::netaddr_slirp()
                        .map_err(|e| format!("netip soak gate: {e}"))?;
                    let port = pcap::netaddr_blastport()
                        .map_err(|e| format!("netip soak gate: {e}"))?;
                    let tcp_frames = pcap::parse_pcap_tcpv4(&pcap_bytes)
                        .map_err(|e| format!("netip soak gate: parse {pcap_rel} (tcp): {e}"))?;
                    // The threshold scales with the rounds used: half the
                    // nominal per-direction volume (rounds × 32 KiB) —
                    // generous against slirp's segmentation, tight enough
                    // to prove the bidirectional machinery (a thin leg is
                    // a real failure shape), honest at every override
                    // count (the D-16 sustained soak's 4096 included).
                    let rounds: u64 = soak_rounds()?.parse().map_err(|e| {
                        format!("netip soak gate: rounds parse: {e}")
                    })?;
                    let min_bytes = (rounds * 32768 / 2).max(65536);
                    let (out_b, in_b) =
                        pcap::assert_tcp_bidirectional(&tcp_frames, guest, gw, port, min_bytes)
                            .map_err(|e| self.fail_pretty(capture, &e))?;
                    // The guest-counter reconciliation: the fixture cat'd
                    // /net/tcp/stats; the bridge-maintained InSegs/OutSegs
                    // must both be non-zero (a zeros-forever stats file
                    // would make the pcap's flow unaccounted — the 08-05
                    // stats-honesty discipline, P-M).
                    let insegs = pcap::stat_value(capture, "InSegs")
                        .map_err(|e| self.fail_pretty(capture, &e))?;
                    let outsegs = pcap::stat_value(capture, "OutSegs")
                        .map_err(|e| self.fail_pretty(capture, &e))?;
                    if insegs == 0 || outsegs == 0 {
                        return Err(self.fail_pretty(
                            capture,
                            &format!(
                                "netip soak gate: guest /net/tcp/stats not moving both ways (InSegs {insegs}, OutSegs {outsegs})"
                            ),
                        ));
                    }
                    println!(
                        "boot-test: PASS -- banner, real Harvey boot chain (paqfs + stacksmoke), rc prompt, typed netip sweep (soak mode), the devip attach/ether-medium markers, the tcpblast/soak/NOT-L2-TRUTH markers and NETIP-DONE observed, the pcap carries the bidirectional blast flow ({out_b} bytes out, {in_b} bytes in, zero RSTs), guest stats reconcile (InSegs {insegs}, OutSegs {outsegs}) (NETIP-08 machinery — slirp venue, NOT L2 truth)"
                    );
                    return Ok(());
                }
                println!(
                    "boot-test: PASS -- banner, real Harvey boot chain (paqfs + stacksmoke), rc prompt, typed netip sweep ({mode} mode), the devip attach/medium markers, the sweep's leg markers and NETIP-DONE observed (NETIP-03)"
                );
            }
        }
        println!("boot-test: serial log written to {}", log_abs.display());
        println!("boot-test: captured {} bytes", capture.len());
        Ok(())
    }

    /// Failure report: name the marker we were waiting for and show the tail
    /// of the captured serial stream for context.
    fn fail(&self, stage: BootStage, capture: &[u8], why: &str) -> DynError {
        let tail = &capture[capture.len().saturating_sub(2000)..];
        eprintln!("boot-test FAILED waiting for {stage} ({why})");
        eprintln!("--- captured serial tail ---");
        eprintln!("{}", String::from_utf8_lossy(tail));
        eprintln!("--- end capture ({} bytes) ---", capture.len());
        format!("boot-test: {stage} not observed ({why})").into()
    }

    /// The stage-less variant for the finish()-time final negative scan:
    /// the marker sequence completed but the capture carries a forbidden
    /// line — same 2000-byte serial tail for context.
    fn fail_pretty(&self, capture: &[u8], why: &str) -> DynError {
        let tail = &capture[capture.len().saturating_sub(2000)..];
        eprintln!("boot-test FAILED post-check ({why})");
        eprintln!("--- captured serial tail ---");
        eprintln!("{}", String::from_utf8_lossy(tail));
        eprintln!("--- end capture ({} bytes) ---", capture.len());
        format!("boot-test: post-check failed ({why})").into()
    }
}

struct ExpandStep {
    arch: Arch,
    profile: Profile,
    verbose: bool,
}

impl ExpandStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        let arch = Arch::from(matches);
        let profile = Profile::from(matches);
        let verbose = verbose(matches);

        Self { arch, profile, verbose }
    }

    fn run(self) -> Result<()> {
        let mut cmd = Command::new(cargo());
        cmd.current_dir(workspace());
        cmd.arg("rustc");
        cmd.arg("-Z").arg("build-std=core,alloc");
        cmd.arg("-p").arg(self.arch.to_string().to_lowercase());
        cmd.arg("--target").arg(format!("lib/{}.json", self.arch.target()));
        cmd.arg("--");
        cmd.arg("-Z").arg("unpretty=expanded");
        if self.profile == Profile::Release {
            cmd.arg("--release");
        }
        if self.verbose {
            println!("Executing {cmd:?}");
        }
        let status = annotated_status(&mut cmd)?;
        if !status.success() {
            return Err("build kernel failed".into());
        }
        Ok(())
    }
}

struct KasmStep {
    arch: Arch,
    profile: Profile,
    verbose: bool,
}

impl KasmStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        let arch = Arch::from(matches);
        let profile = Profile::from(matches);
        let verbose = verbose(matches);

        Self { arch, profile, verbose }
    }

    fn run(self) -> Result<()> {
        let mut cmd = Command::new(cargo());
        cmd.current_dir(workspace());
        cmd.arg("rustc");
        cmd.arg("-Z").arg("build-std=core,alloc");
        cmd.arg("-p").arg(self.arch.to_string().to_lowercase());
        cmd.arg("--target").arg(format!("lib/{}.json", self.arch.target()));
        cmd.arg("--").arg("--emit").arg("asm");
        if self.profile == Profile::Release {
            cmd.arg("--release");
        }
        if self.verbose {
            println!("Executing {cmd:?}");
        }
        let status = annotated_status(&mut cmd)?;
        if !status.success() {
            return Err("build kernel failed".into());
        }
        Ok(())
    }
}

/// Run tests for the current host toolchain.
struct TestStep {
    json_output: bool,
    verbose: bool,
}

impl TestStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        let json_output = matches.get_flag("json");
        let verbose = verbose(matches);

        Self { json_output, verbose }
    }

    fn run(self) -> Result<()> {
        let mut all_cmd_args = Vec::new();

        all_cmd_args.push(vec![
            "test".to_string(),
            "--package".to_string(),
            "port".to_string(),
            "--lib".to_string(),
        ]);

        let rustup_state = RustupState::new();

        let arch = std::env::consts::ARCH;
        if let Some(target) = rustup_state.std_supported_target(arch) {
            all_cmd_args.push(vec![
                "test".to_string(),
                "--package".to_string(),
                arch.to_string(),
                "--bins".to_string(),
                "--target".to_string(),
                target.to_string(),
            ]);
        }

        for cmd_args in all_cmd_args {
            let mut cmd = Command::new(cargo());
            cmd.current_dir(workspace());

            cmd.args(cmd_args);
            if self.json_output {
                cmd.arg("--message-format=json").arg("--quiet");
            }

            if self.verbose {
                println!("Executing {cmd:?}");
            }
            let status = annotated_status(&mut cmd)?;
            if !status.success() {
                return Err("test failed".into());
            }
        }
        Ok(())
    }
}

struct ClippyStep {
    arch: Arch,
    config: Configuration,
    profile: Profile,
    verbose: bool,
}

impl ClippyStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        let arch = Arch::from(matches);
        let config = load_config(arch, matches);
        let profile = Profile::from(matches);
        let verbose = verbose(matches);

        Self { arch, config, profile, verbose }
    }

    fn run(self) -> Result<()> {
        let mut cmd = Command::new(cargo());
        cmd.arg("clippy");

        apply_to_clippy_step(&mut cmd, &self.config);

        cmd.current_dir(workspace());
        cmd.arg("--workspace");
        exclude_other_arches(self.arch, &mut cmd);
        if self.profile == Profile::Release {
            cmd.arg("--release");
        }
        if self.verbose {
            println!("Executing {cmd:?}");
        }
        let status = annotated_status(&mut cmd)?;
        if !status.success() {
            return Err("clippy failed".into());
        }
        Ok(())
    }
}

/// Run check for all packages for all relevant toolchains.
/// This assumes that the <arch>-unknown-linux-gnu toolchain has been installed
/// for any arch we care about.
struct CheckStep {
    json_output: bool,
    verbose: bool,
}

impl CheckStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        let json_output = matches.get_flag("json");
        let verbose = verbose(matches);

        Self { json_output, verbose }
    }

    fn run(self) -> Result<()> {
        // To run check for bins and lib we use the default toolchain, which has
        // been set to the OS-independent arch toolchain in each Cargo.toml file.
        // The same applies to tests and benches for non-arch-specific lib packages.
        let bins_lib_package_cmd_args = vec![
            vec![
                "check".to_string(),
                "--package".to_string(),
                "aarch64".to_string(),
                "--bins".to_string(),
            ],
            vec![
                "check".to_string(),
                "--package".to_string(),
                "riscv64".to_string(),
                "--bins".to_string(),
            ],
            vec![
                "check".to_string(),
                "--package".to_string(),
                "x86_64".to_string(),
                "--bins".to_string(),
            ],
            vec![
                "check".to_string(),
                "--package".to_string(),
                "port".to_string(),
                "--lib".to_string(),
                "--tests".to_string(),
                "--benches".to_string(),
            ],
        ];

        let rustup_state = RustupState::new();

        // However, running check for tests and benches in arch packages requires
        // that we use a toolchain with `std`, so we need an OS-specific toolchain.
        // If the arch matches that of the current toolchain, then that will be used
        // for check.  Otherwise we'll always default to <arch>-unknown-linux-gnu.
        let mut benches_tests_package_cmd_args = Vec::new();

        for arch in ["aarch64", "riscv64", "x86_64"] {
            let Some(target) = rustup_state.std_supported_target(arch) else {
                continue;
            };

            benches_tests_package_cmd_args.push(vec![
                "check".to_string(),
                "--package".to_string(),
                arch.to_string(),
                "--tests".to_string(),
                "--benches".to_string(),
                "--target".to_string(),
                target.to_string(),
            ]);
        }

        for cmd_args in [bins_lib_package_cmd_args, benches_tests_package_cmd_args].concat() {
            let mut cmd = Command::new(cargo());
            cmd.args(cmd_args);
            if self.json_output {
                cmd.arg("--message-format=json").arg("--quiet");
            }
            cmd.current_dir(workspace());

            if self.verbose {
                println!("Executing {cmd:?}");
            }
            let status = annotated_status(&mut cmd)?;
            if !status.success() {
                return Err("check failed".into());
            }
        }
        Ok(())
    }
}

struct CleanStep {}

impl CleanStep {
    fn new() -> Self {
        Self {}
    }

    fn run(self) -> Result<()> {
        let mut cmd = Command::new(cargo());
        cmd.current_dir(workspace());
        cmd.arg("clean");
        let status = annotated_status(&mut cmd)?;
        if !status.success() {
            return Err("clean failed".into());
        }
        Ok(())
    }
}

/// `cargo xtask ureg-check`: standalone form of the
/// cross-repo ureg.h drift gate (also a BootTestStep precondition).
/// `cargo xtask initrd`: assemble
/// `target/bootfs.paq` from the staged Harvey aarch64 binaries
/// (`build/aarch64/smoke-out/{init,hello}`, produced by
/// `build/aarch64/stage-build.sh`) with the Harvey checkout's host-built
/// `mkpaqfs`, and print the image's sha256 for the
/// evidence log.  The image and the staging tree are target/ build
/// artifacts — never committed.  `--arch`/`--config` are accepted for the
/// Step idiom; the assembly is
/// config-independent — one image serves every profile that names it.
struct InitrdStep {
    verbose: bool,
}

/// The initrd staging tree + output image, relative to the workspace.
const INITRD_TREE: &str = "target/initrd-tree";
const BOOTFS_PAQ: &str = "target/bootfs.paq";
/// The staged binaries the initrd carries (the staged
/// init + the hello ABI-bisect binary + the rfork/fd-ops smoke +
/// the notes smoke + the stack-growth smoke + the
/// interruptible-sleep smoke).
const INITRD_FILES: [&str; 6] = ["init", "hello", "smokefork", "smokenote", "stacksmoke", "smokesleep"];

impl InitrdStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        Self { verbose: verbose(matches) }
    }

    fn run(self) -> Result<()> {
        assemble_initrd(self.verbose)
    }
}

/// Assemble target/bootfs.paq from the Harvey SMOKE staging dir
/// (`build/aarch64/smoke-out/` — the exec-family
/// profiles need the smoke init's markers, and the boot chain's REAL init
/// shares the old `out/` path under "last writer wins"; the smoke stage
/// owns smoke-out/ exclusively, so the two gates can never clobber
/// each other).  Both producers
/// are named in the error text when absent — a missing mkpaqfs or a missing
/// staged binary is a loud build error, never a silent boot.
fn assemble_initrd(verbose: bool) -> Result<()> {
    // R9_HARVEY_DIR required — there is no default checkout path.
    let harvey = uregcheck::harvey_dir()?;
    let mkpaqfs = harvey.join(".build/bin/mkpaqfs");
    if !mkpaqfs.is_file() {
        return Err(format!(
            "mkpaqfs host binary missing at {} — build it in the Harvey checkout with \
             `nix develop -c sh build/aarch64/mkpaqfs-host.sh` (the 04-01 recipe)",
            mkpaqfs.display()
        )
        .into());
    }
    let ws = workspace();
    let tree = ws.join(INITRD_TREE);
    if tree.exists() {
        std::fs::remove_dir_all(&tree)?;
    }
    std::fs::create_dir_all(&tree)?;
    // The staged init binds #p at /proc itself in the
    // namespace era (the real chain's connectembed shape) — the mount
    // target must exist, so the tree ships the empty dir.
    std::fs::create_dir_all(tree.join("proc"))?;
    for name in INITRD_FILES {
        let src = harvey.join(format!("build/aarch64/smoke-out/{name}"));
        if !src.is_file() {
            return Err(format!(
                "staged binary {} missing — run `sh build/aarch64/stage-build.sh` in the Harvey \
                 checkout (04-05) to compile the aarch64 userland smoke stage into \
                 build/aarch64/smoke-out/",
                src.display()
            )
            .into());
        }
        std::fs::copy(&src, tree.join(name))?;
    }
    let paq = ws.join(BOOTFS_PAQ);
    let mut cmd = Command::new(&mkpaqfs);
    cmd.arg("-o").arg(&paq).arg(&tree);
    cmd.current_dir(&ws);
    if verbose {
        println!("Executing {cmd:?}");
    }
    let status = annotated_status(&mut cmd)?;
    if !status.success() {
        return Err("mkpaqfs failed".into());
    }
    // Evidence: print the image's sha256 (mkpaqfs embeds a time field, so the
    // digest is per-bake — the phase log records each evidence run's value).
    let sha = sha256_of(&paq)?;
    println!("initrd: {} sha256 {sha}", paq.display());
    Ok(())
}

/// sha256 of a file via a host tool — no new crates.  coreutils
/// `sha256sum` (nix devshell, linux) first, macOS `shasum` as fallback.
fn sha256_of(path: &Path) -> Result<String> {
    let out = Command::new("sha256sum")
        .arg(path)
        .output()
        .or_else(|_| Command::new("shasum").arg("-a").arg("256").arg(path).output())
        .map_err(|e| format!("no sha256 tool on PATH (tried sha256sum, shasum): {e}"))?;
    if !out.status.success() {
        return Err(format!("sha256 of {} failed: {}", path.display(), out.status).into());
    }
    let text = String::from_utf8(out.stdout)?;
    Ok(text.split_whitespace().next().unwrap_or("").to_string())
}

/// Build-if-missing/stale hook: `qemu`/`boot-test` invocations whose
/// config carries an initrd image under target/ (the assembled bootfs.paq)
/// reassemble it when absent or older than the staged inputs.  Configs with
/// no initrd, or with a target/-external image (the committed probe fixture —
/// pinned bytes), pass through untouched.
fn ensure_initrd_image(config: &Configuration, verbose: bool) -> Result<()> {
    let Some(qemu) = &config.qemu else { return Ok(()) };
    let Some(initrd) = &qemu.initrd else { return Ok(()) };
    // A harvey:-prefixed image is produced by the Harvey checkout's own
    // pipeline and consumed as-is: it passes through the target/-resident
    // auto-bake untouched, but absence is a loud error naming the producers
    // (07-05's virtnet profile boots the Harvey-baked bootfs.paq).
    if initrd.file.starts_with("harvey:") {
        let path = initrd.resolved_file()?;
        if !path.is_file() {
            return Err(format!(
                "initrd image {} not found at {} — produce it in the Harvey checkout with \
                 `sh build/aarch64/subset-build.sh` then `sh build/aarch64/mkimage.sh`",
                initrd.file,
                path.display()
            )
            .into());
        }
        return Ok(());
    }
    if !initrd.file.starts_with("target/") {
        return Ok(());
    }
    let paq = workspace().join(&initrd.file);
    // R9_HARVEY_DIR required — there is no default checkout path.
    let harvey = uregcheck::harvey_dir()?;
    let paq_mtime = std::fs::metadata(&paq).and_then(|m| m.modified()).ok();
    let stale = paq_mtime.is_some_and(|paq_mtime| {
        INITRD_FILES.iter().any(|name| {
            std::fs::metadata(harvey.join(format!("build/aarch64/smoke-out/{name}")))
                .and_then(|m| m.modified())
                .ok()
                .is_some_and(|input_mtime| input_mtime > paq_mtime)
        })
    });
    if paq_mtime.is_none() || stale {
        // assemble_initrd fails loudly with the producing command named when
        // mkpaqfs or the staged inputs are absent.
        println!("xtask: initrd {} missing or stale — assembling", initrd.file);
        assemble_initrd(verbose)?;
    }
    Ok(())
}

/// The Multiboot2 header constants the check validates against (spec
/// 3.1.1-3.1.2; mirrors GRUB's loader: MULTIBOOT2_SEARCH/HEADER_ALIGN).
const MB2_MAGIC: u32 = 0xE85250D6;
const MB2_SEARCH: usize = 32768;
const MB2_HEADER_ALIGN: usize = 8;

/// Post-build Multiboot2 header check (T-13-01 mitigation, D-02): parse
/// the built image's first 32 KiB exactly the way GRUB does — scan
/// 8-byte-aligned offsets for the magic, then validate the header
/// arithmetic — so the channel refuses to build an ISO around a kernel
/// GRUB would silently reject (a malformed header is a "not a multiboot
/// image" drop at boot, never a build error).  Returns the file offset
/// of the validated header.
fn check_mb2_header(bytes: &[u8]) -> std::result::Result<usize, String> {
    let window = &bytes[..bytes.len().min(MB2_SEARCH)];
    let read_u16 =
        |off: usize| u16::from_le_bytes([window[off], window[off + 1]]);
    let read_u32 =
        |off: usize| u32::from_le_bytes([window[off], window[off + 1], window[off + 2], window[off + 3]]);

    // GRUB's scan: the magic at an 8-byte-aligned offset within the first
    // 32768 bytes of the image.
    let mut off = 0;
    let header = loop {
        if off + 16 > window.len() {
            return Err(format!(
                "mb2 header check: magic {MB2_MAGIC:#x} not found at any 8-byte-aligned offset \
                 within the first {MB2_SEARCH} bytes (GRUB would reject the image: \"not a \
                 multiboot image\")"
            ));
        }
        if read_u32(off) == MB2_MAGIC {
            break off;
        }
        off += MB2_HEADER_ALIGN;
    };

    let arch = read_u32(header + 4);
    let header_length = read_u32(header + 8) as usize;
    let checksum = read_u32(header + 12);

    // Checksum arithmetic: magic + architecture + header_length + checksum
    // == 0 (mod 2^32), exactly the two's-complement sum l.S encodes.
    if MB2_MAGIC
        .wrapping_add(arch)
        .wrapping_add(header_length as u32)
        .wrapping_add(checksum)
        != 0
    {
        return Err(format!(
            "mb2 header check at {header:#x}: checksum arithmetic wrong (magic {MB2_MAGIC:#x} + \
             arch {arch:#x} + header_length {header_length:#x} + checksum {checksum:#x} != 0)"
        ));
    }
    // r9's header declares architecture 0 (i386 protected-mode entry).
    if arch != 0 {
        return Err(format!(
            "mb2 header check at {header:#x}: architecture {arch:#x} != 0 (i386)"
        ));
    }
    // The header (fixed part + tags + end tag) must be entirely within the
    // searched window and internally consistent.
    if header_length < 16 {
        return Err(format!(
            "mb2 header check at {header:#x}: header_length {header_length} < 16 (fixed part alone)"
        ));
    }
    if header + header_length > window.len() || header + header_length > MB2_SEARCH {
        return Err(format!(
            "mb2 header check at {header:#x}: header ({header_length} bytes) not entirely within \
             the first {MB2_SEARCH} bytes of the image"
        ));
    }

    // Tag walk from the fixed part's end: each tag is {u16 type, u16 flags,
    // u32 size}, 8-byte aligned (size includes padding), terminated by the
    // required end tag (type 0, flags 0, size 8) exactly at header_length.
    let mut tag = header + 16;
    loop {
        if tag + 8 > window.len() {
            return Err(format!(
                "mb2 header check at {header:#x}: tag walk runs off the image before the end tag"
            ));
        }
        let tag_type = read_u16(tag);
        let tag_flags = read_u16(tag + 2);
        let tag_size = read_u32(tag + 4) as usize;
        if tag_size < 8 || !tag_size.is_multiple_of(MB2_HEADER_ALIGN) {
            return Err(format!(
                "mb2 header check at {header:#x}: tag at {tag:#x} size {tag_size} (< 8 or not \
                 8-byte aligned)"
            ));
        }
        if tag_type == 0 {
            if tag_flags != 0 || tag_size != 8 {
                return Err(format!(
                    "mb2 header check at {header:#x}: end tag (flags {tag_flags}, size {tag_size}) \
                     malformed (flags 0, size 8 required)"
                ));
            }
            if tag + 8 != header + header_length {
                return Err(format!(
                    "mb2 header check at {header:#x}: end tag at {tag:#x} does not terminate the \
                     header exactly ({:#x} != header start + header_length {:#x})",
                    tag + 8,
                    header + header_length
                ));
            }
            return Ok(header);
        }
        tag += tag_size;
        if tag > header + header_length {
            return Err(format!(
                "mb2 header check at {header:#x}: tag walk overruns header_length before the end tag"
            ));
        }
    }
}

/// Build the x86_64 boot ISO for the D-02 channel: stage the freshly built
/// kernel ELF + the emitted grub.cfg under target/iso-<config>/, then run
/// grub-mkrescue (BIOS-only; no EFI image, so no mtools).  The post-build
/// mb2 header check runs FIRST — the channel refuses to build an ISO
/// around a kernel GRUB would silently reject (T-13-01).  A config with a
/// [dist] module key (14-05, D-03) stages the boot module at
/// /boot/bootfs.paq and emits the grub.cfg module2 placeholder line IN;
/// module-less configs stage no module and emit the line OUT — their
/// staged loader config stays byte-identical to Phase 13's.  Returns the
/// ISO path relative to the workspace (the qemu steps run with cwd =
/// workspace).
fn build_x86_64_iso(
    kernel_rel: &str,
    config_name: &str,
    config: &Configuration,
    verbose: bool,
) -> Result<String> {
    let kernel_abs = workspace().join(kernel_rel);
    if !kernel_abs.exists() {
        return Err(format!("kernel image {kernel_rel} missing (build step ran first?)").into());
    }

    // Stage boot/kernel + boot/grub/grub.cfg — the emitted grub.cfg's
    // `multiboot2 /boot/kernel` names the staged ELF at its fixed ISO path.
    let stage_rel = format!("target/iso-{config_name}");
    let stage = workspace().join(&stage_rel);
    if stage.exists() {
        std::fs::remove_dir_all(&stage)?;
    }
    std::fs::create_dir_all(stage.join("boot/grub"))?;

    // The staged kernel is a STRIPPED copy of the built ELF (13-03):
    // GRUB loads only the PT_LOAD segments, so debug info is pure load
    // weight through the BIOS-disk channel — under TCG an 18 MB debug
    // image took 20+ MINUTES to load (measured; KVM loads it in
    // seconds).  `objcopy --strip-debug` keeps every loadable segment
    // and the mb2 header byte-identical; the header check below runs
    // against the EXACT stripped bytes GRUB will scan, so the ISO
    // refuse-to-boot guarantee (13-01) is unchanged.
    let staged_kernel = stage.join("boot/kernel");
    let mut strip = Command::new("objcopy");
    strip.arg("--strip-debug").arg(&kernel_abs).arg(&staged_kernel);
    strip.current_dir(workspace());
    if verbose {
        println!("Executing {strip:?}");
    }
    let status = strip
        .status()
        .map_err(|e| format!("spawn objcopy (binutils in the devshell): {e}"))?;
    if !status.success() {
        return Err("objcopy --strip-debug failed".into());
    }
    let kernel_bytes = std::fs::read(&staged_kernel)
        .map_err(|e| format!("read stripped kernel: {e}"))?;
    let header = check_mb2_header(&kernel_bytes)?;
    println!(
        "mb2: header at file offset {header:#x} validated on the stripped ISO kernel \
         (magic/arch/length/checksum/end tag)"
    );

    // D-03's module staging: resolve + copy the boot module (if the
    // config names one) BEFORE the loader config is emitted, so the
    // emitted module2 line never references an absent file (GRUB fails
    // a menuentry whose module is missing — the kernel-only ISOs emit
    // no line for exactly that reason).
    let module_spec = config.dist.as_ref().and_then(|d| d.module.as_deref());
    let module_staged = match module_spec {
        Some(spec) => {
            let path = resolve_boot_module(spec)?;
            let dest = stage.join("boot/bootfs.paq");
            std::fs::copy(&path, &dest).map_err(|e| format!("stage module {path:?}: {e}"))?;
            println!("iso: module {} staged at /boot/bootfs.paq", path.display());
            true
        }
        None => false,
    };
    let template = std::fs::read_to_string(workspace().join("x86_64/lib/grub.cfg"))
        .map_err(|e| format!("read grub.cfg template: {e}"))?;
    std::fs::write(stage.join("boot/grub/grub.cfg"), emit_grub_cfg(&template, module_staged))?;

    let iso_rel = format!("target/boot-{config_name}.iso");
    let mut cmd = Command::new("grub-mkrescue");
    cmd.arg("-o").arg(&iso_rel).arg(&stage_rel);
    cmd.current_dir(workspace());
    if verbose {
        println!("Executing {cmd:?}");
    }
    let status = cmd.status().map_err(|e| {
        format!(
            "spawn grub-mkrescue: {e} — run under `nix develop` (or `nix shell nixpkgs#grub2 \
             nixpkgs#xorriso` until the devshell carries both)"
        )
    })?;
    if !status.success() {
        return Err("grub-mkrescue failed".into());
    }
    Ok(iso_rel)
}

/// Emit the staged loader config from the committed template (14-05,
/// D-03): `# @R9-MODULE-DOC@` lines are template documentation dropped by
/// every emitter; a line carrying the `# @R9-MODULE@ ` marker is the
/// module2 placeholder — emitted IN (the marker stripped, the line's
/// indentation and payload kept) when a boot module is staged, dropped
/// OUT for kernel-only ISOs.  The kernel-only emission is thereby
/// byte-identical to Phase 13's committed grub.cfg — the property the
/// wired boot-iso FOD hash rides on (the unit test pins it against the
/// exact 13-line config).  14-07's flake iso-builder shares the
/// convention.
fn emit_grub_cfg(template: &str, module_staged: bool) -> String {
    const MARKER: &str = "# @R9-MODULE@ ";
    let mut out = String::new();
    for line in template.lines() {
        if let Some(pos) = line.find(MARKER) {
            if module_staged {
                out.push_str(&line[..pos]);
                out.push_str(&line[pos + MARKER.len()..]);
                out.push('\n');
            }
        } else if line.trim_start().starts_with("# @R9-MODULE-DOC@") {
            continue;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Resolve a [dist] module path: a `harvey:`-prefixed value names a file
/// under the Harvey checkout (R9_HARVEY_DIR via uregcheck::harvey_dir —
/// the 07-05 initrd channel's discipline; r9 never hardcodes a sibling
/// layout); ordinary workspace-relative values pass through.  Absence is
/// a LOUD error naming the Harvey-side producers — never a silent
/// kernel-only ISO when the profile expects its bootfs.
fn resolve_boot_module(spec: &str) -> Result<PathBuf> {
    let path = match spec.strip_prefix("harvey:") {
        Some(rest) if !rest.is_empty() => {
            let dir = uregcheck::harvey_dir().map_err(|e| e.to_string())?;
            dir.join(rest)
        }
        Some(_) => {
            return Err(
                "dist module: bare `harvey:` prefix names no file within the Harvey checkout"
                    .into(),
            )
        }
        None => workspace().join(spec),
    };
    if !path.is_file() {
        return Err(format!(
            "boot module {spec} not found at {} — produce it in the Harvey checkout with \
             `sh build/x86_64/stage-build.sh` then `sh build/x86_64/mkimage.sh` (the 14-05 \
             bake; R9_HARVEY_DIR names the checkout)",
            path.display()
        )
        .into());
    }
    Ok(path)
}

struct UregCheckStep {}

impl UregCheckStep {
    fn new() -> Self {
        Self {}
    }

    fn run(self) -> Result<()> {
        uregcheck::run_check().map(|_| ())
    }
}

fn workspace() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR")).ancestors().nth(1).unwrap().to_path_buf()
}

/// Exclude architectures other than the one being built
fn exclude_other_arches(arch: Arch, cmd: &mut Command) {
    match arch {
        Arch::Aarch64 => {
            cmd.arg("--exclude").arg("riscv64");
            cmd.arg("--exclude").arg("x86_64");
        }
        Arch::Riscv64 => {
            cmd.arg("--exclude").arg("aarch64");
            cmd.arg("--exclude").arg("x86_64");
        }
        Arch::X86_64 => {
            cmd.arg("--exclude").arg("aarch64");
            cmd.arg("--exclude").arg("riscv64");
        }
    }
}

/// Annotates the error result with the calling binary's name.
fn annotated_status(cmd: &mut Command) -> Result<process::ExitStatus> {
    Ok(cmd.status().map_err(|e| format!("{}: {}", cmd.get_program().to_string_lossy(), e))?)
}

#[cfg(test)]
mod tests {
    use super::{
        BOOT_ECHO_PROBE, BOOT_Q35_MEMORY_MARKER, BOOT_TICK_MARKER, BOOT_TICK_MIN, check_mb2_header,
        contains_marker, count_marker_lines, count_register_lines, emit_grub_cfg,
        last_marker_line_end, marker_position, negative_scan_hit_for, PCI_NET_DEVICE_MARKER,
        Q35_MCFG_MARKER, Q35_PCIEXBAR_MARKER,
    };

    /// Build a synthetic image around one mb2 header (the fixture shape
    /// l.S emits: fixed part + end tag, header_length 24) at `prefix`
    /// bytes of leading padding, with every field overridable so the
    /// failure modes are one argument away.
    fn mb2_fixture(
        prefix: usize,
        magic: u32,
        arch: u32,
        checksum: u32,
        end_flags: u16,
        end_size: u32,
    ) -> Vec<u8> {
        let mut v = vec![0u8; prefix];
        v.extend_from_slice(&magic.to_le_bytes());
        v.extend_from_slice(&arch.to_le_bytes());
        v.extend_from_slice(&24u32.to_le_bytes()); // header_length (fixed + end tag)
        v.extend_from_slice(&checksum.to_le_bytes());
        v.extend_from_slice(&0u16.to_le_bytes()); // end tag type 0
        v.extend_from_slice(&end_flags.to_le_bytes());
        v.extend_from_slice(&end_size.to_le_bytes());
        v.extend_from_slice(&[0u8; 64]); // trailing image bytes
        v
    }

    fn valid_checksum() -> u32 {
        0xE85250D6u32.wrapping_add(0).wrapping_add(24).wrapping_neg()
    }

    /// The l.S-shaped header validates at offset 0 and at an 8-byte-aligned
    /// offset past the ELF-hack padding — the check mirrors GRUB's scan,
    /// not a fixed offset.
    #[test]
    fn mb2_header_valid_fixture_passes() {
        let img = mb2_fixture(0, 0xE85250D6, 0, valid_checksum(), 0, 8);
        assert_eq!(check_mb2_header(&img), Ok(0));
        let img = mb2_fixture(16, 0xE85250D6, 0, valid_checksum(), 0, 8);
        assert_eq!(check_mb2_header(&img), Ok(16));
    }

    /// No magic in the window (the silent-GRUB-rejection case) is a loud
    /// error naming the scan GRUB performs.
    #[test]
    fn mb2_header_missing_magic_fails() {
        let img = mb2_fixture(0, 0x1BADB002, 0, valid_checksum(), 0, 8);
        assert!(check_mb2_header(&img).is_err());
        assert!(check_mb2_header(&[0u8; 128]).is_err());
    }

    /// Checksum arithmetic, architecture and the end-tag shape are each
    /// validated — one broken field per fixture.
    #[test]
    fn mb2_header_arithmetic_and_end_tag_fails() {
        let bad_sum = valid_checksum().wrapping_add(1);
        assert!(check_mb2_header(&mb2_fixture(0, 0xE85250D6, 0, bad_sum, 0, 8)).is_err());
        // architecture 4 (MIPS) is not r9's i386 entry
        let mips_sum = 0xE85250D6u32.wrapping_add(4).wrapping_add(24).wrapping_neg();
        assert!(check_mb2_header(&mb2_fixture(0, 0xE85250D6, 4, mips_sum, 0, 8)).is_err());
        // end tag flags/size must be 0/8
        assert!(check_mb2_header(&mb2_fixture(0, 0xE85250D6, 0, valid_checksum(), 1, 8)).is_err());
        assert!(check_mb2_header(&mb2_fixture(0, 0xE85250D6, 0, valid_checksum(), 0, 16)).is_err());
    }

    /// A header whose declared header_length disagrees with the end tag's
    /// position fails (the walk must terminate exactly at header_length).
    #[test]
    fn mb2_header_length_mismatch_fails() {
        let mut img = mb2_fixture(0, 0xE85250D6, 0, valid_checksum(), 0, 8);
        // header_length 32 (checksum then also wrong) — the arithmetic
        // fires first; make it arithmetic-consistent but walk-inconsistent.
        img[8..12].copy_from_slice(&32u32.to_le_bytes());
        img[12..16].copy_from_slice(
            &0xE85250D6u32.wrapping_add(0).wrapping_add(32).wrapping_neg().to_le_bytes(),
        );
        assert!(check_mb2_header(&img).is_err());
    }

    /// The always-on negative scan: panic/Unhandled-interrupt lines fail
    /// non-panic profiles, case-insensitively.
    #[test]
    fn negative_scan_hits_panic_and_unhandled() {
        assert_eq!(negative_scan_hit_for(b"ok\r\nkernel panic: bad\r\n", false), Some("panic"));
        assert_eq!(negative_scan_hit_for(b"PANIC\r\n", false), Some("panic"));
        assert_eq!(
            negative_scan_hit_for(b"blah\r\nUnhandled interrupt\r\n", false),
            Some("Unhandled interrupt")
        );
        assert_eq!(
            negative_scan_hit_for(b"unhandled interrupt\r\n", false),
            Some("Unhandled interrupt")
        );
    }

    /// 14-05 (D-03): the kernel-only emission of the COMMITTED grub.cfg
    /// template is byte-identical to Phase 13's staged loader config —
    /// the property the wired boot-iso FOD hash rides on.  Reads the
    /// real committed file (never a synthetic copy): any template edit
    /// that leaks a `# @R9-MODULE*` line into a kernel-only ISO fails
    /// HERE, before any boot.
    #[test]
    fn grub_cfg_kernel_only_emission_is_phase13_identical() {
        let template = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../x86_64/lib/grub.cfg"),
        )
        .expect("committed grub.cfg template readable");
        let emitted = emit_grub_cfg(&template, false);
        // Phase 13's exact staged config (13 lines + trailing newline).
        let phase13 = concat!(
            "# x86_64/lib/grub.cfg — the D-02 boot chain's loader config (13-01).\n",
            "# Staged by xtask's ISO build at <iso>/boot/grub/grub.cfg next to the\n",
            "# kernel ELF at <iso>/boot/kernel.  Serial console first: GRUB's own\n",
            "# errors (\"no multiboot header found\" and friends) land on the same\n",
            "# wire the boot-test gate asserts the kernel banner on — a wedged\n",
            "# channel is visible on the socket, never silent.\n",
            "serial --unit=0 --speed=115200\n",
            "terminal_output serial\n",
            "set timeout=0\n",
            "menuentry \"r9\" {\n",
            "    multiboot2 /boot/kernel\n",
            "    boot\n",
            "}\n",
        );
        assert_eq!(emitted, phase13);
    }

    /// 14-05 (D-03): the module-bearing emission carries the module2
    /// line IN — the placeholder's indentation and payload kept, the
    /// marker stripped — and every template documentation line dropped.
    #[test]
    fn grub_cfg_module_emission_carries_the_module2_line() {
        let template = concat!(
            "# head comment\n",
            "# @R9-MODULE-DOC@ documentation line dropped\n",
            "serial --unit=0 --speed=115200\n",
            "menuentry \"r9\" {\n",
            "    multiboot2 /boot/kernel\n",
            "    # @R9-MODULE@ module2 /boot/bootfs.paq\n",
            "    boot\n",
            "}\n",
        );
        let emitted = emit_grub_cfg(&template, true);
        assert_eq!(
            emitted,
            concat!(
                "# head comment\n",
                "serial --unit=0 --speed=115200\n",
                "menuentry \"r9\" {\n",
                "    multiboot2 /boot/kernel\n",
                "    module2 /boot/bootfs.paq\n",
                "    boot\n",
                "}\n",
            )
        );
        // Kernel-only over the same synthetic template: the placeholder
        // and the doc line both drop, everything else keeps.
        assert_eq!(
            emit_grub_cfg(&template, false),
            concat!(
                "# head comment\n",
                "serial --unit=0 --speed=115200\n",
                "menuentry \"r9\" {\n",
                "    multiboot2 /boot/kernel\n",
                "    boot\n",
                "}\n",
            )
        );
    }

    #[test]
    fn negative_scan_passes_clean_captures() {
        assert_eq!(negative_scan_hit_for(b"r9 from the Internet\r\n", false), None);
        assert_eq!(negative_scan_hit_for(b"hello aarch64 harvey\r\n", false), None);
        assert_eq!(negative_scan_hit_for(b"", false), None);
    }

    /// The scan's Pitfall-N10 arm: the stray-IRQ line's lowercase shape is
    /// a hit (once 07-06 arms the NIC's IRQ source, a stray interrupt is a
    /// bind-ordering bug, not a cosmetic).
    #[test]
    fn negative_scan_hits_stray_irq_line() {
        assert_eq!(negative_scan_hit_for(b"unhandled IRQ 36\r\n", false), Some("unhandled IRQ"));
        assert_eq!(negative_scan_hit_for(b"unhandled irq 36\r\n", true), Some("unhandled IRQ"));
    }

    /// The Net profile exempts exactly the D-02 designed fauth line (the
    /// real Harvey chain prints `syscall 10 unimplemented` twice a boot);
    /// any OTHER unimplemented syscall still fails the run.
    #[test]
    fn negative_scan_net_exempts_fauth_only() {
        assert_eq!(negative_scan_hit_for(b"syscall 10 unimplemented\r\n", true), None);
        assert_eq!(
            negative_scan_hit_for(b"syscall 10 unimplemented\r\n", false),
            Some("unimplemented syscall")
        );
        assert_eq!(
            negative_scan_hit_for(
                b"syscall 10 unimplemented\r\nsyscall 99 unimplemented\r\n",
                true
            ),
            Some("unimplemented syscall")
        );
    }

    /// Serial lines arrive CRLF (console putb maps \n to \r\n); the tick
    /// count must see exactly the line-initial markers.
    #[test]
    fn count_marker_lines_counts_line_initial_only() {
        let capture = b"banner\r\ntick 0\r\ntick 1\r\nuntickled\r\ntick 2\r\n";
        assert_eq!(count_marker_lines(capture, BOOT_TICK_MARKER), 3);
    }

    #[test]
    fn count_marker_lines_ignores_mid_line_and_missing() {
        assert_eq!(count_marker_lines(b"notick here\r\n", BOOT_TICK_MARKER), 0);
        assert_eq!(count_marker_lines(b"", BOOT_TICK_MARKER), 0);
        // A marker directly after \n with no \r still counts (line start).
        assert_eq!(count_marker_lines(b"tick 0\n", BOOT_TICK_MARKER), 1);
    }

    /// The X64 facts stage's ordered-presence anchor (13-02): the q35
    /// memory lines count only STRICTLY PAST the banner's byte offset —
    /// a fact line that rode the wire before the banner can never
    /// satisfy the stage.
    #[test]
    fn q35_memory_facts_count_only_after_the_banner() {
        let wire = b"q35: memory 0x0..0x9fc00 (0x9fc00) available\r\n\
                     r9 from the Internet\r\n\
                     q35: memory 0x100000..0x3ffdf000 available\r\n\
                     q35: acpi rsdt 0x3ffe2345\r\n";
        let after = marker_position(wire, b"r9 from the Internet")
            .map(|p| p + "r9 from the Internet".len())
            .unwrap_or(wire.len());
        // Before the anchor: the pre-banner fact line exists but the
        // whole-wire count would over-credit; past the anchor exactly
        // the one post-banner memory line counts.
        assert_eq!(count_marker_lines(wire, BOOT_Q35_MEMORY_MARKER), 2);
        assert_eq!(count_marker_lines(&wire[after..], BOOT_Q35_MEMORY_MARKER), 1);
        // The acpi fact line shares the `q35: ` prefix but not the
        // memory marker's line prefix.
        assert_eq!(count_marker_lines(&wire[after..], "q35: acpi"), 1);
        assert!(marker_position(b"no marker here", BOOT_Q35_MEMORY_MARKER.as_bytes()).is_none());
    }

    /// The X64 gate's discovery stage (13-04) demands the MCFG parse,
    /// the pciexbar audit, and the device-found markers only past the
    /// facts stage's anchor: lines that rode the wire before the facts
    /// were observed never satisfy the stage, and any missing marker
    /// keeps it red (X64-03's presence contract).
    #[test]
    fn x64_discovery_markers_count_only_past_the_facts() {
        let wire = b"r9 from the Internet\r\n\
                     q35: mcfg 0xb0000000 buses 0x0-0xff (seg 0)\r\n\
                     q35: memory 0x100000..0x3ffdf000 available\r\n\
                     q35: ioapic 0xfec00000 gsib 0 (id 8)\r\n\
                     q35: mcfg 0xb0000000 buses 0x0-0xff (seg 0)\r\n\
                     pciexbar: 0xb0000001 -> 0xb0000001\r\n\
                     pci: virtio-net 1af4:1041 at 00:02.0 rev 01 pin 1 (inta) bars 2\r\n";
        let facts_after = marker_position(wire, b"q35: memory")
            .map(|p| p + "q35: memory 0x100000..0x3ffdf000 available\r\n".len())
            .unwrap_or(wire.len());
        // The pre-facts mcfg line never satisfies the stage: the anchor
        // starts at the facts stage's satisfaction, where all three
        // markers must appear.
        assert!(contains_marker(wire, Q35_MCFG_MARKER.as_bytes()));
        assert!(contains_marker(&wire[facts_after..], Q35_MCFG_MARKER.as_bytes()));
        assert!(contains_marker(&wire[facts_after..], Q35_PCIEXBAR_MARKER.as_bytes()));
        assert!(contains_marker(&wire[facts_after..], PCI_NET_DEVICE_MARKER.as_bytes()));
        // The stage is all-three: a wire missing the audit line (the
        // programming step silently skipped) stays red.
        let no_exbar = b"q35: memory 0x100000..0x3ffdf000 available\r\n\
                         q35: mcfg 0xb0000000 buses 0x0-0xff\r\n\
                         pci: virtio-net 1af4:1041 at 00:02.0 rev 01\r\n";
        assert!(!contains_marker(no_exbar, Q35_PCIEXBAR_MARKER.as_bytes()));
        // And the device line is vendor-exact: a transitional reject
        // print (0x1000) is not the found marker.
        let rejected = b"pci: virtio-net 1af4:1000 at 00:02.0 rejected (transitional)\r\n";
        assert!(!contains_marker(rejected, PCI_NET_DEVICE_MARKER.as_bytes()));
    }

    /// The discovery stage's anchor is POSITIONAL (13-04): one burst
    /// carries banner+facts+discovery together, so the anchor must sit
    /// just past the LAST memory line — not at the capture length the
    /// stage transition observed (which already includes the burst's
    /// discovery lines and would deadlock the stage).
    #[test]
    fn discovery_anchor_is_the_last_memory_line_not_capture_length() {
        // One burst: the whole machine-info block arrives together.
        let burst = b"r9 from the Internet\r\n\
                      q35: memory 0x0..0x9fc00 available\r\n\
                      q35: acpi rsdt 0x3ffe2345 via mb2 acpi tag\r\n\
                      q35: mcfg 0xb0000000 buses 0x0-0xff\r\n\
                      pciexbar: 0xb0000001 -> 0xb0000001\r\n\
                      pci: virtio-net 1af4:1041 at 00:02.0\r\n";
        let anchor = last_marker_line_end(burst, BOOT_Q35_MEMORY_MARKER).unwrap();
        // A capture-length anchor (burst.len()) would exclude every
        // discovery line from its own stage; the positional anchor sits
        // just past the memory line, before the discovery lines.
        assert!(anchor < burst.len());
        assert!(contains_marker(&burst[anchor..], Q35_MCFG_MARKER.as_bytes()));
        assert!(contains_marker(&burst[anchor..], Q35_PCIEXBAR_MARKER.as_bytes()));
        assert!(contains_marker(&burst[anchor..], PCI_NET_DEVICE_MARKER.as_bytes()));
        // And a discovery line BEFORE the last memory line never counts.
        let shuffled = b"pciexbar: 0xb0000001 -> 0xb0000001\r\n\
                         q35: memory 0x0..0x9fc00 available\r\n";
        let anchor = last_marker_line_end(shuffled, BOOT_Q35_MEMORY_MARKER).unwrap();
        assert!(!contains_marker(&shuffled[anchor..], Q35_PCIEXBAR_MARKER.as_bytes()));
        // No memory line at all: None.
        assert!(last_marker_line_end(b"banner\r\n", BOOT_Q35_MEMORY_MARKER).is_none());
    }

    /// The X64 gate's tick stage (13-03) counts `tick` lines only past
    /// the facts stage's anchor: a tick line that rode the wire before
    /// the facts were observed never satisfies the stage, and the aarch64
    /// profiles' whole-capture count (anchor 0) is unchanged.
    #[test]
    fn x64_tick_lines_count_only_past_the_facts() {
        let wire = b"r9 from the Internet\r\n\
                     tick 0\r\n\
                     q35: memory 0x100000..0x3ffdf000 available\r\n\
                     q35: acpi rsdt 0x3ffe2345\r\n\
                     tick 0\r\n\
                     tick 1\r\n\
                     tick 2\r\n";
        // The facts anchor sits just past the banner; the ticks anchor
        // sits just past the last fact line (the stage sets it to the
        // capture length at satisfaction time).
        let facts_after = marker_position(wire, b"r9 from the Internet")
            .map(|p| p + "r9 from the Internet".len())
            .unwrap_or(wire.len());
        let facts_end = marker_position(wire, b"q35: acpi rsdt")
            .map(|p| p + "q35: acpi rsdt 0x3ffe2345\r\n".len())
            .unwrap_or(wire.len());
        // Whole wire: the pre-facts tick line would over-credit (4 >=
        // BOOT_TICK_MIN) — and past the BANNER alone it still counts
        // (the tick line between banner and facts), which is why the
        // ticks stage carries its own anchor past the FACTS.  Past that
        // anchor exactly the 3 post-facts lines count, so the ordered
        // stage still demands 3 real ticks.
        assert_eq!(count_marker_lines(wire, BOOT_TICK_MARKER), 4);
        assert_eq!(count_marker_lines(&wire[facts_after..], BOOT_TICK_MARKER), 4);
        assert_eq!(count_marker_lines(&wire[facts_end..], BOOT_TICK_MARKER), 3);
        // The aarch64 shape: anchor 0 counts everything.
        assert_eq!(count_marker_lines(&wire[0..], BOOT_TICK_MARKER), 4);
        // Fewer than 3 post-anchor lines never satisfies the stage:
        // from the second post-facts tick onward only 2 remain.
        let tick1 = marker_position(wire, b"tick 1").unwrap();
        assert_eq!(count_marker_lines(&wire[tick1..], BOOT_TICK_MARKER), 2);
        assert!(count_marker_lines(&wire[tick1..], BOOT_TICK_MARKER) < BOOT_TICK_MIN);
    }

    #[test]
    fn contains_marker_searches_substrings() {
        assert!(contains_marker(b"xxr9echo7yy", BOOT_ECHO_PROBE));
        assert!(!contains_marker(b"r9echo", BOOT_ECHO_PROBE));
        assert!(!contains_marker(b"", BOOT_ECHO_PROBE));
    }

    /// The echo assertion searches only bytes captured AFTER the probe write,
    /// so a probe string that never came back cannot satisfy it.
    #[test]
    fn echo_window_is_post_write_only() {
        let before = b"tick 0\r\n";
        let mut capture = before.to_vec();
        let probe_at = capture.len();
        assert!(!contains_marker(&capture[probe_at..], BOOT_ECHO_PROBE));
        capture.extend_from_slice(b"r9echo7");
        assert!(contains_marker(&capture[probe_at..], BOOT_ECHO_PROBE));
    }

    /// The panic profile counts TrapFrame {:#?} dump rows: space-indented
    /// `x<N>:` lines, CRLF-terminated on the serial wire.  Bare register
    /// names are the contract (trap.rs prints x0..x30 without annotations).
    #[test]
    fn count_register_lines_counts_trapframe_rows() {
        let capture = b"TrapFrame {\r\n    x0: 0x0000000000000000,\r\n    x30: 0xffff8000400123f0,\r\n    esr_el1: EsrEl1 {\r\n        iss: 0x00000000,\r\n    },\r\n";
        assert_eq!(count_register_lines(capture), 2);
    }

    #[test]
    fn count_register_lines_rejects_annotated_unindented_and_xless() {
        // The retired annotated names ("x29 (frame_pointer):") must NOT match
        // -- the >= 30 row gate sees x0..x30 only with bare names, which is
        // why trap.rs carries them.
        assert_eq!(count_register_lines(b"    x29 (frame_pointer): 0x0,\r\n"), 0);
        assert_eq!(count_register_lines(b"x0: 0x0\r\n"), 0);
        assert_eq!(count_register_lines(b"    x: 0x0\r\n"), 0);
        assert_eq!(count_register_lines(b"    esr_el1: 0x0\r\n"), 0);
        assert_eq!(count_register_lines(b""), 0);
    }
}
