---
phase: 14-harvey-amd64-userspace-on-r9
reviewed: 2026-11-09T12:00:00Z
depth: standard
files_reviewed: 72
files_reviewed_list:
  - ../r9/flake.nix
  - ../r9/x86_64/Cargo.toml
  - ../r9/x86_64/lib/config_q35exec.toml
  - ../r9/x86_64/lib/config_q35shell.toml
  - ../r9/x86_64/lib/grub.cfg
  - ../r9/x86_64/src/cpu.rs
  - ../r9/x86_64/src/dat.rs
  - ../r9/x86_64/src/exec.rs
  - ../r9/x86_64/src/l.S
  - ../r9/x86_64/src/main.rs
  - ../r9/x86_64/src/pagealloc.rs
  - ../r9/x86_64/src/platform/q35/mb2.rs
  - ../r9/x86_64/src/platform/q35/timer.rs
  - ../r9/x86_64/src/proc.rs
  - ../r9/x86_64/src/runtime.rs
  - ../r9/x86_64/src/seg.rs
  - ../r9/x86_64/src/swtch.rs
  - ../r9/x86_64/src/syscall.rs
  - ../r9/x86_64/src/trap.rs
  - ../r9/x86_64/src/uart16550.rs
  - ../r9/x86_64/src/usermem.rs
  - ../r9/xtask/src/config.rs
  - ../r9/xtask/src/main.rs
  - ../r9/xtask/src/uregcheck.rs
  - .gitignore
  - amd64/ABI.md
  - build/mm/boot-9k10t1-sweep.sh
  - build/mm/build-9k10t1.sh
  - build/x86_64/hello.c
  - build/x86_64/mkimage.sh
  - build/x86_64/mkpaqfs-host.sh
  - build/x86_64/netblast.c
  - build/x86_64/nm-input.6
  - build/x86_64/root-tree/README.md
  - build/x86_64/root-tree/lib/namespace
  - build/x86_64/root-tree/lib/netaddr.rc
  - build/x86_64/root-tree/lib/netip-dhcp.rc
  - build/x86_64/root-tree/lib/netip-loopback.rc
  - build/x86_64/root-tree/lib/netip-static.rc
  - build/x86_64/root-tree/lib/netip-tcp.rc
  - build/x86_64/root-tree/rc/bin/termrc
  - build/x86_64/root-tree/usr/glenda/lib/profile
  - build/x86_64/smoke-r9-net.expect
  - build/x86_64/smoke-r9.expect
  - build/x86_64/smokesleep.c
  - build/x86_64/stage-build.sh
  - build/x86_64/subset-build.sh
  - build/x86_64/tcpblast.c
  - build/x86_64/tcpecho.c
  - build/x86_64/tier-termrc
  - build/x86_64/tier1-r9.expect
  - build/x86_64/tier1-sweep.rc
  - build/x86_64/tier1.ledger
  - build/x86_64/tiers/tier0-libs.txt
  - build/x86_64/tiers/tier1-cmds.txt
  - flake.lock
  - flake.nix
  - nix/boot-r9-x64.sh
  - nix/smoke-r9-net-x64.sh
  - nix/smoke-r9-x64.sh
  - nix/tier1-r9-x64.sh
  - nix/toolchain.sh
  - sys/src/9k/boot/embed.c
  - sys/src/9k/boot/mkfile.amd64
  - sys/src/cmd/7a/BUILD
  - sys/src/cmd/7c/BUILD
  - sys/src/cmd/7l/BUILD
  - sys/src/cmd/auth/factotum/mkfile
  - sys/src/cmd/auth/mkfile
  - sys/src/cmd/ip/mkfile
  - sys/src/cmd/mkfile
  - sys/src/cmd/nm.BUILD
  - sys/src/cmd/paqfs/mkpaqfs.c
  - sys/src/mkfile
findings:
  critical: 1
  warning: 3
  info: 3
  total: 7
  necessity: 1
status: issues_found
---

# Phase 14: Code Review Report

**Reviewed:** 2026-11-09T12:00:00Z
**Depth:** standard
**Files Reviewed:** 72
**Status:** issues_found

## Summary

