# Phase 15: Userspace Tiers + Bootfs Payload (amd64) - Research

**Researched:** 2026-11-09
**Domain:** Harvey userspace tier sweep (services/GUI/long tail) on the k10t1 9k donor venue + r9-x86_64, GUI syscall tracing on a draw-enabled 9k donor, and the DESK-03 bootfs payload
**Confidence:** HIGH for machinery/tier/probe/payload findings (all verified in-repo this session); MEDIUM for the draw-donor VGA bring-up routes (three viable routes sized, none built) and the r9 fix-everything gap inventory (gaps named by the committed sweep, sizes not yet measured)

## Summary

Phase 15 is almost pure orchestration of proven Phase-14 machinery: the tier data files + mk filters (5 sites, dual-arch), subset-build.sh's per-entry dispatch, mkimage.sh's four bake contracts (a fifth joins by the same shape), the k10t1 venue scripts, and the r9-x86_64 gate apps (nix/tier1-r9-x64.sh shape). The compile inventory is strong: on aarch64 the whole tier-2/3/4 membership already compiled `build=ok` (fossil, venti, rio, acme, sam, samterm, plumb, upas, auth, ip, ndb) with failures confined to the pcc/APE class — and that class is structurally open on amd64: `sys/src/ape/lib/ap/amd64/` exists (atom.s, cycles.s, lock.c, main9.s, notetramp.c, _seek.c, setjmp.s, ...) and `sys/src/cmd/gs/default.amd64.h` exists, so gs/mp3enc/postscript compile stock [VERIFIED: `ls sys/src/ape/lib/ap/amd64/`, `ls sys/src/cmd/gs/default.amd64.h`, this session].

The two genuinely new engineering surfaces are both in the GUI leg (USPACE-08). **(1) The draw donor config:** the in-tree `k10f` config is the exact device-set template (`draw drawalloc screen vga vgax` / `mouse mouse` / a `vga` section carrying `vgavesa` / `kbmap`), every device file exists — but the vesa route is INCOMPLETE on k10: `vgavesa.c`'s `vbecall()` opens `/dev/realmode`, and the real-mode trampoline was never ported (k10/realmode.c serves only `realmodemem`; the executor lives in legacy `sys/src/9/pc/realmode.c` + `pc/l.s` `realmode0` at line 323, with `RMUADDR/RMCODE/RMBUF` constants in `pc/mem.h:61-63` that k10/mem.h lacks). Three bring-up routes are sized below; a qemu-native DISPI+PCI-LFB VGAdev is the smallest and best-matched to the modern-only venue. **(2) Font delivery:** rio's default font `/lib/font/bit/lucm/unicode.9.font` [VERIFIED: sys/src/cmd/rio/rio.c:160-165] cannot ride the kernel embed (36MB/4303 files). The clean route: qemu `-initrd` registers as an mb1 module under `-kernel`, `multiboot.c`'s Fmods loop already walks modules (today it only reserves their memory via `asmmodinit`) — exposing module content through the existing `addbootfile()` is a ~3-line kernel delta, and the tier-1-built `paqfs` mounts it in-guest: the same bootfs.paq mechanism r9 uses, no srv9, no network.

