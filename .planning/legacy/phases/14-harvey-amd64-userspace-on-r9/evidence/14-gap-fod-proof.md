# 14-VERIFICATION Gap Fix — the boot-iso FOD re-wire proof transcript

**Date:** 2026-11-09 (post-verification fix cycle)
**Gap:** 14-VERIFICATION.md gap 1 — r9's boot-iso FOD wired hash stale at the
phase-close pin e417660; the 14-EVIDENCE.md regression row claimed a green
that was not run at that rev.
**Fix commits:** r9 `4132e25` (the hash re-wire, pushed e417660..4132e25) →
harvey `f6622b35f` (lock-only pin advance, push-before-pin held).
**Discipline:** 13-06/14-07 — two INDEPENDENT derivations of the ISO, each
forcing a FRESH kernel build (a cache-hit `--rebuild` proves only the ISO
layer), must prove byte-identical BEFORE the hash is wired.

## 1. Gap reproduction (the verifier's measurement, re-run)

    $ cd /home/req/code/r9 && nix build .#boot-iso   # at r9@e417660, wired lEqthp7h…
    error: hash mismatch in fixed-output derivation '...r9-boot-iso-e417660.drv':
             specified: sha256-lEqthp7h0JdAN8rUIItI+3U6+EQB+tEqI0Djq6mUqZs=
                got:    sha256-lwbDSqYxYNG8ZIhjkDsziHlEjFM3JJOEYinl3vbKWaw=

The wired hash dated from r9@fe39d83 (14-07 Task 1); the phase's own
post-fe39d83 kernel commits (8a338a6 console RX, e5def72 SWAPGS cmpw,
e417660 user SSE + eager stack) changed the staged kernel ELF bytes — the
standing drift class.

## 2. The two-derivation byte-identity proof (non-FOD realizations)

Two independent derivations, each with a FRESH kernel build forced by a
version-override clone (versions `gapfix-proof-a` / `gapfix-proof-b` never
existed before, so the kernel derivations cannot cache-hit), driven through
`r9.lib.bootIso` with NO outputHash:

    $ nix build --no-link --print-out-paths --impure --expr '
        let f = builtins.getFlake "path:/home/req/code/r9";
            k = f.packages.x86_64-linux.kernel-x64.overrideAttrs
                 (_: { version = "gapfix-proof-a"; });   # then -b
        in f.lib.bootIso { system = "x86_64-linux";
                           kernel-elf = "${k}/kernel-x64.elf"; }'

Both logs show genuinely fresh kernel builds (not cache hits):

    A: building '/nix/store/ragn9ajr…-r9-kernel-x64-gapfix-proof-a.drv'
    B: building '/nix/store/h9yk8n6…-r9-kernel-x64-gapfix-proof-b.drv'
    A out: /nix/store/r8bvlhrwh1m9z3chxh3yjz2kxvmb376c-r9-boot-iso-wip
    B out: /nix/store/50jhb9c4pp6q0prvgi6w4l9gpvspp2ix-r9-boot-iso-wip

Byte-identity (both layers — the file hash AND `cmp`):

    $ sha256sum $OUTA/boot.iso $OUTB/boot.iso
    bd0281bbc00f96f0b7a615bdf23e17b0c69cc00059733fc64abb6304b74804e3  A/boot.iso
    bd0281bbc00f96f0b7a615bdf23e17b0c69cc00059733fc64abb6304b74804e3  B/boot.iso
    $ cmp $OUTA/boot.iso $OUTB/boot.iso   → BYTE-IDENTICAL

The NAR hash (what the FOD checks, outputHashMode=recursive) — measured on
BOTH proof outputs, agreeing, and matching the FOD's own mismatched got-value
(three-way agreement across one FOD and two non-FOD realizations):

    $ nix hash path $OUTA   → sha256-lwbDSqYxYNG8ZIhjkDsziHlEjFM3JJOEYinl3vbKWaw=
    $ nix hash path $OUTB   → sha256-lwbDSqYxYNG8ZIhjkDsziHlEjFM3JJOEYinl3vbKWaw=
    base16: 9706c34aa63160d1bc648863903b338879448c53372493846229e5def6ca59ac

## 3. The wiring (r9@4132e25) and green verification

`packages.boot-iso.outputHash` re-wired to
`sha256-lwbDSqYxYNG8ZIhjkDsziHlEjFM3JJOEYinl3vbKWaw=` with the provenance
comment updated in the 14-07 style (full hashes recorded in flake.nix):

    $ nix build .#boot-iso          → exit 0 (kernel rebuilt fresh, hash verified)
    $ nix build .#boot-iso --rebuild → exit 0 (outputs re-checked)

## 4. Impact scope (why no gate re-run was required)

No XEXEC gate consumes the wired-hash FOD — mechanically confirmed:

    $ grep -rn "boot-iso" flake.nix nix/*.sh   (harvey) → zero hits
    — the four composed x64 apps consume r9.packages.iso-builder
      (the exported script) + kernel-x64; r9's xtask boot-tests build
      their own dev ISO per invocation.

Regression battery proving the tree otherwise untouched (flake.nix was the
only delta; `git diff --stat` = 1 file):

    $ nix develop . -c cargo xtask test → 101 passed; 0 failed
    $ nix develop . -c cargo check      → Finished (cache-hit; pre-existing
                                          aarch64 warnings unchanged)

## 5. The pin advance (push-before-pin)

    r9 push: e417660..4132e25 main → main (origin/main..HEAD = 0 after)
    harvey:   nix flake update r9 → flake.lock only (rev + narHash +
              lastModified; 3+/3-), committed lock-only as f6622b35f
    sanity at the new pin: nix eval .#apps.x86_64-linux.boot-r9-x64.type = "app"

## Standing rule (the drift class, restated for every later phase)

ANY r9 kernel-byte commit re-stales the boot-iso FOD. The close discipline
is re-wire-then-pin: two-derivation proof → wire → `nix build .#boot-iso`
green → push r9 → advance the harvey pin (lock-only). Never record the FOD
green without running it at the pinned rev.
