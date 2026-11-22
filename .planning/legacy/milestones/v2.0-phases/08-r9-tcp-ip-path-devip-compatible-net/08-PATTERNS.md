# Phase 8: r9 TCP/IP Path — devip-Compatible /net — Pattern Map

**Mapped:** 2026-11-05
**Files analyzed:** 37 (24 r9-side new/modified, 11 Harvey-side new/modified, 2 doc/evidence artifacts)
**Analogs found:** 31 / 37 (6 files have no close in-tree analog — all are new-pattern surfaces called out below)

**Tracked-source gate:** every analog path below was verified with `git ls-files` — r9 paths from repo root `/Users/neilskinner/code/r9` (its own git root, not a submodule), Harvey paths from `/Users/neilskinner/code/harvey`. No gitignored mirrors named.

**Cross-repo note:** `../r9` resolves to `/Users/neilskinner/code/r9`. Harvey `sys/src/9k/ip/*` files are **read-only semantics references** — never edited, never copied from as code (C→Rust); they feed the D-03 contract doc. The Rust analogs under `../r9/port/src/` are the code-shape authorities.

## File Classification

### r9-side — new files (the work lands here)

| New File | Role | Data Flow | Closest Analog | Match Quality |
|----------|------|-----------|----------------|---------------|
| `port/src/devip.rs` | device (kernel file tree) | request-response + event-driven | `port/src/devether.rs` | exact |
| `port/src/devip/conv.rs` | model (conv lifecycle) | request-response | `port/src/netif.rs` (ConvBook + lifecycle) | role-match |
| `port/src/devip/proto.rs` | service (ctl verbs, status/stats) | request-response | `port/src/devether.rs` (`write_ctl`) + `netif.rs` (`stats_content`) | role-match |
| `port/src/devip/ipifc.rs` | service (interface/addr model) | request-response | `port/src/devether.rs` (`write_ctl`, `attach`) | partial |
| `port/src/devip/route.rs` | model (route forest) | request-response | none — see No Analog Found | — |
| `port/src/devip/arp.rs` | model (snoop-fed view table) | event-driven (frame snoop) | none — see No Analog Found | — |
| `port/src/devip/netlog.rs` | device file + logging | streaming (blocking log read) | `port/src/devcons.rs` (file table) + Harvey `sys/src/9k/ip/netlog.c` (read-only semantics) | partial |
| `port/src/devip/medium.rs` | registry (medium names → bind) | request-response | `port/src/namec.rs` (`registry_attach`) + `devether.rs` (`parse_spec`) | partial |
| `port/src/devip/stack.rs` | service (THE SEAM: Interface+SocketSet ownership, drive, deadline) | event-driven | `port/src/netif.rs` (NETIF/INGRESS split) + `port/src/devpipe.rs` (hooks) + `aarch64/.../timer.rs` (tick sweep) | role-match |
| `port/src/devip/phyadapt.rs` | driver adapter (smoltcp `Device` impls) | streaming (frame RX/TX) | `port/src/netif.rs` (`FrameStore`) + RESEARCH §Code Examples compiled skeleton | partial |
| `port/src/devip/waker.rs` | utility (RawWaker bridge — UNSAFE) | event-driven | `port/src/mcslock.rs` (unsafe-floor discipline) + `UNSAFE-AUDIT.md` | role-match |
| `port/src/devip_tests.rs` | test (host-side pins) | batch | `port/src/devether_tests.rs` | exact |
| devcons `random`/`urandom` files (D-17; either `devcons.rs` extension or a small `devrandom` — planner's call) | device file | request-response | `port/src/devcons.rs` (`QID_TIME` + `FILES` table) + `port/src/fdt.rs` (DTB seed read) | exact (devcons ext) |
| `docs/devip-contract.md` (r9-side doc home per CONTEXT discretion) | doc (D-03 contract) | — | `HowItWorks.md` / `UNSAFE-AUDIT.md` (doc voice) | role-match |

### r9-side — modified files

| Modified File | Change | Analog In-File |
|---------------|--------|----------------|
| `port/src/lib.rs` | `#[forbid(unsafe_code)] pub mod devip;` + `#[cfg(test)] mod devip_tests;` | existing module block (lib.rs:18-65, 72-88) |
| `port/src/dat.rs` | `pub const DEV_IP: usize = 9;` | `DEV_ETHER` entry (dat.rs:34-38) |
| `port/src/namec.rs` | `'I' =>` registry arm + `DEV_IP` arms in `dev_walk`/`open_chan`/`stat_chan`/`close_chan` | `DEV_ETHER` arms (namec.rs:75, 115, 336, 361, 405-412) |
| `aarch64/src/main.rs` | devip `{current, block, wake}` hook registration | devether registration (main.rs:501-506) |
| `aarch64/src/syscall.rs` | `DEV_IP` arms in `sys_pread`/`sys_pwrite` | `DEV_ETHER` arms (syscall.rs:292-297, 383-388) |
| `aarch64/src/platform/virt/timer.rs` | net-deadline arm in `tick()` | `tick_timeouts` sweep (timer.rs:114-130) |
| `aarch64/src/platform/virt/pci.rs` | ether-adapter "frames pending" wake in `virtio_net_irq` | `rx_ingress`/`tx_retired` calls (pci.rs:873, 904) |
| `Cargo.toml` (workspace) | `cfg(dev_ip)` check-cfg predeclare | `cfg(dev_ether)` entry (Cargo.toml:34) |
| `port/Cargo.toml` | smoltcp dependency entry | `miniz_oxide` no_std feature-pin precedent (port/Cargo.toml:8-13) |
| `Cargo.lock` | committed smoltcp closure | existing committed lockfile |
| `xtask/src/main.rs` | net-profile marker constants + BootStage arms + `finish()` net assertions | Net profile (main.rs:862-875, 1004-1010, 1453-1487, 1568-1598) |
| `UNSAFE-AUDIT.md` | waker-bridge site entry | reconciliation table + per-site SAFETY rule |

### Harvey-side — new/modified files

| File | Role | Data Flow | Closest Analog | Match Quality |
|------|------|-----------|----------------|---------------|
| `build/aarch64/tcpblast.c` (new payload) | command (gate binary) | streaming (TCP I/O loop) | `build/aarch64/netblast.c` | exact |
| `build/aarch64/` echo payload(s) (udp echo + tcp/7 echo server) | command (gate binary) | request-response | `build/aarch64/netblast.c` + `stacksmoke.c` (bake shape) | exact |
| `build/aarch64/subset-build.sh` (modified) | build orchestration | batch | in-file: netblast/dd/bind additive lines (255-277) + `cmd/*:target` rule (224-246) | exact |
| `build/aarch64/root-tree/lib/net-gate rc script(s)` (new fixtures) | config (rc sweep) | batch | `build/aarch64/root-tree/lib/net-sweep.rc` | exact |
| addressing seam data file (D-11; location = agent discretion) | config (single-source data) | — | none — new pattern (Phase 7 D-09 minimal-wiring spirit; see No Analog Found) | — |
| composed net smoke expect script (D-13 sibling of smoke-r9.expect) | test harness | event-driven (expect stages) | `build/aarch64/smoke-r9.expect` | exact |
| `nix/` wrapper for the net smoke (new or extended) | build glue | batch | `nix/smoke-r9.sh` | exact |
| `flake.nix` (modified) | flake app entry | — | `smokeR9Pkg`/`tier1R9Pkg` entries (flake.nix:106-127) | exact |
| `.github/workflows/r9-shell.yml` (modified) | CI config | batch | existing `boot-smoke` job (whole file, 40 lines) | exact |
| `flake.lock` | pin advance (own commit, fresh CI) | — | push-before-pin mechanics (CONTEXT carried-forward) | n/a |
| `.planning/phases/08-*/08-EVIDENCE.md` | evidence doc | — | `.planning/phases/07-*/07-EVIDENCE.md` | exact |

---

## Pattern Assignments

### `port/src/devip.rs` (device, request-response + event-driven)

**Analog:** `../r9/port/src/devether.rs` — the Phase-7 device-surface precedent. devip is the SAME shape: a device as a module of free functions (never the non-constructible `dat::Dev` trait), dispatched by `chan.dev` from namec/syscall arms, with the model half in submodules (devip/*) the way devether's model half lives in netif.rs.

**Module header discipline** (devether.rs:1-78 — copy this *shape*: donor semantics named, deltas-from-donor enumerated, locked ctl depth stated, gate-unexercised paths recorded loudly):
```rust
//! devether — the ethernet device `#l` (NETDRV-03): `/net/ether0` per
//! netif.h over the 07-06 virtio-net driver.  Semantics ported from
//! 9/pc's `devether.c` + 9k's `netif.c` (the donors); the Netif/Netfile
//! model, the IRQ↔syscall handoff, and the EtherHooks live in
//! `netif.rs` (this file is the DEVICE surface: attach/walk/open/stat/
//! read/write/close as free functions, dispatched by chan.dev via
//! namec.rs's legs + syscall.rs's pread/pwrite arms — the established
//! port idiom; the dat.rs Dev trait is non-constructible, see
//! devroot.rs's header).
```
devip.rs's header names `sys/src/9k/ip/devip.c` + `sys/man/3/ip` as donors and enumerates the D-08 deltas (accept-pool listen bridge, /net/arp view divergence, icmpv6 full-packet bridge, `add 0.0.0.0` absorption).

**Imports + forbid** (devether.rs:79-95):
```rust
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::dat::{Chan, Qid, DEV_ETHER, OREAD};
use crate::mcslock::{Lock, LockNode};
use crate::netif::{self, NFILE, NUMSIZE};
use crate::Result;
```
(`#![forbid(unsafe_code)]` is per-module, NOT crate-level — devip.rs and every devip/* submodule except waker.rs carries it; waker.rs is the single named unsafe module per the UNSAFE-AUDIT contract.)

**attach + spec parse** (devether.rs:151-177) — devip's attach parses the stack number 0..15 per `sys/man/3/ip` (Nfs=128 in the donor — contract doc records the r9 bound):
```rust
pub fn attach(spec: &[u8]) -> Result<Chan> {
    let unit = parse_spec(spec)?;
    if unit != 0 {
        return Err("devether: no such interface");
    }
    driver()?; // "devether: no ether0" when the bind never published
    let mut c = Chan::new(DEV_ETHER, Qid { path: 0, vers: 0, typ: QTDIR }, 0);
    c.path = b"#l".to_vec();
    Ok(c)
}

fn parse_spec(spec: &[u8]) -> Result<u64> {
    if spec.is_empty() {
        return Ok(0);
    }
    let mut v: u64 = 0;
    for &b in spec {
        if !b.is_ascii_digit() {
            return Err("devether: bad attach spec");
        }
        v = v.checked_mul(10).and_then(|v| v.checked_add(u64::from(b - b'0'))).ok_or("devether: bad attach spec")?;
    }
    Ok(v)
}
```

**Static file-table + walk by qid-tag match** (devether.rs:184-243) — devip's three-level tree (#I root → proto dirs tcp/udp/icmp/icmpv6/ipifc → conv dirs) mirrors this LEVEL2/LEVEL3 table + `nettype`/`netid` decode shape, with devip's OWN qid scheme `QID(p,c,y) = (p<<17)|(c<<5)|y` (RESEARCH §Pattern 2 — do NOT reuse netif's NETQID packing):
```rust
const LEVEL2_FILES: [(&[u8], u64, u32); 4] = [
    (b"clone", netif::NCLONEQID, 0o666),
    (b"addr", netif::NADDRQID, 0o666),
    (b"stats", netif::NSTATQID, 0o444),
    (b"ifstats", netif::NIFSTATQID, 0o444),
];
// ...
pub fn walk(c: &Chan, name: &[u8]) -> Result<Chan> {
    if c.qid.typ != QTDIR {
        return Err("devether: walk of a non-directory");
    }
    // ... match on c.qid.path's tag via netif::nettype(p), decimal-name
    // parse for conv dirs (parse_decimal, devether.rs:246-258) ...
}
```

**open with clone-allocates + owner-collapse surface** (devether.rs:265-293):
```rust
pub fn open(c: &Chan, mode: u32) -> Result<Chan> {
    let path = c.qid.path;
    if c.qid.typ == QTDIR {
        if mode != OREAD {
            return Err("devether: directory opens OREAD only");
        }
        // ...
    }
    let mut qid = c.qid;
    match netif::nettype(path) {
        netif::NDATAQID | netif::NCTLQID => {
            netif::conv_open(netif::netid(path) as usize)?;
        }
        netif::NCLONEQID => {
            let id = netif::conv_alloc()?;
            qid = Qid { path: netif::netqid(id as u64, netif::NCTLQID), vers: 0, typ: QTFILE };
        }
        // ...
    }
}
```
devip's clone: first free conv in the proto's table, qid rewritten to the new conv's ctl, reading ctl back returns the conv number (`"%lud"` — RESEARCH §Generic conversation semantics).

**stat wire records — FOUR strings, StatWriter** (devether.rs:322-349 + dat.rs:196-237):
```rust
fn write_stat_record(sb, name, qid_path, typ, mode, length) -> Result<u64> {
    let need = 41 + (2 + name.len()) + 3 * (2 + 6);
    if sb.len() < need {
        return Err("devether: stat buffer too small");
    }
    let mut w = crate::dat::StatWriter::new(sb);
    w.u16(0); w.u32(DEV_ETHER as u32); w.u8(typ); w.u32(0); w.u64(qid_path);
    w.u32(mode); w.u32(0); w.u32(0); w.u64(length);
    w.string(name);
    w.string(b"glenda"); w.string(b"glenda"); w.string(b"glenda"); // muid — statcheck walks four
    Ok(w.finish())
}
```
Reuse `dat::StatWriter` directly (it is `pub(crate)`). Owner-collapse modes computed at listing/stat time: 0666 pre-open, 0660 after first open (devether.rs:363-366).

**read dispatch with read_str window + readnum fixed width** (devether.rs:463-547): `read_str` handles offset semantics incl. `u64::MAX` streaming; `readnum_content` (devether.rs:493-510, `pub(crate)` — reusable) is the 12-byte NUMSIZE shape for ctl/conv-number reads. The 32-bit truncation discipline in its doc comment (the donor's `ulong` parameter) applies to devip's conv-number reads too.

**write dispatch + ctl tokenizer with 64-byte cmdbuf bound** (devether.rs:617-708) — THE ctl-verb parse pattern every devip proto/ipifc/arp/netlog write copies:
```rust
fn write_ctl(c: &mut Chan, buf: &[u8]) -> Result<u64> {
    let id = netif::netid(c.qid.path) as usize;
    let n = buf.len();
    // The donor's 64-byte cmdbuf bound.
    let text = &buf[..n.min(63)];
    // matchtoken's shape: the verb, then whitespace-separated args.
    let mut it = text.split(|&b| b == b' ' || b == b'\t' || b == b'\n').filter(|w| !w.is_empty());
    let Some(verb) = it.next() else { return Ok(n as u64) }; // an empty write is a no-op
    match verb {
        b"connect" => { /* parse + range-check + model call */ }
        // ...
        _ => Err("devether: bad ctl"),
    }
}
```
devip maps: empty write → `"short control request"` error (devip semantics differ from devether's no-op — the contract doc's error table rules), unknown verb → `"unknown control request"`.

**Blocking data read — register → drop → block loop** (devether.rs:555-571) — THE shape devip's tcp/udp/icmp data reads reproduce with smoltcp sockets + wakers in place of frame queues:
```rust
fn read_data(c: &mut Chan, buf: &mut [u8]) -> Result<u64> {
    let id = netif::netid(c.qid.path) as usize;
    let mut h: Option<netif::EtherHooks> = None;
    loop {
        match netif::pop_frame(id, buf) {
            netif::Pop::Frame(n) => return Ok(n),
            netif::Pop::Closed => return Ok(0), // EOF — the donor's qclosed queue
            netif::Pop::Empty => {}
        }
        if h.is_none() {
            h = Some(netif::hooks()?);
        }
        let pid = (h.unwrap().current)();
        netif::register_rx_waiter(pid);
        (h.unwrap().block)(); // park mid-syscall; an ingress frame wakes us
    }
}
```
devip's loop: check `socket.can_recv()` → register recv waker + record blocked pid in the conv → `block()` → on wake re-check (spurious-wake discipline, RESEARCH P-C). UDP reads stay Qmsg (discard-what-doesn't-fit); TCP reads are byte-stream.

---

### `port/src/devip/conv.rs` (model, request-response)

**Analog:** `../r9/port/src/netif.rs` — the ConvBook + lifecycle functions.

**Conv bookkeeping struct** (netif.rs:122-143):
```rust
pub(crate) struct ConvBook {
    pub(crate) inuse: usize,
    /// First-opener-owns (the single-user netown collapse): set by the
    /// first data/ctl open; the stat/listing modes read 0660 once set,
    /// 0666 before (the donor's byte-identical surface).
    pub(crate) owner_set: bool,
    pub(crate) typ: i32,
    pub(crate) prom: bool,
}
impl ConvBook {
    const fn new() -> ConvBook {
        ConvBook { inuse: 0, owner_set: false, typ: 0, prom: false }
    }
}
```
devip's Conv carries: state (Idle/Announcing/Announced/Connecting/Connected — RESEARCH line 407), perms/owner collapse, laddr/raddr/lport/rport, ttl/tos, `SocketHandle` into the seam's SocketSet, the waiter pid set, headers flag, err queue. Const-new + fixed array (`[const { ConvBook::new() }; NFILE]`, netif.rs:191) is the allocation-free static pattern.

**Lifecycle: alloc / open / ref / close-with-wake-outside-locks** (netif.rs:407-516). The close pattern is load-bearing for devip (listen hangup wakes, EOF on last close):
```rust
pub(crate) fn conv_close(id: usize) {
    // ...
    let mut pids = [0u64; 16];
    let mut nwake = 0;
    {
        let node = LockNode::new();
        let mut ng = NETIF.lock(&node);
        let mut ig = INGRESS.enter().expect("netif: close with INGRESS contended (boot bug)");
        // ... decrement inuse; at zero reset owner/type, drain queue ...
        nwake = ig.rx_waiters.take(&mut pids);
    }
    // A blocked reader of the closed conversation wakes and reads EOF
    // (the donor's qclose → qwait-returns-0 arm).  The wake hook call is
    // OUTSIDE the locks (never hold either across PROC_TABLE).
    if nwake > 0 && let Ok(h) = hooks() {
        (h.wake)(&pids[..nwake]);
    }
}
```
Rules to copy: (1) locks taken sequentially in a documented order (NETIF → INGRESS here; devip's order = CONV-table → STACK cell, documented in stack.rs), (2) wake AFTER guards drop, (3) the reset-at-zero semantics (devip's `closeconv`: state=Idle, owner="network" collapse).

**`Chan::dup` open-ref bump** (dat.rs:167-186 + devether.rs:297-301) — devip MUST add its own arm to `Chan::dup` (dat.rs:171-173 is the devether precedent) or rfork/dup'd fds double-decrement convs:
```rust
if self.copen && self.dev == DEV_ETHER {
    crate::devether::ref_open(self.qid.path);
}
```

---

### `port/src/devip/proto.rs` (service, request-response)

**Analogs:** `devether.rs` `write_ctl` (above) for verb dispatch; `netif.rs` `stats_content` for the stats/status formatters; Harvey `sys/src/9k/ip/tcp.c`/`udp.c`/`icmp.c`/`icmp6.c` as the READ-ONLY semantics authority (formats enumerated in RESEARCH §The devip Contract Surface — tcp.c:493-509 status format, tcp.c:348-375 statnames, udp.c:124-132 status, icmp.c:128-137 status).

**Live-counter stats content pattern** (netif.rs:733-765) — devip's per-proto stats files are devip-maintained counters (RESEARCH P-M: smoltcp exposes none; all-zero stats files are a contract violation):
```rust
pub(crate) fn stats_content() -> Result<Vec<u8>> {
    let (link, mbps, prom, addr, alen) = {
        let node = LockNode::new();
        let ng = NETIF.lock(&node);
        let Some(nif) = ng.as_ref() else { return Err("devether: no ether0") };
        (nif.link, nif.mbps, nif.prom, nif.addr, nif.alen)
    };
    let (inp, outp, sov) = {
        let ig = INGRESS.enter().ok_or("devether: ingress contended (invariant violation)")?;
        (ig.inpackets, ig.outpackets, ig.soverflows)
    };
    let mut out = Vec::new();
    let mut line = |key: &[u8], val: Vec<u8>| {
        out.extend_from_slice(key);
        out.extend_from_slice(&val);
        out.push(b'\n');
    };
    line(b"in: ", dec(inp));
    // ...
    Ok(out)
}
```
Copy: sequential (never nested) lock acquisition, `dec()` decimal helper (netif.rs:707-717), key-value line closure.

**TCP state-name mapping:** devip `tcpstates[]` ↔ smoltcp `tcp::State` is a pure rename table (RESEARCH line 240 — 11 states 1:1). Pin it host-side in devip_tests.

---

### `port/src/devip/stack.rs` (THE SEAM — event-driven)

**Analogs:** netif.rs locking invariants header + `BusyCell` handoff; devpipe.rs hooks struct; timer.rs tick sweep. The seam is a NEW composition, but every primitive has an in-tree precedent.

**Locking-invariants module header** (netif.rs:42-69 — copy this *documentation discipline* verbatim in spirit: named locks, nesting order, IRQ rules, why-uncontended proof):
```rust
//! LOCKING (the load-bearing invariants):
//!
//! - `NETIF` is an mcslock serializing all SYSCALL-context devether
//!   model ops (open/close/ctl/listing).  It is a LEAF lock: never held
//!   across block()/sched(), never taken from IRQ context.  The ONE
//!   permitted nesting is NETIF → INGRESS ...
//! - `INGRESS` is a `BusyCell` (mcslock.rs's non-spinning sibling) ...
//! - No allocation happens in IRQ context ...
```
stack.rs's header names: the `STACK` BusyCell owning `Interface + SocketSet + adapter`, the IRQ↔syscall handoff rule (poll NEVER in IRQ context — RESEARCH Pattern 1's three verified reasons), the wake-after-drop rule, the stack-debt flag.

**BusyCell usage** (mcslock.rs:170-204 + netif.rs:351-362 static construction):
```rust
pub struct BusyCell<T> { busy: AtomicBool, data: UnsafeCell<T> }
// ...
pub fn enter(&self) -> Option<BusyGuard<'_, T>> {
    if self.busy.swap(true, Ordering::AcqRel) {
        return None;
    }
    Some(BusyGuard { cell: self })
}
```
The seam's `drive_stack()` (RESEARCH §Code Examples drive skeleton) enters the cell with `.expect("stack cell contended")` — fail-fast turns a design error into an accounted drop, per the BusyCell discipline.

**Hooks registration** (devpipe.rs:98-123 + netif.rs:374-395 — identical shape):
```rust
#[derive(Clone, Copy)]
pub struct EtherHooks {
    pub current: fn() -> u64,
    pub block: fn(),
    pub wake: fn(&[u64]),
}

