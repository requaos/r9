---
phase: "02"
slug: "aarch64-toolchain-libc-9front-donor"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-01"
---

# Phase 02 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| network → .build/9front-donor | the pinned 9front clone is the single external code source this phase fetches | donor source code (supply chain) |
| vendored compiler/linker → host | 7a/7c/7l run as host binaries with the developer's privileges | compiled artifacts |
| guest scratch injection | pre-gate scaffolding entered the disposable guest tree only; the committed tree stays scaffolding-free | build scaffolding |
| committed contract → r9 repo (Phase 4) | ureg.h + ABI.md become a cross-repo published contract; an error propagates into r9's svc dispatch | frozen syscall ABI |
| donor tree → guest namespace build | the pinned donor's mkfiles and helper tools execute in the lima guest under sudo unshare | donor build system |
| qemu guest → host | the donor kernel and Harvey binaries execute as guest code under qemu TCG on the developer's machine | guest kernel + user binaries |
| serial/gdbstub/QMP sockets | local unix sockets carry guest console I/O and the gdbstub control channel | console data, guest memory control |
| committed evidence → git history | gate-evidence logs are the phase's proof artifacts; they must faithfully record what executed | execution evidence |
| default build scope | linux-build activation changes what every future `nix run .#build` produces | build enumeration |

Web-style ASVS categories (V2 authn, V3 session, V4 access control, V6 crypto) are not applicable to a toolchain/libc phase; stated explicitly rather than fabricated. The real security content is supply-chain discipline (ASVS V14 analog) and evidence integrity.

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-02-01 | Tampering | .build/9front-donor clone | high | mitigate | pin-verified fetch of SHA 350266104f1fed81c78c3d05fa44f523bec0ae4d with hard rev-parse gate before any copy; pin recorded in each vendored dir's README (D-10) — verified present in sys/src/cmd/7a/7c/7l READMEs and build/aarch64/*.sh | closed |
| T-02-02 | Elevation of Privilege | host-built 7a/7c/7l | low | accept | ken-suite lineage, decades of hostile-source exposure, run unprivileged; no setuid/privileged paths; relaxed -Wno-* flags suppress warnings only, never checks | closed |
| T-02-03 | Tampering | mach.h enum excision | medium | mitigate | enum-position shift in-memory-only (object files carry magic numbers, never enum values — Pitfall C); pre-excision stale objects deleted; all consumers recompile together; zero-residue grep gate re-verified 2026-11-01 (0 alpha lines in libmach/mach.h) | closed |
| T-02-04 | Tampering | 9syscall `case aarch64` (ABI freeze) | high | mitigate | donor-verbatim emission block (verified line-by-line by the phase verifier: TEXT/MOV R0, 0(FP)/SVC/RETURN, `_seek` rename); full-stub assembly exercised in the namespace build; execution proof on the reference kernel before Phase 4 consumes the contract | closed |
| T-02-05 | Tampering | pre-gate guest scratch injection | medium | mitigate | scratch files lived only in the disposable rsync'd guest copy; pre-gate `test ! -e aarch64` + git-status gates held at gate time; plan 02-06 sha256-verified scratch-used headers == committed headers; stale post-02-06 assertion removed by CR-01 fix (064c6a185) | closed |
| T-02-06 | Tampering | donor tree build inputs | medium | mitigate | donor pinned at 350266104f… with hard rev-parse gate before any build (donor-vm-build.sh/donor-ns-build.sh); build runs inside the lima guest mount namespace (sudo unshare -m), not on the host; donor sources never patched (D-12 held — qemu-11.1.2 drifts accommodated harness-side only) | closed |
| T-02-07 | Elevation of Privilege | qemu guest execution on host | low | accept | stock qemu TCG running unprivileged guest code; no host devices passed through; serial/QMP/gdbstub via local unix sockets in a tmpdir removed by the exit trap; guest network-less in this harness configuration | closed |
| T-02-08 | Tampering | gate-evidence.log false positive | high | mitigate | evidence requires the marker line AFTER the rc prompt plus sha256 equality between executed binary and committed build/aarch64/hello.7.out plus zero panic/trap lines (3 PASS blocks, 0 fault lines); human-approved at the 02-05 tracer gate 2026-11-01; drive9.py prompt-proof strengthened post-review (WR-04: anchored ^term% $, order-dependent search_from, rc-^-split probe so echo alone cannot satisfy it) | closed |
| T-02-09 | Tampering | aarch64/include/ureg.h + ABI.md (published contract) | high | mitigate | headers sha256-gated byte-verbatim against the pinned donor (u.h dd1bd611…, ureg.h 04910dfc…); ABI.md gated on the corrected SP+8 wording; contract execution-validated on the reference kernel (02-05) before r9 consumes it | closed |
| T-02-10 | Tampering | NMOUNT leakage onto the donor kernel (false green) | high | mitigate | D-09 host-nm audit over every installed subset binary asserting zero nmount stub references — re-run live during this audit 2026-11-01: 0 references across aarch64/bin; execution log asserts zero trap/panic/note lines | closed |
| T-02-SC | Tampering | npm/pip/cargo installs | high | mitigate | no registry package installs anywhere in this phase — the only fetch is the pinned git clone (T-02-01); package-legitimacy gate vacuous, recorded per policy (plans 02-01…02-07) | closed |
| TF-02-04-01 | Elevation of Privilege | qemu gdbstub unix socket (02-04 threat flag) | low | accept | gdbstub on a tmpdir unix socket is a guest memory read/write control channel — same trust boundary and lifecycle as T-02-07's serial/QMP sockets: local, unprivileged, unlinked by the exit trap; no new external inputs | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-02-01 | T-02-02 | Host-built ken-lineage compilers run unprivileged with developer privileges only; decades of hostile-source exposure; relaxed -Wno-* flags suppress warnings, never checks | plan 02-01 register (recorded here per workflow) | 2026-11-01 |
| AR-02-02 | T-02-07 | qemu TCG guest execution on the host is stock, unprivileged, device-free, network-less; serial/QMP sockets are local tmpdir unix sockets removed on exit | plan 02-04 register | 2026-11-01 |
| AR-02-03 | TF-02-04-01 | gdbstub control socket is local-only, unprivileged, and unlinked by the harness exit trap — same boundary as the plan-modeled serial/QMP sockets | 02-04 threat flag (recorded here per workflow) | 2026-11-01 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-01 | 12 | 12 | 0 | gsd-secure-phase (L1 grep-depth; ASVS 1 short-circuit — register authored at plan time, threats_open 0) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-01
