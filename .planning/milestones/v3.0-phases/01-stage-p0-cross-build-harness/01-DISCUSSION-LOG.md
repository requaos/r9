# Phase 1: Stage P0 - Cross-Build Harness - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-22
**Phase:** 1-Stage P0 - Cross-Build Harness
**Areas discussed:** Pre-built tree supply, Flake flip shape, Boot-to-rio gate, Fallback manifest and swap

---

## Pre-built tree supply

| Question | Options | Selected |
|----------|---------|----------|
| How are the built Harvey artifacts produced? | Nix derivation / Scripts into a cache dir / Owner-built checkout | Nix derivation |
| How does the Mac get the built tree? | Linux builder / Keep the lima script path / Fetch from a binary cache | Linux builder |
| Both guest architectures always, or on demand? | Per-arch outputs / One tree, both archs / You decide | Per-arch outputs |
| How does the Harvey pin move? | Deliberate bumps / Track Harvey's branch / Freeze at legacy-final | Deliberate bumps |

**User's choice:** the recommended option in each case.
**Notes:** none.

---

## Flake flip shape

| Question | Options | Selected |
|----------|---------|----------|
| Where does the gate machinery live? | Move into r9 / Read from the pinned tree / Fold into xtask | Move into r9 |
| What does Harvey's flake still provide? | Build-only / Minimal stub / You decide | Free text (below) |
| How does r9 take Harvey as an input? | Plain source / Flake input | Flake input |
| In what order does the flip land? | r9 first, then Harvey / Single coordinated cut | r9 first, then Harvey |

**User's choice (Harvey flake, free text):** "harvey should still be able to build itself and developer should be able to pull harvey and run one nix command to build and launch harvey in q35 qemu and another to build and launch drawterm to connect to harvey, all without r9."
**Notes:** "Flake input" was chosen over the recommended plain source input. Moving the gate scripts widens the Harvey diff beyond `flake.nix`.

---

## Boot-to-rio gate

| Question | Options | Selected |
|----------|---------|----------|
| What counts as reached rio? | Drawterm gate plus rio marker / Serial-only rio marker / Screenshot comparison | Drawterm gate plus rio marker |
| How is "every commit" enforced? | Run before each commit / Pre-push hook / CI only | Run before each commit |
| Which venue is the required witness? | Local Linux KVM / Mac under TCG / Hosted CI | Local Linux KVM |
| A faster tier for everyday commits? | Two tiers / One gate always / You decide | Two tiers |

**User's choice:** the recommended option in each case.
**Notes:** none.

---

## Fallback manifest and swap

| Question | Options | Selected |
|----------|---------|----------|
| Manifest granularity? | Per file in the image / Per component / Per image | Per file in the image |
| Where is a Rust replacement swapped in? | r9 assembles the image / Repack Harvey's image / Parameterise Harvey's bake | r9 assembles the image |
| What proves the swap? | Trivial Rust a.out stand-in / Marker file swap / Rebuilt C stand-in | Trivial Rust a.out stand-in |
| Does the probe stay? | Keep as a gate test / Remove after proving / You decide | Remove after proving |

**User's choice:** recommended options, except the probe is removed after proving (the recommendation was to keep it as a gate test).
**Notes:** none.

---

## Claude's Discretion

- The rio "serving" marker and how it is observed without a display.
- Manifest file format and location in `xtask/`.
- Routing of existing `R9_HARVEY_DIR` consumers through one resolver.
- Which Linux builder mechanism to document for the Mac.

## Deferred Ideas

- Gates re-expressed as `cargo xtask` commands in Rust (not chosen for this phase).
- A binary cache for the built Harvey tree on the Mac (not chosen).
