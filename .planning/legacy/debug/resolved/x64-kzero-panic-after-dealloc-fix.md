---
status: resolved
trigger: "x86_64 r9 kernel (r9 2226cc4) panics booting Harvey: auth-smoke-x64 panicked at x86_64/src/proc.rs:361:5 (PA outside KZERO window) after `init: starting /bin/rc`; net-smoke-x64 shows `user trap: unhandled vector 6 pc=0x204ff8 addr=0x0`, process 4 exited \"sys: trap\", then same proc.rs panic. Leading (unconfirmed) hypothesis: r9 8958c9e bitmapalloc deallocate fix exposes an x86_64 use-after-free/double-free in a free_phys_page path."
created: 2026-11-18T00:00:00Z
updated: 2026-11-19T03:00:00Z
---

## Current Focus

hypothesis: H1 (confirmed) — x86_64 Proc::free_user_mappings frees the user-half intermediate page tables (mapping_tables) while the surviving PML4 still links them; exec_replace then maps the new image into that same PML4, so the walk descends into freed pages that alloc_phys_page is simultaneously handing out as the new image's text/data/stack leaves
test: done — host regression test red/green + 3 mutants killed; auth + net x64 gates FAIL on 2226cc4 (CI symptoms bit-for-bit) and PASS on cec2299; no-op-dealloc boot clean; r9 check/clippy/test green; aarch64 TCG boot-r9-smoke ALL STAGES PASS (cec2299 and HEAD 57e2952)
expecting: (met) owner confirmation in the real workflow
next_action: DONE — confirmed fixed: r9 cec2299 pushed by the owner, Harvey pinned to it (b0e3cef8f), hosted CI auth-smoke-x64 and net-smoke-x64 green; session archived to resolved/, knowledge-base entry appended
bug_class: Bohrbug (deterministic: reproduced locally on first try, same PC/panic site as CI)

reasoning_checkpoint:
  hypothesis: "exec of any binary on x86_64 corrupts the new image once frees are real: free_user_mappings returns the PDPT/PD/PT pages of the old image to the allocator but leaves PML4[0] and PML4[255] pointing at them; exec_replace's install_image allocates the new image's pages (the bitmap allocator's next_pa_to_scan sits on those just-freed pages, so they come back first) and walks PML4->stale PDPT->..., so the same physical page is both a leaf (text/data bytes written through KZERO) and a live table. Text bytes read as a PTE give a PA >= 1 GiB -> proc.rs:361 panic; a PTE written into a page that is also text corrupts code -> #UD."
  confirming_evidence:
    - "host test: after free_user_mappings PML4[0] = 0x7ffff8014007 (P|RW|US + the freed host-page PDPT) (red); green with the unlink"
    - "baseline capture: `PA 0x368c48348f000 outside the KZERO window` — the masked entry carries bytes 48 83 c4 68 (amd64 ADD RSP,$0x68): code bytes walked as a PTE"
    - "net-smoke CI #UD at pc=0x204ff8: offset 0xff8 = qword 511, the slot every stack-path walk (USTACKTOP, PML4[255]/PDPT[511]/PD[511]) writes"
    - "no-op deallocate (x86_64 only): no panic, sweep proceeds (H0); fix only (frees still real): composed auth gate ALL STAGES PASS"
    - "aarch64 never frees intermediate tables (no mapping_tables) — explains aarch64 green on the same allocator"
  falsification_test: "if the panic persisted with the unlink in place (frees real), or vanished WITHOUT it under a kernel whose only change was elsewhere, H1 would be wrong. Observed: with only the unlink added the gate passes; without it (2226cc4) it panics."
  fix_rationale: "the defect is free-while-still-linked: the fix unlinks the user half (PML4[0..256) — every table under it is a recorded mapping_tables page) and reloads CR3 when the root is active (paging-structure caches may hold upper entries re-cached after the per-leaf invlpg's), THEN frees. The allocator stays correct; frees stay real; the next exec builds fresh, recorded tables."
  blind_spots: "the CR3-reload half is not host-testable (no TLB on host) — covered only by the composed gates; exit_tail also runs free_user_mappings (same unlink; the PML4 is freed at the very end) — not separately stressed beyond the gates' 75 exits; forkstress-scale churn on x86_64 not run (tier1-x64/churn gates not in scope)"
  candidate_causes:
    - "code: x86_64 free_user_mappings frees linked intermediate tables (CONFIRMED)"
    - "code: free_root_table_and_kstack frees the running kstack/PML4 (ruled out by reading: no allocation between the free and the CR3 switch; heap is a static arena)"
    - "code: RFMEM SharedPages refcount imbalance -> shared leaf freed early (ruled out by reading: share/share_new/release balanced on every arm; fix-only kernel passes)"
    - "data/config: allocator itself (8958c9e) wrong (ruled out: port test suite green; fix leaves allocator untouched and frees real)"
  and_gate: "yes — two conditions are jointly necessary: (1) the x86_64 free-while-linked in free_user_mappings (present since 14-04/17-02) AND (2) real frees from 8958c9e; either alone is harmless (pre-8958c9e: the freed tables leaked intact; aarch64: frees real but no table free). The fix removes (1); (2) is correct behavior and stays."

