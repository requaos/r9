# Planning pass 1 spike results (S1 to S6)

Consolidated results of the pass-1 research spikes, the decisions they unlock,
the machine changes they made, and the pass-1 prohibition audit. Written by
Plan 01-07 Task 1, 2026-11-23. The six source records are `S1-FINDINGS.md` to
`S6-FINDINGS.md` in this directory; every verdict below points at the evidence
file that backs it.

Pass 1 was scoped by D-22: spikes only, ending at this owner checkpoint. A
FAIL below is a complete answer with decisive evidence, not an open question.

## Verdict matrix

One row per leg. Venues: **Mac TCG** and **Mac HVF** are this aarch64-darwin
host; **lima builder** is the `harvey` VM (Ubuntu 24.04 aarch64, nix build
sandbox); **x86_64 Linux witness** is the owner's KVM host used by Plan 06
(Linux x86_64 NixOS 7.4.0-rc3, 16 CPU, 30 GiB, `/dev/kvm` writable).

| Spike | Leg id | Venue | Verdict | Evidence |
|-------|--------|-------|---------|----------|
| S1 | s1-rung1-lima-aarch64 | lima builder (`unshare -Ur -m`) | FAIL | `s1/evidence/rung1-builder.log` |
| S1 | s1-rung1-proot-lima-aarch64 | lima builder (`proot -0`) | PASS | `s1/evidence/rung1-builder.log` |
| S1 | s1-hostbin-lima-aarch64 | lima builder | PASS | `s1/evidence/rung2-builder.log` |
| S1 | s1-rung2-lima-aarch64 | lima builder (amd64 libc) | PASS | `s1/evidence/rung2-builder.log` |
| S1 | s1-witness-rung1-x86_64-linux | x86_64 Linux witness (`unshare`) | FAIL | `s1/evidence/witness-x86_64-linux.log` |
| S1 | s1-witness-rung2-x86_64-linux | x86_64 Linux witness (`proot`) | PASS | `s1/evidence/witness-x86_64-linux.log` |
| S2 | s2-input-lima | lima builder, Mac-fetched flake input | PASS | `s2/evidence/casecheck-builder.log` |
| S2 | s2-fod-lima | lima builder, builder-side fixed-output fetch | PASS | `s2/evidence/casecheck-builder.log` |
| S2 | s2-copyback-lima | Mac system store, `nix copy --from` as owner | FAIL | `s2/evidence/casecheck-builder.log` |
| S2 | s2-copyback-chroot-lima | Mac local chroot store on APFS | PASS | `s2/evidence/casecheck-builder.log` |
| S2 | s2-lazy-devshell | aarch64-darwin devshell plus lima build | PASS | `s2/evidence/lazy-devshell.log` |
| S3 | s3-static-srv-global-mac | Mac, static reading of r9 source | PASS | `s3/evidence/static-analysis.md` |
| S3 | s3-static-drawterm-x11-mac | Mac, static reading of Harvey source | PASS | `s3/evidence/static-analysis.md` |
| S3 | s3-rio-kvm-linux | x86_64 Linux witness (KVM) | FAIL | `s3/evidence/rio-kvm-linux.log` |
| S3 | s3-norio-control-kvm-linux | x86_64 Linux witness (KVM, `-G` control) | PASS | `s3/evidence/norio-control-kvm-linux.log` |
| S3 | s3-determinism-kvm-linux | x86_64 Linux witness (two bakes) | FAIL | `s3/evidence/determinism-kvm-linux.txt` |
| S4 | s4-tier0-amd64-lima | lima builder (27 amd64 libraries) | PASS | `s4/evidence/tier-builder.log` |
| S4 | s4-tier1-amd64-lima | lima builder (40 amd64 commands) | PASS | `s4/evidence/tier-builder.log` |
| S5 | s5-q35-virtio-tcg-mac | Mac TCG | PASS | `s5/evidence/q35-virtio-tcg-mac.log` |
| S5 | s5-q35-nonic-tcg-mac | Mac TCG | PASS | `s5/evidence/q35-nonic-tcg-mac.log` |
| S5 | s5-pc-baseline-tcg-mac | Mac TCG | PASS | `s5/evidence/pc-baseline-tcg-mac.log` |
| S5 | s5-q35-kvm-linux | x86_64 Linux witness (KVM) | PASS | `s5/evidence/q35-kvm-linux.log` |
| S6 | s6-amd64-static-mac | Mac, static (`aout.py check --arch amd64`) | PASS | `s6/evidence/amd64-static.txt` |
| S6 | s6-arm64-hvf-mac | Mac HVF (control, probe, flip-back) | PASS | `s6/evidence/arm64-hvf-control.log`, `-probe.log`, `-flipback.log` |
| S6 | s6-x86_64-kvm-linux | x86_64 Linux witness (KVM) | PASS | `s6/evidence/x86_64-kvm-linux.log` |

