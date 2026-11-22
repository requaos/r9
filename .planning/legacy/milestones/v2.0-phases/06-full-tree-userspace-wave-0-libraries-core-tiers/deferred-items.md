# Phase 6 — Deferred Items

Items discovered during execution that are pre-existing and NOT caused by the
current task's changes (executor scope boundary — logged, not fixed).

## Deferred Items

- macexpand expansion path cannot carry a >16KB macro
  status: open
  **Found during:** 06-01 Task 1 (gethunk repro input-shape spike)
  **What:** `cc/macbody`'s `macexpand(s, b)` writes the expansion into
  `Io.b[BUFSIZ=16KB]` (`cc.h:154-161`) with no output bound, and the io
  layer's length field is a `short` — a macro whose expansion exceeds 16KB
  compiles the define fine (post-gethunk-fix) but fails at the use site
  with a misleading "saw $end"-class syntax error (observed: `bigmacro.c:3
  syntax error, last name: 1` on a >600KB macro use with the FIXED 6c/7c).
  Pre-existing upstream limitation, orthogonal to the gethunk hang (which
  fires at define time); no in-tree source defines macros anywhere near
  this size.
  **Status:** open — fix only if a real consumer ever needs >16KB macro
  expansion.

- darwin host sbrk arena is capped at exactly 4096KB per process
  status: open
  **Found during:** 06-01 Task 2 (A4 input-shape spike)
  **What:** measured on macOS 14 SDK (apple-sdk-14.4, darwin-arm64): sbrk
  fails after 4096KB regardless of chunk granularity. Any host-side gate
  needing a process to grow the break past ~4MB (e.g. a fixed gethunk
  linker accumulating past its 2.5MB ceiling, ~5MB total) cannot run
  natively on darwin — route through linux (native or lima guest), as
  `build/aarch64/gethunk-repro-6l.sh` does.
  **Status:** open — platform fact, venue constraint for future
  gethunk-class gates.

