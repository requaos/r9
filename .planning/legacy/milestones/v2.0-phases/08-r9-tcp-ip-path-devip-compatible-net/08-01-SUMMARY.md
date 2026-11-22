---
phase: 08-r9-tcp-ip-path-devip-compatible-net
plan: 01
subsystem: infra
tags: [smoltcp, devip, tcp, udp, icmp, icmpv6, ipifc, plan9, no_std, cargo, aarch64, r9]

# Dependency graph
requires:
  - phase: 07-r9-pci-ecam-virtio-net-devether
    provides: devether/#l file tree, virtio-net machinery, the HowItWorks.md provenance-record precedent (D-04 placement), the importCargoLock vendoring pattern
provides:
  - ../r9/docs/devip-contract.md — the D-03/D-08 contract doc: full enumerated devip surface (qid scheme, top-level files, conv semantics, ctl verbs, per-proto chapters, mediums, error vocabulary) with every man-vs-source divergence flagged, plus the r9 mapping chapter (event seam, accept pool with memory math, per-ipifc Interface demux, panic-boundary rule)
  - smoltcp 0.14.0 as r9 port/'s first external crate: crates.io pin with the locked feature set + committed Cargo.lock (D-01/D-02)
  - cfg(dev_ip) predeclared in the workspace check-cfg list ahead of first code use (08-04)
  - the joint NETIP-01/NETDRV-02 provenance record in ../r9/HowItWorks.md (D-04)
