# Stratum Visualizer — requirements & wishlist

Живой документ. Источник правды — этот файл, а не отдельные ADR / планы. ADR-0001 фиксирует *техническое* решение (layout backend), Plan A — *как делали MVP*; этот файл — *что должно быть и почему*.

## Цель

Визуализатор — это интерактивная карта зависимостей кодовой базы, на которой видно:

1. **Структуру** — слои, контейнеры, модули, рёбра между ними.
2. **Состояние** — где нарушены правила Stratum: какие модули и рёбра ломают архитектуру.
3. **Динамику** (будущее) — как нарушения появляются/исчезают между ревизиями.

Пользователь визуализатора — разработчик команды, который хочет за минуту понять «куда я попал?» в незнакомой части кода, и архитектор, который хочет за минуту понять «насколько мы отъехали от заявленной архитектуры?».

Не цель: заменить редактор, реплицировать UI VS Code, рендерить полную AST, давать quick-fix. Stratum — линтер, не рефакторер (PRD decision #10).

## Архитектурный контракт

- **Источник данных** — `GraphSnapshot v1` (`stratum_graph::snapshot_of`). Версия v1 заморожена; bump = координированный фронт-релиз (см. `RELEASE.md`).
- **Транспорт** — HTTP-сервер встроен в `stratum-lint` (`stratum-lint visualize <root>`), Axum + `rust-embed` для бандла. `/api/snapshot` + `/api/violations`.
- **Рендер** — фронт на SolidJS + PixiJS 8 + `@dagrejs/dagre` + `pixi-viewport`. Frame-free слой `render/` (см. Plan A архитектуру).
- **Конфиг** — `visualize` читает `stratum.config.jsonc` в корне проекта, fallback на zero-config inference. Тот же путь, что у `lint`.
- **Без сервера-снаружи** — всё через локальный binary, нет аккаунтов, телеметрии, облака. Открыто в браузере = всё.

## Перформанс-бюджет (PRD)

| Метрика | Бюджет | Текущее (на 2026-05-19) |
|---|---|---|
| Initial layout time, ≤ 500 нод | < 3 s | ~ms на tiny-ts; на web-client (1948 нод в sliced-режиме) — TBD |
| Bundle (без sourcemaps) | < 1.5 MB | 620 KB |
| Cold lint feeding `visualize` | < 5 s на ~3K файлов | 3.18 s на web-client |
| Sliced threshold | ≤ 500 видимых нод | 500 (ADR-0001) |

«TBD» закрывается следующей сессией с замерами FPS / time-to-first-frame.

## Что есть (v0.1, 2026-05-19)

- [x] Сервер `stratum-lint visualize <root>` — рандомный порт, открывает браузер автоматически (`open` crate).
- [x] `/api/snapshot` отдаёт `GraphSnapshot` (modules + containers + layers + edges).
- [x] `/api/violations` отдаёт `Vec<Violation>` по тому же конфигу, что у `lint`.
- [x] Read of `stratum.config.jsonc` (fallback zero-config).
- [x] Рендер: dagre LR layout, контейнеры (parent), модули, рёбра с arrowhead.
- [x] Пан/зум/wheel/decelerate (`pixi-viewport`).
- [x] Sliced view: >500 нод сворачиваются в layer-aggregate, клик разворачивает один слой целиком.
- [x] Подсветка нарушений: модули `error=красный`, `warning=жёлтый`; рёбра-нарушители (no-cross-layer-import) красные.
- [x] Счётчики в шапке: `N modules · K errors · M warnings`.
- [x] CI: bundle-size budget (без sourcemaps), unit + Playwright smoke.

## Что хочется (приоритизировано)

### P0 — необходимо для повседневного использования

- [ ] **Tooltip на узле**: при ховере — полный путь, слой, контейнер, stage, число входящих/исходящих рёбер, **список нарушений на этом модуле** с текстом каждого.
- [ ] **Tooltip на ребре**: source/target пути, kind (static/di/runtime), сообщение нарушения если есть.
- [ ] **Поиск/фильтр** по пути модуля (Ctrl+P-style): печатаешь подстроку → подсветка совпадений + центрирование на первом.
- [ ] **Outline-панель** слева: дерево слой → контейнер → модуль, цвет элемента отражает наличие нарушений, клик центрирует viewport на узле.
- [ ] **Подсветка агрегата** в sliced view — `shared (847 modules, 14 errors)` — чтобы было видно, какой слой раскрывать в первую очередь.

### P1 — повышает ценность

- [ ] **Фильтр «только нарушители»** — режим, в котором отображаются только модули и рёбра, причастные к ошибкам/предупреждениям. Поможет на больших графах (web-client: 387 модулей из 1948 в нарушениях).
- [ ] **Фокус на цикле** — клик на nooobody-circular-deps violation в outline → центрирование + подсветка всех модулей и рёбер цикла. Лечит проблему 953-узловых сообщений (см. follow-up в smoke-отчёте).
- [ ] **Сохранение состояния viewport** (zoom, раскрытые слои, активный фильтр) в `localStorage` — чтобы перезагрузка не сбрасывала.
- [ ] **Layout-direction toggle** (LR ↔ TB) — для презентаций удобно TB.
- [ ] **Live-reload** через `--watch` режим: визуализатор смотрит на ту же файловую систему, что и lint в watch-mode, при изменении — пересчитывает snapshot + violations + обновляет сцену. Hot-reload и для `.rhai` плагинов тоже (см. Plan D).
- [ ] **Минимап** в углу — нужен на >500 видимых нодах.
- [ ] **Цветовая шкала по stage** опционально — клик в легенде переключает раскраску узлов с kind на stage 1..4.

### P2 — желательно

- [ ] **Diff-режим** — `stratum-lint visualize --diff base.snapshot.json head.snapshot.json`. Узлы/рёбра: добавленные зелёные, удалённые красные, изменённые жёлтые. Отдельно от текущих ошибок Stratum. Закрывает запрос «как мы менялись?» от архитекторов.
- [ ] **Экспорт картинки** (PNG/SVG) текущего viewport — для PR-описаний и презентаций.
- [ ] **Shareable URL** — состояние viewport кодируется в hash; копируешь ссылку, у коллеги открывается на том же узле / в том же фильтре. Работает локально (snapshot тот же), не требует сервера.
- [ ] **Анимация переходов** при click-to-expand — мягкое раскрытие агрегата вместо мгновенного.
- [ ] **Кастомные цвета слоёв** — конфигурируется в `stratum.config.jsonc`, чтобы цветовая схема совпадала с дизайн-системой команды.

### P3 — позже / опционально

- [ ] **Server-side layout для огромных графов** (> 5K модулей). dagre на стороне фронта упрётся; альтернатива — Sugiyama в Rust→WASM (см. ADR-0001 «Custom Sugiyama» — Phase 12+).
- [ ] **Multi-snapshot** — открыть несколько проектов в одной вкладке, видеть зависимости между ними (если они есть через workspace/monorepo links).
- [ ] **WebGPU renderer path** — PixiJS 8 уже умеет, для огромных графов даст FPS-boost.
- [ ] **Voice-over / a11y** — keyboard navigation по графу, screen-reader для outline.

## UX-принципы

- **Тёмная тема по умолчанию** — большинство IDE тёмные, контраст лучше.
- **Никаких модалок и блокирующих диалогов** — всё inline. Левая outline-панель / правая panel for details.
- **F5 = пересчитать snapshot+violations** (после live-reload это станет автоматическим).
- **Esc = вернуть viewport в `fit-to-bounds`**.
- **Все hover-tooltips появляются через ~200 ms** — не дёргаются при быстром движении мыши.

## Известные ограничения

- **`feature → feature` / `entity → entity`** запрет из `eslint-plugin-boundaries` не выражается, пока в Stratum не появились visibility scopes. Визуализатор покажет такие импорты как обычные рёбра, без подсветки.
- **Sliced view** работает только по слоям. Иерархическая агрегация (слой → группа контейнеров → модули) для FSD-style проектов с глубоким `entities/<entity>/<segment>/...` — пока нет.
- **Hot-reload снапшота** ещё не сделан (P1) — после правки кода надо перезапустить `stratum-lint visualize`.

## Не-цели (явно)

- **Edit-in-place** — визуализатор только смотрит, не правит. Кнопка «исправить нарушение» = открыть файл в редакторе, не делать правку из браузера.
- **Auto-layout оптимизация** через ML / прочую магию — dagre + sliced view покрывают MVP. Custom Sugiyama — крайний случай.
- **Замена graphviz / mermaid для ad-hoc диаграмм** — это инструмент для конкретно Stratum-снапшотов, не общий граф-вьюер.
- **Облачный SaaS** — всё локально. Если команда хочет share — `stratum-lint snapshot --out` + статический хостинг JSON.

## Referenced docs

- ADR-0001 — layout backend choice (`@dagrejs/dagre`, sliced threshold 500).
- `docs/superpowers/plans/2026-05-19-pixijs-rendering.md` — план реализации MVP-рендера.
- `docs/superpowers/plans/2026-05-19-web-client-integration.md` — validation плана на реальном проекте (web-client).
- `docs/integration/2026-05-19-web-client-smoke.md` — первые цифры на web-client.
- `RELEASE.md` — Snapshot version policy.
- `UBIQUITOUS_LANGUAGE.md` — глоссарий (Graph Snapshot, Compound DAG, Container, Module, Layer).
