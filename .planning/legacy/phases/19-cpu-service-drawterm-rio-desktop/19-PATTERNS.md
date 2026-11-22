# Phase 19: CPU Service + Drawterm + rio Desktop - Pattern Map

**Mapped:** 2026-11-14
**Files analyzed:** 25 (new + modified, spanning both repos)
**Analogs found:** 23 / 25 (2 partial: the darwin 64-bit Cocoa backend and the `-G`/`-O` flag semantics are novel surface with in-file/in-tree partial precedents only)

All analog paths below verified git-TRACKED (`git ls-files` non-empty; 30/30 Harvey, 10/10 r9). No mirror paths. Paths under `/home/req/code/r9/` are the sibling repo (kernel work lands there, branch `main`, push-before-pin).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `r9/port/src/devip/stack.rs` (M) | kernel device (TCP accept pool) | streaming | `r9/port/src/devip/proto.rs` `tcp_socket` (the 18-07 fix) | exact |
| `r9/port/src/devssl.rs` (N) | kernel device (`#D`) | transform (crypto framing) | `r9/port/src/devcap.rs` + donor `sys/src/9k/port/devssl.c` | exact |
| `r9/x86_64/src/exec.rs` (M) | kernel exec path (`#!` pre-pass) | file-I/O | donor `sys/src/9k/port/sysproc.c:282-324` + own `read_aout` | exact |
| `r9/x86_64/src/syscall.rs` (M) | kernel syscall dispatch | request-response | own DISPATCH table + donor `sys/src/cmd/exportfs/exportsrv.c` consumers | exact |
| `r9/x86_64/src/main.rs` (M) | kernel boot env seed | config | own `seed_root_group` call; donor `sys/src/9k/boot/boot.c` `execinit` | exact |
| `r9/x86_64/src/platform/q35/timer.rs` (M) | kernel timer ISR | event-driven | `r9/aarch64/src/platform/virt/timer.rs:129-130` | exact |
| `r9` host pins (`cfg(test)`) + xtask boot profiles (N) | test | request-response | `r9/port/src/devip/proto.rs` pin `tcp_two_write_client_pattern_round_trips` (doc-cited) | exact |
| `sys/src/cmd/unix/drawterm/cpu.c` (M) | vendor client (`-G`/`-O` flags) | request-response | own ARGBEGIN `'e'`/`'C'` arms | partial (in-file) |
| ~~`sys/src/cmd/unix/drawterm/gui-cocoa/`~~ → `sys/src/cmd/unix/drawterm/gui-none/` (N; OQ-1 RESOLVED 2026-11-14 — gui-cocoa is NOT a deliverable: amended D-03, darwin -G text-only this milestone, deferral + reopener recorded by 19-10; 19-06 delivers gui-none) | vendor no-window backend (loud stubs) | request-response (stdio bridge) | `sys/src/cmd/unix/drawterm/gui-x11/` (interface contract only) + Make.config `AUDIO=none` selection precedent | partial |
| `nix/drawterm.nix` or flake-inline package (N) | nix package (build config) | batch | `flake.nix` `hostEchoX64Pkg` (292-302) + `nix/toolchain.sh` vendored-9cc build | role-match |
| `nix/drawterm-x64.sh` wrapper (N) | flake-app wrapper script | request-response | `nix/boot-r9-x64.sh` arg parse + `mkScriptApp` shape | role-match |
| `flake.nix` (M) | flake config (apps) | config | own `authSmokeR9X64Pkg` block (385-390, 440-443) | exact |
| `build/x86_64/mkimage.sh` §15 (M) | build script (bake contract) | batch/file-I/O | own §14 AUTH contract (1250-1664) | exact |
| `build/x86_64/authdomain-gen.c` (M) | host generator (creds emission, D-12) | file-I/O | own `feed` writefile (line 587) + 14f2b host-side copy | exact |
| `build/x86_64/root-tree/rc/bin/cpurc` (N; naming planner's) | boot script (rc) | event-driven (boot orchestration) | `build/x86_64/auth-termrc` + `auth-sweep.rc`; donor `sys/src/9k/k10/root/k10root.rc` | exact |
| `tcp17010` service file (N; staged by §15) | config data | n/a (2-line script) | `sys/src/9k/k10/root/tcp17010` (verbatim donor) | exact |
| `build/x86_64/tiers/tier2-services.txt` (M) | build data (append-only allowlist) | batch | own `cmd/aux:listen1` entry (line 53) | exact |
| `build/x86_64/drawterm-gate.expect` (N; naming planner's) | expect gate driver (test) | request-response (pty) | `build/x86_64/smoke-r9-auth.expect` (515 lines) | exact |
| `nix/<-G gate driver>.sh` (N) | flake-app gate script | request-response | `nix/smoke-r9-auth-x64.sh` | exact |
| `nix/boot-r9-x64.sh` (M, full-system mode) | boot/launch script | request-response | own `--net` arg parse (147-173) + image-selection seam (234-237) | exact |
| full-chain smoke extension (`nix/smoke-r9-x64.sh` lineage or new app) (M) | smoke gate | request-response | `nix/smoke-r9-x64.sh` / `smoke-r9-auth-x64.sh` chassis | exact |
| `build/x86_64/README.md` (M) | docs | n/a | own existing sections (drawterm/darwin/cpu land here) | role-match |
| `.planning/phases/19-*/19-EVIDENCE.md` (N) | evidence record | n/a | 17-08 maintainer-ratification record style (per D-13) | role-match |
| `build/x86_64/root-tree/lib/ndb/local.template` or §15 compose append (M, discretionary) | config data (ndb) | config | §14f3's `tcp=ticket port=567` append (mkimage.sh:1530-1532) | exact |

(M = modify, N = new.)

## Pattern Assignments

### `r9/port/src/devip/stack.rs` — AcceptPool TCP fix (D-08/CR-03)

**Analog:** `r9/port/src/devip/proto.rs` `tcp_socket` — the 18-07 connect-path fix whose two calls the pool site mirrors.

**Fix site** (`stack.rs:814-822`, `spawn_listener` — sets only hop limit today):
```rust
    fn spawn_listener(core: &mut StackCore, port: u16) -> Result<SocketHandle> {
        let mut s = tcp::Socket::new(
            tcp::SocketBuffer::new(alloc::vec![0; POOL_SOCK_BUF]),
            tcp::SocketBuffer::new(alloc::vec![0; POOL_SOCK_BUF]),
        );
        s.set_hop_limit(Some(255));
        s.listen(port).map_err(|_| "devip: listen failed")?;
        Ok(core.sockets.add(s))
    }
```

**The precedent to copy** (`proto.rs:2124-2135`; the doc comment 2105-2123 already names this phase — "a >2048-byte bulk write would block in the send arm on the same never-driven timer — Phase 19's cpu path rides this seam"):
```rust
fn tcp_socket(ttl: u32) -> tcp::Socket<'static> {
    let mut s = tcp::Socket::new(
        tcp::SocketBuffer::new(alloc::vec![0; stack::POOL_SOCK_BUF]),
        tcp::SocketBuffer::new(alloc::vec![0; stack::POOL_SOCK_BUF]),
    );
    if ttl > 0 && ttl <= 255 {
        s.set_hop_limit(Some(ttl as u8));
    }
    s.set_nagle_enabled(false);
    s.set_ack_delay(None);
    s
}
```

Pool members are 2048/2048 (`POOL_SOCK_BUF: usize = 2048`, stack.rs:703) and are the accepted connection's socket for life (take_connected → devip listen-file open), so the fix covers announce-path sockets end-to-end. Pin precedent: `tcp_two_write_client_pattern_round_trips` (cited at proto.rs:2119-2120) is the exact WR-04-leg shape; `AcceptPool::all_listening`/`listening_count` (stack.rs:831-845) are the existing cfg(test) assertion helpers; `enter_stack`/`install_test_stack` (stack.rs:687-689) is the pin's stack-cell discipline.

---

### `r9/port/src/devssl.rs` — NEW `#D` device (DESK-04)

**Analog (structure):** `r9/port/src/devcap.rs` — the established small-device port skeleton. **Analog (semantics):** `sys/src/9k/port/devssl.c` (1520 lines, donor of record).

**devcap's port posture** (`devcap.rs:1-14`, the module-doc pattern every port follows — donor cited, "port the donor, never the phrase"):
```rust
//! devcap — the `#¤` capability device (AUTH-03, 18-02).  A
//! semantics-level Rust port of 9k's `sys/src/9k/port/devcap.c` (287
//! lines, the donor of record; ...).  Donor line cites below name
//! the semantics being ported; this is not a literal translation (the
//! project's D-06: port the donor, never the phrase).
```

**devcap's skeleton to replicate** (`devcap.rs:50, 71-75, 102-103, 128-134`): `REGISTRY_RUNE` const + `Lock<State>` module static + free-function `attach/walk/open/read/write/stat/remove` + `#[cfg(test)] install_test_state()` for pins. Formatted error transport when `&'static str` Result can't carry bytes (`devcap.rs:141-148`, `CapErr::InvalidCapability(Vec<u8>)` — the pattern devssl's ctl-verb errors reuse). The remove() arm at `devcap.rs:359-367` documents the exact situation devssl's arrival changes: "the REMOVE syscall is loud-unimplemented in r9's table this era — this op is the ported half, host-pinned now, wired when sys_remove lands."

**devssl donor's fidelity corners** (port donor-verbatim — RESEARCH Pitfall 11):
- QID layout (`devssl.c:99-101`): `#define TYPE(x) ((x).path & 0xf)` / `#define CONV(x) (((x).path >> 5)&(Maxdstate-1))` / `#define QID(c, y) (((c)<<5) | (y))`; qid enum at 86-97 (Qtopdir..Qhashalgs), `Maxdstate=512`, `dstate[Maxdstate]` at 82.
- Algorithm tables (`devssl.c:970-977, 1005-1019`, NOSPOOKS active — `#define NOSPOOKS 1` at devssl.c:13): hashtab `{ "sha1", SHA1dlen, sha1, }`; encrypttab `{ "rc4_256", 1, RC4, initRC4key, }` — the rc4_256/sha1 subset pushssl needs.
- ctl verb parse (`devssl.c:1150-1211`): the `alg` verb enforces `if(s->c == 0) error("must set fd before algorithm")` (1153-1154); `max`/`maxpad` arithmetic (1157, 1189-1195) is the "SSL format" framing that must not be "cleaned up"; secrets are **dec64, not hex** (`t = dec64(x, l, p, strlen(p))` at 1196-1209) of 20-char ASCII strings (mksecret-shaped).
- pushssl's exact client protocol both sides must speak: `sys/src/libc/9sys/pushssl.c` — `open("#D/ssl/clone")` → read decimal conv → `#D/ssl/%s/data` → `fprint(ctl, "fd %d")` + `secretin`/`secretout` + `alg`.
- Algorithm bytes: port `sys/src/libsec/port/rc4.c` + `sha1.c` donor-verbatim; r9 already carries an SHA1 block (`r9/port/src/hmacsha1.rs`, `SHA1DLEN` — the reuse RESEARCH blesses).
- Host-pin shape: round-trip the exact mksecret-shaped inputs through rc4_256+sha1 (cfg(test), the devcap `install_test_state` discipline); boot-profile evidence per 14-D-06 split.

---

### `r9/x86_64/src/exec.rs` — `#!` pre-pass (CPU-01)

**Analog:** donor `sys/src/9k/port/sysproc.c:282-324` (algorithm of record) + the file's own header-read path.

**Donor algorithm** (sysproc.c:282-305, verified this session):
```c
	 * If it's a #!, fill in progarg[] with info then read a new header
	 * from the file indicated by the #!.
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
and at sysproc.c:414: progarg lands BEFORE the caller's argv — so `listen → execl(tcp17010, "17010", "tcp", dir)` becomes `/bin/cpu -R <scriptpath> 17010 tcp <dir>`; `-R` hits `remoteside()` via ARGBEGIN and the trailing args are ignored.

**r9 integration site** (`exec.rs:64-82` `read_aout` — the header read the `#!` sniff precedes; `exec.rs:88-124` `install_image` — the argv/segment installer; `exec_replace` at exec.rs:384 is the exec-path entry; `MAX_ARGS: usize = 32` at exec.rs:58 caps args; argv stack builder documented at exec.rs:241-337). `parse_aout` (exec.rs:508+) errors loudly on bad magic — the `#!` pre-pass branches BEFORE it on `p[0]=='#' && p[1]=='!'` and re-execs the interpreter, re-running the whole read/parse against the interpreter's header (donor "read a new header from the file indicated by the #!").

---

### `r9/x86_64/src/syscall.rs` — WSTAT/FWSTAT/REMOVE DISPATCH arms (CPU-03)

**Analog:** own DISPATCH table; consumers define the semantics surface.

**The table and its loud-None discipline** (`syscall.rs:108-141` table; `syscall.rs:95-107` the doc naming today's gap; `syscall.rs:166-174` the loud arm):
```rust
/// ... REMOVE/WSTAT/FWSTAT/OSEEK/FAUTH stay loud-None (the measured
/// evidence ratified the loud gap, not a silent one). ...
static DISPATCH: [Option<Handler>; 55] = {
    let mut t: [Option<Handler>; 55] = [None; 55];
    t[BIND as usize] = Some(sys_bind);
    ...
    t
};
```
```rust
    let result = match DISPATCH.get(n as usize) {
        Some(Some(handler)) => handler(frame, proc),
        _ => {
            println!("syscall {n} unimplemented");
            proc.set_errstr("syscall not implemented");
            -1i64 as u64
        }
    };
```
Add `t[REMOVE as usize] = Some(sys_remove);` etc. — numbers from `sys/src/libc/9syscall/sys.h`: REMOVE=25, _WSTAT=26, _FWSTAT=27, WSTAT=44, FWSTAT=45. **Measure-first:** what exportfs actually exercises is `sys/src/cmd/exportfs/exportsrv.c` `Xremove`/`dirfwstat`/`dirwstat` (research cites 343-423); the 15-06 donor traces (rio has no Wstat; sam uses Remove only) bound the desktop path. `sys_remove` should route to per-device remove arms — `devcap.rs:359-367` already ports one (caphash) and names the wiring gap. Host-pin shape: the existing `unimplemented_number_is_loud` test (syscall.rs:1962-1966) is the pattern for arm-level pins.

---

### `r9/x86_64/src/main.rs` — `service=cpu` env seed (CPU-02)

**Analog:** own `seed_root_group` call (main.rs:520-530) — the kernel-seeded ROOT env group IS r9's plan9.ini-equivalent mechanism:
```rust
    port::devenv::set_group_hook(proc::current_env_group);
    let root_env = port::devenv::seed_root_group(&[
        (b"cputype", b"amd64"),
        (b"nobootprompt", b"embed"),
        (b"readparts", b"0"),
    ]);
    println!("devenv: seeded cputype=amd64 nobootprompt=embed readparts=0");
```
(seed definition: `r9/port/src/devenv.rs:269`.) RESEARCH Pattern 3's zero-fork favorite: add `("init", "/amd64/init -c")` for the cpu image — honored FIRST by boot's `execinit()` (`sys/src/9k/boot/boot.c:207-210`: `cmd = getenv("init"); if(cmd == nil){ sprint(cmdbuf, "/%s/init -%s%s", cputype, cpuflag ? "c" : "t", ...); }`). CAUTION (research): with the `init` override, `cpuflag` stays 0 so `doauth(0)`/`glenda()` still runs (hostowner=glenda, the 18-venue continuity — keep). Alternative seams: `sys/src/9k/boot/bootr9.c:28` `int cpuflag = 0;` (compile-time fork per image) or mb2 cmdline parse. Downstream consumer chain: `sys/src/cmd/init.c:30-46` (service default **"cpu"**, `-c`/`-t` flags, `setenv("#e/service", service)`, `iscpu`), `init.c` `cpustart()` → `execl("/bin/rc", "rc", "-c", "/rc/bin/cpurc", nil)` (verified at init.c:186-189 region). **Image-scoped only** (RESEARCH Pitfall 5: init defaults to cpu; the chain image reaches termrc only because bootr9 passes `-t` — the seed change must never rekey the chain/services/net images).

---

### `r9/x86_64/src/platform/q35/timer.rs` — tick_check wiring (D-10)

**Analog:** `r9/aarch64/src/platform/virt/timer.rs:129-130` — the exact call, in its dev_ip cfg arm:
```rust
        // The net-deadline sweep (08-04, D-05's timer leg): the
        // poll_at-armed one-shot deadline rides the existing 1 ms tick —
        // on expiry the devip waiters wake and drive the stack from
        // their WOKEN SYSCALL context (poll NEVER runs in IRQ/tick
        // context — the no-alloc-in-IRQ and mcslock-LEAF invariants).
        #[cfg(dev_ip)]
        port::devip::stack::tick_check(n + 1);
```
Wire site: q35 `tick()` (timer.rs:199-240) — its `#[cfg(dev_exec)]` block at 221-225 already runs `tick_timeouts` + `preempt`; the `tick_check` call joins there (cfg-keyed per the aarch64 precedent). Evidence per 14-D-06: host pin + boot profile (long-lived-park timer leg).

---

### `sys/src/cmd/unix/drawterm/cpu.c` — `-G` + `-O` local additions (D-04, DESK-01)

**Analog:** the file's own ARGBEGIN — neither `-G` nor `-O` exists in the vendor (RESEARCH Pitfall 2; both are Harvey additions, D-01/D-02 posture: we own the source).

**Env reads to scrub around** (`cpu.c:115-122` — the `$cpu`/`$auth` hijack surface, RESEARCH Pitfall 3):
```c
	user = getenv("USER");
	secstoreserver = nil;
	authserver = getenv("auth");
	if(authserver == nil)
		authserver = "auth";
	system = getenv("cpu");
	if(system == nil)
		system = "cpu";
```

**Flag-with-arg precedent** (`cpu.c:123-137` — `'e'` is the closest in-file analog for `-O <spec>`; note `ealgs` default IS D-15's encrypted default):
```c
	static char	*ealgs = "rc4_256 sha1";   /* cpu.c:35 */
	ARGBEGIN{
	case 'a':
		authserver = EARGF(usage());
		break;
	...
	case 'e':
		ealgs = EARGF(usage());
		if(*ealgs == 0 || strcmp(ealgs, "clear") == 0)
			ealgs = nil;
		break;
```
`'C'` (cpu.c:138-146) is the multi-arg-collect precedent. `-G` (no-window stdio, D-04) branches at the window/exportfs seam: `exportfs(data, msgsize)` at cpu.c:213 is where the GUI session launches — the `-G` arm instead serves the session over the invoking terminal's stdio (real terminal semantics, no offscreen buffer). The dial line: `rexcall` at cpu.c:250 `na = netmkaddr(host, "tcp", "17010");` — the 3-part `-a 'tcp!127.0.0.1!5670'` form passes through netmkaddr verbatim (`sys/src/cmd/unix/drawterm/libc/netmkaddr.c:33-36` — an address that already has a service is used as-is; zero authdial patching needed). Password path is `askuser`/`readcons` (cpu.c:461-540) — interactive raw reads, so the gate MUST drive a pty (16-05), never piped stdin.

**Build surface** (`sys/src/cmd/unix/drawterm/Make.unix`, full 23 lines — X11=/usr/X11R6, `CFLAGS ... -I$(X11)/include`, `LDADD=-L$(X11)/lib64 -L$(X11)/lib -lX11`, `GUI=x11`, libmachdep.a builds `posix-$$arch` by uname): the nix recipe overrides CC/X11 env-style. Note `gui-x11/x11.c:26` includes `<X11/IntrinsicP.h>` (libXt headers needed) though Make.unix links only `-lX11` (RESEARCH verified). `gui-osx/` is Carbon/-m32 and is NOT buildable on aarch64-darwin (OQ-1 — see No Analog Found).

---

### `nix/drawterm.nix` (or flake-inline) — the drawterm package (D-02)

**Analog A — stdenv.mkDerivation over committed source, no fetch** (`flake.nix:292-302`, `hostEchoX64Pkg` — the "committed repo source, stdenv build, no fetched binaries" precedent):
```nix
          hostEchoX64Pkg = pkgs.stdenv.mkDerivation {
            pname = "host-echo";
            version = "16.05";
            dontUnpack = true;
            buildPhase = ''
              cc -O2 -Wall -o host-echo ${./nix/host-echo.c}
            '';
            installPhase = ''
              install -Dm755 host-echo $out/bin/host-echo
            '';
          };
```
For drawterm: `src = ./sys/src/cmd/unix/drawterm` (or `dontUnpack` + `cd`), `buildInputs = [ xorg.libX11 libXt ]`, `buildPhase = make CONF=unix`-shape with X11 paths pointed at nix store, `installPhase` installs `drawterm`. Hermetic, no network fetch, no patchPhase (D-02).

**Analog B — vendored-at-pin host build PATH discipline** (`nix/toolchain.sh:54-55`):
```sh
# vendored 9-cc subset (sys/src/cmd/unix/9cc) — builds iar, installs as ar
(cd "$HARVEY/sys/src/cmd/unix/9cc" && PATH="$_WRAP_PATH" sh BUILD)
```
—the "compile the in-tree vendor with host tools, keep host binutils resolvable" posture (A5's `-ggdb`/`-Wno-missing-braces` are benign; watch bare `ar`/`nm`/`as`/`ranlib` resolution — Make.unix sets them, nix's stdenv provides them).

---

### `nix/drawterm-x64.sh` — the wrapper app (DESK-01)

**Analog:** `nix/boot-r9-x64.sh`'s single-accepted-form arg parse (lines 147-156) + the flake `mkScriptApp` wrapper (flake.nix:26-39 — PLAN9 LAST on PATH). Wrapper duties per DESK-01/D-15: unset/scrub `$cpu` `$auth` `$PASS` (Pitfall 3), pin connection params via the new `-O` (argv the single source), print the exact encrypted-default connect line (`-e clear` documented fallback only). The arg-parse shape to copy:
```sh
NET_FLAG=
for ARG in "$@"; do
	case "$ARG" in
	--net=*)
		[ -z "$NET_FLAG" ] || err "duplicate --net ('$ARG' after '$NET_FLAG') — one net mode per boot"
		NET_FLAG="$ARG"
		;;
	*) err "unknown argument '$ARG' — the accepted form is --net=<mode> ..." ;;
	esac
done
```
(no silent downgrade: unknown modes are usage errors naming the valid set — the OQ-4 recommendation for `--system=cpu` rides this precedent).

---

### `flake.nix` — new apps (drawterm, -G gate, full-chain smoke, boot full-system mode)

**Analog:** the `authSmokeR9X64Pkg` block (flake.nix:385-390) and its app registration (flake.nix:440-443) — app-per-gate:
```nix
          authSmokeR9X64Pkg = mkScriptApp pkgs "harvey-boot-r9-auth-x64" ./nix/smoke-r9-auth-x64.sh ''
            export R9_KERNEL_X64_PKG="${r9.packages.${system}.kernel-x64}"
            export R9_ISO_BUILDER="${r9.packages.${system}.iso-builder}/bin/r9-iso-builder"
            export R9_NETDEV_EMITTER="${r9.packages.${system}.netdev-args}/bin/r9-netdev-args"
          '' ([ pkgs.plan9port pkgs.bash pkgs.qemu pkgs.git pkgs.expect pkgs.iproute2 pkgs.procps pkgs.stdenv.cc ]
            ++ pkgs.lib.optionals isLinux [ pkgs.util-linux pkgs.gnused pkgs.mount pkgs.binutils ]);
```
Rules that bind (flake.nix:409-444 comments): x64 apps ride the `hasAttr all [kernel-x64, iso-builder, netdev-args]` optionalAttrs block — NEVER widen that list ahead of a pin advance; existing apps stay byte-untouched (app-per-gate, one-image-per-gate). The drawterm app is linux-only (D-06 — its own `isLinux` optionalAttrs like `net-doctor` at flake.nix:450-452, consuming no r9 attrs). The -G gate app carries `expect` + the creds-file env (see D-12 pattern below). Note: the package (not just a script app) needs a `packages` output — `harness` (flake.nix:49-58, `rustPlatform.buildRustPackage`) is the packages-output precedent for slotting `drawterm` beside it.

---

### `build/x86_64/mkimage.sh` §15 — the seventh contract (cpu image)

**Analog:** §14 AUTH contract verbatim (mkimage.sh:1250-1664) — the full step ladder to re-key:
- **14a** (1279-1298): assert every staged member's tier entry exists FIRST (`"$GREP" -qE "^$e([[:space:]]|#|$)" "$AUTH_T2" || err ...` — data/bake divergence reds at bake, never in guest). §15 adds `cmd/cpu` (+ `cmd/aux:listen` if the aux/listen arm is chosen) to the asserted set.
- **14b** (1300-1370): the h* staging set COMPUTED from tier data (file/dir/container-leaf/multi-word-TARG resolver; duplicate h* names are a bake error).
- **14d** (1392-1402): host-cc invocation pattern (authdomain-gen at bake — D-12's creds emission rides the same generator run).
- **14e** (1404-1409): skeleton copy `(cd "$ROOT_TREE" && tar -cf - .) | (cd "$STAGE8" && tar -xf -)`.
- **14f** (1416-1453): domain staging + `-V` pairing assert over staged copies (rm-before-cp discipline; AUTHBAKE_DOCTOR teeth seam).
- **14f2/14f2b** (1455-1482): the feed + host-side copy — D-12's direct precedent (below).
- **14f3** (1500-1535): /lib/ndb/local composed from `local.template` (token substitution both-ways asserted; `printf 'tcp=ticket port=567\n' >> ...` service-row append — the `tcp=ncpu port=17010` row rides this exact shape if the alpha dial is chosen).
- **14j** (1603-1609): sweep at /lib + termrc OVERRIDE staged as `rc/bin/termrc` (the 11l rule) — §15 stages cpurc instead (termrc vs cpurc selection is planner discretion; note init sources termrc on the terminal path and `/rc/bin/cpurc` on the cpu path — the override target depends on the CPU-02 seam choice).
- **14k** (1611-1633): pre-bake manifest PRINTED (gate greps these lines).
- **14n/14o** (1647-1662): KERNDATE determinism touch + mkpaqfs per-file 4 MiB cap assert + header/trailer magic checks (`529ab12b`/`6b46e688`) — every new baked file class rides the cap assert (mkpaqfs silent-drop hazard).
- Payload: the DESK-03 fonts ride via `stage_payload "$STAGE" <label>` (section P, mkimage.sh:643-668 — DECLARE-by-invocation; the services contract at 939 is the declaring precedent).

---

### `build/x86_64/authdomain-gen.c` — creds emission (D-12)

**Analog:** own `feed` artifact — the exact writefile lineage to extend (authdomain-gen.c:587 inside `generate()`, declared at 464):
```c
	/* feed: the last user's password + newline — the 18-04 bake's
	 * /boot/authfeed.txt (the 18-05 prompt feeds). ... */
	writefile(dir, "feed", feed, strlen(feed), 0444);
```
D-12's `out/auth-creds.txt` (demo user + password) is a sibling artifact from the same run: emit via the same `writefile(dir, "creds", ...)` helper (authdomain-gen.c:255 — short-write sysfatal discipline), gitignored output only, 18-D-01's ephemeral-DEMO contract untouched, nothing secret-shaped in git. Host-side copy precedent is mkimage 14f2b (1481-1482): `rm -f "$OUT/authfeed.txt"; cp "$AUTHDOM_STAGE/feed" "$OUT/authfeed.txt"` — rm-before-cp (the generator emits read-only 0444; a stale copy would refuse overwrite).

---

### `build/x86_64/root-tree/rc/bin/cpurc` — NEW (or §15-staged override)

**Analogs (three-layer):**
1. **Shape:** `build/x86_64/auth-termrc` (full file) — the termrc-override pattern (`rc /lib/<sweep>.rc r9` in a child + `exec rc -i`; "a failed pairing check SKIPS every daemon leg (fail-loud, never a silent interactive wedge)").
2. **Content style:** `build/x86_64/auth-sweep.rc` — the flat status-ladder rc dialect (76-85 documents the measured 15-02/18-05 rules verbatim: braced `if not {` bodies kill sourced files; bare `=` ends words — `nvram=/adm/nvram` alone on its line is the proven form; no fd-dup in nested blocks; output-redirect open failures are fatal). Key legs to compose cpurc from: the loopback bring-up (202-232, `bind -a '#I' /net` + clone-read + per-write ctl opens), the listener leg (259-294 — `listen1 -t 'tcp!*!567' /amd64/bin/authsrv -d &` + `svc_waitport` bounded spin), factotum key write (317-339 — `key proto=p9sk1 user=... dom=... !password='^$authpw > /mnt/factotum/ctl`, the A3 substrate), the terminal verdict ladder (486-503).
3. **Donor content:** `sys/src/9k/k10/root/k10root.rc` (114 lines — NOT `cfg/example/cpurc`, which is a 12-line stub). The trusted-listener block (97-110):
```rc
sysname=cpu-$ip(1)
bind /boot/rc/bin/service /bin/service
if(test -d /mnt/factotum)
	aux/listen -q tcp
if not
	aux/listen -t /bin/service tcp
```
RESEARCH Pattern 1 + OQ-3: this phase's cpurc runs factotum (the 18 suite), so the `-t` trusted arm is the shape (`listen1 -t` is the 18-04 proven precedent); ordering discipline: pairing → keyfs → authsrv → listen (fail-loud, a bad bake never wedges behind an interactive prompt). Listener mechanics donor: `sys/src/cmd/aux/listen.c:119-120` (`if(!servdir && !trustdir) servdir = "/bin/service"` — with `-t <dir>` only the trustdir is scanned) and `listen.c:541` `execl(s->prog, s->prog, s->serv, proto, dir, nil)` (the blind exec the kernel `#!` pre-pass completes).

**Service file** — verbatim donor `sys/src/9k/k10/root/tcp17010` (entire file): `#!/bin/cpu -N -R` — this phase's CONTEXT pins `#!/bin/cpu -R` (authenticated, no `-N`). Stage via the §15 contract (a tiny text file stages like auth-termrc, never a tier).

---

### `build/x86_64/drawterm-gate.expect` — NEW `-G` gate driver (DESK-06, D-14)

**Analog:** `build/x86_64/smoke-r9-auth.expect` — the complete chassis (515 lines). Copy verbatim:
- env contract loop (100-105): `foreach v {R9_ISO R9_SMOKE_LOG R9_NETDEV_ARGV R9_DEMO_PW} { ... exit 2 }` — the -G gate adds its creds-file env (D-12).
- spawn + log_file + teardown (144-162): spawn qemu with split `R9_NETDEV_ARGV` tokens + `-name $run_tag`; `cleanup` kills the process group on EVERY exit path.
- `stage`/`feedstage` procs (168-189): presence-only assertions, each names its failure; feedstage matches prompt text then sends — the pty discipline for drawterm's `askuser` password prompts (16-05: raw single-char reads, echo-off; NEVER piped stdin).
- `hostfwd_stage` (209-223): host-side `exec ss -tln` observation of the 127.0.0.1:17010/5670 loopback binds while the guest runs (port token anchored `127\\.0\\.0\\.1:${port}( |$)`).
- `expect_before` fast-fail (233-244) on panic markers + the sweep's FAIL marker.
- whole-file post-passes (382-409): stop log_file, re-read capture, never-appears contracts, DEMO-label demands.
- redaction LAST (503-511): `string map [list $pw "<demo-pw-redacted>"]` rewrite after asserts ran over the true capture (T-18-13).
Differences for -G: the gate spawns/steers drawterm (pty) not just qemu; the login → shell-marker flow is the DESK-06 contract; required venue is local KVM green-twice (D-14), hosted CI best-effort.

---

### `nix/<-G gate driver>.sh` — NEW gate flake-app script

**Analog:** `nix/smoke-r9-auth-x64.sh` — the composed pipeline driver. Copy: contract checks (110-129), accel no-silent-downgrade (133-143), emitter capture `NETDEV_ARGV=$("$R9_NETDEV_EMITTER" --mode slirp)` (156-159), KERNDATE provenance (170-176), bake + artifact assert (191-194), **creds read post-bake** (204-208):
```sh
R9_DEMO_PW_FILE="$HARVEY/build/x86_64/out/authfeed.txt"
[ -s "$R9_DEMO_PW_FILE" ] || err "... missing/empty after the bake (mkimage 14f2b's host-side feed copy ...)"
R9_DEMO_PW=$(tr -d '\n\r' < "$R9_DEMO_PW_FILE")
[ -n "$R9_DEMO_PW" ] || err "the feed copy carried no password line"
export R9_DEMO_PW
```
— D-12's `out/auth-creds.txt` (user + password) reads identically. Per-run tag + survivor sweep (220-234): `R9_RUN_TAG="r9x64auth-$$-$(date +%s)"` with a UNIQUE prefix per gate (the sibling-gate no-cross-kill rule; the -G gate needs its own prefix AND must sweep drawterm's process group too — drawterm is a second spawned subtree). ISO compose via iso-builder (236-240). Expect launch with RC capture (258-261).

---

### `nix/boot-r9-x64.sh` — full-system mode (INT-05)

**Analog:** the file's own seams. Image selection (steps 2/3, lines 234-237):
```sh
echo "== step 2/3: bake bootfs.paq + bootfs-chain.paq (build/x86_64/mkimage.sh)"
sh "$HARVEY/build/x86_64/mkimage.sh" || err "bootfs bake failed"
CHAIN_PAQ="$HARVEY/build/x86_64/out/bootfs-chain.paq"
[ -s "$CHAIN_PAQ" ] || err "$CHAIN_PAQ missing/empty after the bake (mkimage's chain section)"
```
— the cpu mode selects `out/bootfs-cpu.paq` (§15's artifact) at this seam. Mode flag: extend the `--net=` single-accepted-form arg parse (147-173) with `--system=cpu|terminal`-style (OQ-4: keep the chain path untouched for existing gates; unknown modes are usage errors naming the valid set). The ONE command then boots net + auth + cpu listener together off the cpu image.

---

### `build/x86_64/tiers/tier2-services.txt` — `cmd/cpu` entry

**Analog:** own `cmd/aux:listen1` line (tier2-services.txt:53) — append-only rule from the file header: "APPEND-ONLY: growth is appending a line, after the entry's per-tier gate passes. Never reorder, edit, or delete." `cmd/cpu` is a mkone single-file service binary → plain `cmd/cpu` entry form (grammar at header lines 17-25). Consequence (RESEARCH Pitfall 6): the tier sweep set changes → re-run `build/mm/build-9k10t1.sh` to re-pin the gitignored witness manifest; per-entry fresh-build rules apply.

---

### Docs + evidence

- `build/x86_64/README.md` — new sections follow the existing drift-notes home (drawterm usage, D-05 SSH-tunnel darwin recipe verbatim from CONTEXT, cpu-service sections, D-06 darwin documented-manual build recipe, D-07 session recipe, `-e clear` fallback, the `-O`/`-G` flag table).
- `.planning/phases/19-*/19-EVIDENCE.md` — D-13 rows: one screenshot per leg + signed human-acceptance row (date, venue, r9 SHA, accepter) — the 17-08 maintainer-ratification record style. Venue honesty: the darwin recipe must be exercised for real before the row is signed.

## Shared Patterns

### Expect gate chassis (all composed gates)
**Source:** `build/x86_64/smoke-r9-auth.expect` (procs at 168-189, 209-223; expect_before 233-244; post-pass 382-409; redaction 503-511)
**Apply to:** `drawterm-gate.expect`, the full-chain smoke extension.
Presence-based timing-tolerant assertions only; CRLF-tolerant substrings; timeouts TCG-sized (600s); markers are boot-flow OUTPUT (nothing typed except sanctioned feedstage surfaces); log_user stays ON (expect 5.45 measured: log_file writes nothing when log_user is 0).

### Flake app-per-gate + never-widen gate list
**Source:** `flake.nix:385-390` (pkg block), `flake.nix:421-444` (hasAttr gating + registration)
**Apply to:** drawterm app (linux-only, own optionalAttrs — the net-doctor precedent at 450-452), -G gate app, smoke extension. Existing apps byte-untouched; the `[ "kernel-x64" "iso-builder" "netdev-args" ]` attr list NEVER widens ahead of a pin advance.

### Bake contract ladder (§14a→§14o)
**Source:** `build/x86_64/mkimage.sh:1250-1664`
**Apply to:** §15. Tier-entry asserts before staging → computed h* set → host-cc generator at bake → skeleton tar → domain staging + pairing assert → feed/creds copies → ndb compose → mount-point dirs → chain binaries → h* payload → sweep/cpurc staging → manifest print → absence asserts (no /boot/factotum, no /sys/log — Pitfall 6) → rcmain cmp → KERNDATE touch + 4 MiB cap + mkpaqfs → magic checks.

### rc dialect discipline (cpurc + any in-guest fixture)
**Source:** `build/x86_64/auth-sweep.rc:76-91` (the measured 15-02/18-05 rules + the `$B/h*` vocabulary rule)
**Apply to:** cpurc, any -G-gate in-guest dial fixtures. Flat status ladders; same-line `if(...)`/`if not cmd`; `nvram=...` whole-word assignment alone on its line; single-quoted `=`-bearing argv; NO output-redirect open failures in sourced files; sourced-not-typed.

### Creds/feed lineage (D-12)
**Source:** mkimage.sh 14f2/14f2b (1455-1482) + authdomain-gen.c:587 + smoke-r9-auth-x64.sh:204-208 + smoke-r9-auth.expect redaction 503-511
**Apply to:** the creds emission (generator), host-side copy (bake), gate read (driver), capture redaction (expect). rm-before-cp; read-only emit (0444); gitignored `$OUT` only; the gate never prints it; redact retained captures after asserts.

### Loopback literal dials (18-05 idiom)
**Source:** auth-sweep.rc leg 9d `-s 127.0.0.1` pin (444-474, with the recorded `$attr` cs-translation failure)
**Apply to:** the -G gate's dial line and any in-guest dial fixture — literal `127.0.0.1`-style 3-part addresses (`tcp!127.0.0.1!17010`), never `$tcp!$cpu!...` variable forms (cs translation reds through r9 kernel namec). Alpha service names need an ndb row (the 14f3 `tcp=ticket port=567` precedent).

### Loud-None → named arm (kernel syscall work)
**Source:** `r9/x86_64/src/syscall.rs:95-107, 166-174`
**Apply to:** REMOVE/_WSTAT/_FWSTAT/WSTAT/FWSTAT. A named handler replaces the None slot; the loud arm stays for everything else; host pins follow `unimplemented_number_is_loud` (syscall.rs:1962-1966).

### Donor-verbatim port posture (kernel device work)
**Source:** `r9/port/src/devcap.rs:1-37` (module doc citing donor + "port the donor, never the phrase"), `devcap.rs:128-134` (install_test_state pin discipline), `devcap.rs:141-148` (formatted-error transport)
**Apply to:** devssl.rs. Error strings carry verbatim where userspace matches on them (`must set fd before algorithm`, `Ebadarg` on unknown verbs).

### Push-before-pin / FOD re-wire / pin-advance-own-commit
**Source:** carried decisions (CONTEXT "Carried forward"); recently-proven task shapes in `.planning/phases/18-in-guest-p9sk1-auth-domain/18-0*-PLAN.md` (boot-marker greps, nix-run smoke apps, tier-file edits, FOD re-wire-then-pin choreography)
**Apply to:** all r9 work (branch `main`, push before the flake.lock pin advances); ONE FOD re-wire pass after the kernel-byte fix set (`#!`, WSTAT/FWSTAT/REMOVE, devssl, AcceptPool, tick_check); pin advance is its OWN commit + fresh hosted CI run (INT-06), never bundled.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `sys/src/cmd/unix/drawterm/gui-cocoa/` (or equivalent 64-bit Cocoa backend, OQ-1) | vendor GUI backend | streaming | No 64-bit Cocoa backend exists in-tree: `gui-osx/` is Carbon/QuickDraw `-m32` (unbuildable on aarch64-darwin); `Make.osx-x11` contradicts D-03. The port target is 9front's `gui-cocoa` class (external, MIT-compatible) brought in as a Harvey commit; the in-tree interface contract it must implement is the vendor's screen interface as `gui-x11/` consumes it. **OQ-1 RESOLVED (2026-11-14, amended D-03): not a deliverable this phase** — darwin is `-G` text-only (19-06 gui-none; 19-10 deferral + reopener); this row stands as the recorded port target for the named reopener only, not a planned file. |
| `-G` / `-O` flag semantics | vendor client surface | request-response | Neither flag exists in the vendor (grepped cpu.c/cpu-bl.c/README; ARGBEGIN cases `a c d e C k r s u`). Both are local additions; only their in-file precedents exist (`'e'` at cpu.c:133-137 for arg-carrying flags; `'C'` at 138-146). OQ-2 pins `-O`'s UX contract at plan time. |

## Metadata

**Analog search scope:** `/home/req/code/harvey/{flake.nix,nix/,build/x86_64/,sys/src/cmd/unix/drawterm/,sys/src/cmd/,sys/src/9k/,sys/src/libc/9sys/,sys/src/libsec/port/,cfg/}` + `/home/req/code/r9/{port/src/,x86_64/src/,aarch64/src/platform/}` + `.planning/phases/18-*/`
**Files scanned:** ~45 (30 Harvey analogs + 10 r9 analogs + 5 donors verified tracked; excerpts read this session)
**Pattern extraction date:** 2026-11-14
**Tracked-source gate:** all analog paths pass `git ls-files` (checked in both repos); zero mirror paths emitted.
