---
last_mapped_commit: bdcf45a51dced761eec09538b5f0786ff81e0399
last_mapped_at: 2026-11-13
---
# Testing Patterns

**Analysis Date:** 2026-11-13

This repo has **no xUnit-style test framework for the C tree**. Testing is gate-based and layered: host-side Rust unit tests, expect-driven qemu boot gates, in-guest rc sweeps emitting a marker vocabulary, build-artifact markers, bake-time manifest asserts with pinned digests, coverage sweeps, and CI workflows. The sibling repo `../r9` carries the kernel-side unit tests (`cargo xtask test` battery).

The prime directive, stated repeatedly in the scripts themselves: **mk/tool exit status is never trusted; markers and artifact asserts are the only gates** (`build/x86_64/subset-build.sh:45-46`). A pass that cannot name its evidence is not a pass.

## Test Framework

**Layers (each with its own runner):**

| Layer | Tool | Config / entry |
|-------|------|----------------|
| Host Rust unit tests (harvey) | `cargo test` | `build/harness/Cargo.toml` (~70 `#[test]` fns) |
| Kernel/host port tests (r9) | `cargo xtask test` | `../r9/xtask/src/main.rs` (`TestStep`, line 3042) |
| Boot gates (both repos) | expect + qemu | `build/x86_64/*.expect`, `build/aarch64/*.expect` |
| In-guest sweeps | rc scripts | `build/x86_64/tier1-sweep.rc`, `services-sweep.rc` |
| Build gates | sh drivers | `build/x86_64/subset-build.sh`, `build/x86_64/mkimage.sh` |
| Coverage proof | sh driver | `build/x86_64/coverage-sweep.sh` → `dir-coverage.txt` |
| CI | GitHub Actions | `.github/workflows/phase6.yml`, `r9-shell.yml`, `harvey.yml` |

**Assertion styles:**
- Rust: standard `#[test]` + `assert_*!` macros
- expect gates: `stage` proc regex matches + whole-file post-pass string/regex scans
- sh gates: `[ ... ] || err "..."` asserts, digest comparisons (`sha256sum`/`md5sum` vs pinned constants), magic-byte checks via `od`

**Run Commands:**
```bash
# Harvey host-harness unit tests (from build/harness/)
cargo test

# r9 regression battery (from ../r9) — port lib tests + arch bin tests
nix develop -c cargo xtask test

# One-command boot gates (from harvey repo root; nix flake apps)
nix run .#boot-r9-smoke-x64        # x86_64 smoke: boot to % prompt + command sweep
nix run .#boot-r9-tier1-x64        # x86_64 tier-1 sweep gate (36+ markers)
nix run .#boot-r9-net-smoke-x64    # net gates: static + DHCP + loopback echo
nix run .#boot-r9-services-x64     # tier-2 services probes (SVC-* markers)
nix run .#boot-r9-churn-x64        # exec churn / heap-marker bound
nix run .#boot-r9-auth-x64        # p9sk1 auth domain: pairing+keyfs+listeners+cs+factotum+auth/debug probe+speaksfor+secstore substrate (client round-trip named-skipped: r9 devip two-write TCP blocker)
nix run .#boot-r9-tier1            # aarch64 tier-1 venue
nix run .#boot-r9-smoke            # aarch64 smoke venue

# Build-only gates
sh build/x86_64/subset-build.sh    # tier build, emits T*-ART markers
sh build/x86_64/mkimage.sh         # bake gates (manifest + digest + magic asserts)
sh build/x86_64/coverage-sweep.sh  # full-tree coverage proof

# Escape hatch (iteration only, NEVER the gate)
R9_SKIP_BUILD=1 nix run .#boot-r9-tier1-x64
```
Env knobs: `R9_ACCEL=kvm|tcg` (no silent downgrade), `R9_CPU`, `R9_TIER1_LOG`, `KERNDATE`, `R9_SKIP_BUILD`.

## Test File Organization

