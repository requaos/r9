---
status: resolved
trigger: "Fix a r9 kernel defect logged by Harvey plan 20-10 (.planning/phases/20-arm64-catch-up-to-v2-1-parity/deferred-items.md): on both aarch64 and x86_64, create(..., OCEXEC) strips the OCEXEC bit, so a newly created file stays open across exec. open(..., OCEXEC) is handled; 20-10 ported aarch64's close-on-exec sweep in r9 1125a20."
created: 2026-11-20T12:40:00Z
updated: 2026-11-20T18:00:00Z
---

## Current Focus

bug_class: Bohrbug (deterministic: RED on every run, HVF and TCG identical; no timing involved)
hypothesis: H1 (RED-confirmed) — both arches' sys_create pass `mode & !(OEXCL|OCEXEC)` to create_chan_union (correct: the donor never hands OCEXEC to the device) but never set `cexec` on the returned chan afterwards (the donor chan.c:1516 `if(omode & OCEXEC) cnew->flag |= CCEXEC`), so exec's take_cexec sweep skips a created OCEXEC fd
test: ALL DONE — probe RED→GREEN HVF+TCG; host tests both arches + mutants; aarch64 TCG smoke/net-smoke/tier1; x64 TCG auth/net-smoke/tier1; darwin + x86_64-linux xtask batteries; boot-iso FOD re-derived and green at the committed HEAD; container removed
expecting: owner confirmation; then the owner's push of r9 437b752 + 42203d8, the lock-only pin move, and the combined arm64 HVF series (kernel-virt bytes change: 3e86d2f1… → d8be7228…)
next_action: DONE — owner confirmed fixed (relayed by the coordinator); r9 pushed to 42203d8; session archived to resolved/ with the KB entry. The tier-1 wiring decision went to plan 20-11 (`T1-OK cexeccreate`, added by its executor). The ORCLOSE gap continues in .planning/debug/orclose-local-devices.md
r9_commits: 437b752 fix (tree == tested d961678), 42203d8 boot-iso hash bump — pushed by the coordinator (origin/main 42203d8)
harvey_commits: 3592699f0 test(debug): cexecprobe -c (build/aarch64/cexecprobe.c only); this file (docs(debug) commit)
reasoning_checkpoint:
  hypothesis: "sys_create's new-file arm (both arches) strips OCEXEC before the device create — correct, donor-faithful — but never records it on the created chan afterwards, so Chan::cexec stays false and exec's take_cexec sweep leaves the fd in the table; the existing-file arm goes through namec::open_chan, which does record it"
  confirming_evidence:
    - "reading: aarch64/src/syscall.rs:1505-1512 and x86_64/src/syscall.rs:1477-1484 call create_chan_union(.., mode & !(OEXCL|OCEXEC), ..) and use the result as-is; no creator (devenv/devsrv/devmnt) or create_chan sets cexec"
    - "RED probe, both accels at the pin kernel: `cexecprobe: LEAK create fd=3` for the OEXCL|OCEXEC new-file create, while the re-create of the same name OCEXEC (existing-file arm, fd 5) IS closed and the plain create (fd 4) survives — exactly the arm split H1 predicts"
    - "donor sys/src/9k/port/chan.c:1514-1517 sets CCEXEC on cnew after the device create; port/src/dat.rs:139-142 already documents sys_create as a recorder of the mark"
  falsification_test: "if the fixed kernel (mark set on the created chan) still prints `LEAK create fd=3`, the loss is downstream of sys_create (fd install clone, the sweep) and H1 is wrong; likewise if the host test of the helper + take_cexec passes WITHOUT the mark line"
  fix_rationale: "the defect is the missing record of fd state on one of create's two arms; setting cexec on the created chan (in a helper that also owns the strip, so the device never sees the bit and the mark is derived from the caller's unstripped mode) restores the donor's Acreate exactly; the sweep, fd table and port namec are already correct and stay untouched"
  blind_spots: "aarch64 sys_create itself can't be host-driven (new_user_proc needs the live kernel table) — its call-site wiring is guarded by the boot probe only, the helper by host tests; x86_64 gets a dispatch-level host test through MockStack; devmnt creates (ramfs /tmp) not exercised at boot by the probe (devenv only) — same code path in sys_create; ORCLOSE (remove-on-close) is unimplemented for local devices on both arches — separate"
  candidate_causes:
    - "code: sys_create new-file arm never sets cexec (H1, both arches) — RED evidence matches"
    - "code: the fd install or Chan::dup drops cexec (fd_install_lowest / ref_clone) — read: Chan::dup copies cexec (dat.rs:250, 277), and the existing-file arm's marked chan survives install to be swept, so install keeps the flag"
    - "data: the probe's name already existed so the create took the other arm — ruled out: fresh rfork(RFCENVG) group + OEXCL forces the new-file arm (an existing name would have failed the create)"
    - "environment: accelerator — HVF and TCG identical"
  and_gate: "no — one code defect on the new-file arm; the trigger needs create of a NEW name with OCEXEC plus a later exec, which no gate's create did (tier-1 creates are plain)"
