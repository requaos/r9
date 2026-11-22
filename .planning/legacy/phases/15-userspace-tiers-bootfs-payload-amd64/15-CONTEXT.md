# Phase 15: Userspace Tiers + Bootfs Payload (amd64) - Context

**Gathered:** 2026-11-09
**Status:** Ready for planning

<domain>
## Phase Boundary

The services/GUI/long-tail tiers ride the Phase-14 re-keyed machinery (append-only allowlists, artifact+execution markers, k10t1 donor venue first then r9-x86_64): the services tier — aux/listen, mntgen, ndb/cs+dns, exportfs, the ip/ suite, the auth/ suite (~80 files) — compiles AND executes on both venues with functional-probe markers (USPACE-07); the GUI stack (libdraw family + rio + acme + sam + plumb) compiles for amd64 with scripted-interaction donor syscall traces captured measure-first on a draw-enabled 9k donor venue (USPACE-08); the long tail (upas + lp + every remaining cmd dir) compiles under a machine-verified full DIRS-coverage proof, with fossil+venti compile-gated/documented runtime-unverifiable and gs compile-gated PLUS a headless render smoke (USPACE-09); and the DESK-03 payload (whole /lib/font/bit tree, trimmed glenda profile, templated /lib/ndb/local) stages in the root-tree skeleton with manifest assertions so Phases 18-19 never block on payload.

**Work in the harvey repo** — with in-phase r9 kernel fixes when services execution surfaces gaps (D-01, deliberate). This phase SUPERSEDES the never-executed v2.0 Phase 10 plans (aarch64-keyed, `.planning/milestones/v2.0-phases/10-*/`); those remain a mechanism reference only.

Addresses USPACE-07, USPACE-08, USPACE-09, DESK-03.

NOT in this phase: host net modes / tap leg / NETIP-08 (Phase 16), the Phase-7/8 review backlog as such (Phase 17 — though D-01's in-phase fixes feed it), the auth substrate — devcap `#¤`, bake-time `/adm/keys`, keyfs, real p9sk1 handshakes (Phase 18), cpu/drawterm/rio-in-session/`#!` exec (Phase 19), the draw-over-9P trace topology rig (deferred — the phase that grows r9's desktop path; see D-04), fossil/venti runtime (no block device in r9), the ape *environment* (~200-program POSIX port; note amd64 ape lib is in-tree anyway), aarch64 anything (frozen at the v2.0 pin).

</domain>

<decisions>
## Implementation Decisions

**Ancestor reconciliation (read first):** this phase is v2.0 Phase 10 re-keyed to x86_64. Four of the ancestor's researched choices were re-litigated deliberately with the v2.1 premises visible — trace venue (D-04: kernel-local NOW, 9P topology later, superseding both the ancestor's draw-over-9P-only D-08 and today's initial kernel-local-only read), trace depth (D-05: the ancestor's scripted-interaction ADOPTED over today's initial startup-to-idle), gs render smoke (D-11: the ancestor's stronger gate ADOPTED), and donor-only execution (superseded by the v2.1 roadmap itself: both venues). Everything else ancestral that still fits carries — marked "carried" below.

### r9 services leg (USPACE-07)
- **D-01:** **Fix everything in-phase.** Every r9 kernel gap surfaced by services execution is root-caused and fixed this phase — push-before-pin, FOD re-wire-then-pin per kernel-byte commit (the 13-06/14-07 discipline). Deliberate user override past the hybrid recommendation: the flush-out doctrine is the milestone's point. **Boundary:** Phase-18-owned substrate is NOT pulled forward — devcap `#¤`, the bake-time `/adm/keys` domain, keyfs stay 18's; auth entries needing that substrate take named SKIP→18. The factotum devproc-protect/noswap + devsrv-mount gap class is NOT 18-owned — under fix-everything it is fix-in-phase if the sweep surfaces it (planner cross-checks Phase 17's named scope for overlap).
- **D-02:** **Functional probe markers for daemons** — a daemon earns its marker when a real client interaction succeeds through it (cs answers a query via /net/cs, listen's announce accepts a loopback connect, dns resolves a baked hosts entry, exportfs serves a guest-side mount, mntgen mounts). Impossible-on-venue = named SKIP, never a weaker pass. (Carries v2.0 D-03's probe semantics re-keyed to the x64 venues — the ancestor's per-service probe list is the researcher's starting map; the Phase-7 UAT test-delivery-not-counters lesson is the rationale.)
- **D-03:** **A new dedicated services image** — mkimage.sh's fifth contract: chain tree + services payload + the 14-09 net bring-up (ipconfig before daemons), its own services-sweep.rc run from termrc (the tier-termrc shape), its own expect gate per venue. One-image-per-gate (14-06 discipline) preserved; Phase 18/19 consume it. (Supersedes the ancestor's donor-only D-04 — the v2.1 roadmap executes on BOTH venues.)

