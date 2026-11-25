# Phase 5: Guide Phase 3 - Cryptographic Memory and Zero-Copy DMA Paging - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-25
**Phase:** 5-Guide Phase 3 - Cryptographic Memory and Zero-Copy DMA Paging
**Areas discussed:** Backing store and scrubbing, Vector path vs FP rule (N7), DMA ingress keys and proof, Scope and review process

---

## Backing store and scrubbing

### Where an evicted page's sealed ciphertext lives

| Option | Description | Selected |
|--------|-------------|----------|
| In-RAM sealed frames (Recommended) | Ciphertext in kernel-owned RAM frames behind a BackingStore seam; eviction frees the shadow frame; a dirty anonymous page nets no memory back; out-of-RAM store later | ✓ |
| Plan 9 swap file over 9P | Sealed pages written to a swap file chan; real relief; pager must block on RPCs from fault context | |
| Both: RAM first, swap file last | In-RAM store and gates first, swap-file store as the final plans | |

**User's choice:** In-RAM sealed frames

### Whether sealed paging is on in every boot

| Option | Description | Selected |
|--------|-------------|----------|
| Switch, on at phase end (Recommended) | Behind a boot switch; last plan turns it on for every profile as an owner stop | |
| New profiles only | Off in default boots for the whole phase; default-on deferred | ✓ |
| On from the start | No switch; eager mapping replaced outright | |

**User's choice:** New profiles only

### N9: frame reused by a different key domain

| Option | Description | Selected |
|--------|-------------|----------|
| Guide rule, typestate-enforced (Recommended) | No zeroing when fully overwritten before mapping; CLFLUSHOPT cross-domain; RFC wording amended | |
| Zero and flush cross-domain | The RFC rule; only same-KeyID reuse skips both; guide wording corrected | ✓ |
| Zero cross-domain, no flush | Zero only; MEM-06 amended | |

**User's choice:** Zero and flush cross-domain

### Key domain before RFCKEY

| Option | Description | Selected |
|--------|-------------|----------|
| Per process, shared text (Recommended) | Per-process key for private pages from a boot secret; shared text under a per-image read-only key | ✓ |
| One system key | Single boot-derived key; cross-domain path reachable only from host tests | |
| Per segment | A key per segment of every process | |

**User's choice:** Per process, shared text

---

## Vector path vs FP rule (N7)

### How the fault decrypt path gets its registers

| Option | Description | Selected |
|--------|-------------|----------|
| Restricted section in fpu (Recommended) | A second, narrower section kind in the fpu module: six registers, guard token, same rules | ✓ |
| Plain with_simd() | Existing full section; full save and restore per cold fault | |
| Guide's own spill, outside fpu | cryptpage.rs spills itself; a named exception to the owner's rule | |

**User's choice:** Restricted section in fpu

### AVX enablement

| Option | Description | Selected |
|--------|-------------|----------|
| Enable AVX, XSAVE user state (Recommended) | OSXSAVE, XCR0 = x87/SSE/AVX, AVX-512 off; user FP state moves to XSAVE in every x86_64 boot | ✓ |
| Stay SSE: xmm0-xmm5 | 128-bit SSE2 path; MEM-04 amended | |
| AVX on only when sealed paging is on | AVX and XSAVE only under the boot switch | |

**User's choice:** Enable AVX, XSAVE user state

### CPUs without AVX2

| Option | Description | Selected |
|--------|-------------|----------|
| Scalar fallback plus one TCG vector run (Recommended) | CPUID picks the path; an extra TCG run with an AVX2 CPU model covers the vector path | ✓ |
| Scalar fallback, vector under KVM only | No new TCG configuration | |
| Require AVX2 for sealed paging | Profiles refuse to start sealed paging without AVX2 | |

**User's choice:** Scalar fallback plus one TCG vector run

### Roaring SIMD kernels

| Option | Description | Selected |
|--------|-------------|----------|
| Measure first, build if needed (Recommended) | Benchmark the scalar set difference; build only if it is a visible share of eviction | |
| Build them now | AVX2 kernels behind the setops seam, fearless_simd probe first | ✓ |
| Defer again | Scalar only, no measurement | |

**User's choice:** Build them now

### MEM-02 versus MEM-03 order

| Option | Description | Selected |
|--------|-------------|----------|
| Verify first, then decrypt (Recommended) | MAC checked before any plaintext; some latency hiding lost | |
| Decrypt, then verify before publish | Decrypt into the unmapped frame, SFENCE, MAC during the drain, publish only on pass | ✓ |
| Build both, benchmark decides | Both behind a switch; owner chooses at a stop | |

**User's choice:** Decrypt, then verify before publish

### Roaring SIMD architectures and form

| Option | Description | Selected |
|--------|-------------|----------|
| AVX2 only, probe picks the form (Recommended) | x86_64 only; fearless_simd probe, inline asm fallback | |
| AVX2 and NEON, probe picks the form | Both architectures behind the same seam | ✓ |
| AVX2 only, inline asm, no probe | No fearless_simd | |

**User's choice:** AVX2 and NEON, probe picks the form

---

## DMA ingress keys and proof

### What "DMA with no kernel copy" means over TCP

