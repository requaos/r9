---
phase: 261105-slx
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ".github/workflows/harvey.yml"
  - ".planning/ROADMAP.md"
  - ".planning/PROJECT.md"
  - ".planning/quick/261105-slx-fix-the-legacy-harvey-yml-workflow-to-bu/261105-slx-SUMMARY.md"
autonomous: true
requirements: ["261105-slx"]
estimate:
  tokens: 18000
  raw_tokens: 12000
  tasks: 2
  confidence: low

must_haves:
  truths:
    - ".github/workflows/harvey.yml runs the repo's own nix pipeline (nix run .#setup -> .#toolchain -> .#build) on ubuntu-latest from a clean checkout — the legacy guest-boot harness is fully replaced"
    - "Every action reference in harvey.yml is a 40-char SHA pin (checkout 11d5960a…, nix-installer-action 3138316d…); permissions: contents: read; concurrency group harvey-<ref> cancels superseded runs"
    - "The build job can never green on a swallowed build failure: toolchain products (pcc/cpp/ar/6c/6l/8c/8l) and build artifacts (amd64/9k10cpu.elf, amd64/bin/rc, amd64/bin/awk) are asserted in-job (the 02-12 exit-0 class)"
    - "Zero legacy-harness tokens survive in non-comment lines of harvey.yml (comment-filtered negative grep)"
    - ".github/workflows/r9-shell.yml and .github/workflows/phase6.yml are byte-untouched (git diff --quiet + clean porcelain)"
    - "Backlog 999.1 Test 1 (ROADMAP.md) and the PROJECT.md carried-over line are ticked with the PENDING FIRST PUSH caveat recorded; the SUMMARY records the hosted run as PENDING FIRST PUSH (nothing fabricated)"
  artifacts:
    - ".github/workflows/harvey.yml (rewritten, ~90 lines, 2 pinned actions, 1 job x 4 steps)"
    - ".planning/quick/261105-slx-fix-the-legacy-harvey-yml-workflow-to-bu/261105-slx-SUMMARY.md"
    - "scoped checkbox edits in .planning/ROADMAP.md (backlog 999.1 Test 1) and .planning/PROJECT.md (carried-over line)"
  key_links:
    - "nix run .#setup / .#toolchain / .#build resolve to real flake apps (flake.nix lines 132-134: setup/toolchain/build = mkApp …) — the verify gate re-asserts this"
    - "The toolchain-product assert + build-artifact assert are copied from phase6.yml's tier-9k leg (the proven 02-12 teeth) — they are what make 'green' mean 'built'"
    - "Both SHA pins are the already-proven in-repo pins (phase6.yml header + r9-shell.yml) — zero NEW external references, so no pin needs API resolution at execution time"
---

<objective>
Replace the legacy harvey.yml CI harness wholesale with the repo's own nix pipeline, so the base "clean checkout builds" gate is green-capable on ubuntu-latest moving forward (backlog 999.1; the master red streak since 2026-11-04T12:01Z is pre-existing/unrelated to Phase 6).

**Legacy-leg decision (deliberate, per the planning context): FULL REPLACEMENT — no legacy leg is kept.** Rationale: (1) the red streak's fragility IS the harness — EOL-Ubuntu package sources plus a PPA, a vanity-URL go-get fetch, an old-branch Go source build for the plan9 target, and an interactive-automation-driven 9legacy ISO boot under qemu x86 running an in-guest build; "repairing" it means re-owning exactly that fragility; (2) the signal it provided ("does the repo build on push/PR") is fully covered by the nix pipeline, which is the repo's actual supported build path and is already proven on ubuntu-latest by phase6.yml's wave0-host / tier-9k / x86_64-poolpanic legs; (3) a retained-but-red legacy leg trains red-blindness (the 06-10 tier-donor retirement lesson); (4) the in-guest ISO build exercised the 9legacy toolchain more than this repo's current source — the signal was stale even when green. The build/ harness scripts (build.bash, the guest-driving command script, the .expect files) stay in-tree untouched — they remain usable locally; only CI stops invoking them.

