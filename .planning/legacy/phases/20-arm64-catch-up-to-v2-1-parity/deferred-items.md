# Phase 20 deferred items

## 20-05: boot-chain trace sweep-stage launch race (out of scope, not fixed)

Found during 20-05 Task 2, baseline run 2 (`.build/trace/base-run2-raw.log`).

- **What failed:** the chain stage passed (`__GSD_CHAIN_DONE__`, one line-start `Fversion` record). The next stage, `rc /boot/trace-boot.rc sweep`, then aborted at the fourth entry with `/boot/trace-boot.rc:272: > can't create: /proc/16353/ctl: file does not exist: '/proc/16353'`. The script went back to `term%` without printing `__GSD_ALL_DONE__`, and the harness kept waiting for its 1800 s `sweep traces` timeout. I stopped qemu, the harness and its srv9 by hand after about 6 minutes, by which point the result was already decided. The run is recorded as a failure.
- **Cause (from the capture):** in `if (~ $mode sweep)`, the parent forks `{ spin; …; exec /boot/ourcat … } &` and then runs `echo stop > /proc/$tpid/ctl`. In Plan 9, `echo` is `/bin/echo`, so the arming write costs a fork and an exec in the parent. The child is protected only by `spin`, a syscall-free loop of about 60×60 iterations (a few hundred ms under TCG). In this run the child got through the spin, the exec and all of `ourcat`'s work, and exited before the parent's `stop` landed. The `ourps` listing just before shows an `rc` (pid 15232, most likely the sweep parent) in state `Fault`, so the parent was slow. In the passing runs the same entry's first traced record is already the post-spin `Create "/dev/null"`, so the margin is thin there too.
- **Why not fixed here:** this is in `build/aarch64/trace-boot.rc`, which 20-05 does not own. It is a launch race in the sweep fixture, not the D-06 `fversion` miss, and the 20-05 changes did not cause it. Possible fixes: a born-traced self-stop for sweep children, as the chain stage already uses (`traceloop1`); a longer spin; or a `test -e /proc/$tpid` guard so a lost race becomes a recorded skip rather than a 30-minute hang. Each one changes the gate's design, so they are left for the owner.
- **Effect on 20-05:** it hit 1 of 6 runs in total (1 of 3 baseline runs, 0 of 3 fixed-harness runs). The D-06 diagnosis does not depend on it.

## 20-06: r9 aarch64 kernel-virt build is not deterministic (RESOLVED at r9 2226cc4, 2026-11-18)

**Resolution:** r9 `2226cc4` adds `--remap-path-prefix` (NIX_BUILD_TOP -> `/build`, workspace -> `/r9`) in xtask's build step. `nix build .#kernel-virt --rebuild` now reports no difference on darwin, and the linux host's `nix build .#boot-iso --rebuild` passes with the existing outputHash. The at-pin series is re-run once at 2226cc4. After that, a pin change that leaves aarch64 untouched is checked by comparing kernel bytes.

Original record:

Found during 20-06, at the second pin (5f3ddb0 -> ff2b632).

- **What happens:** building the same r9 kernel-virt derivation twice gives different bytes. `nix build --rebuild` at `5f3ddb0` printed `may not be deterministic: output … differs from ….check`. The sandbox build directory, which changes on every build, ends up in the panic-location strings, and the symbol and module hashes change with it. Replacing that path with one of equal length still leaves 19783 bytes different from the `ff2b632` image (record: 20-EVIDENCE.md, `KERNEL-BYTES` line).
- **Why it matters:** two kernels can't be compared byte for byte. So a pin change that doesn't touch aarch64 at all (here, the x86_64 boot-iso `outputHash` line) can't be shown to leave the tested kernel unchanged, and a byte-identity rule forces a fresh HVF series of about 4 hours.
- **Fix:** strip or remap the build path at compile time so it doesn't vary, for example `--remap-path-prefix` in RUSTFLAGS for the kernel build, or `trim-paths` where the toolchain supports it. Then confirm that two `--rebuild` passes produce identical output.
- **For now:** the owner stopped the at-pin series re-run on `ff2b632`. The `5f3ddb0` series (1753/36/10 HVF, all passing) stands for group A, because `ff2b632` changes only the x86_64 FOD hash.

## 20-07: 7c (arm64) does not narrow a uchar function result