The five FAILs are findings with evidence: unshare is refused on both measured
builders (S1, two legs); `nix copy --from` into the Mac system store is refused
by signature policy as the owner's user (S2); the rio leg's headless drawterm
never authenticated, because the generated DEMO password for glenda was not
known, so the driver timed out at the password-feed stage (S3); and the second
`mkimage.sh` bake aborts at the auth bake, so no determinism comparison ran
(S3). The `tr-ok` line in the rio capture is the owner's manual graphical
drawterm attempt, not the gate's headless session (owner clarification,
2026-11-23; see DECISION 5).

SPIKE S1: ANSWERED
SPIKE S2: ANSWERED
SPIKE S3: ANSWERED
SPIKE S4: ANSWERED
SPIKE S5: ANSWERED
SPIKE S6: ANSWERED

Count: 6 spikes, 25 legs, 20 PASS, 5 FAIL, 0 unanswered.

## Assumptions

A1 to A16 from `01-RESEARCH.md` ("Assumptions Log"), classified against the
spike evidence.

ASSUMPTION A1: REFUTED - the `unshare -Ur -m` venue is refused on the lima builder (nested `uid_map` EPERM) and on the x86_64 witness (`/nix/store` bind refused, guest exit 32); the same guest work passes through `proot` on both (`S1-FINDINGS.md`).
ASSUMPTION A2: CONFIRMED - the lima NAR transport preserves the 23 case-colliding paths in both source modes, the Mac APFS store applies its reversible case hack, `nix store verify --no-trust` is clean, and all 23 blob sha1s re-check; the qualification is store policy, not case handling (`S2-FINDINGS.md`).
ASSUMPTION A3: CONFIRMED - the amd64 tier content is host-independent; all 27 libraries and 40 tier-1 entries built on the aarch64-linux builder with no x86_64 builder and no emulation (`S4-FINDINGS.md`).
ASSUMPTION A4: CONFIRMED - Harvey's amd64 tier build runs on an aarch64-linux host with two venue ports (`/rc/bin` on PATH and an explicit `INCLUDE` list) (`S4-FINDINGS.md`).
ASSUMPTION A5: CONFIRMED - r9's srv table is kernel-wide and the serial console's `ls /srv` reads it (`/srv/boot`, `/srv/factotum` in the `-G` control); the live rio posting was not reached because the rio leg timed out, so visibility is confirmed and a live rio session remains unproven (`S3-FINDINGS.md`).
ASSUMPTION A6: UNTESTED - the X11 backend is statically confirmed and the gate's headless drawterm starts under Xvfb, but no headless authentication completed: the owner did not know the generated DEMO password for glenda. The `tr-ok glenda@glenda(unknown) -> glenda@bootes` line was the owner's manual graphical drawterm attempt while the run appeared hung, not the gate's headless session; the driver timed out at the password stage (`S3-FINDINGS.md`, owner clarification 2026-11-23).
ASSUMPTION A7: CONFIRMED - a flat rust-lld payload wrapped in the 40-byte Plan 9 header satisfies r9's `parse_aout` rules and loads and runs on the arm64 HVF and x86_64 KVM legs (`S6-FINDINGS.md`).
ASSUMPTION A8: REFUTED - the amd64 library tier builds in 2 s (lima) and 4 s (witness); tier 0 plus tier 1 takes 25 s on the builder, not tens of minutes. That cost belongs to the full tiered userland build and the image bake (`S1-FINDINGS.md`, `S4-FINDINGS.md`).
ASSUMPTION A9: CONFIRMED - a context-discarded aarch64-linux `outPath` exported by the darwin devshell is lazy (entering the shell builds nothing) and equals the later build path (`S2-FINDINGS.md`).
ASSUMPTION A10: UNTESTED - no pass-1 spike performed a Harvey pin bump, so `lib.fileset` filtering's rebuild avoidance was not exercised.
ASSUMPTION A11: CONFIRMED - Harvey's own `9k10cpu.elf` launches under q35 to the `root is from` prompt on Mac TCG and on the x86_64 KVM witness with virtio and no recorded signature; the cpu-service half is out of D-20 scope and is not claimed (`S5-FINDINGS.md`).
ASSUMPTION A12: REFUTED - the two-bake byte-determinism check cannot pass on this tree: bake 1 completes, bake 2 aborts at the auth bake (`hsecstored missing from the computed auth-suite set`), so no image is compared (`S3-FINDINGS.md`).
ASSUMPTION A13: UNTESTED - no pass-1 spike built `drawterm` or `harness` through `inputs.harvey.inputs.nixpkgs.follows`; no flake with that follow was exercised.
ASSUMPTION A14: UNTESTED - the HARN-02 resolver semantics were not implemented in pass 1; the explicit-environment-only reading is deferred to pass 2.
ASSUMPTION A15: CONFIRMED - settled by CONTEXT D-21 (owner chose `R9_HARVEY_DIR_AMD64` / `R9_HARVEY_DIR_ARM64`), not by a spike result; the devshell mechanics are proven by A9.
ASSUMPTION A16: UNTESTED - hosted CI itself was not run; the lima Ubuntu 24.04 guest with `kernel.apparmor_restrict_unprivileged_userns=1` refuses unprivileged userns creation, so that class of host needs the proot venue too (`S1-FINDINGS.md`).

