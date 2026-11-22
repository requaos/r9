# Phase 19: CPU Service + Drawterm + rio Desktop - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-14
**Phase:** 19-CPU Service + Drawterm + rio Desktop
**Areas discussed:** Drawterm provenance, Darwin leg reachability, Kernel-flush posture, Gate creds + acceptance proof

---

## Gray-area selection

The four areas were offered as a multi-select; the first selection pass returned empty (harness dismissal), the retry per answer-validation rules succeeded with **all four areas selected**.

---

## Drawterm provenance

### Q1: Which drawterm source does the flake app build from?

| Option | Description | Selected |
|--------|-------------|----------|
| In-tree vendor | `sys/src/cmd/unix/drawterm` — classic 4e lineage + Harvey-era fixes; p9any/p9sk1, `-e` encr flag, gui-x11 + gui-osx; we own it, `-G` is a small local patch | ✓ |
| Refresh from 0intro upstream | Re-vendor from github.com/0intro/drawterm at a fresh pin, then apply local additions | |
| Switch to 9front drawterm | Replace the vendor with 9front's actively-developed diverged lineage | |

### Q2: How does drawterm get built and pinned?

| Option | Description | Selected |
|--------|-------------|----------|
| Nix builds in-tree vendor | Nix package recipe compiling the vendored source (9cc/9front-toolchain vendor precedent) — hermetic, no fetch | ✓ |
| Nix flake input fetch | Flake input fetching drawterm from GitHub at pinned rev+hash (minimega precedent), patches via patchPhase | |

### Q3: Which darwin graphics backend?

| Option | Description | Selected |
|--------|-------------|----------|
| Native Cocoa backend | gui-osx built natively for aarch64-darwin — no XQuartz dependency | ✓ |
| X11/XQuartz backend | gui-x11 via Make.osx-x11 — simpler recipe, XQuartz dependency | |
| Cocoa primary + X11 fallback | Both buildable, only Cocoa gated | |

### Q4: What shape does the added `-G` text-mode take?

| Option | Description | Selected |
|--------|-------------|----------|
| No-window stdio mode | Connects, authenticates, mounts /mnt/term, session stdio on the invoking terminal | ✓ |
| Headless/offscreen mode | No real terminal interaction; markers through a piped channel | |

**User's choice:** All four recommended options.
**Notes:** Scout discovered the vendor already exists in-tree at `sys/src/cmd/unix/drawterm` (history to the 2000 3e import; Harvey fixes like 5c87831c6; upstream maintained by 0intro) — this grounded the recommendation. No `-G` flag exists in the vendor today; it is a local addition regardless of lineage.

---

## Darwin leg reachability

### Q1: How does the darwin drawterm reach the linux-hosted guest's 17010/567?

| Option | Description | Selected |
|--------|-------------|----------|
| SSH port-forward | `ssh -L 17010:127.0.0.1:17010 -L 5670:127.0.0.1:5670`; hostfwd stays loopback-bound; zero new surface | ✓ |
| Hostfwd rebound to LAN | Rebind slirp hostfwd to LAN/0.0.0.0 (TOML change) — breaks the loopback-only stance | |
| Tap bridge on LAN | Real-LAN variant (16-D-08 documented-not-tested) — biggest surface, host config + doctor work | |

### Q2: Does `nix run .#drawterm` work natively on darwin?

| Option | Description | Selected |
|--------|-------------|----------|
| Native darwin flake app | aarch64-darwin package + app in the flake (standalone, no r9 input) | |
| Documented-manual build | Flake app stays linux-only; darwin builds via README recipe (venue convention) | ✓ |

### Q3: How is the guest kept running during a human drawterm session?

| Option | Description | Selected |
|--------|-------------|----------|
| Interactive boot left running | `nix run .#boot-r9-x64` in a terminal; README session recipe; zero new machinery | ✓ |
| Headless session app | Dedicated -display none app with lifecycle management | |

**User's choice:** SSH port-forward, documented-manual darwin build, interactive boot.
**Notes:** The darwin flake-app option was recommended but declined — scope-keep on darwin venue per the documented-manual convention; the Cocoa backend decision from Area 1 still governs the manual build recipe.

---

## Kernel-flush posture

### Q1: CR-03 (AcceptPool Nagle + delayed-ACK on announced sockets)?

