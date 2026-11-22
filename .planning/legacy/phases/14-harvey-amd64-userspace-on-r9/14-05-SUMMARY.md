---
phase: 14-harvey-amd64-userspace-on-r9
plan: 05
subsystem: exec-runtime-gate
tags: [exec, mb2-module, grub, paqfs, marker, x86_64, r9, q35exec, xexec-02]
duration: 108 min
completed: 2026-11-08
requires:
  - "14-01..04-complete r9 HEAD abee15c (the frozen ABI, the entry trio, usermem + the amd64 loader, proc machinery + the measured-set dispatch)"
provides:
  - "the D-03 delivery leg: mb2 TAG_MODULE=3 parse (per-module collection, checked gets, named errors, the memory-map bounds check at walk completion) + the templated module2 grub staging (kernel-only emission byte-identical to Phase 13's — the FOD-loader-config component preserved) + the [dist] module key with harvey:-prefix resolution + the q35exec profile"
  - "the D-04/D-09 Harvey x64 bake: build/x86_64/{smokesleep.c,stage-build.sh,mkpaqfs-host.sh,mkimage.sh,root-tree/} — the amd64 marker (notify/alarm/sleep/noted via the bp path), S_MAGIC 00008a97 asserted on real 6.out bytes (A1), bootfs.paq in the carried-verbatim format (header 529ab12b / trailer 6b46e688, KERNDATE-pinned, double-bake byte-identical)"
  - "the D-08 exec proof: x86_64 exec_bootstrap (IRQ-masked, l.S module stash, devroot/paqfs mount, hook registrations, BOOT_ENTRY /amd64/init, CR3 switch, trap_return handoff) — the q35exec gate green twice: the marker's OWN WRITE-syscall prints from ring 3 on serial"
  - "the KSTACK_VA canonical-address fix, the l.S linked-VA entry fix, the loud panic handler — three latent x86_64 kernel defects the first ring-3 exec exposed, all fixed with the full Phase-13 regression battery re-green"
affects:
  - "../r9/x86_64/src/{main,l.S,proc,runtime}.rs, ../r9/x86_64/src/platform/q35/mb2.rs, ../r9/x86_64/lib/{grub.cfg,config_q35exec.toml}, ../r9/xtask/src/{main,config}.rs, ../r9/flake.nix"
  - "build/x86_64/* (Harvey), sys/src/cmd/paqfs/mkpaqfs.c, sys/src/cmd/{7a,7c,7l}/BUILD, sys/src/cmd/nm.BUILD, nix/toolchain.sh, .gitignore"
  - "14-06 (the boot chain rides the same module delivery + bake), 14-07 (the flake iso-builder shares the grub template convention), 15+ (the tier payload grows the bake)"
tech-stack:
  added: []
  patterns:
    - "the l.S stash discipline extended to the PAYLOAD: firmware-delivered data living inside the Mach mebibyte must be copied to .bss before the early-boot zeroing — the mb2 info (13-02) and now the bootfs module (14-05) both ride it"
    - "the committed grub.cfg as TEMPLATE SOURCE with @R9-MODULE@/@R9-MODULE-DOC@ markers — every emitter (xtask today, the 14-07 flake builder) substitutes the module2 line IN/OUT; the kernel-only emission is pinned byte-identical to Phase 13 by a unit test against the committed file"
    - "the passwordless-venue namespace build: an unprivileged user+mount namespace (unshare -Ur -m) with a chroot scaffold on a private tmpfs — leaf-mount binds (a directory carrying submounts cannot be plain-bound in a userns), the mapped root IS the invoking user, and NO cleanup rm exists by construction (a recursive rm walking binds deletes through them into the source trees — measured the hard way and recovered from git)"