Found during 20-07 Task 2 (the first two brkprobe builds; captures in `.build/p20-07-brk-v1/` and `.build/p20-07-brk-v2/`).

- **What happens:** `static uchar pat(ulong off, int who){ return (off/Page)*31 + who*0x5a + 1; }`, compared as `p[i] != pat(i, who)` against the byte just stored, compares unequal exactly when the unmasked value exceeds 0xff. The first bad page was page 6 for who=1, page 3 for who=2 and page 0 for who=4, and the printed values were equal (`want=0x15 got=0x15`). The caller seems to use the full register, and neither side narrows the result to 8 bits. Compiled with `7c -FTVw`.
- **Why not fixed here:** it is a compiler issue (`sys/src/cmd/7c`), outside 20-07's files. brkprobe masks with `& 0xff`, with a comment explaining why. Other arm64 code that returns `uchar`/`char`/`ushort` from a computed expression could be affected silently.
- **Next step:** a three-line reproducer under 7c and the 7c/7l return-value code path (`sys/src/cmd/7c/cgen.c`, `sys/src/cmd/cc` for the function-return cast).

## 20-07: the lima guest checkout's .git had core.ignorecase=true (venue fix applied)

Found during 20-07 Task 1. The guest's `~/harvey-build/.git` was copied from darwin, and it carried `core.ignorecase=true` on a case-sensitive disk. Every `nix run .#boot-r9-tier1` build failed in the guest's nix fetch with `sys/lib/postscript/troff/Hx: No such file or directory` (3 of 3 attempts; 20-06 hit the same thing and avoided it with `R9_SKIP_BUILD=1`). Applied in the guest only: `git config core.ignorecase false`. No repo file changed. To revert: `git -C ~/harvey-build config core.ignorecase true`. The darwin-side HX/Hx collision itself (the 11 ` M sys/…` font entries) is untouched and still the owner's call.

## 20-09: the heap census reads `live 0 blocks` at every sample on aarch64

Found during 20-09 Task 3 (every virtchurn capture, `.build/p20-09-virtchurn-run*.log` and `-predrain-run*.log`).

- **What happens:** `churngate: census <tag> live 0 blocks 0 bytes;` at start, one, base and end, while the pooled census grows (for example 36 to 135 blocks). This holds even in the pre-drain runs, where the cursor climbs about 16 KiB per exec. `QuickFit::split_census` (port/src/allocator.rs:601) counts a slot as live when it is `used` and on no free list. On aarch64 it finds none, so either the allocated blocks' header slots are not marked `used`, or they sit beyond the `high_main`/`high_reserve` bound the walk stops at.
- **Why not fixed here:** the gate's verdict rides the bump cursor (`heap_stats`), not the census, so the gate is unaffected. The census code is in `port/src`, which this plan may not edit.
- **Next step:** compare an x86_64 q35churn capture's census lines. If x86_64 also reads live 0, the census is wrong on both arches; if it does not, look at the aarch64 allocation path.

## 20-10: create(..., OCEXEC) does not mark the chan close-on-exec (both arches) — RESOLVED at r9 437b752

**Status: RESOLVED** in r9 437b752 (in the final pin e6e5220; pinned by T1-OK cexeccreate). Originally: The owner chose to fix this now, through a separate debugger session (2026-11-20, after the 20-10 pin move to r9 `1125a20`).

Found during 20-10 Task 3, while checking that open's OCEXEC bit reaches `port::namec::open_chan` on aarch64 (it does: `sys_open` passes the full mode, and `open_chan` sets `cexec`).

- **What happens:** `sys_create` on both arches (`aarch64/src/syscall.rs:1508`, `x86_64/src/syscall.rs:1480`) passes `mode & !(OEXCL | OCEXEC)` to `create_chan_union`, and nothing sets `cexec` on the created chan afterwards. In the donor, `create` with OCEXEC sets `CCEXEC` like `open` does. So a file created with OCEXEC survives exec. The existing-file arm (`open_chan(&existing, mode | OTRUNC)`) does record it.
- **Why not fixed here:** cexecprobe exercises the open path, which is what libthread's procexec uses (`open(PIPEMNT "/data1", OWRITE|OCEXEC)`). The create path is the same on x86_64, so it is a parity gap on both arches, and fixing it is outside this plan's files.
- **Next step:** set `c.cexec = mode & OCEXEC != 0` on the created chan in both arches' `sys_create` (arch code only), and give cexecprobe a create leg.
