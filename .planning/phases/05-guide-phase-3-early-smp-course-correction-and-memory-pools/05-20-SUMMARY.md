---
phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools
plan: 20
subsystem: kernel-9p
tags: [x86_64, aarch64, 9p2000.aead, loopback, fidtable, session, boot-switch, smp-13, d-28]

# Dependency graph
requires:
  - phase: 04
    provides: "9P2000.aead transport (Session/Handshake/Handoff), the plain devmnt client, the Dialect negotiation seam"
  - phase: 05-06
    provides: "the sharded generational slab FidTable (synthetic tier 4x256) with lock-free resolve and owning-client check"
provides:
  - "port/src/aead_loop.rs: boot-switch-gated in-kernel 9P2000.aead loopback server on real FidTable + Session + Arc/Lock duplex link"
  - "port/src/devmnt.rs: Mnt::sealed_session hook (client engine over already-negotiated aead halves)"
  - "port/src/namec.rs: '#M' registry arm delegating to aead_loop::attach_root (Err when dormant)"
  - "aarch64/x86_64 boot wiring: exact aeadloop=1 token parsing from DTB bootargs / Multiboot2 cmdline"
affects: [05-21 (2-CPU stress gate), 05-27 (SMP-08 gate), 06 (distributed 9P / real server end)]

# Actuals (#2632) — chars/4 over the realized diff, same scale as the plan estimate (48000, confidence: low).
actuals:
  tokens: 11612
  tasks: 3
  commits: 4
  plan_head_before: cce4c521569cf1068d788025bd5b3c20bbe0cab6
  plan_head_after: 91f11dc90538e9967bab8145b2380de23dc1fa96

tech-stack:
  added: []
  patterns:
    - "Boot-switch-only kernel server: exact whitespace token parse; default boot allocates nothing and the device attach returns the donor Enodev shape"
    - "In-kernel loopback as a synchronous coroutine: one client write drives exactly one server dispatch over an Arc+Lock duplex byte link"
    - "Sealed MntIo adapter: the plain Mnt client's frames are sealed on write and opened+unpadded on read, tracking server-assigned fid handles by tag"
    - "Real FidTable (synthetic tier) behind a ranked DevTop lock, allocating/resolving/clunking one server entry per 9P fid operation"

key-files:
  created:
    - port/src/aead_loop.rs
  modified:
    - port/src/lib.rs
    - port/src/devmnt.rs
    - port/src/namec.rs
    - aarch64/src/main.rs
    - x86_64/src/main.rs
    - port/tests/aead_fid.rs

key-decisions:
  - "The kernel's own plain devmnt::Mnt drives the loopback through a sealed MntIo adapter (SealedWriteIo/SealedReadIo) rather than a new sealed client engine: the adapter seals outgoing base 9P frames and reconstructs unpadded base 9P replies, so the whole existing client path (attach/walk/open/read/clunk) is reused unchanged."
  - "The server is built behind a ranked LockRank::DevTop lock; the byte queues and the client session/handle maps are unranked so no two DevTop locks ever nest on one CPU (FidTable::alloc/resolve/clunk take rank 40, strictly above DevTop)."
  - "Topen answers with the request's stateful handle (not a stateless cap) so the plain Mnt client can carry the same handle into Tread; the gate exercises FidTable alloc/resolve/clunk, which is what SMP-13 specifies."
  - "The loopback is published as a single process-global root chan; attach_root clones it with a real zero-name Twalk (devmnt::clone_chan), never a bare dup, so namec's clunk_if_last sweep cannot retire the stored root fid."

patterns-established:
  - "Boot-switch gate: parse_boot_switch -> start -> maybe_start_from_cmdline publishes; attach_root is the only consumer"
  - "Sealed wire data bodies are page-padded by msg::write_padded, but a plain client requires count == remaining, so the read adapter re-emits Rread unpadded"

requirements-completed: [SMP-13]

