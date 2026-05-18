# private-project Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run Stratum end-to-end against the real-world project at `<private-repo>` (Electron + Vite + TS/Vue), fix every misfire, crash, or perf regression that surfaces, and add the project as a recurring integration target.

**Architecture:** Two-repo flow. (a) In *stratum* repo, fix Stratum bugs that the real project surfaces. (b) If private-project itself needs changes (added `@stratum-stage` annotations, refactoring to fit the rules), do those in a **separate git worktree** of `<private-repo>` so the user's primary working tree stays clean. We don't commit private-project into the stratum repo as a fixture (too big, license unclear); instead we add a CI job that runs Stratum against it when an env var points at a local checkout, plus a small "synthetic-from-real" reduced fixture under `tests/fixtures/`.

**Tech Stack:** Bash, PowerShell (the worktree commands), `stratum-lint`, `stratum-lsp`, the Phase-11 visualizer (depends on `2026-05-19-pixijs-rendering.md` being merged first).

**Depends on:** The rendering plan should be merged first. Tasks 6-8 here use the visualizer. Tasks 1-5 work without it.

---

### Task 1: Smoke-run `stratum-lint` against private-project; capture findings

**Files:**
- Create: `docs/integration/2026-05-19-private-project-smoke.md` (the report — kept in repo so future runs compare against it)

This task is discovery. The goal is one artifact: a checked-in report of "what broke, what fired, how long it took" on the real codebase.

- [ ] **Step 1: Build a release binary of stratum-lint**

```bash
cargo build --release -p stratum-lint
```

Expected: `target/release/stratum-lint.exe` exists on Windows / `target/release/stratum-lint` elsewhere.

- [ ] **Step 2: Run it against private-project with JSON reporter**

```bash
target/release/stratum-lint.exe lint <private-repo> --reporter json > private-project-lint.json 2>private-project-lint.stderr.log
echo "exit=$?"
```

Expected: exit code 0 or 1 (1 = violations found). If exit code > 1, that's a crash — capture stderr verbatim in the report.

- [ ] **Step 3: Time the run**

```bash
time target/release/stratum-lint.exe lint <private-repo> --reporter compact > /dev/null
```

Expected: under 5 seconds for first-time (cold) lint on a few-thousand-file Electron project. Record actual time.

- [ ] **Step 4: Tally findings**

```bash
jq '[.diagnostics[] | .rule] | group_by(.) | map({rule: .[0], count: length})' private-project-lint.json
```

Expected: a table of `rule → count`. Save the output verbatim in the report.

- [ ] **Step 5: Write the smoke report**

`docs/integration/2026-05-19-private-project-smoke.md`:
```markdown
# private-project smoke report — 2026-05-19

**Target:** `<private-repo>` (Electron + Vite + TS/Vue)
**Stratum version:** [from `target/release/stratum-lint --version`]
**Cold lint time:** [seconds]
**Exit code:** [0 / 1 / >1]

## Findings by rule

[paste jq output here as a table]

## Crashes / panics

[paste stderr verbatim, or "none"]

## False-positive candidates

[list rule:file:line entries that look wrong on inspection — to be triaged in Task 2]

## Performance notes

- File count: [from `find <private-repo>/src -type f -name '*.ts' -o -name '*.vue' | wc -l`]
- Warm lint time (second run): [seconds]
- Memory peak (rough, from Task Manager): [MB]
```

- [ ] **Step 6: Commit**

```bash
git add docs/integration/2026-05-19-private-project-smoke.md
git commit -m "docs(integration): private-project smoke report 2026-05-19"
```

---

### Task 2: Triage findings — file one issue per category

**Files:**
- Modify: `docs/integration/2026-05-19-private-project-smoke.md` (add triage outcomes)

For each problematic finding from Task 1:

- [ ] **Step 1: Categorize each finding**

For every rule that fired, label it as one of:
- `true-positive` — Stratum is right; private-project violates the rule
- `false-positive-rule-bug` — Stratum's rule logic is wrong
- `false-positive-tuning` — rule is correct in spirit, needs config tuning
- `crash` — Stratum panicked

