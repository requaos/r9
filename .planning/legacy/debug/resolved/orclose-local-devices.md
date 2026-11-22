---
status: resolved
trigger: "Fix the ORCLOSE gap now: ORCLOSE must take effect for local devices on both arches, so that /srv files etc. are removed on close. Plan 20-14 owns aarch64 remove (syscall 25 is unimplemented); if ORCLOSE needs that remove path on aarch64, checkpoint the overlap before implementing. Arch-only, otherwise checkpoint with the diff. Plan 20-11 adds `T1-OK cexeccreate` concurrently; don't edit tier1-sweep.rc."
created: 2026-11-20T18:05:00Z
updated: 2026-11-20T20:30:00Z
---

## Current Focus

bug_class: Bohrbug (by reading: no code path records or acts on ORCLOSE for a local device)
hypothesis: H1 (by reading, unambiguous) — r9 never records ORCLOSE on a chan (no CRCLOSE twin on Chan; open_chan and sys_create drop it for local devices) AND devsrv/devenv have no remove op, so a local file opened/created ORCLOSE is never removed at its last close; devmnt is already fine (ORCLOSE rides the Topen/Tcreate wire byte, the server removes at clunk)
test: ALL DONE (option C) — rcloseprobe RED at 42203d8 → GREEN, HVF+TCG; host tests port/aarch64/x86_64 + port mutants; aarch64 TCG gates; x64 container gates + host tests; darwin + linux batteries; boot-iso re-derived and green at the committed HEAD; container removed
expecting: owner confirmation; then the push of r9 5629cf0 + fa38402, a lock-only pin move, and the combined arm64 HVF series at fa38402 (kernel-virt d8be7228… → bd0d9696…)
next_action: DONE — owner confirmed fixed (relayed by the coordinator); r9 pushed to fa38402; archived to resolved/ with the KB entry. Follow-ups owned elsewhere: 20-11 adds `T1-OK rcloseprobe`; 20-14's aarch64 sys_remove routes through port::namec::remove_chan
r9_commits: 5629cf0 fix (tree == tested 03c6038), fa38402 boot-iso bump — pushed by the coordinator (origin/main fa38402)
harvey_commits: 1e6d8e79f test(debug): rcloseprobe (+ subset-build.sh/mkimage.sh wiring); this file
arch_only_verdict: NOT POSSIBLE — (1) devsrv's registry is a private port static (port/src/devsrv.rs:59 `static SRV`) and EnvGroup's entries are private, so no arch code can remove an entry; (2) last-close detection needs a reference shared by the open's dups (Chan is copied by value per fd; arch code cannot add a Chan field); (3) the close paths that must act (exit drains, exec sweep, dup eviction, op-ref release in port/src/fdgrp.rs op_done) run port's close_chan
overlap_with_20-14: ORCLOSE does NOT need syscall 25 (REMOVE) or an aarch64 sys_remove; it needs the DEVICE remove ops (devsrv::remove, devenv::remove — new in port) that REMOVE("/srv/x"), REMOVE("/env/x") would also dispatch to. x86_64's remove routing is arch-local (x86_64/src/syscall.rs:898 remove_chan, DEV_CAP only); the draft adds a second, port-side routing for the close path (namec::remove_if_last_rclose: DEV_SRV, DEV_ENV). 20-14 must decide whether aarch64's sys_remove (and x86_64's remove_chan) gain DEV_SRV/DEV_ENV arms over these ops, or route through one shared port dispatch
related_session: resolved/create-ocexec-not-marked.md (archived 2026-11-20; MemPalace absent — knowledge-base.md is the only index)

## Symptoms

expected: (donor chan.c:1412/1518 + devsrv.c:287 srvclose + devenv.c:229 envclose) a file opened or created with ORCLOSE is removed when the LAST reference to its chan closes — e.g. lib9p postfd's `/srv/<name>` (OWRITE|ORCLOSE|OCEXEC) disappears when the server exits
actual: (reading) r9 records no remove-on-close for local devices; the /srv post outlives its server
errors: none (stale /srv and /env entries)
reproduction: create /srv/x (or /env/x) ORCLOSE, close the fd (or exit), stat the name — it is still there
started: since devsrv/devenv were ported (no remove arm ever existed)

## Eliminated

## Evidence

- timestamp: 2026-11-20T18:05:00Z
  checked: knowledge-base.md (4 entries incl. create-ocexec-not-marked)
  found: create-ocexec-not-marked's meta-lesson applies directly — "an fd-state bit carried in the mode word (OCEXEC, ORCLOSE) must be recorded on EVERY arm that yields a chan"; method notes for host-compiled probes and the amd64 container reused
  implication: known-pattern candidate (same class as the OCEXEC gap); test it first

- timestamp: 2026-11-20T18:05:00Z
  checked: donor sys/src/9k/port chan.c:1412-1413, 1518-1519; devsrv.c:109, 287-297, 324; devenv.c:229; sysfile.c:164
  found: Aopen and Acreate both set CRCLOSE when omode has ORCLOSE (after the device op); srvclose and envclose call srvremove/envremove when CRCLOSE is set (errors ignored — cclose can't fail); dev->close runs only at the LAST cclose (decref); srvopen refuses ORCLOSE on the #s dir (Eperm); srvwrite refuses to post an fd with CCEXEC|CRCLOSE
  implication: the contract is per-device remove at last close, gated by a chan flag

- timestamp: 2026-11-20T18:05:00Z
  checked: r9 42203d8 grep ORCLOSE/CRCLOSE/rclose (aarch64, x86_64, port) and every `fn remove`
  found: ORCLOSE appears only as the dat.rs constant and in devmnt's wire-byte comments (devmnt.rs:879, 1077); no chan flag; remove ops: only port/src/devcap.rs:359 `remove`, reached from x86_64/src/syscall.rs:898 `remove_chan` (arch-local routing: DEV_CAP only, every other device refuses); aarch64 has no REMOVE at all (plan 20-14)
  implication: devsrv and devenv cannot remove an entry today on either arch — the device half of ORCLOSE is missing in port/src, not just the flag

- timestamp: 2026-11-20T18:20:00Z
  checked: r9 42203d8 port/src/dat.rs Chan (fields, new, dup, op_clone, drop_op_ref), namec.rs open_chan/close_chan, devmnt.rs open/create mode byte + clunk_if_last, devsrv.rs (SRV static, create/open/write), devenv.rs (EnvGroup API), fdgrp.rs op_done, both arches' exit_tail + release_fd_group
  found: Chan has no rclose state; dup/op_clone copy by value (shared state rides Arcs — devmnt's fid Arc + clunk_if_last is the last-close model, incl. the op-ref corner in drop_op_ref); devmnt passes `(mode & 0xff)` on Topen and the low byte on Tcreate, so mounted ORCLOSE already works server-side; devsrv's SRV registry and EnvGroup.entries are private with no remove; both exit tails run release_fd_group (drained.drain(..): one chan consumed per close) BEFORE sweep_husk_payloads swaps the env group; fdgrp op_done calls drop_op_ref UNDER the fd-group lock
  implication: an arch-only fix does not exist; a port fix can follow the devmnt-fid pattern (an Arc token per ORCLOSE open, remove when the closing chan holds the last share)

- timestamp: 2026-11-20T18:40:00Z
  checked: draft fix in scratchpad/r9-rc (on 42203d8), patch scratchpad/rc/orclose-draft.patch — port: Chan.rclose: Option<Arc<()>> (new/dup/op_clone), namec::mark_fd_state (OCEXEC+ORCLOSE; open_chan uses it), namec::remove_if_last_rclose (DEV_SRV → devsrv::remove, DEV_ENV → devenv::remove, errors dropped) called first in close_chan and in drop_op_ref, devsrv::remove (+ the donor's Eperm for ORCLOSE on the #s dir), EnvGroup::remove + devenv::remove; arch: both create_new_chan call port::namec::mark_fd_state; 7 files +220/-25
  found: |
    cargo xtask test (darwin) rc=0, incl. namec_tests::{orclose_srv_entry_goes_at_the_last_close_only, orclose_remove_waits_for_an_in_flight_op_ref, orclose_env_key_goes_at_the_last_close}; cargo test -p aarch64 --bins 32/32 (31 + syscall::create_cexec_tests::created_orclose_srv_post_goes_at_the_last_close); x86_64 twin test added (not yet run — container leg); cargo xtask check rc=0, no new compiler warning (the one extra line is nix's dirty-tree notice)
    mutants (port, `-- orclose`): M0 close arm removed (= today) KILLED 2; M1 last-close guard removed KILLED 2; M2 mark not recorded KILLED 3
  implication: the draft is credible and tested on host; held at the checkpoint (port/src + 20-14 overlap) — no r9 or Harvey source change committed

- timestamp: 2026-11-20T18:55:00Z
  checked: owner decision (relayed by the coordinator)
  found: option C approved — the port/src change is approved for this scope: one shared device-remove dispatch in port (namec::remove_chan) used by the ORCLOSE close path; x86_64's remove_chan routes through it; 20-14 later adds only aarch64 sys_remove (do NOT add syscall 25 here); note the fd-group-lock risk in the session AND in a code comment
  implication: implement C on 42203d8; full verification; commit locally

- timestamp: 2026-11-20T19:05:00Z
  checked: option C on the draft (scratchpad/r9-rc, WIP 03c6038 on 42203d8) — new `pub fn namec::remove_chan` (DEV_CAP → devcap::remove, DEV_SRV → devsrv::remove, DEV_ENV → devenv::remove, else the loud `remove: device does not implement remove`); remove_if_last_rclose calls it for DEV_SRV|DEV_ENV only (the donor's capclose ignores CRCLOSE); x86_64 remove_chan = `port::namec::remove_chan(c).map(|_| 0)`; LOCK RISK comment at namec::remove_if_last_rclose and Chan::drop_op_ref; x86_64 tests adapted — remove_syscall_dispatches_and_refuses_loudly now removes `#c` (devcons has no arm; `#e` now has one), remove_round_trips_… gains the devsrv arm
  found: darwin cargo xtask test rc=0 (3 orclose port tests ok), cargo test -p aarch64 --bins 32/32, xtask check rc=0, clippy rc=0 (counts +1 = nix's dirty-tree notice before the WIP commit)
  implication: ready for the boot RED/GREEN

- timestamp: 2026-11-20T19:20:00Z
  checked: RED/GREEN — Harvey build/aarch64/rcloseprobe.c (new; legs srv / dup / exit / env + env-open, all checked per run), host-compiled (sha256 65177df7…), baked from scratch clone scratchpad/harvey-rc (c7044a68b; userland diff -r identical to the main checkout's 13:xx build incl. its guest-built cexecprobe -c 0f77c815…; mkimage.sh/subset-build.sh wired by scratchpad/rc/wire.py), bootfs sha256 3af47705…; driver scratchpad/rc/matrix.sh → scratchpad/probe/probe.sh; RED kernel = r9 42203d8 kernel-virt sw5pfm8m… (d8be7228…); GREEN kernel = WIP 03c6038 /nix/store/gi4qg8fxywj1il65n00bvxq8a04wrnw9-r9-kernel-virt-03c6038 (bd0d969651a6c7287940087d5e12fb5be88c8284704455f9010d85b583b18f81, 60678376 B)
  found: |
    MATRIX unfixed tcg: LEAK srv /srv/rcloseprobe.6 | LEAK dup /srv/rcloseprobe.6.d | LEAK exit /srv/rcloseprobe.6.x | LEAK env /env/rcloseprobe.c | LEAK env-open /env/rcloseprobe.p — status `leak`
    MATRIX unfixed hvf: the same five LEAK lines — status `leak`
    MATRIX fixed tcg: `rcloseprobe: ok` — status nil
    MATRIX fixed hvf: `rcloseprobe: ok` — status nil
    (no EARLY dup — the first of two closes kept the name — and no LOST env — the plain create stayed)
  implication: H1 CONFIRMED on both accelerators: every ORCLOSE arm (create new-file, open, dup'd last close, exit drain; /srv and /env) leaks at the pin and is removed with the fix

- timestamp: 2026-11-20T19:25:00Z
  checked: aarch64 composed gates, TCG, scratchpad/rc/a64gates.sh (HARVEY=scratchpad/harvey-rc, R9_SKIP_BUILD=1, --override-input r9 git+file://…/r9-rc)
  found: boot-r9-smoke rc=0 ALL STAGES PASS; boot-r9-net-smoke rc=0 ALL STAGES PASS; boot-r9-tier1 rc=0 ALL STAGES PASS, 42 T1-OK + 23 T1-SKIP = `expected on r9: 42 T1-OK markers + 23 T1-SKIP markers (computed)` (incl. 20-11's `T1-OK cexeccreate`), 0 T1-FAIL; `-accel tcg -cpu cortex-a72`; kernel gi4qg8fx…-03c6038
  implication: no aarch64 regression

- timestamp: 2026-11-20T19:35:00Z
  checked: darwin battery (scratchpad/cx/battery.sh) on the fix (03c6038) and a fresh base clone (42203d8)
  found: check rc=0 (63 both), clippy x86-64 rc=0 (82 both), xtask test rc=0 (23 warnings both; port 165 → 168), virtshell build rc=0 (37 both), cargo test -p aarch64 --bins 31 → 32; clippy --arch aarch64 rc=1 at both with IDENTICAL aarch64 errors (the 4 pre-existing undocumented-unsafe in proc.rs) and identical warning locations except xtask/src/config.rs (the fresh-clone replay artifact seen in create-ocexec-not-marked, this time on the other side); check and clippy warning locations identical
  implication: no new lint

- timestamp: 2026-11-20T19:45:00Z
  checked: amd64 container rc-x64-orclose (ubuntu:24.04 pre-existing, single-user nix, builder; clones of scratchpad/harvey-rc c7044a68b and scratchpad/r9-rc 03c6038), scratchpad/rc/x64/gates.sh
  found: |
    x86_64 host tests: base 42203d8 115 passed; fix 116 passed / 0 failed (incl. created_orclose_srv_post_goes_at_the_last_close, remove_syscall_dispatches_and_refuses_loudly [#c], remove_round_trips_… [+devsrv arm], the three OCEXEC tests)
    kernel-x64: base /nix/store/wx5m4w7yg78vv1lin2swlsyxc9w936dj-r9-kernel-x64-42203d8 bdab1829… (25783728 B) → fix /nix/store/azazqdjnv7mzz0qxp38c96jqrzq47jhl-r9-kernel-x64-03c6038 8b9d216d73e5cfc31b71f2052f513d873835be25c994e05a9844003a9c7343b4 (25882864 B)
    gates (TCG, full default pipeline, every `r9 kernel package` = azazqdjn…-03c6038): boot-r9-auth-x64 rc=0 423 s ALL STAGES PASS; boot-r9-net-smoke-x64 rc=0 286 s ALL STAGES PASS; boot-r9-tier1-x64 rc=0 213 s ALL STAGES PASS, 38 T1-OK + 0 SKIP = computed, 0 FAIL
    boot-iso: specified sha256-L8Sm1tGU6KOyb3+XkzddGeqKe993L/akNm5xaqLjmV8= got sha256-4vLm2Qsoq87i1cWN9Ief/9LCT2WlKibHnqlZzTKhjGk=; wired build green (boot.iso 82af93b1dd28370b0565619f8a12bf58fae6c3e18da908ab27854c96c8933f69); boot-iso --rebuild rc=0; kernel-x64 --rebuild rc=0
    xtask check rc=0 (63), clippy rc=0 (82), test rc=0 (23; port 168)
  implication: x86_64 green; the new FOD value is reproducible

- timestamp: 2026-11-20T19:55:00Z
  checked: commits — r9 main (was 42203d8 = origin/main, clean): patch 42203d8..03c6038 applied + committed; then the outputHash line; Harvey main: rcloseprobe.c + the wire.py edits (byte-identical to the tested scratch copies; sh -n clean)
  found: r9 5629cf0 `fix: ORCLOSE removes local files at the last close, through one shared remove dispatch` (tree b0fcb351… == tested 03c6038); r9 fa38402 `bump boot-iso hash for the ORCLOSE kernel change`; unpushed (origin/main 42203d8). Harvey 1e6d8e79f `test(debug): rcloseprobe pins ORCLOSE remove-on-close for /srv and /env` (rcloseprobe.c, subset-build.sh, mkimage.sh only). No AI attribution
  implication: committed code == tested code

- timestamp: 2026-11-20T20:00:00Z
  checked: the committed HEAD fa38402 — darwin #kernel-virt; container (branch final) #kernel-x64 + an UNMODIFIED #boot-iso + --rebuild; then cleanup
  found: kernel-virt /nix/store/y1gq491ka27sslgzgkhl8rs9s6v32s5m-r9-kernel-virt-fa38402 bd0d969651a6c7287940087d5e12fb5be88c8284704455f9010d85b583b18f81 (60678376 B) = tested; kernel-x64 /nix/store/kh0ggsfiyvh13080icm0d137fn4abb34-r9-kernel-x64-fa38402 8b9d216d… (25882864 B) = tested; boot-iso rc=0 with the committed hash, boot.iso 82af93b1… = the wired test, --rebuild rc=0. Container removed; image/volume/container lists == the pre-session snapshots; 0 qemu
  implication: every result holds for the committed bytes; venue clean

## Resolution

root_cause: "AND-gate of two port-side gaps (both arches): (1) nothing recorded ORCLOSE on a chan — no CRCLOSE twin on Chan, so open_chan and sys_create's new-file arm dropped it for local devices; (2) devsrv and devenv had no remove op at all (the /srv registry and EnvGroup entries are private to port), so even a recorded bit had nothing to call. A local file opened/created ORCLOSE (lib9p postfd's /srv post) therefore outlived its last close. devmnt was already right (ORCLOSE rides the Topen/Tcreate mode byte; the server removes at clunk)."
fix: "r9 5629cf0 (local, unpushed, on 42203d8): port — Chan.rclose: Option<Arc<()>> (shared by dup/op_clone: the open's refcount); namec::mark_fd_state (OCEXEC + ORCLOSE; open_chan and both arches' create_new_chan); namec::remove_chan, the ONE device-remove dispatch (DEV_CAP/DEV_SRV/DEV_ENV, else loud refusal); namec::remove_if_last_rclose (from close_chan and Chan::drop_op_ref; DEV_SRV|DEV_ENV only, when the closing chan holds the last share); devsrv::remove (closes the posted chan) + the donor's Eperm for ORCLOSE on #s; EnvGroup::remove + devenv::remove. x86_64: remove_chan routes through port's dispatch (REMOVE gains srv/env arms). aarch64 REMOVE(25) NOT added (plan 20-14). Lock-risk comment at namec::remove_if_last_rclose and Chan::drop_op_ref. r9 fa38402 re-wires the boot-iso hash. Harvey 1e6d8e79f adds rcloseprobe (+ build/bake wiring)."
oracle_type: "specified — the donor contract: a file opened or created ORCLOSE is removed when the LAST reference to that open closes (srvclose/envclose), and not before; a plain file stays. Boundary neighbours: dup'd open (first close keeps it), exit drain (no explicit close), the open arm vs the create arm, /srv vs /env, a plain-create control, the op-ref-in-flight corner (host)."
verification:
  target_test: { result: pass, detail: "boot: rcloseprobe RED (5 LEAK lines) at 42203d8 on HVF+TCG → GREEN `rcloseprobe: ok` at 03c6038/fa38402 bytes on HVF+TCG; host: port 168 (+3), aarch64 32 (+1), x86_64 116 (+1)" }
  mutation_check: { result: pass, reason_if_skipped: "no cargo-mutants — manual seeding", mutant_killed: "port `-- orclose`: M0 close arm removed (= the pin's behaviour) KILLED 2; M1 last-close guard removed KILLED 2; M2 mark not recorded KILLED 3 (run on the pre-C draft; C only moved the dispatch into remove_chan)" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: n/a, detail: "adds behaviour; the removed lines are the inline cexec mark (moved into mark_fd_state) and x86_64's local match (moved into port's remove_chan)" }
  adjacent_tests: { result: pass, suites_run: ["darwin xtask check/clippy/test + virtshell build — counts and locations == base", "clippy --arch aarch64 — identical aarch64 diagnostics (pre-existing 4 errors)", "container xtask check/clippy/test rc=0", "x86_64 host 116/116 incl. the adapted REMOVE pins", "aarch64 TCG smoke / net-smoke / tier1 (42 OK + 23 SKIP) PASS", "x64 TCG auth / net-smoke / tier1 (38 OK) PASS"], not_run: "aarch64 HVF composed series (the owner's combined series); a boot-level rcloseprobe on x86_64 (no amd64 build of it — host tests + the x64 gates cover x86_64)" }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true, detail: "revert = the 42203d8 kernel (d8be7228…): every leg LEAKs on HVF and TCG; reapply = 03c6038 == fa38402 bytes (bd0d9696…): `rcloseprobe: ok` on HVF and TCG; host M0 red, restored green" }
  guardrail_verdict: accepted
kernel_bytes:
  aarch64_kernel_virt: "fa38402 /nix/store/y1gq491ka27sslgzgkhl8rs9s6v32s5m-r9-kernel-virt-fa38402 sha256 bd0d969651a6c7287940087d5e12fb5be88c8284704455f9010d85b583b18f81 (60678376 B); 42203d8: d8be7228… — CHANGES, the combined HVF series must run at fa38402"
  x86_64_kernel_x64: "fa38402 /nix/store/kh0ggsfiyvh13080icm0d137fn4abb34-r9-kernel-x64-fa38402 sha256 8b9d216d73e5cfc31b71f2052f513d873835be25c994e05a9844003a9c7343b4 (25882864 B); 42203d8: bdab1829… — CHANGES; x64 TCG gates run here"
  boot_iso: "outputHash sha256-4vLm2Qsoq87i1cWN9Ief/9LCT2WlKibHnqlZzTKhjGk= (was sha256-L8Sm1tGU6KOyb3+XkzddGeqKe993L/akNm5xaqLjmV8=); boot.iso 82af93b1dd28370b0565619f8a12bf58fae6c3e18da908ab27854c96c8933f69; green at fa38402 with --rebuild"
files_changed: [r9 port/src/dat.rs, port/src/namec.rs, port/src/devsrv.rs, port/src/devenv.rs, port/src/namec_tests.rs, aarch64/src/syscall.rs, x86_64/src/syscall.rs, flake.nix; Harvey build/aarch64/rcloseprobe.c, build/aarch64/subset-build.sh, build/aarch64/mkimage.sh]
open_risks:
  - "LOCK (owner-noted): fdgrp::op_done calls Chan::drop_op_ref under the fd-group lock, so in the close-races-an-in-flight-op corner the ORCLOSE remove (and devsrv's close of the posted pipe end) runs with that leaf lock held — as devmnt's clunk_if_last already does. Commented in port/src/namec.rs (remove_if_last_rclose) and port/src/dat.rs (drop_op_ref). Fix shape if it bites: have op_done return the op-ref chan so its release runs after the guard drops."
  - "Harvey mkimage.sh now requires out/rcloseprobe: an R9_SKIP_BUILD=1 bake over a userland built before 1e6d8e79f fails `rcloseprobe missing` — rebuild the userland (subset-build.sh) once first."
related_findings_not_fixed:
  - "aarch64 REMOVE(25) — plan 20-14; it should route through port::namec::remove_chan (now has cap/srv/env arms)."
  - "devsrv: the donor's srvwrite refuses to post an fd carrying CCEXEC|CRCLOSE; r9's write does not (no measured caller posts such an fd)."
  - "EnvGroup::remove keeps insertion order (the donor swap-removes) — r9's deep_copy renumbering reads insertion order."
tier1_suggestion: "marker `T1-OK rcloseprobe` — sweep line `if(/bin/rcloseprobe) echo T1-OK rcloseprobe` / `if not echo T1-FAIL rcloseprobe`, in the r9-venue block beside cexecprobe/cexeccreate; red at any pin without 5629cf0 (and each red run leaves 3 stale /srv/rcloseprobe.<pid>* names until aarch64 has remove)"

## Prevention

human_verification: owner confirmed fixed (2026-11-20, relayed by the coordinator); r9 pushed to fa38402

five_whys (branched, from the AND-gate's two halves):
  - code (flag half): why was an ORCLOSE file never removed? its chan carried no remove-on-close state → why? Chan had no CRCLOSE twin, and the OCEXEC work (19-10 cont 4, then create-ocexec-not-marked) taught one fd-state bit at a time, following the measured consumer (procexec) → why was that possible? there was no single place that records every fd-state bit of a mode word → actionable: namec::mark_fd_state records OCEXEC and ORCLOSE together, and every arm that yields an open chan calls it
  - code (device half): why could even a recorded bit do nothing? devsrv and devenv had no remove op, and the only remove routing was arch-local (x86_64 remove_chan, devcap only) → why? REMOVE was ported per measured consumer (18-02's caphash) and the close path had no remove at all → actionable: one port dispatch (namec::remove_chan) shared by REMOVE and the close path, so a device that gains remove gains it for both
  - test/environment: why no failing gate? no sweep entry creates or opens anything ORCLOSE (lib9p servers, rio and the plumber — the live callers — are not in the arm64 sweep), mounted ORCLOSE was already correct server-side, and a stale /srv name is silent (nothing reads /srv for it)

why_not_caught: no gate existed for this class — no tier-1 or smoke entry opens or creates ORCLOSE, its live callers (lib9p postfd, rio, plumber) are not in the arm64 sweep, a leaked /srv or /env name fails nothing, and no host test covered devsrv/devenv removal (they had no remove)

recurrence_guard: r9 fa38402 host tests `port/src/namec_tests.rs:{orclose_srv_entry_goes_at_the_last_close_only, orclose_remove_waits_for_an_in_flight_op_ref, orclose_env_key_goes_at_the_last_close}` (kill the close-arm-removed, last-close-guard-removed and mark-not-recorded mutants), `aarch64/src/syscall.rs:create_cexec_tests::created_orclose_srv_post_goes_at_the_last_close`, `x86_64/src/syscall.rs:tests::{created_orclose_srv_post_goes_at_the_last_close, remove_round_trips_through_the_devcap_arm_and_refuses_unported, remove_syscall_dispatches_and_refuses_loudly}` (verified present on origin/main and passing); Harvey boot reproducer `build/aarch64/rcloseprobe.c` (`rcloseprobe: ok` required; plan 20-11 wires `T1-OK rcloseprobe`); the shared `namec::mark_fd_state` / `namec::remove_chan` structure; KB pattern below
