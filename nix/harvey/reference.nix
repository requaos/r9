# nix/harvey/reference.nix — the amd64 reference bake (D-29, HARN-03).
#
# One fresh run of Harvey's own x86_64 `mkimage.sh` on a WRITABLE copy of
# the flake-built amd64 tree (`nix/harvey/tree-amd64.nix`), recording the
# exact mkpaqfs input of each of the nine image contracts as a member
# list.  This is the D-29 reference the r9-assembled manifests are
# compared against; byte differentials are not used (assumption A12 was
# refuted).
#
# Venue: a plain `stdenvNoCC` derivation.  mkimage is a host shell bake;
# its host-side 6c/6l builds (sections 11k/14i2/14i3) run without a Plan 9
# namespace, so the proot venue of `mkTreeDerivation` is NOT needed here
# (recorded in the plan SUMMARY).
#
# Secret hygiene (T-01-50, HARN-01's prohibition): the store is
# world-readable, so the derivation NEVER copies a baked image or any auth
# artifact into `$out`.  It writes only `<image>.members` lists and a
# redacted `mkimage.log`, and it fails the build when:
#   * an `auth-creds`-named file reaches `$out`, or
#   * the generated DEMO password value occurs anywhere under `$out`
#     (the log is first filtered of every `password` line), or
#   * fewer/more than nine member lists were recorded.
#
# Auth material is listed by path, size and mode only (`opaque` in the
# sha256 field), so the reference exposes no key bytes.  The exact set is
# named in `opaqueMember` below with the mkimage.sh section that stages
# each path:
#   * §14f  adm/nvram, adm/keys, adm/domain.manifest, lib/ndb/auth
#   * §14f3 / §15e  lib/ndb/local (the @AUTHDOM@ substitution; auth/cpu) —
#     the services image's lib/ndb/local has the auth row stripped and
#     carries no authdom token, so it keeps a real hash.
#   * §14f2 boot/authfeed.txt (the DEMO sentinel feed)
#   * §15h  boot/authid-creds.txt (the authid's p9sk1 server key)
# `out/auth-creds.txt` (§14f2c/§15h) and `out/authfeed.txt` (§14f2b) are
# host-side bake copies under the writable tree, never staging members and
# never copied into `$out`.
#
# Member-list format: the plan 01-11 reference format the engine's
# `parse_reference_members` reads —
#   <path> file <mode> <size> <sha256>     (mode octal, no leading zero)
#   <path> dir  <mode> - -
# sorted by path, path relative to the image root with no leading `/`.
# (The plan's `f <path> <size> <sha256> <mode>` shorthand names the
# `[[tree]]` members file, a different format; `--compare-reference` reads
# this one.  Recorded as a plan-text fix in the SUMMARY.)
{
  pkgs,
  tree,
  hostbin,
  mkpaqfs,
  rev,
  kerndate,
}:

let
  bash = "${pkgs.bash}/bin/bash";
  realMkpaqfs = "${mkpaqfs}/bin/mkpaqfs";

  # The nine contracts, in mkimage.sh's own bake order (§1-6, §8-15).  The
  # shim maps the `-o` basename to these names: bootfs.paq -> marker,
  # bootfs-<x>.paq -> <x> (the churn contract is bootfs-churn.paq -> churn,
  # as CHURN_IMAGE names it in §12).
  images = [
    "marker"
    "chain"
    "tier"
    "net"
    "services"
    "churn"
    "substrate"
    "auth"
    "cpu"
  ];

  # The mkpaqfs shim: record the staging tree (mkpaqfs's exact input) as a
  # member list BEFORE handing the args to the real Linux mkpaqfs.  The
  # stdenv is Linux here, so GNU stat/sha256sum/sort are the tools.
  recordShim = pkgs.writeScript "harvey-reference-mkpaqfs-shim" ''
    #!${bash}
    set -euo pipefail
    real=${realMkpaqfs}

    image=
    stage=
    prev=
    for a in "$@"; do
      if [ "$prev" = "-o" ]; then image=$a; fi
      prev=$a
      stage=$a
    done
    [ -n "$image" ] || { echo "mkpaqfs-shim: no -o IMAGE argument" >&2; exit 2; }
    [ -d "$stage" ] || { echo "mkpaqfs-shim: last argument is not a directory: $stage" >&2; exit 2; }

    base=$(basename "$image")
    case "$base" in
      bootfs.paq) name=marker ;;
      bootfs-*.paq)
        name=''${base#bootfs-}
        name=''${name%.paq}
        ;;
      *)
        echo "mkpaqfs-shim: unrecognised image basename $base (want bootfs.paq or bootfs-<name>.paq)" >&2
        exit 2
        ;;
    esac

    mkdir -p "$R9_REFERENCE_MEMBERS"
    dest="$R9_REFERENCE_MEMBERS/$name.members"
    tmp="$dest.tmp"

    # The opaque set (see the file header).  lib/ndb/local is opaque only
    # for the auth and cpu images: only they carry the @AUTHDOM@ row.
    is_opaque() {
      case "$name:$1" in
        *:adm/nvram|*:adm/keys|*:adm/domain.manifest|*:lib/ndb/auth|\
        *:boot/authfeed.txt|*:boot/authid-creds.txt) return 0 ;;
        auth:lib/ndb/local|cpu:lib/ndb/local) return 0 ;;
      esac
      return 1
    }

    (
      cd "$stage"
      find . -mindepth 1 -print0 | LC_ALL=C sort -z | while IFS= read -r -d "" p; do
        rel=''${p#./}
        if [ -d "$p" ]; then
          mode=$(stat -c "%a" "$p")
          printf "%s dir %s - -\n" "$rel" "$mode"
        else
          mode=$(stat -c "%a" "$p")
          size=$(stat -c "%s" "$p")
          if is_opaque "$rel"; then
            sha=opaque
          else
            sha=$(sha256sum -- "$p" | cut -d" " -f1)
          fi
          printf "%s file %s %s %s\n" "$rel" "$mode" "$size" "$sha"
        fi
      done
    ) > "$tmp"
    mv "$tmp" "$dest"
    echo "mkpaqfs-record: $name members=$(wc -l < "$dest")" >&2

    exec "$real" "$@"
  '';
in
pkgs.stdenvNoCC.mkDerivation {
  pname = "harvey-reference-amd64";
  version = "0";

  dontUnpack = true;
  # The reference output is text member lists and a text log; the fixup
  # hooks have no business in them.
  dontStrip = true;
  dontPatchELF = true;
  dontPatchShebangs = true;

  nativeBuildInputs = [
    pkgs.bash
    pkgs.coreutils
    pkgs.gnused
    pkgs.gawk
    pkgs.gnugrep
    pkgs.findutils
    pkgs.diffutils
    pkgs.gnutar
    # §14d compiles authdomain-gen with the host C compiler.
    pkgs.stdenv.cc
    hostbin
  ];

  buildCommand = ''
    set -euo pipefail
    export TMPDIR="''${TMPDIR:-$NIX_BUILD_TOP}"

    fail() {
      echo "HARVEY-REFERENCE-FAIL $*" >&2
      exit 1
    }

    echo "HARVEY-REFERENCE-BEGIN tree=${tree} rev=${rev} kerndate=${toString kerndate}"

    # A writable copy of the flake-built tree: the store path is read-only
    # and the bake writes into build/x86_64/out, so the copy is mandatory.
    work="$TMPDIR/harvey"
    mkdir -p "$work"
    cp -R "${tree}/." "$work"
    chmod -R u+w "$work"

    # The reference is built from the same pinned tree the manifests draw
    # on (T-01-51): the tree's own stamp must name the plan's Harvey rev.
    [ -f "$work/HARVEY-TREE" ] || fail "the tree has no HARVEY-TREE stamp"
    grep -qx "rev=${rev}" "$work/HARVEY-TREE" \
      || fail "the tree stamp rev does not equal ${rev}: $(grep '^rev=' "$work/HARVEY-TREE")"
    grep -qx "arch=amd64" "$work/HARVEY-TREE" \
      || fail "the tree stamp arch is not amd64"
    [ -f "$work/build/x86_64/mkimage.sh" ] \
      || fail "the tree lacks build/x86_64/mkimage.sh"
    [ -d "$work/sys/src" ] || fail "the tree lacks sys/src"
    [ -d "$work/sys/include" ] \
      || fail "the tree lacks sys/include (tree-amd64.nix's install list must carry it for mkimage's host-side 6c builds)"

    # The shim at mkimage's own mkpaqfs path: it records the staging tree
    # and then runs the real Linux mkpaqfs (the byte-format owner).
    mkdir -p "$work/.build/bin"
    cp ${recordShim} "$work/.build/bin/mkpaqfs"
    chmod 755 "$work/.build/bin/mkpaqfs"
    grep -qF '${realMkpaqfs}' "$work/.build/bin/mkpaqfs" \
      || fail "the mkpaqfs shim was not installed as expected"

    export R9_REFERENCE_MEMBERS="$TMPDIR/members"
    rm -rf "$R9_REFERENCE_MEMBERS"
    mkdir -p "$R9_REFERENCE_MEMBERS"

    log="$TMPDIR/mkimage.log"
    echo "HARVEY-REFERENCE-RUN HARVEY=$work KERNDATE=${toString kerndate} venue=plain-stdenv hostbin=${hostbin}"
    # HARVEY_LINUX_BIN is mkimage §11d/§14i2/§14i3's host-toolchain root;
    # the hostbin path names the same 6c/6l pair.  hostbin is APPENDED to
    # PATH so its plan9port copies of sed/ls/date never shadow the GNU
    # tools the bake's own checks need.
    if ! (
      cd "$work"
      PATH="$PATH:${hostbin}/bin" \
      HARVEY="$work" \
      HARVEY_LINUX_BIN="${hostbin}/bin" \
      KERNDATE="${toString kerndate}" \
        sh build/x86_64/mkimage.sh
    ) > "$log" 2>&1; then
      echo "HARVEY-REFERENCE-FAIL the reference bake did not complete; tail of $log:" >&2
      grep -v -i 'password' "$log" | tail -n 80 >&2 || true
      exit 1
    fi

    # The nine member lists (fail with a named count otherwise).
    mkdir -p "$out"
    for img in ${builtins.concatStringsSep " " images}; do
      [ -s "$R9_REFERENCE_MEMBERS/$img.members" ] \
        || fail "the reference bake recorded no $img.members"
      cp "$R9_REFERENCE_MEMBERS/$img.members" "$out/$img.members"
    done
    count=$(find "$out" -maxdepth 1 -name '*.members' | wc -l | tr -d ' ')
    [ "$count" -eq 9 ] || fail "the reference bake produced $count member lists, want 9"

    # The bake log, minus every password-bearing line (T-01-50).
    grep -v -i 'password' "$log" > "$out/mkimage.log" || true
    [ -s "$out/mkimage.log" ] || fail "the redacted bake log is empty"

    # Secret checks: no auth-creds artifact and no generated password value
    # anywhere in the store output.
    if [ -n "$(find "$out" -iname '*auth-creds*' -print -quit)" ]; then
      fail "an auth-creds file is under \$out (the store is world-readable)"
    fi
    creds="$work/build/x86_64/out/auth-creds.txt"
    [ -s "$creds" ] \
      || fail "the §15 cpu bake left no out/auth-creds.txt — the password scan has no value to look for"
    pw=$(sed -n 's/^password=//p' "$creds" | head -n 1)
    [ -n "$pw" ] || fail "out/auth-creds.txt carries no password= line"
    if grep -RFq -- "$pw" "$out"; then
      fail "the generated DEMO password occurs in the reference output"
    fi
    if grep -Eq '^password=' "$out/mkimage.log"; then
      fail "a password= line survived the bake-log filter"
    fi

    # The auth images must list their key material as opaque (no hashes of
    # key bytes can reach the store either).
    for img in auth cpu; do
      n=$(grep -c ' opaque$' "$out/$img.members" || true)
      [ "$n" -ge 1 ] || fail "$img.members carries no opaque auth entry"
      for p in adm/nvram adm/keys adm/domain.manifest lib/ndb/auth; do
        grep -q "^$p file [0-9]* [0-9]* opaque$" "$out/$img.members" \
          || fail "$img.members does not list $p as opaque"
      done
    done

    for img in ${builtins.concatStringsSep " " images}; do
      echo "HARVEY-REFERENCE $img members=$(wc -l < "$out/$img.members" | tr -d ' ') opaque=$(grep -c ' opaque$' "$out/$img.members" || true)"
    done
    echo "HARVEY-REFERENCE-OK images=$count"
  '';
}
