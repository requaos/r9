---
phase: 07-r9-pci-ecam-virtio-net-devether
reviewed: 2026-11-04T00:00:00Z
depth: standard
files_reviewed: 43
files_reviewed_list:
  - build/aarch64/mkimage.sh
  - build/aarch64/netblast.c
  - build/aarch64/root-tree/lib/net-sweep.rc
  - build/aarch64/root-tree/lib/test/arp.req
  - build/aarch64/run-r9.sh
  - build/aarch64/smoke-r9.expect
  - build/aarch64/subset-build.sh
  - nix/boot-r9.sh
  - ../r9/aarch64/lib/config_virtexec3.toml
  - ../r9/aarch64/lib/config_virtexec.toml
  - ../r9/aarch64/lib/config_virtlowecam.toml
  - ../r9/aarch64/lib/config_virtmulti.toml
  - ../r9/aarch64/lib/config_virtnet.toml
  - ../r9/aarch64/lib/config_virtnotes.toml
  - ../r9/aarch64/lib/config_virtpanic.toml
  - ../r9/aarch64/lib/config_virtshell.toml
  - ../r9/aarch64/lib/config_virt.toml
  - ../r9/aarch64/lib/virt-gicv2-lowecam.dts
  - ../r9/aarch64/src/deviceutil.rs
  - ../r9/aarch64/src/io.rs
  - ../r9/aarch64/src/main.rs
  - ../r9/aarch64/src/platform/virt/gic/gicv2.rs
  - ../r9/aarch64/src/platform/virt/gic/gicv3.rs
  - ../r9/aarch64/src/platform/virt/mod.rs
  - ../r9/aarch64/src/platform/virt/pci.rs
  - ../r9/aarch64/src/proc.rs
  - ../r9/aarch64/src/syscall.rs
  - ../r9/aarch64/src/trap.rs
  - ../r9/aarch64/src/vm.rs
  - ../r9/Cargo.toml
  - ../r9/HowItWorks.md
  - ../r9/port/src/dat.rs
  - ../r9/port/src/devether.rs
  - ../r9/port/src/fdt.rs
  - ../r9/port/src/lib.rs
  - ../r9/port/src/namec.rs
  - ../r9/port/src/netif.rs
  - ../r9/port/src/virtio.rs
  - ../r9/port/src/virtio_tests.rs
  - ../r9/xtask/src/config.rs
  - ../r9/xtask/src/main.rs
  - ../r9/xtask/src/pcap.rs
findings:
  critical: 1
  warning: 8
  info: 5
  total: 14
status: issues_found
---

# Phase 7: Code Review Report

**Reviewed:** 2026-11-04
**Depth:** standard
**Files Reviewed:** 43 (8 Harvey-side shell/rc/C/binary fixtures, 35 r9-side Rust/TOML/DTS/markdown)
**Status:** issues_found

## Summary

The phase's core engineering holds up under adversarial reading. The unsafe-free
claims are real and mechanically enforced: `port/src/virtio.rs`, `netif.rs`, and
`devether.rs` each carry `#![forbid(unsafe_code)]`, re-asserted per-module in
`port/src/lib.rs`, and no `unsafe` token appears in any of them. The ring barrier
discipline matches the virtio spec's publication/acquire points
(§2.7.13 descriptor→avail-entry→Release→idx→Release→flags→notify;
§2.7.14 idx-read→Acquire→entry-reads), and `fence(Release)`/`fence(Acquire)`
lower to `dmb ish`/`dmb ishld` on aarch64 — full/load barriers adequate for
DMA against the DTB's `dma-coherent` pci node. The IRQ ack ordering is correct at
both levels: GICC_IAR/ICC_IAR1 ack in `dispatch_irq`, the virtio ISR byte read
FIRST in the handler (read-to-ack deasserts the level line at the source), then
EOIR — the level-triggered handshake is right. The pcap parser is bounds-checked
throughout with loud truncation errors and real negative tests. The shell/expect
scripts quote correctly; no injection or word-splitting bugs found.
`netblast.c` conforms to style(6) (tabs, `/* */`, `exits(nil)`, `-1`/`%r`
idioms, no auto-init at declaration).

