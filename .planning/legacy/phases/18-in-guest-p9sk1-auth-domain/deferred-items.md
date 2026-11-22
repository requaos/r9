# Phase 18 Deferred Items

## [18-06] mkimage auth-section resolver fails inside `nix develop` (vendored-awk shadowing)

- **Found during:** 18-06 Task 1 (substrate image rebuild), 2026-11-14
- **Class:** environment footgun, out-of-scope for 18-06 (mkimage.sh untouched this plan; not a regression — no code change caused it)
- **Issue:** The harvey devshell (`nix develop`) puts `.build/hostbin-linux-x86_64/bin` FIRST on PATH (by design, for the namespace build's 6c/mk/ar). That directory contains Harvey's vendored Plan 9 `awk`, which resolves before nixpkgs gawk inside the devshell. mkimage.sh §14's auth-suite resolver parses multi-word `TARG` lines through `awk`, and the vendored awk silently produces NO output for the program's `[[:space:]]` regex — so `cmd/auth/secstore`'s 4-word TARG parses as 0 words and the resolver stages only `hsecstore`, failing with `auth bake: sweep subject hsecstored missing from the computed auth-suite set (the resolver lost it)`.
- **Not a gate break:** the sanctioned gate environments (the flake apps, e.g. `nix run .#boot-r9-auth-x64`) put `$PLAN9/bin` LAST and never see hostbin awk; mkimage completes rc=0 there. Verified during 18-06: re-running mkimage with gawk's dir prepended (GNU awk resolving first) bakes everything green, including bootfs-auth.paq (2566503 bytes).
- **Remediation options (for whoever next touches mkimage):** resolve a GNU awk once at mkimage top the way GREP is resolved (a `GAWK=$(...)` guard rejecting the vendored/plan9 awk), or drop `[[:space:]]` for a plain `[ \t]` class the vendored awk handles.
- **Reopener:** any executor running `sh build/x86_64/mkimage.sh` inside `nix develop` (the §1-13 sections are unaffected — only §14's awk-dependent resolver).

## 18-07 (r9 devip two-write fix)

- **x86_64's unwired devip deadline leg (`tick_check`)** — `stack::tick_check` (r9 port/src/devip/stack.rs:340) is called from aarch64's 1ms tick (platform/virt/timer.rs, 08-04) but x86_64's q35 timer tick (x86_64/src/platform/q35/timer.rs::tick) never calls it: with all devip procs parked, NET_DEADLINE passes unwatched, so retransmit, keepalive, FIN/linger aging, and the connect Etimedout deadline never progress on x86_64. NOT fixed by 18-07 (the donor-faithful socket config — Nagle off, no delayed ACK — removes the pattern's TIMER dependence entirely, which is the donor's own shape); the gap predates this phase (Phase 13's x86_64 port never carried the leg over). Reopener: the first venue needing timer-mediated TCP progress under full park (an ether-loss retransmit venue, a connect-timeout gate). Found during 18-07 Task 1's mechanism diagnosis; source-verified (no x86_64 caller of tick_check).
