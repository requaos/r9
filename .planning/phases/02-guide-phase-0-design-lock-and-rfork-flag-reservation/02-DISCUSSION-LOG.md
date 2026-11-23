# Phase 2: Guide Phase 0 - Design Lock and rfork Flag Reservation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-23
**Phase:** 2-Guide Phase 0 - Design Lock and rfork Flag Reservation
**Areas discussed:** Locking the 13 decisions, C prototypes content, Threat model document, Reservation and test shape

---

## Locking the 13 decisions

### How to confirm IG-D01 to IG-D13

| Option | Description | Selected |
|--------|-------------|----------|
| Bulk, exceptions only (Recommended) | One table with a proposed status each; the matching ones locked together; individual rulings only on D03, D07, D11 | ✓ |
| One by one | Each of the thirteen confirmed separately at an owner checkpoint | |
| Decide all here, now | Settle every status in the discussion; no later checkpoint | |

**User's choice:** Bulk, exceptions only

### IG-D07 (port/ forbid wording versus HEAD)

| Option | Description | Selected |
|--------|-------------|----------|
| Lock, amended to HEAD (Recommended) | Per-module forbids, three named floor modules, no new floor module without approval; N2 stays for Phases 3 and 5 | ✓ |
| Lock as the guide wrote it | Crate-level forbid as target; floor modules become recorded debt | |
| Leave unlocked | Settle with N2 in Phase 3 | |

**User's choice:** Lock, amended to HEAD

### Decisions with open details (IG-D09, IG-D11, IG-D13)

| Option | Description | Selected |
|--------|-------------|----------|
| Lock direction, list open parts (Recommended) | Lock the choice, record which W items remain open | |
| Defer the lock | Leave these three unlocked until their W items are settled in Phases 3, 4 and 6 | ✓ |
| Settle the W items now | Pull W1-W7 into this phase | |

**User's choice:** Defer the lock
**Notes:** Departs from the recommended option. IG-D03 is locked with the wording corrected to four flag bits (owner decision 2026-11-22), stated when the area closed.

### Meaning of "locked"

| Option | Description | Selected |
|--------|-------------|----------|
| Owner-only change (Recommended) | Fixed for planning and execution; a conflict stops and asks; changes are dated superseding rows | ✓ |
| Fixed for the roadmap | Cannot change before Phase 16 | |
| Strong default | A later phase may deviate with a recorded reason | |

**User's choice:** Owner-only change

---

## C prototypes content

### What they contain

| Option | Description | Selected |
|--------|-------------|----------|
| The four flag #defines only (Recommended) | Header fragment with the four new flags only | |
| Flags plus the rfork family | Four new flags, the full existing RF* list, and the rfork() declaration | ✓ |
| Every C interface the crypto work touches | Flags, rfork, segattach/segdetach/sbrk with researched corrections | |
| Let me describe it | Owner explains the guide author's meaning | |

**User's choice:** Flags plus the rfork family

### Location

| Option | Description | Selected |
|--------|-------------|----------|
| New sys/include/ (Recommended) | Matches the guide's landing sites; never has to move | ✓ |
| docs/ | Reference document beside devip-contract.md | |
| Beside the Rust constants | Under an arch crate or shared spot | |

**User's choice:** New sys/include/

### File form

| Option | Description | Selected |
|--------|-------------|----------|
| Small r9-owned rfork.h (Recommended) | RF* enum and rfork() declaration only, with Harvey line references | ✓ |
| libc.h holding only this part | Named as the guide does, partial for now | |
| Full copy of Harvey's libc.h, amended | About 700 lines of Harvey C copied in | |

**User's choice:** Small r9-owned rfork.h

### Keeping header and Rust in agreement

| Option | Description | Selected |
|--------|-------------|----------|
| Host test reads the header (Recommended) | Parse RF* values out of rfork.h and compare, like ureg-check | |
| Generate the header | xtask writes rfork.h from the Rust constants; a check fails when stale | ✓ |
| Comment cross-reference only | Review only | |

**User's choice:** Generate the header
**Notes:** Departs from the recommended option.

---

## Threat model document

### Location