static HOOKS: Lock<Option<EtherHooks>> = Lock::new("devether", None);

pub fn set_hooks(hooks: EtherHooks) {
    let node = LockNode::new();
    let mut guard = HOOKS.lock(&node);
    assert!(guard.is_none(), "devether: hooks registered twice");
    *guard = Some(hooks);
}
```
devip registers its own triple in `aarch64/src/main.rs` beside devether's (main.rs:501-506), reusing `proc::pipe_current_pid`/`pipe_block_current`/`pipe_wake` or devip-named equivalents (agent discretion per CONTEXT).

**Fixed-capacity waiter set** (netif.rs:319-349, `WaitSet`) — no allocation anywhere in the IRQ-side drain; pid-0-empty slots; `register` + `take` into a caller's stack buffer. The conv struct (not the waker) owns the waiter set per RESEARCH's waker-bridge note.

**The three drive legs** (RESEARCH Pattern 1): RX leg = woken context runs `drive_stack()`; timer leg = `poll_at()` → `AtomicU64` tick deadline, swept in `timer.rs tick()` beside `proc::tick_timeouts(n + 1)` (timer.rs:123); local-activity leg = one poll per mutating file op. The stack-debt flag drains on the next DEV_IP-dispatched syscall.

---

### `port/src/devip/phyadapt.rs` (smoltcp `Device` impls)

**Analogs:** netif.rs `FrameStore` (fixed-pool store) for the LoopbackDev's bounded queue; `devether.rs` `EtherDriverOps`/`tx_post` seam for EtherDev's TX path. **No in-tree `Device` impl exists** — the RESEARCH §Code Examples skeleton is probe-compiled green and is the starting point:

**Fixed-pool frame store** (netif.rs:221-261):
```rust
struct FrameStore {
    bytes: Vec<u8>,          // slots × ETHERMAXTU, preallocated at clone-open
    free: Vec<u16>,          // free slot indices (a stack)
    queue: VecDeque<(u16, u16)>, // (slot, len) FIFO — whole frames, Qmsg
}
```
LoopbackDev: a bounded VecDeque of whole packets, transmit pushes, receive pops, MTU 65535/`Medium::Ip` (RESEARCH Pattern 3 — do NOT use `smoltcp::phy::Loopback`, it allocates per packet). EtherDev: `Medium::Ethernet`, MTU 1514 (INCLUDES the 14-byte header — RESEARCH's MTU-convention trap), `receive()` merges the 0x800/0x806/0x86dd netif conv queues in arrival order, `transmit()` posts through devether's registered `tx_post` op (devether.rs:111-124). The `/net/arp` snoop lives in this adapter's receive/transmit path, parsing with smoltcp's own `wire::ArpPacket`/ndisc codecs.

---

### `port/src/devip/waker.rs` (the one new unsafe site)

**Analog:** `../r9/port/src/mcslock.rs` unsafe-floor discipline + `../r9/UNSAFE-AUDIT.md`.

Rules (lib.rs:10-17 + UNSAFE-AUDIT.md counting rule):
- Every unsafe block carries a SAFETY comment immediately preceding it — lint-enforced (`clippy::undocumented_unsafe_blocks = "deny"`, workspace Cargo.toml:11-12), never the honor system.
- The module does NOT carry `#[forbid(unsafe_code)]`; its lib.rs entry documents WHY beside `globalalloc`/`mcslock` (lib.rs:10-17 comment shape).
- One SAFETY comment per site; the UNSAFE-AUDIT.md entry joins the reconciliation table.
- Shape per RESEARCH: a tiny RawWaker vtable whose `wake` reads a per-socket wakee-pid slot and calls the arch `wake(&[pid])` hook; the conv owns re-registration (one-shot wakers — re-register after EVERY wake; spurious wakes always re-check `can_recv`/`can_send`).