Append a triage section to `docs/integration/2026-05-19-private-project-smoke.md`:
```markdown
## Triage

| Rule | Count | Category | Action |
|------|-------|----------|--------|
| no-cross-layer-import | 12 | true-positive | needs `@stratum-stage` annotations in private-project |
| forbidden-shared-mutation | 3 | false-positive-rule-bug | issue #N: rule doesn't handle Vue ref unwrapping |
| ... | ... | ... | ... |
```

Open a GitHub issue per `false-positive-rule-bug` and per `crash`. Reference the issue number in the table.

- [ ] **Step 2: Commit triage**

```bash
git add docs/integration/2026-05-19-private-project-smoke.md
git commit -m "docs(integration): private-project triage outcomes"
```

---

### Task 3: Fix crashes (one task per crash from triage)

**Files:** Varies per crash — typically a single rule file under `crates/stratum-rules/src/rules/`.

**This task is a template.** Repeat per crash filed in Task 2.

- [ ] **Step 1: Reproduce in a focused test**

Create a minimal fixture file under `tests/fixtures/private-project-reproductions/<crash-id>/` that triggers the crash. Add a test in the relevant rule's test file:
```rust
#[test]
fn does_not_panic_on_private_project_reproduction_<crash_id>() {
    let result = lint_fixture("private-project-reproductions/<crash_id>");
    assert!(result.is_ok(), "rule must not panic: {:?}", result);
}
```

- [ ] **Step 2: Run, verify it panics**

```bash
cargo test -p stratum-rules does_not_panic_on_private_project_reproduction_<crash_id>
```

Expected: FAIL with the panic message.

- [ ] **Step 3: Fix the rule**

Edit the offending rule. Common patterns: missing `Option::unwrap_or` on AST node lookups, OXC type assumption that doesn't hold for Vue SFC scripts.

- [ ] **Step 4: Run, verify it passes**

```bash
cargo test -p stratum-rules does_not_panic_on_private_project_reproduction_<crash_id>
cargo test --workspace  # nothing else regressed
```

Expected: PASS, no regressions.

- [ ] **Step 5: Commit**

```bash
git add tests/fixtures/private-project-reproductions/<crash_id> crates/stratum-rules/src/...
git commit -m "fix(rules): <rule>: handle <case> without panicking"
```

---

### Task 4: Fix false-positive rule bugs (one task per issue from triage)

**Files:** Varies — typically `crates/stratum-rules/src/rules/<rule_name>.rs` plus its test file.

**Template.** Repeat per `false-positive-rule-bug` issue from triage.

- [ ] **Step 1: Write a failing test from a minimized private-project reproduction**

Add a fixture under `tests/fixtures/private-project-reproductions/<issue-id>/` that contains *only* the structure that misfires. Add a snapshot test:
```rust
#[test]
fn <rule>_does_not_misfire_on_<scenario>() {
    let diagnostics = lint_fixture("private-project-reproductions/<issue-id>");
    let from_rule: Vec<_> = diagnostics.iter().filter(|d| d.rule == "<rule>").collect();
    assert!(from_rule.is_empty(), "rule should not fire: {:?}", from_rule);
}
```

- [ ] **Step 2: Run, verify it fails**

```bash
cargo test -p stratum-rules <rule>_does_not_misfire_on_<scenario>
```

Expected: FAIL — rule still misfires.

- [ ] **Step 3: Fix the rule's logic**

- [ ] **Step 4: Run, verify it passes; also run the rule's existing tests to confirm no regressions**

```bash
cargo test -p stratum-rules <rule>
```

Expected: ALL PASS (including the original true-positive tests).

- [ ] **Step 5: Commit**

```bash
git add tests/fixtures/private-project-reproductions/<issue-id> crates/stratum-rules/...
git commit -m "fix(rules): <rule>: stop misfiring on <scenario>"
```

---

### Task 5: Tune rules that need config-level fixes

**Files:**
- Modify: `crates/stratum-rules/src/<rule>.rs` (default options)
- Modify: `tests/fixtures/private-project-reproductions/<scenario>/stratum.toml` (per-fixture override demonstrating the tuning)
- Modify: `docs/language/glossary.md` if a rule's default options change semantically

**Template.** Repeat per `false-positive-tuning` from triage.

- [ ] **Step 1: Document the current default**

Read the rule file. Note its current default options (severity, thresholds, allowlists).

