{
  description = "S4 spike: Harvey's amd64 tier 0 + tier 1 on an aarch64-linux host in the S1 venue";

  # Same pins as s1 (D-23): r9's nixpkgs rev and the pushed legacy-final
  # Harvey commit as a source tree (flake = false).
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/1c89083e451e891152365b799595cf7790a43325";
    harvey = {
      url = "github:requaos/harvey/bd7a6982a3e4f465f8ed3da5e0c7290521514ff0";
      flake = false;
    };
  };

  outputs =
    { self, nixpkgs, harvey }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f system nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAll (
        system: pkgs:
        let
          hostbin = import ./hostbin.nix { inherit pkgs; harveySrc = harvey; };
        in
        {
          # Byte-identical to s1's hostbin derivation (same pname and inputs),
          # so the builder reuses the toolchain S1 already built.
          s4-hostbin = hostbin;

          s4-tier-amd64 = import ./tier.nix {
            inherit pkgs hostbin;
            harveySrc = harvey;
          };
        }
      );
    };
}
