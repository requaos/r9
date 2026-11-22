---
phase: 261103-giq
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - build/harness/Cargo.toml
  - build/harness/Cargo.lock
  - build/harness/src/main.rs
  - build/harness/src/regex.rs
  - build/harness/src/srv9.rs
  - build/harness/src/gdbpoke9.rs
  - build/harness/src/mkfs9.rs
  - build/harness/src/console.rs
  - build/harness/src/drive9.rs
  - build/harness/src/trace.rs
  - flake.nix
  - build/aarch64/boot-9qemu.sh
  - build/aarch64/run-gate.sh
  - build/aarch64/subset-exec.sh
  - build/aarch64/donor-ns-build.sh
  - build/aarch64/README.md
  - build/aarch64/trace-boot.rc
files_deleted:
  - build/aarch64/srv9.py
  - build/aarch64/trace-syscalls.py
  - build/aarch64/gdbpoke9.py
  - build/aarch64/mkfs9.py
  - build/aarch64/drive9.py
autonomous: true
requirements:
  - QUICK-261103-GIQ

estimate:
  tokens: 75000
  raw_tokens: 75000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "harness srv9|gdbpoke9|mkfs9|drive9|trace-syscalls accept the same flags/positional args as the .py originals, emit the byte-identical scraped markers ('srv9: serving', 'poked', 'processing'/'file system made', '== PASS: ... trace complete'), keep stdout/stderr discipline (mkfs9 list-mode paths on stdout only), and keep exit-code discipline (0 ok / 1 fail / 2 usage)"
    - "boot-9qemu.sh, run-gate.sh, subset-exec.sh, donor-ns-build.sh, and the trace-syscalls internals invoke the harness binary (HARNESS_BIN env, nix-store-first default per D-04) and contain no python3 interpreter dependency — proven by all gates running green AFTER the five .py files are deleted"
    - "flake.nix exposes packages.harness on all four systems and the devshell carries cargo/rustc (stable nixpkgs) with HARNESS_BIN exported (D-06); the r9 input, boot-r9/boot-r9-smoke apps, and existing devshell contents are undisturbed"
    - "guest disk/mkfs is a thin wrapper exec'ing the compiled aarch64-linux harness mkfs9 (D-05), and produces a bootfs tree byte-identical to the Python original on the pinned protos"
    - "harness trace-syscalls (hello path) reproduces the committed 04-01 trace's syscall-name SET (counts may drift by a line; D-07e)"
  artifacts:
    - build/harness/Cargo.toml (std-only, empty [dependencies] — D-02) + src/{main,regex,srv9,gdbpoke9,mkfs9,console,drive9,trace}.rs
    - flake.nix with packages.harness + devshell cargo/rustc/HARNESS_BIN
    - rewired build/aarch64/{boot-9qemu,run-gate,subset-exec,donor-ns-build}.sh
    - updated build/aarch64/README.md (D-08); deleted build/aarch64/{srv9,trace-syscalls,gdbpoke9,mkfs9,drive9}.py
  key_links:
    - "HARNESS_BIN env (devshell shellHook, store path) -> shell callers; std::env::current_exe() fallback inside trace-syscalls for its own srv9/gdbpoke9 spawns"
    - "drive9 --steps FILE (post-shell step table) <- generated steps files in run-gate.sh / subset-exec.sh, replacing the gatedrive9.py / subsetdrive9.py heredocs"
    - "guest: nix build .#harness --out-link (gcroot) -> disk/mkfs wrapper in donor-ns-build.sh"
---

<objective>
Reimplement the five Python harness tools in build/aarch64/ as ONE std-only Rust binary
`harness` (new cargo crate at build/harness/) with subcommands srv9, trace-syscalls,
gdbpoke9, mkfs9, drive9 (D-01, D-02) — drop-in CLI-compatible (D-03) — rewire all shell
callers (D-04), wire flake.nix package + devshell Rust toolchain (D-05, D-06), prove the
five equivalence gates (D-07), delete the .py originals in the same task that passes the
gates, and update README tool references (D-08). r9 repo untouched (D-09).

Purpose: the repo converges on Rust (r9 kernel); Python host tooling is a stranded
dependency the user wants gone (locked motivation).
Output: build/harness crate, flake wiring, rewired callers, gate evidence, no .py.
</objective>

