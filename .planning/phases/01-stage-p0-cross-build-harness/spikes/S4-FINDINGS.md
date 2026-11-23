# S4 — Harvey's amd64 tier 0 + tier 1 on an aarch64-linux host

Question: does Harvey's amd64 tier build — tier 0, the 27 libraries in
`build/x86_64/tiers/tier0-libs.txt`, then tier 1, the 40 commands in
`tier1-cmds.txt` — run on an aarch64-linux host in the S1 venue, so that
Mac-built amd64 trees can come from that builder?
(01-RESEARCH.md Pitfall 4 builder-system paragraph, Pitfall 5; Assumptions
A3, A4, A8; D-04; D-05.)

BUILDER-VENUE: lima (owner, 2026-11-22; carried from `S1-FINDINGS.md`)

The work list is the pinned data, read at the pin, and the dispatch follows
`build/x86_64/subset-build.sh`'s per-tier mk loop and ordering (tier-0
fresh-archive rule; tier-1 `cmd/<single>` link-only + harvest,
`cmd/<dir>` install, `cmd/<container>/<dir>`, `cmd/<dir>:<target>`,
`9k/boot` and `stage/init` driver-only). Unlike the pin's driver, every
entry is built independently so one failure cannot hide the rest.
`subset-build.sh` itself is not called: it assumes a NixOS
`/run/current-system`, a `flock` on the checkout, and `nix run .#setup`.

## Tier 0

VERDICT s4-tier0-amd64-lima: PASS
EVIDENCE s4-tier0-amd64-lima: s4/evidence/tier-builder.log

27/27 libraries built in **6 s** (`S4-TIER0 built=27/27`), including the
`libdraw`/`libmemdraw`/`libframe` GUI closure and `libmach` (which needs the
per-arch `ureg.h` includes; see the venue note below). Each archive is
non-empty and `sha256`-recorded in `tier-report.txt`, and the fresh-archive
rule (`rm -f` before `mk install`) is applied per entry.

## Tier 1

VERDICT s4-tier1-amd64-lima: PASS
EVIDENCE s4-tier1-amd64-lima: s4/evidence/tier-builder.log

40/40 entries built in **19 s** (`S4-TIER1 built=40/40`). Every tier-1
artifact's first four bytes are `00 00 8a 97` (`S_MAGIC` amd64) — 40
`magic=00008a97` lines in the report, including `rc`, `awk` (the host
`maketab` generator path), `nm` (libmach), `ipconfig`, `factotum`, `boot`,
`init` and `paqfs`. The two driver-only entries and the container leaves
install at their pin paths (`/amd64/bin/boot`, `/amd64/bin/init`,
`/amd64/bin/ip/ipconfig`, `/amd64/bin/auth/factotum`,
`/amd64/bin/paqfs`).

Output layout: `$out/amd64/lib/` has the 27 `.a` files; `$out/amd64/bin/`
has 40 files; `$out/tier-report.txt` has the per-entry BUILT lines, the
per-tier wall times and the two `S4-TIERn built=n/N` lines.

## The two venue ports this needed

The first attempt with the unmodified S1 venue built **26/27 and 38/40**:
`9k/boot` failed `membername: No such file or directory`, and `libhtml` /
`cmd/dc` failed `6c ... out of memory` at `build.c:1821` / `dc.c:899`.
Two venue ports close both gaps; both are properties of Harvey's own
userns venue, not changes to the build recipe:

1. **`/rc/bin` on PATH.** `rc/bin/membername` is an rc script with a
   `#!/bin/rc` shebang; the boot mkfile calls it by name. The pin's userns
   venue puts `/rc/bin` on PATH (`PATH="$INNER_BIN:/rc/bin:$PATH"`); the S1
   proot venue does not. With it, `9k/boot` builds (control run:
   `S4-TIER1 built=39/40`, only `dc` left).
2. **`INCLUDE="/amd64/include /sys/include"`.** This is a semantic no-op —
   the same two directories the compiler's default branch adds — but it
   avoids a latent 6c heap-layout failure under proot. Controlled A–F
   experiment inside the same venue (`s4/evidence/tier-builder.log`):
   A (venue default) → `build.c:1821 out of memory`, rc=1; E (`dc.c`
   default) → `dc.c:899 out of memory`, rc=1; B (`ccroot` at the store
   path), C (store source), D (explicit `INCLUDE`) and F (`dc.c` store
   includes) → rc=0. The same two compiles also succeed natively without
   proot and outside the venue. So the host and the pinned 6c are capable;
   the default include-code path under proot is what trips. The mechanism
   is a compiler-internal heap/`sbrk` interaction (the pin's `gethunk`
   merge requires contiguous `sbrk`), not a source or host limit.