| Option | Description | Selected |
|--------|-------------|----------|
| No copy above the transport (Recommended) | Data pages read from the transport straight into destination frames; TCP-stack copies remain; MEM-09 amended | ✓ |
| True NIC placement | Mergeable buffers and a copy-free TCP path | |
| Research first, then decide | Options brought back at an owner stop | |

**User's choice:** No copy above the transport

### Decrypting a received page after a re-key or teardown

| Option | Description | Selected |
|--------|-------------|----------|
| Convert before the key dies (Recommended) | Pager converts untouched pages to page-key sealing before cutover; D-20 unchanged | |
| Keep the old cipher key for pending pages | Retired key reference-counted while undecrypted pages point at it; D-20 amended | ✓ |
| Re-seal on receive | In-place transform at receipt; a cipher pass on receive | |

**User's choice:** Keep the old cipher key for pending pages

### N11, second half: a check at fault time

| Option | Description | Selected |
|--------|-------------|----------|
| Per-page tag made at receipt (Recommended) | Keyed BLAKE3 tag per page under the page MAC key; fault checks it like an evicted page | ✓ |
| Receipt check only | No second check | |
| Keep the packet MAC, recheck at fault | Re-verify the whole packet at first fault | |

**User's choice:** Per-page tag made at receipt

### How live the success criterion 2 proof is

| Option | Description | Selected |
|--------|-------------|----------|
| Host tests plus in-kernel loopback probe (Recommended) | test-9p plus a q35crypt probe over a loopback link with an injected secret | ✓ |
| Live sealed mount over virtio-net | Host-side test server, pre-shared test secret | |
| Host tests only | Host-proven only | |

**User's choice:** Host tests plus in-kernel loopback probe

### Which reads take the sealed ingress path

| Option | Description | Selected |
|--------|-------------|----------|
| Image reads at exec only (Recommended) | Text and data pages of a binary on an aead mount land as sealed frames | |
| Exec plus page-aligned read(2) | Also swaps sealed frames into the caller for whole-page aligned reads | ✓ |
| A kernel-internal API only | Used only by the loopback probe this phase | |

**User's choice:** Exec plus page-aligned read(2)

### Which user pages are sealed in the profiles

| Option | Description | Selected |
|--------|-------------|----------|
| Every user page (Recommended) | Text and data sealed at install; bss, stack, heap at eviction | ✓ |
| Private pages only | Clean text dropped and re-read | |
| Only what eviction touches | Nothing sealed up front | |

**User's choice:** Every user page

### N10: "only the shadow frame contains plaintext"

| Option | Description | Selected |
|--------|-------------|----------|
| Narrow the claim now (Recommended) | Amend docs/threat-model.md; kernel I/O buffers and the boot image outside the claim | ✓ |
| Leave it for Phase 6 | Settle with the legacy gateway | |
| Keep the claim, shrink the buffers | Zeroize kernel buffers holding user file data | |

**User's choice:** Narrow the claim now

---

## Scope and review process

### aarch64

| Option | Description | Selected |
|--------|-------------|----------|
| x86_64 only, portable core (Recommended) | Core in port/, only x86_64 wired; aarch64 gets Roaring NEON only | |
| Both, aarch64 scalar | aarch64 sealed paging with the scalar cipher | |
| Both, with NEON decrypt | aarch64 also gets a NEON decrypt path under its own restricted section | ✓ |

**User's choice:** Both, with NEON decrypt

### port/src review

| Option | Description | Selected |
|--------|-------------|----------|
| Phase-end review, as Phase 4 (Recommended) | Plans commit on green tests; cumulative diff reviewed at phase end | ✓ |
| Two reviews: mid-phase and end | A stop after the portable core is host-green | |
| Diff first, every plan | The standing rule as written | |

**User's choice:** Phase-end review, as Phase 4

### x86_64 byte-change procedure timing

| Option | Description | Selected |
|--------|-------------|----------|
| Two stops: after XSAVE, and at phase end (Recommended) | XSAVE lands first with the full procedure; the rest at phase end | ✓ |
| Once, at phase end | The Phase 4 model | |
| At every kernel-changing plan group | A stop per wave | |

**User's choice:** Two stops: after XSAVE, and at phase end

### Lint scope (N2)

| Option | Description | Selected |
|--------|-------------|----------|
| Whole crate, as an early plan (Recommended) | x86_64 opted in first; the 7 aarch64 blocks fixed in the same pass | ✓ |
| Whole crate, at the end | One closing plan | |
| New modules only | MEM-11 amended | |

**User's choice:** Whole crate, as an early plan

---

## Claude's Discretion

Profile names and settings; boot-secret source and derivation labels; page record layout and what the page tag binds; eviction policy details; XSAVE flavour; `global_asm!` versus `l.S`; the MEM-12 scratchpad structure; pinning expression; page-aligned `read(2)` rules; benchmark harness; module layout and order of work.

## Deferred Ideas

- Plan 9 swap file over 9P as an out-of-RAM backing store
- Sealed paging on by default in every boot
- True NIC placement of data pages and the `#M` ring bypass (Phase 6)
- A live sealed mount over virtio-net with a host-side test server
- The legacy gateway's bounce frames in the threat model (Phase 6)

## Not discussed

- Whether the phase should be split in the roadmap, given how much the decisions added
- The exact amended wording of the requirements, RFC and PROJECT.md rows