The defects that do exist cluster where the phase's own "untrusted device" and
"never silent" postures meet edge validation: one user-triggerable kernel panic
in devether's number-file formatting (CR-01), and a family of capability-walk /
BAR / ring-index validation gaps that turn a "loud bind failure" into a kernel
panic or silent misbehavior on a malformed device (WR-01..WR-04, WR-06). One
stale invariant comment in netif.rs now masks a real silent wake-loss bug
(WR-05, MAX_PROCS grew 16→64 under it). One rc argument-handling hang (WR-07).

## Critical Issues

### CR-01: `readnum_content(-1)` panics the kernel — `connect -1` is an admitted ctl verb

**File:** `../r9/port/src/devether.rs:479-494` (read site `:521-525`)
**Issue:** `write_ctl` explicitly admits `connect -1` (the range check
`typ < -1 || typ > 0xffff` at devether.rs:678 passes -1), so a conversation's
`typ` can legally be -1. Reading that conversation's `type` file then calls
`readnum_content(i64::from(-1))`:

```rust
let v = val as u64;              // -1 -> 18446744073709551615: 20 digits
...
let pad = NUMSIZE - 1 - digits.len();   // 11 - 20: usize UNDERFLOW
out[pad..NUMSIZE - 1].copy_from_slice(&digits);  // slice panic either way
```

In a debug (overflow-checks) build the subtraction panics; otherwise `pad` wraps
to ~2^64 and the slice index panics. With `panic = "abort"` in both kernel
profiles, a userspace `echo connect -1 > /net/ether0/0/ctl` followed by
`cat /net/ether0/0/type` is a **user-triggerable kernel panic**. The function's
own comment claims the donor's shape, but the donor's `readnum` takes a
`u32int` (Plan 9 `long` is 32-bit), so -1 reads back as `4294967295` (10 digits,
fits NUMSIZE=12) — the port sign-extends to 64 bits, which is both the wrong
value and the panic.
**Fix:**
```rust
fn readnum_content(val: i64) -> Vec<u8> {
    // Donor readnum: the int passes through a u32int parameter, so -1
    // reads back as 4294967295 (10 digits — fits NUMSIZE).
    let v = val as u32 as u64;
    // ...unchanged...
}
```
Equivalently guard `digits.len() > NUMSIZE - 1` with a clamp; the u32 cast is
the donor-faithful fix. Add a host test pinning `connect -1` → type-file
readback = `4294967295`.

## Warnings

### WR-01: Capability extent validation admits zero-length/undersized regions — first register touch panics instead of a loud bind-fail

**File:** `../r9/aarch64/src/platform/virt/pci.rs:573-605`; panic site `../r9/aarch64/src/io.rs:22,46` (`.expect("offset outside bounds")`)
**Issue:** The T-07-11 cap checks are only `t.bar == 4` and
`offset + length <= bar_size`. A cap with `offset == bar_size, length == 0`
passes both; `at()` then builds an **empty** VirtRange, and the first access
(e.g. `negotiate()`'s reset write to the common cfg, or the IRQ handler's
`read_reg8(&state.isr, 0)`) fails `offset_addr` and **panics in driver/IRQ
context**. Minimum viable extents are never checked either: the common cfg must
cover the register block through `CFG_QUEUE_DEVICE`+8 (0x38), the isr cap needs
≥1 byte, and — separately — `queue_notify_off × notify_off_multiplier` is never
validated against the notify window's length, so a device advertising a short
notify region panics on the first `notify.write_u16(notify_off, …)`. On qemu all
these hold, so this is latent; on a lying device it converts the module's
documented "aborts the bind loudly" contract into a kernel panic.
**Fix:** At bind, reject: `common.length < 0x40` (or the highest offset used),
`isr.length < 1`, `notify.length < queue_notify_off × multiplier + 2` per
enabled queue, and `offset + length == 0`/empty extents generally. These are
cheap integer checks beside the existing extent checks at pci.rs:573-590.

### WR-02: device-config cap admits `length >= 6`; the link-status read at offset 6 needs 8

**File:** `../r9/aarch64/src/platform/virt/pci.rs:584` (check) and `:949` (read)
**Issue:** The bind accepts the device cap when `d.length >= 6` — six bytes
covers the MAC (offsets 0..5). But the IRQ config-change arm reads the link
`status` le16 at offset 6 (`io::read_reg16(&state.devcfg, 6)`), which requires
bytes 6..8. A cap of length 6 or 7 passes the bind check and then panics
(`offset_addr` → `.expect`) **inside the IRQ handler** on the first
config-change interrupt. qemu's virtio-net config is ≥8 bytes, so this is
latent, but the validation is off by two from the driver's own usage.
**Fix:** Change the bind predicate to `d.length >= 8` (MAC 6 + status 2), with
the error string naming the requirement.

