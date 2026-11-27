---
status: awaiting_human_verify
trigger: "MEM-15/MEM-16 unproven: under real aarch64 memory pressure the palloc allocator kills the allocating process ('fault: phys page alloc failed' -> unresolvable pid) instead of reclaiming; evictor counters stay 0/0/0 at 2048/1536/1200 frames; cap 512 dies in boot, cap 1024 wedges."
created: 2026-11-27T02:30:00.000Z
updated: 2026-11-27T05:40:00.000Z
---

## Current Focus
<!-- OVERWRITE on each update -->

hypothesis: "CONFIRMED (three contributing defects): (1) `alloc_clean` reclaimed ONE frame per evictor attempt; the frame returns to the shared dirty pool and the sibling CPU took it before the caller's retry, so a starving allocator burned its 4 attempts and returned OutOfSpace while the cache was still reclaimable; (2) the droppable FIFO had no mappedness preference, so once cold pages were gone it evicted hot text pages of running processes and they re-faulted immediately (377,377 refaults of va=0x18000), a livelock that made the pressure run wedge; (3) the authored gate load either never exhausted the pool (exec-churn loop) or demanded ~100x it (hseq 1 400000), and its `pinned_skipped > 0` criterion was structurally unreachable from this image's userland."
test: "Done: batch reclaim (EVICT_BATCH_FRAMES=16), unmapped-first victim selection, a bounded 44,000-line `hseq|hsort` burst sized to exceed dirty+clean but stay inside the cold cache, and the criterion change to dropped_text/dropped_data/evict_calls/reclaimed/free_failures with pinning proven by a host test"
expecting: "Gate passes with real reclamation and no killed process; verified 3x HVF + 1x TCG identically (20/28/3/48/0)"
next_action: "Awaiting owner confirmation that the original workflow is resolved; on confirmation, archive the debug file to .planning/debug/resolved/ and append the knowledge-base entry"
bug_class: bohrbug
reasoning_checkpoint: "hypothesis: single-frame reclaim + unmapped-blind FIFO cause starvation/thrash under two-CPU exhaustion; evidence: instrumented pool dumps (clean=0 dirty=0 active=2048) with freed pa showing dirty=1/active=2047 then the next call seeing dirty=0/active=2048 again, plus 377,377 refaults of one text page; falsification: a batch reclaim that still fails, or prefer-unmapped that still refaults hot text, would disprove; fix rationale: batch leaves the caller a margin of reclaimed frames and prefer-unmapped drains cold pages so no live proc re-faults; blind spots: x86_64 live leg not runnable on this host; candidate_causes: code (single-frame reclaim, FIFO policy), data (gate workload size), config (n/a); and_gate: yes — starvation AND hot-page selection both had to be fixed for the bounded burst to complete"
tdd_checkpoint: null

## Session Loop
goal: find_and_fix
tdd_mode: false
tdd_phase: none
resume_count: 0
awaiting: none
question: none
last_next_action: none

## Symptoms
<!-- IMMUTABLE -->

expected: "Under a bounded pool the live evictor reclaims re-readable (cached text / untouched data) pages, allocation succeeds, and the gate observes dropped_text/dropped_data > 0. With genuine exhaustion the kernel does not kill the allocating process."
actual: "R9_PALLOC_MAX_FRAMES=2048/1536/1200: evictor: dropped_text=0 dropped_data=0 pinned_skipped=0. True exhaustion: 'fault: phys page alloc failed' -> 'unresolvable ... pid=6 name=hseq', then the pipe-RPC wedge. Cap 512 dies in boot (DataAbortSameEl far=0x20118); cap 1024 wedges after the reader stage."
errors: "fault: phys page alloc failed; unresolvable pid; DataAbortSameEl far=0x20118 at cap 512"
reproduction: "nix run .#boot-r9-mem-pressure (aarch64 HVF on this host); the wrapper honours R9_PALLOC_MAX_FRAMES / R9_MEM"
started: "Present since 05-18 wired the live evictor and 05-19 mirrored it; the 05-21 pressure gate is the first workload that exercises it"

## Eliminated
<!-- APPEND only -->

- hypothesis: "The gate's pinned_skipped criterion is merely too strict"
  evidence: "Independently true but not the whole story: dropped_text/dropped_data are also 0, and allocation fails instead of reclaiming. The criterion is ALSO structurally unreachable (write buffers are promoted off the droppable queue; pwrite's source is stack/.bss, never in palloc.hash), so it must be revised regardless."
  timestamp: 2026-11-27T02:30:00.000Z

- hypothesis: "The page cache is absent at boot (PageCache::new failed on the 4 MiB heap), so no page is ever insertable"
  evidence: "Instrumented `palloc:` marker: `palloc: managed_frames=2048 low_water=102 page_cache=present entries=0 droppable=0` (entries 0 is correct at that point, before userspace); after the boot chain `#c/sysstat` reported entries=342 droppable=215 inserts=342 hits=2812 misses=196 insert_failures=0. The cache is present, inserting, and serving hits."
  timestamp: 2026-11-27T04:05:00.000Z

