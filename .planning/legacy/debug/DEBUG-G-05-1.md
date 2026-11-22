---
status: resolved
trigger: "G-05-1: nix run .#boot-r9-smoke fails on ubuntu-latest (x86_64-linux) — subset build dies at libString with zero mk compile output"
created: 2026-11-03T00:00:00Z
updated: 2026-11-03T02:30:00Z
---

## Current Focus
<!-- OVERWRITE on each update - reflects NOW -->

hypothesis: CONFIRMED and FIXED — see Resolution. (The orchestrator-supplied symptom "died at libString with zero mk output" was a misread of a streaming log; the raw CI log shows all 13 libs green and the failure at boot(8).)
test: verified — see Resolution.verification
next_action: DONE — root cause confirmed, fix committed (d758c2071), docs committed (7167dc642), knowledge-base entry written. Verified: x86_64 subset build SUBSET-BUILD-OK, lima aarch64 leg SUBSET-BUILD-OK (non-regression), and the FULL exact CI command `nix run .#boot-r9-smoke` end-to-end PASS with 'smoke-r9: ALL STAGES PASS' in the emulated x86_64 container. Remaining external gate: re-run hosted CI after push (out of scope here).

reasoning_checkpoint:
  hypothesis: "execve of rc/bin/membername fails ENOENT because its shebang interpreter /bin/rc does not exist in the mount namespace on a fresh host (the namespace shares the distro /bin; nsrun.sh never provides /bin/rc). Empty `names` → empty libboot.a7 → 7l undefined symbols → mk exit 1 → SUBSET-BUILD-FAIL boot(8) → smoke step 1 fails."
  confirming_evidence:
    - "Raw CI log (run 41750071242): all libs green incl. libString; 'membername: No such file or directory'; empty archive; '7l ... exit status=exit(1)'; SUBSET-BUILD-FAIL: boot(8) aarch64 build"
    - "Emulated x86_64 full-fidelity repro (nix run .#boot-r9-smoke, exact CI command): byte-identical failure at boot(8)/membername"
    - "Micro-proof in x86_64 container: /bin/rc absent; membername via PATH → 'cannot execute: required file not found'; after cp \$PLAN9/bin/rc /bin/rc → membername '...libboot.a7(bootauth.7)...' → 'bootauth.7 aux.7' rc=0"
    - "Lima guest (both passing legs) HAS /bin/rc (613KB, dated Nov 1, undocumented manual state); legacy docker path codified the same convention (build/Dockerfile:42 RUN cp \$PLAN9/bin/rc /bin/rc)"
  falsification_test: "If, with /bin/rc provided in the namespace, the x86_64 subset build still fails at boot(8) with the same undefined-symbol link error, the hypothesis is wrong. (It does not fail — see Verification.)"
  fix_rationale: "Provide /bin/rc at the namespace layer (nsrun.sh), not by editing the three boot mkfiles or the upstream membername script: fixes the whole class (any #!/bin/rc recipe from mk), matches the project's own Dockerfile:42 precedent, keeps native-Plan 9 builds intact (no /usr/bin/env shebang hack), and needs no host-side state."
  blind_spots: "Stages after the subset build (bake, DTB patch, expect smoke) had never run on x86_64 — CLOSED post-fix: the full exact CI command passed end-to-end in the emulated x86_64 container, including the qemu TCG boot and the ALL STAGES PASS marker."
  candidate_causes:
    - "environment: missing /bin/rc on fresh hosts (CONFIRMED root)"
    - "code: mkfile.aarch64's use of the membername idiom (not a bug — the idiom is shared by both upstream bootmkfile templates; the environment contract was the missing piece)"
    - "process: undocumented manual /bin/rc placement in the lima guest masked the missing contract on the two passing legs"
  and_gate: "no — single sufficient cause; the arch axis was a red herring (it correlated only because the passing legs shared one VM image with manual state)."

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: sys/src/9k/boot/mkfile.aarch64:50 (and the shared templates sys/src/9k/mk/bootmkfile:21, sys/src/9/boot/bootmkfile:22) invoke `` `{membername $newprereq} ``; rc/bin/membername is an rc script with shebang `#!/bin/rc`, and build/aarch64/nsrun.sh's namespace never provides /bin/rc — so on any fresh host (GH ubuntu-latest x86_64; reproduced in an emulated x86_64 docker container) exec'ing membername fails ENOENT, `names` is empty, `ar vu libboot.a7` creates an EMPTY archive, and `7l -o boot bootr9.7 libboot.a7 printstub.7` fails with undefined boot/configembed/connectembed → mk exit 1 → SUBSET-BUILD-FAIL: boot(8) aarch64 build → smoke-r9.sh step 1 → CI failure. The darwin/aarch64 legs passed only because the shared lima VM carries an undocumented manually-placed /bin/rc (Nov 1), and the legacy Docker path provides it via build/Dockerfile:42. NOT a libString failure — the reported symptom was a misread of the streaming CI log (raw log: libString and all 13 libs build green).
fix: build/aarch64/nsrun.sh — in the root/namespace section, bind the wrapper's plan9port rc over /bin/rc (touch the mountpoint when missing), fail-hard with a clear message when $PLAN9/bin/rc is absent. Mirrors build/Dockerfile:42's precedent inside the private mount namespace.
verification: |
  BOTH deliverable axes GREEN:
  (1) x86_64-linux container (emulated, Rosetta; ubuntu:24.04 + nix 2.35.4 + flake.lock-pinned toolset + vendored x86_64 hostbin + exact CI command env): BEFORE the fix the byte-identical CI failure reproduced (membername ENOENT → empty libboot.a7 → 7l undefineds → SUBSET-BUILD-FAIL: boot(8)); WITH the fix: membername extracts members (`a - bootauth.7`, `a - boot.7`, ...), link succeeds, `SUBSET-BUILD-OK`, and the driver's host-side PASS asserts all pass (16+2 binaries present; R_MAGIC 00008c47 verified on boot/init/paqfs/stacksmoke) → SUBSET RC=0. Log: /tmp/g05-verify.log.
  (2) aarch64 lima leg (non-regression): `nix develop -c sh build/aarch64/subset-build.sh` on darwin-aarch64 with the fix → SUBSET-BUILD-OK, rc=0, full PASS (33 entries) — the bind over the guest's pre-existing /bin/rc is transparent. Log: /tmp/g05-lima-verify.log.
  Fix-acceptance signals: reproduction of the original failure before fix (byte-identical to CI run 41750071242) ✓; original failure absent after fix on the failing axis ✓; passing axis re-verified ✓; mechanism explained (shebang interpreter resolution) ✓; fix is minimal (11 lines, one file, additive) ✓.
  Remaining external gate: the hosted CI re-run (requires push — not performed in this session).
  BONUS (closes the blind spot): the FULL exact CI command `nix run .#boot-r9-smoke` with the fix in the x86_64 container went end-to-end — SUBSET-BUILD-OK → bake PASS (bootfs.paq, 1015960 bytes) → DTB patch → expect smoke → **`smoke-r9: ALL STAGES PASS — boot, echo, ls/ps/cat sweep, DEL safety, restart`** → SMOKE RC=0 (log: /tmp/g05-verify-full.log, marker at line 1734). The bake/DTB/smoke stages were never-before-run on x86_64; they pass. This demonstrates the gap's full truth locally (modulo Rosetta-emulation vs real-hardware, which is immaterial to the fixed mechanism).