All 72 scoped files across both repos (r9 kernel/xtask/flake and Harvey build wiring/scripts/gates) were reviewed at standard depth. The no_std kernel core held up under adversarial tracing: the SYSCALL entry trio's frame arithmetic was cross-checked qword-by-qword against the frozen 184-byte layout (trap.rs const-asserts are correct and the stub-vector derivation `shrw $3` is provably exact given the 4096-aligned stub array), the usermem validation constructors and their bounds paths are sound, the PML4 walk/free bookkeeping (RFMEM refcounts, unwind paths) is consistent, the deliver_notes window arithmetic covers exactly its writes, and the host-side bake/gate scripts carry identity guards, digest pins, and fresh-capture hygiene throughout. `flake.lock` and `build/x86_64/nm-input.6` are generated/binary artifacts and were verified structurally (lock pinned; object tracked via the `.gitignore` negation at line 152 — confirmed working).

One BLOCKER was found in the exec path: the pre-free argv fit check (`check_seed_fit`) does not mirror the seeding arithmetic it claims to mirror — it omits the 96-byte Tos reservation at the seed page's top and understates the alignment loss — so a reachable band of argv sizes passes validation, then fails AFTER the old image is freed, killing the process instead of returning -1 (Plan 9 exec semantics). Three Warnings cover a frozen-ABI const-assert silently swallowed by a comment in `dat.rs`, an unvalidated tag-size walk in the l.S early-boot module stasher (boot-time infinite loop on corrupt loader data), and ABI.md's enforcement appendix contradicting the checker's actual behavior. Three Infos cover a hot-path scheduler debug print, dead `touser`/`UTZERO` code whose "unused until 14-05" comment is now false, and one necessity finding.

## Critical Issues

### CR-01: check_seed_fit does not mirror seed_user_pages — exec with argv in the gap kills the process post-free instead of returning -1

**File:** `../r9/x86_64/src/exec.rs:225-235` (check), `../r9/x86_64/src/exec.rs:243-314` (seed), `../r9/x86_64/src/syscall.rs:240-241` (arg caps)

**Issue:** `check_seed_fit` exists precisely so `exec_replace` "never fails for ARG-SHAPE reasons after the old image is freed" (its own doc, exec.rs:222-224, and exec.rs:376-380 runs it BEFORE `free_user_mappings`). But its arithmetic is not the seed's:

- check accepts when `strings + 8*(n+2) + 16 <= 4096`, i.e. `strings <= 4064 - 8n` — the "+16 align slack" is its only headroom.
- `seed_user_pages` starts the string cursor at `tos_va = (USTACKTOP - 88) & !0xf = USTACKTOP - 96` (exec.rs:268), so the usable page is 4096-96 = 4000 bytes, and after the strings it additionally loses up to 15 bytes to `cursor &= !0xf` (exec.rs:295) before requiring `8*(argc+2)` to fit (exec.rs:298). Worst-case acceptance is therefore `strings <= 3969 - 8n`.

There is a real acceptance window of ~80-95 bytes in which the pre-free check PASSES and the post-free seed FAILS ("exec: arg strings overflow the seed page" / "exec: stack seed overflow"). In that window `exec_replace` has already freed the caller's mappings (exec.rs:387), so the failure arm kills the process via `exit_tail` (exec.rs:402-405) — the parent reaps `sys: trap`-style death. Plan 9 exec semantics require a -1/errstr return with the old image intact.

The window is reachable from ring 3: `copy_exec_args` (syscall.rs:235-263) admits up to `MAX_EXEC_ARGS = 32` args of up to `MAX_EXEC_ARG_LEN = 128` bytes each, so total string bytes (each +1 for NUL) can reach 4096; for n=32 the gap is `(3713, 3808]` — e.g. 30 args of ~120 bytes plus 2 short ones. No special privileges are needed, only argv content.

**Fix:** make the pre-check recompute the seed's exact arithmetic — reserve the Tos block and the alignment loss:

```rust
fn check_seed_fit(args: &[&[u8]]) -> Result<()> {
    if args.len() > MAX_ARGS {
        return Err("exec: too many args");
    }
    let strings: usize = args.iter().map(|a| a.len() + 1).sum();
    // Mirror seed_user_pages exactly: strings grow down from tos_va
    // (= (USTACKTOP - 88) & !0xf = page top - 96), then the pointer
    // array loses up to 15 more bytes to the !0xf alignment.
    let page_usable = PAGE_SIZE_4K - 96;          // below the Tos block
    let aligned = (page_usable - strings) & !0xf; // cursor after align-down
    if 8 * (args.len() + 2) > aligned {
        return Err("exec: arg strings overflow the seed page");
    }
    Ok(())
}
```