key-files:
  created:
    - ../r9/x86_64/lib/config_q35exec.toml
    - build/x86_64/smokesleep.c
    - build/x86_64/stage-build.sh
    - build/x86_64/mkpaqfs-host.sh
    - build/x86_64/mkimage.sh
    - build/x86_64/root-tree/README.md
  modified:
    - ../r9/x86_64/src/platform/q35/mb2.rs
    - ../r9/x86_64/lib/grub.cfg
    - ../r9/xtask/src/main.rs
    - ../r9/xtask/src/config.rs
    - ../r9/flake.nix
    - ../r9/x86_64/src/main.rs
    - ../r9/x86_64/src/l.S
    - ../r9/x86_64/src/proc.rs
    - ../r9/x86_64/src/runtime.rs
    - sys/src/cmd/paqfs/mkpaqfs.c
    - sys/src/cmd/7a/BUILD
    - sys/src/cmd/7c/BUILD
    - sys/src/cmd/7l/BUILD
    - sys/src/cmd/nm.BUILD
    - nix/toolchain.sh
    - .gitignore
key-decisions:
  - "grub.cfg templating via marker lines (@R9-MODULE@ / @R9-MODULE-DOC@), not a builder-side generated config: the committed file stays the single human-documented source and the kernel-only emission stays byte-identical to Phase 13's 13-line config (cmp-proven + unit-test-pinned against the committed file; the flake FOD stages through the same strip so its loader-config component is unchanged)"
  - "the module BYTES ride an l.S .bss stash (MODSTASHMAX 8 MiB) — GRUB's heap placed the first live module at 0x141000, inside the Mach mebibyte l.S zeroes; the kernel reconciles the stash length against the tag's range (clamp or disagreement = a loud named panic), and the KZERO-alias read the plan sketched became a stash read"
  - "the first ring-3 exec exposed three latent kernel defects, each fixed in place: the long-mode entry left the whole kernel executing in the IDENTITY alias (RIP-relative leaq after the far-jump; fatal the moment a per-process PML4 without slot 0 loaded — fixed with movabsq to the linked VA); KSTACK_VA was non-canonical (index 100's low-half offset sign-extended; every window access #GP(0) — fixed to the true PML4[400] sign-extension 0xffff_c800_0000_0000); the panic handler was a silent loop (the aarch64 loud shape ported)"
  - "the marker IS /amd64/init (D-08 leg 1): stage-build links smokesleep once and stages it under both names; mkimage's identity guard pins marker-IS/real-init-IS-NOT via rodata greps"
  - "the bake is KERNDATE-pinned end to end: mkpaqfs.c writeHeader honors KERNDATE (unset = wall clock, existing bakers unchanged) and mkimage touches the staged tree to the same epoch — two bakes of one tree are byte-identical (proven live: sha256 9cbf26ea… twice)"
  - "the sudoless venue: build/x86_64/stage-build.sh tries the documented sudo-unshare namespace first, then falls back to an unprivileged userns chroot scaffold (tmpfs, leaf binds, no cleanup rm) — required because the execution sandbox has no passworded sudo"
requirements-completed: [XEXEC-02]
commits: 2
plan_head_before: abee15c367821bebd84415805ccb1f5a2508aef6   # r9 repo (code); harvey ledger 5d7e0f3f6892 for the bake + docs commits
plan_head_after: ae4fb7c                                   # r9 repo
actuals:
  tokens: 18546   # chars/4 over the realized changed content (r9 -U0 diff 48872 chars + harvey 25312)
  tasks: 3
  commits: 2      # r9 repo (d426787, ae4fb7c); harvey adds 3131ae41a (Task 2) + this docs commit
