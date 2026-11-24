use xtask::config::{
    Configuration, QemuOverrides, apply_netdev, apply_to_build_step, apply_to_clippy_step,
    apply_to_qemu_step_with_overrides, apply_to_test_step, apply_virt_netdev, prepare_initrd_dtb,
    virt_resolved,
};
use xtask::{Arch, DynError, Profile, Result, env_or, harvey, image, uregcheck};
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

// The config/uregcheck modules and the Arch/Profile/DynError/Result/
// env_or root types moved to the lib target (16-01, D-05) — this bin
// and the emitter bin (src/bin/r9-netdev-args.rs) are both consumers
// of the xtask lib.
//
// D-06 gate machinery: consumed by the Net profile's finish() pcap
// assertion (07-05) and exercised by its in-file unit suite.
mod pcap;

// 01-18 (D-11/D-12/D-14): the golden-invariant gate controller.  It
// lives in its own module (the PATTERNS guidance: only the clap block,
// the dispatch arm and the step constructor touch this file).
mod gate;

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
                "Verify the Harvey trees' arm64/include/ureg.h and \
                 amd64/include/ureg.h match the pinned layouts (D-06/D-02b layer 2, \
                 amd64 arm + negative control; the per-architecture resolver names each \
                 tree: R9_HARVEY_DIR override first, then R9_HARVEY_DIR_ARM64 / \
                 R9_HARVEY_DIR_AMD64, then HARVEY_SRC — no built-in default, D-01/D-21)",
            ),
        )
        .subcommand(
            clap::Command::new("rfork-header")
                .about(
                    "Write sys/include/rfork.h from the Rust constants in \
                     port/src/rfork.rs (DSGN-02, D-12); --check fails instead when \
                     the committed header is missing or stale",
                )
                .args(&[clap::arg!(
                    --check "Fail when sys/include/rfork.h differs from the rendering instead of writing it"
                )]),
        )
        .subcommand(
            clap::Command::new("initrd")
                .about("Assemble target/bootfs.paq from xtask/manifests/arm64/smoke.toml \
                        through the stage-and-bake engine (D-15/D-16): the Harvey aarch64 tree \
                        comes from the per-architecture resolver (R9_HARVEY_DIR override, then \
                        R9_HARVEY_DIR_ARM64, then HARVEY_SRC — no built-in default) and the host \
                        mkpaqfs from R9_MKPAQFS")
                .args(&[
                    clap::arg!(--arch <arch> "Target architecture")
                        .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                    clap::arg!(--config <name> "Configuration")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .default_value("default"),
                    clap::arg!(--verbose "Print commands"),
                ]),
        )
        .subcommand(
            clap::Command::new("image")
                .about("Assemble one image from xtask/manifests/<objtype>/<image>.toml through the \
                        checked engine (D-15/D-16): two-way drift and Rust provenance enforcement, \
                        bake, member list; --compare-reference compares to the reference bake \
                        (D-29), --emit-manifest derives a manifest from that reference list, \
                        --libraries checks the tier-0 library table (HARN-03)")
                .args(&[
                    clap::arg!(--arch <arch> "Target architecture")
                        .value_parser(clap::builder::EnumValueParser::<Arch>::new())
                        .required(true),
                    clap::arg!(--image <name> "Image name (xtask/manifests/<objtype>/<name>.toml); \
                                                   with --libraries and no --image, the `libraries` \
                                                   manifest is used")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new()),
                    clap::arg!(--"compare-reference" "Compare the assembled member list to the \
                                                      reference bake's \
                                                      ($R9_HARVEY_REFERENCE_AMD64/_ARM64/<image>.members)"),
                    clap::arg!(--"emit-manifest" <path> "Write a manifest derived from the \
                                                          reference member list and the tree, then \
                                                          exit non-zero when any entry is UNRESOLVED")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new()),
                    clap::arg!(--libraries "Check the manifest's [[library]] table against the \
                                            tree's tier-0 list"),
                    clap::arg!(--verbose "Print commands"),
                ]),
        )
        .subcommand(
            clap::Command::new("gate")
                .about("The golden-invariant gate (HARN-01, D-11/D-12/D-14): assemble image:cpu, \
                        build the q35cpu kernel, boot it on q35, prove rio is serving (the D-28 \
                        pair) and that the unchanged control sees no rio, then check the Harvey \
                        tree is unmodified; exits 0 only then.  --full runs the moved gate apps \
                        in sequence.")
                .args(&[
                    clap::arg!(--arch <arch> "Target architecture (the quick tier is x86-64)")
                        .value_parser(clap::builder::EnumValueParser::<Arch>::new()),
                    clap::arg!(--full "Run the moved gate apps in sequence (the full tier, D-14)"),
                    clap::arg!(--"force-no-rio" "Exercise the failing direction: the rio assertion \
                                                 over a -G session, which must exit non-zero with \
                                                 GATE-FAIL rio-not-serving"),
                    clap::arg!(--"serial-check" <pair> "A serial-console probe `<cmd>=<marker>` \
                                                          typed after a passing rio assertion; \
                                                          every marker must appear (repeatable)")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .action(clap::ArgAction::Append),
                    clap::arg!(--accel <accel> "Accelerator override (kvm/tcg); beats R9_ACCEL. \
                                                kvm is the default and never silently downgrades \
                                                (D-13)")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new()),
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
        Some(("rfork-header", m)) => RforkHeaderStep::new(m).run(),
        Some(("initrd", m)) => InitrdStep::new(m).run(),
        Some(("image", m)) => ImageStep::new(m).run(),
        Some(("gate", m)) => gate::GateStep::new(m).run(),
        Some(("clean", _)) => CleanStep::new().run(),
        _ => Err("bad subcommand".into()),
    } {
        eprintln!("{e}");
        process::exit(1);
    }
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

/// Load `<arch>/lib/config_<name>.toml` by name (no clap matches).
fn load_named_config(arch: Arch, name: &str) -> Configuration {
    Configuration::load(format!(
        "{}/{}/lib/config_{}.toml",
        workspace().display(),
        arch.to_string().to_lowercase(),
        name
    ))
}

/// The host triple of the rustc cargo will run (`rustc -vV`'s `host:`
/// line), or None when rustc cannot be asked.
fn host_triple() -> Option<String> {
    let output = Command::new(env_or("RUSTC", "rustc")).arg("-vV").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("host: "))
        .map(|h| h.trim().to_string())
}