---

### `port/src/devip_tests.rs` (host-side pins)

**Analog:** `../r9/port/src/devether_tests.rs` — exact match. smoltcp compiles for the host (RESEARCH probe-verified), so the ENTIRE devip↔smoltcp mapping pins host-side before any guest boot.

**Crate-root placement** (lib.rs:79-88):
```rust
// The devether/netif pins live at the crate root for the same reason:
// the frame-store drive and the stub wake log need alloc/std while
// port::devether/port::netif keep their unsafe-free forbid attributes.
#[cfg(test)]
mod devether_tests;
```

**Stub hooks over test-module statics** (devether_tests.rs:49-66):
```rust
const STUB_PID: u64 = 42;
static WAKE_LOG: Mutex<Vec<u64>> = Mutex::new(Vec::new());

fn stub_current() -> u64 { STUB_PID }
fn stub_block() {}
fn stub_wake(pids: &[u64]) {
    WAKE_LOG.lock().unwrap().extend_from_slice(pids);
}
```

**Publish-once discipline warning** (rx_chain_tests.rs:18-25 header): `netif::publish` and `set_hooks` assert once-per-process — a test binary gets ONE publish. devip_tests plans its seams accordingly: drive the seam's primitives directly, keep one publish per test binary, and pin behaviorally (pop_frame-style surface assertions, devether_tests.rs:142-149) rather than by private-field reads.

