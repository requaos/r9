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
              echo "apps: nix run .#build | nix run .#default"
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
        in
        {
          build = { type = "app"; program = "${r9-build}/bin/r9-build"; };
          default = { type = "app"; program = "${r9-build}/bin/r9-build"; };
        });
    };
}