/// D-24: the arch-crate test leg falls back to the host triple on the
/// two host arches the nix devshell ships std for.  The riscv64 leg
/// keeps its rustup-only selection.
fn uses_host_triple_fallback(arch: &str) -> bool {
    matches!(arch, "aarch64" | "x86_64")
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

    /// The gate's fixed-shape constructor (01-18): the quick gate always
    /// builds the q35cpu kernel in the debug profile, so it names the
    /// config directly instead of reading a `--config` arg.
    pub(crate) fn for_gate(arch: Arch, config_name: &str, verbose: bool) -> Self {
        Self {
            arch,
            config: load_named_config(arch, config_name),
            profile: Profile::Debug,
            verbose,
        }
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
                ensure_initrd_image(&config, self.arch, self.verbose)?;
                // If the config carries an initrd, patch a target/ copy
                // of the DTB with /chosen linux,initrd-start/end BEFORE qemu's
                // argv is assembled (the apply path emits the patched path).
                prepare_initrd_dtb(&config, workspace().to_str().unwrap(), self.arch)?;

                let mut cmd = Command::new(qemu_system);

                if platform == Some("virt") {
                    // virt: --accel/--cpu flag > R9_ACCEL/R9_CPU env > TOML
                    // > built-in default (tcg + cortex-a72); --accel hvf
                    // selects the gicv3 machine/DTB and -cpu host (see
                    // virt_resolved).
                    apply_to_qemu_step_with_overrides(&mut cmd, &config, &overrides, self.arch);

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
                    apply_to_qemu_step_with_overrides(&mut cmd, &config, &overrides, self.arch);

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
                apply_to_qemu_step_with_overrides(&mut cmd, &self.config, &overrides, self.arch);
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

/// The q35churn churn-gate markers (17-02, FLUSH-02's D-06 tier (a)):
/// the kernel driver's per-sample heap lines (tag start/one/base/end),
/// the slope line (every number the finish()-time post-pass re-derives
/// the verdict from — parsed from the kernel's own markers, never a
/// second hardcoded constant), the fail-closed FAIL (fast-fails the
/// stage loop from any stage, the netgate precedent), the terminal
/// DONE carrying the iteration + generation counts the post-pass
/// counts the staged binaries' own ring-3 prints against, and those
/// prints themselves (churnmark's per-exec ready line, churnsem's
/// per-generation contender line).
const Q35_CHURN_HEAP_MARKER: &str = "churngate: heap ";
const Q35_CHURN_SLOPE_MARKER: &str = "churngate: slope ";
const Q35_CHURN_DONE_MARKER: &str = "churngate: DONE iterations=";
const Q35_CHURN_FAIL_MARKER: &str = "churngate: FAIL";
const Q35_CHURN_MARK_MARKER: &str = "churnmark: ready";
const Q35_CHURN_SEMGEN_MARKER: &str = "churngate: semgen";

/// The q35auth substrate markers (18-01, AUTH-02/D-05+D-07): the
/// once-per-boot armed-source line (main's registration print — D-05's
/// CPUID-gated source selection, the 13-03 dual-mode precedent applied
/// to entropy; KVM -cpu host arms the RDRAND family, TCG's qemu64 the
/// two-sample TSC fold), the staged driver's OWN distinctness evidence
/// from ring 3 (randread as the substrate image's /amd64/init: two
/// 64-byte /dev/random reads, full-length and pairwise distinct — the
/// whole statistical battery D-07 declined), and the terminal DONE /
/// fail-fast FAIL family (the churngate: FAIL precedent).  The
/// finish()-time venue assert matches the armed label against the
/// RESOLVED accel (flag > R9_ACCEL > TOML, the emission's own chain):
/// an rdrand label on a TCG leg, a jitter label on KVM, or a missing
/// line fails the run — a silent downgrade to the deterministic-seed
/// era can never pass unnoticed (T-18-02).
const RANDOM_SOURCE_RDRAND_MARKER: &str = "random: source=rdrand";
const RANDOM_SOURCE_JITTER_MARKER: &str = "random: source=jitter";
const RANDOM_READS_MARKER: &str = "random: READS-OK";
const SUBSTRATE_DONE_MARKER: &str = "substrate: DONE";
const SUBSTRATE_FAIL_MARKER: &str = "substrate: FAIL";

/// The q35auth devcap markers (18-02, AUTH-03/D-06+D-07): the staged
/// driver's `#¤` legs over a PRECOMPUTED fixture pair (from=glenda,
/// to=norbert — the cap string and its HMAC-SHA1 fixed at
/// implementation time, cross-pinned host-side by
/// devcap_tests::devcap_boot_fixture_hash_matches_the_module).
/// POST-OK: the hostowner posts the 20 hash bytes to #¤/caphash.
/// USE-OK: the capuse present consumes and #c/user reads `norbert` —
/// the first r9 kernel uid mutation, read back live from ring 3.
/// ONESHOT-REJECT: the SAME cap presented again errors the donor's
/// formatted `invalid capability` (remcap consumed it — the one-shot
/// rule).  HOSTOWNER-GATE: the mutated user is no longer eve; the
/// caphash open Eperms (the eve gate holds AFTER the mutation).  All
/// four are the driver's OWN prints (the D-09 discipline); a cap-leg
/// failure prints the SUBSTRATE_FAIL family (the shared fail-fast).
const CAP_POST_MARKER: &str = "cap: POST-OK";
const CAP_USE_MARKER: &str = "cap: USE-OK";
const CAP_ONESHOT_MARKER: &str = "cap: ONESHOT-REJECT";
const CAP_HOSTOWNER_MARKER: &str = "cap: HOSTOWNER-GATE";

/// The hostowner-write REFUSAL marker (18-06, AUTH-03/CR-01): after the
/// HOSTOWNER-GATE leg the mutated (non-eve) user writes its own name to
/// #c/hostowner — the eve-base rewrite the old ungated write arm
/// accepted (the CR-01 bypass: re-gain eve, reopen #¤/caphash).  The
/// donor hostownerwrite's iseve gate now REFUSES the write (Eperm
/// class); the refusal is the marker, a successful write is the
/// substrate fail-fast family (the bypass itself).
const CAP_HOSTOWNER_REFUSED_MARKER: &str = "cap: HOSTOWNER-REFUSED";

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

/// The q35shell boot-chain markers (14-06, XEXEC-03's leg-2 gate):
/// after the q35exec stages (module/paqfs notes + the exec-entry note —
/// the same anchors), pid 1 is the real Harvey boot(8) at /boot/boot,
/// and the chain's OWN prints from ring 3 are the evidence: boot(8)'s
/// paqfs-server handoff then its 9P version handshake ("paqfs...",
/// "version..."), init's rc handoff ("init: starting /bin/rc"), then
/// the interactive rc prompt ("% " — the same literal the aarch64 Net
/// gate asserts; rc/lib/rcmain sets it, staged verbatim by the bake).
/// Presence and order, never timings; the negative panic scan stays on
/// with the real-chain fauth exemption (syscall 10 — the Net-profile
/// precedent: boot's nsinit fauth and settime's mount path print it on
/// every real-chain boot).
const Q35_BOOT_VERSION_MARKER: &str = "version...";
const Q35_CHAIN_INIT_MARKER: &str = "init: starting /bin/rc";
// BOOT_PAQFS_MARKER ("paqfs...") and NET_PROMPT_MARKER ("% ") carry
// over from the aarch64 Net gate — the same literals on the same wire.

/// The q35cpu bootconf-tracer markers (19-01, CPU-02; re-keyed 19-08
/// for 19-07's REAL §15 cpurc): the kernel's cmdline-merge print
/// (`devenv: bootconf` — main's exec prologue naming the cmdline-sourced
/// env pairs) and the cpurc ladder's own verdict markers —
/// `cpurc: PAIR-OK` (the D-02 pairing leg, the first rung) and
/// `cpurc: DONE` (the full ladder green: pairing -> ether -> keyfs ->
/// authsrv listen -> factotum -> tcp17010 listen).  The 19-03 fixture
/// legs (SHEBANG-OK/REMOVE-OK/cpurc: RAN) are RETIRED with the fixture
/// body: their proof is load-bearing at the tcp17010 service point on
/// the composed drawterm gate (every per-connection exec of the service
/// file rides the same kernel #! pre-pass).  The 19-01 negative space
/// (`init: starting /bin/rc` absent) is retired too: the real cpurc
/// RUNS TO COMPLETION and exits, so init's post-cpustart terminal
/// handoff printing `init: starting /bin/rc` (exactly once, after
/// DONE) is the EXPECTED shape now — the failure-loop symptom it used
/// to catch cannot be distinguished from it at line level, and the
/// DONE marker already proves cpustart consumed the bootconf pair.
const Q35_BOOTCONF_MARKER: &str = "devenv: bootconf";
const Q35_CPURC_MARKER: &str = "cpurc: DONE";
const Q35_CPURC_LADDER_MARKER: &str = "cpurc: PAIR-OK";
/// The ladder's failure vocabulary (each leg prints `cpurc: FAIL <leg>`
/// and skips every later rung): prohibited whole-capture.
const Q35_CPU_FAIL_WORD: &str = "FAIL";

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

/// The virtchurn churn-gate markers (20-09, D-08's arm64 churn gate):
/// the aarch64 mirror of the q35churn set.  The kernel driver
/// (aarch64/src/platform/virt/churngate.rs) and the staged churnmark/
/// churnsem binaries print the x86_64 vocabulary verbatim, so the
/// strings equal the Q35_CHURN_* ones; the separate names keep each
/// profile's assertion set self-describing.  The per-sample heap lines
/// (tag start/one/base/end), the slope line the finish()-time post-pass
/// re-derives from, the fail-closed FAIL (fast-fails the stage loop),
/// the terminal DONE with the iteration + generation counts, and the
/// staged binaries' own EL0 prints the post-pass counts against them.
const VIRT_CHURN_HEAP_MARKER: &str = "churngate: heap ";
const VIRT_CHURN_SLOPE_MARKER: &str = "churngate: slope ";
const VIRT_CHURN_DONE_MARKER: &str = "churngate: DONE iterations=";
const VIRT_CHURN_FAIL_MARKER: &str = "churngate: FAIL";
const VIRT_CHURN_MARK_MARKER: &str = "churnmark: ready";
const VIRT_CHURN_SEMGEN_MARKER: &str = "churngate: semgen";

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
/// Profile-aware: every profile that rides the REAL Harvey boot chain
/// (net, netip, q35shell, q35churnshell, q35cpu) boots through boot(8)'s
/// auth handshake, which calls fauth — syscall 10 is loud-None by design
/// (D-02), printing `syscall 10 unimplemented` on every real-chain boot.
/// For those profiles exactly that ONE line is exempt; any OTHER
/// unimplemented-syscall line (a traced-set miss) still fails the run,
/// and every other pattern applies unchanged.
fn negative_scan_hit_for(capture: &[u8], exempt_boot_fauth: bool) -> Option<&'static str> {
    let lower: Vec<u8> = capture.iter().map(|b| b.to_ascii_lowercase()).collect();
    if contains_marker(&lower, b"panic") {
        Some("panic")
    } else if contains_marker(&lower, b"unhandled interrupt") {
        Some("Unhandled interrupt")
    } else if contains_marker(&lower, b"unhandled irq") {
        Some("unhandled IRQ")
    } else if contains_marker(&lower, b"note-timeout") {
        Some("note-timeout")
    } else if unimplemented_hit(&lower, exempt_boot_fauth) {
        Some("unimplemented syscall")
    } else {
        None
    }
}

/// The `unimplemented` arm, line-wise so a real-chain profile can exempt
/// the D-02 designed fauth line while still failing any other
/// unimplemented-syscall hit.
fn unimplemented_hit(lower: &[u8], exempt_boot_fauth: bool) -> bool {
    if !contains_marker(lower, b"unimplemented") {
        return false;
    }
    if !exempt_boot_fauth {
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
    /// The q35churn terminal stage (17-02): the module/paqfs notes
    /// observed, now await the churn driver's terminal
    /// `churngate: DONE iterations=<n> semgens=<k>` marker (a
    /// `churngate: FAIL` line fails fast from any stage; the heap
    /// sample lines and the staged binaries' prints ride unordered
    /// between — the finish()-time post-pass owns them).
    Q35ChurnDone,
    /// The virtchurn terminal stage (20-09, the aarch64 mirror of
    /// Q35ChurnDone): the banner observed, now await the aarch64 churn
    /// driver's terminal `churngate: DONE iterations=<n> semgens=<k>`
    /// marker (a `churngate: FAIL` line fails fast from any stage; the
    /// heap samples and the staged binaries' prints ride unordered
    /// between — the finish()-time post-pass owns them).
    VirtChurnDone,
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
    /// The q35auth substrate stage (18-01): the exec-entry note
    /// observed, now await the staged driver's OWN prints from ring 3
    /// — the /dev/random distinctness evidence then the terminal DONE
    /// (any failure prints `substrate: FAIL`, which fails fast from
    /// any stage; the finish()-time venue assert owns the
    /// armed-source line pair).
    Q35SubstratePrints,
    /// The q35shell chain-boot stage (14-06): the exec-entry note
    /// observed, now await boot(8)'s OWN prints from ring 3 — the
    /// paqfs-server handoff and the 9P version handshake — strictly
    /// after the entry stage's anchor.
    Q35ChainBoot,
    /// The q35shell init stage: boot(8)'s prints observed, now await
    /// init's rc-handoff line.
    Q35ChainInit,
    /// The q35shell prompt stage: init observed, now await the
    /// interactive rc prompt — the gate's terminal marker (the chain
    /// stays running; the interactive expect sweep is 14-07's
    /// Harvey-side gate).
    Q35ChainPrompt,
    /// The q35cpu cpurc stage (19-01, extended 19-03): boot(8)'s
    /// prints observed, now await the cpurc SCRIPT's markers from ring
    /// 3 — `cpurc: RAN` (the init -c path's evidence, itself a #! exec
    /// since 19-03) plus the two legs' OK markers (SHEBANG-OK,
    /// REMOVE-OK; the bootconf seed line printed far earlier,
    /// whole-capture-asserted at finish; an `init: starting /bin/rc`
    /// line here is the failure-loop symptom, and any FAIL word is a
    /// failed leg — both fail the gate).
    Q35CpuRc,
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
            BootStage::Q35ChurnDone => {
                write!(f, "terminal marker {Q35_CHURN_DONE_MARKER:?} iterations=… semgens=…")
            }
            BootStage::VirtChurnDone => {
                write!(f, "terminal marker {VIRT_CHURN_DONE_MARKER:?} iterations=… semgens=…")
            }
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
            BootStage::Q35SubstratePrints => write!(
                f,
                "the substrate driver's own prints {{{RANDOM_READS_MARKER:?}, \
                 {SUBSTRATE_DONE_MARKER:?}}} from ring 3"
            ),
            BootStage::Q35ChainBoot => write!(
                f,
                "boot(8)'s own prints {{{BOOT_PAQFS_MARKER:?}, {Q35_BOOT_VERSION_MARKER:?}}} \
                 after the exec entry"
            ),
            BootStage::Q35ChainInit => {
                write!(f, "init handoff {Q35_CHAIN_INIT_MARKER:?} after boot(8)'s prints")
            }
            BootStage::Q35ChainPrompt => {
                write!(f, "the interactive rc prompt {NET_PROMPT_MARKER:?}")
            }
            BootStage::Q35CpuRc => {
                write!(
                    f,
                    "the cpurc ladder markers {{{Q35_CPURC_LADDER_MARKER:?}, \
                     {Q35_CPURC_MARKER:?}}} from ring 3"
                )
            }
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
    /// The q35shell boot-chain gate (14-06, XEXEC-03's r9-side leg): the
    /// X64 discovery + bind stages, the module/paqfs notes, the
    /// exec-entry note, then the real Harvey boot(8)→init→rc chain's
    /// own prints through to the interactive `% ` prompt.
    X64Shell,
    /// The q35churn churn gate (17-02, FLUSH-02's D-06 tier (a)): the
    /// X64 discovery stages, the module/paqfs notes, then the
    /// kernel-context churn driver's terminal `churngate: DONE` — with
    /// the finish()-time post-pass asserting the heap bound from the
    /// kernel's own markers (the four tagged heap samples, the slope
    /// line, the DONE counts vs the staged binaries' own prints, and
    /// zero `churngate: FAIL` — the fast-fail arm already aborts on
    /// the first).
    X64Churn,
    /// The virtchurn churn gate (20-09, D-08's arm64 churn gate): the
    /// banner, then the aarch64 kernel-context churn driver's terminal
    /// `churngate: DONE` — with the finish()-time post-pass (the
    /// q35churn one, shared) asserting the heap bound from the kernel's
    /// own markers, the DONE counts vs the staged binaries' own prints,
    /// and zero `churngate: FAIL`.
    VirtChurn,
    /// The q35cpu bootconf-tracer gate (19-01 CPU-02, extended 19-03):
    /// the X64 discovery + module/paqfs stages, boot(8)'s chain prints,
    /// then the cpurc SCRIPT's own `cpurc: RAN` + the two 19-03 ring-3
    /// legs' OK markers from ring 3 — with the finish()-time post-pass
    /// asserting the kernel's bootconf-merge line (`devenv: bootconf`),
    /// ZERO `init: starting /bin/rc` lines (the failure-loop symptom),
    /// both leg markers present, and ZERO `FAIL` words (the gate greps
    /// both directions).
    X64Cpu,
    /// The q35auth substrate gate (18-01, AUTH-02/D-05+D-07): the X64
    /// discovery stages, the module/paqfs notes, the exec-entry note,
    /// then the staged driver's OWN prints from ring 3 (`random:
    /// READS-OK` + `substrate: DONE`) — with the finish()-time venue
    /// assert matching the armed-source line pair (`random:
    /// source=rdrand` / `random: source=jitter`) against the RESOLVED
    /// accel leg and re-asserting the markers whole-capture plus zero
    /// `substrate: FAIL` (the fast-fail arm already aborts on the
    /// first).
    X64Substrate,
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
        // arm64/include/ureg.h, x86_64 pins amd64/include/ureg.h —
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
        ensure_initrd_image(&config, self.arch, self.verbose)?;
        // If the config carries an initrd, patch a target/ copy of the
        // DTB with /chosen linux,initrd-start/end (same apply path as QemuStep).
        prepare_initrd_dtb(&config, workspace().to_str().unwrap(), self.arch)?;

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
        apply_to_qemu_step_with_overrides(&mut cmd, &config, &overrides, self.arch);
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
                let mut base = BOOT_TEST_TIMEOUT_SECS;
                // 17-02 (the churn profile's own budget, the 16-08
                // soak_stage_timeout precedent): the #44 repro shape is
                // ~1400 fork+exec iterations + contention generations,
                // and the measured KVM pace is ~0.28 s per exec cycle —
                // consistent with 16-08's measured KVM syscall pacing
                // (~2.5 rounds/s for the tcpblast baseline) — so the
                // whole gate runs ~400 s of wall clock.  The carried
                // 300 s default would kill it mid-loop; 900 s carries
                // the loop + the drain with headroom.  Env still wins.
                if self.config_name == "q35churn" {
                    base = 900;
                }
                // 20-09: the aarch64 virtchurn gate runs the same loop
                // shape; it carries the q35churn base (TCG scales it by
                // TCG_OVERALL_MULT below, like every virt profile).
                if self.arch == Arch::Aarch64 && self.config_name == "virtchurn" {
                    base = 900;
                }
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
        // The Q35CHURN profile ("q35churn", 17-02's D-06 tier (a)) is
        // the same name-selected shape: its dev list is ["exec",
        // "churn"] — exec is IN the list, so any exec-profile dispatch
        // must check this FIRST (the virtnet ordering rule), and the
        // discovery satisfaction branches into the module stage below
        // (no bind stage — the profile is net-free by design).
        let q35churn_profile = self.arch == Arch::X86_64 && self.config_name == "q35churn";
        // The VIRTCHURN profile ("virtchurn", 20-09's D-08 arm64 churn
        // gate) is the q35churn shape on aarch64: dev = ["exec",
        // "ether", "churn"] — exec is IN the list, so the Banner-stage
        // dispatch checks it FIRST (the virtnet ordering rule) and routes
        // straight to the churn driver's terminal marker (no exec-entry
        // note: the driver is a kernel-context proc, never EL0).
        let virtchurn_profile = self.arch == Arch::Aarch64 && self.config_name == "virtchurn";
        // The Q35AUTH profile ("q35auth", 18-01's AUTH-02/D-07
        // substrate gate) is the same name-selected shape: its dev
        // list is ["exec"] (like q35exec's), so the module-stage and
        // entry-stage routing below names it explicitly — the staged
        // driver's prints, not smokesleep's, are the marker set.
        let q35auth_profile = self.arch == Arch::X86_64 && self.config_name == "q35auth";
        // The Q35EXEC profile ("q35exec", 14-05's XEXEC-02 leg-1 gate)
        // is the same name-selected shape: its dev list is ["exec"] and
        // its [dist] module key stages bootfs.paq into the ISO, so the
        // discovery satisfaction branches into the module/paqfs →
        // exec-entry → marker-prints stages below.
        let q35exec_profile = self.arch == Arch::X86_64 && self.config_name == "q35exec";
        // The Q35SHELL profile ("q35shell", 14-06's XEXEC-03 leg-2
        // gate) — the boot-chain profile: dev = ["exec", "bootchain",
        // "ether", "ip"], so the discovery satisfaction routes through
        // the bind stage (dev_ether is on — the Phase-13 netgate
        // placement virtshell carries) and then the module stages chain
        // into boot(8)/init/rc below.  Its [dist] module key stages the
        // CHAIN bake (bootfs-chain.paq — the marker bake stays
        // bootfs.paq for q35exec).
        let q35shell_profile = self.arch == Arch::X86_64 && self.config_name == "q35shell";
        // The Q35CHURNSHELL profile ("q35churnshell", the composed venue's
        // kernel: config_q35shell VERBATIM plus dev_churn) rides the SAME
        // real Harvey boot chain as q35shell — dev_churn+dev_bootchain keeps
        // the normal chain (the churn driver never runs), so every chain
        // stage and the designed boot-chain fauth exemption apply here too
        // (01-33; stop B failed it at the tick stage on syscall 10).
        let q35churnshell_profile =
            self.arch == Arch::X86_64 && self.config_name == "q35churnshell";
        // The Q35CPU profile ("q35cpu", 19-01's CPU-02 tracer) — the
        // bootconf seam's gate: dev = ["exec", "bootchain"] (the chain
        // without ether/ip — the tracer needs no net; the NIC stays
        // present-unbound per the discovery contract), the [dist]
        // module stages the §15 cpu bake (bootfs-cpu.paq), and the
        // [dist] append key rides the grub.cfg @R9-APPEND@ token with
        // `init=/amd64/init -c` — the cmdline the kernel merges into
        // the ROOT env seed, boot(8)'s execinit consumes, and init -c's
        // cpustart answers by running /rc/bin/cpurc.  The control leg
        // is q35shell itself (no append, the terminal path untouched).
        let q35cpu_profile = self.arch == Arch::X86_64 && self.config_name == "q35cpu";
        // The X64 bind stage's ordered-presence anchor (13-05): the byte
        // offset at the discovery stage's satisfaction — a bind line
        // that rode the wire earlier never counts.
        let mut bind_after = 0usize;

        while Instant::now() < deadline {
            // Always-on negative scan: a panic/Unhandled-interrupt line
            // fails any profile except the deliberate-panic one.  The
            // net profile exempts exactly the D-02 designed fauth line (the
            // real Harvey chain prints `syscall 10 unimplemented` twice a
            // boot); every other pattern — and any other unimplemented
            // syscall — still fails the run.  The netip profile rides the
            // same real boot chain — the same single-line exemption, and
            // so does the q35cpu tracer (the same boot(8) fauth path).
            if !panic_profile && let Some(hit) = negative_scan_hit_for(&capture, net_profile || netip_profile || q35shell_profile || q35cpu_profile || q35churnshell_profile) {
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
            // The q35churn fail-fast (17-02): the churn driver's own
            // verdict is fail-closed — a `churngate: FAIL` line (slope
            // over ceiling, a wedge watchdog, a spawn failure) means
            // the missing terminal marker would otherwise only surface
            // as a slow budget expiry.  Fail the gate the moment the
            // kernel names its stage; the staged churnsem binary's own
            // rfork/wait failures print the same family.
            if q35churn_profile && contains_marker(&capture, Q35_CHURN_FAIL_MARKER.as_bytes()) {
                let tail = String::from_utf8_lossy(&capture[capture.len().saturating_sub(200)..]);
                return Err(self.fail(
                    stage,
                    &capture,
                    &format!("kernel churngate failed ({Q35_CHURN_FAIL_MARKER}): {tail}"),
                ));
            }
            // The virtchurn fail-fast (20-09): the aarch64 driver's
            // verdict is fail-closed the same way.
            if virtchurn_profile && contains_marker(&capture, VIRT_CHURN_FAIL_MARKER.as_bytes()) {
                let tail = String::from_utf8_lossy(&capture[capture.len().saturating_sub(200)..]);
                return Err(self.fail(
                    stage,
                    &capture,
                    &format!("kernel churngate failed ({VIRT_CHURN_FAIL_MARKER}): {tail}"),
                ));
            }
            // The q35auth fail-fast (18-01): the staged driver's own
            // verdict is fail-closed — a `substrate: FAIL` line (open
            // failure, short read, identical consecutive reads) means
            // the missing terminal DONE would otherwise only surface
            // as a slow budget expiry.  Fail the gate the moment the
            // driver names its failure.
            if q35auth_profile && contains_marker(&capture, SUBSTRATE_FAIL_MARKER.as_bytes()) {
                let tail = String::from_utf8_lossy(&capture[capture.len().saturating_sub(200)..]);
                return Err(self.fail(
                    stage,
                    &capture,
                    &format!("substrate driver failed ({SUBSTRATE_FAIL_MARKER}): {tail}"),
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
                            } else if virtchurn_profile {
                                BootStage::VirtChurnDone
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
                        } else if q35shell_profile || q35churnshell_profile {
                            // 14-06: the chain profile carries ether (the
                            // virtshell membership) — assert the bind half
                            // first (the /net substrate 14-09 consumes),
                            // the same positional-anchor discipline.  01-33:
                            // q35churnshell is the same chain (dev_churn
                            // never routes to the driver under dev_bootchain).
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
                        } else if q35churn_profile {
                            // 17-02: the churn gate continues into the
                            // same module half (the churn profile's
                            // bootstrap runs exec_bootstrap's own
                            // prologue — identical module/paqfs notes),
                            // then awaits the churn driver's terminal
                            // marker (no exec-entry note: the driver is
                            // a kernel-context proc, never ring 3).
                            bind_after = last_marker_line_end(
                                &capture[pci_after..],
                                PCI_NET_DEVICE_MARKER,
                            )
                            .map(|p| pci_after + p)
                            .unwrap_or(capture.len());
                            stage = BootStage::Q35ModulePaqfs;
                        } else if q35auth_profile {
                            // 18-01: the substrate gate continues into
                            // the same module half (dev = ["exec"] —
                            // exec_bootstrap's own prologue runs, the
                            // module/paqfs notes included), then the
                            // exec-entry note, then the staged
                            // driver's own prints from ring 3.
                            bind_after = last_marker_line_end(
                                &capture[pci_after..],
                                PCI_NET_DEVICE_MARKER,
                            )
                            .map(|p| pci_after + p)
                            .unwrap_or(capture.len());
                            stage = BootStage::Q35ModulePaqfs;
                        } else if q35cpu_profile {
                            // 19-01: the cpu tracer continues into the
                            // same module half (19-08: ether+ip joined
                            // q35cpu's dev set — the §15 ladder's
                            // ipconfig leg needs the bound NIC), then
                            // the exec-entry note and boot(8)'s chain
                            // prints, ending at the cpurc marker below.
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
                        // the previous stage's satisfaction.  The chain
                        // profile (14-06) continues into the module half
                        // instead — its exec bootstrap never runs the
                        // kernel-side netgate.
                        bind_after = capture.len();
                        stage = if q35shell_profile || q35churnshell_profile {
                            BootStage::Q35ModulePaqfs
                        } else {
                            BootStage::X64PingPong
                        };
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
                        stage = if q35churn_profile {
                            // 17-02: the churn venue's next marker is
                            // the driver's terminal DONE — the heap
                            // samples, the exec'd binaries' prints, and
                            // the exit lines ride unordered between
                            // (the finish() post-pass owns them).
                            BootStage::Q35ChurnDone
                        } else {
                            BootStage::Q35ExecEntry
                        };
                    }
                }
                BootStage::Q35ChurnDone => {
                    // Wait for the COMPLETE line, not the prefix: the
                    // marker can satisfy mid-flush (serial chunks) and
                    // the post-pass parses both fields off this very
                    // line — finishing on "…iterations=1400 semge"
                    // truncated the run (17-02's live catch).  The
                    // line helper is CRLF-trimmed; a partial line
                    // lacks semgens= and the loop waits on.
                    if let Some(line) =
                        line_containing(&capture[bind_after..], Q35_CHURN_DONE_MARKER.as_bytes())
                    {
                        if line.contains("semgens=") {
                            return self.finish(log_abs, &capture, BootProfile::X64Churn);
                        }
                    }
                }
                BootStage::VirtChurnDone => {
                    // The COMPLETE line, not the prefix (the q35churn
                    // live catch: the marker can satisfy mid-flush and
                    // the post-pass parses both fields off this line).
                    if let Some(line) =
                        line_containing(&capture, VIRT_CHURN_DONE_MARKER.as_bytes())
                    {
                        if line.contains("semgens=") {
                            return self.finish(log_abs, &capture, BootProfile::VirtChurn);
                        }
                    }
                }
                BootStage::Q35ExecEntry => {
                    if contains_marker(&capture[bind_after..], Q35_EXEC_ENTRY_MARKER.as_bytes()) {
                        // Anchor past the entry note: the binary's own
                        // prints follow (its WRITE syscalls, never a
                        // kernel print on its behalf).  The chain profile
                        // (14-06) awaits boot(8)'s prints instead of the
                        // marker's — pid 1 IS the real boot(8).  The
                        // substrate profile (18-01) awaits the randread
                        // driver's own prints — pid 1 IS the driver.  The
                        // cpu tracer (19-01) rides the chain profile's
                        // arm — its pid 1 is the same real boot(8).
                        bind_after = capture.len();
                        stage = if q35shell_profile || q35churnshell_profile || q35cpu_profile {
                            BootStage::Q35ChainBoot
                        } else if q35auth_profile {
                            BootStage::Q35SubstratePrints
                        } else {
                            BootStage::Q35MarkerPrints
                        };
                    }
                }
                BootStage::Q35SubstratePrints => {
                    // The driver's OWN prints (D-09): the distinctness
                    // evidence then the terminal DONE — both anchored
                    // past the exec entry.  The armed-source venue pair
                    // is finish()'s (whole-capture, accel-matched).
                    if contains_marker(&capture[bind_after..], RANDOM_READS_MARKER.as_bytes())
                        && contains_marker(&capture[bind_after..], SUBSTRATE_DONE_MARKER.as_bytes())
                    {
                        return self.finish(log_abs, &capture, BootProfile::X64Substrate);
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
                BootStage::Q35ChainBoot => {
                    // 14-06: boot(8)'s OWN prints from ring 3 — the
                    // paqfs-server exec handoff and the 9P version
                    // handshake (embed's connectembed + connectroot),
                    // both past the exec-entry anchor (the kernel's own
                    // "q35: paqfs root mounted" note can never satisfy
                    // the "paqfs..." literal).
                    if contains_marker(&capture[bind_after..], BOOT_PAQFS_MARKER.as_bytes())
                        && contains_marker(&capture[bind_after..], Q35_BOOT_VERSION_MARKER.as_bytes())
                    {
                        bind_after = capture.len();
                        // 19-01: the cpu path's cpustart runs BEFORE
                        // any terminal handoff — await the cpurc ladder
                        // marker instead of init's rc handoff (19-08:
                        // the REAL cpurc exits after DONE and init then
                        // hands off to the terminal rc — that later
                        // `init: starting /bin/rc` line is expected; the
                        // stage terminates at DONE before it).
                        stage = if q35cpu_profile {
                            BootStage::Q35CpuRc
                        } else {
                            BootStage::Q35ChainInit
                        };
                    }
                }
                BootStage::Q35ChainInit => {
                    // init's rc handoff — the REAL init's line (the bake's
                    // inverted identity guard pinned out/init on the host
                    // side; this asserts the running binary).
                    if contains_marker(&capture[bind_after..], Q35_CHAIN_INIT_MARKER.as_bytes()) {
                        bind_after = capture.len();
                        stage = BootStage::Q35ChainPrompt;
                    }
                }
                BootStage::Q35ChainPrompt => {
                    // The interactive rc prompt — rcmain's '% ' (staged
                    // verbatim; the same literal the aarch64 Net gate
                    // asserts).  The chain stays running: finishing here
                    // is the gate's terminal state (the interactive
                    // expect sweep is 14-07's Harvey-side gate).
                    if contains_marker(&capture[bind_after..], NET_PROMPT_MARKER.as_bytes()) {
                        return self.finish(log_abs, &capture, BootProfile::X64Shell);
                    }
                }
                BootStage::Q35CpuRc => {
                    // 19-08 re-key: the REAL §15 cpurc (19-07) runs a
                    // six-rung fail-loud ladder to `cpurc: DONE`.  The
                    // 19-01 failure-loop prohibition is retired here —
                    // the real cpurc EXITS and init's terminal handoff
                    // (`init: starting /bin/rc`, once, after DONE) can
                    // win the poll race against this stage seeing DONE;
                    // the ladder's own `cpurc: FAIL <leg>` vocabulary
                    // fast-fails instead.
                    if count_marker_lines(&capture, Q35_CPU_FAIL_WORD) > 0 {
                        return Err(self.fail(
                            stage,
                            &capture,
                            &format!(
                                "cpu gate: a FAIL marker printed — a cpurc ladder leg failed \
                                 (grep the log for {:?}); the last OK rung above it names \
                                 how far the ladder got",
                                 Q35_CPU_FAIL_WORD
                            ),
                        ));
                    }
                    if contains_marker(&capture[bind_after..], Q35_CPURC_MARKER.as_bytes()) {
                        return self.finish(log_abs, &capture, BootProfile::X64Cpu);
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
                matches!(
                    profile,
                    BootProfile::Net | BootProfile::Netip | BootProfile::X64Shell | BootProfile::X64Cpu
                ),
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
            BootProfile::X64Churn => {
                let v = churn_post_pass(capture, &Q35_CHURN_MARKERS)?;
                let ChurnVerdict { iterations, want_marks, semgens, steady, measured, ceiling } = v;
                println!(
                    "boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + module boot: banner, memory \
                     facts, discovery, the module-tag/paqfs mount notes, then the kernel-context \
                     churn driver's {iterations} exec iterations ({want_marks} churnmark ring-3 \
                     prints counted) + {semgens} RFMEM contention generations, drain, and the \
                     heap slope held: steady={steady} measured={measured} <= ceiling={ceiling} \
                     bytes/exec (host re-derived from the kernel's own samples) — D-06 tier (a) \
                     green, CR-03's in-kernel contention leg landed"
                );
            }
            BootProfile::VirtChurn => {
                // 20-09: the same post-pass over the aarch64 driver's
                // markers (the vocabulary is x86_64's verbatim; the
                // ceiling is parsed from the kernel's own slope line).
                let v = churn_post_pass(capture, &VIRT_CHURN_MARKERS)?;
                let ChurnVerdict { iterations, want_marks, semgens, steady, measured, ceiling } = v;
                println!(
                    "boot-test: PASS -- aarch64 virt kernel + initrd boot: banner, then the \
                     kernel-context churn driver's {iterations} exec iterations ({want_marks} \
                     churnmark EL0 prints counted) + {semgens} RFMEM contention generations, \
                     drain, and the heap slope held: steady={steady} measured={measured} <= \
                     ceiling={ceiling} bytes/exec (host re-derived from the kernel's own samples) \
                     — D-08's arm64 churn gate green"
                );
            }
            BootProfile::X64Substrate => {
                // The finish()-time venue assert (18-01, D-07/T-18-02):
                // exactly ONE of the two armed-source labels, matching
                // the RESOLVED accel venue (flag > R9_ACCEL > TOML —
                // the same chain the qemu emission and the budget
                // scaling use).  A missing line means the hooks never
                // armed (the deterministic-seed era); a wrong label
                // means the CPUID gate misclassified the venue; both
                // labels means a double print.  None may pass.
                let effective_accel = self
                    .accel
                    .clone()
                    .or_else(|| self.config.qemu.as_ref().and_then(|q| q.accel.clone()))
                    .unwrap_or_default();
                let (expect, forbid) = if effective_accel == "tcg" {
                    (RANDOM_SOURCE_JITTER_MARKER, RANDOM_SOURCE_RDRAND_MARKER)
                } else {
                    (RANDOM_SOURCE_RDRAND_MARKER, RANDOM_SOURCE_JITTER_MARKER)
                };
                if !contains_marker(capture, expect.as_bytes()) {
                    return Err(self.fail_pretty(
                        capture,
                        &format!(
                            "substrate gate: armed-source line {expect:?} absent — the D-05 \
                             registration print never ran (hooks not armed: the \
                             deterministic-seed era, never a green)"
                        ),
                    ));
                }
                if contains_marker(capture, forbid.as_bytes()) {
                    return Err(self.fail_pretty(
                        capture,
                        &format!(
                            "substrate gate: armed-source venue mismatch — {forbid:?} on a \
                             {effective_accel:?}-accel leg (expected {expect:?}; the CPUID gate \
                             misclassified the venue)"
                        ),
                    ));
                }
                // The driver's evidence, whole-capture: READS-OK + DONE
                // (the stage loop anchored them past the exec entry), the
                // five cap markers (18-02: post/consume/one-shot/hostowner
                // over the fixture pair; 18-06: the CR-01 hostowner-write
                // refusal), and zero FAIL lines
                // anywhere (the fast-fail already aborted on the first;
                // belt-and-braces).
                for marker in [
                    RANDOM_READS_MARKER,
                    SUBSTRATE_DONE_MARKER,
                    CAP_POST_MARKER,
                    CAP_USE_MARKER,
                    CAP_ONESHOT_MARKER,
                    CAP_HOSTOWNER_MARKER,
                    CAP_HOSTOWNER_REFUSED_MARKER,
                ] {
                    if !contains_marker(capture, marker.as_bytes()) {
                        return Err(self
                            .fail_pretty(capture, &format!("substrate marker {marker:?} absent")));
                    }
                }
                let fails = count_marker_lines(capture, SUBSTRATE_FAIL_MARKER);
                if fails != 0 {
                    return Err(self.fail_pretty(
                        capture,
                        &format!("substrate gate: {fails} {SUBSTRATE_FAIL_MARKER} line(s)"),
                    ));
                }
                println!(
                    "boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + module boot: banner, \
                     memory facts, discovery, the module/paqfs mount notes, the exec-entry note, \
                     the armed source {expect:?} (venue {effective_accel:?}), then the staged \
                     driver's OWN ring-3 prints (two 64-byte /dev/random reads, full-length and \
                     pairwise distinct -> READS-OK; the #¤ fixture legs: post, consume with the \
                     /dev/user readback, one-shot reject, hostowner gate, hostowner write \
                     refused -> the five cap markers; DONE via WRITE syscalls) — AUTH-02's \
                     substrate re-key + AUTH-03's devcap live with CR-01's eve-base gate closed \
                     (D-05/D-06/D-07, 18-06)"
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
            BootProfile::X64Shell => {
                // XEXEC-03's r9-side leg (14-06): the ordered stage
                // chain (banner -> facts -> discovery -> bind -> module/
                // paqfs -> exec entry -> boot(8)'s paqfs/version prints
                // -> init's rc handoff -> the `% ` prompt) is asserted
                // stage-wise; the finish-time whole-capture scans are
                // the negative space — the D-09 prohibition shape
                // re-keyed for the chain: every chain marker line
                // (paqfs.../version.../init: starting/% ) is produced
                // by a ring-3 Harvey binary through the real syscall
                // path, never by the kernel on the chain's behalf (the
                // kernel's own notes stop at the exec-entry line).
                println!(
                    "boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + chain-module boot: \
                     banner, memory facts, discovery, the NIC bind, the module-tag/paqfs mount \
                     notes, the exec-entry note, then the REAL boot(8)→init→rc chain's own \
                     ring-3 prints (paqfs/version handoffs, the init rc handoff) through to the \
                     interactive `% ` prompt, observed in order — XEXEC-03 leg 2 (D-08's staged \
                     chain on the proven exec substrate; the composed Harvey-side gate is \
                     14-07's)"
                );
            }
            BootProfile::X64Cpu => {
                // 19-08 re-key (19-07's REAL §15 cpurc): the ordered
                // stage chain (banner -> facts -> discovery -> module/
                // paqfs -> exec entry -> boot(8)'s prints -> the cpurc
                // ladder) is asserted stage-wise, terminating at
                // `cpurc: DONE`.  The finish-time scans keep the seam's
                // own evidence — the bootconf-merge line (the kernel's
                // `devenv: bootconf` print proves the append reached
                // the ROOT env seed) and the ladder's pairing rung
                // (`cpurc: PAIR-OK` — the bake-artifact leg, the first
                // rung DONE implies but named for the evidence).  The
                // 19-01 failure-loop prohibition (`init: starting
                // /bin/rc` == 0) is RETIRED: the real cpurc exits after
                // DONE and init's terminal handoff prints that line
                // exactly once — the expected shape; the DONE marker
                // already proves execinit consumed the bootconf pair.
                // The 19-03 fixture legs (SHEBANG-OK/REMOVE-OK) are
                // retired with the fixture body (their #! proof is
                // load-bearing at the tcp17010 service point on the
                // composed gate).
                if !contains_marker(capture, Q35_BOOTCONF_MARKER.as_bytes()) {
                    return Err(self.fail_pretty(
                        capture,
                        &format!(
                            "cpu gate: the bootconf-merge line {Q35_BOOTCONF_MARKER:?} absent — \
                             the cmdline never reached the ROOT env seed (the append seam or \
                             the mb2 tag-1 parse dropped it)"
                        ),
                    ));
                }
                if !contains_marker(capture, Q35_CPURC_LADDER_MARKER.as_bytes()) {
                    return Err(self.fail_pretty(
                        capture,
                        &format!(
                            "cpu gate: the ladder's pairing rung {Q35_CPURC_LADDER_MARKER:?} \
                             absent — the D-02 pairing leg never passed (a bad bake never \
                             reaches the later rungs)"
                        ),
                    ));
                }
                if !contains_marker(capture, Q35_CPURC_MARKER.as_bytes()) {
                    return Err(self
                        .fail_pretty(capture, &format!("cpu marker {Q35_CPURC_MARKER:?} absent")));
                }
                let fail_words = count_marker_lines(capture, Q35_CPU_FAIL_WORD);
                if fail_words != 0 {
                    return Err(self.fail_pretty(
                        capture,
                        &format!(
                            "cpu gate: {fail_words} line(s) carrying {Q35_CPU_FAIL_WORD:?} — a \
                             cpurc ladder leg verdict or stray failure print"
                        ),
                    ));
                }
                println!(
                    "boot-test: PASS -- SeaBIOS -> GRUB2 -> mb2 kernel + cpu-module boot with \
                     the append seam live: banner, memory facts, discovery, the module/paqfs \
                     notes, the exec-entry note, the bootconf-merge line \
                     ({Q35_BOOTCONF_MARKER}: the cmdline's key=value pairs in the ROOT env \
                     seed), boot(8)'s own ring-3 prints, then init -c's cpustart #!-exec'ing \
                     the REAL /rc/bin/cpurc — the six-rung fail-loud ladder (pairing -> ether \
                     -> keyfs -> authsrv listen -> factotum -> tcp17010 listen) green to \
                     {Q35_CPURC_MARKER} with the pairing rung ({Q35_CPURC_LADDER_MARKER}) and \
                     ZERO FAIL words, then init's post-cpustart terminal handoff to the `% ` \
                     prompt (the retired 19-01 loop-prohibition's expected successor) — \
                     CPU-02's service=cpu architecture live from ring 3 on 19-07's §15 image \
                     (19-01/19-03/19-08)"
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
        // D-12: a stale or missing generated header fails the project's
        // test command before any cargo leg runs.
        xtask::rforkh::check(&workspace())?;
        // Each leg: the cargo args plus an optional profile whose cfg set
        // the leg compiles under (None = the crate's bare default cfg).
        let mut all_cmd_args: Vec<(Vec<String>, Option<Configuration>)> = Vec::new();

        all_cmd_args.push((
            vec![
                "test".to_string(),
                "--package".to_string(),
                "port".to_string(),
                "--lib".to_string(),
                "--tests".to_string(),
            ],
            None,
        ));

        all_cmd_args.push((
            vec![
                "test".to_string(),
                "--package".to_string(),
                "crypto".to_string(),
                "--lib".to_string(),
                "--tests".to_string(),
            ],
            None,
        ));

        // The scalar-backend leg (03-RESEARCH.md F3, T-03-02): the default
        // host build links sha2's hardware SHA path and chacha20's NEON (or
        // AVX2) backend, while the kernels run the scalar code selected by
        // lib/*-unknown-none-elf.json.  The same vectors must pin the code
        // the kernels execute, so this leg forces sha256::soft and chacha20's
        // soft backend with the crates' own --cfg names.  The flags ride in
        // THIS leg's argument vector and never in .cargo/config.toml: a
        // global build.rustflags entry would reach the kernel builds, whose
        // flags xtask sets through the same key.
        all_cmd_args.push((
            vec![
                "test".to_string(),
                "--package".to_string(),
                "crypto".to_string(),
                "--lib".to_string(),
                "--tests".to_string(),
                "--config".to_string(),
                "build.rustflags=[\"--cfg\",\"chacha20_backend=\\\"soft\\\"\",\"--cfg\",\"sha2_backend=\\\"soft\\\"\"]"
                    .to_string(),
            ],
            None,
        ));

        let rustup_state = RustupState::new();

        let arch = std::env::consts::ARCH;
        let mut target = rustup_state.std_supported_target(arch).map(|t| t.to_string());
        // The arch crate's host tests (HOST-TEST-ROUTE A, Harvey
        // 20-12/20-17): the nix devshell has no rustup, so the rustup
        // probe above finds nothing and the leg used to be skipped
        // silently.  On an aarch64 or x86_64 host the host triple itself
        // carries std (the nix toolchain ships it), so fall back to it
        // (D-24).  The riscv64 leg keeps its rustup-only selection.
        if target.is_none() && uses_host_triple_fallback(arch) {
            target = host_triple();
        }
        if let Some(target) = target {
            let leg = vec![
                "test".to_string(),
                "--package".to_string(),
                arch.to_string(),
                "--bins".to_string(),
                "--target".to_string(),
                target,
            ];
            all_cmd_args.push((leg.clone(), None));
            // The second aarch64 leg runs the crate under the virtshell
            // profile's cfg set (platform="virt", dev_exec, dev_bootchain,
            // dev_ether, dev_ip), passed the way the build step passes it,
            // so the virt platform and the dev_exec-gated pure logic
            // (shebang_pass, valid_stat_wire, parse_bootconf_pairs,
            // check_cap_extents) are pinned on the host too.
            if arch == "aarch64" {
                all_cmd_args.push((leg, Some(load_named_config(Arch::Aarch64, "virtshell"))));
            }
        }

        for (cmd_args, config) in all_cmd_args {
            let mut cmd = Command::new(cargo());
            cmd.current_dir(workspace());

            cmd.args(cmd_args);
            if let Some(config) = &config {
                apply_to_test_step(&mut cmd, config);
            }
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

/// `cargo xtask image`: assemble one image from its per-file manifest
/// (D-15/D-16) through the checked engine — two-way drift, Rust
/// provenance, path validation — and optionally compare its member list to
/// the reference bake (D-29), emit a manifest from that reference list, or
/// check the `[[library]]` table against the tier-0 list (HARN-03).
///
/// Manifests live at `xtask/manifests/<amd64|arm64>/<image>.toml`; outputs
/// at `target/images/<x86_64|aarch64>/<image>.{stage,paq,members}`.  The
/// tree comes from the per-architecture resolver, the host `mkpaqfs` from
/// `R9_MKPAQFS` — both with named errors, no derived defaults.
struct ImageStep {
    arch: Arch,
    image_name: String,
    compare_reference: bool,
    emit_manifest: Option<PathBuf>,
    libraries: bool,
    verbose: bool,
}

impl ImageStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        Self {
            arch: Arch::from(matches),
            image_name: matches.get_one::<String>("image").cloned().unwrap_or_default(),
            compare_reference: matches.get_flag("compare-reference"),
            emit_manifest: matches.get_one::<String>("emit-manifest").map(PathBuf::from),
            libraries: matches.get_flag("libraries"),
            verbose: verbose(matches),
        }
    }

    fn run(self) -> Result<()> {
        let arch_name = arch_dir_name(self.arch);
        if let Some(path) = &self.emit_manifest {
            if self.image_name.is_empty() {
                return Err(
                    "image: --emit-manifest needs --image <name> (the reference list it mirrors)"
                        .into(),
                );
            }
            return self.emit(path);
        }
        if self.libraries {
            // The library table is an audit check, not an image assembly:
            // `--libraries` alone reads the `libraries` manifest (the
            // plan's `cargo xtask image --arch <a> --libraries` form).
            let image_name = if self.image_name.is_empty() {
                "libraries"
            } else {
                self.image_name.as_str()
            };
            let manifest_path = image_manifest_path(self.arch, image_name)?;
            let manifest = read_manifest(&manifest_path)?;
            let tree = harvey::harvey_dir(self.arch)?;
            let tier_path = tree.join(format!(
                "build/{}/tiers/tier0-libs.txt",
                build_dir_name(self.arch)
            ));
            let tier_text = std::fs::read_to_string(&tier_path).map_err(|e| -> DynError {
                format!(
                    "tier-0 list {}: {e} — the resolved Harvey tree must carry \
                     build/{{x86_64|aarch64}}/tiers/tier0-libs.txt (HARN-03)",
                    tier_path.display()
                )
                .into()
            })?;
            image::check_libraries(&manifest, &tree, &tier_text)?;
            println!("libraries: image {arch_name}/{image_name} tier-0 list OK");
            return Ok(());
        }
        if self.image_name.is_empty() {
            return Err(
                "image: --image <name> is required (except with --libraries, which defaults to \
                 the `libraries` manifest)"
                    .into(),
            );
        }
        let assembled = assemble_named_image(self.arch, &self.image_name, self.verbose)?;
        println!(
            "image: {arch_name}/{} {} sha256 {} members {}",
            self.image_name,
            assembled.paq.display(),
            assembled.sha256,
            assembled.members.len()
        );
        if self.compare_reference {
            let reference = reference_member_list(self.arch, &self.image_name)?;
            image::compare_members(&assembled, &reference)?;
            println!(
                "image: {arch_name}/{} matches the reference member list ({} members)",
                self.image_name,
                reference.len()
            );
        }
        Ok(())
    }

    /// `--emit-manifest`: derive a manifest from the reference member list
    /// and the tree, write it, then fail non-zero when any entry is
    /// UNRESOLVED (listing them) — a manifest can never silently omit a
    /// file (T-01-39).
    fn emit(&self, path: &Path) -> Result<()> {
        let reference = reference_member_list(self.arch, &self.image_name)?;
        let tree = harvey::harvey_dir(self.arch)?;
        let emitted =
            image::emit_manifest(&reference, &tree, objtype(self.arch)?, &self.image_name)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &emitted.text)
            .map_err(|e| -> DynError { format!("write {}: {e}", path.display()).into() })?;
        if emitted.unresolved.is_empty() {
            println!(
                "image: wrote {} from {} reference members",
                path.display(),
                reference.len()
            );
            Ok(())
        } else {
            eprintln!(
                "image: {} has {} UNRESOLVED entr{} — no tree file carries the reference bytes:",
                path.display(),
                emitted.unresolved.len(),
                if emitted.unresolved.len() == 1 { "y" } else { "ies" }
            );
            for unresolved in &emitted.unresolved {
                eprintln!("  {unresolved}");
            }
            Err("the emitted manifest cannot be assembled until every UNRESOLVED entry is \
                 resolved (HARN-03/T-01-39)"
                .into())
        }
    }
}

/// `cargo xtask initrd`: assemble
/// `target/bootfs.paq` from the per-file manifest
/// `xtask/manifests/arm64/smoke.toml` (D-15/D-16) through the checked
/// engine: every member is staged from the resolved Harvey tree into
/// `target/images/aarch64/`, drift-checked in both directions, baked with
/// the host `mkpaqfs` named by `R9_MKPAQFS`, and the image's sha256 is
/// printed for the evidence log.  The image, the staging tree and the
/// member list are target/ build artifacts — never committed, and nothing
/// is ever written into the Harvey tree (HARN-02).
struct InitrdStep {
    verbose: bool,
}

/// The initrd output image, relative to the workspace (config_virtexec
/// names this path; unchanged by D-16).
const BOOTFS_PAQ: &str = "target/bootfs.paq";

impl InitrdStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        Self { verbose: verbose(matches) }
    }

    fn run(self) -> Result<()> {
        assemble_initrd(self.verbose)
    }
}

/// The host `mkpaqfs` binary: `R9_MKPAQFS` names it explicitly (the nix
/// devshell exports it from plan 01-13 onward; before that a checkout's
/// `.build/bin/mkpaqfs` can be named).  There is no derived default —
/// mkpaqfs is not part of the Harvey TREE contract.
fn mkpaqfs_path() -> Result<PathBuf> {
    match env::var("R9_MKPAQFS") {
        Ok(value) if !value.trim().is_empty() => {
            let path = PathBuf::from(value);
            if !path.is_file() {
                return Err(format!(
                    "R9_MKPAQFS names {} but it is not a file — name the host mkpaqfs binary",
                    path.display()
                )
                .into());
            }
            Ok(path)
        }
        _ => Err("R9_MKPAQFS is not set — name the host mkpaqfs binary explicitly (the nix \
                  devshell exports it from plan 01-13 onward; before that a Harvey checkout's \
                  .build/bin/mkpaqfs can be named, e.g. \
                  R9_MKPAQFS=/path/to/harvey/.build/bin/mkpaqfs); xtask derives no default"
            .into()),
    }
}

/// The xtask output-directory name of an arch (`target/images/<name>/`).
fn arch_dir_name(arch: Arch) -> String {
    image::arch_dir(arch)
}

/// The Plan 9 objtype directory of an arch's manifests (`arm64`/`amd64`).
fn objtype(arch: Arch) -> Result<&'static str> {
    match arch {
        Arch::Aarch64 => Ok("arm64"),
        Arch::X86_64 => Ok("amd64"),
        Arch::Riscv64 => Err("a Harvey image manifest does not exist for riscv64 — the \
                              manifests are per guest architecture (arm64, amd64)"
            .into()),
    }
}

/// The Harvey tree's build-directory name of an arch
/// (`build/x86_64/...` / `build/aarch64/...` — the tree's own names, which
/// differ from the Plan 9 objtype used for manifests).
fn build_dir_name(arch: Arch) -> &'static str {
    match arch {
        Arch::Aarch64 => "aarch64",
        Arch::X86_64 => "x86_64",
        Arch::Riscv64 => "riscv64",
    }
}

/// `xtask/manifests/<objtype>/<image>.toml` in the workspace.
fn image_manifest_path(arch: Arch, image_name: &str) -> Result<PathBuf> {
    image::validate_image_name(image_name)?;
    Ok(workspace().join(format!(
        "xtask/manifests/{}/{image_name}.toml",
        objtype(arch)?
    )))
}

/// `target/images/<arch>/<image>.paq` in the workspace.
fn image_paq_path(arch: Arch, image_name: &str) -> PathBuf {
    workspace().join(image::image_paq(arch, image_name))
}

/// Read and parse one image's manifest with a named error.
fn read_manifest(path: &Path) -> Result<image::Manifest> {
    let text = std::fs::read_to_string(path).map_err(|e| -> DynError {
        format!(
            "image manifest {}: {e} — commit xtask/manifests/<objtype>/<image>.toml",
            path.display()
        )
        .into()
    })?;
    image::parse_manifest(&text)
}

/// Assemble `arch`'s named image through the checked engine into
/// `target/images/<arch>/`.  The drift and provenance checks run first
/// (HARN-05), so a bad manifest or swapped binary never reaches mkpaqfs.
fn assemble_named_image(
    arch: Arch,
    image_name: &str,
    verbose: bool,
) -> Result<image::Assembled> {
    let ws = workspace();
    let manifest_path = image_manifest_path(arch, image_name)?;
    if !manifest_path.is_file() {
        return Err(format!(
            "image manifest {} does not exist — commit the manifest first",
            manifest_path.display()
        )
        .into());
    }
    // The output files are named from the manifest's `image`, not from the
    // CLI value. Require them to agree so a renamed/copied manifest cannot
    // write an image under a name the caller did not ask for (CR-01).
    let manifest = read_manifest(&manifest_path)?;
    if manifest.image != image_name {
        return Err(format!(
            "image manifest {} declares image = {:?} but the request named {:?} — the output \
             files are named from the manifest, so the two must agree",
            manifest_path.display(),
            manifest.image,
            image_name
        )
        .into());
    }
    // The per-architecture resolver: R9_HARVEY_DIR wins, then
    // R9_HARVEY_DIR_ARM64/_AMD64 (D-21).  No built-in default path (D-01).
    let tree = harvey::harvey_dir(arch)?;
    let mkpaqfs = mkpaqfs_path()?;
    let out_dir = ws.join(format!("target/images/{}", arch_dir_name(arch)));
    if verbose {
        println!(
            "Executing {} -o {} {}",
            mkpaqfs.display(),
            out_dir.join(format!("{image_name}.paq")).display(),
            out_dir.join(format!("{image_name}.stage")).display()
        );
    }
    image::assemble_checked(&manifest_path, &tree, std::slice::from_ref(&tree), &ws, &out_dir, &mkpaqfs)
}

/// Whether `arch`'s named image is missing or older than its manifest or
/// any of its sources (the staleness check the on-demand `image:` scheme
/// reads).
fn image_stale(arch: Arch, image_name: &str, paq: &Path) -> Result<bool> {
    let ws = workspace();
    let manifest_path = image_manifest_path(arch, image_name)?;
    let manifest = read_manifest(&manifest_path)?;
    let tree = harvey::harvey_dir(arch)?;
    let Some(paq_mtime) = std::fs::metadata(paq).and_then(|m| m.modified()).ok() else {
        return Ok(true);
    };
    let manifest_dir = manifest_path.parent().unwrap_or(&ws);
    let mut inputs = image::input_paths(&manifest, manifest_dir, &tree, &ws);
    inputs.push(manifest_path);
    Ok(inputs.iter().any(|input| {
        std::fs::metadata(input)
            .and_then(|m| m.modified())
            .ok()
            .is_some_and(|input_mtime| input_mtime > paq_mtime)
    }))
}

/// Resolve an image named by the `image:` scheme, assembling it when the
/// `.paq` is missing or stale (D-16: profiles stop reading Harvey-baked
/// images).  The printed line is the observable on-demand path.
fn ensure_named_image(arch: Arch, image_name: &str, verbose: bool) -> Result<PathBuf> {
    let paq = image_paq_path(arch, image_name);
    if image_stale(arch, image_name, &paq)? {
        println!(
            "xtask: image {}/{} missing or stale — assembling",
            arch_dir_name(arch),
            image_name
        );
        let assembled = assemble_named_image(arch, image_name, verbose)?;
        println!(
            "image: {}/{} {} sha256 {} members {}",
            arch_dir_name(arch),
            image_name,
            assembled.paq.display(),
            assembled.sha256,
            assembled.members.len()
        );
    }
    Ok(paq)
}

/// The reference bake's member list for `arch`'s named image: plans 01-15
/// and 01-17 export `R9_HARVEY_REFERENCE_AMD64` / `_ARM64`; unset is a
/// named error, never a skipped comparison.
fn reference_member_list(arch: Arch, image_name: &str) -> Result<Vec<image::Member>> {
    let var = match arch {
        Arch::Aarch64 => "R9_HARVEY_REFERENCE_ARM64",
        Arch::X86_64 => "R9_HARVEY_REFERENCE_AMD64",
        Arch::Riscv64 => {
            return Err("a reference member list does not exist for riscv64 — the image \
                        manifests are per guest architecture (arm64, amd64)"
                .into())
        }
    };
    let root = match env::var(var) {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => {
            return Err(format!(
                "{var} is not set — --compare-reference and --emit-manifest read \
                 <{var}>/{image_name}.members (the reference bake's D-29 member list); plans \
                 01-15 and 01-17 export it"
            )
            .into())
        }
    };
    let path = root.join(format!("{image_name}.members"));
    let text = std::fs::read_to_string(&path).map_err(|e| -> DynError {
        format!(
            "reference member list {}: {e} — {var} names the reference bake's directory",
            path.display()
        )
        .into()
    })?;
    image::parse_reference_members(&text)
}

/// Assemble target/bootfs.paq from the per-file smoke manifest through the
/// checked engine (D-16): the smoke image gets the two-way drift and Rust
/// provenance checks too.  The staged init binds #p at /proc itself in the
/// namespace era (the real chain's connectembed shape), so the manifest
/// carries the empty `/proc` dir.  A missing R9_MKPAQFS, a missing
/// manifest source or a manifest/image disagreement is a loud build error,
/// never a silent boot.
fn assemble_initrd(verbose: bool) -> Result<()> {
    let ws = workspace();
    let assembled = assemble_named_image(Arch::Aarch64, "smoke", verbose)?;
    let paq = ws.join(BOOTFS_PAQ);
    std::fs::copy(&assembled.paq, &paq)?;
    // Evidence: print the image's sha256 (mkpaqfs embeds a time field, so the
    // digest is per-bake — the phase log records each evidence run's value).
    println!("initrd: {} sha256 {}", paq.display(), assembled.sha256);
    Ok(())
}

/// One churn gate's marker vocabulary (17-02's q35churn set and its
/// 20-09 aarch64 mirror, virtchurn) — the shared post-pass reads the
/// capture through it.
struct ChurnMarkers {
    heap: &'static str,
    slope: &'static str,
    done: &'static str,
    fail: &'static str,
    mark: &'static str,
    semgen: &'static str,
}

const Q35_CHURN_MARKERS: ChurnMarkers = ChurnMarkers {
    heap: Q35_CHURN_HEAP_MARKER,
    slope: Q35_CHURN_SLOPE_MARKER,
    done: Q35_CHURN_DONE_MARKER,
    fail: Q35_CHURN_FAIL_MARKER,
    mark: Q35_CHURN_MARK_MARKER,
    semgen: Q35_CHURN_SEMGEN_MARKER,
};

const VIRT_CHURN_MARKERS: ChurnMarkers = ChurnMarkers {
    heap: VIRT_CHURN_HEAP_MARKER,
    slope: VIRT_CHURN_SLOPE_MARKER,
    done: VIRT_CHURN_DONE_MARKER,
    fail: VIRT_CHURN_FAIL_MARKER,
    mark: VIRT_CHURN_MARK_MARKER,
    semgen: VIRT_CHURN_SEMGEN_MARKER,
};

/// The numbers a green churn post-pass reports.
#[derive(Debug, PartialEq)]
struct ChurnVerdict {
    iterations: u64,
    want_marks: u64,
    semgens: u64,
    steady: u64,
    measured: u64,
    ceiling: u64,
}

/// The finish()-time churn heap-bound post-pass (17-02, T-17-04: the
/// bound has teeth; shared with the aarch64 virtchurn gate in 20-09):
/// every number is PARSED from the kernel's own markers — the DONE
/// line's iteration/generation counts, the tagged heap samples, the
/// slope line's steady/measured/ceiling — never a second hardcoded
/// constant.  The kernel already compared measured vs ceiling
/// fail-closed (DONE withheld on FAIL, and the fast-fail arm aborts on
/// the first FAIL line); this re-derives the verdict from the RAW
/// samples, so a kernel-side comparison bug cannot certify a leak as
/// bounded.
fn churn_post_pass(capture: &[u8], m: &ChurnMarkers) -> Result<ChurnVerdict> {
    let field = |line: &str, key: &str| -> Option<u64> {
        line.split_whitespace()
            .find_map(|t| t.strip_prefix(key).and_then(|v| v.parse().ok()))
    };
    let done_line = line_containing(capture, m.done.as_bytes())
        .ok_or_else(|| "churn gate: no DONE line in capture".to_string())?;
    let iterations = field(&done_line, "iterations=")
        .ok_or_else(|| format!("churn gate: DONE line lacks iterations=: {done_line}"))?;
    let semgens = field(&done_line, "semgens=")
        .ok_or_else(|| format!("churn gate: DONE line lacks semgens=: {done_line}"))?;
    if semgens > iterations {
        return Err(format!(
            "churn gate: semgens={semgens} exceeds iterations={iterations} (driver bookkeeping)"
        ).into());
    }
    // (a) zero FAIL lines whole-capture (the fast-fail
    //     already aborted on the first; belt-and-braces).
    let fails = count_marker_lines(capture, m.fail);
    if fails != 0 {
        return Err(format!("churn gate: {fails} {} line(s)", m.fail).into());
    }
    // (b) the exact-count contract, against the kernel's
    //     own numbers: every iteration entered ring 3
    //     (churnmark's own prints — D-09) and every
    //     generation's contenders completed.
    let marks = count_marker_lines(capture, m.mark) as u64;
    let want_marks = iterations - semgens;
    if marks != want_marks {
        return Err(format!(
            "churn gate: churnmark: ready COUNT FAILED — {marks} for iterations={iterations} - semgens={semgens} (short names a lost exec, long names a duplicate)"
        ).into());
    }
    let gens = count_marker_lines(capture, m.semgen) as u64;
    if gens != semgens {
        return Err(format!(
            "churn gate: semgen COUNT FAILED — {gens} for semgens={semgens}"
        ).into());
    }
    // (c) the slope verdict, re-derived from the raw
    //     samples: the window between the `heap base` and
    //     `heap end` sample lines carries the churn leg —
    //     its mark + generation counts reconstruct the
    //     kernel's exact denominator.
    let slope_line = line_containing(capture, m.slope.as_bytes())
        .ok_or_else(|| "churn gate: no slope line in capture".to_string())?;
    let steady = field(&slope_line, "steady=")
        .ok_or_else(|| format!("churn gate: slope line lacks steady=: {slope_line}"))?;
    let measured = field(&slope_line, "measured=")
        .ok_or_else(|| format!("churn gate: slope line lacks measured=: {slope_line}"))?;
    let ceiling = field(&slope_line, "ceiling=")
        .ok_or_else(|| format!("churn gate: slope line lacks ceiling=: {slope_line}"))?;
    let base_end = last_marker_line_end(capture, &format!("{}base", m.heap))
        .ok_or_else(|| "churn gate: no `churngate: heap base` sample line".to_string())?;
    let end_start = marker_position(capture, format!("{}end", m.heap).as_bytes())
        .ok_or_else(|| "churn gate: no `churngate: heap end` sample line".to_string())?;
    if base_end > end_start {
        return Err("churn gate: heap end sample precedes heap base".into());
    }
    let window = &capture[base_end..end_start];
    let leg_marks = count_marker_lines(window, m.mark) as u64;
    let leg_gens = count_marker_lines(window, m.semgen) as u64;
    if leg_marks + leg_gens == 0 {
        return Err("churn gate: empty churn-leg window (sample anchors wrong)".into());
    }
    // The sample lines' shape: `churngate: heap <tag>
    // <used> <total>` — the cursor is the 4th token.
    let sample_used = |tag: &str| -> Option<u64> {
        let needle = format!("{}{tag}", m.heap);
        let l = line_containing(capture, needle.as_bytes())?;
        l.split_whitespace().nth(3).and_then(|t| t.parse().ok())
    };
    let base_used = sample_used("base")
        .ok_or_else(|| "churn gate: heap base sample lacks its cursor bytes".to_string())?;
    let end_used = sample_used("end")
        .ok_or_else(|| "churn gate: heap end sample lacks its cursor bytes".to_string())?;
    if end_used < base_used {
        return Err(format!(
            "churn gate: cursor SHRANK ({base_used} -> {end_used}) — impossible by the slope doctrine"
        ).into());
    }
    let recomputed = (end_used - base_used) / (leg_marks + leg_gens);
    if recomputed != measured {
        return Err(format!(
            "churn gate: slope re-derivation mismatch — kernel measured={measured}, host recomputed {recomputed} from ({end_used}-{base_used})/({leg_marks}+{leg_gens})"
        ).into());
    }
    if measured > ceiling {
        return Err(format!(
            "churn gate: heap bound FAILED host-side — measured={measured} > ceiling={ceiling} bytes/exec (the kernel's own comparison should have withheld DONE)"
        ).into());
    }
    Ok(ChurnVerdict { iterations, want_marks, semgens, steady, measured, ceiling })
}

/// Build-if-missing/stale hook: see [`image_stale`].
/// config carries an initrd image under target/ (the assembled bootfs.paq)
/// reassemble it when absent or older than the staged inputs.  Configs with
/// no initrd, or with a target/-external image (the committed probe fixture —
/// pinned bytes), pass through untouched.
fn ensure_initrd_image(config: &Configuration, arch: Arch, verbose: bool) -> Result<()> {
    let Some(qemu) = &config.qemu else { return Ok(()) };
    let Some(initrd) = &qemu.initrd else { return Ok(()) };
    if let Some(name) = initrd.file.strip_prefix("image:") {
        if name.is_empty() {
            return Err("qemu initrd: bare `image:` prefix names no image (want image:<name>)"
                .into());
        }
        // D-16: the named image is assembled on demand into
        // target/images/<arch>/ when missing or stale.  The same engine
        // ImageStep runs, so the smoke image and every later image carry
        // the drift and provenance checks.
        ensure_named_image(arch, name, verbose)?;
        return Ok(());
    }
    // A harvey:-prefixed image is produced by the Harvey checkout's own
    // pipeline and consumed as-is: it passes through the target/-resident
    // auto-bake untouched, but absence is a loud error naming the producers
    // (07-05's virtnet profile boots the Harvey-baked bootfs.paq).  The
    // path resolves through the config arch's tree (D-21).
    if initrd.file.starts_with("harvey:") {
        let path = initrd.resolved_file(arch)?;
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
    // The target/-resident image is the aarch64 smoke bake (the only
    // manifest this plan carries); an x86_64 profile naming one would
    // silently receive the wrong architecture's userland.
    if arch != Arch::Aarch64 {
        return Err(format!(
            "initrd image {} lives under target/ and is assembled from the aarch64 smoke \
             manifest; no target/-resident image exists for {arch} yet",
            initrd.file
        )
        .into());
    }
    let paq = workspace().join(&initrd.file);
    // The staleness check reads the manifest's inputs through the resolver;
    // the manifest file itself is an input too (an edited manifest rebakes).
    if image_stale(Arch::Aarch64, "smoke", &paq)? {
        // assemble_initrd fails loudly with the producing command named when
        // R9_MKPAQFS or a manifest source is absent.
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
            let path = resolve_boot_module(spec, Arch::X86_64)?;
            let dest = stage.join("boot/bootfs.paq");
            std::fs::copy(&path, &dest).map_err(|e| format!("stage module {path:?}: {e}"))?;
            println!("iso: module {} staged at /boot/bootfs.paq", path.display());
            true
        }
        None => false,
    };
    let template = std::fs::read_to_string(workspace().join("x86_64/lib/grub.cfg"))
        .map_err(|e| format!("read grub.cfg template: {e}"))?;
    // 19-01 (CPU-02): the append seam — the config's [dist] append key
    // substitutes for the template's @R9-APPEND@ token on the
    // multiboot2 line.  Compose-time validation: a configured append
    // that carries no content, exceeds the kernel's 256-byte cmdline
    // cap (mb2::MAX_CMDLINE — the boot would reject it loudly at first
    // boot), or carries control characters (it is one loader line and
    // one NUL-terminated tag string) is refused HERE, never staged.
    let append = validate_append(config.dist.as_ref().and_then(|d| d.append.as_deref()))?;
    if let Some(a) = append {
        println!("iso: kernel cmdline append {a:?} (mb2 tag 1, the bootconf carrier)");
    }
    std::fs::write(stage.join("boot/grub/grub.cfg"), emit_grub_cfg(&template, module_staged, append)?)?;

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

/// The kernel cmdline's byte cap — mirrors the kernel-side
/// `mb2::MAX_CMDLINE` (the bootconf string's loud reject threshold at
/// parse); compose time refuses EARLIER so a mis-sized append never
/// reaches an ISO the boot would then refuse at first boot.
pub const MAX_CMDLINE_APPEND: usize = 256;

/// Compose-time append validation (19-01, CPU-02): a CONFIGURED append
/// must carry real content, fit the kernel's cmdline cap, and be
/// printable (one loader line, one NUL-terminated tag string).  An
/// absent key passes through untouched (the no-append emission stays
/// byte-identical to the pre-token config).
fn validate_append(append: Option<&str>) -> std::result::Result<Option<&str>, String> {
    let Some(a) = append else {
        return Ok(None);
    };
    if a.trim().is_empty() {
        return Err(format!(
            "dist append: the configured append {a:?} carries no bootconf content — drop the \
             key or set a real cmdline line"
        ));
    }
    if a.len() > MAX_CMDLINE_APPEND {
        return Err(format!(
            "dist append: {} bytes exceeds the kernel cmdline cap of {MAX_CMDLINE_APPEND} \
             (mb2::MAX_CMDLINE — the boot would reject it loudly at first boot)",
            a.len()
        ));
    }
    if a.bytes().any(|b| !(0x20..0x7f).contains(&b)) {
        return Err(format!(
            "dist append: {a:?} carries non-printable bytes — the cmdline is one loader line \
             and one NUL-terminated tag string"
        ));
    }
    Ok(Some(a))
}

/// Emit the staged loader config from the committed template (14-05,
/// D-03; 19-01 adds the append token): `# @R9-MODULE-DOC@` and
/// `# @R9-APPEND-DOC@` lines are template documentation dropped by
/// every emitter; a line carrying the `# @R9-MODULE@ ` marker is the
/// module2 placeholder — emitted IN (the marker stripped, the line's
/// indentation and payload kept) when a boot module is staged, dropped
/// OUT for kernel-only ISOs.  The `@R9-APPEND@` token on the
/// multiboot2 line substitutes ` <append>` when one is configured and
/// strips to nothing when none is — the kernel-only AND module-bearing
/// no-append emissions stay byte-identical to Phase 13's committed
/// grub.cfg, the property the wired boot-iso FOD hash rides on (the
/// unit test pins it against the exact 13-line config).  A configured
/// append against a tokenless template is a LOUD error — the bootconf
/// would be silently dropped (the seam drifted).  14-07's flake
/// iso-builder shares the convention.
fn emit_grub_cfg(template: &str, module_staged: bool, append: Option<&str>) -> std::result::Result<String, String> {
    const MARKER: &str = "# @R9-MODULE@ ";
    const APPEND_TOKEN: &str = "@R9-APPEND@";
    const DOC_MARKERS: [&str; 2] = ["# @R9-MODULE-DOC@", "# @R9-APPEND-DOC@"];
    if append.is_some() && !template.contains(APPEND_TOKEN) {
        return Err(format!(
            "grub.cfg template lacks the {APPEND_TOKEN} token — a configured append would be \
             silently dropped (the committed template and this emitter drifted; fix the template)"
        ));
    }
    let subst = append.map(|a| format!(" {a}")).unwrap_or_default();
    let mut out = String::new();
    for line in template.lines() {
        // Doc lines drop WHOLE, before the marker scan: the committed
        // template's documentation EMBEDS the marker literal in
        // backticks (`# @R9-MODULE@ `), and a marker-first order
        // mangled that comment through the module branch — the
        // module-bearing emission carried a half-stripped doc line
        // (latent since 14-05, caught by 19-01's module-bearing
        // byte-stability pin).  The placeholder line itself never
        // starts with a doc marker (`@R9-MODULE@ ` vs
        // `@R9-MODULE-DOC@` differ right after MODULE), so the order
        // is safe.
        if DOC_MARKERS.iter().any(|m| line.trim_start().starts_with(m)) {
            continue;
        }
        if let Some(pos) = line.find(MARKER) {
            if module_staged {
                out.push_str(&line[..pos]);
                out.push_str(&line[pos + MARKER.len()..]);
                out.push('\n');
            }
        } else if let Some(pos) = line.find(APPEND_TOKEN) {
            out.push_str(&line[..pos]);
            out.push_str(&subst);
            out.push_str(&line[pos + APPEND_TOKEN.len()..]);
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(out)
}

/// Resolve a [dist] module path: an `image:`-prefixed value names an r9
/// image assembled on demand into `target/images/<arch>/<name>.paq` (D-16);
/// a `harvey:`-prefixed value names a file under `arch`'s tree
/// (harvey::harvey_dir — the 07-05 initrd channel's discipline; r9 never
/// hardcodes a sibling layout); ordinary workspace-relative values pass
/// through.  Absence is a LOUD error naming the producers — never a silent
/// kernel-only ISO when the profile expects its bootfs.
fn resolve_boot_module(spec: &str, arch: Arch) -> Result<PathBuf> {
    if let Some(name) = spec.strip_prefix("image:") {
        if name.is_empty() {
            return Err(
                "dist module: bare `image:` prefix names no image (want image:<name>)".into(),
            );
        }
        let path = ensure_named_image(arch, name, false)?;
        if !path.is_file() {
            return Err(format!(
                "boot module {spec} not found at {} — the image assembler did not produce it",
                path.display()
            )
            .into());
        }
        return Ok(path);
    }
    let path = match spec.strip_prefix("harvey:") {
        Some(rest) if !rest.is_empty() => {
            let dir = harvey::harvey_dir(arch).map_err(|e| e.to_string())?;
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

struct RforkHeaderStep {
    check: bool,
}

impl RforkHeaderStep {
    fn new(matches: &clap::ArgMatches) -> Self {
        Self { check: matches.get_flag("check") }
    }

    fn run(self) -> Result<()> {
        if self.check {
            xtask::rforkh::check(&workspace())?;
            println!("rfork-header: sys/include/rfork.h matches port/src/rfork.rs");
        } else {
            xtask::rforkh::write(&workspace())?;
            println!("rfork-header: wrote sys/include/rfork.h");
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
        check_mb2_header, contains_marker, count_marker_lines, count_register_lines,
        emit_grub_cfg, last_marker_line_end, marker_position, negative_scan_hit_for,
        uses_host_triple_fallback, validate_append, BOOT_ECHO_PROBE, BOOT_Q35_MEMORY_MARKER,
        BOOT_TICK_MARKER, BOOT_TICK_MIN, MAX_CMDLINE_APPEND, PCI_NET_DEVICE_MARKER,
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
    /// that leaks a `# @R9-MODULE*`/`# @R9-APPEND*` line or a stray
    /// @R9-APPEND@ token into a kernel-only ISO fails HERE, before any
    /// boot.  19-01: both no-append shapes (kernel-only AND
    /// module-bearing) pin the empty-append byte-stability.
    #[test]
    fn grub_cfg_kernel_only_emission_is_phase13_identical() {
        let template = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../x86_64/lib/grub.cfg"),
        )
        .expect("committed grub.cfg template readable");
        let emitted = emit_grub_cfg(&template, false, None).expect("emission");
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
        // The module-bearing no-append emission over the SAME committed
        // template: identical to the kernel-only one except the module2
        // line IN — the current staged-config shape, byte-stable under
        // the append seam (19-01's pin).
        let module_emitted =
            emit_grub_cfg(&template, true, None).expect("module emission");
        let phase13_module = phase13.replace(
            "    multiboot2 /boot/kernel\n",
            "    multiboot2 /boot/kernel\n    module2 /boot/bootfs.paq\n",
        );
        assert_eq!(module_emitted, phase13_module);
    }

    /// 19-01 (CPU-02): the append seam — a configured append substitutes
    /// ` <cmdline>` for the @R9-APPEND@ token on the multiboot2 line
    /// (module-bearing and kernel-only alike), and a configured append
    /// against a tokenless template is a LOUD error (the bootconf would
    /// be silently dropped).
    #[test]
    fn grub_cfg_append_substitutes_on_the_multiboot_line() {
        let template = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../x86_64/lib/grub.cfg"),
        )
        .expect("committed grub.cfg template readable");
        // The cpu image's bootconf line, module-bearing (the q35cpu
        // shape): the multiboot2 line carries the append; the doc lines
        // and the token are gone.
        let emitted = emit_grub_cfg(&template, true, Some("init=/amd64/init -c"))
            .expect("append emission");
        assert!(
            emitted
                .lines()
                .any(|l| l == "    multiboot2 /boot/kernel init=/amd64/init -c"),
            "the multiboot2 line must carry the append verbatim:\n{emitted}"
        );
        assert!(emitted.lines().any(|l| l == "    module2 /boot/bootfs.paq"));
        assert!(!emitted.contains("@R9-APPEND"));
        // Kernel-only with an append: same substitution, no module line.
        let emitted = emit_grub_cfg(&template, false, Some("a=1")).expect("emission");
        assert!(emitted.lines().any(|l| l == "    multiboot2 /boot/kernel a=1"));
        assert!(!emitted.contains("module2"));

        // The loud drift class: a configured append on a template that
        // lost its token refuses — never a silent drop.
        let tokenless = concat!(
            "menuentry \"r9\" {\n",
            "    multiboot2 /boot/kernel\n",
            "    boot\n",
            "}\n",
        );
        assert!(emit_grub_cfg(tokenless, false, Some("a=1")).is_err());
        // The same template without an append still emits (the token
        // simply is not there to strip).
        assert_eq!(emit_grub_cfg(tokenless, false, None).unwrap(), tokenless);
    }

    /// 19-01: compose-time append validation — the kernel rejects
    /// overlong bootconfs loudly at first boot; the emitter refuses
    /// EARLIER.  Whitespace-only and control-character appends never
    /// reach an ISO (one loader line, one NUL-terminated tag string).
    #[test]
    fn append_validation_is_loud_at_compose_time() {
        assert_eq!(validate_append(None), Ok(None));
        assert_eq!(validate_append(Some("init=/amd64/init -c")), Ok(Some("init=/amd64/init -c")));
        assert!(validate_append(Some("   ")).is_err());
        assert!(validate_append(Some("\t")).is_err());
        let over = "k=".to_string() + &"a".repeat(MAX_CMDLINE_APPEND);
        assert!(validate_append(Some(&over)).is_err());
        // Exactly at the cap passes (256 is the boundary, not over it).
        let at_cap = "k=".to_string() + &"a".repeat(MAX_CMDLINE_APPEND - 2);
        assert_eq!(validate_append(Some(&at_cap)), Ok(Some(at_cap.as_str())));
        assert!(validate_append(Some("a=1\nb=2")).is_err());
        assert!(validate_append(Some("a=1\tb=2")).is_err());
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
        let emitted = emit_grub_cfg(&template, true, None).expect("emission");
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
            emit_grub_cfg(&template, false, None).unwrap(),
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

    /// The real-boot-chain profiles (net, netip, q35shell, q35churnshell,
    /// q35cpu) exempt exactly the D-02 designed fauth line (boot(8)'s
    /// auth handshake prints `syscall 10 unimplemented`, now pid-tagged
    /// by the kernel); any OTHER unimplemented syscall still fails the run.
    #[test]
    fn negative_scan_exempts_boot_fauth_only() {
        assert_eq!(negative_scan_hit_for(b"syscall 10 unimplemented\r\n", true), None);
        // 01-33: the kernel's line now names its caller; the exemption is
        // a prefix match, so the pid/process suffix must not defeat it.
        assert_eq!(
            negative_scan_hit_for(b"syscall 10 unimplemented (pid 2 boot)\r\n", true),
            None
        );
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

    /// A synthetic virtchurn capture: the sample ladder, `marks`
    /// churnmark prints and `gens` semgen lines in the churn leg, the
    /// slope line and the DONE line, with the cursor at `base_used` and
    /// `end_used`.
    fn churn_capture(base_used: u64, end_used: u64, measured: u64, ceiling: u64) -> Vec<u8> {
        let mut c = String::new();
        c.push_str("churngate: heap start 1000 33554432\r\n");
        c.push_str("churnmark: ready\r\n");
        c.push_str("churngate: heap one 2000 33554432\r\n");
        c.push_str("churnmark: ready\r\n");
        c.push_str(&format!("churngate: heap base {base_used} 33554432\r\n"));
        // churn leg: 4 marks + 1 semgen (the denominator is 5)
        for _ in 0..4 {
            c.push_str("churnmark: ready\r\n");
            c.push_str("churngate: heap exit 9 33554432\r\n");
        }
        c.push_str("churngate: semgen 4 contenders done\r\n");
        c.push_str(&format!("churngate: heap end {end_used} 33554432\r\n"));
        c.push_str(&format!("churngate: slope steady=0 measured={measured} ceiling={ceiling}\r\n"));
        c.push_str("churngate: DONE iterations=7 semgens=1\r\n");
        c.into_bytes()
    }

    // 20-09: the shared churn post-pass over the virtchurn vocabulary —
    // a capture whose re-derived slope matches the kernel's and sits
    // under the ceiling passes, reporting the parsed numbers.
    #[test]
    fn virtchurn_post_pass_accepts_a_bounded_slope() {
        let cap = churn_capture(10_000, 15_000, 1000, 1772);
        let v = super::churn_post_pass(&cap, &super::VIRT_CHURN_MARKERS).expect("bounded slope");
        assert_eq!(
            v,
            super::ChurnVerdict {
                iterations: 7,
                want_marks: 6,
                semgens: 1,
                steady: 0,
                measured: 1000,
                ceiling: 1772
            }
        );
    }

    // ...a slope over the ceiling fails host-side even with a DONE line
    // (the kernel-side comparison bug the re-derivation exists for)...
    #[test]
    fn virtchurn_post_pass_rejects_an_over_ceiling_slope() {
        let cap = churn_capture(10_000, 20_000, 2000, 1772);
        let err = super::churn_post_pass(&cap, &super::VIRT_CHURN_MARKERS).unwrap_err();
        assert!(err.to_string().contains("heap bound FAILED"), "{err}");
    }

    // ...a kernel `measured=` that the raw samples do not reproduce
    // fails, and so does any FAIL line.
    #[test]
    fn virtchurn_post_pass_rejects_mismatch_and_fail_lines() {
        let cap = churn_capture(10_000, 15_000, 999, 1772);
        let err = super::churn_post_pass(&cap, &super::VIRT_CHURN_MARKERS).unwrap_err();
        assert!(err.to_string().contains("re-derivation mismatch"), "{err}");
        let mut cap = churn_capture(10_000, 15_000, 1000, 1772);
        cap.extend_from_slice(b"churngate: FAIL exec child did not exit\r\n");
        let err = super::churn_post_pass(&cap, &super::VIRT_CHURN_MARKERS).unwrap_err();
        assert!(err.to_string().contains("churngate: FAIL"), "{err}");
    }

    // The aarch64 vocabulary mirrors the x86_64 one string for string
    // (the kernel driver and the staged binaries print x86_64's
    // vocabulary verbatim on both arches).
    #[test]
    fn virtchurn_markers_mirror_q35churn() {
        let (q, v) = (&super::Q35_CHURN_MARKERS, &super::VIRT_CHURN_MARKERS);
        assert_eq!(
            (q.heap, q.slope, q.done, q.fail, q.mark, q.semgen),
            (v.heap, v.slope, v.done, v.fail, v.mark, v.semgen)
        );
    }

    /// D-24: on both host arches the nix devshell ships std for, the
    /// arch-crate leg must fall back to the host triple when rustup
    /// reports no target; the riscv64 leg must not.
    #[test]
    fn host_triple_fallback_covers_aarch64_and_x86_64() {
        assert!(uses_host_triple_fallback("aarch64"));
        assert!(uses_host_triple_fallback("x86_64"));
        assert!(!uses_host_triple_fallback("riscv64"));
    }
}
