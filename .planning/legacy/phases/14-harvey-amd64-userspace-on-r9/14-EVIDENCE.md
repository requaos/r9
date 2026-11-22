# Phase 14 Evidence Index — Harvey amd64 Userspace on r9

**Phase:** 14-harvey-amd64-userspace-on-r9
**Consolidated:** 2026-11-09 (plan 14-09, task 3)
**Status:** complete — every requirement (XEXEC-01..05) mapped to the gate runs that prove it, at the pinned r9 SHA, with the retained artifacts under `evidence/`.

## Pinned commits

| Repo | SHA | Role |
|------|-----|------|
| r9 (`requaos/r9`, branch `main`) | **`e417660f215e688b65428f97394503477a01dbed`** | the phase-complete HEAD at consolidation — the whole phase is PUSHED (`origin/main..HEAD` = 0; the push-before-pin discipline held at every advance), and Harvey's `flake.lock` pinned exactly this rev at close (the lock-only commit `5294f27b8`). |
| r9 (`requaos/r9`, branch `main`) | **`4132e259e12f559edda97ea7c9ffee830ad7e32f`** | post-verification gap fix (2026-11-09, 14-VERIFICATION gap 1): the boot-iso FOD hash re-wired at the phase-close pin per the two-derivation proof discipline — pushed (`e417660..4132e25`), then pinned by Harvey's lock-only commit `f6622b35f` (push-before-pin held). |
| harvey | `4bf559e6c` | 14-09 Task 1: the x64 net fixtures + the net-binary bake. |
| harvey | `187984ac1` | 14-09 Task 2: the composed net gate — `boot-r9-net-smoke-x64` green twice + both captures retained. |
| harvey | this docs commit | 14-EVIDENCE.md + `evidence/` + the 14-09 summary (the consolidation you are reading). |

**Per-plan commit trail (all ancestors of the pinned HEADs):**

| Plan | Commits | Closed |
|------|---------|--------|
| 14-01 (ABI freeze) | r9 `10fea36` (184-byte Ureg), r9 `0f01476` (ureg-check amd64 + negative control), harvey `0aeab5032` (amd64/ABI.md) | XEXEC-01 |
| 14-02 (syscall entry) | r9 `f44c29f` (MSR arming), r9 `2b16f61` (the entry trio) | XEXEC-02 entry half |
| 14-03 (usermem + loader) | r9 `393f4f0` (usermem), r9 `8c4285a` (RED), r9 `822b3ec` (GREEN: the a.out loader) | XEXEC-02 loader half |
| 14-04 (proc + dispatch) | r9 `1b2f023` (proc machinery), r9 `6977748` (RED), r9 `7132059` (GREEN: the dispatch), r9 `abee15c` (entry wiring) | XEXEC-02 runtime half |
| 14-05 (first exec tracer) | r9 `d426787` (mb2 module + template), r9 `ae4fb7c` (exec_bootstrap), harvey `3131ae41a` (the x64 bake pipeline) | **XEXEC-02 complete** |
| 14-06 (boot chain) | harvey `66cd0fc1b`, `a8cdb4c45`, `86422fdea`; r9 `ac7aa92`, `c166164`, `a82d5a0`, `8aa9ba2` | XEXEC-03 r9-side leg |
| 14-07 (one-command compose) | r9 `fe39d83` (the iso-builder), r9 `8a338a6` (console RX), r9 `e5def72` (SWAPGS fix); harvey `f1e977a5c`, `d3a833f74`, `93236a5f3` + three lock-only pins `fd960ee5e`/`ff510457e`/`7189829d5` | **XEXEC-03 complete** |
| 14-08 (tier machinery) | r9 `e417660` (user SSE + eager stack); harvey `7b515dcee`, `bce0f69f1`, `b50a0e1ca` + pin `5294f27b8` | **XEXEC-04 complete** |
| 14-09 (net gates + evidence) | harvey `4bf559e6c`, `187984ac1` | **XEXEC-05 complete** |

## Requirement → gate-run mapping

### XEXEC-01 — the ABI freeze, published and enforced from the first commit