## Decisions unlocked

Seven decisions for the owner at the closing checkpoint. Each cites the spike
evidence, the options, and an evidence-based recommendation. Task 3 records the
owner's answers; pass 2 is planned from them.

### 1. D-04 Mac builder mechanism

**Evidence.** S2 tested both source modes through the lima builder: the
Mac-fetched flake input and the builder-side fixed-output fetch both preserve
all 23 case-colliding paths (23/23 blob sha1s, no `~nix~case~hack~` names on
the builder). `nix copy --from ssh-ng://lima-harvey` into the Mac **system**
store as the owner's user is refused (`lacks a signature by a trusted key`;
`require-sigs = true`, `trusted-users = root`), while a local chroot store on
the same APFS volume imports and verifies clean (23/23). S4 built the amd64
tier 0 (27/27, 6 s) and tier 1 (40/40, 19 s, every artifact `S_MAGIC`
`00008a97`) on the aarch64-linux lima builder. A9 proves the lazy devshell
path export. The native Determinate builder was **not** exercised in pass 1:
the owner chose lima at S1 and this plan's own prohibitions forbid Mac
configuration changes, so its case behaviour is unmeasured and Determinate
documents a case-collision caveat for it.

**Options.**

- `native` - Determinate native Linux builder after `determinate-nixd auth
  login`. No Nix config edits, aarch64-linux (the system S4 showed is enough),
  the local daemon imports results itself so S2's copy-back failure does not
  apply. Needs a FlakeHub login and Determinate's service; case handling
  unmeasured.
- `lima` - the existing `harvey` VM registered as an `ssh-ng` daemon remote
  builder. No third-party login, the only venue measured end to end for case
  handling and amd64 tiers. Needs a root-readable key, a pinned ssh port and
  `builders =` through the owner's nix-darwin configuration; the
  remote-builder registration itself was not configured in pass 1.
- `both` - document both, with one named primary. Two paths to keep working
  and documented; both feed the same derivation, so D-04's single build path
  still holds.

