---
phase: 18-in-guest-p9sk1-auth-domain
reviewed: 2026-11-14T17:23:39Z
depth: standard
files_reviewed: 57
files_reviewed_list:
  - build/x86_64/authdomain.example
  - build/x86_64/authdomain-gen.c
  - build/x86_64/authdomain-selftest.sh
  - build/x86_64/authdom-donors/auth.h
  - build/x86_64/authdom-donors/authsrv.h
  - build/x86_64/authdom-donors/crypt.c
  - build/x86_64/authdom-donors/des.c
  - build/x86_64/authdom-donors/desCBC.c
  - build/x86_64/authdom-donors/desmodes.c
  - build/x86_64/authdom-donors/hmac.c
  - build/x86_64/authdom-donors/libc.h
  - build/x86_64/authdom-donors/libsec.h
  - build/x86_64/authdom-donors/mp.h
  - build/x86_64/authdom-donors/nvcsum.c
  - build/x86_64/authdom-donors/os.h
  - build/x86_64/authdom-donors/passtokey.c
  - build/x86_64/authdom-donors/sha1block.c
  - build/x86_64/authdom-donors/sha1.c
  - build/x86_64/authdom-donors/strecpy.c
  - build/x86_64/authdom-donors/u.h
  - build/x86_64/authprobe.c
  - build/x86_64/auth-sweep.rc
  - build/x86_64/auth-termrc
  - build/x86_64/mkimage.sh
  - build/x86_64/randread.c
  - build/x86_64/README.md
  - build/x86_64/root-tree/lib/ndb/local.template
  - build/x86_64/root-tree/lib/netaddr.rc
  - build/x86_64/secstore-obj.txt
  - build/x86_64/smoke-r9-auth.expect
  - build/x86_64/stage-build.sh
  - build/x86_64/tiers/tier2-services.txt
  - flake.lock
  - flake.nix
  - .github/workflows/r9-shell.yml
  - .gitignore
  - nix/smoke-r9-auth-x64.sh
  - ../r9/Cargo.toml
  - ../r9/flake.nix
  - ../r9/port/src/dat.rs
  - ../r9/port/src/devcap.rs
  - ../r9/port/src/devcap_tests.rs
  - ../r9/port/src/devcons.rs
  - ../r9/port/src/devip/proto.rs
  - ../r9/port/src/devip_tests.rs
  - ../r9/port/src/hmacsha1.rs
  - ../r9/port/src/lib.rs
  - ../r9/port/src/namec.rs
  - ../r9/x86_64/Cargo.toml
  - ../r9/x86_64/lib/config_q35auth.toml
  - ../r9/x86_64/src/cpu.rs
  - ../r9/x86_64/src/entropy.rs
  - ../r9/x86_64/src/main.rs
  - ../r9/x86_64/src/proc.rs
  - ../r9/x86_64/src/syscall.rs
  - ../r9/xtask/src/lib.rs
  - ../r9/xtask/src/main.rs
findings:
  critical: 3
  warning: 4
  info: 7
  total: 14
  necessity: 1
status: issues_found
---

# Phase 18: Code Review Report

**Reviewed:** 2026-11-14T17:23:39Z (gap-closure round; first review 2026-11-14T07:26:48Z)
**Depth:** standard
**Files Reviewed:** 57 cumulative (Round 1: 55; the gap-closure round added `../r9/port/src/devip/proto.rs` and `../r9/port/src/devip_tests.rs`, and re-reviewed 10 previously-listed files)
**Status:** issues_found

## Summary

Two review rounds, both at standard depth, across both repos (Harvey + the sibling r9 checkout at `/home/req/code/r9`).

**Round 1 (18-01..18-05)** found the build machinery (mkimage.sh §14, authdomain-gen.c, the self-test, the expect gate, the nix driver) unusually rigorous, and landed two Critical r9-kernel findings: CR-01 (the ungated `/dev/hostowner` write defeating the devcap eve gate) and CR-02 (the RDRAND CPUID bit read from leaf-7 ECX instead of leaf-1 ECX). **The gap-closure round (18-06..18-08) closes both correctly**: the hostowner write arm now carries the donor's iseve gate + 128-byte bound with three host pins (gate/bound/eve-path), and `armed_source()` now derives `cpuid1_ecx` from a fresh `cpuid(1,0)` call, exercised through a new scripted-cpuid seam with three probe pins — both fixes verified against the donor source and smoltcp/CPUID semantics this round.

