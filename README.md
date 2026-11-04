# r9
[Plan 9](https://plan9.io/plan9/) in Rust

R9 is a reimplementation of the plan9 kernel in Rust.  It is
not only inspired by but in many ways derived from the original
[Plan 9](https://plan9.io/plan9/) source code.

## Building

We use `cargo` and the `xtask` pattern to build the kernel.

To build r9 for x86_64, we assume you have cloned the git repository
somewhere convenient.  Then simply change into the top-level
directory and, `cargo xtask build --arch x86-64`.

To build for aarch64, run `cargo xtask build --arch aarch64` (supported platforms: Raspberry Pi 3B, Raspberry Pi 4B via `--config raspi4b`, and qemu's `virt` machine via `--config virt`).

There are other useful `xtask` subcommands; run
`cargo xtask help` to see what is available.

Right now, r9 is not self-hosting.

## Runtime Dependencies

`cargo xtask dist`, which `cargo xtask qemu` depends on, requires `llvm-objcopy`. 
This is expected to live in the rust toolchain path.  You can install by running:
```
rustup component add llvm-tools
```

If you get `No such file or directory (os error 2)` messages, 
then install `llvm` separate from the rust toolchain and set:
```
OBJCOPY=$(which llvm-objcopy) cargo xtask qemukvm
```

If `No such file or directory (os error 2)` messages persist, 
check to ensure `qemu` or `qemu-kvm` is installed and the 
`qemu-system-x86_64` binary is in your path (or `qemu-system-aarch64` in the case of aarch64).

## Boot-test / ureg-check environment contract

`cargo xtask boot-test` (any profile), `cargo xtask ureg-check`, and `cargo xtask initrd` require the `R9_HARVEY_DIR` environment variable naming a Harvey checkout (it must contain `aarch64/include/ureg.h`; the initrd bake additionally needs `build/aarch64/smoke-out/` (261103-p64's split — the smoke stage's dedicated dir) + `.build/bin/mkpaqfs`). Unset or empty is a hard error naming the variable — there is no `../harvey` default (Phase 5, D-10: r9 knows nothing about sibling checkouts by default).

## Nix packages

With flakes enabled, `nix build .#kernel-virt` builds the shell-capable aarch64 kernel in a sandbox (the virtshell profile, debug, toolchain pinned by `rust-toolchain.toml`) and installs `kernel-virt.elf` plus `aarch64/lib/virt-gicv2.dtb` and `aarch64/lib/virt-gicv3.dtb` — the artifact Harvey's flake consumes as `inputs.r9.packages.<system>.kernel-virt` (Phase 5, D-11).  The kernel dispatches its GIC driver on the DTB's compatible string, so one ELF serves both machine shapes; Harvey's boot apps pick gicv3 + `-accel hvf -cpu host` on darwin-aarch64 (gicv3 + TCG elsewhere).

## Running on Qemu

R9 can be run using qemu for the various supported architectures:

|Arch|Platform|Commandline|
|----|--------|-----------|
|aarch64|raspi3b|cargo xtask qemu --arch aarch64 --verbose|
|aarch64|raspi4b|cargo xtask qemu --arch aarch64 --config raspi4b --verbose|
|aarch64|virt|cargo xtask qemu --arch aarch64 --config virt|
|aarch64|virt (boot-test harness)|cargo xtask boot-test --arch aarch64 --config virt|
|aarch64|virt (kernel-panic profile)|cargo xtask boot-test --arch aarch64 --config virtpanic|
|aarch64|virt (userspace-exec, SC#2)|cargo xtask boot-test --arch aarch64 --config virtexec|
|aarch64|virt (preemptive multi-proc smoke)|cargo xtask boot-test --arch aarch64 --config virtmulti|
|aarch64|virt (notes smoke, SC#4)|cargo xtask boot-test --arch aarch64 --config virtnotes|
|aarch64|virtshell (Harvey boot chain: pid 1 = `/boot/boot`)|built by `cargo xtask build --arch aarch64 --config virtshell` — not a boot-test profile: its paqfs image is baked and delivered Harvey-side (see `packages.kernel-virt` below; Harvey's `nix run .#boot-r9` boots it to an interactive `/bin/rc`)|
|x86-64|q35|cargo xtask qemu --arch x86-64 --verbose|
|x86-64 (with kvm)|q35|cargo xtask qemu --arch x86-64 --kvm --verbose|
|riscv|virt|cargo xtask qemu --arch riscv64 --verbose|

## Running on Real Hardware™️

R9 has been run on the following hardware to a greater or lesser degree:
- Raspberry Pi 4 (Gets as far as printing 'r9' via the miniuart)

### Raspberry Pi, Netboot

Assuming you can set up a TFTP server (good luck, it's incredibly fiddly, but for what it's worth, dnsmasq can work occasionally), and assuming the location of your netboot directory, you can build and copy the binary using the following command:
```
cargo xtask dist --arch aarch64 --verbose && cp target/aarch64-unknown-none-elf/debug/aarch64-qemu.gz ../netboot/kernel8.img
```

This copies a compressed binary, which should be much faster to copy across the network.

The Raspberry Pi firmware loads `config.txt` before the kernel.  Here we can set which UART to use, amongst other things.  The following contents will set up to use the miniuart:
```
enable_uart=1
core_freq_min=500
```