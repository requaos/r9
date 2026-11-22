# Feature Landscape — v2.0 "Full System & Drawterm"

**Domain:** Plan 9-derivative OS (Harvey userspace on r9 kernel, aarch64, qemu virt)
**Researched:** 2026-11-04
**Confidence:** HIGH (primary evidence is Harvey's own in-tree man pages, rc scripts, and C sources — the project IS a Plan 9 4e derivative, so its tree is the specification; drawterm behavior verified from 9fans/drawterm source at `main`)

---

## 0. The Canonical Drawterm→CPU Session Lifecycle (Verified End-to-End)

This is the expected-behavior contract the milestone must reproduce. Every step below is verified against in-tree Harvey sources (`sys/src/cmd/cpu.c`, `rc/bin/service/tcp17010`, `rc/bin/cpurc`, `usr/glenda/lib/profile`, `sys/man/6/authsrv`) and 9fans/drawterm `cpu.c`/`main.c`. Both are Plan 9 4th-edition lineage — the protocols match exactly.

### 0.1 Connection and authentication

1. **Host:** `drawterm -a <authaddr> -c <cpuaddr> -u glenda` opens a native window (gui-osx / gui-x11), then binds its synthetic devices into its own mini-namespace: `#c` cons, `#m` mouse, `#i` draw, `#I` net, `#U` host filesystem, `#A` audio (verified: drawterm `main.c`).
2. **TCP connect** to `tcp!<cpu>!17010` (port 17010 hard-coded via `netmkaddr(host,"tcp","17010")`).
3. **Auth-method negotiation:** client writes NUL-terminated `"p9 rc4_256 sha1"` (auth method + encryption/hash algs; `-e clear` sends just `"p9"` → no crypto layer). Server (`cpu -R`, started by `aux/listen` as user `none` from `/rc/bin/service/tcp17010`, which is literally `#!/bin/cpu -R`) replies with an empty string to accept.
4. **p9any v.2:** server's factotum sends `"v.2 p9sk1@<authdom>"` (protocol@domain offer list); client answers `"p9sk1 <dom>"`; server replies `OK` (authsrv(6) "P9any" section).
5. **p9sk1 handshake** (authsrv(6) "P9sk1" section — who challenges whom):
   - Client → server: `CHc` (8-byte random challenge).
   - Server → client: Ticketreq `{AuthTreq, IDs=server-hostid, DN=authdom, CHs, -, -}` (server's challenge).
   - Client: converts the typed password with `passtokey()` → 56-bit DES key `Kc`; **dials the auth server directly on tcp/567** (`authdial` → `netmkaddr(authserver,"tcp","567")`), sends the ticket request; AS (`auth/authsrv`, reading `/adm/keys` via keyfs) returns a ticket pair: `Kc{AuthTc,CHs,IDc,IDr,Kn}` + `Ks{AuthTs,CHs,IDc,IDr,Kn}`. If the user doesn't exist, the AS silently uses a one-time random key (no user-name enumeration).
   - Client decrypts its ticket with `Kc` → nonce key `Kn`. Wrong password → `"password mismatch with auth server"`, re-prompt.
   - Client → server: the server's ticket `Ks{AuthTs}` + authenticator `Kn{AuthAc,CHs}`.
   - Server's factotum (holding the machine key `Ks`, normally loaded from nvram at boot) decrypts the ticket, verifies `AuthAc`. Server → client: `Kn{AuthAs,CHc}`; client verifies → **mutual authentication**.
6. **Server becomes the user:** `auth_chuid(ai)` writes the capability to `#¤/capuse` (verified: `sys/src/libauth/auth_chuid.c`) — **requires the kernel cap device `#¤` (devcap: caphash + capuse)** — then `newns(user)` builds a fresh namespace from `/lib/namespace`.
7. **Crypto layer:** both sides exchange 4 random bytes, SHA-1-scramble them into two secrets, then `pushssl(fd, "rc4_256 sha1", ...)`. **`pushssl` hard-codes `#D/ssl/clone`** (verified: `sys/src/libc/9sys/pushssl.c` — "it is impossible for a user-level file server to stand in for the kernel device"). Requires **kernel devssl `#D`** with rc4_256 + sha1. Skipped entirely if the client negotiated `-e clear`.

### 0.2 Post-auth protocol (exportfs, not raw 9P session setup)

8. Client sends its cwd as a string (or `"!cmd"` for `cpu -c` / drawterm `-C`); server `chdir`s.
9. Server writes `"FS"` + `"/"` (the exportfs initial-protocol root announcement); client acks `"OK"` and **execs `exportfs` serving its own synthetic root** — including `/dev/draw`, `/dev/mouse`, `/dev/cons`, `/dev/snarf`, `/dev/audio`, and the host fs `#U`.
10. Server does `fversion(fd, 64*1024)` + `mount(fd, -1, "/mnt/term", MCREATE|MREPL, "")` — the **client's namespace appears at `/mnt/term`** (verified: `cpu.c` `remoteside()`). This is a devmnt kernel 9P client mount — v1.0 already proved FVERSION/MOUNT.
11. Server opens `/mnt/term/dev/cons` as fds 0/1/2 (the *client's* console), then `execl("/bin/rc", "rc", "-li")` with `$service=cpu`, `$user`, `$home` set.
12. **Profile runs** (`rc -l` → `/usr/$user/lib/profile`; verified in-tree `usr/glenda/lib/profile`, `case cpu`):
    - If `/mnt/term/mnt/wsys` exists (real Plan 9 terminal with rio already running) → bind it into `/dev`, the session becomes a window on the *terminal's* rio.
    - Else (**drawterm case**): `bind /mnt/term/dev/cons /dev/cons`, `bind /mnt/term/dev/consctl /dev/consctl`, `bind -a /mnt/term/dev /dev` — the drawterm's draw/mouse/snarf appear in `/dev` because a headless cpu-server kernel has no `#i` of its own — then `exec rio`.
13. **rio starts on the cpu server**, opens `/dev/draw/new` (which is drawterm's devdraw over 9P) and `/dev/mouse`; serves `/mnt/wsys` and posts `/srv/rio.$user.$pid` (rio(1) FILES section, verified). libdraw's `initdraw` opens `/dev/draw/new`, falling back to binding kernel `#i` only if absent (`libdraw/init.c:117`, verified) — **there is no `$drawdev` env var in Plan 9 libdraw; device selection is purely namespace binding** (that's plan9port's devdraw convention, not native).
14. User sees a rio desktop in the host window: right-click menu → `New` → window with rc prompt, running on the guest. Window resize/keyboard/mouse flow back over the 9P mount through the rc4_256/sha1 channel.

### 0.3 Boot-time service scripts

- **`/rc/bin/cpurc`** (verified in-tree): starts `mntgen` (srv `#s` /srv/slashn), `ndb/cs` (sets `/dev/sysname`), runs `cpurc.local` + `/cfg/$sysname/cpurc`, `ip/ipconfig` (+ loopback `ip/ipconfig loopback /dev/null 127.1`), `ndb/dns -r` if no dns, then `aux/listen -q tcp` (the line includes port 17010 in its grep guard), `aux/timesync`.
- **`aux/listen`** (listen(8), verified): scans `/bin/service` (bound from `/rc/bin/service`); service name = net+port (`tcp17010`). Services in `/bin/service` run **as user `none`** with a fresh `/lib/namespace`; services in the trusted dir (`-t`) run as the listen user. `cpu -R` therefore starts as `none` and `auth_chuid` switches identity post-auth.
- **Auth server bits in cpurc** (commented block, verified lines 39–54): on the AS, start `auth/keyfs -wp -m /mnt/keys /adm/keys` and `auth/cron` *before* listening, and rename `/rc/bin/service.auth/authsrv.tcp567` → `tcp567` (exists in-tree) so `authsrv` answers ticket requests on tcp/567 from the trusted service dir.
- **boot(8) `service=cpu`** selects cpurc over termrc; on r9 the v1.0 hand-written `bootr9.c` boot chain must learn this switch (currently only the interactive shell path exists — flag for phase planning).

### 0.4 Auth domain setup on a fresh install (concrete recipe)

From `auth(8)`, `keyfs(4)`, `ndb(6)`, `authsrv(6)`:

1. Choose an **authdom** (48-byte max string, e.g. `harvey`) and a **hostid** (the server's hostowner user name, e.g. `glenda`). Server machines keep `(56-bit DES machine key, authid, authdom)` — classically in **nvram** (`#r/nvram`, written by `auth/wrkey`, read by keyfs/factotum at boot).
2. AS machine (can be the same guest — authsrv(6): "The AS may be a stand-alone machine or a general-use machine such as a CPU server"): create `/adm/keys`; start `auth/keyfs -m /mnt/keys /adm/keys`; `auth/changeuser glenda` per user (prompts twice for password, plus optional APOP secret).
3. **ndb** entries: system entries with `sys=`, `ip=`, `ether=`, plus `auth=<as-address> authdom=<dom>` tuples (ndb(6): the auth+authdom pair selects the AS for a domain); `/lib/ndb/auth` holds speaksfor/hostid mappings.
4. **Terminal/drawterm side:** a real terminal's factotum prompts user+password at boot and validates against the AS. **Drawterm has no factotum** by default — verified in its `cpu.c`: `mountfactotum()` fails → falls back to internal `getkey()`/`passtokey()` (or `-s secstore` fetch). So the host needs no Plan 9 auth infrastructure; just user+password.

---

## 1. Feature Area: Full aarch64 Userspace

### The elephant, quantified (measured in-tree)

| Metric | Count |
|---|---|
| `sys/src/cmd` entries | 274 (130 dirs + 132 single-file + mkfiles) |
| Total `.c`/`.y` files under cmd | **3357** |
| of which ghostscript (`cmd/gs`) | **753** (22% of the tree in one dir) |
| of which toolchain suites (4*/5*/6*/7*/8*/9*/k*/q*/v* + cc/cpp/ld/lex) | ~284 across 28 dirs |
| of which `aux` | 186 (mix of guest daemons and host-only tools) |
| remainder (real porting surface) | **~2134 files** |
| `sys/src/lib*` dirs | 35 total; v1.0 built 13 (libc libbio libregexp libip libndb libmp libsec libString libthread lib9p libauth libauthsrv libflate) → **22 remaining** |
| v1.0 baseline | 16 binaries + boot chain (subset-build.sh) |
| games (`sys/src/games`) | 20 entries (7 dirs) — separate tree, plus `aquarela`/`astro` in cmd |
| APE (`sys/src/ape`) | 15 ported cmds + its own libc — **separate question per PROJECT.md** |
| `cmd/go` | 0 C files (fetch scripts only); Go unsupported on plan9/arm64 — out of scope |

### Table stakes for a *working* terminal/CPU server (what actually gets exercised)

| Group | Contents | Why expected | Complexity |
|---|---|---|---|
| Shell + coreutils | rc ✓, the 12 ✓ + diff, comm, sort, uniq, tr, awk, ed, tar, gzip, bzip2, compress, dd, du, file, strings, xd, history, test ✓… | Everything downstream (cpurc, profile, mkfiles, rc scripts in /rc/bin) assumes them | LOW (v1.0 pattern proven) |
| Guest build chain | 7a/7c/7l, cc, cpp, mk, yacc, lex, ar, nm, strip, size | A cpu server that can't compile for its own arch isn't a cpu server; 9front donor tree proves these build for arm64 | MEDIUM (7a/7c/7l vendored already; cc/cpp/lex/yacc are portable 4e C) |
| Session/system daemons | aux/listen, aux/timesync, mntgen, ndb/cs, ndb/dns, netstat, srv, ramfs, exportfs, import | cpurc + cpu service + namespace mounts fail without them | LOW-MED |
| Auth suite | auth/factotum ✓, keyfs, authsrv, changeuser, passwd, login, newns, wrkey, as, guard.srv (80 files in cmd/auth) | p9sk1 auth domain feature | MEDIUM |
| GUI stack | libdraw, libmemdraw, libmemlayer, libframe, libcontrol (5 of the 22 remaining libs), rio (9 files), acme (20), sam+samterm (20+), plumb, stats, winwatch, page, proof, screenlock | The drawterm session IS a GUI session; acme/sam are the editors everyone launches first | MEDIUM-HIGH (GUI libs are portable C but large; rio/acme need font files `/lib/font/bit/*` in the image) |
| IP userspace | ip/ipconfig ✓, ping, telnet, hget, ip/httpd?, snoopy, dhcpclient (126 files in cmd/ip) | Networking validation + dogfooding | MEDIUM |

### Long tail (compile-green but little runtime value in v2.0)

| Group | Notes | Complexity |
|---|---|---|
| Other-arch toolchains (4/5/6/8/9/k/q/v suites, ~25 dirs) | Useful for cross-compiling; zero session value; some have known host-build breakage (`4a` rfork/bind/lex.c per PROJECT.md) | MED each, LOW value |
| troff suite + postscript + pic/eqn/tbl/grap | Documentation rendering; `troff2html`, `ms2html` | MED |
| lp (printing), fax, telco | No printer/modem in qemu | LOW value |
| upas (mail, 69 files) | Defer unless dogfooding mail | MED |
| fossil/venti/cwfs/9660srv/dossrv/ext2srv/tapefs (39+71+… files) | **No block device in the v2.0 guest** — build-only, untestable at runtime; venti/fossil are the heaviest C in the tree | HIGH, flag as compile-only + unverifiable |
| gs (ghostscript, 753 files) | Biggest single dir; historically fragile (ghostscript fonts are the repo's only GPL island; Dockerfile TODO notes breakage) | HIGH — recommend gating: fix-or-exempt |
| games | aquarela, astro, festoon…; `sys/src/games` (mahjongg has known cpp breakage per PROJECT.md) | LOW value |
| vnc, ssh1/ssh2, cifs, nfs, webfs, nntpfs, wikifs | Network extras; build + smoke only | MED |

### Known structural breakage to fix first (from PROJECT.md, verified)

- `sys/src/cmd/jpg/mkfile:56-68` derives a nonexistent `oc` compiler from `cputype=unix` and **aborts the cmd DIRS recursion** before `ip/` and `venti/` — the full-userspace build literally cannot complete until this is fixed or jpg is gated.
- `mahjonng` cpp breakage (games), `4a` lex.c host-build failure.
- LP64 bug class proven in v1.0 phases (pswt.c sign-extension, memset/memmove width): expect a tail of similar latent 32-bit assumptions in the long tail. Mitigation: the pinned 9front donor tree (same 4e lineage) builds its full userspace for arm64 — proof of feasibility and a diff reference.

### APE's role (the "separate question")

APE is a self-contained POSIX sandbox (own libc, 15 ported cmds). **Nothing in the drawterm/cpu/rio session touches it.** Recommendation: exclude from v2.0 success criteria; revisit only if a needed host tool port demands it.

---

## 2. Feature Area: Guest Networking (r9 virtio-net + TCP/IP)

### Table stakes

| Feature | Why | Complexity | Notes |
|---|---|---|---|
| virtio-net device in r9 (qemu `-M virt` `virtio-net-device`) | Only NIC the virt platform offers | HIGH (kernel driver, r9 side) | v1.0 r9 has zero devices beyond console; first DMA-capable device — flag r9 phase research |
| Kernel IP path (devip-equivalent `#I` + devether `#l`) | Plan 9's TCP/IP is in-kernel; userspace `ip/` daemons are control-plane | HIGH | "kernel devip vs userspace stack" is an ARCHITECTURE question — but the *feature* contract is: `/net/tcp`, `/net/udp`, `ip/ipconfig`, `dial()`, `listen()`, `announce()` behave per man pages |
| Loopback `127.1` | cpurc configures it unconditionally; in-guest AS dial uses it | LOW (comes with devip) | |
| `ip/ipconfig` on the virtio link | v1.0 already builds it; needs kernel `#I` to talk to | LOW userspace / depends HIGH | static IP acceptable for v2.0 (document); DHCP (dhcpclient) is a differentiator |

### Differentiators
| Feature | Value | Complexity |
|---|---|---|
| `ndb/dns` + `ndb/cs` working against a real resolver | Guest resolves names; drawterm can use names | MED |
| dhcpd/dhcpclient | Zero-config guest addressing | MED |

### Anti-features
| Feature | Why avoid | Instead |
|---|---|---|
| Userspace TCP stack (lwip port) contradicting Plan 9 devip ABI | Breaks every `ip/` tool's assumptions; double maintenance | Kernel devip-compatible `#I` (or an r9-native equivalent presenting the same `/net` file interface) |
| IPv6/ppp/6in4 in v2.0 | No consumer in the session flow | defer |

---

## 3. Feature Area: In-Guest p9sk1 Auth Domain

### Table stakes

| Feature | Why | Complexity | Notes |
|---|---|---|---|
| `auth/keyfs` serving `/mnt/keys` over `/adm/keys` | Interface to the auth DB for authsrv + changeuser | LOW (in-tree) | keyfs decrypts keyfile with DES key from `#r/nvram` by default; `-p` prompts — **nvram substitute needed** (see anti-features) |
| `auth/authsrv` on tcp/567 via `service.auth` trusted dir | Issues ticket pairs; **drawterm dials this port directly** — both 567 AND 17010 must be reachable from host | LOW-MED | cpurc's commented block is the recipe |
| Machine key loaded into server factotum at boot | `srvp9auth` needs `Ks` to decrypt tickets | MED | Classic path is nvram; guest has none → bake a key file into the image and `echo 'key proto=p9sk1 dom=harvey user=glenda !password=…' >/mnt/factotum/ctl` from cpurc.local (documented dev-key posture) |
| `auth/changeuser` + baked `/adm/keys` | User provisioning | LOW | **paqfs is read-only** → runtime `changeuser`/`passwd` writes don't persist; bake users at image build, or bind a small ramfs over `/adm` (ephemeral). Scoping decision for requirements |
| ndb populated (sys/authdom/auth + `/lib/ndb/auth`) | AS discovery + speaksfor | LOW | Static file in image |
| r9 kernel **devcap `#¤`** (caphash/capuse) | `auth_chuid` is how `cpu -R` becomes the user | MED (small device, r9 side) | Note cpurc's `rm -f '#¤/caphash'` hardening line conflicts with late capability creation — for v2.0 keep caphash present (we control the guest) |
| DES in libsec | passtokey/tickets | DONE (libsec built in v1.0) | |

### Differentiators
| Feature | Value | Complexity |
|---|---|---|
| `auth/secstore` + drawterm `-s` | Hosts store keys server-side | MED — defer |
| `auth/wrkey` + emulated nvram | Reboot-without-baked-keys fidelity | MED — only if a real nvram file backend is added to qemu launch |
| DP9IK/modern PAKE | Real security | HIGH — explicitly out (TLS modernization milestone) |

### Anti-features
| Feature | Why avoid | Instead |
|---|---|---|
| Shipping no auth (`-a none` listener or unauthenticated rexexec on 17009) | Milestone goal is p9sk1; rexexec/tcp17009 exists in-tree but is unauthenticated legacy | p9sk1 only; keep tcp17009 uninstalled |
| Interactive keyfs `-p` prompt on the serial console in CI | Blocks headless boot | baked DES key + keyfile |
| Persisting auth DB | no writable fs in v2.0 | image-baked keys, documented throwaway |

---

## 4. Feature Area: CPU Server on tcp/17010

### Table stakes

| Feature | Why | Complexity | Notes |
|---|---|---|---|
| `cpu -R` installed at `/bin/cpu` + `/rc/bin/service/tcp17010` | The service itself | LOW (cpu.c builds against libauth/libsec/libc — all green) | needs `auth_proxy`, `auth_chuid`, `pushssl`, `fversion`, `mount` — all in-tree |
| r9 **devssl `#D`** with rc4_256 + sha1_160 | default negotiated crypto; hard-coded in libc pushssl | MED-HIGH (r9 kernel; 9k `devssl.c` ~1300 LoC is the reference) | **escape hatch: drawterm `-e clear`** — verified supported both sides (cpu.c `setamalg`, drawterm `-e`). Recommend: v2.0 success criteria on `clear` first, devssl as follow-on within the milestone |
| `exportfs` on the server… actually on the *client*; server needs devmnt mount | `/mnt/term` mount | DONE in v1.0 (devmnt FVERSION/MOUNT) | exportfs binary needed only if guest also acts as client (import/cpu out) — build it anyway (LOW) |
| `aux/listen` + cpurc wiring | service dispatch as user none | LOW | |
| Note/hangup propagation | rio window DEL → kill | v1.0 has notes + devproc | cpu.c's `cpunote` notefs is a client-side nicety; drawterm (9fans) doesn't implement it — hangup-on-close comes free when exportfs conn dies |
| boot chain `service=cpu` → cpurc | headless server boot | MED | v1.0 bootr9.c only does the shell path; needs the service switch + conffile/plan9.ini equivalent in the paqfs image |

### Differentiators
| Feature | Value | Complexity |
|---|---|---|
| `cpu` *client* from inside the guest (guest→guest cpu) | Dogfoods the other half of cpu.c | LOW once service works |
| `import` (tcp17007 `exportfs -a`) | Guest imports host trees | LOW-MED |
| `aan` (always-available network, reconnect) | Robust sessions | MED — defer |

### Anti-features
| Feature | Why avoid | Instead |
|---|---|---|
| rlogin/telnetd/rexexec listeners (tcp513/tcp23/tcp17009) | Unauthenticated legacy remote exec; security regression as a default | cpu+p9sk1 only |
| Running listen services as hostowner instead of `none` | Breaks the listen(8) trust model | default behavior is correct |

---

## 5. Feature Area: rio Desktop over Drawterm

### Table stakes

| Feature | Why | Complexity | Notes |
|---|---|---|---|
| GUI libs aarch64: libdraw, libmemdraw, libmemlayer, libframe | rio/acme/sam link closure | MED (portable 4e C; 9front arm64 proves the lineage compiles) | libcontrol only needed by fgui/games — optional |
| rio (9 files) + `/lib/font/bit/*` fonts in image | The desktop | LOW-MED | fonts must be staged into paqfs image |
| `/dev/draw`+`/dev/mouse` arrive via drawterm exportfs mount | **No guest GPU/framebuffer needed** (explicit PROJECT.md decision) | DONE-ish (devmnt) | depends on exportfs msize ≥ draw iounit (cpu.c negotiates 64k fversion; drawterm probes /dev/draw iounit) |
| profile `case cpu` doing the `/mnt/term/dev` binds + `exec rio` | Session glue | LOW | ship a sane default `/usr/glenda/lib/profile` in the image (in-tree glenda profile is the reference, minus upas/plumber bits we defer) |
| Mouse: 3-button semantics + scroll | rio menus are button-2/3 | LOW | drawterm handles host mapping (chord on macOS) |

### Differentiators
| Feature | Value | Complexity |
|---|---|---|
| acme, sam+samterm, page, proof | The real Plan 9 desktop experience | MED (each) — acme is the highest-value one |
| plumber | acme/rio integration | LOW-MED |
| winwatch, stats, screenlock | Polish | LOW |
| snarf/clipboard bridging to host | drawterm serves /dev/snarf | LOW (comes free with devmouse) |

### Anti-features
| Feature | Why avoid | Instead |
|---|---|---|
| r9 framebuffer/virtio-gpu + kernel devdraw | Whole point of drawterm architecture is NO guest graphics stack | drawterm's devdraw over 9P |
| `segattach("screen")` fast paths in libdraw | Remote draw doesn't support it; libdraw falls back to draw messages automatically | do nothing — verify fallback works (phase research item, MEDIUM confidence it "just works") |
| vnc server instead of drawterm | Milestone chose drawterm; vncs exists in-tree for later | defer |

---

## 6. Feature Area: Host-Side Drawterm + Tap/Bridge (flake)

### Table stakes

| Feature | Why | Complexity | Notes |
|---|---|---|---|
| Flake package + app for **9fans/drawterm** | `nix run .#drawterm` completes the one-command story | LOW | pure C + vendored libsec; builds with host cc + X11/Cocoa; **protocol-compatible by construction** (same 4e lineage as Harvey cpu.c — verified). 9front's drawterm fork adds dp9ik/TLS-first which Harvey's `cpu -R` does not implement → 9fans is the correct default (MEDIUM-HIGH) |
| Guest reachable on tcp/567 + tcp/17010 from host | drawterm dials BOTH (AS then cpu) | — | two transport options below |
| qemu user-mode + `hostfwd` for 567/17010 | **Unprivileged happy path** — works on darwin and linux, CI-friendly | LOW | guest behind NAT; cs/dns in guest see qemu's 10.0.2.x |
| Documented tap/bridge per host: darwin vmnet (socket_vmnet / lima-managed) and linux tap+bridge | Real L2, guest has own IP, full-fidelity Plan 9 networking (milestone requirement) | MED | privileged; document as the "real" setup; keep hostfwd as the default/CI path |

### Differentiators
| Feature | Value | Complexity |
|---|---|---|
| Drawterm geometry/font flags (`-G`/`-g` are *not* 9fans flags — window sizing is gui-specific; 9fans drawterm uses `-G`? — no: flags are exactly `-d -a -c -e -k -s -u -C` per in-tree man + source) | Avoid inventing flags | — |
| secstore support in flake drawterm | Password-less reconnect | MED — defer with secstore feature |

### Anti-features
| Feature | Why avoid | Instead |
|---|---|---|
| Requiring root/vmnet for the default path | Kills CI + casual use | hostfwd default, tap documented |
| Shipping both drawterm forks | Confusion | 9fans pinned |

---

## 7. Feature Dependencies

```
Full aarch64 userspace (GUI libs, rio, auth suite, aux/listen, exportfs, cpu)
    └──required by──> everything below

Guest networking (virtio-net + kernel IP + ipconfig)
    └──required by──> Auth domain (AS dial tcp/567, even loopback needs #I)
                          └──required by──> CPU service (p9sk1 + tcp/17010)
                                                └──required by──> rio over drawterm
                                                                    └──required by──> host drawterm end-to-end

r9 devcap #¤  ──required by──> CPU service (auth_chuid)
r9 devssl #D  ──required by──> CPU service default crypto (OPTIONAL via drawterm -e clear)
Writable-or-baked /adm/keys + nvram substitute ──required by──> Auth domain
Fonts in paqfs image ──required by──> rio
Host networking (hostfwd or tap) ──required by──> host drawterm end-to-end
```

### Dependency notes

- **CPU service requires auth domain AND networking:** `cpu -R`'s `auth_proxy role=server` talks to the AS; the AS must answer tcp/567; listen must bind tcp/17010. No partial-credit ordering — but auth and net can be validated independently first (ipconfig ping test; authsrv ticket smoke via `auth/debug`).
- **rio requires ONLY the cpu service** (+GUI libs): no guest graphics device at all. The chain auth→cpu→rio is strict.
- **Full userspace is independent** of the networking/auth chain and is the largest parallel workstream — it can proceed concurrently and land incrementally (mk recursion fixes first, then groups).
- **devssl is the only cleanly optional kernel piece** (`-e clear`), making it the natural descope pressure-valve.

### v1.0 capabilities consumed (do NOT re-research)

paqfs initrd delivery; devroot/devsrv/devmnt (the `/mnt/term` mount itself); devcons read+write; devproc + notes (rio window DEL/hangup); devenv (`$service`, `$user`, `$cputype`); devpipe/devdup (rc plumbing); preemptive multitasking (rio/acme are concurrent programs); 26-syscall measured set as the baseline to extend (new demands: `mount` on a network fd exists; watch for `segattach` usage in GUI libs — expected absent, verify; `fversion` 64k exists).

---

## 8. MVP Recommendation (for requirements scoping)

### The milestone's true spine (in dependency order)

1. **Full-userspace build machinery**: fix jpg/mkfile recursion abort + mahjonng/4a host issues; per-group aarch64 bring-up with build-green gates. Success = `mk install` completes over an explicit group list; runtime-tested groups: shell/core, build chain, session daemons, auth, GUI. Compile-only groups: storage servers, gs, troff, upas, games.
2. **Guest networking**: virtio-net + `#I` + ipconfig static IP + loopback; prove with in-guest dial/listen echo and host->guest ping/tcp.
3. **Auth domain**: baked `/adm/keys` + machine key, keyfs, authsrv on 567, devcap in r9; prove with `auth/debug` in-guest.
4. **CPU service**: listen + tcp17010 + cpurc + `service=cpu` boot path; **first with `drawterm -e clear`** (no devssl), then devssl + default crypto.
5. **rio session**: GUI libs + rio + fonts + default profile; success = rio desktop in drawterm window, New window → rc prompt, mouse menus work.
6. **Host flake**: drawterm pkg + hostfwd default + tap/vmnet docs.

### Defer candidates (scope pressure-valves, in order)

- gs (753 files) → compile-gated or exempted
- upas, lp, fax, telco → build-only or excluded
- fossil/venti/cwfs → build-only (untestable without block device)
- secstore, APE entirely, IPv6, aan, dhcpd
- devssl → only if the milestone accepts `-e clear` as the shipped default (documented: private tap/localhost network)

---

## 9. Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---|---|---|---|
| mk recursion + group build machinery | HIGH (unblocks all) | MED | P1 |
| GUI libs + rio + acme + sam | HIGH | MED | P1 |
| virtio-net + kernel IP | HIGH | HIGH | P1 |
| Auth domain (keyfs/authsrv/factotum keys/devcap) | HIGH | MED | P1 |
| cpu -R service + listen + cpurc | HIGH | LOW-MED | P1 |
| Drawterm flake + hostfwd | HIGH | LOW | P1 |
| devssl rc4_256/sha1 | MED (fidelity; `-e clear` exists) | MED-HIGH | P2 |
| Tap/bridge per-host docs | MED | MED | P2 |
| Guest build chain on-guest (7a/7c/7l on aarch64) | MED (dogfooding) | MED | P2 |
| ndb/dns + cs | MED | MED | P2 |
| upas, troff, storage servers, gs, games, APE | LOW for v2.0 | MED-HIGH | P3 |

---

## 10. Concrete Expected User-Visible Behavior (success-criteria-ready)

1. `nix run .#boot-r9` → guest boots to cpurc, no interactive prompts; serial log shows ipconfig address, keyfs mounted, listen running (`netstat` shows tcp 567 + 17010 Listen).
2. `nix run .#drawterm` (host) → window opens, prompts `user`/`password`; wrong password → "password mismatch with auth server" re-prompt; right password → rio desktop appears (~seconds over localhost).
3. In rio: button-3 menu → New → rc prompt window; `cat /dev/sysname`, `lc /`, `ps` work; `acme` launches; DEL deletes window (notes); rio Resize/Move/Exit behave.
4. `echo hello > /dev/cons`-style sanity: text typed in window echoes; `bind`/`mount` of /mnt/term visible: `lc /mnt/term/dev` shows draw/mouse/snarf from the host.
5. Full-userspace: `mk install` green for the declared group list from a clean checkout; `/aarch64/bin` population count asserted in CI (baseline: 16 → target: hundreds).
6. CI leg (linux): boot + auth + cpu + scripted drawterm-equivalent smoke (or expect-driven serial assertion of listener readiness) green; darwin documented-manual.

---

## 11. Sources

| Source | Used for | Confidence |
|---|---|---|
| Harvey in-tree `sys/src/cmd/cpu.c` (1171 lines, read in full) | cpu protocol both sides, notefs, remoteside flow | HIGH (project's own contract) |
| Harvey in-tree `rc/bin/service/tcp17010`, `cpurc`, `lib/namespace`, `usr/glenda/lib/profile`, `rc/bin/service.auth/` | service wiring, rio start idiom, auth-server recipe | HIGH |
| Harvey in-tree man pages: cpu(1), exportfs(4), factotum(4), keyfs(4), authsrv(6), auth(8), ndb(6), rio(1), listen(8), drawterm(8) | protocol + admin behavior | HIGH |
| Harvey in-tree `sys/src/libauth/auth_chuid.c`, `sys/src/libc/9sys/pushssl.c`, `sys/src/libdraw/init.c` | devcap `#¤` requirement, devssl `#D` hard-coding, draw-device discovery | HIGH |
| 9fans/drawterm `main.c`, `cpu.c` (fetched from GitHub `main`) | client flags, device set served back, AS dial on 567, no-factotum fallback, `mkserverticket` fallback | HIGH |
| In-tree counts (`sys/src/cmd`, `lib*`, `games`, `ape`) via direct measurement | quantification | HIGH |
| 9front drawterm fork protocol differences (dp9ik/TLS) | client selection rationale | MEDIUM (not re-verified against 9front source this session) |
| libdraw remote-`segattach` fallback behavior | rio-over-9P performance path | MEDIUM (expected from code structure; verify in phase) |

**Gaps / phase-research flags:** r9 devcap/devssl/devether/devip implementation effort (r9-side research); nvram substitute design (baked key vs qemu nvram file); writable `/adm` scoping (ramfs overlay vs build-time baking); qemu 11.1.2 virtio-net-on-virt quirks (precedent: Phase 2/3 accommodated donor drift harness-side); whether hosted CI permits tap (hostfwd assumed not).
