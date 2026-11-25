# S2 — Harvey's 23 case-colliding paths through the Mac's Linux builder

Question: does the Mac builder path the owner approved (lima VM `harvey`,
aarch64-linux) preserve Harvey's 23 case-colliding source paths end to end —
into the builder from a Mac-fetched flake input, from a builder-side
fixed-output fetch, and back into the Mac's case-insensitive store — so that
D-04's single build path can be trusted with the real Harvey tree?
(01-RESEARCH.md Pitfall 3, Assumption A2; D-04; D-23.)

BUILDER-VENUE: lima (owner, 2026-11-22; carried from `S1-FINDINGS.md`)

The colliding set is `s2/collisions.txt`: exactly 23 `<blob-sha1> <path>`
lines in 11 case groups, generated read-only from
`git -C /Users/neilskinner/code/harvey ls-tree -r bd7a6982a…` by
case-folding every path and keeping the duplicates. `rc/bin/Kill` and
`rc/bin/kill` share one blob (they prove names, not contents); the other 21
paths have distinct blobs.

## Source mode 1 — the Mac-fetched flake input

VERDICT s2-input-lima: PASS
EVIDENCE s2-input-lima: s2/evidence/casecheck-builder.log

`s2-casecheck-input` takes the flake input `harvey` (the same
`github:requaos/harvey/bd7a6982a…` source s1 pins). On the Mac the input is
the case-hacked store path (`kill~nix~case~hack~1`, 12 hacked names in
`/nix/store/v8djqydzaggdvl2w7v3z1svxj0p3zlla-source`); nix ships it to the
builder as a NAR. On the builder all 23 paths are regular files under their
exact names, every git blob sha1 matches, `find` sees zero
`~nix~case~hack~` names, and `ls rc/bin` lists both `kill` and `Kill`. The
derivation writes `report.txt` ending `S2-CASECHECK-ALL-OK` and copies the
23 files in tree layout into `$out/collide/`.

## Source mode 2 — a builder-side fixed-output fetch

VERDICT s2-fod-lima: PASS
EVIDENCE s2-fod-lima: s2/evidence/casecheck-builder.log

`s2-casecheck-fod` is `pkgs.fetchFromGitHub` at the same rev with the flake
lock's narHash as its SRI hash
(`sha256-N6VjnVI69pcxSCpQ9BUxrlpTqnp9LgjKfGcpO3F6Ma0=`). The builder fetched
and unpacked the source itself; the hash was accepted with no mismatch, all
23 blob sha1s match, no case-hack name appears, and the report ends
`S2-CASECHECK-ALL-OK`. This is the route that never sends the source through
APFS at all.

## Copy-back into the Mac store

VERDICT s2-copyback-lima: FAIL
EVIDENCE s2-copyback-lima: s2/evidence/casecheck-builder.log

As specified — `nix copy --from ssh-ng://lima-harvey <out>` **as the owner's
user** — the import is refused before any file is written:

```
error: cannot add path '/nix/store/b03ch0dk2gdc1wn2dx7lbva3k0niqb6l-s2-casecheck-input-0'
       because it lacks a signature by a trusted key
```

The Mac's store has `require-sigs = true` and `trusted-users = root`;
`--no-check-sigs` and `--option require-sigs false` are both ignored
("restricted setting … you are not a trusted user"), and `nix-store
--export | nix-store --import` is refused the same way. Completing this step
needs a trusted user (root) or a Mac store-config change, and this plan is
forbidden from changing the Mac beyond the approved builder venue. No
signature key was added and no `/etc/nix` file was touched.

VERDICT s2-copyback-chroot-lima: PASS
EVIDENCE s2-copyback-chroot-lima: s2/evidence/casecheck-builder.log

The case-collision question itself was answered without touching the system
store: the same two outputs were copied into a **local chroot store on the
Mac's APFS data volume** with `--option require-sigs false` (a direct local
store, where the option is honoured). Both imports were accepted, and on the
Mac side:

- `nix store verify --no-trust --store <chroot> <out>` is clean for both
  outputs — the NAR hash recomputed from the case-hacked on-disk names
  matches the builder's store path;
- the 12 on-disk names that need the hack are exactly the expected ones
  (`kill~nix~case~hack~1`, `Hx~nix~case~hack~1`, …; `lh` is
  `…~hack~2`, as on the input store);
- all 23 blob sha1s re-check from the Mac-side files, mapped back to the
  real names, 23/23 for each output.

Method note (measured): on case-insensitive APFS a plain `[ -f "$dir/Hx" ]`
resolves the **case twin** `HX` and silently reads the wrong bytes. The
Mac-side re-check must enumerate directory entries and match the exact
byte-name first, then the `~nix~case~hack~` suffix. The first pass of this
check used `[ -f ]` and reported 12/23 BAD; the exact-name pass reports
23/23 OK. Any pass-2 consumer that opens a colliding path by name on the Mac
inherits this trap (which is why tree outputs must not contain them).

## A9 — a context-discarded store path in the devshell

Question: can a devshell export a Linux derivation's store path with the
string context discarded without building it, and does that path match the
path the later build produces? D-05 (per-arch trees, built on first use) and
D-21 (`R9_HARVEY_DIR_AMD64` / `R9_HARVEY_DIR_ARM64`) depend on it.

VERDICT s2-lazy-devshell: PASS
EVIDENCE s2-lazy-devshell: s2/evidence/lazy-devshell.log

`s2-lazy-target` is an aarch64-linux `runCommand` with a name unique to this
spike. `devShells.aarch64-darwin.lazy` exports
`S2_LAZY_PATH` from
`builtins.unsafeDiscardStringContext s2-lazy-target.outPath`. The four
recorded observations all hold:

1. The evaluated outPath is
   `/nix/store/wd8i5jwcaggdafpvpg8rv1ri15yrjajb-s2-lazy-target-a9-0`, and
   `nix path-info` on the Mac exits 1 ("path … is not valid") before any
   build.
2. `nix develop -L path:$S/s2#lazy -c printenv S2_LAZY_PATH` prints that
   path; only `nix-shell-env.drv` is built. Afterwards the path is still
   invalid on the Mac **and** on the lima builder — no build was sent.
3. The printed path equals the evaluated outPath
   (`S2-LAZY-EVAL-MATCH=yes`).
4. Building `packages.aarch64-linux.s2-lazy-target` through lima produces
   the same path (`S2-LAZY-BUILD-MATCH=yes`) and it is valid on the builder
   afterwards (`nix path-info --store ssh-ng://lima-harvey` exits 0).

## Exact commands

