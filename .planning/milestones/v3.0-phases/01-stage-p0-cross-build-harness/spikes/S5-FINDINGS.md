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

## Build path, prompt and q35 analysis

EVIDENCE s5-build-path-analysis: s5/evidence/build-path-analysis.md

The read-only analysis answers the D-20 recipe questions with file:line
citations at the pin: the minimal build chain and its sudo/namespace needs
on Linux and darwin, what the `root is from (tcp)[tcp]: ` prompt accepts and
what an rc prompt would need, and the q35 NIC/PCI differences that explain
the Task 1 legs. It lays out two D-20 scope options (boot prompt vs rc
prompt via a host-served 9P root) for the closing checkpoint.

## KVM witness (owner's x86_64 Linux host, Plan 06)

VERDICT s5-q35-kvm-linux: PASS
EVIDENCE s5-q35-kvm-linux: s5/evidence/q35-kvm-linux.log

The witness ran the same script against the same kernel bytes under
`-accel kvm` on the x86_64 host (Linux x86_64 NixOS 7.4.0-rc3, `/dev/kvm`
writable; env: `linux-witness/evidence/env.txt`). The kernel sha256
`ecdcdaddea8d3759d74303b869ddfae8efeba1d4d160382577cfc52b9a32c45b`
matched `s5/evidence/kernel-provenance.txt` (2030226 bytes). The harness
classified banner and prompt within the first 1-second poll
(`BANNER_SECONDS=1`, `PROMPT_SECONDS=1`), saw the virtio NIC and no 01-04
signature: `S5-RESULT machine=q35 accel=kvm net=virtio banner=yes
prompt=yes sigs=none`. No `S5_KERNEL_EXTRA` kernel was supplied, so this
answers for the committed artifact only.

## Assumption A11

ASSUMPTION A11: CONFIRMED - the KVM witness launches the same kernel under q35 to the `root is from` prompt with virtio and no recorded signature; the Mac TCG legs and the x86_64 KVM leg now agree. The cpu-service half remains out of D-20 scope and is not claimed. Evidence: s5/evidence/q35-kvm-linux.log.

## What this means for planning pass 2

- **D-08/D-20:** the evidence supports the smallest scope option — one
  Harvey app that builds and launches `amd64/9k10cpu.elf` under `-M q35`
  and classifies the `root is from` prompt (Mac TCG and the x86_64 KVM
  witness both green). The rc-prompt option is feasible without a Harvey
  source edit by serving a root with Harvey's own `u9fs` on TCP 564, but it
  adds a privileged/userns listener and prompt driving, so it is the next
  increment, not the minimum.
- **Build path the Harvey app would wrap:** `nix run .#setup` →
  `nix run .#toolchain` → `nix run .#build` (native sudo+unshare on Linux,
  lima on darwin), then qemu from the devshell. The app should reuse
  `boot-q35.sh`'s capture-before-start, classification and kill/reap trap
  rather than growing a second launch path.
- **Sudo implications:** on the owner's Linux host the supported build
  needs the setuid host `/bin` installs and `sudo unshare -m`; a
  passwordless-sudo host can use the `unshare -Ur -m` chroot venue
  (`build/mm/build-9k10t1.sh:250-356`), but that is Linux-only. On darwin
  the build must stay in the lima VM; the Mac host itself needs no sudo.
- **Kernel provenance:** the copy's source commit is unknown (no build-path
  or revision string in the ELF), so pass 2 should have the Harvey app
  build the kernel it boots and record the resulting sha256, instead of
  relying on a checkout-resident artifact of unrecorded provenance.
- **Artifact reuse:** the kernel-copy handoff (source mtime, sha256,
  `strings` evidence) and the three trimmed serial logs are enough for the
  checkpoint; no r9 kernel or flake change is implied.

