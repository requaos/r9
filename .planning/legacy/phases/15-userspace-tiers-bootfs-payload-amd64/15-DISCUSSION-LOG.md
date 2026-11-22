# Phase 15: Userspace Tiers + Bootfs Payload (amd64) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-09
**Phase:** 15-Userspace Tiers + Bootfs Payload (amd64)
**Areas discussed:** r9 services leg (USPACE-07), GUI trace venue (USPACE-08), Long-tail sweep scope (USPACE-09), Bootfs payload shape (DESK-03), ancestor reconciliation (v2.0 Phase 10)

---

## r9 services leg (USPACE-07)

### Q1 — Wedge policy

| Option | Description | Selected |
|--------|-------------|----------|
| Hybrid | Local r9 defects fixed in-phase (14-07 class); structural gaps take venue-honest SKIP→17 | |
| All-SKIP to 17 | Strictly Harvey-side; every wedge a named SKIP + ledger entry | |
| Fix everything | Every wedge root-caused and fixed in-phase — maximal flush-out | ✓ |

**User's choice:** Fix everything
**Notes:** Deliberate override past the recommended hybrid — consistent with the flush-out doctrine.

### Q2 — Auth boundary ("fix everything" meets Phase-18-owned substrate)

| Option | Description | Selected |
|--------|-------------|----------|
| 18-owned stays 18 | devcap #¤, bake-time /adm/keys, keyfs stay Phase 18; auth entries get exec-level markers or named SKIP→18 | ✓ |
| Pull devcap forward | Implement #¤ this phase so auth_chuid is exercisable | |
| Pull whole substrate | devcap + keys + keyfs — merges half of Phase 18 into 15 | |

**User's choice:** 18-owned stays 18

### Q3 — Daemon execution marker

| Option | Description | Selected |
|--------|-------------|----------|
| Functional probe | Marker = real client interaction succeeds through the daemon; impossible = named SKIP | ✓ |
| Exec-and-idle only | Startup marker + survive N seconds (the G-07-1 counter-not-delivery trap) | |
| Per-venue ladder | Donor functional, r9 idle-until-promoted | |

**User's choice:** Functional probe

### Q4 — Services image mechanics

| Option | Description | Selected |
|--------|-------------|----------|
| New services image | mkimage fifth contract + own sweep + own expect gates (14-06 one-image-per-gate) | ✓ |
| Extend net image | Append payload onto bootfs-net.paq | |
| Extend tier sweep | Daemon class inside the tier image/sweep | |

**User's choice:** New services image

---

## GUI trace venue (USPACE-08)

### Q5 — Trace venue

| Option | Description | Selected |
|--------|-------------|----------|
| Draw donor venue | New 9k config variant (k10cpu + vga/draw/mouse) under qemu -vga std; real GUI startup traces | ✓ |
| Startup-to-wall | Existing k10t1, trace to first /dev/draw open failure | |
| Draw venue + wall delta | Both, delta measures the display subtree's contribution | |

**User's choice (initial):** Draw donor venue

### Q6 — Trace depth (initial)

| Option | Description | Selected |
|--------|-------------|----------|
| Startup-to-idle | Exec → idle event loop; deterministic | ✓ |
| Scripted session | Drive real interaction per program | |
| Idle + one script | Idle gate + one best-effort scripted interaction | |

**User's choice (initial):** Startup-to-idle — **superseded by the ancestor reconciliation (Q15): scripted ADOPTED**

### Q7 — Trace deliverable shape

| Option | Description | Selected |
|--------|-------------|----------|
| Raw + table | Raw logs + distilled syscall-set table (05-03 format) | ✓ |
| Table only | Distillation unverifiable | |
| Raw only | Analysis cost pushed to the consumer phase | |

**User's choice:** Raw + table

### Q8 — GUI gating

| Option | Description | Selected |
|--------|-------------|----------|
| Trace is the gate | Committed trace+table is the runtime artifact; no separate exec markers | ✓ |
| Add exec markers | GUI-OK markers join the vocabulary on the draw venue | |

**User's choice:** Trace is the gate

---

## Long-tail sweep scope (USPACE-09)

### Q9 — Coverage bar

