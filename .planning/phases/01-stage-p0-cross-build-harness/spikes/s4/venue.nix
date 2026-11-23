# venue.nix — the S1 sandbox venue.
#
# Runs a guest script inside a chroot-like root built from Harvey's
# `build/x86_64/stage-build.sh` venue (the SAFETY and bind-discipline rules
# there) minus its NixOS assumptions:
#
#   * the scaffold is a PRIVATE TMPFS and is NEVER removed with rm — a
#     recursive delete walking bind mounts deletes THROUGH them into the
#     source tree (Harvey measured exactly that). The tmpfs dies with the
#     namespace and takes the scaffold with it. (unshare mode.)
#   * binds are LEAF binds: each Harvey subdirectory, /nix/store, and the
#     /dev nodes one at a time. A directory bind whose subtree carries other
#     mounts fails inside a user namespace.
#   * /bin is a real directory of symlinks (sh/bash to the store's bash, rc
#     and mk to plan9port) so the mkfiles' `#!/bin/rc` and `/bin/mk` resolve.
#   * Harvey's source is COPIED into a writable work dir first; the store
#     path is never bound writable and never written.
#
# Two venues, same guest script:
#   mode = "unshare": `unshare -Ur -m` + tmpfs scaffold + chroot. Needs the
#     process to be able to write its own uid_map (CAP_SETUID in the parent
#     userns). Inside a nix build sandbox, and on Ubuntu 24.04 at all
#     (AppArmor unprivileged-userns restriction), that is refused; the rung
#     1 log records the exact failure.
#   mode = "proot": `proot -0 -r <root> -b ...` instead of unshare. proot
#     fakes the chroot with ptrace, so it needs no capabilities and no
#     uid_map; it is the fallback of 01-RESEARCH.md Pitfall 2 (a).
#
# SANDBOX-EVIDENCE lines are printed before either venue is entered, and the
# build fails if /usr/bin/env exists: inside a nix build sandbox it does not,
# so its presence means the build is not sandboxed at all and the rung proves
# nothing.
#
# Args:
#   pkgs            nixpkgs instance for the build system (a Linux system)
#   harveySrc       Harvey source tree (flake input, flake = false)
#   name            derivation name
#   guestScript     shell script text run inside the root (as /bin/sh)
#   extraPath       extra nativeBuildInputs (also on PATH inside the root)
#   transcriptFile  name of the transcript in $out (default transcript.txt)
#   mode            "unshare" (default) or "proot"
#
# Output: $out/<transcriptFile> (the whole transcript) plus everything the
# guest script wrote under /out (bound to <work>/out and copied to $out).
{
  pkgs,
  harveySrc,
  name,
  guestScript,
  extraPath ? [ ],
  transcriptFile ? "transcript.txt",
  mode ? "unshare",
}:

let
  plan9 = "${pkgs.plan9port}/plan9";