```
S=.planning/phases/01-stage-p0-cross-build-harness/spikes
export NIX_SSHOPTS="-F /Users/neilskinner/.lima/harvey/ssh.config -oControlPath=none"

nix flake lock "path:$S/s2"

nix build "path:$S/s2#packages.aarch64-linux.s2-casecheck-input" \
  --store ssh-ng://lima-harvey --eval-store auto -L --print-out-paths
nix build "path:$S/s2#packages.aarch64-linux.s2-casecheck-fod" \
  --store ssh-ng://lima-harvey --eval-store auto -L --print-out-paths

# as specified, refused (signature policy):
nix copy --from ssh-ng://lima-harvey /nix/store/b03ch0dk2gdc1wn2dx7lbva3k0niqb6l-s2-casecheck-input-0

# the measured route (local chroot store on the same APFS data volume):
nix copy --from ssh-ng://lima-harvey --to "$S/s2/work/mac-store" \
  --option require-sigs false /nix/store/b03ch0dk2gdc1wn2dx7lbva3k0niqb6l-s2-casecheck-input-0
nix copy --from ssh-ng://lima-harvey --to "$S/s2/work/mac-store" \
  --option require-sigs false /nix/store/sydxc6ylbmgvml1y9hadk68vd8k5pdkv-s2-casecheck-fod-0
nix store verify --no-trust --store "$S/s2/work/mac-store" \
  /nix/store/b03ch0dk2gdc1wn2dx7lbva3k0niqb6l-s2-casecheck-input-0
nix store verify --no-trust --store "$S/s2/work/mac-store" \
  /nix/store/sydxc6ylbmgvml1y9hadk68vd8k5pdkv-s2-casecheck-fod-0
# then exact-name enumeration + `git hash-object` per collisions.txt line

# A9 (lazy devshell):
nix eval --raw "path:$S/s2#packages.aarch64-linux.s2-lazy-target.outPath"
nix path-info /nix/store/wd8i5jwcaggdafpvpg8rv1ri15yrjajb-s2-lazy-target-a9-0   # exit 1
nix develop -L "path:$S/s2#lazy" -c printenv S2_LAZY_PATH
nix path-info /nix/store/wd8i5jwcaggdafpvpg8rv1ri15yrjajb-s2-lazy-target-a9-0   # still exit 1
nix build "path:$S/s2#packages.aarch64-linux.s2-lazy-target" \
  --store ssh-ng://lima-harvey --eval-store auto -L --print-out-paths
nix path-info --store ssh-ng://lima-harvey \
  /nix/store/wd8i5jwcaggdafpvpg8rv1ri15yrjajb-s2-lazy-target-a9-0               # exit 0
```

## Assumption

ASSUMPTION A2: CONFIRMED - the lima NAR transport preserves the 23
case-colliding names in both source modes, the Mac-side APFS store applies
its reversible case hack on import, `nix store verify --no-trust` is clean,
and all 23 blob sha1s re-check on the Mac. The qualification is store
policy, not case handling: importing into the **system** store as the
owner's user needs a trusted user (or a remote-builder configuration where
the local daemon itself imports the build result).
ASSUMPTION A9: CONFIRMED - `builtins.unsafeDiscardStringContext` on an
aarch64-linux derivation's outPath exports a plain string from an
aarch64-darwin devshell; entering the shell builds nothing on the builder,
and the later build produces exactly that path (all four A9 observations
in `s2/evidence/lazy-devshell.log`).

## What this means for planning pass 2

- **Either mechanism is safe for the real tree; prefer the builder-side
  fixed-output fetch.** The input mode works and is what `nix build` does by
  default, but it ships 257 MB of source through the Mac store and depends
  on the case-hack reversal. A fixed-output `fetchFromGitHub` on the builder
  (the `s2-casecheck-fod` shape, pinned by rev and SRI hash) never round-trips
  the source through APFS and is the more robust D-04 source supply for the
  pass-2 tree derivation.
- **The copy-back step must not use `nix copy --from` as the owner's user on
  this Mac.** Either configure the lima VM as a remote **builder** so the
  local daemon imports the realised output itself (no signature check for a
  build result), or have the owner make the Mac store trust the relevant
  user/key. Pass 2's "one build path" must state which; switching the Mac's
  store config is an owner decision, not an executor step.
- **Tree outputs read by xtask on the Mac must not contain colliding
  names.** That is stricter than "the store can hold them": the store can,
  but a Mac-side lookup by real path resolves the wrong twin. Copy only what
  the images and ureg checks consume, and assert the installed set has no
  `~nix~case~hack~` name (S1-FINDINGS already requires this; S2 shows the
  failure mode is a silently wrong file, not an error).
- **The HARN-07 devshell supply can be lazy as D-05 requires.** A
  context-discarded `outPath` in the darwin `shellHook` is a plain string:
  `nix develop` built only its env derivation, left the target invalid in
  both stores, and the later `nix build` produced the identical path. The
  pass-2 resolver can use the "exported path exists? use it : print and run
  `nix build <flake>#harvey-tree-<arch>`" contract with no eager build; the
  only open item is the Mac store import policy above, not laziness.
