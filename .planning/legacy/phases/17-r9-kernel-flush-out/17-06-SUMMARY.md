---
phase: 17-r9-kernel-flush-out
plan: 06
subsystem: userspace-rc
tags: [rc, plan9-c, doredir, dup, exec, errstr, argv-stack, harvey-userspace]

requires:
  - phase: 17-r9-kernel-flush-out (17-05)
    provides: the r9 pin 09b59f6f2 (r9 59d28fa) with all gates green — the sweep base this plan re-greens; the namec/#18 audit precedent shape (verify+pin with verbatim-quote evidence)
provides:
  - rc hardened at its three known corners (one Harvey class commit 6779df7d7): doredir's checked Dup in both redirect arms, Execute's first-failure errstr capture, the argv-stack invariant guards at Xqdol/Xcount
  - the re-greened rc-driven sweep family at the 17-05 pin (smoke-x64 + tier1-x64 ALL STAGES PASS with the fixed rc baked; the 38-member witness manifest re-pinned through the donor venue rebuild)
  - WINDOWS #46/#49/#65 closed with native resolve fields (Harvey-SHA reasons); FLUSHOUT rows #46/#49/#65 filled
affects: [17-07 (rc enters the CR-07 stress phase hardened — the load driver), Phase 18 (auth substrate drives rc), Phase 19 (cpu/drawterm/rio drive rc), every composed gate that boots the chain image]

actuals:
  tokens: 9200    # chars/4 over the realized diff (26240 diff chars pre-SUMMARY + ~10.5k SUMMARY chars)
  tasks: 3
  commits: 2      # MEASURED: git rev-list --count 7cc2b889d..HEAD (the task commits; the plan metadata commit follows)

# Plan-commit ledger (#3968) — measured
commits: 2
plan_head_before: 7cc2b889d2769c5d2cfbb11e0f5e9cd2ddb64091
plan_head_after: 406d5cbba04734784db32dbce0d60a1fae8dc849

tech-stack:
  added: []
  patterns: ["the 06-07 invariant-guard shape applied at every pop-then-pushword op (the audit completes the Xbackq fix's class)", "a userspace class commit's proof = the rc-driven sweeps re-greening + the bake's own digest discipline (the witness manifest re-pin is the byte-change procedure, no pin advance)"]

key-files:
  created:
    - .planning/phases/17-r9-kernel-flush-out/17-06-SUMMARY.md
  modified:
    - sys/src/cmd/rc/simple.c
    - sys/src/cmd/rc/plan9.c
    - sys/src/cmd/rc/exec.c
    - .planning/WINDOWS.md
    - .planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md

key-decisions:
  - "17-06: the three rc fixes land as ONE class commit (D-11's letter) — 6779df7d7, style(6)-conformant, panic sites untouched; no r9 pin mechanics anywhere (rc binaries are Harvey-baked; the plan's commits touch no flake.lock)"
  - "17-06: #49's fix is the Xqdol/Xcount pair — the audit found exactly two remaining pop-then-pushword ops in the exec.c orbit (the class that reaches the recorded 'pushword but no argv' panic); the pop-then-DEREF ops (Xdol/Xassign/Xconc/Xlocal) are a different crash surface (fault, not the recorded panic) and stay unguarded — minimal, exactly the recorded class"
  - "17-06: NO fixture for any corner — no sweep fails a dup (#46), guest sessions invoke by absolute path so no sweep exercises the masking path (#65), and the #49 class is timing-only (A6); each corner's proof is invariant-audit + sweeps re-green per D-11's escape clause, recorded per-corner in the ledger rows"
  - "17-06: the byte change in /amd64/bin/rc trips mkimage's witness-manifest digest (the 14-08 identical-6.outs discipline) — the remedy is the failure message's own named procedure (build/mm/build-9k10t1.sh re-run: donor kernels rebuilt green, 38-member manifest re-pinned); gitignored artifacts, no commit, no pin advance"
  - "17-06: services-x64 cannot re-green this plan — the pre-existing #68 dns-stage kernel trap (PF pc=0x0 after `devmnt: server error: cs: file does not exist`) fires identically at this pin; the kernel is byte-identical (r9 59d28fa) so the rc class commit is excluded by construction, every rc-driven stage before the trap was green, and the root-cause is owned by 17-07 (the deferred record's own assignment) — this plan is PROHIBITED from touching it (no flake.lock)"