affects: [08-02 (host pins incl. OQ-1 pin (h) — its verdict lands in the contract doc), 08-03 (devip shell), 08-04 (loopback/null mediums, cfg(dev_ip) first use), 08-05..08-09 (all gates assert the contract doc)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as estimateTokens.
actuals:
  tokens: 16500    # (54980-char r9 diff + 696-char deferred-items + this SUMMARY) / 4
  tasks: 2
  commits: 3       # MEASURED: 2 r9 (8330700, 669c561) + 1 Harvey metadata commit (this file + STATE/ROADMAP/REQUIREMENTS + deferred-items)
plan_head_before: 26b6f614964c94def0be9776eb4cc4c0d9c2a960   # Harvey repo ledger base (#3968)
plan_head_after: 26b6f614964c94def0be9776eb4cc4c0d9c2a960    # Harvey HEAD at SUMMARY write — 0 Harvey commits yet by design (dual-repo plan: production commits landed in r9; the single Harvey metadata commit follows this write per the atomic close-out order)

# Tech tracking
tech-stack:
  added: [smoltcp 0.14.0 (managed 0.8.0, heapless 0.9.3, hash32 0.3.1, stable_deref_trait 1.2.1, byteorder 1.5.0, cfg-if 1.0.5; defmt 1.2.0 optional-uncompiled)]
  patterns: [contract-doc-as-spec (gates assert the doc, divergences flagged [man-vs-src]), crates.io-pin+committed-lockfile supply chain (D-02), joint same-axis provenance records (D-04), check-cfg predeclare before first code use]

key-files:
  created: [../r9/docs/devip-contract.md, deferred-items.md]
  modified: [../r9/port/Cargo.toml, ../r9/Cargo.toml, ../r9/Cargo.lock, ../r9/HowItWorks.md]

key-decisions:
  - "Accept pool depth = 128 (devip-exact Maxincall) with 2048/2048-byte socket buffers = 512 KiB per announced conv (12.5% of the 4 MiB QuickFit arena); ≤2 concurrent announced TCP convs in the phase gates → ≤25% worst case (OQ-3 resolved with the math shown)"
  - "tcp checksum/tcpporthogdefense ctl verbs: accepted-and-recorded with a netlog note (no smoltcp mechanism; D-08 full-fidelity stance over loud-unimplemented) (OQ-4 resolved)"
  - "One smoltcp Interface per bound ipifc with address-class demux (127/8 and ::1 → loopback iface; everything else → ether iface) (OQ-5 resolved)"
  - "Timer leg = one-shot arming from poll_at() over the existing 1 ms tick sweep; literal dynamic-CNTP-rearm considered and rejected (A5 resolved)"
  - "MAXTTL = 255 recorded from source (sys/src/9k/ip/ip.h:44) — the research's '64' parenthetical corrected; the bridge sets hop_limit 255 at conv creation so smoltcp's 64 default never leaks"
  - "r9 stack bound: ONE stack (spec 0); other specs error 'bad specification' (qid packing keeps donor field widths)"

patterns-established:
  - "Contract-doc-as-spec: enumeration from man × source with [man-vs-src] flags; gates assert the doc (D-03/D-08)"
  - "Panic-boundary rule: every smoltcp call site validates first and maps to a devip error string; a kernel panic from a ctl write or packet is a gate red"
  - "r9-first-external-crate shape: crates.io pin + committed Cargo.lock + importCargoLock re-vendoring proven by nix build .#kernel-virt"

requirements-completed: [NETIP-01, NETIP-02]

# Coverage metadata (#1602)
coverage:
  - id: D1
    description: "devip contract doc enumerating the full documented /net surface with divergences flagged and the r9 mapping decisions recorded (incl. accept-pool math)"
    requirement: NETIP-02
    verification:
      - kind: other
        ref: "test -s ../r9/docs/devip-contract.md && grep marker chain (QID(p,c,y), cannot bind null device, Maxincall, unknown control request, man-vs-src, hangupxmit, maxfragsize, tcpstates, 'accept pool') → CONTRACT-DOC-OK"
        status: pass
    human_judgment: false
  - id: D2
    description: "smoltcp 0.14.0 committed dependency with locked feature set + committed Cargo.lock + cfg(dev_ip) predeclare + joint NETIP-01/virtio provenance record; all four verify legs green (check, clippy, boot-test virt, nix kernel-virt)"
    requirement: NETIP-01
    verification:
      - kind: other
        ref: "grep chain on Cargo.lock/Cargo.toml/HowItWorks.md → DEP-AND-RECORD-OK"
        status: pass
      - kind: other
        ref: "cargo xtask check --verbose && cargo xtask clippy --verbose (r9) → exit 0, no smoltcp-named warnings"
        status: pass
      - kind: e2e
        ref: "cargo xtask boot-test --arch aarch64 --config virt → PASS (banner/ticks/echo probe); negative scan: 0 panic/Unhandled lines in target/boot-test-virt.log"
        status: pass
      - kind: integration
        ref: "nix build .#kernel-virt -o target/nix-kernel-virt-smoltcp-check → exit 0 (importCargoLock re-vendored from the smoltcp-carrying lockfile)"
        status: pass
    human_judgment: false

# Metrics
duration: 9 min
completed: 2026-11-05
status: complete
---

# Phase 8 Plan 01: devip Contract Doc + smoltcp Supply Chain Summary

**The D-03/D-08 contract doc (`../r9/docs/devip-contract.md`, 747 lines) enumerating the full devip `/net` surface from `sys/man` × `sys/src/9k/ip/` with every divergence flagged [man-vs-src] and the r9 mapping decisions recorded with their math — plus smoltcp 0.14.0 landed as r9 port/'s first external crate (crates.io pin, committed Cargo.lock, cfg(dev_ip) predeclared, joint NETIP-01/virtio provenance record), with check/clippy/virt-boot-test/nix-vendored-build all green.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-11-05T18:59:41Z
- **Completed:** 2026-11-05T19:09:12Z
- **Tasks:** 2
- **Files modified:** 6 (5 r9 + 1 Harvey planning)

## Accomplishments

- The contract doc lands FIRST as locked sequencing demands: nine chapters covering topology/qid scheme (`QID(p,c,y)` packing, all tag constants), top-level files (arp/iproute/ipselftab/ndb/log with exact format strings; bootp recorded documented-but-absent), protocol dirs + conv lifecycle (clone/owner-collapse/closeconv, Maxincall=128, listen semantics), generic ctl verbs with exact error strings, per-proto chapters (TCP 11-state rename table to smoltcp `tcp::State`, 28-tag stats list, `hangupxmit`/`maxfragsize` source-only verbs; UDP 52-byte Udphdr + Fsnewcall; ICMP lport-is-ident; ICMPv6 full-packet data semantics), the ipifc chapter (all verbs incl. add6/ra6, ipifcbind defaults, source status format), mediums (ether MTU-includes-header trap, loopback 16 KiB, null's "cannot bind null device" contract), and a 28-row error vocabulary table mapping every string to trigger + gate
- The r9 mapping chapter records every resolved open question WITH the math: accept pool depth 128 × (2048+2048) = 512 KiB per announced conv vs the 4 MiB arena (OQ-3), one Interface per bound ipifc with 127/8+::1 demux (OQ-5), checksum/tcpporthogdefense accepted-and-recorded (OQ-4), tick-sweep one-shot arming with the CNTP-rearm alternative rejected (A5), the arp snoop-view static-add divergence, and the `add 0.0.0.0 0.0.0.0` absorption rule (P-F)
- smoltcp 0.14.0 (checksum b6f8b28a…01980f, matching the research record) committed with exactly the locked feature set — no socket-dhcpv4 (D-09), no cubic, no slaac, no segmentation-offload — and the full named closure in Cargo.lock
- NETIP-01 provenance recorded beside the virtio record per D-04: opposite answers on the same adopt-vs-write axis, each named with its reason; Reversibility: costly
- Kernel still builds and boots with smoltcp in the tree: virt boot-test PASS (banner + ticks + echo probe, zero panic lines); `nix build .#kernel-virt` green (importCargoLock re-vendored from the new lockfile — Pitfall 16 closed by construction, no fixed-output hash to re-rive)

## Task Commits

Each task was committed atomically (r9 repo — this plan's files are all r9-side):

1. **Task 1: Write ../r9/docs/devip-contract.md** — `8330700` (feat) — r9 repo
2. **Task 2: smoltcp dependency + lockfile + cfg(dev_ip) + provenance** — `669c561` (feat) — r9 repo

**Plan metadata:** (Harvey repo — this SUMMARY + STATE/ROADMAP/REQUIREMENTS + deferred-items, committed after this write per the atomic close-out order)

## Files Created/Modified

- `../r9/docs/devip-contract.md` — NEW (creates r9's docs/ dir): the D-03 contract doc
- `../r9/port/Cargo.toml` — smoltcp 0.14.0 entry with locked feature set + D-01/D-02/D-09 comment block
- `../r9/Cargo.toml` — `cfg(dev_ip)` predeclared in workspace check-cfg (comment extended naming the 08-04 virtip TOML)
- `../r9/Cargo.lock` — regenerated: smoltcp 0.14.0 + managed/heapless/hash32/stable_deref_trait/byteorder/cfg-if/defmt(optional)
- `../r9/HowItWorks.md` — NETIP-01 provenance paragraph beside the virtio one
- `.planning/phases/08-r9-tcp-ip-path-devip-compatible-net/deferred-items.md` — NEW: pre-existing r9 warnings logged (out of scope)

## Decisions Made

- Accept pool depth = 128 (devip-exact) at 2048/2048 buffers = 512 KiB per announced conv, 12.5% of the 4 MiB arena; pools allocate at announce; ≤2 concurrent announced TCP convs in the phase gates (OQ-3)
- `checksum`/`tcpporthogdefense` accepted-and-recorded with netlog note, not loud-unimplemented (OQ-4; D-08 stance)
- One smoltcp Interface per bound ipifc, address-class demux 127/8+::1→loopback (OQ-5)
- Timer leg arms one-shot deadlines from `poll_at()` over the existing 1 ms tick sweep; literal CNTP rearm rejected (A5)
- MAXTTL = 255 recorded from the source (`ip.h:44`); research's "=64" corrected; bridge sets hop_limit 255 at conv creation
- r9 implements ONE stack this era (spec 0; others → "bad specification")

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] boot-test verify command needed `--arch aarch64`**
- **Found during:** Task 2 (verify leg 3)
- **Issue:** The plan's verify text runs `cargo xtask boot-test --config virt`; xtask's `Arch::from` defaults to x86_64 (`xtask/src/main.rs:60`), so the command as written boot-tests the wrong architecture
- **Fix:** Ran `cargo xtask boot-test --arch aarch64 --config virt` — the exact precedent recorded in STATE.md for Phase 07-01 ("plan verify commands ran with --arch aarch64 added; every historical boot-test invocation carries the flag")
- **Files modified:** none (invocation only)
- **Verification:** boot-test PASS — banner, ≥3 ticks, echo probe in order; negative scan clean
- **Committed in:** n/a (no file change)

**2. [Rule 1 - Bug] Plan/research parenthetical "ttl=MAXTTL=64" contradicted by source (MAXTTL=255)**
- **Found during:** Task 1 (read_first verification of `ipifc.c` defaults)
- **Issue:** The plan's action text and RESEARCH.md both say `ttl=MAXTTL=64` for ipifcbind defaults; the source says `MAXTTL = 255` (`sys/src/9k/ip/ip.h:44`, used at `ipifc.c:152` and `devip.c:1073,1336`)
- **Fix:** The contract doc records the SOURCE value 255 (the doc's own authority rule: source beats man — and beats research notes), flags the research discrepancy inline, and adds a §12.8 mapping decision: smoltcp's `set_hop_limit` default (64) must not leak; the bridge sets 255 at conv creation
- **Files modified:** ../r9/docs/devip-contract.md (content decision)
- **Verification:** marker grep chain passes; the 255 value + the leak-guard decision are in §4/§12.8
- **Committed in:** `8330700` (part of Task 1)

---

**Total deviations:** 2 auto-fixed (1 blocking invocation fix, 1 plan-text bug corrected to source truth)
**Impact on plan:** Both keep the plan's intent intact — the first matches the recorded 07-01 precedent exactly; the second is the contract doc doing its designed job (T-08-02: enumeration derives from source, divergences flagged, never smoothed). No scope creep.

## Issues Encountered

- None beyond the deviations above. Pre-existing r9 warnings (port test/lib, aarch64 bin, x86_64 devcons) observed during the verify legs — none name smoltcp, none caused by this plan; logged to the phase's `deferred-items.md` per the scope boundary.

## Authentication Gates

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for 08-02 (the seam's host-side pins): its Task 2 pin (h) resolves OQ-1 (DHCP src-0.0.0.0 TX); the verdict lands as a dated line in `docs/devip-contract.md` §12.6, replacing the "pin pending" marker
- 08-04's virtip TOML (`dev = ["ip"]`) will be the first cfg(dev_ip) EMISSION; the workspace predeclare already landed here per the predeclare rule
- r9's pre-existing uncommitted `aarch64/src/exec.rs` modification was left untouched and out of every commit, as instructed
- No blockers

## Self-Check: PASSED

- `../r9/docs/devip-contract.md` exists (747 lines, non-empty) — FOUND
- `../r9/port/Cargo.toml` contains `smoltcp = { version = "0.14.0"` with default-features = false and the locked feature list; `socket-dhcpv4` absent from the feature list — VERIFIED
- `../r9/Cargo.lock` contains `name = "smoltcp"` / `version = "0.14.0"` + the named closure — VERIFIED
- `../r9/Cargo.toml` check-cfg contains `cfg(dev_ip)` — VERIFIED
- `../r9/HowItWorks.md` contains the joint smoltcp/virtio provenance with `Reversibility: costly` — VERIFIED
- r9 commits `8330700` and `669c561` exist on r9 main (`git log --oneline -2`) — FOUND
- All four verify legs exit 0 (check, clippy, boot-test virt, nix kernel-virt) — VERIFIED at execution time
- raspi3b/raspi4b/x86_64/riscv64 stay green by construction (no cfg-gated code added; the shared port crate checks clean; x86_64 compiled in the check/clippy legs)

---
*Phase: 08-r9-tcp-ip-path-devip-compatible-net*
*Completed: 2026-11-05*
