# Phase 19: CPU Service + Drawterm + rio Desktop - Research

**Researched:** 2026-11-14
**Domain:** Plan 9 cpu-server protocol (cpu -R remoteside, aux/listen, #! exec) + drawterm client (vendored 4e lineage) + r9 kernel surface (devssl `#D`, WSTAT/FWSTAT/REMOVE, AcceptPool TCP fix, tick_check) + nix flake composition
**Confidence:** HIGH (in-repo donors read verbatim; two MEDIUM web-verified darwin facts; three flagged assumptions)

## Summary

This phase is almost entirely **in-repo composition**: every protocol component already exists as a Harvey donor or a proven Phase 14-18 artifact. The cpu-server flow is verified end-to-end at the source level: `sys/src/cmd/cpu.c`'s `remoteside()` does p9any server auth via `auth_proxy(0, nil, "proto=%q role=server %s")` + `auth_chuid` (which writes `#¤/capuse` — devcap already landed in r9 at Phase 18), then `mount(fd, -1, "/mnt/term", MCREATE|MREPL, "")`, then opens `/mnt/term/dev/cons` as fd 0/1/2 and `execl("/bin/rc", "rc", "-li", nil)`. The listener side is equally concrete: `aux/listen`/`listen1` `exec()` the service file `tcp17010` (donor verbatim: `#!/bin/cpu -N -R` at `sys/src/9k/k10/root/tcp17010`; this phase's CONTEXT names `#!/bin/cpu -R`), and the kernel's `#!` pre-pass in 9k `sysproc.c:282-324` supplies interpreter + flags. The r9 gaps are precisely enumerated: `DISPATCH[REMOVE/WSTAT/FWSTAT]` are loud-`None` today, no devssl, AcceptPool members keep stock smoltcp defaults (the CR-03 fix is literally adding the same two `set_*` calls that `tcp_socket` already carries), and x86_64 never calls `tick_check`.

**Two hard findings need early attention.** First, **D-03's darwin backend cannot work as written**: the vendored `gui-osx/screen.c` is Carbon/QuickDraw (`#include <Carbon/Carbon.h>`, `#include <QuickTime/QuickTime.h>`, `QDBeginCGContext`) with `-m32` in `Make.osx` — Carbon was never 64-bit and is unusable on aarch64-darwin [CITED: eclecticlight.co/2023/06/30/deprecation + Apple docs]; a 64-bit native backend means porting 9front's `gui-cocoa` class frontend into the vendor (MIT-compatible per PROJECT constraints). Second, **DESK-01's `-O` flag does not exist anywhere in the vendored drawterm** (grepped `cpu.c`, `cpu-bl.c`, `README`) — like `-G`, it must be defined as a local vendor addition, and its semantics are the planner's to pin (an explicit Open Question below).