## Symptoms

expected: x86_64 r9 kernel boots Harvey; auth-smoke-x64 and net-smoke-x64 gates pass (as before r9 8958c9e)
actual: auth-smoke-x64 panics at x86_64/src/proc.rs:361:5 (assert pa < KZERO_MAP_LIMIT in phys_ptr_mut) after `init: starting /bin/rc`; net-smoke-x64 gets `user trap: unhandled vector 6 pc=0x204ff8 addr=0x0`, `process 4 exited: "sys: trap"`, rc restarts, some stages pass, then same panic
errors: "panicked at x86_64/src/proc.rs:361:5"; "PA {pa:#x} outside the KZERO window"; "user trap: unhandled vector 6 pc=0x204ff8 addr=0x0"; pre-existing noise: "syscall 10 unimplemented", "#c/swap"/"#c/sysname" namec misses
reproduction: Harvey CI x86_64 composed gates (smoke-r9-auth-x64, smoke-r9-net-x64) with kernel-x64 built from r9 2226cc4
started: after Harvey flake pinned r9 at/after 8958c9e (20-03 bitmapalloc fix); aarch64 green on 2226cc4

## Eliminated

## Evidence

- timestamp: 2026-11-18T00:00:00Z
  checked: .planning/debug/knowledge-base.md (Phase 0)
  found: only entry G-05-1 (missing /bin/rc in namespace build); no symptom overlap with KZERO panic / invalid opcode
  implication: no known-pattern candidate

- timestamp: 2026-11-18T00:00:00Z
  checked: .planning/phases/20-arm64-catch-up-to-v2-1-parity/20-EVIDENCE.md "Second defect"
  found: old deallocate cleared bit `bit_idx` of byte 0 instead of 8*byte_idx+bit_idx -> no page ever freed; first 8 pages of each 128MiB bitmap wrongly freed (on x86_64: PA 0..0x7000). Fix 8958c9e. Notes x86_64 kernel bytes change.
  implication: before 8958c9e every x86_64 free was a leak (masking any UAF); also PA 0..0x7000 could be handed out after first frees

- timestamp: 2026-11-18T00:10:00Z
  checked: r9 x86_64/src/proc.rs free_user_mappings (1411-1433), pml4_map_user (469-522), pml4_unmap_user (526-545); exec.rs exec_replace (553-609), install_image/map_text_eager (244-340)
  found: free_user_mappings unmaps each LEAF (clears only the PT entry) then frees every page in `mapping_tables` (the user-half PDPT/PD/PT intermediates) WITHOUT clearing the PML4/PDPT/PD entries that point at them. exec_replace keeps the same PML4 and immediately calls install_image -> map_user_page -> alloc_phys_page + pml4_map_user. The walk finds PML4[idx] still PRESENT and descends into the freed intermediate pages, while alloc_phys_page hands those same freed pages out again (zeroed, then text/data bytes written through the alias).
  implication: strong H1 candidate — after exec, freed page-table pages are simultaneously live tables (via stale PML4 entries) and leaf pages; text bytes become PTEs -> garbage PA at phys_ptr_mut (proc.rs:361) or wrong leaf -> user #UD. Before 8958c9e the frees never happened, so the stale tables stayed intact and private.

