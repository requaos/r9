# Pitfalls Research — Milestone v2.0 "Full System & Drawterm"

**Domain:** Plan 9 userspace port completion + OS networking/auth/desktop bring-up (Harvey OS on r9, aarch64, qemu `-M virt`)
**Researched:** 2026-11-04
**Confidence:** HIGH (grounded in this repo's v1.0 artifacts + primary-source protocol/qemu docs; see Sources)
**Supersedes:** the v1.0 pitfalls research previously at this path (retrievable from git history; v1.0 pitfalls 1–14 remain valid background — this document covers only the NEW failure modes of v2.0's feature set)

**Phase-area labels used below** (roadmapper assigns numbers):
- **USERSPACE** — full aarch64 userspace: remaining libs + all of `sys/src/cmd` via vendored 7c
- **NETDRV** — r9 virtio-net driver + TCP/IP path
- **HOSTNET** — tap/bridge/vmnet host plumbing + docs
- **AUTH** — in-guest p9sk1 domain (auth server, keyfs, factotum, ndb)
- **DESKTOP** — cpu service tcp/17010 + drawterm + rio session
- **FLAKE** — host drawterm packaging, flake apps, privileged-setup boundary

## Risk Ranking at a Glance

| # | Pitfall | Likelihood | Blast radius | Phase area |
|---|---------|-----------|--------------|------------|
| 1 | Compile-green ≠ run-correct at 250-command scale (whack-a-mole) | Certain | milestone schedule | USERSPACE |
| 2 | Target-compiled generators can't run on the build host | High | build hangs/false red | USERSPACE |
| 3 | aarch64 bug-class catalog (unsigned char, varargs width, LP64 fossils) | Certain (per-file) | per-file silent corruption | USERSPACE |
| 4 | Known latent shared-frontend landmines (gethunk ×2, hswal) fire at full-tree scale | High | toolchain hangs mid-wave | USERSPACE |
| 5 | ape scope creep | Medium | eats a phase | USERSPACE |
| 6 | virtio feature-negotiation landmines (CSUM/TSO/MRG_RXBUF; "ping works, TCP hangs") | High | days of phantom debugging | NETDRV |
| 7 | Virtqueue memory-ordering races on aarch64 | Medium | corruption under load only | NETDRV |
| 8 | GICv2 virtio interrupt plumbing (INTx level SPIs, ISR ack, no-MSI, DTB) | High | driver wedges silently | NETDRV |
| 9 | slirp masks L2 bugs; mode confusion when switching to tap | High | false "networking done" | HOSTNET |
| 10 | darwin vmnet entitlement wall + privileges can't be hermetic | Certain on darwin | blocked macOS path | HOSTNET + FLAKE |
| 11 | Auth first-boot chicken-and-egg without NVRAM | Certain | auth phase blocked at step 0 | AUTH |
| 12 | p9sk1 "authentication failed" opacity (authdom/hostowner/speaksfor triangle) | High | days of opaque failures | AUTH |
| 13 | 9front drawterm defaults to rcpu/TLS/17019 + dp9ik | Certain if 9front drawterm chosen | instant interop failure | DESKTOP + FLAKE |
| 14 | rio started in the wrong script (cpurc vs profile); fonts missing from bootfs | High | blank/failed desktop | DESKTOP |
| 15 | Debugging the TCP stack through the drawterm session that rides it | Medium | undebuggable wedges | NETDRV + DESKTOP |
| 16 | Cross-repo drift: qemu netdev flags, pin-vs-HEAD, staged-bake pollution | High | green gate, broken compose | all (process) |

---

## Critical Pitfalls

### Pitfall 1: Compile-green ≠ run-correct at 250-command scale — the whack-a-mole death spiral

**What goes wrong:**
`nix run .#build` flips to "all of `sys/src/cmd`" and the milestone degenerates into 250 sequential compile-fix-commit loops, each discovered one `mk` failure at a time; worse, the dirs that DO compile ship silently-broken binaries (the v1.0 lesson: build-green masked silent kernel miscompilation — only execution exposed it). Compile gates alone prove nothing about a 7c-produced binary.

**Why it happens:**
The v1.0 subset was 16 commands chosen to be easy. The remaining ~234 include the tree's notorious residents. The default build is dual-arch (`arch=(amd64 aarch64)`, Phase 2 decision), so every broken aarch64 dir breaks the default build for every other workstream — pressure to "just fix it inline" destroys ordering discipline.

**Notorious directories (in-repo evidence + 4e/9front lineage):**

| Dir | Why it's notorious | Handling |
|-----|--------------------|----------|
| `jpg`, `jtagfs` | Phase-1 head blockers (generator mkfiles; `jpg/mkfile:56-68` `oc`-from-`cputype=unix` class) | already fixed for amd64; re-verify generator fix is host-toolchain-correct for aarch64 (Pitfall 2) |
| other-arch compilers `4a–9l`, `qa–ql`, `va–vl` | big cc-frontend C; pointless-but-harmless as *target* binaries; their mkfiles sit in DIRS sweeps | exclude toolchain letters from the aarch64 target sweep (host tools via BUILDTOOLCHAIN only); don't 7c-compile them this milestone |
| `ape/` | NOT `sys/src/cmd` — its own libc, own syscall stubs, own arch dirs; a separate port | Pitfall 5 — exclude explicitly |
| `games/` | `mahjongg` cpp breakage is a known upstream TODO; `snes`/`gb` carry per-arch asm | tier-4; per-game gates; fix or exclude by name |
| `gs` (ghostscript) | huge, non-Plan-9 build assumptions, GPL-fonts entanglement | exclude from tiers; document as known gap (like Go) |
| `upas/` | portable C but drags libauth/libauthsrv/TLS adjacency | build-only gate; no send/receive UAT this milestone |
| `lp/` | postscript pipeline → ghostscript adjacency | build-only; exclude gs-dependent stages |
| `libhtml`, `libhttpd` | big portable C, heavy varargs printers | Pitfall 3 classes; no arch code — low real risk, high volume |
| `libdraw`/`libmemdraw`/`libmemlayer`/`libframe` | per-arch asm dirs exist for some arches; mkfiles must fall back to portable C for aarch64 | verify mkfile arch-asm guards before the GUI wave |
| `aux/vga`, `usbd`, `disk/prep` neighbors | x86-hardware-only or need kernel subsystems r9 doesn't have | exclude by name with a comment; don't "fix" them |
| `prof`, `snap` | build fine; RUN needs kernel profiling / a draw device | build + `-h`-style smoke only; runtime UAT lives in DESKTOP era |

**How to avoid:**
- **Tiered waves with an allowlist, never a flag-flip.** The aarch64 sweep grows monotonically: a dir enters the default build only after passing its per-dir gate. Tier 0: remaining libs (auth-adjacent first: libsec/libmp/libauth/libauthsrv/libndb — AUTH needs them later; then libdraw family for DESKTOP). Tier 1: core cmds. Tier 2: services (`ip/`, `ndb/`, `auth/`, `aux/listen`). Tier 3: GUI (`rio`, `acme`, `sam`, `plumb`). Tier 4: games/misc. ape/gs excluded by decision.
- **Execute each tier on the 9front donor kernel FIRST** (the Phase-2/5 subset harness already does this): the donor runs a full 9front userspace, so a Harvey binary that fails there is a Harvey/7c bug, not an r9 gap — this decouples the two confounders. Divergent syscalls (NMOUNT=54 class) wall loudly; keep the Phase-2 divergence ledger and expect it to grow.
- **Then the same tier on r9** via the measure-first rule (v1.0 D-14/05-03): trace the tier's real syscall footprint on the donor, extend r9's dispatch to the measured set, never speculate.
- Extend `build/aarch64/`'s per-command marker pattern (the SUBSET-OK harness) to per-tier markers so a regression names the dir and the binary.

**Warning signs:**
A build fix commit whose only evidence is "now compiles"; the default dual-arch build breaking on someone's unrelated checkout; the same bug class fixed in three dirs without a grep for the fourth; `mk` exit status trusted (known: `nix run .#build` exit status is always 0 — Phase-2 deferred item; artifact checks are the gate).

**Phase to address:**
USERSPACE (the tier structure IS the phase structure; this pitfall argues for splitting USERSPACE into ≥2 phases: libs+core, then GUI+services+games).

---

### Pitfall 2: Cross-compiled generators can't run on the build host

**What goes wrong:**
Mkfiles with compile-and-run generators (`jpg`'s `ycbcr.h`, `awk`'s `proctab.c`, the `jtagfs` class, v1.0 Pitfall 1) build the generator with `$CC`. For `objtype=aarch64` that's `7c` — producing a `.7` binary the darwin/linux build host **cannot execute**. In v1.0's amd64-in-a-namespace-on-x86_64 flow, host==target so the bug was invisible; for aarch64 it fires on every generator dir.

**Why it happens:**
mkfiles were written for self-hosting Plan 9 where target==host by definition. Phase 1 fixed the *derivation* bugs (`oc` from `cputype=unix`); the *host-vs-target toolchain selection* bug is the aarch64-specific twin.

**How to avoid:**
- Audit every `mkfile` in the tier list for generator recipes (`<|`, `:$name: $O.$tool`, build-then-run shapes) BEFORE the wave that includes the dir — the Phase-1 normalize-on-awk decision (`objtype=$cputype mk <tool>.$cputype`) is the established pattern; extend it: generators always build with the host toolchain (pcc/9c into the per-OS host bin dir), never `$CC`.
- Gate: `file`/`nm` the generator artifact in CI — a `.7` magic generator in the build log is an automatic red.
- grep starter set: `mkfile`s containing `<\|` or recipes whose target is executed by a later recipe.

**Warning signs:**
`mk` failures like `cannot execute binary file` / exec-format errors inside namespace builds; a generator "working" on an aarch64-linux host (can run .7 via qemu-user binfmt on some systems!) but failing on darwin — host-dependent green.

**Phase to address:**
USERSPACE, wave 0 (the audit is a Wave-0 task before any tier starts; it is cheap and total).

---

### Pitfall 3: The aarch64 bug-class catalog — unsigned char, varargs width, LP64 fossils, unaligned struct punning

**What goes wrong:**
Per-file silent misbehavior in code only ever exercised on 386/amd64. The classes:

| Class | Mechanism | Detection pattern |
|-------|-----------|-------------------|
| **Unsigned char on ARM** | Plan 9 ARM compilers (5c/7c lineage, ARM ABI) make plain `char` UNSIGNED; x86 makes it signed. EOF/−1 idioms (`char c = Bgetc(...); if(c < 0)`, `if(c == 0xFF)`) silently never-fire; signed-shift-through-char checksums differ | grep for `char` locals fed by `Bgetc/getc/read` returns and compared against negatives or `0x80–0xFF`; any `char` arithmetic without a cast |
| **Varargs width** | Plan 9 varargs promote per the callee's `va_arg` width; callers passing `int` where callee reads `long`/`vlong` (or vice versa) worked by luck on 32-bit, corrupts the stack on aarch64 | `#pragma varargck` covers printf-like ONLY; hand-rolled `va_arg` users (libbio Bprint wrappers, libhttpd, libhtml) need eyeballs |
| **LP64 fossils in donor/ape-imported code** | code assuming `long`==64-bit breaks on Plan 9 where `long` is 32-bit everywhere (`vlong` is 64) — inverted vs unix LP64; the 9cc libmach `hswal` stride bugs beyond commonllp64 (Phase-2 acknowledged deferred) live here | any `#ifdef` on unix LP64 macros; `sizeof(long)==8` assumptions in ported code |
| **Struct punning / unaligned** | `*(ulong*)(buf+1)` on aarch64: qemu TCG tolerates unaligned Normal-memory loads, so this USUALLY works — until it lands on Device memory or gets optimized into an aligned-only instruction; don't rely on tolerance | grep `*(u*` casts into byte buffers in libip/libndb/wire codecs; fix to `GET32`-style byte assembly only where a gate proves it wrong |
| **Endianness** | aarch64 and amd64 both LE — near-zero risk; the trap is assuming this milestone PROVED BE-cleanliness | none needed; don't add BE "support" speculatively |

**Why it happens:**
The 16-command subset was chosen from heavily-ported core commands; the long tail contains code last touched on 32-bit x86, plus 9front-vendored trees whose upstream tested only on arm64-with-9front-kernel (different syscall ABI, different libc).

**How to avoid:**
- Put the grep patterns above into a Wave-0 audit per tier; file findings as a per-dir checklist, not drive-by fixes.
- The execution gates (Pitfall 1) are the real net — these bugs mostly survive compilation.
- Fix classes by pattern across the tree in ONE commit per class (v1.0's atomic-sweep convention), not interleaved with bring-up commits.

**Warning signs:**
A command that runs but emits wrong output (checksums, dates, sizes); `Bgetc` loops that never terminate; crashes only under TCG-but-not-donor or vice versa.

**Phase to address:**
USERSPACE (Wave-0 audit; per-tier execution gates catch escapees).

---

### Pitfall 4: Known latent shared-frontend landmines fire at full-tree scale

**What goes wrong:**
Phase 2 left three acknowledged-unfixed landmines (deferred-items, all "fix when touched next"): (a) `cc/macbody` `gethunk` reset-not-accumulate — any macro expansion needing >NHUNK-max alloc loops forever; (b) the same pattern in `6l/obj.c` `gethunk`; (c) 9cc libmach `hswal` stride bugs on non-`_MAGIC` a.out paths. The full-userspace wave compiles thousands of files with Harvey's shared cc frontend — including very large ones (libhtml, libdraw, upas) — and (a) is a compile-HANG, not an error: the wave wedges mid-build with 100% CPU.

**Why it happens:**
The arm64-donor-kernel link already fired the 7l variant (malloc(3.7MB) → thunk 88TB → ENOMEM, fixed in `7l/unixcompat.c`). `cc/macbody` and `6l/obj.c` were left "untriggered by current workloads" — the full tree IS the trigger.

**How to avoid:**
Fix both `gethunk` sites preemptively in USERSPACE Wave 0 using the proven 7l fix pattern (accumulate-when-contiguous); each fix is small and its own commit with a synthetic large-macro/large-input repro as the gate. Do not wait for the first hung build to diagnose from scratch. The hswal item only matters if a host tool needs non-`_MAGIC` formats — leave it logged, don't fix speculatively.

**Warning signs:**
`mk` wedged with a compiler at 100% CPU; `ps` showing `7c`/`cpp` in a malloc spin; build log stopping mid-file with no error.

**Phase to address:**
USERSPACE Wave 0 (preemptive; it is a known bug with a known fix — cheapest critical pitfall in this document to close).

---

### Pitfall 5: ape scope creep

**What goes wrong:**
"All of `sys/src/cmd`" quietly becomes "all of `sys/src`" and someone drags `sys/src/ape` into a tier. ape is a second libc with its own per-arch syscall stubs, its own arch config, and ~200 ported unix programs never exercised outside 386/amd64 — it is a separate PORT, not a directory. The Phase-2 deferred list already shows ape-specific bit-rot (`ape/lib/{mp,sec}/port/reduce`'s `bind -ac` silently failing in the Linux namespace build, disabling the port-cull).

**Why it happens:**
"Full userspace" reads as "everything users can see"; ape dirs live adjacent to real cmd dirs in tooling mental-models.

**How to avoid:**
Milestone-explicit decision: ape is OUT for v2.0 (the milestone text says `sys/src/cmd` + remaining libraries — hold that line), documented in the phase plan with the rationale (its syscall layer needs its own measured-trace cycle; GUI deliverable doesn't need it). If a requirement genuinely needs an ape program, that requirement names the program and pays for the ape arch layer explicitly.

**Warning signs:**
A tier list containing `ape/`; a build fix touching `sys/src/ape/lib/9/`; "we'll need diff/patch anyway" reasoning (plan9port on the HOST already provides these for build use).

**Phase to address:**
USERSPACE planning gate (the exclusion is written into the phase goal, not implied).

---

### Pitfall 6: virtio feature-negotiation landmines — "ping works but TCP hangs"

**What goes wrong:**
The r9 driver negotiates offload features it doesn't implement (or implements partially): VIRTIO_NET_F_CSUM / HOST_TSO4 / GUEST_CSUM / MRG_RXBUF. Symptoms are the classic verified-in-the-wild split: ICMP echo and small UDP work (small packets, often checksummed anyway), while TCP establishes then stalls or corrupts — segments flagged checksum-offload get dropped by the peer, or >MTU TSO packets arrive that the stack can't reassemble. MRG_RXBUF additionally changes the `virtio_net_hdr` from 10 to 12 bytes — an off-by-2 header parse yields garbage frames that look like "intermittent corruption."

**Why it happens:**
Feature bits are advertised by qemu and the driver must *subset* them; copying a reference driver's feature mask (or accepting the device's full set) imports obligations. Offload bugs are invisible in smoke tests because smoke traffic is small and symmetric.

**How to avoid:**
- Bring-up feature mask: MAC + STATUS only. Split virtqueues, no EVENT_IDX, no CSUM, no TSO/GSO, no MRG_RXBUF, no CTRL_VQ. Software-checksum everything; the guest TCP stack must emit fully-checksummed packets.
- The acceptance gate is a LONG-LIVED TCP flow with large transfers both directions (e.g. 9P drawterm session + a `dd`-sized file pull), not ping. Add the ping-vs-TCP triage to the runbook: ping green + TCP hang ⇒ offload/MTU/PMTU class, not routing.
- Only after the plain path is proven, consider EVENT_IDX as an optimization behind its own gate (notification-suppression races cause lost RX interrupts that look like random stalls).

**Warning signs:**
TCP connects (SYN/SYN-ACK/ACK visible in a host-side capture) then stalls; throughput collapses or corrupts only past ~1 packet of payload; works with `virtio-net-pci` on slirp but not tap or vice versa (slirp is forgiving — Pitfall 9).

**Phase to address:**
NETDRV (the zero-offload mask is the driver's Day-1 config; the long-flow TCP gate is the phase's exit criterion, not ping).

---

### Pitfall 7: Virtqueue memory-ordering races on aarch64

**What goes wrong:**
aarch64 is weakly ordered: descriptor-table writes are not guaranteed visible to the device before the avail-ring index store without an explicit barrier (`dmb`/`dsb` variants per the virtio spec's device/driver ordering rules). Missing or mis-scoped barriers pass every smoke test (TCG single-vCPU timing is gentle) and corrupt descriptors under load — the rio-over-drawterm session is exactly the sustained bidirectional load that exposes it.

**Why it happens:**
x86 reference code "works without" visible barriers; the bug is timing-dependent, so it survives bring-up and detonates during the DESKTOP phase where it's misdiagnosed as a 9P/rio bug.

**How to avoid:**
- Write the ring code barrier-first: descriptor fill → store-store barrier → avail idx → (notification check) → kick; used-side read barrier before trusting descriptor contents. Treat the virtio 1.x spec's ordering section as the contract, not x86 folklore.
- Add a load gate to NETDRV's exit: parallel large transfers + concurrent small RPCs for N minutes, with descriptor-integrity counters (seq numbers in a debug build) rather than eyeballing.

**Warning signs:**
Corruption that moves when logging is added (heisenbug); failures only above a packet rate; works on `-smp 1` TCG, fails on multi-vCPU or HVF.

**Phase to address:**
NETDRV (barrier audit is part of driver review; the load gate is phase exit).

---

### Pitfall 8: GICv2 virtio interrupt plumbing — INTx, level trigger, ISR ack, and the MSI mirage

**What goes wrong:**
Four sub-failures, all verified against primary sources:
1. **Hardcoded IRQ numbers.** PCI legacy INTx on qemu virt are level-sensitive SPIs whose IDs are *implementation-defined* and published only in the DTB pci node's `interrupt-map` (with the slot/pin swizzle). Hardcoding the number from one qemu's dumpdtb breaks on the next qemu or the next slot.
2. **Wrong trigger type.** Programming the SPI edge-triggered (copying the PL011/timer pattern) loses or sticks interrupts — PCI legacy IRQs are LEVEL.
3. **Missing ISR read-to-ack.** virtio's ISR register byte must be read to deassert the INTx line; skipping it wedges the line asserted (interrupt storm) or never-reasserts (silent RX stall).
4. **The MSI mirage.** GICv2 has NO native PCI MSI support (ARM BSA: v2 needs the GICv2m extension frame; qemu virt+gicv2 does expose a v2m node, but it's a separate driver path). A driver written MSI-first against a GICv3 mental model has no interrupts at all on the milestone's gic-version=2 machine.

**Why it happens:**
r9's existing devices (PL011, timer) are DTB-adjacent and simple; PCI INTx routing via `interrupt-map` is the first nontrivial interrupt plumbing, and most online virtio examples are MSI/MSI-X on x86 or GICv3+ITS.

**How to avoid:**
- INTx-first, DTB-driven: parse `interrupt-map` from the committed DTB (v1.0 precedent: committed `virt-gicv2.dts` is the contract), program the SPI level-triggered, read the virtio ISR byte in the handler's first instruction.
- Keep MSI/v2m as a documented non-goal for this milestone — INTx at qemu virtio rates is not the bottleneck.
- Gate: RX-under-load interrupt accounting (interrupts ≈ packets within a factor, no storm, no stall) in the NETDRV exit suite.

**Warning signs:**
`info irq`-style counters flatlining after the first packet; handler entered exactly once; storm (handler re-entered with no work); works when the device lands on a different PCI slot (slot-dependent IRQ).

**Phase to address:**
NETDRV (Wave-0: the DTB `interrupt-map` parser + level-trigger programming is a spike before the virtqueue code, exactly the v1.0 tracer-first shape).

---

### Pitfall 9: slirp masks L2 bugs — mode confusion between user-mode and tap networking

**What goes wrong:**
Bring-up happens on qemu slirp (`-netdev user`) because it's zero-privilege and hermetic. slirp's built-in ARP/DHCP forgive guest L2 mistakes — it answers ARP for the gateway itself, accepts marginal frames, NATs everything. The driver/stack then "works" until the tap/bridge leg, where real L2 exposes: ARP request/reply handling bugs, broadcast handling, MAC filtering, MTU edge cases. The inverse confusion also bites: ndb/drawterm args hardcoded to slirp's 10.0.2.x (guest .15, host .2) silently wrong on the bridge subnet.

**Why it happens:**
slirp is the path of least resistance and the milestone legitimately wants it as the darwin default (Pitfall 10) — the pitfall is treating slirp-green as networking-done.

**How to avoid:**
- Declare tap/bridge on linux CI the **L2-truth leg** (extends the v1.0 "CI green = linux leg" convention): the NETDRV exit gate runs on tap, even though darwin dev iterates on slirp+hostfwd.
- Never hardcode slirp addresses: the guest's `/lib/ndb/local` entry and the drawterm `-h/-a` args are parameterized per mode at bake/launch time; the flake app prints the actual guest IP + connect line.
- Bring-up order inside the guest follows Plan 9's: devether → devip (`/net`) → ipconfig (DHCP on `/net/ether0`) → cs → dns, each with its own marker in the gate log.

**Warning signs:**
"Networking done" claimed with only slirp evidence; ARP table never populated on the host side of a tap; drawterm args containing 10.0.2.x in a tap-mode run.

**Phase to address:**
HOSTNET owns the mode matrix (slirp default on darwin; tap default on linux; CI exercises tap). NETDRV's exit gate must include the tap leg.

---

### Pitfall 10: darwin vmnet entitlement wall + privileges can't be hermetic

**What goes wrong:**
On macOS, qemu's `vmnet-*` backends require the `com.apple.vm.networking` entitlement; ad-hoc-signed binaries don't get it (qemu's own issue tracker shows the unsigned binary dying `killed: 137`, and the entitled failure mode "general failure (possibly not enough privileges)" even under sudo). **nixpkgs qemu is ad-hoc signed** — so a flake that adds `-netdev vmnet-bridged` to its nix-provided qemu fails on every stock macOS machine, and no amount of sudo fixes it. Separately: privileged setup (tap creation, bridge membership, socket_vmnet daemon, entitlement signing) can never live inside a nix derivation — the store is read-only, setuid is impossible from store paths, and qemu's own `-netdev bridge` helper is a setuid binary that nix cannot ship.

**Why it happens:**
"One command boots the full system" pressure pushes setup into the flake app; the entitlement/sandbox walls are invisible until the first fresh-mac run (v1.0's lima `/bin/rc` lesson: environment conveniences not codified in scripts get discovered by the fresh host, at CI cost).

**How to avoid:**
- darwin default: **slirp + hostfwd** (`hostfwd=tcp:127.0.0.1:17010-:17010`) — hermetic, zero-privilege, and sufficient for the drawterm deliverable (host connects to localhost). The real-L2 deliverable on darwin is a documented opt-in (socket_vmnet — likely already present via the project's lima install — or manually signed qemu with an Apple dev cert), scripted as a **doctor-style checker** in the flake app that verifies and prints the exact sudo/signing lines, never performs them silently.
- linux default: tap via documented `ip tuntap`/bridge commands, same doctor pattern; the app checks `/dev/net/tun` access, tap existence, bridge membership, and nftables/firewalld FORWARD policy, and prints the fix lines.
- The flake app takes `--net=slirp|tap|vmnet` explicitly; there is no "auto" mode that silently downgrades.

**Warning signs:**
vmnet "general failure" on a stock mac; CI green but a contributor's linux laptop blackholes the guest (their nftables default-drops the bridge); a setup step that "worked" because someone ran it by hand last month (lima-rc class).

**Phase to address:**
HOSTNET + FLAKE (the doctor-checker is the deliverable; hermetic-by-default, privileged-by-documentation).

---

### Pitfall 11: Auth first-boot chicken-and-egg — no NVRAM on r9

**What goes wrong:**
Classic p9sk1 bootstrap assumes: `auth/wrkey` stores the machine key + hostowner + hostdomain in NVRAM; `keyfs` decrypts `/adm/keys` with that NVRAM key; `authsrv` answers on tcp/567. r9 has no NVRAM device and no console-attached secure storage. Consequences cascade: keyfs with no machine key fails or prompts at boot (non-interactive gate wedges); a baked `/adm/keys` encrypted under a different key than the baked "nvram" serves undecryptable garbage and EVERY auth fails identically; `changeuser` for the first user needs hostowner-on-the-auth-server, which needs... an authenticated session. And `convkeys` with keyfs running corrupts the DB (man-page BUGS, verified).

**Why it happens:**
Physical Plan 9 auth servers were brought up interactively by an operator at a console; the reproducible-from-clean-checkout constraint deletes the operator.

**How to avoid:**
- Bake-time authority: ONE host-side tool (extend the std-only `build/harness/` binary — v1.0's harness decision) generates the demo domain as a single unit: machine key, `/adm/keys` encrypted under it, hostowner, authdom, ndb entries, plus the nvram-substitute file — hash-paired so keyfs can verify the pairing at start and fail loudly instead of serving garbage.
- The milestone ships a DEMO auth domain (documented: reproducible credentials, not a security boundary — the hostowner password is in the repo by design for the demo, with the real `wrkey`/`changeuser` operator flow documented for anyone deploying for real).
- r9 device surface: the nvram-substitute is a file in bootfs surfaced read-only to auth tools (devroot node), NOT a new syscall/device family.
- Gate: keyfs starts non-interactively in the boot-test; `auth/debug` (verified to exist — it tries each p9sk1 key in factotum and prints progress) runs green inside the guest BEFORE drawterm is ever attempted.

**Warning signs:**
keyfs boot prompt in the serial log; "authentication failed" for every user including hostowner; `/adm/keys` mtime newer than the nvram-substitute (pair drift).

**Phase to address:**
AUTH Wave 0 (the bake-time keydb generator is the phase's first deliverable; nothing else in AUTH can be tested without it).

---

### Pitfall 12: p9sk1 "authentication failed" opacity — the authdom/hostowner/speaksfor triangle

**What goes wrong:**
p9sk1's client-visible failure is a single string. The real causes live server-side and are enumerable: (a) **authdom mismatch** — the `authdom=` in the guest's ndb, the `dom=` attribute on factotum keys, and drawterm's `-a`/authdom notion must all agree; (b) **server-side key missing** — the cpu server's factotum must hold the *server's* (hostowner/machine) key for the domain, not just the user's — ticket decryption fails otherwise; on a loopback auth+cpu machine one factotum holds both roles' keys and it's easy to load only one; (c) **speaksfor missing** — `/lib/ndb/auth` must grant the hostowner speaks-for the users, else tickets validate but uid mapping is refused; (d) **name resolution** — `auth=` naming a DNS name with no working dns/cs yet → "can't reach auth server" (use numeric IPs in `/lib/ndb/local` for the demo domain).

**Why it happens:**
Four independent configs must agree; each fails with the same client string; operators debug by guessing. Also note a **red herring to retire early**: clock skew is NOT a p9sk1 failure mode — the tickets carry no timestamps. Skew only bites if someone reaches for 9front drawterm's TLS rcpu path (Pitfall 13), where cert validity windows apply (and the guest RTC is unset under r9). Don't let "check the clock" consume a debugging cycle on the p9sk1 path.

**How to avoid:**
- Serial-visible auth logging from Day 1: authsrv/keyfs log to the serial console (or `/sys/log` readable over serial) — the out-of-band channel that survives the session being debugged.
- The AUTH phase gate includes a guest-side loopback auth probe (`auth/debug` + a local p9sk1 handshake to 127.0.0.1:567) BEFORE any host drawterm attempt — loopback-green is the entry criterion for DESKTOP.
- Bake the four configs from the same generator (Pitfall 11's tool) so they agree by construction; the gate asserts the agreement (authdom string equality across ndb/keydb/factotum-seed).

**Warning signs:**
"authentication failed" with nothing on the serial auth log (logging not wired — fix that first); works for hostowner but not the demo user (speaksfor class); works by IP but not name (cs/dns class).

**Phase to address:**
AUTH (loopback gate + serial logging are phase-exit criteria; DESKTOP may not start without them).

---

### Pitfall 13: 9front drawterm defaults to rcpu/TLS/17019 + dp9ik — protocol mismatch with Harvey's p9sk1/17010

**What goes wrong:**
Verified from the 9front-drawterm man page: it defaults to the TLS-based `rcpu(1)` protocol and dp9ik authentication; classic `cpu(1)`/p9sk1 (what Harvey speaks, on tcp/17010) requires the `-O` flag. A flake that packages 9front drawterm and invokes it bare gets a TLS handshake against Harvey's non-TLS 17010 listener — hang/reset with no useful error on either side. Additional traps from the same man page: `$cpu`/`$auth`/`$USER`/`$PASS` environment variables silently override flags and suppress the password prompt — a 9front-using developer's shell env hijacks the demo; the `-e` crypt/hash default (`rc4_256 sha1`) applies only to old cpu and must match what Harvey's cpu service offers (`clear` is the bring-up-safest).

**Why it happens:**
"drawterm is drawterm" — the fork drift (9fans/rsc original = p9sk1-native; 9front = dp9ik/rcpu-first) is invisible until interop fails. Both forks' devdraw serving is compatible for rio's purposes; the AUTH and transport layers are what diverged.

**How to avoid:**
- Decide the drawterm provenance as a FLAKE decision with the vendor-and-own discipline (Phase-2 D-10 precedent): either 9fans/rsc drawterm (protocol-fidelity with 4e lineage, no `-O` needed) or 9front drawterm with a wrapper that ALWAYS passes `-O -e clear` (or the negotiated crypt) and scrubs `$cpu`/`$auth`/`$PASS` from the environment before exec.
- The wrapper, not the user's memory, owns: `-h <guest ip> -a <guest ip> -u <user>` printed from the same launch pipeline that baked ndb (Pitfall 9's parameterization).
- Interop gate: DESKTOP's exit criterion includes a scripted drawterm run (text mode `-G` first for CI — headless, no cocoa/X needed; graphical rio session human-accepted on darwin/linux legs, same as v1.0's TCG banner acceptance).

**Warning signs:**
drawterm connects and instantly hangs pre-prompt; guest serial shows a TCP connection with garbage first bytes (TLS ClientHello hitting the p9sk1 listener); auth works in `-G` text mode but the user's interactive run fails (env-var class).

**Phase to address:**
FLAKE (provenance + wrapper) gates DESKTOP (interop proof); the decision lands in the FLAKE phase plan, not at the drawterm invocation site.

---

### Pitfall 14: rio started in the wrong script + fonts/draw prerequisites

**What goes wrong:**
rio needs `/dev/draw`, which exists ONLY inside the drawterm cpu session (drawterm serves `#i`/`#m`/`#b` over 9P; r9 deliberately has no framebuffer/GPU driver). Starting rio in `cpurc` runs it once at guest boot where `/dev/draw` doesn't exist — rio exits or wedges, and every drawterm session inherits a rio-less desktop. The correct site is the connecting user's `lib/profile` (drawterm runs it with `$service=cpu`) or an equivalent per-session stanza. Second: rio needs a font — `$font` or `/lib/font/...` from the GUEST filesystem; Harvey's bootfs never needed fonts before (fonts live in the dist/ISO layer that v1.0 scoped out), so a bootfs without `/lib/font` yields `rio: can't open font` or a blank screen. Third: drawterm `-B` disables kbdfs → dead keyboard in rio; wrapper must not pass it.

**Why it happens:**
cpurc-vs-termrc-vs-profile semantics are the classic Plan 9 newcomer trap; the r9 design (no graphics driver, draw arrives over 9P) makes the ordering constraint absolute rather than convention.

**How to avoid:**
- Ship a known-good `lib/profile` + `cpurc` pair in bootfs (based on `cfg/example/` templates), with the rio start gated on `$service` — and a boot-test marker that proves rio did NOT start at guest boot.
- Extend mkimage's manifest assertion (the v1.0 staged-pollution guard) to cover `/lib/font/**`, `/lib/ndb/local`, `/lib/namespace`, the profile/cpurc pair, and the baked auth DB — one manifest, every baked file accounted for.
- DESKTOP gate sequence: `-G` text-mode drawterm → rc prompt over cpu (proves auth+cpu without the draw stack) → graphical session → rio banner/rect assertion via devdraw interaction or human acceptance.

**Warning signs:**
guest serial shows rio starting at boot; drawterm window opens blank/black; rio runs but no keyboard (`-B` class) or no font error scrolled past on serial.

**Phase to address:**
DESKTOP (profile/cpurc/fonts are deliverables with manifest assertions); USERSPACE must have shipped rio + libdraw family + fonts into the image first (ordering dependency for the roadmap).

---

### Pitfall 15: Debugging the TCP stack through the session that rides it

**What goes wrong:**
Once the workflow is drawterm-over-TCP, a TCP/driver bug wedges the very channel used to observe it — no console, no shell, no logs. This is how driver bugs get misattributed to rio/9P/drawterm and burn days.

**Why it happens:**
The serial console stops being exercised once the desktop lands; qemu packet capture isn't wired by default; the smoke gate asserts end-state, not the failure forensic trail.

**How to avoid:**
- PL011 serial console remains a first-class output forever: auth log, ip stack debug (behind flag variables per repo convention), and the panic path all keep writing to it; the boot-test harness keeps asserting on serial markers even in the desktop era.
- Wire host-side packet capture into the harness from NETDRV day one: qemu `-object filter-dump` (or tap-side `tcpdump` in the doctor script) with the capture path printed per run.
- The triage runbook (committed doc): ping-green/TCP-hang ⇒ offload/MTU (Pitfall 6); ARP-empty ⇒ L2/tap (Pitfall 9); TCP-hang-mid-session ⇒ virtqueue race class (Pitfall 7); connect-refused ⇒ listener/namespace; auth-failed ⇒ Pitfall 12.

**Warning signs:**
Any debugging session that begins with "restart it and watch drawterm"; missing serial lines that used to appear (console regressed); no capture artifact next to a networking bug report.

**Phase to address:**
NETDRV (capture wiring + serial discipline) and DESKTOP (runbook lands with the session gate).

---

### Pitfall 16: Cross-repo drift — qemu flags, pin-vs-HEAD, and staged-bake pollution (v2.0 amplification)

**What goes wrong:**
Three v1.0-proven classes amplify with networking's new surface:
1. **qemu flag duplication across repos.** r9's xtask `[qemu]` TOML owns its boot-test launches; Harvey's flake apps own composed boots. virtio-net adds netdev/device stanzas BOTH need (driver gate in r9; composed session in Harvey). Drift example: r9 tests against `virtio-net-pci-non-transitional` while Harvey composes the transitional default (the v1.0 donor finding: transitional ID 0x1000 vs modern 0x1041 match failure) — driver green in r9, dead in compose.
2. **Pin-vs-HEAD.** v2.0 adds r9 features continuously (driver, /net, devices); the carried-over rule stands: pin bump = its own commit + fresh hosted CI leg, never bundled with the Harvey-side feature that needs it (bisectability + the push-before-pin ordering).
3. **Staged-supply-chain pollution.** v1.0 caught a staged init baked into the image via the mkimage manifest guard; v2.0's bootfs gains keys/ndb/fonts/profiles — each new baked file is a new silent-pollution vector, and `out/` ownership races widen as more stages write it (lima rsync-back + native legs + bake steps).

**Why it happens:**
Two repos, two qemu launch paths, one image supply chain — each individually disciplined in v1.0, but the v2.0 feature set touches all three at once.

**How to avoid:**
- Single source for the netdev/machine stanza: r9's platform TOML carries it as data (the v1.0 "platforms as data" decision extends naturally); Harvey's flake app consumes the same fragment rather than restating flags. Any flag change lands r9-first with its gate, Harvey records the SHA (D-13 ordering unchanged).
- Pin-bump discipline as a named rule in every NETDRV/AUTH/DESKTOP plan that crosses the boundary: r9 commits → r9 gates → push → Harvey pin-bump commit → hosted CI → Harvey feature commit that relies on it.
- Extend mkimage's manifest assertion to the FULL bake set (Pitfall 14); one-writer rule for `out/` per stage with the harness asserting freshness (mtimes vs inputs) rather than assuming.

**Warning signs:**
A driver behavior difference between `cargo xtask boot-test` and `nix run .#boot-r9`; a Harvey commit whose message mentions both "bump r9" and a feature; a bootfs whose ndb/keys differ from the generator's output without a regeneration commit.

**Phase to address:**
All — this is the milestone's process pitfall; the roadmap should put the netdev-flag single-sourcing in NETDRV's first plan and the manifest extension in the earliest USERSPACE/AUTH plan that adds a baked file.

---

## Moderate Pitfalls

### Pitfall M1: Half-kernel/half-userspace IP stack confusion

**What goes wrong:** The FEATURES/architecture research decides where TCP lives (r9 kernel devip-style vs a no_std userspace/stack crate). Either way, the contract is the `/net` file ABI — clone files, per-conversation `ctl`/`data`/`status`, announce/listen via writes — because ipconfig/cs/dns AND the cpu listener all speak it. The pitfall: implementing "a TCP path" that bypasses the ABI (fast-path ioctl-ish shortcuts) and discovering ipconfig/dns/17010 can't ride it. Chicken-egg sub-trap: DHCP needs raw ethernet before IP is configured — the raw-frame channel must exist before the IP path, in whichever layer owns it.
**Prevention:** the NETDRV plan's first milestone is `/net/ether0` + clone/ctl semantics with ipconfig running unmodified; TCP data plane second. Note the leverage: Plan 9 networking needs **zero new syscalls** — announce/listen/dial are file ops — so any plan that grows the syscall table for networking is a red flag (the measured-set rule generalizes to devices).

### Pitfall M2: Linux tap/bridge distro drift + firewall eating the guest

**What goes wrong:** `ip tuntap` + bridge setup differs across NetworkManager/systemd-networkd/netplan hosts; nftables default-drop or firewalld zones silently drop host↔guest frames on the bridge (ARP works — L2 — but TCP SYNs vanish, looking like a guest TCP bug). qemu-bridge-helper is setuid and cannot come from the nix store.
**Prevention:** the doctor-checker (Pitfall 10) asserts: tap exists, bridge membership, FORWARD policy, `/dev/net/tun` perms — and prints exact remediation per detected distro tooling; the flake app uses explicit `-netdev tap,ifname=...,script=no,downscript=no` against the pre-made tap, never `-netdev bridge`.

### Pitfall M3: MTU/PMTU mismatches

**What goes wrong:** 1500-vs-overlay-underlay mismatches on bridged setups blackhole large packets: small pings and interactive echo fine, bulk transfer stalls — same symptom family as Pitfall 6, different cause.
**Prevention:** the triage runbook's second step after offload: large-ping sweep (`ping -s` equivalents) across the path; keep guest MTU at 1500 to match qemu defaults; document the symptom split.

### Pitfall M4: drawterm devdraw/9P semantics under load

**What goes wrong:** full-screen refreshes are bulk transfers over the cpu 9P session; if r9's devmnt caps `msize` small, rio is glacial-but-correct (perf, not correctness); long-lived idle connections probe TCP keepalive/RST corners in a freshly-written stack (abortive-close handling).
**Prevention:** negotiate a real msize (≥64 KiB) in the cpu session; the NETDRV long-flow gate (Pitfall 6) includes a multi-hour idle-then-active cycle to exercise RST/keepalive; perf expectations documented (local VM latency, not LAN).

### Pitfall M5: libthread-era programs exercise unmeasured kernel corners

**What goes wrong:** rio/acme/sam are libthread programs; the v1.0 measured set covered the shell path only. GUI programs are the first likely callers of corners like RENDEZ(2) and richer note interaction; assuming "shell runs ⇒ GUI runs" ships a rio that walls on an unimplemented loud-None mid-demo.
**Prevention:** the measure-first trace on the donor kernel (Pitfall 1) explicitly includes the Tier-3 GUI binaries' boot-to-window syscall footprint BEFORE the DESKTOP phase commits to r9 support; r9's loud-unimplemented default stays loud (v1.0 rule) so gaps name themselves in the serial log.

### Pitfall M6: Games/excluded-dirs re-entry pressure

**What goes wrong:** late-milestone "just one more dir" commits pull `games/`, `gs`, or ape programs into the sweep without their gates, re-breaking the default build.
**Prevention:** the allowlist (Pitfall 1) is append-only-via-gate; exclusions are named in the phase goal; a completion audit diffs the allowlist against the phase plan.

## Minor Pitfalls

| Pitfall | What goes wrong | Prevention |
|---------|-----------------|------------|
| drawterm screen geometry | `-g` mismatch → default-size rio window; cosmetic confusion | wrapper passes a deliberate `-g`; document |
| drawterm audio (`#A`) | host audio probing spews warnings on headless CI | use `-G` text mode in CI; audio never gated |
| `$PASS`/`$cpu`/`$auth` host env leaks | drawterm skips prompts / dials the wrong server | wrapper scrubs env (Pitfall 13) |
| kbdfs disabled (`-B`) | dead keyboard in rio | never pass `-B` in the wrapper |
| guest time unset (no RTC read / no ntp) | log timestamps nonsense; fossil-less so mostly cosmetic; TLS only if Pitfall 13 path taken | set `#c/time` from host at boot (bake or boot-line); don't chase "clock skew" on p9sk1 failures (Pitfall 12 red herring) |
| keyfs DB admin after bake | `convkeys` with keyfs running corrupts the DB (man-page BUGS) | runbook: admin ops only with keyfs down; prefer regenerating via the bake tool |

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Negotiate virtio offloads "for performance" before the plain path is proven | Faster transfers on paper | Pitfall 6's phantom TCP bugs baked into the driver's foundation | **Never** — zero-offload is the milestone config |
| Hardcode ECAM base / virtio IRQ SPIs from dumpdtb | Skip DTB parsing work | Breaks on qemu bump (11.1.2 already moved ECAM once — v1.0 evidence) | Never; the committed DTB is the contract |
| Bake demo auth keys into the repo | Reproducible demo auth | A real deployment reuses demo credentials | Acceptable ONLY as a loudly-documented demo domain; the operator flow (wrkey/changeuser) must exist in docs |
| slirp-only networking evidence | Zero-privilege CI simplicity | L2 bugs ship to the tap leg undetected | darwin iteration yes; milestone exit NO (tap leg required) |
| Defer the `gethunk` ×2 fixes until they fire | One fewer Wave-0 task | A hung build mid-wave, diagnosed from scratch | Never — known bug, known fix, certain trigger |
| Skip per-tier donor-kernel execution, gate on r9 only | Fewer harnesses to maintain | 7c-miscompile vs r9-gap confounded in every failure | Never — the donor harness exists and is the known-good reference |
| Grow r9 syscall table by reading man pages | Skip a trace cycle | Handlers without callers; ABI drift (v1.0's measured-set rule exists because of this) | Never — measure-first on the donor chain |
| Text-mode-only drawterm gate (`-G`) for CI | Headless CI works | rio/devdraw path unproven at ship | Acceptable for CI; milestone exit needs a human-accepted graphical leg on both host OSes (v1.0 TCG-banner precedent) |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| 9front drawterm | Bare invocation → rcpu/TLS/17019/dp9ik against p9sk1/17010 | Wrapper pins `-O` (+ negotiated `-e`), scrubs `$cpu`/`$auth`/`$PASS`, OR choose 9fans drawterm (FLAKE decision) |
| qemu on darwin | `-netdev vmnet-*` on nixpkgs (ad-hoc-signed) qemu | slirp+hostfwd default; vmnet/socket_vmnet documented opt-in with doctor-check |
| qemu on linux | `-netdev bridge` (setuid helper can't come from nix store) | pre-made tap + `-netdev tap,ifname=X,script=no` + doctor-asserted bridge/nftables |
| Guest ndb | Hardcoded 10.0.2.x (slirp) baked into bootfs | ndb entry generated per net-mode at bake/launch; numeric IPs, not DNS names, for the demo domain |
| p9sk1 auth server | Name-resolved `auth=` with no dns/cs yet | numeric `auth=` in `/lib/ndb/local`; loopback guest probe before any host client |
| cpu service | Assuming announce/listen need new r9 syscalls | They're file ops on `/net/tcp` — zero new syscalls; the listener is `aux/listen` + `/rc/bin/service/tcp17010` |
| r9 device growth | Adding virtio-gpu/devdraw "while we're here" | r9 needs NO graphics driver this milestone — draw arrives via drawterm's `#i`; anti-feature, guard the scope |
| Harvey↔r9 qemu flags | Netdev stanzas restated in both repos' launchers | r9 TOML carries the stanza as data; Harvey's app consumes it (Pitfall 16) |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| TCG-only networking legs | Tap/auth/desktop gates take minutes; timeouts misread as hangs | Keep TCG leg timeouts generous; HVF/KVM legs where available (darwin HVF rejects GICv2 on qemu 11.1.2 — v1.0 finding: accel chain ends at tcg, so darwin legs are TCG by design) | Immediately on hosted CI (x86_64-linux has no KVM in the dev fleet — v1.0 accepted TCG for gates) |
| Small `msize` on the cpu 9P session | rio refreshes crawl (5 MB screen ÷ 8 KiB messages) | Negotiate ≥64 KiB; note devmnt's mntchk bounds discipline scales with msize | First full-screen rio redraw |
| INTx interrupt coalescing absent | One IRQ per packet at high pps | Acceptable at demo scale; EVENT_IDX/MSI are post-milestone optimizations | Sustained bulk transfer benchmarks, not the deliverable |
| Full-cmd sweep build time (dual-arch, lima rsync) | Default `nix run .#build` slows for every workstream | Tier allowlist keeps the sweep proportional; per-tier dirs enter only when gated | As tiers land — monitor; v1.0's dual-arch default was the right call, keep it |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Demo auth domain mistaken for a security boundary | Anyone on the bridge owns the guest | Docs + boot banner label it DEMO; operator key flow documented; no real secrets ever baked |
| keyfs/nvram pairing unverifiable | Garbage keydb serves uniform "authentication failed" (Pitfall 11) | Bake-time pairing hash asserted at keyfs start — fail loud, not opaque |
| `hostfwd` bound to 0.0.0.0 on slirp | Guest's cpu/auth ports exposed to the LAN | Bind hostfwd to 127.0.0.1 in the flake app |
| TLS-era libsec changes sneaking in with auth work | Re-opening the v1.0-declared TLS debt (out of scope) | Milestone rule stands: zero libsec/devtls modernization; p9sk1's DES/RC4 surface unchanged; audit at ship (v1.0 Pitfall 13 carries forward) |
| drawterm `-e clear` left as the shipped default | Session traffic plaintext on the bridge | Bring-up default clear, ship default negotiated crypt (`rc4_256 sha1` matches Harvey's libsec); wrapper owns the flag, docs explain |

## "Looks Done But Isn't" Checklist

- [ ] **"cmd builds":** mk exit status is ALWAYS 0 in the namespace build (Phase-2 deferred item) — the gate is artifact existence + per-tier donor-kernel execution markers, never rc.
- [ ] **"libdraw family ported":** portable C compiles ≠ mkfiles correctly skip per-arch asm for aarch64 — verify the mkfile guards, then a binary that actually links libmemdraw.
- [ ] **"virtio-net works":** ping green ≠ TCP works — the exit gate is a sustained bidirectional TCP flow (Pitfall 6) on the TAP leg (Pitfall 9), with interrupt accounting (Pitfall 8) and a load soak (Pitfall 7).
- [ ] **"auth works":** drawterm login green ≠ the domain is sound — verify the loopback guest probe, keyfs pairing assertion, and speaksfor for a NON-hostowner demo user.
- [ ] **"drawterm connects":** text-mode `-G` green ≠ desktop — rio running under the session's `/dev/draw`, keyboard live (kbdfs not `-B`'d), fonts resolved from the guest, rio NOT started at guest boot (serial marker proves absence).
- [ ] **"networking reproducible":** works on the dev mac ≠ works from clean checkout — the doctor-checker must pass on a fresh linux host (CI leg) and the darwin slirp path on a fresh mac (documented-manual leg, v1.0 D-13 convention).
- [ ] **"flake supplies drawterm":** `nix run .#drawterm` exists ≠ correct invocation — the wrapper's flags/env-scrub are the deliverable; proven by the scripted interop gate, not by launching it once by hand.
- [ ] **"bootfs baked":** image builds ≠ contents right — the mkimage manifest asserts every new baked file (keys/ndb/fonts/profiles), extending the v1.0 staged-pollution guard.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| 1 (whack-a-mole) | MEDIUM | Stop flag-flip; re-baseline to the tier allowlist; triage failing dirs by bug class (Pitfall 3 table) rather than by build order |
| 2 (generators) | LOW | Per-dir fix: generator → host toolchain; the Wave-0 audit lists all sites up front so recovery is never needed twice |
| 4 (gethunk hang) | LOW | Kill the wedged compiler; apply the known 7l-pattern fix; synthetic repro as the gate |
| 6/7 (virtio negotiation/races) | MEDIUM | Drop to zero-offload mask + barrier audit; reproduce under the load gate, not interactively |
| 8 (IRQ plumbing) | MEDIUM | QMP/gdbstub inspect GIC registers + DTB interrupt-map re-read (v1.0's GICR_WAKER forensic recipe is the template); never guess SPI numbers |
| 11/12 (auth bootstrap/opacity) | MEDIUM | Regenerate the whole domain with the bake tool (single source); add serial auth logging first if missing — do not debug p9sk1 blind |
| 13 (drawterm protocol) | LOW | Add `-O`/scrub env or swap provenance; interop gate proves it in CI text mode |
| 14 (rio ordering/fonts) | LOW | Move rio to profile; add fonts + manifest; no code changes |
| 10 (vmnet wall) | LOW | Fall back to slirp+hostfwd (the default); pursue entitlement/socket_vmnet as the documented opt-in |
| 16 (cross-repo drift) | LOW-MEDIUM | Realign flag single-source; split any bundled pin+feature commit into two (bisectable) |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase Area | Verification |
|---------|----------------------|--------------|
| 1 whack-a-mole / 5 ape / 6-dir notoriousness | USERSPACE (tier design) | Tier allowlist append-only; per-tier donor+r9 execution markers; exit = all tiers green, ape/gs excluded by name |
| 2 cross generators | USERSPACE Wave 0 | Audit list committed; CI red on target-magic generator artifacts |
| 3 bug-class catalog | USERSPACE Wave 0 + every tier | Grep-audit findings filed per dir; class-fixes land as single atomic commits |
| 4 gethunk landmines | USERSPACE Wave 0 | Both sites fixed preemptively with synthetic large-input repros |
| 6 negotiation / 7 barriers | NETDRV | Zero-offload mask in driver config; long-flow TCP gate + load soak at phase exit |
| 8 IRQ plumbing / ECAM | NETDRV Wave 0 | DTB-driven interrupt-map parser spike before virtqueues; level-trigger + ISR-ack in handler; interrupt accounting green |
| 9 slirp masks L2 | HOSTNET | Tap leg is the L2-truth exit gate (linux CI); slirp is darwin-default dev loop only |
| 10 vmnet/hermetic privileges | HOSTNET + FLAKE | Doctor-checker output on fresh hosts; no setuid/entitlement assumption in any derivation |
| 11 auth bootstrap | AUTH Wave 0 | Bake-time domain generator; keyfs non-interactive start + pairing assertion in boot-test |
| 12 auth opacity | AUTH | Serial auth logging; loopback p9sk1 probe green pre-drawterm; non-hostowner speaksfor gate |
| 13 drawterm protocol | FLAKE | Wrapper flags/env-scrub; scripted `-G` interop gate in CI |
| 14 rio ordering/fonts | DESKTOP | Serial marker proves rio absent at boot; profile-driven rio; font manifest assertion; graphical leg human-accepted per host OS |
| 15 debug-through-TCP | NETDRV + DESKTOP | Serial discipline maintained; filter-dump capture path printed per run; triage runbook committed |
| 16 cross-repo drift | all (process rule) | netdev stanza single-sourced; pin-bump commits isolated + CI'd; mkimage manifest covers full bake set |

**Research flags for phase planners:**
- NETDRV likely needs its own phase-level RESEARCH: where TCP lives (the "resolved in research" kernel-vs-userspace decision) must be FINAL before the driver plan, because it determines whether Pitfall M1's ABI work lands kernel-side or user-side.
- USERSPACE needs no new domain research (the bug classes and landmines are enumerated here); it needs the Wave-0 audits scheduled as real tasks.
- AUTH/DESKTOP are well-covered by 4e-era docs + this file; the open question is which drawterm provenance (FLAKE decision) — everything else follows from it.
- HOSTNET's darwin L2 opt-in (socket_vmnet vs signed-qemu-vmnet) deserves a small decision spike inside its phase, not a research phase.

## Cautionary Tales (from this repo's v1.0, applied forward)

1. **The pswt.c lesson, generalized:** a compiler bug class thought "mostly clean" (7c is 9front-vendored) was only proven on 16 commands. The full tree is a different distribution of inputs — Wave-0 fixes the KNOWN live landmines (gethunk ×2) before the distribution shifts.
2. **The staged-init wedge (05-07):** a stale artifact baked into bootfs wedged the gate and cost a debugging cycle; v2.0 bakes secrets and configs — the manifest guard grows or the same class returns with worse symptoms (wrong key = indistinguishable from auth bug).
3. **The lima `/bin/rc` masking (G-05-1):** an undocumented environment convenience masked a gap until the first fresh host. v2.0's equivalent: any host networking step performed by hand and not codified in the doctor-checker will be discovered by the first fresh linux CI host or contributor mac.
4. **Transitional-vs-modern virtio (donor boot, 02-04):** qemu's default `virtio-net-pci` is transitional (0x1000); the donor driver matched only modern (0x1041). Pin the device variant in the TOML/flags BEFORE writing r9's driver — the two variants have different register layouts, and porting between them wastes a week.
5. **qemu 11.1.2 ECAM move (02-04):** the virt ECAM base moved once already (0x3F000000 → 0x40100000). Anything hardcoding PCI addresses instead of reading the committed DTB re-fights this on the next qemu bump.

## Sources

- **In-repo v1.0 artifacts (HIGH):** `.planning/milestones/v1.0-phases/02-*/deferred-items.md` (gethunk ×2 unfixed sites, hswal, ECAM move, transitional ID, GICR WAKER, pswt.c lineage); `05-*/deferred-items.md` (staged-bake pollution, fid/mntchk discipline); `.planning/RETROSPECTIVE.md` (boot-test rule, measure-first rule, fresh-host lesson); `PROJECT.md` (known upstream breakage list; carried-over pin-bump rule; 9k/ape context); `.planning/research/PITFALLS.md` v1.0 (Pitfalls 1, 13, 14 carried forward).
- **Primary docs fetched (HIGH for the claims cited):** drawterm(1) 9front man page via Debian manpages (rcpu/TLS/dp9ik default, `-O`, `-G`/`-B`, `$cpu`/`$auth`/`$USER`/`$PASS` env, `-e` crypt scope, rsc-original p9sk1 lineage); Plan 9 auth(8) man page via 9p.io (authsrv tcp/567, wrkey→NVRAM, keyfs/convkeys rules incl. the keyfs-down BUGS note, `/lib/ndb/auth` speaksfor, `auth/debug` behavior); ARM Base System Architecture DEN0094C (GICv2 = no native PCIe MSI, v2m extension maps MSI→SPI, PCI legacy INTA–D are level-sensitive SPIs with implementation-defined IDs).
- **Vendor/community trackers (MEDIUM, pattern confirmation):** qemu GitLab #1364 + qemu-devel vmnet patch threads (vmnet entitlement requirement; ad-hoc signing insufficient; root-or-entitlement; `killed 137` evidence); libvirt-users/Red Hat bz 814898/netgate/virtio-win threads (checksum/TSO offload "ping works, TCP hangs" class); ARM char-unsigned ABI confirmation (LWN + embedded references).
- **Seam confidence note:** the research-store seam rated all web digests LOW (unverified provenance); per the v1.0 convention, the man-page/spec/tracker claims above are primary-source and treated as HIGH, the offload symptom-pattern corroboration as MEDIUM.

---
*Pitfalls research for: Harvey OS v2.0 — Full System & Drawterm*
*Researched: 2026-11-04*