**Pin list** (RESEARCH line 337): (a) waker bridge fires on RX/close/buffer-space; (b) register→wake→re-register lost-wakeup shape; (c) deadline arming from poll_at; (d) accept-pool demux (two SYNs → two pool members); (e) udp headers-mode 52-byte Udphdr synthesis from UdpMetadata; (f) tcp state-name table vs `tcpstates[]`; (g) stack-debt drain; (h) FIRST pin of the phase: the DHCP-shaped exchange (OQ-1 — bind 68, broadcast TX, no iface addr).

---

### devcons `random`/`urandom` (D-17 minimal CSPRNG device)

**Analog:** `../r9/port/src/devcons.rs` — QID_TIME + FILES table + read-dispatch arms (devcons.rs:139-200, 365-458).

```rust
const FILES: [(&[u8], u64, u32); 7] = [
    (b"cons", QID_CONS, 0o666),
    (b"null", QID_NULL, 0o666),
    // ...
    (b"time", QID_TIME, 0o664),
];
```
Add `random`/`urandom` entries (0666/0444 per contract-doc decision), QID constants, read arms (urandom = non-blocking read from the DTB-seeded pool; random = same this era, loudly documented minimal — Phase 11 hardens). The DTB seed read uses `port/src/fdt.rs`'s `find_by_path("/chosen")` + `property_value_bytes` (fdt.rs:442, 154) — qemu's `-M virt` DTB carries `rng-seed` in /chosen; CNTFRQ/ticks mixing is the fallback. Every read re-seeds/mixes (never the same bytes twice); the file never errors. ipconfig's `truerand()` opens `/dev/random` once at startup — the minimal device unblocks it unmodified (D-09/D-17).