<execution_context>
@/Users/neilskinner/.config/opencode/gsd-core/workflows/execute-plan.md
@/Users/neilskinner/.config/opencode/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/STATE.md
@build/aarch64/srv9.py
@build/aarch64/gdbpoke9.py
@build/aarch64/mkfs9.py
@build/aarch64/drive9.py
@build/aarch64/trace-syscalls.py
@build/aarch64/boot-9qemu.sh
@build/aarch64/run-gate.sh
@build/aarch64/subset-exec.sh
@build/aarch64/donor-ns-build.sh
@build/aarch64/README.md
@flake.nix

Locked decisions D-01..D-10 = the ten DESIGN DECISIONS in the planning constraints
(one crate/one binary; std-only; drop-in CLI; caller+H harness resolution; guest mkfs
via per-system flake package; flake package+devshell; five gates then delete; README;
r9 out of scope; gates self-drive lima/qemu on this darwin-aarch64 host).

Porting-fidelity notes mined from 02-REVIEW-DISPOSITION (preserve, do not "fix" silently):
srv9 Twstat rename deletes an existing target (IN-05 — harness semantics the subset gate
may rely on; keep, document). srv9 fid table becomes per-connection in Rust (IN-04 —
strictly more correct, behavior-identical under the gates' single-active-connection use).
mkfs9 bare `d` mode must not crash (IN-08): stripped-empty mode falls back to copying the
source mode (unreachable with the pinned protos).
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: harness crate + flake wiring + srv9/gdbpoke9/mkfs9/drive9 subcommands</name>
  <files>build/harness/Cargo.toml, build/harness/Cargo.lock, build/harness/src/main.rs, build/harness/src/regex.rs, build/harness/src/srv9.rs, build/harness/src/gdbpoke9.rs, build/harness/src/mkfs9.rs, build/harness/src/console.rs, build/harness/src/drive9.rs, flake.nix</files>
  <behavior>
    - regex.rs (std-only — D-02 forbids the regex crate): a small byte-oriented backtracking engine covering exactly the pattern families the drivers use: multiline ^/$ anchors, literals, '.', escapes \r \s \b, [...] classes incl. ranges, greedy * ? +, (a|b) alternation, case-insensitive literal mode. Tests: every pattern used by drive9/run-gate/subset-exec/trace-syscalls (PANIC 'panic|trap:' ci, 'bootargs is \(tcp', '^__GSD_SHELL_OK__\r?$', '^term% $', '^ *2', '(UTC|GMT) 20[0-9][0-9]', '\src\r?$', '\bhfactotum\b', 'usage: .*hipconfig', ENTRY-line shape) — positive AND negative cases, incl. the WR-04 echo-attack non-matches.
    - srv9: over a 127.0.0.1 ephemeral-port server the test client speaks raw 9P2000 (LE codec): Tversion/Tauth(Rerror "authentication not required")/Tattach/Twalk/Topen/Tread(file + sorted dotfile-skipped directory, whole-entry reads)/Tstat/Tclunk; writable-prefix leg: Tcreate/Twrite/Tremove/Twstat rename under /tmp, Rerror "read only" outside it, qid.vers bump observable across mutations. Root confinement: walk '../..' never escapes.
    - mkfs9: fixture protos -> list mode prints cleanname'd source paths on STDOUT (nothing else), materialize mode builds the expected tree with modes, '$env' expansion, '+' recursion, missing source warns on stderr and skips the subtree, ARGBEGIN clustering (-aos == -a -o -s), bare-d mode does not panic (copies source mode).
    - gdbpoke9: against a mock RSP peer on a unix socket the client sends $?#.. / M write / D with +ACKs and checksums; watch-file poll gates the connect; stderr carries the 'poked' line after N attempts; exit 1 on deadline, 2 on unknown arg.
    - drive9: base mode flags identical to drive9.py (--serial/--qmp/--log/--budget); --steps FILE parses the tab-separated table (cmd, echo-pattern or '-', out-pattern, timeout-secs, label; '#' comments; default echo = first 16 bytes of cmd, metachar-escaped).
  </behavior>
  <action>
    Order matters: (1) crate skeleton — Cargo.toml ([dependencies] EMPTY per D-02; edition 2021; name "harness"), src/main.rs dispatching argv[1] to the five subcommands with `usage: harness <srv9|trace-syscalls|gdbpoke9|mkfs9|drive9> ...` on stderr + exit 2 for unknown/missing; stub modules. (2) flake.nix: add `harness = pkgs.rustPlatform.buildRustPackage { pname = "harness"; version = "0.1.0"; src = ./build/harness; cargoLock.lockFile = ./build/harness/Cargo.lock; }` to packages forAllSystems (no hasAttr gate needed — pure std builds on all four systems; the r9 gate exists only because r9 ships no x86_64-darwin package); merge into the existing packages attrset, keep the minimega optionalAttrs untouched. devShells: add `cargo rustc` (stable nixpkgs — D-06; NO rust-overlay, NO nightly) and `self.packages.${system}.harness` to mkShell packages, and `export HARNESS_BIN="${self.packages.${system}.harness}/bin/harness"` in shellHook (nix-store-first default per D-04). Do not disturb the r9 input, boot-r9/boot-r9-smoke apps, or any existing devshell line (D-06). Commit Cargo.lock (zero-dep lockfile keeps sandboxed nix build network-free).
    (3) Implement test-first (RED->GREEN) in this order: regex.rs, then the four tools.
    - srv9.rs: port srv9.py semantics 1:1 — MAXMSIZE 65536, LE codec, message set Tversion/Tauth/Tattach/Twalk/Topen/Tcreate/Tread/Twrite/Tclunk/Tremove/Tstat/Twstat/Tflush + Rerror; qid from st_ino (dir=0x80), vmap vers bumps, sorted listings skipping dotfiles, whole-entry dir reads, per-connection fid table (std::thread per conn, Mutex where shared), --writable-prefix repeatables with 0777/0666 honest modes + rename-deletes-target Twstat (documented preservation), stderr startup line byte-exact 'srv9: serving {realroot} on {host}:{port} (9P2000, {mode})' (callers poll for 'srv9: serving'), usage/unknown-arg/not-a-dir exit 2.
    - gdbpoke9.rs: raw RSP — pkt = '$'+data+'#'+%02x checksum, '+' ACK handling, '?' state query, 'M addr,len:hex' write, 'D' detach, watch-file byte-substring poll (default str 'bootargs is'), retry whole connect+write until deadline; defaults addr=0xFFFFFFFFB00A0014 value=0 bytes=4 deadline=120; stderr 'gdbpoke9: poked ...' MUST keep the substring 'poked' (callers grep it); exits 0 landed / 1 timeout / 2 args.
    - mkfs9.rs: proto grammar (tab=8/space=1 indent depth, name [mode uid gid source], '$env' expansion, '#' comments, '+' recursive copy of current source dir), implicit/explicit source resolution + Plan 9 cleanname, ARGBEGIN clustering with -a -o -x -p -v -D -r -U no-ops and -s/-d taking values, list mode prints ONLY paths on stdout, stderr 'processing {proto}' per file and 'file system made' on materialize, exit 2 usage.
    - console.rs + drive9.rs: shared Log (512KiB window/256KiB trim ring + file), reader thread, qmp_cont (greeting -> qmp_capabilities -> cont), the bootargs/auth/shell/prompt drive (rc-caret-split probe 'echo __GSD_SHELL_^OK__', anchored order-dependent prompt proof — WR-04 discipline). drive9 base mode = drive9.py exactly (whole-command sends, exits 0/1/2, 'drive9: ...' stderr prefixes). --steps FILE = the gatedrive9/subsetdrive9 post-shell phase generalized: paced sends (50ms/byte), echo-verified delivery (3 attempts; echo failure retries, clean echo + missing out-pattern is a genuine failure — no retry), panic fast-fail throughout, per-step 'drive9: step i/n OK (label)'; this replaces both embedded heredocs (D-04).
    Errors: Box<dyn Error> or a tiny local error enum per D-02; no external crates anywhere — grep the crate for 'extern crate' must find nothing.
  </action>
  <verify>
    <automated>nix develop -c cargo test --manifest-path build/harness/Cargo.toml && nix build .#harness && nix develop -c sh -c 'harness srv9 2>/dev/null; test $? -eq 2' && nix develop -c sh -c 'harness 2>/dev/null; test $? -eq 2' && test "$(nix develop -c harness mkfs9 2>&1 >/dev/null | head -1)" = "usage: mkfs9 [-a] [-s oldroot] [-d dest] proto ..."</automated>
  </verify>
  <done>
    cargo test green (all behavior bullets), nix build .#harness produces bin/harness on
    aarch64-darwin, the five subcommands present with drop-in flag surfaces, usage/exit-2
    discipline confirmed, devshell exports HARNESS_BIN at a real store path. No caller or
    .py file touched yet.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: trace-syscalls subcommand — orchestrator + parser port</name>
  <files>build/harness/src/trace.rs, build/harness/src/main.rs</files>
  <behavior>
    - ENTRY-line parse: "pid text name pc [args...]" with the pc as BARE hex and possibly the last field (end-of-line case); return fragments beginning " = " never match. Fixture tests from the committed trace artifacts' fragment shapes.
    - syscall set/count extraction + artifact writers reproduce the committed 04-01-d14-trace.txt / 05-03-trace.txt formats (header comment lines, per-syscall rows) byte-shape; stderr summaries keep '== PASS: D-14 trace complete' / '== PASS: real-chain trace complete' and the '   {number} {name} (×{count})' listing.
    - CLI: 'harness trace-syscalls [--boot-chain] [--help]' — same mode split, same env overrides (HARVEY, HARVEY_VM, BOOT_BUDGET, SRV9_PORT) plus new additive TRACE_OUT artifact-path override (used by gate e to avoid clobbering committed evidence); USAGE text no longer names python3.
  </behavior>
  <action>
    Port trace-syscalls.py (1437 lines) to src/trace.rs, reusing Task 1's console.rs boot drive instead of the duplicated inline copy, and spawning the 9P server / wake poker as `Command::new(harness_bin).arg("srv9")/arg("gdbpoke9")...` where harness_bin = $HARNESS_BIN if set else std::env::current_exe() (D-04). Preserve: preconditions (harvey checkout shape, donor pin 350266104f via git rev-parse, ROOTFS completeness, limactl presence + VM exists + auto-start, qemu presence; the srv9.py/gdbpoke9.py file checks become a harness-bin resolvability check; nsrun.sh check stays); guest compile + targeted CONF=qemu rebake via ssh/rsync/'nix develop -c bash nsrun.sh --wrap' Command argv shapes unchanged; the guest-side heredoc scripts (trace-ns-rebuild, boot-chain variants) carried over as raw string literals — they are bash, not python; qemu dumpdtb + paused-launch argv unchanged; serial capture-before-resume discipline, bootargs/auth answers, shell probe, pid capture via __GSD_PID__, trace-loop rc staging, DONE marker, panic fast-fail; parser: fragment reassembly + ENTRY hand-parse (split_whitespace; pc = strip 0x-less hex), syscall-name/number/count tables, artifact writers, both main modes' PASS lines. Argument/exit discipline: --help/-h -> USAGE on stderr exit 0; failures die() with 'trace-syscalls: error: ...' exit 1.
  </action>
  <verify>
    <automated>nix develop -c cargo test --manifest-path build/harness/Cargo.toml && nix develop -c harness trace-syscalls --help 2>&1 | head -1 | grep -q 'usage: harness trace-syscalls'</automated>
  </verify>
  <done>
    Parser/writer unit tests green incl. bare-hex-pc and end-of-line ENTRY fixtures; both
    modes wired behind the subcommand; no python interpreter referenced anywhere in the
    crate; live end-to-end run deferred to Task 3's gate e (it needs the rewired callers
    and lima/qemu).
  </done>
