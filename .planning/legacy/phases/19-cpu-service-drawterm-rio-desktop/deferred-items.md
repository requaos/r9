# Phase 19 deferred items

## [Phase 19 review WR-02 — DEFERRED, 2026-11-17] FdGroup::op_done merges the streaming offset into a possibly REFILLED slot

From 19-REVIEW.md WR-02 (r9 `port/src/fdgrp.rs:120-125`): `op_done(fd,
c)` writes `slot.offset = c.offset` into whatever chan currently
occupies the fd.  The comment covers only the "slot emptied mid-op"
case, but a sibling in a SHARED fd group can also close(fd) and then
open() a different file into the same slot while the original op is
blocked in pread/pwrite — exactly the interleave `op_ref` was built
for.  On wake, the blocked op's advanced offset is merged into the
unrelated new chan, silently jumping its streaming position; the
`store()` comment's impossibility claim is contradicted by `op_ref`'s
own documentation, so the guard-free merge is not covered by an
impossibility argument.

Deferred, not fixed in the 19 review-fix pass: the fix is r9 KERNEL
BYTES (merge the offset only when the slot still names the SAME file
identity — compare `(dev, devno, qid.path)` of the op chan and the
slot chan, or `Arc::ptr_eq` on `mnt_fid` for DEV_MNT chans; otherwise
`drop_op_ref` and discard the offset), which requires the full D-11
pin choreography (push-before-pin, two-derivation FOD proof,
lock-only advance, hosted CI, battery, gates re-green).  The next
r9-touching pass owns it; fix shape per the review's Fix section.

## [Phase 19 review IN-01 — DEFERRED, 2026-11-17] devssl secrets and RC4 state are freed without zeroization