patterns-established:
  - "rc corner fixes are proven by the rc-driven sweep family re-greening plus the bake's digest discipline — never host unit tests (Pitfall 6); the witness manifest re-pin is the standing procedure when a /amd64/bin binary's bytes change"

requirements-completed: [FLUSH-04]

coverage:
  - id: D1
    description: "the three rc corner fixes (#46 checked doredir Dup in both arms, #65 Execute's first-failure errstr, #49 argv-stack guards at Xqdol/Xcount) as one style(6)-conformant class commit"
    requirement: FLUSH-04
    verification:
      - kind: other
        ref: "compile gate: nix develop -c sh build/x86_64/stage-build.sh → STAGE-BUILD-OK, chain rc rebuilt post-edit (out/rc mtime 21:18:53 > last source edit 21:18:21), S_MAGIC verified"
        status: pass
      - kind: other
        ref: "nix run .#boot-r9-smoke-x64 → exit 0, ALL STAGES PASS (boot, typed+answered echo, ls/ps/cat sweep, DEL safety, restart)"
        status: pass
      - kind: other
        ref: "nix run .#boot-r9-tier1-x64 → exit 0, ALL STAGES PASS (the sourced tier sweep's rc pipelines under the fixed rc, zero T1-FAIL, zero panic, exact SKIP set = 0 names)"
        status: pass
    human_judgment: false
  - id: D2
    description: "WINDOWS #46/#49/#65 closed with native resolve fields (Harvey-SHA + sweep-name reasons, #49 with the A6 note) and the three FLUSHOUT rows filled"
    requirement: FLUSH-04
    verification:
      - kind: other
        ref: "for id in 46 49 65: grep -A9 '\"id\": $id,' .planning/WINDOWS.md | grep '\"status\": \"fixed\"' → RC-ROWS-CLOSED; row 65 reason greps the HEAD class-commit SHA → REASON-CITES-HEAD-SHA; FLUSHOUT | #46 | / | #49 | / | #65 | present, zero pending → FLUSHOUT-RC-FILLED"
        status: pass
    human_judgment: false

# Metrics
duration: 41 min
completed: 2026-11-13
status: complete
---

# Phase 17 Plan 06: The Three rc Corner Fixes Summary

**D-11 executed: rc's three known corners fixed as one Harvey class commit (checked doredir dups in both redirect arms, Execute's first-failure errstr surviving the path loop, argv-stack guards at the pop-then-pushword ops) — sweep-proven by smoke-x64 + tier1-x64 ALL STAGES PASS with the fixed rc baked, the witness manifest re-pinned, and WINDOWS #46/#49/#65 closed**

## Performance

- **Duration:** 41 min
- **Started:** 2026-11-13T02:04:14Z
- **Completed:** 2026-11-13T02:45:00Z
- **Tasks:** 3
- **Files modified:** 5 (3 rc sources + WINDOWS.md + 17-FLUSHOUT.md)

## Accomplishments

