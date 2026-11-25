---
gsd_state_version: "1.0"
milestone: v3.0
milestone_name: Cross-Build Harness and Wire Foundations (In Progress)
current_phase: 04
current_phase_name: Guide Phase 2 - 9P2000.aead Transport and FidTable
status: ready_to_execute
stopped_at: Phase 4 gap-closure plans 04-18 to 04-25 planned and checked; not yet executed
last_updated: "2026-11-25T04:37:14.749Z"
last_activity: 2026-11-25
last_activity_desc: Phase 4 gap closure planned — 8 plans in 5 waves, plan check passed
state_head: 2941d36b69e8b249b75724077b438fb069bf5114
progress:
  total_phases: 4
  completed_phases: 3
  total_plans: 78
  completed_plans: 70
  percent: 75
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-11-24)

**Core value:** The OS boots to rio on QEMU q35 at every commit.
**Current focus:** Phase 04 — Guide Phase 2 - 9P2000.aead Transport and FidTable

## Current Position

Phase: 04 (Guide Phase 2 - 9P2000.aead Transport and FidTable) — READY TO EXECUTE
Plan: 17 of 25
Status: 17/25 plans complete; verification `gaps_found` (101/109); gap-closure plans 04-18 to 04-25 are planned and checked, run with `/gsd-execute-phase 4 --gaps-only`
Last activity: 2026-11-25 — gap closure planned (`421bd09`, revised `2941d36`); plan check passed with 0 blockers and 0 warnings

Progress: [████████░░] 75%

## Performance Metrics

**Velocity:**
- Total plans completed: 53

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 34 | - | - |
| 02 | 6 | - | - |
| 03 | 13 | - | - |

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P03 | 3 min | 2 tasks | 8 files |
| Phase 01 P04 | 6 min | 3 tasks | 17 files |
| Phase 01 P05 | 5 min | 2 tasks | 8 files |
| Phase 01 P01 | 13 min | 3 tasks | 8 files |
| Phase 01 P02 | 9min | 3 tasks | 15 files |
| Phase 01 P06 | 80min | 3 tasks | 19 files |
| Phase 01 P07 | 17min | 3 tasks | 5 files |
| Phase 01 P08 | 4min | 2 tasks | 8 files |
| Phase 01 P09 | 4min | 3 tasks | 7 files |
| Phase 02 P01 | 7 min | 4 tasks | 10 files |
| Phase 02 P03 | 5 min | 3 tasks | 5 files |
| Phase 02 P04 | 2 min | 3 tasks | 6 files |
| Phase 02 P05 | 10 min | 3 tasks | 4 files |
| Phase 02 P06 | 138 min | 3 tasks | 3 files |
| Phase 03 P01 | 2 min | 2 tasks | 11 files |
| Phase 03 P12 | 2min | 2 tasks | 12 files |
| Phase 03 P02 | 3min | 2 tasks | 7 files |
| Phase 03 P03 | 2 min | 2 tasks | 7 files |
| Phase 03 P04 | 2min | 3 tasks | 7 files |
| Phase 03 P05 | 4 min | 3 tasks | 4 files |
| Phase 03 P06 | 25 | 2 tasks | 5 files |
| Phase 03 P13 | 13 min | 1 tasks | 2 files |
| Phase 03 P07 | 32 min | 2 tasks | 2 files |
| Phase 03 P08 | 51 min | 2 tasks | 5 files |
| Phase 03 P09 | 1 min | 2 tasks | 5 files |
| Phase 03 P10 | 15 min | 3 tasks | 6 files |
| Phase 03 P11 | 92 min | 3 tasks | 14 files |
| Phase 04 P14 | 23 min | 3 tasks | 8 files |
| Phase 04 P15 | 1h 6m | 3 tasks | 2 files |

## Accumulated Context

### Decisions

Full log is the PROJECT.md Key Decisions table. Recent:

- [Owner 2026-11-25] Phase 4 verification `gaps_found` (101/109) and the owner chose to STOP AND DEFER the fixes: the three re-key criticals (CR-01 un-zeroized exchange secret, CR-02 no exchange traffic discriminator, CR-03 padded transcript contradicting DECISION P2) and the five blocking warnings stay `open` in `04-REVIEW-DISPOSITION.md`. Phase 4 is not marked complete; resume with `/gsd-plan-phase 4 --gaps` then `/gsd-execute-phase 4 --gaps-only`, and expect a second owner stop because the fixes change kernel bytes.

