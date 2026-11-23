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

## Assumption A7

ASSUMPTION A7: UNTESTED — the static half passes (the flat rust-lld
payload wrapped in the header satisfies every rule of r9's x86_64
`parse_aout`, with the PWRITE/EXITS sequences in the disassembly); the
running half is answered by the arm64 HVF leg below and the owner's
x86_64 KVM leg in Plan 06.