- timestamp: 2026-11-18T00:12:00Z
  checked: aarch64/src/proc.rs free_user_mappings (1099-1141)
  found: aarch64 unmaps leaves and frees leaf pages only; it never frees intermediate tables (no mapping_tables concept)
  implication: explains why aarch64 is green on 2226cc4 — the x86_64-only intermediate-table free (added 14-04 / 17-02) is the divergence

- timestamp: 2026-11-18T00:20:00Z
  checked: walk index arithmetic vs the net-smoke #UD pc
  found: the user stack (USTACKTOP 0x7fff_ffff_f000) walks PML4[255] -> PDPT[511] -> PD[511] -> PT[511/510...]: every stack-path table write lands at byte offset 0xff8 of its table page. net-smoke's `user trap: unhandled vector 6 pc=0x204ff8` is offset 0xff8 of a TEXT page.
  implication: consistent with a freed stack-path intermediate table being re-handed out as a text leaf page, then the stale walk (PML4[255] still linked) writing a PTE into qword 511 = the text bytes at 0x204ff8 -> invalid opcode. Mechanism-level match for H1.

- timestamp: 2026-11-18T00:22:00Z
  checked: other x86_64 free sites (brk family arms, rfork copy/unwind, unwind_new_proc, free_root_table_and_kstack, SharedPages refcount), heap allocator (static arena — never takes pagealloc pages), page alloc users (proc machinery + virtio init only), CR4 (PGE on, no PCID; user leaves never global)
  found: every other free either unlinks first, or frees pages of a proc that never runs again (rfork unwind child, exit tail last frees before the CR3 switch with no intervening allocation); refcount release logic is balanced
  implication: free_user_mappings' intermediate-table free on the EXEC path is the only free-while-still-linked-and-walked site found by reading

- timestamp: 2026-11-18T20:25:00Z
  checked: new host test proc::tests::free_user_mappings_unlinks_the_freed_tables_for_exec_reuse (x86_64/src/proc.rs), run in the x86_64-linux container (`nix develop -c cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu`) against unmodified free_user_mappings
  found: FAILED — "PML4[0] still links a freed user-half table after free_user_mappings" (left: 140737354219527 = a present PDPT pointer, right: 0)
  implication: host-level reproduction: after free_user_mappings the surviving PML4 still links the freed intermediate tables (RED)

- timestamp: 2026-11-18T20:24:00Z
  checked: local repro in an x86_64-linux container (Docker Desktop amd64/Rosetta, ubuntu 24.04, single-user nix sandbox=false; Harvey 261553217 clone, r9 clone at 2226cc4): `env R9_ACCEL=tcg nix run .#boot-r9-auth-x64 --override-input r9 git+file://$HOME/dbg/r9`
  found: gate FAILED exactly like CI — after `init: starting /bin/rc`: "panicked at ... kernel panic marker on the wire — aborting", EXIT=1 (expect aborts mid-line)
  implication: the CI failure reproduces locally against r9 2226cc4 (Bohrbug, deterministic)

- timestamp: 2026-11-18T20:40:00Z
  checked: unattended capture of the same kernel + the same baked bootfs-auth.paq (bootcap.sh: r9 iso-builder + qemu q35 TCG, full serial kept)
  found: `panicked at x86_64/src/proc.rs:361:5: PA 0x368c48348f000 outside the KZERO window`. The masked "PTE" 0x0003_68c4_8348_f000 carries bytes `48 83 c4 68` = amd64 `ADD RSP, $0x68` (REX.W 83 /0 ib) — machine code.
  implication: a text page's bytes were walked as a page-table entry — direct observation of an image page doubling as a live table (H1's predicted corruption)

