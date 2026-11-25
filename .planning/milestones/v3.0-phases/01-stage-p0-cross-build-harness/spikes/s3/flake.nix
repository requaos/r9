{
  description = "S3 scratch flake — the headless rio gate over Harvey bd7a698's own locked inputs";

  # Harvey arrives as a NORMAL flake at the phase pin (D-23): its own
  # flake.lock supplies r9 116bcc2 (the packages this gate consumes) and
  # the vendored drawterm package (the X11 backend).  nixpkgs follows
  # Harvey's nixpkgs so the toolchain matches the Harvey-hosted gate.
  inputs = {
    harvey.url = "github:requaos/harvey/bd7a6982a3e4f465f8ed3da5e0c7290521514ff0";
    nixpkgs.follows = "harvey/nixpkgs";
  };

  outputs = { self, harvey, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      harveyPkgs = harvey.packages.${system};
      # Harvey's locked r9 pin, exactly as Harvey's own composed gates
      # consume it.  Referenced by the packages attrs below, NOT by the
      # app wrapper: r9 116bcc2's kernel derivation reads rust-src's
      # Cargo.lock through import-from-derivation, which cannot be built
      # on aarch64-darwin.  Keeping that reference out of the app's
      # build inputs lets `nix eval ...#apps.x86_64-linux.s3-rio-gate.program`
      # succeed on the Mac (the plan's static check); rio-gate.sh resolves
      # these same locked packages at run time on the owner's Linux host.
      r9Pkgs = harvey.inputs.r9.packages.${system};

      # Mirrors Harvey's mkScriptApp (flake.nix:26-38): writeShellApplication
      # with the same PLAN9/PATH header, so the rewired script sees the
      # identical host-tool environment the Harvey-hosted gate sees.  The
      # runtimeInputs are the Harvey gate's list (plan9port bash qemu git
      # expect iproute2 procps stdenv.cc + the Linux-only util-linux
      # gnused mount binutils) plus xvfb: Xvfb is the S3 addition
      # (Pitfall 6 / F8).
      s3App = pkgs.writeShellApplication {
        name = "s3-rio-gate";
        runtimeInputs = with pkgs; [
          plan9port bash qemu git expect iproute2 procps stdenv.cc
          util-linux gnused mount binutils xvfb
        ];
        text = ''
          set -euo pipefail
          export PLAN9="${pkgs.plan9port}/plan9"
          export PATH="$PATH:$PLAN9/bin"
          export DRAWTERM_BIN="${harveyPkgs.drawterm}/bin/drawterm"
          export S3_FLAKE="${self}"
          export S3_EXPECT="${./rio-gate.expect}"
          ${builtins.readFile ./rio-gate.sh}
        '';
      };
    in
    {
      apps.x86_64-linux.s3-rio-gate = {
        type = "app";
        program = "${s3App}/bin/s3-rio-gate";
      };

      # Resolved at run time by rio-gate.sh through `nix build
      # path:$S3_FLAKE#<attr>`; every input is locked by flake.lock.
      packages.x86_64-linux.kernel-x64 = r9Pkgs.kernel-x64;
      packages.x86_64-linux.iso-builder = r9Pkgs.iso-builder;
      packages.x86_64-linux.netdev-args = r9Pkgs.netdev-args;
    };
}