**Recommendation: `both`, with lima as primary.** Lima is the only venue with
measured case handling and amd64 tier builds, and it avoids a third-party
login; registering it as a remote builder is exactly the fix S2 identified for
the copy-back FAIL. Document the native builder as the alternative for a
developer who prefers one login over VM configuration. Either way, use the
builder-side fixed-output fetch for the Harvey source (S2's preferred supply)
and keep all tree outputs free of case-colliding names.

### 2. D-03 venue for the pass-2 tree derivations

**Evidence.** S1 rung 1 (`unshare -Ur -m` plus tmpfs chroot) fails on the lima
builder (nested `uid_map` write EPERM inside the sandbox and on the Ubuntu
24.04 guest) and again on the x86_64 witness (private tmpfs starts, but the
`/nix/store` bind is refused; guest exit 32). The same guest script through
`proot -0` passes on both (rung 1), Harvey's host toolchain builds (rung 2),
and the amd64 libc builds in 2 s on lima and 4 s on the witness (601170 bytes,
259 members), with `nix store verify --no-trust` clean. S4 adds the two venue
ports the real tier build needs.

**Options.** The `unshare` venue (refused on both measured hosts); the `proot`
fallback (measured on both); a different route such as `vmTools.runInLinuxVM`
(needs KVM in the builder) or patching mkfiles (invasive, last resort).

**Recommendation: `proot`, with no dependency on `unshare` or a privileged
builder.** Carry the four host-toolchain adjustments from S1 (store-bash
shebang for `9l`, `pkgs.ed`, plan9port last on PATH, GNU sed shadowing for the
pinned bootstrap) and S4's two ports (`/rc/bin` on PATH and the explicit
`INCLUDE` list) in the venue definition, not in each build step. Copy only
what images and ureg checks consume, and assert no installed name contains
`~nix~case~hack~`.

### 3. D-20 scope for the one Harvey command

**Evidence.** S5 ran Harvey's own `amd64/9k10cpu.elf` on this Mac under TCG
(q35 plus virtio, q35 without a NIC, and the `pc` baseline) and on the
x86_64 KVM witness; every leg reached the `root is from (tcp)[tcp]: ` prompt
with no recorded panic signature. The kernel copy's source commit is unknown
(sha256 `ecdcda…`, 2030226 bytes), so a pass-2 app should build the kernel it
boots and record the resulting sha256. `s5/evidence/build-path-analysis.md`
gives the build chain and its needs: on Linux `nix run .#setup` → `toolchain`
→ `build`, where `build` edits two mkfiles, installs `cpp`/`bind`/`rc`/the
compilers into the host `/bin` with setuid `sudo`, and runs `sudo unshare -m`;
a passwordless-sudo host has the `unshare -Ur -m` chroot venue
(`build/mm/build-9k10t1.sh:250-356`); on darwin the build must stay in lima
and the Mac host needs no sudo. An rc prompt needs a root source; the smallest
Harvey-only route is `u9fs` with `-a none` chrooted at a minimal root and
behind a TCP listener on port 564, which needs root or a userns scaffold.

**Options.**

- Option A - build and boot to the `root is from` prompt. No root image, no
  9P server, no network; one flake app plus script and docs; smallest honest
  D-20 delivery. Proven on Mac TCG and x86_64 KVM.
- Option B - build and boot to an rc prompt through `u9fs` on TCP 564. Adds
  a privileged/userns listener, a staged minimal root and prompt driving; no
  Harvey source edit if `u9fs` is accepted as the root server.

**Recommendation: Option A for Phase 1, with Option B as the next increment.**
Reuse `s5/boot-q35.sh`'s capture-before-start, classification and kill/reap
trap instead of growing a second launch path. Keep the app's claim to "builds,
launches and reaches the prompt"; do not claim a cpu service.

### 4. D-17 probe target tool and recipe

**Evidence.** S6 proved the technique end to end on arm64 HVF (control,
probe and flip-back; flip-back image byte-identical to control,
`746f1dfe…`) and on x86_64 KVM (`S6-X86 control=PASS probe=PASS
flipback=PASS sha-equal=yes`; control/flip-back `3ae4127a…`, probe
`7af2a8e0…`, probe sha256 `db94057f…`). `s6/target-analysis.md` evaluates the
six staged candidates: `ps` is the only one staged in every x86_64
chain-family image, absent from the boot path, and used at only two read-only
gate sites (`smoke-r9.expect:265`, `tier1-sweep.rc:137`). `echo` is
disqualified: it is not an rc builtin (`sys/src/cmd/rc/plan9.c:35-48`), so
cpurc's whole marker ladder and every typed smoke marker would exec the
replacement.

