# Phase 16: Host Net Modes + Networking Exit Gate - Pattern Map

**Mapped:** 2026-11-10
**Files analyzed:** 16 (11 Harvey, 5 r9 — counted as the 5-q35-TOML family = 1 group)
**Analogs found:** 15 / 16 (1 partial — the doctor)

Two-repo phase. Analog paths under `/home/req/code/r9/` are in the sibling r9 repo
(referred to as `../r9/` in planning docs); all others are Harvey-repo relative to
`/home/req/code/harvey/`. Every analog named below was verified git-TRACKED in its
repo (`git ls-files` non-empty) — no gitignored mirrors.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `../r9/xtask/src/lib.rs` (NEW) | library | transform (TOML→argv) | `../r9/xtask/src/config.rs` (code moves out of it) | exact |
| `../r9/xtask/src/bin/<emitter>.rs` (NEW) | CLI front-end | transform | `../r9/xtask/src/main.rs` QemuStep x86_64 arm (:720-768) | role-match |
| `../r9/xtask/src/config.rs` (MOD) | config+emitter | transform | itself — `Netdev` :128-139, `apply_netdev` :479-492, tests :805-860 | exact |
| `../r9/x86_64/lib/config_q35*.toml` ×5 (MOD) | config (data) | — | `../r9/x86_64/lib/config_q35shell.toml` :42-45 | exact |
| `../r9/flake.nix` (MOD) | build/package | build | `../r9/flake.nix` kernel-x64 :312-330 + optionalAttrs gates :369-373 | exact |
| `nix/boot-r9-x64.sh` (MOD) | launcher script | request-response | itself — R9_ACCEL case :95-107, netdev line :162-167 | exact |
| `nix/soak-r9-net-x64.sh` (NEW) | gate script | batch pipeline | `nix/smoke-r9-net-x64.sh` (whole file) | exact |
| `nix/net-doctor.sh` (NEW) | diagnostic script | request-response (probe+report) | `nix/boot-r9-x64.sh` contract checks :76-93 (fail-loud `err`) | partial |
| `nix/host-echo.c` (NEW, home planner choice) | host daemon binary | request-response (concurrent echo) | `../r9/xtask/src/main.rs` `HostEcho` :799-866 + `build/x86_64/tcpecho.c` | exact (two-source) |
| `flake.nix` (MOD) | build/package | build | itself — `netSmokeR9X64Pkg` :216-220, hasAttr gate :266-273 | exact |
| `build/x86_64/netsoak-r9.expect` (NEW) | gate driver | event-driven (serial markers) | `build/x86_64/smoke-r9-net.expect` (whole file) | exact |
| `build/x86_64/root-tree/lib/netaddr.rc` (MOD) | config (data seam) | — | itself — slirp leg :29-44 | exact |
| `build/x86_64/root-tree/lib/netip-tap.rc` (NEW) | in-guest fixture | request-response (net flows) | `build/x86_64/root-tree/lib/netip-static.rc` + `build/aarch64/root-tree/lib/netip-soak.rc` | exact |
| `build/x86_64/mkimage.sh` (MOD) | bake script | batch | itself — net section :480-599, seam assert :512-519 | exact |
| `build/x86_64/README.md` (NEW) | docs | — | `build/aarch64/README.md` (whole file) | role-match |
| `.github/workflows/r9-shell.yml` (MOD) | CI workflow | batch | itself — net-smoke job :57-66 | exact |

Not repo files (do not plan as such): the NixOS/colmena host config (br0, nat,
`environment.etc."qemu/bridge.conf"`, `security.wrappers`) — host OS state the
doctor verifies and the README documents; and any soak-surfaced r9 kernel fixes
(reactive, 15-D-01 push-before-pin, not a planned artifact).

## Pattern Assignments

### r9: the emitter refactor (`xtask/src/lib.rs` NEW, `config.rs` MOD, `bin/<emitter>.rs` NEW)

