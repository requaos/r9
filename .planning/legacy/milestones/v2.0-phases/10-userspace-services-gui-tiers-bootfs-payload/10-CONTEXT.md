# Phase 10: Userspace Services + GUI Tiers + Bootfs Payload - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning

<domain>
## Phase Boundary

The services and GUI tiers ride Phase 6's machinery. Tier-2 (services): the full `sys/src/cmd/ip/` tree, `ndb/` (cs+dns), `aux/listen`, `mntgen`, `exportfs`, and the `auth/` suite (~80 files) compile for aarch64 via append-only allowlist growth, and the roadmap-named set *executes functionally* on the 9front donor kernel with committed markers (USPACE-07). Tier-3 (GUI): rio, acme, sam+samterm, plumb (+ stats/winwatch/page/proof/screenlock class) compile for aarch64, and donor-kernel syscall traces for rio/acme/sam+samterm are captured measure-first — the input for any r9 kernel growth the desktop needs (USPACE-08). Tier-4 (long tail): upas, lp, games, and the remaining cmd dirs compile; fossil+venti are compile-gated and documented runtime-unverifiable this milestone (no block device in r9); gs (753 files) is included, compile-gated, fix-or-exempt, plus one headless donor render smoke — unblocked by vendoring the 9front ape arm64 slice as `sys/src/ape/lib/ap/aarch64/` (D-01a) (USPACE-09). Bootfs payload: `/lib/font/bit/*` (full set), a glenda-style `/usr/glenda/lib/profile`, and a `/lib/ndb/local` template are staged into bootfs with mkimage manifest assertions, so Phases 11–12 never block on payload (DESK-03).

Work happens in this repo (Harvey). r9 is untouched this phase — no r9 execution venue, no kernel growth; the GUI traces are *input* for later phases. Addresses USPACE-07, USPACE-08, USPACE-09, DESK-03.

NOT in this phase: r9 execution of tier-2/3 binaries (Phase 11 runs the auth suite on r9; Phase 12 runs the full chain — D-04); real p9sk1 handshakes, /adm/keys, ndb domain content, keyfs-as-real-service (Phase 11); `#!` exec, service=cpu bootconf, aux/listen live at boot, drawterm, rio actually starting in a session (Phase 12); host netdev modes (Phase 9); cs/dns/ndb gates on r9's /net (Phase 8 owns /net; Phase 8's gates are numeric-only); fossil/venti runtime (no block device — compile-gated only, per roadmap); ape (excluded by milestone decision); upas send/receive UAT (PITFALLS: build-only this milestone); lp's gs-dependent stages (PITFALLS exclusion); gs fonts in bootfs (GPL island — DESK-03 fonts come from `lib/font/bit` only).

</domain>

<decisions>
## Implementation Decisions

### gs (ghostscript) disposition — the tension Phase 6 deferred here
- **D-01:** **gs is INCLUDED in the tier-4 long-tail sweep as a compile-gated member, fix-or-exempt** (roadmap criterion 3's literal reading, chosen over PITFALLS' exclude-and-document recommendation and over a timeboxed attempt). Fixes land one commit per bug class (the v1.0 atomic-sweep convention); anything exceeding the fix budget is excluded by name with a committed comment (Phase 6 D-08 pattern). Verified at discussion time: gs's mkfile needs `$objtype.h` — either `genarch` run on a matching CPU or a committed `default.$objtype.h` (tree carries `default.386.h`/`default.alpha.h`; **no `default.aarch64.h` exists — it must be authored or harvested**); the `genarch` generator is already pcc-native-guarded (06-04 generator-audit G6), so the Pitfall-2 class is pre-cleared.
- **D-01a (AMENDMENT, research escalation OQ-1, user decision 2026-11-05):** **Vendor 9front's ape `arm64` slice as `sys/src/ape/lib/ap/aarch64/`** — chosen over exempt-gs-by-name and over a gs-to-7c native-port attempt. Research verified gs is an APE program (`gs/mkfile:126-129` sets `CC=pcc LD=pcc` → `/aarch64/lib/ape/libap.a`) and `sys/src/ape/lib/ap/` has 386/amd64/arm/mips*/power*/riscv*/sparc* dirs but **no arm64/aarch64** — the stock gs build is structurally dead, not bug-class-broken. The 9front donor HAS arm64 ape (`donor sys/src/ape/lib/ap/arm64/`: main9.s, memmove.s, memset.s, lock.c, cycles.s, getfcr.s, _seek.c, notetramp.c — verified). **Scope discipline:** the milestone's ape exclusion stands for the ape *environment* (the ~200-program POSIX port); this vendoring is a toolchain-level enabler scoped to the pcc class, vendor-and-owned at the donor pin with a per-dir provenance README (Phase 2 D-10 pattern; 9front is MIT — compatible). It unblocks the whole committed-red pcc class (`compress, eqn, grap, gs, lp's lpdaemon/lpdsend, postscript(+6), spin, troff, games/mp3enc` per `build/aarch64/dir-coverage.txt`); each dir remains fix-or-exempt per D-01 after the link wall is gone. The donor's `default.arm64.h` (genarch-produced on real 9front arm64) is vendored as `default.aarch64.h` in the same wave. — **Reversibility:** costly — the tier-4 pcc-class dirs and gs's render smoke all key off the vendored libap.a existing; removing it later re-walls the class.
- **D-02:** **gs gets a headless donor render smoke** beyond its compile marker (user choice, stronger than roadmap-literal compile-only): `gs` renders a tiny checked-in `.ps` fixture to a bitmap *file* device (no display involved) on the donor; gs's `.ps` init/lib files are staged into the guest (srv9 writable-prefix precedent). Rationale: 753 files of ported unix C are exactly the Pitfall-3 LP64-fossil habitat — compile-green would miss them, and the render smoke is the cheap net. **Survives D-01a's vendoring route** (the smoke needs a linkable gs, which the vendored ape slice provides). lp's gs-dependent stages stay excluded (PITFALLS); gs's GPL fonts are never staged anywhere (MIT-licensed `lib/font/bit` is the only font source this phase touches).

