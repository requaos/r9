# Phase 18: In-Guest p9sk1 Auth Domain - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-13
**Phase:** 18-In-Guest p9sk1 Auth Domain
**Areas discussed:** Generator & pairing, r9 kernel substrate, Image & service topology, Domain content & probe

---

## Generator & pairing

### Key material provenance

| Option | Description | Selected |
|--------|-------------|----------|
| Committed demo set | Generator inputs committed in-tree as data; bake derives all four artifacts deterministically; FOD-friendly; DEMO is honest because material is public | |
| Fresh per bake | Real CSPRNG per bake; unique domain per image; breaks the boot-iso FOD hash; CI-hostile | |
| Committed seed + derivation | Committed seed stretched by committed derivation; deterministic but not plaintext; cosmetic security gain | |

**User's choice:** Other (freeform) — uncommitted file at the harvey repo root satisfying a committed interface; developers configure seed/passwords/keys without committing actual values to git history.
**Notes:** Follow-up resolved in plain text: when the file is absent (CI / clean checkout — Phase 19's INT-07 clean-checkout proof), the generator produces **ephemeral DEMO material at bake time** (loud DEMO label, presence-based gates). Accepted consequence: the auth image is intentionally non-byte-reproducible; pairing is internal-consistency, not cross-build identity; the r9 kernel FOD discipline is unaffected (auth image is a per-gate compose).

### Pairing verification venue

| Option | Description | Selected |
|--------|-------------|----------|
| Bake + boot check | mkimage asserts manifest↔artifacts AND guest re-verifies before keyfs with a loud serial abort; AUTH-07 anchor; Phase 19 can rely on guest-proven domain | ✓ |
| Bake assert only | Manifest check in mkimage only; "fails loud" has no runtime venue; hand-edited image boots into confusing keyfs failure | |
| Bake + on-demand tool | Committed in-guest verify command, no automatic boot check; probe exercises it, normal boot doesn't | |

### nvram-substitute delivery

| Option | Description | Selected |
|--------|-------------|----------|
| nvram= env var | readnvram.c getenv("nvram") donor-verbatim path (verified in-tree); zero kernel surface; any venue | ✓ |
| Kernel /dev/nvram device | r9 serves /dev/nvram or #S/sdC0/nvram backed by baked bytes; new kernel device surface for the same bytes | |
| Command-line flags | Explicit nvram-file flags on keyfs/authsrv command lines; local patch to donor mains; diverges from convention | |

### DEMO labeling

| Option | Description | Selected |
|--------|-------------|----------|
| Embedded + banner | Label inside every artifact (nvram authdom, /adm/keys header, ndb token, manifest row) + boot pairing-check serial print; ephemeral path always DEMO; local file may carry own label | ✓ |
| Banner only | Label lives in boot print + ndb spelling; copied-out artifacts carry no marker | |
| Label = source kind | Label semantics ride on file-present vs absent via authdom name only; rename silently unlabels | |

---

## r9 kernel substrate

### /dev/random re-key shape

| Option | Description | Selected |
|--------|-------------|----------|
| In-place re-seed | Keep devcons files + ChaCha core; CPUID-gated RDRAND/RDSEED (KVM) + TSC/jitter fallback (TCG); publish-once armed-source registry (13-03 precedent) | ✓ |
| Full devrandom device | Standalone device with donor devrandom.c semantics (ring buffer, health checks); bigger kernel footprint, no consumer distinguishes | |
| Host-seeded | Entropy via boot-protocol kv from host; randomness derives from qemu command line; weakest claim | |

### devcap fidelity

| Option | Description | Selected |
|--------|-------------|----------|
| Donor-verbatim | Port devcap.c semantics to port/src/devcap.rs: caphash hostowner-only write-once, capuse consume-on-match; Phase 19 sits on these semantics; small device, no kernel key material | ✓ |
| Minimal for auth_chuid | Subset for the one consumer with loud-unimplemented elsewhere; re-audited under 19's load | |
| devcap + chuid now | Port device AND wire kernel-side auth_chuid; out of scope — the path is Phase 19's | |

### Substrate evidence depth

| Option | Description | Selected |
|--------|-------------|----------|
| Host pins + boot profile | r9 host cargo tests (reseed + devcap semantics) + one xtask boot profile (random distinct-bytes + armed-source print; devcap post/consume/one-shot marker); KVM + TCG legs | ✓ |
| Composed gate only | The p9sk1 probe exercises everything; can't distinguish substrate breakage from auth config breakage | |
| Add stats battery | Plus in-guest chi-square-ish distribution test; notoriously flaky at TCG pace | |

---

## Image & service topology

### Image contract

| Option | Description | Selected |
|--------|-------------|----------|
| New auth image | Sixth mkimage contract: chain tree + auth binaries + baked domain + bring-up script + own expect gate + own flake app; services image untouched; one-image-per-gate stays clean | ✓ |
| Extend services image | Services image grows the real domain + listener; sweep venue becomes load-bearing; kitchen-sink inheritance for 19 | |
| Ride the chain image | No image work; expect-typed daemon startup; violates sourced-not-typed; nothing composed for 19 | |

### Listen scope

| Option | Description | Selected |
|--------|-------------|----------|
| Announce * | tcp!*!567 donor default; loopback probe unchanged; 16-D-04 hostfwd lands on a real listener for Phase 19 drawterm day one | ✓ |
| Loopback-only bind | tcp!127.1!567; hostfwd path refused later; announce change + re-green exactly when 19 lands | |
| Seam-driven scope | Announce address as netaddr seam data per venue; flexible, nothing requires it yet | |

### Serial auth logging

| Option | Description | Selected |
|--------|-------------|----------|
| Daemon verbose | keyfs/authsrv started with donor verbose flags so every handshake prints; pairing banner + startup markers; visible on any boot; researcher verifies exact flags | ✓ |
| Probe prints only | auth/debug trace + result markers; "day one" means probe-day; nothing for 19 to debug against | |
| Logged-to-file + tail | Daemon output re-routed to authlog file; adds mechanism serial provides free | |

### Phase 19 relationship

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal now | Auth image carries exactly AUTH-01..09 needs; 19 composes its own (seventh contract fine) | ✓ |
| Design as ancestor | service= switches + pre-staged listen/exportfs now; pays for 19's shape before 19's research | |
| Minimal + seam note | Minimal build plus a one-paragraph composition-seam note in the evidence | |

---

## Domain content & probe

### Baked user membership

| Option | Description | Selected |
|--------|-------------|----------|
| glenda + bootes | Donor-minimal two-user domain; bootes is the non-hostowner probe user | |
| Three users | glenda hostowner + bootes admin + third plain user; probe traffic never admin traffic | ✓ |
| Config-only, no default | Generator refuses without local file; CI supplies via secret — reopens the D-01 fallback | |

**User's choice:** Three users (override of the two-user recommendation).
**Notes:** Exact third user name is planner discretion; the generator interface carries the user list so local files name their own membership.

### Probe composition

| Option | Description | Selected |
|--------|-------------|----------|
| debug + speaksfor legs | auth/debug full p9sk1 as the third user over loopback + speaksfor c= certification leg; markers presence-asserted | ✓ |
| Probe as bootes | Collapses toward two-user shape; probe = admin traffic | |
| Add factotum client leg | factotum-mediated client; drawterm-era machinery 19 owns | |

### secstore depth

| Option | Description | Selected |
|--------|-------------|----------|
| Round-trip probed | secstored listens at boot; secuser creates third user account; client stores+retrieves; markers assert bytes returned | ✓ |
| Listener asserted only | Port announce marker, no store/retrieve; "available" proven by a port | |
| Staged only | Binaries in image, no daemon; AUTH-08 never honestly proven | |

### AUTH-09 documentation venue

| Option | Description | Selected |
|--------|-------------|----------|
| README + PROJECT row | New AUTH section in build/x86_64/README.md (16-07 extend-in-place precedent) + PROJECT.md Key Decisions row at close | ✓ |
| Dedicated AUTH-DOMAIN.md | One new doc home for auth-baking; README section + pointer achieves same with one less home | |
| Phase evidence only | Recorded where operators never look; fails AUTH-09's intent | |

---

## Claude's Discretion

- Generator implementation venue/language (host C aux-tool lineage vs shell vs harness subcommand) within D-01's committed-interface constraint
- Local file name/format; ephemeral DEMO authdom naming scheme; third user's name
- Exact daemon verbose flags + marker vocabulary (researcher verifies against the donor binaries)
- Entropy-gather internals (RDRAND/RDSEED usage, jitter mix, reseed cadence) within D-05
- devcap module placement/name; boot profile name/shape; auth bring-up script structure; keyfs write-back venue under read-only paqfs
- CI placement of the auth gate; evidence-file organization

## Deferred Ideas

- Host-9P `/adm` mount wiring (documented only — D-15; the wiring belongs to the phase needing a real domain)
- factotum-as-client machinery (Phase 19)
- auth_chuid consumption + cpu listener (Phase 19)
- In-guest statistical randomness battery (declined; demand-name later)
- r9 full devrandom device (declined this era)
- Dedicated AUTH-DOMAIN.md doc home (declined)
