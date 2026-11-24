# Phase 3 closing evidence (plan 03-10)

The evidence the Mac can produce before the owner stop: the clippy probe that proves the lint
contract rejects an undocumented `unsafe` block in `ds/` (Task 1), the audit count
reconciliation (Task 1), the full suite twice (Task 2) and the Miri timings (Task 2).  Every
verdict below comes from the run recorded beside it; nothing is written for a command that did
not run.

## Task 1 — the `UNSAFE-AUDIT.md` count and the clippy probe

### Source enumeration (the audit's `ds/` count)

Method: for every `.rs` file under `ds/src/tpht/`, strip `//` line comments and list each
remaining occurrence of the `unsafe` token, with file, line and kind.  The same scan over
`crypto/src` and over `ds/src` outside `tpht/` finds nothing.

| Scope | Lint-visible blocks | `unsafe impl` | `unsafe fn` | `unsafe extern` | Test-only blocks |
|---|---|---|---|---|---|
| `ds/src/tpht/` | 1 (`flattened.rs:440`, `deref_tiny`'s `get_unchecked`) | 0 | 0 | 0 | 0 |
| `crypto/src` | 0 | 0 | 0 | 0 | 0 |
| `ds/src` outside `tpht/` | 0 | 0 | 0 | 0 | 0 |

AUDIT-COUNT tpht source=1 audit=1

### The clippy probe (scratch edit, not committed)

A clean tree at the start (`git status --porcelain -- ds crypto xtask` printed nothing).  One
scratch edit: the six-line `// SAFETY:` comment above the `unsafe` block in
`ds/src/tpht/flattened.rs` (`get_unchecked` in `deref_tiny`) was deleted; nothing else changed.

Command: `nix develop -c cargo xtask clippy` (the default arch form, which is what hosted CI
runs; `--arch aarch64 --config virtshell` is pre-existing red and is not a criterion).

Decisive output:

```text
error: unsafe block missing a safety comment
   --> ds/src/tpht/flattened.rs:434:21
    |
434 |         let words = unsafe { self.arena.get_unchecked(slot) };
    |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
    = help: consider adding a safety comment on the preceding line
    = note: requested on the command line with `-D clippy::undocumented-unsafe-blocks`

error: could not compile `ds` (lib) due to 1 previous error
clippy failed
EXIT=1
```

VERDICT clippy-rejects-undocumented-unsafe: PASS

Exit code: `1`.  Reverted with `git checkout -- ds/src/tpht`; `git status --porcelain -- ds`
then printed nothing, so the probe is not in any commit.