---

### r9-side modified-file arms (all one-shape insertions)

**`namec.rs` registry + dispatch arms** — copy the DEV_ETHER set:
```rust
// registry_attach (namec.rs:75):
'l' => crate::devether::attach(spec),
// → add:  'I' => crate::devip::attach(spec),

// dev_walk (namec.rs:115):
DEV_ETHER => crate::devether::walk(c, comp),
// open_chan (namec.rs:336), stat_chan (namec.rs:361): same shape.
// close_chan (namec.rs:405-412): COPEN-gated close arm —
        DEV_ETHER => {
            if c.copen {
                crate::devether::close(c);
            }
        }
```

**`syscall.rs` pread/pwrite arms** (syscall.rs:292-297 write, 383-388 read):
```rust
        DEV_ETHER => {
            // devether: the netif file contract — dir listings, the
            // blocking whole-frame data read (Qmsg; ...), ctl's conversation id, ...
            port::devether::read(&mut chan, dst.as_slice_mut(), off)
        }
```
devip's arms carry a one-line comment naming the contract (clone/ctl/data/listen/status, blocking reads via the seam).

**`main.rs` hook registration** (main.rs:501-506) — cfg-gated like devether:
```rust
    #[cfg(dev_ether)]
    port::devether::set_hooks(port::devether::EtherHooks {
        current: proc::pipe_current_pid,
        block: proc::pipe_block_current,
        wake: proc::pipe_wake,
    });
```
devip's registration gets `#[cfg(dev_ip)]` — which MUST be predeclared in workspace `Cargo.toml:29-35` check-cfg (the `cfg(dev_ether)` line is the precedent; the comment block at Cargo.toml:14-27 documents the rule).