Purpose: harvey.yml is the original on-push build gate; after this plan it encodes the same self-provisioned nix shape phase6.yml proves on every push: install nix -> setup -> toolchain (products asserted) -> build (artifacts asserted).
Output: the rewritten workflow (locally validated; hosted run PENDING FIRST PUSH), ticked bookkeeping, and the SUMMARY.

Tracer note: quick-mode single-artifact change copying an already-proven CI shape — Task 1 is itself the whole vertical (author the artifact; its <verify> is the end-to-end structural check). No separate tracer task adds information here.
</objective>

<execution_context>
@/Users/neilskinner/.config/opencode/gsd-core/workflows/execute-plan.md
@/Users/neilskinner/.config/opencode/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.github/workflows/harvey.yml
@.github/workflows/phase6.yml
@.github/workflows/r9-shell.yml
@flake.nix
</context>

<tasks>

<task type="auto">
  <name>Task 1: Rewrite harvey.yml onto the nix pipeline (full replacement)</name>
  <files>.github/workflows/harvey.yml</files>
  <read_first>.github/workflows/phase6.yml (the tier-9k job's "host bin + toolchain provisioning" and "build amd64 userspace + kernels" steps are the verbatim copy source), .github/workflows/harvey.yml (the file being replaced), .github/workflows/r9-shell.yml + flake.nix (pin + app references)</read_first>
  <precondition>python3 with PyYAML is importable on the execution host (verified at planning time 2026-11-05); flake.nix defines the setup/toolchain/build apps (lines 132-134 at planning time)</precondition>
  <action>
Replace the ENTIRE content of .github/workflows/harvey.yml (currently 33 lines) with the new workflow specified below. Do not repair or retain any legacy step — the retired pieces are: the distribution-package provisioning step (apt packages plus a toolchain PPA), the go-get fetch of the 9P file server, the old-branch Go source clone+build for the plan9/amd64 target, the tag-pinned checkout action reference, the job-level `defaults.run.working-directory: build` key, and the final step that background-launches the 9P server and runs the guest-driving script under build/ (the i386 ISO boot + in-guest build). Name these descriptively in the header comment only; never reintroduce them as runnable steps.

New file structure (exact):

1. Header comment block (full-line `#` comments, ~15-20 lines; executor phrases the prose, these topics are mandatory): this is the base clean-checkout build gate; rewritten 2026-11-05 (backlog 999.1) replacing the retired legacy harness (describe the retired pieces per the list above); WHY replacement not repair (the harness itself was the fragility — red streak on master since 2026-11-04T12:01Z); the replacement shape (the repo's own nix pipeline, the self-provisioned shape phase6.yml's linux legs prove on every push); pointers to phase6.yml for the deep gates and r9-shell.yml for the aarch64 r9 smoke; the supply-chain block listing BOTH pins with provenance — actions/checkout v4 = 11d5960a326750d5838078e36cf38b85af677262 (API-resolved 2026-11-05, the phase6.yml pin) and DeterminateSystems/nix-installer-action v23 = 3138316df39ed29be04236d7ffc686fa525866aa (the 05-13 pin, already riding r9-shell.yml + phase6.yml); permissions + concurrency rationale.

2. `name: harvey` (unchanged — checks/badge identity).

3. `on: [push, pull_request]` (unchanged).

4. `permissions:` `contents: read`.

5. `concurrency:` group `harvey-${{ github.ref }}`, `cancel-in-progress: true` (the T-06-18 pattern).

6. One job, id `build` (keep the legacy job id — stable checks context), `runs-on: ubuntu-latest`, `timeout-minutes: 60`, four steps:
   a. `- uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262` — no `with:` block (the default shallow checkout suffices; no archive-mode gates run here).
   b. `- name: install nix` / `uses: DeterminateSystems/nix-installer-action@3138316df39ed29be04236d7ffc686fa525866aa`.
   c. `- name: host bin + toolchain provisioning` — copy the run block VERBATIM from phase6.yml's tier-9k job step of the same name (phase6.yml lines ~255-271): `nix run .#setup`, `nix run .#toolchain`, then the hostbin assert loop (`_a=$(uname -m); [ "$_a" = "arm64" ] && _a=aarch64`, `hb=".build/hostbin-linux-$_a/bin"`, loop `for t in pcc cpp ar 6c 6l 8c 8l` with TOOLCHAIN-MISSING per missing product, hostbin listing + exit 1 on any miss, final `TOOLCHAIN-PRODUCTS-OK ($hb)` echo). Keep a short 2-3 line comment: clean-runner provisioning + toolchain.sh's trailing echo masks sub-build failures (the 02-12 exit-0 class), so products are asserted.
   d. `- name: build amd64 userspace + kernels` — derive from phase6.yml's tier-9k job step of the same name (phase6.yml lines ~272-307) with EXACTLY three deltas: (i) the tee log path is `.build/harvey-build.log` (still under `.build/`, created with `mkdir -p .build` first); (ii) DROP the two ipconfig-specific digest blocks (the "ip/ipconfig lines in the build log" grep echo and the `ls amd64/bin/ip/` line) — that failure mode was tier-9k bring-up specific; KEEP the generic digest (bind-error context grep, missing-make-targets grep, `ls amd64/bin/`, first-failure line-number context via sed, last-30-lines tail) and the `BUILD-SWALLOWED-FAIL` message naming the missing artifacts + the 02-12 class; (iii) append a final success line `echo "BUILD-ARTIFACTS-OK (amd64/9k10cpu.elf amd64/bin/rc amd64/bin/awk)"` after the assert block. Keep a shortened 02-12 comment (the break-glass tail reports exit 0 even on failure, so artifacts are asserted). The artifact set is exactly: amd64/9k10cpu.elf, amd64/bin/rc, amd64/bin/awk.

7. Do NOT add: an upload-artifact step (the failure digest prints to the job log/stderr; the deep-evidence legs live in phase6.yml), a fetch-depth override, any third action, any apt/PPA/go-get provisioning, or a darwin leg (CI-green means linux only — the r9-shell.yml/phase6.yml precedent).

8. Do NOT install actionlint or any other validation tool (supply-chain: zero new installs — T-SLX-SC); PyYAML is the validator.

After writing the file, run the verify gate. If pyyaml's safe_load surfaces a YAML 1.1 quirk (top-level `on:` parses as boolean True), the assert already handles it — do not "fix" the YAML to satisfy a miswritten assert.
  </action>
  <verify>
    <automated>cd repo root; then:
python3 - <<'PYEOF'
import yaml
d = yaml.safe_load(open('.github/workflows/harvey.yml'))
ev = d.get(True, d.get('on'))
assert ev == ['push', 'pull_request'], ev
assert d['name'] == 'harvey'
assert d['permissions'] == {'contents': 'read'}, d.get('permissions')
c = d['concurrency']
assert c['group'] == 'harvey-${{ github.ref }}' and c['cancel-in-progress'] is True, c
jobs = d['jobs']
assert set(jobs) == {'build'}, set(jobs)
b = jobs['build']
assert 'defaults' not in b and 'defaults' not in d
assert b['runs-on'] == 'ubuntu-latest' and b['timeout-minutes'] == 60
steps = b['steps']
uses = [s['uses'] for s in steps if 'uses' in s]
assert len(uses) == 2, uses
for u in uses:
    ref = u.rsplit('@', 1)[1]
    assert len(ref) == 40 and all(ch in '0123456789abcdef' for ch in ref), u
assert uses[0] == 'actions/checkout@11d5960a326750d5838078e36cf38b85af677262'
assert uses[1] == 'DeterminateSystems/nix-installer-action@3138316df39ed29be04236d7ffc686fa525866aa'
runs = '\n'.join(s.get('run', '') for s in steps)
for app in ('.#setup', '.#toolchain', '.#build'):
    assert 'nix run ' + app in runs, app
for f in ('amd64/9k10cpu.elf', 'amd64/bin/rc', 'amd64/bin/awk'):
    assert f in runs, f
assert 'TOOLCHAIN-PRODUCTS-OK' in runs and 'BUILD-SWALLOWED-FAIL' in runs and 'BUILD-ARTIFACTS-OK' in runs
print('HARVEY-YML-STRUCTURE-OK jobs={build} pins=2/2 apps=3 artifacts=3')
PYEOF
then the residue / flake-app / neighbor gates:
wf=.github/workflows/harvey.yml
if grep -vE '^[[:space:]]*#' "$wf" | grep -qE 'apt-get|add-apt-repository|qemu-system-i386|go1\.16|cmd/ufs|checkout@v[0-9]|command\.bash'; then echo "LEGACY-RESIDUE-FAIL: a retired-harness token survives in a non-comment line" >&2; grep -vE '^[[:space:]]*#' "$wf" | grep -nE 'apt-get|add-apt-repository|qemu-system-i386|go1\.16|cmd/ufs|checkout@v[0-9]|command\.bash' >&2; exit 1; fi
echo "LEGACY-RESIDUE-OK"
for app in setup toolchain build; do grep -qE "^[[:space:]]+${app} = mkApp" flake.nix || { echo "FLAKE-APP-MISSING: $app" >&2; exit 1; }; done
echo "FLAKE-APPS-OK (setup toolchain build)"
git diff --quiet HEAD -- .github/workflows/r9-shell.yml .github/workflows/phase6.yml || { echo "NEIGHBOR-DRIFT-FAIL" >&2; exit 1; }
test -z "$(git status --porcelain -- .github/workflows/r9-shell.yml .github/workflows/phase6.yml)" || { echo "NEIGHBOR-DIRTY-FAIL" >&2; exit 1; }
echo "NEIGHBORS-OK"</automated>
  </verify>
  <done>harvey.yml parses; exactly two `uses:` references, both the mandated 40-char SHA pins; the nix setup/toolchain/build steps present with toolchain-product and build-artifact asserts; zero legacy-harness tokens in non-comment lines (comment-filtered grep); r9-shell.yml and phase6.yml byte-untouched against HEAD.</done>
</task>

<task type="auto">
  <name>Task 2: Bookkeeping (ROADMAP + PROJECT ticks), SUMMARY with PENDING FIRST PUSH, commit</name>
  <files>.planning/ROADMAP.md, .planning/PROJECT.md, .planning/quick/261105-slx-fix-the-legacy-harvey-yml-workflow-to-bu/261105-slx-SUMMARY.md</files>
  <action>
1. Scoped Edit (NEVER whole-file Write) on .planning/ROADMAP.md: in the "## Backlog" section, the line `- [ ] Test 1: Fix the legacy harvey.yml workflow to build via nix so it is green moving forward (pre-dates this work; red streak on master recorded since 2026-11-04T12:01Z — pre-existing/unrelated to Phase 6) (deferred 2026-11-05)` becomes `- [x]` with the short suffix appended: ` — workflow rewritten onto the nix pipeline 2026-11-05; hosted green run PENDING FIRST PUSH (261105-slx)`. Touch nothing else in the file.
2. Scoped Edit on .planning/PROJECT.md: the carried-over line `- [ ] Fix the legacy harvey.yml workflow to build via nix (pre-dates the nix scaffolding; red streak on master since 2026-11-04T12:01Z) — Phase 6 UAT deferred follow-up, backlog 999.1` becomes `- [x]` with the same short suffix appended. Touch nothing else.
3. Write .planning/quick/261105-slx-fix-the-legacy-harvey-yml-workflow-to-bu/261105-slx-SUMMARY.md per the summary template, following the 261104-cms quick precedent's shape: frontmatter (phase: 261105-slx, plan: 01, subsystem: infra, tags: [ci, github-actions, nix, supply-chain, sha-pins, backlog-999-1], requires/provides/affects, key-files, key-decisions, requirements-completed: [261105-slx], duration, status: complete). Body must record: the full-replacement decision + its four-part rationale (from this plan's objective); the local validation matrix (the Task-1 gate output lines: HARVEY-YML-STRUCTURE-OK, LEGACY-RESIDUE-OK, FLAKE-APPS-OK, NEIGHBORS-OK); and a coverage entry for the hosted run with human_judgment: true whose rationale states the hosted leg is PENDING FIRST PUSH per the 05-13/06-08/06-10 precedent — the executor session never pushes harvey master; a human pushes and watches `gh run list --workflow harvey`. Nothing fabricated: record only validations actually run.
4. Commit per execute-plan conventions: one commit, scope `ci(261105-slx): rewrite legacy harvey.yml CI onto the nix pipeline`, containing .github/workflows/harvey.yml, .planning/ROADMAP.md, .planning/PROJECT.md, this plan's PLAN.md, and the SUMMARY. Do NOT push.
  </action>
  <verify>
    <automated>grep -q '\[x\] Test 1: Fix the legacy harvey.yml' .planning/ROADMAP.md && grep -q 'PENDING FIRST PUSH (261105-slx)' .planning/ROADMAP.md && grep -q '\[x\] Fix the legacy harvey.yml workflow to build via nix' .planning/PROJECT.md && s=.planning/quick/261105-slx-fix-the-legacy-harvey-yml-workflow-to-bu/261105-slx-SUMMARY.md && test -f "$s" && grep -qi 'PENDING FIRST PUSH' "$s" && echo "BOOKKEEPING-OK"</automated>
  </verify>
  <done>Both checkboxes ticked with the PENDING FIRST PUSH suffix; the SUMMARY exists recording the validation matrix + the pending hosted run; one unpushed commit carries the plan's files.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| GitHub-hosted runner -> repo build | CI executes the repo's own scripts on every push/PR; a green/red signal is only as honest as the job's assertions |