### WR-03: BAR4 type never validated; window size ignored at assignment

**File:** `../r9/aarch64/src/platform/virt/pci.rs:372-385` (`size_bar64`), `:534-557` (assignment)
**Issue:** Two gaps in the BAR path. (a) `size_bar64` writes all-ones to BAR4
and BAR4+4 without first checking the BAR's type bits — if BAR4 were a 32-bit
memory BAR or an I/O BAR, the high-dword sizing write lands in BAR5 (or
meaningless I/O space) and the computed size is garbage. The flags nibble
(`mask_lo & 0xF`) is never examined: bit 0 must be 0 (memory) and bits 2:1 must
be 0b10 (64-bit) for the 64-bit sizing to be meaningful. (b) The DTB 64-bit
MMIO window is parsed as `(win_base, _win_size)` and the size discarded — BAR4
is assigned to `win_base` with no `bar_size <= win_size` check and no
`win_base % bar_size == 0` (natural-alignment) check; a BAR larger than the
window would be assigned into space the DTB does not route. Both are silent on
qemu's real device; both are exactly the "device fault → loud" class the module
documents.
**Fix:** After sizing, validate `mask_lo & 1 == 0 && (mask_lo >> 1) & 3 == 0b10`
(memory + 64-bit); at assignment, `if bar_size > win_size || win_base % bar_size != 0 { return Err(...) }`.

### WR-04: Capability-walk pointers never alignment-checked — a misaligned cap pointer is an unaligned volatile u32 read