**Location:**
- Host Rust tests: co-located `#[cfg(test)] mod` blocks at the bottom of each module (`build/harness/src/trace.rs` — 18 tests after the pure parsing core), plus separate `*_tests.rs` files wired from `lib.rs` in `../r9/port/src/` (`namec_tests.rs`, `devip_tests.rs`, `devether_tests.rs`, `allocator_tests.rs`)
- expect gates: `build/<venue>/<name>.expect` beside their sh drivers in `nix/`
- In-guest sweeps: `build/x86_64/tier1-sweep.rc`, `services-sweep.rc` (committed; baked into images at `/lib/`)
- Fixtures: `build/x86_64/root-tree/` (committed image skeleton incl. `/lib` net fixtures), `build/x86_64/hello.c` + committed `nm-input.6` (baked object), `build/x86_64/nm-input.6`
- Evidence: serial captures retained under `.planning/phases/<phase>/evidence/*.log`; ledgers in `build/x86_64/tier{0..3}.ledger`

**Naming:**
- expect: `<purpose>-r9[-x64].expect` (`smoke-r9-x64.expect`, `tier1-r9.expect`, `netsoak-r9.expect`, `stress-r9.expect`, `churn-r9.expect`, `services-r9.expect`)
- Rust: `<module>_tests.rs` or inline `mod tests`
- Logs: `<gate>.log`, run-retained copies `*-run1.log`, `*-run2.log` (gates are run twice for flake detection — "green ×2")

**Structure:**
```
build/x86_64/
├── *.expect              # qemu boot gate drivers (host side)
├── *-sweep.rc            # in-guest marker sweeps (baked into image /lib)
├── tier-termrc, services-termrc   # termrc overrides running sweeps from boot flow
├── mkimage.sh            # bake gates (6 image contracts, each with asserts)
├── subset-build.sh       # tier build driver (T*-ART markers)
├── stage-build.sh        # mkimage input producer (smoke-out/, out/)
├── coverage-sweep.sh     # full-tree build coverage proof
├── tiers/tier{0..4}*.txt # append-only allowlist data
├── root-tree/            # committed image skeleton + rc fixtures
└── tier{0..3}.ledger     # per-artifact verdict records
nix/<gate>-r9[-x64].sh    # flake-app wrappers: build → bake → compose → gate
```

## Test Structure

**expect gate pattern (canonical: `build/x86_64/tier1-r9.expect`):**
```tcl
# 1. env contract, loud, before any spawn
foreach v {R9_ISO R9_TIER1_LOG R9_EXPECT_OK R9_EXPECT_SKIP} {
    if {![info exists env($v)]} { puts stderr "...: \$$v unset"; exit 2 }
}
# 2. fresh capture (a stale capture is dead evidence)
exec rm -f $env(R9_TIER1_LOG)
# 3. spawn qemu with per-run unique -name tag; log_file the serial wire
spawn $QEMU -M $R9_MACHINE -cpu $R9_CPU -accel $R9_ACCEL -cdrom $env(R9_ISO) \
    -nographic -name $run_tag
log_file $env(R9_TIER1_LOG)
# 4. teardown: kill qemu's process group on EVERY exit path
proc cleanup {} { catch { exec kill -KILL -- -$spawn_pid } ... }
# 5. fast-fail on panic anywhere in the boot
expect_before { -re {panic|Unhandled interrupt} { ...; exit 1 } }
# 6. staged ordered assertions, each naming its failure
stage banner {r9 from the Internet}
stage paqfs  {paqfs\.\.\.}
stage sweep-done {T1-DONE}
stage prompt {% }
# 7. whole-file post-pass: every expected T1-OK present, SKIP set EQUALITY
#    (both directions), zero T1-FAIL, zero panic
```

