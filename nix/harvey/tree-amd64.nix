# nix/harvey/tree-amd64.nix — the full amd64 Harvey tree (HARN-07, D-03,
# D-19, D-25).
#
# Every x86_64 image r9 assembles draws from this one read-only store
# path: the tier-0 libraries, the tier 1-4 userland, the net section,
# the stage-build marker/chain/fixture programs, and the source-side
# data the assembler and the moved gates read from Harvey (D-19).
#
# The venue is plan 01-13's proot builder (nix/harvey/build-env.nix),
# reused unchanged: Harvey's hostbin toolchain, the SANDBOX-EVIDENCE
# refusal, S4's two venue ports (/rc/bin on PATH, an explicit INCLUDE),
# the empty-tree guard and the case-clean assertions.  S4 proved tiers
# 0 and 1 here; this plan extends the same dispatch to every tier and
# to the stage-build programs.
#
# The dispatch mirrors build/x86_64/subset-build.sh at the pin (the
# per-tier semantics: tier-0 fresh archive, dir install, link-only
# harvest, container leaves, driver-only entries, per-entry fresh build
# for the mass command tiers), with one deliberate difference: a failing
# entry fails the derivation immediately, naming the entry and the tail
# of its log.  tier4-tail has no driver loop in the pin by design; it is
# built with the tier-2/3 per-entry fresh-build discipline (clean ->
# install -> harvest the new /amd64/bin files).
#
# Work lists are never copied literals: each is extracted from the
# pinned tree at build time and its sha256 asserted against the value
# recorded here, so a Harvey change at a pin bump fails loudly (T-01-47).
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
  #   tiers:  gawk '!/^[ \t]*#/ && NF {print $1}' build/x86_64/tiers/<t>.txt
  #   lines:  grep -m1 '<anchor>' <script>, printed with one trailing
  #           newline (the tree-arm64 STAGE_SRCS precedent)
  tier0ListSha256 = "c0ad974bc0e43172b7a23cd6943da550d803eb11e6825c13bd4afe62b0113ba0";
  tier1ListSha256 = "104e22432a4ec38ad885c8108a17f7e9a82419ed1cca1c60456b76ce56efb941";
  tier2ListSha256 = "ec95153183d7e3b761036bffdff1a18e6a3e0c61357e7ee70c612e35774bf246";
  tier3ListSha256 = "446a26584f8b7ec6282fb418a6364f1367e542b228c6ef0e580495ec19d36c1d";
  tier4ListSha256 = "f9e446d4a6cc5cef204baf3f2424816bd603954961bfaf2f7dced02237552335";
  # STAGE_SRCS="smokesleep churnmark churnsem churncats randread"
  stageSrcsLineSha256 = "449ace85b42b4219f4ed85a674d27b1bef2cb5bb3762fb0ee04fc001ae24d23f";
  # CHAIN_LIBS="bio auth ip mp sec flate authsrv"
  chainLibsLineSha256 = "890f0020018b5d26bc1c494ea799fc5707e31aa7655f7a9b6d674a6e7a6edd2b";
  # for f in boot init rc paqfs ls cat echo ps; do
  chainListLineSha256 = "3d5dea60774dd9900be1be2aaaf52144930ff371e70d4b792da93169423f16c9";
  # for f in fpprobe.c fpprobeasm.s; do
  fpprobeLineSha256 = "5c1de7459fea8dd36fa0123982dda653990254ac9db5f2563520d0b43350db2a";
  # for cmd in ls cat echo ps rm test; do
  chainSweepLineSha256 = "a64beae9502b210e10600e13a27cd87f75e2a44e6dcda2d3b7a491bd30510cec";
  # the `echo "== net cmd: <name> ..."` labels (ping udpecho dd)
  netLabelsSha256 = "409b60ba4d77376895dcc32626075e1696d70aaf448afef3d3fab0facc9db36d";
  # the `echo "== desktop cmd: <name> ..."` labels (syscall awd)
  desktopLabelsSha256 = "5f07b9fa23c329e0e0ba0f4a9a644f16414e1fd075633eee82cbac7fca782ce8";
  # for h in netblast tcpblast tcpecho; do
  netHelpersLineSha256 = "e5e9eadc06c90c835f54c6dfafd8cc95c8cafb66f9687c708fe72ed1b181b30c";
