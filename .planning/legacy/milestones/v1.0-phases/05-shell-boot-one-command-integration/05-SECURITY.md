---
phase: "05"
slug: "shell-boot-one-command-integration"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-04"
---

# Phase 05 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| EL0 → kernel syscall surface | untrusted user pointers/args cross into r9 on every syscall (the measured 26-syscall set + fd/pipe/mount family) | user VAs, path strings, buffers, fd numbers |
| per-process namespace (pgrp) | mount table shared across rfork groups — bad union/stitch semantics leak or hide files across processes | namespace visibility |
| kernel ↔ 9P server (devmnt pipe) | the kernel 9P client parses R-messages from a user-space server (paqfs) — malformed/malicious replies | 9P wire bytes, msize negotiation |
| console input path | PL011 IRQ → bounded queue → cooked discipline → note posting | keystrokes, DEL→interrupt |
| host harness → guest | qemu/lima/drive scripts bridge host and guest; teardown must not kill foreign processes | process lifecycle |
| flake supply chain | inputs.r9 pin, nix-installer action, vendored locks | build inputs |
| image bake | root-tree → bootfs.paq; auth surface by absence | shipped binaries |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-05-01 | Tampering | trap.rs note-delivery fix | medium | mitigate | root-caused x9-clobber fix (r9 `83cbbbe`); 20/20 virtnotes + 20/20 virtmulti burial ×2; DEL gate re-exercises per smoke run | closed |
| T-05-02 | Tampering | xtask harness kill patterns | medium | mitigate | D-16: fresh serial log per run; qemu spawned as process-group leader and group-killed; orphan-verify leg proves no over/under-kill | closed |
| T-05-03 | Tampering | shared boot-chain C sources | high | mitigate | additive objtype-gated `mkfile.aarch64` + hand-written `bootr9.c`; `git diff --exit-code sys/src/9k/` clean; rcmain/init.c/namespace/termrc explicitly checked | closed |
| T-05-04 | Elevation of Privilege | image auth surface | medium | mitigate | auth-stub-by-absence (D-02): `/boot/factotum` asserted ABSENT at every bake by mkimage.sh's manifest assert | closed |
| T-05-05 | Tampering | 05-03-trace.txt measurement integrity | high | mitigate | schema + name-resolution gates, expected-set gate, opencons contiguous-run check, zero source-read names folded; coverage boundary named in artifact header | closed |
| T-05-06 | Tampering | syscall path-string/buffer handling | high | mitigate | T-04-11 discipline → post-refactor: every access via `usermem` fused validate+reside constructors (`aarch64/src/usermem.rs`); CR-02 hardened stat/fstat; brk_ bounds-checked vs bss base/USTOP (05-04 flag) | closed |
| T-05-07 | Elevation of Privilege | namec/pgrp union + MCREATE semantics | medium | mitigate | literal 9k chan.c/pgrp.c port; host tests pin union order/stitching/RFCNAMEG isolation (`port/tests/namespace_test.rs`) | closed |
| T-05-08 | Information Disclosure | devenv per-pgrp isolation | medium | mitigate | host-pinned share/copy/empty + two-group isolation; seeded set carries no secrets; MAXENVSIZE-bounded writes behind validation | closed |
| T-05-09 | Tampering | devroot raw-image node | low | mitigate | read-only node: named write/exec refusals, bounds-checked reads, mount-time collision refusal (`devroot.rs:186-199`) | closed |
| T-05-10 | Tampering | fd/pipe fork-dup semantics | medium | mitigate | Arc-shared queue + clone-on-dup + host-pinned fork matrix + bounded named-overflow queue (`devpipe.rs`, `devpipe_test.rs`) | closed |
| T-05-11 | Information Disclosure | devsrv fd registration | low | mitigate | write clones the registered chan (outlives registrant, host-pinned); listing exposes names only | closed |
| T-05-12 | Tampering | sys_pipe/sys_dup/sys_create user args | medium | mitigate | validation-first handlers (sargs + validate_user_range + 32-bit int masking); sys_seek validates fd, never touches user memory | closed |
| T-05-13 | Tampering | devmnt R-message parsing | high | mitigate | mntchk port: every R-message bounded vs negotiated msize BEFORE body read, tag-matched, checked cursor with exact-consumption; host-proven rejection of oversized/mis-tagged/msize-raising replies; protocol errors are errstrs, never panics | closed |
| T-05-14 | Tampering | mount-table stitching into devmnt walks | medium | mitigate | mount-crossing matrix host-pinned (MAFTER tie-break, union fall-through, MREPL shadow); WR-04 fix: pgrp lock never held across devmnt RPCs | closed |
| T-05-15 | Denial of Service | devcons input queue | medium | mitigate | 4 KiB bounded queue, drop-oldest + one-line warning, host-tested with 5000-byte flood; IRQ handler never blocks | closed |
| T-05-16 | Elevation of Privilege | DEL→interrupt posting | medium | mitigate | DEL posts only to console readers' note groups (RFNOTEG machinery) — no broadcast; rides the 04-08 delivery boundary | closed |
| T-05-17 | Information Disclosure | devproc status/args reads | low | mitigate | per-pid exact resolution; argv kernel-recorded at exec (never read back from user memory); world-readable per the Plan 9 model (named in register) | closed |
| T-05-18 | Tampering | leftover syscall handlers | medium | mitigate | dispatch table = exactly the measured set; everything unmeasured stays loud-None (`syscall N unimplemented` + -1) | closed |
| T-05-19 | Tampering | packages.kernel-virt build | low | mitigate | derivation builds from locked flake source; rust-toolchain.toml pin; Cargo.lock-vendored deps; committed DTB from tracked tree | closed |
| T-05-20 | Tampering | inputs.r9 pin | medium | mitigate | exact-rev `9549cee` + narHash lock in flake.lock; dev override documented dev-only | closed |
| T-05-21 | Tampering | compose script path handling | low | mitigate | kernel/DTB only from the nix-controlled read-only store path (R9_KERNEL_PKG), presence validated before launch; no caller-supplied path exec'd | closed |
| T-05-22 | Tampering | smoke capture lifecycle | high | mitigate | fresh capture per run (caller + driver); two-consecutive-pass gate; markers are the only proof; post-run orphan check in verify | closed |
| T-05-23 | Denial of Service | smoke kill patterns | medium | mitigate | driver group-kills its spawned qemu on every exit path; WR-07 fix (`0f70588e7`): survivor sweeps keyed on per-run unique `-name` tag — foreign r9 qemus never match | closed |
| T-05-24 | Tampering | CI's nix-install action | medium | mitigate | pinned to full 40-char SHA `3138316d…` (v23), gate-enforced; permissions `contents: read` explicit | closed |
| T-05-25 | Tampering | 05-EVIDENCE.md integrity | medium | mitigate | D-13 ordering gate re-run mechanically at evidence-commit time; every SC row quotes log lines; the then-unmet row was marked pending, never claimed — now resolved (run 41825728636 green) | closed |
| T-05-SC (×13 plans) | Tampering | npm/pip/cargo installs | high | mitigate | zero package installs all phase; the new harness crate (`build/harness/`) is std-only with a committed zero-dep Cargo.lock; the one new flake input is the project's own fork | closed |

*Status: open · closed — all 38 rows closed (25 phase threats + 13 per-plan supply-chain rows)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on (high) count toward threats_open*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|

No accepted risks.

**Summary-flag notes (visibility, not risks):** two threat_flags were raised during execution and closed inline — 05-04 (`sys_brk_` full-width address: bounds-checked vs bss base/USTOP, demand-zeroed, no nested paging) and 05-06 (`ensure_user_resident` pages in user buffers at syscall entry — the T-05-06-safe closure of the cold-pointer class). Both were further hardened by the phase review-fix series (CR-02).

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-04 | 38 | 38 | 0 | verify-work orchestrator (L1 grep-depth; register authored at plan time; asvs_level 1 short-circuit per secure-phase step 3) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log (none)
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-04
