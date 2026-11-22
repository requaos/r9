# Phase 10: Userspace Services + GUI Tiers + Bootfs Payload - Research

**Researched:** 2026-11-05
**Domain:** Plan 9 userspace tier sweep (services/GUI/long tail) on the 9front donor kernel + bootfs payload staging
**Confidence:** HIGH for tier machinery/probes/gs-wall/devdraw-facts (all verified in-repo this session); MEDIUM for the GUI-trace server route (proven code identified, unbuilt)

## Summary

Phase 10 rides machinery that is entirely proven: the D-13/D-14 tier data files + mk filters + append-only ledgers (06-02), the sourced in-guest sweep + staged termrc override (06-03, subset-exec.sh), the tracesup born-traced supervisor (`harness trace-syscalls --tier1`, 06-07), the srv9/guestfwd host-9P channel (Phase 2), and the mkimage.sh manifest-assertion bake (Phase 5). The 06-04 committed dir-coverage report already proves the whole tier-2/3/4 membership — minus the ape-linked class — **compiles for aarch64 today** (`build=ok`: auth, ip/*, ndb, aux, exportfs, mntgen's cmd file, rio, acme, sam, samterm, plumb, page, proof, upas/*, fossil, venti; `build=fail` only in the pcc/APE class + games/mp3enc + gs). Phase 10's real work is probes, markers, ledgers, the GUI trace rig, and the bootfs payload — not a compile-fix slog.

Two locked-decision premises fail verification and are flagged as Open Questions for the planner to escalate: **(1) D-09's premise is factually wrong** — plan9port's `devdraw` does NOT serve 9P; it speaks the drawfcall/Wsysmsg protocol over a pipe (its own man page: "Devdraw should probably present a standard 9P server instead of using its own protocol"). The proven 9P-served draw device tree lives in **drawterm** (`kern/devdraw.c` + `kern/devmouse.c` + `kern/devkbd.c` + `kern/devcons.c`, `gui-cocoa` on darwin) — the exact code Phase 12's drawterm ships. **(2) gs is an APE program**: its mkfile sets `CC=pcc LD=pcc`, compiling 753 files against `/sys/include/ape` and linking `/aarch64/lib/ape/libap.a` — and ape has no aarch64 port (milestone exclusion; `sys/src/ape/lib/ap/` carries 386/amd64/arm/... but no arm64/aarch64 dir). The stock gs build is therefore structurally unbuildable for aarch64; D-01's fix-or-exempt valve will hit this wall first, and if exempted, D-02's render smoke dies with it.

**Primary recommendation:** extend the tier machinery by union (filters read all four tier files), run tier-2 as the first mass cmd wave with the anti-staleness decision landed (per-entry `mk clean` in the tier drivers), vendor the donor's `default.arm64.h` as `default.aarch64.h` immediately (cheap, genarch-produced on real 9front arm64, unblocks every gs route), build the GUI trace as `harness trace-syscalls --gui` on the 06-07 tracesup spine against a drawterm-derived serve-only host draw with server-side scripted-input replay, and stage DESK-03 via mkimage.sh bake-time copies + netaddr.rc-sourced template substitution with manifest assertions.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** gs is INCLUDED in the tier-4 long-tail sweep as a compile-gated member, fix-or-exempt. Fixes land one commit per bug class; anything exceeding the fix budget is excluded by name with a committed comment (Phase 6 D-08 pattern). gs's mkfile needs `$objtype.h` — either `genarch` run on a matching CPU or a committed `default.$objtype.h` (no `default.aarch64.h` exists — it must be authored or harvested); genarch is pcc-native-guarded (06-04 G6), Pitfall-2 pre-cleared.
- **D-02:** gs gets a headless donor render smoke beyond its compile marker: renders a tiny checked-in `.ps` fixture to a bitmap *file* device (no display) on the donor; gs's `.ps` init/lib files staged into the guest (srv9 writable-prefix precedent). lp's gs-dependent stages stay excluded; gs's GPL fonts are never staged anywhere.
- **D-03:** Named services get FUNCTIONAL donor probes: `ndb/cs` answers a scripted lookup; `ndb/dns` resolves through cs; `exportfs` serves a guest-side 9P mount; `aux/listen` accepts a scripted connection; `mntgen` mounts; `ipconfig` configures the donor's ether and `ping` reaches the host-side srv9 leg. Phase 6 sourced-not-typed in-guest rc sweep with per-service OK/FAIL serial markers.
- **D-04:** Phase 10 execution gates are DONOR-ONLY. r9 consumes these binaries later (Phase 11 auth, Phase 12 full chain). No coupling to in-flight Phase 8. Ledgers name `venue:donor` explicitly.
- **D-05:** Auth suite compiles complete (~80 files) plus a SHALLOW execution subset on the donor: factotum starts and answers a scripted probe, keyfs serves over a test key file, authsrv starts and listens. Real p9sk1 handshakes, `/adm/keys`, ndb domain content, and the loopback probe are Phase 11's.
- **D-06:** The whole `sys/src/cmd/ip/` tree joins tier-2 as compile-gated members, with the roadmap-named core (ipconfig, ping) getting functional probes; x86-hardware-only or r9-subsystem-less dirs excluded by name with comments.
- **D-07:** Tier-4 long tail is compile-gated: upas + lp + games + remaining cmd dirs; fossil+venti compile-gated AND documented runtime-unverifiable this milestone (no block device in r9); games with per-arch asm and other unmigratable dirs excluded by name with committed comments. upas build-only; lp build-only with gs-dependent stages excluded.
- **D-08:** rio/acme/sam traces captured against a HOST-SERVED draw mounted over 9P into the donor guest (chosen over donor kernel softscreen and over drawterm-into-donor). Verified: the donor arm64 qemu kernel config has no draw/mouse/screen device; draw-over-9P is exactly the syscall mix r9 must grow for (Phase 12 topology). Donor kernel stays pristine; the existing srv9/guestfwd channel is the transport.
- **D-09:** plan9port's `devdraw` serves `/dev/draw` on the host (chosen over a purpose-built Rust fake-draw server in `build/harness/`, and over devdraw-primary/fake-draw-fallback). plan9port is already a host dependency; devdraw serves the full draw file tree (new/ctl/data/mouse/cursor/snarf/kbd) proven against real Plan 9 libdraw clients; zero new protocol code. Runs on cocoa on the darwin dev machine. — **⚠ RESEARCH FINDING: the premise "devdraw serves the full draw file tree over 9P" is false; see Open Question 2.**
- **D-10:** Traces are SCRIPTED-INTERACTION, not launch-settle-exit: a fixed short script per program drives real interaction via input injection (rio: menu New, type into the new rc window; acme: type text, execute a command, quit; sam+samterm: type, edit, quit). The event loop, mouse/kbd read loops, snarf, and sustained draw traffic ARE the measurement.
- **D-11:** The GUI trace leg is darwin documented-manual ONLY (deliberate deviation from Phase 6 D-16's everything-in-CI default, Phase 8 D-16 soak-override precedent). CI carries tier-2/3/4 compile gates and tier-2 donor execution gates (linux legs per convention) but never the GUI trace.
- **D-12:** plumber gets a FUNCTIONAL headless donor probe (starts, a scripted plumb message routes to a registered port, clean shutdown). The rest of tier-3 (rio, acme, sam, samterm, stats, winwatch, page, proof, screenlock) is compile-gated; rio/acme/sam+samterm additionally get the D-08 trace.
- **D-13:** The FULL `lib/font/bit` tree stages into bootfs — all 36MB, no exclusions. Current bootfs.paq is 4.0MB; the initrd grows roughly 8× (paqfs compresses; r9 boots `-m ≤ 1G`). gs's GPL fonts untouched. Reversible.
- **D-14:** `/usr/glenda/lib/profile` stages as the FULL cpu-aware glenda reference, trimmed: `upas/fs` and `news` dropped, the `case terminal` path keeps its binds/tmp/font but loses the `exec rio` (serial console at boot — rio must provably NOT start at boot), the `case cpu` path intact (binds `/mnt/term/dev`, `font=/lib/font/bit/pelm/latin1.8.font`, `exec rio` when called from drawterm). Phase 12 consumes the profile unchanged.
- **D-15:** `/lib/ndb/local` is a committed TEMPLATE with placeholder tokens, substituted at bake time from `build/aarch64/netaddr.rc` (the Phase 8 D-11 addressing seam stays the single source); the mkimage manifest asserts the substituted result.
- **Phase-6-filed checklist items owned by this phase:** libmemdraw drawtest/arctest LP64 pixel-path audit (both tests rejoin the standing sweep when green); stale liboventi/libmemdraw test targets (remove or restore); libdynld dynld.c:128 ULONG<0 note; rc argv-stack residual audit (the 06-07 Xbackq fix's residual class DID eat markers in 06-09 r9 parity runs); tier cmd anti-staleness decision (decide by the first mass tier wave — tier-2 IS that wave).
- **Carried forward:** append-only per-tier committed data files + per-tier append-only marker ledgers as the only gates; mk exit status never trusted; sourced-not-typed sweeps; venue honesty (`venue:donor`); execution-gated ordering; donor-first + measure-first; bake-into-bootfs for gates, ufs-over-tcp for iteration; the mkimage real-init rodata guard; CI = phase6.yml-shape linux legs, darwin documented-manual, 40-char-SHA action pins; exclusions by name with committed comments; Pitfall-3 findings filed as per-dir checklists, one commit per class; `build/aarch64/out/` last-writer-wins; `aarch64` naming; Plan 9 C style; NMOUNT=54 divergence ledger grows; evidence in `.planning/phases/10-*/` with SHAs; markers timing-tolerant.

### the agent's Discretion
- Per-tier membership order and wave decomposition within tiers 2/3/4; functional-probe script internals (fixed inputs, marker vocabulary, port numbers); `default.aarch64.h` authoring route; GUI trace rig mechanics (mount path, input-injection mechanism, trace format/commit layout); gs render-smoke fixture + bitmap-device choice; physical layout/naming in `tiers/` + `markers/`; ndb/local template home + token syntax (D-15's seam-substitution locked); bootfs staging mechanics (committed staged tree vs bake-time copy); CI job topology (D-11's GUI exception noted).

### Deferred Ideas (OUT OF SCOPE)
- r9 execution of tier-2 services; real p9sk1, `/adm/keys` generator, keyfs-as-real-service, ndb domain content, secstore (Phase 11); `#!` exec, `service=cpu` bootconf, aux/listen live at boot, drawterm flake, rio in-session (Phase 12); fossil/venti runtime; upas send/receive UAT; lp gs-dependent stages; linux-CI Xvfb GUI leg; Rust harness fake-draw; virtio-* extras; darwin host sbrk 4096KB cap.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| USPACE-07 | Session daemons + services compile and donor-execute: aux/listen, mntgen, ndb/cs+dns, exportfs, ip/ suite (ipconfig, ping), auth/ suite (~80 files) | All compile `build=ok` today [VERIFIED: build/aarch64/dir-coverage.txt]. Probe semantics per service nailed down below (man pages + sources read). Tier machinery extension shape below. |
| USPACE-08 | GUI stack compiles (libdraw family + rio + acme + sam + plumb); donor-kernel syscall traces for rio/acme/sam captured measure-first | libdraw family all tier-0 green (06-05). rio/acme/sam/samterm compile `build=ok` today. Startup device/font requirements traced in source (below). Trace rig = 06-07 tracesup spine; host draw server route in Open Question 2. |
| USPACE-09 | Long tail compiles: upas + lp + remaining cmd dirs; fossil + venti compile-gated + documented; gs included (753 files) | Coverage report gives the full inventory + the pcc/APE failing class verbatim (below). gs's structural wall documented (Open Question 1). fossil/venti `build=ok` compile-green. |
| DESK-03 | `/lib/font/bit/*` + glenda-style lib/profile + `/lib/ndb/local` template staged in bootfs with manifest assertions | mkimage.sh staging + manifest machinery mapped (below); 36MB fonts measured; mkpaqfs deflate confirmed; profile trim reference read verbatim; ndb(6) tuple format confirmed. |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Tier-2/3/4 compilation (7c/7l over the allowlists) | Host build (linux namespace: lima guest on darwin, native on linux) | — | Plan 9 namespace build; mk filters read the tier files $HARVEY-absolute |
| Tier-2 functional probes (cs/dns/exportfs/listen/mntgen/ipconfig/ping/auth-shallow/plumber) | 9front donor guest (qemu TCG, ethervirtio10 leg) | Host driver (subset-exec.sh shape) | D-03/D-04/D-05/D-12: donor-only execution; probes sourced not typed |
| Draw serving for GUI traces | Host (darwin, cocoa) | — | D-08/D-09: host-served 9P draw tree; guest mounts it |
| Syscall tracing | Donor guest (startsyscall + /proc/N/syscall, tracesup supervisor) | Host parse/artifact (trace.rs) | Measure-first; per-pid fragment files land on the srv9 writable prefix |
| Bootfs payload staging (DESK-03) | Host bake (mkimage.sh + mkpaqfs-host.sh) | r9 consumption Phase 11/12 | Bake-time copy + template substitution + manifest asserts |
| CI legs (compile + tier-2 donor gates) | GitHub ubuntu-latest (TCG) | darwin documented-manual (GUI trace) | D-11; phase6.yml shape |

## Standard Stack

**No new external packages.** This phase consumes only in-tree code and already-provisioned host tooling. The "stack" is the Phase-2/5/6 machinery:

| Component | Version/Pin | Purpose | Why Standard |
|-----------|-------------|---------|--------------|
| Vendored 9front toolchain (7a/7c/7l/cc/cpp) | donor pin `350266104f1fed81c78c3d05fa44f523bec0ae4d` | compile all tiers | Phase 2 vendored; tier-1 green [VERIFIED: build/aarch64/README.md:14] |
| `harness` (Rust, std-only) | in-tree `build/harness/` | srv9, drive9, gdbpoke9, trace-syscalls | quick task 261103-giq; the five Python tools deleted after equivalence gates (c42726256) |
| qemu-system-aarch64 | 11.1.2 (devshell) | donor guest venue | drift accommodations recorded in build/aarch64/README.md |
| plan9port (host) | nixpkgs unstable-2025-11-10 | mk/rc/9c host tools | flake.nix devshell [VERIFIED: /nix/store plan9port tree] |
| drawterm source (9front lineage) | nixpkgs `drawterm-0-unstable-2026-10-12` | the 9P draw-server code donor for the GUI trace host piece | the ONLY proven 9P-served draw device set with a cocoa backend (Open Question 2) |
| lima VM `harvey` | devshell | darwin's linux namespace venue | subset-exec.sh darwin leg |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| drawterm-derived 9P draw server | plan9port devdraw + drawfcall↔9P bridge | devdraw is drawfcall-only (not 9P) — the bridge is NEW protocol-translation code; drawterm's kern/ IS the 9P draw server Phase 12 ships |
| Per-entry `mk clean` in tier drivers | committed object sweep on toolchain change / documented operator step | the operator step is what made 06-03 honest — discipline, not machinery (06-03 deferred item); per-entry clean is deterministic and tier-local |
| Bake-time font copy (mkimage.sh) | committed root-tree/lib/font/bit mirror | lib/font/bit already lives in the repo; a 36MB duplicate tree invites drift — the adm/timezone bake-time-copy precedent (05-02) governs |
| Server-side input replay | OS-level injection (osascript/cliclick/CGEvent) | server-side is deterministic under TCG, needs no accessibility permissions, and commits as data beside the trace; OS-level remains an optional eyeball leg |

## Package Legitimacy Audit

**No external packages are installed by this phase.** The tier sweep builds only in-tree Plan 9 C with the vendored toolchain; host tooling (qemu, lima, plan9port, harness) was provisioned in earlier phases; the drawterm source is consumed read-only as a code donor (fetched via nixpkgs, not installed). `cliclick` (a candidate for OS-level input injection) is NOT installed on the dev machine — the recommended server-side input replay avoids adding it.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
Host (darwin dev machine / linux CI)
  ├─ subset-build.sh ──► linux namespace build (lima/native sudo-unshare)
  │     reads tiers/tier{0..4}-*.txt ──► mk filters (union) ──► /aarch64/bin/...
  │     emits T2-ART/T3-ART/T4-ART sha256 markers
  ├─ subset-exec.sh (tier-2 leg, extended)
  │     stages h*-binaries + tier2-sweep.rc into donor CONF=qemu bootdir
  │     rebakes donor kernel (mkrootall; devroot /boot) ──► boots qemu TCG
  │     srv9 serves root (9P2000/tcp over guestfwd 10.0.2.100:564)
  │     staged termrc override ──► sourced tier2-sweep.rc ──► T2-OK/FAIL markers
  ├─ harness trace-syscalls --gui <prog>   [darwin documented-manual only]
  │     drawterm-derived draw9 server: cocoa window + 9P2000/tcp listener
  │       (kern/devdraw.c devmouse.c devkbd.c devcons.c + gui-cocoa)
  │       + scripted-input replay file (mouse/kbd events, committed as data)
  │     qemu guestfwd #2: guest 10.0.2.100:DRAW9 ──► host 127.0.0.1:DRAW9
  │     guest: trace termrc mounts tcp!10.0.2.100!DRAW9, binds cons/consctl/
  │       mouse/cursor/draw over /dev, spin-arms + launches hrio|hacme|hsam
  │     tracesup step-traces root + born-traced descendants → /tmp/guitrace/<pid>
  │     host parses per-pid fragments → evidence/10-XX-<prog>-trace.txt
  └─ mkimage.sh (DESK-03 leg)
        root-tree tar ──► bake-time copies (rcmain, timezone, netaddr.rc,
          lib/font/bit/**) ──► ndb/local template substitution (sed, tokens)
        ──► manifest asserts (presence + content + count + magic) ──► bootfs.paq
```

### Recommended Project Structure (new files only)

```
build/aarch64/
├── tiers/tier2-services.txt      # D-13-shape data: ip tree, ndb, aux:listen, mntgen, exportfs, auth suite, dial/import/srv
├── tiers/tier3-gui.txt           # rio acme sam samterm plumb stats winwatch page proof screenlock
├── tiers/tier4-tail.txt          # the rest + fossil venti + gs + named exclusions with committed comments
├── markers/tier2.ledger          # committed header-only first (06-02 precedent), D-14 grammar
├── markers/tier3.ledger          # exec=TRACED:donor vocabulary for traced programs; COMPILE-ONLY for the rest
├── markers/tier4.ledger          # exec=COMPILE-ONLY / EXCLUDED:reason / (gs outcome)
├── tier2-sweep.rc                # the sourced functional-probe sweep (subset-exec.sh staged)
└── (trace assets extended in build/harness/src/trace.rs — --gui mode)
.planning/phases/10-*/evidence/   # trace artifacts + gate logs (committed)
```

### Pattern 1: Union tier-file consumption in the mk filters
**What:** The five filter sites (`sys/src/mkfile` LIBS/CMDS/SUBSYS, `sys/src/cmd/mkfile` TARG/DIRS, `cmd/ip`, `cmd/auth`, `cmd/auth/factotum`) currently read only `tier1-cmds.txt`. Phase 10 extends them to read all four tier files in ONE awk call (multi-file args — no new generated union file; per-file provenance preserved). plan9port mk backquote constraints (06-02 measured: single-line, brace-free rc, single-quoted awk) still govern.
**Example** (the shape each filter grows into — the sys/src/mkfile variant):
```mk
# Source: adapted from [VERIFIED: sys/src/mkfile:66-71]
TIER1=$HARVEY/build/aarch64/tiers/tier1-cmds.txt
TIER2=$HARVEY/build/aarch64/tiers/tier2-services.txt
TIER3=$HARVEY/build/aarch64/tiers/tier3-gui.txt
TIER4=$HARVEY/build/aarch64/tiers/tier4-tail.txt
T1KEEP=`{awk '/^[a-z0-9]/{ sub(/[\/: 	].*$/,""); print }' $TIER1 $TIER2 $TIER3 $TIER4}
```
**Critical:** `cmd/aux/mkfile` needs a NEW leaf filter hook (factotum's T1KEEPT shape) BEFORE any `cmd/aux:listen` entry lands — otherwise the keep-rule keeps `aux` in cmd's DIRS and the default aarch64 build sweeps aux's whole TARG (apm/pcmcia/x86 class) and breaks. cmd/ndb joins whole-dir (all 12 TARG members are tier-2), cmd/upas whole-dir, sys/src/games with a target/dir filter dropping mp3enc (its `CC=pcc` mkfile:38 fails on aarch64).

### Pattern 2: Tier driver per-entry form dispatch + anti-staleness
**What:** The tier-2/3/4 drivers extend subset-build.sh's tier-1 loop dispatch (cmd/<single> → cmd per-target install; cmd/<dir> → dir mk install; cmd/<container>/<dir> → nested install; cmd/<dir>:<target> → per-target). **Anti-staleness decision (the 06-03 open item — this phase owns it): RECOMMENDED = per-entry `mk clean` in the tier driver loop before each `mk install`.** Deterministic, tier-local, in-driver (never touches the default build), same philosophy as D-07's fresh-archive rule; donor-vm-build.sh's step-4b stale-object sweep is the in-tree precedent one level down. The working tree TODAY carries gitignored stale `.6`/`.7` objects in cmd/auth/*, cmd/auth/factotum/*, games/*, lp/LOCK.7 — mk's freshness graph never names the toolchain, so without the clean, tier-2's first wave would harvest stale artifacts on dev machines. (CI clean-checkouts are unaffected; the 06-09 plan9port-mk missing-intermediate quirk — "delete BOTH the in-tree binary and the installed copy when refreshing one binary" — is documented in deferred-items and applies to manual iteration.)

### Pattern 3: Sourced sweep + staged termrc override (tier-2 probes)
**What:** `tier2-sweep.rc` is staged into the donor kernel's devroot as /boot/tier2-sweep.rc (CONF bootdir data — text rides through mkrootall), run at boot by the staged termrc override (`exec rc -c '. /boot/tier2-sweep.rc donor'`), restored by the cleanup trap. The sweep emits `T2-OK <svc>` / `T2-FAIL <svc>` / `T2-DONE` over serial; the host asserts presence (never order), reds on any T2-FAIL, and asserts zero `panic|trap:` lines. Writable scratch: the srv9 `--writable-prefix /tmp --writable-prefix /usr/glenda/tmp` precedent. Every address comes from sourcing netaddr.rc values baked/echoed into the sweep — **never a hardcoded 10.0.2.x (Pitfall 9)**. Donor-venue only (D-04): no r9/9k venue guard needed (unlike tier1-sweep.rc's three-venue shape).

### Pattern 4: Born-traced GUI tracing (the 06-07 spine extended)
**What:** `tracesup` (`build/aarch64/tracesup.c`, `usage: tracesup [-v] rootpid outdir`) step-traces one root process and every born-traced descendant (donor sysproc.c:155 inheritance), writing one newline-framed fragment stream per pid to outdir — generic over any process tree, long-running programs included [VERIFIED: build/aarch64/tracesup.c:1-60, read this session]. The GUI mode: a trace termrc override mounts the host-served draw tree, per-file-binds cons/consctl/mouse/cursor and dir-binds draw over /dev, stages fonts, then launches the traced GUI program spin-armed; a scripted input file replays mouse/kbd events at the server; the session ends with the program's real quit path (rio's Exit, acme's Quit, samterm's quit); tracesup's root loop ends on Eprocdied, kills+starts leftovers, writes `_done`. Host parses per-pid fragments into per-program artifacts reusing trace.rs's ChainData/entry_re/resolve machinery (pid→program attribution via session-log pid markers).
**The event-loop proof (CONTEXT watch-out):** the trace must prove rio reached its event loop, not merely that syscalls occurred. Mechanism: the input script types a marker command into the rio New window's rc (its stdout is the serial cons) — `echo T3-RIOLIVE` lands in the serial log; equivalently acme executes `echo T3-ACMELIVE` (button-2 on the word), sam/samterm's marker is `ps` evidence of the samterm child + the session rc's pre/post markers. The host gate asserts these markers AND a per-program required syscall set (geninitdraw's open/read of /dev/draw/new, write to the draw data file).
**rio startup requirements [VERIFIED: sys/src/cmd/rio/rio.c:163-196, read this session]:** font check `access("/lib/font/bit/lucm/unicode.9.font")` (or `$font`) BEFORE initdraw — fonts must be staged in the guest; `open("/dev/snarf", OREAD|OCEXEC)` is unchecked (non-fatal); `geninitdraw` needs /dev/draw/new; `initmouse` needs /dev/mouse + /dev/cursor; `initkeyboard(nil)` opens `/dev/cons` + `/dev/consctl` and writes `rawon` [VERIFIED: sys/src/libdraw/keyboard.c:60-94]. libdraw's initdraw binds `#i` only if /dev/draw/new is absent [VERIFIED: sys/src/libdraw/init.c:117] — on the donor #i doesn't exist, so the host-served mount must be in place FIRST. acme defaults: `"/lib/font/bit/lucidasans/euro.8.font"` + lucm fallback [VERIFIED: sys/src/cmd/acme/acme.c:40-41]; acme also `bind("/acme/bin", "/bin", MBEFORE)` (acme.c:130 — the donor srvroot carries /acme). sam execs `/bin/aux/samterm` on a hardcoded path [VERIFIED: sys/src/cmd/sam/plan9.c:19: `char SAMTERM[] = "/bin/aux/samterm";`] — stage our samterm into the srvroot at `arm64/bin/aux/samterm` (host-side copy, the subset-exec srvroot-staging precedent) so the guest's /bin union resolves it.