files_changed: [build/aarch64/nsrun.sh]

## Symptoms
<!-- Written during gathering, then IMMUTABLE (prefilled by orchestrator) -->

expected: `nix run .#boot-r9-smoke` exits 0 on ubuntu-latest (x86_64-linux) with 'smoke-r9: ALL STAGES PASS' in the run log
actual: Run FAILED on x86_64-linux CI. Subset build enters NATIVE_LINUX path (sudo unshare), builds libc, libbio, libregexp, libip, libndb, libmp, libsec green in order, then prints `== subset lib: libString ` and NOTHING after — zero mk compile lines. Driver tail: `error: guest subset build failed (rc=1)` + hint line. No `SUBSET-BUILD-FAIL: <dir>` marker in captured log (possibly truncated).
errors: `error: guest subset build failed (rc=1)`
reproduction: GitHub Actions ubuntu-latest, workflow .github/workflows/r9-shell.yml — install nix → `nix run .#boot-r9-smoke`. First-ever execution on the x86_64-linux axis.
started: first run on x86_64-linux (never proven on this axis). Passes on darwin-aarch64 (lima, 3/3 at HEAD) and aarch64-linux native (same sudo-unshare path).

## Eliminated
<!-- APPEND only -->

- hypothesis: "libString's mkfile/content breaks mk on x86_64 (parse/startup failure, non-printing byte, arch-specific mk bug)"
  evidence: "mkfile od-verified byte-clean; structurally identical to libregexp's (which built); ALL 13 libs incl. libString built green in the emulated x86_64 repro with identical tools; plan9port commit-log audit found no relevant mk bug; raw CI log shows libString itself built green in CI too — the premise itself was a misread."
  timestamp: 2026-11-03T01:50:00Z

