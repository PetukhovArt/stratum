# WebGL prototype — web-client A/B report

**Date:** 2026-05-23
**Target:** `D:\web-projects\web-client` (Electron + Vite + Vue/TS)
**Stratum:** branch `webgl-prototype`, HEAD `69c95f7`
**Browser:** Chrome via chrome-devtools-mcp (headless, DPR ≈ 1.5)
**Viewport:** 787×786 CSS host, 1181×1179 device pixels

## Dataset shape (the load)

From `/api/snapshot` against web-client at HEAD:

| Metric | Count |
|---|---|
| Modules | **3 031** |
| Containers | **925** |
| Edges | **10 274** |
| Layers | 20 |

## Static measurements (automated, no gesture)

Both renderers loaded the same snapshot. Console clean on both branches.

| Metric | SVG | WebGL prototype | Delta |
|---|---|---|---|
| DOM nodes inside graph surface | **12 171** | **3 938** | **−68 %** |
| Total DOM nodes on page | **41 016** | **32 791** | **−20 %** |
| Console errors | 0 | 0 | — |
| Initial render | succeeds | succeeds | — |
| Fit-to-bounds | applies (zoom 13 %) | applies (scale 0.15) | matched |

The 3 938 nodes on the WebGL branch are entirely the HTML label overlay (3 936 module labels + the host + overlay divs). Geometry — lanes, containers, modules, edges — is GPU-batched inside a single `<canvas>`. SVG renders **everything** as DOM, so geometry + labels share one tree.

That delta is the answer to "why does SVG feel sluggish on web-client": every pan moves 12 K DOM nodes through layout/paint; every zoom recomputes the CTM for each one. WebGL pan/zoom touches only the canvas matrix.

## Why no p50/p95 frame-time numbers yet

The automated A/B run cannot dispatch a representative wheel-zoom + drag-pan session — Chrome DevTools Protocol's synthetic input doesn't trigger the same render path as a real trackpad/mouse. For the frame-time deltas the design spec calls out (§7), a manual DevTools Performance trace by the user is the next step.

## Manual trace recipe (for the user)

Server is currently running: `./target/release/stratum-lint.exe visualize D:/web-projects/web-client --port 18080`. If it's been stopped, restart it.

1. Open Chrome → `http://localhost:18080/?renderer=svg`. Wait for fit-to-bounds (zoom 13 % auto-applies).
2. DevTools → Performance tab → ⚙ → Disable "Hardware concurrency override". CPU throttle: **None**. Network: irrelevant for this measurement.
3. Click record. For 5 seconds: continuous wheel-zoom in/out + drag-pan around. Stop.
4. Note **frame-time p50 / p95** (visible in the FPS chart). Save profile as `target/traces/webgl-prototype-svg.json`.
5. Toggle the header button `◇ SVG` → it switches to `🌐 WebGL`. URL becomes `?renderer=webgl`. The snapshot stays cached so no re-lint.
6. Repeat the 5-second session. Save as `target/traces/webgl-prototype-webgl.json`.
7. Fill the table below.

## Decision threshold (per design §7)

- `WebGL_p95 ≥ SVG_p95` → **FAIL**. Roll back per Appendix A of the plan.
- `SVG_p95 × 0.7 ≤ WebGL_p95 < SVG_p95` → **AMBIGUOUS**. Decide whether <30 % win justifies the new surface.
- `WebGL_p95 < SVG_p95 × 0.7` → **PASS**. Promote: add text atlas, drop SVG path, retire ADR-0001.

## Results table (user to fill in)

| Metric | SVG | WebGL | WebGL / SVG | Verdict |
|---|---|---|---|---|
| Frame time p50 (ms) | … | … | … | |
| Frame time p95 (ms) | … | … | … | ← gate |
| First paint (ms) | … | … | … | |
| JS heap delta (MB) | … | … | … | |
| Cold render (ms) | … | … | … | |

Tick exactly one once filled:

- [ ] **FAIL**
- [ ] **AMBIGUOUS**
- [ ] **PASS**

## Known constraints in the WebGL prototype (intentional, per design §8)

- **No edge hover** — edges in WebGL would need line-distance hit-test (out of scope; modules-only hit-test for the prototype).
- **No text on canvas** — module labels are HTML overlay synced to viewport matrix per frame. If we promote, swap to MSDF/SDF atlas.
- **Minimap stays SVG** — small surface, no perf pressure.
- **Bundle delta:** Pixi v8 + pixi-viewport add ~280 KB gzipped (split into WebGL/WebGPU/Renderer chunks, lazy-loaded only when WebGL active).

## Artefacts

- `target/screenshots/webgl-prototype-svg-webclient.png` — SVG renderer, web-client
- `target/screenshots/webgl-prototype-webgl-webclient.png` — WebGL renderer, web-client
- `target/screenshots/webgl-prototype-{svg,webgl}-tinyts*.png` — tiny-ts smoke
- Plan: `docs/superpowers/plans/2026-05-23-webgl-renderer-prototype.md`
- Spec: `docs/superpowers/specs/2026-05-23-webgl-renderer-prototype-design.md`
- Branch: `webgl-prototype` (12 commits on top of `main` @ `c1739b7`)
