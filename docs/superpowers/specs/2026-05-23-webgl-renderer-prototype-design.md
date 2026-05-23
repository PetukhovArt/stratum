# WebGL renderer prototype — design

**Дата:** 2026-05-23
**Статус:** approved (design phase)
**Ветка:** будет создана отдельная при имплементации
**Owner:** PetukhovArt

## 1. Цель

Throwaway-прототип: заменить SVG `<g.strat-graph>` в визуализаторе на WebGL canvas (PixiJS v8), остальной UI (Header, OutlinePanel, DetailsPanel, CommandPalette, Minimap, StatusBar) не трогать. Проверить на полевом тесте — web-client (~3000 модулей), — даёт ли WebGL стабильные 60fps при пан/зум там, где SVG проседает. Решение «продвигать в main или выбрасывать» принимаем по конкретным метрикам (см. §7).

Прототип не претендует на визуальный паритет 1:1 и не требует тестов. Это инструмент решения: WebGL — да или нет.

## 2. Контекст

Визуализатор был переписан 2026-05-19 с Solid+PixiJS+dagre на Solid+SVG с богатым fractal-лейаутом (commit `6edb922`). Память `project-stratum-visualizer-fractal-rewrite` фиксирует решение «escape hatch: если > 5000 узлов → slicing или hybrid SVG/Pixi». Web-client — ~3000 модулей, на грани комфорта; SVG нагружен blur-фильтрами, oklch палитрой, dashed strokes, severity halos. Перформанс-итерации (`8d313a5 perf`, `abce91b perf`) выжали что могли из SVG.

Текущий код: `frontend/stratum-visualizer-frontend/src/components/Graph.tsx` (943 строки), общий объём `src/` — 5657 строк.

## 3. Архитектура

```
<App/>
 ├─ <Header/>                ← добавляется тоггл "SVG | WebGL"
 ├─ renderer === 'webgl'
 │     ? <GraphWebGL/>       ← новый
 │     : <Graph/>            ← существующий, не меняем
 ├─ <OutlinePanel/>
 ├─ <DetailsPanel/>
 ├─ <Minimap/>               ← остаётся SVG (мелкая поверхность)
 └─ <StatusBar/>
```

Решение по renderer хранится в сигнале `renderer: 'svg' | 'webgl'` в `App.tsx`. Источники начального значения, в порядке приоритета:

1. URL-параметр `?renderer=webgl`
2. `localStorage['stratum.renderer']`
3. `'svg'` по умолчанию

При смене в UI обновляются URL (`history.replaceState`) и `localStorage`. Перезагрузка не требуется — переключение между `<Graph/>` и `<GraphWebGL/>` через `<Show when=…/>`, монтаж/демонтаж Pixi Application — в onCleanup.

### Внутреннее устройство `<GraphWebGL/>`

```
<div class="strat-graph-webgl">
 ├─ <canvas/>                ← PixiJS Application
 └─ <div class="labels-overlay">
      └─ <For each={visibleModules}/>
           └─ <div class="module-label">{m.label}</div>
```

Layers внутри Pixi (Z-order):
1. `laneBg` — фоновые полосы lanes
2. `containerBg` — фоны контейнеров (с layer-tinted цветом)
3. `modules` — прямоугольники модулей
4. `edges` — линии рёбер (включая cycle-dashed)

