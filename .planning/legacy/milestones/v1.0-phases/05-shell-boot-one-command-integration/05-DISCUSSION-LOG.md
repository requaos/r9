# Phase 5: Shell Boot + One-Command Integration - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-02
**Phase:** 5-shell-boot-one-command-integration
**Areas discussed:** Boot path to rc, Shell environment scope, Flake composition + CI, Known-flake policy

---

## Boot path to rc

| Option | Description | Selected |
|--------|-------------|----------|
| Staged init exec's rc directly | Minimal path: staged init gains one final job — exec /bin/rc with preopened console fds; no bind/mount/env machinery | |
| Mini-init with setup, then rc | Grow staged init into mini-init doing Plan 9-ish setup (env, namespace) first | |
| Port the real Harvey init/boot chain | Full fidelity: real boot/init/rcmain flow | ✓ |

**User's choice:** Port the real Harvey init/boot chain

| Option | Description | Selected |
|--------|-------------|----------|
| Real init as pid 1, skip boot(8) | Kernel execs real /bin/init; boot(8)'s job (auth, root mount) has no meaning with embedded root | |
| Include a no-op boot stage | Stripped boot(8) that prints and execs init | |
| Full boot(8) with auth stubs | boot.c's real flow with authentication paths stubbed | ✓ |

**User's choice:** Full boot(8) with auth stubs
**Notes:** Agent scoped boot.c beforehand: doauthenticate degrades gracefully without auth (fsession trbuf[0]==0 no-op; nsinit falls through to unauthenticated mount).

| Option | Description | Selected |
|--------|-------------|----------|
| bind yes, mount/devmnt no | r9 grows BIND + per-process namespaces; embedded method skips the 9P root mount | |
| Full mount/devmnt too | bind + mount + devmnt; boot.c runs unmodified; sets up virtio-9p prerequisite | ✓ |
| Kernel-fixed namespace, bind as no-op | Kernel pre-populates pid 1's namespace; bind trivially succeeds | |

**User's choice:** Full mount/devmnt too
**Notes:** Agent surfaced the `sys/src/9k/boot/paq.c` precedent mid-discussion — connectpaq pipes + execs a userspace paqfs 9P server and returns the fd for mount via devmnt; adopted as the embedded-root shape (D-04).

| Option | Description | Selected |
|--------|-------------|----------|
| init -t + scoped termrc → rc -i | Faithful terminal path; scoped /rc/bin/termrc (no ipconfig/ndb/rio) ends in exec rc -i → rcmain → '% ' prompt | ✓ |
| init cmd-hook → rc -i directly | init's argv hook execs rc -c 'exec rc -i'; no termrc file | |
| You decide (trace-first) | Researcher traces donor boot-to-prompt path; planner picks | |

**User's choice:** init -t + scoped termrc → rc -i
**Notes:** rc always bootstraps /rc/lib/rcmain (compiled-in, exec.c:216); interactive path is `. -i '#d/0'` — devdup mandatory.

---

## Shell environment scope

| Option | Description | Selected |
|--------|-------------|----------|
| Cooked line discipline + DEL note | Canonical input, erase/kill-line, DEL interrupt note; no consctl | |
| Full devcons incl. consctl/raw | The real devcons including raw-mode switching | ✓ |
| Raw passthrough | Per-keystroke reads, no editing | |

**User's choice:** Full devcons incl. consctl/raw

| Option | Description | Selected |
|--------|-------------|----------|
| Real per-pgrp env (RFENVG semantics) | devenv keyed by process group; RFENVG shares, RFCENVG copies | ✓ |
| Single global env table | One env table; RFENVG ignored | |
| You decide (trace-first) | Researcher reads 9front devenv + trace | |

**User's choice:** Real per-pgrp env (RFENVG semantics)

| Option | Description | Selected |
|--------|-------------|----------|
| Ship scoped /lib/namespace, newns runs | Per-login namespace rebuild exercised end-to-end | ✓ |
| Boot-built namespace only, newns degrades | init tolerates newns failure; rc inherits boot's namespace | |
| You decide (trace-first) | Researcher diffs /lib/namespace against r9 device set | |

**User's choice:** Ship scoped /lib/namespace, newns runs

| Option | Description | Selected |
|--------|-------------|----------|
| Prompt + scripted command sweep | Smoke types echo/ls/ps/cat and asserts markers; SC#2 as asserted evidence | ✓ |
| Prompt + single echo marker | INT-02 literal minimum + input liveness | |
| Prompt only | Prompt assertion; command verification manual | |

