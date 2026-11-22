# Handoff — 2026-11-22

Written at the end of the Harvey-rooted session that finished Harvey phase 20
and set up the legacy checkpoint. Read this first in a new session rooted in
`r9`. Delete it once the first r9 planning pass has absorbed it.

## Where things stand

- **Legacy checkpoint (pushed):** both repos have a `legacy` branch and an
  annotated `legacy-final` tag — r9 `116bcc2`, Harvey `bd7a6982a` (its
  `flake.lock` pins r9 `116bcc2`). This is the last traditional-Plan-9 state.
- **Planning has moved here.** Harvey's `.planning/` is frozen
  (`.planning/RELOCATED.md` there). A verbatim copy is in `.planning/legacy/`
  (see `PROVENANCE.md`); it is history only.
- **Authoritative documents** (this directory):
  `r9-architecture-and-code-v4.md`, `r9-implementation-guide-amended-v3.md`,
  `rfc-9p2000-aead.md`, `r9-permissions-and-test-plan.md`.
- **Unpushed at handoff:** r9 `5b8b70a` (these documents + `legacy/`, 53 MB)
  and this handoff commit; Harvey `25d97dc4d` (the freeze note). The owner had
  not yet decided whether to push, or whether to keep `legacy/` in r9 history.
- **No GSD project exists in r9 yet.** `.planning/` has only the documents,
  `legacy/` and this file.

## Next step

1. Get the owner's push decision for the commits above.
2. Run `/gsd-ingest-docs` on the four documents to create PROJECT,
   REQUIREMENTS and ROADMAP here. Treat `legacy/` as background, not as live
   state.
3. First phase to plan: the owner's "Phase 2" — Stage P0, the cross-build
   harness (Harvey as a read-only reference at `vendor/harvey/` or
   `HARVEY_SRC`; `cargo xtask` draws C userland from the pre-built Harvey tree
   and swaps in Rust replacements; golden invariant: boots to rio on QEMU q35
   at every commit). Note r9 already has `R9_HARVEY_DIR` for boot-test/initrd.

## Owed checks at the checkpoint

- Owner's linux `nix build .#boot-iso --rebuild` at r9 `116bcc2`
  (expected `sha256-TIzwBaHF0R4m31l6EbW2IU3rkUl73g6ljlNE0S5hGFY=`).
- Hosted CI on r9 `116bcc2` and Harvey `bd7a6982a`.

## Carried-over open items

- **Full arm64 HVF series** (smoke 1753, net-smoke 36, tier-1 10, cpu 36, plus
  3 TCG each; ~4.4 h): deferred by the owner to the SMP work (Milestone S).
  Until then, pin moves get a quick pass only: tier-1 on HVF and TCG; smoke,
  net-smoke and cpu once on HVF.
- **WINDOWS #70** (aarch64 scheduler resume skips note delivery): deferred to
  the SMP work. **#49** root-caused and fixed (I-cache), left open until the
  full series runs. **#66** (x86_64 triple fault under 8-way soak): to be
  fixed on an x86_64 host venue. Ledger: `legacy/WINDOWS.md`
  (51 open / 4 waived / 17 fixed / 72 total).
- **`#!` recursion cap** has no host or live test.
- Phase 20 never ran `/gsd-secure-phase` (no SECURITY.md).
- SMP prerequisites noted during phase 20: FP ownership is already per-CPU
  (`CpuFp`); aarch64 still needs per-CPU current frame, kernel stack and proc
  pointers; x86_64 needs a per-CPU `Mach` mapping.

## Standing rules from the owner

- Never add AI attribution to commits or PRs (global CLAUDE.md).
- Pushes in either repo need explicit owner approval. Check what is unpushed
  before pushing; push only the approved commits.
- Edits under r9 `port/src` need owner approval (show the diff first).
  Arch-only edits (`aarch64/`, `x86_64/`, `xtask/`) do not.
- In Harvey, never stage the 11 pre-existing modified troff/postscript font
  files (a darwin case-collision artifact).
- An x86_64 kernel byte change needs: the `boot-iso` `outputHash` re-derived
  and committed, the x64 TCG gates in an amd64 Docker container (cleaned up
  afterwards), and the owner's linux boot-iso check.
- An aarch64 kernel byte change: quick pass now, full HVF series at the SMP
  work. Kernel builds are reproducible (`--remap-path-prefix`), so compare
  kernel sha256 to prove a pin move changes nothing.
- Design rule kept from the FP work: kernels are built without FP/SIMD; kernel
  SIMD goes through `fpu::kernel_begin()` / `with_simd()`; user FP state is
  per process (save at switch, copy on fork, reset on exec). arm64 is the
  reference for x86_64.

## Practical notes

- `gh` is not installed: use `nix run nixpkgs#gh -- …`.
- GNU sed is on PATH on the Mac (`sed -i`, not `sed -i ''`).
- macOS Unix socket paths are capped at 104 bytes: run r9 boot tests from a
  short path.
- The lima VM `harvey` is the Harvey namespace build venue; its
  `~/harvey-build` has `core.ignorecase=false` set on purpose.
- r9 exec caps each argument at 128 bytes; the PL011 console buffer is 256
  bytes (keep typed fixture lines shorter on HVF).
- Debug knowledge base from phase 20: `legacy/debug/knowledge-base.md`.