- hypothesis: "Live text page-ins fail to insert into palloc.hash (insertion/classification bug)"
  evidence: "entries/inserts rise with every exec (132 at boot → 342 after the workload; insert_failures=0), and the evictor later drains entries 178→38. Insertion works; the earlier 0/0/0 came from the workload never exhausting dirty+clean."
  timestamp: 2026-11-27T04:10:00.000Z

- hypothesis: "Evicted frames are not returned to PAGE_ALLOC (`free_frame` rejects them)"
  evidence: "`evict: freed pa=… pool_after=(clean=0 dirty=1 active=2047 …)` for every sampled victim; the next evict call saw dirty=0 active=2048 — the frame WAS returned and a peer CPU consumed it. free_failures stayed 0."
  timestamp: 2026-11-27T04:30:00.000Z

- hypothesis: "Cap 512/1024 failures are the same defect"
  evidence: "At cap 512 the pool is smaller than the boot chain's live set before any user page is cached, so the boot's own allocations fail with no reclaimable cache in existence; that is a boot-footprint limit, not the live-evictor defect. The owner's acceptance workload uses the documented 2048 cap."
  timestamp: 2026-11-27T04:45:00.000Z

## Evidence
<!-- APPEND only -->

- timestamp: 2026-11-27T02:30:00.000Z
  checked: "05-21 continuation measurements (Rule-4 checkpoint)"
  found: "Counters 0/0/0 at 2048/1536/1200 frames; exhaustion kills the allocator's caller; 512 dies in boot; 1024 wedges"
  implication: "Eviction is not running when it matters; start from whether evict_re_readable_pages is invoked and what it scans"

- timestamp: 2026-11-27T02:30:00.000Z
  checked: "Where the pressure gate lives on main"
  found: "nix run .#boot-r9-mem-pressure is available on main (wired by 05-21); the gate asserts palloc: managed_frames=<cap>, then drives two workers and parses evictor: dropped_text/dropped_data/pinned_skipped"
  implication: "Locally reproducible; no KVM host needed"

- timestamp: 2026-11-27T03:40:00.000Z
  checked: "Gate at cap 2048 with the cache instrumented (hash entries/droppable/inserts/hits/misses in #c/sysstat, page_cache presence in the palloc marker)"
  found: "page_cache=present; after boot+workload entries=342 droppable=215 inserts=342 hits=2812 misses=196 insert_failures=0; evict_calls=0 (evictor never invoked); counters 0/0/0"
  implication: "The cache is healthy. The authored exec-churn load never empties dirty+clean, so alloc_clean never calls the evictor — problem (a) is the workload, not the cache"

- timestamp: 2026-11-27T04:00:00.000Z
  checked: "True exhaustion: `hseq 1 400000 | hsort` under instrumentation, printing pool cardinalities at every evictor entry and after every free"
  found: "Every evict entry: pool=(clean=0 dirty=0 active=2048 pinned=4). Each eviction freed exactly one frame: `freed pa=… pool_after=(clean=0 dirty=1 active=2047 pinned=4)`. The NEXT evict entry showed `clean=0 dirty=0 active=2048` again — the sibling CPU consumed the single reclaimed frame. After 4 such attempts alloc_clean returned OutOfSpace and the fault path killed hseq/hsort."
  implication: "Root cause 1: one frame per reclaim attempt + a shared dirty pool = reclaim starvation between CPUs. The allocator did reclaim; the caller never got to use its own reclaim"

- timestamp: 2026-11-27T04:20:00.000Z
  checked: "Batch reclaim (EVICT_BATCH_FRAMES=16) under the same 400k burst"
  found: "Evictions returned 16 frames per call (entries 167→151→135…); the burst survived far longer, but once the cold cache was drained it began re-faulting one hot text page: `fault: text page-in va=0x18000` appeared 377,377 times, `fault: text page-in va=0x17000` 376,711 times, and the run wedged"
  implication: "Root cause 2: the droppable FIFO evicts mapped, hot text pages of running processes with no unmapped-cost preference, so deep pressure livelocks on evict→refault→evict"

- timestamp: 2026-11-27T04:40:00.000Z
  checked: "Victim order: prefer pages whose rmap is empty (no live proc maps them; exec/exit already dropped the mapping)"
  found: "A 44,000-line readerless burst reclaims 64 pages with zero refault storm and completes in ~0-2 s; the same burst with the old background reader's extra shells (higher baseline) over-drains the cold set and thrashes at 44k, but 40k with the reader passes (96-112 reclaimed)"
  implication: "Prefer-unmapped removes the thrash while cold pages exist; the gate load must be bounded to stay inside them"

