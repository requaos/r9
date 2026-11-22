# Phase 18: In-Guest p9sk1 Auth Domain - Research

**Researched:** 2026-11-13
**Domain:** Plan 9 p9sk1 auth domain (keyfs/authsrv/secstore daemons + nvram/keys/ndb artifact formats) + r9 kernel substrate (entropy re-seed, devcap port)
**Confidence:** HIGH (donor sources read verbatim in-tree; external facts verified via Intel DRNG guide + qemu docs; open items named in Assumptions Log)

**Graph note:** knowledge graph is 162h / 291 commits stale (built at v2.1 start, 84576 nodes) — donor-auth structure queries returned only restatements of files read directly this session; all load-bearing claims below cite direct file reads, not the graph.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

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

### Deferred Ideas (OUT OF SCOPE)
- **Host-9P `/adm` mount wiring** — AUTH-09's real-use answer is documented this phase (D-15), implemented never here; the wiring belongs to the phase that first needs a real (non-CI) domain (post-v2.1 or a demand-named follow-up).
- **factotum-as-client / drawterm-era client machinery** — Phase 19 owns the client side; this phase's probe is auth/debug-direct (D-13).
- **auth_chuid consumption + cpu listener** — Phase 19 (D-06 delivers the device, not the path).
- **A in-guest statistical randomness battery** — declined (D-07); if a future consumer needs distribution proof, name it then.
- **A r9-side full devrandom device** (ring buffer, health checks, per-reader state) — declined this era (D-05); the devcons files stand until a consumer distinguishes.
- **Dedicated AUTH-DOMAIN.md doc home** — declined (D-15); the README section + PROJECT row suffice; revisit if the baking surface grows beyond one section.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AUTH-01 | Bake-time domain generator: machine key + `/adm/keys` + nvram-substitute + ndb entries as ONE hash-paired unit; demo domain loudly labeled DEMO; pairing fails loud | `/adm/keys` byte format fully verified (writeusers layout, DES-CBC zero-IV, KEYDBOFF=8, KEYDBLEN=73); Nvrsafe layout verified (109 bytes, 5 checksummed fields via nvcsum seed 9); pairing invariant = `/adm/keys` encrypted under nvram `machkey` (getauthkey.c:14); key-encryption-key seam verified |
| AUTH-02 | r9 `/dev/random` CSPRNG seeded from x86_64 entropy (no DTB), re-keyed from DTB-seeded design | RandomHooks seam verified (devcons.rs:1034-1055); x86_64 currently registers NO hooks (deterministic seed — confirmed at random_read None-arm); cpuid()/rdtsc() exist in cpu.rs; RDRAND=CPUID.1:ECX[30], RDSEED=CPUID.7:EBX[18] verified; TCG qemu64 lacks RDRAND, KVM -cpu host passes it, local host has rdrand+rdseed (verified) |
| AUTH-03 | r9 devcap `#¤` (caphash/capuse) — the auth_chuid path | Donor semantics verified (9k/port/devcap.c full read; legacy twin differs only in capstat signature); consumer contract verified via as.c mkcap/usecap + factotum rpc.c:341; r9 gaps named: no per-proc user (Proc struct lacks it), no eve (devenv comment), no HMAC-SHA1 in-tree; registry_attach match-arm verified |
| AUTH-04 | keyfs starts non-interactively over baked `/adm/keys`; authsrv listens on tcp/567 | keyfs flag surface verified (-p/-m/-w/keyfile; NO verbose flag); getauthkey interactive seam verified (`can't read NVRAM, please enter machine key` + getpass); authsrv is a per-connection stdin/stdout filter — listener options verified (aux/listen1 passes flags incl. -d; listen staged-copy cannot) |
| AUTH-05 | ndb content (authdom, hostid, sys, ether) staged in image | cs query path verified (`!authdom=X auth=*` answers straight from ndb — dns only intercepts dom=/ip= keys); template has `ip=@GUESTV4@ sys=harvey dom=harvey.local` with documented Phase-18 extension point; /lib/ndb/auth (hostid= speaksfor rows) is load-bearing for AUTH-06 and must be staged |
| AUTH-06 | In-guest loopback p9sk1 probe (auth/debug) green before drawterm; non-hostowner speaksfor gate | auth/debug verified: NO flags, keys enumerated from /mnt/factotum/ctl, password via /dev/cons readcons; **stock auth/debug cannot drive hostid≠uid** (verified: tr.hostid=tr.uid=user at debug.c:233-234) — speaksfor leg needs either a small C probe (precedent: host-side 6c/6l hsvctcpecho) or factotum role=speakfor (p9sk1.c:260-264 form verified); sentinel-bind drives auth/debug passwords; cs must run (csgetvalue has NO ndb fallback) |
| AUTH-07 | Serial-visible auth logging from day one | syslog fallback verified: `/sys/log/<name>` unwritable/absent → lines go to `#c/cons` (serial) — the natural mechanism on read-only paqfs; authsrv -d marker vocabulary verified (tr-ok/tr-fail/cr-ok/cr-fail lines); keyfs prints `%d keys read`; secstored -v stays foreground + syslog ANNOUNCE/AUTH/[id] PUT/GET markers |
| AUTH-08 | secstore available for persistent key storage | secstored verified: self-contained announce/listen/accept on `tcp!*!5356` default; client `-i` reads password from STDIN (scriptable, loud failure); who-file format verified; `/adm/secstore` needs a writable venue (SECSTORE_DIR verified); secuser interactive seams mapped (getpassm needs ORDWR /dev/cons — sentinel trick kills it; stdin prompts redirectable); zero-clock expiry math verified OK (default expire 31536000 > now=0) |
| AUTH-09 | writable-/adm + nvram-substitute scoping decided and documented | D-15 locked the venue (README AUTH section + PROJECT row); research sized the actual write surfaces: keyfs write-back non-fatal on failure, Qlog writes never call writeusers, secstore is the real writable consumer |
</phase_requirements>

## Summary

