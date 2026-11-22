---
phase: 16-host-net-modes-networking-exit-gate
reviewed: 2026-11-11T12:00:00Z
depth: standard
files_reviewed: 27
files_reviewed_list:
  - build/x86_64/mkimage.sh
  - build/x86_64/netsoak-r9.expect
  - build/x86_64/README.md
  - build/x86_64/root-tree/lib/netaddr.rc
  - build/x86_64/root-tree/lib/netip-tap.rc
  - build/x86_64/smoke-r9-net.expect
  - flake.lock
  - flake.nix
  - .github/workflows/r9-shell.yml
  - nix/boot-r9-x64.sh
  - nix/host-echo.c
  - nix/net-doctor.sh
  - nix/smoke-r9-net-x64.sh
  - nix/soak-r9-net-x64.sh
  - ../r9/flake.nix
  - ../r9/x86_64/lib/config_q35exec.toml
  - ../r9/x86_64/lib/config_q35lowmcfg.toml
  - ../r9/x86_64/lib/config_q35net.toml
  - ../r9/x86_64/lib/config_q35shell.toml
  - ../r9/x86_64/lib/config_q35.toml
  - ../r9/xtask/Cargo.toml
  - ../r9/xtask/src/bin/r9-netdev-args.rs
  - ../r9/xtask/src/config.rs
  - ../r9/xtask/src/lib.rs
  - ../r9/xtask/src/main.rs
  - ../r9/xtask/src/pcap.rs
  - ../r9/xtask/src/uregcheck.rs
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
  necessity: 0
status: issues_found
---

# Phase 16: Code Review Report

**Reviewed:** 2026-11-11T12:00:00Z
**Depth:** standard
**Files Reviewed:** 27
**Status:** issues_found

## Summary

Dual-repo review of the Host Net Modes + Networking Exit Gate phase: the r9
netdev TOML mode-tables and the ONE shared argv emitter exported as
`r9-netdev-args` (16-01), the Harvey-side `netaddr.rc` tap_ seam and
`netip-tap.rc` guest fixture (16-02), the `net-doctor` host checker (16-03),
the `--net` surface on `boot-r9-x64.sh` (16-04), the soak gate + host-echo
peer (16-05), the emitter re-key of the slirp net smoke + CI job (16-06),
and the 16-08 stage-budget fix.