### GUI leg (USPACE-08)
- **D-04:** **Draw-enabled kernel-local donor venue THIS phase; the 9P topology later.** A new 9k donor config variant (k10cpu device set + vga/draw/mouse — `sys/src/9k/port/devdraw.c` + `sys/src/9k/k10/devvga.c` exist, config-gated today) boots under qemu -vga std; rio/acme/sam genuinely start against a kernel-local devdraw; the full client-side syscall mix (every open/read/write/poll on /dev/draw, mouse, cons, snarf, wctl) is captured. The ancestor's topology argument (Phase 19's /dev/draw arrives MOUNTED from drawterm via devmnt — kernel-local misses the RPC shapes) is honored by DEFERRAL: the drawterm-derived serve-only 9P server + venue channel + topology re-capture belong to the phase that grows r9's desktop path (see deferred). Rationale: r9's devmnt client is already proven under the net gates — the client-side syscall mix is the unknown that gates kernel growth; the donor kernel is ours, so the venue is in-repo work. — **Reversibility:** costly — the trace rig, staged fonts, and replay scripts key off the venue; switching topology later means re-capturing every trace (the deferral accepts exactly this, once, at the right time).
- **D-05 (ancestor reconciliation — ADOPTS v2.0 D-10):** **Scripted-interaction traces.** Each traced program is driven by a committed replay script — rio: menu New + type into the rc window; acme: type, execute a command, quit; sam+samterm: type, edit, write. The sustained event loop + mouse/kbd read loops + snarf + draw traffic ARE the measurement. On the kernel-local venue the deterministic injection is in-guest synthetic input streams (the /dev/kbin + /dev/mousein class — researcher verifies 9k's exact served files); determinism rides the replay mechanism, never GUI-automation luck. Startup-to-idle is the floor every script starts from, not the ceiling.
- **D-06:** **Raw + distilled table, both committed.** Raw per-binary trace logs (the proof of a real run) plus a distilled syscall-set table per binary (syscall → count → args-shape notes, the 05-03 measured-set format) — the table is what r9-growth plans cite. Two-layer shape, same as tier markers+ledgers.
- **D-07:** **The trace is the gate.** No separate GUI exec markers — the committed trace+table is USPACE-08's runtime artifact, and the rig asserts the script COMPLETED (reached the event loop and finished the script — the ancestor's watch-out: never merely "syscalls occurred"). GUI compile gets artifact markers like every tier.
- **D-08 (carried, v2.0 D-05 re-keyed):** **Auth suite shallow execution subset** where substrate allows: factotum starts and answers a scripted probe, keyfs serves over a test key file, authsrv starts and listens — bounded by D-01's 18-owned boundary (the k10t1 donor has cap/ssl/tls in its device set, so the donor leg runs these for real; the r9 leg follows the machinery). Real p9sk1 handshakes stay Phase 18's loopback probe — never duplicated here.