This phase stands up a complete donor-verified p9sk1 auth domain. The good news from reading the donors: everything the domain needs already exists as tier-built binaries (keyfs, authsrv, auth/debug, secstore suite, changeuser, factotum at tier-1), and the four "configs" that must agree are fully specified byte formats I verified in-tree: the nvram-substitute is a 109-byte `Nvrsafe` with five nvcsum-checksummed fields; `/adm/keys` is `8 random bytes + nu × 73-byte records` DES-CBC-encrypted (zero IV, libsec's "lacy cryptlib" partial-tail rule) under the nvram `machkey` — that encryption-key identity IS the pairing invariant; the ndb side needs BOTH `/lib/ndb/local` rows (`authdom=`/`auth=`/`sys=`/`ether=`) AND `/lib/ndb/auth` speaksfor rows (`hostid=` + `uid=`), because authsrv's `speaksfor()` consults the latter when a ticket request carries `hostid ≠ uid`.

The r9 substrate work is precisely scoped. `/dev/random` already has the exact seam D-05 needs: `RandomHooks { rng_seed, jitter }` registered once at boot (devcons.rs), and x86_64 currently registers none (the "deterministic-seed era" STATE records) — the re-seed is a hooks registration + an `rdrand`/`rdtsc`-based implementation, with `cpuid()`/`rdtsc()` already in cpu.rs and the 13-03 CPUID-gated helper pattern to copy. devcap is a 287-line donor whose kernel needs are: HMAC-SHA1 (absent in r9 — the hand-rolled ChaCha20 precedent says hand-roll, no new crate), a per-proc user field (absent — Proc struct verified without one; `/dev/user` is a static `glenda` const), and a hostowner/eve gate (devcons's HOSTOWNER static is the candidate). The device joins as one `registry_attach('¤', …)` match arm; namec's `first_rune` already decodes the 2-byte UTF-8 rune.

Three discoveries materially shape the plan. **First**: stock `auth/debug` cannot drive the speaksfor leg — it takes no flags and always sets `tr.hostid = tr.uid = user` (debug.c:233-234, verified); the speaksfor form is `tr.hostid = key-user, tr.uid = sysuser` (factotum p9sk1.c:260-264), so leg 2 needs either a small purpose-built C probe (the hsvctcpecho host-side 6c/6l precedent) or a factotum `role=speakfor` key — and auth/debug in any case REQUIRES a mounted factotum (it enumerates keys from `/mnt/factotum/ctl`) and cs running (`csgetvalue` opens `/net/cs` with no ndb fallback). **Second**: AUTH-07's serial logging is nearly free — libc `syslog(0, name, …)` falls back to `#c/cons` whenever `/sys/log/<name>` can't be opened for write, which is always on the read-only paqfs root, so `authsrv -d`'s `tr-ok`/`tr-fail` lines reach serial with zero new machinery (do NOT create a writable /sys/log/auth); passing `-d` through a listener needs `aux/listen1` (verified in-tree, `listen1 [-tv] address cmd args...`) because the staged-copy trick execs a bare binary. **Third**: the interactive-prompt map is now exact — keyfs's failure seam is `getauthkey`'s `print("can't read NVRAM, please enter machine key")` + getpass (readln opens /dev/cons by path; the 15-02 sentinel-bind neutralizes it), auth/debug's password prompts are sentinel-drivable, but secuser's `getpassm` opens `/dev/cons` ORDWR and sysfatals under a read-only sentinel bind — secuser's two password prompts must be console-fed (expect-typed) while its remaining prompts read fd 0 (stdin-redirectable); the secstore client needs none of this (`-i` reads the password from stdin).

**Primary recommendation:** build the generator as host C in the mkimage environment compiling the donor's own algorithm sources (passtokey + libsec DES-CBC + nvcsum; plan9port `9c` is already the bootstrap `pcc` in setup.sh — verify libsec/libauthsrv linking in the nix env as a Wave-0 probe, vendoring the three tiny files as fallback); drive leg 2 with a small host-side-6c-built probe client modeled on debug.c's `authfutz` with `hostid ≠ uid`; run authsrv via `aux/listen1 -t 'tcp!*!567' /amd64/bin/authsrv -d` (add the tier leaf line); give secstore a writable `/adm/secstore` via the established scratch-server venue; and let syslog's console fallback carry AUTH-07.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Domain artifact generation (nvram/keys/ndb/manifest) | Host build (mkimage bake time) | — | Bake-time unit production (D-01); needs DES/passtokey — donor C code, host-compiled; never runs in guest |
| Pairing verify (bake) | Host build (mkimage assert) | — | D-02 bake arm; digest/literal asserts over staged artifacts |
| Pairing verify (boot) | Guest boot flow (auth bring-up script) | — | D-02 boot arm; runs before keyfs; prints label banner on serial |
| p9sk1 key serving (keyfs) | Guest user daemon | kernel `/dev/random` (its randombytes fallback path reads it) | Donor daemon unmodified; runs as hostowner over mounted /adm/keys via nvram= env |
| p9sk1 ticket service (authsrv tcp/567) | Guest user daemon (per-connection via listen1) | — | authsrv is a stdin/stdout filter; listener owns announce |
| Name resolution for the probe (authdom→auth) | Guest daemon (ndb/cs) | /lib/ndb/local + /lib/ndb/auth data | csgetvalue requires /net/cs; cs answers authdom= keys straight from ndb |
| Entropy source (RDRAND/RDSEED/jitter) | r9 kernel (x86_64 arch crate) | port RandomHooks seam | Arch-specific instructions; port cannot read CPUID/RDTSC |
| CSPRNG (ChaCha20 DRBG) | r9 kernel (port devcons) | — | Existing core survives D-05; only seeding changes |
| Capability device (#¤) | r9 kernel (port) | — | Kernel-mediated hash table; donor-verbatim semantics; needs per-proc user + eve + HMAC-SHA1 |
| Probe legs (p9sk1 + speaksfor + secstore round-trip) | Guest boot flow (auth bring-up script) | expect gate (markers) | Run from boot flow (D-13/D-14); presence-asserted by the gate |
| Serial-visible auth logging | Guest daemons (syslog console fallback) | expect gate (asserts) | libc syslog falls to #c/cons when /sys/log/<n> unwritable |
| Gate composition (ISO + expect) | Host (flake app + expect) | r9 pinned kernel-x64 + iso-builder | App-per-gate lineage (net-smoke-x64) |
| slirp hostfwd 5670→567 | r9 TOML data (qemu.netdev.slirp) | — | Already committed (16-D-04); D-09's listener lands on it day one |

## Standard Stack

No new external packages. The stack is the donor auth suite (already tier-built, amd64) + the existing build/gate machinery + in-tree r9 kernel patterns. "Version" here = the in-tree source of record.

### Core (the donor suite — all already built by the services tier)
| Component | Source of record | Purpose | Verified facts |
|-----------|-----------------|---------|----------------|
| keyfs | `sys/src/cmd/auth/keyfs.c` (1114 lines) | Serves /adm/keys as a 9P tree at /mnt/keys; DES-CBC read/write | Flags `[-p] [-m mtpt] [-w warn] [keyfile]` (keyfs.c:135); default keyfile `/adm/keys` (line 163); NO verbose/debug flag exists; prints `%d keys read` at load (line 915); server forks RFPROC|RFNAMEG|RFNOTEG|RFNOWAIT|RFENVG|RFFDG (line 179) [VERIFIED: sys/src/cmd/auth/keyfs.c:135-192] |
| authsrv | `sys/src/cmd/auth/authsrv.c` (914 lines) | Per-connection p9sk1 ticket service on stdin/stdout | Only flag `-d` → syslog(0,"auth",...) debug lines `tr-ok`/`tr-fail`/`cr-ok`/`cr-fail` (lines 49-52, 117-155); alarm 10 min per conn; ndbopen("/lib/ndb/auth") at start (line 60) [VERIFIED: sys/src/cmd/auth/authsrv.c:43-104] |
| auth/debug | `sys/src/cmd/auth/debug.c` (345 lines) | The loopback probe | NO flags (`usage: auth/debug`, line 43); enumerates keys from `/mnt/factotum/ctl` (line 131); passwords via /dev/cons readcons (raw); success marker `ticket request using %s@%s key succeeded` (line 267); always sets tr.hostid=tr.uid=user (lines 233-234) — no speaksfor form [VERIFIED: sys/src/cmd/auth/debug.c] |
| secstored / secstore / secuser | `sys/src/cmd/auth/secstore/{secstored,secstore,secuser}.c` + `secstore.h` | PAK-authenticated store on its own port | secstored: `[-R] [-S name] [-s tcp!*!5356] [-v] [-x netmtpt]` (secstored.c:20-21), `-v` keeps foreground (line 348), self-contained announce/listen/accept loop, syslog markers `ANNOUNCE`, `AUTH %s`, `[%s] GET/PUT`; client: `-i` = password from stdin (secstore.c:410-421, loud `exits("invalid password on standard input")`), `-n` = from nvram `config` field, put success prints `secstore: saved %ld bytes` (line 226); secuser: `[-v] <user>` only, creates `/adm/secstore{,/who,/store}` [VERIFIED: sys/src/cmd/auth/secstore/*] |
| factotum | `sys/src/cmd/auth/factotum/` (tier-1 line `cmd/auth/factotum:factotum`) | Key store for auth/debug's enumeration; /mnt/factotum mount | Sweep-proven start `hfactotum -s factotum` + ctl key write `key proto=pass ... !password=...` + read-back (services-sweep.rc:297-310); p9sk1 keyprompt `user? !password?` (p9sk1.c bottom) [VERIFIED] |
| ndb/cs | `sys/src/cmd/ndb` (tier-2 `cmd/ndb`) | Resolves authdom→auth for the probe | csgetvalue opens `/net/cs` ORDWR, returns 0 on failure — NO ndb fallback (csgetval.c:30-33); cs's generic pair parser: first pair is the ndb key, dns only intercepts dom=/ip= (cs.c:1728-1768) — `!authdom=X auth=*` answers straight from ndb [VERIFIED: sys/src/libndb/csgetval.c, sys/src/cmd/ndb/cs.c] |
| aux/listen1 | `sys/src/cmd/aux/listen1.c` (NOT yet a tier entry) | Per-connection runner that passes flags | `usage: listen1 [-tv] address cmd args...`; announce(argv[0]) then per-conn rfork(RFFDG|RFPROC|RFNOWAIT|RFENVG|RFNAMEG|RFNOTEG) + exec(argv[1], argv+1) — flags ride (listen1.c:11, 84-120) [VERIFIED] — add tier leaf line `cmd/aux:listen1` |
| changeuser | `sys/src/cmd/auth/changeuser.c` | The generator's in-tree semantic reference | Operates on the MOUNTED tree (create /mnt/keys/<user> dir, write <user>/key 7 bytes, <user>/expire) — the offline generator must produce byte-equivalent records via writeusers' layout [VERIFIED: sys/src/cmd/auth/changeuser.c:105-133] |

### Supporting (build/gate machinery this phase extends)
| Component | Path | Role |
|-----------|------|------|
| mkimage contracts | `build/x86_64/mkimage.sh` (1145 lines) | Services contract = the direct precedent (section 11, lines 762-1000): tier-entry asserts (821-824), computed h* staging from tier data (826-873), host-side 6c/6l probe-client builds (954-966, from `$HARVEY_LINUX_BIN` = `.build/hostbin-linux-x86_64/bin` after `nix run .#setup && nix run .#toolchain`), KERNDATE mtime pinning |
| tier data | `build/x86_64/tiers/tier2-services.txt` | Auth entries verified: `cmd/auth/authsrv`(:112), `cmd/auth/keyfs`(:122), `cmd/auth/debug`(:117), `cmd/auth/changeuser`(:113), `cmd/auth/as`(:109), `cmd/auth/secstore`(:141 dir — stages secstore/secstored/secuser/aescbc); factotum tier-1 (:103) |
| netaddr seam | `build/x86_64/root-tree/lib/netaddr.rc` | Single addressing source; the venue-literal grep assert covers new AUTH-05 content; ether= key absent today (planner adds) |
| ndb template | `build/x86_64/root-tree/lib/ndb/local.template` | `ip=@GUESTV4@ sys=harvey dom=harvey.local` + ipmask/ipgw; header documents the Phase-18 extension point (ipnet= blocks with authdom/auth) |
| bring-up script shape | `build/x86_64/services-termrc` + `services-sweep.rc` | `rc /lib/<script>.rc <venue>` child + `exec rc -i`; sweep's mntgen/loopback/cs bring-up blocks are the direct precedent (mntgen mounts /mnt; `bind -a '#I' /net`; loopback ipifc 127.0.0.1 + a second non-loopback address because myipaddr skips loopback; `hcs -f <ndb>` with load-bearing trailing newline) |
| gate driver + expect | `nix/smoke-r9-net-x64.sh` + `build/x86_64/smoke-r9-net.expect` + `flake.nix` netSmokeR9X64Pkg (lines 237-244) | App-per-gate shape; runtimeInputs already include `pkgs.plan9port`, expect, qemu, iproute2; R9_KERNEL_X64_PKG/R9_ISO_BUILDER/R9_NETDEV_EMITTER exports |
| CI | `.github/workflows/r9-shell.yml` (Harvey repo) | `net-smoke-x64` job: `env R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64` (lines 84-90) — the auth gate's placement neighbor |
| scratch/writable venue | mntgen (tier-2, `cmd/mntgen`) | The sweep's mount-point provider; 15-06 measured mntgen read-only on the tree for FILE writes — the writable-/tmp note in 15-06 named ramfs; verify which serves /adm/secstore writes in the auth image (Wave-0 probe — see Assumptions) |

### r9 substrate surfaces (verified this session)
| Surface | Path | Fact |
|---------|------|------|
| RandomHooks seam | `port/src/devcons.rs:1029-1055` | `RandomHooks { rng_seed: fn(&mut [u8]) -> usize, jitter: fn() -> u64 }`; `set_random_hooks` asserts single registration; hooks None on x86_64 today → `random_read` seeds deterministically (lines 1111-1136); once-marker `devcons: random seeded`; per-read jitter mix [VERIFIED: ../r9/port/src/devcons.rs] |
| CPUID/RDTSC | `x86_64/src/cpu.rs:164-216` | `pub fn cpuid(leaf, subleaf) -> (eax, ebx, ecx, edx)` (safe, rbx-rescued); `pub fn rdtsc() -> u64`; decision-helper precedent `has_x2apic(cpuid1_ecx)` / `has_tsc_deadline` — mirror as `has_rdrand`/`has_rdseed`; NO rdrand asm wrapper yet [VERIFIED] |
| device registry | `port/src/namec.rs:60-80` | `registry_attach(rune, spec)` match arms ('c','e','l','I'); `first_rune` decodes multi-byte UTF-8 (namec.rs:87-105) — `'¤'` (0xC2 0xA4) works today [VERIFIED] |
| Proc | `x86_64/src/proc.rs:978` | Real per-proc struct (pid/name/state/kstack/pml4/fd_group/mappings/frame) — **no user field**; port/dat.rs:340 `pub struct Proc {}` is a stub; `/dev/user` is `const USER_CONTENT: &[u8] = b"glenda"` (devcons.rs:317-321) [VERIFIED] |
| hostowner | `port/src/devcons.rs:177-181` | `static HOSTOWNER: Lock<Vec<u8>>` behind `/dev/hostowner` (QID 4, 0664) — the eve-comparison candidate for devcap's hostowner gate; factotum's writehostowner targets `#c/hostowner` (factotum/util.c:951-963) [VERIFIED] |
| devcap donor | `sys/src/9k/port/devcap.c` (287 lines) | Full semantics below in Patterns; legacy twin `sys/src/9/port/devcap.c` differs ONLY in capstat return type (diff run: one hunk) [VERIFIED] |
| entropy facts | Intel DRNG guide + qemu docs | RDRAND = CPUID.(EAX=1):ECX bit 30; RDSEED = CPUID.(EAX=7,ECX=0):EBX bit 18 (check max-leaf first); RDSEED = conditioned source for seeding PRNGs [VERIFIED: Intel DRNG Software Implementation Guide]. qemu qemu64 default does NOT expose RDRAND; TCG implements RDRAND only when enabled (`-cpu qemu64,+rdrand` / `-cpu max`); KVM `-cpu host` passes host flags — CI TCG leg therefore rides the jitter path exactly as D-05 states [VERIFIED: qemu.org i386 CPU docs + KVM flag writeups]. Local host: rdrand+rdseed on all 16 CPUs (verified via /proc/cpuinfo), qemu 11.1.2 |
| r9 HEAD | `../r9` main @ 8f3ff11 | Matches STATE's Phase-17 terminus (r9 untouched == origin/main); push-before-pin applies to this phase's kernel bytes |

**Installation:** nothing to install. One optional toolchain note: the host-side 6c/6l already exist at `.build/hostbin-linux-x86_64/bin/` (verified present); plan9port is a gate runtimeInput AND setup.sh's pcc bootstrap (`install_tool "$PLAN9/bin/9c" pcc`, setup.sh:26-28).

## Package Legitimacy Audit

This phase installs **no external packages** (npm/PyPI/crates). The r9 substrate deliberately avoids new crates (recommendation: hand-rolled SHA-1/HMAC per the ChaCha20 "no new crate" precedent, devcons.rs:939-941). The only package-adjacent surface is nixpkgs `plan9port`, already a committed runtimeInput of the existing gate apps — no new registry lookups performed, none required.

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
BAKE TIME (host, mkimage sixth contract)
  local key file (gitignored, D-01)          [absent -> ephemeral DEMO]
        |
        v
  +------------------  domain generator (host C, donor algorithms)  ------------------+
  | nvram-substitute: 109-byte Nvrsafe { machkey|authkey|config|authid|authdom + nvcsums } |
  | /adm/keys: KEYDBOFF(8 rand) + nu*73B records, DES-CBC(machkey, zero IV, lacy tail)     |
  | /lib/ndb/local rows: authdom=/auth=127.0.0.1/sys=/ether=  (via netaddr seam tokens)    |
  | /lib/ndb/auth rows: hostid=<speaker> uid=!sys uid=!adm uid=*  (speaksfor rules)        |
  | manifest: hash-paired unit + DEMO label row                                             |
  +------------------------------------ | ---------------------------------------------+
                                       v
              mkimage pairing assert (D-02 bake arm: refuses on mismatch)
                                       |
                                       v
              bootfs-auth.paq --> r9 pinned kernel-x64 --> ISO (compose, per-gate)

GUEST BOOT (auth bring-up script, from boot flow)
  [D-02 boot arm] pairing re-verify  --FAIL--> abort loud on serial (before keyfs)
       |  pass: print domain label banner (D-04)
       v
  nvram=/adm/nvram in env  (D-03; generator baked path)
       v
  keyfs (no -p!) -- getauthkey -> readnvram(&safe,0) -> machkey  [interactive ONLY on failure]
       |  mounts /mnt/keys; prints "%d keys read"                     ^ watch-out
       v
  listen1 -t 'tcp!*!567' /amd64/bin/authsrv -d   (per-conn fork; tr-ok/tr-fail lines)
  secstored -v                                    (tcp!*!5356; ANNOUNCE/AUTH/[id] markers)
  writable venue over /adm/secstore               (scratch server; who/ + store/ trees)
  cs over /lib/ndb/local                          (authdom->auth resolution; loopback #I bind first)
  factotum -s factotum + ctl key proto=p9sk1      (auth/debug's enumeration source)
       |
       v
  PROBE (boot flow; markers presence-asserted by expect gate, green-twice)
   1. auth/debug as third user: factotum ctl lists p9sk1 key -> authdial via cs ->
      full Treq/AuthTreq round-trip -> "ticket request using USER@DOM key succeeded"
      (passwords fed by sentinel-bound /dev/cons; syslog lines reach #c/cons)
   2. speaksfor leg: Ticketreq{authid=bootes, hostid=bootes, uid=thirduser} ->
      authsrv speaksfor() consults /lib/ndb/auth hostid= rows -> suid=uid granted
      (stock auth/debug cannot send hostid!=uid: small C probe client needed)
   3. secstore round-trip: secuser creates third user's account (console-fed passwords)
      -> secstore -i -p obj / -g obj (stdin password) -> bytes-back marker
       |
       v
  AUTH-07: serial console carries pairing banner + "%d keys read" + tr-ok/tr-fail +
  ANNOUNCE/AUTH/[id] lines on EVERY boot (syslog console fallback via #c/cons)
```

### Donor formats (the pairing invariants — all verified verbatim)

**Nvrsafe** — `sys/include/authsrv.h:140-157` (exact layout; all fields char/uchar, sizeof = 7+1+7+1+14+1+28+1+48+1 = 109 bytes):
```c
struct Nvrsafe
{
	char	machkey[DESKEYLEN];	/* was file server's authid's des key */
	uchar	machsum;
	char	authkey[DESKEYLEN];	/* authid's des key from password */
	uchar	authsum;
	char	config[CONFIGLEN];	/* secstore key on non-file-servers */
	uchar	configsum;
	char	authid[ANAMELEN];	/* auth userid, e.g., bootes */
	uchar	authidsum;
	char	authdom[DOMLEN]; /* auth domain, e.g., cs.bell-labs.com */
	uchar	authdomsum;
};
```
Constants (`authsrv.h:17-28`, verbatim): `ANAMELEN=28, DOMLEN=48, DESKEYLEN=7, CHALLEN=8, NETCHLEN=16, CONFIGLEN=14, SECRETLEN=32, KEYDBOFF=8, OKEYDBLEN=ANAMELEN+DESKEYLEN+4+2 (41), KEYDBLEN=OKEYDBLEN+SECRETLEN (73)`.
Checksum: `nvcsum` = byte sum seeded with 9 (`sys/src/libauthsrv/nvcsum.c`). readnvram CHECKS only machkey/authid/authdom sums (config check commented out, authkey unchecked) and rejects empty authid/authdom (`readnvram.c:299-311`) — the generator should emit all five sums valid regardless. **The nvram `config` field is the secstore password when the client runs `-n`** (secstore.c:408-411) — an optional fifth pairing point.

**/adm/keys** — writeusers layout (`keyfs.c:741-795`): `KEYDBOFF(8)` random bytes, then per user `Namelen(28) name | DESKEYLEN(7) key | status(1) | warnings(1) | expire(4 LE) | SECRETLEN(32) secret`; the whole buffer encrypted with `oldCBCencrypt(authkey=machkey)` = des56to64 + DES-CBC, zero ivec (`keyfs.c:714-724`), where machkey comes from `getkey`: `memmove(authkey, safe.machkey, DESKEYLEN)` (`sys/src/cmd/auth/lib/getauthkey.c:14`). libsec's CBC tail rule (`sys/src/libsec/port/desCBC.c`): full 8-byte blocks CBC-encrypted; a partial tail is XORed with ECB(ivec) — "to be compatible with lacy's cryptlib"; header warns the decryptor must be fed the same buffer size (keyfs's readusers deliberately violates this — `n -= n % KEYDBLEN` at keyfs.c:889 — only the last user's secret tail is affected; harmless for p9sk1 which uses the 7-byte key only). `passtokey` (`sys/src/libauthsrv/passtokey.c`, pure C + DES `encrypt()`) derives each user's 7-byte key from their password — the generator MUST use this exact function.

**/lib/ndb/auth** (speaksfor rules) — consumed by `speaksfor()` (`authsrv.c:791-821`): speaker==user always passes; otherwise `ndbsearch(db, &s, "hostid", speaker)` then scan the tuple's `uid=` attrs: `!user` denies (wins even after an earlier allow), `*` or exact match allows. secstored also opens it (secstored.c:376-381). `db == nil` (file absent) → all hostid≠uid requests fail closed.

**/lib/ndb/local (AUTH-05 rows)** — `authdom=`/`auth=` tuples consumed via cs's generic pair parser (cs.c:1731-1768): `!authdom=DOM auth=*` → `ndbsearch(db, &s, "authdom", DOM)` — straight from ndb, dns never intercepts non-dom/ip keys. Use numeric auth= addresses (v2.0 Pitfall 12(d)); the committed template already carries `sys=harvey` and documents the Phase-18 ipnet= composition point.

### Pattern 1: The nvram= env-var path (D-03, verified end-to-end)
`findnvram` (readnvram.c:160-210): `nvrfile = getenv("nvram")`; if set and non-empty → `openrd(v[0])` (splitting on `!`), `safelen = sizeof(Nvrsafe)`, **`safeoff = 0` unless the path contains "/9fat"** (then a DOS walker hunts plan9.nvr); env `nvrlen`/`nvroff` override. So `nvram=/adm/nvram` + a 109-byte file = a clean non-kernel nvram on any venue. The rc landmine does NOT bite a plain assignment statement (`nvram=/adm/nvram` then the daemon) — bare `=` is only fatal inside words/argv (15-02); keep key=value argv forms quoted or avoided.

### Pattern 2: The interactive-prompt failure map (what wedges a headless gate)
- **keyfs (parent, before rfork)**: `getauthkey` → `readnvram(&safe, 0)` returns −1 silently (flag 0 = no readnvram-side prompts; its own readcons prompts fire only under NVwrite/NVwriteonerr, readnvram.c:315-330) → `print("can't read NVRAM, please enter machine key\n")` + `getpass` (readln opens `/dev/cons` OREAD per line + `/dev/consctl` rawon) — getauthkey.c:20-27. THE gate-assert string; D-02's boot check before keyfs makes it unreachable.
- **auth/debug**: `readcons` prompts (`password for %s@%s [hit enter to skip test]`, raw; `cpu server owner for domain %s [bootes]`, cooked) — debug.c:48-117, 217-218, 270-271, 291-292. All read `/dev/cons` by path → **the 15-02 sentinel-bind drives them**: every prompt's fresh OREAD returns the sentinel's first line (offset resets per open). If all DEMO users share one password, both password prompts succeed; the bootes-owner prompt gets the password string as authid (harmless noise — that leg then fails to decrypt and prints diagnostics; the FIRST test's success line is the probe's green marker).
- **secuser**: password prompts via `getpassm` which opens `/dev/cons` **ORDWR** + `/dev/consctl` and sysfatals if either fails (`util.c:36-48`) — **a read-only sentinel bind over /dev/cons kills secuser** ("couldn't open cons"). Its other prompts (expires/Enabled/STA/comments) read fd 0 (`userinput`, secuser.c:210-227) → stdin-redirectable. So: feed the two passwords over the console (expect-typed — feeding a program's console input is not the typed-rc-script hazard), redirect stdin for the rest.
- **secstore client**: never prompts under `-i` (password [+ optional `\nPIN+STA`] from stdin; failure exits loud: `exits("invalid password on standard input")`, secstore.c:410-446).

### Pattern 3: AUTH-07 for free — syslog's console fallback (verified)
`syslog(cons=0, name, ...)` (libc/9sys/syslog.c:42-115): opens `/sys/log/<name>` OWRITE|OCEXEC; **if that fails (absent or read-only — always, on the paqfs root) `cons` is forced to 1 and the line is written to `#c/cons`** — the real console, unaffected by any `/dev/cons` bind (the `#c` attach bypasses the namespace, namec's nomount arm). Line shape: `<sysname> <ctime+4..+19> <message>` — gates must match on message tokens (e.g. `tr-ok`), never the date. Consequence: do NOT bind a writable /sys/log/auth into the auth image — the fallback IS the serial-visibility mechanism. keyfs adds `%d keys read` (print, fd 1); secstored `-v` stays foreground so its fprint(2) lines stay on the boot console, and its syslog markers (ANNOUNCE/AUTH/[id] op) ride the same fallback. Marker vocabulary for the gate: `tr-ok`, `tr-fail`, `cr-ok`, `cr-fail`, `no /lib/ndb/auth`, `ANNOUNCE tcp!*!5356`(via netmtpt prefix), `AUTH <user>`, `[<user>] PUT/GET/RM`, `<n> keys read`, the pairing banner, `secstore: saved`.

### Pattern 4: the listener topology for tcp/567 (D-09)
authsrv announces nothing itself — it is a stdin/stdout filter started per connection (readn(0, TICKREQLEN) loop; argv last arg = conn dir for `getraddr` `<dir>/remote`). Two in-tree runner options: (a) `aux/listen` (tier-built) announcing staged `/boot/tcpNNNN` service-name copies — **execs the file bare, no flags possible** (the 15-02 tcp9902=authsrv trick); (b) `aux/listen1 [-tv] address cmd args...` — announce + per-conn rfork + `exec(argv[1], argv+1)` **with flags** (listen1.c:84-120). AUTH-07 wants `authsrv -d`; recommend (b): add `cmd/aux:listen1` to tier2 (one leaf line; the aux leaf-filter discipline), run `listen1 -t 'tcp!*!567' /amd64/bin/authsrv -d`. secstored needs no listener (self-contained announce on 5356).

### Pattern 5: r9 substrate insert points (D-05/D-06)
- **Entropy**: implement `rng_seed`/`jitter` fns in the x86_64 crate (RDRAND/RDSEED with the Intel retry-loop guidance when `has_rdrand(cpuid(1).2)`/`has_rdseed(cpuid(7,0).1)` — mirroring cpu.rs's `has_x2apic` helpers; TSC-read jitter mix otherwise), register via `set_random_hooks` in x86_64 main; print the armed source once (the `RANDOM_SEEDED_PRINTED` idiom's neighborhood). The DRBG core, per-read jitter mix, and once-marker stay untouched.
- **devcap**: new `port/src/devcap.rs` following devsrv/devpipe; one `registry_attach` arm `'¤'`; config TOML `dev = [...]` entry + workspace check-cfg predeclare (the declared rule in r9 Cargo.toml). Semantics from the donor (below). Kernel prerequisites this phase must add: HMAC-SHA1 (hand-rolled, safe Rust, the ChaCha20 precedent), per-proc user (Proc field, seeded "glenda", copied at fork, served by /dev/user), eve comparison (devcons HOSTOWNER static vs current user).

### Donor devcap semantics, complete (sys/src/9k/port/devcap.c — the D-06 port target)
```
Hashlen = SHA1dlen (20); Maxhash = 256
Dirtab: "." {Qdir,0,QTDIR} DMDIR|0500 | "capuse" {Quse} 0222 | "caphash" {Qhash} 0200
  ("caphash must be last": capremove on Qhash by eve sets ncapdir = nelem-1, permanently hiding it)
open:  dir → OREAD only (else Ebadarg); Qhash → iseve() else Eperm
write Qhash: iseve() else Eperm; n < Hashlen → Eshort; addcap(first Hashlen bytes)
  addcap: under qlock — trim while nhash >= Maxhash (drop OLDEST, head of list); append at tail; record ticks (never checked — no time-based expiry in this donor)
write Quse: copy n bytes + NUL; key = strrchr(cp, '@') (LAST @) else Eshort; *key++ = 0;
  hmac_sha1(from, strlen(from), key, strlen(key), hash, nil);   /* data="from@to", key=random part */
  remcap(hash) — consume-on-match (unlink + free); nil → error "invalid capability %s@%s";
  to = strchr(from, '@') (FIRST @): nil → to = from (single-user form); else split and
  strcmp(from, up->user) != 0 → error "capability must match user";
  kstrdup(&up->user, to); up->basepri = PriNormal
read: Qdir → devdirread; else Eperm.  remove: eve&&Qhash → hide caphash; else Eperm.
Dev: L'¤', "cap", devreset/devinit/devshutdown, capattach/capwalk/capstat/capopen/devcreate/
     capclose/capread/devbread/capwrite/devbwrite/capremove/devwstat
```
The userspace cap format (as.c:121-166, same code in factotum util.c:592-618): cap string = `"<from>@<to>@<base64 20 rand bytes>"`; hash = HMAC-SHA1(data=`from@to`, key=base64-random); caphash write = the 20 hash bytes; capuse write = the full cap string. **Note the donor table is a FIFO of ≤256 entries, not one-shot-per-post** — D-06's "write-once-per-post" phrasing maps to "each posted hash consumes on first successful match; unmatched entries age out FIFO at 256" — port THIS, not the phrase.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| p9sk1 key derivation | custom password→key | donor `passtokey` (sys/src/libauthsrv/passtokey.c) | byte-exact legacy DES derivation; any deviation = undecryptable tickets, opaque failures |
| /adm/keys encryption | custom CBC | donor `oldCBCencrypt` shape = libsec `desCBCencrypt`/`desCBCdecrypt` (zero ivec, lacy partial-tail rule) | the tail rule and the known readusers length quirk are load-bearing fidelity |
| Nvrsafe checksums | custom checksum | donor `nvcsum` (seed 9) | trivial but must match readnvram's check exactly |
| The p9sk1 wire exchange | raw socket code from memory | donor client pattern: `debug.c authfutz` (Treq → `_asgetticket` → `convM2T`, check `t.num==AuthTc`, chal compare) + `libauthsrv` codecs | the protocol constants/enums are in authsrv.h; debug.c is the working reference implementation |
| speaksfor probe leg | hoping auth/debug has a flag | a ~60-line probe client extending debug.c's authfutz with `tr.hostid≠tr.uid` (host-side 6c/6l, the hsvctcpecho precedent), or factotum role=speakfor | verified: debug.c has no flag and always self-speaks; libauthsrv gives everything needed |
| HMAC-SHA1 in r9 | a new crate | hand-rolled safe-Rust SHA-1 + HMAC (RFC 3174/2104 shape) in port, beside the hand-rolled ChaCha20 | the no-new-crate kernel discipline (devcons.rs:939); SHA-1 here is the donor cap-format contract, not a crypto choice |
| entropy gating | runtime guessing | the 13-03 CPUID-gated dual-mode helper pattern (`has_x2apic` → `has_rdrand`) | KVM/TCG divergence is a CPUID fact, not a venue flag |
| guest writable dirs | new scratch server | the established scratch/mount-point machinery (mntgen for mount points; verify file-write venue per Assumptions A4) | one venue per class, already sweep-proven |
| bake asserts | ad-hoc greps | the mkimage assert families (tier-entry presence, staged==source, literal grep with negative controls) | teeth proven by doctored-input controls in 15-08/16-02 |

**Key insight:** every byte format in this phase is a legacy donor contract (DES-CBC keys file, Nvrsafe, cap hash format). The failure mode of hand-rolling any of them is not a crash — it is an opaque `p9sk1: authentication failed` weeks later (v2.0 Pitfall 12). Compile the donor's own code wherever possible.

## Common Pitfalls

### Pitfall 1: The silent interactive wedge behind a missing/invalid nvram
**What goes wrong:** keyfs never starts; the boot appears hung with `can't read NVRAM, please enter machine key` (or `readnvram: can't find nvram`, or `bad authentication password/id/domain`, or `empty nvram authid or authdom`) sitting on serial behind a raw-mode getpass.
**Why:** `getauthkey` escalates readnvram failure into `getpass` on `/dev/cons` (getauthkey.c:24-25); readnvram's OWN prompts only fire under NVwrite/NVwriteonerr (readnvram.c:315) — flag 0 never prompts, but the nvtab fallback path (cputype `pc` rows → `#S/sdC0/nvram` etc., readnvram.c:28-56) fails on this venue and errors out.
**How to avoid:** D-02's boot pairing check runs BEFORE keyfs and aborts loud; the gate asserts the prompt strings NEVER appear (`can't read NVRAM, please enter machine key` is the exact marker). Bake the nvram file with valid sums so the happy path never consults nvtab.
**Warning signs:** serial stops after the pairing banner; `%d keys read` never appears.

### Pitfall 2: Stock auth/debug cannot perform the speaksfor leg
**What goes wrong:** the plan assumes `auth/debug -<flag>` can request as-user-for-another; it cannot.
**Why:** verified — `usage: auth/debug` (no flags, debug.c:43); the ticket request always sets `tr.hostid = user; tr.uid = user` (debug.c:233-234); the speaksfor wire form (`hostid≠uid`) only exists in factotum's client (p9sk1.c:260-264) and authsrv's server-side gate (authsrv.c:130-138).
**How to avoid:** build the leg-2 probe client (host-side 6c/6l into the image, the hsvctcpecho/hmkdir precedent at mkimage.sh:954-966), modeled on debug.c's `authfutz` with `tr.hostid = <speaker>`, `tr.uid = <third user>`, asserting `t.num == AuthTc && t.suid == <third user>`; negative control = `uid=!<denied>` row → decrypt must fail. Alternatively drive factotum with a `role=speakfor` key — but that is closer to the factotum-as-client machinery D-13 excluded; the C probe is the cleaner read of the decision.
**Warning signs:** none at plan time — this is a plan-time verification result, already done.

### Pitfall 3: auth/debug's hidden dependencies — factotum mount AND cs
**What goes wrong:** `auth/debug` prints `debug: cannot open /mnt/factotum/ctl` and exits, or authdial reports `no auth server found for <dom>`.
**Why:** `main` → `debugfactotumkeys()` reads `/mnt/factotum/ctl` (debug.c:131) — no factotum, no probe; `authdial` → `csgetvalue(nil, "authdom", dom, "auth", nil)` opens `/net/cs` and returns nil on failure with NO ndb fallback (csgetval.c:30-33); `dial("tcp!$auth!ticket")` also consults cs.
**How to avoid:** the bring-up starts factotum (`-s factotum`, sweep-proven), writes the p9sk1 key to its ctl (`key proto=p9sk1 user=<third> dom=<DOM> !password=<pw>` — role attrs are not needed for the ctl listing), starts cs over the baked `/lib/ndb/local` (loopback `bind -a '#I' /net` + ipifc first, `hcs -f` shape from the sweep), and stages `authdom=<DOM> auth=127.0.0.1` rows (numeric — Pitfall 12(d)).
**Warning signs:** `no auth server found for`, `cannot dial auth server`.

### Pitfall 4: The zero clock poisons failure counters, not the happy path
**What goes wrong:** after ~10 failed attempts (negative controls, mistyped feeds), a user becomes unreachable with `user in purgatory` — FOREVER, since time never advances past it.
**Why:** r9 has no RTC (`time(0)==0` era — STATE 17-05: `hdate -n == 0`, no /dev/bintime); keyfs's Qlog write does `if(f->user->bad && bad%MAXBAD == 0) purgatory = time(0) + bad` → `purgatory=10 > now=0` permanently (keyfs.c:570-571); Read's Qkey gate refuses while `purgatory > time(0)` (keyfs.c:434). secstore's `failed >= 10` lockout reads `mtimePW(id)+300` similarly (password.c:100-108). NOTE the happy paths are safe: expire checks pass (`expire != 0 && expire <= now` is false for any positive expire at now=0 — secuser's default `time(0)+365*24*60*60 = 31536000` works verbatim; keyfs expire=0/"never" recommended), and keyfs never time-expires users with expire=0.
**How to avoid:** generator sets expire=0; negative controls use ONE wrong attempt per user (assert the failure line), never loops; the failed-login counters reset on `"good"` writes (log.c:32-38).
**Warning signs:** `user in purgatory`, `too many failures; try again in five minutes`.

### Pitfall 5: secuser is not sentinel-drivable (ORDRW console open)
**What goes wrong:** `secuser` sysfatals `couldn't open cons` when the 15-02 read-only sentinel is bound over /dev/cons.
**Why:** `getpassm` opens `/dev/cons` ORDWR + `/dev/consctl` and treats failure as fatal (util.c:42-48) — unlike readln (OREAD + OWRITE separately, OWRITE falls back to fd 1).
**How to avoid:** run secuser OUTSIDE any sentinel window; feed the two password prompts over the console (expect sends after matching the prompt text — feeding a program's console prompts is the expect gate's native job, distinct from the typed-rc-script hazard) and redirect fd 0 from a feed file for the expires/Enabled/STA/comments prompts (`secuser -v <user> < /tmp/feed`). Passwords must be ≥7 chars (secuser.c:84-86).
**Warning signs:** `couldn't open cons`, `password must be at least 7 characters`.

### Pitfall 6: Killing AUTH-07 by creating a writable /sys/log
**What goes wrong:** auth lines vanish from serial — they land in a file nobody tails.
**Why:** syslog prefers `/sys/log/<name>` whenever it opens (syslog.c:107-110); the console fallback only fires when the open FAILS.
**How to avoid:** do not stage or bind a writable `/sys/log/auth` or `/sys/log/secstore`. If a file venue is ever wanted, it must come with an explicit tail-to-console — the default design keeps the fallback.
**Warning signs:** gate expects `tr-ok` on serial but the pairing banner was the last line.

### Pitfall 7: The staged-copy listener cannot pass -d
**What goes wrong:** authsrv runs without debug; `tr-ok` lines never appear; AUTH-07 regresses to start-only evidence.
**Why:** aux/listen execs the service-name file bare (the 15-02 tcp9902 shape); argv flags have no carrier.
**How to avoid:** use `aux/listen1 -t 'tcp!*!567' /amd64/bin/authsrv -d` (flags ride via exec(argv[1], argv+1)); listen1 is in-tree but NOT tier-built — add the `cmd/aux:listen1` leaf line first (one-line tier-data change, the aux leaf-filter discipline).
**Warning signs:** connections succeed but no per-ticket log lines.

### Pitfall 8: rc dialect + the nvram env var
**What goes wrong:** `nvram=/adm/nvram`-shaped WORDS in argv (e.g. `somecmd nvram=/adm/nvram`) hit the bare-`=` lexer error and kill a sourced file; braced if-not bodies kill the script mid-stream.
**Why:** 15-02 measured rc landmines (STATE).
**How to avoid:** plain assignment statements are SAFE (nvram=/adm/nvram on its own line, then the daemon inherits the env — verified semantics: `getenv("nvram")` reads the environment, not argv); flat status ladders; no `=` inside words; sourced-not-typed scripts. Note keyfs's rfork carries RFENVG — the child sees a COPY of the env at fork time, so set nvram BEFORE starting keyfs (assignments in the same script satisfy this).
**Warning signs:** `token '=...': syntax error` on serial.

### Pitfall 9: Pairing vs byte-reproducibility confusion in gates
**What goes wrong:** a gate (or a reviewer) demands two bakes produce identical auth images.
**Why:** D-01's accepted consequence — ephemeral DEMO material differs per bake; the pairing manifest is an INTERNAL-consistency proof (all artifacts from one run, cross-hashed), never cross-build identity.
**How to avoid:** gates assert manifest↔artifact agreement (bake) + banner (boot) + probe markers; no gate ever hashes bootfs-auth.paq across builds; the r9 FOD re-wire applies ONLY to the kernel-byte pass (random + devcap), once, per the carried-forward rule.
**Warning signs:** any "auth image digest" appearing in a plan.

### Pitfall 10: mkpaqfs silent drop + KERNDATE
**What goes wrong:** a staged file silently missing from the paq (15-07 standing hazard) or unexpected digest churn.
**Why:** mkpaqfs drops files past blocksize²/4 with a warn and exit 0; auth artifacts are tiny (no size risk) but the bake must still fail on ANY mkpaqfs diagnostic; mtimes are KERNDATE-pinned (deterministic layer) while CONTENT varies per bake (ephemeral keys) — both true at once, by design.
**How to avoid:** keep the fail-on-any-diagnostic assert pattern; keep KERNDATE pinning for the deterministic layers.
**Warning signs:** missing-file errors only in-guest.

### Pitfall 11: The speaksfor ndb rows are a distinct file from /lib/ndb/local
**What goes wrong:** AUTH-05's ndb staged, probe leg 1 green, leg 2 fails with tickets that never decrypt.
**Why:** authsrv consults `/lib/ndb/auth` (separate file — authsrv.c:60, secstored.c:376) for `hostid=` speaksfor rules; missing file = all hostid≠uid requests fail CLOSED (speaksfor returns 0, keys scrambled silently — only `-d` shows `tr-fail ... no speaks for`).
**How to avoid:** the generator's "ndb entries" artifact covers BOTH files; the manifest pairs both; negative controls assert the `tr-fail` line appears (proving the debug path is armed).
**Warning signs:** leg-2 decrypt failures with leg-1 green.

### Pitfall 12: TCG red after KVM green on the entropy leg is real
**What goes wrong:** KVM-local gate green; hosted CI (R9_ACCEL=tcg, `--cpu qemu64`) fails the random marker.
**Why:** qemu64 exposes no RDRAND (verified) — the jitter path is load-bearing in CI; a TCG-only red is a substrate bug (jitter mix degenerate under TCG timing), not gate noise (CONTEXT watch-out).
**How to avoid:** host `cfg(test)` pins for the mixing logic; the boot profile asserts the armed-source line prints on both legs (`random: source=rdrand` vs `random: source=jitter` — exact vocabulary planner's).
**Warning signs:** `random seeded` without an armed-source line.

## Code Examples

### Generator core: constructing /adm/keys with donor code (host C, plan9port-compiled)
```c
/* Sources: sys/src/cmd/auth/keyfs.c:741-795 (writeusers layout), :714-724 (oldCBCencrypt),
   sys/src/libauthsrv/passtokey.c, sys/src/libauthsrv/nvcsum.c, sys/include/authsrv.h:15-30 */
/* Record: Namelen(28) name | DESKEYLEN(7) key | status(1) | warnings(1) | expire(4 LE) | SECRETLEN(32) */
/* status: Sok; expire 0 = "never" (zero-clock-safe, keyfs.c:436); secret NUL-terminated */
/* Whole buffer: KEYDBOFF(8) random + nu*73, DES-CBC(machkey as des56to64 key, zero ivec) */
/* The tail rule is libsec's desCBC.c partial-block XOR-with-ECB(ivec) — compile the donor file. */

ulong buflen = KEYDBOFF + nu*KEYDBLEN;
uchar *buf = malloc(buflen);
randombytes(buf, KEYDBOFF);			/* host randomness, not the guest DRBG */
uchar *p = buf + KEYDBOFF;
for(each user u){
	strncpy((char*)p, u->name, Namelen); p += Namelen;
	passtokey(p, u->password); p += DESKEYLEN;	/* donor derivation — never hand-roll */
	*p++ = Sok;					/* status */
	*p++ = 0;					/* warnings */
	ulong expire = 0;				/* "never" */
	*p++ = expire; *p++ = expire>>8; *p++ = expire>>16; *p++ = expire>>24;
	memset(p, 0, SECRETLEN); p += SECRETLEN;	/* secret unused by p9sk1 */
}
oldCBCencrypt(nvrmachkey, buf, p - buf);		/* donor fn shape, keyfs.c:714 */
write(outfd, buf, p - buf);
```

### Generator: the Nvrsafe nvram-substitute (109 bytes)
```c
/* Source: sys/include/authsrv.h:140-157 (layout), sys/src/libauthsrv/nvcsum.c (sum, seed 9) */
Nvrsafe s;
memset(&s, 0, sizeof s);
passtokey(s.machkey, machine_password);		/* THE /adm/keys encryption key (getauthkey.c:14) */
strecpy(s.config, s.config+sizeof s.config, secstore_password); /* optional: client -n pairing */
strecpy(s.authid, s.authid+sizeof s.authid, "bootes");	/* AUTH-05 hostid owner */
strecpy(s.authdom, s.authdom+sizeof s.authdom, authdom);	/* carries the DEMO label (D-04) */
s.machsum = nvcsum(s.machkey, DESKEYLEN);
s.authsum = nvcsum(s.authkey, DESKEYLEN);
s.configsum = nvcsum(s.config, CONFIGLEN);
s.authidsum = nvcsum(s.authid, sizeof s.authid);
s.authdomsum = nvcsum(s.authdom, sizeof s.authdom);
write(fd, &s, sizeof s);			/* exactly 109 bytes at offset 0 (readnvram.c:188-190) */
```

### The speaksfor ndb rows (AUTH-05; /lib/ndb/auth)
```
# Mechanism verified: authsrv.c:791-821 — ndbsearch "hostid" <speaker>; uid=!x denies, uid=* / uid=name allows
# The exact donor example line is [ASSUMED] formatting (mechanism verified, example composed):
hostid=bootes uid=!sys uid=!adm uid=*
```

### The speaksfor probe client (leg 2) — the delta from debug.c's authfutz
```c
/* Sources: sys/src/cmd/auth/debug.c:209-267 (authfutz), sys/include/authsrv.h:33-67 (wire structs) */
Ticketreq tr;
tr.type = AuthTreq;
strecpy(tr.authid, tr.authid+sizeof tr.authid, "bootes");	/* server key holder */
strecpy(tr.authdom, tr.authdom+sizeof tr.authdom, dom);
strecpy(tr.hostid, tr.hostid+sizeof tr.hostid, "bootes");	/* speaker — DIFFERS from uid: */
strecpy(tr.uid,   tr.uid+sizeof tr.uid,       thirduser);	/* the speaks-for form (p9sk1.c:260-264) */
memset(tr.chal, 0xAA, sizeof tr.chal);
convTR2M(&tr, trbuf);
fd = dial(netmkaddr(authsrv_addr, nil, "ticket"), 0, 0, 0);
_asgetticket(fd, trbuf, tbuf);
convM2T(tbuf, &t, booteskey);			/* decrypt with the SPEAKER's key */
if(t.num != AuthTc || strcmp(t.cuid, thirduser) != 0 || strcmp(t.suid, thirduser) != 0)
	fatal("speaksfor rejected");		/* authsrv scrambled keys (authsrv.c:130-138) */
print("authprobe: SPEAKSFOR-OK %s for %s\n", "bootes", thirduser);
/* Negative control: request uid=sys (uid=!sys row) -> convM2T yields t.num != AuthTc */
```

### Boot env + bring-up skeleton (rc, sourced; flat status ladder)
```rc
# nvram= is a plain assignment — safe (getenv reads it); never put nvram=... inside an argv word
nvram=/adm/nvram
# D-02 boot arm BEFORE keyfs: verify pairing, print label, else abort loud
rc /lib/authpaircheck.rc			# prints AUTHDOM-<label> banner or exits loud
keyfs -m /mnt/keys /adm/keys			# NO -p (nvram path); prints "N keys read"
listen1 -t tcp!*!567 /amd64/bin/authsrv -d &	# tr-ok/tr-fail to #c/cons via syslog fallback
secstored -v &					# tcp!*!5356, stays foreground-verbose in background rc &
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| r9 /dev/random: DTB /chosen/rng-seed + CNTFRQ/CNTPCT jitter (aarch64 D-17) | x86_64: no DTB — CPUID-gated RDRAND/RDSEED + TSC/jitter mix via RandomHooks (D-05) | this phase | hooks registered on x86_64 for the first time (today: deterministic seed, devcons.rs None-arm) |
| devcap absent from r9 ("no devcap, no auth adjacency" — devcons.rs:933 comment) | donor-verbatim port with per-proc user + eve + hand-rolled HMAC-SHA1 (D-06) | this phase | unblocks Phase 19 auth_chuid; first r9 kernel uid mutation |
| 15-02 keyfs probe: sentinel-bind + throwaway keyfile + getpass-derived key | real baked domain: nvram= env + paired /adm/keys (D-03) | this phase | keyfs start becomes non-interactive by construction |
| 15-02 authsrv: staged-copy under listener, connect-accept only | listen1 with -d, real p9sk1 exchanges, serial-visible logging | this phase | AUTH-04/06/07 satisfied on the same substrate |
| v2.0 plan: "auth first-boot chicken-and-egg" (Pitfall 11) | bake-time hash-paired generator (D-01/D-02) | this phase | the four configs agree by construction |

**Deprecated/outdated within this phase's scope:** none (the phase deliberately does NOT modernize crypto — dp9ik/TLS is the future hardening milestone, REQUIREMENTS Out of Scope).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | plan9port (nixpkgs, already a gate runtimeInput) links `-lsec`/libauthsrv for a host generator build (`9c`); setup.sh proves `9c`→pcc exists but lib linking in the nix env is unverified | Standard Stack / Summary | generator venue shifts to vendored donor .c files with a gcc shim (passtokey + des.c + desCBC.c + nvcsum are self-contained; small); Wave-0 probe recommended |
| A2 | The exact donor example line `hostid=bootes uid=!sys uid=!adm uid=*` (format of /lib/ndb/auth rows) — mechanism verified against speaksfor(); the literal example is composed from convention, not read from a donor file (sys/man/8/authsrv does not exist in this tree) | Code Examples | cosmetic; the generator emits rows the verified ndbsearch path consumes — validate with the negative control anyway |
| A3 | A writable-venue server for /adm/secstore file writes exists in the current image vocabulary (mntgen serves mount-point walk-creates per the sweep; 15-06 measured mntgen READ-ONLY for file writes and named ramfs for /tmp — ramfs is NOT in tier data). Which binary serves file-writable dirs in the auth image is unverified | Patterns / Supporting | probe may need `cmd/ramfs` added to tier data (one leaf line) or a copy-before-start scheme; Wave-0 in-guest probe recommended |
| A4 | auth/debug's sentinel-driven bootes leg noise (password string consumed as the "cpu server owner" answer) is acceptable to the gate design — success marker is leg 1's line | Pitfalls | cosmetic; gate marker choice is planner discretion (D-10) |
| A5 | `listen1 -t` semantics: `-t` is the trusted/no-lookup arm (usage string verified; exact flag meaning not read to the end of the source) | Pattern 4 | planner reads listen1.c fully before pinning the invocation (one file, small) |
| A6 | TCG CI runners of this project use `--cpu qemu64` without `+rdrand` (committed 13-06/16-06 flag chain; qemu docs verified for the general qemu64 default) — so jitter is load-bearing in CI | Pitfalls #12 | if CI ever adds +rdrand, the RDRAND leg also gets CI coverage — strictly better, no risk |
| A7 | Third user name, DEMO authdom token spelling, local-file format/name, marker vocabulary — planner discretion per CONTEXT; research proposes none as locked | throughout | none (explicitly discretionary) |

## Open Questions (RESOLVED)

All four questions carry executable in-plan resolutions (annotated 2026-11-13 at plan revision — none remains open; pointers name the resolving plan task):

1. **Generator host-build venue** — plan9port libsec/libauthsrv linking vs vendoring the four donor algorithm files?
   - What we know: setup.sh installs plan9port `9c` as pcc; plan9port ships libsec/libauthsrv upstream; the needed code (passtokey, des, desCBC, nvcsum) is small and self-contained.
   - What's unclear: whether nixpkgs' plan9port builds/links those libs in the mkimage environment.
   - Recommendation: Wave-0 probe compiles a 20-line host C with 9c linking -lauthsrv -lsec; fallback = vendored files (gcc or 9c). Either way the ALGORITHM code is the donor's, not a reimplementation (A1).
   - RESOLVED: 18-03 Task 1 runs the Wave-0 A1 probe (9c link test against -lauthsrv -lsec) with the vendored-donor-files fallback pinned inside the same task; the probe verdict is recorded in the 18-03 SUMMARY.
2. **Writable /adm/secstore venue** — mntgen (mount-points, read-only files per 15-06) vs ramfs (not in tier data) vs copy-before-start?
   - Recommendation: decide by a 10-minute in-guest probe during Wave 0 of the auth image work (create/write/read a file under each candidate); the sweep already proves mntgen mount-points.
   - RESOLVED: 18-05 Task 2 owns the in-guest venue probe (create/write/read under the mntgen-served /adm/secstore; add the `cmd/ramfs` tier leaf and serve the dir from ramfs if file writes fail); the verdict + evidence line land in the 18-05 SUMMARY.
3. **Does the auth image run cs from /lib/ndb/local directly (`-f` default) or a composed file?**
   - The sweep used `-f /env/<file>`; the auth domain wants the REAL baked local. cs's default db is /lib/ndb/local; verify cs -f accepts the baked path or run default. Trivial, in-plan.
   - RESOLVED: 18-04 Task 3 pins cs running with its DEFAULT db — the baked /lib/ndb/local — no `-f` (recorded in the task's read_first and its sweep leg 3).
4. **The `ether=` ndb value's single source** (AUTH-05): qemu's default NIC MAC (52:54:00:12:34:56 when unspecified) is a venue literal — it belongs in netaddr.rc (new ether key) or in the r9 TOML netdev stanza, per the single-source rule. Planner picks the home; the seam grep enforces it.
   - RESOLVED: 18-04 Task 3 pins netaddr.rc as the single home (new `ether=` key carrying the qemu-default MAC, documented in a comment) with `ether=@ETHER@` in local.template; the venue-literal grep assert enforces it.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qemu-system-x86_64 | gates, boot profiles | ✓ (host) | 11.1.2 | — |
| KVM + host CPU RDRAND/RDSEED | D-05 KVM leg | ✓ (rdrand+rdseed ×16 CPUs, /proc/cpuinfo) | — | TCG jitter leg (CI default) |
| r9 checkout at ../r9, branch main | kernel substrate | ✓ @ 8f3ff11 (== STATE's Phase-17 terminus) | — | — |
| host cross toolchain 6c/6l | probe-client + host-side builds | ✓ `.build/hostbin-linux-x86_64/bin/{6c,6l}` | — | `nix run .#setup && nix run .#toolchain` (mkimage's named remediation) |
| mkpaqfs | image bake | ✓ `.build/bin/mkpaqfs` (mkimage.sh var) | — | — |
| plan9port | generator host build (A1) | ✓ as nix runtimeInput; 9c as pcc in setup | nixpkgs pin | vendored donor files + gcc |
| expect | auth gate driver | ✓ (gate runtimeInputs) | — | — |
| nix flake app plumbing | auth gate app | ✓ (mkScriptApp, R9_* export shape at flake.nix:237-244) | — | — |
| CI hosted runner (TCG, KVM-less) | auth gate CI leg | ✓ (r9-shell.yml net-smoke-x64 precedent) | — | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** plan9port lib linking (A1) — vendored-files fallback named above.

## Security Domain

`security_enforcement` absent in `.planning/config.json` → enabled → section included.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | p9sk1 (donor protocol, PROTOCOL-locked by milestone scope: dp9ik/TLS explicitly Out of Scope); DES keys via passtokey — legacy by design, hardened post-v2.1 |
| V3 Session Management | no (this phase) | tickets/authenticators exist but session consumption (auth_chuid) is Phase 19 |
| V4 Access Control | yes | devcap hostowner gate (iseve), speaksfor fail-closed rules (`!uid` deny-wins), keyfs Qhash/Qlog perms (0200/0222) — all donor-verbatim |
| V5 Input Validation | yes | keyfs `userok` rune/control-char/space-slash rejection (keyfs.c:822-855); secstore `validatefile` (no path chars, ≤250); capuse length/parsing errors (Eshort, invalid capability) |
| V6 Cryptography | yes — legacy, deliberately | DES-CBC (keys file), DES p9sk1, SHA-1 HMAC (devcap) — DONOR CONTRACTS, do not "upgrade" any of them in this phase (that breaks interop with the donor userspace and Phase 19); ChaCha20 DRBG (r9 random) is the modern piece |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| DEMO/private key material leaking into git | Information Disclosure | D-01 gitignored local file + committed interface only; bake uses ephemeral DEMO on absence; never commit the manifest of a real domain |
| nvram machine key (= /adm/keys master key) exposed in image | Information Disclosure | the DEMO domain's keys are throwaway by construction; the image is per-gate, never published; real domains ride the documented host-9P /adm path (AUTH-09, not wired here) |
| authsrv anti-enumeration | Spoofing | donor behavior retained: unknown authid/hostid get random fabricated keys (`mkkey`) so callers can't distinguish (authsrv.c:114-125) — keep it in any probe negative controls |
| capuse replay | Elevation of Privilege | donor consume-on-match + FIFO aging (devcap.c remcap/addcap) — port exactly; no time-based expiry exists in the donor (ticks recorded, never checked) — do not invent one |
| secstore account lockout abuse | DoS | `failed>=10` + 5-min mtime window; zero-clock caveat (Pitfall 4) — negative controls avoid multi-failure loops |
| speaksfor escalation | Elevation of Privilege | `!uid` deny-wins ordering (authsrv.c:811-815); fail-closed when /lib/ndb/auth absent; negative control asserted in the gate |

## Sources

### Primary (HIGH confidence — direct file reads this session)
- `sys/src/libauthsrv/readnvram.c` — nvram= env path (getenv at :161), safeoff-0 (:188-190), checksum checks (:299-311), prompt classes (:315-330), nvtab
- `sys/src/cmd/auth/lib/getauthkey.c` — machkey extraction (:14), interactive fallback (:24-25); `lib/readln.c` — readln cons discipline; `lib/log.c` — Qlog writes; `lib/fs.c` — KEYDB/NETKEYDB paths; `authcmdlib.h`
- `sys/src/cmd/auth/keyfs.c` — flags (:135), main/rfork (:139-192), Write/Qlog (:509-585), writeusers/readusers (:740-917), oldCBC (:714-738)
- `sys/src/cmd/auth/authsrv.c` — -d (:49-52), ticketrequest (:106-158), speaksfor (:791-821), replyerror/tickauthreply/getraddr/mkkey (:827-914)
- `sys/src/cmd/auth/debug.c` — complete (no flags; factotum ctl enumeration; authfutz; hostid=uid)
- `sys/src/cmd/auth/as.c` — mkcap/usecap (caphash/capuse userspace contract)
- `sys/src/9k/port/devcap.c` — complete donor; legacy twin diff (capstat only)
- `sys/src/cmd/auth/secstore/{secstore.h,password.c,util.c,secuser.c,secstored.c,secstore.c}` — SECSTORE_DIR, PW format, getpassm, flags, PAK/login flow
- `sys/src/cmd/auth/changeuser.c` — mounted-tree user creation semantics
- `sys/src/cmd/auth/factotum/p9sk1.c` — speakfor client form (:206-297); `factotum/util.c` — mkcap/caphash/writehostowner
- `sys/include/authsrv.h` — constants (:15-30), wire structs (:58-95), Nvrsafe (:140-157)
- `sys/src/libc/9sys/syslog.c` — console fallback mechanism
- `sys/src/libndb/csgetval.c` — /net/cs requirement; `sys/src/cmd/ndb/cs.c` — generic pair parser (:1709-1768)
- `sys/src/libsec/port/desCBC.c` — lacy tail rule; `sys/src/libauthsrv/{passtokey,nvcsum}.c`
- `sys/src/cmd/aux/listen1.c` — usage/exec shape
- r9: `port/src/devcons.rs` (random section 925-1156, HOSTOWNER, USER_CONTENT), `port/src/namec.rs` (registry_attach, first_rune), `x86_64/src/cpu.rs` (cpuid/rdtsc/helpers), `x86_64/src/proc.rs:978` (Proc), `port/Cargo.toml`, `x86_64/lib/config_q35.toml` (hostfwd data), `Cargo.toml` (check-cfg rule), git @ 8f3ff11
- Harvey build: `build/x86_64/mkimage.sh` (services contract 762-1000, host 6c/6l builds), `tiers/tier2-services.txt` / `tier1-cmds.txt`, `services-termrc` / `services-sweep.rc` (mntgen/loopback/cs/keyfs/authsrv probes), `root-tree/lib/netaddr.rc`, `root-tree/lib/ndb/local.template`, `root-tree/rc/bin/termrc`, `flake.nix` (:237-244), `nix/smoke-r9-net-x64.sh`, `nix/setup.sh`, `.github/workflows/r9-shell.yml`
- `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md` §18, `.planning/STATE.md` (zero-clock fact, rc landmines, sweep findings)

### Secondary (MEDIUM confidence — official docs via WebSearch)
- [Intel DRNG Software Implementation Guide](https://www.intel.com/content/www/us/en/developer/articles/guide/intel-digital-random-number-generator-drng-software-implementation-guide.html) — CPUID.1:ECX[30] RDRAND, CPUID.(7,0):EBX[18] RDSEED, retry guidance
- [QEMU i386 CPU configuration](https://www.qemu.org/docs/master/system/i386/cpu.html) + [QEMU wiki CPUModels](https://wiki.qemu.org/Features/CPUModels) — qemu64 minimal baseline, no RDRAND; TCG supports it when enabled; KVM -cpu host passthrough

### Tertiary (LOW confidence)
- None — no claim in this document rests on an unverified web source. Graph context (stale, 291 commits) contributed nothing load-bearing.

## Metadata

**Confidence breakdown:**
- Donor daemon semantics/formats: HIGH — every load-bearing format and flag read verbatim from source this session, with line cites
- r9 substrate insert points: HIGH — seams (RandomHooks, registry_attach, cpu.rs, Proc, HOSTOWNER) read directly; gaps (per-proc user, eve, HMAC-SHA1) verified absent, not assumed
- Probe strategy: HIGH on mechanics (factotum/cs dependencies, sentinel applicability, speaksfor wire form), MEDIUM on composition choices (planner discretion items A4/A7)
- Pitfalls: HIGH — each traced to a specific verified code path or a STATE-recorded measurement

**Research date:** 2026-11-13
**Valid until:** 2026-12-13 (stable donor code; revisit only if r9 HEAD moves past this phase's own commits)


