# Phase 5: Shell Boot + One-Command Integration - Pattern Map

**Mapped:** 2026-11-02
**Files analyzed:** 24 (11 Harvey-side, 13 r9-side)
**Analogs found:** 24 / 24 (every file has at least one analog; r9 kernel files have two-tier analogs: r9 idiom + 9k C semantic reference)

**Repo key:** paths are prefixed `harvey/` (= repo root, this repo) or `r9/` (= sibling repo `../r9`). All analog paths verified git-tracked via `git ls-files` in their owning repo (2026-11-02).

## File Classification

### Harvey repo (userland, image, composition, CI)

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `flake.nix` (modify: +`inputs.r9`, +`boot-r9` app, +smoke app) | config | batch (nix eval) | `flake.nix` itself (mkScriptApp, lines 12-25, 37-72) | exact |
| `nix/boot-r9.sh` (new — compose: userland build → bake → qemu launch) | utility (host script) | batch → spawn/streaming | `nix/vm-build.sh` (lima flow) + `nix/build.sh` (linux flow) | role-match |
| `nix/smoke-r9.sh` + `build/aarch64/smoke-r9.expect` (new — expect driver) | test | event-driven (expect/stdio) | `build/build.bash` (lines 27-79) | exact |
| `build/aarch64/mkimage.sh` (new — root-tree bake via mkpaqfs) | utility (host script) | file-I/O, batch | `build/aarch64/stage-build.sh` + `mkpaqfs-host.sh` | exact |
| `build/aarch64/root-tree/` skeleton (new: `rc/bin/termrc`, `lib/namespace`, `usr/glenda/lib/profile`, empty mount-point dirs) | config (data files) | file-I/O (image contents) | `sys/src/9k/k10/root/k10root.proto` + `k10root.namespace` + `lib/namespace` + `cfg/example/termrc` | exact |
| `sys/src/9k/boot/` additive aarch64 boot build (new mkfile or mk fragment + hand-written bootconf C) | config (build) | batch (mk) | `sys/src/9k/mk/bootmkfile` + `sys/src/9k/mk/parse` (mkbootconf) + `sys/src/cmd/paqfs/mkfile` | exact |
| `sys/src/9k/boot/embed.c` (reuse as-is; at most an additive sibling method file) | service (boot method) | request-response (9P over pipe) | `sys/src/9k/boot/embed.c` itself (the D-04 precedent) | exact |
| `.github/workflows/r9-shell.yml` (new — linux CI leg) | config (CI) | batch | `.github/workflows/harvey.yml` (shape only; nix-based tooling is new) | role-match |
| `build/REPRODUCIBILITY.md` (update for INT-03) | doc | — | itself (lines 1-50, KERNDATE policy shape) | exact |

### r9 repo (kernel — the big stream)