in
env.mkTreeDerivation {
  name = "harvey-tree-amd64";
  arch = "amd64";

  guestScript = ''
    set -eu

    echo "TREE-AMD64-BEGIN objtype=$objtype uid=$(id -u)"

    fail() {
      echo "TREE-AMD64-FAIL $*" >&2
      exit 1
    }

    # The entry's own log tail is the evidence a tier entry failed; the
    # plan's rule is stop-and-report, never guess.
    dump_fail() {
      entry=$1
      log=$2
      echo "TREE-AMD64-FAIL $entry" >&2
      if [ -f "$log" ]; then
        echo "TREE-AMD64-FAIL log $log (last 40 lines):" >&2
        tail -n 40 "$log" >&2 || true
      fi
      exit 1
    }

    # cp -R of a store directory asks the builder's filesystem to set a
    # read-only directory mode, which the determinic-nixd builder refuses
    # with ENOTSUP ("setting permissions ...: Operation not supported").
    # Copy the files instead and create the directories with mkdir; the
    # final tree is read-only anyway and Nix normalises the modes.
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

    # ------------------------------------------------------------------
    # The mkfiles reach a few tree dirs by absolute path.  The venue
    # binds sys/amd64/rc/lib/build; the rest are staged from the pinned
    # store copy into the (writable, disposable) proot root:
    #   * libmach's 5.c/8.c/i.c/k.c/q.c/v.c include /arm,/386,/riscv,
    #     /sparc,/power,/mips include/ureg.h by absolute path;
    #   * /unix is awk's host-safe maketab mkfile ($cputype.maketab);
    #   * /acme rides S4's proven copy set.
    for d in 386 arm arm64 mips mips64 power power64 riscv riscv64 sparc acme unix; do
      if [ ! -e "/$d" ] && [ -d "${harveySrc}/$d" ]; then
        copy_tree_files "${harveySrc}/$d" "/$d"
      fi
    done
    export HARVEY=

    # plan9port's ed writes its scratch file at the absolute
    # /var/tmp/eXXXXX (the 6l/7l mkcname and the compiler mkenam
    # generators run `ed`); the proot root carries no /var.
    mkdir -p /var/tmp
    # upas/misc's install copies gone.msg/gone.fishing into /mail/lib;
    # build/mkdirs lays out the other mail spools but not this one.
    mkdir -p /mail/lib

    # plan9port 9l's autolib execs bare `nm` on HOST ELF objects; the
    # hostbin carries Harvey's ken-format nm, so shadow it (the S4 /
    # subset-build shim).
    SHIM=/tmp/tree-amd64-shim
    mkdir -p "$SHIM"
    ln -sf "${pkgs.binutils}/bin/nm" "$SHIM/nm"
    # Plan 9's default path ends in `.`, and rc's `.` builtin / mk's bare
    # command lookup resolve cwd-local generator scripts (the tier-4
    # mkenam/lex generators) through $path, never the cwd.  The coverage
    # venue appends it for exactly this class; keep it LAST so a cwd file
    # can never shadow a toolchain command.
    export PATH="$SHIM:$PATH:."

    # pcc (the APE/gs class) executes its pipeline by ABSOLUTE path.
    # The coverage venue's proven set; /bin is the proot root's own dir.
    for t in cpp 6c 6l 6a awk; do
      [ -x "$HOSTBIN/$t" ] && ln -sfn "$HOSTBIN/$t" "/bin/$t"
    done

    export objtype=amd64 cputype=unix FORCERCFORMK=1
    export INCLUDE="/amd64/include /sys/include"

    mkdir -p /amd64/bin /amd64/lib /out/build/x86_64/out /out/build/x86_64/smoke-out

    # ------------------------------------------------------------------
    # The work lists: the pinned data, never a copied literal.
    # ------------------------------------------------------------------
    extract_tier() {
      gawk '!/^[ \t]*#/ && NF {print $1}' "/build/x86_64/tiers/$1.txt" > "/tmp/$1.entries"
      got=$(sha256sum "/tmp/$1.entries" | cut -d' ' -f1)
      [ "$got" = "$2" ] || {
        echo "TREE-AMD64-FAIL $1 list changed at this pin: sha256 $got != $2" >&2
        exit 1
      }
      n=$(wc -l < "/tmp/$1.entries" | tr -d ' ')
      echo "TREE-AMD64-LIST $1 entries=$n sha256=$got"
    }

    extract_tier tier0-libs ${tier0ListSha256}
    extract_tier tier1-cmds ${tier1ListSha256}
    extract_tier tier2-services ${tier2ListSha256}
    extract_tier tier3-gui ${tier3ListSha256}
    extract_tier tier4-tail ${tier4ListSha256}

    line_sha() {
      printf '%s\n' "$1" | sha256sum | cut -d' ' -f1
    }

    stage_line=$(grep -m1 '^STAGE_SRCS=' /build/x86_64/stage-build.sh) \
      || fail "no STAGE_SRCS= line in stage-build.sh"
    [ "$(line_sha "$stage_line")" = "${stageSrcsLineSha256}" ] \
      || fail "STAGE_SRCS changed at this pin: sha256 $(line_sha "$stage_line") != ${stageSrcsLineSha256} (line: $stage_line)"
    STAGE_SRCS=$(printf '%s' "$stage_line" | gawk '{ sub(/^STAGE_SRCS="/, ""); sub(/"$/, ""); print }')

    chain_libs_line=$(grep -m1 '^CHAIN_LIBS=' /build/x86_64/stage-build.sh) \
      || fail "no CHAIN_LIBS= line in stage-build.sh"
    [ "$(line_sha "$chain_libs_line")" = "${chainLibsLineSha256}" ] \
      || fail "CHAIN_LIBS changed at this pin: sha256 $(line_sha "$chain_libs_line") != ${chainLibsLineSha256}"
    CHAIN_LIBS=$(printf '%s' "$chain_libs_line" | gawk '{ sub(/^CHAIN_LIBS="/, ""); sub(/"$/, ""); print }')

    chain_line=$(grep -m1 '^for f in boot init rc paqfs' /build/x86_64/stage-build.sh) \
      || fail "no chain-binaries loop in stage-build.sh"
    [ "$(line_sha "$chain_line")" = "${chainListLineSha256}" ] \
      || fail "the stage-build chain list changed at this pin: sha256 $(line_sha "$chain_line") != ${chainListLineSha256}"
    chain_words=$(printf '%s' "$chain_line" | gawk '{ sub(/^for f in /, ""); sub(/; do$/, ""); print }')

    fpprobe_line=$(grep -m1 '^for f in fpprobe.c' /build/x86_64/stage-build.sh) \
      || fail "no fpprobe source loop in stage-build.sh"
    [ "$(line_sha "$fpprobe_line")" = "${fpprobeLineSha256}" ] \
      || fail "the fpprobe source list changed at this pin: sha256 $(line_sha "$fpprobe_line") != ${fpprobeLineSha256}"
    FPPROBE_SRCS=$(printf '%s' "$fpprobe_line" | gawk '{ sub(/^for f in /, ""); sub(/; do$/, ""); print }')

    sweep_line=$(grep -m1 '^for cmd in ls cat echo ps rm test' /build/x86_64/stage-build.sh) \
      || fail "no chain-sweep loop in stage-build.sh"
    [ "$(line_sha "$sweep_line")" = "${chainSweepLineSha256}" ] \
      || fail "the chain-sweep list changed at this pin: sha256 $(line_sha "$sweep_line") != ${chainSweepLineSha256}"
    SWEEP_CMDS=$(printf '%s' "$sweep_line" | gawk '{ sub(/^for cmd in /, ""); sub(/; do$/, ""); print }')

    gawk '/^echo "== net cmd: /{ sub(/^echo "== net cmd: /,""); sub(/ .*/,""); print }' \
      /build/x86_64/subset-build.sh > /tmp/net.entries
    got=$(sha256sum /tmp/net.entries | cut -d' ' -f1)
    [ "$got" = "${netLabelsSha256}" ] \
      || fail "the net-section labels changed at this pin: sha256 $got != ${netLabelsSha256}"
    NET_CMDS=$(tr '\n' ' ' < /tmp/net.entries)

    gawk '/^echo "== desktop cmd: /{ sub(/^echo "== desktop cmd: /,""); sub(/ .*/,""); print }' \
      /build/x86_64/subset-build.sh > /tmp/desktop.entries
    got=$(sha256sum /tmp/desktop.entries | cut -d' ' -f1)
    [ "$got" = "${desktopLabelsSha256}" ] \
      || fail "the desktop-helper labels changed at this pin: sha256 $got != ${desktopLabelsSha256}"
    DESKTOP_CMDS=$(tr '\n' ' ' < /tmp/desktop.entries)

    helpers_line=$(grep -m1 'for h in netblast tcpblast tcpecho' /build/x86_64/subset-build.sh) \
      || fail "no net-helper loop in subset-build.sh"
    [ "$(line_sha "$helpers_line")" = "${netHelpersLineSha256}" ] \
      || fail "the net-helper list changed at this pin: sha256 $(line_sha "$helpers_line") != ${netHelpersLineSha256}"
    NET_HELPERS=$(printf '%s' "$helpers_line" | gawk '{ sub(/^[ \t]*for h in /, ""); sub(/; do$/, ""); print }')

    echo "TREE-AMD64-STAGE-SRCS=$STAGE_SRCS"
    echo "TREE-AMD64-CHAIN-LIBS=$CHAIN_LIBS"
    echo "TREE-AMD64-CHAIN=$chain_words"
    echo "TREE-AMD64-SWEEP=$SWEEP_CMDS"
    echo "TREE-AMD64-NET=$NET_CMDS"
    echo "TREE-AMD64-DESKTOP=$DESKTOP_CMDS"
    echo "TREE-AMD64-NET-HELPERS=$NET_HELPERS"

    # ------------------------------------------------------------------
    # Every installed artifact must be a Plan 9 amd64 a.out (S_MAGIC
    # 0x8A97, big-endian: 00 00 8a 97).
    # ------------------------------------------------------------------
    check_magic() {
      f=$1
      what=$2
      [ -s "$f" ] || fail "$what: no artifact $f"
      m=$(od -An -tx1 -N4 "$f" | tr -d ' \n')
      [ "$m" = "00008a97" ] || fail "$what: bad magic '$m' (want 00008a97) at $f"
    }

    # A mass-tier dir install may also install rc scripts (#!...) and
    # disk's touch-created empty ksync placeholder — both are pinned
    # Harvey outputs, not binaries (see the SUMMARY's deviation note).
    check_output() {
      f=$1
      m4=$(od -An -tx1 -N4 "$f" | tr -d ' \n')
      m2=$(od -An -tx1 -N2 "$f" | tr -d ' \n')
      case "$m4" in
      00008a97) : ;;
      *)
        # a pinned rc/cpu script (any #! shebang) or disk's ksync
        if [ "$m2" = "2321" ]; then
          :
        elif [ ! -s "$f" ] && [ "$f" = "/amd64/bin/disk/ksync" ]; then
          :
        else
          fail "non-binary output $f (first bytes '$m4')"
        fi
        ;;
      esac
    }

    # ------------------------------------------------------------------
    # Tier 0: the library allowlist, with the D-07 fresh-archive rule
    # (rm -f the target .a before mk install — ar vu never prunes).
    # ------------------------------------------------------------------
    build_tier0() {
      T=/tmp/tier0-libs.entries
      N=$(wc -l < "$T" | tr -d ' ')
      start=$SECONDS
      # The pin's explicit, idempotent libc bootstrap before the loop
      # (everything links the archive).
      rm -f /amd64/lib/libc.a
      ( cd /sys/src/libc && objtype=amd64 mk install ) > /tmp/tier0-libc.log 2>&1 \
        || dump_fail "tier0 libc bootstrap" /tmp/tier0-libc.log
      [ -s /amd64/lib/libc.a ] || fail "tier0 libc: no /amd64/lib/libc.a"
      k=0
      while read -r d; do
        [ -n "$d" ] || continue
        case "$d" in
        liblex) lib=libl.a ;;
        *) lib="lib''${d#lib}.a" ;;
        esac
        log=/tmp/tier0-$d.log
        rm -f "/amd64/lib/$lib"
        if ( cd "/sys/src/$d" && objtype=amd64 mk install ) > "$log" 2>&1 \
           && [ -s "/amd64/lib/$lib" ]; then
          k=$((k+1))
        else
          dump_fail "tier0 $d" "$log"
        fi
      done < "$T"
      echo "TREE-AMD64-SECONDS tier0=$((SECONDS-start))"
      echo "TREE-AMD64 tier0 built=$k/$N"
      [ "$k" = "$N" ] || fail "tier0 built $k of $N"
    }

    # ------------------------------------------------------------------
    # Tier 1: the command allowlist, dispatched per entry form exactly
    # as subset-build.sh does (driver-only boot/init, container leaves,
    # dir installs, link-only singles with the driver's cp harvest).
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
          ( cd /sys/src/9k/boot && objtype=amd64 mk -f mkfile.amd64 ) > "$log" 2>&1 || ok=0
          if [ "$ok" = 1 ]; then
            cp /sys/src/9k/boot/boot /out/build/x86_64/out/boot >> "$log" 2>&1 || ok=0
          fi
          ( cd /sys/src/9k/boot && objtype=amd64 mk -f mkfile.amd64 clean ) > /dev/null 2>&1 || true
          art=/out/build/x86_64/out/boot
          ;;
        stage/init)
          ( cd /sys/src/cmd && objtype=amd64 mk init.install ) > "$log" 2>&1 || ok=0
          if [ "$ok" = 1 ]; then
            cp /amd64/init /out/build/x86_64/out/init >> "$log" 2>&1 || ok=0
          fi
          art=/out/build/x86_64/out/init
          ;;
        cmd/*:*)
          dir=''${entry%%:*}
          tgt=''${entry##*:}
          ( cd "/sys/src/$dir" && objtype=amd64 mk "$tgt.install" ) > "$log" 2>&1 || ok=0
          art="/amd64/bin/''${dir#cmd/}"
          if [ "$ok" = 1 ] && [ "$tgt" = paqfs ]; then
            cp /amd64/bin/paqfs /out/build/x86_64/out/paqfs >> "$log" 2>&1 || ok=0
          fi
          ;;
        cmd/*)
          if [ -d "/sys/src/$entry" ]; then
            ( cd "/sys/src/$entry" && objtype=amd64 mk install ) > "$log" 2>&1 || ok=0
            art="/amd64/bin/''${entry#cmd/}"
          else
            name=''${entry#cmd/}
            ( cd /sys/src/cmd && objtype=amd64 mk "6.$name" ) > "$log" 2>&1 || ok=0
            if [ "$ok" = 1 ]; then
              cp "/sys/src/cmd/6.$name" "/amd64/bin/$name" >> "$log" 2>&1 || ok=0
            fi
            rm -f "/sys/src/cmd/6.$name" "/sys/src/cmd/$name.6"
            art="/amd64/bin/$name"
          fi
          ;;
        *)
          ok=0
          echo "entry form not understood: $entry" > "$log"
          ;;
        esac
        [ "$ok" = 1 ] || dump_fail "tier1 $entry" "$log"
        check_magic "$art" "tier1 $entry"
        k=$((k+1))
      done < "$T"
      echo "TREE-AMD64-SECONDS tier1=$((SECONDS-start))"
      echo "TREE-AMD64 tier1 built=$k/$N"
      [ "$k" = "$N" ] || fail "tier1 built $k of $N"
    }

    # ------------------------------------------------------------------
    # Tiers 2-4: the per-entry fresh-build discipline (the 06-03 rule):
    # mk clean before every build, then install/link-only harvest.  The
    # new /amd64/bin files are found by a before/after snapshot (robust
    # against same-second mtimes) and every one must be a pinned output.
    # ------------------------------------------------------------------
    build_mass_tier() {
      n=$1
      T=$2
      N=$(wc -l < "$T" | tr -d ' ')
      start=$SECONDS
      k=0
      while read -r entry; do
        [ -n "$entry" ] || continue
        log="/tmp/tier$n-$(printf '%s' "$entry" | tr '/:' '__').log"
        case "$entry" in
        cmd/cc)
          # the shared frontend: tier evidence is the cc.a6 archive the
          # 6c build links (subset-build's own special case)
          rm -f /sys/src/cmd/cc/cc.a6
          ( cd /sys/src/cmd/cc && objtype=amd64 mk install ) > "$log" 2>&1 \
            || dump_fail "tier$n $entry" "$log"
          [ -s /sys/src/cmd/cc/cc.a6 ] \
            || dump_fail "tier$n $entry (no cc.a6)" "$log"
          ;;
        cmd/*:*)
          dir=''${entry%%:*}
          tgt=''${entry##*:}
          ( cd "/sys/src/$dir" && objtype=amd64 mk "$tgt.clean" ) > "$log" 2>&1 \
            || dump_fail "tier$n $entry (clean)" "$log"
          rm -f "/amd64/bin/$tgt"
          ( cd "/sys/src/$dir" && objtype=amd64 mk "6.$tgt" ) >> "$log" 2>&1 \
            || dump_fail "tier$n $entry (link)" "$log"
          cp "/sys/src/$dir/6.$tgt" "/amd64/bin/$tgt" >> "$log" 2>&1 \
            || dump_fail "tier$n $entry (harvest)" "$log"
          rm -f "/sys/src/$dir/6.$tgt" "/sys/src/$dir/$tgt.6"
          check_magic "/amd64/bin/$tgt" "tier$n $entry"
          ;;
        cmd/*)
          if [ -d "/sys/src/$entry" ]; then
            ( cd "/sys/src/$entry" && objtype=amd64 mk clean ) > "$log" 2>&1 \
              || dump_fail "tier$n $entry (clean)" "$log"
            find /amd64/bin -type f | sort > /tmp/bin-before
            ( cd "/sys/src/$entry" && objtype=amd64 mk install ) >> "$log" 2>&1 \
              || dump_fail "tier$n $entry" "$log"
            find /amd64/bin -type f | sort > /tmp/bin-after
            comm -13 /tmp/bin-before /tmp/bin-after > /tmp/bin-new
            found=0
            while read -r f; do
              [ -n "$f" ] || continue
              check_output "$f"
              found=$((found+1))
            done < /tmp/bin-new
            if [ "$found" = 0 ] && [ "$entry" != cmd/ld ] && [ "$entry" != cmd/go ]; then
              # Two pinned no-ops: cmd/ld's mkfile recurses its whole
              # rule set to `#`, and cmd/go's install is guarded on an
              # absolute /bin/go (the sandbox has none, so the pin's own
              # arm prints "No go, skipping").  Every other empty install
              # is a lost artifact, not a pass.
              dump_fail "tier$n $entry (install produced no artifact)" "$log"
            fi
          else
            name=''${entry#cmd/}
            base=''${name##*/}
            if [ "$name" = "$base" ]; then
              srcdir=/sys/src/cmd
            else
              srcdir="/sys/src/cmd/''${name%/*}"
            fi
            rm -f "$srcdir/6.$base" "$srcdir/$base.6" "/amd64/bin/$name"
            ( cd "$srcdir" && objtype=amd64 mk "6.$base" ) > "$log" 2>&1 \
              || dump_fail "tier$n $entry (link)" "$log"
            cp "$srcdir/6.$base" "/amd64/bin/$name" >> "$log" 2>&1 \
              || dump_fail "tier$n $entry (harvest)" "$log"
            rm -f "$srcdir/6.$base" "$srcdir/$base.6"
            check_magic "/amd64/bin/$name" "tier$n $entry"
          fi
          ;;
        *)
          dump_fail "tier$n $entry (entry form not understood)" /dev/null
          ;;
        esac
        k=$((k+1))
      done < "$T"
      echo "TREE-AMD64-SECONDS tier$n=$((SECONDS-start))"
      echo "TREE-AMD64 tier$n built=$k/$N"
      [ "$k" = "$N" ] || fail "tier$n built $k of $N"
    }

    # ------------------------------------------------------------------
    # The ape/lib substrate: the pcc/APE class's /amd64/lib/ape link
    # closure (libap and friends).  The coverage sweep's PASS B builds
    # it before the dir universe for exactly this reason; the tier-4
    # APE commands (compress, gs, postscript, lp, ...) link against it.
    # ------------------------------------------------------------------
    build_ape_libs() {
      start=$SECONDS
      echo "== ape/lib substrate (the pcc/APE class's /amd64/lib/ape closure)"
      ( cd /sys/src/ape/lib && objtype=amd64 mk install ) > /tmp/ape-libs.log 2>&1 \
        || dump_fail "ape/lib" /tmp/ape-libs.log
      [ -s /amd64/lib/ape/libap.a ] || fail "ape/lib: no /amd64/lib/ape/libap.a"
      echo "TREE-AMD64-SECONDS ape-libs=$((SECONDS-start))"
      echo "TREE-AMD64 ape-libs built=ok"
    }

    SECONDS=0
    build_tier0
    build_tier1
    build_ape_libs
    build_mass_tier 2 /tmp/tier2-services.entries
    build_mass_tier 3 /tmp/tier3-gui.entries
    build_mass_tier 4 /tmp/tier4-tail.entries

    # ------------------------------------------------------------------
    # The net section (subset-build.sh's 14-09 payload): the ip trio,
    # dd, bind, the desktop-session helpers, and the three 6c helpers
    # into build/x86_64/out.
    # ------------------------------------------------------------------
    build_net() {
      start=$SECONDS
      k=0
      n=0
      mkdir -p /amd64/bin/ip

      echo "== net cmd: ping"
      n=$((n+1))
      ( cd /sys/src/cmd/ip && objtype=amd64 mk 6.ping ) > /tmp/net-ping.log 2>&1 \
        || dump_fail "net ping" /tmp/net-ping.log
      cp /sys/src/cmd/ip/6.ping /amd64/bin/ip/ping >> /tmp/net-ping.log 2>&1 \
        || dump_fail "net ping harvest" /tmp/net-ping.log
      rm -f /sys/src/cmd/ip/6.ping /sys/src/cmd/ip/ping.6
      check_magic /amd64/bin/ip/ping "net ping"
      k=$((k+1))

      echo "== net cmd: udpecho"
      n=$((n+1))
      ( cd /sys/src/cmd/ip && objtype=amd64 mk 6.udpecho ) > /tmp/net-udpecho.log 2>&1 \
        || dump_fail "net udpecho" /tmp/net-udpecho.log
      cp /sys/src/cmd/ip/6.udpecho /amd64/bin/ip/udpecho >> /tmp/net-udpecho.log 2>&1 \
        || dump_fail "net udpecho harvest" /tmp/net-udpecho.log
      rm -f /sys/src/cmd/ip/6.udpecho /sys/src/cmd/ip/udpecho.6
      check_magic /amd64/bin/ip/udpecho "net udpecho"
      k=$((k+1))

      echo "== net cmd: dd"
      n=$((n+1))
      ( cd /sys/src/cmd && objtype=amd64 mk 6.dd ) > /tmp/net-dd.log 2>&1 \
        || dump_fail "net dd" /tmp/net-dd.log
      cp /sys/src/cmd/6.dd /amd64/bin/dd >> /tmp/net-dd.log 2>&1 \
        || dump_fail "net dd harvest" /tmp/net-dd.log
      rm -f /sys/src/cmd/6.dd /sys/src/cmd/dd.6
      check_magic /amd64/bin/dd "net dd"
      k=$((k+1))

      # 14-08's bind: the k10t1 donor embed's member, and the loopback
      # bring-up every net/auth gate calls.
      echo "== net cmd: bind"
      n=$((n+1))
      ( cd /sys/src/cmd && objtype=amd64 mk 6.bind ) > /tmp/net-bind.log 2>&1 \
        || dump_fail "net bind" /tmp/net-bind.log
      cp /sys/src/cmd/6.bind /amd64/bin/bind >> /tmp/net-bind.log 2>&1 \
        || dump_fail "net bind harvest" /tmp/net-bind.log
      rm -f /sys/src/cmd/6.bind /sys/src/cmd/bind.6
      check_magic /amd64/bin/bind "net bind"
      k=$((k+1))

      # 19-10's desktop-session helpers (the cpu image's profile legs).
      echo "== desktop cmd: syscall"
      n=$((n+1))
      rm -f /amd64/bin/syscall
      ( cd /sys/src/cmd/syscall && objtype=amd64 mk install ) > /tmp/desk-syscall.log 2>&1 \
        || dump_fail "desktop syscall" /tmp/desk-syscall.log
      check_magic /amd64/bin/syscall "desktop syscall"
      k=$((k+1))

      echo "== desktop cmd: awd"
      n=$((n+1))
      ( cd /sys/src/cmd && objtype=amd64 mk 6.awd ) > /tmp/desk-awd.log 2>&1 \
        || dump_fail "desktop awd" /tmp/desk-awd.log
      cp /sys/src/cmd/6.awd /amd64/bin/awd >> /tmp/desk-awd.log 2>&1 \
        || dump_fail "desktop awd harvest" /tmp/desk-awd.log
      rm -f /sys/src/cmd/6.awd /sys/src/cmd/awd.6
      check_magic /amd64/bin/awd "desktop awd"
      k=$((k+1))

      for h in $NET_HELPERS; do
        n=$((n+1))
        echo "== net helper: $h"
        ( cd /build/x86_64 && objtype=amd64 6c -FTVw "$h.c" \
          && objtype=amd64 6l -o "/out/build/x86_64/out/$h" "$h.6" \
          && rm -f "$h.6" ) > "/tmp/net-$h.log" 2>&1 \
          || dump_fail "net helper $h" "/tmp/net-$h.log"
        check_magic "/out/build/x86_64/out/$h" "net helper $h"
        k=$((k+1))
      done

      echo "TREE-AMD64-SECONDS net=$((SECONDS-start))"
      echo "TREE-AMD64 net built=$k/$n"
      [ "$k" = "$n" ] || fail "net built $k of $n"
    }

    build_net

    # ------------------------------------------------------------------
    # The stage-build programs (build/x86_64/stage-build.sh): the
    # marker sources (STAGE_SRCS) plus fpprobe into smoke-out/, the
    # init alias, the chain (boot/init/rc/paqfs + the sweep set) into
    # out/, exactly the relative paths mkimage.sh reads.
    # ------------------------------------------------------------------
    build_stage() {
      start=$SECONDS
      k=0
      n=0

      for lib in $CHAIN_LIBS; do
        ( cd "/sys/src/lib$lib" && objtype=amd64 mk install ) > "/tmp/chain-lib-$lib.log" 2>&1 \
          || dump_fail "stage chain lib $lib" "/tmp/chain-lib-$lib.log"
        [ -s "/amd64/lib/lib$lib.a" ] || fail "stage chain lib $lib: no archive"
      done

      for f in $STAGE_SRCS; do
        n=$((n+1))
        [ -f "/build/x86_64/$f.c" ] || fail "stage $f: build/x86_64/$f.c missing"
        ( cd /build/x86_64 && objtype=amd64 6c -FTVw "$f.c" \
          && objtype=amd64 6l -o "/out/build/x86_64/smoke-out/$f" "$f.6" \
          && rm -f "$f.6" ) > "/tmp/stage-$f.log" 2>&1 \
          || dump_fail "stage $f" "/tmp/stage-$f.log"
        check_magic "/out/build/x86_64/smoke-out/$f" "stage $f"
        k=$((k+1))
      done

      n=$((n+1))
      cp "/out/build/x86_64/smoke-out/smokesleep" "/out/build/x86_64/smoke-out/init" \
        || fail "stage init alias"
      check_magic "/out/build/x86_64/smoke-out/init" "stage init"
      k=$((k+1))

      for f in $FPPROBE_SRCS; do
        [ -f "/build/x86_64/$f" ] || fail "stage fpprobe: build/x86_64/$f missing"
      done
      n=$((n+1))
      ( cd /build/x86_64 && objtype=amd64 6c -FTVw fpprobe.c \
        && objtype=amd64 6a fpprobeasm.s \
        && objtype=amd64 6l -o "/out/build/x86_64/smoke-out/fpprobe" fpprobe.6 fpprobeasm.6 \
        && rm -f fpprobe.6 fpprobeasm.6 ) > /tmp/stage-fpprobe.log 2>&1 \
        || dump_fail "stage fpprobe" /tmp/stage-fpprobe.log
      check_magic "/out/build/x86_64/smoke-out/fpprobe" "stage fpprobe"
      k=$((k+1))

      # the boot chain (14-06)
      n=$((n+1))
      ( cd /sys/src/9k/boot && objtype=amd64 mk -f mkfile.amd64 ) > /tmp/stage-boot.log 2>&1 \
        || dump_fail "stage chain boot" /tmp/stage-boot.log
      cp /sys/src/9k/boot/boot /out/build/x86_64/out/boot >> /tmp/stage-boot.log 2>&1 \
        || dump_fail "stage chain boot harvest" /tmp/stage-boot.log
      ( cd /sys/src/9k/boot && objtype=amd64 mk -f mkfile.amd64 clean ) > /dev/null 2>&1 || true
      check_magic /out/build/x86_64/out/boot "stage chain boot"
      k=$((k+1))

      n=$((n+1))
      ( cd /sys/src/cmd && objtype=amd64 mk init.install ) > /tmp/stage-init.log 2>&1 \
        || dump_fail "stage chain init" /tmp/stage-init.log
      cp /amd64/init /out/build/x86_64/out/init >> /tmp/stage-init.log 2>&1 \
        || dump_fail "stage chain init harvest" /tmp/stage-init.log
      check_magic /out/build/x86_64/out/init "stage chain init"
      k=$((k+1))

      n=$((n+1))
      ( cd /sys/src/cmd/rc && objtype=amd64 mk install ) > /tmp/stage-rc.log 2>&1 \
        || dump_fail "stage chain rc" /tmp/stage-rc.log
      cp /amd64/bin/rc /out/build/x86_64/out/rc >> /tmp/stage-rc.log 2>&1 \
        || dump_fail "stage chain rc harvest" /tmp/stage-rc.log
      check_magic /out/build/x86_64/out/rc "stage chain rc"
      k=$((k+1))

      n=$((n+1))
      ( cd /sys/src/cmd/paqfs && objtype=amd64 mk paqfs.install ) > /tmp/stage-paqfs.log 2>&1 \
        || dump_fail "stage chain paqfs" /tmp/stage-paqfs.log
      cp /amd64/bin/paqfs /out/build/x86_64/out/paqfs >> /tmp/stage-paqfs.log 2>&1 \
        || dump_fail "stage chain paqfs harvest" /tmp/stage-paqfs.log
      check_magic /out/build/x86_64/out/paqfs "stage chain paqfs"
      k=$((k+1))

      # the sweep set (the composed smoke's gate vocabulary)
      for cmd in $SWEEP_CMDS; do
        n=$((n+1))
        ( cd /sys/src/cmd && objtype=amd64 mk "6.$cmd" ) > "/tmp/stage-cmd-$cmd.log" 2>&1 \
          || dump_fail "stage sweep $cmd" "/tmp/stage-cmd-$cmd.log"
        cp "/sys/src/cmd/6.$cmd" "/out/build/x86_64/out/$cmd" >> "/tmp/stage-cmd-$cmd.log" 2>&1 \
          || dump_fail "stage sweep $cmd harvest" "/tmp/stage-cmd-$cmd.log"
        rm -f "/sys/src/cmd/6.$cmd" "/sys/src/cmd/$cmd.6"
        check_magic "/out/build/x86_64/out/$cmd" "stage sweep $cmd"
        k=$((k+1))
      done

      echo "TREE-AMD64-SECONDS stage=$((SECONDS-start))"
      echo "TREE-AMD64 stage built=$k/$n"
      [ "$k" = "$n" ] || fail "stage built $k of $n"
    }

    build_stage

    # ------------------------------------------------------------------
    # The auth image's host-side programs (mkimage §14i2/§14i3; plan
    # 01-16): the bake builds hsvctcpecho and hauthprobe with its HOST
    # 6c/6l from tree sources.  r9's assembler copies tree artifacts (it
    # never compiles), so the tree builds them here with the same host
    # toolchain and flags as the bake, after the chain libs — authprobe
    # links libauthsrv via its header's autolib record.  The reference
    # member list proves the bytes.
    # ------------------------------------------------------------------
    build_auth_helpers() {
      start=$SECONDS
      k=0
      n=0
      # The reference bake builds these in a writable tree copy at
      # /build/harvey, and 6c/6l's output depends on the build's path
      # strings: the reference's exact bytes need /build/harvey as cwd
      # and the /build/mkimage-x64.XXXXXX/root-auth staging shape (probe:
      # identical shas only with that shape).  Build in a copy at that
      # path so the tree artifact equals the reference member list byte
      # for byte.
      mkdir -p /build/harvey/sys /build/harvey/amd64 /build/harvey/build/x86_64
      cp -R /sys/include /build/harvey/sys/include
      cp -R /amd64/include /build/harvey/amd64/include
      cp -R /amd64/lib /build/harvey/amd64/lib
      cp /build/x86_64/svctcpecho.c /build/x86_64/authprobe.c /build/harvey/build/x86_64/
      for h in svctcpecho authprobe; do
        n=$((n+1))
        echo "== auth helper: $h"
        [ -x "$HOSTBIN/6c" ] && [ -x "$HOSTBIN/6l" ] \
          || fail "auth helper $h: hostbin 6c/6l missing"
        [ -f "/build/harvey/build/x86_64/$h.c" ] || fail "auth helper $h: source missing"
        mkdir -p "/build/mkimage-x64.r9auth/root-auth/amd64/bin"
        ( cd /build/harvey && \
          "$HOSTBIN/6c" -FTVw -o "/build/mkimage-x64.r9auth/root-auth/.$h.6" \
            -Isys/include -Iamd64/include "build/x86_64/$h.c" && \
          "$HOSTBIN/6l" -o "/build/mkimage-x64.r9auth/root-auth/amd64/bin/h$h" \
            -L "/build/harvey/amd64/lib" "/build/mkimage-x64.r9auth/root-auth/.$h.6" && \
          rm -f "/build/mkimage-x64.r9auth/root-auth/.$h.6" ) > "/tmp/auth-$h.log" 2>&1 \
          || dump_fail "auth helper $h" "/tmp/auth-$h.log"
        cp "/build/mkimage-x64.r9auth/root-auth/amd64/bin/h$h" "/out/build/x86_64/out/$h" \
          || fail "auth helper $h: harvest failed"
        check_magic "/out/build/x86_64/out/$h" "auth helper $h"
        k=$((k+1))
      done
      echo "TREE-AMD64-SECONDS auth-helpers=$((SECONDS-start))"
      echo "TREE-AMD64 auth-helpers built=$k/$n"
      [ "$k" = "$n" ] || fail "auth helpers built $k of $n"
    }

    build_auth_helpers

    echo "TREE-AMD64-BUILD-OK"
  '';

  installScript = ''
    set -eu

    fail() {
      echo "TREE-AMD64-FAIL $*" >&2
      exit 1
    }

    TREE=/out

    # ---- the built userland -------------------------------------------
    mkdir -p "$TREE/amd64/bin" "$TREE/amd64/lib" "$TREE/amd64/include"
    copy_tree_files /amd64/bin "$TREE/amd64/bin"
    copy_tree_files /amd64/lib "$TREE/amd64/lib"
    copy_tree_files /amd64/include "$TREE/amd64/include"
    [ -s /amd64/init ] || fail "no /amd64/init after the tier build"
    cp /amd64/init "$TREE/amd64/init"

    # build/x86_64/out and smoke-out are written directly under /out by
    # the guest build (the chain + marker/fixture programs at the exact
    # relative paths mkimage.sh reads).
    [ -s "$TREE/build/x86_64/out/boot" ] || fail "out/boot missing"
    [ -s "$TREE/build/x86_64/smoke-out/init" ] || fail "smoke-out/init missing"

    # ---- the source-side install list (D-19) ---------------------------
    # Every entry below is named by a read in build/x86_64/mkimage.sh at
    # the pin (sections 1-15 and the churn variant), or by a moved gate's
    # fixture read.  The path is installed read-only at the same
    # tree-relative location.  rc/bin is never installed wholesale (the
    # case-colliding set); no 9k kernel or r9 artifact rides here.
    #
    #   build/x86_64/root-tree/          mkimage §1/8b/9c/10c/11e/12b/13b/14e/15d
    #                                    (the committed skeleton: cpurc, tcp17010,
    #                                    netaddr.rc, netip-*.rc, local.template,
    #                                    churn-cats.rc, profile, termrc, namespace)
    #   build/x86_64/tiers/              mkimage §9b/11a/11b/14a/14b/15a/15b
    #                                    (the five tier lists the bakes compute from)
    #   build/x86_64/authdom-donors/     mkimage §14d (the vendored generator sources)
    #   sys/include/                     mkimage §11k/§14i2/§14i3 (the host-side
    #                                    6c builds' -Isys/include headers; the
    #                                    reference bake is the first consumer)
    #   lib/font/bit/                    mkimage §P1/§11e2/§15e (the whole DESK-03 font tree)
    #   lib/ndb/                         the ndb data home (plan 01-14's named set)
    #   rc/lib/                          mkimage §8e/9g/10g/11n/12b/14j/15l (rcmain verbatim)
    #   adm/timezone/                    mkimage §8e/9g/10g/11n/12b/14j/15l (init's cpenv data)
    src_dirs="
    build/x86_64/root-tree
    build/x86_64/tiers
    build/x86_64/authdom-donors
    sys/include
    lib/font/bit
    lib/ndb
    rc/lib
    adm/timezone
    "
    for d in $src_dirs; do
      [ -d "${harveySrc}/$d" ] || fail "source-side dir ${harveySrc}/$d missing"
      copy_tree_files "${harveySrc}/$d" "$TREE/$d"
    done

    # Single-file fixtures, each with the read site that names it.
    #   mkimage.sh                          the reference bake itself (plan 01-15's
    #                                       D-29 reference); the pin's own script
    #   tier1-sweep.rc / tier-termrc        mkimage §9f/§9g (the tier image's sweep + override)
    #   services-sweep.rc / services-termrc mkimage §11a/§11l (the services image)
    #   auth-sweep.rc / auth-termrc         mkimage §14a/§14j (the auth image)
    #   bootfs-payload.txt                  mkimage §P (the DESK-03 payload manifest)
    #   nm-input.6                          mkimage §9f (the committed nm fixture)
    #   secstore-obj.txt                    mkimage §14f4 (the secstore round-trip payload)
    #   authdomain-gen.c / authdomain.example  mkimage §14d (the host generator's source + interface shape)
    #   svctcpecho.c / authprobe.c          mkimage §11k/§14i2/§14i3 (host-side 6c/6l builds)
    #   sys/src/cmd/mkdir.c                 mkimage §11k (the hmkdir host-side build)
    #   sys/src/cmd/auth/secstore/mkfile    mkimage §14b (the auth-suite resolver's
    #                                       multi-word TARG authority: secstore
    #                                       secstored secuser aescbc — without it
    #                                       the computed set loses hsecstored)
    #   usr/glenda/lib/plumbing             mkimage §15l (plumber's default rules)
    #   sys/lib/plumb/basic, fileaddr       mkimage §15l (plumber's include targets)
    src_files="
    build/x86_64/mkimage.sh
    build/x86_64/tier1-sweep.rc
    build/x86_64/tier-termrc
    build/x86_64/services-sweep.rc
    build/x86_64/services-termrc
    build/x86_64/auth-sweep.rc
    build/x86_64/auth-termrc
    build/x86_64/bootfs-payload.txt
    build/x86_64/nm-input.6
    build/x86_64/secstore-obj.txt
    build/x86_64/authdomain-gen.c
    build/x86_64/authdomain.example
    build/x86_64/svctcpecho.c
    build/x86_64/authprobe.c
    sys/src/cmd/mkdir.c
    sys/src/cmd/auth/secstore/mkfile
    usr/glenda/lib/plumbing
    sys/lib/plumb/basic
    sys/lib/plumb/fileaddr
    "
    for f in $src_files; do
      [ -f "${harveySrc}/$f" ] || fail "source-side file ${harveySrc}/$f missing"
      mkdir -p "$TREE/$(dirname "$f")"
      cp "${harveySrc}/$f" "$TREE/$f"
    done

    # ---- the final-tree scan ------------------------------------------
    # Binaries are amd64 a.out; the long-tail tier's rc scripts (#!...) and
    # disk's touch-created empty ksync are the only other pinned outputs.
    nb=0
    ns=0
    ne=0
    for f in $(find "$TREE/amd64/bin" -type f); do
      m4=$(od -An -tx1 -N4 "$f" | tr -d ' \n')
      m2=$(od -An -tx1 -N2 "$f" | tr -d ' \n')
      if [ "$m4" = "00008a97" ]; then
        nb=$((nb+1))
      elif [ "$m2" = "2321" ]; then
        ns=$((ns+1))
      elif [ ! -s "$f" ] && [ "$f" = "$TREE/amd64/bin/disk/ksync" ]; then
        ne=$((ne+1))
      else
        fail "non-binary file under amd64/bin: $f (first bytes '$m4')"
      fi
    done
    total=$(find "$TREE/amd64/bin" -type f | wc -l | tr -d ' ')
    [ "$((nb+ns+ne))" = "$total" ] || fail "bin scan mismatch: $nb+$ns+$ne != $total"
    echo "TREE-AMD64-BIN-SCAN binaries=$nb scripts=$ns empty=$ne total=$total"

    # ---- the stamp the resolver checks --------------------------------
    cat > "$TREE/HARVEY-TREE" <<'HARVEY_TREE_STAMP'
    rev=${rev}
    narHash=${narHash}
    arch=amd64
    system=${pkgs.stdenv.hostPlatform.system}
    HARVEY_TREE_STAMP

    echo "TREE-AMD64-INSTALL-OK members=$(find "$TREE" -type f | wc -l | tr -d ' ')"
  '';
}
