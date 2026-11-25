---
phase: "1"
slug: "stage-p0-cross-build-harness"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-23"
---

# Phase 1 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
>
> **Pass-1 scope (D-22).** Phase 1 pass 1 is a spike/documentation phase: it implements no HARN
> requirement and changes no r9 source. The threats below are the spike activity's own boundaries
> (running executor-authored scripts on the owner's machines, reading the pinned Harvey tree, handling
> DEMO credentials). Every mitigation was checked at L1 grep depth against the committed scripts,
> summaries and evidence; ASVS level 1.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Mac ↔ builder VM (lima) | Nix derivations run on the owner's aarch64-linux VM over loopback ssh | Read-only pinned Harvey source, nix store paths; no credentials |
| Mac ↔ GitHub ↔ Linux KVM host | Pull-only witness flow: the owner pulls r9, runs committed scripts, returns logs on a branch | Committed scripts, spike evidence, redacted logs |
| Spike scripts → owner's machines | Executor-authored shell/expect runs on the Mac and the Linux host | Build inputs; the scripts write only to their own scratch/output dirs |
| Pinned Harvey tree → derivations | Read-only reference at bd7a6982a; never built in place | Harvey source, the 9k10cpu.elf copy; no writes |
| Returned logs → r9 repository | Evidence excerpts from the owner's runs are committed | Host facts, gate transcripts; DEMO credentials must never cross |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-01-01 | Information disclosure | FlakeHub credential (native option) | high | mitigate | Owner chose lima; no FlakeHub login was taken. Native remains documented for pass 2 with an owner-run login and token-free evidence | closed |
| T-01-02 | Elevation of privilege | lima guest trusted-users entry | medium | mitigate | Guest-only `/etc/nix/nix.custom.conf` edit, exact revert recorded in `s1/evidence/builder-setup.txt`; no Mac `/etc/nix` edit; loopback ssh only | closed |
| T-01-03 | Tampering | Harvey checkout (read-only reference) | high | mitigate | Derivations use the pinned github input, not the checkout; `HARVEY-UNTOUCHED` guard passed (HEAD `25d97dc4d`, status sha256 `041d2929…`) | closed |
| T-01-04 | Tampering | r9 kernel, xtask, root flake | high | mitigate | Spike code lives under `spikes/s1` with `path:` refs; zero protected-path diffs/status in every plan | closed |
| T-01-05 | Elevation of privilege | Harvey scripts and plan9port in the build | low | accept | Runs in the nix sandbox inside a builder VM as mapped root of a nested userns; no network | closed (accepted) |
| T-01-SC | Tampering | nix inputs (nixpkgs, harvey) | low | mitigate | No npm/pip/cargo installs; committed flake locks; nixpkgs rev equals r9's root lock | closed |
| T-01-06 | Tampering | Builder copy-back of colliding names | medium | mitigate | 23/23 colliding files checked against git blob sha1 on the builder and the Mac; `nix store verify --no-trust` clean in a same-volume chroot store; system-store copy-back refused by signature policy and recorded | closed |
| T-01-07 | Denial of service | Builder VM CPU/memory | low | accept | Bounded by the VM size; owner-approved venue; `limactl stop` reverts | closed (accepted) |
| T-01-08 | Tampering | Harvey checkout | high | mitigate | Only `git ls-tree/show` and a read-only `ls`; `HARVEY-UNTOUCHED` passed | closed |
| T-01-09 | Tampering | r9 protected paths and `spikes/s1` | high | mitigate | Zero protected-path and zero `spikes/s1` diffs from plan 01-02 | closed |
| T-01-SC | Tampering | nix inputs and fixed-output fetch | low | mitigate | Locks by narHash; fixed-output fetch pinned by rev + SRI hash | closed |
| T-01-10 | Tampering | Harvey checkout | high | mitigate | `cp` out of `amd64/9k10cpu.elf` only; `HARVEY-UNTOUCHED` passed | closed |
| T-01-11 | Denial of service | Stray qemu on the Mac | low | mitigate | `boot-q35.sh` traps every exit path; acceptance `pgrep` clean | closed |
| T-01-12 | Tampering | r9 protected paths | high | mitigate | Plan builds nothing in r9; zero protected-path diffs | closed |
| T-01-SC | Tampering | nix shell of shellcheck/qemu | low | mitigate | qemu from the locked devshell; shellcheck from nixpkgs; no registry installs | closed |
| T-01-13 | Tampering | Harvey checkout | high | mitigate | All image work in `spikes/s6/work/harvey-arm64`; mtime check + `HARVEY-UNTOUCHED` | closed |
| T-01-14 | Tampering | r9 workspace and kernel crates | high | mitigate | Probe built by plain rustc outside the cargo workspace; zero Cargo/xtask diffs | closed |
| T-01-15 | Denial of service | Stray qemu on the Mac | low | mitigate | Expect drivers quit qemu; kill/reap on every exit path; `pgrep` acceptance | closed |
| T-01-16 | Elevation of privilege | `linux-x86_64.sh` on the owner's host | medium | mitigate | Refuses root and a real checkout, never calls sudo, writes only to scratch/S6_OUT, requires KVM | closed |
| T-01-SC | Tampering | Toolchain and nix shells | low | mitigate | Zero-dependency probe; toolchain from the locked devshell | closed |
| T-01-17 | Information disclosure | DEMO p9sk1 creds and drawterm password | medium | mitigate | Harvey driver redaction kept; scripts never copy `auth-creds.txt`; returned logs scanned clean | closed |
| T-01-18 | Tampering | Running the gate against a real Harvey checkout | high | mitigate | Scripts refuse `HARVEY/.git` or a `/nix/store` path; writes only to `S3_OUT` and the scratch copy | closed |
| T-01-19 | Information disclosure / DoS | Xvfb listener and survivor processes | low | mitigate | Xvfb TCP listening off; run-tag survivor sweep; killed on exit paths | closed |
| T-01-SC | Tampering | Harvey flake inputs | low | mitigate | Inputs from Harvey's committed lock, relocked in `spikes/s3`; no registry installs | closed |
| T-01-20 | Elevation of privilege / tampering | `run-all.sh` on the owner's host | high | mitigate | Refuses root, never calls sudo, writes only under `R9_SPIKE_WORK`/`./out`, verifies `MANIFEST.sha256`, pins Harvey/r9 by full sha; pull-only staging re-verifies the manifest before any step | closed |
| T-01-21 | Information disclosure | DEMO creds and password in returned logs | medium | mitigate | Driver deletes `auth-creds*` and scans for the password value; Task 3 rescanned every excerpt; `secret-scan.txt` clean | closed |
| T-01-22 | Tampering | Bundle in transit | low | mitigate | The scp path was retired for the owner's pull-only flow; `run-from-repo.sh` stages from git and verifies `MANIFEST.sha256` from a fresh extraction before running | closed |
| T-01-23 | Denial of service | Surviving qemu/Xvfb/drawterm | medium | mitigate (residual) | Gate sweeps and `survivors.txt` work; one drawterm (PID 88800) survived from the owner's manual graphical login attempt and is recorded for pass 2 teardown. Below the high threshold; non-blocking | closed — below high threshold (residual documented) |
| T-01-24 | Information disclosure | Guest hostfwd ports | low | mitigate | Loopback-only `127.0.0.1` binds from the committed netdev TOML | closed |
| T-01-SC | Tampering | nix inputs and GitHub clones on the owner's host | low | mitigate | Committed locks; git clones pinned by full commit sha; no registry installs | closed |
| T-01-25 | Repudiation | Owner decisions | low | mitigate | Reply recorded verbatim, dated, one `DECISION` line each, in committed `SPIKE-RESULTS.md` | closed |
| T-01-26 | Tampering | Mac configuration during revert | medium | mitigate | No revert requested; builder ledger retains exact commands; no `/etc/nix` edit; no FlakeHub logout needed | closed |
| T-01-SC | Tampering | Package installs (plan 01-07) | low | accept | Plan installs and fetches nothing | closed (accepted) |

