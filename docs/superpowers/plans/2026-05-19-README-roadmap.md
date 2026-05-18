# Stratum post-v0.1.0 roadmap — 2026-05-19

Four plans ordered by impact. They are independent enough to ship in different orders, but Plan B's "visualizer on web-client" task formally depends on Plan A being merged.

## Preliminary (do this once before any plan, ~5 minutes)

Bump GitHub Actions to v5 across `.github/workflows/*.yml`:

```bash
# All three workflow files:
sed -i 's|actions/checkout@v4|actions/checkout@v5|g' .github/workflows/*.yml
sed -i 's|actions/setup-node@v4|actions/setup-node@v5|g' .github/workflows/*.yml
sed -i 's|actions/upload-artifact@v4|actions/upload-artifact@v5|g' .github/workflows/*.yml
sed -i 's|actions/download-artifact@v4|actions/download-artifact@v5|g' .github/workflows/*.yml
# And bump Node to current LTS:
sed -i "s|node-version: '20'|node-version: '22'|g" .github/workflows/*.yml
```

Verify the diff is clean, then commit + push + watch CI:

```bash
git diff .github/workflows
git add .github/workflows
git commit -m "chore(ci): bump actions to v5 and Node 22 LTS"
git push
gh run watch --exit-status
```

If anything breaks, fix forward before starting Plan A.

## Plans

| # | Plan | File | Why it ranks here |
|---|------|------|-------------------|
| A | PixiJS rendering | [`2026-05-19-pixijs-rendering.md`](./2026-05-19-pixijs-rendering.md) | Most user-visible. The "MVP without graph" looks unfinished. Real rendering also unblocks Plan B's visualizer validation. |
| B | Web-client integration | [`2026-05-19-web-client-integration.md`](./2026-05-19-web-client-integration.md) | Real-world validation. Surfaces every misfire, crash, and perf hole in the actual product Stratum exists to lint. Confidence multiplier for any future change. |
| C | IDE integrations (Zed + WebStorm) | [`2026-05-19-ide-integrations.md`](./2026-05-19-ide-integrations.md) | Adoption blocker for everyone outside VS Code. Discoverable dev-console errors are the half of "easy to launch" the user explicitly asked for. |
| D | Rhai plugin hot-reload | [`2026-05-19-rhai-hot-reload.md`](./2026-05-19-rhai-hot-reload.md) | Quality-of-life for plugin authors. Small change, well-defined scope. |

## Dependencies between plans

```
Preliminary (Actions v5)
        │
        ▼
   Plan A (rendering) ──┐
        │               │
        ▼               │
   Plan B Task 7 ◄──────┘  (visualizer on web-client)
   (other Plan B tasks have no dep on A)

   Plan C ── independent
   Plan D ── independent
```

If executing in parallel: A and (B-tasks-1-through-6) can run concurrently if you have two engineers; B-task-7 onwards waits for A to merge. C and D can start anytime.

## Suggested execution order (single engineer)

1. **Preliminary** (Actions v5) — done in 5 minutes
2. **Plan A** (rendering) — biggest user-facing change; finish before moving on
3. **Plan B** (web-client integration) — fold the visualizer into the integration check at Task 7
4. **Plan D** (hot-reload) — small, contained, ship between B and C as a palate cleanser
5. **Plan C** (IDE integrations) — last; benefits from Plan B's hardened rules and Plan D's hot-reload story

## What this roadmap does NOT cover

- **Salsa per-rule queries.** Discussed; remains deferred. Revisit only if Plan B surfaces real perf pain on web-client.
- **Marketplace publishing** for VS Code / Zed / JetBrains plugins. Manual install paths from Plan C are sufficient until adoption demands it.
- **Custom Sugiyama (Rust→WASM)** for layout. Plan A picks `@dagrejs/dagre`; the custom path stays a Phase 12+ option per ADR-0001.
- **Auto-fix / quickfix LSP actions.** Bigger feature, separate plan when prioritized.

## Execution instructions

Each plan starts with a header pointing at `superpowers:subagent-driven-development` or `superpowers:executing-plans`. Pick one approach per plan; don't mix mid-plan.
