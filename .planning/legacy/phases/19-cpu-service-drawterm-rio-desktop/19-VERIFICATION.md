---
phase: 19-cpu-service-drawterm-rio-desktop
verified: 2026-11-17T14:30:00Z
status: passed
score: 13/13 must-haves verified
covered_files: [".planning/phases/19-cpu-service-drawterm-rio-desktop/19-01-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-01-SUMMARY.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-02-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-02-SUMMARY.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-03-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-03-SUMMARY.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-04-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-04-SUMMARY.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-05-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-05-SUMMARY.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-06-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-06-SUMMARY.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-07-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-07-SUMMARY.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-08-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-08-SUMMARY.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-09-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-09-SUMMARY.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-10-PLAN.md",".planning/phases/19-cpu-service-drawterm-rio-desktop/19-10-SUMMARY.md","authdomain.local.EXAMPLE","build/x86_64/README.md","build/x86_64/authdomain-gen.c","build/x86_64/cpu-bringup.expect","build/x86_64/drawterm-gate.expect","build/x86_64/mkimage.sh","build/x86_64/root-tree/bin/service/tcp17010","build/x86_64/root-tree/rc/bin/cpurc","build/x86_64/root-tree/usr/glenda/lib/profile","build/x86_64/smoke-r9.expect","build/x86_64/tiers/tier2-services.txt","flake.lock","flake.nix","nix/boot-r9-x64.sh","nix/drawterm-gate-x64.sh","nix/drawterm-x64.sh","nix/drawterm.nix","nix/smoke-r9-x64.sh","sys/src/cmd/unix/drawterm/cpu.c","sys/src/cmd/unix/drawterm/flagtest.sh","sys/src/cmd/unix/drawterm/gui-none/Makefile","sys/src/cmd/unix/drawterm/gui-none/screen.c","sys/src/cmd/unix/drawterm/gui-x11/x11.c","sys/src/cmd/unix/drawterm/main.c"]
covered_digest: "v4:sha256:027efc47c8ad41d76efbafa8d4fd1b41356e28f237c200a4c049e5ab104eb3d2"
behavior_unverified: 0
overrides_applied: 1
overrides:
  - must_have: "The graphical leg is human-accepted on linux AND darwin (DESK-07)"
    reason: "Owner-resolved deviation: the linux graphical row is SIGNED (human-accepted 2026-11-16, Neil Skinner, r9 0921a1bc, screenshot evidence/19-10-linux-rio.png — independently verified by the verifier to show a live rio desktop with interactive cpu% windows inside a drawterm X11 window); the darwin -G leg is deferred by owner decision 2026-11-17 (verbatim intent: 'aarch64 cpu port, but just note it so that I can move back to the mac and perform the work natively on the aarch64 machine' — SSH-tunnel venue and mac-local TCG substitute both declined; reopener: the owner-performed aarch64 cpu port) and the darwin graphical leg is deferred 2026-11-14 (gui-cocoa reopener, XQuartz rejected). Recorded deferrals with named reopeners in 19-EVIDENCE.md Row 2, REQUIREMENTS.md DESK-07 (accepted-with-recorded-deferral, the AUTH-08 precedent), and STATE.md Deferred Items — never silent completions."
    accepted_by: "Neil Skinner (owner)"
    accepted_at: "2026-11-17"
---

# Phase 19: CPU Service + Drawterm + rio Desktop Verification Report

**Phase Goal:** The milestone's payoff — `nix run .#boot-r9` remains the ONE command on x86_64 and now brings up the full system (net + auth + cpu listener); `nix run .#drawterm` connects from the host, authenticates, and lands the user in an interactive rio desktop session — on the flush-out-hardened kernel. Work spans both repos plus the flake glue.
**Verified:** 2026-11-17T14:30:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