*Status: open · closed · open — below {block_on} threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01-01 | T-01-05, T-01-07, T-01-SC (01-07) | Sandboxed spike code runs as mapped root inside a nested userns; the builder VM's resource use and the no-install plans are bounded and revertible | owner | 2026-11-22 |
| AR-01-02 | T-01-23 | One drawterm process survived the owner's manual graphical login attempt (PID 88800). It is outside the gate's teardown; pass 2 must sweep it before the next gate run | owner (recorded) | 2026-11-23 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-23 | 33 | 33 | 0 | execute-phase verify:post / L1 audit |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-23 — pass-1 spike phase, no source implementation (D-22)

## Security Audit 2026-11-23

| Metric | Count |
|---|---|
| Threats found | 33 |
| Closed | 33 |
| Open | 0 |

---

## Pass 2 — plan threat models (plans 01-19 … 01-34)

Pass-2 scope: the implementation plans build and move the gates, images, trees, witness driver and
the Harvey flip. The register below records the 49 pass-2 threats (36 numbered + 13 supply-chain
rows) verified against the implemented code and the stop-B/C/E evidence. Every high-severity threat
is closed.

| Threat ID | Plan | Category | Severity | Disposition | Evidence |
|---|---|---|---|---|---|
| T-01-63 | 01-19 | Tampering (ndb composition) | low | mitigate | `xtask/src/image.rs` strip + substitution; three ndb-template tests; `services.toml` reference match |
| T-01-64 | 01-19 | Tampering (bake-time compile drift) | medium | mitigate | `nix/harvey/tree-amd64.nix` builds `hmkdir` with the bake's own `6c/6l`; harvested sha equals the reference |
| T-01-SC | 01-19 | Tampering (package installs) | low | accept | No install commands; locked nixpkgs |
| T-01-65 | 01-20 | Spoofing (relabelled Harvey file as Rust) | high | mitigate | `image.rs` `check_provenance` + tests; stop-B3 probe marker |
| T-01-66 | 01-20 | Tampering (leftover probe code) | low | mitigate | Zero `source = "rust"` manifest entries; no probe subcommand remains |
| T-01-SC | 01-20 | Tampering (package installs) | low | accept | Pinned rustc; no crate added |
| T-01-67 | 01-21 | Tampering (shell injection) | medium | mitigate | `writeShellApplication` shellcheck; explicit shellcheck rc 0 |
| T-01-68 | 01-21 | Tampering (writes into tree) | medium | mitigate | Outputs under `R9_GATE_OUT`; `TREE-UNMODIFIED` |
| T-01-69 | 01-21 | Information disclosure (guest forwards) | low | mitigate | Loopback-only hostfwd; driver host-listener assertion |
| T-01-SC | 01-21 | Tampering (nixpkgs tools) | low | mitigate | Locked nixpkgs |
| T-01-70 | 01-22 | Information disclosure (returned logs) | high | mitigate | Driver secret scan; stop-B/C scans clean |
| T-01-71 | 01-22 | Repudiation (verdict without run) | medium | mitigate | WITNESS-LOG verdict/evidence pairs; summary.tsv rows |
| T-01-72 | 01-22 | Tampering (silent kernel-byte change) | high | mitigate | boot-iso FOD `outputHash`; stop-E build + `--rebuild` OK |
| T-01-73 | 01-22 | Elevation of privilege (driver on host) | high | mitigate | Root refusal; no sudo; work dir constrained |
| T-01-SC | 01-22 | Tampering (nix inputs) | low | mitigate | Locked flakes |
| T-01-74 | 01-23 | Tampering (shell injection) | medium | mitigate | `writeShellApplication` shellcheck; quoted expansions |
| T-01-75 | 01-23 | Tampering (writes into tree) | medium | mitigate | Outputs under `R9_GATE_OUT`; stop-E store verify |
| T-01-76 | 01-23 | Information disclosure (guest forwards) | low | mitigate | Loopback-only hostfwd |
| T-01-SC | 01-23 | Tampering (nixpkgs tools) | low | mitigate | Locked nixpkgs |
| T-01-78 | 01-24 | Information disclosure (tap venue) | medium | mitigate | `net-doctor --mode tap` preflight; stop-E soak/stress L2-TRUTH tap |
| T-01-79 | 01-24 | Tampering (host-echo source) | low | mitigate | `host-echo.c` byte-identical to the pin |
| T-01-SC | 01-24 | Tampering (nixpkgs tools) | low | mitigate | Locked nixpkgs |
| T-01-80 | 01-25 | Tampering (action supply chain) | medium | mitigate | Every `uses:` pinned to 40-hex; read-only permissions |
| T-01-81 | 01-25 | Repudiation (TCG mistaken for witness) | low | mitigate | Workflow header names local KVM as the witness; TCG explicit |
| T-01-SC | 01-25 | Tampering (package installs) | low | accept | No installs |
| T-01-82 | 01-26 | Tampering (owner's Harvey checkout) | high | mitigate | Work in `$HOME/r9-harvey-flip`; commit only after green C + approval |
| T-01-83 | 01-26 | Denial of service (deleting needed files) | medium | mitigate | D-19 read-set intersection empty; kept drawterm-gate exception; trees build |
| T-01-84 | 01-26 | Elevation of privilege (Harvey build sudo) | medium | mitigate | `nix/q35.sh` has no sudo; owner ran the build |
| T-01-85 | 01-26 | Tampering (equivalence baselines) | low | mitigate | `equivalence.tsv` paths/fresh runs; four recorded rows owner-approved |
| T-01-SC | 01-26 | Tampering (nix inputs) | low | mitigate | Pinned revs/locks |
| T-01-86 | 01-27 | Tampering (unapproved/wider Harvey commit) | high | mitigate | Commit path set identical to the approved diff; 36 paths, +285/−9972 |
| T-01-87 | 01-27 | Tampering (staging 11 font files) | high | mitigate | Zero font paths in the commit; 11 remain unstaged |
| T-01-88 | 01-27 | Repudiation (commit attribution) | low | mitigate | No AI attribution; subject matches the plan |
| T-01-89 | 01-27 | Information disclosure (publishing `25d97dc4d`) | low | accept | Owner-owned push; recorded in HARVEY-FLIP |
| T-01-SC | 01-27 | Tampering (package installs) | low | accept | No installs |
| T-01-90 | 01-28 | Spoofing (unpushed/wrong rev locked) | medium | mitigate | ls-remote = lock rev = `c697666`; no nested r9 |
| T-01-91 | 01-28 | Tampering (non-lock changes) | low | mitigate | Bump commit is `flake.lock` only |
| T-01-92 | 01-28 | Information disclosure (final evidence) | high | mitigate | stop-E secret scan clean; repo-wide `password=` matches classified as contracts/fixtures/docs |
| T-01-SC | 01-28 | Tampering (package installs) | low | accept | No installs |
| T-01-93 | 01-29 | Tampering (profile on unverified image) | medium | mitigate | Six profiles use `image:*`; no `harvey:` module; reference compare before edits |
| T-01-SC | 01-29 | Tampering (package installs) | low | accept | No installs |
| T-01-94 | 01-30 | Tampering (shell injection) | medium | mitigate | `writeShellApplication` shellcheck |
| T-01-95 | 01-30 | Tampering (writes into arm64 tree) | medium | mitigate | Outputs under `R9_GATE_OUT`; `TREE-UNMODIFIED` |
| T-01-96 | 01-30 | Information disclosure (net-smoke forwards) | low | mitigate | No hostfwd in the aarch64 net-smoke/cpu drivers |
| T-01-SC | 01-30 | Tampering (nixpkgs tools) | low | mitigate | Locked nixpkgs |
| T-01-77 | 01-31 | Information disclosure (password in env/argv) | medium | mitigate | 0600 password file, unset, EXIT-trap removal; no by-value password |
| T-01-97 | 01-31 | Information disclosure (drawterm developer password) | medium | mitigate | Presence-only creds check; `-s '' -O` only; no extraction |
| T-01-98 | 01-31 | Information disclosure (boot tap forwards) | low | mitigate | net-doctor tap preflight; stop-C tap runs PASS |
| T-01-SC | 01-31 | Tampering (nixpkgs tools) | low | mitigate | Locked nixpkgs |

**Blocking count:** 8 high-severity threats (-65, -70, -72, -73, -82, -86, -87, -92) — all **CLOSED**.
No open threat at any severity.

### Pass-2 audit warnings (non-blocking; never counted in `threats_open`)

| ID | Finding |
|---|---|
| W-1 | Plans 01-33 and 01-34 have no `<threat_model>` despite changing trusted-kernel code (`trap.rs`, `proc.rs`, `syscall.rs`, `timer.rs`, `churngate.rs`). Audit-trail gap, not evidence of a vulnerability; recommend a retroactive threat model or explicit owner risk acceptance before any future kernel change. |
| W-2 | `01-25-SUMMARY.md` and `01-26-SUMMARY.md` omit `## Threat Flags`; their surfaces are covered by T-01-80/81 and T-01-82…85. Format-only. |
| W-3 | `witness/evidence/stop-e/secret-scan.txt` records a placeholder pattern rather than the exact reproducible pattern; an independent repo-wide scan is clean. Strengthen the evidence record. |
| W-4 | `flake.nix` still carries the pre-flip comment about a nested `harvey/r9` lock node; the lock has none. Documentation drift only. |
| W-5 | The applied Harvey commit message carries the planned subject only, not the body bullets; no attribution, so T-01-88 remains closed. |
| W-6 | Four `recorded` equivalence rows are owner-approved expectations rather than committed captures/fresh runs; documented in 01-34 with per-row rationale; the gate expect files remain the acceptance. Accepted risk AR-01-05. |

### Pass-2 accepted risks

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---|---|---|---|---|
| AR-01-03 | T-01-89 | The owner's push published the pre-existing freeze commit `25d97dc4d` as the flip parent; owner-owned decision recorded in HARVEY-FLIP | owner | 2026-11-23 |
| AR-01-04 | T-01-SC (01-19, 01-20, 01-25, 01-27, 01-28, 01-29) | No package installs and no crate added; toolchains/inputs are the locked nixpkgs and pinned rust toolchain | owner (plan dispositions) | 2026-11-23 |
| AR-01-05 | T-01-85 | Four `recorded` equivalence expectations are owner-approved with per-row rationale; committed gate expect files remain the acceptance | owner | 2026-11-23 |

### Pass-2 audit trail

| Audit Date | Threats Total | Closed | Open | Run By |
|---|---|---|---|---|
| 2026-11-23 | 82 (33 pass-1 + 49 pass-2) | 82 | 0 | pass-2 restore audit / L1 |

**Pass-2 approval:** verified 2026-11-23 — all 49 pass-2 threats closed; `threats_open: 0`; W-1 is the
only open process item and is recorded, not a vulnerability.