coverage:
  - id: D1
    description: "Exact aeadloop=1 parsing gates the in-kernel 9P2000.aead loopback server; a default boot allocates nothing and #M attach returns 'namec: unknown # device'"
    requirement: SMP-13
    verification:
      - kind: unit
        ref: "port/src/aead_loop.rs#aead_loop_parse_boot_switch_is_exact_token,aead_loop_dormant_default_boot_exposes_nothing"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_loop_boot_switch_is_exact_and_starts_nothing_by_default"
        status: pass
    human_judgment: false
  - id: D2
    description: "When active, real FidTable + Session serve data0..7 over the loopback and the kernel Mnt client reads byte-exact 4 KiB files via attach/walk/open/read/clunk"
    requirement: SMP-13
    verification:
      - kind: unit
        ref: "port/src/aead_loop.rs#aead_loop_enabled_serves_deterministic_files"
        status: pass
      - kind: unit
        ref: "port/tests/aead_fid.rs#aead_loop_public_api_reads_a_deterministic_file"
        status: pass
    human_judgment: false
  - id: D3
    description: "Two threads concurrently drive opens/reads/clunks over a shared FidTable with exact payloads, no leaked fids at barrier boundaries, and wrong-client handles rejected"
    requirement: SMP-13
    verification:
      - kind: unit
        ref: "port/src/aead_loop.rs#aead_loop_two_threads_share_one_table_without_leaks"
        status: pass
    human_judgment: false
  - id: D4
    description: "Both architectures parse aeadloop=1 from their boot command line and log the active device; non-dev_exec aarch64 virt configs compile without referencing platform::bootconf"
    requirement: SMP-13
    verification:
      - kind: integration
        ref: "cargo xtask check; cargo xtask clippy --arch aarch64 --config virtshell; --config virt; --arch x86-64 --config q35cpu"
        status: pass
    human_judgment: true
    rationale: "Compile/clippy proven; the live -smp 2 boot that exercises '#M/data0..7' from userland is the 05-21/05-27 stress gate, not this plan."

duration: 6min
completed: 2026-11-27
status: complete
---

# Phase 5 Plan 20: Boot-Switch In-Kernel 9P2000.aead Loopback Server Summary

**A real `FidTable`+`Session` `9P2000.aead` server now runs inside the kernel over an `Arc`+`Lock` duplex link, mounts through the existing plain `devmnt` client behind a sealed `MntIo` adapter, and is reachable as `#M/data0..7` only when the exact `aeadloop=1` boot token is present.**

## Performance

- **Duration:** ~6 min
- **Started:** 2026-11-27T00:19:53Z
- **Completed:** 2026-11-27T00:26:05Z
- **Tasks:** 3
- **Files modified:** 7 (1 created)

## Accomplishments

