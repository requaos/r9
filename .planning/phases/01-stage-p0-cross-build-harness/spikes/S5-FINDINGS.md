# S5 — Harvey's own kernel under QEMU q35 (read-only feasibility)

Question: Can Harvey's own 9k kernel `amd64/9k10cpu.elf` be launched under
QEMU `-M q35` to a prompt, as D-20 requires of the one Harvey command Phase 1
must deliver — and what does building it take without r9? (01-RESEARCH.md,
"Wave 0 Gaps", S5; Flag F2; Assumption A11; Open Question 2.)

S5 is read-only toward Harvey: the kernel was copied out of
`/Users/neilskinner/code/harvey/amd64/9k10cpu.elf`; nothing was built, edited
or executed inside the checkout. r9's own sources and flake are untouched.

## Kernel under test

- `sha256 ecdcdaddea8d3759d74303b869ddfae8efeba1d4d160382577cfc52b9a32c45b`
- 2030226 bytes, ELF 32-bit LSB executable, Intel 80386, statically linked,
  stripped.
- Provenance, strings hints and the source-commit caveat:
  `s5/evidence/kernel-provenance.txt`. No string in the binary proves the
  source commit; the copy's mtime (2026-11-18) is later than the recorded
  2026-11-05 pool-panic run, so the commit is unknown. The KVM witness in
  Plan 06 must use these exact bytes, or a fresh build whose sha256 is then
  recorded.

## Mac legs (TCG)

All three legs ran on this Mac (aarch64-darwin) through r9's devshell
(`qemu-system-x86_64` 11.1.2), against the byte-identical kernel copy. PASS
means: `Plan 9` banner seen, the `root is from` prompt line printed, and none
of the recorded 01-04 signatures (`bad magic`, `panic`,
`suicide: sys: trap`) in the whole capture.

VERDICT s5-q35-virtio-tcg-mac: PASS
EVIDENCE s5-q35-virtio-tcg-mac: s5/evidence/q35-virtio-tcg-mac.log

VERDICT s5-q35-nonic-tcg-mac: PASS
EVIDENCE s5-q35-nonic-tcg-mac: s5/evidence/q35-nonic-tcg-mac.log

VERDICT s5-pc-baseline-tcg-mac: PASS
EVIDENCE s5-pc-baseline-tcg-mac: s5/evidence/pc-baseline-tcg-mac.log

The `pc` leg reproduces the recorded earlier Mac TCG result (Harvey commit
`c8f5422`, 2026-11-05, `.build/poolpanic-verdict.txt`: PASS
no-panic-signature boot-continues, ending at `root is from (tcp)[tcp]:`). The
kernel bytes differ from that run (sha256 above; the earlier file was
rebuilt on 2026-11-18), so this is a reproduction of the behaviour, not of
the exact artifact.

## Commands (verbatim form)

From the r9 checkout root, with `S` =
`.planning/phases/01-stage-p0-cross-build-harness/spikes`:

```
S5_KERNEL="$PWD/$S/s5/work/9k10cpu.elf" S5_ACCEL=tcg S5_MACHINE=q35 S5_NET=virtio \
  S5_LOG="$PWD/$S/s5/work/raw-q35-virtio-tcg-mac.log" \
  nix develop -c sh "$S/s5/boot-q35.sh"

S5_KERNEL="$PWD/$S/s5/work/9k10cpu.elf" S5_ACCEL=tcg S5_MACHINE=q35 S5_NET=none \
  S5_LOG="$PWD/$S/s5/work/raw-q35-nonic-tcg-mac.log" \
  nix develop -c sh "$S/s5/boot-q35.sh"

S5_KERNEL="$PWD/$S/s5/work/9k10cpu.elf" S5_ACCEL=tcg S5_MACHINE=pc S5_NET=default \
  S5_LOG="$PWD/$S/s5/work/raw-pc-baseline-tcg-mac.log" \
  nix develop -c sh "$S/s5/boot-q35.sh"
```

Each leg's final harness line is `S5-RESULT machine=<m> accel=tcg
net=<n> banner=yes prompt=yes sigs=none`; the full stdout and serial capture
are in the evidence files. `pgrep -f 9k10cpu` printed nothing after the legs.

## Observed prompt, NIC and timings

- Prompt, identical on all three legs: `root is from (tcp)[tcp]: ` — the
  embedded boot's root-method prompt; it goes no further without a root
  source (no `version...`, no `argc=`).
- q35 + virtio-net: `#l0: virtio: 1Gbps port 0xc040 irq 11: 525400123456`.
  (The preceding `intrenable: nil handler for 11, tbdf 0xc001000 for ether0`
  line is noise on the virtio path, not one of the 01-04 signatures.)
- q35 + no NIC: no `#l` lines; the prompt is still reached.
- pc + qemu default NIC: `#l0: i82543: 1Gbps port 0xfebc0000 irq 11: 525400123456`
  (the `igbe` driver binding the e1000-class default NIC).
- Banner and prompt each landed within the first 1-second poll interval on
  every leg (`BANNER_SECONDS=1`, `PROMPT_SECONDS=1`; the poll is 1 s, so the
  true times are < 1 s).

## The script

`s5/boot-q35.sh` is the reusable launch-and-classify harness: `S5_KERNEL`
(required), `S5_ACCEL=tcg|kvm`, `S5_MACHINE=q35|pc`, `S5_NET=virtio|none|default`,
`S5_LOG`, `S5_BOOT_BUDGET`, `S5_PROMPT_BUDGET`, `S5_QEMU`. It truncates the
capture before launch, traps every exit path to kill and reap qemu, and
refuses `S5_ACCEL=kvm` without a writable `/dev/kvm` with the named error
`S5-ERROR kvm-unavailable` — it never downgrades KVM to TCG silently (D-13).
shellcheck is clean.
