# 17-04 Deferred Items

## Deferred Items

- The services-x64 gate faults at the dns stage — a kernel NULL-call (PF at pc=0x0, cs=0x8, sp in the kstack window) immediately after `devmnt: server error: cs: file does not exist`; the driver times out at `dns-verdict` (stage timeout 900s), exit 1
  status: open
  **What:** Pre-existing relative to 17-04 — A/B-proven: the IDENTICAL trap frame fires at Harvey pin b02408cad (r9 658001b, pre-17-04) and at pin 02218e88c (r9 63bfda9, post-17-04); the only frame difference is `ax` (0xffffffff80227608 vs 0xffffffff802276f8 — the link-layout shift of the 17-04 code, same static symbol). Captures: `build/x86_64/services-r9-x64.log` (both venues, main + /tmp/harvey-ab worktree). Last green services run: the 15-08 pin era (eba6b07e2); services was not run in 17-01..17-03, so the fault's introducing commit sits somewhere in the 17-01..17-03 window (suspect orbit: the 17-02 exit_tail/fd-group drain fixes c4f2f2c/72edbba, or kernel-x64's re-point at q35churnshell whose per-exit churngate markers now print during the services sweep) — undetermined. Out of 17-04's scope (close-what-you-touch + the plan's fails_when names only over-rejection as this plan's battery risk); the root-cause belongs to the next plan that touches the services venue (17-05..17-07 own it or a dedicated fix).

- r9 repo-wide `cargo fmt --check` drift (~439 pre-existing diff sites across aarch64/port/x86_64/xtask, no fmt gate in r9's CI — xtask.yml runs check+clippy only); 17-04 kept its OWN added lines fmt-clean and introduced zero new clippy warnings (verified by stash-diff)
  status: open
  **What:** Pre-existing formatting drift; a repo-wide `cargo fmt` pass is a standalone cosmetic commit no current plan owns.
