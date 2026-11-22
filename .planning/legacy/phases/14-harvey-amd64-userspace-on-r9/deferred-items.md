
## 14-05 (Task 1): boot-iso FOD outputHash stale since 14-01 (pre-existing, out of scope)

`nix build .#boot-iso --rebuild` at r9 fails the wired Phase-13 hash
(dCSPPV…) — proven PRE-EXISTING at the pre-plan commit abee15c (fresh
build yields w6uDAH…, also ≠ specified). Every Phase-14 kernel commit
(14-01 onward) changed the kernel ELF bytes the FOD stages; the hash was
wired in 13-06 against the Phase-13 kernel and never re-wired. 14-05's
grub.cfg component is NOT the cause: the kernel-only emission is
byte-identical to Phase 13's (cmp-proven + pinned by an xtask unit
test); the FOD's sed staging keeps the loader-config component stable so
14-07's re-proof isolates kernel-byte drift. 14-07 owns the re-wire (its
plan: the parameterized builder + a fresh two-derivation byte-identity
proof + the new hash), per OQ-1's resolution.

## RESOLVED 14-07: boot-iso FOD outputHash re-wired (the stale-since-14-01 item above)

14-07 re-expressed packages.boot-iso through the parameterized
iso-builder and re-wired the hash after the refactor's own
byte-preservation proof plus fresh independent-derivation proofs: the
old 13-06 FOD body and the new builder, over FOUR independent fresh
kernel builds, produced the identical ISO (file sha256 cdcfa008…,
NAR 944aad86… = sha256-lEqthp7h…); `nix build .#boot-iso --rebuild`
re-verifies on every rebuild (r9@fe39d83, the kernel re-keyed to
q35shell for the compose).

## RESOLVED (14-VERIFICATION gap fix, 2026-11-09): the hash re-staled at the phase-close pin — the drift class is standing

The 14-07 re-wire (above) went stale AGAIN before the phase closed: the
phase's own post-fe39d83 kernel commits (8a338a6 console RX, e5def72
SWAPGS cmpw, e417660 user SSE + eager stack — the phase-close pin
itself) changed the staged kernel bytes, and neither re-wired the hash
nor recorded the staleness; the 14-EVIDENCE regression row then claimed
a green that was never run at the pinned rev (the verifier caught it:
`nix build .#boot-iso` exit 1 at e417660 — 14-VERIFICATION gap 1).
The gap fix re-wired it at r9@4132e25 with the same two-derivation
discipline (two independent fresh-kernel version-override clones through
the non-FOD lib.bootIso, byte-identical: file sha256 bd0281bb…, NAR
9706c34a… = sha256-lwbDSqYx…, three-way agreement with the FOD's own
measured got-value); `nix build .#boot-iso` + `--rebuild` green at the
new pin; r9 pushed e417660..4132e25, then the harvey lock-only pin
f6622b35f (proof transcript: evidence/14-gap-fod-proof.md; WINDOWS rows
56 + 60 closed).

STANDING RULE (every later phase that lands r9 kernel-byte commits):
ANY kernel-byte commit re-stales the boot-iso FOD.  The close discipline
is re-wire-then-pin: run the two-derivation fresh-kernel proof, wire the
hash, verify `nix build .#boot-iso` (+ `--rebuild`) green, push r9, THEN
advance the harvey pin (lock-only).  Never record the FOD green without
running it at the pinned rev.