status: complete
coverage:
  - deliverable: "Task 1 — mb2 module tag + templated module2 staging + config_q35exec + the xtask gate surface (tracer)"
    verification:
      - kind: command
        ref: "cargo test -p x86_64 mb2 — 7 passed (5 prior + module round-trip/repeat/before-mmap ordering + truncated/inverted/out-of-mmap x3/TooManyModules + the truncation sweep extended); cargo test -p xtask — 99 passed incl. the two emission tests (kernel-only = Phase-13 bytes pinned against the committed file; module-IN carries the placeholder's payload)"
        status: pass
      - kind: command
        ref: "cargo xtask check + clippy exit 0; q35 PASS (1559-byte capture, kernel-only behavior unchanged); q35net PASS (4352 ping-pong frames, the pcap assertion holds); aarch64 virt PASS — the tracer gate re-ran end-to-end green (⚡ Tracer verified end-to-end — expanding)"
        status: pass
      - kind: command
        ref: "acceptance greps: TAG_MODULE=11 (>=2), module2 grub.cfg=3 + main.rs=7 (>=3 total), config_q35exec exec=5 (>=2), initrd-key lines=0; cmp of the staged kernel-only grub.cfg vs the Phase-13 bytes — identical"
        status: pass
    human_judgment: false
  - deliverable: "Task 2 — the Harvey x64 marker + bake pipeline"
    verification:
      - kind: command
        ref: "sh build/x86_64/stage-build.sh && sh build/x86_64/mkimage.sh — PASS: bootfs.paq 36207 bytes, header 529ab12b / trailer 6b46e688 via od -j, the paq magic in the first 4 bytes (PAQ-OK)"
        status: pass
      - kind: command
        ref: "the A1 real-binary proof: 'stage: init magic 00008a97 verified' + smokesleep on real 6.out bytes (grep count 2 in the stage log); determinism: two bakes byte-identical (sha256 9cbf26ea… twice, KERNDATE 1794171510)"
        status: pass
      - kind: command
        ref: "acceptance greps: notify=4 (>=2), objtype=amd64=6 (>=1), 00008a97=2 (>=1), 529ab12b=3 + 6b46e688=3 (>=1), factotum=4 (>=1, the absence assert), bootfs.paq non-empty with the magic"
        status: pass
    human_judgment: false
  - deliverable: "Task 3 — exec_bootstrap + the q35exec gate green twice (the exec proof)"
    verification:
      - kind: command
        ref: "cargo xtask boot-test --arch x86-64 --config q35exec — PASS TWICE CONSECUTIVELY (captures retained at /tmp/q35exec-run{1,2}.log, 2 smokesleep lines each): banner -> memory facts -> discovery -> 'q35: module @0x141000 +0x8d6f (mb2 tag 3, stashed)' -> 'q35: paqfs root mounted' -> 'devroot: /amd64/init open (35293 bytes)' -> 'exec: entering ring 3 pc=0x20010d sp=0x7fffffffefd8' -> 'smokesleep: got note alarm' + 'smokesleep: interrupted' (the binary's OWN WRITE syscalls) -> 'all user processes exited'"
        status: pass
      - kind: command
        ref: "full regression: q35 PASS, q35net PASS, ureg-check both arches Match + the negative control fired, aarch64 virt PASS, check/clippy exit 0, 60 x86_64 host tests + 99 xtask tests green, aarch64/+port diff empty at every commit"
        status: pass
      - kind: command
        ref: "acceptance greps: exec_bootstrap=3 (>=2), mount_initrd=2 (>=1), BOOT_ENTRY=5 (>=1)"
        status: pass
    human_judgment: false
---

# Phase 14 Plan 5: The Runtime Exec Tracer — First Harvey amd64 Exec on r9 x86_64 Summary

