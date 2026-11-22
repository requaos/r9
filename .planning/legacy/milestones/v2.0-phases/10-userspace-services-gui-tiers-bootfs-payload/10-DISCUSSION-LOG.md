# Phase 10: Userspace Services + GUI Tiers + Bootfs Payload - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-05
**Phase:** 10-Userspace Services + GUI Tiers + Bootfs Payload
**Areas discussed:** gs (ghostscript) disposition, Tier gate depth + r9 venue, GUI trace venue, Bootfs payload specifics

---

## gs (ghostscript) disposition

| Option | Description | Selected |
|--------|-------------|----------|
| Include, compile-gated, fix-or-exempt | Roadmap criterion 3's literal reading: gs joins the tier-4 sweep as a compile-gated member; fixes one commit per bug class; over-budget items excluded by name (D-08 pattern) | ✓ |
| Timeboxed attempt, then exempt | One plan attempts the compile; >3 distinct bug classes → documented-exempt; bounds whack-a-mole risk | |
| Exclude + document the gap | PITFALLS' recommendation: exclude from tiers, document as known gap (like Go), amend roadmap | |

**User's choice:** Include, compile-gated, fix-or-exempt
**Notes:** Scouting verified gs/mkfile needs `$objtype.h` (genarch on matching CPU or committed `default.$objtype.h`; no `default.aarch64.h` exists) and that genarch is already pcc-native-guarded (06-04 G6) — Pitfall-2 pre-cleared. The donor guest is a native arm64 Plan 9 system, so genarch could legally run there.

| Option | Description | Selected |
|--------|-------------|----------|
| Compile-only marker | Roadmap-literal: USPACE-09 scopes the long tail to 'compile' | |
| + headless render smoke | Compile marker PLUS gs renders a tiny checked-in .ps to a bitmap file device on the donor; .ps init/lib files staged into the guest | ✓ |

**User's choice:** + headless render smoke
**Notes:** User chose stronger than roadmap-literal — the Pitfall-3 net for 753 files of ported unix C. lp's gs-dependent stages stay excluded (PITFALLS); gs's GPL fonts never touch bootfs.

---

## Tier gate depth + r9 venue

| Option | Description | Selected |
|--------|-------------|----------|
| Functional probes for the named set | cs answers a lookup, dns resolves, exportfs serves a guest mount, listen accepts, mntgen mounts, ipconfig configures + ping reaches the host leg | ✓ |
| Functional clients, smoke daemons | Clients functional; daemons start + readiness marker + clean shutdown only | |
| Functional or exempt, no smoke tier | Everything with an execution marker must show function, else documented-exempt | |

**User's choice:** Functional probes for the named set
**Notes:** The donor runs a full 9front userspace (ethervirtio10 + ip proven Phase 2, devcap present) — functional probes need no r9 dependency.

| Option | Description | Selected |
|--------|-------------|----------|
| Compile all + shallow exec subset | ~80 files compile; factotum/keyfs/authsrv start+respond probes; real p9sk1 = Phase 11 on r9 | ✓ |
| Compile-only (Phase 11 owns exec) | Auth suite compile-gated only; deviates from criterion 1's 'runs on the donor' wording | |
| Full p9sk1 probe on donor | auth/debug loopback against donor authsrv this phase; duplicates Phase 11's core gate | |

**User's choice:** Compile all + shallow exec subset

| Option | Description | Selected |
|--------|-------------|----------|
| Donor-only; r9 consumes in 11/12 | No coupling to in-flight Phase 8 (4/9); ledgers name venue:donor | ✓ |
| r9 leg gated on Phase 8 | Tier-2 r9 leg unblocks when Phase 8's ether+DHCP lands; Phase 10 hostage to Phase 8 | |
| r9 leg for /net-free services only | exportfs/mntgen on r9 now — thin value (tier-1 already runs on r9) | |

**User's choice:** Donor-only; r9 consumes in 11/12

| Option | Description | Selected |
|--------|-------------|----------|
| Full ip/ tree compiles in tier-2 | ~40 dirs compile-gated; named core (ipconfig, ping) functional; x86-only excluded by name | ✓ |
| Named set in tier-2; rest to tier-4 | ip/ splits across two tiers | |

**User's choice:** Full ip/ tree compiles in tier-2

---

## GUI trace venue

| Option | Description | Selected |
|--------|-------------|----------|
| Host-served draw over 9P | Donor guest mounts host-served draw as /dev/draw — EXACTLY the Phase-12 topology (drawterm-served draw over devmnt); donor kernel pristine | ✓ |
| Donor config variant + softscreen | Guest-scratch kernel config adds devdraw; unverified 9front arm64 softscreen feasibility; measures the WRONG topology (no devmnt) | |
| drawterm-into-donor (noauth) | Full cpu/session topology on the donor; rehearses Phase 11 auth + Phase 12 plumbing on a throwaway venue | |

**User's choice:** Host-served draw over 9P
**Notes:** Verified during scouting: the 9front donor arm64 qemu kernel config has no draw/mouse/screen device — devdraw.c exists in port/ but is not compiled in. Decisive argument: kernel-local draw would measure the wrong syscall mix for r9.