From 19-REVIEW.md IN-01 (r9 `port/src/devssl.rs:160-176` OneWay and
`:1019-1043` close): on the last close, the conv's `Vec<u8>` secrets
and the `Rc4state` permutation drop into the kernel heap without
being zeroed first, so the p9sk1 session keys (the dec64'd mksecret
material) remain readable in freed kernel memory until the pages are
reused.  Donor parity (the donor's sslclose `free`s without memset)
and the hardening milestone owns crypto upgrades — deferred there.
Fix shape when it lands: zero `OneWay::secret` and `Rc4state::state`
on direction drop (a `Drop` impl or an explicit zeroing in `close()`'s
last-ref arm).

## [19-10 Leg-1 continuation 6 — LANDED, 2026-11-17] the window never deleted on shell exit: TWO r9 defects + one exposed regression, all fixed; `exit` now deletes the window AT THE PINNED TREE

The owner's report at c787ce9 (everything else green): `exit` in a
window kills the shell (prompt gone) but the window stays and still
echoes typed input.  Reproduced with xdotool; three instrumented-kernel
(diag serial prints) rounds isolated the chain — pointer (a) wait()
was EXONERATED early (the serial showed winshell's _schedexecwait
completing: "process N exited: procexec"), and the break was in the
fd/fid close chain:

1. **exit_tail's fd drain closed under a borrow** — every duplicate of
   one server fid inside the dying fd table pinned the Arc count, so
   clunk_if_last never fired (measured: 4 skips at count=2 on the wsys
   /dev/cons chans, zero wsys Tclunks).  The donor closefgrp nils each
   slot BEFORE its cclose.  Fix: ownership-consuming drain.
2. **The donor closepgrp's mount half was never ported** — a namespace
   group's death dropped the mount table as plain Rust drops; the
   mounted tree's ROOT/attach fid never Tclunked.  rio's wsys attach
   carries a window ref (xfidattach's `if(!newlymade) incref(w)`), so
   even with both cons fids clunked the refcount stayed one short and
   wclosewin never ran.  Fix: Pgrp::take_mount_chans + the
   last-holder drain in sweep_husk_payloads.

**The regression the first pin exposed** (drawterm-gate FAILED at
session-exit, rc=1): the new exit-time clunks race the teardown kill
notes; an interrupted Tclunk's LATE reply hit the loud 'unexpected
reply tag' error and drained a HEALTHY waiter's rpc (capture preserved
in the gate log).  r9 1cbc1db restored the donor mountmux DROP arm (a
late reply is logged and dropped, never fatal); the tag-mismatch pin
re-keyed to the drop contract + the new
mux_foreign_reply_is_dropped_and_live_waiters_still_complete pin.

Landing (the full choreography, twice for the two FOD rounds):
- r9 ab0a977 (pushed, CI SUCCESS 43983837655) — the exit-deletion fix
  + the namespace_test take_mount_chans pin; battery green.
- r9 1cbc1db (pushed, CI SUCCESS 43989749900) — the drop arm; battery
  green (port 231 + x86_64 104).
- r9 0921a1b (pushed, CI SUCCESS 43990489795) — the FOD re-wire;
  two-derivation proofs (ab0a977+marker accf880; 1cbc1db+marker
  06b94f8; byte-identical ISOs both rounds).
- harvey 3f449fd72 — flake.lock 543c375 -> 0921a1b lock-only.
- Gates at the final pin: drawterm-gate ALL STAGES PASS, smoke
  (chain+cpu) ALL STAGES PASS.

**The proof at the pinned tree** (evidence
19-10-leg1-cont6-proof-record.txt + -exit-deletes-atpin.png): two
windows swept, `exit` typed into window 1 — window 1 DELETED (its area
repaints desktop grey), window 2 unaffected.  On the instrumented
precursor the clunk trace shows the whole wsys session's fid set
Tclunking at shell exit (cons fids 3+4, mount root 2, attach 1).

The one remaining residual is unchanged (the login-time desktop
auto-start skip on this session's app-baked venues; manual
`plumber; rio` green — see continuation 4's record).

## [19-10 Leg-1 continuation 5 — LANDED, 2026-11-16] the stuck-contextual-cursor residual: root-caused (the swallowed zero-count Twrite), fixed at r9, the REVERT PROVEN with real sprite observation at the pinned tree

The owner's live report at 3e5ed0c (everything else green): the
contextual edge/corner cursor shapes fire, render, and track — but the
cursor never RETURNS to the arrow off the edges; it sticks at the last
contextual shape over interiors and background.

**Root cause (r9 devmnt):** the revert rides a ZERO-LENGTH write —
libdraw setcursor(nil) writes 0 bytes (write(cfd, curs, 0)) and
drawterm's devmouse carries the arrow reset on exactly that event
(Qcursor n < 72 -> cursorarrow()).  The donor mntrdwr is a DO-WHILE:
even a zero-length write sends one Twrite with count=0 and the server
OBSERVES it.  r9's devmnt::write was `while done < buf.len()` — a
zero-length write sent NOTHING and returned Ok(0): the revert
"succeeded" client-side (riosetcursor set lastcursor=nil and never
retried) while the server never saw it.  This also retro-explains a
flaw in continuation 4's proof claim: every retained trace since the
first wedged session carried ONLY n=72 cursor writes — no revert ever
reached the server on ANY session, wedged or fixed (the "arrow cursor
at the end" sightings in cont-4's screenshots were the vision tool
reporting a cursor that X screenshots do not capture — the sprite is
X-side; import/xdotool capture the framebuffer only.  The window/menu/
second-window claims stand — that is framebuffer content).

**Landing (the full choreography):**
- r9 501a7c5 (pushed, CI SUCCESS 43958057930): both devmnt::write and
  ::read re-keyed to the donor do-while (at least one rpc always; a
  zero-length request is a real protocol event).  Pin:
  port/tests/devmnt_test zero_length_write_sends_the_zero_count_twrite
  (scripted-server writes-log proves the empty write lands as ONE
  observed zero-count Twrite; RED evidence
  19-10-leg1-cont5-zerowrite-red.txt).  Battery: check+clippy clean,
  port 229, x86_64 bins 104.
- r9 c787ce9 (pushed, CI SUCCESS 43959845447): the FOD re-wire —
  two-derivation proof kernel A 501a7c5 + kernel B marker 7c275a5,
  byte-identical ISOs (file sha256 b3ae4de3…, NAR sha256-YaslMB…
  three-way agreed; FOD realization green).
- harvey d109744b2: flake.lock 3e5ed0c -> c787ce9 lock-only.
- Gates at the pin: drawterm-gate ALL STAGES PASS, smoke (chain+cpu)
  ALL STAGES PASS.

**The revert proof at the pinned tree (evidence
19-10-leg1-cont5-cursor-proof.txt + -cursorfix screenshot):** cursor
observed via XFixesGetCursorImage (the REAL X sprite — not
framebuffer pixels): interior after sweep = the default (sum 485048…),
right edge = rio's 'r' shape (sum 121645…), BACK INSIDE = the default
again, TL corner = rio's 'tl' shape, bare desktop = the default; and
the host-side trace now carries n=0 Qcursor writes — the first ever
observed on this venue — in 1:1 correspondence with the revert
moments.  Window 2 swept clean after (both windows in the screenshot).

The ONE remaining residual is unchanged (the login-time desktop
auto-start skip on this session's app-baked venues; manual
`plumber; rio` green — see continuation 4's record below).

## [19-10 Leg-1 continuation 4 — LANDED, 2026-11-16] the post-sweep mouse death: root-caused at two layers, BOTH fixes landed at the pin, the two-window proof GREEN at the pinned tree

STATUS UPDATE (supersedes the resume block below): everything in the
resume block executed.  Landing state:

- **r9 99714a6** (pushed, hosted CI SUCCESS run 43941101250): the
  close-on-exec port — Chan.cexec recorded at namec::open_chan (the
  donor Aopen arm; the Acreate arm deliberately not carried, see the
  commit message), carried by dup/op_clone, FdGroup::take_cexec, and
  exec_replace's close_on_exec_fds sweep in the donor's sysexec order.
  RED→GREEN pins retained (19-10-leg1-cexec-red.txt + the two test
  fns in-tree).  **r9 3e5ed0c** (pushed, hosted CI SUCCESS run
  43942790544): the boot-iso FOD hash re-wire — two-derivation proof
  kernel A plain 99714a6 + kernel B marker clone 609a5e9, fresh
  derivations, byte-identical ISOs (file sha256 44146f06…, NAR
  sha256-4xttiQ…, three-way agreed; FOD realization green).
- **harvey d70c4efb6**: the profile guard fix (the FLAT dialect-safe
  shape — single-condition ifs + wsysup flag; kills the ghost rio).
  **harvey 6d6190676**: flake.lock advanced 56109c3 -> 3e5ed0c
  lock-only.
- **Battery + gates at the pin**: r9 port 164 lib + 228 all-targets +
  x86_64 104 bins zero failures; harvey drawterm-gate ALL STAGES PASS
  and smoke (chain + cpu legs) ALL STAGES PASS at 6d6190676.
- **The two-window proof at the PINNED tree** (evidence
  19-10-leg1-cont4-proof-record.txt + -twowindow.png +
  -twowindow-atpin.png): window 1 sweeps with its prompt, the ARROW
  CURSOR restores, the B3 menu REOPENS, window 2 sweeps — both windows
  live; the mouse chain alive end-to-end (39 read parks, cursor writes
  present; the wedge signature absent).

OPEN RESIDUAL (one, clearly named — the login-time desktop auto-start
skip): on app-baked venues from the 13:2x runs onward (INCLUDING the
pinned venue), the windowed session lands at a cpu% shell and the
desktop does NOT auto-start — `plumber; rio` at the prompt brings it up
clean and everything downstream is green.  The skip reproduced with
BOTH the old and new profiles (NOT caused by the guard change), did
not reproduce on the owner's 10:19 venue, and pairs with a foreground
`rio` failing `can't open display: screeninit failed` after a first
failed attempt.  Mechanism open; first suspects: the §15 bake's
draw-suite staging (the app-bake path's products differ from the
direct-bake path's — the 18-06 awk-shadowing class is the recorded
sibling for auth) or a drawterm-side per-connection devdraw state
quirk.  Reopener: the owner's Leg-1 re-presentation boot (if the
desktop auto-starts there, the skip is venue-of-the-day; either way
`rio` at the prompt is the documented fallback) or the next §15-bake
plan.  The hazards section below (bake-path, background-qemu,
pgrep-self-match, XTEST typing) remains load-bearing for whoever
drives this venue next.

## [SUPERSEDED — the original continuation-4 record, kept for the audit trail]

## [19-10 Leg-1 continuation 4, 2026-11-16] the post-sweep mouse death: ROOT-CAUSED at two layers; BOTH fixes implemented but UNCOMMITTED (live end-to-end proof blocked by the session's bake-pipeline instability — see the resume block)

Owner report (at pin 56109c3, after the identity fix held — window created,
prompt in-window, keystrokes landing): after the right-button sweep, the
cursor does NOT return to the arrow (stuck as the sweep crosscursor —
screenshot-verified on my repro) and the button-3 menu cannot be opened for
a second window. Owner's live discriminator answers: the cursor still MOVES
(as the cross), typing still lands. Cursor motion is X-native and proves
nothing about delivery — the decisive probes below were run live instead.

### The wedge signature (reproduced 4x: owner's pin kernel, CEXEC-fixed
### kernel, and two earlier kernels; host-side instrumented drawterm)

Evidence `19-10-leg1-cont4-drawterm-wedge-trace.log` (every mouse event of
the sweep is served one-by-one; at the sweep's release):
1. rio's sweep-exit `moveto` write arrives at drawterm ("m720 524").
2. Exactly ONE further mouse record is served — the XWarpPointer motion
   the moveto itself generated.
3. The guest then NEVER issues another /dev/mouse read. A B3 press piles
   up unserved (drawterm's ring advances, no reader).
4. No cursor-restore write ever happens (no Qcursor write after the
   sweep-exit moveto) — hence the stuck crosscursor.
5. Draw writes from other threads and the keyboard path keep working.

### Guest-side discriminators (evidence 19-10-leg1-cont4-guest-probes.txt)

- `cat /dev/mouse` on the wedged session: NO Einuse — **the outer rio's
  mouse fid was CLUNKED** (measured on BOTH the owner-pin kernel and the
  CEXEC-fixed kernel — the fd loss is pre-existing, not caused by my fix).
- The proc map: rio's threads are userspace threads inside pid 51; the
  sweep creates a proc burst (70-76) that is NOT just winctl/winshell:
  **every window's shell rc -i re-sources the cpu profile and — because
  the profile's guard checks `/mnt/term/mnt/wsys` (the stock drawterm
  TERMINAL-side-rio idiom) — misses THIS venue's remote-rio shape
  (winshell's own `mount /mnt/wsys MREPL`, fsys.c filsysmount) and runs
  `plumber` + `exec rio` A SECOND TIME inside every window: A NESTED RIO
  (the ghost).** The ghost's own helper procs + per-window ramfs/plumber
  collateral are the extra "rio"-args procs; pid 71 (dead) is the first
  ghost that died at `initmouse` Einuse. Every -G probe made with a
  draw-serving drawterm ALSO spawned a whole headless rio tree (the ps
  pollution seen on long-lived guests).
- The ghost chain runs over the SAME 9P session: its exec forks
  (RFFDG copies) carry clones of the outer's mouse/draw/cons fds, and
  the outer's parked mouse read holds an op-ref across all of it — the
  fid accounting around that chain is where the outer's mouse fid dies
  (the clunk was measured twice; the exact arm inside the chain was not
  isolated — the kernel-side syscall tracer could not run: a println in
  sys_close kills the boot on this venue, and the serial-magic dump
  trigger (also tried) never got a clean guest — see Hazards).

### Root cause (two independent defects, both required to explain every
### observed fact)

1. **HARVEY (the trigger): the staged profile's cpu case guards the
   desktop exec with the WRONG PATH for the remote-rio venue.** Window
   shells must detect the desktop via `/mnt/wsys` (fsys.c's mount), not
   only `/mnt/term/mnt/wsys`. With the wrong guard every window spawns
   plumber x2 + ramfs + a nested rio that races the outer rio on the
   session's shared devices (draw: re-paints the grey; mouse: the outer's
   fid is clunked somewhere in the ghost chain's fork/exec/fid lifecycle
   — measured; the exact arm open).
2. **r9 (missing kernel semantics): NO close-on-exec.** OCEXEC is masked
   away at open/create and never recorded; exec_replace never closes
   CEXEC fds (donor sysexec's `fdclose(i, CCEXEC)` loop, 9k
   sysproc.c:519-523, was never ported). libthread's procexec depends on
   it: its exec-status pipe write end is `OWRITE|OCEXEC`, and the
   parent's status read only EOFs when the exec'ed child's copy closes
   at exec — without the sweep rio's `new()` parks forever at
   `recvul(cpid)` (mousethread never returns to the event loop). The
   CEXEC fix alone did NOT cure the live wedge (the mouse fd was already
   being lost through the ghost-chain arm) — both layers must be fixed.

### Implemented, pinned, UNCOMMITTED (handoff state 2026-11-16)

- **r9 tree (/home/req/code/r9): the CEXEC port, STAGED.** Chan.cexec
  recorded at namec::open_chan (donor Aopen arm) + carried by
  dup/op_clone; FdGroup::take_cexec (fdclose's per-fd arm);
  exec_replace's close_on_exec_fds sweep in donor order (after args
  committed, before the segment frees). Pins: port lib
  `namec_tests::open_chan_records_oceexec_as_fd_state` (GREEN) and
  x86_64 bins `exec::tests::exec_replace_closes_cexec_fds_and_keeps_plain_fds`
  (RED→GREEN; RED evidence 19-10-leg1-cexec-red.txt). Battery at the
  handoff state: xtask check + clippy green, port 164 lib + 228
  integration total, x86_64 bins green. Patch:
  evidence/19-10-leg1-cont4-r9-cexec-fix.patch. (The donor's Acreate-arm
  recording (sys_create hunk) was implemented, then DROPPED after the
  candidate guest's cpu listener refused dials — that hunk is the only
  delta from a kernel whose venue ran healthy; do NOT re-add it without
  isolating why. The wedge path only needs the OPEN-arm recording.)
- **harvey tree: the profile guard fix, UNSTAGED** in
  build/x86_64/root-tree/usr/glenda/lib/profile (the cpu case now binds
  and exits early when EITHER /mnt/wsys (remote-rio window shells) or
  /mnt/term/mnt/wsys (terminal-side rio) exists; the desktop-exec block
  only runs when neither does). P6's bake asserts still pass (terminal
  rio-free, cpu keeps exec rio + latin1.8). Patch:
  evidence/19-10-leg1-cont4-profile-guard-fix.patch.

### Resume block (the exact next steps)

1. Commit the profile fix (harvey) — mkimage must run via the SANCTIONED
   app path (`nix run .#boot-r9-x64 -- --system=cpu`), NOT from a plain
   shell: bakes run directly in this session's shells produced paqs whose
   guests' cpu listener refused dials (see Hazards; the 18-06 devshell
   finding is the recorded sibling). NOTE: the flake app only sees
   TRACKED/STAGED content — stage or commit the profile first.
2. The r9 CEXEC fix needs the full D-11 choreography BEFORE any pin
   advance (push-before-pin, two-derivation FOD proof, lock-only
   advance, hosted CI observed, battery, gates re-green) — it is kernel
   bytes. The candidate kernel can ride the sanctioned app via
   `nix run .#boot-r9-x64 --override-input r9 path:/home/req/code/r9`
   (proven working this session: the override venue boots, ladder
   completes, -G login green with the fixed kernel).
3. THE LIVE PROOF still owed: on the combined candidate (profile fix +
   CEXEC kernel), drive the windowed session: menu → New → sweep →
   THE CURSOR MUST RESTORE (arrow over the desktop) and a SECOND window
   must be creatable (menu re-open). The windowed password feed needs
   click-to-focus before xdotool type (the plain windowfocus+type flaked
   ~60% this session); the -G pty path is 100% reliable for health
   checks. The prior session ran out of wall-clock before this proof
   (see Hazards for everything that burned time).
4. If the combined candidate still loses the mouse fd (Einuse test),
   the ghost-chain fid-accounting arm is next: the suspects are
   devmnt's clunk_if_last ref accounting around a parked read's op-ref
   concurrent with the RFFDG deep_copy chain of the ghost's execs, and
   devip/devpipe close dispatch in that same chain. The DUMPNOW
   serial-magic dump trigger (uart16550 rx_ring_buffer patch — reverted,
   never commit as-is; a magic println in the RX sweep is boot-safe on a
   wait-mode serial, but see Hazards) plus /proc/N/args typed into the
   window are the working instruments.

### Hazards recorded for the next session (all hit live)

- A `println!` in sys_close KILLS THE BOOT (all-blocked wedge at ladder
  end; wedgedbg dumps on serial). Never instrument close paths with
  prints on this venue.
- The Bash tool's background mechanism runs qemu from a DIFFERENT PATH
  (system qemu without `virtio-net-pci-non-transition`) — use absolute
  /run/current-system/sw/bin/qemu-system-x86_64, and note nohup+disown
  children are reaped nondeterministically at call end; prefer the
  tool's run_in_background for the qemu itself.
- `pgrep -f <pattern>` matches THIS SESSION'S OWN wrapper bash (the
  pattern string is in its cmdline) — kill loops built on it kill their
  own wrappers and can miss real targets. Match on `ps -eo pid,args |
  grep '[q]emu-system'` instead.
- The owner's live guest (qemu 2352707, ports 17010/5670) went down at
  ~12:55 EST 2026-11-16 — NOT killed by this session's commands (every
  kill loop excluded it or pattern-matched only this session's own
  guests); cause unknown (possibly owner action, possibly environment
  process reaping). The canonical ports are free; the owner's reboot
  rides the fixed image once the fixes land.
- xdotool XTEST typing into the windowed drawterm is ~40-60% reliable
  on a fresh Xvfb; click-to-focus first. The -G pty feed (password +
  \r, the gate's exact idiom) never failed once all session.

## [19-10 Leg-1 continuation 3, 2026-11-16] the dual-cons-reader defect: ROOT-CAUSED LIVE and FIXED at the kernel layer (r9 56109c3)

Owner report recap: swept window's shell alternating keystrokes with rio's
keyboard reader, prompt/output painted at the drawterm cons layer's left
edge over the desktop.

**Root cause (proven on the owner's LIVE guest, evidence
`19-10-leg1-livecons-probe.log`):** r9's mount-table consults were keyed
by PATH STRING, not by the mount-point CHAN's identity (the donor
findmount/eqchanddq shape). rio's winshell child (wind.c:1341:
`mount /mnt/wsys MREPL` + `bind -b /mnt/wsys /dev`) then opens
`/dev/cons`; the cpu profile REPL-binds the terminal cons AT `/dev/cons`
(`bind /mnt/term/dev/cons /dev/cons`), so namec's final-component
redirect re-consulted "/dev/cons" BY PATH and replaced the correctly
head-resolved wsys cons chan with the SESSION terminal cons — fds 0/1/2
of the window's rc landed on the session cons (two readers on one cons =
the alternation; rc's prompt to fd 1 = the left-edge painting). The live
discriminator on the owner's guest: `bind -b '#e' /dev` +
`cat /dev/consctl` → `devmnt: server error` (the open was hijacked into
the REPL bind's 9P chan; the control `cat /dev/livprobe` proved the union
head itself was consulted and first-accepting).

