# Phase 19 Evidence — DESK-07 human acceptance + the INT-07 clean-checkout proof

**Phase:** 19-cpu-service-drawterm-rio-desktop
**Consolidated:** 2026-11-15 (plan 19-10)
**Status:** CLOSED 2026-11-17 — the linux graphical leg is SIGNED (Row 1: human-accepted 2026-11-16, r9 `0921a1bc`, screenshot in place, commit d4f0eeae5); the darwin -G leg is DEFERRED BY OWNER DECISION 2026-11-17 with the aarch64-port reopener (Row 2 below — its recipe was never exercised and is recorded as such, not as evidence); the darwin graphical deferral stands unchanged (owner decision 2026-11-14); the INT-07 clean-checkout record is automated and GREEN (both gates). Nothing below is claimed human-accepted that a human did not accept: **an unsigned row is not evidence** (the plan's own prohibition), and a recipe that was never run is not evidence either (venue honesty).

## The pinned substrate

| Repo | SHA | Role |
|------|-----|------|
| r9 (`requaos/r9`, flake input `r9`, type github) | `df4ad78d0177b60b48603e00fd9ba786e2d3c69f` | the flake.lock pin every session and gate below rides; `== origin/main` (pushed, verified 2026-11-15). The clean-checkout proof consumes r9 through this pin only — nix fetches the locked commit itself; no sibling clone participates. |
| harvey | `e4001b38352fb92fc53ff8accf19f529e6ef08b2` | the scaffold commit — the SHA both clean-checkout proof clones rode (the INT-07 record below) |

Pin history: the linux leg's D-11 fix loop advanced the standing r9 pin
df4ad78 → 0921a1bc across six lock-only choreography advances (Row 1 names the
layer-verified SHAs of the loop); `0921a1b` == flake.lock pin == r9 origin/main
today. The INT-07 legs below rode the df4ad78 pin as recorded, and Row 1's
accepted session rode the final 0921a1b pin.

## DESK-07 human-acceptance rows (D-13 — one signed row + one screenshot per leg)

Each row becomes evidence ONLY when signed (date, venue, r9 SHA, accepter) with a
screenshot that resolves at its named path. Until then the row is a scaffold and
says so on its face.

### Row 1 — the linux graphical leg (the rio desktop in a drawterm window)

| Field | Value |
|-------|-------|
| Status | **ACCEPTED — linux graphical leg, human-accepted 2026-11-16** |
| Venue | linux-x86_64, KVM (qemu q35; slirp + the committed D-04 hostfwd pair `127.0.0.1:17010`/`127.0.0.1:5670`) |
| r9 SHA | `0921a1bc9337a3e35598248877782ae54172dbad` (the accepted session's pin; the leg's D-11 fix loop spanned df4ad78 → dae4397 → 56109c3 → 3e5ed0c → 0921a1b, each layer live-verified by the owner before the next) |
| Date | 2026-11-16 |
| Accepter | Neil Skinner (the owner; sign-off recorded at the owner's direction following their acceptance "linux leg accepted", 2026-11-16) |
| Screenshot | `evidence/19-10-linux-rio.png` — present (PNG 2616×1690) |
| Session notes | Desktop ran IN-SESSION in the drawterm window after login (not at boot; the boot-side absence is the 19-07/19-08/19-09 gates' proof). Owner-verified behaviors: keystrokes land in-window; cursor contextual shapes on window edges/corners with correct revert to the arrow; right-button menu reopens after sweep; second window sweeps; `exit` deletes the window while other windows live. Plumbing observation (DESK-05): plumber present and serving in-session — the owner's own live `ps` capture (this phase's evidence) shows the plumber processes; `/mnt/plumb` serves its full port set. |

The recipe (README §"The linux session recipe" + §"The full-system boot", executed
verbatim): boot `nix run .#boot-r9-x64 -- --system=cpu` in one terminal and leave
it running (wait for `cpurc: LISTEN-17010`); then `nix run .#drawterm` in another
terminal with a display — authenticate with the developer's local-file domain
credentials (the 18-D-01 shape, NOT the DEMO material) — accept the rio desktop
when it starts in the drawterm window; exercise it minimally (open a window, run
a command, one plumbing observation); take ONE screenshot; sign this row.

### Row 2 — the darwin -G text leg — DEFERRED BY OWNER DECISION 2026-11-17 (historical scaffold: the interactive cpu shell over the SSH tunnel)

| Field | Value |
|-------|-------|
| Status | **DEFERRED BY OWNER DECISION 2026-11-17** — the row will not be signed this milestone; the decision record and reopener are below. The scaffold context is kept for the reopener. |
| Venue | *(historical scaffold — never exercised)* darwin-aarch64, reaching the linux-hosted guest over the SSH tunnel (D-05) — `ssh -L 17010:127.0.0.1:17010 -L 5670:127.0.0.1:5670 user@linuxhost` with the linux guest left running per D-07 |
| r9 SHA | *(historical scaffold)* `df4ad78d0177b60b48603e00fd9ba786e2d3c69f` (the flake.lock pin at scaffold time — the same commit the darwin build clones) |
| Date | — (no session ran; deferred 2026-11-17) |
| Accepter | — (the owner's decision below is a deferral, not an acceptance) |
| Screenshot | none — the recipe was never exercised on darwin; per the file's own rule, a recipe that was never run is not evidence |
| Session notes | none — the row stays unsigned by design; when the reopener lands, the session runs FOR REAL and the row is signed per D-13 |

**The owner decision (2026-11-17), verbatim intent:** "aarch64 cpu port, but
just note it so that I can move back to the mac and perform the work natively
on the aarch64 machine".

**What the decision resolves:** the owner declined the recorded SSH-tunnel
venue (drawterm-on-mac tunneling to a linux-hosted guest) and declined a
mac-local x86_64 TCG substitute. The chosen path is to port the cpu stack to
aarch64 natively and perform the darwin leg against that port. That port is
phase-scale work — the DTB bootconf seam, the auth-suite tiers, the §15
mkimage contract, devssl `#D` registration, and the drawterm gate are all
currently x86_64-only — and is explicitly the OWNER's own future work on the
mac (aarch64 stays frozen at the v2.0 pin until then; the port is not v2.1
scope).

**Reopener:** the aarch64 cpu port — owner-performed on the darwin/aarch64
machine; the DESK-07 darwin -G row reopens against it.

**DESK-07's standing evidence for this milestone:** the signed linux graphical
row (Row 1 above — human-accepted 2026-11-16, r9 `0921a1bc`, screenshot in
place) plus the INT-07 clean-checkout record below (automated, GREEN both
gates) carry DESK-07; the two darwin legs (graphical deferred 2026-11-14;
-G deferred 2026-11-17) are recorded deferrals with named reopeners — never
silent completions. The deferral is recorded in REQUIREMENTS.md (the DESK-07
annotation) and STATE.md (the Decisions log + the Deferred Items row).

The historical scaffold recipe (kept verbatim for the reopener — it was never
executed; README §"The darwin page — DOCUMENTED-MANUAL, `-G` only", the
amended-D-03 gui-none build — no Carbon, no XQuartz):

```
git clone <harvey> && cd harvey/sys/src/cmd/unix/drawterm
make CONF=osx GUI=none
ssh -L 17010:127.0.0.1:17010 -L 5670:127.0.0.1:5670 user@linuxhost
./drawterm -G -s '' -O 'cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda'
```

## The darwin graphical deferral (owner decision — recorded, not a this-phase leg)

**Deferred 2026-11-14 by owner decision** (the amended D-03 / the OQ-1
settlement): this milestone ships no darwin window backend. The darwin `-G` text
leg above IS this phase's darwin row (Row 2). **Reopener:** a future milestone
phase porting a 64-bit Cocoa/AppKit backend (the 9front `gui-cocoa` class) into
`sys/src/cmd/unix/drawterm`. **XQuartz was considered and rejected** (an
X11-on-mac dependency for a Plan 9 client is a compatibility layer too far; the
native window path belongs to the reopener). The deferral is recorded in
REQUIREMENTS.md (the DESK-07 annotation) and STATE.md (the Deferred Items row).
*[Amended 2026-11-17: the darwin -G row (Row 2) is now itself deferred by owner
decision with the aarch64-port reopener — see Row 2's deferral block above.
DESK-07 closes this milestone as accepted-with-recorded-deferral: the standing
evidence is the signed Row 1 + the INT-07 record; both darwin legs carry named
reopeners, never silent completions.]*

## INT-07 — the clean-checkout proof of the v2.1 chain (linux-x86_64)

**GREEN — both gates, from fresh clones, on 2026-11-15.** The v2.1 chain
reproduces from nothing but a git clone of harvey at HEAD: no sibling r9
checkout, no untracked working-tree state, no manual prerequisites (cpu mode
runs §15's own named producers — the ONE command holds off a clean checkout).

| Leg | Command (from the fresh clone) | Clone SHA | Verdict |
|-----|-------------------------------|-----------|---------|
| full-chain smoke | `nix run .#boot-r9-smoke-x64` | `e4001b38352fb92fc53ff8accf19f529e6ef08b2` | **exit 0 — CLEAN-SMOKE-GREEN** — ALL STAGES PASS, both legs: the chain leg (boot, typed+answered echo, empty-line safety, ls/ps/cat sweep, DEL safety, restart, pid-warning-absent, nic-bind) + the cpu-chain leg (§15 boot with the bootconf append, the hostfwd pair observed host-side 17010+5670, the cpurc ladder in order PAIR-OK → ETHER-OK → KEYFS-OK → AUTHSRV-LISTEN → FACTOTUM-OK → LISTEN-17010 → DONE, zero `cpurc: FAIL`, zero panic, zero desktop font-line markers) |
| drawterm -G interop | `nix run .#drawterm-gate-x64` | `e4001b38352fb92fc53ff8accf19f529e6ef08b2` (a second, independent fresh clone) | **exit 0 — CLEAN-GATE-GREEN** — ALL STAGES PASS: the §15 cpu boot through the ladder, drawterm `-G` on a pty through the committed hostfwd pair, p9any→p9sk1, the encrypted rc4_256 sha1 session (no `-e` flag — D-15), the remote shell prompt, the fed command's echo, the teardown (exportfs hangup + pty EOF), tr-ok on the serial drain, zero FAIL, zero kernel panic, the capture redacted |

**The r9 locked rev:** `df4ad78d0177b60b48603e00fd9ba786e2d3c69f` — consumed
solely through the clone's `flake.lock` github pin (the r9 input is type
github; nix fetched the locked commit itself). No sibling r9 clone
participated in either leg; nothing outside the clone's committed files was
read (T-19-SC: the clean-checkout nix runs evaluate committed files only).

**Retained artifacts** (driver consoles; both verified to carry **zero**
occurrences of their bakes' DEMO passwords before retention — T-19-24/T-18-13):

| File | What it is |
|------|------------|
| `evidence/19-10-clean-smoke.console` | the smoke leg's full driver console (149140 bytes): tier-build + bake lines, both boots' stage-ok sequence, ALL STAGES PASS |
| `evidence/19-10-clean-gate.console` | the -G gate leg's full driver console (144396 bytes): bake, session stages, the redaction post-pass, ALL STAGES PASS |

Provenance note: the proof clones rode HEAD `e4001b383` (the scaffold commit —
the record you are reading lands in the commit after it, which is the normal
ordering for a run-and-record proof). The DEMO domain bakes green on clean
checkouts by design (18-D-01) — per-bake ephemeral material, correctly labeled,
is what both legs exercised.

---
*Phase: 19-cpu-service-drawterm-rio-desktop*
*Scaffold: 2026-11-15 (plan 19-10)*
*Closed: 2026-11-17 — Row 1 signed 2026-11-16 (linux graphical leg human-accepted); Row 2 deferred by owner decision 2026-11-17 (reopener: the aarch64 cpu port, owner-performed)*