- **The frozen frame:** r9's x86_64 Ureg reworked to the 184-byte 9k-verbatim layout (u16 ds/es/fs/gs quartet packed in one qword at 120, type@128, cs@152), always-on const asserts in `dat.rs`, every frame producer and SWAPPED-witness site re-derived (r9@`10fea36`).
- **The publication:** `amd64/ABI.md` — the FULL exec interface (Ureg offsets + the `9syscall/sys.h` table + the wire contract: number in BP, args at SP+8 with SP+0 the return-address slot, AX result, Sfmask=If; the nsec no-spill / `_seek` / `_exits` stub quirks; the loud-unimplemented rule; the USTKTOP `0x00007ffffffff000` + 16 MiB stack policy) — every frozen fact citing its read-verbatim source (harvey@`0aeab5032`).
- **Gate runs:** `cargo xtask ureg-check` — BOTH arches print Match and the D-02b NEGATIVE CONTROL fires (an in-memory copy of the REAL amd64 header, cs→cseg, must be rejected; a pass is the loud `NEGATIVE CONTROL FAILED`); the check is live as a boot-test precondition on BOTH arches (`R9_HARVEY_DIR=/nonexistent-coverage` exits 1 with the named amd64 error before qemu spawns). `cargo test -p xtask` 97 green incl. the 6 new amd64 tests.
- **Provenance:** 14-01 (all three tasks).

### XEXEC-02 — the staged exec proof (14-02 → 14-05, the shared-ID gate closed at 14-05)

