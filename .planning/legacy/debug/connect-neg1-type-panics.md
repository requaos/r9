---
status: investigating
trigger: "Gap G-07-2: connect -1 accepted by ctl; type file read panics at port/src/devether.rs:492:15 (attempt to subtract with overflow) instead of printing 4294967295 per donor semantics. Goal: find_root_cause_only."
created: 2026-11-05T00:00:00Z
updated: 2026-11-05T00:00:00Z
---

## Current Focus
<!-- OVERWRITE on each update - reflects NOW -->

reasoning_checkpoint:
  hypothesis: "r9's readnum_content reinterprets the i32 conversation type as a 64-bit value (i64::from → as u64), so connect -1 formats as u64::MAX (20 digits) and NUMSIZE-1-digits.len() = 11-20 underflows usize; the donor instead passes the int through a 32-bit ulong parameter, truncating -1 to 4294967295 (10 digits, fits NUMSIZE=12)."
  confirming_evidence:
    - "r9 devether.rs:479-494 readnum_content: `let v = val as u64;` on i64 -1 = 18446744073709551615, 20 digits; line 492 `NUMSIZE - 1 - digits.len()` = 11-20 → usize underflow (panic message verbatim: 492:15)"
    - "r9 devether.rs:521-524 NTYPEQID read: `readnum_content(i64::from(t))` — sign-extends the i32 -1 to i64 -1"
    - "r9 netif.rs:135 ConvBook.typ is i32; conv_connect stores -1 after write_ctl's range check (devether.rs:678 `typ < -1 || typ > 0xffff`) admits -1"
    - "Harvey donor sys/src/9k/port/netif.h:40 `int type` (32-bit); netif.c:251 `readnum(offset, a, n, f->type, NUMSIZE)`"
    - "Harvey donor sys/src/9k/port/devcons.c:712 `readnum(ulong off, char *buf, ulong n, ulong val, int size)` — value param is ulong; amd64/include/u.h:4 `typedef unsigned long ulong` and u.h:7 `typedef long long vlong` — Plan 9 long is 32-bit, vlong is the 64-bit type"
    - "Harvey donor sys/src/9k/port/portdat.h:780 NUMSIZE=12 (designed for a 32-bit number: 10 digits + pad + space; 64-bit uses VLNUMSIZE=22, devcons.c:681)"
  falsification_test: "If Plan 9's long were 64-bit on amd64, or readnum's value param were uvlong, the donor would also print 20 digits and the hypothesis collapses. Refuted by u.h:4/7/8/21 (vlong/uvlong/u64int are the 64-bit typedefs; long/ulong are separate) and by VLNUMSIZE=22 existing precisely because NUMSIZE=12 cannot hold a 64-bit value."
  fix_rationale: "`let v = val as u32 as u64;` reproduces the donor's C conversion exactly: i32 -1 → u32 truncation = 4294967295 → 10 digits → pad=1 → ' 4294967295 ' (12 bytes) — byte-identical to the donor's snprint %*lud(11, 4294967295) + trailing space. Fixes the root cause (wrong conversion width), not the symptom (underflow)."
  blind_spots: "Did not boot a donor kernel to observe the literal bytes; the claim rests on C conversion rules + fmt %lud semantics (unsigned 32-bit) which are unambiguous. NCTLQID caller (netid as i64) also gets truncated to u32 by the fix — donor-faithful (NETID macro truncates to ulong first, netif.h:26) and ids are 0..7 in practice."
  candidate_causes:
    - "code: wrong conversion width in readnum_content (i64/u64 instead of donor's 32-bit ulong) — CONFIRMED"
    - "code: stale/wrong doc comment at devether.rs:475-478 claiming 'donor's sign-extended %lud shape' — the donor sign-extends nowhere; it truncates to 32-bit"
    - "data: connect -1 is legal input per donor (netif.c:316-322 admits any atoi result, no range check) and per r9's locked range check — the value -1 must round-trip; not a data-validation bug"
  and_gate: "no — a single condition (the 64-bit reinterpretation) is sufficient; the range check admitting -1 is correct donor behavior, not a contributing defect"
next_action: "Return ROOT CAUSE FOUND diagnosis (goal: find_root_cause_only — no fix implementation)."

## Symptoms
<!-- Written during gathering, then IMMUTABLE -->

