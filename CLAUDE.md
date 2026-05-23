# CLAUDE.md — Stratum

Stratum — архитектурный линтер для TS/Vue с LSP-сервером и интерактивным визуализатором. Workspace из Rust-кретов + Solid+TS фронтенд визуализатора, инлайнящийся в бинарь через `rust-embed`.

См. также: [`README.md`](./README.md) — public-фейс, [`_hot.md`](./_hot.md) — текущее состояние main, [`docs/superpowers/specs/`](./docs/superpowers/specs/) и [`docs/superpowers/plans/`](./docs/superpowers/plans/) — активные спеки и планы. Memory-индекс пользователя: `~/.claude/projects/D--web-projects-stratum/memory/MEMORY.md`.

## Перед началом задачи

1. `_hot.md` — что в работе сейчас
2. `MEMORY.md` пользователя — фазы 0–10 закрыты, плюс post-v0.1 roadmap + WebGL-прототип
3. `docs/superpowers/plans/` — если задача исполняет план
4. Соответствующий CLAUDE.md / `_index.md` подмодуля если есть

Планы новых фич сохраняй в `docs/superpowers/plans/YYYY-MM-DD-<topic>.md`, спеки — в `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`.

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
    dist/               # vite build → embedded в stratum-lint через rust-embed
editor-extensions/      # VSCode / Zed / WebStorm LSP-обёртки
tests/fixtures/         # tiny-ts (~14 модулей), tiny-ts-violations, exemplar-stratum-app
docs/
  integration/          # отчёты по реальным проектам (web-client smoke и т.п.)
  superpowers/
    specs/              # дизайн-документы фич
    plans/              # пошаговые планы имплементации
```

Pipeline: `adapt(GraphSnapshot, Violation[]) → DesignData → computeLayout → Scene → <Graph/> или <GraphWebGL/>`.

## Dev-loop A: UI визуализатора (основной случай)

Когда правишь Solid-компоненты, CSS, цвета, layout, рендер-слой — **этот режим**. HMR за миллисекунды, Rust пересобирать не надо.

### Один раз: проверь worktree web-client

```bash
ls D:/web-projects/web-client-stratum-stages/stratum.config.jsonc
```

Это форк `D:/web-projects/web-client` с `stratum.config.jsonc`, описывающим слои `core → shared → entities → features → pages → app` (как в Claude Design handoff). **Используй именно его** — на голом `web-client` без конфига Stratum инферит 20 шумных «слоёв» из реальных папок (`api/assets/composables/...`).

### Терминал 1 — Rust backend (живёт фоном)

```powershell
cd D:\web-projects\stratum
.\target\release\stratum-lint.exe visualize D:\web-projects\web-client-stratum-stages --port 8080
```

Cold lint web-client'а — ~30 сек, дальше тёплый кэш. Эндпоинты:
- `GET /api/snapshot` — граф (~5 MB JSON)
- `GET /api/violations` — нарушения
- `GET /` — встроенный dist (в этом режиме НЕ используем — фронт идёт через vite)

Один раз собрать `.exe` если ещё нет: `cargo build --release -p stratum-lint`.

### Терминал 2 — Vite dev

```powershell
cd D:\web-projects\stratum\frontend\stratum-visualizer-frontend
$env:VITE_API_PORT="8080"; npm run dev
```

Bash: `VITE_API_PORT=8080 npm run dev`.

Vite слушает `http://localhost:5173`, проксирует `/api/*` на `127.0.0.1:8080` (см. `vite.config.ts:9-11`). Дефолт `apiPort` в конфиге — `8080`, так что переменная нужна только если backend на другом порту.

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
cargo watch -w crates -x "run --release -p stratum-lint -- visualize D:\web-projects\web-client-stratum-stages --port 8080"
```

`cargo-watch`:
- Слушает изменения в `crates/**`
- Убивает старый `stratum-lint` процесс (важно на Windows из-за file-lock на .exe)
- Запускает заново
- Re-lint web-client'а — снова ~30 сек cold

Терминал 2 (vite dev) живёт независимо. После рестарта backend'а **F5 в браузере** — Solid через `createResource(loadSnapshot)` подтянет новый snapshot. HMR между Rust и Vite напрямую не существует — это две независимые песочницы; единственное окно связи между ними — твой ручной refresh.

### Когда web-client — overkill

Для большинства правок Rust (рендер, http, layout) вид графа неважен — переключайся на лёгкую фикстуру:

```powershell
cargo watch -w crates -x "run --release -p stratum-lint -- visualize tests\fixtures\tiny-ts --port 8080"
```

`tiny-ts` — 14 модулей, lint < 50ms. Готово к итерациям сразу. Когда дойдёшь до интеграции — переключаешься обратно на web-client.

### Если backend перестал отвечать

Скорее всего Cargo пересобирал `.exe` пока он был запущен (Windows lock). Останови сервер (Ctrl+C или TaskManager → `stratum-lint.exe`), потом `cargo build`. `cargo-watch` от этого защищает — но если запускал вручную, бывает.

## Dev-loop C: end-to-end (embedded, перед коммитом)

Проверяешь, что dist собрался и инлайнится в .exe корректно.

```powershell
cd D:\web-projects\stratum\frontend\stratum-visualizer-frontend
npm run build                                                  # → dist/

cd D:\web-projects\stratum
cargo build --release -p stratum-lint                          # 20-45 сек
.\target\release\stratum-lint.exe visualize D:\web-projects\web-client-stratum-stages --port 8080
```

Открываешь `http://localhost:8080/?renderer=webgl` (НЕ 5173 — backend сам отдаёт встроенный dist).

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
| `8080` | Rust backend `stratum-lint visualize` — дефолт для пары с vite |
| `18080` | Альтернативный — если 8080 занят |
| `18090` | (опц.) Эталонный Claude Design handoff: `python -m http.server 18090` в `.tmp/viz-handoff/stratum-vizualiser/project/` |

## Что НЕ коммитим

- `frontend/stratum-visualizer-frontend/dist/` — артефакт сборки, но **нужен** для `cargo build --release` (`rust-embed` инлайнит). В .gitignore с feature-flag `debug-embed` для dev-билдов.
- `target/` — Rust build cache
- `.tmp/viz-handoff/` — Claude Design handoff, остаётся локальным
- `node_modules/`

## Code style

Загружай скилл `code-style` при работе с `.ts/.tsx/.vue/.js/.jsx` файлами. Канон правил — там.

Для русского текста в MR-описаниях, документации, коммитах, комментариях — скилл `make-ru` (Ильяхов, инфостиль, без англицизмов).

## Процесс

- Новая фича / refactor — сначала `superpowers:brainstorming`, потом `superpowers:writing-plans`, потом `superpowers:subagent-driven-development` или `superpowers:executing-plans`
- Баг / странное поведение — `superpowers:systematic-debugging`
- Перед claim'ом «готово» — `superpowers:verification-before-completion`
- Перед merge — `superpowers:requesting-code-review` или скилл `code-review`

## Активные ветки на момент написания

- `main` — Phase 10 complete, post-v0.1 roadmap смержен (2026-05-19)
- `webgl-prototype` — WebGL прототип визуализатора (PixiJS v8 + pixi-viewport), 14 коммитов поверх main. См. `docs/superpowers/specs/2026-05-23-webgl-renderer-prototype-design.md` и `docs/integration/2026-05-23-webgl-prototype-trace.md`
