# tier.nix — S4: Harvey's amd64 tier 0 + tier 1 in the S1 venue.
#
# The venue is the byte-identical copy of spikes/s1/venue.nix (proot mode:
# unshare is refused by this builder, S1-FINDINGS). The work list is the
# pinned data at build/x86_64/tiers/{tier0-libs,tier1-cmds}.txt and the
# dispatch follows build/x86_64/subset-build.sh's per-tier mk loop and
# ordering at bd7a698, with two deliberate differences:
#
#   * every entry is built INDEPENDENTLY: a failure is recorded and the loop
#     continues, so one broken entry cannot hide the rest (the pin's driver
#     aborts on the first failure);
#   * the artifacts are copied to $out instead of harvested into the
#     checkout: $out/amd64/lib/*.a, $out/amd64/bin/**, and
#     $out/tier-report.txt with BUILT/FAILED, per-tier wall time and the
#     first four bytes of every tier-1 binary (must be 00 00 8a 97).
#
# The venue binds only sys/amd64/arm64/rc/lib/out. The Plan 9 mkfiles also
# reach a few read-only tree dirs by absolute path, so the guest stages them
# from the pinned source through the venue's /nix/store bind:
#
#   * the per-arch dirs (libmach's 4.c..v.c include /386/include/ureg.h,
#     /mips/include/ureg.h, ... by absolute path);
#   * /unix (its mkfile is the host-toolchain cc/ld rule set that awk's
#     `$cputype.maketab` host generator builds with);
#   * /build (the $HARVEY/build/x86_64/tiers filters read by cmd/mkfile,
#     cmd/ip/mkfile, cmd/auth/mkfile and cmd/auth/factotum/mkfile).
#
# HARVEY is exported empty, so the filters' $HARVEY/build/... resolves to
# /build/..., matching the root layout. The hostbin is first on PATH for
# the Plan 9 tools; subset-build.sh's nm shim (binutils nm for 9l's autolib
# on host ELF objects) is prepended ahead of it, exactly as the pin does.
{
  pkgs,
  harveySrc,
  hostbin,
}:

import ./venue.nix {
  inherit pkgs harveySrc;
  name = "s4-tier-amd64";
  transcriptFile = "tier-builder.txt";
  mode = "proot";
  extraPath = [
    hostbin
    pkgs.binutils
  ];
  guestScript = ''
    HARVEY_STORE="${harveySrc}"

    for d in 386 arm arm64 mips mips64 power power64 riscv riscv64 sparc acme unix build; do
      if [ ! -e "/$d" ] && [ -d "$HARVEY_STORE/$d" ]; then
        cp -R "$HARVEY_STORE/$d" "/$d"
      fi
    done
    export HARVEY=

    HOSTBIN="${hostbin}/bin"
    SHIM=/tmp/s4-shims
    mkdir -p "$SHIM"
    ln -sf "${pkgs.binutils}/bin/nm" "$SHIM/nm"
    export PATH="$SHIM:$HOSTBIN:/rc/bin:$PATH"
    export objtype=amd64 cputype=unix FORCERCFORMK=1
    # Harvey's own userns venue puts /rc/bin on PATH (membername is an rc
    # script with a #!/bin/rc shebang; the boot mkfile calls it by name), so
    # the tier build needs it too. The compiler's include list is made
    # explicit because the default code path under proot hits a latent 6c
    # heap-layout failure on libhtml/build.c and cmd/dc.c ("out of memory");
    # INCLUDE is semantically the same list (". /amd64/include
    # /sys/include"). Both are measured in the S4 work logs and summarized
    # in S4-FINDINGS.md.
    export INCLUDE="/amd64/include /sys/include"

    mkdir -p /amd64/bin /amd64/lib
    OUT=/out
    mkdir -p "$OUT/amd64/bin" "$OUT/amd64/lib"
    report="$OUT/tier-report.txt"
    : > "$report"

    errline() {
      e=$(grep -m1 -iE 'error|fail|cannot|undefined|no such|missing|not found' "$1" || true)
      [ -n "$e" ] || e=$(grep -v '^$' "$1" | tail -n 1 || true)
      printf '%s' "$e"
    }

    T0=/build/x86_64/tiers/tier0-libs.txt
    T1=/build/x86_64/tiers/tier1-cmds.txt
    [ -f "$T0" ] || { echo "S4-FAIL tier0 data missing: $T0"; exit 1; }
    [ -f "$T1" ] || { echo "S4-FAIL tier1 data missing: $T1"; exit 1; }
    awk '!/^[ \t]*#/ && NF {print $1}' "$T0" > /tmp/tier0.entries
    awk '!/^[ \t]*#/ && NF {print $1}' "$T1" > /tmp/tier1.entries
    echo "S4-TIER0-ENTRIES=$(wc -l < /tmp/tier0.entries | tr -d ' ')"
    echo "S4-TIER1-ENTRIES=$(wc -l < /tmp/tier1.entries | tr -d ' ')"

    # The pin's explicit, idempotent libc bootstrap before the tier-0 loop.
    echo "== tier0 bootstrap: libc"
    ( cd /sys/src/libc && objtype=amd64 mk install ) > /tmp/t0-libc-bootstrap.log 2>&1 || true

    t0_built=0
    t0_start=$SECONDS
    while read -r d; do
      [ -n "$d" ] || continue
      libname=$(printf '%s' "$d" | sed 's/^lib//')
      lib="lib$libname.a"
      log=/tmp/t0-$d.log
      rm -f "/amd64/lib/$lib"
      if ( cd "/sys/src/$d" && objtype=amd64 mk install ) > "$log" 2>&1 && [ -s "/amd64/lib/$lib" ]; then
        t0_built=$((t0_built+1))
        printf 'TIER0 %s BUILT sha256=%s\n' "$d" "$(sha256sum "/amd64/lib/$lib" | cut -d' ' -f1)" >> "$report"
      else
        printf 'TIER0 %s FAILED %s\n' "$d" "$(errline "$log")" >> "$report"
      fi
    done < /tmp/tier0.entries
    t0_secs=$((SECONDS-t0_start))

    t1_built=0
    t1_start=$SECONDS
    while read -r entry; do
      [ -n "$entry" ] || continue
      log=/tmp/t1-$(printf '%s' "$entry" | tr '/:' '__').log
      art=
      ok=1
      case "$entry" in
      9k/boot)
        ( cd /sys/src/9k/boot && objtype=amd64 mk -f mkfile.amd64 ) > "$log" 2>&1 || ok=0
        if [ "$ok" -eq 1 ]; then
          cp /sys/src/9k/boot/boot /amd64/bin/boot >> "$log" 2>&1 || ok=0
        fi
        ( cd /sys/src/9k/boot && objtype=amd64 mk -f mkfile.amd64 clean ) > /dev/null 2>&1 || true
        art=/amd64/bin/boot
        ;;
      stage/init)
        ( cd /sys/src/cmd && objtype=amd64 mk init.install ) > "$log" 2>&1 || ok=0
        if [ "$ok" -eq 1 ]; then
          cp /amd64/init /amd64/bin/init >> "$log" 2>&1 || ok=0
        fi
        art=/amd64/bin/init
        ;;
      cmd/*:*)
        dir=$(printf '%s' "$entry" | cut -d: -f1)
        tgt=$(printf '%s' "$entry" | cut -d: -f2)
        ( cd "/sys/src/$dir" && objtype=amd64 mk "$tgt.install" ) > "$log" 2>&1 || ok=0
        art="/amd64/bin/$(printf '%s' "$dir" | sed 's|^cmd/||')"
        ;;
      cmd/*)
        if [ -d "/sys/src/$entry" ]; then
          ( cd "/sys/src/$entry" && objtype=amd64 mk install ) > "$log" 2>&1 || ok=0
          art="/amd64/bin/$(printf '%s' "$entry" | sed 's|^cmd/||')"
        else
          name=$(printf '%s' "$entry" | sed 's|^cmd/||')
          ( cd /sys/src/cmd && objtype=amd64 mk "6.$name" ) > "$log" 2>&1 || ok=0
          if [ "$ok" -eq 1 ]; then
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
      if [ "$ok" -eq 1 ] && [ -n "$art" ] && [ -s "$art" ]; then
        magic=$(od -An -tx1 -N4 "$art" | tr -d ' \n')
        if [ "$magic" = "00008a97" ]; then
          t1_built=$((t1_built+1))
          printf 'TIER1 %s BUILT sha256=%s magic=%s\n' "$entry" "$(sha256sum "$art" | cut -d' ' -f1)" "$magic" >> "$report"
        else
          printf 'TIER1 %s FAILED bad-magic=%s want=00008a97 artifact=%s\n' "$entry" "$magic" "$art" >> "$report"
        fi
      else
        printf 'TIER1 %s FAILED %s\n' "$entry" "$(errline "$log")" >> "$report"
      fi
    done < /tmp/tier1.entries
    t1_secs=$((SECONDS-t1_start))

    cp -a /amd64/bin/. "$OUT/amd64/bin/" 2>/dev/null || true
    cp -a /amd64/lib/. "$OUT/amd64/lib/" 2>/dev/null || true
    printf 'S4-TIER0-SECONDS=%s\n' "$t0_secs" >> "$report"
    printf 'S4-TIER1-SECONDS=%s\n' "$t1_secs" >> "$report"
    printf 'S4-TIER0 built=%s/27\n' "$t0_built" >> "$report"
    printf 'S4-TIER1 built=%s/40\n' "$t1_built" >> "$report"
    cat "$report"
  '';
}