(Alternatively hoist the real layout computation into one shared helper both functions call, so the two can never diverge again — the same shape `Segments`/`segment_layout` already uses.)

## Warnings

### WR-01: The ax@0 frozen-ABI const assert is swallowed by a line comment — the witness does not compile, and ABI.md's enforcement claim is false for ax

**File:** `../r9/x86_64/src/dat.rs:98`

**Issue:** Line 98 reads:

```rust
// checkout needed (the xtask ureg-check covers the header itself).const _: () = assert!(core::mem::offset_of!(Ureg, ax) == 0);
```

Everything after `//` is comment, so the `ax == 0` assertion — one of the "one `core::mem::offset_of!` assertion per pinned offset (ax, bp, r15, ds, …)" that `amd64/ABI.md`'s kernel-side enforcement appendix (amd64/ABI.md:315-319) promises — is silently dead. All the other offsets (bp:99 through ss/sizeof:111) do assert. Today `ax` is the first `repr(C)` field so the guarded state cannot occur, but the entire purpose of the always-on layer is to make any layout drift a build failure; a field inserted before `ax` would pass the in-crate layer, and the doc's claim would be silently untrue. This is the exact "witness that certifies nothing" failure mode the negative-control discipline exists to catch.

**Fix:** Put the assert on its own line:

```rust
// Always on — any layout drift fails `cargo build`, no Harvey checkout
// needed (the xtask ureg-check covers the header itself).
const _: () = assert!(core::mem::offset_of!(Ureg, ax) == 0);
```

### WR-02: l.S module-tag walk performs no minimum tag-size validation — a corrupt size-0 tag loops forever at 32-bit boot, and module-header reads can run 4 bytes past the stash

**File:** `../r9/x86_64/src/l.S:170-198`