Truths merged from ROADMAP Success Criteria 1-5 and the ten PLAN frontmatters (deduplicated: PLAN truths restate roadmap SCs at task granularity).

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | r9 execs `#!` scripts with donor argv order; listen's blind execl of a service file becomes the interpreter with -R (CPU-01, SC1) | ✓ VERIFIED | `../r9/x86_64/src/exec.rs`: `shebang_pass` + `exec_with_shebang` (SHEBANG_LINE_BOUND=32, MAX_SHEBANG_DEPTH=2) before parse_aout; 19 pin tests recorded passing; live ring-3 proof at the tcp17010 service point in every gate capture (`tcp17010` is literally `#!/amd64/bin/cpu -R` — kernel #! is load-bearing there) |
| 2 | `service=cpu` arrives via the boot-protocol cmdline (plan9.ini equivalent) so cpurc runs instead of termrc (CPU-02, SC1) | ✓ VERIFIED | `../r9/x86_64/src/platform/q35/mb2.rs` tag-1 parse (256-byte loud cap), `main.rs:541-563` bootconf merge + `devenv: bootconf` print, grub.cfg `@R9-APPEND@` token, `config_q35cpu.toml` `append = "init=/amd64/init -c"`, smoke driver composes the ISO `--append="init=/amd64/init -c"`; `devenv: bootconf` present exactly once in today's current-tree capture `build/x86_64/smoke-r9-cpu.log` (2026-11-17 07:55) and in both clean-checkout consoles |
| 3 | aux/listen + tcp17010 live at boot with exportfs served; r9 implements WSTAT/FWSTAT/REMOVE (CPU-03, SC1) | ✓ VERIFIED | cpurc leg 6 `aux/listen -t /bin/service tcp` (cpurc:284) + `listen1 -t 'tcp!*!567'` authsrv (cpurc:216); `root-tree/bin/service/tcp17010` = `#!/amd64/bin/cpu -R`; `../r9/x86_64/src/syscall.rs:139-143` DISPATCH carries Some(sys_remove/sys_wstat/sys_fwstat) at 25/26/27/44/45; `cpurc: LISTEN-17010` in every capture (bringup a/b, smoke a/b, clean smoke, clean gate) |
| 4 | Full `cpu -R` remoteside flow verified end-to-end with serial evidence: p9any→p9sk1 → auth_chuid → exportfs at /mnt/term → cons stdio → rc -li (CPU-04, SC2) | ✓ VERIFIED | `evidence/19-10-clean-gate.console`: stage-ok sequence `session-prompt (the remote interactive shell; CPU-04's rc -li evidence)` → `fed-echo` → `session-exit` → `drawterm EOF after the hangup` → `serial drained (authsrv's exchange lines are in the capture; CPU-04's serial evidence)` + `ALL STAGES PASS`; the same gate green twice at the pin (`19-08-gate-pin-{a,b}.log`, redaction contract held) |
| 5 | Flake drawterm app exists; wrapper pins -O, scrubs $cpu/$auth/$PASS, prints the exact connect line; provenance recorded (DESK-01, SC3) | ✓ VERIFIED | `nix/drawterm.nix` (in-tree src, zero fetchurl/fetchgit/patchPhase), `nix/drawterm-x64.sh:66` `unset cpu auth PASS USER` + pinned `-O cpu=127.0.0.1,auth=tcp!127.0.0.1!5670,user=glenda` + `-s ''`, cpu.c `case 'O'`/`case 'G'` at 145/139; `nix eval .#apps.x86_64-linux.drawterm.program` exit 0 (re-run live this verification) |
| 6 | The -G text-mode interop gate is green via hostfwd (scripted login → shell marker), encrypted default (DESK-06, DESK-04 live, SC3) | ✓ VERIFIED | `build/x86_64/drawterm-gate.expect` (pty spawn, R9_DEMO_PW contract, redaction LAST) + `nix/drawterm-gate-x64.sh` + flake app; pinned green-twice captures + TCG green (`19-08-gate-{pin-a,pin-b,tcg}.log`, each ends `redaction contract held — <demo-pw-redacted>`); clean-checkout gate green; app eval exit 0 |
| 7 | rio starts only in-session: profile case cpu binds /mnt/term/dev over /dev and execs rio behind the draw guard; boot proves rio did NOT start (DESK-02, SC4) | ✓ VERIFIED | `root-tree/usr/glenda/lib/profile`: `bind /mnt/term/dev/cons /dev/cons` + `bind -a /mnt/term/dev /dev`, `exec rio` behind `test -e /mnt/term/dev/draw` (line 60-62); font-line count 0 in `19-09-smoke-cpu-{a,b}.log` and today's `build/x86_64/smoke-r9-cpu.log` (the 2 occurrences in the clean-smoke DRIVER console are bake-manifest staging lines, not boot output) |
| 8 | plumb runs in the drawterm session (DESK-05, SC4) | ✓ VERIFIED | profile: `plumber` starts in-session behind the same draw guard (line 61, DESK-05 named in the guard comment); the signed human row's session notes record the owner's live ps showing plumber processes serving and /mnt/plumb's port set — human-accepted evidence for a human-session criterion |
| 9 | r9 devssl `#D` (rc4_256/sha1) serves pushssl's protocol for encrypted sessions; -e clear documented fallback (DESK-04, SC4) | ✓ VERIFIED | `../r9/port/src/devssl.rs` (41456 bytes) + `rc4.rs` registered (`lib.rs:70,76`), namec `DEV_SSL` walk/open/stat arms (`namec.rs:34,150,427,462`); the live proof is the gate itself: clean-gate verdict names `the encrypted rc4_256 sha1 session (no -e flag — D-15)`; README documents `-e clear` DEBUG-only |
| 10 | Graphical leg human-accepted; DESK-07 closes (SC5) | ✓ PASSED (override) | Linux row SIGNED 2026-11-16 (Neil Skinner, r9 0921a1bc, commit d4f0eeae5); the screenshot `evidence/19-10-linux-rio.png` (PNG 2616x1690) was independently inspected this verification and shows the rio desktop with two interactive cpu% windows inside a drawterm X11 window. Darwin legs deferred by owner decision (2026-11-14 graphical / 2026-11-17 -G) with named reopeners recorded in 19-EVIDENCE.md Row 2, REQUIREMENTS.md, STATE.md — see overrides frontmatter |
| 11 | boot-r9 remains the ONE command, now bringing up the full system; boot-r9-smoke extended to the full chain (INT-05, SC5) | ✓ VERIFIED | `nix/boot-r9-x64.sh`: `--system=*` single-accepted-form parse (duplicate/unknown = loud errors naming terminal\|cpu), cpu mode selects `out/bootfs-cpu.paq` with the §15 PASS-line grep + append compose + tier build; `nix/smoke-r9-x64.sh` two-leg compose (`--append=` ISO); `smoke-r9.expect` cpu ladder stages incl. `stage cpu-bootconf`; today's current-tree captures (2026-11-17 07:55) show the full 7-marker ladder green, zero FAIL |
| 12 | r9 flake.lock pin advances via push-before-pin as its OWN lock-only commit with fresh hosted CI (INT-06, SC5) | ✓ VERIFIED | All 8 pin-advance commits spot-checked touch flake.lock ONLY (dfb6e5ab1, 4ceb275de, a562a5b9d, ff21c4786, 6d6190676, d109744b2, a190e6bd3, 3f449fd72); flake.lock r9 rev `0921a1bc9337a3e35598248877782ae54172dbad` == `../r9` HEAD == origin/main (clean tree); hosted CI re-verified LIVE this verification: `gh run list` shows run 43990489795 at head 0921a1bc conclusion SUCCESS (plus 43989749900, 43984701640, 43983837655... all success) |
| 13 | Full v2.1 chain reproduces from clean checkouts on linux-x86_64 (INT-07, SC5) | ✓ VERIFIED | `evidence/19-10-clean-smoke.console` (149140 bytes) and `19-10-clean-gate.console` (144396 bytes) both carry `ALL STAGES PASS` from two independent fresh clones at e4001b383, r9 consumed solely via the flake.lock github pin (df4ad78 at proof time); both consoles verified zero DEMO-password occurrences at retention per the recorded post-pass |

