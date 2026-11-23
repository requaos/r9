# nix/harvey/mkpaqfs.nix — Harvey's in-tree mkpaqfs as a host binary.
#
# The paqfs byte format is Harvey's; r9 does not reimplement it (01-RESEARCH
# "Don't Hand-Roll").  This is `build/aarch64/mkpaqfs-host.sh` as a
# derivation: plan9port's `9c` compiles `sys/src/cmd/paqfs/mkpaqfs.c` and
# `9l` links it against plan9port's flate/sec/mp/bio/lib9 — NOT the 9front
# donor's copy, and NOT a Harvey-built Plan 9 tool.  It is a per-HOST
# system tool (darwin and linux), outside every Harvey tree, so it builds
# on all three of r9's systems; the devshell exports its path as
# R9_MKPAQFS (01-08's explicit input, no derived default).
#
# The source comes from the flake input path: this derivation builds on the
# host itself (never a Linux-tree derivation), so the Mac-fetched source is
# the right input and the case-collision question does not arise for the
# paqfs sources.
{
  pkgs,
  harveySrc,
}:

let
  plan9 = "${pkgs.plan9port}/plan9";
in
pkgs.stdenvNoCC.mkDerivation {
  pname = "harvey-mkpaqfs";
  version = "0";

  dontUnpack = true;
  # A plan9port-linked host binary; the fixup strip hook has no business in
  # it (and its bytes are not r9's contract).
  dontStrip = true;

  nativeBuildInputs = [
    pkgs.bash
    pkgs.coreutils
    pkgs.plan9port
    # plan9port's 9l autolib calls bare `nm` on host ELF objects; without
    # binutils on PATH the link still succeeds but prints "nm: command not
    # found" twice (measured) and cannot resolve autolibs.
    pkgs.binutils
  ];

  buildCommand = ''
    set -euo pipefail

    export TMPDIR="''${TMPDIR:-$NIX_BUILD_TOP}"
    # plan9port's 9c/9l source $PLAN9/config for their compiler and tool
    # variables; without PLAN9 set, 9c's warning filter runs with an empty
    # `$egrep`/`$uniq` and dies ("-v: command not found").
    export PLAN9="${plan9}"
    HARVEY_ROOT="${harveySrc}"
    mkdir -p "$out/bin"
    SCRATCH=$(mktemp -d "$TMPDIR/mkpaqfs.XXXXXX")
    trap 'rm -rf "$SCRATCH"' EXIT

    echo "mkpaqfs: [9c] compile sys/src/cmd/paqfs/mkpaqfs.c"
    ( cd "$SCRATCH" && "${plan9}/bin/9c" -O0 -I"$HARVEY_ROOT/sys/src/cmd/paqfs" \
        -c "$HARVEY_ROOT/sys/src/cmd/paqfs/mkpaqfs.c" )

    echo "mkpaqfs: [9l] link bin/mkpaqfs (-lflate -lsec -lmp -lbio -l9)"
    ( cd "$SCRATCH" && "${plan9}/bin/9l" -o "$out/bin/mkpaqfs" mkpaqfs.o \
        -lflate -lsec -lmp -lbio -l9 )

    # The binary must run and bake a paq (a link that produced something
    # unrunnable fails here, not at the first image assembly).  An empty
    # root is the cheap probe; never run it with no arguments — that
    # archives "." onto stdout.
    mkdir -p "$TMPDIR/probe-root"
    "$out/bin/mkpaqfs" -o "$TMPDIR/probe.paq" "$TMPDIR/probe-root"
    [ -s "$TMPDIR/probe.paq" ] || {
      echo "mkpaqfs: FAIL — the built binary produced no paq for an empty root" >&2
      exit 1
    }
    echo "mkpaqfs: bake probe ok ($(wc -c < "$TMPDIR/probe.paq" | tr -d ' ') bytes)"

    echo "mkpaqfs: built $out/bin/mkpaqfs"
  '';
}