**Issue:** The pre-kernel walker over the stashed mb2 info advances by `ecx = (tag_size + 7) & ~7`. A tag whose declared `size` is 0 — or any size ≥ 0xFFFFFFF9 whose +7 wraps to 0 — advances `%ebx` by exactly 0, so label `1:` re-reads the same tag forever with interrupts masked and no console alive: a silent hang before the first instruction of Rust, on data the kernel elsewhere explicitly treats as untrusted (`mb2.rs`'s own header: "The pointer GRUB hands the kernel is untrusted firmware input (T-13-03)"; the Rust parser rejects `size < 8` loudly at mb2.rs:282-284 — but it never runs if the asm walk hangs first). Additionally, the module arm reads `mod_start`/`mod_end` at `8/12(%esi,%ebx)` with `%ebx` allowed up to `MB2INFOMAX-8-1 = 4087`, so `12(%esi,%ebx)` can read up to byte 4099 — up to 4 bytes past the 4096-byte `mb2info` buffer (into adjacent .bss — zeroed today, so benign in effect, but out of bounds of the declared buffer).

**Fix:** Validate the tag size before advancing, and bound the module header read like the info-stash copy is bounded:

```asm
1:	cmpl	$(MB2INFOMAX - 16), %ebx	// header + fixed module body fit
	jae	4f
	movl	(%esi,%ebx), %eax	// tag type
	movl	4(%esi,%ebx), %ecx	// tag size
	cmpl	$8, %ecx		// a tag is at least its 8-byte header
	jb	4f			// corrupt: treat as no module (the
				// kernel-side parser reports loudly)
	cmpl	$3, %eax
	je	2f
	...
```

### WR-03: ABI.md's enforcement appendix contradicts the checker it freezes — stale `../harvey` default, wrong pinned-table size, wrong mutation medium

**File:** `amd64/ABI.md:320-342`

**Issue:** The kernel-side enforcement appendix (part of the frozen cross-repo contract) states the header is "located via the `R9_HARVEY_DIR` environment variable, default `../harvey`" (line 322-323). That default was deliberately removed: `uregcheck.rs:70-84` makes an unset `R9_HARVEY_DIR` a named hard error and its doc header says "The implicit `../harvey` sibling-checkout default is REMOVED". The same appendix says the check compares against "a pinned 22-slot table" (line 324) — the actual pinned table (`PINNED_FIELDS_AMD64`, uregcheck.rs:56-59) has 26 entries (22 u64 + the 4 u16 segment fields the appendix itself insists are never skipped), and it describes the negative control as run against "a deliberately-mutated temp copy" (line 337-338) when the mutation is in-memory (`replacen`, uregcheck.rs:184). In a document whose whole point is that both repos re-freeze together, these drifts mislead: someone relying on the documented default would not discover that bare invocations now fail.

**Fix:** Amend the appendix to match the implementation: "`R9_HARVEY_DIR`, REQUIRED (no default — an unset variable is a named error)"; "a pinned 26-entry table (22 u64int fields plus the four u16int segment fields)"; "a deliberately-mutated in-memory copy of the header text".

## Info

### IN-01: `sched: park` println on every preemption — debug artifact in the scheduler hot path

**File:** `../r9/x86_64/src/proc.rs:1686`

**Issue:** `reschedule()` prints `sched: park {pid}` on every tick-driven park. At the 100 Hz slice rate with two or more runnable procs (the composed chain: rc, paqfs, boot children), this emits ~10 serial lines/second indefinitely — console noise in every gate capture and a per-preemption serial-write cost that perturbs the very timings the gates are TCG-sized around. It is not a Console-lock hazard (preempt only acts when the tick interrupted ring 3, which never holds the lock), but it is unconditioned output in the hottest path in the kernel, unlike the carefully cfg-gated tick println (timer.rs:214-217).

**Fix:** Gate it behind a cfg or debug flag (mirroring the tick print's `#[cfg(not(dev_exec))]` shape), or delete it now that the 14-06 park-and-swtch design is proven.

### IN-02: Dead code: `touser` and `UTZERO` are never called, and their "unused until 14-05" comments are now false

**File:** `../r9/x86_64/src/trap.rs:43-45, 453-502`

**Issue:** `touser` (trap.rs:480, `#[allow(dead_code)]`) and `UTZERO` (trap.rs:45) have no call sites anywhere in the crate (verified by grep — only comments reference them). Both comments say "Unused until 14-05's exec_bootstrap", but 14-05 landed and `exec_bootstrap` hands off through `trap_return` + the seeded frame (main.rs:545-550), never through `touser`. The comments now assert a future that already happened differently. Note `UTZERO+0x28`'s VALUE lives on correctly in `exec::INITTEXT`/`TEXT_MAP_BASE`.

**Fix:** Either delete `touser`/`UTZERO` (the donor-verbatim rationale is preserved in git history and ABI.md's "Return to user / first entry" section), or correct the comments to say "retained donor-verbatim; superseded by trap_return" — keeping a wrong "unused until" comment is how the next reader concludes it is still wired up.

### IN-03: Impossible-state guard — check_data_bss_below_ustop cannot fire for any parseable header

**File:** `../r9/x86_64/src/exec.rs:525-536`

**Category:** necessity

**Issue:** The USTOP ceiling check compares `data_va + data + bss >= USTOP` where `data`/`bss` are u32-derived u64s from the a.out header (exec.rs:503-505) and `USTACKTOP - 16 MiB` sits ~128 TiB up the user map. Two u32 fields sum to < 2^33 bytes, so the check can never fire for any header that passed `parse_aout` — the file's own comment states this ("with u32 a.out size fields (~8 GiB max) against the 128 TiB user map this can never fire for a parseable header"). The impossibility is provable from the repo (field widths at exec.rs:503-505 vs `USTACKTOP`/`USTKSIZE` at exec.rs:419-423), and the same boundary is implicitly re-proven by `sys_brk_`'s independent `addr > exec::USTOP` check (syscall.rs:801-803). Retained as documented defense-in-depth for a future 64-bit a.out format, with its boundary arithmetic host-pinned (exec.rs:651-675) — so removal is a scope decision, not a defect fix; recorded per the necessity rubric.

**Fix:** If the future-format concern lapses, remove the parse-path call and keep only the host-pinned test of the boundary arithmetic; otherwise leave as is — the comment already discloses the impossibility honestly.

---

_Reviewed: 2026-11-09T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