**File:** `../r9/port/src/virtio.rs:137-175` (`Caps::parse`); UB site `../r9/aarch64/src/io.rs:26,49` via `EcamCfg::read_u32`
**Issue:** The walk bounds-checks `p + CAP_BASE_LEN > cfg.len()` but never
checks `p & 3`. PCI capability pointers are dword-aligned by spec, but the
module's own V5 posture treats the device as untrusted: a malformed device
returning a misaligned list pointer makes `cfg.read_u32(p + CAP_OFF_OFFSET)` an
unaligned `read_volatile(… as *const u32)` — UB in Rust, and on aarch64 Device
memory an alignment fault → EL1 data abort → kernel panic, instead of the
promised loud parse error. Same for the `next` pointer chaining.
**Fix:** At the top of the loop: `if p & 3 != 0 { return Err("virtio: misaligned capability pointer"); }`. (While here: the notify arm checks `p + CAP_NOTIFY_LEN > cfg.len()` but not `cap_len >= CAP_NOTIFY_LEN` — a notify cap declaring `cap_len == 16` has its multiplier read out of the *next* cap's bytes. Reject that too.)

### WR-05: `WaitSet` capacity is 16 on a stale "MAX_PROCS=16" invariant — the table is now 64, and the full case silently drops a waiter

**File:** `../r9/port/src/netif.rs:315-335` (`struct WaitSet { pids: [u64; 16] }`); contradicts `../r9/aarch64/src/proc.rs:67` (`MAX_PROCS: usize = 64`)
**Issue:** The comment reads "one waiter per proc, ≤ 16 procs" and
`register()`'s full arm silently does nothing ("The full case is impossible").
MAX_PROCS is now 64 (bumped for the 07-07 sweep), so the invariant the capacity
rests on is false: with >16 procs blocked on `#l` reads/TX-full writes, the
17th registration is dropped and that proc is **never woken** — a silent wedge,
the precise failure mode this module's header and the D-08 wedge-hunt posture
forbid. The gates never block that many readers (OQ-1), so this is latent, but
the code now provably contradicts its own comment.
**Fix:** Size the sets to the proc table (`[u64; 64]`, ideally a shared const),
and make the full arm count a `soverflows`-style loud drop so a violation is
visible rather than silent. Fix the comment.

### WR-06: RX retire never checks the slot was actually posted — a duplicated used-ring id aliases one buffer page into two chains

**File:** `../r9/port/src/virtio.rs:753-756` (slot recovery) + `../r9/aarch64/src/platform/virt/pci.rs:886-902` (free-stack push)
**Issue:** `next_used`'s V5 validation checks `id < qsize`, evenness, and len
bounds, then unconditionally `posted[slot] = false`. A faulty/malicious device
returning the **same** used id twice passes all checks; the second retire takes
the `Some(Ok(used))` arm, and pci.rs pushes `rx_buf_pa[slot]` into `rx_free` a
second time. `rx_replenish` then hands that one page to two different slots —
two live RX descriptor chains DMA into the same 4 KiB page (frame corruption),
plus duplicate frame delivery. The module header says the device is untrusted
input (V5); slot-liveness is the missing check in that validation set.
**Fix:** In `next_used`, before clearing: `if !self.posted[slot] { return Some(Err("virtio: used-ring id for an unposted slot (device fault)")); }`. Host-test it in `virtio_tests.rs` beside the existing out-of-range/odd-id negatives.

### WR-07: net-sweep.rc's count validation admits leading zeros that the arp loop can never match — infinite loop

**File:** `build/aarch64/root-tree/lib/net-sweep.rc:35-42` (validation), `:74-81` (loop)
**Issue:** The count check admits `[0-9][0-9]` and `[0-9][0-9][0-9]`, so `04` is
accepted. The arp loop terminates on `while(! ~ $#i $n)` — a *string* match
between rc's `$#i` (canonical decimal: `4`) and the user's `n` (`04`), which
never compare equal. `net-sweep.rc arp 04` loops forever (the gate's timeout
makes it loud eventually, but the sweep itself wedges — the script's own header
says "a gate must never hang silently"). blast mode is unaffected (netblast's
`atoi` canonicalizes).
**Fix:** Reject leading zeros in the validation (`case 0[0-9]*` → usage), or
canonicalize `n` before the loop (e.g. strip leading zeros via a small
`while`/`switch`) so the `$#i` string comparison can ever match.

### WR-08: Decoded DTB interrupt trigger flags are printed but never honored — ICFGR is always programmed level

**File:** `../r9/aarch64/src/platform/virt/pci.rs:154-162`; `../r9/aarch64/src/platform/virt/gic/gicv2.rs:99-103`; `../r9/aarch64/src/platform/virt/gic/gicv3.rs:231-239`
**Issue:** Discovery decodes the route's flags cell and prints
`level-high`/`edge-rising` accordingly — then both GIC drivers' `enable()`
unconditionally clear the ICFGR trigger bit (level). On the committed DTBs
(flags 0x04) this agrees; on any future DTB advertising edge, the kernel prints
"edge-rising" while programming level — a silent mismatch between the decoded
fact and the programmed state, in a module whose documented posture is "no
silent defaults; qemu defaults never relied on."
**Fix:** Either plumb `flags` into `gic::enable(intid, prio, trigger)` and
program ICFGR to match, or reject non-level routes loudly at discovery
(`pci: irq flags {:#x} not level-high — unsupported`). The latter fits the
current single-machine reality.

## Info

### IN-01: Dead `dd` trailer extraction in mkimage.sh

**File:** `build/aarch64/mkimage.sh:179`
**Issue:** `trailer=$(dd if="$IMAGE" …)` is assigned and then unconditionally
reassigned by the `od -j` form at line 184 before any read — pure dead code
(the comment explains the shadowing history, but the dd line itself is
inert).
**Fix:** Delete line 179; keep the od form and the comment.

### IN-02: `VirtRange::offset_addr` bounds-checks only the access start, not start+width

**File:** `../r9/port/src/mem.rs:29-32`; used by all width accessors in `../r9/aarch64/src/io.rs`
**Issue:** `offset_addr(offset)` checks `start <= addr < end` but the io.rs
callers then perform 2/4/8-byte volatile accesses at `addr` — an access at the
final bytes of a range silently exceeds the VirtRange's stated extent. The
doc comments ("Panics if offset is outside any range") overstate the check.
Practically harmless today (every MMIO/ring range is page-backed, so the access
stays inside the mapped page), but the helper's contract is wrong for the
multi-byte accessors built on it.
**Fix:** Either document the start-only semantics at `offset_addr` and the io.rs
wrappers, or take a width parameter (`offset_addr(offset, width)`) and check
`offset + width <= size`.

### IN-03: Documentation drift — proc-table/fd-table sizes and a stale virtnet header comment

**File:** `../r9/HowItWorks.md:64`; `../r9/aarch64/lib/config_virtnet.toml:19-21`
**Issue:** HowItWorks.md still says "PROC_TABLE is a fixed 8-proc array" and
"a 16-slot fd table"; proc.rs is now `MAX_PROCS = 64` / `MAX_FDS = 32` (and the
doc's own 05-06 section records the 8→16/16→32 bumps, so the doc disagrees with
itself). Separately, config_virtnet.toml's header says "the first kernel CODE
use lands in 07-06, so until then this kernel is behavior-identical to
virtshell's" — stale since 07-06/07-07 landed.
**Fix:** Update the numbers and drop the transitional comment.

### IN-04: `DescPool::free` has no bounds or duplicate-free checks

**File:** `../r9/port/src/virtio.rs:462-466`
**Issue:** `free()` indexes `free[self.len as usize]` with no `idx < qsize`
check and no `len < qsize` guard — a double-free or out-of-range index panics
or corrupts the pool. Currently test-only (`#[allow(dead_code)]`), so no live
impact, but the pool is the ring's index-arena foundation and a future
queue-reset path will call this for real.
**Fix:** Add `assert!(idx < self.qsize_capacity && (self.len as usize) < MAX_QSIZE as usize)`-style guards (store qsize on the pool) before the write.

### IN-05: netblast's `atoi` silently accepts trailing garbage

**File:** `build/aarch64/netblast.c:52-58`
**Issue:** `atoi("12x")` yields 12 with no error — a typo'd count is silently
truncated rather than rejected. The sanctioned path (net-sweep.rc) pre-validates
digits, so the gate is clean; standalone use accepts junk. (Note also `atoi`
returns `int` while `n` is `long` — fine at the TCG-sane scales in use.)
**Fix:** Optional: parse with `strtol` + end-pointer check and reject junk with
the existing "bad count" path; or document the sweep as the only caller.

---

## Incremental Review — 07-09 gap closure

**Reviewed:** 2026-11-05
**Depth:** standard
**Scope:** the 07-09 source-change set only — `../r9/port/src/devether.rs`
(the GREEN fix: `readnum_content` + doc comment, r9 commit `1c16193`) and
`../r9/port/src/devether_tests.rs` (the RED pins, r9 commit `1e77266`).
**New findings:** 0 (critical 0, warning 0, info 0). Frontmatter counts above
are unchanged: the union of findings is the original 14; CR-01's resolution by
this plan is recorded in the disposition ledger, not here.

### Verdict

The CR-01 fix is correct, donor-exact over the full admitted domain, and the
new tests assert what they claim. Adversarial pass found no new defects.

### Verification detail (the four review questions)

1. **Donor-exactness over the admitted domain.** Verified against the in-tree
   donors: `sys/src/9k/port/devcons.c:712` is
   `readnum(ulong off, char *buf, ulong n, ulong val, int size)` with
   `snprint(tmp, sizeof(tmp), "%*lud", size-1, val); tmp[size-1] = ' ';`;
   `sys/src/9k/port/netif.h:40` is `int type;` inside `struct Netfile`;
   `portdat.h:780` is `NUMSIZE = 12`. `val as u32 as u64` reproduces the C
   int→ulong conversion bit-for-bit. `write_ctl` admits `typ ∈ [-1, 0xffff]`
   (devether.rs:693); the u32 view of that domain is {4294967295} ∪
   [0, 65535], at most 10 digits against NUMSIZE-1=11 columns — every value
   right-justifies with ≥1 pad space and the trailing space, byte-identical
   to the donor's `%*lud` + overwrite shape. Boundary 0xffff →
   `"      65535 "` (6 pad + 5 digits + space) is pinned; -1 →
   `" 4294967295 "` is pinned byte-exact; the NCTLQID caller's ids 0..7 are
   unaffected by the cast change (non-negative values are identical under
   both the old and new code).

2. **Remaining underflow/overflow paths in `readnum_content`.** None. For
   *any* i64 input, `as u32 as u64` ≤ 4294967295 → `digits.len() ∈ [1,10]` →
   `pad = NUMSIZE-1-digits.len() ∈ [1,10]` — the subtraction provably cannot
   underflow, and the slice copy's lengths are equal by construction. The
   doc comment's "do NOT add a saturating clamp" posture is consistent with
   the repo's loud-failure convention (a clamp would silently mask a future
   width regression) and is safe precisely because the truncation bounds the
   digit count.

