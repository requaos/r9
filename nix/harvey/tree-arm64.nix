# nix/harvey/tree-arm64.nix — the full arm64 Harvey tree (HARN-03,
# HARN-07, D-03, D-19, D-25).
#
# Plan 01-13 shipped the tracer's smoke slice; this file extends the same
# derivation to everything Harvey's aarch64 `mkimage.sh` bakes into
# `build/aarch64/bootfs.paq` and everything the aarch64 gates read:
#
#   * the tier-0 library allowlist (build/aarch64/tiers/tier0-libs.txt)
#     with the D-07 fresh-archive rule;
#   * the tier-1 command allowlist (tiers/tier1-cmds.txt) dispatched
#     exactly as build/aarch64/subset-build.sh does (driver-only boot,
#     init and stacksmoke; the cc.a7 evidence entry; container leaves;
#     dir installs; per-target installs);
#   * the additive image payloads (dd, bind, ramfs, the net tools
#     ipconfig/ping/udpecho/aan/tcpblast/tcpecho/netblast, the probes
#     stacksmoke/forkstress/rfdprobe/brkprobe/cexecprobe/rcloseprobe/
#     sslprobe/fpprobe/cpuprobe) and the two tier-0 lib test binaries
#     mkimage bakes (htdraw, hdisktest);
#   * the committed skeleton, tiers, sweeps and fixed inputs mkimage and
#     the moved gates read from Harvey (D-19), as an explicit list with
#     read-site comments;
#   * the 01-13 smoke slice, kept so virtexec's tracer path still boots.
#
# The venue is plan 01-13's proot builder (nix/harvey/build-env.nix),
# reused unchanged: Harvey's hostbin toolchain, S4's two venue ports
# (/rc/bin on PATH, an explicit INCLUDE), the empty-tree guard and the
# case-clean assertions.  The store→proot copy ports (the nm shim,
# /var/tmp and PATH ending in `.`) follow plan 01-14's measured set.
#
# Work lists are never copied literals: each is extracted from the pinned
# tree at build time and its sha256 asserted against the value recorded
# here, so a Harvey change at a pin bump fails loudly (T-01-55).
{
  pkgs,
  harveySrc,
  rev,
  narHash,
}:

let
  env = import ./build-env.nix { inherit pkgs harveySrc; };

  # sha256 of each extracted list text at
  # bd7a6982a3e4f465f8ed3da5e0c7290521514ff0.  Re-record at a pin bump.
  #
  # Recipe (identical in the guest below):
  #   tiers:  gawk '!/^[ \t]*#/ && NF {print $1}' build/aarch64/tiers/<t>.txt
  #   lines:  grep '^<ANCHOR>' <script>            (the STAGE_SRCS precedent)
  #   probes: grep -oE '7c -FTVw [a-z0-9]+\.c' subset-build.sh | basenames
  tier0ListSha256 = "35fb5e839a228a5100c90494b8bdccdaf0e0ee8eeac243a220857699c41940f7";
  tier1ListSha256 = "71fd1c83d90045e665b6122dc676baa7d2e64c4c34fba54aa6d7583eedd0cc83";
  # mkimage.sh's four SUBSET= lines (the v1.0 16 subset + dd/bind/ramfs).
  subsetLinesSha256 = "3ab2229d50a3a8880cd8e92aeba51e3bd7f9340ca677c22a54721b534d056b7f";
  # mkimage.sh's three TIER1_BINS= lines (the baked tier-1 payload names).
  tier1BinsLinesSha256 = "44bfdbacb1a1e290b624c130add86786c11f6ee41ba48be8d031cc009cc1127e";
  # subset-build.sh's 7c -FTVw source list (the image probes).
  probeListSha256 = "4ab7dd0ba8af1ac4fce092ce6edba05b82fbd358990ad1eb45344c4ee8894f3f";
  # mkimage.sh's committed tier-1 fixed inputs line.
  fixedInputsLineSha256 = "4de59b09f52cbbd737ff0a80b038482394d00e410050b1c50ffa257ebac3c4c8";
  # sha256 of the pinned `STAGE_SRCS=...` line (stage-build.sh:54 at
  # bd7a6982a3e4f465f8ed3da5e0c7290521514ff0).  Re-record at a pin bump.
  stageSrcsLineSha256 = "311086077628ded705ccd5f10ebfca2676f3d718fd08f3f18e6f80b12c3f9e2d";
