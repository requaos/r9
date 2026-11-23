# nix/harvey/tree-arm64.nix — the arm64 Harvey tree, smoke slice (HARN-07).
#
# The tracer's manual supply (a writable checkout's build/aarch64/smoke-out/
# and .build/bin/mkpaqfs) becomes a read-only store path: the arm64 libc,
# the eight smoke binaries and the two source-side files the arm64 path
# reads (the headers ureg-check parses and netaddr.rc the pcap seam reads).
# Plans 01-14 and 01-17 extend the same machinery to the full amd64 and
# arm64 trees.
#
# The source list is Harvey's own `STAGE_SRCS=` line read at build time from
# the pinned build/aarch64/stage-build.sh, with its sha256 asserted against
# the value below: a Harvey change to the list fails the next pin bump
# loudly instead of silently drifting (research Pitfall 5).
{
  pkgs,
  harveySrc,
  rev,
  narHash,
}:

let
  env = import ./build-env.nix { inherit pkgs harveySrc; };

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

    # The smoke source list is the pinned data, never a copied literal.
    stage_line=$(grep -m1 '^STAGE_SRCS=' /build/aarch64/stage-build.sh) || {
      echo "TREE-ARM64-FAIL no STAGE_SRCS= line in /build/aarch64/stage-build.sh" >&2
      exit 1
    }
    stage_hash=$(printf '%s\n' "$stage_line" | sha256sum | cut -d' ' -f1)
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

    mkdir -p /out/stage

    t0=$SECONDS
    echo "== arm64 libc (idempotent; everything staged links /arm64/lib/libc.a)"
    ( cd /sys/src/libc && objtype=arm64 mk install ) || {
      echo "TREE-ARM64-FAIL libc" >&2
      exit 1
    }
    [ -s /arm64/lib/libc.a ] || {
      echo "TREE-ARM64-FAIL no /arm64/lib/libc.a after the libc install" >&2
      exit 1
    }
    echo "TREE-ARM64-LIBC-BYTES=$(wc -c < /arm64/lib/libc.a | tr -d ' ')"

    echo "== smoke sources -> /out/stage"
    for f in $STAGE_SRCS; do
      ( cd /build/aarch64 && objtype=arm64 7c -FTVw "$f.c" && objtype=arm64 7l -o "/out/stage/$f" "$f.7" && rm -f "$f.7" ) || {
        echo "TREE-ARM64-FAIL compile $f" >&2
        exit 1
      }
    done
    t1=$SECONDS
    echo "TREE-ARM64-SECONDS=$((t1-t0))"
    echo "TREE-ARM64-BUILD-OK"
  '';

  installScript = ''
    set -eu

    TREE=/out
    DEST="$TREE/build/aarch64/smoke-out"
    mkdir -p "$DEST" "$TREE/arm64/include"

    for f in $STAGE_SRCS; do
      [ -s "/out/stage/$f" ] || {
        echo "TREE-ARM64-FAIL staged /out/stage/$f missing" >&2
        exit 1
      }
      cp "/out/stage/$f" "$DEST/$f"
    done
    # The committed ABI bisect binary rides the tree verbatim (as `hello`).
    cp /build/aarch64/hello.7.out "$DEST/hello"

    # Every staged binary must carry the arm64 a.out magic (R_MAGIC 0x8C47,
    # big-endian: 00 00 8c 47).
    for f in $STAGE_SRCS hello; do
      magic=$(od -An -tx1 -N4 "$DEST/$f" | tr -d ' \n')
      [ "$magic" = "00008c47" ] || {
        echo "TREE-ARM64-FAIL $f: bad magic $magic (want 00008c47)" >&2
        exit 1
      }
      echo "TREE-ARM64-RMAGIC $f $magic"
    done

    # The headers ureg-check parses (ureg.h at least; u.h rides along) and
    # the netaddr.rc the pcap seam reads.
    cp -R /arm64/include/. "$TREE/arm64/include/"
    cp /build/aarch64/netaddr.rc "$TREE/build/aarch64/netaddr.rc"

    chmod +x "$DEST"/*

    # The stamp the resolver checks (01-08's harvey.rs reads `arch=`, and
    # D-05's realiser compares the exported path).
    cat > "$TREE/HARVEY-TREE" <<'HARVEY_TREE_STAMP'
    rev=${rev}
    narHash=${narHash}
    arch=arm64
    system=${pkgs.stdenv.hostPlatform.system}
    HARVEY_TREE_STAMP

    rm -rf /out/stage
    echo "TREE-ARM64-INSTALL-OK members=$(ls "$DEST" | wc -l | tr -d ' ')"
  '';
}