probe_design: default (no args) run UNCHANGED so the tier-1 gate at the 1125a20 pin keeps its T1-OK cexecprobe; the create leg is opt-in `-c` (typed by hand, like brkprobe). Wiring `-c` into tier1-sweep.rc is a checkpoint decision (gate goes red at any pin without the fix)
scratch_paths: scratchpad = /private/tmp/claude-501/-Users-neilskinner-code-harvey/55ded085-3397-4d2c-931d-1d3a7a81443a/scratchpad; probe build = scratchpad/cx/build; driver = scratchpad/probe/{probe.sh,p20-07-probe.expect}

## Symptoms

expected: a file created with create(path, mode|OCEXEC, perm) is closed by a successful exec (donor chan.c Acreate: CCEXEC set on the new chan; sysexec fdclose(i, CCEXEC))
actual: the created chan's cexec stays false, so the fd survives exec (reading: aarch64/src/syscall.rs:1508, x86_64/src/syscall.rs:1480 strip OCEXEC and nothing re-marks it)
errors: none (silent fd leak across exec)
reproduction: create a new file with OCEXEC, exec, fstat the fd in the new image — it is still open
started: since sys_create was written (both arches); the existing-file arm (open_chan(existing, mode|OTRUNC)) marks correctly since open_chan learned OCEXEC (19-10 cont 4 x86_64; 20-10 1125a20 added aarch64's sweep)

## Eliminated

- hypothesis: H2 — the created chan's mark is lost downstream of sys_create (fd_install_lowest / Chan::dup / the exec sweep)
  evidence: the fixed kernel (only the mark added inside sys_create's new-file arm) prints `cexecprobe: create ok` on HVF and TCG; Chan::dup copies cexec (port/src/dat.rs:250, 277); the existing-file arm's marked chan was already swept at the pin
  timestamp: 2026-11-20T13:50:00Z

- hypothesis: data — the probe's create took the existing-file arm (stale name)
  evidence: fresh rfork(RFCENVG) group + OEXCL; RED printed LEAK for the OEXCL create only, while the re-create (existing arm) was closed
  timestamp: 2026-11-20T13:05:00Z

- hypothesis: environment — accelerator-specific behaviour
  evidence: HVF and TCG identical in RED and GREEN; open-leg logs byte-identical across kernels
  timestamp: 2026-11-20T13:50:00Z

## Evidence

- timestamp: 2026-11-20T00:00:00Z
  checked: .planning/debug/knowledge-base.md (3 entries: G-05-1, x64-kzero, shared-fgrp-pipe-leak)
  found: no symptom match (create / OCEXEC / exec fd leak not present). Method notes taken: x86_64 host tests must be run explicitly (`cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu`) in an amd64 container; x64 gates via Docker Desktop amd64 ubuntu:24.04 --privileged, single-user nix sandbox=false + filter-syscalls=false, non-root builder, gcc installed; aarch64 gates from a scratch Harvey clone with R9_SKIP_BUILD=1 and --override-input r9
  implication: no known-pattern candidate; investigate fresh

- timestamp: 2026-11-20T00:00:00Z
  checked: Harvey donor sys/src/9k/port/chan.c:1514-1517 (Acreate) and :1408-1411 (Aopen); sysproc.c:522
  found: create: `cnew->dev->create(cnew, name, omode&~(OEXCL|OCEXEC), perm); ... if(omode & OCEXEC) cnew->flag |= CCEXEC;` — open: `c->dev->open(c, omode&~OCEXEC); if(omode & OCEXEC) c->flag |= CCEXEC;`; sysexec `fdclose(i, CCEXEC)`
  implication: the contract is specified — the create arm must record CCEXEC after the device create, exactly as open does

- timestamp: 2026-11-20T00:00:00Z
  checked: r9 1125a20 aarch64/src/syscall.rs:1410-1530 and x86_64/src/syscall.rs:1396-1497 (sys_create), port/src/namec.rs:407-445 (open_chan) and :475-537 (create_chan/create_chan_union), port/src/dat.rs:139-151 (Chan::cexec doc)
  found: both sys_create bodies are the same shape. Existing-file arm: `open_chan(&existing, mode | OTRUNC)` — open_chan sets `opened.cexec = true` when mode has OCEXEC (marked). Create arm: `create_chan_union(&parent, name, mode & !(OEXCL|OCEXEC), perm, ..)` — create_chan only sets copen; nothing sets cexec. port/src/dat.rs:141 already documents the contract as "Recorded by namec::open_chan and sys_create when the open mode carries OCEXEC" and `cexec` is `pub`
  implication: H1 matches by reading; the fix belongs in the arch sys_create (as the port doc already says), no port/src change needed

- timestamp: 2026-11-20T12:45:00Z
  checked: port/src/devenv.rs:418 create, port/src/devsrv.rs:121 create, devmnt create; grep cexec in all three
  found: no creator touches cexec (devenv builds Chan::new(.., mode & OMASK); devsrv Chan::new(.., OWRITE)); create_chan only sets copen. Real Harvey consumers of create(..OCEXEC): lib9p/srv.c:839 postfd, rio/fsys.c:95, plumb/fsys.c:209 (all `/srv/<name>` OWRITE|ORCLOSE|OCEXEC), sam/acme disk.c, imap4d
  implication: every create(OCEXEC) on r9 leaks its fd into exec'd children — e.g. a lib9p server's /srv post fd rides into anything it execs

- timestamp: 2026-11-20T12:50:00Z
  checked: probe venue — aarch64 REMOVE(25) is loud-None (syscall.rs:112), so the probe cannot clean up; bare rfork(RFCENVG) is implemented on both arches (proc::rfork_env_split); devenv stat resolves through the caller's env group (kept across exec)
  found: a probe that does rfork(RFCENVG) first gets a fresh, private, empty /env: OEXCL creates there are guaranteed new-file-arm and vanish with the process
  implication: venue = /env under a fresh env group; no leftovers in rc's env

- timestamp: 2026-11-20T12:55:00Z
  checked: darwin host 7c/7l (.build/hostbin-darwin-aarch64/bin) building the committed cexecprobe.c: `7c -FTVw -I$H/arm64/include -I$H/sys/include` + `7l -l -E _main -o .. cexecprobe.7 $H/arm64/lib/libc.a` (7l hardcodes /arm64/lib for #pragma lib; -l skips it, -E keeps _main referenced)
  found: a.out header text/data/bss/entry identical to the executor's guest build (out/cexecprobe sha256 6dcb19d2…); text+data segments byte-IDENTICAL (39800 bytes); only the symbol table differs (+176 B: host include paths in the file history)
  implication: host-compiled probe binaries are faithful — the probe can be built without the lima venue

- timestamp: 2026-11-20T13:05:00Z
  checked: RED matrix — create-leg cexecprobe (scratchpad/cx/build/cexecprobe.c, binary sha256 28ace06f…) baked into a scratch image from a Harvey clone at 699f3dcf9 (scratchpad/harvey-cx; arm64/bin, arm64/lib, build/aarch64/out, .build/bin/mkpaqfs copied from the main checkout, diff -r identical; only out/cexecprobe swapped), mkimage.sh PASS, bootfs sha256 b0fcb62a…; kernel = the pin's /nix/store/j0d1q06r…-r9-kernel-virt-1125a20 (ELF sha256 3e86d2f1…); driver scratchpad/cx/matrix.sh → scratchpad/probe/probe.sh; TCG cortex-a72 and HVF -cpu host, virt,gic-version=3
  found: |
    MATRIX unfixed tcg c lines=`cexecprobe: LEAK create fd=3` status=`CXST leak` (process 6 exited: "leak")
    MATRIX unfixed tcg o lines=`cexecprobe: ok` status=`CXOT` (nil)
    MATRIX unfixed hvf c lines=`cexecprobe: LEAK create fd=3` status=`CXST leak`
    MATRIX unfixed hvf o lines=`cexecprobe: ok` status=`CXOT`
    The create-existing arm (fd 5) and the plain create (fd 4) are correct on both accels. Captures scratchpad/cx/matrix/unfixed-{tcg,hvf}-{c,o}.{log,out} (log sha256 prefixes 19a75f01, 0209be5b, 1a07d4e6, 119aaa96)
  implication: H1's prediction met exactly — only the new-file arm loses the mark; the existing-file arm (open_chan) and the open path are fine. Deterministic on both accels (Bohrbug)

- timestamp: 2026-11-20T13:10:00Z
  checked: r9 main checkout HEAD before cloning the fix base
  found: main moved 1125a20 → 7d464ad `ci(20-11): build aarch64 virtchurn in CI` (another session; .github/workflows/xtask.yml only, +12 lines; unpushed — origin/main still 1125a20)
  implication: fix base = 7d464ad (main HEAD); no kernel source differs from the pin, so the RED kernel (1125a20) is the unfixed base kernel
- timestamp: 2026-11-20T13:12:00Z
  checked: r9 ORCLOSE handling (grep ORCLOSE/CRCLOSE in aarch64/x86_64/port)
  found: ORCLOSE only rides the devmnt wire (devmnt.rs:879/1077); no remove-on-close for local creators (devsrv/devenv); REMOVE(25) loud-None on aarch64
  implication: related, separate gap for the live consumers' OWRITE|ORCLOSE|OCEXEC /srv posts — reported, not fixed here

- timestamp: 2026-11-20T13:30:00Z
  checked: fix in scratch clone scratchpad/r9-cx, WIP commit d961678 on 7d464ad — `create_new_chan(parent, name, mode, perm, pg, nomount)` in aarch64/src/syscall.rs and x86_64/src/syscall.rs (strip OEXCL|OCEXEC for the device, then `c.cexec = true` when the caller's mode has OCEXEC); sys_create's new-file arm calls it; x86_64 exec::close_on_exec_fds made pub(crate) for a host test. Host tests: aarch64 syscall::create_cexec_tests (2), x86_64 syscall::tests (3: the same two + a CREATE dispatch through MockStack followed by the real exec sweep)
  found: darwin `nix develop -c cargo test -p aarch64 --bins --target aarch64-apple-darwin` 31 passed / 0 failed (29 at the base per 20-10 + 2 new)
  implication: aarch64 host leg green with the fix

- timestamp: 2026-11-20T13:35:00Z
  checked: manual mutation seeding at the fix site, aarch64 (scratchpad/cx/mutants.py; `-- create_cexec_tests`)
  found: M0 mark removed (= the original defect, RED) KILLED 2/2 tests failed; M1 mark from the stripped mode KILLED; M2 mark unconditional KILLED; M3 mark from OEXCL KILLED; M4 sys_create bypasses the helper SURVIVED (aarch64 sys_create cannot be host-driven — new_user_proc needs the live kernel table); tree restored byte-identical
  implication: the helper tests assert the root cause; the aarch64 call-site wiring is guarded by the boot probe (next entry) — on x86_64 the dispatch test covers M4

- timestamp: 2026-11-20T13:50:00Z
  checked: GREEN matrix — fixed kernel /nix/store/z45cmm4kqzhv6qiwq3g1pqyzl8196vv7-r9-kernel-virt-d961678 (ELF sha256 d8be7228d31cc5d398d530a09196662402170bcec8dc12aa80bbbbec4bd57cb4, 60600128 bytes), same image b0fcb62a…, same driver
  found: |
    MATRIX fixed tcg c `cexecprobe: create ok` status=`CXST` (nil)
    MATRIX fixed tcg o `cexecprobe: ok` status nil
    MATRIX fixed hvf c `cexecprobe: create ok` status nil
    MATRIX fixed hvf o `cexecprobe: ok` status nil
    open-leg logs byte-identical to the unfixed runs (sha256 prefixes 0209be5b tcg, 119aaa96 hvf); create-leg logs 33f84af6 (tcg), f83f3ad8 (hvf)
  implication: H1 CONFIRMED end to end on both accelerators: red at the pin kernel, green with only the mark added; the open path is untouched

- timestamp: 2026-11-20T14:05:00Z
  checked: x86_64 host leg in the amd64 container (Docker Desktop linux/amd64 Rosetta, ubuntu:24.04 [pre-existing image, not pulled], --privileged, single-user nix 2.36.0 sandbox=false filter-syscalls=false, builder user, gcc; container cx-x64-ocexec; clones of scratchpad/harvey-cx and scratchpad/r9-cx); `nix develop -c cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` on d961678; mutants `-- ocexec`
  found: 115 passed / 0 failed incl. syscall::tests::{created_ocexec_fd_is_taken_by_the_exec_sweep, create_records_ocexec_beside_the_other_create_bits, create_syscall_ocexec_fd_is_closed_by_exec}. Mutants: M0 mark removed (the original defect) KILLED 3/3; M1 stripped-mode KILLED 3/3; M2 unconditional KILLED 3/3; M3 OEXCL KILLED 3/3; M4 sys_create bypasses the helper KILLED (by the dispatch test); tree restored (dirty=0)
  implication: x86_64 RED (M0) → GREEN, and on x86_64 the real CREATE handler + real exec sweep are pinned end to end

- timestamp: 2026-11-20T14:10:00Z
  checked: aarch64 composed gates, TCG, from scratchpad/harvey-cx (699f3dcf9; never the main checkout or the lima venue) via scratchpad/cx/a64gates.sh: `HARVEY=<clone> R9_SKIP_BUILD=1 R9_ACCEL=tcg nix run .#<app> --no-write-lock-file --override-input r9 git+file://<scratchpad>/r9-cx`; userland = the byte-verified copy + the create-leg cexecprobe; bake ran as defined (bootfs sha256 46011dfd…)
  found: boot-r9-smoke rc=0 `smoke-r9: ALL STAGES PASS`; boot-r9-net-smoke rc=0 `smoke-r9-net: ALL STAGES PASS`; boot-r9-tier1 rc=0 `tier1-r9: ALL STAGES PASS`, 41 T1-OK + 23 T1-SKIP (the expected set) incl. `cexecprobe: ok` / `T1-OK cexecprobe`; qemu line `-accel tcg -cpu cortex-a72`, kernel z45cmm4k…-r9-kernel-virt-d961678. Captures scratchpad/cx/gates/
  implication: no regression in the aarch64 composed gates; the default cexecprobe run (open leg) is unchanged by the probe extension

- timestamp: 2026-11-20T14:15:00Z
  checked: darwin r9 battery (scratchpad/cx/battery.sh) on the fix (d961678) and a fresh base clone (7d464ad)
  found: |
    check rc=0 (63 warnings both); clippy x86-64 rc=0 (82 both); xtask test rc=0 (same per-leg pass counts both; arch leg skipped — no rustup in the devShell); virtshell build rc=0 (37 warnings both); `cargo test -p aarch64 --bins --target aarch64-apple-darwin` 29 → 31
    clippy --arch aarch64 rc=1 at BOTH with the same 4 pre-existing `unsafe block missing a safety comment` errors in aarch64/src/proc.rs; normalized diagnostic diff = only xtask/src/config.rs warnings replayed in the fresh base clone (xtask untouched) — no aarch64 diagnostic added or removed
    rustfmt --check drift on the three touched files: 130 lines at base and fix, identical (no new drift; CI runs no fmt)
  implication: battery green relative to base; no new lint

- timestamp: 2026-11-20T16:40:00Z
  checked: amd64 container — base x86_64 host tests (r9 7d464ad clone) and `nix build #kernel-x64` at base and fix
  found: base 112 passed / 0 failed (fix 115: +3 new). kernel-x64 base /nix/store/6jdryiasnwpzjk6pjnq6pc1kc192qm9a-r9-kernel-x64-7d464ad ELF sha256 9b8ea730581d324a03b82ee8f4162f312325e1cbff0cd17225183053a44c7ca1 (25782176 B — the bytes r9 27f38a3 wired the current boot-iso hash for); fix /nix/store/631xh7j5n3gq06y6byxnvfpd9asnb06h-r9-kernel-x64-d961678 sha256 bdab182989cb338651354dd41e2e0c729a0f3a25275dcce6a03a83d3eb999eff (25783728 B)
  implication: the x86_64 kernel bytes change (code + panic-location line shifts in syscall.rs), so the boot-iso FOD outputHash must be re-derived and the x64 gates run
- timestamp: 2026-11-20T16:40:00Z
  checked: Harvey main checkout and r9 main while the gates ran
  found: Harvey moved to c3e601f89 `docs(20-11): r9 7d464ad pushed (owner option 3); pin held for the OCEXEC fix`; r9 origin/main = local main = 7d464ad
  implication: the fix commits go on 7d464ad (still main HEAD); the owner plans one lock-only pin move + one combined HVF series after this fix
- timestamp: 2026-11-20T16:45:00Z
  checked: darwin `nix build git+file://<scratchpad>/r9-cxbase#kernel-virt` (base 7d464ad)
  found: /nix/store/w2zh2d8h46x09gyvpz461av3rky92zw2-r9-kernel-virt-7d464ad ELF sha256 3e86d2f12a7fe0df6640b75726abd0232fba99576ac3336fcbeea52fc88955c6 = the 1125a20 RED kernel
  implication: RED and GREEN differ by exactly the fix (base bytes = RED bytes)

- timestamp: 2026-11-20T16:55:00Z
  checked: x64 composed gates on the fix, amd64 container (scratchpad/cx/x64/gates.sh), Harvey 699f3dcf9 clone, full default pipeline (amd64 userland built in-container), `env R9_ACCEL=tcg nix run .#<app> --no-write-lock-file --override-input r9 git+file:///home/builder/dbg/r9` (r9 d961678); every gate's `r9 kernel package` = /nix/store/631xh7j5n3gq06y6byxnvfpd9asnb06h-r9-kernel-x64-d961678
  found: boot-r9-auth-x64 rc=0 secs=479 `smoke-r9-auth-x64: ALL STAGES PASS` / `== composed auth gate PASS`; boot-r9-net-smoke-x64 rc=0 secs=291 `smoke-r9-net-x64: ALL STAGES PASS`; boot-r9-tier1-x64 rc=0 secs=217 `tier1-r9-x64: ALL STAGES PASS`, 38 T1-OK + 0 T1-SKIP = `expected on r9: 38 T1-OK markers + 0 T1-SKIP markers (computed)`, 0 T1-FAIL, q35 `-accel tcg -cpu qemu64`. Captures scratchpad/cx/x64out/boot-r9-*-x64.*
  implication: no x86_64 regression in the composed gates

- timestamp: 2026-11-20T17:00:00Z
  checked: boot-iso FOD re-derivation on d961678 (same container): `nix build .#boot-iso` → mismatch got-value; a clone with it wired; `--rebuild` of the wired boot-iso and of kernel-x64; then `cargo xtask check/clippy/test` on d961678 (CI's x86_64-linux shape)
  found: `specified: sha256-Dk1zd/JQVH7PtNXrwUD+Yi3To3h1dZWIprofEutR6Yk=  got: sha256-L8Sm1tGU6KOyb3+XkzddGeqKe993L/akNm5xaqLjmV8=`; wired build green, boot.iso file sha256 b1608bd4da47cbf61602bc5549fcd490e8a479d1443521c058c37f68d69e3e55; boot-iso --rebuild rc=0, kernel-x64 --rebuild rc=0 (no `not deterministic`); xtask check rc=0 (63 warnings), clippy rc=0 (82), test rc=0 (23; same leg counts as darwin)
  implication: the new FOD value is reproducible; x86_64-linux battery green

- timestamp: 2026-11-20T17:10:00Z
  checked: commits on the main r9 checkout (HEAD was 7d464ad = origin/main, clean): patch 7d464ad..d961678 applied with `git apply --index`, committed; then the outputHash line only (the 27f38a3 shape)
  found: 437b752 `fix: create(OCEXEC) marks the new chan close-on-exec on both arches` — tree 5837cc7287a86b79b02184ac5cd454a3bd141949 == the tested d961678 tree; 42203d8 `bump boot-iso hash for the create(OCEXEC) kernel change` (flake.nix 1 line). Unpushed (origin/main 7d464ad). No AI attribution in either message
  implication: the committed code is byte-for-byte the tested code

- timestamp: 2026-11-20T17:20:00Z
  checked: the committed HEAD 42203d8 — darwin `nix build git+file:///Users/neilskinner/code/r9?rev=42203d8…#kernel-virt`; container (branch `final` fetched into the scratch clone) `#kernel-x64` and an UNMODIFIED `#boot-iso` + `--rebuild`
  found: kernel-virt /nix/store/sw5pfm8m3i83dqw4kxcp80452z39h1ii-r9-kernel-virt-42203d8 sha256 d8be7228d31cc5d398d530a09196662402170bcec8dc12aa80bbbbec4bd57cb4 (60600128 B) = tested; kernel-x64 /nix/store/wx5m4w7yg78vv1lin2swlsyxc9w936dj-r9-kernel-x64-42203d8 sha256 bdab182989cb338651354dd41e2e0c729a0f3a25275dcce6a03a83d3eb999eff (25783728 B) = tested; boot-iso rc=0 with the committed hash, /nix/store/zpr09qbcm73xk2a086fnjx8kk8kdip21-r9-boot-iso-42203d8, boot.iso sha256 b1608bd4… (= wired test), --rebuild rc=0
  implication: every gate result above holds for the committed HEAD's bytes on both arches; the boot-iso FOD is green at HEAD

- timestamp: 2026-11-20T17:25:00Z
  checked: cleanup — `docker rm -f cx-x64-ocexec`; image / volume / container lists vs the pre-session snapshots (scratchpad/cx/{images,volumes,containers}-{before,after}.txt); qemu processes
  found: all three lists identical to before (ubuntu:24.04 pre-existed — not pulled, not removed); 0 qemu processes; no executor process touched (its lima VM and the main Harvey checkout's build outputs untouched — only read/copied)
  implication: venue clean

## Resolution

root_cause: "r9 sys_create's new-file arm (aarch64/src/syscall.rs and x86_64/src/syscall.rs) handed create_chan_union `mode & !(OEXCL | OCEXEC)` — correct, the donor never passes either bit to the device — but never recorded OCEXEC on the created chan afterwards (the donor chan.c Acreate's `if(omode & OCEXEC) cnew->flag |= CCEXEC`). Chan::cexec stayed false, so exec's close-on-exec sweep (FdGroup::take_cexec via close_on_exec_fds) left the fd in the table and a file created OCEXEC stayed open across exec. The existing-file arm (open_chan(existing, mode|OTRUNC)) already recorded the bit. Both arches; arch code only."
fix: "r9 437b752 (local, unpushed, on main 7d464ad) `fix: create(OCEXEC) marks the new chan close-on-exec on both arches` — new `create_new_chan(parent, name, mode, perm, pg, nomount)` in both arches' syscall.rs strips OEXCL|OCEXEC for the device and sets `c.cexec = true` when the caller's mode carries OCEXEC; sys_create's new-file arm calls it. x86_64 exec::close_on_exec_fds → pub(crate) (test access only). Host tests: aarch64 syscall::create_cexec_tests (2), x86_64 syscall::tests (3, incl. a CREATE dispatched through MockStack + the real exec sweep). port/src untouched. Then r9 42203d8 `bump boot-iso hash for the create(OCEXEC) kernel change` (outputHash sha256-L8Sm1tGU6KOyb3+XkzddGeqKe993L/akNm5xaqLjmV8=). Harvey 3592699f0 adds the opt-in `cexecprobe -c` create leg (default run unchanged)."
oracle_type: "specified — the donor Acreate contract: a create whose mode carries OCEXEC yields an fd that a successful exec closes, on both create arms, and a plain create's fd survives. Asserted on host as `chan.cexec` + take_cexec/close_on_exec_fds slot state, and at boot as `dirfstat` in the exec'd image. Boundary neighbours: plain create (must survive), OEXCL|OCEXEC (new-file arm forced), the existing-file re-create OCEXEC (other arm), lib9p postfd's OWRITE|ORCLOSE|OCEXEC, OEXCL alone (no mark)."
verification:
  target_test: { result: pass, detail: "boot: `cexecprobe -c` RED `LEAK create fd=3` at base bytes 3e86d2f1 (HVF+TCG) → GREEN `create ok` at d8be7228 (HVF+TCG); host: aarch64 31/31 (29+2), x86_64 115/115 (112+3)" }
  mutation_check: { result: pass, reason_if_skipped: "no cargo-mutants in the devShell — MANUAL seeding (scratchpad/cx/mutants.py)", mutant_killed: "aarch64: M0 mark-removed (the defect), M1 stripped-mode, M2 unconditional, M3 OEXCL — 4/4 at the fix site; M4 sys_create-bypasses-helper survives the host tests (aarch64 sys_create not host-drivable) and is killed by the boot probe (the base kernel IS that behaviour: RED). x86_64: M0–M4 5/5 killed (M4 by the dispatch test)" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: n/a, detail: "adds behaviour (the mark); the 15/9 removed lines in sys_create are the inline create_chan_union call MOVED into the helper; +194/-25 incl. tests" }
  adjacent_tests: { result: pass, suites_run: ["darwin cargo xtask check/clippy/test rc=0, same warning counts as base 7d464ad", "darwin clippy --arch aarch64 rc=1 at base AND fix with the identical aarch64 diagnostic set (4 pre-existing undocumented-unsafe errors in proc.rs)", "darwin virtshell build rc=0", "cargo test -p aarch64 --bins (darwin) 31/31", "amd64 container: cargo test -p x86_64 --bins 115/115; cargo xtask check/clippy/test rc=0", "aarch64 TCG boot-r9-smoke / boot-r9-net-smoke / boot-r9-tier1 ALL STAGES PASS (41 T1-OK + 23 T1-SKIP)", "x64 TCG boot-r9-auth-x64 / boot-r9-net-smoke-x64 / boot-r9-tier1-x64 ALL STAGES PASS (38 T1-OK + 0 SKIP)", "rustfmt drift unchanged (130 lines base and fix)"], not_run: "aarch64 HVF composed series (owner runs it separately); a boot-level create probe on x86_64 (amd64 cexecprobe not built — the x86_64 dispatch-level host test covers the real handler + sweep)" }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true, detail: "revert = the base 7d464ad kernel (bytes 3e86d2f1… = 1125a20): LEAK create fd=3 on HVF and TCG; reapply = d961678 / committed 42203d8 (bytes d8be7228…): create ok on HVF and TCG; host M0 (mark removed) red on both arches, restored green" }
  guardrail_verdict: accepted
kernel_bytes:
  aarch64_kernel_virt: "42203d8 /nix/store/sw5pfm8m3i83dqw4kxcp80452z39h1ii-r9-kernel-virt-42203d8 sha256 d8be7228d31cc5d398d530a09196662402170bcec8dc12aa80bbbbec4bd57cb4 (60600128 B); pin 1125a20 / 7d464ad: 3e86d2f12a7fe0df6640b75726abd0232fba99576ac3336fcbeea52fc88955c6 — CHANGES, needs the HVF series"
  x86_64_kernel_x64: "42203d8 /nix/store/wx5m4w7yg78vv1lin2swlsyxc9w936dj-r9-kernel-x64-42203d8 sha256 bdab182989cb338651354dd41e2e0c729a0f3a25275dcce6a03a83d3eb999eff (25783728 B); 7d464ad: 9b8ea730581d324a03b82ee8f4162f312325e1cbff0cd17225183053a44c7ca1 (25782176 B) — CHANGES; x64 TCG gates run here"
  boot_iso: "outputHash sha256-L8Sm1tGU6KOyb3+XkzddGeqKe993L/akNm5xaqLjmV8= (was sha256-Dk1zd/JQVH7PtNXrwUD+Yi3To3h1dZWIprofEutR6Yk=); boot.iso file sha256 b1608bd4da47cbf61602bc5549fcd490e8a479d1443521c058c37f68d69e3e55; green at 42203d8 with --rebuild"
files_changed: [r9 aarch64/src/syscall.rs, r9 x86_64/src/syscall.rs, r9 x86_64/src/exec.rs, r9 flake.nix (hash bump), Harvey build/aarch64/cexecprobe.c]
related_findings_not_fixed:
  - "ORCLOSE (remove-on-close) has no local-device implementation on either arch (it only rides the devmnt wire); REMOVE(25) is loud-None on aarch64. The live create(OCEXEC) callers (lib9p postfd, rio, plumber) pass OWRITE|ORCLOSE|OCEXEC, so their /srv posts outlive the server even with this fix."
  - "tier-1 wiring of the create leg is OPEN (owner decision): the default `cexecprobe` run is unchanged; adding `/bin/cexecprobe -c` to tier1-sweep.rc (new T1 name or folded into T1-OK cexecprobe) makes the gate red at any pin without 437b752 — not done (checkpoint rule)."

## Prevention

human_verification: owner confirmed fixed (2026-11-20, relayed by the coordinator); r9 pushed to 42203d8

five_whys (branched, from the candidate_causes):
  - code: why did a created OCEXEC fd survive exec? its chan's cexec was false → why? sys_create's new-file arm stripped OCEXEC for the device and never re-recorded it → why was that possible? fd state lives in the same mode word the device receives, and the strip and the record are two separate steps in the donor (Acreate) — the port of create kept the strip and dropped the record, with no single place that owned both → actionable: one helper (create_new_chan) owns the strip AND the mark, so neither can be ported without the other
  - process/cross-arm: why did open have the mark and create not? 19-10 cont 4 taught open_chan OCEXEC because its live consumer (libthread procexec) opens, not creates; the fix followed the measured consumer, and the sibling arm (create) was not audited at the same time → actionable: when an fd-state bit is taught to one arm, grep every arm that yields a chan (open, create-new, create-existing)
  - test/environment: why no failing gate? 20-10's cexecprobe pinned the open leg only, the tier-1 sweep's creates are plain, port's namec test covers open_chan only, and sys_create had no host test on either arch (aarch64's is not host-drivable)

why_not_caught: no gate existed for this class — 20-10's cexecprobe pinned only the open arm (libthread procexec's shape), the tier-1 sweep's creates are all plain, the port-side test (namec_tests::open_chan_records_oceexec_as_fd_state) covers open_chan only, and sys_create had no host test on either arch

recurrence_guard: r9 42203d8 host tests `aarch64/src/syscall.rs:create_cexec_tests::{created_ocexec_fd_is_taken_by_the_exec_sweep, create_records_ocexec_beside_the_other_create_bits}` and `x86_64/src/syscall.rs:tests::{created_ocexec_fd_is_taken_by_the_exec_sweep, create_records_ocexec_beside_the_other_create_bits, create_syscall_ocexec_fd_is_closed_by_exec}` (verified passing; they kill the mark-removed mutant, and the x86_64 dispatch test kills a helper bypass); Harvey `cexecprobe -c` boot reproducer (build/aarch64/cexecprobe.c, `cexecprobe: create ok` required; plan 20-11 wires it as `T1-OK cexeccreate`); KB pattern "an fd-state mode bit must be recorded on every arm that yields a chan"
