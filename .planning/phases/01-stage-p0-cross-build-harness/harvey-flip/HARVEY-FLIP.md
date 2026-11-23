# The prepared Harvey change (D-10 step 4)

**Status: APPROVED, APPLIED and PUSHED — committed as `c697666ba315f137ca2fd11768f835d04bd9f018` and pushed to `origin/master` (`master`) by the owner on 2026-11-23.**
The owner approved the prepared diff and the four HARN-08 equivalence
expectations on 2026-11-23, reported the standalone amd64 TCG gate
`GATE-PASS`, and the diff was applied to `/Users/neilskinner/code/harvey`
from base `25d97dc4d4976bfb7411f01970755acf6261c81b`. The owner push
completed the flip: `origin/master` now carries `c697666ba` (and the
pre-existing `25d97dc4d` freeze commit).

## Stop C2/C3 outcome (plan 01-34) — APPROVED, APPLIED AND PUSHED

- **Gates:** stop C2 full 8/8 with `fpprobe: ok` + tap stress PASS; stop
  C3 soak / equiv (all 10 rows match) / kernel (the `b6Oc` boot-iso hash,
  BUILD + REBUILD OK) / store_verify PASS; the standalone amd64 TCG gate
  is `GATE-PASS` (owner report).
- **HARN-08 equivalence:** APPROVED 2026-11-23 owner — tier1 `T1-OK` 39,
  auth `AUTH-OK` 10, churn `CATS-DONE 1400` 1, stress `netstress: venue
  tap` 1.
- **Harvey diff:** APPROVED 2026-11-23 owner (sha256
  `f7b0272d23bc6c131c2f63b37cde1330c0bebddd25b905c22040829232cd7f22`,
  511797 bytes, 36 paths, +285/−9972).
- **Applied and committed** from base `25d97dc4d4976bfb7411f01970755acf6261c81b`
  with `git apply` + an explicit 36-path `git add --`; the 11 pre-modified
  font paths were never staged. The commit's stat matches the prepared
  diff (36 files, +285/−9972) and carries zero `sys/lib/(postscript|troff)`
  or `devpost.add` paths.

HARVEY-COMMIT: c697666ba315f137ca2fd11768f835d04bd9f018
HARVEY-APPROVED: 2026-11-23 owner
HARVEY-PUSHED: master c697666ba315f137ca2fd11768f835d04bd9f018
HARVEY-PUSHED-REPORTED: 2026-11-23 owner (verified: `git ls-remote origin refs/heads/master` = c697666ba315f137ca2fd11768f835d04bd9f018)

Owner push (branch `master`, completed by the owner on 2026-11-23):

    git -C /Users/neilskinner/code/harvey push origin HEAD:master

The push published the pre-existing planning-freeze commit `25d97dc4d`
as well — the owner already owned that decision.

## Stop C outcome (plan 01-27, historical)

- **Return:** `witness-src/01-27` at
  `6346c68cd67264f6a58fd72369c0dc8523cc9ffe` (source HEAD `3f717f7…`,
  `teeniebox`, 2026-11-23T17:37:29Z → 18:32:12Z).
- **Verdicts:** `stopc-full` FAIL (7/8; `boot-r9-tier1-x64` `T1-FAIL
  fpprobe`), `stopc-soak` FAIL (tap precondition: the `HARVEY` seam was not
  set), `stopc-stress` FAIL (same), `stopc-equiv` FAIL (capture gaps plus
  `boot-r9-x64` markers 0/1), `stopc-store-verify` PASS.
- **No Harvey commit:** the owner's checkout is untouched — HEAD
  `25d97dc4d4976bfb7411f01970755acf6261c81b`, status-porcelain sha256
  `041d2929…`; nothing was staged or committed.
- **HARVEY-COMMIT (stop C, superseded by the C2/C3 section above): none** — not made (stop C not green; no owner approval).
- **HARVEY-APPROVED (stop C, superseded): none** — the owner did not approve the diff then.
- **HARVEY-PUSHED (stop C, superseded): none** — the owner push (Task 3) was not reached then.

The diff remains at `$HOME/r9-harvey-flip/harvey-flip.diff` — 511797 bytes,
sha256 `f7b0272d23bc6c131c2f63b37cde1330c0bebddd25b905c22040829232cd7f22` —
as the reviewed record of the applied commit. See
`witness/WITNESS-LOG.md` "Stop C2/C3" and `witness/evidence/stop-c2|stop-c3/`
for the green verdicts and the approval record.