### Pattern 5: Host-served 9P draw over a second guestfwd channel
**What:** The existing stanza `-netdev user,id=n0,guestfwd=tcp:10.0.2.100:564-tcp:127.0.0.1:1564` [VERIFIED: build/aarch64/subset-exec.sh:734] grows a second comma-separated guestfwd (e.g. guest 10.0.2.100:9999 → host 127.0.0.1:<draw9 port>). The guest mounts `tcp!10.0.2.100!9999` (numeric — no cs needed) and binds its files over /dev, mirroring the glenda profile's cpu-case choreography [VERIFIED: usr/glenda/lib/profile:23-25: `bind /mnt/term/dev/cons /dev/cons` + `bind /mnt/term/dev/consctl /dev/consctl` + `bind -a /mnt/term/dev /dev`]. The served tree's root presents `cons consctl mouse mousectl cursor snarf draw/{new,ctl,N/{ctl,data,colormap,refresh}}` — Harvey's own 9k devdraw/devmouse define that exact shape [VERIFIED: sys/src/9k/port/devdraw.c:271-314 ("new","colormap","ctl","data","refresh"), devmouse.c:84-96 ("cursor","mouse","mousein","mousectl")] — and drawterm's kern/devdraw.c replicates it [VERIFIED: nixpkgs drawterm source, kern/devdraw.c:256-299].