**Options.** `ps` (the S6 recommendation); `echo` (D-17's prose example, but
it rewrites the boot marker vocabulary); `ls`/`cat` (sweep-critical);
`rm`/`test` (not staged in the chain image).

**Recommendation: `ps`, with the S6 recipe.** `#![no_std] #![no_main]`, a
looping panic handler, and a `global_asm!` `_start` that writes the marker with
PWRITE(51) and exits with EXITS(8); build with plain `rustc --target
x86_64-unknown-none|aarch64-unknown-none -C panic=abort -C opt-level=s
-C relocation-model=static -C link-arg=-T<ld>
-C link-arg=--oformat=binary` from the devshell, never in the cargo workspace;
link flat at `0x200028` (amd64) or `0x10028` (arm64); wrap in the 40-byte
big-endian header and validate with the `aout.py check` rules that mirror r9's
`parse_aout`/`read_aout`. Remove the probe and its manifest entry after the
proof (D-18); the default image stays all-Harvey.

### 5. D-11 rio marker

**Evidence.** The static half is sound: r9's srv table is kernel-wide
(`port/src/devsrv.rs:59`) and rio posts `/srv/rio.<user>.<pid>` and
`/srv/riowctl.<user>.<pid>` (`rio/fsys.c:152-153`, `:169-170`); the cpu image
binds `#s` at `/srv` and the serial console's namespace is that image's
`/lib/namespace`. The `-G` control PASSed: the serial `ls /srv` showed only
`/srv/boot` and `/srv/factotum`, with zero `cpurc: FAIL` and zero panic
markers, so the discriminator is specific. The positive rio leg FAILed
without ever observing `S3-RIO-SERVING`. Owner clarification (2026-11-23):
the `tr-ok glenda@glenda(unknown) -> glenda@bootes` line in the capture came
from the owner's own **graphical** drawterm login attempt while the run
appeared hung, not from the gate's headless drawterm. The owner did not know
the generated DEMO password for glenda, and believes the headless drawterm
login attempt did not work at all. The positive leg is therefore unproven
because headless authentication and the password feed never succeeded — not
merely because of an expect prompt mismatch. The marker design itself is not
invalidated; the headless credential path and feed are.

**Options.** (a) Keep the two srv entries observed from the serial console,
determine the generated DEMO password/credential path, fix the headless
drawterm auth and password feed, and re-run the rio leg with `control-noG`
unchanged; (b) take the research fallback now (a second drawterm session, or
an `xwd -root` pixel check on Xvfb), which is weaker and more brittle.

**Recommendation: (a), conditional on the re-run.** Keep the serial `/srv`
marker as the primary design because A5 and the control support it, and
resolve the generated DEMO password/credential path and the headless drawterm
auth before the marker is trusted. Do not treat the `tr-ok` line as gate
evidence. If the re-run still cannot observe both entries, fall back to (b).
Carry the gate-teardown finding into the pass-2 gate: reap drawterm by the
scratch/dial identity or the pty child pid, not by the `r9s3riogate-dt-*`
argv0 tag (PID 88800 survived and `ps` showed the store path).

### 6. D-16 assembler verification method

**Evidence.** A12 is REFUTED. Bake 1 completed, but bake 2 aborted at the auth
bake with `hsecstored missing from the computed auth-suite set`, so none of the
nine images reached an IDENTICAL/DIFFERENT verdict and no byte differential
ran. The static analysis confirms the design split: the six non-auth images
are deterministic by construction (staged mtimes and header time ride
`KERNDATE`), while the auth and cpu images embed a fresh bake-time p9sk1
domain (`mkimage.sh:1290-1305`, `:1848-1856`).

**Options.** (a) Byte-identical differential tests for the six deterministic
images plus member-list and size comparison for auth and cpu, once a second
bake completes; (b) member-list comparison (paths, sizes, hashes) for all
images until then.

**Recommendation: (b) as the pass-2 plan, with (a) as a later strengthening.**
Resolve the auth-suite resolver (`hsecstored`) before promising any byte
differential. The assembler is still testable now by comparing the staged
member list and sizes against a reference bake; do not plan a whole-image byte
test that cannot run.

### 7. Keep the spike builder for pass 2, or revert it now