**`timer.rs` tick arm** (timer.rs:114-130) — the net deadline sweep rides inside the `#[cfg(dev_exec)]` block beside `tick_timeouts`, gated `#[cfg(dev_ip)]`, checking `now >= net_deadline` and waking devip waiters (RESEARCH A5: one-shot arming over the existing 1 ms periodic tick; the literal-CNTP-rearm alternative was researched and rejected — record it in the contract doc).

**`pci.rs` IRQ interest wake** (pci.rs:875-909) — the ether adapter's "RX pending" interest hooks where `rx_ingress` is already called; a WAKE, never a poll (RESEARCH Pattern 1 leg 1). Copy the existing call-site discipline: no port mcslocks from IRQ context, BusyCell handoff only, wake after guard drop.

**`port/Cargo.toml` smoltcp entry** — the miniz_oxide precedent (port/Cargo.toml:8-13) of no_std feature pinning with a comment:
```toml
miniz_oxide = { version = "0.9", default-features = false, features = ["with-alloc"] }
```
smoltcp entry per RESEARCH §Standard Stack (default-features = false + the locked feature list; `socket-dhcpv4` NEVER in the set — D-09). Commit `../r9/Cargo.lock` in the same commit (D-02).

---

### Harvey-side: `build/aarch64/tcpblast.c` + echo payloads (gate binaries)

**Analog:** `../harvey/build/aarch64/netblast.c` — exact. The whole file is the pattern (71 lines):

**Header comment naming the requirement + TCG scaling + loud-failure posture** (netblast.c:4-19):
```c
/*
 * netblast — the D-08 TX-bulk generator, serving NETDRV-04's wedge
 * hunt: loop whole-frame writes to a devether conversation's data file
 * as fast as the ring drains.  argv[1] is the clone-opened conversation
 * id ...; argv[2] is an optional frame count
 * (default 256, TCG-sane).  ... Every write must return exactly
 * 60; a short or failed write is a loud error.  No sleeps, no pacing —
 * when the TX ring is full the write blocks on the driver's wake hook,
 * so a stalled retire wedges the binary and the gate's timeout catches
 * it loudly.
 */
```

**Plan 9 C shape** (netblast.c:40-71) — `u.h`/`libc.h` only, usage-to-stderr + `exits("usage")`, `sysfatal` with `%r`, fprint(2) + `exits("errors")` on failure, success marker line on stdout:
```c
void
main(int argc, char *argv[])
{
	char path[64];
	long n, i;
	int fd;

	n = DEFAULTN;
	if(argc < 2 || argc > 3){
		fprint(2, "usage: netblast conv [count]\n");
		exits("usage");
	}
	// ...
	fd = open(path, OWRITE);
	if(fd < 0)
		sysfatal("netblast: open %s: %r", path);
	for(i = 0; i < n; i++){
		if(write(fd, frame, FRAMESZ) != FRAMESZ){
			fprint(2, "netblast: write failed at %ld: %r\n", i);
			exits("errors");
		}
	}
	print("netblast: sent %ld frames\n", n);
	exits(nil);
}
```
tcpblast: dial/announce via **unmodified libc** (`dial("tcp!...")`, `announce`+`listen`+`accept` — the file tree is the whole ABI), bidirectional loop, TCG-scaled counts, marker vocabulary is agent discretion (CONTEXT). Echo payloads: udp echo + in-guest tcp/7 echo server, same shape. aan compiles from `sys/src/cmd/aan.c` unmodified (payload rule, not a new C file).

---

### Harvey-side: `subset-build.sh` payload lines

**Analog:** in-file — the insertion-only additive block pattern (subset-build.sh:255-277) and the `cmd/*:target` per-target install rule (224-246):

```sh
# 07-04: netblast, the D-08 TX-bulk generator, joins the image supply
# chain beside stacksmoke (same compile/harvest shape)
echo "== boot chain: netblast (07-04, D-08)"
(cd "$HARVEY/build/aarch64" && objtype=aarch64 7c -FTVw netblast.c \
	&& objtype=aarch64 7l -o "$HARVEY/build/aarch64/out/netblast" netblast.7 \
	&& rm -f netblast.7) || fail "netblast"
```

