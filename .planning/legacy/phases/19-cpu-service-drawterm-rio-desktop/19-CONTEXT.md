# Phase 19: CPU Service + Drawterm + rio Desktop - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

<domain>
## Phase Boundary

The v2.1 payoff: the x86_64 guest becomes a real cpu server — r9 execs `#!` scripts, `service=cpu` arrives via the boot-protocol plan9.ini equivalent so cpurc runs, and aux/listen + tcp17010 (`#!/bin/cpu -R`) + exportfs are live at boot on the flush-out-hardened kernel. The ONE command (`nix run .#boot-r9-x64`, the x64 boot app in full-system mode) brings up net + auth + cpu listener together. Host-side, `nix run .#drawterm` builds from the IN-TREE vendored drawterm, connects through the Phase-16 hostfwd pair (127.0.0.1:17010/5670), authenticates p9any→p9sk1 against the Phase-18 domain, and lands the user in an interactive rio desktop that starts ONLY in-session (profile `case cpu` binds /mnt/term/dev over /dev). The `-G` no-window text mode carries the CI-able interop gate (scripted login → shell marker); devssl `#D` (rc4_256/sha1) makes every ordinary session encrypted with `-e clear` the documented fallback. Close-out: full-chain smoke extension, the r9 pin advance as its own commit with a fresh hosted CI run, and the clean-checkout proof of the whole v2.1 chain on linux-x86_64. **Work spans both repos**: r9 carries `#!` exec, WSTAT/FWSTAT/REMOVE, devssl `#D`, the CR-03 AcceptPool TCP fix, and proactive tick_check wiring (kernel work in `../r9`, branch `main`, push-before-pin).

Addresses CPU-01, CPU-02, CPU-03, CPU-04, DESK-01, DESK-02, DESK-04, DESK-05, DESK-06, DESK-07, INT-05, INT-06, INT-07 (DESK-03 staged complete in Phase 15).

NOT in this phase: aarch64 anything (frozen at the v2.0 pin), TLS/security modernization (rc4_256/sha1 is the era shape; stronger crypto is PROTOCOL-blocked per DESK-04), fossil/venti runtime (no block device), factotum-as-client rework beyond what the vendored drawterm already carries, new host networking surfaces (no LAN exposure, no doctor additions — the darwin leg rides SSH tunnels over the existing loopbound hostfwd), rio at boot on the terminal path (in-session only), the #66 cure (the class stays waived with its reopener policy recorded below).

</domain>

<decisions>
## Implementation Decisions

