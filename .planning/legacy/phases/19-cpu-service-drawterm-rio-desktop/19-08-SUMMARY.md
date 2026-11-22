---
phase: 19-cpu-service-drawterm-rio-desktop
plan: "08"
subsystem: infra (the -G interop gate)
tags: [drawterm, -G, pty, expect, hostfwd, p9any, p9sk1, devssl, ipconfig, ether, flake-app, devmnt, tag-mux, srv-sessions, blocking-pipe-write, note-interrupt, fod, pin-advance]

# Dependency graph
requires:
  - phase: 19-cpu-service-drawterm-rio-desktop/19-06
    provides: the drawterm -O/-G surface + the nix package (DRAWTERM_BIN) the gate spawns
  - phase: 19-cpu-service-drawterm-rio-desktop/19-07
    provides: the §15 cpu image (bootfs-cpu.paq) + out/auth-creds.txt (the D-12 parse contract) + the cpu-bringup stage vocabulary
  - phase: 16-host-net-modes-networking-exit-gate
    provides: the committed D-04 hostfwd pair (127.0.0.1:17010/5670) the session dials
provides:
  - build/x86_64/drawterm-gate.expect — the TWO-subtree -G session chassis, validated LIVE end-to-end (green twice on KVM + a TCG leg green)
  - nix/drawterm-gate-x64.sh + the flake app .#drawterm-gate-x64 — the full pipeline (tier build -> bake -> loud creds read -> ISO-with-append -> gate -> sweeps), green twice AT THE ADVANCED PIN
  - the r9 concurrency chain this pass landed (all PUSHED and PINNED at df4ad78): the devssl direction split (the donor's in.q/out.q), the devmnt tag-mux (the donor's mountio/mountmux — a transferable single-wire-reader BusyCell token spanning whole-message framing, the outstanding-tag registry, owner-side reply validation), the session-teardown Drop (the donor mntdec's cclose), the note-interrupt fix (postnote's ready(p) + the notepending probe), devpipe's BLOCKING write on full queues (the donor's qwrite)
  - the -G session's complete serial evidence (CPU-04): password prompt -> tr-ok -> the encrypted rc4_256 sha1 session -> the remote rc -li prompt on the drawterm pty -> the fed command's round-trip -> the full teardown chain (noteproc killed by note -> session Drop -> TCP hangup -> the client exportfs death -> pty EOF), zero kernel panic, the capture redacted
  - the pin choreography closed: flake.lock 0a4b853 -> aa55a27 -> df4ad78 (two lock-only commits), the boot-iso FOD re-wired TWICE (two-derivation proofs, three-way NAR agreement each), hosted CI observed SUCCESS at both pushed HEADs before each advance (runs 43711744441, 43717790664)
