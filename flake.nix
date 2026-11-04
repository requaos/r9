{
  description = "r9 dev environment — pinned rust toolchain, qemu, cargo xtask";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, rust-overlay }:
    let
      # Intel-Mac tuple deliberately omitted — out of project scope (REQUIREMENTS.md)
      systems = [ "aarch64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      # rust-toolchain.toml is the single source of truth for channel,
      # components and targets (D-09) — never restate pins here; r9's
      # "bump rust" commits must not require dual-pin edits.
      toolchainFor = system:
        (pkgsFor system).rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      # llvm-tools lands in the toolchain's rustlib dir; xtask's objcopy()
      # already honors $OBJCOPY first (xtask/src/main.rs:246-271).
      # NB: nixpkgs' bintools wrapper (present even under mkShellNoCC —
      # stdenv keeps ar/strip for fixup) unconditionally exports
      # OBJCOPY=objcopy, a bare-name placeholder, BEFORE the shellHook runs.
      # Treat only absolute-path values as genuine user overrides.
      objcopyHook = toolchain: ''
        case "''${OBJCOPY:-}" in
          /*) : ;;  # user override (absolute path) — keep
          *) export OBJCOPY="$(ls ${toolchain}/lib/rustlib/*/bin/llvm-objcopy 2>/dev/null | head -1)" ;;
        esac
      '';
      # Same default for apps. Pure-bash glob loop: writeShellApplication's
      # checkPhase runs shellcheck, which rejects `ls … | head -1` (SC2012).
      objcopyExport = toolchain: ''
        _r9_objcopy=""
        for _r9_c in ${toolchain}/lib/rustlib/*/bin/llvm-objcopy; do
          if [ -e "$_r9_c" ]; then _r9_objcopy="$_r9_c"; break; fi
        done
        export OBJCOPY="''${OBJCOPY:-$_r9_objcopy}"
      '';
    in
    {
      # D-11 (05-10): the ONLY artifact Harvey's flake input consumes — a
      # derivation that builds the shell-capable aarch64 kernel ELF
      # (`cargo xtask build --arch aarch64 --config virtshell`, debug profile:
      # the gate default the apps and boot-test harness use) and installs it
      # plus the committed aarch64/lib/virt-gicv2.dtb.  The toolchain comes
      # from rust-toolchain.toml via toolchainFor (D-09's single-source rule —
      # never a restated pin); cargo deps are vendored from the committed
      # Cargo.lock via importCargoLock so the sandboxed build never touches
      # the network (T-05-19).  Harvey bakes its own initrd and launches qemu
      # itself — it consumes these as store paths, never by running xtask.
      packages = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          toolchain = toolchainFor system;
          # `-Z build-std=core,alloc` resolves the rust sysroot graph from
          # the toolchain's rust-src component; that graph has its own
          # crates.io members (rustc-literal-escaper et al., pinned by
          # rust-src's library Cargo.lock), so the vendored set must cover
          # BOTH lockfiles or the sandboxed sysroot build dies offline
          # (the two sets are disjoint crate-name dirs — symlinkJoin merges).
          # The name is load-bearing: importCargoLock's outputs ship a
          # .cargo/config.toml whose vendored-sources directory is the
          # RELATIVE "cargo-vendor-dir", and cargoSetupHook prefers that
          # shipped config — the joined output must keep the name so the
          # hook's copy lands where the config points.
          cargoDeps = pkgs.symlinkJoin {
            name = "cargo-vendor-dir";
            paths = [
              (pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; })
              (pkgs.rustPlatform.importCargoLock {
                lockFile = "${toolchain}/lib/rustlib/src/rust/library/Cargo.lock";
              })
            ];
          };
        in
        {
          kernel-virt = pkgs.stdenv.mkDerivation {
            pname = "r9-kernel-virt";
            version = self.shortRev or "wip";
            src = ./.;
            nativeBuildInputs = [ toolchain pkgs.rustPlatform.cargoSetupHook ];
            inherit cargoDeps;
            buildPhase = ''
              runHook preBuild
              ${objcopyExport toolchain}
              cargo xtask build --arch aarch64 --config virtshell
              runHook postBuild
            '';
            installPhase = ''
              runHook preInstall
              mkdir -p "$out/aarch64/lib"
              # The store-path contract Harvey's compose app (05-11) reads:
              # the built kernel ELF, and the committed DTBs at their
              # repo-relative paths (gicv2 = the TCG reference shape;
              # gicv3 = the hvf/TCG gic-version=3 shape — the kernel
              # dispatches on the DTB's compatible string, so one ELF
              # serves both).
              cp target/aarch64-unknown-none-elf/debug/aarch64 "$out/kernel-virt.elf"
              cp aarch64/lib/virt-gicv2.dtb "$out/aarch64/lib/virt-gicv2.dtb"
              cp aarch64/lib/virt-gicv3.dtb "$out/aarch64/lib/virt-gicv3.dtb"
              runHook postInstall
            '';
          };
        });

      devShells = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          toolchain = toolchainFor system;
        in
        {
          default = pkgs.mkShellNoCC {
            packages = [ toolchain pkgs.qemu pkgs.dtc ];
            shellHook = ''
              ${objcopyHook toolchain}
              echo "r9 devshell: $(rustc --version)"
              echo "$(qemu-system-aarch64 --version | head -1)"
              echo "apps: nix run .#build | nix run .#boot-virt | nix run .#default"
            '';
          };
        });

      apps = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          toolchain = toolchainFor system;
          # writeShellApplication embeds runtimeInputs on PATH, so the app
          # runs bare (no devshell) per D-08's zero-host-setup intent.
          r9-build = pkgs.writeShellApplication {
            name = "r9-build";
            runtimeInputs = [ toolchain pkgs.qemu ];
            text = ''
              ${objcopyExport toolchain}
              exec cargo xtask build --arch aarch64 "$@"
            '';
          };
          # D-11 acceptance harness: boot virt on a serial socket and assert
          # banner -> >=3 ticks -> typed-char echo (SC#3/SC#4), writing the
          # captured log to target/boot-test-virt.log.
          r9-boot-virt = pkgs.writeShellApplication {
            name = "r9-boot-virt";
            runtimeInputs = [ toolchain pkgs.qemu ];
            text = ''
              ${objcopyExport toolchain}
              exec cargo xtask boot-test --arch aarch64 --config virt "$@"
            '';
          };
        in
        {
          build = { type = "app"; program = "${r9-build}/bin/r9-build"; };
          boot-virt = { type = "app"; program = "${r9-boot-virt}/bin/r9-boot-virt"; };
          default = { type = "app"; program = "${r9-build}/bin/r9-build"; };
        });
    };
}
