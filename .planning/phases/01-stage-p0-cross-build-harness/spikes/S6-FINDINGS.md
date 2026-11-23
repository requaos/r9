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

## x86_64 KVM leg (owner's Linux host, Plan 06)

VERDICT s6-x86_64-kvm-linux: PASS
EVIDENCE s6-x86_64-kvm-linux: s6/evidence/x86_64-kvm-linux.log

The witness ran the leg on the x86_64 KVM host (Linux x86_64 NixOS
7.4.0-rc3, `/dev/kvm` writable; env: `linux-witness/evidence/env.txt`).
The probe rebuilt on the host matched the recorded sha256
(`db94057f04c751c59a648de5c8505f452bb225b6a276714e7ee5544396a5dcd5`;
probe match: yes). Control and flip-back booted the same chain image
(`3ae4127abe1d...`), the probe image differed (`7af2a8e07e4c...`), and
the terminal line was `S6-X86 control=PASS probe=PASS flipback=PASS
sha-equal=yes`. The guest printed `R9-RUST-AOUT-PROBE-OK` for the typed
`/bin/echo x`, and control/flip-back printed `R9-ECHO-CONTROL`. The
q35shell boot fingerprints agree with that split (`e7e84ec2...` for
control and flip-back; `ec6b0005...` for the probe).

The leg cannot run on this Mac: macOS has no `/dev/kvm`, and the q35shell
profile needs the x86_64 Linux ISO path (grub2/xorriso are in r9's
devshell on Linux only).

What the script does, per leg (control, probe, flip-back):

1. puts the right `echo` at the chain contract's source path,
   `$S6_HARVEY/build/x86_64/out/echo` (harvested by
   `build/x86_64/stage-build.sh:197-201`, staged by
   `build/x86_64/mkimage.sh:308`);
2. re-bakes with Harvey's own
   `$S6_HARVEY/build/x86_64/mkimage.sh` under `HARVEY=$S6_HARVEY` and
   `KERNDATE=1795382101`, and records the sha256 of
   `build/x86_64/out/bootfs-chain.paq`;
3. boots
   `R9_HARVEY_DIR=$S6_HARVEY nix develop -c cargo xtask qemu --arch x86-64 --config q35shell --accel kvm`
   inside the pinned scratch r9 clone, driven by `q35shell-probe.expect`
   (wait for `% `, type `/bin/echo R9-ECHO-CONTROL` for control and
   flip-back or `/bin/echo x` for the probe, match the marker, require a
   fresh prompt, quit qemu with Ctrl-A x).

Refusals: running as root, an unwritable `/dev/kvm` (no downgrade — D-13
witness), a `S6_HARVEY` that is a real checkout (`.git` present), an
`S6_R9` not at `116bcc23c8052a1728992cdbc7b625143260d683`, a malformed or
wrong-architecture probe (the bundle's `aout.py check --arch amd64`), and
a scratch tree missing the chain set. A trap restores the original echo
and kills/reaps qemu on every exit path.

Static checks run on this Mac: `bash -n` clean; shellcheck clean on
`linux-x86_64.sh`, `run-arm64-hvf.sh` and `build-probe.sh`; the Tcl
`info complete` check prints 1 for `q35shell-probe.expect`. The script
contains no software-emulation fallback and no `sudo`.

Plan 06 invocation (the S3 scratch tree and the pinned r9 clone):

```
S6_HARVEY=<S3-built scratch Harvey> \
S6_R9=<scratch r9 clone at 116bcc23c8052a1728992cdbc7b625143260d683> \
S6_PROBE=<this bundle's s6/work/probe-amd64.aout> \
S6_OUT=<output dir> \
KERNDATE=1795382101 \
bash .planning/phases/01-stage-p0-cross-build-harness/spikes/s6/linux-x86_64.sh
```

Expected terminal line (seen on the witness):
`S6-X86 control=PASS probe=PASS flipback=PASS sha-equal=yes`.

## What this means for planning pass 2

- **D-17 probe recipe (proven on arm64 HVF and x86_64 KVM):**
  - source: `#![no_std] #![no_main]`, a panic handler that loops, and a
    `core::arch::global_asm!` `_start` that writes the marker with
    PWRITE(51) then calls EXITS(8);
  - build: plain `rustc --edition 2021 --crate-type bin --target
    <x86_64-unknown-none|aarch64-unknown-none> -C panic=abort
    -C opt-level=s -C relocation-model=static
    -C link-arg=-T<ld> -C link-arg=--oformat=binary`, from r9's devshell,
    never added to the cargo workspace;
  - linker script: one `. = 0x200028` (amd64) or `0x10028` (arm64) output
    section, `KEEP(*(.text._start))` first, then text/rodata; discard
    `.eh_frame*`, `.comment`, `.note*`; no data or bss output;
  - header: 8 big-endian u32 (magic 0x8A97/0x8C47, text = flat length,
    data/bss/syms 0, vestigial entry32, spsz/pcsz 0) plus the big-endian
    u64 entry at 0x200028/0x10028, then the flat payload;
  - validation: `aout.py check` mirrors r9's `parse_aout`/`read_aout`
    (magic, text != 0, entry in text, exact file length, no data/bss/syms).
- **Target tool: `ps`.** It is staged in every chain-family x86_64 image,
  is absent from the boot path, and has only two gate sites; `echo` is
  not an rc builtin and carries cpurc's whole marker ladder, so D-17's
  prose example is the one tool not to pick. Details in
  `s6/target-analysis.md`.
- **Flip-back:** the arm64 image bytes are identical on control and
  flip-back (sha256 `746f1dfe0f3f96e46d6cf1e4f4b963dd5d3dad195b370b19c973d17394f6f5b4`),
  with the probe image differing only as expected
  (`242ea95aeb89c2ab968fbc71bdf78805e1f4443c32a205e128ee355bfe22f310`).
  The x86_64 witness reproduced the same equality on the q35shell chain
  image: control and flip-back `3ae4127abe1d8466617d22877f1171f4a30086f05e8ef7c1406c1d5dd8fa9de8`,
  probe `7af2a8e07e4c969907727b3b31964bfddb3cad642fda643fc20766bda1d05f12`.
- **D-18:** every S6 source lives only under
  `.planning/phases/01-stage-p0-cross-build-harness/spikes/`; the probe is
  not in the cargo workspace and is built with plain rustc. The pass-2
  in-tree probe (and its manifest entry) is removed once it has proven
  the mechanism; the default image stays all-Harvey.

## Assumption A7

ASSUMPTION A7: CONFIRMED - the flat rust-lld payload wrapped in the 40-byte header satisfies every rule of r9's x86_64 `parse_aout` and now loads and runs under r9's x86_64 loader on the KVM witness: `/bin/echo x` printed `R9-RUST-AOUT-PROBE-OK`, control and flip-back printed `R9-ECHO-CONTROL`, and all three legs passed with `sha-equal=yes`. Evidence: s6/evidence/x86_64-kvm-linux.log.