| Option | Description | Selected |
|--------|-------------|----------|
| Full DIRS proof | Every cmd dir compiles, machine-verified coverage assertion | ✓ |
| Named targets only | upas/lp/gs/fossil/venti + what other legs pull in | |

**User's choice:** Full DIRS proof

### Q10 — Breakage disposition

| Option | Description | Selected |
|--------|-------------|----------|
| Fix in-tree | No skip-vocabulary for compile failures; coverage ends only green | ✓ |
| Fix + named exceptions | Documented exception class asserted by the coverage proof | |

**User's choice:** Fix in-tree

### Q11 — Tier data organization

| Option | Description | Selected |
|--------|-------------|----------|
| Per-class files | New tierN-*.txt beside untouched tier0/tier1; each gate reads its own file | ✓ |
| One growing file | Everything appends into tier1-cmds.txt (~260 mixed entries) | |

**User's choice:** Per-class files

---

## Bootfs payload shape (DESK-03)

### Q12 — Payload home

| Option | Description | Selected |
|--------|-------------|----------|
| Root-tree skeleton | Shared base every contract stages from; 18/19 inherit by construction | ✓ |
| Dedicated desk image | bootfs-desk.paq with its own manifest gate | |
| Services image only | ndb for 18; fonts deferred to 19 | |

**User's choice:** Root-tree skeleton

### Q13 — Font scope

| Option | Description | Selected |
|--------|-------------|----------|
| Whole bit tree | All of lib/font/bit (~36MB); font choice stays Phase 19's concern | ✓ |
| Named subset | lucm/latin1/fixed mixes only — guessing at 19's needs | |

**User's choice:** Whole bit tree

### Q14 — Manifest assertions

| Option | Description | Selected |
|--------|-------------|----------|
| Data file + bake assert | Committed manifest consumed by mkimage; bake fails on missing/empty | ✓ |
| Inline mkimage checks | Hardcoded test -f per path | |

**User's choice:** Data file + bake assert

---

## Ancestor reconciliation (v2.0 Phase 10 context, discovered post-discussion)

The v2.0 Phase-10 CONTEXT (aarch64-keyed, never executed) was found after the four areas closed. It carries researched decisions for the same scope; two conflicted with choices made above (the options had been presented without the ancestor's evidence). The user re-decided with both premise sets visible.

### Q15 — Trace venue vs ancestor D-08 (draw-over-9P topology argument)

| Option | Description | Selected |
|--------|-------------|----------|
| Keep kernel-local | Today's choice; full client-side syscall mix | |
| Flip to draw-over-9P | v2.0 shape; needs a drawterm-derived host server + a venue channel k10t1 doesn't have | |
| Kernel-local now, 9P later | Kernel-local this phase; 9P topology rig deferred to the phase that grows r9's desktop path | ✓ |

**User's choice:** Kernel-local now, 9P later

### Q16 — Trace depth vs ancestor D-10 (scripted-interaction argument)

| Option | Description | Selected |
|--------|-------------|----------|
| Keep startup-to-idle | Today's choice | |
| Adopt scripted (v2.0) | Replay-script-driven interaction; the sustained event loop IS the measurement | ✓ |
| Idle now, script later | Idle gate this phase; scripted capture in the consumer phase | |

**User's choice:** Adopt scripted (v2.0)

### Q17 — gs render smoke vs ancestor D-02

| Option | Description | Selected |
|--------|-------------|----------|
| Adopt render smoke | .ps → bitmap file device on the donor; amd64 ape wall doesn't exist (verified in-tree) | ✓ |
| Compile-only | Roadmap-literal | |

**User's choice:** Adopt render smoke

---

## Claude's Discretion

Granted during discussion: exact tierN file names; draw-donor config variant naming/build; synthetic-input mechanism files on 9k; replay-script format; probe script internals; ndb template home/token syntax; CI leg topology; evidence-file organization; wave decomposition.

## Deferred Ideas

- draw-over-9P topology rig (drawterm-derived server + venue channel + re-capture) — the phase that grows r9's desktop path
- Phase-18 auth substrate (devcap, keys, keyfs, real handshakes) — Phase 18
- fossil/venti runtime — needs a block device
- upas send/receive UAT; lp gs-dependent stages — ancestor PITFALLS exclusions, carried unless re-derived
- Hosted-CI donor-execution legs — planner discretion