### Tier gate depth + r9 venue
- **D-03:** **The named services get FUNCTIONAL donor probes** — "runs" means demonstrably works, per the project's test-delivery-not-counters ethos (Phase 7 UAT lesson). Concretely: `ndb/cs` answers a scripted lookup; `ndb/dns` resolves through cs; `exportfs` serves a guest-side 9P mount; `aux/listen` accepts a scripted connection; `mntgen` mounts; `ipconfig` configures the donor's ether and `ping` reaches the host-side srv9 leg. All ride the Phase 6 sourced-not-typed in-guest rc sweep with per-service OK/FAIL serial markers. The donor runs a full 9front userspace (ethervirtio10 + ip stack proven in Phase 2, devcap present), so functional probes are possible without any r9 dependency.
- **D-04:** **Phase 10 execution gates are DONOR-ONLY.** r9 consumes these binaries later at the natural venues: Phase 11 executes the auth suite on r9 (its core gate), Phase 12 runs the full chain. No coupling to in-flight Phase 8 (4/9 plans at discussion time) — Phase 10 stays parallel per the roadmap's depends-on-Phase-6-only. Ledgers name `venue:donor` explicitly (Phase 6 venue-honesty convention: named venues, never an unearned marker).
- **D-05:** **The auth suite compiles complete (~80 files) plus a SHALLOW execution subset on the donor**: factotum starts and answers a scripted probe, keyfs serves over a test key file, authsrv starts and listens. Real p9sk1 handshakes, `/adm/keys`, ndb domain content, and the loopback probe are Phase 11's in-guest domain on r9 — deliberately NOT duplicated on the donor (chosen over compile-only and over a full donor p9sk1 probe).
- **D-06:** **The whole `sys/src/cmd/ip/` tree joins tier-2 as compile-gated members** (telnetd, pptpd, snoopy, httpd, imap4d, dhcpd, ftpfs, …), with the roadmap-named core (ipconfig, ping) getting the functional probes; x86-hardware-only or r9-subsystem-less dirs are excluded by name with comments (the `aux/vga`/`usbd`/`disk/prep` class per PITFALLS). Chosen over splitting ip/ across tier-2/tier-4.
- **D-07:** **Tier-4 long tail is compile-gated** per the roadmap criterion: upas + lp + games + remaining cmd dirs; fossil+venti compile-gated AND documented runtime-unverifiable this milestone (no block device in r9 — the roadmap's own words); games with per-arch asm (snes/gb class) and other unmigratable dirs are excluded by name with committed comments. upas is build-only (no send/receive UAT — PITFALLS); lp is build-only with gs-dependent stages excluded.

### GUI trace venue (USPACE-08)
- **D-08:** **rio/acme/sam traces are captured against a HOST-SERVED draw mounted over 9P into the donor guest** — chosen over a donor kernel config variant (softscreen) and over drawterm-into-donor. Verified at discussion time: the 9front donor's arm64 qemu kernel config has **no draw/mouse/screen device** (`dev` section: root/cons/env/pipe/proc/mnt/srv/shr/dup/tls/cap/fs/ether/ip/uart/usb/rtc/pci/sd); `devdraw.c` exists in the donor's `port/` but is not compiled in. Decisive argument: on r9 in Phase 12, `/dev/draw` arrives as a mounted 9P device from drawterm (r9 deliberately has no framebuffer) — so a kernel-local softscreen would measure the WRONG topology (no devmnt RPCs in the trace); draw-over-9P is exactly the syscall mix r9 must grow for. Donor kernel stays pristine; the existing srv9/guestfwd channel is the transport. — **Reversibility:** costly — the trace rig, the staged fonts, and the scripted-input machinery all key off the 9P-served-draw topology; switching to kernel-local draw later means re-capturing every trace.
- **D-09 (AMENDED, research escalation OQ-2, user decision 2026-11-05):** **A drawterm-derived serve-only binary serves `/dev/draw` on the host** — amends the named tool, not the topology. Research verified plan9port's `devdraw` does NOT serve 9P (its own man page BUGS: "Devdraw should probably present a standard 9P server instead of using its own protocol"; it speaks drawfcall/Wsysmsg over a pipe, exec'd by client libdraw — unmountable). The substitute is drawterm's `kern/dev{draw,mouse,kbd,cons}.c` + `gui-cocoa` built as a serve-only host binary — the exact 9P-served draw device tree Phase 12 ships, zero new protocol design, proven against real Plan 9 libdraw clients, cocoa on the darwin dev machine. Chosen over a devdraw pipe-to-9P bridge shim (more moving parts, unproven) and over reopening D-08.
- **D-10:** **Traces are SCRIPTED-INTERACTION, not launch-settle-exit** — a fixed short script per program drives real interaction (rio: menu New, type into the new rc window; acme: type text, execute a command, quit; sam+samterm: type, edit, quit). **Input injection mechanism (amended by OQ-2): server-side replay of a committed event script** in the drawterm-derived server (deterministic under TCG; no OS-level dependency); osascript/cliclick OS-level injection stays an optional eyeball leg (cliclick is not installed). Rationale: USPACE-08's own parenthetical — libthread programs exercise unmeasured kernel corners; the event loop, mouse/kbd read loops, snarf, and sustained draw traffic ARE the measurement. Chosen over the cheaper/flakier-resistant settle-only trace and over a two-stage landing.
- **D-11:** **The GUI trace leg is darwin documented-manual ONLY** (user choice — a deliberate deviation from Phase 6 D-16's everything-in-CI default for this leg, following the Phase 8 D-16 soak-override precedent): cocoa draw-server on the dev machine, the exact command documented in the phase evidence. **Extended by D-11a below.** — **Reversibility:** reversible — a linux Xvfb CI leg can be added later if the manual discipline rots; the trace artifacts and scripts are venue-independent.
- **D-11a (AMENDMENT, research escalation OQ-3, user decision 2026-11-05):** **ALL tier-2/3 donor-execution gates run documented-manual on aarch64 dev hosts (darwin via lima) this phase** — not in hosted CI. Hosted CI carries only the tier compile gates. Rationale: G-06-1/CR-01 retired the hosted 9front donor leg ("9front stays aarch64-local donor only"), and these gates boot the arm64 donor — a hosted x86_64 leg would be the retired shape (qemu-system-aarch64 TCG), and hosted aarch64 runners (ubuntu-24.04-arm) are a new unproven CI dependency in tension with the retirement's spirit. Consistent with D-11's GUI posture and the 05-13 documented-manual + PENDING-FIRST-PUSH precedent. Hosted execution legs can be added once proven. — **Reversibility:** reversible — additive CI legs later; the gate machinery is venue-independent.
- **D-11b (VENUE NOTE, 2026-11-06 host migration — not a decision change):** the primary dev host is now **NixOS linux-x86_64** (`.planning/HOSTS.md`). Read D-11/D-11a's "darwin documented-manual" as "**primary-host documented-manual**": the GUI trace leg's draw server uses drawterm's **gui-x11** (X11/Xwayland on NixOS) instead of gui-cocoa — same code, other backend; the donor gates run on **native linux** (no lima — `subset-build.sh`'s NATIVE_LINUX leg). Additionally, the migration surfaced a possible **x86_64 target pivot** (see PROJECT.md Context + HOSTS.md) — if the milestone re-scopes, this phase's plans are aarch64-keyed and would be re-planned; the decisions' *mechanisms* (tier machinery, probe shapes, trace rig, payload staging) are arch-parameterized and carry over.
- **D-12:** **plumber gets a FUNCTIONAL headless donor probe** (the one tier-3 GUI binary that needs no draw): starts, a scripted plumb message routes to a registered port, clean shutdown. Pre-validates DESK-05's dependency before Phase 12's drawterm session discovers a broken plumber mid-desktop. The rest of tier-3 (rio, acme, sam, samterm, stats, winwatch, page, proof, screenlock) is compile-gated; rio/acme/sam+samterm additionally get the D-08 trace.