3. **Tests assert what they claim.** Every netif.rs citation in the test
   comments was re-checked against the source: `WaitSet.register` drops
   pid==0 (netif.rs:328 — hence the nonzero `STUB_PID`); `conv_alloc` is the
   only store allocator and sets inuse=1 (417-420); `conv_open` errors on a
   store-less slot (444-446); `conv_close` decrements (487-488) and the
   inuse==0 reset arm (490-508) clears typ/owner, drains the store queue,
   and takes the rx waiter set; the multicast gate (589-593) and demux match
   (599-613) deliver the pinned 2054 frame; the waiter-take at 624-625
   justifies registering after the pushes; `pop_frame` pops the store before
   the open check (685-690), so `Pop::Closed` genuinely proves drained +
   closed. `Pop`'s variants match the test's matches; the `NETIF → convs →
   fields` chain is pub(crate)-visible from the crate-root test module.
   `cargo test -p port --lib`: all 4 devether tests pass (51 lib tests
   total, 0 failed). RED was genuinely RED: at `1e77266` the code has
   `val as u64`, so `readnum_content(-1)` panics on the `11 - 20` usize
   underflow — the pin fails before the fix. GREEN touched only the cast +
   comment; no assertion was weakened.

4. **Panic-introducing constructs reaching the kernel.** None.
   `devether_tests` is `#[cfg(test)]`-gated (lib.rs:82-83) and lib.rs:3's
   `#![cfg_attr(not(any(test)), no_std)]` means the module (and its
   `std::sync::Mutex` wake log) vanishes from kernel builds. The kernel-side
   change *removed* the only panic path rather than adding one; the
   publish-once/hook-once asserts the test relies on are pre-existing boot
   invariants, and grep confirms this test is the only publish/set_hooks
   caller in the test binary, so the asserts cannot fire from test
   parallelism.

