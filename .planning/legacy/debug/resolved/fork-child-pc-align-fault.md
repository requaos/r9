---
status: resolved
trigger: "arm64 r9 fork-child fault found by Harvey plan 20-14's pin series: HVF net-smoke run 23/36 at r9 105d48e — `user trap: unhandled ec=0x22 addr=0x318e7 pc=0x318e7`, `process 49 exited: \"fault\"` inside rc /lib/netip-loopback.rc at the conv=`{cat /net/ipifc/clone} backquote"
created: 2026-11-21T16:30:00Z
updated: 2026-11-21T22:10:00Z
---

## Current Focus

hypothesis: H4 (CONFIRMED at mechanism level) — rfork_copy writes child text through a data alias into recycled pages with no I-cache maintenance; on the HVF host (CTR_EL0.DIC=0) the instruction side keeps stale code per (VA, PA) across TLB maintenance, so the fork child that lands on the previous process's just-executed page at the SAME VA runs the old owner's instructions (net-smoke: rc's simple.c page 0x18000 on bind's page 0x18000 at the 36->49 fork)
test: confirmation series on r9 79c55b1 (committed fix): net-smoke HVF 5×60, tier-1 HVF ×4 + TCG ×1, smoke HVF ×20, net-smoke TCG ×1 (scratch clones $SP/dbg/harvey-{a..g}, original fixture, rev-pinned override)
expecting: all green (no fork-child faults / can't-dup / pushword); forkstress + tier-1 unaffected by the per-fork ic ialluis
next_action: none (resolved) — step A (push r9 79c55b1+46a5c87, lock-only pin move, pin series) rides with the FP/SIMD fix
scratch: SP=/private/tmp/claude-501/-Users-neilskinner-code-harvey/55ded085-3397-4d2c-931d-1d3a7a81443a/scratchpad — $SP/dbg/{harvey-a..g (scratch Harvey clones), r9-dbg (FXDBG revs 1-3), r9-dbg-fa (fa38402+FXDBG, unused control), r9-fix (fix candidate), r9-pre (FXTXT/FXPA tracker), r9-ict (ICTEST), icprobe/, icprobe3/, series/ (all captures), icache-fix.patch}
bug_class: Heisenbug/Mandelbug (HVF-only, timing-dependent; TCG 0 fires) — SBFL skipped (no per-test coverage; flaky spectrum)
known_pattern_candidate: "WINDOWS #49 family (stale TTBR0/TLB across vm::switch) — tested first: switch-in PAR probes found translations correct at every fire's fork; the residual is a DIFFERENT mechanism in the same symptom family (stale instructions, not stale translations)"

reasoning_checkpoint:
  hypothesis: "rfork_copy's text copies are written through the KZERO data alias into recycled physical pages with no IC maintenance; with CTR_EL0.DIC=0 and (VA,PA)-keyed stale instruction state that survives TLBI (measured), a fork child whose copy of VA v lands on the page the previous process executed at the same v fetches the old owner's instructions — at the net-smoke 36->49 fork rc's simple.c page 0x18000 lands on bind's page 0x18000, producing the three observed misbehaviours"
  confirming_evidence:
    - "ICTEST on the HVF host: after a data-alias rewrite of a code page (D cleaned to PoU, no IC op) the same-VA fetch returned the OLD instruction 64/64, also through unmap+tlbi+remap of the VA; other VAs aliasing the page 0/64 stale; after ic ialluis 64/64 fresh"
    - "FXTXT tracker, both boots identical: at 36->49, rc VA 0x18000 copy lands on the PA bind (pid 48) mapped at VA 0x18000 and executed last before exiting; the only other same-VA landing per boot is 6->15 at boot time"
    - "all three fires sit in that page's functions: exitnext (run-23), doredir/dupfailed (fx1-b run 9: stores never executed, stale stack printed), searchpath/execexec (2226cc4: strncmp on 0x1)"
    - "FX probes on 540 instrumented boots: translations (AT S1E0R vs mapping_log), PA ownership, fork-time page/frame snapshots and user-VA-vs-KZERO views were consistent at every checkpoint — the bytes were right, the executed instructions were not"
    - "CTR_EL0 0x99444c004: L1Ip PIPT, IDC=1, DIC=0; fault.rs text page-in does dc cvau+ic ivau, rfork_copy did nothing; first fire (2226cc4) only after 8958c9e made frees real"
  falsification_test: "fork-child misbehaviour of this class (pid 49 at the site, or any rc fork child faulting/aborting right after exec'd children exit) recurring on 79c55b1 at the baseline rate; or ICTEST showing same-VA fetch fresh without IC maintenance"
  fix_rationale: "DIC=0 makes I-cache invalidation architecturally mandatory after instructions are written by data stores; the fix applies exactly the primitive ICTEST shows restores correct fetch (dc cvau per copied text page, then ic ialluis; dsb ish; isb) at the one text-writing path that lacked it — it removes the cause (stale I-side state for the copied pages), not a symptom"
  blind_spots: "gate-level revert-and-reconfirm is statistically weak (stress A/B: stock 1/180 vs fix 0/180, p≈0.37 under equal rates; plain-gate base rate ~1/540 instrumented) — no deterministic end-to-end reproducer (the next-fit allocator defeats self-contained probes); the microarchitectural reason same-VA-only bites is inferred (Apple does not document it); the boot-time 6->15 same-VA landing never fired (lines likely evicted or not executed first); other EL0 SMC (user writable text) remains the user's responsibility"
  candidate_causes:
    - "code: rfork_copy missing I-cache maintenance after writing text through a data alias (CONFIRMED defect)"
    - "environment: Apple M-series I-side keeps (VA,PA)-keyed stale code across TLBI under a 4K guest granule (measured property; architecturally permitted with DIC=0) — contributing"
    - "data/config: next-fit allocator recycles a just-exited command's pages into the next fork since 8958c9e (correct behaviour) — contributing precondition"
    - "eliminated: stale TLB/TTBR (#49 original), PA double-ownership, lost stores/cache discard, DMA-after-free, FP/SIMD state (rc uses none), notes, Tos, CURFRAME/SP_EL1, candidate commits fa38402..105d48e"
  and_gate: "yes — the failure needs (1) the missing IC maintenance AND (2) same-VA reuse of a just-executed page AND (3) DIC=0 hardware with stale (VA,PA) I-side state; only (1) is a defect, so root_cause names (1) with (2)/(3) as contributing conditions"

## Symptoms

expected: HVF net-smoke 36/36 at r9 105d48e (as fa38402 was 36/36); netip-loopback.rc completes
actual: run 23 of 36 (after 22 passes) — pid 49 (backquote's forked rc child, not exec'd) traps with EC 0x22 (PC alignment fault) at pc=0x318e7 (odd address, inside rc's text range); netip FAIL
errors: `user trap: unhandled ec=0x22 addr=0x00000000000318e7 pc=0x00000000000318e7`; `process 49 exited: "fault"`; `netip: FAIL`
reproduction: `R9_ACCEL=hvf R9_SKIP_BUILD=1 sh build/aarch64/repeat-gate.sh boot-r9-net-smoke N` at the 105d48e pin; ~1 fire in 23 boots (1607 forks) so far
started: first seen at the 105d48e pin series (2026-11-21); fa38402 series 36/36 on this leg; #49 family pre-existing at ~1.8e-5/fork on isb-only kernels
captures: /Users/neilskinner/code/harvey/.build/hvf-series/boot-r9-net-smoke-hvf-20261121T105011/run-23.{log,out}

## Eliminated

## Evidence

- timestamp: 2026-11-21T16:30:00Z
  checked: knowledge-base.md (5 entries) and WINDOWS #49
  found: KB match on "fork child", "HVF-only", "rc" → #49 family (stale translation across vm::switch with no ASIDs; child executed on parent's pages; isb cb9cde3 + dsb ishst 57e2952). The 2226cc4 residual fire was pid 49 in netip-loopback.rc right after its parent reaped pid 48 (fault in strncmp via searchpath, corrupt argv word 0x1)
  implication: test the #49-family hypothesis first, but the new site differs in mechanism (odd PC, not a data fault)

- timestamp: 2026-11-21T16:30:00Z
  checked: run-23.out lines 2695-2720
  found: `netip: random ok`, pid 47 exited, pid 48 exited (text 0x10000/0x12000/0x18000), pid 50 page-ins 0x10000/0x11000/0x14000 + 3 stack-growth pages then `process 50 exited`, then pid 49's EC 0x22 at pc 0x318e7. The unhandled-EC print carries no TrapFrame (unlike the 2226cc4 `user fault: unresolvable` print)
  implication: pid 49 forked pid 50 (cat), reaped it, and then branched to an odd address — the fault is AFTER pid 49 has run rc code, forked and waited, not on its first instruction

- timestamp: 2026-11-21T16:40:00Z
  checked: run-22.out (passing) at the same site; build/aarch64/root-tree/lib/netip-loopback.rc; rc simple.c Xsimple/exitnext, havefork.c Xbackq
  found: normal sequence = pid 47 echo ("random ok"), pid 48 bind ('#I'), pid 49 = backquote child which execs cat DIRECTLY (exitnext() true: the backquote body ends in Xexit) — cat's page-in profile (0x10000/0x11000/0x14000 + 3 stack-growth) is printed for pid 49, then `process 49 exited`, then pid 50 = test. In run-23 the SAME cat profile is followed by `process 50 exited`, then pid 49 faults
  implication: in run-23 pid 49 did not exec cat itself; a pid 50 ran cat and pid 49 reaped it — pid 49 took a fork path that the normal backquote child never takes (either exitnext() false, or the child re-issued fork / ran Xbackq's parent arm)

- timestamp: 2026-11-21T16:45:00Z
  checked: arm64/bin/rc (built 2026-11-21 10:48 by the series' run 1; a.out arm64 HDR_MAGIC, text 0x146f0, data 0x2648 @0x30000, etext 0x24718), nm -n, data bytes decoded with python
  found: 0x318e7 is NOT in rc text (etext=0x24718); it is in DATA and is exactly Builtin[10].name — the pointer to the string "rfork" (Builtin @0x32220; entry 10 = {0x318e7 "rfork", 0x1a788 execnewpgrp}). Every non-builtin Xsimple compares a->word against all 11 builtin names with strcmp, the last being "rfork" (7c passes the 2nd arg on the stack)
  implication: pid 49 branched to a DATA POINTER value that rc routinely leaves in a stack outgoing-arg slot / register after a non-builtin Xsimple. Most likely a `ret` whose saved-LR slot held stale 0x318e7 (stack word from another moment or another process's frame), or a blr through a wrong slot. Need x30/sp at the trap: the unhandled-EC print has no TrapFrame

- timestamp: 2026-11-21T17:00:00Z
  checked: the 2226cc4 residual fire (.build/hvf-series/boot-r9-net-smoke-hvf-20261118T134933/run-21.out)
  found: SAME SITE — pid 49 = the `conv=`{cat /net/ipifc/clone}` backquote child, right after pid 48 (`bind -a '#I' /net`) exited; it faulted on its FIRST action (execexec → searchpath(strncmp) on argv word 0x1); heap pointers in x3/x7/x12/x15/x18/x19/x25 span 0x37000..0x3dfff (rc heap = brk'd Bss pages)
  implication: both post-hardening-era fires are the same fork child at the same script position, each reading a corrupt rc HEAP word (argv word 0x1; code word 0x318e7). One mechanism (child heap page with wrong contents) explains both; the 105d48e signature differs only in which word was bad. The site recurring argues for a deterministic precondition plus a timing trigger

- timestamp: 2026-11-21T17:05:00Z
  checked: rc code.c (backquote compiles `Xbackq jmp body Xexit`), simple.c exitnext/Xsimple, exec.c start(); r9 trap.rs, syscall.rs (dispatch/rfork/await/exits/exit_tail), proc.rs (rfork_clone/rfork_copy/scheduler/pick_next/reschedule/free_user_mappings/free_root_table_and_kstack/rfmem_brk_set), fault.rs (page-in), vm.rs (map_to temp-recursive dance, write_entry → dsb ishst+tlbi vmalle1is per write, switch with dsb ishst/msr/isb/tlbi/dsb/isb), bitmapalloc (next-fit from the last alloc/free, wraps), aarch64 allocator (kernel heap = static 4 MiB arena, not page-allocated)
  found: no kprocs; devip attach is trivial (print once); rfork_copy eagerly copies every mapping_log page through KZERO aliases into fresh pages; exit_tail frees user pages, then root table + kstack while still running on the kstack, then swtch; the scheduler always does the full TTBR0 switch. 49ba196 is a pure refactor; 7adb773 only re-reads the a.out header (benign apart from exec timing); 12bce96/de03bf7 are boot-time / virtio-bind only; 98feebc/9173af0 add REMOVE/WSTAT (rm through devmnt) — not on this path
  implication: no candidate change touches fork/switch/page lifecycle directly; a pre-existing defect in the #49 family (made more or less likely by timing shifts) is at least as plausible as a regression. Need runtime facts, not more reading

- timestamp: 2026-11-21T17:30:00Z
  checked: venue — scratch Harvey clones $SP/dbg/harvey-{a,b,c,d} at 4888b718b with the main checkout's 105d48e-built arm64/ + build/aarch64/out/ + .build/{bin,hostbin-darwin-aarch64} rsynced (arm64/bin/rc cmp-identical); scratch r9 $SP/dbg/r9-dbg (105d48e + FXDBG commits, never landed); stock control boot at the pin from harvey-a passed (71 forks, 16 s); bootfs.paq differs from the main one only by staged mtimes (mkpaqfs records copy times)
  found: instrumented kernel f9db9de (FXDBG rev 1: switch-in PAR probe + dup-PA scan + event ring + forensic dump on unhandled-EC/unresolvable) boots clean with 0 FX lines
  implication: venue valid; parallel HVF series possible (net-smoke has no hostfwd, -smp 1, slirp only)

- timestamp: 2026-11-21T17:45:00Z
  checked: series fx1-b (FXDBG rev 1, HVF) run 9 — $SP/dbg/series/fx1-b/boot-r9-net-smoke-hvf-20261121T115235/run-9.log
  found: THIRD signature at the SAME SITE: after `process 48 exited`, pid 49 printed `rc: can't dup 199533 to 202983: ` and exited "error". 202983 = 0x318e7 ("rfork" name pointer again); 199533 = 0x30b6d = simple.c's "exec" literal (pushword("exec") in Xsimple's exitnext arm). struct redir's from/to are `short` — 199533 cannot come from them: pfmt read dupfailed's stack-passed varargs and got STALE stack words (values an earlier Xsimple frame left at those slots). ZERO FXPROBE/FXDUP lines in that run: at pid 49's switch-in every mapping_log page translated (AT S1E0R) to its recorded PA, and no PA was held twice across live procs (user/root/kstack)
  implication: not a stale TLB at switch-in and not PA sharing between live procs. The child behaves as if user STORES are lost / reloads return older memory (or SP/FP is wrong). Same symptom family as the 2226cc4 (argv word 0x1) and run-23 (code word 0x318e7) fires

- timestamp: 2026-11-21T17:50:00Z
  checked: other mechanisms by reading — TCR_EL1 (walks inner-shareable WB cacheable, so cached page-table stores are walker-visible); MAIR/Entry attrs (kernel alias and user mappings both Normal/Inner-shareable); data-cache maintenance (no dc ivac/civac/isw anywhere in r9 — no line-discard path); bitmapalloc bit order (LSB-first everywhere); new_user_proc (root page zeroed + self-entry); trap.S (frame at SP_EL1-288, tail restores from CURFRAME, which pick_next/reschedule set before every resume)
  found: all consistent
  implication: eliminated: walker/cache incoherency on table pages, alias attribute mismatch, cache-invalidate discard, allocator bit mix-up, stale root contents

- timestamp: 2026-11-21T17:55:00Z
  checked: FP/SIMD state — r9 kernel target `+neon,+fp-armv8`, CPACR_EL1.FPEN=TrapNothing (EL0+EL1), TrapFrame/swtch Context save no v0-v31/FPCR/FPSR; rc text scanned for SIMD/FP encodings
  found: rc text has 0 real FP/SIMD instructions (all 50 class matches are literal-pool words)
  implication: eliminated for rc. SIDE FINDING (not this bug): r9 aarch64 never saves/restores user FP/SIMD state across traps/IRQs/switches while the Rust kernel itself uses NEON — any EL0 program using FP/SIMD can see corrupted vector registers

- timestamp: 2026-11-21T18:20:00Z
  checked: series fx1-{a,b,c} complete (FXDBG rev 1 kernel f9db9de, HVF, stop-on-fail off, 3 clones in parallel)
  found: 300 boots, 299 pass, 1 fail (fx1-b run 9, the can't-dup signature), 21,285 forks, 0 FXPROBE/FXDUP lines in any boot
  implication: rate at 105d48e (instrumented) ≈ 1/300 boots ≈ 4.7e-5 per fork — the same order as the #49 isb-only residual (1.8e-5) and far below the pin series' 1/23 (that was luck). Rate bisection would need many hundreds of boots per kernel to separate 0.3% from 0 → mechanism-first instead

- timestamp: 2026-11-21T18:25:00Z
  checked: the "exec" literal 0x30b6d — simple.c's, used only by Xsimple's exitnext arm (pushword("exec")); the parent pid 36 never runs that arm (its simple commands fork via execforkexec, which uses havefork.c's own "exec")
  found: the stale words pfmt printed in the can't-dup fire were pid 49's OWN earlier stack stores (Xsimple's strcmp arg "rfork", pushword's "exec"); the later stores (doredir/dupfailed outgoing args, after the dup syscall) were not visible to pfmt
  implication: within the child, stores before a point stuck and stores after it were lost (or control flow resumed somewhere that skipped them) — corruption happens DURING the child's run, around a syscall, not inherited from the parent's memory

- timestamp: 2026-11-21T18:30:00Z
  checked: notes (post_note sources: devproc note/notepg, console interrupt, alarm timeouts; a group note would print "rc: note:" — none), Tos (r9 never writes Tos fields), trap.S/CURFRAME/SP_EL1 discipline for blocked + preempted + fresh procs (every user proc's frame is kstack_top-288; pick_next/reschedule set CURFRAME before every resume), page-free sites (only proc lifecycle in proc.rs; virtio pages allocated and never freed → no DMA-after-free; no double-free path found), kernel .bss layout (HEAP 32 MiB → interrupt stack → boot stack → CURFRAME/CURKSTACK/SCHED_CONTEXT)
  found: nothing inconsistent by reading; 2226cc4 fire's proc dump shows a background ipconfig (pid 25, DHCP daemon from the previous stage, sleeping on a timer) alive at the fault
  implication: need the runtime view: rev 2 adds fork-time page-hash + frame snapshot checked at the child's first switch-in and a forensic dump when a pre-exec fork child exits non-empty (catches the can't-dup shape); rev 3 adds a lost-store detector — at every EL0 trap entry and syscall exit, each page of the current proc is read through its live user VA (its own TLB, no maintenance first) and through the recorded PA's KZERO alias; any difference is printed (FXVIEW)

- timestamp: 2026-11-21T18:45:00Z
  checked: fx2-d (rev 2, 120 boots) and fx3-{a,b,c,e,f} (rev 3, 120 boots before being stopped)
  found: 240 boots, 0 fires, 0 FXVIEW/FXPROBE/FXDUP lines; FXFIRST lines only for ping's RFMEM children (shared Data/Bss pages the parent legitimately writes; frames unchanged) — no rc fork child ever had a page or frame change between rfork_copy and its first run; rev 3's per-syscall check slows each boot 16.5 → 25 s
  implication: instrumented baseline at 105d48e = 1 fire / 540 boots. Memory and translations stay consistent around the child at every checkpoint measured — the corruption is not in the data the child sees through its page tables

- timestamp: 2026-11-21T18:50:00Z
  checked: the fixed-site question — the 6 other `x=`{cat ...}` backquotes in netip-static.rc/netip-dhcp.rc (same shape) never fire; only the one right after `echo` (pid 47) and `bind` (pid 48) exit. CTR_EL0 as printed by the guest at boot: 0x99444c004 (host: Apple M5 Max, HVF -cpu host)
  found: CTR_EL0 decode — IminLine 64 B, L1Ip = 0b11 (PIPT I-cache), DminLine 64 B, IDC = 1, DIC = 0. DIC = 0 means instruction-cache invalidation IS required for instruction/data coherence; the I-cache is physically tagged. r9 rfork_copy copies every text page into a FRESH page through the KZERO data alias and does NO I-cache maintenance ("identical bytes land at identical VAs (a stale VIPT line would return the same bytes)") — that reasoning fails for a PIPT I-cache: the PA is new, and any line still cached for it belongs to the page's PREVIOUS owner. fault.rs's text page-in does dc cvau + ic ivau; rfork's copy is the only text path without it
  implication: H4 (new leading hypothesis) — since 8958c9e made frees real, the LIFO allocator hands the fork child the pages the just-exited commands freed (bind's/echo's text, freshly executed → hot in the I-cache); the child's rc text copies land on them and the child fetches STALE INSTRUCTIONS of the old owner for whichever lines are still cached. Explains: HVF-only (TCG models no caches), rare and timing-dependent (the stale line must survive until fetched), fixed site (deterministic allocation layout puts hot rc code on a hot freed text PA only at this fork), first fire only after 8958c9e (2226cc4), memory/translations consistent at every check (the bytes are right; the fetched instructions are not), all three signatures (wrong exitnext result, a dupfailed whose stores never ran, a bogus strncmp argument). 20-02's experiment (d) (I-cache maintenance on copied text) was "ruled out" when pages were never reused (pre-8958c9e bitmap bug) — it does not apply now

- timestamp: 2026-11-21T18:55:00Z
  checked: rate-raising reproducer + candidate fix (scratch only). Fixture: scratch clones' build/aarch64/root-tree/lib/netip-loopback.rc gains a 90-iteration loop of the pid-49 shape right after `. /lib/netaddr.rc` (`echo fx >/dev/null; bind '#I' /mnt; fxc=`{cat /env/cputype} || FAIL; ~ $#fxc 1 || FAIL`) → 342 forks/boot, sanity boot green. Candidate fix (scratch r9 $SP/dbg/r9-fix 3c352db on 105d48e, arch-only): rfork_copy cleans each copied text page to PoU through its KZERO alias (dc cvau) and, if any text was copied, runs ic ialluis; dsb ish; isb once after the loop (vm.rs gains clean_dcache_to_pou + invalidate_icache_all)
  found: A/B launched interleaved: stock pinned kernel on clones a/b/c, fix kernel on d/e/f, 60 stress boots each
  implication: prediction if H4 holds — stock stress boots fire at a clearly higher rate than 1/540; fix boots 0

- timestamp: 2026-11-21T19:20:00Z
  checked: precondition tracker (scratch r9 $SP/dbg/r9-pre c997ffd on 105d48e: records the last text use — pid, VA, content hash — of every PA on text page-in and rfork text copy; reports a child text copy landing on a PA that held DIFFERENT text), 2 net-smoke boots on the unmodified fixture (clone harvey-g)
  found: deterministic and identical in both boots. At the 36->49 fork, rc's text page 0x18000 — simple.c: Xsimple 0x183c4, dupfailed 0x1855c, doredir 0x1859c, searchpath 0x18690, execexec 0x18740 (exitnext alongside) — lands on the PA that held BIND's (pid 48) text page 0x18000 moments earlier; other pid-49 text copies land on bind's and echo's (pid 47) just-executed text pages. Reuse of other programs' text PAs is common at every fork (10–20 pages each), but ONLY at 36->49 does rc's page land on a page at the SAME VA of the program that ran immediately before (bind's 0x18000: its libc tail, executed last as it exited) — the most recently fetched lines in the I-cache under exactly the code the child runs first
  implication: explains the fixed site. All three signatures live in that one page: exitnext (run-23: wrong result → forked cat as pid 50, then a jump to 0x318e7), doredir/dupfailed (fx1-b run 9: can't dup with stale args — the stores were never executed), searchpath/execexec (2226cc4: strncmp on 0x1)

- timestamp: 2026-11-21T19:25:00Z
  checked: stress A/B in progress — stock clone a run 17 ($SP/dbg/series/st-stock-a/.../run-17.log)
  found: `user fault: unresolvable class=7 addr=0x0 pc=0x19d24 read pid=217 name=rc` (freetree, x30 0x19f10), pid 217 = the stress loop's backquote child, right after pid 215 (echo profile) and pid 216 (bind profile) exited — the same shape as the original site at a different pid/code page
  implication: the stress loop reproduces the class on the stock kernel

- timestamp: 2026-11-21T19:45:00Z
  checked: stress A/B complete ($SP/dbg/series/st-{stock-a,b,c,fix-d,e,f}/), 60 boots each, interleaved in parallel
  found: stock 1 fire / 180 boots (stock-a run 17); fix 0 / 180 boots. 342 forks/boot both arms
  implication: consistent with H4 but NOT significant (P(0/180 | stock rate 1/180) ≈ 0.37). The rc stress loop raised the per-boot rate only ~3x over the plain gate — the hazard needs a specific layout, not just "fork after exec'd children exit"

- timestamp: 2026-11-21T19:55:00Z
  checked: direct probe v1 ($SP/dbg/icprobe/: gen2.py → hotbin (≈20 text pages of straight-line arithmetic, executed hot right before exit) + icprobe (≈17 pages of different-constant pads, all faulted in first; loop: fork+exec hotbin, reap, fork a child that recomputes every pad from its rfork-copied text and compares; counts mismatches/faults); built on the darwin host with 7c/7l — note the darwin host toolchain's 4 MiB sbrk arena (WINDOWS #30) caps the probe size; installed executable in the scratch root-tree/lib; fixture runs `/lib/icprobe 170 /lib/hotbin` (412 forks/boot)
  found: stock kernel sanity boot: `icprobe: ok 0 of 170`
  implication: PA reuse alone does not bite. The tracker showed the one fixed-site landing is rc page 0x18000 onto bind's page at the SAME VA (0x18000); every other fork reuses old text PAs at DIFFERENT VAs and never fires — consistent with an I-side that is effectively VA-indexed for 4K guest pages (Apple cores natively use 16K pages: VA bits above 4K can take part in the index), so a stale line is only found when the new mapping is VA-congruent with the old one. Probe v2 needs same-VA landings

- timestamp: 2026-11-21T20:30:00Z
  checked: ICTEST — boot-time hardware self-test (scratch r9 $SP/dbg/r9-ict 2f39189, then v2): one page with a r/w KZERO alias and executable kernel aliases X0, X1 = X0+4K (incongruent mod 16K), X2 = X0+16K (congruent); write `movz w0,#A; ret`, dc cvau + ic ialluis, call X0 64× (warm); rewrite `movz w0,#B; ret` through the data alias, dc cvau, NO I-cache maintenance; call X0/X1/X2; then ic ialluis and call X0. 64 rounds, line offset varied. v2 adds unmap X0 + tlbi + remap X0 to the same PA + tlbi before the fetch (the fork-path sequence). HVF, Apple M5 Max, CTR_EL0 0x99444c004
  found: v1: stale same-VA 64/64, stale X1 0/64, stale X2 0/64, fresh after ic ialluis 64/64. v2 (unmap+tlbi+remap): IDENTICAL — stale same-VA 64/64, other VAs 0/64, fresh after ic ialluis 64/64
  implication: CONFIRMED at the hardware level: a code page rewritten through a data alias is fetched STALE at the same (VA, PA) — deterministically, and TLB maintenance does not clear it — while other VAs aliasing the page fetch the new bytes; only I-cache invalidation restores correct fetch. This is exactly the rfork_copy write path, and exactly the same-VA landing the tracker found at the failing fork (bind executes VA 0x18xxx at PA p → exits (unmap+tlbi) → switch (tlbi) → rfork_copy writes rc's simple.c bytes into p via KZERO → switch (tlbi) → child maps VA 0x18000 → p → fetches bind's stale lines)

- timestamp: 2026-11-21T20:35:00Z
  checked: why the probes v1/v3 saw 0 BAD — tracker on probe v1 (600 hazard landings, 0 at the same VA); FXPA on probe v3 (C1/C2 target-page PAs)
  found: the allocator is next-fit (cursor = last alloc/free, ascending scan): consecutive fork children get fresh ascending PAs (0x429af000 → 0x429bc000 → 0x429c6000 …), so a C1/C2 alternation never shares pages; same-VA reuse only arises from a fork right after an exec'd child's exit frees pages inside the cursor's local window (the rc/bind shape)
  implication: the probes could not create the precondition; no contradiction with H4. A deterministic end-to-end reproducer would need an allocator-shaped construction — not pursued (the hardware test + the deterministic precondition carry the mechanism)

- timestamp: 2026-11-21T20:50:00Z
  checked: fix applied to the real r9 checkout (patch = scratch r9-fix diff vs 105d48e), battery inside `nix develop`
  found: `cargo xtask check` rc 0 (63 warnings), `clippy` rc 0 (82), `test` rc 0 (23 warnings; 237 passed, 0 failed — the 105d48e counts), `cargo test -p aarch64 --bins --target aarch64-apple-darwin` 32/32, `cargo xtask build` --arch aarch64 --config virtshell / --arch aarch64 / --arch x86-64 all rc 0; rustfmt --edition 2024 --check: vm.rs clean, proc.rs 13 hunks both at 105d48e and with the fix (pre-existing drift, none added); x86_64 kernel artifact sha256 35101a2a… identical with and without the change (aarch64 virtshell bytes change, as expected) → no boot-iso hash re-derivation, no x64 gates needed. Committed r9 79c55b1 (local, unpushed) on main 105d48e; nix kernel r9-kernel-virt-79c55b1 boots, net-smoke HVF sanity pass
  implication: the fix is arch-only, additive, and passes the battery; confirmation series launched (net-smoke HVF 5×60, tier-1 HVF ×4 + TCG ×1, smoke HVF ×20, net-smoke TCG ×1)

- timestamp: 2026-11-21T22:05:00Z
  checked: owner decision relayed by the coordinator — "confirmed fixed, with B then A" (B = deterministic HVF regression guard now; A = push + pin + series later, together with the separate FP/SIMD fix)
  found: B built in scratch $SP/dbg/r9-guard, cherry-picked to the real r9 as 46a5c87 on 79c55b1 (local, unpushed; shared checkout was clean at 79c55b1, no FP/SIMD-debugger commits or edits present, trap/switch paths untouched). textcopy.rs: TextCopy = the ONE rfork page-copy path (copy_page cleans text to PoU and arms the sync; finish/drop runs ic ialluis once); selftest() runs at every exec-profile boot before any process exists (warm movz/ret at an executable kernel VA, overwrite the page through TextCopy from a "parent" page, require the NEW code at the same VA, 64 rounds; control arm = the same overwrite with no maintenance); main9 prints `icache: rfork text-copy selftest N/64 fresh (control without maintenance: M/64 stale)` and panics by name unless N == 64. Results: guard HVF 64/64 fresh, control 64/64 stale (smoke 3/3 pass); MUTANT (TextCopy maintenance removed) HVF 0/64 fresh → boot panic, smoke 3/3 FAIL ("kernel panic marker on the wire"); TCG guard and mutant both 64/64 fresh with control 0/64 (TCG has no I-cache model — the control line says the venue is blind). Host test textcopy::tests::text_copies_owe_exactly_one_icache_sync (aarch64 host tests 33/33). Battery: xtask check/clippy/test rc 0, warnings 63/82/23 unchanged, 237 passed; rustfmt: textcopy.rs clean, no added drift (proc.rs 13 = base, crate tree 57 = base); x86_64 kernel bytes unchanged (35101a2a…). Committed rev on HVF: net-smoke 3/3, smoke 3/3, tier-1 1/1 (forkstress), 7/7 boots print 64/64 fresh + control 64/64 stale, 0 fault lines
  implication: deterministic regression guard in place — every HVF gate boot (smoke ×1753 per series, net-smoke, tier-1) now fails loudly if the rfork text-copy path ever loses its I-cache maintenance

## Prevention

five_whys:
  code: "a fork child ran stale instructions → rfork_copy wrote text through a data alias with no IC maintenance → the code assumed 'identical bytes at identical VAs' makes a stale line harmless → that ignores that the destination PA is NEW and its cached lines belong to the page's previous owner → the text-copy path had no single owner/contract for cache coherence (fault.rs did its own maintenance, rfork did none) [actionable: one helper owns it — textcopy.rs]"
  environment: "only HVF fired → the HVF host's I-side keeps (VA, PA)-keyed stale code across TLBI under a 4K granule (CTR_EL0.DIC=0), TCG models no caches → every CI/TCG venue is blind to this class [actionable: a boot-time self-test that is sensitive on the hardware venue and reports its own sensitivity]"
  data/allocator: "the hazard appeared only after 8958c9e → frees became real and the next-fit allocator recycles a just-exited command's pages into the next fork → page reuse turned a latent coherence gap into a live one; the 20-02 I-cache experiment (d) had been 'ruled out' when no page was ever reused, and the ruling was not revisited when reuse began [actionable: KB pattern — re-test cache-maintenance rulings when page lifetime changes]"
why_not_caught: "no gate existed for this class — TCG gates and host tests cannot observe an I-cache; the HVF gates saw it only as a ~0.2–0.5%/boot residual indistinguishable from the old #49 TLB defect; the forkstress probe checks data, not code"
recurrence_guard: "r9 46a5c87: boot-time selftest aarch64/src/textcopy.rs selftest() called from main9 for every exec-profile boot — panics by name unless 64/64 fresh (HVF kills the maintenance-removed mutant 64/64 and fails every gate boot); host test aarch64/src/textcopy.rs textcopy::tests::text_copies_owe_exactly_one_icache_sync; TextCopy is the single rfork copy path; KB entry fork-child-pc-align-fault"

## Resolution

root_cause: r9 aarch64 `rfork_copy` (aarch64/src/proc.rs) wrote every child text page into a fresh physical page through the destination's KZERO DATA alias and did no instruction-cache maintenance (the in-code justification "identical bytes land at identical VAs" does not hold for a new PA). On hardware with CTR_EL0.DIC = 0 whose instruction side keeps stale code per (VA, PA) across TLB maintenance — measured deterministically on the HVF host (Apple M5 Max): 64/64 stale same-VA fetches after a data rewrite, through unmap+tlbi+remap — a fork child whose copy of VA v lands on the page the previous process mapped and executed at the same v runs the OLD OWNER'S INSTRUCTIONS. Contributing conditions (not defects): 8958c9e made page frees real, so the next-fit allocator recycles a just-exited command's pages into the next fork; in Harvey's net-smoke fixture the deterministic layout puts rc's simple.c text page (VA 0x18000: Xsimple/exitnext/execexec/searchpath/doredir/dupfailed) on bind's just-executed page at VA 0x18000 for the `conv=`{cat /net/ipifc/clone}` backquote child (pid 49) — every residual fire (2226cc4 strncmp on 0x1; run-23 jump to 0x318e7; fx1-b `can't dup` with stale args) is that fork; TCG models no caches. Not a regression from fa38402..105d48e (none of those commits touch the path; 2226cc4 fired at the same site); the pin series' 1/23 was an unlucky draw of a ~0.2–0.5%/boot residual.
fix: r9 79c55b1 `fix(aarch64): invalidate the I-cache after rfork copies text (WINDOWS #49 residual)` — in rfork_copy, `vm::clean_dcache_to_pou(KZERO+pa, 4096)` for each copied text page and, if any text was copied, `vm::invalidate_icache_all()` (ic ialluis; dsb ish; isb) once after the copy loop; vm.rs gains the two helpers; the old "no icache maintenance" comment replaced with the measured reason. Guard: r9 46a5c87 `test(aarch64): rfork text copy through one helper, guarded by a boot-time I-cache selftest` (textcopy.rs TextCopy + selftest + host test; main9 call). Both local and unpushed; arch-only (aarch64), x86_64 bytes unchanged. Step A (push, lock-only pin move, pin series) is deferred to ride with the separate FP/SIMD save/restore fix.
verification:
  target_test:
    result: pass (statistical at gate level; deterministic at mechanism level)
    detail: "stress A/B (90-iteration pid-49-shape loop, 342 forks/boot): fix 0/180 boots; ICTEST: with ic ialluis the same-VA fetch is correct 64/64; confirmation on r9 79c55b1: see confirmation_series"
  mutation_check:
    result: skipped
    reason_if_skipped: "no Stryker for the r9 no_std kernel; the revert arm of the interleaved A/B (stock kernel = fix removed) plays the seeded mutant — it fired (stock 1/180), and ICTEST's no-IC arm is the deterministic mutant (stale 64/64)"
  no_op_deletion:
    result: pass
    deletion_justified_by_rca: n/a — purely additive (66+/3-; the 3 deleted lines are the incorrect 'no icache maintenance' comment)
  adjacent_tests:
    result: pass
    suites_run: ["cargo xtask check (63 warnings, = baseline)", "cargo xtask clippy (82, = baseline)", "cargo xtask test 237 passed / 0 failed (= baseline)", "cargo test -p aarch64 --bins --target aarch64-apple-darwin 32/32", "cargo xtask build aarch64 virtshell / aarch64 / x86-64", "tier-1 HVF 4/4 (T1-OK forkstress ×4) + TCG 1/1", "smoke HVF 20/20", "net-smoke TCG 1/1"]
  revert_and_reconfirm:
    result: pass at mechanism level; statistically weak at gate level
    bug_returned_on_revert: "yes — stock (fix reverted) stress 1 fire/180 boots (st-stock-a run 17, same shape); ICTEST without IC maintenance: stale 64/64"
    fixed_on_reapply: "yes — fix stress 0/180, ICTEST with ic ialluis fresh 64/64; gate-level significance not reached (P(0/180 | unchanged rate) ≈ 0.37)"
  confirmation_series: "r9 79c55b1 (kernel r9-kernel-virt-79c55b1, rev-pinned override, scratch clones, original fixture): net-smoke HVF 5×60 = 300/300 (21,307 forks); tier-1 HVF 4/4 (T1-OK forkstress ×4, 1,496 forks) + TCG 1/1; smoke HVF 20/20; net-smoke TCG 1/1; 327 captures, 0 user trap/user fault/can't dup/pushword/rc note lines. Fix-code HVF net-smoke total incl. the stress arm: 480 boots, 0 fires. Pooled stock rate ≈ 3/743 boots (pin 1/23, instrumented 1/540, stress 1/180) → P(0/480 | unchanged) ≈ 0.15: supportive, not significant — the mechanism evidence (ICTEST + tracker) carries the confirmation"
  x86_64_bytes: unchanged (sha256 35101a2a… with and without) — no boot-iso hash, no x64 gates
  guardrail_verdict: accepted (pending human verification; the gate-level statistical gap is disclosed in the checkpoint)
files_changed: [r9 aarch64/src/proc.rs, r9 aarch64/src/vm.rs, r9 aarch64/src/textcopy.rs (new), r9 aarch64/src/main.rs]
human_verification: "owner: confirmed fixed (relayed by the coordinator, 2026-11-21), with B (guard, done: 46a5c87) then A (deferred)"