affects: [19-09 (boots the same §15 image at the df4ad78 pin — the mux/blocking-write kernel bytes are its substrate), 19-10 (DESK-07's evidence rows — the gate is green), the next r9-touching pass (the FOD re-stales on any kernel-byte commit, the standing drift class)]

actuals:
  tokens: 27300    # chars/4 over the realized production diff (r9 917685c..df4ad78 ~101.4kc + harvey 18172e509..HEAD code ~8.0kc; the estimate was 52000 across ALL passes — the three passes together are within it)
  tasks: 3         # all three tasks complete this pass (Tasks 1-2 prior passes; Task 3 closed here)
  commits: 9       # MEASURED: git rev-list --count 4f5a55a..HEAD (the plan-ledger base; spans all three passes — 4 production + 2 pin-advance lock-only + 3 prior-pass commits). r9: 7 commits in its own repo (22d40fb..df4ad78), pushed == origin/main
plan_head_before: 4f5a55a6030957de1755fee4cd0f857fafa526ea
plan_head_after: 4ceb275de   # the second lock-only pin advance; this SUMMARY's metadata commit sits directly atop

tech-stack:
  added: []          # zero packages (T-19-SC honored)
  patterns:
    - "The r9 BusyCell reader-token: an ownership that spans a BLOCKING read must be a try-enter cell with park-retry contenders, NEVER an MCS lock (an MCS guard across a park spin-wedges an IRQ-masked contender on the single-CPU era) — and a token that spans whole-message framing must have NO release point between a message's header and its body (the mid-frame window is the boot race the first rework attempt died of)"
    - "Note-interruptible blocking IO needs BOTH halves or neither works: post_note must READY a Blocked target (the donor's ready(p)) AND the parking loop must surface the pending note at its re-block arm ('interrupted') — a wake without the probe re-parks forever; a probe without the wake never runs"
    - "Whole-message transports over bounded queues: the moment a protocol multiplexes concurrent rpcs over ONE pipe, server replies accumulate while owners park between polls — an error-on-full write kills ramfs-class servers fatally ('paqfs: mount write'); the donor's BLOCKING qwrite is load-bearing, not stylistic"
    - "Never-exercised assertion code is unverified code: the gate's drain loop, its 'goodbye' teardown marker, and its whole-file panic scan all carried pass-1 design assumptions falsified the moment the teardown first went green ([\s\S] is a PCRE-ism POSIX regcomp rejects; the vendored drawterm's sysfatal routes through kern/panic's Unix exit, never cpu.c's exits() override)"

key-files:
  created:
    - build/x86_64/drawterm-gate.expect          # (prior pass) the two-subtree -G session gate — this pass re-keyed live (session-exit, drain, panic-scan)
    - nix/drawterm-gate-x64.sh                    # (prior pass) the pipeline driver
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-08-gate-pin-a.log   # THIS pass: green run A at the pin (redacted)
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-08-gate-pin-b.log   # THIS pass: green run B (green-twice; redacted)
    - .planning/phases/19-cpu-service-drawterm-rio-desktop/evidence/19-08-gate-tcg.log     # THIS pass: the TCG leg, green (redacted)
  modified:
    - flake.lock                                   # r9 0a4b853 -> aa55a27 (dfb6e5ab1) -> df4ad78 (4ceb275de), lock-only
    - ../r9/port/src/devssl.rs                     # the direction split (22d40fb)
    - ../r9/port/src/devmnt.rs                     # the tag-mux + the session Drop (091614a, 3c67e2a)
    - ../r9/port/src/mcslock.rs                    # the MntIoBox Send proof
    - ../r9/port/src/devpipe.rs                    # the blocking write + the wwaiter wake paths (56acaea)
    - ../r9/port/src/devip/{proto,stack}.rs        # the note_pending probe in the data-read block arms
    - ../r9/x86_64/src/proc.rs                     # push_note's ready(p) + note_pending
    - ../r9/x86_64/src/main.rs                     # the hook registrations
    - ../r9/port/tests/devmnt_test.rs              # the split-wire re-key + the two-thread tag-mux pin
    - ../r9/port/tests/devpipe_test.rs             # the blocking-write pin re-key + the scripted-test serialization guard
    - ../r9/flake.nix                              # the TWO FOD re-wires (aa55a27, df4ad78)

key-decisions:
  - "The named suspects were eliminated STRUCTURALLY rather than chased with probes: the reader token (a BusyCell) spans whole-message framing by construction (suspect a — the mid-frame release/re-take window cannot exist), and no MCS lock is ever held across a park (suspect b — the tick-retry paths hold only BusyCells and leaf locks, and BusyCell contenders park). The rebuilt rework booted q35cpu PASS x5 + shell/auth/churn on its first session — the boot race did not recur at those venues; the one residual race it exposed (tier1's paqfs root death) was a DIFFERENT defect (error-on-full pipe writes), root-caused from the capture and fixed donor-faithfully"
  - "The session teardown needed THREE more layers past the deadlock, each root-caused from live captures before fixing (D-11's measure-first discipline): (1) the Mnt's io chans dropped without close_chan — the ssl conv never hung its TCP wire up (the donor mntdec's cclose shape, landed as Mnt::drop); (2) push_note only queued — an indefinitely-parked reader was never readied, so the kill note never landed (the donor postnote ready(p) + the notepending probe at the block arms); (3) devpipe's write ERRORED on full — under the mux, concurrent rpcs let server replies pile up while owners park between polls and the root paqfs died at its 'mount write' fatal (the donor's BLOCKING qwrite)"
  - "The gate's session-exit stage was re-keyed to the LIVE-OBSERVED teardown: the pass-1 'goodbye' marker is unreachable in the -G session end against ANY server (the vendored drawterm's exportfs fatal routes through kern/panic's Unix exit(1), never cpu.c's exits() override); the deterministic end marker is the exportfs hangup line ('eof: n=-1 ... i/o on hungup channel') followed by the pty EOF. Same for the drain's [\s\S]+ (a PCRE-ism glibc regcomp rejects — the loop had never executed before the teardown first went green) and the panic scan (drawterm's own 'panic: sysfatal' exit pair excluded; any other panic still fails)"
  - "The pin choreography ran TWICE (kernel bytes changed twice): FOD probe -> two-derivation proof (plain + fresh marker-clone kernel builds, cmp-clean ISOs, three-way NAR agreement with the FOD's own got-value) -> re-wire (flake.nix only) -> push -> hosted CI observed SUCCESS at the pushed HEAD -> lock-only advance. CI runs 43711744441 (aa55a27) and 43717790664 (df4ad78), both observed green BEFORE the respective advance commits landed"

patterns-established:
  - "The two-thread host pin for kernel concurrency classes: a condvar-blocking MntIo fake whose stash knob delivers a concurrent rpc's reply BEFORE the parked reader's own — the parked reader (holding the token) must route a foreign reply and each side receives exactly its own; the -G session's exact shape at host determinism (needs the doc-hidden park-hook test seam — the kernel's contention hook has no host analogue)"

requirements-completed: [DESK-06, CPU-04, DESK-04]   # plan frontmatter verbatim; the shared-ID gate verb governs the actual checkbox flips

coverage:
  - id: D1
    description: "The pty-driven session chassis (Task 1): two subtrees, feedstage password discipline, presence asserts, fast-fail, post-passes, last-step redaction shape"
    requirement: DESK-06
    verification:
      - kind: other
        ref: "GATE-FILE-SHAPE-OK (the plan's verify, pass 1) + validated LIVE green twice at the pin this pass (the file's stages all fire in 19-08-gate-pin-{a,b}.log)"
      - kind: other
        ref: "This pass's live re-key verified in the retained captures: the session-exit hangup marker present (2 occurrences — drawterm's prefixed + bare lines), the drain completed, the panic scan passed with drawterm's own exit pair excluded"
        status: pass
    human_judgment: false
  - id: D2
    description: "The driver + flake app (Task 2): the full pipeline with the loud creds read, the append-bearing ISO compose, both survivor sweeps"
    requirement: DESK-06
    verification:
      - kind: other
        ref: "GATE-APP-EVAL-OK + DRIVER-SHAPE-OK (the plan's verifies, pass 2) + re-evaluated at the advanced pin; the app itself drove the green-twice runs (nix run .#drawterm-gate-x64 at df4ad78)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The ether reception seam: the cpu image receives the committed hostfwd pair (cpurc leg + §15 ipconfig staging)"
    requirement: DESK-06
    verification:
      - kind: e2e
        ref: "cpurc ETHER-OK + the accept firing in every capture this pass (pin-a/pin-b/tcg: 'tcp call for tcp17010'); the q35cpu boot profile PASS at the rework trees (6 consecutive boots + the profile set)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Green twice on local KVM (D-14's required venue) + the redacted retained capture + the honest TCG leg — DESK-06's bar, CPU-04's consolidated evidence, DESK-04's live encrypted proof (D-15)"
    requirement: DESK-06
    verification:
      - kind: e2e
        ref: "GATE-A-GREEN: nix run .#drawterm-gate-x64 at the df4ad78 pin exit 0, ALL STAGES PASS (evidence/19-08-gate-pin-a.log)"
        status: pass
      - kind: e2e
        ref: "GATE-GREEN-TWICE: the second consecutive run exit 0, ALL STAGES PASS (evidence/19-08-gate-pin-b.log)"
        status: pass
      - kind: other
        ref: "REDACTION-OK both retained captures (the <demo-pw-redacted> token present, zero real-password occurrences)"
        status: pass
      - kind: e2e
        ref: "The TCG leg (R9_ACCEL=tcg) GREEN end-to-end at the pin — ALL STAGES PASS (evidence/19-08-gate-tcg.log); recorded honestly: green, not a venue limitation"
        status: pass
      - kind: other
        ref: "Hosted CI (best-effort per D-14): runs 43711744441 and 43717790664 both SUCCESS at the pushed HEADs, observed before each pin advance"
        status: pass
    human_judgment: false
  - id: D5
    description: "The q35cpu boot-profile re-key (19-07's carried item) — re-proven at the rework trees"
    requirement: DESK-06
    verification:
      - kind: other
        ref: "boot-test PASS x6 consecutive at the mux tree + PASS at the note-interrupt tree + PASS at the blocking-write tree (the six-rung ladder green, cpurc DONE, zero FAIL) + the profile set (shell/auth/churn) green at the final tree"
        status: pass
    human_judgment: false

duration: 205 min (this pass; 120 + 280 the two prior)
completed: 2026-11-15
status: complete
commits_note: "harvey plan_head_before=4f5a55a (the pass-1 ledger base) plan_head_after=4ceb275de; r9 917685c..df4ad78 (7 commits, pushed == origin/main)"
---

# Phase 19 Plan 08: The -G interop gate Summary

**The -G interop gate is GREEN end-to-end and green-twice at the advanced pin (df4ad78): a scripted drawterm -G session dials through the committed hostfwd pair, authenticates p9any-to-p9sk1, rides the encrypted rc4_256 sha1 session, lands the remote rc -li prompt on the pty, round-trips a fed command, and tears down cleanly (noteproc killed by note -> session Drop -> TCP hangup -> the client exportfs death -> pty EOF) — closed by a five-layer r9 concurrency chain landed this pass (the devssl direction split, the devmnt tag-mux, the session-teardown Drop, the note-interrupt fix, the blocking pipe write), all push-before-pin with two FOD proofs and two hosted-CI observations**

## Performance

- **Duration:** 205 min this pass (root-cause + five kernel fixes + the choreography ×2 + the gate legs); 120 + 280 the prior passes
- **Started:** 2026-11-15T10:21:25Z (this continuation; the prior passes 02:58:44Z / 05:04:08Z)
- **Completed:** 2026-11-15T13:46:28Z
- **Tasks:** 3/3 complete (Tasks 1-2 prior passes; Task 3 closed this pass)
- **Files modified:** r9 11 (7 commits, pushed) + harvey 2 (the gate re-key, flake.lock ×2) + 3 evidence captures

## The concurrency chain that closed the session (Task 3, D-11's umbrella — each layer root-caused from a live capture)

1. **The devssl direction split** (r9 22d40fb): the conv's single Dstate lock was held across the blocking wire read, so a parked ssl read blocked every ssl write on the conv (and an MCS guard across a park spin-wedges an IRQ-masked contender). The conv became brief-op shared state + TWO BusyCell direction cells (the donor's `s->in.q`/`s->out.q`), each carrying its OWN dup of the underlying chan — a parked reader and an active writer hold separate handles. The five devssl host pins passed UNCHANGED (the public API held).
2. **The devmnt tag-mux** (r9 091614a): the busy flag's one-outstanding-rpc limit was the root deadlock. The donor's mountio/mountmux model: every rpc queues a tag slot and sends (io_w cell); exactly ONE wire reader (the io_r BusyCell IS the donor's `m->rip`) frames whole messages and routes queued rpcs' replies into their slots; a reader parked on an indefinite rpc (cpunote, console stdin) keeps serving everyone. The token spans whole-message framing ONLY — the two named boot-race suspects eliminated structurally (no mid-frame release/re-take window exists; no MCS lock is ever held across a park). NEW two-thread host pin: a condvar-blocking fake whose stash knob delivers a concurrent rpc's reply BEFORE the parked reader's own — the -G shape at host determinism.
3. **The session-teardown Drop** (r9 3c67e2a): with (1)+(2) the session reached prompt + fed-echo + rc's NEXT prompt, but the wire never died — the Mnt's io chans dropped without close_chan, the ssl conv's refc never reached zero, and the TCP stayed up forever (the donor mntdec's cclose). `MntIo::close_chan` + `Mnt::drop`: at refcount zero (which proves no rpc in flight — any caller holds the session Arc through its chan) both halves release their kernel reference; the chain fires: pgrp death -> mount entry -> session Arc -> Drop -> ssl conv free -> TCP dups closed -> hangup.
4. **The note-interrupt fix** (r9 1f14a50): the teardown still varied run-to-run — `push_note` only QUEUED notes, and the tick's wake arm readies only deadline sleepers; an indefinitely-parked reader (no deadline) never saw the kill note, so the noteproc never died and (3)'s chain never fired. Two halves of the donor's note-interruptible blocking IO: push_note READIES a Blocked target (postnote's `ready(p)`), and the parking loops (devpipe read + the devip data reads) surface a pending note at their re-block arm as "interrupted".
5. **The blocking pipe write** (r9 56acaea): the tier1 gate then died at the ROOT — `paqfs: mount write` (the root file server's fatal). The mux lets many client rpcs be outstanding, so server replies accumulate in the 4 KiB pipe while their owners park between polls — and devpipe's write ERRORED on full, which ramfs-class servers treat as fatal. The donor's qwrite BLOCKS: the write parks (register under the pipe lock, drop, block) until a draining read makes room; reads wake parked writers; the close paths wake them into their fate. The pipe-full pin re-keyed to the blocking contract.

**The gate then went green end-to-end** — and exposed three never-exercised pass-1 assumptions in its own file (the re-key is its own harvey commit c1d8a09b8): the session-exit "goodbye" marker is unreachable against ANY server (drawterm's exportfs fatal routes through kern/panic's Unix exit, never cpu.c's exits() override) — the deterministic end marker is the exportfs hangup line + pty EOF; the drain's `[\s\S]+` is a PCRE-ism glibc regcomp rejects; the whole-file panic scan excludes drawterm's own "panic: sysfatal" exit pair.

## The pin choreography (ran TWICE — kernel bytes changed twice)

| Step | First pass (aa55a27) | Second pass (df4ad78) |
|---|---|---|
| FOD got-value | sha256-KhMByB5… | sha256-+0pYwTBh… |
| Proof A (plain) | 1f14a50, cached drv | 56acaea, cached drv |
| Proof B (fresh) | 9858748 marker clone | 3fe2807 marker clone |
| ISO file sha256 | d1e8a9fd5de60cc1… | 9055d76e9bdcb91c… |
| Three-way NAR | A == B == got-value, cmp clean | A == B == got-value, cmp clean |
| Re-wire commit | r9 aa55a27 (flake.nix only) | r9 df4ad78 (flake.nix only) |
| Hosted CI | run 43711744441 SUCCESS, observed BEFORE the advance | run 43717790664 SUCCESS, observed BEFORE the advance |
| Pin advance | harvey dfb6e5ab1 (lock-only) | harvey 4ceb275de (lock-only) |

## Task Commits

1. **Task 1: drawterm-gate.expect — the pty-driven session chassis** - `8b1b0ecd5` (feat, pass 1)
2. **Task 2: the -G gate driver + flake app registration** - `6defbd9eb` (feat, pass 1)
3. **[Rule 3] the cpu image's hostfwd reception seam** - `66f3e9a13` (fix, pass 1)
4. **Task 3 evidence (pass 1)** - `03a11d673` (test)
5. **Task 3 (pass 2): the five-defect kernel chain** - r9 `917685c` (+ `9704b2e`), pushed
6. **Task 3 (pass 3, THIS pass): the concurrency chain + the choreography** — r9 `22d40fb`, `091614a`, `3c67e2a`, `1f14a50`, `aa55a27`, `56acaea`, `df4ad78` (all pushed == origin/main); harvey `c1d8a09b8` (the gate re-key), `dfb6e5ab1` + `4ceb275de` (the two lock-only pin advances)

**Plan metadata:** (this SUMMARY + STATE/ROADMAP — the docs commit follows)

## Files Created/Modified

- `build/x86_64/drawterm-gate.expect` — the live re-key (session-exit, drain, panic-scan)
- `flake.lock` — r9 pin 0a4b853 -> aa55a27 -> df4ad78
- r9 (its own repo, pushed): `port/src/devssl.rs`, `port/src/devmnt.rs`, `port/src/mcslock.rs`, `port/src/devpipe.rs`, `port/src/devip/proto.rs`, `port/src/devip/stack.rs`, `x86_64/src/proc.rs`, `x86_64/src/main.rs`, `port/tests/devmnt_test.rs`, `port/tests/devpipe_test.rs`, `port/src/devip_tests.rs`, `flake.nix`
- Evidence: `evidence/19-08-gate-pin-a.log`, `19-08-gate-pin-b.log`, `19-08-gate-tcg.log` (all redacted)

## Decisions Made

See key-decisions. Additionally: the carried list's "isolate the boot race against the two named suspects first" was resolved by CONSTRUCTION rather than reproduction — the rebuilt design makes suspect (a) unrepresentable (the token spans framing; there is no release point between header and body) and suspect (b) moot (no MCS lock exists on any park path). The residual tier1 race the rebuilt tree exposed was a THIRD defect class (error-on-full writes), root-caused from its capture within three gate runs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Kernel concurrency] devssl's single-conv lock blocked writes across a parked read**
- **Found during:** Task 3 (the root-caused record's named defect, layer 1)
- **Fix:** the direction split (BusyCell cells per direction, io dups per direction) — r9 22d40fb; the five devssl pins passed unchanged

**2. [Rule 1 - Kernel concurrency] devmnt's single-outstanding busy flag deadlocked session exit**
- **Fix:** the donor's tag-mux (queue + slots + the transferable reader token) — r9 091614a; NEW two-thread host pin for the demux class

**3. [Rule 1 - Kernel leak] the session's io chans never closed — the wire never hung up**
- **Found during:** Task 3 (the mux-green run stalled at teardown with the whole service tree exited)
- **Fix:** MntIo::close_chan + Mnt::drop (the donor mntdec's cclose) — r9 3c67e2a

**4. [Rule 1 - Kernel defect] notes never interrupted indefinitely-parked readers (the teardown variance)**
- **Found during:** Task 3 (the same tree tore down once, stalled the next)
- **Fix:** push_note's ready(p) + the notepending probe at the block arms — r9 1f14a50

**5. [Rule 1 - Kernel defect] error-on-full pipe writes killed the root paqfs under the mux**
- **Found during:** Task 3 (the at-the-pin tier1 run: `paqfs: mount write` -> root death)
- **Fix:** the donor's blocking qwrite + the wake paths — r9 56acaea; the pipe-full pin re-keyed

**6. [Rule 1 - Gate defect] the gate file's three never-exercised assumptions**
- **Fix:** the live re-key (the exportfs-hangup teardown marker, the glob drain catch-all, the panic scan's drawterm-exit exclusion) — harvey c1d8a09b8; validated by the green runs

---

**Total deviations:** 6 auto-fixed (5 kernel Rule-1 + 1 gate Rule-1). **Impact on plan:** all six were prerequisites for the gate's green; no scope creep.

## The prior passes' halt records (preserved)

### Halt 1 (pass 1, 2026-11-15): the pinned-kernel + auth_chuid boundary
Tasks 1-2 committed; the gate's session died at the -O auth leg against the pinned kernel — superseded by pass 2's root-cause (the five-defect chain). The detail lives in git history (49a1fca39's SUMMARY record) and the pass-1 evidence files.

### Halt 2 (pass 2, 2026-11-15): the session-exit deadlock
The session ALIVE through prompt + fed-echo at pushed r9 917685c, but session-exit deadlocked: devmnt's busy flag across the parked reply read + devssl's Dstate lock across the wire read — both root-caused with donor-faithful designs recorded; the pass-2 rework attempt (tag-mux + split) had been BUILT and reverted for a boot race whose named suspects (the mid-frame release window; the tick-retry vs the parked read) this pass eliminated structurally. The five-defect chain's record: q35churnshell +cap, Pipe::mnt_session srv re-mount reuse, DEVSSL chans as 9P connections, Chan::dup's #D refc rule, the q35cpu re-key (r9 917685c).

## Issues Encountered

- **The tier1 host-build flake (1 of 4 runs):** the userns tier build DIED mid-tier3 (the log stops mid-rio; the driver's last-step report named the wrong entry). A host-side flake, not the tree — the immediate rerun was green with identical inputs. No action.
- **The note-interrupt boundary, recorded honestly:** the OTHER blocking sites in devip (connect/listen/write-full arms) now WAKE on notes (push_note's ready(p)) but do not yet surface the interrupt at their re-block arms — pre-existing behavior for those paths, no measured caller this era; the data-read pair (tcp + icmp + devpipe read/write) is the -G session's whole surface and is fully interruptible. The boundary is documented in r9 1f14a50's commit message.
- **D-09 record:** no triple-fault/capture-context firing occurred (the waived #66/CR-07 class never fired in any run this pass — the full 8-way concurrent bulk transfer is not this gate's shape).
- The drawterm wrapper's `-s ''` gap (the pass-1 note) stands for 19-09/19-10's human sessions.

## Known Stubs

None - no stubs, placeholders, or unwired data paths were introduced.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- 19-09 (--system=cpu boot mode) proceeds at the df4ad78 pin — the same §15 image, and now on the concurrency-corrected kernel (the tag-mux + blocking writes are its substrate)
- 19-10 (DESK-07 evidence) UNBLOCKED — the gate is green twice with retained captures
- The next r9-touching pass re-runs the FOD choreography (any kernel-byte commit re-stales it — the standing drift class)

## Self-Check: PENDING (appended below after verification)

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Completed: 2026-11-15 (the third pass closed the plan; the two halt records preserved above)*

## Self-Check: PASSED

- Created/modified files exist: the gate expect, the driver, flake.lock, the three retained captures (pin-a/pin-b/tcg), the SUMMARY — FOUND
- Task commits exist: harvey 8b1b0ecd5, 6defbd9eb, 66f3e9a13, 03a11d673 (prior passes), c1d8a09b8 (the gate re-key), dfb6e5ab1 + 4ceb275de (the two lock-only pin advances) all ancestors of HEAD; r9 22d40fb, 091614a, 3c67e2a, 1f14a50, aa55a27, 56acaea, df4ad78 all ancestors of origin/main (PUSHED-CLEAN, HEAD == origin/main)
- The pin: flake.lock r9 rev == r9 origin/main (df4ad78, LOCK-EQUALS-R9-ORIGIN-MAIN); both pin-advance commits are lock-only
- Plan verify legs at close-out: GATE-A-GREEN + GATE-GREEN-TWICE + REDACTION-OK (both captures) + the TCG leg GREEN — all at the df4ad78 pin; hosted CI runs 43711744441 / 43717790664 SUCCESS observed before each advance; battery 224/224 + xtask check green at the final r9 tree; composed gates green (smoke/tier1×3/churn/auth + the q35cpu boot profile set)
