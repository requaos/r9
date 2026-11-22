---
phase: 18-in-guest-p9sk1-auth-domain
plan: 03
subsystem: auth (p9sk1 domain generation, host build tooling)
tags: [p9sk1, authdomain, nvram, Nvrsafe, keyfs, DES-CBC, passtokey, sha1, donor-code, hash-paired-manifest, host-cc, AUTH-01]

requires:
  - phase: 17-r9-kernel-flush-out
    provides: the x86_64 q35 venue and build/x86_64 tool family the generator joins (svctcpecho host-C harness style)
provides:
  - authdomain-gen with -o/-f/-V/-i modes — the one-run producer of the hash-paired unit (nvram 109B Nvrsafe, keys 8+73/user DES-CBC under machkey, ndb-auth speaksfor rows, sha1 manifest with label row)
  - build/x86_64/authdom-donors/ — byte-verbatim Harvey donor algorithm files + thin host-cc shim; the generator's crypto is the donor's own code, compiled not rewritten
  - build/x86_64/authdomain.example — the committed interface (label/authdom/machine_password/secstore_password + user lines); /authdomain.local gitignored (D-01)
  - authdomain-gen -V verify semantics (all sha1 rows recomputed, five Nvrsafe sums, keys length vs users row, label token in authdom) — the SAME semantics 18-04's bake arm and boot arm consume
  - authdomain-gen -i info mode — parsed fields + decrypted user names (the pairing identity, inspectable without binary tools)
  - build/x86_64/authdomain-selftest.sh — the committed CI-able proof (DEMO + local-file paths, four doctored-input negative controls, label asserts, zero repo-tree writes, GEN-SELFTEST-OK terminal)
affects: [18-04 mkimage auth contract, 18-05 probe legs, keyfs non-interactive start, AUTH-01]

actuals:
  tokens: 24269   # chars/4 over the three production commits' realized diff (estimate was 40000 — the same over-projection family this project always records)
  tasks: 3
  commits: 4      # 3 production + this SUMMARY; plan_head_before cbd60f46beea6f9ab1397c7a8788992c09074f10 — rev-list counts from that base include the sibling 18-01 SUMMARY commit (parallel wave), excluded here by ownership
  plan_head_before: cbd60f46beea6f9ab1397c7a8788992c09074f10

tech-stack:
  added: []        # no new stack: host cc (cc/gcc) over vendored portable C — no packages, no nix changes
  patterns:
    - "vendored-donor-algorithm build: byte-verbatim in-tree crypto sources + thin shim headers under a dedicated donors dir, compiled by the host cc (donor bytes, any shell)"
    - "generate-time decrypt round-trip: every run re-decrypts the keys buffer and re-derives each user key with donor code — algorithm drift aborts at generate, never as an opaque p9sk1 failure weeks later"
    - "per-class doctored-input markers: each verify-refusal class prints its own NEG-CONTROL-REFUSED-<class> receipt (the 15-08 teeth discipline)"

key-files:
  created:
    - build/x86_64/authdomain-gen.c
    - build/x86_64/authdomain.example
    - build/x86_64/authdomain-selftest.sh
    - build/x86_64/authdom-donors/   # 17 files: 11 verbatim donor .c/.h + libsec.h/authsrv.h/os.h verbatim + 4 shim headers (u.h, libc.h, mp.h, auth.h)
  modified:
    - .gitignore    # /authdomain.local

key-decisions:
  - "Wave-0 A1 venue verdict: plan9port 9c/9l linking -lauthsrv -lsec SUCCEEDED in the nix devshell (A1-PROBE-OK), but the vendored venue was chosen — the must-have pins the algorithm bytes to THIS tree's donor files (plan9port's prebuilt libs are a moving 9fans descendant), and the committed self-test must run under the bare host shell exactly as the plan's own verify commands invoke it; cross-validated: the vendored Harvey donors under host cc reproduce plan9port's linked output byte-for-byte on a fixed vector (passtokey key=ed1d, desCBC buf0=36)"
  - "Shim ulong keeps HOST width (glibc already typedefs the name; Plan 9 long is 32-bit): proven harmless for the vendored set — DES does all arithmetic in u32int locals with explicit masks, SHA-1 counts in uvlong; plan9port makes the same LP64 choice"
  - "-i info mode decrypts the keys records and prints the user names (beyond the plan's letter): the machkey sits in the nvram artifact, so info doubles as a live proof of the pairing identity"
  - "nvram authkey emitted as zeros with a VALID sum — no consumer this era (getauthkey.c:14 reads machkey only; readnvram never checks authsum), documented in place"
  - "authdom-donors/ grew past the plan's named four files: strecpy.c and hmac.c joined (sha1.c's hmac_sha1 needs hmac_x; strecpy is the donor field-copy idiom), libsec.h and authsrv.h vendored WHOLE (zero curation drift), os.h per the libmp donor shape"
  - "DEMO defaults pinned: label DEMO, authdom DEMO-harvey, users glenda/bootes/norbert sharing one 12-char host-random password (>=7 secuser-safe, <=13 config-field-safe, rc-safe alphabet)"

