#!/bin/sh
# S6 build-probe.sh (D-17): build one no_std Rust probe with plain rustc
# from r9's locked devshell, link it flat with rust-lld at the arch's
# INITTEXT, then wrap and check the Plan 9 a.out header.
#
# Usage: sh build-probe.sh <amd64|arm64>
#
# This is deliberately NOT part of the cargo workspace (D-18): rustc is
# invoked directly, with no crates and no Cargo.toml involvement. Outputs
# live under spikes/s6/work/ (gitignored).
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
arch=${1:-}

case "$arch" in
amd64)
	target=x86_64-unknown-none
	src="$here/probe-amd64.rs"
	ld="$here/probe-amd64.ld"
	;;
arm64)
	target=aarch64-unknown-none
	src="$here/probe-arm64.rs"
	ld="$here/probe-arm64.ld"
	;;
*)
	echo "usage: sh $0 <amd64|arm64>" >&2
	exit 2
	;;
esac

command -v nix >/dev/null 2>&1 || {
	echo "error: nix is not on PATH (the probe builds inside r9's devshell)" >&2
	exit 1
}

root=$(git -C "$here" rev-parse --show-toplevel)
mkdir -p "$here/work"

# x86_64-unknown-none defaults to static PIE; relocation-model=static is
# load-bearing (a PIE link would leave relocations the flat image cannot
# carry). aarch64-unknown-none is static already; the flag is harmless.
(cd "$root" && nix develop -c rustc \
	--edition 2021 \
	--crate-type bin \
	--target "$target" \
	-C panic=abort \
	-C opt-level=s \
	-C relocation-model=static \
	-C "link-arg=-T$ld" \
	-C link-arg=--oformat=binary \
	-o "$here/work/probe-$arch.flat" \
	"$src")

python3 -I "$here/aout.py" wrap --arch "$arch" \
	"$here/work/probe-$arch.flat" "$here/work/probe-$arch.aout"
python3 -I "$here/aout.py" check --arch "$arch" "$here/work/probe-$arch.aout"