### Bootfs payload (DESK-03)
- **D-13:** **The FULL `lib/font/bit` tree stages into bootfs — all 36MB, no exclusions** (user choice over the ~3MB minimal desktop set lucm+pelm+lucidasans and over a ~9MB Latin+common set). Verified sizes at discussion time: shinonome 13MB, symbola 6MB, lucida 5MB, gb 4MB, the CJK block ~22MB total; current bootfs.paq is 4.0MB, so the initrd grows roughly 8× (paqfs compresses; r9 boots `-m ≤ 1G`). Consequence accepted by the user. Every app finds every font; no app-font exclusion bookkeeping. gs's GPL fonts are untouched by this (they are not `lib/font/bit`). — **Reversibility:** reversible — the payload manifest can be narrowed later with a manifest-comment diff; nothing downstream keys off specific fonts beyond the staged set being a superset.
- **D-14:** **`/usr/glenda/lib/profile` stages as the FULL cpu-aware glenda reference, trimmed**: `upas/fs` and `news` dropped (upas is long-tail compile-only; news has no payload), the `case terminal` path keeps its binds/tmp/font but **loses the `exec rio`** (the guest console is serial — no draw at boot; Pitfall 14/DESK-02: rio must provably NOT start at boot), and the `case cpu` path is intact (binds `/mnt/term/dev`, `font=/lib/font/bit/pelm/latin1.8.font`, `exec rio` when called from drawterm). Phase 12 consumes the profile unchanged. In-tree reference verified: `usr/glenda/lib/profile`; rio's default font is `/lib/font/bit/lucm/unicode.9.font` (`sys/src/cmd/rio/rio.c:165`), acme's is `lucidasans/euro.8.font` with lucm fallback (`sys/src/cmd/acme/acme.c:40-41`), samterm uses `$font`.
- **D-15:** **`/lib/ndb/local` is a committed TEMPLATE with placeholder tokens, substituted at bake time from `build/aarch64/netaddr.rc`** — the Phase 8 D-11 addressing seam stays the single source (Pitfall 9's no-scattered-hardcodes rule); the mkimage manifest asserts the substituted result. Chosen over a static committed slirp-values file (drift-prone) and over full generation without a template (invisible to review).

### Phase-6-filed checklist items owned by this phase (already queued — no re-litigation)
- **libmemdraw pixel-path LP64/struct-punning audit**: drawtest's stochastic memimagedraw verification mismatch + arctest's wild-pointer write in memsetl (06-05, evidence in 06-EVIDENCE.md Phase-10 checklist) — fixed here; both tests rejoin the standing sweep when green.
- **Stale mkfile test targets** (liboventi `$O.vttest`/`test:` reference absent `vttest.c`; libmemdraw `$O.test` references absent `test.c`) — this phase decides: remove the targets or restore the sources.
- **libdynld dynld.c:128** "useless or misleading comparison: ULONG < 0" (7c note) — Pitfall-3 audit item.
- **rc argv-stack residual audit**: the 06-07 Xbackq fix's residual class DID eat markers in 06-09 r9 parity runs — audit rc's pushlist/poplist balance across every op.
- **tier cmd anti-staleness decision** (06-03 open item): per-entry `mk clean` in tier loops vs committed object sweep on toolchain change vs documented operator step — decide by the first mass tier wave this phase (tier-2 IS the first mass wave since the 06-03 7l fix class).

### Carried forward (locked in roadmap/prior phases — do not re-litigate)
- Tier machinery: append-only per-tier committed data files (`build/aarch64/tiers/tier2-services.txt`, `tier3-gui.txt`, `tier4-tail.txt` shape per Phase 6 D-13); per-tier append-only marker ledgers as the only gates (D-14); mk exit status never trusted; artifact+execution markers only; a dir enters the default build only after its per-tier gate passes.
- Gate mechanics: sourced-not-typed in-guest rc sweep + serial markers (Phase 6 D-10); timing-tolerant presence assertions with TCG-generous timeouts; expect-typed interaction does not scale.
- Donor-kernel execution FIRST (USPACE-04); measure-first trace discipline (v1.0 D-14/05-03) — the GUI traces ARE this phase's measure-first deliverable.
- Bake-into-bootfs for gates (deterministic), ufs-over-tcp for iteration; the `mkimage.sh` real-init rodata guard applies to all payloads; every baked file is a manifest assertion (Pitfall: staged-supply-chain pollution).
- CI: hosted CI carries tier compile gates only this phase; ALL donor-execution gates (tier-2/3 functional probes) and the GUI trace leg are documented-manual on aarch64 dev hosts (D-11/D-11a — G-06-1 consistency); every action 40-char-SHA pinned.
- Exclusions by name with committed comments, never silent drops (D-08); Pitfall-3 findings filed as per-dir checklists, not drive-by fixes; fix classes land one commit per class.
- `build/aarch64/out/` last-writer-wins ownership — tier builds and bakes share it; no third writer (CONCERNS.md; the mkimage real-init guard is the tripwire).
- `aarch64` naming everywhere; Plan 9 C style in Harvey (`sys/man/6/style`); NMOUNT=54 divergence ledger lives and grows (divergent syscalls wall loudly on the donor).
- Evidence consolidated in `.planning/phases/10-*/` with SHAs recorded; markers timing-tolerant (presence, never interleaving order).

### the agent's Discretion
- Exact per-tier membership order and wave decomposition within tiers 2/3/4 (planner's dependency sort; tier data files are the record). fgui's tier home (research recommends tier-2 compile-only).
- Functional-probe script internals per service (fixed inputs, marker vocabulary, port numbers on the donor's network leg) — the probe *semantics* are locked by D-03/D-05/D-12 and detailed in 10-RESEARCH.md.
- The vendored ape slice's mk wiring (how `/aarch64/lib/ape/libap.a` builds under the namespace build; the slice's exact file list from the donor pin) and `default.aarch64.h`'s vendored content (from the donor's genarch-produced `default.arm64.h`).
- GUI trace rig mechanics: the drawterm-derived server's build/packaging (host tool — where it lives in-tree), how the donor guest mounts it over the srv9/guestfwd channel, the committed event-script format for server-side replay, and trace-file format/commit layout.
- gs render-smoke fixture content and the bitmap-device choice (any file-output device; no display semantics).
- Physical layout/naming inside `build/aarch64/tiers/` + `build/aarch64/markers/` extensions and the mk filter consumption of the new tier files (Phase 6 D-13/D-14 homes).
- ndb/local template file home and placeholder token syntax (D-15's seam-substitution constraint locked).
- Bootfs staging mechanics for fonts/profile (committed staged tree vs bake-time copy — the Phase 5 adm/timezone bake-time-copy precedent is available).
- CI job topology within the phase workflow (fast compile legs vs slow donor boot legs; D-11's GUI exception noted).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/phases/10-userspace-services-gui-tiers-bootfs-payload/10-RESEARCH.md` — this phase's research (HIGH confidence; probe semantics per service, devdraw/gs wall evidence chains, trace-rig mechanics, tier machinery extension shape) — **MUST read; its Open Questions 1–3 are resolved by D-01a/D-09-amended/D-11a in this CONTEXT**
- `.planning/ROADMAP.md` §Phase 10 — goal, success criteria 1–4, requirement mapping; §Phase 11/12 for how r9 consumes this phase's binaries
- `.planning/REQUIREMENTS.md` — USPACE-07/08/09 + DESK-03 full text; Out of Scope table (ape, Go, gs-fonts-adjacent items); traceability
- `.planning/PROJECT.md` — Key Decisions (tier machinery row, venue-honesty row), Constraints
- `.planning/STATE.md` — Accumulated Context (Phase 6/7/8 execution decisions incl. rc residual-class 06-09 update)

### Phase 6 — the machinery this phase rides (READ FIRST)
- `.planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-CONTEXT.md` — D-08 (exclusions by name), D-09/D-10/D-11 (tier membership + sweep gates), D-13/D-14 (allowlist data + ledgers, both rated one-way), D-16 (CI default), the Carried-forward list
- `.planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/deferred-items.md` — the Phase-10-filed checklist items: libmemdraw drawtest/arctest faults, stale test targets, libdynld note, rc argv-stack residual (with the 06-09 marker-eating update), tier anti-staleness decision
- `.planning/phases/06-full-tree-userspace-wave-0-libraries-core-tiers/06-EVIDENCE.md` — the Phase-10 checklist evidence (drawtest/arctest fault logs)
- `build/aarch64/tiers/tier0-libs.txt` + `build/aarch64/tiers/tier1-cmds.txt` — entry-form grammar, keep-rule, append-only discipline the new tier files inherit (read the header comments)
- `build/aarch64/markers/tier0.ledger` + `tier1.ledger` — ledger line grammar

### v2.0 milestone research (the phase's design source)
- `.planning/research/SUMMARY.md` §"Phase 5: USERSPACE Services + GUI Tiers + Bootfs Payload" (research numbering) — the phase sketch: tiers ride Phase-1(research) machinery, GUI compile-gated with traces landing here, payload staged so later phases never block
- `.planning/research/PITFALLS.md` — Pitfall 1 (whack-a-mole + notorious-directories table: games/upas/lp/gs/aux-vga handling), Pitfall 2 (generator host-toolchain rule), Pitfall 3 (aarch64 bug-class catalog — the tier-4/gs audit patterns), Pitfall 5 (ape exclusion), Pitfall 9 (no hardcoded 10.0.2.x — D-15's motivation), Pitfall 14 (fonts missing from bootfs; rio started in the wrong script)
- `.planning/research/FEATURES.md` — GUI stack table (rio/acme/sam/plumb + the headless notes), gs row ("recommend gating: fix-or-exempt" — D-01 adopts it), the `/usr/glenda/lib/profile` guidance ("minus upas/plumber bits we defer" — D-14 trims upas; plumber is staged since DESK-05 needs it in Phase 12)
- `.planning/research/ARCHITECTURE.md` — the drawterm-session namespace walk (why draw-over-9P is the faithful trace topology, D-08)

### Semantics references (in-tree)
- `sys/src/cmd/gs/mkfile` — the `$objtype.h` rule (lines ~149-170): genarch-on-matching-cpu or `default.$objtype.h`; `default.386.h`/`default.alpha.h` are the shape references; genarch pcc-native guard confirmed by 06-04 G6
- `usr/glenda/lib/profile` — the D-14 trim reference (upas/fs, news, terminal-case `exec rio -i riostart`, cpu-case binds + `exec rio` with `pelm/latin1.8.font`)
- `sys/src/cmd/rio/rio.c:162-165` — rio default font `/lib/font/bit/lucm/unicode.9.font`
- `sys/src/cmd/acme/acme.c:40-41` — acme default `lucidasans/euro.8.font` + lucm fallback
- `sys/src/cmd/samterm/` — samterm lives as a SIBLING dir (not under cmd/sam); uses `$font` from the environment
- `sys/man/8/listen`, `sys/man/8/exportfs`, `sys/man/8/cs`, `sys/man/8/dns`, `sys/man/4/keyfs`, `sys/man/8/authsrv` — the functional-probe semantics for the named services
- `lib/font/bit/` — the full 36MB DESK-03 font payload (D-13)
- `build/aarch64/netaddr.rc` — the Phase 8 D-11 addressing seam; `/lib/ndb/local` template substitution source (D-15)

### Harness + supply chain (in-tree)
- `build/aarch64/README.md` — donor-kernel harness (the tier execution venue + the 05-03 trace rig lineage the GUI traces extend), qemu-11.1.2 drift accommodations, srv9 writable-prefix mechanics (gs .ps staging, D-02)
- `build/aarch64/{subset-build.sh,subset-exec.sh,mkimage.sh,mkpaqfs-host.sh}` — the tier drivers + bake harness the tier-2/3/4 waves extend; mkimage's real-init rodata guard + manifest assertion machinery (D-13/D-15 assert here)
- `.github/workflows/phase6.yml` — the CI shape this phase extends (new sibling or extension is planner discretion; 40-char-SHA pin rule)
- `flake.nix` — unchanged this phase (no r9 pin advance: no r9 work)

### Donor facts (verified at discussion/research time — re-verify only if the donor pin moves)
- The 9front donor arm64 qemu kernel config (`.build/9front-donor/sys/src/9/arm64/qemu`) has NO draw/mouse/screen device — devdraw.c/devmouse.c exist in its `port/` but are not compiled in. This is D-08's premise.
- The donor guest IS a native arm64 Plan 9 system — gs's `genarch` can legally run there.
- The donor carries `sys/src/ape/lib/ap/arm64/` (main9.s, memmove.s, memset.s, lock.c, cycles.s, getfcr.s, _seek.c, notetramp.c) and a genarch-produced `default.arm64.h` — the D-01a vendoring sources.
- plan9port's devdraw does NOT serve 9P (man page BUGS; drawfcall/Wsysmsg over a pipe) — the D-09 amendment's evidence. drawterm's `kern/dev{draw,mouse,kbd,cons}.c` + `gui-cocoa` DO serve the draw tree over 9P.
- `build/aarch64/dir-coverage.txt` records the pcc class red on aarch64: `compress, eqn, grap, gs, lp(partial), postscript(+6), spin, troff, games/mp3enc` — the D-01a blast radius.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Phase 6 tier machinery: `build/aarch64/tiers/` data files + `markers/` ledgers + the objtype-gated mk filters (`sys/src/mkfile.proto`, `cmd/mkfile`, container mkfiles cmd/ip, cmd/auth, cmd/auth/factotum) — tiers 2/3/4 are pure appends to this machinery
- `build/aarch64/subset-exec.sh` + the sourced in-guest rc sweep (tier1-sweep.rc shape) — the tier-2 functional probes extend it; the staged-termrc override (06-03) runs the sweep at boot without typed console interaction
- The 05-03 measure-first trace rig (born-traced arming + tagged fragments on the donor) — the GUI trace capture's direct ancestor
- `harness srv9 --writable-prefix` — guest staging for gs's .ps init/lib files and any probe fixtures (D-02)
- drawterm `kern/dev{draw,mouse,kbd,cons}.c` + `gui-cocoa` (host) — the D-09 (amended) draw server source; a serve-only binary is derived from it; zero new protocol design
- `build/aarch64/netaddr.rc` — the committed addressing seam (D-15's substitution source)
- `usr/glenda/lib/profile` — the D-14 trim reference, in-tree
- `lib/font/bit/` complete — D-13 stages it wholesale; no selection machinery needed

### Established Patterns
- Execution-gated ordering (riscv64 vaporware rule): a dir enters the default build only after its per-tier gate passes — never compile-only flag-flips
- Sourced-not-typed sweeps: the donor's devcons refuses typed console reads (Egreg by design) — every in-guest gate is a sourced rc script emitting serial markers
- Venue honesty: ledgers name their venue (`venue:donor` this phase); named T1-SKIP vocabulary over unearned OKs (06-09)
- Atomic class-sweeps: Pitfall-3 bug-class fixes land one commit per class, never interleaved with bring-up commits
- Single-source-as-data: tier lists, ledgers, netaddr.rc, and now the ndb template — data files CI can diff
- Phase-owned bootfs payloads keep tier allowlists pure (Phase 7 arp.req/netblast precedent); the mkimage manifest asserts every baked file
- No-drive-by: Pitfall-3 findings become per-dir checklist entries, not opportunistic edits mid-wave

### Integration Points
- `build/aarch64/tiers/tier{2,3,4}-*.txt` + `markers/tier{2,3,4}.ledger` (new) — D-13/D-14-pattern data homes
- `sys/src/cmd/ip/mkfile`, `sys/src/cmd/auth/mkfile`, `sys/src/cmd/ndb/` — the tier-2 container mkfiles the existing filter hooks already know how to read (06-02 keep-rule: container form `cmd/ip/ipconfig`)
- `build/aarch64/subset-exec.sh` + a new tier-2 sweep rc — functional probes land here
- The donor guest's network leg (ethervirtio10 + srv9 hostfwd) — where ipconfig/ping/cs/dns probes get their addresses (from netaddr.rc's slirp leg)
- Host drawterm-derived draw server ↔ guest mount over the srv9/guestfwd channel — the D-08/D-09 trace topology
- `build/aarch64/mkimage.sh` — DESK-03 payload staging + manifest assertions; `build/aarch64/mkpaqfs-host.sh` — the paqfs bake
- `.github/workflows/` — the tier compile-gate CI legs; donor-execution gates + GUI trace excluded per D-11/D-11a

</code_context>

<specifics>
## Specific Ideas

- **The acceptance story, end-to-end:** the vendored ape aarch64 slice + `default.aarch64.h` land first (the pcc-class link wall falls; D-01a) → tier-2/3/4 data files append (full ip/ tree, ndb, aux/listen, mntgen, exportfs, auth suite; rio/acme/sam/samterm/plumb + friends; upas/lp/games/the-rest + fossil+venti compile-gated + gs) → the mk sweep compiles each tier fresh (per-tier ledger artifact markers) → the tier-2 donor sweep runs functional probes: cs answers, dns resolves, exportfs mounts, listen accepts, mntgen serves, ipconfig configures + ping reaches the host leg, factotum/keyfs/authsrv start+respond, plumber routes a scripted message — per-service OK markers over serial, committed (documented-manual on aarch64, D-11a) → the GUI trace day on the darwin dev machine: the drawterm-derived server up, donor guest mounts it as /dev/draw, scripted rio/acme/sam+samterm sessions run under the trace rig with server-side event-script replay, traces committed (documented-manual, D-11) → the gs long-tail leg: gs compiles through its stock pcc route against the vendored libap.a, the tiny .ps render smoke passes on the donor with staged init files → DESK-03: full 36MB fonts + trimmed cpu-aware glenda profile + substituted ndb/local baked, manifest asserts green → ledgers append `venue:donor`; compile-gate CI legs green; EVIDENCE.md consolidates.
- **Watch-out:** the gs fix budget is a *valve*, not a blank check — fix-or-exempt per bug class, each exemption a committed comment; if the class count balloons, the planner stops fixing and starts exempting (D-01).
- **Watch-out:** 753-file gs + ~40-dir ip/ + 80-file auth in one phase — wave decomposition is the planner's, but per-tier gates must stay green between waves (execution-gated ordering; mid-phase instability must not red unrelated gates).
- **Watch-out:** the donor's network is the srv9/guestfwd leg — probe scripts take ALL addresses from `netaddr.rc`; a hardcoded 10.0.2.x anywhere is a Pitfall-9 red.
- **Watch-out:** devdraw-over-9P means libdraw's client side runs against a *mounted* tree — rio's startup does `/dev/draw/new` → ctl handshake; if the mount is wrong the trace measures a failure path. The trace rig must assert rio actually reached its event loop (a marker in the scripted session), not merely that syscalls occurred.
- **Watch-out:** TCG timing for the functional probes — presence-based marker assertions with generous budgets (v1.0/Phase-6 discipline); functional probes are more failure-prone under TCG than compile gates.
- **Watch-out:** the full 36MB font payload makes bootfs.paq ~8× larger — verify the paqfs bake + r9 boot memory headroom (`-m ≤ 1G`) in the DESK-03 plan's early task, not at the end.
- **Watch-out:** tier anti-staleness (06-03 open item) fires HERE — tier-2 is the first mass cmd wave since the toolchain fixes; the chosen mechanism (per-entry `mk clean` vs committed sweep vs operator step) must be decided in the first tier-2 plan, not discovered mid-wave.
- **Watch-out:** libmemdraw's drawtest/arctest faults are LP64 pixel-path bugs — the GUI tier's libdraw family compiled green in Phase 6 but is NOT proven correct; the D-08 traces will exercise libmemdraw hard. Budget the audit (Phase-6-filed checklist) before or alongside the trace capture, or the traces may record garbage pixels.
- **User overrides to honor downstream (deliberate, don't second-guess):** gs included compile-gated over PITFALLS' exclude recommendation (D-01); **vendoring 9front's ape arm64 slice over exempting gs** (D-01a — narrows the milestone's ape exclusion deliberately: the ape *environment* stays excluded, the vendored slice is a toolchain enabler for the pcc class); gs render smoke stronger than roadmap-literal compile-only (D-02); functional probes over smoke for services (D-03); drawterm-derived draw server over the plan9port devdraw named in discussion (D-09 amended — devdraw's no-9P premise failed verification); server-side event-script replay for input injection (D-10 amended); darwin-manual GUI trace leg AND documented-manual donor-execution gates over the D-16 everything-in-CI default (D-11/D-11a — G-06-1 consistency); full 36MB fonts over curated subsets (D-13); auth shallow-subset over both compile-only and full-donor-p9sk1 (D-05).

</specifics>

<deferred>
## Deferred Ideas

- r9 execution of tier-2 services (ipconfig/ping/cs/dns over r9's /net) — Phase 8 completes /net; Phase 11/12 consume the binaries on r9 naturally (D-04)
- Real p9sk1 handshakes, `/adm/keys` bake-time domain generator, keyfs as the real key service, ndb domain content, secstore — Phase 11 (D-05's shallow subset deliberately stops short)
- `#!` exec, `service=cpu` bootconf, aux/listen live at boot on r9, drawterm flake app, rio actually starting in-session, devssl — Phase 12 (the staged profile/fonts/ndb are this phase's contribution to that)
- fossil/venti runtime verification — future milestone (needs a block device in r9, e.g. virtio-blk)
- upas send/receive UAT; lp gs-dependent stages — not this milestone (PITFALLS)
- A linux-CI Xvfb GUI trace leg — D-11 rejected for now; revisit if the darwin-manual discipline rots
- Hosted CI donor-execution legs (x86_64 TCG aarch64 or ubuntu-24.04-arm runners) — D-11a rejected for now; revisit once the donor gates are proven locally and G-06-1's spirit is re-weighed
- plan9port devdraw + a pipe-to-9P bridge shim — rejected (D-09 amended: devdraw cannot serve 9P at all; drawterm-derived server chosen). Rust harness fake-draw server — rejected earlier (D-09 discussion)
- virtio-rng, virtio-blk, virtio-9p host mount, SMP/PSCI, RTC — future milestones (carried)
- darwin host sbrk 4096KB cap (06-01 deferred) — platform fact; relevant only if a gethunk-class host gate is needed natively on darwin

</deferred>

---

*Phase: 10-Userspace Services + GUI Tiers + Bootfs Payload*
*Context gathered: 2026-11-05*