| Option | Description | Selected |
|--------|-------------|----------|
| plan9port devdraw | Full /dev/draw file tree server, already a host dep, zero new protocol code, proven against real libdraw clients | ✓ |
| Rust harness fake-draw server | Scripted/deterministic/hermetic but reimplements devdraw semantics; mis-ack = garbage traces | |
| devdraw primary, fake-draw fallback | Keeps proven path primary with a fallback spike | |

**User's choice:** plan9port devdraw

| Option | Description | Selected |
|--------|-------------|----------|
| Scripted-interaction traces | Fixed short scripts drive real interaction (rio menu New + type; acme type/execute/quit; sam+samterm edit/quit) via input injection — captures libthread event loop, mouse/kbd, snarf | ✓ |
| Launch-settle-exit traces | Init/fonts/image-alloc/mouse-open captured; steady-state missed — cheaper, less flaky | |
| Staged: settle first, then interaction | Settle trace committed first, interaction grows coverage | |

**User's choice:** Scripted-interaction traces

| Option | Description | Selected |
|--------|-------------|----------|
| CI linux leg (Xvfb) | Phase 6 D-16 everything-in-CI default; Xvfb + xdotool in hosted CI | |
| darwin documented-manual only | cocoa devdraw on the dev machine; CI carries compile gates but never the trace (Phase 8 D-16 soak-override precedent) | ✓ |

**User's choice:** darwin documented-manual only
**Notes:** Deliberate D-16 deviation for this leg; the exact command must be documented in the phase evidence.

| Option | Description | Selected |
|--------|-------------|----------|
| plumber gets a functional probe | Starts, scripted plumb message routes to a registered port, clean shutdown — pre-validates DESK-05 | ✓ |
| plumber compile-only too | plumber's first run is Phase 12's drawterm session | |

**User's choice:** plumber gets a functional probe
**Notes:** plumber is the one tier-3 GUI binary that runs headless (message bus, no draw).

---

## Bootfs payload specifics

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal desktop set (~3MB) | lucm + pelm + lucidasans — covers rio/acme/samterm/glenda-profile fonts; CJK 22MB, symbola 6MB, lucida 5MB excluded by name | |
| Latin+common set (~9MB) | Minimal + times/palatino/courier/misc/special/smiley/fixed | |
| Full 36MB | All of lib/font/bit staged; zero exclusion decisions; initrd grows ~8× (bootfs.paq currently 4.0MB) | ✓ |

**User's choice:** Full 36MB
**Notes:** RAM-resident paqfs initrd cost accepted by the user (r9 boots -m ≤ 1G). Verified font defaults: rio → lucm/unicode.9.font (rio.c:165), acme → lucidasans/euro.8.font + lucm fallback (acme.c:40-41), samterm → $font, glenda profile → pelm/euro.9.font + pelm/latin1.8.font.

| Option | Description | Selected |
|--------|-------------|----------|
| Full cpu-aware profile, trimmed | glenda reference minus upas/fs+news; terminal case WITHOUT rio exec (serial console, Pitfall 14/DESK-02); cpu case intact — Phase 12 consumes unchanged | ✓ |
| Terminal-only minimal now | Phase 12 authors the cpu-case stanza later; weakens 'never block on payload' | |

**User's choice:** Full cpu-aware profile, trimmed
**Notes:** In-tree reference usr/glenda/lib/profile verified during scouting.

| Option | Description | Selected |
|--------|-------------|----------|
| Template + bake-time substitution | Committed template with placeholder tokens, substituted from build/aarch64/netaddr.rc at bake (Phase 8 D-11 seam stays single source); manifest asserts result | ✓ |
| Static committed slirp values | Simplest but drifts from the seam — Pitfall 9's named failure mode | |
| Full generation, no template | ndb output invisible to review between regenerations | |

**User's choice:** Template + bake-time substitution

---

## the agent's Discretion

- Per-tier membership order and wave decomposition within tiers 2/3/4
- Functional-probe script internals (fixed inputs, marker vocabulary, donor port numbers)
- gs `default.aarch64.h` route: hand-authored vs genarch-on-donor harvested
- GUI trace rig mechanics (guest mount of host devdraw, darwin input injection mechanism, trace-file format)
- gs render-smoke fixture + bitmap device choice
- `tiers/` + `markers/` physical layout extensions; mk filter consumption of new tier files
- ndb/local template home + placeholder token syntax
- Bootfs staging mechanics (committed staged tree vs bake-time copy — Phase 5 adm/timezone precedent)
- CI job topology (fast compile legs vs slow donor boot legs)

## Deferred Ideas

- r9 execution of tier-2 services — Phase 8 completes /net; Phases 11/12 consume binaries on r9
- Real p9sk1 domain (bake-time generator, /adm/keys, keyfs-as-service, ndb domain, secstore) — Phase 11
- `#!` exec, service=cpu, aux/listen at boot on r9, drawterm, rio in-session, devssl — Phase 12
- fossil/venti runtime — future milestone (needs block device)
- upas send/receive UAT; lp gs-dependent stages — not this milestone
- Linux-CI Xvfb GUI trace leg — rejected for now (D-11); revisit if manual discipline rots
- Rust fake-draw server — rejected (D-09); plan9port devdraw is proven
- virtio-rng/blk/9p, SMP/PSCI, RTC — future milestones (carried)
- darwin host sbrk 4096KB cap — platform fact (06-01 deferred)