patterns-established:
  - "Donor-crypto-not-reimplemented has a buildable shape now: vendored byte-verbatim sources + shim headers, one cc invocation, no nix dependency — reusable for any future host-side crypto need"
  - "The manifest format: `label <t>` / `users <n>` / `<sha1>  <name>` rows, strictly three artifact rows, verified by recompute — 18-04's bake and boot arms reuse these exact semantics"

requirements-completed: [AUTH-01]   # the generator half — AUTH-01's bake/boot arms are 18-04's

coverage:
  - id: D1
    description: "authdomain-gen produces the hash-paired unit from one run (nvram 109B / keys 8+3*73 / ndb-auth / manifest with label row) on the ephemeral DEMO path"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "sh build/x86_64/authdomain-selftest.sh: DEMO leg — generate exit 0, verify 'OK dir=demo label=DEMO users=3', byte asserts 109/227, label+users rows; run green twice"
        status: pass
    human_judgment: false
  - id: D2
    description: "committed interface + gitignored local-file contract (D-01): authdomain.example is the shape proof; /authdomain.local never lands in git"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "selftest local-file leg (LOCALFILE-PATH-OK: filled example drives -o d2 -f, custom label wins, DEMO absent) + git check-ignore authdomain.local (GITIGNORE-OK) + zero repo-tree writes (porcelain digest compare)"
        status: pass
    human_judgment: false
  - id: D3
    description: "verify-mode teeth: doctored nvram, doctored keys, doctored manifest row, and truncated keys each refuse with nonzero exit and their own marker"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "selftest: NEG-CONTROL-REFUSED-nvram/-keys/-manifest/-truncated each with the refusal line quoted (plan Task-2 verify command: ALL-NEG-CONTROLS-FIRED)"
        status: pass
    human_judgment: false
  - id: D4
    description: "donor-compiled crypto + the A1 probe verdict recorded: passtokey/nvcsum/des set/sha1 are Harvey's own files compiled, never reimplemented"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "selftest builds the generator from authdom-donors/*.c with the host cc; the /tmp cross-validation probe reproduced plan9port's linked output byte-identically (key=ed1d buf0=36); generate-time decrypt round-trip re-proves derivation every run"
        status: pass
    human_judgment: false
  - id: D5
    description: "-i info mode prints authid, authdom, label, user count (plus the decrypted names and the pairing line)"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "selftest info-mode asserts: authid: bootes / authdom: DEMO-harvey / label: DEMO / users: 3 / names: glenda bootes norbert / keys: decrypt-ok under nvram machkey"
        status: pass
    human_judgment: false
  - id: D6
    description: "local-file path: the example parses as-is, custom label/authdom land in nvram+manifest, distinct passwords work, missing -f refuses loud (MISSING-F-REFUSED)"
    requirement: AUTH-01
    verification:
      - kind: unit
        ref: "selftest LOCALFILE-PATH-OK + MISSING-F-REFUSED legs; unmodified-example parse proven (exdir generate+verify green during Task 1 acceptance)"
        status: pass
    human_judgment: false

duration: 53min
completed: 2026-11-14
status: complete
---

# Phase 18 Plan 03: Bake-time p9sk1 Domain Generator Summary

**Host-C p9sk1 domain generator compiling Harvey's own donor crypto (passtokey, nvcsum, the libsec DES-CBC set with the lacy tail rule, sha1 — byte-verbatim under authdom-donors/) into the hash-paired nvram/keys/ndb-auth/manifest unit, with a teeth-proven verify mode, an ephemeral loud-DEMO path, a committed interface + gitignored local-file contract, and a self-test proving it all from a bare shell.**

## Performance
- **Duration:** 53min - **Started:** 2026-11-14T01:52Z - **Completed:** 2026-11-14T02:45Z - **Tasks:** 3/3 - **Files modified:** 21 (17 new under authdom-donors/, 3 new top-level, .gitignore)

