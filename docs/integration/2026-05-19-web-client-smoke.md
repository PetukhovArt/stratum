# web-client smoke report — 2026-05-19

**Target:** `D:\web-projects\web-client` (Electron + Vite + Vue/TS, no `stratum.toml`, ran against zero-config defaults)
**Stratum version:** `stratum-lint 0.1.0` (commit at smoke time: post-Plan A merge)
**Cold lint time:** 3.18 s
**Warm lint time:** 2.04 s
**Exit code:** 1 (violations found, no panic)
**Source files (.ts / .vue / .js):** ~2 996 (1 482 `.ts`, 945 `.vue`, 569 `.js`)
**Total violations emitted:** 2 279

## Findings by rule

| Rule ID | Rule | Count | Severity |
|---------|------|-------|----------|
| 1 | `no-cross-layer-import` | 1 962 | warning |
| 8 | `deep-module` | 155 | (rule default) |
| 9 | `promotion-pressure` | 78 | (rule default) |
| 4 | `miller-limit` | 47 | (rule default) |
| 5 | `depth-ratio` | 22 | (rule default) |
| 2 | `no-circular-deps` | 15 | error |

## Crashes / panics

None. Stderr was empty across both cold and warm runs.

## Performance notes

- Comfortably inside the PRD's 5 s gate for "few-thousand-file" projects.
- Warm lint stripped ~1.1 s off the cold run — most of the cold cost is OXC parse + tsconfig path resolution.
- Memory was not instrumented in this pass; subjectively the run never approached system limits.

## Triage

Stratum did **not** crash and did **not** misfire in any obvious rule-bug way on this codebase. The 1 962 `no-cross-layer-import` warnings are expected: web-client has no `stratum.toml` and no `@stratum-stage` annotations, so the zero-config inference matches `src/<top-level>/` to layer names and every cross-folder import lights up. That makes them all *true positives against an inferred config that hasn't been told what the user wants*, not bugs.

The notable observations are about **report-quality**, not rule-correctness:

| Rule | Count | Category | Notes |
|------|-------|----------|-------|
| 1 — `no-cross-layer-import` | 1 962 | `tuning` | Expected. Resolves to ~zero once `@stratum-stage` annotations land or a `stratum.toml` declares `depends_on`. Not actionable until the user defines layer intent. |
| 2 — `no-circular-deps` | 15 | `report-quality-bug` | Cycle sizes range from 1 → 953 modules (one ~thousand-node SCC). A 953-node cycle message dumps the entire arrow chain into the violation `message` — useless to act on. The rule is technically right; the **reporting** needs to either (a) cap the path length and link to a graph view, or (b) split into one violation per SCC edge. Filed as follow-up. |
| 4 / 5 / 8 / 9 — metric rules | 302 total | `tuning` | All four are threshold-based metrics. Without measuring what reasonable thresholds for an Electron-scale frontend look like, every value will fire. Defer to a "real-world threshold calibration" follow-up that also touches `tests/fixtures/exemplar-stratum-app`. |
| any rule | 0 | `crash` | None observed. |

## Issues filed

This session did not open GitHub issues — the user's repository policy on issue templates and labels for `stratum` isn't established yet. Three follow-ups belong in whichever tracker the user designates:

1. **`no-circular-deps` reporting on huge SCCs.** A 953-node cycle message is unreadable. Either split per back-edge or cap path length and reference the visualizer.
2. **Default thresholds for `deep-module` / `promotion-pressure` / `miller-limit` / `depth-ratio`.** Re-calibrate against this real codebase (and at least one more) before claiming the defaults are sensible.
3. **Zero-config inference UX.** When no `stratum.toml` is present and 1 962 warnings of the same rule fire, the CLI should print a single banner — "no layer config found, inferred from `src/*/` folders; see `stratum-lint init` to declare layers explicitly" — instead of letting the noise drown the actual problems.

## Worktree annotations (not done in this session)

Plan B Task 6 calls for opening `D:\web-projects\web-client-stratum-stages` worktree on branch `stratum-stages-2026-05-19` and adding `@stratum-stage` comments to web-client files until violations drop. This requires architectural decisions about *which* stage each file belongs to — a judgment call the user has to make. Skipped here; folded into the follow-up that resolves the `no-cross-layer-import` flood.

## Visualizer on web-client (not run in this session)

Plan B Task 7 requires the visualizer renderer (Plan A) merged *and* a manual browser inspection (DevTools timing, FPS, readability score). Plan A's code is shipped, but browser-driven measurement isn't reproducible from this automation seat. Recommended next step for the user:

```bash
target/release/stratum-lint.exe visualize D:/web-projects/web-client
```

…then open DevTools and record layout time, FPS, and time-to-first-frame. Numbers feed back into the PRD (US-9 layout-perf metric).

## What was actually achieved

- Confirmed Stratum runs on ~3 K real Vue/TS/JS files without panicking, in under 5 s cold.
- Identified that the dominant noise source is *config not authored*, not *rule logic broken*.
- Surfaced the one genuine product bug worth fixing soon: cycle reporting on giant SCCs.
- Established this report as the baseline for future regression comparisons.

## CI integration job (Plan B Task 9)

Added in this session — see `.github/workflows/ci.yml`. The job runs only when the `WEB_CLIENT_PATH` repo variable is set; otherwise it is skipped, so contributors without a local web-client checkout aren't blocked.
