---
phase: 19-cpu-service-drawterm-rio-desktop
plan: 04
subsystem: kernel
tags: [devssl, ssl, rc4, rc4_256, sha1, pushssl, encryption, kernel-device, dec64, r9, x86_64]

# Dependency graph
requires:
  - phase: 19-cpu-service-drawterm-rio-desktop/19-03
    provides: the exec/syscall kernel tree (f691914) this plan builds on, and the #!/WSTAT/FWSTAT/REMOVE surface the cpu listener path consumes
  - phase: 18-in-guest-p9sk1-auth-domain/18-02
    provides: the devcap small-device skeleton (the port precedent devssl replicates) and the hmacsha1 SHA1 block devssl reuses — not duplicates
provides:
  - port/src/rc4.rs — libsec rc4.c's key schedule + stream op donor-verbatim (the rc4_256 key class), the SSL-format max/maxpad framing arithmetic as free functions (devssl.c:1189-1195), and dec64 (libc u64.c's codec with the load-bearing skip-not-stop '=' quirk)
  - port/src/devssl.rs — the `#D` device pushssl hard-codes: the donor QID layout (QID(c,y)=(c<<5)|y, Maxdstate=512), clone/ctl/data/secretin/secretout/encalgs/hashalgs, the ctl verb machine with fd-before-alg enforcement and the donor's exact error strings, dec64 secret decode at the donor buffer bounds, the SSL-format record framing (|0x80 2-byte header counting digest+payload, sha1(secret‖message‖msgid) digest prepended INSIDE the encrypted body), per-conv state locking per the dstate discipline, conv exhaustion refusing loudly (Enodev)
  - the fd seam — devssl::set_fd_resolve_hook(proc::current_fd_resolve) (the devsrv set_resolve_hook idiom; the "fd N" verb's buftochan incref shape) + the namec '#D' registry arm + walk/open/stat/close dispatch + the x86_64 pread/pwrite arms: #D walkable in the guest namespace
  - the Pitfall-11 pin set: the exact mksecret-shaped inputs (sha1("0123456789abcdef") -> FE5567E8D76955085218 / 2CDF69D74BB16DFF8E29) through rc4_256+sha1 in BOTH secret orderings over a REAL pipe pair, wire-format inspection (header form + len counting digest+payload + ciphertext-not-plaintext), the corruption leg (one flipped bit -> "bad digest"), and the unswapped negative (client order meets client order -> the digest fails — the swap is load-bearing)
affects: [19-05 (the pin advance over these kernel bytes — f691914..07889e2), 19-07 (the seventh contract's cpu session rides #D), 19-08 (the live encrypted -G gate — DESK-04's composed half)]

# Actuals (#2632) — pairs with the plan's estimate (58000 tokens, low confidence).
# chars/4 over the realized r9 diff (73,239 chars across 8 files, 5 commits).
actuals:
  tokens: 18310
  tasks: 3
  commits: 5   # MEASURED: git rev-list --count f691914..07889e2 (r9 only; no harvey code — all plan code lands in r9 per the cross-repo discipline)

# Tech tracking
tech-stack:
  added: []   # zero external packages — in-tree donors only (T-19-SC honored)
  patterns:
    - "the fd-resolver hook seam: a device whose ctl plane names an fd (donor buftochan) resolves it through an arch-registered fn(u64)->Option<Chan> — the devsrv set_resolve_hook idiom, with a cfg(test) fd-table twin so host pins wire REAL pipe ends fd-to-fd"
    - "donor-error-literal inventory before choosing the error transport: devcap needed CapErr because its donor error embeds user bytes; devssl's donor errors are ALL literals, so the plain &'static str Result carries them verbatim — the transport decision is per-device, mechanical"
    - "wire-inspection loopback: the pin drains the framed record off the real pipe, asserts the SSL-format header bytes, and reinjects (modified for the corruption leg) — the framing is pinned against the WIRE, not against the implementation's own output"

key-files:
  created:
    - ../r9/port/src/rc4.rs
    - ../r9/port/src/devssl.rs
    - ../r9/port/src/devssl_tests.rs
  modified:
    - ../r9/port/src/lib.rs
    - ../r9/port/src/dat.rs
    - ../r9/port/src/namec.rs
    - ../r9/x86_64/src/syscall.rs
    - ../r9/x86_64/src/main.rs

key-decisions:
  - "#D registers UNCONDITIONALLY in the namec registry (no dev_ssl cfg key, NO boot profile joins — the plan's own instruction, recorded here): the donor carries devssl in every standard k10 CONF, the module is self-contained (rc4_256/sha1 live in port), and the evidence split is the registry-level pin (Task 3) + 19-08's live encrypted -G gate — a cfg key with no declaring profile would leave the device unreachable and untestable"
  - "The secrets are dec64 of mksecret-shaped 20-char uppercase-hex ASCII -> 15-byte rc4 keys (a hex decode's 10 bytes would silently break both arms — the must-have's exact words, pinned by python-derived byte-exact vectors); the plan's '20/13-byte splits' pinned as the codec's tail arithmetic: 20 chars (multiple of 4) -> 15 bytes, the 18-char boundary -> 13 bytes (2-char tail), 19 -> 14 (3-char tail), plus the donor's skip-not-stop '=' quirk (TQ==TQ== -> 4D04D0, standard base64 would give MM)"
  - "The donor's per-conv in.q/in.ctlq/out.q fold to ONE lock per conv (Arc<Lock<Dstate>> under the table lock): a tightening, never a loosening — the donor's ctl path takes both axes anyway; blocking wire I/O holds only the conv lock so a peer conv on the same connection never waits on the table"
  - "The io dispatch covers DEV_PIPE (the pin substrate) + DEV_IP (the live TCP venue, with the syscall layer's drain-debt discipline) and refuses loudly otherwise — nobody ssl-wraps another device this era (donor accepts any chan; the scoping is recorded in the module doc)"
  - "No CapErr-style formatted-error transport: every donor devssl error is a literal (must set fd before algorithm / bad algorithm / algorithm but no secret / arg too long / bad digest / bad pad in ssl message / cannot ssl encrypt devssl files / the error.h classes) — carried verbatim as &'static str"
  - "SHA1 reused, not duplicated: the donor's streaming digestb/checkdigestb (secret, then message, then msgid) composes exactly as hmacsha1::sha1(secret || message || mid_be) — one SHA1 block remains in the tree (grep-proven)"
  - "encalgs/hashalgs serve the SCOPED tables (\"rc4_256\" / \"sha1\") — the fuller donor tables are out of measure, recorded in rc4.rs's module doc with the era-shape crypto acceptance (T-19-08: rc4 known-weak, PROTOCOL-blocked from stronger per DESK-04's requirement text)"

patterns-established:
  - "Registry-level name-resolution pin: namec(b\"#D/ssl/clone\") through the '#'-arm + walk funnel + open_chan — the substrate evidence for a device with no boot-profile consumer yet (14-D-06's split, the plan's acceptance criterion wording)"

requirements-completed: []   # DESK-04 co-declared by 19-08 (the -G gate IS the live encrypted proof) — the shared-ID gate blocks it until 19-08's SUMMARY exists; this plan delivered the kernel half

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The rc4_256 key class + framing math + dec64 codec (Task 1): donor-verbatim KSA/stream (the classic RC4 vector BBF316E8D940AF0AD3 + 32-byte key round-trip), state independence/position sensitivity, the max/maxpad donor literals at stream and block-8 edges (32767/32747; 32760/16376; 32744/16360), dec64 of the mksecret shapes byte-exact with the 13/14-byte tail splits and the skip-not-stop quirk"
    verification:
      - kind: unit
        ref: "r9 port lib (cargo test -p port --lib -- rc4): 4 passed — rc4_known_vector_and_32byte_key_round_trip, rc4_states_are_independent_position_sensitive_copies, framing_max_maxpad_match_the_donor_formulas, dec64_decodes_mksecret_shapes_with_the_donor_tail_splits"
        status: pass
      - kind: integration
        ref: "RED evidence evidence/19-04-t1-red-evidence.json classified RED_EVIDENCE_OK (4 discovered / 0 passed / 4 planned failures) then GREEN — the test->feat commit pair 3737fd1/b1e7e3e"
        status: pass
    human_judgment: false
  - id: D2
    description: "The #D device (Task 2): the pushssl protocol walk (clone open rewrites the qid to the conv ctl, ctl reads the decimal id, the data path resolves by name), the verb ladder fd->secretin->secretout->alg, alg clear (the -e fallback) round-tripping framed-but-clear, the donor verb-error strings verbatim, the rc4_256+sha1 loopback with wire inspection + the loud corruption leg, and the swapped server-arm ordering round-tripping both directions with the unswapped negative failing the digest"
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- devssl: 4 passed — pushssl_protocol_walk_serves_the_verb_order_and_alg_clear, ctl_verb_errors_carry_the_donor_strings, loopback_round_trips_rc4_256_sha1_and_detects_corruption, swapped_server_arm_secrets_round_trip_both_directions"
        status: pass
      - kind: integration
        ref: "RED evidence evidence/19-04-t2-red-evidence.json classified RED_EVIDENCE_OK (4 / 0 / 4 planned failures at the clone-open gate) then GREEN — the test->feat commit pair e899ca7/270fdbf"
        status: pass
    human_judgment: false
  - id: D3
    description: "The registry wiring + battery + push (Task 3): namec resolves #D/ssl/clone through the '#'-arm + walk funnel + open_chan (the registry-level pin), the x86_64 pread/pwrite arms carry the device, main.rs wires the fd-resolver hook, battery green twice (check+clippy+port 160 + x86_64 103), four boot legs green twice (q35cpu with cpurc: RAN / q35auth / q35shell / q35net), r9 pushed f691914..07889e2 (LEGS-GREEN-PUSHED), zero aarch64 files"
    verification:
      - kind: unit
        ref: "cargo test -p port --lib -- devssl rc4: 9 passed (incl. registry_level_name_resolution_of_the_d_walk)"
        status: pass
      - kind: e2e
        ref: "the plan's verify commands verbatim: BATTERY-OK (twice: CHECK-OK + CLIPPY-OK + port 160 + x86_64 103, re-run at close-out) and LEGS-GREEN-PUSHED (q35cpu PASS x2 + q35auth PASS x2 + the q35shell/q35net controls PASS x2 each + git log origin/main..HEAD empty after push)"
        status: pass
    human_judgment: false

# Metrics
duration: 37 min
completed: 2026-11-15
status: complete
commits_note: "r9 plan_head_before=f691914 plan_head_after=07889e2 (both == origin/main, LEGS-GREEN-PUSHED); no harvey code — the flake.lock advance is 19-05's, never here"
---

# Phase 19 Plan 04: devssl `#D` Summary

**r9 gains the `#D` ssl device pushssl hard-codes — rc4_256/sha1 donor-verbatim from libsec's rc4.c + 9k's devssl.c (dec64 of mksecret-shaped secrets, SSL-format framing, the fd-before-alg verb machine with the donor's exact error strings) — pinned by a real-pipe loopback that inspects the wire, flips a bit (bad digest), and proves the srvp9auth secret swap load-bearing; #D walkable via namec, battery + four boot legs green twice, pushed**

## Performance

- **Duration:** 37 min
- **Started:** 2026-11-15T00:31:35Z
- **Completed:** 2026-11-15T01:11:09Z
- **Tasks:** 3
- **Files modified:** 8 (all in r9) + 4 evidence artifacts beside this SUMMARY

## Accomplishments

- **Task 1 (TDD, RED->GREEN):** `rc4.rs` — libsec rc4.c's key schedule and stream op transcribed donor-verbatim (the classic known vector + the 32-byte rc4_256 key-class round-trip + state independence/position sensitivity), the max/maxpad framing arithmetic as free functions (the donor formulas pinned at the stream and block-8 edges with hand-evaluated literals), and dec64 (u64.c's codec — skip-not-stop INVAL semantics, the 15/13/14-byte tail splits, byte-exact against python-derived vectors of the real mksecret composition). SHA1 reused from hmacsha1.rs (grep-proven single block). Scope recorded in the module doc: rc4_256+sha1 only, era-shape accepted (T-19-08).
- **Task 2 (TDD, RED->GREEN):** `devssl.rs` — the #D device on the devcap skeleton: the donor qid layout/enum, clone allocation (first-empty-slot, Enodev-class loud refuse at Maxdstate=512) with the qid-rewrite to the conv ctl, the ctl verb machine (fd-before-alg with "must set fd before algorithm" verbatim, Ebadarg-class unknown verbs, "bad algorithm", "algorithm but no secret", the single-secret auto-copy, dec64 secrets at the donor buffer bounds, alg clear as the -e fallback), sslput/sslbread over Vec buffers (the 2-byte |0x80 header counting digest+payload, the digest inside the encrypted body, the 3-byte padded form still parsed), per-conv Arc lock discipline, conv-free releasing the fd chan through close_chan (the donor's cclose).
- **Task 3:** the wiring — namec's '#D' registry arm (unconditional: no boot profile joins, recorded per the plan), walk/open/stat/close dispatch arms, the x86_64 pread/pwrite arms, and main.rs's `set_fd_resolve_hook(proc::current_fd_resolve)` beside the devsrv/devdup hooks; the registry-level pin (namec resolves #D/ssl/clone end-to-end through the '#'-arm + funnels); battery twice + four boot legs twice + push (LEGS-GREEN-PUSHED).

## Task Commits

Each task was committed atomically (cross-repo: all code lands in r9 `main`, push-before-pin; the harvey flake.lock advance is 19-05's, never here):

1. **Task 1 RED: the rc4/framing/dec64 pins + stubs** - r9 `3737fd1` (test)
2. **Task 1 GREEN: the donor-verbatim implementations** - r9 `b1e7e3e` (feat)
3. **Task 2 RED: the #D device pins + skeleton stubs** - r9 `e899ca7` (test)
4. **Task 2 GREEN: the device implementation** - r9 `270fdbf` (feat)
5. **Task 3: the registry wiring + fd hook + registry pin** - r9 `07889e2` (feat)

**Plan metadata:** (this SUMMARY + STATE/ROADMAP commit, below)

## Files Created/Modified

- `../r9/port/src/rc4.rs` - setup_rc4_state/Rc4state::rc4 (libsec rc4.c), ssl_max/ssl_maxpad (the framing arithmetic), dec64 (u64.c) + the 4 known-answer pins
- `../r9/port/src/devssl.rs` - the #D device: attach/walk/open/read/write/stat/close, the ctl verb machine, sslput/sslbread, ensure/check_digest, the fd-resolver hook + test seams
- `../r9/port/src/devssl_tests.rs` - the four pin classes + the registry-level pin (devcons user-seam + pipe-pair loopback discipline)
- `../r9/port/src/lib.rs` - pub mod rc4 + devssl + devssl_tests registration
- `../r9/port/src/dat.rs` - DEV_SSL = 11
- `../r9/port/src/namec.rs` - the '#D' registry arm + walk/open/stat/close dispatch arms
- `../r9/x86_64/src/syscall.rs` - the pread/pwrite DEV_SSL arms
- `../r9/x86_64/src/main.rs` - set_fd_resolve_hook(proc::current_fd_resolve)

## Decisions Made

See key-decisions. Additionally: the walkable conv set is bounded by the donor's dshiwat semantics (decimal names string-matched below the ever-growing hiwat mark — "007" never matches "7"); stat serves glenda uid (stat content is not on the pushssl path, which never stats — the devcap shape); writing encalgs/hashalgs refuses loudly where the donor PANICS (sslwrite's default arm — the port's loud-refusal discipline for the donor's own bug class, documented in place).

## TDD Gate Compliance

Tasks 1 and 2 are `tdd="true"` with the RED->GREEN cycle and the commit-scope contract honored (test(...) precedes feat(...), both in r9): Task 1 `3737fd1`/`b1e7e3e`, Task 2 `e899ca7`/`270fdbf`. Both RED captures are libtest-JSON runs mechanically converted to TAP (19-02/19-03's transform, raw streams preserved) and classified `RED_EVIDENCE_OK` — Task 1: 4 discovered / 0 passed / exactly the 4 planned assertion failures against the honest stubs (`evidence/19-04-t1-red-evidence.json`); Task 2: 4 / 0 / 4 at the clone-open gate (`evidence/19-04-t2-red-evidence.json`) — a wholly-new device has no earlier divergence point than its first op, and the semantic assessment records that every failure is an assertion failure, never a compile/load/fixture fault (the pre-evidence compile faults — a String/&String vector-table type, the const-block array repeat, the heterogeneous tuple arrays, devpipe's 3-arg calls — were fixed BEFORE the evidence runs). Task 1's GREEN phase caught and fixed one real bug pre-commit: `ORDWR` was missing from the import list, making that match arm a BINDING pattern (clippy's unreachable-pattern warning exposed it — the donor-default perm arm would have been shadowed). Plan-level `workflow.tdd_mode` is false (config); the task-level cycles are complete. No REFACTOR phase needed for either task (direct donor transcriptions; the clippy-driven collapses landed inside GREEN before its commit).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Module registration beyond the named files (Tasks 1+2)**
- **Found during:** Task 1 (the first test run) and Task 2 (the device chans)
- **Issue:** The plan's files lists named rc4.rs/devssl.rs (+ lib.rs for Task 2) only, but a new port module must be declared in lib.rs, the device chans need a DEV_SSL registry index in dat.rs, and Task 1's Test-4 pin (dec64) needs its home — rc4.rs carries it as the framing-math half per the task's action text
- **Fix:** lib.rs gains pub mod rc4 + pub mod devssl + mod devssl_tests; dat.rs gains DEV_SSL = 11; dec64 lives in rc4.rs with the donor cite
- **Files modified:** ../r9/port/src/lib.rs, ../r9/port/src/dat.rs
- **Verification:** the port suite green (160); clippy clean on the new files
- **Committed in:** r9 3737fd1 (Task 1 RED) and e899ca7 (Task 2 RED)

