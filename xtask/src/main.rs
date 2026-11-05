use crate::config::Configuration;
use config::{
    QemuOverrides, apply_to_build_step, apply_to_clippy_step, apply_to_qemu_step_with_overrides,
    apply_virt_netdev, prepare_initrd_dtb, virt_resolved,
};
use std::{
    env, fmt,
    io::{Read, Write},
    os::unix::{net::UnixStream, process::CommandExt},
    path::{Path, PathBuf},
    process::{self, Command},
    str::FromStr,
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
                "Verify the Harvey checkout's aarch64/include/ureg.h matches the pinned \
                 TrapFrame layout (D-06 layer 2; R9_HARVEY_DIR names the checkout — required \
                 since 05-10, 05-CONTEXT D-10)",
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
                let mut cmd = Command::new(objcopy());
                cmd.arg("--input-target=elf64-x86-64");
                cmd.arg("--output-target=elf32-i386");
                cmd.arg(format!("target/{}/{}/x86_64", self.arch.target(), self.profile.dir()));
                cmd.arg(format!("target/{}/{}/r9.elf32", self.arch.target(), self.profile.dir()));
                cmd.current_dir(workspace());
                if self.verbose {
                    println!("Executing {cmd:?}");
                }
                let status = annotated_status(&mut cmd)?;
                if !status.success() {
                    return Err("objcopy failed".into());
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
                let mut cmd = Command::new(qemu_system);
                cmd.arg("-nographic");
                // cmd.arg("-display");
                // cmd.arg("curses");
                if self.kvm {
                    cmd.arg("-accel").arg("kvm");
                    cmd.arg("-cpu").arg("host,pdpe1gb,xsaveopt,fsgsbase,apic,msr");
                } else {
                    cmd.arg("-M").arg("q35");
                    cmd.arg("-cpu").arg("qemu64,pdpe1gb,xsaveopt,fsgsbase,apic,msr");
                }
                cmd.arg("-smp");
                cmd.arg("8");
                cmd.arg("-s");
                cmd.arg("-m");
                cmd.arg("8192");
                if self.wait_for_gdb {
                    cmd.arg("-s").arg("-S");
                }
                //cmd.arg("-device");
                //cmd.arg("ahci,id=ahci0");
                //cmd.arg("-drive");
                //cmd.arg("id=sdahci0,file=sdahci0.img,if=none");
                //cmd.arg("-device");
                //cmd.arg("ide-hd,drive=sdahci0,bus=ahci0.0");
                cmd.arg("-kernel");
                cmd.arg(format!("target/{target}/{dir}/r9.elf32"));
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

/// The netip sweep's mode: R9_NETIP_SWEEP selects the leg set — `loopback`
/// (the 08-04 ping + udp echo legs) or `tcp` (the 08-05 tcp/7 echo +
/// stats-honesty legs).  A typo'd value is a harness bug, not a gate
/// failure — validated before qemu spawns, never after a wasted boot.
fn netip_sweep_mode() -> Result<String> {
    let mode = env::var("R9_NETIP_SWEEP").unwrap_or_else(|_| "loopback".to_string());
    match mode.as_str() {
        "loopback" | "tcp" => Ok(mode),
        _ => Err(format!("R9_NETIP_SWEEP must be \"loopback\" or \"tcp\", got {mode:?}").into()),
    }
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
        if self.arch != Arch::Aarch64 {
            return Err("boot-test currently supports aarch64 only".into());
        }

        // The cross-repo ureg.h drift gate runs before qemu
        // spawns — a drifted (or absent) Harvey checkout fails every
        // boot-test profile (an ABI mismatch is a build failure, not a
        // runtime mystery).  Bare `cargo xtask build` instead tolerates a
        // missing checkout — the const asserts still guard there.
        uregcheck::check_for_boot_test()?;

        // The virtnet profile's sweep mode is validated BEFORE qemu
        // spawns — a typo'd R9_NET_SWEEP_MODE is a harness bug, not a gate
        // failure, and a wasted TCG boot proves nothing.
        if self.config_name == "virtnet" {
            net_sweep_mode()?;
        }
        // Same discipline for the virtip profile (R9_NETIP_SWEEP, 08-04).
        if self.config_name == "virtip" {
            netip_sweep_mode()?;
        }

        let target = self.arch.target();
        let dir = self.profile.dir();
        // On virt the cargo-built ELF is the -kernel directly.
        let kernel = format!("target/{target}/{dir}/aarch64");
        let kernel_path = workspace().join(&kernel);
        if !kernel_path.exists() {
            return Err(format!("kernel image {kernel} missing (build step ran first?)").into());
        }

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

        // Spawn qemu with the serial console on the socket chardev.  The
        // machine/cpu/smp/memory/accel/dtb plumbing is the resolved config's,
        // via the same apply path QemuStep uses.
        let mut cmd = Command::new(self.arch.qemu_system());
        apply_to_qemu_step_with_overrides(&mut cmd, &config, &overrides);
        // D-09/D-10: every aarch64 virt boot carries the idle virtio-net NIC
        // + a fresh per-run pcap capture of its netdev (the gate evidence
        // channel; no kernel code probes the device this plan).
        apply_virt_netdev(&mut cmd, &config, &pcap_rel);
        cmd.arg("-display");
        cmd.arg("none");
        cmd.arg("-chardev");
        cmd.arg(format!("socket,id=cons,path={sock_rel},server=on,wait=on"));
        cmd.arg("-serial");
        cmd.arg("chardev:cons");
        cmd.arg("-kernel");
        cmd.arg(&kernel);
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

        let result = self.assert_markers(&mut child, &sock_abs, &log_abs);

        // Always reap the guest (by process group) and drop the
        // socket file.  One teardown site covers the pass, fail, and
        // timeout paths: assert_markers always returns here.
        kill_process_group(&mut child);
        let _ = std::fs::remove_file(&sock_abs);

        result
    }

    /// Connect, capture, and assert the ordered markers.  Writes every
    /// captured byte to the evidence log at target/boot-test-<config>.log.
    fn assert_markers(
        &self,
        child: &mut process::Child,
        sock_abs: &Path,
        log_abs: &Path,
    ) -> Result<()> {
        let timeout_secs: u64 = env::var("R9_BOOT_TEST_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(BOOT_TEST_TIMEOUT_SECS);
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);

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
            match stage {
                BootStage::Banner => {
                    if contains_marker(&capture, BOOT_BANNER_MARKER.as_bytes()) {
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
                BootStage::Ticks => {
                    if count_marker_lines(&capture, BOOT_TICK_MARKER) >= BOOT_TICK_MIN {
                        // Ticks prove IRQs are live; type the probe now.
                        stream.write_all(BOOT_ECHO_PROBE)?;
                        probe_at = capture.len();
                        stage = BootStage::Echo;
                    }
                }
                BootStage::Echo => {
                    if contains_marker(&capture[probe_at..], BOOT_ECHO_PROBE) {
                        return self.finish(log_abs, &capture, BootProfile::Ticks);
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
                        let profile = if notes_profile {
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
                            // netip-loopback.rc; 08-05: netip-tcp.rc).
                            let mode = netip_sweep_mode()?;
                            let fixture = match mode.as_str() {
                                "tcp" => "rc /lib/netip-tcp.rc\r",
                                _ => "rc /lib/netip-loopback.rc\r",
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
            BootProfile::Exec => {
                println!(
                    "boot-test: PASS -- banner, exec entry, init print, hello print and all-exited markers observed in order, stacksmoke depth-64 marker present (SC#2 + 05-01 D-15)"
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
                let mut markers: Vec<&str> = vec![DEVIP_ATTACH_MARKER, DEVIP_MEDIUM_MARKER];
                match mode.as_str() {
                    "tcp" => {
                        markers.push(NETIP_TCP_OK_MARKER);
                        markers.push(NETIP_TCP_STATS_OK_MARKER);
                    }
                    _ => {
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
        BOOT_ECHO_PROBE, BOOT_TICK_MARKER, contains_marker, count_marker_lines,
        count_register_lines, negative_scan_hit_for,
    };

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