- timestamp: 2026-11-18T21:00:00Z
  checked: H0 experiment — r9 2226cc4 + x86_64 deallocate_physpage made a no-op (container clone r9-noop, 35df55b, x86_64/src/pagealloc.rs only; port/src untouched), unattended capture with the same bootfs-auth.paq
  found: no panic; boot passes `init: starting /bin/rc` and runs the auth sweep: 8 x AUTH-OK (keyfs, authsrv-listen, secstored, cs, factotum, p9sk1-norbert, speaksfor, speaksfor-deny) up to secuser's interactive `norbert password:` prompt (needs the expect driver's typed feed) when the 420 s capture timer ended
  implication: H0 CONFIRMED — the panic needs frees to be real; a freed page is still in use somewhere on x86_64

- timestamp: 2026-11-18T22:02:00Z
  checked: candidate fix (container clone r9-fix 332806b = 2226cc4 + x86_64/src/proc.rs: free_user_mappings unlinks PML4[0..256) + CR3 reload before freeing mapping_tables) — the DEFAULT gate command incl. the full userland build: `env R9_ACCEL=tcg nix run .#boot-r9-auth-x64 --override-input r9 git+file:///home/builder/dbg/r9-fix`
  found: `smoke-r9-auth-x64: ALL STAGES PASS — ...` and `== composed auth gate PASS`, EXIT=0; capture has 75 process exits, 0 `panicked`, 0 `deallocate`/`NotAllocated`, 0 `vector 6`
  implication: H1 CONFIRMED — the only change vs the failing kernel is the exec-path table unlink; frees stay real (allocator untouched)

- timestamp: 2026-11-19T01:30:00Z
  checked: `env R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64 --override-input r9 git+file:///home/builder/dbg/r9-fix` (container; first attempt died in the bake: `no host C compiler for authdomain-gen` — the minimal ubuntu image had no cc, which ubuntu-latest has; installed gcc, re-ran)
  found: `smoke-r9-net-x64: ALL STAGES PASS — ...`, `== composed net gate PASS`, EXIT=0; 73 exits, 0 panicked/deallocate/vector 6/sys: trap
  implication: net gate green with the fix

- timestamp: 2026-11-19T01:35:00Z
  checked: manual mutation check (no Stryker for Rust): mutants at the fix site in a scratch copy — A: unlink call removed; B: range 0..255 (drops the stack slot); C: range 1..256 (drops the text slot)
  found: the regression test KILLS all three (A, C: "PML4[0] still links..."; B: "PML4[255] still links..."); unmutated passes. The CR3-reload half has no host-observable effect (no TLB on host) — not killable by a host test.
  implication: the test asserts the root cause (stale links from the surviving root), not a symptom