### Drawterm provenance & build (DESK-01, DESK-06)
- **D-01:** **The in-tree vendored drawterm at `sys/src/cmd/unix/drawterm/` is the source of record** — the classic Plan 9 4e lineage carrying Harvey-era fixes (history back to the 2000 3e import; Harvey fixes like 5c87831c6; upstream maintenance by 0intro at github.com/0intro/drawterm is NOT re-vendored). The vendor already has everything the phase needs: p9any→p9sk1 client auth in `cpu.c`, the `-e` encryption flag (DESK-04's `-e clear` rides the existing surface), `gui-x11` + `gui-osx` backends, Unix Makefiles. This IS the provenance decision DESK-01 requires recorded. — **Reversibility:** costly — the `-G` addition, the nix recipe, the wrapper, and the CI gate all key to this source's flag surface and 9P/auth corners; switching lineage later re-touches every one of them.
- **D-02:** **Nix builds the in-tree vendor — no flake-input fetch.** A nix package recipe compiles `sys/src/cmd/unix/drawterm` directly (the 9cc / 9front-toolchain vendored-at-pin precedent), so the build is hermetic with no network fetch, and local modifications (`-G`, any interop fixes) land as normal Harvey commits on the vendor — no patchPhase indirection. The flake `drawterm` app wraps this package for linux.
- **D-03:** **darwin's backend is native Cocoa (`gui-osx`)** built for aarch64-darwin — no XQuartz dependency on the darwin leg. The `Make.osx-x11` path exists but is not part of this phase's surface. Per D-06 the darwin build is documented-manual, so Cocoa is exercised through the README recipe, not a flake output.
- **D-04:** **`-G` is a no-window stdio mode**: drawterm connects, authenticates, mounts /mnt/term, and the session's stdio rides the invoking terminal — real terminal semantics, no offscreen buffer machinery. The DESK-06 gate drives a scripted login → shell marker through exactly this path. The flag is a small local addition to the vendor's `cpu.c` arg parsing (D-01/D-02 posture: we own the source).

### Darwin leg reachability (DESK-07)
- **D-05:** **The darwin drawterm reaches the linux-hosted guest via SSH local port-forward** — `ssh -L 17010:127.0.0.1:17010 -L 5670:127.0.0.1:5670 user@linuxhost`, then drawterm connects to darwin's own loopback. Zero new network surface: the slirp hostfwd pair stays 127.0.0.1-bound (16-D-04) and the guest stays off external networks (16-D-08); SSH carries the transport encryption; no host config changes, no doctor additions. Documented in the README's darwin section as the DESK-07 recipe. — **Reversibility:** reversible — a documented recipe; a different venue later adds a paragraph, not code.
- **D-06:** **darwin drawterm is a documented-manual build** (the darwin documented-manual venue convention): the flake `drawterm` app is linux-only; the darwin leg builds the same vendored source (Cocoa backend, D-03) via the README recipe. Deliberate scope-keep: no aarch64-darwin flake surface this phase.
- **D-07:** **A human drawterm session runs against the interactive `nix run .#boot-r9-x64` left running in a terminal** on the linux host — zero new machinery. The README documents the session recipe (boot → connect → accept → teardown). No dedicated headless session app.

### Kernel-flush posture (CPU-*, DESK-04; the recorded reopeners this phase walks into)
- **D-08:** **The CR-03 AcceptPool TCP fix is a planned first-class item, not a contingency.** tcp17010 is an announced connection (listen1-accept → `AcceptPool::spawn_listener` in r9 `stack.rs`), and exportfs bulk writes >2048 bytes into announced sockets park forever under stock smoltcp Nagle + 10ms delayed-ACK (the 18-07 finding). The fix is the two `set_*` calls at the pool site + the WR-04 pin legs, exactly as the 18-REVIEW-DISPOSITION record names. Sequenced as a dependency of the cpu/exportfs work — the gates cannot go green without it.
- **D-09:** **#66/CR-07 firing policy: honor the reopener only if the waiver's capture context is actually reached.** If the waived triple-fault class fires during 19's work AND the context the waiver names is captured (e.g. an exception-cascade capture on the new workload, or a TCG-leg repro), the reopener condition is met — root-cause in-phase. If it fires without context: record-enrich and continue. The phase's success criteria never hinge on killing the class (the 17-08 maintainer-ratified waiver stands).
- **D-10:** **[USER OVERRIDE of the recommended reopener-stands] tick_check is wired proactively this phase.** The x86_64 tick path gains the `tick_check` call, closing the recorded adjacent latent defect (retransmit/keepalive/connect-timeout timers never fire under park) BEFORE long-lived authenticated drawterm sessions can hang on it. Long-lived authenticated sessions are the classic timer-mediated-TCP consumer; the 18-07 record named this phase as the natural reopener and the user chose to close it now rather than wait for a stall. Pins ride the same host-test/boot-profile shapes as the rest of the kernel work.
- **D-11:** **Umbrella: fix everything surfaced** — any kernel defect 19's load surfaces is root-caused, fixed push-before-pin, and pinned in-phase (the Phase 14-17 doctrine). cpu/listen/exportfs/devssl/exercise fresh kernel surface; surprises are expected and owned.

### Gate credentials + acceptance proof (DESK-06, DESK-07, DESK-01 wrapper)
- **D-12:** **The Phase-18 domain generator additionally emits the demo credentials to a gitignored file in the bake output** (demo user + password, e.g. `out/auth-creds.txt` shape — planner's naming), and the `-G` gate reads it post-bake. 18-D-01's "real randomness" ephemeral-DEMO contract is preserved verbatim; nothing secret-shaped lands in git; hosted CI works with zero committed credentials. The human legs use the developer's local-file domain (the 18-D-01 shape).
- **D-13:** **DESK-07 evidence = one screenshot per leg + a signed human-acceptance row in `19-EVIDENCE.md`** — the rio desktop visible in the drawterm window on linux and on darwin, each row carrying date, venue, r9 SHA, and who accepted (the 17-08 maintainer-ratification record style). No screen recordings required.
- **D-14:** **[USER OVERRIDE of the recommended hosted-CI-required] The `-G` interop gate's required green witness is the local KVM run; hosted CI runs it best-effort.** INT-06's fresh hosted CI run after the pin advance still happens (the pin-advance discipline is untouched) — it simply is not the `-G` gate's required venue. Local KVM is the interactive drawterm session's real venue; green-twice on KVM is the gate's bar.
- **D-15:** **Encrypted default.** The wrapper's printed "exact connect line" and every ordinary session default to encrypted — devssl `#D` rc4_256/sha1 via pushssl (DESK-04 proven live by ordinary use, not just a dedicated probe) — and the `-G` gate rides encrypted too. `-e clear` stays the documented debugging fallback. (The vendor's existing `-e` flag surface carries this; no new flag machinery.)

### Carried forward (locked in prior phases / roadmap — do not re-litigate)
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

### Claude's Discretion
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase definition (Harvey repo)
- `.planning/ROADMAP.md` §Phase 19 — goal, success criteria 1–5, requirement mapping; the Progress table's execution order (19 strictly last)
- `.planning/REQUIREMENTS.md` — CPU-01..CPU-04, DESK-01/02/04..07, INT-05..07 full text + coverage table (DESK-03 already Complete via Phase 15)
- `.planning/PROJECT.md` — Key Decisions (push-before-pin, gates-split, venue honesty, FOD discipline rows), Constraints
- `.planning/STATE.md` — Accumulated Context (the 18-07/CR-03 reopener row, 18-05 loopback idiom, 15-06 trace pointers, 16 net-mode decisions); Blockers/Concerns (CR-03 open advisory, #66 waiver terms)

### Prior-phase context (the machinery this phase rides)
- `.planning/phases/18-in-guest-p9sk1-auth-domain/18-CONTEXT.md` — D-01 (DEMO material contract D-12 extends), D-03 (nvram env), D-08/D-11 (sixth contract; 19 composes its own), D-12 (three baked users), D-13 (speaksfor probe — the cpu path's non-hostowner shape)
- `.planning/phases/16-host-net-modes-networking-exit-gate/16-CONTEXT.md` — D-04 (hostfwd 17010/5670 remap), D-08 (guest off external networks — D-05's SSH-tunnel rationale)
- `.planning/phases/15-userspace-tiers-bootfs-payload-amd64/15-CONTEXT.md` — the DESK-03 payload rows, the rio/acme/sam donor syscall traces (the measure-first input), D-17 (profile shape)
- `.planning/phases/17-r9-kernel-flush-out/17-CONTEXT.md` — the fix-everything/pin doctrine D-11 executes; the ledger-closure vocabulary

### Drawterm vendor + guest-side donors (in-tree — the fidelity references)
- `sys/src/cmd/unix/drawterm/cpu.c` — THE client: p9any→p9sk1 flow, `-e` encr flag surface, remoteside protocol, exportfs launch; where `-G` lands (D-01/D-04)
- `sys/src/cmd/unix/drawterm/main.c`, `gui-x11/`, `gui-osx/`, `Make.unix`/`Make.osx` — the build surface D-02/D-03 target
- `sys/src/cmd/cpu.c` — the guest-side remoteside binary (CPU-01's `#!/bin/cpu -R` target; the CPU-04 flow: p9any→p9sk1 → auth_chuid → client exportfs at /mnt/term → stdio from /mnt/term/dev/cons → `rc -li`)
- `sys/src/cmd/exportfs/` (exportfs.c, exportsrv.c, pattern.c) — what tcp17010 serves; the WSTAT/FWSTAT/REMOVE consumers (CPU-03)
- `sys/src/cmd/aux/listen.c` + `listen1.c` — the listener machinery (`listen1 -t` trusted-arm precedent from 18-04's AUTH bake)
- `cfg/example/cpurc` — the cpurc donor CPU-02 re-keys
- `sys/src/cmd/rio/` + `sys/src/cmd/plumb/` — the in-session desktop (DESK-02/DESK-05; rio rides /mnt/term/dev, plumb runs in-session)
- `sys/src/9k/port/devssl.c` (legacy twin `sys/src/9/port/devssl.c`) — the devssl donor of record for r9 `#D` (DESK-04)
- `sys/src/libsec/port/` (rc4.c, sha1.c class) — the algorithm donors behind rc4_256/sha1

### r9 repo (sibling, `../r9` — where the kernel substrate lands)
- `../r9/port/src/devip/` — `stack.rs` `AcceptPool::spawn_listener` (the CR-03 fix site, D-08); `tcp_socket`'s 18-07 donor-shape fix (Nagle off, ACK immediate — the connect-path precedent the pool fix mirrors)
- `../r9/port/src/devcap.rs` — the small-device port precedent devssl follows (18-D-06)
- `../r9/x86_64/src/` — the tick/timer path D-10's `tick_check` wiring joins (the 13-03 timer lineage)
- `../r9/xtask/` — the boot-profile chassis for kernel-side pins (D-08/D-10/D-11 evidence)

### Build machinery (Harvey repo — the surfaces this phase changes)
- `build/x86_64/mkimage.sh` — the six existing contracts (auth = §14); the seventh cpu contract joins; the services contract remains the tier-staging precedent
- `build/x86_64/root-tree/rc/bin/termrc` + `build/x86_64/root-tree/usr/glenda/lib/profile` — the boot script + cpu-aware profile DESK-02 rides (15's pre-staged shape)
- `nix/smoke-r9-auth-x64.sh` — the composed auth-gate lineage the `-G`/full-chain gates mirror (creds-file reading joins here per D-12)
- `nix/boot-r9-x64.sh` — the ONE-command surface INT-05 extends to full-system mode
- `flake.nix` — the drawterm app (D-02), the `-G` gate app, the full-chain smoke extension (app-per-gate shape)
- `build/x86_64/root-tree/lib/netaddr.rc` — the single addressing seam (the -G dial line's loopback literal rides the 18-05 idiom)
- `build/x86_64/README.md` — the drift-notes home: drawterm, the darwin SSH-tunnel recipe (D-05/D-06/D-07), the cpu-service sections land here

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- The vendored drawterm already carries the full client stack: p9any/p9sk1 (`cpu.c`), its own libauthsrv/libsec/libmp/libdraw, `gui-x11` + `gui-osx` backends — the phase patches (`-G`) and builds; it does not write a drawterm
- Harvey's guest side is fully in-tree and tier-built or tier-buildable: `cpu.c` (remoteside), `exportfs/`, `aux/listen`/`listen1`, rio, plumb, the whole auth suite from Phase 18
- Phase 18's compose is directly extensible: the sixth contract's bake (pairing check → keyfs → authsrv → secstored → probe) is the bring-up shape cpurc generalizes; `smoke-r9-auth-x64.sh` is the expect-gate lineage
- The hostfwd pair (17010/5670) is committed TOML data — drawterm dials day one with zero qemu-side work
- r9's established `cfg(test)` host-pin homes + xtask boot-profile chassis carry D-08/D-10/D-11 evidence; the 18-07 `tcp_two_write_client_pattern_round_trips` pin is the exact WR-04-leg precedent

### Established Patterns
- Vendored-at-pin + nix recipe (9cc, 9front toolchain) — D-02's build posture
- One implementation, committed interface, mechanical asserts; fail-loud + no silent downgrade (the R9_NET/R9_ACCEL lineage the drawterm wrapper joins with its env scrubbing)
- App-per-gate + one-image-per-gate; presence-based timing-tolerant expect assertions; green-twice; sourced-not-typed guest scripts
- Push-before-pin + single FOD re-wire pass at kernel-byte end; pin advance its own commit + fresh hosted CI

### Integration Points
- `../r9` `main` branch: `#!` exec, WSTAT/FWSTAT/REMOVE, devssl `#D`, AcceptPool fix, tick_check wiring (push-before-pin; one FOD re-wire; INT-06 pin discipline)
- `build/x86_64/mkimage.sh` seventh contract + `flake.nix` apps (drawterm, -G gate, full-chain smoke) + `nix/boot-r9-x64.sh` full-system mode
- Phase 18's generator (creds emission, D-12) and domain artifacts — consumed as-baked, not rebuilt
- `build/x86_64/README.md` — the drawterm/darwin/cpu documentation home

</code_context>

<specifics>
## Specific Ideas

- **The acceptance story, end-to-end:** r9 gains `#!` exec + WSTAT/FWSTAT/REMOVE + devssl `#D` (donor-verbatim semantics, host pins + boot profiles), the CR-03 AcceptPool fix (two `set_*` calls + WR-04 pin legs — the dependency, D-08), and proactive tick_check wiring (D-10) → the FOD re-wire pass + pin advance as its own commit + fresh hosted CI (INT-06) → the seventh mkimage contract composes the cpu image (chain + auth suite + cpu/exportfs/listen + the re-keyed cpurc + 15's pre-staged profile/payload), `service=cpu` delivered via the boot-protocol plan9.ini equivalent (CPU-02) → the ONE command boots net + auth + cpu listener together; boot-r9-smoke's extension asserts the full chain (INT-05) → the vendored drawterm gains `-G` and a nix recipe (D-02/D-04); the wrapper pins `-O`, scrubs `$cpu`/`$auth`/`$PASS`, prints the encrypted-default connect line (D-15) → the generator emits demo creds (D-12); the `-G` expect gate scripts login → shell marker over hostfwd, green-twice on local KVM (D-14) → the cpu -R remoteside flow proves end-to-end with serial evidence (CPU-04) → rio starts ONLY in-session (profile `case cpu`; serial marker proves it did NOT start at boot; plumb runs in-session — DESK-02/05) → human graphical acceptance on linux and darwin (SSH tunnel, documented-manual Cocoa build, interactive guest — D-05/06/07): screenshot + signed row per leg in 19-EVIDENCE.md (D-13) → the clean-checkout proof closes v2.1 (INT-07).
- **Watch-out:** the `-G` gate's dial line and any in-guest dial fixtures must use the 18-05 loopback idiom (literal `127.0.0.1`-style addressing) — the `$attr` cs translation reds through r9 kernel namec; do not plan `$tcp!$cpu!...` variable-form dials without verifying cs resolution first.
- **Watch-out:** exportfs writes into the accepted tcp17010 connection are exactly the CR-03 class — if the D-08 fix slips late in the wave order, every downstream gate run pays a diagnosis cycle. Sequence the fix before the end-to-end legs.
- **Watch-out:** the auth image deliberately runs the pairing check before keyfs (18-D-02) and cpurc now layers listeners on top — keep the fail-loud ordering (pairing → keyfs → authsrv → listen) so a bad bake never wedges behind an interactive prompt.
- **Watch-out:** drawterm's `askuser` password path is interactive on the terminal — the `-G` gate must drive it via expect on a pty (the 16-05 host-tool findings: spawned ptys stay canonical; read exact byte counts), never a piped stdin feed.
- **Watch-out:** the darwin documented-manual build must be exercised for real on the darwin machine before the DESK-07 row is signed — a recipe that was never run is not evidence (venue honesty).
- **Watch-out:** `nix run .#boot-r9` in INT-05's roadmap phrasing means the x64 boot app — do not re-open or alias the frozen aarch64 `boot-r9` app; the full-system mode lands on `boot-r9-x64`.
- **User overrides to honor downstream (deliberate, don't second-guess):** tick_check fixed proactively (D-10); the -G gate's required witness is local KVM, hosted CI best-effort (D-14).

</specifics>

<deferred>
## Deferred Ideas

- **The #66/CR-07 cure** — stays waived under the 17-08 ratification; D-09 records this phase's firing policy (root-cause only if capture context reached); the reproduction instrument stays committed.
- **A headless cpu-session app** (qemu -display none + lifecycle management) — declined this phase (D-07); a demand-named future item if session hosting becomes routine.
- **A native aarch64-darwin drawterm flake app** — declined this phase (D-06); the documented-manual recipe is the venue; revisit if darwin becomes a first-class interactive host again.
- **LAN-exposed guest networking** (hostfwd rebind / tap-on-LAN) — declined (D-05); SSH tunnels suffice; any LAN exposure is its own future decision with doctor support.
- **rio at boot on the terminal path** — out of scope by requirement (in-session only); a future phase owns any terminal-GUI revival if ever demanded.
- **Stronger drawterm crypto (TLS-class)** — PROTOCOL-blocked per DESK-04's requirement text; the hardening milestone's item, not v2.1's.

</deferred>

---

*Phase: 19-CPU Service + Drawterm + rio Desktop*
*Context gathered: 2026-11-14*