Pan/Zoom через `pixi-viewport@^6` (drop, plugin'ы `drag`, `wheel`, `pinch`).

HTML-overlay для текста модулей синхронизируется одной CSS-transform на корне `div.labels-overlay`: та же матрица camera (translateX/Y, scale), что у `viewport`. Каждый label позиционируется через CSS `transform: translate(absX, absY)` относительно overlay-корня — синхронность достигается тем, что Pixi и overlay получают одну матрицу.

### Hit-testing

Один `<canvas/>` не даёт per-element pointer-events. Решение: bucket-grid spatial index, перестраиваемый при изменении `Scene`. Cell size = max(modWidth, modHeight) среди модулей. `pointermove` → query index → если попали в модуль → emit `onHover`. Throttle 16ms (один query на кадр).

## 4. Файлы

**Новые:**

| Путь | Назначение | Прим. размер |
|---|---|---|
| `src/components/GraphWebGL.tsx` | Solid-компонент, lifecycle Pixi App + overlay | ~250 lines |
| `src/render/webgl/scene.ts` | Построение Pixi-объектов из `Scene` | ~200 lines |
| `src/render/webgl/styles.ts` | oklch → linear-sRGB, толщины/радиусы | ~80 lines |
| `src/render/webgl/hitTest.ts` | Bucket-grid spatial index | ~120 lines |

**Изменяемые:**

| Путь | Что меняется |
|---|---|
| `src/App.tsx` | Сигнал `renderer`, чтение URL/localStorage, `<Show>` на оба варианта |
| `src/components/Header.tsx` | Тоггл "SVG / WebGL" рядом с command palette |
| `package.json` | `pixi.js@^8.x`, `pixi-viewport@^6.x` в `dependencies` |

**Не трогаем:**

- `src/render/layout.ts` — `Scene` shape остаётся 1:1
- `src/render/adapt.ts`, `src/render/design.ts`
- `src/components/Graph.tsx` — SVG-версия живёт как была
- `src/components/Panels.tsx`, `Header.tsx` (кроме одного тоггла), `StatusBar.tsx`
- `src/state.ts`
- `src/styles.css` (кроме добавления нескольких классов для overlay)

## 5. Data flow

Pipeline снапшота → adapter → layout → Scene не меняется. `<GraphWebGL/>` подписывается на тот же `Scene` сигнал, что и `<Graph/>`. Любые изменения filter/tweaks триггерят `rebuild scene` (тот же re-render trigger, что у SVG-ветки).

Соответствие источников данных:

| SVG-источник | WebGL-эквивалент |
|---|---|
| `<g.lanes>` over `scene.lanes` | Pixi `Container` layer 1, rect-по-lane |
| `<g.containers>` over `scene.containers` | Pixi `Container` layer 2, rect-per-container |
| `<g.modules>` over `scene.modulePos` | Pixi `Container` layer 3, rect-per-module |
| `<g.edges>` over `scene.edges` | Pixi `Container` layer 4, line-per-edge |
| `<text class="module-label">` | HTML `<div class="module-label">` в overlay |

## 6. Визуальный паритет — компромиссы

| Сейчас (SVG) | WebGL-прототип | Степень паритета |
|---|---|---|
| `filter: url(#halo)` (blur 8px) | Pre-baked radial-gradient sprite + `tint` + `BlendMode.SCREEN` | ≈90% |
| Dashed strokes (cycle edges) | Полилиния из коротких сегментов + uniform `dashOffset` через ticker | 100% |
| oklch палитра | Конвертим в gamma-corrected linear-sRGB при build scene | визуально идентично |
| Rounded rects | `Graphics.roundRect` нативно в Pixi v8 | 100% |
| Текст модулей (svg `<text>`) | HTML-overlay (без atlas / MSDF) | 100% (тот же шрифт CSS) |
| Severity glow vs hover-glow | Один sprite-pool, разный tint | 90% |

Что **намеренно** теряем (можно вернуть после прототипа, если решим продвигать):
- Per-element CSS hover transition (в WebGL hover мгновенный — приемлемо)
- SVG `<filter>` композиции, если они окажутся за пределами sprite+tint

## 7. Метрики и success criteria

**Один Chrome DevTools Performance trace на web-client snapshot, оба рендера.**

Сценарий замера:
1. Открыть визуализатор на web-client (`stratum-lint visualize D:/web-projects/web-client`)
2. Дождаться first interactive frame
3. Записать Performance trace 5 секунд: wheel-zoom + drag-pan непрерывно
4. Повторить для второго рендера, на той же машине, той же сессии браузера

SVG baseline снимается в той же сессии браузера, что и WebGL — так что абсолютная цифра roller-coastera не важна, важно соотношение. Все четыре метрики сравниваем парой `SVG_value` vs `WebGL_value` из одного и того же замера.

Целевые ориентиры для WebGL (абсолютные):

| Метрика | WebGL таргет |
|---|---|
| Frame time p50 | ≤ 12ms |
| Frame time p95 | ≤ 16ms |
| First paint (layout-end → first frame) | в пределах ±30% от SVG |
| JS heap delta (после full render) | в пределах ±50% от SVG |
| Cold render (init → interactive) | ≤ SVG × 1.5 |

**Решение по прототипу — количественные пороги:**

- `WebGL_p95 ≥ SVG_p95` → прототип провален. Выбрасываем флаг + 4 файла + 2 dep.
- `SVG_p95 × 0.7 ≤ WebGL_p95 < SVG_p95` → выигрыш меньше 30%. Обсуждаем стоимость (HTML-overlay текст, риск pixi-viewport совместимости, новая поверхность поддержки) против выигрыша; решение совместное с пользователем.
- `WebGL_p95 < SVG_p95 × 0.7` → выигрыш ≥30%. Продвигаем в follow-up plan: text в WebGL (atlas), уход SVG-варианта, обновление ADR-0001 и памяти `project-stratum-visualizer-fractal-rewrite`.

## 8. Out of scope

- WebGPU renderer (Pixi v8 поддерживает, но WebGL backend хватит для прототипа)
- Text rendering в canvas (MSDF / atlas builder)
- Minimap — остаётся SVG (поверхность мелкая, оверхеда нет)
- Slicing UI редизайн — `Scene` shape не трогаем
- E2E-тесты на WebGL-ветку (только ручной A/B; SVG e2e продолжают работать)
- ADR-0001 ревизия (откладывается до решения по продвижению)
- Marketplace/distribution-изменения

## 9. Риски и митигации

| Риск | Митигация |
|---|---|
| `pixi-viewport@6` несовместим с `pixi.js@8` | Сверить версионную таблицу перед install; если несовместим — использовать ручной pan/zoom через event listeners + matrix transform (~50 строк) |
| HTML-overlay text labels «дёргаются» относительно canvas-боксов | Использовать одну `DOMMatrix`, обновлять CSS transform overlay-корня и Pixi viewport из общего источника правды (signal `viewportMatrix`) каждый ticker |
| Hover-tooltip «мигает» без throttle | 16ms throttle на pointermove → spatial-index query |
| Pixi context loss (resume from sleep) | `app.renderer.on('contextlost', …)` → перестраиваем Scene из `Scene` сигнала |
| Bundle weight | Pixi v8 ~250KB min, pixi-viewport ~30KB. Cumulative +280KB поверх текущих 87KB gzip. Допускаем для прототипа; если продвигаем — chunk-split по renderer |
| Цвета (oklch → sRGB) расходятся с SVG | Конвертер один раз при build scene; визуальный diff допустим в пределах прототипа |

## 10. Открытые вопросы

Нет blocking. Все приняты в §4–§9 как решения.

## Приложение A — Команды для воспроизведения замера

```bash
# Сборка фронтенда
cd frontend/stratum-visualizer-frontend
npm run build

# Запуск визуализатора на web-client
cargo run -p stratum-lint --release -- visualize D:/web-projects/web-client --port 18080

# В отдельном окне — открыть http://localhost:18080
# - Дождаться layout-end
# - DevTools → Performance → Start recording
# - 5 сек wheel/drag
# - Stop, сохранить trace
# - Toggle SVG <-> WebGL в Header (или URL ?renderer=webgl)
# - Повторить
```