### Notes (not findings)

- `cargo test --workspace` fails to compile the `aarch64`/`riscv64`/`x86_64`
  no_std kernel crates under the host test harness (missing `std`/`test`
  crates). This is a pre-existing repo condition untouched by 07-09 (the
  diff is port-only); the sanctioned lane is per-crate `cargo test -p port`,
  matching the virtio_tests precedent.
- `assert_eq!(id, 0)` in the conv_close pin depends on remaining the only
  netif-state-mutating test in the binary. True today (grep-verified) and
  explicitly documented in the test header; a future second netif test must
  respect the publish-once constraint.

---

## Incremental Review — 07-10 RX offset fix

**Reviewed:** 2026-11-05
**Depth:** standard
**Scope:** the 07-10 source-change set only — `../r9/aarch64/src/platform/virt/pci.rs`
(the GREEN fix: RX drain slice base, ~:889-903), `../r9/port/src/virtio.rs`
(the extracted `pub const fn virtio_net_frame_offset()` seam, :835-848), and
`../r9/port/src/rx_chain_tests.rs` (new — the chain-faithful RED pins).
**New findings:** 0 (critical 0, warning 0, info 0). Frontmatter counts and
file list above are unchanged: the union of findings remains the original 14,
and the newly-reviewed file's scope is recorded here per the 07-09 precedent.

### Verdict

The G-07-1 fix is correct and spec-exact: the two-descriptor chain (12-byte
virtio_net_hdr descriptor NEXT-chained to the payload descriptor, virtio.rs
`setup_chains` :546-565) makes the descriptor boundary a hard I/O-vector
boundary per §5.1.6.3.2, so the device writes the frame at the payload page's
offset 0 and only `used.len` carries the +12. Returning 0 from the extracted
seam is the right cure, and the new pins genuinely assert what they claim.
Adversarial pass found no new defects.

### Verification detail (the four review questions)

1. **Offset 0 is correct for every caller.** Grep-verified:
   `virtio_net_frame_offset` has exactly ONE production caller — the RX drain
   at pci.rs:899 — plus the test's mirrored `drain_slice`. No TX-side or other
   consumer exists that could legitimately need 12: `virtnet_tx_post`
   (pci.rs:753-779) copies the outbound frame at its payload page's offset 0
   and the TX chain's header descriptor points at the shared header page,
   zeroed at allocation (`alloc_ring_page` zero-fills, pci.rs:457) — which is
   exactly `VirtioNetHdr::tx().to_le_bytes()` under the D-03 zero-offload mask
   (pinned at virtio_tests.rs:753-758). `VirtioNetHdr::to_le_bytes` keeps its
   test consumers, so the seam extraction introduced no dead code. The 12-byte
   (not 10-byte legacy) header form is guaranteed by D-03's VERSION_1-only
   negotiation, so no MRG_RXBUF/legacy path can reintroduce a header-in-buffer
   layout (MRG_RXBUF is never accepted — ACCEPT_LO/HI, virtio.rs:220-222).