The controlled no-`INCLUDE` run (`S4-TIER0 26/27`, `S4-TIER1 39/40`) and the
final run (`27/27`, `40/40`) differ only by that one environment variable.

## Copies and provenance

`s4/venue.nix` and `s4/hostbin.nix` are byte copies of the S1 files; the S1
files are untouched:

```
ffe670f3b4932c2b790d9c8200bb129822fa15da98cfb01c96d80d97cb8421ce  s1/venue.nix
ffe670f3b4932c2b790d9c8200bb129822fa15da98cfb01c96d80d97cb8421ce  s4/venue.nix
4c30dc935b1e8679156d8b96564b2172403c9b8594cda6b7d479f973fdbf5a8c  s1/hostbin.nix
4c30dc935b1e8679156d8b96564b2172403c9b8594cda6b7d479f973fdbf5a8c  s4/hostbin.nix
```

`s4-hostbin` is the same derivation as S1's hostbin (same pname and inputs),
so the builder reuses the toolchain S1 already built.

## Informational: `echo` compared with the checkout copy

Built `echo` sha256
`3bef6e2234f9910699355f0e898762d68ed5705eb7c9075c7502edc0b4b00bf7`;
`/Users/neilskinner/code/harvey/amd64/bin/echo` sha256
`bbc0b906fe154e90e9e331d8bf5a967a1d7697f735b384b3d56d523e7d7d322d`.
**Different** — recorded as informational only (that checkout copy's
provenance is unknown; the file was read, never written).

## Exact commands

```
S=.planning/phases/01-stage-p0-cross-build-harness/spikes
export NIX_SSHOPTS="-F /Users/neilskinner/.lima/harvey/ssh.config -oControlPath=none"

nix flake lock "path:$S/s4"
nix eval --raw "path:$S/s4#packages.x86_64-linux.s4-tier-amd64.drvPath"   # evaluates

nix build "path:$S/s4#packages.aarch64-linux.s4-tier-amd64" \
  --store ssh-ng://lima-harvey --eval-store auto -L --print-out-paths
```

## Assumptions

ASSUMPTION A3: CONFIRMED - the amd64 tier content is host-independent, so
the aarch64-linux builder is the right system for Mac builds: every one of
the 27 libraries and 40 tier-1 entries built there, with no x86_64 builder
and no emulation.
ASSUMPTION A4: CONFIRMED - the amd64 tier build runs on an aarch64-linux
host, with the two venue ports above (`/rc/bin` on PATH and the explicit
`INCLUDE`). Without them the S1 proot venue fails on `9k/boot` and on the
two large 6c compiles; the controlled experiments show the host and the
pinned compiler are fine and the failures are venue-default behaviour.
ASSUMPTION A8: REFUTED - the tier-0 library build is 6 s and the tier-1
command build is 19 s on this builder (25 s for the whole tier 0 + tier 1
set), not "tens of minutes per arch". The full userland build's cost is
elsewhere (tiers 2-4 and the image bake), not in this tier pair.

## What this means for planning pass 2

- **Mac-built amd64 trees can come from the aarch64-linux builder (D-04,
  D-05).** The aarch64 host produces every amd64 tier-0 and tier-1 artifact,
  all with the right `S_MAGIC`; no x86_64 builder and no Rosetta/emulation is
  needed. The pass-2 tree derivation must carry the two venue ports measured
  here — `/rc/bin` on PATH and the explicit `INCLUDE` list — and should state
  them where the venue is defined, not in each build step.
- **Budget seconds, not minutes.** Ten tiers of work should be estimated from
  the measured 25 s for tiers 0-1, with the image bake and the larger tiers
  2-4 as the remaining cost. `lib.fileset` filtering still pays for itself by
  avoiding full rebuilds on unrelated pin bumps, but do not size timeouts or
  builder memory for "tens of minutes" on this pair.
- **Keep the tier data as the single work list.** The derivation should read
  `build/x86_64/tiers/*.txt` at the pin (never a copied list); the acceptance
  vocabulary in `tier.nix` (`tier1-cmds.txt`) is the same data. For the real
  tree, a failing entry should fail the derivation (the spike's
  build-everything-and-report mode is for evidence, not production).
- **The tier outputs are clean for the Mac.** None of the 27 libraries or 40
  commands contains a case-colliding name, so the S2 rule (tree outputs must
  not contain colliding names) is satisfied by this tier set; the S1 venue
  copies whole subtrees, so the pass-2 derivation must still install only the
  tier entries, not the source subtrees.