**Fix (r9 fe67c75, pin advanced to 56109c3 by flake.lock commit ff21c4786):**
identity-keyed mount points — `Chan.devno` (devmnt stamps each 9P session,
the donor's mntalloc.id++), pgrp's MountPoint keyed by the mount-point chan
(Mhead.from), all five namec consult sites match (dev, devno, qid.path).
Also aligned sys_unmount's two arg resolutions to the donor sysunmount
shape (old→Amount, name→Aopen OREAD; the pre-fix code had them swapped).
Choreography executed in full: host battery green (xtask check + clippy +
port 163 lib + 74 integration incl. the new RED→GREEN hijack pin),
two-derivation FOD proof (fe67c75 plain + fcab10c marker clone:
byte-identical ISOs, file sha256 fb11e6b0…, NAR sha256-BfU8eS…, three-way
agreed), FOD re-wired (r9 56109c3, flake.nix only), pushed, hosted CI
observed SUCCESS at the pushed HEAD (run 43887537787), harvey lock-only
advance.

**Live fix verification on a fresh guest at the new pin** (evidence
`19-10-leg1-livecons-verify.log`; verification guest booted with hostfwd
remapped to 17011/5671 — the owner's live guest held the canonical ports;
cpurc ladder ETHER-OK/LISTEN-17010/DONE + authenticated drawterm session):
the SAME discriminator now returns `HEAD-WINS-CTL` — the union head's
resolution survives the final redirect. Windowed-leg proof rides the
owner's reboot + re-presentation (window creation still needs a real
mouse sweep; the synthetic-mouse block stands).

