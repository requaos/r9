---
phase: 01-stage-p0-cross-build-harness
plan: "01"
subsystem: infra
tags: [nix, sandbox, proot, userns, harvey, plan9port, lima, cross-build]

requires:
  - phase: none
    provides: first plan of the phase; no prior phase output
provides:
  - S1 verdicts for the sandboxed build venue (unshare refused, proot passes)
  - spikes/s1 venue.nix, hostbin.nix and the pinning flake (both Linux systems)
  - the measured host-toolchain port recipe (four sandbox adjustments)
  - amd64 libc proof artifact facts and the nix store verify method
affects: [pass 2 planning (D-03 tree derivation), Plan 06 witness run, D-04 mechanism choice]

actuals:
  tokens: 10330
  tasks: 3
  commits: 2

tech-stack:
  added: [proot 5.5.0, plan9port (nixpkgs), util-linux unshare, GNU ed, gnused]
  patterns:
    - "proot -0 -r <root> -b leaf-mounts as the capability-free chroot venue"
    - "SANDBOX-EVIDENCE block + /usr/bin/env-must-be-absent guard before any venue"
    - "Harvey source copied to a writable work dir; the store path never written"

key-files:
  created:
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/venue.nix
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/hostbin.nix
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/flake.nix
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/flake.lock
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/S1-FINDINGS.md
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/evidence/builder-setup.txt
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/evidence/rung1-builder.log
    - .planning/phases/01-stage-p0-cross-build-harness/spikes/s1/evidence/rung2-builder.log
  modified: []

key-decisions:
  - "Builder venue is the existing lima VM harvey (owner reply 'lima'); no FlakeHub login, no Mac /etc/nix edit, and the Mac's only change was starting an already-running VM"
  - "unshare -Ur -m is unusable on this venue (nested uid_map EPERM inside the nix sandbox; Ubuntu 24.04 AppArmor userns restriction outside it), so the spike records a FAIL leg and uses proot as the plan's fallback (a)"
  - "A1 is REFUTED for the unshare venue; D-03 stays viable through proot, with the ptrace overhead measured at 2 s for the amd64 libc tier"
  - "The host toolchain port needs four sandbox adjustments: store-bash shebang for the 9l shim, pkgs.ed, plan9port last on PATH, GNU sed shadowing for 7l/BUILD"

patterns-established:
  - "Sandbox proof: print id/uid_map/ns-user/uname and fail when /usr/bin/env exists — a non-sandboxed build can never be recorded as a pass"
  - "Guest work directory + leaf binds + never rm a scaffold that binds a source tree (Harvey's SAFETY rule)"
  - "The tree derivation's outputs must exclude the 23 case-colliding Harvey paths (Pitfall 3)"

requirements-completed: [HARN-07, HARN-02]

coverage:
  - id: D1
    description: "Rung 1: the sandbox venue runs Harvey's tmpfs/chroot work inside a nix build sandbox and records sandbox evidence"
    requirement: HARN-07
    verification:
      - kind: integration
        ref: "nix build path:$S/s1#packages.aarch64-linux.s1-nsprobe-proot --store ssh-ng://lima-harvey (S1-RUNG1-SYS-OK / RC-OK / MK-OK, S1-VENUE-OK)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Rung 1 negative leg: nested unshare -Ur -m is refused on the builder, with the exact error and the builder's sysctls recorded"
    verification:
      - kind: integration
        ref: "nix build path:$S/s1#packages.aarch64-linux.s1-nsprobe --store ssh-ng://lima-harvey (unshare: write failed /proc/self/uid_map: Operation not permitted)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Rung 2: Harvey's host toolchain (6a..8l, cpp, nm, ar, pcc) and the amd64 libc build in the sandbox; libc.a facts and nix store verify recorded"
    requirement: HARN-02
    verification:
      - kind: integration
        ref: "nix build path:$S/s1#packages.aarch64-linux.s1-{hostbin,libc-amd64} (S1-HOSTBIN-OK, S1-RUNG2-LIBC-OK, 259 members, store verify exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The S1 flake evaluates for x86_64-linux so Plan 06's owner witness run builds the same derivations"
    verification:
      - kind: other
        ref: "nix eval --raw path:$S/s1#packages.x86_64-linux.{s1-nsprobe,s1-hostbin,s1-libc-amd64}.drvPath"
        status: pass
    human_judgment: false
  - id: D5
    description: "Witness legs on the owner's x86_64 KVM host (rung 1 and rung 2) — assigned to Plan 06"
    human_judgment: true
    rationale: "x86_64 Linux with a writable /dev/kvm is the owner's machine; the Mac cannot run it, and the plan assigns both witness legs to Plan 06."
    verification: []