**Primary recommendation:** Sequence the r9 kernel work first (AcceptPool fix → `#!` exec → WSTAT/FWSTAT/REMOVE → devssl → tick_check, each with host pins/boot profiles per 14-D-06), then the FOD re-wire + pin advance as its own commit, then the seventh mkimage contract (§15) composing chain + auth suite + `cmd/cpu` + service files + fonts + re-keyed cpurc, then the drawterm package/wrapper/`-G`, then gates and the human legs. Resolve the darwin-backend decision (OQ-1) at plan time — it changes the DESK-07 darwin recipe materially.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions (verbatim from 19-CONTEXT.md ## Implementation Decisions)

- **D-01:** **The in-tree vendored drawterm at `sys/src/cmd/unix/drawterm/` is the source of record** — the classic Plan 9 4e lineage carrying Harvey-era fixes (history back to the 2000 3e import; Harvey fixes like 5c87831c6; upstream maintenance by 0intro at github.com/0intro/drawterm is NOT re-vendored). The vendor already has everything the phase needs: p9any→p9sk1 client auth in `cpu.c`, the `-e` encryption flag (DESK-04's `-e clear` rides the existing surface), `gui-x11` + `gui-osx` backends, Unix Makefiles. This IS the provenance decision DESK-01 requires recorded. — **Reversibility:** costly.
- **D-02:** **Nix builds the in-tree vendor — no flake-input fetch.** A nix package recipe compiles `sys/src/cmd/unix/drawterm` directly (the 9cc / 9front-toolchain vendored-at-pin precedent), so the build is hermetic with no network fetch, and local modifications (`-G`, any interop fixes) land as normal Harvey commits on the vendor — no patchPhase indirection. The flake `drawterm` app wraps this package for linux.
- **D-03:** **darwin's backend is native Cocoa (`gui-osx`)** built for aarch64-darwin — no XQuartz dependency on the darwin leg. The `Make.osx-x11` path exists but is not part of this phase's surface. Per D-06 the darwin build is documented-manual, so Cocoa is exercised through the README recipe, not a flake output.
- **D-04:** **`-G` is a no-window stdio mode**: drawterm connects, authenticates, mounts /mnt/term, and the session's stdio rides the invoking terminal — real terminal semantics, no offscreen buffer machinery. The DESK-06 gate drives a scripted login → shell marker through exactly this path. The flag is a small local addition to the vendor's `cpu.c` arg parsing (D-01/D-02 posture: we own the source).
- **D-05:** **The darwin drawterm reaches the linux-hosted guest via SSH local port-forward** — `ssh -L 17010:127.0.0.1:17010 -L 5670:127.0.0.1:5670 user@linuxhost`, then drawterm connects to darwin's own loopback. Zero new network surface: the slirp hostfwd pair stays 127.0.0.1-bound (16-D-04) and the guest stays off external networks (16-D-08); SSH carries the transport encryption; no host config changes, no doctor additions. Documented in the README's darwin section as the DESK-07 recipe. — **Reversibility:** reversible.
- **D-06:** **darwin drawterm is a documented-manual build** (the darwin documented-manual venue convention): the flake `drawterm` app is linux-only; the darwin leg builds the same vendored source (Cocoa backend, D-03) via the README recipe. Deliberate scope-keep: no aarch64-darwin flake surface this phase.
- **D-07:** **A human drawterm session runs against the interactive `nix run .#boot-r9-x64` left running in a terminal** on the linux host — zero new machinery. The README documents the session recipe (boot → connect → accept → teardown). No dedicated headless session app.
- **D-08:** **The CR-03 AcceptPool TCP fix is a planned first-class item, not a contingency.** tcp17010 is an announced connection (listen1-accept → `AcceptPool::spawn_listener` in r9 `stack.rs`), and exportfs bulk writes >2048 bytes into announced sockets park forever under stock smoltcp Nagle + 10ms delayed-ACK (the 18-07 finding). The fix is the two `set_*` calls at the pool site + the WR-04 pin legs, exactly as the 18-REVIEW-DISPOSITION record names. Sequenced as a dependency of the cpu/exportfs work — the gates cannot go green without it.
- **D-09:** **#66/CR-07 firing policy: honor the reopener only if the waiver's capture context is actually reached.** If the waived triple-fault class fires during 19's work AND the context the waiver names is captured (e.g. an exception-cascade capture on the new workload, or a TCG-leg repro), the reopener condition is met — root-cause in-phase. If it fires without context: record-enrich and continue. The phase's success criteria never hinge on killing the class (the 17-08 maintainer-ratified waiver stands).
- **D-10:** **[USER OVERRIDE of the recommended reopener-stands] tick_check is wired proactively this phase.** The x86_64 tick path gains the `tick_check` call, closing the recorded adjacent latent defect (retransmit/keepalive/connect-timeout timers never fire under park) BEFORE long-lived authenticated drawterm sessions can hang on it. Long-lived authenticated sessions are the classic timer-mediated-TCP consumer; the 18-07 record named this phase as the natural reopener and the user chose to close it now rather than wait for a stall. Pins ride the same host-test/boot-profile shapes as the rest of the kernel work.
- **D-11:** **Umbrella: fix everything surfaced** — any kernel defect 19's load surfaces is root-caused, fixed push-before-pin, and pinned in-phase (the Phase 14-17 doctrine). cpu/listen/exportfs/devssl exercise fresh kernel surface; surprises are expected and owned.
- **D-12:** **The Phase-18 domain generator additionally emits the demo credentials to a gitignored file in the bake output** (demo user + password, e.g. `out/auth-creds.txt` shape — planner's naming), and the `-G` gate reads it post-bake. 18-D-01's "real randomness" ephemeral-DEMO contract is preserved verbatim; nothing secret-shaped lands in git; hosted CI works with zero committed credentials. The human legs use the developer's local-file domain (the 18-D-01 shape).
- **D-13:** **DESK-07 evidence = one screenshot per leg + a signed human-acceptance row in `19-EVIDENCE.md`** — the rio desktop visible in the drawterm window on linux and on darwin, each row carrying date, venue, r9 SHA, and who accepted (the 17-08 maintainer-ratification record style). No screen recordings required.
- **D-14:** **[USER OVERRIDE of the recommended hosted-CI-required] The `-G` interop gate's required green witness is the local KVM run; hosted CI runs it best-effort.** INT-06's fresh hosted CI run after the pin advance still happens (the pin-advance discipline is untouched) — it simply is not the `-G` gate's required venue. Local KVM is the interactive drawterm session's real venue; green-twice on KVM is the gate's bar.
- **D-15:** **Encrypted default.** The wrapper's printed "exact connect line" and every ordinary session default to encrypted — devssl `#D` rc4_256/sha1 via pushssl (DESK-04 proven live by ordinary use, not just a dedicated probe) — and the `-G` gate rides encrypted too. `-e clear` stays the documented debugging fallback. (The vendor's existing `-e` flag surface carries this; no new flag machinery.)

### Carried forward (locked in prior phases — do not re-litigate)

- **18's auth domain**: keyfs/authsrv on tcp/567, DEMO-labeled ephemeral material when the local key file is absent (18-D-01), nvram via env var (18-D-03), three baked users glenda/bootes/third (18-D-12), the sixth mkimage contract + `nix/smoke-r9-auth-x64.sh` gate lineage, and 18-D-11's anticipation: **19 composes its own image — a seventh contract is fine**
- **16's hostfwd pair** as TOML data: host 127.0.0.1:17010 → guest 17010 (cpu), 5670 → 567 (auth) — loopback-bound, never widened
- **15's pre-staged payload**: fonts, `usr/glenda/lib/profile` cpu-aware (cpu case intact, terminal rio-free — D-17 shape), ndb template; plus the **rio/acme/sam donor syscall traces (15-06)** as the measure-first input for any kernel growth the desktop path needs
- push-before-pin; **FOD re-wire-then-pin after the kernel-byte fix pass** (19 has kernel bytes: `#!` exec, WSTAT/FWSTAT/REMOVE, devssl, AcceptPool fix, tick_check — one re-wire pass, pin advance its OWN commit + fresh hosted CI run per INT-06, never bundled)
- Gates split by artifact (14-D-06): kernel substrate evidence r9-side (host pins + boot profiles), composed gates Harvey-side
- One-image-per-gate (14-06) + app-per-gate; single-source-as-data (netaddr seam — no venue literal outside `netaddr.rc`); 18-05's loopback idiom (`-s 127.0.0.1`-style literals — `$attr` cs translation reds on r9, name it for the -G gate's dial line)
- Venue honesty: named venues, presence-based timing-tolerant assertions, green-twice, no unearned OK; sourced-not-typed in-guest scripts; rc dialect landmines (15-02: bare `=` ends words, braced if-not bodies kill sourced files, fd-dup unreliable in nested blocks; 18-05: rc output-redirect open failures are fatal to sourced sweeps)
- The 18-06 environment fact: mkimage TARG resolver breaks in the nix devshell — the flake-app PATH (plan9port LAST) is the sanctioned venue
- aarch64 frozen at the v2.0 pin — no aarch64 changes
- mkpaqfs silent-drop standing hazard — every new baked file class rides the fail-on-any-diagnostic bake assert

### Claude's Discretion (verbatim)

- The seventh mkimage contract's exact shape (composition from the chain/auth/cpu suites, section numbering — the auth bake is §14, cpu lands §15+; cpurc/termrc selection; what rides the image vs. the devroot)
- cpurc content re-keyed from `cfg/example/cpurc` (which listeners at boot, ordering, marker vocabulary; `listen1 -t` trusted-arm precedent from 18-04)
- The `service=cpu` delivery mechanism (CPU-02: plan9.ini equivalent via GRUB cmdline/`-append` vs a baked config file — and the dev UX of terminal-vs-cpu boot selection)
- `-G` implementation detail in the vendor (arg parse, window skip, stdio wiring — D-04 fixes the shape, not the code)
- The drawterm nix package recipe (gui-x11 deps on linux; exact wrapper surface for DESK-01: `-O` pinning semantics, which env vars scrubbed, the printed connect line's exact form)
- devssl module placement/shape in r9 (`port/src/devssl.rs` suggested; donor semantics from `sys/src/9k/port/devssl.c`; the rc4_256/sha1 subset pushssl actually needs; host-pin + boot-profile evidence split per 14-D-06)
- `#!` exec implementation in r9 (the exec-path change + host pins) and the WSTAT/FWSTAT/REMOVE syscall surface (what exportfs/cpu actually exercise vs the full donor set — measure-first via the 15-06 traces where applicable)
- The -G gate's flake app shape (its own app per app-per-gate; expect marker vocabulary; where the creds file is read)
- Whether/which r9 boot profiles join vs composed-gate-only evidence per substrate item
- Evidence file organization under `.planning/phases/19-*/` and the README's new sections (drawterm, darwin leg, cpu service)

### Deferred Ideas (OUT OF SCOPE — verbatim)

- **The #66/CR-07 cure** — stays waived under the 17-08 ratification; D-09 records this phase's firing policy; the reproduction instrument stays committed.
- **A headless cpu-session app** (qemu -display none + lifecycle management) — declined this phase (D-07).
- **A native aarch64-darwin drawterm flake app** — declined this phase (D-06).
- **LAN-exposed guest networking** (hostfwd rebind / tap-on-LAN) — declined (D-05).
- **rio at boot on the terminal path** — out of scope by requirement (in-session only).
- **Stronger drawterm crypto (TLS-class)** — PROTOCOL-blocked per DESK-04; the hardening milestone's item.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CPU-01 | r9 `#!` exec (tcp17010 is literally `#!/bin/cpu -R`, execl'd by listen) | Donor semantics verified verbatim at `sys/src/9k/port/sysproc.c:282-324` (read order, arg push); listener `execl(s->prog, s->prog, s->serv, proto, dir, nil)` at `sys/src/cmd/aux/listen.c:541`; r9 integration site `x86_64/src/exec.rs` (`parse_aout` + argv stack builder) |
| CPU-02 | `service=cpu` bootconf path: plan9.ini equivalent → cpurc runs instead of termrc | Full chain decoded: init flags `-c`/`-t` (`sys/src/cmd/init.c`), boot's `execinit()` honors `getenv("init")` FIRST (`sys/src/9k/boot/boot.c:208`), r9's plan9.ini-equivalent today is `seed_root_group` in `x86_64/src/main.rs` (`cputype=amd64 nobootprompt=embed readparts=0`), `bootr9.c` `cpuflag=0` hardcodes terminal |
| CPU-03 | aux/listen + tcp17010 live at boot; exportfs served; r9 implements WSTAT/FWSTAT/REMOVE | listen mechanics verified (scandir→`tcp!*!<port>` announce, trust arms); donor service file `sys/src/9k/k10/root/tcp17010` = `#!/bin/cpu -N -R`; sys.h numbers REMOVE=25/_WSTAT=26/_FWSTAT=27/WSTAT=44/FWSTAT=45; r9 DISPATCH loud-None for exactly these; exportfs consumers `remove/dirfwstat/dirwstat` at `sys/src/cmd/exportfs/exportsrv.c:343-423`; sam trace shows live `Remove` use |
| CPU-04 | Full `cpu -R` remoteside flow verified end-to-end | `sys/src/cmd/cpu.c:285-372 remoteside()` read in full: setamalg → srvp9auth (`auth_proxy` role=server + `auth_chuid` + pushssl server-side) → xdir/cmd → "FS"/"/" → `fversion(fd, 64*1024,...)` → `mount(...MCREATE|MREPL...)` → rmtnoteproc rforks → `/mnt/term/dev/cons` as 0/1/2 → `execl("/bin/rc","rc","-li",nil)` |
| DESK-01 | Flake drawterm app: provenance recorded; wrapper pins `-O`, scrubs `$cpu`/`$auth`/`$PASS`, prints exact connect line | Vendor flag surface verified (`-a -c -d -e -C -k -r -s -u`; NO `-O`, NO `-G` — both are local additions); env reads `$USER`/`$auth`/`$cpu` with defaults "auth"/"cpu"; netmkaddr 3-part passthrough solves the 5670 auth remap |
| DESK-02 | profile `case cpu` binds /mnt/term/dev over /dev and execs rio; serial marker proves rio did NOT start at boot | The staged profile's `case cpu` block read verbatim at `build/x86_64/root-tree/usr/glenda/lib/profile` (binds + `exec rio` guarded by `! test -e /mnt/term/mnt/wsys`); `$service` set by remoteside's `putenv("service","cpu")` |
| DESK-04 | r9 devssl `#D` (rc4_256/sha1) via pushssl; `-e clear` documented fallback | Donor `sys/src/9k/port/devssl.c` mapped: QID(c,y)=(c<<5)\|y, clone/ctl/data/secretin/secretout, ctl verbs `fd`/`alg`/`secretin`/`secretout`, encalgs incl. `"rc4_256", 1, RC4, initRC4key`, hashtab `sha1` (SHA1dlen), secrets via `dec64`; `pushssl.c` protocol read verbatim; r9 device-registry precedent `devcap.rs` |
| DESK-05 | plumb/plumber runs in the drawterm session | `cmd/plumb` is tier-3-built (`tiers/tier3-gui.txt`: TARG=plumber plumb); in-session plumbing rides /mnt/term-mounted namespaces; plumber is a plain user-space 9P server (lib9p/libplumb — no new kernel surface measured) |
| DESK-06 | `-G` text-mode drawterm interop gate in CI via hostfwd | D-04 shape; expect-on-pty discipline from 16-05 findings (STATE); creds lineage verified: mkimage 14f2b stages `$OUT/authfeed.txt`, driver exports `R9_DEMO_PW` (nix/smoke-r9-auth-x64.sh:204-208); D-12 extends the generator |
| DESK-07 | Human-accepted graphical rio leg on linux AND darwin | D-05 SSH-tunnel recipe; D-06 documented-manual darwin; **research found the vendored `gui-osx` is 32-bit Carbon/QuickDraw and cannot build on aarch64-darwin** — OQ-1 must be resolved at plan time |
| INT-05 | ONE command brings up full system; boot-r9-smoke extended | `nix/boot-r9-x64.sh` read in full — the extension point is the image-selection seam (`CHAIN_PAQ`, step 2/3) plus a mode/flag for the cpu image; smoke extension rides `build/x86_64/smoke-r9.expect` lineage |
| INT-06 | r9 pin advances via push-before-pin as its OWN commit + fresh hosted CI | r9 at 9103522 == origin/main (verified live); FOD re-wire discipline documented in STATE (13-06/14-07/18-08 lineage); r9 CI = `cargo xtask check/clippy/test` + boot-test-x86-64 (explicit TCG on hosted runners) |
| INT-07 | Clean-checkout proof of full v2.1 chain on linux-x86_64 | Standard close-out; DEMO domain bakes green on clean checkouts by design (18-D-01, mkimage 14e-14f) |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `#!` exec | r9 kernel (exec path, `x86_64/src/exec.rs`) | — | Donor does it in sysexec (`sysproc.c`); listeners exec scripts blind |
| `service=cpu` delivery | r9 kernel boot env seed (`seed_root_group`) | boot(8) `execinit` / init argv | The kernel-seeded root env group IS r9's plan9.ini-equivalent mechanism (replaces 9k's #ec); init's `-c` flag is the final consumer |
| cpurc boot script | Guest userland (baked root-tree) | mkimage §15 contract | Runs as pid-1's rc child; composes 18's proven daemon sequence |
| Listener topology | Guest userland (`aux/listen`/`listen1` + `/bin/service/tcp17010`) | r9 devip (announce/listen/accept — exists) | Pure exec + announce orchestration; trust arm chosen per namespace needs |
| WSTAT/FWSTAT/REMOVE | r9 kernel (DISPATCH + per-device wstat/remove arms) | — | Syscalls 25/26/27/44/45 are loud-None today |
| devssl `#D` | r9 kernel (`port/src/devssl.rs` new; `devcap.rs` precedent) | libsec donors (rc4/sha1) for algorithm bytes | Kernel device; pushssl hard-codes `#D` |
| AcceptPool TCP fix (CR-03) | r9 kernel (`port/src/devip/stack.rs` `spawn_listener`) | — | Announced connections ARE pool members for life |
| tick_check wiring (D-10) | r9 kernel (x86_64 timer ISR path, `platform/q35/timer.rs` + caller) | — | Mirrors `aarch64/src/platform/virt/timer.rs:130` |
| drawterm build + wrapper + `-G`/`-O` | Host (nix package + flake app; vendor edits as Harvey commits) | — | D-01/D-02: we own the vendored source |
| `-G` interop gate | Host (expect driver over pty, flake app) | qemu hostfwd pair (committed TOML) | App-per-gate; D-14 local-KVM required witness |
| rio + plumb in-session | Guest userland (drawterm's draw/mouse served over 9P as /mnt/term/dev) | drawterm host-side serves `#i`/`#m`/cons | "The draw device arrives over 9P from drawterm — the entire point" (REQUIREMENTS Out of Scope) — r9 needs NO devdraw |
| Graphical acceptance | Human (linux + darwin legs) | README recipes + 19-EVIDENCE.md | D-05/D-06/D-07/D-13 |
| Pin advance | Both repos (r9 push → FOD re-wire → flake.lock lock-only → hosted CI) | — | INT-06; never bundled |

## Standard Stack

### Core (all in-tree — no external packages)

| Component | Location | Purpose | Why Standard |
|-----------|----------|---------|--------------|
| drawterm client | `sys/src/cmd/unix/drawterm/` (vendored) | The host-side terminal: p9any→p9sk1 in `cpu.c`, own libauthsrv/libsec/libmp/libdraw/libip, own `exportfs/`, `kern/devssl.c` (client pushssl), `gui-x11`/`gui-osx` | D-01 locked: source of record, never re-vendor |
| guest cpu(1) | `sys/src/cmd/cpu.c` (mkone) | The `cpu -R` remoteside + notefs/cpunote machinery | Full CPU-04 flow verified in source |
| exportfs | `sys/src/cmd/exportfs/` (mkone) | 9P file export; Xcreate/Xremove/Xwstat consumers of the new syscalls | Tier-2 entry exists |
| aux/listen + listen1 | `sys/src/cmd/aux/listen.c`, `listen1.c` | Service listeners; `listen1 [-t] addr cmd` carries flags; aux/listen scans service dirs by file name | Tier-2 entries exist (`cmd/aux:listen`, `cmd/aux:listen1`) |
| devssl donor | `sys/src/9k/port/devssl.c` (1520 lines) + `sys/src/libc/9sys/pushssl.c` | The `#D` semantics of record for the r9 port | Donor-verbatim porting posture |
| libsec algorithm donors | `sys/src/libsec/port/rc4.c`, `sha1.c` (class) | rc4_256/sha1 bytes behind devssl | r9 already carries an SHA1 block in `port/src/hmacsha1.rs` (devcap, 18-02) |
| auth domain (Phase 18) | `build/x86_64/authdomain-gen.c` + bake §14 | keyfs/authsrv/nvram/ndb as ONE paired unit; D-12 extends with creds emission | Proven green-twice lineage |
| expect gate chassis | `build/x86_64/smoke-r9-auth.expect` + `nix/smoke-r9-auth-x64.sh` | The `-G` gate's direct ancestor (creds read, hostfwd, presence asserts, redaction) | 18-08 flipped to presence demands |
| r9 kernel work | `../r9` (`stack.rs`, `exec.rs`, `syscall.rs`, new `devssl.rs`, `platform/q35/timer.rs` caller, xtask pins) | All kernel bytes this phase adds | Push-before-pin on `main` |

### Supporting (host-side, declarative)

| Item | Purpose | When to Use |
|------|---------|-------------|
| nixpkgs `xorg.libX11` (+ likely `libXt` headers) | gui-x11 build deps for the linux drawterm package | `x11.c` includes `<X11/IntrinsicP.h>` [VERIFIED: sys/src/cmd/unix/drawterm/gui-x11/x11.c:26] though `Make.unix` links only `-lX11` [VERIFIED: Make.unix LDADD line] |
| `expect` (flake runtimeInputs) | `-G` gate driver + pty discipline | NOT on this host's interactive PATH — the flake app provides it (the auth-gate lineage) |
| `stdenv.cc` | bake-time authdomain-gen host build | mkimage §14d precedent; the flake app carries it |
| plan9port (flake runtimeInputs, LAST on PATH) | mk/TARG venue | 18-06: the devshell breaks the TARG resolver — flake-app venue only |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Vendored drawterm (D-01) | 0intro upstream re-vendor / 9front drawterm | LOCKED — do not re-litigate; upstreams differ in flag surface and 9P/auth corners |
| `aux/listen -t /bin/service tcp` | `listen1 -t 'tcp!*!17010' /bin/service/tcp17010` | listen1 is the 18-04 proven shape and carries no service-dir scan; aux/listen satisfies CPU-03's literal naming and auto-discovers future services — both need kernel `#!`; planner picks (see Patterns) |
| `seed_root_group` `init=` env | bootr9.c `cpuflag=1` (second boot build) / mb2 cmdline parse | env var keeps ONE boot binary and is the documented "ONE mechanism"; cpuflag is a compile-time fork per image; cmdline adds r9 parser surface for runtime selection |
| 9front gui-cocoa port for darwin | Make.osx-x11 + XQuartz | research finding: gui-osx is Carbon/32-bit (OQ-1); XQuartz contradicts D-03's no-XQuartz; gui-cocoa port keeps D-03's spirit (native, no X11) at the cost of a port |

**Installation:** none — zero external packages. The nix recipe compiles the in-tree vendor (`make CONF=unix` shape, `gui-x11` on linux); r9's Cargo deps are unchanged this phase.

**Version verification:** not applicable (no ecosystem packages; all sources are in-tree at pinned commits; r9 verified at `9103522` == `origin/main` this session).

## Package Legitimacy Audit

**No external packages are installed by this phase.** Every binary is built from in-tree vendored source (drawterm, cpu, exportfs, listen, auth suite) or the sibling r9 repo at its pin. The nix recipe is a builder over committed source with no `fetchurl`/`fetchgit` of third-party code. Registry checks: not applicable.

**Packages removed due to [SLOP]:** none.
**Packages flagged [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```text
 HOST (linux-x86_64, primary; darwin via SSH -L for DESK-07)
 ┌────────────────────────────────────────────────────────────────┐
 │ nix run .#drawterm  (flake app → wrapper → vendored drawterm)  │
 │  wrapper: pins -O, scrubs $cpu/$auth/$PASS, prints line        │
 │  drawterm -G (gate) or gui-x11 window (human)                  │
 │    │ rexcall: dial tcp!127.0.0.1!17010   ───────────────┐      │
 │    │ authdial: dial tcp!127.0.0.1!5670 (3-part -a)  ──┐│      │
 └───────────────────────────────────────────────────────││──────┘
        qemu hostfwd (committed TOML, 127.0.0.1-bound)   ││
   host 17010 ═══════════════════► guest 17010 (cpu)  ◄──┘│
   host 5670  ═══════════════════► guest 567   (auth) ◄───┘
 ┌──────────────────────────── GUEST (qemu q35, ONE command) ────┐
 │ r9 kernel (flush-out-hardened pin)                             │
 │  ├ #! exec ── WSTAT/FWSTAT/REMOVE ── devssl #D ── AcceptPool   │
 │  │  fix ── tick_check                    (kernel work, r9)     │
 │  └ seed_root_group: cputype/nobootprompt/... [+ init/service]  │
 │ boot(8) → execinit (getenv("init") > cpuflag c/t) → init [-c]  │
 │  init: setenv #e/service → iscpu → rc -c /rc/bin/cpurc         │
 │ cpurc (§15 image, re-keyed from k10root.rc + 18's sweep):      │
 │  pairing-check → keyfs (nvram= env) → authsrv (listen 567)     │
 │  → secstored → cs → factotum(-S, hostowner key) → listener     │
 │  listener(trusted): announce tcp!*!17010                       │
 │   └ per-conn: exec /bin/service/tcp17010 ── kernel #! ──►      │
 │      /bin/cpu -R  (remoteside)                                 │
 │       ├ p9any server (factotum) → auth_chuid (#¤/capuse)       │
 │       ├ pushssl #D rc4_256 sha1 (server arm) [encrypted]       │
 │       ├ fversion + mount(client exportfs) at /mnt/term         │
 │       └ stdio ← /mnt/term/dev/cons → rc -li                    │
 │            └ . lib/profile: case cpu →                          │
 │               bind /mnt/term/dev over /dev → exec rio          │
 │               (draw/mouse = drawterm's, over 9P; plumb runs    │
 │                in-session; rio provably NOT at boot)           │
 └────────────────────────────────────────────────────────────────┘
 Gate evidence: expect markers on serial + the -G shell marker;
 kernel substrate evidence: r9 host pins + boot profiles (14-D-06).
```

### Recommended Project Structure (the surfaces this phase touches)

```text
harvey/
├── sys/src/cmd/unix/drawterm/     # D-01 vendor: -G/-O additions, cpu.c arg parse
├── build/x86_64/
│   ├── mkimage.sh                 # §15: the seventh contract (cpu image)
│   ├── root-tree/rc/bin/cpurc     # NEW: the re-keyed cpurc (or cpurc override file)
│   ├── root-tree/bin/service/tcp17010  # the #! script (bake-staged)
│   ├── tiers/tier2-services.txt   # cmd/cpu joins (sweep + witness re-pin)
│   └── drawterm-gate.expect (naming: planner)  # the -G driver
├── nix/
│   ├── drawterm.nix (or flake inline)  # package: in-tree vendor build (gui-x11)
│   ├── drawterm-x64.sh            # wrapper app (scrub/pin/print)
│   └── boot-r9-x64.sh             # full-system mode (image-selection seam)
├── flake.nix                      # apps: drawterm, -G gate, smoke extension
└── build/x86_64/README.md         # drawterm + darwin SSH recipe + cpu sections
../r9/
├── x86_64/src/exec.rs             # #! pre-pass
├── x86_64/src/syscall.rs          # DISPATCH: REMOVE/_WSTAT/_FWSTAT/WSTAT/FWSTAT
├── x86_64/src/{main,timer caller} # seed_root_group service= / init= ; tick_check
├── port/src/devssl.rs             # NEW: #D (devcap.rs precedent)
└── port/src/devip/stack.rs        # spawn_listener: the two set_* calls
```

### Pattern 1: The trusted listener arm (namespace preservation)

**What:** `cpu -R` needs `/mnt/factotum` (auth_proxy opens `/mnt/factotum/rpc` [VERIFIED: sys/src/libauth/auth_proxy.c:201]) and devcap caphash written by hostowner-owned factotum. An untrusted listener calls `becomenone()` (`#c/user` write + `newns("none")`) which strips both.
**When to use:** every cpu/auth service whose child needs the boot namespace.
**Evidence (verbatim, k10root.rc:98-110):**
```rc
# Listen is run trusted if there is no factotum running,
# as 'cpu -R' with no authentication needs to be able to
# open '#¤/caphash' in order to change the owner and
# that can only be done if running as '#c/hostowner'.
sysname=cpu-$ip(1)
bind /boot/rc/bin/service /bin/service
if(test -d /mnt/factotum)
	aux/listen -q tcp
if not
	aux/listen -t /bin/service tcp
```
[VERIFIED: sys/src/9k/k10/root/k10root.rc:98-110 — note: with `-t <dir>` and no `-d`, only the trustdir is scanned, because `if(!servdir && !trustdir) servdir = "/bin/service"` fires only when BOTH are unset (sys/src/cmd/aux/listen.c:119-120)]

### Pattern 2: `#!` exec is kernel work (the donor's exact algorithm)

**What:** listen `execl`s the service file blind; the kernel re-execs it with the interpreter.
**Evidence (verbatim, sys/src/9k/port/sysproc.c:282-305):**
```c
	 * If it's a #!, fill in progarg[] with info then read a new header
	 * from the file indicated by the #!.
	 * The #! line must be less than sizeof(Exec) in size,
	 * including the terminating \n.
	 */
	hdrsz = chan->dev->read(chan, &hdr, sizeof(Hdr), 0);
	if(hdrsz < 2)
		error(Ebadexec);
	p = (char*)&hdr;
	argc = 0;
	if(p[0] == '#' && p[1] == '!'){
		p = memccpy(line, (char*)&hdr, '\n', MIN(sizeof(Exec), hdrsz));
		if(p == nil)
			error(Ebadexec);
		*(p-1) = '\0';
		argc = tokenize(line+2, progarg, nelem(progarg));
		if(argc == 0)
			error(Ebadexec);

		/* The original file becomes an extra arg after #! line */
		progarg[argc++] = file;
```
and at sysproc.c:414: "Next push any arguments found from a #! header" — progarg lands BEFORE the caller's argv. So `listen → execl(tcp17010, "17010", "tcp", dir)` becomes `/bin/cpu -R <scriptpath> 17010 tcp <dir>`; `-R` hits `remoteside()` via ARGBEGIN and the trailing args are ignored. [VERIFIED: sys/src/9k/port/sysproc.c:282-324, 414]

### Pattern 3: service=cpu delivery — three viable seams, one zero-fork favorite

1. **r9 `seed_root_group` gains `("init", "/amd64/init -c")`** — boot's `execinit()` checks `cmd = getenv("init")` FIRST and execs it verbatim [VERIFIED: sys/src/9k/boot/boot.c:207-210: `cmd = getenv("init"); if(cmd == nil){ sprint(cmdbuf, "/%s/init -%s%s", cputype, cpuflag ? "c" : "t", mflag ? "m" : ""); cmd = cmdbuf; }`]. One boot binary serves both images; the "kernel-seeded ROOT env group — the ONE mechanism replacing both 9k's #ec config env and an argv hint" [VERIFIED: r9 x86_64/src/main.rs:520-528 comment + `seed_root_group(&[(b"cputype", b"amd64"), (b"nobootprompt", b"embed"), (b"readparts", b"0")])`]. CAUTION: with the `init` override, `cpuflag` stays 0 so `doauth(0)`/`glenda()` still runs — hostowner becomes glenda (the 18-venue continuity; keep it).
2. **bootr9.c `cpuflag = 1`** for the cpu image's boot build [VERIFIED: sys/src/9k/boot/bootr9.c:28 `int cpuflag = 0;		/* terminal → init -t */`] — a second boot binary per image (one-image-per-gate tolerates it), but ALSO flips `doauth(cpuflag)` and `outin`'s alarm(15s) prompts.
3. **mb2 cmdline (GRUB `-append`)** parsed by r9 into the env seed — runtime terminal-vs-cpu selection on one image, at the cost of new kernel cmdline-parsing surface.
Init's own logic either way: `service = "cpu"` default; `-c` sets cpu; `-t` sets terminal; `setenv("#e/service", service)`; `iscpu → fexec(cpustart) → execl("/bin/rc", "rc", "-c", "/rc/bin/cpurc", nil)`; terminal → `. /rc/bin/termrc; home=/usr/$user; cd; . lib/profile` [VERIFIED: sys/src/cmd/init.c:30-64, 174-186].

### Pattern 4: pushssl — the exact #D protocol both sides must speak

**Evidence (verbatim, sys/src/libc/9sys/pushssl.c):**
```c
	ctl = open("#D/ssl/clone", ORDWR);
	...
	n = read(ctl, buf, sizeof(buf)-1);          /* decimal conv number */
	snprint(dname, sizeof dname, "#D/ssl/%s/data", buf);
	data = open(dname, ORDWR);
	if(fprint(ctl, "fd %d", fd) < 0 ||
	   fprint(ctl, "secretin %s", secin) < 0 ||
	   fprint(ctl, "secretout %s", secout) < 0 ||
	   fprint(ctl, "alg %s", alg) < 0){ ...
```
Client passes `(fromclientsecret, fromserversecret)`; the server's `srvp9auth` passes them SWAPPED [VERIFIED: sys/src/cmd/cpu.c:399 vs 647]. Secrets are **dec64**, not hex: `t = dec64(x, l, p, strlen(p))` for both secretin/secretout [VERIFIED: sys/src/9k/port/devssl.c:1196-1210]. Algorithms (NOSPOOKS table active — `#define NOSPOOKS 1` at devssl.c:13): `{ "rc4_256", 1, RC4, initRC4key, }`, `{ "rc4_40", 1, RC4, initRC4key_40, }`, hashtab `{ "sha1", SHA1dlen, sha1, }` [VERIFIED: sys/src/9k/port/devssl.c:970-1027]. QID layout `QID(c,y)=((c)<<5)|y`, `CONV(x)=(((x).path>>5)&(Maxdstate-1))`, `Maxdstate=512` [VERIFIED: devssl.c:87-100]. drawterm's own `kern/devssl.c` matches the layout/verbs (Maxdstate=128 — capacity only) [VERIFIED: sys/src/cmd/unix/drawterm/kern/devssl.c:75,92-99,123-124].

### Pattern 5: the creds/feed lineage D-12 extends

mkimage §14f2 already stages the generator's `feed` (single newline-terminated password line) at `/boot/authfeed.txt` AND copies it host-side to `$OUT/authfeed.txt`; the gate driver reads it post-bake: `R9_DEMO_PW_FILE="$HARVEY/build/x86_64/out/authfeed.txt" ... R9_DEMO_PW=$(tr -d '\n\r' < ...)` exported for expect [VERIFIED: build/x86_64/mkimage.sh:1455-1482, nix/smoke-r9-auth-x64.sh:204-208]. D-12's `out/auth-creds.txt` (user + password) is a sibling artifact from the same generator run; the feed discipline (rm-before-cp, read-only, redaction in retained captures) carries over verbatim.

### Pattern 6: CR-03 fix = mirror tcp_socket's two calls into spawn_listener

**Evidence (verbatim, r9):** the fixed connect-path shape in `port/src/devip/proto.rs`:
```rust
    s.set_nagle_enabled(false);
    s.set_ack_delay(None);
```
[VERIFIED: ../r9/port/src/devip/proto.rs:2132-2133 — and the doc comment names this phase: "a >2048-byte bulk write would block in the send arm on the same never-driven timer — Phase 19's cpu path rides this seam"]. The pool site `fn spawn_listener(core: &mut StackCore, port: u16)` sets only hop-limit today [VERIFIED: ../r9/port/src/devip/stack.rs:814-823: `s.set_hop_limit(Some(255)); s.listen(port)...`]. Pool members keep 2048/2048 buffers (`POOL_SOCK_BUF: usize = 2048` [VERIFIED: stack.rs:703]) and are the accepted connection's socket for life — the fix covers announce-path sockets end-to-end.

### Anti-Patterns to Avoid

- **Untrusted listener for cpu/auth services** — `becomenone()`'s `newns("none")` strips `/mnt/factotum` and bootes' namespace; the service dies mid-auth. Use the trusted arm (Pattern 1).
- **Patching authdial for the 5670 remap** — the 3-part `netmkaddr` form already passes through verbatim ("if there is already a service, use it" — `netmkaddr.c:33-36`); zero vendor code needed. [VERIFIED: sys/src/cmd/unix/drawterm/libc/netmkaddr.c]
- **`-e clear` as default** — D-15 locks encrypted default; clear is the documented debug fallback only.
- **Feeding drawterm's password via piped stdin** — `askuser`/`getkey` ride `readcons` on `/dev/cons` (raw interactive reads); the gate must drive a pty (16-05 findings, carried in the CONTEXT watch-outs).
- **New rc dialect experiments in cpurc** — bare `=` ends words, braced if-not bodies kill sourced files, fd-dup unreliable in nested blocks, output-redirect open failures fatal (15-02/18-05, carried forward).
- **Baking via the devshell** — the 18-06 TARG resolver fact; flake-app PATH only.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Terminal client | a new cpu client | the vendored drawterm (D-01) | 731-line cpu.c carries p9any, secstore, exportfs launch, ssl; decades of corners |
| rc4/sha1 | new crypto | port `libsec` donors (rc4.c/sha1.c) donor-verbatim; r9 may reuse the SHA1 block already in `port/src/hmacsha1.rs` | interop is byte-exact; both ends must agree |
| #! parsing semantics | an argv convention of your own | donor sysproc.c algorithm (Pattern 2) | the arg order (interp args, then script path, then caller args) is what every existing script assumes |
| Listener | a bespoke accept loop | aux/listen/listen1 | announce semantics, re-scan, trust arms, remoteaddr logging all handled |
| Auth plumbing | new ticket code | factotum/authsrv/keyfs (Phase 18, green-twice) + drawterm's own client stack | p9sk1 is protocol-frozen |
| pty expect harness | a new driver framework | the smoke-r9-auth expect chassis + 16-05 pty findings | green-twice lineage, redaction, process-group kill on every exit path |

**Key insight:** every novel line of code this phase writes is either (a) a donor-verbatim port into Rust, (b) a vendored-source flag addition, or (c) composition glue in mkimage/nix/expect. The risk lives in fidelity and ordering, not design.

## Runtime State Inventory

Not a rename/refactor/migration phase — feature composition; no runtime state keyed to renamed strings. (Checklist item: N/A by phase shape.)

## Common Pitfalls

### Pitfall 1: The darwin backend is Carbon, not Cocoa (D-03 conflict)
**What goes wrong:** `Make.osx` carries `-m32 ... -framework Carbon -framework QuickTime` and `gui-osx/screen.c` includes `<Carbon/Carbon.h>` + `<QuickTime/QuickTime.h>` and calls `QDBeginCGContext/GetWindowPort` [VERIFIED: sys/src/cmd/unix/drawterm/Make.osx, gui-osx/screen.c:8-9,729-739]. Carbon was never 64-bit and 32-bit support ended at macOS 10.15; Apple Silicon cannot run or build it [CITED: eclecticlight.co/2023/06/30/deprecation; Apple Developer docs]. The DESK-07 darwin graphical leg cannot ship as written.
**How to avoid:** resolve OQ-1 at plan time — port a 64-bit Cocoa (AppKit) frontend (9front's `gui-cocoa` class, `CONF=osx-cocoa make`, builds natively arm64 [CITED: github.com/Plan9-Archive/drawterm-metal-cocoa, github.com/rcarmo/drawterm]) into the Harvey vendor as a Harvey commit (D-01/D-02 posture allows it; 9front imports are MIT per PROJECT constraints), or re-decide with the user. A darwin `-G`-only build (no GUI) is a lesser fallback that cannot satisfy DESK-07's screenshot row. **(RESOLVED 2026-11-14, amended D-03: the owner re-decided — darwin is `-G` text-only this milestone and the darwin `-G` leg IS the DESK-07 darwin row (19-10, screenshot of the Terminal session); the gui-cocoa-class port is the named reopener, not a this-phase deliverable. The "cannot satisfy DESK-07" clause above is superseded by the owner's amended scope; 19-06 delivers the gui-none build.)**
**Warning signs:** any darwin build log mentioning Carbon/QuickTime/-m32.

### Pitfall 2: `-O` does not exist in the vendor
**What goes wrong:** DESK-01 says "wrapper pins `-O`" but the vendored drawterm has no `-O` (ARGBEGIN cases: `a c d e C k r s u` — verified in both `cpu.c:123-161` and `cpu-bl.c`); passing it today hits `default: usage()`.
**How to avoid:** treat `-O` exactly like `-G` — a local vendor addition whose semantics the plan defines (recommend: the pinning flag through which the wrapper supplies the connection parameters, making argv — not env — the single source; see OQ-2).
**Warning signs:** any plan step that assumes a pre-existing `-O`.

### Pitfall 3: `$cpu`/`$auth` env defaults silently hijack the connection
**What goes wrong:** drawterm reads `authserver = getenv("auth")` defaulting to `"auth"` and `system = getenv("cpu")` defaulting to `"cpu"` [VERIFIED: sys/src/cmd/unix/drawterm/cpu.c:115-122] — hostname lookups that hang or fail obscurely on a loopback venue.
**How to avoid:** the wrapper scrubs `$cpu`/`$auth`/`$PASS` (DESK-01) and passes `-a 'tcp!127.0.0.1!5670' -c 127.0.0.1 -u <demo-user>` explicitly. The 3-part `-a` form is honored verbatim by netmkaddr (an address that already has a service is used as-is).

### Pitfall 4: CR-03 fix sequenced late
**What goes wrong:** every end-to-end leg (`-G`, CPU-04 serial evidence, drawterm sessions) writes >2048-byte bursts into the announced 17010 connection (devmnt bulk traffic, big reads of /mnt/term files); under stock pool defaults these park forever (the 18-07 class).
**How to avoid:** D-08 — land the two `set_*` calls + WR-04 pin legs BEFORE any end-to-end gate work; the CONTEXT watch-out repeats it.
**Warning signs:** gates that hang mid-session after auth succeeds.

### Pitfall 5: init defaults to cpu — protect the terminal chain
**What goes wrong:** `init`'s default `service` is `"cpu"` [VERIFIED: sys/src/cmd/init.c:31 `service = "cpu";`]; the chain image reaches termrc only because bootr9 passes `-t`. A careless seed/env change on the shared boot path flips the chain image to a missing `/rc/bin/cpurc` (init loops printing "init: starting /bin/rc").
**How to avoid:** one-image-per-gate — the cpu delivery mechanism must be image-scoped (bake-time env seed or a cpu-image boot build), never a global r9 change that rekeys the chain/services/net images. Re-green the existing gates after the kernel change.

### Pitfall 6: the mkimage witness + tier machinery reacts to new /amd64/bin members
**What goes wrong:** `cmd/cpu` is in NO tier file today [VERIFIED: grep over build/x86_64/tiers/*.txt — zero hits]; adding it (and any staged service scripts) changes the tier sweep set and trips the k10t1 witness-manifest digest (14-08 discipline; 17-06 precedent).
**How to avoid:** add `cmd/cpu` to tier2-services.txt (it is a mkone service binary), re-run `build/mm/build-9k10t1.sh` to re-pin the manifest (gitignored, no commit), and keep the per-entry fresh-build rules (15-06 tier hygiene). tcp17010 is a tiny rc-ish text file — it stages via the contract, not a tier.

### Pitfall 7: mkpaqfs caps and the font payload
**What goes wrong:** mkpaqfs silently drops files >4 MiB (15-07); the cpu image composes the DESK-03 font tree (~26.5 MB services paq measured at 15-08; single files are small but the class rides the fail-on-any-diagnostic assert) and the r9 boot-module stash is 64 MiB (15-08 fix) — capacity fits, but any NEW large file class needs the assert.
**How to avoid:** reuse `stage_payload` + the committed `bootfs-payload.txt` manifest (the 15-08 DECLARE-by-invocation shape); keep the per-file cap assert (§14n precedent) on the new contract.

### Pitfall 8: expect/pty discipline on the -G gate
**What goes wrong:** drawterm's password prompt does raw single-char reads with echo-off on the terminal; a piped-stdin feed never satisfies it; spawned ptys stay canonical for children that don't set raw mode; `head` under timeout emits nothing (read exact byte counts) — all measured 16-05 facts.
**How to avoid:** drive drawterm under expect on a pty; type the creds from `R9_DEMO_PW`/the D-12 file; assert the shell marker presence (never timings); kill qemu's + drawterm's process groups on every exit path (the auth-gate chassis); redact the password from retained captures (T-18-13 lineage).

### Pitfall 9: rc-dialect landmines inside cpurc
**What goes wrong:** the k10root donor uses `flag x`, `. -i '#d/0'`, and backquote-heavy idioms; the in-guest rc (15-02/18-05 measured) kills sourced files on braced if-not bodies, ends words at bare `=`, mis-dup fds in nested blocks, and dies on output-redirect open failures.
**How to avoid:** re-key with flat status ladders (the auth-sweep style); `nvram=/adm/nvram` assignment is safe exactly as written (bare `=` ends words — but `nvram=...` as a whole word assignment is the proven form, auth-sweep leg 2).

### Pitfall 10: seeded-env vs baked-file divergence for `service=cpu`
**What goes wrong:** delivering `service=cpu` via a kernel-seeded env var only works if boot actually consults it (the `init=` path); delivering via init flags requires rebuilding boot; mixing both yields images whose behavior depends on which boot binary is staged.
**How to avoid:** pick ONE seam per image (Pattern 3), assert it at bake (grep the staged boot/env composition), and name the mechanism in the image manifest so the gate can pin it.

### Pitfall 11: devssl port fidelity corners
**What goes wrong:** the secrets are dec64 of 20-char ASCII strings (not hex-decoded keys) and the wire framing prepends the digest and pads ("SSL format", `max`/`maxpad` arithmetic at devssl.c:1160-1190) — a "cleaner" port breaks interop with drawterm's vendored kern/devssl.c silently (auth succeeds, then every read garbles).
**How to avoid:** port donor-verbatim including dec64 and the framing; pin with a host test that round-trips the exact `mksecret`-shaped inputs through rc4_256+sha1; prove live via an encrypted `-G` run (D-15: the gate rides encrypted, so the gate IS the interop probe).

## Code Examples

### CPU-04: the remoteside core (guest, verbatim donor)
```c
/* Source: sys/src/cmd/cpu.c:285-372 (read this session) */
void
remoteside(void)
{
	...
	rfork(RFENVG);
	putenv("service", "cpu");
	fd = 0;
	n = readstr(fd, cmd, sizeof(cmd));            /* "p9 rc4_256 sha1" */
	if(setamalg(cmd) < 0){ ... }
	fd = (*am->sf)(fd, user);                     /* srvp9auth: auth_proxy(0,nil,"proto=%q role=server %s") + auth_chuid */
	putenv("user", user); sprint(home, "/usr/%s", user); putenv("home", home);
	... xdir/cmd protocol ...
	writestr(fd, "FS", "FS", 0);
	writestr(fd, "/", "exportfs dir", 0);
	n = read(fd, buf, sizeof(buf));               /* "OK" */
	strcpy(buf, VERSION9P);
	if(fversion(fd, 64*1024, buf, sizeof buf) < 0)
		exits("fversion failed");
	if(mount(fd, -1, "/mnt/term", MCREATE|MREPL, "") < 0)
		exits("mount failed");
	close(fd);
	rmtnoteproc();                                /* rfork RFPROC|RFFDG|RFNOTEG|RFNAMEG|RFMEM ... */
	for(i = 0; i < 3; i++)
		close(i);
	if(open("/mnt/term/dev/cons", OREAD) != 0)
		exits("open stdin");
	if(open("/mnt/term/dev/cons", OWRITE) != 1)
		exits("open stdout");
	dup(1, 2);
	...
	execl("/bin/rc", "rc", "-li", nil);
}
```

### The encrypted handshake both sides run (verbatim, cpu.c p9auth/srvp9auth)
```c
/* client: sys/src/cmd/unix/drawterm/cpu.c:369-403 (srvp9auth twin at sys/src/cmd/cpu.c:608-651) */
	ai = p9any(fd);
	...
	memmove(key+4, ai->secret, ai->nsecret);
	if(ealgs == nil)
		return fd;                            /* -e clear */
	for(i = 0; i < 4; i++) key[i] = fastrand();
	write(fd, key, 4);
	readn(fd, key+12, 4);
	sha1(key, sizeof(key), digest, nil);
	mksecret(fromclientsecret, digest);
	mksecret(fromserversecret, digest+10);
	i = pushssl(fd, ealgs, fromclientsecret, fromserversecret, nil);
```

### DESK-02: the staged cpu-aware profile (verbatim, root-tree)
```rc
# Source: build/x86_64/root-tree/usr/glenda/lib/profile (15-08 staged, "Phase 19 consumes it unchanged")
switch($service){
case terminal
	prompt=('term% ' '	')
	fn term%{ $* }
case cpu
	if (test -e /mnt/term/mnt/wsys) { # rio already running
		bind -a /mnt/term/mnt/wsys /dev
		if(test -w /dev/label)
			echo -n $sysname > /dev/label
	}
	bind /mnt/term/dev/cons /dev/cons
	bind /mnt/term/dev/consctl /dev/consctl
	bind -a /mnt/term/dev /dev
	prompt=('cpu% ' '	')
	fn cpu%{ $* }
	if (! test -e /mnt/term/mnt/wsys) {	# cpu call from drawterm
		font=/lib/font/bit/pelm/latin1.8.font
		exec rio
	}
case con
	prompt=('cpu% ' '	')
}
```
NOTE: `/dev/label` write and `/mnt/term/mnt/wsys` probe ride drawterm's served devices; the `test -w /dev/label` guard already tolerates absence — but an rc FAILED redirect RAISES in a sourced file (18-05), so keep the guards exactly as staged.

### The donor service file + ndb rows
```text
# Source: sys/src/9k/k10/root/tcp17010 (verbatim, full file):
#!/bin/cpu -N -R
```
```text
# Source: sys/src/9k/root/common:261-263 (service-name rows; cs translates alpha names):
tcp=exportfs port=17007
tcp=ncpu port=17010
tcp=cpu port=17013
```
This phase's CONTEXT pins the script text as `#!/bin/cpu -R` (no `-N` — authenticated); if the `-G`/gate dials use the alpha name, an ndb `tcp=ncpu port=17010` row joins the composed `/lib/ndb/local` (the 18-05 `tcp=ticket port=567` precedent), else dial `17010` numerically (the 18-05 loopback idiom).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Carbon/QuickDraw macOS GUIs (vendored `gui-osx`, `-m32`) | 64-bit Cocoa/AppKit frontends (9front `gui-cocoa`, `CONF=osx-cocoa`; community Metal/HiDPI forks) | Carbon deprecated 2012, dead at macOS 10.15 (2019); arm64 macOS never had it [CITED] | darwin leg needs a Cocoa-class backend port into the vendor (OQ-1) |
| p9sk1 + rc4_256/sha1 sessions | dp9ik/TLS-class (9front rcpu) | PROTOCOL-blocked for Harvey 4e (REQUIREMENTS Out of Scope) | era-shape crypto stands; stronger crypto is the hardening milestone |
| connect-path sockets fixed (18-07, r9 ab4cb66) | accept-pool sockets still stock (CR-03 open) | open since 18-07 record | this phase's D-08 closes it |

**Deprecated/outdated:**
- `gui-osx` (Carbon) — unbuildable on any current macOS for arm64; do not attempt to "fix" it in place.
- `cpu-bl.c` in the vendor — the blast-era variant; not the phase's surface (D-01 names `cpu.c`).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `-O` is a NEW local vendor flag whose semantics this phase defines (recommend: the wrapper's pinned-connection flag; argv over env) | DESK-01 / Pitfall 2 | low — the flag is ours either way; only its UX contract needs pinning at plan time |
| A2 | `cmd/cpu` (mkone, no container) joins tier2-services cleanly like other single-binary service entries | Pitfall 6 | low-medium — if cpu.c's mkfile pulls unusual libs, the tier closure grows (measure at build) |
| A3 | The cpu image's factotum carries the hostowner's p9sk1 key via the ctl-write idiom proven in the auth sweep (`key proto=p9sk1 user=<owner> !password=...`), fed from the D-12 creds | cpurc composition | medium — if factotum -S instead requires the nvram path interactively, cpurc needs the sentinel-feed treatment (18's keyfs probe isolation precedent) |
| A4 | `boot(8)`'s `getenv("init")` override path works without a boot rebuild on the current chain boot binary (execinit reads it before the cpuflag arm) | Pattern 3 | low — fallback is the cpuflag=1 boot build (a second binary per image, tolerated by one-image-per-gate) |
| A5 | drawterm's `posix-amd64` machdep + vendored libc build cleanly under modern gcc/clang with only Makefile env overrides (CC/X11 paths) in nix | D-02 recipe | low-medium — old Makefiles; `-ggdb`/`-Wno-missing-braces` are benign; X11 include paths need the nix deps |
| A6 | devmnt's existing Tstat/Twalk/Topen over /mnt/term plus rio's measured syscall set (no Wstat in rio's table; sam uses Remove only) means the desktop path needs no kernel work beyond the three named syscalls | DESK-02/05 | low — the 15-06 tables are the measure-first evidence; surprises are owned under D-11 |

## Open Questions

> **(RESOLVED 2026-11-14 — all four settled at planning; the dispositions below supersede the per-item recommendations in this section.)**
> - **OQ-1 RESOLVED — owner decision, amended D-03 (2026-11-14):** darwin is `-G` text-mode only this milestone; the gui-cocoa-class port is NOT taken — the live recommendation below (option a, port gui-cocoa now) is superseded. Settlement: 19-06 (gui-none backend — no Carbon, no XQuartz; darwin -G documented-manual recipe) and 19-10 (darwin graphical row deferred with the named reopener: a future milestone porting a 64-bit Cocoa/AppKit backend of the 9front gui-cocoa class into the vendor; REQUIREMENTS/STATE deferral records).
> - **OQ-2 RESOLVED — 19-06:** `-O cpu=HOST,auth=ADDR,user=NAME` comma grammar; argv is the single source (env reads skipped when -O present); malformed specs are usage errors naming the accepted keys.
> - **OQ-3 RESOLVED — 19-07:** `aux/listen -t /bin/service tcp` in cpurc (CPU-03's letter, exercises `#!` through the donor path) with `listen1 -t` retained for flags-carrying authsrv; both announce markers asserted by the bring-up probe.
> - **OQ-4 RESOLVED — 19-09:** `--system=cpu|terminal` on boot-r9-x64, default terminal (chain path byte-untouched); unknown modes are usage errors naming the valid set.

1. **The darwin backend decision (OQ-1 — must resolve at plan time; D-03 as written is unsatisfiable)**
   - What we know: vendored `gui-osx` is 32-bit Carbon/QuickDraw; `-m32` unsupported on arm64 toolchains; QuickTime gone. D-03 says "native Cocoa ... aarch64-darwin, no XQuartz". 9front's `gui-cocoa` is a 64-bit AppKit frontend building natively on Apple Silicon [CITED]; Harvey's license posture permits 9front MIT imports.
   - What's unclear: whether the user wants (a) a gui-cocoa-class port into the vendor as a Harvey commit (keeps D-03's spirit; a real port task with its own risk), (b) the Make.osx-x11 + XQuartz path (contradicts D-03's letter), or (c) a scope change for the darwin DESK-07 leg.
   - Recommendation: present to the user at plan kickoff; default recommendation (a) — it preserves "in-tree vendor, no XQuartz, aarch64" and the port is bounded (screen.c/alloc.c/draw.c/keycodes against the vendor's existing libmemdraw screen interface).
2. **`-O` semantics (OQ-2)**
   - What we know: no `-O` exists in the vendor; DESK-01 requires the wrapper to "pin -O".
   - Recommendation: define `-O <conn-spec>` (or `-O` as "options pinned": the flag through which the wrapper passes cpu/auth/user so environment cannot override) in the plan's vendor-task, recorded in the README's flag table. Confirm with the user if any stronger meaning was intended.
3. **cpurc's listener shape: aux/listen vs listen1 (CPU-03's letter vs the 18-04 precedent)**
   - What we know: both are in-tree and tier-2-built; CPU-03 names aux/listen; 18-04's proven gate used `listen1 -t`. aux/listen `-t <dir> tcp` scans the trusted dir and execs service files by name (needs `#!` + `$cputype` env — seeded); listen1 carries flags on argv but announces exactly one port.
   - Recommendation: use `aux/listen -t /bin/service tcp` in cpurc (satisfies CPU-03's letter, exercises `#!` through the donor path) with `listen1 -t` retained for any flags-carrying one-off (e.g. authsrv if it moves into cpurc); assert both listeners' announce markers in the gate.
4. **Where the full-system mode lands on boot-r9-x64 (INT-05 dev UX)**
   - What we know: the app currently bakes the chain paq and boots interactive; INT-05 wants the ONE command to bring up net+auth+cpu.
   - Recommendation: a `--system=cpu`-style flag (or making the cpu image the default with `--system=terminal` retained) selecting the seventh-contract paq at step 2/3 — keep the chain image path untouched for the existing gates (no silent downgrade rule: unknown modes are usage errors naming the valid set).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| qemu-system-x86_64 | all gates + human session | ✓ | host package (/run/current-system) | — |
| /dev/kvm | D-14 required witness venue | ✓ | present | TCG explicit (R9_ACCEL=tcg) |
| nix | flake apps, package build | ✓ | 2.35.6 | — |
| host cc (gcc) | bake-time authdomain-gen (mkimage §14d) | ✓ | present | flake app carries stdenv.cc |
| gnumake | drawterm vendor build | ✓ | present | nix pkgs.gnumake in recipe |
| expect | -G gate + smoke extension | via flake runtimeInputs (not on interactive PATH) | nix pkgs | — |
| plan9port | mkimage venue (PATH-last) | via flake runtimeInputs (store path present) | 0-unstable-2025-11-10 | — |
| X11 dev headers (libX11, likely libXt for `IntrinsicP.h`) | gui-x11 drawterm build | declarative (nixpkgs) | — | — |
| r9 sibling repo (writable, `../r9`) | all kernel work | ✓ | at 9103522 == origin/main (verified) | — |
| aarch64-darwin machine | DESK-07 darwin leg (human) | ✗ not on this host | — | human venue per D-06/D-13; recipe must be exercised for real before the row is signed |

**Missing dependencies with no fallback:** none blocking planning; the darwin machine is a human-run venue by design.
**Missing dependencies with fallback:** none (expect/plan9port ride the flake app contract).

## Security Domain

`security_enforcement` absent in `.planning/config.json` → enabled; included. This is an OS-bring-up phase, not a web service — the ASVS mapping is correspondingly narrow.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | p9any→p9sk1 via factotum/authsrv (Phase 18 domain, DEMO-labeled ephemeral keys); drawterm client's vendored stack; `auth_chuid` + `#¤/capuse` (devcap, 18-02, eve-gated) |
| V3 Session Management | partial | session = the cpu connection; `listen`'s `becomenone` / trusted-arm choice; no cookie/session-token surface |
| V4 Access Control | yes | devcap caphash/capuse ownership model; hostowner (`#c/hostowner`) CR-01 eve-gate (18-06); namespace isolation via rfork/newns |
| V5 Input Validation | yes (device ctl planes) | devssl ctl verbs validated donor-verbatim (`Ebadarg` on unknown); ndb/libndb Brdline disciplines; `#p/N/args` writes bounded (donor `128` buffer) |
| V6 Cryptography | yes — DO NOT hand-roll | rc4_256/sha1 via libsec donors; **known-weak by modern standards (rc4): accepted era-shape, PROTOCOL-blocked from stronger (DESK-04 requirement text; hardening milestone owns dp9ik/TLS)** |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| DEMO credentials leaking into git | Information Disclosure | D-12: creds only to gitignored bake output; feed discipline (14f2b: rm-before-cp, read-only, gate redaction T-18-13 lineage) |
| Credential-bearing env override of the connect target | Spoofing | DESK-01 wrapper scrubs `$cpu`/`$auth`/`$PASS`; argv (pinned `-O`) is the single source |
| rc4_256 session crypto weakness | Information Disclosure | accepted + documented (`-e clear` even weaker — debug only); PROTOCOL-blocked upgrade; revisit in the hardening milestone |
| Network surface widening | Elevation / Reconnaissance | hostfwd pair stays 127.0.0.1-bound (16-D-04); guest off external networks (16-D-08); darwin rides SSH tunnels (D-05) — no new listeners beyond tcp17010/567/5356 |
| Untrusted-listener namespace escape of auth material | Privilege Escalation | trusted-arm-only for cpu/auth services (Pattern 1); `becomenone` for any future untrusted service class |
| devssl/devcap ctl plane injection | Tampering | donor-verbatim verb validation; `fd` before `alg` ordering enforced (`must set fd before algorithm`) |

## Sources

### Primary (HIGH confidence — read this session)
- `sys/src/cmd/cpu.c` (full), `sys/src/cmd/unix/drawterm/cpu.c` (full), `main.c` (full) — CPU-04 client+server flows verbatim
- `sys/src/cmd/aux/listen.c`, `listen1.c` — listener mechanics, trust arms, execl shape
- `sys/src/cmd/init.c` (service selection), `sys/src/9k/boot/boot.c` (execinit), `bootr9.c` (full), `bootauth.c` — CPU-02 seam
- `sys/src/9k/port/sysproc.c:282-324,414` — `#!` donor algorithm
- `sys/src/9k/port/devssl.c` (structure, alg tables, ctl parsing), `sys/src/libc/9sys/pushssl.c` (full) — DESK-04
- `sys/src/cmd/exportfs/exportsrv.c` (Xremove/Xwstat), `sys/src/libauth/auth_chuid.c`, `auth_proxy.c`
- `sys/src/9k/k10/root/{tcp17010,k10root.rc}` — service file + cpurc donor
- `build/x86_64/mkimage.sh` (§14 auth contract in full), `auth-termrc`, `auth-sweep.rc` (legs), `root-tree/{termrc,usr/glenda/lib/profile,lib/netaddr.rc}`
- `nix/boot-r9-x64.sh` (full), `nix/smoke-r9-auth-x64.sh` (creds lineage), `flake.nix` (app inventory)
- `../r9`: `port/src/devip/stack.rs` (AcceptPool verbatim), `port/src/devip/proto.rs` (18-07 fix), `x86_64/src/syscall.rs` (DISPATCH), `x86_64/src/main.rs` (seed_root_group), `port/src/devcap.rs` (precedent), `x86_64/src/platform/q35/timer.rs`, aarch64 `platform/virt/timer.rs:130`, CI workflow
- 15-06 trace tables (rio/acme/sam) — measured syscall sets
- STATE.md / REQUIREMENTS.md / PROJECT.md — disciplines, CR-03 record, #66 waiver terms

### Secondary (MEDIUM confidence)
- [The Eclectic Light Company — Depreciation](https://eclecticlight.co/2023/06/30/deprecation) — Carbon status on Apple Silicon
- Apple Developer docs — universal macOS binaries / arm64
- [Plan9-Archive/drawterm-metal-cocoa](https://github.com/Plan9-Archive/drawterm-metal-cocoa), [rcarmo/drawterm](https://github.com/rcarmo/drawterm) — 9front gui-cocoa arm64 build shape (`CONF=osx-cocoa make`)

### Tertiary (LOW confidence)
- none — no claim in this document rests on unverified training knowledge alone

## Metadata

**Confidence breakdown:**
- Standard stack (in-tree composition): HIGH — every component read in source this session
- Architecture (boot/service selection, listener trust, auth topology): HIGH — all three `service=cpu` seams verified in donor source; the recommendation among them is judgment (flagged)
- Pitfalls: HIGH for repo-internal facts; MEDIUM for the two web-verified darwin facts (CITED)
- Drawterm vendor build under nix: MEDIUM — Makefiles read, not yet built (A5)

**Research date:** 2026-11-14
**Valid until:** 2026-12-14 (in-tree facts stable at the pin; darwin web facts stable)
**Graph note:** the knowledge graph (87,506 nodes) is 40 commits stale; treated as approximate — all load-bearing findings re-verified by direct file reads.
