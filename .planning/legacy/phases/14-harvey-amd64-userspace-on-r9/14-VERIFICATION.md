---
phase: 14-harvey-amd64-userspace-on-r9
verified: 2026-11-09T16:47:20Z
status: passed
score: 6/6 must-haves verified
covered_files: [".planning/phases/14-harvey-amd64-userspace-on-r9/14-01-PLAN.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-01-SUMMARY.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-02-PLAN.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-02-SUMMARY.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-03-PLAN.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-03-SUMMARY.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-04-PLAN.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-04-SUMMARY.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-05-PLAN.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-05-SUMMARY.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-06-PLAN.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-06-SUMMARY.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-07-PLAN.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-07-SUMMARY.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-08-PLAN.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-08-SUMMARY.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-09-PLAN.md", ".planning/phases/14-harvey-amd64-userspace-on-r9/14-09-SUMMARY.md", "flake.lock"]
covered_digest: "v4:sha256:23ea18a4b38765eeea6933c94bd1a4a7574c290ec1bc6d25bed620211326809e"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 5/6
  gaps_closed:
    - "r9's boot-iso FOD wired hash verifies at the phase pin — 14-07 must-have \"nix build on r9's boot-iso still verifies its hash\" (was stale at the prior pin e417660; re-wired at r9@4132e25 and verified LIVE by this run)"
  gaps_remaining: []
  regressions: []
---

# Phase 14: Harvey amd64 Userspace on r9 Verification Report

**Phase Goal:** The core value lands on x86_64 — Harvey amd64 binaries exec on r9 through the re-derived ABI contract, the boot chain reaches the interactive `% ` prompt via one command, and the tier + `/net` gate machinery runs keyed to `objtype=amd64` (no new toolchain work — 6c/6a/6l already green). Work spans both repos plus the flake glue.
**Verified:** 2026-11-09T16:47:20Z
**Status:** passed
**Re-verification:** Yes — after gap closure (supersedes the 2026-11-09T16:23:25Z report; its one gap is resolved by this run)

**Dual-repo note.** Harvey at `/home/req/code/harvey` (HEAD `20b52f004`, working tree clean apart from untracked planning cache), r9 at `/home/req/code/r9` (HEAD `4132e259e12f559edda97ea7c9ffee830ad7e32f`, clean, PUSHED — `origin/main..HEAD` = 0). Harvey's `flake.lock` pins r9 at exactly `4132e259e12f559edda97ea7c9ffee830ad7e32f` = r9 HEAD (the pin advance `f6622b35f` is lock-only, its own commit — push-before-pin held).

**Re-verification mode.** The prior report verified 5/6 truths with one gap: the boot-iso FOD wired hash was stale at the then-pin e417660 (verifier-run `nix build .#boot-iso` exited 1), and 14-EVIDENCE.md's phase-close regression row claimed that build green without running it at the pin. This run verifies the gap closure LIVE at the new pin and regression-checks the five previously-verified truths. "Verifier-run" below = re-executed by this verification in its own process; "executor-retained" = capture verified on disk for existence and assertion content.

## Gap Closure (the failed truth, full 3-level verification)

**The fix, as committed:**
- r9 `4132e25` — `packages.boot-iso.outputHash` re-wired to `sha256-lwbDSqYxYNG8ZIhjkDsziHlEjFM3JJOEYinl3vbKWaw=` (git show: flake.nix ONLY, 18+/4-; the whole fix is the hash plus a provenance comment carrying the full file/NAR hashes and the standing drift rule). The tree is otherwise untouched, so all Rust/kernel substrate verified at e417660 is byte-identical at the pin.
- harvey `f6622b35f` — lock-only pin advance (3+/3- in flake.lock: rev + narHash + lastModified).
- harvey `20b52f004` — the honesty correction: 14-EVIDENCE.md's regression row now states the close itself was "**NOT run, and NOT green**" at e417660 (naming the verifier's exit-1 mismatch) and records the fix; WINDOWS.md rows 56 + new row 60 marked fixed; deferred-items.md gains the RESOLVED entry; the two-derivation proof transcript is retained at `evidence/14-gap-fod-proof.md`.

