# Phase 1: Stage P0 - Cross-Build Harness - Context

**Gathered:** 2026-11-22
**Status:** Ready for planning

<domain>
## Phase Boundary

From the r9 repo alone, `cargo xtask` builds and boots r9 to rio on QEMU q35 x86_64, drawing every C library object and userland binary from a read-only, pre-built Harvey tree supplied by r9's nix flake, with a per-file fallback manifest that lets a Rust replacement swap in for one artifact at a time. Flake ownership flips in this phase: r9's flake imports Harvey and owns the composed boot and gate outputs; Harvey's flake no longer imports r9.

Requirements: HARN-01 to HARN-08. Not in this phase: porting any real tool (Track P), any kernel behaviour change, the `rfork` flag reservation (Phase 2).

</domain>

<decisions>
## Implementation Decisions

### Carried in from the roadmap session (owner, 2026-11-22)
- **D-01:** r9's flake pins Harvey, and the default devshell (`nix develop`) exports the read-only tree location (`R9_HARVEY_DIR`) so `cargo xtask` finds it with no manual setup. xtask keeps no built-in default path; outside the devshell an unset variable stays a named error. (HARN-07)
- **D-02:** Flake ownership flips: r9 imports Harvey and produces the composed outputs Harvey's flake owns today; Harvey's flake drops its `r9` input. (HARN-08) — **Reversibility:** costly — both flakes, both lock files, CI and every gate entry point change together.

### Pre-built tree supply
- **D-03:** The built Harvey artifacts are produced by a nix derivation in r9's flake, built from the pinned Harvey source. The devshell points at the store path, so the tree is read-only by construction. Harvey's in-tree build scripts (unshare namespaces, writes into the checkout) must be made to run inside the nix sandbox.
- **D-04:** On aarch64-darwin the same derivation is built by a nix Linux builder (the nix-darwin linux-builder, or the existing lima VM registered as a remote builder). One build path on every host; no separate lima script path for r9.
- **D-05:** Outputs are per guest architecture (amd64, arm64). The devshell exposes both; each is built only when a command for that architecture first needs it.
- **D-06:** The Harvey pin moves by deliberate bumps: one r9 commit per bump, which must pass the golden gate. For local Harvey work, an input override points at a checkout for that session only; it is never the default.

### Flake flip shape
- **D-07:** The gate machinery moves into r9: the `nix/*.sh` gate scripts and the `build/<arch>/*.expect` drivers are copied into r9, which owns them from then on. Harvey's copies are deleted in the Harvey commit. This widens the approved Harvey diff beyond `flake.nix` to those deletions. — **Reversibility:** costly — about twenty scripts and fifteen expect drivers change home.
- **D-08:** Harvey's flake stays self-sufficient without r9. The owner's words: "harvey should still be able to build itself and developer should be able to pull harvey and run one nix command to build and launch harvey in q35 qemu and another to build and launch drawterm to connect to harvey, all without r9." Harvey therefore keeps its build, setup, toolchain and iso apps, its devshell, its `drawterm` package, and a one-command q35 launch of Harvey's own kernel. If that launch command does not exist today it is added in Harvey as part of this phase.
- **D-09:** r9 takes Harvey as a normal flake input (not `flake = false`) and may reuse Harvey's packages (for example `drawterm`, `harness`, `minimega`). The built-tree derivation of D-03 is still defined in r9.
- **D-10:** Order of the flip: r9 first. r9 gains the Harvey input and every composed output and proves each gate green from r9. Only then does a Harvey commit remove the `r9` input, the composed outputs and the moved scripts, followed by an r9 pin bump to that Harvey commit. No commit may leave a lock cycle or a gate with no home.

### Boot-to-rio gate
- **D-11:** "Reached rio" is proven by the existing drawterm gate (boot the cpu image on q35, connect drawterm over the encrypted session) extended to start rio and assert a marker that rio is serving its window files. A display-less way to confirm rio is up is needed.
- **D-12:** Enforcement is by practice, not by hook: one `cargo xtask` command is the gate, it is run and green before each commit lands on main during planned work, and the result is recorded in the phase summary. CI is a second witness.
- **D-13:** The required green witness is a local x86_64 Linux run under KVM. Hosted CI runs the gate best-effort. On the Mac the gate may run under TCG as a convenience but is not the witness.
- **D-14:** Two tiers. The quick gate (reuse the already-built Harvey tree from the store, boot, reach rio) runs for every commit. The full set of moved gates (smoke, net, auth, services and the rest) runs at phase boundaries and at Harvey pin bumps.

### Fallback manifest and swap
- **D-15:** The manifest has one entry per file in the root image (each binary and library object), per guest architecture, marked Harvey or Rust. The harness fails when manifest and image disagree in either direction.
- **D-16:** r9 assembles the root image. It draws each file from the built Harvey tree or from the r9 Rust build according to the manifest and bakes the `.paq` itself. Harvey supplies files, not finished images. — **Reversibility:** costly — r9 becomes the owner of image contents and of the bake that Harvey's `mkimage` scripts perform today.
- **D-17:** The swap is proven with a trivial `no_std` Rust program built as a Plan 9 `a.out` that replaces one harmless tool (for example `echo`) and prints a recognisable marker; the gate checks the marker, then the entry is flipped back. This pulls the minimum Rust-to-`a.out` path (`elf2aout`) forward from Phase 9, only as far as this probe needs.
- **D-18:** The probe is removed once it has proven the mechanism; the result is recorded. The default image stays all-Harvey and no throwaway code remains in the tree.

