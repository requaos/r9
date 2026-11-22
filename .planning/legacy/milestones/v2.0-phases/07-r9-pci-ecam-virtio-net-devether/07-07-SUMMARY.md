---
phase: 07-r9-pci-ecam-virtio-net-devether
plan: 07
subsystem: infra
tags: [r9, devether, netif, virtio-net, pci, dev-ether, ether, arp, ping-pong, tx-blast, wedge-hunt, netdrv-03, netdrv-04, qmsg, namec, await, aarch64, slirp, filter-dump, busywait-free, forbidden-unsafe]

# Dependency graph
requires:
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: 07-03 (the virtio transport + Ring seams + Posted/Replenish accounting shape), 07-04 (the baked guest payload: arp.req fixture + net-sweep.rc + netblast + dd), 07-05 (the virtnet gate profile + the pcap/serial assertion suite), 07-06 (the bound virtio-net driver: published device state, rings live, IRQ armed, counters)
provides:
  - port::netif (Netif/Netfile model, NETQID qid scheme, 8-slot conversation table, the BusyCell IRQ handoff with per-conversation 64 KiB whole-frame stores + fixed WaitSets + live counters, EtherHooks) — forbid(unsafe_code)
  - port::devether (#l: attach(spec) + Enodev absence path, the three-level netif.h file tree, clone/open/close with inuse + last-close reset, the blocking whole-frame Qmsg data read, the netif-faithful TX write path (bounds + src-MAC overwrite + block-on-full), ctl verbs, stats/addr/type/ifstats content) — forbid(unsafe_code)
  - namec/dat/syscall wiring: DEV_ETHER=8, the 'l' registry arm, full per-op dispatch, Chan::dup inuse bump
  - the virtnet gate GREEN in both modes on r9 HEAD 2afa3d6: arp ping-pong (4 requests field-exact + 4 slirp replies, pcap == 2N, stats in:4/out:4, ifstats drained + nintr in [1,4]) and the 256-frame TX blast (pcap == sent == stats out == 256, in == 0, nintr_tx=86, zero soverflows, NET-DONE, negative scan clean)
  - committed evidence (logs + pcaps) under the phase dir per D-06/D-13
  - two gate-exposed pre-existing r9 kernel fixes: sys_await %q status quoting + MAX_PROCS 16→64
  - Harvey-side payload fixes: /bin/bind and the /net mount-point dir in the baked image
affects: [07-08, phase-8 (devip/arp/ethermedium binds to #l; the TX loopback omission + the read path's gate-unexercised status are named for it), phase-9]

# Actuals (#2632) — pairs with the plan's estimate (tokens 90000, confidence: low).
actuals:
  tokens: 25558    # chars/4 over the realized r9 diff (92e9b26..2afa3d6, 98890 chars ≈ 24722) + the harvey build-script diff (~836); the committed evidence logs/pcaps are bulk gate captures, not authored tokens
  tasks: 2
  commits: 2        # MEASURED r9: git rev-list --count 92e9b26..2afa3d6 = 2; harvey: 2 plan commits (7e5cdb965 build fixes, 0a8d76d8d evidence) + the SUMMARY metadata commit rides on top

plan_head_before: 92e9b2671fee91777a1db74fa72efda330738f10   # r9 repo (code commits)
plan_head_after: 2afa3d6ebfde3a6a5fcc123d2cfaf89fd6dc0466    # r9 repo — BOTH gate modes green on this exact HEAD (the evidence is its captures)

# Tech tracking
tech-stack:
  added: []   # zero new dependencies anywhere (T-07-SC vacuous: no Cargo.toml/Cargo.lock touch)
  patterns:
    - "The BusyCell IRQ<->syscall handoff (netif.rs INGRESS): the mcslock module's non-spinning sibling as the plan's sanctioned 'small lock-free handoff' — provably uncontended on this kernel (syscalls IRQ-masked; the handler runs only from EL0/the idle wfi), fail-fast None instead of a spin-deadlock; register -> drop -> block is IRQ-atomic here, so a wakeup is un-losable"
    - "No allocation and no mcslock from IRQ context: per-conversation frame stores preallocated at clone-open (43 x 1514 B slots ~= the 64 KiB OQ-3 bound); waiter sets are fixed [u64; 16] arrays (one waiter per proc, MAX_PROCS-bounded) drained into a stack buffer"
    - "Driver-ops seam: devether::EtherDriverOps { mac, tx_post, ifstats } published by the arch bind (publish_driver) — port never sees arch; the ops' fns touch the publish-once VIRTIO_NET state in syscall context (IRQ-masked = mutually exclusive with the handler by the kernel invariant)"
    - "Qmsg whole-frame reads (one frame per read, excess discarded on a short buffer — qio.c's qread Qmsg arm); the read path is complete but gate-unexercised per RESEARCH OQ-1 (recorded loudly in both module headers)"
    - "Gate-exposed latent-bug pattern: a red gate that names the sweep stage drove root-causing to TWO pre-existing kernel bugs (await status quoting inverting rc's ||/&& for successful children; the never-reaped proc table as a fork budget) — the file contract's first real consumer is the audit"

key-files:
  created:
    - r9:port/src/netif.rs
    - r9:port/src/devether.rs
    - .planning/phases/07-r9-pci-ecam-virtio-net-devether/evidence/07-07-virtnet-arp.log
    - .planning/phases/07-r9-pci-ecam-virtio-net-devether/evidence/07-07-virtnet-arp.pcap
    - .planning/phases/07-r9-pci-ecam-virtio-net-devether/evidence/07-07-virtnet-blast.log
    - .planning/phases/07-r9-pci-ecam-virtio-net-devether/evidence/07-07-virtnet-blast.pcap
  modified:
    - r9:port/src/lib.rs (module registration, both forbid-attributed)
    - r9:port/src/dat.rs (DEV_ETHER=8 + the Chan::dup inuse-bump rule)
    - r9:port/src/namec.rs (the 'l' registry arm + walk/open/stat/close dispatch arms)
    - r9:port/src/virtio.rs (tx_post -> Posted{slot,notified}; tx_next_slot + used_idx accessors)
    - r9:port/src/virtio_tests.rs (call-site updates + the suppression-flag pins)
    - r9:aarch64/src/platform/virt/pci.rs (TX frame pages bound at bind, the driver ops, publish_driver, the RX retire's etheriq demux + the TX retire wake)
    - r9:aarch64/src/main.rs (EtherHooks registered alongside devpipe's)
    - r9:aarch64/src/syscall.rs (pread/pwrite DEV_ETHER arms; sys_await %q status quoting)
    - r9:aarch64/src/proc.rs (MAX_PROCS 16 -> 64)
    - r9:xtask/src/pcap.rs (reply-length assertion accepts the live slirp shape; runt-reply pin test)
    - build/aarch64/mkimage.sh (/net mount-point dir + manifest assert)
    - build/aarch64/subset-build.sh (bind per-target install + host assert)
    - .planning/phases/07-r9-pci-ecam-virtio-net-devether/deferred-items.md (07-07 findings)

key-decisions:
  - "07-07: the IRQ-side handoff is a BusyCell (netif.rs INGRESS) — the plan's sanctioned executor's-safest-reading of the devpipe precedent: provably uncontended by the kernel's IRQ-masked-syscall invariant, fail-fast on violation, zero allocation in IRQ context (preallocated frame stores + fixed WaitSets); the Netif mcslock stays the syscall-side LEAF lock (NETIF -> INGRESS is the one permitted nesting, never the reverse)"
  - "07-07: nonblocking is LOUD-UNIMPLEMENTED (not a silent no-op): there is no output queue this era — writes are synchronous ring posts, so accepting the verb would lie (the plan's either-way discretion, taken loud and documented in devether.rs's header)"
  - "07-07: sys_await's status field is %q-quoted (SINGLE quotes, only when empty/whitespace/quote) — the donor waitmsg shape libc wait()'s tokenize strips; the prior double-quoted format made EVERY successful child read as FAILED to rc's ||/&& (the sweep's bind gate exposed it; the 05-era chain never branched on a child status, so it never showed)"
  - "07-07: MAX_PROCS 16 -> 64 with the budget semantics documented in proc.rs: slots are never reaped this era (the exit-sweep deferral), so the table is a boot-session fork budget — the sweep forks ~20 times and wedged at 16 ('proc: table full' at arp-send 3); the reaping fix is the tracked follow-up (deferred-items.md)"
  - "07-07: slirp's ARP replies arrive at 64 bytes (its padding, never ours — measured on the committed pcap); the 07-05 reply-length assertion relaxed to >=60/runt-fails (the plan's gate truth pins oper=2 + address fields, never a reply length)"
  - "07-07: the evidence commit landed entangled with a PARALLEL Phase-6 commit via a mistaken --amend (d1a08212d carries the evidence refresh) — tree content verified byte-correct at HEAD; no history surgery on the live shared master (Issues Encountered)"

patterns-established:
  - "netif file contract in r9: NETQID(i,t) qids, three-level tree with the donor's conversation-0 stats/ifstats aliasing (NSTATQID/NIFSTATQID are interface-level wherever they appear), readnum's 12-byte fixed shape, the stats block's exact field order, addr bare-hex-no-newline, four-string stat records"
  - "the driver-hook ifstats line `vq<q>: avail=<a> used=<u> lastused=<l> nintr=<x> nnote=<m>` as THE NETDRV-04 reconciliation surface (07-05's parser consumes exactly this)"
  - "the wedge-hunt write contract: a full TX ring BLOCKS on the wake hook until the retire frees a slot — a stalled retire wedges the writer and the gate's deadline catches it (never a silent drop, D-08)"

requirements-completed: [NETDRV-03, NETDRV-04]  # copied verbatim from plan frontmatter; REQUIREMENTS.md marking is gated by ready-ids — NETDRV-04 marks here (07-07-only); NETDRV-03 is shared with 07-08 (no SUMMARY yet) and stays open until it lands

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "The #l file tree per netif.h over the bound virtio-net driver: /net/ether0/{clone,addr,stats,ifstats,N/{data,ctl,stats,type,ifstats}} with clone/open/close semantics, four-string stat records, the stats/addr/type content byte-faithful to netif.c, and live counters (inpackets/outpackets/soverflows)"
    requirement: NETDRV-03
    verification:
      - kind: e2e
        ref: "cargo xtask boot-test --arch aarch64 --config virtnet PASS (the typed sweep exercises bind/clone/connect/dd/stats/ifstats through the real file contract; the log carries '#l0: virtio: 1000Mbps: 52:54:00:12:34:56', 4x 'netsweep: arp sent', the stats block in:4/out:4, both vq lines, NET-DONE; zero 'unhandled irq' lines)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The D-07 real-file-contract ARP ping-pong reconciling EXACTLY: the pcap carries exactly 8 frames (4 requests sha=the negotiated MAC parsed from the bind line, spa=10.0.2.15, tpa=10.0.2.2, len=60; 4 replies oper=2 spa=10.0.2.2 tpa=10.0.2.15 tha=the guest MAC), stats in:4/out:4, ifstats used==lastused per queue with nintr in [1,4]"
    requirement: NETDRV-03
    verification:
      - kind: e2e
        ref: "pcap::assert_arp_pingpong ran in finish() (BootProfile::Net) and passed on the live capture (evidence/07-07-virtnet-arp.pcap = 8 frames, verified byte-exact vs the committed blob); xtask suite 47/47 incl. the synthetic goldens/negatives"
        status: pass
    human_judgment: false
  - id: D3
    description: "The D-08 TX-bulk wedge-hunt under sustained load: netblast's 256 pure-TX frames reconcile exactly (pcap frames == 256 == netblast: sent == stats out; stats in == 0; zero replies; zero soverflows; nintr_tx=86 in [1,256]; used==lastused; NET-DONE inside the budget; negative scan clean) — the block-on-full write path + IRQ retire + wake machinery held with no silent virtqueue wedge"
    requirement: NETDRV-04
    verification:
      - kind: e2e
        ref: "R9_NET_SWEEP_MODE=blast cargo xtask boot-test --arch aarch64 --config virtnet PASS on the same r9 HEAD (2afa3d6) as the arp leg; evidence/07-07-virtnet-blast.{log,pcap} committed (pcap = 256 frames, all oper=1, verified byte-exact)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Regression invisibility: all seven non-net aarch64 virt profiles green and the workspace CI legs (test/check/clippy) green at the final r9 HEAD; raspi/x86_64/riscv64 builds unaffected (the port additions compile everywhere)"
    requirement: NETDRV-04
    verification:
      - kind: e2e
        ref: "for p in virt virtlowecam virtexec virtexec3 virtmulti virtnotes virtpanic — all PASS at 2afa3d6; cargo xtask test (port 45/45, xtask 47/47) + check + clippy all exit 0 at 2afa3d6"
        status: pass
    human_judgment: false
  - id: D5
    description: "The blocking whole-frame Qmsg data-file read path (register -> drop -> block -> wake on ingress, EOF on last close) — complete and correct per netif's qread semantics but GATE-UNEXERCISED by design (RESEARCH OQ-1: no Phase-7 gate blocks on a data read; the ARP gate's RX proof is the stats counter + pcap)"
    requirement: NETDRV-03
    verification: []
    human_judgment: true
    rationale: "Deliberate plan disposition (07-07 must_haves + RESEARCH OQ-1): the read path's correctness is code-review-visible and its wake machinery shares the TX path's gate-proven hooks, but no automated gate this phase ever blocks on a data read. Phase 8's ethermedium/arp binding is the first real consumer; the omission is recorded loudly in both module headers per the plan's rule."
  - id: D6
    description: "Committed evidence (both logs + both pcaps) under the phase dir with the r9 SHA recorded (D-06's commit-both rule), each copy riding its own green run"
    requirement: NETDRV-04
    verification:
      - kind: other
        ref: "git ls-files --error-unmatch on all four + content-mode guards (arp log has 'netsweep: arp sent' x4, blast log has 'netblast: sent 256', both pcaps non-empty) + committed blobs byte-identical to the 2afa3d6 gate captures (sha256-verified at summary time)"
        status: pass
    human_judgment: false

# Metrics
duration: 63 min
completed: 2026-11-05
status: complete
---

# Phase 07 Plan 07: devether `#l` + the netif model on the virtio-net driver — both gates green Summary

**`/net/ether0` exists per netif.h on the 07-06 driver — the full three-level file tree with live counters and the blocking whole-frame Qmsg data path — and the phase's payoff gates are both green on r9 HEAD 2afa3d6: the guest's four ARP requests crossed the real userspace→devether data-file path into the host pcap (exactly 8 frames, field-exact ping-pong, stats in:4/out:4, both rings drained, nintr in [1,4] per queue), and the 256-frame TX blast reconciled exactly with no wedge (pcap == sent == stats out == 256, zero RX, nintr_tx=86, zero soverflows) — with two gate-exposed pre-existing kernel bugs root-caused and buried en route (sys_await's status quoting inverting rc's status branches for successful children; the never-reaped proc table wedging the sweep mid-loop).**

## Performance

- **Duration:** 63 min
- **Started:** 2026-11-05T01:11:10Z
- **Completed:** 2026-11-05T02:13:55Z
- **Tasks:** 2 (1 tracer + 1 auto)
- **Files modified:** 13 r9 + 2 harvey build scripts + 4 harvey evidence artifacts + 1 phase deferred-items

## Accomplishments

- **Task 1 (tracer):** `port::netif` (830 lines) + `port::devether` (715 lines), both `#![forbid(unsafe_code)]` (the compile-enforced zero-unsafe rule rides the attribute, counted per file + lib.rs's outer registrations = 23 total). The Netif model (qid scheme, 8-slot conversation table, counters, EtherHooks) with the BusyCell IRQ handoff (preallocated per-conversation 64 KiB whole-frame stores, fixed waiter sets, zero IRQ-context allocation/locking). The device surface: attach's controller parse + Enodev absence path, the netifgen-faithful tree (modes, owner-collapse 0666→0660, four-string records, the conversation-0 stats/ifstats aliasing preserved), clone→conversation-ctl qid rewrite, readnum/readstr byte shapes, the stats block's exact field order, the `vq<q>: …` ifstats line, ctl verbs at the plan's locked depth (connect N functional + typeinuse; promiscuous accepted no-op; nonblocking/addmulti/remmulti/scanbs/bridge/headersonly loud; unknown loud). The write path is netif-faithful (60..=1514 bounds, src-MAC overwrite, block-on-full via the wake hook).
- **The wiring:** DEV_ETHER=8 in dat.rs; the `'l'` registry arm + walk/open/stat/close arms in namec.rs; the pread/pwrite dispatch arms in syscall.rs; Chan::dup's inuse-bump rule (the devpipe ref_open_end analogue); EtherHooks registered in exec_bootstrap alongside devpipe's.
- **pci.rs's 07-06 handler grew the real RX path:** the retire strips the 12-byte header, reads the frame from the slot's page (acquire-ordered via next_used's fence), demuxes per etheriq (inpackets++ before demux; type/-1 match + tome/multi/prom gate; queue-full → soverflows++), recycles the page, wakes blocked readers; the TX retire wakes blocked writers. TX frame pages (one per slot) bound at bind; the driver ops (tx_post/ifstats) published via `devether::publish_driver` before the GIC arm.
- **virtio.rs seam evolution (07-06's sanctioned shape):** tx_post returns `Posted{slot, notified}` (exact nnote under suppression); `tx_next_slot` + `used_idx` accessors for the driver ops; the 22+2 port tests green with strengthened suppression pins.
- **Task 2:** both gate modes green on the same HEAD with the evidence committed — each capture copied in the same shell invocation as its own green run (the arp evidence then refreshed once to track the attribute-only cleanup commit, so the committed captures are byte-identical to the FINAL gated HEAD 2afa3d6).
- **Tracer feedback gate:** interactive + end-of-phase + automated-only verify → all three verify legs re-ran end-to-end post-commit (arp gate, seven-profile regression, workspace test/check/clippy) — TRACER-GATE-ALL-GREEN; expanded per the row-3 auto-continue.

## Task Commits

Each task was committed atomically per the dual-repo contract:

**r9 repo (branch main):**
1. **Task 1 (tracer): devether #l + netif model + virtio-net TX/RX wiring — the arp-mode gate green** — `f44b31f` (feat) — 12 files
2. **Attribute cleanup: inner forbid(unsafe_code) markers on the two new modules** — `2afa3d6` (style) — 2 files, no behavior change

**Harvey repo (branch master):**
3. **Gate-exposed payload fixes: /bin/bind + the /net mount-point dir join the baked image** — `7e5cdb965` (feat) — 2 files
4. **Evidence: both gate logs + both pcaps (D-06 commit-both)** — `0a8d76d8d` (test) — 4 files (the refresh to the final-HEAD captures rode into the parallel Phase-6 commit `d1a08212d` — see Issues)

**Plan metadata:** the docs commit (this SUMMARY + deferred-items + STATE/ROADMAP/REQUIREMENTS) rides on top in the Harvey repo.

## Files Created/Modified

- `r9:port/src/netif.rs` (NEW, 830) — the model: qid helpers, Netif/ConvBook under the LEAF mcslock, the BusyCell INGRESS (ConvSlot + FrameStore + WaitSet + counters), EtherHooks, the lifecycle ops (alloc/open/close/connect/promiscuous), the IRQ entries (rx_ingress/tx_retired/tx_posted), the read-path pop/register entries, the stats/addr content builders
- `r9:port/src/devether.rs` (NEW, 715) — the #l device: EtherDriverOps + publish_driver, attach/walk/open/stat/read/write/close/ref_open, the ctl verb set, the listings, readnum/readstr, the blocking Qmsg read loop, the block-on-full write loop
- `r9:port/src/lib.rs` / `dat.rs` / `namec.rs` — registration, DEV_ETHER, dispatch arms, the dup rule
- `r9:port/src/virtio.rs` + `virtio_tests.rs` — the Posted seam + accessors, test pins
- `r9:aarch64/src/platform/virt/pci.rs` — TX pages, driver ops, publish_driver, the RX demux + TX wake in the handler, the two-writers discipline comments
- `r9:aarch64/src/main.rs` — the EtherHooks registration
- `r9:aarch64/src/syscall.rs` — the DEV_ETHER pread/pwrite arms + the sys_await %q fix
- `r9:aarch64/src/proc.rs` — MAX_PROCS 64 with the budget-semantics comment
- `r9:xtask/src/pcap.rs` — the reply-length relaxation + the runt/padding pin test
- `build/aarch64/mkimage.sh` / `subset-build.sh` — /net + /bin/bind in the image (insertion-only per the 07-04 discipline)
- the four evidence artifacts under the phase dir
- `deferred-items.md` — the 07-07 findings (the proc-table reaping wall, slirp's 64-byte reply shape)

## Decisions Made

Surfaced in the frontmatter key-decisions; the short forms:

1. **BusyCell for the IRQ handoff** — the plan's sanctioned safest reading; provably uncontended here, fail-fast on violation, zero IRQ allocation/locking.
2. **`nonblocking` loud-unimplemented** — there is no output queue this era; accepting the verb would lie.
3. **sys_await %q quoting** (the gate-exposed latent bug — see Deviations 3).
4. **MAX_PROCS 64** with the fork-budget semantics documented (Deviations 4).
5. **The slirp 64-byte reply shape accepted by the gate** (Deviations 6) — reply padding is slirp's business, never our contract.
6. **No history surgery after the evidence-commit collision** — the live shared master had a parallel Phase-6 commit land on top of the evidence commit between my runs; the refresh rode a mistaken --amend into it, was verified byte-correct at HEAD, and is documented rather than rewritten (Issues).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The read/write dispatch lives in syscall.rs, not namec.rs**
- **Found during:** Task 1 pre-implementation ground-truthing (namec.rs carries walk/open/stat/close; the pread/pwrite device dispatch is aarch64/src/syscall.rs's match)
- **Issue:** the plan's Task-1 step 4 names namec.rs's "dev_read/dev_write" arms; no such arms exist there — the dispatch is arch-side.
- **Fix:** DEV_ETHER arms added in syscall.rs's sys_pread/sys_pwrite (routing to devether::read/write) alongside the namec.rs arms; the plan's intent (per-op dispatch routing) realized at its real sites.
- **Files modified:** r9:aarch64/src/syscall.rs
- **Verification:** the sweep's dd/cat exercised both arms live (gate green)
- **Committed in:** `f44b31f`

**2. [Rule 3 - Blocking] pci.rs is required but absent from files_modified**
- **Found during:** Task 1 planning (the plan's own step 3 rewires "the 07-06 handler's RX retire", which lives in pci.rs; the driver ops and publish_driver must live where the device state lives)
- **Issue:** the file list names only the port modules + main.rs; the handler/seam half has no home in it.
- **Fix:** pci.rs edited per the plan's step-3 text (RX demux, TX wake, TX pages, ops, publish); the dual-repo contract's never-stage-outside-scope rule read against the plan's ACTION text, which explicitly assigns this work.
- **Files modified:** r9:aarch64/src/platform/virt/pci.rs
- **Verification:** both gates green; the handler's behavior is the gates' subject
- **Committed in:** `f44b31f`

**3. [Rule 1 - Bug] sys_await double-quoted the status — rc's `||` fired on successful children**
- **Found during:** Task 1's first green-candidate run (the bind SUCCEEDED — a temporary `bind-dbg` kernel print proved `ok=true` — yet the sweep printed `netsweep: FAIL bind`: rc's `||` saw a non-empty status for a child whose status was empty)
- **Issue:** r9's await record formatted the status as `"{}"` (DOUBLE quotes); libc wait()'s tokenize (gettokens' qtoken) strips only SINGLE quotes (the %q shape), so a successful child's empty status arrived as the two-character string `""` — non-empty — and EVERY `cmd || …` / `&&` on a child status inverted. Latent since the 05-era (no earlier script branched on a child status).
- **Fix:** sys_await formats the status per libc fmtquote's %q rule (bare unless empty/whitespace/control/quote; single-quoted with doubling otherwise). The temporary debug print was removed before the commit.
- **Files modified:** r9:aarch64/src/syscall.rs
- **Verification:** the sweep's bind/clone/connect/dd chain then ran clean (arp sent ×4); the boot chain's own status flows (stacksmoke, exec) unaffected — all seven other profiles green twice
- **Committed in:** `f44b31f`

**4. [Rule 3 - Blocking] The proc table is a fork budget and the sweep overflows it**
- **Found during:** Task 1 gate iteration (after the await fix: `arp sent 1..3` then `rc: try again: proc: table full` and a deadline wedge)
- **Issue:** PROC_TABLE slots are never reaped this era (a Moribund husk pins its slot — the documented exit-sweep deferral), so 16 slots = 16 forks per boot session; the sweep forks ~20 (dd + echo per iteration, bind/clone/cat/test machinery).
- **Fix:** MAX_PROCS 16 → 64 with the comment rewritten to state the budget semantics and name the reaping follow-up (deferred-items.md). Reaping is NOT attempted this era — the ProcGuard raw-pointer discipline ("slots never freed/moved") is load-bearing.
- **Files modified:** r9:aarch64/src/proc.rs
- **Verification:** the full sweep completes (both modes) with headroom; seven-profile regression green twice
- **Committed in:** `f44b31f`

**5. [Rule 3 - Blocking] The sweep's `bind` couldn't run at all: /bin/bind and /net both missing from the baked image**
- **Found during:** Task 1 gate iteration (run 1: `can't exec: namec: does not exist` — bind is NOT an rc builtin; the boot chain's own binds are init's /lib/namespace interpretation. Run 2: /bin/bind present but the target resolution failed — /net absent and the paqfs root is read-only this era, so no runtime mkdir)
- **Issue:** 07-04's payload added dd/netblast/the fixture but not the sweep's FIRST command's binary, and nobody had ever needed /net in the scoped image.
- **Fix:** bind joins SUBSET (subset-build.sh per-target install + host assert; mkimage.sh SUBSET append — insertion-only per the 07-04 discipline); mkimage.sh gains the /net empty dir + manifest assert.
- **Files modified:** build/aarch64/subset-build.sh, build/aarch64/mkimage.sh (Harvey repo)
- **Verification:** the re-baked bootfs.paq carries both (the paqfs root listing prints `net`; bind execs and succeeds); gate green
- **Committed in:** `7e5cdb965` (harvey)

**6. [Rule 1 - Bug] The 07-05 reply-length assertion pinned a synthetic-fixture fiction**
- **Found during:** Task 1's first pcap-asserting run (`ARP reply #0: len got 64 want 60`)
- **Issue:** the assertion pinned reply frames at exactly ETHERMINTU=60 (the fixture's shape); the live capture shows slirp's replies at 64 bytes (slirp pads its replies past the Ethernet minimum). The plan's own gate truth pins replies to oper=2 + the address fields — never a reply length; our kernel's ETHERMINTU enforcement is only about OUR writes.
- **Fix:** replies accept len ≥ 60 (a runt reply still fails loudly); requests stay pinned at exactly 60; a pin test (64-byte passes, 59-byte fails) added. xtask suite 47/47.
- **Files modified:** r9:xtask/src/pcap.rs
- **Verification:** the gate green on the live capture; the synthetic goldens still pass; the runt negative pinned
- **Committed in:** `f44b31f`

**7. [Rule 2 - Missing Critical] virtio.rs's tx_post couldn't report the notify truth and exposed no slot peek**
- **Found during:** Task 1 implementation (the driver op needs the slot BEFORE posting to copy the frame into its page, and nnote must stay exact under suppression — the 07-06 Replenish evolution's TX twin)
- **Fix:** tx_post returns `Posted { slot, notified }`; `tx_next_slot()` (the free-slot peek) + `used_idx()` (the ifstats `used=` field) added; the test call sites updated and the suppression tests strengthened with the flag.
- **Files modified:** r9:port/src/virtio.rs, r9:port/src/virtio_tests.rs
- **Verification:** port suite green (45/45 incl. the virtio tests); the gates' ifstats lines reconcile (nnote_tx=4 arp / 247 blast — qemu's suppression under load observed and counted exactly)
- **Committed in:** `f44b31f`

**8. [Rule 2 - Missing Critical] IRQ-context allocation/locking discipline made concrete**
- **Found during:** Task 1 design (a VecDeque<Vec<u8>> per-conversation queue would allocate in IRQ context; a Vec waiter set would free in IRQ context at drain-drop; the Netif mcslock is IRQ-forbidden by the plan)
- **Fix:** per-conversation frame stores preallocated at clone-open (43 × 1514 B ≈ the 64 KiB OQ-3 bound, circulating slots — zero IRQ allocation); waiter sets are fixed `[u64; 16]` arrays (one waiter per proc, MAX_PROCS-bounded) drained into a stack buffer; the whole handoff state rides a BusyCell (the mcslock module's non-spinning sibling — provably uncontended on this kernel, fail-fast None instead of a spin-deadlock). Documented in netif.rs's header (the LEAF rule verbatim + the one permitted NETIF → INGRESS nesting).
- **Files modified:** r9:port/src/netif.rs
- **Verification:** the gates exercised the handoff under load (blast: 86 TX IRQs retiring 256 frames with blocked writers waking; zero wedge)
- **Committed in:** `f44b31f`

**9. [Rule 3 - Blocking] Plan's verify commands omit `--arch aarch64`**
- **Found during:** Task 1 verification (the 07-01/07-02/07-05/07-06 class — xtask Arch::from defaults to x86_64)
- **Fix:** all boot-test invocations ran as `cargo xtask boot-test --arch aarch64 --config <p>`.
- **Files modified:** none
- **Committed in:** n/a

---

**Total deviations:** 9 auto-fixed (3 bug, 5 blocking, 2 missing-critical — counting the shared-class plan-command correction). **Impact on plan:** two of the fixes (3, 4) are genuine pre-existing KERNEL bugs/limitations the gate was designed to expose — the file contract's first real consumer was the audit; both fixes are minimal and documented. The rest are the plan's intent realized through the codebase's real seams. No scope creep — every plan truth bullet holds as measured on the gates.

## Issues Encountered

- **Evidence-commit collision with the parallel Phase-6 execution:** while this plan ran, a Phase-6 executor committed `d1a08212d` on top of my evidence commit `0a8d76d8d` on the shared master. My evidence REFRESH (re-capturing at the final r9 HEAD 2afa3d6 per D-13's evidence==gated-HEAD rule) then ran `git commit --amend` against what I believed was my own tip and instead folded the refresh into Phase 6's commit. Verified after the fact: the tree at HEAD carries the correct final captures (committed blobs sha256-verified byte-identical to the 2afa3d6 gate runs); nothing of Phase 6's content was lost. No history surgery was attempted (the branch is live and shared). The blemish is that `d1a08212d`'s diff now includes my four evidence files' refresh — ownership entanglement, not corruption. The plan-ledger note: harvey-side plan commits are `7e5cdb965` (build fixes) + `0a8d76d8d` (evidence v1) + the refresh riding `d1a08212d`.
- **Debugging-through-the-gate was deliberate and slow-but-correct:** each red iteration produced a named stage (the 07-04/07-05 machinery's fast-fail contract working as designed): FAIL bind (exec) → FAIL bind (target) → the await/status inversion (proved with a one-line kernel print, removed before commit) → the fork-budget wall → the reply-length fiction. No gate was weakened to pass; the one assertion change (Deviation 6) RELAXED a fixture-fiction over-pin to the plan's own written gate truth.
- Pre-existing warnings observed and deliberately untouched (scope boundary): the raspi3b/riscv64 unused-import set and the bare-build dead-code set (22-ish on config builds) — my additions add zero net warnings (the virtio.rs accessors are all consumed; the removed `VirtioNetState.mac` field deleted rather than allow-listed).

## User Setup Required

None - no external service configuration required. (R9_HARVEY_DIR is the established cross-repo channel; the verify commands set it explicitly. R9_NET_SWEEP_MODE=blast selects the TX-bulk leg.)

## Next Phase Readiness

- **07-08 (Harvey compose + evidence + pin):** the packaged virtshell kernel's dev = ["exec", "bootchain", "ether"] binds the NIC and #l attaches; the virtnet gate is green machinery + green kernel. The flake.lock r9 pin advance can name a main HEAD that carries the whole phase (2afa3d6 or later) — remember push-before-pin (r9 main must be pushed BEFORE the pin commit resolves from a clean clone).
- **Phase 8 (devip/arp/ethermedium):** the #l file contract it binds to is live and proven. TWO named omissions are its to consume or close: (a) no TX loopback/broadcast feedback into the input path (etheroq → etheriq) — ethermedium expects it (both module headers record it loudly); (b) the blocking Qmsg read path is complete but Phase-7-gate-unexercised (OQ-1) — ethermedium's read loop is the first real consumer. Note the era semantics it must hold a conversation OPEN across: a conversation RESETS on last close (type 0, queue drained — netifclose verbatim), so a per-command open/close pattern (like the sweep's) never receives.
- **Era limits sharpened in deferred-items.md:** the proc-table reaping wall (any long-lived service era fills 64 too) and slirp's 64-byte reply shape (measured, committed pcap is the reference).
- No blockers.

## Self-Check: PASSED

- Files: r9 port/src/netif.rs + devether.rs FOUND on disk (830/715 lines ≥ the 150/200 minima); all four evidence artifacts FOUND and git-tracked
- Commits (r9): `f44b31f`, `2afa3d6` FOUND in git log; measured count `git rev-list --count 92e9b26..2afa3d6` = 2 ✓
- Commits (harvey): `7e5cdb965`, `0a8d76d8d` FOUND in git log
- Acceptance greps (re-run post-commit): lib.rs forbid count 23 (21+2); netif.rs+devether.rs forbid total 2; namec.rs `'l'` arm = 1; DEV_ETHER in dat.rs+namec.rs = 7 (≥2); devether.rs carries the bounds (`write too big`/`write too small`, minmtu=60 in the header + read from the model) + the src-MAC overwrite (`copy_from_slice(&ops.mac)`)
- Gates re-run at summary time on the final r9 HEAD 2afa3d6: arp-mode PASS + blast-mode PASS (both verify commands, same HEAD, no intermediate commits between the runs); the evidence grep guard `EVIDENCE-COMMITTED`; seven-profile regression + workspace test/check/clippy ALL GREEN at 2afa3d6; `grep -ci 'unhandled irq'` = 0 on both evidence logs; r9 worktree clean of plan files post-commit (`git status --porcelain` empty); pre-existing dirty files in both repos untouched
- Evidence integrity: committed blobs at harvey HEAD sha256-verified byte-identical to the 2afa3d6 gate captures; content-mode guards hold (arp log carries 4× `netsweep: arp sent` and no netblast line; blast log carries `netblast: sent 256`)

---
*Phase: 07-r9-pci-ecam-virtio-net-devether*
*Completed: 2026-11-05*
