---
schema_version: 1
open_count: 51
waived_count: 4
fixed_count: 17
total_count: 72
last_updated: 2026-11-22T20:34:58.236Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 02 | deviation | sys/src/libmach/executable.c |  | 02-02: 9c host probe substituted with direct host-clang (plan9port header shadowing, A3 fallback); 7.c/7obj.c compile proofs deferred to 02-06/02-03 by plan design | open |  | 2026-10-31T17:02:41.072Z |  |
| 2 | 02 | deviation | sys/src/libmach/executable.c |  | 02-02: 9c host probe substituted with direct host-clang (plan9port header shadowing, A3 fallback); 7.c/7obj.c compile proofs deferred to 02-06/02-03 by plan design | open |  | 2026-10-31T17:02:46.189Z |  |
| 3 | 02 | deviation | sys/src/cmd/pcc.c |  | 02-06 verify(a): pcc functional proof relocated to lima guest — pcc.c execs /bin/cpp + /bin/7c absolute; darwin /bin SIP-sealed (pre-existing); proof passed in guest (PCC-PROOF-PASS) | open |  | 2026-11-01T15:15:57.249Z |  |
| 4 | 03 | deviation | .planning/phases/03-r9-qemu-virt-platform-bring-up/03-01-PLAN.md | 101 | 03-01 gate-2 toolchain probe (rustc --version \| grep nightly-2026-01-30) unsatisfiable — rustc prints source-commit date; corrected to store-path identity proof (rust-default-1.95.0-nightly-2026-01-30). See 03-01-SUMMARY deviation 4. | open |  | 2026-11-01T22:33:17.552Z |  |
| 5 | 03 | deviation | xtask/src/main.rs |  | 03-03 raspi3b boot-smoke gate satisfied by substituted A/B evidence (human-approved); upstream raspi3b/qemu-11.1.2 defect deferred | open |  | 2026-11-01T23:15:26.857Z |  |
| 6 | 03 | deviation | r9/aarch64/src/main.rs |  | 03-04: verify block-2's 'mkdir -p ... && qemu ... &' binds & to the whole list, so $QP names a subshell, not qemu; with a stale log the first poll (i=1) reads pre-boot content and the kill orphans qemu mid-boot. Gate passes deterministically from a fresh log (rm first). 03-05's D-11 boot-test socket chardev avoids this pattern. (File ref is r9-repo-relative: ../r9) | open |  | 2026-11-01T23:44:10.289Z |  |
| 7 | 03 | unrun-verify | r9/aarch64/src/trap.rs |  | 03-06: trap.rs sync dump path gained 'kernel halted' + wfi park on ALL platforms; verified live on virt (virtpanic evidence) but compile-only on raspi — raspi boot verification is blocked by the known upstream raspi-on-qemu-11.1.2 defect (03-03/03-04 deferred, WINDOWS #5). Behavior change is dump-path-only. (File ref is r9-repo-relative: ../r9) | open |  | 2026-11-02T00:33:39.701Z |  |
| 8 | 04 | deviation | .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-05-PLAN.md |  | 04-05: plan's Task-2 raw-qemu verify leg written against the pre-amendment -initrd mechanism (falsified 04-01, D-01 AMENDED); executed with the amended loader+patched-DTB delivery, grep chain unchanged | fixed |  | 2026-11-02T13:58:26.801Z | 2026-11-02T13:58:40.779Z |
| 9 | 04 | deviation | .planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-05-PLAN.md |  | test-shape-probe | waived | Accidental probe entry appended while checking the windows CLI output shape (executor error, no content); the real entry is #8. | 2026-11-02T13:58:30.416Z | 2026-11-02T13:58:40.990Z |
| 10 | 04 | deviation | .planning/phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md |  | 04-07: exec_bootstrap IRQ-masked until first EL0 entry (heisenbug exposure closed by construction; root cause = deferred-items #4) | open |  | 2026-11-02T18:41:05.291Z |  |
| 11 | 05 | stub | r9:port/src/devcons.rs |  | cons reads return loud -1 until 05-08 wires the PL011 RX ring through the device (plan-mandated; open succeeds) | open |  | 2026-11-03T05:59:51.093Z |  |
| 12 | 05 | stub | r9:port/src/devproc.rs |  | devproc directory reads + status/args absent this plan; 05-08/05-09 grow them (ps needs /proc/*/status) | open |  | 2026-11-03T05:59:51.314Z |  |
| 13 | 05 | deviation | r9:aarch64/src/exec.rs |  | exec maps data+bss eagerly (cold syscall-arg residency) — D-09-letter adjustment, T-05-06 honored; phase-04 deferred #3 closed for the argument class | open |  | 2026-11-03T05:59:51.532Z |  |
| 14 | 05 | stub | r9:aarch64/src/syscall.rs |  | CREATE(22) stays loud-unimplemented until 05-06 wires sys_create; devenv's create op is host-tested and slots in unchanged (init's setenv/cpenv path); plan-mandated ordering (05-05 Task 1 action step 1) | fixed |  | 2026-11-03T06:32:58.158Z | 2026-11-03T08:27:38.113Z |
| 15 | 05 | deviation | r9:aarch64/src/proc.rs |  | 05-05: plan's RFENVG/RFCENVG bullet inverted vs fork(2)+9k sysproc.c — donor governs (neither=share, RFENVG=copy, RFCENVG=empty); the 05-04 RFNAMEG/RFCNAMEG precedent; pinned by devenv_test.rs + the env-driven boot gate | open |  | 2026-11-03T06:32:58.375Z |  |
| 16 | 05 | deviation | r9:aarch64/src/proc.rs |  | 05-06: plan's fd-group bullet inverts RFFDG vs default vs fork(2)+9k sysproc.c (neither=SHARE, RFFDG=COPY, RFCFDG=empty) — the phase's third flag inversion; donor governs (libc fork()=RFFDG=COPY is what connectembed's dup/close needs); pinned by fdgroup_test.rs | open |  | 2026-11-03T08:27:38.289Z |  |
| 17 | 05 | deviation | r9:aarch64/src/fault.rs |  | 05-06: cold-syscall-buffer class fixed at the gate (EL1h DataAbort on paqfs's emalloc'd brk_-grown buffer — the kernel's pread made the first touch) — fault::ensure_user_resident generalizes 04-08's explicit page-in to any writable user buffer before deref; T-05-06 holds (never a nested fault); phase-04 deferred #3 closed for the growth class | open |  | 2026-11-03T08:27:38.460Z |  |
| 18 | 05 | todo | r9:port/src/namec.rs |  | 05-07: pgrp lock held across namec walks incl. devmnt's blocking Twalk RPCs — a pgrp-SHARING contender (fork shares the pgrp) spins IRQ-masked = wedge once the first concurrent namespace caller exists (rc pipelines/backgrounded cmds); the gate never contends (strictly sequential chain). Fix: snapshot the union chain under a brief lock, walk unlocked (phase deferred-items.md) | fixed | r9 59d28fa + the 17-05 four-site audit (verify+pin: all four pgrp-lock sites in x86_64/src/syscall.rs conform to the snapshot-then-drop contract — sys_bind :527 / sys_unmount :599 / sys_mount :716 lock the mount-table mutation only and drop(guard) before any close dispatch; sys_create :1159 is the chain_snapshot block shape verbatim with redirect_clone running outside it) + the 17-02 contention captures (the q35churn profile's 39 RFMEM semacquire/semrelease generations + the composed /bin/cat x1400 fork+exec churn, both green twice — the rc-pipeline/backgrounded-command contender shape, no wedge) | 2026-11-03T10:19:28.856Z | 2026-11-13T01:47:22.000Z |
| 19 | 05 | todo | r9:port/src/devcons.rs |  | 05-07: #c/pid absent — getpid() fails, init's pri-10 write names #p/0/ctl (the designed warning fires either way; the gate is unaffected). 05-09's devproc ctl is the first real consumer — add devcons's Qpid file (donor devcons.c) via a current-pid hook before it lands | open |  | 2026-11-03T10:19:29.077Z |  |
| 20 | 05 | deviation | build/aarch64/drive-r9.expect |  | interactive-driver input wedge flake: 2 of ~7 early sweep runs wedged at a prompt read (line never completes); 6 consecutive clean passes since; not reproducing under the cons watchdog; deferred as the top debug candidate for 05-12/05-13's CI leg | open |  | 2026-11-03T15:09:13.138Z |  |
| 21 | 05 | stub | .planning/phases/05-shell-boot-one-command-integration/deferred-items.md |  | r9-side: stat(5) records carry 3 strings (no muid) in devcons/devroot/devenv/devsrv/devdup listings — dirreadall's statcheck walks FOUR and rejects them (devproc fixed 05-09; the rest latent, no dirreadall consumer this era) | open |  | 2026-11-03T15:09:20.285Z |  |
| 22 | 05 | deviation | build/aarch64/drive-r9.expect |  | interactive-driver input wedge flake: 2 of ~7 early sweep runs wedged at a prompt read (line never completes); 6 consecutive clean passes since; not reproducing under the cons watchdog; deferred as the top debug candidate for 05-12/05-13's CI leg | open |  | 2026-11-03T15:09:20.518Z |  |
| 23 | 05 | stub | .planning/phases/05-shell-boot-one-command-integration/deferred-items.md |  | r9-side: stat records carry 3 strings no muid in devcons/devroot/devenv/devsrv/devdup listings - dirreadall statcheck walks FOUR and rejects them (devproc fixed 05-09) | waived | accidental duplicate of #21 (executor re-ran the append after a misleading tail display) | 2026-11-03T15:09:25.686Z | 2026-11-03T15:09:56.078Z |
| 24 | 05 | deviation | build/aarch64/drive-r9.expect |  | interactive-driver input wedge flake: 2 of ~7 early sweep runs wedged at a prompt read; 6 consecutive clean passes since; top debug candidate for 05-12/05-13 CI | waived | accidental duplicate of #20 (executor re-ran the append after a misleading tail display) | 2026-11-03T15:09:25.913Z | 2026-11-03T15:09:56.309Z |
| 25 | 261103-giq | deviation | build/harness/src/srv9.rs |  | Twstat name offset corrected to 47 (Dir wire shape is doubly size-prefixed); the .py's offset-45 rename was dead code — mv fell back to copy+delete | open |  | 2026-11-03T18:24:06.467Z |  |
| 26 | 261103-giq | deviation | build/harness/src/trace.rs |  | trace-syscalls: prompt-substring wait after pid probe closes the .py's latent echo/prompt race | open |  | 2026-11-03T18:24:06.688Z |  |
| 27 | 261103-giq | deviation | build/aarch64/vm-run.sh |  | rsync exclusion for build/harness/target added to the four vm-sync scripts | open |  | 2026-11-03T18:24:06.904Z |  |
| 28 | 261103-giq | deviation | build/aarch64/subset-exec.sh |  | guest-scratch CONF=qemu bootdir reset to pristine (39 accumulated rows exceeded Nbootfiles=32; pre-existing guest state) | open |  | 2026-11-03T18:24:07.120Z |  |
| 29 | 06 | deviation | build/aarch64/gethunk-repro-cc.sh |  | cc repro input shape changed from plan's >600KB string literal to digit/comma macro define (NSYMB/STRINGSZ caps reject the sketched shape before gethunk); use line dropped (macexpand Io.b 16KB cap) | open |  | 2026-11-04T23:18:26.722Z |  |
| 30 | 06 | deviation | build/aarch64/gethunk-repro-6l.sh |  | 6l repro input shape settled by A4 spike to >2.5MB archive __.SYMDEF bang-malloc (array toothless, string-literal impossible); venue linux-only (darwin sbrk 4096KB arena); fixed pair built from working tree not hostbin | open |  | 2026-11-04T23:18:26.940Z |  |
| 31 | 07 | deviation | build/aarch64/root-tree/lib/net-sweep.rc | 77 | dd syntax corrected to dash-pair form (-if/-of); the baked dd.c rejects classic if=/of= (plan prose corrected, gate-blocking) | open |  | 2026-11-05T00:04:57.930Z |  |
| 32 | 07 | deviation | build/aarch64/mkimage.sh | 165 | trailer magic re-extracted via od -j (Harvey host dd shadows POSIX dd on the devshell PATH — D-02 class) | open |  | 2026-11-05T00:04:58.144Z |  |
| 33 | 07 | deviation | build/aarch64/root-tree/lib/net-sweep.rc | 77 | dd syntax corrected to dash-pair form (-if/-of); the baked dd.c rejects classic if=/of= (plan prose corrected, gate-blocking) | open |  | 2026-11-05T00:05:04.202Z |  |
| 34 | 07 | deviation | build/aarch64/mkimage.sh | 165 | trailer magic re-extracted via od -j (Harvey host dd shadows POSIX dd on the devshell PATH — D-02 class) | open |  | 2026-11-05T00:05:08.206Z |  |
| 35 | 06 | deviation | .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-02-PLAN.md | 216 | 06-02 Task-3 verify gate 2 text defective: mk -n never recurses, so the nested-path grep on cmd's own output always counts 0; intent proven by chain probes in 06-02-SUMMARY; repair gate text at 06-03 planning | open |  | 2026-11-05T00:10:54.539Z |  |
| 36 | 07 | unrun-verify | r9:aarch64/src/platform/virt/pci.rs |  | 07-06 D6: virtio IRQ handler armed-and-inert-proven only; traffic-level proof lands with 07-08's NETDRV-04 gates (no traffic source until 07-07) | open |  | 2026-11-05T01:09:17.807Z |  |
| 37 | 06 | deviation | build/aarch64/coverage-sweep.sh |  | 06-04: second-chance visitation pass added — plan9port mk recipes abort at the first failing command (measured), so container recursion stops at the first failing sibling; the plan's adjacency truth (ipconfig+6 siblings visited) is otherwise unreachable with tier-0 libs outstanding; pass 2 gives each unvisited in-universe dir its own shimmed guarded mk all | open |  | 2026-11-05T02:10:46.163Z |  |
| 38 | 06 | deviation | build/aarch64/coverage-sweep.sh |  | 06-04: pass-2 initially invoked $REAL_MK directly — a leaf dir runs no nested mk so no VISIT line was logged (run-3 reconciliation failure); pass-2 visits now invoke bare mk through the shim | open |  | 2026-11-05T02:10:46.381Z |  |
| 39 | 06 | deviation | build/aarch64/coverage-sweep.sh |  | 06-04: namespace/devshell Plan 9-tool dialect fixes — sed {/pat/!p} group garbled (host), sed s/\|.*$// garbled by hostbin Plan 9 sed (namespace; silently emptied the exclusions in run 2), date -u +fmt prints the epoch under Plan 9 date — awk index/substr + host-precomputed plain-path exclusions + probe-verified ISO date | open |  | 2026-11-05T02:10:46.602Z |  |
| 40 | 06 | deviation | build/aarch64/generator-scan.sh |  | 06-04: ssh tree probes drained the audit-rows while-loop's stdin (only G1 checked on the first run) — all ssh probe sites carry </dev/null now | open |  | 2026-11-05T02:10:46.815Z |  |
| 41 | 06 | deviation | build/aarch64/generator-audit.txt |  | 06-04: plan/RESEARCH cells corrected against live mkfiles (plan-mandated re-confirmation): G5 artifact is o.pcode (cpuobjtype=o via /unix/mkfile O=o); G6 genarch is pcc-built native-only (build guard gs/mkfile:164), never built nor run in the aarch64 sweep | open |  | 2026-11-05T02:10:47.035Z |  |
| 42 | 06 | deviation | .planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-05-PLAN.md |  | 06-05 Task-2's literal no-PENDING-CLASSIFY grep contradicts Task-1's mandated append-only seed lines; ran the intent gate (latest-line-per-dir clean) — gate-text repair filed for the next planning pass | open |  | 2026-11-05T02:51:20.644Z |  |
| 43 | 08 | unmet-truth | r9:aarch64/src/proc.rs |  | 08-04: the NETIP-03 ping-127.0.0.1 truth is MET — RFMEM landed per the blocking-human decision (option A, donor dupseg semantics): Data/Bss pages share under RFMEM with PA-keyed refcount-gated frees, RFMEM families carry the shared break (sys_brk_ family arm births grown pages shared — the checkpoint's 'ping does not brk_-grow post-fork' premise was falsified at the gate and the donor's shared-segment growth is the implemented semantics), the rfork bss sweep closes the untouched-page hole, and SEMACQUIRE(37)/SEMRELEASE(38) implemented for the contended libc lock() path; virtip gate green (netip: ping ok + NETIP-DONE), 8/8 regression profiles green, r9 4298fc0 | fixed |  | 2026-11-05T19:54:26.000Z | 2026-11-05T22:30:00.000Z |
| 44 | 08 | deviation | r9:aarch64/src/proc.rs |  | 08-05: kernel arena (4 MiB QuickFit) drains under a spin loop of short-lived fork+exec — ~1400 iterations of /bin/cat from a fixture's port-discovery spin exhausted it (oom size=4160 panic); exec staging teardown (ByteBacking::Mnt) is the suspect; fixture-side workaround applied (invoke the baked name so the spin terminates) | fixed | r9 c4f2f2c + 72edbba (the churn gate's measured #44 root causes: exit_tail's never-dropped transients — the detached fd-group Arc ~8 KiB/exit — plus the Moribund husk's retained group handles/chans; the drain measured 16 KiB/exec pre-fix, 60 bytes/exec post-fix) + the q35churn boot profile + the boot-r9-churn-x64 composed gate (both green twice) | 2026-11-05T23:35:00.000Z | 2026-11-12T21:18:02.000Z |
| 45 | 08 | deviation | r9:port/src/namec.rs |  | 08-05: Creat on a top-level file of a union-mounted dir misses non-head members — walk_one never consults the union chain (documented shape); rc's `>` is Creat-first so `> /net/log` failed with 'namec: device has no create'; fixture uses `>>` (open-first); the two-level-deep ctl writes unaffected | fixed | r9 cfe7af0 + 421d5e1 (walk_one_union/create_chan_union: sys_create's final-component probe and the create dispatch consult the union chain per the per-consult chain_snapshot shape — first-accepting member, refusals propagate; the nomount follow-up keeps #-rooted parents off the empty mount-table key the boot's `bind -a /root /` owns — the tier1 gate caught the over-reach, T1-FAIL date from init's silently-failing setenv('#e/timezone')) + the namec_tests host pin (venue A: 6 pins RED_EVIDENCE_OK, incl. the empty-key counterfactual) + tier1/smoke/net-smoke/q35shell gates green at Harvey pin 09b59f6f2 (r9 59d28fa) | 2026-11-05T23:55:00.000Z | 2026-11-13T01:47:22.000Z |
| 46 | 08 | deviation | .planning/phases/08-r9-tcp-ip-path-devip-compatible-net/deferred-items.md |  | 08-08: rc redirect-stack/fd-reuse corner — a file-open redirect (`> file`) or an in-block `>[2]/dev/null` on a backquote dd INSIDE a subshell block carrying a held-open fd redirect (`} >[6]/mnt/data`) leaks the payload into the held fd's pipe (measured twice in-guest; root cause unpinned — suspect class: doredir re-applies a stale ROPEN record whose from-fd was closed/reused, Dup() return unchecked at rc plan9.c:583); 08-08's aan fixture sidesteps by construction (the drop moved in-band via baked /lib/netip-aan-drop.rc; dd summary rides the console); recorded in the phase's deferred-items.md | fixed | harvey 6779df7d7 (doredir's BOTH redirect arms — ROPEN and RDUP — check Dup's return; a failed dup now reports "rc: can't dup N to M: %r" through the new dupfailed() and exits with errors, so rc never execs against an unset target descriptor) + boot-r9-smoke-x64 / boot-r9-tier1-x64 re-green ALL STAGES PASS at the 17-05 pin (r9 59d28fa) with the fixed rc baked (out/rc == /amd64/bin/rc, sha256 51f81de8…, the re-pinned 38-member witness manifest); no targeted fixture — no sweep fails a dup and the fixture route rides the fd-dup-in-sourced-block hazard (15-02's measured dialect), disproportionate per D-11's escape clause | 2026-11-06T03:05:00.000Z | 2026-11-13T02:43:07.000Z |
| 47 | 08 | deviation | r9:port/src/devip/stack.rs |  | 08-09: the RX-IRQ wake with MULTIPLE registered devip waiters wedges/panics — the first NIC-RX wake with >1 blocked net proc never reaches the ping reader (measured: the composed net smoke's static leg wedged the guest fully idle on BOTH gicv2+tcg and gicv3+hvf with udpecho+tcpecho blocked in loopback reads; an earlier form panicked at the same stage; the static-alone boot with the identical spawn shape PASSES, and every xtask virtip mode runs =1 waiter). Root cause unpinned. BLOCKER-CLASS for Phase 9's tap venue + Phase 12's cpu service (aan over real net = many blocked net procs); the composed smoke sidesteps by stage order (ether legs first, single-waiter per exchange). Recorded in the phase's deferred-items.md | fixed | 17-07 fold verdict (17-CR07-ROOTCAUSE section 6, D-09 SPLIT): the WEDGE form — closed-by-discipline, independent of the #66 reset class. The fix families it anticipated landed and pinned after the Phase-8 record: re-probe-after-drive in every devip block loop (the Phase-8 gate-caught wakeup-loss class) + CR-03 register-per-iteration loop (r9 75970f0 + x86_64 sem pins f9088d0). The exact #47 shape (multiple readers blocked across loopback+ether, a NIC wake, the composed static leg) is GREEN on x86_64 at every pin since 14-09 (boot-r9-net-smoke-x64 multi-reader echo legs, exit 0 at pin 8f3ff11). Reopener: a multi-waiter WEDGE (idle guest, no reset) on any venue. Owner 2026-11-22: also green on aarch64 — phase 20's arm64 net-smoke multi-reader legs passed on HVF. | 2026-11-06T06:30:00.000Z | 2026-11-13T06:40:00.000Z |
| 48 | 08 | deviation | r9:port/src/devcons.rs |  | 08-09: a console input burst longer than the PL011-side 256-byte RX ring truncates mid-line with no reader-visible signal — the cooked line never completes and the guest wedges (measured 2/2: a 316-char typed line arrived as 255 chars; wedgedbg showed line=255 stuck). The drop-oldest overrun discipline is intentional; gate-author discipline (short typed lines / paced continuation lines) now encoded in smoke-r9-net.expect's header; a kernel-side 'line truncated' signal is the possible later refinement | open |  | 2026-11-06T06:30:00.000Z |  |
| 49 | 08 | deviation | sys/src/cmd/rc/exec.c |  | 08-09: rc's argv-stack-underflow flake residual — 'pushword but no argv!' + abort fired inside baked fixtures (netip-loopback.rc's udp spin; netip-static.rc's captures) AND in the plain 05-12 smoke's typed echo stage, on gicv3+hvf only (2/~6 HVF runs today; 0 TCG runs across the phase's whole gate history; the 06-07 Xbackq fix covered the top-level case — the residual fires in other op sequences; the aborted child's exit status prints <bad utf8>). Pre-existing class, HVF-timing-exposed; the CI/truth venue is TCG. Recorded in the phase's deferred-items.md | open | harvey 6779df7d7 (the argv-stack invariant restored at exec.c's two pop-then-pushword ops, Xqdol and Xcount — the havefork.c 06-07 Xbackq guard shape, `if(runq->argv==0) pushlist()`; the pushword/popword panic sites themselves untouched) + boot-r9-smoke-x64 / boot-r9-tier1-x64 re-green ALL STAGES PASS at the 17-05 pin; A6 note: the historical fires were HVF-timing-only (darwin, 2/~6 runs; zero on the linux TCG/KVM venue across the whole gate history) — unreproducible on this venue, so the fix is invariant-audit-grounded; reopener: a live "pushword but no argv" fire on any venue; REOPENED phase 20 (2026-11-17): research saw 4 fires in 13 HVF boots at the 8e4e730-era pin, and 20-02 measured 15 fires in 54 HVF boots (0 in 9 TCG) at r9 46364d0 with the 6779df7d7 rc guards in place, so the rc-side fix was not the cause. Root cause (20-EVIDENCE.md ## Root cause, confirmed by a PAR_EL1 probe): r9 aarch64 vm::switch wrote TTBR0_EL1 with no isb before tlbi vmalle1is; with no ASIDs a walk still using the old root refilled the outgoing proc's entries, and the fork child ran on its parent's live pages (110 of 114 child pages resolved to the parent's PAs, a process-isolation fault). Re-fixed in r9 cb9cde3 (isb after the TTBR0 and TTBR1 writes), pushed with r9 5f3ddb0 and pinned lock-only at Harvey 980f09c4e; that pin's series, no override, was HVF smoke 1753/1753, net-smoke 36/36, tier-1 10/10 (17860 gate forks, 0 fires), TCG 3/3 each (20-06). REOPENED AGAIN 2026-11-18 (20-06, owner decision A): at the final pin r9 2226cc4 (reproducible kernel sha256 d3537304...), net-smoke HVF failed run 21 of 36: a netip-loopback.rc fork child (pid 49) faulted at addr 0x1 in rc searchpath/strncmp on a corrupt runq->argv word, right after its parent reaped a child - the same class, a fault instead of the pushword abort; 1 fire in 16768 HVF gate forks. The cb9cde3 isb fix is necessary but not shown sufficient; next: dsb ishst hardening of vm::switch (20-03 work), then the series again. Reopener for the next close: any fork-child argv corruption (pushword abort or user fault in rc) on any venue. HARDENED 2026-11-19 (20-06, owner decision b): r9 57e2952 adds dsb ishst before the TTBR0/TTBR1 writes and the full tlbi; pinned lock-only at Harvey 0dafd07ad (kernel-virt sha256 da4ab2da...); its series at the plan's N was green (HVF smoke 1753/1753, net-smoke 36/36, tier-1 10/10, 17864 gate forks, 0 fires; TCG 3/3 each). Still OPEN: the isb-only residual was ~1.8e-5 per fork (1 fire in ~54,600 HVF gate forks), so 17864 clean forks cannot separate the hardened kernel from it. Follow-up to close: about 2,300 net-smoke HVF boots (~166,000 forks, ~9.6 h, stop-on-fail off) clean on the pinned kernel ROOT-CAUSED 2026-11-21 (debug session fork-child-pc-align-fault, resolved; .planning/debug/resolved/): the residual (105d48e pin series net-smoke run 23: rc fork child pid 49 jumped to 0x318e7; the 2226cc4 strncmp-on-0x1 fire; a can't-dup fire with stale stack args, all the same pid-49 backquote fork) is a DIFFERENT mechanism from the TTBR/TLB defect: r9 aarch64 rfork_copy wrote child text through the KZERO data alias with no I-cache maintenance, and with CTR_EL0.DIC=0 the HVF host keeps stale instructions per (VA, PA) across unmap+tlbi+remap (boot self-test: 64/64 stale same-VA, 0/64 other VAs, 64/64 fresh after ic ialluis), so the fork child whose rc simple.c page (VA 0x18000) landed on bind's just-executed page at the same VA ran bind's code (possible only since 8958c9e made frees real). Fixed r9 79c55b1 + regression guard 46a5c87 (TextCopy, the single rfork copy path; boot-time selftest prints icache: rfork text-copy selftest N/64 fresh and panics unless 64/64; HVF kills the maintenance-removed mutant 0/64), both pushed and in the final pin r9 e6e5220. Likely explains the whole post-cb9cde3 residual; the 2,300-boot follow-up is superseded by the deterministic guard. Close at the pin series on the pushed fix (step A, riding with the separate FP/SIMD save/restore fix). | 2026-11-06T06:30:00.000Z |  |
| 50 | 08 | deviation | r9:port/src/devpipe.rs |  | 08-09: intermittent early-boot EL1 DataAbort (translation fault level 3) in devpipe::read (devpipe.rs:282, the user-buffer copy) at FAR 0x4e428 — fired once during the composed smoke's HVF leg in the boot chain right after 'version...' (the boot(8) fauth leg), 1/~6 HVF runs, never on TCG; the same tree re-ran green end-to-end. The cold-syscall-buffer residence machinery (05-06's ensure_user_resident) is the suspect area — an intermittent miss in the early-boot pipe read path. Recorded in the phase's deferred-items.md | fixed | Owner 2026-11-22: evidence shows as fixed. The waiver's reopener (aarch64 reactivation) fired in phase 20, and no devpipe::read early-boot abort appeared across phase 20's HVF series and confidence passes (tens of thousands of HVF boots; see .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md). Prior waiver (history, 2026-11-13): 17-07 fold verdict (17-CR07-ROOTCAUSE section 6, D-09 SPLIT): a DIFFERENT CLASS, split out with its own 3-part disposition. WHERE: the aarch64 devpipe read path + the cold-syscall-buffer residence machinery (ensure_user_resident), frozen at the v2.0 pin. WHY not fixed: the aarch64 freeze policy (v2.1 keys to x86_64) + the fire being HVF-timing-only (1-in-6, never TCG, unreproducible on the linux KVM/TCG venue). Reopener: aarch64 reactivation, or a live early-boot pipe-read fault on any active venue. | 2026-11-06T06:30:00.000Z | 2026-11-22T19:03:17.039Z |
| 51 | 14 | stub | r9:x86_64/src/trap.rs |  | 14-02: syscall dispatch body is a print+panic fillable stub (plan-declared; 14-04 replaces the body, not the shape) | fixed |  | 2026-11-08T19:38:43.486Z | 2026-11-08T20:57:49.236Z |
| 52 | 14 | stub | r9:x86_64/src/trap.rs |  | 14-02: touser naked fn has no caller yet (14-05 exec_bootstrap is the handoff consumer) | open |  | 2026-11-08T19:38:43.605Z |  |
| 53 | 14 | deviation | r9/x86_64/src/exec.rs |  | x86_64 exec maps text EAGERLY (no trap-context pager this era) — demand text + deeper stack/tos growth beyond the two seed pages arrive with the pager (14-04) | open |  | 2026-11-08T20:57:10.410Z |  |
| 54 | 14 | deviation | r9/x86_64/src/syscall.rs |  | NSEC(53) returns the loud no-RTC errstr on x86_64 — no epoch clock this era; the nsec_now seam is ready for one (14-04) | open |  | 2026-11-08T20:57:10.551Z |  |
| 55 | 14 | deviation | r9/x86_64/src/l.S |  | the boot PML4's slot-0 identity window is dead weight post-14-05 (the linked-VA entry fix left it never-entered; retire it in a one-line l.S cleanup) | open |  | 2026-11-08T22:55:27.720Z |  |
| 56 | 14 | deviation | r9/flake.nix |  | boot-iso FOD outputHash stale since 14-01 (kernel-byte drift; proven pre-existing at abee15c) — 14-07 re-wires with a fresh byte-identity proof | fixed |  | 2026-11-08T22:55:27.857Z | 2026-11-09T16:38:25.859Z |
| 57 | 14 | deviation | build/x86_64/stage-build.sh |  | the sudoless userns venue is CI/agent-only machinery; a present user's documented venue stays sudo-unshare (nsrun) — fold the adapter into nsrun.sh if a third consumer appears | open |  | 2026-11-08T22:55:27.978Z |  |
| 59 | 14 | unmet-truth | .planning/phases/14-harvey-amd64-userspace-on-r9/14-06-SUMMARY.md |  | 14-06 open: two-proc scheduler corruption in r9 x86_64 (CURRENT_PROC := &SCHED_CONTEXT by a wild bss write at init's fexec fork; the q35shell gate red just past 'init: starting /bin/rc') — evidence + leading theory in 14-06-SUMMARY Known Issues; the fix is the one open item before 14-07 | fixed |  | 2026-11-09T01:27:39.799Z | 2026-11-09T05:13:38.890Z |
| 60 | 14 | deviation | r9/flake.nix |  | 14-VERIFICATION gap fix: the boot-iso FOD re-staled AGAIN at the phase-close pin — the phase's own post-fe39d83 kernel commits (8a338a6/e5def72/e417660) changed kernel bytes after the 14-07 re-wire, and the 14-EVIDENCE regression row claimed a green never run at that rev. Re-wired at r9@4132e25 with the two-derivation proof (fresh-kernel clones byte-identical: file bd0281bb…, NAR 9706c34a…=sha256-lwbDSqYx…), nix build .#boot-iso + --rebuild green, pushed, harvey pin f6622b35f. STANDING RULE: any r9 kernel-byte commit re-stales the FOD — re-wire-then-pin at every close; never record the FOD green unrun at the pin | fixed |  | 2026-11-09T16:38:25.739Z | 2026-11-09T16:38:46.914Z |
| 61 | 15 | todo | sys/src/libmemdraw/draw.c | 943 | rgba word-lane fast path (MUL0123+MUL0123, delta==4 channels e.g. r8g8b8a8) still double-rounds — same imprecision family as the fixed byte path; no reproducer in the default r8g8b8 drawtest run; plan9port CALC42 is the reference shape (15-09 scope boundary) | open |  | 2026-11-10T06:04:08.256Z |  |
| 62 | 15 | deviation | sys/src/cmd/samterm |  | samterm input-state fragility around file exchanges (menu-New which/Hcurrent + non-empty-dot cut-stall) — rig works around it; a real fix owns samterm's io.c/main.c | open |  | 2026-11-10T14:17:53.759Z |  |
| 63 | 15 | deviation | sys/src/libc/amd64/main9.s |  | fall-off-main exits("main") — a failure status to rc conditionals (arctest measured); rejoin gates on completion output instead | open |  | 2026-11-10T14:17:53.850Z |  |
| 64 | 15 | deviation | sys/src/cmd/paqfs/mkpaqfs.c |  | mkpaqfs silently DROPS files past blocksize^2/4 (4 MiB at the 4096 default: warn, exit 0) — the 15-07 gs bake asserts the fit and fails on diagnostics; every other bake script rides the default unprotected | open |  | 2026-11-10T18:10:52.108Z |  |
| 65 | 15 | deviation | sys/src/cmd/rc/plan9.c |  | rc Execute prints only the LAST path candidate's errstr, masking the real exec failure behind the trailing-dot candidate — guest sessions invoke by absolute path; a fix owns plan9.c | fixed | harvey 6779df7d7 (Execute keeps the FIRST path candidate's errstr — a candidate cerrstr seeded on the first failure and preserved across later candidates, so the trailing candidate's "can't open" no longer masks the real exec failure; the allocat/" full"/i-o-error break arms still take the stopping reason verbatim per the function's own comment contract) + boot-r9-smoke-x64 / boot-r9-tier1-x64 re-green ALL STAGES PASS at the 17-05 pin (r9 59d28fa) with the fixed rc baked | 2026-11-10T18:10:52.208Z | 2026-11-13T02:43:07.000Z |
| 66 | 16 | deviation | .planning/phases/16-host-net-modes-networking-exit-gate/deferred-items.md |  | 16-08: intermittent r9 x86_64 guest triple fault under the soak burst leg (8-way multi-waiter devip load; 1 failure in 5 runs, observer-healed; root cause open — recorded with the diagnostic toolkit + stress-loop shape in the phase deferred-items) 17-07 ENRICHED (17-CR07-ROOTCAUSE.md; the row stays OPEN): the class reproduced LIVE 3x on the uninstrumented pinned kernel (2 pre-fix at r9 59d28fa + 1 post-fix at r9 8f3ff11 — the deregister fix REFUTED as the cure by the stress series at the fix pin), always ~25s in the burst window, CPU reset with NO kernel trap dump (silent shutdown, triple-fault class); the load-narrowing matrix (30/30 green) localizes the class to the FULL 8-way concurrent bulk transfer (the launch storm alone and <=4-way full-depth never fire); the recorded candidates C1/C2/C3 exonerated structurally; the KVM capture ceiling established empirically (-d int blind at death, cpu_reset post-state only, userspace-APIC refused, TCG-tap wedges at ping). Reopener: a venue that reaches the faulting context host-side or non-healing instrumentation; the committed boot-r9-stress-x64 instrument fires the class at will in ~25s | waived | 17-08 maintainer disposition (3-part D-15, ratified at the 17-08 checkpoint by Neil Skinner 2026-11-13T14:42:08Z): WHERE the full 8-way concurrent bulk-transfer exception-delivery window (17-CR07-ROOTCAUSE section 4 — transient corruption of exception-delivery state, silent-reset class). WHY not fixed: the cure is venue-blocked — the KVM capture ceiling is an established platform fact (section 2, completed by section 8's TCG x slirp avenue) and the strongest-candidate fix (r9 d4a5d38) was refuted as the cure by the N=25 series at the fix pin; the class stays live (~8%/run under the committed instrument). Reopener: a host-side venue that reaches the faulting context, or instrumentation that does not heal the window — boot-r9-stress-x64 fires the class at will in ~25s. Owner 2026-11-22: to be fixed on an x86_64 host venue. | 2026-11-11T20:04:55.434Z | 2026-11-13T14:42:08Z |
| 67 | 17 | deviation | build/x86_64/root-tree/lib/churn-cats.rc |  | rc userland malloc pool corrupts under sustained backquote churn (~428+ iterations, poisoned-header signature) — found by 17-02's composed churn leg; repro committed; deferred to 17-03's rc rows | open |  | 2026-11-12T21:27:16.731Z |  |
| 68 | 17 | deviation | build/x86_64/services-r9-x64.log | 132 | services-x64 gate faults at the dns stage: kernel NULL-call (PF pc=0x0) after devmnt cs error — pre-existing, A/B-proven identical at pins b02408cad and 02218e88c; last green services run was the 15-08 era; root cause open (17-04-deferred-items.md) | open |  | 2026-11-12T23:34:20.636Z |  |
| 69 | 17 | deviation | build/x86_64/tier1-sweep.rc |  | 17-05: the tier1 gate's date check passes on a pre-existing display quirk, not a real epoch — hdate -n == 0 every r9-x64 kernel this era (r9 devcons serves no /dev/bintime; libc nsec() returns 0 on the failed open) and the sweep's *20[0-9][0-9]* expectation is met only because ctime(0) under the EST timezone data displays 'Feb 7 2006'; identical at r9 63bfda9 (pre) and 59d28fa (post), live-measured both sides; root fix belongs to the clock-seam owner (Phase-14's no-RTC era shape) | open |  | 2026-11-13T01:59:58.328Z |  |
| 70 | 20 | todo | r9:aarch64/src/proc.rs | 1753 | aarch64 sched_loop resumes a Ready proc that is not blocked_in_kernel (preempted at EL0, or a fork child's first run) with a bare trap_return (Resume::Return) — no note boundary on that return to EL0, so a note posted while the proc was off the CPU waits for its next tick, syscall or demand fault (the fault-return boundary landed in r9 5f7c4e1). x86_64 has no such path (a preempted proc resumes inside its own IRQ trap()). Deferred, needs its own design: delivering, or the no-handler kill, from sched-loop context (scheduler stack, PROC_TABLE lock, exit_tail re-entering the scheduler). Found by debug session arm64-fault-return-notes; the fpprobe note-fault round's criterion (no later than the first fault) stays valid when it lands. Deferred by the owner to the SMP work. | open |  | 2026-11-22T08:30:00.000Z |  |
| 71 | 20 | deviation | r9:x86_64/src/cpu.rs | 50 | x86_64 SYSCALL entry keeps user RFLAGS bits: IA32_FMASK = IF only (cpu.rs fmask(), documented D-02a) and syscallentry has no cld, so a user's TF (single-step #DB at the first kernel instruction, on the still-user stack), DF (the Rust kernel assumes DF=0; forward rep movs would copy backwards), NT (set inside a note handler it reaches the NOTED return's IRETQ, r9 0ffcec6, with the kernel's RFLAGS.NT=1: #GP(0) in IA-32e mode, kernel halt) and AC reach the kernel. Not reproduced (audit against SDM/QEMU semantics). Options: A widen FMASK to IF+TF+DF+NT+AC (reverses D-02a; user flags still return via R11/the frame; the only option that stops the TF #DB), B keep D-02a and reset flags in the entry stub (cld, clear NT/AC/TF; cannot stop the TF #DB). Either changes x86 bytes (x64 container gates + boot-iso hash). DEFERRED by the owner. Found by debug session noted-psr-trust | fixed | r9 10f675e (IA32_FMASK = IF\|TF\|DF\|NT\|AC, the Linux set — option A, owner 2026-11-22; reverses D-02a's If-only value, recorded in x86_64/src/cpu.rs fmask()'s doc) + 116bcc2 (boot-iso hash). Reproduced first: on e6e5220 Harvey's amd64 fpprobe flags leg (c15b1bbab) halts the kernel with `trap GP` at trap_return's iretq while ring 0 holds the user's NT — NT set in a handler before noted, NT in the saved Ureg, and a blocking syscall whose NT reached another process's IRETQ through swtch (which carries no RFLAGS); with the other two fixes in place a FMASK=IF mutant gives `trap DB` at syscallentry's first instruction with the user RSP; a DIAG kernel shows AC in ring 0 at syscall entry on e6e5220 and no TF/DF/NT/AC on the fix. TCG cannot show the SYSCALL-DF half (QEMU drops DF on SYSCALL itself), so host tests cpu::tests::{fmask_clears_the_flags_the_kernel_must_not_inherit, fmask_is_exactly_if_tf_df_nt_ac} pin every bit. NOTED's IRETQ needs no NT clearing: its #GP reads the kernel's live NT, which no kernel context can hold after the fix. x64 tier-1 (39 T1-OK, fpprobe ok) / auth / net-smoke green at 116bcc2. Debug session .planning/debug/resolved/x86-syscall-entry-flags.md; owner confirmed fixed 2026-11-22; r9 116bcc2 pushed by the coordinator. | 2026-11-22T09:40:00.000Z | 2026-11-22T20:34:39.922Z |
| 72 | 20 | deviation | r9:x86_64/src/trap.rs | 335 | x86_64 interrupt/exception entry keeps the user's DF: the IDT gates clear TF, NT and RF (and IF) but architecturally keep DF and AC, and intrcommon ran no cld, so an interrupt taken while ring 3 held DF (amd64 libc memmove's backward copy runs STD ... CLD) ran trap(), the IRQ handlers and note delivery with every compiler_builtins rep movs/stos copying backwards (the Rust kernel relies on SysV DF=0). Reproduced on r9 e6e5220 by fpprobe flags-dfirq: kernel `trap PF` in MCSLock::unlock (a smashed stack LockNode) with DF set in the kernel flags. AC stays inert at CPL0 until SMAP. Found by debug session x86-syscall-entry-flags (WINDOWS #71 sibling) | fixed | r9 6f8de40 (cld first in intrcommon; a CLAC note for SMAP). Red on e6e5220: fpprobe flags-dfirq → kernel `trap PF` (smashed stack LockNode, DF in the kernel flags); green at 116bcc2; the no-cld mutant reds it again; host test trap::tests::intrcommon_clears_df_before_calling_trap. Debug session .planning/debug/resolved/x86-syscall-entry-flags.md; owner confirmed fixed 2026-11-22; r9 116bcc2 pushed by the coordinator. | 2026-11-22T20:34:40.116Z | 2026-11-22T20:34:58.042Z |
| 73 | 20 | deviation | r9:x86_64/src/trap.rs | 279 | x86_64 intrspecial (the #DB, NMI, #BP and #DF entry) chose its SWAPGS by comparing RDGSBASE with the Mach, but RDGSBASE needs CR4.FSGSBASE, which the boot CPU never sets (l.S: TCG qemu64 cannot), so the instruction raised #UD inside the entry and every such exception ended in a kernel `trap UD` halt — including a ring-3 single-step (POPFQ TF) or int3, a user-triggerable kernel halt, and the legitimate ring-3 #DB that the #71 FMASK fix produces. Reproduced on r9 e6e5220 by fpprobe flags-tf: kernel `trap UD` at intrspecial's rdgsbaseq. Found by debug session x86-syscall-entry-flags (WINDOWS #71 sibling) | fixed | r9 f1cd71b (intrspecial reads IA32_GS_BASE with rdmsr, rax/rcx/rdx saved, the same KMACH comparison). Red on e6e5220: fpprobe flags-tf → kernel `trap UD` at intrspecial's rdgsbaseq; green at 116bcc2 (the ring-3 single-step kills only the child, `sys: trap`); the rdgsbase mutant reds it again; host test trap::tests::entry_asm_uses_no_fsgsbase_instruction. Debug session .planning/debug/resolved/x86-syscall-entry-flags.md; owner confirmed fixed 2026-11-22; r9 116bcc2 pushed by the coordinator. | 2026-11-22T20:34:40.291Z | 2026-11-22T20:34:58.236Z |

````json
[
  {
    "id": 1,
    "kind": "deviation",
    "phase": "02",
    "file": "sys/src/libmach/executable.c",
    "line": null,
    "description": "02-02: 9c host probe substituted with direct host-clang (plan9port header shadowing, A3 fallback); 7.c/7obj.c compile proofs deferred to 02-06/02-03 by plan design",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-31T17:02:41.072Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 2,
    "kind": "deviation",
    "phase": "02",
    "file": "sys/src/libmach/executable.c",
    "line": null,
    "description": "02-02: 9c host probe substituted with direct host-clang (plan9port header shadowing, A3 fallback); 7.c/7obj.c compile proofs deferred to 02-06/02-03 by plan design",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-10-31T17:02:46.189Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 3,
    "kind": "deviation",
    "phase": "02",
    "file": "sys/src/cmd/pcc.c",
    "line": null,
    "description": "02-06 verify(a): pcc functional proof relocated to lima guest — pcc.c execs /bin/cpp + /bin/7c absolute; darwin /bin SIP-sealed (pre-existing); proof passed in guest (PCC-PROOF-PASS)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-01T15:15:57.249Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 4,
    "kind": "deviation",
    "phase": "03",
    "file": ".planning/phases/03-r9-qemu-virt-platform-bring-up/03-01-PLAN.md",
    "line": 101,
    "description": "03-01 gate-2 toolchain probe (rustc --version | grep nightly-2026-01-30) unsatisfiable — rustc prints source-commit date; corrected to store-path identity proof (rust-default-1.95.0-nightly-2026-01-30). See 03-01-SUMMARY deviation 4.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-01T22:33:17.552Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 5,
    "kind": "deviation",
    "phase": "03",
    "file": "xtask/src/main.rs",
    "line": null,
    "description": "03-03 raspi3b boot-smoke gate satisfied by substituted A/B evidence (human-approved); upstream raspi3b/qemu-11.1.2 defect deferred",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-01T23:15:26.857Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 6,
    "kind": "deviation",
    "phase": "03",
    "file": "r9/aarch64/src/main.rs",
    "line": null,
    "description": "03-04: verify block-2's 'mkdir -p ... && qemu ... &' binds & to the whole list, so $QP names a subshell, not qemu; with a stale log the first poll (i=1) reads pre-boot content and the kill orphans qemu mid-boot. Gate passes deterministically from a fresh log (rm first). 03-05's D-11 boot-test socket chardev avoids this pattern. (File ref is r9-repo-relative: ../r9)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-01T23:44:10.289Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 7,
    "kind": "unrun-verify",
    "phase": "03",
    "file": "r9/aarch64/src/trap.rs",
    "line": null,
    "description": "03-06: trap.rs sync dump path gained 'kernel halted' + wfi park on ALL platforms; verified live on virt (virtpanic evidence) but compile-only on raspi — raspi boot verification is blocked by the known upstream raspi-on-qemu-11.1.2 defect (03-03/03-04 deferred, WINDOWS #5). Behavior change is dump-path-only. (File ref is r9-repo-relative: ../r9)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-02T00:33:39.701Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 8,
    "kind": "deviation",
    "phase": "04",
    "file": ".planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-05-PLAN.md",
    "line": null,
    "description": "04-05: plan's Task-2 raw-qemu verify leg written against the pre-amendment -initrd mechanism (falsified 04-01, D-01 AMENDED); executed with the amended loader+patched-DTB delivery, grep chain unchanged",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-02T13:58:26.801Z",
    "resolved_at": "2026-11-02T13:58:40.779Z",
    "milestone": null
  },
  {
    "id": 9,
    "kind": "deviation",
    "phase": "04",
    "file": ".planning/phases/04-r9-userspace-exec-harvey-r9-abi/04-05-PLAN.md",
    "line": null,
    "description": "test-shape-probe",
    "status": "waived",
    "reason": "Accidental probe entry appended while checking the windows CLI output shape (executor error, no content); the real entry is #8.",
    "recorded_at": "2026-11-02T13:58:30.416Z",
    "resolved_at": "2026-11-02T13:58:40.990Z",
    "milestone": null
  },
  {
    "id": 10,
    "kind": "deviation",
    "phase": "04",
    "file": ".planning/phases/04-r9-userspace-exec-harvey-r9-abi/deferred-items.md",
    "line": null,
    "description": "04-07: exec_bootstrap IRQ-masked until first EL0 entry (heisenbug exposure closed by construction; root cause = deferred-items #4)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-02T18:41:05.291Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 11,
    "kind": "stub",
    "phase": "05",
    "file": "r9:port/src/devcons.rs",
    "line": null,
    "description": "cons reads return loud -1 until 05-08 wires the PL011 RX ring through the device (plan-mandated; open succeeds)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T05:59:51.093Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 12,
    "kind": "stub",
    "phase": "05",
    "file": "r9:port/src/devproc.rs",
    "line": null,
    "description": "devproc directory reads + status/args absent this plan; 05-08/05-09 grow them (ps needs /proc/*/status)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T05:59:51.314Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 13,
    "kind": "deviation",
    "phase": "05",
    "file": "r9:aarch64/src/exec.rs",
    "line": null,
    "description": "exec maps data+bss eagerly (cold syscall-arg residency) — D-09-letter adjustment, T-05-06 honored; phase-04 deferred #3 closed for the argument class",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T05:59:51.532Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 14,
    "kind": "stub",
    "phase": "05",
    "file": "r9:aarch64/src/syscall.rs",
    "line": null,
    "description": "CREATE(22) stays loud-unimplemented until 05-06 wires sys_create; devenv's create op is host-tested and slots in unchanged (init's setenv/cpenv path); plan-mandated ordering (05-05 Task 1 action step 1)",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-03T06:32:58.158Z",
    "resolved_at": "2026-11-03T08:27:38.113Z",
    "milestone": null
  },
  {
    "id": 15,
    "kind": "deviation",
    "phase": "05",
    "file": "r9:aarch64/src/proc.rs",
    "line": null,
    "description": "05-05: plan's RFENVG/RFCENVG bullet inverted vs fork(2)+9k sysproc.c — donor governs (neither=share, RFENVG=copy, RFCENVG=empty); the 05-04 RFNAMEG/RFCNAMEG precedent; pinned by devenv_test.rs + the env-driven boot gate",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T06:32:58.375Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 16,
    "kind": "deviation",
    "phase": "05",
    "file": "r9:aarch64/src/proc.rs",
    "line": null,
    "description": "05-06: plan's fd-group bullet inverts RFFDG vs default vs fork(2)+9k sysproc.c (neither=SHARE, RFFDG=COPY, RFCFDG=empty) — the phase's third flag inversion; donor governs (libc fork()=RFFDG=COPY is what connectembed's dup/close needs); pinned by fdgroup_test.rs",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T08:27:38.289Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 17,
    "kind": "deviation",
    "phase": "05",
    "file": "r9:aarch64/src/fault.rs",
    "line": null,
    "description": "05-06: cold-syscall-buffer class fixed at the gate (EL1h DataAbort on paqfs's emalloc'd brk_-grown buffer — the kernel's pread made the first touch) — fault::ensure_user_resident generalizes 04-08's explicit page-in to any writable user buffer before deref; T-05-06 holds (never a nested fault); phase-04 deferred #3 closed for the growth class",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T08:27:38.460Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 18,
    "kind": "todo",
    "phase": "05",
    "file": "r9:port/src/namec.rs",
    "line": null,
    "description": "05-07: pgrp lock held across namec walks incl. devmnt's blocking Twalk RPCs — a pgrp-SHARING contender (fork shares the pgrp) spins IRQ-masked = wedge once the first concurrent namespace caller exists (rc pipelines/backgrounded cmds); the gate never contends (strictly sequential chain). Fix: snapshot the union chain under a brief lock, walk unlocked (phase deferred-items.md)",
    "status": "fixed",
    "reason": "r9 59d28fa + the 17-05 four-site audit (verify+pin: all four pgrp-lock sites in x86_64/src/syscall.rs conform to the snapshot-then-drop contract — sys_bind :527 / sys_unmount :599 / sys_mount :716 lock the mount-table mutation only and drop(guard) before any close dispatch; sys_create :1159 is the chain_snapshot block shape verbatim with redirect_clone running outside it) + the 17-02 contention captures (the q35churn profile's 39 RFMEM semacquire/semrelease generations + the composed /bin/cat x1400 fork+exec churn, both green twice — the rc-pipeline/backgrounded-command contender shape, no wedge)",
    "recorded_at": "2026-11-03T10:19:28.856Z",
    "resolved_at": "2026-11-13T01:47:22.000Z",
    "milestone": null
  },
  {
    "id": 19,
    "kind": "todo",
    "phase": "05",
    "file": "r9:port/src/devcons.rs",
    "line": null,
    "description": "05-07: #c/pid absent — getpid() fails, init's pri-10 write names #p/0/ctl (the designed warning fires either way; the gate is unaffected). 05-09's devproc ctl is the first real consumer — add devcons's Qpid file (donor devcons.c) via a current-pid hook before it lands",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T10:19:29.077Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 20,
    "kind": "deviation",
    "phase": "05",
    "file": "build/aarch64/drive-r9.expect",
    "line": null,
    "description": "interactive-driver input wedge flake: 2 of ~7 early sweep runs wedged at a prompt read (line never completes); 6 consecutive clean passes since; not reproducing under the cons watchdog; deferred as the top debug candidate for 05-12/05-13's CI leg",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T15:09:13.138Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 21,
    "kind": "stub",
    "phase": "05",
    "file": ".planning/phases/05-shell-boot-one-command-integration/deferred-items.md",
    "line": null,
    "description": "r9-side: stat(5) records carry 3 strings (no muid) in devcons/devroot/devenv/devsrv/devdup listings — dirreadall's statcheck walks FOUR and rejects them (devproc fixed 05-09; the rest latent, no dirreadall consumer this era)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T15:09:20.285Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 22,
    "kind": "deviation",
    "phase": "05",
    "file": "build/aarch64/drive-r9.expect",
    "line": null,
    "description": "interactive-driver input wedge flake: 2 of ~7 early sweep runs wedged at a prompt read (line never completes); 6 consecutive clean passes since; not reproducing under the cons watchdog; deferred as the top debug candidate for 05-12/05-13's CI leg",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T15:09:20.518Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 23,
    "kind": "stub",
    "phase": "05",
    "file": ".planning/phases/05-shell-boot-one-command-integration/deferred-items.md",
    "line": null,
    "description": "r9-side: stat records carry 3 strings no muid in devcons/devroot/devenv/devsrv/devdup listings - dirreadall statcheck walks FOUR and rejects them (devproc fixed 05-09)",
    "status": "waived",
    "reason": "accidental duplicate of #21 (executor re-ran the append after a misleading tail display)",
    "recorded_at": "2026-11-03T15:09:25.686Z",
    "resolved_at": "2026-11-03T15:09:56.078Z",
    "milestone": null
  },
  {
    "id": 24,
    "kind": "deviation",
    "phase": "05",
    "file": "build/aarch64/drive-r9.expect",
    "line": null,
    "description": "interactive-driver input wedge flake: 2 of ~7 early sweep runs wedged at a prompt read; 6 consecutive clean passes since; top debug candidate for 05-12/05-13 CI",
    "status": "waived",
    "reason": "accidental duplicate of #20 (executor re-ran the append after a misleading tail display)",
    "recorded_at": "2026-11-03T15:09:25.913Z",
    "resolved_at": "2026-11-03T15:09:56.309Z",
    "milestone": null
  },
  {
    "id": 25,
    "kind": "deviation",
    "phase": "261103-giq",
    "file": "build/harness/src/srv9.rs",
    "line": null,
    "description": "Twstat name offset corrected to 47 (Dir wire shape is doubly size-prefixed); the .py's offset-45 rename was dead code — mv fell back to copy+delete",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T18:24:06.467Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 26,
    "kind": "deviation",
    "phase": "261103-giq",
    "file": "build/harness/src/trace.rs",
    "line": null,
    "description": "trace-syscalls: prompt-substring wait after pid probe closes the .py's latent echo/prompt race",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T18:24:06.688Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 27,
    "kind": "deviation",
    "phase": "261103-giq",
    "file": "build/aarch64/vm-run.sh",
    "line": null,
    "description": "rsync exclusion for build/harness/target added to the four vm-sync scripts",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T18:24:06.904Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 28,
    "kind": "deviation",
    "phase": "261103-giq",
    "file": "build/aarch64/subset-exec.sh",
    "line": null,
    "description": "guest-scratch CONF=qemu bootdir reset to pristine (39 accumulated rows exceeded Nbootfiles=32; pre-existing guest state)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-03T18:24:07.120Z",
    "resolved_at": null,
    "milestone": null
  },
  {
    "id": 29,
    "kind": "deviation",
    "phase": "06",
    "file": "build/aarch64/gethunk-repro-cc.sh",
    "line": null,
    "description": "cc repro input shape changed from plan's >600KB string literal to digit/comma macro define (NSYMB/STRINGSZ caps reject the sketched shape before gethunk); use line dropped (macexpand Io.b 16KB cap)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-04T23:18:26.722Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 30,
    "kind": "deviation",
    "phase": "06",
    "file": "build/aarch64/gethunk-repro-6l.sh",
    "line": null,
    "description": "6l repro input shape settled by A4 spike to >2.5MB archive __.SYMDEF bang-malloc (array toothless, string-literal impossible); venue linux-only (darwin sbrk 4096KB arena); fixed pair built from working tree not hostbin",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-04T23:18:26.940Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 31,
    "kind": "deviation",
    "phase": "07",
    "file": "build/aarch64/root-tree/lib/net-sweep.rc",
    "line": 77,
    "description": "dd syntax corrected to dash-pair form (-if/-of); the baked dd.c rejects classic if=/of= (plan prose corrected, gate-blocking)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T00:04:57.930Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 32,
    "kind": "deviation",
    "phase": "07",
    "file": "build/aarch64/mkimage.sh",
    "line": 165,
    "description": "trailer magic re-extracted via od -j (Harvey host dd shadows POSIX dd on the devshell PATH — D-02 class)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T00:04:58.144Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 33,
    "kind": "deviation",
    "phase": "07",
    "file": "build/aarch64/root-tree/lib/net-sweep.rc",
    "line": 77,
    "description": "dd syntax corrected to dash-pair form (-if/-of); the baked dd.c rejects classic if=/of= (plan prose corrected, gate-blocking)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T00:05:04.202Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 34,
    "kind": "deviation",
    "phase": "07",
    "file": "build/aarch64/mkimage.sh",
    "line": 165,
    "description": "trailer magic re-extracted via od -j (Harvey host dd shadows POSIX dd on the devshell PATH — D-02 class)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T00:05:08.206Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 35,
    "kind": "deviation",
    "phase": "06",
    "file": ".planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-02-PLAN.md",
    "line": 216,
    "description": "06-02 Task-3 verify gate 2 text defective: mk -n never recurses, so the nested-path grep on cmd's own output always counts 0; intent proven by chain probes in 06-02-SUMMARY; repair gate text at 06-03 planning",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T00:10:54.539Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 36,
    "kind": "unrun-verify",
    "phase": "07",
    "file": "r9:aarch64/src/platform/virt/pci.rs",
    "line": null,
    "description": "07-06 D6: virtio IRQ handler armed-and-inert-proven only; traffic-level proof lands with 07-08's NETDRV-04 gates (no traffic source until 07-07)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T01:09:17.807Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 37,
    "kind": "deviation",
    "phase": "06",
    "file": "build/aarch64/coverage-sweep.sh",
    "line": null,
    "description": "06-04: second-chance visitation pass added — plan9port mk recipes abort at the first failing command (measured), so container recursion stops at the first failing sibling; the plan's adjacency truth (ipconfig+6 siblings visited) is otherwise unreachable with tier-0 libs outstanding; pass 2 gives each unvisited in-universe dir its own shimmed guarded mk all",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T02:10:46.163Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 38,
    "kind": "deviation",
    "phase": "06",
    "file": "build/aarch64/coverage-sweep.sh",
    "line": null,
    "description": "06-04: pass-2 initially invoked $REAL_MK directly — a leaf dir runs no nested mk so no VISIT line was logged (run-3 reconciliation failure); pass-2 visits now invoke bare mk through the shim",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T02:10:46.381Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 39,
    "kind": "deviation",
    "phase": "06",
    "file": "build/aarch64/coverage-sweep.sh",
    "line": null,
    "description": "06-04: namespace/devshell Plan 9-tool dialect fixes — sed {/pat/!p} group garbled (host), sed s/|.*$// garbled by hostbin Plan 9 sed (namespace; silently emptied the exclusions in run 2), date -u +fmt prints the epoch under Plan 9 date — awk index/substr + host-precomputed plain-path exclusions + probe-verified ISO date",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T02:10:46.602Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 40,
    "kind": "deviation",
    "phase": "06",
    "file": "build/aarch64/generator-scan.sh",
    "line": null,
    "description": "06-04: ssh tree probes drained the audit-rows while-loop's stdin (only G1 checked on the first run) — all ssh probe sites carry </dev/null now",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T02:10:46.815Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 41,
    "kind": "deviation",
    "phase": "06",
    "file": "build/aarch64/generator-audit.txt",
    "line": null,
    "description": "06-04: plan/RESEARCH cells corrected against live mkfiles (plan-mandated re-confirmation): G5 artifact is o.pcode (cpuobjtype=o via /unix/mkfile O=o); G6 genarch is pcc-built native-only (build guard gs/mkfile:164), never built nor run in the aarch64 sweep",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T02:10:47.035Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 42,
    "kind": "deviation",
    "phase": "06",
    "file": ".planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-05-PLAN.md",
    "line": null,
    "description": "06-05 Task-2's literal no-PENDING-CLASSIFY grep contradicts Task-1's mandated append-only seed lines; ran the intent gate (latest-line-per-dir clean) — gate-text repair filed for the next planning pass",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-05T02:51:20.644Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 43,
    "kind": "unmet-truth",
    "phase": "08",
    "file": "r9:aarch64/src/proc.rs",
    "line": null,
    "description": "08-04: the NETIP-03 ping-127.0.0.1 truth is MET — RFMEM landed per the blocking-human decision (option A, donor dupseg semantics): Data/Bss pages share under RFMEM with PA-keyed refcount-gated frees, RFMEM families carry the shared break (sys_brk_ family arm births grown pages shared — the checkpoint's 'ping does not brk_-grow post-fork' premise was falsified at the gate and the donor's shared-segment growth is the implemented semantics), the rfork bss sweep closes the untouched-page hole, and SEMACQUIRE(37)/SEMRELEASE(38) implemented for the contended libc lock() path; virtip gate green (netip: ping ok + NETIP-DONE), 8/8 regression profiles green, r9 4298fc0",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-05T19:54:26.000Z",
    "resolved_at": "2026-11-05T22:30:00.000Z",
    "milestone": "v2.0"
  },
  {
    "id": 44,
    "kind": "deviation",
    "phase": "08",
    "file": "r9:aarch64/src/proc.rs",
    "line": null,
    "description": "08-05: kernel arena (4 MiB QuickFit) drains under a spin loop of short-lived fork+exec — ~1400 iterations of /bin/cat from a fixture's port-discovery spin exhausted it (oom size=4160 panic); exec staging teardown (ByteBacking::Mnt) is the suspect; fixture-side workaround applied (invoke the baked name so the spin terminates)",
    "status": "fixed",
    "reason": "r9 c4f2f2c + 72edbba (the churn gate's measured #44 root causes: exit_tail's never-dropped transients — the detached fd-group Arc ~8 KiB/exit — plus the Moribund husk's retained group handles/chans; the drain measured 16 KiB/exec pre-fix, 60 bytes/exec post-fix) + the q35churn boot profile + the boot-r9-churn-x64 composed gate (both green twice)",
    "recorded_at": "2026-11-05T23:35:00.000Z",
    "resolved_at": "2026-11-12T21:18:02.000Z",
    "milestone": "v2.0"
  },
  {
    "id": 45,
    "kind": "deviation",
    "phase": "08",
    "file": "r9:port/src/namec.rs",
    "line": null,
    "description": "08-05: Creat on a top-level file of a union-mounted dir misses non-head members — walk_one never consults the union chain (documented shape); rc's `>` is Creat-first so `> /net/log` failed with 'namec: device has no create'; fixture uses `>>` (open-first); the two-level-deep ctl writes unaffected",
    "status": "fixed",
    "reason": "r9 cfe7af0 + 421d5e1 (walk_one_union/create_chan_union: sys_create's final-component probe and the create dispatch consult the union chain per the per-consult chain_snapshot shape — first-accepting member, refusals propagate; the nomount follow-up keeps #-rooted parents off the empty mount-table key the boot's `bind -a /root /` owns — the tier1 gate caught the over-reach, T1-FAIL date from init's silently-failing setenv('#e/timezone')) + the namec_tests host pin (venue A: 6 pins RED_EVIDENCE_OK, incl. the empty-key counterfactual) + tier1/smoke/net-smoke/q35shell gates green at Harvey pin 09b59f6f2 (r9 59d28fa)",
    "recorded_at": "2026-11-05T23:55:00.000Z",
    "resolved_at": "2026-11-13T01:47:22.000Z",
    "milestone": "v2.0"
  },
  {
    "id": 46,
    "kind": "deviation",
    "phase": "08",
    "file": ".planning/phases/08-r9-tcp-ip-path-devip-compatible-net/deferred-items.md",
    "line": null,
    "description": "08-08: rc redirect-stack/fd-reuse corner — a file-open redirect (`> file`) or an in-block `>[2]/dev/null` on a backquote dd INSIDE a subshell block carrying a held-open fd redirect (`} >[6]/mnt/data`) leaks the payload into the held fd's pipe (measured twice in-guest; root cause unpinned — suspect class: doredir re-applies a stale ROPEN record whose from-fd was closed/reused, Dup() return unchecked at rc plan9.c:583); 08-08's aan fixture sidesteps by construction (the drop moved in-band via baked /lib/netip-aan-drop.rc; dd summary rides the console); recorded in the phase's deferred-items.md",
    "status": "fixed",
    "reason": "harvey 6779df7d7 (doredir's BOTH redirect arms — ROPEN and RDUP — check Dup's return; a failed dup now reports \"rc: can't dup N to M: %r\" through the new dupfailed() and exits with errors, so rc never execs against an unset target descriptor) + boot-r9-smoke-x64 / boot-r9-tier1-x64 re-green ALL STAGES PASS at the 17-05 pin (r9 59d28fa) with the fixed rc baked (out/rc == /amd64/bin/rc, sha256 51f81de8…, the re-pinned 38-member witness manifest); no targeted fixture — no sweep fails a dup and the fixture route rides the fd-dup-in-sourced-block hazard (15-02's measured dialect), disproportionate per D-11's escape clause",
    "recorded_at": "2026-11-06T03:05:00.000Z",
    "resolved_at": "2026-11-13T02:43:07.000Z",
    "milestone": "v2.0"
  },
  {
    "id": 47,
    "kind": "deviation",
    "phase": "08",
    "file": "r9:port/src/devip/stack.rs",
    "line": null,
    "description": "08-09: the RX-IRQ wake with MULTIPLE registered devip waiters wedges/panics — the first NIC-RX wake with >1 blocked net proc never reaches the ping reader (measured: the composed net smoke's static leg wedged the guest fully idle on BOTH gicv2+tcg and gicv3+hvf with udpecho+tcpecho blocked in loopback reads; an earlier form panicked at the same stage; the static-alone boot with the identical spawn shape PASSES, and every xtask virtip mode runs =1 waiter). Root cause unpinned. BLOCKER-CLASS for Phase 9's tap venue + Phase 12's cpu service (aan over real net = many blocked net procs); the composed smoke sidesteps by stage order (ether legs first, single-waiter per exchange). Recorded in the phase's deferred-items.md",
    "status": "fixed",
    "reason": "17-07 fold verdict (17-CR07-ROOTCAUSE section 6, D-09 SPLIT): the WEDGE form — closed-by-discipline, independent of the #66 reset class. The fix families it anticipated landed and pinned after the Phase-8 record: re-probe-after-drive in every devip block loop (the Phase-8 gate-caught wakeup-loss class) + CR-03 register-per-iteration loop (r9 75970f0 + x86_64 sem pins f9088d0). The exact #47 shape (multiple readers blocked across loopback+ether, a NIC wake, the composed static leg) is GREEN on x86_64 at every pin since 14-09 (boot-r9-net-smoke-x64 multi-reader echo legs, exit 0 at pin 8f3ff11). Reopener: a multi-waiter WEDGE (idle guest, no reset) on any venue. Owner 2026-11-22: also green on aarch64 — phase 20's arm64 net-smoke multi-reader legs passed on HVF.",
    "recorded_at": "2026-11-06T06:30:00.000Z",
    "resolved_at": "2026-11-13T06:40:00.000Z",
    "milestone": "v2.0"
  },
  {
    "id": 48,
    "kind": "deviation",
    "phase": "08",
    "file": "r9:port/src/devcons.rs",
    "line": null,
    "description": "08-09: a console input burst longer than the PL011-side 256-byte RX ring truncates mid-line with no reader-visible signal — the cooked line never completes and the guest wedges (measured 2/2: a 316-char typed line arrived as 255 chars; wedgedbg showed line=255 stuck). The drop-oldest overrun discipline is intentional; gate-author discipline (short typed lines / paced continuation lines) now encoded in smoke-r9-net.expect's header; a kernel-side 'line truncated' signal is the possible later refinement",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-06T06:30:00.000Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 49,
    "kind": "deviation",
    "phase": "08",
    "file": "sys/src/cmd/rc/exec.c",
    "line": null,
    "description": "08-09: rc's argv-stack-underflow flake residual — 'pushword but no argv!' + abort fired inside baked fixtures (netip-loopback.rc's udp spin; netip-static.rc's captures) AND in the plain 05-12 smoke's typed echo stage, on gicv3+hvf only (2/~6 HVF runs today; 0 TCG runs across the phase's whole gate history; the 06-07 Xbackq fix covered the top-level case — the residual fires in other op sequences; the aborted child's exit status prints <bad utf8>). Pre-existing class, HVF-timing-exposed; the CI/truth venue is TCG. Recorded in the phase's deferred-items.md",
    "status": "open",
    "reason": "harvey 6779df7d7 (the argv-stack invariant restored at exec.c's two pop-then-pushword ops, Xqdol and Xcount — the havefork.c 06-07 Xbackq guard shape, `if(runq->argv==0) pushlist()`; the pushword/popword panic sites themselves untouched) + boot-r9-smoke-x64 / boot-r9-tier1-x64 re-green ALL STAGES PASS at the 17-05 pin; A6 note: the historical fires were HVF-timing-only (darwin, 2/~6 runs; zero on the linux TCG/KVM venue across the whole gate history) — unreproducible on this venue, so the fix is invariant-audit-grounded; reopener: a live \"pushword but no argv\" fire on any venue; REOPENED phase 20 (2026-11-17): research saw 4 fires in 13 HVF boots at the 8e4e730-era pin, and 20-02 measured 15 fires in 54 HVF boots (0 in 9 TCG) at r9 46364d0 with the 6779df7d7 rc guards in place, so the rc-side fix was not the cause. Root cause (20-EVIDENCE.md ## Root cause, confirmed by a PAR_EL1 probe): r9 aarch64 vm::switch wrote TTBR0_EL1 with no isb before tlbi vmalle1is; with no ASIDs a walk still using the old root refilled the outgoing proc's entries, and the fork child ran on its parent's live pages (110 of 114 child pages resolved to the parent's PAs, a process-isolation fault). Re-fixed in r9 cb9cde3 (isb after the TTBR0 and TTBR1 writes), pushed with r9 5f3ddb0 and pinned lock-only at Harvey 980f09c4e; that pin's series, no override, was HVF smoke 1753/1753, net-smoke 36/36, tier-1 10/10 (17860 gate forks, 0 fires), TCG 3/3 each (20-06). REOPENED AGAIN 2026-11-18 (20-06, owner decision A): at the final pin r9 2226cc4 (reproducible kernel sha256 d3537304...), net-smoke HVF failed run 21 of 36: a netip-loopback.rc fork child (pid 49) faulted at addr 0x1 in rc searchpath/strncmp on a corrupt runq->argv word, right after its parent reaped a child - the same class, a fault instead of the pushword abort; 1 fire in 16768 HVF gate forks. The cb9cde3 isb fix is necessary but not shown sufficient; next: dsb ishst hardening of vm::switch (20-03 work), then the series again. Reopener for the next close: any fork-child argv corruption (pushword abort or user fault in rc) on any venue. HARDENED 2026-11-19 (20-06, owner decision b): r9 57e2952 adds dsb ishst before the TTBR0/TTBR1 writes and the full tlbi; pinned lock-only at Harvey 0dafd07ad (kernel-virt sha256 da4ab2da...); its series at the plan's N was green (HVF smoke 1753/1753, net-smoke 36/36, tier-1 10/10, 17864 gate forks, 0 fires; TCG 3/3 each). Still OPEN: the isb-only residual was ~1.8e-5 per fork (1 fire in ~54,600 HVF gate forks), so 17864 clean forks cannot separate the hardened kernel from it. Follow-up to close: about 2,300 net-smoke HVF boots (~166,000 forks, ~9.6 h, stop-on-fail off) clean on the pinned kernel ROOT-CAUSED 2026-11-21 (debug session fork-child-pc-align-fault, resolved; .planning/debug/resolved/): the residual (105d48e pin series net-smoke run 23: rc fork child pid 49 jumped to 0x318e7; the 2226cc4 strncmp-on-0x1 fire; a can't-dup fire with stale stack args, all the same pid-49 backquote fork) is a DIFFERENT mechanism from the TTBR/TLB defect: r9 aarch64 rfork_copy wrote child text through the KZERO data alias with no I-cache maintenance, and with CTR_EL0.DIC=0 the HVF host keeps stale instructions per (VA, PA) across unmap+tlbi+remap (boot self-test: 64/64 stale same-VA, 0/64 other VAs, 64/64 fresh after ic ialluis), so the fork child whose rc simple.c page (VA 0x18000) landed on bind's just-executed page at the same VA ran bind's code (possible only since 8958c9e made frees real). Fixed r9 79c55b1 + regression guard 46a5c87 (TextCopy, the single rfork copy path; boot-time selftest prints icache: rfork text-copy selftest N/64 fresh and panics unless 64/64; HVF kills the maintenance-removed mutant 0/64), both pushed and in the final pin r9 e6e5220. Likely explains the whole post-cb9cde3 residual; the 2,300-boot follow-up is superseded by the deterministic guard. Close at the pin series on the pushed fix (step A, riding with the separate FP/SIMD save/restore fix).",
    "recorded_at": "2026-11-06T06:30:00.000Z",
    "resolved_at": null,
    "milestone": "v2.0"
  },
  {
    "id": 50,
    "kind": "deviation",
    "phase": "08",
    "file": "r9:port/src/devpipe.rs",
    "line": null,
    "description": "08-09: intermittent early-boot EL1 DataAbort (translation fault level 3) in devpipe::read (devpipe.rs:282, the user-buffer copy) at FAR 0x4e428 — fired once during the composed smoke's HVF leg in the boot chain right after 'version...' (the boot(8) fauth leg), 1/~6 HVF runs, never on TCG; the same tree re-ran green end-to-end. The cold-syscall-buffer residence machinery (05-06's ensure_user_resident) is the suspect area — an intermittent miss in the early-boot pipe read path. Recorded in the phase's deferred-items.md",
    "status": "fixed",
    "reason": "Owner 2026-11-22: evidence shows as fixed. The waiver's reopener (aarch64 reactivation) fired in phase 20, and no devpipe::read early-boot abort appeared across phase 20's HVF series and confidence passes (tens of thousands of HVF boots; see .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md). Prior waiver (history, 2026-11-13): 17-07 fold verdict (17-CR07-ROOTCAUSE section 6, D-09 SPLIT): a DIFFERENT CLASS, split out with its own 3-part disposition. WHERE: the aarch64 devpipe read path + the cold-syscall-buffer residence machinery (ensure_user_resident), frozen at the v2.0 pin. WHY not fixed: the aarch64 freeze policy (v2.1 keys to x86_64) + the fire being HVF-timing-only (1-in-6, never TCG, unreproducible on the linux KVM/TCG venue). Reopener: aarch64 reactivation, or a live early-boot pipe-read fault on any active venue.",
    "recorded_at": "2026-11-06T06:30:00.000Z",
    "resolved_at": "2026-11-22T19:03:17.039Z",
    "milestone": "v2.0"
  },
  {
    "id": 51,
    "kind": "stub",
    "phase": "14",
    "file": "r9:x86_64/src/trap.rs",
    "line": null,
    "description": "14-02: syscall dispatch body is a print+panic fillable stub (plan-declared; 14-04 replaces the body, not the shape)",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-08T19:38:43.486Z",
    "resolved_at": "2026-11-08T20:57:49.236Z",
    "milestone": "v2.1"
  },
  {
    "id": 52,
    "kind": "stub",
    "phase": "14",
    "file": "r9:x86_64/src/trap.rs",
    "line": null,
    "description": "14-02: touser naked fn has no caller yet (14-05 exec_bootstrap is the handoff consumer)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-08T19:38:43.605Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 53,
    "kind": "deviation",
    "phase": "14",
    "file": "r9/x86_64/src/exec.rs",
    "line": null,
    "description": "x86_64 exec maps text EAGERLY (no trap-context pager this era) — demand text + deeper stack/tos growth beyond the two seed pages arrive with the pager (14-04)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-08T20:57:10.410Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 54,
    "kind": "deviation",
    "phase": "14",
    "file": "r9/x86_64/src/syscall.rs",
    "line": null,
    "description": "NSEC(53) returns the loud no-RTC errstr on x86_64 — no epoch clock this era; the nsec_now seam is ready for one (14-04)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-08T20:57:10.551Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 55,
    "kind": "deviation",
    "phase": "14",
    "file": "r9/x86_64/src/l.S",
    "line": null,
    "description": "the boot PML4's slot-0 identity window is dead weight post-14-05 (the linked-VA entry fix left it never-entered; retire it in a one-line l.S cleanup)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-08T22:55:27.720Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 56,
    "kind": "deviation",
    "phase": "14",
    "file": "r9/flake.nix",
    "line": null,
    "description": "boot-iso FOD outputHash stale since 14-01 (kernel-byte drift; proven pre-existing at abee15c) — 14-07 re-wires with a fresh byte-identity proof",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-08T22:55:27.857Z",
    "resolved_at": "2026-11-09T16:38:25.859Z",
    "milestone": "v2.1"
  },
  {
    "id": 57,
    "kind": "deviation",
    "phase": "14",
    "file": "build/x86_64/stage-build.sh",
    "line": null,
    "description": "the sudoless userns venue is CI/agent-only machinery; a present user's documented venue stays sudo-unshare (nsrun) — fold the adapter into nsrun.sh if a third consumer appears",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-08T22:55:27.978Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 59,
    "kind": "unmet-truth",
    "phase": "14",
    "file": ".planning/phases/14-harvey-amd64-userspace-on-r9/14-06-SUMMARY.md",
    "line": null,
    "description": "14-06 open: two-proc scheduler corruption in r9 x86_64 (CURRENT_PROC := &SCHED_CONTEXT by a wild bss write at init's fexec fork; the q35shell gate red just past 'init: starting /bin/rc') — evidence + leading theory in 14-06-SUMMARY Known Issues; the fix is the one open item before 14-07",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-09T01:27:39.799Z",
    "resolved_at": "2026-11-09T05:13:38.890Z",
    "milestone": "v2.1"
  },
  {
    "id": 60,
    "kind": "deviation",
    "phase": "14",
    "file": "r9/flake.nix",
    "line": null,
    "description": "14-VERIFICATION gap fix: the boot-iso FOD re-staled AGAIN at the phase-close pin — the phase's own post-fe39d83 kernel commits (8a338a6/e5def72/e417660) changed kernel bytes after the 14-07 re-wire, and the 14-EVIDENCE regression row claimed a green never run at that rev. Re-wired at r9@4132e25 with the two-derivation proof (fresh-kernel clones byte-identical: file bd0281bb…, NAR 9706c34a…=sha256-lwbDSqYx…), nix build .#boot-iso + --rebuild green, pushed, harvey pin f6622b35f. STANDING RULE: any r9 kernel-byte commit re-stales the FOD — re-wire-then-pin at every close; never record the FOD green unrun at the pin",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-11-09T16:38:25.739Z",
    "resolved_at": "2026-11-09T16:38:46.914Z",
    "milestone": "v2.1"
  },
  {
    "id": 61,
    "kind": "todo",
    "phase": "15",
    "file": "sys/src/libmemdraw/draw.c",
    "line": 943,
    "description": "rgba word-lane fast path (MUL0123+MUL0123, delta==4 channels e.g. r8g8b8a8) still double-rounds — same imprecision family as the fixed byte path; no reproducer in the default r8g8b8 drawtest run; plan9port CALC42 is the reference shape (15-09 scope boundary)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-10T06:04:08.256Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 62,
    "kind": "deviation",
    "phase": "15",
    "file": "sys/src/cmd/samterm",
    "line": null,
    "description": "samterm input-state fragility around file exchanges (menu-New which/Hcurrent + non-empty-dot cut-stall) — rig works around it; a real fix owns samterm's io.c/main.c",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-10T14:17:53.759Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 63,
    "kind": "deviation",
    "phase": "15",
    "file": "sys/src/libc/amd64/main9.s",
    "line": null,
    "description": "fall-off-main exits(\"main\") — a failure status to rc conditionals (arctest measured); rejoin gates on completion output instead",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-10T14:17:53.850Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 64,
    "kind": "deviation",
    "phase": "15",
    "file": "sys/src/cmd/paqfs/mkpaqfs.c",
    "line": null,
    "description": "mkpaqfs silently DROPS files past blocksize^2/4 (4 MiB at the 4096 default: warn, exit 0) — the 15-07 gs bake asserts the fit and fails on diagnostics; every other bake script rides the default unprotected",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-10T18:10:52.108Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 65,
    "kind": "deviation",
    "phase": "15",
    "file": "sys/src/cmd/rc/plan9.c",
    "line": null,
    "description": "rc Execute prints only the LAST path candidate's errstr, masking the real exec failure behind the trailing-dot candidate — guest sessions invoke by absolute path; a fix owns plan9.c",
    "status": "fixed",
    "reason": "harvey 6779df7d7 (Execute keeps the FIRST path candidate's errstr — a candidate cerrstr seeded on the first failure and preserved across later candidates, so the trailing candidate's \"can't open\" no longer masks the real exec failure; the allocat/\" full\"/i-o-error break arms still take the stopping reason verbatim per the function's own comment contract) + boot-r9-smoke-x64 / boot-r9-tier1-x64 re-green ALL STAGES PASS at the 17-05 pin (r9 59d28fa) with the fixed rc baked",
    "recorded_at": "2026-11-10T18:10:52.208Z",
    "resolved_at": "2026-11-13T02:43:07.000Z",
    "milestone": "v2.1"
  },
  {
    "id": 66,
    "kind": "deviation",
    "phase": "16",
    "file": ".planning/phases/16-host-net-modes-networking-exit-gate/deferred-items.md",
    "line": null,
    "description": "16-08: intermittent r9 x86_64 guest triple fault under the soak burst leg (8-way multi-waiter devip load; 1 failure in 5 runs, observer-healed; root cause open — recorded with the diagnostic toolkit + stress-loop shape in the phase deferred-items) 17-07 ENRICHED (17-CR07-ROOTCAUSE.md; the row stays OPEN): the class reproduced LIVE 3x on the uninstrumented pinned kernel (2 pre-fix at r9 59d28fa + 1 post-fix at r9 8f3ff11 — the deregister fix REFUTED as the cure by the stress series at the fix pin), always ~25s in the burst window, CPU reset with NO kernel trap dump (silent shutdown, triple-fault class); the load-narrowing matrix (30/30 green) localizes the class to the FULL 8-way concurrent bulk transfer (the launch storm alone and <=4-way full-depth never fire); the recorded candidates C1/C2/C3 exonerated structurally; the KVM capture ceiling established empirically (-d int blind at death, cpu_reset post-state only, userspace-APIC refused, TCG-tap wedges at ping). Reopener: a venue that reaches the faulting context host-side or non-healing instrumentation; the committed boot-r9-stress-x64 instrument fires the class at will in ~25s",
    "status": "waived",
    "reason": "17-08 maintainer disposition (3-part D-15, ratified at the 17-08 checkpoint by Neil Skinner 2026-11-13T14:42:08Z): WHERE the full 8-way concurrent bulk-transfer exception-delivery window (17-CR07-ROOTCAUSE section 4 — transient corruption of exception-delivery state, silent-reset class). WHY not fixed: the cure is venue-blocked — the KVM capture ceiling is an established platform fact (section 2, completed by section 8's TCG x slirp avenue) and the strongest-candidate fix (r9 d4a5d38) was refuted as the cure by the N=25 series at the fix pin; the class stays live (~8%/run under the committed instrument). Reopener: a host-side venue that reaches the faulting context, or instrumentation that does not heal the window — boot-r9-stress-x64 fires the class at will in ~25s. Owner 2026-11-22: to be fixed on an x86_64 host venue.",
    "recorded_at": "2026-11-11T20:04:55.434Z",
    "resolved_at": "2026-11-13T14:42:08Z",
    "milestone": "v2.1"
  },
  {
    "id": 67,
    "kind": "deviation",
    "phase": "17",
    "file": "build/x86_64/root-tree/lib/churn-cats.rc",
    "line": null,
    "description": "rc userland malloc pool corrupts under sustained backquote churn (~428+ iterations, poisoned-header signature) — found by 17-02's composed churn leg; repro committed; deferred to 17-03's rc rows",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-12T21:27:16.731Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 68,
    "kind": "deviation",
    "phase": "17",
    "file": "build/x86_64/services-r9-x64.log",
    "line": 132,
    "description": "services-x64 gate faults at the dns stage: kernel NULL-call (PF pc=0x0) after devmnt cs error — pre-existing, A/B-proven identical at pins b02408cad and 02218e88c; last green services run was the 15-08 era; root cause open (17-04-deferred-items.md)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-12T23:34:20.636Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 69,
    "kind": "deviation",
    "phase": "17",
    "file": "build/x86_64/tier1-sweep.rc",
    "line": null,
    "description": "17-05: the tier1 gate's date check passes on a pre-existing display quirk, not a real epoch — hdate -n == 0 every r9-x64 kernel this era (r9 devcons serves no /dev/bintime; libc nsec() returns 0 on the failed open) and the sweep's *20[0-9][0-9]* expectation is met only because ctime(0) under the EST timezone data displays 'Feb 7 2006'; identical at r9 63bfda9 (pre) and 59d28fa (post), live-measured both sides; root fix belongs to the clock-seam owner (Phase-14's no-RTC era shape)",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-13T01:59:58.328Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 70,
    "kind": "todo",
    "phase": "20",
    "file": "r9:aarch64/src/proc.rs",
    "line": 1753,
    "description": "aarch64 sched_loop resumes a Ready proc that is not blocked_in_kernel (preempted at EL0, or a fork child's first run) with a bare trap_return (Resume::Return) — no note boundary on that return to EL0, so a note posted while the proc was off the CPU waits for its next tick, syscall or demand fault (the fault-return boundary landed in r9 5f7c4e1). x86_64 has no such path (a preempted proc resumes inside its own IRQ trap()). Deferred, needs its own design: delivering, or the no-handler kill, from sched-loop context (scheduler stack, PROC_TABLE lock, exit_tail re-entering the scheduler). Found by debug session arm64-fault-return-notes; the fpprobe note-fault round's criterion (no later than the first fault) stays valid when it lands. Deferred by the owner to the SMP work.",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-22T08:30:00.000Z",
    "resolved_at": null,
    "milestone": "v2.1"
  },
  {
    "id": 71,
    "kind": "deviation",
    "phase": "20",
    "file": "r9:x86_64/src/cpu.rs",
    "line": 50,
    "description": "x86_64 SYSCALL entry keeps user RFLAGS bits: IA32_FMASK = IF only (cpu.rs fmask(), documented D-02a) and syscallentry has no cld, so a user's TF (single-step #DB at the first kernel instruction, on the still-user stack), DF (the Rust kernel assumes DF=0; forward rep movs would copy backwards), NT (set inside a note handler it reaches the NOTED return's IRETQ, r9 0ffcec6, with the kernel's RFLAGS.NT=1: #GP(0) in IA-32e mode, kernel halt) and AC reach the kernel. Not reproduced (audit against SDM/QEMU semantics). Options: A widen FMASK to IF+TF+DF+NT+AC (reverses D-02a; user flags still return via R11/the frame; the only option that stops the TF #DB), B keep D-02a and reset flags in the entry stub (cld, clear NT/AC/TF; cannot stop the TF #DB). Either changes x86 bytes (x64 container gates + boot-iso hash). DEFERRED by the owner. Found by debug session noted-psr-trust",
    "status": "fixed",
    "reason": "r9 10f675e (IA32_FMASK = IF|TF|DF|NT|AC, the Linux set — option A, owner 2026-11-22; reverses D-02a's If-only value, recorded in x86_64/src/cpu.rs fmask()'s doc) + 116bcc2 (boot-iso hash). Reproduced first: on e6e5220 Harvey's amd64 fpprobe flags leg (c15b1bbab) halts the kernel with `trap GP` at trap_return's iretq while ring 0 holds the user's NT — NT set in a handler before noted, NT in the saved Ureg, and a blocking syscall whose NT reached another process's IRETQ through swtch (which carries no RFLAGS); with the other two fixes in place a FMASK=IF mutant gives `trap DB` at syscallentry's first instruction with the user RSP; a DIAG kernel shows AC in ring 0 at syscall entry on e6e5220 and no TF/DF/NT/AC on the fix. TCG cannot show the SYSCALL-DF half (QEMU drops DF on SYSCALL itself), so host tests cpu::tests::{fmask_clears_the_flags_the_kernel_must_not_inherit, fmask_is_exactly_if_tf_df_nt_ac} pin every bit. NOTED's IRETQ needs no NT clearing: its #GP reads the kernel's live NT, which no kernel context can hold after the fix. x64 tier-1 (39 T1-OK, fpprobe ok) / auth / net-smoke green at 116bcc2. Debug session .planning/debug/resolved/x86-syscall-entry-flags.md; owner confirmed fixed 2026-11-22; r9 116bcc2 pushed by the coordinator.",
    "recorded_at": "2026-11-22T09:40:00.000Z",
    "resolved_at": "2026-11-22T20:34:39.922Z",
    "milestone": "v2.1"
  },
  {
    "id": 72,
    "kind": "deviation",
    "phase": "20",
    "file": "r9:x86_64/src/trap.rs",
    "line": 335,
    "description": "x86_64 interrupt/exception entry keeps the user's DF: the IDT gates clear TF, NT and RF (and IF) but architecturally keep DF and AC, and intrcommon ran no cld, so an interrupt taken while ring 3 held DF (amd64 libc memmove's backward copy runs STD ... CLD) ran trap(), the IRQ handlers and note delivery with every compiler_builtins rep movs/stos copying backwards (the Rust kernel relies on SysV DF=0). Reproduced on r9 e6e5220 by fpprobe flags-dfirq: kernel `trap PF` in MCSLock::unlock (a smashed stack LockNode) with DF set in the kernel flags. AC stays inert at CPL0 until SMAP. Found by debug session x86-syscall-entry-flags (WINDOWS #71 sibling)",
    "status": "fixed",
    "reason": "r9 6f8de40 (cld first in intrcommon; a CLAC note for SMAP). Red on e6e5220: fpprobe flags-dfirq → kernel `trap PF` (smashed stack LockNode, DF in the kernel flags); green at 116bcc2; the no-cld mutant reds it again; host test trap::tests::intrcommon_clears_df_before_calling_trap. Debug session .planning/debug/resolved/x86-syscall-entry-flags.md; owner confirmed fixed 2026-11-22; r9 116bcc2 pushed by the coordinator.",
    "recorded_at": "2026-11-22T20:34:40.116Z",
    "resolved_at": "2026-11-22T20:34:58.042Z",
    "milestone": "v2.1"
  },
  {
    "id": 73,
    "kind": "deviation",
    "phase": "20",
    "file": "r9:x86_64/src/trap.rs",
    "line": 279,
    "description": "x86_64 intrspecial (the #DB, NMI, #BP and #DF entry) chose its SWAPGS by comparing RDGSBASE with the Mach, but RDGSBASE needs CR4.FSGSBASE, which the boot CPU never sets (l.S: TCG qemu64 cannot), so the instruction raised #UD inside the entry and every such exception ended in a kernel `trap UD` halt — including a ring-3 single-step (POPFQ TF) or int3, a user-triggerable kernel halt, and the legitimate ring-3 #DB that the #71 FMASK fix produces. Reproduced on r9 e6e5220 by fpprobe flags-tf: kernel `trap UD` at intrspecial's rdgsbaseq. Found by debug session x86-syscall-entry-flags (WINDOWS #71 sibling)",
    "status": "fixed",
    "reason": "r9 f1cd71b (intrspecial reads IA32_GS_BASE with rdmsr, rax/rcx/rdx saved, the same KMACH comparison). Red on e6e5220: fpprobe flags-tf → kernel `trap UD` at intrspecial's rdgsbaseq; green at 116bcc2 (the ring-3 single-step kills only the child, `sys: trap`); the rdgsbase mutant reds it again; host test trap::tests::entry_asm_uses_no_fsgsbase_instruction. Debug session .planning/debug/resolved/x86-syscall-entry-flags.md; owner confirmed fixed 2026-11-22; r9 116bcc2 pushed by the coordinator.",
    "recorded_at": "2026-11-22T20:34:40.291Z",
    "resolved_at": "2026-11-22T20:34:58.236Z",
    "milestone": "v2.1"
  }
]
````
