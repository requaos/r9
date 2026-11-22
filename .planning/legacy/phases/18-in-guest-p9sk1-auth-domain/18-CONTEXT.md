# Phase 18: In-Guest p9sk1 Auth Domain - Context

**Gathered:** 2026-11-13
**Status:** Ready for planning

<domain>
## Phase Boundary

The x86_64 guest becomes a self-contained p9sk1 auth domain: one bake-time generator produces the machine key, `/adm/keys`, nvram-substitute, and ndb entries as a **hash-paired unit** so all four p9sk1 configs agree by construction; keyfs + authsrv serve it (keyfs non-interactive over the baked keys, authsrv on tcp/567); secstore is available for persistent key storage; and the in-guest loopback probe — auth/debug full p9sk1 exchange + the non-hostowner speaksfor gate — is green **before any drawterm is ever attempted** (this probe green is Phase 19's entry criterion). **Work spans both repos**: r9 gets `/dev/random` re-keyed to an x86_64 entropy source + devcap `#¤` (kernel work in `../r9`, branch `main`, push-before-pin); Harvey gets the domain generator, the bake machinery, the auth image, and the composed gate.

Addresses AUTH-01, AUTH-02, AUTH-03, AUTH-04, AUTH-05, AUTH-06, AUTH-07, AUTH-08, AUTH-09.

NOT in this phase: cpu service / drawterm / rio / `#!` exec / auth_chuid consumption (Phase 19 — this phase delivers the devcap device the cpu path depends on, not the path), host-9P `/adm` mount WIRING (AUTH-09's real-use answer is *documented* this phase, not implemented), factotum-as-client / drawterm-era client machinery, TLS/security modernization, aarch64 anything (frozen at the v2.0 pin — reference reads of the DTB-seeded random are fine), fossil/venti runtime.

</domain>

<decisions>
## Implementation Decisions

### Domain generator & key material (AUTH-01)
- **D-01:** **Key material comes from an uncommitted local file satisfying a committed interface.** A gitignored file at the Harvey repo root carries the developer's seed/passwords/keys fields; a committed interface (field names/types) defines its shape — actual seed/password/key bytes never land in git history. **Absent file (CI / clean checkout) → the generator produces ephemeral DEMO material at bake time** (real randomness, loud DEMO label on every artifact) — CI stays green on presence-based gates and the Phase-19 INT-07 clean-checkout proof holds. Consequence, accepted: the auth image is legitimately NOT byte-reproducible per developer/bake; the pairing manifest is an internal-consistency proof (all four artifacts from one generator run), never a cross-build byte-identity claim. The auth image is a per-gate dev compose (not FOD-bound); the r9 kernel-byte FOD discipline is untouched by this. — **Reversibility:** costly — the interface file's shape becomes the operator contract for every future real domain; changing fields later re-touches the generator, the bake, the docs, and any developer's local file.
- **D-02:** **Pairing verification fires at bake AND at boot.** mkimage asserts the manifest matches the staged artifacts before the image builds (bake refuses on mismatch); the guest re-verifies the pairing at boot **before keyfs starts**, aborting loud on serial. This gives AUTH-01's "fails loud" a runtime venue, catches post-bake drift, gives AUTH-07 its natural day-one anchor, and lets Phase 19 rely on the guest having proven its own domain. The boot check also prints the domain label (D-04).
- **D-03:** **The nvram-substitute reaches keyfs/authsrv via the `nvram=` env var** — the donor-verbatim path: `readnvram.c` does `getenv("nvram")` → plain file, safeoff 0, `Nvrsafe`-sized read (verified in-tree at `sys/src/libauthsrv/readnvram.c`). Zero kernel surface; keyfs/authsrv unmodified; works on any venue. The generator bakes an `Nvrsafe`-format file (path planner's choice, e.g. `/adm/nvram`) and the boot env carries `nvram=<path>`. Watch the failure class: a missing/invalid nvram drops `readnvram` into an **interactive console prompt** — exactly what AUTH-04's "non-interactively" forbids — so D-02's boot check must precede keyfs.
- **D-04:** **The DEMO label is embedded in every artifact + the boot banner.** The label is a first-class generator output landing inside the nvram authdom field, the `/adm/keys` header, the ndb authdom token, and the manifest row; the boot pairing check prints it on serial before keyfs starts. The ephemeral no-file path (D-01) is always DEMO; a developer's local file may carry its own label field (default DEMO). Any tool reading any artifact sees the label — no consumer-side remembering. — **Reversibility:** costly — the label is baked into artifact formats and the manifest; removing it later breaks the pairing format.

### r9 kernel substrate (AUTH-02, AUTH-03)
- **D-05:** **`/dev/random` takes an in-place re-seed, not a new device.** Keep the devcons `random`/`urandom` files and the ChaCha20 DRBG core; replace the DTB seed with an x86_64 entropy gather — CPUID-gated RDRAND/RDSEED when present (KVM `cpu=host`), TSC/jitter mix fallback (TCG qemu64) — publishing which source armed at boot, the 13-03 CPUID-gated dual-mode precedent (x2APIC/xAPIC) applied to entropy. AUTH-02's letter names exactly this ("re-keyed from v2.0's DTB-seeded design"); p9sk1 challenges are the only consumer this era. Venue fact shaping this: hosted CI runs TCG (the jitter path is load-bearing there) while the primary host runs KVM (RDRAND) — both legs get exercised by the existing gate topology.
- **D-06:** **devcap `#¤` is a donor-verbatim port** into r9 (`port/src/devcap.rs` suggested): caphash = hostowner-only, write-once-per-post hash table; capuse = present-the-capability, consume-on-match; the same 2-file qid tree the donor serves. The device is small (a kernel-mediated hash table — no kernel key material), Phase 19's cpu server sits directly on these semantics, and the project has repeatedly paid for faithful ports (the RFMEM donor-fidelity lesson). Semantics-level port to Rust with host-testable structure — not a literal C translation.
- **D-07:** **The substrate evidence is host pins + one xtask boot profile.** (a) r9-side host `cargo test` pins the reseed/mixing logic and the devcap hash-table semantics deterministically (the established `cfg(test)` pattern); (b) one boot profile proves live behavior: random — a marker reads N bytes, asserts non-empty + distinct across reads, and the armed-source line (RDRAND vs jitter) prints on serial; devcap — a marker posts a hash, consumes via capuse, and asserts the one-shot/hostowner rules. KVM + TCG legs both run. No in-guest statistical battery (distribution tests on VM entropy redden gates for statistical noise; distinctness + armed-source is what the p9sk1 consumer needs).

### Image & service topology (AUTH-04, AUTH-07)
- **D-08:** **The live auth domain lives in a NEW sixth mkimage contract** — chain tree + auth suite binaries (already tier-2-built) + the baked domain artifacts + an auth bring-up script (pairing verify → keyfs → authsrv → secstored → probe), with its own expect gate and its own flake app (the net-smoke-x64 lineage; app-per-gate). The services image keeps its sweep semantics untouched; one-image-per-gate (14-06) stays clean. — **Reversibility:** costly once Phase 19's composition names it — retracting or reshaping the contract after 19 plans against it re-touches the compose chain; additive and cheap to change before that.
- **D-09:** **authsrv announces `tcp!*!567`** (the donor default). The loopback probe works unchanged, AND the 16-D-04 slirp hostfwd (host 5670 → guest 567) lands on a real listener the day Phase 19's drawterm dials it — no rework, no auth-substrate re-audit between phases. The roadmap's "serve it over loopback /net" describes the probe's venue, not a bind restriction.
- **D-10:** **AUTH-07 serial visibility = daemon-verbose flags + the pairing banner + startup markers.** keyfs/authsrv start with their donor verbose/debug flags so every handshake prints to the console on its own — auth activity visible on ANY boot of the image, not just gate runs. The gate presence-asserts handshake lines. Researcher verifies the exact flags each donor binary honors (which flag prints what) before the plan pins the marker vocabulary.
- **D-11:** **The auth image is minimal; Phase 19 composes its own.** It carries exactly what AUTH-01..09 need — nothing speculative (no service=cpu switches, no pre-staged listen/exportfs). 19 composes from the contracts; if that is a seventh contract, fine — one-image-per-gate keeps each gate's blast radius honest.

### Domain content & probe (AUTH-05, AUTH-06, AUTH-08)
- **D-12:** **The ephemeral DEMO domain bakes three users** (user override of the two-user recommendation): glenda (hostowner — the boot-time owner), bootes (the canonical auth/cpu admin user), and a third plain user so probe traffic is never admin traffic. Exact third user name is planner discretion; the generator interface carries the user list, so a developer's local file names its own membership. AUTH-05's ndb content (authdom, hostid, sys, ether) follows from the generator's output.
- **D-13:** **The probe is two legs, both over loopback /net, run from the boot flow:** (1) auth/debug as the third (plain, non-hostowner, non-admin) user completes the full p9sk1 challenge-response against authsrv — the roadmap's named probe; (2) the speaksfor gate — the client authenticating in the speaks-for form (the c= certification path; researcher verifies auth/debug's exact flag/form against `debug.c`) as/for another user, asserting the non-hostowner path Phase 19's cpu server rides. Markers presence-asserted by the expect gate. No factotum-as-client leg (drawterm-era machinery Phase 19 owns).
- **D-14:** **secstore is proven by round-trip.** secstored listens at boot; the probe exercises a real store/retrieve: secuser creates the third user's secstore account, the client stores and retrieves a small object, markers assert the bytes came back (the test-delivery-not-counters doctrine). "Available" (AUTH-08) is proven by use, not by a port.
- **D-15:** **AUTH-09's scoping documentation lands as a new AUTH section in `build/x86_64/README.md`** (the 16-07 extend-in-place drift-notes home) **plus a PROJECT.md Key Decisions row at phase close** — operator-facing doc + decision record, the standard split. Content is fixed by the roadmap: baked+paired for CI; host-9P `/adm` mount for real use (documented, not wired).

### Carried forward (locked in prior phases / roadmap — do not re-litigate)
- **15-D-01 boundary**: this phase owns devcap `#¤`, the bake-time `/adm/keys` domain, keyfs-as-real-service, real p9sk1 handshakes, secstore — deliberately 18's, not pulled forward into 15
- **15-D-08**: shallow auth execution (start+answer) already proven in the services tier; this phase's real p9sk1 loopback probe never duplicates that
- **16-D-04**: guest port 567 is the in-guest contract (host enters at 5670 via slirp hostfwd — already wired as TOML data)
- **15-D-16**: ndb content rides the committed-template + netaddr-seam discipline — the bake-time venue-literal grep assert applies to AUTH-05's ndb entries
- push-before-pin; FOD re-wire-then-pin after every r9 kernel-byte fix pass (this phase's random re-seed + devcap are kernel bytes → one re-wire pass, pin advance its own commit); the auth image itself is a per-gate compose, never FOD-bound
- Gates split by artifact (14-D-06): kernel substrate evidence r9-side (D-07), composed auth gate Harvey-side
- Venue honesty: named venues, presence-based timing-tolerant assertions, green-twice, no unearned OK; TCG-generous timeouts
- Sourced-not-typed in-guest scripts (never typed lines — the known-flaky venue); rc dialect landmines (15-02: bare `=` in words, braced if-not bodies, fd-dup in nested blocks)
- App-per-gate / one-image-per-gate (14-06); single-source-as-data (seam, tier files, manifests)
- aarch64 frozen at the v2.0 pin — reference reads of the DTB-seeded devcons random are fine, no aarch64 changes
- Loopback probe green is Phase 19's entry criterion (roadmap) — the probe must be a committed, re-runnable gate, not a one-off observation

### Claude's Discretion
- The generator's implementation venue/language (host C via the aux host-tool lineage vs shell in mkimage vs a harness subcommand) — the committed-interface requirement (D-01) is the constraint
- The local file's exact name/format (rc-vars/ini/json) and the interface's committed definition home
- The ephemeral DEMO authdom naming scheme (must carry the loud DEMO token)
- The third user's name (D-12)
- Exact daemon verbose flags and the marker vocabulary (D-10 — researcher verifies what keyfs/authsrv/secstored actually honor)
- Entropy-gather internals (RDRAND vs RDSEED usage, jitter-mix shape, reseed cadence, fork mixing) within D-05's shape
- The devcap module's exact placement/name in r9; the boot profile's name/shape in xtask
- The auth bring-up script's structure (authrc vs termrc override — the services-termrc precedent), and keyfs's `/adm/keys` write-back mechanics under the read-only paqfs root (ramfs overlay or copy-before-start — the 15-02 through-mount/sentinel precedents apply)
- CI placement of the auth gate (join the r9-shell.yml net-smoke leg vs a new leg)
- Evidence-file organization under `.planning/phases/18-*/`

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 18 — goal, success criteria 1–5, requirement mapping; §Phase 19 (the consumer: cpu/drawterm ride this phase's auth domain + devcap)
- `.planning/REQUIREMENTS.md` — AUTH-01..AUTH-09 full text + Out of Scope table
- `.planning/PROJECT.md` — Key Decisions (push-before-pin, gates-split, venue honesty, FOD discipline rows), Constraints
- `.planning/STATE.md` — Accumulated Context (15-02's keyfs probe + rc dialect findings, 16-04's hostfwd/emitter decisions)

### Prior-phase context (the machinery this phase rides)
- `.planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-CONTEXT.md` — D-01 (the 18-owned boundary this phase executes), D-08 (shallow auth subset), D-16 (ndb template + seam)
- `.planning/phases/16-host-net-modes-networking-exit-gate/16-CONTEXT.md` — D-04 (hostfwd 5670→567), D-13 (app-per-gate soak shape the auth gate mirrors)
- `.planning/phases/14-harvey-amd64-userspace-on-r9/14-CONTEXT.md` — D-06 (gates by artifact), the one-image-per-gate discipline
- `.planning/milestones/v2.0-phases/08-r9-tcp-ip-path-devip-compatible-net/08-CONTEXT.md` — D-17: the minimal random device decision (the DTB-seeded ChaCha20 this phase re-keys — read for what was deliberately deferred here)

### Donor semantics (in-tree — the fidelity references)
- `sys/src/9k/port/devcap.c` (+ legacy twin `sys/src/9/port/devcap.c`) — the devcap donor of record (D-06)
- `sys/src/libauthsrv/readnvram.c` — the `getenv("nvram")` convention, `Nvrsafe` layout, the interactive-prompt failure class (D-03; verified at discussion time)
- `sys/src/libauthsrv/` — passthru/p9sk1 client-server glue (the speaksfor form verification)
- `sys/src/cmd/auth/keyfs.c`, `sys/src/cmd/auth/authsrv.c` — the daemons (AUTH-04); what verbose arms exist (D-10)
- `sys/src/cmd/auth/debug.c` — the probe (AUTH-06); its exact flags/forms (D-13)
- `sys/src/cmd/auth/secstore/` (secstore.c, secstored via mkfile, secuser) — the round-trip pieces (D-14)
- `sys/src/cmd/auth/changeuser.c` — how users/keys get created (the generator's in-tree semantic reference)
- `sys/man/8/authsrv`, `sys/man/4/keyfs` — daemon semantics references

### Build machinery (Harvey repo — the surfaces this phase changes)
- `build/x86_64/mkimage.sh` — the five existing contracts; the sixth auth contract joins; the services contract (~lines 764–870) is the nearest precedent (auth binaries staged, probe shapes)
- `build/x86_64/tiers/tier2-services.txt` — the auth suite entries (debug:117, secstore:141, authsrv/keyfs/listen asserted by the services bake); `tier1-cmds.txt` (factotum rides tier-1)
- `build/x86_64/services-termrc` + `services-sweep.rc` — the bring-up + sweep shapes the auth image's script extends; 15-02's keyfs probe (sentinel bind, through-mount observable) is the direct precedent
- `build/x86_64/root-tree/lib/netaddr.rc` — the single addressing seam; the ndb template substitution + literal-grep assert (15-D-16) that AUTH-05's entries must satisfy
- `nix/smoke-r9-net-x64.sh` + the `*.expect` gates — the app-per-gate + expect shape the auth gate copies
- `flake.nix` — the auth gate app joins (mkScriptApp + R9_* exports)
- `build/x86_64/README.md` — the AUTH section lands here (D-15; 16-07 extend-in-place precedent)

### r9 repo (sibling, `../r9` — where the kernel substrate lands)
- `../r9/port/src/devcons.rs` — the D-17 random section (QID_RANDOM/QID_URANDOM, ChaCha20 DRBG, the "no devcap, no auth adjacency" note) — the re-seed site (D-05)
- `../r9/port/src/devsrv.rs` / `devpipe.rs` — the small-device port patterns devcap follows (D-06)
- `../r9/x86_64/src/` — CPUID/RDTSC access + the 13-03 publish-once registry precedent the entropy gather mirrors (D-05)
- `../r9/xtask/` — the boot-test profile machinery (D-07's profile)
- `../r9/x86_64/lib/config_*.toml` — the q35 config lineage the substrate profile joins

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- The whole tier-2 auth suite — keyfs, authsrv, auth/debug, secstore/secstored/secuser, changeuser, convkeys, factotum (tier-1) — already compiled for amd64 and staged by the services bake; this phase wires them into a real domain, it does not build them
- mkimage's five contracts + the services contract's probe-precedent machinery (tier-entry asserts, host-side probe clients, expect gates)
- r9's devcons ChaCha20 DRBG — the core survives; only the seed source changes (D-05)
- r9's established `cfg(test)` host-pin homes (10+ modules) + xtask boot-profile chassis (the 13/14 gate lineage) for D-07's evidence
- 15-02's keyfs probe techniques: the sentinel bind isolating interactive prompts, the through-mount write-back observable
- The slirp hostfwd pair (17010 + 5670→567) already committed as TOML data — D-09's listener is reachable from the host on day one

### Established Patterns
- Single-source-as-data: seam for addresses, tier files for membership, manifests for payloads — the domain generator's interface file (D-01) and pairing manifest (D-02) join this family
- One implementation, committed interface, mechanical asserts (bake-time literal grep, manifest digest checks, negative controls)
- Fail-loud + no silent downgrade: the boot pairing abort (D-02) and the armed-source print (D-05) follow the venue-honesty lineage
- Presence-based, timing-tolerant expect assertions over serial markers; sourced-not-typed guest scripts

### Integration Points
- `../r9` `main` branch — `/dev/random` re-seed + devcap + host pins + the substrate boot profile (push-before-pin; FOD re-wire-then-pin once at the kernel-byte pass end)
- `build/x86_64/mkimage.sh` — the sixth contract; `flake.nix` — the auth gate app; `build/x86_64/README.md` — the AUTH doc section
- The netaddr seam + ndb template — AUTH-05's entries ride them
- `build/x86_64/root-tree/` — the baked domain artifacts stage here (contract-declared, manifest-asserted)

</code_context>

<specifics>
## Specific Ideas

- **The acceptance story, end-to-end:** the committed interface + gitignored local-file contract land (D-01) → the generator produces the hash-paired unit (machine key/Nvrsafe nvram-substitute + `/adm/keys` + ndb entries + manifest; ephemeral loud-DEMO when the file is absent) with the label embedded in every artifact (D-04) → mkimage's sixth contract bakes the auth image with pairing asserted at bake (D-02) → r9's `/dev/random` re-seeds from CPUID-gated RDRAND + TSC/jitter fallback and devcap `#¤` lands donor-verbatim, host-pinned + boot-profile-proven on KVM and TCG (D-05/06/07; push-before-pin; FOD re-wire once) → the auth image boots: pairing re-verified on serial with the domain banner, then keyfs (non-interactive via `nvram=` env, D-03) → authsrv announcing `tcp!*!567` (D-09) → secstored listening (D-14) — all daemon-verbose (D-10) → the probe runs from the boot flow: auth/debug full p9sk1 as the third user over loopback + the speaksfor leg green (D-13), secuser account + secstore round-trip green (D-14) → the expect gate presence-asserts everything, green-twice → AUTH-09's scoping doc lands in the README + PROJECT row (D-15) → Phase 19 is unblocked with its entry criterion met.
- **Watch-out:** a missing/invalid nvram makes `readnvram` prompt interactively on the console — a headless gate wedges silently behind that prompt. The boot pairing check (D-02) MUST precede keyfs, and the gate must assert the prompt never appears.
- **Watch-out:** keyfs serves `/adm/keys` but its protocol allows changes (user/key writes) — under the read-only paqfs root those writes need a venue (ramfs overlay / copy-before-start). The 15-02 probe never exercised change-write-back; the speaksfor/secstore legs may. Researcher sizes this before the plan freezes the bring-up script.
- **Watch-out:** the auth image is intentionally non-byte-reproducible (D-01) — no gate may ever assert byte-identity on it; the pairing manifest is the internal-consistency proof. The r9 FOD re-wire discipline applies ONLY to the kernel-byte pass (random + devcap), once.
- **Watch-out:** rc dialect landmines apply to the auth bring-up script (bare `=` ends words — awkward for `nvram=/adm/nvram`-style lines in rc; braced if-not bodies kill sourced files; fd-dup unreliable in nested blocks — the 15-02 measured list). Flat status ladders, quoted forms.
- **Watch-out:** secuser/password flows are interactive by donor design — the probe needs the 15-02 sentinel-bind technique or a scripted feed; researcher verifies each interactive seam (changeuser, secuser, secstore client) before the plan pins the probe steps.
- **Watch-out:** the speaksfor leg's exact form (which flag, who speaks for whom, what keyfs entry it needs) must be verified against `debug.c`/libauthsrv — do not plan from memory of the protocol.
- **Watch-out:** CI is TCG → the jitter entropy path is load-bearing there; a TCG-only red after a KVM-local green is a real substrate bug, not gate noise.
- **User overrides to honor downstream (deliberate, don't second-guess):** the uncommitted local-file interface for key material over committed-demo-set/fresh-per-bake (D-01); three baked users over the two-user minimal (D-12).
- **mkpaqfs silent-drop standing hazard** (15-07 deferred item): any new baked file class in the auth image rides the same blocksize-fit risk — small keys are fine, but the bake assert pattern (fail on any mkpaqfs diagnostic) is the protection.

</specifics>

<deferred>
## Deferred Ideas

- **Host-9P `/adm` mount wiring** — AUTH-09's real-use answer is documented this phase (D-15), implemented never here; the wiring belongs to the phase that first needs a real (non-CI) domain (post-v2.1 or a demand-named follow-up).
- **factotum-as-client / drawterm-era client machinery** — Phase 19 owns the client side; this phase's probe is auth/debug-direct (D-13).
- **auth_chuid consumption + cpu listener** — Phase 19 (D-06 delivers the device, not the path).
- **A in-guest statistical randomness battery** — declined (D-07); if a future consumer needs distribution proof, name it then.
- **A r9-side full devrandom device** (ring buffer, health checks, per-reader state) — declined this era (D-05); the devcons files stand until a consumer distinguishes.
- **Dedicated AUTH-DOMAIN.md doc home** — declined (D-15); the README section + PROJECT row suffice; revisit if the baking surface grows beyond one section.

</deferred>

---

*Phase: 18-In-Guest p9sk1 Auth Domain*
*Context gathered: 2026-11-13*
