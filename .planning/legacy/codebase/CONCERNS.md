---
last_mapped_commit: bdcf45a51dced761eec09538b5f0786ff81e0399
last_mapped_at: 2026-11-13
---
# Codebase Concerns

**Analysis Date:** 2026-11-13

> Scope note: Harvey at `/home/req/code/harvey` is paired with the adopted kernel r9 at `/home/req/code/r9` (sibling repo, pinned in `flake.lock` at `8f3ff11`). Most kernel-class concerns live in r9 but are owned/tracked by Harvey's planning ledgers; r9 paths are cited repo-relative (`../r9/...`) per the ledger convention. The authoritative concern ledgers are `.planning/WINDOWS.md` (68 rows: ~50 open, 13 fixed, 3-5 waived per the varying counts below), `.planning/phases/*/deferred-items.md`, and `.planning/phases/17-r9-kernel-flush-out/17-REVIEW-DISPOSITION.md` (8 open review findings). This document indexes them; the ledgers stay the truth.

## Tech Debt

**Regenerated yacc output with host-absolute paths in the working tree (uncommitted):**
- Issue: `sys/src/cmd/eqn/eqn.c` (tracked, generated from `eqn.y`) is locally modified (143+/111-): `#line` directives rewritten from `/sys/src/cmd/eqn/eqn.y` to `/home/req/code/harvey/sys/src/cmd/eqn/eqn.y` and tables changed `short yyact[]` → `static const short yyact[]`. A build artifact with machine-specific paths baked in.
- Files: `sys/src/cmd/eqn/eqn.c`
- Impact: spurious drift for every other checkout; trips the tier witness manifest digest if baked; re-regenerates on the next `yacc` run. Filed as review finding IN-05 (outside declared scope of its phase, never dispositioned).
- Fix approach: revert the working-tree change (or regenerate with canonical `/sys/src/...` paths from inside the namespace build); keep generated parser output out of source commits. See `.planning/phases/17-r9-kernel-flush-out/17-REVIEW.md` IN-05.

**Legacy vs active kernel tree divergence:**
- Issue: `sys/src/9/` (legacy multi-arch Plan 9 kernel: 479 `.c` files across bcm/mtx/omap/pc/ppc/teg2/loongson/...) coexists with `sys/src/9k/` (active Harvey C kernel: 153 files) and both now sit beside the adopted r9 Rust kernel. Duplicated subsystems drift: `sys/src/9/ip/tcp.c` and `sys/src/9k/ip/tcp.c` are both 3486 lines and must be maintained in parallel for any shared-lib-facing change.
- Files: `sys/src/9/`, `sys/src/9k/`
- Impact: triple-maintenance surface for kernel-adjacent fixes; new work must target 9k (standalone track) or r9 (the adoption track) — `sys/src/9/` is dead weight but still compiled by legacy mkfiles.
- Fix approach: deduplication is ruled Out of Scope in `.planning/PROJECT.md` ("rendered moot by r9 adoption") — do not invest in `sys/src/9/`; treat it as frozen reference.

