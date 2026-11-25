# Milestones

## v3.0 Cross-Build Harness and Wire Foundations (Shipped: 2026-11-25)

**Phases completed:** 4 phases, 78 plans, 180 tasks
**Git range:** `e61d7eb`..`bb68d98` — 585 commits, 777 files changed (+271,767 / −666)
**Timeline:** 4 days (2026-11-22 → 2026-11-25)
**Requirements:** 38/38 complete (HARN-01..08, DSGN-01..05, CRYP-01..04, DS-01..07, LINT-01, WIRE-01..13)
**Verification:** all four phases `passed`; UAT 112/112 on Phase 4 and 3/3 on Phases 2 and 3; audit `tech_debt` (no blockers)

**What shipped:**

- **The golden invariant is an r9-owned gate.** `cargo xtask gate` boots r9 to rio on QEMU q35 from a pre-built, read-only Harvey tree, with per-architecture fallback manifests, two-way drift and Rust-provenance checks, a one-entry swap proven at flip and flip-back, and every composed gate moved from Harvey's flake into r9's. Harvey is locked at `c697666` and has no `r9` input.
- **Design lock.** `docs/threat-model.md` is committed with the compendium's boundary quoted, and all sixteen `rfork` flags have one definition in `port::rfork`, with `sys/include/rfork.h` generated from it and drift-checked before any test leg. Four crypto flag bits are reserved.
- **`crypto/` and `ds/` foundations.** HKDF-SHA256, ChaCha20, keyed BLAKE3, the key hierarchy, the TPHT shard with its Boehm seqlock and measured spill threshold, Roaring and `ClientSet`, all safe code under the per-module forbid layout and pinned by RFC vectors, independent Python KATs and Miri.
- **`9P2000.aead` end to end on the host.** A 28-byte header with the MAC over every byte before it, per-connection per-direction keys from a transcript-bound HKDF, in-band re-key with strict per-direction cutover and confirm, the parameter-block-plus-4096-byte-page ciphertext body, the six `ExtMask` bits with the stateless-capability TLV, and the hybrid `FidHandle`/`FidEntry` on TPHT with the failover sweep.
- **The plain path is untouched.** A plain offer still gets the unchanged `Mnt::negotiate`; the owner's x86_64 Linux host boots to rio under KVM, native TCG and the full gate set at `0dd1037`, and `main` is published at `bb68d98`.
- **Review discipline held.** The Phase 4 code review found 3 critical / 9 warning / 7 info; all twelve blocking findings were fixed and pinned, the closing evidence was re-taken from a from-nothing build, and the second owner stop returned eleven PASS verdicts.

**Key accomplishments:**

1. Cross-build harness and flake ownership flip (HARN-01..08)
2. Design lock, threat model and single-home `rfork` flags (DSGN-01..05)
3. `crypto/` and `ds/` foundations with RFC pins, Miri and lint contracts (CRYP, DS, LINT)
4. `9P2000.aead` transport, FidTable and stateless capabilities (WIRE-01..13)
5. Review-driven gap closure with re-taken kernel and gate evidence

**Known verification overrides:** phases 1-3 verification reports read `stale` at close because later phases changed shared files they covered (`flake.nix`, `xtask/src/lib.rs`, `xtask/src/main.rs`, `x86_64/src/syscall.rs`); each report passed against its own phase state. One artifact newly acknowledged (the Phase 3 repo-wide `cargo fmt` note; see STATE.md Deferred Items). No requirement override.

**Known gaps / tech debt (carried to the v3.1 backlog):**

- `cargo xtask test-9p` and `cargo test -p xtask` are not invoked by CI or by `cargo xtask test` (INT-03/INT-04), so the scalar-backend twins and xtask's own guards are manual-only.
- The Phase 4 `boot-iso` evidence does not support the "fixes moved the linked kernel bytes" claim: the unchanged `outputHash` built and rebuilt green. The fixes are pinned by the library tests, and the booted kernel does not select the Aead path by design (D-16/D-23) (INT-02).
- The >90% TPHT home-hit property is measured with fixture-sized banks while production defaults differ, and the Phase 4 `FidShard` uses 0 banks (INT-05).
- `04-VALIDATION.md` is `draft` (Nyquist coverage TODO: `/gsd-validate-phase 4`).
- `docs/threat-model.md:181` still frames rfork bits 18-20 as unreserved (WR-04).
- Accepted deviations: Miri latency 230-402 s against the 120 s target; pre-existing aarch64 clippy (7 undocumented unsafe blocks); Phase 1 open warnings WR-01..WR-12.

---