| Option | Description | Selected |
|--------|-------------|----------|
| Fix as planned item | Two set_* calls at the pool site + WR-04 pin legs — a dependency of exportfs over tcp17010 | ✓ |
| Fix only when fired | Wire up, run gates, diagnose if the stall fires | |

### Q2: If the waived #66 class fires under drawterm/exportfs load?

| Option | Description | Selected |
|--------|-------------|----------|
| Honor reopener if context | Root-cause in-phase ONLY if the waiver's capture context is reached; otherwise record-enrich + continue | ✓ |
| Always root-cause | Any firing opens a root-cause work item regardless of capture feasibility | |
| Record + re-waive | Any firing is recorded and re-waived; phase never stops | |

### Q3: x86_64 tick_check gap (retransmit/keepalive/connect timers under park)?

| Option | Description | Selected |
|--------|-------------|----------|
| Reopener stands, fix-if-needed | Keep the standing reopener; fix only if a 19 gate/session stalls on it | |
| Fix proactively | Wire tick_check into the x86_64 tick path this phase, close the latent defect before sessions hang | ✓ |

### Q4: Umbrella posture for unforeseen kernel gaps?

| Option | Description | Selected |
|--------|-------------|----------|
| Fix everything surfaced | Root cause, fix push-before-pin, pin in-phase (14-17 doctrine) | ✓ |
| Blockers only + disposition | Fix what blocks success criteria; disposition the rest | |

**User's choice:** Planned CR-03 fix; reopener-honored-if-context for #66; **proactive tick_check (override of the recommended reopener-stands)**; fix-everything-surfaced.
**Notes:** The tick_check choice is a deliberate user override — flagged in CONTEXT.md D-10 so downstream agents don't second-guess it.

---

## Gate creds + acceptance proof

### Q1: How does the CI -G gate get login credentials for the DEMO domain?

| Option | Description | Selected |
|--------|-------------|----------|
| Generator emits creds file | Gitignored demo user/password in the bake output; gate reads post-bake; 18-D-01 real-randomness preserved | ✓ |
| Fixed committed demo password | Simplest plumbing; re-litigates 18-D-01; literal credential string in git | |

### Q2: What evidence does DESK-07's human graphical acceptance produce?

| Option | Description | Selected |
|--------|-------------|----------|
| Screenshot + signed row | Per leg: rio desktop screenshot + signed row in 19-EVIDENCE.md (date, venue, r9 SHA, accepter) | ✓ |
| Recording + signed row | Screen recording per leg + signed row — richer, heavier | |
| Markers + sign-off only | Serial markers + human row, no visual capture | |

### Q3: Where is the DESK-06 -G gate's required green witness?

| Option | Description | Selected |
|--------|-------------|----------|
| Hosted CI required | TCG/slirp required leg (net-smoke-x64 pattern); local KVM never required | |
| Local primary, CI best-effort | Local KVM is the required witness; hosted CI best-effort | ✓ |

### Q4: What does the wrapper's connect line default to, and what does the -G gate prove?

| Option | Description | Selected |
|--------|-------------|----------|
| Encrypted default | devssl rc4_256/sha1 via pushssl on every ordinary session + the gate; `-e clear` documented fallback | ✓ |
| Clear default, both gate legs | Wrapper prints `-e clear`; gate runs clear + encrypted legs | |

**User's choice:** Generator creds file; screenshot + signed row; **local-KVM-primary witness (override of the recommended hosted-CI-required)**; encrypted default.
**Notes:** The local-primary choice is a deliberate user override — flagged in CONTEXT.md D-14. INT-06's fresh hosted CI run after the pin advance still happens per the standing discipline.

---

## Claude's Discretion

(As recorded in CONTEXT.md — no "you decide" answers were given during the session; the discretion list enumerates planner/researcher freedom within the locked decisions.)

## Deferred Ideas

- The #66/CR-07 cure (stays waived; D-09 firing policy)
- A headless cpu-session app (declined, D-07)
- A native aarch64-darwin drawterm flake app (declined, D-06)
- LAN-exposed guest networking (declined, D-05)
- rio at boot on the terminal path (out of scope by requirement)
- Stronger drawterm crypto / TLS-class (PROTOCOL-blocked per DESK-04)