**Analog:** `../r9/xtask/src/config.rs` — the arg-building moves out; both front-ends consume it.

The struct today (:128-139) — grows mode sub-tables (`slirp`/`tap`/`vmnet` + a
`default` key; D-05's breaking change, migrated with all 5 TOMLs + tests together):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Netdev {
    /// netdev backend mode ("user" — slirp)
    pub mode: String,
    /// PCI device model bound to the netdev (virtio-net-pci-non-transitional
    /// — the modern-only, non-transitional transport, Phase 7 D-03)
    pub device: String,
    /// attach a per-run filter-dump pcap capture object when true
    pub pcap: bool,
}
```

The emission being refactored into a pure `netdev_args(config, mode, pcap_path) -> Vec<String>` (:479-492):

```rust
pub fn apply_netdev(cmd: &mut Command, config: &Configuration, pcap_path: &str) {
    let q35 = config.config.as_ref().and_then(|c| c.platform.as_deref()) == Some("q35");
    if !q35 {
        return;
    }
    let Some(netdev) = config.qemu.as_ref().and_then(|q| q.netdev.as_ref()) else {
        return;
    };
    cmd.arg("-netdev").arg(format!("{},id=n0", netdev.mode));
    cmd.arg("-device").arg(format!("{},netdev=n0", netdev.device));
    if netdev.pcap {
        cmd.arg("-object").arg(format!("filter-dump,id=f0,netdev=n0,file={pcap_path}"));
    }
}
```

Both in-process call sites keep byte-identical shapes — QemuStep x86_64 arm
(`main.rs:742-746`, pcap path `target/qemu-<config>.pcap`) and BootTestStep
(`main.rs:1699`, pcap path `target/boot-test-<config>.pcap`):

```rust
apply_netdev(
    &mut cmd,
    &self.config,
    &format!("target/qemu-{}.pcap", self.config_name),
);
```

The regression guard that must keep passing for the default path —
`q35_netdev_stanza_emits_from_data` (`config.rs:806-848`) asserts the exact argv
`["-netdev","user,id=n0","-device","virtio-net-pci-non-transitional,netdev=n0",
"-object","filter-dump,...,file=<path>"]`, plus `q35_netdev_table_requires_all_three_keys`
(:854-860, serde names a missing key) and `minimal_qemu_section_emits_neither_netdev_nor_extra`
(:901-911, emit-only-when-set). New mode-table tests copy these three shapes.

**Crate shape fact:** `../r9/xtask/Cargo.toml` is a single-bin crate (no `[[bin]]`,
no `[lib]`; deps = clap 4.5 / serde / target-lexicon / toml 0.9). Research's
recommended realization: add `src/lib.rs` holding the shared builder + config
module; `main.rs` and a new `src/bin/<emitter>.rs` become consumers — the
least-churn "ONE emission implementation, two front-ends" (D-05). Error idiom
inside config code: `eprintln!` + `exit(1)` (see `Initrd::loader_device_arg`
:188-193 and `Configuration::load` :242-259).

### r9: the TOML mode tables (`x86_64/lib/config_q35*.toml` ×5 MOD)

**Analog:** `../r9/x86_64/lib/config_q35shell.toml:42-45` — today's flat stanza
(identical in all five: config_q35, q35exec, q35lowmcfg, q35net, q35shell):

```toml
[qemu.netdev]
mode = "user"
device = "virtio-net-pci-non-transitional"
pcap = true
```

grows into per-mode sub-tables + a default key (D-05): `[qemu.netdev] default=...`
with `[qemu.netdev.slirp]` (mode/device/hostfwd list incl. D-04's
`tcp:127.0.0.1:17010-:17010` + `tcp:127.0.0.1:5670-:567`/pcap), `[qemu.netdev.tap]`
(mode/device/br/helper=`/run/wrappers/bin/qemu-bridge-helper`/pcap — the helper
path MUST be explicit, the compiled-in default is the nosuid store libexec),
`[qemu.netdev.vmnet]` (darwin-only). Comment style: the file's provenance headers
(`# The q35shell boot-chain profile (14-06, ...)`) carry decision IDs — extend,
don't strip. The TOML owns everything qemu sees; guest addresses stay in
Harvey's netaddr seam (D-06).