**Evidence.** The lima VM `harvey` is the only builder with measured case
handling (S2), host-toolchain and libc builds (S1) and amd64 tier 0/1 builds
(S4); pass 2's tree derivations need the same venue. The guest-only
`trusted-users` line and the running VM are the only live changes; their exact
revert commands are in the ledger below.

**Options.** Keep the VM and its guest configuration for pass 2; or revert now
(stop the VM and remove the guest line), leaving pass 2 to re-create it.

**Recommendation: keep for pass 2.** Reverting now would make pass 2 redo the
bring-up S1 already measured. If the owner prefers a clean Mac, the ledger's
revert commands are one `limactl stop harvey` plus one guest `sed`/daemon
restart; the Mac host itself was never changed.

## Machine changes made by the spikes

Every change the spikes made, with its exact revert. No Mac configuration was
changed: no FlakeHub login, no `/etc/nix` edit, no Mac store-policy change, and
no Harvey checkout edit.

| # | Change | When | Revert |
|---|--------|------|--------|
| 1 | Guest-only `/etc/nix/nix.custom.conf` line `trusted-users = root neilskinner` appended inside the lima VM, then the guest `nix-daemon` restarted | 2026-11-22 (S1 bring-up) | `ssh -F ~/.lima/harvey/ssh.config lima-harvey 'sudo sed -i "/^trusted-users = root neilskinner$/d" /etc/nix/nix.custom.conf && sudo systemctl restart nix-daemon.service'` |
| 2 | lima VM `harvey` started (it was already running at inspection; `limactl start harvey` was a no-op) | 2026-11-22 (S1) | `/nix/store/8szjcdjs279pd9h4d0s5dmyzjvi0lamh-lima-2.2.0/bin/limactl stop harvey` (`limactl` is not on `PATH`) |
| 3 | Builder-side store paths on the lima VM from S1, S2 and S4 (`s1-nsprobe-proot`, `s1-hostbin`, `s1-libc-amd64`, `s2-casecheck-input`, `s2-casecheck-fod`, `s2-lazy-target`, `s4-tier-amd64`, spike flakes and inputs) | 2026-11-22 | `NIX_SSHOPTS="-F /Users/neilskinner/.lima/harvey/ssh.config -oControlPath=none" nix store gc --store ssh-ng://lima-harvey` (inert otherwise) |
| 4 | Mac local scratch: the S2 chroot store at `spikes/s2/work/mac-store` and spike build/log scratch under `spikes/*/work/` (all gitignored) | 2026-11-22 to 2026-11-23 | `rm -rf .planning/phases/01-stage-p0-cross-build-harness/spikes/*/work` (keeps the committed `spikes/s5/9k10cpu.elf`) |
| 5 | Owner's x86_64 Linux witness host: scratch tree, store paths and logs under `~/r9-spike-work` (default), written by the pull-only bundle | 2026-11-23 (Plan 06) | `rm -rf ~/r9-spike-work` (owner's action; state not observable from this Mac) |
| 6 | Mac Nix authentication: none taken | pass 1 | not applicable; if a FlakeHub login is ever taken for the native builder, `determinate-nixd auth logout` is the owner's own action |

**State after the closing checkpoint (2026-11-23): kept for pass 2.** The owner
requested no revert, so no command from the Revert column was run. Change 1
(the guest-only `trusted-users` line) and change 2 (the lima VM `harvey`)
remain in place for pass 2, as do change 3's builder-side store paths and
change 4's gitignored Mac scratch. Change 6 remains "none taken" — there is no
Mac Nix authentication to log out of. The revert commands stay documented for
a later owner decision.

## Prohibition audit

Run from the r9 checkout at HEAD `671e1e7` before this plan's Task 1 commit,
and re-run at HEAD `e03f760` before the Task 3 commit.

- **Harvey untouched.** The guard command from this plan's verification section
  printed:

  ```
  HARVEY-UNTOUCHED
  ```

  Measured: Harvey HEAD `25d97dc4d4976bfb7411f01970755acf6261c81b` and
  `git status --porcelain` sha256
  `041d29292e5032db0a39f4d7720b6e175b470e03b2292735c28c587f10b6439e`,
  both matching the plan's expected values. The 11 pre-modified
  troff/postscript font files are the known pre-existing darwin case-collision
  artifacts included in that hash; no spike wrote to the checkout.
