# CLAUDE.md — Stratum

Stratum — архитектурный линтер для TS/Vue с LSP-сервером и интерактивным визуализатором. Workspace из Rust-кретов + Solid+TS фронтенд визуализатора, инлайнящийся в бинарь через `rust-embed`.

См. также: [`README.md`](./README.md) — public-фейс, [`_hot.md`](./_hot.md) — текущее состояние main.

## Правила
Do not add tests which simply restate the implementation. These provide zero confidence. 

## Структура

```
crates/
  stratum-core          # типы, source ranges, severity, snapshot v1 на проводе
  stratum-config        # JSONC-парсинг stratum.config.jsonc
  stratum-parser-ts     # OXC-обёртка для .ts/.js/.tsx/.jsx
  stratum-parser-vue    # SFC <script>-extractor поверх parser-ts
  stratum-graph         # Salsa-инкрементальный модуль/контейнер/edge graph
  stratum-rules         # 9 встроенных правил + RuleEngine
  stratum-plugins-rhai  # Rhai-скрипт плагины (sandboxed)
  stratum-lint          # CLI: lint / visualize / init / version (+ rust-embed dist)
  stratum-lsp           # tower-lsp 0.20, diagnostics + tracing→window/logMessage
frontend/
  stratum-visualizer-frontend/
    src/                # Solid + TypeScript
editor-extensions/      # VSCode / Zed / WebStorm LSP-обёртки
docs/
  integration/          # отчёты по реальным проектам (web-client smoke и т.п.)
```

Pipeline: `adapt(GraphSnapshot, Violation[]) → DesignData → computeLayout → Scene → <Graph/> или <GraphWebGL/>`. `SnapshotContainer` рекурсивный — папочная иерархия из `stratum-graph` приходит уже вложенной.

## Тестовая база: всегда web-client форк

Все локальные прогоны визуализатора и Rust-итераций — на форке web-client с готовым `stratum.config.jsonc`:

```
D:\web-projects\web-client-stratum-stages
```