**User's choice:** Prompt + scripted command sweep

---

## Flake composition + CI

| Option | Description | Selected |
|--------|-------------|----------|
| Harvey app composes everything | Harvey flake app builds userland, pulls r9 kernel from flake input, bakes initrd, launches qemu; r9→Harvey glue inverts | ✓ |
| r9 owns boot app, Harvey wraps it | r9 composite app takes userland dir as arg | |
| You decide (map-first) | Researcher maps both flakes | |

**User's choice:** Harvey app composes everything

| Option | Description | Selected |
|--------|-------------|----------|
| kernel package + xtask package | Harvey runs r9's xtask for initrd/boot-test | |
| kernel package only, Harvey bakes+launches | r9 exposes packages.kernel-virt; Harvey bakes with own mkpaqfs and launches qemu itself | ✓ |
| You decide (map-first) | Planner picks after mapping xtask needs | |

**User's choice:** kernel package only, Harvey bakes+launches

| Option | Description | Selected |
|--------|-------------|----------|
| Expect script + flake app in Harvey | expect (in devshell, Phase 1 lineage) drives composed boot; r9 boot-test unchanged | ✓ |
| r9 boot-test virtshell profile | Rust socket harness gains shell profile | |
| You decide (map-first) | Planner picks | |

**User's choice:** Expect script + flake app in Harvey

| Option | Description | Selected |
|--------|-------------|----------|
| GH Actions matrix, full pipeline both legs | ubuntu + macos-14, full pipeline each | |
| Matrix with macos-tuned timeouts | Same shape, macos serialized/longer timeout | |
| Self-hosted darwin runner | M5 Max dev fleet for the darwin leg | |

**User's choice (free text):** "GH Actions cover the Linux leg full pipeline. No need to validate the darwin leg in CI"
**Notes:** Amends ROADMAP SC#3's literal "CI green on both" — CI-green = linux full pipeline; darwin verified by documented manual command (Phase 3 D-10 pattern).

---

## Known-flake policy

| Option | Description | Selected |
|--------|-------------|----------|
| Debug-first Wave 0 | /gsd-debug-style wave on the note-timeout race before the shell path depends on it | ✓ |
| Fix on exposure | Build shell path first; debug only if smoke flakes | |
| Parallel bounded spike | Timeboxed investigation alongside the main line | |

**User's choice:** Debug-first Wave 0

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit stack-growth smoke binary | Deep-recursion 7c binary forces stack page-ins, prints marker | ✓ |
| rc is the test | rc's parser is the real-world exercise; fix on exposure | |
| You decide (trace-first) | Planner judges from donor trace | |

**User's choice:** Explicit stack-growth smoke binary

| Option | Description | Selected |
|--------|-------------|----------|
| Only the two decided items | Note-race Wave 0 + stack-growth smoke only | |
| Plus qemu cleanup (#2) | Harness qemu cleanup explicitly in scope for CI reliability | ✓ |
| Planner triages at plan time | Per-item triage while sequencing waves | |

**User's choice:** Plus qemu cleanup (#2)

---

## the agent's Discretion

- Embedded boot-method naming/argv contract and method-file placement (additive, modeled on paq.c)
- Auth stub granularity (objtype-gated file swap vs runtime degrade) — contract: never fatal in the embedded path
- paqfs image root-tree skeleton beyond the mandated files
- devproc file surface beyond status/args (+ note, + ctl) — trace-driven
- Expect-script mechanics, marker strings, flake app naming, wave ordering (subject to debug-first)
- qemu cleanup (#2) implementation mechanics

## Deferred Ideas

- virtio-9p host mount — v1.x top candidate; this phase's devmnt is its prerequisite
- boot(8) disk/network methods (local fossil/kfs, tcp) — hardware/network era
- Real auth (factotum handshake, auth server, secstore) — security milestone
- factotum/ipconfig in the boot image — network-era concern
- SMP/PSCI, virtio-net/blk, RTC, PCIe ECAM — v2
- rio/acme/devdraw/devmouse — graphical era
- amd64-on-r9 — v2
- 9k-amd64 pool-panic re-test (phase-02 deferred #8) — amd64-side debt
- Phase-04 deferreds #3/#4 + 8 open Info findings — milestone backlog
- raspi3b-on-qemu-11.1.2 upstream defect — pre-existing, human-accepted
- HVF acceleration — needs GICv3 driver
- Upstream contribution flow to original r9