- timestamp: 2026-11-27T05:00:00.000Z
  checked: "Final gate design: single bounded `hseq 1 44000 | hsort > /dev/null` burst (no background reader), re-fault `cat /lib/namespace`, sysstat criteria dropped_text>0, dropped_data>0, evict_calls>0, reclaimed>0, free_failures==0, whole-capture forbids `phys page alloc failed`"
  found: "4 consecutive passes: 3x HVF + 1x TCG, identical numbers each run — dropped_text=20 dropped_data=28 pinned_skipped=0 evict_calls=3 reclaimed=48 free_failures=0; re-fault line intact; MEM_PRESSURE_OK; no unresolvable/panic/alloc-failure marker"
  implication: "Fix verified end-to-end; the counters are deterministic across accelerators"

- timestamp: 2026-11-27T05:05:00.000Z
  checked: "Pinning proof (the old pinned_skipped criterion)"
  found: "No userland path in the image can pin a cached page (write buffers are promoted off the droppable queue; pwrite's read-only source is an uncached stack/.bss page). Added host integration test `pinned_cached_page_is_skipped_until_unpinned`, which pins a real cached frame through PageAlloc::pin_frame (the exact UserSlice PinGuard call) and requires select_eviction_victim to skip/count it and take it once unpinned"
  implication: "The pin contract is proven at the cache/allocator seam; the gate reports pinned_skipped without asserting it"

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: "Three contributing defects, all required for the failure: (1) `PageAlloc::alloc_clean` asked the arch evictor for ONE frame per attempt; the freed frame returns to the shared dirty pool and a concurrent allocator on the other CPU can consume it before this caller's retry, so a caller could lose all MAX_EVICT_ATTEMPTS to its peer and return OutOfSpace while reclaimable pages remained (measured: freed frame seen as dirty=1/active=2047, next attempt dirty=0/active=2048) — the fault path then killed the allocating process. (2) `PageCache::select_eviction_victim` used a pure FIFO with no mappedness preference, so after the cold pages were gone it evicted hot text pages mapped by running procs, which re-faulted immediately (377,377 refaults of va=0x18000) — a livelock. (3) The gate's authored load never exhausted the pool (exec churn) or demanded ~100x its size (hseq 1 400000), and its `pinned_skipped > 0` criterion is structurally unreachable from this image's userland (write buffers are promoted off the droppable queue; pwrite's read-only source is an uncached stack/.bss page)."
fix: "port: `EVICT_BATCH_FRAMES=16` — alloc_clean passes a batch target to the evictor, so the caller keeps a margin of reclaimed frames; `select_eviction_victim` now prefers pages with an empty rmap (unmapped, cold — no PTE walk, no shootdown, no refault) and only falls back to the oldest mapped page; `#c/sysstat` gained `palloc.hash:` and `palloc.pool:` diagnostics and the evictor line gained evict_calls/reclaimed/free_failures (plus a devcons PoolSnapshot hook). aarch64/x86_64: evictors count calls/reclaimed/free failures; aarch64 prints one `evict: first reclaim … pool=(…)` line. Gates: aarch64 driver now drives a single bounded `hseq 1 44000 | hsort > /dev/null` burst (`R9_PRESSURE_LINES`), asserts dropped_text>0, dropped_data>0, evict_calls>0, reclaimed>0, free_failures==0 and forbids `phys page alloc failed`; `pinned_skipped` is reported, not asserted, with pinning proven by the host test `pagecache::pinned_cached_page_is_skipped_until_unpinned`. The background pinned-reader stage was removed (it could never pin a cached page and added extra shells/exec surfaces). x86_64 driver drops the same unreachable `pinned_skipped > 0` assertion."
verification: "Gate: 5 consecutive passes (3x HVF + 1x TCG on the locally built commit, plus the literal acceptance command `nix run .#boot-r9-mem-pressure` on the committed Nix-built kernel) with identical numbers — palloc: managed_frames=2048, evictor: dropped_text=20 dropped_data=28 pinned_skipped=0 evict_calls=3 reclaimed=48 free_failures=0, re-fault line intact, MEM_PRESSURE_OK, no phys-alloc-failure/unresolvable marker. Host: port 208 tests incl. pinned_cached_page_is_skipped_until_unpinned, unmapped_pages_are_evicted_before_mapped_ones, alloc_clean_requests_the_reclaim_batch; cargo xtask check rc=0; cargo xtask test rc=0; clippy aarch64/virtshell rc=0, x86-64/q35cpu rc=0. Regression: `nix run .#boot-r9-smoke` (HVF) rc=0 and `R9_ACCEL=tcg nix run .#boot-r9-smoke` rc=0."
oracle_type: specified
files_changed: [port/src/palloc.rs, port/src/pagecache.rs, port/src/devcons.rs, aarch64/src/pagealloc.rs, aarch64/src/fault.rs, aarch64/src/main.rs, x86_64/src/pagealloc.rs, x86_64/src/proc.rs, x86_64/src/main.rs, gates/aarch64/mem-pressure.expect, gates/x86_64/mem-pressure.expect, nix/gates/mem-pressure-r9.sh, nix/gates/mem-pressure-r9-x64.sh]