expected: `cat /net/ether0/0/type` prints `4294967295` after `echo connect -1 > /net/ether0/0/ctl`
actual: "connect -1" admitted cleanly; the type read panicked: "panicked at port/src/devether.rs:492:15: attempt to subtract with overflow" (panic=abort, kernel halted)
errors: kernel panic at port/src/devether.rs:492:15
reproduction: Test 2 in 07-UAT.md — confirmed live during UAT on 2026-11-05
started: Found by code review (CR-01), confirmed by inspection by phase verifier, reproduced live in UAT

## Eliminated
<!-- APPEND only -->

- hypothesis: "The bug is in the ctl parse or range check (connect -1 should have been rejected)"
  evidence: "Both donor (sys/src/9k/port/netif.c:316-322 — `type = atoi(p)`, no range check, `if(f->type < 0) nif->all++`) and r9 (devether.rs:678 `typ < -1 || typ > 0xffff` deliberately admits -1 per T-07-15) treat -1 as the legal 'all ethertypes' sentinel; the demux in r9 netif.rs:603 (`slot.typ >= 0` gate) honors it. Rejecting -1 would break donor semantics."
  timestamp: 2026-11-05T00:00:00Z
- hypothesis: "The doc comment's claim — donor sign-extends the int through %lud — is correct and the fix is only an underflow guard"
  evidence: "readnum's value parameter is `ulong` (devcons.c:712, portfns.h:285); Plan 9 amd64 `ulong` = `unsigned long` is 32-bit (amd64/include/u.h:4 vs. the separate 64-bit `vlong`/`uvlong`/`u64int` at u.h:7-8,21). C conversion int -1 → 32-bit ulong = 4294967295, never 2^64-1. VLNUMSIZE=22 (devcons.c:681) exists because NUMSIZE=12 cannot hold a 64-bit value. A clamp-only fix would print the wrong value (2^64-1 or a truncated field) — the u32 cast is the donor-faithful fix."
  timestamp: 2026-11-05T00:00:00Z

## Evidence
<!-- APPEND only -->

- timestamp: 2026-11-05T00:00:00Z
  checked: "07-UAT.md test 2 + G-07-2 gap record; 07-REVIEW.md CR-01"
  found: "Live reproduction: connect -1 clean, `cat type` panicked at devether.rs:492:15 'attempt to subtract with overflow' under panic=abort. CR-01 proposed `val as u32 as u64`."
  implication: "Panic site and proposed fix direction pre-identified; my job is to pin donor semantics exactly."
- timestamp: 2026-11-05T00:00:00Z
  checked: "r9 port/src/devether.rs:440-717 (full read/write/ctl path)"
  found: "write_ctl:675-683 parses 'connect' via parse_i32 (697-717: i64 accumulate, i32::try_from) → conv_connect(id, typ) with range check `typ < -1 || typ > 0xffff`. Read NTYPEQID:521-524: `type_of(id)?` → i32 → `readnum_content(i64::from(t))` SIGN-EXTENDS. readnum_content:479-494: `val as u64` → -1i64 = 0xFFFFFFFFFFFFFFFF = 20 digits → `pad = NUMSIZE - 1 - digits.len()` = 11 - 20 → usize underflow at 492 (column 15 = the subtraction). Comment at 475-478 asserts the wrong donor shape ('sign-extended %lud')."
  implication: "Defect chain fully traced: i32 -1 → i64 sign-extension → u64 reinterpretation → 20 digits → underflow. Every step verified in code."
- timestamp: 2026-11-05T00:00:00Z
  checked: "r9 port/src/netif.rs (full, 832 lines)"
  found: "ConvBook.typ: i32 (135); ConvSlot.typ: i32 (269); conv_connect:522-540 stores -1 verbatim; type_of:778-786 returns i32; NUMSIZE=12 at :116 'the donor's NUMSIZE (portdat.h:780)'. No NUMSIZE arithmetic anywhere in netif.rs — stats/addr use unpadded `dec`/hexaddr. Demux:603 `slot.typ != etype && slot.typ >= 0` → -1 matches all (donor's nif->all semantics)."
  implication: "Storage is correct (32-bit signed, donor-faithful). Only the formatting width is wrong."