Ветка `stratum-stages-2026-05-19`. Конфиг описывает слои `core → shared → entities → features → pages → app` (зеркало `eslint-plugin-boundaries` web-client'а).

Голый `D:\web-projects\web-client` НЕ используем — без конфига Stratum инферит 20 шумных «слоёв» из реальных папок (`api/assets/composables/...`), и шапка визуализатора превращается в кашу из чипов.

Web-client после первого холодного линта (~30 сек) кэшируется; следующие пересчёты тёплые.

## Dev-loop A: UI визуализатора (основной случай)

Когда правишь Solid-компоненты, CSS, цвета, layout, рендер-слой — **этот режим**. HMR за миллисекунды, Rust пересобирать не надо.

### Загрузка PixiJS-скиллов
WebGL — основной рендер сцены. **Перед любой работой с PixiJS подгружай router `pixijs-skills:pixijs`** — он содержит таблицу под-скиллов и fallback на `llms.txt`, дальше под задачу загружай нужный под-скилл из роутера.

### Терминал 1 — Rust backend (живёт фоном)

```powershell
cd D:\web-projects\stratum
.\target\release\stratum-lint.exe visualize D:\web-projects\web-client-stratum-stages --port 18080
```

Один раз собрать `.exe` если ещё нет: `cargo build --release -p stratum-lint`.

Эндпоинты:
- `GET /api/snapshot` — граф (~5 MB JSON)
- `GET /api/violations` — нарушения
- `GET /` — встроенный dist (в этом режиме НЕ используем — фронт идёт через vite)

### Терминал 2 — Vite dev

```powershell
cd D:\web-projects\stratum\frontend\stratum-visualizer-frontend
npm run dev
```

Vite слушает `http://localhost:5173`, проксирует `/api/*` на `127.0.0.1:18080` (см. `vite.config.ts:9-11`). Дефолт `apiPort` в конфиге — `18080` (на Windows-dev'е порт 8080 обычно занят EnterpriseDB/WAMP/Jenkins). Если запускаешь backend на 8080 — `VITE_API_PORT=8080 npm run dev`.

Открываешь:
- `http://localhost:5173/?renderer=svg` — текущий боевой рендер
- `http://localhost:5173/?renderer=webgl` — WebGL прототип (ветка `webgl-prototype`)

Значение запоминается в `localStorage['stratum.renderer']`.

### Что переживёт что

| Меняешь | Кого перезапускать |
|---|---|
| `.tsx` / `.ts` / `.css` фронта | ничего — HMR |
| `vite.config.ts` | Терминал 2 |
| Rust код в `crates/**` | Терминал 1 + `cargo build --release` |
| `stratum.config.jsonc` в worktree | Терминал 1 |
| Файлы внутри web-client | Терминал 1 (переснэпшотить) |

## Dev-loop B: Rust бэкенд

Когда правишь правила, парсер, snapshot shape, HTTP-обработчики, layout-логику в Rust.

### Авто-рестарт через `cargo-watch`

Один раз:
```bash
cargo install cargo-watch
```

Дальше — Терминал 1 заменяешь на:

```powershell
cd D:\web-projects\stratum
cargo watch -w crates -x "run --release -p stratum-lint -- visualize D:\web-projects\web-client-stratum-stages --port 18080"
```

`cargo-watch`:
- Слушает изменения в `crates/**`
- Убивает старый `stratum-lint` процесс (важно на Windows из-за file-lock на `.exe`)
- Запускает заново, тёплый кэш web-client'а быстро прогревается

Терминал 2 (vite dev) живёт независимо. После рестарта backend'а **F5 в браузере** — Solid через `createResource(loadSnapshot)` подтянет новый snapshot. Прямого HMR между Rust и Vite нет (два независимых процесса) — единственное окно связи между ними твой ручной refresh.

### Если backend перестал отвечать

Скорее всего Cargo пересобирал `.exe` пока он был запущен (Windows lock). Останови сервер (Ctrl+C или TaskManager → `stratum-lint.exe`), потом `cargo build`. `cargo-watch` от этого защищает — но если запускал вручную, бывает.

## Dev-loop C: end-to-end (embedded, перед коммитом)

Проверяешь, что dist собрался и инлайнится в `.exe` корректно.

```powershell
cd D:\web-projects\stratum\frontend\stratum-visualizer-frontend
npm run build                                                  # → dist/

cd D:\web-projects\stratum
cargo build --release -p stratum-lint                          # 20-45 сек
.\target\release\stratum-lint.exe visualize D:\web-projects\web-client-stratum-stages --port 18080
```

Открываешь `http://localhost:18080/?renderer=webgl` (НЕ 5173 — backend сам отдаёт встроенный dist).

## Анализ и верификация UI — Chrome MCP

Для любой визуальной проверки, дебага рендера, инспекции DOM/canvas, замеров перфоманса и e2e-сценариев в браузере — **используй MCP-инструменты**, а не Playwright-скрипты или ручные скриншоты:

- **`mcp__claude-in-chrome__*`** — навигация, клики, скриншоты, чтение страницы, console/network логи, GIF-запись многошаговых сценариев. Основной инструмент для «открой `http://localhost:5173/?renderer=webgl`, проверь, что граф отрендерился, сними скриншот».
- **`mcp__plugin_chrome-devtools-mcp_chrome-devtools__*`** — DevTools-протокол: performance trace, lighthouse audit, memory snapshot, evaluate_script в контексте страницы, детальный network. Для перф-регрессий WebGL-рендера и утечек.

Триггеры на подключение скиллов: `chrome-devtools`, `a11y-debugging`, `debug-optimize-lcp`, `memory-leak-debugging`, `pixijs-performance`.

Workflow по умолчанию: **перед** правкой UI — снять baseline-скриншот через chrome MCP, **после** — снять второй и сравнить. Не «я поменял CSS, должно работать», а evidence-based.

## Тесты

```powershell
# Rust
cargo test --workspace

# Frontend unit
cd frontend\stratum-visualizer-frontend
npm test

# Frontend e2e (Playwright) — только если меняешь Graph/lifecycle
npm run test:e2e

# Typecheck без билда
npm run typecheck
```

CI запускает `fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace` на Linux/macOS/Windows + frontend bundle-size budget 1.5 MB.

## Шпаргалка по портам

| Порт | Что |
|---|---|
| `5173` | Vite dev — **этот URL открывай в браузере** в HMR-режиме |
| `18080` | Rust backend `stratum-lint visualize` — дефолт для пары с vite (8080 на Windows почти всегда занят) |
| `8080` | Альтернатива — если хочется привычный порт; запускай vite с `VITE_API_PORT=8080` |

## Активные ветки

- `main` — Phase 10 complete, post-v0.1 roadmap смержен (2026-05-19)
- `webgl-prototype` — WebGL прототип визуализатора (PixiJS v8 + pixi-viewport). Статус и follow-up'ы — в [`_hot.md`](./_hot.md)