| New/Modified File | Role | Data Flow | Idiom Analog (r9) | Semantic Reference (harvey 9k C) |
|-------------------|------|-----------|-------------------|----------------------------------|
| `port/src/namec.rs` / `namespace.rs` (new — name resolution, union walk, mount table) | service (kernel subsystem) | request-response (path→Chan) | `port/src/devroot.rs` (walk, lines 87-105) | `sys/src/9k/port/chan.c` (1693 lines), `sys/src/9k/port/dev.c` (461) |
| `port/src/pgrp.rs` (new — per-process namespace/env groups) | model (kernel state) | CRUD (mount table) | `port/src/dat.rs` (Chan, lines 40-78) | `sys/src/9k/port/pgrp.c` (322 lines) |
| `port/src/devmnt.rs` (new — kernel 9P client) | service (device + RPC client) | request-response (T/R messages over fd) | `port/src/devproc.rs` (free-fn device idiom) | `sys/src/9k/port/devmnt.c` (1206 lines) |
| `port/src/devpipe.rs` (new — shared-queue pipes) | service (device) | streaming (queue, Arc-shared) | `port/src/devproc.rs`; `port/src/dat.rs` Chan::dup caveat (lines 63-77) | `sys/src/9k/port/devpipe.c` (416) + `qio.c` |
| `port/src/devdup.rs` (new — per-fd files `#d/0`…) | service (device) | request-response | `port/src/devproc.rs` (walk/open shape) | `sys/src/9k/port/devdup.c` (145 lines) |
| `port/src/devsrv.rs` (new — `#s` service registry) | service (device) | CRUD | `port/src/devproc.rs` | `sys/src/9k/port/devsrv.c` (360 lines) |
| `port/src/devenv.rs` (new — per-pgrp env) | service (device) | CRUD (per-group kv) | `port/src/devproc.rs` + `mcslock.rs` static-publish idiom | `sys/src/9k/port/devenv.c` (441 lines) |
| `port/src/devcons.rs` (extend — read path, cooked, consctl, DEL note) | service (device) | streaming + event-driven (UART RX IRQ → note) | itself (lines 1-113) + `platform/virt/pl011.rs` | `sys/src/9k/port/devcons.c` (1405 lines) |
| `port/src/devproc.rs` (extend — status/args/ctl + dir reads) | service (device) | CRUD | itself (lines 1-143) | `sys/src/9k/port/devproc.c` (1667 lines) |
| `aarch64/src/syscall.rs` (extend — measured syscall set) | middleware (dispatch) | request-response (SVC) | itself (DISPATCH table lines 74-95; sys_open lines 374-440) | `sys/src/libc/9syscall/sys.h` (numbers) |
| `aarch64/src/proc.rs` (extend — pgrp/env/fd-group fields, sizing) | model | — | itself (Proc struct lines 167-229; MAX_PROCS/MAX_FDS lines 52-54) | — |
| `flake.nix` (extend — `packages.kernel-virt`) | config | batch | itself (lines 44-93) + `harvey/flake.nix` packages block (lines 28-35) | — |
| `xtask/src/main.rs` + `xtask/src/config.rs` (remove/invert initrd + `../harvey` glue) | utility | — | itself (config.rs fdtput recipe lines 385-434 — the shape Harvey's launcher adopts) | — |
| `HowItWorks.md` (update — namespace/devmnt/devices are material) | doc | — | itself (header mandate lines 1-5) | — |

## Pattern Assignments

### `flake.nix` — Harvey (modify; config, batch)

**Analog:** `flake.nix` (this repo, lines 12-25, 37-72)

**mkScriptApp pattern** (lines 12-25) — every app is a `writeShellApplication` with the PLAN9-last header:

```nix
mkScriptApp = pkgs: name: file: extraHeader: runtimeInputs:
  pkgs.writeShellApplication {
    inherit name runtimeInputs;
    text = ''
      set -euo pipefail
      export PLAN9="${pkgs.plan9port}/plan9"
      # plan9 bin LAST per BUILD_ON_UNIX — plan9port ships a 9p `mount`
      # (mount addr mtpt) which must not shadow util-linux mount in the
      # namespace build
      export PATH="$PATH:$PLAN9/bin"
      ${extraHeader}
      ${builtins.readFile file}
    '';
  };
```

**App wiring pattern** (lines 37-72) — per-system let block, `mkApp`, platform-conditional script selection (the linux/darwin split the compose app needs):

```nix
apps = forAllSystems (system:
  let
    pkgs = pkgsFor system;
    isLinux = pkgs.stdenv.hostPlatform.isLinux;
    mkApp = pkg: { type = "app"; program = "${pkg}/bin/${pkg.name}"; };
    …
    buildPkg = if isLinux
      then mkScriptApp pkgs "harvey-build" ./nix/build.sh ''
        export GNU_SED="${pkgs.gnused}/bin/sed"
      '' [ pkgs.plan9port pkgs.bash pkgs.util-linux pkgs.gnused pkgs.mount pkgs.binutils ]
      else mkScriptApp pkgs "harvey-build" ./nix/vm-build.sh ''
        export LIMA_YAML="${./nix/lima-harvey.yaml}"
      '' [ pkgs.lima pkgs.openssh pkgs.rsync ];
  in {
    setup = mkApp setupPkg;
    …
  });
```

**New elements this phase adds:** `inputs.r9.url = "github:requaos/r9"` (flake.lock-pinned, per D-10/INT-04), then `r9.packages.${system}.kernel-virt` referenced inside the compose app's script. Script files live in `nix/` (`builtins.readFile file` discipline — no inline heredocs in the flake).

---

### `nix/boot-r9.sh` — Harvey compose app script (new; utility, batch → streaming)

**Analogs:** `nix/vm-build.sh` (darwin/lima transport) + `nix/build.sh` (linux native namespace) + `r9/xtask/src/config.rs` (qemu argv + DTB patch recipe, lines 360-434)

**Platform split:** the flake app picks the script variant per `isLinux` (pattern above); the script itself follows the established host-driver discipline — `HARVEY="${HARVEY:-$PWD"}`, checkout sanity check, then per-platform transport.

**KERNDATE-style provenance header** (from `nix/build.sh` lines 40-51) — the compose app's build steps honor the same derivation:

```sh
if [ -z "${KERNDATE:-}" ]; then
	KERNDATE=$(cd "$HARVEY" && git log -1 --format=%ct 2>/dev/null) || KERNDATE=
fi
if ! [[ "$KERNDATE" =~ ^[0-9]+$ ]]; then
	KERNDATE=$(date -n 2>/dev/null) || KERNDATE=
fi
```

**qemu argv assembly** (from `r9/xtask/src/config.rs` lines 360-371 — the shape Harvey's launcher reimplements per D-11; values from `r9/aarch64/lib/config_virtexec.toml` lines 21-43):

```
qemu-system-aarch64 -M virt,gic-version=2 -cpu cortex-a72 -smp 1 \
  -m 1024M -accel tcg -nographic \
  -kernel <r9-elf> -dtb <patched-copy> \
  -device loader,file=<bootfs.paq>,addr=0x48000000
```

**fdtput /chosen patch recipe** (from `r9/xtask/src/config.rs` lines 415-432 — two 32-bit BE cells, high word first, because dtc has no 8-byte type):

```sh
end=$((0x48000000 + size))
for pair in "linux,initrd-start 0x48000000" "linux,initrd-end $end"; do
  # hi = value >> 32, lo = value & 0xffffffff
  fdtput -t x "$patched_dtb" /chosen "$prop" "$hi" "$lo"
done
```

---

### `nix/smoke-r9.sh` + `build/aarch64/smoke-r9.expect` — Harvey smoke (new; test, event-driven)

**Analog:** `build/build.bash` (lines 27-79) — the heredoc-expect lineage D-12 names; this phase writes the expect body as a **committed file** (not a heredoc) spawned by the flake app.

**Core expect pattern** (lines 27-40):

```expect
spawn qemu-system-i386 $FLAG_KVM -nographic …
expect -exact "Selection:"
send "2\n"
expect -exact "Plan 9"
```

**Timeout sizing** (lines 33, 46): `expect -timeout 600 …` / `set timeout 2400` — TCG on GH ubuntu runners is minutes-scale; size generously (RESEARCH Pitfall 11).

**Phase-5 specifics to add** (no in-repo analog — from CONTEXT D-09/D-16 and RESEARCH Patterns 6 + Pitfalls 10-11):
- staged ordered assertions: banner → boot prints → `init: starting /bin/rc` → `% `, then the command sweep (`echo` marker, `ls /bin` subset names, `ps`, `cat` of a known file) — **presence assertions only, never interleaving order** (Phase 4 D-13)
- markers must tolerate the kernel console's `\n`→`\r\n` cooking (`r9/port/src/devcons.rs:105-113`)
- qemu hygiene per D-16: fresh log per run (`rm -f` before launch), process-group kill, bracket-fixed `pkill -f '[q]emu-system-aarch64'` on darwin, EXIT-trap kill

---

### `build/aarch64/mkimage.sh` — root-tree bake (new; utility, file-I/O)

**Analogs:** `build/aarch64/stage-build.sh` (whole file — the host-driver shape) + `build/aarch64/mkpaqfs-host.sh` (the mkpaqfs producer)

**Host-driver skeleton** (from `stage-build.sh` lines 19-56, 79-121) — env defaults, lima probe, guest-script-via-/tmp discipline, rsync-back, magic assertion:

```sh
set -eu
HARVEY="${HARVEY:-$PWD}"
VM="${HARVEY_VM:-harvey}"
GUEST_DIR="${HARVEY_GUEST_DIR:-harvey-build}"
LOG="${STAGE_BUILD_LOG:-$HARVEY/.build/stage-build.log}"
…
TMPD=$(mktemp -d "${TMPDIR:-/tmp}/stage-build.XXXXXX")
cleanup() { rm -rf "$TMPD"; }
trap cleanup EXIT
# guest script: single committed file regenerated per run, rsynced to
# guest /tmp, executed via vm-run.sh (nsrun.sh namespace wrap)
```

**Marker discipline** (lines 116, 130): guest prints `STAGE-BUILD-OK` / host greps for it; staged binaries asserted by R_MAGIC `od -An -tx1 -N4` = `00008c47`. The bake script's analogue: assert `boot`, `init`, `paqfs` magics + mkpaqfs exit + image size sanity.

**mkpaqfs invocation** (from `mkpaqfs-host.sh` lines 33-44 — host tool at `.build/bin/mkpaqfs`, built once by the existing script; the bake consumes it):

```sh
"$PLAN9/bin/9c" -O0 -I"$HARVEY_ROOT/sys/src/cmd/paqfs" -c …/mkpaqfs.c
"$PLAN9/bin/9l" -o "$HARVEY_ROOT/.build/bin/mkpaqfs" mkpaqfs.o -lflate -lsec -lmp -lbio -l9
```

PATH discipline (lines 23-26): strip any existing `$PLAN9/bin`, then append — `$PLAN9/bin` LAST, always.

---

### `build/aarch64/root-tree/` — paqfs image skeleton (new; config/data)

**Analogs:** `sys/src/9k/k10/root/k10root.proto` (empty-dir tree shape) + `sys/src/9k/k10/root/k10root.namespace` (scoped namespace shape) + `lib/namespace` (full file being scoped) + `cfg/example/termrc` (script being extended) + `rc/lib/rcmain` (shipped verbatim)

**Empty-dir tree precedent** (k10root.proto lines 1-49 — dirs from `/tmp/empty`, perms/owner columns):

```
adm d775 sys sys /tmp/empty
lib d775 sys sys /tmp/empty
	namespace 664 sys sys ./root/k10root.namespace
mnt d775 sys sys /tmp/empty
tmp d775 sys sys /tmp/empty
usr d775 sys sys /tmp/empty
	glenda d775 sys sys /tmp/empty
		lib d775 sys sys /tmp/empty
			profile 664 sys sys ../root/profile
```

The paqfs image is read-only (RESEARCH Pitfall 8) — every bind/mount *target* (`/root /env /proc /dev /srv /fd /bin /tmp /mnt`) must pre-exist as an empty dir in the skeleton.

**Scoped namespace precedent** (k10root.namespace lines 1-29 — the D-08 shape, note the `/root/` prefix because the served tree mounts at `/root` before newns runs):

```
# kernel devices
bind #c /dev
bind #d /fd
bind -c #e /env
bind #p /proc
bind -c #s /srv
# standard bin
bind /root/$cputype/bin /bin
bind -a /root/rc/bin /bin
```

D-08's scoped variant drops k10root's network lines (`#I`, `#l0`) and the factotum mount; keeps `#d`→/fd (devdup mandatory, D-05). `lib/namespace` lines 51-56 show what is being *removed* (`/usr/$user/tmp` bind, `cd`, `.`-includes) — scope to taste but keep `bind -c /usr/$user/tmp /tmp` or ship `/tmp` (settime mounts there).

**Scoped termrc** (D-05): `cfg/example/termrc` is 5 lines of near-empty rc — the scoped file ends in `exec rc -i` (no analog; new content, modeled on init.c's execl contract at `sys/src/cmd/init.c:177-178`).

**rcmain** ships **verbatim** from `rc/lib/rcmain` (lines 1-34; the `. -i '#d/0'` interactive arm at lines 23-28 is why devdup is mandatory). Do not edit it — the amd64-safety constraint keys on image *contents*, not shared sources.

---

### `sys/src/9k/boot/` additive aarch64 boot build (new mkfile/fragment + hand-written bootconf; config/build)

**Analogs:** `sys/src/9k/mk/bootmkfile` (whole file) + `sys/src/9k/mk/parse` (mkbootconf, lines 138-174) + `sys/src/cmd/paqfs/mkfile` (the standalone-command mk shape)

**BOOTLIB/member pattern** (bootmkfile lines 1-28):

```
BOOTDIR=../boot
BOOTLIB=$BOOTDIR/libboot.a$O
BOOTFILES=bootauth.$O aux.$O boot.$O bootcache.$O bootip.$O local.$O embed.$O settime.$O parts.$O paq.$O printstub.$O usb.$O
$BOOTLIB:	${BOOTFILES:%=$BOOTLIB(%)}
	names=`{membername $newprereq}
	ar vu $BOOTLIB $names
	rm $names
%.$O:	$BOOTDIR/%.c
	$CC -I$BOOTDIR $CFLAGS $BOOTDIR/$stem.c
```

**Generated bootconf being replaced by hand** (parse lines 138-167 — what `awk -mkbootconf` emits; the hand-written aarch64 equivalent supplies exactly these four symbols):

```c
#include <u.h>
#include <libc.h>
#include "../boot/boot.h"
Method method[] = {
	{ "embed", configembed, connectembed, "<default-image-path>" },
	…
};
int cpuflag = 0;              /* terminal → init -t (D-05) */
char* rootdir = "/root";
char* bootdisk = "";
```

**Extern contract** (`sys/src/9k/boot/boot.h` lines 17-25 — the symbols the hand-written file must define): `bootdisk`, `rootdir`, `cpuflag`, `method[]` are declared `extern … /* defined in ../$arch/boot$CONF.c */`.

**Standalone-command mk shape** (paqfs/mkfile — `</$objtype/mkfile` first line, `TARG`, `< /sys/src/cmd/mkmany` last) is the model if the planner places the aarch64 boot build as its own mkfile rather than a fragment. All edits additive + objtype-gated; the amd64 `bootmkfile` path is untouched (carried-forward constraint).

---

### `sys/src/9k/boot/embed.c` — the connect-method precedent (reuse as-is; service, request-response)

**Analog:** itself — the full verified flow (lines 24-74):

```c
int
connectembed(void)
{
	int i, p[2];
	Dir *dir;
	char **arg, **argp;

	dir = dirstat("/boot/paqfs");
	if(dir == nil)
		return -1;
	…
	print("paqfs...");
	if(bind("#c", "/dev", MREPL) < 0)
		fatal("bind #c");
	if(bind("#p", "/proc", MREPL) < 0)
		fatal("bind #p");
	if(pipe(p)<0)
		fatal("pipe");
	switch(fork()){
	case 0:
		…
		*argp++ = "/boot/paqfs";
		*argp++ = "-iv";
		*argp++ = paqfile;
		…
		dup(p[0], 0);
		dup(p[1], 1);
		exec("/boot/paqfs", arg);
		fatal("can't exec paqfs");
	}
	waitpid();
	close(p[1]);
	return p[0];
}
```

**Load-bearing details:** the `bind("#c",…)`/`bind("#p",…)` calls are **fatal** — r9 must have devcons and devproc attachable by `#`-name before boot reaches here (contrast boot.c's fail-soft `#æ`/`#S` binds). The method arg supplies the raw-image path (the devroot raw-image node the kernel must expose — RESEARCH "two-views watch-out"). If a new sibling method file is added instead of reusing embed.c directly, copy this file's shape exactly (Plan 9 style: tabs, `nil`, `fatal`, no header guards).

---

### `.github/workflows/r9-shell.yml` — CI linux leg (new; config, batch)

**Analog:** `.github/workflows/harvey.yml` (whole file — shape only; it predates nix CI)

**Existing shape** (lines 1-33): `on: [push, pull_request]`, single job on `ubuntu-latest`, apt deps, then the build script. The new workflow lands **alongside** (do not edit harvey.yml), same trigger block, but the steps are nix-based (RESEARCH A4: a nix-install action — DeterminateSystems or cachix — then `nix run .#boot-r9-smoke`). No in-repo nix-CI analog exists; the flake apps are the interface, so the workflow body is thin: checkout → install nix → `nix run .#boot-r9-smoke` with a generous step timeout. qemu runs TCG on the runner; the namespace build uses native `unshare` (RESEARCH A3).

---

### `build/REPRODUCIBILITY.md` — INT-03 doc update (modify; doc)

**Analog:** itself (lines 1-50). Copy its structure: Policy paragraph → Scope (what is/ isn't covered) → Derivation (exact commands) → Override. The INT-03 section documents the clean-checkout commands for both repos: `nix run .#boot-r9` (darwin: lima path; linux: native), the flake.lock pins (r9, nixpkgs, rust-overlay), and the KERNDATE policy's application to the aarch64 userland build.

---

### r9 kernel files — the two-tier analog pattern

Every r9 kernel file in this phase has **two** analogs, and the planner should cite both in plan actions:

1. **Idiom analog (r9, Rust):** how the code is *structured* — free functions not the `Dev` trait, mcslock static-publish, `Result<T, &'static str>` errors, `Chan` manipulation, qid encoding.
2. **Semantic reference (harvey, C):** what the code must *do* — the 9k port file whose semantics are ported literally (RESEARCH: "port don't redesign").

**Idiom analog 1: `r9/port/src/devroot.rs`** — the canonical device-operation shape (attach/walk/open/read/stat as inherent methods over a stateless-per-call handle). Key excerpts:

*Module-header documentation discipline* (lines 1-17 — every port device explains scope, why the Dev trait is not implemented, and the phase plan reference):

```rust
//! devroot — the root file tree over the initrd paqfs image (04-05, D-15).
//! …
//! The dat.rs `Dev` trait is NOT implemented here: its skeleton is
//! undispatched and partially non-constructible as written (`Walkqid` has an
//! unsized `_qids: [Qid]` field, …).  This module implements the device
//! OPERATIONS the plan names (attach/walk/open/read/stat) as inherent
//! methods; trait conformance lands when the devtab dispatch era (04-06+)
//! grows the skeleton into a constructible shape.
```

*Walk shape* (lines 87-105 — single-component walk, error strings, path bookkeeping):

```rust
pub fn walk(&self, c: &Chan, name: &[u8]) -> Result<Chan> {
    let dir = c.paqdir.as_ref().ok_or("devroot: walk of unattached chan")?;
    if dir.mode & DMDIR == 0 {
        return Err("devroot: walk of a non-directory");
    }
    if name.is_empty() || name.contains(&b'/') {
        return Err("devroot: walk takes a single name component");
    }
    let entries = self.fs()?.read_dir(dir)?;
    let Some(entry) = entries.into_iter().find(|e| e.name == name) else {
        return Err("devroot: name not found");
    };
    let mut nc = Chan::new(c.dev, qid_of(&entry), 0);
    nc.paqdir = Some(entry);
    nc.path = c.path.clone();
    nc.path.push(b'/');
    nc.path.extend_from_slice(name);
    Ok(nc)
}
```

*One-time publish idiom* (lines 42-60 — `static ROOT_IMG: Lock<Option<…>>`, mcslock `LockNode` per call):

```rust
static ROOT_IMG: Lock<Option<&'static [u8]>> = Lock::new("devroot", None);
pub fn mount_initrd(img: &'static [u8]) -> Result<DevRoot> {
    Paqfs::open_image(img)?;
    let node = LockNode::new();
    *ROOT_IMG.lock(&node) = Some(img);
    Ok(DevRoot { img })
}
```

*Stat wire writer* (lines 144-172, 197-225 — the stat(5) LE byte layout; devproc's status and every directory read need this same `StatWriter` shape).

**Idiom analog 2: `r9/port/src/devproc.rs`** — the per-process device shape being extended (walk qid encoding, hook injection). Key excerpts:

*Qid encoding scheme* (lines 50-57 — the never-collide encoding devproc's extension keeps):

```rust
fn dir_qid(pid: u64) -> Qid { Qid { path: pid * 2, vers: 0, typ: QTDIR } }
fn note_qid(pid: u64) -> Qid { Qid { path: pid * 2 + 1, vers: 0, typ: QTFILE } }
```

*Dependency-injection hook* (lines 31-47 — arch-side registration, the pattern devenv's seeded root group and devcons's note-post reuse):

```rust
type PostHook = fn(u64, &[u8]) -> Result<()>;
static POST_HOOK: Lock<Option<PostHook>> = Lock::new("devproc", None);
pub fn set_post_hook(hook: PostHook) {
    let node = LockNode::new();
    let mut guard = POST_HOOK.lock(&node);
    assert!(guard.is_none(), "devproc: post hook registered twice");
    *guard = Some(hook);
}
```

**Idiom analog 3: `r9/port/src/dat.rs`** — the Chan/Qid/mode vocabulary (lines 14-35, 40-78):

```rust
pub const DEV_ROOT: usize = 0;
pub const DEV_CONS: usize = 1;
pub const DEV_PROC: usize = 2;
…
pub const OREAD: u32 = 0;  /* OWRITE 1, ORDWR 2, OEXEC 3; OMASK = 3 */
```

**Chan::dup caveat** (lines 63-77 — RESEARCH Pitfall 4's load-bearing detail: the value copy "approximates" shared-Chan semantics; devpipe chans must instead carry an **Arc-backed shared queue** or connectembed deadlocks silently). New devices with shared state across fork copy an `Arc`, never the bytes.

**Idiom analog 4: `r9/port/src/mcslock.rs`** (lines 17-60) — the `Lock`/`LockNode` MCS idiom every static-publish uses; syscall context takes these locks briefly, IRQ-context code must never take a lock syscall context holds (the devcons write-path rule, `r9/aarch64/src/syscall.rs` header lines 27-32).

**Semantic references (harvey `sys/src/9k/port/`, port literally):**

| r9 file | 9k C reference | Size | What to port |
|---------|---------------|------|--------------|
| `namec.rs`/`namespace.rs` | `chan.c` + `dev.c` | 1693 + 461 | namec/walk, union-dir resolution, mount-point stitching, MREPL/MAFTER/MBEFORE/MCREATE semantics (frozen flag values: `sys/include/libc.h:558-564`) |
| `pgrp.rs` | `pgrp.c` | 322 | mount table + env group per pgrp; RFNAMEG share / RFCNAMEG copy (frozen rfork values: `sys/include/libc.h:613-623`, already mirrored at `r9/aarch64/src/syscall.rs:58-68`) |
| `devmnt.rs` | `devmnt.c` | 1206 | Tversion/Tattach/Twalk/Topen/Tread/Tstat/Tclunk over an fd; mntchk bounds-checking against negotiated msize (security table, RESEARCH) |
| `devpipe.rs` | `devpipe.c` + `qio.c` | 416 + (qio) | the shared Queue both pipe ends reference |
| `devdup.rs` | `devdup.c` | 145 | per-fd files by number (`#d/0`) |
| `devsrv.rs` | `devsrv.c` | 360 | `#s` create/write registry (boot's `srvcreate("boot", fd)` is **fatal** on failure — `sys/src/9k/boot/aux.c`) |
| `devenv.rs` | `devenv.c` | 441 | per-pgrp kv; RFENVG share / RFCENVG copy; kernel-seeded root group (`cputype=aarch64`, `nobootprompt=embed`, `service=terminal` — RESEARCH Pattern 3 / Pitfall 1) |
| `devcons.rs` | `devcons.c` | 1405 | cooked line discipline, `/dev/consctl` raw switch, DEL→"interrupt" note to the note group, `#c/hostowner` write target |
| `devproc.rs` | `devproc.c` | 1667 | root dir listing of live pids, per-pid dir, `status` (≥12 whitespace fields matching `sys/src/cmd/ps.c:63-75` tokenizer), `args`, keep `note`, add `ctl` accepting `pri 10` |

---

### `r9/aarch64/src/syscall.rs` — dispatch growth (modify; middleware, request-response)

**Analog:** itself. Key excerpts:

**DISPATCH table pattern** (lines 74-95 — the growth point; new entries land as `Some(sys_…)` at their frozen sys.h number, everything else stays `None` → loud unimplemented):

```rust
static DISPATCH: [Option<Handler>; 55] = {
    let mut t: [Option<Handler>; 55] = [None; 55];
    t[CLOSE as usize] = Some(sys_close);
    …
    t
};
```

**Handler contract** (lines 70-72): `type Handler = fn(&mut TrapFrame, &mut Proc) -> u64;` — frame carries args in and result out (dispatch writes `frame.x0`, lines 100-128).

**Loud-unimplemented default** (lines 109-116):

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

**Prefix routing being replaced** (lines 385-433 — the hardcoded `sys_open` routing D-03 retires; new devices must NOT extend this — the namespace core owns resolution. RESEARCH anti-patterns: "do not extend it for the new devices").

**T-04-11 user-VA validation** (header lines 17-25 + `sargs`/`copy_user_str` at lines 374-382): every new handler validates user pointers via `Proc::validate_user_range` before any kernel read/write. This is the phase's core security control — copy the existing handlers' validation-first shape.

**Width discipline** (header + lines 130-140 region): int-width args are 32-bit-masked from their 8-byte slots (7c leaves upper-half garbage) — new handlers copy this masking.

---

### `r9/aarch64/src/proc.rs` — Proc growth (modify; model)

**Analog:** itself (lines 46-59 sizing constants, 167-229 field block).

**Sizing constants** (lines 52-54 — RESEARCH Pitfall 3's growth point; the trace settles the new values, exhaustion must be loud):

```rust
pub const MAX_PROCS: usize = 8;
pub const MAX_FDS: usize = 16;
```

**Field-block shape** (lines 167-229 — new pgrp/env/fd-group fields join this struct; note the established comment discipline: every field names its phase/plan provenance and its locking/lifetime rule):

```rust
pub struct Proc {
    pub pid: u64,
    …
    /// fd slots.  fd 0/1/2 are devcons chans, preopened at construction
    /// (D-15, Plan 9 convention) — 04-06 fills the seam 04-04 left.
    pub fd_table: [Option<Chan>; MAX_FDS],
    …
    pub note_queue: VecDeque<Note>,
    …
    pub errstr: [u8; ERRMAX],
}
```

rfork flag handling grows real RFNAMEG/RFCNAMEG/RFENVG/RFCENVG semantics here (frozen values already mirrored at `syscall.rs:58-68`).

---

### `r9/flake.nix` — `packages.kernel-virt` (modify; config)

**Analog:** itself (lines 44-93 — currently devShells + apps only, no `packages.*`) + `harvey/flake.nix` lines 28-35 (the `packages = forAllSystems …` block shape).

**Toolchain discipline** (lines 17-21 — the single-source-of-truth rule the kernel package build must honor):

```nix
# rust-toolchain.toml is the single source of truth for channel,
# components and targets (D-09) — never restate pins here
toolchainFor = system:
  (pkgsFor system).rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
```

**writeShellApplication + objcopy export** (lines 62-92 — the apps block; the package's build derivation reuses the same `objcopyExport`/toolchain plumbing). D-11's output: built aarch64 ELF + the committed `aarch64/lib/virt-gicv2.dtb` — Harvey consumes these as `inputs.r9.packages.${system}.kernel-virt`.

---

### `r9/xtask/` — initrd glue removal/inversion (modify; utility)

**Analog:** itself. `xtask/src/config.rs` lines 385-434 (`prepare_initrd_dtb` — the fdtput recipe above) is what Harvey's launcher reimplements/documents per D-11; `xtask/src/main.rs`'s `initrd` subcommand/`ensure_initrd_image` (research-cited lines 1613-1735) and the `R9_HARVEY_DIR`/`../harvey` glue are what gets removed or slimmed per D-10. **Constraint:** r9's boot-test profiles (virt/virtexec/virtmulti/virtnotes/virtpanic) consume `target/bootfs.paq` — the r9-side regression-gate supply chain must keep working (RESEARCH Open Question 2; planner resolves, lowest-risk = keep a minimal bake r9-side).

**TOML-as-data profile shape** (`r9/aarch64/lib/config_virtexec.toml` lines 21-47 — the one-profile-per-file convention; `[qemu]` keys map 1:1 to launch flags, comments carry the decision provenance).

---

### `r9/HowItWorks.md` — material-change update (modify; doc)

**Analog:** itself (lines 1-5 — the mandate is in the header):

```markdown
# How r9 Works

**Please update with any material code change!**
```

Namespace machinery, devmnt, and the new device set are material — the update follows the doc's existing section shape (per-topic `###` sections under `## aarch64`, flow-numbered steps, file paths with line refs, decision provenance in parentheses).

---

## Shared Patterns

### r9: device module idiom (applies to all new `port/src/dev*.rs` files)

**Source:** `r9/port/src/devproc.rs` (whole file) + `r9/port/src/devroot.rs` (whole file)
**Apply to:** `devmnt.rs`, `devpipe.rs`, `devdup.rs`, `devsrv.rs`, `devenv.rs`, and the devcons/devproc extensions

1. Module header: `//!`-doc naming the phase/plan provenance, the hard scope, and why the `Dev` trait is not implemented.
2. Free functions / inherent methods: `attach() -> Chan`, `walk(&Chan, &[u8]) -> Result<Chan>`, `open(&Chan, u32) -> Result<Chan>`, `read/write(&mut Chan, …) -> Result<u64>` — never the dat.rs `Dev` trait.
3. Qid encoding: a collision-free `path` scheme documented in a comment (devproc's `pid*2`/`pid*2+1` shape).
4. Errors: `Result<T, &'static str>` with `"devX: …"` prefixed static strings; loud errors, never silent degradation (unless the plan says degrade).
5. Cross-crate state: dependency-injected hooks (`set_post_hook` shape) or one-time-published statics under `mcslock::Lock` + per-call `LockNode::new()`.
6. Frozen constants mirrored from Harvey headers with a provenance comment (`// Harvey sys/include/libc.h — frozen values`).

### r9: syscall handler discipline (applies to every new DISPATCH entry)

**Source:** `r9/aarch64/src/syscall.rs` (header lines 1-32, dispatch lines 100-128, sys_open lines 374-440)
**Apply to:** BIND, CHDIR, DUP, PIPE, CREATE, FVERSION, MOUNT, STAT/FSTAT/WSTAT, SEEK, SLEEP, ALARM, FAUTH, FD2PATH, REMOVE (the trace-settled set)

1. Number constant named from `sys.h` with a provenance comment; `DISPATCH` entry `Some(handler)` — unimplemented stays `None` (loud).
2. `sargs(frame, proc, N)` first; every user VA validated via `Proc::validate_user_range` before dereference (T-04-11); int-width args masked to 32 bits.
3. Result rides `frame.x0`; `-1` + `proc.set_errstr("…")` on failure.
4. No new prefix routing — paths resolve through the namespace core.

### Harvey: Plan 9 C style boundary (applies to any Harvey C touched — boot method file, bootconf)

**Source:** `sys/src/9k/boot/embed.c` (whole file) + AGENTS.md conventions
**Apply to:** all C in `sys/src/9k/boot/`

Tabs only; `nil` never `NULL`; `-1`+errstr / `fatal()` / `exits()`; block comments only (`/* */`); no `#ifndef` guards; automatic variables not initialized at declaration; return type on its own line. Styles never cross the repo boundary (r9 follows its rustfmt.toml).

### Harvey: host-shell script discipline (applies to all new `build/aarch64/*.sh` and `nix/*.sh`)

**Source:** `build/aarch64/stage-build.sh` + `subset-build.sh` + `mkpaqfs-host.sh`
**Apply to:** `mkimage.sh`, `boot-r9.sh`, `smoke-r9.sh`

1. `set -eu` (`-euo pipefail` inside flake apps), header comment naming plan/decision provenance + usage line.
2. `HARVEY="${HARVEY:-$PWD}"`, checkout sanity check, `err() { echo "error: $*" >&2; exit 1; }`.
3. `$PLAN9/bin` LAST in PATH (strip-then-append dance, mkpaqfs-host.sh lines 23-26); host grep called as `/usr/bin/grep` (plan9 grep lacks `-q`/`-x`).
4. Guest work: single generated script → rsync to guest `/tmp` → `vm-run.sh` (namespace wrap) → host greps for the `-OK` marker.
5. Artifacts rsynced back to gitignored paths; asserted present (and magic-checked for binaries) host-side.
6. `mktemp -d` + `trap … EXIT` cleanup.

### Both repos: evidence + marker discipline

**Source:** Phase 4 carried-forward (04 EVIDENCE; RESEARCH Pattern 6, Pitfalls 10-11)
**Apply to:** the smoke app, CI leg, and every kernel-capability gate

Marker presence assertions (never interleaving order); fresh log per run; qemu process-group kill with bracket-fixed pkill; TCG-sized timeouts; canonical evidence consolidated in `.planning/phases/05-*/` with the r9 commit SHA recorded.

## No Analog Found

None — every file in this phase has at least one in-repo analog (the phase's novelty is *placement*, not *design*; RESEARCH "Key insight"). The closest calls:

| File | Note |
|------|------|
| `.github/workflows/r9-shell.yml` | Only the workflow *shape* is analogous (harvey.yml predates nix); the nix-action scaffolding follows RESEARCH A4, not in-repo code |
| `build/aarch64/root-tree/rc/bin/termrc` | Content is new (5-line `cfg/example/termrc` is a skeleton, not a model for `exec rc -i`); the *contract* it fulfills is pinned by `sys/src/cmd/init.c:177-178` and `rc/lib/rcmain:23-28` |
| `r9/port/src/namec.rs` dispatch/registry shape | The 9k C semantics are the reference, but r9's dispatch *shape* (hand-built registry vs generated devtab) is planner's-choice — RESEARCH Open Question 3 recommends hand-built registry matching the established free-function idiom |

## Metadata

**Analog search scope:** `harvey/{flake.nix, nix/, build/, sys/src/9k/boot/, sys/src/9k/mk/, sys/src/9k/k10/root/, sys/src/9k/port/, sys/src/cmd/, lib/, rc/, cfg/, .github/workflows/}`, `r9/{flake.nix, port/src/, aarch64/src/, aarch64/lib/, xtask/src/, HowItWorks.md}`
**Files scanned:** 30+ (24 read in full or targeted ranges; 10 9k port C files confirmed present + sized)
**Tracked-source gate:** all 44 cited analog paths verified `git ls-files`-tracked in their owning repo (harvey: 34, r9: 12; some cited from both lists) — no gitignored mirrors referenced
**Pattern extraction date:** 2026-11-02