- hypothesis: "set -e on an unwrapped silent command / violent guest-bash death with rc mangling (the 'no SUBSET-BUILD-FAIL marker' constraint)"
  evidence: "ubuntu 24.04 su maps signal death to 128+N (empirical: SIGSEGV→139, SIGKILL→137, exit1→1); nixpkgs util-linux ships no su (CI uses the same distro su); bash empirically ALWAYS prints the fail marker + a signal-notification on subshell failure. The constraint dissolved when the raw CI log showed the marker WAS present."
  timestamp: 2026-11-03T01:50:00Z

- hypothesis: "ENOSPC (runner disk full) swallowing all log writes mid-loop"
  evidence: "Mechanism test showed append-writes continue into the log file's partially-used last page (marker survives); knife-edge margin required; then definitively eliminated by the raw CI log (full error trail present, no truncation)."
  timestamp: 2026-11-03T01:50:00Z

- hypothesis: "signed-char / x86_64 miscompile in vendored host tools (7c/cpp/ar/iar)"
  evidence: "zero-mk-output signature would have required the failure before mk's first recipe echo; libString sources are pure ASCII; and the x86_64 container built everything with the same-rev nix gcc."
  timestamp: 2026-11-03T01:35:00Z

- hypothesis: "261103-giq harness rewiring (Rust harness replacing Python tools)"
  evidence: "the rewiring touched boot-9qemu.sh/donor-ns-build.sh/run-gate.sh/subset-exec.sh/trace-boot.rc only — none on the subset-build/nsrun path; the harness enters at bake time, after the failing step. Structurally exonerated, then confirmed by the byte-identical repro at HEAD which includes those commits."
  timestamp: 2026-11-03T02:00:00Z

## Evidence
<!-- APPEND only -->

- timestamp: 2026-11-03T00:00:00Z
  checked: build/aarch64/subset-build.sh (full read)
  found: lib loop at lines 161-164 is exactly as orchestrator described: `for d in libc libbio ... libflate; do echo "== subset lib: $d"; (cd "/sys/src/$d" && objtype=aarch64 mk install) || fail "lib $d"; done`. Guest script has `set -e` (line 137). fail() prints `SUBSET-BUILD-FAIL: $1` to stderr and exit 1 (line 139). Driver redirects both stdout+stderr to $LOG (line 235: `> "$LOG" 2>&1`), so the fail marker WOULD be captured if fail ran. Missing marker + rc=1 ⇒ either the subshell was killed by a signal that bypassed `||` (impossible — `||` catches any non-zero), OR `set -e` fired on an unwrapped command, OR the fail echo itself didn't flush, OR the log paste was truncated.
  implication: The death is inside the `(cd ... && objtype=aarch64 mk install)` subshell OR in an unwrapped line. Since the echo DID print (`== subset lib: libString` seen), and mk produced ZERO output, mk likely died at startup/parse — before printing anything. rc=1 (not 128+N) argues against signal death of the whole guest script... but nsrun.sh/sudo wrapping may mask the real rc.

- timestamp: 2026-11-03T00:00:00Z
  checked: sys/src/libString/mkfile (full read)
  found: Structurally standard mksyslib mkfile: `</$objtype/mkfile`, LIB=/$objtype/lib/libString.a, 16 OFILES (s_alloc through s_unique), HFILES=/sys/include/String.h, UPDATE block with `${LIB:/$objtype/%=/386/%}` substitution, includes `</sys/src/cmd/mksyslib`.
  implication: Nothing arch-specific visible in the mkfile itself. The `${LIB:...=/386/%}` substitution is interesting (hardcodes /386/ in UPDATE) but UPDATE is only used by the `update` target, not `install`. Need to compare with libregexp mkfile and read mksyslib.