- 6l (and likely 8l/5l/9l/ql/vl/4l/kl) share the 7l LP64 double-constant
  emission bug — latent, no current consumer
  status: open
  **Found during:** 06-03 Task 1 (the hawk float-canary fault → 7l fix)
  **What:** 7l's datfill D_FCONST case-8 emitted double constants via a
  cast over `struct Ieee {long l; long h;}` — 16 bytes on LP64 hosts,
  so only l's low half + l's zero high half were written (fixed in 06-03
  for 7l, commit `2bbca81ed`).  6l's asm.c D_FCONST case-8 has the same
  shape (`cast = (uchar*)&p->to.ieee` with Ieee EMBEDDED in the Adr
  union — 16 bytes on LP64, fnuxi8 reads only the first 8 = l's bytes).
  Latent: the amd64 9k build carries no non-trivial double data
  constants, so nothing in the default build consumes the path (v1.0
  never printed a float anywhere).  Fix when an amd64/386 consumer
  appears, using the 06-03 7l fix as the pattern (emit the two 32-bit
  halves via int[2]); the dormant-arch letters (8l/5l/9l/ql/vl/4l/kl)
  should be audited at the same time.
  **Status:** open — latent toolchain class, deliberately not fixed in
  06-03 (amd64 leg kept byte-untouched; scope boundary).

- tier cmd binaries have no committed anti-staleness mechanism after a
  toolchain change
  status: open
  **Found during:** 06-03 Task 1 (the 7l LP64 fix's revalidation)
  **What:** mk's freshness graph never names the toolchain binaries, so
  after a toolchain fix (e.g. the 06-03 7l fix), tier-1 cmd binaries'
  `mk install` relinks nothing — stale objects produce stale binaries.
  D-07's fresh-archive rule covers only the tier-0 `.a` archives.  The
  06-03 tracer re-run was made honest by the guest tree's .7-clean state
  (vm-run's host→guest `--delete` rsync swept them) + the fixed 7l
  copied into the guest hostbin — an operator step, not machinery.
  06-05/06-06's mass sweeps should decide: per-entry `mk clean` in the
  tier loops, a committed object sweep on toolchain change, or a
  documented operator step.  donor-vm-build.sh step 4b's sweep is the
  in-tree precedent.
  **Status:** open — needs a decision by the first mass tier wave after
  any toolchain change.

- libmemdraw drawtest + arctest fault on aarch64 (Pitfall-3 class) —
  Phase-10 per-dir checklist
  status: resolved
  **Found during:** 06-05 Task 2 (the tier-0 wave's D-06 lib test runs
  on the 9front donor)
  **What:** the in-tree tests compiled and ran but FAULT on aarch64:
  drawtest abort()s its own stochastic memimagedraw pixel verification
  (fault pc 0x1ab50 symbolizes inside abort() — the null read is abort's
  mechanism; the verification detected a real pixel mismatch), and
  arctest takes a wild-pointer write (fault write addr=0x1c001374bc)
  with the pc inside libmemdraw's memsetl (LP64-fossil signature).
  Evidence: 06-EVIDENCE.md Phase-10 checklist +
  evidence/06-05-libtest-donor.log.  Filed per the phase's no-drive-by
  rule (Pitfall-3 classes are Phase-10 audit work, not sweep drive-bys);
  both tests rejoin the standing sweep when the lib is fixed.
  **Status:** resolved — plan 15-09 (2026-11-10) root-caused both as
  arch-independent C bugs and fixed in-tree as two class commits:
  drawtest = compositing-arithmetic mismatch (the test's reference
  truncated while the library rounded, and the byte-path calculators
  double-rounded — both sides now round-to-nearest, the
  9front/plan9port-converged shape; `506ca9605`); arctest =
  zero-extension fossil in byteaddr (an unsigned 32-bit row-offset
  product for negative rows zero-extends into 64-bit pointers, landing
  exactly +2^32 out — the measured fault addr is base+4GiB; row offsets
  now wrap in int, `e8df41c7d`).  Runtime re-join on the draw venue
  (DRAWTEST-OK / ARCTEST-OK) is 15-06 Task 3's stage.

- stale mkfile test targets reference absent sources (liboventi,
  libmemdraw)
  status: resolved
  **Found during:** 06-05 Task 2 (lib test-binary discovery)
  **What:** liboventi's `$O.vttest` + `test:` targets reference a
  `vttest.c` that does not exist; libmemdraw's `$O.test` references a
  `test.c` that does not exist (only drawtest.c/arctest.c are real).
  Harmless unless the targets are invoked.
  **Status:** resolved — plan 15-09 (2026-11-10) removed the stale
  entries (plus libmemdraw's same-class `$O.drawtime` over a nonexistent
  drawtime.c) while keeping the real `$O.drawtest`/`$O.arctest` targets
  verbatim (`9ecac570b`); liboventi builds green from the cleaned
  mkfile.

- libdynld dynld.c:128 "useless or misleading comparison: ULONG < 0"
  (7c note)
  status: open
  **Found during:** 06-05 Task 1 (the dynld-aarch64 rename probe build)
  **What:** the only compiler diagnostic of the 22-lib wave; the lib
  archived green.  A real Pitfall-3-class suspicion (unsigned compares)
  for the Phase-10 per-dir audit.
  **Status:** open — Phase-10 checklist.

- 06-05-PLAN's literal Task-2 no-PENDING-CLASSIFY grep contradicts its
  own Task-1 seed step under the ledger's append-only rule
  status: open
  **Found during:** 06-05 Task 2 (classification)
  **What:** Task 1 step 5 mandates exec=PENDING-CLASSIFY seed lines and
  D-14 makes the ledger append-only, so superseded seeds remain as
  history; Task 2's verify greps the WHOLE file for zero
  PENDING-CLASSIFY and can never pass after a seeded run.  The intent
  gate (latest line per dir per the ledger's own last-line-wins rule)
  passes and is what 06-05 ran.  Same class as 06-02's recorded gate-2
  text defect.
  **Status:** open — the next planning pass should repair the gate text
  (latest-line-per-dir form).

- rc argv-stack discipline: residual drain sites beyond Xbackq (06-07)
  status: open
  **Found during:** 06-07 Task 2 (the r9 tier-1 gate — the FIRST time
  the full sweep ran under Harvey's OWN rc; the donor gate drives
  9front's rc)
  **What:** rc's Xbackq parent arm poplist()ed the split word's list
  without a guaranteed list below; at a top-level backquote assignment
  the stack drained to argv==0 and the next op crashed ("pushword but
  no argv!").  06-07 fixed the Xbackq site (cebb127ae — verified: the
  sweep-killing cascade gone, 10/10 T1-DONE, the gate's capture clean).
  The RESIDUAL class: any OTHER opcode path that could drain the argv
  stack (the panic text was observed on rare occasions post-fix in
  helper subshells, never eating a marker).  Present unchanged in
  plan9port's rc; 9front rewrote the path.  A Phase-10 per-dir audit
  item: audit rc's pushlist/poplist balance across every op (Xsubshell,
  Xpipefd, Xassign, Xqdol, ...).
  **Status:** open — Phase-10 checklist.
  **06-09 update:** the residual class DID eat markers during 06-09's r9
  parity runs — run 1 took the pushword panic at the tr check (missing
  T1-OK tr; sweep continued), run 5 died at the float-canary stage
  (sweep rc exited '<bad utf8>').  Both on a host under parallel TCG
  load; the 06-09 sweep edits are byte-neutral on the r9 path
  (execution-trace parity proven host-side: rc -x traces of the 06-07
  and 06-09 sweeps with venue=r9 are byte-identical modulo filename and
  pids).  The audit item stands, sharpened: the class is live under
  load, not just in helper subshells.

- plan9port mk: missing-intermediate prereq tolerated when the install
  target exists
  status: open
  **Found during:** 06-09 (the venue binary relink iteration)
  **What:** `mk <name>.install` in sys/src/cmd reports '<name>.install
  is up to date' when the in-tree `6.<name>` is DELETED but
  `/amd64/bin/<name>` exists — the missing intermediate prereq is
  treated as time-0, so the existing install target is 'newer'.  The
  relink silently never happens (the venue then embeds stale binaries).
  Contrast: GNU make errors on the unbuildable prereq.  Iteration rule:
  delete BOTH the in-tree binary and the installed copy (or run the
  full clean-checkout build).  Affects any local 'refresh one binary'
  workflow; CI's clean-checkout path is unaffected.
  **Status:** open — documented in 06-EVIDENCE.md's 06-09 section;
  no tree change (the mk behavior is plan9port's; the workaround is
  procedural).