| action registry -> workflow | `uses:` references resolve external action code at run time; tag pins are re-targetable |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-SLX-01 | Tampering | .github/workflows/harvey.yml action pins | high | mitigate | Every action pinned by 40-char SHA — actions/checkout 11d5960a326750d5838078e36cf38b85af677262 (v4, the phase6.yml pin) and nix-installer-action 3138316df39ed29be04236d7ffc686fa525866aa (v23, the 05-13 pin); `permissions: contents: read`; the legacy tag-pinned checkout reference is not repeated; the Task-1 verify gate asserts the exact pin set 2/2 |
| T-SLX-02 | Tampering | CI signal integrity — `.#build`'s break-glass tail exits 0 on failure (the 02-12 class) | medium | mitigate | The build step asserts the artifacts (amd64/9k10cpu.elf, amd64/bin/rc, amd64/bin/awk) and prints the failure digest + exit 1 on absence; toolchain step asserts pcc/cpp/ar/6c/6l/8c/8l products — a red build can never green the job |
| T-SLX-03 | Denial of Service | 60-min build legs amplifying on push storms | low | mitigate | `concurrency: { group: harvey-${{ github.ref }}, cancel-in-progress: true }` (the T-06-18 pattern); single fast job keeps the signal quick |
| T-SLX-SC | Tampering | host/toolchain provisioning | high | mitigate | ZERO package-manager installs remain — the entire legacy apt/PPA/go-get provisioning chain is deleted; provisioning is nix-only from the repo's own locked flake; the only external references are the two SHA-pinned actions, both already proven in-repo (no Package Legitimacy Audit needed: no npm/pip/cargo installs, no NEW actions to resolve) |
</threat_model>

<verification>
Plan-level: Task 1's gate (HARVEY-YML-STRUCTURE-OK + LEGACY-RESIDUE-OK + FLAKE-APPS-OK + NEIGHBORS-OK) and Task 2's BOOKKEEPING-OK both green; exactly one unpushed commit. The hosted GitHub run is explicitly OUT of the executor gate per the 05-13/06-08/06-10 precedent — recorded PENDING FIRST PUSH in the SUMMARY; a human pushes and watches `gh run list --workflow harvey`.
</verification>

<success_criteria>
The legacy harness is fully replaced by the nix pipeline on ubuntu-latest with both supply-chain pins, the concurrency group, and the 02-12-proof artifact asserts; neighbors byte-untouched; backlog 999.1 Test 1 + PROJECT.md ticked with the honesty suffix; SUMMARY records local validation + PENDING FIRST PUSH.
</success_criteria>

<output>
Create `.planning/quick/261105-slx-fix-the-legacy-harvey-yml-workflow-to-bu/261105-slx-SUMMARY.md` when done (Task 2).
</output>