**2. [Rule 3 - Blocking] The registry wiring spans namec.rs + syscall.rs, not just lib.rs/main.rs (Task 3)**
- **Found during:** Task 3 (the wiring)
- **Issue:** The plan's files named lib.rs (module registration) + x86_64 main.rs (registry hook) — but the rune dispatch and the walk/open/stat/close/read/write dispatch live in port/src/namec.rs and x86_64/src/syscall.rs (the codebase's actual registry sites, established by every prior device); wiring only lib.rs/main.rs would leave #D unreachable
- **Fix:** namec.rs gains the '#D' arm + the four dispatch arms; syscall.rs gains the pread/pwrite arms; main.rs wires the fd-resolver hook
- **Files modified:** ../r9/port/src/namec.rs, ../r9/x86_64/src/syscall.rs, ../r9/x86_64/src/main.rs
- **Verification:** the registry-level pin green (namec resolves #D/ssl/clone); the full battery + four boot legs green twice
- **Committed in:** r9 07889e2

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both were registration/wiring prerequisites the plan's file lists under-specified; the ported semantics themselves needed no scope change. The unconditional-'D' choice (no cfg key) follows the plan's own "no separate boot profile joins" instruction and is recorded in key-decisions.

## Issues Encountered

- The first Task-1 RED capture did not compile (a String/&String comparison type in the dec64 pin's vector table) and the first Task-2 RED capture exposed three more test-local compile faults — all fixed BEFORE the evidence runs so every recorded failure is an assertion failure (the INVALID_RED rule).
- clippy's unreachable-pattern warning on `open_perm` exposed a genuine latent bug: the un-imported `ORDRL` match arm had become a binding pattern (matching every value) — fixed by importing the constant; the OEXEC default arm behaves donor-exactly now.

## Known Stubs

None - no stubs, placeholders, or unwired data paths were introduced. (The io dispatch covering DEV_PIPE+DEV_IP only is a recorded scoping with a loud refusal naming the gap, not a stub; DES-class algorithms are out of measure by the plan's own NOSPOOKS-subset instruction, documented in rc4.rs's module doc.)

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- DESK-04's kernel half is landed, pinned, live-walkable, and pushed (r9 at `07889e2` == origin/main); 19-07 (the seventh contract) and 19-08 (the live encrypted -G gate — DESK-04's composed half and the shared-ID unblock) build on this tree
- Kernel bytes changed (8 files in r9): the boot-iso FOD hash re-stales structurally (the standing 13-06/14-07 discipline) — 19-05's re-wire-then-pin pass covers it; the harvey flake.lock r9 pin is NOT advanced here (the cross-repo discipline)
- DESK-04 stays open at the requirements level: 19-08 co-declares it (the gate IS the interop probe, riding encrypted) — the shared-ID gate blocks until 19-08's SUMMARY exists

## Self-Check: PASSED

- Created files exist on disk: ../r9/port/src/rc4.rs, devssl.rs, devssl_tests.rs FOUND
- Task commits exist and are pushed: r9 3737fd1, b1e7e3e, e899ca7, 270fdbf, 07889e2 (== origin/main, LEGS-GREEN-PUSHED, origin/main..HEAD empty) FOUND
- RED evidence exists and classified: evidence/19-04-t1-red-evidence.json + 19-04-t2-red-evidence.json (both RED_EVIDENCE_OK) FOUND
- Acceptance criteria re-verified at close-out: rc4.rs carries the donor-citing module doc; one SHA1 block remains (hmacsha1.rs); the devssl pin set green (protocol walk, donor strings, loopback + corruption, swapped arms); namec resolves #D/ssl/clone at the registry level; battery twice (BATTERY-OK + BATTERY-OK-2 at check/clippy/port-160/x86_64-103); q35cpu/q35auth/q35shell/q35net PASS x2 each with cpurc: RAN in the q35cpu log; zero aarch64 files in the diff

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-15*