```sh
	cmd/*:*)
		# dir:target — per-target install in the dir (the mkmany
		# %.install rule), never the whole dir TARG.
		dir="${entry%%:*}"
		tgt="${entry##*:}"
		(cd "/sys/src/$dir" && objtype=aarch64 mk "$tgt.install") || fail "cmd $entry"
		art="/aarch64/bin/${dir#cmd/}"
```
Rules to copy: (1) additive lines with a dated phase-tag comment (`# 08-NN: ...`) — never fold into the tier-1 loop (tier files stay pure, D-12); (2) `|| fail "<name>"` on every step; (3) full-tree binaries (ipconfig/ping/aan) build via targeted `objtype=aarch64 mk <tgt>.install` in their dirs — NOT tier-allowlist appends; (4) the manifest/magic asserts at the script tail (subset-build.sh:405-408 — the `00008c47` od check) gain one line per new payload; (5) ipconfig links libip (already aarch64-green) — verify the closure when adding its rule.

---

### Harvey-side: gate rc fixtures (`root-tree/lib/`)

**Analog:** `../harvey/build/aarch64/root-tree/lib/net-sweep.rc` — exact. The whole 109-line file is the pattern:

- Sourced-not-typed discipline header; usage switch with digit-count validation (net-sweep.rc:21-42).
- Every step `|| { echo netsweep: FAIL <stage> >[1=2]; exit <stage> }` — a gate never hangs silently (44-107).
- `test -e` before redirects (an uncatchable rc Xerror otherwise) (65-68).
- Marker vocabulary: `netsweep: FAIL <stage>` fast-fail, `netsweep: NET-DONE` completion — the r9 xtask keys on exactly these prefixes; the Phase-8 scripts pick their own prefix (e.g. `netgate: …`) and the xtask profile asserts it.
- No timing assumptions, no sleeps; TCG-modest counts with 1-3 digit validation.

---

### Harvey-side: composed expect smoke (D-13 sibling)

**Analog:** `../harvey/build/aarch64/smoke-r9.expect` — exact. Key patterns:

**Env contract + hygiene** (smoke-r9.expect:68-84): required env vars checked loudly before spawn (exit 2 on contract violation); fresh capture rm -f'd per run.

**Spawn shape with the always-on netdev** (smoke-r9.expect:93-101):
```tcl
spawn $QEMU -M $R9_MACHINE -cpu $R9_CPU -smp 1 -m 1024M \
    -accel $R9_ACCEL \
    -kernel $env(R9_KERNEL) \
    -dtb $env(R9_DTB) \
    -device loader,file=$env(R9_INITRD),addr=0x48000000 \
    -netdev user,id=n0 \
    -device virtio-net-pci-non-transitional,netdev=n0 \
    -nographic \
    -name $run_tag
```

**Staged assertions + two-occurrence echo discipline** (smoke-r9.expect:119-153): `proc stage {name re}` with timeout/eof naming the stage; `expect_before` panic abort (130-136); typed commands appear on the wire BEFORE output (cooked echo) — response markers are staged two-occurrence and sweep captures strip through the first newline (`strip_echoed_line`, 164-168). TCG-sized 600 s per-stage timeouts. The net sibling drives: `ping 127.0.0.1`, udp echo, tcp/7 echo, ipconfig static + dhcp — each a named stage, presence-asserted, never interleaving order.

---

### Harvey-side: CI + flake

**`.github/workflows/r9-shell.yml`** — the whole 40-line file is the pattern: one job, `timeout-minutes: 90` with the TCG comment, pinned nix-installer SHA, `nix run .#<app>`. D-16's short net gates extend as a new step/job in the same shape (agent discretion on placement); the sustained soak is documented-manual (the exact dev-host command lands in 08-EVIDENCE.md — D-16's discipline).

**`flake.nix`** — the `smokeR9Pkg`/`tier1R9Pkg` `mkScriptApp` entries (flake.nix:106-127) are the pattern for a net-smoke app: same env export (`R9_KERNEL_PKG` from `r9.packages.${system}.kernel-virt`), same composed pipeline, new wrapper script. `flake.lock` r9 pin advance is its OWN commit with fresh hosted CI (carried-forward rule) — and the r9 kernel-virt cargo vendor hash re-rivals on the smoltcp lockfile change (RESEARCH Pitfall 16 note).

---

## Shared Patterns

