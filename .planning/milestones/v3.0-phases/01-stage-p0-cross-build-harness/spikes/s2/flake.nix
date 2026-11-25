{
  description = "S2 spike: Harvey's 23 case-colliding paths through the Mac's Linux builder";

  # Same pins as spikes/s1/flake.nix (D-23): r9's nixpkgs rev and the
  # pushed legacy-final Harvey commit, flake = false (a source tree only).
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

      # One casecheck derivation per source mode. The source is either the
      # flake input (fetched at eval time on the Mac into the case-hacked
      # APFS store, then shipped to the builder as a NAR) or a fixed-output
      # fetch performed by the builder itself.
      mkCasecheck =
        pkgs: name: src:
        pkgs.stdenvNoCC.mkDerivation {
          pname = name;
          version = "0";
          inherit src;
          dontUnpack = true;
          nativeBuildInputs = [
            pkgs.coreutils
            pkgs.findutils
            pkgs.gnugrep
          ];
          buildCommand = ''
            set -euo pipefail

            srcRoot="${src}"
            collisions="${./collisions.txt}"
            report="$out/report.txt"

            mkdir -p "$out/collide"
            : > "$report"

            fail=0
            while read -r expected path; do
              [ -n "''${expected:-}" ] || continue
              f="$srcRoot/$path"
              if [ ! -f "$f" ]; then
                printf 'BAD %s MISSING %s\n' "$expected" "$path" >> "$report"
                fail=1
                continue
              fi
              size=$(wc -c < "$f" | tr -d ' ')
              got=$( { printf 'blob %s' "$size"; printf '\0'; cat "$f"; } | sha1sum | cut -d' ' -f1 )
              if [ "$got" = "$expected" ]; then
                printf 'OK %s %s\n' "$got" "$path" >> "$report"
              else
                printf 'BAD %s %s %s\n' "$expected" "$got" "$path" >> "$report"
                fail=1
              fi
              mkdir -p "$out/collide/$(dirname "$path")"
              cp "$f" "$out/collide/$path"
            done < "$collisions"

            report_lines=$(grep -c . "$report" || true)
            if [ "$report_lines" != "23" ]; then
              printf 'BAD report-lines %s expected 23\n' "$report_lines" >> "$report"
              fail=1
            fi

            # A builder-side source must never carry the Mac store's
            # case-hack names; their presence means the NAR did not
            # round-trip the real names.
            hacked=$(find "$srcRoot" -name '*~nix~case~hack~*' | wc -l | tr -d ' ')
            if [ "$hacked" = "0" ]; then
              printf 'OK casehack-names 0 in source\n' >> "$report"
            else
              printf 'BAD casehack-names %s in source\n' "$hacked" >> "$report"
              fail=1
            fi

            kill_lc=$(ls "$srcRoot/rc/bin" | grep -cx kill || true)
            kill_uc=$(ls "$srcRoot/rc/bin" | grep -cx Kill || true)
            if [ "$kill_lc" = "1" ] && [ "$kill_uc" = "1" ]; then
              printf 'OK rc/bin kill and Kill present\n' >> "$report"
            else
              printf 'BAD rc/bin kill=%s Kill=%s\n' "$kill_lc" "$kill_uc" >> "$report"
              fail=1
            fi

            if [ "$fail" -eq 0 ]; then
              printf 'S2-CASECHECK-ALL-OK\n' >> "$report"
            else
              printf 'S2-CASECHECK-FAILED\n' >> "$report"
            fi
            cat "$report"
            [ "$fail" -eq 0 ]
          '';
        };

      # A9 (01-RESEARCH.md Pattern 1): a target on the Mac's builder system
      # (aarch64-linux) whose store path the aarch64-darwin devshell exports
      # with its string context discarded, so `nix develop` never builds it.
      # The name is unique to this spike so it cannot already be in a store.
      linuxArm = nixpkgs.legacyPackages.aarch64-linux;
      lazyTarget = linuxArm.runCommand "s2-lazy-target-a9-0" { } ''
        mkdir -p "$out"
        printf 'S2-LAZY-TARGET a9\n' > "$out/target.txt"
      '';
      lazyTargetPath = builtins.unsafeDiscardStringContext lazyTarget.outPath;
    in
    {
      packages = forAll (
        system: pkgs:
        rec {
          # Source fetched at eval time on the Mac, into the case-hacked APFS
          # store, then shipped to the builder as a NAR.
          s2-casecheck-input = mkCasecheck pkgs "s2-casecheck-input" harvey;

          # Source fetched by the builder itself (fixed-output derivation);
          # the SRI hash is the flake lock's Harvey narHash.
          s2-casecheck-fod = mkCasecheck pkgs "s2-casecheck-fod" (
            pkgs.fetchFromGitHub {
              owner = "requaos";
              repo = "harvey";
              rev = "bd7a6982a3e4f465f8ed3da5e0c7290521514ff0";
              hash = "sha256-N6VjnVI69pcxSCpQ9BUxrlpTqnp9LgjKfGcpO3F6Ma0=";
            }
          );
        }
        // nixpkgs.lib.optionalAttrs (system == "aarch64-linux") {
          s2-lazy-target = lazyTarget;
        }
      );

      # The lazy devshell D-05/D-21 need: S2_LAZY_PATH is a plain string (no
      # context), so entering the shell does not realise the Linux target.
      devShells.aarch64-darwin.lazy = nixpkgs.legacyPackages.aarch64-darwin.mkShell {
        shellHook = ''
          export S2_LAZY_PATH="${lazyTargetPath}"
        '';
      };
    };
}
