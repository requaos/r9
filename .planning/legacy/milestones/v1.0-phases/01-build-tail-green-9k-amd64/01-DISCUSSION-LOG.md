# Phase 1: Build-Tail Green (9k amd64) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-30
**Phase:** 1-build-tail-green-9k-amd64
**Areas discussed:** Fix depth + loop resilience; aarch64 quarantine mechanics

---

## Fix depth + loop resilience

### Q1: Fix-depth target for the cmd build chain

| Option | Description | Selected |
|--------|-------------|----------|
| Fix blockers, prune non-core | Fix chain-blockers (jpg/mkfile, 4a); prune mahjonng/ghostscript from default DIRS per CONCERNS.md | |
| Fix everything | Every cmd in sys/src/cmd must build — incl. CVE-era ghostscript, mahjonng | ✓ |
| Minimal + resilient loop | Smallest fix for kernels; continue-on-error loop | |

**User's choice:** Fix everything
**Notes:** Larger commitment acknowledged — vendored 2002-era code in scope.

### Q2: Policy for failures inside vendored snapshots (gs, antiword, apeg)

| Option | Description | Selected |
|--------|-------------|----------|
| Patch vendored in place | This fork owns them; no upstream compat to preserve | ✓ |
| Hands off vendored code | Fix around them (flags, excludes, shims) | |
| Fix our tools, workaround vendor | Fix our cpp bugs properly, tolerate vendor hacks | |

**User's choice:** Patch vendored in place
**Notes:** With a toolchain-first diagnostic rule added later in the session: vendored code built fine for decades — a fresh failure likely means OUR cpp/6c/pcc drifted (cf. mahjongg = our cpp's bug). Check the toolchain first; patch vendored source only when the vendored code is genuinely at fault.

### Q3: mk DIRS loop semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Strict default + report-all target | Default loops abort; new checkall-style target reports all failures | ✓ |
| Continue-on-error default | Loops continue, exit non-zero at end | |
| Upstream semantics as-is | Abort at first failure, unchanged | |

**User's choice:** Strict default + report-all target

### Q4: Fix sequencing

| Option | Description | Selected |
|--------|-------------|----------|
| Build-order whack-a-mole | Run, fix current blocker, rerun | ✓ |
| Inventory first, then fix | Full failure list first, dependency-order fixes | |
| Kernels first, then sweep | jpg→ip/venti chain first, then rest | |

**User's choice:** Build-order whack-a-mole

---

## aarch64 quarantine mechanics

### Q1: Stale arm64 vestiges

| Option | Description | Selected |
|--------|-------------|----------|
| Rename + remove | build_arm64→build_aarch64 (corrected refs), drop mkdirs entry, remove empty arm64/ | ✓ |
| Delete outright | Remove dead funcs entirely; Phase 2 writes fresh wiring | |
| Comment-quarantine | Leave with quarantine comment | |

**User's choice:** Rename + remove

### Q2: 9front donor `arm64` naming on import (Phase 2 rule)

| Option | Description | Selected |
|--------|-------------|----------|
| Rename on import | Normalize to aarch64 via repeatable step; donor contents untouched | ✓ |
| Keep donor names | Imported paths keep arm64; forces objtype=arm64 | |
| Mixed | Internals donor-named, docs/tooling aarch64 | |

**User's choice:** Rename on import

---

## the agent's Discretion

- Reproducibility policy (BUILD-04): pinned timestamps (SOURCE_DATE_EPOCH-style), kernels scope
- minimega smoke harness (BUILD-05): kernel+initrd boot of 9k10cpu, serial-log assertion, in-repo `.mm` scripts, no in-guest agent

## Deferred Ideas

- Attic/quarantine for skeletal ports beyond arm64 vestiges (riscv64 vaporware, arm/mips/power fragments) — later hygiene phase
- TLS 1.3 / cipher pruning — hardening milestone
- 386 kernel + ISO/dist stage — deferred per project scope
- minimega cluster/CI deployment — local harness first