### r9: the exported emitter package (`flake.nix` MOD)

**Analog:** `../r9/flake.nix` kernel-x64 derivation (:312-330) — the emitter is a
host-std bin, so it will NOT need the objcopy/sysroot machinery, but the
packaging skeleton (src = `./.`, toolchain via `toolchainFor`, cargoDeps
symlinkJoin, `installPhase` cp to `$out`) is the shape to mirror:

```nix
kernel-x64 = pkgs.stdenv.mkDerivation {
  pname = "r9-kernel-x64";
  version = self.shortRev or "wip";
  src = ./.;
  nativeBuildInputs = [ toolchain pkgs.rustPlatform.cargoSetupHook ];
  inherit cargoDeps;
  buildPhase = ''
    runHook preBuild
    ${objcopyExport toolchain}
    cargo xtask build --arch x86-64 --config q35shell
    runHook postBuild
  '';
  installPhase = ''
    runHook preInstall
    mkdir -p "$out"
    cp target/x86_64-unknown-none-elf/debug/x86_64 "$out/kernel-x64.elf"
    runHook postInstall
  '';
};
```

Export gating precedent (:369-373) — the emitter joins a `optionalAttrs` set,
keeping eval lazy where absent:

```nix
// nixpkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
  inherit kernel-x64;
}
// nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
  inherit (builder) iso-builder;
```

For an exported SCRIPT (if the emitter wraps the bin in shell), the closer shape
is `iso-builder`'s `pkgs.writeShellApplication` (:75-82): `name` +
`runtimeInputs` + `text`, `usage()`/`err()` helpers first (:87-96). A5 risk
(this is the first host-std r9 package) is planner-visible; a plain
`cargo build -p xtask --bin <emitter>` line is the fallback.

### Harvey: `nix/boot-r9-x64.sh` (MOD — `--net`/R9_NET + emitter substitution + doctor-inline)

**Analog:** itself. Three regions:

1. Fail-loud `err` + env-contract checks (:72, :89-93) — the D-01/D-03 error
   vocabulary; `R9_NET` joins the header's env-contract comment block (:38-56):

```sh
err() { echo "error: $*" >&2; exit 1; }
...
[ -n "${R9_ISO_BUILDER:-}" ] || err "R9_ISO_BUILDER unset — run via the flake app ..."
[ -f "$R9_ISO_BUILDER" ] && [ -x "$R9_ISO_BUILDER" ] \
	|| err "R9_ISO_BUILDER $R9_ISO_BUILDER: not an executable file (want <pkg>/bin/r9-iso-builder — ...)"
```

