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
          # X64-05 (13-06): the x86_64 sibling of kernel-virt, built the
          # same way (same cargoDeps symlinkJoin, same toolchain/
          # cargoSetupHook inputs) — the Phase-14 compose contract:
          # r9.packages.<system>.kernel-x64.  The buildPhase invokes the
          # SAME `cargo xtask build --arch x86-64 --config q35` the
          # KVM-native gates prove — the shipped artifact is the gated
          # artifact.
          kernel-x64 = pkgs.stdenv.mkDerivation {
            pname = "r9-kernel-x64";
            version = self.shortRev or "wip";
            src = ./.;
            nativeBuildInputs = [ toolchain pkgs.rustPlatform.cargoSetupHook ];
            inherit cargoDeps;
            buildPhase = ''
              runHook preBuild
              ${objcopyExport toolchain}
              cargo xtask build --arch x86-64 --config q35
              runHook postBuild
            '';
            installPhase = ''
              runHook preInstall
              mkdir -p "$out"
              cp target/x86_64-unknown-none-elf/debug/x86_64 "$out/kernel-x64.elf"
              runHook postInstall
            '';
          };
          # X64-05 (13-06): the reproducible boot ISO — the flake owns
          # the DISTRIBUTABLE ISO (the OQ-3 boundary decision); the
          # xtask gate keeps building its own dev copy each invocation
          # so the gate's evidence is always fresh against the tree it
          # just built.  Byte-reproducibility mechanics (measured on
          # grub 2.14 + xorriso 1.5.8.pl02): grub-mkrescue injects a
          # CURRENT-TIME --modification-date into its xorriso argv and
          # has no SOURCE_DATE_EPOCH support, and the staged/grub file
          # mtimes ride through into Rock Ridge TF records — the
          # `--xorriso` wrapper below pins the modification date,
          # overrides every file date (--set_all_file_dates) and pins
          # the volume id.  BIOS-only (nixpkgs grub2 carries only the
          # i386-pc platform; no EFI image, so no mtools).  The
          # outputHash is wired ONLY after the double-build
          # byte-identity check passes (nix build --rebuild): a hash
          # wired to non-reproducible bytes would red on every rebuild.
          boot-iso = pkgs.stdenv.mkDerivation {
            pname = "r9-boot-iso";
            version = self.shortRev or "wip";
            src = ./.;
            nativeBuildInputs = [ pkgs.grub2 pkgs.xorriso ];
            buildPhase = ''
              runHook preBuild
              export SOURCE_DATE_EPOCH=1735689600
              export R9_ISO_DATE=2025010100000000
              export R9_ISO_VOLID=R9BOOT
              xorrisoPinned="$NIX_BUILD_TOP/xorriso-pinned"
              cat > "$xorrisoPinned" <<'WRAP'
              #!/bin/sh
              a1=$1; shift
              a2=$1; shift
              n=$#
              i=0
              while [ "$i" -lt "$n" ]; do
                x=$1
                shift
                case "$x" in
                  --modification-date=*) x="--modification-date=$R9_ISO_DATE" ;;
                esac
                set -- "$@" "$x"
                i=$((i+1))
              done
              exec xorriso "$a1" "$a2" \
                "--modification-date=$R9_ISO_DATE" \
                "--set_all_file_dates" "$R9_ISO_DATE" \
                "-volid" "$R9_ISO_VOLID" \
                "$@"
WRAP
              chmod +x "$xorrisoPinned"
              # The grub module tree rides the ISO verbatim, and
              # nixpkgs' modinfo.sh carries a store-path bash shebang —
              # a fixed-output derivation must not reference store
              # paths, so graft from a copy with the shebang scrubbed.
              moddir="$NIX_BUILD_TOP/grub-i386-pc"
              cp -r "${pkgs.grub2}/lib/grub/i386-pc" "$moddir"
              chmod -R u+w "$moddir"
              sed -i "s|^#![^ ]*bin/bash|#!/bin/sh|" "$moddir/modinfo.sh"
              stage="$NIX_BUILD_TOP/stage"
              mkdir -p "$stage/boot/grub"
              # The staged kernel is a FULLY stripped copy — strip-all,
              # not the xtask dev-copy's strip-debug: rustc emits
              # per-invocation-random `.llvm.<u64>` suffixes on ~200
              # anonymous-namespace symbols (measured: two fresh
              # sandbox builds from identical source produce kernels
              # differing ONLY there), so any symtab-carrying stage
              # breaks ISO byte-identity.  GRUB's mb2 loader reads only
              # PT_LOAD segments (all intact under strip-all; the mb2
              # header rides .text.boot); debug weight is pure
              # BIOS-disk load time besides.  The sed then zeroes the
              # store-hash runs inside build-std's panic-location
              # strings (/nix/store/<hash>-rust-default/... rides the
              # kernel's .rodata): a fixed-output derivation must not
              # reference store paths, and nix's scanner keys on the
              # hash, so a prefix remap alone is not enough — the
              # length-preserving zero-zap keeps the strings readable
              # while breaking both the full-path and the hash match.
              objcopy --strip-all "${kernel-x64}/kernel-x64.elf" "$stage/boot/kernel"
              sed -E "s|/nix/store/[a-z0-9]{32}-|/nix/store/00000000000000000000000000000000-|g" -i "$stage/boot/kernel"
              cp x86_64/lib/grub.cfg "$stage/boot/grub/grub.cfg"
              grub-mkrescue -d "$moddir" -o "$NIX_BUILD_TOP/boot.iso" "$stage" --xorriso "$xorrisoPinned"
              runHook postBuild
            '';
            installPhase = ''
              runHook preInstall
              mkdir -p "$out"
              cp "$NIX_BUILD_TOP/boot.iso" "$out/boot.iso"
              runHook postInstall
            '';
            # Fixed-output, wired ONLY after the double-build
            # byte-identity check passed: two INDEPENDENT derivations
            # (fresh kernel builds under both) produced the identical
            # ISO sha256
            # 87a96c23a421ce886411ef103723b36c025fbb46f7e94ced726685369ca82003
            # with zero live store-hash references in the image.
            # `nix build .#boot-iso --rebuild` re-runs the derivation
            # and verifies this hash against freshly produced bytes —
            # the reproducibility proof on every rebuild.
            outputHashAlgo = "sha256";
            outputHashMode = "recursive";
            outputHash = "sha256-dCSPPVirOX7LkVLyVHy6HTPZDAZBbTlICiJr0xBXYXo=";
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
        }
        # The x86_64 deliverables: the kernel package on every linux
        # host (the rustc target spec is cross-platform), the BIOS ISO
        # only where grub2 ships the i386-pc modules it embeds — the
        # q35 channel's loaders and gates are x86_64-native (X64-05).
        // nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          inherit kernel-x64;
        }
        // nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
          inherit boot-iso;
        });

      devShells = forAllSystems (system:
        let
          pkgs = pkgsFor system;
          toolchain = toolchainFor system;
        in
        {
          default = pkgs.mkShellNoCC {
            # grub2 + xorriso (13-01): grub-mkrescue for the x86_64 mb2
            # boot channel — permanently in the devshell so `nix develop`
            # builds the q35 ISO without a wrapping `nix shell`.
            packages = [ toolchain pkgs.qemu pkgs.dtc pkgs.grub2 pkgs.xorriso ];
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