## Evidence (continued)

- timestamp: 2026-11-03T00:15:00Z
  checked: .planning/phases/05-shell-boot-one-command-integration/05-UAT.md (test 1 result)
  found: User's report of the hosted run: "guest subset build died entering libString... NO 'SUBSET-BUILD-FAIL: <dir>' line present in the log, so the guest script died without reaching its own fail() — suspicious of set -e kill or mk instant failure. Host toolchain bootstrap succeeded; libc/libbio/libregexp/libip/libndb/libmp/libsec all built green... failure is exactly at libString's mk invocation."
  implication: Marker genuinely absent (not paste truncation). rc=1, not 128+N — so NOT an obvious signal-death of the guest bash. Every in-loop failure path (cd fail, mk fail, mk crash) routes through `|| fail` which prints the marker via unbuffered write(2). Absent marker ⇒ death occurred OUTSIDE the `||` protection, OR mk exited 1 with zero output and fail's echo... no, fail's echo is unbuffered bash echo to stderr — it WOULD appear. Strongest residual candidate: the guest bash died via `set -e` on a command whose own stderr was empty, or mk exited 1 silently.

- timestamp: 2026-11-03T00:15:00Z
  checked: git ls-files sys/src/libString + dir listing
  found: 18 tracked files (mkfile + 17 .c). The .6/.8 objects in the dir are UNTRACKED local artifacts (prior amd64/386 builds on this mac) — not present in CI's fresh checkout. All 17 OFILES have matching .c files. No missing-source angle.
  implication: libString's tracked content is complete and mundane; the mkfile is structurally identical to libregexp's (same includes, same UPDATE substitution, same mksyslib tail). If the failure is libString-specific, the trigger is either (a) a non-printing byte in the mkfile, (b) state accumulated across 7 prior iterations (env/fs), or (c) not libString-specific at all but x86_64-tool-specific and merely first-tripped at libString.

- timestamp: 2026-11-03T00:15:00Z
  checked: flake.nix smokeR9Pkg + mkScriptApp + guest PATH chain
  found: smoke app = writeShellApplication with runtimeInputs [plan9port bash qemu dtc git expect util-linux gnused mount binutils]; mkScriptApp exports PATH="$PATH:$PLAN9/bin" (plan9 LAST). subset-build.sh prepends $HARVEY_LINUX_BIN (hostbin) FIRST. nsrun --wrap captures `export` output, re-sources inside namespace; su -c appends /rc/bin:. . Guest's mk = plan9port mk (from nix store, arch-specific build); ar/7a/7c/7l/cpp = vendored hostbin built by host cc during .#toolchain. CI runs `nix run` (NOT devshell) so driver line 105's exports (FORCERCFORMK=1 objtype=amd64 etc.) are set by subset-build.sh itself.
  implication: All tools already exercised by 7 prior libs on the same x86_64 host. An x86_64-only tool bug would need to be triggered by something libString's mkfile/invocation does that the prior 7 don't.

- timestamp: 2026-11-03T00:25:00Z
  checked: 05-13-linux-clean-clone.log (passing aarch64-linux run, same NATIVE_LINUX path)
  found: libString's expected output = 17 `7c -FTVw s_*.c` lines, then `ar vu /aarch64/lib/libString.a <17 .7 files>`, `ar: creating`, `a - s_*.7` lines. The failing x86_64 log has NONE of these — mk never executed even the first recipe line. mk echoes recipe lines as it runs, so mk died/exited during mkfile parse or dependency-graph evaluation, OR mk exited 1 silently, OR mk's buffered stdout was lost to a hard kill (but rc=1 argues against signal death of the guest).
  implication: Bohrbug class (deterministic, first-ever-run, arch-specific). The differentiator between mk-on-x86_64 and mk-on-aarch64 with identical inputs is now the crux. Static analysis exhausted (mkfile byte-clean, structure identical to libregexp, all tools proven on 7 prior libs). Moving to empirical reproduction in emulated x86_64 docker.