duration: 13min
completed: 2026-11-22
status: complete
---

# Phase 1 Plan 1: S1, the sandboxed build venue

**Rung 1 proves Harvey's tmpfs-chroot venue runs inside a nix build sandbox via proot (unshare is refused by the builder); rung 2 builds Harvey's host toolchain and the amd64 libc in it in 2 s, with a clean `nix store verify`.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-11-22T19:57:00-05:00
- **Completed:** 2026-11-22T20:10:00-05:00
- **Tasks:** 3 (1 owner checkpoint, 2 execution)
- **Files modified:** 8

## Accomplishments

- Owner picked `lima` at Task 1; the only machine change was starting the
  already-running VM, and the one config edit (guest `trusted-users`) is
  recorded with its exact revert in `builder-setup.txt`.
- Rung 1 answers A1 with evidence: `unshare -Ur -m` fails twice (nested
  `uid_map` EPERM inside the nix sandbox; Ubuntu 24.04's
  `kernel.apparmor_restrict_unprivileged_userns=1` outside it), while the
  same guest script passes through `proot` — `/sys` write-back, plan9port
  `rc`, plan9port `mk` on a tmpfs mkfile.
- Rung 2 ports `nix/setup.sh` + `nix/toolchain.sh` into a derivation that
  produces `6a 6c 6l 7a 7c 7l 8a 8c 8l cpp nm ar pcc` and then builds
  Harvey's amd64 `libc` (259 members, 601170 bytes, 2 s) inside the venue.
- The S1 flake pins Harvey `bd7a698` and r9's nixpkgs rev, and its
  `x86_64-linux` attributes evaluate, so Plan 06's witness run builds the
  same derivations.

## Task Commits

Each task was committed atomically:

1. **Task 1: Owner chooses the spike builder venue** - no commit (owner reply
   `lima`; recorded in the Task 2 commit's `S1-FINDINGS.md` as
   `BUILDER-VENUE: lima (owner, 2026-11-22)`)
2. **Task 2: S1 rung 1 end to end** - `1ae4d2c` (docs)
3. **Task 3: S1 rung 2 hostbin + amd64 libc** - `a26ba8c` (docs)

**Plan metadata:** `see git log` (docs: complete plan)

## Files Created/Modified

- `spikes/s1/venue.nix` - the venue function: work-copy, `SANDBOX-EVIDENCE`
  guard, `unshare` and `proot` modes, transcript and `/out` artifacts
- `spikes/s1/flake.nix` - pins nixpkgs `1c89083e` and Harvey `bd7a698`
  (`flake = false`), exposes `s1-nsprobe`, `s1-nsprobe-proot`, `s1-hostbin`,
  `s1-libc-amd64` for both Linux systems
- `spikes/s1/hostbin.nix` - Harvey's host toolchain bootstrap as a nix build
- `spikes/s1/flake.lock` - locks both inputs (Harvey rev present)
- `spikes/s1/.gitignore` - `work/`, `result*`
- `spikes/S1-FINDINGS.md` - verdicts, evidence pointers, recipe, A1/A8/A16,
  pass-2 implications, two `PENDING-OWNER` witness legs
- `spikes/s1/evidence/builder-setup.txt` - bring-up commands and outputs (no
  credential material)
- `spikes/s1/evidence/rung1-builder.log`, `.../rung2-builder.log` - build
  excerpts (33 and 30 lines, both far under the 200-line cap)

## Decisions Made

- **Venue: lima, as the owner chose.** The executor never handled a FlakeHub
  credential and never edited the Mac's `/etc/nix`; the guest-only
  `trusted-users` line carries its revert in the evidence file.
- **proot is the venue, not unshare.** The plan's fallback (a) is the only
  route on this builder; the flake keeps the unshare derivation so the FAIL
  leg stays reproducible.
- **nix's remote store needs `NIX_SSHOPTS="… -oControlPath=none"` with
  lima's `ssh.config`.** Without it nix's `ssh -O check` probe finds lima's
  own master socket and the daemon handshake dies with
  `protocol mismatch, got 'started … oixd'`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] 9l shim shebang does not exist in the nix sandbox**
- **Found during:** Task 3 (hostbin build)
- **Issue:** `nix/toolchain.sh`'s `9l` shim uses `#!/usr/bin/env bash`; the
  sandbox has no `/usr/bin`, so every link died with `bad interpreter`.