in
pkgs.stdenvNoCC.mkDerivation {
  pname = name;
  version = "0";

  dontUnpack = true;

  nativeBuildInputs =
    [
      pkgs.bash
      pkgs.coreutils
      pkgs.util-linux
      pkgs.plan9port
      pkgs.gnused
      pkgs.gawk
      pkgs.gnugrep
      pkgs.diffutils
    ]
    ++ pkgs.lib.optional (mode == "proot") pkgs.proot
    ++ extraPath;

  buildCommand = ''
    set -euo pipefail

    export TMPDIR="''${TMPDIR:-$NIX_BUILD_TOP}"
    work="$TMPDIR/work"
    mkdir -p "$work"

    # Harvey's build writes into the checkout (build/mkdirs, .build, the arch
    # dirs), so it runs on a writable copy, never on the store path.
    for d in sys amd64 arm64 rc lib build; do
      cp -r "${harveySrc}/$d" "$work/$d"
    done
    chmod -R u+w "$work"
    mkdir -p "$work/out"

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
      echo "S1-VENUE-FAIL /usr/bin/env exists: this build is not in a nix sandbox" >&2
      exit 1
    fi
    echo "SANDBOX-EVIDENCE usr-bin-env=absent"

    mkdir -p "$TMPDIR/guest"
    cat > "$TMPDIR/guest/guest.sh" <<'S1_GUEST_EOF'
    ${guestScript}
    S1_GUEST_EOF

    # ---- the unshare venue: nested userns + private tmpfs + chroot ----
    cat > "$TMPDIR/guest/ns.sh" <<'S1_NS_EOF'
    #!/usr/bin/env bash
    # NO cleanup trap and NO rm: the tmpfs dies with this namespace.
    set -euo pipefail

    # belt and braces: a child userns locks mount propagation, but ask for a
    # private tree explicitly so nothing can ever propagate to the parent.
    mount --make-rprivate / 2>/dev/null || echo "S1-VENUE-NOTE make-rprivate unavailable"

    NSX=$(mktemp -d "$TMPDIR/nsroot.XXXXXX")
    mount -t tmpfs -o mode=755 tmpfs "$NSX"
    echo "S1-VENUE-NSX=$NSX (private tmpfs)"

    mkdir -p "$NSX"/sys "$NSX"/amd64 "$NSX"/arm64 "$NSX"/rc "$NSX"/lib \
             "$NSX"/tmp "$NSX"/nix/store "$NSX"/dev "$NSX"/bin "$NSX"/out

    mount --bind "$WORK/sys" "$NSX/sys"
    mount --bind "$WORK/amd64" "$NSX/amd64"
    mount --bind "$WORK/arm64" "$NSX/arm64"
    mount --bind "$WORK/rc" "$NSX/rc"
    mount --bind "$WORK/lib" "$NSX/lib"
    mount --bind "$WORK/out" "$NSX/out"
    mount --bind /nix/store "$NSX/nix/store"

    for d in null zero random urandom; do
      if [ -e "/dev/$d" ]; then
        touch "$NSX/dev/$d"
        mount --bind "/dev/$d" "$NSX/dev/$d"
      else
        echo "S1-VENUE-NOTE /dev/$d absent in the sandbox"
      fi
    done

    # /bin: a real dir of symlinks (never a bind of the distro /bin).
    ln -s "$BASH_BIN" "$NSX/bin/sh"
    ln -s "$BASH_BIN" "$NSX/bin/bash"
    ln -s "$PLAN9_BIN/rc" "$NSX/bin/rc"
    ln -s "$PLAN9_BIN/mk" "$NSX/bin/mk"

    cp "$GUEST" "$NSX/guest.sh"

    echo "S1-VENUE-ENTER-CHROOT"
    exec "$CHROOT_BIN" "$NSX" /bin/sh /guest.sh
    S1_NS_EOF

    # ---- the proot venue: same root, faked with ptrace (no caps) ----
    cat > "$TMPDIR/guest/proot.sh" <<'S1_PROOT_EOF'
    #!/usr/bin/env bash
    # proot fakes the chroot with ptrace: no user namespace, no uid_map, no
    # mount calls. Binds are proot's -b mappings onto the work copy.
    set -euo pipefail

    NSX=$(mktemp -d "$TMPDIR/prootroot.XXXXXX")
    PROOT_TMP=$(mktemp -d "$TMPDIR/proot-tmp.XXXXXX")
    echo "S1-VENUE-PROOT-ROOT=$NSX"

    mkdir -p "$NSX"/dev "$NSX"/nix/store "$NSX"/bin "$NSX"/out

    ln -s "$BASH_BIN" "$NSX/bin/sh"
    ln -s "$BASH_BIN" "$NSX/bin/bash"
    ln -s "$PLAN9_BIN/rc" "$NSX/bin/rc"
    ln -s "$PLAN9_BIN/mk" "$NSX/bin/mk"
    cp "$GUEST" "$NSX/guest.sh"

    set -- -0 -r "$NSX" -w /
    set -- "$@" -b "$WORK/sys:/sys" -b "$WORK/amd64:/amd64"
    set -- "$@" -b "$WORK/arm64:/arm64" -b "$WORK/rc:/rc"
    set -- "$@" -b "$WORK/lib:/lib" -b "$WORK/out:/out"
    set -- "$@" -b /nix/store:/nix/store -b "$PROOT_TMP:/tmp"
    for d in null zero random urandom; do
      if [ -e "/dev/$d" ]; then
        set -- "$@" -b "/dev/$d:/dev/$d"
      else
        echo "S1-VENUE-NOTE /dev/$d absent in the sandbox"
      fi
    done

    echo "S1-VENUE-ENTER-PROOT"
    exec "$PROOT_BIN" "$@" /bin/sh /guest.sh
    S1_PROOT_EOF

    export WORK="$work"
    export GUEST="$TMPDIR/guest/guest.sh"
    export PLAN9_BIN="${plan9}/bin"
    export PLAN9="${plan9}"
    export BASH_BIN="${pkgs.bash}/bin/bash"
    export CHROOT_BIN="$(command -v chroot)"
    export PROOT_BIN="${pkgs.proot}/bin/proot"

    # plan9port's own bin (mk, rc, 9c, 9l, yacc, sed, ls, rm, ...) is not on
    # the nix PATH (only its `9` wrapper is); Harvey's build scripts assume
    # the Plan 9 tools resolve by name. Harvey's own wrapper appends
    # $PLAN9/bin LAST ("plan9 bin LAST per BUILD_ON_UNIX — plan9port ships a
    # 9p `mount` which must not shadow util-linux mount"), so do the same.
    export PATH="$PATH:$PLAN9_BIN"

    set +e
    {
      echo "S1-VENUE-BUILD-START mode=${mode}"
      echo "S1-VENUE-PATH=$PATH"
      if [ "${mode}" = "proot" ]; then
        "$BASH_BIN" "$TMPDIR/guest/proot.sh"
      else
        unshare -Ur -m "$BASH_BIN" "$TMPDIR/guest/ns.sh"
      fi
    } 2>&1 | tee "$TMPDIR/transcript.txt"
    rc="''${PIPESTATUS[0]}"
    set -e
    echo "S1-VENUE-RC=$rc"

    mkdir -p "$out"
    cp "$TMPDIR/transcript.txt" "$out/${transcriptFile}"
    if [ -n "$(ls -A "$work/out" 2>/dev/null)" ]; then
      cp -r "$work/out/." "$out/"
    fi

    [ "$rc" -eq 0 ] || {
      echo "S1-VENUE-FAIL the guest run exited $rc" >&2
      exit "$rc"
    }
    echo "S1-VENUE-OK"
  '';
}