Cross-checked and found CORRECT (no finding): the `flake.lock` r9 pin
`d1c4849dfa1d769240630c29b9b03b88d5819558` matches r9's pushed HEAD
(d1c4849, "the 16-01 pushed SHA" per the README) and the lock carries the
narHash; the `[qemu.netdev]` sections of all five q35 TOMLs are
byte-identical (verified by digest of the extracted sections); the
emitter's slirp/tap/vmnet emission matches the qemu argv shapes the README
documents and the tests pin; the soak driver's stage order matches
`netip-tap.rc`'s actual emission order (static ok → arp view ok → L2-TRUTH
label → burst-wait → stats → soak ok → NETIP-DONE); the marker-count
post-pass (1 + burst_count = 9 `tcpblast: done`) is consistent with the
fixture's baseline-plus-8-way burst and the typed launch line carries none
of the staged markers; the doctor's 12-check list, FAIL-only exit gating,
and WARN-class masquerade degrade match the README contract; the bake-time
seam-literal grep in `mkimage.sh` 10b covers both venue prefixes and both
`netip-tap.rc` and `netaddr.rc` are clean of stray literals; `host-echo`'s
poll-loop compaction (downward iteration + last-slot move) is correct and
its array bounds hold at MAXCONN; the emitter's `--config` default embed
(`include_str!` of `config_q35shell.toml`) is the same config
`kernel-x64` builds with. No Critical findings. Three Warnings (one
supply-chain, two robustness gaps in the new gate machinery) and four Info
items follow. The necessity reference
(`$HOME/.zcode/gsd-core/references/necessity-review.md`) was read; the
candidates examined (the vmnet table on linux TOMLs — a recorded HNET-04
design decision with a named consumer; the soak's whole-file `netip: FAIL`
backstop — deliberate defense-in-depth, excluded by the rubric; the
emitter's `--config` override — an operator/test seam) are all in the
rubric's negative space, so `necessity: 0`.

## Warnings

### WR-01: Unpinned `actions/checkout@v4` in the boot-smoke job (supply-chain rule the file itself states)

**File:** `.github/workflows/r9-shell.yml:35`
**Issue:** The `boot-smoke` job pins checkout to the floating `@v4` tag while
the `net-smoke` (line 61) and `net-smoke-x64` (line 87) jobs pin the
API-resolved full SHA `11d5960a326750d5838078e36cf38b85af677262`. The file's
own header (lines 17-21) declares the supply-chain rule ("pinned to a full
40-char commit SHA, never a floating tag"), and the net-smoke job's comment
explicitly notes the `@v4` in boot-smoke "stays as-is" — a documented
deviation, but still a mutable tag on a job that executes nix builds of
pinned inputs; a tag repoint would change the checked-out tree silently.
**Fix:** Pin the boot-smoke job the same way (same resolved SHA, or a fresh
API resolution):

```yaml
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4 — API-resolved (the 06-08 rule)
```

### WR-02: `r9-netdev-args` does not enforce its own no-spaces token contract for caller-supplied `--pcap`/`--config` values

**File:** `../r9/xtask/src/bin/r9-netdev-args.rs:37-57,78` (with
`../r9/xtask/src/config.rs:618-627`)
**Issue:** The emitter's committed contract — "Prints the netdev argv tokens
… one space-separated line on stdout (tokens contain no spaces)" — is what
every consumer (boot-r9-x64.sh:264, smoke-r9-net-x64.sh:156,
soak-r9-net-x64.sh:277, and both expect drivers' `split` loops) relies on
when word-splitting the output back into qemu argv tokens. The builder
interpolates the CALLER-SUPPLIED `--pcap` path (and `--config` path)
verbatim into a token (`filter-dump,…,file={path}`) with no whitespace
check: `--pcap "/tmp/my dir/x.pcap"` emits
`…file=/tmp/my dir/x.pcap`, which every consumer splits into
`…file=/tmp/my` plus a stray `dir/x.pcap` qemu argument — silently
corrupted argv, and the stray tokens become arbitrary additional qemu
options parsed from the path value. The committed callers pass space-free
mktemp paths, so nothing committed breaks today; this is a published CLI
whose stated invariant it cannot itself uphold.
**Fix:** Reject whitespace (and empty) values at the CLI boundary in
`main()` before building argv:

```rust
for (flag, v) in [("--pcap", &pcap), ("("--config", &config_path)] { /* pairs */ }
if let Some(p) = &pcap {
    if p.is_empty() || p.chars().any(char::is_whitespace) {
        usage_exit(&format!("--pcap value {p:?} contains whitespace/is empty — tokens are split on spaces by every consumer"));
    }
}
```

(The `--config` value is not interpolated into the output, so it is
optional to check; `--pcap` is the load-bearing one.)

### WR-03: `boot-r9-net-smoke-x64` app lacks `procps` for its `pkill` survivor sweep — the sweep can silently no-op

**File:** `flake.nix:234-239` (runtimeInputs) with
`nix/smoke-r9-net-x64.sh:211` (the `pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true` cleanup)
**Issue:** The net-smoke app's EXIT trap runs `pkill` (procps), but
`netSmokeR9X64Pkg`'s runtimeInputs are
`[plan9port bash qemu git expect iproute2] + linux extras` — no `procps`.
`2>/dev/null || true` masks `pkill: command not found`, so on a host whose
ambient PATH lacks procps the survivor sweep silently does nothing and a
wedged qemu leaks past the gate. The sibling `netSoakR9X64Pkg` (flake.nix
line 321) added `pkgs.procps` for exactly this mechanism ("procps (the
survivor sweep's pkill) always") — the same-phase inconsistency is the
defect. (The expect driver's own group-kill covers the normal exit paths;
this trap is the survivor backstop, and it is the one part that degrades
silently.)
**Fix:** Add `pkgs.procps` to `netSmokeR9X64Pkg`'s runtimeInputs in
`flake.nix:238`, matching the soak app:

```nix
          '' ([ pkgs.plan9port pkgs.bash pkgs.qemu pkgs.git pkgs.expect pkgs.iproute2 pkgs.procps ]
```

## Info

### IN-01: `host-echo` accepts port arguments above 65535 via strtoul truncation

**File:** `nix/host-echo.c:126-129`
**Issue:** `port = (unsigned short)strtoul(argv[2], &end, 10)` checks only
`*end != '\0'` and `port == 0`. A value like `65537` truncates to `1` and
silently binds port 1 (then fails bind without root — confusing perror),
and `70000` becomes `4464` (a wrong-but-bindable port). The usage message
promises "want 1-65535" but the code does not enforce the upper bound. The
committed gate passes `blastport=7777` from the seam, so unreachable in
gate runs.
**Fix:** Validate the full `strtoul` result before narrowing:

```c
	unsigned long p;
	p = strtoul(argv[2], &end, 10);
	if(*argv[2] == '\0' || *end != '\0' || p < 1 || p > 65535){
		fprintf(stderr, "host-echo: bad port '%s' (want 1-65535)\n", argv[2]);
		exit(1);
	}
	port = (unsigned short)p;
```

### IN-02: `r9-netdev-args` silently accepts duplicate `--mode`/`--pcap`/`--config` flags (last wins)

**File:** `../r9/xtask/src/bin/r9-netdev-args.rs:42-57`
**Issue:** `mode = Some(value_of(...))` overwrites without complaint on a
second occurrence, while the Harvey front-ends consuming this bin reject
duplicate `--net` outright (`boot-r9-x64.sh:151`, `soak-r9-net-x64.sh:140`
— "one net mode per boot"). A copy-paste `--mode tap --pcap x --mode slirp`
runs slirp with no hint. Fail-loud elsewhere on this surface; inconsistent
here.
**Fix:** Reject a second occurrence the way `value_of` rejects a missing
value — e.g. track `seen` flags and `usage_exit("duplicate --mode …")`, or
error when the Option is already Some.

### IN-03: The five-TOML consistency test checks presence, not value equality — the documented "identical mode tables" invariant is not machine-checked

**File:** `../r9/xtask/src/lib.rs:136-157`
(`netdev_all_five_q35_tomls_carry_the_mode_tables`), with the invariant
stated at `../r9/x86_64/lib/config_q35lowmcfg.toml:62` ("Identical netdev
mode tables to the base config") and `../r9/xtask/src/config.rs` table docs
**Issue:** The test asserts each TOML parses, `default == "slirp"`, and
`slirp`/`tap`/`vmnet` are `Some` — it does not compare the tables' VALUES
across configs. A future edit that changes, say, only `config_q35net`'s
hostfwd pair or helper path would pass every test while the in-process
q35net gate (which boots its own table) and the exported emitter (which
embeds q35shell) silently disagree — exactly the drift the "ONE emission
path" rule exists to prevent. I verified the five sections are currently
byte-identical (md5 of each extracted `[qemu.netdev]…[dist]` region), so
this is latent, not live.
**Fix:** Extend the test to pin value equality — cheapest shape: compare
the serialized tables of config_q35 (the documented base) against the
other four, e.g. assert `toml::to_string(&q35.netdev) ==
toml::to_string(&other.netdev)` for each, or pin the emitted
`netdev_args_checked` output per mode for all five configs.

### IN-04: net-doctor's default `--mode all` hard-fails outside the Harvey checkout, contradicting the header's "HARVEY optional" framing

**File:** `nix/net-doctor.sh:45,253-260,417-435` with
`nix/net-doctor.sh:36-41` (the env contract)
**Issue:** The header documents `HARVEY` as optional and stresses that
"slirp/vmnet NEVER read the tap_ keys" — but the DEFAULT mode is `all`,
which includes `check_tap` → `seam_init`, and seam_init errs fatally when
`$HARVEY/build/x86_64/root-tree/lib/netaddr.rc` is absent (HARVEY defaults
to `$PWD`). So `nix run .#net-doctor` from any directory other than the
repo root exits 1 before printing any check lines; the seam-free modes
(`--mode slirp`, `--mode vmnet`) are the only truly HARVEY-independent
invocations. The failure is loud and self-explaining (the err names the
fix), so this is a contract-phrasing/robustness nit, not incorrect
behavior: the flake app does not export HARVEY, unlike the soak app which
exports `HARVEY_NETADDR_SEAM`.
**Fix:** Either reword the env contract ("HARVEY required for tap/all
modes"), or make the doctor default seam-independent (e.g. export
`HARVEY_NETADDR_SEAM` from the flake app as the soak app does, and have
`seam_init` honor it), so the default invocation works from any cwd.

---

_Reviewed: 2026-11-11T12:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