## Where the change lives

| Item | Path / value |
|---|---|
| Scratch clone | `$HOME/r9-harvey-flip/harvey` |
| Clone source | `git clone --no-hardlinks /Users/neilskinner/code/harvey` — reads the owner's checkout, never writes to it |
| Base | `25d97dc4d4976bfb7411f01970755acf6261c81b` (the owner's `master`, ahead 1) |
| Full diff | `$HOME/r9-harvey-flip/harvey-flip.diff` |
| Diff size / sha256 | 511797 bytes / `f7b0272d23bc6c131c2f63b37cde1330c0bebddd25b905c22040829232cd7f22` |
| Owner's checkout | **untouched**: `HARVEY-UNTOUCHED` holds (HEAD `25d97dc4d` + status hash `041d2929…`) |

Neither the clone nor the diff lives in a repository.

## DIFFSTAT

DIFFSTAT:

```
 36 files changed, 285 insertions(+), 9972 deletions(-)

 .github/workflows/phase6.yml      |  24 +-
 .github/workflows/r9-shell.yml    | 145 -----------
 build/aarch64/cpu-r9.expect       | 172 -------------
 build/aarch64/drive-r9.expect     | 231 -----------------
 build/aarch64/smoke-r9-net.expect | 437 --------------------------------
 build/aarch64/smoke-r9.expect     | 307 -----------------------
 build/aarch64/tier1-r9.expect     | 240 ------------------
 build/x86_64/churn-r9.expect      | 300 ----------------------
 build/x86_64/cpu-bringup.expect   | 276 --------------------
 build/x86_64/netsoak-r9.expect    | 336 -------------------------
 build/x86_64/services-r9.expect   | 241 ------------------
 build/x86_64/smoke-r9-auth.expect | 515 --------------------------------------
 build/x86_64/smoke-r9-net.expect  | 504 -------------------------------------
 build/x86_64/smoke-r9.expect      | 462 ----------------------------------
 build/x86_64/stress-r9.expect     | 359 --------------------------
 build/x86_64/tier1-r9.expect      | 211 ----------------
 flake.lock                        |  59 +----
 flake.nix                         | 472 +++-------------------------------
 nix/boot-r9-x64.sh                | 374 ---------------------------
 nix/boot-r9.sh                    | 231 -----------------
 nix/churn-r9-x64.sh               | 186 --------------
 nix/cpu-r9.sh                     | 218 ----------------
 nix/drawterm-gate-x64.sh          | 259 -------------------
 nix/host-echo.c                   | 233 -----------------
 nix/net-doctor.sh                 | 442 --------------------------------
 nix/q35.sh                        | 249 ++++++++++++++++++
 nix/services-r9-x64.sh            | 250 ------------------
 nix/smoke-r9-auth-x64.sh          | 267 --------------------
 nix/smoke-r9-net-x64.sh           | 253 -------------------
 nix/smoke-r9-net.sh               | 241 ------------------
 nix/smoke-r9-x64.sh               | 246 ------------------
 nix/smoke-r9.sh                   | 255 -------------------
 nix/soak-r9-net-x64.sh            | 363 ---------------------------
 nix/stress-r9-net-x64.sh          | 356 --------------------------
 nix/tier1-r9-x64.sh               | 246 ------------------
 nix/tier1-r9.sh                   | 297 ----------------------
```

The diff is generated as `git diff HEAD -- . ':(exclude)sys/**'` in the
scratch clone, plus the intent-to-add of the new `nix/q35.sh`. The `sys/**`
exclusion is deliberate: the scratch clone sits on a case-insensitive APFS
volume, so Harvey's 11 case-colliding `sys/lib/...` + `devpost.add` paths
(the pre-modified font set D-02 protects) materialise as `M` on checkout.
They are **not** part of this change — see "Untouched" below.

## What the change does

### `flake.nix` — the flip (D-02, D-07, D-10, D-19, D-20, D-26)

- Drops `inputs.r9` (and its comment block) and `r9` from the `outputs`
  arguments. The flake's only input is `nixpkgs` again.
- Removes every package and app that consumes r9 or exists only for the
  moved gates: `bootR9Pkg`, `smokeR9Pkg`, `netSmokeR9Pkg`, `tier1R9Pkg`,
  `cpuR9Pkg`, `bootR9X64Pkg`, `smokeR9X64Pkg`, `tier1R9X64Pkg`,
  `netSmokeR9X64Pkg`, `servicesR9X64Pkg`, `netSoakR9X64Pkg`,
  `churnR9X64Pkg`, `stressR9X64Pkg`, `authSmokeR9X64Pkg`,
  `drawtermGateX64Pkg`, plus the gates-only `netDoctorPkg` and
  `hostEchoX64Pkg`; and the `optionalAttrs` blocks that registered
  `boot-r9*`, `drawterm-gate-x64`, `net-doctor` and the x64 apps.
- Keeps everything D-08/D-20 keeps: the `harness`, `minimega` and
  `drawterm` packages, the `setup`, `toolchain`, `build`, `iso` and
  `default` apps, the `drawterm` app, and the devshell. None consumes an r9
  attribute.
- Adds the one new command (D-26): app `q35` →
  `mkScriptApp pkgs "harvey-q35" ./nix/q35.sh`, with the build app's
  platform split plus `qemu`, `git`, `coreutils` and `gnugrep` in its
  runtimeInputs. It is registered for all three systems.

### `nix/q35.sh` — the new own-kernel launcher (D-26)

Built from `spikes/s5/boot-q35.sh`, the harness that proved the Mac TCG and
x86_64 KVM legs:

- builds `amd64/9k10cpu.elf` through Harvey's existing platform build when
  the kernel is absent or `--build` is given (`nix/build.sh` on Linux,
  `nix/vm-build.sh` on darwin); `--no-build` skips it;
- records the kernel's path, sha256 and size (`Q35-KERNEL`);
- boots `qemu-system-x86_64 -M q35` with virtio-net and a serial capture
  (`.build/q35-serial.log`), classifies the `Plan 9` banner, the
  `root is from (tcp)[tcp]:` prompt and the 01-04 signature set
  (`bad magic`, `panic`, `suicide: sys: trap`);
- kills and reaps qemu on every exit (EXIT/INT/TERM/HUP trap);
- uses KVM when `/dev/kvm` is writable, otherwise requires an explicit
  `HARVEY_Q35_ACCEL=tcg` and **never** downgrades silently (D-13).

`HARVEY` defaults to the git toplevel of `PWD` (else `PWD`); run the app
from the Harvey checkout root (`nix run .#q35`) or set `HARVEY` — invoked
from inside another checkout it fails loudly (`not-a-checkout`) rather than
booting the wrong tree.

### Deletions (the moved files, D-07/D-19)

16 gate scripts, `nix/host-echo.c`, all 10 `build/x86_64` and all 5
`build/aarch64` expect drivers:

| Directory | Files |
|---|---|
| `nix/` (16) | `boot-r9.sh`, `boot-r9-x64.sh`, `churn-r9-x64.sh`, `cpu-r9.sh`, `drawterm-gate-x64.sh`, `net-doctor.sh`, `services-r9-x64.sh`, `smoke-r9.sh`, `smoke-r9-x64.sh`, `smoke-r9-net.sh`, `smoke-r9-net-x64.sh`, `smoke-r9-auth-x64.sh`, `soak-r9-net-x64.sh`, `stress-r9-net-x64.sh`, `tier1-r9.sh`, `tier1-r9-x64.sh` + `host-echo.c` |
| `build/x86_64/` (10) | `churn-r9.expect`, `cpu-bringup.expect`, `netsoak-r9.expect`, `services-r9.expect`, `smoke-r9.expect`, `smoke-r9-net.expect`, `smoke-r9-auth.expect`, `stress-r9.expect`, `tier1-r9.expect`, `drawterm-gate.expect` **← kept as an exception, see below** |
| `build/aarch64/` (5) | `cpu-r9.expect`, `drive-r9.expect`, `smoke-r9.expect`, `smoke-r9-net.expect`, `tier1-r9.expect` |

Their r9 homes are `nix/gates/` and `gates/x86_64|aarch64/` (D-07). The
guest-side fixtures (sweep `.rc` scripts, `root-tree`, tier lists, fixture
`.c` sources) stay untouched in Harvey (D-19).

### CI

- `.github/workflows/r9-shell.yml` — **deleted whole**: every job
  (`boot-smoke`, `net-smoke`, `cpu-smoke`, `net-smoke-x64`,
  `auth-smoke-x64`) is an r9 leg.
- `.github/workflows/phase6.yml` — the `tier-r9` job is removed (its only
  r9 leg), with the header block and the venue-pin echo line updated so
  the file describes the jobs it has. The non-r9 jobs (`wave0-host`,
  `tier-2`, `tier-9k`, `x86_64-poolpanic`) and every functional assertion
  are unchanged.

## Kept exceptions

| File | Referencing file | Why it is kept |
|---|---|---|
| `build/x86_64/drawterm-gate.expect` | `nix/drawterm-x64.sh` (retained `drawterm` app, D-20) | The retained drawterm wrapper names the gate as the source of its `-s ''` pin (`nix/drawterm-x64.sh:22`). The plan names this the expected exception; the file stays. |

No other moved file is referenced by a retained **executable** path. The
per-file reference grep (`git grep -F <basename>` over the retained tracked
files) also found the documentation/comment mentions listed below; they are
not uses and are left untouched because this change's scope is the flake,
the moved files and the r9 CI legs:

| Deleted file | Retained file mentioning it (comment/doc only) |
|---|---|
| `nix/boot-r9-x64.sh`, `nix/net-doctor.sh`, `nix/soak-r9-net-x64.sh`, `nix/host-echo.c` | `build/x86_64/README.md` |
| `nix/boot-r9.sh` | `build/aarch64/run-r9.sh`, `build/x86_64/mkimage.sh`, `build/x86_64/README.md` |
| `nix/cpu-r9.sh` | `build/aarch64/repeat-gate.sh`, `build/aarch64/root-tree/rc/bin/cpurc` |
| `nix/smoke-r9-auth-x64.sh` | `build/x86_64/mkimage.sh` (an error message naming the app) |
| `nix/smoke-r9.sh` | `build/aarch64/subset-build.sh` |
| `nix/tier1-r9-x64.sh` | `build/x86_64/tier1-sweep.rc` |
| `nix/tier1-r9.sh` | `build/aarch64/tier1-sweep.rc` |
| `build/x86_64/smoke-r9-net.expect`, `build/aarch64/smoke-r9-net.expect` | `build/x86_64/root-tree/lib/netip-tcp.rc` |
| `build/x86_64/tier1-r9.expect`, `build/aarch64/tier1-r9.expect` | `build/x86_64/tier1-sweep.rc`, `build/x86_64/tiers/tier1-cmds.txt` |

A follow-up doc sweep in Harvey (README sections and the `.rc` headers) is
the owner's call; no code path reads these names.

## D-19 check — r9's read sets are untouched

The read sets (`nix/harvey/tree-amd64.nix`, `nix/harvey/tree-arm64.nix`,
`nix/harvey/reference.nix`) name 102 `nix/…`/`build/…` paths. Their
intersection with the deletion list is **empty**; no `.expect` and no
`nix/*.sh` appears in any read set:

```
comm -12 <(r9 read-set paths) <(delete set)  →  (no output)
grep -c '\.expect' r9-harvey-paths.txt       →  0
```

D-19-OK. The only r9-side textual mentions of the deleted basenames are
comments (`nix/harvey/tree-arm64.nix:567` about `tier1-r9.sh`'s T1-SKIP
computation; `xtask/src/main.rs:1205` about the aarch64 smoke's staging);
both describe the gate scripts r9 now owns under `nix/gates/`, not files
read from the pinned tree.

## Validations (Mac, aarch64-darwin, 2026-11-23)

| # | Command | Result |
|---|---|---|
| 1 | `nix flake lock` | exit 0. Removed input `r9` and its nodes `r9/nixpkgs`, `r9/rust-overlay`, `r9/rust-overlay/nixpkgs`. |
| 2 | `jq -r '.nodes.root.inputs \| keys[]' flake.lock` | `nixpkgs` only. `grep -c r9 flake.lock` → 0. |
| 3 | `nix flake show --all-systems --json "path:$HF/harvey"` | exit 0. `jq … \| grep -cE '^(boot-r9\|drawterm-gate-x64$\|net-doctor$)'` → **0**. Apps: `aarch64-darwin` = build/default/iso/q35/setup/toolchain; `aarch64-linux` and `x86_64-linux` = +drawterm. |
| 4 | `nix eval --raw "…#apps.<SYS>.{setup,toolchain,build,iso,q35}.program"` | EVAL-OK for all three systems; drawterm EVAL-OK for both Linux systems. |
| 5 | `nix eval` of `packages.{aarch64-linux.drawterm,x86_64-linux.minimega,aarch64-darwin.harness}.drvPath` and both devshells | EVAL-OK. |
| 6 | **Boot half:** copy the committed S5 kernel to `$HF/harvey/amd64/9k10cpu.elf` (sha256 `ecdcdadd…`, 2030226 bytes), then, **from `$HF/harvey`**, `HARVEY_Q35_ACCEL=tcg nix run "path:$HF/harvey#q35" -- --no-build` | exit 0. `Q35-RESULT machine=q35 accel=tcg banner=yes prompt=yes sigs=none`; prompt line `root is from (tcp)[tcp]:`; banner and prompt within 1 s. `grep -c 'root is from'` → 1. No qemu survivor (`pgrep -f 'qemu-system-x86_64.*9k10cpu'` → none). From any other cwd the app refuses loudly (`q35: error: not-a-checkout`) unless `HARVEY` names the checkout. |
| 7 | No-accel refusal on this KVM-less host: `env -u HARVEY_Q35_ACCEL nix run "path:$HF/harvey#q35" -- --no-build` | exit 2, `q35: error: accel-unavailable: /dev/kvm is not writable…` — no silent TCG downgrade. |
| 8 | `q35` app derivation build (`writeShellApplication`) | shellcheck clean; the derivation builds. |
| 9 | HARVEY-UNTOUCHED guard | `HARVEY-UNTOUCHED` printed. |
| 10 | `grep -cE 'sys/lib/(postscript\|troff)\|devpost.add' $HF/harvey-flip.diff` | **0**. |
| 11 | `test -s $HF/harvey-flip.diff` | non-empty (511797 bytes). |

The build half (`--build`) was **not** run: it uses sudo on Linux and the
owner's lima build directory on the Mac (plan prohibition, T-01-84). The
launcher implements it through the same scripts the `build` app runs; the
dependencies it asserts come from Harvey's existing setup/toolchain.

## Untouched

- The owner's `/Users/neilskinner/code/harvey` checkout: HEAD and the
  working-tree status hash are unchanged (`HARVEY-UNTOUCHED`).
- Harvey's 11 pre-modified troff/postscript font files and the
  `devpost.add/c1..c3` paths: not edited, not in the diff.
- Harvey's `harness`, `minimega` and `drawterm` packages, the
  setup/toolchain/build/iso apps, the devshell, the `drawterm` app, and the
  `build/aarch64` guest fixtures.
- The scratch clone's 11 `sys/**` `M` entries are APFS case-collision
  checkout artifacts (Git checks out both members of a case-colliding pair
  into one file). They are present in the clone before this change's first
  edit; the diff excludes `sys/**`.

## The exact commands the owner uses at stop C

```sh
HF=$HOME/r9-harvey-flip

# the whole diff, paged
less "$HF/harvey-flip.diff"

# verify the file the review is based on
shasum -a 256 "$HF/harvey-flip.diff"   # f7b0272d23bc6c131c2f63b37cde1330c0bebddd25b905c22040829232cd7f22

# per-file stat and the flake-only view
git -C "$HF/harvey" diff --stat HEAD -- . ':(exclude)sys/**'
git -C "$HF/harvey" diff HEAD -- flake.nix flake.lock

# the new launcher
sed -n '1,120p' "$HF/harvey/nix/q35.sh"

# the deletion set
git -C "$HF/harvey" diff --name-status HEAD -- . ':(exclude)sys/**' | grep '^D'
```

To preview the app list after the flip:

```sh
nix flake show --all-systems "path:$HF/harvey"
```

## Planned commit message (no attribution)

```
refactor(flake): move r9 composition to the r9 repo; add the q35 own-kernel launcher

- drop inputs.r9 and every package/app that consumes it (D-02, D-07, D-10)
- delete the gate scripts and expect drivers now owned by r9 (D-19)
- remove the r9 CI legs (r9-shell.yml; phase6.yml's tier-r9)
- add the q35 app: build and boot Harvey's own kernel to the root prompt (D-26)
```

## Note on the owner's checkout

The owner's `master` was **ahead 1** before this change: `25d97dc4d
docs(planning): freeze — …` was unpushed. The applied commit `c697666ba`
is a child of it; the owner pushed both on 2026-11-23, so `origin/master`
now equals `c697666ba315f137ca2fd11768f835d04bd9f018`. D-23's rule is
about r9 never locking an unpushed Harvey commit; the push was the
owner's action after the approval.
