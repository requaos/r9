---
phase: quick-261117-bz5
plan: 01
subsystem: build
tags: [objtype, arm64, rename, toolchain, donor-collision, r9-lockstep]
status: complete
requires: []
provides:
  - "objtype/cputype/arch dir arm64 across Harvey (arch dir, per-arch sources, toolchain strings, build wiring, harness, nix, CI)"
  - "r9 lockstep: cputype seed arm64, ureg-check reads arm64/include/ureg.h"
affects: [build/aarch64 harness, build/x86_64, build/mm, nix, CI, ../r9]
tech-stack:
  added: []
  patterns:
    - "donor and Harvey /arm64 trees are told apart by namespace, never by name"
key-files:
  created: []
  modified:
    - arm64/ (git mv from aarch64/)
    - sys/src/libc/arm64/ (git mv), sys/src/libmp/arm64/mkfile, sys/src/libsec/arm64/mkfile
    - sys/src/libthread/arm64.c, sys/src/libdynld/dynld-arm64.c, sys/src/9k/boot/mkfile.arm64
    - sys/src/cmd/7a/lex.c, sys/src/cmd/7c/txt.c, sys/src/cmd/7l/obj.c, sys/src/cmd/pcc.c, sys/src/libmach/7.c
    - sys/src/mkfile, sys/src/mkfile.proto, sys/src/libc/9syscall/mkfile, sys/src/BUILD_SETUP_NAMESPACE
    - sys/src/cmd/{mkfile,mkmany,ip/mkfile,auth/mkfile,auth/factotum/mkfile,aux/mkfile}
    - build/scripts/{build,linux-build}, build/mkdirs, dist/replica/plan9.proto, .gitignore
    - build/aarch64/{nsrun,donor-ns-build,run-gate,subset-exec,subset-build,stage-build,mkimage,coverage-sweep,pre-gate-build,ledger-r9-append,generator-scan}.sh
    - build/aarch64/{trace-boot.rc,tier1-sweep.rc,README.md,dir-coverage.txt,generator-audit.txt,tiers/*.txt,drive-r9.expect,smoke-r9.expect}
    - build/harness/src/trace.rs
    - build/x86_64/{coverage-sweep.sh,subset-build.sh,dir-coverage.txt,tier1-sweep.rc,tiers/*.txt}, build/mm/{build-9k10t1,build-9kdraw}.sh
    - nix/{vm-build,tier1-r9}.sh, .github/workflows/phase6.yml
    - AGENTS.md, .planning/PROJECT.md
    - ../r9: xtask/src/{uregcheck,main}.rs, aarch64/src/{main.rs,syscall.rs,trap.rs,trap.S}, port/tests/devenv_test.rs, README.md, HowItWorks.md
decisions:
  - "Objtype naming reversed to arm64 (2026-11-17); host-side names keep aarch64"
  - "Donor /aarch64 compatibility alias deleted; separation is by namespace"
  - "Served donor root carries one mixed /arm64 (donor bin+init, Harvey include+lib/libc.a)"
  - "Trace chain: /boot/ourinit bound over the donor's /arm64/init instead of reordering trace-namespace (deviation from the chosen option B — see Deviations)"
metrics:
  started: 2026-11-17T13:44:38Z
  completed: 2026-11-17T14:16:11Z
  duration: "32 min execution across two passes (checkpoint between)"
actuals:
  tokens: 60000
  tasks: 3
  commits: 3
plan_head_before: 29ee10d78e88a570d1f3c3aaa0c70d95bd14d76d
plan_head_after: cd0fa04d285a3d5e3194b23d09930eff34f28943
---

# Quick 261117-bz5: Rename Harvey objtype aarch64 -> arm64 — Summary

Harvey's 64-bit ARM objtype, cputype and arch dir are `arm64` again everywhere Plan 9 naming governs, with r9 in lockstep; host-side names (`build/aarch64/`, qemu binary, triples, nix systems, r9's crate) are unchanged.

## Commits

| Task | Repo | Commit | Content |
|------|------|--------|---------|
| 1 | Harvey | 27cc1d25e | arch dir + per-arch sources (`git mv`), thestring, build wiring, namespace binds (48 files) |
| 2 | Harvey | a15e787e9 | harness / nix / CI re-key, donor alias removal, the two collision sites (42 files) |
| 2 (STEP C) | r9 | 8e4e730 | cputype seed, `arm64/include/ureg.h` join, tests, docs (9 files) — NOT pushed |
| 3 | Harvey | cd0fa04d2 | AGENTS.md + PROJECT.md naming reversal record |

No commit carries an attribution trailer. The 11 pre-existing modified files are still unstaged and untouched.

## Donor-collision decision table

| Site | Namespace | `/arm64` is | `/aarch64` was | Harvey arch dir bound or read there? | Resolution |
|------|-----------|-------------|----------------|--------------------------------------|------------|
| `donor-ns-build.sh --ns` | its own `sudo unshare -m` | `$DONOR/arm64` | alias of `$DONOR/arm64` | no | alias deleted; header + README prose rewritten (separation by namespace) |
| `run-gate.sh --ns` | its own `sudo unshare -m` | `$DONOR/arm64` | alias | no | alias deleted |
| `subset-exec.sh --ns` | its own `sudo unshare -m` | `$DONOR/arm64` | alias | no (Harvey binaries are copied by host path in the wrap phase) | alias deleted |
| `trace.rs` embedded scripts ×3 | each its own `sudo unshare -m` | `$DONOR/arm64` | alias | no | alias deleted in all three |
| `subset-exec.sh` served guest root (`$ROOTFS`) + `trace.rs` mirror | donor-kernel guest, srv9 root | donor `arm64/{bin,init}` | Harvey `include` + `lib/libc.a` | yes — both trees in one root | merged into one `/arm64`; no overlap (donor srvroot has no `arm64/include` or `arm64/lib`, re-confirmed at edit time against donor-ns-build.sh and the on-disk srvroot); documented in the script header |
| `trace-boot.rc` chain | donor-kernel guest, boot(8)'s namespace | donor `/arm64/init` ahead of Harvey's image in boot's `/` union | the name only Harvey's image had | yes | `cputype=arm64`; `bind /boot/ourinit /arm64/init` before launch — see Deviations |

Gate result: six `mount --bind "$DONOR/arm64" /arm64` lines unchanged, zero binds onto `/aarch64`.

## Residue audit — `git grep aarch64 -- ':!.planning'`

119 files, 743 lines, 785 occurrences. Every occurrence falls in a keep class:

| Class | Occurrences | Files | Justification |
|-------|-------------|-------|---------------|
| Host harness dir `build/aarch64` | 430 | 64 | directory name kept by the plan (pairs with `build/x86_64`) |
| Host mapping / hostbin / venue / loader names | 54 | 33 | `uname` idioms, `hostbin-*-aarch64`, `linux-aarch64` / `darwin-aarch64`, `ld-linux-aarch64`, `aarch64-linux-gnu`, Dockerfile and 9-cc.patch host cases |
| `qemu-system-aarch64` | 38 | 13 | external binary name |
| Rust triple / xtask flag | 1 | 1 | `--arch aarch64` |
| nix system strings, lima arch | 7 | 4 | external naming |
| r9 crate paths | 12 | 8 | `../r9/aarch64/...`, `$R9_KERNEL_PKG/aarch64/lib/...`, `aarch64/src/...` |
| Serial marker + pinned binaries | 19 | 10 | `hello aarch64 harvey`, `hello.7.out`, `nm-input.7`, the srv9 7-byte assertion |
| Append-only ledgers / evidence | 67 | 3 | `markers/tier{0,1}.ledger`, `gate-evidence.log` — recorded history |
| Prose | 157 | 61 | ISA, host, r9 kernel and historical venue/milestone names ("the aarch64 gate", "aarch64 userland", "r9 aarch64 kernel ELF"), plus 9 notes that record this rename |

`git ls-files | grep aarch64` outside `build/aarch64/` and `.planning/` is empty.

Judgment calls in the prose class:
- "the aarch64 gate / driver / venue / bake / sweep" in `build/x86_64/*` scripts and expect files is kept as the historical venue name.
- Tier-list comments (`tiers/*.txt`) and mkfile filter-arm comments are renamed, as the plan directs.
- `build/aarch64/tiers/tier0-libs.txt:60` keeps "dynld-aarch64 rename verdict (2d2fa5725)" — a record of that commit; the file is now `dynld-arm64.c`.

Task 2 gate 1 reports 5 lines, all keep-list over-matches not edited: `nix/{boot-r9,smoke-r9,smoke-r9-net,tier1-r9}.sh` `$R9_KERNEL_PKG/aarch64/lib/...` and `build/aarch64/run-r9.sh:32` `../r9/aarch64/lib/config_virtexec.toml` (r9-rooted paths), plus the tier0-libs line above.

Ledger consumers: only `ledger-r9-append.sh` writes the ledgers and nothing resolves an existing row's `artifact=` against the live tree, so no consumer change was needed. New rows will carry `arm64/...` prefixes; old rows keep `aarch64/...`.

Historical items left unedited: both ledgers, `gate-evidence.log`, r9 `port/tests/05-05-red-evidence.json`. No tracked fixture encodes the trace-namespace bind order, and `trace-namespace` itself is unchanged.

## Verification

| Check | Result |
|-------|--------|
| Task 1 / 2 / 3 gates | pass (gate 1 of Task 2: 5 justified over-matches above) |
| `objtype=arm64` libc smoke | **PASS** — lima guest `harvey` (linux-aarch64), via `nix develop -c bash build/aarch64/vm-run.sh 'cd /sys/src/libc && objtype=arm64 mk nuke && objtype=arm64 mk install && cd $HARVEY/build/aarch64 && 7c -FVw -o /tmp/bz5-hello.7 hello.c && 7l -o /tmp/bz5-hello.7.out /tmp/bz5-hello.7'`. Guest 7a/7c/7l rebuilt first (`nix run .#toolchain` in the guest). `arm64/lib/libc.a` written 09:08:51 guest time, during the run; hello linked with R_MAGIC `00008c47`. Guest copy only — nothing pulled back. |
| darwin host toolchain | rebuilt (`nix run .#toolchain`); `.build/hostbin-darwin-aarch64/bin/7{a,c,l}` are fresh |
| `build/harness` `cargo build` + `cargo test` | 70 passed, 0 failed (re-run on the committed tree) |
| r9 `cargo xtask ureg-check` (`R9_HARVEY_DIR` = this checkout) | pass — "35 fields match arm64/include/ureg.h", amd64 26 fields, negative control fired |
| r9 `cargo test -p port` | **NOT run** — r9 pins `nightly-2026-01-30`; only cargo 1.99 stable is available here and `port` does not compile on it (E0554/E0046), and r9's own `nix develop` rejects this platform |
| Donor-venue gates (`subset-exec.sh`, trace chain) | **NOT run** — hand-driven donor venue; the merged `/arm64` staging and the `ourinit` bind are unexecuted |
| `graphify update .` | run; `graphify-out/` is gitignored |

## Deviations from Plan

**1. [Rule 1 - Bug] Trace chain: bind over the donor's init instead of reordering `trace-namespace` (differs from the chosen option B)**
- **Found during:** Task 2 STEP A, implementing option B.
- **Issue:** The collision is in boot(8)'s own `nsinit`, not in `trace-namespace`. boot does `bind / /` (which carries the donor shell's whole `/` union along) and then binds the image `MAFTER`; `execinit` runs in that namespace, before init's `newns` ever reads `trace-namespace`. Reordering `trace-namespace` therefore cannot change which init is exec'd. It would also put the image's `/boot` ahead of the donor devroot's `/boot`, so `bind /boot/ourtermrc ...` in the same file would stop resolving. My checkpoint description of this site was wrong on that point.
- **Fix:** `trace-boot.rc` seeds `cputype=arm64` and binds `/boot/ourinit` (the same binary the image stages at `/arm64/init`) over the donor's `/arm64/init` before launching boot. The exec path string, the cputype branch and boot's syscall stream are unchanged; `trace-namespace` is untouched, so the traced bind sequence still matches r9's `lib/namespace`. Header and inline comments explain it; the generated trace header in `trace.rs` mentions it.
- **Status:** reasoned from `sys/src/9k/boot/boot.c` and Plan 9 union-bind semantics, NOT executed. Needs owner confirmation, and a donor-venue run before the trace is relied on.
- **Commit:** a15e787e9

**2. [Rule 1 - Bug] Paths my keep filter masked**
- `harvey-build/aarch64/*.7.out` (guest copy of Harvey's arch dir, three sites in `trace.rs`) and every `&& objtype=aarch64` matched the "host dir" and "uname idiom" keep patterns; both were found by the Task 2 gate and fixed before the commit.

**3. Scope notes**
- `sys/src/cmd/{mkfile,ip/mkfile,auth/mkfile,auth/factotum/mkfile}` `~ $objtype` filter arms were functional renames, landed in Task 2 as the plan lists them.
- `build/mm/build-9k10t1.sh`, `build/mm/build-9kdraw.sh`, `build/x86_64/{coverage-sweep,subset-build}.sh` bind every arch dir by name in a loop; between the Task 1 and Task 2 commits those loops pointed at a missing dir.
- ROADMAP.md: no edit — none of its five hits name the objtype for upcoming work.
- The darwin toolchain rebuild left seven new untracked host binaries (`sys/src/cmd/{6a/6a,6c/6c,6l/6l,8a/8a,8c/8c,8l/8l,cpp/cpp}`); I removed those seven, restoring the pre-run `git status`.
- `gsd-tools windows append` refused both entries (pre-existing ledger count mismatch: frontmatter 52/3/13/68 vs entries 49/5/14/68), so the two unrun verifications above are NOT in `.planning/WINDOWS.md`.

## Follow-ups

- r9 commit 8e4e730 is local only. Harvey's flake pins r9, so `nix run .#boot-r9` and the other aarch64 r9 apps still get a kernel that seeds `cputype=aarch64` while the image now stages `arm64/`; they will not reach init until r9 is pushed and the pin advanced.
- `.build/9front-dist/srvroot/aarch64/` (old staging) is left on disk; it is ignored build state and harmless.
- Run the donor-venue gates and r9 `cargo test -p port` on a host that has them.

## Self-Check: PASSED

- Harvey commits 27cc1d25e, a15e787e9, cd0fa04d2 are ancestors of HEAD; r9 8e4e730 is r9 HEAD.
- `arm64/mkfile`, `arm64/include/ureg.h`, `arm64/ABI.md`, `sys/src/libc/arm64/mkfile`, `sys/src/9k/boot/mkfile.arm64`, `sys/src/libdynld/dynld-arm64.c` exist; `aarch64/` is absent.
- `git rev-list --count 29ee10d78..HEAD` = 3.