### Long tail (USPACE-09)
- **D-09:** **Full DIRS-coverage proof re-keyed to amd64** — every sys/src/cmd dir (plus any unproven lib dirs) compiles for amd64, machine-verified coverage assertion, not sampled. The Phase-6 dir-coverage machinery re-keyed.
- **D-10:** **Fix in-tree, no skip-vocabulary for compile failures.** Every sweep failure is root-caused and fixed (the Phase-6 landmine discipline — jpg/mahjongg/4a precedent; fix classes land one commit per class); the coverage proof only ends green. The ancestor's aarch64 exclusion classes are RE-DERIVED, not carried: the "x86-hardware-only" class (aux/vga, usbd, disk/prep) is NOT an exclusion on amd64 — it compiles natively; per-arch-asm classes (games snes/gb) re-derived for amd64.
- **D-11 (ancestor reconciliation — ADOPTS v2.0 D-02):** **gs render smoke beyond compile.** gs renders a tiny checked-in .ps fixture to a bitmap FILE device (no display) on the donor — 753 files of ported unix C are the LP64-fossil habitat and compile-green misses them. On amd64 the ancestor's ape wall DOES NOT EXIST (verified at discussion: `sys/src/ape/lib/ap/amd64/` and `sys/src/cmd/gs/default.amd64.h` are in-tree) — gs compiles stock through its pcc route; the smoke is pure upside. gs's GPL fonts are never staged anywhere (DESK-03 fonts come from `lib/font/bit` only).
- **D-12:** **Per-class append-only tier data files** — new tier files (the ancestor's tier2-services/tier3-gui/tier4-tail shape) beside the untouched tier0/tier1 files; each gate computes expectations from its own file; diffs review one class at a time. Exact names are planner discretion.

### Bootfs payload (DESK-03)
- **D-13:** **Payload lives in the committed root-tree skeleton** (`build/x86_64/root-tree/`) — the shared base every mkimage contract stages from — so images that declare it inherit fonts/profile/ndb by construction and Phases 18/19 compose without re-staging. — **Reversibility:** costly — once contracts stage from the skeleton and 18/19 depend on it, retracting payload touches every bake contract plus the downstream consumers.
- **D-14 (carried, v2.0 D-13):** **The WHOLE `lib/font/bit` tree ships** — all 36MB, no exclusions. Every app finds every font; no exclusion bookkeeping; font choice stays a Phase-19 runtime concern. Verified consequence (ancestor measurement): bootfs.paq 4.0MB → roughly 8× growth; the boot-memory-headroom check happens EARLY in the DESK-03 plan, not at the end.
- **D-15:** **Manifest = committed data file + bake-time assert.** A plain-data manifest file (tier-allowlist pattern) consumed by mkimage.sh; the bake FAILS if any listed path is missing/empty in the staged tree; no second hardcoded list anywhere (the 14-08 single-source rule; the ancestor's "every baked file is a manifest assertion" carried).
- **D-16 (carried, v2.0 D-15):** **/lib/ndb/local is a committed TEMPLATE substituted at bake time from the netaddr seam** — the 14-09 D-11 single-seam discipline (mkimage's net section greps root-tree/lib for venue literals outside netaddr.rc; the template MUST be substitution-based to pass that assert). Chosen over a static slirp-values file (drift-prone) and over full generation (invisible to review).
- **D-17 (carried, v2.0 D-14):** **/usr/glenda/lib/profile stages as the trimmed cpu-aware reference**: `upas/fs` + `news` dropped; the `case terminal` path keeps its binds/tmp/font but LOSES `exec rio` (serial console — rio must provably not start at boot); the `case cpu` path intact (binds /mnt/term/dev, `font=/lib/font/bit/pelm/latin1.8.font`, `exec rio`) for Phase 19 to consume unchanged.

### Carried forward (locked in roadmap/prior phases — do not re-litigate)
- Tier machinery: append-only committed data files; artifact+execution markers the only gates; mk exit status never trusted; venue honesty (named SKIP lines, SKIP-whitelist converse assert, never an unearned OK); k10t1 donor first then r9; the 14-08 sha256-manifest ABI-witness discipline.
- The ipconfig T1-SKIP reason is stale on x64 and flips live with this phase's tier append (14-09 note — the port devcons serves /dev/random there).
- fossil+venti: compile-gated, documented runtime-unverifiable (no block device in r9). gs included (D-11).
- push-before-pin; FOD re-wire-then-pin after every r9 kernel-byte fix pass; no /boot/factotum (auth stub = absence); paq format verbatim.
- Sourced-not-typed in-guest sweeps + serial markers, timing-tolerant presence assertions (never interleaving order); TCG-generous timeouts.
- Atomic class-sweeps: bug-class fixes land one commit per class, never interleaved with bring-up commits.
- Phase-6-filed checklist items re-derived on amd64 where still live (the ancestor owned them; this phase inherits): libmemdraw drawtest/arctest LP64 pixel-path faults (arch-independent C bugs — the GUI traces exercise libmemdraw hard; budget the audit BEFORE/alongside trace capture or the traces may record garbage pixels), stale mkfile test targets (liboventi vttest, libmemdraw test), libdynld ULONG<0 comparison, rc argv-stack residual audit, the tier anti-staleness decision (decided in the FIRST mass tier-2 plan — tier-2 is the first mass cmd wave since the toolchain fixes).

### Claude's Discretion
- Exact tierN file names and per-tier membership order; wave decomposition within tiers 2/3/4 (planner's dependency sort; tier files are the record).
- The draw donor config variant's name/build (k10t1 sibling vs k10cpu variant config), its vga bring-up mechanics under qemu q35 (-vga std), and how the venue scripts (build/mm) gain its leg.
- The synthetic-input mechanism's exact files on 9k (kbin/mousein class) and the replay-script format; trace capture mechanics on the donor (the 05-03 rig lineage re-keyed); trace file layout under the phase dir.
- Functional-probe script internals per service (fixed inputs, marker vocabulary, loopback ports) — semantics locked by D-02/D-08.
- The ndb/local template's home + placeholder token syntax (D-16's seam constraint locked); bootfs staging mechanics for fonts/profile (committed staged tree vs bake-time copy — the adm/timezone bake-time-copy precedent is available).
- CI job topology: tier compile gates in CI; donor/r9 execution legs follow the Phase-14 venue precedent (k10t1 self-provisioned runners; nix-driven x64 gates) — planner decides per-leg placement.
- Evidence-file organization under `.planning/phases/15-*/`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 15 — goal, success criteria 1–4, requirement mapping; §Phase 16 (parallel), §18/§19 (payload consumers)
- `.planning/REQUIREMENTS.md` — USPACE-07/08/09 + DESK-03 full text; Out of Scope table
- `.planning/PROJECT.md` — Key Decisions (tier machinery, venue honesty, push-before-pin, FOD discipline rows), Constraints
- `.planning/STATE.md` — Accumulated Context (Phase 13/14 decisions incl. the 14-09 ipconfig-SKIP-stale note)

### The machinery this phase rides (READ FIRST)
- `.planning/phases/14-harvey-amd64-userspace-on-r9/14-CONTEXT.md` — D-03 mb2 module bootfs, D-05 compose split, D-06 gates-by-artifact, the tier/net venue shape this phase extends
- `build/x86_64/tiers/tier0-libs.txt` + `tier1-cmds.txt` — entry-form grammar, keep-rule, append-only discipline the new tier files inherit (READ the header comments)
- `build/x86_64/mkimage.sh` — the four existing bake contracts (marker/chain/tier/net) the fifth joins; the root-tree skeleton D-13 stages into
- `build/x86_64/tier1-sweep.rc` + `tier1-r9.expect` + `subset-build.sh` — the sweep/gate/driver shapes the services sweep extends
- `build/mm/build-9k10t1.sh` + `boot-9k10t1-sweep.sh` — the k10t1 donor venue (NO srv9, NO 9front tree — trace-rig premises must not assume the aarch64 harness channels)
- `build/aarch64/tier1-sweep.rc` — the donor venue's sweep (byte-preserved in .tier1-stage)

### The v2.0 Phase-10 ancestor (mechanism source — premises superseded)
- `.planning/milestones/v2.0-phases/10-userspace-services-gui-tiers-bootfs-payload/10-CONTEXT.md` — the aarch64-keyed decisions this context reconciles against (D-03 probes, D-05 auth subset, D-10 scripted traces, D-13/14/15 payload carried; D-04 donor-only, D-08/D-09 draw-over-9P superseded/deferred per this context's reconciliation notes)
- `.planning/milestones/v2.0-phases/10-userspace-services-gui-tiers-bootfs-payload/10-RESEARCH.md` — HIGH-confidence research (probe semantics per service, gs/devdraw evidence chains, trace-rig mechanics) — Open Questions 1–3 were resolved aarch64-side by the ancestor; re-verify the amd64 premises
- `.planning/milestones/v2.0-phases/10-userspace-services-gui-tiers-bootfs-payload/10-0{1..10}-PLAN.md` — never-executed aarch64 plans; wave-shape reference ONLY

### Phase 6 — the tier machinery origin
- `.planning/milestones/v2.0-phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-CONTEXT.md` — D-08 exclusions-by-name, D-09..D-11 membership+sweeps, D-13/D-14 allowlist-data+ledgers, D-16 CI default; the Carried-forward list
- `.planning/milestones/v2.0-phases/06-full-tree-userspace-wave-0-libraries-core-tiers/deferred-items.md` — the Phase-10-filed checklist items this phase inherits (drawtest/arctest, stale test targets, libdynld, rc argv residual, tier anti-staleness)
- `build/aarch64/markers/tier0.ledger` + `tier1.ledger` — ledger line grammar
- `.planning/milestones/v1.0-phases/05-shell-boot-one-command-integration/05-CONTEXT.md` — the 05-03 measure-first syscall-trace doctrine D-06's table format follows

### Semantics references (in-tree)
- `sys/src/9k/k10/k10cpu` — the donor device set the draw variant extends (root/cons/env/pipe/proc/mnt/srv/dup/rtc/ssl/tls/cap/kprof/acpi/ether/ip/uart — NO draw/vga/mouse today)
- `sys/src/9k/port/devdraw.c`, `sys/src/9k/k10/devvga.c` — the config-gated draw substrate D-04 compiles in
- `sys/src/9k/port/devcons.c` — the synthetic-input file surface (kbin class) D-05's replay depends on (researcher verifies)
- `sys/src/cmd/{ip,auth,aux,ndb,rio,acme,sam,samterm,plumb,upas,lp,gs,fossil,venti}` — the tier sources; note samterm is a SIBLING dir of sam, not under it
- `sys/src/cmd/gs/mkfile` + `sys/src/cmd/gs/default.amd64.h` + `sys/src/ape/lib/ap/amd64/` — the verified stock-amd64 gs build route (no vendoring needed)
- `sys/src/cmd/rio/rio.c:162-165` (default font lucm/unicode.9), `sys/src/cmd/acme/acme.c:40-41` (lucidasans/euro.8 + lucm fallback) — GUI font defaults
- `usr/glenda/lib/profile` — the D-17 trim reference
- `lib/font/bit/` — the 36MB D-14 payload
- `sys/man/8/{listen,exportfs,cs,dns,authsrv}`, `sys/man/4/keyfs` — the functional-probe semantics for the named services
- The netaddr seam (14-09 D-11; researcher verifies the x64 home — `netaddr.rc` shape) — D-16's substitution source

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- The whole Phase-14 x64 substrate: subset-build driver, mkimage contracts, tier sweep/expect gates, k10t1 venue scripts, nix gate wiring — tiers 2/3/4 are pure appends plus one new image contract
- 9k's config-gated devdraw/devvga — exists in-tree, just not compiled into k10cpu; a config variant away from a draw donor venue
- The 05-03 measure-first trace rig lineage (born-traced arming on the donor) — the GUI trace capture's ancestor
- The v2.0 P10 research + plans — mechanism source with superseded premises (the reconciliation deltas are recorded in the decisions above)

### Established Patterns
- Sourced-not-typed sweeps from termrc; serial markers; presence-based, timing-tolerant gate assertions
- Single-source-as-data: tier lists, ledgers, manifests, ndb template — data files gates can diff
- Execution-gated ordering: a dir enters the default build only after its per-tier gate passes
- Venue honesty: named venues in ledgers; named SKIPs over unearned OKs; the SKIP-whitelist converse assert
- Atomic class-sweeps for compile-failure fixes (one commit per bug class)
- mkimage bake-time asserts as mechanical policy (the net-section literal grep — D-16's template must pass it)

### Integration Points
- `build/x86_64/tiers/tier{2,3,4}-*.txt` (new data) + the mk filter consumers already hooked (sys/src/mkfile CMDS/SUBSYS, cmd/mkfile TARG/DIRS, cmd/ip + cmd/auth + cmd/auth/factotum container mkfiles)
- `build/x86_64/mkimage.sh` — fifth (services) contract + the root-tree skeleton payload + manifest consumption
- `build/x86_64/services-sweep.rc` (new) + its per-venue expect gates; the 14-09 net bring-up sequence reused
- The draw donor config (sys/src/9k/k10 sibling config) + its build/mm venue leg + the trace rig
- `.github/workflows/` — compile-gate CI legs per the planner's topology decision

</code_context>

<specifics>
## Specific Ideas

- **The acceptance story, end-to-end:** tier-2/3/4 data files append (services: full ip/ tree + ndb + aux/listen + mntgen + exportfs + auth suite; GUI: libdraw family + rio/acme/sam+samterm/plumb + the stats/winwatch/page/proof/screenlock class; tail: upas + lp + the rest + fossil/venti compile-gated + gs) → the mk sweep compiles each tier fresh with artifact markers, the full DIRS-coverage proof asserting every cmd dir green → the services image bakes (net bring-up + daemons + probes) and its sweep runs: cs answers, dns resolves, exportfs mounts, listen accepts, mntgen serves, factotum/keyfs/authsrv start+answer — first on k10t1, then on r9-x86_64 (in-phase push-before-pin fixes for every surfaced kernel gap; 18-substrate entries SKIP→18 by name) → the draw donor venue boots (new 9k config variant, qemu -vga std), the replay scripts drive rio/acme/sam sessions under the trace rig, raw logs + distilled syscall tables commit → gs compiles stock through pcc and renders the .ps fixture to a bitmap file on the donor → DESK-03: the whole 36MB font tree + trimmed profile + substituted ndb template stage in the root-tree skeleton, the manifest data file + bake assert green, the 8× bootfs growth headroom checked EARLY → ledgers append with named venues; evidence consolidates with r9 SHAs recorded.
- **Watch-out:** the 36MB font payload makes bootfs roughly 8× — verify bake + boot memory headroom in the DESK-03 plan's early task (the ancestor measured this exact consequence).
- **Watch-out:** libmemdraw's drawtest/arctest faults are arch-independent LP64 pixel-path bugs — the GUI tier compiled green in Phase 6 but is NOT proven correct; audit before/with the trace capture or the traces may record garbage pixels.
- **Watch-out:** donor vga bring-up under qemu q35 (-vga std) is real kernel-config work (vesa/framebuffer path) — the researcher sizes it before the planner sequences the trace wave.
- **Watch-out:** tier anti-staleness fires at the FIRST mass tier-2 wave (06-03 open item) — decide the mechanism in the first tier-2 plan, not mid-wave.
- **Watch-out:** replay determinism rides the synthetic-input mechanism, not timing — the scripts are committed data, the gate asserts script COMPLETION markers, and TCG timeouts stay generous.
- **Watch-out:** the k10t1 venue has NO srv9/9front channels — do not assume the aarch64 harness's host-serving paths anywhere in the x64 rig.
- **Watch-out:** amd64 exclusion classes are re-derived, not inherited — what aarch64 excluded as "x86-hardware-only" compiles natively here; carry nothing over unexamined.
- **User overrides to honor downstream (deliberate, don't second-guess):** fix-everything over hybrid/All-SKIP (D-01 — with the 18-owned boundary kept); kernel-local venue now + 9P topology deferred over both the pure-today read and the v2.0 flip (D-04); scripted traces ADOPTED over today's startup-to-idle (D-05); gs render smoke over roadmap-literal compile-only (D-11); whole 36MB font tree (D-14); payload in the shared skeleton (D-13).

</specifics>

<deferred>
## Deferred Ideas

- **The draw-over-9P topology rig** — drawterm-derived serve-only host binary (x11 on NixOS per the ancestor's D-11b note), a host-serving channel on the venue, and the topology re-capture of the GUI traces: belongs to the phase that grows r9's desktop path (post-v2.1 desktop hardening or a dedicated phase); the design rationale is on record in 10-CONTEXT D-08/D-09
- Phase-18 auth substrate: devcap `#¤`, bake-time `/adm/keys` domain generator, keyfs-as-real-service, real p9sk1 handshakes, secstore — Phase 18 (D-01 boundary, D-08's shallow subset stops short)
- `#!` exec, service=cpu bootconf, aux/listen live at boot, drawterm flake app, rio in-session — Phase 19 (consumes this phase's payload + binaries)
- fossil/venti runtime verification — future milestone (needs a block device in r9, e.g. virtio-blk)
- upas send/receive UAT; lp gs-dependent stages — carry the ancestor's PITFALLS exclusions unless the amd64 re-derivation changes them
- Hosted-CI donor-execution legs — planner discretion per the venue precedent; revisit if the manual/local discipline rots
- virtio-blk/rng/9p host mount, SMP — future milestones (carried)

</deferred>

---

*Phase: 15-Userspace Tiers + Bootfs Payload (amd64)*
*Context gathered: 2026-11-09*