### Pattern 6: DESK-03 bake-time staging + manifest assertions
**What:** mkimage.sh stages `root-tree/` via tar, then bake-time copies (the `adm/timezone/local` precedent [VERIFIED: build/aarch64/mkimage.sh:187]). Fonts: `cp -R lib/font/bit "$STAGE/lib/font/bit"` (36MB raw — measured this session: shinonome 13MB, symbola 6MB, lucida 5MB, gb 4MB [VERIFIED: `du -sm` on lib/font/bit]) + manifest asserts on count + the three app-named fonts (`lucm/unicode.9.font`, `lucidasans/euro.8.font`, `pelm/latin1.8.font`) + a size floor. Profile: the trimmed D-14 profile is a COMMITTED root-tree file (it replaces the current 3-line skeleton [VERIFIED: build/aarch64/root-tree/usr/glenda/lib/profile, 3 lines]) — asserted by content greps (`case cpu` present, `upas/fs` absent). ndb/local: committed template with placeholder tokens; mkimage.sh sources `build/aarch64/netaddr.rc` (all bare key=value — POSIX-sh compatible [VERIFIED: build/aarch64/netaddr.rc, read this session]) and seds the tokens, then asserts the substituted values present + zero leftover tokens. mkpaqfs deflates blocks [VERIFIED: sys/src/cmd/paqfs/mkpaqfs.c:373-375 `deflateblock`/`DeflateEnc`], so the 36MB bitmap fonts compress hard; the size + an r9 boot at `-m 1G` is verified in the DESK-03 plan's EARLY task (CONTEXT watch-out), not at the end. Optional flourish available: a post-bake donor-side mount+ls of the image via our proven paqfs binary.