- **r9 protected paths.** `git diff --name-only ce51503989b2d26efbf919e59e9748097f16f637 HEAD -- flake.nix flake.lock xtask x86_64 aarch64 riscv64 port lib .github .cargo Cargo.toml Cargo.lock rust-toolchain.toml`
  printed **0 lines** (exit 0). The same paths show **0** `git status --porcelain`
  lines. No protected path changed across the seven plans.
- **No push.** Harvey `refs/remotes/origin/HEAD` is
  `bd7a6982a3e4f465f8ed3da5e0c7290521514ff0`, as the plan expects. r9
  `refs/remotes/origin/main` is `c92a3f4`, **not** the plan's expected
  `116bcc2`: Plan 06's owner-approved pull-only commits were pushed so the
  Linux host could pull them (`01-06-SUMMARY.md`, "Plan verification drift";
  reflog shows the push at 2026-11-22 20:30:56 -0500). `116bcc2` is an
  ancestor. At Task 3 time the local commits `8ba927e`, `671e1e7`, `2d5a2ce`
  and `e03f760` were unpushed, and this plan has pushed nothing.
- **Pass-1 prohibitions held.** Harvey checkout unchanged; r9 root flake,
  lock, `xtask/`, kernel crates, `.cargo`, Cargo files, toolchain file and CI
  unchanged; no push from this executor; no Mac configuration change beyond
  the guest-only line above.
- **Task 3 re-run (HEAD `e03f760`).** The Harvey guard again printed
  `HARVEY-UNTOUCHED` with the same HEAD and status hash; the protected-path
  diff and status checks were again empty (0 lines, exit 0); r9
  `refs/remotes/origin/main` remained `c92a3f4` and Harvey
  `refs/remotes/origin/HEAD` remained `bd7a698`; this executor pushed
  nothing and reverted nothing.

## Open items

None. Every one of the 25 legs has a PASS or FAIL verdict with committed
evidence, and no leg anywhere in S1 to S6 is left unanswered. The five FAILs
are findings the pass-2 plan must handle (proot venue, remote-builder
registration, headless rio authentication/password feed, auth-suite resolver),
not missing answers.

## Deferred to planning pass 2

Per D-22 this pass plans only the research spikes S1 to S6 and the closing
checkpoint. Everything below remains unplanned on purpose and is planned after
the spike results exist. Edge and prohibition predicates for the shipped
harness are also deferred to pass 2 (no phase SPEC.md exists; the spec-less
edge probe was not run for this spikes-only pass).

**Requirement IDs whose implementation is unplanned (spikes only de-risked feasibility):**