</task>

<task type="auto">
  <name>Task 3: caller rewiring + docs + five equivalence gates + .py deletion</name>
  <files>build/aarch64/boot-9qemu.sh, build/aarch64/run-gate.sh, build/aarch64/subset-exec.sh, build/aarch64/donor-ns-build.sh, build/aarch64/README.md, build/aarch64/trace-boot.rc, build/aarch64/srv9.py, build/aarch64/trace-syscalls.py, build/aarch64/gdbpoke9.py, build/aarch64/mkfs9.py, build/aarch64/drive9.py</files>
  <precondition>host has the phase-2..5 infrastructure live: lima VM 'harvey' (aarch64-linux, nix inside), .build/9front-donor at pin 350266104f, .build/9front-dist artifacts — gates self-drive lima/qemu per D-10; long TCG runs expected (BOOT_BUDGET 600+).</precondition>
  <reversibility rating="reversible">all edits + deletions are git-tracked; `git revert` restores the Python harness exactly.</reversibility>
  <action>
    Rewire the four callers (D-04). Common pattern replacing the PYTHON resolution block:
    HARNESS_BIN="${HARNESS_BIN:-$(command -v harness || true)}" then err if unset/non-executable (message names HARNESS_BIN and the nix devshell). Replace the python one-liner port-probe loops with a poll loop on `grep -q 'srv9: serving' "$TMPD/srv9.log"` (the Rust server prints it after listen(); keep the kill -0 liveness leg and the 60s cap). Replace every "$PYTHON" "$HARVEY/build/aarch64/{srv9,drive9,gdbpoke9}.py" invocation with "$HARNESS_BIN" {srv9,drive9,gdbpoke9} keeping all flags byte-identical (srv9 'serve --root ... --host ... --port ... [--writable-prefix ...]', gdbpoke9 '--sock ... --watch-file ... --watch-str ... --deadline ...', drive9 '--serial ... --qmp ... --log ... --budget ...'). Update header comments that name the .py tools.
    - run-gate.sh: DELETE the gatedrive9.py heredoc; generate a steps file instead (two rows: '/boot/hello' with out-pattern 'hello aarch64 harvey' timeout 120 label '/boot/hello'; 'echo __GSD_EXIT__$status' with out-pattern '^__GSD_EXIT__\r?$' timeout 60 label 'exit-status probe') and invoke "$HARNESS_BIN" drive9 --steps "$TMPD/steps.tsv" --serial ... --qmp ... --log ... --budget ... . All RUNLOG greps unchanged.
    - subset-exec.sh: DELETE the subsetdrive9.py heredoc; emit the SAME step table (the 20 STEPS rows + FINAL SUBSET-OK sentinel row, patterns/timeouts/labels preserved verbatim) as a steps file; same drive9 --steps invocation; srv9 keeps both --writable-prefix flags.
    - donor-ns-build.sh (D-05): replace `install -m 755 "$HARVEY/build/aarch64/mkfs9.py" "$SCRATCH/disk/mkfs"` with: `nix build "$HARVEY#harness" --out-link "$SCRATCH/harness-pkg"` (gcroot pins the store path in the guest) then write a $SCRATCH/disk/mkfs wrapper `#!/bin/sh` / `exec "$SCRATCH/harness-pkg/bin/harness" mkfs9 "$@"` (chmod 755). Keep the explanatory comment block, updated to name the Rust harness. The mk-side invocation `{disk/mkfs -aos / $BOOTFSPROTOS}` is untouched.
    - Docs (D-08): README.md — tool table row (helpers now 'harness <subcommand>'), the python3 mention in the devshell line becomes cargo/rustc + harness, the srv9.py mention in the ufs-over-tcp note. trace-boot.rc's comment now names 'harness trace-syscalls --boot-chain'. Do NOT touch .planning/codebase/*.md.
    Then the equivalence gates (D-07), in this order, each must PASS before the next:
      1. (c) mkfs9 A/B INSIDE the lima guest while python3 still exists there: run the old tool (build/aarch64/mkfs9.py, pre-deletion) and the new ('harness mkfs9') against the pinned bootfs.proto+disk.proto+devusb.proto with identical -s/-d into two scratch trees — `diff -r` the trees empty AND diff the `-aos /` list-mode stdout empty. Then run donor-ns-build.sh's install path (wrapper in place) and re-run the donor bootfs materialization with the wrapper as disk/mkfs.
      2. (a)+(b)+(d): `sh build/aarch64/boot-9qemu.sh` green end-to-end — the guest attaching/reading its whole root over srv9 IS the 9P attach/read gate; the 'poked' grep IS the gdbpoke9 gate; the term% proof IS the drive9 gate.
      3. `sh build/aarch64/run-gate.sh` and `sh build/aarch64/subset-exec.sh` green — exercises drive9 --steps (both shapes) and srv9 writable-prefix mutations.
      4. (e) `HARNESS_BIN=... harness trace-syscalls` hello path with TRACE_OUT pointed at a scratch file; extract the syscall-name set from the scratch artifact and from the committed build/aarch64/04-01-d14-trace.txt, sort -u both, diff must be empty (counts may legitimately drift — D-07e). Restore/git-checkout any evidence file the run touched.
    Only now, in THIS task's final commit (D-07): git rm build/aarch64/{srv9,trace-syscalls,gdbpoke9,mkfs9,drive9}.py. Post-deletion sanity: `sh -n` the four scripts, `nix flake show` evaluates, and one quick post-deletion smoke — `nix develop -c sh -c 'command -v harness'` resolves and `harness drive9 2>/dev/null; test $? -eq 2`. Any missed caller would now fail loudly; the green gates above already prove the real paths.
  </action>
  <verify>
    <automated>sh -n build/aarch64/boot-9qemu.sh build/aarch64/run-gate.sh build/aarch64/subset-exec.sh build/aarch64/donor-ns-build.sh && nix flake show >/dev/null && test ! -e build/aarch64/srv9.py && test ! -e build/aarch64/trace-syscalls.py && test ! -e build/aarch64/gdbpoke9.py && test ! -e build/aarch64/mkfs9.py && test ! -e build/aarch64/drive9.py && nix develop -c sh -c 'command -v harness >/dev/null'</automated>
  </verify>
  <done>
    All five gates green: (a) 9P attach/read served by harness srv9 across a full donor
    boot, (b) gdbpoke9 wake poke landed ('poked' in the log), (c) mkfs9 byte-identical
    trees + list output vs the Python original and the donor bootfs rebuilds via the
    wrapper, (d) boot-9qemu.sh to the rc prompt, plus run-gate and subset-exec green,
    (e) hello-path trace syscall-name SET equal to the committed 04-01 trace. The five
    .py files are deleted in the same commit as the passing gates; callers, README, and
    trace-boot.rc reference only the harness; no python3 dependency remains anywhere in
    the build/aarch64 harness path.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| guest→srv9 | 9P2000 messages from the qemu guest arrive over guestfwd tcp/127.0.0.1; malformed or malicious frames cross into the host file server |
