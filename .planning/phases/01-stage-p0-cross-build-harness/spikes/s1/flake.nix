{
  description = "S1 spike: Harvey's namespace build venue inside a nix sandbox";

  # nixpkgs rev is r9's root flake.lock rev (read only there); Harvey is the
  # pushed legacy-final commit (D-23) — flake = false, a source tree only.
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/1c89083e451e891152365b799595cf7790a43325";
    harvey = {
      url = "github:requaos/harvey/bd7a6982a3e4f465f8ed3da5e0c7290521514ff0";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      harvey,
    }:
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
        {
          # Rung 1: the venue itself. Writes under /sys, runs plan9port rc,
          # runs plan9port mk on a two-line mkfile created on the private
          # tmpfs, and copies the transcript to $out/rung1.txt.
          s1-nsprobe = import ./venue.nix {
            inherit pkgs;
            harveySrc = harvey;
            name = "s1-nsprobe";
            transcriptFile = "rung1.txt";
            mode = "unshare";
            guestScript = ''
              set -e
              echo "S1-RUNG1-GUEST-BEGIN uid=$(id -u) cwd=$(pwd)"
              echo "S1-RUNG1-TOOLCHAIN rc=$(command -v rc) mk=$(command -v mk)"

              echo s1-rung1 > /sys/.s1-rung1-probe
              [ "$(cat /sys/.s1-rung1-probe)" = s1-rung1 ] || { echo "S1-RUNG1-FAIL sys-write"; exit 1; }
              rm -f /sys/.s1-rung1-probe
              echo "S1-RUNG1-SYS-OK"

              /bin/rc -c 'echo S1-RUNG1-RC-OK'

              mkdir -p /tmp/mkprobe
              cd /tmp/mkprobe
              printf 'all:V:\n\techo S1-RUNG1-MK-OK\n' > mkfile
              mk
              echo "S1-RUNG1-GUEST-DONE"
            '';
          };

          # Rung 1 fallback (01-RESEARCH.md Pitfall 2 (a)): the same guest
          # script through proot's ptrace-faked chroot, which needs no
          # CapEff and no writable uid_map.
          s1-nsprobe-proot = import ./venue.nix {
            inherit pkgs;
            harveySrc = harvey;
            name = "s1-nsprobe-proot";
            transcriptFile = "rung1-proot.txt";
            mode = "proot";
            guestScript = ''
              set -e
              echo "S1-RUNG1-GUEST-BEGIN uid=$(id -u) cwd=$(pwd)"
              echo "S1-RUNG1-TOOLCHAIN rc=$(command -v rc) mk=$(command -v mk)"

              echo s1-rung1 > /sys/.s1-rung1-probe
              [ "$(cat /sys/.s1-rung1-probe)" = s1-rung1 ] || { echo "S1-RUNG1-FAIL sys-write"; exit 1; }
              rm -f /sys/.s1-rung1-probe
              echo "S1-RUNG1-SYS-OK"

              /bin/rc -c 'echo S1-RUNG1-RC-OK'

              mkdir -p /tmp/mkprobe
              cd /tmp/mkprobe
              printf 'all:V:\n\techo S1-RUNG1-MK-OK\n' > mkfile
              mk
              echo "S1-RUNG1-GUEST-DONE"
            '';
          };
        }
      );
    };
}
