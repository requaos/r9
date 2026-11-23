# S6 — the minimum Rust-to-Plan-9-a.out probe (D-17)

Question: Does a `no_std` Rust program linked flat by rust-lld and wrapped
in the 40-byte Plan 9 a.out header load and run under r9's loader, and
which harmless image tool should D-17 replace? (01-RESEARCH.md, "Probe:
minimal a.out facts (D-17)"; Assumption A7; D-17/D-18.)

## amd64, static half (this Mac)

VERDICT s6-amd64-static-mac: PASS
EVIDENCE s6-amd64-static-mac: s6/evidence/amd64-static.txt

The probe is `s6/probe-amd64.rs` (no_std, no dependencies, a
`global_asm!` `_start` that issues PWRITE(51) then EXITS(8)) linked by
rust-lld to a flat payload at 0x200028 and wrapped by `s6/aout.py` in the
40-byte big-endian header. `aout.py check --arch amd64` passes every rule
of r9's `x86_64/src/exec.rs` `parse_aout`/`read_aout`: magic 0x8A97, text
96 bytes, entry 0x200028 inside text, file length 40+96, data/bss/syms
zero. The disassembly at 0x200028 starts at `_start`, shows
`mov ebp, 0x33` + `syscall` and `mov ebp, 0x8` + `syscall`, and the marker
bytes `R9-RUST-AOUT-PROBE-OK\n` follow in the same text payload. The
a.out sha256 is recorded in the evidence file; built binaries stay under
`s6/work/` (gitignored), and Plan 06 rebuilds them from these sources.

Two ABI facts were confirmed against Harvey at the pin and are cited in
the evidence file: the number rides in BP with arguments at SP+8
(`amd64/ABI.md`, "Wire contract"), and PWRITE is 51 and EXITS is 8
(`sys/src/libc/9syscall/sys.h:50`, `:9`).

## D-17 replacement target

**Recommendation: `ps`.** It is in every x86_64 chain-family image
(`build/x86_64/mkimage.sh:306-309`), has zero boot-path invocations, and
has only two gate sites (`build/x86_64/smoke-r9.expect:265`,
`build/x86_64/tier1-sweep.rc:137`). `echo` is not an rc builtin
(`sys/src/cmd/rc/plan9.c:35-48`), so it drives cpurc's entire marker
ladder and is disqualified despite being D-17's prose example. Full
evidence and the rejected alternatives are in `s6/target-analysis.md`.

## arm64, end to end under HVF (this Mac)

S6-ARM64 control=PASS probe=PASS flipback=PASS sha-equal=yes
VERDICT s6-arm64-hvf-mac: PASS
EVIDENCE s6-arm64-hvf-mac: s6/evidence/arm64-hvf-control.log
EVIDENCE s6-arm64-hvf-mac: s6/evidence/arm64-hvf-probe.log
EVIDENCE s6-arm64-hvf-mac: s6/evidence/arm64-hvf-flipback.log

Three legs against a SCRATCH copy of Harvey's arm64 tree
(`s6/work/harvey-arm64/`; the real checkout was never written). Each leg
re-baked `build/aarch64/bootfs.paq` with Harvey's own host `mkpaqfs`
(via `HARVEY=<scratch> sh <scratch>/build/aarch64/mkimage.sh`) and booted
r9's aarch64 virtnet profile under HVF:

```
R9_HARVEY_DIR=<scratch> nix develop -c cargo xtask qemu \
    --arch aarch64 --config virtnet --accel hvf
```

`cargo xtask qemu` is the boot command, not `boot-test`: qemu leaves the
`-serial mon:stdio` console interactive, which is what lets
`s6/virt-probe.expect` type each leg's own command; `boot-test` drives a
fixed probe string of its own and cannot run a leg-specific command.

- **control** — original `arm64/bin/echo`; typed
  `/bin/echo R9-ECHO-CONTROL`; the typed line and the program output both
  appear (two-occurrence match), then a fresh `% ` prompt.
  Image sha256 `746f1dfe0f3f96e46d6cf1e4f4b963dd5d3dad195b370b19c973d17394f6f5b4`.
- **probe** — `arm64/bin/echo` replaced by `s6/work/probe-arm64.aout`
  (sha256 `cab75a69fedd7bb3cbb8f744d099ef17b49150961d6f6757465b096200126695`);
  typed `/bin/echo x`; the probe printed `R9-RUST-AOUT-PROBE-OK`, exited
  (`process 6 exited`), and rc re-prompted. Image sha256
  `242ea95aeb89c2ab968fbc71bdf78805e1f4443c32a205e128ee355bfe22f310`.
- **flipback** — original echo restored (its sha256 matches the source
  checkout's `arm64/bin/echo`,
  `11836773d560c6eb11db209f997a4d2d68a214fbbee0ab635b8fe56631feda8f`);
  the control marker again; image sha256 equals the control leg's.

The same probe source is built by `build-probe.sh arm64` with plain rustc
(target `aarch64-unknown-none`), linked flat at 0x10028 and wrapped by
`aout.py`, which passes every rule of r9's `aarch64/src/exec.rs`
`parse_aout`. The ABI used is the frozen one: number in X0, arguments at
SP+8, `SVC` (`arm64/ABI.md`, "Wire contract"; `aarch64/src/syscall.rs:1-10`).

**Determinism note (scratch-only).** Harvey's aarch64 `mkimage.sh` has no
KERNDATE/mtime pin (its x86_64 sibling does), and the checkout's prebuilt
darwin `mkpaqfs` predates the KERNDATE feature — two bakes of one tree
differ in file mtimes, header time and the trailer SHA1. So
`run-arm64-hvf.sh` installs a shim at the scratch's
`.build/bin/mkpaqfs` that pins staged mtimes, runs Harvey's real binary
(`mkpaqfs.real`, unmodified) and then pins the header time to the bd7a698
commit time (`1795382101`) and recomputes the trailer digest. This makes
the flip-back byte-equality check meaningful without editing Harvey. The
real checkout stayed untouched: HEAD `25d97dc4d`, its `git status
--porcelain` sha256 is `041d29292e...`, and no file under
`build/aarch64` has an mtime newer than the plan's start.

## Assumption A7

ASSUMPTION A7: UNTESTED — the static half passes (the flat rust-lld
payload wrapped in the header satisfies every rule of r9's x86_64
`parse_aout`, with the PWRITE/EXITS sequences in the disassembly); the
arm64 running half is CONFIRMED above under HVF (probe, marker,
flip-back, identical image bytes); the amd64 running half is the owner's
x86_64 KVM leg in Plan 06, so A7 as written (0x200028 under r9's x86_64
loader) stays untested until then.