**Score:** 13/13 truths verified (0 present, behavior-unverified)

### Required Artifacts

All artifacts verified manually at all applicable levels (exists, substantive, wired; the plans declare artifacts as plain strings, so `verify.artifacts` parsed 0 — manual verification covered every declared path). r9-side artifacts verified against the working tree at the pinned commit 0921a1bc (== flake.lock == origin/main, clean).

| Artifact | Expected | Status | Details |
|----------|----------|--------|--------|
| `build/x86_64/root-tree/rc/bin/cpurc` | six-stage fail-loud ladder + both listener shapes | ✓ VERIFIED | 18 marker sites; PAIR-OK/ETHER-OK/KEYFS-OK/AUTHSRV-LISTEN/FACTOTUM-OK/LISTEN-17010/DONE + FAIL vocabulary; listen1 -t + aux/listen -t |
| `build/x86_64/root-tree/bin/service/tcp17010` | `#!/amd64/bin/cpu -R` first line | ✓ VERIFIED | exactly that line |
| `build/x86_64/root-tree/usr/glenda/lib/profile` | draw guard + plumber + wsys guards | ✓ VERIFIED | final D-11 shape: dual wsys guard, `bind /mnt/term/dev/*`, guarded `plumber; exec rio` |
| `build/x86_64/cpu-bringup.expect` | bring-up probe | ✓ VERIFIED | 16 cpurc stage references |
| `build/x86_64/drawterm-gate.expect` | two-subtree pty gate | ✓ VERIFIED | env contract incl. R9_DEMO_PW/DRAWTERM_BIN, pty discipline, redaction |
| `nix/drawterm.nix` / `nix/drawterm-x64.sh` | hermetic package + scrub wrapper | ✓ VERIFIED | no fetch verbs; unset + pinned -O + `-s ''` |
| `sys/src/cmd/unix/drawterm/gui-none/{screen.c,Makefile}` | loud-stub none backend | ✓ VERIFIED | exists; loud `screeninit called` exit per flagtest record |
| `../r9/x86_64/lib/config_q35cpu.toml` | boot profile with append | ✓ VERIFIED | `append = "init=/amd64/init -c"`, module harvey:bootfs-cpu.paq, dev incl. ether/ip |
| `../r9/port/src/{devssl.rs,rc4.rs}` | #D device + crypto | ✓ VERIFIED | 41456/14489 bytes; registered in lib.rs + namec |
| `build/x86_64/out/bootfs-cpu.paq` (gitignored bake) | §15 image | ✓ VERIFIED | 22003607 bytes baked 2026-11-17 07:55 (this session's regression pass); gitignored |

### Key Link Verification

Key links verified by reading both endpoints and the connecting wire in the actual files.

| From | To | Via | Status | Details |
|------|----|----|--------|---------|
| config append / --append flag | kernel env seed | grub.cfg `@R9-APPEND@` → mb2 tag 1 → main.rs bootconf merge → seed_root_group | ✓ WIRED | all five hops read in source; `devenv: bootconf` observed in live captures |
| cpurc | tcp17010 service | `aux/listen -t /bin/service tcp` → kernel `#!` → `/amd64/bin/cpu -R` | ✓ WIRED | cpurc:284 + service file + exec.rs pre-pass; `tcp call for tcp17010` + LISTEN-17010 in captures |
| profile cpu case | in-session desktop | bind /mnt/term/dev → draw guard → plumber → exec rio | ✓ WIRED | profile lines 53-62; boot captures carry zero font lines |
| flake | drawterm gates | packages.drawterm → DRAWTERM_BIN export → apps drawterm + drawterm-gate-x64 | ✓ WIRED | flake.nix:63-72, 314-316, 448-453; both app programs eval green |
| drawterm-gate driver | creds + pty session | bake §15 → out/auth-creds.txt read → expect pty spawn → redaction LAST | ✓ WIRED | driver grep hits auth-creds/R9_ACCEL/bootfs-cpu; captures end with the redaction-contract line |
| smoke driver | full-chain gate | tier build → §15 assert → append ISO → two-capture expect | ✓ WIRED | driver:175-215; expect cpu stages incl. `stage cpu-bootconf` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|-------------------|--------|
| drawterm-gate session | R9_DEMO_USER/R9_DEMO_PW | out/auth-creds.txt (D-12 emission, authdomain-gen.c:647) | Yes — live per-bake creds feed the pty | ✓ FLOWING |
| smoke cpu capture | cpurc markers | guest serial through qemu → expect log | Yes — full ladder in today's capture | ✓ FLOWING |
| clean-checkout proof | r9 kernel bytes | flake.lock github pin (nix fetches locked rev) | Yes — no sibling clone participated | ✓ FLOWING |
| wrapper connect line | -O spec | committed hostfwd data (127.0.0.1:17010/5670) | Yes — pinned literals, env scrubbed | ✓ FLOWING |

### Behavioral Spot-Checks

Full qemu-booting gates were not re-run end-to-end this verification (each takes minutes and boots a guest); instead the existing runtime evidence was verified directly and cheap live checks were executed.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| drawterm flake app evaluates | `nix eval .#apps.x86_64-linux.drawterm.program` | store path, exit 0 | ✓ PASS |
| drawterm-gate flake app evaluates | `nix eval .#apps.x86_64-linux.drawterm-gate-x64.program` | store path, exit 0 | ✓ PASS |
| AcceptPool donor-shape pin (CR-03 fix) | `cd ../r9 && nix develop -c cargo test -p port --lib -- pool_members_carry` | 1 passed | ✓ PASS |
| Bulk-write park pin (WR-04) | `... cargo test -p port --lib -- bulk_write_round_trips` | 1 passed | ✓ PASS |
| Hosted CI at the final pin (INT-06) | `gh run list` (r9) | run 43990489795 at 0921a1bc = SUCCESS | ✓ PASS |
| Clean-checkout smoke verdict | grep over `evidence/19-10-clean-smoke.console` | ALL STAGES PASS present; full cpurc ladder incl. ETHER-OK | ✓ PASS |
| Clean-checkout gate verdict | grep over `evidence/19-10-clean-gate.console` | ALL STAGES PASS + session-prompt/fed-echo/session-exit/serial-drained stages + redaction | ✓ PASS |
| Current-tree smoke (this session's regression) | grep over `build/x86_64/smoke-r9-cpu.log` (2026-11-17 07:55) | 7/7 ladder markers, FAIL=0, font=0, bootconf=1 | ✓ PASS |
| Signed screenshot shows a rio desktop | image inspection of `evidence/19-10-linux-rio.png` | drawterm X11 window containing rio with two interactive cpu% windows (`ls /bin` output incl. /bin/rio, /bin/plumber) | ✓ PASS |

### Probe Execution

The phase's probes are the qemu-booting composed gates (drawterm-gate-x64, boot-r9-smoke-x64). Not re-booted this verification; runtime evidence verified instead: today's regression-pass captures on disk at the final tree (smoke chain+cpu, 2026-11-17 07:55), the pinned green-twice captures (19-08), the bringup captures (19-07), and the two clean-checkout consoles (19-10). All green. Two named r9 host tests and both flake app evals re-ran live this pass (see spot-checks).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| CPU-01 | 19-03 | r9 `#!` exec | ✓ SATISFIED | Truth 1 |
| CPU-02 | 19-01, 19-07 | service=cpu bootconf path; cpurc runs | ✓ SATISFIED | Truths 2, 3 |
| CPU-03 | 19-03, 19-07 | aux/listen + tcp17010 live at boot; WSTAT/FWSTAT/REMOVE | ✓ SATISFIED | Truth 3 |
| CPU-04 | 19-02, 19-08 | full remoteside flow e2e | ✓ SATISFIED | Truth 4 |
| DESK-01 | 19-06 | flake drawterm app + wrapper | ✓ SATISFIED | Truth 5 |
| DESK-02 | 19-07 | rio in-session only | ✓ SATISFIED | Truth 7 |
| DESK-04 | 19-04, 19-08 | devssl #D encrypted sessions | ✓ SATISFIED | Truth 9 |
| DESK-05 | 19-07 | plumber in-session | ✓ SATISFIED | Truth 8 |
| DESK-06 | 19-08 | -G interop gate | ✓ SATISFIED | Truth 6 |
| DESK-07 | 19-10 | human-accepted graphical legs | ✓ SATISFIED (override) | Truth 10 — linux signed + darwin owner-deferred with reopeners |
| INT-05 | 19-09 | ONE command full system | ✓ SATISFIED | Truth 11 |
| INT-06 | 19-05 | lock-only pin advances + hosted CI | ✓ SATISFIED | Truth 12 |
| INT-07 | 19-10 | clean-checkout proof | ✓ SATISFIED | Truth 13 |

No orphaned requirements: REQUIREMENTS.md maps exactly these 13 IDs to Phase 19 (DESK-03 was staged in Phase 15, per the phase contract); all 13 are declared across the ten plans' frontmatter and all are Complete in the traceability table.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `../r9/aarch64/src/syscall.rs` | fe67c75 | 19-10 D-11 fix touched one aarch64 file (sys_unmount donor arg-shape alignment, applied to both arch dispatch files together) | ⚠️ Warning | Scope observation, not a goal blocker: the plan-scoped prohibitions (19-01/19-02/19-09 "no aarch64 file touched") are honored — their own commits touch no aarch64 path; fe67c75 is an owner-directed D-11 fix whose commit message documents the both-arches touch explicitly, aarch64 builds/tests stayed green in hosted CI at every pin (run 43990489795 SUCCESS), and no aarch64 port work was done. Flagged for the audit trail since the "frozen aarch64" discipline is a standing milestone constraint |
| (various) | mktemp templates | `XXXXXX` strings matched the XXX scan | ℹ️ Info | False positives — mktemp patterns in nix/mkimage scripts, not debt markers |

Debt-marker scan (TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER) over all phase-modified files in both repos: zero unreferenced markers. r9 kernel files: zero `todo!`/`unimplemented!`. Review gate: 19-REVIEW-DISPOSITION.md — 5 findings, 0 open (CR-01/WR-01 fixed at 13091fe41 — commit present in history; WR-02/IN-01 deferred with reopeners in deferred-items.md; IN-02 skipped, donor fidelity).

### Decision Coverage

All trackable CONTEXT.md decisions are honored by shipped artifacts (15/15; gate non-blocking).

### Human Verification Required

None remaining. The phase's two blocking-human checkpoints were resolved during execution: the linux graphical leg was human-accepted and SIGNED (Row 1, verified this pass including the screenshot), and the darwin -G leg was deferred by recorded owner decision with a named reopener (carried as the DESK-07 override). No new behavior-unverified truths exist — every behavior-dependent truth has runtime evidence (gate captures, named tests, live CI status, or the signed human acceptance).

### Gaps Summary

No gaps. The phase goal is achieved and evidenced end-to-end: the ONE command brings up the full system (today's current-tree smoke capture shows the complete cpurc ladder at the final pin), drawterm connects/authenticates/lands an interactive session (clean-checkout gate capture with the full stage sequence), the rio desktop is human-accepted on linux with screenshot and signed row, and the whole chain reproduces from clean checkouts. The r9 pin (0921a1bc) equals origin/main with green hosted CI. The two darwin legs of DESK-07 are owner-deferred with recorded reopeners — accepted deviations, not silent completions. Standing reopeners for future milestones (aarch64 cpu port, gui-cocoa backend, WR-02/IN-01, D-11 residuals) are all recorded in deferred-items.md / STATE.md.

---

_Verified: 2026-11-17T14:30:00Z_
_Verifier: Claude (gsd-verifier)_