- timestamp: 2026-11-03T00:25:00Z
  checked: git clone on macOS host
  found: Repo has case-insensitive-FS collisions (rc/bin/Kill vs kill, troff HX/Hx, devpost C1/c1...) — clone on APFS loses files. NOT the CI bug (both CI legs are case-sensitive Linux), but means any local-mac repro is invalid; must clone INSIDE the linux container (case-sensitive overlayfs).
  implication: Container repro will `git clone /orig /src` inside, not bind-mount a mac clone.

- timestamp: 2026-11-03T00:45:00Z
  checked: plan9port full commit log (src.adamsgaard.dk mirror) for mk x86_64 bugs
  found: The search snippet "silent exit 1 at startup ... O_LARGEFILE on x86_64 linux" was a mashup of TWO unrelated commits: "lib9: fix int vs ulong confusion causing silent exit 1 at startup" (2018, the gview graphics viewer — irrelevant) and a 9pfuse glibc O_LARGEFILE workaround (2014, ARM — irrelevant). No mk x86_64 silent-exit bug in plan9port history. mk-relevant fixes (hash function #315, repeated-target infinite loop, overlapping strcpy) are all ancient (2015-2020) and long shipped in nixpkgs.
  implication: ELIMINATED — no known upstream mk bug explains the signature. (Not ruling out a nixpkgs x86_64 build-specific issue, but no evidence for one.)

- timestamp: 2026-11-03T00:45:00Z
  checked: passing-log transition calibration (05-13-linux-clean-clone.log lines 1680-1696)
  found: libsec's tail: bare `aarch64` line (recursion dir echo), then `ar vu /aarch64/lib/libsec.a ` (EMPTY newprereq, trailing space — the :V-forced recipe with no new prereqs), exiting 0 silently. Then `== subset lib: libString` (NO trailing space — od-verified), then `7c -FTVw s_alloc.c`. The CI log's "trailing space" after libString is a transcription artifact, not data.
  implication: The empty-`ar vu` idiom already ran twice on the x86_64 CI host (libmp + libsec aarch64 legs) without killing anything. libString's own `ar` at the END of its build is not the first-trip point either (compile lines would precede it).

- timestamp: 2026-11-03T00:50:00Z
  checked: GitHub API for the failing run (repo requaos/harvey is public)
  found: r9-shell run id=41750071242, run_number 1, head_sha=f7d60d579, conclusion=failure, duration ~8min (00:52:38→01:00:54). ~8 minutes ≈ nix install + toolchain bootstrap + 7 libs — consistent with dying early at libString, NOT with a hang/timeout. Log download needs auth (403) — user's "no SUBSET-BUILD-FAIL marker" report stands as the evidence.
  implication: rc=1 at ~8min = orderly, early exit. Final constraint set for the root cause: (1) zero mk output, (2) no fail marker, (3) rc=1 not 128+N, (4) x86_64-linux only. Every `||`-wrapped path prints the marker, so the death is EITHER set -e on a silent unwrapped command (none visible in the script between the echo and the subshell — points INSIDE the subshell's own early moments or at mk's process level) OR mk exiting 1 before its first recipe echo with stderr lost/empty.

- timestamp: 2026-11-03T00:50:00Z
  checked: docker repro attempts 1-2 (env setup, not the bug itself)
  found: classic nix installer refuses single-user-as-root; DeterminateSystems installer hits "unable to load seccomp BPF program: Invalid argument" — nix's build sandbox is incompatible with x86_64 emulation (Rosetta/qemu-user). v3 script: non-root `builder` user (matches CI's non-root runner), single-user nix, `sandbox = false` in nix.conf.
  implication: repro environment adjusted; running.

- timestamp: 2026-11-03T01:10:00Z
  checked: empirical bash semantics test (host, darwin bash — POSIX semantics hold)
  found: `( bash -c 'kill -SEGV $$' ) || fail "lib x"` under `set -e` prints BOTH bash's own "Segmentation fault: 11" diagnostic (stderr, captured) AND the SUBSET-BUILD-FAIL marker; outer rc=1. So ANY in-subshell failure — ordinary or violent — yields the marker, and violent deaths additionally yield a bash signal-notification line.
  implication: The absent marker + absent signal-notification means the libString subshell NEVER FAILED in any way. Combined with zero mk output, the subshell either never executed mk, or the CI log capture is incomplete.