- [ ] **Step 2: Propose new default in the rule's docstring**

Decide: change the global default, or just expose an option the user can tune in `stratum.toml`?
- Change global default: if the current default is wrong for a typical project.
- Expose option only: if private-project is an unusual case.

- [ ] **Step 3: Update the rule + its docs + its tests**

- [ ] **Step 4: Re-run private-project smoke**

```bash
target/release/stratum-lint.exe lint <private-repo> --reporter compact 2>&1 | tee /tmp/private-project-after-tune.log
```

Expected: the tuned rule no longer fires (or fires only on legitimate violations).

- [ ] **Step 5: Commit**

```bash
git add crates/stratum-rules tests/fixtures/private-project-reproductions docs/language/glossary.md
git commit -m "tune(rules): <rule>: <what changed and why>"
```

---

### Task 6: Add `@stratum-stage` annotations to private-project (in a worktree)

**Files:** None in the stratum repo. Changes are made in a *separate worktree* of `<private-repo>`.

For `true-positive` findings that private-project should fix to conform to its declared architecture, do not edit the user's primary working tree.

- [ ] **Step 1: Confirm private-project's current branch and clean state**

```bash
cd <private-repo>
git status
git rev-parse --abbrev-ref HEAD
```

Expected: a known branch with no uncommitted changes. If there are uncommitted changes, STOP and ask the user how to proceed.

- [ ] **Step 2: Create a worktree for the Stratum work**

```bash
cd <private-repo>
git worktree add ../private-project-stratum-stages stratum-stages-2026-05-19
```

Expected: `<private-repo>` exists, on new branch `stratum-stages-2026-05-19`.

- [ ] **Step 3: Add `@stratum-stage` annotations**

For each true-positive cross-layer-import violation, add a `// @stratum-stage` comment header to the file(s) per the user's architectural intent. The Stratum glossary (`docs/language/glossary.md` in stratum repo) is the canonical reference for annotation syntax.

Run stratum-lint after each batch:
```bash
cd <private-repo>
"D:/projects/stratum/target/release/stratum-lint.exe" lint . --reporter compact
```

Continue until violations drop to zero or only intended ones remain.

- [ ] **Step 4: Commit in the worktree**

```bash
cd <private-repo>
git add -A
git commit -m "chore: add @stratum-stage annotations per Stratum lint"
```

- [ ] **Step 5: Report results in the stratum repo**

Append to `docs/integration/2026-05-19-private-project-smoke.md`:
```markdown
## Worktree changes proposed for private-project

Branch: `stratum-stages-2026-05-19` in `<private-repo>`.
Files annotated: [count]
Violations remaining after annotations: [count, with explanation]

The user owns the decision to merge or discard this branch.
```

- [ ] **Step 6: Commit the report update**

```bash
cd D:/projects/stratum
git add docs/integration/2026-05-19-private-project-smoke.md
git commit -m "docs(integration): record private-project worktree proposal"
```

---

### Task 7: Validate the visualizer on private-project

**Prerequisite:** Plan `2026-05-19-pixijs-rendering.md` must be merged.

**Files:**
- Modify: `docs/integration/2026-05-19-private-project-smoke.md` (append visualizer findings)

- [ ] **Step 1: Run the visualizer against private-project**

```bash
target/release/stratum-lint.exe visualize <private-repo>
```

Expected: HTTP server starts, browser opens. Snapshot loads, renders.

- [ ] **Step 2: Inspect — record readability + perf**