- **The staged-init discipline (D-08):** entry arming (14-02, armed-but-idle through every gate) → usermem + the amd64 a.out loader (14-03, host-tested RED→GREEN: 10 loader tests + 8 usermem tests, the empty/adjacency edges pinned) → proc machinery + the measured-set dispatch (14-04, 58 host tests; per-process PML4/CR3 with the kernel half shared, TSS-rsp0 kstacks) → the first real exec (14-05).
- **THE gate run:** `cargo xtask boot-test --arch x86-64 --config q35exec` — **green TWICE CONSECUTIVELY**. The marker binary (smokesleep, staged as `/amd64/init`) prints through its OWN WRITE syscalls from ring 3: `smokesleep: got note alarm` + `smokesleep: interrupted` (the note delivered via ureg->bp, the sleep interrupted by the alarm, `noted(NCONT)` resumed) → `all user processes exited`. A kernel print can never produce the marker's lines (D-09).
- **The bring-up that proved the gate bites:** the first ring-3 exec exposed and fixed THREE latent kernel defects — the identity-alias entry (l.S RIP-relative since 13-01), the non-canonical KSTACK_VA, the silent panic handler (r9@`ae4fb7c`); plus the AX/Tos exec-return seed correction (14-06, ABI.md amended — `main9.s` stores AX into `_tos`).
- **The A1 real-binary proof:** the stage build asserts `00008a97` (amd64 S_MAGIC) on real 6.out bytes and prints it ("stage: init magic 00008a97 verified"); the bake is KERNDATE-deterministic (two bakes byte-identical).
- **Provenance:** 14-02/03/04 SUMMARYs + 14-05 SUMMARY (the shared-ID gate completed with 14-05's SUMMARY).

### XEXEC-03 — the boot chain to the interactive prompt + the one-command compose

- **r9-side leg (14-06):** `cargo xtask boot-test --arch x86-64 --config q35shell` — **green twice consecutively (four consecutive passes total at r9@8aa9ba2)**: banner → facts → discovery → bind → module/paqfs → exec entry → boot(8)'s `paqfs...`/`version...` → `init: starting /bin/rc` → **the interactive `% ` prompt**. Getting there surfaced and fixed: the conforming-ktext ring-3 delivery defect, the AX/Tos seed, the 2-page stack seed, lazy-brk-without-a-pager, embed.c's sleeping-4e waitpid deadlock (Harvey in-tree), the preempt-park CR3 aliasing + the device-MMIO window, and — cycle 4, root-caused with pid-keyed memory-side probe rings — sched_loop's stale-SCHED_CONTEXT resume arm (r9@`8aa9ba2`).
- **Composed leg (14-07):** `nix run .#boot-r9-x64 </dev/null` — the ONE command (userland build → bake → r9's iso-builder → `qemu -M q35 -accel kvm -cdrom`) reaches the interactive `% ` prompt. **THE gate:** `nix run .#boot-r9-smoke-x64` — **green TWICE CONSECUTIVELY, both captures retained** (22 stage-oks each: the echo round-trip, the EMPTY-LINE safety stage — the flagged assumption, landed — the ls/ps/cat sweep, DEL safety, init restart, pid-warning-absent, nic-bind; the two runs' composed ISOs byte-identical, sha256 `2951bbb0…` both). The gate surfaced and fixed two more kernel defects: console RX delivery (r9@`8a338a6`) and the SWAPGS witness-bit stale-byte leak (r9@`e5def72`, the donor's CS-word cmpw).
- **Provenance:** 14-06 + 14-07 SUMMARYs.

### XEXEC-04 — the both-kernels tier witness (identical 6.outs, structurally proven)

- **THE witness is structural (T-14-15):** `build/mm/build-9k10t1.sh`'s r9 leg pins a 38-member sha256 manifest of the exact `/amd64/bin` set the k10t1 kernel embedded; `build/x86_64/mkimage.sh`'s tier bake VERIFYs every digest before staging the r9 image ("witness manifest verified: 38 members match the donor venue's embed set" — printed on every bake, re-verified in this plan's runs). A drifted member reds the bake, never the eye.
- **Gate runs:** `sh build/x86_64/subset-build.sh` — `TIER-BUILD-OK`, 15 T0-ART + 40 T1-ART markers; `sh build/mm/boot-9k10t1-sweep.sh` — the 9k leg `T1-VENUE-PASS ok=36 skip=24 fail=0` AND the r9 leg `T1-R9X64-PASS ok=36 skip=2 fail=0`, **the two captures' T1-OK name sets diff EMPTY (36 = 36)** — D-01's both-kernels consequence as executable proof.
- **The r9-side gate:** `nix run .#boot-r9-tier1-x64` — **green TWICE CONSECUTIVELY** through the full pipeline; the capture retained at `evidence/14-08-tier1-r9-x64.log` and the donor capture at `evidence/14-08-tier-9k-serial.log`; the 37 marker lines committed in `build/x86_64/tier1.ledger` with sha256 values matching the witness manifest.
- **The sweep's T1-FAIL posture (the flagged assumption, held):** the first r9 sweep surfaced TWO real kernel defects as CLASSIFIED captures with digests (never silent skips, never hangs) — user SSE never armed (CR4.OSFXSR absent) and the 32-page eager stack 392 bytes too shallow for cmp — both fixed donor-faithfully in r9@`e417660` with the full regression battery green (q35exec/q35shell/q35/q35net/aarch64 virt + 101 port tests + clippy), push-before-pin held (pushed, then the lock-only pin `5294f27b8`).
- **Honest note (the sweep's declared SKIP):** the x64 tier sweep's `T1-SKIP ipconfig` carried the aarch64 06-07 measured reason ("r9 has no devrandom this era") — stale for x86_64: the port devcons serves `/dev/random` there (never erroring; no entropy hooks registered this era, so the DRBG seeds deterministically — counter advance still yields distinct bytes per read). XEXEC-05's net gates below prove ipconfig green in-guest on r9-x64; the sweep check flips with Phase 15's tier append (never a quiet edit of the committed sweep).
- **Provenance:** 14-08 SUMMARY + this plan's re-verification.

### XEXEC-05 — the /net stack re-gated on x86_64 (the composed net gate, venue-honest)

- **THE gate:** `nix run .#boot-r9-net-smoke-x64` — **GREEN TWICE CONSECUTIVELY, both captures retained** at `evidence/14-09-net-smoke-run{1,2}.log` (45 stage-oks each run). One boot drives the whole composed flow: chain boot (banner → `paqfs...` → `version...` → the `% ` prompt) → the D-11 seam sourced (`. /lib/netaddr.rc`) → **static ipconfig** (`rc /lib/netip-static.rc`: the REAL unmodified ipconfig assigns the seam's static IPv4 + default route; ipselftab/iproute asserts, gateway ping, the arp view) → **the duplicate-binding rejection** → the ether unbind → **DHCP ipconfig** (`rc /lib/netip-dhcp.rc`: the dhcpclient DORA over /net/udp broadcast, the lease + default route + `/net/ndb` write + gateway ping) → the second ether unbind → **loopback ping + udp echo** (`rc /lib/netip-loopback.rc`: the random leg, `ping 127.0.0.1` via `$lo_guestv4`, the udpecho round-trip) → **the no-address rejection** → **the tcp/7 echo leg** (typed inline against the live loopback core: the baked tcpecho server + the fd-4-held conv client exchange) → the `NET-SMOKE: ALL STAGES PASS` completion marker (two-occurrence) → the whole-capture post-passes (pid-warning-absent + nic-bind).
- **The degenerate-input edges (the backstop truths, as NAMED stages in the captures):**
  - *adjacency:* `netgate: DUPBIND-REJECTED` — an equal-address ipconfig attempt against the still-bound ether is rejected VISIBLY: devip's own `interface already bound` error rides the wire (stage-asserted) and the marker prints only because ipconfig exited non-nil (two-occurrence staged; a silent merge would fail the second occurrence). Both runs' captures carry the pair.
  - *empty input:* `netgate: NOADDR-REJECTED` — a ping with NO target address (`$noaddr` deliberately unset) fails loudly at ping's argv check (usage + non-nil exit → the marker), never wedges. Two-occurrence staged in both captures.
- **THE VENUE-HONESTY RECORD (Pitfall 10 / T-14-16):** the XEXEC-05 gates run on qemu `-netdev user` (slirp) and are explicitly **NOT L2 truth** — slirp forgives L2 bugs by design. The label `netip: NOT-L2-TRUTH slirp venue` is printed INTO both captures BY the static and dhcp fixtures and ASSERTED as stages (`static-l2`, `dhcp-l2`) — the evidence can never be re-read as tap-venue proof. The tap-leg truth verdict is **Phase 16's NETIP-08 gate, out of scope here** (the aarch64 Phase-8 re-key under slirp is exactly what these gates are).
- **The D-11 single-seam discipline (T-14-17, mechanically asserted):** every address in the x64 fixtures comes from `build/x86_64/root-tree/lib/netaddr.rc` (the `lo_`/`slirp_`/`blastport` keys); the bake itself greps `root-tree/lib/` for `10.0.2.` literals outside `netaddr.rc` and a stray FAILS the bake (plus the plan-level verify: `! grep -rn '10\.0\.2\.' build/x86_64/root-tree/lib/ | grep -v netaddr.rc | grep -q .` — clean).
- **The net binaries build and bake:** ipconfig + ping + udpecho via the ip container's amd64 paths, tcpecho/tcpblast/netblast as the 6c C-helper re-keys — all `00008a97`-verified, baked into `bootfs-net.paq` (KERNDATE-deterministic: sha256 `ffdccd6c…` on consecutive bakes), 31-line named-input manifest printed as bake evidence.
- **The aarch64 net fixtures, expect, and app are byte-untouched** (`git diff` empty on `build/aarch64/netaddr.rc`, `build/aarch64/root-tree/lib/`, `build/aarch64/smoke-r9-net.expect`, `nix/smoke-r9-net.sh` — the frozen-at-pin sibling).
- **Provenance:** 14-09 (this plan).

## The flagged-assumption outcomes (all landed, all green)

| Flagged assumption (plan) | Outcome | Evidence |
|---------------------------|---------|----------|
| the empty-line echo stage (14-07: a bare newline at the prompt must return a fresh `% `) | LANDED as a gate stage (`emptyline-prompt`) — the aarch64 template lacked it | the 14-07 twice-green captures; recorded in the 14-07 SUMMARY |
| the tier sweep's T1-FAIL posture (14-08: venue gaps surface as classified FAIL captures, never silent skips) | HELD — the first r9 sweep's 4 T1-FAILs carried digests and localized two kernel defects (user SSE, eager stack) | 14-08 SUMMARY + r9@e417660 |
| the net degenerate-input stages (14-09: duplicate/equal binding + no-address must fail with named markers, not merge/wedge) | LANDED as the two `netgate:` degenerate stages above | both `evidence/14-09-net-smoke-run{1,2}.log` captures |
| the exec-critical miniz-inflate heisenbug watch (14-CONTEXT) | no recurrence — the paq format carried verbatim (header `529ab12b` / trailer `6b46e688` on every bake this phase) | every bake log; the magic asserts in mkimage.sh |

## The witness-parity record (14-08, restated)

ONE `/amd64/bin` set — the 38-member sha256 manifest the donor (k10t1) build pins and every tier bake verifies — executed on BOTH kernels with IDENTICAL 36-marker T1-OK sets (diff-proven at gate time): `T1-VENUE-PASS ok=36 skip=24 fail=0` (9k10t1, the donor venue) and `T1-R9X64-PASS ok=36 skip=2 fail=0` (r9-x64). The SKIP sets differ by DESIGN (the two venues read different tier data files this phase — aarch64's full v2.0 list vs the x64 core scope; Phase 15's appends converge them); the parity claim covers the shared membership — exactly the embed-set witness. `build/x86_64/tier1.ledger` is the committed 37-line execution record.

## The discretion / OQ resolutions (recorded for the record)

- **The address-space model (14-04, the locked discretion):** per-process PML4/CR3 with the kernel half SHARED (boot-PML4 entries 510/511 copied at construction); kstacks at a fixed kernel-half window (PML4[400], US-clear) behind TSS rsp0 — the ONE kernel-stack mechanism both ring-3 entry paths share. TEXT maps EAGERLY (no trap-context pager this era; the pager + deeper stack/tos growth are the recorded follow-ups).
- **The flake API shape (14-07, OQ-1):** BOTH-shapes-from-one-source — r9 exports the argv-driven `r9-iso-builder` script AND the flake-lib `bootIso` function from ONE `writeShellApplication` text (D-05, un-forked); Harvey's apps consume the binary path (`<pkg>/bin/r9-iso-builder` — the root-directory trap caught and fixed at harvey@`d3a833f74`).
- **The grub template (14-05):** the committed `grub.cfg` is a TEMPLATE SOURCE (`@R9-MODULE@` markers); the kernel-only emission is byte-identical to Phase 13's (cmp-proven + unit-test-pinned), and every later consumer (xtask, the 14-07 flake builder) substitutes the `module2` line IN/OUT.
- **The app naming (14-05→09):** the x64 compose apps are `boot-r9-x64` / `boot-r9-smoke-x64` / `boot-r9-tier1-x64` / `boot-r9-net-smoke-x64` — the aarch64 app names and bodies byte-untouched (frozen at the v2.0 pin; Phase 19's INT-05 owns any one-command consolidation).
- **The two-images-not-one rule (14-06):** the chain bakes `out/bootfs-chain.paq` while q35exec keeps `out/bootfs.paq` (the marker IS the leg-1 image's `/amd64/init`); the tier and net bakes (`bootfs-tier.paq`, `bootfs-net.paq`) extend the same separation — four images, four contracts, one mkfile of named-input asserts.

## Regression state at the phase close (r9 @ e417660, fully pushed)

| Leg | Result |
|-----|--------|
| q35exec, q35shell, q35, q35net boot-tests + aarch64 virt sibling | PASS (the full battery re-run at r9@e417660, the 14-08 close) |
| `cargo xtask test` (101 port tests) / check / clippy | green / exit 0 / clean (exec.rs zero warnings) |
| `nix build .#boot-iso` (r9) | at the close itself (r9@e417660): **NOT run, and NOT green** — the verifier caught the wired hash stale there (exit 1: specified `sha256-lEqthp7h…` ≠ got `sha256-lwbDSqYx…`; the fe39d83-era 14-07 hash could not survive the phase's own post-fe39d83 kernel commits `8a338a6`/`e5def72`/`e417660` — 14-VERIFICATION gap 1, the row previously claiming green here has been corrected). FIXED post-verification: re-wired at r9@`4132e25` after the two-derivation fresh-kernel byte-identity proof; `nix build .#boot-iso` AND `--rebuild` both exit 0 at the new pin (proof transcript retained at `evidence/14-gap-fod-proof.md`). |
| Harvey flake | `nix run .#boot-r9-x64` → `% `; `.#boot-r9-smoke-x64` green ×2 (14-07); `.#boot-r9-tier1-x64` green ×2 (14-08); `.#boot-r9-net-smoke-x64` green ×2 (14-09) |
| this plan's runs | subset-build `TIER-BUILD-OK` (15+40 markers + the net section); all four bakes PASS with the witness manifest verified; the net gate green ×2 consecutively |

## Retained artifacts (`evidence/`)

| File | What it is |
|------|-----------|
| `evidence/14-08-tier1-r9-x64.log` | the r9-x64 tier gate capture (ok=36 skip=2 fail=0, T1-DONE, the % prompt) |
| `evidence/14-08-tier-9k-serial.log` | the donor (k10t1) venue capture of the same binary set (ok=36) — the both-kernels witness pair |
| `evidence/14-09-net-smoke-run1.log` | the composed net gate, run 1 — all 45 stages (static + dupbind + dhcp + unbinds + loopback + noaddr + tcp + completion + post-passes), the venue labels and both `netgate:` degenerate markers present |
| `evidence/14-09-net-smoke-run2.log` | run 2 — the re-green, identical stage set |

The x64 gate captures named in the 14-07/14-08 SUMMARYs (`build/x86_64/smoke-r9-run{1,2}.log`, `build/x86_64/tier1-r9-x64.log`) are gitignored transients by design — the passing copies land here (`evidence/`); the r9-side boot-test captures live in the 13-EVIDENCE discipline (r9's own tree/logs) and the 14-05/14-06 gate captures were the expect/xtask outputs quoted line-for-line in those SUMMARYs.

---
*Phase: 14-harvey-amd64-userspace-on-r9*
*Consolidated: 2026-11-09*
