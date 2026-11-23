# nix/harvey/build-env.nix — Harvey's host toolchain and the proot tree venue.
#
# This is the ONE build environment every Harvey tree derivation rides
# (HARN-07, D-03, D-24, D-25):
#
#   * `hostbin` is a port of Harvey's `nix/setup.sh` + `nix/toolchain.sh`
#     (the plan9port `9c`-as-pcc bootstrap, the vendored 6a/6c/6l,
#     8a/8c/8l, 7a/7c/7l, cpp and aux toolchains, and the vendored 9cc
#     `iar` installed as `ar`).  It carries S1's four measured sandbox
#     adjustments — the store-bash shebang on the 9l shim, `pkgs.ed` for
#     mkcname, plan9port LAST on PATH (its 9p `mount` must not shadow
#     util-linux mount), and GNU sed shadowing for 7l/BUILD's malformed
#     `_SYSRM` line at the pin.
#
#   * `mkTreeDerivation` is the S1/S4 venue, in **proot mode only**
#     (D-25: the `unshare` branch is deliberately absent — nested
#     user-namespace creation was refused inside the nix sandbox on both
#     measured builders, S1-FINDINGS).  Every tree derivation:
#       - copies the Harvey subdirectories it reads into a writable
#         `$TMPDIR/work` (the store path is never written; there is no
#         writable `/bin` and no sudo anywhere);
#       - prints the SANDBOX-EVIDENCE block and REFUSES to run when
#         `/usr/bin/env` exists, so a non-sandboxed build can never be
#         recorded as a pass (T-01-46);
#       - runs the guest under `proot -0`, with S4's two venue ports
#         applied to every guest: `/rc/bin` on PATH (membername and the
#         boot mkfile's rc scripts) and an explicit
#         `INCLUDE="/<objtype>/include /sys/include"` (the latent 6c
#         heap-layout failure under proot, S4-FINDINGS);
#       - fails the derivation when any build step fails: there is no
#         report-and-continue mode.
#
# The install script runs inside the same guest (after `guestScript`), and
# writes the final tree layout under `/out` (bound to `$work/out`).  The
# derivation copies that to `$out` and then applies the tree assertions
# that must see the final store layout: no case-fold duplicate name and no
# `~nix~case~hack~` name (HARN-02, T-01-44).
{
  pkgs,
  harveySrc,
}:

let
  plan9 = "${pkgs.plan9port}/plan9";

  hostbin = pkgs.stdenv.mkDerivation {
    pname = "harvey-hostbin";
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
      echo "HARVEY-HOSTBIN-SETUP-TOOLS=$(ls "$HARVEY_LINUX_BIN" | tr '\n' ' ')"

      # ---- nix/toolchain.sh ----
      _WRAP_PATH="$PATH"
      # Harvey's flake wrapper puts plan9port LAST ("plan9 bin LAST per
      # BUILD_ON_UNIX — plan9port ships a 9p `mount` which must not shadow
      # util-linux mount"); hostbin goes first, as toolchain.sh does. The
      # sandbox has no /usr/bin, so `ed` (mkcname's generator) is explicit.
      export PATH="$HARVEY_LINUX_BIN:$PATH:$PLAN9/bin"

      SHIMDIR=$(mktemp -d "$TMPDIR/9l-shim.XXXXXX")
      cat > "$SHIMDIR/9l" <<'HARVEY_9L_SHIM'
      #!${pkgs.bash}/bin/bash
      args=()
      for a in "$@"; do
        [ "$a" = "-static" ] || args+=("$a")
      done
      exec "''${PLAN9:?}/bin/9l" "''${args[@]}"
      HARVEY_9L_SHIM
      chmod +x "$SHIMDIR/9l"

      _HOSTNM=$(PATH="$_WRAP_PATH" command -v nm 2>/dev/null || true)
      if [ -n "$_HOSTNM" ]; then
        ln -sf "$_HOSTNM" "$SHIMDIR/nm"
      fi
      # 7l/BUILD munges PATH with a multi-command `sed` script and the pinned
      # plan9port sed answers `sed: Too much text` there (measured: the 7l
      # bootstrap dies, cnam.c is never generated).  Harvey's own build.sh
      # exports GNU_SED for exactly this class of call; the BUILD scripts have
      # no such variable, so shadow `sed` with GNU sed for the bootstrap.
      ln -sf "${pkgs.gnused}/bin/sed" "$SHIMDIR/sed"
      export PATH="$SHIMDIR:$PATH"
      echo "HARVEY-HOSTBIN-SHIM nm=$_HOSTNM sed=${pkgs.gnused}/bin/sed"

      ( cd "$HARVEY/sys/src" && sh BUILDTOOLCHAIN )
      ( cd "$HARVEY/sys/src/cmd" && bash BUILD )
      ( cd "$HARVEY/sys/src/cmd/unix/9cc" && PATH="$_WRAP_PATH" sh BUILD )

      echo "HARVEY-HOSTBIN-TOOLS=$(ls "$HARVEY_LINUX_BIN" | tr '\n' ' ')"
      for t in 6a 6c 6l 7a 7c 7l 8a 8c 8l cpp nm ar pcc; do
        [ -x "$HARVEY_LINUX_BIN/$t" ] || {
          echo "HARVEY-HOSTBIN-FAIL missing $t" >&2
          exit 1
        }
      done
      echo "HARVEY-HOSTBIN-OK"
    '';
  };

  # The proot venue (D-25).  `arch` is Harvey's objtype for the guest
  # ("arm64" or "amd64"); `guestScript` builds, `installScript` lays the
  # final tree out under `/out`, and both run in ONE shell inside the venue
  # so build variables (the staged source list, the output staging dir)
  # carry over.
  mkTreeDerivation =
    {
      name,
      arch,
      guestScript,
      installScript ? "",
      # Extra Harvey top-level directories the guest needs beside the
      # arch's own (01-14's libmach includes read e.g. /386/include).
      extraDirs ? [ ],
    }:
    pkgs.stdenvNoCC.mkDerivation {
      pname = name;
      version = "0";

      dontUnpack = true;
      # The tree holds Plan 9 a.out binaries and archives; the fixup
      # strip hook has no business in them.
      dontStrip = true;
      dontPatchELF = true;
      dontPatchShebangs = true;

      nativeBuildInputs = [
        pkgs.bash
        pkgs.coreutils
        pkgs.util-linux
        pkgs.plan9port
        pkgs.gnused
        pkgs.gawk
        pkgs.gnugrep
        pkgs.diffutils
        pkgs.findutils
        pkgs.proot
        hostbin
      ];

      buildCommand = ''
        set -euo pipefail

        export TMPDIR="''${TMPDIR:-$NIX_BUILD_TOP}"
        work="$TMPDIR/work"
        mkdir -p "$work" "$work/out"

        # Harvey's build writes into the checkout (build/mkdirs, .build, the
        # arch dirs), so it runs on a writable copy, never on the store path.
        for d in sys ${arch} rc lib build ${builtins.concatStringsSep " " extraDirs}; do
          cp -r "${harveySrc}/$d" "$work/$d"
        done
        chmod -R u+w "$work"

        # Harvey's own tree layout step (build/mkdirs is relative to the tree
        # root and only mkdir -p's; run it on the copy, before the venue).
        ( cd "$work" && bash build/mkdirs )

        # ---- pre-venue evidence: are we actually inside a nix sandbox? ----
        echo "SANDBOX-EVIDENCE id-u=$(id -u)"
        echo "SANDBOX-EVIDENCE uid-map=$(tr '\n' ';' < /proc/self/uid_map)"
        echo "SANDBOX-EVIDENCE user-ns=$(readlink /proc/self/ns/user)"
        echo "SANDBOX-EVIDENCE uname=$(uname -a)"
        if [ -e /usr/bin/env ]; then
          echo "SANDBOX-EVIDENCE usr-bin-env=PRESENT"
          echo "HARVEY-VENUE-FAIL /usr/bin/env exists: this build is not in a nix sandbox" >&2
          exit 1
        fi
        echo "SANDBOX-EVIDENCE usr-bin-env=absent"

        mkdir -p "$TMPDIR/guest"
        cat > "$TMPDIR/guest/guest.sh" <<'HARVEY_GUEST_EOF'
        ${guestScript}
        ${installScript}
        HARVEY_GUEST_EOF

        # ---- the proot venue: ptrace-faked chroot, no caps, no uid_map ----
        cat > "$TMPDIR/guest/proot.sh" <<'HARVEY_PROOT_EOF'
        #!/usr/bin/env bash
        set -euo pipefail

        NSX=$(mktemp -d "$TMPDIR/prootroot.XXXXXX")
        PROOT_TMP=$(mktemp -d "$TMPDIR/proot-tmp.XXXXXX")
        echo "HARVEY-VENUE-PROOT-ROOT=$NSX"

        mkdir -p "$NSX"/dev "$NSX"/nix/store "$NSX"/bin "$NSX"/out

        ln -s "$BASH_BIN" "$NSX/bin/sh"
        ln -s "$BASH_BIN" "$NSX/bin/bash"
        ln -s "$PLAN9_BIN/rc" "$NSX/bin/rc"
        ln -s "$PLAN9_BIN/mk" "$NSX/bin/mk"
        cp "$GUEST" "$NSX/guest.sh"

        set -- -0 -r "$NSX" -w /
        set -- "$@" -b "$WORK/sys:/sys" -b "$WORK/${arch}:/${arch}"
        set -- "$@" -b "$WORK/rc:/rc" -b "$WORK/lib:/lib"
        set -- "$@" -b "$WORK/build:/build" -b "$WORK/out:/out"
        set -- "$@" -b /nix/store:/nix/store -b "$PROOT_TMP:/tmp"
        for d in null zero random urandom; do
          if [ -e "/dev/$d" ]; then
            set -- "$@" -b "/dev/$d:/dev/$d"
          else
            echo "HARVEY-VENUE-NOTE /dev/$d absent in the sandbox"
          fi
        done

        echo "HARVEY-VENUE-ENTER-PROOT"
        exec "$PROOT_BIN" "$@" /bin/sh /guest.sh
        HARVEY_PROOT_EOF

        export WORK="$work"
        export GUEST="$TMPDIR/guest/guest.sh"
        export PLAN9_BIN="${plan9}/bin"
        export PLAN9="${plan9}"
        export BASH_BIN="${pkgs.bash}/bin/bash"
        export PROOT_BIN="${pkgs.proot}/bin/proot"
        export HOSTBIN="${hostbin}/bin"

        # S4's two venue ports, carried by the venue itself (never per step).
        # plan9port stays LAST on PATH (BUILD_ON_UNIX); hostbin first, then
        # the guest-visible /rc/bin.
        export PATH="$HOSTBIN:/rc/bin:$PATH:$PLAN9_BIN"
        export INCLUDE="/${arch}/include /sys/include"
        export objtype="${arch}" cputype=unix FORCERCFORMK=1
        # The mkfiles reach $HARVEY/<path>; the venue root IS the tree.
        export HARVEY=

        set +e
        {
          echo "HARVEY-VENUE-BUILD-START mode=proot arch=${arch}"
          echo "HARVEY-VENUE-PATH=$PATH"
          "$BASH_BIN" "$TMPDIR/guest/proot.sh"
        } 2>&1 | tee "$TMPDIR/transcript.txt"
        rc="''${PIPESTATUS[0]}"
        set -e
        echo "HARVEY-VENUE-RC=$rc"

        if [ "$rc" -ne 0 ]; then
          echo "HARVEY-VENUE-FAIL the guest run exited $rc (last 80 transcript lines follow)" >&2
          tail -n 80 "$TMPDIR/transcript.txt" >&2 || true
          exit "$rc"
        fi

        mkdir -p "$out"
        # NB: hostbin shadows `ls` with plan9port's ls (no -A/-a flags); use
        # find for the emptiness test.
        if [ -n "$(find "$work/out" -mindepth 1 -print -quit 2>/dev/null)" ]; then
          cp -r "$work/out/." "$out/"
        fi

        # ---- the tree-shape assertions (T-01-44): run on the final layout ----
        dupes=$(find "$out" | tr '[:upper:]' '[:lower:]' | sort | uniq -d | wc -l)
        if [ "$dupes" != "0" ]; then
          echo "HARVEY-TREE-FAIL case-fold duplicate names in the tree ($dupes):" >&2
          find "$out" | tr '[:upper:]' '[:lower:]' | sort | uniq -d >&2
          exit 1
        fi
        hacked=$(find "$out" -name '*~nix~case~hack~*' | wc -l)
        if [ "$hacked" != "0" ]; then
          echo "HARVEY-TREE-FAIL $hacked ~nix~case~hack~ names in the tree:" >&2
          find "$out" -name '*~nix~case~hack~*' >&2
          exit 1
        fi
        echo "HARVEY-TREE-CASE-OK (no case-fold duplicates, no case-hack names)"
        echo "HARVEY-TREE-OK"
      '';
    };
in
{
  inherit hostbin mkTreeDerivation;
}