**LIVE verification of the fix (this run's own process):**

| Check | Command (from /home/req/code/r9) | Result | Status |
| ----- | ------------------------------- | ------ | ------ |
| FOD hash verifies at the pin | `nix build .#boot-iso` | exit 0 — and a FRESH chain: nix printed "these 2 derivations will be built" and built `…-r9-kernel-x64-4132e25.drv` then `…-r9-boot-iso-4132e25.drv` (not a cache substitute); the freshly built ISO passed the wired hash | ✓ PASS (verifier-run) |

The fresh-build leg is the stronger form of the proof: the kernel was recompiled from source and the resulting ISO verified against the wired NAR hash. The transcript's two-derivation byte-identity discipline (fresh-kernel clones `gapfix-proof-a`/`-b`, byte-identical ISO, NAR `9706c34a…` = the wired value three ways) is retained at `evidence/14-gap-fod-proof.md` and its arithmetic is consistent with the wired hash and this run's green build. The `--rebuild` corroboration leg claimed in the transcript was not re-run by the verifier — the fresh-build leg subsumes it (a rebuild of a FOD re-checks the same output the fresh build just verified).

**14-EVIDENCE.md row re-check (the overclaim):** the corrected row no longer overclaims — it explicitly records the close itself as NOT green at e417660 with the exact mismatch, and its remaining green claim ("`nix build .#boot-iso` AND `--rebuild` both exit 0 at the new pin") is corroborated by this run's live exit-0 build for the primary leg. The repo-SHA table now also carries the `4132e259…` row with the correct push/pin provenance (`f6622b35f`).

**Impact scope re-confirmed:** zero `boot-iso` references in Harvey's `flake.nix` + `nix/*.sh` (verifier grep = 0 hits) — no XEXEC gate consumes the wired-hash FOD, so the gap never touched the five success criteria; it is now closed on its own terms as well.

## Goal Achievement

### Observable Truths

Same six must-haves as the prior report (5 ROADMAP success criteria + the 14-07 boot-iso hash clause). Failed item: full re-verification; passed items: regression check (live re-proof where cheap, retained-capture content check otherwise).

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | **XEXEC-01** — amd64 Ureg/TrapFrame ABI contract published and build-enforced: offsets frozen, const asserts + xtask ureg-check live on every gate, negative control firing on drift | ✓ VERIFIED | REGRESSION RE-PROOFS at 4132e25 (verifier-run): `R9_HARVEY_DIR=/home/req/code/harvey nix develop . -c cargo xtask ureg-check` → exit 0, "35 fields match aarch64/include/ureg.h" + "26 fields match amd64/include/ureg.h" + "negative control fired — the mutated amd64 header copy (cs -> cseg) was rejected with a named-field diff"; gate-precondition negative check `R9_HARVEY_DIR=/nonexistent-reverify … boot-test --arch x86-64 --config q35` → exit 1 with the named ureg-check error before qemu spawns. Static substrate unchanged at the pin (4132e25 touched flake.nix only): dat.rs `size_of::<Ureg>() == 184` + `UREG_TRAPNO_OFFSET = 16*8` + `UREG_CS_OFFSET = 19*8` + 20 offset_of const-asserts; uregcheck.rs u16int ×11 / NEGATIVE CONTROL ×6; `amd64/ABI.md` 15013 bytes with the 184 markers |
| 2 | **XEXEC-02** — Harvey amd64 binaries exec on r9 x86_64 through the real EL/syscall path (staged-init + hello-equivalent execution evidence on serial) | ✓ VERIFIED | REGRESSION RE-PROOF (verifier-run at 4132e25): `nix develop . -c cargo xtask boot-test --arch x86-64 --config q35exec` → exit 0, PASS, 1913-byte capture at r9 `target/boot-test-q35exec.log` — the marker binary's OWN ring-3 prints (notify got-note + alarm-interrupted sleep) via WRITE syscalls. Exec substrate byte-identical at the pin (flake.nix-only fix commit) |
| 3 | **XEXEC-03** — full boot chain boot(8)→init→rc reaches the interactive `% ` prompt on x86_64; the expect smoke gate green twice consecutively | ✓ VERIFIED | REGRESSION RE-PROOF (verifier-run at 4132e25): `nix develop . -c cargo xtask boot-test --arch x86-64 --config q35shell` → exit 0, PASS, 2473-byte capture — the REAL boot(8)→init→rc chain's own ring-3 prints through to the interactive `% ` prompt. Composed leg (executor-retained, content re-checked on disk): `build/x86_64/smoke-r9-run{1,2}.log` both present (4730 bytes each) with the `% ` prompt markers and the DEL-safety `r9x64delok` markers ×2 each — the twice-green composed session evidence intact |
| 4 | **XEXEC-04** — tier machinery keyed to objtype=amd64: tier-0 libs + tier-1 core commands proven executing on r9-x86_64 with committed artifact+execution markers, donor venue = k10t1 | ✓ VERIFIED | REGRESSION CHECK: `build/x86_64/tier1.ledger` 37 lines (35 `T1-OK:r9x64` + 2 SKIP rows) committed; retained captures present — `evidence/14-08-tier1-r9-x64.log` (T1-DONE, 2 T1-SKIP) and `evidence/14-08-tier-9k-serial.log` (T1-DONE). Verifier-run parity re-diff: unique `T1-OK <name>` sets extracted from both captures → 36 = 36, `diff` EMPTY — the identical-6.outs both-kernels ABI witness still holds on the retained evidence |
| 5 | **XEXEC-05** — /net re-gated on x86_64: loopback ping/udp/tcp echo, static + DHCP ipconfig, composed net smoke all green as committed gates | ✓ VERIFIED | REGRESSION CHECK (executor-retained, content re-checked on disk): `evidence/14-09-net-smoke-run{1,2}.log` (7045/7062 bytes — distinct runs) BOTH contain `NET-SMOKE: ALL STAGES PASS` ×2, `netip: NOT-L2-TRUTH slirp venue` ×2, `netgate: DUPBIND-REJECTED` ×2, `netgate: NOADDR-REJECTED` ×2 — the twice-green composed net gates with the venue-honesty label and both backstop edge markers (duplicate-binding rejection; no-address named FAIL) asserted as stages in each capture |
| 6 | 14-07 plan truth — "`nix build` on r9's boot-iso still verifies its hash" (kernel-only FOD), restated in 14-EVIDENCE.md's phase-close regression table | ✓ VERIFIED (gap closed) | VERIFIER-RUN at the new pin 4132e259: `nix build .#boot-iso` → exit 0 with a genuinely fresh kernel build (`building …-r9-kernel-x64-4132e25.drv`) plus the ISO FOD — the wired `sha256-lwbDSqYx…` verifies over the freshly derived ISO. Fix is scoped exactly as claimed (r9@4132e25 = flake.nix only, verified via git show; harvey pin advanced lock-only in its own commit; the EVIDENCE row corrected to record the close honestly). Two-derivation proof transcript retained at `evidence/14-gap-fod-proof.md` |

**Score:** 6/6 truths verified (0 present, behavior-unverified)

### Deferred Items

None — the prior gap is closed, not deferred. (deferred-items.md's historical 14-05/14-07 FOD entries carry RESOLVED headers; WINDOWS rows 56 and 60 are marked fixed with the standing re-wire-then-pin rule recorded.)

### Required Artifacts

All previously-verified artifacts unchanged at the new pin — the gap-fix round's only implementation deltas are r9's `flake.nix` (the hash re-wire, now ✓ VERIFIED by the live build), Harvey's `flake.lock` (the pin, verified = r9 HEAD), and planning-doc corrections. Regression checks run for this report:

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `../r9/flake.nix` | boot-iso outputHash wired at the pin | ✓ VERIFIED | `sha256-lwbDSqYx…` + full provenance comment; live build exit 0 (fresh kernel + ISO) |
| `flake.lock` | pins r9 at the fixed rev | ✓ VERIFIED | rev `4132e259e12f559edda97ea7c9ffee830ad7e32f` = r9 HEAD = origin/main |
| `evidence/14-gap-fod-proof.md` | two-derivation re-wire proof transcript | ✓ VERIFIED | on disk, 99 lines; arithmetic consistent with the wired hash and the live green build |
| `14-EVIDENCE.md` regression row | no overclaim | ✓ VERIFIED | close recorded as NOT green at e417660; fix recorded with the new-pin green this run corroborates |
| Retained captures (smoke ×2, tier ×2, net ×2, ledger) | twice-green gate evidence | ✓ VERIFIED | all present; marker content re-checked (see truths 3-5) |
| All other phase artifacts (ABI.md, dat.rs, uregcheck.rs, trap.rs, exec substrate, bake pipeline, mkfile.amd64, tiers, netaddr seam, expect gates, flake apps) | unchanged from prior verification | ✓ VERIFIED (by construction + spot greps) | r9@4132e25 touched flake.nix only (git show --stat: 1 file); harvey implementation files untouched since d89d938d5 (docs-only commits after); spot greps confirm const asserts / NEGATIVE CONTROL / ABI.md content |

### Key Link Verification

Previously-verified links re-confirmed at the new pin (all wiring is in files byte-identical to e417660 — the fix commit touched r9's flake.nix hash line and comment only, not the builder wiring; Harvey's flake.nix is unchanged, only flake.lock advanced):

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `harvey flake.lock` | `../r9@4132e259` | the pin names the re-wired rev | ✓ WIRED | rev matches r9 HEAD and origin/main (verifier-checked) |
| `../r9/flake.nix` boot-iso FOD | the ISO derivation | outputHash over the fresh derivation | ✓ WIRED | live build built kernel + ISO and verified the hash (exit 0) |
| `xtask/uregcheck.rs` → `xtask/main.rs` boot-test gate | both arches, before qemu | ✓ WIRED | bogus-R9_HARVEY_DIR → exit 1, named error (verifier-run at the pin) |
| All other key links from the prior report (ABI↔dat.rs, trap→syscall dispatch, syscall→usermem, main→devroot, grub templating, mkfile tier arms, net gate→fixtures) | | ✓ WIRED | unchanged files; see prior report rows, spot-checked via the live gates that exercise them (q35exec, q35shell, ureg-check) |

### Data-Flow Trace (Level 4)

Unchanged from the prior report: no static/hollow markers; the live gates re-run here (q35exec/q35shell) and the retained captures' markers are all produced by the asserted producers (ring-3 WRITE syscalls, in-guest sweep/fixtures). The FOD's data path is the live proof itself: fresh kernel build → ISO derivation → wired NAR hash → verified.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| boot-iso FOD hash at the new pin (the gap) | `nix build .#boot-iso` (r9@4132e259, cwd /home/req/code/r9) | exit 0; fresh `r9-kernel-x64-4132e25.drv` + `r9-boot-iso-4132e25.drv` built and hash-verified | ✓ PASS (verifier-run) |
| Dual-arch ureg-check + negative control | `R9_HARVEY_DIR=/home/req/code/harvey nix develop . -c cargo xtask ureg-check` | exit 0; 35 + 26 fields match; negative control fired | ✓ PASS (verifier-run) |
| boot-test gate precondition (x86_64) | `R9_HARVEY_DIR=/nonexistent-reverify … boot-test --config q35` | exit 1, named ureg-check error, before qemu | ✓ PASS (verifier-run) |
| First amd64 exec on r9 | `nix develop . -c cargo xtask boot-test --arch x86-64 --config q35exec` | exit 0 PASS; 1913 bytes; marker's own ring-3 prints | ✓ PASS (verifier-run) |
| Boot chain to `% ` | `nix develop . -c cargo xtask boot-test --arch x86-64 --config q35shell` | exit 0 PASS; 2473 bytes; chain prints to `% ` | ✓ PASS (verifier-run) |
| r9 workspace tests | `nix develop . -c cargo xtask test` | exit 0; 101 passed; 0 failed | ✓ PASS (verifier-run) |
| Tier both-kernels parity | extract+diff `T1-OK` sets from the two retained venue captures | 36 = 36, diff empty | ✓ PASS (verifier-run over retained captures) |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes declared; the phase's gates are xtask boot-tests and nix flake apps, covered above.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| XEXEC-01 | 14-01 | amd64 ABI contract published + build-enforced | ✓ SATISFIED | truth #1 (verifier-run gates at the pin) |
| XEXEC-02 | 14-02, 14-03, 14-04, 14-05 | amd64 binaries exec on r9 via real syscall path | ✓ SATISFIED | truth #2 (verifier-run q35exec PASS at the pin) |
| XEXEC-03 | 14-06, 14-07 | boot chain to `% ` + smoke green twice | ✓ SATISFIED | truth #3 (verifier-run q35shell PASS + retained twice-green composed captures) |
| XEXEC-04 | 14-08 | tier machinery on objtype=amd64, both venues | ✓ SATISFIED | truth #4 (ledger + retained captures + verifier-run parity diff) |
| XEXEC-05 | 14-09 | /net gates green on x86_64 | ✓ SATISFIED | truth #5 (retained twice-green captures, all stage markers ×2 re-checked) |

REQUIREMENTS.md marks all five Complete under Phase 14; every phase requirement ID (XEXEC-01..05) is claimed by plans and satisfied; no orphaned IDs.

### Anti-Patterns Found

Gap-fix-round files scanned (r9 flake.nix, flake.lock, 14-EVIDENCE.md, WINDOWS.md, STATE.md, deferred-items.md, evidence/14-gap-fod-proof.md): no debt markers, no stubs. The single `XXX` grep hit in r9's flake.nix (line 120) is a mktemp template string (`r9-iso-builder.XXXXXX`) — false positive. Pre-existing markers noted by the prior report (dat.rs `// XXX: remove soon.` blamed 2025-12-23; xtask FIXMEs blamed 2023/early-phase) are unchanged — Info (pre-existing), not phase-introduced, and the files are byte-identical to the prior verification.

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `../r9/x86_64/src/dat.rs` | 265 | `// XXX: remove soon.` | ℹ️ Info (pre-existing) | blame-dated 2025-12-23, predates Phase 14; unchanged at the pin |
| `../r9/xtask/src/main.rs` | 654, 694-695 | TODO/FIXME ×3 | ℹ️ Info (pre-existing) | blame-dated before Phase 14's base; unchanged at the pin |

### Code Review Context (not goal gaps)

14-REVIEW.md (7 findings: 1 CR, 3 WR, 3 IN) remains recorded OPEN in 14-REVIEW-DISPOSITION.md. Judged as known-open quality findings on delivered, goal-achieving code — none falsifies a phase truth (XEXEC-01's enforcement is re-proven live by this run's ureg-check + negative control + gate precondition). Flagged for the developer; the goal is judged on its truths above.

### Human Verification Required

None — the closed gap and all six truths carry verifier-run evidence from this run (live nix build, live boot-tests, live gates) or executor twice-green retained captures whose assertion content was re-checked on disk, per the sanctioned dual-evidence convention for heavy composed gates.

### Gaps Summary

No gaps. The prior report's single gap — the stale boot-iso FOD hash plus the EVIDENCE overclaim — is closed and verified LIVE at the new pin: r9@4132e25 (flake.nix-only re-wire to `sha256-lwbDSqYx…`, two-derivation proof transcript retained), Harvey pin advanced lock-only (`f6622b35f`) to exactly that pushed rev, the 14-EVIDENCE.md regression row corrected to record the close honestly (NOT green at e417660; green at the fix pin — corroborated by this run's exit-0 fresh build), and the standing re-wire-then-pin rule recorded (WINDOWS row 60, deferred-items RESOLVED). The five previously-verified truths regression-clean at the new pin: ureg-check + negative control + gate precondition green, q35exec and q35shell boot-tests PASS, 101/101 workspace tests, tier parity diff empty, all retained captures present with their markers intact.

---

_Verified: 2026-11-09T16:47:20Z_
_Verifier: Claude (gsd-verifier)_