2. **The `used.len − 12` math cannot underflow.** For RX rings,
   `Ring::new` sets `used_len_min = VIRTIO_NET_HDR_LEN` (virtio.rs:518-521)
   and `next_used` rejects `len < used_len_min` as a device fault
   (virtio.rs:747-749). The drain computes `virtio_net_frame_len(used.len)`
   only inside the `Some(Ok(used))` arm (pci.rs:878-889), so the subtraction
   sees `len ≥ 12` structurally — the helper's documented precondition holds at
   its only call site. The upper side is equally tight: `used_len_max` is the
   chain capacity `12 + 4096` (pci.rs:490), so `frame_len ≤ 4096` and the slice
   `[page+0 .. page+frame_len]` stays within the 4 KiB payload page by
   construction (the SAFETY comment at pci.rs:891-896 is accurate). Boundary
   case `len == 12` yields a 0-byte frame; `rx_ingress`'s runt drop
   (`frame.len() < 14`, netif.rs:585-587) handles it without panic.

3. **The tests assert what they claim.** Traced line-by-line against the real
   `Ring`: `DescPool::new(8)` pops ascending, so slot 0 gets chain head 0 /
   payload 1 — matching test 1's descriptor pins (`addr=HDR_PA, len=12,
   flags=NEXT|WRITE, next=1`; payload `addr=PAY_PA, len=4096, flags=WRITE`
   after replenish). `replenish_slot0`'s `Replenish { offered: 1, notified:
   true }` expectation matches `rx_replenish`'s control flow exactly (one
   post, alloc dries up, closing notify under `USED_F_NO_NOTIFY == 0`).
   `device_complete` correctly models the device's independent used cursor
   (reads `used.idx`, writes the entry at the device position, advances).
   `drain_slice` mirrors pci.rs's arithmetic with *safe* indexing, so the old
   defect fails loudly: offset 12 would byte-mismatch test 2's
   `slice == PCAP_REPLY` and panic out-of-bounds in test 3's `[12..4108]` on a
   4096-byte page — both pins are genuinely RED against the pre-fix
   arithmetic. The `PCAP_REPLY` vector was independently verified byte-exact
   against `evidence/07-07-virtnet-arp.pcap` packet 2 (incl_len 64): dst
   52:54:00:12:34:56, src 52:55:0a:00:02:02, etype 0806, ARP oper 0002, spa
   10.0.2.2, tha guest MAC, tpa 10.0.2.15, 22 zero pad bytes — the provenance
   claim in the test header is true. Executed: `nix develop --offline
   --command cargo test -p port --lib rx_chain` → **3 passed, 0 failed** (51
   pre-existing lib tests filtered out).

4. **No new panic path reaches kernel code.** The kernel-side change adds a
   `const fn` returning 0 to a slice base — no new panic construct; the
   pre-existing bounds argument above covers the slice extent.
   `rx_chain_tests` is `#[cfg(test)]`-gated (lib.rs:87-88) under the crate's
   `#![cfg_attr(not(any(test)), no_std)]` (lib.rs:3), so the Vec/Rc/RefCell
   rig vanishes from kernel builds; `port::virtio` keeps its
   `#![forbid(unsafe_code)]`.

### Notes (not findings)

- Test-fidelity boundaries (deliberate, documented in the module header): the
  device-write model pokes the payload page directly rather than resolving
  PAY_PA through the descriptor table — the composition is still covered
  because test 1 pins the descriptor addresses; the `hdr_page` sentinel write
  is set dressing (never asserted), matching the driver truth that the header
  content is never consumed with MRG_RXBUF off. The netif::rx_ingress leg is
  covered by devether_tests' synthetic drive; the publish-once constraint
  forbidding a second full-path drive is real (devether_tests already consumes
  the single publish).
- WR-06 (unposted-slot retire) is untouched by this fix and unchanged in
  disposition; the drain's free-stack push accounting it threatens is the same
  code path, so its remediation remains the relevant follow-up.
- The `rx_ingress` doc comment ("header already stripped", netif.rs:575-576)
  reads as net-effect accurate post-fix (the delivered frame is header-free);
  pci.rs:881-888 carries the precise never-in-the-buffer explanation.

---

_Reviewed: 2026-11-04_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
