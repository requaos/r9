//! devip — the IP protocol device `#I` (NETIP-02): the devip-exact `/net`
//! file tree over smoltcp 0.14 sockets (D-01) on the event-driven seam
//! (D-05).  Semantics donors: `sys/src/9k/ip/devip.c` + `sys/man/3/ip`
//! (read-only references — the C is never copied); the D-03/D-08 contract
//! doc `docs/devip-contract.md` is the design authority — every divergence
//! from the donor is enumerated and recorded there.
//!
//! MODULE SHAPE (08-02): this file is the module home ONLY.  The device
//! surface (attach/walk/open/stat/read/write/close, the qid scheme, the
//! conv tables, the namec/syscall/main wiring) lands in 08-04 — there is
//! no kernel surface to wire before then.  What exists this plan is the
//! SEAM, compilable standalone and host-pinnable:
//!
//! - `stack` — THE SEAM: single ownership of the smoltcp Interface +
//!   SocketSet + Device-adapter state behind a BusyCell; `drive_stack()`
//!   (bounded ingress via `poll_ingress_single` under an explicit
//!   `RX_DRIVE_CAP`, then egress, then maintenance); the `poll_at`-armed
//!   one-shot deadline (`NET_DEADLINE`, tick units) and the stack-debt
//!   flag (`STACK_DEBT`); the DevipHooks block/wake triple and the
//!   tick-source registration.
//! - `phyadapt` — the smoltcp `Device` adapters: the fixed loopback
//!   `LoopbackDev` (`Medium::Ip`, MTU 16384 — devip's loopback maxtu) and
//!   the test-only `ScriptedDev` RX/TX capture device the host pins drive.
//! - `waker` — the RawWaker bridge.  NOT declared here: it is the phase's
//!   ONE named unsafe-bearing devip module, and this file's
//!   `#![forbid(unsafe_code)]` (like the one on `pub mod devip;` in
//!   lib.rs) PROPAGATES to submodules — as `devip::waker` the named
//!   exception could not compile.  It is mounted at the crate root via
//!   `#[path = "devip/waker.rs"] pub mod devip_waker;` beside
//!   `globalalloc`/`mcslock` (lib.rs carries the comment).

#![forbid(unsafe_code)]

extern crate alloc;

pub mod phyadapt;
pub mod stack;