- timestamp: 2026-11-19T01:40:00Z
  checked: r9 committed as cec2299 on top of 058b848 (docs-only README/SVG commit that appeared on r9 main during the session, not made by this session); diff cec2299 vs tested 332806b = README.md + docs/img/plan-r.svg only
  found: x86_64-linux container on cec2299: `cargo xtask check` rc=0, `cargo xtask clippy` (default arch x86-64 = CI's) rc=0, `cargo xtask test` rc=0 (port 232 passed / 0 failed — the devShell has no rustup, so xtask skips its arch leg), explicit `cargo test -p x86_64 --bins --target x86_64-unknown-linux-gnu` 105 passed / 0 failed (incl. the new test). Mac (darwin): check rc=0, clippy rc=0, xtask test rc=0 (port 232/0). `cargo xtask clippy --arch aarch64` rc=1 with 5 errors at BOTH 2226cc4 and cec2299 (aarch64/src/proc.rs undocumented unsafe — pre-existing, untouched)
  implication: adjacent/held-out suites green; no new lint

- timestamp: 2026-11-19T01:45:00Z
  checked: aarch64 — lima guest (aarch64-linux, TCG): `nix run .#boot-r9-smoke --override-input r9 git+file:///home/neilskinner.guest/dbg-x64/r9-final` (Harvey 261553217 clone, r9 cec2299, kernel r9-kernel-virt-cec2299 = the virtshell config); and `nix build #kernel-virt` at 2226cc4 vs cec2299 in the same guest
  found: `smoke-r9: ALL STAGES PASS — boot, echo (typed + answered), ls/ps/cat sweep, DEL safety, restart, ...`, EXIT=0; kernel-virt.elf sha256 e3ff5eeaf2d358b2a0024b191fa34ca94e679ced854bf89d1c8089efaf199071 for BOTH 2226cc4 and cec2299
  implication: the aarch64 kernel is byte-identical — the fix cannot affect aarch64

- timestamp: 2026-11-19T01:50:00Z
  checked: r9 git log after the commit
  found: another session committed 57e2952 `fix(20-06): dsb ishst before the TTBR writes and the full tlbi ...` (aarch64/src/vm.rs only, 20:44:59 -0500) ON TOP of cec2299; main is now origin/main + 3 (058b848 docs, cec2299 this fix, 57e2952 aarch64 vm). Not made by this session.
  implication: this session's fix is cec2299; verification targets cec2299; an extra aarch64 smoke against 57e2952 was started so "the local r9" HEAD is covered too

- timestamp: 2026-11-19T02:05:00Z
  checked: lima guest TCG: `nix run .#boot-r9-smoke --override-input r9 git+file:///home/neilskinner.guest/dbg-x64/r9-head` (r9 57e2952 = current local HEAD, kernel r9-kernel-virt-57e2952)
  found: `smoke-r9: ALL STAGES PASS — ...`, EXIT=0
  implication: aarch64 green on the current local r9 HEAD too

- timestamp: 2026-11-19T02:10:00Z
  checked: FINAL — `env R9_ACCEL=tcg nix run .#boot-r9-auth-x64 --override-input r9 git+file:///home/builder/dbg/r9-final` (r9 cec2299dbaeb, clean; kernel r9-kernel-x64-cec2299; full default pipeline incl. userland build)
  found: `smoke-r9-auth-x64: ALL STAGES PASS — ...`, `== composed auth gate PASS`, EXIT=0; 75 exits, 0 panicked/deallocate/vector 6/sys: trap
  implication: the committed fix passes the auth gate

- timestamp: 2026-11-19T02:20:00Z
  checked: FINAL — `env R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64 --override-input r9 git+file:///home/builder/dbg/r9-final` (r9 cec2299dbaeb, clean; kernel r9-kernel-x64-cec2299; full default pipeline)
  found: `NET-SMOKE: ALL STAGES PASS`, `smoke-r9-net-x64: ALL STAGES PASS — ...`, `== composed net gate PASS`, EXIT=0; 74 exits, 0 panicked/deallocate/vector 6/sys: trap
  implication: the committed fix passes the net gate

- timestamp: 2026-11-19T02:30:00Z
  checked: revert-and-reconfirm on the net gate — `env R9_ACCEL=tcg nix run .#boot-r9-net-smoke-x64 --override-input r9 git+file:///home/builder/dbg/r9` (r9 2226cc4, fix absent)
  found: FAILED exactly like CI: `user trap: unhandled vector 6 pc=0x204ff8 addr=0x0`, `process 4 exited: "sys: trap"`, `init: rc exit status: sys: trap`, prompt + seam-prompt stages pass, then `panicked at` -> abort, EXIT=1
  implication: both CI symptoms reproduce bit-for-bit (same pc) without the fix and are gone with it

- timestamp: 2026-11-19T02:40:00Z
  checked: `nix build #kernel-x64` sha256 (x86_64-linux container) for 2226cc4 / cec2299 / 57e2952 (current HEAD)
  found: cec2299 and 57e2952 both 00144550f78c42a5b453f0f958863380e8bb7bcbaaa8db81657b6d2357cd1865; 2226cc4 f89cb290b8aff177cb46c7eaefdf60365341b647778382d93a9b9c5807cb3188
  implication: the x64 gate results on cec2299 hold byte-for-byte for the current local HEAD

- timestamp: 2026-11-19T03:00:00Z
  checked: owner/coordinator confirmation (human-verify checkpoint response)
  found: "Confirmed fixed" — r9 cec2299 pushed by the owner; Harvey pinned to it (b0e3cef8f, plus the unrelated /bin/ls CI fix 0d01baa43); hosted CI auth-smoke-x64 and net-smoke-x64 both pass
  implication: verified in the real workflow; session resolved

## Resolution

root_cause: "AND-gate of two conditions: (1) r9 x86_64/src/proc.rs Proc::free_user_mappings freed every user-half intermediate page table (PDPT/PD/PT pages recorded in mapping_tables) while the proc's PML4 still linked them (PML4[0] text/data subtree, PML4[255] stack subtree); EXEC (exec.rs exec_replace -> install_image) keeps that PML4 and immediately maps the new image through it, so pml4_map_user walked into the freed tables while alloc_phys_page handed those same pages out (next_pa_to_scan sits on them) as the new image's leaves — image bytes written through KZERO became PTEs (garbage PA -> proc.rs:361 KZERO panic) and stale walks wrote PTEs into image pages (stack-path slot 511 = offset 0xff8 -> #UD at pc 0x204ff8); (2) r9 8958c9e made bitmapalloc deallocate actually free (correct) — before it every free leaked, so the freed tables stayed intact and private. aarch64 never frees intermediate tables, hence green."
fix: "x86_64/src/proc.rs: new unlink_user_half(pml4_pa) clears PML4[0..USER_PML4_SLOTS=256) and reloads CR3 when that root is the active one (drops paging-structure-cache entries re-cached after the per-leaf invlpg's; no PCIDs, user leaves never global); free_user_mappings calls it after the leaf unmaps and BEFORE freeing mapping_tables. Compile-time assert KSTACK_PML4_INDEX >= USER_PML4_SLOTS (the kstack slot 400 and kernel slots 510/511 are untouched — the exit tail still runs on them). New host test proc::tests::free_user_mappings_unlinks_the_freed_tables_for_exec_reuse. Allocator (port/src) untouched; frees stay real. r9 commit cec2299 (local, not pushed)."
oracle_type: "specified — the PML4 user half must be empty after free_user_mappings, and every reachable user-half table page must be owned by the proc (mapping_tables) after the re-map; boundary neighbors = both user-half subtrees (slot 0 text, slot 255 stack) plus the kstack slot 400 that must survive"
verification:
  target_test: { result: pass, note: "red on 2226cc4 shape (PML4[0] = 0x7ffff8014007), green with fix; x86_64 battery 105/105" }
  mutation_check: { result: pass, reason_if_skipped: "no Stryker for Rust — manual mutants", mutant_killed: "A (unlink removed), B (0..255), C (1..256) all killed; CR3-reload half not host-observable" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: n/a, note: "pure addition: an unlink + flush before the existing free" }
  adjacent_tests: { result: pass, suites_run: ["x86_64-linux: cargo xtask check/clippy/test (port 232/0)", "cargo test -p x86_64 --bins (105/0)", "darwin: cargo xtask check/clippy/test (port 232/0)", "aarch64 virtshell build (Mac xtask build; guest nix kernel-virt byte-identical to 2226cc4)", "Harvey boot-r9-smoke TCG aarch64 ALL STAGES PASS"] }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true, note: "2226cc4 (fix absent): auth gate panics, capture `PA 0x368c48348f000 outside the KZERO window`; 332806b (2226cc4 + fix only): auth + net gates ALL STAGES PASS; cec2299 final gates: see Evidence" }
  guardrail_verdict: accepted
files_changed: [x86_64/src/proc.rs]
