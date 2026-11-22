---
status: awaiting_human_verify
trigger: "Owner (via coordinator, 2026-11-19): after shared-fgrp-pipe-leak (r9 3a1c9e5) — fix the three follow-ups as separate local r9 commits on main HEAD: (1) bare rfork(RFCFDG) gets a fresh empty table on both arches; (2) port the aarch64 borrow-close fix (x86_64 ab0a977) and the #44 exited-proc cleanup; (3) the flaky proc::sem_tests isolation. port/src edits need a diff checkpoint first; regression test per fix where feasible; same verification as before; report x86_64 byte change (boot-iso outputHash re-derive, x64 TCG auth+net gates in a fresh amd64 container) and the final aarch64 kernel hash; no push."
created: 2026-11-19T03:30:00Z
updated: 2026-11-19T03:30:00Z
---

## Current Focus

base: r9 main 3a1c9e5 (on b82ee9d); work clone scratchpad/r9-fu; scratchpad = /private/tmp/claude-501/-Users-neilskinner-code-harvey/55ded085-3397-4d2c-931d-1d3a7a81443a/scratchpad
parent_session: .planning/debug/resolved/shared-fgrp-pipe-leak.md (findings D2, borrow-close/#44, sem flake)
plan:
  - fix1 (both arches, syscall.rs): bare RFCFDG = fresh FdGroup via a returning helper `proc::split_fd_group_in_place(slot, copy)` (RFFDG copy arm moves in too); host tests on aarch64 (darwin) and x86_64 (amd64 docker)
  - fix2 (aarch64 only): release_fd_group consumes one chan at a time (ab0a977); `sweep_husk_payloads` = closepgrp mount half on the last pgrp ref + pgrp/env/rgrp singleton swaps + slash/dot plain drop (x86_64's final 72edbba+ab0a977 shape); sys_exits copies msg to a stack buffer. Host tests: consume-order test via a shared Arc count; mount-half test on a Pgrp with mounts
  - fix3 (aarch64; x86_64 if it shares the code): sem_deregister_waiter swap_remove → order-preserving remove (the FIFO the wake documents); deterministic regression test
commits (clone scratchpad/r9-fu, on 3a1c9e5): f8c2ac9 fix1, 5893ae9 fix2, 7b598f2 fix3; kernels K0 3a1c9e5 mhi4in5c…, K1 f8c2ac9 7asg72dh…, K2 5893ae9 hzqx82la…, K3 7b598f2 /nix/store/2inqg9mw20zfb9c8300zrhiy8x5l0nqv-r9-kernel-virt-7b598f2
status_note: all three landed on main r9 (fast-forward to 7b598f2, unpushed); verification complete (see Resolution). Awaiting owner verification; then archive to resolved/ + KB entry.
next_action: on owner confirmation — archive this file + KB entry; owner decides the port-side refcount accessor (fix 2 host pin), the r9 push, the Harvey pin advance + arm64 HVF series, and the x64 boot-iso outputHash + x64 TCG auth/net gates

## Symptoms

(see parent session; each follow-up is a reading-based finding from it)

## Eliminated

## Evidence

- timestamp: 2026-11-19T03:40:00Z
  checked: fix-1 scope — the bare (non-RFPROC) split arms on both arches (aarch64 b82ee9d, x86_64 since 15-03) and donor sysproc.c (`ofg = up->fgrp; up->fgrp = dupfgrp(...); closefgrp(ofg)`)
  found: besides the singleton (D2), both arms DROP the old handle with no closefgrp. On a private table (count 1) its chans skip close_chan: after RFFDG every pipe end keeps deep_copy's extra open count (Chan::dup bumps pipe/ether/ip/ssl exactly as close_chan decrements them, so release is balanced); after RFCFDG nothing closes. Harvey bare-RFFDG callers with private tables: rio.c:290, acme exec.c, plumb match.c:425, cfs, pptp (lib9p post.c and libthread exec.c share their tables at that point)
  implication: fix 1 = fresh table AND release of the old one (proc::split_fd_group_in_place → release_fd_group), both arches

- timestamp: 2026-11-19T04:10:00Z
  checked: fix 1 (clone commit f8c2ac9): aarch64 + x86_64 split_fd_group_in_place, x86_64 gains release_fd_group (consume drain, exit_tail now calls it); 3 new pins on aarch64, 6 on x86_64 (3 release + 3 split)
  found: aarch64 host tests 24/24; x86_64 (amd64 docker rust:1-bookworm, nightly-2026-01-30 via rust-toolchain.toml) 111/111 (baseline 105 at 3a1c9e5); `cargo xtask check` rc=0; `cargo xtask clippy` (x86-64) rc=0 with a diagnostic set IDENTICAL to the 3a1c9e5 baseline (82 normalized lines, diff empty); the two imports fix 1 orphaned (aarch64 Arc + Lock, x86_64 Lock) were removed — aarch64 warning set back to the pre-existing 4. Manual mutants at split_fd_group_in_place 3/3 KILLED (RFCFDG→empty_fdgroup() [D2 itself], old handle dropped without release, copy arms swapped)
  implication: fix 1 accepted at host level; boot probes R1/R2 pending

- timestamp: 2026-11-19T04:40:00Z
  checked: fix 2 (clone commit 5893ae9, aarch64 only): release_fd_group consumes one chan at a time; empty_pgrp/empty_env_group/empty_rendez_group + release_pgrp (closepgrp with the mount half) + sweep_husk_payloads (slash/dot plain drop per 72edbba), called in exit_tail after rendez_remove and BEFORE free_user_mappings; sys_exits stack buffer
  found: aarch64 host tests 28/28 (+ last_sharer_drain_closes_every_duplicate, husk_sweep_tests x3). Mutants: release_pgrp `>= 1` KILLED, release_pgrp no-close KILLED; fd drain back to the BORROWED batch close SURVIVED — the difference shows only in Arc strong counts (devmnt clunk_if_last's test), and Chan.pipe / Chan.mnt_fid are pub(crate) in port, so no arch-crate host test can observe it. (First mutant pass wrongly ran only fdgrp_release_tests — re-run with both modules.)
  implication: the borrow-close part needs a boot-level observable (probe M1: ramfs -D Tclunk count) or a port test accessor (port/src → owner checkpoint); not host-pinned

- timestamp: 2026-11-19T05:00:00Z
  checked: fix 3 (clone commit 7b598f2, both arches): sem_deregister_waiter swap_remove → remove; pin deregister_on_one_word_keeps_another_words_fifo on both
  found: the swap_remove mutant fails the pin deterministically (left [72, 71]); aarch64 fresh test binary 0/3000 failures (baseline 1/500 — expected ~6, P(0)≈0.25%); x86_64 112/112 and 0/1000 binary reruns. Venue pitfall caught: macOS `cp` (clonefile) preserved the backup's old mtime, so cargo kept a stale MUTANT binary after a cp-restore (2965/3000 "failures" on correct source); a touch + rebuild fixed it. Earlier mutant runs were unaffected (each mutant is a perl rewrite → fresh mtime → rebuild).
  implication: fix 3 accepted at host level

- timestamp: 2026-11-19T05:40:00Z
  checked: follow-up boot probes, TCG, same snapshot image (sha256 30f61249…) and driver (scratchpad/probe/fumatrix.sh, one typed line per boot); kernels K0 3a1c9e5 / K1 f8c2ac9 / K2 5893ae9 / K3 7b598f2
  found: R1 `x=`{rfork f; echo hi}` (private-table bare RFFDG): K0 HANG, K1 + K3 `DIAGR hi`. R2 `x=`{rfork F; /bin/echo hi}` (private-table bare RFCFDG): K0 HANG (no prompt returns), K1 + K3 `DIAGQ done`. M2 `@{exec /bin/echo hi <[3]/n/wp/f >[4=3]}` under `ramfs -D` (one process exits holding a ramfs fid AND its dup): K1 0 Tclunks (fid 5 never clunked — the borrow drain), K2 + K3 1 Tclunk of fid 5 at the exit. P1 `@{rfork n; ramfs -D -m /n/wp; ...}` (a private namespace's last holder exits): K1 0 Tclunks, K2 + K3 `Tclunk fid 1` (the Tattach fid) at the subshell's exit. K3 also: `rfdprobe -s` `DIAGS rfdprobe: share ok`, default `DIAGD rfdprobe: ok`. Captures: scratchpad/probe/fumatrix{,2}/
  implication: boot-level revert-and-reconfirm for fix 1 (closefgrp: K0 vs K1) and for BOTH parts of fix 2 (K1 vs K2) — M2 is the observable the host tests could not reach for the one-at-a-time drain. Probe-design notes: M1 (the same dup line without `exec`) does NOT discriminate — rc opens a redirection in the PARENT rc and closes it after waiting for the child (Xpopredir), so the parent's late close fires the clunk on every kernel; and a done-regex must not match the echoed typed text (fixed with rc's `e^nd`)

- timestamp: 2026-11-19T05:55:00Z
  checked: final battery on 7b598f2 (darwin nix develop; x86_64 in amd64 docker) and composed TCG gates from scratchpad/harvey-gate (Harvey 8bad466f1 tree; HEAD 50bb1b762 is docs-only since; same verified userland copy; R9_SKIP_BUILD=1; --override-input r9 git+file://…/r9-fu)
  found: `cargo xtask check` rc=0; `cargo xtask clippy` (x86-64) rc=0, normalized diagnostics IDENTICAL to 3a1c9e5; `cargo xtask clippy --arch aarch64` rc=1 at base and final with identical aarch64/port diagnostics (4 pre-existing undocumented-unsafe errors at proc.rs:427/458/1716/2628, 26 warnings; the only diff lines are xtask lints cargo did not re-emit from cache); `cargo xtask test` rc=0 (port 232/0); `cargo test -p aarch64 --bins --target aarch64-apple-darwin` 29/29; `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` 112/112 (baseline 105). Gates: boot-r9-smoke ALL STAGES PASS; boot-r9-net-smoke ALL STAGES PASS; boot-r9-tier1 ALL STAGES PASS, 39 T1-OK (incl. writeprobe) + 23 T1-SKIP, 0 T1-FAIL; no panic lines
  implication: guardrail accepted for all three; landed by fast-forward

- timestamp: 2026-11-19T06:00:00Z
  checked: main r9 checkout fast-forwarded 3a1c9e5 → 7b598f2 (clean before and after; 5 commits ahead of origin/main: b82ee9d, 3a1c9e5, f8c2ac9, 5893ae9, 7b598f2; nothing pushed); `nix build git+file:///Users/neilskinner/code/r9#kernel-virt`
  found: /nix/store/2inqg9mw20zfb9c8300zrhiy8x5l0nqv-r9-kernel-virt-7b598f2, kernel-virt.elf sha256 92cc905bdd4392b15291849d4fe475f01d4610e0a9f8c971f8d3b557f6f45578 (== the tested K3). x86-64 kernel ELF (`cargo xtask build --arch x86-64`, default config): 817a422b… at 3a1c9e5 → ad8e19adf0c89d6f221fbddaa59f5cc66cf2ddb1667efaf981ffd6d74573c6f3 at 7b598f2 (CHANGED — fix 1 and fix 3 touch x86_64/src)
  implication: x86_64 needs its linux-host boot-iso outputHash re-derived and the x64 TCG auth + net gates in a fresh amd64 container (not run here); arm64 needs the HVF series at the pin carrying 7b598f2

- timestamp: 2026-11-19T06:30:00Z
  checked: x86_64 composed-gate venue per the coordinator (KB x64-kzero recipe) — fresh container `r9x64gate`: `docker run -d --platform linux/amd64 --privileged --cpus=6` ubuntu:24.04 (already present before this session), read-only bind mounts of the two main checkouts (/mnt/harvey, /mnt/r9) + scratchpad/x64ctr as /out, no named volumes (volume list diff empty); setup.sh: curl/xz/git/sudo/gcc/rsync, user `builder` with passwordless sudo, single-user nix 2.35.6 (sandbox = false, filter-syscalls = false, max-jobs 2, cores 3), clones ~/dbg/harvey (176723fc9) and ~/dbg/r9 (7b598f2). The rust:1-bookworm image pulled earlier for the host tests was deleted (no containers referenced it)
  found: first gates.sh attempt failed instantly (`nix: command not found` — `docker exec -u builder` leaves HOME unset, so the nix profile was never sourced); re-run with `-e HOME=/home/builder`
  implication: runner = scratchpad/x64ctr/gates.sh (kernel-x64 hash, then boot-r9-auth-x64, boot-r9-net-smoke-x64, boot-r9-tier1-x64 with `env R9_ACCEL=tcg nix run .#<app> --no-write-lock-file --override-input r9 git+file:///home/builder/dbg/r9`, full default pipeline)

- timestamp: 2026-11-19T07:30:00Z
  checked: x86_64 composed gates in the container at r9 7b598f2 (Harvey 176723fc9 clone; full default pipeline incl. the amd64 userland namespace build; explicit TCG — no /dev/kvm). Container CPU ~11% of the --cpus=6 cap during fetch/build; the 20-08 executor's HVF qemu (r9smoke-33099…) was never touched
  found: kernel /nix/store/r2qmqcz3qfm30mc7yhasgwj28fbk5gr4-r9-kernel-x64-7b598f2, kernel-x64.elf sha256 9b8ea730581d324a03b82ee8f4162f312325e1cbff0cd17225183053a44c7ca1 (pin 57e2952: 00144550f78c42a5b453f0f958863380e8bb7bcbaaa8db81657b6d2357cd1865, re-measured). boot-r9-auth-x64 rc=0 in 555 s: `smoke-r9-auth-x64: ALL STAGES PASS`, `== composed auth gate PASS`, 75 exits, 0 panicked, 0 `sys: trap`/vector 6. boot-r9-net-smoke-x64 rc=0 in 307 s: `NET-SMOKE: ALL STAGES PASS`, `smoke-r9-net-x64: ALL STAGES PASS`, `composed net gate PASS`, 74 exits, 0 panicked/trap. boot-r9-tier1-x64 rc=0 in 238 s: `tier1-r9-x64: ALL STAGES PASS` (marker parity + exact SKIP set), every expected T1-OK present (38 names), 0 T1-FAIL, 0 panicked/trap. The exit counts equal the cec2299 session's (75 / 74). Captures: scratchpad/x64ctr/
  implication: the x86_64 side of f8c2ac9 (fresh RFCFDG table + closefgrp of the old one, the release_fd_group refactor of exit_tail) and 7b598f2 (FIFO deregister) is green on all three x64 gates

- timestamp: 2026-11-19T07:45:00Z
  checked: r9's x86_64-linux `packages.boot-iso` FOD (flake.nix:822, outputHash sha256-rl+HA3pgTy+xfK7ztJ6ArQ4V9geBhXK/vJrBXw20tpQ=, wired at ff2b632) — scratchpad/x64ctr/bootiso.sh + a pin check
  found: at 7b598f2 `nix build .#boot-iso` FAILS: specified sha256-rl+HA3pg…, got sha256-Dk1zd/JQVH7PtNXrwUD+Yi3To3h1dZWIprofEutR6Yk=. Wired (uncommitted, scratch clone r9-iso) it builds: /nix/store/fizd3jb4is5q6f94slr59idxhnyfbnd7-r9-boot-iso-wip, boot.iso file sha256 a6f7051bd448db07d8a8fac9a036de43cf4394a1282a18feae31c1c3cdb84141; `nix build --rebuild` of the wired boot-iso rc=0 and of kernel-x64 rc=0 (both deterministic). AT THE PIN 57e2952 the FOD ALSO fails (got sha256-hjTC/FPLt0HBFlaX71KZu/m+0vv4evbzNlqLFA1Df5g=): cec2299 changed the x86 kernel bytes after ff2b632's wiring without a re-wire
  implication: the boot-iso hash was already stale at the current pin (since cec2299); at 7b598f2 the value to wire is sha256-Dk1zd/JQVH7PtNXrwUD+Yi3To3h1dZWIprofEutR6Yk=. Not committed (report-only per the coordinator); the r9 convention (flake.nix comment) asks for a two-independent-fresh-kernel-build proof at re-wire — here one derivation + --rebuild determinism was measured. Harvey's gates never check this FOD (they build their ISO through iso-builder at runtime), which is why they pass at both pins

- timestamp: 2026-11-19T07:50:00Z
  checked: cleanup
  found: container r9x64gate removed (`docker rm -f`); no r9* containers remain; `docker volume ls` identical to before (no volumes were created); rust:1-bookworm (pulled this session for the host tests) deleted; ubuntu:24.04 was present BEFORE this session (images-before.txt) and was kept; the 2 dangling images are 4-5 months old (not this session's) and were left
  implication: Docker state is back to its pre-session shape except for the deleted rust image

- timestamp: 2026-11-19T08:05:00Z
  checked: owner-requested r9 commit 27f38a3 `bump boot-iso hash for the 3a1c9e5..7b598f2 kernel changes` (flake.nix:822 only, rl+HA3pg… → sha256-Dk1zd/JQVH7PtNXrwUD+Yi3To3h1dZWIprofEutR6Yk=; author Neil Skinner <neil.skinner@kaseya.com>; no attribution; main 6 commits ahead of origin, unpushed). Evidence detail: the wired scratch build compiled its OWN r9-kernel-x64-wip (dirty tree → version "wip"), so two independently built kernels gave the same ISO NAR, plus --rebuild matches. Owner confirms with `nix build .#boot-iso --rebuild` on the linux host before the push
  found: `nix build git+file:///Users/neilskinner/code/r9#kernel-virt` at 27f38a3 → /nix/store/193ys32r0j6m137cljadqy2dxzc2n60a-r9-kernel-virt-27f38a3; `diff -r` against /nix/store/2inqg9mw20zfb9c8300zrhiy8x5l0nqv-r9-kernel-virt-7b598f2 is EMPTY (kernel-virt.elf sha256 92cc905bdd4392b15291849d4fe475f01d4610e0a9f8c971f8d3b557f6f45578 both, DTBs identical)
  implication: the store PATH necessarily changes with every r9 commit (pname-version carries self.shortRev and src is the whole tree), but the aarch64 kernel BYTES are unchanged — an HVF series measured on 2inqg9mw… holds for 193ys32r… byte-for-byte

## Resolution

root_cause: three follow-ups from shared-fgrp-pipe-leak. (1) The bare rfork(RFFDG/RFCFDG) split on both arches handed RFCFDG callers the exit-husk singleton (a table shared with every husk and every other RFCFDG caller, never drained) and dropped the old handle without closefgrp (a private table's chans never closed; after RFFDG every pipe end kept deep_copy's extra open count). (2) aarch64 lacked x86_64's ab0a977 + #44 exit drains: the last sharer's fd drain closed under a borrow (duplicate devmnt fids never Tclunked), a namespace's death never closed its mount table (attach fids leaked), husks kept pgrp/env/rgrp/slash/dot, and sys_exits' msg Vec rode the diverging tail. (3) sem_deregister_waiter's swap_remove reordered OTHER words' waiters in the shared SEM_WAITERS Vec (a FIFO break, and the ~1/500 parallel-test flake), both arches.
fix: r9 main (local, unpushed) f8c2ac9 (fix 1: proc::split_fd_group_in_place on both arches — fresh copy or fresh empty table, then release_fd_group on the old handle; x86_64 gains release_fd_group, exit_tail uses it), 5893ae9 (fix 2, aarch64: release_fd_group consumes one chan at a time; empty_pgrp/empty_env_group/empty_rendez_group + release_pgrp + sweep_husk_payloads called before free_user_mappings; sys_exits stack buffer), 7b598f2 (fix 3, both arches: order-preserving remove). No port/src edits.
oracle_type: specified — donor semantics (sysproc.c dupfgrp/closefgrp, pexit closefgrp/closepgrp, semwakeup FIFO); asserted as open counts → reader EOF, close counts, strong counts, singleton identity, FIFO order; plus boot-level Tclunk counts from ramfs -D (M2/P1)
verification:
  target_test: { result: pass, detail: "aarch64 29/29 (+11 across the three fixes), x86_64 112/112 (+7)" }
  mutation_check: { result: pass_with_note, mutant_killed: "fix1 3/3; fix2 release_pgrp 2/2; fix3 swap_remove 1/1 (deterministic)", survived: "fix2 borrowed-batch fd close — host-unobservable (Chan.pipe/mnt_fid are pub(crate) in port); killed at boot level instead by probe M2 (K1 0 Tclunks vs K2 1)" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: true, detail: "removed lines are moved into helpers (x86_64 exit_tail drain → release_fd_group) or replaced by the donor-correct arm; orphaned imports only" }
  adjacent_tests: { result: pass, suites_run: ["xtask check rc=0", "xtask clippy x86-64 rc=0 (identical to base)", "xtask clippy --arch aarch64 identical to base (pre-existing rc=1)", "xtask test port 232/0", "aarch64 bins 29/29 + 3000 reruns 0 fail", "x86_64 bins 112/112 + 1000 reruns 0 fail", "TCG boot-r9-smoke / boot-r9-net-smoke / boot-r9-tier1 ALL STAGES PASS (39+23)", "rfdprobe -s + default on K3"] }
  revert_and_reconfirm: { result: pass, detail: "fix1: R1/R2 hang on K0, complete on K1; fix2: M2/P1 0 Tclunks on K1, 1 on K2; fix3: swap_remove mutant fails the pin, remove passes (0/3000 vs baseline 1/500)" }
  guardrail_verdict: accepted
  byte_identity: "aarch64 kernel-virt: 3a1c9e5 2edf4f0e… → f8c2ac9 6cd70ca3… → 5893ae9 33e7ec7a… → FINAL 7b598f2 92cc905bdd4392b15291849d4fe475f01d4610e0a9f8c971f8d3b557f6f45578 (/nix/store/2inqg9mw20zfb9c8300zrhiy8x5l0nqv-r9-kernel-virt-7b598f2). x86-64 kernel CHANGED (default-config ELF 817a422b… → ad8e19ad…): the linux host's boot-iso outputHash must be re-derived, and the x64 TCG auth + net gates run in a fresh amd64 container (the old r9dbg one is gone) — not run here."
files_changed: [r9 aarch64/src/proc.rs, r9 aarch64/src/syscall.rs, r9 x86_64/src/proc.rs, r9 x86_64/src/syscall.rs]
open_items:
  - "fix 2's one-at-a-time drain has no HOST pin: a test needs a port-side accessor for a chan's shared-object refcount (e.g. a `pub fn shared_refs(&self) -> usize` on Chan, or making the devmnt fid Arc observable) — a port/src edit, so owner checkpoint first; the boot probe M2 is the current guard"
  - "M2/P1/R1/R2 are typed lines in a scratch driver, not baked gates — a Harvey-side tier-1 entry (e.g. the R1 backquote + the M2 Tclunk count via ramfs -D) would make them permanent"
  - "exec.rs's post-free kill path still holds heap locals (name/args) in a frame that diverges into exit_tail — heap-only, unmeasured"