## Accomplishments
- **The pairing unit exists as a one-run product**: `authdomain-gen -o <dir>` emits the 109-byte Nvrsafe nvram (all five nvcsum-seed-9 sums valid; machkey = passtokey(machine password) and simultaneously the /adm/keys master key — the getauthkey.c:14 identity), keys in keyfs writeusers' exact layout (8 random + 73/user, Sok, warnings 0, expire 0 LE, zeroed secret, whole buffer oldCBCencrypt'ed under the nvram machkey with zero ivec and libsec's lacy partial-tail rule), the `/lib/ndb/auth` speaksfor rows with the label/authdom header comment, and the sha1 pairing manifest (`label`/`users` rows + one hash row per artifact).
- **Wave-0 A1 probe verdict (recorded per Task 1's done criteria)**: plan9port `9c -c` + `9l -L$PLAN9/lib ... -lauthsrv -lsec` linking **SUCCEEDED** inside the nix devshell and ran (A1-PROBE-OK, key=ed1d buf0=36) — so plan9port's prebuilt libsec/libauthsrv are available. The **vendored venue was chosen anyway**: the must-have pins the algorithm bytes to `sys/src/libauthsrv/passtokey.c, nvcsum.c, libsec des/desCBC set` — THIS tree's files — and the committed self-test must run under the bare host shell (the plan's own verify commands are `sh build/x86_64/authdomain-selftest.sh`, no nix bootstrap). Cross-validation closed the loop: the vendored Harvey donors compiled with host cc reproduce plan9port's linked-lib output byte-for-byte on a fixed vector, and every generate run re-proves the derivation + CBC pairing via a donor decrypt round-trip before writing a byte.
- **Verify mode has proven teeth**: all four doctored-input classes refuse with nonzero exits and per-class markers, the refusal naming the artifact and both hashes; beyond the plan's letter, generate itself refuses if the round-trip decrypt or any re-derived user key mismatches.
- **The DEMO path is loud and throwaway by construction**: label DEMO, authdom DEMO-harvey (token asserted in the 109 bytes' authdom span and the manifest), users glenda/bootes/norbert sharing one 12-char host-random password; all passwords from /dev/urandom, failing loud if unreadable.
- **The local-file contract is real**: the committed example parses as-is (placeholders sized to the fields), a filled copy drives its own label/authdom/users with distinct passwords, the local file WINS over DEMO defaults, and an explicitly requested-but-missing -f refuses loud — silent fallback exists ONLY on the implicit default path.
- **Zero repo-tree writes**: every artifact lands in the caller's staging dir; the self-test asserts the porcelain digest is unchanged across the whole run.

## Task Commits
1. **Task 1: Wave-0 venue probe + generator producing the paired unit on the DEMO path** - `259d4b99f` (feat)
2. **Task 2: Verify-mode teeth + label discipline in every artifact** - `b55734675` (test)
3. **Task 3: Local-file interface path + example-as-interface proof + self-test committed** - `a5fda3085` (test)
**Plan metadata:** this SUMMARY alone (docs(18-03): complete bake-time p9sk1 domain generator plan — the parallel-wave contract; STATE/ROADMAP/REQUIREMENTS are the orchestrator's writes)

## Files Created/Modified
- `build/x86_64/authdomain-gen.c` — the generator: -o/-f/-V/-i modes, DEMO defaults, interface parser, donor-crypto calls, generate-time round-trip teeth, strict manifest verify
- `build/x86_64/authdom-donors/` — 17 files: verbatim donors (passtokey.c, nvcsum.c, crypt.c, des.c, desCBC.c, desmodes.c, sha1.c, sha1block.c, hmac.c, strecpy.c, authsrv.h, libsec.h, os.h) + thin host-cc shims (u.h, libc.h, mp.h, auth.h)
- `build/x86_64/authdomain.example` — the committed interface, fully documented (fields, user lines, DEMO default, gitignore rule)
- `build/x86_64/authdomain-selftest.sh` — the standing CI-able proof; terminal GEN-SELFTEST-OK
- `.gitignore` — `/authdomain.local`

## Decisions Made
- **Vendored-donor venue over the technically-passing 9c link venue** (the A1 verdict above) — the plan's action text sanctions "the host cc with a thin shim — donor bytes either way"; the artifacts list's "only if the A1 probe rejects" conditional is superseded by the must-have's named Harvey donor files + the bare-shell self-test requirement. Recorded here as the probe verdict the plan asked for, with the reasoning.
- **Shim type widths**: ulong/uint/ushort keep host widths (glibc typedef collisions; identical typedefs are legal, differing ones are errors). Proven width-agnostic for the vendored DES/SHA-1 set (u32int-local arithmetic, explicit masks, uvlong bit counts) — the same choice plan9port makes on LP64.
- **-i prints decrypted user names** — the nvram artifact carries the machkey, so info proves the pairing identity live; the plan asked only for authid/authdom/label/count.
- **authkey field: zeros + valid sum** (no consumer this era — documented at the emit site); **authid fixed at bootes** across both paths (the interface deliberately has no authid key).
- **Manifest shape frozen**: `label <t>` / `users <n>` / exactly three `<sha1>  <name>` rows — strict-order parse, duplicate/unknown/missing/trailing rows all refuse; the users row feeds the keys-length check so doctoring it alone also refuses.

## Deviations from Plan

**1. [Discretion exercised — artifacts-list conditional superseded] authdom-donors/ created although the A1 probe did NOT reject plan9port linking**
- **Found during:** Task 1 (the Wave-0 probe itself)
- **Issue:** the artifacts list marks `build/x86_64/authdom-donors/` "only if the A1 probe rejects plan9port linking"; the probe succeeded. However the must-have pins the compiled algorithm files to Harvey's in-tree donor sources, and venue A would compile plan9port-2025-11-10's descendant bytes (a moving target across nixpkgs bumps) while breaking the bare-shell self-test the plan's own verify commands assume.
- **Fix:** vendored venue chosen — explicitly sanctioned by the task's action text ("9c or the host cc with a thin shim — donor bytes either way"); probe verdict + fixed-vector cross-validation (byte-identical outputs) recorded per the plan's own instruction.
- **Files modified:** none beyond the plan's named set (authdom-donors/ is the plan's own conditional artifact)
- **Verification:** vendored probe reproduced `key=ed1d buf0=36` (the 9c-linked result) on the fixed vector
- **Commit:** 259d4b99f

**2. [Rule 2 - missing critical functionality] generate-time decrypt round-trip self-check**
- **Found during:** Task 1 design (the "opaque p9sk1 failure weeks later" fear the plan's prohibition names)
- **Issue:** the plan proves donor correctness only through the committed self-test; a future host-cc/shim drift could pass compile and still emit wrong bytes silently.
- **Fix:** generate decrypts a copy of the keys buffer with donor oldCBCdecrypt and re-derives every user key with donor passtokey before writing anything — mismatch aborts the run loudly.
- **Files modified:** build/x86_64/authdomain-gen.c
- **Verification:** green on every generate (all self-test legs); the mechanism is donor code on both sides
- **Commit:** 259d4b99f

**Total deviations:** 2 (1 discretion-documented, 1 Rule-2 auto-hardening). **Impact:** none on scope or downstream contracts; both strengthen the must-have's own invariants.

## Issues Encountered
None — build and all gates green on first committed state (the development-time shim fixes — glibc typedef collisions, implicit-decl of donor `encrypt`, `%ux` vs `%x` verb divergence, ARGBEGIN multi-arg USED under the ANSI cpp — were resolved inside Task 1 before its commit; the fixed-vector cross-check caught the format-verb gap that would otherwise have silently mis-printed key material diagnostics).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Sibling plan 18-01 (r9 entropy/devcap substrate) ran in parallel in `../r9`; this plan never touched it. Ready for 18-04 — wave 3 consumes this plan together with 18-02.
- **18-04's bake arm consumes, verbatim:** `authdomain-gen -o <staging-dir>` (DEMO default on absent local file), `-V` verify semantics, artifact names `nvram`/`keys`/`ndb-auth`/`manifest` (staged as /adm/nvram, /adm/keys, /lib/ndb/auth, /adm/domain.manifest), and the pinned DEMO defaults (label DEMO, authdom DEMO-harvey, glenda/bootes/norbert, one shared demo password). Build wiring note: the generator compiles with plain `cc` over `build/x86_64/authdom-donors/*.c` + `-I build/x86_64/authdom-donors` (the committed self-test's build line is the reference invocation); the mkimage app env carries plan9port but not a C compiler — 18-04 either adds a cc runtimeInput or reuses this exact build shape wherever a compiler exists.
- **Never asserted by any later gate:** cross-bake byte identity of the auth image or its key artifacts (D-01's accepted consequence — the pairing manifest is an internal-consistency proof only).
- A2 (the literal speaksfor row format) rides to 18-05's negative control as planned.

Coverage contract note: every deliverable above carries automated verification with pass statuses from the committed self-test; no human-judgment deliverables.
