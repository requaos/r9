---
phase: "06"
slug: "full-tree-userspace-wave-0-libraries-core-tiers"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-05"
---

# Phase 06 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| host toolchain -> swept artifacts | cross-compiled 7c/7l output is the tier payload; a tampered compiler poisons every marker | object archives, executables |
| serial log -> host asserts | the venues' only evidence channel; a false-green gate corrupts the phase's marker truth | boot/sweep logs, marker lines |
| embedded root -> kernel image | working-tree binaries/data baked into the booted kernel; provenance is the clean-checkout build | 43-file devroot payload |
| tier allowlists -> mk filter | tier files are data consumed by the objtype-gated mk recursion; script-written edits would bypass review | tier0-libs.txt, tier1-cmds.txt |
| CI workflow -> runner | phase6.yml provisions toolchains and runs qemu on hosted runners; unpinned actions are a supply-chain seam | GitHub Actions, nix store |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-06-01 | Denial of Service | cc/macbody + 6l/obj.c gethunk loops | medium | mitigate | unbounded-loop fixes + timeout-gated repros (`build/aarch64/gethunk-repro-cc.sh`, `gethunk-repro-6l.sh`) | closed |
| T-06-02 | Tampering | shared cc frontend + 6l (amd64 host toolchain) | high | mitigate | tail-only additive edits; amd64-safety re-proof (06-01; k10cpu relink re-proven again in 06-09) | closed |
| T-06-03 | Tampering | build/aarch64/tiers/*.txt | medium | mitigate | tier files are data — append-by-edit only, reviewable diffs; no script/mk recipe writes them | closed |
| T-06-04 | Spoofing | objtype-gated mk filter | high | mitigate | mk -n amd64 byte-parity (PARITY-OK ×5) + filtered-listing equality probes (06-02 SUMMARY; filter in `sys/src/mkfile`, `sys/src/cmd/mkfile`) | closed |
| T-06-05 | Spoofing | marker assertion block | high | mitigate | fresh serial log per run + artifact sha256 at gate time + `T1-DONE`/`T1-VENUE-PASS` required + zero-`panic|trap:` whole-file scan | closed |
| T-06-06 | Denial of Service | gate teardown (qemu/srv9/gdbpoke PIDs) | medium | mitigate | cleanup-trap discipline preserved in `subset-exec.sh` (three PIDs + TMPD, `exit $status`) | closed |
| T-06-07 | Spoofing | dir-coverage.txt reach claim | high | mitigate | sweep runs COVERAGE_SWEEP=1 — D-13 filters disengaged at every depth; the sweep never certifies the allowlist it rides | closed |
| T-06-08 | Tampering | generator-audit.txt / generator-scan.sh pair | medium | mitigate | whitelist derived from the audit file; both committed + CI-regenerated; drift reds | closed |
| T-06-09 | Spoofing | tier-0 artifact markers | high | mitigate | D-07 fresh-archive (`rm -f` before mk install) + sha256 at gate time + `$O`-suffixed OFILES | closed |
| T-06-10 | Denial of Service | residual hang wedging the lib wave | medium | mitigate | gethunk class closed pre-wave (06-04 sweep proof); per-lib failure isolation + second-chance visitation pass | closed |
| T-06-11 | Tampering | srv9 --writable-prefix surface | medium | mitigate | writable scope exactly /tmp + /usr/glenda/tmp, opt-in on the gate script only | closed |
| T-06-12 | Spoofing | self-host canary evidence | high | mitigate | compile and run markers separate; known-output string required in the retained serial log | closed |
| T-06-13 | Tampering | flake.lock r9 pin | high | mitigate | push-before-pin with locked-rev==live-HEAD gate; pin bump is its own flake.lock-only commit | closed |
| T-06-14 | Denial of Service | tier1-r9 survivor sweep | medium | mitigate | R9_RUN_TAG per-run keying + bracket-fixed `pkill -f "[q]emu…${R9_RUN_TAG}"` | closed |
| T-06-15 | Spoofing | baked-image freshness | high | mitigate | mkimage.sh manifest asserts name every tier-1 path pre-bake; real-init rodata guard tripwire | closed |
| T-06-16 | Tampering | phase6.yml action pins | high | mitigate | every action 40-char-SHA pinned (checkout 11d5960a…, nix-installer 3138316d…, upload-artifact ea165f8d… — 11/11 PINS-OK) | closed |
| T-06-17 | Spoofing | USPACE-10 verdict | high | mitigate | job asserts verdict PRODUCTION (banner + classification file), never the outcome; empty capture = FAIL no-banner | closed |
| T-06-18 | Denial of Service | 90-min TCG serial legs amplifying per push | low | mitigate | `concurrency: { group: phase6-<ref>, cancel-in-progress: true }`; fast/slow job split | closed |
| T-06-19 | Tampering | boot-9k10t1-sweep.sh marker asserts | medium | mitigate | CR-normalized log + boundary-anchored regex (`T1-OK $m([^A-Za-z0-9.-]|$)`) + SKIP whitelist converse assert | closed |
| T-06-20 | Tampering | evidence interpretation (embedded-root venue) | medium | mitigate | `9k` venue token names the venue in the retained log; write-flow/build-chain/selfhost/libtest emit named T1-SKIP (never unearned T1-OK); no amd64 tier-ledger lines | closed |
| T-06-21 | Tampering | sys/src/9k/port/devroot.c (shared 9k code) | medium | mitigate | Nbootfiles 32→64 additive-only; k10cpu relinked same build; x86_64-poolpanic CI leg re-boots it every push | closed |
| T-06-22 | Tampering | phase6.yml job roster | high | mitigate | tier-donor retirement CI-asserted: wave0-host venue-pin step negative-greps the retired job key + donor-script references | closed |
| T-06-23 | Tampering | wave0-host / tier-r9 / x86_64-poolpanic job bodies | medium | mitigate | byte-preservation discipline on the three green-proven jobs (git-diff range review in the 06-10 verify gate) | closed |
| T-06-24 | Tampering | dir-coverage.txt wording vs generator emission | low | mitigate | D-15 normalized drift diff enforces venue-block byte-equality on every push | closed |
| T-06-SC | Tampering | npm/pip/cargo installs (reserved supply-chain row) | high | mitigate | no package installs in any plan; no new actions; existing pins untouched | closed |

*Status: closed — all 25 register rows verified at L1 (grep-depth) against the implementation on 2026-11-05*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on (high) count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

No accepted risks.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-05 | 25 | 25 | 0 | gsd-secure-phase (L1 grep-depth; ASVS level 1 short-circuit — register authored at plan time, threats_open: 0) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log (none)
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-05