### Claude's Discretion
- The exact marker used to show rio is serving (D-11) and how it is observed without a display.
- Manifest file format and location inside `xtask/`.
- How existing `R9_HARVEY_DIR` consumers (boot-test, ureg-check, initrd, pcap `netaddr.rc`, `harvey:` module paths) are routed through one resolver.
- Which Linux builder mechanism to document for the Mac (D-04), provided the build path stays single.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition
- `.planning/ROADMAP.md` (Phase 1 section) — goal, seven success criteria, findings at HEAD
- `.planning/REQUIREMENTS.md` (HARN-01 to HARN-08) — the phase requirements
- `.planning/PROJECT.md` (note N4, Key Decisions) — flake ownership and golden-invariant context
- `.planning/STATE.md` — owner standing rules (push approval, `port/src` diff approval, x86_64 kernel byte-change procedure)
- `.planning/r9-implementation-guide-amended-v3.md` §8 — Track P, Stage P0 as specified by the guide

### r9 flake and harness
- `flake.nix` — current inputs (no Harvey), `kernel-*`, `iso-builder`, `netdev-args`, `boot-iso` packages, devshell at the end of the file
- `xtask/src/uregcheck.rs` — `harvey_dir()`, the single `R9_HARVEY_DIR` reader today
- `xtask/src/config.rs`, `xtask/src/main.rs`, `xtask/src/pcap.rs` — the other Harvey-tree consumers and the `harvey:` path scheme
- `x86_64/lib/config_q35shell.toml`, `aarch64/lib/config_virtnet.toml` — configs that name Harvey-built images
- `HowItWorks.md` (section "The flake kernel package + the R9_HARVEY_DIR contract") — the deliberate "no default path" decision
- `.github/workflows/xtask.yml` — how CI checks out Harvey today

### Harvey side (sibling checkout `../harvey`, read-only reference)
- `../harvey/flake.nix` — `inputs.r9`, the composed apps to move, the build/setup/toolchain/iso apps that stay
- `../harvey/nix/drawterm-gate-x64.sh` — the gate D-11 extends; its header documents the pipeline and the KVM witness rule
- `../harvey/nix/build.sh`, `../harvey/nix/vm-build.sh`, `../harvey/nix/lima-harvey.yaml` — the in-tree build that D-03 must run in the sandbox
- `../harvey/nix/*.sh` and `../harvey/build/x86_64/*.expect`, `../harvey/build/aarch64/*.expect` — the gate scripts and drivers that move under D-07

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `uregcheck::harvey_dir()` — already the one place `R9_HARVEY_DIR` is read; the natural seat for the resolver.
- r9 flake packages `iso-builder`, `netdev-args`, `kernel-x64`, `kernel-virt`, `kernel-virt-cpu` — Harvey's gate scripts consume these through environment variables (`R9_KERNEL_X64_PKG`, `R9_ISO_BUILDER`, `R9_NETDEV_EMITTER`); after the flip they are referenced inside r9's own flake.
- Harvey's `drawterm` package and `drawterm-gate.expect` — the basis of the rio proof.

### Established Patterns
- r9 consumes baked images from Harvey (`build/x86_64/out/bootfs-chain.paq`, `build/aarch64/bootfs.paq`), plus `ureg.h` headers and `build/aarch64/netaddr.rc`. D-16 changes this to per-file assembly in r9.
- Harvey's build is Linux-native (unshare) and uses the lima VM on the Mac; built outputs such as `amd64/bin` are gitignored, so a source input alone holds no binaries.
- The r9 devshell provides `grub2` and `xorriso` on Linux only; ISO composition on the Mac is not available today.
- Kernel builds are reproducible; a flake or xtask change in this phase should change no kernel bytes. If it does, the x86_64 byte-change procedure applies.

### Integration Points
- `flake.nix` devshell `shellHook` — where the tree location is exported.
- Harvey's `flake.lock` pins r9 `116bcc2`; the `legacy-final` tags keep the pre-flip arrangement reproducible.
- CI workflow checks Harvey out next to r9 and sets `R9_HARVEY_DIR`; it must move to the flake supply.

</code_context>

<specifics>
## Specific Ideas

- Harvey, after the flip, is a repo a developer can pull and use on its own: one nix command builds and launches Harvey in q35 QEMU, another builds and launches drawterm to connect to it.
- The quick/full gate split mirrors the owner's existing quick-pass versus full-series rule for arm64.

</specifics>

<deferred>
## Deferred Ideas

- Re-expressing the shell and expect gates as `cargo xtask` commands in Rust was considered and not chosen for this phase; it remains an option before Harvey retirement (Phase 16).
- A binary cache for the built Harvey tree was considered for the Mac and not chosen.

</deferred>

---

*Phase: 1-Stage P0 - Cross-Build Harness*
*Context gathered: 2026-11-22*
