---
phase: "04"
slug: "r9-userspace-exec-harvey-r9-abi"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-02"
---

# Phase 04 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| host→guest (loader + patched DTB) | initrd image bytes and DTB /chosen properties cross from host-controlled files into kernel-consumed memory | kernel image delivery (host-controlled) |
| guest user→kernel (EL0 SVC) | every syscall argument crosses the EL0→EL1 trust boundary | user pointers, syscall numbers, fd ops |
| guest process→process | RFORK children share no writable pages; /proc/N/note crosses process scope | notes (text), wait records |
| donor guest serial | trace fragments cross from the instrumented donor kernel to the host parser | syscall trace (build-time evidence only) |
| crates.io→r9 | miniz_oxide 0.9 / adler2 2 (04-02) — the phase's only package installs | decompression library code |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-04-01 | Tampering | main9 pagealloc setup | high | mitigate | initrd PhysRange reserved in used_ranges before init_page_allocator (Pitfall 9) | closed |
| T-04-02 | DoS | /chosen initrd discovery | medium | mitigate | range validated against DTB memory node before use; violation prints + panics loudly | closed |
| T-04-03 | DoS / info leak | port/src/paqfs.rs parser | high | mitigate | every offset bounds-checked; magic/version/size validated; adler32 per block; negative host tests prove Err-without-panic | closed |
| T-04-04 | DoS | DeflateEnc decode (miniz_oxide) | medium | mitigate | decompress hard-capped at blocksize; exact-length + adler32 verification | closed |
| T-04-05 | Tampering | trap.rs / fault decode | high | mitigate | in-frame ESR/FAR deleted (compile-enforced); both registers read live at trap entry | closed |
| T-04-06 | Elevation of Privilege | trap.S restore path | high | mitigate | SP_EL0 written only for EL0t-origin frames; EL1h returns write ELR/SPSR only; post-review fix adds force_psr_el0t on NOTED restore (WR-04) | closed |
| T-04-07 | DoS | const-assert/xtask static checks | medium | mitigate | two independent layers (in-crate const asserts + cross-repo ureg-check with proven negative control); drift is a hard failure | closed |
| T-04-08 | Tampering / DoS | exec.rs a.out parser | high | mitigate | donor sysexec check set ported (magic/HDR/entry-in-text/sizes-vs-USTOP/alignment); loud Err before any mapping | closed |
| T-04-09 | DoS | fault.rs (trap context) | high | mitigate | IRQs-off handler; kernel-context fault panics loudly with fault class; brief MCS lock, never under PROC_TABLE; post-review fix: wild user FAR kills process, never panics (WR-08) | closed |
| T-04-10 | DoS | main.rs D-10 teardown ordering | high | mitigate | Pitfall-11 sequence; `exec: ttbr0 switched, console alive` liveness print at every exec-profile boot | closed |
| T-04-11 | Elevation of Privilege | syscall.rs arg handling | critical | mitigate | validate_user_range on every user-VA argument (Sargs block, buffers, paths, argv strings) before any kernel read/write; invalid → errstr + -1 | closed |
| T-04-12 | DoS | devcons write vs timer IRQ | high | mitigate | tick print cfg(not(dev_exec))-gated out of exec profiles; PL011 RX lock-free ring | closed |
| T-04-13 | Tampering / DoS | exec.rs image replacement | high | mitigate | parse+validate+precheck before freeing the old image; failure leaves old image intact; post-review fix: post-free seed failure kills via exit_tail (WR-07) | closed |
| T-04-14 | Tampering | trap.S stack/frame discipline | critical | mitigate | CURKSTACK/CURFRAME per-proc kernel stacks; preemption swaps stack+frame+TTBR0 as one locked step; post-review fixes: rfork_clone error unwinding (WR-02), RFNOWAIT accounting (WR-03) | closed |
| T-04-15 | Elevation of Privilege (cross-process) | RFORK address-space copy | high | mitigate | eager page-for-page copy into fresh phys pages mapped only in the child's root table; fd table value-dup'd | closed |
| T-04-16 | DoS | trap.rs note delivery | high | mitigate | donor notify() okaddr discipline: validate-then-write the whole window, sp alignment, loud kill on bad target; post-review fix: in-handler guard (WR-05) | closed |
| T-04-17 | Elevation of Privilege (cross-process) | devproc-lite write path | medium | mitigate | numeric-pid-only resolution against PROC_TABLE (no broadcast/notepg); NOTIFY handler VA confined to caller's own text; exec clears the handler | closed |
| T-04-SC | Tampering | npm/pip/cargo installs | high | mitigate | only miniz_oxide/adler2 added (04-02), both OK-seam approved by the 04-RESEARCH.md Package Legitimacy Audit; versions pinned by Cargo.lock; every other plan installed nothing | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|

*No accepted risks.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-02 | 18 | 18 | 0 | orchestrator (L1 grep-depth, ASVS 1; register authored at plan time) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log (none)
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-02
