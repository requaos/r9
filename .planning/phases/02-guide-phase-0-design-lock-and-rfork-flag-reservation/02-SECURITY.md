---
phase: "02"
slug: "guide-phase-0-design-lock-and-rfork-flag-reservation"
status: verified
threats_open: 0
asvs_level: 1
created: "2026-11-24"
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Built from the `<threat_model>` blocks of plans 02-01 to 02-06. ASVS level 1
> (grep-depth mitigation verification) and `security_block_on: high`; no threat at
> or above the blocking threshold remains open.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| `port/src/rfork.rs` → kernels and xtask | One file is compiled into both kernels and into the host tool | Flag bit values and names |
| Rust constants → `sys/include/rfork.h` | A generated artefact that C code reads in later phases | Generated C enumerators and prototype |
| Harvey checkout → r9 | Harvey's `libc.h` is read as a reference and never written | Reference text only (read-only) |
| Owner's ruling → PROJECT.md | Later phases treat a recorded lock as fixed | Decision status and wording |
| Mac → GitHub → owner's host | Code travels on a pushed branch; the owner's hash commit and results travel back | Source, branch refs, evidence logs |
| Owner's return → evidence | Verdicts are decided only from returned lines | Test/gate logs and exit codes |
| Local `main` → `origin/main` | A push publishes every unpushed commit | Repository history |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-02-01 | Tampering | A wrong value typed into the flag table | high | mitigate | `port/tests/rfork_test.rs` pins all sixteen values literally and the union `0x3c7c7f`; test passes | closed |
| T-02-02 | Tampering | Header and kernel disagree about a flag value | high | mitigate | Header rendered from the file the kernels compile; `rfork-header --check` rejects missing, empty and stale files, and is the first statement of `TestStep::run` | closed |
| T-02-03 | Elevation of privilege | Two flags sharing a bit | high | mitigate | `rfork_flags_do_not_overlap` asserts single, disjoint bits; passes | closed |
| T-02-04 | Repudiation | `port/src` change committed without owner review, or a different change committed after it | medium | mitigate | Owner stop `gate="blocking-human"`; patch committed with sha256 in `PINS.md`; Task 4 commits only on byte equality; owner reply recorded in `02-01-SUMMARY.md` | closed |
| T-02-05 | Tampering | Harvey edited by accident | medium | mitigate | Harvey read-only; acceptance compares `git -C ... status --porcelain` before and after; no Harvey write occurred | closed |
| T-02-SC | Tampering | npm/pip/cargo installs | low | accept | No package is installed in this phase; `Cargo.lock` and flake inputs unchanged (accepted risk below) | closed |
| T-02-06 | Information disclosure | A document that overstates protection | high | mitigate | Status paragraph; `not implemented` in every protection row, enforced by `threat_model_protections_are_not_implemented`; owner review before commit | closed |
| T-02-07 | Tampering | The scope boundary drifting from AC 2.2 | medium | mitigate | `threat_model_quotes_the_ac_boundary` compares against the source file; 6/6 document tests pass | closed |
| T-02-08 | Repudiation | An unsourced claim presented as sourced | medium | mitigate | Every row and bullet needs a citation tag or `**[new]**` (test); executor re-opens each cited line; owner saw the `**[new]**` list | closed |
| T-02-25 | Tampering | A document other than the reviewed draft being committed | high | mitigate | Draft crosses the stop as a committed record with sha256 in `PINS-threat-model.md`; Task 3 fails closed without it; verify gate requires `cmp` equality | closed |
| T-02-09 | Repudiation | A lock recorded that the owner did not confirm | high | mitigate | Owner stop `gate="blocking-human"`; reply stored verbatim in `02-DECISION-TABLE.md`; only confirmed statuses written | closed |
| T-02-10 | Tampering | A decision with open warnings recorded as locked | medium | mitigate | Fixed outcome forms; acceptance counts require three deferred rows; verified 8/2/3 | closed |
| T-02-11 | Tampering | A whole-file rewrite of a planning file destroying other entries | medium | mitigate | Scoped replacements only; `git diff --stat` lists only named files; unchanged-row acceptance criteria | closed |
| T-02-12 | Elevation of privilege | A reserved flag or `RFNOMNT` accepted by `rfork` after the refactor | high | mitigate | `rfork_reserved_bits_are_unsupported` in each arch crate; mask pinned to `0x3c7f`; `.text` equality in plan 02-05 | closed |
| T-02-13 | Tampering | A second definition of a flag reappearing in an arch crate | medium | mitigate | `rfork_flags_have_one_definition` reads the four arch files; passes | closed |
| T-02-14 | Tampering | Header drift unnoticed | medium | mitigate | `rforkh::check` is the first statement of `TestStep::run`; stale-header path exercised and green | closed |
| T-02-15 | Repudiation | A test reported green that never ran (x86_64 leg skipped) | high | mitigate | Host-triple fallback widened to x86_64 hosts; ledger records `not-run-on-mac`; owner's x86_64 log shows `140 passed` and the mask test `ok` at `HEAD=4624ea2` | closed |
| T-02-16 | Tampering | Executable kernel code changed by a refactor meant to change none | high | mitigate | Six `.text`/`.text.boot` sha256 equality verdicts on four configs; any difference stops the phase | closed |
| T-02-17 | Repudiation | Evidence compared across venues | medium | mitigate | Venue lines match; the file states Mac hashes are not comparable with Linux ones | closed |
| T-02-18 | Repudiation | A quick-pass verdict without a run | medium | mitigate | Verdicts only from a leg's own exit code and log; reruns recorded | closed |
| T-02-26 | Tampering | A stale `boot-iso` `outputHash` reaching `main` after a later fix moved kernel bytes | high | mitigate | One-commit rule: host tests before the hash step; every log names its commit; old round voided and rerun at `4624ea2`; verdicts refuse other commits | closed |
| T-02-19 | Elevation of privilege | A checklist command that escalates on the owner's host | medium | mitigate | Only `nix`, `git`, `cargo xtask` and the witness driver, which refuses root and writes only under its work directory | closed |
| T-02-20 | Elevation of privilege | A push without owner approval | high | mitigate | Both stops `gate="blocking-human"`; executor ran only the exact approved command after printing the unpushed list | closed |
| T-02-21 | Tampering | An unexpected commit arriving on the transport branch | high | mitigate | Task 2 allowed only the hash commit (`flake.nix` only) and return commits (return directory only); a misplaced return commit was rejected and replaced | closed |
| T-02-22 | Repudiation | A verdict without a run | medium | mitigate | Verdicts only from returned lines; round 1's unstamped lines were rejected as FAIL | closed |
| T-02-23 | Tampering | A silent x86_64 kernel byte change | high | mitigate | Fixed-output `boot-iso` build with the re-derived hash, then `--rebuild`, at the gated commit; `HEAD=` lines equal `gated-commit`; `.text` equality from plan 02-05 | closed |
| T-02-27 | Tampering | `main` merged or pushed from an agent worktree or another branch | high | mitigate | Main-tree check opens every task and is a verify gate of Task 2; push ran from the main working tree with `main` checked out | closed |
| T-02-24 | Information disclosure | Credentials in returned gate logs | high | mitigate | Checklist forbids credentials; `secret-scan.txt` clean; no credential appeared and nothing needed redaction | closed |

*Status: closed · open · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-02-01 | T-02-SC | No package manager installs occur in this phase; `Cargo.lock` and `flake.lock` are unchanged. Residual supply-chain surface is the locked nix flake inputs, unchanged from Phase 1. | Phase 2 plan disposition (accepted, low, below block threshold) | 2026-11-24 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-24 | 28 | 28 | 0 | gsd-execute-phase orchestrator (ASVS L1 grep-depth; short-circuit path, `register_authored_at_plan_time: true`) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-24