- [Owner 2026-11-22] `RFNOLEGACY` is a fourth `rfork` flag, `1 << 21`, assigned in guide Phase 0 (GSD Phase 2) with `RFCKEY` 18, `RFROBASE` 19, `RFRANDMEM` 20. Resolves warning W11; the guide's "3 flag bits" wording is corrected to four. Reserved in r9 only, in one `port/` module (`port/src/rfork.rs`) that both arch crates import (owner amendment 2026-11-23, Phase 2 CONTEXT D-17); Harvey is not edited. Recorded deviation from the guide's "reserve bits 18-20 in `libc.h`". Phase 6 implements the behaviour.
- [Owner 2026-11-22] The r9 nix flake supplies the read-only Harvey tree to `cargo xtask` through the `nix develop` devshell (HARN-07, Phase 1); xtask keeps no built-in default path.
- [Owner 2026-11-22] Phase 1 flips flake ownership (HARN-08): r9's flake imports Harvey and produces the composed outputs Harvey's flake owns today; Harvey's flake stops importing r9. Harvey's `flake.nix` is edited for this; the Harvey commit and pushes need owner approval.
- [Owner 2026-11-22] Phase 1 is planned in two passes (CONTEXT D-22). Pass 1 (plans 01-01 to 01-07) covers spikes S1 to S6 and an owner checkpoint only. Coverage gates overridden for pass 1 on that basis: HARN-05 and decisions D-02, D-06, D-07, D-09, D-10, D-12, D-14, D-15 are not yet in any plan, and no HARN requirement is implemented until pass 2. Re-run `/gsd-plan-phase 1` after the spike results to write pass 2.
- [2026-11-23] Phase 1 planning pass 2 written: plans 01-08 to 01-31 (waves 4 to 17) implement HARN-01 to HARN-08. Coverage gates now pass without override (decisions 29/29). Four owner stops: A (01-12, register the lima builder and re-run the rio leg on KVM), B (01-22, KVM witness), C (01-27, full gate set, Harvey diff approval, Harvey commit and push), E (01-28, lock-only pin bump).
- [Roadmap] Phase order is the owner's migration plan; Stage P0 is Phase 1; "Milestone S" and "Milestone STD" are source labels for Phases 8 and 9.
- [Ingest] Precedence IG > AC > RFC > PT. Phase 2 (2026-11-23): the status of IG-D01 to IG-D13 is recorded in PROJECT.md Key Decisions; IG-D09, IG-D11 and IG-D13 are not locked and are deferred to Phases 3, 4 and 6.
- [Phase 01]: S5 kernel provenance: the Mac copy of Harvey's 9k10cpu.elf has sha256 ecdcdad... and an unknown source commit; the KVM witness pins the sha256, not a commit.
- [Phase 01]: S5 harness: one boot-q35.sh serves the Mac TCG matrix and the owner's KVM witness; KVM without a writable /dev/kvm exits S5-ERROR kvm-unavailable, never a silent TCG downgrade (D-13).
- [Phase 01]: S5 D-20 minimum scope: the evidence supports a Harvey app that builds and boots to the root is from prompt; an rc prompt via Harvey's own u9fs on TCP 564 is the next increment.
- [Phase 01]: S5 pc baseline reproduces the recorded 2026-11-05 behaviour, not the exact artifact; the kernel was rebuilt on 2026-11-18 and no string proves its commit.
- [Phase 01]: S6 D-17 target is `ps`, not D-17's prose example `echo`: ps is staged in every chain-family x86_64 image, absent from the boot path, and has two gate sites; `echo` is not an rc builtin (`sys/src/cmd/rc/plan9.c:35-48`), so every bare echo in cpurc and the smoke gates would run the replacement.
- [Phase 01]: S6 arm64 swap proven end to end under HVF: control, probe marker and flip-back all PASS, and the flip-back bootfs.paq is byte-identical to control (`746f1dfe...`). The x86_64 KVM leg is scripted and statically checked but PENDING-OWNER for Plan 06.
- [Phase 01]: S6's aarch64 bakes needed a scratch-only mkpaqfs shim (pin mtimes, header time and trailer SHA1) because Harvey's aarch64 mkimage has no KERNDATE pin and the prebuilt darwin mkpaqfs predates it; Harvey stayed untouched.
- [Phase 01]: S3 rio mode waits on the serial console rc prompt + a rio budget, not a drawterm shell prompt: the cpu profile ends in `exec rio`, so rc never prints one and rio writes nothing to the pty; the driver then types `ls /srv` on the serial console.
- [Phase 01]: S3 dials as glenda (S3_DT_USER), not the creds-file probe user norbert: only /usr/glenda/lib/profile is staged, rcmain sources a profile only for a readable $home/lib/profile, and the DEMO domain shares one password so the parsed password authenticates glenda.
- [Phase 01]: The S3 flake re-exports Harvey's locked r9 116bcc2 packages and resolves them at run time with `nix build path:$S3_FLAKE#<attr>`: r9's kernel derivation has an import-from-derivation (rust-src Cargo.lock) that cannot build on aarch64-darwin, so eval-time interpolation would fail the plan's Mac check.
- [Phase 01]: determinism.sh refuses a first bake whose log records a KERNDATE other than the requested one; Plan 06 passes KERNDATE=1795382101 to both gate runs so an all-DIFFERENT result cannot be blamed on the clock.
- [Phase 01]: S3 static verdicts: A5 PASS (kernel-wide static srv table at devsrv.rs:59; #s bound at /srv for init's console rc and the cpu session) and A6 PASS (drawterm.nix builds Make.unix GUI=x11); the three KVM legs and A12 stay PENDING-OWNER for Plan 06.
- [Phase 01]: S1 venue: the owner chose the existing lima VM harvey; unshare -Ur -m is refused on it (nested uid_map EPERM inside the nix sandbox, Ubuntu 24.04 AppArmor userns restriction outside it) and the same guest script passes through proot, so A1 is REFUTED for unshare and D-03 stays viable on the proot venue.
- [Phase 01]: S1 host toolchain: nix/setup.sh + nix/toolchain.sh port to a sandbox with four adjustments - store-bash shebang for the 9l shim, pkgs.ed (mkcname), plan9port last on PATH (its 9p mount must not shadow util-linux mount), and GNU sed shadowing for 7l/BUILD's malformed _SYSRM line at the pin.
- [Phase 01]: S1 measurement: the amd64 libc tier builds in 2 s inside the sandbox (259 members, 601170 bytes) and nix store verify --no-trust is clean on the output; A8's tens-of-minutes belongs to the full tier build, not the library tier.
- [Phase 01]: S1 flake: pins Harvey bd7a698 (flake = false) and r9's nixpkgs rev 1c89083e, exposes s1-nsprobe, s1-nsprobe-proot, s1-hostbin and s1-libc-amd64 for x86_64-linux and aarch64-linux; the x86_64 witness legs stay PENDING-OWNER for Plan 06.
- [Phase 01]: S2: both source modes preserve the 23 case-colliding paths; the pass-2 tree derivation should fetch Harvey on the builder through a fixed-output fetch (rev + SRI) so the source never round-trips through APFS
- [Phase 01]: S2 copy-back: the Mac system store refuses nix copy --from as the owner's user (require-sigs=true, trusted-users=root); the case mechanism itself is sound (same-volume chroot store verify clean, 23/23 blobs), so pass 2 needs a remote-builder config or an owner-approved store-policy change
- [Phase 01]: A9: a context-discarded outPath in the darwin devshell is lazy (no build sent) and equals the later build path, so HARN-07's supply can meet D-05
- [Phase 01]: S4: the amd64 tier 0 (27 libs, 6 s) and tier 1 (40 cmds, 19 s, all S_MAGIC 00008a97) build on the aarch64-linux builder with two venue ports: /rc/bin on PATH for membername and an explicit INCLUDE list for a latent 6c heap-layout failure under proot; A3/A4 CONFIRMED, A8 REFUTED
- [Phase 01]: On APFS a bare [ -f ] lookup resolves the case twin; Mac-side re-checks must enumerate exact names, and tree outputs read by xtask must not contain colliding names
- [Phase 01]: Plan 01-06 witness verdicts: PASS s1 rung2 (proot libc 601170 B/259 members), s3 control, s5 q35 KVM (kernel ecdcdad matched), s6 x86_64 (control/probe/flipback sha-equal=yes; probe db94057f matched); FAIL s1 rung1 (unshare /nix/store bind, exit 32), s3 rio (headless drawterm never authenticated; password-feed timeout, no S3-RIO-SERVING; the tr-ok line was the owner's manual graphical attempt, corrected 2026-11-23), s3 determinism (bake2 aborted, hsecstored missing).
- [Phase 01]: A1 scoped: unshare is REFUTED on both the lima aarch64 builder and the x86_64 witness, but proot rung 2 PASSes on both; pass 2's tree derivation must use proot and not assume unshare.
- [Phase 01]: A12 REFUTED: the second mkimage bake aborts at the auth bake (hsecstored missing from the computed auth-suite set), so no image reached a determinism comparison; resolve the auth-suite resolver before planning byte-diff tests.
- [Phase 01]: A5/A7/A11 CONFIRMED (A6 downgraded to UNTESTED on 2026-11-23): the -G control's serial ls /srv shows no rio entry, the headless drawterm starts under Xvfb but never authenticated (the generated DEMO password was unknown; the tr-ok line was the owner's manual graphical attempt), the amd64 a.out probe prints R9-RUST-AOUT-PROBE-OK, and Harvey's kernel reaches the q35 KVM prompt with no signature.
- [Phase 01]: Gate teardown finding: drawterm PID 88800 survived the rio-gate timeout; ps shows the store path, not the r9s3riogate-dt-* argv0 tag, so pass 2 must reap by scratch/dial identity or the pty child pid.
- [Phase 01]: HARN-01/04/07/08 stay pending in REQUIREMENTS.md: pass 1 is spikes only under the D-22 override; the 01-06 summary lists them for traceability but mark-complete was not run.
- [Phase 01]: D-04: document both Mac builder mechanisms with lima primary. Lima is the only measured venue for case handling and amd64 tiers and needs no third-party login; the native Determinate builder is the documented alternative.
- [Phase 01]: D-03: proot for the pass-2 tree derivations, with no unshare or privileged-builder dependency; both measured builders refused unshare.
- [Phase 01]: D-20: option A, build and boot Harvey's own kernel to the root-is-from prompt; the rc prompt through u9fs is the next increment. D-17: ps with the S6 recipe. D-16: member lists now, byte differentials after the hsecstored resolver. Decision 7: keep the spike builder for pass 2; no revert.
- [Phase 01]: D-11: keep the two /srv/rio.* and /srv/riowctl.* entries as the marker. Owner correction 2026-11-23: the tr-ok line was the owner's manual graphical drawterm attempt, not the gate; the headless drawterm never authenticated because the generated DEMO password was unknown. s3-rio stays FAIL; next action is the DEMO password/credential path, headless auth and feed, then re-run with control-noG unchanged.
- [Phase 01]: A6 is UNTESTED, not CONFIRMED: the X11 backend is statically proven and the headless drawterm starts, but no headless login completed (generated DEMO password unknown). Pass 1 is closed with PASS-2 READY: yes; re-run /gsd-plan-phase 1 for pass 2.
- [Phase 01]: Plan 01-08: the aarch64 smoke initrd is assembled by r9 from xtask/manifests/arm64/smoke.toml through the stage-and-bake engine; virtexec boots it green under HVF and the D-29 member list is byte-identical to the pre-D-16 assembler's.
- [Phase 01]: Plan 01-08: manifest modes follow the staged binaries (0775, hello 0755) rather than a uniform 0755, because the member list records mode and D-29 requires an empty diff.
- [Phase 01]: Plan 01-08: R9_MKPAQFS is an explicit input with no derived default; mkpaqfs is not part of the Harvey TREE contract.
- [Phase 01]: Plan 01-08: all seven Harvey-tree consumer sites resolve through harvey::harvey_dir(arch); uregcheck's zero-argument resolver is deleted and an include_str! source invariant keeps it that way.
- [Phase 01]: Plan 01-09: the DEMO credential path is the bake-time p9sk1 generator's out/auth-creds.txt (user=norbert, the shared DEMO password also authenticates glenda); drawterm's X11 prompt reads /dev/cons -> kbdq fed only by X key events, never the pty (the host-stdin reader exists only with -G), so the pass-1 dt-password timeout had both a pty-path and an unknown-password cause
- [Phase 01]: Plan 01-09: rio mode feeds the password through XTEST (xdotool) on the private Xvfb display, locating the window by WM class because drawterm never sets _NET_WM_PID; the password travels only through a 0600 scratch file whose path is exported, and a preloaded factotum key is the documented fallback if XTEST fails on the witness
- [Phase 01]: Plan 01-09: teardown records qemu/drawterm exp_pid and Xvfb $! and sweeps /proc/<pid>/{cmdline,environ} for the run identity; the argv0 tag is never the only key (pass-1 PID 88800 showed the store path). The reusable witness driver runs rio, control and a13 at stop A (01-12)
- [Phase 02]: `port::rfork` is the single home of all sixteen `rfork` flags; `sys/include/rfork.h` is generated from it by `cargo xtask rfork-header` and drift-checked; both arch `RFORK_SUPPORTED` masks stay `0x3c7f`; the owner approved the `port/src` patch as shown and the committed change is byte-equal to it
- [Phase 02]: the x86_64 kernel changed by panic line numbers only: six `.text`/`.text.boot` verdicts are IDENTICAL on four configs; the owner re-derived the `boot-iso` `outputHash` (`sha256-x07uJKM8Y7j3uXNwqT8Txhd8zhkRV8YrBkKyT2sg490=`) and all nine owner-stop verdicts PASS at gated commit `4624ea2`; `origin/main` is `3d01f07`, branch `phase-02/design-lock` is `f8d077d`
- [Phase 02]: three of the guide's thirteen decisions are deferred (IG-D09 Phase 3, IG-D11 Phase 4, IG-D13 Phase 6); the ten locked/amended decisions are owner-only change and are recorded in PROJECT.md Key Decisions
- [Phase 02]: code review recorded 0 Critical / 4 Warning / 4 Info; WR-01 (xtask tests not in `cargo xtask test`/CI), WR-02 (narrow one-definition scan) and WR-04 (threat-model section 7 wording) are open hardening follow-ups; WR-03 (no include guard) is skipped by design
- [Phase 03]: Plan 03-01: crypto/ joins the workspace with six exact pins (sha2 0.11.1, hmac 0.13.0, hkdf 0.13.0, chacha20 0.10.2, blake3 1.8.7, zeroize 1.9.2), all default-features = false and each with a provenance comment; Cargo.lock gained 176 lines and removed none.
- [Phase 03]: Plan 03-01: hmac_sha256 wraps hmac::Hmac<sha2::Sha256> with no upstream type in a public signature (D-18); the RFC 4231 section 4.2 case 1 pin passes on the host default backend and on the scalar backend the kernels select.
- [Phase 03]: Plan 03-01: the scalar-backend test leg carries its rustflags in its own argument vector; no --target-dir was needed (both variants coexist and the second run recompiled nothing). Both kernel ELF hashes are unchanged, so D-21's stop did not trigger.
- [Phase 03]: Plan 03-12: ds/ joins the workspace with the D-09 per-module forbid layout (tpht exempt; roaring, setops, clientset forbidden) and roaring =0.11.5 as its only external crate, under [dev-dependencies] with a provenance comment
- [Phase 03]: Plan 03-12: the two lib/*-unknown-none-elf.json cross legs with -Z build-std=core,alloc are the no_std FP/SIMD-free proof; the built-in bare-metal legs are kept only to meet D-21's wording and the code says the built-in aarch64 target proves nothing
- [Phase 03]: Plan 03-12: Cargo.lock gained 13 lines and removed none (ds and roaring only) and now has its final Phase 3 shape; both kernels are byte-identical (@03-12 IDENTICAL), so D-21's stop did not fire
- [Phase 03]: Plan 03-02: the static lint contract is one xtask module (six rule groups, plain text scans) called in TestStep::run directly after rforkh::check; a violation stops cargo xtask test with a lint-contract: error naming file and line before any leg compiles
- [Phase 03]: Plan 03-02: the unsafe token rule strips // comments and matches whole words so unsafe_code never fires; the DS-07 Funnel absence and the //! restriction note in ds/src/lib.rs are pinned by 12 fixture tests
- [Phase 03]: Plan 03-02: Miri comes from toolchainMiri in flake.nix (the four pinned components plus miri) exported as R9_MIRI_TOOLCHAIN; rust-toolchain.toml and the devshell packages list are untouched, so the kernel toolchain store path does not move
- [Phase 03]: Plan 03-02: cargo xtask miri runs <toolchain>/bin/cargo miri test --package ds --lib tpht with many seeds and refuses on zero tests; the devshell appends the Miri bin to PATH (append-only deviation) so cargo-miri resolves by absolute path
- [Phase 03]: Plan 03-03: hkdf_extract and hkdf_expand wrap hkdf::Hkdf over Sha256 behind crypto::kdf; the RFC 5869 255 x 32 length rule is checked before the output is touched and no fallback derivation exists (CRYP-01)
- [Phase 03]: Plan 03-03: chacha20_xor restates the pinned crate's remaining-blocks rule in r9 code before writing a byte, so block index u32::MAX is never produced; CHACHA20_MAX_BLOCKS is 2^32 - 1 blocks (274,877,906,880 bytes from counter 0), one block fewer than RFC 8439 (CRYP-02, T-03-09)
- [Phase 03]: Plan 03-03: the crypto::page module doc states the per-key-and-nonce limit and the caller's explicit nonce/counter duty; the 8-byte wire nonce to 12-byte ChaCha20 nonce rule stays Phase 4's (D-06)
- [Phase 03]: Plan 03-03: RFC 4231 cases 1 to 7 and RFC 5869 cases 1 to 3 pass on the default and scalar-backend legs; RFC 8439 2.3.2 and 2.4.2 pass on both legs, with the boundary measured into evidence/PINS-03-03.md
- [Phase 03]: verify_keyed takes 1..=32-byte tags: 32-byte tags compare through blake3::Hash's constant-time equality; shorter tags fold every compared byte with no early exit and test the accumulator once
- [Phase 03]: Blake3 wraps blake3::Hasher in a private field, so no public item in crypto::mac names an upstream type (D-18)
- [Phase 03]: k_file builds a fixed 12-byte info (Qid.path 8 bytes little-endian, then Qid.vers 4 bytes little-endian) and is pinned by an independently computed Python hmac/hashlib KAT
- [Phase 03]: derive_labelled uses Hkdf::expand_multi_info(&[label, context]) because crypto/ is no_std with no allocator; the pinned ("ab","c")==("a","bc") test makes the unframed concatenation observable
- [Phase 03]: KeyCapability's read-only guarantee is type-level: keyed BLAKE3 is symmetric, so the guarantee is no seal method, private fields and no key accessor, proven by a compile_fail doctest with a passing twin
- [Phase 03]: read_only() keeps the parent's k_dec (IG-P01) and sets blake3_verify from k_mac, keeping cipher and MAC keys separate (T-03-18)
- [Phase 03]: A run container stores (start, length) covering start..=start+length, sorted, non-overlapping and non-adjacent, as in Lemire et al.
- [Phase 03]: Container conversion happens only at the four fixed sites (array to bitmap past ARRAY_MAX, bitmap to array at ARRAY_MAX on removal, run_optimize, insert_compact's single-container rewrite); set operations build new array/bitmap results and never convert an operand.
- [Phase 03]: smallest_form compares payload bytes and breaks ties array, then run, then bitmap, so conversions are deterministic and a 4096-cardinality set stays an array.
- [Phase 03]: Set-op dispatch: bitmap|bitmap through the setops kernels, array|array intersection through array_intersect, array union/difference by two-pointer merges, and mixed pairings by probe/paint so sparse chunk sets avoid 8 KiB scratch copies per chunk.
- [Phase 03]: zero_words builds the boxed [u64; 1024] through Vec::into_boxed_slice and try_into, avoiding an 8 KiB stack temporary in the crate Phase 5 links into the kernel.
- [Phase 03]: RoaringBitmap PartialEq compares contents and Debug prints only the length, so equal values in different container kinds are equal and a failing assert stays readable.
- [Phase 03]: Claude's discretion: insert_compact rewrites the touched container in its smallest form after every insert; one container (at most 8 KiB) scanned per insert, accepted because the lease path reads far more often than it changes (T-03-21, revisit in Phase 6).
- [Phase 03]: Claude's discretion: Roaring does not downgrade when ids are removed; a cohort that outgrew four clients stays a Roaring set, documented and pinned by removal_rules.
- [Phase 03]: The ClientSet no-heap property is proved by variant and size_of <= 24, not by a counting allocator, which would need a third restricted-feature module outside ds/src/tpht/ (D-09).
- [Phase 03]: Plan 03-06: the TPHT home block index is the low log2(num_blocks) bits of the mixed half L' ((key >> 32) ^ mix32(key as u32)), not P's low bits as compendium 4.1 words it; the test-only split_unmixed keeps the compendium reading for plan 03-08's evidence (RESEARCH C4, outside the IG-D09 lock)
- [Phase 03]: Plan 03-06: mix32 is MurmurHash3's 32-bit finalizer pinned by independent Python and Node known answers; the header's quotients field holds a 16-bit fingerprint of the remainder and a caller closure verifies candidates against the full key (DS-02 at the function level)
- [Phase 03]: Plan 03-06: the 64-byte bucket header is all atomics with the Boehm seqlock (writer Release data stores and generation bump; reader Acquire seq load, Relaxed data loads, fence(Acquire), seq reload); listing 9.3's orderings and plain fields are not used; Miri over 64 seeds passes and ds/src/tpht/ still holds no unsafe site
- [Phase 03]: Plan 03-06: Miri-side test sizes were halved once per task under the plan's 120-second latency rule (feistel 142 -> 73 s, tpht-shard 121 -> 104 s); native sizes, the 64 seeds and the test list are unchanged
- [Phase 03]: Plan 03-06: DS-01 is only part-delivered here (structure and concurrency); its over-90% home-hit measurement stays plan 03-08's, so only DS-02 is marked complete (03-08's frontmatter also claims DS-01)
- [Phase 03]: Plan 03-13: deref_tiny resolves a (key, tiny_ptr) pair from one arena slot read, with zero header reads and no seqlock; the counting probe asserts the exact pair (arena_reads, header_reads) == (1, 0) and lookup afterwards proves the probe is wired
- [Phase 03]: Plan 03-13: deref_tiny's unchecked arena read (ds/src/tpht/flattened.rs:258, get_unchecked) is the only unsafe site in ds/src/tpht/, sound because slot_mask + 1 = HOME_SLOTS * num_blocks slots are fixed in the constructor; its SAFETY comment names that invariant for plan 03-10's audit (D-12, D-13)
- [Phase 03]: Plan 03-13: the Miri-side model sizes were halved once under the 120-second latency rule (blocks 16 -> 8, operations 200 -> 100; first warm run 123 s, resized run 116 s); native sizes (1,024 blocks, 2,000 operations), the 64 seeds and the test list are unchanged, and the counting and wrap tests keep the size the property needs
- [Phase 03]: Plan 03-07: ChainedTphtOverflow is all-atomic (state, fingerprint, full key, value words, next link): a claim is a compare-exchange free to WRITING (0xFF), the payload is stored Relaxed, the state goes LIVE with Release, then the node is linked at the block head with Release; a reader treats a non-LIVE node as absent. ds/src/tpht/chained.rs holds no unsafe and does not copy listing 9.4.
- [Phase 03]: Plan 03-07: a chain link is (candidate choice << 8) | node index with 0xFFFF as the end marker; the 255 bank indices are 0..=254, so no link or claimed index is 0xFF. fill() is used/(num_banks * 255) and the 8-seed pin measured 0.9920..0.9966 before the first BanksFull.
- [Phase 03]: Plan 03-07: Miri-side sizes were taken to the plan's floors and the 22-test tpht suite still took 230 s (a second warm run 235 s) against the 120 s budget; MIRI-OVER chained 230 is recorded and reported to the orchestrator, with native sizes, the 64 seeds and the test list unchanged (cutting seeds or tests is not the executor's call).
- [Orchestrator 2026-11-24]: Plan 03-07 Miri over-budget deviation ACCEPTED — the run is green over all 22 tests × 64 seeds with no Undefined Behavior or data race; 230 s exceeds the 120 s feedback-latency budget only. Coverage, seed count and native sizes stay unchanged. Revisit only if `cargo xtask miri` latency becomes a commit-workflow blocker; no plan edit authorized.
- [Phase 03]: Plan 03-08: SPILL_THRESHOLD = 16 at HIT_RATE_OCCUPANCY = 1.00, chosen by the rule committed before the measurement (rule b479d28, tables 4150faf); at occupancy 1.00 T=16 is the only threshold over 0.90 for every distribution and seed, and the weakest of the fifteen runs is sequential 236,035/262,144 = 0.900402 (MARGIN)
- [Phase 03]: Plan 03-08: the mixed-half index is what carries structured keys: at the chosen cell pagekey is 0.9036 mixed versus 0.2500 with the compendium's unmixed low bits of P, and upper32/upper32_or7 are 1.0000 versus about 0.0001; plan 03-09 records the correction (RESEARCH C4, outside the IG-D09 lock)
- [Phase 03]: Plan 03-08: the shard spills a block's entries past the threshold into ChainedTphtOverflow under that block's seqlock; insert returns Ok(Some(ptr)) at home and Ok(None) in a bank, lookup returns Found { value, home }, and a spilled entry has no tiny pointer (reached by lookup only, never deref_tiny)
- [Phase 03]: Plan 03-08: with_banks(n, 0) keeps plan 03-06's 16-slot home-only behaviour and HomeFull; a shard with banks never returns HomeFull (BanksFull instead). The default overflow is one bank per 64 blocks, minimum 2; the measurement and pin pass banks_for explicitly
- [Phase 03]: Plan 03-08: the six tests advanced by plans 03-06 and 03-13 were re-pinned on a home-only shard; only constructors, insert/lookup unwrapping and two placement assertions changed, each named by a REPIN ledger line
- [Phase 03]: Plan 03-08: Miri is over the 120-second budget and reported, not fixed by cutting coverage: 981 s then 315/329 s at the floors for tpht-spill, 628 s then 402 s for tpht-hit-rate; seeds, tests and native sizes unchanged (plan 03-07 precedent)
- [Orchestrator 2026-11-24]: Plan 03-08 Miri over-budget deviation ACCEPTED — both runs are rc 0 over 64 seeds with no Undefined Behavior or data race; only feedback latency is out of budget. Coverage, seeds and native sizes stay unchanged. Carry to phase-end review: the 120 s budget in 03-VALIDATION.md is not met by tpht-spill/tpht-hit-rate, so a later phase should recalibrate it or split the Miri step; no plan edit authorized now.
- [Phase 03]: Plan 03-09: PROJECT.md records W3, N1, W5, W6 and N2 as settled and IG-D09 as locked amended 2026-11-24 (succinct TPHT primary, Funnel scratchpad only, ds/src/tpht/ unsafe-holding, 16 home slots, spill threshold outside the lock); the Open Questions table is down to W1, W2, W4, W7-W10.
- [Phase 03]: Plan 03-09: the RFC text is amended in section 3.1 (ChaCha20 with the RFC 8439 32-bit counter and 96-bit nonce; Keyed BLAKE3 the only MAC; [RFC8439] cited for ChaCha20 only) with a dated note; the guide and compendium are byte-identical to baseline 4beb5c8.
- [Owner 2026-11-24]: Phase 3 verification complete. The owner accepted the nine planning-level rows, `zeroize =1.9.2`, the ChaCha20 2^32-1 block limit, the DS-07 no-Funnel backstop, and the Miri latency deviation (230-402 s at 64 seeds, coverage unchanged). UAT 3/3 passed; `03-VERIFICATION.md` canonicalized to passed.
- [Phase 03]: Plan 03-09: nine planning-level rows are recorded for owner review at Phase 3 verification (compendium 4.1 mixed-half correction, type-level ReadOnly, MurmurHash3 f(R), K_file info, parallel chain heads, Boehm seqlock, build/gate readings, ChaCha20 per-key limit, zeroize =1.9.2 with =1.9.1 as the older alternative).
- [Phase 03]: Plan 03-09: STATE.md records the pre-existing aarch64 clippy failure (7 undocumented unsafe blocks) and the TPHT neighbourhood-displacement gap as deferred items.
- [Phase 03]: Plan 03-10: UNSAFE-AUDIT.md names the four gated crates (aarch64, port, crypto, ds), records ds 1 lint-visible block (ds/src/tpht/flattened.rs:440, deref_tiny's unchecked arena read) and crypto 0, so SAFETY is 152 -> 153 (129 + 23 + 1 + 0); the aarch64 count of 129 is kept with the note that 7 pre-existing undocumented sites may make it stale (RESEARCH C5, outside Phase 3)
- [Phase 03]: Plan 03-10: the clippy probe is evidence, not an assertion — deleting the six-line SAFETY comment above the arena read made the default 'nix develop -c cargo xtask clippy' exit 1 with 'unsafe block missing a safety comment' at flattened.rs:434; reverted with git checkout -- ds/src/tpht and 'git status --porcelain -- ds' printed nothing, so no probe reached a commit
- [Phase 03]: Plan 03-10: the third-party dependency record gives one row per crate Cargo.lock shows reachable from crypto/ — the six direct pins with their exact feature sets and 17 transitive crates — with raw grep -ow unsafe counts over upstream src/ labelled as including backends r9 never selects; F10's figures are reproduced where F10 lists the crate and the five F10 omits (inout 31, libc 810, cc 36, shlex 7, find-msvc-tools 55) were counted the same way
- [Phase 03]: Plan 03-10: both suite runs are green at 1be121c (the phase's last code commit) — suite-run-1 and suite-run-2 each have five exit codes 0, 30 'test result: ok' lines, the RFC 4231/5869/8439, BLAKE3 and keys pins on both the default and the soft-backend crypto legs, the doctest pair, and the ds leg (roaring:: 6, clientset:: 5, setops:: 2, tpht:: 30 with the D-15 generator ignored)
- [Phase 03]: Plan 03-10: both kernels are byte-identical to the baseline at the end of the phase on one venue after a from-nothing rebuild — virtshell 9218b998...fa083e, q35churnshell c9dcef0b...8b78b (@03-10 IDENTICAL), a supplementary cmp against target/phase03-kernel-bytes/before/ also equal, and the protected-path diff against 4beb5c8 quiet; D-21's stop did not fire
- [Phase 03]: Plan 03-10: Miri is over the 120-second budget at the end of the phase — MIRI-TIME closing-run-1 350 and closing-run-2 389 for 64 seeds over 29 tpht tests; the plan forbids source edits so nothing was resized, both runs exit 0 with no Undefined Behavior or Data race, and the number is carried to the phase-end review alongside the 03-07/03-08 overages
- [Phase 03]: Plan 03-10: 03-OWNER-STOP.md is the ordered x86_64 Linux page plan 03-11 consumes — branch push approval, fetch, the two sandboxed kernel builds at the tip, the same two at the base commit in a second worktree (so their HEAD= lines carry the base), the four-hash comparison, boot-iso plus --rebuild, the four host suite commands and the report-back; every verdict command runs through run_logged so each log under ~/r9-p03-stop/ begins with HEAD= and ends with EXIT=
- [Phase 03]: Plan 03-10: the owner stop puts the two open choices in front of the owner without blocking — zeroize =1.9.2 with =1.9.1 as the older alternative, and the ChaCha20 per-key-and-nonce limit of 2^32 - 1 blocks (274,877,906,880 bytes); COVERAGE.md records the reasoned no-external-API declaration in one line, and nothing was pushed (origin/main unchanged, the transport branch not yet created)
- [Phase 03]: Plan 03-11: the owner stop closed 11 of 11 verdicts PASS from the returned logs - both sandboxed kernel derivations build at the tip 95fe66a with Phase 3's enlarged Cargo.lock (A8 retired), the tip and base bytes are equal on both architectures (kernel-x64 6a5f5a66...01cf6a5, kernel-virt 954e5110...30f4eee), the fixed-output boot-iso builds and rebuilds on its committed outputHash, and host-test/check/clippy/miri all exit 0 on the x86_64 host (miri 64 seeds, 29 passed each)
- [Phase 03]: Plan 03-11: main is pushed to origin at a64352625efe9cb8a924d7941c6be09c4f738a11 on the owner's second ruling (push-main), after origin/main had been 3d01f07; the VERDICT stop-main-push: PASS line was committed as 5226d11 and stays local until a later approved push, so local main is one evidence commit ahead of origin/main
- [Phase 03]: Plan 03-11: the transport branch phase-03/crypto-ds-foundations is on the remote at 5f565ec (Mac tip 95fe66a plus two owner return commits that add 12 files only under evidence/owner-stop/return/); no kernel byte changed on Linux, so the D-21 boot-iso outputHash procedure did not apply and the boot-to-rio gate keeps holding by identity with the Phase 2 gated state 4624ea2; no source file, flake.nix or outputHash was touched
- [Phase 04]: Owner approved the cumulative port/src change at f72cc2a; Task 3 measured at 3398df4 with no source diff since.
- [Phase 04]: Both whole-ELF hashes differ from the baseline; Phase 2's .text-identical rule is not used (D-29). x86_64 text 760137/2097152, 1337015 bytes headroom.
- [Phase 04]: FP/SIMD counts EQUAL both kernels (309 aarch64, 6 x86_64); Miri 348s reported as the accepted deviation.

### Open Questions (four warnings, approved to be carried, not resolved)

Settle each when its phase is planned. Detail: `.planning/INGEST-CONFLICTS.md`. Eleven notes (N1-N11) are in PROJECT.md; N9 to N11 are source contradictions the Phase 2 threat model found untracked (stale-frame mechanism, plaintext in legacy bounce frames, where the wire MAC is verified).

| ID | Settle in | Question |
|----|-----------|----------|
| W7 | Phase 6 | `DiscoveryBeaconV2` 192 versus 202 bytes |
| W8 | Phase 13 | `/adm/users` UID 0 `adm` versus persona `glenda` |
| W9 | Phase 13 | Test 3.2 needs `alice` absent from `/adm/users` |
| W10 | Phase 13 | Suite 3 cross-VLAN mounts with no route |

### Pending Todos

None yet.

### Blockers/Concerns

- **Phase 4 verification `gaps_found` (101/109), owner deferred the fixes on 2026-11-25.** The code review (`04-REVIEW.md`, 3 critical / 9 warning / 7 info) and the verifier (`04-VERIFICATION.md`) independently confirmed the same blocking gaps, all in the in-band re-key path and two fid-table guards: CR-01 exchange secret + `Rekey::out` survive `fail()` while `keys_zeroized()` returns true; CR-02 exchange traffic carries no tag/fid discriminator so ordinary `Twrite`/`Rwrite`/`Tread`/`Rread` are absorbed or close the connection; CR-03 the re-key transcript absorbs the page-padded body, contradicting the accepted DECISION P2; WR-01 re-key never returns to idle (one re-key per connection); WR-02 `SendHalf::send` ignores the ACK deadline; WR-04 a torn clunk can resolve an all-zero identity; WR-05 `sweep` can delete a live entry that reused a swept slot; WR-07 control messages use tag 0, not `NOTAG`. Partial: WR-03, WR-06, WR-08, WR-09. All 19 findings are `open` in `04-REVIEW-DISPOSITION.md`. **Any fix changes kernel bytes**, so it voids `04-14`'s byte evidence and the `boot-iso` `outputHash` commit (`sha256-5A6QdJdWz7uWyUrfUAc3jAO+ZQcG/TowydbTLoF+u9I=`) and needs a second owner stop. Resume with `/gsd-plan-phase 4 --gaps`, then `/gsd-execute-phase 4 --gaps-only`.
- **Phase 4 push state:** `origin/main` = `5d1549a` (the owner-approved push of 142 commits). Local `main` is ahead with `2bc5e09` (`stop-main-push: PASS`), `59c855a` (04-15 SUMMARY), `6ad61d3` (tracking), `e00d42d` (review disposition), `b944127` (review + verification) and the gap-deferral commit — held for a later approved push. `origin/phase-04/aead-wire` = `405984d`.
- **Phase 4 capability steps not run:** `04-SECURITY.md` does not exist (`/gsd-secure-phase 4`), the Nyquist audit for the re-key path was not run (`/gsd-validate-phase 4`), and UI review does not apply. The verify:post contract routes security as a next step rather than auto-running it.
- Unpushed work, checked against the local remote-tracking refs on 2026-11-24 after the plan 03-11 owner stop: `origin/main` was `a643526` and `git push origin main` published every unpushed commit up to it. The Phase 3 transport branch `phase-03/crypto-ds-foundations` is `5f565ec` (Mac tip `95fe66a` plus two return commits); the Phase 2 branch `phase-02/design-lock` is at `f8d077d` (gated commit `4624ea2` plus two return commits). Earlier unpushed work is published: r9 `5b8b70a` (planning documents plus `legacy/`, 53 MB) and `6ee45b5` are on `origin/main`, and Harvey `25d97dc4d` is on `origin/master`.
- Owed checks at checkpoint r9 `116bcc2`: the owner's linux `nix build .#boot-iso --rebuild` (expected `sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=`), and hosted CI on r9 `116bcc2` and Harvey `bd7a6982a`.
- Phase 20 of the Harvey-rooted work never ran `/gsd-secure-phase` (no SECURITY.md).
- Docs pin r9 at `c787ce9`; HEAD is far later, and `crypto/` and `ds/` now exist and are host-tested while `kern/`, `sys/` and `venti/` are still absent. Re-verify every landing-site path when its phase is planned.
- S6 x86_64 KVM leg: scripted and statically checked, PENDING-OWNER. Plan 06 must run `spikes/s6/linux-x86_64.sh` on the x86_64 Linux host (writable `/dev/kvm`, the S3 scratch Harvey tree, the r9 clone at `116bcc2`) before the pass-1 checkpoint.
- Mac store copy-back: nix copy --from ssh-ng://lima-harvey as the owner's user is refused (require-sigs=true, trusted-users=root). Pass 2 needs a remote-builder configuration or an owner-approved store-policy change; the case names themselves round-trip cleanly in a same-volume chroot store.

### Standing Owner Rules

- Pushes in either repo need explicit owner approval; check what is unpushed and push only the approved commits.
- Edits under r9 `port/src` need owner approval (show the diff first). `aarch64/`, `x86_64/`, `xtask/` edits do not.
- x86_64 kernel byte change: re-derive and commit the `boot-iso` `outputHash`, run the x64 TCG gates in an amd64 Docker container (clean up afterwards), and get the owner's linux boot-iso check.
- aarch64 kernel byte change: quick pass now (tier-1 on HVF and TCG; smoke, net-smoke, cpu once on HVF); full HVF series at Phase 8. Kernel builds are reproducible, so compare kernel sha256 to prove a pin move changes nothing.
- Kernels are built without FP/SIMD; kernel SIMD goes through `fpu::kernel_begin()` / `with_simd()`; user FP state is per process (save at switch, copy on fork, reset on exec); arm64 is the reference for x86_64.
- No AI attribution in commits or PRs. Never stage the 11 pre-modified troff/postscript font files in Harvey (a darwin case-collision artifact).

### Practical Notes

`gh` is not installed (`nix run nixpkgs#gh -- ...`). GNU sed is on PATH. macOS Unix socket paths cap at 104 bytes: run boot tests from a short path. The lima VM `harvey` is the Harvey namespace build venue (`core.ignorecase=false` on purpose). r9 exec caps each argument at 128 bytes; the PL011 console buffer is 256 bytes. Debug history: `.planning/legacy/debug/knowledge-base.md` (frozen).

## Deferred Items

Items carried over from the legacy-final checkpoint (HANDOFF.md):

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| Verification | Full arm64 HVF series (smoke 1753, net-smoke 36, tier-1 10, cpu 36, plus 3 TCG each; about 4.4 h) | Deferred by owner to Phase 8 (SMP-05) | legacy-final | v3.2 |
| Defect | WINDOWS #70: aarch64 scheduler resume skips note delivery | Deferred to Phase 8 (SMP-06) | legacy-final | v3.2 |
| Defect | WINDOWS #49: I-cache, root-caused and fixed | Left open until the full series runs (SMP-06) | legacy-final | v3.2 |
| Defect | WINDOWS #66: x86_64 triple fault under 8-way soak | Fix on an x86_64 host venue (SMP-06) | legacy-final | v3.2 |
| Test gap | `#!` recursion cap has no host or live test | Unscheduled; add a test when exec is next touched | legacy-final | - |
| Prerequisite | SMP: aarch64 per-CPU current frame, kernel stack, proc pointers; x86_64 per-CPU `Mach` mapping (FP ownership already per-CPU) | Phase 8 (SMP-02) | legacy-final | v3.2 |
| Lint | `cargo xtask clippy --arch aarch64 --config virtshell` fails at HEAD on 7 undocumented unsafe blocks in `aarch64/` (`platform/virt/rtc.rs:75`, `proc.rs:439`, `470`, `1823`, `2753`, `textcopy.rs:158`, `172`); `UNSAFE-AUDIT.md`'s aarch64 count of 129 may be stale | Pre-existing; found by Phase 3 research; outside Phase 3 | Phase 3 | - |
| Design | TPHT writer assigns tiny pointers 1..=16 only; compendium 4.3 neighbourhood displacement (17..=255) and clock-nibble eviction are not built | Not a Phase 3 requirement; for owner review | Phase 3 | - |

The WINDOWS ledger (51 open, 4 waived, 17 fixed, 72 total) is frozen at `.planning/legacy/WINDOWS.md`; re-home open entries to a live file before updating them.

## Session Continuity

Last session: 2026-11-25T03:11:07.960Z
Stopped at: Completed 04-15-PLAN.md
Resume file: None
