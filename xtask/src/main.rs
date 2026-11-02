use crate::config::Configuration;
use config::{
    QemuOverrides, apply_to_build_step, apply_to_clippy_step, apply_to_qemu_step_with_overrides,
};
use std::{
    env, fmt,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    process::{self, Command},
    str::FromStr,
    time::{Duration, Instant},
};
use target_lexicon::Triple;

mod config;

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
                // D-05: [dist] format = "elf" — the cargo-built ELF is the
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
        let profile = Profile::from(matches);
        let wait_for_gdb = matches.get_flag("gdb");
        let kvm = matches.get_flag("kvm");
        let dump_dtb: String = matches
            .try_get_one::<String>("dump_dtb")
            .ok()
            .flatten()
            .unwrap_or(&"".to_string())
            .clone();
        // D-06 override chain, per-invocation layer: the CLI flag beats the
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

        Self { arch, config, profile, wait_for_gdb, kvm, dump_dtb, accel, cpu, verbose }
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
                let mut cmd = Command::new(qemu_system);

                let platform = self.config.config.as_ref().and_then(|c| c.platform.as_deref());
                if platform == Some("virt") {
                    // virt: the D-06 chain completes here — per-invocation
                    // override (--accel/--cpu flag > R9_ACCEL/R9_CPU env) >
                    // TOML > built-in default. The default is tcg +
                    // cortex-a72, NEVER hvf: darwin acceptance runs TCG this
                    // phase because HVF rejects GICv2 on qemu 11.1.2 (research
                    // P2); the hvf accel exists only for a future GICv3 driver.
                    let toml = self.config.qemu.as_ref();
                    let accel = self
                        .accel
                        .clone()
                        .or_else(|| toml.and_then(|q| q.accel.clone()))
                        .unwrap_or_else(|| "tcg".to_string());
                    let cpu = self
                        .cpu
                        .clone()
                        .or_else(|| toml.and_then(|q| q.cpu.clone()))
                        .unwrap_or_else(|| "cortex-a72".to_string());
                    let overrides = QemuOverrides { accel: Some(accel), cpu: Some(cpu) };
                    apply_to_qemu_step_with_overrides(&mut cmd, &self.config, &overrides);

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
                    // D-05: the cargo-built ELF is the -kernel directly
                    // (DistStep skips the objcopy/gzip flat-image step).
                    cmd.arg("-kernel");
                    cmd.arg(format!("target/{target}/{dir}/aarch64"));
                } else {
                    // raspi: flag/env overrides still beat the TOML value if
                    // supplied, but with none set nothing new is emitted — the
                    // existing invocation shape stays byte-identical.
                    let overrides =
                        QemuOverrides { accel: self.accel.clone(), cpu: self.cpu.clone() };
                    apply_to_qemu_step_with_overrides(&mut cmd, &self.config, &overrides);

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

/// `cargo xtask boot-test` (D-11): build the kernel, boot it under qemu with
/// the serial console on a unix socket, assert the ordered boot markers
/// (banner -> >=3 periodic ticks -> typed-char echo), and write the captured
/// serial log.  `server=on,wait=on` on the socket chardev holds guest start
/// until we connect, giving capture-before-start for free (the Phase 1
/// lesson; the socket chardev drops pre-connect data without it).
///
/// Zero new crate dependencies: std::os::unix::net::UnixStream + std::process
/// only (rexpect/expectrl remain the documented D-11 fallback ONLY if this
/// socket design proves infeasible).  Marker assertions only, never timings
/// (research P11: TCG speed varies).
struct BootTestStep {
    arch: Arch,
    config: Configuration,
    config_name: String,
    profile: Profile,
    accel: Option<String>,
    cpu: Option<String>,
    verbose: bool,
}

/// Assertion markers, named so 03-06's panic profile and the evidence task
/// reuse them.
const BOOT_BANNER_MARKER: &str = "r9 from the Internet";
const BOOT_TICK_MARKER: &str = "tick";
const BOOT_TICK_MIN: usize = 3;
/// The probe string typed into the serial socket.  qemu's socket serial does
/// not self-echo, so these bytes can only reappear via the kernel's INTID-33
/// RX echo path (SC#4).
const BOOT_ECHO_PROBE: &[u8] = b"r9echo7";
/// Overall wall-clock budget for the whole marker sequence (overridable via
/// R9_BOOT_TEST_TIMEOUT); one total budget, never per-marker timings (P11).
const BOOT_TEST_TIMEOUT_SECS: u64 = 300;

/// Panic-profile markers (03-06, D-12): a config with dev = ["panic"] boots
/// virt to a deliberate brk #0 sync exception BEFORE trap::unmask_irq(), so
/// the log is deterministic (zero IRQ interleaving) -- banner, start marker,
/// full TrapFrame register dump, clean-halt marker.  No ticks, no echo:
/// IRQs never unmask in this profile.
const BOOT_PANIC_START_MARKER: &str = "virtpanic: deliberate sync exception";
const BOOT_PANIC_HALT_MARKER: &str = "kernel halted";
/// The TrapFrame {:#?} Debug prints one `    x<N>:` row per register x0..x30.
const BOOT_PANIC_REGISTER_MIN: usize = 30;

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
        }
    }
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
        // D-06 override chain, per-invocation layer: the CLI flag beats the
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

        let target = self.arch.target();
        let dir = self.profile.dir();
        // D-05: on virt the cargo-built ELF is the -kernel directly.
        let kernel = format!("target/{target}/{dir}/aarch64");
        let kernel_path = workspace().join(&kernel);
        if !kernel_path.exists() {
            return Err(format!("kernel image {kernel} missing (build step ran first?)").into());
        }

        let sock_rel = format!("target/boot-test-{}.sock", self.config_name);
        let log_rel = format!("target/boot-test-{}.log", self.config_name);
        let sock_abs = workspace().join(&sock_rel);
        let log_abs = workspace().join(&log_rel);
        // A stale socket file from a crashed run would break qemu's bind.
        let _ = std::fs::remove_file(&sock_abs);

        // Spawn qemu with the serial console on the socket chardev.  The
        // machine/cpu/smp/memory/accel/dtb plumbing is the config's, via the
        // same apply path QemuStep uses; the virt arm's built-in defaults
        // (tcg + cortex-a72, NEVER hvf -- research P2) complete the chain.
        let mut cmd = Command::new(self.arch.qemu_system());
        let platform = self.config.config.as_ref().and_then(|c| c.platform.as_deref());
        let toml = self.config.qemu.as_ref();
        let overrides = if platform == Some("virt") {
            QemuOverrides {
                accel: Some(
                    self.accel
                        .clone()
                        .or_else(|| toml.and_then(|q| q.accel.clone()))
                        .unwrap_or_else(|| "tcg".to_string()),
                ),
                cpu: Some(
                    self.cpu
                        .clone()
                        .or_else(|| toml.and_then(|q| q.cpu.clone()))
                        .unwrap_or_else(|| "cortex-a72".to_string()),
                ),
            }
        } else {
            QemuOverrides { accel: self.accel.clone(), cpu: self.cpu.clone() }
        };
        apply_to_qemu_step_with_overrides(&mut cmd, &self.config, &overrides);
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
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("{}: {}", cmd.get_program().to_string_lossy(), e))?;

        let result = self.assert_markers(&mut child, &sock_abs, &log_abs);

        // Always reap the guest and drop the socket file.
        let _ = child.kill();
        let _ = child.wait();
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

        let mut log_file = std::fs::File::create(log_abs)?;
        let mut capture: Vec<u8> = Vec::new();
        let mut stage = BootStage::Banner;
        let mut probe_at = 0usize;

        // Assertion profile selection (D-12): a [config] dev list containing
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

        while Instant::now() < deadline {
            match stage {
                BootStage::Banner => {
                    if contains_marker(&capture, BOOT_BANNER_MARKER.as_bytes()) {
                        stage = if panic_profile {
                            BootStage::PanicStart
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
                        return self.finish(log_abs, &capture, false);
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
                        return self.finish(log_abs, &capture, true);
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

    fn finish(&self, log_abs: &Path, capture: &[u8], panic_profile: bool) -> Result<()> {
        if panic_profile {
            println!(
                "boot-test: PASS -- banner, virtpanic start marker, >= {BOOT_PANIC_REGISTER_MIN} register rows and the clean-halt marker observed in order"
            );
        } else {
            println!(
                "boot-test: PASS -- banner, >= {BOOT_TICK_MIN} ticks and the echo probe observed in order"
            );
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
        count_register_lines,
    };

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
