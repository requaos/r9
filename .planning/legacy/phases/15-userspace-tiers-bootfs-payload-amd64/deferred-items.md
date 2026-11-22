# 15-REVIEW Deferred Items

From the phase 15 code review (15-REVIEW.md WR-07; recorded next to
the MUL0123 lane-path deferral the 15-09 summary already carries):

- libmemdraw's MUL2 (draw.c) is exactly (s+127)/255 for coefficient
  sums s <= 255*255 — the whole reachable range for convex (no-alpha /
  premultiplied) composites, which covers every channel configuration
  the venue rig runs (r8g8b8-family).  With an alpha-bearing source
  the pair is non-convex (M = 255-round(sa*m/255) leaves m+M > 255
  whenever sa*m <= 127), s reaches 130050, and on 253 values in
  (65025, 130050] the shift-form macro differs from the ideal — and
  from drawtest's exact-division reference — by 1 LSB (first at
  s=65663: 257 vs 258, i.e. 1 vs 2 after byte truncation).  Byte-path
  drawtest runs with alpha or mixed channels (e.g. `r8g8b8a8
  r8g8b8`) can therefore still abort on those values.  The 15-09
  summary's "any channel arguments the rig passes are equally covered
  on the byte path" claim is thereby scoped to the convex regime the
  rig exercises; the residue is now documented at both sites (draw.c's
  MUL2 comment and drawtest.c's reference comment).
  status: open
  **What:** MUL2 vs round-to-nearest diverges by 1 LSB on 253 values
  above s=65025 (non-convex alpha pairs only); whichever plan next
  touches libmemdraw's byte path should adopt a clamped two-term
  shape whose identity holds to 130050.

# 15-08 Deferred Items

Out-of-scope discoveries from the DESK-03 payload staging (measured
live this plan; worked around in-plan — the fixes own later plans):

- The tier-1 witness manifest is TREE-HISTORY-SENSITIVE for the three
  tier-1 directory entries that carry yacc grammars (cmd/rc syn.y,
  cmd/grep grep.y, cmd/awk awkgram.y).  Measured sequence (15-08):
  the tree matched the 14-08-era pinned digests before the plan's
  first tier build; that FIRST build produced different bytes for
  exactly rc/grep/awk; every subsequent tier build (4+ runs, two
  consecutive-driver-runs compared) reproduced the ORIGINAL digests
  byte-for-byte.  A yacc root-single (cmd/units) never drifted, so
  the class is dir-entry yacc builds, not yacc generally.  Mechanism
  undetermined (candidates: stale generated y.tab.* state left by a
  prior build route such as the 15-07 coverage sweep's mk-all, or
  archive/regeneration order inside the first per-entry fresh build
  after such a route).  Consequence: any /amd64 writer between a
  donor build and the next mkimage reds the witness (mkimage 9a), and
  the remediation ORDER matters — run the tier driver to convergence
  FIRST, then re-pin via build/mm/build-9k10t1.sh (15-08 initially
  re-pinned from the one-off bytes and the next gate run re-reded —
  the working order is driver-then-pin).
  status: open
  **What:** one tier-driver run produced different bytes for
  cmd/{rc,grep,awk} from identical sources (then converged); whether
  a prior build route's leftover state caused it is unestablished —
  root cause open.

- build/x86_64/tier1.ledger still carries its two 14-08-era
  `exec=SKIP:r9x64-…` rows (ipconfig no-devrandom, factotum
  no-auth-era) although 15-03 flipped both subjects live in the same
  commit as every gate expectation — the r9 tier-1 gate has expected
  38 T1-OK / zero SKIP since, and every 15-08 re-run shows 38.  The
  ledger is a stale RECORD, not a gate claim, but it contradicts the
  live gate; a fix owns the ledger's format (add the flip rows /
  annotate) rather than a quiet edit.
  status: open
  **What:** build/x86_64/tier1.ledger rows for cmd/ip/ipconfig and
  cmd/auth/factotum:factotum predate the 15-03 flips.

- The expect gates' panic fast-fail (`expect_before -re {panic|…}` in
  build/x86_64/services-r9.expect and siblings) kills qemu the
  instant the word "panic" appears on the wire — before the panic's
  message and location finish printing, so the retained capture never
  contains the kernel's own diagnosis text (15-08's stash-clamp panic
  arrived as a bare `panicked at`; the "grow the stash" message had
  to be read out of the r9 source).  Letting the line drain (a short
  post-match expect before cleanup) would put the kernel's text on
  the wire.  Gate-side diagnosability improvement; owns whichever
  plan next touches the expect drivers.
  status: open
  **What:** the fast-fail arm truncates panic messages mid-line.

