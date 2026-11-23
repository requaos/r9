# nix/harvey/authdomain-gen.nix — the host p9sk1 domain generator
# (D-16, T-01-48).
#
# Harvey's mkimage.sh (sections 14 and 15) invokes a host build of
# build/x86_64/authdomain-gen.c over the vendored donor algorithm files
# in build/x86_64/authdom-donors/ to create the per-bake p9sk1 domain.
# Plan 01-16 moves that bake-time step into r9's assembler, which names
# this derivation through the devshell's R9_AUTHDOMAIN_GEN.
#
# The build line is the committed authdomain-selftest.sh's own
# invocation (the 18-03 venue): plain host cc, -O2, the donor include
# dir, the generator plus the ten donor .c files.
#
# Threat T-01-48: this derivation packages the TOOL only.  No domain is
# generated inside the derivation and no key material is installed —
# the generator runs at bake time, on the host, into a private
# directory; the world-readable store carries only the executable.
{
  pkgs,
  harveySrc,
}:

pkgs.stdenv.mkDerivation {
  pname = "harvey-authdomain-gen";
  version = "0";

  dontUnpack = true;

  nativeBuildInputs = [ pkgs.stdenv.cc ];

  buildCommand = ''
    set -euo pipefail
    mkdir -p "$out/bin"

    # The donors are byte-verbatim Harvey sources with thin shim headers
    # (u.h/libc.h/authsrv.h/libsec.h/mp.h/os.h); the generator and every
    # donor .c compile together into one host binary.
    cc -O2 -Wno-unknown-pragmas \
      -I"${harveySrc}/build/x86_64/authdom-donors" \
      -o "$out/bin/authdomain-gen" \
      "${harveySrc}/build/x86_64/authdomain-gen.c" \
      "${harveySrc}/build/x86_64/authdom-donors/passtokey.c" \
      "${harveySrc}/build/x86_64/authdom-donors/nvcsum.c" \
      "${harveySrc}/build/x86_64/authdom-donors/crypt.c" \
      "${harveySrc}/build/x86_64/authdom-donors/des.c" \
      "${harveySrc}/build/x86_64/authdom-donors/desCBC.c" \
      "${harveySrc}/build/x86_64/authdom-donors/desmodes.c" \
      "${harveySrc}/build/x86_64/authdom-donors/sha1.c" \
      "${harveySrc}/build/x86_64/authdom-donors/sha1block.c" \
      "${harveySrc}/build/x86_64/authdom-donors/hmac.c" \
      "${harveySrc}/build/x86_64/authdom-donors/strecpy.c"

    # A compile-only proof: no generate/verify run happens here, so no
    # domain bytes can ever land in the store (T-01-48).
    test -x "$out/bin/authdomain-gen"
    echo "authdomain-gen: host tool installed (no domain generated)"
  '';
}