- `port/src/aead_loop.rs` is new: `parse_boot_switch` (exact whitespace token), `start`/`maybe_start_from_cmdline`, `attach_root`, and the loopback itself — a `LoopServer` holding a real `Session` finished from the canonical plain handshake and a real `FidTable::synthetic` (4 shards x 256 slots, D-12), served over an `Arc<LoopLink>` duplex byte link (the host fixture's `Rc<RefCell>` replaced with `Arc`+`Lock`, server core at `LockRank::DevTop`).
- The kernel's own `devmnt::Mnt` client mounts the loopback: it negotiates `Dialect::Aead`, finishes the transcript-bound handshake, then drives the sealed connection through `SealedWriteIo`/`SealedReadIo`, which seal each base 9P frame and reconstruct unpadded replies, tracking server-assigned 8-byte fid handles by reply tag.
- `#M` serves a deterministic synthetic tree (`data0`..`data7`, 4 KiB each) with `alloc`/`resolve`/`clunk` on the real `FidTable` on every operation; `attach_root` returns a fresh zero-name-Twalk clone, so `namec`'s sweep never retires the stored root fid.
- `port/src/namec.rs` gains the `'M'` registry arm, so userland reaches `#M/dataN` directly and survives `init`'s `rfork(RFCNAMEG)`/`newns` with no writable `/n` in read-only `bootfs.paq`.
- Both architectures read their boot command line (`DTB /chosen/bootargs` on aarch64, Multiboot2 `facts.cmdline()` on x86_64) inside the `dev_exec` exec block and start the server + log `aeadloop: active device=#M dialect=9P2000.aead` only on the exact token.
- Default boots are unchanged: no allocation, no server, `#M` attach returns `namec: unknown # device`, and the kernel still speaks plain `9P2000`.

## Task Commits

Each task was committed atomically (Task 1 followed RED → GREEN):

1. **Task 1 RED: boot-switch/dormancy/read pins** — `4d46c21` (test)
2. **Task 1 GREEN: loopback server + devmnt hook + namec #M arm** — `e213553` (feat)
3. **Task 2: aeadloop=1 wiring on aarch64 and x86_64** — `66f3882` (feat)
4. **Task 3: public-API loopback boot-switch and read pins** — `91f11dc` (test)

**Plan metadata:** committed separately below.

## Files Created/Modified

- `port/src/aead_loop.rs` — the loopback server, the duplex link, the sealed client adapter, the boot-switch parser, the global root and the `#M` attach arm.
- `port/src/lib.rs` — `#[forbid(unsafe_code)] pub mod aead_loop;`.
- `port/src/devmnt.rs` — `Mnt::sealed_session` (client engine over already-negotiated aead halves).
- `port/src/namec.rs` — `'M' => crate::aead_loop::attach_root(spec)`.
- `aarch64/src/main.rs` — read DTB bootargs, gate `maybe_start_from_cmdline`, reuse `BootArgs` for `exec_bootstrap`.
- `x86_64/src/main.rs` — read `facts.cmdline()`, gate `maybe_start_from_cmdline`, then `exec_bootstrap`.
- `port/tests/aead_fid.rs` — public-API boot-switch/dormancy and deterministic-read integration pins.

## Decisions Made

- **Sealed `MntIo` adapter instead of a second client engine.** The real `devmnt::Mnt` reproduces the userland path; a small adapter seals/opens frames and maps client fids to server handles. This exercises the actual kernel client code the stress gate will drive.
- **`Topen` returns the stateful handle**, so the plain client can use it on `Tread`; the gate is about `FidTable` alloc/resolve/clunk, which this preserves.
- **Server core ranked `DevTop`, byte queues/client maps unranked** — avoids same-rank nesting while keeping the FidTable's rank-40 acquisitions strictly above it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plaintext handshake frames polluted the sealed queue**
- **Found during:** Task 1 (GREEN)
- **Issue:** The raw `LoopMntIo` pushed every write into the server's `c2s` queue, but the plain handshake was answered in place, so the queued plaintext frames desynchronised the first sealed request's framing.
- **Fix:** Queue only sealed packets; handle plaintext frames in place without queuing.
- **Files modified:** `port/src/aead_loop.rs`
- **Verification:** `cargo test -p port aead_loop`
- **Committed in:** `e213553`

**2. [Rule 1 - Bug] Sealed data bodies are page-padded, but the plain client requires `count == remaining`**
- **Found during:** Task 1 (GREEN)
- **Issue:** `msg::write_padded` pads `Rread` data to a whole page on the sealed wire, while `devmnt::Mnt::read` (a plain client) rejects `count != remaining`.
- **Fix:** The read adapter decodes the message and re-emits `Rread` as unpadded `count[4] data[count]`; the write adapter pads outgoing `Twrite`/`Tstream` data to the same page shape.
- **Files modified:** `port/src/aead_loop.rs`
- **Verification:** `cargo test -p port aead_loop`, `cargo xtask test-9p`
- **Committed in:** `e213553`

**3. [Rule 3 - Blocking] Plan line references were stale**
- **Found during:** Task 2
- **Issue:** The plan cited `platform::bootconf::read_bootargs(&dt)` at line 467 and an `exec_bootstrap(initrd_img, bootargs)` signature; the actual call is at line 506 with `exec_bootstrap(&dt, initrd_img, bootargs)`.
- **Fix:** Bound `bootargs` inside the same `#[cfg(all(dev_exec, any(not(dev_churn), dev_bootchain)))]` block and passed it to the real signature.
- **Files modified:** `aarch64/src/main.rs`
- **Committed in:** `66f3882`

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 blocking).
**Impact on plan:** All required for the sealed loopback to function through the real client path; no scope creep. The x86_64/aarch64 kernel bytes change only when a gate passes `aeadloop=1`; the default-boot path is untouched.

## Issues Encountered

- The repo's `println!` is the kernel console macro, so host-test debugging output is invisible; diagnosis relied on distinct `&'static str` error strings.
- The two-thread test cannot read a shared `FidTable`'s live count mid-flight (the sibling thread's transient entry skews it); the test uses a barrier so both threads are idle before the exact count assertion.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: gate-only-server | port/src/aead_loop.rs, port/src/namec.rs, aarch64/src/main.rs, x86_64/src/main.rs | A hardcoded test secret and an in-kernel loopback mount are reachable only behind the exact `aeadloop=1` boot token; default boots expose nothing (T-05-36 mitigated) |

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `SMP-13` wiring is complete and host-verified; the two-CPU stress gate (`SMP-08`, plans 05-21/05-27) can boot with `aeadloop=1` and drive `'#M/data0..7'` from userland on both architectures.
- A real `9P2000.aead` server end (non-loopback, authenticated peer) remains Phase 6 work (D-28 boundary).

## Self-Check: PASSED

- Created file `port/src/aead_loop.rs` exists.
- Commits `4d46c21`, `e213553`, `66f3882`, `91f11dc` are ancestors of HEAD.

---
*Phase: 05-guide-phase-3-early-smp-course-correction-and-memory-pools*
*Completed: 2026-11-27*
