# hostbin.nix — Harvey's host toolchain bootstrap as a nix derivation.
#
# A port of Harvey's `nix/setup.sh` (populate the host tool bin: plan9port's
# 9c as pcc, the Plan 9 userland tools the mkfiles call, a `size` stub) and
# `nix/toolchain.sh` (build the 6a/6c/6l, 8a/8c/8l, 7a/7c/7l, cpp and aux
# toolchains plus Harvey's own pcc, then the vendored 9cc `iar` installed as
# `ar`). The three measured traps those scripts encode are kept:
#
#   * `9l` is shimmed to drop `-static` (neither nix's glibc toolchain nor
#     modern Apple ld links the static crt0 the 6c/8c BUILDs ask for);
#   * `nm` is shadowed with the wrapper PATH's (binutils) nm while building,
#     because plan9port's 9l autolib calls bare `nm` on HOST ELF objects and
#     Harvey's own nm reads ken-format objects only;
#   * the vendored 9cc build runs on the WRAPPER PATH, never the hostbin
#     one, because once `iar` is installed as `ar` a bare `ar` would write
#     Plan 9-format archives that GNU ld rejects.
#
# Harvey's source is copied to a writable work dir; the store path is never
# written. No sudo, no network, no nix calls.
{
  pkgs,
  harveySrc,
}:

let
  plan9 = "${pkgs.plan9port}/plan9";
in
pkgs.stdenv.mkDerivation {
  pname = "s1-hostbin";
  version = "0";

  dontUnpack = true;

  nativeBuildInputs = [
    pkgs.bash
    pkgs.coreutils
    pkgs.gnused
    pkgs.gawk
    pkgs.binutils
    pkgs.plan9port
    pkgs.bison
    pkgs.flex
    pkgs.ed
  ];

  buildCommand = ''
    set -euo pipefail

    export TMPDIR="''${TMPDIR:-$NIX_BUILD_TOP}"
    work="$TMPDIR/work"
    mkdir -p "$work"
    cp -r "${harveySrc}/sys" "$work/sys"
    chmod -R u+w "$work"

    export HARVEY="$work"
    export PLAN9="${plan9}"
    export HARVEY_LINUX_BIN="$out/bin"
    mkdir -p "$HARVEY_LINUX_BIN"

    # ---- nix/setup.sh ----
    install_tool() {
      rm -f "$HARVEY_LINUX_BIN/$2"
      cp "$1" "$HARVEY_LINUX_BIN/$2"
      chmod u+w "$HARVEY_LINUX_BIN/$2"
    }
    install_tool "$PLAN9/bin/9c" pcc
    for t in sed yacc lex awk date dd rm basename ls; do
      install_tool "$PLAN9/bin/$t" "$t"
    done
    rm -f "$HARVEY_LINUX_BIN/size"
    printf '#!/bin/sh\nexit 0\n' > "$HARVEY_LINUX_BIN/size"
    chmod +x "$HARVEY_LINUX_BIN/size"
    echo "S1-HOSTBIN-SETUP-TOOLS=$(ls "$HARVEY_LINUX_BIN" | tr '\n' ' ')"

    # ---- nix/toolchain.sh ----
    _WRAP_PATH="$PATH"
    # Harvey's flake wrapper puts plan9port LAST ("plan9 bin LAST per
    # BUILD_ON_UNIX — plan9port ships a 9p `mount` which must not shadow
    # util-linux mount"); hostbin goes first, as toolchain.sh does. The
    # sandbox has no /usr/bin, so `ed` (mkcname's generator) is explicit.
    export PATH="$HARVEY_LINUX_BIN:$PATH:$PLAN9/bin"

    SHIMDIR=$(mktemp -d "$TMPDIR/9l-shim.XXXXXX")
    cat > "$SHIMDIR/9l" <<'S1_9L_SHIM'
    #!${pkgs.bash}/bin/bash
    args=()
    for a in "$@"; do
      [ "$a" = "-static" ] || args+=("$a")
    done
    exec "''${PLAN9:?}/bin/9l" "''${args[@]}"
    S1_9L_SHIM
    chmod +x "$SHIMDIR/9l"

    _HOSTNM=$(PATH="$_WRAP_PATH" command -v nm 2>/dev/null || true)
    if [ -n "$_HOSTNM" ]; then
      ln -sf "$_HOSTNM" "$SHIMDIR/nm"
    fi
    # 7l/BUILD munges PATH with a multi-command `sed` script and the pinned
    # plan9port sed answers `sed: Too much text` there (measured: the 7l
    # bootstrap dies, cnam.c is never generated). Harvey's own build.sh
    # exports GNU_SED for exactly this class of call; the BUILD scripts have
    # no such variable, so shadow `sed` with GNU sed for the bootstrap.
    ln -sf "${pkgs.gnused}/bin/sed" "$SHIMDIR/sed"
    export PATH="$SHIMDIR:$PATH"
    echo "S1-HOSTBIN-SHIM nm=$_HOSTNM sed=${pkgs.gnused}/bin/sed"

    ( cd "$HARVEY/sys/src" && sh BUILDTOOLCHAIN )
    ( cd "$HARVEY/sys/src/cmd" && bash BUILD )
    ( cd "$HARVEY/sys/src/cmd/unix/9cc" && PATH="$_WRAP_PATH" sh BUILD )

    echo "S1-HOSTBIN-TOOLS=$(ls "$HARVEY_LINUX_BIN" | tr '\n' ' ')"
    for t in 6a 6c 6l 7a 7c 7l 8a 8c 8l cpp nm ar pcc; do
      [ -x "$HARVEY_LINUX_BIN/$t" ] || {
        echo "S1-HOSTBIN-FAIL missing $t" >&2
        exit 1
      }
    done
    echo "S1-HOSTBIN-OK"
  '';
}