Residual observations recorded below (the /srv item's attribution
CORRECTED by this pass).

### [CORRECTED 2026-11-16] the "r9 /srv open blocks foreign rio driving" finding

The `mount /srv/rio… → mount: namec: does not exist` line was rc's
EXEC-failure print: **`mount`/`unmount` binaries are not in the §15 cpu
image** (zero occurrences in out/bootfs-cpu.paq's name table; verified
2026-11-16). mount(1) never ran; the /srv open was never reached. The
OTHER leg stands unexplained: writing through a posted /srv file →
`devsrv: fd string too long` (a real devsrv parser observation).
Reopener unchanged (first r9 devsrv pass), plus: stage mount/unmount
into a future image tier when a venue needs them.

### the r9 /srv open/write path blocks foreign rio driving (two shapes)

From `evidence/19-10-leg1-cont-srvctl.log` (live guest, glenda sessions):

- Writing THROUGH a posted `/srv` file (`echo 'new …' >/srv/riowctl…`)
  → `devsrv: fd string too long` — the write reaches the publish-command
  parser, not the posted service's pipe.
- Opening/mounting rio's posted service (`mount /srv/rio.glenda.51 /mnt`)
  → `mount: namec: does not exist` from mount(1) — **[corrected
  2026-11-16: this was rc failing to exec mount(1) — the binary is not in
  the §15 image; see the corrected item above]**