| Option | Description | Selected |
|--------|-------------|----------|
| docs/threat-model.md (Recommended) | Product document beside the other docs | ✓ |
| Top-level SECURITY.md | Repo root, surfaced by GitHub | |
| .planning/ | Beside the source documents | |

**User's choice:** docs/threat-model.md

### Depth

| Option | Description | Selected |
|--------|-------------|----------|
| Boundaries plus a phase map (Recommended) | In/out lists with mechanism and delivering phase per protection | |
| Boundaries only | AC section 2.2 restated | |
| Full analysis | Assets, attacker classes, trust boundaries, protection table | ✓ |

**User's choice:** Full analysis
**Notes:** Departs from the recommended option.

### New security claims

| Option | Description | Selected |
|--------|-------------|----------|
| Trace or mark, then owner review (Recommended) | Each statement cites a source or is marked new; boundaries exact; owner reviews the draft | ✓ |
| Source-backed only | Only what the four documents say; silence listed as open | |
| Free hand | Written from the architecture, reviewed whole | |

**User's choice:** Trace or mark, then owner review

### Standing of the document

| Option | Description | Selected |
|--------|-------------|----------|
| Locked boundaries, living table (Recommended) | Boundary locked; status per protection updated by later phases | |
| Locked whole, written as target | Entire document locked, not edited later | |
| Living document, nothing locked | Later phases may revise freely, including the boundary | ✓ |

**User's choice:** Living document, nothing locked
**Notes:** Departs from the recommended option.

---

## Reservation and test shape

### Source of truth for the flags

| Option | Description | Selected |
|--------|-------------|----------|
| A flag table in xtask (Recommended) | xtask table generates the header and checks both arch files; no port/src edit | |
| One definition in port/ | Constants move into the port crate; both arch crates import; needs diff approval and departs from the roadmap's two named files | ✓ |
| x86_64 is the reference | Header generated from x86_64; aarch64 checked against it | |

**User's choice:** One definition in port/
**Notes:** Departs from the recommended option. Recorded as an owner amendment to ROADMAP criterion 2 and DSGN-03.

### Which flags move

| Option | Description | Selected |
|--------|-------------|----------|
| All rfork flags (Recommended) | Existing and new in one module; duplicates deleted from both arch files | ✓ |
| Only the four new flags | Existing eleven stay duplicated | |
| All in port/, arch files re-export | Definitions move; arch names kept as re-exports | |

**User's choice:** All rfork flags

### What "behaviour unchanged" pins

| Option | Description | Selected |
|--------|-------------|----------|
| Pin the rejection (Recommended) | Host test asserts no reserved bit is in either SUPPORTED mask | ✓ |
| Pin values and overlap only | As DSGN-04 states; behaviour argued from the diff | |
| Also a live boot check | Guest program calls rfork with each reserved bit | |

**User's choice:** Pin the rejection

### RFNOMNT (bit 14)

| Option | Description | Selected |
|--------|-------------|----------|
| Define it, unsupported (Recommended) | Named constant outside SUPPORTED | ✓ (second ask) |
| Leave it out | Bare "taken in Harvey" value in the test only | |
| Define and implement it | Make RFNOMNT work now | first answer |

**User's choice:** First answered "Define and implement it". That is a kernel behaviour change, outside this phase's fixed goal, so it was redirected to Deferred Ideas and the question was asked again with the two in-scope options. Second answer: Define it, unsupported.
**Notes:** Kernel byte proof was confirmed to follow the standing rules (sha256 comparison of both kernels; the x86_64 procedure if bytes change); the port/src diff is shown before commit.

---

## Claude's Discretion

- Name and file of the port module holding the flags
- How the generator reads the Rust constants; name and home of the xtask step and staleness check
- Where the host tests sit (port for values and overlap; arch crates for the SUPPORTED masks)
- Layout of docs/threat-model.md and the form of the "new in this document" mark
- Layout of the thirteen-decision table and order of work in the phase

## Deferred Ideas

- Implement RFNOMNT (bit 14) in r9's kernel — a behaviour change; needs its own phase
- Locking IG-D09, IG-D11, IG-D13 — deferred to Phases 3, 4 and 6