**aarch64 frozen at the v2.0 pin:**
- Issue: all v2.1 work keys to x86_64; the aarch64 r9 crate and Harvey aarch64 bits are frozen. Known aarch64-era defects (WINDOWS #50 devpipe DataAbort, 07-WR-08 DTB/ICFGR trigger flags) are waived frozen with reopeners.
- Files: `../r9/aarch64/`, `build/aarch64/`
- Impact: any "in passing" aarch64 fix violates the freeze policy and has no gate to prove it.
- Fix approach: none this milestone; reopeners recorded in `.planning/phases/17-r9-kernel-flush-out/17-CR07-ROOTCAUSE.md` §6.

**mkpaqfs silent drop past blocksize^2/4 — unprotected bakes:**
- Issue: `paqFile()`/`paqDir()` warn ("file too big for blocksize") and drop the file, then mkpaqfs exits 0. At the 4096 default blocksize the cap is 4 MiB. Only the 15-07 gs bake asserts the fit (`-b 16384` + fail-on-diagnostics); the marker/chain/tier/net bakes in `build/x86_64/mkimage.sh` and `build/mm/build-9kdraw.sh`'s bootfs-draw.paq ride the default unprotected.
- Files: `sys/src/cmd/paqfs/mkpaqfs.c` (lines 174, 247, 269), `build/x86_64/mkimage.sh`, `build/mm/build-9kdraw.sh`
- Impact: a payload member growing past 4 MiB vanishes from the image with only a stderr line; current members all fit (fonts max 46089 bytes) but this is a silent-failure trap.
- Fix approach: make the drop fatal in `sys/src/cmd/paqfs/mkpaqfs.c`, or add size-fit asserts to the shared bake helpers. Owner: whichever plan next touches the payload machinery (WINDOWS #64; phase-15 deferred items).

**Expect gates truncate kernel panics mid-line (fast-fail):**
- Issue: the `expect_before -re {panic|Unhandled interrupt}` fast-fail arm (e.g. `build/x86_64/services-r9.expect:134` and siblings) kills qemu the instant "panic" appears, before the kernel's message and location finish printing — retained captures never contain the kernel's own diagnosis text.
- Files: `build/x86_64/services-r9.expect`, `build/x86_64/smoke-r9.expect`, `build/x86_64/tier1-r9.expect`, `build/x86_64/churn-r9.expect`, `build/x86_64/stress-r9.expect`, `build/x86_64/netsoak-r9.expect`
- Impact: post-mortem debugging loses the kernel's text (had to be read out of r9 source in one recorded case).
- Fix approach: let the line drain (a short post-match expect before cleanup). Owner: whichever plan next touches the expect drivers (phase-15 deferred items).

**r9 repo-wide `cargo fmt` drift (~439 pre-existing diff sites) with no fmt gate:**
- Issue: r9's CI (`xtask.yml`) runs check+clippy only; formatting drifted across aarch64/port/x86_64/xtask. Plans keep only their own added lines fmt-clean.
- Files: `../r9/` (repo-wide)
- Impact: cosmetic; a repo-wide `cargo fmt` pass would be a standalone commit no current plan owns.
- Fix approach: one standalone cosmetic commit + add fmt to r9 CI (17-04 deferred items).

**Host build artifacts polluting the source tree:**
- Issue: untracked host-built ELF binaries `sys/src/cmd/aux/data2s` and `sys/src/cmd/aux/mklatinkbd` (x86-64 glibc/nix-linked, with debug info) sit in the source directory alongside their sources; ~32 MB of gate logs (some untracked: `build/x86_64/churn-r9.log`, `services-r9-x64.log`, `stress-r9.log`) accumulate in `build/x86_64/`.
- Files: `sys/src/cmd/aux/data2s`, `sys/src/cmd/aux/mklatinkbd`, `build/x86_64/*.log`
- Impact: `mk clean`/`nuke` hygiene risk; nix flake builds see only git-tracked files so these are invisible there, but any host-side find/glob over `sys/src/cmd/aux` may pick up a host binary named like a Plan 9 command.
- Fix approach: remove the stray binaries (they are reproducible from `data2s.c`/`mklatinkbd.c`), keep logs on the evidence side or gitignore the run-log names.

**Stale tier-1 ledger rows contradicting the live gate:**
- Issue: `build/x86_64/tier1.ledger` still carries two 14-08-era `exec=SKIP:r9x64-…` rows (ipconfig no-devrandom, factotum no-auth-era) although 15-03 flipped both subjects live; the gate has expected 38 T1-OK / zero SKIP since.
- Files: `build/x86_64/tier1.ledger`
- Impact: the ledger is a stale RECORD, not a gate claim, but it contradicts the live gate and misleads audits.
- Fix approach: add the flip rows / annotate — owns the ledger's format, not a quiet edit (phase-15 deferred items).

**rc diagnosability wart (fixed) + remaining rc/lexer dialect hazards:**
- Issue: rc's Execute errstr-masking (#65), fd-dup unchecked returns (#46), and argv-stack underflow (#49) were fixed in Harvey commit `6779df7d7`, but the in-guest rc dialect remains a measurement hazard: braced if-not bodies are parse errors that kill a sourced file mid-stream; bare `=` in match patterns is a lexer token error (a word ends at every bare `=` — single-quote `=`-bearing argv); fd-dup redirects are unreliable in nested sourced blocks; libndb drops an unterminated final line whole.
- Files: `sys/src/cmd/rc/exec.c`, `sys/src/cmd/rc/plan9.c`, `sys/src/cmd/rc/havefork.c`
- Impact: guest scripts must use flat status ladders, `=`-free patterns, absolute-path invocations; violations fail in ways that look like kernel bugs.
- Fix approach: documented as the reliable forms (STATE.md Phase-15 decisions); fixing the rc lexer itself is unowned.

## Known Bugs

**CR-07 / WINDOWS #66 — intermittent 8-way-transfer triple-fault class (WAIVED, LIVE):**
- Symptoms: silent CPU reset with NO kernel trap dump (triple-fault class) ~25 s into the burst-launch window of the 8-way concurrent bulk transfer; empty-status exit cascade then SeaBIOS + GRUB reset chain on serial.
- Files: r9 devip/multi-waiter terrain; instrument `build/x86_64/stress-r9.expect` + `../r9/xtask` stress app (`boot-r9-stress-x64`); record `.planning/phases/17-r9-kernel-flush-out/17-CR07-ROOTCAUSE.md`
- Trigger: full 8-way concurrent tcpblast bulk transfer racing ~16 fork+exec+exit cycles under RX/TX IRQ storm (launch storm alone and ≤4-way never fire). Rate ≈8%/run (2/23 reduced shape).
- Workaround: none; maintainer-ratified waiver (Neil Skinner 2026-11-13) of the CURE this era — KVM capture ceiling (empty `-d int` at death, post-reset-only `cpu_reset` dump, userspace-APIC refused) + TCG×slirp N=10 NO-FIRE-IN-SERIES exhausted; the strongest-candidate fix (NET_WAITERS deregister, r9 `d4a5d38`) was REFUTED as the cure by the N=25 stress series firing at run 3 at the fix pin. Reopener: a context-reaching venue or non-healing instrumentation. The reproduction instrument is committed and fires the class at will in ~25 s.

**WINDOWS #68 — services-x64 dns-stage kernel trap (pre-existing, root cause open):**
- Symptoms: services gate faults at the dns stage — kernel NULL-call (PF at pc=0x0, cs=0x8) immediately after `devmnt: server error: cs: file does not exist`; driver times out at `dns-verdict`, exit 1.
- Files: evidence `build/x86_64/services-r9-x64.log` (line 132); suspect orbit in the 17-01..17-03 r9 commit window (exit_tail/fd-group drain fixes c4f2f2c/72edbba, or the q35churnshell re-point's per-exit markers printing during the services sweep)
- Trigger: running the services-x64 gate; A/B-proven identical at pins b02408cad and 02218e88c (kernel byte-identical), last green services run was the 15-08 era (pin eba6b07e2).
- Workaround: services gate is skipped/blocked; smoke/tier1/net-smoke/churn gates remain green. Root cause belongs to the next plan touching the services venue (Phase 18 auth domain is the likely owner).

**WINDOWS #67 — rc userland malloc pool corruption under backquote churn:**
- Symptoms: "bad arena size / pool sbrkmem block <addr> / hdr ... deadbeef" then "panic: pool panic" after ~428+ iterations of `while(! ~ ...)` + a dc backquote counter per iteration.
- Files: repro `build/x86_64/root-tree/lib/churn-cats.rc`; rc/libc malloc terrain (`sys/src/cmd/rc/`, `sys/src/libc/port/pool.c` class)
- Trigger: sustained per-iteration backquote allocation (frees-then-use suspected); deterministic threshold, kernel side held flat through corrupting runs.
- Workaround: the composed churn leg drives a C churn driver (`churncats.c`) so the gate's subject stays the kernel's exec path. Open — was deferred to 17-03's rc rows; those rows (#46/#49/#65) landed fixes for DIFFERENT classes, so this corruption remains unfixed.

**r9 sys_sleep units slip (ms stored, ticks compared):**
- Symptoms: `sleep(N)` waits ~10x longer than asked — `deadline = ticks() + ms` mixes milliseconds into a 100 Hz tick base.
- Files: `../r9/x86_64/src/syscall.rs` (sys_sleep)
- Trigger: any direct sys_sleep caller.
- Workaround: benign today (its only users wake early via the alarm note); churn driver uses tick units directly. Open (17-02 deferred items).

**r9 exit tail is not 9P-safe (bounded fid leak):**
- Symptoms: a devmnt clunk dispatched from mid-exit measured userland pool corruption in the mounted venue; the sweep now drops chans without the close dispatch, leaving the husk's server-side fid to leak bounded until the server dies.
- Files: `../r9/x86_64/src/proc.rs` (sweep_husk_payloads doc comment)
- Trigger: exiting a process holding mounted chans.
- Workaround: dropped clunks; real fix (exit-path 9P safety or a deferred-clunk queue) is follow-up work (17-02 deferred items).

**Phase-17 review WR-01 — sys_brk_ regrowth duplicate mapping records / stale semaphore PA key:**
- Symptoms: shrink-then-regrow of bss leaves stale `mapping_log` records; first-match consumers read the stale record — `sem_word_pa` can return an already-freed physical page, and after allocator recycling two unrelated procs' semaphore words can key the same PA in `SEM_WAITERS` and cross-wake.
- Files: `../r9/x86_64/src/syscall.rs:814-827` (with `../r9/x86_64/src/proc.rs:1375-1392`; contrast the guarded RFMEM sweep at `proc.rs:2603`)
- Trigger: heap shrink followed by regrowth with a semaphore placed in the regrown page (rc's malloc does not shrink — hence Warning, not Critical).
- Workaround: none; fix shape (mirror the RFMEM liveness check in the private growth arm) recorded in `.planning/phases/17-r9-kernel-flush-out/17-REVIEW.md` WR-01. Open.

**Phase-17 review WR-02 — conv_close pending-incall sweep resets in-use convs:**
- Symptoms: the announcer's last close runs `close_reset()` on queued incall convs with no `inuse` check — a directly-opened queued conv (`/net/udp/N/ctl` walk) is zeroed under live fds; the victim's later close no-ops.
- Files: `../r9/port/src/devip/conv.rs:606-611` (donor semantics: 9k `closeconv` decrements)
- Trigger: direct open of a still-queued spawned conv, then announcer close.
- Workaround: none; donor-shape fix (decrement, reset at zero) recorded in 17-REVIEW WR-02. Open.

**Phase-17 review WR-03 — accept-pool pop leaks sockets on two error arms (v6-reachable):**
- Symptoms: `tcp_listen_pop` drops the popped `SocketHandle` without `abort()`/`sockets.remove()` on the "no bound medium" and v6-quad arms — a live socket lingers in the seam's `SocketSet` until the core unbinds. The v6 arm is reachable by network input: `spawn_listener` listens address-less (both families), so a v6 SYN to the announced port occupies a member that fails the v4-only pop.
- Files: `../r9/port/src/devip/stack.rs:754-763` (consumer `../r9/port/src/devip.rs:860-871`)
- Trigger: v6 SYN to an announced v4 port (or unbound medium at pop time); each event leaks one socket.
- Workaround: none; fix recorded in 17-REVIEW WR-03. Open.

**Fall-off-main exits with failure status:**
- Symptoms: Harvey's amd64 runtime epilogue calls `exits("main")` for programs that fall off main (stock Plan 9 exits nil) — a NON-EMPTY failure status to rc's `&&`/`||`; `cmd && echo` never fires.
- Files: `sys/src/libc/amd64/main9.s`
- Trigger: any program without an explicit `exits(0)` (measured: arctest).
- Workaround: gates key on completion output, not status (WINDOWS #63). Runtime-level decision unowned.

**libmemdraw residual rounding divergence (open tail of the 15-09 fix):**
- Symptoms: the rgba word-lane fast path (MUL0123+MUL0123) still double-rounds; MUL2's shift form differs from round-to-nearest by 1 LSB on 253 values of s in (65025, 130050] — reachable only with alpha-bearing (non-convex) source pairs, so default r8g8b8 drawtest passes.
- Files: `sys/src/libmemdraw/draw.c` (MUL2 comment ~line 943 area), `sys/src/cmd/drawtest/drawtest.c`
- Trigger: byte-path draws with alpha or mixed channels (e.g. `r8g8b8a8 r8g8b8`).
- Workaround: venue rig stays in the convex regime; fix shape (clamped two-term identity to 130050) recorded in phase-15 deferred items (WINDOWS #61). Open.

**samterm input-state fragility:**
- Symptoms: menu-New leaves `which` unset pending Hcurrent; a `B` command reply leaving a non-empty dot makes `type()` take the cut-selection branch and stall behind the kbdblock mask.
- Files: `sys/src/cmd/samterm/io.c`, `sys/src/cmd/samterm/main.c`, `sys/src/cmd/samterm/menu.c`
- Trigger: GUI replay driving menu-New or post-B typed lines.
- Workaround: the committed replay drives the proven path (B on missing name + B1 dot-reset). A real fix owns samterm's state machine (WINDOWS #62). Open.

**devip RX-ring console truncation (aarch64-era, shape carries):**
- Symptoms: a console input burst longer than the 256-byte RX ring truncates mid-line with no reader-visible signal; the cooked line never completes and the guest wedges (measured 2/2 at 316 chars → 255).
- Files: r9 devcons/PL011 ring (aarch64 era; `../r9/port/src/devcons.rs`)
- Trigger: long/pasted typed lines.
- Workaround: gate-author discipline (short typed lines / paced continuations) encoded in `build/x86_64/smoke-r9-net.expect`'s header (WINDOWS #48). A kernel-side "line truncated" signal is the possible refinement. Open.

## Security Considerations

**GPL-licensed ghostscript components inside an MIT tree:**
- Risk: Harvey is MIT since 2021, but GPL enters via ghostscript (`sys/src/cmd/gs/`, LICENSE.gpl) and AFPL/Lucida terms also exist (`LICENSE.afpl`, `LICENSE.lucida`); distribution mixes must respect this.
- Files: `/home/req/code/harvey/LICENSE*`, `sys/src/cmd/gs/`
- Current mitigation: documented in PROJECT.md ("GPL only via ghostscript fonts"); the 15-07 gs bake stages zero font payloads and measured-minimal config tables.
- Recommendations: none new; keep the license files at root and keep font payloads out of bakes.

**Crypto era — TLS modernization explicitly out of scope:**
- Risk: libsec carries RC4/DES/3DES-era ciphers and a TLS 1.2 cap; libauthsrv protocols (p9sk1, wep, wpapsk) are of their era.
- Files: `sys/src/libsec/port/`, `sys/src/libauthsrv/`
- Current mitigation: declared Out of Scope in `.planning/PROJECT.md` ("TLS/security modernization — separate hardening milestone").
- Recommendations: do not expose services beyond loopback/hostfwd before that milestone; Phase 18's auth domain is loopback-first by design (probe green before any drawterm).

**No in-guest auth domain yet (the Phase 18 subject):**
- Risk: keyfs/authsrv runtime surfaces are tier-baked but the p9sk1 domain, `/dev/random` from x86_64 entropy, and devcap `#¤` are not yet wired; devcons on x64 serves a deterministic-seed /dev/random era (the port devcons serves it, no-hooks era).
- Files: `sys/src/cmd/auth/`, r9 devcons (`../r9/port/src/devcons.rs`)
- Current mitigation: Phase 18 is planned and ready to execute (`.planning/phases/18-in-guest-p9sk1-auth-domain/18-CONTEXT.md`).
- Recommendations: treat any pre-18 random-material consumer as non-secret.

**Kernel NULL-call PF class (#68) as an attack-surface signal:**
- Risk: the dns-stage trap (PF pc=0x0 after a devmnt cs error) shows an error path that can vector through a NULL function pointer — robustness, and potentially safety, relevance once network input is adversarial.
- Files: r9 devmnt error path (root cause open; see WINDOWS #68)
- Current mitigation: A/B-proven pre-existing; gate blocks the services venue only.
- Recommendations: root-cause before Phase 19 exposes `cpu -R` beyond loopback.

## Performance Bottlenecks

**IRQ-context wake work over a 512-slot table (CR-07 terrain):**
- Problem: `pipe_wake` walks the 512-slot PROC_TABLE in IRQ context; the (now-fixed) stale-registration herd made every RX-IRQ/tick wake reach non-devip contexts and scan `pids.contains` per entry precisely in the launch window where the triple-fault class fires.
- Files: `../r9/x86_64/src/proc.rs:2126` (pipe_wake), `../r9/port/src/devip.rs` (NET_WAITERS)
- Cause: wake-path design; exonerated as the corruption MECHANISM but remains a latency burden (17-CR07-ROOTCAUSE §3-4).
- Improvement path: the deregister counterpart landed (r9 `d4a5d38`) keeps NET_WAITERS exact; further narrowing belongs to a plan owning the wake path.

**Eager text mapping — no demand pager this era:**
- Problem: x86_64 exec maps TEXT eagerly (and data+bss); demand text, deeper stack/tos growth beyond seed pages arrive with the pager.
- Files: `../r9/x86_64/src/exec.rs` (WINDOWS #53)
- Cause: recorded era shape (Phase 14).
- Improvement path: the trap-context pager is a future-milestone item; NSEC/epoch (WINDOWS #54) rides the same seam.

**Fixed 32 MiB kernel heap with per-exit residue:**
- Problem: the heap was raised 4→32 MiB after the services venue exhausted it mid-sweep; a ~60 bytes/exec post-fix residue remains (was ~16 KiB/exec); a ~16 KiB per-exit fd-group Arc drain was fixed at r9 c4f2f2c/72edbba.
- Files: r9 kernel allocator (`../r9/x86_64/src/`), measured by the q35churn heap-marker gates
- Cause: exec staging + resident daemons + residue; bounded-memory gates now assert the window (churn profile, composed churn).
- Improvement path: residue LEAK audit stays a backlog item (the 15-03 RFMEM-OOM note); oom handler prints failed layout + arena high-water.

**console print paths under masked-post discipline:**
- Problem: post-sti boot-context code must mask around driver ops AND console prints (console MCS lock vs timer tick IRQ print) — unmasked prints in early contexts wedge.
- Files: r9 netgate/console (`../r9/x86_64/src/`)
- Cause: single-console MCS lock; discipline documented (STATE Phase-13 decisions).
- Improvement path: any new early-boot print site must follow the masked-post shape.

## Fragile Areas

**Tier-1 witness tree-history sensitivity (yacc dirs):**
- Files: `sys/src/cmd/rc/syn.y`, `sys/src/cmd/grep/grep.y`, `sys/src/cmd/awk/awkgram.y` consumers via `build/mm/build-9k10t1.sh` (the 38-member manifest)
- Why fragile: one tier-driver run produced different bytes for exactly cmd/{rc,grep,awk} from identical sources (then converged; a yacc root-single like cmd/units never drifted). Cause undetermined (suspect: stale y.tab.* state from a prior build route, or archive/regeneration order). Any /amd64 writer between a donor build and the next mkimage reds the witness.
- Safe modification: after any /amd64/bin byte change, re-pin via `build/mm/build-9k10t1.sh` in the working ORDER tier-driver-THEN-pin (re-pinning from one-off bytes re-reds the next run).
- Test coverage: the witness manifest digest check in mkimage (9a) is the tripwire.

**mkimage witness-manifest digest trips on any /amd64/bin byte change:**
- Files: `build/x86_64/mkimage.sh`, `build/mm/build-9k10t1.sh`
- Why fragile: the standing remedy ritual (donor rebuild + 38-member manifest re-pin, gitignored artifacts, no commit, no pin advance) must be re-run after e.g. an rc fix (done for `6779df7d7`); forgetting it reds every tier-consuming gate.
- Safe modification: always re-run the tier driver to convergence FIRST, then re-pin.
- Test coverage: gates themselves; the ritual is documented in 17-06/17-FLUSHOUT.

**boot-iso FOD re-stale standing rule:**
- Files: `../r9/flake.nix` (packages.boot-iso), wired through `flake.lock`
- Why fragile: ANY r9 kernel-byte commit re-stales the FOD outputHash. The close discipline is re-wire-then-pin: two-derivation fresh-kernel byte-identity proof, wire the hash, `nix build .#boot-iso` (+ `--rebuild`) green, push r9, THEN advance the Harvey pin (lock-only). Never record the FOD green without running it at the pinned rev (this bit Phase 14 — WINDOWS #56/#60).
- Safe modification: follow the pin-advance lineage table in `.planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md`.
- Test coverage: `nix build .#boot-iso --rebuild` on every close.

**WINDOWS.md ledger internal inconsistency (residual of the historical rows-44/45 corruption):**
- Files: `.planning/WINDOWS.md`
- Why fragile: the ledger was historically corrupted (rows 44/45 present in the table but missing from the fenced JSON block — discovered 13-01, blocking `windows append`). The entries are now synced (68 table rows = 68 JSON entries, ids 44/45 present), but the FRONTMATTER counts still disagree with the body: `open_count: 52 / waived_count: 3 / fixed_count: 13` vs the JSON block's actual 49 open / 5 waived / 14 fixed (table ~50/3/13 — descriptions containing `|` also skew naive table parses). Tools keying off the frontmatter (the `/gsd-ship` windows gate) read stale numbers.
- Safe modification: re-sync frontmatter counts to the JSON body before the next ledger mutation.
- Test coverage: none (hand-maintained).

**In-guest rc dialect hazards (measurement surface):**
- Files: `build/x86_64/root-tree/lib/*.rc` (all guest fixtures)
- Why fragile: rc parses blocks whole (a parse error kills a sourced file mid-stream); bare `=` ends a word (single-quote `=`-bearing argv); fd-dup redirects unreliable in nested sourced blocks; rc cannot collect per-child statuses after a bare `wait`; `arctest`-style fall-off-main returns failure status (see above). These produced multi-hour debugging detours before being measured.
- Safe modification: flat status ladders, `=`-free patterns, absolute-path invocations, marker-count asserts on the host expect side.
- Test coverage: encoded in the committed gate scripts; documented in STATE Phase-15 decisions.

**Host-tool dialect shadowing on the devshell PATH:**
- Files: `build/` scripts generally; recorded instances in `build/aarch64/mkimage.sh:165` (Harvey host dd shadows POSIX dd), coverage-sweep sed/date garbling (WINDOWS #39)
- Why fragile: Harvey's host-built dd/sed/date on PATH behave differently from POSIX (dd wants `-if/-of` dash-pair form; Plan 9 sed group syntax differs; `date -u +fmt` printed the epoch).
- Safe modification: probe-verify each host tool's dialect in situ; prefer od/awk forms for portable extraction.
- Test coverage: the gates that run them.

**KVM/APIC host sensitivity + accel discipline:**
- Files: r9 x86_64 LAPIC/timer (`../r9/x86_64/src/`); drift notes in `build/x86_64/README.md`
- Why fragile: this host's history includes a real KVM APICv canonical-state bug (kernel 7.3.0-rc6, fixed by 7.3.1 bump) and the MSR-0x832 x2APIC range gate (our bug, fixed r9 351287f); hosted runners are KVM-less so every CI leg selects TCG explicitly. The no-silent-downgrade rule: accel only from flag > R9_ACCEL/R9_CPU env > TOML kvm default; a KVM-less venue fails loudly.
- Safe modification: never hardcode `-enable-kvm`; add accel flags as config data.
- Test coverage: boot-test gates on both KVM and TCG legs; drift table in `build/x86_64/README.md`.

**Interactive-driver input wedge flakes (aarch64-era, dormant):**
- Files: `build/aarch64/drive-r9.expect` (WINDOWS #20/#22 duplicates)
- Why fragile: 2 of ~7 early sweep runs wedged at a prompt read (line never completes); 6 consecutive clean passes since; not reproducing under the cons watchdog.
- Safe modification: n/a (aarch64 frozen).
- Test coverage: the aarch64 gates (frozen).

## Scaling Limits

**WaitSet capacity vs MAX_PROCS (IN-01):**
- Current capacity: MAX_WAITERS=512 fixed sets ("sized to the live proc table — 512 on both arches") vs `dev_churn` profile's MAX_PROCS=2048 — a 4x mismatch under that profile.
- Limit: under dev_churn + dev_ip/dev_ether (not a current build), the 513rd distinct waiter hits the loud full arm.
- Scaling path: derive MAX_WAITERS from per-arch MAX_PROCS or assert against it; open review finding IN-01 (`.planning/phases/17-r9-kernel-flush-out/17-REVIEW-DISPOSITION.md`).

**Kernel heap 32 MiB fixed:**
- Current capacity: raised from 4 MiB after the services venue exhausted it mid-sweep (exec staging + resident daemons + residue).
- Limit: unbounded daemon growth (services venues) will hit it again; oom handler prints layout + high-water for diagnosis.
- Scaling path: the residue-leak audit backlog item; larger arenas are config data.

**r9 boot-module stash 64 MiB (fixed .bss cap):**
- Current capacity: raised 8→64 MiB (r9 236d5a7) for the ~26.5 MB services paq (font tree ~18.8 MB deflated).
- Limit: a payload beyond 64 MiB panics the stash-length reconciliation at the EARLY check's first boot.
- Scaling path: bump the .bss cap in r9 (a fixed constant, not venue -m).

**mkpaqfs per-file cap 4 MiB at default blocksize:** see Tech Debt — files past blocksize^2/4 silently drop; scaling a payload member means passing `-b` and asserting the fit.

## Dependencies at Risk

**plan9port host dependency:**
- Risk: the entire mk/rc/host-9c toolchain depends on 9fans/plan9port being installed and `$PLAN9/bin` positioned LAST in PATH per BUILD_ON_UNIX; upstream is a volunteer project.
- Impact: build revival breaks host-side if plan9port regresses; Harvey's own compilers (8c/6c family) are then unbuildable on new hosts.
- Migration plan: the nix pipeline (flake) pins the host toolchain — keep nix as the reproducible venue and darwin as documented-manual.

**qemu pinned at 11.1.2 (venue quirks recorded):**
- Risk: `-global q35-pcihost.MCFG` crashes qemu 11.1.2 at machine init (worked around via `-acpitable` injection); raspi3b boots defect (FAR 0xfe215004) accepted pre-existing; KVM APIC MMIO canonical-state bug on kernel 7.3.0-rc6 (fixed by host bump).
- Impact: a qemu pin advance changes MCFG/accommodation behavior; the recorded re-test conditions live in `build/x86_64/README.md`'s drift-notes table.
- Migration plan: re-test the accommodations when the qemu pin advances (explicit re-test condition recorded).

**Upstream Harvey dormant (last commit 2022-03-30):**
- Risk: this fork IS the maintained line; no upstream fixes will arrive; 9front imports are the fresh-blood source (MIT-compatible).
- Impact: in-tree fixes preferred over external patch stacks (PROJECT.md constraint).
- Migration plan: continue fork-first policy; r9 (requaos/r9 fork) likewise owns its kernel line.

**Hosted CI observation still unrecorded:**
- Risk: the rewritten `.github/workflows/harvey.yml` (nix pipeline, full replacement of the retired legacy harness 2026-11-05) has its hosted green run recorded as PENDING in the ledgers (PROJECT.md Active "pending first push (261105-slx)"; STATE.md Deferred Items "backlog 999.1"). The origin remote has since advanced (origin/master == HEAD == `bdcf45a51` per the push reflog, past the earlier `79cd4ab1d` position), but no document records an observed hosted-green run of harvey.yml/r9-shell.yml/phase6.yml at the current head.
- Impact: push/PR build signal is asserted, not observed; a silently red hosted CI would go unnoticed.
- Migration plan: observe one hosted run post-push and close the 999.1 backlog row.

## Missing Critical Features

**No RTC / epoch clock on r9 x86_64:**
- Problem: NSEC(53) returns the loud no-RTC errstr; r9 devcons serves no `/dev/bintime` so libc `nsec()` returns 0 — the tier1 gate's date check currently passes only via the ctime(0)+EST "Feb 7 2006" display quirk (WINDOWS #69).
- Blocks: honest timekeeping for anything time-keyed (auth tickets, logs); Phase 18's auth domain is the likely first consumer via the `nsec_now` seam (ready).

**No block device driver (virtio-blk) in r9:**
- Problem: fossil + venti are compile-gated only — runtime-unverifiable until r9 has a block device.
- Blocks: persistent storage venues (future milestone).

**No per-proc FP state (era limit):**
- Problem: user SSE is armed (CR4.OSFXSR/OSXMMEXCPT + per-exec fninit/ldmxcsr reset) but there is no per-proc FXSAVE — context switches do not preserve user FP registers across procs (WINDOWS-class era note from 14-08).
- Blocks: any multi-proc FP-heavy workload correctness.

**IPv6 verification deferred:**
- Problem: v6 is "not free to verify" (16-02 OQ-4) — no v6 key this phase; WR-03's v6-quad leak arm exists precisely because listeners accept both families unverified.
- Blocks: v6 claims of any kind.

**fgui / libcontrol closure unowned:**
- Problem: fgui stays excluded from tiers (its `<control.h>` → libcontrol closure is owned by no plan); it appends when one does, never silently.
- Blocks: GUI-tier completeness for control-toolkit apps.

## Test Coverage Gaps

**rc corner fixes are audit-grounded, not fixture-pinned (WINDOWS #46/#49/#65):**
- What's not tested: no targeted fixtures exist — no sweep fails a dup; absolute-path invocation never exercises #65's masking path; the #49 class was HVF-timing-only (unreproducible on the linux TCG/KVM venue). Pins are the re-greened sweeps + invariant shapes.
- Files: `sys/src/cmd/rc/exec.c`, `sys/src/cmd/rc/plan9.c`, `sys/src/cmd/rc/havefork.c` (fix commit `6779df7d7`)
- Risk: a regression in these exact arms would not redden any gate; reopeners are live-fire ("a live 'pushword but no argv' fire on any venue").
- Priority: Medium (documented escape clause D-11).

**services-x64 gate has no green run since the 15-08 era:**
- What's not tested: the entire services sweep (dns, auth-adjacent daemons) is blocked by the pre-existing #68 dns-stage kernel trap.
- Files: `build/x86_64/services-r9.expect`, evidence `build/x86_64/services-r9-x64.log`
- Risk: service-class regressions ship unnoticed; Phase 18/19 depend on this venue.
- Priority: High (root cause owned by the next services-touching plan).

**CR-07 class has a reproduction instrument but no passing regression gate:**
- What's not tested: the waived class cannot be covered by a deterministic gate (timing-sensitive, ~8%/run); the committed `boot-r9-stress-x64` instrument fires it at will but a green run proves little (P(green|unfixed) ≈ 91%).
- Files: `build/x86_64/stress-r9.expect`, `.planning/phases/17-r9-kernel-flush-out/17-CR07-ROOTCAUSE.md` §5, §8-9
- Risk: the reopener (a context-reaching venue or non-healing instrumentation) is the only path to closure.
- Priority: accepted-by-waiver; reopen on instrument availability.

**Payload bakes beyond gs lack fit/diagnostic asserts:**
- What's not tested: marker/chain/tier/net bakes and bootfs-draw.paq do not fail on mkpaqfs warnings (see Tech Debt).
- Files: `build/x86_64/mkimage.sh`, `build/mm/build-9kdraw.sh`
- Risk: a silently-dropped payload member ships an incomplete image green.
- Priority: Medium.

**In-guest GUI execution is trace-gated, artifact-gated only at tier 3:**
- What's not tested: T3-ART is artifact evidence ONLY — compile green is never execution proof for the GUI tier; traces (15-06) are the gate, and samterm's fragile paths are worked around rather than covered.
- Files: `build/x86_64/` trace rig, `sys/src/cmd/samterm/`
- Risk: GUI-class regressions surface only in trace diffs.
- Priority: Low (era-appropriate).

---

*Concerns audit: 2026-11-13*