- timestamp: 2026-11-03T01:12:00Z
  checked: ubuntu 24.04 su exit-code mapping (x86_64 container, empirical)
  found: su -c "exit 1" → rc=1; su -c "kill -SEGV" → rc=139; su -c "kill -KILL" → rc=137. Signal death is preserved as 128+N through su; the same holds transitively for unshare/sudo (standard 128+N propagation).
  implication: CI's rc=1 ⟹ ORDERLY exit-1 of the guest script. NOT a signal death of anything in the chain. The orderly-exit-1 paths are: fail() [would print marker — absent] or `set -e` on a failing command whose own stderr is empty/absent. The loop body contains only echo + the ||-wrapped subshell... so EITHER the reporter's log is incomplete (marker+mk-error truncated), OR an extraordinary mechanism (e.g., log-file write failure — disk full — swallowing all output including the marker, which also explains the clean line-boundary cutoff).

- timestamp: 2026-11-03T01:25:00Z
  checked: docker repro environment bring-up (v5)
  found: Emulation is Rosetta (VirtualApple CPU signature) — faithful for userspace C; a vendored-tool bug WILL reproduce. seccomp-BPF failure under emulation defeated via /etc/nix/nix.conf sandbox=false + filter-syscalls=false; nix 2.35.4 works from the store path; nix run .#setup COMPLETED (hostbin populated). nixpkgs pinned at 31ecbaca1bde (2026-10-27 nixos-unstable) — identical rev both arches. Toolchain (vendored 6a..7l/cpp/aux/iar via nix gcc x86_64) compiling now.
  implication: repro environment is faithful for the failure class in question. Awaiting subset-build outcome.

- timestamp: 2026-11-03T01:45:00Z
  checked: FULL x86_64 emulated repro of the exact CI sequence (ubuntu 24.04 + nix + flake-equivalent env + vendored x86_64 toolchain + sudo-unshare NATIVE_LINUX subset build)
  found: **ALL 13 LIBS BUILT GREEN — libString (log line 1194) compiled 17/17 and archived fine.** The run proceeded through libthread/lib9p/libauth/libauthsrv/libflate and the cmd subset, failing only MUCH later at "boot(8) aarch64 build" (a different, later 05-02 step — separate matter). The libString failure DOES NOT REPRODUCE with byte-identical sources, the same flake.lock nixpkgs rev (same store binaries), and the same-arch vendored toolchain.
  implication: PIVOT — the cause is NOT in committed code/mkfiles/toolchain source. It is specific to the GH ubuntu-latest x86_64 runner environment. Remaining deltas vs my container: (1) runner disk pressure — GH images have ~14-25GB free and a FAT /usr/lib that nsrun.sh copies wholesale into /tmp (NSLIB staging); my container's /usr/lib is tiny; (2) ext4 vs overlayfs; (3) Rosetta vs real hardware; (4) detsys daemon-mode nix vs my single-user (n/a to the namespace build).