2. The R9_ACCEL case (:97-107) — the exact template `--net` copies (flag wins
   over env, default, unknown-value usage error, cross-venue rejection naming
   the venue's valid modes). The script parses NO args today; `--net=tap`
   arrives as `$1` and needs a new arg-parse block adjacent to this:

```sh
R9_ACCEL="${R9_ACCEL:-kvm}"
case "$R9_ACCEL" in
	kvm)
		[ -e /dev/kvm ] || err "/dev/kvm absent and R9_ACCEL=$R9_ACCEL — no silent downgrade: set R9_ACCEL=tcg explicitly ..."
		R9_CPU="${R9_CPU:-host}"
		;;
	tcg)
		R9_CPU="${R9_CPU:-qemu64}"
		;;
	*) err "R9_ACCEL=$R9_ACCEL: want kvm|tcg" ;;
esac
```

3. The hardcoded line D-05 heals (:162-167) — the two netdev/device tokens
   become command substitution of the emitter's output for the selected mode
   (flake app exports the emitter path env-style, `R9_ISO_BUILDER` precedent):

```sh
set -- qemu-system-x86_64 \
	-M q35 -cpu "$R9_CPU" -smp 1 -m 1024M -accel "$R9_ACCEL" \
	-cdrom "$ISO" \
	-netdev user,id=n0 \
	-device virtio-net-pci-non-transitional,netdev=n0 \
	-nographic
```

Also keep: the argv echo before launch (:171-172, "captured gate logs are
self-describing") — the emitter's substituted argv should print the same way.

### Harvey: `nix/soak-r9-net-x64.sh` (NEW — D-13 soak app body)

**Analog:** `nix/smoke-r9-net-x64.sh` stage-for-stage. Copy the whole pipeline
skeleton and re-key: header comment block (purpose + env contract + hygiene
notes, :1-81), `set -e` + `err()` (:83-87), kernel-package/builder contract
(:91-104), accel case (:106-118), KERNDATE provenance (:125-133),
subset-build step (:135-142), net bake step asserting `out/bootfs-net.paq`
(:144-149), and — critically — the per-run tag + survivor-sweep trap (:153-175),
with a NEW tag prefix (e.g. `r9x64soak-`) so the soak never cross-kills the
sibling gates:

```sh
R9_RUN_TAG="r9x64net-$$-$(date +%s)"
export R9_RUN_TAG
cleanup() {
	status=$?
	pkill -f "[q]emu-system-x86_64.*${R9_RUN_TAG}" 2>/dev/null || true
	rm -rf "$TMPD"
	exit "$status"
}
trap cleanup EXIT
```

Deltas from the analog, per D-11/D-12/D-13: doctor-preflight step FIRST (invoke
the shared net-doctor implementation; refuse slirp runs outright — the
venue-honesty control); start the flake-built host echo peer on the bridge IP
as the app's own child before boot; boot `--net=tap` through the emitter;
expect driver is the new netsoak-r9.expect; green-twice with captures retained
+ r9 SHA recorded in evidence. Log contract mirrors `R9_NET_LOG` (:185-196:
default path, `rm -f` BEFORE launch, export to the driver).

### Harvey: `nix/net-doctor.sh` (NEW — D-14)

**No direct analog exists** (first diagnose+print checker in the tree). Closest
pattern pieces:

- Fail-loud `err` + contract vocabulary from `nix/boot-r9-x64.sh:72-93` (every
  check prints a PASS/FAIL block; every FAIL prints the exact remediation —
  NixOS declarative snippet first on NixOS, generic commands otherwise).
- Packaging via `mkScriptApp` (see Shared Patterns) with `iproute2`/`iptables`
  class tools in runtimeInputs (nft is NOT on the host PATH — research Environment
  Availability).
- Shape discipline: read-only probes (`ip link show br0`, `sysctl -n
  net.ipv4.ip_forward`, `ss -tln` port match, wrapper `-r-s--x--x` test,
  bridge.conf exact `allow br0` grep); exit 0/1 = CI-able preflight with the
  WARN class explicitly defined for the root-gated masquerade read (Pitfall 8);
  never fixes (D-14).
- One implementation, two front-ends: the boot script's tap leg (D-03) invokes
  this same script inline and embeds its per-issue remediation blocks in the
  failure output, naming `nix run .#net-doctor`.

### Harvey: `nix/host-echo.c` (NEW — D-12 host echo peer)

**Analogs (two-source):** echo semantics from `build/x86_64/tcpecho.c` (:29-48,
announce → listen/accept loop → read-what-came/write-it-back, loud sysfatal +
`%r` on every failure, stdout markers per event):

```c
cfd = announce("tcp!*!7", dir);
if(cfd < 0)
	sysfatal("tcpecho: announce tcp!*!7: %r");
print("tcpecho: announced\n");
for(;;){
	lfd = listen(dir, ldir);
	...
	dfd = accept(lfd, ldir);
	...
	print("tcpecho: conn\n");
	while((n = read(dfd, buf, sizeof buf)) > 0)
		if(write(dfd, buf, n) != n)
			sysfatal("tcpecho: echo write: %r");
```

Concurrency + lifecycle from r9's `HostEcho` (`../r9/xtask/src/main.rs:799-866`)
— the aarch64 NETIP-08 soak's own host peer: spawned BEFORE qemu ("the guest
never dials a closed port"), joined/stop-flagged after the run (dies with the
gate — D-12's no-daemon rule), nonblocking accept + per-stream read-timeout
poll loop. The C port must be fork-per-accept or a poll loop (Pitfall 7: the
in-guest tcpecho's strictly sequential accept would serialize the 8-way burst);
it binds the BRIDGE IP + blastport (from the committed seam values), not
127.0.0.1. Marker/byte-accounting discipline: deterministic per-connection
markers owned by the gate (D-12's rejection of socat drift).

### Harvey: `flake.nix` (MOD — 2 new apps + emitter-path export)

**Analog:** itself — `netSmokeR9X64Pkg` (:216-220) is the verbatim shape both
new apps copy (comment block naming the phase/decision IDs, `mkScriptApp`,
R9_* exports in `extraHeader`, expect in runtimeInputs):

```nix
netSmokeR9X64Pkg = mkScriptApp pkgs "harvey-boot-r9-net-smoke-x64" ./nix/smoke-r9-net-x64.sh ''
  export R9_KERNEL_X64_PKG="${r9.packages.${system}.kernel-x64}"
  export R9_ISO_BUILDER="${r9.packages.${system}.iso-builder}/bin/r9-iso-builder"
'' ([ pkgs.plan9port pkgs.bash pkgs.qemu pkgs.git pkgs.expect ]
  ++ pkgs.lib.optionals isLinux [ pkgs.util-linux pkgs.gnused pkgs.mount pkgs.binutils ]);
```

The new apps add `export R9_NETDEV_EMITTER="${r9.packages.${system}.<emitter>}/bin/<emitter>"`
(R9_ISO_BUILDER precedent). Gating — do NOT extend the existing list
(:266-273, anti-pattern per research Pitfall 6); the new apps carry their OWN
`optionalAttrs` keyed on the attrs they actually consume (added only after the
r9 pin advance commit):

```nix
// nixpkgs.lib.optionalAttrs
  (builtins.all (a: builtins.hasAttr a r9.packages.${system}) [ "kernel-x64" "iso-builder" ]) {
  boot-r9-x64 = mkApp bootR9X64Pkg;
  ...
```

The doctor app needs no r9 attrs (host-only checks) and the soak app also
consumes the host-echo binary — a `pkgs.stdenv.mkDerivation`/`pkgs.cc` build of
`nix/host-echo.c` joined into its runtimeInputs, or a packages attr beside
`harness` (:49-58, the existing rustPlatform build precedent for a committed
binary — a C variant uses `pkgs.stdenv` `buildPhase = 'cc ...'`).

### Harvey: `build/x86_64/netsoak-r9.expect` (NEW — soak driver)

**Analog:** `build/x86_64/smoke-r9-net.expect` (whole file). Carry verbatim:

- The header discipline blocks: stage-order rationale (:47-56 — including the
  named multi-waiter RX-IRQ defect warning "Phase 16's tap legs WILL hit the
  multi-waiter path"), venue-label note (:87-93), marker discipline (:102-127:
  PRESENCE-only never order, CRLF-tolerant substrings, two-occurrence staging
  for typed-line-contained markers, ≤120-char typed lines with continuation-prompt pacing).
- Env contract + fresh-capture (`exec rm -f`, :152-165) and the DIRECT qemu
  spawn (:174-180) — the soak replaces the two netdev lines with the emitter's
  tap argv (caller shells out once, exports tokens e.g. `R9_NETDEV_ARGS` for
  the driver to split):

```tcl
spawn $QEMU -M $R9_MACHINE -cpu $R9_CPU -smp 1 -m 1024M \
    -accel $R9_ACCEL \
    -cdrom $env(R9_ISO) \
    -netdev user,id=n0 \
    -device virtio-net-pci-non-transitional,netdev=n0 \
    -nographic \
    -name $run_tag
```

- The `stage` proc (:199-206), `expect_before` fast-fail arms (:217-228 — panic
  + `netip: FAIL`), teardown `cleanup` killing the process group (:189-193).
- The whole-capture post-pass shape (:410-430: `log_file` stop+flush, read the
  file, string-match asserts) — the burst leg's marker-COUNT assert (count N
  `tcpblast: done` occurrences == N instances, never order) and the ARP
  host-entry assert are new post-passes in this shape. Typed-line fixture
  launches ride `send "rc /lib/netip-tap.rc\r"` + output-only marker stages
  (the :251-256 static-leg shape).

### Harvey: `build/x86_64/root-tree/lib/netaddr.rc` (MOD — tap_ leg)

**Analog:** itself — the slirp leg block (:29-44) is the exact shape the tap
leg copies (comment block citing the venue's properties, prefixed keys,
`blastport` reused; research recommends adopting the host's existing
br0/192.168.122.1/24 → `tap_guestv4=192.168.122.15`, `tap_maskv4=255.255.255.0`,
`tap_gwv4=192.168.122.1`):

```rc
# --- slirp leg (the netip-static/dhcp fixtures: static ipconfig, DHCP
#     via ipconfig's dhcpclient) ---
# qemu -netdev user defaults: vnetwork 10.0.2.0/24, host 10.0.2.2, guest
# 10.0.2.15 (DHCP start).  Hermetic/unprivileged/CI-able; every
# slirp-green gate is labeled not-L2-truth ...
slirp_guestv4=10.0.2.15
slirp_maskv4=255.255.255.0
slirp_gwv4=10.0.2.2

# tcpblast host-echo port (...)
blastport=7777
```

Constraints: ADDITIVE keys only (the slirp gates' fixtures must not perturb —
last-wins discipline); the header's v6 note (:19-22) is re-keyed only under
D-09's trivially-free clause; prose mentioning the tap subnet may appear ONLY
here (the 15-07 comment-prose lesson).

### Harvey: `build/x86_64/root-tree/lib/netip-tap.rc` (NEW — re-venued fixture)

**Analogs:** `build/x86_64/root-tree/lib/netip-static.rc` (the re-venuing base)
+ `build/aarch64/root-tree/lib/netip-soak.rc` (the soak-leg extension).

From netip-static.rc carry the whole skeleton: the sourced-not-typed header
naming the phase/fixture IDs; the seam source with `test -e` first (:40-44 —
"a failed source is uncatchable"); `bind -a '#I' /net` (:46-50); the REAL
ipconfig static flow with seam values (:57-60); word-wise `/net/ipselftab`,
`/net/iproute` assertions (:64-100); the gateway ping (:106-109); the
`/net/arp` post-ping view (:114-127 — on tap this becomes the L2-TRUTH assert,
D-09); every failure `echo netip: FAIL <stage> >[1=2]; exit <stage>`:

```rc
test -e /lib/netaddr.rc || {
	echo netip: FAIL seam >[1=2]
	exit seam
}
. /lib/netaddr.rc
...
ip/ipconfig -g $slirp_gwv4 ether /net/ether0 add $slirp_guestv4 $slirp_maskv4 || {
	echo netip: FAIL ipconfig >[1=2]
	exit ipconfig
}
```

The tap venue keys replace `slirp_*` with `tap_*`; the venue label flips from
the NOT-L2-TRUTH disclaimer (netip-static.rc :129-133) to the tap-venue TRUTH
label. From netip-soak.rc carry: the `$1` rounds argv validation switch
(:49-69), the blast-leg `tcpblast $tap_gwv4 $blastport [$rounds]` with the ||
FAIL idiom (:100-111), and the `/net/tcp/stats` cat + InSegs/OutSegs non-zero
reconciliation (:113-146) if the soak's stats post-pass is wanted host-side.
The BURST leg (D-11b) is new: N backgrounded tcpblasts + bare `wait` + a
completion marker the expect post-pass COUNTS (rc cannot collect per-child
statuses after `wait` — `sys/man/1/rc:736-743, 881-883`; research's rc burst
shape). Backgrounding precedent in-tree: `build/x86_64/services-sweep.rc:199,221,280`
(`$B/hcs -f /env/svcndb &` then a `svc_waitpath` poll — the daemon-background
idiom the burst copies for tcpblast).

### Harvey: `build/x86_64/mkimage.sh` (MOD — assert extension + soak payload)

**Analog:** itself — the net bake section (:480-599). Two edits:

1. The seam assert (:512-519) gains a second grep for the tap subnet literal,
   same netaddr.rc exclusion (D-06):

```sh
_stray=$("$GREP" -rn '10\.0\.2\.' "$ROOT_TREE/lib/" 2>/dev/null | "$GREP" -v 'netaddr\.rc' || true)
[ -z "$_stray" ] || err "venue address literal outside netaddr.rc (the D-11 seam violated): $_stray"
```

2. The payload lists extend additively: `netip-tap.rc` joins the fixture
   presence check (:508-510), the staging copy (:528 tar of ROOT_TREE carries
   it automatically), and the pre-bake manifest (:568-579 — add
   `lib/netip-tap.rc`). If the soak needs its own image variant, copy the
   section-10 structure (inputs check → skeleton tar → payload cp → manifest →
   determinism touch → mkpaqfs → magic check :597-599) as a new numbered
   section; the tcpblast binary is already staged (:554-556).

### Harvey: `build/x86_64/README.md` (NEW — per-distro docs, D-10)

**Analog:** `build/aarch64/README.md` — the drift-notes discipline: a title
scoping the directory, prose sections carrying decision IDs (D-08/D-10), a
"Scripts and their contracts" table (one row per app/script: name → contract),
probe-evidence sections with exact measured facts ("TCG-only constraint"
:45-63 is the model — the NixOS qemu-bridge-helper nosuid-store divergence and
the bridge.conf `allow virbr0`-vs-`allow br0` live findings belong in exactly
that voice), and explicit not-tested labels for the generic-Linux and darwin
pages. Content: NixOS declarative path (research Code Examples has the verified
snippet — networking.bridges/nat/sysctl/trustedInterfaces, environment.etc
bridge.conf, security.wrappers), generic-Linux imperative one-pager, darwin
socket_vmnet manual leg, the D-04 5670 remap rationale + the 1:1 sysctl
opt-in, and the real-LAN variant paragraph (not tested).

### Harvey: `.github/workflows/r9-shell.yml` (MOD — HNET-01 CI leg)

**Analog:** itself — the net-smoke job (:57-66) is the verbatim job shape the
new x64 leg copies (comment block citing phase/decision + supply-chain
provenance, `runs-on: ubuntu-latest`, `timeout-minutes: 90` TCG scale,
checkout pinned to the API-resolved SHA, nix-installer pinned SHA, one
`nix run .#<app>` line):

```yaml
  net-smoke:
    runs-on: ubuntu-latest
    timeout-minutes: 90 # TCG scale; the smoke's own per-stage 600s timeouts localize failures
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4 — API-resolved 2026-11-06 (the 06-08 rule)
      - name: install nix
        # pinned SHA (see header): the supply-chain rule
        uses: DeterminateSystems/nix-installer-action@3138316df39ed29be04236d7ffc686fa525866aa
      - name: composed net smoke (boot-r9-net-smoke)
        run: nix run .#boot-r9-net-smoke
```

The new leg runs the x64 composed net gate (or the slirp-default boot variant —
research Open Question 2) with `R9_ACCEL=tcg` (env: prefix on the run line),
asserting the hostfwd ports bind. Header supply-chain rules (:17-21) already
in force. Tap/soak legs NEVER run here (impossible unprivileged — D-13).

## Shared Patterns

### Fail-loud mode selection (flag > env > default, no silent downgrade)

**Source:** `nix/boot-r9-x64.sh:95-107` (excerpt above)
**Apply to:** the `--net`/`R9_NET` parse (D-01), the soak app's slirp refusal
(D-11), the doctor's per-mode check gating (D-14), the emitter's unknown-mode
error. Idiom: `case` ladder + `err "X=$X: want a|b|c"`; cross-venue requests
are usage errors naming the venue's valid modes.

### mkScriptApp + hasAttr app-per-gate shape

**Source:** `flake.nix:26-39` (mkScriptApp), `:216-220` (an app), `:266-273` (gating)
**Apply to:** both new Harvey apps (soak, doctor) and the `R9_NETDEV_EMITTER`
export. PLAN9 env + `$PLAN9/bin` LAST on PATH rides every app; new apps get
their OWN optionalAttrs gate, never an extension of the frozen list.

### One implementation, two front-ends

**Source:** `../r9/flake.nix:43-75` (iso-builder: one script behind the FOD, the
flake-lib, and Harvey's app) + `apply_netdev` shared by QemuStep/BootTestStep
**Apply to:** the emitter (xtask in-process + exported bin, D-05) and the
doctor (standalone app + boot-script tap leg, D-03/D-12). Constraint: both
front-ends must produce/consume byte-identical output.

### Venue honesty + evidence discipline

**Source:** `build/x86_64/root-tree/lib/netip-static.rc:129-133` + the NOT-L2-TRUTH
stage asserts at `smoke-r9-net.expect:254,298`
**Apply to:** the tap fixture's venue label (now the L2-TRUTH claim), the soak
app's slirp refusal, captures retained green-twice with the r9 SHA recorded.

### Serial-marker discipline (presence-only, count-based under concurrency)

**Source:** `build/x86_64/smoke-r9-net.expect:102-127` (single/two-occurrence
staging, CRLF tolerance, ≤120-char typed lines), post-passes `:410-430`
**Apply to:** `netsoak-r9.expect` — burst assertions are per-marker-COUNT
post-passes, never interleaving order (CONTEXT watch-out).

### rc fixture dialect

**Source:** `netip-static.rc` (FAIL-stage exits, `test -e` before source,
`~` word compares, no `=` in patterns) + `services-sweep.rc:192-211` (background
`&` + poll-wait)
**Apply to:** `netip-tap.rc` — including the 15-02 constraints (flat status
ladders, no braced if-not bodies).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `nix/net-doctor.sh` | diagnostic | probe+report | First read-only host-state checker in either repo — no PASS/FAIL-block precedent exists. Composition guidance above (err idiom + mkScriptApp + research's live-probed check list + NixOS remediation snippet). Use RESEARCH.md "Code Examples" (NixOS declarative remediation) + Pitfalls 1/2/8 for the check semantics. |

Everything else has an in-tree analog. The NixOS/colmena host config and any
reactive r9 kernel fixes are not planned files (see File Classification note).

## Metadata

**Analog search scope:** Harvey `nix/`, `build/x86_64/` (incl. root-tree/lib),
`build/aarch64/`, `flake.nix`, `.github/workflows/`; r9 `xtask/src/`,
`x86_64/lib/*.toml`, `flake.nix`. Grep-verified: no other x64 gate legs exist in
`.github/`; `services-sweep.rc` lives at `build/x86_64/` (NOT root-tree/lib —
research's path citation imprecise, tracked either way).
**Files scanned:** 18 analog files read (all tracked-source-verified).
**Pattern extraction date:** 2026-11-10