### Error convention: `Result<T, &'static str>` with device-prefixed strings
**Source:** `../r9/port/src/dat.rs:10-13` + `lib.rs:90`
**Apply to:** every devip module + the syscall arms.
```rust
/// A device/kernel error carrying a static message — the port crate's
/// `Result<T, &'static str>` convention applied at the Dev-trait boundary
pub struct Error(pub &'static str);
// lib.rs:90:
pub type Result<T> = core::result::Result<T, &'static str>;
```
Error strings are `"devip: …"`-prefixed statics; devip-exact user-visible strings ("unknown control request", "not announced", "listen hungup", "cannot bind null device"…) come from the contract doc's error table (RESEARCH §Error vocabulary). **A smoltcp panic must never cross the file boundary** — every smoltcp call site maps onto a devip error string; non-unicast addresses / port 0 / overflow are validated BEFORE the smoltcp call (RESEARCH P-F, V5).

### Locking: mcslock `Lock` + `LockNode` + `BusyCell`, documented invariants
**Source:** `../r9/port/src/mcslock.rs` (Lock:100-129, BusyCell:170-204) + netif.rs:42-69 header
**Apply to:** stack.rs, conv.rs, arp.rs, netlog.rs, devrandom.
```rust
let node = LockNode::new();
let mut guard = NETIF.lock(&node);
```
Rules: LEAF locks only (never held across block()/sched(), never from IRQ context); nesting orders documented and one-directional; BusyCell for IRQ↔syscall handoff with fail-fast `enter()`; no allocation in IRQ context (frame/socket stores preallocated at clone-open in syscall context); wakes ALWAYS after guards drop.

### Block/wake hooks: dependency-injected `{current, block, wake}`
**Source:** `../r9/port/src/devpipe.rs:98-123`, `netif.rs:374-395`; registration at `aarch64/src/main.rs:485-506`
**Apply to:** devip's blocking reads/connect/listen + the seam's waker bridge.
Register once pre-exec; assert-once; lazy `hooks()` accessor returns Err when unregistered. Register-waiter → drop-lock → block() is wakeup-loss-free because syscalls run IRQ-masked (the Phase-4 invariant, devpipe.rs:16-22).

### cfg gating: `cfg(dev_ip)` predeclared + virt-only
**Source:** `../r9/Cargo.toml:28-35`, `aarch64/src/main.rs:501` (`#[cfg(dev_ether)]`)
**Apply to:** every devip touch of shared code (main.rs, timer.rs, pci.rs, syscall.rs arms can stay ungated — they dispatch by dev index — but the hook registration and timer/IRQ arms are cfg-gated). The xtask config TOML emission is the first appearance; predeclare in the workspace lints before the first code use. raspi3b/raspi4b/x86_64/riscv64 stay green — the port crate is shared (carried-forward).

### Evidence: serial markers + negative scans + TCG-sized timeouts
**Source:** `../r9/xtask/src/main.rs:775-958` (marker constants, `negative_scan_hit_for`, `contains_marker`), `main.rs:1514-1627` (finish/fail)
**Apply to:** the new net profiles.
Presence, never interleaving order; one total wall-clock budget (`R9_BOOT_TEST_TIMEOUT`, never per-marker timings); the negative scan (panic/Unhandled/unhandled-IRQ/note-timeout/unimplemented) fails any profile; new markers are const strings named beside the existing set (main.rs:862-875 is the net-profile block); the typed-line echo can never false-match because assertions key on the sweep's prefixed lines (main.rs:1465-1473).

### Host pcap assertion
**Source:** `../r9/xtask/src/pcap.rs` + `main.rs:1568-1598` (the Net finish arm) + `config.rs:576-584` (`apply_virt_netdev` filter-dump stanza)
**Apply to:** the TCP/ARP/soak gates — host-side ground truth over this run's `target/boot-test-<config>.pcap` (per-run path, never a stale capture). The netdev stanza is already always-on; no Harvey-side netdev change this phase.

---

## No Analog Found

| File | Role | Data Flow | Reason | Planner Guidance |
|------|------|-----------|--------|------------------|
| `port/src/devip/route.rs` | model | request-response | No route table exists in r9 today (devether has none; the donor `iproute.c` forest is C). | Use the devip.rs ctl-parse + netif stats-content shapes for the file surface; semantics from RESEARCH §iproute (verbs/flags/format verified against `sys/src/9k/ip/iproute.c`). smoltcp `Routes` mirrors the default route only. |
| `port/src/devip/arp.rs` | model | event-driven snoop | smoltcp 0.14 has NO neighbor-cache accessor (verified) — the snoop-fed view table is a new design with no in-tree or in-donor Rust analog. | RESEARCH §Pattern 4 is the design authority (90-byte lines, NCACHE=256 LRU, flush/add/del verbs, wire::ArpPacket/ndisc snoop in the adapter). Record the static-add divergence in the contract doc. |
| `port/src/devip/stack.rs` (composition) | service | event-driven | The primitives all have analogs (above), but the single-ownership Interface+SocketSet+drive() composition is new — no r9 module owns an external engine today. | RESEARCH §Pattern 1 is the design authority; land the host-side pins (devip_tests) BEFORE any boot gate depends on the seam (CONTEXT watch-out). |
| `port/src/devip/phyadapt.rs` (Device impl) | driver adapter | streaming | No smoltcp `Device` impl exists in-tree (first external-crate integration in port/). | The RESEARCH §Code Examples skeleton compiled green on r9's pinned toolchain — start there; FrameStore (netif.rs:221-261) supplies the fixed-pool shape. |
| addressing seam data file (D-11) | config | — | First single-source-as-data file in the harness (Phase 7 D-09's "minimal wiring now" was flags, not a data file). | Agent discretion on format/location (CONTEXT). Constraint locked: ONE committed file defines guest/host addresses per leg; baked ndb/cpurc fragments and gate assertions derive from it; no scattered `10.0.2.x` hardcodes. Phase 9 lifts it into TOML. |
| `docs/devip-contract.md` (D-03) | doc | — | r9 has no `docs/` dir; no prior contract doc exists (HowItWorks.md is prose architecture, UNSAFE-AUDIT.md is an inventory). | New pattern this phase (CONTEXT: contract-doc-as-spec). It lands FIRST and is reviewed before gate-writing; enumerates the full documented devip surface from `sys/man/3/ip`+`sys/man/2/ip`+`sys/man/8/ipconfig` × `sys/src/9k/ip/` (RESEARCH §The devip Contract Surface is its table of contents, with the [man-vs-src] divergences flagged). |

## Metadata

**Analog search scope:** `/Users/neilskinner/code/r9/port/src/` (all 32 modules), `/Users/neilskinner/code/r9/aarch64/src/` (main, syscall, platform/virt/{timer,pci}), `/Users/neilskinner/code/r9/xtask/src/`, `/Users/neilskinner/code/r9/{Cargo.toml,UNSAFE-AUDIT.md}`, `/Users/neilskinner/code/harvey/build/aarch64/`, `/Users/neilskinner/code/harvey/{nix,.github/workflows,flake.nix}`, Harvey `sys/src/9k/ip/` + `sys/man/` + `sys/src/cmd/` + `sys/src/libc/9sys/` (existence-verified, read-only semantics references — content already extracted into 08-RESEARCH.md with line numbers)
**Files scanned:** 24 read directly (devether.rs, netif.rs, devpipe.rs, dat.rs, lib.rs, namec.rs, mcslock.rs, devcons.rs, devether_tests.rs, allocator_tests.rs, rx_chain_tests.rs, syscall.rs, main.rs, timer.rs, pci.rs, xtask/main.rs, xtask/config.rs, port/Cargo.toml, Cargo.toml, UNSAFE-AUDIT.md, fdt.rs headers; Harvey: netblast.c, subset-build.sh, net-sweep.rc, smoke-r9.expect, smoke-r9.sh, r9-shell.yml, flake.nix)
**Early stopping:** stopped at the pattern-sufficiency point — every new file has either an exact/role-match analog above or an explicit No-Analog entry with guidance
**Pattern extraction date:** 2026-11-05
