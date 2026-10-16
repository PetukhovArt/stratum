# Eval fixtures

Mini projects that pin what each rule must and must not report. The runner is `crates/stratum-lint/tests/eval.rs`; it runs in `cargo test -p stratum-lint`.

```
eval/fixtures/<rule>/bad/<case>/    a project that breaks <rule>
eval/fixtures/<rule>/good/<case>/   a project that is fine
```

`<rule>` is the slug without `stratum/` (`public-api`, `no-circular-deps`). A case is a whole project: `stratum.config.jsonc`, the sources, and for `bad` an `expected.jsonl`.

## What the runner checks

It lints each case with `stratum-lint <case> --format jsonl --no-baseline` and compares:

- `bad`: every line of `expected.jsonl` must appear in the output. Output may hold more lines (superset), but an empty `expected.jsonl` fails. An expected line lists only the contract fields it pins, usually `rule`, `severity`, `file`, `line`; every field it names must match, so message and fix wording can change without touching the fixture.
- `good`: the output must be empty. Turn off, in the case's config, any rule that is not under test and fires on it.

Rules at `error` in `stratum:fsd` need a `bad` case for each way they fire: the target is 100% recall on them.

## Adding a case by hand

1. Create the folder and write the smallest project that shows the behaviour: a config, 2 to 6 source files, relative imports (no tsconfig needed).
2. Run `stratum-lint eval/fixtures/<rule>/bad/<case> --format jsonl --no-baseline`.
3. Copy the lines you meant to pin into `expected.jsonl`, dropping `v`, `tool`, `message` and `fix`. If a line you did not mean appears, the case is not minimal; fix the case, not the expectation.
4. Add the matching `good` case: the same project with the fix applied.

## Mining real mistakes from private-project

private-project is proprietary and read-only here. Never write inside it, never commit its source or names.

1. From a scratch directory (`%TEMP%`), list commits that touched `src`: `git -C <private-project> log --format=%H -n 500 -- src`.
2. For each commit `c`: `stratum-lint <private-project> --config docs/examples/private-project.stratum.config.jsonc diff --base c^ > %TEMP%/eval/c.md`. `diff` reads the base from git objects only. Outputs stay in `%TEMP%`.
3. Imports that broke a rule at `c^` and are gone in the working tree are candidates: a real mistake someone made and later removed. `eslint-boundaries-suppressions.json` in private-project is a second source of the same kind.
4. For each candidate, read the commit with `git -C <private-project> show c`, and write the case from the shape of the mistake, not from the code.

### Minimize and rename into a neutral domain

This repository is public and MIT; the source it was mined from is not. Before anything is committed:

- Cut the project to the fewest files that still trigger the rule.
- Rename everything into a neutral domain (billing, invoices, catalog, notifications): folders, files, identifiers, strings. No product name, customer name, route, endpoint or comment survives.
- Keep the structure that matters (layers, depth, direction of imports), nothing else.
- Check with `git grep` for any word of the source that could have leaked.

## Not here yet

`--files F` equivalence with a filtered full run, and warm versus cold results after edits, belong to the budget and equivalence slice; they will reuse these fixtures.