- timestamp: 2026-11-03T01:45:00Z
  checked: hypothesis re-rank after negative repro
  found: Only ENOSPC fits ALL observed facts simultaneously: (a) mk's Bwrite/recipe echoes to a full log fs fail SILENTLY (mk never checks Bwrite rc); (b) 7c's "can't create s_alloc.7" stderr write fails silently; (c) mk's "exit status" stderr write fails silently; (d) fail()'s marker write fails silently; (e) the guest exits ORDERLY with rc=1 (fail's exit 1 / set -e) — matches su's proven 128+N-vs-1 mapping; (f) the log ends at the last successful write — the "== subset lib: libString" echo — a clean line boundary; (g) the driver's own stderr lines survive because they go to the step's PIPE, not the disk; (h) x86_64-runner-only because only the GH fat image's /usr/lib + nix store + runner free-space margin collide — the lima guest and my slim container have small /usr/lib and ample space.
  implication: ENOSPC is the leading hypothesis. Falsification test: fill the guest log+work fs mid-build in the container and check the EXACT signature (clean echo-boundary cutoff, zero mk output, no marker, rc=1). Mechanism fix lever: nsrun.sh's NSLIB staging copies ALL of the distro /usr/lib — the dominant disk consumer unique to fat CI images.

- timestamp: 2026-11-03T01:47:00Z
  checked: ENOSPC mechanism test (tmpfs filled mid-loop in x86_64 container)
  found: When the fs filled, the log's partially-used last page kept absorbing output — the marker AND error text landed. A disk-full cutoff would rarely land exactly on a clean echo boundary; and the driver stderr lines surviving while disk writes fail is only consistent if the GH agent streams logs over network. Plausibility marginal (runner free space vs footprint).
  implication: ENOSPC weakened but not eliminated at this point — SUPERSEDED minutes later by the raw CI log (see 01:50 entry), which eliminated it entirely: the marker and a full mk error trail ARE present in the raw log. The reported "libString/zero-output/no-marker" signature never existed.

- timestamp: 2026-11-03T01:50:00Z
  checked: RAW CI log (fetched via API with the machine's gh keychain token; run 41750071242)
  found: **The orchestrator-supplied symptom was inaccurate.** Raw log: ALL 13 libs build green (libString at step-log line 2581, with full 17-compile output), cmd subset green, then the boot chain: `names=`{membername $newprereq}` → `membername: No such file or directory` → `ar: creating libboot.a7` (EMPTY) → `7l -o boot bootr9.7 libboot.a7 printstub.7` → `main: undefined: boot` + `semacquire: configembed: not defined` + `semacquire: connectembed: not defined` → `mk: 7l ... exit status=exit(1)` → **SUBSET-BUILD-FAIL: boot(8) aarch64 build** → `error: userland build failed` (smoke-r9.sh step 1). The marker IS present; the log does NOT end at libString. The "libString, zero output, no marker" report was a misread of the streaming log view (the driver cat's all lines share timestamp 01:00:49.3 — the log appears all at once at step end).
  implication: Investigation pivots from "libString silent death" (which was never real) to "membername ENOENT at boot(8) on x86_64-linux".

- timestamp: 2026-11-03T01:55:00Z
  checked: membername provenance (repo + nixpkgs plan9port + passing log)
  found: `membername` is an rc script at rc/bin/membername (mode 100755, shebang `#!/bin/rc`, a tr|sed|tr pipeline extracting member names from archive(member) tokens). NO membername.c in sys/src; aux/BUILD builds only mklatinkbd+data2s; nixpkgs plan9port 2025-11-10 ships NO membername on darwin-aarch64 OR x86_64-linux. It resolves via the namespace PATH tail `/rc/bin` (nsrun.sh su line). Used by 3 build rules: sys/src/9k/boot/mkfile.aarch64:50 (05-02), sys/src/9k/mk/bootmkfile:21, sys/src/9/boot/bootmkfile:22. Passing aarch64 log line 2017: the same recipe ran fine (members archived).
  implication: exec'ing /rc/bin/membername requires the kernel to exec shebang interpreter /bin/rc. /bin is the distro /bin inside the namespace (never bound). Fresh hosts lack /bin/rc.

- timestamp: 2026-11-03T02:00:00Z
  checked: /bin/rc presence across environments
  found: lima guest (BOTH passing legs — darwin drives it; 05-13's "aarch64-linux native" run was INSIDE it): /bin/rc EXISTS (613640 bytes, root, dated Nov 1 11:01 — undocumented manual state; no provisioning/doc reference found in repo). Legacy docker path: build/Dockerfile:42 `RUN cp ${PLAN9}/bin/rc /bin/rc` — the convention was codified there but never carried into the nix/namespace path (nsrun.sh). Fresh x86_64 hosts (GH runner, my container): /bin/rc ABSENT.
  implication: Root cause chain closed. The arch axis was a red herring — it correlated only because both proven legs shared one VM image carrying manual state.

- timestamp: 2026-11-03T02:05:00Z
  checked: micro-proof in x86_64 container (plan9port 2025-11-10, same store path as CI)
  found: /bin/rc absent; `membername 'libboot.a7(bootauth.7)' 'libboot.a7(aux.7)'` via PATH → "cannot execute: required file not found" (the ENOENT). After `cp $PLAN9/bin/rc /bin/rc` → prints `bootauth.7 aux.7`, rc=0.
  implication: Mechanism proven at the smallest unit. Fix: provide /bin/rc in the private namespace (nsrun.sh) per the Dockerfile:42 precedent.

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: (see line 34 — the Resolution block above)
fix: (see line 35)
verification: (see Verification section appended below)
files_changed: []