### Anti-Patterns to Avoid
- **Typed console interaction on the donor** — devcons /dev/cons reads fail Egreg by design; every in-guest gate is a sourced rc script (06-03).
- **Trusting mk exit status** — namespace builds exit 0 always (02-12); artifact + execution markers only.
- **Hardcoding 10.0.2.x anywhere** — Pitfall 9; netaddr.rc is the single source (D-15).
- **Settle-only GUI traces** — D-10: the event loop and sustained draw traffic ARE the measurement.
- **Adding the GUI trace to CI** — D-11 deliberate deviation; the trace scripts/artifacts stay venue-independent so a later Xvfb leg is possible.
- **Expect-typed per-binary driving** — doesn't scale (D-10, 05-03).
- **Silent exclusion drops** — every exclusion is a named, committed comment (Phase 6 D-08).
- **Per-bug drive-by fixes** — Pitfall-3 classes land one commit per class; mid-wave findings become per-dir checklist entries.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Host 9P draw server | a Rust fake-draw in build/harness (D-09's rejected option) | drawterm-derived serve-only binary (kern/devdraw.c + devmouse.c + devkbd.c + devcons.c + gui-cocoa) | proven 9P draw semantics; the exact code Phase 12 ships; input replay is a read-queue injection, no protocol design |
| Guest-side TCP probe dialer | a new C dialer | `cmd/dial/pass` (`-q`, stdin→connection) + `cmd/dial/expect` (goodstring/badstring matching) [VERIFIED: sys/src/cmd/dial/pass.c, expect.c read this session] | in-tree, compiles native, rides tier-2 as members |
| cs query client | hand-rolled /net/cs writes | `ndb/csquery` [VERIFIED: sys/src/cmd/ndb/csquery.c: open ORDWR → write addr → seek 0 → read] | the canned client; also a tier-2 member |
| dns query client | hand-rolled | `ndb/dnsquery` | same |
| Local 9P service posting | pipe-pair plumbing | `srv -e 'cmd' name [mtpt]` [VERIFIED: sys/src/cmd/srv.c usage: `srv -e [-abcCm] command [srvname [mtpt]]`], or the listen-backed topology | srv -e execs and posts the command's 9P fd — the exportfs probe needs nothing else |
| gs arch header | genarch on the linux host (WRONG — pcc probes LP64 host longs) | vendor the donor's `default.arm64.h` as `default.aarch64.h` | genarch-produced on real 9front arm64; Plan 9 type semantics match (long=4); see below |
| keyfile crafting for keyfs | a keyfile generator | `keyfs -m /mnt/keys /tmp/testkeys` over an ABSENT test file — keyfs serves an empty DB [VERIFIED: sys/src/cmd/auth/keyfs.c:866-868 `fd = open(userkeys, OREAD); if(fd < 0) return 0;`] and writes back on create | zero new code; exercises the write path |
| paqfs baking | donor mkpaqfs | in-tree `.build/bin/mkpaqfs` via mkpaqfs-host.sh | byte-format owner (04-01) |

**Key insight:** the phase's output is 90% orchestration of proven machinery. Every "don't build" above has a compiled-green in-tree or donor-verified answer; the two places where the honest answer is "this needs a decision" (gs build route, draw server route) are escalated as Open Questions rather than silently hand-rolled.

## Common Pitfalls

### Pitfall A: gs's build is pcc→APE, and APE is milestone-excluded — the stock route is dead for aarch64
**What goes wrong:** A plan treats gs as "753 files of portable C to fix-or-exempt per bug class" and discovers at the first task that gs's mkfile compiles NOTHING with 7c: `CC=pcc LD=pcc` [VERIFIED: sys/src/cmd/gs/mkfile:126-129]. pcc (sys/src/cmd/pcc.c, read this session) pipes cpp+7c with `-I/aarch64/include/ape -I/sys/include/ape` and links `/aarch64/lib/ape/libap.a` [VERIFIED: sys/src/cmd/pcc.c:165-193]. Neither path exists — `sys/src/ape/lib/ap/` has 386/amd64/arm/mips*/power*/riscv*/sparc/spim* dirs and NO arm64/aarch64 [VERIFIED: ls this session]; the milestone excludes ape. The 9front donor HAS arm64 ape (`donor/sys/src/ape/lib/ap/arm64/`: main9.s, memmove.s, memset.s, lock.c, cycles.s, getfcr.s, _seek.c, notetramp.c) which is why gs builds there.
**Why it happens:** gs is a Unix program; the Plan 9 port strategy was APE libc + a Plan 9 display device (gdevplan9.c), and ld.tr links `gp_unix.o` (the Unix OS layer) [VERIFIED: src/ld.tr head]. Harvey's dir-coverage records the whole pcc class failing on aarch64: `compress, eqn, grap, gs, lp, postscript(+6), spin, troff, games/mp3enc` [VERIFIED: build/aarch64/dir-coverage.txt build=fail rows].
**How to avoid:** surface this at the FIRST gs task (checkpoint:human-verify). Routes: (1) exempt gs by name with the committed comment naming the pcc→ape→no-aarch64-ape chain (D-01's valve; D-02's smoke dies with it — tell the user); (2) repoint gs to 7c + libstdio + native libc — a real port (new gp_plan9 os layer; POSIX shim; 753 files) — far over any bug-class budget; (3) vendor 9front's ape arm64 — the milestone-excluded ape port in miniature, contradicts the exclusion.
**Warning signs:** `mk` in gs fails at the first compile with missing `/aarch64/include/ape/*` headers, or at link with missing libap.a.
**Note the lp coupling:** lp's lpdaemon/lpdsend are pcc-built [VERIFIED: sys/src/cmd/lp/mkfile — `$O.lpdsend: lpdsend.$O / pcc -o ...`]; LOCK/lpsend/tonet are native mkmany members. lp joins tier-4 with lpdaemon/lpdsend excluded by name, or rides the same checkpoint.

### Pitfall B: plan9port devdraw does not serve 9P
**What goes wrong:** D-09's premise — "devdraw serves the full draw file tree (new/ctl/data/mouse/cursor/snarf/kbd)" over a mountable protocol — is false. plan9port's devdraw speaks the drawfcall/Wsysmsg protocol over a pipe, exec'd by the client's libdraw [VERIFIED: plan9port man page devdraw.1 BUGS, verbatim: "Devdraw should probably present a standard 9P server instead of using its own protocol."; and plan9port src/libdraw/drawclient.c:97-133 — fork/exec devdraw over a pipe]. The devdraw binary IS present and cocoa-capable on the dev machine [VERIFIED: /nix/store/7zjck7swbs0033s1h65m4w5ihzzk8dl8-plan9port-0-unstable-2025-11-10/plan9/bin/devdraw, Mach-O arm64] — it just can't be mounted.
**How to avoid:** Open Question 2 — the drawterm-derived serve-only binary is the recommended replacement (its kern/ device set is the 9P draw server Phase 12 uses; gui-cocoa is the darwin backend).
**Warning signs:** a plan task that says "mount the host devdraw" with no bridge component.

### Pitfall C: the ipconfig probe can wedge the root-serving net leg
**What goes wrong:** The donor guest's root fs arrives over 9P2000/tcp on the ethervirtio10 leg (bootargs=tcp fs=10.0.2.100). A functional ipconfig probe that re-DHCPs or rewrites /net/ipifc on THAT leg can bounce the address and hang the root mount (9P has no reconnect) mid-sweep.
**How to avoid:** ipconfig carries `-x mtpt` (alternate net mount point) [VERIFIED: sys/src/cmd/ip/ipconfig/main.c:543 `setnetmtpt(conf.mpoint, ...)`] — the probe configures a FRESH net tree (bind '#I'/'#l0' into the alternate mtpt first), leaving the root leg untouched. Same-values static reconfigure is the fallback. ping's target comes from netaddr.rc (slirp_gwv4; whether slirp answers ICMP on the guestfwd alias 10.0.2.100 is unmeasured — decide at probe bring-up, never hardcode).
**Warning signs:** the sweep dies mid-run with the serial log freezing after the ipconfig step.

### Pitfall D: TCG timing + functional probes
**What goes wrong:** Functional probes are more failure-prone under TCG than compile gates (CONTEXT watch-out): daemons need settle windows, listen+connect round-trips are slow.
**How to avoid:** presence-based marker assertions with generous budgets (the Phase-6 discipline); per-probe internal timeouts with named FAIL markers (never a wedge — USPACE-04's never-wedge edge); the host gate reds on T2-FAIL, never on absence-order.

### Pitfall E: the rc argv-stack residual class is live under load
**What goes wrong:** The 06-09 update: the pushlist/poplist residual class DID eat markers in r9 parity runs (pushword panic at the tr check; sweep rc exit `<bad utf8>`) on hosts under parallel TCG load [VERIFIED: deferred-items.md, 06-09 update]. Tier-2/3 sweeps are rc programs on the donor.
**How to avoid:** keep sweep constructs to the proven shapes (06-05's `~` dialect notes: quoting disables wildcarding; multi-star spans unreliable; subshell-free fn producers); the per-op pushlist/poplist audit (this phase's checklist item) covers exec.c's X* op family — pushlist/poplist are defined at exec.c:77/86 with ~49 call sites across exec.c (29), simple.c (15), glob.c (2) [VERIFIED: grep this session]; audit priority: Xbackq's siblings Xsubshell/Xpipefd/Xassign/Xqdol/Xfor/Xglob error paths.

### Pitfall F: default-build breakage via incomplete filter coverage
**What goes wrong:** The dual-arch default build sweeps whatever the filters keep. A `cmd/aux:listen` tier entry without a leaf filter in cmd/aux/mkfile sweeps all of aux (apm/pcmcia → aarch64 default build red for every workstream).
**How to avoid:** the aux leaf filter lands in the SAME commit as the first aux entry; games/mkfile's filter drops mp3enc by name; CI's wave0-host coverage-diff enforces the chain.

### Pitfall G: marker honesty at tier scale
**What goes wrong:** ~150 new tier entries invite counter-thinking ("150 green") instead of test-delivery (Phase 7's lesson).
**How to avoid:** named vocabulary per gate depth: T2-OK (functional), TRACED (syscall capture), COMPILE-ONLY (tier-4 default), EXCLUDED:reason; venue:donor on every ledger line; fossil/venti lines carry the documented-runtime-unverifiable comment.

## Code Examples

### `default.aarch64.h` (vendor the donor's genarch output — the whole file, verbatim)
The 9front donor carries `sys/src/cmd/gs/default.arm64.h` — genarch-produced on real arm64 [VERIFIED: .build/9front-donor/sys/src/cmd/gs/default.arm64.h:1-44, read this session]. Harvey's tree carries default.{386,alpha,amd64,arm,mips,power,riscv,riscv64}.h but NO aarch64 [VERIFIED: ls sys/src/cmd/gs/default.*]. The vendored file gets a provenance header comment (the `sys/src/cmd/unix/9cc/README` provenance pattern) naming the donor pin. Values (verbatim from the donor file):
```c
#define ARCH_ALIGN_SHORT_MOD 2
#define ARCH_ALIGN_INT_MOD 4
#define ARCH_ALIGN_LONG_MOD 4
#define ARCH_ALIGN_PTR_MOD 8
#define ARCH_ALIGN_FLOAT_MOD 4
#define ARCH_ALIGN_DOUBLE_MOD 8
#define ARCH_ALIGN_STRUCT_MOD 8
#define ARCH_LOG2_SIZEOF_SHORT 1
#define ARCH_LOG2_SIZEOF_INT 2
#define ARCH_LOG2_SIZEOF_LONG 2
#define ARCH_LOG2_SIZEOF_LONG_LONG 3
#define ARCH_SIZEOF_PTR 8
#define ARCH_SIZEOF_FLOAT 4
#define ARCH_SIZEOF_DOUBLE 8
#define ARCH_FLOAT_MANTISSA_BITS 24
#define ARCH_DOUBLE_MANTISSA_BITS 53
#define ARCH_MAX_UCHAR ((unsigned char)0xff + (unsigned char)0)
#define ARCH_MAX_USHORT ((unsigned short)0xffff + (unsigned short)0)
#define ARCH_MAX_UINT ((unsigned int)~0 + (unsigned int)0)
#define ARCH_MAX_ULONG ((unsigned long)~0L + (unsigned long)0)
#define ARCH_CACHE1_SIZE 1048576
#define ARCH_CACHE2_SIZE 1048576
#define ARCH_IS_BIG_ENDIAN 0
#define ARCH_PTRS_ARE_SIGNED 0
#define ARCH_FLOATS_ARE_IEEE 1
#define ARCH_ARITH_RSHIFT 2
#define ARCH_CAN_SHIFT_FULL_LONG 1
#define ARCH_DIV_NEG_POS_TRUNCATES 1
```
The gs mkfile's `$objtype.h` rule [VERIFIED: sys/src/cmd/gs/mkfile:148-170] runs genarch only when `$objtype == $cputype` (never in our namespace builds, where cputype=unix) — otherwise it copies `default.$objtype.h` and fails loudly if absent. genarch's build guard (mkfile:163-170) is likewise `$objtype==$cputype`-gated with pcc — the 06-04 generator-audit G6 row. The alternative D-01 route (run genarch on the donor guest) is viable (the donor is a native arm64 Plan 9 system, pcc-class toolchain present) but strictly heavier; vendoring is measure-equivalent since the donor file IS genarch's output on the same ABI.

### Tier-2 probe shapes (semantics locked by D-03/D-05/D-12; internals discretionary)
```rc
# cs: hcs -f /tmp/test.ndb; the canned client answers a scripted lookup
#   (test.ndb: 'sys=probehost ip=10.9.9.9 dom=probehost.test' class)
hcsquery probehost > /tmp/cs.out
grep 10.9.9.9 /tmp/cs.out && echo T2-OK cs

# dns: hdns -f /tmp/test.ndb (cs already running); canned answer from ndb
hdnsquery probehost.test > /tmp/dns.out
&& echo T2-OK dns

# exportfs: srv -e execs + posts + mounts (no network leg needed)
srv -e 'hexportfs -r /tmp/xroot' xprobe /mnt/xprobe
cmp /mnt/xprobe/known.txt /tmp/xroot/known.txt && echo T2-OK exportfs

# aux/listen: test srvdir, scripted connect via dial/pass
hlisten -d /tmp/service tcp &	# /tmp/service/tcp<PORT> execs a banner-echo rc
echo probe | hpass tcp!$guestv4!<PORT> > /tmp/listen.out
grep T2-LISTEN-BANNER /tmp/listen.out && echo T2-OK listen

# mntgen: on-demand dir creation + /srv post
hmntgen -s mnttest /tmp/mnt
ls /tmp/mnt/probedir >/dev/null && test -e /srv/mnttest && echo T2-OK mntgen

# ipconfig+ping: -x alternate mtpt (Pitfall C); addresses from netaddr.rc
hipconfig -x /net.alt ether /net.alt/ether0 $slirp_guestv4 $slirp_maskv4 $slirp_gwv4
hping $slirp_gwv4 && echo T2-OK ipconfig-ping

# auth shallow (D-05): factotum ctl round-trip; keyfs over absent test file;
# authsrv via the listener (a garbage/EOF ticket request → clean exits(0))
hfactotum
echo 'key proto=p9sk1 dom=probe user=probe !password=x' > /mnt/factotum/ctl
grep 'proto=p9sk1' /mnt/factotum/ctl && echo T2-OK factotum
hkeyfs -m /mnt/keys /tmp/testkeys
mkdir /mnt/keys/probeuser && test -s /tmp/testkeys && echo T2-OK keyfs

# plumber (D-12): test rules file, routed message, clean shutdown
hplumber -p /tmp/plumbing
echo 'T2-PLUMB-PAYLOAD' > /mnt/plumb/send	# rules route it to the 'probe' port
read-from /mnt/plumb/probe and assert && echo T2-OK plumber
```
keyfs details [VERIFIED: sys/src/cmd/auth/keyfs.c:140-191 read this session]: `-m mntpt` (default /mnt/keys), optional keyfile arg (default /adm/keys), `getauthkey` failure is a warning not an exit (line 175-176), mount MREPL|MCREATE, and readnvram has NO arm64 nvtab entry so the donor leg always takes the zero-key+warning path [VERIFIED: sys/src/libauthsrv/readnvram.c:28-56 nvtab — sparc/pc/mips/power/arm/debug rows only; the `nvram` env var or cputype=debug selects /tmp/nvram]. authsrv is a per-connection program (reads a TICKREQLEN ticket request on fd 0; EOF or unknown type → exits(0)) [VERIFIED: sys/src/cmd/auth/authsrv.c:65-101] — "starts and listens" = the aux/listen + service-script topology, exactly as the donor's `rc/bin/service.auth/tcp567` (`exec /bin/auth/authsrv -N $3`) [VERIFIED: donor file read this session]. plumber [VERIFIED: sys/src/cmd/plumb/plumber.c threadmain + fsys.c:194-220]: needs `$user`/`$home`, a rules file (`-p`), posts `/srv/plumb.$user.$pid` (ORCLOSE — rm it for clean shutdown), mounts /mnt/plumb.

### The aux leaf filter (pattern to replicate)
```mk
# Source shape: sys/src/cmd/auth/factotum/mkfile:9-17 (read this session), retargeted
TIER1=$HARVEY/build/aarch64/tiers/tier1-cmds.txt
TIER2=$HARVEY/build/aarch64/tiers/tier2-services.txt
T1KEEPT=`{awk '/^cmd\/aux:/{ sub(/^cmd\/aux:/,""); sub(/[\/: 	].*$/,""); print }' $TIER1 $TIER2}
TARG=`{if(~ $objtype aarch64 && ! ~ $"COVERAGE_SWEEP 1) for(i in $ALLTARG) if(~ $i $T1KEEPT) echo $i; if not echo $ALLTARG}
```

### ndb/local template + substitution (D-15)
Committed template (home: `build/aarch64/root-tree/lib/ndb/local.template` or `build/aarch64/ndb-local.template` — discretion), tokens `@GUESTV4@`-style; mkimage.sh sources netaddr.rc and seds:
```sh
# mkimage.sh fragment (shape; ndb(6) tuple format verified: attr=value tuples,
# /lib/ndb/local is the database root [VERIFIED: sys/man/6/ndb:6-25])
. "$HARVEY/build/aarch64/netaddr.rc"	# bare key=value, sh-compatible
sed -e "s/@GUESTV4@/$slirp_guestv4/g" -e "s/@GWV4@/$slirp_gwv4/g" \
	"$template" > "$STAGE/lib/ndb/local"
# manifest: substituted value present AND no leftover token
grep -q "$slirp_guestv4" "$STAGE/lib/ndb/local" || err "ndb/local substitution missing"
grep -q '@' "$STAGE/lib/ndb/local" && err "ndb/local carries an unsubstituted token"
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| trace-syscalls.py (Python) | `harness trace-syscalls` (Rust, build/harness/src/trace.rs; hello + --boot-chain + --tier1 modes) | 261103-giq (Nov 3) | the GUI mode extends the Rust rig; the .py is deleted |
| typed drive9 steps (22 rows) | sourced sweep + staged termrc override | 06-03 | the tier-2 probe sweep inherits this |
| 05-03 rc traceloop + tagged fragments | tracesup in-guest supervisor (one fragment file per pid on the srv9 writable prefix) | 06-07 | kills per-fragment forks + mid-line gluing; generic over long-running trees — the GUI trace rides it as-is |
| per-lib PENDING-CLASSIFY | ledger exec vocabulary (CARRIED/BOOT/T1-OK/TEST-FAIL) | 06-05/06-06 | tier-2/3/4 ledgers extend the vocabulary (TRACED/COMPILE-ONLY) — never an unearned OK |
| tier-donor hosted CI leg | RETIRED 06-10 (tier-9k = Harvey's own kernel on x86_64); aarch64 donor gates LOCAL-only | 2026-11-05 | Phase 10's CI topology question — see Open Question 3 |

**Deprecated/outdated:** `trace-syscalls.py` (deleted); the tier-1 donor CI leg (retired); expect-typed sweeps (05-03 lesson); committed `.6`/`.8` lib objects (USPACE-05 — never cargo-culted).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | slirp answers ICMP echo on the guestfwd alias 10.0.2.100 (the D-03 "host-side srv9 leg" ping target); if not, the target is slirp_gwv4=10.0.2.2 | Pitfall C | probe targets the wrong address — caught at bring-up, either value comes from netaddr.rc |
| A2 | hosted-CI provisioning of the 9front donor works as designed — the 06-08 tier-donor leg was authored but NEVER ran hosted (PENDING FIRST PUSH, then retired by 06-10) | Open Question 3 | first tier-2 CI leg needs a staged rollout or stays documented-manual initially |
| A3 | paqfs deflate compresses the 36MB bitmap fonts to ~≤15MB (bitmap data is zero-heavy) | Pattern 6 | measured at the DESK-03 early task; worst case the initrd is ~40MB raw — still trivial against `-m 1G` |
| A4 | drawterm's kern/ device set + gui-cocoa lift into a serve-only host binary in days, not weeks (the 9P server loop and the cocoa backend are separable; drawterm main.c's cpu-client half is what gets dropped) | Open Question 2 | unbuilt; if it drags, the fallback is a drawfcall↔9P bridge in front of plan9port devdraw |
| A5 | rio/acme tolerate an absent /dev/snarf (open is unchecked at rio.c:179/acme.c:125; downstream use guarded — rio.c:244/266) | Pattern 4 | none — verified in source; noted for the served-tree shape |
| A6 | a second guestfwd 9P service mounts unauthenticated in the guest (the srv9 root-mount precedent; 05-03 measured fauth falling through unauthenticated) | Pattern 5 | low — same transport as the proven root mount |

## Open Questions

1. **gs disposition vs the pcc/APE wall (D-01/D-02 conflict — escalate to the user at the first tier-4/gs plan).** — **(RESOLVED 2026-11-05: D-01a in 10-CONTEXT.md — vendor 9front's ape arm64 slice as `sys/src/ape/lib/ap/aarch64/` plus the donor's genarch-produced `default.arm64.h` as `default.aarch64.h`; the pcc-class link wall falls, each dir remains fix-or-exempt per D-01, and D-02's render smoke survives.)**
   - What we know: gs compiles via pcc→APE (mkfile:126-129); ape has no aarch64 port (milestone exclusion); the donor has arm64 ape; the whole pcc class fails on aarch64 in the committed coverage data; `default.aarch64.h` vendored from the donor unblocks only the header step, not the link.
   - What's unclear: whether the user's D-01/D-02 intent holds given the wall — exempt-by-name (fix budget exceeded at the build-system class; the render smoke dies) vs a bounded native-port attempt vs vendoring 9front's ape arm64 slice (contradicts the ape exclusion).
   - Recommendation: `checkpoint:human-verify` in the gs plan presenting the evidence chain (Pitfall A); the likely landing is exempt-with-comment + document, which also retires D-02. `default.aarch64.h` lands either way (cheap, unblocks any future route).
2. **D-09 correction: what serves the host draw for the GUI traces?** — **(RESOLVED 2026-11-05: D-09 amended in 10-CONTEXT.md — a drawterm-derived serve-only binary serves `/dev/draw` on the host (plan9port devdraw cannot serve 9P); input injection is server-side replay of a committed event script, D-10 amended accordingly.)**
   - What we know: plan9port devdraw = drawfcall-over-pipe, not 9P (man page BUGS + libdraw/drawclient.c, verified). drawterm (nixpkgs unstable-2026-10-12, 9front lineage) carries the full 9P-served device set (kern/devdraw.c, devmouse.c, devkbd.c, devcons.c) + gui-cocoa — the Phase-12 draw server code itself.
   - What's unclear: user sign-off on substituting the D-09 named tool (plan9port devdraw) with a drawterm-derived serve-only binary.
   - Recommendation: adopt the drawterm-derived server (D-09's intent — proven, zero new protocol design, cocoa — preserved; only the named package changes). Input injection: server-side replay of a committed event script (deterministic under TCG; osascript/cliclick OS-level injection stays an optional eyeball leg — cliclick is not installed). Both sub-choices warrant a human checkpoint since D-09/D-10 are locked.
3. **CI topology for the tier-2 donor execution gates (D-11 says CI carries them; the venue-pin says CI never provisions 9front).** — **(RESOLVED 2026-11-05: D-11a in 10-CONTEXT.md — ALL tier-2/3 donor-execution gates run documented-manual on aarch64 dev hosts (darwin via lima) this phase; hosted CI carries only the tier compile gates; hosted execution legs may be added once proven.)**
   - What we know: phase6.yml's venue pin greps only phase6.yml (`wf=.github/workflows/phase6.yml`) — a sibling phase10.yml doesn't trip it; but the tier-donor leg never ran hosted (A2), and the retirement's spirit was "no hosted 9front dependency for tier gates."
   - What's unclear: sibling workflow with donor provisioning (unproven) vs tier-2 execution gates landing documented-manual first (like the GUI leg) with hosted legs added once proven.
   - Recommendation: compile gates in CI (phase6.yml-shape sibling, linux, SHA-pinned); tier-2 donor execution gates documented-manual darwin FIRST (the subset-exec.sh venue is darwin/lima-native), hosted linux leg as a follow-up once the provisioning path is proven — surface to the user at plan time.
4. **fgui's tier home.** — **(RESOLVED at planning: adopted per this question's recommendation — `cmd/auth/factotum:fgui` joins tier-2 compile-only in 10-04.)** factotum's sibling target (excluded from tier-1 as out-of-tier graphics) is now compilable (libdraw/libcontrol tier-0 green since 06-05). Append `cmd/auth/factotum:fgui` to tier-2 (compile marker only — it needs draw at runtime) or tier-3. Recommendation: tier-2 with COMPILE-ONLY/TRACED-adjacent honesty vocabulary.
5. **rio trace's spawned rc.** — **(RESOLVED at planning: no action needed — 10-08's trace artifacts carry the coverage-boundary header naming the born-traced window rc, per this question's record.)** rio's New window execs the guest's /bin/rc (donor's rc — born-traced, gets traced as a descendant). That's bonus coverage, not a defect — the artifact's coverage-boundary header names it (05-03 precedent). No action; recorded so the parse doesn't confuse the window rc's stream with rio's.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qemu-system-aarch64 | all guest legs | ✓ (devshell) | 11.1.2 (pinned by flake usage) | — |
| limactl + lima VM `harvey` | darwin namespace builds | ✓ (devshell) | — | native linux leg (subset-exec FORCE_NATIVE_LINUX) |
| `harness` (srv9/drive9/gdbpoke9/trace-syscalls) | all donor legs | ✓ (devshell, in-tree Rust) | — | — |
| 9front donor checkout at pin 350266104f | donor venue | ✓ `.build/9front-donor/` (pin-gated by every script) | 350266104f | — |
| plan9port | host mk/rc/9c | ✓ nix store unstable-2025-11-10 | — | — |
| plan9port devdraw (cocoa) | (superseded by OQ-2) | ✓ Mach-O arm64 binary present | — | drawterm-derived server |
| osascript | optional OS-level input leg | ✓ /usr/bin/osascript | — | server-side replay (recommended) |
| cliclick | optional OS-level input leg | ✗ not installed | — | server-side replay; or devshell add |
| drawterm source (code donor) | OQ-2 server | ✓ fetched this session (nixpkgs) | unstable-2026-10-12 | — |
| GitHub ubuntu-latest | CI legs | ✓ | — | — |

**Missing dependencies with no fallback:** none for the recommended routes.
**Missing dependencies with fallback:** cliclick (avoided via server-side replay).

## Security Domain

Scoped — this phase adds no new network-exposed or secret-bearing surface, but two boundaries exist:

| Boundary | Threat | Mitigation |
|----------|--------|------------|
| Host 9P draw listener (darwin trace leg) | a LAN peer mounts the served draw tree (input injection into the guest session) | bind 127.0.0.1 only — the hostfwd/guestfwd discipline (PITFALLS security table: never 0.0.0.0); unauthenticated 9P on loopback is the hermetic norm |
| Test key material (keyfs/factotum probes) | test keys mistaken for real credentials | absent/throwaway test keyfile only; zero real secrets baked; the DEMO-domain labeling discipline is Phase 11's, but probe fixtures carry `dom=probe`/clearly-test values |

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | p9sk1 handshakes are Phase 11; this phase's auth probes are start/respond only (D-05 shallow) |
| V5 Input Validation | marginal | probe fixtures are fixed committed inputs; no parser consumes host-controlled data |
| V6 Cryptography | no | zero crypto changes; libsec untouched |

## Sources

### Primary (HIGH confidence — read in-repo/nix-store this session)
- `build/aarch64/tiers/tier{0,1}-*.txt`, `markers/tier{0,1}.ledger` — tier/ledger grammars (headers + entry forms, verbatim)
- `sys/src/mkfile:60-75`, `sys/src/cmd/mkfile:17-33`, `cmd/ip/mkfile`, `cmd/auth/mkfile`, `cmd/auth/factotum/mkfile` — the five filter sites + keep-rule
- `build/aarch64/subset-build.sh:138-253` — tier driver dispatch; `subset-exec.sh` — the donor gate stack (staging, bootdir patch, termrc override, marker asserts)
- `build/harness/src/trace.rs` (3624 lines — trace-syscalls modes) + `build/aarch64/tracesup.c` — the trace rig spine
- `build/aarch64/dir-coverage.txt` — the committed per-dir aarch64 build verdicts (the tier-2/3/4 compile inventory)
- `build/aarch64/mkimage.sh`, `mkpaqfs-host.sh`, `netaddr.rc`, `root-tree/` — the DESK-03 machinery
- gs: `sys/src/cmd/gs/mkfile` (CC=pcc, $objtype.h rule), `sys/src/cmd/pcc.c`, `.build/9front-donor/sys/src/cmd/gs/default.arm64.h` (verbatim), `sys/src/ape/lib/ap/` (no arm64), donor `sys/src/ape/lib/ap/arm64/` (exists)
- services/auth: keyfs.c, authsrv.c, factotum/fs.c, plumber.c+fsys.c, ndb/cs.c+csquery.c+dns.c+dnsquery.c, ip/ipconfig/main.c, mntgen(4), exportfs(4), listen(8), keyfs(4), ndb(6), libauthsrv/readnvram.c, dial/pass.c+expect.c, srv.c, donor rc/bin/service.auth/tcp567
- GUI: rio.c, acme.c, sam/io.c+plan9.c, libdraw/{init,event,keyboard,mouse}.c, 9k/port/dev{draw,mouse}.c, lib/font/bit sizes, usr/glenda/lib/profile (verbatim), rc/lib/rcmain
- plan9port (nix store): man/man1/devdraw.1 (the BUGS sentence, verbatim), src/cmd/devdraw/*, src/libdraw/drawclient.c
- drawterm (nixpkgs source): kern/dev{draw,mouse,kbd,cons}.c, gui-cocoa/, Make.osx-cocoa
- `.planning/phases/06-*/06-CONTEXT.md`, `deferred-items.md`, `06-EVIDENCE.md` — the Phase-6 machinery decisions + Phase-10-filed checklist evidence
- `.github/workflows/phase6.yml` — the CI shape + venue pin

### Secondary (MEDIUM confidence)
- 9front/9front donor tree at pin 350266104f — default.arm64.h, ape arm64 slice, service.auth/tcp567 (read from the pinned local checkout, gitignored but present)

### Tertiary (LOW confidence)
- none — no web-only claims carried forward

## Metadata

**Confidence breakdown:**
- Standard stack / tier machinery: HIGH — every mechanism read in-tree this session; compile state from the committed coverage report
- Architecture (trace rig + probes + staging): HIGH — the spine (tracesup/termrc-override/srv9/guestfwd/manifest asserts) is committed and gate-proven in 06-03/06-07; the GUI mode is an extension, not a new rig
- Pitfalls: HIGH for the gs/devdraw/ipconfig walls (verified); MEDIUM for probe-timing behavior under TCG (measured in prior phases, not this phase's probes)
- Open Questions 1-2: the two escalations are evidence-backed corrections to locked decisions, not speculation

**Research date:** 2026-11-05
**Valid until:** 2026-12-05 (stable — all critical claims are in-repo verified against committed files; re-verify only if the donor pin moves or the tier machinery is restructured)