- **Fix:** the shim is generated with the store's bash path as its shebang.
- **Files modified:** `spikes/s1/hostbin.nix`
- **Verification:** the full toolchain built; `S1-HOSTBIN-OK`.
- **Committed in:** `a26ba8c`

**2. [Rule 3 - Blocking] `ed` is absent in the nix sandbox**
- **Found during:** Task 3 (7l bootstrap)
- **Issue:** `7l/mkcname` writes `cnam.c` with `ed - l.h`; Harvey's devshell
  inherited the host's `/usr/bin/ed`, the sandbox has none, so `cnam.c` was
  never created and `9c` died on it.
- **Fix:** `pkgs.ed` is an explicit input.
- **Files modified:** `spikes/s1/hostbin.nix`
- **Verification:** `7l` built; the expected tool set is complete.
- **Committed in:** `a26ba8c`

**3. [Rule 3 - Blocking] plan9port must be LAST on PATH, not first**
- **Found during:** Task 3 (hostbin build)
- **Issue:** putting `plan9/bin` first shadows the host tools; Harvey's own
  flake wrapper appends it for exactly this reason ("plan9port ships a 9p
  `mount` which must not shadow util-linux mount").
- **Fix:** `PATH=<hostbin>:<normal>:<plan9/bin>` in the hostbin build and
  `PATH=<normal>:<plan9/bin>` in the venue.
- **Files modified:** `spikes/s1/hostbin.nix`, `spikes/s1/venue.nix`
- **Verification:** rung 1 and rung 2 both re-ran green after the change.
- **Committed in:** `a26ba8c`

**4. [Rule 1 - Bug] Harvey's `7l/BUILD` `_SYSRM` sed line is malformed at the pin**
- **Found during:** Task 3 (7l bootstrap)
- **Issue:** the line ends `s/::*$` with no replacement, so the pinned
  plan9port sed answers `sed: Too much text` and GNU sed answers
  `unterminated 's' command`. The line tolerates the failure (`|| echo rm`),
  but the following, well-formed `_HOSTNM` line needs a working GNU sed.
- **Fix:** shadow `sed` with GNU sed for the bootstrap (the same class of fix
  Harvey's `build.sh` makes with its exported `GNU_SED`).
- **Files modified:** `spikes/s1/hostbin.nix`
- **Verification:** `7l` built and the whole toolchain reached
  `S1-HOSTBIN-OK`; the bug is recorded in `S1-FINDINGS.md` for pass 2.
- **Committed in:** `a26ba8c`

---

**Total deviations:** 4 auto-fixed (3 blocking, 1 bug)
**Impact on plan:** all four are ports of Harvey's host-side assumptions into
a sandbox; none widens scope, none touches Harvey's checkout, and none uses
sudo. The unshare FAIL is a result, not a deviation.

### Deliberate protocol deviation

**Requirements checkboxes stay Pending.** The plan's frontmatter lists
`HARN-07` and `HARN-02`, and `requirements.mark-complete` flipped both to
Complete in `REQUIREMENTS.md`. That was reverted: the owner's recorded pass-1
decision (STATE.md, D-22) says no HARN requirement is implemented until
pass 2, and this plan delivers spike evidence only — no devshell export, no
xtask resolver. The SUMMARY keeps the two IDs as the plan's declared scope;
`REQUIREMENTS.md` keeps them Pending so pass 2 is the plan that closes them.

## Issues Encountered

- The lima VM was already running, so `limactl start harvey` was a no-op;
  recorded in `builder-setup.txt` rather than restarting the owner's VM.
- nix's ssh-ng store and lima's `ControlMaster auto` conflict (see Decisions);
  `-oControlPath=none` resolves it and is recorded for every later plan.

## User Setup Required

None for this plan. The owner's FlakeHub login was NOT needed because the
owner chose `lima`.

## Next Phase Readiness

- Pass 2 can design D-03's tree derivation against a measured venue: proot
  inside the nix sandbox, with the four host-toolchain ports and the
  case-collision exclusion (Pitfall 3) as hard requirements.
- Plan 06 must still run the two `PENDING-OWNER` witness legs on the owner's
  x86_64 KVM host; the flake already evaluates for `x86_64-linux`.

---
*Phase: 01-stage-p0-cross-build-harness*
*Completed: 2026-11-22*

## Self-Check: PASSED

All nine spike artifacts exist and are non-empty; both task commits
(`1ae4d2c`, `a26ba8c`) are ancestors of HEAD.