in
env.mkTreeDerivation {
  name = "harvey-tree-arm64";
  arch = "arm64";

  guestScript = ''
    set -eu

    echo "TREE-ARM64-BEGIN objtype=$objtype uid=$(id -u)"

    fail() {
      echo "TREE-ARM64-FAIL $*" >&2
      exit 1
    }

    # The entry's own log tail is the evidence a build step failed; the
    # rule is stop-and-report, never guess.
    dump_fail() {
      entry=$1
      log=$2
      echo "TREE-ARM64-FAIL $entry" >&2
      if [ -f "$log" ]; then
        echo "TREE-ARM64-FAIL log $log (last 40 lines):" >&2
        tail -n 40 "$log" >&2 || true
      fi
      exit 1
    }

    # cp -R of a store directory asks the builder's filesystem to set a
    # read-only directory mode, which the determinate-nixd builder refuses
    # with ENOTSUP (plan 01-14's measured class).  Copy the files instead;
    # the final tree is read-only anyway and Nix normalises the modes.
    copy_tree_files() {
      src=$1
      dst=$2
      [ -d "$src" ] || fail "copy_tree_files: $src missing"
      mkdir -p "$dst"
      ( cd "$src" && find . -type f -print ) | while read -r rel; do
        rel=''${rel#./}
        mkdir -p "$dst/$(dirname "$rel")"
        cp "$src/$rel" "$dst/$rel" || fail "copy $src/$rel -> $dst/$rel"
      done
    }

    # The mkfiles reach the tree at $HARVEY/<path>; the venue root IS the
    # tree (build-env.nix exports HARVEY empty and binds /sys, /arm64,
    # /rc, /lib and /build).
    export HARVEY=

    # libmach's per-arch readers include /<arch>/include/ureg.h by
    # ABSOLUTE path (arm64/lib/libmach.a is a tier-0 entry: nm is a
    # tier-1 command).  The venue binds only this tree's own arch dir, so
    # stage the sibling arch dirs from the pinned store copy — plan
    # 01-14's measured port set.
    for d in 386 amd64 arm mips mips64 power power64 riscv riscv64 sparc acme unix; do
      if [ ! -e "/$d" ] && [ -d "${harveySrc}/$d" ]; then
        copy_tree_files "${harveySrc}/$d" "/$d"
      fi
    done

    # plan9port's ed writes its scratch at the absolute /var/tmp/eXXXXX
    # (the compiler mkcname/mkenam generators run ed); the proot root has
    # no /var (the 01-14 measured port).
    mkdir -p /var/tmp

    # plan9port 9l's autolib execs bare `nm` on HOST ELF objects; hostbin
    # carries Harvey's ken-format nm, so shadow it with binutils' (the S4
    # / subset-build shim).
    SHIM=/tmp/tree-arm64-shim
    mkdir -p "$SHIM"
    ln -sf "${pkgs.binutils}/bin/nm" "$SHIM/nm"
    # Plan 9's default path ends in `.`, and rc's `.` builtin / mk's bare
    # command lookup resolve cwd-local generator scripts through $path;
    # keep it LAST so a cwd file can never shadow a toolchain command.
    export PATH="$SHIM:$PATH:."

    export objtype=arm64 cputype=unix FORCERCFORMK=1
    export INCLUDE="/arm64/include /sys/include"

    # sys/src/cmd/mkfile's `%.install` rule stages every command through
    # `$cpuobjtype._cp`, whose prerequisite is the ABSOLUTE /bin/cp (the
    # rule runs `exec -a cp /bin/cp`).  The proot root's /bin carries only
    # the venue's sh/bash/rc/mk symlinks, so name the host cp there (plan
    # 01-14's singles went through the 6.<name> link + manual copy instead;
    # the arm64 dispatch is mk <name>.install, so the rule's prerequisite
    # must resolve).  The generated o._cp wrapper's
    # `#!/usr/bin/env bash` shebang also needs /usr/bin/env inside the
    # proot root (the sandbox-evidence refusal is about the HOST /usr/bin,
    # checked before proot starts; this is the disposable NSX root).
    ln -sfn "$(command -v cp)" /bin/cp
    mkdir -p /usr/bin
    ln -sfn "$(command -v env)" /usr/bin/env

    mkdir -p /arm64/bin /arm64/lib /out/build/aarch64/out

    # ------------------------------------------------------------------
    # The work lists: the pinned data, never a copied literal.
    # ------------------------------------------------------------------
    extract_tier() {
      gawk '!/^[ \t]*#/ && NF {print $1}' "/build/aarch64/tiers/$1.txt" > "/tmp/$1.entries"
      got=$(sha256sum "/tmp/$1.entries" | cut -d' ' -f1)
      [ "$got" = "$2" ] || {
        echo "TREE-ARM64-FAIL $1 list changed at this pin: sha256 $got != $2" >&2
        exit 1
      }
      n=$(wc -l < "/tmp/$1.entries" | tr -d ' ')
      echo "TREE-ARM64-LIST $1 entries=$n sha256=$got"
    }

    extract_tier tier0-libs ${tier0ListSha256}
    extract_tier tier1-cmds ${tier1ListSha256}

    line_sha() {
      printf '%s\n' "$1" | sha256sum | cut -d' ' -f1
    }

    # mkimage.sh's SUBSET list (the v1.0 16 subset plus dd, bind, ramfs —
    # the append form keeps each addition insertion-only at the pin).
    subset_lines=$(grep '^SUBSET=' /build/aarch64/mkimage.sh) \
      || fail "no SUBSET= lines in build/aarch64/mkimage.sh"
    [ "$(line_sha "$subset_lines")" = "${subsetLinesSha256}" ] \
      || fail "the SUBSET lines changed at this pin: sha256 $(line_sha "$subset_lines") != ${subsetLinesSha256}"
    SUBSET=$(printf '%s\n' "$subset_lines" \
      | sed -e 's/^SUBSET="//' -e 's/".*$//' -e 's/^\$SUBSET //' \
      | tr '\n' ' ')
    echo "TREE-ARM64-SUBSET=$SUBSET"

    # mkimage.sh's TIER1_BINS list (the h* payload each tier-1 name bakes).
    tier1bins_lines=$(grep '^TIER1_BINS=' /build/aarch64/mkimage.sh) \
      || fail "no TIER1_BINS= lines in build/aarch64/mkimage.sh"
    [ "$(line_sha "$tier1bins_lines")" = "${tier1BinsLinesSha256}" ] \
      || fail "the TIER1_BINS lines changed at this pin: sha256 $(line_sha "$tier1bins_lines") != ${tier1BinsLinesSha256}"
    TIER1_BINS=$(printf '%s\n' "$tier1bins_lines" \
      | sed -e 's/^TIER1_BINS="//' -e 's/".*$//' -e 's/^\$TIER1_BINS //' \
      | tr '\n' ' ')
    echo "TREE-ARM64-TIER1-BINS=$TIER1_BINS"

    # The image probes: every `7c -FTVw <name>.c` compile in
    # subset-build.sh, in the pinned file order.
    grep -oE '7c -FTVw [a-z0-9]+\.c' /build/aarch64/subset-build.sh \
      | sed 's/.* //; s/\.c$//' > /tmp/probe.entries
    got=$(sha256sum /tmp/probe.entries | cut -d' ' -f1)
    [ "$got" = "${probeListSha256}" ] \
      || fail "the probe source list changed at this pin: sha256 $got != ${probeListSha256}"
    PROBES=$(tr '\n' ' ' < /tmp/probe.entries)
    echo "TREE-ARM64-PROBES=$PROBES"

    # mkimage.sh's committed fixed inputs (the tier-1 sweep's data).
    fixed_line=$(grep -m1 '^for f in tiny.y tiny.l tiny.mk tiny.s nm-input.7 hello.c;' /build/aarch64/mkimage.sh) \
      || fail "no committed fixed-inputs loop in build/aarch64/mkimage.sh"
    [ "$(line_sha "$fixed_line")" = "${fixedInputsLineSha256}" ] \
      || fail "the fixed-inputs list changed at this pin: sha256 $(line_sha "$fixed_line") != ${fixedInputsLineSha256}"
    FIXED_INPUTS=$(printf '%s' "$fixed_line" | gawk '{ sub(/^for f in /, ""); sub(/; do$/, ""); print }')

    # The 01-13 smoke source list is the pinned data, never a copied literal.
    stage_line=$(grep -m1 '^STAGE_SRCS=' /build/aarch64/stage-build.sh) || {
      echo "TREE-ARM64-FAIL no STAGE_SRCS= line in /build/aarch64/stage-build.sh" >&2
      exit 1
    }
    stage_hash=$(line_sha "$stage_line")
    echo "TREE-ARM64-STAGE-SRCS-SHA256=$stage_hash"
    [ "$stage_hash" = "${stageSrcsLineSha256}" ] || {
      echo "TREE-ARM64-FAIL STAGE_SRCS changed at this pin: sha256 $stage_hash != ${stageSrcsLineSha256}" >&2
      echo "TREE-ARM64-FAIL line: $stage_line" >&2
      exit 1
    }
    STAGE_SRCS=$(printf '%s' "$stage_line" | sed -e 's/^STAGE_SRCS=//' -e 's/^"//' -e 's/"$//')
    echo "TREE-ARM64-STAGE-SRCS=$STAGE_SRCS"
    for f in $STAGE_SRCS; do
      [ -f "/build/aarch64/$f.c" ] || {
        echo "TREE-ARM64-FAIL build/aarch64/$f.c missing" >&2
        exit 1
      }
    done

    # ------------------------------------------------------------------
    # Every installed artifact must be an arm64 Plan 9 a.out
    # (R_MAGIC 0x8C47, big-endian: 00 00 8c 47).
    # ------------------------------------------------------------------
    check_magic() {
      f=$1
      what=$2
      [ -s "$f" ] || fail "$what: no artifact $f"
      m=$(od -An -tx1 -N4 "$f" | tr -d ' \n')
      [ "$m" = "00008c47" ] || fail "$what: bad magic '$m' (want 00008c47) at $f"
    }

    # ------------------------------------------------------------------
    # Tier 0: the library allowlist, with the D-07 fresh-archive rule
    # (rm -f the target .a before mk install — ar vu never prunes).
    # ------------------------------------------------------------------
    build_tier0() {
      T=/tmp/tier0-libs.entries
      N=$(wc -l < "$T" | tr -d ' ')
      start=$SECONDS
      rm -f /arm64/lib/libc.a
      ( cd /sys/src/libc && objtype=arm64 mk install ) > /tmp/tier0-libc.log 2>&1 \
        || dump_fail "tier0 libc bootstrap" /tmp/tier0-libc.log
      [ -s /arm64/lib/libc.a ] || fail "tier0 libc: no /arm64/lib/libc.a"
      k=0
      while read -r d; do
        [ -n "$d" ] || continue
        case "$d" in
        liblex) lib=libl.a ;;
        *) lib="lib''${d#lib}.a" ;;
        esac
        log=/tmp/tier0-$d.log
        rm -f "/arm64/lib/$lib"
        if ( cd "/sys/src/$d" && objtype=arm64 mk install ) > "$log" 2>&1 \
           && [ -s "/arm64/lib/$lib" ]; then
          k=$((k+1))
        else
          dump_fail "tier0 $d" "$log"
        fi
      done < "$T"
      echo "TREE-ARM64-SECONDS tier0=$((SECONDS-start))"
      echo "TREE-ARM64 tier0 built=$k/$N"
      [ "$k" = "$N" ] || fail "tier0 built $k of $N"
    }

    # ------------------------------------------------------------------
    # Tier 1: the command allowlist, dispatched per entry form exactly
    # as subset-build.sh does (driver-only boot/init/stacksmoke, the
    # cc.a7 evidence entry, container leaves, dir installs, per-target
    # installs with the driver's paqfs harvest).
    # ------------------------------------------------------------------
    build_tier1() {
      T=/tmp/tier1-cmds.entries
      N=$(wc -l < "$T" | tr -d ' ')
      start=$SECONDS
      k=0
      while read -r entry; do
        [ -n "$entry" ] || continue
        log="/tmp/tier1-$(printf '%s' "$entry" | tr '/:' '__').log"
        ok=1
        art=
        case "$entry" in
        9k/boot)
          ( cd /sys/src/9k/boot && objtype=arm64 mk -f mkfile.arm64 ) > "$log" 2>&1 || ok=0
          if [ "$ok" = 1 ]; then
            cp /sys/src/9k/boot/boot /out/build/aarch64/out/boot >> "$log" 2>&1 || ok=0
          fi
          ( cd /sys/src/9k/boot && objtype=arm64 mk -f mkfile.arm64 clean ) > /dev/null 2>&1 || true
          art=/out/build/aarch64/out/boot
          ;;
        stage/init)
          ( cd /sys/src/cmd && objtype=arm64 mk init.install ) > "$log" 2>&1 || ok=0
          if [ "$ok" = 1 ]; then
            cp /arm64/init /out/build/aarch64/out/init >> "$log" 2>&1 || ok=0
          fi
          art=/out/build/aarch64/out/init
          ;;
        stage/stacksmoke)
          ( cd /build/aarch64 && objtype=arm64 7c -FTVw stacksmoke.c \
            && objtype=arm64 7l -o /out/build/aarch64/out/stacksmoke stacksmoke.7 \
            && rm -f stacksmoke.7 ) > "$log" 2>&1 || ok=0
          art=/out/build/aarch64/out/stacksmoke
          ;;
        cmd/cc)
          # the shared frontend: the tier evidence is the cc.a7 archive the
          # 7c build links (subset-build's own special case)
          rm -f /sys/src/cmd/cc/cc.a7
          ( cd /sys/src/cmd/cc && objtype=arm64 mk install ) > "$log" 2>&1 || ok=0
          [ "$ok" = 1 ] && [ -s /sys/src/cmd/cc/cc.a7 ] || ok=0
          ;;
        cmd/*:*)
          dir=''${entry%%:*}
          tgt=''${entry##*:}
          ( cd "/sys/src/$dir" && objtype=arm64 mk "$tgt.install" ) > "$log" 2>&1 || ok=0
          art="/arm64/bin/''${dir#cmd/}"
          if [ "$ok" = 1 ] && [ "$tgt" = paqfs ]; then
            cp /arm64/bin/paqfs /out/build/aarch64/out/paqfs >> "$log" 2>&1 || ok=0
          fi
          ;;
        cmd/*)
          if [ -d "/sys/src/$entry" ]; then
            ( cd "/sys/src/$entry" && objtype=arm64 mk install ) > "$log" 2>&1 || ok=0
            art="/arm64/bin/''${entry#cmd/}"
          else
            name=''${entry#cmd/}
            ( cd /sys/src/cmd && objtype=arm64 mk "$name.install" ) > "$log" 2>&1 || ok=0
            art="/arm64/bin/$name"
          fi
          ;;
        *)
          ok=0
          echo "entry form not understood: $entry" > "$log"
          ;;
        esac
        [ "$ok" = 1 ] || dump_fail "tier1 $entry" "$log"
        [ "$entry" = cmd/cc ] || check_magic "$art" "tier1 $entry"
        k=$((k+1))
      done < "$T"
      echo "TREE-ARM64-SECONDS tier1=$((SECONDS-start))"
      echo "TREE-ARM64 tier1 built=$k/$N"
      [ "$k" = "$N" ] || fail "tier1 built $k of $N"
    }

    # ------------------------------------------------------------------
    # The additive payloads (subset-build.sh's post-tier blocks): dd,
    # bind and ramfs; the net tools; the image probes; the two tier-0
    # lib test binaries mkimage bakes (htdraw, hdisktest).
    # hdrawtest/harctest are excluded BY DESIGN by mkimage (the faulting
    # libmemdraw pair), so they are not built here.
    # ------------------------------------------------------------------
    build_payloads() {
      start=$SECONDS
      k=0
      n=0

      echo "== subset cmd: dd (per-target install, 07-04)"
      ( cd /sys/src/cmd && objtype=arm64 mk dd.install ) > /tmp/subset-dd.log 2>&1 \
        || dump_fail "subset dd" /tmp/subset-dd.log
      check_magic /arm64/bin/dd "subset dd"
      n=$((n+1)); k=$((k+1))

      echo "== subset cmd: bind (per-target install, 07-07)"
      ( cd /sys/src/cmd && objtype=arm64 mk bind.install ) > /tmp/subset-bind.log 2>&1 \
        || dump_fail "subset bind" /tmp/subset-bind.log
      check_magic /arm64/bin/bind "subset bind"
      n=$((n+1)); k=$((k+1))

      echo "== subset cmd: ramfs (per-target install, 20-07)"
      ( cd /sys/src/cmd && objtype=arm64 mk ramfs.install ) > /tmp/subset-ramfs.log 2>&1 \
        || dump_fail "subset ramfs" /tmp/subset-ramfs.log
      check_magic /arm64/bin/ramfs "subset ramfs"
      n=$((n+1)); k=$((k+1))

      echo "== subset cmd: ipconfig (dir install + out/ harvest, 08-03)"
      ( cd /sys/src/cmd/ip/ipconfig && objtype=arm64 mk install ) > /tmp/subset-ipconfig.log 2>&1 \
        || dump_fail "subset ip/ipconfig" /tmp/subset-ipconfig.log
      cp /arm64/bin/ip/ipconfig /out/build/aarch64/out/ipconfig >> /tmp/subset-ipconfig.log 2>&1 \
        || dump_fail "subset ip/ipconfig harvest" /tmp/subset-ipconfig.log
      check_magic /arm64/bin/ip/ipconfig "subset ip/ipconfig"
      check_magic /out/build/aarch64/out/ipconfig "subset ip/ipconfig harvest"
      n=$((n+2)); k=$((k+2))

      echo "== subset cmd: ping (per-target install + out/ harvest, 08-03)"
      ( cd /sys/src/cmd/ip && objtype=arm64 mk ping.install ) > /tmp/subset-ping.log 2>&1 \
        || dump_fail "subset ip/ping" /tmp/subset-ping.log
      cp /arm64/bin/ip/ping /out/build/aarch64/out/ping >> /tmp/subset-ping.log 2>&1 \
        || dump_fail "subset ip/ping harvest" /tmp/subset-ping.log
      check_magic /arm64/bin/ip/ping "subset ip/ping"
      check_magic /out/build/aarch64/out/ping "subset ip/ping harvest"
      n=$((n+2)); k=$((k+2))

      echo "== subset cmd: udpecho (per-target install + out/ harvest, 08-03)"
      ( cd /sys/src/cmd/ip && objtype=arm64 mk udpecho.install ) > /tmp/subset-udpecho.log 2>&1 \
        || dump_fail "subset ip/udpecho" /tmp/subset-udpecho.log
      cp /arm64/bin/ip/udpecho /out/build/aarch64/out/udpecho >> /tmp/subset-udpecho.log 2>&1 \
        || dump_fail "subset ip/udpecho harvest" /tmp/subset-udpecho.log
      check_magic /arm64/bin/ip/udpecho "subset ip/udpecho"
      check_magic /out/build/aarch64/out/udpecho "subset ip/udpecho harvest"
      n=$((n+2)); k=$((k+2))

      echo "== subset cmd: aan (per-target install + out/ harvest, 08-03)"
      ( cd /sys/src/cmd && objtype=arm64 mk aan.install ) > /tmp/subset-aan.log 2>&1 \
        || dump_fail "subset aan" /tmp/subset-aan.log
      cp /arm64/bin/aan /out/build/aarch64/out/aan >> /tmp/subset-aan.log 2>&1 \
        || dump_fail "subset aan harvest" /tmp/subset-aan.log
      check_magic /arm64/bin/aan "subset aan"
      check_magic /out/build/aarch64/out/aan "subset aan harvest"
      n=$((n+2)); k=$((k+2))

      # The probes (subset-build.sh's 7c -FTVw blocks).  fpprobe carries a
      # WORD-encoded asm file (fpprobeasm.s) assembled by 7a beside the C.
      for p in $PROBES; do
        n=$((n+1))
        echo "== image probe: $p"
        if [ "$p" = fpprobe ]; then
          ( cd /build/aarch64 && objtype=arm64 7c -FTVw fpprobe.c \
            && objtype=arm64 7a fpprobeasm.s \
            && objtype=arm64 7l -o "/out/build/aarch64/out/fpprobe" fpprobe.7 fpprobeasm.7 \
            && rm -f fpprobe.7 fpprobeasm.7 ) > "/tmp/probe-$p.log" 2>&1 \
            || dump_fail "probe $p" "/tmp/probe-$p.log"
        else
          ( cd /build/aarch64 && objtype=arm64 7c -FTVw "$p.c" \
            && objtype=arm64 7l -o "/out/build/aarch64/out/$p" "$p.7" \
            && rm -f "$p.7" ) > "/tmp/probe-$p.log" 2>&1 \
            || dump_fail "probe $p" "/tmp/probe-$p.log"
        fi
        check_magic "/out/build/aarch64/out/$p" "probe $p"
        k=$((k+1))
      done

      # The two tier-0 lib test binaries mkimage stages (htdraw, hdisktest).
      echo "== tier-0 lib test binaries (libdraw/libdisk, the mkimage payload)"
      ( cd /sys/src/libdraw && objtype=arm64 mk 7.test ) > /tmp/libdraw-test.log 2>&1 \
        || dump_fail "libdraw 7.test" /tmp/libdraw-test.log
      cp /sys/src/libdraw/7.test /out/build/aarch64/out/htdraw >> /tmp/libdraw-test.log 2>&1 \
        || dump_fail "libdraw htdraw harvest" /tmp/libdraw-test.log
      rm -f /sys/src/libdraw/7.test /sys/src/libdraw/test.7
      check_magic /out/build/aarch64/out/htdraw "libdraw htdraw"
      n=$((n+1)); k=$((k+1))

      ( cd /sys/src/libdisk && objtype=arm64 mk 7.test ) > /tmp/libdisk-test.log 2>&1 \
        || dump_fail "libdisk 7.test" /tmp/libdisk-test.log
      cp /sys/src/libdisk/7.test /out/build/aarch64/out/hdisktest >> /tmp/libdisk-test.log 2>&1 \
        || dump_fail "libdisk hdisktest harvest" /tmp/libdisk-test.log
      rm -f /sys/src/libdisk/7.test /sys/src/libdisk/test.7
      check_magic /out/build/aarch64/out/hdisktest "libdisk hdisktest"
      n=$((n+1)); k=$((k+1))

      # mkimage's TIER1_BINS: every baked tier-1 name must be installed.
      for f in $TIER1_BINS; do
        [ -f "/arm64/bin/$f" ] || fail "tier1 payload /arm64/bin/$f missing after the build"
      done
      # mkimage's SUBSET: every baked subset name must be installed.
      for f in $SUBSET; do
        [ -f "/arm64/bin/$f" ] || fail "subset payload /arm64/bin/$f missing after the build"
      done

      echo "TREE-ARM64-SECONDS payloads=$((SECONDS-start))"
      echo "TREE-ARM64 payloads built=$k/$n"
      [ "$k" = "$n" ] || fail "payloads built $k of $n"
    }

    # ------------------------------------------------------------------
    # The 01-13 smoke slice (kept: virtexec's tracer path boots it).
    # ------------------------------------------------------------------
    build_smoke() {
      start=$SECONDS
      mkdir -p /out/build/aarch64/smoke-out
      for f in $STAGE_SRCS; do
        ( cd /build/aarch64 && objtype=arm64 7c -FTVw "$f.c" \
          && objtype=arm64 7l -o "/out/build/aarch64/smoke-out/$f" "$f.7" \
          && rm -f "$f.7" ) || {
          echo "TREE-ARM64-FAIL compile $f" >&2
          exit 1
        }
      done
      # The committed ABI bisect binary rides the tree verbatim (as `hello`).
      cp /build/aarch64/hello.7.out /out/build/aarch64/smoke-out/hello
      for f in $STAGE_SRCS hello; do
        check_magic "/out/build/aarch64/smoke-out/$f" "smoke $f"
      done
      echo "TREE-ARM64-SECONDS smoke=$((SECONDS-start))"
    }

    SECONDS=0
    build_tier0
    build_tier1
    build_payloads
    build_smoke

    # ------------------------------------------------------------------
    # The built count: every artifact the image and the smoke slice
    # carry, magic-checked.  built=<k>/<n> fails unless equal.
    # ------------------------------------------------------------------
    k=0
    n=0
    for f in $(find /arm64/bin /out/build/aarch64/out /out/build/aarch64/smoke-out /arm64/init -type f); do
      n=$((n+1))
      m=$(od -An -tx1 -N4 "$f" | tr -d ' \n')
      [ "$m" = "00008c47" ] || fail "final scan: $f has bad magic '$m' (want 00008c47)"
      k=$((k+1))
    done
    echo "TREE-ARM64 built=$k/$n"
    [ "$k" = "$n" ] || fail "built $k of $n"
    echo "TREE-ARM64-BUILD-OK"
  '';

  installScript = ''
    set -eu

    fail() {
      echo "TREE-ARM64-FAIL $*" >&2
      exit 1
    }

    TREE=/out

    # ---- the built userland -------------------------------------------
    mkdir -p "$TREE/arm64/bin" "$TREE/arm64/lib" "$TREE/arm64/include"
    copy_tree_files /arm64/bin "$TREE/arm64/bin"
    copy_tree_files /arm64/lib "$TREE/arm64/lib"
    copy_tree_files /arm64/include "$TREE/arm64/include"
    [ -s /arm64/init ] || fail "no /arm64/init after the tier build"
    cp /arm64/init "$TREE/arm64/init"

    # build/aarch64/out and smoke-out are written directly under /out by
    # the guest build (the exact relative paths mkimage.sh reads).
    [ -s "$TREE/build/aarch64/out/boot" ] || fail "out/boot missing"
    [ -s "$TREE/build/aarch64/smoke-out/init" ] || fail "smoke-out/init missing"

    # ---- the source-side install list (D-19) ---------------------------
    # Every entry below is named by a read in build/aarch64/mkimage.sh at
    # the pin, or by a moved aarch64 gate's fixture read.  The path is
    # installed read-only at the same tree-relative location.  rc/bin is
    # never installed wholesale (the case-colliding set); no 9k kernel or
    # r9 artifact rides here.
    #
    #   build/aarch64/root-tree/      mkimage step 1 (the committed
    #                                 skeleton: namespace, termrc, cpurc,
    #                                 shebangecho, net-sweep.rc, netip-*.rc,
    #                                 writeprobe.rc, test/arp.req, profile)
    #   build/aarch64/tiers/          mkimage's TIER1 input and the
    #                                 `cargo xtask image --libraries` tier-0
    #                                 audit (tier0-libs.txt, tier1-cmds.txt)
    #   build/aarch64/mkimage.sh      the reference bake itself (the D-29
    #                                 arm64 reference, plan 01-17)
    #   build/aarch64/netaddr.rc      mkimage step 4 (the D-11 seam) and
    #                                 xtask pcap's netaddr reader
    #   build/aarch64/tier1-sweep.rc  mkimage step 3b (the D-10 sweep) and
    #                                 tier1-r9.sh's T1-SKIP computation
    #   build/aarch64/{tiny.y,tiny.l,tiny.mk,tiny.s,nm-input.7,hello.c}
    #                                 mkimage step 3b (the committed tier-1
    #                                 fixed inputs: yacc/lex/mk/as/nm data)
    #   rc/lib/rcmain                 mkimage step 4 (ships VERBATIM; the
    #                                 single source of truth for rc startup)
    #   adm/timezone/local            mkimage step 4 (init's cpenv data)
    src_dirs="
    build/aarch64/root-tree
    build/aarch64/tiers
    "
    for d in $src_dirs; do
      [ -d "${harveySrc}/$d" ] || fail "source-side dir ${harveySrc}/$d missing"
      copy_tree_files "${harveySrc}/$d" "$TREE/$d"
    done

    src_files="
    build/aarch64/mkimage.sh
    build/aarch64/netaddr.rc
    build/aarch64/tier1-sweep.rc
    build/aarch64/tiny.y
    build/aarch64/tiny.l
    build/aarch64/tiny.mk
    build/aarch64/tiny.s
    build/aarch64/nm-input.7
    build/aarch64/hello.c
    rc/lib/rcmain
    adm/timezone/local
    "
    for f in $src_files; do
      [ -f "${harveySrc}/$f" ] || fail "source-side file ${harveySrc}/$f missing"
      mkdir -p "$TREE/$(dirname "$f")"
      cp "${harveySrc}/$f" "$TREE/$f"
    done

    # mkimage step 3b's generated tier-1 sentinel: the exact printf bytes
    # at build/aarch64/mkimage.sh ("alpha bravo charlie" /
    # "__SUBSET_GREP_LINE__").  mkimage generates this file itself; the
    # tree carries the same bytes so the D-29 manifest emitter can map
    # the reference member to a Harvey-sourced file instead of an
    # UNRESOLVED entry (the emitter matches by sha256; no generator is
    # registered for it).
    printf 'alpha bravo charlie\n__SUBSET_GREP_LINE__\n' > "$TREE/build/aarch64/tier1-sentinel.txt"

    # ---- the stamp the resolver checks --------------------------------
    cat > "$TREE/HARVEY-TREE" <<'HARVEY_TREE_STAMP'
    rev=${rev}
    narHash=${narHash}
    arch=arm64
    system=${pkgs.stdenv.hostPlatform.system}
    HARVEY_TREE_STAMP

    echo "TREE-ARM64-INSTALL-OK members=$(find "$TREE" -type f | wc -l | tr -d ' ')"
  '';
}