In the browser:
1. Note total node count (visible in dev console or page title once we add a counter — for now, count from the snapshot JSON: `curl http://localhost:PORT/snapshot.json | jq '.nodes | length'`).
2. Open DevTools → Performance. Reload the page. Record:
   - Layout time (from a `console.time('layout') / console.timeEnd('layout')` we'll add in this task — see Step 3)
   - Time-to-first-frame
   - FPS during pan/zoom

- [ ] **Step 3: Add timing instrumentation if missing**

Edit `frontend/stratum-visualizer-frontend/src/render/index.ts` to wrap `computeLayout` and `createScene` in `performance.mark` / `performance.measure`, then log to console:
```ts
performance.mark('layout-start');
const positioned = computeLayout(working, { rankdir: 'LR' });
performance.mark('layout-end');
const measure = performance.measure('layout', 'layout-start', 'layout-end');
console.info(`[stratum] layout: ${measure.duration.toFixed(0)}ms (${positioned.nodes.length} nodes, ${positioned.edges.length} edges)`);
```

- [ ] **Step 4: Re-run, record numbers**

Append to the smoke report:
```markdown
## Visualizer on private-project

- Total nodes: [N]
- Layout time: [ms]
- Sliced view triggered: [yes / no]
- FPS during pan/zoom: [N]
- Time-to-first-frame: [ms]
- Subjective readability: [1-5, comments]
```

- [ ] **Step 5: If layout > 3 s or readability < 3, file follow-ups; do not block this plan**

Either:
- Lower `SLICE_THRESHOLD` in `frontend/stratum-visualizer-frontend/src/render/sliced.ts` and re-measure
- File an issue for "improve dagre tuning for large graphs"

- [ ] **Step 6: Commit instrumentation and report update**

```bash
git add frontend/stratum-visualizer-frontend/src/render/index.ts docs/integration/2026-05-19-private-project-smoke.md
git commit -m "feat(visualizer): perf instrumentation + private-project visualizer report"
```

---

### Task 8: Update PRD perf numbers from real measurements

**Files:**
- Modify: `docs/prd.md` (or whatever the PRD file is — find it via `find docs -iname 'prd*'`)

- [ ] **Step 1: Find current PRD perf numbers**

```bash
grep -n -iE 'perf|throughput|layout|seconds|ms|gate' docs/prd.md
```

- [ ] **Step 2: Update with real measurements**

Replace provisional numbers with measurements from Task 7. Keep the original "≤ 3 s" gate as the assertion; add a "measured on private-project: X seconds" note next to it.

- [ ] **Step 3: Commit**

```bash
git add docs/prd.md
git commit -m "docs(prd): update perf numbers with private-project measurements"
```

---

### Task 9: Add private-project as an opt-in CI integration job

**Files:**
- Modify: `.github/workflows/ci.yml` (add a new job, optional via env var)

The job runs only if a `PRIVATE_PROJECT_PATH` env var is set, so contributors without private-project checked out don't fail. Useful for the maintainer's self-hosted runner or local CI invocation.

- [ ] **Step 1: Add the job**

Append to `.github/workflows/ci.yml`:
```yaml
  private-project-integration:
    name: private-project integration
    runs-on: ubuntu-latest
    if: ${{ vars.PRIVATE_PROJECT_PATH != '' }}
    steps:
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: build stratum-lint
        run: cargo build --release -p stratum-lint
      - name: run on private-project
        run: |
          target/release/stratum-lint lint "${{ vars.PRIVATE_PROJECT_PATH }}" --reporter compact
        timeout-minutes: 5
```

Note: GitHub's `vars` context is org-/repo-level variables. The maintainer sets `PRIVATE_PROJECT_PATH` once in repo settings.

- [ ] **Step 2: Validate yaml**

```bash
gh workflow view ci.yml > /dev/null
```

Expected: no error.

- [ ] **Step 3: Push and confirm the job is skipped by default**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: optional private-project integration job (gated on PRIVATE_PROJECT_PATH var)"
git push
gh run list --limit 1
```

Expected: the run shows `private-project-integration` as skipped (until the maintainer sets the var).

---

## Done definition

- `docs/integration/2026-05-19-private-project-smoke.md` exists with smoke + triage + worktree + visualizer sections fully filled in
- All `crash` findings have fixes shipped
- All `false-positive-rule-bug` findings have fixes shipped or have GitHub issues open with reduced reproductions in `tests/fixtures/private-project-reproductions/`
- private-project `@stratum-stage` annotations live on branch `stratum-stages-2026-05-19` in a worktree — user owns the merge decision
- Visualizer renders private-project end-to-end; perf numbers recorded
- PRD has real numbers
- Optional CI job exists, gated on `PRIVATE_PROJECT_PATH`

## Out of scope (explicitly)

- Auto-fix for any violation. Stratum is a linter, not a refactorer.
- Committing private-project into the stratum repo. Too big, license unclear, owned by the user.
- Productionizing the integration job (notifications, alerting). It's a smoke check.