| guest→drivers | serial/QMP/gdbstub payloads (guest-controlled bytes) feed regex matching and the trace parser |
| guest fs mutations | --writable-prefix lets the guest create/write/rename/remove host files under scoped prefixes |
| nix supply chain | harness package builds from the committed crate + committed Cargo.lock |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-GIQ-01 | Tampering/DoS | srv9.rs wire decode | high | mitigate | length-prefix sanity (size < 7 or > MAXMSIZE+24 drops the conn, as in the original), all integer reads bounds-checked slices, no panic paths — handler errors map to Rerror; unit tests feed truncated/malformed frames |
| T-GIQ-02 | Info disclosure | srv9.rs Twalk | high | mitigate | root-confinement: every resolved path canonicalized and must equal-or-be-under the served root ('..' clamped at root); traversal test included |
| T-GIQ-03 | Tampering | srv9.rs writable ops | medium | mitigate | writable check on canonicalized paths only under --writable-prefix prefixes; default remains read-only Rerror; rename target confined to the same writable check |
| T-GIQ-04 | Tampering | regex.rs / trace.rs parsing of guest bytes | low | mitigate | regex engine operates on bounded buffers with step-capped backtracking; parser ignores non-matching lines by construction (fixture tests) |
| T-GIQ-SC | Tampering | cargo builds | high | mitigate | zero external dependencies (D-02) — [dependencies] empty, committed Cargo.lock has no third-party entries; sandboxed nix build is network-free; nothing to audit beyond the crate itself |
</threat_model>

<verification>
Task 1+2: cargo test (protocol, proto-grammar, RSP, regex, parser fixtures) + nix build
.#harness + CLI surface/exit-code checks. Task 3: the five D-07 gates run live on this
darwin-aarch64 host (lima + qemu self-driven), then file-absence + syntax + flake-eval
checks post-deletion.
</verification>

<success_criteria>
One Rust binary replaces five Python tools with byte-compatible CLI surfaces; every
caller rewired; flake provides the package per-system and the devshell toolchain; the
donor boot, run-gate, subset-exec, and hello-path trace all pass with the .py files
deleted from the tree; r9 repo untouched.
</success_criteria>

<output>
Create `.planning/quick/261103-giq-reimplement-the-five-python-harness-tool/261103-giq-01-SUMMARY.md` when done
</output>