Landed XEXEC-02's leg-1 proof: one Harvey amd64 marker binary (smokesleep, staged at /amd64/init) execs on r9 x86_64 end-to-end — GRUB delivers bootfs.paq as an mb2 tag-3 module off the same ISO, the kernel stashes/mounts/walks it to the paqfs tree, and the marker's OWN WRITE syscalls put its serial markers on the wire through the real EL/SYSCALL path — gated green twice consecutively with every Phase-13 gate preserved. Bring-up exposed and fixed three latent kernel defects (the identity-alias entry, the non-canonical kstack window, the silent panic handler) and one environment class (the toolchain's /bin/rm traps on NixOS).

**Duration:** 108 min (2026-11-08T21:00:18Z → 22:48:05Z)
**Tasks:** 3/3 complete
**Files:** 16 r9-side (1 created, 15 modified) + 10 Harvey-side (6 created, 4 modified + the toolchain BUILD fixes)

## Commits

| Repo | Hash | Message |
|------|------|---------|
| r9 | d426787 | feat(14-05): mb2 module tag + templated module2 staging + the q35exec profile (D-03 delivery leg) |
| harvey | 3131ae41a | feat(14-05): the Harvey x64 marker + bake — build/x86_64 re-keyed (D-04/D-09) |
| r9 | ae4fb7c | feat(14-05): exec_bootstrap — the first Harvey amd64 exec on r9 x86_64 (XEXEC-02 leg 1) |
| harvey | (docs) | docs(14-05): complete the runtime tracer plan (this file + state) |

## Accomplishments

### Task 1: the D-03 delivery leg (tracer)

- **mb2.rs TAG_MODULE=3**: per-module collection (tags repeat; MAX_MODULES 4 headroom), checked le_u32 gets for mod_start/mod_end, `InvalidModuleBody` (size < 16), `InvalidModuleRange` (end <= start), and — at walk completion, because the mmap tag may trail the module tag — `ModuleNotInMemoryMap`: every module range must sit inside ONE available memory-map entry (T-14-08's bounds-check-before-any-mapping, checked_add discipline) plus `TooManyModules`. Six new test assertions across two tests (round-trip, repeat tags, tag-before-mmap ordering, kernel-only-none, truncated, inverted/empty, out-of-mmap in reserved/above-all/straddling shapes, overflow) and the truncation-sweep fixture extended with a module tag.
- **grub.cfg as TEMPLATE SOURCE**: `# @R9-MODULE-DOC@` lines (always dropped) + the `# @R9-MODULE@ module2 /boot/bootfs.paq` placeholder; `emit_grub_cfg` emits the line IN (marker stripped, indentation/payload kept) or OUT. The kernel-only emission is byte-identical to Phase 13's 13-line config — proven by cmp against the HEAD bytes AND pinned by an xtask unit test that reads the committed file and asserts the exact Phase-13 output. The flake's boot-iso FOD stages through the same strip so its loader-config component is unchanged (the derivation's wired hash is already stale from 14-01's kernel-byte drift — pre-existing, see Deviations).
- **xtask**: `[dist] module` key (harvey:-prefix resolution via R9_HARVEY_DIR, loud producer-naming absence error naming the Harvey bake scripts); build_x86_64_iso stages /boot/bootfs.paq + emits the config; the q35exec BootStage chain (Q35ModulePaqfs → Q35ExecEntry → Q35MarkerPrints → AllExited) + BootProfile::X64Exec, each stage order-anchored past the previous satisfaction.
- **config_q35exec.toml**: dev=["exec"], the 14-04 tick cfg, q35 [qemu]/netdev values, NO initrd key (the D-03 note recorded in-file), module = "harvey:build/x86_64/out/bootfs.paq".
- Tracer feedback gate re-ran the full verify end-to-end green.

### Task 2: the Harvey x64 marker + bake (D-04/D-09)

- **smokesleep.c** re-keyed: the same notify/alarm/sleep/noted(NCONT) flow and markers, exercising the ABI's note-delivery convention (the handler's ureg arg0 through the bp path) — the q35exec BOOT_ENTRY binary.
- **stage-build.sh**: objtype=amd64 6c/6l, the NATIVE_LINUX unshare shape only; the marker staged as BOTH smoke-out/smokesleep and smoke-out/init (leg 1's rule: the marker IS /amd64/init); S_MAGIC 00008a97 asserted AND printed on real 6.out bytes (the A1 proof, live: "stage: init magic 00008a97 verified"). Venue split: the documented sudo-unshare namespace when passwordless sudo exists, else the unprivileged-userns chroot scaffold (tmpfs + leaf binds + the plan9port rc bind; NO cleanup rm by construction).
- **mkpaqfs-host.sh** re-keyed (arch-neutral tool, shared .build/bin/mkpaqfs); **mkpaqfs.c writeHeader honors KERNDATE** (unset = wall clock — existing bakers unchanged).
- **mkimage.sh**: the leg-1 image contract (marker at /amd64/init, mount-point dirs incl. /net, NO /boot/factotum — the absence assert, NO /boot|/rc — 14-06's), the init-identity guard (marker-IS/real-init-IS-NOT via rodata greps), KERNDATE-pinned mtimes + header, the od -j header/trailer magic checks. Determinism proven live: two bakes byte-identical (sha256 9cbf26ea…).
- **Toolchain venue fixes** (blocking, Rule 3): the /bin/rm EXIT traps in nix/toolchain.sh + cmd/{7a,7c,7l}/BUILD + nm.BUILD fail on NixOS-class hosts (/bin holds only sh) and poison exit statuses mid-build — the system rm resolved via the hostbin-stripped PATH (the scripts' own _HOSTNM idiom).

### Task 3: exec_bootstrap + the gate (D-08)

- **exec_bootstrap** (main.rs, the aarch64 orchestration port): module found (D-03) with its bytes from the l.S stash (length reconciled against the tag — clamp/disagreement = a loud named panic), the full port-device hook block (devproc/cons/pipes/mnt/dup/srv; ether/ip under their cfgs), root env seeded (cputype=amd64), devroot mounted → walk/open OEXEC → exec_into, the one-way CR3 switch (kernel half 510/511 shared — the console-alive print proves it), activate_first (+ CURFRAME record), trap_return handoff. IF=0 through the whole bootstrap; the seeded frame's If flag unmasks at ring 3. The netgate trigger keeps its own cfg.
- **l.S**: the module stash (the 13-02 info-stash discipline extended to the payload — the first live module landed at 0x141000 INSIDE the Mach mebibyte the early boot zeroes; the asm tag-walk + capped copy land the bytes in the 8 MiB .bss modstash before the zeroing) and the linked-VA entry fix (movabsq $main — the RIP-relative leaq after the far-jump left the WHOLE KERNEL executing in the identity alias since 13-01, invisible while the boot PML4's slot-0 identity map lived, #PF-fetch the moment a per-process PML4 loaded).
- **proc.rs**: KSTACK_VA corrected to 0xffff_c800_0000_0000 — the canonical sign-extension of PML4[400]<<39; the bring-up constant 0xffff_3200_0000_0000 was index 100's low-half offset wrongly sign-extended (non-canonical: every window access #GP(0), invisible while nothing dereferenced the window — the 14-04 host tests check PTEs and write via the PA alias).
- **runtime.rs**: the panic handler prints (the aarch64 loud shape) — the silent loop hid the first mount failure behind a motionless serial line.
- **The gate, green twice**: banner → facts → discovery → module/paqfs notes → exec entry (pc=0x20010d sp=0x7fffffffefd8) → **smokesleep: got note alarm / smokesleep: interrupted** (the binary's OWN prints — the note delivered via ureg->bp, the sleep interrupted by the alarm, noted(NCONT) resumed) → all-exited. Captures retained. The prohibitions honored: no kernel print ever produced the marker's lines (D-09).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The toolchain apps failed on this NixOS host (/bin/rm traps)**
- **Found during:** Task 2 precondition (the toolchain bootstrap)
- **Issue:** `trap '/bin/rm …' EXIT` in nix/toolchain.sh + sys/src/cmd/{7a,7c,7l}/BUILD + nm.BUILD — NixOS keeps only sh in /bin, the trap fails, and its failure poisons the script's exit status (a failing EXIT trap overrides it), aborting the toolchain build midway (ar/9cc never built).
- **Fix:** the system rm resolved once via the hostbin-stripped PATH (the scripts' own _HOSTNM idiom), trap quotes the variable. Toolchain then builds green (6a/6c/6l, 7a/7c/7l, 8a/8c/8l, ar, cpp, nm, pcc…).
- **Files modified:** nix/toolchain.sh, sys/src/cmd/7a/BUILD, sys/src/cmd/7c/BUILD, sys/src/cmd/7l/BUILD, sys/src/cmd/nm.BUILD
- **Commit:** harvey@3131ae41a

**2. [Rule 2 - Missing-critical] mkimage's KERNDATE pinning needed an mkpaqfs seam**
- **Found during:** Task 2 (the determinism contract)
- **Issue:** the plan requires a KERNDATE-pinned bake, but the in-tree mkpaqfs stamps its header with time(nil) — the aarch64 bakes accepted per-bake digests; 14-07's composed-ISO FOD inherits this bake.
- **Fix:** writeHeader honors KERNDATE (unset = wall clock, byte-for-byte the old behavior); mkimage pins the staged tree's mtimes to the same epoch. Proven: two bakes identical.
- **Files modified:** sys/src/cmd/paqfs/mkpaqfs.c, build/x86_64/mkimage.sh
- **Commit:** harvey@3131ae41a

**3. [Rule 3 - Blocking] No usable sudo in the execution sandbox — the namespace build's documented venue**
- **Found during:** Task 2 (stage-build's first run)
- **Issue:** nsrun.sh's sudo-unshare needs a password (fingerprint prompt, no terminal) — the documented native-linux venue presumes an interactive user.
- **Fix:** the venue split in stage-build.sh: sudo-unshare when `sudo -n` works, else an unprivileged user+mount namespace (unshare -Ur -m) with a chroot scaffold — private-tmpfs fake root, LEAF binds only (measured: a directory carrying submounts — /nix, /run, /dev — cannot be plain-bound in a userns; /nix/store, the recreated /run/current-system symlink, per-node /dev binds), the usrmerge /bin symlink farm + the plan9port rc bind, hostbin re-rooted for inside. The mapped root IS the invoking user (same uid, no privilege, correct artifact ownership).
- **Files modified:** build/x86_64/stage-build.sh
- **Commit:** harvey@3131ae41a

**4. [Rule 1 - Incident + recovery] The scaffold's cleanup trap deleted live trees through bind mounts**
- **Found during:** Task 2 (the first userns scaffold iteration)
- **Issue:** the adapter's `trap 'rm -rf "$NSX"' EXIT` walked a scaffold containing bind mounts of sys/, amd64/, rc/ — rm operates on pathnames, and through a bind the paths ARE the source files: 11,652 tracked files deleted from the checkout (the exact bind set).
- **Fix + recovery:** every file was git-tracked — restored with a path-targeted `git restore sys amd64 rc` (0 deletions remain; the four in-tree edits re-applied). The scaffold redesign eliminates the class: the fake root is its OWN tmpfs and there is NO cleanup path at all — the scaffold dies with the user namespace (mount -t tmpfs + the SAFETY comment recording the measured failure mode). Recorded here and in the script so no future venue reintroduces an rm over binds.
- **Files modified:** build/x86_64/stage-build.sh (+ git restore of sys/amd64/rc)
- **Commit:** harvey@3131ae41a

**5. [Rule 1 - Bug] Three latent kernel defects the first ring-3 exec exposed (Task 3 bring-up)**
- **Found during:** Task 3's gate iterations
- **Issue:** (a) the module's bytes lived in the Mach mebibyte l.S zeroes (the mount failed silently because (b) the panic handler was a silent loop); (c) the whole kernel executed in the identity alias since 13-01 (l.S's RIP-relative jump after the far-jump) — fatal only at the first per-process CR3 load; (d) KSTACK_VA was non-canonical (14-04's arithmetic) — every kstack-window access #GP(0), invisible while nothing dereferenced the window.
- **Fix:** the l.S module stash (the payload rides the 13-02 info-stash discipline; .bss modstash, 8 MiB cap, length reconciled kernel-side); the loud panic handler (aarch64's shape); `movabsq $main` (the linked-VA entry — the identity window is now dead weight under the boot PML4 and absent from every proc table by design); KSTACK_VA = 0xffff_c800_0000_0000 (the true sign-extension; the 14-04 host tests' self-consistent assertions carried automatically).
- **Files modified:** ../r9/x86_64/src/{l.S,main.rs,proc.rs,runtime.rs}
- **Commit:** r9@ae4fb7c

**6. [Rule 3 - Placement] The flake FOD's grub staging needed the strip rule in this plan**
- **Found during:** Task 1 (the template conversion)
- **Issue:** the boot-iso FOD `cp`s grub.cfg verbatim — the templated committed file would change its staged bytes. (The wired outputHash turns out to be stale since 14-01's kernel-byte drift regardless — PROVEN pre-existing by a fresh FOD build at the pre-plan commit abee15c, which also fails the wired hash; logged to deferred-items.md. 14-07 owns the re-wire per OQ-1.)
- **Fix:** the FOD stages through the same sed strip as xtask — the loader-config component stays Phase-13-identical so 14-07's re-proof isolates kernel-byte drift.
- **Files modified:** ../r9/flake.nix
- **Commit:** r9@d426787

**7. [Rule 3 - Verify-command re-key] The 14-01..04 environment class, plus one plan-command refinement**
- **Found during:** every task's verify
- **Issue:** the no_std default-target/`--bins --target x86_64-unknown-linux-gnu` re-key, nix-develop wrapping, R9_HARVEY_DIR exports (documented by the prior plans); the plan's mkimage verify greps the stage log for 00008a97, but the assert only NAMED the magic on failure.
- **Fix:** the prior plans' command re-keys carried; the stage PASS lines now print each verified magic as evidence ("stage: init magic 00008a97 verified").
- **Files modified:** build/x86_64/stage-build.sh
- **Commit:** harvey@3131ae41a

**Total deviations:** 7 auto-fixed (2 blocking-infrastructure, 1 missing-critical seam, 1 venue blocker, 1 recovered incident, 1 latent-defect batch, 1 verify re-key). **Impact:** none open — every plan verify leg ran and passed, and the incident's recovery is proven (0 deletions, all edits re-landed).

## Authentication Gates

None.

## Known Stubs

None in the shipped code. The l.S identity window (PML4[0] under the boot table) is now dead weight — never entered, never consumed; retiring it is a one-line l.S cleanup for a later plan (recorded in WINDOWS as a deviation note for visibility). The boot-iso FOD outputHash staleness is pre-existing since 14-01 (deferred-items.md + WINDOWS; 14-07's declared work).

## Issues Encountered

The Task-2 scaffold incident (Deviation 4) — fully recovered; the tree is verified whole (0 deletions, every acceptance grep green on the re-landed files).

## TDD Gate Compliance

N/A — this plan's tasks carry no tdd="true" (the mb2 module-tag fixtures and the xtask emission tests are plan-declared verification content, landed with their implementations).

## Next Plan Readiness

Ready for 14-06 — the boot chain has its delivery (the module tag + the grub template), its bake pipeline (mkimage re-keys to the real /boot/boot + /amd64/init in one step), its kernel orchestration (exec_bootstrap's BOOT_ENTRY swaps to /boot/boot under dev_bootchain), and the exec substrate is PROVEN end-to-end by the marker's own prints.

## Self-Check: PASSED

- r9 files exist: config_q35exec.toml created; {mb2.rs, grub.cfg, main.rs, config.rs, xtask/main.rs, flake.nix, l.S, proc.rs, runtime.rs} modified in r9@{d426787,ae4fb7c}
- harvey files exist: build/x86_64/{smokesleep.c,stage-build.sh,mkpaqfs-host.sh,mkimage.sh,root-tree/README.md} created; {mkpaqfs.c,7a/7c/7l BUILD,nm.BUILD,toolchain.sh,.gitignore} modified in harvey@3131ae41a
- commits are ancestors of HEAD: r9 d426787 ✓ ae4fb7c ✓; harvey 3131ae41a ✓
- all task acceptance-criteria greps re-run PASS (logged in coverage)
- plan-level verification green: q35exec ×2 (marker's own output captured), q35, q35net, ureg-check + negative control, aarch64 virt, check/clippy, 60+99 tests, aarch64+port diff empty