- mkimage's per-file cap assert (11r, 15-08) covers the SERVICES bake
  only; the marker/chain/tier/net bakes in mkimage.sh and
  build/mm/build-9kdraw.sh's bootfs-draw.paq still ride the mkpaqfs
  default blocksize with no fit assert — the standing 15-07 hazard
  item below remains open for those bakes (their current members all
  fit; fonts max 46089 bytes, binaries under the cap).
  status: open
  **What:** the other four mkimage contracts + build-9kdraw.sh lack
  the cap assert (15-07's item, narrowed by 15-08 to these).

# 15-07 Deferred Items

Out-of-scope discoveries from the coverage sweep + gs render smoke
(pre-existing behavior, measured live; both worked around in this
plan's own artifacts — neither is this plan's surface to fix):

- mkpaqfs silently DROPS any file larger than blocksize^2/4 bytes:
  paqFile() warns ("file too big for blocksize") and returns nil, the
  file vanishes from the image, and mkpaqfs still exits 0.  At the
  4096 default blocksize the cap is 4 MiB — the 15.5 MB gs binary
  disappeared from bootfs-gs.paq and the guest saw no /bin/gs
  (measured on the gs smoke's third boot).  Every OTHER bake
  (build-9kdraw.sh's bootfs-draw.paq, the trace rig, mkimage's
  payloads) rides the default blocksize with no stderr check: a
  payload member growing past 4 MiB vanishes with only a stderr line.
  15-07's own harness asserts the fit and fails on any mkpaqfs
  diagnostic; the general fix (make the drop fatal in
  sys/src/cmd/paqfs/mkpaqfs.c, or assert sizes in the shared bake
  helpers) owns a plan that touches the payload machinery.
  status: open
  **What:** sys/src/cmd/paqfs/mkpaqfs.c paqFile()/paqDir() warn-and-drop.

- rc's Execute prints only the LAST path candidate's errstr when a
  command is not found (sys/src/cmd/rc/plan9.c Execute: the loop
  overwrites errstr per candidate), so `gs: './gs' file does not
  exist` masked the real /bin/gs failure behind the trailing-dot
  candidate.  Diagnosability wart (the first candidate's or the
  matching candidate's error would say what actually failed); worked
  around in-guest by absolute-path invocations.  A fix owns rc's
  plan9.c.
  status: open
  **What:** sys/src/cmd/rc/plan9.c Execute errstr overwrite.

# 15-06 Deferred Items

Out-of-scope discoveries from the trace-rig bring-up (pre-existing
behavior, measured live on the draw donor venue; the rig works around
both — neither is this plan's surface):

- samterm's input machinery is pacing/protocol fragile around file
  exchanges: the menu-New path leaves `which` unset pending the host's
  Hcurrent (a second B3 press re-opens the menu instead of entering the
  sweep — the +1px menuhit warps on the capture pin it), and after a
  `B` command whose reply leaves a non-empty dot on the command window,
  `type()` takes the cut-selection branch and never consumes the pending
  runes (the first typed line after the file command stalls behind the
  kbdblock mask).  The committed replay-sam.rc drives the proven path
  (B on a missing name + a B1 dot-reset click); a real fix owns
  samterm's io.c/main.c state machine.
  status: open
  **What:** sys/src/cmd/samterm/{io,main,menu}.c input-state handling.

- Harvey's amd64 runtime epilogue calls `exits("main")` for programs
  that fall off main (sys/src/libc/amd64/main9.s: `CALL exits` with
  RARG=$_exits<>` — a NON-EMPTY failure status to rc's `&&`), where
  stock Plan 9 exits with a nil/empty status.  arctest.c (no explicit
  exits) is the measured case: its timings print, its exit status is
  "main", so `cmd && echo` never fires.  The rejoin gates on the
  completion output instead (15-06); a runtime-level decision (fix
  main9.s or audit the fall-off-main programs) belongs to a plan that
  owns the libc runtime.
  status: open
  **What:** sys/src/libc/amd64/main9.s epilogue status.
