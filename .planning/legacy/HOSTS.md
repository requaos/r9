# Hosts — Development Machine Guide + Migration Log

**Current primary dev host (since 2026-11-06):** NixOS **linux-x86_64**.
**Previous:** darwin-aarch64 (Apple Silicon) — remains a supported documented-manual venue.

## Clean-clone resumption checklist (NixOS linux-x86_64)

The hosted CI legs prove this path continuously (r9-shell.yml + phase6.yml run
green on ubuntu-latest x86_64 from clean checkouts). Locally:

1. `git clone git@github.com:requaos/harvey.git && git clone git@github.com:requaos/r9.git` (sibling dirs — r9 resolves Harvey via `R9_HARVEY_DIR`, default `../harvey`).
2. Nix with flakes (NixOS: `nix.settings.experimental-features = [ "nix-command" "flakes" ]`). Everything else — qemu, expect, plan9port, the vendored toolchain, Rust (r9's rust-overlay devshell) — comes from the flakes. `nix develop` in each repo.
3. Smoke the composition: `nix run .#boot-r9-smoke` (expect-gated full boot to `/bin/rc`).
4. The 9front donor re-fetches at pin `350266104f1fed81c78c3d05fa44f523bec0ae4d` (hard-coded + verified in `build/aarch64/{donor-vm-build,donor-ns-build,pre-gate-build,run-gate}.sh`) — nothing donor-side needs to travel between machines.
5. Lima is **not** needed on linux — the namespace build runs natively (`sys/src/RUN_BUILD_IN_PRIVATE_NAMESPACE`; `subset-build.sh`'s NATIVE_LINUX leg via nsrun.sh sudo-unshare).

## What changes vs darwin (and what doesn't)

- **aarch64 guests are TCG on x86_64 — identical to darwin.** KVM is same-arch only, and darwin was already TCG (HVF rejects GICv2 on qemu 11.1.2). No guest-side velocity is lost by the move.
- **amd64 (9k) kernels gain KVM** on linux-x86_64 (`/dev/kvm`; the Phase-1 TCG acceptance and the k10t1 venue both speed up). minimega (`build/mm/`) — deferred in Phase 6 (D-03) for lack of an amd64 dev host — is now unblocked on paper.
- **No lima, no virtiofs workarounds** — the rsync-to-guest choreography is darwin-only. Native linux does in-place namespace builds (`build/aarch64/subset-build.sh` auto-selects the NATIVE_LINUX leg).
- **GUI venue:** Phase 10's trace leg and Phase 12's drawterm use drawterm's gui layer — **gui-x11 on linux** (needs an X server or Xwayland; NixOS Wayland desktops provide Xwayland), gui-cocoa on darwin. The 9front-lineage drawterm (nixpkgs) builds with either.
- **Case-sensitivity:** linux ext4 is case-sensitive — see below.

## Case-colliding tracked paths (darwin hazard, linux non-issue)

The repo tracks case-colliding pairs that cannot coexist on darwin's
case-insensitive APFS: `rc/bin/Kill` vs `rc/bin/kill`,
`sys/lib/troff/font/devutf/{HB,Hb}`, `{HI,Hi}`, `{HX,Hx}`,
`charlib/{LH,lh}`-class, `sys/src/cmd/postscript/devpost.add/{C1,c1}`-class.

On darwin, `git status` perpetually shows one member of each pair as modified
(the physical file serves two index entries). **Do not "fix" these on darwin** —
the index is right; the filesystem is lossy. On linux both materialize and
status is honest. (2026-11-06 audit: the Kill rewrite was committed via git
plumbing — `update-index --cacheinfo` — precisely because of this.)

## Migration audit log (2026-11-06, darwin-aarch64 → linux-x86_64)

What the pre-move audit found and where it landed:

| Item | Disposition | Commit |
|------|-------------|--------|
| 9 unpushed master commits (Phase 8 completion, review round, r9 pin 0a4ea82, Phase 10 context+plans) | pushed | — |
| r9 `exec.rs` MAX_ARGS 16→32 (root-cause fix for the 06-07/06-09 rc "pushword but no argv" residual class) | committed + pushed to r9 main, gates green (virtexec/virtmulti/virtip) | r9 `dbea82f` |
| r9 stash: `hfvd` HVF-race debug instrumentation (proc.rs/trap.rs prints) | dropped — darwin/HVF-only, marked temporary in its own comments; the GICv3 driver it hunted is long pushed (f9d43a9) | — |
| Orphan source edits (`rc/bin/Kill`, `rc/bin/juke`, `x509-ape.c` SHA-2 OIDs) — no session record | committed with honest provenance notes | `40846d759`, `a3a1d0427` |
| Build-churn regenerations (eqn.c yacc + host `#line` paths, 5c/enam.c, jpg/ycbcr.h host float drift, troff fonts, devpost C1-3) | reverted to committed | — |
| `06-PATTERNS.md`, `08-PATTERNS.md`, 08 probe JSONs, `.planning/state.json`, `config.json` | committed (PATTERNS files were referenced as canonical refs while untracked!) | `53caef171` |
| `.arvey-build/` (942MB scratch), `.planning/graphs/` (210MB derived KG), `.planning/tmp/`, plain-rate logs | gitignored | (this change) |
| In-tree full-tree build output flood (`6.out`/`7.out`/`*.a6`/`*.a7`, yacc `.c`, install-time copies into sys/lib, sys/man, rc/bin) | left untracked — all regenerable; a clean clone never has them | — |
| 9front donor checkout (`.build/9front-donor/`, gitignored) | re-fetches at the committed pin on the new machine | — |

## The x86_64 question (decided 2026-11-06): target pivot under consideration

User direction at migration: "resume development in an **x86_64-focused
direction** … also pivot target to x86_64." State capture happened first (this
document); the pivot itself is a **milestone-level re-scope conversation** to be
held on the new machine (likely `/gsd-new-milestone` or roadmap revision).

Context the pivot discussion needs:

- **r9 has an x86_64 crate** (`../r9/x86_64/`) — the port crate is shared; Phase 7/8's virtio/devether/devip work is `port/`-side (arch-neutral); the virt platform tree is `aarch64/src/platform/virt`. An x86_64 qemu-pc/virt platform would be new platform-layer work, not a rewrite of the net stack.
- **Harvey's amd64 userspace is already green** — v1.0 kept amd64 sacred and Phase 6 (G-06-1) made Harvey's own 9k amd64 kernel the x86_64 CI venue (k10t1). An amd64-userspace-on-r9 path reuses the tier machinery with `objtype=amd64` (the tier data files are arch-parameterized by construction — the filters key off `$objtype`).
- **The aarch64-locked artifacts:** `aarch64/ABI.md` (TrapFrame≡Ureg contract), the vendored 7a/7c/7l toolchain work, the aarch64 syscall ABI ledger (NMOUNT=54 divergence), the GICv2/GICv3 + DTB platform work. An x86_64 pivot re-derives equivalents (APIC/HPET, no DTB — r9 x86_64 boot protocol), it does not reuse them.
- **The drawterm/auth/cpu payoff (Phases 11–12) is arch-agnostic** once a kernel+userspace pair boots — the p9sk1 domain, cpu service, and rio session don't care which arch serves them.
- **KVM on linux-x86_64 accelerates x86_64 guests natively** — the strongest practical argument for the pivot (every aarch64 gate today is TCG-paced).
- **Current state:** Phase 8 complete (devip /net green on aarch64 TCG), Phase 10 fully planned (aarch64 tiers), Phase 9 not started. A pivot decision should land before Phase 10 execution begins, since 10-01..10-10 are aarch64-keyed (`7c` compiles, aarch64 donor gates).

## Standing host facts (all venues)

- darwin devshell/builds work; the lima VM path remains for namespace builds on macOS (documented-manual).
- Hosted CI is x86_64-linux (ubuntu-latest) — green legs are the clean-checkout proof of record.
- plan9port `$PLAN9/bin` LAST in PATH (BUILD_ON_UNIX) — applies on every host.
- qemu 11.1.2 drift accommodations live in `build/aarch64/README.md`; re-verify only if the qemu version changes.