| ID | What pass 2 must still plan | Spike input |
|----|-----------------------------|-------------|
| HARN-01 | The `cargo xtask` quick gate that boots to rio and exits non-zero without it, with its negative control | S3 (marker), S6 x86_64 leg |
| HARN-02 | The single resolver (`R9_HARVEY_DIR` override, per-arch variables, `HARVEY_SRC`), routing the seven consumer sites, the named error, the after-gate unmodified proof | S1 (store verify method) |
| HARN-03 | Per-arch manifests of every image file plus the tier-0 library table, and the r9 assembler with differential tests | S3 determinism (A12) |
| HARN-04 | The in-tree swap probe, flip and flip-back in the gate, then its removal | S6 |
| HARN-05 | Drift detection with named errors in both directions and the Rust-provenance rule | none; no spike bears on it, so it is planned from research alone |
| HARN-06 | Existing q35 profiles and the aarch64 virt quick pass through manifest-assembled images | S2, S4 (Mac-built trees) |
| HARN-07 | `inputs.harvey`, the `harvey-hostbin`, `harvey-tree-amd64`, `harvey-tree-arm64` and `mkpaqfs` derivations, devshell exports `R9_HARVEY_DIR_AMD64`/`R9_HARVEY_DIR_ARM64`, CI on the flake supply, the documented Mac builder | S1, S2, S4, A9 |
| HARN-08 | Composed apps in r9, copied and rewired gate scripts and expect drivers, relocated CI legs, the owner-approved Harvey commit, the lock-only pin bump | S5 (Harvey's own launch) |

**CONTEXT decisions not implemented in pass 1**, one row each. D-22 is
satisfied by this pass's two-pass structure; D-23 is applied by every spike
flake pinning Harvey `bd7a6982a3e4f465f8ed3da5e0c7290521514ff0`.

| Decision | What pass 2 must plan or carry |
|----------|--------------------------------|
| D-01 | Devshell exports the read-only tree; no xtask default (HARN-07) |
| D-02 | Flake ownership flip; r9 imports Harvey, Harvey drops `r9` (HARN-08) |
| D-03 | The tree derivation itself (venue chosen at this checkpoint) |
| D-04 | Documenting and wiring the chosen Mac builder |
| D-05 | Lazy per-arch outputs, built on first use |
| D-06 | Pin-bump practice and the local override |
| D-07 | Gate machinery moves into r9 |
| D-08 | Harvey self-sufficiency without r9 |
| D-09 | Harvey as a normal flake input; package reuse |
| D-10 | Flip order: r9 first, then Harvey, then the pin bump |
| D-11 | The rio gate implementation and its marker |
| D-12 | Enforcement by practice, not by hook |
| D-13 | KVM witness practice for the shipped gate |
| D-14 | Quick and full gate tiers |
| D-15 | The per-file Harvey/Rust manifest |
| D-16 | The r9 assembler and the image bake |
| D-17 | The in-tree swap probe |
| D-18 | Probe removal after the proof |
| D-19 | Fixtures read from the pinned tree |
| D-20 | The one Harvey own-kernel command (scope chosen at this checkpoint) |
| D-21 | Per-arch variable names |
| D-22 | Two-pass planning (satisfied by this pass's structure) |
| D-23 | First pin is the pushed `legacy-final` commit `bd7a698` (applied by every spike flake) |

## Owner decisions (closing checkpoint)

**Owner reply received 2026-11-23** (recorded verbatim by Plan 01-07 Task 3):

> D-04: both, lima primary.
> Recommendations 2, 3, 4, 6, 7: accept as written.
> Recommendation 5 (D-11 rio marker): keep the two `/srv/rio.*` and `/srv/riowctl.*` entries as the marker, but the owner clarifies the s3-rio failure evidence: the `tr-ok glenda@glenda(unknown) -> glenda@bootes` line came from the owner's own graphical drawterm login attempt while the run appeared hung, not from the gate's headless drawterm. The owner did not know the generated DEMO password for glenda. The owner believes the headless drawterm login attempt did not work at all. So the positive leg is unproven because headless authentication/password feed never succeeded, not merely because of an expect prompt mismatch. Next action: determine the generated DEMO password/credential path, fix the headless drawterm auth and password feed, then re-run the rio leg; do not treat the `tr-ok` line as gate evidence.
> No revert requested: keep the lima VM and guest config for pass 2.

DECISION 1 (D-04): both, lima primary - accepted recommendation (lima is the only measured venue, with case handling and amd64 tier builds, and needs no third-party login; the native builder is documented as the alternative)
DECISION 2 (D-03): proot, with no dependency on unshare or a privileged builder - accepted recommendation
DECISION 3 (D-20): Option A (build and boot to the "root is from" prompt); Option B is the next increment - accepted recommendation
DECISION 4 (D-17): ps, with the S6 recipe - accepted recommendation
DECISION 5 (D-11): keep the two `/srv/rio.*` and `/srv/riowctl.*` entries as the marker - owner override (the `tr-ok` line is not gate evidence; determine the generated DEMO password/credential path, fix the headless drawterm auth and password feed, re-run the rio leg, keep `control-noG` unchanged)
DECISION 6 (D-16): member-list comparison for all images (option b) as the pass-2 plan; byte differential later - accepted recommendation
DECISION 7 (D-22): keep the spike builder for pass 2; no revert - accepted recommendation

PASS-2 READY: yes

**Record notes.** Decision 7 is the pass-transition half of D-04/D-22 rather
than a new CONTEXT decision: the builder the owner keeps is the lima venue
chosen in Decision 1. The contemporaneous `01-06-SUMMARY.md` wording that the
rio leg "authenticated (`tr-ok`)" is superseded by Decision 5's owner
clarification; the verdict is unchanged (FAIL). No machine revert was
requested or performed.
