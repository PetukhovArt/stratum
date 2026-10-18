# Calibration

Defaults of the metric rules come from sweeps on one large real project (~4K modules, 6 layers, the `stratum:fsd` preset), counted from `--format json`. The rule is: no metric rule is ever `error`, and a default is the knee of its curve, not the point that silences the rule.

## Sweeps

Hits per threshold; the shipped default is marked.

| Rule | Option | Hits |
|---|---|---|
| `stratum/miller-limit` | `max_children` | 5: 175, 7: 130, 10: 94, 15: 67, **20: 53**, 30: 34 |
| `stratum/promotion-pressure` | `threshold` | 3: 1, **5: 0** |
| `stratum/deep-module` | `max_exports` | 8: 385, 12: 201, 16: 113, **20: 72** |
| `stratum/depth-ratio` | `min_ratio` | 0.5: 9, **1.0: 36**, 2.0: 68, 3.0: 101 |
| `stratum/container-breadth` | `max_kinds` | 4: 61, **6: 35**, 8: 20 |
| `stratum/slice-fan-in` | `max_fan_in` | 3: 5, **5: 3**, 8: 2 |

Distributions behind them: miller fan-in p95 5, p99 27, max 467; module exports p95 13, p99 27, max 160.

## Changes from the earlier defaults

| Option | Was | Now |
|---|---|---|
| `miller-limit.max_children` | 7 | 20 |
| `deep-module.max_exports` | 12 | 20 |
| `depth-ratio.min_ratio` | 3.0 | 1.0 |

`promotion-pressure` (5), `container-breadth.max_kinds` (6) and `slice-fan-in.max_fan_in` (5) keep their values.

## Severities

`miller-limit`, `promotion-pressure`, `deep-module` and `depth-ratio` report at `info` in the presets. `container-breadth` is `info` in `stratum:recommended` and `warning` in `stratum:fsd`; `slice-fan-in` is `off` in `stratum:recommended` and `warning` in `stratum:fsd`.

## Budget project

The budgets in [harness.md](harness.md#budgets) are checked on a generated project, so the gate needs no private code: `stratum-budget generate DIR` writes 4,000 modules in six layers (`shared`, `entities`, `features`, `widgets`, `pages`, `app`) under the `stratum:fsd` preset, with slices, entries, segments, `_shared` zones, a few `.vue` files, `paths` in `tsconfig.json`, `project.entry_points`, a few cycles and some imports that break the layer, public-API and sibling rules. It is a git repository, so `--changed` works. `stratum-budget check` builds the release `stratum-lint`, runs the four scenarios 20 times each and fails when a p95 or a peak RSS is over its budget.

The budgets sit 2 to 5 times above the time, and 4 to 8 times above the memory, that a real ~4K-module private project (3,800 modules) measured on Windows in 2026-10, after the graph cache landed:

| Scenario | p95 | Peak RSS |
|---|---|---|
| Full, cold | ~361 ms | 39 MB |
| Full, warm | ~177 ms | 23 MB |
| `--files F`, warm | ~141 ms | 23 MB |
| `--changed`, no changed file | ~120 ms | |

The headroom covers slower CI runners and macOS process start, not growth: a budget that is hit points at a regression. The generated project runs faster than the real one (about 110 ms warm, 215 ms cold on the same machine), because it is simpler to parse; the check is a regression gate, not a forecast.

## Exemplar

The exemplar fixture (`tests/fixtures/exemplar-stratum-app`) has no hits for these rules except `depth-ratio`: 2 at 2.0, 4 at 3.0. Its config pins `min_ratio` to 3.0 so the snapshot does not move with the default.
