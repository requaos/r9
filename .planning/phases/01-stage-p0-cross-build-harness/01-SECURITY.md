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