Net effect: no foreign process (a `-G` side session, the serial shell) can
drive a live rio's wctl — window creation without a real mouse is
impossible on this venue. Also candidate material for the /tmp finding
below. Reopener: the first r9 devsrv pass.

### /tmp unwritable in a cpu session despite the ramfs stanza

`mkdir /tmp/u` in a `-G` session → `namec: does not exist`
(`evidence/19-10-leg1-cont-gunion.log`), while the profile's ramfs stanza
runs in every cpu session (ps showed the parallel session's ramfs process
alive). The image stages `/tmp` (§15's STAGE9 mkdir set) and the profile
falls back to ramfs on the create failure, yet the writable mount did not
take effect in the probed session. One observation, not root-caused
(possibly the same posted-/srv mount class as above — ramfs mounts through
its own postmountsrv). Reopener: the next venue needing session `/tmp`.

## [19-10 Leg-1 continuation 2, 2026-11-16] the dual-cons-reader report vs the live tree: profile exonerated, window-child path open

(SUPERSEDED by the root-cause item above — kept for the record.)

Owner report (after the sweep fix worked): a window swept open OK, but a
prompt painted on the cons layer at the left edge OVER the desktop and only
every other keystroke reached the window (typed chars visible in the window;
output at the out-of-window prompt). Orchestrator diagnosis: the session
`rc -li` left alive beside rio; prescribed fix: the windowed profile case
must end `exec rio`.

Live refutation (KVM guest, the current tree's bake, 2026-11-15/16):

- The staged profile's guarded windowed block ALREADY ends `exec rio`
  (`build/x86_64/root-tree/usr/glenda/lib/profile` line 49; §15's P6 bake
  assert pins `exec rio` in the cpu case) — there was nothing profile-side
  to fix, and any image re-bake would be byte-identical.
- A `-G` side session's `ps` against the LIVE windowed session (evidence
  `19-10-leg1-cont-ps.log`) shows the windowed session's chain as
  `tcp17010 → rio` (rio pid 51 + its libthread procs 70-74) with `plumber`
  (68/69) and `ramfs` (59) — **no `rc -li` survives the exec**. The two
  other `rc` entries are init's serial console shell and the probe's own.
- Login is single-reader end to end: an XTEST-typed marker line + Enter at
  the empty grey desktop produced NO painting on the cons layer
  (`19-10-leg1-cont-desktop-keys-2.png`; with no window open, rio swallows
  keys silently).
- r9's MBEFORE union semantics are head-insert/first-accepting by source
  (`port/src/pgrp.rs` bind(), MBEFORE arm) — rio's winshell
  `/dev/cons`-through-the-union shape is kernel-correct on paper.

Open mechanism (hypothesis, NOT root-caused — the fix-everything bar is
unmet without reproduction): the only process a sweep adds is the window's
own `rc -i`, and the symptom's alternation + left-side prompt/output fits
that child reading and writing the SESSION cons instead of its wsys window
cons. Not reproducible on the automated venue: the synthetic XTEST mouse
still drives nothing (the prior recorded finding), and both programmatic
window-creation avenues into a live rio are blocked by r9-side defects
(below). Reopener: the owner's next windowed session with the README's
`ps` discriminator hot (a live one-line capture names the class), or the
first r9 devsrv/srv-mount pass that unblocks automated rio driving.


## [19-10 Leg-1 continuation] the §14 auth image's sweep hardcodes dom=DEMO-harvey (user-domain breakage)

Found during the 19-10 Leg-1 authid root-cause (2026-11-15): `build/x86_64/auth-sweep.rc:331`
loads factotum's client key as `key proto=p9sk1 user=norbert dom=DEMO-harvey ...` — a
hardcoded DEMO domain token in the §14 AUTH image's bring-up sweep. With a repo-root
`authdomain.local` present (any user-supplied domain), the §14 bake stages that domain's
paired material, the local authsrv advertises the user authdom, and the sweep's
client-side p9sk1 probes fail their key lookup on the dom mismatch — `nix run
.#smoke-r9-auth-x64` would red on such a checkout (it was green when the gate last ran
with no authdomain.local present; the owner's file landed after).

- Same family as the Leg-1 defect (the p9sk1 key/dom pairing assumed the DEMO domain),
  but a DIFFERENT image (§14 auth, not §15 cpu) and a DIFFERENT consumer (the gate
  sweep's client probes, not the cpu server's listener key).
- Out of the Leg-1 fix's scope: Leg-1's path (boot --system=cpu + drawterm) never
  touches the §14 image; fixing the sweep's literal means re-keying the auth gate's
  expectations (the sweep's marker vocabulary) against a per-bake authdom.
- Reopener/owner: the next plan that runs `.#smoke-r9-auth-x64` on a checkout carrying
  an `authdomain.local`, or the next §14-touching plan. Fix shape: parse the authdom
  from the baked /lib/ndb/local (the cpurc leg-5 idiom, group-free sed) instead of the
  literal.

## 19-10 Leg-1 substrate (D-11 continuation, 2026-11-15): two open findings

### 1. drawterm cpu-session mouse delivery — rio runs but synthetic X mouse events drive nothing

- Observed: the windowed session's desktop is UP (rio's 0x777777 grey, deterministic
  across runs; evidence/19-10-leg1-desktop-grey*.png) and rio's fsys serves
  (label/winname probes on the serial), but XTEST mouse moves/clicks (xdotool) never
  open the B3 menu or sweep a window; the whole window stays uniform grey. The X
  keyboard path WORKS (the password feed; the pre-fix sessions' rio error lines
  printed to the window), and the 9P cons path works both ways over /mnt/term.
- Suspect: the drawterm-side /dev/mouse read path under the exportfs (a parked
  mouseread + the xmouse wakeup) — a path NO prior gate exercises (the -G gate has
  no mouse; the 15-06 rio traces rode the KERNEL-side devdraw + mousein replays).
  Not root-caused: host ptrace is yama-restricted in this venue (strace could not
  follow drawterm's threads), and the 2-attempt fix budget was spent on the
  root-caused iounit defect instead.
- Impact on Leg 1: the OWNER with a real mouse may well be unaffected (real X events
  vs XTEST synthetics were not distinguished before the venue ran out of probing
  tools); the automated Xvfb leg could not open a window, so no window screenshot.
- Reopener: the first drawterm-side plan; drive drawterm's exportfs against its own
  devmouse on the host (no guest needed) and pin the parked-read wakeup.

### 2. aborted wrong-password connections poison the guest's auth service

- Observed: three windowed connections fed the wrong password (norbert's, on the
  owner's distinct-password domain) and were abandoned mid-negotiation; after the
  first abort, every later connection died at `cpu: srvauth: namec: does not exist`
  (the server-side p9any proxy missing a file — consistent with the per-connection
  proxy's factotum/rpc state dying on the aborted connection and the /mnt/factotum
  mount going stale for later listeners' children). A fresh boot restored the
  service; the single-connection wrong-password RETRY loop (drawterm's re-prompt)
  is fine — only the connection-abort class poisoned it.
- Not root-caused (the harness artifact forced the fresh-boot route); recorded so
  the robustness defect has a home. Reopener: the next r9 kernel pass (factotum's
  hangup handling under aux/listen's per-connection forks).