- timestamp: 2026-11-05T00:00:00Z
  checked: "Harvey donor sys/src/9k/port/netif.c:240-329, netif.h:32-49, devcons.c:711-724, portfns.h:285, portdat.h:780, amd64/include/u.h:1-21"
  found: "Netfile.type is `int` (netif.h:40). Type read: netif.c:251 `readnum(offset, a, n, f->type, NUMSIZE)`. readnum (devcons.c:712): `readnum(ulong off, char *buf, ulong n, ulong val, int size)` — `snprint(tmp, sizeof(tmp), \"%*lud\", size-1, val); tmp[size-1] = ' ';`. ulong = unsigned long = 32-bit on Plan 9 amd64 (u.h:4; 64-bit is vlong/uvlong/u64int, u.h:7-8,21). C conversion: int -1 → 32-bit ulong = 4294967295 (10 digits). Printed right-justified in 11 cols + trailing space = ' 4294967295 ' (12 bytes). NUMSIZE=12 (portdat.h:780). Donor connect (netif.c:316-322): `type = atoi(p)` with NO range check; typeinuse returns 0 for type <= 0; -1 admitted, nif->all++."
  implication: "Donor prints exactly ' 4294967295 ' — 32-bit truncation, no sign extension. The review's u32 cast reproduces this byte-for-byte."
- timestamp: 2026-11-05T00:00:00Z
  checked: "Legacy donor sys/src/9/port/netif.c:248 + netif.h:41"
  found: "Identical: `readnum(offset, a, n, f->type, NUMSIZE)` with `int type` in Netfile. Legacy and 9k donors agree."
  implication: "Semantics are stable across both donor trees — 4294967295 is THE oracle value."
- timestamp: 2026-11-05T00:00:00Z
  checked: "Class sweep: all NUMSIZE arithmetic in r9 port (grep NUMSIZE - / digits.len across port/src)"
  found: "readnum_content (devether.rs:492-493) is the ONLY site with `NUMSIZE - 1 - digits.len()` arithmetic. Its only callers: NCTLQID (510, netid(path) as i64 — ids 0..7, always ≤ 2 digits, cannot underflow) and NTYPEQID (523, the -1 defect). devether.rs:368 uses NUMSIZE only as a stat length. netif.rs has zero NUMSIZE arithmetic."
  implication: "The class is exactly one function with two call sites; the readnum_content fix covers both. NCTLQID gets u32 truncation from the fix — donor-faithful (NETID macro truncates to ulong first, netif.h:26)."
- timestamp: 2026-11-05T00:00:00Z
  checked: "Host-test precedent: port/src/lib.rs:3,72-78; port/Cargo.toml; virtio_tests.rs header"
  found: "port is `#![cfg_attr(not(any(test)), no_std)]` — host tests run under std via cargo test. Crate-root test modules are the precedent (allocator_tests.rs, virtio_tests.rs — 'crate-root placement' documented at lib.rs:67-78)."
  implication: "The pin test belongs in a new crate-root `devether_tests.rs` (or appended to an existing crate-root test module), exercising readnum_content directly."

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: "r9's readnum_content (port/src/devether.rs:479) takes the conversation type as i64 and formats it via `val as u64` (line 481), reinterpreting the sign-extended 64-bit -1 as 18446744073709551615 (20 digits). NUMSIZE-1-digits.len() = 11-20 underflows usize at line 492, panicking (panic=abort → kernel halt). The donor never sign-extends: Netfile.type is a 32-bit int (9k netif.h:40) passed through readnum's 32-bit `ulong` value parameter (devcons.c:712), so -1 truncates to 4294967295 (10 digits, fits NUMSIZE=12). The port's doc comment (devether.rs:475-478) asserts the donor's shape is 'sign-extended %lud' — false; Plan 9's long is 32-bit, vlong is the 64-bit type (amd64/include/u.h:4,7)."
fix: "[fix direction only — implementation deferred to /gsd-plan-phase --gaps] (1) readnum_content: `let v = val as u32 as u64;` (donor-faithful 32-bit truncation; produces ' 4294967295 ' byte-identical to the donor); (2) rewrite the wrong doc comment at devether.rs:475-478; (3) host test pin in a crate-root devether_tests.rs: readnum_content(-1) == b\" 4294967295 \" (12 bytes), plus boundary neighbors readnum_content(0) == b\"          0 \"? — exact shapes: 0 → '0' at pad 10; 65535 (max admitted ether-type) → '      65535 '; (4) optional belt-and-braces: saturating guard `digits.len() > NUMSIZE-1` is NOT needed after the u32 cast (u32 max = 10 digits < 11) — do not add a clamp that would mask a future width regression."
verification: "[for the fix phase] host test red→green; guest UAT re-run of 07-UAT.md test 2: connect -1 then cat type prints 4294967295, no panic; sweep confirms no other NUMSIZE-underflow site (verified: only readnum_content has the arithmetic)."
files_changed: []