Synthetic input is half-there: `/dev/mousein` EXISTS on 9k [VERIFIED: sys/src/9k/port/devmouse.c:93-96 — `"mousein", {Qmousein}, 0, 0220` — and its write handler parses `M x y b msec` text into `mousetrack()`], but there is NO `/dev/kbin` anywhere in 9k (nor 9/pc): keyboard injection either rides the serial console (k10's `uartconsole()` registers the uart input queue via `addkbdq` [VERIFIED: sys/src/9k/port/devuart.c:149] so `/dev/cons` reads see serial input — zero kernel work, host-anchored) or adds a small kbin-class write file to devcons (a dozen lines feeding the same `kbdputc(kbdq,...)` path 386/kbd.c:477 uses — fully deterministic in-guest replay). The trace substrate is present: 9k devproc serves `/proc/N/syscall` and the `startsyscall` ctl verb [VERIFIED: sys/src/9k/port/devproc.c:98,115], and `build/aarch64/tracesup.c` is arch-neutral Plan 9 C that builds with 6c.

**Primary recommendation:** land tier-2 as the first mass wave with the anti-staleness decision (per-entry `mk clean` in the tier driver) in that plan; build the services image as mkimage.sh's fifth contract riding the 14-09 net bring-up; size the draw-donor as a new k10t1-sibling config with the DISPI+PCI-LFB route (realmode port as the fallback), the `-initrd`→addbootfile→paqfs font channel, `/dev/mousein` as-is, and a new kbin-class devcons write file for deterministic keyboard replay; stage DESK-03 into root-tree with bake-time font copies, a token-substituted ndb template, and the early boot-memory headroom check (36MB raw → deflate; r9 legs boot `-m 1024M`).

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** **Fix everything in-phase.** Every r9 kernel gap surfaced by services execution is root-caused and fixed this phase — push-before-pin, FOD re-wire-then-pin per kernel-byte commit (the 13-06/14-07 discipline). **Boundary:** Phase-18-owned substrate is NOT pulled forward — devcap `#¤`, the bake-time `/adm/keys` domain, keyfs stay 18's; auth entries needing that substrate take named SKIP→18. The factotum devproc-protect/noswap + devsrv-mount gap class is NOT 18-owned — fix-in-phase if the sweep surfaces it (planner cross-checks Phase 17's named scope for overlap).
- **D-02:** **Functional probe markers for daemons** — a daemon earns its marker when a real client interaction succeeds through it (cs answers a query via /net/cs, listen's announce accepts a loopback connect, dns resolves a baked hosts entry, exportfs serves a guest-side mount, mntgen mounts). Impossible-on-venue = named SKIP, never a weaker pass. (Carries v2.0 D-03's probe semantics re-keyed to the x64 venues.)
- **D-03:** **A new dedicated services image** — mkimage.sh's fifth contract: chain tree + services payload + the 14-09 net bring-up (ipconfig before daemons), its own services-sweep.rc run from termrc (the tier-termrc shape), its own expect gate per venue. One-image-per-gate preserved; Phase 18/19 consume it.
- **D-04:** **Draw-enabled kernel-local donor venue THIS phase; the 9P topology later.** A new 9k donor config variant (k10cpu device set + vga/draw/mouse) boots under qemu -vga std; rio/acme/sam genuinely start against a kernel-local devdraw; the full client-side syscall mix is captured. The drawterm-derived serve-only 9P topology belongs to the phase that grows r9's desktop path (deferred). — **Reversibility:** costly.
- **D-05 (ADOPTS v2.0 D-10):** **Scripted-interaction traces.** Each traced program is driven by a committed replay script — rio: menu New + type into the rc window; acme: type, execute a command, quit; sam+samterm: type, edit, write. On the kernel-local venue the deterministic injection is in-guest synthetic input streams (the /dev/kbin + /dev/mousein class — researcher verifies 9k's exact served files); determinism rides the replay mechanism, never GUI-automation luck. Startup-to-idle is the floor, not the ceiling.
- **D-06:** **Raw + distilled table, both committed.** Raw per-binary trace logs plus a distilled syscall-set table per binary (syscall → count → args-shape notes, the 05-03 measured-set format).
- **D-07:** **The trace is the gate.** No separate GUI exec markers — the committed trace+table is USPACE-08's runtime artifact; the rig asserts the script COMPLETED (reached the event loop and finished the script — never merely "syscalls occurred"). GUI compile gets artifact markers like every tier.
- **D-08 (carried, v2.0 D-05 re-keyed):** **Auth suite shallow execution subset** where substrate allows: factotum starts and answers a scripted probe, keyfs serves over a test key file, authsrv starts and listens — bounded by D-01's 18-owned boundary (the k10t1 donor has cap/ssl/tls in its device set, so the donor leg runs these for real; the r9 leg follows the machinery). Real p9sk1 handshakes stay Phase 18's.
- **D-09:** **Full DIRS-coverage proof re-keyed to amd64** — every sys/src/cmd dir (plus any unproven lib dirs) compiles for amd64, machine-verified coverage assertion, not sampled. The Phase-6 dir-coverage machinery re-keyed.
- **D-10:** **Fix in-tree, no skip-vocabulary for compile failures.** Every sweep failure is root-caused and fixed (one commit per bug class); the coverage proof only ends green. The aarch64 exclusion classes are RE-DERIVED, not carried: the "x86-hardware-only" class compiles natively on amd64; per-arch-asm classes re-derived.
- **D-11 (ADOPTS v2.0 D-02):** **gs render smoke beyond compile.** gs renders a tiny checked-in .ps fixture to a bitmap FILE device (no display) on the donor. On amd64 the ape wall DOES NOT EXIST (verified at discussion: `sys/src/ape/lib/ap/amd64/` and `sys/src/cmd/gs/default.amd64.h` are in-tree) — gs compiles stock through its pcc route. gs's GPL fonts are never staged anywhere.
- **D-12:** **Per-class append-only tier data files** — new tier files beside the untouched tier0/tier1 files; each gate computes expectations from its own file; diffs review one class at a time. Exact names are planner discretion.
- **D-13:** **Payload lives in the committed root-tree skeleton** (`build/x86_64/root-tree/`) — the shared base every mkimage contract stages from. — **Reversibility:** costly.
- **D-14 (carried, v2.0 D-13):** **The WHOLE `lib/font/bit` tree ships** — all 36MB, no exclusions. bootfs growth ~8×; the boot-memory-headroom check happens EARLY in the DESK-03 plan.
- **D-15:** **Manifest = committed data file + bake-time assert.** A plain-data manifest consumed by mkimage.sh; the bake FAILS if any listed path is missing/empty; no second hardcoded list anywhere.
- **D-16 (carried, v2.0 D-15):** **/lib/ndb/local is a committed TEMPLATE substituted at bake time from the netaddr seam** — the template MUST be substitution-based to pass mkimage's net-section literal grep assert.
- **D-17 (carried, v2.0 D-14):** **/usr/glenda/lib/profile stages as the trimmed cpu-aware reference**: `upas/fs` + `news` dropped; the `case terminal` path keeps its binds/tmp/font but LOSES `exec rio` (rio must provably not start at boot); the `case cpu` path intact (binds /mnt/term/dev, `font=/lib/font/bit/pelm/latin1.8.font`, `exec rio`) for Phase 19.

### Claude's Discretion
- Exact tierN file names and per-tier membership order; wave decomposition within tiers 2/3/4 (planner's dependency sort; tier files are the record).
- The draw donor config variant's name/build (k10t1 sibling vs k10cpu variant config), its vga bring-up mechanics under qemu q35 (-vga std), and how the venue scripts (build/mm) gain its leg.
- The synthetic-input mechanism's exact files on 9k (kbin/mousein class) and the replay-script format; trace capture mechanics on the donor (the 05-03 rig lineage re-keyed); trace file layout under the phase dir.
- Functional-probe script internals per service (fixed inputs, marker vocabulary, loopback ports) — semantics locked by D-02/D-08.
- The ndb/local template's home + placeholder token syntax (D-16's seam constraint locked); bootfs staging mechanics for fonts/profile (committed staged tree vs bake-time copy — the adm/timezone bake-time-copy precedent is available).
- CI job topology: tier compile gates in CI; donor/r9 execution legs follow the Phase-14 venue precedent — planner decides per-leg placement.
- Evidence-file organization under `.planning/phases/15-*/`.

### Deferred Ideas (OUT OF SCOPE)
- **The draw-over-9P topology rig** — drawterm-derived serve-only host binary, host-serving channel, topology re-capture: the phase that grows r9's desktop path.
- Phase-18 auth substrate: devcap `#¤`, bake-time `/adm/keys` domain generator, keyfs-as-real-service, real p9sk1 handshakes, secstore.
- `#!` exec, service=cpu bootconf, aux/listen live at boot, drawterm flake app, rio in-session — Phase 19.
- fossil/venti runtime verification — future milestone (needs a block device in r9).
- upas send/receive UAT; lp gs-dependent stages — carry the ancestor's PITFALLS exclusions unless the amd64 re-derivation changes them.
- Hosted-CI donor-execution legs — planner discretion per the venue precedent.
- virtio-blk/rng/9p host mount, SMP — future milestones.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| USPACE-07 | Services tier compiles AND executes: aux/listen, mntgen, ndb/cs+dns, exportfs, ip/ suite, auth/ suite (~80 files) on k10t1 then r9-x86_64 | Compile inventory from the aarch64 coverage (auth/ip/ndb/exportfs all `build=ok` there; amd64 is the easier arch). Probe clients all in-tree (csquery, dnsquery, dial/pass, srv -e, mntgen). Probe semantics per service below. Services image = mkimage fifth contract. r9 gap inventory: ipconfig SKIP stale (flips live); factotum-class gaps are D-01 fix-in-phase candidates. |
| USPACE-08 | GUI stack compiles (libdraw family + rio + acme + sam + plumb); donor-kernel syscall traces for rio/acme/sam captured | libdraw/libmemdraw/libmemlayer tier-0 appends; rio needs libdraw+libframe+libthread+libplumb [VERIFIED mkfiles]. Draw donor: k10f config template + the vesa/realmode gap documented + 3 sized routes; font delivery via -initrd→addbootfile→paqfs; /dev/mousein exists, kbin must be added (or serial-injection rig); devproc startsyscall + tracesup re-key. |
| USPACE-09 | Long tail compiles: upas + lp + remaining cmd dirs; fossil+venti compile-gated/documented; gs included + render smoke | 130 cmd dirs + ~130 single .c files. pcc/APE class opens on amd64 (ape lib + default.amd64.h in-tree). Exclusion classes re-derived (Harvey games has no snes/gb; mp3enc CC=pcc now viable). coverage-sweep.sh re-keyed to amd64. gs render smoke shape below. |
| DESK-03 | /lib/font/bit/* + glenda-style profile + /lib/ndb/local template staged in bootfs with manifest assertions | root-tree skeleton mapped (9 files at plan-revision time — the earlier "10 files" was a miscount); 36MB/4303 files measured; paq sizes measured (chain 372KB, tier 1.85MB); bake-time copy + manifest shape; ndb template must pass the 10b literal grep; profile trim reference read verbatim; early headroom check. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tier-2/3/4 compilation (6c/6l over allowlists) | Host namespace build (unshare; linux-native, no lima leg on x64) | — | The proven subset-build.sh path; mk filters read the tier files $HARVEY-absolute |
| Services functional probes | k10t1 donor guest first, then r9-x86_64 guest | Host drivers (venue scripts + expect gates) | D-02/D-03: both venues execute; sourced sweeps from termrc; serial markers |
| Draw serving for GUI traces | 9k donor kernel (devdraw #i, kernel-local) | — | D-04: kernel-local THIS phase; the draw device tree binds via `bind("#i", "/dev", MAFTER)` in libdraw's initdraw |
| VGA mode-set + LFB on the donor | 9k k10 kernel (new VGAdev route) | qemu std-vga (SeaBIOS DISPI) | No bootloader on the -kernel venue; the kernel must set the mode itself (see routes) |
| Synthetic input (mouse/kbd) | 9k kernel (mousein exists; kbin-class addition) + in-guest replay scripts | serial chardev (host-anchored alternative) | D-05: determinism rides the replay mechanism |
| Syscall tracing | 9k donor guest (devproc startsyscall + tracesup) | Host parse (distilled tables) | Born-traced arming; fragments egress over serial (no srv9 on this venue) |
| gs render smoke | donor guest (headless bitmap FILE device) | — | D-11: no display dependency |
| Bootfs payload staging (DESK-03) | Host bake (mkimage.sh + root-tree skeleton) | r9 consumption Phases 18/19 | D-13/D-15: manifest asserts at bake time |
| r9 kernel fixes surfaced by services | ../r9 repo (push-before-pin) | Harvey pin advance | D-01 fix-everything, own commits |

## Standard Stack

**No new external packages.** Everything consumes in-tree sources and Phase-14-provisioned host tooling.

| Component | Version/Pin | Purpose | Why Standard |
|-----------|-------------|---------|--------------|
| Harvey 6c/6a/6l toolchain | `.build/hostbin-linux-x86_64/bin` (subset-build bootstraps) | compile all tiers | XEXEC-04-proven; `nix run .#setup && nix run .#toolchain` |
| mkimage.sh + mkpaqfs-host.sh | in-tree, `.build/bin/mkpaqfs` already built | bootfs bakes | 4 contracts proven; the fifth joins the same shape |
| qemu-system-x86_64 | 11.1.2 (devshell, verified on this host) | donor + r9 venues | KVM writable on the primary host; TCG fallback with generous budgets |
| build/mm venue scripts | build-9k10t1.sh, boot-9k10t1-sweep.sh | donor build + boot harness | 43-file embed proven; the r9 arm drives the composed ISO |
| nix gate apps | nix/tier1-r9-x64.sh shape (flake app boot-r9-tier1-x64) | r9-x64 gates | services gates extend the same pipeline shape |
| tracesup.c | build/aarch64/tracesup.c (arch-neutral Plan 9 C) | born-traced supervisor | builds with 6c for amd64; feeds off /proc/N/syscall |
| plan9port | devshell | host mk/rc | $PLAN9/bin LAST per project rule (mkimage.sh:95-99 enforces) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Per-entry `mk clean` in the tier-2 driver (anti-staleness) | committed object sweep on toolchain change / documented operator step | the operator step is what made 06-03 honest; per-entry clean is deterministic, tier-local, in-driver — RECOMMENDED (decide in the FIRST tier-2 plan per the carried checklist) |
| -initrd→addbootfile→in-guest paqfs (draw-donor payload channel) | network leg + host 9P server | the net route re-introduces a host-serving channel the venue explicitly lacks (NO srv9); the module route reuses r9's own bootfs.paq mechanism with a ~3-line kernel delta |
| kbin-class devcons write file (keyboard replay) | serial-chardev injection (host-anchored sends) | serial works TODAY with zero kernel work; kbin is ~a dozen lines and makes replay fully in-guest/deterministic (D-05's letter) — planner picks per the determinism bar |
| Bake-time font copy (mkimage.sh) | committed root-tree/lib/font/bit mirror | a 36MB duplicate tree invites drift — the adm/timezone bake-time-copy precedent (mkimage.sh:256-257) governs |

## Package Legitimacy Audit

**No external packages are installed by this phase.** All tier builds are in-tree Plan 9 C with the vendored toolchain; host tooling (qemu, nix, plan9port, mkpaqfs) was provisioned in earlier phases. No npm/pypi/crates operations occur.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
Host (linux x86_64 primary; nix devshell)
  ├─ subset-build.sh (extended: tiers 2/3/4 appends)
  │     reads build/x86_64/tiers/tier{0..4}-*.txt ──► namespace build (userns chroot
  │     scaffold — no passwordless sudo on this host) ──► /amd64/{bin,lib}
  │     emits T0/T1/T2/T3/T4-ART sha256 markers (mk exit never trusted)
  ├─ mkimage.sh ──► 4 existing contracts + NEW services contract:
  │     chain tree + h*services payload + netaddr-derived fixtures +
  │     services-sweep.rc baked at /lib + services termrc override
  │     ──► out/bootfs-services.paq ──► r9 iso-builder ──► composed ISO
  ├─ coverage-sweep.sh re-keyed (objtype=amd64, COVERAGE_SWEEP=1)
  │     ──► build/x86_64/dir-coverage.txt (machine-verified DIRS proof)
  │
  ├─ k10t1 donor leg (build/mm): build-9k10t1.sh (CONF=k10t1) ──► qemu
  │     -kernel 9k10t1.elf -m 512 … services-sweep from embedded root
  │     ──► serial markers (SVC-OK/FAIL/SKIP) ──► boot-9k10t1-sweep.sh classify
  ├─ r9-x64 leg: composed services ISO ──► qemu -M q35 -m 1024M -cdrom …
  │     ──► services termrc runs the sweep ──► nix/services-r9-x64.sh expect gate
  │     ──► surfaced kernel gaps ──► ../r9 fixes (push-before-pin, own commits)
  │
  └─ DRAW DONOR leg (new): build/mm gains a k10-draw variant
        qemu -kernel 9kdraw.elf -initrd bootfs-draw.paq -vga std …
        kernel: k10cpu set + draw/mouse/vga/kbmap devices (k10f template)
        mb1 module exposed via addbootfile ──► in-guest paqfs mounts /n/paq
        ──► bind fonts/GUI binaries; vga mode-set (DISPI route) ──► /dev/draw/new
        replay script (committed rc data) ──► /dev/mousein + kbin-class writes
        tracesup (6c-built) step-traces rio|acme|sam trees ──► /tmp fragments
        (mntgen scratch) ──► serialized out over the serial capture
        ──► host distills raw logs + per-binary syscall tables (both committed)
```

### Recommended Project Structure (new files only)

```
build/x86_64/
├── tiers/tier2-services.txt     # D-12 data: ip tree, ndb, aux:listen, mntgen, exportfs, auth suite, dial/import/srv, write-flow class
├── tiers/tier3-gui.txt          # rio acme sam samterm plumb stats winwatch page proof screenlock + libdraw-family lib appends ride tier0
├── tiers/tier4-tail.txt         # the rest + fossil venti (compile-gated) + gs + lp/upas + re-derived exclusions-with-comments
├── markers/tier{2,3,4}.ledger   # or flat tier{2,3,4}.ledger beside tier1.ledger — planner names them
├── services-sweep.rc            # the D-02/D-08 functional-probe sweep (sourced from termrc)
├── services-termrc              # the services image's termrc override (tier-termrc shape)
├── services-r9.expect           # the r9 venue's expect gate (tier1-r9.expect shape)
├── dir-coverage.txt             # the D-09 amd64 coverage proof (coverage-sweep re-key output)
├── root-tree/lib/ndb/local.template   # D-16 token template (no literals)
├── root-tree/usr/glenda/lib/profile   # D-17 trimmed (replaces the 3-line skeleton)
└── (manifest data file consumed by mkimage's services/payload asserts)
build/mm/
├── build-9kdraw.sh              # the draw donor build (k10t1 script shape, CONF=k10draw or variant)
└── boot-9kdraw-trace.sh         # the trace venue harness (banner→script-complete markers)
.planning/phases/15-*/evidence/  # trace raw logs + distilled tables + gate captures (committed)
```

### Pattern 1: Union tier-file consumption in the five mk filters
**What:** The five filter mkfiles (sys/src/mkfile LIBS/CMDS/SUBSYS; cmd/mkfile TARG/DIRS; cmd/ip, cmd/auth, cmd/auth/factotum container mkfiles) each carry dual-arch T1KEEP/T1KEEPA variants today [VERIFIED: sys/src/mkfile:69-82, cmd/mkfile:29-37, cmd/auth/factotum/mkfile:11-19, this session]. Phase 15 extends each amd64 variant's awk to read tier1+tier2+tier3+tier4 files in ONE multi-file awk call (plan9port mk backquote constraints: single-line, brace-free, single-quoted — 06-02 measured).
**Critical leaf-filter ordering:** a `cmd/aux:listen` (or `cmd/aux:mouse`) entry needs a NEW leaf filter in cmd/aux/mkfile (factotum's T1KEEPT pattern) in the SAME commit as the entry — aux is a 40-member mkmany TARG + 6 DIRS (mnihongo flashfs gps na vga realemu) [VERIFIED: sys/src/cmd/aux/mkfile:3-60], and the keep-rule keeps `aux` whole in cmd's DIRS otherwise. On amd64 the apm/pcmcia class compiles natively (no aarch64-era breakage) but the tier discipline still scopes what each tier builds.

### Pattern 2: Tier driver per-entry dispatch + the anti-staleness decision
**What:** subset-build.sh's entry-form dispatch (cmd/<single> link-only `mk 6.<cmd>` + driver cp harvest — the ._cp lesson; cmd/<dir> dir install; container nested; dir:target per-target) extends to tiers 2/3/4 by data append [VERIFIED: build/x86_64/subset-build.sh:170-231]. **The 06-03 open item lands HERE:** tier-2 is the first mass cmd wave since the toolchain fixes; RECOMMENDED = per-entry `mk clean` in the tier driver loop before each `mk install` (deterministic, tier-local, in-driver; donor-vm-build step-4b precedent). The 06-09 plan9port-mk missing-intermediate quirk (delete BOTH the in-tree binary and the installed copy when refreshing one) still governs manual iteration.

### Pattern 3: Services image = mkimage.sh's fifth contract (D-03)
**What:** Clone the tier bake's shape (section 9): committed skeleton tar + chain binaries + the services payload (h*-prefixed binaries computed from the tier-2 data — never a second hardcoded list) + the 14-09 net bring-up (the netip-static fixture's ipconfig flow: bind ether → add → -g default route through /net, all before daemons start) + services-sweep.rc baked at /lib + a services termrc override (tier-termrc's `rc /lib/services-sweep.rc <venue>` + `exec rc -i` shape). Manifest printed at bake; /boot/factotum absence asserted (the same D-02 rule at 9b/8g/9i/10i); KERNDATE determinism; magic checks `529ab12b`/`6b46e688`.

### Pattern 4: The draw donor config (D-04) — the k10f template + the realmode gap
**What:** `sys/src/9k/k10/k10f` is the in-tree template for the draw device set; its deltas vs k10cpu are exactly [VERIFIED: sys/src/9k/k10/k10f:23-28,39-40,65,193-199]:
```
	draw		drawalloc screen vga vgax
	mouse		mouse
	vga
	kbmap
…
vga +dev
	vgavesa
…
	realmode          (in link)
…
 lib
	libmemlayer
	libmemdraw
	libdraw           (+ the k10cpu lib set)
```
The parse grammar maps these mechanically: `dev` section entries become `&<name>devtab` (draw→devdraw.c, mouse→devmouse.c, vga→devvga.c, kbmap→devkbmap.c); extra words on a dev line are bare object names (drawalloc/screen/vga/vgax → port/drawalloc.c, k10/screen.c, k10/vga.c, k10/vgax.c); the separate `vga +dev` section emits `vgadev[]` (vgavesadev from 386/vgavesa.c — reachable because k10's `dir` section lists `386`) and `vgacur[]` with `swcursor`; `link` entries become `<name>link()` calls [VERIFIED: sys/src/9k/mk/parse:294-317,352-371].
**THE GAP:** vgavesa's `vbecall()` opens BOTH `/dev/realmodemem` AND `/dev/realmode` [VERIFIED: sys/src/9k/386/vgavesa.c:59-64 — `cmem = namec("/dev/realmodemem", Aopen, ORDWR, 0);` / `creg = namec("/dev/realmode", Aopen, ORDWR, 0);`]. k10/realmode.c serves only `addarchfile("realmodemem", 0660, rmemread, rmemwrite);` [VERIFIED: sys/src/9k/k10/realmode.c:47-51] — there is no `/dev/realmode` anywhere in 9k, no `realmode0` asm, and no RMUADDR/RMCODE/RMBUF constants in k10/mem.h (legacy: pc/realmode.c:135-139 serves both files; pc/l.s:323 `TEXT realmode0(SB)` is the ~100-line 16-bit trampoline; pc/mem.h:61-63 defines `RMUADDR (KZERO+0x7C00)`, `RMCODE (KZERO+0x8000)`, `RMBUF (KZERO+0x9000)`). vgavesa would COMPILE in a k10f-style config but fail at runtime with Enonexist.
**Three bring-up routes (discretion — sized):**
1. **DISPI + PCI-LFB (RECOMMENDED for the qemu-only venue):** a new small VGAdev for k10 (e.g. `vgadispi.c`) that (a) sets the mode through the qemu std-vga VBE DISPI interface (I/O ports 0x1CE index / 0x1CF data: ID check, XRES, YRES, BPP, ENABLE|LINEAR) — no BIOS, no realmode; (b) maps the LFB through the in-tree `vgalinearpci()` (scans display-class PCI BARs, `ccrb == 3`/Pcibcdisp) [VERIFIED: sys/src/9k/k10/screen.c:440-503] — qemu's std VGA at 00:01.0 exposes a 16MB BAR0. ~100-150 lines of C + a config `vga` section entry. Matches the modern-only theme; qemu-specific (fine — the donor IS a qemu venue by definition).
2. **Port the realmode trampoline:** k10 realmode0 (the 64→32→16→real→back dance; legacy is 32→16 only — the long-mode exit via CR0.PG plus a low identity mapping in the k10 PML4 world is the new work), realmode() C + mem.h constants + serve `/dev/realmode` in k10/realmode.c. Legacy-complete (aux/vga then works too — and aux/vga is a tier-4 member that will compile on amd64), hardest asm. The `*norealmode` conf guard (pc/realmode.c:34) carries.
3. **GRUB gfxpayload + mb1 vbe parse:** GRUB sets a VBE LFB mode pre-kernel; `Mbi.vbe[6]` exists in multiboot.c un-parsed [VERIFIED: sys/src/9k/k10/multiboot.c:24 `u32int vbe[6];` + Fvbe flag defined, no consumer]. Weakest fit: changes the venue boot mechanics off `-kernel`, and mb1 GRUB loading of the 6.out-format kernel is unproven. Not recommended.
**Mode-set choreography:** the venue boots text-mode (qemu -kernel sets no gfx mode); after route 1 or 2's enable, the userspace side drives `/dev/vgactl` (`size 1024x768x32 x8r8g8b8` → `drawinit` — the CMsize/CMdrawinit verbs exist [VERIFIED: sys/src/9k/k10/devvga.c:34-49]); `screensize()` then builds the gscreen (soft-screen malloc path when paddr==0, hard LFB + flush otherwise) [VERIFIED: sys/src/9k/k10/screen.c:49-109].
**Config discipline:** k10t1's header rule ("DO NOT PRUNE: changing the device set risks new breakage") means the draw variant is a NEW config file (k10f-sibling or k10t1-sibling), k10cpu/k10t1 untouched; k10/mkfile's CONFLIST is `k10aoe k10cpu k10root` today [VERIFIED: sys/src/9k/k10/mkfile:3] — CONF= override builds any config (build-9k10t1.sh proves it), so CONFLIST need not change.

### Pattern 5: Font/payload delivery to the draw donor — the -initrd→addbootfile→paqfs channel
**What:** The draw donor needs `/lib/font/bit/lucm/unicode.9.font` (rio's default, rio.c:160-165; acme's lucidasans/euro.8.font + lucm fallback, acme.c:40-43) plus the GUI binaries — 36MB cannot ride the kernel embed. Mechanism: under `qemu -kernel`, `-initrd` registers as an mb1 module; k10's multiboot() already iterates `Fmods` modules and calls `asmmodinit(modstart, modend, string)` [VERIFIED: sys/src/9k/k10/multiboot.c:83-96] — but asmmodinit only RESERVES the module's memory (asm.c:249-263). The delta: in the Fmods branch, also `addbootfile(name, KADDR(mod->modstart), mod->modend - mod->modstart)` (the devroot API the rootdir embed uses, devroot.c:80; Nbootfiles=64 ceiling noted). Then the venue boot script (the k10t1.rc-as-/boot/boot shape) runs the tier-1-built `paqfs` over the exposed module and binds the font/GUI subtrees. This reuses r9's own bootfs.paq mechanism — no srv9, no network, no host channel.
**Trace-scratch:** the donor root is read-only; `mntgen` (a tier-2 member — USPACE-07 proves it mounts) serves the writable /tmp for trace fragments; fragments egress over the proven serial capture at session end (the venue's ONLY channel — no srv9).

### Pattern 6: Synthetic input (D-05) — what 9k actually serves
**Mouse — `/dev/mousein` EXISTS** [VERIFIED: sys/src/9k/port/devmouse.c:93-96, verbatim dirtab rows: `"cursor", {Qcursor}, 0, 0666,` / `"mouse", {Qmouse}, 0, 0666,` / `"mousein", {Qmousein}, 0, 0220,` / `"mousectl", {Qmousectl}, 0, 0220,`]. Its write handler parses text — first char skipped, then four strtols — into `mousetrack(pt.x, pt.y, b, msec)` [VERIFIED: devmouse.c Qmousein case, `msec = strtol(p, &p, 0); if(msec == 0) msec = TK2MS(sys->ticks); mousetrack(pt.x, pt.y, b, msec); return n;`]. Open is `iseve()`-gated (hostowner) — the replay script runs as hostowner, fine. Deterministic in-guest mouse injection works TODAY.
**Keyboard — no kbin on 9k:** devcons's dirtab has cons/consctl/kprint/… but NO kbin/kbd entry [VERIFIED: sys/src/9k/port/devcons.c consdir listing, this session]; `/dev/cons` writes are console OUTPUT. Two routes:
  (a) **serial injection, zero kernel work:** k10's `uartconsole()` registers the uart input queue with `addkbdq(p->iq, -1)` and the output with `addconsdev` [VERIFIED: sys/src/9k/port/devuart.c:140-150] — `/dev/cons` reads see serial input, so a host-side replay writer on the serial chardev injects "keyboard". Determinism is host-anchored (send-after-marker discipline), the rig the expect gates already use.
  (b) **kbin-class devcons write file (~a dozen lines):** a new dirtab entry whose write handler feeds the same rune path the i8042 keyboard uses (`kbdputc(kbdq, c)` — 386/kbd.c:477) — fully in-guest, committed-script determinism (D-05's letter). 9front's devcons kbin is the shape precedent. Small, in-repo, no protocol.
**Console wiring note:** k10 console output fans to BOTH cga (`consputs = cgaconsputs`, main.c:183) and the serial consdev — the venue's markers on serial prove the fanout; serial is the capture truth.

### Pattern 7: The trace rig (05-03 lineage re-keyed to 9k)
**What:** 9k devproc serves `/proc/N/syscall` (`"syscall", {Qsyscall}, 0, 0400`), the `startsyscall` ctl verb (`CMstartsyscall, "startsyscall", 1`), and `trace` [VERIFIED: sys/src/9k/port/devproc.c:98,115,119]. `build/aarch64/tracesup.c` — "step-trace one root process and every born-traced descendant, writing each pid's fragment stream to outdir/<pid>, one newline-framed fragment per line" [VERIFIED: header, this session] — is plain Plan 9 C (`<u.h> <libc.h> <bio.h>`); build it with 6c into the tier set. The arming discipline (read the pending fragment BEFORE the first startsyscall on born-traced children; the startsyscall write blocks) is documented in its header and carries verbatim. The rig asserts script COMPLETION (D-07): the replay script's final action per program emits a marker that lands on serial (e.g. the rio New window's rc echoes a marker to the serial cons — the ancestor's event-loop proof shape).
**Egress + distillation:** fragments land in the mntgen scratch; serialized over the serial capture after the session; the host distills raw logs + per-binary syscall tables (syscall → count → args-shape) — both committed under the phase evidence dir (D-06).

### Pattern 8: Functional-probe semantics per service (D-02/D-08, ancestor-verified, arch-free — sources re-confirmed present)
```rc
# cs: hcs over a test ndb; the canned client answers a scripted lookup
hcs -f /tmp/test.ndb ... ; hcsquery probehost   # cs.c/csquery.c exist [VERIFIED: ls cmd/ndb]
# dns: hdns -f /tmp/test.ndb; hdnsquery resolves a baked entry   (dns.c/dnsquery.c exist)
# exportfs: srv -e 'hexportfs -r /tmp/xroot' xprobe /mnt/xprobe  (srv.c -e exists; exportfs/ dir exists)
#   then cmp a known file through the mount
# listen: hlisten -d /tmp/service tcp &  +  scripted connect via dial/pass  (listen.c, dial/pass.c exist)
# mntgen: hmntgen -s mnttest /tmp/mnt; create + stat a subdir; test -e /srv/mnttest  (mntgen.c exists)
# auth shallow (D-08): factotum ctl round-trip (key proto=... write/read);
#   keyfs -m /mnt/keys /tmp/testkeys over an ABSENT file (serves an empty DB);
#   authsrv via the listener topology (per-connection program on fd 0)
```
**keyfs on amd64 — the readnvram path verified:** readnvram maps cputype `386`/`amd64`/`alpha` → `"pc"` [VERIFIED: sys/src/libauthsrv/readnvram.c:170-173 `if(strcmp(cputype, "386")==0 || strcmp(cputype, "amd64")==0 || strcmp(cputype, "alpha")==0) { free(cputype); cputype = strdup("pc"); }`]; the pc rows all target `#S/sdX0/nvram`/`9fat` and `#f/fd0disk` [nvtab, this session] — the k10t1 device set has NO sd and no floppy, so every open fails and readnvram takes the zero-key+warning path (same landing as aarch64's no-row route, via row-match-but-open-fail). keyfs's `getauthkey` failure is a warning, not an exit; `$nvram` env can point at a file if the probe wants the file path.
**ipconfig probe on the donor:** the same Pitfall-C discipline as the ancestor — the donor's console/root is NOT net-served on this venue (no srv9), so the aarch64 root-bounce hazard does not apply; the r9 services image rides the 14-09 net bring-up where ipconfig configures the slirp leg from netaddr.rc.
**r9-x64 venue truths:** the ipconfig T1-SKIP is STALE — r9's port devcons serves `/dev/random` [VERIFIED: ../r9/port/src/devcons.rs:209 `(b"random", QID_RANDOM, 0o444)`] — it flips live with the tier-2 append. factotum's named gaps (devproc `protect`/`noswap` ctl verbs — absent from ../r9/port/src/devproc.rs, grep empty this session — and devsrv-service mounts in the pipe-9P-only sys_mount era) are D-01 fix-in-phase candidates with the 17-scope cross-check.

### Pattern 9: DESK-03 staging (D-13..D-17)
**Skeleton:** `build/x86_64/root-tree/` today carries 9 files (README, lib/namespace, lib/netaddr.rc, lib/netip-{loopback,static,dhcp,tcp}.rc, rc/bin/termrc, usr/glenda/lib/profile) [re-measured at plan revision: `find -type f | wc -l` = 9; the original "10 files" was a miscount]. D-13 stages into it (or into bake-time copies riding the ancestor's adm/timezone precedent — planner picks).
**Fonts:** `cp -R lib/font/bit` at bake time; manifest asserts: a file-count floor (4303 today — assert an exact count or a floor + the three app-named fonts), presence of `lucm/unicode.9.font` (rio), `lucidasans/euro.8.font` (acme), `pelm/latin1.8.font` + `pelm/euro.9.font` (profile), and a size floor. Measured: 36MB raw (shinonome 13MB, symbola 6MB, lucida 5MB, gb 4MB) [VERIFIED: du -sm this session]. Current x64 paqs: marker 50KB, chain 372KB, tier 1.85MB, net 839KB [VERIFIED: ls -l out/*.paq] — the DESK-03-carrying image grows ~two orders of magnitude, NOT the ancestor's literal 8x (that was aarch64's 4.0MB base); paqfs deflates blocks (mkpaqfs `deflateblock`), bitmap fonts compress hard (~≤15MB expected — ASSUMED until measured; the EARLY bake+boot headroom check is mandatory, r9 legs boot `-m 1024M`, the donor `-m 512`+).
**ndb/local (D-16):** committed template with @TOKEN@ placeholders (home: root-tree/lib/ndb/local.template or beside mkimage — discretion); mkimage sources root-tree/lib/netaddr.rc (rc-syntax key=value: `lo_guestv4`, `slirp_guestv4/maskv4/gwv4`, `blastport` [VERIFIED: root-tree/lib/netaddr.rc:27-44, read this session]) and seds; asserts substituted values present + zero leftover tokens. MUST contain no `10.0.2.` literal — the bake's seam grep (`grep -rn '10\.0\.2\.' "$ROOT_TREE/lib/" | grep -v netaddr.rc`) fails the bake otherwise [VERIFIED: mkimage.sh:488-489].
**Profile (D-17):** trim reference read verbatim [VERIFIED: usr/glenda/lib/profile, 17 lines]. Trims: drop `upas/fs` (line 7) and `news` (line 28); terminal case keeps lines 1-6 (binds/tmp/font=`pelm/euro.9.font`) but drops `exec rio -i riostart` (line 16) — rio must provably not start; cpu case intact (lines 17-32: /mnt/term binds, `font=/lib/font/bit/pelm/latin1.8.font`, `exec rio`). Note the terminal case's `'#m/mousectl'` writes (lines 12-13) and `plumber` (line 11): on a serial-console image without devmouse these fail loudly enough to matter — the trimmed shape (drop or keep guarded) is planner discretion within D-17's letter; `syscall` (line 4) is cmd/syscall/ (exists [VERIFIED]) and `ramfs` (line 5) is cmd/ramfs.c (exists).
**Manifest (D-15):** a committed plain-data manifest file consumed by mkimage (tier-allowlist pattern); bake fails on any missing/empty listed path; no second hardcoded list — the services-cargo and payload manifests follow the printed-manifest shape at 8f/9h/10h.

### Anti-Patterns to Avoid
- **Trusting mk exit status** — namespace builds lie (02-12); artifact + execution markers only.
- **Typed console interaction on the 9k donor** — devcons /dev/cons reads come from kbdq; the sweep is sourced from the boot flow (k10t1.rc shape), never typed.
- **Hardcoding venue addresses** — netaddr.rc is the single seam; the bake greps root-tree/lib mechanically.
- **Settle-only GUI traces** — D-05/D-07: the event loop + sustained draw traffic ARE the measurement; the gate asserts script COMPLETION.
- **A k10f-copied config assuming vesa works** — /dev/realmode does not exist on k10; vbecall errors Enonexist (Pattern 4).
- **Embedding fonts in the kernel rootdir** — 36MB vs a ~16MB-class kernel; the -initrd module channel is the route.
- **Silent exclusion drops** — every re-derived exclusion is a named, committed comment (D-10); fossil/venti carry the documented-runtime-unverifiable note.
- **Assuming the aarch64 harness channels** — no srv9, no guestfwd, no 9front tree on the x64 venues; serial is the only donor egress.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Probe client programs | new C dialers/query tools | `ndb/csquery`, `ndb/dnsquery`, `dial/pass`, `dial/expect`, `srv -e` [VERIFIED: all present in cmd/] | in-tree, tier-2 members themselves |
| Guest-side 9P serving for probes | pipe-pair plumbing | `srv -e 'hexportfs -r …'` (execs+posts+mounts); `mntgen -s` | the ancestor's proven probe topology |
| Trace supervisor | a new tracer | `build/aarch64/tracesup.c` re-keyed to 6c | born-traced arming + teardown discipline proven 06-07 |
| Writable scratch on the donor | a new ramfs service | `mntgen` (tier-2 member, D-02 proves it mounts) | the probe and the infrastructure are the same binary |
| LFB mapping | BAR parsing from scratch | k10 `vgalinearpci()` (display-class BAR scan) | in-tree, handles the heuristic set |
| paq image baking | new archiver | `.build/bin/mkpaqfs` via mkpaqfs-host.sh | byte-format owner (04-01/14-05) |
| Keyfile crafting for keyfs | a keyfile generator | keyfs over an ABSENT test file (serves empty DB; write-back on create) | zero new code; exercises the write path |
| Replayed mouse events | writing mouse-Qs from the rig | `/dev/mousein` text writes (`M x y b msec`) | exists on 9k today; same mousetrack entry as PS/2 |

**Key insight:** every probe, channel, and rig component has a compiled-green in-tree answer except three deliberate builds: the vga bring-up (route choice), the kbin-class file (optional), and the addbootfile module exposure (~3 lines) — all sized above.

## Common Pitfalls

### Pitfall 1: k10f compiles but vesa cannot run — the /dev/realmode hole
**What goes wrong:** a draw config copying k10f's `vga/vgavesa` builds green, then `aux/vga`-less mode discovery fails at runtime: vgavesa's vbecall opens `/dev/realmode` which no k10 kernel serves — Enonexist, draw bring-up dead.
**Why:** the realmode trampoline (realmode0 + RMUADDR/RMCODE/RMBUF + realmode()) was never ported to k10; 9k/realmode.c is only the memory window.
**How to avoid:** pick the bring-up route FIRST (DISPI+PCI-LFB recommended; realmode port as the legacy-complete fallback), size it as its own task, gate on `/dev/draw/new` opening — never on compile.
**Warning signs:** "vesa bios error" / Enonexist on the serial capture at first drawinit.

### Pitfall 2: 36MB of fonts meets the venue
**What goes wrong:** the DESK-03-carrying image bake balloons (chain-scale 372KB → tens of MB) and either the donor (-m 512) or the r9 leg (-m 1024M) runs out of memory at bootfs inflate, or the bake/CI artifacts churn git.
**Why:** bitmap fonts are zero-heavy and deflate well (expected), but nobody has measured the x64 image; the ancestor's "8x" was a different base.
**How to avoid:** the EARLY DESK-03 task bakes + boots the image and records sizes; paq artifacts stay gitignored (out/); the manifest carries the count/size floors.
**Warning signs:** paqfs inflate panics; r9 boot stalls post-banner.

### Pitfall 3: aux leaf filter ordering (the Phase-6 class, re-keyed)
**What goes wrong:** a `cmd/aux:listen` tier entry without a leaf filter in cmd/aux/mkfile keeps `aux` whole in cmd's DIRS — the default amd64 build sweeps aux's 40 TARG members + 6 DIRS.
**Why:** the keep-rule matches the first path component; aux is a mkmany+DIRS container.
**How to avoid:** the leaf filter lands in the SAME commit as the first aux entry (factotum's T1KEEPT pattern); same check for any other container entry (ndb whole-dir is fine — all members are tier-2).
**Warning signs:** the default build suddenly compiling apm/pcmia/realemu.

### Pitfall 4: the ipconfig SKIP staleness and the SKIP-whitelist converse
**What goes wrong:** the r9 gate keeps asserting `T1-SKIP ipconfig` after the tier-2 append makes it live — the converse assert reds ("an emitted/expected SKIP mismatch") — or worse, the sweep keeps the SKIP and an unearned SKIP masks a real /dev/random regression.
**Why:** 14-09 measured the stale reason but the flip is Phase-15 work.
**How to avoid:** the tier-2 append commit updates the sweep's SKIP block AND the gate's computed expectations in one commit; every new SKIP is named with its venue reason.

### Pitfall 5: TCG/timing + daemons + traces
**What goes wrong:** probes with settle-free sequencing wedge under TCG; trace sessions outrun budgets.
**How to avoid:** presence-based marker assertions, per-probe internal timeouts with named FAIL markers (never a wedge), TCG-generous budgets (the venue defaults: 1500s sweeps), KVM preferred (writable on the primary host).

### Pitfall 6: stale tier objects (the 06-03/06-09 class) at the first mass wave
**What goes wrong:** mk's freshness graph never names the toolchain; tier-2/3/4 `mk install` relinks nothing after toolchain or object staleness — stale binaries get markers.
**How to avoid:** the anti-staleness decision lands in the FIRST tier-2 plan (per-entry `mk clean` recommended); the 06-09 delete-both rule for manual refresh.
**Warning signs:** artifact sha256s unchanged after source edits.

### Pitfall 7: latent 6l D_FCONST bug wakes on the GUI tier
**What goes wrong:** the deferred 06-03 finding — 6l's `asm.c` D_FCONST case-8 emits double constants through a 16-byte-LP64 Ieee cast — has no amd64 consumer today; GUI code (draw/memdraw float data) could be the first.
**How to avoid:** if tier-3 links fail or float-armed GUI binaries misbehave, check the 7l fix pattern (commit 2bbca81ed, emit two 32-bit halves) before suspecting the sources.
**Warning signs:** weird float constants in GUI binaries; the float-d01 canary still green but GUI text metrics wrong.

### Pitfall 8: libmemdraw pixel-path faults poison the traces
**What goes wrong:** drawtest aborts on stochastic pixel verification and arctest takes a wild-pointer write (both aarch64-measured, arch-independent C bugs); the GUI traces record garbage pixels and the distilled tables lie about correctness.
**How to avoid:** the inherited checklist item runs BEFORE/alongside trace capture (budget the audit as its own task — D-14's watch-out); both tests rejoin the sweep when green.
**Warning signs:** trace-session panics in memsetl/memimagedraw.

### Pitfall 9: man-page references that don't exist
**What goes wrong:** plans citing `sys/man/8/{cs,dns,authsrv,exportfs}` — those pages are NOT in Harvey's tree (only `listen(8)` and `keyfs(4)` exist among the named set) [VERIFIED: ls sys/man/8, this session].
**How to avoid:** probe semantics come from the sources (the ancestor read them; cs.c/dns.c/authsrv.c/keyfs.c/exportfs.c are the authority).

## Code Examples

### The draw-donor config delta (k10f's, over k10cpu) — the shape the new variant copies
```
# Source: [VERIFIED: sys/src/9k/k10/k10f:23-28,39-40,65] — dev/link/lib sections only
dev +dev
	...
	draw		drawalloc screen vga vgax
	mouse		mouse
	vga
	kbmap

vga +dev
	vgavesa

link +dev
	...
	realmode

lib
	libmemlayer
	libmemdraw
	libdraw
	libip
	libsec
	libmp
	libc
```
(With the DISPI route, `vgavesa`/`realmode` are replaced by the new dev; with the realmode route they are kept and the trampoline ported.)

### The addbootfile module exposure (the ~3-line multiboot delta — shape)
```c
/* Source shape: [VERIFIED: sys/src/9k/k10/multiboot.c:83-96] — the Fmods loop today */
	if(mbi->flags & Fmods){
		for(i = 0; i < mbi->modscount; i++){
			mod = KADDR(mbi->modsaddr + i*16);
			...
			else
				asmmodinit(mod->modstart, mod->modend, p);
		}
	}
/* the delta: also expose the module bytes through devroot (Nbootfiles ceiling 64,
   devroot.c:14) so the in-guest paqfs can mount them — qemu -initrd rides this path */
```

### The replay script's input vocabulary (mousein format — verbatim semantics)
```rc
# mouse: /dev/mousein takes text  M x y buttons msec  (first char skipped;
# msec 0 → kernel timestamp) [VERIFIED: sys/src/9k/port/devmouse.c Qmousein case]
echo 'M 512 384 0 0' > '#m/mousein'
echo 'M 600 400 1 0' > '#m/mousein'	# button-1 press at 600,400
# keyboard (kbin-class file, to be added): runes → the kbdputc(kbdq, c) path
# (the same path 386/kbd.c:477 feeds); serial chardev injection is the
# zero-kernel-work alternative — /dev/cons reads see serial via addkbdq
```

### The tier-2 filter extension (one awk, four files — the cmd/mkfile amd64 variant)
```mk
# Source shape: [VERIFIED: sys/src/cmd/mkfile:33-37] — extended per Pattern 1
TIER1A=$HARVEY/build/x86_64/tiers/tier1-cmds.txt
T2=$HARVEY/build/x86_64/tiers/tier2-services.txt
T3=$HARVEY/build/x86_64/tiers/tier3-gui.txt
T4=$HARVEY/build/x86_64/tiers/tier4-tail.txt
T1KEEPA=`{awk '/^cmd\//{ sub(/^cmd\//,""); sub(/[\/: 	].*$/,""); print }' $TIER1A $T2 $T3 $T4}
TARG=`{if(~ $objtype amd64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLTARG) if(~ $i $T1KEEPA) echo $i; if not echo $ALLTARG}
```

### ndb/local template + bake-time substitution (D-16)
```sh
# Source: mkimage.sh's net-contract discipline (10b's seam grep at :488-489);
# netaddr.rc keys [VERIFIED: root-tree/lib/netaddr.rc:37-39]
#   slirp_guestv4=10.0.2.15 / slirp_maskv4=255.255.255.0 / slirp_gwv4=10.0.2.2
# template tokens (no literals — the grep must stay clean):
#   ip=@GUESTV4@ ipmask=@MASKV4@ ipgw=@GWV4@
sed -e "s/@GUESTV4@/$slirp_guestv4/g" -e "s/@MASKV4@/$slirp_maskv4/g" \
    -e "s/@GWV4@/$slirp_gwv4/g" "$template" > "$STAGE/lib/ndb/local"
grep -q '@' "$STAGE/lib/ndb/local" && err "ndb/local carries an unsubstituted token"
```

## State of the Art

| Old Approach (v2.0 P10, aarch64) | Current Approach (this phase, x64) | Impact |
|---|---|---|
| 9front arm64 donor + srv9/guestfwd channels | k10t1 (Harvey's own 9k kernel), no host channels | serial is the only egress; -initrd module channel replaces srv9 for payload |
| draw-over-9P host draw server (drawterm-derived) | kernel-local devdraw on a draw donor config | the client-side syscall mix is the measurement; topology re-capture deferred (D-04) |
| gs blocked by no-aarch64-ape (vendored arm64 slice) | ape/lib/ap/amd64 + default.amd64.h in-tree — stock pcc route | gs + the pcc class compile natively; render smoke is pure upside |
| tier1-cmds single-file filters | dual-arch T1KEEP/T1KEEPA at five mkfile sites | tier2/3/4 extend the amd64 variants in one awk over N files |
| aarch64 tier1.ledger | build/x86_64/tier1.ledger `tier=1 dir=… bin=… sha256=… exec=T1-OK:r9x64 evidence=… commit=… date=…` [VERIFIED: first lines, this session] | tier2/3/4 ledgers extend the grammar with venue names k10t1/r9x64 |

**Deprecated/outdated:** typed in-guest sweeps (sourced-from-termrc everywhere); the aarch64 harness's srv9 writable-prefix assumptions; carrying aarch64 exclusion classes unexamined.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | qemu's std-VGA DISPI interface (ports 0x1CE/0x1CF) is reachable and sufficient for mode-set on both pc and q35 machines under qemu 11.1.2 | Pattern 4 route 1 | the DISPI route dies; fall back to the realmode port (route 2) — verified only against training knowledge of qemu's VBE implementation, not probed this session |
| A2 | qemu `-initrd` registers as an mb1 module when `-kernel` loads the 9k kernel (qemu's mb loader path) | Pattern 5 | the font channel needs a different carrier (e.g. a GRUB ISO for the draw venue only) — the -kernel/-initrd mb1 pairing is qemu-documented behavior, not run this session |
| A3 | paqfs deflate compresses the 36MB font tree to ~≤15MB | Pitfall 2 / DESK-03 | bigger initrd; the EARLY headroom check catches it; worst case raw ~40MB still fits -m 1024M |
| A4 | the pcc/APE class (gs 753 files, compress, eqn, grap, lp, pic, postscript, spin, troff, mp3enc) compiles on amd64 with in-tree ape/lib/ap/amd64 | USPACE-09 | individual fix-in-tree per D-10; the class was fail-on-aarch64 for the MISSING-lib reason only (no per-file audit done this session) |
| A5 | tracesup.c compiles under 6c unchanged (it was built by 7c for aarch64) | Pattern 7 | trivial port if any 7c-isms appear |
| A6 | the donor services embed (~20 extra binaries + sweep) fits devroot's Nbootfiles=64 ceiling | Pattern 3/5 | raise the ceiling again (the 32→64 precedent, devroot.c:14) or ride the -initrd channel on the donor too |
| A7 | r9's devmnt/sys_mount can be grown (or already works) for srv-mounted services (exportfs/mntgen/factotum mounts) within D-01's fix-everything budget | USPACE-07 r9 leg | probes degrade to named SKIPs — a D-02 violation requiring user-visible disposition, not silent |

## Open Questions (RESOLVED at plan time — every question's resolution is adopted by an executable plan task)

1. **The VGA bring-up route choice (D-04 discretion, sized but unbuilt)**
   - What we know: three routes; DISPI+PCI-LFB is smallest and qemu-native; the realmode port is legacy-complete (also unlocks aux/vga runtime); GRUB is the weakest fit.
   - What's unclear: whether the user/planner wants qemu-only device code in the k10 tree (route 1) vs the legacy-faithful trampoline (route 2). D-04's reversibility note (the rig keys off the venue) makes this the phase's most consequential discretionary pick.
   - Recommendation: route 1 (DISPI+PCI-LFB) as primary — modern-only theme, qemu-only venue, ~100-150 lines; keep route 2 as the documented fallback if DISPI misbehaves under the exact qemu pin.
   - Resolution (RESOLVED): route 1 adopted as primary with route 2 as the in-plan documented fallback — adopted by 15-05 Task 1 (the A1 pivot path); the outcome is recorded for the evidence bundle by 15-08 Task 3.
2. **Keyboard replay determinism level (D-05)**
   - What we know: mousein is in-guest deterministic; keyboard is either serial-anchored (host) or a new kbin-class file (in-guest).
   - What's unclear: whether "determinism rides the replay mechanism" (D-05) requires the in-guest kbin route or the proven marker-anchored serial sends suffice.
   - Recommendation: add the kbin-class file (a dozen lines) so the whole replay script is committed in-guest data; serial injection remains the bring-up bootstrap.
   - Resolution (RESOLVED): the in-guest kbin-class file adopted — adopted by 15-05 Task 3 (the eve-gated kbin file + the OQ-2 determinism record) and consumed by 15-06's committed replay scripts; recorded in the bundle by 15-08 Task 3.
3. **r9 services-leg kernel-gap sizing (D-01)**
   - What we know: the named gaps (devproc protect/noswap, devsrv-service mounts) plus whatever the daemons surface (factotum's fs, keyfs's mount, dns's timers).
   - What's unclear: total size — this is inherently flush-out work; the phase's point.
   - Recommendation: budget it as named fix tasks per surfaced gap with push-before-pin discipline; the 17-scope cross-check happens at plan time (Phase 17 owns the CR/WR backlog, not these).
   - Resolution (RESOLVED): budgeted as named per-gap fix tasks under push-before-pin — adopted by 15-03 Task 3 (the D-01 fix loop with the 17-scope cross-check); sizing stays flush-out by design, and every surfaced gap lands as its own ../r9 commit.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| linux x86_64 host | namespace builds, venues | ✓ (this machine) | kernel 7.3.1 x86_64 | — |
| KVM (/dev/kvm writable) | fast donor/r9 legs | ✓ (probed this session) | — | TCG with generous budgets (venue scripts auto-probe) |
| qemu-system-x86_64 | all guest legs | ✓ | 11.1.2 (probed) | — |
| nix + devshell | toolchain bootstrap, gate apps | ✓ (probed) | — | — |
| passwordless sudo | RUN_BUILD_IN_PRIVATE_NAMESPACE venue | ✗ (probed: no) | — | the unprivileged userns chroot scaffold venue (14-08-proven, build-9k10t1.sh carries both) |
| `.build/bin/mkpaqfs` | mkimage bakes | ✓ (present) | — | mkpaqfs-host.sh rebuilds it |
| `.build/9k10t1-tier-manifest.sha256` + `amd64/9k10t1.elf` | witness manifest + venue kernel | ✓ (present) | — | rebuild via build/mm scripts |
| ../r9 checkout | D-01 fixes | ✓ (readable, at e560405) | — | — |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** passwordless sudo (userns venue proven).

## Security Domain

Scoped — this phase adds no new externally-exposed surface; boundaries that exist:

| Boundary | Threat | Mitigation |
|----------|--------|------------|
| Test key material (keyfs/factotum probes) | test keys mistaken for real credentials | absent/throwaway test files only (`dom=probe`-style values); zero real secrets baked; DEMO labeling is Phase 18's but fixtures stay clearly-test |
| /dev/mousein + kbin-class writes | unprivileged input injection into sessions | mousein is `iseve()`-gated today (devmouse.c); the kbin-class addition must take the same eve-gate — carry it into the pattern |
| Baked images | payload tambling/drift | manifest asserts + KERNDATE determinism + sha256 artifact markers; paq artifacts gitignored (built, not committed) |
| gs fixtures | unexpected file access in-guest | fixed committed .ps fixture, bitmap FILE device, no network on the donor |

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | p9sk1 is Phase 18; D-08 probes are start/respond only |
| V4 Access Control | marginal | eve-gating on synthetic-input files (above) |
| V5 Input Validation | no | fixed committed fixtures; no host-controlled parser input |
| V6 Cryptography | no | libsec compiled, not modified |

## Sources

### Primary (HIGH confidence — read in-repo this session)
- Configs + parse: `sys/src/9k/k10/{k10cpu,k10t1,k10f,k10cpuf,k10root}`, `sys/src/9k/mk/parse`, `sys/src/9k/k10/mkfile`
- Draw substrate: `sys/src/9k/port/{devdraw.c,devmouse.c,devcons.c,devkbmap.c,drawalloc.c,devproc.c,devuart.c,devroot.c}`, `sys/src/9k/k10/{devvga.c,screen.c,vga.c,mouse.c,realmode.c,multiboot.c,asm.c,cga.c,main.c,mem.h}`, `sys/src/9k/386/{vgavesa.c,kbd.c}`, legacy `sys/src/9/pc/{realmode.c,l.s(via grep),mem.h(via grep)}`
- x64 machinery: `build/x86_64/{mkimage.sh,subset-build.sh,tier1-sweep.rc,tier1-r9.expect(listed),tier-termrc,tier1.ledger,tiers/tier0-libs.txt,tiers/tier1-cmds.txt,root-tree/*}`, `build/mm/{build-9k10t1.sh,boot-9k10t1-sweep.sh}`, `nix/tier1-r9-x64.sh`
- Sources of the tiers: `sys/src/cmd/{aux/mkfile,ndb/,dial/,gs/mkfile,rio/mkfile,acme/mkfile,samterm/mkfile,games/mkfile,mp3enc mkfile(via grep)}`, `sys/src/ape/lib/ap/amd64/`, `sys/src/libauthsrv/readnvram.c`, `usr/glenda/lib/profile`, `lib/font/bit/` (measured)
- Ancestor + machinery docs: `.planning/milestones/v2.0-phases/10-*/10-RESEARCH.md`, `.planning/milestones/v2.0-phases/06-*/deferred-items.md`, `build/aarch64/{dir-coverage.txt,tracesup.c,coverage-sweep.sh}`, `.planning/phases/14-*/14-CONTEXT.md`, `.planning/ROADMAP.md` §15, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`
- r9 sibling: `../r9/port/src/{devproc.rs,devcons.rs}` (at e560405)

### Secondary (MEDIUM confidence)
- rio/acme/sam startup semantics and probe internals carried from the ancestor research (verified against sources in ITS session; file paths re-confirmed present this session)

### Tertiary (LOW confidence)
- qemu DISPI/-initrd-mb1 behavior details (A1/A2) — training knowledge, flagged as assumptions

## Metadata

**Confidence breakdown:**
- Tier machinery + services image + payload staging: HIGH — every mechanism read in-tree this session; the fifth contract and payload are clones of proven shapes
- Draw donor sizing: MEDIUM — the device-set template, the realmode gap, the input surface, and the font channel are all verified; the three bring-up routes are sized from in-tree code but none is built (A1/A2 assumptions)
- r9 fix-everything inventory: MEDIUM — the named gaps are committed evidence; sizes inherently unknowable pre-sweep
- Coverage re-derivation: HIGH for the inventory structure (aarch64 rows + amd64 ape presence), MEDIUM for per-dir amd64 outcomes until the sweep runs

**Research date:** 2026-11-09
**Valid until:** 2026-12-09 (in-repo verified against commit 7e542e5; re-verify if the Phase-14 machinery, the r9 pin, or the 9k tree moves — the graphify graph is 86 commits stale and was not relied upon)