- **The class commit `6779df7d7`** carries all three fixes across `sys/src/cmd/rc/{simple,plan9,exec}.c`, style(6)-conformant, panic sites untouched (verified: exec.c:46/:55/:58 outside the diff)
- **Compile green through the established path**: `stage-build.sh` chain rc rebuilt from the edited sources (S_MAGIC verified), and the bake carried it into every image
- **The rc-driven sweeps re-greened at the 17-05 pin** (r9 59d28fa, no pin advance — the plan's commits touch no flake.lock): smoke-x64 and tier1-x64 each exit 0 with ALL STAGES PASS
- **The ledger closed**: WINDOWS #46/#49/#65 (JSON + table, resolved_at stamped, Harvey-SHA reasons), FLUSHOUT rows filled; ledger counts 55/10 → 52/13 open/fixed

## The Three Fix Shapes (quoted)

**#46 — doredir's checked Dup (simple.c):** both arms now check; a failed dup reports and exits with errors — never an exec against an unset target descriptor:

```c
static void
dupfailed(int from, int to)
{
	pfmt(err, "rc: can't dup %d to %d: %r\n", from, to);
	flush(err);
	Exit("error");
}
...
		case ROPEN:
			if(rp->from!=rp->to){
				if(Dup(rp->from, rp->to) < 0)
					dupfailed(rp->from, rp->to);
				close(rp->from);
			}
			break;
		case RDUP:
			if(Dup(rp->from, rp->to) < 0)
				dupfailed(rp->from, rp->to);
			break;
```

The `Exit("error")` is the exits-with-error idiom for BOTH call contexts (the exitnext `exec` arm and the execforkexec child — the child's parent collects the error status through Waitfor).

**#65 — Execute's first-failure errstr (plan9.c):** a candidate `cerrstr` seeded on the first failure and preserved across later candidates; the stopping-reason break arms take the current reason verbatim per the function's own :383-389 comment contract:

```c
		rerrstr(cerrstr, sizeof cerrstr);
		if (strstr(cerrstr, " allocat") != nil ||
		    strstr(cerrstr, " full") != nil ||
		    strstr(cerrstr, "i/o error") != nil){
			strcpy(errstr, cerrstr);
			break;
		}
		/*
		 * #65 (17-06): keep the FIRST candidate's failure — only the
		 * last candidate's rerrstr used to survive the loop, so the
		 * trailing candidate's "can't open" masked the real reason an
		 * earlier (matching) candidate failed to exec.
		 */
		if(errstr[0] == '\0')
			strcpy(errstr, cerrstr);
```

**#49 — the argv-stack invariant guards (exec.c):** the audit completed the 06-07 class — Xbackq was the first pop-then-pushword op; Xqdol and Xcount are the remaining two (each `poplist()`s the variable-name list then `pushword`s the result onto the list below; if the popped list was the stack's only list, the pushword hits the recorded panic). The 06-07 guard shape, verbatim:

```c
	poplist();
	/* 17-06 (#49): like Xbackq's 06-07 case, the popped name list can
	 * be the stack's ONLY list — the result pushword below would then
	 * abort rc ("pushword but no argv", the flake class the backquote
	 * fixtures measured).  Restore the invariant every reader assumes:
	 * Xqdol leaves a list on the stack for the result. */
	if(runq->argv==0)
		pushlist();
```

(the Xcount twin carries the same guard before its `pushword(num)`).

## Task Commits

1. **Task 1: the three rc corner fixes — one class commit** - `6779df7d7` (fix)
2. **Task 2: the sweeps re-green** - no commit (no fixture landed — see the per-corner proof records; the manifest re-pin lives in gitignored `.build/`)
3. **Task 3: WINDOWS #46/#49/#65 closure + FLUSHOUT rows** - `406d5cbba` (docs)

**Plan metadata:** the docs commit that carries this SUMMARY (see below).

## The Sweep Re-Green Records (Task 2)

Precondition held: flake.lock's r9 input is `59d28fa` (17-05's advance), `/dev/kvm` present, nix 2.34.9.

| Gate | Result | Evidence |
|------|--------|----------|
| boot-r9-smoke-x64 | **exit 0 — ALL STAGES PASS** | boot, echo (typed + answered), empty-line safety, ls/ps/cat sweep, DEL safety, restart, pid-warning-absent, nic-bind; capture at `build/x86_64/smoke-r9.log` |
| boot-r9-tier1-x64 | **exit 0 — ALL STAGES PASS** | the sourced tier sweep from the boot flow under the fixed rc (38 T1-OK, zero T1-FAIL, zero panic, SKIP set = the sweep's declared 0 names), float canary, marker parity; capture at `build/x86_64/tier1-r9-x64.log` |
| boot-r9-services-x64 | exit 1 — the pre-existing #68 dns-stage kernel trap | every rc-driven stage GREEN (banner, paqfs, version, mntgen-verdict, net-bringup, cs-verdict), then `devmnt: server error: cs: file does not exist` → kernel PF at pc=0x0 → halt, driver times out at dns-verdict; capture at `build/x86_64/services-r9-x64.log` (the #68 evidence file, refreshed with this pin's capture) |

**The rc binary provenance** (the bake's own discipline): `build/x86_64/out/rc == /amd64/bin/rc`, sha256 `51f81de8…` — the stage-build chain compile and the donor venue's tier build produce identical bytes from the class commit's sources, and the tier bake digest-verifies them against the re-pinned 38-member manifest.

**The services A/B**: the failure is byte-for-byte the recorded #68 signature — last-green-stage `cs-verdict`, the same `devmnt: server error: cs: file does not exist` → PF pc=0x0 (cx=0x0) → `kernel halted` sequence, with the frame's `ax` shifted (0xffffffff802277b8 vs the recorded 0xffffffff802276f8/0xffffffff80227608) exactly per the 17-04 deferred record's link-layout-shift pattern. The kernel is byte-identical at this pin (no r9 commit, no flake.lock change), so the rc class commit is excluded by construction; the trap's root cause stays with 17-07 per the deferred record's own assignment ("the next plan that touches the services venue").

**No pin advance**: both plan commits touch no flake.lock (`git show --name-only 6779df7d7` = the three rc files; `406d5cbba` = the two ledgers); HEAD names no flake.lock → HARVEY-CLASS-COMMIT echoed; unpushed commit count 38 (the standing push-cadence convention, not this plan's gate).

## Per-Corner Proof Notes (the fixture assessment)

No fixture landed — each corner's honest record, per D-11's escape clause ("if the fixture route is disproportionate for a corner, record that corner's proof as the invariant-audit + sweeps re-green"):

- **#46** — no sweep ever fails a dup (the plan's own assessment). A targeted fixture (a redirect against a closed fd asserting `rc: can't dup N to M: %r` + non-nil status) is craftable but rides the fd-dup redirect form measured unreliable in sourced blocks (15-02's dialect lessons) or needs an existing expect-driver edit; disproportionate. Proof = the checked-return shape (quoted above) + the sweeps re-green.
- **#65** — guest sessions invoke by absolute path (single-candidate), so no sweep exercises the masking path. A fixture needs a writable fs + a crafted bad-executable + a multi-element `path` — unmeasured machinery on this venue; disproportionate. Proof = the first-failure capture shape + the sweeps re-green.
- **#49 (the A6 shape)** — the historical fires were HVF-timing-only (darwin, 2/~6 runs; zero on the linux TCG/KVM venue across the whole gate history); the class is not deterministically triggerable here. The fix is invariant-audit-grounded: the audit enumerated every pop-then-pushword op in the exec.c orbit and found exactly the two now guarded (the recorded panic is reachable only through that op shape). **Reopener: a live "pushword but no argv" fire on any venue.**

## Files Created/Modified

- `sys/src/cmd/rc/simple.c` - #46: dupfailed() + the checked Dup in doredir's ROPEN and RDUP arms
- `sys/src/cmd/rc/plan9.c` - #65: Execute's candidate cerrstr, first-failure preserved, break arms take the stopping reason
- `sys/src/cmd/rc/exec.c` - #49: the argv-stack guards at Xqdol (:715-722) and Xcount (:833-838); panic sites untouched
- `.planning/WINDOWS.md` - #46/#49/#65 closed (JSON + table, counts 55/10 → 52/13)
- `.planning/phases/17-r9-kernel-flush-out/17-FLUSHOUT.md` - the three rows filled (action/pin/commit/refs)

## Decisions Made

- #49's fix sites are the Xqdol/Xcount pair — the audit's finding that these are the only remaining pop-then-pushword ops (the exact class that reaches the recorded panic); the pop-then-deref ops are a different crash surface and stay untouched (minimal, exactly the recorded class)
- No fixture for any corner (the disproportionate-route records above); the sweeps re-green + the invariant shapes are the pin, per D-11's own letter
- The donor venue rebuild (manifest re-pin) executed without hesitation when the bake demanded it — the failure message names the procedure; it is the 14-08 discipline working as designed, not a plan gap

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The bake's witness-manifest drift blocked the first smoke run**
- **Found during:** Task 2 (the first smoke run)
- **Issue:** the class commit's rc byte change tripped mkimage.sh's tier-bake digest check ("the /amd64/bin set drifted from the donor venue's manifest: amd64/bin/rc") — the 14-08 identical-6.outs witness requires ONE binary set across both kernel venues
- **Fix:** re-ran `build/mm/build-9k10t1.sh` (the failure message's own named remedy): both donor kernels rebuilt green (9k10t1.elf, 58 embedded root files, k10 tree git-clean), the 38-member manifest re-pinned to the new set
- **Files modified:** none committed (gitignored `.build/9k10t1-tier-manifest.sha256` + the donor kernel artifacts)
- **Verification:** all subsequent gate bakes pass ("witness manifest verified: 38 members match"); out/rc == /amd64/bin/rc (51f81de8…)
- **Committed in:** nothing (the standing byte-change procedure, no repo change)

**2. [Plan-letter deviation] The services sweep cannot exit 0 this plan**
- **Found during:** Task 2
- **Issue:** the plan's verify demands all three gates exit 0; services-x64 exits 1 at the documented pre-existing #68 dns-stage kernel trap (WINDOWS #68 open since 17-04, A/B-proven pre-existing there, last green in the 15-08 era)
- **Resolution:** out-of-scope by the plan's own constraints — the fix needs r9 kernel commits + a pin advance, and this plan is prohibited from touching flake.lock; the A/B evidence (identical signature, kernel byte-identical at this pin, every rc-driven stage green before the trap) excludes the class commit; the root cause stays assigned to 17-07 per the 17-04 deferred record
- **Files modified:** none (the outcome is recorded in the FLUSHOUT #46 row and here)
- **Verification:** the trap signature match against the committed capture (only the link-layout `ax` delta differs, the documented pattern)

---

**Total deviations:** 2 (1 Rule-3 blocking auto-handled, 1 plan-letter deviation resolved by honest recording)
**Impact on plan:** No scope creep. The first is the standing digest discipline executing as designed; the second is a pre-existing kernel defect outside this plan's authority, fully evidenced and ledgered.

## Issues Encountered

- The services gate's pre-existing #68 trap (above) — the only issue; smoke and tier1 were green on their first post-manifest-repin runs
- Pre-existing unrelated working-tree state left untouched and unstaged: `sys/src/cmd/eqn/eqn.c` (a prior session's modification), untracked gate captures/research caches

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- rc — the load driver — enters 17-07 (the CR-07 stress phase) hardened at all three known corners; the sweeps prove the common paths unchanged
- 17-07 inherits: the services venue's #68 root cause (the deferred record's assignment), the #47/#50/#66 CR-07 fold, and the fresh services capture at this pin as its A-side baseline
- No pin advance this plan: the composed gates all ride r9 59d28fa; the next r9-side fix pass resumes the re-wire-then-pin cycle from a clean, fully-pushed-to-plan base (38 unpushed commits at close — the standing Harvey push cadence)

## Self-Check: PASSED

All six key files exist on disk; both task commits (6779df7d7, 406d5cbba) are ancestors of HEAD; the frontmatter parses (plan 06, status complete, commits 2 measured from the ledger, requirements [FLUSH-04]).

---
*Phase: 17-r9-kernel-flush-out*
*Completed: 2026-11-13*