The gap-closure round lands one new Critical (CR-03): the 18-07 two-write TCP fix (`tcp_socket`: Nagle off, ACK immediate) was applied **only to the connect path** — `AcceptPool::spawn_listener` (stack.rs) still constructs every announced-side socket with smoltcp's stock defaults (`nagle: true`, `ack_delay: Some(10ms)`), and `tcp_listen_pop` materializes each accepted conv around exactly that pool-member socket. On x86_64, where the 1 ms tick never calls `stack::tick_check` (aarch64-only wiring — the fix commit's own words), a client write larger than the 2048-byte tx buffer into an announced connection (a secstore PUT of any real file — the protocol's purpose) parks the writer forever awaiting the server socket's delayed pure ACK: the same 18-05 stall class, still live on the un-fixed half. The committed gate is green only because the DEMO round-trip object is 401 bytes.

The expect gate's presence-demand flip (18-08) was checked tooth-by-tooth and is a strict strengthening — every pre-existing post-pass (nvram-prompt-absent, zero AUTH-FAIL, DEMO label, ANNOUNCE, DENIED-exactly-once, tr-ok, no-speaks-for, redaction-last) stayed armed and six new presence demands (saved/AUTH/PUT/GET/SECOBS/verdict) were added both as ordered stages and whole-capture post-passes; round-1 IN-01 (the workflow comment overstating CI coverage) is resolved by the reword. The rc sweep's restored 9d legs honor the measured in-guest dialect constraints (single-quoted `=`-bearing argv, `if not` same-line, plain — not fd-dup — redirects, newline-only ifs backquotes); the lock-only pin advance (flake.lock → r9 `9103522`) matches the r9 checkout head exactly.

The known recorded items (r9 devip two-write TCP defect — now closed on the connect path only, see CR-03; #68 services dns-stage trap; eqn.c regeneration noise; WINDOWS ledger drift) are tracked in the deferred-items ledger.

## Round 1 (18-01..18-05) — First Review (preserved verbatim)

Standard-depth adversarial review of the Phase 18 "In-Guest p9sk1 Auth Domain" file set across both repos. The build machinery (mkimage.sh §14, authdomain-gen.c, the self-test, the expect gate, the nix driver) is unusually rigorous — bake-time pairing asserts, negative controls, redaction, and survivor sweeps all check out against the donor sources (Nvrsafe 109/112 layout, KEYDBLEN 73, writeusers record shape, speaksfor row form, and the randread/devcap HMAC fixture all re-verified independently this review; the fixture HMAC reproduces exactly). The vendored donor set is byte-identical to the in-tree originals (crypt/des*/sha1*/strecpy identical to libsec/libc/libauthsrv; authsrv.h/libsec.h identical to sys/include) — no findings there, per the phase contract.

Two Critical findings land in the r9 kernel half. First, the phase wired `iseve()` (the base of devcap's `#¤/caphash` hostowner gate) to the `#c/hostowner` static while leaving the `/dev/hostowner` write completely ungated — the donor's `hostownerwrite` (sys/src/9k/port/sysauth.c:152-153) is iseve-gated, so any process can rewrite the eve base and mint capability hashes, defeating the exact trust boundary the phase's own HOSTOWNER-GATE marker claims to prove. Second, the new x86_64 entropy classifier reads the RDRAND CPUID bit from leaf 7's ECX instead of leaf 1's ECX — objectively wrong wiring that both current venues (TCG qemu64, KVM -cpu host with RDSEED) mask by accident.

The known recorded items (r9 devip two-write TCP defect / AUTH-08 partial, #68 services dns-stage trap, eqn.c regeneration noise, WINDOWS ledger drift) were not re-reported.

### Critical Issues (Round 1)

#### CR-01: Ungated `/dev/hostowner` write defeats the devcap eve gate (authorization bypass)

**File:** `../r9/port/src/devcons.rs:579-585` (write arm), `../r9/port/src/devcons.rs:288-295` (`iseve`), consumer `../r9/port/src/devcap.rs:199,247-248`
**Issue:** Phase 18-02 (commit 0358cbe) made `HOSTOWNER` the comparison base of `iseve()` — and therefore the trust boundary of `#¤/caphash` (devcap's hostowner gate) — but the `QID_HOSTOWNER` write arm stores any bytes from any process with no `iseve()` check, no length bound, and no validation (`open()` at devcons.rs:348-363 does not gate it either; `sys_pwrite` dispatches straight to `devcons::write`). The module comment at devcons.rs:178 even says "the glenda no-op's **guarded** `#c/hostowner` write stores here" — no guard exists. Exploit chain inside this phase's own guest: a process mutated to a non-eve user by capuse writes its own name to `/dev/hostowner` (now `current_user() == base` → `iseve()` true), opens `#¤/caphash` (the Eperm gate passes), posts an arbitrary HMAC hash, presents the matching `from@to@key` string, and becomes any user. This directly falsifies the substrate gate's own asserted contract — randread's `cap: HOSTOWNER-GATE` marker ("the mutated user is no longer eve") is bypassable by one write to a file the same kernel serves. The donor is explicit: `hostownerwrite` (sys/src/9k/port/sysauth.c:148-165) errors `Eperm` for non-eve writers and `Ebadarg` for n <= 0 or n >= 128, then renames the user and re-seeds both eve and up->user. The write pre-dates this phase (present at 8f753ca), but only this phase made it security-load-bearing.
**Status:** FIXED in 18-06 (r9 652f025) — gate + bound landed as prescribed; verified this round (see WR-03 for the residual).
**Fix (as prescribed):**

```rust
QID_HOSTOWNER => {
    if !iseve()? {
        return Err("devcons: hostowner requires the hostowner");
    }
    if buf.is_empty() || buf.len() >= 128 {
        return Err("devcons: bad hostowner write");
    }
    let node = LockNode::new();
    let mut stored = HOSTOWNER.lock(&node);
    stored.clear();
    stored.extend_from_slice(buf);
    Ok(buf.len() as u64)
}
```

(Decide deliberately whether the donor's rename-user side effects belong in this era; the gate itself is the must-fix. Add a devcons host pin for the non-eve refusal, and consider having randread's HOSTOWNER-GATE leg attempt the `/dev/hostowner` write and assert its refusal — that would give the bypass its own marker.)

#### CR-02: RDRAND CPUID bit read from leaf 7 ECX instead of leaf 1 ECX (entropy source misclassification)

**File:** `../r9/x86_64/src/entropy.rs:108-111`
**Issue:** `armed_source()` does:

```rust
let (max_leaf, _, _, _) = cpu::cpuid(0, 0);
let (_, ebx7, ecx1, _) = cpu::cpuid(7, 0);
classify(max_leaf, ecx1, ebx7)
```

`cpu::cpuid` returns `(eax, ebx, ecx, edx)` (cpu.rs:164-185), so `ecx1` is bound to **leaf 7's ECX**, while `has_rdrand` tests leaf-**1** ECX bit 30 (`cpu::has_rdrand`, cpu.rs:224-226, whose doc and the entropy.rs module header both state "RDRAND when only leaf-1 ECX bit 30 is present"). Leaf 7 subleaf 0 ECX bit 30 is not the RDRAND feature bit. Consequences: on RDRAND-without-RDSEED hardware (Ivy Bridge/Hasberg class — RDRAND shipped two generations before RDSEED) the classifier falls through to the jitter fold, silently downgrading the entropy source and printing the wrong `random: source=` label (the q35auth venue assert would then red the KVM leg as a "venue mismatch"); in the inverse direction, a CPU/hypervisor that sets leaf-7 ECX bit 30 without leaf-1 RDRAND arms the RDRAND instruction path on a CPU that would #UD on it (kernel fault at first seed). Both current venues classify correctly only by accident: TCG qemu64 exposes neither feature bit, and the dev KVM host has RDSEED, which wins via the correctly-sourced leaf-7 EBX. The host pins exercise `classify()` with synthetic values (`entropy_classify_orders_rdseed_rdrand_jitter`), so no test executes the mis-wired probe.
**Status:** FIXED in 18-06 (r9 f594b4c) — the rebind landed exactly as prescribed, plus the scripted-cpuid seam and three probe pins; verified against cpu.rs this round.
**Fix (as prescribed):**

```rust
let (max_leaf, _, _, _) = cpu::cpuid(0, 0);
let (_, _, cpuid1_ecx, _) = cpu::cpuid(1, 0);
let (_, cpuid7_ebx, _, _) = cpu::cpuid(7, 0);
classify(max_leaf, cpuid1_ecx, cpuid7_ebx)
```

(Note the second call's leaf must remain 7 for EBX; the bug is only the ECX binding — one fresh `cpuid(1, 0)` call supplies the correct ECX.)

### Warnings (Round 1)

#### WR-01: One-byte stack overflow in `authdomain-gen -V`/`-i` on a manifest of exactly `Filecap` bytes

**File:** `build/x86_64/authdomain-gen.c:615-616` (verify), `build/x86_64/authdomain-gen.c:775-776` (info)
**Issue:** `readfile(dir, "manifest", (uchar*)manifest, Filecap)` can legitimately return `Filecap` (65536) — the exactly-full case is only distinguished from "one more byte hiding" when a further read succeeds; a file of exactly `Filecap` bytes returns `cap` with no error. Both callers then execute `manifest[n] = 0` with `n == Filecap`, writing one NUL past the end of `char manifest[Filecap]`. `-V` is explicitly the doctored-input-facing arm of the pairing contract (the self-test's negative-control class and `AUTHBAKE_DOCTOR` exist to prove its teeth), so a hostile/corrupted staging dir reaching exactly 64 KiB turns the refusal tool into a stack OOB write (crash under a stack protector; memory corruption otherwise). Same shape in `info()`.
**Fix:** Size the read one byte short so the terminator always fits, and treat the extra byte as an error:

```c
char manifest[Filecap];
int n = readfile(dir, "manifest", (uchar*)manifest, Filecap-1);
manifest[n] = 0;
```

(a file of `Filecap` or more bytes then fails loudly inside `readfile`'s "bigger than %d bytes" check, which is the correct verdict for a non-artifact.)

#### WR-02: Generator accepts passwords the donor `passtokey` silently truncates at 27 chars

**File:** `build/x86_64/authdomain-gen.c:84,343-344,357-362` vs `build/x86_64/authdom-donors/passtokey.c:11-13`
**Issue:** `adduser` rejects passwords only at `strlen(pw) >= Maxpw` (32) and `setfield` allows `machine_password` up to 31 chars, but the donor `passtokey` caps input at `ANAMELEN-1` = 27 (`n = strlen(p); if(n >= ANAMELEN) n = ANAMELEN-1;`) with no diagnostic. A real-domain `authdomain.local` with two user lines (or a machine password) whose first 27 characters are shared silently derives identical DES keys — the keys file pairs "correctly" (the manifest verifies, keyfs loads) while the domain does not mean what the operator wrote. The committed `authdomain.example` documents no length limit at all. The `Maxpw=32` comment ("passtokey reads <=27") shows the constraint was known but never enforced where the bytes enter.
**Fix:** Enforce the donor envelope at the interface boundary — in `adduser` and in the `machine_password`/`secstore_password` handling (or at least `machine_password`, which derives the master key):

```c
if(strlen(pw) >= 28)
	sysfatal("user %s: password longer than 27 chars (passtokey's limit)", name);
```

and document the 27-char limit in `authdomain.example` (the secstore field is already correctly bounded at 13 by CONFIGLEN).

### Info (Round 1)

#### IN-01: Workflow comment overstates the CI contract — the secstore round-trip is NAMED-SKIPPED, not exercised

**File:** `.github/workflows/r9-shell.yml:96`
**Issue:** The `auth-smoke-x64` job comment says the gate runs "the speaksfor C probe with its fail-closed deny control) and the secstore PUT/GET round-trip." The gate does the opposite on the round-trip: `smoke-r9-auth.expect:442-447` asserts `AUTH-OK secstore-roundtrip` is **PROVABLY absent** (the named SKIP for the recorded r9 devip two-write defect). Gates are measurement instruments whose comments describe the contract; this one misdescribes what green means to any reader auditing CI's proof coverage.
**Status:** RESOLVED in 18-08 — the comment now says "EXERCISED and presence-demanded" and names the "PENDING FIRST PUSH" convention, matching the flipped gate.
**Fix (was):** Reword to describe the actual contract (superseded by the 18-08 flip).

#### IN-02: Sweep's secondary probe address lives outside the netaddr seam

**File:** `build/x86_64/auth-sweep.rc:220`
**Issue:** Leg 3 adds a raw `192.168.237.1 255.255.255.255` probe address inline. The D-11 single-seam doctrine (netaddr.rc: "NO gate or fixture hardcodes a ... per-leg address anywhere else") mechanically covers only `root-tree/lib` (mkimage §10b greps), so this is not a bake-red violation, but it is the same class of literal the seam exists to prevent — a future venue colliding with 192.168.237.0/24 would have to be found by eye, exactly what D-11's "never the eye later" rationale rejects.
**Fix:** Add a `probe2_guestv4=192.168.237.1` key to `root-tree/lib/netaddr.rc` and substitute at bake time (or extend the seam grep scope to the committed sweeps if in-guest fixtures are deliberately out of seam scope — then record that verdict in netaddr.rc like the v6 deferral note).

#### IN-03: Modulo bias in `genpassword`

**File:** `build/x86_64/authdomain-gen.c:215`
**Issue:** `alph[r[i] % (nelem(alph)-1)]` folds 256 byte values onto 62 symbols; 256 % 62 = 8, so the first 8 alphabet characters are ~1.6% more likely than the rest. Immaterial for the throwaway DEMO material (D-01) and real key material comes from `authdomain.local`, so this is recorded, not demanded.
**Fix:** Rejection-sample (`if(r[i] < 248) use it; else redraw`) or note the bias as accepted for DEMO-only use.

#### IN-04: Necessity — tick-hook infrastructure feeds a value that provably nothing reads

**File:** `../r9/port/src/devcap.rs:84-90,109-123,216`; registration `../r9/x86_64/src/main.rs:461-465`
**Category:** necessity
**Issue:** The donor's `Caphash.ticks` is written by `addcap` and consulted by nothing upstream (verified: the only other reference in donor devcap.c is the assignment), and the port carries that dead field deliberately (documented, `#[allow(dead_code)]`). But the port goes beyond donor parity: it builds a live `TICK_HOOK` registry with assert-once registration, a `set_tick_hook`/`ticks_now` seam, and a `main.rs` registration under `cfg(dev_cap)` — ~30 lines of live machinery whose only product is a value that cannot be read by anything in either tree. The guarded state ("no tick source") is not a state that matters; the field it feeds is provably inert (devcap.rs's own doc comment is the proof). Judgment-level, not repo-provable-harmful, hence Info: removal is a scope decision against the documented donor-honesty rationale.
**Fix:** Either seed `ticks: 0` with a comment citing donor parity (dropping the hook machinery and the main.rs registration), or keep the field but document the hook as a deliberate test/audit seam in devcap.rs so a later necessity pass does not re-litigate it.

#### IN-05: Whitespace-bearing local-file passwords break the sweep's factotum leg non-obviously

**File:** `build/x86_64/authdomain-gen.c:402-410` (password = rest of line) vs `build/x86_64/auth-sweep.rc:324-325` (`if(~ $#authpw 1)`)
**Issue:** `parsefile`'s `user <name> <password>` form accepts any bytes after the second field, including spaces; `authdomain.example` documents no restriction. The sweep captures the feed with rc's default `ifs`, so a password containing whitespace splits into multiple words, `$#authpw` != 1, and the factotum leg is skipped with `AUTH-FAIL factotum` — loud, but the operator's path from that marker back to "my password has a space" is long. `genpassword` never generates such passwords (alnum only), so the DEMO path is unaffected.
**Fix:** Cheapest is at the generator: reject passwords containing IFS-class bytes (`' '`, `\t`) in `adduser` with a named error, since the committed interface cannot express them to the sweep anyway.

## Gap-Closure Round (18-06..18-08) — Second Review

**Scope:** the three kernel fixes in r9 (`b2a6151..9103522`: the CR-01 hostowner gate 652f025, the CR-02 RDRAND rebind f594b4c, the 18-07 two-write TCP fix ab4cb66, plus the FOD re-wire 9103522) and the Harvey-side gate/sweep/CI/doc changes + the lock-only pin advance (`0ba03e3..dc990a4`).

**Verified sound this round:**

- **CR-01's fix (devcons.rs:584-603):** iseve-first ordering with the lock discipline called out correctly (iseve takes HOSTOWNER under its own lock; the store re-takes it after — no self-deadlock, no TOCTOU under the single-CPU serialized-syscall invariant); the donor's exact bound (`n <= 0 || n >= 128` over `char buf[128]`) including the 127-byte boundary; three host pins (non-eve refusal with store-untouched, the bound + boundary, the eve path with the documented fail-closed rename divergence). The randread HOSTOWNER-REFUSED leg (randread.c:132-150) refuses-by-writing (open succeeds, write refuses) which is the correct shape for a write-time gate, and the xtask finish() presence list (CAP_HOSTOWNER_REFUSED_MARKER) closes the loop host-side.
- **CR-02's fix (entropy.rs:107-141):** `cpuid(1,0)`'s ECX feeds `has_rdrand` (leaf-1 ECX bit 30), `cpuid(7,0)`'s EBX feeds `has_rdseed` (bit 18, max-leaf-first) — verified against `cpu::cpuid`'s `(eax, ebx, ecx, edx)` return (cpu.rs:164-185). The TEST_SOURCE override retirement is the right call: the scripted-cpuid seam (`push_cpuid`, panic-on-unscripted-leaf) makes the probe pins exercise the real probe→classify composition, and pin 2 (leaf-7 ECX bit 30 must NOT arm Rdrand) is exactly the regression pin for the original mis-wiring.
- **The expect flip (smoke-r9-auth.expect):** a strict strengthening, not a weakening — six new ordered stages (secstored-auth/put, saved, get, bytesback, verdict) plus six whole-capture `string first` presence checks replace the old skip stages and the verdict-absent post-pass; every pre-existing tooth (nvram-prompt-absent, zero-AUTH-FAIL via both `expect_before` and the post-pass, PAIRED-DEMO label, ANNOUNCE, DENIED-exactly-once, tr-ok, no-speaks-for, redaction running last over the true capture) stays armed. The bracketed syslog tokens are correctly regex-escaped in stages and correctly `string first` (not `string match`) in the post-pass.
- **The sweep's restored 9d (auth-sweep.rc:469-484):** honors the measured in-guest rc dialect — `-s 127.0.0.1` bears no `=` token, the capture backquote rides the standing newline-only `ifs`, the verdict fires only on the delivered-bytes grep (`AUTH-ROUNDTRIP-PAYLOAD-18-05`), and the terminal `sweepstat` consult includes `rtstat`. The `AUTH-SKIP` family is genuinely retired (no emittable site remains).
- **The lock advance (flake.lock):** r9 rev `9103522c273a0ee1a05c6218c6224a56bc881720` == the r9 checkout head; lock-only, narHash present, consistent with the README/workflow claims.
- **randread.c:** the write-refusal direction is correct (a successful non-eve write prints `substrate: FAIL` — the bypass itself fails fast); `#c/hostowner` device-direct opens are the right pre-namespace form.

### Critical Issues (Gap-Closure Round)

#### CR-03: 18-07 TCP fix applied only to the connect path — accept-pool sockets keep Nagle + delayed ACK, so bulk writes into announced connections hang (the 18-05 stall class, still live)

**File:** `../r9/port/src/devip/proto.rs:2104-2135` (`tcp_socket`, the fix — its ONLY caller is `connect_proto` at proto.rs:565); `../r9/port/src/devip/stack.rs:813-823` (`AcceptPool::spawn_listener` — constructs pool sockets with `tcp::Socket::new` + `set_hop_limit` ONLY); `../r9/port/src/devip.rs:845,886` (`tcp_listen_pop` materializes every accepted conv around the pool-member socket via `fs_new_call_tcp`)
**Issue:** The 18-07 fix sets `set_nagle_enabled(false)` + `set_ack_delay(None)` in `tcp_socket` — but only connect-path sockets are built there. Every ANNOUNCED (listener) conv's accepted sockets are `spawn_listener` products, which leave smoltcp 0.14's stock defaults in place (`nagle: true`, `ack_delay: Some(ACK_DELAY_DEFAULT=10ms)` — smoltcp socket/tcp.rs:617-620). Consequences on x86_64 (the q35auth/secstore venue), where the 1 ms tick never calls `stack::tick_check` (its ONLY wiring is `aarch64/src/platform/virt/timer.rs:130` — the fix commit's own message records this) and every `drive_stack` caller is a devip file op:

1. **Client bulk write > 2048 (`POOL_SOCK_BUF`) into an announced connection parks forever.** The client's `tcp_write_data` fills its tx buffer and blocks in the `W::Block` arm; the freeing pure ACK is the SERVER socket's, which is delayed 10 ms and only egresses in a poll at/after `now+10ms` (smoltcp tcp.rs:2255-2275, 2880). With both procs devip-parked no poll ever runs, and the armed net-deadline wake (`tick_check`) is unwired on x86_64. The immediate-ACK force-expiry needs more than one remote MSS of unacked data (tcp.rs:2401-2407) — impossible with a 2048 tx buffer against the loopback MSS (~16344, MTU 16384). This is a secstore PUT of any real file — the protocol's entire purpose (the committed DEMO object is 401 bytes, which is the only reason the gate is green twice). On aarch64 the same shape self-resolves in ~10 ms via the tick wake; on x86_64 it is a permanent hang.
2. **The fix's own stated contract is not delivered.** `tcp_socket`'s doc comment claims "the 10ms delayed ACK also gated tx-buffer drain: a >2048-byte bulk write would block in the send arm on the same never-driven timer — Phase 19's cpu path rides this seam" — that drain is closed only for connect-path sockets. The server-side small two-write reply pattern (PAKserver) survives today solely because the CLIENT's now-immediate ACK egresses during the server's own pre-block drive on the shared loopback core — an accident of the loopback topology, not the donor shape.

The pin (`tcp_two_write_client_pattern_round_trips`, devip_tests.rs:2104-2163) writes only from the CLIENT (connect-path) conv and only 2+31 bytes, so it cannot see either hole — see WR-04.
**Fix:** Route pool members through the same donor shape — either move `tcp_socket` to a shared constructor both paths call, or duplicate the two calls:

```rust
// stack.rs — AcceptPool::spawn_listener
let mut s = tcp::Socket::new(
    tcp::SocketBuffer::new(alloc::vec![0; POOL_SOCK_BUF]),
    tcp::SocketBuffer::new(alloc::vec![0; POOL_SOCK_BUF]),
);
s.set_hop_limit(Some(255));
s.set_nagle_enabled(false);   // 18-07: the donor's TCP shape — every devip tcp
s.set_ack_delay(None);        // socket, pool members included (CR-03)
s.listen(port).map_err(|_| "devip: listen failed")?;
```

(`proto::tcp_socket` itself is `fn`-private with a `ttl` parameter the pool doesn't carry; the least-noise form is making it `pub(crate)` with a ttl of `conv::MAX_TTL`-shaped default, or hoisting the two `set_*` calls into a `pub(crate)` helper both sites call.)

### Warnings (Gap-Closure Round)

#### WR-03: Hostowner write drops the donor's rename — a single write is an unrecoverable per-boot eve lockout

**File:** `../r9/port/src/devcons.rs:584-603` (the 18-06 gate + store)
**Issue:** The gate and bound are donor-faithful, but the donor's success path also renames the writer (`renameuser(eve, buf); kstrdup(&eve, buf); kstrdup(&up->user, buf)` — sys/src/9k/port/sysauth.c:148-165 and sys/src/9/port/auth.c:124-139, identical), which is what keeps the WRITER eve after the base moves. The port deliberately skips the rename (documented "no in-tree writer exists... fail-closed"), and the store keeps the raw bytes — including any trailing newline. Combined state machine: after ANY successful hostowner write, `current_user()` (the writer) no longer equals the new base, so nobody is eve; the gate then refuses every future hostowner write (needs iseve); and no user can ever be minted to match the new base (capuse requires a caphash post, which requires eve — circular; and the natural `echo bootes > /dev/hostowner` form stores `"bootes\n"`, which no mintable user string can equal). The donor cannot reach this state — its writer stays eve and can always write again. The port's comment defers the rename "to the first real hostowner-write consumer", but the deferral leaves a one-way trap: the file is write-once-per-boot, and the natural first thing an operator or boot script does with it (`echo name > /dev/hostowner`, the universal Plan 9 idiom) permanently ends eve for the boot.
**Fix:** Port the donor's rename on the success path — one call through the existing hook:

```rust
stored.clear();
stored.extend_from_slice(buf);
let _ = set_current_user(buf.to_vec()); // the donor's kstrdup(&up->user, buf): the writer stays eve
Ok(buf.len() as u64)
```

(or, if the rename stays deferred, at least strip a trailing `b'\n'` from the stored base so the stored name is a mintable user string and the lockout is not aggravated by the echo idiom — and say so beside the existing fail-closed comment).

#### WR-04: The two-write pin covers only the fixed half — no server-side pattern and no bulk-drain leg (CR-03's blind spot)

**File:** `../r9/port/src/devip_tests.rs:2104-2163` (`tcp_two_write_client_pattern_round_trips`)
**Issue:** The 18-07 pin writes the count+payload pair only from the CLIENT conv — whose socket comes from the fixed `tcp_socket` — and only at SConn size (2 + 31 bytes). Nothing pins (a) the accepted/server conv's own write pattern (the pool socket's Nagle hold, released today only by the client's immediate ACK on the shared loopback core), or (b) a client write larger than `POOL_SOCK_BUF` (2048) against the pool socket's delayed ACK — exactly the leg whose absence let CR-03 ship green with a "RED->GREEN" claim. The harness already has the right teeth: the `set_on_block` park bound converts either stall into a loud pin failure with the 18-07 mechanism diagnosis.
**Fix:** Extend the pin (or add a sibling) with two legs against the same announced pair: (1) after the client reads, have the SERVER conv write its own 2+31 count+payload pair and the client read it back in order; (2) a client write of, say, 5000 bytes before any read, then the server reads it back in order — both under the standing ON_BLOCK panic. With CR-03's fix they stay green; without it, (2) parks and the pin names the mechanism.

### Info (Gap-Closure Round)

#### IN-06: Sweep 9d's copy redirect is fatal-on-failure with mislocalizing timeout evidence

**File:** `build/x86_64/auth-sweep.rc:471` (`$B/hcat /boot/secstore-obj.txt > /adm/secstore/secstore-obj.txt`)
**Issue:** rc's redirect-open failures are FATAL to a sourced sweep (the file's own leg-3b rationale), so if this create into the ramfs venue ever fails, the sweep dies at the line with no AUTH-FAIL marker, and the host gate's next pending stage (`secstored-auth`, 600 s TCG-sized) times out naming the wrong leg. The live venue probe at leg 3b (create+write+read through the same mount) bounds the risk to "ramfs broke between 3b and 9d", which is small — but the failure mode is a 600-second mislocalized red, the exact shape the sweep's fail-loud design exists to avoid.
**Fix:** Either stage the copy through a `$B/h*` utility whose exit status can branch into `echo AUTH-FAIL secstore-copy` (if a copy utility exists in the tier), or keep the redirect and note the residual explicitly in the 9d comment (the 3b probe is the bound; the timeout will name `secstored-auth`, not the copy).

#### IN-07: README probe note still names the previous r9 pin as current

**File:** `build/x86_64/README.md:359-365` ("The r9 pin relationship: every emitted netdev argv this era comes from Harvey's `flake.lock` pin `d1c4849dfa1d769240630c29b9b03b88d5819558` ...")
**Issue:** The pin advanced to `9103522` this round (flake.lock, in scope) and the same README's new "closed blocker" section names `9103522` — but the older probe-evidence note still states the current pin in present tense as `d1c4849`. Anyone auditing the emitter-supply chain from that paragraph checks the wrong rev. (The surrounding facts — the emitter export arriving at the 16-04 advance, the `r9-netdev-args-d1c4849` argv echo in the 16-04 capture — are historical and correct; only the present-tense framing is now wrong.)
**Fix:** Reword to past tense tied to the 16-04 era ("the 16-04 boot capture's echoed argv carried `r9-netdev-args-d1c4849`; the pin has since advanced — see flake.lock") or update the rev and keep the derivation-independence rule sentence.

---

_Reviewed: 2026-11-14T17:23:39Z (gap-closure round appended; Round 1 reviewed 2026-11-14T07:26:48Z)_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