**Patterns:**
- Setup: driver wrapper (`nix/*.sh`) does build → bake → compose ISO → rm stale log → export env → run expect
- Teardown: expect `cleanup` proc + caller EXIT trap `pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}"` (bracket-fixed so pkill can't self-match)
- Assertion: presence-only markers over serial; whole-file scan for set equality

**In-guest sweep pattern (`build/x86_64/tier1-sweep.rc:80-85`):**
```rc
# check NAME CMD... — run the fixed input; zero exit status → T1-OK.
# A failing entry never aborts the sweep.
fn check {
	n=$1
	shift
	if($*) echo T1-OK $n
	if not echo T1-FAIL $n
}
```
Every check runs a FIXED input with a pinned expectation (exact digests, exact first lines, deterministic word orders), e.g.:
```rc
md5out=`{$B/hmd5sum $D/sentinel.txt}
if(~ $"md5out '6c3d57e94871f9606c5b63c315e76135 '^$D^/sentinel.txt) echo T1-OK md5sum
```

**Rust unit test pattern (`build/harness/src/trace.rs`):**
```rust
// Pure parsing core is separated from the live driver SO IT CAN BE
// UNIT-TESTED; the live driver feeds it captured bytes.
#[test]
fn sysh_parses_defines() {
    let m = load_sysh("#define BIND 2\n");
    assert_eq!(m.get("bind"), Some(&2));
}
```
`../r9/port/src/*_tests.rs` fixtures build synthetic paqfs images byte-by-byte (magic constants `0x529AB12B` etc.) and stub dev tables — the "host venue" pattern; group-dependent tests serialize on a SEAM lock because hooks are process-global.

## Mocking

**Framework:** None. There is no mocking library in any language layer.

**What exists instead:**
- Real execution: gates boot the real kernel under qemu and run the real freshly-built binaries — the marker is proof of execution, never of a supervisor's opinion
- Synthetic fixtures in Rust tests: hand-built byte images (synthetic paqfs trees in `../r9/port/src/namec_tests.rs`) — the only "mocks", and they mock data, not code
- Substitution seams: `build/x86_64/root-tree/lib/netaddr.rc` is the ONE place a venue address may live (D-11 seam); bakes assert via grep that no venue literal leaked into any other fixture (`build/x86_64/mkimage.sh:541-544`)
- Marker binaries: images stage a deliberately-tiny `init` (the `smokesleep` marker) whose rodata strings are grep-asserted so the wrong init cannot ride (`mkimage.sh:216-219`)

**What to Mock:** nothing at the gate level; synthetic data images in host unit tests only.

**What NOT to Mock:** the kernel, the boot chain, or the binaries under test. A daemon "earns its SVC-OK marker ONLY through a real client interaction through it ... a daemon that merely started is NEVER an OK" (`build/x86_64/services-sweep.rc:7-11`).

## Fixtures and Factories

**Test Data:**
- **Sentinel file** — byte-exact `'alpha bravo charlie\n__SUBSET_GREP_LINE__\n'` (41 bytes), created at bake time by `printf`, digest-pinned in TWO places (bake assert + in-guest assert): md5 `6c3d57e94871f9606c5b63c315e76135`, sha1 `893404a2c36b54f5c7874070d1acefe38f7ce652` (`build/x86_64/mkimage.sh:437-441`)
- **Baked object fixture** — `build/x86_64/nm-input.6`, produced at authoring time by host `6c` over the committed `build/x86_64/hello.c`; reproducible byte-identically only from the same absolute source path (the path rides the object's file symbol) — the reproduction command is recorded in comments (`tier1-sweep.rc:56-64`)
- **Image skeleton** — `build/x86_64/root-tree/` committed tree (termrc, namespace, profile, net fixtures `lib/netip-*.rc`); staged via `tar` into each bake
- **Float canary** — `T1-OK float-d01` is MANDATORY on every venue: `awk 'BEGIN{printf "%.3f\n", 1.0/3.0}'` must print `0.333`, execution-proving user SSE/FP state end-to-end (`tier1-sweep.rc:147-154`)
- **ndb token template** — `root-tree/lib/ndb/local.template` carries `@TOKEN@`s only; substituted at bake time from the netaddr seam and asserted BOTH ways (every seam value present, zero leftover `@`)

**Location:** fixtures live beside their consumers under `build/<venue>/` (`root-tree/lib/`, `hello.c`, `nm-input.6`); in-guest data bakes to `/lib/tier1/` in the image.

**Digest/magic pinning conventions:**
- paqfs header magic `529ab12b` (first 4 bytes), trailer magic `6b46e688` — checked via `od` on every baked image
- Plan 9 a.out magic `00008a97` (amd64) checked on host-side-linked binaries (`mkimage.sh:965-973`)
- Cross-venue binary identity: `.build/9k10t1-tier-manifest.sha256` (38-member sha256 manifest of `/amd64/bin`) is digest-verified by the tier bake — the identical-binary-set ABI witness for both kernels (`mkimage.sh:369-382`)

## Coverage

**Requirements:** No line-coverage tool. Coverage means TREE coverage: every dir under `sys/src/cmd`, `sys/src/games`, every `lib*`, every cmd-root single, and every container member is VISITED, BUILT under a per-dir budget, and verdicted.

**View Coverage:**
```bash
sh build/x86_64/coverage-sweep.sh   # regenerates build/x86_64/dir-coverage.txt
```
- Output grammar: `<key> visited=N build=ok|fail|hang` — DATA ROWS ONLY
- `COVERAGE_SWEEP=1` disengages the six mk allowlist filter arms so the sweep certifies the WHOLE tree, "never the allowlist to itself"
- A wedged dir is a classified `timeout` row (`build=hang`), never a silent pass; VISIT-with-no-EXIT classifies hang
- Every exclusion is a named `# exclude: <path> <reason>` line in `tiers/tier4-tail.txt` — a stale exclusion fails the run
- The green gate requires the full universe with every row ok — a partial file never passes as proof

## Test Types

**Unit Tests:**
- `build/harness` Rust: pure parsing cores (syscall-name resolution, regex, console parsing, mkfs9, trace ENTRY fragments) — `cargo test`
- `../r9/port/src/*_tests.rs`: kernel logic host-side (namec union consult, devip, devether, allocator) with synthetic paqfs images and stub devices; group-global hooks serialize on a SEAM lock

**Integration / Boot Gates (expect + qemu):**
- Smoke: banner → paqfs → version → `% ` prompt → typed command sweep (`build/x86_64/smoke-r9.expect`)
- Tier-1: the sweep runs from the boot flow (termrc override — no typed interaction); float canary first; marker parity + exact SKIP set; T1-DONE; the interactive prompt
- Net: static + DHCP + loopback tcp/udp echo through real binaries (`build/x86_64/smoke-r9-net.expect`, drivers `nix/smoke-r9-net-x64.sh`, soak `netsoak-r9.expect`, `stress-r9.expect`)
- Services: tier-2 daemon functional probes (mntgen/cs/dns/exportfs/listen/factotum/keyfs/authsrv) — real client interactions, bounded spin waits, never-wedge discipline (`build/x86_64/services-sweep.rc`)
- Churn: bounded fork+exec churn window with per-exit heap-marker delta (`build/x86_64/churn-r9.expect`, `churncats.c`)
- Donor venue: Harvey's own 9k kernel (k10t1) runs the SAME sweep — both venues assert identical marker sets (`build/mm/boot-9k10t1-sweep.sh`)

**Build/Bake Gates (host-side asserts):**
- `subset-build.sh`: per-entry fresh build (`mk <target>.clean` / object rm BEFORE every build — a stale object can never earn a marker), fresh archives (`rm -f` target `.a`; `ar vu` never prunes), T0/T1/T2/T3-ART sha256 markers, magic checks
- `mkimage.sh`: pre-bake manifests (every mandated path present, PRINTED as evidence), init-identity grep guards (marker vs real init, inverted per image), `/boot/factotum` ABSENCE asserts (auth-stub-by-absence), rcmain byte-verbatim `cmp` guards, KERNDATE mtime pinning for byte-reproducible bakes, mkpaqfs per-file 4194304-byte cap assert (silent-drop hazard), header/trailer magic checks
- Negative controls: r9's `ureg-check` gate runs the real check then a deliberately-mutated header copy (cs → cseg) and asserts the mutation IS rejected — "the mutated ... copy was rejected with a named-field diff" (`../r9/xtask/src/uregcheck.rs:178-195`); CI's gethunk repro gates rebuild pristine compilers from git and verify the hang signature on them, green only on the fixed pair (`.github/workflows/phase6.yml`, `build/aarch64/gethunk-repro-*.sh`)

**CI (`.github/workflows/`):**
- `phase6.yml`: wave0-host (repro gates, coverage regenerated in place + `git diff --exit-code` drift signal, generator .7-magic scan, grep-hygiene, venue pin), tier-9k + tier-r9 (both kernel venues, 90m), tier-2 compile gate (T2-ART marker parity), x86_64-poolpanic (verdict PRODUCTION asserted, never the outcome)
- `r9-shell.yml`: clean checkout → nix install (action pinned to full 40-char SHA) → `nix run .#boot-r9-smoke` (aarch64, TCG) + the x64 legs (`net-smoke-x64`, `auth-smoke-x64` — both `R9_ACCEL=tcg`; the auth leg carries the p9sk1 probe legs + the secstore SKIP); permissions `contents: read`, no secrets
- `harvey.yml`: legacy 9legacy-ISO pipeline — untouched sibling

## Common Patterns

**Async / never-wedge testing (in-guest):**
```rc
# bounded spin, never an unguarded wait (TCG-timing tolerant)
$B/hfactotum -s t1facto &
factctl=0
for(i in 1 2 3 4 5 6 7 8 9 10){
	if($B/htest -e /mnt/factotum/ctl) factctl=1
	if(~ $factctl 0) $B/hsleep 1
}
if(~ $factctl 1){ ...probe... }
if not echo T1-FAIL factotum
```

**Error testing (negative assertions):**
- SKIP-set EQUALITY: the gate extracts the sweep's own declared `echo T1-SKIP` lines verbatim and asserts the observed set equals it in BOTH directions — "a missing declared SKIP is as red as an extra one" (`tier1-r9.expect:182-193`). Venue honesty: "named T1-SKIP lines, never an unearned T1-OK"
- Zero-T1-FAIL and zero-panic whole-file scans after the staged pass
- Usage-level execution proof: a bad flag (`-Z`) must print usage text naming the program — proof the binary ran, on network-less venues (`tier1-sweep.rc:216-220`)

**Computed expectations (never a second hardcoded list):**
```bash
# nix/tier1-r9-x64.sh:156-191 — the OK set is COMPUTED from the committed
# tier data minus SKIP members plus the float canary; the SKIP set is
# extracted VERBATIM from the committed sweep's own SKIP lines
T1_SKIP=$({ grep -oE '(^|[[:space:]])echo T1-SKIP [A-Za-z0-9.-]+' ... || true; } | awk '{print $NF}' | sort -u)
```
Whenever a check flips between SKIP and OK, the check AND the gate's computed expectations change in the SAME commit (Pitfall 4 discipline).

**Evidence discipline:**
- Gates run ×2 on byte-identical composed ISOs; passing captures retained in `.planning/phases/<phase>/evidence/`
- Ledgers record `tier=N dir=... bin=... sha256=... exec=T1-OK:r9x64|SKIP:<reason> evidence=<log> commit=<sha> date=<iso>` (`build/x86_64/tier1.ledger`) — compile-only vs execution distinction lives in the `exec` field
- KERNDATE provenance: git `%ct` of the checkout (overridable) pins mtimes so two bakes of one tree are byte-identical

---

*Testing analysis: 2026-11-13*
