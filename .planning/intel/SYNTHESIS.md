# Ingest Synthesis Summary

Mode: new. Entry point for gsd-roadmapper.

## Documents by type
- SPEC: 4 (ADR 0, PRD 0, DOC 0). UNKNOWN: 0. Cross-reference cycles: none (all `cross_refs` empty).
- Precedence (owner-approved manifest, lower wins):
  - 0: /Users/neilskinner/code/r9/.planning/r9-implementation-guide-amended-v3.md (IG)
  - 1: /Users/neilskinner/code/r9/.planning/r9-architecture-and-code-v4.md (AC)
  - 2: /Users/neilskinner/code/r9/.planning/rfc-9p2000-aead.md (RFC)
  - 3: /Users/neilskinner/code/r9/.planning/r9-permissions-and-test-plan.md (PT)
- Source docs were read in full (about 3,300 lines). Classifier summaries were shallow, so extraction is from the sources.

## Decisions locked
- 0. No ADRs and nothing classified locked. See `decisions.md`. IG section 2's "non-negotiable" decisions are carried as constraints IG-D01 to IG-D13 for the owner to lock or not.

## Requirements extracted
- 0. No PRDs. See `requirements.md` for where acceptance-style content lives (phase exit gates in `context.md`, test assertions in `constraints.md` PT-T01 to PT-T06).

## Constraints
- 61 entries in `constraints.md`, by type:
  - nfr: 26 (IG-D01, D02, D04 to D08; IG-A01; IG-M02 to M04; IG-S03, S04; IG-T01; IG-V01, V02; AC-T01; AC-R02; RFC-P10; PT-C01; PT-T01 to T06)
  - protocol: 19 (IG-D11, D13; IG-P01 to P04; AC-P01 to P05; RFC-P01 to P04, P06 to P09)
  - schema: 12 (IG-D09, D10; IG-M01, M05 to M08; IG-S01, S02; AC-R01; RFC-P05; PT-C02)
  - api-contract: 4 (IG-D03, D12; IG-L01; AC-G01)
- Groups: IG (precedence 0), AC (1), RFC (2), PT (3).

## Context topics
- 8 topics in `context.md`: pinned commits versus HEAD, repo state at the pin, document lineage, project character, roadmap as specified (12 phases and milestones with dependencies and exit gates), superseded baselines, cited work, out-of-scope boundaries.

## Conflicts
- 0 blockers, 11 warnings (competing or unresolved items needing an owner decision), 13 info (auto-resolved by precedence or recorded for transparency). Detail: /Users/neilskinner/code/r9/.planning/INGEST-CONFLICTS.md
- Warnings in one line each: W1 wire header layout and AAD contents; W2 FidHandle 8 bytes versus StatelessCap 24 bytes; W3 MAC primitive (Keyed BLAKE3 versus BLAKE3 / Poly1305); W4 session key scope (per FID versus per connection and direction); W5 `unsafe` in `ds/src/tpht/` versus `forbid(unsafe_code)` on `ds/`; W6 TPHT block 4 versus 16 slots; W7 DiscoveryBeaconV2 192 versus 202 bytes; W8 `/adm/users` UID 0 and glenda; W9 alice present in `/adm/users` but test needs her absent; W10 Suite 3 mounts cross VLANs with no route; W11 `RFNOLEGACY` flag bit unassigned.
- W1 to W7 and W11 affect Phase 0 to Phase 4 design and should be settled before those phases are planned. W8 to W10 only affect the PT test fixtures.

## Per-type intel files
- /Users/neilskinner/code/r9/.planning/intel/decisions.md
- /Users/neilskinner/code/r9/.planning/intel/requirements.md
- /Users/neilskinner/code/r9/.planning/intel/constraints.md
- /Users/neilskinner/code/r9/.planning/intel/context.md
- Classifier inputs: /Users/neilskinner/code/r9/.planning/intel/classifications/

## Notes for the roadmapper
- Roadmap order and dependencies come from IG section 8 (identical in AC except Phase 0). Reproduced in `context.md`.
- HEAD has none of the new crates or directories the docs name as landing sites (`crypto/`, `ds/`, `kern/`, `sys/`, `venti/`); re-verify paths against HEAD.
- The AC appendix listings are non-normative where they conflict with the IG ledger (see I5).
- `/Users/neilskinner/code/r9/.planning/legacy/` and `HANDOFF.md` were not ingested and nothing in them was treated as existing context.
